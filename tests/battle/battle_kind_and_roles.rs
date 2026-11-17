//! Ticket 26 — `BattleKind` (`Normal` vs `Pk`), `0x0B 02 03` map-NPC PK,
//! and `0B FA` (`0x03`/`0x05`/`0x04`) + ListQS-only `0x0B 01 03`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::RwLock as AsyncRwLock;

use ts_dream::battle::construction::{Battle, BattleKind};
use ts_dream::battle::npc_world::NpcWorld;
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, DbTarget, Out, Outcome,
    PlayerSnapshot, Stat,
};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::{EveFightData, EveFightEnemy};
use ts_dream::data::tables::{Npc, NpcOnMap, Skill, TexpRow};
use ts_dream::server::dispatcher::{dispatch, HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::battle;
use ts_dream::server::handlers::quest::BattleTrigger;
use ts_dream::server::session::{lock_online_sessions, Conn, PetState, Session};

fn make_texps() -> Vec<TexpRow> {
    (0..=50)
        .map(|lv| TexpRow {
            lv,
            reborn: [lv * 10, lv * 10, lv * 10, lv * 10],
        })
        .collect()
}

fn make_skills() -> HashMap<i64, Skill> {
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
        15001,
        Skill {
            id: 15001,
            name: "Catch".to_string(),
            sl_danh: 1,
            skill_type: 11,
            delay: 5,
            ..Default::default()
        },
    );
    skills
}

fn make_pk_vs_normal_battle(kind: BattleKind, npc_hp: i64) -> (Battle, HashMap<i64, Npc>) {
    let mut npcs = HashMap::new();
    npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"Boar".to_vec(),
            lv: 20,
            hp: npc_hp,
            sp: 10,
            atk: 10,
            def: 1,
            agi: 1,
            item: [26001; 6],
            can_capture: true,
            ..Default::default()
        },
    );

    let mut battle = Battle::with_seeds(1, 112, 4, 1, 1).with_kind(kind);
    // Player at (3, 2)
    {
        let p = battle.cell_mut(3, 2).unwrap();
        p.typ = 2;
        p.id = 100;
        p.hp = 500;
        p.hp_max = 500;
        p.sp = 100;
        p.sp_max = 100;
        p.lv = 20;
        p.atk = 500;
        p.def = 50;
        p.agi = 200;
        p.team = 1;
        p.leader_id = 100;
    }
    // Player's active pet at (2, 2)
    {
        let pet = battle.cell_mut(2, 2).unwrap();
        pet.typ = 4;
        pet.id = 1002;
        pet.id_char = 100;
        pet.id_npc_on_map = 1;
        pet.hp = 500;
        pet.hp_max = 500;
        pet.sp = 100;
        pet.sp_max = 100;
        pet.lv = 20;
        pet.atk = 500;
        pet.def = 50;
        pet.agi = 190;
        pet.team = 1;
        pet.leader_id = 100;
    }
    // Enemy NPC at (0, 2)
    battle.add_npc(npcs.get(&1001).unwrap(), 1, 0, 2, 3);
    (battle, npcs)
}

#[test]
fn battle_kind_defaults_to_normal_and_is_independent_of_fight_type() {
    let mut b_default = Battle::new(1, 112);
    assert_eq!(b_default.kind, BattleKind::Normal);
    assert!(b_default.kind.allows_rewards());

    b_default.fight_type = 5;
    assert_eq!(b_default.kind, BattleKind::Normal);

    let mut b_pk = Battle::new(2, 112).with_kind(BattleKind::Pk);
    assert_eq!(b_pk.kind, BattleKind::Pk);
    assert!(!b_pk.kind.allows_rewards());

    b_pk.fight_type = 0;
    assert_eq!(b_pk.kind, BattleKind::Pk);
}

#[test]
fn pk_battle_suppresses_drop_gold_exp_and_god_while_normal_grants_all() {
    let skills = make_skills();
    let items = HashMap::new();
    let mut pet_slots = HashMap::new();
    pet_slots.insert(100, [1002, 0, 0, 0]);
    let texps = make_texps();

    let mut players = HashMap::new();
    players.insert(
        100,
        PlayerSnapshot {
            texp: 0,
            job: 0,
            hpx: 10,
            spx: 10,
            god: 5,
            ..Default::default()
        },
    );

    let run_kind = |kind: BattleKind| -> Vec<Out> {
        let (mut battle, npcs) = make_pk_vs_normal_battle(kind, 1);
        let data = BattleData {
            npcs: &npcs,
            skills: &skills,
            binary_skills: None,
            items: &items,
            pet_slots: &pet_slots,
            player_skills: None,
            world: None,
            talking_battle: 0,
            map_id: 12001,
            players: &players,
            texps: &texps,
        };
        let mut cmds: BattleCommands = HashMap::new();
        // Pet (agi 220, atk 20) chips first without killing (gap 20 > 11 so no combo),
        // then player (agi 200, atk 2000) finishes the NPC so both earn EXP in Normal.
        battle.cell_mut(2, 2).unwrap().agi = 220;
        battle.cell_mut(2, 2).unwrap().atk = 20;
        battle.cell_mut(3, 2).unwrap().atk = 2000;
        battle.cell_mut(0, 2).unwrap().hp = 500;
        battle.cell_mut(0, 2).unwrap().hp_max = 500;
        cmds.insert(
            (3, 2),
            BattleCommand {
                row: 3,
                col: 2,
                target_row: 0,
                target_col: 2,
                action: BattleAction::NormalAttack,
            },
        );
        cmds.insert(
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
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::PlayerWin);
        battle.finish(&data, 1, false, true, &mut out);
        out
    };

    // Normal battle grants drop, gold, player EXP (with level up), and deducts Stat::God.
    let normal_out = run_kind(BattleKind::Normal);
    assert!(
        normal_out.iter().any(|o| matches!(o, Out::Drop { .. })),
        "Normal battle must emit Out::Drop"
    );
    assert!(
        normal_out
            .iter()
            .any(|o| matches!(o, Out::Broadcast(f) if f.contains("3504"))),
        "Normal battle must emit 35 04 drop frame"
    );
    assert!(
        normal_out.iter().any(|o| matches!(o, Out::Gold { .. })),
        "Normal battle must emit Out::Gold"
    );
    assert!(
        normal_out.iter().any(|o| matches!(
            o,
            Out::Db(u) if u.target == DbTarget::Player(100) && u.stat == Stat::Texp && u.value > 0
        )),
        "Normal battle must grant Player Texp"
    );
    assert!(
        normal_out.iter().any(|o| matches!(
            o,
            Out::PetExp { owner: 100, stt: 1, exp } if *exp > 0
        )),
        "Normal battle must grant PetExp when pet hits"
    );
    assert!(
        normal_out.iter().any(|o| matches!(
            o,
            Out::Db(u) if u.target == DbTarget::Player(100) && u.stat == Stat::God && u.value == 4
        )),
        "Normal battle must decrement Stat::God"
    );

    // PKBattle suppresses ALL drops, gold, player EXP, level ups, pet EXP, and Stat::God deduction.
    let pk_out = run_kind(BattleKind::Pk);
    assert!(
        !pk_out.iter().any(|o| matches!(o, Out::Drop { .. })),
        "PKBattle must not emit Out::Drop"
    );
    assert!(
        !pk_out
            .iter()
            .any(|o| matches!(o, Out::Broadcast(f) if f.contains("3504"))),
        "PKBattle must not emit 35 04 drop frame"
    );
    assert!(
        !pk_out.iter().any(|o| matches!(o, Out::Gold { .. })),
        "PKBattle must not emit Out::Gold"
    );
    assert!(
        !pk_out
            .iter()
            .any(|o| matches!(o, Out::PetExp { .. })),
        "PKBattle must not emit Out::PetExp"
    );
    assert!(
        !pk_out.iter().any(|o| matches!(
            o,
            Out::Db(u) if matches!(u.stat, Stat::Texp | Stat::Lv | Stat::God)
        )),
        "PKBattle must not write Texp, Lv, or deduct God"
    );
    // Cleanup packets (0B 00, 0B 01, 05 04, 14 08) must still be emitted.
    assert!(pk_out
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if f.contains("0B00") && f.contains("0B01"))));
    assert!(pk_out
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if f == "F44402000504")));
    assert!(pk_out
        .iter()
        .any(|o| matches!(o, Out::Broadcast(f) if f == "F44402001408")));
}

#[test]
fn pk_battle_preserves_capture_15001_and_npc_respawn_without_pet_exp() {
    let skills = make_skills();
    let items = HashMap::new();
    let mut pet_slots = HashMap::new();
    pet_slots.insert(100, [0, 0, 0, 0]);
    let texps = make_texps();
    let mut players = HashMap::new();
    players.insert(100, PlayerSnapshot::default());

    let world = RwLock::new(NpcWorld::new(&[NpcOnMap {
        map_id: 12001,
        id: 7,
        npc_id: 1001,
        x: 500,
        y: 500,
        coord: 40,
        so_luong: 1,
        ..Default::default()
    }]));

    // 1. Capture succeeds in PKBattle when HP < 50% and level advantage yields 100% roll.
    let (mut battle, mut npcs) = make_pk_vs_normal_battle(BattleKind::Pk, 100);
    battle.cell_mut(3, 2).unwrap().lv = 120;
    battle.cell_mut(0, 2).unwrap().hp = 40;
    npcs.get_mut(&1001).unwrap().hp = 100;

    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: None,
        items: &items,
        pet_slots: &pet_slots,
        player_skills: None,
        world: Some(&world),
        talking_battle: 7,
        map_id: 12001,
        players: &players,
        texps: &texps,
    };

    // Player (agi 200) captures NPC (15001) on turn 1.
    let mut cmds: BattleCommands = HashMap::new();
    cmds.insert(
        (3, 2),
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::Capture,
        },
    );
    cmds.insert(
        (2, 2),
        BattleCommand {
            row: 2,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::Defend,
        },
    );

    let mut out = Vec::new();
    let outcome = battle.run_turn(&data, &cmds, &mut out);
    assert_eq!(outcome, Outcome::PlayerWin, "Capture of sole NPC wins the PKBattle");
    assert!(
        out.iter()
            .any(|o| matches!(o, Out::Catch { owner: 100, npc_id: 1001 })),
        "Capture (15001) must still succeed inside PKBattle"
    );

    battle.finish(&data, 1, false, true, &mut out);
    assert!(
        out.iter()
            .any(|o| matches!(o, Out::Respawn { npc_id: 7, map_id: 12001, .. })),
        "Map NPC respawn (when talking_battle > 0) must still run after PKBattle"
    );
    assert!(
        !out.iter().any(|o| matches!(o, Out::PetExp { .. })),
        "PKBattle must not award PetExp"
    );
}

fn split_frames(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = raw;
    while rest.starts_with("F444") && rest.len() >= 8 {
        let lo = u8::from_str_radix(&rest[4..6], 16).unwrap() as usize;
        let hi = u8::from_str_radix(&rest[6..8], 16).unwrap() as usize;
        let byte_len = 4 + (lo | (hi << 8));
        let hex_len = byte_len * 2;
        if rest.len() < hex_len {
            break;
        }
        out.push(rest[..hex_len].to_string());
        rest = &rest[hex_len..];
    }
    out
}

fn service_game_data() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"MapNpc".to_vec(),
            lv: 10,
            hp: 1,
            sp: 10,
            atk: 10,
            def: 1,
            agi: 1,
            item: [26001; 6],
            can_capture: true,
            ..Default::default()
        },
    );
    data.skills = make_skills();
    data.texps = make_texps();
    data
}

async fn drain_until_battle_ends(
    rx: &mut tokio::sync::mpsc::Receiver<String>,
) -> Vec<String> {
    let mut frames = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(chunk)) => {
                let split = split_frames(&chunk);
                let finished = split.iter().any(|f| f == "F44402001408");
                if split.is_empty() {
                    frames.push(chunk);
                } else {
                    frames.extend(split);
                }
                if finished {
                    break;
                }
            }
            _ => break,
        }
    }
    frames
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handle_attack_npc_0b_02_03_decodes_slot_uses_diahinh_112_and_runs_pk_battle_even_when_pk_off() {
    let data = service_game_data();
    let mut service = BattleService::new(Arc::new(data.clone()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let pid = 7101u32;
    let sess = Arc::new(AsyncRwLock::new(Session {
        id: pid,
        pk: 0, // Option PK switch OFF
        level: 20,
        atk: 2000,
        agi: 200,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        god: 5,
        active_pet_stt: 1,
        pets: vec![PetState {
            stt: 1,
            id: 1002,
            level: 20,
            hp: 500,
            hp_max: 500,
            sp: 100,
            sp_max: 100,
            atk: 10,
            agi: 50,
            ..Default::default()
        }],
        ..Default::default()
    }));
    let mut rx = service.register(i64::from(pid), sess.clone());
    lock_online_sessions().insert(pid, sess.read().await.clone());

    let mut conn = Conn::default();
    conn.session = sess.read().await.clone();

    // C->S 0x0B 02 03 <npcId=1001 LE32> <npcMapSlot=42 LE16>
    let mut payload = vec![3u8];
    payload.extend_from_slice(&1001u32.to_le_bytes());
    payload.extend_from_slice(&42u16.to_le_bytes());
    let mut out = HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x0B,
        sub: 2,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };
    battle::handle_battle(&mut ctx);
    assert_ne!(
        conn.session.battle_id, 0,
        "0x0B 02 03 must open a battle even when session.pk == 0"
    );

    // Send 0B FA scene ack + 0x32 01 NormalAttack so the battle completes cleanly.
    let ack_frame = [0xF4, 0x44, 0x02, 0x00, 0x0B, 0xFA];
    let _ = dispatch(&mut conn, &ack_frame, &data, &service, &ServerEnv::none()).await;

    let atk_frame = [
        0xF4, 0x44, 0x0A, 0x00, 0x32, 0x01, 0x03, 0x02, 0x00, 0x02, 0x10, 0x27, 0x00, 0x00,
    ];
    let _ = dispatch(&mut conn, &atk_frame, &data, &service, &ServerEnv::none()).await;

    let frames = drain_until_battle_ends(&mut rx).await;

    // 1. Check 0B FA diahinh == 112 (0x0070 -> LE "7000"), NOT slot 42 ("2A00").
    let open_frame = frames
        .iter()
        .find(|f| f.len() >= 16 && &f[8..12] == "0BFA")
        .expect("leader must receive 0B FA");
    assert_eq!(
        &open_frame[12..16],
        "7000",
        "0x0B 02 03 must use default DiaHinh 112 (7000 LE), not the map slot"
    );

    // 2. Check 0B 05 03 (entity_npc) carries id_npc_on_map == 42 (0x002A -> LE "2A00") at bytes 5..7.
    let npc_frame = frames
        .iter()
        .find(|f| f.len() >= 28 && &f[8..14] == "0B0503")
        .expect("leader must receive 0B 05 03 entity_npc");
    // After F444 + LEN(2B) + 0B0503(3B) = 7 bytes (14 hex chars), WarInfo packet starts:
    // typ(1B) + id(4B) + id_npc_on_map(2B LE) => offset 14 + 10 = 24..28.
    assert_eq!(
        &npc_frame[24..28],
        "2A00",
        "entity_npc must carry decoded npc_on_map_id = 42 (2A00 LE)"
    );

    // 3. Verify PKBattle restrictions: no drop/gold/Texp/PetExp and Stat::God unchanged.
    assert!(
        !frames.iter().any(|f| f.len() >= 12 && &f[8..12] == "3504"),
        "no 35 04 drop in PKBattle"
    );
    assert!(
        !frames.iter().any(|f| f.len() >= 12 && &f[8..12] == "1706"),
        "no 17 06 item in PKBattle"
    );
    assert!(
        !frames.iter().any(|f| f.len() >= 12 && &f[8..12] == "1A01"),
        "no 1A 01 gold in PKBattle"
    );
    assert!(
        !frames.iter().any(|f| f.starts_with("F4440C00080124")),
        "no Stat::Texp in PKBattle"
    );
    assert!(
        !frames.iter().any(|f| f.len() >= 12 && &f[8..12] == "0F08"),
        "no 0F 08 PetExp in PKBattle"
    );

    let final_sess = sess.read().await;
    assert_eq!(final_sess.god, 5, "PKBattle must not deduct Stat::God");
    lock_online_sessions().remove(&pid);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normal_pve_encounter_and_teamdef_keep_normal_battle_kind_and_rewards_even_with_pk_on() {
    let data = service_game_data();
    let mut service = BattleService::new(Arc::new(data.clone()));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    // A) Eve random encounter with session.pk == 1 and fight_type == 3
    let pid1 = 7102u32;
    let sess1 = Arc::new(AsyncRwLock::new(Session {
        id: pid1,
        pk: 1,
        level: 20,
        atk: 2000,
        agi: 200,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        god: 5,
        ..Default::default()
    }));
    let mut rx1 = service.register(i64::from(pid1), sess1.clone());
    lock_online_sessions().insert(pid1, sess1.read().await.clone());

    let mut conn1 = Conn::default();
    conn1.session = sess1.read().await.clone();
    let fight = EveFightData {
        eve_no: 1,
        fight_type: 3,
        left_enemies: vec![EveFightEnemy {
            npc_id: 1001,
            location_pos: 2,
            ..Default::default()
        }],
        right_enemies: vec![],
        ..Default::default()
    };
    let bid1 = service.start_encounter_battle(&mut conn1.session, &fight, 109);
    assert_ne!(bid1, 0);
    let _ = dispatch(
        &mut conn1,
        &[0xF4, 0x44, 0x02, 0x00, 0x0B, 0xFA],
        &data,
        &service,
        &ServerEnv::none(),
    )
    .await;
    let _ = dispatch(
        &mut conn1,
        &[
            0xF4, 0x44, 0x0A, 0x00, 0x32, 0x01, 0x03, 0x02, 0x00, 0x02, 0x10, 0x27, 0x00, 0x00,
        ],
        &data,
        &service,
        &ServerEnv::none(),
    )
    .await;
    let frames1 = drain_until_battle_ends(&mut rx1).await;
    assert!(
        frames1.iter().any(|f| f.len() >= 12 && &f[8..12] == "1A01"),
        "Normal Eve encounter must still award gold even when pk == 1"
    );
    assert!(
        frames1.iter().any(|f| f.starts_with("F4440C00080124")),
        "Normal Eve encounter must still award Texp even when pk == 1"
    );
    assert_eq!(sess1.read().await.god, 4, "Normal Eve encounter must deduct Stat::God");
    lock_online_sessions().remove(&pid1);

    // B) TeamDef battle with session.pk == 1
    let pid2 = 7103u32;
    let sess2 = Arc::new(AsyncRwLock::new(Session {
        id: pid2,
        pk: 1,
        level: 20,
        atk: 2000,
        agi: 200,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        god: 5,
        ..Default::default()
    }));
    let mut rx2 = service.register(i64::from(pid2), sess2.clone());
    lock_online_sessions().insert(pid2, sess2.read().await.clone());

    let mut conn2 = Conn::default();
    conn2.session = sess2.read().await.clone();
    let trigger = BattleTrigger {
        teamdef: vec![4712, 1001],
        diahinh: 4712,
    };
    let bid2 = service.start_teamdef_battle(&mut conn2.session, &trigger);
    assert_ne!(bid2, 0);
    let _ = dispatch(
        &mut conn2,
        &[0xF4, 0x44, 0x02, 0x00, 0x0B, 0xFA],
        &data,
        &service,
        &ServerEnv::none(),
    )
    .await;
    let _ = dispatch(
        &mut conn2,
        &[
            0xF4, 0x44, 0x0A, 0x00, 0x32, 0x01, 0x03, 0x02, 0x00, 0x00, 0x10, 0x27, 0x00, 0x00,
        ],
        &data,
        &service,
        &ServerEnv::none(),
    )
    .await;
    let frames2 = drain_until_battle_ends(&mut rx2).await;
    assert!(
        frames2.iter().any(|f| f.len() >= 12 && &f[8..12] == "1A01"),
        "TeamDef battle must still award gold even when pk == 1"
    );
    assert!(
        frames2.iter().any(|f| f.starts_with("F4440C00080124")),
        "TeamDef battle must still award Texp even when pk == 1"
    );
    assert_eq!(sess2.read().await.god, 4, "TeamDef battle must deduct Stat::God");
    lock_online_sessions().remove(&pid2);
}

fn self_kind_from_open_frame(frame: &str) -> u8 {
    assert!(frame.starts_with("F444"));
    let lo = u8::from_str_radix(&frame[4..6], 16).unwrap() as usize;
    let hi = u8::from_str_radix(&frame[6..8], 16).unwrap() as usize;
    let declared = lo | (hi << 8);
    let payload = &frame[8..8 + declared * 2];
    assert!(payload.starts_with("0BFA"));
    u8::from_str_radix(&payload[8..10], 16).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn battle_0b_fa_assigns_03_to_leader_05_to_member_and_04_to_listqs_spectator() {
    let data = service_game_data();
    let mut service = BattleService::new(Arc::new(data.clone()));
    service.set_input_timeout(std::time::Duration::from_millis(200));

    let leader_id = 7201u32;
    let member_id = 7202u32;
    let spec_id = 7203u32;

    let leader_sess = Arc::new(AsyncRwLock::new(Session {
        id: leader_id,
        id_leader: leader_id,
        id_mem: [member_id, 0, 0, 0],
        level: 20,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        ..Default::default()
    }));
    let member_sess = Arc::new(AsyncRwLock::new(Session {
        id: member_id,
        id_leader: leader_id,
        level: 20,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        ..Default::default()
    }));
    let spec_sess = Arc::new(AsyncRwLock::new(Session {
        id: spec_id,
        level: 20,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        ..Default::default()
    }));

    let mut rx_leader = service.register(i64::from(leader_id), leader_sess.clone());
    let mut rx_member = service.register(i64::from(member_id), member_sess.clone());
    let mut rx_spec = service.register(i64::from(spec_id), spec_sess.clone());

    lock_online_sessions().insert(leader_id, leader_sess.read().await.clone());
    lock_online_sessions().insert(member_id, member_sess.read().await.clone());
    lock_online_sessions().insert(spec_id, spec_sess.read().await.clone());

    let mut leader_conn = Conn::default();
    leader_conn.session = leader_sess.read().await.clone();
    let fight = EveFightData {
        eve_no: 1,
        fight_type: 1,
        left_enemies: vec![EveFightEnemy {
            npc_id: 1001,
            location_pos: 2,
            ..Default::default()
        }],
        right_enemies: vec![],
        ..Default::default()
    };
    let bid = service.start_encounter_battle(&mut leader_conn.session, &fight, 112);
    assert_ne!(bid, 0);

    // Spectator joins via 0x0B 02 04 targeting the leader.
    let mut spec_conn = Conn::default();
    spec_conn.session = spec_sess.read().await.clone();
    let mut spec_out = HandleOutcome::default();
    let mut join_payload = vec![4u8];
    join_payload.extend_from_slice(&leader_id.to_le_bytes());
    let mut join_ctx = OpcodeCtx {
        conn: &mut spec_conn,
        data: &data,
        service: &service,
        out: &mut spec_out,
        opcode: 0x0B,
        sub: 2,
        payload: &join_payload,
        decoded: &[],
        env: ServerEnv::none(),
    };
    battle::handle_battle(&mut join_ctx);
    assert_eq!(spec_conn.session.battle_id, bid);
    assert!(service.is_spectator(i64::from(spec_id)));

    let leader_open = rx_leader
        .try_recv()
        .ok()
        .and_then(|c| split_frames(&c).into_iter().find(|f| f.contains("0BFA")))
        .expect("leader 0B FA");
    let member_open = {
        let mut found = None;
        while let Ok(c) = rx_member.try_recv() {
            if let Some(f) = split_frames(&c).into_iter().find(|f| f.contains("0BFA")) {
                found = Some(f);
                break;
            }
        }
        found.expect("member 0B FA")
    };
    let spec_open = {
        let mut found = None;
        while let Ok(c) = rx_spec.try_recv() {
            if let Some(f) = split_frames(&c).into_iter().find(|f| f.contains("0BFA")) {
                found = Some(f);
                break;
            }
        }
        found.expect("spectator 0B FA")
    };

    assert_eq!(self_kind_from_open_frame(&leader_open), 0x03);
    assert_eq!(self_kind_from_open_frame(&member_open), 0x05);
    assert_eq!(self_kind_from_open_frame(&spec_open), 0x04);

    // Spectator leaves via 0x0B 01 03 and cleans up spectator state.
    let mut leave_out = HandleOutcome::default();
    let mut leave_ctx = OpcodeCtx {
        conn: &mut spec_conn,
        data: &data,
        service: &service,
        out: &mut leave_out,
        opcode: 0x0B,
        sub: 1,
        payload: &[3],
        decoded: &[],
        env: ServerEnv::none(),
    };
    battle::handle_battle(&mut leave_ctx);
    assert_eq!(spec_conn.session.battle_id, 0);
    assert!(!service.is_spectator(i64::from(spec_id)));
    assert!(leave_out.outgoing.iter().any(|f| f.frame.contains("0B00")));
    assert!(leave_out.outgoing.iter().any(|f| f.frame.contains("0504")));
    assert!(leave_out.outgoing.iter().any(|f| f.frame.contains("1408")));

    // Member (combatant) trying 0x0B 01 03 is rejected.
    let mut mem_conn = Conn::default();
    mem_conn.session = member_sess.read().await.clone();
    assert_eq!(mem_conn.session.battle_id, bid);
    let mut mem_leave_out = HandleOutcome::default();
    let mut mem_leave_ctx = OpcodeCtx {
        conn: &mut mem_conn,
        data: &data,
        service: &service,
        out: &mut mem_leave_out,
        opcode: 0x0B,
        sub: 1,
        payload: &[3],
        decoded: &[],
        env: ServerEnv::none(),
    };
    battle::handle_battle(&mut mem_leave_ctx);
    assert_eq!(mem_conn.session.battle_id, bid);
    assert!(mem_leave_out.outgoing.is_empty());

    lock_online_sessions().remove(&leader_id);
    lock_online_sessions().remove(&member_id);
    lock_online_sessions().remove(&spec_id);
}
