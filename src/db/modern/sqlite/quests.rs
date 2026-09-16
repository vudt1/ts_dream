//! Mission bookkeeping repository: `character_missions`,
//! `character_mission_flags`, `character_bit_flags`,
//! `character_completed_events`, `character_skills`.

use crate::db::modern::model::{MissionRow, SkillRow};
use crate::db::modern::traits::{QuestRepository, RepoResult};
use crate::db::pool::DbPool;
use sqlx::Row;

pub struct SqliteQuestRepository<'a> {
    pub pool: &'a DbPool,
}

impl QuestRepository for SqliteQuestRepository<'_> {
    async fn upsert_mission(&self, character_id: i64, mission: &MissionRow) -> RepoResult<()> {
        sqlx::query(
            "INSERT INTO character_missions (playerid, missionid, step, state, updatedat) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(playerid, missionid) DO UPDATE SET \
             step = excluded.step, state = excluded.state, updatedat = excluded.updatedat",
        )
        .bind(character_id)
        .bind(mission.mission_id)
        .bind(mission.step)
        .bind(mission.state)
        .bind(mission.updated_at)
        .execute(&self.pool.write)
        .await?;
        Ok(())
    }

    async fn list_missions(&self, character_id: i64) -> RepoResult<Vec<MissionRow>> {
        let rows = sqlx::query(
            "SELECT missionid AS mission_id, step, state, updatedat AS updated_at FROM character_missions \
             WHERE playerid = ?",
        )
        .bind(character_id)
        .fetch_all(&self.pool.read)
        .await?;
        Ok(rows
            .iter()
            .map(|r| MissionRow {
                mission_id: r.get("mission_id"),
                step: r.get("step"),
                state: r.get("state"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    async fn set_bit_flag(&self, character_id: i64, flag_index: u32, now: i64) -> RepoResult<()> {
        // INSERT OR IGNORE semantics in SQLite: once latched, a forever flag never rewrites.
        sqlx::query(
            "INSERT OR IGNORE INTO character_bit_flags (playerid, flagindex, set_at) \
             VALUES (?, ?, ?)",
        )
        .bind(character_id)
        .bind(flag_index)
        .bind(now)
        .execute(&self.pool.write)
        .await?;
        Ok(())
    }

    async fn has_bit_flag(&self, character_id: i64, flag_index: u32) -> RepoResult<bool> {
        let hits: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_bit_flags WHERE playerid = ? AND flagindex = ?",
        )
        .bind(character_id)
        .bind(flag_index)
        .fetch_one(&self.pool.read)
        .await?;
        Ok(hits > 0)
    }

    async fn mark_event_completed(
        &self,
        character_id: i64,
        event_id: i64,
        completed_at: i64,
    ) -> RepoResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO character_completed_events (playerid, eventid, completedat) \
             VALUES (?, ?, ?)",
        )
        .bind(character_id)
        .bind(event_id)
        .bind(completed_at)
        .execute(&self.pool.write)
        .await?;
        Ok(())
    }

    async fn has_completed_event(&self, character_id: i64, event_id: i64) -> RepoResult<bool> {
        let hits: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_completed_events \
             WHERE playerid = ? AND eventid = ?",
        )
        .bind(character_id)
        .bind(event_id)
        .fetch_one(&self.pool.read)
        .await?;
        Ok(hits > 0)
    }

    async fn replace_skills(
        &self,
        character_id: i64,
        skills: &[SkillRow],
        tx: &mut sqlx::SqliteConnection,
    ) -> RepoResult<()> {
        sqlx::query("DELETE FROM character_skills WHERE playerid = ?")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
        if skills.is_empty() {
            return Ok(());
        }
        // One multi-row INSERT so the whole set lands in a single statement.
        let placeholders = vec!["(?, ?, ?, ?, ?)"; skills.len()].join(", ");
        let sql = format!(
            "INSERT INTO character_skills (playerid, skillid, level, sp, saveflag) \
             VALUES {placeholders}"
        );
        let mut insert = sqlx::query(&sql);
        for s in skills {
            insert = insert
                .bind(character_id)
                .bind(s.skill_id)
                .bind(s.level)
                .bind(s.sp)
                .bind(s.save_flag);
        }
        insert.execute(&mut *tx).await?;
        Ok(())
    }
}
