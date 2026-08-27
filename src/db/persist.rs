//! Modern-only write-through persistence for the live Rust server.
//!
//! The public functions retain the handler-facing API, but every SQL statement
//! targets the normalized 0002/0005 tables. `player_id` means the protocol
//! account id; it resolves to `characters.id` through `characters.account_id`.

use crate::server::session::{InventoryItem, PetState, Session};
use sqlx::{MySqlPool, MySqlTransaction};

async fn character_id_tx(
    tx: &mut MySqlTransaction<'_>,
    account_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM characters WHERE account_id = ? FOR UPDATE")
        .bind(account_id)
        .fetch_optional(&mut **tx)
        .await
}

fn character_column(column: &str) -> Option<&'static str> {
    Some(match column {
        "Lv" => "level",
        "Hp" => "hp",
        "HpMax" => "hp_max",
        "Sp" => "sp",
        "SpMax" => "sp_max",
        "Point" => "stat_point",
        "SkillPoint" => "skill_point",
        "Int" => "int_attr",
        "Atk" => "atk",
        "Def" => "def",
        "Hpx" => "hpx",
        "Spx" => "spx",
        "Agi" => "agi",
        "Int2" => "int2",
        "Atk2" => "atk2",
        "Def2" => "def2",
        "Hpx2" => "hpx2",
        "Spx2" => "spx2",
        "Agi2" => "agi2",
        "Texp" => "texp",
        "God" => "god",
        "tiengtam" => "tiengtam",
        "gocnhin" => "gocnhin",
        "Pk" => "pk",
        "ThamChien" => "tham_chien",
        "HP_Store" => "hp_store",
        "SP_Store" => "sp_store",
        "tanthu" => "tanthu",
        "savemap" => "savemap",
        "MapId" => "map_id",
        "MapX" => "map_x",
        "MapY" => "map_y",
        "Reborn" => "reborn",
        "Job" => "job",
        "Hair" => "hair",
        _ => return None,
    })
}

/// Update one modern `characters` field by protocol account id.
pub async fn update_player(pool: Option<&MySqlPool>, player_id: u32, column: &str, value: i64) {
    let Some(pool) = pool else { return };
    let account_id = i64::from(player_id);
    if matches!(column, "Gold" | "BankGold" | "ShopPoint") {
        let field = match column {
            "Gold" => "gold",
            "BankGold" => "bank_gold",
            "ShopPoint" => "shop_point",
            _ => unreachable!(),
        };
        let sql = format!(
            "INSERT INTO character_money (character_id, {field}) SELECT id, ? FROM characters WHERE account_id = ? ON DUPLICATE KEY UPDATE {field} = VALUES({field})"
        );
        if let Err(e) = sqlx::query(&sql)
            .bind(value)
            .bind(account_id)
            .execute(pool)
            .await
        {
            tracing::warn!("update_player({column}) failed: {e}");
        }
        return;
    }
    let Some(field) = character_column(column) else {
        tracing::warn!("skipped unknown modern character column write: {column}");
        return;
    };
    let sql = format!("UPDATE characters SET {field} = ? WHERE account_id = ?");
    if let Err(e) = sqlx::query(&sql)
        .bind(value)
        .bind(account_id)
        .execute(pool)
        .await
    {
        tracing::warn!("update_player({column}) failed: {e}");
    }
}

/// Update the normalized hotkey row formerly stored in `skillsave`.
pub async fn update_skillsave(pool: Option<&MySqlPool>, player_id: u32, slot: u8, skill: u16) {
    let Some(pool) = pool else { return };
    let result = sqlx::query(
        "INSERT INTO character_hotkeys (character_id, slot, skill_id)
         SELECT id, ?, ? FROM characters WHERE account_id = ?
         ON DUPLICATE KEY UPDATE skill_id = VALUES(skill_id)",
    )
    .bind(i64::from(slot))
    .bind(i64::from(skill))
    .bind(i64::from(player_id))
    .execute(pool)
    .await;
    if let Err(e) = result {
        tracing::warn!("update_skillsave(slot {slot}) failed: {e}");
    }
}

fn storage_type(table: &str) -> Option<u8> {
    Some(match table {
        "homdo" => 1,
        "tientrang" => 2,
        "trangbi" => 8,
        "tuideo" => 4,
        "luulang" => 16,
        _ => return None,
    })
}

pub async fn clear_items(pool: Option<&MySqlPool>, player_id: u32, table: &str) {
    let Some(pool) = pool else { return };
    let Some(storage) = storage_type(table) else {
        return;
    };
    let sql = "DELETE i FROM inventories i JOIN characters c ON c.id = i.character_id WHERE c.account_id = ? AND i.storage_type = ?";
    if let Err(e) = sqlx::query(sql)
        .bind(i64::from(player_id))
        .bind(storage)
        .execute(pool)
        .await
    {
        tracing::warn!("clear_items({table}) failed: {e}");
    }
}

pub async fn upsert_item(
    pool: Option<&MySqlPool>,
    player_id: u32,
    table: &str,
    item: &InventoryItem,
) {
    let Some(pool) = pool else { return };
    let Some(storage) = storage_type(table) else {
        return;
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!("begin item update failed: {e}");
            return;
        }
    };
    let result = async {
        if item.id == 0 {
            let id = character_id_tx(&mut tx, i64::from(player_id)).await?;
            if let Some(id) = id {
                sqlx::query(
                    "DELETE FROM inventories WHERE character_id = ? AND storage_type = ? AND slot = ?",
                )
                .bind(id)
                .bind(storage)
                .bind(i64::from(item.slot))
                .execute(&mut *tx)
                .await?;
            }
            Ok::<(), sqlx::Error>(())
        } else {
            upsert_inventory_tx(&mut tx, i64::from(player_id), storage, item).await
        }
    }
    .await;
    match result {
        Ok(()) => {
            if let Err(e) = tx.commit().await {
                tracing::warn!("commit item update failed: {e}");
            }
        }
        Err(e) => {
            tracing::warn!("upsert_item({table}, slot {}) failed: {e}", item.slot);
            let _ = tx.rollback().await;
        }
    }
}

async fn upsert_inventory_tx(
    tx: &mut MySqlTransaction<'_>,
    account_id: i64,
    storage: u8,
    item: &InventoryItem,
) -> Result<(), sqlx::Error> {
    let Some(id) = character_id_tx(tx, account_id).await? else {
        return Ok(());
    };
    sqlx::query("INSERT INTO inventories (character_id, storage_type, slot, item_id, quantity, damage, item_level, int1, atk1, def1, hpx1, spx1, agi1, fai1, int2, atk2, def2, hpx2, spx2, agi2, fai2, item_hp, item_sp, item_type, item_element, item_element_value, grow_exp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE item_id=VALUES(item_id), quantity=VALUES(quantity), damage=VALUES(damage), item_level=VALUES(item_level), int1=VALUES(int1), atk1=VALUES(atk1), def1=VALUES(def1), hpx1=VALUES(hpx1), spx1=VALUES(spx1), agi1=VALUES(agi1), fai1=VALUES(fai1), int2=VALUES(int2), atk2=VALUES(atk2), def2=VALUES(def2), hpx2=VALUES(hpx2), spx2=VALUES(spx2), agi2=VALUES(agi2), fai2=VALUES(fai2), item_hp=VALUES(item_hp), item_sp=VALUES(item_sp), item_type=VALUES(item_type), item_element=VALUES(item_element), item_element_value=VALUES(item_element_value), grow_exp=VALUES(grow_exp)")
        .bind(id).bind(storage).bind(i64::from(item.slot)).bind(i64::from(item.id)).bind(i64::from(item.count)).bind(i64::from(item.doben)).bind(i64::from(item.lv))
        .bind(i64::from(item.int1)).bind(i64::from(item.atk1)).bind(i64::from(item.def1)).bind(i64::from(item.hpx1)).bind(i64::from(item.spx1)).bind(i64::from(item.agi1)).bind(i64::from(item.fai1))
        .bind(i64::from(item.int2)).bind(i64::from(item.atk2)).bind(i64::from(item.def2)).bind(i64::from(item.hpx2)).bind(i64::from(item.spx2)).bind(i64::from(item.agi2)).bind(i64::from(item.fai2))
        .bind(i64::from(item.item_hp)).bind(i64::from(item.item_sp)).bind(i64::from(item.loai)).bind(i64::from(item.thuoctinh)).bind(i64::from(item.giatri_thuoctinh)).bind(i64::from(item.texp))
        .execute(&mut **tx).await?;
    Ok(())
}

pub async fn persist_shop_transaction(
    pool: Option<&MySqlPool>,
    buyer_id: u32,
    buyer_gold: u32,
    buyer_items: &[InventoryItem],
    seller_id: Option<u32>,
    seller_gold: Option<u32>,
    seller_items: Option<&[InventoryItem]>,
) -> bool {
    let Some(pool) = pool else { return true };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!("begin shop transaction failed: {e}");
            return false;
        }
    };
    let result = async {
        update_money_tx(&mut tx, i64::from(buyer_id), "gold", i64::from(buyer_gold)).await?;
        replace_inventory_tx(&mut tx, i64::from(buyer_id), 1, buyer_items).await?;
        if let (Some(id), Some(gold), Some(items)) = (seller_id, seller_gold, seller_items) {
            update_money_tx(&mut tx, i64::from(id), "gold", i64::from(gold)).await?;
            replace_inventory_tx(&mut tx, i64::from(id), 1, items).await?;
        }
        Ok::<(), sqlx::Error>(())
    }
    .await;
    match result {
        Ok(()) => tx.commit().await.is_ok(),
        Err(e) => {
            tracing::warn!("shop transaction rolled back: {e}");
            let _ = tx.rollback().await;
            false
        }
    }
}

pub async fn persist_sessions_transaction(
    pool: Option<&MySqlPool>,
    sessions: &[&Session],
    _tables: &[&str],
) -> bool {
    let Some(pool) = pool else { return true };
    let repo = crate::db::modern::mysql::session::MySqlSessionRepository { pool };
    for session in sessions {
        if let Err(e) = repo.save(session).await {
            tracing::warn!("modern session save for player {} failed: {e}", session.id);
            return false;
        }
    }
    true
}

async fn replace_inventory_tx(
    tx: &mut MySqlTransaction<'_>,
    account_id: i64,
    storage: u8,
    items: &[InventoryItem],
) -> Result<(), sqlx::Error> {
    let Some(id) = character_id_tx(tx, account_id).await? else {
        return Ok(());
    };
    sqlx::query("DELETE FROM inventories WHERE character_id = ? AND storage_type = ?")
        .bind(id)
        .bind(storage)
        .execute(&mut **tx)
        .await?;
    for item in items.iter().filter(|i| i.id > 0 && i.count > 0) {
        upsert_inventory_tx(tx, account_id, storage, item).await?;
    }
    Ok(())
}

async fn update_money_tx(
    tx: &mut MySqlTransaction<'_>,
    account_id: i64,
    field: &str,
    value: i64,
) -> Result<(), sqlx::Error> {
    let field = match field {
        "gold" => "gold",
        "bank_gold" => "bank_gold",
        "shop_point" => "shop_point",
        _ => return Ok(()),
    };
    let sql = format!("INSERT INTO character_money (character_id, {field}) SELECT id, ? FROM characters WHERE account_id = ? ON DUPLICATE KEY UPDATE {field}=VALUES({field})");
    sqlx::query(&sql)
        .bind(value)
        .bind(account_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn persist_shop_point_and_item(
    pool: Option<&MySqlPool>,
    player_id: u32,
    points: u32,
    item: &InventoryItem,
) -> bool {
    let Some(pool) = pool else { return true };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return false,
    };
    let result = async {
        update_money_tx(
            &mut tx,
            i64::from(player_id),
            "shop_point",
            i64::from(points),
        )
        .await?;
        upsert_inventory_tx(&mut tx, i64::from(player_id), 1, item).await
    }
    .await;
    match result {
        Ok(()) => tx.commit().await.is_ok(),
        Err(e) => {
            tracing::warn!("modern mall transaction failed: {e}");
            let _ = tx.rollback().await;
            false
        }
    }
}

pub(crate) async fn upsert_item_tx(
    tx: &mut MySqlTransaction<'_>,
    player_id: i64,
    slot: u8,
    item: &InventoryItem,
) -> Result<(), sqlx::Error> {
    let mut item = item.clone();
    item.slot = slot;
    upsert_inventory_tx(tx, player_id, 1, &item).await
}

pub async fn upsert_skill(
    pool: Option<&MySqlPool>,
    player_id: u32,
    skill_id: u16,
    lv: u8,
    sp: u8,
    save: u8,
) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query("INSERT INTO character_skills (character_id, skill_id, level, sp, save_flag) SELECT id, ?, ?, ?, ? FROM characters WHERE account_id = ? ON DUPLICATE KEY UPDATE level=VALUES(level), sp=VALUES(sp), save_flag=VALUES(save_flag)")
        .bind(i64::from(skill_id)).bind(i64::from(lv)).bind(i64::from(sp)).bind(i64::from(save)).bind(i64::from(player_id)).execute(pool).await { tracing::warn!("modern upsert_skill failed: {e}"); }
}

pub async fn delete_reborn_skills(pool: Option<&MySqlPool>, player_id: u32) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query("DELETE s FROM character_skills s JOIN characters c ON c.id=s.character_id WHERE c.account_id=? AND s.skill_id BETWEEN 10001 AND 13033 AND s.skill_id NOT IN (10016,10017,10018,10019,11016,11017,11018,11019,12016,12017,12018,12019,13015,13016,13017,13018)")
        .bind(i64::from(player_id)).execute(pool).await { tracing::warn!("modern delete_reborn_skills failed: {e}"); }
}

pub const DELETE_SYSTEM_SKILLS_SQL: &str = "DELETE s FROM character_skills s JOIN characters c ON c.id = s.character_id WHERE c.account_id = ? AND s.skill_id >= 0 AND s.skill_id <= 9";

pub async fn delete_system_skills(pool: Option<&MySqlPool>, player_id: u32) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query(DELETE_SYSTEM_SKILLS_SQL)
        .bind(i64::from(player_id))
        .execute(pool)
        .await
    {
        tracing::warn!("modern delete_system_skills failed: {e}");
    }
}

pub async fn upsert_pet(pool: Option<&MySqlPool>, player_id: u32, pet: &PetState) {
    let Some(pool) = pool else { return };
    let storage = if pet.stt <= 4 { 1u8 } else { 3u8 };
    let slot = if pet.stt <= 4 {
        pet.stt
    } else {
        pet.stt.saturating_sub(4)
    };
    let [s1, s2, s3, s4] = pet.skills;
    let result = sqlx::query("INSERT INTO character_pets (character_id, storage_type, slot, pet_id, name, level, element, reborn, hp, hp_max, sp, sp_max, int_attr, atk, def, hpx, spx, agi, fai, int2, atk2, def2, hpx2, spx2, agi2, thd, texp, skill_point, quest, skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, skill4_id, skill4_level) SELECT id, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? FROM characters WHERE account_id = ? ON DUPLICATE KEY UPDATE pet_id=VALUES(pet_id), name=VALUES(name), level=VALUES(level), element=VALUES(element), reborn=VALUES(reborn), hp=VALUES(hp), hp_max=VALUES(hp_max), sp=VALUES(sp), sp_max=VALUES(sp_max), int_attr=VALUES(int_attr), atk=VALUES(atk), def=VALUES(def), hpx=VALUES(hpx), spx=VALUES(spx), agi=VALUES(agi), fai=VALUES(fai), int2=VALUES(int2), atk2=VALUES(atk2), def2=VALUES(def2), hpx2=VALUES(hpx2), spx2=VALUES(spx2), agi2=VALUES(agi2), thd=VALUES(thd), texp=VALUES(texp), skill_point=VALUES(skill_point), quest=VALUES(quest), skill1_id=VALUES(skill1_id), skill1_level=VALUES(skill1_level), skill2_id=VALUES(skill2_id), skill2_level=VALUES(skill2_level), skill3_id=VALUES(skill3_id), skill3_level=VALUES(skill3_level), skill4_id=VALUES(skill4_id), skill4_level=VALUES(skill4_level)")
        .bind(storage).bind(i64::from(slot)).bind(i64::from(pet.id)).bind(&pet.name).bind(i64::from(pet.level)).bind(i64::from(pet.thuoctinh)).bind(i64::from(pet.reborn)).bind(i64::from(pet.hp)).bind(i64::from(pet.hp_max)).bind(i64::from(pet.sp)).bind(i64::from(pet.sp_max)).bind(i64::from(pet.int1)).bind(i64::from(pet.atk)).bind(i64::from(pet.def)).bind(i64::from(pet.hpx)).bind(i64::from(pet.spx)).bind(i64::from(pet.agi)).bind(i64::from(pet.fai)).bind(i64::from(pet.int2)).bind(i64::from(pet.atk2)).bind(i64::from(pet.def2)).bind(i64::from(pet.hpx2)).bind(i64::from(pet.spx2)).bind(i64::from(pet.agi2)).bind(i64::from(pet.thd)).bind(i64::from(pet.texp)).bind(i64::from(pet.skill_point)).bind(i64::from(pet.quest)).bind(i64::from(s1.0)).bind(i64::from(s1.1)).bind(i64::from(s2.0)).bind(i64::from(s2.1)).bind(i64::from(s3.0)).bind(i64::from(s3.1)).bind(i64::from(s4.0)).bind(i64::from(s4.1)).bind(i64::from(player_id)).execute(pool).await;
    if let Err(e) = result {
        tracing::warn!("modern upsert_pet(stt {}) failed: {e}", pet.stt);
    }
}
