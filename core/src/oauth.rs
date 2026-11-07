//! OAuth2 pour application de bureau (Google et Microsoft) : code d'autorisation + PKCE,
//! redirection sur un serveur local éphémère, rafraîchissement des jetons.

use crate::store::{self, Account, Config};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Google,
    Microsoft,
}

impl Provider {
    pub fn id(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Microsoft => "microsoft",
        }
    }
    pub fn from_id(s: &str) -> Result<Self> {
        match s {
            "google" => Ok(Provider::Google),
            "microsoft" => Ok(Provider::Microsoft),
            other => bail!("fournisseur inconnu : {other}"),
        }
    }
    fn auth_url(self) -> &'static str {
        match self {
            Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            // « common » accepte les comptes professionnels (Office 365) et personnels (Hotmail/Outlook.com).
            Provider::Microsoft => "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
        }
    }
    fn token_url(self) -> &'static str {
        match self {
            Provider::Google => "https://oauth2.googleapis.com/token",
            Provider::Microsoft => "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        }
    }
    fn scope(self) -> &'static str {
        match self {
            // IMAP/SMTP chez Google exigent le scope complet.
            Provider::Google => "https://mail.google.com/ openid email",
            Provider::Microsoft => "https://outlook.office.com/IMAP.AccessAsUser.All https://outlook.office.com/SMTP.Send offline_access openid email profile",
        }
    }
    /// Microsoft n'accepte que « localhost » pour la redirection (le port est ignoré), Google préfère 127.0.0.1.
    fn redirect_host(self) -> &'static str {
        match self {
            Provider::Google => "127.0.0.1",
            Provider::Microsoft => "localhost",
        }
    }
    fn creds(self, cfg: &Config) -> Result<(String, Option<String>)> {
        let path = || store::config_path().map(|p| p.display().to_string()).unwrap_or_else(|_| "~/.config/sxm-mail/config.json".into());
        let (id, secret, key) = match self {
            Provider::Google => (&cfg.google_client_id, &cfg.google_client_secret, "google_client_id"),
            Provider::Microsoft => (&cfg.microsoft_client_id, &None, "microsoft_client_id"),
        };
        let id = id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("« {key} » manquant dans {} (voir docs/GOOGLE.md et docs/MICROSOFT.md)", path()))?;
        Ok((id.to_string(), secret.clone().filter(|s| !s.is_empty())))
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: u64,
}

pub struct Authorized {
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

fn random_string(n: usize) -> String {
    rand::thread_rng().sample_iter(&Alphanumeric).take(n).map(char::from).collect()
}

fn http_error(e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, resp) => anyhow!("le serveur de connexion a répondu {code} : {}", resp.into_string().unwrap_or_default()),
        other => anyhow!("erreur réseau : {other}"),
    }
}

/// Adresse e-mail lue dans l'id_token (reçu directement du serveur de jetons en TLS : pas de vérification de signature nécessaire).
pub fn email_from_id_token(id_token: &str) -> Result<String> {
    let payload = id_token.split('.').nth(1).ok_or_else(|| anyhow!("id_token invalide"))?;
    let claims: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload.trim_end_matches('='))?)?;
    claims["email"]
        .as_str()
        .or_else(|| claims["preferred_username"].as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("adresse e-mail absente de la réponse"))
}

/// Flux complet. `open_browser` reçoit l'URL à ouvrir (navigateur par défaut, ouvert côté application).
pub fn authorize(provider: Provider, cfg: &Config, open_browser: impl Fn(&str)) -> Result<Authorized> {
    let (client_id, client_secret) = provider.creds(cfg)?;

    let v4 = TcpListener::bind("127.0.0.1:0").context("ouverture du port local")?;
    let port = v4.local_addr()?.port();
    let mut listeners = vec![v4];
    // « localhost » peut se résoudre en IPv6 : on écoute aussi sur ::1 si possible.
    if let Ok(v6) = TcpListener::bind(format!("[::1]:{port}")) {
        listeners.push(v6);
    }
    for l in &listeners {
        l.set_nonblocking(true)?;
    }
    let redirect = format!("http://{}:{port}", provider.redirect_host());

    let verifier = random_string(96);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_string(32);

    let mut url = url::Url::parse(provider.auth_url())?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", &client_id)
            .append_pair("redirect_uri", &redirect)
            .append_pair("response_type", "code")
            .append_pair("scope", provider.scope())
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state);
        match provider {
            Provider::Google => q.append_pair("access_type", "offline").append_pair("prompt", "consent"), // garantit un refresh_token
            Provider::Microsoft => q.append_pair("prompt", "select_account"),
        };
    }
    open_browser(url.as_str());

    let code = wait_for_code(&listeners, &state)?;

    let mut form = vec![
        ("code", code.as_str()),
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("grant_type", "authorization_code"),
        ("code_verifier", verifier.as_str()),
    ];
    if let Some(s) = client_secret.as_deref() {
        form.push(("client_secret", s));
    }
    if provider == Provider::Microsoft {
        form.push(("scope", provider.scope()));
    }
    let tokens: TokenResponse = ureq::post(provider.token_url()).send_form(&form).map_err(http_error)?.into_json()?;
    let refresh_token = tokens
        .refresh_token
        .ok_or_else(|| anyhow!("aucun refresh token reçu (révoque l'accès de l'application dans ton compte puis réessaie)"))?;
    let email = email_from_id_token(tokens.id_token.as_deref().ok_or_else(|| anyhow!("id_token absent"))?)?;

    Ok(Authorized { email, access_token: tokens.access_token, refresh_token, expires_in: tokens.expires_in })
}

fn wait_for_code(listeners: &[TcpListener], expected_state: &str) -> Result<String> {
    let start = Instant::now();
    loop {
        if start.elapsed() > LOGIN_TIMEOUT {
            bail!("connexion expirée (5 minutes) : réessaie");
        }
        let mut idle = true;
        for listener in listeners {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    idle = false;
                    stream.set_nonblocking(false)?;
                    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");
                    let parsed = url::Url::parse(&format!("http://localhost{path}"))?;
                    let q: HashMap<_, _> = parsed.query_pairs().into_owned().collect();

                    // Les requêtes parasites (favicon…) sont ignorées.
                    if !q.contains_key("code") && !q.contains_key("error") {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        continue;
                    }
                    let (ok, msg) = if let Some(err) = q.get("error") {
                        (false, format!("Connexion refusée : {err} {}", q.get("error_description").cloned().unwrap_or_default()))
                    } else if q.get("state").map(String::as_str) != Some(expected_state) {
                        (false, "Réponse invalide (state différent).".to_string())
                    } else {
                        (true, "Connexion réussie. Tu peux fermer cet onglet et revenir à SXM Mail.".to_string())
                    };
                    let html = format!(
                        "<!doctype html><meta charset=utf-8><title>SXM Mail</title><body style=\"font-family:system-ui;text-align:center;margin-top:20vh\"><h2>{}</h2>",
                        msg.replace('&', "&amp;").replace('<', "&lt;")
                    );
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    );
                    if !ok {
                        bail!(msg);
                    }
                    return q.get("code").cloned().ok_or_else(|| anyhow!("code d'autorisation absent"));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
        }
        if idle {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

fn refresh(provider: Provider, cfg: &Config, refresh_token: &str) -> Result<TokenResponse> {
    let (client_id, client_secret) = provider.creds(cfg)?;
    let mut form = vec![("client_id", client_id.as_str()), ("refresh_token", refresh_token), ("grant_type", "refresh_token")];
    if let Some(s) = client_secret.as_deref() {
        form.push(("client_secret", s));
    }
    if provider == Provider::Microsoft {
        form.push(("scope", provider.scope()));
    }
    Ok(ureq::post(provider.token_url()).send_form(&form).map_err(http_error)?.into_json()?)
}

/// Cache mémoire des access tokens (~1 h) ; renouvelés avec le refresh token du trousseau.
#[derive(Default)]
pub struct TokenCache(Mutex<HashMap<String, (String, Instant)>>);

impl TokenCache {
    pub fn put(&self, account_id: &str, token: &str, expires_in: u64) {
        let expiry = Instant::now() + Duration::from_secs(expires_in.saturating_sub(60));
        self.0.lock().unwrap().insert(account_id.to_string(), (token.to_string(), expiry));
    }

    pub fn forget(&self, account_id: &str) {
        self.0.lock().unwrap().remove(account_id);
    }

    pub fn access_token(&self, cfg: &Config, account: &Account) -> Result<String> {
        if let Some((tok, expiry)) = self.0.lock().unwrap().get(&account.id) {
            if Instant::now() < *expiry {
                return Ok(tok.clone());
            }
        }
        let provider = Provider::from_id(&account.provider)?;
        let stored = store::load_refresh_token(&account.id)?;
        let fresh = refresh(provider, cfg, &stored)
            .context("renouvellement du jeton impossible (accès révoqué ou expiré ? ajoute à nouveau le compte)")?;
        // Microsoft fait tourner les refresh tokens : on conserve le plus récent.
        if let Some(new_refresh) = fresh.refresh_token.as_deref().filter(|t| *t != stored) {
            store::save_refresh_token(&account.id, new_refresh)?;
        }
        self.put(&account.id, &fresh.access_token, fresh.expires_in);
        Ok(fresh.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_dans_id_token() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"preferred_username":"laurent@hotmail.com"}"#);
        assert_eq!(email_from_id_token(&format!("x.{payload}.y")).unwrap(), "laurent@hotmail.com");
        let payload = URL_SAFE_NO_PAD.encode(br#"{"email":"l@gmail.com","preferred_username":"autre"}"#);
        assert_eq!(email_from_id_token(&format!("x.{payload}.y")).unwrap(), "l@gmail.com");
    }
}
