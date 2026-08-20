//! Persist the list of web properties as JSON.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const APP_ID: &str = "edt-down-for-me";

pub const DEFAULT_SITES: &[&str] = &[
    "google.com",
    "github.com",
    "gitlab.com",
    "microsoft.com",
    "okta.com",
];

#[derive(Debug, Serialize, Deserialize, Default)]
struct FileFormat {
    sites: Vec<String>,
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_ID).join("sites.json")
}

pub fn load() -> Vec<String> {
    let path = config_path();
    if path.exists() {
        load_from(&path)
    } else {
        let sites = default_sites();
        let _ = save_to(&path, &sites);
        sites
    }
}

pub fn load_from(path: &Path) -> Vec<String> {
    let Ok(bytes) = fs::read(path) else {
        return default_sites();
    };
    match serde_json::from_slice::<FileFormat>(&bytes) {
        Ok(file) => {
            let mut out = Vec::new();
            for raw in file.sites {
                if let Ok(t) = crate::host::parse_target(&raw) {
                    if !out.iter().any(|s: &String| s.eq_ignore_ascii_case(&t.display)) {
                        out.push(t.display);
                    }
                }
            }
            if out.is_empty() {
                default_sites()
            } else {
                out
            }
        }
        Err(_) => default_sites(),
    }
}

pub fn save(sites: &[String]) -> io::Result<()> {
    save_to(&config_path(), sites)
}

pub fn save_to(path: &Path, sites: &[String]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = FileFormat {
        sites: sites.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&payload).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&json)?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn default_sites() -> Vec<String> {
    DEFAULT_SITES.iter().map(|s| (*s).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn round_trip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("edt-down-for-me-test-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sites.json");
        let sites = vec!["google.com".into(), "1.1.1.1".into()];
        save_to(&path, &sites).unwrap();
        assert_eq!(load_from(&path), sites);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let path = PathBuf::from("/tmp/definitely-missing-edt-down-for-me-xyz.json");
        assert_eq!(load_from(&path), default_sites());
    }
}
