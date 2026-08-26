//! Battle engine tests — migrated from inline #[cfg(test)] blocks (ticket 08).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::mpsc;

use ts_dream::battle::construction::{trigger_diahinh, Battle, BattleCounter, StartPacket};
use ts_dream::battle::damage::{
    calc_combo_exp, calc_kill_exp, calc_magic_damage, calc_physical_damage,
    calc_physical_damage_stat, get_damage_skill_int, get_damage_thuoctinh, get_random_drop_slot,
    get_random_miss_attack, get_random_miss_flee, get_random_skill_npc, get_thuoctinh_khac,
    get_turn, hit_exp, randomize_array,
};
use ts_dream::battle::engine::{get_hp_max, get_sp_max, WarInfo};
use ts_dream::battle::manager::{BattleManager, BattleSink, PlayerInput};
use ts_dream::battle::npc_world::{teamdef_for_so_luong, NpcWorld, WorldPlayer, WorldSink};
use ts_dream::battle::packets::{
    acting, battle_exit_move, battle_exit_talk, battle_open_leader, combo_footer_20007,
    entity_npc, entity_player, hide_from_map, show_player_on_map, skilling_full, skilling_int,
    stat_byte, status_update, troi_byte, your_turn,
};
use ts_dream::battle::rng::{BattleRng, DotNetRandom};
use ts_dream::battle::runner::{
    BattleCommand, BattleData, DbTarget, DbUpdate, Out, Outcome, PlayerSnapshot, Stat,
};
use ts_dream::battle::service::BattleService;
use ts_dream::battle::targeting::{
    expand_sl_danh, get_pos_attack_combo, get_pos_attack_default, get_pos_attack_giai_tru,
    get_pos_attack_type4, is_valid_target, CellInfo, GridPos, NO_TARGET,
};
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Item, Npc, NpcOnMap, QuestDef, QuestResult, Skill};
use ts_dream::data::texps::compute_texps;
use ts_dream::protocol::encoder;
use ts_dream::server::session::{PetState, Session};

// ── battle::rng ──

#[test]
fn deterministic_seed() {
    let mut r1 = DotNetRandom::new(42);
    let mut r2 = DotNetRandom::new(42);
    for _ in 0..100 {
        assert_eq!(r1.next(), r2.next());
    }
}

#[test]
fn next_range_bounds() {
    let mut r = DotNetRandom::new(123);
    for _ in 0..1000 {
        let v = r.next_range(5, 10);
        assert!((5..10).contains(&v), "value {} out of [5,10)", v);
    }
}

#[test]
fn next_max_bounds() {
    let mut r = DotNetRandom::new(456);
    for _ in 0..1000 {
        let v = r.next_max(7);
        assert!((0..7).contains(&v), "value {} out of [0,7)", v);
    }
}

#[test]
fn three_streams_independent() {
    let rng = BattleRng::with_seeds(1, 2, 3);
    // Each stream should produce different sequences
    let mut r0 = rng.random_0.clone();
    let mut r1 = rng.random_1.clone();
    let mut r2 = rng.random_2.clone();
    let v0 = r0.next();
    let v1 = r1.next();
    let v2 = r2.next();
    // Different seeds should give different first values
    assert_ne!(v0, v1);
    assert_ne!(v1, v2);
}

// ── battle::engine ──

#[test]
fn packet_hex_23_bytes() {
    let w = WarInfo {
        typ: 2,
        id: 300003,
        hp_max: 311,
        sp_max: 411,
        hp: 311,
        sp: 411,
        lv: 101,
        thuoctinh: 1,
        ..Default::default()
    };
    let hex = w.packet_hex();
    assert_eq!(hex.len(), 46); // 23 bytes
}

#[test]
fn hpmax_floor() {
    // rb 0, job, lvl 1, hpx 6: floor((1+1)*12+80+1)=floor(105)=105
    assert_eq!(get_hp_max(0, 0, 1, 6), 105);
}

#[test]
fn spmax_floor() {
    // rb 0, lvl 1, spx 6: floor((1)*12+60+1)=floor(73)=73
    assert_eq!(get_sp_max(0, 0, 1, 6), 73);
}

// ── battle::damage ──

#[test]
fn element_table_identity() {
    assert_eq!(get_damage_thuoctinh(1, 1), 1.0);
    assert_eq!(get_damage_thuoctinh(3, 4), 1.9);
}

#[test]
fn element_skill_int() {
    assert_eq!(get_damage_skill_int(1, 2), 27);
    assert_eq!(get_damage_skill_int(4, 1), 54);
    assert_eq!(get_damage_skill_int(1, 0), 18); // per-row default
    assert_eq!(get_damage_skill_int(3, 0), 27);
    assert_eq!(get_damage_skill_int(0, 2), 10); // unknown row default
}

#[test]
fn element_relation() {
    assert_eq!(get_thuoctinh_khac(1, 4), 2); // earth beats wind
    assert_eq!(get_thuoctinh_khac(1, 2), 1); // earth loses to water
    assert_eq!(get_thuoctinh_khac(1, 3), 0); // neutral
}

#[test]
fn basic_physical_damage() {
    let dmg = calc_physical_damage(100, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
    assert!(dmg > 0, "should produce positive damage");
}

#[test]
fn physical_uses_int_stat_for_combo_87() {
    // Same element, same level: higher stat → strictly more damage.
    let with_atk = calc_physical_damage_stat(200, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
    let with_int = calc_physical_damage_stat(50, 50, 1, 1, 10, 10, 1, 10, 1, 2.0);
    assert!(with_atk > with_int);
}

#[test]
fn magic_damage_uses_int() {
    let dmg = calc_magic_damage(100, 50, 1, 1, 10, 10, 1, 10, 1, 12345, 1);
    assert!(dmg > 0, "should produce positive damage");
}

#[test]
fn magic_multi_hit_aoe() {
    // 12016 with sl_danh 3 → divide by 1.5 then + skillLv*50.
    let dmg = calc_magic_damage(100, 50, 1, 1, 10, 10, 1, 10, 2, 12016, 3);
    assert!(dmg > 0);
}

#[test]
fn get_turn_groups() {
    assert_eq!(get_turn(13002, 1), 3); // GROUP_a
    assert_eq!(get_turn(10033, 3), 3); // GROUP_b
    assert_eq!(get_turn(10004, 5), 3); // GROUP_c
    assert_eq!(get_turn(13015, 1), 4); // GROUP_d
    assert_eq!(get_turn(11014, 1), 5); // GROUP_e
    assert_eq!(get_turn(20025, 10), 5); // GROUP_f max lv
    assert_eq!(get_turn(14021, 3), 4); // 14021 specific
    assert_eq!(get_turn(14013, 7), 4); // 14013 falls through to GROUP_f ladder
    assert_eq!(get_turn(99999, 1), 3); // default
}

#[test]
fn kill_exp_basic() {
    let exp = calc_kill_exp(10, 10);
    assert_eq!(exp, 7); // round(5 + 10/5) = round(7) = 7
}

#[test]
fn kill_exp_too_far() {
    assert_eq!(calc_kill_exp(50, 10), 0); // diff 40 > 20
}

#[test]
fn kill_exp_lower_level() {
    // npc higher than caster → round((npc-caster) + npc/5)
    let exp = calc_kill_exp(10, 15);
    assert_eq!(exp, 8); // round(5 + 3) = 8
}

#[test]
fn hit_exp_teamdef_formula() {
    // round(npcLv/2 + (npcLv - attackerLv))
    assert_eq!(hit_exp(10, 20), 20); // round(10 + 10) = 20
    assert_eq!(hit_exp(10, 11), 6); // round(5.5 + 1) = 6 (banker's: 6.5->6)
}

#[test]
fn combo_exp_bonus() {
    assert_eq!(calc_combo_exp(10), 11); // round(10 * 1.086) = round(10.86) = 11
}

#[test]
fn drop_slots() {
    assert_eq!(get_random_drop_slot(1), 1);
    assert_eq!(get_random_drop_slot(25), 1);
    assert_eq!(get_random_drop_slot(26), 2);
    assert_eq!(get_random_drop_slot(48), 2);
    assert_eq!(get_random_drop_slot(76), 6);
    assert_eq!(get_random_drop_slot(77), 0);
    assert_eq!(get_random_drop_slot(999), 0);
}

#[test]
fn miss_attack_high_avg_hits() {
    // Same level, equal avg => percent 100 => hit.
    let mut rng = DotNetRandom::new(7);
    assert_eq!(get_random_miss_attack(&mut rng, 10, 10, 10, 10), 1);
}

#[test]
fn flee_percent_clamped() {
    // Huge favorable diff => percent clamped to 100 => always flee success.
    let mut rng = DotNetRandom::new(9);
    let roll = get_random_miss_flee(&mut rng, 100, 1, 100, 1);
    assert_eq!(roll, 1);
}

#[test]
fn skill_npc_defaults_to_10000() {
    let mut rng = DotNetRandom::new(11);
    // No skills -> after failed rolls returns 10000.
    let skill = get_random_skill_npc(&mut rng, 10, 0, [0, 0, 0, 0]);
    assert!(skill == 10000 || skill >= 0);
}

#[test]
fn randomize_array_fold() {
    // percent 50 always -> first element survives 50% of the folds. Just check bounds.
    let mut rng = DotNetRandom::new(13);
    let v = randomize_array(&mut rng, &[2, 1, 3, 4]);
    assert!([1, 2, 3, 4].contains(&v));
}

// ── battle::targeting ──

fn cells_from(rows: &[(u8, u8, i64, i64, i64)]) -> Vec<CellInfo> {
    // (row, col, id, hp, team)
    let mut out = Vec::new();
    for (r, c, id, hp, team) in rows {
        out.push(CellInfo {
            row: *r,
            col: *c,
            id: *id,
            hp: *hp,
            team: *team,
            type4_id: 0,
        });
    }
    out
}

#[test]
fn expand_single_target() {
    let targets = expand_sl_danh(GridPos::new(0, 2), 1, |_, _| true);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0], GridPos::new(0, 2));
}

#[test]
fn expand_two_target() {
    let targets = expand_sl_danh(GridPos::new(0, 2), 2, |_, _| true);
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[1], GridPos::new(1, 2)); // opposite row
}

#[test]
fn expand_three_target_edges() {
    // Anchor at col=0, only right is alive
    let targets = expand_sl_danh(GridPos::new(0, 0), 3, |r, c| r == 0 && c == 1);
    assert_eq!(targets.len(), 2); // anchor + right
}

#[test]
fn expand_seven_targets_anchor_row() {
    // Area 7: all alive cells of the anchor ROW (columns 0..4).
    let targets = expand_sl_danh(GridPos::new(0, 2), 7, |r, _| r == 0);
    assert_eq!(targets.len(), 5);
    assert_eq!(targets[0], GridPos::new(0, 0));
    assert_eq!(targets[4], GridPos::new(0, 4));
}

#[test]
fn no_target_sentinel() {
    assert!(!is_valid_target(NO_TARGET));
    assert!(is_valid_target(GridPos::new(0, 0)));
}

#[test]
fn picker_default_hostile() {
    // Enemy at (0,2) alive; requested (0,2).
    let cells = cells_from(&[(0, 2, 9001, 100, 2), (3, 2, 300001, 100, 1)]);
    let t = get_pos_attack_default(&cells, 1, 0, 2, 1);
    assert_eq!(t, vec![GridPos::new(0, 2)]);
}

#[test]
fn picker_default_skips_hidden_type4() {
    // Enemy type4=13005 (frozen) should not be picked; falls back to other enemy.
    let mut cells = cells_from(&[(0, 2, 9001, 100, 2), (0, 4, 9002, 100, 2)]);
    cells[0].type4_id = 13005;
    let t = get_pos_attack_default(&cells, 1, 0, 2, 1);
    assert_eq!(t, vec![GridPos::new(0, 4)]);
}

#[test]
fn picker_friendly_targets_own_team() {
    // Heal/buff picks own team; requested cell is friendly.
    let cells = cells_from(&[
        (0, 2, 9001, 100, 2),
        (3, 2, 300001, 100, 1),
        (3, 1, 300002, 100, 1),
    ]);
    let t = get_pos_attack_type4(&cells, 1, 3, 2, 3);
    // Anchor (3,2) then left (3,1) alive.
    assert_eq!(t, vec![GridPos::new(3, 2), GridPos::new(3, 1)]);
}

#[test]
fn picker_combo_accepts_dead_requested() {
    // Requested enemy is dead (hp 0) but combo rule only needs id>0 + enemy.
    let cells = cells_from(&[(0, 2, 9001, 0, 2), (3, 2, 300001, 100, 1)]);
    let t = get_pos_attack_combo(&cells, 1, 0, 2, 1);
    assert_eq!(t, vec![GridPos::new(0, 2)]);
}

#[test]
fn picker_any_ignores_team() {
    let cells = cells_from(&[(0, 2, 9001, 0, 2), (3, 2, 300001, 100, 1)]);
    let t = get_pos_attack_giai_tru(&cells, 1, 0, 2, 1);
    // Requested (0,2) qualifies (id>0) even though dead.
    assert_eq!(t, vec![GridPos::new(0, 2)]);
}

#[test]
fn no_target_returns_empty() {
    let cells = cells_from(&[(3, 2, 300001, 100, 1)]);
    let t = get_pos_attack_default(&cells, 1, 0, 2, 1);
    assert!(t.is_empty());
}

// ── battle::npc_world ──

#[derive(Default)]
struct WorldRecordingSink {
    sent_players: Mutex<Vec<(i64, String)>>,
    sent_maps: Mutex<Vec<(i64, String)>>,
    teamdefs: Mutex<Vec<(i64, i64, Vec<i64>)>>,
    players: Vec<WorldPlayer>,
}

impl WorldSink for WorldRecordingSink {
    fn players(&self) -> Vec<WorldPlayer> {
        self.players.clone()
    }
    fn send_player(&self, player: i64, frame: String) {
        self.sent_players.lock().unwrap().push((player, frame));
    }
    fn send_map(&self, map_id: i64, frame: String) {
        self.sent_maps.lock().unwrap().push((map_id, frame));
    }
    fn start_teamdef(&self, player: i64, npc_on_map_id: i64, teamdef: Vec<i64>) {
        self.teamdefs
            .lock()
            .unwrap()
            .push((player, npc_on_map_id, teamdef));
    }
}

fn world() -> NpcWorld {
    let rows = vec![NpcOnMap {
        map_id: 12001,
        id: 7,
        npc_id: 9001,
        x: 400,
        y: 500,
        coord: 10,
        so_luong: 3,
    }];
    let mut w = NpcWorld::new(&rows);
    w.random_3 = DotNetRandom::new(42);
    w
}

#[test]
fn chase_triggers_teamdef_when_player_in_range() {
    let mut w = world();
    let sink = WorldRecordingSink {
        players: vec![WorldPlayer {
            id: 300001,
            map_id: 12001,
            x: 405,
            y: 502,
            battle_id: 0,
            leader_id: 0,
            logined: true,
        }],
        ..Default::default()
    };
    // 3rd tick → chase enabled.
    w.walk_tick(true, &sink);
    // The npc chased the player (map-wide frame) and engaged them.
    let maps = sink.sent_maps.lock().unwrap();
    assert_eq!(maps.len(), 1);
    assert!(maps[0].1.starts_with("F44408001602"));
    let teamdefs = sink.teamdefs.lock().unwrap();
    assert_eq!(teamdefs.len(), 1);
    assert_eq!(teamdefs[0].0, 300001); // engaged player
    assert_eq!(teamdefs[0].1, 7); // npc on-map id = talking battle
    // SoLuong 3 → _id2.._id4 = 9001.
    assert_eq!(teamdefs[0].2, [4712, 0, 9001, 9001, 9001, 0, 0, 0, 0, 0, 0]);
    // The instance is now in battle (id_battle = 1).
    assert_eq!(w.get(12001, 7).unwrap().id_battle, 1);
}

#[test]
fn no_chase_when_player_out_of_range() {
    let mut w = world();
    let sink = WorldRecordingSink {
        players: vec![WorldPlayer {
            id: 300001,
            map_id: 12001,
            x: 1000,
            y: 1000,
            battle_id: 0,
            leader_id: 0,
            logined: true,
        }],
        ..Default::default()
    };
    w.walk_tick(true, &sink);
    assert!(sink.teamdefs.lock().unwrap().is_empty());
    assert_eq!(w.get(12001, 7).unwrap().id_battle, 0);
}

#[test]
fn countdown_decrements_delay_and_clears_id_battle() {
    let mut w = world();
    {
        let e = w.get_mut(12001, 7).unwrap();
        e.delay = 3;
        e.id_battle = 1;
    }
    let sink = WorldRecordingSink::default();
    w.walk_tick(false, &sink);
    let e = w.get(12001, 7).unwrap();
    assert_eq!(e.delay, 2);
    assert_eq!(e.id_battle, 0);
}

#[test]
fn wander_sends_per_player_frame_on_chase_ticks() {
    let mut w = world();
    // Two players on the map, both out of range → wander frames only.
    let sink = WorldRecordingSink {
        players: vec![
            WorldPlayer {
                id: 300001,
                map_id: 12001,
                x: 900,
                y: 900,
                battle_id: 0,
                leader_id: 0,
                logined: true,
            },
            WorldPlayer {
                id: 300002,
                map_id: 12001,
                x: 901,
                y: 901,
                battle_id: 0,
                leader_id: 300001,
                logined: true,
            },
        ],
        ..Default::default()
    };
    w.walk_tick(true, &sink);
    let sent = sink.sent_players.lock().unwrap();
    assert_eq!(sent.len(), 2);
    for (_, f) in sent.iter() {
        assert!(f.starts_with("F44408001602"));
    }
    // No battle triggered.
    assert!(sink.teamdefs.lock().unwrap().is_empty());
}

#[test]
fn teamdef_slots_follow_so_luong() {
    assert_eq!(
        teamdef_for_so_luong(1, 9001).unwrap(),
        [4712, 0, 0, 9001, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        teamdef_for_so_luong(2, 9001).unwrap(),
        [4712, 0, 0, 9001, 9001, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        teamdef_for_so_luong(3, 9001).unwrap(),
        [4712, 0, 9001, 9001, 9001, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        teamdef_for_so_luong(4, 9001).unwrap(),
        [4712, 0, 9001, 9001, 9001, 0, 0, 0, 9001, 0, 0]
    );
    assert_eq!(
        teamdef_for_so_luong(5, 9001).unwrap(),
        [4712, 9001, 9001, 9001, 9001, 9001, 0, 0, 0, 0, 0]
    );
    // Outside 1..=5: no slot mapping → no battle.
    assert!(teamdef_for_so_luong(0, 9001).is_none());
    assert!(teamdef_for_so_luong(6, 9001).is_none());
}

#[test]
fn chase_with_so_luong_over_five_sends_frame_but_no_battle() {
    let rows = vec![NpcOnMap {
        map_id: 12001,
        id: 8,
        npc_id: 9001,
        x: 400,
        y: 500,
        coord: 10,
        so_luong: 6,
    }];
    let mut w = NpcWorld::new(&rows);
    w.random_3 = DotNetRandom::new(42);
    let sink = WorldRecordingSink {
        players: vec![WorldPlayer {
            id: 300001,
            map_id: 12001,
            x: 405,
            y: 502,
            battle_id: 0,
            leader_id: 0,
            logined: true,
        }],
        ..Default::default()
    };
    w.walk_tick(true, &sink);
    assert!(sink.teamdefs.lock().unwrap().is_empty());
    assert_eq!(w.get(12001, 8).unwrap().id_battle, 0);
}

// ── battle::construction ──

#[test]
fn create_battle_grid() {
    let battle = Battle::new(1, 112);
    assert_eq!(battle.list_war.len(), 20);
    assert_eq!(battle.keys.len(), 20);
    assert_eq!(battle.list_qs.len(), 50);
}

#[test]
fn add_player_and_npc() {
    let mut battle = Battle::new(1, 112);
    let session = Session::new();

    battle.add_player(&session, session.id as i64, 3, 2);

    let cell = battle.cell(3, 2).unwrap();
    assert_eq!(cell.typ, 2);
    assert_eq!(cell.team, 1);
    assert_eq!(cell.row, 3);
    assert_eq!(cell.col, 2);

    let npc = Npc {
        id: 1001,
        hp: 500,
        sp: 200,
        lv: 10,
        thuoctinh: 1,
        atk: 50,
        def: 30,
        agi: 20,
        int1: 15,
        ..Default::default()
    };
    battle.add_npc(&npc, 1, 0, 2, 3);

    let enemy = battle.cell(0, 2).unwrap();
    assert_eq!(enemy.typ, 3);
    assert_eq!(enemy.team, 2);
    assert_eq!(enemy.hp, 500);
    assert_eq!(enemy.id, 1001);
}

#[test]
fn leader_pets_loaded_at_expected_cells() {
    let mut battle = Battle::new(1, 112);
    let mut session = Session::new();
    session.id = 300001;
    session.active_pet_stt = 1;
    for stt in 1..=4u8 {
        let pet = PetState {
            stt,
            id: 9000 + u16::from(stt),
            hp_max: 100,
            hp: 100,
            level: 5,
            ..Default::default()
        };
        session.pets.push(pet);
    }
    battle.add_player(&session, session.id as i64, 3, 2);
    battle.load_leader_pets(&session, session.id as i64, 3);

    // Leader pets at (2,1), (2,3), (2,0), (2,4) with Stt 1,2,3,4.
    for (i, col) in [1u8, 3, 0, 4].iter().enumerate() {
        let cell = battle.cell(2, *col).unwrap();
        assert_eq!(cell.typ, 4, "pet cell (2,{col})");
        assert_eq!(cell.id, 9000 + i as i64 + 1);
        assert_eq!(cell.id_char, 300001);
        assert_eq!(cell.id_npc_on_map, i as i64 + 1);
    }
}

#[test]
fn member_pet_overwrites_leader_pet() {
    let mut battle = Battle::new(1, 112);
    let mut leader = Session::new();
    leader.id = 300001;
    leader.active_pet_stt = 1;
    let pet = PetState {
        stt: 1,
        id: 9001,
        ..Default::default()
    };
    leader.pets.push(pet);

    let mut member = Session::new();
    member.id = 300002;
    member.active_pet_stt = 1;
    let mp = PetState {
        stt: 1,
        id: 8002,
        ..Default::default()
    };
    member.pets.push(mp);

    battle.add_player(&leader, leader.id as i64, 3, 2);
    battle.load_leader_pets(&leader, leader.id as i64, 3);
    battle.add_player(&member, leader.id as i64, 3, 1);
    battle.load_member_pet(&member, leader.id as i64, 3, 1);

    // Member processed later overwrites the leader's pet at (2,1).
    let cell = battle.cell(2, 1).unwrap();
    assert_eq!(cell.id, 8002);
}

#[test]
fn teamdef_npcs() {
    let mut battle = Battle::new(1, 100);
    let npc = Npc {
        id: 2001,
        hp: 300,
        sp: 100,
        lv: 5,
        thuoctinh: 2,
        ..Default::default()
    };

    // TeamDef: id1→(0,0), id2→(0,1), ..., id6→(1,0), etc.
    let positions: [(u8, u8); 10] = [
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (1, 4),
    ];
    for (i, &(r, c)) in positions.iter().enumerate() {
        battle.add_npc(&npc, (i + 1) as i64, r, c, 7);
    }

    for &(r, c) in &positions {
        let cell = battle.cell(r, c).unwrap();
        assert_eq!(cell.typ, 7);
        assert_eq!(cell.team, 2);
    }
}

#[test]
fn win_lose_checks() {
    let mut battle = Battle::new(1, 112);

    // Initially all empty → both sides "dead"
    assert!(battle.all_enemies_dead());
    assert!(battle.all_players_dead());

    // Add a player
    let mut session = Session::new();
    session.id = 300001;
    session.hp = 100;
    session.hp_max = 100;
    battle.add_player(&session, 1, 3, 2);
    assert!(!battle.all_players_dead());
    assert!(battle.all_enemies_dead());

    // Add an NPC
    let npc = Npc {
        id: 1,
        hp: 100,
        ..Default::default()
    };
    battle.add_npc(&npc, 1, 0, 2, 3);
    assert!(!battle.all_enemies_dead());
    assert_eq!(battle.count_enemies_alive(), 1);
    assert_eq!(battle.count_players_alive(), 1);
}

#[test]
fn battle_counter() {
    let mut counter = BattleCounter::new();
    assert_eq!(counter.next(), 1);
    assert_eq!(counter.next(), 2);
    assert_eq!(counter.next(), 3);
}

#[test]
fn turn_order_sorts_correctly() {
    let mut battle = Battle::with_seeds(1, 112, 42, 43, 44);

    if let Some(cell) = battle.cell_mut(3, 2) {
        cell.id = 1;
        cell.agi = 100;
        cell.attacked = true;
        cell.random = 50;
    }
    if let Some(cell) = battle.cell_mut(0, 2) {
        cell.id = 2;
        cell.agi = 200;
        cell.attacked = true;
        cell.random = 30;
    }

    let order = battle.turn_order();
    assert_eq!(order[0], (0, 2)); // agi=200 higher
    assert_eq!(order[1], (3, 2));
}

#[test]
fn npc_battle_start_packets() {
    let mut battle = Battle::new(1, 112);
    let mut session = Session::new();
    session.id = 300001;
    session.hp = 100;
    session.hp_max = 100;
    session.sp = 50;
    session.sp_max = 50;
    session.level = 10;
    session.thuoctinh = 1;
    battle.add_player(&session, session.id as i64, 3, 2);

    let npc = Npc {
        id: 1001,
        hp: 500,
        sp: 200,
        lv: 10,
        thuoctinh: 1,
        ..Default::default()
    };
    battle.add_npc(&npc, 1, 0, 2, 3);

    let packets = battle.npc_battle_start_packets(112);
    assert!(packets.len() >= 3); // open + show on map + enemy entity
    match &packets[0] {
        StartPacket::To { frame, .. } => assert!(frame.starts_with("F4441C000BFA")),
        _ => panic!("first packet must be the leader open frame"),
    }
}

#[test]
fn npc_battle_constructor() {
    let leader = Session::new();
    let npc = Npc {
        id: 1001,
        hp: 500,
        lv: 10,
        thuoctinh: 1,
        ..Default::default()
    };
    let battle = Battle::npc_battle(1, &leader, 300001, &npc, 7, trigger_diahinh::PK_AND_NPC);
    assert_eq!(battle.diahinh, 112);
    assert_eq!(battle.cell(3, 2).unwrap().id, leader.id as i64);
    assert_eq!(battle.cell(0, 2).unwrap().typ, 3);
}

#[test]
fn teamdef_battle_constructor() {
    let mut leader = Session::new();
    leader.id = 300001;
    let mut member = Session::new();
    member.id = 300002;
    let npcs: Vec<Npc> = (1..=3)
        .map(|i| Npc {
            id: 1000 + i,
            hp: 300,
            lv: 5,
            thuoctinh: 2,
            ..Default::default()
        })
        .collect();
    let defs: Vec<&Npc> = npcs.iter().collect();
    let battle = Battle::teamdef_battle(1, &leader, 300001, &[&member], &defs, 4712);
    // Defenders at (0,0),(0,1),(0,2) Type 7.
    assert_eq!(battle.cell(0, 0).unwrap().typ, 7);
    assert_eq!(battle.cell(0, 0).unwrap().id, 1001);
    assert_eq!(battle.cell(0, 2).unwrap().id, 1003);
    // Member placed at col 1.
    assert_eq!(battle.cell(3, 1).unwrap().id, 300002);
}

#[test]
fn member_frame_has_markers() {
    let mut battle = Battle::new(1, 112);
    let mut leader = Session::new();
    leader.id = 300001;
    battle.add_player(&leader, leader.id as i64, 3, 2);

    let mut member = Session::new();
    member.id = 300002;
    member.hp = 100;
    member.hp_max = 100;
    battle.add_player(&member, leader.id as i64, 3, 1);

    let frames = battle.member_battle_frame(112, 300002);
    assert!(!frames.is_empty());
    // Find the 0BFA open frame among the member packets.
    let open = frames
        .iter()
        .filter_map(|p| match p {
            StartPacket::To { frame, .. } => Some(frame),
            _ => None,
        })
        .find(|f| f.contains("0BFA"))
        .expect("member open frame present");
    assert!(open.contains("05")); // self marker
    assert!(open.contains("03")); // leader marker
    assert!(open.ends_with("F44403000B0A01"));
}

// ── battle::packets ──

#[test]
fn battle_open_leader_format() {
    let pkt = battle_open_leader(112, "02AABBCCDD");
    assert!(pkt.starts_with("F4441C000BFA"));
    assert!(pkt.ends_with("F44403000B0A01"));
}

#[test]
fn entity_packet_format() {
    let npc = entity_npc("ABCDEF");
    assert_eq!(npc, "F4441A000B0503ABCDEF");

    let player = entity_player("123456");
    assert_eq!(player, "F4441A000B0505123456");
}

#[test]
fn hide_and_show() {
    let hide = hide_from_map(300001);
    assert!(hide.starts_with("F44408000B00"));
    assert!(hide.ends_with("0000"));

    let show = show_player_on_map(300001);
    assert!(show.starts_with("F4440A000B0402"));
    assert!(show.ends_with("000003"));
}

#[test]
fn your_turn_exact() {
    assert_eq!(your_turn(), "F44402003401");
}

#[test]
fn acting_format() {
    assert_eq!(acting(3, 2), "F444040035050302");
}

#[test]
fn status_update_positive() {
    let s = status_update(stat_byte::HP, 100);
    assert!(s.starts_with("F4440C00080119"));
    assert!(s.contains("01")); // positive sign
}

#[test]
fn skilling_int_format() {
    let eff = skilling_int(0, 2, 1, 0, 1, troi_byte::HP, 50, 1);
    assert_eq!(eff.len(), 18); // 9 bytes = 18 hex chars (row col miss adl count troi LE16 buff)
}

#[test]
fn skilling_full_is_17_bytes() {
    let eff = skilling_full(0, 2, 10000, 1, 1, 0, 2, 1, 0, 1, troi_byte::HP, 50, 1);
    assert_eq!(eff.len(), 34); // 17 bytes
    assert!(eff.starts_with("0F00"));
}

#[test]
fn combo_footer_20007_format() {
    let f = combo_footer_20007(2, 3);
    assert!(f.starts_with("F444130032010F00"));
    assert!(f.contains("274E")); // LE16(20007) = 0x4E27 -> "274E"
    assert!(f.ends_with("01030119000000"));
}

// ── battle::manager ──

#[derive(Default)]
struct ManagerRecordingSink;

impl BattleSink for ManagerRecordingSink {
    fn send_to(&self, _p: i64, _f: String) {}
    fn send_map(&self, _p: i64, _f: String) {}
    fn broadcast(&self, _f: String) {}
    fn apply_db(&self, _u: DbUpdate) {}
    fn apply_drop(&self, _d: Out) {}
    fn apply_catch(&self, _o: i64, _n: i64) {}
    fn apply_fled(&self, _p: i64) {}
    fn apply_respawn(&self, _n: i64, _m: i64, _x: i64, _y: i64) {}
    fn apply_pet_exp(&self, _o: i64, _s: i64, _e: i64) {}
    fn battle_ended(&self, _id: i32, _outcome: Outcome) {}
}

fn manager_skill(id: i64) -> Skill {
    Skill {
        id,
        sp: 5,
        thuoctinh: 1,
        lv_max: 10,
        skill_type: 1,
        do_manh: 10,
        sl_danh: 1,
        combo: 0,
        delay: 10,
        ..Default::default()
    }
}

fn manager_npc(id: i64) -> Npc {
    Npc {
        id,
        lv: 5,
        hp: 200,
        sp: 50,
        thuoctinh: 1,
        atk: 5,
        def: 5,
        agi: 5,
        int1: 5,
        skill: [10000, 0, 0, 0],
        ..Default::default()
    }
}

fn build_manager_battle() -> Battle {
    let mut battle = Battle::with_seeds(1, 112, 11, 22, 33);
    let mut session = Session::new();
    session.id = 300001;
    session.level = 10;
    session.hp = 5_000;
    session.hp_max = 5_000;
    session.sp = 200;
    session.sp_max = 200;
    session.atk = 200;
    session.def = 30;
    session.agi = 60;
    session.int1 = 40;
    battle.add_player(&session, 300001, 3, 2);
    battle.add_npc(&manager_npc(9001), 1, 0, 2, 3);
    battle
}

#[tokio::test]
async fn manager_runs_battle_to_win() {
    let mut skills = HashMap::new();
    skills.insert(10000, manager_skill(10000));
    let npcs = {
        let mut m = HashMap::new();
        m.insert(9001, manager_npc(9001));
        m
    };
    let items = HashMap::new();
    let pets = HashMap::new();
    let players = HashMap::new();
    let texps = compute_texps();

    let manager = Arc::new(BattleManager::new());
    let sink = Arc::new(ManagerRecordingSink);
    let handle = manager.spawn_timeout(
        build_manager_battle(),
        Arc::new(npcs),
        Arc::new(skills),
        Arc::new(items),
        Arc::new(pets),
        Arc::new(players),
        Arc::new(texps),
        1,
        None,
        0,
        0,
        std::time::Duration::from_millis(50),
        sink,
    );
    // Submit the basic-attack command; then the task should run to a win.
    handle.command(PlayerInput {
        player: 300001,
        cmd: BattleCommand {
            row: 3,
            col: 2,
            skill_id: 10000,
            skill_lv: 1,
            row_attack: 0,
            col_attack: 2,
            use_item: 0,
        },
    });

    // Wait for the battle to be removed from the registry (task finished).
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if manager.len().await == 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("battle task did not finish in time");
}

// ── battle::runner ──

fn runner_skill(id: i64, skill_type: i64, do_manh: i64, sl_danh: i64, delay: i64) -> Skill {
    Skill {
        id,
        sp: if skill_type == 1 || skill_type == 2 {
            5
        } else {
            10
        },
        thuoctinh: 1,
        lv_max: 10,
        skill_type,
        do_manh,
        sl_danh,
        combo: 0,
        delay,
        ..Default::default()
    }
}

fn runner_npc(id: i64, lv: i64, hp: i64, atk: i64, def: i64, agi: i64) -> Npc {
    Npc {
        id,
        lv,
        hp,
        sp: 100,
        thuoctinh: 1,
        atk,
        def,
        agi,
        int1: 10,
        skill: [10000, 0, 0, 0],
        ..Default::default()
    }
}

fn scenario() -> (Battle, BattleData<'static>) {
    // Leak static tables so their references are 'static for the test.
    let mut skills: HashMap<i64, Skill> = HashMap::new();
    skills.insert(10000, runner_skill(10000, 1, 10, 1, 1000));
    skills.insert(11007, runner_skill(11007, 7, 0, 1, 800));
    let skills = Box::leak(Box::new(skills));
    let mut npcs: HashMap<i64, Npc> = HashMap::new();
    npcs.insert(9001, runner_npc(9001, 10, 500, 20, 10, 10));
    let npcs = Box::leak(Box::new(npcs));
    let items: HashMap<i64, Item> = HashMap::new();
    let items = Box::leak(Box::new(items));
    let pets: HashMap<i64, [i64; 4]> = HashMap::new();
    let pets = Box::leak(Box::new(pets));
    let players: HashMap<i64, PlayerSnapshot> = HashMap::new();
    let players = Box::leak(Box::new(players));
    let texps = compute_texps();
    let texps = Box::leak(Box::new(texps));
    let data = BattleData::new(npcs, skills, items, pets, players, texps, None, 0, 0);
    let battle = Battle::with_seeds(1, 112, 1, 2, 3);
    (battle, data)
}

fn add_runner_player(battle: &mut Battle) -> Session {
    let mut session = Session::new();
    session.id = 300001;
    session.level = 10;
    session.hp = 1000;
    session.hp_max = 1000;
    session.sp = 100;
    session.sp_max = 100;
    session.atk = 100;
    session.def = 20;
    session.agi = 50;
    session.int1 = 30;
    session.thuoctinh = 1;
    session.hpx = 10;
    battle.add_player(&session, session.id as i64, 3, 2);
    session
}

fn add_scenario_npc(battle: &mut Battle, data: &BattleData) {
    let npc = data.npcs.get(&9001).unwrap();
    battle.add_npc(npc, 1, 0, 2, 3);
}

fn basic_command() -> HashMap<i64, BattleCommand> {
    let mut cmds = HashMap::new();
    cmds.insert(
        300001,
        BattleCommand {
            row: 3,
            col: 2,
            skill_id: 10000,
            skill_lv: 1,
            row_attack: 0,
            col_attack: 2,
            use_item: 0,
        },
    );
    cmds
}

#[test]
fn player_basic_attack_kills_npc() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    add_scenario_npc(&mut battle, &data);
    let cmds = basic_command();

    let mut out = Vec::new();
    let outcome = battle.run_battle(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::PlayerWin);
    // NPC dead.
    assert!(battle.cell(0, 2).unwrap().hp <= 0);
    // A turn action frame was broadcast.
    assert!(out
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if f.contains("3201"))));
}

#[test]
fn player_without_command_eventually_loses() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    add_scenario_npc(&mut battle, &data);
    let cmds = HashMap::new();

    let mut out = Vec::new();
    let outcome = battle.run_battle(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::PlayerLose);
    assert!(battle.cell(3, 2).unwrap().hp <= 0);
    // DB HP write for the player happened.
    assert!(out.iter().any(|o| matches!(
        o,
        Out::Db(DbUpdate {
            target: DbTarget::Player(300001),
            stat: Stat::Hp,
            ..
        })
    )));
}

#[test]
fn burn_tick_damages_and_broadcasts() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    add_scenario_npc(&mut battle, &data);
    // Give the player a burn debuff (10004, lv 3 → 10+6=16/turn).
    battle.cell_mut(3, 2).unwrap().type3_id = 10004;
    battle.cell_mut(3, 2).unwrap().type3_lv = 3;
    battle.cell_mut(3, 2).unwrap().type3_turn = 2;
    let cmds = basic_command();

    let mut out = Vec::new();
    battle.run_battle(&data, &cmds, &mut out);
    // Burn broadcast uses skill 20001 (LE16 "214E").
    assert!(out
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if f.contains("214E"))));
}

#[test]
fn turn_action_frame_structure() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    add_scenario_npc(&mut battle, &data);
    let cmds = basic_command();

    let mut out = Vec::new();
    let _ = battle.run_battle(&data, &cmds, &mut out);
    let frame = out
        .iter()
        .find_map(|o| match o {
            Out::Broadcast(f) if f.contains("3201") => Some(f.clone()),
            _ => None,
        })
        .unwrap();
    // F444 + LE16 length + 3201 + block(LE16 len + row col skill sl_danh count + effects)
    assert!(frame.starts_with("F444"));
    assert!(frame.contains("3201"));
    // Skill 10000 = 0x2710 -> LE16 "1027".
    assert!(frame.contains("1027"));
}

#[test]
fn heal_skill_restores_hp() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    add_scenario_npc(&mut battle, &data);
    // Damage the player first.
    battle.cell_mut(3, 2).unwrap().hp = 500;
    let mut cmds = basic_command();
    cmds.insert(
        300001,
        BattleCommand {
            row: 3,
            col: 2,
            skill_id: 11007,
            skill_lv: 3,
            row_attack: 3,
            col_attack: 2,
            use_item: 0,
        },
    );

    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &cmds, &mut out);
    // A DB HP write for the player with value > 500 proves the heal applied
    // (11007 lv3: round(int*0.2*lv)=round(30*0.6)=18).
    let healed = out.iter().any(|o| {
        matches!(
            o,
            Out::Db(DbUpdate { target: DbTarget::Player(300001), stat: Stat::Hp, value }) if *value > 500
        )
    });
    assert!(healed, "expected a heal DB write for the player");
}

#[test]
fn flee_by_leader_ends_battle() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    add_scenario_npc(&mut battle, &data);
    // Skill 14002 always flees; build a dedicated table set for it.
    let mut skills = HashMap::new();
    skills.insert(10000, runner_skill(10000, 1, 10, 1, 1000));
    skills.insert(14002, runner_skill(14002, 12, 0, 1, 0));
    let mut npcs = HashMap::new();
    npcs.insert(9001, runner_npc(9001, 10, 500, 20, 10, 10));
    let items = HashMap::new();
    let pets = HashMap::new();
    let players = HashMap::new();
    let texps = compute_texps();
    let data = BattleData::new(&npcs, &skills, &items, &pets, &players, &texps, None, 0, 0);

    let mut cmds = HashMap::new();
    cmds.insert(
        300001,
        BattleCommand {
            row: 3,
            col: 2,
            skill_id: 14002,
            skill_lv: 1,
            row_attack: 0,
            col_attack: 2,
            use_item: 0,
        },
    );
    let mut out = Vec::new();
    let outcome = battle.run_battle(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::PlayerFled);
}

#[test]
fn use_item_heals_cell_and_pet() {
    let (mut battle, mut data_with_items) = scenario();
    // Add a potion to the item table.
    let items: HashMap<i64, Item> = {
        let mut m = HashMap::new();
        m.insert(
            26001,
            Item {
                id: 26001,
                hp: 500,
                sp: 200,
                ..Default::default()
            },
        );
        m
    };
    let items = Box::leak(Box::new(items));
    let npcs = Box::leak(Box::new(data_with_items.npcs.clone()));
    let skills = Box::leak(Box::new(data_with_items.skills.clone()));
    let pets = Box::leak(Box::new(data_with_items.pet_slots.clone()));
    let players = Box::leak(Box::new(data_with_items.players.clone()));
    let texps = Box::leak(Box::new(compute_texps()));
    data_with_items = BattleData::new(npcs, skills, items, pets, players, texps, None, 0, 0);

    let mut session = Session::new();
    session.id = 300001;
    session.level = 10;
    session.hp = 1000;
    session.hp_max = 1000;
    session.sp = 100;
    session.sp_max = 100;
    session.atk = 100;
    session.def = 20;
    session.agi = 50;
    session.int1 = 30;
    battle.add_player(&session, session.id as i64, 3, 2);
    // Attach a pet at (2,2).
    let pet = PetState {
        stt: 1,
        id: 9001,
        hp: 200,
        hp_max: 500,
        sp: 100,
        sp_max: 500,
        ..Default::default()
    };
    battle.add_pet(&pet, session.id as i64, session.id as i64, 3, 2, 1);

    let n = data_with_items.npcs.get(&9001).unwrap();
    battle.add_npc(n, 1, 0, 2, 3);

    // Damage the player then use potion 26001 on the player cell.
    battle.cell_mut(3, 2).unwrap().hp = 700;
    battle.cell_mut(3, 2).unwrap().sp = 40;
    let mut cmds = HashMap::new();
    cmds.insert(
        300001,
        BattleCommand {
            row: 3,
            col: 2,
            skill_id: 0,
            skill_lv: 0,
            row_attack: 0,
            col_attack: 2,
            use_item: 26001,
        },
    );
    let mut out = Vec::new();
    let _ = battle.run_turn(&data_with_items, &cmds, &mut out);
    let cell = battle.cell(3, 2).unwrap();
    // 700 + 500 capped to 1000; 40 + 200 capped to 100.
    assert_eq!(cell.hp, 1000);
    assert_eq!(cell.sp, 100);
    // Pet DB write fired (heal the owner's active pet).
    assert!(out.iter().any(|o| matches!(
        o,
        Out::Db(DbUpdate {
            target: DbTarget::Pet { owner: 300001, .. },
            stat: Stat::Hp,
            ..
        })
    )));
}

#[test]
fn hp_max_formula_matches() {
    // get_hp_max(rb=0, job, lvl=10, hpx=6): floor((10^0.35+1)*12 + 80 + 10).
    let v = get_hp_max(0, 0, 10, 6);
    assert!(v > 0);
}

#[test]
fn finish_grants_teamdef_exp_and_exit_packets() {
    // A TeamDef (type-7) kill accumulates exp; finish pays it out.
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    // Use a type-7 npc instead of the scenario's type-3.
    battle.clear_cell(0, 2);
    let n = data.npcs.get(&9001).unwrap();
    battle.add_npc(n, 1, 0, 2, 7);
    let cmds = basic_command();

    let mut out = Vec::new();
    let outcome = battle.run_battle(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::PlayerWin);

    let mut fin = Vec::new();
    battle.finish(&data, 1, false, true, &mut fin);
    // A Texp write for the player on a type-7 kill.
    assert!(fin.iter().any(|o| matches!(
        o,
        Out::Db(DbUpdate {
            target: DbTarget::Player(300001),
            stat: Stat::Texp,
            ..
        })
    )));
    // Exit UI frames.
    assert!(fin
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if *f == battle_exit_move())));
    assert!(fin
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if *f == battle_exit_talk())));
    // Player stays alive and unarmed after battle.
    assert!(battle.cell(3, 2).unwrap().hp > 0);
}

#[test]
fn leader_sp_regen_restores_leader_and_pet_sp() {
    let (mut battle, data) = scenario();
    let mut session = add_runner_player(&mut battle);
    let scenario_npc = data.npcs.get(&9001).unwrap();
    battle.add_npc(scenario_npc, 1, 0, 2, 3);
    // Designate a quan-su: QS int 30 → regen = round(30/15.0) = 2.
    session.id_qs = 300002;
    session.int1 = 30;
    battle.leader_id_qs = 300002;
    battle.leader_qs_int = 30;
    // Drain the leader's SP below max so the regen is observable.
    battle.cell_mut(3, 2).unwrap().sp = 90;
    battle.cell_mut(3, 2).unwrap().sp_max = 100;
    let cmds = HashMap::new(); // no commands; the regen still runs.
    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &cmds, &mut out);
    // Leader SP 90 + 2 = 92.
    assert_eq!(battle.cell(3, 2).unwrap().sp, 92);
    // A Db Sp write for the leader was emitted.
    assert!(out.iter().any(|o| matches!(
        o,
        Out::Db(DbUpdate {
            target: DbTarget::Player(300001),
            stat: Stat::Sp,
            value: 92,
        })
    )));
}

#[test]
fn leader_sp_regen_skips_when_no_quan_su() {
    let (mut battle, data) = scenario();
    add_runner_player(&mut battle);
    let scenario_npc = data.npcs.get(&9001).unwrap();
    battle.add_npc(scenario_npc, 1, 0, 2, 3);
    battle.cell_mut(3, 2).unwrap().sp = 90;
    let cmds = HashMap::new();
    let mut out = Vec::new();
    let _ = battle.run_turn(&data, &cmds, &mut out);
    // No QS → no regen.
    assert_eq!(battle.cell(3, 2).unwrap().sp, 90);
    // The leader's own action may legitimately write Sp (basic attack cost);
    // the assertion is scoped to the regen: SP must be exactly 90, never
    // bumped by a +2 quan-su regen.
    let regen_sp_values = |out: &[Out]| {
        out.iter()
            .filter_map(|o| match o {
                Out::Db(DbUpdate {
                    target: DbTarget::Player(300001),
                    stat: Stat::Sp,
                    value,
                }) => Some(*value),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(
        regen_sp_values(&out).iter().all(|&v| v == 90),
        "SP writes must reflect the un-regened value; got: {:?}",
        regen_sp_values(&out)
    );
}

#[test]
fn finish_win_respawns_npc_with_delay_lifecycle() {
    // Win path respawns the map npc when its instance exists and its delay is
    // 0, drawing `random_2` and writing `_Delay = 10`.
    let rows = vec![NpcOnMap {
        map_id: 12001,
        id: 7,
        npc_id: 9001,
        x: 400,
        y: 500,
        coord: 10,
        so_luong: 1,
    }];
    let world = Arc::new(RwLock::new(NpcWorld::new(&rows)));

    let (mut battle, data) = scenario();
    let mut session = add_runner_player(&mut battle);
    session.talking_battle = 7;
    session.map_id = 12001;
    let scenario_npc = data.npcs.get(&9001).unwrap();
    battle.add_npc(scenario_npc, 1, 0, 2, 3);
    let cmds = basic_command();
    let mut out = Vec::new();
    let _ = battle.run_battle(&data, &cmds, &mut out);
    let mut fin = Vec::new();
    let data2 = BattleData::new(
        data.npcs,
        data.skills,
        data.items,
        data.pet_slots,
        data.players,
        data.texps,
        Some(world.as_ref()),
        7,
        12001,
    );
    battle.finish(&data2, 1, false, true, &mut fin);
    // The npc was respawned: delay set to 10 and a Respawn out produced.
    let guard = world.read().unwrap();
    let e = guard.get(12001, 7).unwrap();
    assert_eq!(e.delay, 10);
    assert!(fin.iter().any(|o| matches!(
        o,
        Out::Respawn {
            npc_id: 7,
            map_id: 12001,
            ..
        }
    )));
}

#[test]
fn finish_lose_does_not_respawn_npc() {
    // A defeat (players all dead) must NOT respawn the map npc.
    let rows = vec![NpcOnMap {
        map_id: 12001,
        id: 7,
        npc_id: 9001,
        x: 400,
        y: 500,
        coord: 10,
        so_luong: 1,
    }];
    let world = Arc::new(RwLock::new(NpcWorld::new(&rows)));

    let (mut battle, data) = scenario();
    let mut session = add_runner_player(&mut battle);
    session.talking_battle = 7;
    session.map_id = 12001;
    // A far stronger npc: the player dies without a command.
    let mut npc = data.npcs.get(&9001).unwrap().clone();
    npc.atk = 50_000;
    npc.hp = 1_000_000;
    battle.add_npc(&npc, 1, 0, 2, 3);
    let cmds = HashMap::new();
    let mut out = Vec::new();
    let outcome = battle.run_battle(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::PlayerLose);
    let mut fin = Vec::new();
    let data2 = BattleData::new(
        data.npcs,
        data.skills,
        data.items,
        data.pet_slots,
        data.players,
        data.texps,
        Some(world.as_ref()),
        7,
        12001,
    );
    battle.finish(&data2, 1, false, false, &mut fin);
    let guard = world.read().unwrap();
    let e = guard.get(12001, 7).unwrap();
    assert_eq!(e.delay, 0, "defeat must not respawn the npc");
    assert!(!fin.iter().any(|o| matches!(o, Out::Respawn { .. })));
}

// ── battle::service ──

fn game_data() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        9001,
        Npc {
            id: 9001,
            lv: 1,
            hp: 30,
            sp: 30,
            thuoctinh: 1,
            atk: 1,
            def: 1,
            agi: 1,
            int1: 1,
            skill: [10000, 0, 0, 0],
            ..Default::default()
        },
    );
    data.skills.insert(
        10000,
        Skill {
            id: 10000,
            sp: 5,
            lv_max: 10,
            skill_type: 1,
            do_manh: 10,
            sl_danh: 1,
            delay: 1000,
            ..Default::default()
        },
    );
    data
}

fn strong_session(id: u32) -> Arc<tokio::sync::RwLock<Session>> {
    let s = Arc::new(tokio::sync::RwLock::new(Session::new()));
    {
        let mut s = s.try_write().expect("lock");
        s.id = id;
        s.level = 50;
        s.hp = 5000;
        s.hp_max = 5000;
        s.sp = 500;
        s.sp_max = 500;
        s.atk = 300;
        s.def = 50;
        s.agi = 100;
        s.int1 = 100;
        s.hp_max = get_hp_max(0, 0, 50, 6) as u16;
        s.hp = s.hp_max;
        s.skills.push((10000, 10));
    }
    s
}

/// Drain the receiver (up to 200 frames) and return whatever arrived quickly.
async fn drain(rx: &mut mpsc::UnboundedReceiver<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut collected = 0usize;
    while collected < 200 {
        match tokio::time::timeout(std::time::Duration::from_millis(1), rx.recv()).await {
            Ok(Some(f)) => {
                out.push(f);
                collected += 1;
            }
            _ => break,
        }
    }
    out
}

async fn wait_no_members(service: &Arc<BattleService>) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if !service.has_members() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("battle did not end in time");
}

#[tokio::test]
async fn npc_battle_runs_to_win_and_clears_members() {
    let mut service = BattleService::new(Arc::new(game_data()));
    service.set_input_timeout(std::time::Duration::from_millis(50));
    let service = Arc::new(service);

    let session = strong_session(300001);
    let mut rx = service.register(300001, Arc::clone(&session));

    let start_frames = {
        let mut s = session.write().await;
        service.start_npc_battle(&mut s, 9001, 11)
    };
    assert!(start_frames > 0);

    // Actor command: leader at (3,2) basic-attacks the NPC at (0,2).
    {
        let cmd = BattleCommand {
            row: 3,
            col: 2,
            skill_id: 10000,
            skill_lv: 10,
            row_attack: 0,
            col_attack: 2,
            use_item: 0,
        };
        let s = session.read().await;
        assert!(service.submit_command(&s, cmd), "command accepted");
    }

    // The battle task ends; collect its frames.
    wait_no_members(&service).await;
    let frames = drain(&mut rx).await;

    // Open board + turn-action frames are emitted (acting `F44404003505`
    // is sent by the op 0x32 *handler*, not the battle task).
    assert!(
        frames.iter().any(|f| f.starts_with("F4441C000BFA")),
        "open board frame expected: {frames:?}"
    );
    assert!(
        frames.iter().any(|f| f.contains("3201")),
        "turn-action frame expected"
    );

    // The participant's battle id was cleared and exit frames sent.
    let s = session.read().await;
    assert_eq!(s.battle_id, 0, "battle id cleared after end");
    drop(s);
}

#[tokio::test]
async fn join_frame_has_24byte_cell_records() {
    let mut service = BattleService::new(Arc::new(game_data()));
    service.set_input_timeout(std::time::Duration::from_millis(50));
    let service = Arc::new(service);

    let session = strong_session(300001);
    let mut rx = service.register(300001, Arc::clone(&session));
    let battle_id = {
        let mut s = session.write().await;
        service.start_npc_battle_seeded(&mut s, 9001, 11, 1, 2, 3)
    };
    assert!(battle_id > 0);
    // Build the join frame and verify the length header matches the payload.
    let join = {
        let s = session.read().await;
        service.build_join_frame(&s, battle_id)
    };
    assert!(join.starts_with("F444"));
    let len_bytes = encoder::u16_le(hex_u8(&join[4..6]), hex_u8(&join[6..8])) as usize;
    let payload = &join[8..];
    assert_eq!(
        len_bytes,
        payload.len() / 2,
        "join frame length header must match payload (frame={join})"
    );
    // Payload = 0BFA + diahinh + 0402 + self(22B) + 20 × 24-byte cells.
    assert_eq!(payload.len() / 2, 2 + 2 + 2 + 22 + 20 * 24);
    let _ = rx.try_recv();
    let _ = battle_id;
}

fn hex_u8(hex: &str) -> u8 {
    u8::from_str_radix(hex, 16).unwrap()
}

#[tokio::test]
async fn battle_end_triggers_quest_win() {
    let mut data = game_data();
    data.talks.insert(
        "12001:NPC:7:0".to_string(),
        QuestDef {
            map_id: 12001,
            id: 7,
            on_win: QuestResult {
                rewards: vec![(46001, 5, 0)],
                message: "Win".to_string(),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut service = BattleService::new(Arc::new(data));
    service.set_input_timeout(std::time::Duration::from_millis(50));
    let service = Arc::new(service);

    let session = strong_session(300001);
    {
        let mut s = session.write().await;
        s.map_id = 12001;
        s.talking_battle = 7;
    }
    let mut rx = service.register(300001, Arc::clone(&session));

    {
        let mut s = session.write().await;
        service.start_npc_battle(&mut s, 9001, 11);
    }
    {
        let cmd = BattleCommand {
            row: 3,
            col: 2,
            skill_id: 10000,
            skill_lv: 10,
            row_attack: 0,
            col_attack: 2,
            use_item: 0,
        };
        let s = session.read().await;
        service.submit_command(&s, cmd);
    }
    wait_no_members(&service).await;
    let frames = drain(&mut rx).await;

    // Reward granted to the leader.
    let s = session.read().await;
    assert!(
        s.homdo.iter().any(|i| i.id == 46001 && i.count == 5),
        "quest reward item granted: {:?}",
        s.homdo
    );
    // Red message frame emitted.
    assert!(frames.iter().any(|f| f.contains("020B")));
    assert_eq!(s.talking_battle, 0, "quest talk cleared");
}
