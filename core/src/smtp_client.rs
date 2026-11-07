//! Envoi via SMTP + XOAUTH2. Gmail et Office 365 enregistrent eux-mêmes une copie dans « Envoyés ».

use anyhow::{anyhow, bail, Context, Result};
use lettre::message::header::ContentType;
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

pub fn send_mail(provider: &str, from: &str, access_token: &str, to: &str, subject: &str, body: &str) -> Result<()> {
    let mut builder = Message::builder().from(from.parse().context("adresse d'expédition invalide")?);
    let mut count = 0;
    for r in to.split(|c| c == ',' || c == ';').map(str::trim).filter(|s| !s.is_empty()) {
        builder = builder.to(r.parse().with_context(|| format!("destinataire invalide : {r}"))?);
        count += 1;
    }
    if count == 0 {
        bail!("indique au moins un destinataire");
    }
    let message = builder
        .subject(subject.trim())
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| anyhow!("message invalide : {e}"))?;

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
        assert!(send_mail("google", "a@b.com", "t", " ; ", "s", "b").is_err());
        assert!(send_mail("google", "a@b.com", "t", "pas une adresse", "s", "b").is_err());
    }
}
