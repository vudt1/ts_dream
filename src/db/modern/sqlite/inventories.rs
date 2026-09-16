//! `inventories` repository — one table for all five item containers.
//!
//! The 20 item columns map 1:1 to the 35-byte ThingData wire struct
//! (`src/protocol/codecs/thing_data.rs`) in field order.

use crate::db::modern::model::{InventorySlot, StorageType};
use crate::db::modern::traits::{InventoryRepository, RepoResult};
use crate::db::pool::DbPool;
use crate::protocol::codecs::thing_data::ThingData;
use sqlx::Row;

pub struct SqliteInventoryRepository<'a> {
    pub pool: &'a DbPool,
}

const SELECT_COLUMNS: &str = "storagetype AS storage_type, slot, itemid AS item_id, quantity, damage, element, \
     elementvalue AS element_value, proofkind AS proof_kind, growlevel AS grow_level, growexp AS grow_exp, \
     specialkind AS special_kind, stoneattr AS stone_attr, stonelevel AS stone_level, \
     enhancelevel AS enhance_level, deletetime AS delete_time, damageditemid AS damaged_item_id, \
     islocked AS is_locked, reinforced, affix1, affix2, affix3, stylelevel AS style_level";

impl InventoryRepository for SqliteInventoryRepository<'_> {
    async fn load_storage(
        &self,
        character_id: i64,
        storage_type: StorageType,
    ) -> RepoResult<Vec<InventorySlot>> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM inventories \
             WHERE playerid = ? AND storagetype = ? AND itemid > 0 ORDER BY slot"
        );
        let rows = sqlx::query(&sql)
            .bind(character_id)
            .bind(storage_type.value())
            .fetch_all(&self.pool.read)
            .await?;
        Ok(rows.iter().filter_map(row_to_slot).collect())
    }

    async fn save_slot<'e, E>(
        &self,
        character_id: i64,
        slot: &InventorySlot,
        executor: E,
    ) -> RepoResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(
            "INSERT INTO inventories \
             (playerid, storagetype, slot, itemid, quantity, damage, element, \
              elementvalue, proofkind, growlevel, growexp, specialkind, stoneattr, \
              stonelevel, enhancelevel, deletetime, damageditemid, islocked, reinforced, \
              affix1, affix2, affix3, stylelevel) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(playerid, storagetype, slot) DO UPDATE SET \
             itemid = excluded.itemid, quantity = excluded.quantity, damage = excluded.damage, \
             element = excluded.element, elementvalue = excluded.elementvalue, \
             proofkind = excluded.proofkind, growlevel = excluded.growlevel, \
             growexp = excluded.growexp, specialkind = excluded.specialkind, \
             stoneattr = excluded.stoneattr, stonelevel = excluded.stonelevel, \
             enhancelevel = excluded.enhancelevel, deletetime = excluded.deletetime, \
             damageditemid = excluded.damageditemid, islocked = excluded.islocked, \
             reinforced = excluded.reinforced, affix1 = excluded.affix1, affix2 = excluded.affix2, \
             affix3 = excluded.affix3, stylelevel = excluded.stylelevel",
        )
        .bind(character_id)
        .bind(slot.storage_type.value())
        .bind(slot.slot)
        .bind(slot.item.item_id)
        .bind(slot.item.quantity)
        .bind(slot.item.damage)
        .bind(slot.item.element)
        .bind(slot.item.element_value)
        .bind(slot.item.proof_kind)
        .bind(slot.item.grow_level)
        .bind(slot.item.grow_exp)
        .bind(slot.item.special_kind)
        .bind(slot.item.stone_attr)
        .bind(slot.item.stone_level)
        .bind(slot.item.enhance_level)
        .bind(slot.item.delete_time)
        .bind(slot.item.damaged_item_id)
        .bind(slot.item.is_locked)
        .bind(slot.item.reinforced)
        .bind(slot.item.affix1)
        .bind(slot.item.affix2)
        .bind(slot.item.affix3)
        .bind(slot.item.style_level)
        .execute(executor)
        .await?;
        Ok(())
    }

    async fn clear_slot<'e, E>(
        &self,
        character_id: i64,
        storage_type: StorageType,
        slot: u16,
        executor: E,
    ) -> RepoResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        // Keep the row (vacancy marker) but zero every item column so a later
        // upsert never resurrects stale attributes.
        sqlx::query(
            "UPDATE inventories SET itemid = 0, quantity = 0, damage = 0, element = 0, \
             elementvalue = 0, proofkind = 0, growlevel = 0, growexp = 0, \
             specialkind = 0, stoneattr = 0, stonelevel = 0, enhancelevel = 0, \
             deletetime = 0, damageditemid = 0, islocked = 0, reinforced = 0, \
             affix1 = 0, affix2 = 0, affix3 = 0, stylelevel = 0 \
             WHERE playerid = ? AND storagetype = ? AND slot = ?",
        )
        .bind(character_id)
        .bind(storage_type.value())
        .bind(slot)
        .execute(executor)
        .await?;
        Ok(())
    }

    async fn load_slot(
        &self,
        character_id: i64,
        storage_type: StorageType,
        slot: u16,
    ) -> RepoResult<Option<InventorySlot>> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM inventories \
             WHERE playerid = ? AND storagetype = ? AND slot = ?"
        );
        let row = sqlx::query(&sql)
            .bind(character_id)
            .bind(storage_type.value())
            .bind(slot)
            .fetch_optional(&self.pool.read)
            .await?;
        Ok(row.as_ref().and_then(row_to_slot))
    }
}

/// Maps one `inventories` row to an [`InventorySlot`]. Rows with an unknown
/// `storage_type` are skipped on load (defensive against manual edits).
fn row_to_slot(r: &sqlx::sqlite::SqliteRow) -> Option<InventorySlot> {
    let storage_raw: u8 = r.try_get("storage_type").ok()?;
    let storage_type = StorageType::from_value(storage_raw)?;
    let item = ThingData {
        item_id: r.try_get("item_id").ok()?,
        quantity: r.try_get("quantity").ok()?,
        damage: r.try_get("damage").ok()?,
        element: r.try_get("element").ok()?,
        element_value: r.try_get("element_value").ok()?,
        proof_kind: r.try_get("proof_kind").ok()?,
        grow_level: r.try_get("grow_level").ok()?,
        grow_exp: r.try_get("grow_exp").ok()?,
        special_kind: r.try_get("special_kind").ok()?,
        stone_attr: r.try_get("stone_attr").ok()?,
        stone_level: r.try_get("stone_level").ok()?,
        enhance_level: r.try_get("enhance_level").ok()?,
        delete_time: r.try_get("delete_time").ok()?,
        damaged_item_id: r.try_get("damaged_item_id").ok()?,
        is_locked: r.try_get::<bool, _>("is_locked").unwrap_or(false),
        reinforced: r.try_get("reinforced").ok()?,
        affix1: r.try_get("affix1").ok()?,
        affix2: r.try_get("affix2").ok()?,
        affix3: r.try_get("affix3").ok()?,
        style_level: r.try_get("style_level").ok()?,
    };
    Some(InventorySlot {
        storage_type,
        slot: r.try_get("slot").ok()?,
        item,
    })
}
