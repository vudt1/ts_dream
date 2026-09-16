//! Modern-only write-through persistence for the live Rust server.
//!
//! The public functions retain the handler-facing API, but every SQL statement
//! targets the normalized 0001 tables. `player_id` is the shared PK
//! `accounts.player_id = characters.character_id` (1:1).

use crate::db::pool::DbPool;
use crate::server::session::{InventoryItem, PetState, Session};
use sqlx::{Sqlite, Transaction};

pub type SqliteTx<'a> = Transaction<'a, Sqlite>;

async fn character_id_tx(
    tx: &mut SqliteTx<'_>,
    account_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar("SELECT playerid FROM characters WHERE playerid = ?")
        .bind(account_id)
        .fetch_optional(&mut **tx)
        .await
}

fn character_column(column: &str) -> Option<&'static str> {
    Some(match column {
        "Lv" | "level" => "level",
        "Hp" | "curhp" => "curhp",
        "HpMax" | "maxhp" => "maxhp",
        "Sp" | "cursp" => "cursp",
        "SpMax" | "maxsp" => "maxsp",
        "Point" | "freepoints" => "freepoints",
        "SkillPoint" | "skillpoint" => "skillpoint",
        "Int" | "baseint" => "baseint",
        "Atk" | "baseatk" => "baseatk",
        "Def" | "basedef" => "basedef",
        "Hpx" | "basehpx" => "basehpx",
        "Spx" | "basespx" => "basespx",
        "Agi" | "baseagi" => "baseagi",
        "Pk" | "pk" => "pk",
        "Texp" | "Exp" | "texp" | "exp" => "curexp",
        "Newbie" | "newbie" => "newbie",
        "MapId" | "mapid" => "mapid",
        "MapX" | "mapx" => "mapx",
        "MapY" | "mapy" => "mapy",
        "Reborn" | "rebornstage" => "rebornstage",
        "Job" | "jobtype" => "jobtype",
        "Hair" | "hair" => "hair",
        _ => return None,
    })
}

/// Update one modern `characters` field by protocol account id.
pub async fn update_player(pool: Option<&DbPool>, player_id: u32, column: &str, value: i64) {
    let Some(pool) = pool else { return };
    let account_id = i64::from(player_id);
    if column == "SttPetXuatchien" {
        let mut tx = match pool.write.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::warn!("begin SttPetXuatchien tx failed: {e}");
                return;
            }
        };
        let _ = sqlx::query("UPDATE character_pets SET isactive = 0 WHERE playerid = ?")
            .bind(account_id)
            .execute(&mut *tx)
            .await;
        if value > 0 {
            let _ = sqlx::query(
                "UPDATE character_pets SET isactive = 1 WHERE playerid = ? AND storagetype = 1 AND slot = ?",
            )
            .bind(account_id)
            .bind(value)
            .execute(&mut *tx)
            .await;
        }
        if let Err(e) = tx.commit().await {
            tracing::warn!("commit SttPetXuatchien tx failed: {e}");
        }
        return;
    }
    if matches!(column, "Gold" | "BankGold" | "ShopPoint") {
        let field = match column {
            "Gold" => "gold",
            "BankGold" => "bankgold",
            "ShopPoint" => "shoppoint",
            _ => unreachable!(),
        };
        let sql = format!(
            "INSERT INTO character_money (playerid, {field}) VALUES (?, ?) \
             ON CONFLICT(playerid) DO UPDATE SET {field} = excluded.{field}"
        );
        if let Err(e) = sqlx::query(&sql)
            .bind(account_id)
            .bind(value)
            .execute(&pool.write)
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
    let sql = format!("UPDATE characters SET {field} = ? WHERE playerid = ?");
    if let Err(e) = sqlx::query(&sql)
        .bind(value)
        .bind(account_id)
        .execute(&pool.write)
        .await
    {
        tracing::warn!("update_player({column}) failed: {e}");
    }
}

/// Update the normalized hotkey row formerly stored in `skillsave`.
pub async fn update_skillsave(pool: Option<&DbPool>, player_id: u32, slot: u8, skill: u16) {
    let Some(pool) = pool else { return };
    let result = sqlx::query(
        "INSERT INTO character_hotkeys (playerid, slot, skillid)
         VALUES (?, ?, ?)
         ON CONFLICT(playerid, slot) DO UPDATE SET skillid = excluded.skillid",
    )
    .bind(i64::from(player_id))
    .bind(i64::from(slot))
    .bind(i64::from(skill))
    .execute(&pool.write)
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

pub async fn clear_items(pool: Option<&DbPool>, player_id: u32, table: &str) {
    let Some(pool) = pool else { return };
    let Some(storage) = storage_type(table) else {
        return;
    };
    let sql = "DELETE FROM inventories WHERE playerid = ? AND storagetype = ?";
    if let Err(e) = sqlx::query(sql)
        .bind(i64::from(player_id))
        .bind(storage)
        .execute(&pool.write)
        .await
    {
        tracing::warn!("clear_items({table}) failed: {e}");
    }
}

pub async fn upsert_item(
    pool: Option<&DbPool>,
    player_id: u32,
    table: &str,
    item: &InventoryItem,
) {
    let Some(pool) = pool else { return };
    let Some(storage) = storage_type(table) else {
        return;
    };
    let mut tx = match pool.write.begin().await {
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
                    "DELETE FROM inventories WHERE playerid = ? AND storagetype = ? AND slot = ?",
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
    tx: &mut SqliteTx<'_>,
    account_id: i64,
    storage: u8,
    item: &InventoryItem,
) -> Result<(), sqlx::Error> {
    let Some(id) = character_id_tx(tx, account_id).await? else {
        return Ok(());
    };
    let _ = item;
    sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(playerid, storagetype, slot) DO UPDATE SET itemid=excluded.itemid, quantity=excluded.quantity, damage=excluded.damage")
        .bind(id).bind(storage).bind(i64::from(item.slot)).bind(i64::from(item.id)).bind(i64::from(item.count)).bind(i64::from(item.doben))
        .execute(&mut **tx).await?;
    Ok(())
}

pub async fn persist_shop_transaction(
    pool: Option<&DbPool>,
    buyer_id: u32,
    buyer_gold: u32,
    buyer_items: &[InventoryItem],
    seller_id: Option<u32>,
    seller_gold: Option<u32>,
    seller_items: Option<&[InventoryItem]>,
) -> bool {
    let Some(pool) = pool else { return true };
    let mut tx = match pool.write.begin().await {
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
    pool: Option<&DbPool>,
    sessions: &[&Session],
    _tables: &[&str],
) -> bool {
    let Some(pool) = pool else { return true };
    let repo = crate::db::modern::sqlite::session::SqliteSessionRepository { pool };
    for session in sessions {
        if let Err(e) = repo.save(session).await {
            tracing::warn!("modern session save for player {} failed: {e}", session.id);
            return false;
        }
    }
    true
}

async fn replace_inventory_tx(
    tx: &mut SqliteTx<'_>,
    account_id: i64,
    storage: u8,
    items: &[InventoryItem],
) -> Result<(), sqlx::Error> {
    let Some(id) = character_id_tx(tx, account_id).await? else {
        return Ok(());
    };
    sqlx::query("DELETE FROM inventories WHERE playerid = ? AND storagetype = ?")
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
    tx: &mut SqliteTx<'_>,
    account_id: i64,
    field: &str,
    value: i64,
) -> Result<(), sqlx::Error> {
    let field = match field {
        "gold" => "gold",
        "bank_gold" | "bankgold" => "bankgold",
        "shop_point" | "shoppoint" => "shoppoint",
        _ => return Ok(()),
    };
    let Some(id) = character_id_tx(tx, account_id).await? else {
        return Ok(());
    };
    let sql = format!("INSERT INTO character_money (playerid, {field}) VALUES (?, ?) ON CONFLICT(playerid) DO UPDATE SET {field}=excluded.{field}");
    sqlx::query(&sql)
        .bind(id)
        .bind(value)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn persist_shop_point_and_item(
    pool: Option<&DbPool>,
    player_id: u32,
    points: u32,
    item: &InventoryItem,
) -> bool {
    let Some(pool) = pool else { return true };
    let mut tx = match pool.write.begin().await {
        Ok(tx) => tx,
        Err(_) => return false,
    };
    let result = async {
        update_money_tx(
            &mut tx,
            i64::from(player_id),
            "shoppoint",
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
    tx: &mut SqliteTx<'_>,
    player_id: i64,
    slot: u8,
    item: &InventoryItem,
) -> Result<(), sqlx::Error> {
    let mut item = item.clone();
    item.slot = slot;
    upsert_inventory_tx(tx, player_id, 1, &item).await
}

pub async fn upsert_skill(
    pool: Option<&DbPool>,
    player_id: u32,
    skill_id: u16,
    lv: u8,
    sp: u8,
    save: u8,
) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query("INSERT INTO character_skills (playerid, skillid, level, sp, saveflag) VALUES (?, ?, ?, ?, ?) ON CONFLICT(playerid, skillid) DO UPDATE SET level=excluded.level, sp=excluded.sp, saveflag=excluded.saveflag")
        .bind(i64::from(player_id)).bind(i64::from(skill_id)).bind(i64::from(lv)).bind(i64::from(sp)).bind(i64::from(save)).execute(&pool.write).await { tracing::warn!("modern upsert_skill failed: {e}"); }
}

pub async fn delete_reborn_skills(pool: Option<&DbPool>, player_id: u32) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query("DELETE FROM character_skills WHERE playerid=? AND skillid BETWEEN 10001 AND 13033 AND skillid NOT IN (10016,10017,10018,10019,11016,11017,11018,11019,12016,12017,12018,12019,13015,13016,13017,13018)")
        .bind(i64::from(player_id)).execute(&pool.write).await { tracing::warn!("modern delete_reborn_skills failed: {e}"); }
}

pub const DELETE_SYSTEM_SKILLS_SQL: &str = "DELETE FROM character_skills WHERE playerid = ? AND skillid >= 0 AND skillid <= 9";

pub async fn delete_system_skills(pool: Option<&DbPool>, player_id: u32) {
    let Some(pool) = pool else { return };
    if let Err(e) = sqlx::query(DELETE_SYSTEM_SKILLS_SQL)
        .bind(i64::from(player_id))
        .execute(&pool.write)
        .await
    {
        tracing::warn!("modern delete_system_skills failed: {e}");
    }
}

pub async fn upsert_pet(pool: Option<&DbPool>, player_id: u32, pet: &PetState) {
    let Some(pool) = pool else { return };
    let storage = if pet.stt <= 4 { 1u8 } else { 3u8 };
    let slot = if pet.stt <= 4 {
        pet.stt
    } else {
        pet.stt.saturating_sub(4)
    };
    let [s1, s2, s3, s4] = pet.skills;
    let result = sqlx::query("INSERT INTO character_pets (playerid, storagetype, slot, petid, name, level, element, rebornstage, curhp, maxhp, cursp, maxsp, baseint, baseatk, basedef, basehpx, basespx, baseagi, fai, thd, texp, skillpoint, quest, skill1_id, skill1_level, skill2_id, skill2_level, skill3_id, skill3_level, skill4_id, skill4_level) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(playerid, storagetype, slot) DO UPDATE SET petid=excluded.petid, name=excluded.name, level=excluded.level, element=excluded.element, rebornstage=excluded.rebornstage, curhp=excluded.curhp, maxhp=excluded.maxhp, cursp=excluded.cursp, maxsp=excluded.maxsp, baseint=excluded.baseint, baseatk=excluded.baseatk, basedef=excluded.basedef, basehpx=excluded.basehpx, basespx=excluded.basespx, baseagi=excluded.baseagi, fai=excluded.fai, thd=excluded.thd, texp=excluded.texp, skillpoint=excluded.skillpoint, quest=excluded.quest, skill1_id=excluded.skill1_id, skill1_level=excluded.skill1_level, skill2_id=excluded.skill2_id, skill2_level=excluded.skill2_level, skill3_id=excluded.skill3_id, skill3_level=excluded.skill3_level, skill4_id=excluded.skill4_id, skill4_level=excluded.skill4_level")
        .bind(i64::from(player_id)).bind(storage).bind(i64::from(slot)).bind(i64::from(pet.id)).bind(&pet.name).bind(i64::from(pet.level)).bind(i64::from(pet.thuoctinh)).bind(i64::from(pet.reborn)).bind(i64::from(pet.hp)).bind(i64::from(pet.hp_max)).bind(i64::from(pet.sp)).bind(i64::from(pet.sp_max)).bind(i64::from(pet.int1)).bind(i64::from(pet.atk)).bind(i64::from(pet.def)).bind(i64::from(pet.hpx)).bind(i64::from(pet.spx)).bind(i64::from(pet.agi)).bind(i64::from(pet.fai)).bind(i64::from(pet.thd)).bind(i64::from(pet.texp)).bind(i64::from(pet.skill_point)).bind(i64::from(pet.quest)).bind(i64::from(s1.0)).bind(i64::from(s1.1)).bind(i64::from(s2.0)).bind(i64::from(s2.1)).bind(i64::from(s3.0)).bind(i64::from(s3.1)).bind(i64::from(s4.0)).bind(i64::from(s4.1)).execute(&pool.write).await;
    if let Err(e) = result {
        tracing::warn!("modern upsert_pet(stt {}) failed: {e}", pet.stt);
    }
}

