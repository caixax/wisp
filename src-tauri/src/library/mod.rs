//! Game library: owned titles, DLC, installed versions and cover art.
//!
//! Legendary is the source of truth, but calling it costs about a second per
//! invocation, so its raw `--json` output is cached on disk. Startup reads
//! the cache and shows the library immediately; a refresh re-runs Legendary
//! and rewrites the cache.
//!
//! The raw output is what gets cached, not a digested form. Deriving the
//! model on demand means a change to how Wisp presents a game never requires
//! invalidating a user's cache.

pub mod covers;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::error::{Error, Result};
use crate::legendary::{json, Legendary, GAME_PLATFORM};

const CATALOG_FILE: &str = "catalog.json";
const INSTALLED_FILE: &str = "installed.json";
const STAMP_FILE: &str = "library-stamp.json";

/// Where a game is installed, when it is.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledInfo {
    pub version: String,
    pub install_path: String,
    pub install_size: u64,
    pub can_run_offline: bool,
    pub needs_verification: bool,
    pub platform: String,
    pub executable: Option<String>,
    pub save_path: Option<String>,
}

/// One row in the library grid.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    pub app_name: String,
    pub title: String,
    pub developer: Option<String>,
    /// Build version Epic currently offers, if the title has an asset for
    /// this platform. Titles without one cannot be installed by Wisp.
    pub available_version: Option<String>,
    pub installed: Option<InstalledInfo>,
    pub update_available: bool,
    pub has_cover: bool,
    pub cloud_saves: bool,
    /// Set for titles that really live on Origin, Ubisoft Connect and such.
    pub third_party_store: Option<String>,
    pub installable: bool,
    pub dlc_count: usize,
    pub installed_dlc_count: usize,
}

/// A DLC entry, as shown on a game's page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DlcSummary {
    pub app_name: String,
    pub title: String,
    pub available_version: Option<String>,
    pub installed: Option<InstalledInfo>,
    pub update_available: bool,
}

/// Everything the game page shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetails {
    #[serde(flatten)]
    pub summary: GameSummary,
    pub description: Option<String>,
    pub dlcs: Vec<DlcSummary>,
}

/// Summary of the whole library, for the header and filters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub games: Vec<GameSummary>,
    pub installed_count: usize,
    pub update_count: usize,
    /// Unix seconds of the last successful refresh, if there has been one.
    pub updated_at: Option<u64>,
    /// False before the first refresh, so the UI can tell "empty library"
    /// apart from "not fetched yet".
    pub fetched: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Stamp {
    updated_at: Option<u64>,
}

/// Parsed Legendary output plus everything derived from it.
#[derive(Debug, Default)]
pub struct Catalog {
    /// Raw entries from `legendary list --json`.
    entries: Vec<Value>,
    /// Raw entries from `legendary list-installed --json`, by app name.
    installed: HashMap<String, Value>,
    updated_at: Option<u64>,
    fetched: bool,
}

impl Catalog {
    pub fn from_raw(entries: Vec<Value>, installed_entries: Vec<Value>) -> Self {
        let installed = installed_entries
            .into_iter()
            .filter_map(|entry| {
                let name = entry.get("app_name")?.as_str()?.to_owned();
                Some((name, entry))
            })
            .collect();

        Self {
            entries,
            installed,
            updated_at: None,
            fetched: true,
        }
    }

    fn installed_info(&self, app_name: &str) -> Option<InstalledInfo> {
        let entry = self.installed.get(app_name)?;
        Some(InstalledInfo {
            version: entry
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            install_path: entry
                .get("install_path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            install_size: entry
                .get("install_size")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            can_run_offline: entry
                .get("can_run_offline")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            needs_verification: entry
                .get("needs_verification")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            platform: entry
                .get("platform")
                .and_then(Value::as_str)
                .unwrap_or(GAME_PLATFORM)
                .to_owned(),
            executable: entry
                .get("executable")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            save_path: entry
                .get("save_path")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        })
    }

    fn dlc_entries(entry: &Value) -> &[Value] {
        entry
            .get("dlcs")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn summary(&self, entry: &Value, covers: &covers::CoverCache) -> Option<GameSummary> {
        let app_name = entry.get("app_name")?.as_str()?.to_owned();
        let metadata = entry.get("metadata").cloned().unwrap_or(Value::Null);

        let title = entry
            .get("app_title")
            .and_then(Value::as_str)
            .or_else(|| metadata.get("title").and_then(Value::as_str))
            .unwrap_or(&app_name)
            .trim()
            .to_owned();

        let available_version = json::available_version(entry, GAME_PLATFORM);
        let installed = self.installed_info(&app_name);
        let third_party_store = json::third_party_store(&metadata);

        let dlcs = Self::dlc_entries(entry);
        let installed_dlc_count = dlcs
            .iter()
            .filter(|dlc| {
                dlc.get("app_name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| self.installed.contains_key(name))
            })
            .count();

        Some(GameSummary {
            update_available: update_available(installed.as_ref(), available_version.as_deref()),
            // Epic lists titles with no build for this platform, and titles
            // that are really managed by another launcher. Wisp can show them
            // but cannot install them.
            installable: available_version.is_some() && third_party_store.is_none(),
            has_cover: covers.has(&app_name),
            cloud_saves: json::supports_cloud_saves(&metadata),
            developer: metadata
                .get("developer")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            dlc_count: dlcs.len(),
            installed_dlc_count,
            app_name,
            title,
            available_version,
            installed,
            third_party_store,
        })
    }

    /// Every owned title, DLC excluded, sorted by title.
    pub fn games(&self, covers: &covers::CoverCache) -> Vec<GameSummary> {
        let mut games: Vec<GameSummary> = self
            .entries
            .iter()
            .filter(|entry| {
                let metadata = entry.get("metadata").unwrap_or(&Value::Null);
                !json::is_dlc(metadata)
            })
            .filter_map(|entry| self.summary(entry, covers))
            .collect();

        games.sort_by_key(|game| game.title.to_lowercase());
        games
    }

    pub fn snapshot(&self, covers: &covers::CoverCache) -> LibrarySnapshot {
        let games = self.games(covers);
        LibrarySnapshot {
            installed_count: games.iter().filter(|game| game.installed.is_some()).count(),
            update_count: games.iter().filter(|game| game.update_available).count(),
            updated_at: self.updated_at,
            fetched: self.fetched,
            games,
        }
    }

    pub fn details(&self, app_name: &str, covers: &covers::CoverCache) -> Option<GameDetails> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.get("app_name").and_then(Value::as_str) == Some(app_name))?;

        let summary = self.summary(entry, covers)?;
        let metadata = entry.get("metadata").unwrap_or(&Value::Null);

        let dlcs = Self::dlc_entries(entry)
            .iter()
            .filter_map(|dlc| {
                let name = dlc.get("app_name")?.as_str()?.to_owned();
                let available_version = json::available_version(dlc, GAME_PLATFORM);
                let installed = self.installed_info(&name);
                Some(DlcSummary {
                    update_available: update_available(
                        installed.as_ref(),
                        available_version.as_deref(),
                    ),
                    title: dlc
                        .get("app_title")
                        .and_then(Value::as_str)
                        .unwrap_or(&name)
                        .trim()
                        .to_owned(),
                    app_name: name,
                    available_version,
                    installed,
                })
            })
            .collect();

        Some(GameDetails {
            summary,
            description: metadata
                .get("description")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            dlcs,
        })
    }

    /// Covers that are owned but not cached yet, as (app name, url) pairs.
    pub fn missing_covers(&self, covers: &covers::CoverCache) -> Vec<(String, String)> {
        let mut seen = HashSet::new();
        let mut missing = Vec::new();

        for entry in &self.entries {
            let Some(app_name) = entry.get("app_name").and_then(Value::as_str) else {
                continue;
            };
            if json::is_dlc(entry.get("metadata").unwrap_or(&Value::Null)) {
                continue;
            }
            if !seen.insert(app_name.to_owned()) || covers.has(app_name) {
                continue;
            }
            if let Some(url) = json::cover_url(entry.get("metadata").unwrap_or(&Value::Null)) {
                missing.push((app_name.to_owned(), url));
            }
        }

        missing
    }
}

/// An installed build that no longer matches what Epic offers needs updating.
/// A title with no available version (delisted, or not on this platform)
/// is never reported as out of date.
fn update_available(installed: Option<&InstalledInfo>, available: Option<&str>) -> bool {
    match (installed, available) {
        (Some(installed), Some(available)) => {
            !available.is_empty() && installed.version != available
        }
        _ => false,
    }
}

/// Reads and writes the cached Legendary output.
#[derive(Debug, Clone)]
pub struct LibraryStore {
    directory: PathBuf,
}

impl LibraryStore {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            directory: cache_dir.to_path_buf(),
        }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    /// Loads whatever was cached. A missing or unreadable cache is simply an
    /// empty library, not an error: a refresh will fix it.
    pub fn load(&self) -> Catalog {
        let entries = read_array(&self.file(CATALOG_FILE));
        let installed = read_array(&self.file(INSTALLED_FILE));

        let fetched = self.file(CATALOG_FILE).is_file();
        let stamp: Stamp = std::fs::read_to_string(self.file(STAMP_FILE))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

        let mut catalog = Catalog::from_raw(entries, installed);
        catalog.updated_at = stamp.updated_at;
        catalog.fetched = fetched;
        catalog
    }

    pub fn save(&self, entries: &[Value], installed: &[Value]) -> Result<u64> {
        std::fs::create_dir_all(&self.directory)?;

        write_json(&self.file(CATALOG_FILE), entries)?;
        write_json(&self.file(INSTALLED_FILE), installed)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        write_json(
            &self.file(STAMP_FILE),
            &Stamp {
                updated_at: Some(now),
            },
        )?;

        Ok(now)
    }

    pub fn clear(&self) -> Result<()> {
        for name in [CATALOG_FILE, INSTALLED_FILE, STAMP_FILE] {
            let path = self.file(name);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}

fn read_array(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<Value>>(&raw).ok())
        .unwrap_or_default()
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, serde_json::to_vec(value)?)?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

/// Event carrying cover download progress.
pub const COVERS_EVENT: &str = "library:covers";

/// How many covers are fetched at once. Enough to fill a library quickly
/// without hammering Epic's CDN.
const COVER_CONCURRENCY: usize = 6;

/// Progress of the background cover download.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverProgress {
    pub done: usize,
    pub total: usize,
}

/// The library as the rest of the app sees it: a cached catalogue that can be
/// refreshed from Legendary.
pub struct LibraryService {
    store: LibraryStore,
    covers: covers::CoverCache,
    catalog: tokio::sync::RwLock<Catalog>,
    /// Held while a refresh runs so two cannot overlap.
    refreshing: tokio::sync::Mutex<()>,
}

impl LibraryService {
    /// Loads whatever is cached. Never fails: an unreadable cache is an
    /// empty library.
    pub fn new(cache_dir: &Path) -> Self {
        let store = LibraryStore::new(cache_dir);
        let catalog = store.load();
        Self {
            store,
            covers: covers::CoverCache::new(cache_dir),
            catalog: tokio::sync::RwLock::new(catalog),
            refreshing: tokio::sync::Mutex::new(()),
        }
    }

    pub fn covers(&self) -> &covers::CoverCache {
        &self.covers
    }

    pub async fn snapshot(&self) -> LibrarySnapshot {
        self.catalog.read().await.snapshot(&self.covers)
    }

    pub async fn details(&self, app_name: &str) -> Option<GameDetails> {
        self.catalog.read().await.details(app_name, &self.covers)
    }

    /// Re-reads the library from Legendary and rewrites the cache.
    ///
    /// Cover art is fetched afterwards in the background: the grid should
    /// appear as soon as the titles are known, not once every image has
    /// been downloaded.
    pub async fn refresh(
        &self,
        app: &AppHandle,
        legendary: &Legendary,
        force: bool,
    ) -> Result<LibrarySnapshot> {
        let Ok(_guard) = self.refreshing.try_lock() else {
            return Err(Error::Legendary(
                "the library is already being refreshed".into(),
            ));
        };

        let entries = legendary.list(force).await?;
        let installed = legendary.list_installed().await.unwrap_or_else(|e| {
            // A fresh account has nothing installed, and Legendary has been
            // known to complain rather than print an empty list.
            tracing::debug!("no installed games reported: {e}");
            Vec::new()
        });

        let updated_at = self.store.save(&entries, &installed)?;

        let mut catalog = Catalog::from_raw(entries, installed);
        catalog.updated_at = Some(updated_at);

        let missing = catalog.missing_covers(&self.covers);
        let snapshot = catalog.snapshot(&self.covers);
        *self.catalog.write().await = catalog;

        if !missing.is_empty() {
            spawn_cover_downloads(app.clone(), self.covers.clone(), missing);
        }

        Ok(snapshot)
    }

    /// Forgets the cached library and every cover. Used on sign out, so one
    /// user's library is never shown to the next.
    pub async fn clear(&self) -> Result<()> {
        self.store.clear()?;
        self.covers.clear()?;
        *self.catalog.write().await = Catalog::default();
        Ok(())
    }
}

/// Downloads the covers that are not cached yet, a few at a time, reporting
/// progress as it goes. Failures are logged and skipped: a missing image is
/// a placeholder tile, not a broken library.
fn spawn_cover_downloads(
    app: AppHandle,
    cache: covers::CoverCache,
    missing: Vec<(String, String)>,
) {
    tauri::async_runtime::spawn(async move {
        let total = missing.len();
        let permits = Arc::new(tokio::sync::Semaphore::new(COVER_CONCURRENCY));
        let done = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::with_capacity(total);

        for (app_name, url) in missing {
            let permits = Arc::clone(&permits);
            let done = Arc::clone(&done);
            let cache = cache.clone();

            tasks.push(tauri::async_runtime::spawn(async move {
                let _permit = permits.acquire().await;
                let fetched = tauri::async_runtime::spawn_blocking(move || {
                    if let Err(e) = cache.fetch(&app_name, &url) {
                        tracing::debug!(game = %app_name, "no cover art: {e}");
                    }
                })
                .await;
                if fetched.is_err() {
                    tracing::debug!("a cover download task did not finish");
                }
                done.fetch_add(1, Ordering::Relaxed);
            }));
        }

        // Report while the downloads run, then once more at the end so the
        // UI always sees a final, complete count.
        let reporter = {
            let done = Arc::clone(&done);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                while done.load(Ordering::Relaxed) < total {
                    tokio::time::sleep(COVER_PROGRESS_INTERVAL).await;
                    let _ = app.emit(
                        COVERS_EVENT,
                        CoverProgress {
                            done: done.load(Ordering::Relaxed).min(total),
                            total,
                        },
                    );
                }
            })
        };

        for task in tasks {
            let _ = task.await;
        }
        reporter.abort();

        let _ = app.emit(COVERS_EVENT, CoverProgress { done: total, total });
        tracing::info!(covers = total, "finished downloading cover art");
    });
}

const COVER_PROGRESS_INTERVAL: std::time::Duration = std::time::Duration::from_millis(400);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(app_name: &str, title: &str, version: Option<&str>) -> Value {
        json!({
            "app_name": app_name,
            "app_title": title,
            "asset_infos": version
                .map(|v| json!({ "Windows": { "build_version": v } }))
                .unwrap_or(json!({})),
            "metadata": {
                "title": title,
                "developer": "A Studio",
                "description": "Words about the game.",
                "keyImages": [{ "type": "DieselGameBoxTall", "url": "https://img/tall.jpg" }]
            },
            "dlcs": []
        })
    }

    fn installed(app_name: &str, version: &str, size: u64) -> Value {
        json!({
            "app_name": app_name,
            "title": "whatever",
            "version": version,
            "install_path": "C:/Games/Thing",
            "install_size": size,
            "can_run_offline": true,
            "needs_verification": false,
            "platform": "Windows",
            "executable": "thing.exe"
        })
    }

    fn cache() -> covers::CoverCache {
        covers::CoverCache::new(Path::new("X:/cache"))
    }

    #[test]
    fn lists_games_sorted_by_title_ignoring_case() {
        let catalog = Catalog::from_raw(
            vec![
                game("c", "zebra", Some("1")),
                game("a", "Apple", Some("1")),
                game("b", "mango", Some("1")),
            ],
            vec![],
        );

        let titles: Vec<_> = catalog
            .games(&cache())
            .into_iter()
            .map(|g| g.title)
            .collect();
        assert_eq!(titles, ["Apple", "mango", "zebra"]);
    }

    #[test]
    fn merges_installed_state_into_the_summary() {
        let catalog = Catalog::from_raw(
            vec![game("Thing", "Thing", Some("2.0"))],
            vec![installed("Thing", "1.0", 1024)],
        );

        let games = catalog.games(&cache());
        let thing = &games[0];
        assert!(thing.update_available, "1.0 installed, 2.0 available");
        assert_eq!(thing.installed.as_ref().unwrap().install_size, 1024);
        assert_eq!(thing.available_version.as_deref(), Some("2.0"));
    }

    #[test]
    fn a_matching_version_is_not_an_update() {
        let catalog = Catalog::from_raw(
            vec![game("Thing", "Thing", Some("2.0"))],
            vec![installed("Thing", "2.0", 1024)],
        );
        assert!(!catalog.games(&cache())[0].update_available);
    }

    #[test]
    fn a_title_with_no_build_is_never_out_of_date_and_is_not_installable() {
        let catalog = Catalog::from_raw(
            vec![game("Thing", "Thing", None)],
            vec![installed("Thing", "1.0", 1)],
        );
        let thing = &catalog.games(&cache())[0];
        assert!(!thing.update_available);
        assert!(!thing.installable);
    }

    #[test]
    fn third_party_titles_are_listed_but_not_installable() {
        let mut entry = game("Origin Thing", "Origin Thing", Some("1"));
        entry["metadata"]["customAttributes"] =
            json!({ "ThirdPartyManagedApp": { "value": "Origin" } });

        let catalog = Catalog::from_raw(vec![entry], vec![]);
        let thing = &catalog.games(&cache())[0];
        assert_eq!(thing.third_party_store.as_deref(), Some("Origin"));
        assert!(!thing.installable);
    }

    #[test]
    fn dlc_is_nested_under_its_game_and_kept_out_of_the_grid() {
        let mut base = game("Base", "Base Game", Some("1"));
        base["dlcs"] = json!([{
            "app_name": "Expansion",
            "app_title": "An Expansion",
            "asset_infos": { "Windows": { "build_version": "3" } },
            "metadata": { "mainGameItem": { "releaseInfo": [{ "appId": "Base" }] } }
        }]);

        let catalog = Catalog::from_raw(vec![base], vec![installed("Expansion", "2", 10)]);

        let games = catalog.games(&cache());
        assert_eq!(games.len(), 1, "DLC must not appear as its own tile");
        assert_eq!(games[0].dlc_count, 1);
        assert_eq!(games[0].installed_dlc_count, 1);

        let details = catalog.details("Base", &cache()).unwrap();
        assert_eq!(details.dlcs.len(), 1);
        assert!(details.dlcs[0].update_available, "2 installed, 3 available");
        assert_eq!(
            details.description.as_deref(),
            Some("Words about the game.")
        );
    }

    #[test]
    fn standalone_dlc_entries_are_filtered_out_of_the_grid() {
        let mut dlc = game("Expansion", "An Expansion", Some("1"));
        dlc["metadata"]["mainGameItem"] = json!({ "releaseInfo": [{ "appId": "Base" }] });

        let catalog = Catalog::from_raw(vec![game("Base", "Base", Some("1")), dlc], vec![]);
        assert_eq!(catalog.games(&cache()).len(), 1);
    }

    #[test]
    fn details_for_an_unknown_game_are_absent() {
        let catalog = Catalog::from_raw(vec![game("Base", "Base", Some("1"))], vec![]);
        assert!(catalog.details("Nope", &cache()).is_none());
    }

    #[test]
    fn missing_covers_skips_cached_and_dlc_entries() {
        let mut dlc = game("Expansion", "An Expansion", Some("1"));
        dlc["metadata"]["mainGameItem"] = json!({ "releaseInfo": [{ "appId": "Base" }] });
        let mut no_images = game("NoArt", "No Art", Some("1"));
        no_images["metadata"]["keyImages"] = json!([]);

        let catalog = Catalog::from_raw(
            vec![game("Base", "Base", Some("1")), dlc, no_images],
            vec![],
        );

        let missing = catalog.missing_covers(&cache());
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].0, "Base");
        assert_eq!(missing[0].1, "https://img/tall.jpg");
    }

    #[test]
    fn snapshot_counts_installed_and_outdated_games() {
        let catalog = Catalog::from_raw(
            vec![
                game("A", "A", Some("2")),
                game("B", "B", Some("1")),
                game("C", "C", Some("1")),
            ],
            vec![installed("A", "1", 1), installed("B", "1", 1)],
        );

        let snapshot = catalog.snapshot(&cache());
        assert_eq!(snapshot.games.len(), 3);
        assert_eq!(snapshot.installed_count, 2);
        assert_eq!(snapshot.update_count, 1);
        assert!(snapshot.fetched);
    }

    #[test]
    fn an_empty_cache_reads_as_not_fetched_yet() {
        let tmp = tempfile::tempdir().unwrap();
        let store = LibraryStore::new(tmp.path());

        let catalog = store.load();
        assert!(!catalog.fetched);
        assert!(catalog.snapshot(&cache()).games.is_empty());
    }

    #[test]
    fn saved_output_round_trips_through_the_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let store = LibraryStore::new(tmp.path());

        let entries = vec![game("A", "A Game", Some("1"))];
        let installed_entries = vec![installed("A", "1", 512)];
        let stamped = store.save(&entries, &installed_entries).unwrap();
        assert!(stamped > 0);

        let catalog = store.load();
        assert!(catalog.fetched);
        assert_eq!(catalog.updated_at, Some(stamped));

        let snapshot = catalog.snapshot(&cache());
        assert_eq!(snapshot.games.len(), 1);
        assert_eq!(snapshot.installed_count, 1);
        assert_eq!(snapshot.games[0].title, "A Game");
    }

    #[test]
    fn clearing_the_cache_leaves_an_empty_library() {
        let tmp = tempfile::tempdir().unwrap();
        let store = LibraryStore::new(tmp.path());
        store.save(&[game("A", "A", Some("1"))], &[]).unwrap();

        store.clear().unwrap();

        assert!(!store.load().fetched);
    }

    #[test]
    fn a_corrupt_cache_is_treated_as_empty_rather_than_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(CATALOG_FILE), "{not json").unwrap();

        let catalog = LibraryStore::new(tmp.path()).load();
        assert!(catalog.snapshot(&cache()).games.is_empty());
    }
}
