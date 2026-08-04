//! Pet actions (Opcode 0x0F), Pet stable (Opcode 0x1F), & Pet summon/recall (Opcode 0x13) handlers.

use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::session::Conn;


/// Handle Opcode 0x0F — Pet actions (release, store, mount/unmount, rename, take, swap).
pub fn handle_pet_actions(
    conn: &mut Conn,
    sub: u8,
    payload: &[u8],
    out: &mut HandleOutcome,
) {
    if payload.is_empty() {
        return;
    }

    match sub {
        // Sub 2: Release pet
        2 => {
            let stt = payload[0];
            if let Some(pos) = conn.session.pets.iter().position(|p| p.stt == stt) {
                let pet = conn.session.pets.remove(pos);
                if conn.session.active_pet_stt == stt {
                    conn.session.active_pet_stt = 0;
                }
                let id4 = encoder::le32(conn.session.id);
                out.send(format!("F44407000F02{}{:02X}", id4, stt));
                let _ = pet;
            }
        }
        // Sub 3: Store pet to stable (slots 5..8)
        3 => {
            let stt = payload[0];
            if let Some(pos) = conn.session.pets.iter().position(|p| p.stt == stt) {
                // Find empty stable slot (5..8)
                let used_slots: Vec<u8> = conn.session.pets.iter().map(|p| p.stt).collect();
                if let Some(stable_slot) = (5..=8).find(|s| !used_slots.contains(s)) {
                    conn.session.pets[pos].stt = stable_slot;
                    let pet_id = conn.session.pets[pos].id;
                    let player_id_le = encoder::le32(conn.session.id);

                    out.send(format!("F44405001F06{:02X}0000", stt));
                    out.send(format!(
                        "F4440C000F01{}{:02X}{}01",
                        player_id_le, stt, encoder::le32(pet_id as u32)
                    ));
                    out.send("F44402001F0C");
                }
            }
        }
        // Sub 4: Mount horse
        4 => {
            if payload.len() >= 2 {
                let pet_id = encoder::u16_le(payload[0], payload[1]);
                if (18000..=19000).contains(&pet_id)
                    && conn.session.pets.iter().any(|p| p.id == pet_id)
                {
                    conn.session.horse_pet_id = pet_id;
                    let id4 = encoder::le32(conn.session.id);
                    out.send(format!(
                        "F4440E000F05{}{}{}00000000",
                        id4,
                        encoder::le16(pet_id),
                        "0000"
                    ));
                }
            }
        }
        // Sub 5: Unmount horse
        5 => {
            conn.session.horse_pet_id = 0;
            let id4 = encoder::le32(conn.session.id);
            out.send(format!("F44406000F06{}", id4));
        }
        // Sub 6: Rename pet
        6 => {
            let stt = payload[0];
            let name = &payload[1..];
            if let Some(pet) = conn.session.pets.iter_mut().find(|p| p.stt == stt) {
                pet.name = name.to_vec();
                let id4 = encoder::le32(conn.session.id);
                let name_hex = encoder::strhex(name);
                let body = format!("{}{:02X}{}", id4, stt, name_hex);
                let total_len = 2 + body.len() / 2;
                out.send(format!("F444{}0F09{}", encoder::le16(total_len as u16), body));
            }
        }
        // Sub 7: Take pet from stable
        7 => {
            let stt = payload[0];
            if conn.session.active_pet_stt == stt {
                out.send("F44402001F09");
                return;
            }
            if let Some(pos) = conn.session.pets.iter().position(|p| p.stt == stt) {
                let used_slots: Vec<u8> = conn.session.pets.iter().map(|p| p.stt).collect();
                if let Some(free_slot) = (1..=4).find(|s| !used_slots.contains(s)) {
                    conn.session.pets[pos].stt = free_slot;
                    out.send("F44402001F09");
                }
            }
        }
        // Sub 8: Swap pet positions
        8 => {
            if payload.len() >= 2 {
                let stt1 = payload[0];
                let stt2 = payload[1];
                let pos1 = conn.session.pets.iter().position(|p| p.stt == stt1);
                let pos2 = conn.session.pets.iter().position(|p| p.stt == stt2);
                if let (Some(p1), Some(p2)) = (pos1, pos2) {
                    conn.session.pets[p1].stt = stt2;
                    conn.session.pets[p2].stt = stt1;
                    conn.session.pets.swap(p1, p2);
                }
                out.send("F44402001F09F44402001F0C");
            }
        }
        _ => {}
    }
}

/// Handle Opcode 0x1F — Pet stable menu.
pub fn handle_pet_stable(
    conn: &mut Conn,
    sub: u8,
    payload: &[u8],
    out: &mut HandleOutcome,
) {
    match sub {
        3 | 7 | 8 => handle_pet_actions(conn, sub, payload, out),
        _ => {}
    }
}

/// Handle Opcode 0x13 — Pet summon / recall.
pub fn handle_pet_summon(
    conn: &mut Conn,
    sub: u8,
    payload: &[u8],
    out: &mut HandleOutcome,
) {
    match sub {
        // Sub 1: Summon pet
        1 => {
            if payload.len() >= 2 {
                let pet_id = encoder::u16_le(payload[0], payload[1]);
                if let Some(pet) = conn.session.pets.iter().find(|p| p.id == pet_id) {
                    conn.session.active_pet_stt = pet.stt;
                    out.send(format!("F44406001301{}", encoder::le32(pet_id as u32)));
                }
            }
        }
        // Sub 2: Recall pet
        2 => {
            conn.session.active_pet_stt = 0;
            out.send("F44402001302");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::session::PetState;


    #[test]
    fn test_mount_unmount_horse() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.pets.push(PetState {
            stt: 1,
            id: 18001,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        // Mount horse 18001 (0x4651) -> payload: 0x51, 0x46
        handle_pet_actions(&mut conn, 4, &[0x51, 0x46], &mut out);

        assert_eq!(conn.session.horse_pet_id, 18001);
        assert!(out.outgoing[0].contains("0F05"));

        let mut out2 = HandleOutcome::default();
        handle_pet_actions(&mut conn, 5, &[0], &mut out2);
        assert_eq!(conn.session.horse_pet_id, 0);
        assert!(out2.outgoing[0].contains("0F06"));
    }

    #[test]
    fn test_pet_summon_recall() {
        let mut conn = Conn::new();
        conn.session.pets.push(PetState {
            stt: 1,
            id: 15001,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        // Summon pet 15001 (0x3A99) -> payload: 0x99, 0x3A
        handle_pet_summon(&mut conn, 1, &[0x99, 0x3A], &mut out);

        assert_eq!(conn.session.active_pet_stt, 1);
        assert!(out.outgoing[0].contains("1301"));

        let mut out2 = HandleOutcome::default();
        handle_pet_summon(&mut conn, 2, &[], &mut out2);
        assert_eq!(conn.session.active_pet_stt, 0);
        assert_eq!(out2.outgoing[0], "F44402001302");
    }
}
