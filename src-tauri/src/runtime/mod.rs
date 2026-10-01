//! Legendary runtime management (sprint 1).
//!
//! Wisp does not depend on Python being installed. It downloads the official
//! standalone Legendary binary published on GitHub Releases into
//! [`WispPaths::runtime`], verifying it against the SHA-256 digest the GitHub
//! API reports for the asset, and runs that.
//!
//! The same mechanism works on every platform Legendary builds for
//! (`legendary_windows_x64.exe`, `legendary_linux_x64`, ...), so the only
//! platform-specific part is which asset name to ask for.
//!
//! Updates are checked against the releases API and never applied without
//! asking first. Uninstalling Wisp leaves nothing behind.
//!
//! [`WispPaths::runtime`]: crate::platform::WispPaths::runtime
