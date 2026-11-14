//! Integration tests for Ticket 03: SQLite migration `0002`, single-transaction
//! `SqliteSessionRepository::save`, `(playerid, scene_id, eventid)` key, `savemap`,
//! `character_bit_flags`, `PetStorageType` preservation, `characters::delete`
//! cleanup, and `auto_save::fingerprint` / `SaveLedger` integrity.

mod common;

use common::test_db::setup_test_db;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::db::modern::model::PetStorageType;
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::persist;
use ts_dream::db::pool::DbPool;
use ts_dream::server::auto_save::{self, SaveLedger};
use ts_dream::server::dispatcher::{self, ServerEnv};
use ts_dream::server::handlers::npc_event;
use ts_dream::server::session::{lock_online_sessions, Conn, PetState, Session};

async fn seed_account_and_char(pool: &DbPool, account_id: i64, name: &[u8]) {
    sqlx::query(
        "INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '123456', '123456', 0)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert account");

    let repos = SqliteRepositories::new(pool.clone());
    let seed = CharacterSeed {
        level: 5,
        sex: 1,
        hair: 1,
        element: 2,
        map_id: 12001,
        map_x: 100,
        map_y: 200,
    };
    repos
        .characters()
        .create(account_id, name, &seed)
        .await
        .expect("create character");
}

#[tokio::test]
async fn migration_0002_schema_and_round_trip_savemap_bitflags_pet_storage() {
    let pool = setup_test_db().await;
    let account_id = 300101i64;
    seed_account_and_char(&pool, account_id, b"MigHero").await;

    // 1. Verify obsolete `character_quest_items` table is dropped.
    let quest_items_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'character_quest_items'",
    )
    .fetch_one(&pool.read)
    .await
    .expect("check sqlite_master");
    assert_eq!(quest_items_exists, 0, "character_quest_items must be dropped by 0002");

    // 2. Build session with savemap, bit_flags, ghost, god, and pets in all 4 storages.
    let mut session = Session::new();
    session.id = account_id as u32;
    session.db_character_id = account_id;
    session.level = 25;
    session.ghost = 1;
    session.god = 7;
    session.savemap = 12001;
    session.savemap_x = 345;
    session.savemap_y = 678;
    session.bit_flags.insert(42);
    session.bit_flags.insert(1337);
    session.quest_tasks.insert(218, (1, 3));
    session.quest_dont.insert(500);
    session.active_pet_stt = 1;
    session.pets = vec![
        PetState {
            stt: 1,
            storage_type: PetStorageType::Carried,
            id: 11001,
            name: b"CarriedPet".to_vec(),
            level: 10,
            quest: 2,
            ..Default::default()
        },
        PetState {
            stt: 5,
            storage_type: PetStorageType::Cart,
            id: 11002,
            name: b"CartPet".to_vec(),
            level: 12,
            quest: 3,
            ..Default::default()
        },
        PetState {
            stt: 6,
            storage_type: PetStorageType::Hotel,
            id: 11003,
            name: b"HotelPet".to_vec(),
            level: 14,
            quest: 4,
            ..Default::default()
        },
        PetState {
            stt: 7,
            storage_type: PetStorageType::Warehouse,
            id: 11004,
            name: b"WarePet".to_vec(),
            level: 16,
            quest: 5,
            ..Default::default()
        },
    ];

    let repos = SqliteRepositories::new(pool.clone());
    repos.sessions().save(&session).await.expect("save session");

    // Verify DB storage_type values directly.
    let pet_storages: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT petid, storagetype FROM character_pets WHERE playerid = ? ORDER BY petid",
    )
    .bind(account_id)
    .fetch_all(&pool.read)
    .await
    .expect("query character_pets");
    assert_eq!(
        pet_storages,
        vec![(11001, 1), (11002, 2), (11003, 3), (11004, 4)]
    );

    // 3. Reload into a fresh Session and verify full round-trip.
    let mut loaded = Session::new();
    let found = repos
        .sessions()
        .load(account_id, &mut loaded)
        .await
        .expect("load session");
    assert!(found);
    assert_eq!(loaded.savemap, 12001);
    assert_eq!(loaded.savemap_x, 345);
    assert_eq!(loaded.savemap_y, 678);
    assert_eq!(loaded.ghost, 1);
    assert_eq!(loaded.god, 7);
    assert_eq!(loaded.bit_flags.len(), 2);
    assert!(loaded.bit_flags.contains(&42));
    assert!(loaded.bit_flags.contains(&1337));
    assert_eq!(loaded.active_pet_stt, 1);
    assert_eq!(loaded.pets.len(), 4);
    assert_eq!(loaded.pets[0].storage_type, PetStorageType::Carried);
    assert_eq!(loaded.pets[0].quest, 2);
    assert_eq!(loaded.pets[1].storage_type, PetStorageType::Cart);
    assert_eq!(loaded.pets[1].quest, 3);
    assert_eq!(loaded.pets[2].storage_type, PetStorageType::Hotel);
    assert_eq!(loaded.pets[2].quest, 4);
    assert_eq!(loaded.pets[3].storage_type, PetStorageType::Warehouse);
    assert_eq!(loaded.pets[3].quest, 5);
}

#[tokio::test]
async fn completed_eve_counts_are_scene_scoped_and_deleted_when_removed() {
    let pool = setup_test_db().await;
    let account_id = 300102i64;
    seed_account_and_char(&pool, account_id, b"SceneHero").await;

    let repos = SqliteRepositories::new(pool.clone());
    let mut session = Session::new();
    session.id = account_id as u32;
    session.db_character_id = account_id;
    session.map_id = 12001;
    // Same eve_no = 5 on two different scenes must not overwrite each other.
    session.completed_eve_counts.insert((12001, 5), 2);
    session.completed_eve_counts.insert((12136, 5), 7);

    repos.sessions().save(&session).await.expect("save both scenes");

    // Snapshot for map 12001 sees only count=2; snapshot for map 12136 sees only count=7.
    let snap_12001 = npc_event::snapshot_state(&session);
    assert_eq!(snap_12001.completed_eve_counts.get(&5), Some(&2));
    session.map_id = 12136;
    let snap_12136 = npc_event::snapshot_state(&session);
    assert_eq!(snap_12136.completed_eve_counts.get(&5), Some(&7));

    // Reload and verify both rows exist independently.
    let mut loaded = Session::new();
    repos.sessions().load(account_id, &mut loaded).await.expect("load");
    assert_eq!(loaded.completed_eve_counts.get(&(12001, 5)), Some(&2));
    assert_eq!(loaded.completed_eve_counts.get(&(12136, 5)), Some(&7));

    // Remove (12001, 5) from session and save -> DB must delete (12001, 5) while keeping (12136, 5).
    loaded.completed_eve_counts.remove(&(12001, 5));
    repos.sessions().save(&loaded).await.expect("save after removal");

    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT scene_id, eventid, completioncount FROM character_completed_events WHERE playerid = ?",
    )
    .bind(account_id)
    .fetch_all(&pool.read)
    .await
    .expect("query completed events");
    assert_eq!(rows, vec![(12136, 5, 7)]);
}

#[tokio::test]
async fn negative_mid_tx_sql_error_rolls_back_and_preserves_save_ledger() {
    let pool = setup_test_db().await;
    let account_id = 300103i64;
    seed_account_and_char(&pool, account_id, b"RollbackHero").await;

    let ledger = SaveLedger::default();
    let mut session = Session::new();
    session.id = account_id as u32;
    session.db_character_id = account_id;
    session.authed = true;
    session.level = 10;
    session.gold = 1000;
    session.savemap = 12001;
    session.bit_flags.insert(77);

    // Clean out any leftover sessions from other tests in the shared registry.
    lock_online_sessions().clear();
    ts_dream::server::session::registry_register_for_login(&session);

    let wrote = auto_save::run_cycle(Some(&pool), &ledger).await;
    assert_eq!(wrote, 1, "initial auto-save must succeed");
    let initial_fp = {
        let guard = ledger.lock().unwrap();
        *guard.get(&session.id).expect("initial ledger entry")
    };

    // Inject a failure at the very last table written in `save()` (`character_completed_events`).
    sqlx::raw_sql(
        "CREATE TRIGGER fail_completed_events BEFORE INSERT ON character_completed_events
         BEGIN
             SELECT RAISE(ABORT, 'injected mid-transaction SQL failure');
         END;",
    )
    .execute(&pool.write)
    .await
    .expect("create fail trigger");

    // Mutate multiple tables' worth of state in RAM.
    session.level = 88;
    session.gold = 999999;
    session.savemap = 10817;
    session.bit_flags.insert(88);
    session.completed_eve_counts.insert((12001, 9), 1);
    ts_dream::server::session::registry_update_if_online(&session);

    let wrote_failed = auto_save::run_cycle(Some(&pool), &ledger).await;
    assert_eq!(wrote_failed, 0, "failed transaction must report 0 saved");

    // SaveLedger must NOT have been updated to the failed fingerprint.
    let fp_after_fail = {
        let guard = ledger.lock().unwrap();
        *guard.get(&session.id).expect("ledger entry still present")
    };
    assert_eq!(
        fp_after_fail, initial_fp,
        "SaveLedger must not advance when save() fails"
    );

    // Verify full SQLite rollback: earlier statements in the same tx (`characters`,
    // `character_money`, `character_bit_flags`) must all be rolled back.
    let (db_level, db_savemap): (i64, i64) =
        sqlx::query_as("SELECT level, savemap_id FROM characters WHERE playerid = ?")
            .bind(account_id)
            .fetch_one(&pool.read)
            .await
            .expect("query characters");
    let db_gold: i64 = sqlx::query_scalar("SELECT gold FROM character_money WHERE playerid = ?")
        .bind(account_id)
        .fetch_one(&pool.read)
        .await
        .expect("query gold");
    let db_flags: Vec<i64> = sqlx::query_scalar(
        "SELECT flagindex FROM character_bit_flags WHERE playerid = ? ORDER BY flagindex",
    )
    .bind(account_id)
    .fetch_all(&pool.read)
    .await
    .expect("query bit flags");
    assert_eq!(db_level, 10, "characters.level must be rolled back");
    assert_eq!(db_savemap, 12001, "characters.savemap_id must be rolled back");
    assert_eq!(db_gold, 1000, "character_money.gold must be rolled back");
    assert_eq!(db_flags, vec![77], "character_bit_flags must be rolled back");

    // Clean up trigger and verify retry succeeds.
    sqlx::raw_sql("DROP TRIGGER fail_completed_events;")
        .execute(&pool.write)
        .await
        .expect("drop trigger");
    let wrote_retry = auto_save::run_cycle(Some(&pool), &ledger).await;
    assert_eq!(wrote_retry, 1, "retry after transient failure must succeed");
    lock_online_sessions().clear();
}

#[tokio::test]
async fn character_delete_cleans_all_quest_and_eve_tables_without_orphans() {
    let pool = setup_test_db().await;
    let victim_id = 300104i64;
    let survivor_id = 300105i64;
    seed_account_and_char(&pool, victim_id, b"VictimChar").await;
    seed_account_and_char(&pool, survivor_id, b"SurvivorChar").await;

    let repos = SqliteRepositories::new(pool.clone());
    for id in [victim_id, survivor_id] {
        let mut s = Session::new();
        s.id = id as u32;
        s.db_character_id = id;
        s.quest_tasks.insert(218, (1, 2));
        s.quest_dont.insert(350);
        s.bit_flags.insert(99);
        s.completed_eve_counts.insert((12001, 5), 1);
        repos.sessions().save(&s).await.expect("save char");
    }

    repos
        .characters()
        .delete(victim_id)
        .await
        .expect("delete victim");

    for table in [
        "characters",
        "character_quest_tasks",
        "character_quest_dont",
        "character_bit_flags",
        "character_completed_events",
    ] {
        let victim_rows: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE playerid = ?"))
                .bind(victim_id)
                .fetch_one(&pool.read)
                .await
                .expect("count victim rows");
        assert_eq!(victim_rows, 0, "orphan rows left in {table} for deleted character");

        let survivor_rows: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE playerid = ?"))
                .bind(survivor_id)
                .fetch_one(&pool.read)
                .await
                .expect("count survivor rows");
        assert_eq!(survivor_rows, 1, "collateral deletion in {table} for survivor");
    }
}

#[test]
fn auto_save_fingerprint_covers_persistent_fields_and_ignores_transient_arrays() {
    let base = Session::new();
    let fp_base = auto_save::fingerprint(&base);

    let mut s_ghost = base.clone();
    s_ghost.ghost = 1;
    assert_ne!(fp_base, auto_save::fingerprint(&s_ghost), "ghost");

    let mut s_god = base.clone();
    s_god.god = 3;
    assert_ne!(fp_base, auto_save::fingerprint(&s_god), "god");

    let mut s_flag = base.clone();
    s_flag.bit_flags.insert(55);
    assert_ne!(fp_base, auto_save::fingerprint(&s_flag), "bit_flags");

    let mut s_savemap = base.clone();
    s_savemap.savemap = 12001;
    assert_ne!(fp_base, auto_save::fingerprint(&s_savemap), "savemap");

    let mut s_savemap_xy = s_savemap.clone();
    s_savemap_xy.savemap_x = 100;
    assert_ne!(
        auto_save::fingerprint(&s_savemap),
        auto_save::fingerprint(&s_savemap_xy),
        "savemap_x"
    );

    let mut s_eve_a = base.clone();
    s_eve_a.completed_eve_counts.insert((12001, 5), 1);
    let mut s_eve_b = base.clone();
    s_eve_b.completed_eve_counts.insert((12136, 5), 1);
    assert_ne!(
        auto_save::fingerprint(&s_eve_a),
        auto_save::fingerprint(&s_eve_b),
        "completed_eve_counts scene_id"
    );

    let mut s_pet = base.clone();
    s_pet.pets.push(PetState {
        stt: 1,
        storage_type: PetStorageType::Carried,
        id: 11001,
        quest: 1,
        ..Default::default()
    });
    let fp_pet = auto_save::fingerprint(&s_pet);
    let mut s_pet_quest = s_pet.clone();
    s_pet_quest.pets[0].quest = 2;
    assert_ne!(fp_pet, auto_save::fingerprint(&s_pet_quest), "pet.quest");

    let mut s_pet_storage = s_pet.clone();
    s_pet_storage.pets[0].storage_type = PetStorageType::Cart;
    assert_ne!(
        fp_pet,
        auto_save::fingerprint(&s_pet_storage),
        "pet.storage_type"
    );

    // Transient in-memory arrays not persisted to DB must NOT dirty fingerprint.
    let mut s_transient = base.clone();
    s_transient.quest_steps.push((100, 1));
    assert_eq!(
        fp_base,
        auto_save::fingerprint(&s_transient),
        "transient quest_steps must not affect fingerprint"
    );
}

#[tokio::test]
async fn upsert_pet_and_pet_actions_preserve_storage_type_across_reload() {
    let pool = setup_test_db().await;
    let account_id = 300106i64;
    seed_account_and_char(&pool, account_id, b"PetStorageHero").await;

    // 1. Direct `persist::upsert_pet` must preserve Cart (2) and Warehouse (4), not coerce to Hotel (3).
    let cart_pet = PetState {
        stt: 5,
        storage_type: PetStorageType::Cart,
        id: 11010,
        name: b"CartPet".to_vec(),
        level: 10,
        ..Default::default()
    };
    let ware_pet = PetState {
        stt: 6,
        storage_type: PetStorageType::Warehouse,
        id: 11011,
        name: b"WarePet".to_vec(),
        level: 11,
        ..Default::default()
    };
    persist::upsert_pet(Some(&pool), account_id as u32, &cart_pet).await;
    persist::upsert_pet(Some(&pool), account_id as u32, &ware_pet).await;

    let repos = SqliteRepositories::new(pool.clone());
    let mut loaded = Session::new();
    repos.sessions().load(account_id, &mut loaded).await.expect("load");
    assert_eq!(loaded.pets.len(), 2);
    assert_eq!(loaded.pets[0].storage_type, PetStorageType::Cart);
    assert_eq!(loaded.pets[1].storage_type, PetStorageType::Warehouse);

    // 2. Moving a Hotel pet (stt=5, slot=1) -> Roster via `0x0F sub 3` must update storage_type to Carried (1).
    let mut conn = Conn::default();
    conn.session.id = account_id as u32;
    conn.session.db_character_id = account_id;
    conn.session.authed = true;
    conn.session.pets = vec![PetState {
        stt: 5,
        storage_type: PetStorageType::Hotel,
        id: 11012,
        name: b"HotelPet".to_vec(),
        level: 15,
        ..Default::default()
    }];
    repos.sessions().save(&conn.session).await.expect("save initial hotel pet");

    let data = std::sync::Arc::new(GameData::default());
    let service = BattleService::new(data.clone());
    let env = ServerEnv::with_pool(&pool);
    // 0x0F sub 3: stable slot 1 -> roster slot 1 (decoded F4 44 frame)
    dispatcher::dispatch(
        &mut conn,
        &[0xF4, 0x44, 0x03, 0x00, 0x0F, 0x03, 0x01],
        &data,
        &service,
        &env,
    )
    .await;
    assert_eq!(conn.session.pets[0].stt, 1);
    assert_eq!(conn.session.pets[0].storage_type, PetStorageType::Carried);

    let mut reloaded = Session::new();
    repos.sessions().load(account_id, &mut reloaded).await.expect("reload after stable->roster");
    assert_eq!(reloaded.pets.len(), 1);
    assert_eq!(reloaded.pets[0].stt, 1);
    assert_eq!(reloaded.pets[0].storage_type, PetStorageType::Carried);
}

#[tokio::test]
async fn migration_0002_in_place_upgrade_preserves_existing_completed_events_under_scene_0() {
    let pool = setup_test_db().await;
    // Reset to pure 0001_init.sql schema (pre-0002) and seed an old `character_completed_events` row.
    sqlx::raw_sql(
        "PRAGMA foreign_keys = OFF;
         DROP TABLE IF EXISTS character_completed_events;
         DROP TABLE IF EXISTS characters;
         DROP TABLE IF EXISTS character_money;
         DROP TABLE IF EXISTS accounts;
         PRAGMA foreign_keys = ON;",
    )
    .execute(&pool.write)
    .await
    .expect("drop tables for pre-0002 simulation");
    sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
        .execute(&pool.write)
        .await
        .expect("apply 0001_init.sql");

    let account_id = 300107i64;
    sqlx::query("INSERT INTO accounts (playerid, pass1, pass2, createdat) VALUES (?, '123456', '123456', 0)")
        .bind(account_id)
        .execute(&pool.write)
        .await
        .expect("insert account");
    sqlx::query("INSERT INTO characters (playerid, name, level) VALUES (?, 'LegacyHero', 10)")
        .bind(account_id)
        .execute(&pool.write)
        .await
        .expect("insert char");
    sqlx::query("INSERT INTO character_money (playerid) VALUES (?)")
        .bind(account_id)
        .execute(&pool.write)
        .await
        .expect("insert money");
    sqlx::query(
        "INSERT INTO character_completed_events (playerid, eventid, completedat, completioncount) VALUES (?, 5, 111111, 3)",
    )
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert pre-0002 completed event");

    // Apply 0002 migration in-place.
    sqlx::raw_sql(include_str!("../migrations/0002_eve_quest_persistence.sql"))
        .execute(&pool.write)
        .await
        .expect("apply 0002_eve_quest_persistence.sql in-place");

    let repos = SqliteRepositories::new(pool.clone());
    let mut loaded = Session::new();
    repos.sessions().load(account_id, &mut loaded).await.expect("load migrated session");
    assert_eq!(loaded.completed_eve_counts.get(&(0, 5)), Some(&3));

    // Adding (12001, 5) must coexist with migrated (0, 5).
    loaded.completed_eve_counts.insert((12001, 5), 1);
    repos.sessions().save(&loaded).await.expect("save both");

    let rows: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT scene_id, eventid, completedat, completioncount FROM character_completed_events WHERE playerid = ? ORDER BY scene_id",
    )
    .bind(account_id)
    .fetch_all(&pool.read)
    .await
    .expect("query migrated rows");
    assert_eq!(rows, vec![(0, 5, 111111, 3), (12001, 5, rows[1].2, 1)]);
}
