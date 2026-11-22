//! Practice records in `user.db` (M5): estimation answers (v4); flaw drills and spaced
//! review join in later migrations.

use serde::Serialize;

use crate::user::UserDb;
use crate::{Result, now_ms};

/// One drill's record: how many checks, and whether any was within its band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EstimateStatus {
    pub problem_id: String,
    pub drill_id: String,
    pub tries: i64,
    pub done: bool,
}

impl UserDb {
    pub async fn record_estimate(
        &self,
        problem_id: &str,
        drill_id: &str,
        answer: f64,
        ok: bool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO estimates (problem_id, drill_id, answer, ok, at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(problem_id)
        .bind(drill_id)
        .bind(answer)
        .bind(i64::from(ok))
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Every drill with at least one check, by problem then drill.
    pub async fn estimate_statuses(&self) -> Result<Vec<EstimateStatus>> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT problem_id, drill_id, count(*), max(ok) FROM estimates
             GROUP BY problem_id, drill_id ORDER BY problem_id, drill_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(problem_id, drill_id, tries, ok)| EstimateStatus {
                problem_id,
                drill_id,
                tries,
                done: ok == 1,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::temp_dir;
    use crate::user::UserDb;

    #[tokio::test]
    async fn estimates_count_tries_and_remember_a_hit() {
        let db = UserDb::open(&temp_dir("estimates").join("user.db"))
            .await
            .unwrap();
        db.record_estimate("url-shortener", "qps", 1.0, false)
            .await
            .unwrap();
        db.record_estimate("url-shortener", "qps", 12_000.0, true)
            .await
            .unwrap();
        db.record_estimate("url-shortener", "qps", 1.0, false)
            .await
            .unwrap();
        db.record_estimate("pastebin", "storage", 3.0, false)
            .await
            .unwrap();
        let s = db.estimate_statuses().await.unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            (s[0].problem_id.as_str(), s[0].done, s[0].tries),
            ("pastebin", false, 1)
        );
        assert_eq!(
            (s[1].drill_id.as_str(), s[1].done, s[1].tries),
            ("qps", true, 3)
        );
        db.close().await;
    }
}
