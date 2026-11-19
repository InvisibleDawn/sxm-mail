//! Mises à jour à partir des versions publiées sur la page Releases du dépôt GitHub officiel.
//! Le fichier téléchargé est vérifié en SHA256 (empreinte publiée par GitHub) avant toute installation.

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const RELEASES_API: &str = "https://api.github.com/repos/InvisibleDawn/sxm-mail/releases/latest";
const OFFICIAL_PREFIX: &str = "https://github.com/InvisibleDawn/sxm-mail/releases/download/";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub version: String,
    /// Notes de version (Markdown : titres ### et puces -).
    pub notes: String,
    pub page: String,
    pub asset_url: String,
    pub asset_name: String,
    /// Empreinte SHA256 publiée par GitHub (sans le préfixe « sha256: »).
    pub digest: Option<String>,
    pub size: u64,
}

pub fn parse_version(v: &str) -> Vec<u64> {
    let mut parts: Vec<u64> = v.trim().trim_start_matches('v').split(['.', '-', '+']).take(3).map(|p| p.parse().unwrap_or(0)).collect();
    parts.resize(3, 0);
    parts
}

/// Fichier d'installation adapté au système en cours.
pub fn wanted_asset(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    if cfg!(target_os = "macos") {
        n.ends_with(".dmg")
    } else if cfg!(target_os = "linux") {
        n.ends_with(".x86_64.rpm") && cfg!(target_arch = "x86_64")
    } else {
        false
    }
}

/// Seuls les fichiers publiés sur le dépôt officiel sont acceptés.
pub fn official_url(url: &str) -> bool {
    url.starts_with(OFFICIAL_PREFIX) && !url.contains("..")
}

/// Nouvelle version publiée, si elle est plus récente que `current`.
pub fn check(current: &str) -> Result<Option<UpdateInfo>> {
    let rel: serde_json::Value = ureq::get(RELEASES_API)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "SXM-Mail")
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .map_err(|e| anyhow!("vérification impossible : {e}"))?
        .into_json()?;
    let version = rel["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    if version.is_empty() || parse_version(&version) <= parse_version(current) {
        return Ok(None);
    }
    let asset = rel["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str().is_some_and(wanted_asset)))
        .ok_or_else(|| anyhow!("la version {version} n'a pas encore de fichier pour ce système"))?;
    let asset_url = asset["browser_download_url"].as_str().unwrap_or_default().to_string();
    if !official_url(&asset_url) {
        bail!("adresse de mise à jour non officielle");
    }
    Ok(Some(UpdateInfo {
        current: current.to_string(),
        version,
        notes: rel["body"].as_str().unwrap_or_default().to_string(),
        page: rel["html_url"].as_str().unwrap_or_default().to_string(),
        asset_name: asset["name"].as_str().unwrap_or_default().to_string(),
        digest: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_lowercase),
        size: asset["size"].as_u64().unwrap_or(0),
        asset_url,
    }))
}

/// Télécharge le fichier dans un dossier temporaire et vérifie son empreinte.
/// `progress(fait, total)` est appelé pendant le téléchargement.
pub fn download(url: &str, name: &str, digest: Option<&str>, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf> {
    if !official_url(url) || name.contains(['/', '\\']) || name.starts_with('.') || !wanted_asset(name) {
        bail!("mise à jour refusée : fichier non officiel");
    }
    let Some(expected) = digest.filter(|d| d.len() == 64 && d.chars().all(|c| c.is_ascii_hexdigit())) else {
        bail!("mise à jour refusée : empreinte SHA256 absente");
    };
    let dir = std::env::temp_dir().join("sxm-mail-update");
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(name);
    let _ = std::fs::remove_file(&file);
    let resp = ureq::get(url).set("User-Agent", "SXM-Mail").call().map_err(|e| anyhow!("téléchargement impossible : {e}"))?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut reader = resp.into_reader();
    let mut out = std::fs::File::create(&file).context("création du fichier temporaire")?;
    let mut hasher = Sha256::new();
    let (mut done, mut buf, mut last) = (0u64, vec![0u8; 64 * 1024], 0u64);
    loop {
        let n = reader.read(&mut buf).context("téléchargement interrompu")?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
        done += n as u64;
        if done - last >= 256 * 1024 {
            last = done;
            progress(done, total);
        }
    }
    out.flush()?;
    progress(done, total);
    let got = hex(&hasher.finalize());
    if !got.eq_ignore_ascii_case(expected) {
        let _ = std::fs::remove_file(&file);
        bail!("le fichier téléchargé est altéré (empreinte SHA256 différente) : mise à jour annulée");
    }
    Ok(file)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Installe le fichier téléchargé. Renvoie "installed" (redémarrage nécessaire).
/// - Fedora : `pkexec dnf install` (le mot de passe est demandé par le système) ;
/// - macOS : l'application est remplacée par celle du .dmg (dans le même dossier).
pub fn install(file: &Path) -> Result<&'static str> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("pkexec")
            .args(["dnf", "install", "-y"])
            .arg(file)
            .output()
            .context("lancement de dnf")?;
        let _ = std::fs::remove_file(file);
        if !out.status.success() {
            let log = String::from_utf8_lossy(&out.stderr).trim().to_string();
            bail!(if log.is_empty() { "installation annulée".to_string() } else { format!("échec de l'installation : {log}") });
        }
        return Ok("installed");
    }
    #[cfg(target_os = "macos")]
    {
        return mac::install_dmg(file);
    }
    #[allow(unreachable_code)]
    {
        let _ = file;
        bail!("mise à jour automatique non disponible sur ce système")
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::process::Command;

    /// Dossier « SXM Mail.app » de l'application en cours d'exécution.
    fn current_bundle() -> Result<PathBuf> {
        let exe = std::env::current_exe()?;
        exe.ancestors()
            .find(|p| p.extension().is_some_and(|e| e == "app"))
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("application introuvable (lancée hors d'un dossier .app)"))
    }

    pub fn install_dmg(dmg: &Path) -> Result<&'static str> {
        let target = current_bundle()?;
        let mount = std::env::temp_dir().join(format!("sxm-mail-dmg-{}", std::process::id()));
        std::fs::create_dir_all(&mount)?;
        let ok = Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"])
            .arg(&mount)
            .arg(dmg)
            .status()
            .context("ouverture du .dmg")?
            .success();
        if !ok {
            bail!("impossible d'ouvrir le fichier .dmg");
        }
        let result = (|| {
            let app = std::fs::read_dir(&mount)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .find(|p| p.extension().is_some_and(|e| e == "app"))
                .ok_or_else(|| anyhow!("application absente du .dmg"))?;
            // Copie à côté, puis échange : l'ancienne version reste intacte si la copie échoue.
            let staged = target.with_extension("app.new");
            let old = target.with_extension("app.old");
            let _ = std::fs::remove_dir_all(&staged);
            let _ = std::fs::remove_dir_all(&old);
            let copied = Command::new("ditto").arg(&app).arg(&staged).status().context("copie de la nouvelle version")?.success();
            if !copied {
                let _ = std::fs::remove_dir_all(&staged);
                bail!("copie impossible : vérifie que SXM Mail est dans un dossier où tu peux écrire (Applications)");
            }
            std::fs::rename(&target, &old).context("remplacement de l'ancienne version")?;
            if let Err(e) = std::fs::rename(&staged, &target) {
                let _ = std::fs::rename(&old, &target);
                return Err(anyhow!("remplacement impossible : {e}"));
            }
            let _ = std::fs::remove_dir_all(&old);
            Ok("installed")
        })();
        let _ = Command::new("hdiutil").args(["detach", "-quiet"]).arg(&mount).status();
        let _ = std::fs::remove_dir(&mount);
        let _ = std::fs::remove_file(dmg);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(parse_version("0.3.0") > parse_version("0.2.1"));
        assert!(parse_version("v1.0") > parse_version("0.9.9"));
        assert!(parse_version("0.3.0") == parse_version("v0.3"));
        assert!(parse_version("0.10.0") > parse_version("0.9.0"));
    }

    #[test]
    fn adresses_officielles() {
        assert!(official_url("https://github.com/InvisibleDawn/sxm-mail/releases/download/v0.3.0/SXM.Mail_0.3.0_universal.dmg"));
        assert!(!official_url("https://github.com/autre/sxm-mail/releases/download/v1/x.dmg"));
        assert!(!official_url("https://github.com/InvisibleDawn/sxm-mail/releases/download/../../x"));
        assert!(!official_url("http://github.com/InvisibleDawn/sxm-mail/releases/download/v1/x.dmg"));
    }

    #[test]
    fn refuse_sans_empreinte() {
        let url = "https://github.com/InvisibleDawn/sxm-mail/releases/download/v9/SXM.Mail-9.0.0-1.x86_64.rpm";
        assert!(download(url, "SXM.Mail-9.0.0-1.x86_64.rpm", None, |_, _| {}).is_err());
        assert!(download(url, "../x.rpm", Some(&"a".repeat(64)), |_, _| {}).is_err());
    }
}
