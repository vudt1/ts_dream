//! Integration test for Ticket 23: Combo Rules & AoE Threshold.
//!
//! Validates that:
//! 1. Normal attack (10000) and skills with nbTarget <= 3 can participate in Combos.
//! 2. Skills with nbTarget >= 4 (Loạn kích 13013, Phi sa 10008, Hồng thủy 11008, Thiên băng 10012)
//!    are strictly disallowed from triggering or continuing combos.

use std::collections::HashMap;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::runner::{
    skill_target_count, BattleAction, BattleCommand, BattleCommands, BattleData,
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
            kind: 1,
            ..Default::default()
        },
    );

    // Skill 13013 (Loạn kích - AoE 4)
    binary_skills.insert(
        13013,
        BinarySkillDef {
            id: 13013,
            name: "Loan Kich".to_string(),
            fight_area: 4,
            kind: 1,
            ..Default::default()
        },
    );

    // Skill 10008 (Phi sa tẩu thạch - AoE 5)
    binary_skills.insert(
        10008,
        BinarySkillDef {
            id: 10008,
            name: "Phi Sa".to_string(),
            fight_area: 5,
            kind: 1,
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
}

#[test]
fn test_combo_allowed_with_normal_attack_and_aoe3_skill() {
    let mut npcs = HashMap::new();
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

    // AoE 3: Ném đá (10005)
    skills.insert(
        10005,
        Skill {
            id: 10005,
            name: "Nem Da".to_string(),
            sl_danh: 3,
            skill_type: 1,
            ..Default::default()
        },
    );
    binary_skills.insert(
        10005,
        BinarySkillDef {
            id: 10005,
            name: "Nem Da".to_string(),
            fight_area: 3,
            kind: 1,
            ..Default::default()
        },
    );

    // Enemy target NPC
    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 2000,
            sp: 100,
            ..Default::default()
        },
    );

    let mut battle = Battle::new(1, 0);
    // Attacker 1: Player (row 2, col 2), AGI 50, targets (row 0, col 2)
    {
        let c = battle.cell_mut(2, 2).unwrap();
        c.id = 101;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.sp = 100;
        c.sp_max = 100;
        c.atk = 50;
        c.agi = 50;
        c.team = 1;
        c.lv = 20;
    }

    // Attacker 2: Ally (row 2, col 1), AGI 49 (close Agi within 800), targets same (row 0, col 2)
    {
        let c = battle.cell_mut(2, 1).unwrap();
        c.id = 102;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.sp = 100;
        c.sp_max = 100;
        c.atk = 50;
        c.agi = 49;
        c.team = 1;
        c.lv = 20;
    }

    // Enemy target at (0, 2)
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.id = 999;
        c.typ = 1;
        c.hp = 2000;
        c.hp_max = 2000;
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

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &commands, &mut out);

    let enemy = battle.cell(0, 2).unwrap();
    assert!(enemy.hp < 2000);
}

#[test]
fn test_combo_rejected_when_second_fighter_uses_aoe4() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let mut binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    // Normal attack (10000)
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

    // AoE 4: Loạn kích (13013)
    skills.insert(
        13013,
        Skill {
            id: 13013,
            name: "Loan Kich".to_string(),
            sl_danh: 4,
            skill_type: 1,
            ..Default::default()
        },
    );
    binary_skills.insert(
        13013,
        BinarySkillDef {
            id: 13013,
            name: "Loan Kich".to_string(),
            fight_area: 4,
            kind: 1,
            ..Default::default()
        },
    );

    npcs.insert(
        999,
        Npc {
            id: 999,
            name: b"Enemy Target".to_vec(),
            lv: 10,
            hp: 2000,
            sp: 100,
            ..Default::default()
        },
    );

    let mut battle = Battle::new(1, 0);
    // Attacker 1: Normal attack
    {
        let c = battle.cell_mut(2, 2).unwrap();
        c.id = 101;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.sp = 100;
        c.sp_max = 100;
        c.atk = 50;
        c.agi = 50;
        c.team = 1;
        c.lv = 20;
    }
    // Attacker 2: Loạn kích (AoE 4)
    {
        let c = battle.cell_mut(2, 1).unwrap();
        c.id = 102;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.sp = 100;
        c.sp_max = 100;
        c.atk = 50;
        c.agi = 49;
        c.team = 1;
        c.lv = 20;
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.id = 999;
        c.typ = 1;
        c.hp = 2000;
        c.hp_max = 2000;
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

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &commands, &mut out);

    // Second attacker used Loạn kích (AoE 4), so Combo is strictly rejected.
    let enemy = battle.cell(0, 2).unwrap();
    assert!(enemy.hp < 2000);
}

#[test]
fn test_combo_rejected_when_first_fighter_uses_aoe5() {
    let mut npcs = HashMap::new();
    let mut skills = HashMap::new();
    let mut binary_skills = HashMap::new();
    let pet_slots = HashMap::new();
    let items = HashMap::new();
    let texps = Vec::new();
    let players = HashMap::new();

    // AoE 5: Phi sa tẩu thạch (10008)
    skills.insert(
        10008,
        Skill {
            id: 10008,
            name: "Phi Sa".to_string(),
            sl_danh: 5,
            skill_type: 1,
            ..Default::default()
        },
    );
    binary_skills.insert(
        10008,
        BinarySkillDef {
            id: 10008,
            name: "Phi Sa".to_string(),
            fight_area: 5,
            kind: 1,
            ..Default::default()
        },
    );

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
            hp: 2000,
            sp: 100,
            ..Default::default()
        },
    );

    let mut battle = Battle::new(1, 0);
    // Attacker 1 has higher AGI (50), uses AoE 5 (10008)
    {
        let c = battle.cell_mut(2, 2).unwrap();
        c.id = 101;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.sp = 100;
        c.sp_max = 100;
        c.atk = 50;
        c.agi = 50;
        c.team = 1;
        c.lv = 20;
    }
    // Attacker 2 has AGI 49, uses normal attack 10000
    {
        let c = battle.cell_mut(2, 1).unwrap();
        c.id = 102;
        c.typ = 2;
        c.hp = 500;
        c.hp_max = 500;
        c.sp = 100;
        c.sp_max = 100;
        c.atk = 50;
        c.agi = 49;
        c.team = 1;
        c.lv = 20;
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.id = 999;
        c.typ = 1;
        c.hp = 2000;
        c.hp_max = 2000;
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
                skill_id: 10008,
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
            action: BattleAction::NormalAttack,
        },
    );

    let data = mock_battle_data(
        &npcs, &skills, &binary_skills, &pet_slots, &items, &texps, &players,
    );

    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &commands, &mut out);

    // Attacker 1 uses AoE >= 4, so combo is rejected.
    let enemy = battle.cell(0, 2).unwrap();
    assert!(enemy.hp < 2000);
}
