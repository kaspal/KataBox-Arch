//! Installed problem packs in `user.db` (v6, M6.4, ADR 0011). The pack files themselves
//! live in `<data>/packs/<id>/`; this table is what was installed, and when.

use serde::Serialize;

use crate::user::UserDb;
use crate::{Result, now_ms};

/// A pack as recorded: removed ones keep their row (`removed_at` set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PackRow {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub licence: String,
    pub sha256: String,
    pub installed_at: i64,
    pub removed_at: Option<i64>,
}

type Tuple = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    Option<i64>,
);

fn from_tuple(t: Tuple) -> PackRow {
    PackRow {
        id: t.0,
        name: t.1,
        author: t.2,
        version: t.3,
        licence: t.4,
        sha256: t.5,
        installed_at: t.6,
        removed_at: t.7,
    }
}

const COLUMNS: &str = "id, name, author, version, licence, sha256, installed_at, removed_at";

impl UserDb {
    /// Every pack ever installed, by id.
    pub async fn packs(&self) -> Result<Vec<PackRow>> {
        let rows: Vec<Tuple> = sqlx::query_as(&format!("SELECT {COLUMNS} FROM packs ORDER BY id"))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(from_tuple).collect())
    }

    pub async fn pack(&self, id: &str) -> Result<Option<PackRow>> {
        let row: Option<Tuple> =
            sqlx::query_as(&format!("SELECT {COLUMNS} FROM packs WHERE id = ?"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(from_tuple))
    }

    /// Record an install (or an upgrade, or a reinstall after removal).
    pub async fn record_pack(
        &self,
        id: &str,
        name: &str,
        author: &str,
        version: &str,
        licence: &str,
        sha256: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO packs (id, name, author, version, licence, sha256, installed_at, removed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, NULL)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, author = excluded.author,
               version = excluded.version, licence = excluded.licence, sha256 = excluded.sha256,
               installed_at = excluded.installed_at, removed_at = NULL",
        )
        .bind(id)
        .bind(name)
        .bind(author)
        .bind(version)
        .bind(licence)
        .bind(sha256)
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark a pack removed; its row (and the user's history with its problems) stays.
    pub async fn mark_pack_removed(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE packs SET removed_at = ? WHERE id = ? AND removed_at IS NULL")
            .bind(now_ms())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::temp_dir;
    use crate::user::UserDb;

    #[tokio::test]
    async fn packs_install_upgrade_and_removal_keep_one_row() {
        let db = UserDb::open(&temp_dir("packs").join("user.db"))
            .await
            .unwrap();
        assert!(db.packs().await.unwrap().is_empty());
        db.record_pack("drills", "Drills", "Ana", "1.0.0", "MIT", "aa")
            .await
            .unwrap();
        db.record_pack("drills", "Drills", "Ana", "1.1.0", "MIT", "bb")
            .await
            .unwrap();
        let p = db.pack("drills").await.unwrap().unwrap();
        assert_eq!(
            (p.version.as_str(), p.sha256.as_str(), p.removed_at),
            ("1.1.0", "bb", None)
        );
        db.mark_pack_removed("drills").await.unwrap();
        let rows = db.packs().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].removed_at.is_some());
    }
}
