use serde::Serialize;
use std::sync::Arc;
use sxm_core::{oauth, store, Account, MessageSummary};
use tauri::{async_runtime::spawn_blocking, State};

type Cache = Arc<oauth::TokenCache>;
type CmdResult<T> = Result<T, String>;

/// Affiche la chaîne complète des causes (« contexte : cause ») dans l'interface.
fn msg(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn join_err(e: tauri::Error) -> String {
    format!("tâche interrompue : {e}")
}

#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo { name: "SXM Mail", version: env!("CARGO_PKG_VERSION") }
}

#[tauri::command]
fn config_path() -> CmdResult<String> {
    store::config_path().map(|p| p.display().to_string()).map_err(msg)
}

#[tauri::command]
fn list_accounts() -> CmdResult<Vec<Account>> {
    store::list_accounts().map_err(msg)
}

/// Ouvre le navigateur sur la page de connexion ("google" ou "microsoft") et attend le retour (jusqu'à 5 minutes).
#[tauri::command]
async fn add_account(provider: String, cache: State<'_, Cache>) -> CmdResult<Account> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let provider = oauth::Provider::from_id(&provider)?;
        let cfg = store::load_config()?;
        let auth = oauth::authorize(provider, &cfg, |url| {
            if sxm_core::open_in_browser(url).is_err() {
                eprintln!("Impossible d'ouvrir le navigateur. Ouvre cette adresse : {url}");
            }
        })?;
        let account = store::upsert_account(provider.id(), &auth.email)?;
        store::save_refresh_token(&account.id, &auth.refresh_token)?;
        cache.put(&account.id, &auth.access_token, auth.expires_in);
        Ok::<_, anyhow::Error>(account)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

#[tauri::command]
fn remove_account(id: String, cache: State<'_, Cache>) -> CmdResult<()> {
    cache.forget(&id);
    store::remove_account(&id).map_err(msg)
}

#[tauri::command]
async fn fetch_inbox(account_id: String, cache: State<'_, Cache>) -> CmdResult<Vec<MessageSummary>> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::fetch_inbox(sxm_core::imap_host(&account.provider), &account.id, &account.email, &token, 60)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

#[tauri::command]
async fn fetch_sent(account_id: String, cache: State<'_, Cache>) -> CmdResult<Vec<MessageSummary>> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::fetch_sent(sxm_core::imap_host(&account.provider), &account.id, &account.email, &token, 40)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

/// Messages plus anciens que `before_uid` (bouton « Charger les mails plus anciens »).
#[tauri::command]
async fn fetch_older(account_id: String, mailbox: String, before_uid: u32, sent: bool, cache: State<'_, Cache>) -> CmdResult<Vec<MessageSummary>> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::fetch_older(sxm_core::imap_host(&account.provider), &account.id, &account.email, &token, &mailbox, before_uid, 40, sent)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

#[tauri::command]
async fn fetch_message(account_id: String, mailbox: String, uid: u32, cache: State<'_, Cache>) -> CmdResult<sxm_core::MessageBody> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::fetch_message(sxm_core::imap_host(&account.provider), &account.email, &token, &mailbox, uid)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

#[tauri::command]
async fn send_mail(account_id: String, to: String, subject: String, body: String, cache: State<'_, Cache>) -> CmdResult<()> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::send_mail(&account.provider, &account.email, &token, &to, &subject, &body)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

/// Change l'état d'un message dans Gmail : "delete", "archive", "star", "unstar".
#[tauri::command]
async fn message_action(account_id: String, mailbox: String, uid: u32, action: String, cache: State<'_, Cache>) -> CmdResult<()> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::message_action(sxm_core::imap_host(&account.provider), &account.email, &token, &mailbox, uid, &action)
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

/// Enregistre une pièce jointe dans Téléchargements et renvoie le chemin du fichier.
#[tauri::command]
async fn save_attachment(account_id: String, mailbox: String, uid: u32, index: usize, cache: State<'_, Cache>) -> CmdResult<String> {
    let cache = cache.inner().clone();
    spawn_blocking(move || {
        let cfg = store::load_config()?;
        let account = store::get_account(&account_id)?;
        let token = cache.access_token(&cfg, &account)?;
        sxm_core::save_attachment(sxm_core::imap_host(&account.provider), &account.email, &token, &mailbox, uid, index)
            .map(|p| p.display().to_string())
    })
    .await
    .map_err(join_err)?
    .map_err(msg)
}

#[tauri::command]
fn open_url(url: String) -> CmdResult<()> {
    sxm_core::open_url(&url).map_err(msg)
}

#[tauri::command]
fn cache_load() -> CmdResult<Option<String>> {
    store::load_cache().map_err(msg)
}

#[tauri::command]
fn cache_save(data: String) -> CmdResult<()> {
    store::save_cache(&data).map_err(msg)
}

pub fn run() {
    tauri::Builder::default()
        .manage::<Cache>(Arc::new(oauth::TokenCache::default()))
        .invoke_handler(tauri::generate_handler![
            app_info,
            config_path,
            list_accounts,
            add_account,
            remove_account,
            fetch_inbox,
            fetch_message,
            fetch_sent,
            fetch_older,
            message_action,
            save_attachment,
            open_url,
            cache_load,
            cache_save,
            send_mail
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de SXM Mail");
}
