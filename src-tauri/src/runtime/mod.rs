//! Legendary runtime management.
//!
//! Wisp does not depend on Python being installed. It downloads the official
//! standalone Legendary binary published on GitHub Releases into
//! [`WispPaths::runtime`], verifies it against the SHA-256 digest the GitHub
//! API reports for the asset, and runs that.
//!
//! The same mechanism works on every platform Legendary builds for
//! (`legendary_windows_x64.exe`, `legendary_linux_x64`, ...), so the only
//! platform-specific part is which asset name to ask for.
//!
//! Updates are never applied without asking first, and uninstalling Wisp
//! leaves nothing behind outside its own data folder.
//!
//! [`WispPaths::runtime`]: crate::platform::WispPaths::runtime

pub mod github;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::error::{Error, Result};
use crate::legendary::Legendary;
use crate::platform::Platform;

/// Event the frontend listens on while the runtime is being provisioned.
pub const PROGRESS_EVENT: &str = "runtime:progress";

/// Partial downloads land here before being verified and moved into place.
const DOWNLOAD_FILE: &str = "legendary.download";
const METADATA_FILE: &str = "legendary.json";

/// How often progress is pushed to the UI. Fast enough to look live, slow
/// enough that a 17 MB download does not flood the event channel.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// What Wisp recorded about the binary it installed, so the common case
/// (app start) does not have to spawn a process to know what is there.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Metadata {
    pub version: Option<String>,
    pub asset: Option<String>,
    pub sha256: Option<String>,
    /// False when GitHub published no digest for the asset, so the download
    /// could only be trusted to the TLS connection.
    pub verified: bool,
    /// Unix seconds.
    pub installed_at: Option<u64>,
}

/// Cheap snapshot of the runtime: no process is spawned to build it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub installed: bool,
    pub executable: String,
    /// Version recorded when Wisp installed the binary.
    pub version: Option<String>,
    pub verified: bool,
    /// Asset this platform needs, or `None` on an unsupported architecture.
    pub asset: Option<String>,
}

/// Result of actually running `legendary --version`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProbe {
    pub ok: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current: Option<String>,
    pub latest: String,
    pub published_at: Option<String>,
    pub asset: String,
    pub size: u64,
    pub update_available: bool,
}

/// Progress of a runtime installation, pushed to the frontend as it happens.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "phase")]
pub enum Progress {
    /// Asking GitHub which release is current.
    Checking,
    Downloading {
        version: String,
        downloaded: u64,
        total: Option<u64>,
    },
    Verifying,
    Installing,
    Done {
        version: String,
    },
    Failed {
        message: String,
    },
}

pub struct RuntimeManager {
    platform: Arc<dyn Platform>,
    /// Held for the duration of an install so two cannot overlap.
    busy: tokio::sync::Mutex<()>,
}

impl RuntimeManager {
    pub fn new(platform: Arc<dyn Platform>) -> Self {
        Self {
            platform,
            busy: tokio::sync::Mutex::new(()),
        }
    }

    pub fn executable(&self) -> PathBuf {
        self.platform
            .paths()
            .runtime
            .join(self.platform.legendary_executable_name())
    }

    fn metadata_path(&self) -> PathBuf {
        self.platform.paths().runtime.join(METADATA_FILE)
    }

    /// A Legendary handle pointed at Wisp's isolated configuration.
    pub fn legendary(&self) -> Legendary {
        Legendary::new(self.executable(), self.platform.paths().legendary.clone())
    }

    fn metadata(&self) -> Metadata {
        std::fs::read_to_string(self.metadata_path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn write_metadata(&self, metadata: &Metadata) -> Result<()> {
        let path = self.metadata_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_vec_pretty(metadata)?)?;
        Ok(())
    }

    pub fn status(&self) -> RuntimeStatus {
        let executable = self.executable();
        let metadata = self.metadata();

        RuntimeStatus {
            installed: executable.is_file(),
            executable: executable.display().to_string(),
            version: metadata.version,
            verified: metadata.verified,
            asset: self.platform.legendary_asset_name().ok().map(str::to_owned),
        }
    }

    /// Runs `legendary --version` to confirm the binary actually works.
    /// Never returns an error: a broken runtime is a UI state, not a crash.
    pub async fn probe(&self) -> RuntimeProbe {
        match self.legendary().version().await {
            Ok(version) => RuntimeProbe {
                ok: true,
                version: Some(version),
                error: None,
            },
            Err(e) => RuntimeProbe {
                ok: false,
                version: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Asks GitHub for the current release and compares it with what is
    /// installed. Purely informational: nothing is downloaded here.
    pub async fn check_for_update(&self) -> Result<UpdateCheck> {
        let asset_name = self.platform.legendary_asset_name()?;
        let release = fetch_latest_release().await?;
        let asset = release.asset(asset_name).ok_or_else(|| {
            Error::Runtime(format!(
                "release {} does not include {asset_name}",
                release.tag_name
            ))
        })?;

        let current = self.metadata().version;

        Ok(UpdateCheck {
            update_available: is_newer(&release.tag_name, current.as_deref()),
            current,
            latest: release.tag_name.clone(),
            published_at: release.published_at.clone(),
            asset: asset.name.clone(),
            size: asset.size,
        })
    }

    /// Downloads and installs the current Legendary release.
    ///
    /// With `force` false this is a no-op when the installed binary is already
    /// the latest version, which makes it safe to call from "repair" and from
    /// first-run setup alike.
    pub async fn install(&self, app: &AppHandle, force: bool) -> Result<RuntimeStatus> {
        let Ok(_guard) = self.busy.try_lock() else {
            return Err(Error::Runtime(
                "the runtime is already being installed".into(),
            ));
        };

        match self.install_inner(app, force).await {
            Ok(status) => Ok(status),
            Err(e) => {
                emit(
                    app,
                    Progress::Failed {
                        message: e.to_string(),
                    },
                );
                Err(e)
            }
        }
    }

    async fn install_inner(&self, app: &AppHandle, force: bool) -> Result<RuntimeStatus> {
        emit(app, Progress::Checking);

        let asset_name = self.platform.legendary_asset_name()?;
        let release = fetch_latest_release().await?;
        let asset = release
            .asset(asset_name)
            .ok_or_else(|| {
                Error::Runtime(format!(
                    "release {} does not include {asset_name}",
                    release.tag_name
                ))
            })?
            .clone();

        let executable = self.executable();
        let metadata = self.metadata();
        let already_current =
            executable.is_file() && metadata.version.as_deref() == Some(release.tag_name.as_str());

        if already_current && !force {
            emit(
                app,
                Progress::Done {
                    version: release.tag_name,
                },
            );
            return Ok(self.status());
        }

        std::fs::create_dir_all(&self.platform.paths().runtime)?;
        let download_path = self.platform.paths().runtime.join(DOWNLOAD_FILE);

        emit(
            app,
            Progress::Downloading {
                version: release.tag_name.clone(),
                downloaded: 0,
                total: Some(asset.size).filter(|size| *size > 0),
            },
        );

        let digest = {
            let app = app.clone();
            let asset = asset.clone();
            let version = release.tag_name.clone();
            let destination = download_path.clone();

            tauri::async_runtime::spawn_blocking(move || {
                let mut last = Instant::now();
                github::download(&asset, &destination, &mut |downloaded, total| {
                    if last.elapsed() >= PROGRESS_INTERVAL {
                        last = Instant::now();
                        emit(
                            &app,
                            Progress::Downloading {
                                version: version.clone(),
                                downloaded,
                                total,
                            },
                        );
                    }
                })
            })
            .await
            .map_err(|e| Error::Runtime(format!("the download task failed: {e}")))??
        };

        emit(app, Progress::Verifying);

        let published = asset.sha256();
        if let Some(expected) = published {
            if !expected.eq_ignore_ascii_case(&digest) {
                // Never leave an unverified binary lying around.
                let _ = std::fs::remove_file(&download_path);
                return Err(Error::Checksum {
                    expected: expected.to_owned(),
                    actual: digest,
                });
            }
        } else {
            tracing::warn!(
                asset = %asset.name,
                "GitHub published no checksum for this asset; relying on TLS only"
            );
        }

        emit(app, Progress::Installing);

        std::fs::rename(&download_path, &executable).map_err(|e| {
            Error::Runtime(format!(
                "could not move the download into place at {}: {e}",
                executable.display()
            ))
        })?;
        self.platform.make_executable(&executable)?;

        self.write_metadata(&Metadata {
            version: Some(release.tag_name.clone()),
            asset: Some(asset.name.clone()),
            sha256: Some(digest),
            verified: published.is_some(),
            installed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs()),
        })?;

        tracing::info!(version = %release.tag_name, "installed Legendary");
        emit(
            app,
            Progress::Done {
                version: release.tag_name,
            },
        );

        Ok(self.status())
    }
}

async fn fetch_latest_release() -> Result<github::Release> {
    tauri::async_runtime::spawn_blocking(github::latest_release)
        .await
        .map_err(|e| Error::Runtime(format!("the release check task failed: {e}")))?
}

fn emit(app: &AppHandle, progress: Progress) {
    if let Err(e) = app.emit(PROGRESS_EVENT, &progress) {
        tracing::warn!("could not emit runtime progress: {e}");
    }
}

/// Is `latest` a higher version than what is installed?
///
/// Versions are compared component by component so `0.21.10` beats `0.21.9`.
/// Anything unparseable falls back to "different means newer", which at worst
/// offers an update the user can decline.
fn is_newer(latest: &str, current: Option<&str>) -> bool {
    let Some(current) = current else {
        return true;
    };
    if latest == current {
        return false;
    }

    match (parse_version(latest), parse_version(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => true,
    }
}

fn parse_version(version: &str) -> Option<Vec<u64>> {
    let parts: Vec<u64> = version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse::<u64>())
        .collect::<std::result::Result<_, _>>()
        .ok()?;
    (!parts.is_empty()).then_some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_install_always_needs_the_download() {
        assert!(is_newer("0.21.1", None));
    }

    #[test]
    fn the_same_version_is_not_an_update() {
        assert!(!is_newer("0.21.1", Some("0.21.1")));
    }

    #[test]
    fn compares_components_numerically() {
        assert!(is_newer("0.21.10", Some("0.21.9")));
        assert!(!is_newer("0.21.9", Some("0.21.10")));
        assert!(is_newer("0.22.0", Some("0.21.34")));
        assert!(!is_newer("0.20.34", Some("0.21.0")));
    }

    #[test]
    fn handles_a_leading_v_and_differing_lengths() {
        assert!(is_newer("v0.21.2", Some("0.21.1")));
        assert!(is_newer("0.21.1.1", Some("0.21.1")));
    }

    #[test]
    fn unparseable_versions_fall_back_to_offering_the_update() {
        assert!(is_newer("nightly", Some("0.21.1")));
        assert!(is_newer("0.21.2", Some("unknown")));
    }

    #[test]
    fn metadata_round_trips_and_tolerates_old_files() {
        let metadata = Metadata {
            version: Some("0.21.1".into()),
            asset: Some("legendary_windows_x64.exe".into()),
            sha256: Some("abc".into()),
            verified: true,
            installed_at: Some(1_759_000_000),
        };
        let json = serde_json::to_string(&metadata).unwrap();
        let back: Metadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version.as_deref(), Some("0.21.1"));
        assert!(back.verified);

        let sparse: Metadata = serde_json::from_str(r#"{"version":"0.20.34"}"#).unwrap();
        assert_eq!(sparse.version.as_deref(), Some("0.20.34"));
        assert!(!sparse.verified);
    }
}
