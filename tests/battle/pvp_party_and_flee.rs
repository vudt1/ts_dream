//! Ticket 27 — PvP Party symmetry (`0x0B 02 02`), PK guards, and Flee formula
//! (`14002` +20%, NPC guard, fail emits `18002`).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use ts_dream::battle::construction::{Battle, BattleKind};
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, DbTarget, Out, Outcome,
    PlayerSnapshot, Stat,
};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, Skill};
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::battle;
use ts_dream::server::session::{lock_online_sessions, Conn, PetState, Session};

fn make_flee_skills() -> HashMap<i64, Skill> {
    let mut skills = HashMap::new();
    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            delay: 5,
            ..Default::default()
        },
    );
    skills.insert(
        14002,
        Skill {
            id: 14002,
            name: "Thuat Dao Tau".to_string(),
            sl_danh: 1,
            skill_type: 12,
            sp: 10,
            delay: 5,
            ..Default::default()
        },
    );
    skills.insert(
        18001,
        Skill {
            id: 18001,
            name: "Chay Tron".to_string(),
            sl_danh: 1,
            skill_type: 12,
            delay: 5,
            ..Default::default()
        },
    );
    skills.insert(
        18002,
        Skill {
            id: 18002,
            name: "Chay Tron That Bai".to_string(),
            sl_danh: 1,
            skill_type: 12,
            delay: 5,
            ..Default::default()
        },
    );
    skills
}

fn run_single_flee_turn(
    seed0: i32,
    attacker_typ: u8,
    attacker_lv: i64,
    enemy_lv: i64,
    skill_id: i64,
) -> (Outcome, Vec<Out>) {
    let skills = make_flee_skills();
    let binary_skills = HashMap::new();
    let npcs = HashMap::new();
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 0,
    };

    let mut battle = Battle::with_seeds(1, 112, seed0, 1, 1);
    {
        let a = battle.cell_mut(3, 2).unwrap();
        a.typ = attacker_typ;
        a.id = 100;
        a.leader_id = 100;
        a.hp = 500;
        a.hp_max = 500;
        a.sp = 100;
        a.sp_max = 100;
        a.lv = attacker_lv;
        a.agi = 500; // ensure attacker acts first
        a.team = 1;
    }
    {
        let e = battle.cell_mut(0, 2).unwrap();
        e.typ = 3;
        e.id = 1001;
        e.hp = 500;
        e.hp_max = 500;
        e.lv = enemy_lv;
        e.agi = 1;
        e.team = 2;
    }

    let mut cmds = BattleCommands::new();
    cmds.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: if skill_id == 18001 {
                BattleAction::Flee
            } else {
                BattleAction::CastSkill {
                    skill_id,
                    skill_lv: 1,
                }
            },
        },
    );
    cmds.insert(
        (0, 2),
        BattleCommand {
            row: 0,
            col: 2,
            target_row: 3,
            target_col: 2,
            action: BattleAction::Defend,
        },
    );

    let mut out = Vec::new();
    let outcome = battle.run_turn(&data, &cmds, &mut out);
    (outcome, out)
}

#[test]
fn flee_14002_adds_20_percent_and_does_not_auto_win_at_large_disadvantage() {
    // Attacker lv 10 vs enemy lv 60:
    // lv1 - lv2 = -50, lvtb1 - lvtb2 = -50
    // 60 - 50 - 50 + 20 = -20 -> clamped to 0% (must NEVER auto-win!)
    for seed in 1..=20 {
        let (outcome, out) = run_single_flee_turn(seed, 2, 10, 60, 14002);
        assert_eq!(
            outcome,
            Outcome::Running,
            "14002 must not auto-win at 0% clamped chance (seed={seed})"
        );
        let broadcasts: String = out
            .iter()
            .filter_map(|o| match o {
                Out::Broadcast(f) => Some(f.as_str()),
                _ => None,
            })
            .collect();
        // 18002 LE16 = 5246, 14003 LE16 = B336
        assert!(
            broadcasts.contains("32010F0003025246010C"),
            "failed 14002 flee must emit skill_id=18002 (5246) with skill_type=12 (0C), got: {broadcasts}"
        );
        assert!(
            !broadcasts.contains("B336"),
            "failed 14002 flee must never emit 14003 (B336), got: {broadcasts}"
        );
    }

    // Check +20% bonus: lv1=50, lv2=75 -> lv1-lv2=-25, lvtb1-lvtb2=-25.
    // 18001: 60 - 25 - 25 = 10%
    // 14002: 60 - 25 - 25 + 20 = 30%
    // Across 200 seeds, 14002 must succeed strictly more often than 18001.
    let mut wins_18001 = 0;
    let mut wins_14002 = 0;
    for seed in 1..=200 {
        if run_single_flee_turn(seed, 2, 50, 75, 18001).0 == Outcome::PlayerFled {
            wins_18001 += 1;
        }
        if run_single_flee_turn(seed, 2, 50, 75, 14002).0 == Outcome::PlayerFled {
            wins_14002 += 1;
        }
    }
    assert!(
        wins_14002 > wins_18001,
        "14002 (+20% bonus) should succeed more often than 18001 (14002={wins_14002}, 18001={wins_18001})"
    );
}

#[test]
fn flee_18001_failure_emits_18002_with_skill_type_12_miss_0_adl_2() {
    // 60 - 50 - 50 = -40 -> clamped to 0% -> guaranteed fail
    let (outcome, out) = run_single_flee_turn(42, 2, 10, 60, 18001);
    assert_eq!(outcome, Outcome::Running);
    let broadcasts: String = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.as_str()),
            _ => None,
        })
        .collect();
    // F444 1300 3201 0F00 03 02 5246 01 0C 00 02 00 02 ...
    assert!(
        broadcasts.contains("32010F0003025246010C00020002"),
        "failed 18001 flee must emit 18002 (5246) skill_type=12 (0C) miss=0 (00) adl=2 (02), got: {broadcasts}"
    );
}

#[test]
fn flee_npc_entity_is_blocked_from_player_flee_branch() {
    // Player (typ=2, lv=1) on team 1 defends; enemy NPC (typ=3, lv=200) on team 2
    // attempts BattleAction::Flee. NPC must NEVER trigger Outcome::PlayerFled or emit 18002.
    let skills = make_flee_skills();
    let binary_skills = HashMap::new();
    let npcs = HashMap::new();
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 0,
    };

    let mut battle = Battle::with_seeds(1, 112, 1, 1, 1);
    {
        let p = battle.cell_mut(3, 2).unwrap();
        p.typ = 2;
        p.id = 100;
        p.leader_id = 100;
        p.hp = 500;
        p.hp_max = 500;
        p.lv = 1;
        p.agi = 1;
        p.team = 1;
    }
    {
        let npc = battle.cell_mut(0, 2).unwrap();
        npc.typ = 3;
        npc.id = 1001;
        npc.hp = 500;
        npc.hp_max = 500;
        npc.lv = 200;
        npc.agi = 500;
        npc.team = 2;
    }

    let mut cmds = BattleCommands::new();
    cmds.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::Defend,
        },
    );
    cmds.insert(
        (0, 2),
        BattleCommand {
            row: 0,
            col: 2,
            target_row: 3,
            target_col: 2,
            action: BattleAction::Flee,
        },
    );

    let mut out = Vec::new();
    let outcome = battle.run_turn(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::Running);
    let broadcasts: String = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        !broadcasts.contains("5246"),
        "NPC entity must be blocked from player flee branch, got: {broadcasts}"
    );
}

#[test]
fn pvp_finish_cleans_up_both_team_1_and_team_2_without_rewards_or_god() {
    let skills = make_flee_skills();
    let binary_skills = HashMap::new();
    let npcs = HashMap::new();
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let mut players = HashMap::new();
    players.insert(
        100,
        PlayerSnapshot {
            god: 3,
            ..Default::default()
        },
    );
    players.insert(
        200,
        PlayerSnapshot {
            god: 5,
            ..Default::default()
        },
    );
    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 12001,
    };

    let mut battle = Battle::with_seeds(1, 112, 1, 1, 1).with_kind(BattleKind::Pk);
    // Team 1 leader at (3, 2) (alive, with exp accumulated)
    {
        let p1 = battle.cell_mut(3, 2).unwrap();
        p1.typ = 2;
        p1.id = 100;
        p1.leader_id = 100;
        p1.hp = 200;
        p1.hp_max = 200;
        p1.lv = 20;
        p1.exp = 500;
        p1.team = 1;
    }
    // Team 2 leader at (0, 2) (defeated: hp = 0)
    {
        let p2 = battle.cell_mut(0, 2).unwrap();
        p2.typ = 2;
        p2.id = 200;
        p2.leader_id = 200;
        p2.hp = 0;
        p2.hp_max = 200;
        p2.lv = 20;
        p2.team = 2;
    }

    let mut out = Vec::new();
    battle.finish(&data, 1, false, true, &mut out);

    // Team 2 defeated player must be restored to Hp = 1
    let p2_hp_restore = out.iter().any(|o| {
        matches!(
            o,
            Out::Db(u) if u.target == DbTarget::Player(200) && u.stat == Stat::Hp && u.value == 1
        )
    });
    assert!(p2_hp_restore, "Team 2 defeated player must be restored to Hp = 1");

    // No EXP or God deduction for either team
    assert!(
        !out.iter().any(|o| matches!(
            o,
            Out::Db(u) if matches!(u.stat, Stat::Texp | Stat::God)
        )),
        "PK battle finish must not grant Texp or deduct God"
    );

    // MapBroadcast hide_from_map for BOTH players 100 and 200
    let map_hides: Vec<i64> = out
        .iter()
        .filter_map(|o| match o {
            Out::MapBroadcast { player, .. } => Some(*player),
            _ => None,
        })
        .collect();
    assert!(map_hides.contains(&100), "Team 1 player 100 must get MapBroadcast hide");
    assert!(map_hides.contains(&200), "Team 2 player 200 must get MapBroadcast hide");

    // Broadcast contains 0B00 (hide), 0B01 (reset_cell), 0504 (exit move), 1408 (exit talk)
    let broadcasts: String = out
        .iter()
        .filter_map(|o| match o {
            Out::Broadcast(f) => Some(f.as_str()),
            _ => None,
        })
        .collect();
    // 200 LE32 = C8000000
    assert!(
        broadcasts.contains("0B00C8000000"),
        "Broadcast must include hide_from_map (0B00) for Team 2 player 200, got: {broadcasts}"
    );
    assert!(
        broadcasts.contains("0B010002") && broadcasts.contains("0B010102"),
        "Broadcast must include reset_cell (0B01) for Team 2 player (0,2) and pet (1,2), got: {broadcasts}"
    );
    assert!(broadcasts.contains("0504"), "Broadcast must include 0504");
    assert!(broadcasts.contains("1408"), "Broadcast must include 1408");
}

fn pk_test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a BattleService,
    out: &'a mut HandleOutcome,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        opcode: 0x0B,
        sub: 2,
        payload,
        decoded: payload,
        data,
        env: ServerEnv::none(),
        service,
        out,
    }
}

fn pk_challenge_payload(target_id: u32) -> Vec<u8> {
    let mut p = vec![0x02];
    p.extend_from_slice(&target_id.to_le_bytes());
    p.extend_from_slice(&112u16.to_le_bytes());
    p
}

#[tokio::test]
async fn pk_challenge_0b_02_02_guards_reject_invalid_targets_and_emit_210101_when_pk_off() {
    let mut data = GameData::default();
    data.skills = make_flee_skills();
    let service = BattleService::new(Arc::new(data.clone()));

    let mut challenger_conn = Conn::default();
    challenger_conn.session.id = 100;
    challenger_conn.session.pk = 1;
    challenger_conn.session.map_id = 12001;
    challenger_conn.session.hp = 500;
    challenger_conn.session.hp_max = 500;
    let challenger_arc = Arc::new(AsyncRwLock::new(challenger_conn.session.clone()));
    let _rx_c = service.register(100, challenger_arc);

    // 1. Self-PK (target == 100) -> blocked
    {
        let payload = pk_challenge_payload(100);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "self-PK must be blocked");
    }

    // Helper to register target session
    let target_sess = Arc::new(AsyncRwLock::new(Session {
        id: 200,
        pk: 1,
        map_id: 12001,
        hp: 500,
        hp_max: 500,
        ..Default::default()
    }));
    let _rx_t = service.register(200, target_sess.clone());

    // 2. Different map -> blocked + sys_msg_frame
    {
        target_sess.write().await.map_id = 12002;
        let payload = pk_challenge_payload(200);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "different map PK must be blocked");
        assert!(out.outgoing.iter().any(|f| f.contains("020C")), "must emit sys_msg_frame");
        target_sess.write().await.map_id = 12001;
    }

    // 3. Same party (id_leader) -> blocked + sys_msg_frame
    {
        challenger_conn.session.id_leader = 100;
        target_sess.write().await.id_leader = 100;
        let payload = pk_challenge_payload(200);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "same party PK must be blocked");
        assert!(out.outgoing.iter().any(|f| f.contains("020C")), "must emit sys_msg_frame");
        challenger_conn.session.id_leader = 0;
        target_sess.write().await.id_leader = 0;
    }

    // 4. Target is GM/Angel -> blocked + sys_msg_frame
    {
        target_sess.write().await.gm_level = 10;
        let payload = pk_challenge_payload(200);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "GM target PK must be blocked");
        assert!(out.outgoing.iter().any(|f| f.contains("020C")), "must emit sys_msg_frame");
        target_sess.write().await.gm_level = 0;
    }

    // 5. Target has active player shop (Sạp hàng) -> blocked + sys_msg_frame
    {
        target_sess.write().await.shop.active = true;
        let payload = pk_challenge_payload(200);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "active shop target PK must be blocked");
        assert!(out.outgoing.iter().any(|f| f.contains("020C")), "must emit sys_msg_frame");
        target_sess.write().await.shop.active = false;
    }

    // 6. Target is ListQS spectator -> blocked
    {
        // Start an unrelated battle 999 and join target 200 as spectator
        let npc_sess = Arc::new(AsyncRwLock::new(Session {
            id: 300,
            pk: 1,
            map_id: 12001,
            hp: 500,
            hp_max: 500,
            ..Default::default()
        }));
        let _rx_n = service.register(300, npc_sess.clone());
        // Join via an existing battle: start a quick battle for 300 vs 400
        let npc_sess2 = Arc::new(AsyncRwLock::new(Session {
            id: 400,
            pk: 1,
            map_id: 12001,
            hp: 500,
            hp_max: 500,
            ..Default::default()
        }));
        let _rx_n2 = service.register(400, npc_sess2);
        let mut s300 = npc_sess.write().await;
        let bid = service.start_pk_battle(&mut s300, 400, 112);
        drop(s300);
        assert!(bid > 0);
        {
            let mut s200 = target_sess.write().await;
            assert!(service.join_battle(&mut s200, bid));
        }
        let payload = pk_challenge_payload(200);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "ListQS spectator target PK must be blocked");
        {
            let mut s200 = target_sess.write().await;
            service.leave_spectate(&mut s200);
        }
    }

    // 7. Target has Option PK off (pk == 0) -> blocked + emits 21 01 01
    {
        target_sess.write().await.pk = 0;
        let payload = pk_challenge_payload(200);
        let mut out = HandleOutcome::default();
        let mut ctx = pk_test_ctx(&mut challenger_conn, &data, &service, &mut out, &payload);
        battle::handle_battle(&mut ctx);
        assert_eq!(challenger_conn.session.battle_id, 0, "pk=0 target must be blocked");
        assert!(
            out.outgoing.iter().any(|f| f == "F4440300210101"),
            "pk=0 target must emit F4440300210101, got: {:?}",
            out.outgoing
        );
    }
}

fn make_pvp_session(id: u32, id_leader: u32, id_mem: [u32; 4], pet_id: u16) -> Session {
    let mut s = Session {
        id,
        name: format!("P{id}").into_bytes(),
        level: 30,
        hp: 500,
        hp_max: 500,
        sp: 200,
        sp_max: 200,
        atk: 50,
        def: 20,
        agi: 20,
        int1: 20,
        map_id: 12001,
        pk: 1,
        id_leader,
        id_mem,
        active_pet_stt: 1,
        skills: vec![(10001, 1)],
        ..Default::default()
    };
    s.pets.push(PetState {
        stt: 1,
        id: pet_id,
        name: format!("Pet{pet_id}").into_bytes(),
        level: 30,
        hp: 300,
        hp_max: 300,
        sp: 100,
        sp_max: 100,
        ..Default::default()
    });
    s
}

#[tokio::test]
async fn pvp_party_0b_02_02_spawns_both_sides_symmetrically_with_03_05_markers_and_0b_05_05() {
    let mut data = GameData::default();
    data.skills = make_flee_skills();
    data.skills.insert(
        10001,
        Skill {
            id: 10001,
            name: "Hoa Cau".to_string(),
            sl_danh: 1,
            skill_type: 2,
            sp: 5,
            delay: 5,
            ..Default::default()
        },
    );
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"Pet1".to_vec(),
            lv: 30,
            hp: 300,
            sp: 100,
            ..Default::default()
        },
    );
    data.npcs.insert(
        2001,
        Npc {
            id: 2001,
            name: b"Pet2".to_vec(),
            lv: 30,
            hp: 300,
            sp: 100,
            ..Default::default()
        },
    );

    let service = BattleService::new(Arc::new(data.clone()));

    let s100 = make_pvp_session(100, 100, [101, 0, 0, 0], 1001);
    let s101 = make_pvp_session(101, 100, [0; 4], 1001);
    let s200 = make_pvp_session(200, 200, [201, 0, 0, 0], 2001);
    let s201 = make_pvp_session(201, 200, [0; 4], 2001);

    let arc100 = Arc::new(AsyncRwLock::new(s100.clone()));
    let arc101 = Arc::new(AsyncRwLock::new(s101.clone()));
    let arc200 = Arc::new(AsyncRwLock::new(s200.clone()));
    let arc201 = Arc::new(AsyncRwLock::new(s201.clone()));

    let mut rx100 = service.register(100, arc100.clone());
    let mut rx101 = service.register(101, arc101.clone());
    let mut rx200 = service.register(200, arc200.clone());
    let mut rx201 = service.register(201, arc201.clone());

    {
        let mut online = lock_online_sessions();
        online.insert(100, s100.clone());
        online.insert(101, s101.clone());
        online.insert(200, s200.clone());
        online.insert(201, s201.clone());
    }

    let mut conn = Conn {
        session: s100.clone(),
        ..Default::default()
    };
    let payload = pk_challenge_payload(200);
    let mut out = HandleOutcome::default();
    let mut ctx = pk_test_ctx(&mut conn, &data, &service, &mut out, &payload);
    battle::handle_battle(&mut ctx);

    let bid = conn.session.battle_id;
    assert!(bid > 0, "PvP party battle must start");
    assert_eq!(arc101.read().await.battle_id, bid, "Team 1 member 101 must get battle_id");
    assert_eq!(arc200.read().await.battle_id, bid, "Team 2 leader 200 must get battle_id");
    assert_eq!(arc201.read().await.battle_id, bid, "Team 2 member 201 must get battle_id");

    let drain = |rx: &mut tokio::sync::mpsc::Receiver<String>| -> String {
        let mut acc = String::new();
        while let Ok(f) = rx.try_recv() {
            acc.push_str(&f);
        }
        acc
    };
    let f100 = drain(&mut rx100);
    let f101 = drain(&mut rx101);
    let f200 = drain(&mut rx200);
    let f201 = drain(&mut rx201);

    // 0BFA self markers: 03 for leaders (100, 200), 05 for members (101, 201)
    assert!(f100.contains("0BFA700003"), "Leader 100 must receive 0BFA with 03 marker, got: {f100}");
    assert!(f101.contains("0BFA700005"), "Member 101 must receive 0BFA with 05 marker, got: {f101}");
    assert!(f200.contains("0BFA700003"), "Leader 200 must receive 0BFA with 03 marker, got: {f200}");
    assert!(f201.contains("0BFA700005"), "Member 201 must receive 0BFA with 05 marker, got: {f201}");

    // All 4 players must receive 0B0505 (entity_player) and NEVER 0B0503 (entity_npc)
    for (who, frames) in [("100", &f100), ("101", &f101), ("200", &f200), ("201", &f201)] {
        assert!(frames.contains("0B0505"), "{who} must receive 0B0505 player entity frames");
        assert!(!frames.contains("0B0503"), "{who} must NOT receive 0B0503 NPC entity frames");
    }

    // All 4 players can submit 0x32 commands (both Team 1 and Team 2!)
    let cmd_100 = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::CastSkill { skill_id: 10001, skill_lv: 1 },
    };
    let cmd_101 = BattleCommand {
        row: 3,
        col: 1,
        target_row: 0,
        target_col: 1,
        action: BattleAction::CastSkill { skill_id: 10001, skill_lv: 1 },
    };
    let cmd_200 = BattleCommand {
        row: 0,
        col: 2,
        target_row: 3,
        target_col: 2,
        action: BattleAction::CastSkill { skill_id: 10001, skill_lv: 1 },
    };
    let cmd_201 = BattleCommand {
        row: 0,
        col: 1,
        target_row: 3,
        target_col: 1,
        action: BattleAction::CastSkill { skill_id: 10001, skill_lv: 1 },
    };
    assert!(service.submit_command(&conn.session, cmd_100), "Team 1 leader 100 command must succeed");
    assert!(service.submit_command(&*arc101.read().await, cmd_101), "Team 1 member 101 command must succeed");
    assert!(service.submit_command(&*arc200.read().await, cmd_200), "Team 2 leader 200 command must succeed");
    assert!(service.submit_command(&*arc201.read().await, cmd_201), "Team 2 member 201 command must succeed");

    {
        let mut online = lock_online_sessions();
        for id in [100u32, 101, 200, 201] {
            online.remove(&id);
        }
    }
}
