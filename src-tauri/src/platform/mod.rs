//! Platform abstraction layer.
//!
//! Everything that differs between operating systems — data directories,
//! Python discovery, how a game process is spawned and detected — lives
//! behind this module. The rest of the app only talks to the [`Platform`]
//! trait, so adding Linux support (chapter 2) means adding an implementation
//! here, not touching call sites.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

/// Name of the marker file that switches Wisp into portable mode.
/// It must sit next to the executable.
pub const PORTABLE_FLAG: &str = "portable.flag";

/// Resolved on-disk locations Wisp uses. Created by [`WispPaths::ensure`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WispPaths {
    /// True when a `portable.flag` file was found next to the executable.
    pub portable: bool,
    /// Portable mode: the `data` folder next to the executable.
    /// Installed mode: the per-user data root.
    pub root: PathBuf,
    /// User settings (`settings.json`). Roams with the user profile on Windows.
    pub config: PathBuf,
    /// Private Python runtime and Legendary installation.
    pub runtime: PathBuf,
    /// Exported as `LEGENDARY_CONFIG_PATH` so Wisp never touches a
    /// Legendary or Heroic installation the user already has.
    pub legendary: PathBuf,
    /// Cover art and other regenerable downloads.
    pub cache: PathBuf,
    /// Application logs.
    pub logs: PathBuf,
}

impl WispPaths {
    /// Creates every directory in this set if it does not exist yet.
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            &self.root,
            &self.config,
            &self.runtime,
            &self.legendary,
            &self.cache,
            &self.logs,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// Absolute path of the user settings file.
    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }
}

/// Operating-system specific behaviour.
///
/// Implementations are selected at compile time; call [`current`] to get one.
pub trait Platform: Send + Sync {
    /// Stable identifier used in logs and in the UI ("windows", "linux").
    fn id(&self) -> &'static str;

    /// Resolved data directories for this installation.
    fn paths(&self) -> &WispPaths;

    /// Where games are installed unless the user picks something else.
    fn default_install_dir(&self) -> PathBuf;

    /// File name of the Legendary executable managed by Wisp.
    fn legendary_executable_name(&self) -> &'static str;
}

/// Returns the platform implementation for the current target.
pub fn current() -> Result<Box<dyn Platform>> {
    imp::create().map(|p| Box::new(p) as Box<dyn Platform>)
}

/// Directory containing the running executable.
pub fn exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Paths("the executable has no parent directory".into()))
}

/// Portable mode is active when `portable.flag` sits next to the executable.
/// Returns the portable root (`<exe dir>/data`) when that is the case.
pub fn portable_root() -> Option<PathBuf> {
    let dir = exe_dir().ok()?;
    if dir.join(PORTABLE_FLAG).is_file() {
        Some(dir.join("data"))
    } else {
        None
    }
}

/// Shared layout used by every platform once a root directory is known,
/// so portable mode behaves identically across operating systems.
pub(crate) fn portable_layout(root: PathBuf) -> WispPaths {
    WispPaths {
        portable: true,
        config: root.join("config"),
        runtime: root.join("runtime"),
        legendary: root.join("legendary"),
        cache: root.join("cache"),
        logs: root.join("logs"),
        root,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_layout_keeps_everything_under_the_root() {
        let root = PathBuf::from("X:/wisp/data");
        let paths = portable_layout(root.clone());

        assert!(paths.portable);
        for dir in [
            &paths.config,
            &paths.runtime,
            &paths.legendary,
            &paths.cache,
            &paths.logs,
        ] {
            assert!(dir.starts_with(&root), "{dir:?} escaped the portable root");
        }
        assert_eq!(
            paths.settings_file(),
            root.join("config").join("settings.json")
        );
    }

    #[test]
    fn ensure_creates_every_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = portable_layout(tmp.path().join("data"));

        paths.ensure().unwrap();

        for dir in [
            &paths.root,
            &paths.config,
            &paths.runtime,
            &paths.legendary,
            &paths.cache,
            &paths.logs,
        ] {
            assert!(dir.is_dir(), "{dir:?} was not created");
        }
    }
}
