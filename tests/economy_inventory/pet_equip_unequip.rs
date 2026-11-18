use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::session::{Conn, InventoryItem, PetState, Session};

use crate::common::test_db::setup_test_db;

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

#[tokio::test]
async fn test_player_and_pet_pickaxe_equip_swap_unequip_and_rejections() {
    use ts_dream::data::tables::Item;

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let acc_id = ts_dream::db::accounts::create(&pool, "pick_user", "pick_pass")
        .await
        .expect("create account");

    let seed = ts_dream::db::modern::traits::CharacterSeed {
        level: 110,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(acc_id, b"Miner", &seed)
        .await
        .expect("create character");

    let mut data = ts_dream::data::loader::GameData::default();
    let pickaxes = [
        Item {
            id: 10001,
            loai: 39,
            equip_pos: 3,
            level: 0,
            atk1: 5,
            ..Default::default()
        },
        Item {
            id: 10530,
            loai: 39,
            equip_pos: 3,
            level: 90,
            atk1: 15,
            ..Default::default()
        },
        Item {
            id: 10534,
            loai: 39,
            equip_pos: 3,
            level: 110,
            atk1: 30,
            ..Default::default()
        },
        Item {
            id: 29335, // Cuốc đào (non-equippable)
            loai: 27,
            equip_pos: 0,
            level: 0,
            ..Default::default()
        },
    ];
    for item in pickaxes {
        data.items.insert(item.id, item);
    }

    let mut session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"Miner".to_vec(),
        level: 110,
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            name: b"Golem".to_vec(),
            level: 50, // enough for 10001 (lv0), not enough for 10530 (lv90)
            ..Default::default()
        }],
        homdo: vec![
            InventoryItem { slot: 1, id: 10001, count: 1, ..Default::default() },
            InventoryItem { slot: 2, id: 10534, count: 1, ..Default::default() },
            InventoryItem { slot: 3, id: 10530, count: 1, ..Default::default() },
            InventoryItem { slot: 4, id: 29335, count: 1, ..Default::default() },
            InventoryItem { slot: 5, id: 32012, count: 4, ..Default::default() }, // collateral
        ],
        trangbi: vec![
            InventoryItem { slot: 2, id: 19737, count: 1, loai: 2, agi1: 1, ..Default::default() }, // collateral
        ],
        ..Default::default()
    };
    session.hydrate_templates(&data);
    repos.sessions().save(&session).await.expect("save session");

    let mut conn = Conn::default();
    conn.session = session;
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    // 1. Player equips 10001 (slot 1) -> trangbi slot 3, emits 17 11 and 17 0B, atk2 += 5
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 11, // player equip
            payload: &[1],
        };
        handle_inventory(&mut ctx).await;
    }
    assert!(out.outgoing.iter().any(|f| f.frame == "F4440300171101"));
    assert!(out.outgoing.iter().any(|f| f.frame.len() >= 12 && &f.frame[8..12] == "170B"));
    assert_eq!(conn.session.trangbi.iter().find(|i| i.slot == 3).map(|i| i.id), Some(10001));
    assert_eq!(conn.session.atk2, 5);

    // 2. Player swaps 10001 with 10534 (slot 2) -> trangbi slot 3 becomes 10534, 10001 returns to homdo slot 2
    out.outgoing.clear();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 11,
            payload: &[2],
        };
        handle_inventory(&mut ctx).await;
    }
    assert_eq!(conn.session.trangbi.iter().find(|i| i.slot == 3).map(|i| i.id), Some(10534));
    assert_eq!(conn.session.atk2, 30);
    assert!(conn.session.homdo.iter().any(|i| i.slot == 2 && i.id == 10001));

    // 3. Negative: Player tries to equip 29335 (slot 4, equip_pos=0) -> rejected, item stays in homdo slot 4
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 11,
            payload: &[4],
        };
        handle_inventory(&mut ctx).await;
    }
    assert_eq!(conn.session.trangbi.iter().find(|i| i.slot == 3).map(|i| i.id), Some(10534));
    assert!(conn.session.homdo.iter().any(|i| i.slot == 4 && i.id == 29335));

    // 4. Negative: Pet (lv 50) tries to equip 10530 (lv 90, slot 3) -> rejected
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 17, // pet equip
            payload: &[1, 3],
        };
        handle_inventory(&mut ctx).await;
    }
    assert!(conn.session.trangbi.iter().all(|i| i.slot != 13));
    assert!(conn.session.homdo.iter().any(|i| i.slot == 3 && i.id == 10530));

    // 5. Pet equips 10001 (slot 2) -> trangbi slot 13 (1*10 + 3)
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 17,
            payload: &[1, 2],
        };
        handle_inventory(&mut ctx).await;
    }
    assert_eq!(conn.session.trangbi.iter().find(|i| i.slot == 13).map(|i| i.id), Some(10001));
    assert!(conn.session.homdo.iter().all(|i| i.slot != 2));

    // 6. Negative: Pet equip & unequip blocked when battle_id > 0
    conn.session.battle_id = 42;
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 18, // pet unequip
            payload: &[1, 3],
        };
        handle_inventory(&mut ctx).await;
    }
    assert_eq!(
        conn.session.trangbi.iter().find(|i| i.slot == 13).map(|i| i.id),
        Some(10001),
        "pet unequip must be blocked during battle"
    );
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 17, // pet equip
            payload: &[1, 3],
        };
        handle_inventory(&mut ctx).await;
    }
    assert_eq!(
        conn.session.trangbi.iter().find(|i| i.slot == 13).map(|i| i.id),
        Some(10001),
        "pet equip must be blocked during battle"
    );

    // 7. Out of battle: pet unequip & player unequip succeed
    conn.session.battle_id = 0;
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 18, // pet unequip
            payload: &[1, 3],
        };
        handle_inventory(&mut ctx).await;
    }
    assert!(conn.session.trangbi.iter().all(|i| i.slot != 13));
    assert!(conn.session.homdo.iter().any(|i| i.id == 10001));

    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: 0x17,
            sub: 12, // player unequip
            payload: &[3, 6],
        };
        handle_inventory(&mut ctx).await;
    }
    assert!(conn.session.trangbi.iter().all(|i| i.slot != 3));
    assert_eq!(conn.session.atk2, 0);
    assert!(conn.session.homdo.iter().any(|i| i.slot == 6 && i.id == 10534));

    // 8. Collateral integrity: homdo slot 5 (32012x4) and trangbi slot 2 (19737) untouched
    assert!(conn.session.homdo.iter().any(|i| i.slot == 5 && i.id == 32012 && i.count == 4));
    assert!(conn.session.trangbi.iter().any(|i| i.slot == 2 && i.id == 19737));
}

#[tokio::test]
async fn test_party_member_pickaxe_equip_syncs_online_sessions_for_battle() {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use ts_dream::battle::runner::{BattleAction, BattleCommand};
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loaders::{EveFightData, EveFightEnemy};
    use ts_dream::data::tables::{Item, Npc};
    use ts_dream::protocol::{
        encoder, ITEM_SUB_EQUIP, ITEM_SUB_PET_EQUIP, ITEM_SUB_PET_UNEQUIP, ITEM_SUB_UNEQUIP,
        OP_BATTLE, OP_ITEM,
    };
    use ts_dream::server::session::{
        lock_online_sessions, registry_get, registry_register_for_login,
    };

    let mut data = ts_dream::data::loader::GameData::default();
    let pickaxes = [
        Item {
            id: 10001,
            loai: 39,
            equip_pos: 3,
            level: 0,
            atk1: 25,
            ..Default::default()
        },
        Item {
            id: 10534,
            loai: 39,
            equip_pos: 3,
            level: 50,
            atk1: 30,
            ..Default::default()
        },
    ];
    for item in pickaxes {
        data.items.insert(item.id, item);
    }
    data.npcs.insert(
        25002,
        Npc {
            id: 25002,
            kind: 16,
            hp: 20,
            sp: 50,
            lv: 5,
            def: 1,
            agi: 1,
            ..Default::default()
        },
    );
    let data = Arc::new(data);

    let leader_id = 88_101u32;
    let member_id = 88_102u32;

    let leader_sess = Arc::new(RwLock::new(Session {
        id: leader_id,
        map_id: 10817,
        level: 50,
        hp: 500,
        hp_max: 500,
        sp: 200,
        sp_max: 200,
        agi: 50,
        id_leader: leader_id,
        id_mem: [member_id, 0, 0, 0],
        ..Default::default()
    }));

    let mut member_initial = Session {
        id: member_id,
        map_id: 10817,
        level: 50,
        hp: 500,
        hp_max: 500,
        sp: 200,
        sp_max: 200,
        atk: 120,
        agi: 200,
        id_leader: leader_id,
        active_pet_stt: 1,
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            name: b"Golem".to_vec(),
            level: 50,
            hp: 500,
            hp_max: 500,
            sp: 100,
            sp_max: 100,
            atk: 100,
            agi: 150,
            ..Default::default()
        }],
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 10001,
                count: 1,
                ..Default::default()
            },
            InventoryItem {
                slot: 2,
                id: 10001,
                count: 1,
                ..Default::default()
            },
            InventoryItem {
                slot: 3,
                id: 10534,
                count: 1,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    member_initial.hydrate_templates(&data);

    // Register initial sessions before equipping (simulates logged-in party).
    {
        let leader_guard = leader_sess.read().await;
        registry_register_for_login(&leader_guard);
    }
    registry_register_for_login(&member_initial);

    let mut member_conn = Conn::default();
    member_conn.session = member_initial;

    let mut service = BattleService::new(Arc::clone(&data));
    service.set_input_timeout(std::time::Duration::from_millis(60));
    let mut rx_leader = service.register(i64::from(leader_id), Arc::clone(&leader_sess));
    let member_sess = Arc::new(RwLock::new(member_conn.session.clone()));
    let _rx_member = service.register(i64::from(member_id), Arc::clone(&member_sess));

    let env = ServerEnv::none();
    let mut out = HandleOutcome::default();

    // Party member equips pickaxe for self (slot 1) and pet 1 (slot 2), then swaps self to 10534 (slot 3).
    for (sub, payload) in [
        (ITEM_SUB_EQUIP, &[1u8][..]),
        (ITEM_SUB_PET_EQUIP, &[1u8, 2u8][..]),
        (ITEM_SUB_EQUIP, &[3u8][..]),
    ] {
        let mut ctx = OpcodeCtx {
            conn: &mut member_conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: OP_ITEM,
            sub,
            payload,
        };
        handle_inventory(&mut ctx).await;
    }

    // 1. Registry must immediately reflect swapped player pickaxe and pet pickaxe.
    let synced = registry_get(member_id).expect("member must stay in online_sessions");
    assert_eq!(
        synced.trangbi.iter().find(|i| i.slot == 3).map(|i| i.id),
        Some(10534),
        "player equip/swap must sync trangbi slot 3 to online_sessions"
    );
    assert_eq!(
        synced.trangbi.iter().find(|i| i.slot == 13).map(|i| i.id),
        Some(10001),
        "pet equip must sync trangbi slot 13 to online_sessions"
    );

    // 2. Start party encounter against Mineral NPC 25002 (20 HP).
    // Leader defends (no pickaxe), member attacks with newly equipped pickaxe!
    let fight = EveFightData {
        eve_no: 901,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 25002,
            location_pos: 2,
            ai: 0,
        }],
        right_enemies: Vec::new(),
        bk_music: 0,
    };

    let battle_id = {
        let mut guard = leader_sess.write().await;
        service.start_encounter_battle(&mut guard, &fight, 112)
    };
    assert!(battle_id > 0);

    // Leader at (3,2) defends; member at (3,1) normal-attacks Mineral NPC at (0,2).
    let leader_snap = leader_sess.read().await.clone();
    assert!(service.submit_command(
        &leader_snap,
        BattleCommand {
            row: 3,
            col: 2,
            target_row: 3,
            target_col: 2,
            action: BattleAction::Defend,
        },
    ));
    let member_snap = member_sess.read().await.clone();
    assert!(service.submit_command(
        &member_snap,
        BattleCommand {
            row: 3,
            col: 1,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    ));
    assert!(service.submit_command(
        &member_snap,
        BattleCommand {
            row: 2,
            col: 1,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    ));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        if leader_sess.read().await.battle_id == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() <= deadline,
            "party mineral battle did not finish (pickaxe weapon_id failed to sync?)"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    // Verify battle-end frame for leader is emitted once member's synced pickaxe kills Mineral NPC.
    let mut frames = Vec::new();
    while let Ok(f) = rx_leader.try_recv() {
        frames.push(f);
    }
    let battle_end_prefix = format!("{:02X}00{}", OP_BATTLE, encoder::le32(leader_id));
    assert!(
        frames.iter().any(|f| f.contains(&battle_end_prefix)),
        "battle must conclude with finish frame once member's synced pickaxe kills Mineral NPC"
    );

    // 3. Unequip player & pet gear -> must also sync empty trangbi back to online_sessions.
    member_conn.session.battle_id = 0;
    for (sub, payload) in [
        (ITEM_SUB_UNEQUIP, &[3u8, 1u8][..]),
        (ITEM_SUB_PET_UNEQUIP, &[1u8, 3u8, 2u8][..]),
    ] {
        let mut ctx = OpcodeCtx {
            conn: &mut member_conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: OP_ITEM,
            sub,
            payload,
        };
        handle_inventory(&mut ctx).await;
    }
    let synced_after = registry_get(member_id).expect("member still registered");
    assert!(
        synced_after.trangbi.iter().all(|i| i.slot != 3 && i.slot != 13),
        "unequip must sync removed gear to online_sessions"
    );

    let mut online = lock_online_sessions();
    online.remove(&leader_id);
    online.remove(&member_id);
}

#[tokio::test]
async fn test_equip_unequip_rejections_emit_sys_msg_frame() {
    use ts_dream::protocol::{
        ITEM_SUB_EQUIP, ITEM_SUB_PET_EQUIP, ITEM_SUB_PET_UNEQUIP, ITEM_SUB_UNEQUIP, OP_ITEM,
    };
    use ts_dream::server::spawn::sys_msg_frame;

    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv::none();

    let mut conn = Conn::default();
    conn.session = Session {
        id: 77_001,
        level: 20,
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            level: 20,
            ..Default::default()
        }],
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 29335, // non-equippable (loai 27, equip_pos 0)
                count: 1,
                loai: 27,
                ..Default::default()
            },
            InventoryItem {
                slot: 2,
                id: 10530, // pickaxe lv 90 > level 20
                count: 1,
                loai: 39,
                equip_pos: 3,
                lv: 90,
                ..Default::default()
            },
        ],
        trangbi: vec![
            InventoryItem {
                slot: 3,
                id: 10001,
                count: 1,
                loai: 39,
                equip_pos: 3,
                ..Default::default()
            },
            InventoryItem {
                slot: 13,
                id: 10001,
                count: 1,
                loai: 39,
                equip_pos: 3,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let cases: &[(u8, Vec<u8>, i32, &str)] = &[
        // 1. Player equip non-equippable item
        (ITEM_SUB_EQUIP, vec![1], 0, "Vật phẩm này không thể trang bị!"),
        // 2. Pet equip insufficient level
        (ITEM_SUB_PET_EQUIP, vec![1, 2], 0, "Cấp độ võ tướng chưa đủ để trang bị!"),
        // 3. Player unequip to occupied homdo slot 1
        (ITEM_SUB_UNEQUIP, vec![3, 1], 0, "Ô túi đồ đích đã có vật phẩm!"),
        // 4. Pet equip blocked in battle
        (ITEM_SUB_PET_EQUIP, vec![1, 2], 99, "Không thể thay đổi trang bị khi đang trong trận đấu!"),
        // 5. Pet unequip blocked in battle
        (ITEM_SUB_PET_UNEQUIP, vec![1, 3], 99, "Không thể thay đổi trang bị khi đang trong trận đấu!"),
        // 6. Player equip blocked in battle
        (ITEM_SUB_EQUIP, vec![2], 99, "Không thể thay đổi trang bị khi đang trong trận đấu!"),
        // 7. Player unequip blocked in battle
        (ITEM_SUB_UNEQUIP, vec![3, 5], 99, "Không thể thay đổi trang bị khi đang trong trận đấu!"),
    ];

    for (sub, payload, battle_id, expected_msg) in cases {
        conn.session.battle_id = *battle_id;
        let mut out = HandleOutcome::default();
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &[],
            opcode: OP_ITEM,
            sub: *sub,
            payload,
        };
        handle_inventory(&mut ctx).await;
        assert_eq!(
            out.outgoing.iter().map(|f| f.frame.as_str()).collect::<Vec<_>>(),
            vec![sys_msg_frame(expected_msg)],
            "sub {sub} (battle_id={battle_id}) must emit expected sys_msg_frame"
        );
    }

    // 8. Pet unequip when homdo is full (all 25 slots occupied)
    conn.session.battle_id = 0;
    conn.session.homdo.clear();
    for s in 1..=25u8 {
        conn.session.homdo.push(InventoryItem {
            slot: s,
            id: 32012,
            count: 1,
            ..Default::default()
        });
    }
    let mut out_full = HandleOutcome::default();
    let mut ctx_full = OpcodeCtx {
        conn: &mut conn,
        out: &mut out_full,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: OP_ITEM,
        sub: ITEM_SUB_PET_UNEQUIP,
        payload: &[1, 3],
    };
    handle_inventory(&mut ctx_full).await;
    assert_eq!(
        out_full.outgoing.iter().map(|f| f.frame.as_str()).collect::<Vec<_>>(),
        vec![sys_msg_frame("Túi đồ đã đầy!")],
        "pet unequip on full bag must emit sys_msg_frame"
    );
}

