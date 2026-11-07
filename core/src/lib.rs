//! Cœur de SXM Mail, indépendant de l'interface.
//! - `store`   : configuration, liste des comptes, refresh tokens (trousseau système)
//! - `oauth`   : OAuth2 Google et Microsoft (PKCE, redirection locale), rafraîchissement
//! - `imap_client` : lecture de l'Inbox via IMAP + XOAUTH2

pub mod imap_client;
pub mod oauth;
pub mod smtp_client;
pub mod store;

pub use imap_client::{fetch_inbox, fetch_message, fetch_older, fetch_sent, imap_host, message_action, save_attachment, MessageBody, MessageSummary};
pub use smtp_client::send_mail;
pub use store::{Account, Config};

/// Ouvre un lien d'un mail dans le navigateur par défaut (http/https uniquement, sans passer par un shell).
pub fn open_url(raw: &str) -> anyhow::Result<()> {
    let u = url::Url::parse(raw.trim())?;
    if !matches!(u.scheme(), "http" | "https") {
        anyhow::bail!("type de lien non pris en charge : {}", u.scheme());
    }
    open_in_browser(u.as_str())
}

/// Ouvre une adresse avec le navigateur par défaut du système (argument passé directement, sans shell).
pub fn open_in_browser(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    const OPENER: &str = "open";
    #[cfg(not(target_os = "macos"))]
    const OPENER: &str = "xdg-open";
    std::process::Command::new(OPENER).arg(url).spawn()?;
    Ok(())
}
