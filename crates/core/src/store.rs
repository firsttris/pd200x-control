//! `settings.json` in the config folder (`$XDG_CONFIG_HOME/pd200x-control`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::Settings;
use crate::{Error, Result};

/// `$XDG_CONFIG_HOME/pd200x-control`, else `~/.config/pd200x-control`.
pub fn config_dir_from(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    match (xdg.filter(|x| !x.is_empty()), home.filter(|h| !h.is_empty())) {
        (Some(x), _) => Some(Path::new(x).join("pd200x-control")),
        (None, Some(h)) => Some(Path::new(h).join(".config/pd200x-control")),
        _ => None,
    }
}

pub fn system_config_dir() -> Option<PathBuf> {
    config_dir_from(std::env::var("XDG_CONFIG_HOME").ok().as_deref(), std::env::var("HOME").ok().as_deref())
}

/// The settings in `dir`, defaults when the file does not exist yet. A
/// broken file is kept aside as `settings.json.broken` and defaults are used.
pub fn load(dir: &Path) -> Result<Settings> {
    let path = dir.join("settings.json");
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(s) => Ok(s),
            Err(_) => {
                fs::rename(&path, dir.join("settings.json.broken"))?;
                Ok(Settings::default())
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.into()),
    }
}

/// Writes atomically: a temporary file, then a rename.
pub fn save(dir: &Path, settings: &Settings) -> Result<()> {
    fs::create_dir_all(dir)?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| Error::InvalidValue(e.to_string()))?;
    let tmp = dir.join("settings.json.tmp");
    fs::write(&tmp, json + "\n")?;
    fs::rename(tmp, dir.join("settings.json"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Lighting;

    #[test]
    fn config_dir_prefers_xdg() {
        assert_eq!(config_dir_from(Some("/x"), Some("/h")), Some(PathBuf::from("/x/pd200x-control")));
        assert_eq!(config_dir_from(Some(""), Some("/h")), Some(PathBuf::from("/h/.config/pd200x-control")));
        assert_eq!(config_dir_from(None, None), None);
    }

    #[test]
    fn round_trip_and_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("cfg");
        assert_eq!(load(&dir).unwrap(), Settings::default());
        let s = Settings { lighting: Some(Lighting::default()), boost_db: 14.6, ..Settings::default() };
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir).unwrap(), s);
        assert!(!dir.join("settings.json.tmp").exists());
    }

    #[test]
    fn a_broken_file_is_kept_aside() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("settings.json"), "{ nope").unwrap();
        assert_eq!(load(tmp.path()).unwrap(), Settings::default());
        assert!(tmp.path().join("settings.json.broken").exists());
    }
}
