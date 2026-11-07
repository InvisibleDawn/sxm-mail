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

/// Les `limit` derniers messages envoyés. Le dossier est trouvé par son attribut \Sent
/// (son nom dépend de la langue du compte Gmail : « Sent Mail », « Messages envoyés »…).
pub fn fetch_sent(host: &str, account_id: &str, email: &str, access_token: &str, limit: u32) -> Result<Vec<MessageSummary>> {
    let mut session = open_session(host, email, access_token)?;
    let name = special_folder(&mut session, "\\Sent", &["[Gmail]/Sent Mail", "[Gmail]/Messages envoyés"])?;
    let out = match name {
        Some(n) => fetch_mailbox(&mut session, account_id, &n, limit, true),
        None => Err(anyhow!("dossier des messages envoyés introuvable")),
    };
    let _ = session.logout();
    out
}

/// Dossier repéré par son attribut spécial (\\Sent, \\Trash, \\All…) : son nom dépend de la langue du compte.
fn special_folder(session: &mut Session, attr: &str, fallbacks: &[&str]) -> Result<Option<String>> {
    let names = session.list(None, Some("*")).context("liste des dossiers")?;
    let by_attr = names
        .iter()
        .find(|n| n.attributes().iter().any(|a| matches!(a, imap::types::NameAttribute::Custom(s) if s.eq_ignore_ascii_case(attr))));
    Ok(by_attr
        .or_else(|| names.iter().find(|n| fallbacks.iter().any(|f| n.name().eq_ignore_ascii_case(f))))
        .map(|n| n.name().to_string()))
}

/// Actions synchronisées avec Gmail : "delete" (corbeille), "archive" (retire de la boîte de réception), "star", "unstar".
pub fn message_action(host: &str, email: &str, access_token: &str, mailbox: &str, uid: u32, action: &str) -> Result<()> {
    let mut session = open_session(host, email, access_token)?;
    let result = (|| {
        let target = match action {
            "delete" => Some(special_folder(&mut session, "\\Trash", &["[Gmail]/Trash", "[Gmail]/Corbeille"])?.ok_or_else(|| anyhow!("corbeille introuvable"))?),
            "archive" => Some(
                special_folder(&mut session, "\\All", &["[Gmail]/All Mail", "[Gmail]/Tous les messages"])?.ok_or_else(|| anyhow!("dossier « Tous les messages » introuvable"))?,
            ),
            "star" | "unstar" => None,
            other => bail!("action inconnue : {other}"),
        };
        session.select(mailbox).with_context(|| format!("ouverture du dossier {mailbox}"))?;
        match (action, target) {
            ("star", _) => session.uid_store(uid.to_string(), "+FLAGS (\\Flagged)").map(|_| ())?,
            ("unstar", _) => session.uid_store(uid.to_string(), "-FLAGS (\\Flagged)").map(|_| ())?,
            (_, Some(t)) => session.uid_mv(uid.to_string(), &t).context("déplacement du message")?,
            _ => {}
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

/// Les `limit` messages précédant `before_uid` dans un dossier (pour « Charger plus »).
pub fn fetch_older(
    host: &str,
    account_id: &str,
    email: &str,
    access_token: &str,
    mailbox_name: &str,
    before_uid: u32,
    limit: usize,
    sent: bool,
) -> Result<Vec<MessageSummary>> {
    if before_uid <= 1 {
        return Ok(vec![]);
    }
    let mut session = open_session(host, email, access_token)?;
    let result = (|| {
        session.examine(mailbox_name).with_context(|| format!("ouverture du dossier {mailbox_name}"))?;
        let mut uids: Vec<u32> = session.uid_search(format!("UID 1:{}", before_uid - 1))?.into_iter().filter(|u| *u < before_uid).collect();
        uids.sort_unstable();
        let older: Vec<String> = uids.iter().rev().take(limit).map(|u| u.to_string()).collect();
        if older.is_empty() {
            return Ok(vec![]);
        }
        let fetched = session.uid_fetch(older.join(","), "(UID FLAGS BODY.PEEK[HEADER])").context("lecture des en-têtes")?;
        Ok(summaries(&fetched, account_id, mailbox_name, sent))
    })();
    let _ = session.logout();
    result
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
    fn xoauth2_format() {
        let a = XOAuth2 { user: "a@b.c".into(), token: "tok".into() };
        let r = imap::Authenticator::process(&a, b"");
        assert_eq!(r, "user=a@b.c\x01auth=Bearer tok\x01\x01");
    }
}
