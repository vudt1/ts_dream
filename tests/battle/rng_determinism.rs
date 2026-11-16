//! Ticket 10 — deterministic battle RNG.
//!
//! The runner used to create a fresh `DotNetRandom::time_seeded()` for the
//! legacy Type15-in-`Type1`/`Type2` damage roll (attacker buffed by skills
//! `10016..=10019`), which broke `with_seeds` / golden replay. Ticket 10 draws
//! those rolls from `BattleRng::random_skill` — a battle-owned stream seeded
//! from the battle seed — so two battles with the same seed emit identical
//! frames.
//!
//! Draw-order contract: `random_skill` is a brand-new stream. The existing
//! drop/AI/damage streams (`random_0` / `random_1` / `random_2` /
//! `random_damage`) keep the exact same seeds and draw order — nothing in the
//! 10016..10019 branch reads from them.

use std::collections::HashMap;

use ts_dream::battle::construction::Battle;
use ts_dream::battle::rng::BattleRng;
use ts_dream::battle::runner::{BattleAction, BattleCommand, BattleCommands, BattleData, Out};
use ts_dream::data::tables::{Item, Npc, Skill, TexpRow};

/// Magic skill id used to force the `apply_magic` path; its own id is unrelated
/// to the `10016..=10019` buff marker checked on the attacker.
const MAGIC_SKILL: i64 = 11051;

const SEED0: i32 = 0x1357_2468;
const SEED1: i32 = 0x0ACE_0BAD;
const SEED2: i32 = 0x0BAD_F00D;

fn skills() -> HashMap<i64, Skill> {
    let mut skills = HashMap::new();
    skills.insert(
        MAGIC_SKILL,
        Skill {
            id: MAGIC_SKILL,
            sp: 0,
            skill_type: 2, // magic
            lv_max: 1,
            sl_danh: 1,
            ..Skill::default()
        },
    );
    skills
}

/// One player (team 1, magic caster) vs one durable NPC (team 2).
///
/// `attacker_type15` seeds the buff marker checked by the `10016..=10019`
/// branch: `10016` exercises it, `0` is the control.
fn battle_with(attacker_type15: i64) -> Battle {
    let mut battle = Battle::with_seeds(1, 112, SEED0, SEED1, SEED2);
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 7;
        c.leader_id = 7;
        c.team = 1;
        c.hp = 9_999;
        c.hp_max = 9_999;
        c.sp = 5_000;
        c.sp_max = 5_000;
        c.lv = 30;
        c.atk = 100;
        c.int1 = 220;
        c.def = 5;
        c.agi = 60;
        c.type15_id = attacker_type15;
        c.type15_lv = 1;
        c.type15_turn = 9; // > 1 so phase 1 does not clear the marker
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.team = 2;
        c.hp = 9_999_999;
        c.hp_max = 9_999_999;
        c.lv = 5;
        c.def = 0;
        c.agi = 10;
    }
    battle
}

fn magic_command() -> BattleCommands {
    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: MAGIC_SKILL,
                skill_lv: 1,
            },
        },
    );
    commands
}

/// Run 3 turns and return every frame-like `Out` in emission order.
fn run_frames(attacker_type15: i64) -> Vec<Out> {
    let npcs: HashMap<i64, Npc> = HashMap::new();
    let skills = skills();
    let items: HashMap<i64, Item> = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();

    let mut battle = battle_with(attacker_type15);
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0);
    let commands = magic_command();

    let mut out = Vec::new();
    for _ in 0..3 {
        battle.run_turn(&data, &commands, &mut out);
    }
    out
}

#[test]
fn two_seeded_battles_produce_identical_frames() {
    // Same seed twice, including the skill 10016..10019 branch.
    let first = run_frames(10016);
    let second = run_frames(10016);
    assert_eq!(
        first, second,
        "two battles with the same seed must emit byte-identical frames"
    );
    assert!(!first.is_empty(), "the seeded battle must emit frames");
}

#[test]
fn type15_branch_is_actually_exercised() {
    // Control (no 10016..10019 marker) vs the buffed caster. Magic damage is
    // unconditionally rescaled inside the branch, so the frame streams must
    // differ — otherwise the determinism assertion above would be vacuous.
    let control = run_frames(0);
    let buffed = run_frames(10016);
    assert_ne!(
        control, buffed,
        "the 10016..10019 branch must alter the emitted frames"
    );
}

#[test]
fn existing_streams_keep_their_draw_order() {
    // `random_skill` is additive: the drop/AI/damage streams are seeded and
    // drawn exactly as before, independent of any `random_skill` draw.
    let mut plain = BattleRng::with_seeds(SEED0, SEED1, SEED2);
    let mut interleaved = BattleRng::with_seeds(SEED0, SEED1, SEED2);

    for _ in 0..8 {
        // Burn draws on the new stream in the middle; the legacy streams must
        // not observe it.
        let _ = interleaved.random_skill.next_range(1, 3);
        assert_eq!(
            plain.random_0.next(),
            interleaved.random_0.next(),
            "random_0 (drop/skill pick) draw order changed"
        );
        assert_eq!(
            plain.random_1.next(),
            interleaved.random_1.next(),
            "random_1 (AI tie-breaker) draw order changed"
        );
        assert_eq!(
            plain.random_2.next(),
            interleaved.random_2.next(),
            "random_2 (npc respawn) draw order changed"
        );
        assert_eq!(
            plain.random_damage.next(),
            interleaved.random_damage.next(),
            "random_damage (combat formula) draw order changed"
        );
    }
}

#[test]
fn runner_source_no_longer_time_seeds() {
    // Guards the ticket invariant directly: no `time_seeded()` on the battle
    // runner path.
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/battle/runner.rs"
    ))
    .expect("read src/battle/runner.rs");
    assert!(
        !source.contains("time_seeded"),
        "src/battle/runner.rs must not create time-seeded RNGs"
    );
}
