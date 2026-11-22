//! The one error type every command returns: a stable code the UI translates
//! (`errors.<code>`), plus English text for logs and as a fallback.

use ka_store::StoreError;
use serde::{Deserialize, Serialize};

/// Stable, machine-readable error code. Serialized in snake_case: **never rename a
/// variant**, only add new ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum AppErrorCode {
    /// No such file or entry.
    NotFound,
    /// A command argument is malformed.
    InvalidInput,
    /// A database error (user.db).
    Store,
    /// Reading or writing a file failed (permissions, disk full, locked).
    Io,
    /// A `.kbarch` file is larger than the cap.
    FileTooLarge,
    /// A `.kbarch` file is not valid JSON, or has unknown or missing fields.
    InvalidFile,
    /// A `.kbarch` file was written by a newer version of the app.
    NewerFileSchema,
    /// The design breaks a rule (wiring, ids, prop ranges, …).
    InvalidDesign,
    /// The UI may not read or write this setting.
    SettingNotAllowed,
    /// The UI asked to open or save a path it was not given by a dialog or Recents.
    PathNotAllowed,
    /// Anything else, including the app failing to start.
    Internal,
    /// A request can't get through the design (no path from a client to a service).
    NoPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[error("{code:?}: {message}")]
pub struct AppError {
    pub code: AppErrorCode,
    /// English text: for logs, the developer console and as a fallback.
    pub message: String,
}

impl AppError {
    pub fn new(code: AppErrorCode, message: impl Into<String>) -> Self {
        AppError {
            code,
            message: message.into(),
        }
    }
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        AppError::new(AppErrorCode::Store, e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        let code = match e.kind() {
            std::io::ErrorKind::NotFound => AppErrorCode::NotFound,
            _ => AppErrorCode::Io,
        };
        AppError::new(code, e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable_snake_case() {
        let e = AppError::new(AppErrorCode::NewerFileSchema, "x");
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({"code": "newer_file_schema", "message": "x"})
        );
    }

    #[test]
    fn io_not_found_maps_to_not_found() {
        let e: AppError = std::io::Error::from(std::io::ErrorKind::NotFound).into();
        assert_eq!(e.code, AppErrorCode::NotFound);
        let e: AppError = std::io::Error::from(std::io::ErrorKind::PermissionDenied).into();
        assert_eq!(e.code, AppErrorCode::Io);
    }
}
