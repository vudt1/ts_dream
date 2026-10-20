//! Ticket 09 — `finish()` rewards by each cell's actual `team`/`typ`.
//!
//! `finish()` used to scan row 3 only for players and row 2 only for the
//! leader's pets, i.e. it assumed the player is always seated in row 3. The
//! team rule is `row < 2` ⇒ side 2 and `row >= 2` ⇒ side 1
//! (`battle::engine::team_for_row`, ticket 08), so a player can sit in row 2
//! and their pet then lives in row 3. The old scan silently skipped that
//! player, and it attributed *every* row-2 pet's exp to *every* row-3 player
//! (duplicate grants).
//!
//! The generalized scan walks `keys` in (row, col) creation order, keeps the
//! player cells of our side (`typ == 2 && team == 1`) whatever their row, and
//! routes each pet's exp to its own owner (`id_char`). The cell-clear cluster
//! is emitted per player cell as `(row, col)` + `(row ^ 1, col)`.

use std::collections::HashMap;

use ts_dream::battle::construction::Battle;
use ts_dream::battle::packets;
use ts_dream::battle::runner::{BattleData, DbTarget, Out, PlayerSnapshot, Stat};
use ts_dream::data::tables::{Item, Npc, Skill, TexpRow};

/// Two-row Texps table: threshold 18 at lv 1, unreachable at lv 2 — a texp of
/// 20 therefore grants exactly one level (`texp_get_lv_up`).
fn texps_one_level() -> Vec<TexpRow> {
    vec![
        TexpRow {
            lv: 0,
            reborn: [0, 0, 0, 0],
        },
        TexpRow {
            lv: 1,
            reborn: [18, 18, 18, 18],
        },
        TexpRow {
            lv: 2,
            reborn: [1_000_000, 1_000_000, 1_000_000, 1_000_000],
        },
    ]
}

/// Snapshot with a clean Texps counter and no bonus HP/SP.
fn snapshot() -> PlayerSnapshot {
    PlayerSnapshot {
        texp: 0,
        job: 0,
        hpx: 0,
        spx: 0,
        hpx2: 0,
        spx2: 0,
    }
}

/// A player cell on our side (team 1), alive, level 1 with `exp` battle exp.
fn put_player(battle: &mut Battle, row: u8, col: u8, id: i64, exp: i64) {
    let c = battle.cell_mut(row, col).unwrap();
    c.typ = 2;
    c.id = id;
    c.leader_id = id;
    c.team = 1;
    c.lv = 1;
    c.hp = 100;
    c.hp_max = 100;
    c.sp = 50;
    c.sp_max = 100;
    c.exp = exp;
}

/// A pet cell owned by `owner` (`stt` = pet slot), carrying `exp` battle exp.
fn put_pet(battle: &mut Battle, row: u8, col: u8, id: i64, owner: i64, stt: i64, exp: i64) {
    let c = battle.cell_mut(row, col).unwrap();
    c.typ = 4;
    c.id = id;
    c.id_char = owner;
    c.id_npc_on_map = stt;
    c.team = 1;
    c.lv = 1;
    c.hp = 50;
    c.hp_max = 50;
    c.exp = exp;
}

/// Count the DB writes for one target/stat pair.
fn db_count(out: &[Out], target: DbTarget, stat: Stat) -> usize {
    out.iter()
        .filter(|o| matches!(o, Out::Db(u) if u.target == target && u.stat == stat))
        .count()
}

/// The stats emitted for one player, in order.
fn stat_seq(out: &[Out], player: i64) -> Vec<Stat> {
    out.iter()
        .filter_map(|o| match o {
            Out::Db(u) if u.target == DbTarget::Player(player) => Some(u.stat),
            _ => None,
        })
        .collect()
}

/// ---- Players in rows 2 and 3 both get the full reward burst --------------

#[test]
fn finish_rewards_players_in_rows_two_and_three() {
    // Grid (our side = team 1):
    //   (2,0) player B 202  ← pet B at (3,0), the front cell of B's col
    //   (2,2) pet A (owner 101)
    //   (3,0) pet B (owner 202)
    //   (3,2) player A 101
    //   (0,2) enemy player 999 (team 2) — must get nothing.
    let mut battle = Battle::new(1, 0);
    put_player(&mut battle, 3, 2, 101, 20);
    put_pet(&mut battle, 2, 2, 5001, 101, 1, 7);
    put_player(&mut battle, 2, 0, 202, 20);
    put_pet(&mut battle, 3, 0, 5002, 202, 2, 9);
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 2;
        c.id = 999;
        c.team = 2;
        c.lv = 1;
        c.hp = 100;
        c.hp_max = 100;
        c.exp = 100;
    }

    let mut players = HashMap::new();
    players.insert(101, snapshot());
    players.insert(202, snapshot());
    let (npcs, skills, items, pet_slots) = (
        HashMap::<i64, Npc>::new(),
        HashMap::<i64, Skill>::new(),
        HashMap::<i64, Item>::new(),
        HashMap::<i64, [i64; 4]>::new(),
    );
    let texps = texps_one_level();
    let data = BattleData::new(
        &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
    );

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    // Each player receives the Texp → Lv → … → SkillPoint burst exactly once,
    // whichever row they sit in.
    let expected_burst = vec![
        Stat::Texp,
        Stat::Lv,
        Stat::Hpmax,
        Stat::Hp,
        Stat::Spmax,
        Stat::Sp,
        Stat::Point,
        Stat::SkillPoint,
    ];
    assert_eq!(
        stat_seq(&out, 101),
        expected_burst,
        "row-3 player 101 must get the full reward burst once"
    );
    assert_eq!(
        stat_seq(&out, 202),
        expected_burst,
        "row-2 player 202 must get the full reward burst once (regression: old scan skipped row 2)"
    );

    // `05 0A` LevelUP is emitted for each leveled character.
    assert!(out.contains(&Out::Broadcast(packets::level_up(101))));
    assert!(out.contains(&Out::Broadcast(packets::level_up(202))));

    // Every pet's exp follows its owner, exactly once.
    let pet_exps: Vec<(i64, i64, i64)> = out
        .iter()
        .filter_map(|o| match o {
            Out::PetExp { owner, stt, exp } => Some((*owner, *stt, *exp)),
            _ => None,
        })
        .collect();
    assert_eq!(
        pet_exps,
        vec![(202, 2, 9), (101, 1, 7)],
        "pet exp must be granted once per pet, to the pet's own owner"
    );

    // The enemy-side player cell is untouched.
    assert!(
        !out.iter()
            .any(|o| matches!(o, Out::Db(u) if u.target == DbTarget::Player(999))),
        "team-2 player 999 must not receive any reward"
    );
    assert!(
        !out.iter()
            .any(|o| matches!(o, Out::PetExp { owner, .. } if *owner == 999)),
        "team-2 owner 999 must not receive pet exp"
    );
}

/// ---- Cell-clear cluster targets the correct (player, front) pair ---------

#[test]
fn finish_clear_cluster_matches_each_player_cell_pair() {
    let mut battle = Battle::new(1, 0);
    put_player(&mut battle, 3, 2, 101, 0); // no exp → no reward frames
    put_pet(&mut battle, 2, 2, 5001, 101, 1, 0);
    put_player(&mut battle, 2, 0, 202, 0);
    put_pet(&mut battle, 3, 0, 5002, 202, 2, 0);

    let mut players = HashMap::new();
    players.insert(101, snapshot());
    players.insert(202, snapshot());
    let (npcs, skills, items, pet_slots) = (
        HashMap::<i64, Npc>::new(),
        HashMap::<i64, Skill>::new(),
        HashMap::<i64, Item>::new(),
        HashMap::<i64, [i64; 4]>::new(),
    );
    let texps = texps_one_level();
    let data = BattleData::new(
        &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
    );

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    // Keys scan order is by row then col ⇒ player (2,0) first, then (3,2).
    let expected = format!(
        "{}{}{}{}{}{}",
        packets::hide_from_map(202),
        packets::reset_cell(2, 0),
        packets::reset_cell(3, 0),
        packets::hide_from_map(101),
        packets::reset_cell(3, 2),
        packets::reset_cell(2, 2),
    );
    assert!(
        out.contains(&Out::Broadcast(expected)),
        "cell-clear cluster must pair each player cell with its own front cell, got {out:?}"
    );

    // The hide frame is also broadcast to the player's map, once per player.
    for id in [101u32, 202] {
        let count = out
            .iter()
            .filter(|o| matches!(o, Out::MapBroadcast { frame, .. } if *frame == packets::hide_from_map(id)))
            .count();
        assert_eq!(count, 1, "exactly one map hide frame for {id}");
    }
}

/// ---- A lone row-2 player still gets rewarded -----------------------------

#[test]
fn finish_rewards_a_player_seated_only_in_row_two() {
    // Under the old hardcoded row-3 scan this configuration produced nothing.
    let mut battle = Battle::new(1, 0);
    put_player(&mut battle, 2, 0, 303, 20);
    put_pet(&mut battle, 3, 0, 5003, 303, 3, 11);

    let mut players = HashMap::new();
    players.insert(303, snapshot());
    let (npcs, skills, items, pet_slots) = (
        HashMap::<i64, Npc>::new(),
        HashMap::<i64, Skill>::new(),
        HashMap::<i64, Item>::new(),
        HashMap::<i64, [i64; 4]>::new(),
    );
    let texps = texps_one_level();
    let data = BattleData::new(
        &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
    );

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    assert_eq!(db_count(&out, DbTarget::Player(303), Stat::Texp), 1);
    assert_eq!(db_count(&out, DbTarget::Player(303), Stat::Lv), 1);
    assert_eq!(db_count(&out, DbTarget::Player(303), Stat::Point), 1);
    assert!(out.contains(&Out::Broadcast(packets::level_up(303))));
    assert_eq!(
        out.iter()
            .filter(|o| matches!(o, Out::PetExp { owner: 303, stt: 3, exp: 11 }))
            .count(),
        1,
        "the row-2 player's pet gets its exp once"
    );
}

/// ---- Pets on the same row are not double-granted -------------------------

#[test]
fn finish_does_not_duplicate_pet_exp_across_players() {
    // Two players on row 3, each with a pet in row 2. The old scan granted
    // every row-2 pet's exp to every row-3 player.
    let mut battle = Battle::new(1, 0);
    put_player(&mut battle, 3, 2, 101, 0);
    put_pet(&mut battle, 2, 2, 5001, 101, 1, 7);
    put_player(&mut battle, 3, 1, 202, 0);
    put_pet(&mut battle, 2, 1, 5002, 202, 2, 9);

    let mut players = HashMap::new();
    players.insert(101, snapshot());
    players.insert(202, snapshot());
    let (npcs, skills, items, pet_slots) = (
        HashMap::<i64, Npc>::new(),
        HashMap::<i64, Skill>::new(),
        HashMap::<i64, Item>::new(),
        HashMap::<i64, [i64; 4]>::new(),
    );
    let texps = texps_one_level();
    let data = BattleData::new(
        &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
    );

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    let pet_exps: Vec<(i64, i64, i64)> = out
        .iter()
        .filter_map(|o| match o {
            Out::PetExp { owner, stt, exp } => Some((*owner, *stt, *exp)),
            _ => None,
        })
        .collect();
    // Scan order is (row, col) ⇒ player (3,1) [202] before player (3,2) [101].
    assert_eq!(
        pet_exps,
        vec![(202, 2, 9), (101, 1, 7)],
        "each pet's exp is granted exactly once, to its own owner"
    );
}
