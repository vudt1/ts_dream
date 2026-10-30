use std::path::Path;
use ts_dream::data::loaders::compound::CompoundDatLoader;
use ts_dream::data::loaders::item::ItemDatLoader;

#[test]
fn test_compound_dat_load_and_index() {
    let path = Path::new("Data/Compound.Dat");
    assert!(path.exists(), "Data/Compound.Dat must exist");
    let bytes = std::fs::read(path).expect("read Compound.Dat");

    // Total file size is 11,152 bytes (1,394 slots of 8 bytes)
    assert_eq!(bytes.len(), 11152);

    let recipes = CompoundDatLoader::load(&bytes).expect("load Compound.Dat");

    // Header (slot 0) must be skipped, leaving exactly 1,393 valid recipes
    assert_eq!(recipes.len(), 1393);

    // Verify all keys are sorted ascending
    for i in 1..recipes.len() {
        assert!(
            recipes[i].lookup_key >= recipes[i - 1].lookup_key,
            "recipes must be sorted by lookup_key ascending: index {} has key {} < previous {}",
            i,
            recipes[i].lookup_key,
            recipes[i - 1].lookup_key
        );
    }

    // Verify known recipes from CLIENT_COMPOUND_SPEC.md:
    // Slot 1 (raw offset 0x08): result_kind 1, mats [1, 1, 0] -> sorted [0, 1, 1], deg [0, 1, 1], key 11
    let r1 = recipes.iter().find(|r| r.lookup_key == 11 && r.result_kind == 1);
    assert!(r1.is_some(), "recipe with key 11 and result_kind 1 must exist");
    let r1 = r1.unwrap();
    assert_eq!(r1.materials, [0, 1, 1]);
    assert_eq!(r1.degrees, [0, 1, 1]);

    // Slot 2: result_kind 1, mats [1, 2, 0] -> sorted [0, 1, 2], deg [0, 1, 1], key 12
    let r2 = recipes.iter().find(|r| r.lookup_key == 12 && r.result_kind == 1);
    assert!(r2.is_some(), "recipe with key 12 and result_kind 1 must exist");
    let r2 = r2.unwrap();
    assert_eq!(r2.materials, [0, 1, 2]);
    assert_eq!(r2.degrees, [0, 1, 1]);

    // Check recipe with key 363636
    let r36 = recipes.iter().find(|r| r.lookup_key == 363636);
    assert!(r36.is_some(), "recipe with key 363636 must exist");
    let r36 = r36.unwrap();
    assert_eq!(r36.materials, [36, 36, 36]);
    assert_eq!(r36.degrees, [1, 1, 1]);

    // Max recipe key is 373737
    let last = recipes.last().unwrap();
    assert_eq!(last.lookup_key, 373737);
    assert_eq!(last.materials, [37, 37, 37]);
}

#[test]
fn test_item_dat_compound_material_and_rank() {
    let path = Path::new("Data/Item.dat");
    assert!(path.exists(), "Data/Item.dat must exist");
    let bytes = std::fs::read(path).expect("read Item.dat");

    let items = ItemDatLoader::load(&bytes).expect("load Item.dat");
    assert!(!items.is_empty());

    // Verify items have compound_material and compound_rank parsed
    let mut has_material = false;
    let mut has_rank = false;

    for item in items.values() {
        if item.compound_material > 0 {
            has_material = true;
        }
        if item.compound_rank > 0 {
            has_rank = true;
        }
    }

    assert!(has_material, "At least some items must have compound_material > 0");
    assert!(has_rank, "At least some items must have compound_rank > 0");
}

#[test]
fn test_bankers_rounding() {
    use ts_dream::crafting::basic::delphi_round;

    assert_eq!(delphi_round(2.5), 2); // 2.5 rounds to even: 2
    assert_eq!(delphi_round(3.5), 4); // 3.5 rounds to even: 4
    assert_eq!(delphi_round(1.5), 2); // 1.5 rounds to even: 2
    assert_eq!(delphi_round(2.1), 2);
    assert_eq!(delphi_round(2.9), 3);
}

#[test]
fn test_delphi_rng_determinism() {
    use ts_dream::crafting::basic::DelphiRng;

    let mut rng1 = DelphiRng::new(12345);
    let mut rng2 = DelphiRng::new(12345);

    for _ in 0..10 {
        assert_eq!(rng1.random_int(100), rng2.random_int(100));
    }
}

#[test]
fn test_validator_rules() {
    use ts_dream::crafting::basic::validator::{validate_compound_inputs, CraftInputSlot};
    use ts_dream::crafting::error::CraftError;
    use ts_dream::data::loader::GameData;

    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");

    // Case 1: < 2 items
    let inputs1 = [
        CraftInputSlot::new(1, 10001, 1),
        CraftInputSlot::new(2, 0, 0),
        CraftInputSlot::new(3, 0, 0),
    ];
    assert_eq!(
        validate_compound_inputs(&inputs1, &game_data),
        Err(CraftError::NeedAtLeastTwoItems)
    );

    // Case 2: Qty > 50
    let inputs2 = [
        CraftInputSlot::new(1, 10001, 51),
        CraftInputSlot::new(2, 10002, 1),
        CraftInputSlot::new(3, 0, 0),
    ];
    assert_eq!(
        validate_compound_inputs(&inputs2, &game_data),
        Err(CraftError::QuantityExceeded)
    );

    // Case 3: Out of ID bounds
    let inputs3 = [
        CraftInputSlot::new(1, 5000, 1),
        CraftInputSlot::new(2, 10002, 1),
        CraftInputSlot::new(3, 0, 0),
    ];
    assert_eq!(
        validate_compound_inputs(&inputs3, &game_data),
        Err(CraftError::InvalidItemForCompound)
    );
}

#[test]
fn test_execute_compound_with_real_game_data() {
    use ts_dream::crafting::basic::{execute_compound, CraftInputSlot, DelphiRng};
    use ts_dream::data::loader::GameData;

    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");

    // Find two valid mineral items with compound_material > 0 deterministically
    let mineral_1 = game_data
        .item_defs
        .values()
        .filter(|i| i.id >= 10001 && i.id <= 50000 && i.compound_material > 0 && i.compound_rank > 0 && i.kind >= 15)
        .min_by_key(|i| i.id)
        .expect("must find a valid mineral");

    let mineral_2 = game_data
        .item_defs
        .values()
        .filter(|i| i.id > mineral_1.id && i.id <= 50000 && i.compound_material > 0 && i.compound_rank > 0 && i.kind >= 15)
        .min_by_key(|i| i.id)
        .expect("must find another mineral");

    let inputs = [
        CraftInputSlot::new(1, mineral_1.id, 5),
        CraftInputSlot::new(2, mineral_2.id, 5),
        CraftInputSlot::new(3, 0, 0),
    ];

    let mut rng = DelphiRng::new(42);
    let outcome = execute_compound(&inputs, &game_data, &mut rng)
        .expect("execute compound should succeed");

    assert!(outcome.item_id >= 10001);
    assert!(outcome.qty > 0);
    // If output is weapon/armor (kind 1..14), qty must be 1. Otherwise min_qty (5).
    if (1..=14).contains(&outcome.chosen_kind) {
        assert_eq!(outcome.qty, 1);
    } else {
        assert_eq!(outcome.qty, 5);
    }
}

#[test]
fn test_compound_fallback_when_unmatched() {
    use ts_dream::crafting::basic::{execute_compound, CraftInputSlot, DelphiRng};
    use ts_dream::data::loader::GameData;

    let data_dir = Path::new("Data");
    let mut game_data = GameData::load(data_dir).expect("load GameData");

    // Empty the recipe table to guarantee recipe lookup fails and triggers fallback
    game_data.compounds.clear();

    // Use two items that exist with valid id 10001..=50000
    let item1 = game_data
        .item_defs
        .values()
        .find(|i| i.id >= 10001 && i.id <= 50000 && (1..=14).contains(&i.kind))
        .unwrap();
    let item2 = game_data
        .item_defs
        .values()
        .find(|i| i.id != item1.id && i.id >= 10001 && i.id <= 50000 && (1..=14).contains(&i.kind))
        .unwrap();

    let inputs = [
        CraftInputSlot::new(1, item1.id, 1),
        CraftInputSlot::new(2, item2.id, 1),
        CraftInputSlot::new(3, 0, 0),
    ];

    let mut rng = DelphiRng::new(999);
    let outcome = execute_compound(&inputs, &game_data, &mut rng)
        .expect("fallback compound should succeed without losing items");

    // Fallback forces target_rank = 1
    assert_eq!(outcome.target_rank, 1);
    // Output must be rank 1 item
    let result_item = game_data.item_defs.get(&outcome.item_id).expect("result item must exist");
    assert_eq!(result_item.compound_rank, 1);
}

// =========================================================================
// Integration Test Helper: In-memory DB bootstrap
// =========================================================================
async fn setup_test_db() -> ts_dream::db::pool::DbPool {
    let pool = ts_dream::db::pool::bootstrap("sqlite::memory:?cache=shared", None)
        .await
        .expect("bootstrap in-memory SQLite");
    let schema = include_str!("../migrations/0001_init.sql");
    sqlx::raw_sql(schema)
        .execute(&pool.write)
        .await
        .expect("execute 0001_init.sql migration");
    pool
}

// =========================================================================
// Test Ticket 05: Feather Socketing Pure Logic Engine
// =========================================================================
#[test]
fn test_socketing_engine_pure_logic() {
    use ts_dream::crafting::error::CraftError;
    use ts_dream::crafting::socketing::feather::{parse_feather, socket_feather};
    use ts_dream::server::session::InventoryItem;

    // 1. parse_feather
    let f1 = parse_feather(24021).expect("parse earth feather");
    assert_eq!(f1.element, 1);
    assert_eq!(f1.level, 1);
    let f2 = parse_feather(24022).expect("parse water feather");
    assert_eq!(f2.element, 2);
    let f_invalid = parse_feather(10001);
    assert_eq!(f_invalid, None);

    // 2. Blank equip initial socketing (element 0 -> element 1, level 1, 100% success)
    let mut equip = InventoryItem {
        id: 11001,
        slot: 1,
        count: 1,
        long_val: 0,
        giatri_long: 0,
        ..Default::default()
    };
    let outcome = socket_feather(&mut equip, 1, 24021, 99).expect("initial socketing should succeed");
    assert!(outcome.success);
    assert_eq!(outcome.new_element, 1);
    assert_eq!(outcome.new_element_value, 1);
    assert!(!outcome.is_element_switched);
    assert_eq!(equip.long_val, 1);
    assert_eq!(equip.giatri_long, 1);

    // 3. Same element upgrade: +1 to +2 with rate 100% (feather level 2: 24025, roll 99 < 100)
    let outcome2 = socket_feather(&mut equip, 1, 24025, 99).expect("upgrade +1 to +2");
    assert!(outcome2.success);
    assert_eq!(outcome2.new_element_value, 2);
    assert_eq!(equip.giatri_long, 2);

    // 4. Same element upgrade: +3 to +4 failure (feather level 4: 24034, rate 80%, roll 85 >= 80)
    equip.giatri_long = 3;
    let outcome_fail3 = socket_feather(&mut equip, 1, 24034, 85).expect("fail at +3");
    assert!(!outcome_fail3.success);
    // At level <= 3, failure does NOT reduce level: stays at 3
    assert_eq!(outcome_fail3.new_element_value, 3);
    assert_eq!(equip.giatri_long, 3);

    // 5. Same element upgrade: +4 to +5 failure (feather level 5: 24035, rate 60%, roll 75 >= 60)
    equip.giatri_long = 4;
    let outcome_fail4 = socket_feather(&mut equip, 1, 24035, 75).expect("fail at +4");
    assert!(!outcome_fail4.success);
    // At level >= 4, failure drops 1 level: 4 -> 3
    assert_eq!(outcome_fail4.new_element_value, 3);
    assert_eq!(equip.giatri_long, 3);

    // 6. Element switch: equip is Earth (1) +3, khảm Water feather (24022)
    let outcome_switch = socket_feather(&mut equip, 1, 24022, 50).expect("element switch");
    assert!(outcome_switch.success);
    assert!(outcome_switch.is_element_switched);
    assert_eq!(outcome_switch.new_element, 2);
    assert_eq!(outcome_switch.new_element_value, 1);
    assert_eq!(equip.long_val, 2);
    assert_eq!(equip.giatri_long, 1);

    // 7. Invalid equip kind (e.g. kind 15 mineral)
    let res = socket_feather(&mut equip, 15, 24021, 0);
    assert_eq!(res, Err(CraftError::InvalidEquipmentType));
}

// =========================================================================
// Test Ticket 03: Compound Wire & DB Flow Integration Test
// =========================================================================
#[tokio::test]
async fn test_handle_compound_wire_and_db_flow() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::{handle_compound, SOUND_BLACKSMITH};
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    // Create account & character
    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("Craft_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    // Find two valid minerals with compound_material > 0 deterministically
    let mineral_1 = game_data
        .item_defs
        .values()
        .filter(|i| i.id >= 10001 && i.id <= 50000 && i.compound_material > 0 && i.compound_rank > 0 && i.kind >= 15)
        .min_by_key(|i| i.id)
        .unwrap();
    let mineral_2 = game_data
        .item_defs
        .values()
        .filter(|i| i.id > mineral_1.id && i.id <= 50000 && i.compound_material > 0 && i.compound_rank > 0 && i.kind >= 15)
        .min_by_key(|i| i.id)
        .unwrap();

    // Insert items into DB inventories (storagetype = 1)
    // Setup slots 1 & 2 for compound, and slots 3 & 4 as adjacent collateral test slots
    sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity) VALUES (?, 1, 1, ?, 5), (?, 1, 2, ?, 5), (?, 1, 3, 10003, 7), (?, 1, 4, 10004, 13)")
        .bind(char_id)
        .bind(mineral_1.id as i64)
        .bind(char_id)
        .bind(mineral_2.id as i64)
        .bind(char_id)
        .bind(char_id)
        .execute(&pool.write)
        .await
        .expect("insert items into inventories");

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: mineral_1.id,
        count: 5,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: mineral_2.id,
        count: 5,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 3,
        id: 10003,
        count: 7,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 4,
        id: 10004,
        count: 13,
        ..Default::default()
    });

    // Execute handle_compound: slot 1 (qty 2), slot 2 (qty 2), no slot 3
    let payload = vec![1, 2, 2, 2, 0, 0];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx).await;
    }

    // 1. Verify wire packets
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    // Sound blacksmith WA0014
    assert!(frames.iter().any(|f| f == SOUND_BLACKSMITH), "Must emit SFX WA0014");
    // Deduction for slot 1 (rem 3) and slot 2 (rem 3)
    assert!(frames.iter().any(|f| f == "F444040017090103"), "Must emit deduction for slot 1");
    assert!(frames.iter().any(|f| f == "F444040017090203"), "Must emit deduction for slot 2");
    // New item added frame (1706)
    assert!(frames.iter().any(|f| f.starts_with("F4440E001706")), "Must emit 1706 item added");
    // Toast sys msg (020102)
    assert!(frames.iter().any(|f| f.contains("020102")), "Must emit sys msg toast");

    // 2. Verify RAM session
    let item1 = conn.session.homdo.iter().find(|i| i.slot == 1).unwrap();
    assert_eq!(item1.count, 3);
    let item2 = conn.session.homdo.iter().find(|i| i.slot == 2).unwrap();
    assert_eq!(item2.count, 3);
    let item3 = conn.session.homdo.iter().find(|i| i.slot == 3).unwrap();
    assert_eq!(item3.id, 10003);
    assert_eq!(item3.count, 7);
    let item4 = conn.session.homdo.iter().find(|i| i.slot == 4).unwrap();
    assert_eq!(item4.id, 10004);
    assert_eq!(item4.count, 13);
    let created_item = conn.session.homdo.iter().find(|i| i.slot > 4).unwrap();
    assert_eq!(created_item.slot, 5);
    assert!(created_item.id >= 10001);
    assert!(created_item.count > 0);

    // 3. Verify SQLite DB state
    let row1: (i64,) = sqlx::query_as("SELECT quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 1")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(row1.0, 3);

    let row3: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 3")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(row3.0, 10003);
    assert_eq!(row3.1, 7);

    let row4: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 4")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(row4.0, 10004);
    assert_eq!(row4.1, 13);

    let row_created: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?")
        .bind(char_id)
        .bind(created_item.slot as i64)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(row_created.0 as u16, created_item.id);
    assert_eq!(row_created.1 as u8, created_item.count);

    // 4. Test bag full probe-first rejection (25 occupied slots)
    conn.session.homdo.clear();
    for slot in 1..=25 {
        conn.session.homdo.push(InventoryItem {
            slot,
            id: 10001,
            count: 1,
            ..Default::default()
        });
    }
    let mut out_full = HandleOutcome::default();
    {
        let mut ctx_full = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_full,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx_full).await;
    }
    let full_frames: Vec<String> = out_full.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(full_frames.iter().any(|f| f.contains("020102")), "Bag full must emit toast");
    assert!(!full_frames.iter().any(|f| f == SOUND_BLACKSMITH), "Bag full must not emit SFX");
}

// =========================================================================
// Test Ticket 04: Select Feather Wire & DB Flow Integration Test
// =========================================================================
#[tokio::test]
async fn test_handle_select_feather_wire_and_db_flow() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::{handle_select_feather, SOUND_BLACKSMITH};
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("Feather_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;

    // Case 1: Choice = 0 -> rejected
    let mut out0 = HandleOutcome::default();
    {
        let mut ctx0 = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out0,
            opcode: 0x23,
            sub: 0x03,
            payload: &[0, 0],
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx0).await;
    }
    let frames0: Vec<String> = out0.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames0.iter().any(|f| f.contains("020102")), "Choice 0 must emit error toast");
    assert!(conn.session.homdo.is_empty(), "No item added for choice 0");

    // Case 2: Choice 1 (Earth -> 24021)
    let mut out1 = HandleOutcome::default();
    {
        let mut ctx1 = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out1,
            opcode: 0x23,
            sub: 0x03,
            payload: &[1, 0],
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx1).await;
    }
    let frames1: Vec<String> = out1.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames1.iter().any(|f| f.starts_with("F4440E001706")), "Must emit 1706 item added");
    assert!(frames1.iter().any(|f| f == SOUND_BLACKSMITH), "Must emit SFX");
    assert_eq!(conn.session.homdo.len(), 1);
    assert_eq!(conn.session.homdo[0].id, 24021);
    assert_eq!(conn.session.homdo[0].slot, 1);

    // Verify DB persistence
    let db_item: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 1")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(db_item.0, 24021);
    assert_eq!(db_item.1, 1);

    // Case 3: Bag full rejection (25 occupied slots)
    conn.session.homdo.clear();
    for slot in 1..=25 {
        conn.session.homdo.push(InventoryItem {
            slot,
            id: 10001,
            count: 1,
            ..Default::default()
        });
    }
    let mut out_full = HandleOutcome::default();
    {
        let mut ctx_full = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_full,
            opcode: 0x23,
            sub: 0x03,
            payload: &[2, 0],
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx_full).await;
    }
    let frames_full: Vec<String> = out_full.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_full.iter().any(|f| f.contains("020102")), "Bag full must emit toast");
}

// =========================================================================
// Test Ticket 05: Feather Socketing Wire & DB Flow Integration Test
// =========================================================================
#[tokio::test]
async fn test_handle_socket_feather_wire_and_db_flow() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::{handle_socket_feather, SOUND_BLACKSMITH};
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("Socket_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    // Find a weapon or armor item (kind 1..=14)
    let equip_def = game_data
        .item_defs
        .values()
        .find(|i| (1..=14).contains(&i.kind) && i.id >= 10001 && i.id <= 50000)
        .unwrap();

    let feather_id = 24021; // Earth feather

    // Put weapon in slot 1, feather in slot 2, and collateral slots 3 and 4 in DB
    sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, element, elementvalue) VALUES (?, 1, 1, ?, 1, 0, 0), (?, 1, 2, ?, 1, 0, 0), (?, 1, 3, 10003, 5, 0, 0), (?, 1, 4, 10004, 9, 0, 0)")
        .bind(char_id)
        .bind(equip_def.id as i64)
        .bind(char_id)
        .bind(feather_id as i64)
        .bind(char_id)
        .bind(char_id)
        .execute(&pool.write)
        .await
        .expect("insert items");

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: equip_def.id,
        count: 1,
        long_val: 0,
        giatri_long: 0,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: feather_id,
        count: 1,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 3,
        id: 10003,
        count: 5,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 4,
        id: 10004,
        count: 9,
        ..Default::default()
    });

    // Dispatch handle_socket_feather with payload [1, 2]
    let payload = vec![1, 2];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 0x3A,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_socket_feather(&mut ctx).await;
    }

    // Verify wire packets
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f == SOUND_BLACKSMITH), "Must emit SFX WA0014");
    assert!(frames.iter().any(|f| f == "F444040017090200"), "Must deduct feather from slot 2");
    assert!(frames.iter().any(|f| f.starts_with("F4440E001706")), "Must emit updated item frame for slot 1");
    assert!(frames.iter().any(|f| f.contains("020102")), "Must emit sys msg toast");

    // Verify RAM session
    let equip_item = conn.session.homdo.iter().find(|i| i.slot == 1).unwrap();
    assert_eq!(equip_item.long_val, 1); // Earth element
    assert_eq!(equip_item.giatri_long, 1); // +1
    assert!(!conn.session.homdo.iter().any(|i| i.slot == 2), "Feather slot 2 must be removed");
    let item3 = conn.session.homdo.iter().find(|i| i.slot == 3).unwrap();
    assert_eq!(item3.id, 10003);
    assert_eq!(item3.count, 5);
    let item4 = conn.session.homdo.iter().find(|i| i.slot == 4).unwrap();
    assert_eq!(item4.id, 10004);
    assert_eq!(item4.count, 9);

    // Verify DB persistence
    let equip_db: (i64, i64) = sqlx::query_as("SELECT element, elementvalue FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 1")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(equip_db.0, 1);
    assert_eq!(equip_db.1, 1);

    let row3: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 3")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(row3.0, 10003);
    assert_eq!(row3.1, 5);

    let row4: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 4")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(row4.0, 10004);
    assert_eq!(row4.1, 9);

    let feather_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = 2")
        .bind(char_id)
        .fetch_one(&pool.read)
        .await
        .unwrap();
    assert_eq!(feather_count.0, 0);
}

#[test]
fn test_catalog_find_candidate_recipes_degree_isolation() {
    use ts_dream::crafting::basic::catalog::CompoundCatalog;
    use ts_dream::data::loaders::compound::CompoundDef;

    // Create a recipe with key 12, degrees [0, 1, 1], result_kind 100
    let recipes = vec![CompoundDef {
        lookup_key: 12,
        result_kind: 100,
        materials: [0, 1, 2],
        degrees: [0, 1, 1],
        ..Default::default()
    }];

    // Case A: Exact match [0, 1, 1]
    let cand_ok = CompoundCatalog::find_candidate_recipes(&recipes, 12, [0, 1, 1]);
    assert_eq!(cand_ok, vec![100]);

    // Case B: Slot 1 fails (input degree is 0, but recipe requires degree 1)
    let cand_fail1 = CompoundCatalog::find_candidate_recipes(&recipes, 12, [0, 0, 1]);
    assert!(cand_fail1.is_empty(), "Recipe must be rejected if slot 1 does not match");

    // Case C: Slot 2 fails (input degree is 0, but recipe requires degree 1)
    let cand_fail2 = CompoundCatalog::find_candidate_recipes(&recipes, 12, [0, 1, 0]);
    assert!(cand_fail2.is_empty(), "Recipe must be rejected if slot 2 does not match");
}

#[test]
fn test_parse_feather_all_levels_including_level_10() {
    use ts_dream::crafting::socketing::feather::parse_feather;

    // Level 8
    let f8_earth = parse_feather(24052).expect("parse earth C8");
    assert_eq!(f8_earth.element, 1);
    assert_eq!(f8_earth.level, 8);

    // Level 9
    let f9_earth = parse_feather(24056).expect("parse earth C9");
    assert_eq!(f9_earth.element, 1);
    assert_eq!(f9_earth.level, 9);
    let f9_univ = parse_feather(24064).expect("parse universal C9");
    assert_eq!(f9_univ.element, 0);
    assert_eq!(f9_univ.level, 9);

    // Level 10
    let f10_earth = parse_feather(24060).expect("parse earth C10");
    assert_eq!(f10_earth.element, 1);
    assert_eq!(f10_earth.level, 10);
    let f10_water = parse_feather(24061).expect("parse water C10");
    assert_eq!(f10_water.element, 2);
    assert_eq!(f10_water.level, 10);
    let f10_fire = parse_feather(24062).expect("parse fire C10");
    assert_eq!(f10_fire.element, 3);
    assert_eq!(f10_fire.level, 10);
    let f10_wind = parse_feather(24063).expect("parse wind C10");
    assert_eq!(f10_wind.element, 4);
    assert_eq!(f10_wind.level, 10);
    let f10_univ = parse_feather(24065).expect("parse universal C10");
    assert_eq!(f10_univ.element, 0);
    assert_eq!(f10_univ.level, 10);
}

#[test]
fn test_socketing_to_level_10_and_max_level_rejection() {
    use ts_dream::crafting::error::CraftError;
    use ts_dream::crafting::socketing::feather::socket_feather;
    use ts_dream::server::session::InventoryItem;

    // 1. Upgrade from +9 to +10 using Earth C10 (24060) with roll 2 (< 5% rate -> success)
    let mut equip = InventoryItem {
        id: 11001,
        slot: 1,
        count: 1,
        long_val: 1, // Earth
        giatri_long: 9, // +9
        ..Default::default()
    };
    let outcome10 = socket_feather(&mut equip, 1, 24060, 2).expect("upgrade to +10");
    assert!(outcome10.success);
    assert_eq!(outcome10.new_element_value, 10);
    assert_eq!(equip.giatri_long, 10);

    // 2. Already at +10: attempt same element upgrade -> EquipmentMaxLevel error
    let res_max = socket_feather(&mut equip, 1, 24060, 0);
    assert_eq!(res_max, Err(CraftError::EquipmentMaxLevel));

    // 3. Already at +10: attempt universal feather upgrade -> EquipmentMaxLevel error
    let res_univ = socket_feather(&mut equip, 1, 24065, 0);
    assert_eq!(res_univ, Err(CraftError::EquipmentMaxLevel));

    // 4. Already at +10: attempt element switch (e.g. Water C1 24022) -> resets to +1
    let switch_out = socket_feather(&mut equip, 1, 24022, 50).expect("element switch from +10");
    assert!(switch_out.success);
    assert!(switch_out.is_element_switched);
    assert_eq!(switch_out.new_element, 2); // Water
    assert_eq!(switch_out.new_element_value, 1);
    assert_eq!(equip.long_val, 2);
    assert_eq!(equip.giatri_long, 1);
}

#[tokio::test]
async fn test_handle_socket_feather_level_10_and_max_level_wire() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::handle_socket_feather;
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("Max_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    let equip_def = game_data
        .item_defs
        .values()
        .find(|i| (1..=14).contains(&i.kind) && i.id >= 10001 && i.id <= 50000)
        .unwrap();

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;
    // Equip already at +10
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: equip_def.id,
        count: 1,
        long_val: 1,
        giatri_long: 10,
        ..Default::default()
    });
    // Feather Earth C10 (24060)
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 24060,
        count: 1,
        ..Default::default()
    });

    let payload = vec![1, 2];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 0x3A,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_socket_feather(&mut ctx).await;
    }

    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    // Must emit max level toast
    assert!(frames.iter().any(|f| f.contains("020102")), "Must emit max level toast");
    // Feather must NOT be consumed on max level rejection
    assert_eq!(conn.session.homdo.len(), 2, "Feather must remain in bag");
}

#[tokio::test]
async fn test_compound_1_item_triggers_need_at_least_two_items_toast() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::handle_compound;
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let mut conn = Conn::default();
    conn.session.id = 999;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 5,
        ..Default::default()
    });

    let expected_toast = ts_dream::server::spawn::sys_msg_frame("Hợp thành cần ít nhất 2 vật phẩm!");

    // Test 1-item payload [slot1, qty1] (len = 2)
    let payload = vec![1, 1];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx).await;
    }
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f == &expected_toast), "Must emit 'Hợp thành cần ít nhất 2 vật phẩm!' for len=2");

    // Test 1-item payload with trailing zeroes [slot1, qty1, 0, 0]
    let payload2 = vec![1, 1, 0, 0];
    let mut out2 = HandleOutcome::default();
    {
        let mut ctx2 = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out2,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload2,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx2).await;
    }
    let frames2: Vec<String> = out2.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames2.iter().any(|f| f == &expected_toast), "Must emit 'Hợp thành cần ít nhất 2 vật phẩm!' for slot2=0");

    // Test empty payload [] (len = 0)
    let mut out_empty = HandleOutcome::default();
    {
        let mut ctx_empty = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_empty,
            opcode: 0x17,
            sub: 0x39,
            payload: &[],
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx_empty).await;
    }
    let frames_empty: Vec<String> = out_empty.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_empty.iter().any(|f| f == &expected_toast), "Must emit 'Hợp thành cần ít nhất 2 vật phẩm!' for empty payload");

    // Test 1-byte payload [1] (len = 1)
    let mut out_1b = HandleOutcome::default();
    {
        let mut ctx_1b = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_1b,
            opcode: 0x17,
            sub: 0x39,
            payload: &[1],
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx_1b).await;
    }
    let frames_1b: Vec<String> = out_1b.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_1b.iter().any(|f| f == &expected_toast), "Must emit 'Hợp thành cần ít nhất 2 vật phẩm!' for 1-byte payload");
}

#[test]
fn test_socket_blank_gear_with_high_level_feather_rejected() {
    use ts_dream::crafting::error::CraftError;
    use ts_dream::crafting::socketing::feather::socket_feather;
    use ts_dream::server::session::InventoryItem;

    let mut equip = InventoryItem {
        slot: 1,
        id: 11001,
        count: 1,
        long_val: 0,
        giatri_long: 0,
        ..Default::default()
    };

    // 1. Feather Level 2 on blank equipment (24025: Earth Level 2)
    let res_c2 = socket_feather(&mut equip, 1, 24025, 99);
    assert_eq!(res_c2, Err(CraftError::InvalidFeatherLevel));
    assert_eq!(equip.long_val, 0);
    assert_eq!(equip.giatri_long, 0);

    // 2. Feather Level 10 on blank equipment (24060: Earth Level 10)
    let res_c10 = socket_feather(&mut equip, 1, 24060, 99);
    assert_eq!(res_c10, Err(CraftError::InvalidFeatherLevel));
    assert_eq!(equip.long_val, 0);
    assert_eq!(equip.giatri_long, 0);

    // 3. Universal / Fairy Feather (24035: Dong Thoai Vu level 5, element = 0)
    let res_dong_thoai = socket_feather(&mut equip, 1, 24035, 99);
    assert_eq!(res_dong_thoai, Err(CraftError::InvalidFeather));
    assert_eq!(equip.long_val, 0);
    assert_eq!(equip.giatri_long, 0);

    // 4. Universal Feather Level 1 (24031: Dong Thoai Vu level 1, element = 0)
    let res_dt1 = socket_feather(&mut equip, 1, 24031, 99);
    assert_eq!(res_dt1, Err(CraftError::InvalidFeather));
    assert_eq!(equip.long_val, 0);
    assert_eq!(equip.giatri_long, 0);

    // 5. Valid: Level 1 elemental feather (24021: Earth Level 1) MUST succeed
    let res_ok = socket_feather(&mut equip, 1, 24021, 99);
    assert!(res_ok.is_ok());
    assert_eq!(equip.long_val, 1);
    assert_eq!(equip.giatri_long, 1);

    // 6. Element switch with Level 2 feather on equipment that already has element (must be rejected)
    let mut equip_water = InventoryItem {
        slot: 1,
        id: 11001,
        count: 1,
        long_val: 2, // Water
        giatri_long: 1,
        ..Default::default()
    };
    let res_switch_c2 = socket_feather(&mut equip_water, 1, 24025, 99); // Earth Level 2
    assert_eq!(res_switch_c2, Err(CraftError::InvalidFeatherLevel));
    assert_eq!(equip_water.long_val, 2);
    assert_eq!(equip_water.giatri_long, 1);
}

#[tokio::test]
async fn test_select_feather_wire_payload_variants() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::handle_select_feather;
    use ts_dream::server::session::Conn;

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("FeatherVar_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;

    // Variant 1: Short payload [3, 0] -> Fire feather 24023
    let mut out_short = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_short,
            opcode: 0x23,
            sub: 0x03,
            payload: &[3, 0],
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx).await;
    }
    assert_eq!(conn.session.homdo.len(), 1);
    assert_eq!(conn.session.homdo[0].id, 24023); // Fire feather
    assert_eq!(conn.session.homdo[0].slot, 1);

    // Clean up for Variant 2
    conn.session.homdo.clear();
    sqlx::query("DELETE FROM inventories WHERE playerid = ?")
        .bind(char_id)
        .execute(&pool.write)
        .await
        .unwrap();

    // Variant 2: Delphi form packet with 2 len-prefixed strings and trailing [3, 0]
    let mut delphi_payload = Vec::new();
    delphi_payload.push(11u8); // len 11
    delphi_payload.extend_from_slice(b"TFormSelect");
    delphi_payload.push(4u8);  // len 4
    delphi_payload.extend_from_slice(b"user");
    delphi_payload.push(3u8);  // elem_choice = Fire
    delphi_payload.push(0u8);  // sub_choice = 0

    let mut out_delphi = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_delphi,
            opcode: 0x23,
            sub: 0x03,
            payload: &delphi_payload,
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx).await;
    }
    assert_eq!(conn.session.homdo.len(), 1);
    assert_eq!(conn.session.homdo[0].id, 24023); // Fire feather
    assert_eq!(conn.session.homdo[0].slot, 1);

    // Variant 3: Invalid choice (choice = 5) -> must emit error toast
    let mut out_inv = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_inv,
            opcode: 0x23,
            sub: 0x03,
            payload: &[5, 0],
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx).await;
    }
    let frames_inv: Vec<String> = out_inv.outgoing.into_iter().map(|f| f.frame).collect();
    let expected_toast = ts_dream::server::spawn::sys_msg_frame("Lựa chọn bảo vật không hợp lệ!");
    assert!(frames_inv.iter().any(|f| f == &expected_toast));
}

#[tokio::test]
async fn test_collateral_slot_preservation_on_compound_and_socketing() {
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::{handle_compound, handle_socket_feather};
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("Collateral_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    let mineral_1 = game_data
        .item_defs
        .values()
        .filter(|i| i.id >= 10001 && i.id <= 50000 && i.compound_material > 0 && i.compound_rank > 0 && i.kind >= 15)
        .min_by_key(|i| i.id)
        .unwrap();
    let mineral_2 = game_data
        .item_defs
        .values()
        .filter(|i| i.id > mineral_1.id && i.id <= 50000 && i.compound_material > 0 && i.compound_rank > 0 && i.kind >= 15)
        .min_by_key(|i| i.id)
        .unwrap();
    let equip_def = game_data
        .item_defs
        .values()
        .find(|i| (1..=14).contains(&i.kind) && i.id >= 10001 && i.id <= 50000)
        .unwrap();

    // Setup collateral slots 3 (id 10003, qty 7), 4 (id 10004, qty 13), 5 (id 10005, qty 19)
    sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, element, elementvalue) VALUES (?, 1, 1, ?, 5, 0, 0), (?, 1, 2, ?, 5, 0, 0), (?, 1, 3, 10003, 7, 0, 0), (?, 1, 4, 10004, 13, 0, 0), (?, 1, 5, 10005, 19, 0, 0)")
        .bind(char_id)
        .bind(mineral_1.id as i64)
        .bind(char_id)
        .bind(mineral_2.id as i64)
        .bind(char_id)
        .bind(char_id)
        .bind(char_id)
        .execute(&pool.write)
        .await
        .expect("insert initial items");

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;
    conn.session.homdo.push(InventoryItem { slot: 1, id: mineral_1.id, count: 5, ..Default::default() });
    conn.session.homdo.push(InventoryItem { slot: 2, id: mineral_2.id, count: 5, ..Default::default() });
    conn.session.homdo.push(InventoryItem { slot: 3, id: 10003, count: 7, ..Default::default() });
    conn.session.homdo.push(InventoryItem { slot: 4, id: 10004, count: 13, ..Default::default() });
    conn.session.homdo.push(InventoryItem { slot: 5, id: 10005, count: 19, ..Default::default() });

    // 1. Run compound on slots 1 & 2
    let payload = vec![1, 2, 2, 2, 0, 0];
    let mut out = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx).await;
    }

    // Assert collateral slots 3, 4, 5 are intact in RAM
    for (s, expected_id, expected_qty) in [(3, 10003, 7), (4, 10004, 13), (5, 10005, 19)] {
        let it = conn.session.homdo.iter().find(|i| i.slot == s).unwrap();
        assert_eq!(it.id, expected_id, "Slot {} id mutated", s);
        assert_eq!(it.count, expected_qty, "Slot {} count mutated", s);
    }

    // Assert collateral slots 3, 4, 5 are intact in DB
    for (s, expected_id, expected_qty) in [(3, 10003, 7), (4, 10004, 13), (5, 10005, 19)] {
        let db_row: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?")
            .bind(char_id)
            .bind(s as i64)
            .fetch_one(&pool.read)
            .await
            .unwrap();
        assert_eq!(db_row.0, expected_id as i64, "DB Slot {} id mutated", s);
        assert_eq!(db_row.1, expected_qty as i64, "DB Slot {} quantity mutated", s);
    }

    // 2. Setup socketing: slot 1 is equip, slot 2 is Earth feather level 1 (24021)
    conn.session.homdo.retain(|i| i.slot != 1 && i.slot != 2);
    conn.session.homdo.push(InventoryItem { slot: 1, id: equip_def.id, count: 1, ..Default::default() });
    conn.session.homdo.push(InventoryItem { slot: 2, id: 24021, count: 1, ..Default::default() });

    sqlx::query("UPDATE inventories SET itemid = ?, quantity = 1, element = 0, elementvalue = 0 WHERE playerid = ? AND storagetype = 1 AND slot = 1")
        .bind(equip_def.id as i64)
        .bind(char_id)
        .execute(&pool.write)
        .await
        .unwrap();
    sqlx::query("UPDATE inventories SET itemid = 24021, quantity = 1, element = 0, elementvalue = 0 WHERE playerid = ? AND storagetype = 1 AND slot = 2")
        .bind(char_id)
        .execute(&pool.write)
        .await
        .unwrap();

    let payload_sock = vec![1, 2];
    let mut out_sock = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_sock,
            opcode: 0x17,
            sub: 0x3A,
            payload: &payload_sock,
            decoded: &[],
            env,
        };
        handle_socket_feather(&mut ctx).await;
    }

    // Assert collateral slots 3, 4, 5 are STILL intact in RAM after socketing
    for (s, expected_id, expected_qty) in [(3, 10003, 7), (4, 10004, 13), (5, 10005, 19)] {
        let it = conn.session.homdo.iter().find(|i| i.slot == s).unwrap();
        assert_eq!(it.id, expected_id, "Slot {} id mutated after socketing", s);
        assert_eq!(it.count, expected_qty, "Slot {} count mutated after socketing", s);
    }

    // Assert collateral slots 3, 4, 5 are STILL intact in DB after socketing
    for (s, expected_id, expected_qty) in [(3, 10003, 7), (4, 10004, 13), (5, 10005, 19)] {
        let db_row: (i64, i64) = sqlx::query_as("SELECT itemid, quantity FROM inventories WHERE playerid = ? AND storagetype = 1 AND slot = ?")
            .bind(char_id)
            .bind(s as i64)
            .fetch_one(&pool.read)
            .await
            .unwrap();
        assert_eq!(db_row.0, expected_id as i64, "DB Slot {} id mutated after socketing", s);
        assert_eq!(db_row.1, expected_qty as i64, "DB Slot {} quantity mutated after socketing", s);
    }
}

#[tokio::test]
async fn test_compound_with_arbitrary_box_positions() {
    use std::path::Path;
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::handle_compound;
    use ts_dream::server::session::{Conn, InventoryItem};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();

    let mineral_1 = game_data.item_defs.values().filter(|i| (23..=37).contains(&i.kind)).min_by_key(|i| i.id).unwrap();
    let mineral_2 = game_data.item_defs.values().filter(|i| (23..=37).contains(&i.kind) && i.id != mineral_1.id).min_by_key(|i| i.id).unwrap();

    let acc_id = ts_dream::db::accounts::create(&pool, "pass1", "pass2")
        .await
        .expect("create account");
    let char_id = acc_id;
    sqlx::query("INSERT INTO characters (playerid, name) VALUES (?, ?)")
        .bind(char_id)
        .bind(format!("BoxPos_{}", char_id).as_bytes())
        .execute(&pool.write)
        .await
        .expect("insert character");

    let mut conn = Conn::default();
    conn.session.id = char_id as u32;
    conn.session.db_character_id = char_id;
    conn.session.homdo.push(InventoryItem { slot: 1, id: mineral_1.id, count: 5, ..Default::default() });
    conn.session.homdo.push(InventoryItem { slot: 2, id: mineral_2.id, count: 5, ..Default::default() });

    // 1. Box 1 & Box 3 filled, Box 2 is empty: payload = [1, 2, 0, 0, 2, 2]
    let payload_1_3 = vec![1, 2, 0, 0, 2, 2];
    let mut out_1_3 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_1_3,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload_1_3,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx).await;
    }
    let frames: Vec<String> = out_1_3.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f.starts_with("F4440200170F")), "WA0014 sound must be emitted for Box 1 & 3");
    assert!(frames.iter().any(|f| f.starts_with("F4440E001706")), "1706 must be emitted for Box 1 & 3");

    // 2. Box 2 & Box 3 filled, Box 1 is empty: payload = [0, 0, 1, 1, 2, 1]
    let payload_2_3 = vec![0, 0, 1, 1, 2, 1];
    let mut out_2_3 = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_2_3,
            opcode: 0x17,
            sub: 0x39,
            payload: &payload_2_3,
            decoded: &[],
            env,
        };
        handle_compound(&mut ctx).await;
    }
    let frames_2_3: Vec<String> = out_2_3.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_2_3.iter().any(|f| f.starts_with("F4440200170F")), "WA0014 sound must be emitted for Box 2 & 3");
    assert!(frames_2_3.iter().any(|f| f.starts_with("F4440E001706")), "1706 must be emitted for Box 2 & 3");
}

#[tokio::test]
async fn test_socket_and_select_feather_short_payload_toast() {
    use std::path::Path;
    use ts_dream::battle::service::BattleService;
    use ts_dream::data::loader::GameData;
    use ts_dream::db::modern::sqlite::SqliteRepositories;
    use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
    use ts_dream::server::handlers::crafting::{handle_select_feather, handle_socket_feather};
    use ts_dream::server::session::Conn;
    use ts_dream::server::spawn::sys_msg_frame;

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };
    let data_dir = Path::new("Data");
    let game_data = GameData::load(data_dir).expect("load GameData");
    let service = BattleService::default();
    let mut conn = Conn::default();

    // 1. handle_socket_feather with empty payload
    let mut out_sock_empty = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_sock_empty,
            opcode: 0x17,
            sub: 0x3A,
            payload: &[],
            decoded: &[],
            env,
        };
        handle_socket_feather(&mut ctx).await;
    }
    let expected_sock_toast = sys_msg_frame("Vị trí trang bị hoặc lông không hợp lệ!");
    let frames: Vec<String> = out_sock_empty.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames.iter().any(|f| f == &expected_sock_toast));

    // 2. handle_socket_feather with 1-byte payload
    let mut out_sock_1b = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_sock_1b,
            opcode: 0x17,
            sub: 0x3A,
            payload: &[1],
            decoded: &[],
            env,
        };
        handle_socket_feather(&mut ctx).await;
    }
    let frames_1b: Vec<String> = out_sock_1b.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_1b.iter().any(|f| f == &expected_sock_toast));

    // 3. handle_select_feather with empty payload
    let mut out_sel_empty = HandleOutcome::default();
    {
        let mut ctx = OpcodeCtx {
            conn: &mut conn,
            data: &game_data,
            service: &service,
            out: &mut out_sel_empty,
            opcode: 0x23,
            sub: 0x03,
            payload: &[],
            decoded: &[],
            env,
        };
        handle_select_feather(&mut ctx).await;
    }
    let expected_sel_toast = sys_msg_frame("Hãy chọn lựa một trong các loại bảo vật");
    let frames_sel: Vec<String> = out_sel_empty.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(frames_sel.iter().any(|f| f == &expected_sel_toast));
}



