//! KataBox Arch in the browser: the simulator compiled to `wasm32-unknown-unknown`.
//!
//! No file system, no clock, no threads. The viewer API (M13) is added here; until then the crate only proves
//! that the simulator builds for the target.

/// The simulator's registry as JSON, so the viewer and the desktop app read the same data.
pub fn registry_json() -> String {
    serde_json::to_string(&ka_core::registry::registry()).unwrap_or_default()
}
