//! Wisp: a fast, lightweight launcher for an Epic Games library.
//!
//! The backend is organised as a handful of focused modules:
//!
//! * [`platform`]: per-OS paths and behaviour, selected at compile time.
//! * [`runtime`]: the Legendary binary Wisp downloads and keeps up to date.
//! * [`legendary`]: the only place that talks to the Legendary CLI.
//! * [`auth`]: signing in to Epic.
//! * [`library`], [`downloads`]: game metadata and install/update jobs.
//! * [`settings`]: the user's `settings.json`.

pub mod auth;
pub mod commands;
pub mod downloads;
pub mod error;
pub mod legendary;
pub mod library;
pub mod platform;
pub mod runtime;
pub mod settings;

use std::sync::Arc;

/// Name of the log file inside the logs directory.
pub const LOG_FILE: &str = "wisp.log";

use library::LibraryService;
use platform::Platform;
use runtime::RuntimeManager;
use settings::SettingsStore;

/// Application state shared by every Tauri command.
pub struct AppState {
    pub platform: Arc<dyn Platform>,
    pub settings: SettingsStore,
    pub runtime: RuntimeManager,
    pub library: LibraryService,
}

/// Builds and runs the Tauri application.
///
/// Startup is deliberately cheap: resolve directories, read one small JSON
/// file, show the window. Anything slower (provisioning Legendary, refreshing
/// the library) happens on demand with progress reported to the UI.
pub fn run() {
    let platform: Arc<dyn Platform> = match platform::current() {
        Ok(platform) => Arc::from(platform),
        Err(e) => {
            // Without data directories there is nothing meaningful to show.
            eprintln!("Wisp cannot start: {e}");
            std::process::exit(1);
        }
    };

    init_tracing(&platform.paths().logs);

    let settings_path = platform.paths().settings_file();
    let settings = SettingsStore::load(settings_path.clone()).unwrap_or_else(|e| {
        // A corrupt settings file must not block startup: carry on with
        // defaults, and let the next change the user makes rewrite the file.
        tracing::error!("falling back to default settings: {e}");
        SettingsStore::defaults_at(settings_path)
    });

    tracing::info!(
        platform = platform.id(),
        portable = platform.paths().portable,
        root = %platform.paths().root.display(),
        "starting Wisp {}",
        env!("CARGO_PKG_VERSION")
    );

    let runtime = RuntimeManager::new(Arc::clone(&platform));
    let library = LibraryService::new(&platform.paths().cache);
    let covers = library.covers().clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Cover art lives in the cache folder, not in the bundle, so it is
        // served through a protocol of its own rather than the asset one.
        // Only names the library knows about resolve to a file.
        .register_uri_scheme_protocol("cover", move |_ctx, request| serve_cover(&covers, &request))
        .manage(AppState {
            platform,
            settings,
            runtime,
            library,
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::set_language,
            commands::set_theme,
            commands::set_onboarding_completed,
            commands::runtime_status,
            commands::runtime_probe,
            commands::runtime_check_update,
            commands::runtime_install,
            commands::auth_status,
            commands::auth_login,
            commands::auth_login_url,
            commands::auth_submit_code,
            commands::auth_logout,
            commands::library_snapshot,
            commands::library_refresh,
            commands::game_details,
            commands::open_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wisp");
}

/// Serves a cached cover image.
///
/// The request path is an app name, which is hashed into a file name, so a
/// crafted request cannot reach outside the cover folder. Anything not
/// cached yet answers 404 and the UI shows a placeholder.
fn serve_cover(
    covers: &library::covers::CoverCache,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode};

    let app_name = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('?')
        .next()
        .unwrap_or_default();
    let app_name = percent_decode(app_name);

    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .expect("a 404 response is always valid")
    };

    if app_name.is_empty() {
        return not_found();
    }

    match std::fs::read(covers.path(&app_name)) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", library::covers::content_type(&bytes))
            // Cached art never changes for a given app name; a new image
            // replaces the file and the UI reloads it explicitly.
            .header("Cache-Control", "max-age=86400")
            .body(bytes)
            .unwrap_or_else(|_| not_found()),
        Err(_) => not_found(),
    }
}

/// Decodes the percent-escapes a webview puts in a request path.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            if let Some(byte) = hex {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Logs to `<logs>/wisp.log` and, when there is one, to the console.
///
/// A file is not optional on Windows: release builds have no console, so
/// without it there would be no way to find out why something failed on a
/// user's machine. Verbosity comes from `WISP_LOG` and defaults to `info`.
fn init_tracing(logs_dir: &std::path::Path) {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    use tracing_subscriber::{fmt, EnvFilter};

    // Dependencies are chatty at debug level and their output drowns ours,
    // so they are quiet unless WISP_LOG asks for them by name.
    let filter = EnvFilter::try_from_env("WISP_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,ureq=warn,rustls=warn"));

    let file = std::fs::create_dir_all(logs_dir)
        .and_then(|()| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(logs_dir.join(LOG_FILE))
        })
        .ok();

    match file {
        Some(file) => {
            // Duplicating the handle per event is cheap next to the write,
            // and keeps the writer `'static` without a global.
            let to_file = move || file.try_clone().expect("the log file handle is still open");
            let _ = fmt()
                .with_env_filter(filter)
                .with_target(false)
                .with_ansi(false)
                .with_writer(to_file.and(std::io::stdout))
                .try_init();
        }
        None => {
            let _ = fmt().with_env_filter(filter).with_target(false).try_init();
            tracing::warn!("could not open a log file; logging to the console only");
        }
    }
}
