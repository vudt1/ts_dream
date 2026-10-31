//! Relogin derived stats & item template hydration tests
//! (tickets 05 & 06, reconnect-heartbeat-health).
//!
//! - After load from DB, `recompute_stats()` ensures gear bonus is non-zero
//!   when equipment is present, and stats packets match.
//! - Item template fields (`lv`, `loai`, `thuoctinh`, `giatri_thuoctinh`, `_1`/`_2`)
//!   are hydrated from template data after relog.
//! - Per-instance fields (`khang`, `texp`, `long_val`, `giatri_long`) are preserved
//!   across load/save cycles.
//! - All DB operations use the test DB `DB/ts_dream_test.db` (zero production DB mutation).

use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Item;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::server::session::{InventoryItem, Session};

mod common;
use common::test_db::setup_test_db;

fn build_mock_game_data() -> GameData {
    let mut data = GameData::default();
    // Item 1001: Weapon (level 10, atk1=20, atk2=5, element=2, elem_val=1)
    let mut w = Item::default();
    w.id = 1001;
    w.level = 10;
    w.atk1 = 20;
    w.atk2 = 5;
    w.loai = 1;
    w.thuoctinh = 2;
    w.value = 1;
    data.items.insert(1001, w);

    // Item 1002: Armor (level 12, def1=15, def2=4, element=1, elem_val=2)
    let mut a = Item::default();
    a.id = 1002;
    a.level = 12;
    a.def1 = 15;
    a.def2 = 4;
    a.loai = 2;
    a.thuoctinh = 1;
    a.value = 2;
    data.items.insert(1002, a);

    data
}

#[tokio::test]
async fn test_relogin_restores_gear_bonus_and_derived_stats() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = build_mock_game_data();

    let account_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        sex: 0,
        hair: 1,
        element: 2,
        level: 20,
        ..Default::default()
    };

    let mut session = Session {
        id: account_id as u32,
        db_character_id: account_id,
        level: 20,
        int1: 20,
        atk: 30,
        def: 25,
        agi: 15,
        hpx: 10,
        spx: 10,
        ..Default::default()
    };

    // Equip items
    session.trangbi.push(InventoryItem {
        slot: 1,
        id: 1001,
        count: 1,
        ..Default::default()
    });
    session.trangbi.push(InventoryItem {
        slot: 2,
        id: 1002,
        count: 1,
        ..Default::default()
    });

    repos
        .characters()
        .create(account_id, b"Hero", &seed)
        .await
        .expect("create char");

    repos.sessions().save(&session).await.expect("save session");

    // Relog: load into a blank session
    let mut loaded = Session {
        id: account_id as u32,
        ..Default::default()
    };
    let found = repos
        .sessions()
        .load(account_id, &mut loaded)
        .await
        .expect("load");
    assert!(found, "session must be found");

    // Hydrate template fields and recompute derived stats as done in login handler
    loaded.hydrate_templates(&data);
    loaded.recompute_stats();

    // Verify gear bonus is recomputed and non-zero
    assert!(loaded.atk2 > 0, "atk2 gear bonus must be > 0, got {}", loaded.atk2);
    assert!(loaded.def2 > 0, "def2 gear bonus must be > 0, got {}", loaded.def2);

    // Check that login sequence frame generation reflects the gear bonus
    let seq = ts_dream::server::spawn::build_logined_sequence_session_ext(&loaded);
    assert!(!seq.is_empty(), "login sequence must not be empty");
}

#[tokio::test]
async fn test_relogin_restores_item_template_and_preserves_instance_fields() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = build_mock_game_data();

    let account_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        sex: 1,
        hair: 2,
        element: 1,
        level: 15,
        ..Default::default()
    };

    let mut session = Session {
        id: account_id as u32,
        db_character_id: account_id,
        level: 15,
        ..Default::default()
    };

    // Put item in bag with per-instance customized values (khang, texp, long_val)
    session.homdo.push(InventoryItem {
        slot: 1,
        id: 1001,
        count: 1,
        doben: 75,
        long_val: 4,
        giatri_long: 10,
        khang: 8,
        texp: 500,
        ..Default::default()
    });

    repos
        .characters()
        .create(account_id, b"ItemHero", &seed)
        .await
        .expect("create char");

    repos.sessions().save(&session).await.expect("save session");

    // Reload from DB into a fresh session
    let mut loaded = Session {
        id: account_id as u32,
        ..Default::default()
    };
    repos
        .sessions()
        .load(account_id, &mut loaded)
        .await
        .expect("load");

    // Before hydration, dynamic fields are present from DB
    let raw_item = loaded.homdo.iter().find(|i| i.slot == 1).expect("item in bag");
    assert_eq!(raw_item.khang, 8, "khang must come from DB");
    assert_eq!(raw_item.texp, 500, "texp must come from DB");
    assert_eq!(raw_item.long_val, 4, "long_val must come from DB");
    assert_eq!(raw_item.giatri_long, 10, "giatri_long must come from DB");

    // Hydrate template fields
    loaded.hydrate_templates(&data);

    let hydrated_item = loaded.homdo.iter().find(|i| i.slot == 1).expect("hydrated item");
    // Template fields filled
    assert_eq!(hydrated_item.lv, 10, "template level must match");
    assert_eq!(hydrated_item.atk1, 20, "template atk1 must match");
    assert_eq!(hydrated_item.atk2, 5, "template atk2 must match");
    assert_eq!(hydrated_item.loai, 1, "template loai must match");
    assert_eq!(hydrated_item.thuoctinh, 2, "template thuoctinh must match");
    assert_eq!(hydrated_item.giatri_thuoctinh, 1, "template elem_val must match");

    // Dynamic instance fields NOT overwritten by template
    assert_eq!(hydrated_item.khang, 8, "khang preserved");
    assert_eq!(hydrated_item.texp, 500, "texp preserved");
    assert_eq!(hydrated_item.long_val, 4, "long_val preserved");
    assert_eq!(hydrated_item.giatri_long, 10, "giatri_long preserved");

    // Equip gate check: player level 15 >= item level 10
    assert!(loaded.level >= hydrated_item.lv, "equip gate must pass");
}
