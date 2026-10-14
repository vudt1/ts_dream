use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::session::{Conn, InventoryItem, PetState, Session};

async fn setup_test_db() -> DbPool {
    let pool = bootstrap("sqlite::memory:?cache=shared", None)
        .await
        .expect("bootstrap in-memory SQLite");

    let schema = include_str!("../migrations/0001_init.sql");
    sqlx::raw_sql(schema)
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");

    pool
}

#[tokio::test]
async fn test_pet_equip_and_unequip_flow() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "pet_user", "pet_pass")
        .await
        .expect("create account");

    let seed = ts_dream::db::modern::traits::CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(acc_id, b"Trainer", &seed).await.expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"Trainer".to_vec(),
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            name: b"Wolf".to_vec(),
            level: 20,
            ..Default::default()
        }],
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 5001,
                count: 1,
                loai: 1, // weapon
                lv: 15,  // pet level 20 >= 15
                ..Default::default()
            },
            InventoryItem {
                slot: 2,
                id: 5002,
                count: 1,
                loai: 1, // weapon
                lv: 18,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("save session");

    let mut conn = Conn::default();
    conn.session = session;
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    // 1. Equip item from homdo slot 1 to pet 1: sub 17, payload [pet_stt=1, item_slot=1]
    let payload_equip = [0x01, 0x01];
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_equip,
    };
    handle_inventory(&mut ctx).await;

    // Verify item 5001 is now in trangbi at slot 11 (pet 1 weapon = 1*10 + 1)
    assert!(conn.session.homdo.iter().all(|i| i.id != 5001));
    let eq = conn.session.trangbi.iter().find(|i| i.slot == 11);
    assert!(eq.is_some(), "pet equipment must be in trangbi slot 11");
    assert_eq!(eq.unwrap().id, 5001);

    // 2. Equip item 5002 from homdo slot 2 to pet 1: should swap existing equipped item 5001 back to homdo slot 2
    let payload_equip2 = [0x01, 0x02];
    let mut ctx2 = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_equip2,
    };
    handle_inventory(&mut ctx2).await;

    let eq2 = conn.session.trangbi.iter().find(|i| i.slot == 11).unwrap();
    assert_eq!(eq2.id, 5002, "new weapon equipped");
    let swapped = conn.session.homdo.iter().find(|i| i.slot == 2).unwrap();
    assert_eq!(swapped.id, 5001, "old weapon returned to homdo slot 2");

    // 3. Level gate test: try to equip high level item to pet
    conn.session.homdo.push(InventoryItem {
        slot: 3,
        id: 5099,
        count: 1,
        loai: 1,
        lv: 99, // pet is only level 20
        ..Default::default()
    });
    let payload_high_lv = [0x01, 0x03];
    let mut ctx_high = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_high_lv,
    };
    handle_inventory(&mut ctx_high).await;
    assert_eq!(conn.session.trangbi.iter().find(|i| i.slot == 11).unwrap().id, 5002, "not equipped due to level gate");

    // 4. Unequip pet weapon: sub 18, payload [pet_stt=1, equip_sub=1]
    let payload_unequip = [0x01, 0x01];
    let mut ctx_un = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 18,
        payload: &payload_unequip,
    };
    handle_inventory(&mut ctx_un).await;

    assert!(conn.session.trangbi.iter().all(|i| i.slot != 11), "pet gear unequipped from slot 11");
    let returned = conn.session.homdo.iter().find(|i| i.id == 5002);
    assert!(returned.is_some(), "weapon 5002 returned to homdo");

    // 5. Bag full gate: fill homdo to capacity (25) and try unequip
    // First re-equip 5002
    let slot = returned.unwrap().slot;
    let payload_reequip = [0x01, slot];
    let mut ctx_re = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_reequip,
    };
    handle_inventory(&mut ctx_re).await;

    // Fill all remaining homdo slots 1..=25
    conn.session.homdo.clear();
    for s in 1..=25 {
        conn.session.homdo.push(InventoryItem {
            slot: s,
            id: 9999,
            count: 1,
            ..Default::default()
        });
    }

    // Try unequip when bag is full
    let mut ctx_full = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 18,
        payload: &payload_unequip,
    };
    handle_inventory(&mut ctx_full).await;
    // Must remain equipped!
    assert!(conn.session.trangbi.iter().any(|i| i.slot == 11 && i.id == 5002), "item must remain equipped when bag is full");

    // 6. Reload from DB test
    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("reload");
    assert!(reloaded.trangbi.iter().any(|i| i.slot == 11 && i.id == 5002), "pet gear must persist across save/reload");
}

#[tokio::test]
async fn test_pet_equip_bounds_and_player_equip_persist() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "pet_guard", "pass_guard")
        .await
        .expect("create account");

    let seed = ts_dream::db::modern::traits::CharacterSeed {
        level: 50,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(acc_id, b"HeroGuard", &seed).await.expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        level: 50,
        pets: vec![
            PetState {
                stt: 1,
                id: 11001,
                level: 30,
                ..Default::default()
            },
            PetState {
                stt: 5, // in stable!
                id: 11002,
                level: 30,
                ..Default::default()
            },
        ],
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 5001,
                count: 1,
                loai: 1, // weapon
                lv: 15,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("save session");

    let mut conn = Conn::default();
    conn.session = session;
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    // 1. Try equip to stable pet (stt 5) -> must be rejected
    let payload_stt5 = [0x05, 0x01];
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_stt5,
    };
    handle_inventory(&mut ctx).await;
    assert!(conn.session.homdo.iter().any(|i| i.slot == 1), "Item must still be in homdo when target is stable pet");
    assert!(conn.session.trangbi.is_empty(), "No item equipped to stable pet");

    // 2. Try equip to invalid pet stt 0 -> must be rejected
    let payload_stt0 = [0x00, 0x01];
    let mut ctx0 = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_stt0,
    };
    handle_inventory(&mut ctx0).await;
    assert!(conn.session.homdo.iter().any(|i| i.slot == 1), "Item must still be in homdo");

    // 3. Equip to player (sub 11, payload [homdo_slot=1])
    let payload_player_equip = [0x01];
    let mut ctx_pe = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 11,
        payload: &payload_player_equip,
    };
    handle_inventory(&mut ctx_pe).await;
    assert_eq!(conn.session.trangbi.len(), 1);
    assert_eq!(conn.session.trangbi[0].slot, 1);
    assert!(conn.session.homdo.is_empty());

    // Verify DB: homdo slot 1 must be empty/0
    let reloaded_homdo = repos.sessions().load_items(acc_id, ts_dream::db::modern::model::StorageType::Bag).await.unwrap();
    assert!(reloaded_homdo.is_empty(), "homdo slot in DB must be cleared after equip");

    // 4. Unequip from player (sub 12, payload [trangbi_slot=1, homdo_slot=2])
    let payload_player_unequip = [0x01, 0x02];
    let mut ctx_pu = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 12,
        payload: &payload_player_unequip,
    };
    handle_inventory(&mut ctx_pu).await;
    assert!(conn.session.trangbi.is_empty());
    assert_eq!(conn.session.homdo.len(), 1);
    assert_eq!(conn.session.homdo[0].slot, 2);

    // Verify DB: trangbi must be completely empty, no phantom slot 2 in trangbi
    let reloaded_eq = repos.sessions().load_items(acc_id, ts_dream::db::modern::model::StorageType::Equip).await.unwrap();
    assert!(reloaded_eq.is_empty(), "trangbi must not contain unequipped item in DB");
}

#[test]
fn test_pet_stat_recompute_and_elemental_affinity() {
    let mut session = Session {
        level: 30,
        thuoctinh: 1, // Earth
        pets: vec![
            PetState {
                stt: 1,
                id: 11001,
                level: 20,
                thuoctinh: 3, // Fire
                hpx: 25,
                spx: 15,
                atk: 50,
                def: 40,
                int1: 30,
                agi: 20,
                ..Default::default()
            },
            PetState {
                stt: 2,
                id: 11002,
                level: 20,
                thuoctinh: 2, // Water
                hpx: 25,
                spx: 15,
                atk: 50,
                def: 40,
                int1: 30,
                agi: 20,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    // Slot 11: Fire weapon for Pet 1 (Fire element 3 matches item element 3)
    // atk1 = 20, atk2 = 5, thuoctinh = 3, giatri_thuoctinh = 10 -> each nonzero field gets +10!
    // atk2 bonus = (20 + 5) + 10 (for atk1) + 10 (for atk2) = 45!
    session.trangbi.push(InventoryItem {
        slot: 11,
        id: 5001,
        count: 1,
        atk1: 20,
        atk2: 5,
        thuoctinh: 3,
        giatri_thuoctinh: 10,
        ..Default::default()
    });

    // Slot 21: Same weapon type for Pet 2 (Water element 2 does NOT match item element 3)
    // atk2 bonus = 20 + 5 = 25 (no elemental affinity bonus)
    session.trangbi.push(InventoryItem {
        slot: 21,
        id: 5002,
        count: 1,
        atk1: 20,
        atk2: 5,
        thuoctinh: 3,
        giatri_thuoctinh: 10,
        ..Default::default()
    });

    session.recompute_stats();

    let pet1 = session.pets.iter().find(|p| p.stt == 1).unwrap();
    assert_eq!(pet1.atk2, 45, "Pet 1 (Fire) should receive elemental affinity bonus");

    let pet2 = session.pets.iter().find(|p| p.stt == 2).unwrap();
    assert_eq!(pet2.atk2, 25, "Pet 2 (Water) should NOT receive affinity bonus from Fire gear");

    // Player stats should remain 0 bonus
    assert_eq!(session.atk2, 0, "Player stats must remain unaffected by pet equipment");
}

#[test]
fn test_pet_battle_grid_includes_equipment_bonus() {
    use ts_dream::battle::construction::Battle;

    let mut battle = Battle::new(1001, 10817);
    let pet = PetState {
        stt: 1,
        id: 12001,
        level: 25,
        thuoctinh: 1,
        hp: 300,
        hp_max: 300,
        sp: 150,
        sp_max: 150,
        int1: 20,
        int2: 10,
        atk: 60,
        atk2: 35,
        def: 50,
        def2: 25,
        agi: 30,
        agi2: 15,
        ..Default::default()
    };

    battle.add_pet(&pet, 1001, 1001, 0, 0, 1);

    let cell = battle.cell(1, 0).expect("Pet cell should exist at row 1, col 0");
    assert_eq!(cell.typ, 4);
    assert_eq!(cell.int1, 20 + 10, "Battle cell INT must equal base + bonus");
    assert_eq!(cell.atk, 60 + 35, "Battle cell ATK must equal base + bonus");
    assert_eq!(cell.def, 50 + 25, "Battle cell DEF must equal base + bonus");
    assert_eq!(cell.agi, 30 + 15, "Battle cell AGI must equal base + bonus");
}

#[tokio::test]
async fn test_live_pet_equip_unequip_recomputes_session_stats() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "live_trainer", "pass123")
        .await
        .expect("create account");

    let seed = ts_dream::db::modern::traits::CharacterSeed {
        level: 10,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(acc_id, b"LiveTrainer", &seed).await.expect("create char");

    let mut session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"LiveTrainer".to_vec(),
        pets: vec![PetState {
            stt: 1,
            id: 11005,
            level: 30,
            thuoctinh: 1,
            hp: 200,
            hp_max: 200,
            sp: 100,
            sp_max: 100,
            atk: 40,
            ..Default::default()
        }],
        homdo: vec![InventoryItem {
            slot: 1,
            id: 6001,
            count: 1,
            loai: 1, // weapon
            lv: 20,
            atk1: 30,
            atk2: 10,
            ..Default::default()
        }],
        ..Default::default()
    };
    session.recompute_stats();
    assert_eq!(session.pets[0].atk2, 0, "Initial pet bonus should be 0");

    let mut conn = Conn::default();
    conn.session = session;
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    // Equip item to Pet 1
    let payload_equip = [0x01, 0x01];
    let mut ctx_eq = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 17,
        payload: &payload_equip,
    };
    handle_inventory(&mut ctx_eq).await;

    let pet_after_eq = conn.session.pets.iter().find(|p| p.stt == 1).unwrap();
    assert_eq!(pet_after_eq.atk2, 40, "Pet atk2 should be recomputed to 40 (30 + 10)");

    // Unequip item from Pet 1
    let payload_unequip = [0x01, 0x01, 0x01];
    let mut ctx_uneq = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 18,
        payload: &payload_unequip,
    };
    handle_inventory(&mut ctx_uneq).await;

    let pet_after_uneq = conn.session.pets.iter().find(|p| p.stt == 1).unwrap();
    assert_eq!(pet_after_uneq.atk2, 0, "Pet atk2 should reset back to 0 after unequip");
}

#[test]
fn test_pet_summary_wire_format_includes_10_byte_equipment() {
    let session = Session {
        id: 1,
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            name: b"Pikachu".to_vec(),
            level: 15,
            ..Default::default()
        }],
        trangbi: vec![InventoryItem {
            slot: 11, // pet 1 weapon
            id: 5001,
            count: 1,
            doben: 80,
            thuoctinh: 2,
            giatri_thuoctinh: 15,
            khang: 5,
            texp: 1234,
            ..Default::default()
        }],
        ..Default::default()
    };

    let summary_frames = ts_dream::server::spawn::pet_summary(&session);
    assert!(!summary_frames.is_empty());
    let stat_frame = &summary_frames[0];
    assert!(stat_frame.starts_with("F444"));
    assert!(stat_frame.contains("0F08"));

    // Verify item 5001 (0x1389 -> 8913 in LE) is encoded
    assert!(stat_frame.contains("8913"), "Pet equipment item ID must be present in 0F08 frame");
}

