//! Minimal GitHub Releases client, used only to fetch Legendary.
//!
//! Blocking on purpose: these calls run on a blocking task, which keeps the
//! HTTP stack small (no async runtime plumbing for two requests) and keeps
//! the download loop easy to follow.

use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// Legendary moved from `derrod/legendary` to the `legendary-gl` organisation;
/// the old URL still redirects, but point at the current one.
const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/legendary-gl/legendary/releases/latest";

const USER_AGENT: &str = concat!("Wisp/", env!("CARGO_PKG_VERSION"));

/// Read in chunks rather than into memory: the binary is around 17 MB and
/// progress has to be reported while it arrives.
const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
    /// GitHub reports this as `sha256:<hex>`. Older releases may not have one.
    #[serde(default)]
    pub digest: Option<String>,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.name == name)
    }
}

impl Asset {
    /// The published SHA-256, without the `sha256:` prefix.
    pub fn sha256(&self) -> Option<&str> {
        self.digest
            .as_deref()
            .and_then(|digest| digest.strip_prefix("sha256:"))
            .filter(|hex| hex.len() == 64)
    }
}

/// Fetches the most recent Legendary release.
pub fn latest_release() -> Result<Release> {
    let mut response = ureq::get(LATEST_RELEASE_URL)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| Error::Http(format!("could not reach the GitHub release API: {e}")))?;

    response.body_mut().read_json::<Release>().map_err(|e| {
        Error::Http(format!(
            "the GitHub release API returned unexpected data: {e}"
        ))
    })
}

/// Downloads `asset` to `destination`, hashing it on the way through.
///
/// `on_progress` receives bytes written so far and the expected total when the
/// server declares one. Returns the hex SHA-256 of what was written.
pub fn download(
    asset: &Asset,
    destination: &Path,
    on_progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<String> {
    let mut response = ureq::get(&asset.browser_download_url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| Error::Http(format!("could not download {}: {e}", asset.name)))?;

    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .or(Some(asset.size).filter(|size| *size > 0));

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(destination)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    let mut written: u64 = 0;

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| Error::Http(format!("the download was interrupted: {e}")))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        written += read as u64;
        on_progress(written, total);
    }

    file.flush()?;
    // Make sure the bytes are on disk before anything renames the file.
    file.sync_all()?;

    Ok(to_hex(&hasher.finalize()))
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(digest: Option<&str>) -> Asset {
        Asset {
            name: "legendary_windows_x64.exe".into(),
            browser_download_url: "https://example.invalid/legendary".into(),
            size: 17_654_404,
            digest: digest.map(str::to_owned),
        }
    }

    #[test]
    fn strips_the_algorithm_prefix_from_the_digest() {
        let hex = "6be77857dc0a6dd33ea7442ce8b5291944ec6da53757f81ca95b57ce794baf5b";
        assert_eq!(asset(Some(&format!("sha256:{hex}"))).sha256(), Some(hex));
    }

    #[test]
    fn ignores_digests_it_cannot_use() {
        assert_eq!(asset(None).sha256(), None);
        assert_eq!(asset(Some("md5:abc")).sha256(), None);
        assert_eq!(asset(Some("sha256:tooshort")).sha256(), None);
    }

    #[test]
    fn finds_the_asset_for_this_platform() {
        let release = Release {
            tag_name: "0.21.1".into(),
            published_at: None,
            assets: vec![asset(None)],
        };
        assert!(release.asset("legendary_windows_x64.exe").is_some());
        assert!(release.asset("legendary_linux_x64").is_none());
    }

    #[test]
    fn hex_encoding_is_lowercase_and_padded() {
        assert_eq!(to_hex(&[0x00, 0x0f, 0xff]), "000fff");
    }

    #[test]
    fn parses_a_real_release_payload() {
        let json = r#"{
            "tag_name": "0.21.1",
            "published_at": "2026-09-08T21:07:05Z",
            "assets": [
                {
                    "name": "legendary_windows_x64.exe",
                    "browser_download_url": "https://example.invalid/x.exe",
                    "size": 17654404,
                    "digest": "sha256:6be77857dc0a6dd33ea7442ce8b5291944ec6da53757f81ca95b57ce794baf5b",
                    "unexpected_field": true
                }
            ]
        }"#;

        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "0.21.1");
        assert_eq!(release.assets[0].size, 17_654_404);
        assert!(release
            .asset("legendary_windows_x64.exe")
            .unwrap()
            .sha256()
            .is_some());
    }
}
