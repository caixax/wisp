//! Private Python + Legendary runtime management (sprint 1).
//!
//! Wisp never installs anything into the user's global Python. It provisions
//! an isolated interpreter under [`WispPaths::runtime`] and keeps Legendary
//! there, so uninstalling Wisp leaves nothing behind.
//!
//! [`WispPaths::runtime`]: crate::platform::WispPaths::runtime
