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

const SELECT_COLUMNS: &str = "storage_type, slot, item_id, quantity, damage, element, \
     element_value, proof_kind, grow_level, grow_exp, special_kind, stone_attr, stone_level, \
     enhance_level, delete_time, damaged_item_id, is_locked, reinforced, affix1, affix2, \
     affix3, style_level";

impl InventoryRepository for SqliteInventoryRepository<'_> {
    async fn load_storage(
        &self,
        character_id: i64,
        storage_type: StorageType,
    ) -> RepoResult<Vec<InventorySlot>> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM inventories \
             WHERE character_id = ? AND storage_type = ? AND item_id > 0 ORDER BY slot"
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
             (character_id, storage_type, slot, item_id, quantity, damage, element, \
              element_value, proof_kind, grow_level, grow_exp, special_kind, stone_attr, \
              stone_level, enhance_level, delete_time, damaged_item_id, is_locked, reinforced, \
              affix1, affix2, affix3, style_level) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(character_id, storage_type, slot) DO UPDATE SET \
             item_id = excluded.item_id, quantity = excluded.quantity, damage = excluded.damage, \
             element = excluded.element, element_value = excluded.element_value, \
             proof_kind = excluded.proof_kind, grow_level = excluded.grow_level, \
             grow_exp = excluded.grow_exp, special_kind = excluded.special_kind, \
             stone_attr = excluded.stone_attr, stone_level = excluded.stone_level, \
             enhance_level = excluded.enhance_level, delete_time = excluded.delete_time, \
             damaged_item_id = excluded.damaged_item_id, is_locked = excluded.is_locked, \
             reinforced = excluded.reinforced, affix1 = excluded.affix1, affix2 = excluded.affix2, \
             affix3 = excluded.affix3, style_level = excluded.style_level",
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
            "UPDATE inventories SET item_id = 0, quantity = 0, damage = 0, element = 0, \
             element_value = 0, proof_kind = 0, grow_level = 0, grow_exp = 0, \
             special_kind = 0, stone_attr = 0, stone_level = 0, enhance_level = 0, \
             delete_time = 0, damaged_item_id = 0, is_locked = 0, reinforced = 0, \
             affix1 = 0, affix2 = 0, affix3 = 0, style_level = 0 \
             WHERE character_id = ? AND storage_type = ? AND slot = ?",
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
             WHERE character_id = ? AND storage_type = ? AND slot = ?"
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
