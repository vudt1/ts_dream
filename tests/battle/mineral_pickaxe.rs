//! Integration tests for Mineral NPC Immunity & Pickaxe Weapon Rules (Ticket 03).
//!
//! Validates:
//! 1. `Battle::add_player`, `load_leader_pets`, `load_member_pet` independently load `weapon_id`
//!    from `Session.trangbi` (slot `3` for player, `pet.stt * 10 + 3` for pets), while preserving
//!    the 23-byte `WarInfo::packet_hex` wire snapshot.
//! 2. Pickaxe `[10001, 10530, 10531, 10532, 10533, 10534]` allows normal attack (`10000`),
//!    physical (`skill_type == 1`), magic (`skill_type == 2`), Holy Spirit (`14053`, `type 19`),
//!    and debuffs (`type 3`, `type 15`) to hit Mineral NPCs (`kind == 16`, `22010`, `22011`).
//! 3. Without a valid pickaxe (unarmed, normal weapon, or `10497`/`10499`/`10500`), all harmful
//!    attacks/skills against Mineral NPCs return `MISS` (`0` damage, no debuff, no SP drain,
//!    no buff purge, no shield reflect, no hit EXP), while consuming attacker SP and ending turn.
//! 4. Strict independence between player and pet weapon slots.
//! 5. Mixed AoE (normal NPC + Mineral NPC) and Mixed Combo (pickaxe + non-pickaxe) in `32 01`.
//! 6. Rear Guard (`20006` *Hộ chủ*) evaluates mineral immunity on the `rear` cell actually
//!    receiving damage.
//! 7. Capture (`15001`) is rejected on `22010`, `22011`, and `25001..=25252` even `< 50%` HP.
//! 8. Player-owned pets (`typ == 4`) with ID `22010`/`22011` in PvP do NOT receive mineral immunity.

use std::collections::HashMap;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::engine::PICKAXE_IDS;
use ts_dream::battle::packets::miss_status;
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, Out, PlayerSnapshot,
};
use ts_dream::data::tables::{Item, Npc, Skill, TexpRow};
use ts_dream::protocol::SKILL_HOLY_SPIRIT;
use ts_dream::server::session::{InventoryItem, PetState, Session};

fn make_session(id: u32, weapon_id: u16, pets: Vec<(u8, u16, u16)>) -> Session {
    let mut s = Session::default();
    s.id = id;
    s.level = 120;
    s.hp = 1000;
    s.hp_max = 1000;
    s.sp = 500;
    s.sp_max = 500;
    s.atk = 200;
    s.int1 = 200;
    s.def = 100;
    s.agi = 100;
    if weapon_id > 0 {
        s.trangbi.push(InventoryItem {
            slot: 3,
            id: weapon_id,
            count: 1,
            ..Default::default()
        });
    }
    if let Some((first_stt, _, _)) = pets.first() {
        s.active_pet_stt = *first_stt;
    }
    for (stt, pet_id, pet_weapon_id) in pets {
        s.pets.push(PetState {
            stt,
            id: pet_id,
            name: b"Pet".to_vec(),
            level: 120,
            thuoctinh: 1,
            reborn: 0,
            hp: 800,
            hp_max: 800,
            sp: 400,
            sp_max: 400,
            int1: 200,
            atk: 200,
            def: 100,
            agi: 95,
            ..Default::default()
        });
        if pet_weapon_id > 0 {
            s.trangbi.push(InventoryItem {
                slot: stt * 10 + 3,
                id: pet_weapon_id,
                count: 1,
                ..Default::default()
            });
        }
    }
    s
}

fn base_npcs() -> HashMap<i64, Npc> {
    let mut npcs = HashMap::new();
    // Normal NPC
    npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"NormalNpc".to_vec(),
            lv: 20,
            hp: 500,
            sp: 200,
            def: 10,
            agi: 10,
            kind: 0,
            can_capture: true,
            ..Default::default()
        },
    );
    // Mineral NPC kind == 16 (25002)
    npcs.insert(
        25002,
        Npc {
            id: 25002,
            name: b"Ore25002".to_vec(),
            lv: 20,
            hp: 500,
            sp: 200,
            def: 10,
            agi: 10,
            kind: 16,
            can_capture: true,
            ..Default::default()
        },
    );
    // Special Mineral NPC 22010 (Thuy Tinh Quai)
    npcs.insert(
        22010,
        Npc {
            id: 22010,
            name: b"ThuyTinh".to_vec(),
            lv: 20,
            hp: 500,
            sp: 200,
            def: 10,
            agi: 10,
            kind: 0,
            can_capture: true,
            ..Default::default()
        },
    );
    // Special Mineral NPC 22011 (Khoang Chi Quai)
    npcs.insert(
        22011,
        Npc {
            id: 22011,
            name: b"KhoangChi".to_vec(),
            lv: 20,
            hp: 500,
            sp: 200,
            def: 10,
            agi: 10,
            kind: 0,
            can_capture: true,
            ..Default::default()
        },
    );
    // Normal NPC with skill 20006 (Hộ chủ) for Rear Guard test
    npcs.insert(
        1002,
        Npc {
            id: 1002,
            name: b"GuardNormal".to_vec(),
            lv: 20,
            hp: 500,
            sp: 200,
            def: 10,
            agi: 200,
            kind: 0,
            skill: [20006, 20006, 20006, 20006],
            ..Default::default()
        },
    );
    // Mineral NPC with skill 20006 (Hộ chủ) for Rear Guard test
    npcs.insert(
        25003,
        Npc {
            id: 25003,
            name: b"GuardMineral".to_vec(),
            lv: 20,
            hp: 500,
            sp: 200,
            def: 10,
            agi: 200,
            kind: 16,
            skill: [20006, 20006, 20006, 20006],
            ..Default::default()
        },
    );
    // Pet template used by player pets
    npcs.insert(
        41001,
        Npc {
            id: 41001,
            name: b"PlayerPet".to_vec(),
            lv: 120,
            hp: 800,
            sp: 400,
            skill: [10000, 10001, 10008, 13001],
            ..Default::default()
        },
    );
    npcs
}

fn base_skills() -> HashMap<i64, Skill> {
    let mut skills = HashMap::new();
    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    skills.insert(
        10001,
        Skill {
            id: 10001,
            name: "PhysSkill".to_string(),
            sl_danh: 1,
            skill_type: 1,
            sp: 10,
            do_manh: 50,
            ..Default::default()
        },
    );
    skills.insert(
        10005,
        Skill {
            id: 10005,
            name: "NemDaAoE".to_string(),
            sl_danh: 3,
            skill_type: 1,
            sp: 15,
            do_manh: 40,
            ..Default::default()
        },
    );
    skills.insert(
        10008,
        Skill {
            id: 10008,
            name: "MagicSkill".to_string(),
            sl_danh: 1,
            skill_type: 2,
            sp: 20,
            do_manh: 60,
            ..Default::default()
        },
    );
    skills.insert(
        13001,
        Skill {
            id: 13001,
            name: "Debuff3".to_string(),
            sl_danh: 1,
            skill_type: 3,
            sp: 15,
            ..Default::default()
        },
    );
    skills.insert(
        13015,
        Skill {
            id: 13015,
            name: "DrainSP3".to_string(),
            sl_danh: 1,
            skill_type: 3,
            sp: 15,
            ..Default::default()
        },
    );
    skills.insert(
        13010,
        Skill {
            id: 13010,
            name: "Debuff15".to_string(),
            sl_danh: 1,
            skill_type: 15,
            sp: 15,
            ..Default::default()
        },
    );
    skills.insert(
        i64::from(SKILL_HOLY_SPIRIT),
        Skill {
            id: i64::from(SKILL_HOLY_SPIRIT),
            name: "HolySpirit".to_string(),
            sl_danh: 1,
            skill_type: 19,
            sp: 25,
            ..Default::default()
        },
    );
    skills.insert(
        20006,
        Skill {
            id: 20006,
            name: "HoChu".to_string(),
            sl_danh: 1,
            skill_type: 17,
            ..Default::default()
        },
    );
    skills.insert(
        15001,
        Skill {
            id: 15001,
            name: "Catch".to_string(),
            sl_danh: 1,
            skill_type: 11,
            ..Default::default()
        },
    );
    skills
}

#[derive(Debug)]
struct DecodedTargetHit {
    row: u8,
    col: u8,
    hit: u8,
    damage: u16,
}

#[derive(Debug)]
struct DecodedActionBlock {
    src_row: u8,
    src_col: u8,
    hits: Vec<DecodedTargetHit>,
}

fn parse_3201_blocks(out: &[Out]) -> Vec<DecodedActionBlock> {
    let mut blocks = Vec::new();
    for msg in out {
        let Out::Broadcast(hex) = msg else { continue };
        for frame in hex.split("F444") {
            if frame.len() < 8 {
                continue;
            }
            let payload = &frame[4..];
            if !payload.starts_with("3201") {
                continue;
            }
            let bytes = (0..payload.len() - 4)
                .step_by(2)
                .map(|i| u8::from_str_radix(&payload[4 + i..6 + i], 16).unwrap())
                .collect::<Vec<u8>>();
            let mut idx = 0usize;
            while idx + 2 <= bytes.len() {
                let block_len = u16::from_le_bytes([bytes[idx], bytes[idx + 1]]) as usize;
                idx += 2;
                if idx + block_len > bytes.len() || block_len < 6 {
                    break;
                }
                let b = &bytes[idx..idx + block_len];
                idx += block_len;
                let src_row = b[0];
                let src_col = b[1];
                let count = b[5] as usize;
                let mut hits = Vec::new();
                let mut p = 6usize;
                for _ in 0..count {
                    if p + 8 > b.len() {
                        break;
                    }
                    let row = b[p];
                    let col = b[p + 1];
                    let hit = b[p + 2];
                    let eff_cnt = b[p + 4] as usize;
                    let mut dmg = 0u16;
                    let mut ep = p + 5;
                    if ep + 4 <= b.len() {
                        dmg = u16::from_le_bytes([b[ep + 1], b[ep + 2]]);
                        ep += eff_cnt * 4;
                    }
                    hits.push(DecodedTargetHit {
                        row,
                        col,
                        hit,
                        damage: dmg,
                    });
                    p = ep;
                }
                blocks.push(DecodedActionBlock {
                    src_row,
                    src_col,
                    hits,
                });
            }
        }
    }
    blocks
}

#[test]
fn test_war_info_weapon_id_loading_and_23byte_snapshot_parity() {
    let leader = make_session(101, 10001, vec![(1, 41001, 10534), (2, 41001, 0)]);
    let member = make_session(102, 10531, vec![(1, 41001, 10532)]);

    let mut battle = Battle::new(1, 42);
    battle.add_player(&leader, 101, 3, 2);
    battle.load_leader_pets(&leader, 101, 3);
    let leader_pet1 = battle.cell(2, 1).unwrap();
    assert_eq!(leader_pet1.weapon_id, 10534);
    assert_eq!(leader_pet1.packet_hex().len(), 46);

    let leader_pet2 = battle.cell(2, 3).unwrap();
    assert_eq!(leader_pet2.weapon_id, 0);

    battle.add_player(&member, 101, 3, 1);
    battle.load_member_pet(&member, 101, 3, 1);

    let member_cell = battle.cell(3, 1).unwrap();
    assert_eq!(member_cell.weapon_id, 10531);

    let member_pet = battle.cell(2, 1).unwrap(); // member pet at (3^1, 1) overwrites leader pet1
    assert_eq!(member_pet.weapon_id, 10532);
}

#[test]
fn test_all_six_pickaxes_damage_and_debuff_mineral_npcs() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let pet_slots = HashMap::<i64, [i64; 4]>::new();
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();

    for &pickaxe_id in &PICKAXE_IDS {
        for &mineral_id in &[25002i64, 22010, 22011] {
            let leader = make_session(101, pickaxe_id, vec![]);
            let mut battle = Battle::with_seeds(1, 777, 1, 1, 1);
            battle.add_player(&leader, 101, 3, 2);
            battle.add_npc(npcs.get(&mineral_id).unwrap(), 1, 0, 2, 3);

            let mut cmds = BattleCommands::new();
            cmds.insert(
                (3, 2),
                BattleCommand {
                    row: 3,
                    col: 2,
                    target_row: 0,
                    target_col: 2,
                    action: BattleAction::NormalAttack,
                },
            );

            let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);
            let mut out = Vec::new();
            battle.run_turn(&data, &cmds, &mut out);

            let target = battle.cell(0, 2).unwrap();
            assert!(
                target.hp < 500,
                "Pickaxe {pickaxe_id} should damage mineral NPC {mineral_id}, hp={}",
                target.hp
            );
        }
    }

    // Physical (10001), Magic (10008), Holy Spirit (14053), Type3 (13001), Type15 (13010)
    for (skill_id, skill_lv) in [(10001, 5), (10008, 5), (i64::from(SKILL_HOLY_SPIRIT), 5)] {
        let leader = make_session(101, 10534, vec![]);
        let mut battle = Battle::with_seeds(1, 100, 1, 1, 1);
        battle.add_player(&leader, 101, 3, 2);
        battle.add_npc(npcs.get(&25002).unwrap(), 1, 0, 2, 3);

        let mut cmds = BattleCommands::new();
        cmds.insert(
            (3, 2),
            BattleCommand {
                row: 3,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill { skill_id, skill_lv },
            },
        );
        let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);
        let mut out = Vec::new();
        battle.run_turn(&data, &cmds, &mut out);
        assert!(
            battle.cell(0, 2).unwrap().hp < 500,
            "Skill {skill_id} with pickaxe should damage mineral NPC"
        );
    }

    // Debuff type 3 & type 15 with pickaxe: use a seed that hits
    for (skill_id, check_type3) in [(13001i64, true), (13010i64, false)] {
        let mut hit_observed = false;
        for seed in 1..=20u64 {
            let leader = make_session(101, 10001, vec![]);
            let mut battle = Battle::with_seeds(1, seed as i32, seed as i32, seed as i32, seed as i32);
            battle.add_player(&leader, 101, 3, 2);
            battle.add_npc(npcs.get(&22010).unwrap(), 1, 0, 2, 3);
            let mut cmds = BattleCommands::new();
            cmds.insert(
                (3, 2),
                BattleCommand {
                    row: 3,
                    col: 2,
                    target_row: 0,
                    target_col: 2,
                    action: BattleAction::CastSkill {
                        skill_id,
                        skill_lv: 5,
                    },
                },
            );
            let data =
                BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);
            let mut out = Vec::new();
            battle.run_turn(&data, &cmds, &mut out);
            let t = battle.cell(0, 2).unwrap();
            if (check_type3 && t.type3_id > 0) || (!check_type3 && t.type15_id > 0) {
                hit_observed = true;
                break;
            }
        }
        assert!(
            hit_observed,
            "Pickaxe holder should be able to inflict debuff skill {skill_id} on mineral NPC"
        );
    }
}

#[test]
fn test_non_pickaxe_weapons_miss_all_harmful_skills_and_consume_sp_and_turn() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let pet_slots = HashMap::<i64, [i64; 4]>::new();
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();

    // Unarmed (0), normal sword (10002), and excluded kind-39 items (10497, 10499, 10500)
    for weapon_id in [0u16, 10002, 10497, 10499, 10500] {
        for &mineral_id in &[25002i64, 22010, 22011] {
            for action in [
                BattleAction::NormalAttack,
                BattleAction::CastSkill {
                    skill_id: 10001,
                    skill_lv: 5,
                },
                BattleAction::CastSkill {
                    skill_id: 10008,
                    skill_lv: 5,
                },
                BattleAction::CastSkill {
                    skill_id: i64::from(SKILL_HOLY_SPIRIT),
                    skill_lv: 5,
                },
                BattleAction::CastSkill {
                    skill_id: 13015,
                    skill_lv: 5,
                },
                BattleAction::CastSkill {
                    skill_id: 13010,
                    skill_lv: 5,
                },
            ] {
                let leader = make_session(101, weapon_id, vec![]);
                let mut battle = Battle::with_seeds(1, 555, 1, 1, 1);
                battle.add_player(&leader, 101, 3, 2);
                battle.add_npc(npcs.get(&mineral_id).unwrap(), 1, 0, 2, 3);
                // Put reflect shield + buff on mineral NPC to ensure neither reflect nor purge triggers
                {
                    let m = battle.cell_mut(0, 2).unwrap();
                    m.type4_id = 10015;
                    m.type4_turn = 3;
                    m.type3_id = 14001; // stunned so NPC doesn't counter-attack player
                    m.type3_turn = 2;
                }
                {
                    let p = battle.cell_mut(3, 2).unwrap();
                    p.hp = 400; // < hp_max to detect any accidental drain-heal
                }

                let sp_before = battle.cell(3, 2).unwrap().sp;
                let exp_before = battle.cell(3, 2).unwrap().exp;

                let mut cmds = BattleCommands::new();
                cmds.insert(
                    (3, 2),
                    BattleCommand {
                        row: 3,
                        col: 2,
                        target_row: 0,
                        target_col: 2,
                        action,
                    },
                );

                let data = BattleData::new(
                    &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
                );
                let mut out = Vec::new();
                battle.run_turn(&data, &cmds, &mut out);

                let target = battle.cell(0, 2).unwrap();
                assert_eq!(target.hp, 500, "weapon={weapon_id} mineral={mineral_id} action={action:?}");
                assert_eq!(target.sp, 200);
                assert_eq!(target.type3_id, 0);
                assert_eq!(target.type15_id, 0);
                assert_eq!(target.type4_id, 10015, "Holy Spirit must not purge buff on miss");

                let attacker = battle.cell(3, 2).unwrap();
                assert!(attacker.hp <= 400, "No drain heal on mineral miss");
                assert!(
                    !out.iter().any(|o| matches!(o, Out::Broadcast(h) if h.contains("234E"))),
                    "No 20003 reflect frame on mineral miss"
                );
                assert_eq!(attacker.exp, exp_before, "No hit EXP on mineral miss");
                assert!(attacker.attacked, "Turn must be marked attacked");
                if let BattleAction::CastSkill { skill_id, .. } = action {
                    let expected_cost = skills.get(&skill_id).unwrap().sp;
                    assert_eq!(attacker.sp, sp_before - expected_cost);
                }

                let blocks = parse_3201_blocks(&out);
                let player_blk = blocks
                    .iter()
                    .find(|b| b.src_row == 3 && b.src_col == 2)
                    .expect("Player 3201 block present");
                assert_eq!(player_blk.hits.len(), 1);
                assert_eq!(player_blk.hits[0].hit, miss_status::MISS);
                assert_eq!(player_blk.hits[0].damage, 0);
            }
        }
    }
}

#[test]
fn test_player_and_pet_pickaxe_strict_independence() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let pet_slots = HashMap::<i64, [i64; 4]>::new();
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();

    // Case 1: Player has pickaxe (10001), Pet has NO pickaxe (0)
    let s1 = make_session(101, 10001, vec![(1, 41001, 0)]);
    let mut b1 = Battle::with_seeds(1, 123, 1, 1, 1);
    b1.add_player(&s1, 101, 3, 2);
    b1.load_leader_pets(&s1, 101, 3);
    b1.add_npc(npcs.get(&25002).unwrap(), 1, 0, 2, 3);
    b1.cell_mut(0, 2).unwrap().hp = 5000;
    b1.cell_mut(0, 2).unwrap().hp_max = 5000;
    // Make player act before pet so both act independently without combo interference
    b1.cell_mut(3, 2).unwrap().agi = 200;
    b1.cell_mut(2, 1).unwrap().agi = 50;

    let mut cmds = BattleCommands::new();
    cmds.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 10001,
                skill_lv: 5,
            },
        },
    );
    cmds.insert(
        (2, 1),
        BattleCommand {
            row: 2,
            col: 1,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    );
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);
    let mut out1 = Vec::new();
    b1.run_turn(&data, &cmds, &mut out1);

    let blocks1 = parse_3201_blocks(&out1);
    let player_blk = blocks1.iter().find(|b| b.src_row == 3 && b.src_col == 2).unwrap();
    let pet_blk = blocks1.iter().find(|b| b.src_row == 2 && b.src_col == 1).unwrap();
    assert!(player_blk.hits[0].damage > 0);
    assert_eq!(pet_blk.hits[0].hit, miss_status::MISS);
    assert_eq!(pet_blk.hits[0].damage, 0);

    // Case 2: Player has NO pickaxe (0), Pet has pickaxe (10534)
    let s2 = make_session(101, 0, vec![(1, 41001, 10534)]);
    let mut b2 = Battle::with_seeds(1, 123, 1, 1, 1);
    b2.add_player(&s2, 101, 3, 2);
    b2.load_leader_pets(&s2, 101, 3);
    b2.add_npc(npcs.get(&25002).unwrap(), 1, 0, 2, 3);
    b2.cell_mut(0, 2).unwrap().hp = 5000;
    b2.cell_mut(0, 2).unwrap().hp_max = 5000;
    b2.cell_mut(3, 2).unwrap().agi = 200;
    b2.cell_mut(2, 1).unwrap().agi = 50;

    let mut out2 = Vec::new();
    b2.run_turn(&data, &cmds, &mut out2);
    let blocks2 = parse_3201_blocks(&out2);
    let player_blk2 = blocks2.iter().find(|b| b.src_row == 3 && b.src_col == 2).unwrap();
    let pet_blk2 = blocks2
        .iter()
        .find(|b| b.src_row == 2 && b.src_col == 1)
        .unwrap_or_else(|| panic!("missing pet_blk2: blocks2={blocks2:?} out2={out2:?}"));
    assert_eq!(player_blk2.hits[0].hit, miss_status::MISS);
    assert_eq!(player_blk2.hits[0].damage, 0);
    assert!(pet_blk2.hits[0].damage > 0);
}

#[test]
fn test_mixed_aoe_and_mixed_combo_per_target_and_per_attacker() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let pet_slots = HashMap::<i64, [i64; 4]>::new();
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);

    // 1. Mixed AoE: unarmed player casts 10005 (AoE 3) on (0,2)=normal, (0,1)=mineral, (0,4)=normal
    let s_aoe = make_session(101, 0, vec![]);
    let mut b_aoe = Battle::with_seeds(1, 42, 1, 1, 1);
    b_aoe.add_player(&s_aoe, 101, 3, 2);
    b_aoe.add_npc(npcs.get(&1001).unwrap(), 1, 0, 2, 3);
    b_aoe.add_npc(npcs.get(&25002).unwrap(), 2, 0, 1, 3);
    b_aoe.add_npc(npcs.get(&1001).unwrap(), 3, 0, 4, 3);

    let mut cmds_aoe = BattleCommands::new();
    cmds_aoe.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 10005,
                skill_lv: 5,
            },
        },
    );
    let mut out_aoe = Vec::new();
    b_aoe.run_turn(&data, &cmds_aoe, &mut out_aoe);

    assert!(b_aoe.cell(0, 2).unwrap().hp < 500, "Normal NPC (0,2) damaged");
    assert!(b_aoe.cell(0, 4).unwrap().hp < 500, "Normal NPC (0,4) damaged");
    assert_eq!(b_aoe.cell(0, 1).unwrap().hp, 500, "Mineral NPC (0,1) untouched");
    let blocks_aoe = parse_3201_blocks(&out_aoe);
    let player_aoe = blocks_aoe
        .iter()
        .find(|b| b.src_row == 3 && b.src_col == 2)
        .unwrap();
    let min_hit = player_aoe
        .hits
        .iter()
        .find(|h| h.row == 0 && h.col == 1)
        .unwrap();
    assert_eq!(min_hit.hit, miss_status::MISS);
    assert_eq!(min_hit.damage, 0);

    // 2. Mixed Combo: Player (10001 pickaxe) + Pet (no pickaxe) combo on Mineral NPC (0,2)
    let mut combo_observed = false;
    for seed in 1..=25u64 {
        let s_combo = make_session(101, 10001, vec![(1, 41001, 0)]);
        let mut b_combo = Battle::with_seeds(1, 0, seed as i32, seed as i32, seed as i32);
        b_combo.add_player(&s_combo, 101, 3, 2);
        b_combo.load_leader_pets(&s_combo, 101, 3);
        b_combo.add_npc(npcs.get(&25002).unwrap(), 1, 0, 2, 3);
        b_combo.cell_mut(0, 2).unwrap().hp = 5000;
        b_combo.cell_mut(0, 2).unwrap().hp_max = 5000;
        // Same AGI => eligible for combo
        b_combo.cell_mut(3, 2).unwrap().agi = 100;
        b_combo.cell_mut(2, 1).unwrap().agi = 99;

        let mut cmds = BattleCommands::new();
        cmds.insert(
            (3, 2),
            BattleCommand {
                row: 3,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        cmds.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        let mut out = Vec::new();
        b_combo.run_turn(&data, &cmds, &mut out);
        let has_two_player_records_in_one_3201 = out.iter().any(|o| {
            let Out::Broadcast(hex) = o else { return false };
            let blks = parse_3201_blocks(&[Out::Broadcast(hex.clone())]);
            blks.iter().any(|b| b.src_row == 3 && b.src_col == 2)
                && blks.iter().any(|b| b.src_row == 2 && b.src_col == 1)
        });
        if has_two_player_records_in_one_3201 {
            combo_observed = true;
            let blocks = parse_3201_blocks(&out);
            let p_blk = blocks.iter().find(|b| b.src_row == 3 && b.src_col == 2).unwrap();
            let pet_blk = blocks.iter().find(|b| b.src_row == 2 && b.src_col == 1).unwrap();
            assert!(p_blk.hits[0].damage > 0, "Pickaxe player deals combo damage");
            assert_eq!(pet_blk.hits[0].hit, miss_status::MISS);
            assert_eq!(pet_blk.hits[0].damage, 0);
            break;
        }
    }
    assert!(combo_observed, "Should observe a mixed combo frame within 25 seeds");
}

#[test]
fn test_rear_guard_20006_evaluates_immunity_on_actual_rear_receiver() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let pet_slots = HashMap::<i64, [i64; 4]>::new();
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);

    // Case A: Mineral NPC at (0,2), Normal NPC guarding at rear (1,2) with skill 20006 (agi 200)
    // Unarmed attacker targets (0,2) -> rear Normal NPC takes damage!
    let s = make_session(101, 0, vec![]);
    let mut rear_hit_observed = false;
    for seed in 1..=10i32 {
        let mut b_a = Battle::with_seeds(1, 0, seed, seed, seed);
        b_a.add_player(&s, 101, 3, 2);
        b_a.add_npc(npcs.get(&25002).unwrap(), 1, 0, 2, 3);
        b_a.add_npc(npcs.get(&1002).unwrap(), 2, 1, 2, 3);

        let mut cmds = BattleCommands::new();
        cmds.insert(
            (3, 2),
            BattleCommand {
                row: 3,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 10001,
                    skill_lv: 5,
                },
            },
        );
        let mut out_a = Vec::new();
        b_a.run_turn(&data, &cmds, &mut out_a);
        assert_eq!(b_a.cell(0, 2).unwrap().hp, 500, "Front mineral untouched");
        if b_a.cell(1, 2).unwrap().hp < 500 {
            rear_hit_observed = true;
            break;
        }
    }
    assert!(
        rear_hit_observed,
        "Rear normal guard takes damage even without pickaxe"
    );

    // Case B: Normal NPC at (0,2), Mineral NPC guarding at rear (1,2) with skill 20006 (agi 200)
    // Unarmed attacker targets (0,2) -> rear Mineral NPC is immune (MISS, 0 damage)!
    let mut b_b = Battle::with_seeds(1, 0, 1, 1, 1);
    b_b.add_player(&s, 101, 3, 2);
    b_b.add_npc(npcs.get(&1001).unwrap(), 1, 0, 2, 3);
    b_b.add_npc(npcs.get(&25003).unwrap(), 2, 1, 2, 3);

    let mut cmds = BattleCommands::new();
    cmds.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 10001,
                skill_lv: 5,
            },
        },
    );
    let mut out_b = Vec::new();
    b_b.run_turn(&data, &cmds, &mut out_b);
    assert_eq!(b_b.cell(0, 2).unwrap().hp, 500, "Front normal untouched");
    assert_eq!(
        b_b.cell(1, 2).unwrap().hp,
        500,
        "Rear mineral guard must be immune when attacker has no pickaxe"
    );
    let blocks_b = parse_3201_blocks(&out_b);
    let player_blk_b = blocks_b
        .iter()
        .find(|b| b.src_row == 3 && b.src_col == 2)
        .unwrap_or_else(|| panic!("missing player_blk_b: blocks_b={blocks_b:?} out_b={out_b:?}"));
    assert_eq!(player_blk_b.hits[0].hit, miss_status::MISS);
    assert_eq!(player_blk_b.hits[0].damage, 0);
}

#[test]
fn test_capture_15001_rejected_on_all_mineral_npcs_and_22010_22011() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let mut pet_slots = HashMap::<i64, [i64; 4]>::new();
    pet_slots.insert(101, [0, 0, 0, 0]);
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);

    for &mineral_id in &[25002i64, 22010, 22011] {
        let leader = make_session(101, 10001, vec![]);
        let mut battle = Battle::with_seeds(1, 1, 1, 1, 1);
        battle.add_player(&leader, 101, 3, 2);
        battle.add_npc(npcs.get(&mineral_id).unwrap(), 1, 0, 2, 3);
        // Set HP strictly below 50% (100 / 500 = 20%)
        battle.cell_mut(0, 2).unwrap().hp = 100;

        let mut cmds = BattleCommands::new();
        cmds.insert(
            (3, 2),
            BattleCommand {
                row: 3,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::Capture,
            },
        );
        let mut out = Vec::new();
        battle.run_turn(&data, &cmds, &mut out);

        assert!(
            !out.iter().any(|o| matches!(o, Out::Catch { .. })),
            "Capture 15001 must be rejected for mineral NPC {mineral_id}"
        );
        assert_eq!(battle.cell(0, 2).unwrap().id, mineral_id);
    }
}

#[test]
fn test_pvp_player_pet_22010_22011_not_immune() {
    let npcs = base_npcs();
    let skills = base_skills();
    let items = HashMap::<i64, Item>::new();
    let pet_slots = HashMap::<i64, [i64; 4]>::new();
    let players = HashMap::<i64, PlayerSnapshot>::new();
    let texps = Vec::<TexpRow>::new();
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);

    for &pet_id in &[22010u16, 22011u16] {
        let p1 = make_session(101, 0, vec![]); // Unarmed attacker
        let p2 = make_session(202, 0, vec![(1, pet_id, 0)]);

        let mut battle = Battle::with_seeds(1, 42, 1, 1, 1);
        battle.add_player(&p1, 101, 3, 2);
        battle.add_player(&p2, 202, 0, 2);
        battle.load_leader_pets(&p2, 202, 0); // Puts p2's pet at (1, 1) with typ == 4
        battle.cell_mut(1, 1).unwrap().agi = 10;

        let mut cmds = BattleCommands::new();
        cmds.insert(
            (3, 2),
            BattleCommand {
                row: 3,
                col: 2,
                target_row: 1,
                target_col: 1,
                action: BattleAction::CastSkill {
                    skill_id: 10001,
                    skill_lv: 5,
                },
            },
        );
        let mut out = Vec::new();
        battle.run_turn(&data, &cmds, &mut out);

        let enemy_pet = battle.cell(1, 1).unwrap();
        assert_eq!(enemy_pet.typ, 4);
        assert!(
            enemy_pet.hp < 800,
            "PvP player pet {pet_id} (typ == 4) must NOT receive mineral immunity"
        );
    }
}
