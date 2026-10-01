//! Crate-wide error type.
//!
//! Every Tauri command returns `Result<T, Error>`. `Error` serialises to a
//! stable `{ kind, message }` shape so the frontend can branch on `kind`
//! without parsing human-readable text (which is translated in the UI).

use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to (de)serialise data: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("could not resolve a required application directory: {0}")]
    Paths(String),

    #[error("{0}")]
    Config(String),

    #[error("this feature is not implemented on {platform} yet")]
    UnsupportedPlatform { platform: &'static str },
}

impl Error {
    /// Machine-readable discriminant, mirrored by `ErrorKind` in `src/lib/types.ts`.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Io(_) => "io",
            Error::Serde(_) => "serde",
            Error::Paths(_) => "paths",
            Error::Config(_) => "config",
            Error::UnsupportedPlatform { .. } => "unsupported_platform",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
