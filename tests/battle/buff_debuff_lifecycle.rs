//! Integration tests for the battle buff & debuff lifecycle (Ticket:
//! `battle-buff-debuff-lifecycle`).
//!
//! Covers the full status-effect loop against the PC wire format:
//! 1. `damage::get_turn` is the single source of truth for duration (G2), and
//!    the PC loader never leaks the visual sprite animation id (`state`) into
//!    `BinarySkillDef.round` (G1).
//! 2. Anti-stacking: a target that already holds a status in a channel
//!    (Type 3 / Type 4 / Type 15) must reject a re-application with
//!    `miss_status::MISS` — no duration refresh, no id overwrite (G3).
//! 3. Cross-channel concurrency: one entity may simultaneously carry Type 3
//!    (Freeze), Type 4 (Mirror), Type 15 (Poison) and Type 19 (Job Aura)
//!    without the channels clobbering each other (G3).
//! 4. Attack skills with `hit_status > 0` obtain their secondary status
//!    duration from `damage::get_turn` via `binary_skill_input` when the PC
//!    binary carries `round == 0` (G1/SSoT), and the status lands on a clean
//!    target (combat_formula.rs gate).
//! 5. Turn countdown, poison DOT (`30 + lv * 15`) and the auto-dispel at
//!    `turn <= 1` (purge `0x35 0x01` + HUD clear `0x35 0x0F`) (G4).
//!
//! All battles run in memory against seeded `DotNetRandom` streams — no DB,
//! no I/O, fully deterministic.

use std::collections::HashMap;
use std::sync::LazyLock;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::damage;
use ts_dream::battle::packets::{self, attack_status, miss_status, troi_byte};
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, Out, PlayerSnapshot,
};
use ts_dream::data::loaders::pc_to_binary;
use ts_dream::data::tables::{BinarySkillDef, Item, Npc, PcSkillDef, Skill, TexpRow};
use ts_dream::protocol::encoder;

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

/// PC `Skill.Dat` loader semantics: `round` stays 0 and durations resolve via
/// `damage::get_turn`. `hit_status` is the client's u8 secondary-status code
/// (0 = pure buff/debuff; e.g. 6 = ICE_BOUND/frozen).
fn binary_skill(id: u16, attribute: u8, hit_status: u8) -> BinarySkillDef {
    BinarySkillDef {
        id,
        name: format!("Skill {id}"),
        kind: 1,
        require_sp: 0,
        element: 1,
        numerical: 150,
        attribute,
        fight_area: 0, // hostile/friendly anchor targeting (not fight_area expansion)
        round: 0,      // PC layout has no round field — never the visual state id
        hit_status,
        how_much_times: 1,
        max_lv: 9,
        ..Default::default()
    }
}

fn command(row: u8, col: u8, target_row: u8, target_col: u8, action: BattleAction) -> BattleCommand {
    BattleCommand { row, col, target_row, target_col, action }
}

fn broadcast_frames(out: &[Out]) -> Vec<String> {
    out.iter()
        .filter_map(|o| match o {
            Out::Broadcast(frame) => Some(frame.clone()),
            _ => None,
        })
        .collect()
}

/// Seed a grid cell with the given stats. `team 1` = rows 2-3, `team 2` = rows 0-1.
#[allow(clippy::too_many_arguments)]
fn seed_unit(
    battle: &mut Battle,
    row: u8,
    col: u8,
    id: i64,
    team: i64,
    lv: i64,
    hp: i64,
    int1: i64,
) {
    let cell = battle.cell_mut(row, col).expect("cell exists");
    cell.typ = 2; // player-kind cell: never auto-acts, commands only
    cell.id = id;
    cell.team = team;
    cell.lv = lv;
    cell.hp = hp;
    cell.hp_max = hp;
    cell.sp = 500;
    cell.sp_max = 500;
    cell.atk = 500;
    cell.def = 0;
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

// ---------------------------------------------------------------------------
// Test Case 1 — get_turn canonical table parity + PC loader hygiene (G1, G2)
// ---------------------------------------------------------------------------
#[test]
fn test_skill_round_table_parity() {
    // Băng phong (11014, GROUP_e): always 5 turns.
    assert_eq!(damage::get_turn(11014, 1), 5);
    assert_eq!(damage::get_turn(11014, 5), 5);

    // Cây tinh (10004, GROUP_c): always 3 turns.
    assert_eq!(damage::get_turn(10004, 1), 3);
    assert_eq!(damage::get_turn(10004, 5), 3);

    // Kính (10015, GROUP_b): 2 (Lv 1-2), 3 (Lv 3-4), 4 (Lv 5).
    for (lv, expected) in [(1, 2), (2, 2), (3, 3), (4, 3), (5, 4)] {
        assert_eq!(damage::get_turn(10015, lv), expected, "Kính (10015) lv {lv}");
    }

    // Bỏ độc (14015, GROUP_c): always 3 turns.
    assert_eq!(damage::get_turn(14015, 1), 3);
    assert_eq!(damage::get_turn(14015, 5), 3);

    // Hỗn loạn (14021): 2..6 turns for Lv 1..5.
    for lv in 1..=5i64 {
        assert_eq!(damage::get_turn(14021, lv), lv + 1, "Hỗn loạn (14021) lv {lv}");
    }

    // Kết giới (10010, GROUP_f): 2 (Lv 1-3), 3 (Lv 4-6), 4 (Lv 7-9), 5 (Lv 10).
    for (lv, expected) in [(1, 2), (3, 2), (4, 3), (6, 3), (7, 4), (9, 4), (10, 5)] {
        assert_eq!(damage::get_turn(10010, lv), expected, "Kết giới (10010) lv {lv}");
    }

    // The PC loader must never map the client's visual sprite animation
    // (`state`, e.g. 171 = Poison) onto `BinarySkillDef.round`.
    let pc = PcSkillDef {
        id: 14015,
        state: 171, // visual poison animation id — NOT 171 turns
        ..Default::default()
    };
    assert_eq!(pc_to_binary(&pc).round, 0);

    let pc_mirror = PcSkillDef {
        id: 10015,
        state: 103, // visual mirror animation id — NOT 103 turns
        ..Default::default()
    };
    assert_eq!(pc_to_binary(&pc_mirror).round, 0);
}

// ---------------------------------------------------------------------------
// Test Case 2 — anti-stacking rejects re-application with MISS (G3)
// ---------------------------------------------------------------------------
#[test]
fn test_anti_stacking_enforcement() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(11014, binary_skill(11014, 3, 0));
    binary_skills.insert(10010, binary_skill(10010, 4, 0));
    binary_skills.insert(14015, binary_skill(14015, 15, 0));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // --- Channel Type 3 (crowd control): Freeze re-cast on a Cây tinh victim.
    let mut battle = Battle::with_seeds(1, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 2000, 3000); // caster, int1 → landing 100%
    seed_unit(&mut battle, 0, 2, 501, 2, 10, 5000, 0); // victim
    {
        let victim = battle.cell_mut(0, 2).unwrap();
        victim.type3_id = 10004; // Cây tinh
        victim.type3_lv = 1;
        victim.type3_turn = 3;
    }
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: 11014, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);
    let victim = battle.cell(0, 2).unwrap();
    // Phase 1 decrements 3 -> 2; the cast itself must not overwrite or refresh.
    assert_eq!(victim.type3_id, 10004, "Type 3 id must stay Cây tinh");
    assert_eq!(victim.type3_turn, 2, "Type 3 must only lose the round-start decrement");
    assert_eq!(victim.type3_lv, 1);
    let miss = packets::skilling_int(0, 2, miss_status::MISS, attack_status::ATTACK, 1, troi_byte::MISS, 0, 1);
    assert!(
        frames.iter().any(|f| f.contains(&miss)),
        "anti-stacked Type 3 cast must emit miss_status::MISS, got: {frames:?}"
    );

    // --- Channel Type 4 (protective barriers): Kết giới re-cast on a Mirrored ally.
    let mut battle = Battle::with_seeds(2, 0, 42, 7, 99);
    // An idle enemy keeps `all_enemies_dead()` false so the turn actually runs.
    seed_unit(&mut battle, 0, 2, 501, 2, 10, 2000, 0);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 2000, 3000); // caster
    seed_unit(&mut battle, 3, 1, 102, 1, 20, 2000, 0); // mirrored ally
    {
        let ally = battle.cell_mut(3, 1).unwrap();
        ally.type4_id = 10015; // Kính
        ally.type4_lv = 2;
        ally.type4_turn = 3;
    }
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 3, 1, BattleAction::CastSkill { skill_id: 10010, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);
    let ally = battle.cell(3, 1).unwrap();
    assert_eq!(ally.type4_id, 10015, "Type 4 id must stay Kính");
    assert_eq!(ally.type4_turn, 2, "Type 4 must only lose the round-start decrement");
    assert_eq!(ally.type4_lv, 2);
    let miss = packets::skilling_int(3, 1, miss_status::MISS, attack_status::DEF, 1, troi_byte::MISS, 0, 1);
    assert!(
        frames.iter().any(|f| f.contains(&miss)),
        "anti-stacked Type 4 cast must emit miss_status::MISS, got: {frames:?}"
    );

    // --- Channel Type 15 (curses / DOT): Bỏ độc re-cast on a poisoned victim.
    let mut battle = Battle::with_seeds(3, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 2000, 3000); // caster
    seed_unit(&mut battle, 0, 2, 501, 2, 10, 5000, 0); // victim
    {
        let victim = battle.cell_mut(0, 2).unwrap();
        victim.type15_id = 14015; // Bỏ độc
        victim.type15_lv = 1;
        victim.type15_turn = 3;
    }
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: 14015, skill_lv: 1 }),
    );
    let frames = run(&mut battle, &data, commands);
    let victim = battle.cell(0, 2).unwrap();
    assert_eq!(victim.type15_id, 14015, "Type 15 id must stay Bỏ độc");
    assert_eq!(victim.type15_turn, 2, "Type 15 must only lose the round-start decrement");
    assert_eq!(victim.type15_lv, 1);
    let miss = packets::skilling_int(0, 2, miss_status::MISS, attack_status::ATTACK, 1, troi_byte::MISS, 0, 1);
    assert!(
        frames.iter().any(|f| f.contains(&miss)),
        "anti-stacked Type 15 cast must emit miss_status::MISS, got: {frames:?}"
    );
}

// ---------------------------------------------------------------------------
// Test Case 3 — cross-channel concurrency: Freeze + Mirror + Poison + Aura (G3)
// ---------------------------------------------------------------------------
#[test]
fn test_cross_channel_concurrency() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(11014, binary_skill(11014, 3, 0));
    binary_skills.insert(10015, binary_skill(10015, 4, 0));
    binary_skills.insert(14015, binary_skill(14015, 15, 0));
    binary_skills.insert(14040, binary_skill(14040, 19, 0));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Team 1 (players): two hostile casters. Team 2: victim + two friendly casters.
    let mut battle = Battle::with_seeds(7, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 2000, 3000); // P1 — casts Băng phong
    seed_unit(&mut battle, 3, 3, 102, 1, 20, 2000, 3000); // P2 — casts Bỏ độc
    seed_unit(&mut battle, 0, 2, 501, 2, 10, 5000, 0); // E  — the victim
    seed_unit(&mut battle, 0, 3, 502, 2, 10, 2000, 100); // EC1 — casts Kính on E
    seed_unit(&mut battle, 0, 4, 503, 2, 10, 2000, 100); // EC2 — casts Bá ý on E

    // Round 1 — four independent channels applied to the same victim.
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: 11014, skill_lv: 1 }),
    );
    commands.insert(
        (3, 3),
        command(3, 3, 0, 2, BattleAction::CastSkill { skill_id: 14015, skill_lv: 2 }),
    );
    commands.insert(
        (0, 3),
        command(0, 3, 0, 2, BattleAction::CastSkill { skill_id: 10015, skill_lv: 5 }),
    );
    commands.insert(
        (0, 4),
        command(0, 4, 0, 2, BattleAction::CastSkill { skill_id: 14040, skill_lv: 5 }),
    );
    let _ = run(&mut battle, &data, commands);

    let victim = battle.cell(0, 2).unwrap();
    assert_eq!(victim.type3_id, 11014, "Freeze (Type 3) must land");
    assert_eq!(victim.type3_turn, 5);
    assert_eq!(victim.type15_id, 14015, "Poison (Type 15) must land");
    assert_eq!(victim.type15_lv, 2);
    assert_eq!(victim.type15_turn, 3);
    assert_eq!(victim.type4_id, 10015, "Mirror (Type 4) must land");
    assert_eq!(victim.type4_turn, 4, "Kính Lv5 → 4 turns (GROUP_b)");
    assert_eq!(victim.type19_id, 14040, "Bá ý (Type 19) must land");
    assert_eq!(victim.type19_turn, 4, "Bá ý Lv5 → (2 + 5/2).clamp(2, 4)");

    // Round 2 — every channel decrements independently; poison ticks 60 damage.
    let _ = run(&mut battle, &data, BattleCommands::new());
    let victim = battle.cell(0, 2).unwrap();
    assert_eq!((victim.type3_id, victim.type3_turn), (11014, 4));
    assert_eq!((victim.type15_id, victim.type15_turn), (14015, 2));
    assert_eq!((victim.type4_id, victim.type4_turn), (10015, 3));
    assert_eq!((victim.type19_id, victim.type19_turn), (14040, 3));
    assert_eq!(victim.hp, 5000 - (30 + 2 * 15), "poison DOT: 30 + lv * 15");

    // Round 3 — poison expires and is purged; the other three channels survive.
    let frames_r3 = run(&mut battle, &data, BattleCommands::new());
    let victim = battle.cell(0, 2).unwrap();
    assert_eq!(victim.type15_id, 0, "expired poison must be dispelled");
    assert_eq!(victim.type15_turn, 0);
    assert_eq!(victim.type3_id, 11014, "Freeze must survive the poison dispel");
    assert_eq!(victim.type3_turn, 3);
    assert_eq!(victim.type4_id, 10015, "Mirror must survive the poison dispel");
    assert_eq!(victim.type4_turn, 2);
    assert_eq!(victim.type19_id, 14040, "Bá ý must survive the poison dispel");
    assert_eq!(victim.type19_turn, 2);

    // The purge in round 3 must be scoped to the Type 15 channel only.
    let purge_debuff = packets::purge_status(0, 2, packets::PURGE_TYPE_DEBUFF);
    assert!(frames_r3.contains(&purge_debuff), "poison dispel must emit PURGE_TYPE_DEBUFF");
    assert!(
        !frames_r3.contains(&packets::purge_status(0, 2, packets::PURGE_TYPE_DISABLE)),
        "poison dispel must not purge the Type 3 channel"
    );
    assert!(
        !frames_r3.contains(&packets::purge_status(0, 2, packets::PURGE_TYPE_BUFF)),
        "poison dispel must not purge the Type 4 channel"
    );
    assert!(
        !frames_r3.contains(&packets::purge_status(0, 2, packets::PURGE_TYPE_AURA)),
        "poison dispel must not purge the Type 19 channel"
    );
}

// ---------------------------------------------------------------------------
// Test Case 4 — attack skills with hit_status > 0 inflict a secondary status
// with the canonical get_turn duration, even though the PC binary carries
// round == 0 (G1, combat_formula.rs gate)
// ---------------------------------------------------------------------------
#[test]
fn test_attack_secondary_status_infliction() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    const TEST_STRIKE: u16 = 19001; // test-only catalog entry: damaging attack
    // `hit_status` is the PC client's u8 secondary-status code (6 = ICE_BOUND / frozen).
    let mut binary_skills = HashMap::new();
    binary_skills.insert(TEST_STRIKE, binary_skill(TEST_STRIKE, 1, 6));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Single attacker on team 1 (no same-team successor → combo never rolls).
    let mut battle = Battle::with_seeds(11, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 2000, 100);
    {
        let attacker = battle.cell_mut(3, 2).unwrap();
        attacker.atk = 500;
        attacker.agi = 200; // hit_rate clamps to 99
        attacker.thuoctinh = 1;
    }
    seed_unit(&mut battle, 0, 2, 501, 2, 1, 9999, 0);

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(
            3,
            2,
            0,
            2,
            BattleAction::CastSkill { skill_id: i64::from(TEST_STRIKE), skill_lv: 3 },
        ),
    );
    let _ = run(&mut battle, &data, commands);

    let victim = battle.cell(0, 2).unwrap();
    assert!(victim.hp < 9999, "the attack must deal damage");
    assert_eq!(victim.type3_id, 6, "secondary status (ICE_BOUND/frozen) must land on a clean target");
    assert_eq!(victim.type3_lv, 3);
    // TEST_STRIKE is uncataloged in damage::get_turn → documented wildcard default (3).
    // A literal 3 pins the wiring: round == 0 in the PC binary must not suppress or
    // zero the secondary status duration.
    assert_eq!(victim.type3_turn, 3, "secondary status duration must come from damage::get_turn, not the PC binary");
}

// ---------------------------------------------------------------------------
// Test Case 5 — countdown, poison DOT and auto-dispel at turn <= 1 (G4)
// ---------------------------------------------------------------------------
#[test]
fn test_turn_countdown_dot_and_auto_dispel_lifecycle() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(14015, binary_skill(14015, 15, 0));
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(13, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 1, 20, 2000, 3000); // caster (landing 100%)
    seed_unit(&mut battle, 0, 2, 501, 2, 10, 5000, 0); // victim

    // Round 1 — cast Bỏ độc (Lv 2). DOT starts next round, duration is canonical.
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: 14015, skill_lv: 2 }),
    );
    let _ = run(&mut battle, &data, commands);
    let victim = battle.cell(0, 2).unwrap();
    assert_eq!(victim.type15_id, 14015);
    assert_eq!(victim.type15_lv, 2);
    assert_eq!(victim.type15_turn, 3, "Bỏ độc duration must come from damage::get_turn");
    assert_eq!(victim.hp, 5000, "DOT must not fire on the casting round");

    // Round 2 — round-start decrement 3 -> 2, poison ticks for 30 + 2*15 = 60.
    let frames = run(&mut battle, &data, BattleCommands::new());
    let victim = battle.cell(0, 2).unwrap();
    assert_eq!(victim.type15_turn, 2);
    assert_eq!(victim.hp, 4940);
    let dot_marker = encoder::le16(20003); // poison skill id embedded in the 0x32 0x01 DOT frame
    assert!(
        frames.iter().any(|f| f.starts_with("F444") && f.contains("3201") && f.contains(&dot_marker)),
        "poison DOT must broadcast a 0x32 0x01 frame, got: {frames:?}"
    );
    assert!(
        !frames.contains(&packets::purge_status(0, 2, packets::PURGE_TYPE_DEBUFF)),
        "poison must not dispel while turns remain"
    );

    // Round 3 — decrement 2 -> 1, final poison tick, then auto-dispel (turn <= 1).
    let frames = run(&mut battle, &data, BattleCommands::new());
    let victim = battle.cell(0, 2).unwrap();
    assert_eq!(victim.hp, 4880, "final poison tick: 30 + 2 * 15");
    assert_eq!(victim.type15_id, 0, "expired poison must reset type15_id");
    assert_eq!(victim.type15_lv, 0);
    assert_eq!(victim.type15_turn, 0);

    let purge_debuff = packets::purge_status(0, 2, packets::PURGE_TYPE_DEBUFF);
    let clear_def = packets::clear_unit_status(0, 2, packets::STATUS_DEF_MODIFIER);
    let clear_atk = packets::clear_unit_status(0, 2, packets::STATUS_ATK_MODIFIER);
    assert!(frames.contains(&purge_debuff), "auto-dispel must broadcast 0x35 0x01 PURGE_TYPE_DEBUFF");
    assert!(frames.contains(&clear_def), "auto-dispel must clear the DEF HUD slot (0x35 0x0F)");
    assert!(frames.contains(&clear_atk), "auto-dispel must clear the ATK HUD slot (0x35 0x0F)");
    let purge_idx = frames.iter().position(|f| f == &purge_debuff).unwrap();
    let clear_def_idx = frames.iter().position(|f| f == &clear_def).unwrap();
    let clear_atk_idx = frames.iter().position(|f| f == &clear_atk).unwrap();
    assert!(purge_idx < clear_def_idx && purge_idx < clear_atk_idx, "purge must precede the HUD clears");
}
