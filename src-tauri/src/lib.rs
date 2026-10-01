//! Wisp — a fast, lightweight launcher for an Epic Games library.
//!
//! The backend is organised as a handful of focused modules:
//!
//! * [`platform`] — per-OS paths and behaviour, selected at compile time.
//! * [`runtime`] — the private Python + Legendary installation Wisp manages.
//! * [`legendary`] — the only place that talks to the Legendary CLI.
//! * [`library`], [`downloads`] — game metadata and install/update jobs.
//! * [`settings`] — the user's `settings.json`.

pub mod commands;
pub mod downloads;
pub mod error;
pub mod legendary;
pub mod library;
pub mod platform;
pub mod runtime;
pub mod settings;

use platform::Platform;
use settings::SettingsStore;

/// Application state shared by every Tauri command.
pub struct AppState {
    pub platform: Box<dyn Platform>,
    pub settings: SettingsStore,
}

/// Builds and runs the Tauri application.
///
/// Startup is deliberately cheap: resolve directories, read one small JSON
/// file, show the window. Anything slower (runtime provisioning, library
/// refresh) happens on demand with progress reported to the UI.
pub fn run() {
    init_tracing();

    let platform = match platform::current() {
        Ok(platform) => platform,
        Err(e) => {
            // Without data directories there is nothing meaningful to show.
            eprintln!("Wisp cannot start: {e}");
            std::process::exit(1);
        }
    };

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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { platform, settings })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::set_language,
            commands::set_theme,
            commands::set_onboarding_completed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wisp");
}

/// Logs to stderr, filtered by `WISP_LOG` (defaults to `info`).
/// File logging arrives with the settings page in sprint 5.
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_env("WISP_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}
