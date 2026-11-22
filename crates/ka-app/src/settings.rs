//! Which settings the UI may read and write. Anything else is refused, so the webview
//! can't fill user.db with arbitrary keys or overwrite what only Rust writes
//! (`window.bounds`).

/// Keys the UI may get and set.
pub const UI_SETTINGS: &[&str] = &[
    "theme",
    "ui_lang",
    "ui.contrast",
    "ui.motion",
    "sandbox.layout",
    "canvas.snap",
    // Home offered the first-run tour (shown once; M3.3).
    "tour.offered",
];

/// The largest JSON value the UI may store, in bytes.
pub const MAX_SETTING_BYTES: usize = 16 * 1024;

pub fn ui_may_access(key: &str) -> bool {
    UI_SETTINGS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_bounds_is_rust_only() {
        assert!(!ui_may_access(crate::window::WINDOW_BOUNDS_KEY));
        assert!(ui_may_access("theme"));
        assert!(ui_may_access("tour.offered"));
        assert!(!ui_may_access("theme2"));
        assert!(!ui_may_access(""));
    }
}
