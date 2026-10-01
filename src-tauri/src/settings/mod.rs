//! User settings: a single small JSON file, loaded once at startup and
//! written atomically on every change.
//!
//! The file is intentionally forgiving: unknown fields are ignored and missing
//! fields fall back to their defaults, so downgrading Wisp never corrupts a
//! configuration written by a newer version.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    /// Follow the operating system preference.
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// BCP-47 language tag. `None` means "follow the system language".
    pub language: Option<String>,
    pub theme: Theme,
    /// Set once the first-run introduction has been completed or skipped.
    pub onboarding_completed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: None,
            theme: Theme::Dark,
            onboarding_completed: false,
        }
    }
}

/// In-memory settings guarded by a mutex, backed by `settings.json`.
#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// Reads the settings file. A missing file yields defaults; a corrupt file
    /// is reported so the UI can tell the user instead of silently resetting.
    pub fn load(path: PathBuf) -> Result<Self> {
        let current = read_file(&path)?;
        Ok(Self {
            path,
            current: Mutex::new(current),
        })
    }

    /// Starts from defaults while still pointing at `path`, so the next
    /// change the user makes repairs the file. Used when `load` fails.
    pub fn defaults_at(path: PathBuf) -> Self {
        Self {
            path,
            current: Mutex::new(Settings::default()),
        }
    }

    pub fn get(&self) -> Settings {
        self.current
            .lock()
            .expect("settings mutex poisoned")
            .clone()
    }

    /// Applies `edit` to the settings and persists the result.
    pub fn update<F>(&self, edit: F) -> Result<Settings>
    where
        F: FnOnce(&mut Settings),
    {
        let mut guard = self.current.lock().expect("settings mutex poisoned");
        let mut next = guard.clone();
        edit(&mut next);
        write_file(&self.path, &next)?;
        *guard = next.clone();
        Ok(next)
    }
}

fn read_file(path: &Path) -> Result<Settings> {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).map_err(|e| {
            Error::Config(format!(
                "{} could not be parsed ({e}); fix or delete the file",
                path.display()
            ))
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(Error::Io(e)),
    }
}

/// Writes through a temporary file so an interrupted write cannot leave a
/// half-written `settings.json` behind.
fn write_file(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let store = SettingsStore::load(tmp.path().join("settings.json")).unwrap();

        let settings = store.get();
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.language, None);
        assert!(!settings.onboarding_completed);
    }

    #[test]
    fn updates_round_trip_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");

        let store = SettingsStore::load(path.clone()).unwrap();
        store
            .update(|s| {
                s.language = Some("es".into());
                s.theme = Theme::Light;
            })
            .unwrap();

        let reloaded = SettingsStore::load(path).unwrap().get();
        assert_eq!(reloaded.language.as_deref(), Some("es"));
        assert_eq!(reloaded.theme, Theme::Light);
    }

    #[test]
    fn unknown_fields_are_ignored_and_missing_ones_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"theme":"light","somethingNew":42}"#).unwrap();

        let settings = SettingsStore::load(path).unwrap().get();
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.language, None);
    }

    #[test]
    fn corrupt_file_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, "not json").unwrap();

        let err = SettingsStore::load(path).unwrap_err();
        assert_eq!(err.kind(), "config");
    }
}
