//! Integration tests for Tickets 04 and 05: Continuous Dungeon In-Engine Wave Chaining
//! and Tower Climbing Progression (40 NPC, Rescue 2K, Cửu Thiên).
//!
//! Verifies:
//! - 40 NPC entry gating at map 12001 NPC 15162 (ticket 46072 vs 1000 gold vs rejection)
//! - In-engine unbroken wave progression in `Battle` and `BattleRunner` without map reloads
//! - Rescue 2K party size gating at map 12003 NPC 15312 (strictly 1 or 5 members)
//! - Broadcast alert `14 29 02` upon Rescue 2K start
//! - Rescue 2K completion rewards (feather 23086, bean 26051, credentials 46049/46050)
//! - Cửu Thiên entry gating at map 33033 NPC 61209 (companion follower 14735 + weekly quota)
//! - Weekly quota reset via Item 46644 ("Cửu Thiên Chiến Kỳ")
//! - Cửu Thiên floor chest NPC 20001 rewards and weekly completion flag at floor 49969
//! - Disconnection cleanup evacuating players in continuous dungeon instances to `savemap`

use ts_dream::battle::construction::Battle;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Item, Npc};
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::handlers::talk::handle_talk;
use ts_dream::server::session::{lock_online_sessions, Conn, InventoryItem, PetState, Session};
use ts_dream::web::server_control::ServerControl;

mod common;
use common::test_db::setup_test_db;

// =========================================================================
// Ticket 04: Continuous Dungeon 40 NPC Wave Chaining
// =========================================================================

#[tokio::test]
async fn test_40_npc_entry_gating() {
    let data = GameData::default();
    let service = BattleService::default();

    // 1. Success via Ticket 46072
    {
        let mut conn = Conn::default();
        conn.session.id = 4001;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        conn.session.gold = 100;
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 46072,
            count: 1,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let payload = [0x3A, 0x3B]; // NPC 15162
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 10851, "must warp to arena map 10851");
        assert!(conn.session.homdo.iter().all(|i| i.id != 46072 || i.count == 0), "ticket must be consumed");
        assert_eq!(conn.session.gold, 100, "gold must not be touched");
        assert_eq!(
            conn.session.quest_tasks.get(&218).map(|&(_, s)| s),
            Some(1),
            "quest 218 must be set"
        );
    }

    // 2. Success via 1,000 gold when no ticket
    {
        let mut conn = Conn::default();
        conn.session.id = 4002;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        conn.session.gold = 2500;

        let mut out = HandleOutcome::default();
        let payload = [0x3A, 0x3B];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 10851, "must warp to arena map 10851");
        assert_eq!(conn.session.gold, 1500, "1000 gold must be deducted");
        assert_eq!(
            conn.session.quest_tasks.get(&218).map(|&(_, s)| s),
            Some(1),
            "quest 218 must be set"
        );
    }

    // 3. Rejection when neither ticket nor 1,000 gold
    {
        let mut conn = Conn::default();
        conn.session.id = 4003;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        conn.session.gold = 999;

        let mut out = HandleOutcome::default();
        let payload = [0x3A, 0x3B];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 12001, "must remain on map 12001");
        assert_eq!(conn.session.gold, 999, "gold must remain untouched");
        let has_err = out.outgoing.iter().any(|f| f.contains("Khong du tien hoac ve loi dai") || f.contains("020102"));
        assert!(has_err, "must toast rejection error message");
    }
}

#[test]
fn test_in_engine_wave_chaining() {
    let mut session = Session::default();
    session.id = 1001;
    session.hp = 1000;
    session.sp = 500;

    let mut npc1 = Npc::default();
    npc1.id = 101;
    npc1.hp = 50;
    npc1.lv = 10;

    let mut npc2 = Npc::default();
    npc2.id = 102;
    npc2.hp = 60;
    npc2.lv = 15;

    let mut npc3 = Npc::default();
    npc3.id = 103;
    npc3.hp = 80;
    npc3.lv = 20;

    let mut battle = Battle::new(501, 100);
    battle.add_player(&session, 1001, 3, 2);
    // Initial wave 1: npc1 at (0, 2)
    battle.add_npc(&npc1, 1, 0, 2, 3);

    // Queue wave 2 and wave 3
    let waves = vec![
        vec![(npc1.clone(), 0, 2)],
        vec![(npc2.clone(), 0, 1), (npc2.clone(), 0, 3)],
        vec![(npc3.clone(), 0, 2)],
    ];
    battle.set_waves(waves);
    assert_eq!(battle.current_wave, 0);

    // 1. Defeat wave 1: set enemy cell HP to 0
    if let Some(cell) = battle.cell_mut(0, 2) {
        cell.hp = 0;
    }
    assert!(battle.all_enemies_dead());

    // Advance to wave 2
    let mut out = Vec::new();
    let advanced = battle.advance_wave(&mut out);
    assert!(advanced, "wave must advance to wave 2");
    assert_eq!(battle.current_wave, 1);

    let has_reset = out.iter().any(|o| match o {
        ts_dream::battle::runner::Out::Broadcast(s) => s.starts_with("F44404000B01"),
        _ => false,
    });
    assert!(has_reset, "reset_cell (0B 01) must be emitted on wave advance");

    let has_entity = out.iter().any(|o| match o {
        ts_dream::battle::runner::Out::Broadcast(s) => s.starts_with("F444") && s.contains("0B0503"),
        _ => false,
    });
    assert!(has_entity, "entity_npc (0B 05 03) must be emitted for new wave NPCs");

    assert!(!battle.all_enemies_dead());
    assert!(battle.cell(0, 1).is_some());
    assert!(battle.cell(0, 3).is_some());

    // 2. Defeat wave 2
    if let Some(cell) = battle.cell_mut(0, 1) {
        cell.hp = 0;
    }
    if let Some(cell) = battle.cell_mut(0, 3) {
        cell.hp = 0;
    }
    assert!(battle.all_enemies_dead());

    // Advance to wave 3 (final wave)
    let advanced2 = battle.advance_wave(&mut out);
    assert!(advanced2, "wave must advance to wave 3");
    assert_eq!(battle.current_wave, 2);

    // 3. Defeat wave 3
    if let Some(cell) = battle.cell_mut(0, 2) {
        cell.hp = 0;
    }
    assert!(battle.all_enemies_dead());

    // Advance beyond final wave must return false
    let advanced_final = battle.advance_wave(&mut out);
    assert!(!advanced_final, "advance_wave must return false after clearing final wave");
}

// =========================================================================
// Ticket 05: Tower Climbing Progression for Rescue 2K & Cửu Thiên
// =========================================================================

#[tokio::test]
async fn test_rescue_2k_party_size_gating() {
    let data = GameData::default();
    let service = BattleService::default();

    // 1. Solo challenger (party size 1): accepted
    {
        let mut conn = Conn::default();
        conn.session.id = 5001;
        conn.session.map_id = 12003;
        conn.session.map_x = 400;
        conn.session.map_y = 400;

        let mut out = HandleOutcome::default();
        let payload = [0xD0, 0x3B]; // NPC 15312
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 12911, "solo player must warp to floor 1 (12911)");
        assert_eq!(
            conn.session.quest_tasks.get(&212).map(|&(_, s)| s),
            Some(1),
            "quest 212 must be set"
        );
        let has_broadcast = out.outgoing.iter().any(|f| f.contains("142902"))
            || out.map_broadcast.iter().any(|b| b.frame.contains("142902"));
        assert!(has_broadcast, "must broadcast alert 14 29 02 event started");
    }

    // 2. Full team of 5 (leader + 4 members): accepted
    {
        let mut conn = Conn::default();
        conn.session.id = 5002;
        conn.session.id_leader = 5002;
        conn.session.id_mem = [5003, 5004, 5005, 5006];
        conn.session.map_id = 12003;
        conn.session.map_x = 400;
        conn.session.map_y = 400;

        let mut out = HandleOutcome::default();
        let payload = [0xD0, 0x3B];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 12911, "leader must warp to floor 1 (12911)");
    }

    // 3. Invalid party size: 3 members -> rejected
    {
        let mut conn = Conn::default();
        conn.session.id = 5007;
        conn.session.id_leader = 5007;
        conn.session.id_mem = [5008, 5009, 0, 0];
        conn.session.map_id = 12003;

        let mut out = HandleOutcome::default();
        let payload = [0xD0, 0x3B];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 12003, "must remain on map 12003");
        let has_err = out.outgoing.iter().any(|f| f.contains("Chi cho phep to doi 1 nguoi hoac 5 nguoi") || f.contains("020102"));
        assert!(has_err, "must toast party size error");
    }

    // 4. Non-leader party member attempts entry -> rejected
    {
        let mut conn = Conn::default();
        conn.session.id = 5010;
        conn.session.id_leader = 5002; // Leader is 5002, conn is member 5010
        conn.session.id_mem = [5002, 5011, 5012, 5013];
        conn.session.map_id = 12003;

        let mut out = HandleOutcome::default();
        let payload = [0xD0, 0x3B];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 12003, "must remain on map 12003");
        let has_err = out.outgoing.iter().any(|f| f.contains("Chi doi truong moi co the dang ky") || f.contains("020102"));
        assert!(has_err, "must toast non-leader error");
    }
}

#[tokio::test]
async fn test_rescue_2k_floor_rewards() {
    let mut data = GameData::default();
    data.items.insert(23086, Item { id: 23086, name: b"Feather".to_vec(), ..Default::default() });
    data.items.insert(26051, Item { id: 26051, name: b"Bean".to_vec(), ..Default::default() });
    data.items.insert(46049, Item { id: 46049, name: b"CredentialDai".to_vec(), ..Default::default() });
    let service = BattleService::default();

    let mut conn = Conn::default();
    conn.session.id = 6001;
    conn.session.map_id = 12927; // Floor 17 (Đại Kiều)
    conn.session.map_x = 400;
    conn.session.map_y = 400;

    let mut out = HandleOutcome::default();
    let payload = [0x21, 0x4E]; // NPC 20001
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x14,
        sub: 1,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    handle_talk(&mut ctx).await;
    let has_feather = conn.session.homdo.iter().any(|i| i.id == 23086 && i.count >= 1);
    let has_bean = conn.session.homdo.iter().any(|i| i.id == 26051 && i.count >= 1);
    let has_cred = conn.session.homdo.iter().any(|i| i.id == 46049 && i.count >= 1);

    assert!(has_feather, "must receive feather 23086");
    assert!(has_bean, "must receive bean 26051");
    assert!(has_cred, "must receive credential 46049");
}

#[tokio::test]
async fn test_cuu_thien_gating_and_quota_reset() {
    let data = GameData::default();
    let service = BattleService::default();

    // 1. Missing companion follower Hầu Tôn (14735) -> rejected
    {
        let mut conn = Conn::default();
        conn.session.id = 7001;
        conn.session.map_id = 33033;
        conn.session.map_x = 400;
        conn.session.map_y = 400;

        let mut out = HandleOutcome::default();
        let payload = [0x19, 0xEF]; // NPC 61209
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 33033, "must remain on map 33033");
        let has_err = out.outgoing.iter().any(|f| f.contains("Can phai co Hau Ton di theo") || f.contains("020102"));
        assert!(has_err, "must toast missing follower error");
    }

    // 1b. Companion Hầu Tôn present in pet list (stt 2) but NOT active follower (active_pet_stt == 0) -> rejected
    {
        let mut conn = Conn::default();
        conn.session.id = 70012;
        conn.session.map_id = 33033;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        conn.session.active_pet_stt = 0; // NOT active
        conn.session.pets.push(PetState {
            stt: 2,
            id: 14735,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let payload = [0x19, 0xEF]; // NPC 61209
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 33033, "must remain on map 33033");
        let has_err = out.outgoing.iter().any(|f| f.contains("Can phai co Hau Ton di theo") || f.contains("020102"));
        assert!(has_err, "must toast missing follower error when pet is inactive in roster");
    }

    // 2. Companion follower present, but weekly quota already used (QuestStep 12310 == 1) -> rejected
    {
        let mut conn = Conn::default();
        conn.session.id = 7002;
        conn.session.map_id = 33033;
        conn.session.active_pet_stt = 1;
        conn.session.pets.push(PetState {
            stt: 1,
            id: 14735,
            ..Default::default()
        });
        conn.session.quest_tasks.insert(12310, (1, 1));

        let mut out = HandleOutcome::default();
        let payload = [0x19, 0xEF];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut ctx).await;
        assert_eq!(conn.session.map_id, 33033, "must remain on map 33033");
        let has_err = out.outgoing.iter().any(|f| f.contains("Hien tai van chua den luc") || f.contains("020102"));
        assert!(has_err, "must toast weekly quota already consumed");
    }

    // 3. Consuming Item 46644 ("Cửu Thiên Chiến Kỳ") resets weekly quota
    {
        let mut conn = Conn::default();
        conn.session.id = 7003;
        conn.session.quest_tasks.insert(12310, (1, 1));
        conn.session.homdo.push(InventoryItem {
            slot: 1,
            id: 46644,
            count: 1,
            ..Default::default()
        });

        let mut out = HandleOutcome::default();
        let payload = [1, 0];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 15,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_inventory(&mut ctx).await;
        assert_eq!(
            conn.session.quest_tasks.get(&12310).map(|&(_, s)| s),
            Some(0),
            "quota step must be reset to 0"
        );
        assert!(
            conn.session.homdo.iter().all(|i| i.id != 46644 || i.count == 0),
            "flag item must be consumed"
        );
        let has_toast = out.outgoing.iter().any(|f| f.contains("Da thiet lap lai luot Chinh Phuc Cuu Thien") || f.contains("020102"));
        assert!(has_toast, "must toast quota reset confirmation");

        // Now entry with Hầu Tôn succeeds!
        conn.session.map_id = 33033;
        conn.session.active_pet_stt = 1;
        conn.session.pets.push(PetState {
            stt: 1,
            id: 14735,
            ..Default::default()
        });

        let mut entry_out = HandleOutcome::default();
        let payload = [0x19, 0xEF];
        let mut entry_ctx = OpcodeCtx {
            conn: &mut conn,
            data: &data,
            service: &service,
            out: &mut entry_out,
            opcode: 0x14,
            sub: 1,
            payload: &payload,
            decoded: &[],
            env: ServerEnv::none(),
        };

        handle_talk(&mut entry_ctx).await;
        assert_eq!(conn.session.map_id, 49960, "must warp to Cửu Thiên floor 1 (49960)");
        assert_eq!(
            conn.session.quest_tasks.get(&12309).map(|&(_, s)| s),
            Some(1),
            "quest 12309 must be set"
        );
    }
}

#[tokio::test]
async fn test_cuu_thien_floor_chest_and_final_quota() {
    let mut data = GameData::default();
    data.items.insert(23086, Item { id: 23086, name: b"Feather".to_vec(), ..Default::default() });
    data.items.insert(26051, Item { id: 26051, name: b"Bean".to_vec(), ..Default::default() });
    let service = BattleService::default();

    let mut conn = Conn::default();
    conn.session.id = 8001;
    conn.session.map_id = 49969;
    conn.session.map_x = 400;
    conn.session.map_y = 400;

    let mut out = HandleOutcome::default();
    let payload = [0x21, 0x4E];
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        data: &data,
        service: &service,
        out: &mut out,
        opcode: 0x14,
        sub: 1,
        payload: &payload,
        decoded: &[],
        env: ServerEnv::none(),
    };

    handle_talk(&mut ctx).await;
    assert_eq!(
        conn.session.quest_tasks.get(&12310).map(|&(_, s)| s),
        Some(1),
        "floor 49969 chest must set weekly quota quest 12310 step 1"
    );
    let has_bean = conn.session.homdo.iter().any(|i| i.id == 26051 && i.count >= 1);
    let has_feather = conn.session.homdo.iter().any(|i| i.id == 23086 && i.count >= 1);
    assert!(has_bean, "must receive bean 26051");
    assert!(has_feather, "must receive feather 23086");
}

#[tokio::test]
async fn test_dungeon_disconnect_evacuation_to_savemap() {
    let db = setup_test_db().await;
    let repos = SqliteRepositories::new(db.clone());

    let acc_id = ts_dream::db::accounts::create(&db, "dungeon_runner", "pass")
        .await
        .expect("create account");

    let seed = ts_dream::db::modern::traits::CharacterSeed {
        level: 50,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 10851,
        map_x: 400,
        map_y: 400,
    };
    repos.characters().create(acc_id, b"DungeonHero", &seed).await.expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"DungeonHero".to_vec(),
        map_id: 10851, // Arena 40 NPC
        map_x: 400,
        map_y: 400,
        savemap: 12001,
        authed: true,
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("save session");

    let app = std::sync::Arc::new(tokio::sync::RwLock::new(ts_dream::state::AppState::new(100)));
    let control = ServerControl::new(6414, app, None, Some(db.clone()));

    lock_online_sessions().insert(acc_id as u32, session);

    // Trigger player disconnect
    control.disconnect_player(acc_id as u32).await;

    // Verify session removed from online registry
    assert!(lock_online_sessions().get(&(acc_id as u32)).is_none(), "session must be removed from online registry");

    // Verify reloaded session from database was evacuated to savemap (12001)
    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("session must exist in DB");
    assert_eq!(reloaded.map_id, 12001, "disconnected player must be evacuated to savemap 12001");
}
