//! Inventory base (Op 0x17 subs 2, 3, 10, 11, 12) & Use items (Op 0x17 sub 15) & Reborn (sub 46).

use crate::protocol::encoder;
use crate::server::handler::{hex_of, HandleOutcome};
use crate::server::handlers::stats::build_stat_update;
use crate::server::session::{Conn, InventoryItem};

/// Dispatch Opcode 0x17 — Inventory operations.
pub fn handle_inventory(
    conn: &mut Conn,
    sub: u8,
    payload: &[u8],
    decoded: &[u8],
    out: &mut HandleOutcome,
) {
    match sub {
        // Sub 2: Pick up map drop
        2 => handle_pickup(conn, payload, out),
        // Sub 3: Drop item
        3 => handle_drop(conn, payload, out),
        // Sub 10: Move / stack item (echo raw packet back)
        10 => handle_move_stack(conn, payload, decoded, out),
        // Sub 11: Equip player item
        11 => handle_equip(conn, payload, out),
        // Sub 12: Unequip player item
        12 => handle_unequip(conn, payload, out),
        // Sub 15: Use item
        15 => handle_use_item(conn, payload, out),
        // Sub 46: Player reborn
        46 => handle_reborn(conn, out),
        _ => {}
    }
}

fn handle_pickup(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if conn.session.battle_id > 0 || payload.is_empty() {
        return;
    }
    // Dummy / mock item pick: item ID 10001 count 1
    let item = InventoryItem {
        slot: 0,
        id: 10001,
        count: 1,
        doben: 100,
        loai: 1,
        ..Default::default()
    };
    if conn.session.add_homdo_item(item).is_some() {
        out.send(conn.session.dump_homdo());
    } else {
        out.send("F44403001B0102"); // Inventory full
    }
}

fn handle_drop(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if conn.session.battle_id > 0 || payload.is_empty() {
        return;
    }
    let slot = payload[0];
    if let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == slot) {
        conn.session.homdo.remove(pos);
        out.send(format!("F44404001703{:02X}00", slot));
        out.send(conn.session.dump_homdo());
    }
}

fn handle_move_stack(
    conn: &mut Conn,
    payload: &[u8],
    decoded: &[u8],
    out: &mut HandleOutcome,
) {
    if payload.len() < 2 {
        return;
    }
    let src_slot = payload[0];
    let dst_slot = payload[1];

    let src_idx = conn.session.homdo.iter().position(|i| i.slot == src_slot);
    let dst_idx = conn.session.homdo.iter().position(|i| i.slot == dst_slot);

    match (src_idx, dst_idx) {
        (Some(s), Some(d)) => {
            // Swap items
            conn.session.homdo[s].slot = dst_slot;
            conn.session.homdo[d].slot = src_slot;
            conn.session.homdo.swap(s, d);
            out.send(hex_of(decoded));
        }
        (Some(s), None) => {
            // Move item to empty slot
            conn.session.homdo[s].slot = dst_slot;
            out.send(hex_of(decoded));
        }
        _ => {}
    }
}

fn handle_equip(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if conn.session.battle_id > 0 || payload.is_empty() {
        return;
    }
    let homdo_slot = payload[0];
    if let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == homdo_slot) {
        let item = conn.session.homdo[pos].clone();
        let loai = item.loai;

        if item.id > 0 && (1..=6).contains(&loai) {
            conn.session.homdo.remove(pos);
            // Unequip current item in that slot if exists
            if let Some(old_pos) = conn.session.trangbi.iter().position(|i| i.slot == loai) {
                let mut old_item = conn.session.trangbi.remove(old_pos);
                old_item.slot = homdo_slot;
                conn.session.homdo.push(old_item);
            }

            let mut equipped = item;
            equipped.slot = loai;
            conn.session.trangbi.push(equipped);

            out.send(format!("F44403001711{:02X}", homdo_slot));

            // Recompute bonus stats
            conn.session.recompute_stats();
            out.send(conn.session.dump_trangbi());

            // Emit stat update packets
            out.send(build_stat_update(0xCF, conn.session.hpx2 as i32));
            out.send(build_stat_update(0xD0, conn.session.spx2 as i32));
            out.send(build_stat_update(0xD2, conn.session.atk2 as i32));
            out.send(build_stat_update(0xD3, conn.session.def2 as i32));
            out.send(build_stat_update(0xD4, conn.session.int2 as i32));
            out.send(build_stat_update(0xD6, conn.session.agi2 as i32));
        }
    }
}

fn handle_unequip(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if conn.session.battle_id > 0 || payload.len() < 2 {
        return;
    }
    let trangbi_slot = payload[0];
    let homdo_slot = payload[1];

    if let Some(pos) = conn.session.trangbi.iter().position(|i| i.slot == trangbi_slot) {
        let mut item = conn.session.trangbi.remove(pos);
        item.slot = homdo_slot;
        conn.session.homdo.push(item);

        out.send(format!("F44404001710{:02X}{:02X}", trangbi_slot, homdo_slot));

        conn.session.recompute_stats();
        out.send(conn.session.dump_trangbi());
        out.send(conn.session.dump_homdo());

        out.send(build_stat_update(0xCF, conn.session.hpx2 as i32));
        out.send(build_stat_update(0xD0, conn.session.spx2 as i32));
        out.send(build_stat_update(0xD2, conn.session.atk2 as i32));
        out.send(build_stat_update(0xD3, conn.session.def2 as i32));
        out.send(build_stat_update(0xD4, conn.session.int2 as i32));
        out.send(build_stat_update(0xD6, conn.session.agi2 as i32));
    }
}

fn handle_use_item(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if payload.is_empty() {
        return;
    }
    let slot = payload[0];

    if let Some(pos) = conn.session.homdo.iter().position(|i| i.slot == slot) {
        let item = conn.session.homdo[pos].clone();
        if item.id == 0 {

            return;
        }

        // Potion / book / item effects:
        match item.id {
            // HP Potion (e.g. 10001): restores 100 HP
            10001 => {
                conn.session.hp = (conn.session.hp + 100).min(conn.session.hp_max);
                out.send(build_stat_update(0x19, conn.session.hp as i32));
            }
            // SP Potion (e.g. 10002): restores 100 SP
            10002 => {
                conn.session.sp = (conn.session.sp + 100).min(conn.session.sp_max);
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
            }
            // Skill Book (e.g. 20001): learns skill 10001 at lv 1
            20001 => {
                let skill_id = 10001u16;
                let lv = 1u8;
                if !conn.session.skills.iter().any(|(id, _)| *id == skill_id) {
                    conn.session.skills.push((skill_id, lv));
                }
                out.send(format!(
                    "F4440C0008016E01{}{}",
                    encoder::le32(lv as u32),
                    encoder::le32(skill_id as u32)
                ));
            }
            // Stat Point Book (e.g. 20002): adds 5 allocation points
            20002 => {
                conn.session.point += 5;
                out.send(build_stat_update(0x26, conn.session.point as i32));
            }
            // Gold item (e.g. 20003): adds 1000 gold
            20003 => {
                conn.session.gold += 1000;
                out.send(format!(
                    "F4440A001A04{}00000000",
                    encoder::le32(conn.session.gold)
                ));
            }
            // Doll summon (e.g. 20004): doll 101, npc 5001
            20004 => {
                let id4 = encoder::le32(conn.session.id);
                let npcid = encoder::le16(5001);
                out.send(format!(
                    "F44408000505{}{}", id4, npcid
                ));
                out.send(format!("F44404001709{:02X}{:02X}", slot, item.count.saturating_sub(1)));
                out.send("F4440200170F".to_string());
                // Consume item
                if item.count > 1 {
                    conn.session.homdo[pos].count -= 1;
                } else {
                    conn.session.homdo.remove(pos);
                }
                return;
            }
            _ => {
                // Generic item use default
            }
        }

        // Decrement item count or remove
        let rem = item.count.saturating_sub(1);
        if rem > 0 {
            conn.session.homdo[pos].count = rem;
        } else {
            conn.session.homdo.remove(pos);
        }

        // Standard end feedback
        out.send(format!("F44404001709{:02X}{:02X}", slot, rem));
        out.send("F4440200170F".to_string());
    }
}

fn handle_reborn(conn: &mut Conn, out: &mut HandleOutcome) {
    // Requires no equipment in trangbi slots 1..6
    if !conn.session.trangbi.is_empty() {
        return;
    }
    conn.session.reborn += 1;
    conn.session.level = 1;
    conn.session.skills.clear();
    conn.session.recompute_stats();

    out.send("F44402002C01");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equip_and_unequip() {
        let mut conn = Conn::new();
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 12001,
            count: 1,
            doben: 100,
            loai: 1,
            atk1: 15,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let decoded = vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x0B, 0x01];
        handle_inventory(&mut conn, 11, &[1], &decoded, &mut out);

        assert_eq!(conn.session.trangbi.len(), 1);
        assert_eq!(conn.session.trangbi[0].id, 12001);
        assert_eq!(conn.session.atk2, 15);
        assert!(out.outgoing.iter().any(|f| f.contains("171101")));

        // Unequip
        let mut out2 = HandleOutcome::default();
        let decoded2 = vec![0xF4, 0x44, 0x04, 0x00, 0x17, 0x0C, 0x01, 0x02];
        handle_inventory(&mut conn, 12, &[1, 2], &decoded2, &mut out2);

        assert_eq!(conn.session.trangbi.len(), 0);
        assert_eq!(conn.session.atk2, 0);
        assert!(out2.outgoing.iter().any(|f| f.contains("17100102")));
    }

    #[test]
    fn test_use_hp_potion() {
        let mut conn = Conn::new();
        conn.session.hp_max = 200;
        conn.session.hp = 50;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 10001,
            count: 2,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let decoded = vec![0xF4, 0x44, 0x03, 0x00, 0x17, 0x0F, 0x01];
        handle_inventory(&mut conn, 15, &[1], &decoded, &mut out);

        assert_eq!(conn.session.hp, 150);
        assert_eq!(conn.session.homdo[0].count, 1);
        assert!(out.outgoing.iter().any(|f| f.contains("17090101")));
        assert!(out.outgoing.iter().any(|f| f.contains("170F")));
    }
}
