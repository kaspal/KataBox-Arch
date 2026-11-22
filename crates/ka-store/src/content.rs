//! `content.db`: the read-only catalogue the app ships (problems now; Guide, patterns,
//! samples, concept cards, decision guides and the glossary from M3). `ka-pack build` writes
//! it from `content/`; the app only reads it. Schema 1, recorded in `PRAGMA user_version`;
//! a newer file is refused. Structured parts are JSON strings, so this crate doesn't depend on
//! the content types (`ka-core::problem`).

use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::{Connection, Row, SqliteConnection};

use crate::{Result, StoreError};

pub const CONTENT_SCHEMA: i64 = 1;

const SCHEMA_SQL: &str = r#"
CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE problems (
  id           TEXT PRIMARY KEY,
  number       INTEGER NOT NULL,
  "set"        INTEGER NOT NULL,
  difficulty   TEXT NOT NULL,
  title_en     TEXT NOT NULL,
  title_es     TEXT NOT NULL,
  teaches_en   TEXT NOT NULL,
  teaches_es   TEXT NOT NULL,
  tags         TEXT NOT NULL,
  nfr          TEXT NOT NULL,
  def          TEXT NOT NULL,
  statement_en TEXT NOT NULL,
  statement_es TEXT NOT NULL,
  hints_en     TEXT NOT NULL,
  hints_es     TEXT NOT NULL,
  editorial_en TEXT NOT NULL,
  editorial_es TEXT NOT NULL,
  reference    TEXT NOT NULL,
  ref_cost     REAL NOT NULL,
  ref_p95_ms   REAL NOT NULL
);
CREATE TABLE guide (
  kind  TEXT PRIMARY KEY,
  entry TEXT NOT NULL,
  md_en TEXT NOT NULL,
  md_es TEXT NOT NULL
);
CREATE TABLE patterns (
  id       TEXT PRIMARY KEY,
  entry    TEXT NOT NULL,
  md_en    TEXT NOT NULL,
  md_es    TEXT NOT NULL,
  template TEXT NOT NULL
);
CREATE TABLE samples (
  id        TEXT PRIMARY KEY,
  def       TEXT NOT NULL,
  snapshots TEXT NOT NULL
);
CREATE TABLE concepts (
  kind TEXT PRIMARY KEY,
  def  TEXT NOT NULL
);
CREATE TABLE decide (
  id  TEXT PRIMARY KEY,
  def TEXT NOT NULL
);
CREATE TABLE glossary (
  id    TEXT PRIMARY KEY,
  entry TEXT NOT NULL
);
"#;

/// One problem as stored. `def` is the full `ProblemDef` as JSON (read in Rust only: hidden
/// tests never reach the UI from here); `nfr` and `tags` are JSON too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProblemRow {
    pub id: String,
    pub number: i64,
    pub set: i64,
    pub difficulty: String,
    pub title_en: String,
    pub title_es: String,
    pub teaches_en: String,
    pub teaches_es: String,
    pub tags: Vec<String>,
    pub nfr: String,
    pub def: String,
    pub statement_en: String,
    pub statement_es: String,
    /// Three tiers each.
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    pub editorial_en: String,
    pub editorial_es: String,
    /// The reference design (`.kbarch` JSON).
    pub reference: String,
    /// The reference design's monthly cost on the problem's first test (for the cost score).
    pub ref_cost: f64,
    /// The reference's p95 on the first test (ms), for the cost vs speed chart.
    pub ref_p95_ms: f64,
}

/// Everything `ka-pack build` writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContentData {
    pub meta: Vec<(String, String)>,
    pub problems: Vec<ProblemRow>,
}

/// Write a fresh `content.db` at `path` (replacing any file there).
pub async fn write_content(path: &Path, data: &ContentData) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Delete);
    let mut conn = SqliteConnection::connect_with(&opts).await?;
    let mut tx = conn.begin().await?;
    sqlx::raw_sql(SCHEMA_SQL).execute(&mut *tx).await?;
    for (k, v) in &data.meta {
        sqlx::query("INSERT INTO meta (key, value) VALUES (?, ?)")
            .bind(k)
            .bind(v)
            .execute(&mut *tx)
            .await?;
    }
    for p in &data.problems {
        sqlx::query(
            r#"INSERT INTO problems (id, number, "set", difficulty, title_en, title_es, teaches_en,
               teaches_es, tags, nfr, def, statement_en, statement_es, hints_en, hints_es,
               editorial_en, editorial_es, reference, ref_cost, ref_p95_ms)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&p.id)
        .bind(p.number)
        .bind(p.set)
        .bind(&p.difficulty)
        .bind(&p.title_en)
        .bind(&p.title_es)
        .bind(&p.teaches_en)
        .bind(&p.teaches_es)
        .bind(serde_json::to_string(&p.tags)?)
        .bind(&p.nfr)
        .bind(&p.def)
        .bind(&p.statement_en)
        .bind(&p.statement_es)
        .bind(serde_json::to_string(&p.hints_en)?)
        .bind(serde_json::to_string(&p.hints_es)?)
        .bind(&p.editorial_en)
        .bind(&p.editorial_es)
        .bind(&p.reference)
        .bind(p.ref_cost)
        .bind(p.ref_p95_ms)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::raw_sql(&format!("PRAGMA user_version = {CONTENT_SCHEMA}"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    sqlx::raw_sql("VACUUM").execute(&mut conn).await?;
    conn.close().await?;
    Ok(())
}

/// The shipped catalogue, opened read-only.
#[derive(Debug, Clone)]
pub struct ContentDb {
    pool: SqlitePool,
}

const PROBLEM_COLUMNS: &str = r#"id, number, "set", difficulty, title_en, title_es, teaches_en,
    teaches_es, tags, nfr, def, statement_en, statement_es, hints_en, hints_es, editorial_en,
    editorial_es, reference, ref_cost, ref_p95_ms"#;

fn problem_row(r: &sqlx::sqlite::SqliteRow) -> Result<ProblemRow> {
    let json_list = |col: &str| -> Result<Vec<String>> {
        let s: String = r.try_get(col)?;
        Ok(serde_json::from_str(&s)?)
    };
    Ok(ProblemRow {
        id: r.try_get("id")?,
        number: r.try_get("number")?,
        set: r.try_get("set")?,
        difficulty: r.try_get("difficulty")?,
        title_en: r.try_get("title_en")?,
        title_es: r.try_get("title_es")?,
        teaches_en: r.try_get("teaches_en")?,
        teaches_es: r.try_get("teaches_es")?,
        tags: json_list("tags")?,
        nfr: r.try_get("nfr")?,
        def: r.try_get("def")?,
        statement_en: r.try_get("statement_en")?,
        statement_es: r.try_get("statement_es")?,
        hints_en: json_list("hints_en")?,
        hints_es: json_list("hints_es")?,
        editorial_en: r.try_get("editorial_en")?,
        editorial_es: r.try_get("editorial_es")?,
        reference: r.try_get("reference")?,
        ref_cost: r.try_get("ref_cost")?,
        ref_p95_ms: r.try_get("ref_p95_ms")?,
    })
}

impl ContentDb {
    pub async fn open(path: &Path) -> Result<ContentDb> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .immutable(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await?;
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await?;
        if version > CONTENT_SCHEMA {
            pool.close().await;
            return Err(StoreError::NewerSchema {
                found: version,
                supported: CONTENT_SCHEMA,
            });
        }
        if version != CONTENT_SCHEMA {
            pool.close().await;
            return Err(StoreError::Corrupt(format!(
                "content.db schema {version}, expected {CONTENT_SCHEMA}"
            )));
        }
        Ok(ContentDb { pool })
    }

    pub async fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(sqlx::query_scalar("SELECT value FROM meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// Every problem, in catalogue order (set, then number).
    pub async fn problems(&self) -> Result<Vec<ProblemRow>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {PROBLEM_COLUMNS} FROM problems ORDER BY "set", number"#
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(problem_row).collect()
    }

    pub async fn problem(&self, id: &str) -> Result<Option<ProblemRow>> {
        let row = sqlx::query(&format!(
            "SELECT {PROBLEM_COLUMNS} FROM problems WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(problem_row).transpose()
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    fn problem(id: &str, number: i64) -> ProblemRow {
        ProblemRow {
            id: id.into(),
            number,
            set: 1,
            difficulty: "easy".into(),
            title_en: format!("Title {number}"),
            title_es: format!("Título {number}"),
            teaches_en: "Caching".into(),
            teaches_es: "Caché".into(),
            tags: vec!["caching".into()],
            nfr: r#"{"peak_rps":1000}"#.into(),
            def: r#"{"id":"x"}"#.into(),
            statement_en: "# Statement\nñ".into(),
            statement_es: "# Enunciado".into(),
            hints_en: vec!["a".into(), "b".into(), "c".into()],
            hints_es: vec!["a".into(), "b".into(), "c".into()],
            editorial_en: "Editorial".into(),
            editorial_es: "Editorial".into(),
            reference: "{}".into(),
            ref_cost: 1234.5,
            ref_p95_ms: 42.5,
        }
    }

    #[tokio::test]
    async fn content_round_trips_in_catalogue_order() {
        let path = temp_dir("content").join("content.db");
        let data = ContentData {
            meta: vec![("hash".into(), "abc".into())],
            problems: vec![problem("b", 2), problem("a", 1)],
        };
        write_content(&path, &data).await.unwrap();
        // Rewriting replaces the file.
        write_content(&path, &data).await.unwrap();
        let db = ContentDb::open(&path).await.unwrap();
        assert_eq!(db.meta("hash").await.unwrap().as_deref(), Some("abc"));
        let all = db.problems().await.unwrap();
        assert_eq!(
            all.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(db.problem("b").await.unwrap(), Some(problem("b", 2)));
        assert_eq!(db.problem("zzz").await.unwrap(), None);
        db.close().await;
    }

    #[tokio::test]
    async fn a_newer_schema_is_refused() {
        let path = temp_dir("content-newer").join("content.db");
        write_content(&path, &ContentData::default()).await.unwrap();
        {
            let opts = SqliteConnectOptions::new().filename(&path);
            let mut conn = SqliteConnection::connect_with(&opts).await.unwrap();
            sqlx::raw_sql("PRAGMA user_version = 2")
                .execute(&mut conn)
                .await
                .unwrap();
            conn.close().await.unwrap();
        }
        assert!(matches!(
            ContentDb::open(&path).await.unwrap_err(),
            StoreError::NewerSchema { found: 2, .. }
        ));
    }
}
