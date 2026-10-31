use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::server::auto_save::fingerprint;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::session::{Conn, InventoryItem, Session};

mod common;
use common::test_db::setup_test_db;

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
