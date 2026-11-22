//! `user.db`: settings and recent files. Read-write, WAL mode, numbered migrations
//! applied at open after a backup copy (the KataBox pattern, its ADR 0010).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

use crate::{Result, StoreError, now_ms};

/// `user.db.bak-*` files kept after a migration backup, the new one included.
pub const BACKUPS_KEPT: usize = 5;

/// Entries kept in the recent-files list.
pub const RECENT_KEPT: i64 = 10;

/// One numbered migration. Versions start at 1 and are contiguous; a released
/// migration is never edited.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Migration {
    pub version: i64,
    pub sql: &'static str,
}

pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../migrations/0002_progress.sql"),
    },
];

/// A recently opened or saved `.kbarch` file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RecentFile {
    pub path: String,
    /// The design's name when it was last opened or saved.
    pub name: String,
    pub opened_at: i64,
}

/// Read-write handle to user.db. One connection: the app is the only writer, and a
/// single connection rules out SQLITE_BUSY between our own transactions.
#[derive(Debug, Clone)]
pub struct UserDb {
    pub(crate) pool: SqlitePool,
    backup: Option<PathBuf>,
}

impl UserDb {
    /// Open (creating if needed) and bring the schema up to date.
    pub async fn open(path: &Path) -> Result<UserDb> {
        Self::open_with(path, MIGRATIONS).await
    }

    pub(crate) async fn open_with(path: &Path, migrations: &[Migration]) -> Result<UserDb> {
        let existed = std::fs::metadata(path).is_ok_and(|m| m.len() > 0);
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        match migrate(&pool, path, existed, migrations).await {
            Ok(backup) => Ok(UserDb { pool, backup }),
            Err(e) => {
                pool.close().await;
                Err(e)
            }
        }
    }

    /// The backup written before migrating at this open, if any.
    pub fn backup_path(&self) -> Option<&Path> {
        self.backup.as_deref()
    }

    pub async fn schema_version(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    // --- Settings -------------------------------------------------------------------

    pub async fn get_setting(&self, key: &str) -> Result<Option<Value>> {
        let v: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(v.map(|s| serde_json::from_str(&s)).transpose()?)
    }

    pub async fn set_setting(&self, key: &str, value: &Value) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value,
                                             updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(serde_json::to_string(value)?)
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Recent files ---------------------------------------------------------------

    /// Record that `path` was opened or saved now, then keep only the newest
    /// [`RECENT_KEPT`] entries.
    pub async fn touch_recent(&self, path: &str, name: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO recent_files (path, name, opened_at) VALUES (?, ?, ?)
             ON CONFLICT (path) DO UPDATE SET name = excluded.name,
                                              opened_at = excluded.opened_at",
        )
        .bind(path)
        .bind(name)
        .bind(now_ms())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM recent_files WHERE path NOT IN
             (SELECT path FROM recent_files ORDER BY opened_at DESC, path LIMIT ?)",
        )
        .bind(RECENT_KEPT)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The recent files, newest first.
    pub async fn recent_files(&self) -> Result<Vec<RecentFile>> {
        sqlx::query("SELECT path, name, opened_at FROM recent_files ORDER BY opened_at DESC, path")
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|r| {
                Ok(RecentFile {
                    path: r.try_get("path")?,
                    name: r.try_get("name")?,
                    opened_at: r.try_get("opened_at")?,
                })
            })
            .collect()
    }

    pub async fn is_recent(&self, path: &str) -> Result<bool> {
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM recent_files WHERE path = ?")
            .bind(path)
            .fetch_one(&self.pool)
            .await?;
        Ok(n > 0)
    }

    pub async fn forget_recent(&self, path: &str) -> Result<()> {
        sqlx::query("DELETE FROM recent_files WHERE path = ?")
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Apply pending migrations; returns the backup written first, if any.
async fn migrate(
    pool: &SqlitePool,
    path: &Path,
    existed: bool,
    migrations: &[Migration],
) -> Result<Option<PathBuf>> {
    let current: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;
    let latest = migrations.last().map_or(0, |m| m.version);
    if current > latest {
        return Err(StoreError::NewerSchema {
            found: current,
            supported: latest,
        });
    }
    if current == latest {
        return Ok(None);
    }
    let backup = if existed && current > 0 {
        Some(backup(pool, path, current).await?)
    } else {
        None
    };
    for m in migrations.iter().filter(|m| m.version > current) {
        let mut tx = pool.begin().await?;
        sqlx::raw_sql(m.sql).execute(&mut *tx).await?;
        sqlx::raw_sql(&format!("PRAGMA user_version = {}", m.version))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(backup)
}

/// `VACUUM INTO <path>.bak-v<version>-<UTC time>`: a consistent copy even in WAL mode.
async fn backup(pool: &SqlitePool, path: &Path, version: i64) -> Result<PathBuf> {
    let stamp: String = sqlx::query_scalar("SELECT strftime('%Y%m%dT%H%M%SZ', 'now')")
        .fetch_one(pool)
        .await?;
    let base = format!("{}.bak-v{version}-{stamp}", path.display());
    let mut target = PathBuf::from(&base);
    let mut n = 1;
    while target.exists() {
        target = PathBuf::from(format!("{base}-{n}"));
        n += 1;
    }
    let target_str = target
        .to_str()
        .ok_or_else(|| StoreError::Corrupt(format!("non-UTF-8 path {}", target.display())))?;
    sqlx::query("VACUUM INTO ?")
        .bind(target_str)
        .execute(pool)
        .await?;
    prune_backups(path, &target);
    Ok(target)
}

/// Keep the newest [`BACKUPS_KEPT`] backups of `db` (by the UTC time in their names),
/// always including `just_written`. Best effort: a file that cannot be deleted stays,
/// and files that do not follow the backup naming pattern are never touched.
fn prune_backups(db: &Path, just_written: &Path) {
    let (Some(dir), Some(name)) = (db.parent(), db.file_name().and_then(|n| n.to_str())) else {
        return;
    };
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let prefix = format!("{name}.bak-v");
    let mut older: Vec<((String, u32), PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().into_string().ok()?;
            let key = backup_key(file.strip_prefix(&prefix)?)?;
            let p = e.path();
            (e.file_type().ok()?.is_file() && p.file_name() != just_written.file_name())
                .then_some((key, p))
        })
        .collect();
    older.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    for (_, p) in older.into_iter().skip(BACKUPS_KEPT.saturating_sub(1)) {
        let _ = std::fs::remove_file(p);
    }
}

/// Sort key of a backup name after `<db>.bak-v`: `<version>-<YYYYMMDDTHHMMSSZ>[-<n>]`
/// gives `(stamp, n)`. `None` for anything else.
fn backup_key(rest: &str) -> Option<(String, u32)> {
    let (version, rest) = rest.split_once('-')?;
    version.parse::<i64>().ok()?;
    let (stamp, n) = match rest.split_once('-') {
        Some((stamp, n)) => (stamp, n.parse().ok()?),
        None => (rest, 0),
    };
    let valid = stamp.len() == 16
        && stamp.as_bytes()[8] == b'T'
        && stamp.ends_with('Z')
        && stamp[..8]
            .bytes()
            .chain(stamp[9..15].bytes())
            .all(|b| b.is_ascii_digit());
    valid.then(|| (stamp.to_string(), n))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    const TABLES: &[&str] = &[
        "settings",
        "recent_files",
        "submissions",
        "problem_status",
        "problem_drafts",
        "hint_unlocks",
        "runs",
        "activity_days",
    ];

    fn backups(dir: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains(".bak-"))
            .collect()
    }

    #[test]
    fn migration_versions_are_contiguous_from_one() {
        for (i, m) in MIGRATIONS.iter().enumerate() {
            assert_eq!(m.version, i64::try_from(i + 1).unwrap());
        }
    }

    #[tokio::test]
    async fn migrates_from_empty_without_backup() {
        let dir = temp_dir("empty");
        let db = UserDb::open(&dir.join("user.db")).await.unwrap();
        assert_eq!(db.schema_version().await.unwrap(), MIGRATIONS.len() as i64);
        assert!(db.backup_path().is_none());
        for t in TABLES {
            let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {t}"))
                .fetch_one(&db.pool)
                .await
                .unwrap();
            assert_eq!(n, 0, "{t}");
        }
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(mode, "wal");
        db.close().await;
        // Reopening at the latest version migrates nothing and backs up nothing.
        let db = UserDb::open(&dir.join("user.db")).await.unwrap();
        assert!(db.backup_path().is_none());
        db.close().await;
        assert!(backups(&dir).is_empty());
    }

    #[tokio::test]
    async fn backs_up_before_migrating_and_refuses_newer_schemas() {
        let dir = temp_dir("backup");
        let path = dir.join("user.db");
        let v1 = &MIGRATIONS[..1];
        let db = UserDb::open_with(&path, v1).await.unwrap();
        db.set_setting("theme", &Value::from("dark")).await.unwrap();
        db.close().await;

        let v2 = [
            v1[0],
            Migration {
                version: 2,
                sql: "CREATE TABLE extra (x INTEGER); INSERT INTO extra VALUES (7);",
            },
        ];
        let db = UserDb::open_with(&path, &v2).await.unwrap();
        assert_eq!(db.schema_version().await.unwrap(), 2);
        assert_eq!(
            db.get_setting("theme").await.unwrap(),
            Some(Value::from("dark"))
        );
        let bak = db.backup_path().unwrap().to_path_buf();
        db.close().await;
        assert_eq!(backups(&dir), std::slice::from_ref(&bak));
        assert!(
            bak.to_string_lossy().contains("user.db.bak-v1-"),
            "{}",
            bak.display()
        );

        // The backup is the version-1 database with its data.
        let old = UserDb::open_with(&bak, v1).await.unwrap();
        assert_eq!(old.schema_version().await.unwrap(), 1);
        assert_eq!(
            old.get_setting("theme").await.unwrap(),
            Some(Value::from("dark"))
        );
        old.close().await;

        // A database newer than the binary is refused, never downgraded.
        assert!(matches!(
            UserDb::open_with(&path, v1).await,
            Err(StoreError::NewerSchema {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn backup_names_parse() {
        assert_eq!(
            backup_key("1-20261111T093000Z"),
            Some(("20261111T093000Z".into(), 0))
        );
        assert_eq!(
            backup_key("12-20261111T093000Z-2"),
            Some(("20261111T093000Z".into(), 2))
        );
        for bad in [
            "x-20261111T093000Z",
            "1-2026",
            "1-20261111T093000Z-x",
            "1-20261111X093000Z",
        ] {
            assert_eq!(backup_key(bad), None, "{bad}");
        }
    }

    #[tokio::test]
    async fn keeps_the_newest_backups() {
        let dir = temp_dir("prune");
        let path = dir.join("user.db");
        let fake = |n: &str| {
            let p = dir.join(n);
            std::fs::write(&p, b"old").unwrap();
            p
        };
        let old: Vec<PathBuf> = (1..=6)
            .map(|d| fake(&format!("user.db.bak-v1-2020010{d}T000000Z")))
            .collect();
        let unrelated = fake("user.db.bak-notes.txt");
        let other_db = fake("other.db.bak-v1-20200101T000000Z");
        let new = fake("user.db.bak-v1-20261121T000000Z");
        prune_backups(&path, &new);
        // The new one plus the 4 newest older ones survive.
        assert!(new.exists());
        assert!(old[2..].iter().all(|p| p.exists()));
        assert!(old[..2].iter().all(|p| !p.exists()));
        assert!(unrelated.exists() && other_db.exists());
    }

    #[tokio::test]
    async fn settings_round_trip() {
        let dir = temp_dir("settings");
        let db = UserDb::open(&dir.join("user.db")).await.unwrap();
        assert_eq!(db.get_setting("theme").await.unwrap(), None);
        db.set_setting("theme", &Value::from("light"))
            .await
            .unwrap();
        db.set_setting("theme", &Value::from("dark")).await.unwrap();
        assert_eq!(
            db.get_setting("theme").await.unwrap(),
            Some(Value::from("dark"))
        );
        db.close().await;
    }

    #[tokio::test]
    async fn recent_files_keep_the_newest_ten() {
        let dir = temp_dir("recent");
        let db = UserDb::open(&dir.join("user.db")).await.unwrap();
        for i in 0..12 {
            db.touch_recent(&format!("C:/designs/d{i}.kbarch"), &format!("D{i}"))
                .await
                .unwrap();
            // Distinct timestamps keep the order deterministic.
            std::thread::sleep(Duration::from_millis(2));
        }
        // Re-opening an old one moves it to the top and updates its name.
        db.touch_recent("C:/designs/d5.kbarch", "Renamed")
            .await
            .unwrap();
        let list = db.recent_files().await.unwrap();
        assert_eq!(list.len(), 10);
        assert_eq!(list[0].path, "C:/designs/d5.kbarch");
        assert_eq!(list[0].name, "Renamed");
        assert!(
            !list
                .iter()
                .any(|r| r.path.ends_with("d0.kbarch") || r.path.ends_with("d1.kbarch"))
        );
        assert!(db.is_recent("C:/designs/d11.kbarch").await.unwrap());
        db.forget_recent("C:/designs/d11.kbarch").await.unwrap();
        assert!(!db.is_recent("C:/designs/d11.kbarch").await.unwrap());
        db.close().await;
    }
}
