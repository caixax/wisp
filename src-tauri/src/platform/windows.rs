//! Windows implementation of the platform layer.
//!
//! Installed mode follows the usual Windows split: roaming configuration in
//! `%APPDATA%`, machine-local runtime and caches in `%LOCALAPPDATA%`.

use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::platform::{portable_layout, portable_root, Platform, WispPaths};

const APP_FOLDER: &str = "Wisp";

pub struct WindowsPlatform {
    paths: WispPaths,
}

pub fn create() -> Result<WindowsPlatform> {
    let paths = match portable_root() {
        Some(root) => portable_layout(root),
        None => installed_layout()?,
    };
    paths.ensure()?;
    Ok(WindowsPlatform { paths })
}

fn installed_layout() -> Result<WispPaths> {
    let roaming = dirs::config_dir()
        .ok_or_else(|| Error::Paths("%APPDATA% is not available".into()))?
        .join(APP_FOLDER);
    let local = dirs::data_local_dir()
        .ok_or_else(|| Error::Paths("%LOCALAPPDATA% is not available".into()))?
        .join(APP_FOLDER);

    Ok(WispPaths {
        portable: false,
        config: roaming,
        runtime: local.join("runtime"),
        legendary: local.join("legendary"),
        cache: local.join("cache"),
        logs: local.join("logs"),
        root: local,
    })
}

impl Platform for WindowsPlatform {
    fn id(&self) -> &'static str {
        "windows"
    }

    fn paths(&self) -> &WispPaths {
        &self.paths
    }

    fn default_install_dir(&self) -> PathBuf {
        if self.paths.portable {
            return self.paths.root.join("Games");
        }
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\"))
            .join("Games")
            .join("Wisp")
    }

    fn legendary_executable_name(&self) -> &'static str {
        "legendary.exe"
    }
}
