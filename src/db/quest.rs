//! `quest` table repository (Chapter 5 §5.4 — the 9th gameplay table).
//!
//! C# stored per-player quest progress in the member file; the shared MySQL
//! schema stores rows keyed `(player_id, QuestId)`. Every statement is scoped
//! by `player_id` — the C# `DELETE FROM Quest WHERE MapId = …` patterns
//! (FTalk.cs:789-955) must each carry `AND player_id = ?`.

use sqlx::{MySql, MySqlPool, Transaction};

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

/// Read the `Step` for one NPC quest (`MapId`,`NpcId`), scoped by `player_id`.
pub async fn step_for_npc(
    pool: &MySqlPool,
    player_id: i64,
    map_id: i64,
    npc_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64,)>(
        "SELECT Step FROM quest WHERE player_id = ? AND MapId = ? AND NpcId = ? LIMIT 1",
    )
    .bind(player_id)
    .bind(map_id)
    .bind(npc_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0).unwrap_or(0))
}

/// Replace the player's quest step rows (C# rebuild pattern) or upsert a single
/// row. Both carry `player_id`.
pub async fn upsert_npc_step(
    pool: &MySqlPool,
    player_id: i64,
    map_id: i64,
    npc_id: i64,
    step: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO quest (player_id, QuestId, MapId, NpcId, WarpId, Step) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE Step = VALUES(Step), WarpId = VALUES(WarpId)",
    )
    .bind(player_id)
    .bind(npc_id)
    .bind(map_id)
    .bind(npc_id)
    .bind(0i64)
    .bind(step)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete one NPC quest step (scoped); C# also deletes `MPC/WARP`-keyed rows.
pub async fn delete_npc(
    pool: &MySqlPool,
    player_id: i64,
    map_id: i64,
    npc_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM quest WHERE player_id = ? AND MapId = ? AND NpcId = ?")
        .bind(player_id)
        .bind(map_id)
        .bind(npc_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete all quest rows for a player (C# quest resets).
pub async fn delete_all(pool: &MySqlPool, player_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM quest WHERE player_id = ?")
        .bind(player_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Scoped reset used inside the character-deletion / login flows (tx variant).
pub async fn delete_all_tx(
    tx: &mut Transaction<'_, MySql>,
    player_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM quest WHERE player_id = ?")
        .bind(player_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}