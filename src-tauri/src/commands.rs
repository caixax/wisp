//! Tauri commands exposed to the frontend.
//!
//! Commands stay thin: they validate input, call into a module and return a
//! serialisable value. All user-facing wording lives in the frontend
//! translation files, never here.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::{Error, Result};
use crate::platform::WispPaths;
use crate::runtime::{RuntimeProbe, RuntimeStatus, UpdateCheck};
use crate::settings::{Settings, Theme};
use crate::AppState;

/// Everything the UI needs on boot, in a single round trip.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: &'static str,
    pub platform: &'static str,
    pub portable: bool,
    pub paths: WispPaths,
    pub default_install_dir: String,
    /// Operating system locale, used when the user has no explicit preference.
    pub system_locale: Option<String>,
    pub settings: Settings,
    pub runtime: RuntimeStatus,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: state.platform.id(),
        portable: state.platform.paths().portable,
        paths: state.platform.paths().clone(),
        default_install_dir: state.platform.default_install_dir().display().to_string(),
        system_locale: sys_locale::get_locale(),
        settings: state.settings.get(),
        runtime: state.runtime.status(),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

/// `None` restores "follow the system language".
#[tauri::command]
pub fn set_language(state: State<'_, AppState>, language: Option<String>) -> Result<Settings> {
    state.settings.update(|s| s.language = language)
}

#[tauri::command]
pub fn set_theme(state: State<'_, AppState>, theme: Theme) -> Result<Settings> {
    state.settings.update(|s| s.theme = theme)
}

#[tauri::command]
pub fn set_onboarding_completed(state: State<'_, AppState>, completed: bool) -> Result<Settings> {
    state
        .settings
        .update(|s| s.onboarding_completed = completed)
}

/// Cheap: reads a metadata file, never spawns Legendary.
#[tauri::command]
pub fn runtime_status(state: State<'_, AppState>) -> RuntimeStatus {
    state.runtime.status()
}

/// Runs `legendary --version`. Costs about a second, so the UI calls it
/// after the window is already up rather than during boot.
#[tauri::command]
pub async fn runtime_probe(state: State<'_, AppState>) -> Result<RuntimeProbe> {
    Ok(state.runtime.probe().await)
}

#[tauri::command]
pub async fn runtime_check_update(state: State<'_, AppState>) -> Result<UpdateCheck> {
    state.runtime.check_for_update().await
}

/// Installs or repairs the Legendary runtime, reporting progress on the
/// `runtime:progress` event. With `force` false an up-to-date install is
/// left alone.
#[tauri::command]
pub async fn runtime_install(
    app: AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> Result<RuntimeStatus> {
    state.runtime.install(&app, force).await
}

/// Opens one of Wisp's own folders in the system file manager.
///
/// Restricted to paths inside the data directories on purpose: the frontend
/// should never be able to ask the backend to open an arbitrary location.
#[tauri::command]
pub fn open_path(state: State<'_, AppState>, path: String) -> Result<()> {
    let requested = PathBuf::from(&path);
    let paths = state.platform.paths();
    let allowed = [
        &paths.root,
        &paths.config,
        &paths.runtime,
        &paths.legendary,
        &paths.cache,
        &paths.logs,
    ];

    if !allowed.iter().any(|root| requested.starts_with(root)) {
        return Err(Error::Config(format!(
            "{path} is outside Wisp's own folders"
        )));
    }

    tauri_plugin_opener::open_path(requested, None::<&str>)
        .map_err(|e| Error::Config(format!("could not open {path}: {e}")))
}
