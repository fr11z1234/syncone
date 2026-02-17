//! Supabase Storage: upload/download Save.zip and Mods.zip.

use crate::sync::{clear_dir, get_latest_mtime_recursive, set_synced_organisation_name, SyncConfig, SyncTarget};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Money.json structure (we only need LifetimeEarnings for progress check).
#[derive(Deserialize)]
struct MoneyData {
    #[serde(rename = "LifetimeEarnings")]
    lifetime_earnings: Option<f64>,
}

/// Returns the maximum LifetimeEarnings found in any SaveGame_*/Money.json under the save root.
/// Used to prevent uploading a save that is behind the cloud (progress can't go backwards).
fn max_lifetime_earnings_from_save_dir(save_root: &Path) -> Option<f64> {
    // Also check if Money.json exists directly in save_root (flat save structure).
    if let Ok(bytes) = fs::read(save_root.join("Money.json")) {
        if let Ok(data) = serde_json::from_slice::<MoneyData>(&bytes) {
            if let Some(v) = data.lifetime_earnings {
                return Some(v);
            }
        }
    }

    // Otherwise scan SaveGame_* subdirectories.
    let dir = fs::read_dir(save_root).ok()?;
    let mut max_val = None::<f64>;
    for entry in dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("SaveGame_") || !entry.path().is_dir() {
            continue;
        }
        let money_path = entry.path().join("Money.json");
        let bytes = match fs::read(&money_path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let data: MoneyData = match serde_json::from_slice(&bytes) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if let Some(v) = data.lifetime_earnings {
            max_val = Some(max_val.map_or(v, |m| m.max(v)));
        }
    }
    max_val
}

pub(crate) fn use_supabase(config: &SyncConfig) -> bool {
    config
        .supabase_url
        .as_ref()
        .and(config.supabase_key.as_ref())
        .and(config.bucket_name.as_ref())
        .is_some()
}

fn system_time_to_unix(t: SystemTime) -> Option<i64> {
    t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64)
}

/// Zip en mappe rekursivt til bytes.
fn zip_dir(path: &Path) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .unix_permissions(0o755)
            .compression_method(zip::CompressionMethod::Deflated);

        fn add_dir<W: std::io::Write + std::io::Seek>(
            zip: &mut zip::ZipWriter<W>,
            options: zip::write::SimpleFileOptions,
            dir: &Path,
            prefix: &str,
        ) -> Result<(), String> {
            for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let e = e.map_err(|e| e.to_string())?;
                let name = e.file_name();
                let full = dir.join(&name);
                let entry_path = if prefix.is_empty() {
                    name.to_string_lossy().to_string()
                } else {
                    format!("{}/{}", prefix, name.to_string_lossy())
                };
                if e.file_type().map_err(|e| e.to_string())?.is_dir() {
                    add_dir(zip, options, &full, &entry_path)?;
                } else {
                    zip.start_file(&entry_path, options)
                        .map_err(|e| e.to_string())?;
                    let mut f = fs::File::open(&full).map_err(|e| e.to_string())?;
                    std::io::copy(&mut f, zip).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        }

        if path.is_dir() {
            add_dir(&mut zip, options, path, "")?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

/// Udpak zip-bytes til en mappe (mappen tømmes først).
fn unzip_to_dir(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    clear_dir(dest).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        let out = dest.join(&name);
        if file.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn supabase_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| e.to_string())
}

fn supabase_upload(
    url: &str,
    key: &str,
    bucket: &str,
    object_name: &str,
    data: &[u8],
) -> Result<(), String> {
    let url = url.trim_end_matches('/');
    let endpoint = format!("{}/storage/v1/object/{}/{}", url, bucket, object_name);
    let res = supabase_client()?
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", key))
        .header("apikey", key)
        .header("Content-Type", "application/zip")
        .header("x-upsert", "true")
        .body(data.to_vec())
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().unwrap_or_default();
        return Err(format!("Upload failed: {} {}", status, body));
    }
    Ok(())
}

fn supabase_download(url: &str, key: &str, bucket: &str, object_name: &str) -> Result<Vec<u8>, String> {
    let url = url.trim_end_matches('/');
    let endpoint = format!("{}/storage/v1/object/{}/{}", url, bucket, object_name);
    let res = supabase_client()?
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", key))
        .header("apikey", key)
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Download failed: {}", res.status()));
    }
    let bytes = res.bytes().map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}

#[derive(Deserialize)]
struct ListObject {
    name: String,
    updated_at: Option<String>,
}

fn supabase_list(url: &str, key: &str, bucket: &str) -> Result<Vec<ListObject>, String> {
    let url = url.trim_end_matches('/');
    let endpoint = format!("{}/storage/v1/object/list/{}", url, bucket);
    let res = supabase_client()?
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", key))
        .header("apikey", key)
        .header("Content-Type", "application/json")
        .body(r#"{"prefix":""}"#)
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("List failed: {}", res.status()));
    }
    let list: Vec<ListObject> = res.json().map_err(|e| e.to_string())?;
    Ok(list)
}

/// Returns (Save.zip updated_at unix, Mods.zip updated_at unix).
fn supabase_object_mtimes(
    url: &str,
    key: &str,
    bucket: &str,
) -> Result<(Option<i64>, Option<i64>), String> {
    let list = supabase_list(url, key, bucket)?;
    let mut save_ts = None;
    let mut mods_ts = None;
    for obj in list {
        let ts = obj
            .updated_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());
        if obj.name == "Save.zip" {
            save_ts = ts;
        } else if obj.name == "Mods.zip" {
            mods_ts = ts;
        }
    }
    Ok((save_ts, mods_ts))
}

pub fn sync_pull_supabase(config: &SyncConfig, target: SyncTarget, force: bool) -> Result<String, String> {
    let save_path = config.save_path.as_ref().ok_or("Save path is not set")?;
    let mods_path = config.mods_path.as_ref().ok_or("Mods path is not set")?;
    let url = config.supabase_url.as_ref().ok_or("Supabase URL is missing")?;
    let key = config.supabase_key.as_ref().ok_or("Supabase key is missing")?;
    let bucket = config.bucket_name.as_ref().ok_or("Bucket name is missing")?;

    let local_save = Path::new(save_path);
    let local_mods = Path::new(mods_path);
    let mut messages = Vec::new();

    let (save_ts, mods_ts) = supabase_object_mtimes(url, key, bucket)?;

    if target == SyncTarget::Save || target == SyncTarget::Both {
        let local_save_mtime = local_save
            .exists()
            .then(|| get_latest_mtime_recursive(local_save).ok())
            .flatten()
            .and_then(system_time_to_unix);
        if let Some(cloud_ts) = save_ts {
            let should_pull = match local_save_mtime {
                Some(local) => cloud_ts > local,
                None => true,
            };
            if should_pull {
                let data = supabase_download(url, key, bucket, "Save.zip")?;

                // Unless force, check if local save is more advanced than the cloud version.
                if !force {
                    let temp_dir = std::env::temp_dir().join(format!(
                        "syncone_pull_check_{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    ));
                    let _ = fs::create_dir_all(&temp_dir);
                    if unzip_to_dir(&data, &temp_dir).is_ok() {
                        let cloud_max = max_lifetime_earnings_from_save_dir(&temp_dir);
                        let local_max = max_lifetime_earnings_from_save_dir(local_save);
                        let _ = fs::remove_dir_all(&temp_dir);
                        if let (Some(local_val), Some(cloud_val)) = (local_max, cloud_max) {
                            if local_val > cloud_val {
                                return Err(format!(
                                    "PROGRESS_WARNING:The cloud save appears to be behind your local save.\n\nYour lifetime earnings: {:.0}\nCloud lifetime earnings: {:.0}\n\nFetching would overwrite your more advanced local save.",
                                    local_val, cloud_val
                                ));
                            }
                        }
                    } else {
                        let _ = fs::remove_dir_all(&temp_dir);
                    }
                }

                unzip_to_dir(&data, local_save)?;
                crate::sync::inject_has_exited_rv(local_save)?;
                set_synced_organisation_name(local_save)?;
                messages.push("Save fetched from Supabase.");
            }
        }
    }

    if target == SyncTarget::Mods || target == SyncTarget::Both {
        let local_mods_mtime = local_mods
            .exists()
            .then(|| get_latest_mtime_recursive(local_mods).ok())
            .flatten()
            .and_then(system_time_to_unix);
        if let Some(cloud_ts) = mods_ts {
            let should_pull = match local_mods_mtime {
                Some(local) => cloud_ts > local,
                None => true,
            };
            if should_pull {
                let data = supabase_download(url, key, bucket, "Mods.zip")?;
                unzip_to_dir(&data, local_mods)?;
                messages.push("Mods fetched from Supabase.");
            }
        }
    }

    if messages.is_empty() {
        Ok("Nothing new to fetch – you already have the latest version.".to_string())
    } else {
        Ok(messages.join(" "))
    }
}

pub fn sync_push_supabase(config: &SyncConfig, target: SyncTarget, force: bool) -> Result<String, String> {
    let save_path = config.save_path.as_ref().ok_or("Save path is not set")?;
    let mods_path = config.mods_path.as_ref().ok_or("Mods path is not set")?;
    let url = config.supabase_url.as_ref().ok_or("Supabase URL is missing")?;
    let key = config.supabase_key.as_ref().ok_or("Supabase key is missing")?;
    let bucket = config.bucket_name.as_ref().ok_or("Bucket name is missing")?;

    let local_save = Path::new(save_path);
    let local_mods = Path::new(mods_path);
    let mut messages = Vec::new();

    if (target == SyncTarget::Save || target == SyncTarget::Both) && local_save.exists() {
        // Unless force=true, check LifetimeEarnings to prevent uploading a save that is behind the cloud.
        if !force {
        if let Ok(cloud_bytes) = supabase_download(url, key, bucket, "Save.zip") {
            let temp_dir = std::env::temp_dir().join(format!(
                "syncone_check_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ));
            let _ = fs::create_dir_all(&temp_dir);
            if unzip_to_dir(&cloud_bytes, &temp_dir).is_ok() {
                let cloud_max = max_lifetime_earnings_from_save_dir(&temp_dir);
                let local_max = max_lifetime_earnings_from_save_dir(local_save);
                let _ = fs::remove_dir_all(&temp_dir);
                if let (Some(local_val), Some(cloud_val)) = (local_max, cloud_max) {
                    if local_val < cloud_val {
                        return Err(format!(
                            "PROGRESS_WARNING:Your local save appears to be behind the cloud version.\n\nYour lifetime earnings: {:.0}\nCloud lifetime earnings: {:.0}\n\nUploading would overwrite a more advanced save.",
                            local_val, cloud_val
                        ));
                    }
                }
            } else {
                let _ = fs::remove_dir_all(&temp_dir);
            }
        }
        } // end if !force

        // Stamp the save name so it's easy to recognize in-game.
        let _ = set_synced_organisation_name(local_save);

        let data = zip_dir(local_save)?;
        supabase_upload(url, key, bucket, "Save.zip", &data)?;
        messages.push("Save uploaded to Supabase.");
    }
    if (target == SyncTarget::Mods || target == SyncTarget::Both) && local_mods.exists() {
        let data = zip_dir(local_mods)?;
        supabase_upload(url, key, bucket, "Mods.zip", &data)?;
        messages.push("Mods uploaded to Supabase.");
    }

    if messages.is_empty() {
        Ok("No local folders to upload.".to_string())
    } else {
        Ok(messages.join(" "))
    }
}

pub fn get_sync_status_supabase(config: &SyncConfig) -> crate::sync::SyncStatus {
    let mut status = crate::sync::SyncStatus {
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
    let (save_path, mods_path) = match (config.save_path.as_deref(), config.mods_path.as_deref()) {
        (Some(s), Some(m)) => (Path::new(s), Path::new(m)),
        _ => return status,
    };
    status.save_path_used = Some(save_path.to_string_lossy().to_string());
    status.mods_path_used = Some(mods_path.to_string_lossy().to_string());

    let (url, key, bucket) = match (
        config.supabase_url.as_ref(),
        config.supabase_key.as_ref(),
        config.bucket_name.as_ref(),
    ) {
        (Some(u), Some(k), Some(b)) => (u.as_str(), k.as_str(), b.as_str()),
        _ => return status,
    };

    status.save_local_mtime = save_path
        .exists()
        .then(|| get_latest_mtime_recursive(save_path).ok())
        .flatten()
        .and_then(system_time_to_unix);
    status.mods_local_mtime = mods_path
        .exists()
        .then(|| get_latest_mtime_recursive(mods_path).ok())
        .flatten()
        .and_then(system_time_to_unix);

    let (save_cloud_ts, mods_cloud_ts) = match supabase_object_mtimes(url, key, bucket) {
        Ok(t) => t,
        Err(_) => return status,
    };
    status.save_cloud_mtime = save_cloud_ts;
    status.mods_cloud_mtime = mods_cloud_ts;

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

