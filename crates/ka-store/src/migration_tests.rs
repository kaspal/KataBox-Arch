//! Up-migration checks on a real-shaped database (the KataBox pattern). Every new
//! migration adds one entry to `seeds` (if it adds tables worth seeding) and is covered by
//! `every_version_upgrades_without_losing_rows`: a database at each older version, full
//! of realistic rows, is upgraded and every table that existed before keeps its rows,
//! byte for byte, in the columns it had.

use std::collections::BTreeMap;
use std::path::Path;

use sqlx::Row;

use crate::testutil::temp_dir;
use crate::user::{MIGRATIONS, UserDb};

/// Rows written for each schema version (applied in order up to the DB's version).
/// Times are fixed so fingerprints are stable.
fn seeds(version: i64) -> Vec<String> {
    match version {
        1 => vec![
            "INSERT INTO settings VALUES ('theme', '\"dark\"', 1700000000000),
                 ('ui_lang', '\"es\"', 1700000000001),
                 ('sandbox.layout', '{\"main\":[20,60,20]}', 1700000000002),
                 ('window.bounds', '{\"x\":10,\"y\":20,\"width\":1280,\"height\":800,\"scale\":1.5,\"maximized\":false}', 1700000000003)"
                .into(),
            "INSERT INTO recent_files VALUES
                 ('C:\\Users\\me\\Diseños\\acortador.kbarch', 'Acortador de URLs ñ', 1700000001000),
                 ('/home/me/designs/feed.kbarch', 'News feed 🎯', 1700000002000)"
                .into(),
        ],
        2 => vec![
            "INSERT INTO submissions (problem_id, created_at, design, verdict, score, correctness,
                 cost_score, latency_score, cost_usd, p95_ms, results) VALUES
                 ('url-shortener', 1700000003000, '{\"nodes\":[]}', 'slo_breach', 52, 52, 0, 0,
                  1850.5, 96.25, '{\"tests\":[{\"id\":\"viral\"}]}'),
                 ('url-shortener', 1700000004000, '{\"nodes\":[1]}', 'accepted', 91, 70, 14, 7,
                  1320.0, 41.5, '{\"tests\":[]}')"
                .into(),
            "INSERT INTO problem_status VALUES
                 ('url-shortener', 'solved', 91, 2, 1700000004000, 1700000004000),
                 ('pastebin', 'attempted', 30, NULL, NULL, 1700000005000)"
                .into(),
            "INSERT INTO problem_drafts VALUES ('pastebin', '{\"nodes\":[\"ñ\"]}', 1700000006000)".into(),
            "INSERT INTO hint_unlocks VALUES ('pastebin', 1, 1700000007000), ('pastebin', 2, 1700000008000)".into(),
            "INSERT INTO runs VALUES ('pastebin', 1700000009000), ('pastebin', 1700000010000)".into(),
            "INSERT INTO activity_days VALUES ('2026-11-20'), ('2026-11-21')".into(),
        ],
        3 => vec![
            "INSERT INTO reads VALUES ('guide', 'cache', 1700000011000), ('guide', 'lb', 1700000012000)".into(),
        ],
        5 => vec![
            "INSERT INTO quiz_attempts VALUES ('guide:cache:1', 0, '2026-11-20', 1700000015000),
                 ('guide:cache:1', 1, '2026-11-21', 1700000016000)"
                .into(),
            "INSERT INTO review_cards VALUES ('guide:cache:1', 1, 0, '2026-11-24', '2026-11-21')".into(),
        ],
        4 => vec![
            "INSERT INTO estimates VALUES ('url-shortener', 'redirect-qps', 12000.5, 1, 1700000013000),
                 ('pastebin', 'storage-year', 0.25, 0, 1700000014000)"
                .into(),
        ],
        _ => vec![],
    }
}

/// A user.db at `version` (a prefix of [`MIGRATIONS`]) with realistic rows in every
/// table that exists at that version.
pub(crate) async fn real_shaped_db(path: &Path, version: usize) {
    let db = UserDb::open_with(path, &MIGRATIONS[..version])
        .await
        .unwrap();
    for v in 1..=version as i64 {
        for sql in seeds(v) {
            sqlx::raw_sql(&sql)
                .execute(&db.pool)
                .await
                .unwrap_or_else(|e| panic!("seed v{v}: {e}\n{sql}"));
        }
    }
    db.close().await;
}

/// Table name -> (its columns, row count, a fingerprint of every row in those columns).
type Fingerprints = BTreeMap<String, (Vec<String>, i64, String)>;

async fn fingerprints(db: &UserDb, only: Option<&Fingerprints>) -> Fingerprints {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table'
         AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let mut out = Fingerprints::new();
    for t in tables {
        let cols: Vec<String> = match only {
            Some(before) => match before.get(&t) {
                Some((cols, _, _)) => cols.clone(),
                None => continue,
            },
            None => sqlx::query(&format!("PRAGMA table_info({t})"))
                .fetch_all(&db.pool)
                .await
                .unwrap()
                .iter()
                .map(|r| r.get::<String, _>("name"))
                .collect(),
        };
        let quoted = cols
            .iter()
            .map(|c| format!("quote({c})"))
            .collect::<Vec<_>>()
            .join(" || '|' || ");
        let order = cols.join(", ");
        let rows: Vec<String> =
            sqlx::query_scalar(&format!("SELECT {quoted} FROM {t} ORDER BY {order}"))
                .fetch_all(&db.pool)
                .await
                .unwrap();
        out.insert(t, (cols, rows.len() as i64, rows.join("\n")));
    }
    out
}

/// Seed a DB at `from`, upgrade it to the latest version and check that every old
/// table kept every row (in its old columns) and that a backup was written.
pub(crate) async fn assert_upgrade_keeps(name: &str, from: usize) {
    let dir = temp_dir(name);
    let path = dir.join("user.db");
    real_shaped_db(&path, from).await;
    let old = UserDb::open_with(&path, &MIGRATIONS[..from]).await.unwrap();
    let before = fingerprints(&old, None).await;
    old.close().await;
    assert!(before.values().any(|(_, n, _)| *n > 0), "seeded rows");

    let db = UserDb::open(&path).await.unwrap();
    assert_eq!(db.schema_version().await.unwrap(), MIGRATIONS.len() as i64);
    if from < MIGRATIONS.len() {
        let bak = db.backup_path().expect("backup before migrating");
        assert!(bak.exists());
    }
    let after = fingerprints(&db, Some(&before)).await;
    db.close().await;
    for (t, (_, n, rows)) in &before {
        let (_, n2, rows2) = after.get(t).unwrap_or_else(|| panic!("table {t} dropped"));
        assert_eq!(n, n2, "row count of {t} from v{from}");
        assert_eq!(rows, rows2, "rows of {t} from v{from}");
    }
}

#[tokio::test]
async fn every_version_upgrades_without_losing_rows() {
    for from in 1..=MIGRATIONS.len() {
        assert_upgrade_keeps(&format!("upgrade-v{from}"), from).await;
    }
}

#[tokio::test]
async fn the_latest_schema_takes_every_seed() {
    // Seeds written for older versions still insert at the latest one (new columns
    // must be nullable or defaulted, as the migration rules require).
    let dir = temp_dir("seed-latest");
    real_shaped_db(&dir.join("user.db"), MIGRATIONS.len()).await;
}
