//! Respec handler (Opcode 0x2A) for skills and attributes.
//!
//! Handles:
//! - Subcode 0x01 (C→S 0x2A 01): Skill level reduction using Memory Manual (46035 / 46166).
//! - Subcode 0x02 (C→S 0x2A 02): Attribute point reduction using Careful Pouch (46203).
//!
//! Concurrency & Parity:
//! - Protected by Two-Phase State Lock (`lock_player_operations`).
//! - Atomic commit to SQLite within a single transaction.
//! - Pre-transaction session snapshot rollback on failure with native error frame.
//! - Attribute reduction floor enforced.
//! - Synchronizes in-memory session via `registry_update_if_online`.
//! - Emits VISCII 1.1 warning frame via `sys_msg_frame` upon validation failure.

use crate::db::persist;
use crate::protocol::{
    encoder, ELEMENT_EARTH, ELEMENT_FIRE, ELEMENT_WATER, ELEMENT_WIND,
};
use crate::server::dispatcher::OpcodeCtx;
use crate::server::handlers::stats::build_stat_update;
use crate::server::session::{lock_player_operations, registry_update_if_online, InventoryItem, Session};
use crate::server::spawn::sys_msg_frame;

pub const ITEM_MEMORY_MANUAL_PRIMARY: u16 = 46035;   // Cẩm nang tâm tư (sơ cấp / thường)
pub const ITEM_MEMORY_MANUAL_ADVANCED: u16 = 46166;  // Cẩm nang tâm tư (cao cấp)
pub const ITEM_CAREFUL_POUCH: u16 = 46203;           // Túi gấm cẩn thận
pub const RESPEC_SUB_SKILL: u8 = 0x01;               // Tẩy điểm kỹ năng
pub const RESPEC_SUB_ATTR: u8 = 0x02;                // Tẩy điểm thuộc tính
pub const RESPEC_STATUS_SUCCESS: u8 = 0x00;          // Phản hồi thành công
pub const RESPEC_STATUS_FAILED: u8 = 0x01;           // Phản hồi thất bại

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Attr {
    Int,
    Atk,
    Def,
    Agi,
    Hpx,
    Spx,
}

/// Tính toán mức sàn thuộc tính ban đầu của nhân vật (Starting base stat floor).
/// Dựa theo race / hệ nguyên tố ở cấp 1 (tối thiểu 1 cho mọi chỉ số, 2 cho thuộc tính đặc thù).
pub fn calculate_starting_base_stat(session: &Session, attr: Attr) -> u16 {
    match (session.thuoctinh, attr) {
        (ELEMENT_EARTH, Attr::Def) => 2,
        (ELEMENT_WATER, Attr::Hpx) | (ELEMENT_WATER, Attr::Spx) => 2,
        (ELEMENT_FIRE, Attr::Atk) => 2,
        (ELEMENT_WIND, Attr::Agi) => 2,
        _ => 1,
    }
}

/// Trừ 1 vật phẩm khỏi homdo tại vị trí `pos`.
/// Trả về (slot, remaining_count, updated_item).
pub fn deduct_homdo_item_at(homdo: &mut Vec<InventoryItem>, pos: usize) -> (u8, u8, InventoryItem) {
    let slot = homdo[pos].slot;
    let remaining_count = homdo[pos].count.saturating_sub(1);
    let mut updated_item = homdo[pos].clone();
    updated_item.count = remaining_count;
    if remaining_count == 0 {
        updated_item.id = 0;
        homdo.retain(|i| i.slot != slot);
    } else {
        homdo[pos].count = remaining_count;
    }
    (slot, remaining_count, updated_item)
}

/// Dispatch Level-2 respec subcodes.
pub async fn handle_respec(ctx: &mut OpcodeCtx<'_>) {
    match ctx.sub {
        RESPEC_SUB_SKILL => handle_skill_respec(ctx).await,
        RESPEC_SUB_ATTR => handle_attr_respec(ctx).await,
        _ => {}
    }
}

/// C→S 0x2A 01: Downgrade a skill by 1 level using Memory Manual (46035 / 46166).
async fn handle_skill_respec(ctx: &mut OpcodeCtx<'_>) {
    if ctx.payload.len() < 2 {
        return;
    }
    let skill_id = encoder::u16_le(ctx.payload[0], ctx.payload[1]);
    let pid = ctx.conn.session.id;

    // Two-Phase State Lock
    let _guard = lock_player_operations([pid]).await;

    // Capture pre-transaction snapshot for RAM rollback integrity
    let before_session = ctx.conn.session.clone();

    // 1. Verify skill is learned with level >= 1
    let skill_pos = ctx
        .conn
        .session
        .skills
        .iter()
        .position(|&(id, lv)| id == skill_id && lv >= 1);
    let Some(spos) = skill_pos else {
        ctx.out.send(sys_msg_frame(
            "Kỹ năng chưa học hoặc đã ở cấp tối thiểu!",
        ));
        return;
    };

    // 2. Verify player has Memory Manual in homdo (46035 or 46166)
    let manual_pos = ctx
        .conn
        .session
        .homdo
        .iter()
        .position(|item| matches!(item.id, ITEM_MEMORY_MANUAL_PRIMARY | ITEM_MEMORY_MANUAL_ADVANCED) && item.count >= 1);
    let Some(mpos) = manual_pos else {
        ctx.out.send(sys_msg_frame(
            "Không có Cẩm nang tâm tư trong hành trang!",
        ));
        return;
    };

    // 3. Apply state mutation
    let old_lv = ctx.conn.session.skills[spos].1;
    let new_lv = old_lv.saturating_sub(1);
    if new_lv > 0 {
        ctx.conn.session.skills[spos].1 = new_lv;
    } else {
        ctx.conn.session.skills.remove(spos);
    }
    ctx.conn.session.skill_point = ctx.conn.session.skill_point.saturating_add(1);

    let (manual_slot, remaining_count, updated_item) =
        deduct_homdo_item_at(&mut ctx.conn.session.homdo, mpos);

    // 4. Atomic database commit
    if let Some(pool) = ctx.env.pool {
        let account_id = i64::from(pid);
        let mut tx = match pool.write.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::warn!("handle_skill_respec begin tx failed: {e}");
                ctx.conn.session = before_session;
                ctx.out.send(format!(
                    "F44403002A{:02X}{:02X}",
                    RESPEC_SUB_SKILL, RESPEC_STATUS_FAILED
                ));
                return;
            }
        };

        let res: Result<(), sqlx::Error> = async {
            if new_lv > 0 {
                sqlx::query(
                    "UPDATE character_skills SET level = ? WHERE playerid = ? AND skillid = ?",
                )
                .bind(i64::from(new_lv))
                .bind(account_id)
                .bind(i64::from(skill_id))
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "DELETE FROM character_skills WHERE playerid = ? AND skillid = ?",
                )
                .bind(account_id)
                .bind(i64::from(skill_id))
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("UPDATE characters SET skillpoint = ? WHERE playerid = ?")
                .bind(i64::from(ctx.conn.session.skill_point))
                .bind(account_id)
                .execute(&mut *tx)
                .await?;
            persist::upsert_item_tx(&mut tx, account_id, manual_slot, &updated_item).await?;
            Ok(())
        }
        .await;

        if let Err(e) = res {
            tracing::warn!("handle_skill_respec queries failed: {e}");
            let _ = tx.rollback().await;
            ctx.conn.session = before_session;
            ctx.out.send(format!(
                "F44403002A{:02X}{:02X}",
                RESPEC_SUB_SKILL, RESPEC_STATUS_FAILED
            ));
            return;
        }

        if let Err(e) = tx.commit().await {
            tracing::warn!("commit handle_skill_respec tx failed: {e}");
            ctx.conn.session = before_session;
            ctx.out.send(format!(
                "F44403002A{:02X}{:02X}",
                RESPEC_SUB_SKILL, RESPEC_STATUS_FAILED
            ));
            return;
        }
    }

    // 5. Registry update
    registry_update_if_online(&ctx.conn.session);

    // 6. Emit wire frames: 17 09, 08 01 6E 02, 08 01 25 01, 2A 01 00
    ctx.out.send(crate::server::inventory::item_count_changed_frame(
        manual_slot,
        remaining_count,
    ));
    ctx.out.send(format!(
        "F4440C0008016E02{}{}",
        encoder::le32(1),
        encoder::le32(skill_id as u32)
    ));
    ctx.out.send(format!(
        "F4440C0008012501{}00000000",
        encoder::le32(ctx.conn.session.skill_point as u32)
    ));
    ctx.out.send(format!(
        "F44403002A{:02X}{:02X}",
        RESPEC_SUB_SKILL, RESPEC_STATUS_SUCCESS
    ));
}

/// C→S 0x2A 02: Reduce an attribute point by 1 using Careful Pouch (46203).
async fn handle_attr_respec(ctx: &mut OpcodeCtx<'_>) {
    if ctx.payload.is_empty() {
        return;
    }
    let attr_code = ctx.payload[0];
    let pid = ctx.conn.session.id;

    // Two-Phase State Lock
    let _guard = lock_player_operations([pid]).await;

    // Capture pre-transaction snapshot for RAM rollback integrity
    let before_session = ctx.conn.session.clone();

    let attr = match attr_code {
        0 | 1 | 27 => Attr::Int,
        2 | 28 => Attr::Atk,
        3 | 29 => Attr::Def,
        4 | 30 => Attr::Agi,
        5 | 31 => Attr::Hpx,
        6 | 32 => Attr::Spx,
        _ => {
            ctx.out.send(sys_msg_frame("Mã thuộc tính không hợp lệ!"));
            return;
        }
    };

    // 1. Verify attribute has points above baseline floor
    let current_val = match attr {
        Attr::Int => ctx.conn.session.int1,
        Attr::Atk => ctx.conn.session.atk,
        Attr::Def => ctx.conn.session.def,
        Attr::Agi => ctx.conn.session.agi,
        Attr::Hpx => ctx.conn.session.hpx,
        Attr::Spx => ctx.conn.session.spx,
    };

    let base_val = calculate_starting_base_stat(&ctx.conn.session, attr);
    if current_val <= base_val {
        ctx.out.send(sys_msg_frame(
            "Thuộc tính đã ở mức tối thiểu, không thể giảm tiếp!",
        ));
        return;
    }

    // 2. Verify player has Careful Pouch in homdo (46203)
    let pouch_pos = ctx
        .conn
        .session
        .homdo
        .iter()
        .position(|item| item.id == ITEM_CAREFUL_POUCH && item.count >= 1);
    let Some(ppos) = pouch_pos else {
        ctx.out.send(sys_msg_frame(
            "Không có Túi gấm cẩn thận trong hành trang!",
        ));
        return;
    };

    // 3. Apply state mutation
    let new_val = current_val.saturating_sub(1);
    let (stat_prop, column_name) = match attr {
        Attr::Int => {
            ctx.conn.session.int1 = new_val;
            (0x1B, "baseint")
        }
        Attr::Atk => {
            ctx.conn.session.atk = new_val;
            (0x1C, "baseatk")
        }
        Attr::Def => {
            ctx.conn.session.def = new_val;
            (0x1D, "basedef")
        }
        Attr::Agi => {
            ctx.conn.session.agi = new_val;
            (0x1E, "baseagi")
        }
        Attr::Hpx => {
            ctx.conn.session.hpx = new_val;
            ctx.conn.session.recompute_stats();
            (0x1F, "basehpx")
        }
        Attr::Spx => {
            ctx.conn.session.spx = new_val;
            ctx.conn.session.recompute_stats();
            (0x20, "basespx")
        }
    };
    ctx.conn.session.point = ctx.conn.session.point.saturating_add(1);

    let (pouch_slot, remaining_count, updated_item) =
        deduct_homdo_item_at(&mut ctx.conn.session.homdo, ppos);

    // 4. Atomic database commit
    if let Some(pool) = ctx.env.pool {
        let account_id = i64::from(pid);
        let mut tx = match pool.write.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::warn!("handle_attr_respec begin tx failed: {e}");
                ctx.conn.session = before_session;
                ctx.out.send(format!(
                    "F44403002A{:02X}{:02X}",
                    RESPEC_SUB_ATTR, RESPEC_STATUS_FAILED
                ));
                return;
            }
        };

        let sql = format!(
            "UPDATE characters SET {column_name} = ?, freepoints = ?, maxhp = ?, maxsp = ? WHERE playerid = ?"
        );
        let res: Result<(), sqlx::Error> = async {
            sqlx::query(&sql)
                .bind(i64::from(new_val))
                .bind(i64::from(ctx.conn.session.point))
                .bind(i64::from(ctx.conn.session.hp_max))
                .bind(i64::from(ctx.conn.session.sp_max))
                .bind(account_id)
                .execute(&mut *tx)
                .await?;
            persist::upsert_item_tx(&mut tx, account_id, pouch_slot, &updated_item).await?;
            Ok(())
        }
        .await;

        if let Err(e) = res {
            tracing::warn!("handle_attr_respec queries failed: {e}");
            let _ = tx.rollback().await;
            ctx.conn.session = before_session;
            ctx.out.send(format!(
                "F44403002A{:02X}{:02X}",
                RESPEC_SUB_ATTR, RESPEC_STATUS_FAILED
            ));
            return;
        }

        if let Err(e) = tx.commit().await {
            tracing::warn!("commit handle_attr_respec tx failed: {e}");
            ctx.conn.session = before_session;
            ctx.out.send(format!(
                "F44403002A{:02X}{:02X}",
                RESPEC_SUB_ATTR, RESPEC_STATUS_FAILED
            ));
            return;
        }
    }

    // 5. Registry update
    registry_update_if_online(&ctx.conn.session);

    // 6. Emit wire frames: 17 09, 08 01 26 01, stat update, 2A 02 00
    ctx.out.send(crate::server::inventory::item_count_changed_frame(
        pouch_slot,
        remaining_count,
    ));
    ctx.out
        .send(build_stat_update(0x26, ctx.conn.session.point as i32));
    ctx.out.send(build_stat_update(stat_prop, new_val as i32));
    ctx.out.send(format!(
        "F44403002A{:02X}{:02X}",
        RESPEC_SUB_ATTR, RESPEC_STATUS_SUCCESS
    ));
}
