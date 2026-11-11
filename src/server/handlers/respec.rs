//! Respec System Handler (Opcode 0x2A `OP_RESET`).
//!
//! Handles skill level regression via Memory Manuals (`46035` / `46166`, Subcode `0x01`)
//! and attribute point regression via Careful Pouches (`46203`, Subcode `0x02`).
//! Enforces Two-Phase State Lock, starting base stat floors, atomic SQLite write
//! transactions, RAM snapshot rollback on DB failure, and Delphi client ACK frames.

use crate::protocol::{
    encoder, ELEMENT_EARTH, ELEMENT_FIRE, ELEMENT_WATER, ELEMENT_WIND,
};
use crate::server::dispatcher::OpcodeCtx;
use crate::server::handlers::stats::build_stat_update;
use crate::server::inventory::item_count_changed_frame;
use crate::server::session::{lock_player_operations, registry_update_if_online, Session};
use crate::server::spawn::sys_msg_frame;

/// Cẩm nang tâm tư (Primary skill respec consumable).
pub const ITEM_MEMORY_MANUAL_PRIMARY: u16 = 46035;
/// Cẩm nang tâm tư (Alternate skill respec consumable).
pub const ITEM_MEMORY_MANUAL_ALT: u16 = 46166;
/// Túi gấm cẩn thận (Attribute respec consumable).
pub const ITEM_CAREFUL_POUCH: u16 = 46203;

/// Subcode `0x01`: Skill regression (`TBK_SkillRegressForm`).
pub const RESPEC_SUB_SKILL: u8 = 0x01;
/// Subcode `0x02`: Attribute regression (`TBK_AttrRegressForm`).
pub const RESPEC_SUB_ATTR: u8 = 0x02;

/// Status code `0x00`: Respec succeeded (plays `WA0014.wav`, shows success toast, closes form).
pub const RESPEC_STATUS_SUCCESS: u8 = 0x00;
/// Status code `0x01`: Respec failed (shows failure toast, closes form).
pub const RESPEC_STATUS_FAIL: u8 = 0x01;

/// Base character attribute targeted by `0x2A 02`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Int,
    Atk,
    Def,
    Hpx,
    Spx,
    Agi,
}

impl Attr {
    /// Parses the attribute code sent by `TBK_AttrRegressForm` (`1..=6` or prop `27..=32`).
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 | 27 => Some(Self::Int),
            2 | 28 => Some(Self::Atk),
            3 | 29 => Some(Self::Def),
            4 | 31 => Some(Self::Hpx),
            5 | 32 => Some(Self::Spx),
            6 | 30 => Some(Self::Agi),
            _ => None,
        }
    }

    #[must_use]
    const fn db_column(self) -> &'static str {
        match self {
            Self::Int => "baseint",
            Self::Atk => "baseatk",
            Self::Def => "basedef",
            Self::Hpx => "basehpx",
            Self::Spx => "basespx",
            Self::Agi => "baseagi",
        }
    }

    #[must_use]
    const fn prop_code(self) -> u8 {
        match self {
            Self::Int => 0x1B,
            Self::Atk => 0x1C,
            Self::Def => 0x1D,
            Self::Agi => 0x1E,
            Self::Hpx => 0x1F,
            Self::Spx => 0x20,
        }
    }
}

/// Returns the level-1 starting base stat floor for a given character element and attribute.
#[must_use]
pub fn calculate_starting_base_stat(session: &Session, attr: Attr) -> u16 {
    match (session.thuoctinh, attr) {
        (ELEMENT_EARTH, Attr::Def) => 2,
        (ELEMENT_WATER, Attr::Hpx) | (ELEMENT_WATER, Attr::Spx) => 2,
        (ELEMENT_FIRE, Attr::Atk) => 2,
        (ELEMENT_WIND, Attr::Agi) => 2,
        _ => 1,
    }
}

/// Deducts 1 unit from the inventory item at `slot` in `session.homdo`, removing the entry
/// if its quantity reaches 0, and returns the remaining count.
pub fn deduct_homdo_item_at(session: &mut Session, slot: u8) -> u8 {
    if let Some(pos) = session.homdo.iter().position(|i| i.slot == slot) {
        if session.homdo[pos].count > 1 {
            session.homdo[pos].count -= 1;
            return session.homdo[pos].count;
        }
        session.homdo.remove(pos);
    }
    0
}

fn build_respec_ack_frame(sub: u8, status: u8) -> String {
    format!("F44403002A{:02X}{:02X}", sub, status)
}

/// Entrypoint for Opcode `0x2A` (`OP_RESET`).
pub async fn handle_respec(ctx: &mut OpcodeCtx<'_>) {
    if ctx.conn.session.battle_id > 0 {
        return;
    }

    let pid = ctx.conn.session.id;
    let _op_guard = lock_player_operations([pid]).await;

    match ctx.sub {
        RESPEC_SUB_SKILL => handle_skill_respec(ctx).await,
        RESPEC_SUB_ATTR => handle_attr_respec(ctx).await,
        _ => {}
    }
}

async fn handle_skill_respec(ctx: &mut OpcodeCtx<'_>) {
    if ctx.payload.len() < 2 {
        return;
    }
    let skill_id = encoder::u16_le(ctx.payload[0], ctx.payload[1]);
    if skill_id == 0 {
        return;
    }

    let Some(sk_pos) = ctx
        .conn
        .session
        .skills
        .iter()
        .position(|(id, lv)| *id == skill_id && *lv >= 1)
    else {
        ctx.out
            .send(sys_msg_frame("Khong tim thay ky nang de khoi phuc"));
        return;
    };

    let Some(manual_slot) = ctx
        .conn
        .session
        .homdo
        .iter()
        .find(|i| {
            matches!(
                i.id,
                ITEM_MEMORY_MANUAL_PRIMARY | ITEM_MEMORY_MANUAL_ALT
            ) && i.count >= 1
        })
        .map(|i| i.slot)
    else {
        ctx.out.send(sys_msg_frame("Khong co Cam nang tam tu"));
        return;
    };

    let before_session = ctx.conn.session.clone();
    let current_lv = ctx.conn.session.skills[sk_pos].1;
    let new_lv = current_lv - 1;

    if new_lv == 0 {
        ctx.conn.session.skills.remove(sk_pos);
    } else {
        ctx.conn.session.skills[sk_pos].1 = new_lv;
    }
    ctx.conn.session.skill_point = ctx.conn.session.skill_point.saturating_add(1);
    let remaining_count = deduct_homdo_item_at(&mut ctx.conn.session, manual_slot);

    let pid = ctx.conn.session.id;
    let new_sp = ctx.conn.session.skill_point;

    if let Some(pool) = ctx.env.pool {
        let db_result: Result<(), sqlx::Error> = async {
            let mut tx = pool.write.begin().await?;

            if new_lv == 0 {
                sqlx::query("DELETE FROM character_skills WHERE playerid = ? AND skillid = ?")
                    .bind(i64::from(pid))
                    .bind(i64::from(skill_id))
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query(
                    "UPDATE character_skills SET level = ? WHERE playerid = ? AND skillid = ?",
                )
                .bind(i64::from(new_lv))
                .bind(i64::from(pid))
                .bind(i64::from(skill_id))
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query("UPDATE characters SET skillpoint = ? WHERE playerid = ?")
                .bind(i64::from(new_sp))
                .bind(i64::from(pid))
                .execute(&mut *tx)
                .await?;

            if remaining_count == 0 {
                sqlx::query(
                    "DELETE FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                )
                .bind(i64::from(pid))
                .bind(i64::from(manual_slot))
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE inventories SET quantity = ? WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                )
                .bind(i64::from(remaining_count))
                .bind(i64::from(pid))
                .bind(i64::from(manual_slot))
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
            Ok(())
        }
        .await;

        if let Err(e) = db_result {
            tracing::warn!("Skill respec transaction failed for player {pid}: {e}");
            ctx.conn.session = before_session;
            ctx.out
                .send(build_respec_ack_frame(RESPEC_SUB_SKILL, RESPEC_STATUS_FAIL));
            return;
        }
    }

    ctx.out
        .send(item_count_changed_frame(manual_slot, remaining_count));
    ctx.out.send(format!(
        "F4440C0008016E02{}{}",
        encoder::le32(1),
        encoder::le32(u32::from(skill_id))
    ));
    ctx.out.send(format!(
        "F4440C0008012501{}00000000",
        encoder::le32(u32::from(new_sp))
    ));
    ctx.out.send(build_respec_ack_frame(
        RESPEC_SUB_SKILL,
        RESPEC_STATUS_SUCCESS,
    ));

    registry_update_if_online(&ctx.conn.session);
}

async fn handle_attr_respec(ctx: &mut OpcodeCtx<'_>) {
    if ctx.payload.is_empty() {
        return;
    }
    let Some(attr) = Attr::from_code(ctx.payload[0]) else {
        ctx.out
            .send(sys_msg_frame("Thuoc tinh khoi phuc khong hop le"));
        return;
    };

    let current_val = match attr {
        Attr::Int => ctx.conn.session.int1,
        Attr::Atk => ctx.conn.session.atk,
        Attr::Def => ctx.conn.session.def,
        Attr::Hpx => ctx.conn.session.hpx,
        Attr::Spx => ctx.conn.session.spx,
        Attr::Agi => ctx.conn.session.agi,
    };

    let floor = calculate_starting_base_stat(&ctx.conn.session, attr);
    if current_val <= floor {
        ctx.out
            .send(sys_msg_frame("Chi so da dat muc san khoi diem"));
        return;
    }

    let Some(pouch_slot) = ctx
        .conn
        .session
        .homdo
        .iter()
        .find(|i| i.id == ITEM_CAREFUL_POUCH && i.count >= 1)
        .map(|i| i.slot)
    else {
        ctx.out.send(sys_msg_frame("Khong co Tui gam can than"));
        return;
    };

    let before_session = ctx.conn.session.clone();
    let new_val = current_val - 1;

    match attr {
        Attr::Int => ctx.conn.session.int1 = new_val,
        Attr::Atk => ctx.conn.session.atk = new_val,
        Attr::Def => ctx.conn.session.def = new_val,
        Attr::Hpx => {
            ctx.conn.session.hpx = new_val;
            ctx.conn.session.recompute_stats();
        }
        Attr::Spx => {
            ctx.conn.session.spx = new_val;
            ctx.conn.session.recompute_stats();
        }
        Attr::Agi => ctx.conn.session.agi = new_val,
    }
    ctx.conn.session.point = ctx.conn.session.point.saturating_add(1);
    let remaining_count = deduct_homdo_item_at(&mut ctx.conn.session, pouch_slot);

    let pid = ctx.conn.session.id;
    let new_point = ctx.conn.session.point;
    let hp_max = ctx.conn.session.hp_max;
    let sp_max = ctx.conn.session.sp_max;

    if let Some(pool) = ctx.env.pool {
        let db_col = attr.db_column();
        let db_result: Result<(), sqlx::Error> = async {
            let mut tx = pool.write.begin().await?;

            let sql = format!(
                "UPDATE characters SET freepoints = ?, {db_col} = ?, maxhp = ?, maxsp = ? WHERE playerid = ?"
            );
            sqlx::query(&sql)
                .bind(i64::from(new_point))
                .bind(i64::from(new_val))
                .bind(i64::from(hp_max))
                .bind(i64::from(sp_max))
                .bind(i64::from(pid))
                .execute(&mut *tx)
                .await?;

            if remaining_count == 0 {
                sqlx::query(
                    "DELETE FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                )
                .bind(i64::from(pid))
                .bind(i64::from(pouch_slot))
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE inventories SET quantity = ? WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                )
                .bind(i64::from(remaining_count))
                .bind(i64::from(pid))
                .bind(i64::from(pouch_slot))
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
            Ok(())
        }
        .await;

        if let Err(e) = db_result {
            tracing::warn!("Attribute respec transaction failed for player {pid}: {e}");
            ctx.conn.session = before_session;
            ctx.out
                .send(build_respec_ack_frame(RESPEC_SUB_ATTR, RESPEC_STATUS_FAIL));
            return;
        }
    }

    ctx.out
        .send(item_count_changed_frame(pouch_slot, remaining_count));
    ctx.out.send(build_stat_update(0x26, i32::from(new_point)));
    ctx.out
        .send(build_stat_update(attr.prop_code(), i32::from(new_val)));
    ctx.out.send(build_respec_ack_frame(
        RESPEC_SUB_ATTR,
        RESPEC_STATUS_SUCCESS,
    ));

    registry_update_if_online(&ctx.conn.session);
}
