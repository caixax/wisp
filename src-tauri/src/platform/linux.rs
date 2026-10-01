//! Linux implementation of the platform layer.
//!
//! Chapter 1 ships only the directory layout (XDG base directories) so the
//! rest of the codebase can already be written platform-agnostically.
//! Wine/Proton handling and process detection arrive in chapter 2, sprint 7.

use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::platform::{portable_layout, portable_root, Platform, WispPaths};

const APP_FOLDER: &str = "wisp";

pub struct LinuxPlatform {
    paths: WispPaths,
}

pub fn create() -> Result<LinuxPlatform> {
    let paths = match portable_root() {
        Some(root) => portable_layout(root),
        None => installed_layout()?,
    };
    paths.ensure()?;
    Ok(LinuxPlatform { paths })
}

fn installed_layout() -> Result<WispPaths> {
    let config = dirs::config_dir()
        .ok_or_else(|| Error::Paths("XDG_CONFIG_HOME is not available".into()))?
        .join(APP_FOLDER);
    let data = dirs::data_dir()
        .ok_or_else(|| Error::Paths("XDG_DATA_HOME is not available".into()))?
        .join(APP_FOLDER);
    let cache = dirs::cache_dir()
        .ok_or_else(|| Error::Paths("XDG_CACHE_HOME is not available".into()))?
        .join(APP_FOLDER);

    Ok(WispPaths {
        portable: false,
        config,
        runtime: data.join("runtime"),
        legendary: data.join("legendary"),
        logs: data.join("logs"),
        cache,
        root: data,
    })
}

impl Platform for LinuxPlatform {
    fn id(&self) -> &'static str {
        "linux"
    }

    fn paths(&self) -> &WispPaths {
        &self.paths
    }

    fn default_install_dir(&self) -> PathBuf {
        if self.paths.portable {
            return self.paths.root.join("Games");
        }
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/"))
            .join("Games")
            .join("Wisp")
    }

    fn legendary_executable_name(&self) -> &'static str {
        "legendary"
    }
}

/// Placeholder for chapter 2: Epic titles are Windows builds, so running them
/// on Linux requires a Wine/Proton compatibility layer that does not exist yet.
#[allow(dead_code)]
pub fn compatibility_layer_unavailable() -> Error {
    Error::UnsupportedPlatform { platform: "linux" }
}
