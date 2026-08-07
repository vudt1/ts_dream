//! Learn / upgrade skills (Opcode 0x1C) & Pet Reborn (Opcode 0x2C) handlers.

use crate::battle::rng::DotNetRandom;
use crate::data::tables::Npc;
use crate::db::persist;
use crate::protocol::encoder;
use crate::server::handler::{HandleOutcome, OpcodeCtx};
use crate::server::session::Conn;

pub fn can_learn_element(player_element: u8, skill_element: i64) -> bool {
    match player_element {
        1 => skill_element != 3,
        2 => skill_element != 4,
        3 => skill_element != 1,
        4 => skill_element != 2,
        _ => true,
    }
}

pub fn get_point_skill_add(player_element: u8, skill_element: i64, base_point: i64) -> u16 {
    let double_cost = match player_element {
        1 => skill_element == 2 || skill_element == 4,
        2 => skill_element == 1 || skill_element == 3,
        3 => skill_element == 2 || skill_element == 4,
        4 => skill_element == 1 || skill_element == 3,
        _ => false,
    };
    if double_cost {
        (base_point * 2) as u16
    } else {
        base_point as u16
    }
}

/// A pet base stat the reborn bonus points can land on.
#[derive(Clone, Copy, PartialEq)]
enum PetStatKind {
    Int,
    Atk,
    Def,
    Hpx,
    Spx,
    Agi,
}

/// Weighted random stat pick — C# `Data.GetRandomPointPet` (Data.cs:98-155).
///
/// Consumes 7 `.NET` draws per call in the exact C# order: 6 tie-break
/// `Next(1,999)` (one per stat, Int→Agi) + 1 selection `Next(1,1000)`.
fn random_point_stat(rng: &mut DotNetRandom, npc: &Npc) -> PetStatKind {
    let raw = [
        (npc.int1, PetStatKind::Int),
        (npc.atk, PetStatKind::Atk),
        (npc.def, PetStatKind::Def),
        (npc.hpx, PetStatKind::Hpx),
        (npc.spx, PetStatKind::Spx),
        (npc.agi, PetStatKind::Agi),
    ];
    let total: i64 = raw.iter().map(|(v, _)| *v).sum();
    if total <= 0 {
        return PetStatKind::Atk; // C# default `result = "Atk"`
    }
    let mut rows: Vec<(i64, i64, PetStatKind)> = raw
        .into_iter()
        .map(|(v, kind)| {
            let w = ((v as f64 / total as f64) * 1000.0).round() as i64;
            let r = i64::from(rng.next_range(1, 999));
            (w, r, kind)
        })
        .collect();
    // Sort `Point ASC, Random DESC` (Data.cs:122).
    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    let num7 = rng.next_range(1, 1000);
    let mut acc = 0i64;
    for (w, _, kind) in &rows {
        if i64::from(num7) <= acc + *w {
            return *kind;
        }
        acc += *w;
    }
    rows.last().map(|(_, _, k)| *k).unwrap_or(PetStatKind::Atk)
}

/// Dispatch Opcode 0x1C — Learn / upgrade skills.
pub async fn handle_skills(ctx: &mut OpcodeCtx<'_>) {
    let pool = ctx.env.pool;
    let data = ctx.data;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Player skill learn / upgrade
        1 => handle_player_skill_learn(conn, payload, out, data, pool).await,
        // Sub 2: Pet skill upgrade
        2 => handle_pet_skill_upgrade(conn, payload, out, data, pool).await,
        _ => {}
    }
}

async fn handle_player_skill_learn(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    data: &crate::data::loader::GameData,
    pool: Option<&sqlx::MySqlPool>,
) {
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

        let Some(skill_def) = data.skills.get(&i64::from(skill_id)) else {
            break;
        };

        if target_lv > skill_def.lv_max as u8 || (skill_def.reborn as u8) > conn.session.reborn {
            continue;
        }

        let existing_pos = conn
            .session
            .skills
            .iter()
            .position(|(id, _)| *id == skill_id);

        if let Some(pos) = existing_pos {
            let current_lv = conn.session.skills[pos].1;
            if target_lv <= current_lv {
                continue;
            }
            let needed = u16::from(target_lv - current_lv);
            if conn.session.skill_point < needed {
                break;
            }
            conn.session.skill_point -= needed;
            conn.session.skills[pos].1 = target_lv;

            out.send(format!(
                "F4440C0008016E01{}{}",
                encoder::le32(target_lv as u32),
                encoder::le32(skill_id as u32)
            ));
            persist::upsert_skill(
                pool,
                conn.session.id,
                skill_id,
                target_lv,
                skill_def.sp as u8,
                0,
            )
            .await;
            count += 1;
        } else {
            // Learning a new skill
            if !can_learn_element(conn.session.thuoctinh, skill_def.thuoctinh) {
                break;
            }
            // Check prerequisites id_dk (must be all 0, or at least 1 learned)
            let has_prereq = skill_def.id_dk.iter().all(|&id| id == 0)
                || skill_def.id_dk.iter().any(|&id| {
                    id > 0
                        && conn
                            .session
                            .skills
                            .iter()
                            .any(|(s_id, _)| *s_id == id as u16)
                });
            if !has_prereq {
                break;
            }

            let base_cost =
                get_point_skill_add(conn.session.thuoctinh, skill_def.thuoctinh, skill_def.point);
            let extra_cost = u16::from(target_lv.saturating_sub(1));
            let total_needed = base_cost + extra_cost;

            if skill_def.point <= 0 || conn.session.skill_point < total_needed {
                break;
            }

            conn.session.skill_point -= total_needed;
            conn.session.skills.push((skill_id, target_lv));

            out.send(format!(
                "F4440C0008016E01{}{}",
                encoder::le32(target_lv as u32),
                encoder::le32(skill_id as u32)
            ));
            persist::upsert_skill(
                pool,
                conn.session.id,
                skill_id,
                target_lv,
                skill_def.sp as u8,
                0,
            )
            .await;
            count += 1;
        }
    }

    if count > 0 {
        persist::update_player(
            pool,
            conn.session.id,
            "SkillPoint",
            i64::from(conn.session.skill_point),
        )
        .await;
        out.send(format!(
            "F4440C0008012501{}00000000",
            encoder::le32(conn.session.skill_point as u32)
        ));
    }
}

async fn handle_pet_skill_upgrade(
    conn: &mut Conn,
    payload: &[u8],
    out: &mut HandleOutcome,
    data: &crate::data::loader::GameData,
    pool: Option<&sqlx::MySqlPool>,
) {
    if payload.len() < 4 {
        return;
    }
    let stt = payload[0];
    let skill_id = encoder::u16_le(payload[1], payload[2]);
    let target_lv = payload[3];

    let Some(skill_def) = data.skills.get(&i64::from(skill_id)) else {
        return;
    };

    if target_lv > skill_def.lv_max as u8 {
        return;
    }

    if let Some(pet) = conn.session.pets.iter_mut().find(|p| p.stt == stt) {
        // Upgrade ONLY an existing skill slot
        if let Some(sk) = pet.skills.iter_mut().find(|s| s.0 == skill_id && s.0 != 0) {
            if target_lv > sk.1 {
                let cost = u16::from(target_lv - sk.1);
                if pet.skill_point >= cost {
                    pet.skill_point -= cost;
                    sk.1 = target_lv;

                    // Reply with LE16 hex for stt (4 hex chars)
                    out.send(format!(
                        "F4440F00080204{}6E01{}{}",
                        encoder::le16(stt as u16),
                        encoder::le32(target_lv as u32),
                        encoder::le32(skill_id as u32)
                    ));
                    persist::upsert_pet(pool, conn.session.id, pet).await;
                }
            }
        }
    }
}

/// Handle Opcode 0x2C — Pet Reborn.
pub async fn handle_pet_reborn(ctx: &mut OpcodeCtx<'_>) {
    let pool = ctx.env.pool;
    let hub = ctx.env.hub;
    let data = ctx.data;
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.is_empty() {
        return;
    }
    let stt = payload[0];

    let Some(pet_pos) = conn.session.pets.iter().position(|p| p.stt == stt) else {
        return;
    };

    let pet_id = conn.session.pets[pet_pos].id;
    let pet_lv = conn.session.pets[pet_pos].level;
    let pet_reborn = conn.session.pets[pet_pos].reborn;

    let threshold = if pet_reborn == 0 { 30u8 } else { 60u8 };

    // Scan homdo slots 1..25 for reborn item
    let mut match_slot = None;
    let mut target_npc_id = 0i64;

    for item in &conn.session.homdo {
        if item.slot >= 1 && item.slot <= 25 && item.id > 0 && item.count > 0 {
            if let Some(item_def) = data.items.get(&i64::from(item.id)) {
                if item_def.rb_pet_from == i64::from(pet_id)
                    && item_def.rb_pet_to > 0
                    && data.npcs.contains_key(&item_def.rb_pet_to)
                {
                    match_slot = Some(item.slot);
                    target_npc_id = item_def.rb_pet_to;
                    break;
                }
            }
        }
    }

    let Some(hd_slot) = match_slot else {
        return; // Silent failure when no valid Rb item found
    };

    // Consume 1 unit of reborn item from homdo
    let hd_pos = conn
        .session
        .homdo
        .iter()
        .position(|i| i.slot == hd_slot)
        .unwrap();
    if conn.session.homdo[hd_pos].count > 1 {
        conn.session.homdo[hd_pos].count -= 1;
    } else {
        conn.session.homdo.remove(hd_pos);
    }
    let updated_item = conn
        .session
        .homdo
        .iter()
        .find(|i| i.slot == hd_slot)
        .cloned()
        .unwrap_or(crate::server::session::InventoryItem {
            slot: hd_slot,
            ..Default::default()
        });
    persist::upsert_item(pool, conn.session.id, "homdo", &updated_item).await;

    let new_npc = &data.npcs[&target_npc_id];

    let bonus_points = (u16::from(pet_lv).saturating_sub(u16::from(threshold))) / 5;

    // C# `RebornPet` snapshots the NPC base stats (Client.cs:9893-9895) BEFORE
    // the bonus distribution and derives HpMax/SpMax from them (Data.cs:9958-9959);
    // the boosted values are stored to the stat columns after.
    let base_hpx = new_npc.hpx as u16;
    let base_spx = new_npc.spx as u16;
    let mut int_val = new_npc.int1 as u16;
    let mut atk_val = new_npc.atk as u16;
    let mut def_val = new_npc.def as u16;
    let mut hpx_val = base_hpx;
    let mut spx_val = base_spx;
    let mut agi_val = new_npc.agi as u16;

    // Distribute bonus points via the weighted `GetRandomPointPet` draw
    // (Data.cs:9961-9990), NOT a fixed rotation. Fresh time-seeded RNG mirrors
    // the C# global `Data.random_0` (non-deterministic, so not golden-covered).
    let mut rng = DotNetRandom::time_seeded();
    for _ in 0..bonus_points {
        match random_point_stat(&mut rng, new_npc) {
            PetStatKind::Int => int_val += 1,
            PetStatKind::Atk => atk_val += 1,
            PetStatKind::Def => def_val += 1,
            PetStatKind::Hpx => hpx_val += 1,
            PetStatKind::Spx => spx_val += 1,
            PetStatKind::Agi => agi_val += 1,
        }
    }

    // Pet HpMax/SpMax map the reborn level onto the player formula:
    // `getPetHpMax/getPetSpMax` (Data.cs:5569-5603): rb 0/1 → getXmax(0), rb 2 →
    // getXmax(1), all at level 1 and from the BASE stats.
    let rb_map = if new_npc.reborn == 2 { 1 } else { 0 };
    let hp_max = crate::battle::engine::get_hp_max(rb_map, 0, 1, i64::from(base_hpx)) as u16;
    let sp_max = crate::battle::engine::get_sp_max(rb_map, 0, 1, i64::from(base_spx)) as u16;

    let pet = &mut conn.session.pets[pet_pos];
    pet.id = new_npc.id as u16;
    pet.name = new_npc.name.clone();
    pet.level = 1;
    pet.thuoctinh = new_npc.thuoctinh as u8;
    pet.reborn = new_npc.reborn as u8;
    pet.hp = hp_max;
    pet.hp_max = hp_max;
    pet.sp = sp_max;
    pet.sp_max = sp_max;
    pet.int1 = int_val;
    pet.atk = atk_val;
    pet.def = def_val;
    pet.hpx = hpx_val;
    pet.spx = spx_val;
    pet.agi = agi_val;
    pet.fai = 60;
    pet.texp = 6;

    for i in 0..4 {
        let sk_id = new_npc.skill[i] as u16;
        let lv = match sk_id {
            10016 | 11016 | 12016 | 13015 => 10,
            _ => 1,
        };
        pet.skills[i] = if sk_id > 0 { (sk_id, lv) } else { (0, 0) };
    }

    persist::upsert_pet(pool, conn.session.id, pet).await;

    let player_id_le = encoder::le32(conn.session.id);
    let pet_id_le = encoder::le32(pet.id as u32);

    // Map broadcasts first (C# `Server.SendToAllMapid` includes the sender, so
    // `07000F02` / `0C000F01` go to the player AND the map — the latter via
    // `broadcast_except`). Client.cs:9993-9995.
    let f_0f02 = format!("F44407000F02{}{:02X}", player_id_le, stt);
    let f_0f01 = format!("F4440C000F01{}{:02X}{}01", player_id_le, stt, pet_id_le);
    out.send(f_0f02.clone());
    out.send(f_0f01.clone());
    if let Some(hub) = hub {
        hub.broadcast_except(conn.session.id, &f_0f02).await;
        hub.broadcast_except(conn.session.id, &f_0f01).await;
    }

    // Pet status + trailer (C# `Data.SendStatusPet`, Data.cs:2212-2278).
    for f in crate::server::spawn::pet_status_single(&conn.session, stt) {
        out.send(f);
    }

    out.send(format!("F44406001301{}", pet_id_le));
    out.send("F44402002C01");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::data::tables::{Item, Npc, Skill};
    use crate::server::handler::{test_ctx, HandleOutcome};
    use crate::server::session::{Conn, InventoryItem, PetState};
    use std::sync::Arc;

    fn test_game_data() -> GameData {
        let mut data = GameData::default();
        // Add skill 10001 (element 1, point 1, lv_max 10, reborn 0)
        data.skills.insert(
            10001,
            Skill {
                id: 10001,
                name: "Earth Skill".into(),
                point: 1,
                thuoctinh: 1,
                lv_max: 10,
                reborn: 0,
                ..Default::default()
            },
        );
        data.skills.insert(
            10002,
            Skill {
                id: 10002,
                name: "Fire Skill".into(),
                point: 1,
                thuoctinh: 3,
                lv_max: 10,
                reborn: 0,
                ..Default::default()
            },
        );
        // Add reborn pet item and pet NPC templates
        data.items.insert(
            20001,
            Item {
                id: 20001,
                rb_pet_from: 15001,
                rb_pet_to: 15002,
                ..Default::default()
            },
        );
        data.npcs.insert(
            15002,
            Npc {
                id: 15002,
                name: b"Reborn Pet".to_vec(),
                reborn: 1,
                thuoctinh: 1,
                hpx: 10,
                spx: 10,
                atk: 20,
                skill: [10001, 0, 0, 0],
                ..Default::default()
            },
        );
        data
    }

    #[tokio::test]
    async fn test_player_skill_learn_success() {
        let mut conn = Conn::new();
        conn.session.skill_point = 5;
        conn.session.thuoctinh = 1; // Earth

        let data = test_game_data();
        let service = BattleService::new(Arc::new(test_game_data()));
        let mut out = HandleOutcome::default();
        let payload = vec![0x11, 0x27, 0x01]; // skill 10001 target lv 1
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
        handle_skills(&mut ctx).await;

        assert_eq!(conn.session.skills.len(), 1);
        assert_eq!(conn.session.skills[0], (10001, 1));
        assert_eq!(conn.session.skill_point, 4);
        assert_eq!(out.outgoing.len(), 2);
    }

    #[tokio::test]
    async fn test_player_skill_learn_opposing_element_rejected() {
        let mut conn = Conn::new();
        conn.session.skill_point = 5;
        conn.session.thuoctinh = 1; // Earth player cannot learn Fire skill (10002)

        let data = test_game_data();
        let service = BattleService::new(Arc::new(test_game_data()));
        let mut out = HandleOutcome::default();
        let payload = vec![0x12, 0x27, 0x01]; // skill 10002 target lv 1
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &payload);
        handle_skills(&mut ctx).await;

        assert!(
            conn.session.skills.is_empty(),
            "Earth player cannot learn Fire skill"
        );
        assert_eq!(conn.session.skill_point, 5);
    }

    #[tokio::test]
    async fn test_pet_reborn_consumes_item_and_transforms_pet() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.pets.push(PetState {
            stt: 1,
            id: 15001,
            level: 50,
            reborn: 0,
            skills: [(10001, 1), (0, 0), (0, 0), (0, 0)],
            ..Default::default()
        });
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 20001,
            count: 1,
            ..Default::default()
        });

        let data = test_game_data();
        let service = BattleService::new(Arc::new(test_game_data()));
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1]);
        handle_pet_reborn(&mut ctx).await;

        assert_eq!(conn.session.pets[0].id, 15002);
        assert_eq!(conn.session.pets[0].reborn, 1);
        assert_eq!(conn.session.pets[0].level, 1);
        assert!(conn.session.homdo.is_empty(), "Reborn item consumed");
        assert!(out.outgoing.iter().any(|f| f.contains("0F08")));
        assert!(out.outgoing.iter().any(|f| f.contains("2C01")));
    }
}
