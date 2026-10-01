//! Helpers for reading Legendary's `--json` output.
//!
//! Legendary dumps its Python dataclasses straight to JSON, so the shapes are
//! stable but wide, and new fields appear between releases. Everything here
//! reads defensively: a missing or unexpected field yields `None` rather than
//! failing the whole parse, because one odd entry must not hide a library.

use serde_json::Value;

/// `value["a"]["b"]` as a string, if every step exists.
pub fn path_str<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current.as_str()
}

/// Epic stores many flags as `{"customAttributes": {"Name": {"value": "x"}}}`.
pub fn custom_attribute<'a>(metadata: &'a Value, name: &str) -> Option<&'a str> {
    path_str(metadata, &["customAttributes", name, "value"])
}

pub fn has_custom_attribute(metadata: &Value, name: &str) -> bool {
    metadata
        .get("customAttributes")
        .and_then(|attrs| attrs.get(name))
        .is_some()
}

/// Mirrors `Game.is_dlc`: DLC entries carry a reference to their base game.
pub fn is_dlc(metadata: &Value) -> bool {
    metadata.get("mainGameItem").is_some()
}

/// App name of the base game, for a DLC entry.
pub fn dlc_parent(metadata: &Value) -> Option<String> {
    metadata
        .get("mainGameItem")?
        .get("releaseInfo")?
        .as_array()?
        .first()?
        .get("appId")?
        .as_str()
        .map(str::to_owned)
}

/// Mirrors `Game.third_party_store`: set when the title is really managed by
/// Origin, Ubisoft Connect or similar, and cannot simply be installed.
pub fn third_party_store(metadata: &Value) -> Option<String> {
    custom_attribute(metadata, "ThirdPartyManagedApp")
        .or_else(|| custom_attribute(metadata, "ThirdPartyManagedProvider"))
        .map(str::to_owned)
}

/// Mirrors `Game.supports_cloud_saves`.
pub fn supports_cloud_saves(metadata: &Value) -> bool {
    has_custom_attribute(metadata, "CloudSaveFolder")
}

/// Build version Epic currently offers for a platform, from `asset_infos`.
pub fn available_version(entry: &Value, platform: &str) -> Option<String> {
    path_str(entry, &["asset_infos", platform, "build_version"]).map(str::to_owned)
}

/// Cover art, preferring portrait art because the library is a grid of tiles.
///
/// Epic publishes several images per title under `keyImages` and not every
/// title has every type, so this walks a preference list and takes the first
/// one that exists.
pub fn cover_url(metadata: &Value) -> Option<String> {
    const PREFERRED: [&str; 6] = [
        "DieselGameBoxTall",
        "OfferImageTall",
        "Thumbnail",
        "DieselGameBox",
        "OfferImageWide",
        "DieselStoreFrontTall",
    ];

    let images = metadata.get("keyImages")?.as_array()?;
    for wanted in PREFERRED {
        let found = images.iter().find(|image| {
            image.get("type").and_then(Value::as_str) == Some(wanted)
                && image.get("url").and_then(Value::as_str).is_some()
        });
        if let Some(image) = found {
            return image.get("url").and_then(Value::as_str).map(str::to_owned);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metadata() -> Value {
        json!({
            "title": "A Game",
            "developer": "A Studio",
            "customAttributes": {
                "CloudSaveFolder": { "type": "STRING", "value": "{appdata}/saves" },
                "ThirdPartyManagedApp": { "type": "STRING", "value": "Origin" }
            },
            "keyImages": [
                { "type": "DieselGameBox", "url": "https://img/wide.jpg" },
                { "type": "DieselGameBoxTall", "url": "https://img/tall.jpg" }
            ],
            "mainGameItem": {
                "releaseInfo": [{ "appId": "BaseGameApp" }]
            }
        })
    }

    #[test]
    fn prefers_portrait_cover_art() {
        assert_eq!(
            cover_url(&metadata()).as_deref(),
            Some("https://img/tall.jpg")
        );
    }

    #[test]
    fn falls_back_through_the_preference_list() {
        let only_wide =
            json!({ "keyImages": [{ "type": "DieselGameBox", "url": "https://img/w.jpg" }] });
        assert_eq!(cover_url(&only_wide).as_deref(), Some("https://img/w.jpg"));
    }

    #[test]
    fn missing_images_are_not_an_error() {
        assert_eq!(cover_url(&json!({})), None);
        assert_eq!(cover_url(&json!({ "keyImages": [] })), None);
        assert_eq!(
            cover_url(&json!({ "keyImages": [{ "type": "Thumbnail" }] })),
            None
        );
    }

    #[test]
    fn reads_dlc_relationship() {
        assert!(is_dlc(&metadata()));
        assert_eq!(dlc_parent(&metadata()).as_deref(), Some("BaseGameApp"));
        assert!(!is_dlc(&json!({ "title": "Base" })));
        assert_eq!(dlc_parent(&json!({ "mainGameItem": {} })), None);
    }

    #[test]
    fn reads_custom_attributes() {
        assert!(supports_cloud_saves(&metadata()));
        assert_eq!(third_party_store(&metadata()).as_deref(), Some("Origin"));
        assert!(!supports_cloud_saves(&json!({})));
        assert_eq!(third_party_store(&json!({})), None);
    }

    #[test]
    fn reads_the_available_build_version() {
        let entry = json!({
            "asset_infos": {
                "Windows": { "build_version": "1.2.3" },
                "Mac": { "build_version": "1.2.0" }
            }
        });
        assert_eq!(
            available_version(&entry, "Windows").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(available_version(&entry, "Linux"), None);
        assert_eq!(available_version(&json!({}), "Windows"), None);
    }
}
