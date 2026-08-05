//! DB write-through helpers (Chapter 5).
//!
//! Handlers mutate the in-memory `Session` and persist the same mutation to
//! MySQL here, mirroring the C# per-operation `UPDATE` calls (`PlayerUpdateDataId`,
//! `SkillSaveUpdateId`, `HomdoUpdateItem`…). All functions are best-effort:
//! a write failure only surfaces via tracing, never aborts the handler (C#
//! swallows the exception too). Live server passes `Some(&pool)`; golden
//! replay passes `None` and skips the DB entirely.

use crate::server::session::InventoryItem;
use sqlx::MySqlPool;

/// Safe column-identifier whitelist for `players` write-throughs. `Int` is a
/// reserved word and always backticked (0001_init.sql uses `` `Int` ``).
fn player_column(col: &str) -> Option<&'static str> {
    Some(match col {
        "Lv" => "Lv",
        "Hp" => "Hp",
        "HpMax" => "HpMax",
        "Sp" => "Sp",
        "SpMax" => "SpMax",
        "Point" => "Point",
        "SkillPoint" => "SkillPoint",
        "Int" => "`Int`",
        "Atk" => "Atk",
        "Def" => "Def",
        "Hpx" => "Hpx",
        "Spx" => "Spx",
        "Agi" => "Agi",
        "Int2" => "Int2",
        "Atk2" => "Atk2",
        "Def2" => "Def2",
        "Hpx2" => "Hpx2",
        "Spx2" => "Spx2",
        "Agi2" => "Agi2",
        "Texp" => "Texp",
        "Gold" => "Gold",
        "God" => "God",
        _ => return None,
    })
}

/// `UPDATE players SET <col> = ? WHERE player_id = ?` (C# `PlayerUpdateDataId`).
pub async fn update_player(
    pool: Option<&MySqlPool>,
    player_id: u32,
    col: &str,
    value: i64,
) {
    let Some(pool) = pool else { return };
    let Some(col) = player_column(col) else {
        tracing::warn!("skipped unknown players column write: {col}");
        return;
    };
    let q = format!("UPDATE players SET {col} = ? WHERE player_id = ?");
    if let Err(e) = sqlx::query(&q).bind(value).bind(i64::from(player_id)).execute(pool).await {
        tracing::warn!("update_player({col}) failed: {e}");
    }
}

/// `UPDATE skillsave SET IdSkill = ? WHERE player_id = ? AND ID = ?`
/// (C# `SkillSaveUpdateId`; rows 1..10 are seeded at character creation).
pub async fn update_skillsave(pool: Option<&MySqlPool>, player_id: u32, slot: u8, skill: u16) {
    let Some(pool) = pool else { return };
    let slot_id = i64::from(slot);
    if let Err(e) = sqlx::query("UPDATE skillsave SET IdSkill = ? WHERE player_id = ? AND ID = ?")
        .bind(i64::from(skill))
        .bind(i64::from(player_id))
        .bind(slot_id)
        .execute(pool)
        .await
    {
        tracing::warn!("update_skillsave(slot {slot}) failed: {e}");
    }
}

/// Backtick-quoted table name suffixes: homdo / trangbi / tientrang / tuideo /
/// luulang share the same item columns (verified against 0001_init.sql).
fn item_table(table: &str) -> Option<&'static str> {
    Some(match table {
        "homdo" => "homdo",
        "trangbi" => "trangbi",
        "tientrang" => "tientrang",
        "tuideo" => "tuideo",
        "luulang" => "luulang",
        _ => return None,
    })
}

/// Wipe every row of `table` for the player (C# items are rewritten wholesale
/// on login; mutations here use single-upserts instead, so this is only a
/// safety reset and is not called by ordinary flows).
#[allow(dead_code)]
pub async fn clear_items(pool: Option<&MySqlPool>, player_id: u32, table: &str) {
    let Some(pool) = pool else { return };
    let Some(table) = item_table(table) else { return };
    let q = format!("DELETE FROM {table} WHERE player_id = ?");
    if let Err(e) = sqlx::query(&q).bind(i64::from(player_id)).execute(pool).await {
        tracing::warn!("clear_items({table}) failed: {e}");
    }
}

/// Upsert one item row into `table` (INSERT … ON DUPLICATE KEY UPDATE). Mirrors
/// `HomdoUpdateSlot`. `item.id == 0` deletes the row (empty slot).
pub async fn upsert_item(
    pool: Option<&MySqlPool>,
    player_id: u32,
    table: &str,
    item: &InventoryItem,
) {
    let Some(pool) = pool else { return };
    let Some(table) = item_table(table) else { return };
    let player_id = i64::from(player_id);
    let res = if item.id == 0 {
        let q = format!("DELETE FROM {table} WHERE player_id = ? AND Slot = ?");
        sqlx::query(&q)
            .bind(player_id)
            .bind(i64::from(item.slot))
            .execute(pool)
            .await
    } else {
        let q = format!(
            "INSERT INTO {table} (\
             player_id, Slot, Id, `Count`, DoBen, Int1, Atk1, Def1, Hpx1, Spx1, Agi1, \
             Fai1, `Long`, GiatriLong, Khang, Thuoctinh, Loai, Texp) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE Id = VALUES(Id), `Count` = VALUES(`Count`), \
             DoBen = VALUES(DoBen), Int1 = VALUES(Int1), Atk1 = VALUES(Atk1), \
             Def1 = VALUES(Def1), Hpx1 = VALUES(Hpx1), Spx1 = VALUES(Spx1), \
             Agi1 = VALUES(Agi1), Fai1 = VALUES(Fai1), `Long` = VALUES(`Long`), \
             GiatriLong = VALUES(GiatriLong), Khang = VALUES(Khang), \
             Thuoctinh = VALUES(Thuoctinh), Loai = VALUES(Loai), Texp = VALUES(Texp)"
        );
        sqlx::query(&q)
            .bind(player_id)
            .bind(i64::from(item.slot))
            .bind(i64::from(item.id))
            .bind(i64::from(item.count))
            .bind(i64::from(item.doben))
            .bind(i64::from(item.int1))
            .bind(i64::from(item.atk1))
            .bind(i64::from(item.def1))
            .bind(i64::from(item.hpx1))
            .bind(i64::from(item.spx1))
            .bind(i64::from(item.agi1))
            .bind(i64::from(item.fai1))
            .bind(i64::from(item.long_val))
            .bind(i64::from(item.giatri_long))
            .bind(i64::from(item.khang))
            .bind(i64::from(item.thuoctinh))
            .bind(i64::from(item.loai))
            .bind(i64::from(item.texp))
            .execute(pool)
            .await
    };
    if let Err(e) = res {
        tracing::warn!("upsert_item({table}, slot {}) failed: {e}", item.slot);
    }
}