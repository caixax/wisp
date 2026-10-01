//! Adapter around the Legendary CLI (sprint 1+).
//!
//! This is the only module allowed to spawn the Legendary process. Everything
//! else in the app talks to the typed API exposed here, which keeps command
//! line flags, JSON shapes and progress parsing in one place.
