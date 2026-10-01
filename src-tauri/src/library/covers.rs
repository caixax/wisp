//! On-disk cache for cover art.
//!
//! Epic serves cover images from its CDN. Downloading them on every start
//! would be slow and would leak a request per game to Epic, so each image is
//! fetched once and kept in Wisp's cache folder. The files are regenerable:
//! deleting the folder only costs one round of downloads.

use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const USER_AGENT: &str = concat!("Wisp/", env!("CARGO_PKG_VERSION"));

/// Cover art is small. Anything larger is not an image we want to cache.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct CoverCache {
    directory: PathBuf,
}

impl CoverCache {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            directory: cache_dir.join("covers"),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Where the cover for `app_name` lives.
    ///
    /// The name is derived, never taken from input, so a hostile app name
    /// cannot escape the cache folder. A short hash keeps names unique after
    /// the readable part has been sanitised.
    pub fn path(&self, app_name: &str) -> PathBuf {
        self.directory.join(file_name(app_name))
    }

    pub fn has(&self, app_name: &str) -> bool {
        self.path(app_name).is_file()
    }

    /// Downloads one cover, replacing any existing file only once the new one
    /// has arrived in full.
    pub fn fetch(&self, app_name: &str, url: &str) -> Result<()> {
        std::fs::create_dir_all(&self.directory)?;

        let mut response = ureq::get(url)
            .header("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| Error::Http(format!("could not fetch cover art: {e}")))?;

        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|e| Error::Http(format!("could not read cover art: {e}")))?;

        if bytes.is_empty() {
            return Err(Error::Http("the cover art response was empty".into()));
        }

        let destination = self.path(app_name);
        let temporary = destination.with_extension("part");
        std::fs::write(&temporary, &bytes)?;
        std::fs::rename(&temporary, &destination)?;
        Ok(())
    }

    /// Removes every cached image.
    pub fn clear(&self) -> Result<()> {
        if self.directory.is_dir() {
            std::fs::remove_dir_all(&self.directory)?;
        }
        Ok(())
    }
}

fn file_name(app_name: &str) -> String {
    let safe: String = app_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(48)
        .collect();

    let digest = Sha256::digest(app_name.as_bytes());
    let suffix: String = digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect();

    format!("{safe}-{suffix}")
}

/// Content type for bytes already on disk, from their magic number.
/// Epic serves JPEG, PNG and the occasional WebP.
pub fn content_type(bytes: &[u8]) -> &'static str {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        [b'G', b'I', b'F', b'8', ..] => "image/gif",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_stay_inside_the_cache_folder() {
        let cache = CoverCache::new(Path::new("X:/cache"));

        for hostile in ["../../etc/passwd", "..\\..\\windows", "/absolute", "a/b/c"] {
            let path = cache.path(hostile);
            assert_eq!(
                path.parent(),
                Some(cache.directory()),
                "{hostile} escaped the cache folder"
            );
        }
    }

    #[test]
    fn different_app_names_get_different_files() {
        let cache = CoverCache::new(Path::new("X:/cache"));
        assert_ne!(cache.path("Fortnite"), cache.path("Fortnite2"));
        // Names that sanitise to the same readable part stay distinct.
        assert_ne!(cache.path("a/b"), cache.path("a-b"));
    }

    #[test]
    fn the_same_app_name_always_maps_to_the_same_file() {
        let cache = CoverCache::new(Path::new("X:/cache"));
        assert_eq!(cache.path("Fortnite"), cache.path("Fortnite"));
    }

    #[test]
    fn sniffs_the_image_types_epic_serves() {
        assert_eq!(content_type(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(content_type(b"\x89PNG\r\n\x1a\n"), "image/png");
        assert_eq!(content_type(b"RIFF\0\0\0\0WEBPVP8 "), "image/webp");
        assert_eq!(content_type(b"nonsense"), "application/octet-stream");
        assert_eq!(content_type(&[]), "application/octet-stream");
    }
}
