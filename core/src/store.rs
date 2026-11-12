//! Stockage local : config.json, accounts.json (dans ~/.config/sxm-mail/)
//! et refresh tokens dans le trousseau système (Secret Service : GNOME Keyring / KWallet).

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SERVICE: &str = "sxm-mail";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    /// "google" pour l'instant ; "microsoft" à l'étape 3.
    pub provider: String,
    pub email: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub google_client_id: Option<String>,
    /// Les clients « application de bureau » de Google exigent un secret, même s'il n'est pas confidentiel.
    pub google_client_secret: Option<String>,
    /// Application Azure de type « client public » : pas de secret.
    pub microsoft_client_id: Option<String>,
}

pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("dossier de configuration introuvable"))?
        .join("sxm-mail");
    private_dir(&dir).context("création du dossier de configuration")?;
    Ok(dir)
}

/// Crée le dossier si besoin et le réserve à l'utilisateur (droits 700) :
/// les autres comptes de la machine ne peuvent ni lister ni lire son contenu.
fn private_dir(dir: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

/// Écriture atomique d'un fichier lisible par l'utilisateur seul (droits 600).
fn write_private(path: &std::path::Path, data: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        // Un ancien fichier temporaire aurait pu garder des droits plus larges.
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.json"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    // config.json contient le secret client Google : on le remet en 600 s'il a été créé trop ouvert.
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    let raw = std::fs::read_to_string(&path).with_context(|| format!("lecture de {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("{} n'est pas un JSON valide", path.display()))
}

fn accounts_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("accounts.json"))
}

pub fn list_accounts() -> Result<Vec<Account>> {
    let path = accounts_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn save_accounts(list: &[Account]) -> Result<()> {
    write_private(&accounts_path()?, &serde_json::to_vec_pretty(list)?)
}

/// Ajoute le compte, ou met à jour celui qui a la même adresse.
pub fn upsert_account(provider: &str, email: &str) -> Result<Account> {
    let mut list = list_accounts()?;
    if let Some(a) = list.iter().find(|a| a.provider == provider && a.email.eq_ignore_ascii_case(email)) {
        return Ok(a.clone());
    }
    let account = Account {
        id: format!("{}-{}", provider, email.to_lowercase()),
        provider: provider.into(),
        email: email.into(),
    };
    list.push(account.clone());
    save_accounts(&list)?;
    Ok(account)
}

pub fn remove_account(id: &str) -> Result<()> {
    let mut list = list_accounts()?;
    list.retain(|a| a.id != id);
    save_accounts(&list)?;
    // Un jeton déjà absent du trousseau n'est pas une erreur.
    if let Ok(entry) = keyring::Entry::new(SERVICE, id) {
        let _ = entry.delete_credential();
    }
    Ok(())
}

pub fn get_account(id: &str) -> Result<Account> {
    list_accounts()?
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| anyhow!("compte inconnu : {id}"))
}

pub fn save_refresh_token(account_id: &str, token: &str) -> Result<()> {
    keyring::Entry::new(SERVICE, account_id)?
        .set_password(token)
        .context("enregistrement du jeton dans le trousseau système (GNOME Keyring / KWallet démarré ?)")
}

pub fn load_refresh_token(account_id: &str) -> Result<String> {
    keyring::Entry::new(SERVICE, account_id)?
        .get_password()
        .context("jeton introuvable dans le trousseau : ajoute à nouveau le compte")
}

/// Cache de la liste des mails (démarrage instantané). Fichier lisible par l'utilisateur seul.
fn cache_path() -> Result<PathBuf> {
    let dir = dirs::cache_dir().ok_or_else(|| anyhow!("dossier de cache introuvable"))?.join("sxm-mail");
    private_dir(&dir)?;
    Ok(dir.join("messages.json"))
}

const MAX_CACHE: usize = 20_000_000;

pub fn load_cache() -> Result<Option<String>> {
    let path = cache_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    Ok((raw.len() <= MAX_CACHE).then_some(raw))
}

pub fn save_cache(data: &str) -> Result<()> {
    if data.len() > MAX_CACHE {
        anyhow::bail!("cache trop volumineux");
    }
    serde_json::from_str::<serde_json::Value>(data).context("cache invalide")?;
    write_private(&cache_path()?, data.as_bytes())
}
