//! `quest` table repository (Chapter 5 §5.4 — the 9th gameplay table).
//!
//! C# stored per-player quest progress in the member file; the shared MySQL
//! schema stores rows keyed `(player_id, QuestId)`. Every statement is scoped
//! by `player_id`. Only the login read is exercised today; the H6 `DELETE`
//! patterns (FTalk.cs:789-955) must each carry `AND player_id = ?` when the
//! compiled H6 table lands.

use sqlx::MySqlPool;

/// One scoped quest-step row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuestRow {
    pub quest_id: i64,
    pub map_id: i64,
    pub npc_id: i64,
    pub warp_id: i64,
    pub step: i64,
}

/// Load every quest row for `player_id` (scoped read).
pub async fn list(pool: &MySqlPool, player_id: i64) -> Result<Vec<QuestRow>, sqlx::Error> {
    sqlx::query_as::<_, QuestRow>(
        "SELECT QuestId AS quest_id, MapId AS map_id, NpcId AS npc_id, \
         WarpId AS warp_id, Step AS step FROM quest WHERE player_id = ?",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
}