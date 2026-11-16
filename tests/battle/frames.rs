//! Ticket 02 — frame `33 01` (in-battle HP/SP refresh).
//!
//! Byte layout is re-derived from client `aLogin.exe` `FUN_0064cae0`
//! (`.scratch/client-pseudo-op-code/battle/02_battle_positions.md §2.5`) using
//! the `HANDOFF-client-reverse.md §0.2` index convention (`RP[0]` = sub,
//! `content[j] == RP[j-1]`):
//!
//! ```text
//! content[0]    = 0x33          MainOp
//! content[1]    = 0x01          SubOp
//! record i = content[2 + 7i .. 2 + 7i + 7]
//!     +0        grp   (0..3)
//!     +1        slot  (0..4)
//!     +2        propId  (0x19 = HP, 0x1A = SP)
//!     +3..+6    value u32 LE
//! ```
//!
//! `propId 0x19` writes `unit+0x3EA` (HP) and, with `value > 0` on a dead unit,
//! runs the revive effect (`0064cae0.c:124-180`). `propId 0x1A` writes
//! `unit+0xFB` (SP).

use std::collections::HashMap;

use ts_dream::battle::construction::Battle;
use ts_dream::battle::packets;
use ts_dream::battle::runner::{BattleCommands, BattleData, Out};
use ts_dream::data::tables::{Item, Npc, Skill, TexpRow};

/// Payload length declared in the `F444` header, in bytes.
fn frame_len(frame: &str) -> usize {
    let lo = u8::from_str_radix(&frame[4..6], 16).unwrap() as usize;
    let hi = u8::from_str_radix(&frame[6..8], 16).unwrap() as usize;
    lo | (hi << 8)
}

/// Total on-wire length implied by the hex string.
fn frame_total_bytes(frame: &str) -> usize {
    frame.len() / 2
}

/// ---- Builder: byte-exact `33 01` layouts ---------------------------------

#[test]
fn set_hp_frame_is_byte_exact() {
    // grp 3, slot 2, propId 0x19, value 90 (0x5A).
    let frame = packets::set_hp(3, 2, 90);
    assert_eq!(frame, "F444090033010302195A000000");
    assert_eq!(frame_len(&frame), 9, "payload = 3301 + 7-byte record");
    assert_eq!(frame_total_bytes(&frame), 13, "4-byte header + 9-byte payload");
}

#[test]
fn set_sp_frame_is_byte_exact() {
    // propId 0x1A = SP, value 350 (0x15E).
    let frame = packets::set_sp(2, 3, 350);
    assert_eq!(frame, "F4440900330102031A5E010000");
    assert_eq!(frame_len(&frame), 9);
}

#[test]
fn damage_frame_records_damaged_hp() {
    // grp 0, slot 2, HP reduced to 350 (0x15E).
    let frame = packets::set_hp(0, 2, 350);
    assert_eq!(frame, "F444090033010002195E010000");
    assert_eq!(frame_len(&frame), 9);
}

#[test]
fn revive_frame_is_hp_with_positive_value() {
    // Revive = HP 0 -> >0, still propId 0x19 with value > 0 so the client runs
    // the revive effect (02_battle_positions.md §2.5, 0064cae0.c:124-180).
    let frame = packets::set_hp(0, 1, 100);
    assert_eq!(frame, "F4440900330100011964000000");
    assert_eq!(frame_len(&frame), 9);
    // Value (u32 LE at hex 18..26) must never be 0 for a revive.
    assert_eq!(&frame[18..26], "64000000");
    assert_ne!(&frame[18..26], "00000000");
}

#[test]
fn multiple_records_pack_into_one_frame() {
    // Two 7-byte records: HP 90 then SP 60 at the same cell.
    let frame = packets::set_unit_prop_frame(&[
        (3, 2, packets::unit_prop::HP, 90),
        (3, 2, packets::unit_prop::SP, 60),
    ]);
    assert_eq!(frame, "F444100033010302195A00000003021A3C000000");
    assert_eq!(frame_len(&frame), 16, "payload = 3301 + 2 * 7");
    assert_eq!(frame_total_bytes(&frame), 20, "4-byte header + 16-byte payload");
}

#[test]
fn single_record_helper_matches_manual_record() {
    // `set_unit_prop` is the 7-byte record primitive behind `set_hp`/`set_sp`.
    let record = packets::set_unit_prop(3, 2, packets::unit_prop::HP, 90);
    assert_eq!(record, "0302195A000000");
    assert_eq!(record.len() / 2, 7);
    assert_eq!(packets::set_hp(3, 2, 90), format!("F44409003301{record}"));
}

#[test]
fn negative_value_clamps_to_zero() {
    // HP/SP are unsigned on the wire; guard against overflow/wrap.
    let record = packets::set_unit_prop(1, 1, packets::unit_prop::HP, -5);
    assert_eq!(record, "01011900000000");
}

/// ---- Runner: HP/SP changes reach the wire (not only the DB) --------------

fn table() -> (HashMap<i64, Npc>, HashMap<i64, Skill>, HashMap<i64, Item>) {
    (HashMap::new(), HashMap::new(), HashMap::new())
}

fn entity_data<'a>(
    npcs: &'a HashMap<i64, Npc>,
    skills: &'a HashMap<i64, Skill>,
    items: &'a HashMap<i64, Item>,
    pet_slots: &'a HashMap<i64, [i64; 4]>,
    players: &'a HashMap<i64, ts_dream::battle::runner::PlayerSnapshot>,
    texps: &'a [TexpRow],
) -> BattleData<'a> {
    BattleData::new(npcs, skills, items, pet_slots, players, texps, None, 0, 0)
}

#[test]
fn burn_damage_broadcasts_hp_frame() {
    let (npcs, skills, items) = table();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();

    let mut battle = Battle::new(1, 0);
    // Player cell: HP 100, burning (Type3 10004) for 10 dmg per tick.
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 7;
        c.leader_id = 7;
        c.lv = 10;
        c.hp = 100;
        c.hp_max = 100;
        c.sp = 50;
        c.sp_max = 100;
        c.team = 1;
        c.type3_id = 10004;
        c.type3_lv = 0; // dmg = 10 + lv*2 = 10
        c.type3_turn = 5;
    }
    // A living enemy keeps the battle running; it never acts (no skill).
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.hp = 100;
        c.hp_max = 100;
        c.team = 2;
        c.type3_turn = 1;
    }

    let data = entity_data(&npcs, &skills, &items, &pet_slots, &players, &texps);
    let mut out = Vec::new();
    battle.run_turn(&data, &BattleCommands::new(), &mut out);

    // HP 100 - 10 = 90, broadcast as a 33 01 HP record for (grp 3, slot 2).
    assert!(
        out.contains(&Out::Broadcast(packets::set_hp(3, 2, 90))),
        "burn damage must broadcast 33 01 HP update, got {out:?}"
    );
}

#[test]
fn leader_sp_regen_broadcasts_sp_frame() {
    let (npcs, skills, items) = table();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();

    let mut battle = Battle::new(1, 0);
    // SP regen = Round(qs.Int / 15) = Round(150 / 15) = 10.
    battle.leader_id_qs = 1;
    battle.leader_qs_int = 150;
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 7;
        c.leader_id = 7;
        c.hp = 100;
        c.hp_max = 100;
        c.sp = 50;
        c.sp_max = 100;
        c.team = 1;
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.hp = 100;
        c.hp_max = 100;
        c.team = 2;
        c.type3_turn = 1;
    }

    let data = entity_data(&npcs, &skills, &items, &pet_slots, &players, &texps);
    let mut out = Vec::new();
    battle.run_turn(&data, &BattleCommands::new(), &mut out);

    // SP 50 + 10 = 60, broadcast as a 33 01 SP record for (grp 3, slot 2).
    assert!(
        out.contains(&Out::Broadcast(packets::set_sp(3, 2, 60))),
        "SP regen must broadcast 33 01 SP update, got {out:?}"
    );
}

#[test]
fn turn_action_finish_frame_is_byte_exact() {
    let frame = packets::turn_action_finish();
    assert_eq!(frame, "F44402003202");
    assert_eq!(frame_len(&frame), 2);
    assert_eq!(frame_total_bytes(&frame), 6);
}

#[test]
fn run_turn_emits_turn_action_finish() {
    let (npcs, skills, items) = table();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();

    let mut battle = Battle::new(1, 0);
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 7;
        c.hp = 100;
        c.hp_max = 100;
        c.team = 1;
    }
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.hp = 100;
        c.hp_max = 100;
        c.team = 2;
    }

    let data = entity_data(&npcs, &skills, &items, &pet_slots, &players, &texps);
    let mut out = Vec::new();
    battle.run_turn(&data, &BattleCommands::new(), &mut out);

    assert_eq!(
        out.last(),
        Some(&Out::Broadcast(packets::turn_action_finish())),
        "turn_action_finish (32 02) must be the last action broadcast in run_turn"
    );
}
