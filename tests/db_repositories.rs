//! Ticket 06 — modern schema migration & repository layer tests.
//!
//! Pure unit tests run everywhere. The CRUD/transaction tests need a live
//! MySQL 8 instance; they activate when `TS_TEST_DB_URL` is set, e.g.:
//!
//! ```text
//! TS_TEST_DB_URL=mysql://root:root@localhost:3306/ts_dream_test cargo test --test db_repositories
//! ```
//!
//! The target database must already exist; migrations from `migrations/`
//! are applied to it on first connect. Every test cleans up after itself.

use sqlx::{MySqlPool, Row};
use ts_dream::db::modern::model::{
    InventorySlot, MissionRow, Money, PetRecord, PetSkill, PetStorageType, SkillRow,
    StorageType,
};
use ts_dream::db::modern::mysql::MySqlRepositories;
use ts_dream::db::modern::traits::{
    AccountRepository, CharacterRepository, CharacterSeed, InventoryRepository, PetRepository,
    QuestRepository,
};
use ts_dream::db::modern::transactions::{bank_transfer, p2p_trade, shop_buy, TxError};
use ts_dream::protocol::codecs::thing_data::ThingData;

// ---------------------------------------------------------------------------
// Pure unit tests (no database)
// ---------------------------------------------------------------------------

#[test]
fn storage_type_values_match_schema() {
    assert_eq!(StorageType::Bag.value(), 1);
    assert_eq!(StorageType::Secondary.value(), 2);
    assert_eq!(StorageType::Bank.value(), 4);
    assert_eq!(StorageType::Equip.value(), 8);
    assert_eq!(StorageType::Warehouse.value(), 16);
}

#[test]
fn storage_type_roundtrip_and_rejects_unknown() {
    for st in [
        StorageType::Bag,
        StorageType::Secondary,
        StorageType::Bank,
        StorageType::Equip,
        StorageType::Warehouse,
    ] {
        assert_eq!(StorageType::from_value(st.value()), Some(st));
    }
    assert_eq!(StorageType::from_value(0), None);
    assert_eq!(StorageType::from_value(3), None);
    assert_eq!(StorageType::from_value(32), None);
}

#[test]
fn pet_storage_type_roundtrip_and_rejects_unknown() {
    for st in [
        PetStorageType::Carried,
        PetStorageType::Cart,
        PetStorageType::Hotel,
        PetStorageType::Warehouse,
    ] {
        assert_eq!(PetStorageType::from_value(st.value()), Some(st));
    }
    assert_eq!(PetStorageType::from_value(5), None);
}

#[test]
fn empty_inventory_slot_carries_zero_thing_data() {
    let s = InventorySlot::empty(StorageType::Bag, 7);
    assert_eq!(s.slot, 7);
    assert!(s.is_empty());
    assert_eq!(s.item.to_bytes(), [0u8; 35]);
}

// ---------------------------------------------------------------------------
// Database-gated integration tests
// ---------------------------------------------------------------------------

/// Connects to the test database, or `None` when `TS_TEST_DB_URL` is unset /
/// unreachable so the suite degrades to the unit tests only.
async fn test_pool() -> Option<MySqlPool> {
    let url = std::env::var("TS_TEST_DB_URL").ok()?;
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(4)
            .connect(&url),
    )
    .await
    {
        Ok(Ok(pool)) => {
            let _ = sqlx::migrate!("./migrations").run(&pool).await;
            Some(pool)
        }
        _ => {
            eprintln!("TS_TEST_DB_URL set but unreachable; skipping DB tests");
            None
        }
    }
}

fn sample_item(item_id: u16, slot: u16) -> InventorySlot {
    InventorySlot {
        storage_type: StorageType::Bag,
        slot,
        item: ThingData {
            item_id,
            quantity: 50,
            damage: 10,
            element: 3,
            element_value: 45,
            proof_kind: 2,
            grow_level: 5,
            grow_exp: 12_000,
            special_kind: 1,
            stone_attr: 4,
            stone_level: 7,
            enhance_level: 10,
            delete_time: 44_927.5,
            damaged_item_id: 23_000,
            is_locked: true,
            reinforced: 3,
            affix1: 12,
            affix2: 15,
            affix3: 18,
            style_level: 6,
        },
    }
}

fn seed(level: i64) -> CharacterSeed {
    CharacterSeed {
        level,
        sex: 1,
        hair: 2,
        element: 3,
        map_id: 10_801,
        map_x: 400,
        map_y: 500,
    }
}

/// Unique per-test account id so parallel runs never collide.
fn fresh_account_id() -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static NEXT: AtomicI64 = AtomicI64::new(900_000_000);
    NEXT.fetch_add(1, Ordering::SeqCst) + std::process::id() as i64 * 1000
}

#[tokio::test]
async fn character_crud_and_money_ledger() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_id = fresh_account_id();

    // accounts comes from 0001; seed one row for the FK-less relation.
    sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'abc', 'def')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();

    let char_id = repos
        .characters()
        .create(account_id, b"TESTCHAR", &seed(10))
        .await
        .unwrap();

    // Byte-exact credential checks (latin1_bin / HEX comparison).
    assert!(repos.accounts().verify_pass1(account_id, b"abc").await.unwrap());
    assert!(!repos.accounts().verify_pass1(account_id, b"ABC").await.unwrap());
    assert!(repos.accounts().verify_pass2(account_id, b"def").await.unwrap());

    let listed = repos.characters().list_by_account(account_id).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, b"TESTCHAR");
    assert_eq!(listed[0].level, 10);

    assert_eq!(
        repos.characters().find_id_by_name(b"TESTCHAR").await.unwrap(),
        Some(char_id)
    );

    // Fresh ledger starts zeroed; direct update then read back.
    let money = repos.characters().load_money(char_id).await.unwrap();
    assert_eq!(money, Money::default());
    sqlx::query("UPDATE character_money SET gold = 777 WHERE character_id = ?")
        .bind(char_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(repos.characters().load_money(char_id).await.unwrap().gold, 777);

    repos.characters().delete(char_id).await.unwrap();
    assert_eq!(
        repos.characters().find_id_by_name(b"TESTCHAR").await.unwrap(),
        None
    );
    sqlx::query("DELETE FROM accounts WHERE player_id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn inventory_slot_full_field_roundtrip() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_id = fresh_account_id();
    sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'x', 'y')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let char_id = repos
        .characters()
        .create(account_id, b"INVTEST", &seed(1))
        .await
        .unwrap();

    let item = sample_item(23_145, 3);
    repos.inventories().save_slot(char_id, &item, &pool).await.unwrap();

    // Upsert overwrites every column instead of inserting a duplicate row.
    repos.inventories().save_slot(char_id, &item, &pool).await.unwrap();

    let loaded = repos
        .inventories()
        .load_storage(char_id, StorageType::Bag)
        .await
        .unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0], item);
    // Byte-for-byte parity with the wire codec.
    assert_eq!(loaded[0].item.to_bytes(), sample_item(23_145, 3).item.to_bytes());

    let single = repos
        .inventories()
        .load_slot(char_id, StorageType::Bag, 3)
        .await
        .unwrap();
    assert_eq!(single, Some(item.clone()));

    repos
        .inventories()
        .clear_slot(char_id, StorageType::Bag, 3, &pool)
        .await
        .unwrap();
    assert!(repos
        .inventories()
        .load_storage(char_id, StorageType::Bag)
        .await
        .unwrap()
        .is_empty());
    // Clearing again is a no-op, not an error.
    repos
        .inventories()
        .clear_slot(char_id, StorageType::Bag, 3, &pool)
        .await
        .unwrap();

    repos.characters().delete(char_id).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE player_id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn pet_four_storages_roundtrip() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_id = fresh_account_id();
    sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'x', 'y')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let char_id = repos
        .characters()
        .create(account_id, b"PETTEST", &seed(1))
        .await
        .unwrap();

    let pet = |slot: u16| PetRecord {
        slot,
        pet_id: 15_001,
        name: b"THIENLONG".to_vec(),
        level: 42,
        element: 3,
        reborn: 1,
        hp: 900,
        hp_max: 1000,
        sp: 90,
        sp_max: 100,
        int_attr: 11,
        atk: 22,
        def: 33,
        hpx: 44,
        spx: 55,
        agi: 66,
        fai: 77,
        texp: 88,
        skill_point: 5,
        thd: 9,
        skills: [
            PetSkill { id: 101, level: 3 },
            PetSkill { id: 102, level: 4 },
            PetSkill { id: 103, level: 5 },
            PetSkill { id: 104, level: 6 },
        ],
        quest: 7,
    };

    for (i, st) in [
        PetStorageType::Carried,
        PetStorageType::Cart,
        PetStorageType::Hotel,
        PetStorageType::Warehouse,
    ]
    .into_iter()
    .enumerate()
    {
        repos.pets().save_pet(char_id, st, &pet(i as u16)).await.unwrap();
    }

    for (i, st) in [
        PetStorageType::Carried,
        PetStorageType::Cart,
        PetStorageType::Hotel,
        PetStorageType::Warehouse,
    ]
    .into_iter()
    .enumerate()
    {
        let loaded = repos.pets().load_storage(char_id, st).await.unwrap();
        assert_eq!(loaded.len(), 1, "storage {st:?}");
        assert_eq!(loaded[0], pet(i as u16));
    }

    repos.pets().delete_pet(char_id, PetStorageType::Cart, 1).await.unwrap();
    assert!(repos
        .pets()
        .load_storage(char_id, PetStorageType::Cart)
        .await
        .unwrap()
        .is_empty());

    repos.characters().delete(char_id).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE player_id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn missions_bit_flags_completed_events() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_id = fresh_account_id();
    sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'x', 'y')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let char_id = repos
        .characters()
        .create(account_id, b"QSTTEST", &seed(1))
        .await
        .unwrap();

    repos
        .quests()
        .upsert_mission(
            char_id,
            &MissionRow { mission_id: 501, step: 1, state: 0, updated_at: 100 },
        )
        .await
        .unwrap();
    repos
        .quests()
        .upsert_mission(
            char_id,
            &MissionRow { mission_id: 501, step: 2, state: 1, updated_at: 200 },
        )
        .await
        .unwrap();

    let missions = repos.quests().list_missions(char_id).await.unwrap();
    assert_eq!(missions.len(), 1);
    assert_eq!(missions[0].step, 2);
    assert_eq!(missions[0].state, 1);

    repos.quests().set_bit_flag(char_id, 17, 999).await.unwrap();
    repos.quests().set_bit_flag(char_id, 17, 1234).await.unwrap(); // idempotent
    assert!(repos.quests().has_bit_flag(char_id, 17).await.unwrap());
    assert!(!repos.quests().has_bit_flag(char_id, 18).await.unwrap());
    let set_at: i64 = sqlx::query_scalar(
        "SELECT set_at FROM character_bit_flags WHERE character_id = ? AND flag_index = 17",
    )
    .bind(char_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(set_at, 999, "first latch wins forever");

    repos.quests().mark_event_completed(char_id, 601, 555).await.unwrap();
    assert!(repos.quests().has_completed_event(char_id, 601).await.unwrap());
    assert!(!repos.quests().has_completed_event(char_id, 602).await.unwrap());

    // replace_skills inside an explicit transaction.
    let mut tx = pool.begin().await.unwrap();
    repos
        .quests()
        .replace_skills(
            char_id,
            &[
                SkillRow { skill_id: 10_001, level: 3, sp: 1, save_flag: 0 },
                SkillRow { skill_id: 10_002, level: 7, sp: 2, save_flag: 1 },
            ],
            &mut tx,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM character_skills WHERE character_id = ?")
            .bind(char_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);

    repos.characters().delete(char_id).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE player_id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn bank_transfer_atomic_with_overdraft_guard() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_id = fresh_account_id();
    sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'x', 'y')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let char_id = repos
        .characters()
        .create(account_id, b"BANKTEST", &seed(1))
        .await
        .unwrap();

    // Deposit 500 when holding nothing -> rejected, nothing changes.
    match bank_transfer(&pool, char_id, 500).await {
        Err(TxError::InsufficientFunds) => {}
        other => panic!("expected InsufficientFunds, got {other:?}"),
    }

    // Seed gold, deposit, withdraw partially, over-withdraw rejected.
    sqlx::query("UPDATE character_money SET gold = 1000 WHERE character_id = ?")
        .bind(char_id)
        .execute(&pool)
        .await
        .unwrap();

    let money = bank_transfer(&pool, char_id, 400).await.unwrap();
    assert_eq!(
        money,
        Money { gold: 600, bank_gold: 400, shop_point: 0 }
    );

    let money = bank_transfer(&pool, char_id, -150).await.unwrap();
    assert_eq!(money.gold, 750);
    assert_eq!(money.bank_gold, 250);

    match bank_transfer(&pool, char_id, -300).await {
        Err(TxError::InsufficientFunds) => {}
        other => panic!("expected InsufficientFunds on overdraft, got {other:?}"),
    }
    // Balance untouched by the failed attempt.
    let money = repos.characters().load_money(char_id).await.unwrap();
    assert_eq!(money.gold, 750);
    assert_eq!(money.bank_gold, 250);

    repos.characters().delete(char_id).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE player_id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn shop_buy_grants_item_only_when_gold_covers_price() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_id = fresh_account_id();
    sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'x', 'y')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let char_id = repos
        .characters()
        .create(account_id, b"SHOPTEST", &seed(1))
        .await
        .unwrap();

    let purchase = sample_item(30_001, 0);

    // Too poor: no gold leaves, no item appears.
    match shop_buy(&pool, char_id, 500, &purchase).await {
        Err(TxError::InsufficientFunds) => {}
        other => panic!("expected InsufficientFunds, got {other:?}"),
    }
    assert!(repos
        .inventories()
        .load_storage(char_id, StorageType::Bag)
        .await
        .unwrap()
        .is_empty());

    sqlx::query("UPDATE character_money SET gold = 1000 WHERE character_id = ?")
        .bind(char_id)
        .execute(&pool)
        .await
        .unwrap();
    shop_buy(&pool, char_id, 500, &purchase).await.unwrap();

    let money = repos.characters().load_money(char_id).await.unwrap();
    assert_eq!(money.gold, 500);
    let bag = repos
        .inventories()
        .load_storage(char_id, StorageType::Bag)
        .await
        .unwrap();
    assert_eq!(bag.len(), 1);
    assert_eq!(bag[0].item.item_id, 30_001);

    repos.characters().delete(char_id).await.unwrap();
    sqlx::query("DELETE FROM accounts WHERE player_id = ?")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn p2p_trade_moves_item_between_characters_atomically() {
    let Some(pool) = test_pool().await else { return };
    let repos = MySqlRepositories::new(pool.clone());
    let account_a = fresh_account_id();
    let account_b = fresh_account_id();
    for (aid, nm) in [(account_a, b"TRADERS".as_slice()), (account_b, b"TRADERE".as_slice())] {
        sqlx::query("INSERT IGNORE INTO accounts (player_id, pass1, pass2) VALUES (?, 'x', 'y')")
            .bind(aid)
            .execute(&pool)
            .await
            .unwrap();
        repos
            .characters()
            .create(aid, nm, &CharacterSeed::default())
            .await
            .unwrap();
    }
    let char_a = repos.characters().find_id_by_name(b"TRADERS").await.unwrap().unwrap();
    let char_b = repos.characters().find_id_by_name(b"TRADERE").await.unwrap().unwrap();

    // Source empty -> rejected before anything moves.
    match p2p_trade(
        &pool,
        char_a,
        (StorageType::Bag, 1),
        char_b,
        (StorageType::Bank, 2),
    )
    .await
    {
        Err(TxError::SourceSlotEmpty) => {}
        other => panic!("expected SourceSlotEmpty, got {other:?}"),
    }

    let offer = sample_item(46_001, 1);
    repos.inventories().save_slot(char_a, &offer, &pool).await.unwrap();

    // Destination occupied -> rejected, source still holds the item.
    repos
        .inventories()
        .save_slot(char_b, &sample_item(11, 2), &pool)
        .await
        .unwrap();
    match p2p_trade(
        &pool,
        char_a,
        (StorageType::Bag, 1),
        char_b,
        (StorageType::Bank, 2),
    )
    .await
    {
        Err(TxError::DestinationSlotOccupied) => {}
        other => panic!("expected DestinationSlotOccupied, got {other:?}"),
    }
    assert_eq!(
        repos
            .inventories()
            .load_slot(char_a, StorageType::Bag, 1)
            .await
            .unwrap()
            .unwrap()
            .item
            .item_id,
        46_001
    );
    sqlx::query("DELETE FROM inventories WHERE character_id = ?")
        .bind(char_b)
        .execute(&pool)
        .await
        .unwrap();

    // Happy path: source cleared, destination carries identical ThingData bytes.
    p2p_trade(
        &pool,
        char_a,
        (StorageType::Bag, 1),
        char_b,
        (StorageType::Bank, 2),
    )
    .await
    .unwrap();

    assert!(repos
        .inventories()
        .load_slot(char_a, StorageType::Bag, 1)
        .await
        .unwrap()
        .map(|s| s.is_empty())
        .unwrap_or(true));
    let received = repos
        .inventories()
        .load_slot(char_b, StorageType::Bank, 2)
        .await
        .unwrap()
        .expect("item landed");
    assert_eq!(received.storage_type, StorageType::Bank);
    assert_eq!(received.slot, 2);
    assert_eq!(received.item.to_bytes(), offer.item.to_bytes());

    repos.characters().delete(char_a).await.unwrap();
    repos.characters().delete(char_b).await.unwrap();
    for aid in [account_a, account_b] {
        sqlx::query("DELETE FROM accounts WHERE player_id = ?")
            .bind(aid)
            .execute(&pool)
            .await
            .unwrap();
    }
}

/// Sanity: the migration actually produced the ticket's table set.
#[tokio::test]
async fn modern_schema_tables_exist() {
    let Some(pool) = test_pool().await else { return };
    let expected = [
        "accounts",
        "characters",
        "character_money",
        "inventories",
        "character_pets",
        "character_skills",
        "character_hotkeys",
        "character_missions",
        "character_mission_flags",
        "character_bit_flags",
        "character_completed_events",
        "friends",
        "mails",
    ];
    for table in expected {
        let found: i64 = sqlx::query(
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = DATABASE() AND table_name = ?",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i64, usize>(0);
        assert_eq!(found, 1, "table `{table}` missing");
    }

    // latin1_bin preservation on text columns (ticket requirement).
    let collations: Vec<(String, String)> = sqlx::query(
        "SELECT table_name, table_collation FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_name IN ('characters', 'mails')",
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.get::<String, _>(0), r.get::<String, _>(1)))
    .collect();
    for (_, collation) in collations {
        assert_eq!(collation, "latin1_bin");
    }
}
