//! Sync logic: compare folder mtimes and copy save/mod folders to/from cloud.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Which part to sync: save only, mods only, or both.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncTarget {
    Save,
    Mods,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncConfig {
    /// Path to the Schedule I save folder (e.g. .../Saves/ID)
    pub save_path: Option<String>,
    /// Path to the Schedule I mods folder
    pub mods_path: Option<String>,
    /// Path to the cloud folder (e.g. Google Drive) – bruges kun hvis Supabase ikke er sat
    pub cloud_path: Option<String>,
    /// Supabase project URL (fx https://xxx.supabase.co)
    pub supabase_url: Option<String>,
    /// Supabase anon key eller service_role key
    pub supabase_key: Option<String>,
    /// Bucket-navn i Supabase Storage
    pub bucket_name: Option<String>,
}

pub(crate) fn get_latest_mtime_recursive(path: &Path) -> std::io::Result<SystemTime> {
    let meta = fs::metadata(path)?;
    let mut latest = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if meta.is_dir() {
        for e in fs::read_dir(path)? {
            let e = e?;
            let p = e.path();
            if let Ok(t) = get_latest_mtime_recursive(&p) {
                if t > latest {
                    latest = t;
                }
            }
        }
    }
    Ok(latest)
}

/// Copy directory recursively. Creates destination dir. Overwrites existing files.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let name = e.file_name();
        let src_p = src.join(&name);
        let dst_p = dst.join(&name);
        if e.file_type()?.is_dir() {
            copy_dir_all(&src_p, &dst_p)?;
        } else {
            fs::copy(&src_p, &dst_p)?;
        }
    }
    Ok(())
}

/// Remove directory contents and the directory itself, then recreate empty dir (so we can replace with cloud version).
pub(crate) fn clear_dir(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

/// Copy content of src into dst (dst is cleared first if it exists). So "merge" is: clear dst, then copy src -> dst.
fn replace_dir_with(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    clear_dir(dst)?;
    if src.is_dir() {
        for e in fs::read_dir(src)? {
            let e = e?;
            let name = e.file_name();
            copy_dir_all(&src.join(&name), &dst.join(&name))?;
        }
    }
    Ok(())
}

pub fn sync_pull(config: &SyncConfig, target: SyncTarget) -> Result<String, String> {
    if crate::supabase_sync::use_supabase(config) {
        return crate::supabase_sync::sync_pull_supabase(config, target);
    }
    let save_path = config
        .save_path
        .as_ref()
        .ok_or("Save path is not set")?;
    let mods_path = config.mods_path.as_ref().ok_or("Mods path is not set")?;
    let cloud_path = config.cloud_path.as_ref().ok_or("Cloud path is not set")?;

    let cloud_save = Path::new(cloud_path).join("Save");
    let cloud_mods = Path::new(cloud_path).join("Mods");
    let local_save = Path::new(save_path);
    let local_mods = Path::new(mods_path);

    let mut messages = Vec::new();

    if target == SyncTarget::Save || target == SyncTarget::Both {
        if cloud_save.exists() {
            let cloud_t = get_latest_mtime_recursive(&cloud_save).map_err(|e| e.to_string())?;
            let local_t = if local_save.exists() {
                get_latest_mtime_recursive(local_save).unwrap_or(SystemTime::UNIX_EPOCH)
            } else {
                SystemTime::UNIX_EPOCH
            };
            if cloud_t > local_t {
                replace_dir_with(&cloud_save, local_save).map_err(|e| e.to_string())?;
                messages.push("Save fetched from cloud.");
            }
        }
    }

    if target == SyncTarget::Mods || target == SyncTarget::Both {
        if cloud_mods.exists() {
            let cloud_t = get_latest_mtime_recursive(&cloud_mods).map_err(|e| e.to_string())?;
            let local_t = if local_mods.exists() {
                get_latest_mtime_recursive(local_mods).unwrap_or(SystemTime::UNIX_EPOCH)
            } else {
                SystemTime::UNIX_EPOCH
            };
            if cloud_t > local_t {
                replace_dir_with(&cloud_mods, local_mods).map_err(|e| e.to_string())?;
                messages.push("Mods fetched from cloud.");
            }
        }
    }

    if messages.is_empty() {
        Ok("Nothing new to fetch – you already have the latest version.".to_string())
    } else {
        Ok(messages.join(" "))
    }
}

fn config_file_path() -> std::io::Result<std::path::PathBuf> {
    #[cfg(windows)]
    let base = std::env::var("APPDATA").map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "APPDATA not set"))?;
    #[cfg(not(windows))]
    let base = std::env::var("HOME").map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
    let dir = Path::new(&base).join("Syncone");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("syncone_config.json"))
}

pub fn load_config() -> Result<SyncConfig, String> {
    let path = config_file_path().map_err(|e| e.to_string())?;
    if !path.exists() {
        return Ok(SyncConfig::default());
    }
    let s = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

/// Unix timestamp (seconds) from SystemTime, or None if before epoch / error.
fn system_time_to_unix(t: SystemTime) -> Option<i64> {
    t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub save_local_mtime: Option<i64>,
    pub save_cloud_mtime: Option<i64>,
    pub mods_local_mtime: Option<i64>,
    pub mods_cloud_mtime: Option<i64>,
    pub save_local_newer: bool,
    pub save_cloud_newer: bool,
    pub mods_local_newer: bool,
    pub mods_cloud_newer: bool,
    /// Path used for "Local" Save on this PC (for display)
    pub save_path_used: Option<String>,
    pub mods_path_used: Option<String>,
}

pub fn get_sync_status(config: &SyncConfig) -> SyncStatus {
    if crate::supabase_sync::use_supabase(config) {
        return crate::supabase_sync::get_sync_status_supabase(config);
    }
    let mut status = SyncStatus {
        save_local_mtime: None,
        save_cloud_mtime: None,
        mods_local_mtime: None,
        mods_cloud_mtime: None,
        save_local_newer: false,
        save_cloud_newer: false,
        mods_local_newer: false,
        mods_cloud_newer: false,
        save_path_used: None,
        mods_path_used: None,
    };
    let (save_path, mods_path, cloud_path) = match (
        config.save_path.as_deref(),
        config.mods_path.as_deref(),
        config.cloud_path.as_deref(),
    ) {
        (Some(s), Some(m), Some(c)) => (Path::new(s), Path::new(m), Path::new(c)),
        _ => return status,
    };
    let cloud_save = cloud_path.join("Save");
    let cloud_mods = cloud_path.join("Mods");

    status.save_path_used = Some(save_path.to_string_lossy().to_string());
    status.mods_path_used = Some(mods_path.to_string_lossy().to_string());

    status.save_local_mtime = save_path
        .exists()
        .then(|| get_latest_mtime_recursive(save_path).ok())
        .flatten()
        .and_then(system_time_to_unix);
    status.save_cloud_mtime = cloud_save
        .exists()
        .then(|| get_latest_mtime_recursive(&cloud_save).ok())
        .flatten()
        .and_then(system_time_to_unix);
    status.mods_local_mtime = mods_path
        .exists()
        .then(|| get_latest_mtime_recursive(mods_path).ok())
        .flatten()
        .and_then(system_time_to_unix);
    status.mods_cloud_mtime = cloud_mods
        .exists()
        .then(|| get_latest_mtime_recursive(&cloud_mods).ok())
        .flatten()
        .and_then(system_time_to_unix);

    if let (Some(local), Some(cloud)) = (status.save_local_mtime, status.save_cloud_mtime) {
        status.save_local_newer = local > cloud;
        status.save_cloud_newer = cloud > local;
    } else if status.save_local_mtime.is_some() && status.save_cloud_mtime.is_none() {
        status.save_local_newer = true;
    } else if status.save_cloud_mtime.is_some() && status.save_local_mtime.is_none() {
        status.save_cloud_newer = true;
    }
    if let (Some(local), Some(cloud)) = (status.mods_local_mtime, status.mods_cloud_mtime) {
        status.mods_local_newer = local > cloud;
        status.mods_cloud_newer = cloud > local;
    } else if status.mods_local_mtime.is_some() && status.mods_cloud_mtime.is_none() {
        status.mods_local_newer = true;
    } else if status.mods_cloud_mtime.is_some() && status.mods_local_mtime.is_none() {
        status.mods_cloud_newer = true;
    }
    status
}

pub fn save_config(config: &SyncConfig) -> Result<(), String> {
    let path = config_file_path().map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    let mut f = fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn sync_push(config: &SyncConfig, target: SyncTarget) -> Result<String, String> {
    if crate::supabase_sync::use_supabase(config) {
        return crate::supabase_sync::sync_push_supabase(config, target);
    }
    let save_path = config
        .save_path
        .as_ref()
        .ok_or("Save path is not set")?;
    let mods_path = config.mods_path.as_ref().ok_or("Mods path is not set")?;
    let cloud_path = config.cloud_path.as_ref().ok_or("Cloud path is not set")?;

    let cloud_save = Path::new(cloud_path).join("Save");
    let cloud_mods = Path::new(cloud_path).join("Mods");
    let local_save = Path::new(save_path);
    let local_mods = Path::new(mods_path);

    fs::create_dir_all(&cloud_save).map_err(|e| e.to_string())?;
    fs::create_dir_all(&cloud_mods).map_err(|e| e.to_string())?;

    let mut messages = Vec::new();

    if (target == SyncTarget::Save || target == SyncTarget::Both) && local_save.exists() {
        replace_dir_with(local_save, &cloud_save).map_err(|e| e.to_string())?;
        messages.push("Save uploaded to cloud.");
    }
    if (target == SyncTarget::Mods || target == SyncTarget::Both) && local_mods.exists() {
        replace_dir_with(local_mods, &cloud_mods).map_err(|e| e.to_string())?;
        messages.push("Mods uploaded to cloud.");
    }

    if messages.is_empty() {
        Ok("No local folders to upload.".to_string())
    } else {
        Ok(messages.join(" "))
    }
}
