//! Crafting and Socketing network packet handlers (Opcode 0x17 Sub 0x39, 0x17 Sub 0x3A/14, 0x23 Sub 0x03).
//!
//! Enforces strict Homdo parity (slots 1..=25), Probe-First free-slot checking,
//! Two-Phase State Lock, and atomic SQLite persistence.

use crate::crafting::basic::algorithm::{execute_compound, DelphiRng};
use crate::crafting::basic::validator::CraftInputSlot;
use crate::crafting::error::CraftError;
use crate::crafting::socketing::feather::socket_feather;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::inventory::HOMDO_SLOTS;
use crate::server::session::{lock_player_operations, registry_update_if_online};
use crate::server::spawn::sys_msg_frame;

/// SFX trigger frame for WA0014.wav blacksmith hammering.
pub const SOUND_BLACKSMITH: &str = "F4440200170F";

/// Handles Opcode 0x17 Subcode 0x39 — Compound items from `form_compound`.
pub async fn handle_compound(ctx: &mut OpcodeCtx<'_>) {
    let payload = ctx.payload;
    if payload.len() < 2 {
        ctx.out.send(sys_msg_frame("Hợp thành cần ít nhất 2 vật phẩm!"));
        return;
    }

    // Extract up to 3 candidate pairs (slot, qty)
    let mut pairs = Vec::with_capacity(3);
    for chunk in payload.chunks_exact(2).take(3) {
        let slot = chunk[0];
        let qty = chunk[1];
        if slot > 0 && qty > 0 {
            pairs.push((slot, qty));
        }
    }

    // At least 2 valid items required (US 5)
    if pairs.len() < 2 {
        ctx.out.send(sys_msg_frame("Hợp thành cần ít nhất 2 vật phẩm!"));
        return;
    }

    // 1. Strict Homdo constraint (slots 1..=25)
    for (slot, _) in &pairs {
        if !(1..=HOMDO_SLOTS).contains(slot) {
            ctx.out.send(sys_msg_frame("Vị trí vật phẩm không hợp lệ!"));
            return;
        }
    }

    // No duplicate slots
    let mut seen = std::collections::HashSet::new();
    for (slot, _) in &pairs {
        if !seen.insert(*slot) {
            ctx.out.send(sys_msg_frame("Không thể chọn cùng một ô vật phẩm!"));
            return;
        }
    }

    // Two-Phase State Lock: lock player RAM operations before inspecting or mutating state
    let player_id = ctx.conn.session.id;
    let _locks = lock_player_operations([player_id]).await;

    // 2. Probe-First: check free slots in homdo
    // If bag has 25 occupied items, reject immediately!
    let occupied_count = ctx
        .conn
        .session
        .homdo
        .iter()
        .filter(|i| i.id > 0)
        .count();
    if occupied_count >= HOMDO_SLOTS as usize {
        ctx.out.send(sys_msg_frame("Túi đồ đã đầy!"));
        return;
    }

    // 3. Verify player possesses sufficient items in the specified slots
    let mut items = Vec::with_capacity(pairs.len());
    for &(slot, qty) in &pairs {
        let item = ctx
            .conn
            .session
            .homdo
            .iter()
            .find(|i| i.slot == slot && i.id > 0 && i.count >= qty)
            .cloned();
        match item {
            Some(it) => items.push((slot, qty, it)),
            None => {
                ctx.out.send(sys_msg_frame("Số lượng vật phẩm trong túi không đủ!"));
                return;
            }
        }
    }

    // Build CraftInputSlot array for the domain engine
    let mut inputs = [CraftInputSlot::default(); 3];
    for (idx, &(slot, qty, ref it)) in items.iter().enumerate() {
        inputs[idx] = CraftInputSlot::new(slot, it.id, qty);
    }

    // Seed RNG from timestamp
    let seed = (chrono::Utc::now().timestamp_millis() & 0x7FFF_FFFF) as i32;
    let mut rng = DelphiRng::new(seed);

    // 4. Run the pure domain compound algorithm
    let outcome = match execute_compound(&inputs, ctx.data, &mut rng) {
        Ok(out) => out,
        Err(CraftError::NeedAtLeastTwoItems) => {
            ctx.out.send(sys_msg_frame("Hợp thành cần ít nhất 2 vật phẩm!"));
            return;
        }
        Err(CraftError::InvalidItemForCompound) => {
            ctx.out.send(sys_msg_frame("Có lẫn vật phẩm không thể hợp thành!"));
            return;
        }
        Err(CraftError::QuantityExceeded) => {
            ctx.out.send(sys_msg_frame("Số lượng một ô không được vượt quá 50!"));
            return;
        }
        Err(CraftError::ForbiddenResultKind(_)) => {
            ctx.out.send(sys_msg_frame("Lỗi dữ liệu hợp thành không hợp lệ!"));
            return;
        }
        Err(CraftError::NoMatchingItem) => {
            ctx.out.send(sys_msg_frame("Cơ sở dữ liệu không có vật phẩm phù hợp!"));
            return;
        }
        Err(_) => {
            ctx.out.send(sys_msg_frame("Hợp thành thất bại!"));
            return;
        }
    };

    // Determine ingredient deductions
    let mut deductions: Vec<(u8, u8)> = Vec::with_capacity(items.len());
    for &(slot, qty, ref it) in &items {
        deductions.push((slot, it.count.saturating_sub(qty)));
    }

    // Find the first free slot in 1..=25 for the created item
    let mut active_slots = std::collections::HashSet::new();
    for item in &ctx.conn.session.homdo {
        if item.id > 0 {
            // If this item will be completely consumed, its slot may not necessarily be reused
            // immediately in the same step unless needed, but to be strictly safe and deterministic:
            let is_exhausted = deductions.iter().any(|(s, rem)| *s == item.slot && *rem == 0);
            if !is_exhausted {
                active_slots.insert(item.slot);
            }
        }
    }

    let free_slot = (1..=HOMDO_SLOTS)
        .find(|s| !active_slots.contains(s))
        .unwrap_or(1);

    // Create the new item
    let mut new_item = crate::server::inventory::from_template(ctx.data, outcome.item_id, outcome.qty);
    new_item.slot = free_slot;

    // 6. Atomic SQLite transaction (if DB pool available)
    if let Some(pool) = ctx.env.pool {
        let char_id = if ctx.conn.session.db_character_id > 0 {
            ctx.conn.session.db_character_id
        } else {
            i64::from(player_id)
        };

        if let Ok(mut tx) = pool.write.begin().await {
            // Apply deductions
            for (s, rem) in &deductions {
                if *rem == 0 {
                    let _ = sqlx::query(
                        "DELETE FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                    )
                    .bind(char_id)
                    .bind(i64::from(*s))
                    .execute(&mut *tx)
                    .await;
                } else {
                    let _ = sqlx::query(
                        "UPDATE inventories SET quantity = ? WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                    )
                    .bind(i64::from(*rem))
                    .bind(char_id)
                    .bind(i64::from(*s))
                    .execute(&mut *tx)
                    .await;
                }
            }

            // Insert new item
            let _ = crate::server::inventory::upsert_inventory_slot(
                &mut *tx,
                char_id,
                1,
                &new_item,
            )
            .await;

            let _ = tx.commit().await;
        }
    }

    // 7. Update RAM Session
    for (s, rem) in &deductions {
        if *rem == 0 {
            ctx.conn.session.homdo.retain(|i| i.slot != *s);
        } else if let Some(item) = ctx.conn.session.homdo.iter_mut().find(|i| i.slot == *s) {
            item.count = *rem;
        }
    }
    // Remove any stale item in free_slot and place new_item
    ctx.conn.session.homdo.retain(|i| i.slot != free_slot);
    ctx.conn.session.homdo.push(new_item.clone());

    registry_update_if_online(&ctx.conn.session);

    // 8. Emit wire packets
    // SFX WA0014
    ctx.out.send(SOUND_BLACKSMITH.to_string());

    // Deduction packets (17 09 <slot> <count>)
    for (s, rem) in deductions {
        ctx.out.send(crate::server::inventory::item_count_changed_frame(s, rem));
    }

    // New item packet (17 06)
    ctx.out.send(crate::server::inventory::item_added_frame(&new_item));

    // Toast notification
    let item_name = ctx
        .data
        .item_defs
        .get(&new_item.id)
        .map(|d| d.name.as_str())
        .unwrap_or("Vật phẩm mới");
    ctx.out.send(sys_msg_frame(&format!("Hợp thành thành công: {}", item_name)));
}

/// Handles Opcode 0x23 Subcode 0x03 — TFT_SelectFeatherForm feather choice.
pub async fn handle_select_feather(ctx: &mut OpcodeCtx<'_>) {
    let payload = ctx.payload;
    if payload.is_empty() {
        ctx.out.send(sys_msg_frame("Hãy chọn lựa một trong các loại bảo vật"));
        return;
    }

    let (elem_choice, _sub_choice) = if payload.len() <= 4 {
        let elem = payload[0];
        let sub = payload.get(1).copied().unwrap_or(0);
        (elem, sub)
    } else {
        // Try parsing 2 len-prefixed strings
        let mut rest = payload;
        let mut strings_found = 0;
        let mut ok = true;
        for _ in 0..2 {
            if rest.is_empty() {
                ok = false;
                break;
            }
            let len = rest[0] as usize;
            rest = &rest[1..];
            if len > rest.len() {
                ok = false;
                break;
            }
            rest = &rest[len..];
            strings_found += 1;
        }
        if ok && strings_found == 2 && !rest.is_empty() {
            let elem = rest[0];
            let sub = rest.get(1).copied().unwrap_or(0);
            (elem, sub)
        } else {
            let elem = payload[0];
            let sub = payload.get(1).copied().unwrap_or(0);
            (elem, sub)
        }
    };

    // Validate choice
    if elem_choice == 0 {
        ctx.out.send(sys_msg_frame("Hãy chọn lựa một trong các loại bảo vật"));
        return;
    }
    if !(1..=4).contains(&elem_choice) {
        ctx.out.send(sys_msg_frame("Lựa chọn bảo vật không hợp lệ!"));
        return;
    }

    // Two-Phase State Lock: lock player operations before checking or updating homdo
    let player_id = ctx.conn.session.id;
    let _locks = lock_player_operations([player_id]).await;

    // Check homdo space
    let occupied = ctx
        .conn
        .session
        .homdo
        .iter()
        .filter(|i| i.id > 0)
        .count();
    if occupied >= HOMDO_SLOTS as usize {
        ctx.out.send(sys_msg_frame("Túi đồ đã đầy!"));
        return;
    }

    // Map choice to Feather ID
    let feather_id: u16 = match elem_choice {
        1 => 24021, // Earth
        2 => 24022, // Water
        3 => 24023, // Fire
        4 => 24024, // Wind
        _ => return,
    };

    // Find free slot
    let free_slot = (1..=HOMDO_SLOTS)
        .find(|s| !ctx.conn.session.homdo.iter().any(|i| i.slot == *s && i.id > 0))
        .unwrap_or(1);

    let mut item = crate::server::inventory::from_template(ctx.data, feather_id, 1);
    item.slot = free_slot;

    // Atomic SQLite insert
    if let Some(pool) = ctx.env.pool {
        let char_id = if ctx.conn.session.db_character_id > 0 {
            ctx.conn.session.db_character_id
        } else {
            i64::from(player_id)
        };

        if let Ok(mut tx) = pool.write.begin().await {
            let _ = crate::server::inventory::upsert_inventory_slot(
                &mut *tx,
                char_id,
                1,
                &item,
            )
            .await;
            let _ = tx.commit().await;
        }
    }

    // Update RAM
    ctx.conn.session.homdo.retain(|i| i.slot != free_slot);
    ctx.conn.session.homdo.push(item.clone());
    registry_update_if_online(&ctx.conn.session);

    // Emit frames
    ctx.out.send(crate::server::inventory::item_added_frame(&item));
    ctx.out.send(SOUND_BLACKSMITH.to_string());

    let item_name = ctx
        .data
        .item_defs
        .get(&feather_id)
        .map(|d| d.name.as_str())
        .unwrap_or("Lông vũ");
    ctx.out.send(sys_msg_frame(&format!("Nhận được bảo vật: {}", item_name)));
}

/// Handles Opcode 0x17 Subcode 0x3A / 14 — Socketing feather into equipment.
pub async fn handle_socket_feather(ctx: &mut OpcodeCtx<'_>) {
    let payload = ctx.payload;
    if payload.len() < 2 {
        ctx.out.send(sys_msg_frame("Vị trí trang bị hoặc lông không hợp lệ!"));
        return;
    }

    let equip_slot = payload[0];
    let feather_slot = payload[1];

    if !(1..=HOMDO_SLOTS).contains(&equip_slot) || !(1..=HOMDO_SLOTS).contains(&feather_slot) {
        ctx.out.send(sys_msg_frame("Vị trí trang bị hoặc lông không hợp lệ!"));
        return;
    }
    if equip_slot == feather_slot {
        ctx.out.send(sys_msg_frame("Không thể chọn cùng một ô trang bị và lông!"));
        return;
    }

    // Two-Phase State Lock: lock player operations before reading or updating homdo
    let player_id = ctx.conn.session.id;
    let _locks = lock_player_operations([player_id]).await;

    // Find items in homdo
    let equip_item = ctx
        .conn
        .session
        .homdo
        .iter()
        .find(|i| i.slot == equip_slot && i.id > 0)
        .cloned();
    let feather_item = ctx
        .conn
        .session
        .homdo
        .iter()
        .find(|i| i.slot == feather_slot && i.id > 0 && i.count >= 1)
        .cloned();

    let (Some(mut equip), Some(feather)) = (equip_item, feather_item) else {
        ctx.out.send(sys_msg_frame("Không tìm thấy trang bị hoặc lông vũ!"));
        return;
    };

    let equip_kind = ctx
        .data
        .item_defs
        .get(&equip.id)
        .map(|d| d.kind)
        .unwrap_or(0);

    // Roll RNG using deterministic Delphi PRNG (0..99)
    let seed = (chrono::Utc::now().timestamp_millis() & 0x7FFF_FFFF) as i32;
    let mut rng = DelphiRng::new(seed);
    let roll = rng.random_int(100) as u32;

    let outcome = match socket_feather(&mut equip, equip_kind, feather.id, roll) {
        Ok(out) => out,
        Err(CraftError::InvalidEquipmentType) => {
            ctx.out.send(sys_msg_frame("Vật phẩm không thể khảm lông!"));
            return;
        }
        Err(CraftError::InvalidFeather) => {
            ctx.out.send(sys_msg_frame("Lông vũ không hợp lệ!"));
            return;
        }
        Err(CraftError::InvalidFeatherLevel) => {
            ctx.out.send(sys_msg_frame("Cấp độ lông vũ không phù hợp!"));
            return;
        }
        Err(CraftError::EquipmentMaxLevel) => {
            ctx.out.send(sys_msg_frame("Trang bị đã đạt cấp cường hóa tối đa (+10)!"));
            return;
        }
        Err(_) => {
            ctx.out.send(sys_msg_frame("Khảm lông thất bại!"));
            return;
        }
    };

    let feather_rem = feather.count.saturating_sub(1);

    // Atomic SQLite update
    if let Some(pool) = ctx.env.pool {
        let char_id = if ctx.conn.session.db_character_id > 0 {
            ctx.conn.session.db_character_id
        } else {
            i64::from(player_id)
        };

        if let Ok(mut tx) = pool.write.begin().await {
            // Update equipment element and elementvalue
            let _ = sqlx::query(
                "UPDATE inventories SET element = ?, elementvalue = ? WHERE playerid = ? AND storagetype = 1 AND slot = ?",
            )
            .bind(i64::from(equip.long_val))
            .bind(i64::from(equip.giatri_long))
            .bind(char_id)
            .bind(i64::from(equip_slot))
            .execute(&mut *tx)
            .await;

            // Deduct feather
            if feather_rem == 0 {
                let _ = sqlx::query(
                    "DELETE FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                )
                .bind(char_id)
                .bind(i64::from(feather_slot))
                .execute(&mut *tx)
                .await;
            } else {
                let _ = sqlx::query(
                    "UPDATE inventories SET quantity = ? WHERE playerid = ? AND storagetype = 1 AND slot = ?",
                )
                .bind(i64::from(feather_rem))
                .bind(char_id)
                .bind(i64::from(feather_slot))
                .execute(&mut *tx)
                .await;
            }

            let _ = tx.commit().await;
        }
    }

    // Update RAM
    if let Some(e) = ctx.conn.session.homdo.iter_mut().find(|i| i.slot == equip_slot) {
        e.long_val = equip.long_val;
        e.giatri_long = equip.giatri_long;
    }
    if feather_rem == 0 {
        ctx.conn.session.homdo.retain(|i| i.slot != feather_slot);
    } else if let Some(f) = ctx.conn.session.homdo.iter_mut().find(|i| i.slot == feather_slot) {
        f.count = feather_rem;
    }
    registry_update_if_online(&ctx.conn.session);

    // Emit frames
    ctx.out.send(SOUND_BLACKSMITH.to_string());
    ctx.out.send(crate::server::inventory::item_count_changed_frame(feather_slot, feather_rem));

    // Update equipment frame
    ctx.out.send(crate::server::inventory::item_added_frame(&equip));

    if outcome.success {
        if outcome.is_element_switched {
            ctx.out.send(sys_msg_frame("Chuyển đổi hệ nguyên tố trang bị thành công!"));
        } else {
            ctx.out.send(sys_msg_frame(&format!(
                "Cường hóa khảm lông thành công (+{})!",
                outcome.new_element_value
            )));
        }
    } else {
        ctx.out.send(sys_msg_frame(&format!(
            "Cường hóa thất bại! Cấp thuộc tính hiện tại: +{}",
            outcome.new_element_value
        )));
    }
}
