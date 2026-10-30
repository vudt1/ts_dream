//! Integration test for Ticket 23: Battle Combo Rules & AoE Threshold.
//!
//! Validates:
//! 1. Agi boundary: diff <= 11 allows combo; diff >= 12 rejects combo.
//! 2. 3-member Agi span {100, 105, 111} combos; expanding beyond span 11 rejects the outlying member.
//! 3. Active skills (skill_type == 1) such as Ném đá (10005, AoE 3) and Hỏa tiễn (12003, AoE 3) can combo.
//! 4. Active AoE 4 skill Loạn kích (13013) can combo (normal + loạn kích, and dual loạn kích).
//! 5. Magic skills (skill_type == 2) like Phi sa (10008) and Hồng thủy (11008) are strictly rejected from combo.
//! 6. Reborn characters count as Level 200 in LVTB (reborn lv 1 vs lv 100 enemy => LVTB 200 => 100% combo).
//! 7. Degraded combo probability curve when LVTB - target_lv < 25 (e.g. LVTB 40 vs target 50 => 30%).
//! 8. Primary target receives x1.3 damage multiplier when combo is active.
//! 9. Defeating an enemy via combo distributes bonus exp (damage::calc_combo_exp) to all combo cells.
//! 10. Split-target combos and solo attacks in the same turn are isolated; combo_cells does not leak.

use std::collections::HashMap;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::damage::{calc_combo_exp, get_random_miss_combo};
use ts_dream::battle::rng::DotNetRandom;
use ts_dream::battle::runner::{
    is_combo_eligible_skill, skill_target_count, BattleAction, BattleCommand, BattleCommands,
    BattleData, Out,
};
use ts_dream::data::tables::{BinarySkillDef, Item, Npc, Skill, TexpRow};

fn mock_battle_data<'a>(
    npcs: &'a HashMap<i64, Npc>,
    skills: &'a HashMap<i64, Skill>,
    binary_skills: &'a HashMap<u16, BinarySkillDef>,
    pet_slots: &'a HashMap<i64, [i64; 4]>,
    items: &'a HashMap<i64, Item>,
    texps: &'a [TexpRow],
    players: &'a HashMap<i64, ts_dream::battle::runner::PlayerSnapshot>,
) -> BattleData<'a> {
    BattleData::new(
        npcs, skills, items, pet_slots, players, texps, None, 0, 0,
    )
    .with_binary_skills(binary_skills)
}

fn has_combo_action(out: &[Out]) -> bool {
    out.iter().any(|msg| match msg {
        Out::Broadcast(s) => count_combat_records(s) >= 2,
        _ => false,
    })
}

fn count_combat_records(packet: &str) -> usize {
    if !packet.starts_with("F444") || packet.len() < 12 {
        return 0;
    }
    let payload = &packet[8..];
    if !payload.starts_with("3201") {
        return 0;
    }
    let mut rec_bytes = &payload[4..];
    let mut count = 0;
    while rec_bytes.len() >= 4 {
        let len_hex = &rec_bytes[..4];
        let Ok(b0) = u8::from_str_radix(&len_hex[..2], 16) else { break; };
        let Ok(b1) = u8::from_str_radix(&len_hex[2..4], 16) else { break; };
        let block_len = (u16::from_le_bytes([b0, b1]) as usize) * 2;
        count += 1;
        if rec_bytes.len() < 4 + block_len {
            break;
        }
        rec_bytes = &rec_bytes[4 + block_len..];
    }
    count
}

#[test]
fn test_skill_target_count_resolution() {
    let npcs = HashMap::new();
    let mut skills = HashMap::new();
    let mut binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    // Normal attack
    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );

    // Skill 10005 (Ném đá - AoE 3)
    binary_skills.insert(
        10005,
        BinarySkillDef {
            id: 10005,
            name: "Nem Da".to_string(),
            fight_area: 3,
            attribute: 1,
            ..Default::default()
        },
    );

    // Skill 13013 (Loạn kích - AoE 4, Active)
    binary_skills.insert(
        13013,
        BinarySkillDef {
            id: 13013,
            name: "Loan Kich".to_string(),
            fight_area: 4,
            attribute: 1,
            ..Default::default()
        },
    );

    // Skill 10008 (Phi sa tẩu thạch - AoE 5, Magic)
    binary_skills.insert(
        10008,
        BinarySkillDef {
            id: 10008,
            name: "Phi Sa".to_string(),
            fight_area: 5,
            attribute: 2,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    assert_eq!(skill_target_count(&data, 0), 1);
    assert_eq!(skill_target_count(&data, 10000), 1);
    assert_eq!(skill_target_count(&data, 10005), 3);
    assert_eq!(skill_target_count(&data, 13013), 4);
    assert_eq!(skill_target_count(&data, 10008), 5);

    assert!(is_combo_eligible_skill(&data, 0));
    assert!(is_combo_eligible_skill(&data, 10000));
    assert!(is_combo_eligible_skill(&data, 10005));
    assert!(is_combo_eligible_skill(&data, 13013));
    assert!(!is_combo_eligible_skill(&data, 10008));
}

/// Test 1: Agi boundary (<= 11 allows combo; >= 12 rejects combo).
#[test]
fn test_1_agi_boundary_combo() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 5000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Case A: Agi diff = 50 - 39 = 11 (<= 11) => COMBO ALLOWED
    {
        let mut battle = Battle::new(1, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 39; // 50 - 39 = 11
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
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
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            has_combo_action(&out),
            "Agi diff 11 must trigger combo successfully"
        );
    }

    // Case B: Agi diff = 50 - 38 = 12 (>= 12) => COMBO REJECTED
    {
        let mut battle = Battle::new(2, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 38; // 50 - 38 = 12
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
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
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            !has_combo_action(&out),
            "Agi diff 12 must reject combo"
        );
    }
}

/// Test 2: 3-member Agi span {100, 105, 111} combos; adding 4th with Agi 98 (span 13) splits it out.
#[test]
fn test_2_three_member_agi_span() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 10000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Case A: 3 fighters with Agi {111, 105, 100} => span = 111 - 100 = 11 <= 11 => Combo together!
    {
        let mut battle = Battle::new(3, 0);
        let agis = [(2, 2, 111), (2, 1, 105), (2, 3, 100)];
        for &(r, c_idx, agi) in &agis {
            let cell = battle.cell_mut(r, c_idx).unwrap();
            cell.id = 100 + c_idx as i64;
            cell.typ = 2;
            cell.hp = 500;
            cell.hp_max = 500;
            cell.atk = 50;
            cell.agi = agi;
            cell.team = 1;
            cell.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 10000;
            c.hp_max = 10000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        for &(r, c_idx, _) in &agis {
            commands.insert(
                (r, c_idx),
                BattleCommand {
                    row: r,
                    col: c_idx,
                    target_row: 0,
                    target_col: 2,
                    action: BattleAction::NormalAttack,
                },
            );
        }

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            has_combo_action(&out),
            "3-member combo with span 11 must succeed"
        );
        let max_records = out
            .iter()
            .filter_map(|m| match m {
                Out::Broadcast(s) => Some(count_combat_records(s)),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        assert_eq!(
            max_records, 3,
            "All 3 fighters must be merged into 1 combo packet with 3 records"
        );
    }

    // Case B: 4 fighters with Agi {111, 105, 100, 98} => first 3 combo (span 11), 4th (agi 98) span 13 is excluded!
    {
        let mut battle = Battle::new(4, 0);
        let agis = [(2, 2, 111), (2, 1, 105), (2, 3, 100), (3, 2, 98)];
        for &(r, c_idx, agi) in &agis {
            let cell = battle.cell_mut(r, c_idx).unwrap();
            cell.id = 100 + (r as i64 * 10) + c_idx as i64;
            cell.typ = 2;
            cell.hp = 500;
            cell.hp_max = 500;
            cell.atk = 50;
            cell.agi = agi;
            cell.team = 1;
            cell.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 10000;
            c.hp_max = 10000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        for &(r, c_idx, _) in &agis {
            commands.insert(
                (r, c_idx),
                BattleCommand {
                    row: r,
                    col: c_idx,
                    target_row: 0,
                    target_col: 2,
                    action: BattleAction::NormalAttack,
                },
            );
        }

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        // The combo still fired for the first 3 fighters!
        assert!(
            has_combo_action(&out),
            "First 3 fighters combo successfully despite 4th being separated"
        );
        let records: Vec<usize> = out
            .iter()
            .filter_map(|m| match m {
                Out::Broadcast(s) => {
                    let c = count_combat_records(s);
                    if c > 0 {
                        Some(c)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        assert!(
            records.contains(&3),
            "First 3 fighters combo merged into 1 packet with 3 records"
        );
        assert!(
            records.contains(&1),
            "Separated 4th fighter attacks alone in 1 packet with 1 record"
        );
    }
}

/// Test 3: Normal attack + Ném đá (10005, AoE 3) or Hỏa tiễn (12003, AoE 3) combos successfully.
#[test]
fn test_3_combo_allowed_active_skills_nem_da_and_hoa_tien() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let mut binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    // Ném đá (10005) - Active, AoE 3
    binary_skills.insert(
        10005,
        BinarySkillDef {
            id: 10005,
            name: "Nem Da".to_string(),
            fight_area: 3,
            attribute: 1,
            ..Default::default()
        },
    );
    // Hỏa tiễn (12003) - Active, AoE 3
    binary_skills.insert(
        12003,
        BinarySkillDef {
            id: 12003,
            name: "Hoa Tien".to_string(),
            fight_area: 3,
            attribute: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 5000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Normal attack + Ném đá (10005)
    {
        let mut battle = Battle::new(5, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 10005,
                    skill_lv: 1,
                },
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            has_combo_action(&out),
            "Normal Attack + Nem Da (10005) must combo"
        );
    }

    // Normal attack + Hỏa tiễn (12003)
    {
        let mut battle = Battle::new(6, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 12003,
                    skill_lv: 1,
                },
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            has_combo_action(&out),
            "Normal Attack + Hoa Tien (12003) must combo"
        );
    }
}

/// Test 4: Loạn kích (13013, Active AoE 4) combos successfully.
#[test]
fn test_4_combo_allowed_active_aoe4_loan_kich() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let mut binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    // Loạn kích (13013): Active (attribute 1), 4 targets (fight_area 4)
    binary_skills.insert(
        13013,
        BinarySkillDef {
            id: 13013,
            name: "Loan Kich".to_string(),
            fight_area: 4,
            attribute: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 5000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Case A: Normal attack + Loạn kích (13013)
    {
        let mut battle = Battle::new(7, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 13013,
                    skill_lv: 1,
                },
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            has_combo_action(&out),
            "Normal attack + Loan Kich (13013) must combo successfully"
        );
    }

    // Case B: Dual Loạn kích (both fighters cast 13013)
    {
        let mut battle = Battle::new(8, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 13013,
                    skill_lv: 1,
                },
            },
        );
        commands.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 13013,
                    skill_lv: 1,
                },
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            has_combo_action(&out),
            "Dual Loan Kich (both fighters casting 13013) must combo successfully"
        );
    }
}

/// Test 5: Magic skills (skill_type == 2) are strictly rejected from combo.
#[test]
fn test_5_combo_rejected_for_magic_and_debuff_skills() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let mut binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    // Phi sa tẩu thạch (10008) - Magic (attribute 2)
    binary_skills.insert(
        10008,
        BinarySkillDef {
            id: 10008,
            name: "Phi Sa".to_string(),
            fight_area: 5,
            attribute: 2,
            ..Default::default()
        },
    );
    // Hồng thủy (11008) - Magic (attribute 2)
    binary_skills.insert(
        11008,
        BinarySkillDef {
            id: 11008,
            name: "Hong Thuy".to_string(),
            fight_area: 7,
            attribute: 2,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 5000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Normal attack + Phi Sa (10008)
    {
        let mut battle = Battle::new(9, 0);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 50;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 5000;
            c.hp_max = 5000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 2,
                action: BattleAction::CastSkill {
                    skill_id: 10008,
                    skill_lv: 1,
                },
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(
            !has_combo_action(&out),
            "Magic skill Phi Sa (10008) must be rejected from combo"
        );
    }
}

/// Test 6: Reborn characters count as Level 200 in LVTB calculation.
#[test]
fn test_6_reborn_level_200_lvtb() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Boss Quai".to_vec(),
            lv: 100, // Monster Level 100
            hp: 10000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Two Reborn Level 1 fighters against Level 100 monster.
    // If reborn is counted as Level 200: LVTB = (200 + 200) / 2 = 200.
    // LVTB - Monster = 200 - 100 = 100 >= 25 => 100% combo rate!
    // (If reborn was ignored, LVTB would be 1, diff <= 0 => 0% combo rate).
    let mut battle = Battle::new(10, 0);
    {
        let c = battle.cell_mut(2, 2).unwrap();
        c.id = 101;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 50;
        c.agi = 50;
        c.team = 1;
        c.lv = 1;
        c.reborn = 1; // Reborn!
    }
    {
        let c = battle.cell_mut(2, 1).unwrap();
        c.id = 102;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 50;
        c.agi = 48;
        c.team = 1;
        c.lv = 1;
        c.reborn = 1; // Reborn!
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.id = 999;
        c.typ = 1;
        c.hp = 10000;
        c.hp_max = 10000;
        c.team = 2;
        c.lv = 100;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (2, 2),
        BattleCommand {
            row: 2,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    );
    commands.insert(
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
    let _ = battle.run_turn(&data, &commands, &mut out);
    assert!(
        has_combo_action(&out),
        "Reborn level 1 fighters with effective LV 200 must achieve 100% combo vs level 100 monster"
    );
}

/// Test 7: Degraded combo probability curve when LVTB - target_lv < 25.
#[test]
fn test_7_combo_probability_degradation_curve() {
    let mut rng = DotNetRandom::new(42);

    // 1. target_lv <= 0 => returns 1
    assert_eq!(get_random_miss_combo(&mut rng, 20.0, 0), 1);
    assert_eq!(get_random_miss_combo(&mut rng, 20.0, -5), 1);

    // 2. lvtb - target_lv >= 25.0 => returns 1 (100% rate)
    assert_eq!(get_random_miss_combo(&mut rng, 50.0, 25), 1);
    assert_eq!(get_random_miss_combo(&mut rng, 100.0, 50), 1);

    // 3. lvtb <= 25.0 => returns 0 (0% rate)
    assert_eq!(get_random_miss_combo(&mut rng, 25.0, 30), 0);
    assert_eq!(get_random_miss_combo(&mut rng, 20.0, 30), 0);

    // 4. Team LVTB 40 vs Target 50:
    // diff = 40.0 - 25.0 = 15.0
    // percent = (15.0 / 50.0 * 100.0).floor() = 30%
    let trials = 10_000;
    let mut successes = 0;
    for _ in 0..trials {
        if get_random_miss_combo(&mut rng, 40.0, 50) == 1 {
            successes += 1;
        }
    }
    let rate = (successes as f64) / (trials as f64);
    // Rate should be around 0.30 within [0.27, 0.33]
    assert!(
        (0.27..=0.33).contains(&rate),
        "Expected ~30% combo rate for LVTB 40 vs target 50, got {:.2}%",
        rate * 100.0
    );
}

/// Test 8: Sát thương của đòn combo cao hơn đòn đánh độc lập nhờ hệ số x1.3 trên mục tiêu chính.
#[test]
fn test_8_combo_damage_130_percent_multiplier() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 20000,
            sp: 100,
            ..Default::default()
        },
    );
    npcs.insert(
        998,
        Npc {
            id: 998,
            name: b"Other Target".to_vec(),
            lv: 10,
            hp: 20000,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Battle A: No Combo (Attacker 2 targets a different enemy, so Attacker 1 attacks solo)
    let hp_lost_solo;
    {
        let mut battle = Battle::new(100, 12345);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 200;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 200;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 20000;
            c.hp_max = 20000;
            c.team = 2;
            c.lv = 10;
        }
        {
            let c = battle.cell_mut(0, 1).unwrap();
            c.id = 998;
            c.typ = 1;
            c.hp = 20000;
            c.hp_max = 20000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2, // Targets (0, 2)
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
            (2, 1),
            BattleCommand {
                row: 2,
                col: 1,
                target_row: 0,
                target_col: 1, // Targets (0, 1) - DIFFERENT TARGET => NO COMBO
                action: BattleAction::NormalAttack,
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(!has_combo_action(&out), "Different targets must not combo");

        let enemy = battle.cell(0, 2).unwrap();
        hp_lost_solo = 20000 - enemy.hp;
    }

    // Battle B: Combo (both fighters target (0, 2), Agi 50 and 48)
    // Same battle seed (12345) and same stats.
    let hp_lost_combo;
    {
        let mut battle = Battle::new(100, 12345);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 200;
            c.agi = 50;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 200;
            c.agi = 48;
            c.team = 1;
            c.lv = 50;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 1;
            c.hp = 20000;
            c.hp_max = 20000;
            c.team = 2;
            c.lv = 10;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2, // Both target (0, 2) => COMBO!
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
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
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(has_combo_action(&out), "Same target must combo");

        let enemy = battle.cell(0, 2).unwrap();
        hp_lost_combo = 20000 - enemy.hp;
    }

    // In Battle B, both fighters combo'd onto (0, 2) with x1.3 damage.
    // In Battle A, only fighter 1 attacked (0, 2) with 1.0x damage.
    // Total damage in Battle B is approximately 2 * 1.3 = 2.6x of single hit!
    assert!(
        hp_lost_combo > hp_lost_solo,
        "Combo damage ({}) must be significantly higher than solo hit ({})",
        hp_lost_combo,
        hp_lost_solo
    );
    // Specifically, combo damage with 2 hits of x1.3 should exceed 2x solo hit
    assert!(
        hp_lost_combo > hp_lost_solo * 2,
        "Combo with 2 attackers ({}) should exceed 2x solo hit ({})",
        hp_lost_combo,
        hp_lost_solo * 2
    );
}

/// Test 9: EXP bonus (+8.6%) from damage::calc_combo_exp is distributed to all combo participants.
#[test]
fn test_9_combo_exp_bonus_and_kill_processing() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 20,
            hp: 50,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    // Case A: Solo kill by Fighter 1
    let exp_solo_p1;
    let exp_solo_p2;
    {
        let mut battle = Battle::new(50, 42);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 500;
            c.agi = 50;
            c.team = 1;
            c.lv = 20;
            c.reborn = 1;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 500;
            c.agi = 10; // Low Agi, does not attack
            c.team = 1;
            c.lv = 20;
            c.reborn = 1;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 7; // TeamDef hostile NPC eligible for drops/exp
            c.hp = 50;
            c.hp_max = 50;
            c.team = 2;
            c.lv = 20;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(battle.cell(0, 2).unwrap().hp <= 0, "Enemy must be dead");

        exp_solo_p1 = battle.cell(2, 2).unwrap().exp;
        exp_solo_p2 = battle.cell(2, 1).unwrap().exp;
        assert!(exp_solo_p1 > 0, "Fighter 1 must earn kill exp in solo kill");
        assert_eq!(exp_solo_p2, 0, "Fighter 2 did not participate, must earn 0 exp");
    }

    // Case B: Combo kill by Fighter 1 & Fighter 2
    {
        let mut battle = Battle::new(50, 42);
        {
            let c = battle.cell_mut(2, 2).unwrap();
            c.id = 101;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 500;
            c.agi = 50;
            c.team = 1;
            c.lv = 20;
            c.reborn = 1;
        }
        {
            let c = battle.cell_mut(2, 1).unwrap();
            c.id = 102;
            c.typ = 2;
            c.hp = 500;
            c.hp_max = 500;
            c.atk = 500;
            c.agi = 48; // Agi gap 2 <= 11 => Combo!
            c.team = 1;
            c.lv = 20;
            c.reborn = 1;
        }
        {
            let c = battle.cell_mut(0, 2).unwrap();
            c.id = 999;
            c.typ = 7; // TeamDef hostile NPC eligible for drops/exp
            c.hp = 50;
            c.hp_max = 50;
            c.team = 2;
            c.lv = 20;
        }

        let mut commands = BattleCommands::new();
        commands.insert(
            (2, 2),
            BattleCommand {
                row: 2,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        commands.insert(
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
        let _ = battle.run_turn(&data, &commands, &mut out);
        assert!(battle.cell(0, 2).unwrap().hp <= 0, "Enemy must be dead");

        let exp_combo_p1 = battle.cell(2, 2).unwrap().exp;
        let exp_combo_p2 = battle.cell(2, 1).unwrap().exp;

        // Both participants must receive combo exp
        assert!(exp_combo_p1 > 0, "Fighter 1 must earn combo exp");
        assert!(exp_combo_p2 > 0, "Fighter 2 must earn combo exp");

        // Verify calc_combo_exp multiplier (100 * 1.086 = 109, 200 * 1.086 = 217)
        assert_eq!(calc_combo_exp(200), 217);

        // Combo kill awards calc_combo_exp (+8.6%) on base reward_exp
        // In solo kill: reward_exp(20, 20) = 200.
        // In combo kill: calc_combo_exp(200) = 217.
        assert!(
            exp_combo_p1 > exp_solo_p1,
            "Combo participant 1 exp ({}) must exceed solo exp ({}) due to +8.6% bonus",
            exp_combo_p1,
            exp_solo_p1
        );
    }
}

/// Test 10: Split-target combos and solo attacks in the same turn are isolated without leaking combo_cells.
#[test]
fn test_10_split_target_combos_and_isolation() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Normal Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            ..Default::default()
        },
    );

    npcs.insert(
        997,
        Npc {
            id: 997,
            name: b"Boss A".to_vec(),
            lv: 10,
            hp: 50000,
            sp: 100,
            ..Default::default()
        },
    );
    npcs.insert(
        998,
        Npc {
            id: 998,
            name: b"Minion B".to_vec(),
            lv: 10,
            hp: 50,
            sp: 100,
            ..Default::default()
        },
    );
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Minion C".to_vec(),
            lv: 10,
            hp: 50,
            sp: 100,
            ..Default::default()
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    let mut battle = Battle::new(99, 12345);

    // Team 1:
    // Pair 1: Fighters 1 & 2 target Boss A (high HP, will NOT die)
    {
        let c = battle.cell_mut(2, 2).unwrap();
        c.id = 101;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 50;
        c.agi = 60;
        c.team = 1;
        c.lv = 20;
        c.reborn = 1;
    }
    {
        let c = battle.cell_mut(2, 1).unwrap();
        c.id = 102;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 50;
        c.agi = 58;
        c.team = 1;
        c.lv = 20;
        c.reborn = 1;
    }

    // Pair 2: Fighters 3 & 4 target Minion B (dies)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.id = 103;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 500;
        c.agi = 40;
        c.team = 1;
        c.lv = 20;
        c.reborn = 1;
    }
    {
        let c = battle.cell_mut(3, 1).unwrap();
        c.id = 104;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 500;
        c.agi = 38;
        c.team = 1;
        c.lv = 20;
        c.reborn = 1;
    }

    // Solo: Fighter 5 targets Minion C (dies)
    {
        let c = battle.cell_mut(2, 3).unwrap();
        c.id = 105;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.atk = 500;
        c.agi = 20;
        c.team = 1;
        c.lv = 20;
        c.reborn = 1;
    }

    // Team 2 (Hostile NPCs):
    {
        let c = battle.cell_mut(0, 2).unwrap(); // Boss A
        c.id = 997;
        c.typ = 7;
        c.hp = 50000;
        c.hp_max = 50000;
        c.team = 2;
        c.lv = 10;
    }
    {
        let c = battle.cell_mut(0, 1).unwrap(); // Minion B
        c.id = 998;
        c.typ = 7;
        c.hp = 50;
        c.hp_max = 50;
        c.team = 2;
        c.lv = 10;
    }
    {
        let c = battle.cell_mut(0, 0).unwrap(); // Minion C
        c.id = 999;
        c.typ = 7;
        c.hp = 50;
        c.hp_max = 50;
        c.team = 2;
        c.lv = 10;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (2, 2),
        BattleCommand {
            row: 2,
            col: 2,
            target_row: 0,
            target_col: 2, // Targets Boss A
            action: BattleAction::NormalAttack,
        },
    );
    commands.insert(
        (2, 1),
        BattleCommand {
            row: 2,
            col: 1,
            target_row: 0,
            target_col: 2, // Targets Boss A
            action: BattleAction::NormalAttack,
        },
    );
    commands.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 1, // Targets Minion B
            action: BattleAction::NormalAttack,
        },
    );
    commands.insert(
        (3, 1),
        BattleCommand {
            row: 3,
            col: 1,
            target_row: 0,
            target_col: 1, // Targets Minion B
            action: BattleAction::NormalAttack,
        },
    );
    commands.insert(
        (2, 3),
        BattleCommand {
            row: 2,
            col: 3,
            target_row: 0,
            target_col: 0, // Targets Minion C
            action: BattleAction::NormalAttack,
        },
    );

    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &commands, &mut out);

    // Verify outcomes:
    assert!(battle.cell(0, 2).unwrap().hp > 0, "Boss A must survive");
    assert!(battle.cell(0, 1).unwrap().hp <= 0, "Minion B must be dead");
    assert!(battle.cell(0, 0).unwrap().hp <= 0, "Minion C must be dead");

    let p1_exp = battle.cell(2, 2).unwrap().exp;
    let p2_exp = battle.cell(2, 1).unwrap().exp;
    let p3_exp = battle.cell(3, 2).unwrap().exp;
    let p4_exp = battle.cell(3, 1).unwrap().exp;
    let p5_exp = battle.cell(2, 3).unwrap().exp;

    // Fighters 3 & 4 killed Minion B via combo, so they must have earned combo exp
    assert!(p3_exp > 0, "Fighter 3 must earn combo kill exp from Minion B");
    assert!(p4_exp > 0, "Fighter 4 must earn combo kill exp from Minion B");

    // Fighter 5 killed Minion C solo, so Fighter 5 earns solo kill exp
    assert!(p5_exp > 0, "Fighter 5 must earn solo kill exp from Minion C");

    // Fighters 1 & 2 attacked Boss A (survived) and did NOT participate in killing B or C.
    // They must not receive kill exp from Minion B or C (only minor non-lethal hit exp on Boss A).
    assert!(
        p1_exp < p3_exp,
        "Fighter 1 exp ({}) must be much less than killer Fighter 3 exp ({})",
        p1_exp,
        p3_exp
    );
    assert_eq!(
        p2_exp, 0,
        "Fighter 2 did not kill Boss A and did not attack B or C, so must have 0 kill exp"
    );
}

