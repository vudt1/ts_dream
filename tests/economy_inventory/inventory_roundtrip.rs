use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::server::auto_save::fingerprint;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::session::{Conn, InventoryItem, Session};

use crate::common::test_db::setup_test_db;

fn sample_item(slot: u8, id: u16, count: u8) -> InventoryItem {
    InventoryItem {
        slot,
        id,
        count,
        lv: 15,
        doben: 80,
        long_val: 3,
        giatri_long: 25,
        khang: 5,
        texp: 450,
        int1: 10,
        atk1: 20,
        def1: 15,
        hpx1: 50,
        spx1: 30,
        agi1: 12,
        fai1: 8,
        int2: 2,
        atk2: 4,
        def2: 3,
        hpx2: 10,
        spx2: 6,
        agi2: 2,
        fai2: 1,
        item_hp: 100,
        item_sp: 50,
        loai: 1,
        equip_pos: 1,
        thuoctinh: 2,
        giatri_thuoctinh: 5,
    }
}

#[tokio::test]
async fn test_full_inventory_and_reputation_roundtrip() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "user123", "pass123")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(acc_id, b"Hero", &seed).await.expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"Hero".to_vec(),
        level: 30,
        tiengtam: 1250, // reputation
        homdo: vec![sample_item(1, 1001, 10), sample_item(2, 1002, 1)],
        tientrang: vec![sample_item(1, 2001, 5)],
        tuideo: vec![sample_item(1, 3001, 2)],
        trangbi: vec![sample_item(1, 4001, 1)],
        luulang: vec![sample_item(1, 5001, 8)],
        ..Default::default()
    };

    // Save session (writes characters table including reputation and all 5 containers)
    repos.sessions().save(&session).await.expect("save session");

    // Load back into empty session
    let mut loaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    let found = repos.sessions().load(acc_id, &mut loaded).await.expect("load session");
    assert!(found, "session must be found");

    assert_eq!(loaded.tiengtam, 1250, "reputation must round-trip");
    assert_eq!(loaded.homdo.len(), 2);
    assert_eq!(loaded.tientrang.len(), 1);
    assert_eq!(loaded.tuideo.len(), 1);
    assert_eq!(loaded.trangbi.len(), 1);
    assert_eq!(loaded.luulang.len(), 1);

    // Verify all 8 instance fields on an item in homdo
    let item = &loaded.homdo[0];
    assert_eq!(item.slot, 1);
    assert_eq!(item.id, 1001);
    assert_eq!(item.count, 10);
    assert_eq!(item.doben, 80);
    assert_eq!(item.long_val, 3);
    assert_eq!(item.giatri_long, 25);
    assert_eq!(item.khang, 5);
    assert_eq!(item.texp, 450);

    // Verify bank item
    let bank_item = &loaded.tientrang[0];
    assert_eq!(bank_item.slot, 1);
    assert_eq!(bank_item.id, 2001);
    assert_eq!(bank_item.count, 5);

    // Verify tuideo item
    let tui_item = &loaded.tuideo[0];
    assert_eq!(tui_item.slot, 1);
    assert_eq!(tui_item.id, 3001);

    // Verify trangbi item
    let eq_item = &loaded.trangbi[0];
    assert_eq!(eq_item.slot, 1);
    assert_eq!(eq_item.id, 4001);

    // Verify luulang item
    let luu_item = &loaded.luulang[0];
    assert_eq!(luu_item.slot, 1);
    assert_eq!(luu_item.id, 5001);
}

#[tokio::test]
async fn test_move_stack_persistence_roundtrip() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "user_mv", "pass_mv")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(acc_id, b"Mover", &seed).await.expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"Mover".to_vec(),
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 10001,
                count: 1,
                loai: 1, // equipment
                ..Default::default()
            },
            InventoryItem {
                slot: 3,
                id: 20001,
                count: 10,
                loai: 0, // stackable
                ..Default::default()
            },
            InventoryItem {
                slot: 4,
                id: 20001,
                count: 15,
                loai: 0, // stackable
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("initial save");

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

    // Move equipment from slot 1 to empty slot 2 (opcode 0x17 sub 10 payload: [src=1, count=1, dst=2])
    let decoded = [0xF4, 0x44, 0x05, 0x00, 0x17, 0x0A, 0x01, 0x01, 0x02];
    let payload = [0x01, 0x01, 0x02];
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &decoded,
        opcode: 0x17,
        sub: 10,
        payload: &payload,
    };
    handle_inventory(&mut ctx).await;

    // Reload from DB into a fresh session — slot 1 must be gone, slot 2 must exist!
    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("reload");

    let s1 = reloaded.homdo.iter().find(|i| i.slot == 1);
    let s2 = reloaded.homdo.iter().find(|i| i.slot == 2);
    assert!(s1.is_none(), "slot 1 must be empty after move");
    assert!(s2.is_some(), "slot 2 must have the moved item");
    assert_eq!(s2.unwrap().id, 10001);

    // Now test merge: move all 10 items from slot 3 into slot 4 (count becomes 25, slot 3 emptied)
    let decoded_merge = [0xF4, 0x44, 0x05, 0x00, 0x17, 0x0A, 0x03, 0x0A, 0x04];
    let payload_merge = [0x03, 0x0A, 0x04];
    let mut ctx_merge = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &decoded_merge,
        opcode: 0x17,
        sub: 10,
        payload: &payload_merge,
    };
    handle_inventory(&mut ctx_merge).await;

    // Reload from DB again
    let mut reloaded2 = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded2).await.expect("reload after merge");

    let s3 = reloaded2.homdo.iter().find(|i| i.slot == 3);
    let s4 = reloaded2.homdo.iter().find(|i| i.slot == 4);
    assert!(s3.is_none(), "slot 3 must be empty after full merge");
    assert!(s4.is_some(), "slot 4 must contain merged items");
    assert_eq!(s4.unwrap().count, 25);
}

#[test]
fn test_fingerprint_mixes_all_5_containers() {
    let mut s1 = Session::default();
    s1.id = 100;
    let base_fp = fingerprint(&s1);

    // Mutate tientrang
    let mut s_bank = s1.clone();
    s_bank.tientrang.push(sample_item(1, 500, 1));
    assert_ne!(base_fp, fingerprint(&s_bank), "tientrang mutation must change fingerprint");

    // Mutate tuideo
    let mut s_tui = s1.clone();
    s_tui.tuideo.push(sample_item(1, 501, 1));
    assert_ne!(base_fp, fingerprint(&s_tui), "tuideo mutation must change fingerprint");

    // Mutate luulang
    let mut s_luu = s1.clone();
    s_luu.luulang.push(sample_item(1, 502, 1));
    assert_ne!(base_fp, fingerprint(&s_luu), "luulang mutation must change fingerprint");

    // Mutate giatri_long
    let mut s_long = s1.clone();
    let mut item = sample_item(1, 503, 1);
    item.giatri_long = 42;
    s_long.homdo.push(item);
    let fp_long1 = fingerprint(&s_long);
    s_long.homdo[0].giatri_long = 43;
    let fp_long2 = fingerprint(&s_long);
    assert_ne!(fp_long1, fp_long2, "giatri_long change must change fingerprint");
}

#[tokio::test]
async fn test_move_stack_partial_split_to_empty_slot() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "user_split", "pass_split")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(acc_id, b"Splitter", &seed)
        .await
        .expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"Splitter".to_vec(),
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 20001,
                count: 30,
                ..Default::default()
            },
            // Adjacent decoy items to guard against collateral corruption
            InventoryItem {
                slot: 2,
                id: 20002,
                count: 7,
                ..Default::default()
            },
            InventoryItem {
                slot: 5,
                id: 10001,
                count: 1,
                loai: 1,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("initial save");

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

    // Split 10 items from slot 1 into empty slot 3
    let decoded = [0xF4, 0x44, 0x05, 0x00, 0x17, 0x0A, 0x01, 0x0A, 0x03];
    let payload = [0x01, 0x0A, 0x03];
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &decoded,
        opcode: 0x17,
        sub: ts_dream::protocol::ITEM_SUB_MOVE_STACK,
        payload: &payload,
    };
    handle_inventory(&mut ctx).await;

    assert_eq!(out.outgoing.len(), 2, "must emit 17 0A echo and 17 05 dump");
    assert_eq!(out.outgoing[0], "F4440500170A010A03");
    assert!(
        out.outgoing[1].starts_with("F444") && out.outgoing[1].contains("1705"),
        "second frame must be 17 05 homdo dump, got: {}",
        out.outgoing[1].frame
    );

    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("reload");

    let s1 = reloaded.homdo.iter().find(|i| i.slot == 1).expect("slot 1 kept");
    let s2 = reloaded.homdo.iter().find(|i| i.slot == 2).expect("decoy slot 2 intact");
    let s3 = reloaded.homdo.iter().find(|i| i.slot == 3).expect("slot 3 created");
    let s5 = reloaded.homdo.iter().find(|i| i.slot == 5).expect("decoy slot 5 intact");
    assert_eq!(s1.id, 20001);
    assert_eq!(s1.count, 20);
    assert_eq!(s2.id, 20002);
    assert_eq!(s2.count, 7);
    assert_eq!(s3.id, 20001);
    assert_eq!(s3.count, 10);
    assert_eq!(s5.id, 10001);
    assert_eq!(s5.count, 1);
}

#[tokio::test]
async fn test_move_stack_partial_split_negative_rejects() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "user_split_neg", "pass_split_neg")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(acc_id, b"SplitNeg", &seed)
        .await
        .expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"SplitNeg".to_vec(),
        homdo: vec![
            InventoryItem {
                slot: 1,
                id: 20001,
                count: 30,
                ..Default::default()
            },
            // Different item id
            InventoryItem {
                slot: 2,
                id: 20002,
                count: 10,
                ..Default::default()
            },
            // Same item id, but 45 + 10 > 50
            InventoryItem {
                slot: 3,
                id: 20001,
                count: 45,
                ..Default::default()
            },
            // Same item id, but different dynamic attrs
            InventoryItem {
                slot: 4,
                id: 20001,
                count: 10,
                doben: 99,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("initial save");

    let mut conn = Conn::default();
    conn.session = session.clone();
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let invalid_cases: &[[u8; 3]] = &[
        [1, 10, 2], // partial split onto different item
        [1, 10, 3], // partial split exceeding STACK_CAP (45 + 10 > 50)
        [1, 10, 4], // partial split onto different dynamic attrs
        [1, 0, 5],  // zero count
        [1, 31, 5], // count > src_count
        [1, 10, 1], // oldslot == newslot
        [1, 10, 26], // newslot out of HOMDO_SLOTS
    ];

    for payload in invalid_cases {
        let mut out = HandleOutcome::default();
        let decoded = [
            0xF4, 0x44, 0x05, 0x00, 0x17, 0x0A, payload[0], payload[1], payload[2],
        ];
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            out: &mut out,
            env,
            data: &data,
            service: &service,
            decoded: &decoded,
            opcode: 0x17,
            sub: ts_dream::protocol::ITEM_SUB_MOVE_STACK,
            payload,
        };
        handle_inventory(&mut ctx).await;
        assert!(
            out.outgoing.is_empty(),
            "invalid move/split {:?} must emit no frames, got {:?}",
            payload,
            out.outgoing
        );
    }

    assert_eq!(conn.session.homdo.len(), 4);
    assert_eq!(conn.session.homdo[0].count, 30);

    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("reload");
    assert_eq!(reloaded.homdo.len(), 4);
    assert_eq!(reloaded.homdo.iter().find(|i| i.slot == 1).unwrap().count, 30);
    assert_eq!(reloaded.homdo.iter().find(|i| i.slot == 2).unwrap().count, 10);
    assert_eq!(reloaded.homdo.iter().find(|i| i.slot == 3).unwrap().count, 45);
    assert_eq!(reloaded.homdo.iter().find(|i| i.slot == 4).unwrap().count, 10);
}

#[tokio::test]
async fn test_sort_homdo_merges_and_sorts() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let acc_id = ts_dream::db::accounts::create(&pool, "user_sort", "pass_sort")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(acc_id, b"Sorter", &seed)
        .await
        .expect("create character");

    let session = Session {
        id: acc_id as u32,
        db_character_id: acc_id,
        name: b"Sorter".to_vec(),
        homdo: vec![
            InventoryItem {
                slot: 8,
                id: 20002,
                count: 30,
                ..Default::default()
            },
            InventoryItem {
                slot: 3,
                id: 20002,
                count: 35,
                ..Default::default()
            },
            // Same id as 20002, different dynamic attrs -> must NOT merge
            InventoryItem {
                slot: 5,
                id: 20002,
                count: 10,
                doben: 99,
                ..Default::default()
            },
            // Equipment (must NOT merge)
            InventoryItem {
                slot: 7,
                id: 10005,
                count: 1,
                loai: 1,
                ..Default::default()
            },
            InventoryItem {
                slot: 2,
                id: 10005,
                count: 1,
                loai: 1,
                ..Default::default()
            },
            InventoryItem {
                slot: 10,
                id: 20001,
                count: 5,
                ..Default::default()
            },
        ],
        // Decoy item in tientrang to ensure other containers are untouched
        tientrang: vec![InventoryItem {
            slot: 1,
            id: 30001,
            count: 12,
            ..Default::default()
        }],
        ..Default::default()
    };
    repos.sessions().save(&session).await.expect("initial save");

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

    let decoded = [0xF4, 0x44, 0x02, 0x00, 0x17, 0x3C];
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &decoded,
        opcode: 0x17,
        sub: ts_dream::protocol::ITEM_SUB_SORT,
        payload: &[],
    };
    handle_inventory(&mut ctx).await;

    assert_eq!(out.outgoing.len(), 1, "sort must emit 17 05 homdo dump");
    assert!(
        out.outgoing[0].starts_with("F444") && out.outgoing[0].contains("1705"),
        "expected 17 05 frame, got {}",
        out.outgoing[0].frame
    );

    let mut reloaded = Session {
        id: acc_id as u32,
        ..Default::default()
    };
    repos.sessions().load(acc_id, &mut reloaded).await.expect("reload");

    assert_eq!(reloaded.homdo.len(), 6);
    assert_eq!((reloaded.homdo[0].slot, reloaded.homdo[0].id, reloaded.homdo[0].count), (1, 10005, 1));
    assert_eq!((reloaded.homdo[1].slot, reloaded.homdo[1].id, reloaded.homdo[1].count), (2, 10005, 1));
    assert_eq!((reloaded.homdo[2].slot, reloaded.homdo[2].id, reloaded.homdo[2].count), (3, 20001, 5));
    assert_eq!(
        (reloaded.homdo[3].slot, reloaded.homdo[3].id, reloaded.homdo[3].count, reloaded.homdo[3].doben),
        (4, 20002, 50, 0)
    );
    assert_eq!(
        (reloaded.homdo[4].slot, reloaded.homdo[4].id, reloaded.homdo[4].count, reloaded.homdo[4].doben),
        (5, 20002, 15, 0)
    );
    assert_eq!(
        (reloaded.homdo[5].slot, reloaded.homdo[5].id, reloaded.homdo[5].count, reloaded.homdo[5].doben),
        (6, 20002, 10, 99)
    );

    // Other container untouched
    assert_eq!(reloaded.tientrang.len(), 1);
    assert_eq!(reloaded.tientrang[0].id, 30001);
    assert_eq!(reloaded.tientrang[0].count, 12);
}
