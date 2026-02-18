mod sync;
mod supabase_sync;

use serde::{Deserialize, Serialize};
use sync::{load_config, save_config, sync_pull, sync_push, SyncConfig, SyncTarget};

#[derive(Debug, Serialize, Deserialize)]
struct SyncResult {
    ok: bool,
    message: String,
}

#[tauri::command]
fn get_config() -> Result<SyncConfig, String> {
    load_config()
}

#[tauri::command]
fn set_config(config: SyncConfig) -> Result<(), String> {
    save_config(&config)
}

#[tauri::command]
fn get_sync_status() -> Result<sync::SyncStatus, String> {
    let config = load_config()?;
    Ok(sync::get_sync_status(&config))
}

#[tauri::command]
fn do_sync_pull(target: Option<SyncTarget>, force: Option<bool>) -> Result<SyncResult, String> {
    let config = load_config()?;
    let target = target.unwrap_or(SyncTarget::Both);
    let force = force.unwrap_or(false);
    let message = sync_pull(&config, target, force)?;
    Ok(SyncResult { ok: true, message })
}

#[tauri::command]
fn do_sync_push(target: Option<SyncTarget>, force: Option<bool>) -> Result<SyncResult, String> {
    let config = load_config()?;
    let target = target.unwrap_or(SyncTarget::Both);
    let force = force.unwrap_or(false);
    let message = sync_push(&config, target, force)?;
    Ok(SyncResult { ok: true, message })
}

#[tauri::command]
fn get_startup_folder() -> Result<String, String> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| "APPDATA not set".to_string())?;
        let path = std::path::Path::new(&appdata)
            .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        Ok(path.to_string_lossy().to_string())
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        Ok(std::path::Path::new(&home)
            .join(".config/autostart")
            .to_string_lossy()
            .to_string())
    }
}

#[tauri::command]
fn open_startup_folder() -> Result<(), String> {
    let path = get_startup_folder()?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(&path).spawn().map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "macos"))]
        std::process::Command::new("xdg-open").arg(&path).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            get_sync_status,
            do_sync_pull,
            do_sync_push,
            get_startup_folder,
            open_startup_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
