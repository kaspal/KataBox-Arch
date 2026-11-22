//! Storage for KataBox Arch. M0 has one SQLite file, `user.db` ([`UserDb`]): read-write,
//! WAL mode, numbered migrations embedded in the binary and applied at open after a
//! backup copy. It holds settings and the recent-files list. `content.db` ([`ContentDb`])
//! is the read-only catalogue the app ships (built by `ka-pack`).
//!
//! Times are UTC: `INTEGER` milliseconds since the Unix epoch.
//! sqlx is used with runtime queries only, so building needs no `DATABASE_URL`.

pub mod content;
#[cfg(test)]
mod migration_tests;
pub mod packs;
pub mod practice;
pub mod progress;
pub mod user;

pub use content::{
    CONTENT_SCHEMA, ConceptRow, ContentData, ContentDb, DocRow, GuideRow, JsonRow, PatternRow,
    ProblemRow, SampleRow, write_content,
};
pub use packs::PackRow;
pub use practice::{EstimateStatus, ReviewCardRow};
pub use progress::{NewSubmission, ProblemStatus, Submission, SubmissionSummary, day_number};
pub use user::{BACKUPS_KEPT, RECENT_KEPT, RecentFile, UserDb};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("user.db schema version {found} is newer than this app supports ({supported})")]
    NewerSchema { found: i64, supported: i64 },
    #[error("unexpected data in the database: {0}")]
    Corrupt(String),
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

/// Now, in UTC milliseconds since the Unix epoch.
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) mod testutil {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh, empty temp folder for one test.
    pub fn temp_dir(name: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ka-store-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
