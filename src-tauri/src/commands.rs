//! Tauri commands exposed to the frontend.
//!
//! Commands stay thin: they validate input, call into a module and return a
//! serialisable value. All user-facing wording lives in the frontend
//! translation files, never here.

use serde::Serialize;
use tauri::State;

use crate::error::Result;
use crate::platform::WispPaths;
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
