//! Use item handler (Op 0x17 sub 15) — C# `Update_H17` case 15.
//!
//! The toy hard-coded switch is replaced with a data-driven dispatch over
//! `GameData.items` mirroring the C# branch order: warp items, add-pet,
//! leader-only sleep, skill/point books, party-buff/special frames, and the
//! generic `Hp*Sp*Fai1` potion path (player or pet, gated by the use-type
//! byte). Every consumable ends with the standard `1709`+used-count+`170F`
//! frame pair unless the branch returns early (C# §2.3.14).

use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::persist;
use crate::server::session::Conn;

/// Warp items: item id → (map_id, x, y). Source: C# case-15 warp table.
/// Item 46016 warps to the save map (here: the current map).
fn warp_target(id: u16, current_map: u16) -> Option<(u16, u16, u16)> {
    Some(match id {
        46016 => (current_map, 410, 510),
        46022 => (12403, 442, 375),
        46027 => (12003, 530, 510),
        54002 => (12901, 202, 1175),
        46084 => (21011, 222, 455),
        46055 => (18990, 602, 235),
        46105 => (26011, 502, 775),
        46085 => (23241, 402, 515),
        46054 => (14241, 402, 495),
        46086 => (25241, 662, 655),
        46103 => (20001, 762, 615),
        46102 => (19241, 462, 435),
        46052 => (15025, 462, 375),
        45005 => (54811, 500, 500),
        46104 => (24262, 362, 395),
        54001 => (54812, 500, 500),
        46051 => (15002, 442, 335),
        46019 => (15000, 522, 535),
        46025 => (15001, 562, 515),
        46087 => (15012, 222, 295),
        46023 => (54901, 1722, 835),
        45822 => (54004, 426, 635),
        45003 => (54826, 402, 375),
        46070 => (59401, 402, 775),
        _ => return None,
    })
}

/// Skill books: item id → learned skill id (C# case-15 skill-book family).
fn skill_book_target(id: u16) -> Option<u16> {
    Some(match id {
        46230 => 10016,
        46231 => 11016,
        46232 => 12016,
        46233 => 13015,
        46246 => 14038,
        _ => return None,
    })
}

/// Consume `count` of the item at `slot` and emit the standard end feedback
/// `F44404001709` + slot + used-count + `F4440200170F` (C# `HomdoUseHPSPFAI`).
/// Returns true when the item was consumed.
async fn consume(
    conn: &mut Conn,
    slot: u8,
    count: u16,
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
) -> bool {
    let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == slot) else {
        return false;
    };
    if conn.session.homdo[pos].id == 0 {
        return false;
    }
    let used = count.min(u16::from(conn.session.homdo[pos].count));
    if used == 0 {
        return false;
    }
    let rem = conn.session.homdo[pos].count - used as u8;
    if rem > 0 {
        conn.session.homdo[pos].count = rem;
    } else {
        conn.session.homdo.remove(pos);
    }
    match conn.session.homdo.iter().find(|i| i.slot == slot) {
        Some(kept) => persist::upsert_item(pool, conn.session.id, "homdo", kept).await,
        None => {
            let empty = crate::server::session::InventoryItem {
                slot,
                ..Default::default()
            };
            persist::upsert_item(pool, conn.session.id, "homdo", &empty).await;
        }
    }
    out.send(format!("F44404001709{:02X}{:02X}", slot, used));
    out.send("F4440200170F".to_string());
    true
}

/// Op 0x17 sub 15 — use item at `slot`, `count_used` times.
pub async fn use_item(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&sqlx::MySqlPool>,
    data: &GameData,
) {
    if payload.is_empty() {
        return;
    }
    let slot = payload[0];
    let count = if payload.len() >= 2 && payload[1] > 0 {
        payload[1] as u16
    } else {
        1
    };
    let use_type = payload.get(2).copied().unwrap_or(0);
    if count == 0 {
        return;
    }

    let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == slot) else {
        return;
    };
    let item = conn.session.homdo[pos].clone();
    let id = item.id;
    if id == 0 || item.count == 0 || u16::from(item.count) < count {
        return; // C# case-15 gate: `iD12 > 0 && num61 > 0`
    }

    // --- Warp items (C# flag2 table): consume + warp, no 170F tail. ---
    if let Some((map_id, x, y)) = warp_target(id, conn.session.map_id) {
        conn.session.map_id = map_id;
        conn.session.map_x = x;
        conn.session.map_y = y;
        // C# sends `1709`+used-count, then `Data.Warped` (0x0C warp confirm).
        out.send(format!("F44404001709{:02X}{:02X}", slot, count));
        let frame = format!(
            "F4440D000C{}{}{}{}00",
            encoder::le32(conn.session.id),
            encoder::le16(map_id),
            encoder::le16(x),
            encoder::le16(y)
        );
        out.send(frame);
        consume(conn, slot, count, out, pool).await;
        return;
    }

    // --- Add-pet items (C# `_AddPet > 10000`). ---
    let add_pet = data
        .items
        .get(&i64::from(id))
        .map(|i| i.add_pet)
        .unwrap_or(0);
    if add_pet > 10000 {
        let pet_id = add_pet as u16;
        if conn.session.pets.iter().any(|p| p.id == pet_id) {
            // Already owned: C# sends a red message and breaks (no consume).
            out.send(spawn_sys_msg("Ban da co pet nay roi"));
            return;
        }
        if conn.session.pets.len() < 4 {
            let stt = (1..=4)
                .find(|s| !conn.session.pets.iter().any(|p| p.stt == *s))
                .unwrap_or(1);
            conn.session.pets.push(crate::server::session::PetState {
                stt,
                id: pet_id,
                level: 1,
                hp: 100,
                hp_max: 100,
                sp: 100,
                sp_max: 100,
                ..Default::default()
            });
            consume(conn, slot, count, out, pool).await;
            return;
        }
        // Pet box full: C# red message, no consume.
        out.send(spawn_sys_msg("Pet box full"));
        return;
    }

    // --- Leader-only sleep item (C# 46167). ---
    if id == 46167 {
        let leader_ok =
            conn.session.id == conn.session.id_leader || conn.session.id_leader == 0;
        if leader_ok {
            conn.session.hp = conn.session.hp_max;
            conn.session.sp = conn.session.sp_max;
            out.send("F44402001F0A".to_string());
            out.send(crate::server::handlers::stats::build_stat_update(
                0x19, conn.session.hp as i32,
            ));
            out.send(crate::server::handlers::stats::build_stat_update(
                0x1A, conn.session.sp as i32,
            ));
            consume(conn, slot, count, out, pool).await;
        }
        return;
    }

    // --- Skill books (C# 46230 family): learn + learn packet, no 1709/170F. ---
    if let Some(skill_id) = skill_book_target(id) {
        if !conn.session.skills.iter().any(|(s, _)| *s == skill_id) {
            let lv = 1u8;
            conn.session.skills.push((skill_id, lv));
            // C# `F4440C0008016E01` + le32(lv) + le32(skillid).
            let body = format!("6E01{}{}", encoder::le32(lv as u32), encoder::le32(skill_id as u32));
            out.send(crate::protocol::frame("0801", &body));
            consume(conn, slot, count, out, pool).await;
        } else {
            out.send(spawn_sys_msg("Ban da co ky nang nay roi"));
        }
        return;
    }

    // --- Point / SkillPoint books (C# 50010 / 50011). Not consumed (C# quirk). ---
    match id {
        50010 => {
            conn.session.point += 1;
            persist::update_player(pool, conn.session.id, "Point", i64::from(conn.session.point)).await;
            out.send(crate::server::handlers::stats::build_stat_update(0x26, conn.session.point as i32));
            // C# tail: standard end feedback, item kept.
            out.send(format!("F44404001709{:02X}{:02X}", slot, count));
            out.send("F4440200170F".to_string());
            return;
        }
        50011 => {
            conn.session.skill_point += 1;
            persist::update_player(pool, conn.session.id, "SkillPoint", i64::from(conn.session.skill_point)).await;
            out.send(crate::server::handlers::stats::build_stat_update(0x25, conn.session.skill_point as i32));
            out.send(format!("F44404001709{:02X}{:02X}", slot, count));
            out.send("F4440200170F".to_string());
            return;
        }
        _ => {}
    }

    // --- Party-buff / special frames (C# 46092 / 46041 / 46093). ---
    match id {
        46092 => {
            out.send("F44404000B0702FF".to_string());
            consume(conn, slot, count, out, pool).await;
            return;
        }
        46041 | 46093 => {
            out.send("F44404000B09FF01".to_string());
            consume(conn, slot, count, out, pool).await;
            return;
        }
        _ => {}
    }

    // --- Generic potion path (C# default: `Hp*Sp*Fai1` × count). ---
    let info = data.items.get(&i64::from(id));
    let hp_amt = info.map(|i| i.hp.saturating_mul(i64::from(count))).unwrap_or(0);
    let sp_amt = info.map(|i| i.sp.saturating_mul(i64::from(count))).unwrap_or(0);
    let fai_amt = info.map(|i| i.fai1.saturating_mul(i64::from(count))).unwrap_or(0);
    if hp_amt == 0 && sp_amt == 0 && fai_amt == 0 {
        // Unknown/statless item: standard consume + end feedback (ticket #12).
        consume(conn, slot, count, out, pool).await;
        return;
    }
    if use_type == 0 {
        // Restore the player.
        let mut restored = false;
        if hp_amt > 0 && conn.session.hp < conn.session.hp_max {
            let new_hp = (i64::from(conn.session.hp) + hp_amt).min(i64::from(conn.session.hp_max)) as u16;
            conn.session.hp = new_hp;
            persist::update_player(pool, conn.session.id, "Hp", i64::from(new_hp)).await;
            out.send(crate::server::handlers::stats::build_stat_update(0x19, new_hp as i32));
            restored = true;
        }
        if sp_amt > 0 && conn.session.sp < conn.session.sp_max {
            let new_sp = (i64::from(conn.session.sp) + sp_amt).min(i64::from(conn.session.sp_max)) as u16;
            conn.session.sp = new_sp;
            persist::update_player(pool, conn.session.id, "Sp", i64::from(new_sp)).await;
            out.send(crate::server::handlers::stats::build_stat_update(0x1A, new_sp as i32));
            restored = true;
        }
        if restored {
            consume(conn, slot, count, out, pool).await;
        } else {
            // Already full: C# sends the stat packet but consumes nothing; the
            // standard end feedback still closes the use (ticket #12).
            out.send(format!("F44404001709{:02X}{:02X}", slot, count));
            out.send("F4440200170F".to_string());
        }
        return;
    }
    if (1..=4).contains(&use_type) {
        // Restore the pet in slot `use_type` (C# case 1..4).
        let stt = use_type;
        let Some(pet_idx) = conn.session.pets.iter().position(|p| p.stt == stt) else {
            return;
        };
        let mut restored = false;
        if hp_amt > 0 && conn.session.pets[pet_idx].hp < conn.session.pets[pet_idx].hp_max {
            conn.session.pets[pet_idx].hp = (i64::from(conn.session.pets[pet_idx].hp) + hp_amt)
                .min(i64::from(conn.session.pets[pet_idx].hp_max)) as u16;
            restored = true;
        }
        if sp_amt > 0 && conn.session.pets[pet_idx].sp < conn.session.pets[pet_idx].sp_max {
            conn.session.pets[pet_idx].sp = (i64::from(conn.session.pets[pet_idx].sp) + sp_amt)
                .min(i64::from(conn.session.pets[pet_idx].sp_max)) as u16;
            restored = true;
        }
        if fai_amt > 0 && conn.session.pets[pet_idx].fai < 100 {
            conn.session.pets[pet_idx].fai = (i64::from(conn.session.pets[pet_idx].fai) + fai_amt)
                .min(100) as u16;
            restored = true;
        }
        if restored {
            consume(conn, slot, count, out, pool).await;
        }
    }
}

/// Red-message banner (op 0x02 sub 0x0B) for use-item feedback.
fn spawn_sys_msg(msg: &str) -> String {
    let mut body = String::from("00000000");
    body.push_str(&encoder::strhex(msg.as_bytes()));
    crate::protocol::frame("020B", &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::session::InventoryItem;

    fn item(id: u16, count: u8) -> InventoryItem {
        InventoryItem {
            slot: 1,
            id,
            count,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn potion_restores_hp_and_ends_standard() {
        let mut conn = Conn::new();
        conn.session.hp = 50;
        conn.session.hp_max = 200;
        conn.session.sp = 30;
        conn.session.sp_max = 200;
        conn.session.homdo.push(item(30001, 5));
        let mut data = GameData::default();
        data.items.insert(30001, crate::data::tables::Item { id: 30001, hp: 100, sp: 50, ..Default::default() });
        let mut out = HandleOutcome::default();
        // slot 1, count 2, use_type 0
        use_item(&mut conn, &[1, 2, 0], &mut out, None, &data).await;

        assert_eq!(conn.session.hp, 200); // 50 + 200 capped
        assert_eq!(conn.session.sp, 130); // 30 + 100
        assert_eq!(conn.session.homdo[0].count, 3); // 5 - 2
        assert_eq!(
            out.outgoing,
            vec![
                "F4440C0008011901C800000000000000".to_string(), // Hp -> 200
                "F4440C0008011A018200000000000000".to_string(), // Sp -> 130
                "F444040017090102".to_string(),                 // 1709 slot 1, used 2
                "F4440200170F".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn warp_item_moves_map_and_consumes() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        conn.session.homdo.push(item(46022, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        use_item(&mut conn, &[1, 1], &mut out, None, &data).await;

        assert_eq!(conn.session.map_id, 12403);
        assert_eq!(conn.session.map_x, 442);
        assert_eq!(conn.session.map_y, 375);
        assert!(conn.session.homdo.is_empty(), "warp item consumed");
        assert!(out.outgoing.iter().any(|f| f.contains("17090101")));
        assert!(out.outgoing.iter().any(|f| f.starts_with("F4440D000C")));
    }

    #[tokio::test]
    async fn add_pet_item_gives_pet() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(46001, 1));
        let mut data = GameData::default();
        data.items.insert(46001, crate::data::tables::Item { id: 46001, add_pet: 10001, ..Default::default() });
        let mut out = HandleOutcome::default();
        use_item(&mut conn, &[1, 1], &mut out, None, &data).await;

        assert_eq!(conn.session.pets.len(), 1);
        assert_eq!(conn.session.pets[0].id, 10001);
        assert!(conn.session.homdo.is_empty(), "pet item consumed");
    }

    #[tokio::test]
    async fn point_book_adds_point_and_keeps_item() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(50010, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        use_item(&mut conn, &[1, 1], &mut out, None, &data).await;

        assert_eq!(conn.session.point, 1);
        assert_eq!(conn.session.homdo.len(), 1, "point book is not consumed (C# quirk)");
        assert!(out.outgoing.iter().any(|f| f.starts_with("F4440C0008012601")));
    }

    #[tokio::test]
    async fn unknown_item_consumes_and_ends_standard() {
        let mut conn = Conn::new();
        conn.session.homdo.push(item(40001, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        use_item(&mut conn, &[1, 1], &mut out, None, &data).await;
        assert!(conn.session.homdo.is_empty());
        assert!(out.outgoing.iter().any(|f| f.contains("17090101")));
        assert!(out.outgoing.iter().any(|f| f.contains("170F")));
    }

    #[tokio::test]
    async fn item_missing_data_and_zero_effects_still_consumes() {
        // An item id with zero Hp/Sp/Fai1 still consumes (ticket #12 wording).
        let mut conn = Conn::new();
        conn.session.homdo.push(item(40002, 1));
        let data = GameData::default();
        let mut out = HandleOutcome::default();
        use_item(&mut conn, &[1, 1], &mut out, None, &data).await;
        assert!(conn.session.homdo.is_empty());
        assert!(!out.outgoing.is_empty());
    }
}