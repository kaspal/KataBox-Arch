//! The "same SLO, half the cost" challenge in `user.db` (migration 0007, M8.5): per problem,
//! the cheapest design that passed every test within the challenge budget, when it was first
//! solved, and the tries.

use serde::Serialize;
use sqlx::Row;

use crate::user::UserDb;
use crate::{Result, now_ms};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChallengeResult {
    pub problem_id: String,
    pub best_cost: Option<f64>,
    pub solved_at: Option<i64>,
    pub attempts: i64,
}

impl UserDb {
    /// One challenge try: `passed_cost` is its cost when every test passed within the
    /// challenge budget (None: it didn't).
    pub async fn record_challenge(&self, problem_id: &str, passed_cost: Option<f64>) -> Result<()> {
        let now = now_ms();
        sqlx::query(
            "INSERT INTO challenge_results (problem_id, best_cost, solved_at, attempts)
             VALUES (?1, ?2, CASE WHEN ?2 IS NULL THEN NULL ELSE ?3 END, 1)
             ON CONFLICT (problem_id) DO UPDATE SET
               attempts = attempts + 1,
               best_cost = CASE
                 WHEN ?2 IS NULL THEN best_cost
                 WHEN best_cost IS NULL OR ?2 < best_cost THEN ?2
                 ELSE best_cost END,
               solved_at = COALESCE(solved_at, CASE WHEN ?2 IS NULL THEN NULL ELSE ?3 END)",
        )
        .bind(problem_id)
        .bind(passed_cost)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn challenge_results(&self) -> Result<Vec<ChallengeResult>> {
        let rows = sqlx::query(
            "SELECT problem_id, best_cost, solved_at, attempts FROM challenge_results ORDER BY problem_id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(ChallengeResult {
                    problem_id: r.try_get("problem_id")?,
                    best_cost: r.try_get("best_cost")?,
                    solved_at: r.try_get("solved_at")?,
                    attempts: r.try_get("attempts")?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::temp_dir;
    use crate::user::UserDb;

    #[tokio::test]
    async fn a_challenge_keeps_the_cheapest_pass_and_counts_tries() {
        let dir = temp_dir("challenges");
        let db = UserDb::open(&dir.join("user.db")).await.unwrap();
        db.record_challenge("p", None).await.unwrap();
        db.record_challenge("p", Some(500.0)).await.unwrap();
        db.record_challenge("p", Some(700.0)).await.unwrap();
        db.record_challenge("p", Some(450.0)).await.unwrap();
        let r = &db.challenge_results().await.unwrap()[0];
        assert_eq!(r.attempts, 4);
        assert_eq!(r.best_cost, Some(450.0));
        assert!(r.solved_at.is_some());
        db.record_challenge("q", None).await.unwrap();
        let q = &db.challenge_results().await.unwrap()[1];
        assert_eq!((q.best_cost, q.solved_at, q.attempts), (None, None, 1));
    }
}
