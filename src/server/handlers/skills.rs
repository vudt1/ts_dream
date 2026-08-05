//! Learn / upgrade skills (Opcode 0x1C) & Pet Reborn (Opcode 0x2C) handlers.

use crate::protocol::encoder;
use crate::server::handler::{HandleOutcome, OpcodeCtx};
use crate::server::session::Conn;

/// Dispatch Opcode 0x1C — Learn / upgrade skills.
pub fn handle_skills(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Player skill learn / upgrade
        1 => handle_player_skill_learn(conn, payload, out),
        // Sub 2: Pet skill upgrade
        2 => handle_pet_skill_upgrade(conn, payload, out),
        _ => {}
    }
}

fn handle_player_skill_learn(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if payload.is_empty() {
        return;
    }

    let mut idx = 0;
    let mut count = 0u32;

    while idx + 2 < payload.len() {
        let skill_id = encoder::u16_le(payload[idx], payload[idx + 1]);
        let target_lv = payload[idx + 2];
        idx += 3;

        if skill_id == 0 {
            continue;
        }

        // Check or consume skill points
        if conn.session.skill_point > 0 {
            conn.session.skill_point = conn.session.skill_point.saturating_sub(1);
        }

        if let Some(existing) = conn.session.skills.iter_mut().find(|(id, _)| *id == skill_id) {
            existing.1 = target_lv;
        } else {
            conn.session.skills.push((skill_id, target_lv));
        }

        out.send(format!(
            "F4440C0008016E01{}{}",
            encoder::le32(target_lv as u32),
            encoder::le32(skill_id as u32)
        ));
        count += 1;
    }

    if count > 0 {
        out.send(format!(
            "F4440C0008012501{}00000000",
            encoder::le32(conn.session.skill_point as u32)
        ));
    }
}

fn handle_pet_skill_upgrade(conn: &mut Conn, payload: &[u8], out: &mut HandleOutcome) {
    if payload.len() < 4 {
        return;
    }
    let stt = payload[0];
    let skill_id = encoder::u16_le(payload[1], payload[2]);
    let target_lv = payload[3];

    if let Some(pet) = conn.session.pets.iter_mut().find(|p| p.stt == stt) {
        for sk in &mut pet.skills {
            if sk.0 == skill_id || sk.0 == 0 {
                sk.0 = skill_id;
                sk.1 = target_lv;

                out.send(format!(
                    "F4440F00080204{:02X}6E01{}{}",
                    stt,
                    encoder::le32(target_lv as u32),
                    encoder::le32(skill_id as u32)
                ));
                break;
            }
        }
    }
}

/// Handle Opcode 0x2C — Pet Reborn.
pub fn handle_pet_reborn(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.is_empty() {
        return;
    }
    let stt = payload[0];

    if let Some(pos) = conn.session.pets.iter().position(|p| p.stt == stt) {
        let pet = &mut conn.session.pets[pos];
        pet.reborn += 1;
        pet.level = 1;

        let pet_id_le = encoder::le32(pet.id as u32);
        let player_id_le = encoder::le32(conn.session.id);

        out.send(format!("F44407000F02{}{:02X}", pet_id_le, stt));
        out.send(format!(
            "F4440C000F01{}{:02X}{}01",
            player_id_le, stt, pet_id_le
        ));
        out.send(format!("F44406001301{}", pet_id_le));
        out.send("F44402002C01");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::{Conn, PetState};
    use std::sync::Arc;

    #[test]
    fn test_player_skill_learn() {
        let mut conn = Conn::new();
        conn.session.skill_point = 5;

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        // skill 10001 (0x2711) target lv 1
        let payload = vec![0x11, 0x27, 0x01];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
        handle_skills(&mut ctx);

        assert_eq!(conn.session.skills.len(), 1);
        assert_eq!(conn.session.skills[0], (10001, 1));
        assert_eq!(conn.session.skill_point, 4);
        assert_eq!(out.outgoing.len(), 2);
    }

    #[test]
    fn test_pet_reborn() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.pets.push(PetState {
            stt: 1,
            id: 15001,
            level: 50,
            reborn: 0,
            ..Default::default()
        });

        let data = GameData::default();
        let service = BattleService::new(Arc::new(GameData::default()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1]);
        handle_pet_reborn(&mut ctx);

        assert_eq!(conn.session.pets[0].reborn, 1);
        assert_eq!(conn.session.pets[0].level, 1);
        assert_eq!(out.outgoing.len(), 4);
    }
}
