//! Integration tests for Battle Dispel & Cleanse Matrix Parity
//! (Tickets: 11, 12, 13, 14; Feature: `battle-dispel-cleanse-matrix-parity`).
//!
//! Validates:
//! 1. Archetype A: Specific status counters (11015 thaw, 14007 wake, 14014 poison cure,
//!    14022 confusion cure, 10009 barrier dispel, 10014 mirror dispel) with negative MISS paths.
//! 2. Archetype B: Mass cleanse (11012, 11025, 11031, 22005) with friendly-only boundary,
//!    clean target rejection, and strict Type 4 / Type 19 buff preservation.
//! 3. Archetype C: Cleanse & heal (Type 18) dynamic skill-level scaling, clean-target healing,
//!    and SP Infinite Loop Gate (caster receives 0 SP on self-targeting).
//! 4. Delphi wire frame sequence: 0x35 0x01 purge -> 0x35 0x0F HUD clear -> 0x32 0x01 action.

use std::collections::HashMap;
use std::sync::LazyLock;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::dispel::{
    self, SKILL_BANG_TAN, SKILL_BANG_TAN_1, SKILL_CUONG_HIEU_GIAI_TRU, SKILL_GIAI_DOC,
    SKILL_GIAI_HON, SKILL_GIAI_HON_LOAN, SKILL_GIAI_KET_GIOI, SKILL_GIAI_THOAT,
    SKILL_GIAI_TRU_KINH, SKILL_SIEU_TRONG_GIAI, SKILL_THUC_TINH_1,
    SKILL_TIEN_GIAI_TRONG_GIAI, SKILL_TRANG_THAI_GIAI_TRU, SKILL_TU_NHAN_TRANG_GIAI,
};
use ts_dream::battle::packets::{self, miss_status, troi_byte};
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, Out, PlayerSnapshot,
};
use ts_dream::data::tables::{BinarySkillDef, Item, Npc, Skill, TexpRow};

static EMPTY_ITEMS: LazyLock<HashMap<i64, Item>> = LazyLock::new(HashMap::new);

fn create_test_data<'a>(
    npcs: &'a HashMap<i64, Npc>,
    skills: &'a HashMap<i64, Skill>,
    pet_slots: &'a HashMap<i64, [i64; 4]>,
    players: &'a HashMap<i64, PlayerSnapshot>,
    texps: &'a [TexpRow],
    binary_skills: &'a HashMap<u16, BinarySkillDef>,
) -> BattleData<'a> {
    BattleData::new(npcs, skills, &EMPTY_ITEMS, pet_slots, players, texps, None, 0, 0)
        .with_binary_skills(binary_skills)
}

fn binary_skill(id: u16, attribute: u8, fight_area: u8) -> BinarySkillDef {
    BinarySkillDef {
        id,
        name: format!("Skill {id}"),
        kind: 1,
        require_sp: 0,
        element: 1,
        numerical: 150,
        attribute,
        fight_area,
        round: 0,
        hit_status: 0,
        how_much_times: 1,
        max_lv: 10,
        ..Default::default()
    }
}

fn command(row: u8, col: u8, target_row: u8, target_col: u8, action: BattleAction) -> BattleCommand {
    BattleCommand {
        row,
        col,
        target_row,
        target_col,
        action,
    }
}

fn broadcast_frames(out: &[Out]) -> Vec<String> {
    out.iter()
        .filter_map(|o| match o {
            Out::Broadcast(frame) => Some(frame.clone()),
            _ => None,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn seed_unit(
    battle: &mut Battle,
    row: u8,
    col: u8,
    id: i64,
    team: i64,
    lv: i64,
    hp: i64,
    hp_max: i64,
    sp: i64,
    sp_max: i64,
    int1: i64,
) {
    let cell = battle.cell_mut(row, col).expect("cell exists");
    cell.typ = 2; // player-kind
    cell.id = id;
    cell.team = team;
    cell.lv = lv;
    cell.hp = hp;
    cell.hp_max = hp_max;
    cell.sp = sp;
    cell.sp_max = sp_max;
    cell.atk = 200;
    cell.def = 100;
    cell.agi = 50;
    cell.int1 = int1;
    cell.thuoctinh = 1;
    cell.row = row;
    cell.col = col;
}

fn run(battle: &mut Battle, data: &BattleData<'_>, commands: BattleCommands) -> Vec<String> {
    let mut out = Vec::new();
    battle.run_turn(data, &commands, &mut out);
    broadcast_frames(&out)
}

// ═══════════════════════════════════════════════════════════════════════════
// TICKET 11: Specific Status Counters (Archetype A)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_specific_dispel_thaw_freeze() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_BANG_TAN as u16, binary_skill(SKILL_BANG_TAN as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(1, 0, 42, 7, 99);
    // Team 1: Caster at (3, 2), Frozen Ally at (3, 1)
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    // Team 2: Idle enemy at (0, 2)
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    {
        let ally = battle.cell_mut(3, 1).unwrap();
        ally.type3_id = 11014; // Băng phong
        ally.type3_lv = 1;
        ally.type3_turn = 4;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_BANG_TAN, skill_lv: 1 }),
    );

    let frames = run(&mut battle, &data, commands);

    // Verify cell state: Freeze completely removed
    let ally = battle.cell(3, 1).unwrap();
    assert_eq!(ally.type3_id, 0, "Freeze type3_id must be reset to 0");
    assert_eq!(ally.type3_turn, 0, "Freeze type3_turn must be reset to 0");
    assert_eq!(ally.type3_lv, 0, "Freeze type3_lv must be reset to 0");

    // Verify Delphi wire frames: purge disable FX (0x35 0x01) followed by HUD clear (0x35 0x0F)
    let purge_frame = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    let clear_frame = packets::clear_unit_status(3, 1, packets::STATUS_SEAL_RESIST_PCT);
    assert!(frames.contains(&purge_frame), "Must emit PURGE_TYPE_DISABLE frame: {purge_frame}");
    assert!(frames.contains(&clear_frame), "Must emit clear_unit_status for seal resist: {clear_frame}");

    // Verify purge occurs before clear
    let purge_idx = frames.iter().position(|f| f == &purge_frame).unwrap();
    let clear_idx = frames.iter().position(|f| f == &clear_frame).unwrap();
    assert!(purge_idx < clear_idx, "Purge FX must precede HUD clear");

    // Verify action frame emits troi_byte::TYPE3 (0xDD) for Type 3 thaw
    let act_thaw = packets::skilling_int(3, 1, miss_status::ATTACK, 0, 1, troi_byte::TYPE3, 0, 1);
    assert!(frames.iter().any(|f| f.contains(&act_thaw)), "Must emit TYPE3 troi_byte in action frame");
}

#[test]
fn test_specific_dispel_wake_sleep() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_GIAI_HON as u16, binary_skill(SKILL_GIAI_HON as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(2, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    {
        let ally = battle.cell_mut(3, 1).unwrap();
        ally.type3_id = 14008; // Hôn mê
        ally.type3_lv = 1;
        ally.type3_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_GIAI_HON, skill_lv: 1 }),
    );

    let frames = run(&mut battle, &data, commands);

    let ally = battle.cell(3, 1).unwrap();
    assert_eq!(ally.type3_id, 0, "Sleep type3_id must be cured to 0");
    let purge_frame = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    let clear_frame = packets::clear_unit_status(3, 1, packets::STATUS_SEAL_RESIST_PCT);
    assert!(frames.contains(&purge_frame));
    assert!(frames.contains(&clear_frame));
}

#[test]
fn test_specific_dispel_cure_poison_and_confusion() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_GIAI_DOC as u16, binary_skill(SKILL_GIAI_DOC as u16, 5, 1));
    binary_skills.insert(SKILL_GIAI_HON_LOAN as u16, binary_skill(SKILL_GIAI_HON_LOAN as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Test 1: Cure Poison (14014 cures 14015)
    let mut battle = Battle::with_seeds(3, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle.cell_mut(3, 1).unwrap();
        ally.type15_id = 14015; // Bỏ độc
        ally.type15_lv = 1;
        ally.type15_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_GIAI_DOC, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let ally = battle.cell(3, 1).unwrap();
    assert_eq!(ally.type15_id, 0, "Poison type15_id must be cured to 0");
    let purge_debuff = packets::purge_status(3, 1, packets::PURGE_TYPE_DEBUFF);
    let clear_def = packets::clear_unit_status(3, 1, packets::STATUS_DEF_MODIFIER);
    let clear_atk = packets::clear_unit_status(3, 1, packets::STATUS_ATK_MODIFIER);
    assert!(frames.contains(&purge_debuff));
    assert!(frames.contains(&clear_def));
    assert!(frames.contains(&clear_atk));
    let act_poison = packets::skilling_int(3, 1, miss_status::ATTACK, 0, 1, troi_byte::TYPE15, 0, 1);
    assert!(frames.iter().any(|f| f.contains(&act_poison)), "Must emit TYPE15 troi_byte in action frame");

    // Test 2: Cure Confusion (14022 cures 14021)
    let mut battle2 = Battle::with_seeds(4, 0, 42, 7, 99);
    seed_unit(&mut battle2, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle2, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle2, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle2.cell_mut(3, 1).unwrap();
        ally.type15_id = 14021; // Hỗn loạn
        ally.type15_lv = 1;
        ally.type15_turn = 3;
    }

    let mut commands2 = BattleCommands::new();
    commands2.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_GIAI_HON_LOAN, skill_lv: 1 }),
    );
    let frames2 = run(&mut battle2, &data, commands2);

    let ally2 = battle2.cell(3, 1).unwrap();
    assert_eq!(ally2.type15_id, 0, "Confusion type15_id must be cured to 0");
    assert!(frames2.contains(&purge_debuff));
    let act_conf = packets::skilling_int(3, 1, miss_status::ATTACK, 0, 1, troi_byte::TYPE15, 0, 1);
    assert!(frames2.iter().any(|f| f.contains(&act_conf)), "Must emit TYPE15 troi_byte for confusion cure");

    // Test 3: Rejection of Advanced Confusion 20014 by 14022 (returns MISS, untouched)
    let mut battle3 = Battle::with_seeds(44, 0, 42, 7, 99);
    seed_unit(&mut battle3, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle3, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle3, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle3.cell_mut(3, 1).unwrap();
        ally.type15_id = 20014; // Advanced confusion
        ally.type15_lv = 1;
        ally.type15_turn = 3;
    }

    let mut commands3 = BattleCommands::new();
    commands3.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_GIAI_HON_LOAN, skill_lv: 1 }),
    );
    let frames3 = run(&mut battle3, &data, commands3);

    let ally3 = battle3.cell(3, 1).unwrap();
    assert_eq!(ally3.type15_id, 20014, "Advanced confusion 20014 must NOT be cured by 14022");
    assert_eq!(ally3.type15_turn, 2, "Only decremented by round-start");
    assert!(!frames3.contains(&purge_debuff), "Must not emit purge frames when 14022 misses on 20014");
    let miss_frame = packets::skilling_int(3, 1, miss_status::MISS, 0, 1, troi_byte::MISS, 0, 1);
    assert!(frames3.iter().any(|f| f.contains(&miss_frame)), "Must emit MISS action frame with troi_byte::MISS");
}

#[test]
fn test_specific_dispel_hostile_barrier_and_mirror() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_GIAI_KET_GIOI as u16, binary_skill(SKILL_GIAI_KET_GIOI as u16, 16, 1));
    binary_skills.insert(SKILL_GIAI_TRU_KINH as u16, binary_skill(SKILL_GIAI_TRU_KINH as u16, 16, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Test 1: Giải kết giới (10009) on enemy Kết giới (10010)
    let mut battle = Battle::with_seeds(5, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let enemy = battle.cell_mut(0, 2).unwrap();
        enemy.type4_id = 10010; // Kết giới
        enemy.type4_lv = 2;
        enemy.type4_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: SKILL_GIAI_KET_GIOI, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let enemy = battle.cell(0, 2).unwrap();
    assert_eq!(enemy.type4_id, 0, "Enemy barrier must be dispelled to 0");
    let purge_buff = packets::purge_status(0, 2, packets::PURGE_TYPE_BUFF);
    let clear_def = packets::clear_unit_status(0, 2, packets::STATUS_DEF_MODIFIER);
    assert!(frames.contains(&purge_buff));
    assert!(frames.contains(&clear_def));

    // Test 2: Giải trừ kính (10014) on enemy Kính (10015)
    let mut battle2 = Battle::with_seeds(6, 0, 42, 7, 99);
    seed_unit(&mut battle2, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle2, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let enemy = battle2.cell_mut(0, 2).unwrap();
        enemy.type4_id = 10015; // Kính
        enemy.type4_lv = 2;
        enemy.type4_turn = 3;
    }

    let mut commands2 = BattleCommands::new();
    commands2.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: SKILL_GIAI_TRU_KINH, skill_lv: 1 }),
    );
    let frames2 = run(&mut battle2, &data, commands2);

    let enemy2 = battle2.cell(0, 2).unwrap();
    assert_eq!(enemy2.type4_id, 0, "Enemy mirror must be dispelled to 0");
    assert!(frames2.contains(&purge_buff));
    assert!(frames2.contains(&clear_def));
}

#[test]
fn test_specific_dispel_negative_cases() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_BANG_TAN as u16, binary_skill(SKILL_BANG_TAN as u16, 5, 1));
    binary_skills.insert(SKILL_GIAI_KET_GIOI as u16, binary_skill(SKILL_GIAI_KET_GIOI as u16, 16, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Negative 1: Băng tan cast on clean ally (no freeze) -> returns MISS, 0 purge frames
    let mut battle = Battle::with_seeds(7, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_BANG_TAN, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let miss_target = packets::skilling_int(3, 1, miss_status::MISS, 0, 1, troi_byte::MISS, 0, 1);
    assert!(frames.iter().any(|f| f.contains(&miss_target)), "Must emit MISS status on clean target");
    let purge_any = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    assert!(!frames.contains(&purge_any), "Must not emit purge frames when action misses");

    // Negative 2: Băng tan cast on ally with DIFFERENT debuff (e.g. Cây tinh 10004) -> returns MISS, Cây tinh untouched
    let mut battle2 = Battle::with_seeds(8, 0, 42, 7, 99);
    seed_unit(&mut battle2, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle2, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle2, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle2.cell_mut(3, 1).unwrap();
        ally.type3_id = 10004; // Cây tinh
        ally.type3_lv = 1;
        ally.type3_turn = 3;
    }

    let mut commands2 = BattleCommands::new();
    commands2.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_BANG_TAN, skill_lv: 1 }),
    );
    run(&mut battle2, &data, commands2);

    let ally2 = battle2.cell(3, 1).unwrap();
    assert_eq!(ally2.type3_id, 10004, "Cây tinh must remain intact when Băng tan is cast");
    assert_eq!(ally2.type3_turn, 2, "Only decremented by round-start");

    // Negative 3: Hostile dispel (Giải kết giới 10009) cast on friendly ally carrying barrier -> MISS!
    let mut battle3 = Battle::with_seeds(9, 0, 42, 7, 99);
    seed_unit(&mut battle3, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle3, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle3, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle3.cell_mut(3, 1).unwrap();
        ally.type4_id = 10010; // Kết giới
        ally.type4_lv = 2;
        ally.type4_turn = 3;
    }

    let mut commands3 = BattleCommands::new();
    commands3.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_GIAI_KET_GIOI, skill_lv: 1 }),
    );
    run(&mut battle3, &data, commands3);

    let ally3 = battle3.cell(3, 1).unwrap();
    assert_eq!(ally3.type4_id, 10010, "Friendly barrier must NOT be dispelled by hostile counter");
}

// ═══════════════════════════════════════════════════════════════════════════
// TICKET 12: Mass Cleanse & Friendly Buff Preservation (Archetype B)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_mass_cleanse_buff_preservation() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_TRANG_THAI_GIAI_TRU as u16, binary_skill(SKILL_TRANG_THAI_GIAI_TRU as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(10, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    // Ally carries:
    // - Type 3: Freeze (11014)
    // - Type 15: Poison (14015)
    // - Type 4: Mirror (10015) -> MUST BE PRESERVED!
    // - Type 19: Bá ý (20006) -> MUST BE PRESERVED!
    {
        let ally = battle.cell_mut(3, 1).unwrap();
        ally.type3_id = 11014;
        ally.type3_lv = 1;
        ally.type3_turn = 4;
        ally.type15_id = 14015;
        ally.type15_lv = 1;
        ally.type15_turn = 3;
        ally.type4_id = 10015;
        ally.type4_lv = 2;
        ally.type4_turn = 3;
        ally.type19_id = 20006;
        ally.type19_lv = 1;
        ally.type19_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_TRANG_THAI_GIAI_TRU, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let ally = battle.cell(3, 1).unwrap();
    // Debuffs cleared
    assert_eq!(ally.type3_id, 0, "Type 3 debuff must be cleansed");
    assert_eq!(ally.type15_id, 0, "Type 15 debuff must be cleansed");
    // Protective buffs preserved!
    assert_eq!(ally.type4_id, 10015, "Friendly Kính (Mirror) must remain intact!");
    assert_eq!(ally.type4_turn, 2, "Mirror loses only round-start decrement");
    assert_eq!(ally.type19_id, 20006, "Job specialization aura must remain intact!");
    assert_eq!(ally.type19_turn, 2, "Aura loses only round-start decrement");

    // Purge frames for Type 3 and Type 15 emitted
    let purge_disable = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    let purge_debuff = packets::purge_status(3, 1, packets::PURGE_TYPE_DEBUFF);
    assert!(frames.contains(&purge_disable));
    assert!(frames.contains(&purge_debuff));

    // NO purge frames emitted for Type 4 or Type 19 on ally (3, 1)
    let purge_buff = packets::purge_status(3, 1, packets::PURGE_TYPE_BUFF);
    let purge_aura = packets::purge_status(3, 1, packets::PURGE_TYPE_AURA);
    assert!(!frames.contains(&purge_buff), "Must NOT purge friendly buff!");
    assert!(!frames.contains(&purge_aura), "Must NOT purge friendly aura!");
}

#[test]
fn test_mass_cleanse_enemy_and_clean_ally_rejected() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_TRANG_THAI_GIAI_TRU as u16, binary_skill(SKILL_TRANG_THAI_GIAI_TRU as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Negative 1: Cast Trạng thái giải trừ on enemy unit -> MISS, zero effect
    let mut battle = Battle::with_seeds(11, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let enemy = battle.cell_mut(0, 2).unwrap();
        enemy.type3_id = 11014;
        enemy.type3_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: SKILL_TRANG_THAI_GIAI_TRU, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let enemy = battle.cell(0, 2).unwrap();
    assert_eq!(enemy.type3_id, 11014, "Enemy debuff must NOT be cleansed by friendly mass cleanse");
    let miss_frame = packets::skilling_int(0, 2, miss_status::MISS, 0, 1, troi_byte::MISS, 0, 1);
    assert!(frames.iter().any(|f| f.contains(&miss_frame)));

    // Negative 2: Cast on clean friendly unit -> MISS
    let mut battle2 = Battle::with_seeds(12, 0, 42, 7, 99);
    seed_unit(&mut battle2, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle2, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle2, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    let mut commands2 = BattleCommands::new();
    commands2.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_TRANG_THAI_GIAI_TRU, skill_lv: 1 }),
    );
    let frames2 = run(&mut battle2, &data, commands2);

    let miss_frame2 = packets::skilling_int(3, 1, miss_status::MISS, 0, 1, troi_byte::MISS, 0, 1);
    assert!(frames2.iter().any(|f| f.contains(&miss_frame2)));
}

#[test]
fn test_multi_target_cleanse_coverage_3_and_6_targets() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    // 11025: Tiên Giai Trọng Giải (fight_area = 3)
    binary_skills.insert(SKILL_TIEN_GIAI_TRONG_GIAI as u16, binary_skill(SKILL_TIEN_GIAI_TRONG_GIAI as u16, 5, 3));
    // 11031: Siêu Trọng Giải Thuật (fight_area = 6)
    binary_skills.insert(SKILL_SIEU_TRONG_GIAI as u16, binary_skill(SKILL_SIEU_TRONG_GIAI as u16, 5, 6));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Part 1: Tiên Giai Trọng Giải (11025, 3 targets)
    let mut battle = Battle::with_seeds(13, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle, 3, 4, 103, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    {
        let a1 = battle.cell_mut(3, 1).unwrap();
        a1.type3_id = 11014;
        a1.type3_turn = 3;
        let a2 = battle.cell_mut(3, 4).unwrap();
        a2.type15_id = 14015;
        a2.type15_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 2, BattleAction::CastSkill { skill_id: SKILL_TIEN_GIAI_TRONG_GIAI, skill_lv: 1 }),
    );
    let frames1 = run(&mut battle, &data, commands);

    let a1 = battle.cell(3, 1).unwrap();
    let a2 = battle.cell(3, 4).unwrap();
    assert_eq!(a1.type3_id, 0, "Ally 1 in 3-target area must be cleansed of Freeze");
    assert_eq!(a2.type15_id, 0, "Ally 2 in 3-target area must be cleansed of Poison");

    // Wire verification: count = 2 with DD000001 for ally 1, count = 2 with DF000001 for ally 2
    let act_a1 = format!("{}DD000001", packets::skilling_int(3, 1, miss_status::ATTACK, 0, 2, troi_byte::MISS, 0, 1));
    let act_a2 = format!("{}DF000001", packets::skilling_int(3, 4, miss_status::ATTACK, 0, 2, troi_byte::MISS, 0, 1));
    assert!(frames1.iter().any(|f| f.contains(&act_a1)), "Must emit count=2 and DD000001 for Type 3 cleanse");
    assert!(frames1.iter().any(|f| f.contains(&act_a2)), "Must emit count=2 and DF000001 for Type 15 cleanse");

    // Part 2: Siêu Trọng Giải Thuật (11031, 6 targets) covering full team
    let mut battle6 = Battle::with_seeds(14, 0, 42, 7, 99);
    // Seed 6 allies on team 1: (3, 2) caster, (3, 1), (2, 4), (3, 4), (2, 2), (2, 1)
    seed_unit(&mut battle6, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle6, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle6, 2, 4, 103, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle6, 3, 4, 104, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle6, 2, 2, 105, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle6, 2, 1, 106, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle6, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    // Give various debuffs and keep buffs
    {
        let c1 = battle6.cell_mut(3, 1).unwrap();
        c1.type3_id = 11014; // Freeze
        c1.type3_turn = 3;
        c1.type4_id = 10010; // Barrier preserved!
        c1.type4_turn = 4;

        let c2 = battle6.cell_mut(2, 4).unwrap();
        c2.type15_id = 14015; // Poison
        c2.type15_turn = 3;

        let c3 = battle6.cell_mut(3, 4).unwrap();
        c3.type3_id = 14008; // Sleep
        c3.type3_turn = 3;
        c3.type15_id = 14021; // Confusion
        c3.type15_turn = 3;

        let c4 = battle6.cell_mut(2, 2).unwrap();
        c4.type15_id = 20014; // Advanced confusion
        c4.type15_turn = 3;

        let c5 = battle6.cell_mut(2, 1).unwrap();
        c5.type3_id = 10001; // Stun
        c5.type3_turn = 3;
    }

    let mut commands6 = BattleCommands::new();
    commands6.insert(
        (3, 2),
        command(3, 2, 3, 2, BattleAction::CastSkill { skill_id: SKILL_SIEU_TRONG_GIAI, skill_lv: 1 }),
    );
    let frames6 = run(&mut battle6, &data, commands6);

    // Verify all allies debuffs are cleansed, buffs preserved
    let c1 = battle6.cell(3, 1).unwrap();
    assert_eq!(c1.type3_id, 0);
    assert_eq!(c1.type4_id, 10010, "Friendly barrier preserved across 6-target cleanse!");

    let c2 = battle6.cell(2, 4).unwrap();
    assert_eq!(c2.type15_id, 0);

    let c3 = battle6.cell(3, 4).unwrap();
    assert_eq!(c3.type3_id, 0);
    assert_eq!(c3.type15_id, 0);

    let c4 = battle6.cell(2, 2).unwrap();
    assert_eq!(c4.type15_id, 0);

    let c5 = battle6.cell(2, 1).unwrap();
    assert_eq!(c5.type3_id, 0);

    // For dual-debuff ally (3, 4): count must be 3 with DD000001 and DF000001
    let act_dual = format!("{}DD000001DF000001", packets::skilling_int(3, 4, miss_status::ATTACK, 0, 3, troi_byte::MISS, 0, 1));
    assert!(frames6.iter().any(|f| f.contains(&act_dual)), "Must emit count=3 with both DD and DF for dual debuff target");
}

// ═══════════════════════════════════════════════════════════════════════════
// TICKET 13: Cleanse & Heal Scaling & SP Infinite Loop Gate (Archetype C)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_type18_skill_level_scaling_progression() {
    // Direct domain formula unit test: Cường hiệu giải trừ (20050)
    // Formula: round(int1 * 0.40) + skill_lv * 40 HP, round(int1 * 0.20) + skill_lv * 20 SP
    let mut caster = ts_dream::battle::engine::WarInfo::default();
    caster.id = 101;
    caster.team = 1;
    caster.row = 3;
    caster.col = 2;
    caster.int1 = 150;

    let mut ally_lv1 = ts_dream::battle::engine::WarInfo::default();
    ally_lv1.id = 102;
    ally_lv1.team = 1;
    ally_lv1.row = 3;
    ally_lv1.col = 1;
    ally_lv1.hp = 100;
    ally_lv1.hp_max = 2000;
    ally_lv1.sp = 50;
    ally_lv1.sp_max = 1000;

    let mut ally_lv5 = ally_lv1.clone();

    let outcome_lv1 = dispel::apply_cleanse_and_heal(&caster, &mut ally_lv1, SKILL_CUONG_HIEU_GIAI_TRU, 1);
    let outcome_lv5 = dispel::apply_cleanse_and_heal(&caster, &mut ally_lv5, SKILL_CUONG_HIEU_GIAI_TRU, 5);

    // int1 = 150:
    // Lv 1: round(150 * 0.40) + 1 * 40 = 60 + 40 = 100 HP, round(150 * 0.20) + 1 * 20 = 30 + 20 = 50 SP
    assert_eq!(outcome_lv1.hp_healed, 100);
    assert_eq!(outcome_lv1.sp_restored, 50);

    // Lv 5: round(150 * 0.40) + 5 * 40 = 60 + 200 = 260 HP, round(150 * 0.20) + 5 * 20 = 30 + 100 = 130 SP
    assert_eq!(outcome_lv5.hp_healed, 260);
    assert_eq!(outcome_lv5.sp_restored, 130);

    assert!(outcome_lv5.hp_healed > outcome_lv1.hp_healed, "Lv 5 must heal strictly more HP than Lv 1");
    assert!(outcome_lv5.sp_restored > outcome_lv1.sp_restored, "Lv 5 must restore strictly more SP than Lv 1");

    // Direct domain formula unit test: Tứ nhân trạng giải (20015) & Giải thoát (20019)
    // Formula: round(int1 * 0.35) + skill_lv * 25 HP, round(int1 * 0.15) + skill_lv * 12 SP
    let mut ally_tn_lv1 = ally_lv1.clone();
    let mut ally_tn_lv10 = ally_lv1.clone();
    let outcome_tn_1 = dispel::apply_cleanse_and_heal(&caster, &mut ally_tn_lv1, SKILL_TU_NHAN_TRANG_GIAI, 1);
    let outcome_tn_10 = dispel::apply_cleanse_and_heal(&caster, &mut ally_tn_lv10, SKILL_TU_NHAN_TRANG_GIAI, 10);

    // int1 = 150:
    // Lv 1: round(150 * 0.35) + 1 * 25 = 52 + 25 = 77 HP (banker round 52.5 -> 52), round(150 * 0.15) + 1 * 12 = 22 + 12 = 34 SP
    assert_eq!(outcome_tn_1.hp_healed, 52 + 25);
    assert_eq!(outcome_tn_1.sp_restored, 22 + 12);
    // Lv 10: 52 + 250 = 302 HP, 22 + 120 = 142 SP
    assert_eq!(outcome_tn_10.hp_healed, 52 + 250);
    assert_eq!(outcome_tn_10.sp_restored, 22 + 120);

    // Test Giải thoát (20019) shares same formula
    let mut ally_gt = ally_lv1.clone();
    let outcome_gt = dispel::apply_cleanse_and_heal(&caster, &mut ally_gt, SKILL_GIAI_THOAT, 10);
    assert_eq!(outcome_gt.hp_healed, 52 + 250);
    assert_eq!(outcome_gt.sp_restored, 22 + 120);
}

#[test]
fn test_type18_sp_infinite_loop_gate() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_CUONG_HIEU_GIAI_TRU as u16, binary_skill(SKILL_CUONG_HIEU_GIAI_TRU as u16, 18, 5));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(14, 0, 42, 7, 99);
    // Caster at (3, 2), damaged and missing SP
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 500, 2000, 100, 500, 150);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    // Self-cast Cường hiệu giải trừ
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 2, BattleAction::CastSkill { skill_id: SKILL_CUONG_HIEU_GIAI_TRU, skill_lv: 5 }),
    );
    let frames = run(&mut battle, &data, commands);

    let caster = battle.cell(3, 2).unwrap();
    // HP must be healed (500 + 260 = 760)
    assert_eq!(caster.hp, 760, "Caster must receive full HP healing when self-targeting");
    // SP MUST REMAIN EXACTLY 100 — 0 SP restored due to SP Infinite Loop Gate!
    assert_eq!(caster.sp, 100, "SP Loop Gate: caster must receive EXACTLY 0 SP when self-targeting!");

    // Verify Delphi action frame: subcode 19 has HP heal (260 = 0x0104), subcode 1A has 0 SP (0x0000)
    let sp_zero_subcode = "1A000000";
    assert!(
        frames.iter().any(|f| f.contains(sp_zero_subcode)),
        "Action frame must broadcast 0 SP restored for self-targeting caster: {frames:?}"
    );
}

#[test]
fn test_type18_clean_target_heals_hp_and_sp() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_CUONG_HIEU_GIAI_TRU as u16, binary_skill(SKILL_CUONG_HIEU_GIAI_TRU as u16, 18, 1));
    binary_skills.insert(SKILL_TRANG_THAI_GIAI_TRU as u16, binary_skill(SKILL_TRANG_THAI_GIAI_TRU as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Ally at (3, 1) has ZERO debuffs, but is low on HP and SP
    let mut battle = Battle::with_seeds(15, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 200, 1000, 50, 500, 50);
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_CUONG_HIEU_GIAI_TRU, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let ally = battle.cell(3, 1).unwrap();
    // Type 18 executes with ATTACK status and heals even on clean target!
    assert!(ally.hp > 200, "Clean target must receive HP healing from Type 18");
    assert!(ally.sp > 50, "Clean ally must receive SP restoration from Type 18");

    // Zero purge frames should be broadcast since there were no debuffs
    let purge_any = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    assert!(!frames.contains(&purge_any), "Clean target must not emit false purge frames");
}

#[test]
fn test_type18_enemy_target_rejected() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_CUONG_HIEU_GIAI_TRU as u16, binary_skill(SKILL_CUONG_HIEU_GIAI_TRU as u16, 18, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(16, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    // Damaged enemy with debuff at (0, 2)
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 500, 1000, 100, 500, 50);
    {
        let enemy = battle.cell_mut(0, 2).unwrap();
        enemy.type3_id = 11014;
        enemy.type3_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: SKILL_CUONG_HIEU_GIAI_TRU, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    let enemy = battle.cell(0, 2).unwrap();
    // Enemy must NOT be healed, debuff NOT cleansed
    assert_eq!(enemy.hp, 500, "Enemy HP must not be healed by Type 18");
    assert_eq!(enemy.sp, 100, "Enemy SP must not be restored by Type 18");
    assert_eq!(enemy.type3_id, 11014, "Enemy debuff must remain intact");

    // Action frame must report MISS
    let miss_frame = packets::skilling_int(0, 2, miss_status::MISS, 0, 1, troi_byte::MISS, 0, 1);
    assert!(frames.iter().any(|f| f.contains(&miss_frame)), "Must broadcast MISS when Type 18 is cast on enemy");
}

#[test]
fn test_specific_dispel_tier_variants_11009_and_11004() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(SKILL_BANG_TAN_1 as u16, binary_skill(SKILL_BANG_TAN_1 as u16, 5, 1));
    binary_skills.insert(SKILL_THUC_TINH_1 as u16, binary_skill(SKILL_THUC_TINH_1 as u16, 5, 1));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Part 1: Băng tan tier 1 (11009) cures Freeze tier 1 (11008)
    let mut battle1 = Battle::with_seeds(17, 0, 42, 7, 99);
    seed_unit(&mut battle1, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle1, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle1, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle1.cell_mut(3, 1).unwrap();
        ally.type3_id = 11008; // Băng phong tier 1
        ally.type3_lv = 1;
        ally.type3_turn = 3;
    }

    let mut commands1 = BattleCommands::new();
    commands1.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_BANG_TAN_1, skill_lv: 1 }),
    );
    let frames1 = run(&mut battle1, &data, commands1);

    let ally1 = battle1.cell(3, 1).unwrap();
    assert_eq!(ally1.type3_id, 0, "Băng phong tier 1 (11008) must be cured to 0 by 11009");
    assert_eq!(ally1.type3_turn, 0);
    assert_eq!(ally1.type3_lv, 0);

    let purge_frame = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    let clear_frame = packets::clear_unit_status(3, 1, packets::STATUS_SEAL_RESIST_PCT);
    assert!(frames1.contains(&purge_frame), "Must emit PURGE_TYPE_DISABLE frame");
    assert!(frames1.contains(&clear_frame), "Must emit clear_unit_status for seal resist");
    let act_thaw = packets::skilling_int(3, 1, miss_status::ATTACK, 0, 1, troi_byte::TYPE3, 0, 1);
    assert!(frames1.iter().any(|f| f.contains(&act_thaw)), "Must emit TYPE3 troi_byte in action frame");

    // Part 2: Thức tỉnh tier 1 (11004) wakes Sleep tier 1 (11003)
    let mut battle2 = Battle::with_seeds(18, 0, 42, 7, 99);
    seed_unit(&mut battle2, 3, 2, 101, 1, 20, 1000, 1000, 500, 500, 100);
    seed_unit(&mut battle2, 3, 1, 102, 1, 20, 1000, 1000, 500, 500, 50);
    seed_unit(&mut battle2, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);
    {
        let ally = battle2.cell_mut(3, 1).unwrap();
        ally.type3_id = 11003; // Hôn mê tier 1
        ally.type3_lv = 1;
        ally.type3_turn = 3;
    }

    let mut commands2 = BattleCommands::new();
    commands2.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: SKILL_THUC_TINH_1, skill_lv: 1 }),
    );
    let frames2 = run(&mut battle2, &data, commands2);

    let ally2 = battle2.cell(3, 1).unwrap();
    assert_eq!(ally2.type3_id, 0, "Hôn mê tier 1 (11003) must be cured to 0 by 11004");
    assert_eq!(ally2.type3_turn, 0);
    assert_eq!(ally2.type3_lv, 0);

    assert!(frames2.contains(&purge_frame));
    assert!(frames2.contains(&clear_frame));
    let act_wake = packets::skilling_int(3, 1, miss_status::ATTACK, 0, 1, troi_byte::TYPE3, 0, 1);
    assert!(frames2.iter().any(|f| f.contains(&act_wake)));
}

#[test]
fn test_cleanse_heal_same_id_ally_sp_restoration_and_skill_20015() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    // 20015: Tứ nhân trạng giải (fight_area = 5 covers cross / team)
    binary_skills.insert(SKILL_TU_NHAN_TRANG_GIAI as u16, binary_skill(SKILL_TU_NHAN_TRANG_GIAI as u16, 18, 5));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(19, 0, 42, 7, 99);
    // Same template ID 15001 for both caster mob and ally mob
    // Caster mob at (3, 2)
    seed_unit(&mut battle, 3, 2, 15001, 1, 20, 500, 2000, 100, 500, 150);
    // Ally mob at (3, 1) with SAME id 15001 and advanced confusion 20014
    seed_unit(&mut battle, 3, 1, 15001, 1, 20, 200, 1000, 50, 500, 50);
    // Enemy at (0, 2)
    seed_unit(&mut battle, 0, 2, 501, 2, 20, 1000, 1000, 500, 500, 50);

    {
        let ally = battle.cell_mut(3, 1).unwrap();
        ally.type15_id = 20014; // Advanced confusion debuff
        ally.type15_lv = 1;
        ally.type15_turn = 3;
    }

    // Caster targets self-position for AOE 5
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 2, BattleAction::CastSkill { skill_id: SKILL_TU_NHAN_TRANG_GIAI, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);

    // 1. Caster check: self-cast SP Infinite Loop Gate
    let caster = battle.cell(3, 2).unwrap();
    assert!(caster.hp > 500, "Caster must be healed");
    assert_eq!(caster.sp, 100, "Caster must receive EXACTLY 0 SP when self-targeting");

    // 2. Non-self ally check: same template ID 15001, but different cell -> receives FULL SP!
    let ally = battle.cell(3, 1).unwrap();
    assert!(ally.hp > 200, "Ally must receive HP healing");
    assert!(ally.sp > 50, "Non-self ally with same template ID MUST receive full SP restoration!");

    // 3. Status check: Skill 20015 successfully cleanses debuff 20014
    assert_eq!(ally.type15_id, 0, "Advanced debuff 20014 must be cleansed to 0 by skill 20015");
    assert_eq!(ally.type15_lv, 0);
    assert_eq!(ally.type15_turn, 0);

    // 4. Frames check: Purge and HUD clear emitted for ally
    let purge_debuff = packets::purge_status(3, 1, packets::PURGE_TYPE_DEBUFF);
    let clear_def = packets::clear_unit_status(3, 1, packets::STATUS_DEF_MODIFIER);
    let clear_atk = packets::clear_unit_status(3, 1, packets::STATUS_ATK_MODIFIER);
    assert!(frames.contains(&purge_debuff), "Must emit PURGE_TYPE_DEBUFF for ally");
    assert!(frames.contains(&clear_def), "Must emit clear_unit_status DEF");
    assert!(frames.contains(&clear_atk), "Must emit clear_unit_status ATK");
}
