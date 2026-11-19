//! Lecture de l'Inbox via IMAP (imap.gmail.com:993, TLS) avec authentification XOAUTH2.
//! Une connexion par appel : simple et robuste pour l'étape 2 (la synchro persistante viendra à l'étape 4).

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use mailparse::{addrparse, dateparse, parse_headers, parse_mail, MailAddr, MailHeaderMap, ParsedMail};
use serde::Serialize;

const IMAP_PORT: u16 = 993;

/// Serveur IMAP selon le fournisseur (Office 365 et Hotmail/Outlook.com partagent le même).
pub fn imap_host(provider: &str) -> &'static str {
    if provider == "microsoft" { "outlook.office365.com" } else { "imap.gmail.com" }
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageSummary {
    /// Identifiant unique côté interface : "<compte>:<uid>".
    pub id: String,
    pub uid: u32,
    pub acct: String,
    pub from: String,
    /// Adresse e-mail de l'expéditeur (pour répondre).
    pub addr: String,
    pub subject: String,
    /// Horodatage Unix (secondes) ; l'interface le formate selon la langue du système.
    pub ts: i64,
    pub unread: bool,
    /// "people" | "notifications" | "newsletters"
    pub cat: String,
    /// Étoilé dans Gmail (= « épinglé » dans SXM Mail).
    pub flagged: bool,
    /// Dossier IMAP d'origine ("INBOX" ou le dossier des envoyés), à renvoyer pour ouvrir le message.
    pub mailbox: String,
}

struct XOAuth2 {
    user: String,
    token: String,
}

impl imap::Authenticator for XOAuth2 {
    type Response = String;
    fn process(&self, challenge: &[u8]) -> Self::Response {
        if challenge.is_empty() {
            format!("user={}\x01auth=Bearer {}\x01\x01", self.user, self.token)
        } else {
            // Le serveur envoie un JSON d'erreur : on répond par une ligne vide pour qu'il termine proprement.
            String::new()
        }
    }
}

type Session = imap::Session<native_tls::TlsStream<std::net::TcpStream>>;

fn open_session(host: &str, email: &str, access_token: &str) -> Result<Session> {
    let tls = native_tls::TlsConnector::builder().build().context("initialisation TLS")?;
    let client = imap::connect((host, IMAP_PORT), host, &tls).with_context(|| format!("connexion à {host}"))?;
    let auth = XOAuth2 { user: email.to_string(), token: access_token.to_string() };
    client
        .authenticate("XOAUTH2", &auth)
        .map_err(|(e, _)| anyhow!("authentification IMAP refusée : {e}"))
}

/// Les `limit` messages les plus récents de l'Inbox, du plus récent au plus ancien.
pub fn fetch_inbox(host: &str, account_id: &str, email: &str, access_token: &str, limit: u32) -> Result<Vec<MessageSummary>> {
    let mut session = open_session(host, email, access_token)?;
    let out = fetch_mailbox(&mut session, account_id, "INBOX", limit, false);
    let _ = session.logout();
    out
}

/// Un dossier du compte : boîte de réception, dossiers spéciaux de Gmail ou dossier personnel (libellé).
#[derive(Debug, Clone, Serialize)]
pub struct FolderInfo {
    /// "inbox" | "sent" | "drafts" | "junk" | "trash" | "all" | "user"
    pub kind: String,
    /// Nom IMAP exact (encodé en UTF-7 modifié), à renvoyer tel quel au serveur.
    pub name: String,
    /// Nom lisible (décodé) : « Marie », « Clients/Hôtel »…
    pub label: String,
}

/// Rôle d'un dossier d'après ses attributs (RFC 6154), sinon "user".
fn folder_kind(n: &imap::types::Name) -> Option<&'static str> {
    if n.name().eq_ignore_ascii_case("INBOX") {
        return Some("inbox");
    }
    if n.attributes().iter().any(|a| matches!(a, imap::types::NameAttribute::NoSelect)) {
        return None;
    }
    for a in n.attributes() {
        if let imap::types::NameAttribute::Custom(s) = a {
            let s = s.to_ascii_lowercase();
            let k = match s.as_str() {
                "\\sent" => "sent",
                "\\drafts" => "drafts",
                "\\junk" => "junk",
                "\\trash" => "trash",
                "\\all" => "all",
                // Dossiers virtuels de Gmail (Important, Suivis) : déjà couverts par SXM Mail.
                "\\important" | "\\flagged" => return None,
                _ => continue,
            };
            return Some(k);
        }
    }
    // Les autres dossiers internes de Gmail ([Gmail]/…) ne sont pas des dossiers personnels.
    if n.name().starts_with("[Gmail]") || n.name().starts_with("[Google Mail]") {
        return None;
    }
    Some("user")
}

fn all_folders(session: &mut Session) -> Result<Vec<FolderInfo>> {
    let names = session.list(None, Some("*")).context("liste des dossiers")?;
    let mut out: Vec<FolderInfo> = names
        .iter()
        .filter_map(|n| {
            folder_kind(n).map(|k| FolderInfo { kind: k.into(), name: n.name().to_string(), label: decode_utf7_imap(n.name()) })
        })
        .collect();
    out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    Ok(out)
}

/// Tous les dossiers utiles du compte (spéciaux et personnels).
pub fn list_folders(host: &str, email: &str, access_token: &str) -> Result<Vec<FolderInfo>> {
    let mut session = open_session(host, email, access_token)?;
    let out = all_folders(&mut session);
    let _ = session.logout();
    out
}

/// Nom IMAP du dossier ayant ce rôle ("sent", "junk", "trash", "all", "drafts").
fn folder_by_kind(session: &mut Session, kind: &str) -> Result<String> {
    if kind == "inbox" {
        return Ok("INBOX".into());
    }
    all_folders(session)?
        .into_iter()
        .find(|f| f.kind == kind)
        .map(|f| f.name)
        .ok_or_else(|| anyhow!("{} introuvable dans ce compte", kind_label(kind)))
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "sent" => "le dossier des messages envoyés",
        "junk" => "le dossier Courrier indésirable",
        "trash" => "la corbeille",
        "all" | "archive" => "le dossier « Tous les messages »",
        "drafts" => "le dossier Brouillons",
        _ => "le dossier",
    }
}

/// Recherche Gmail des archives : les messages sortis de la boîte de réception.
const ARCHIVE_QUERY: &str = "X-GM-RAW \"-in:inbox -in:sent -in:drafts -in:chats\"";

/// Une page de messages d'un dossier.
#[derive(Debug, Clone, Serialize)]
pub struct FolderPage {
    pub mailbox: String,
    pub messages: Vec<MessageSummary>,
}

/// Messages d'un dossier, du plus récent au plus ancien.
/// - `kind` : "inbox", "sent", "junk", "trash", "archive" ou "user" (alors `name` = nom IMAP exact) ;
/// - `before_uid` : pour « Charger les mails plus anciens ».
pub fn fetch_folder(
    host: &str,
    account_id: &str,
    email: &str,
    access_token: &str,
    kind: &str,
    name: Option<&str>,
    limit: u32,
    before_uid: Option<u32>,
) -> Result<FolderPage> {
    let mut session = open_session(host, email, access_token)?;
    let result = (|| {
        let mailbox = match (kind, name) {
            ("user", Some(n)) => n.to_string(),
            ("user", None) => bail!("nom de dossier manquant"),
            ("archive", _) => folder_by_kind(&mut session, "all")?,
            (k, _) => folder_by_kind(&mut session, k)?,
        };
        let sent = kind == "sent";
        let messages = if kind == "archive" || before_uid.is_some() {
            if before_uid.is_some_and(|b| b <= 1) {
                return Ok(FolderPage { mailbox, messages: vec![] });
            }
            session.examine(&mailbox).with_context(|| format!("ouverture du dossier {}", decode_utf7_imap(&mailbox)))?;
            let mut query = match before_uid {
                Some(b) => format!("UID 1:{}", b - 1),
                None => "ALL".to_string(),
            };
            if kind == "archive" {
                query = format!("{query} {ARCHIVE_QUERY}");
            }
            let mut uids: Vec<u32> = session.uid_search(&query).context("recherche des messages")?.into_iter().filter(|u| before_uid.map_or(true, |b| *u < b)).collect();
            uids.sort_unstable();
            let page: Vec<String> = uids.iter().rev().take(limit as usize).map(|u| u.to_string()).collect();
            if page.is_empty() {
                vec![]
            } else {
                let fetched = session.uid_fetch(page.join(","), "(UID FLAGS BODY.PEEK[HEADER])").context("lecture des en-têtes")?;
                summaries(&fetched, account_id, &mailbox, sent)
            }
        } else {
            fetch_mailbox(&mut session, account_id, &mailbox, limit, sent)?
        };
        Ok(FolderPage { mailbox, messages })
    })();
    let _ = session.logout();
    result
}

/// Crée un dossier personnel (un libellé dans Gmail) ; renvoie son nom IMAP. Ne fait rien s'il existe déjà.
pub fn create_folder(host: &str, email: &str, access_token: &str, label: &str) -> Result<String> {
    let label = clean_folder_label(label)?;
    let mut session = open_session(host, email, access_token)?;
    let result = (|| {
        if let Some(f) = all_folders(&mut session)?.into_iter().find(|f| f.label.eq_ignore_ascii_case(&label)) {
            return Ok(f.name);
        }
        let raw = encode_utf7_imap(&label);
        session.create(&raw).with_context(|| format!("création du dossier « {label} »"))?;
        Ok(raw)
    })();
    let _ = session.logout();
    result
}

fn clean_folder_label(label: &str) -> Result<String> {
    let l: String = label.chars().filter(|c| !c.is_control()).collect::<String>().trim().trim_matches('/').to_string();
    if l.is_empty() {
        bail!("donne un nom au dossier");
    }
    if l.chars().count() > 100 {
        bail!("nom de dossier trop long (100 caractères au maximum)");
    }
    if l.eq_ignore_ascii_case("inbox") || l.starts_with("[Gmail]") || l.starts_with("[Google Mail]") {
        bail!("ce nom est réservé par Gmail");
    }
    Ok(l)
}

/// Déplace un message vers un dossier : `target` = rôle ("inbox", "junk", "trash", "archive")
/// ou "user" avec `target_name` = nom IMAP exact.
pub fn move_message(
    host: &str,
    email: &str,
    access_token: &str,
    mailbox: &str,
    uid: u32,
    target: &str,
    target_name: Option<&str>,
) -> Result<String> {
    let mut session = open_session(host, email, access_token)?;
    let result = (|| {
        let dest = match (target, target_name) {
            ("user", Some(n)) => n.to_string(),
            ("user", None) => bail!("dossier de destination manquant"),
            ("archive", _) => folder_by_kind(&mut session, "all")?,
            (k, _) => folder_by_kind(&mut session, k)?,
        };
        if dest == mailbox {
            return Ok(dest);
        }
        session.select(mailbox).with_context(|| format!("ouverture du dossier {}", decode_utf7_imap(mailbox)))?;
        session.uid_mv(uid.to_string(), &dest).context("déplacement du message")?;
        Ok(dest)
    })();
    let _ = session.logout();
    result
}

/// Actions synchronisées avec Gmail : "delete" (corbeille), "purge" (suppression définitive, depuis la corbeille
/// ou les indésirables), "archive" (retire de la boîte de réception), "star", "unstar", "read", "unread".
pub fn message_action(host: &str, email: &str, access_token: &str, mailbox: &str, uid: u32, action: &str) -> Result<()> {
    match action {
        "delete" => return move_message(host, email, access_token, mailbox, uid, "trash", None).map(|_| ()),
        "archive" => return move_message(host, email, access_token, mailbox, uid, "archive", None).map(|_| ()),
        _ => {}
    }
    let flags = match action {
        "star" => "+FLAGS (\\Flagged)",
        "unstar" => "-FLAGS (\\Flagged)",
        "read" => "+FLAGS (\\Seen)",
        "unread" => "-FLAGS (\\Seen)",
        "purge" => "+FLAGS (\\Deleted)",
        other => bail!("action inconnue : {other}"),
    };
    let mut session = open_session(host, email, access_token)?;
    let result = (|| {
        if action == "purge" {
            // Par sécurité, la suppression définitive n'est permise que depuis la corbeille ou les indésirables.
            let trash = folder_by_kind(&mut session, "trash").ok();
            let junk = folder_by_kind(&mut session, "junk").ok();
            if Some(mailbox) != trash.as_deref() && Some(mailbox) != junk.as_deref() {
                bail!("suppression définitive possible seulement depuis la corbeille ou le courrier indésirable");
            }
        }
        session.select(mailbox).with_context(|| format!("ouverture du dossier {}", decode_utf7_imap(mailbox)))?;
        session.uid_store(uid.to_string(), flags).context("mise à jour du message")?;
        if action == "purge" {
            session.uid_expunge(uid.to_string()).context("suppression définitive")?;
        }
        Ok(())
    })();
    let _ = session.logout();
    result
}

fn fetch_mailbox(session: &mut Session, account_id: &str, mailbox_name: &str, limit: u32, sent: bool) -> Result<Vec<MessageSummary>> {
    let mailbox = session.examine(mailbox_name).with_context(|| format!("ouverture du dossier {mailbox_name}"))?;
    if mailbox.exists == 0 {
        return Ok(vec![]);
    }
    let start = mailbox.exists.saturating_sub(limit.saturating_sub(1)).max(1);
    let fetched = session
        .fetch(format!("{start}:{}", mailbox.exists), "(UID FLAGS BODY.PEEK[HEADER])")
        .context("lecture des en-têtes")?;
    Ok(summaries(&fetched, account_id, mailbox_name, sent))
}

fn summaries(fetched: &[imap::types::Fetch], account_id: &str, mailbox_name: &str, sent: bool) -> Vec<MessageSummary> {
    let mut out = Vec::new();
    for f in fetched.iter() {
        let (Some(uid), Some(raw)) = (f.uid, f.header()) else { continue };
        let Ok((headers, _)) = parse_headers(raw) else { continue };
        // Pour les envoyés, on affiche le destinataire à la place de l'expéditeur.
        let who_raw = headers.get_first_value(if sent { "To" } else { "From" }).unwrap_or_default();
        let subject = headers.get_first_value("Subject").unwrap_or_default();
        let ts = headers.get_first_value("Date").and_then(|d| dateparse(&d).ok()).unwrap_or(0);
        let (name, addr) = sender(&who_raw);
        let cat = if sent { "people" } else { categorize(&addr, &headers) };
        out.push(MessageSummary {
            id: format!("{account_id}:{mailbox_name}:{uid}"),
            mailbox: mailbox_name.to_string(),
            uid,
            acct: account_id.to_string(),
            from: if name.is_empty() { addr.clone() } else { name },
            addr,
            subject: if subject.trim().is_empty() { "(sans objet)".into() } else { subject },
            ts,
            unread: !sent && !f.flags().iter().any(|fl| matches!(fl, imap::types::Flag::Seen)),
            flagged: f.flags().iter().any(|fl| matches!(fl, imap::types::Flag::Flagged)),
            cat: cat.into(),
        });
    }
    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    out
}

/// Contenu d'un message pour l'affichage.
#[derive(Debug, Clone, Serialize)]
pub struct MessageBody {
    /// Version texte (text/plain, sinon HTML converti) : sert aussi d'aperçu et de citation pour répondre.
    pub text: String,
    /// Version HTML, avec les images intégrées (cid:) converties en data: ; affichée dans un cadre isolé.
    pub html: Option<String>,
    /// Le HTML contient des images distantes (bloquées par défaut : elles servent souvent au pistage).
    pub has_remote_images: bool,
    pub attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentInfo {
    /// Position dans le message (pour l'enregistrer ensuite).
    pub index: usize,
    pub filename: String,
    pub mime: String,
    pub size: usize,
}

const MAX_HTML: usize = 2_000_000;
const MAX_INLINE_TOTAL: usize = 8_000_000;

/// Lit un message complet et le marque comme lu.
pub fn fetch_message(host: &str, email: &str, access_token: &str, mailbox: &str, uid: u32) -> Result<MessageBody> {
    let mut session = open_session(host, email, access_token)?;
    session.select(mailbox).with_context(|| format!("ouverture du dossier {mailbox}"))?;
    let fetched = session.uid_fetch(uid.to_string(), "BODY.PEEK[]").context("lecture du message")?;
    let raw = fetched.iter().next().and_then(|f| f.body()).ok_or_else(|| anyhow!("message introuvable"))?;
    let body = parse_body(raw)?;
    let _ = session.uid_store(uid.to_string(), "+FLAGS (\\Seen)");
    let _ = session.logout();
    Ok(body)
}

fn parse_body(raw: &[u8]) -> Result<MessageBody> {
    let parsed = parse_mail(raw).context("analyse du message")?;
    let html = find_part(&parsed, "text/html").filter(|h| h.len() <= MAX_HTML).map(|h| {
        let mut inline = Vec::new();
        collect_inline_images(&parsed, &mut inline, &mut 0);
        inline.into_iter().fold(h, |acc, (cid, data)| acc.replace(&format!("cid:{cid}"), &data))
    });
    let text = find_part(&parsed, "text/plain")
        .or_else(|| html.as_deref().map(strip_html))
        .unwrap_or_else(|| "(Ce message ne contient pas de texte lisible.)".into());
    let has_remote_images = html.as_deref().map(has_remote_images).unwrap_or(false);
    let attachments = attachment_parts(&parsed)
        .into_iter()
        .enumerate()
        .map(|(index, p)| AttachmentInfo {
            index,
            filename: attachment_name(p, index),
            mime: p.ctype.mimetype.to_lowercase(),
            size: p.get_body_raw().map(|b| b.len()).unwrap_or(0),
        })
        .collect();
    Ok(MessageBody { text: text.chars().take(50_000).collect(), html, has_remote_images, attachments })
}

/// Pièces jointes : parties marquées « attachment », ou nommées sans Content-ID (les images intégrées au mail sont exclues).
fn attachment_parts<'a>(mail: &'a ParsedMail<'a>) -> Vec<&'a ParsedMail<'a>> {
    let mut out = Vec::new();
    fn walk<'a>(m: &'a ParsedMail<'a>, out: &mut Vec<&'a ParsedMail<'a>>) {
        if m.subparts.is_empty() {
            let disp = m.get_content_disposition();
            let named = disp.params.contains_key("filename") || m.ctype.params.contains_key("name");
            let has_cid = m.headers.get_first_value("Content-ID").is_some();
            if disp.disposition == mailparse::DispositionType::Attachment || (named && !has_cid) {
                out.push(m);
            }
        } else {
            for p in &m.subparts {
                walk(p, out);
            }
        }
    }
    walk(mail, &mut out);
    out
}

/// Nom de fichier sûr : pas de chemin, pas de caractères de contrôle, jamais vide.
fn attachment_name(p: &ParsedMail, index: usize) -> String {
    let disp = p.get_content_disposition();
    let raw = disp.params.get("filename").or_else(|| p.ctype.params.get("name")).cloned().unwrap_or_default();
    let cleaned: String = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .trim_start_matches('.')
        .to_string();
    if cleaned.is_empty() { format!("piece-jointe-{}", index + 1) } else { cleaned.chars().take(180).collect() }
}

/// Enregistre une pièce jointe dans le dossier Téléchargements ; renvoie le chemin créé (jamais d'écrasement).
pub fn save_attachment(host: &str, email: &str, access_token: &str, mailbox: &str, uid: u32, index: usize) -> Result<std::path::PathBuf> {
    let mut session = open_session(host, email, access_token)?;
    let raw = (|| {
        session.examine(mailbox).with_context(|| format!("ouverture du dossier {mailbox}"))?;
        let fetched = session.uid_fetch(uid.to_string(), "BODY.PEEK[]").context("lecture du message")?;
        fetched.iter().next().and_then(|f| f.body()).map(<[u8]>::to_vec).ok_or_else(|| anyhow!("message introuvable"))
    })();
    let _ = session.logout();
    let raw = raw?;
    let parsed = parse_mail(&raw).context("analyse du message")?;
    let parts = attachment_parts(&parsed);
    let part = parts.get(index).ok_or_else(|| anyhow!("pièce jointe introuvable"))?;
    let bytes = part.get_body_raw().context("décodage de la pièce jointe")?;
    let dir = dirs::download_dir().or_else(dirs::home_dir).ok_or_else(|| anyhow!("dossier Téléchargements introuvable"))?;
    std::fs::create_dir_all(&dir)?;
    let name = attachment_name(part, index);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    for n in 0..1000 {
        let candidate = dir.join(if n == 0 { name.clone() } else { format!("{stem} ({n}){ext}") });
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(&bytes)?;
                return Ok(candidate);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    bail!("impossible de trouver un nom de fichier libre")
}

/// Images jointes référencées par Content-ID, converties en URI data: (aucun accès réseau).
fn collect_inline_images(mail: &ParsedMail, out: &mut Vec<(String, String)>, total: &mut usize) {
    if mail.subparts.is_empty() {
        let mime = mail.ctype.mimetype.to_lowercase();
        if let (true, Some(cid)) = (mime.starts_with("image/"), mail.headers.get_first_value("Content-ID")) {
            if let Ok(bytes) = mail.get_body_raw() {
                if *total + bytes.len() <= MAX_INLINE_TOTAL {
                    *total += bytes.len();
                    let cid = cid.trim().trim_start_matches('<').trim_end_matches('>').to_string();
                    out.push((cid, format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))));
                }
            }
        }
        return;
    }
    for p in &mail.subparts {
        collect_inline_images(p, out, total);
    }
}

fn has_remote_images(html: &str) -> bool {
    let l = html.to_ascii_lowercase().replace(['\'', ' '], "\"");
    ["src=\"http", "url(http", "url(\"http", "background=\"http", "srcset=\"http"].iter().any(|k| l.contains(k))
}

fn find_part(mail: &ParsedMail, mime: &str) -> Option<String> {
    if mail.subparts.is_empty() {
        let is_attachment = mail.get_content_disposition().disposition == mailparse::DispositionType::Attachment;
        if mail.ctype.mimetype.eq_ignore_ascii_case(mime) && !is_attachment {
            return mail.get_body().ok();
        }
        return None;
    }
    mail.subparts.iter().find_map(|p| find_part(p, mime))
}

fn sender(raw: &str) -> (String, String) {
    if let Ok(list) = addrparse(raw) {
        for a in list.iter() {
            if let MailAddr::Single(info) = a {
                return (info.display_name.clone().unwrap_or_default(), info.addr.clone());
            }
        }
    }
    (String::new(), raw.trim().to_string())
}

/// Tri des mails reçus :
/// - « newsletters » : listes de diffusion et envois de masse (List-Unsubscribe, List-Id, Precedence: bulk/list) ;
/// - « notifications » : messages automatiques (adresses no-reply, services, plateformes d'envoi, Auto-Submitted) ;
/// - « people » : le reste.
fn categorize(addr: &str, headers: &[mailparse::MailHeader]) -> &'static str {
    let has = |h: &str| headers.get_first_value(h).is_some();
    let precedence = headers.get_first_value("Precedence").unwrap_or_default().to_lowercase();
    if has("List-Unsubscribe") || has("List-Id") || precedence == "bulk" || precedence == "list" {
        return "newsletters";
    }
    let a = addr.to_lowercase();
    let (local, domain) = a.split_once('@').unwrap_or((a.as_str(), ""));
    let automatic_local = [
        "noreply", "no-reply", "no_reply", "donotreply", "do-not-reply", "notification", "notify", "alert", "mailer-daemon",
        "postmaster", "info", "news", "newsletter", "marketing", "support", "service", "services", "account", "accounts",
        "security", "billing", "invoice", "factur", "team", "hello", "contact", "communication", "messages", "updates",
    ];
    let automatic_domain = [
        "paypal.", "lastpass.", "amazon.", "apple.com", "google.com", "facebookmail.com", "linkedin.com", "microsoft.com",
        "accounts.google.com", "ebay.", "aliexpress.", "netflix.com", "spotify.com", "uber.com", "airbnb.", "booking.com",
    ];
    let auto_submitted = headers.get_first_value("Auto-Submitted").map(|v| !v.trim().eq_ignore_ascii_case("no")).unwrap_or(false);
    let sending_platform = ["Feedback-ID", "X-SES-Outgoing", "X-Mailgun-Sid", "X-SG-EID", "X-MC-User", "X-Campaign", "X-Mailer-RecptId"]
        .iter()
        .any(|h| has(h));
    if auto_submitted
        || sending_platform
        || automatic_local.iter().any(|k| local.contains(k))
        || automatic_domain.iter().any(|d| domain.contains(d))
    {
        "notifications"
    } else {
        "people"
    }
}

fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    // ASCII seulement : garde les mêmes décalages d'octets que `html` (accents, emojis…).
    let lower = html.to_ascii_lowercase();
    let (mut i, bytes) = (0usize, html.as_bytes());
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Ignore complètement <style> et <script>.
            let skip_to = ["style", "script"].iter().find_map(|tag| {
                lower[i..].starts_with(&format!("<{tag}")).then(|| lower[i..].find(&format!("</{tag}>")).map(|p| i + p + tag.len() + 3))
            });
            if let Some(end) = skip_to {
                i = end.unwrap_or(bytes.len());
                continue;
            }
            let end = html[i..].find('>').map(|p| i + p + 1).unwrap_or(bytes.len());
            let tag = lower[i..end].trim_start_matches('<').trim_start_matches('/');
            if tag.starts_with("br") || tag.starts_with("p") || tag.starts_with("div") || tag.starts_with("tr") || tag.starts_with("li") {
                out.push('\n');
            }
            i = end;
        } else {
            let next = html[i..].find('<').map(|p| i + p).unwrap_or(bytes.len());
            out.push_str(&html[i..next]);
            i = next;
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    // Compacte les lignes vides multiples.
    let mut result = String::new();
    let mut blank = 0;
    for line in decoded.lines().map(str::trim) {
        if line.is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        result.push_str(line);
        result.push('\n');
    }
    result.trim().to_string()
}

/// Décode un nom de dossier IMAP (UTF-7 modifié, RFC 3501 §5.1.3) : « Messages envoy&AOk-s » → « Messages envoyés ».
pub fn decode_utf7_imap(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let Some(j) = after.find('-') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let chunk = &after[..j];
        if chunk.is_empty() {
            out.push('&');
        } else {
            let b64 = chunk.replace(',', "/");
            match base64::engine::general_purpose::STANDARD_NO_PAD.decode(b64.trim_end_matches('=')) {
                Ok(bytes) => {
                    let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                    out.push_str(&String::from_utf16_lossy(&units));
                }
                Err(_) => {
                    out.push('&');
                    out.push_str(chunk);
                    out.push('-');
                }
            }
        }
        rest = &after[j + 1..];
    }
    out.push_str(rest);
    out
}

/// Encode un nom de dossier en UTF-7 modifié pour le serveur IMAP.
pub fn encode_utf7_imap(s: &str) -> String {
    let mut out = String::new();
    let mut pending: Vec<u16> = Vec::new();
    let flush = |pending: &mut Vec<u16>, out: &mut String| {
        if pending.is_empty() {
            return;
        }
        let bytes: Vec<u8> = pending.iter().flat_map(|u| u.to_be_bytes()).collect();
        out.push('&');
        out.push_str(&base64::engine::general_purpose::STANDARD_NO_PAD.encode(bytes).replace('/', ","));
        out.push('-');
        pending.clear();
    };
    for c in s.chars() {
        if (' '..='~').contains(&c) {
            flush(&mut pending, &mut out);
            if c == '&' {
                out.push_str("&-");
            } else {
                out.push(c);
            }
        } else {
            let mut buf = [0u16; 2];
            pending.extend_from_slice(c.encode_utf16(&mut buf));
        }
    }
    flush(&mut pending, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_devient_texte() {
        let t = strip_html("<style>p{color:red}</style><p>Bonjour&nbsp;<b>Laurent</b></p><script>x()</script><div>A &amp; B</div>");
        assert_eq!(t, "Bonjour Laurent\n\nA & B");
        assert_eq!(strip_html("<p>Élève très été</p>"), "Élève très été");
    }

    #[test]
    fn tri_simple() {
        fn h(raw: &'static str) -> Vec<mailparse::MailHeader<'static>> {
            parse_headers(raw.as_bytes()).unwrap().0
        }
        let none = h("X: y\r\n\r\n");
        assert_eq!(categorize("noreply@banque.fr", &none), "notifications");
        assert_eq!(categorize("service@paypal.fr", &none), "notifications");
        assert_eq!(categorize("lastpass@lastpass.com", &none), "notifications");
        assert_eq!(categorize("shop@shop.com", &h("List-Unsubscribe: <mailto:u@x>\r\n\r\n")), "newsletters");
        assert_eq!(categorize("bob@ex.com", &h("Feedback-ID: 1:2:3\r\n\r\n")), "notifications");
        assert_eq!(categorize("marie@exemple.com", &none), "people");
    }

    #[test]
    fn pieces_jointes() {
        let raw = concat!(
            "Content-Type: multipart/mixed; boundary=B\r\n\r\n",
            "--B\r\nContent-Type: text/plain\r\n\r\nVoici le devis\r\n",
            "--B\r\nContent-Type: application/pdf; name=\"../../devis.pdf\"\r\nContent-Disposition: attachment; filename=\"../../devis.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0=\r\n",
            "--B\r\nContent-Type: image/png\r\nContent-ID: <logo>\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0KGgo=\r\n--B--\r\n"
        );
        let b = parse_body(raw.as_bytes()).unwrap();
        assert_eq!(b.attachments.len(), 1);
        assert_eq!(b.attachments[0].filename, "devis.pdf");
        assert_eq!(b.attachments[0].size, 5);
    }

    #[test]
    fn expediteur() {
        assert_eq!(sender("Marie Laurent <marie@ex.com>"), ("Marie Laurent".into(), "marie@ex.com".into()));
    }

    #[test]
    fn html_et_image_integree() {
        let raw = concat!(
            "Content-Type: multipart/related; boundary=B\r\n\r\n",
            "--B\r\nContent-Type: text/html; charset=utf-8\r\n\r\n",
            "<p>Bonjour</p><img src=\"cid:logo1\"><img src='https://pistage.example/p.gif'>\r\n",
            "--B\r\nContent-Type: image/png\r\nContent-ID: <logo1>\r\nContent-Transfer-Encoding: base64\r\n\r\n",
            "iVBORw0KGgo=\r\n--B--\r\n"
        );
        let b = parse_body(raw.as_bytes()).unwrap();
        let html = b.html.unwrap();
        assert!(html.contains("src=\"data:image/png;base64,iVBORw0KGgo="));
        assert!(b.has_remote_images);
        assert!(b.text.contains("Bonjour"));
    }

    #[test]
    fn noms_de_dossiers_utf7() {
        assert_eq!(decode_utf7_imap("[Gmail]/Messages envoy&AOk-s"), "[Gmail]/Messages envoyés");
        assert_eq!(decode_utf7_imap("Tom &- Jerry"), "Tom & Jerry");
        for name in ["Élodie", "Clients/Hôtel Été", "Tom & Jerry", "日本語", "Café ☕ 🎉", "Marie"] {
            assert_eq!(decode_utf7_imap(&encode_utf7_imap(name)), name);
        }
        assert_eq!(encode_utf7_imap("Élodie"), "&AMk-lodie");
        assert_eq!(encode_utf7_imap("Marie"), "Marie");
    }

    #[test]
    fn nom_de_dossier_controle() {
        assert_eq!(clean_folder_label("  /Marie/ ").unwrap(), "Marie");
        assert!(clean_folder_label("   ").is_err());
        assert!(clean_folder_label("INBOX").is_err());
        assert!(clean_folder_label("[Gmail]/Spam").is_err());
    }

    #[test]
    fn xoauth2_format() {
        let a = XOAuth2 { user: "a@b.c".into(), token: "tok".into() };
        let r = imap::Authenticator::process(&a, b"");
        assert_eq!(r, "user=a@b.c\x01auth=Bearer tok\x01\x01");
    }
}
