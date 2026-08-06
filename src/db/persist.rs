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
pub async fn update_player(pool: Option<&MySqlPool>, player_id: u32, col: &str, value: i64) {
    let Some(pool) = pool else { return };
    let Some(col) = player_column(col) else {
        tracing::warn!("skipped unknown players column write: {col}");
        return;
    };
    let q = format!("UPDATE players SET {col} = ? WHERE player_id = ?");
    if let Err(e) = sqlx::query(&q)
        .bind(value)
        .bind(i64::from(player_id))
        .execute(pool)
        .await
    {
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
    let Some(table) = item_table(table) else {
        return;
    };
    let q = format!("DELETE FROM {table} WHERE player_id = ?");
    if let Err(e) = sqlx::query(&q)
        .bind(i64::from(player_id))
        .execute(pool)
        .await
    {
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
    let Some(table) = item_table(table) else {
        return;
    };
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

/// Upserts a player skill entry into MySQL `skill` table.
pub async fn upsert_skill(
    pool: Option<&MySqlPool>,
    player_id: u32,
    skill_id: u16,
    lv: u8,
    sp: u8,
    save: u8,
) {
    let Some(pool) = pool else { return };
    let q = "INSERT INTO skill (player_id, Id, Lv, Sp, Save) VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE Lv = VALUES(Lv), Sp = VALUES(Sp), Save = VALUES(Save)";
    if let Err(e) = sqlx::query(q)
        .bind(i64::from(player_id))
        .bind(i64::from(skill_id))
        .bind(i64::from(lv))
        .bind(i64::from(sp))
        .bind(i64::from(save))
        .execute(pool)
        .await
    {
        tracing::warn!("upsert_skill(skill {skill_id}) failed: {e}");
    }
}

/// Scope-deletes active player skills on rebirth while retaining special reborn/passive skills.
pub async fn delete_reborn_skills(pool: Option<&MySqlPool>, player_id: u32) {
    let Some(pool) = pool else { return };
    let q = "DELETE FROM skill WHERE player_id = ? AND Id >= 10001 AND Id <= 13033 \
             AND Id NOT IN (10016, 10017, 10018, 10019, 11016, 11017, 11018, 11019, 12016, 12017, 12018, 12019, 13015, 13016, 13017, 13018)";
    if let Err(e) = sqlx::query(q)
        .bind(i64::from(player_id))
        .execute(pool)
        .await
    {
        tracing::warn!("delete_reborn_skills(player {player_id}) failed: {e}");
    }
}

/// Scoped login-time skill purge (see [`delete_system_skills`]). The `player_id`
/// predicate is mandatory in the shared schema (§5.4 note 2) — a verbatim C#
/// port (`DELETE FROM Skill WHERE Id >= 0 AND Id <= 9`) would wipe every player.
pub const DELETE_SYSTEM_SKILLS_SQL: &str =
    "DELETE FROM skill WHERE player_id = ? AND Id >= 0 AND Id <= 9";

/// Purges the system/basic `Skill` rows (`Id 0..9`) at the tail of every login,
/// mirroring C# `Logined1` (`Client.cs:8193` — "DELETE FROM Skill WHERE
/// Id >= 0 AND Id <= 9", §5.4 note 2 / §5.6). These rows are transient UI/system
/// skills that get re-derived on the next read; in the shared MySQL schema the
/// DELETE must be scoped by `player_id` (per-file cleanup became per-row).
///
/// Ordering matches C#: the DELETE runs *after* the Logined1 stats frame is
/// built (which still shows the pre-purge skill list), so the handler calls
/// this at the very end of the successful login path.
///
/// No-op when `pool` is `None` (golden replay never touches the DB).
pub async fn delete_system_skills(pool: Option<&MySqlPool>, player_id: u32) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query(DELETE_SYSTEM_SKILLS_SQL)
        .bind(i64::from(player_id))
        .execute(pool)
        .await
    {
        tracing::warn!("delete_system_skills(player {player_id}) failed: {e}");
    }
}

/// Upserts a pet entry in MySQL `pet` table.
pub async fn upsert_pet(
    pool: Option<&MySqlPool>,
    player_id: u32,
    pet: &crate::server::session::PetState,
) {
    let Some(pool) = pool else { return };
    let name_str = String::from_utf8_lossy(&pet.name);
    let q = "INSERT INTO pet (player_id, Stt, Id, Name, Lv, Thuoctinh, Reborn, Hp, HpMax, Sp, SpMax, \
             `Int`, Atk, Def, Hpx, Spx, Agi, Fai, Texp, Int2, Atk2, Def2, Hpx2, Spx2, Agi2, Thd, \
             SkillPoint, Quest, Idskill1, LvSkill1, IdSkill2, LvSkill2, IdSkill3, LvSkill3, IdSkill4, LvSkill4) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, 0, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE Id = VALUES(Id), Name = VALUES(Name), Lv = VALUES(Lv), \
             Thuoctinh = VALUES(Thuoctinh), Reborn = VALUES(Reborn), Hp = VALUES(Hp), HpMax = VALUES(HpMax), \
             Sp = VALUES(Sp), SpMax = VALUES(SpMax), `Int` = VALUES(`Int`), Atk = VALUES(Atk), Def = VALUES(Def), \
             Hpx = VALUES(Hpx), Spx = VALUES(Spx), Agi = VALUES(Agi), Fai = VALUES(Fai), Texp = VALUES(Texp), \
             SkillPoint = VALUES(SkillPoint), Idskill1 = VALUES(Idskill1), LvSkill1 = VALUES(LvSkill1), \
             IdSkill2 = VALUES(IdSkill2), LvSkill2 = VALUES(LvSkill2), IdSkill3 = VALUES(IdSkill3), \
             LvSkill3 = VALUES(LvSkill3), IdSkill4 = VALUES(IdSkill4), LvSkill4 = VALUES(LvSkill4)";
    if let Err(e) = sqlx::query(q)
        .bind(i64::from(player_id))
        .bind(i64::from(pet.stt))
        .bind(i64::from(pet.id))
        .bind(name_str.as_ref())
        .bind(i64::from(pet.level))
        .bind(i64::from(pet.thuoctinh))
        .bind(i64::from(pet.reborn))
        .bind(i64::from(pet.hp))
        .bind(i64::from(pet.hp_max))
        .bind(i64::from(pet.sp))
        .bind(i64::from(pet.sp_max))
        .bind(i64::from(pet.int1))
        .bind(i64::from(pet.atk))
        .bind(i64::from(pet.def))
        .bind(i64::from(pet.hpx))
        .bind(i64::from(pet.spx))
        .bind(i64::from(pet.agi))
        .bind(i64::from(pet.fai))
        .bind(i64::from(pet.texp))
        .bind(i64::from(pet.skill_point))
        .bind(i64::from(pet.skills[0].0))
        .bind(i64::from(pet.skills[0].1))
        .bind(i64::from(pet.skills[1].0))
        .bind(i64::from(pet.skills[1].1))
        .bind(i64::from(pet.skills[2].0))
        .bind(i64::from(pet.skills[2].1))
        .bind(i64::from(pet.skills[3].0))
        .bind(i64::from(pet.skills[3].1))
        .execute(pool)
        .await
    {
        tracing::warn!("upsert_pet(stt {}) failed: {e}", pet.stt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §5.4 note 2: every ported DELETE over the 9 gameplay tables must carry a
    /// `player_id` predicate — the C# verbatim (`WHERE Id >= 0 AND Id <= 9`)
    /// would clear every player's basic skills in the shared schema.
    #[test]
    fn delete_system_skills_is_player_scoped() {
        assert!(
            DELETE_SYSTEM_SKILLS_SQL.contains("player_id = ?"),
            "skill purge must be player-scoped: {DELETE_SYSTEM_SKILLS_SQL}"
        );
        assert!(
            DELETE_SYSTEM_SKILLS_SQL.contains("Id >= 0 AND Id <= 9"),
            "must mirror the C# Logined1 predicate: {DELETE_SYSTEM_SKILLS_SQL}"
        );
    }
}
