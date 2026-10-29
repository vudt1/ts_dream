//! Integration tests for Ticket 18: Opcode 0x35 timing wiring and Dynamic Skill Delay Pacing.
//!
//! Verifies the 4 phases of turn execution (Command -> Stance -> Action -> Cleanup):
//! 1. Phase 1 (Turn Start): `35 01` (purge_status) and `35 0F` (clear_unit_status) on expired effects.
//! 2. Phase 2 (Stance): `35 03` (force_state7) emitted immediately before `32 01` action.
//! 3. Phase 3 (Action Impact): `35 0F` (battle_unit_status) emitted immediately after `32 01` on buff/debuff.
//! 4. Phase 4 (Cleanup & Pacing): `35 06` (battle_npc_talk) on opening dialogue, followed by
//!    `32 02` (turn_action_finish), and Dynamic Skill Delay Pacing clamped between 800ms and 3500ms.
//! 5. Dispel / Cleanse: `35 01` and `35 0F` emitted when effects are removed by skills.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::manager::{BattleManager, BattleSink};
use ts_dream::battle::packets;
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, Out, PlayerSnapshot,
};
use ts_dream::data::tables::{Npc, Skill, TexpRow};

fn create_test_data<'a>(
    npcs: &'a HashMap<i64, Npc>,
    skills: &'a HashMap<i64, Skill>,
    pet_slots: &'a HashMap<i64, [i64; 4]>,
    players: &'a HashMap<i64, PlayerSnapshot>,
    texps: &'a [TexpRow],
) -> BattleData<'a> {
    static EMPTY_ITEMS: std::sync::LazyLock<HashMap<i64, ts_dream::data::tables::Item>> =
        std::sync::LazyLock::new(HashMap::new);
    BattleData::new(
        npcs,
        skills,
        &EMPTY_ITEMS,
        pet_slots,
        players,
        texps,
        None,
        0,
        0,
    )
}

#[derive(Default)]
struct CapturingSink {
    frames: std::sync::Mutex<Vec<(Option<i64>, String)>>,
    ended: std::sync::Mutex<Option<(i32, ts_dream::battle::runner::Outcome)>>,
}

impl BattleSink for CapturingSink {
    fn send_to(&self, player: i64, frame: String) {
        self.frames.lock().unwrap().push((Some(player), frame));
    }

    fn send_map(&self, _player: i64, _frame: String) {}

    fn broadcast(&self, frame: String) {
        self.frames.lock().unwrap().push((None, frame));
    }

    fn apply_db(&self, _update: ts_dream::battle::runner::DbUpdate) {}
    fn apply_drop(&self, _drop: ts_dream::battle::runner::Out) {}
    fn apply_catch(&self, _owner: i64, _npc_id: i64) {}
    fn apply_fled(&self, _player: i64) {}
    fn apply_respawn(&self, _npc_id: i64, _map_id: i64, _x: i64, _y: i64) {}
    fn apply_pet_exp(&self, _owner: i64, _stt: i64, _exp: i64) {}

    fn battle_ended(&self, id: i32, outcome: ts_dream::battle::runner::Outcome) {
        *self.ended.lock().unwrap() = Some((id, outcome));
    }
}

#[test]
fn phase1_turn_start_emits_purge_and_clear_status_on_expired_effects() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps);

    let mut battle = Battle::new(1, 0);

    // Setup unit at (3, 2) with 2 turns on type3 (disable) and type4 (buff)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
        c.type3_id = 10001;
        c.type3_turn = 2;
        c.type4_id = 10010;
        c.type4_turn = 2;
    }
    // Setup ally at (3, 1) with 1 turn on type15 (debuff) and type19 (aura)
    {
        let c = battle.cell_mut(3, 1).unwrap();
        c.typ = 2;
        c.id = 102;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
        c.type15_id = 14015;
        c.type15_turn = 1;
        c.type19_id = 13020;
        c.type19_turn = 1;
    }
    // Setup enemy at (0, 2) with no status (type3_id = 0, dummy type3_turn = 1)
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
        c.type3_id = 0;
        c.type3_turn = 1;
    }

    let mut out = Vec::new();
    battle.run_turn(&data, &BattleCommands::new(), &mut out);

    // Verify Phase 1 emitted purge_status (35 01) and clear_unit_status (35 0F) for cell (3, 2)
    let purge_type3 = packets::purge_status(3, 2, packets::PURGE_TYPE_DISABLE);
    let clear_type3 = packets::clear_unit_status(3, 2, packets::STATUS_SEAL_RESIST_PCT);
    let purge_type4 = packets::purge_status(3, 2, packets::PURGE_TYPE_BUFF);
    let clear_type4_def = packets::clear_unit_status(3, 2, packets::STATUS_DEF_MODIFIER);
    let clear_type4_atk = packets::clear_unit_status(3, 2, packets::STATUS_ATK_MODIFIER);

    // Verify Phase 1 emitted purge_status and clear_unit_status for cell (3, 1) with 1-turn effects
    let purge_type15 = packets::purge_status(3, 1, packets::PURGE_TYPE_DEBUFF);
    let clear_type15_def = packets::clear_unit_status(3, 1, packets::STATUS_DEF_MODIFIER);
    let clear_type15_atk = packets::clear_unit_status(3, 1, packets::STATUS_ATK_MODIFIER);
    let purge_type19 = packets::purge_status(3, 1, packets::PURGE_TYPE_AURA);
    let clear_type19 = packets::clear_unit_status(3, 1, packets::STATUS_DMG_BONUS_PCT);

    // Enemy at (0, 2) has type3_id == 0, must NOT emit purge or clear
    let unneeded_purge = packets::purge_status(0, 2, packets::PURGE_TYPE_DISABLE);

    let broadcasts: Vec<String> = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.clone()),
            _ => None,
        })
        .collect();

    assert!(
        broadcasts.contains(&purge_type3),
        "Phase 1 must broadcast purge_status for expired type3"
    );
    assert!(
        broadcasts.contains(&clear_type3),
        "Phase 1 must broadcast clear_unit_status for expired type3"
    );
    assert!(
        broadcasts.contains(&purge_type4),
        "Phase 1 must broadcast purge_status for expired type4"
    );
    assert!(
        broadcasts.contains(&clear_type4_def),
        "Phase 1 must broadcast clear_unit_status DEF for expired type4"
    );
    assert!(
        broadcasts.contains(&clear_type4_atk),
        "Phase 1 must broadcast clear_unit_status ATK for expired type4"
    );

    // 1-turn debuff and aura verification on cell (3, 1)
    assert!(
        broadcasts.contains(&purge_type15),
        "Phase 1 must broadcast purge_status for expired 1-turn type15"
    );
    assert!(
        broadcasts.contains(&clear_type15_def),
        "Phase 1 must broadcast clear_unit_status DEF for expired type15"
    );
    assert!(
        broadcasts.contains(&clear_type15_atk),
        "Phase 1 must broadcast clear_unit_status ATK for expired type15"
    );
    assert!(
        broadcasts.contains(&purge_type19),
        "Phase 1 must broadcast purge_status for expired 1-turn type19"
    );
    assert!(
        broadcasts.contains(&clear_type19),
        "Phase 1 must broadcast clear_unit_status for expired type19"
    );

    // Inactive unit must not emit spurious purge
    assert!(
        !broadcasts.contains(&unneeded_purge),
        "Phase 1 must not broadcast purge_status when type3_id == 0"
    );

    // Verify purge frame appears before clear frame
    let idx_purge = broadcasts.iter().position(|f| f == &purge_type3).unwrap();
    let idx_clear = broadcasts.iter().position(|f| f == &clear_type3).unwrap();
    assert!(
        idx_purge < idx_clear,
        "purge_status must precede clear_unit_status"
    );
}

#[test]
fn phase2_and_phase3_force_state7_precedes_32_01_and_status_follows() {
    let npcs = HashMap::new();
    let mut skills = HashMap::new();
    // Register buff skill 10010 (Type 4 buff, sl_danh = 1, delay = 12)
    skills.insert(
        10010,
        Skill {
            skill_type: 4,
            sl_danh: 1,
            delay: 12,
            ..Default::default()
        },
    );

    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps);

    let mut battle = Battle::new(1, 0);
    // Attacker at (3, 2)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    // Target ally at (3, 1)
    {
        let c = battle.cell_mut(3, 1).unwrap();
        c.typ = 2;
        c.id = 102;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    // Enemy at (0, 2) so battle continues
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 3,
            target_col: 1,
            action: BattleAction::CastSkill {
                skill_id: 10010,
                skill_lv: 1,
            },
        },
    );

    let mut out = Vec::new();
    battle.run_turn(&data, &commands, &mut out);

    let broadcasts: Vec<String> = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.clone()),
            _ => None,
        })
        .collect();

    let state7_frame = packets::force_state7(3, 2);
    let expected_status = packets::battle_unit_status(
        3,
        1,
        packets::STATUS_DEF_MODIFIER,
        2,
        packets::stat_modifier_val(10),
    );

    let idx_state7 = broadcasts
        .iter()
        .position(|f| f == &state7_frame)
        .expect("force_state7 must be broadcast for attacker (3, 2)");

    let idx_status = broadcasts
        .iter()
        .position(|f| f == &expected_status)
        .expect("battle_unit_status must be broadcast for buff target (3, 1)");

    // The player's 32 01 action frame must sit directly between force_state7 and battle_unit_status
    let player_3201 = &broadcasts[idx_state7 + 1];
    assert!(
        player_3201.contains("3201"),
        "The frame immediately following force_state7 must be the 32 01 action frame, got: {}",
        player_3201
    );
    assert_eq!(
        idx_state7 + 2,
        idx_status,
        "battle_unit_status must immediately follow the 32 01 action frame"
    );
}

#[test]
fn phase4_opening_dialogue_broadcast_before_turn_action_finish() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps);

    let mut battle = Battle::new(1, 0).with_opening_dialog(1234);
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }

    let mut out = Vec::new();
    // Turn 1
    battle.run_turn(&data, &BattleCommands::new(), &mut out);

    let broadcasts: Vec<String> = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.clone()),
            _ => None,
        })
        .collect();

    let expected_talk = packets::battle_npc_talk(1, 0, 1234);
    let finish_frame = packets::turn_action_finish();

    let idx_talk = broadcasts.iter().position(|f| f == &expected_talk);
    let idx_finish = broadcasts.iter().rposition(|f| f == &finish_frame);

    assert!(
        idx_talk.is_some(),
        "35 06 battle_npc_talk must be emitted on opening turn"
    );
    assert!(
        idx_finish.is_some(),
        "32 02 turn_action_finish must be emitted"
    );

    assert!(
        idx_talk.unwrap() < idx_finish.unwrap(),
        "35 06 must be emitted BEFORE 32 02 in Phase 4"
    );

    // Turn 2: opening dialog must NOT be repeated
    out.clear();
    battle.run_turn(&data, &BattleCommands::new(), &mut out);
    let broadcasts_turn2: Vec<String> = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !broadcasts_turn2.contains(&expected_talk),
        "35 06 opening dialogue must only be emitted once"
    );
}

#[test]
fn dynamic_skill_delay_pacing_calculation_and_clamping() {
    let npcs = HashMap::new();
    let mut skills = HashMap::new();
    // Skill with delay 5 (5 * 100 = 500ms -> clamped to 800ms)
    skills.insert(
        20001,
        Skill {
            skill_type: 1,
            sl_danh: 1,
            delay: 5,
            ..Default::default()
        },
    );
    // Skill with delay 20 (20 * 100 = 2000ms -> 2000ms)
    skills.insert(
        20002,
        Skill {
            skill_type: 1,
            sl_danh: 1,
            delay: 20,
            ..Default::default()
        },
    );
    // Skill with delay 50 (50 * 100 = 5000ms -> clamped to 3500ms)
    skills.insert(
        20003,
        Skill {
            skill_type: 1,
            sl_danh: 1,
            delay: 50,
            ..Default::default()
        },
    );

    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps);

    // Case 1: Minimum clamp (800ms)
    let mut battle1 = Battle::new(1, 0);
    {
        let c = battle1.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    {
        let c = battle1.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }
    let mut cmds1 = BattleCommands::new();
    cmds1.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 20001,
                skill_lv: 1,
            },
        },
    );
    let mut out1 = Vec::new();
    battle1.run_turn(&data, &cmds1, &mut out1);
    assert_eq!(
        battle1.last_turn_delay_ms(),
        800,
        "Delay 5 * 100 = 500ms must clamp to 800ms minimum"
    );

    // Case 2: Intermediate delay (20 * 100 = 2000ms)
    let mut battle2 = Battle::new(2, 0);
    {
        let c = battle2.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    {
        let c = battle2.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }
    let mut cmds2 = BattleCommands::new();
    cmds2.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 20002,
                skill_lv: 1,
            },
        },
    );
    let mut out2 = Vec::new();
    battle2.run_turn(&data, &cmds2, &mut out2);
    assert_eq!(
        battle2.last_turn_delay_ms(),
        2000,
        "Delay 20 * 100 = 2000ms is within [800, 3500]"
    );

    // Case 3: Maximum clamp (3500ms)
    let mut battle3 = Battle::new(3, 0);
    {
        let c = battle3.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    {
        let c = battle3.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }
    let mut cmds3 = BattleCommands::new();
    cmds3.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 20003,
                skill_lv: 1,
            },
        },
    );
    let mut out3 = Vec::new();
    battle3.run_turn(&data, &cmds3, &mut out3);
    assert_eq!(
        battle3.last_turn_delay_ms(),
        3500,
        "Delay 50 * 100 = 5000ms must clamp to 3500ms maximum"
    );
}

#[test]
fn dispel_and_cleanse_emits_purge_and_clear_status() {
    let npcs = HashMap::new();
    let mut skills = HashMap::new();
    // Skill 11014 (Dispel Type4 10010, skill_type = 5)
    skills.insert(
        11014,
        Skill {
            skill_type: 5,
            sl_danh: 1,
            delay: 10,
            ..Default::default()
        },
    );
    // Skill 11016 (Cleanse Type3 and Type15 + Heal, skill_type = 18)
    skills.insert(
        11016,
        Skill {
            skill_type: 18,
            sl_danh: 1,
            delay: 10,
            ..Default::default()
        },
    );

    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps);

    // --- Part 1: Dispel (Skill 11014, skill_type 5) on enemy buff ---
    let mut battle = Battle::new(1, 0);
    // Attacker at (3, 2)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    // Target at (0, 2) with active buff 10010
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
        c.type4_id = 10010;
        c.type4_turn = 3;
    }

    let mut commands = BattleCommands::new();
    commands.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::CastSkill {
                skill_id: 11014,
                skill_lv: 1,
            },
        },
    );

    let mut out = Vec::new();
    battle.run_turn(&data, &commands, &mut out);

    let broadcasts: Vec<String> = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.clone()),
            _ => None,
        })
        .collect();

    let purge_buff = packets::purge_status(0, 2, packets::PURGE_TYPE_BUFF);
    let clear_def = packets::clear_unit_status(0, 2, packets::STATUS_DEF_MODIFIER);

    assert!(
        broadcasts.contains(&purge_buff),
        "Dispel must broadcast purge_status on target (0, 2)"
    );
    assert!(
        broadcasts.contains(&clear_def),
        "Dispel must broadcast clear_unit_status on target (0, 2)"
    );

    // --- Part 2: Cleanse (Skill 11016, skill_type 18) on afflicted ally ---
    let mut battle_cleanse = Battle::new(2, 0);
    // Healer at (3, 2)
    {
        let c = battle_cleanse.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    // Afflicted ally at (3, 1) with active seal (type3) and debuff (type15)
    {
        let c = battle_cleanse.cell_mut(3, 1).unwrap();
        c.typ = 2;
        c.id = 102;
        c.hp = 500;
        c.hp_max = 1000;
        c.team = 1;
        c.type3_id = 10001;
        c.type3_turn = 3;
        c.type15_id = 14015;
        c.type15_turn = 3;
    }
    // Living enemy at (0, 2) so battle continues
    {
        let c = battle_cleanse.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }

    let mut cmds_cleanse = BattleCommands::new();
    cmds_cleanse.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 3,
            target_col: 1,
            action: BattleAction::CastSkill {
                skill_id: 11016,
                skill_lv: 1,
            },
        },
    );

    let mut out_cleanse = Vec::new();
    battle_cleanse.run_turn(&data, &cmds_cleanse, &mut out_cleanse);

    let broadcasts_cleanse: Vec<String> = out_cleanse
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.clone()),
            _ => None,
        })
        .collect();

    let purge_disable = packets::purge_status(3, 1, packets::PURGE_TYPE_DISABLE);
    let clear_seal = packets::clear_unit_status(3, 1, packets::STATUS_SEAL_RESIST_PCT);
    let purge_debuff = packets::purge_status(3, 1, packets::PURGE_TYPE_DEBUFF);
    let clear_debuff_def = packets::clear_unit_status(3, 1, packets::STATUS_DEF_MODIFIER);
    let clear_debuff_atk = packets::clear_unit_status(3, 1, packets::STATUS_ATK_MODIFIER);

    assert!(
        broadcasts_cleanse.contains(&purge_disable),
        "Cleanse must broadcast purge_status(DISABLE) on cleansed ally (3, 1)"
    );
    assert!(
        broadcasts_cleanse.contains(&clear_seal),
        "Cleanse must broadcast clear_unit_status(STATUS_SEAL_RESIST_PCT) on cleansed ally"
    );
    assert!(
        broadcasts_cleanse.contains(&purge_debuff),
        "Cleanse must broadcast purge_status(DEBUFF) on cleansed ally (3, 1)"
    );
    assert!(
        broadcasts_cleanse.contains(&clear_debuff_def),
        "Cleanse must broadcast clear_unit_status(STATUS_DEF_MODIFIER) on cleansed ally"
    );
    assert!(
        broadcasts_cleanse.contains(&clear_debuff_atk),
        "Cleanse must broadcast clear_unit_status(STATUS_ATK_MODIFIER) on cleansed ally"
    );
}

#[tokio::test]
async fn battle_manager_turn_pacing_and_frame_sequence_integration() {
    let mut battle = Battle::new(42, 0).with_opening_dialog(999);
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 101;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 1;
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 501;
        c.hp = 1000;
        c.hp_max = 1000;
        c.team = 2;
    }

    let sink = Arc::new(CapturingSink::default());
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(150),
        sink.clone(),
    );

    // Release handshake barrier immediately
    handle.ack_scene();

    // Submit an attack command
    let submit_ok = handle.command(ts_dream::battle::manager::PlayerInput {
        player: 101,
        cmd: BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    });
    assert!(submit_ok, "command submission must succeed");

    // Wait for the turn to complete
    let deadline = tokio::time::Instant::now() + Duration::from_millis(800);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames
                .iter()
                .any(|(_, f)| f.starts_with("F44404003503")) // force_state7
                && frames.iter().any(|(_, f)| f == &packets::turn_action_finish())
            {
                break;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out waiting for turn frames in battle manager");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let frames = sink.frames.lock().unwrap();
    let frame_strings: Vec<&str> = frames.iter().map(|(_, f)| f.as_str()).collect();

    // Validate opening dialogue 35 06 is emitted
    let talk_frame = packets::battle_npc_talk(1, 0, 999);
    assert!(
        frame_strings.iter().any(|f| *f == talk_frame.as_str()),
        "opening dialogue 35 06 must be emitted"
    );

    // Validate force_state7 35 03 is emitted
    let s7_frame = packets::force_state7(3, 2);
    assert!(
        frame_strings.iter().any(|f| *f == s7_frame.as_str()),
        "force_state7 35 03 must be emitted"
    );

    // Validate turn_action_finish 32 02 is emitted
    let finish_frame = packets::turn_action_finish();
    assert!(
        frame_strings.iter().any(|f| *f == finish_frame.as_str()),
        "turn_action_finish 32 02 must be emitted"
    );
}
