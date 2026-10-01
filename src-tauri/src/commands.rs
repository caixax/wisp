//! Tauri commands exposed to the frontend.
//!
//! Commands stay thin: they validate input, call into a module and return a
//! serialisable value. All user-facing wording lives in the frontend
//! translation files, never here.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::auth::{self, AuthState};
use crate::error::{Error, Result};
use crate::library::{GameDetails, LibrarySnapshot};
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

/// Who is signed in, according to Legendary. Costs one process spawn.
#[tauri::command]
pub async fn auth_status(state: State<'_, AppState>) -> Result<AuthState> {
    auth::status(&state.runtime.legendary()).await
}

/// Opens Epic's own login page in a separate window. The result arrives on
/// the `auth:changed` event, because the user may take a while.
///
/// Async on purpose: building a window from a synchronous command runs on
/// the main thread, which is the thread that has to process the window
/// creation, and the webview never finishes initialising.
#[tauri::command]
pub async fn auth_login(app: AppHandle) -> Result<()> {
    auth::open_login_window(&app)
}

/// The URL to open in a normal browser when the embedded window will not do.
#[tauri::command]
pub fn auth_login_url() -> String {
    auth::login_url()
}

/// Completes a login from a code the user pasted in.
#[tauri::command]
pub async fn auth_submit_code(state: State<'_, AppState>, code: String) -> Result<AuthState> {
    auth::submit_code(&state.runtime.legendary(), &code).await
}

/// Signs out and forgets the cached library, so the next account does not
/// see the previous one's games.
#[tauri::command]
pub async fn auth_logout(state: State<'_, AppState>) -> Result<AuthState> {
    let result = auth::logout(&state.runtime.legendary()).await;
    state.library.clear().await?;
    result
}

/// The library as last cached. Cheap, and what the grid renders on start.
#[tauri::command]
pub async fn library_snapshot(state: State<'_, AppState>) -> Result<LibrarySnapshot> {
    Ok(state.library.snapshot().await)
}

/// Re-reads the library from Legendary. `force` also makes Legendary
/// re-download the metadata it has cached itself.
#[tauri::command]
pub async fn library_refresh(
    app: AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> Result<LibrarySnapshot> {
    state
        .library
        .refresh(&app, &state.runtime.legendary(), force)
        .await
}

#[tauri::command]
pub async fn game_details(
    state: State<'_, AppState>,
    app_name: String,
) -> Result<Option<GameDetails>> {
    Ok(state.library.details(&app_name).await)
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
