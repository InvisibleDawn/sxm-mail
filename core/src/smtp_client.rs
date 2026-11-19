//! Envoi via SMTP + XOAUTH2. Gmail et Office 365 enregistrent eux-mêmes une copie dans « Envoyés ».

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use serde::Deserialize;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{Message, SmtpTransport, Transport};

/// Comptes personnels Microsoft (Hotmail/Outlook.com/Live) et professionnels n'utilisent pas le même serveur SMTP.
fn microsoft_host(email: &str) -> &'static str {
    let domain = email.rsplit('@').next().unwrap_or("").to_lowercase();
    if ["hotmail.", "outlook.", "live.", "msn."].iter().any(|p| domain.starts_with(p)) {
        "smtp-mail.outlook.com"
    } else {
        "smtp.office365.com"
    }
}

/// Taille totale maximale des pièces jointes (limite de Gmail : 25 Mo).
pub const MAX_ATTACHMENTS: u64 = 25 * 1024 * 1024;

/// Pièce jointe à envoyer : contenu en base64 (choisie avec le sélecteur de fichiers)
/// ou chemin d'un fichier (glissé-déposé sur la fenêtre).
#[derive(Debug, Clone, Deserialize)]
pub struct OutgoingAttachment {
    pub name: String,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// Image collée dans le corps du message (HTML) : référencée par `cid:<cid>` dans le HTML.
#[derive(Debug, Clone, Deserialize)]
pub struct InlineImage {
    pub cid: String,
    pub name: String,
    /// Contenu en base64.
    pub data: String,
}

fn valid_cid(cid: &str) -> bool {
    !cid.is_empty() && cid.len() <= 100 && cid.chars().all(|c| c.is_ascii_alphanumeric() || "._-@".contains(c))
}

/// Lit le contenu des pièces jointes et contrôle la taille totale.
pub fn load_attachments(list: &[OutgoingAttachment]) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut total = 0u64;
    for a in list {
        let bytes = match (&a.data, &a.path) {
            (Some(d), _) => base64::engine::general_purpose::STANDARD.decode(d.as_bytes()).with_context(|| format!("pièce jointe illisible : {}", a.name))?,
            (None, Some(p)) => {
                let path = std::path::Path::new(p);
                let meta = std::fs::metadata(path).with_context(|| format!("fichier introuvable : {}", a.name))?;
                if !meta.is_file() {
                    bail!("« {} » n'est pas un fichier (les dossiers ne peuvent pas être joints)", a.name);
                }
                std::fs::read(path).with_context(|| format!("lecture impossible : {}", a.name))?
            }
            (None, None) => bail!("pièce jointe vide : {}", a.name),
        };
        total += bytes.len() as u64;
        if total > MAX_ATTACHMENTS {
            bail!("pièces jointes trop lourdes : 25 Mo au maximum au total (limite de Gmail)");
        }
        out.push((clean_name(&a.name), bytes));
    }
    Ok(out)
}

fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let n: String = base.chars().filter(|c| !c.is_control() && *c != '"').collect::<String>().trim().to_string();
    if n.is_empty() { "piece-jointe".into() } else { n }
}

/// Type MIME d'après l'extension (application/octet-stream par défaut).
fn mime_of(name: &str) -> ContentType {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let m = match ext.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "zip" => "application/zip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    };
    ContentType::parse(m).unwrap_or(ContentType::TEXT_PLAIN)
}

pub fn send_mail(
    provider: &str,
    from: &str,
    access_token: &str,
    to: &str,
    subject: &str,
    body: &str,
    attachments: &[OutgoingAttachment],
) -> Result<()> {
    send_mail_rich(provider, from, access_token, to, subject, body, None, &[], attachments)
}

/// Construit le message. Avec `html` (images collées dans le texte) :
/// multipart/alternative (texte brut + multipart/related HTML et images), puis les pièces jointes.
#[allow(clippy::too_many_arguments)]
pub fn build_message(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    html: Option<&str>,
    inline: &[InlineImage],
    attachments: &[OutgoingAttachment],
) -> Result<Message> {
    let mut builder = Message::builder().from(from.parse().context("adresse d'expédition invalide")?);
    let mut count = 0;
    for r in to.split(|c| c == ',' || c == ';').map(str::trim).filter(|s| !s.is_empty()) {
        builder = builder.to(r.parse().with_context(|| format!("destinataire invalide : {r}"))?);
        count += 1;
    }
    if count == 0 {
        bail!("indique au moins un destinataire");
    }
    let files = load_attachments(attachments)?;
    let mut total: u64 = files.iter().map(|(_, b)| b.len() as u64).sum();
    let builder = builder.subject(subject.trim());

    // Corps : texte seul, ou texte + HTML avec images intégrées.
    let content: Option<MultiPart> = match html.filter(|h| !h.trim().is_empty()) {
        None => None,
        Some(h) => {
            let mut related = MultiPart::related().singlepart(SinglePart::html(h.to_string()));
            for img in inline {
                if !valid_cid(&img.cid) {
                    bail!("image collée invalide");
                }
                let bytes = base64::engine::general_purpose::STANDARD.decode(img.data.as_bytes()).with_context(|| format!("image illisible : {}", img.name))?;
                total += bytes.len() as u64;
                if total > MAX_ATTACHMENTS {
                    bail!("images et pièces jointes trop lourdes : 25 Mo au maximum au total (limite de Gmail)");
                }
                let name = clean_name(&img.name);
                let ct = mime_of(&name);
                related = related.singlepart(Attachment::new_inline_with_name(img.cid.clone(), name).body(bytes, ct));
            }
            Some(MultiPart::alternative().singlepart(SinglePart::plain(body.to_string())).multipart(related))
        }
    };

    let message = match (content, files.is_empty()) {
        (None, true) => builder.header(ContentType::TEXT_PLAIN).body(body.to_string()),
        (Some(alt), true) => builder.multipart(alt),
        (content, false) => {
            let mut parts = match content {
                Some(alt) => MultiPart::mixed().multipart(alt),
                None => MultiPart::mixed().singlepart(SinglePart::plain(body.to_string())),
            };
            for (name, bytes) in files {
                let ct = mime_of(&name);
                parts = parts.singlepart(Attachment::new(name).body(bytes, ct));
            }
            builder.multipart(parts)
        }
    }
    .map_err(|e| anyhow!("message invalide : {e}"))?;
    Ok(message)
}

#[allow(clippy::too_many_arguments)]
pub fn send_mail_rich(
    provider: &str,
    from: &str,
    access_token: &str,
    to: &str,
    subject: &str,
    body: &str,
    html: Option<&str>,
    inline: &[InlineImage],
    attachments: &[OutgoingAttachment],
) -> Result<()> {
    let message = build_message(from, to, subject, body, html, inline, attachments)?;

    let transport = if provider == "microsoft" {
        SmtpTransport::starttls_relay(microsoft_host(from))?
    } else {
        SmtpTransport::relay("smtp.gmail.com")?
    };
    transport
        .credentials(Credentials::new(from.to_string(), access_token.to_string()))
        .authentication(vec![Mechanism::Xoauth2])
        .build()
        .send(&message)
        .map_err(|e| anyhow!("envoi refusé : {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serveur_microsoft() {
        assert_eq!(microsoft_host("a@hotmail.fr"), "smtp-mail.outlook.com");
        assert_eq!(microsoft_host("a@hotel.com"), "smtp.office365.com");
    }

    #[test]
    fn destinataire_obligatoire() {
        assert!(send_mail("google", "a@b.com", "t", " ; ", "s", "b", &[]).is_err());
        assert!(send_mail("google", "a@b.com", "t", "pas une adresse", "s", "b", &[]).is_err());
    }

    #[test]
    fn pieces_jointes() {
        let a = OutgoingAttachment { name: "../Facture été.pdf".into(), data: Some("SGVsbG8=".into()), path: None };
        let files = load_attachments(&[a]).unwrap();
        assert_eq!(files[0].0, "Facture été.pdf");
        assert_eq!(files[0].1, b"Hello");
        assert!(load_attachments(&[OutgoingAttachment { name: "x".into(), data: Some("%%%".into()), path: None }]).is_err());
        assert!(load_attachments(&[OutgoingAttachment { name: "d".into(), data: None, path: Some("/".into()) }]).is_err());
        assert_eq!(mime_of("photo.JPG"), ContentType::parse("image/jpeg").unwrap());
        assert_eq!(mime_of("inconnu.xyz"), ContentType::parse("application/octet-stream").unwrap());
    }

    #[test]
    fn message_avec_image_collee() {
        let img = InlineImage { cid: "img1@sxm".into(), name: "capture.png".into(), data: "iVBORw0KGgo=".into() };
        let att = OutgoingAttachment { name: "a.pdf".into(), data: Some("JVBERg==".into()), path: None };
        let m = build_message("a@b.com", "c@d.com", "s", "Bonjour [Image]", Some("<p>Bonjour <img src=\"cid:img1@sxm\"></p>"), &[img.clone()], &[att]).unwrap();
        let raw = String::from_utf8(m.formatted()).unwrap();
        for part in ["multipart/mixed", "multipart/alternative", "multipart/related", "text/html", "image/png", "Content-ID: <img1@sxm>", "application/pdf"] {
            assert!(raw.contains(part), "absent : {part}");
        }
        let m = build_message("a@b.com", "c@d.com", "s", "b", Some("<p>b</p>"), &[img], &[]).unwrap();
        assert!(!String::from_utf8(m.formatted()).unwrap().contains("multipart/mixed"));
        let bad = InlineImage { cid: "a b>".into(), name: "x.png".into(), data: "".into() };
        assert!(build_message("a@b.com", "c@d.com", "s", "b", Some("<p>b</p>"), &[bad], &[]).is_err());
        let m = build_message("a@b.com", "c@d.com", "s", "b", None, &[], &[]).unwrap();
        assert!(String::from_utf8(m.formatted()).unwrap().contains("text/plain"));
    }

    #[test]
    fn message_avec_piece_jointe() {
        let body = MultiPart::mixed()
            .singlepart(SinglePart::plain("Bonjour".to_string()))
            .singlepart(Attachment::new("Facture été.pdf".into()).body(b"%PDF".to_vec(), mime_of("x.pdf")));
        let m = Message::builder().from("a@b.com".parse().unwrap()).to("c@d.com".parse().unwrap()).subject("s").multipart(body).unwrap();
        let raw = String::from_utf8(m.formatted()).unwrap();
        assert!(raw.contains("multipart/mixed"));
        assert!(raw.contains("application/pdf"));
        assert!(raw.contains("attachment"));
    }
}
