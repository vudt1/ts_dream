use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::{CharacterRepository, CharacterSeed, QuestRepository};
use ts_dream::db::pool::{bootstrap, DbPool};
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::login::handle_login;
use ts_dream::server::session::{Conn, InventoryItem, PetState, Session};
use ts_dream::server::spawn;

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
async fn test_account_creation_and_auth() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    // Test creating an account
    let account_id = ts_dream::db::accounts::create(&pool, "mypass123", "mypass456")
        .await
        .expect("create account");
    assert!(
        account_id >= 300000,
        "playerid must be >= 300000 from sequence seed, got {account_id}"
    );

    // Verify pass1 via modern repo
    let pass1_ok = repos
        .accounts()
        .verify_pass1(account_id, b"mypass123")
        .await
        .expect("verify pass1");
    assert!(pass1_ok, "pass1 should verify");

    let pass1_bad = repos
        .accounts()
        .verify_pass1(account_id, b"wrongpass")
        .await
        .expect("verify pass1 bad");
    assert!(!pass1_bad, "wrong pass1 should fail");

    // Verify pass2 via modern repo
    let pass2_ok = repos
        .accounts()
        .verify_pass2(account_id, b"mypass456")
        .await
        .expect("verify pass2");
    assert!(pass2_ok, "pass2 should verify");

    // Check access
    let access = repos
        .accounts()
        .access(account_id)
        .await
        .expect("load access")
        .expect("account exists");
    assert_eq!(access.account_id, account_id);
    assert_eq!(access.gm_level, 0);
    assert!(!access.is_suspended);

    // Touch login
    let now = chrono::Utc::now().timestamp_millis();
    repos
        .accounts()
        .touch_login(account_id, now, "127.0.0.1")
        .await
        .expect("touch login");

    // List accounts
    let accounts = ts_dream::db::accounts::list(&pool)
        .await
        .expect("list accounts");
    let acc = accounts
        .iter()
        .find(|a| a.player_id == account_id)
        .expect("found account");
    assert_eq!(acc.last_login_at, Some(now));
    assert_eq!(acc.last_login_ip.as_deref(), Some("127.0.0.1"));

    // Change pass
    let changed = ts_dream::db::accounts::change_pass(&pool, account_id, "newpass12", "newpass34")
        .await
        .expect("change pass");
    assert!(changed);
    let new_pass1_ok = repos
        .accounts()
        .verify_pass1(account_id, b"newpass12")
        .await
        .expect("verify new pass1");
    assert!(new_pass1_ok);
}

#[tokio::test]
async fn test_character_creation_and_listing() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "charpass1", "charpass2")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 3,
        element: 4,
        map_id: 10817,
        map_x: 500,
        map_y: 600,
    };
    let char_name = b"AnhHung";
    let char_id = repos
        .characters()
        .create(account_id, char_name, &seed)
        .await
        .expect("create character");
    assert_eq!(char_id, account_id);

    // Find id by name
    let found = repos
        .characters()
        .find_id_by_name(char_name)
        .await
        .expect("find by name");
    assert_eq!(found, Some(account_id));

    // List by account
    let list = repos
        .characters()
        .list_by_account(account_id)
        .await
        .expect("list by account");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, account_id);
    assert_eq!(list[0].name, char_name);
    assert_eq!(list[0].level, 1);
    assert_eq!(list[0].sex, 1);
    assert_eq!(list[0].hair, 3);
    assert_eq!(list[0].element, 4);

    // Initial money ledger created with default 0
    let money = repos
        .characters()
        .load_money(account_id)
        .await
        .expect("load money");
    assert_eq!(money.gold, 0);
    assert_eq!(money.bank_gold, 0);
    assert_eq!(money.shop_point, 0);
}

#[tokio::test]
async fn test_session_load_and_save() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "sesspass1", "sesspass2")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 2,
        element: 2,
        map_id: 10817,
        map_x: 400,
        map_y: 500,
    };
    let char_name = b"NuHiep";
    repos
        .characters()
        .create(account_id, char_name, &seed)
        .await
        .expect("create character");

    let session = Session {
        id: account_id as u32,
        account_name: b"NuHiep".to_vec(),
        gm_level: 0,
        db_character_id: account_id,
        logined: true,
        authed: true,
        idtalking: 0,
        idnpctalking: 0,
        select_menu: 0,
        talk_count: 0,
        warp_finish: false,
        talk_type: String::new(),
        battle_id: 0,
        pending_pass: Vec::new(),
        pending_new_char_name: Vec::new(),
        name: char_name.to_vec(),
        map_id: 10817,
        map_x: 420,
        map_y: 520,
        gocnhin: 0,
        dongtac: 0,
        pk: 1,
        tham_chien: 0,
        level: 10,
        reborn: 0,
        job: 1,
        sex: 0,
        hair: 2,
        thuoctinh: 2,
        hp: 150,
        hp_max: 150,
        sp: 80,
        sp_max: 80,
        point: 5,
        skill_point: 2,
        int1: 15,
        atk: 25,
        def: 20,
        hpx: 10,
        spx: 10,
        agi: 18,
        int2: 0,
        atk2: 0,
        def2: 0,
        hpx2: 0,
        spx2: 0,
        agi2: 0,
        texp: 1200,
        gold: 5000,
        tiengtam: 0,
        god: 0,
        hp_store: 0,
        sp_store: 0,
        newbie: 1,
        savemap: 10817,
        color: String::new(),
        skills: vec![(10001, 3), (10002, 1)],
        hotkeys: [0, 10001, 10002, 0, 0, 0, 0, 0, 0, 0, 0],
        homdo: vec![InventoryItem {
            slot: 1,
            id: 10001,
            count: 5,
            doben: 100,
            ..Default::default()
        }],
        trangbi: vec![],
        tientrang: vec![],
        tuideo: vec![],
        luulang: vec![],
        pets: vec![PetState {
            stt: 1,
            id: 11001,
            name: b"LuuBi".to_vec(),
            level: 15,
            thuoctinh: 3,
            reborn: 0,
            hp: 200,
            hp_max: 200,
            sp: 100,
            sp_max: 100,
            int1: 20,
            atk: 30,
            def: 25,
            hpx: 15,
            spx: 10,
            agi: 22,
            fai: 80,
            thd: 50,
            texp: 3000,
            skill_point: 1,
            quest: 0,
            skills: [(12001, 2), (0, 0), (0, 0), (0, 0)],
            ..Default::default()
        }],
        active_pet_stt: 1,
        shop: Default::default(),
        open_shop_id: 0,
        bank_gold: 10000,
        shop_point: 200,
        horse_pet_id: 0,
        trade: Default::default(),
        talking_battle: 0,
        completed_quests: vec![],
        click_npc_id: 0,
        id_mem: [0; 4],
        id_leader: 0,
        id_qs: 0,
        quest_steps: vec![],
        warp_steps: vec![],
    };

    // Save session
    repos.sessions().save(&session).await.expect("save session");

    // Load into a blank session
    let mut loaded_session = Session {
        id: account_id as u32,
        account_name: Vec::new(),
        gm_level: 0,
        db_character_id: 0,
        logined: false,
        authed: false,
        idtalking: 0,
        idnpctalking: 0,
        select_menu: 0,
        talk_count: 0,
        warp_finish: false,
        talk_type: String::new(),
        battle_id: 0,
        pending_pass: Vec::new(),
        pending_new_char_name: Vec::new(),
        name: Vec::new(),
        map_id: 0,
        map_x: 0,
        map_y: 0,
        gocnhin: 0,
        dongtac: 0,
        pk: 0,
        tham_chien: 0,
        level: 0,
        reborn: 0,
        job: 0,
        sex: 0,
        hair: 0,
        thuoctinh: 0,
        hp: 0,
        hp_max: 0,
        sp: 0,
        sp_max: 0,
        point: 0,
        skill_point: 0,
        int1: 0,
        atk: 0,
        def: 0,
        hpx: 0,
        spx: 0,
        agi: 0,
        int2: 0,
        atk2: 0,
        def2: 0,
        hpx2: 0,
        spx2: 0,
        agi2: 0,
        texp: 0,
        gold: 0,
        tiengtam: 0,
        god: 0,
        hp_store: 0,
        sp_store: 0,
        newbie: 0,
        savemap: 0,
        color: String::new(),
        skills: vec![],
        hotkeys: [0; 11],
        homdo: vec![],
        trangbi: vec![],
        tientrang: vec![],
        tuideo: vec![],
        luulang: vec![],
        pets: vec![],
        active_pet_stt: 0,
        shop: Default::default(),
        open_shop_id: 0,
        bank_gold: 0,
        shop_point: 0,
        horse_pet_id: 0,
        trade: Default::default(),
        talking_battle: 0,
        completed_quests: vec![],
        click_npc_id: 0,
        id_mem: [0; 4],
        id_leader: 0,
        id_qs: 0,
        quest_steps: vec![],
        warp_steps: vec![],
    };

    let loaded = repos
        .sessions()
        .load(account_id, &mut loaded_session)
        .await
        .expect("load session");
    assert!(loaded, "session must be found and loaded");

    assert_eq!(loaded_session.db_character_id, account_id);
    assert_eq!(loaded_session.level, 10);
    assert_eq!(loaded_session.job, 1);
    assert_eq!(loaded_session.sex, 0);
    assert_eq!(loaded_session.hair, 2);
    assert_eq!(loaded_session.thuoctinh, 2);
    assert_eq!(loaded_session.hp, 150);
    assert_eq!(loaded_session.hp_max, 150);
    assert_eq!(loaded_session.sp, 80);
    assert_eq!(loaded_session.sp_max, 80);
    assert_eq!(loaded_session.point, 5);
    assert_eq!(loaded_session.skill_point, 2);
    assert_eq!(loaded_session.int1, 15);
    assert_eq!(loaded_session.atk, 25);
    assert_eq!(loaded_session.def, 20);
    assert_eq!(loaded_session.hpx, 10);
    assert_eq!(loaded_session.spx, 10);
    assert_eq!(loaded_session.agi, 18);
    assert_eq!(loaded_session.map_id, 10817);
    assert_eq!(loaded_session.map_x, 420);
    assert_eq!(loaded_session.map_y, 520);
    assert_eq!(loaded_session.gold, 5000);
    assert_eq!(loaded_session.bank_gold, 10000);
    assert_eq!(loaded_session.shop_point, 200);
    assert_eq!(loaded_session.newbie, 1);
    assert_eq!(loaded_session.pk, 1, "pk flag must be persisted and loaded");
    assert_eq!(loaded_session.texp, 1200, "character texp/exp must be persisted and loaded");
    assert_eq!(loaded_session.active_pet_stt, 1, "active pet status must be persisted and loaded");

    // Inventories
    assert_eq!(loaded_session.homdo.len(), 1);
    assert_eq!(loaded_session.homdo[0].slot, 1);
    assert_eq!(loaded_session.homdo[0].id, 10001);
    assert_eq!(loaded_session.homdo[0].count, 5);

    // Skills
    assert_eq!(loaded_session.skills, vec![(10001, 3), (10002, 1)]);

    // Hotkeys
    assert_eq!(loaded_session.hotkeys[1], 10001);
    assert_eq!(loaded_session.hotkeys[2], 10002);

    // Pets
    assert_eq!(loaded_session.pets.len(), 1);
    let pet = &loaded_session.pets[0];
    assert_eq!(pet.stt, 1);
    assert_eq!(pet.id, 11001);
    assert_eq!(pet.name, b"LuuBi");
    assert_eq!(pet.level, 15);
    assert_eq!(pet.thuoctinh, 3);
    assert_eq!(pet.hp, 200);
    assert_eq!(pet.atk, 30);
    assert_eq!(pet.thd, 50);
    assert_eq!(pet.texp, 3000);
    assert_eq!(pet.skills[0], (12001, 2));
}

#[tokio::test]
async fn test_quest_repository_and_deletion() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "questpass1", "questpass2")
        .await
        .expect("create account");

    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(account_id, b"QuestChar", &seed)
        .await
        .expect("create character");

    // Missions
    let mission = ts_dream::db::modern::model::MissionRow {
        mission_id: 101,
        step: 2,
        state: 1,
        updated_at: 1234567,
    };
    repos
        .quests()
        .upsert_mission(account_id, &mission)
        .await
        .expect("upsert mission");
    let missions = repos
        .quests()
        .list_missions(account_id)
        .await
        .expect("list missions");
    assert_eq!(missions.len(), 1);
    assert_eq!(missions[0].mission_id, 101);
    assert_eq!(missions[0].step, 2);
    assert_eq!(missions[0].state, 1);

    // Bit flags
    assert!(!repos
        .quests()
        .has_bit_flag(account_id, 42)
        .await
        .expect("has flag"));
    repos
        .quests()
        .set_bit_flag(account_id, 42, 1000)
        .await
        .expect("set flag");
    assert!(repos
        .quests()
        .has_bit_flag(account_id, 42)
        .await
        .expect("has flag"));

    // Completed events
    assert!(!repos
        .quests()
        .has_completed_event(account_id, 999)
        .await
        .expect("has event"));
    repos
        .quests()
        .mark_event_completed(account_id, 999, 2000)
        .await
        .expect("mark event");
    assert!(repos
        .quests()
        .has_completed_event(account_id, 999)
        .await
        .expect("has event"));

    // Delete character cascades
    repos
        .characters()
        .delete(account_id)
        .await
        .expect("delete character");
    let list = repos
        .characters()
        .list_by_account(account_id)
        .await
        .expect("list characters");
    assert!(list.is_empty(), "character should be deleted");
    let missions_after = repos
        .quests()
        .list_missions(account_id)
        .await
        .expect("list missions after delete");
    assert!(
        missions_after.is_empty(),
        "missions should be cleaned up on delete"
    );
}

#[tokio::test]
async fn test_item_code_redeem_and_special_gift() {
    use ts_dream::db::item_code::{redeem_and_grant, redeem_special_gift, reward_for, RedeemOutcome};

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "giftpass1", "giftpass2")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 1,
        element: 3,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(account_id, b"GiftHero", &seed)
        .await
        .expect("create character");

    // Insert an item code
    sqlx::query(
        "INSERT INTO item_code (code, password, playerid, itemid, count) VALUES ('GIFT99', 'PASS99', 0, 46238, 2)",
    )
    .execute(&pool.write)
    .await
    .expect("insert item_code");

    // 1. Preview reward
    let preview = reward_for(&pool, "GIFT99", "PASS99").await.expect("reward_for");
    assert_eq!(preview, Some((46238, 2)));

    // 2. Redeem successfully
    let item = InventoryItem {
        slot: 1,
        id: 46238,
        count: 2,
        doben: 100,
        ..Default::default()
    };
    let outcome = redeem_and_grant(&pool, account_id, "GIFT99", "PASS99", 1, &item)
        .await
        .expect("redeem");
    assert_eq!(
        outcome,
        RedeemOutcome::Granted {
            item_id: 46238,
            count: 2,
            newbie: false,
        }
    );

    // 3. Redeem again -> should be InvalidOrUsed
    let outcome2 = redeem_and_grant(&pool, account_id, "GIFT99", "PASS99", 1, &item)
        .await
        .expect("redeem again");
    assert_eq!(outcome2, RedeemOutcome::InvalidOrUsed);

    // 4. Special gift TSVN123 / TSVN456
    sqlx::query(
        "INSERT INTO item_code (code, password, playerid, itemid, count) VALUES ('TSVN123', 'TSVN456', 0, 10001, 1)",
    )
    .execute(&pool.write)
    .await
    .expect("insert special code");

    let special_items = vec![InventoryItem {
        slot: 1,
        id: 10001,
        count: 1,
        doben: 100,
        ..Default::default()
    }];
    let gift_outcome = redeem_special_gift(&pool, account_id, &special_items)
        .await
        .expect("redeem special gift");
    assert_eq!(
        gift_outcome,
        RedeemOutcome::Granted {
            item_id: 10001,
            count: 1,
            newbie: true,
        }
    );

    // Try special gift second time -> AlreadyGifted
    let gift_outcome2 = redeem_special_gift(&pool, account_id, &special_items)
        .await
        .expect("redeem special gift again");
    assert_eq!(gift_outcome2, RedeemOutcome::AlreadyGifted);
}

#[tokio::test]
async fn test_nonexistent_and_edge_cases() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let nonexistent_id = 999999;

    // Load session for non-existent player
    let mut empty_session = Session {
        id: nonexistent_id as u32,
        account_name: Vec::new(),
        gm_level: 0,
        db_character_id: 0,
        logined: false,
        authed: false,
        idtalking: 0,
        idnpctalking: 0,
        select_menu: 0,
        talk_count: 0,
        warp_finish: false,
        talk_type: String::new(),
        battle_id: 0,
        pending_pass: Vec::new(),
        pending_new_char_name: Vec::new(),
        name: Vec::new(),
        map_id: 0,
        map_x: 0,
        map_y: 0,
        gocnhin: 0,
        dongtac: 0,
        pk: 0,
        tham_chien: 0,
        level: 0,
        reborn: 0,
        job: 0,
        sex: 0,
        hair: 0,
        thuoctinh: 0,
        hp: 0,
        hp_max: 0,
        sp: 0,
        sp_max: 0,
        point: 0,
        skill_point: 0,
        int1: 0,
        atk: 0,
        def: 0,
        hpx: 0,
        spx: 0,
        agi: 0,
        int2: 0,
        atk2: 0,
        def2: 0,
        hpx2: 0,
        spx2: 0,
        agi2: 0,
        texp: 0,
        gold: 0,
        tiengtam: 0,
        god: 0,
        hp_store: 0,
        sp_store: 0,
        newbie: 0,
        savemap: 0,
        color: String::new(),
        skills: vec![],
        hotkeys: [0; 11],
        homdo: vec![],
        trangbi: vec![],
        tientrang: vec![],
        tuideo: vec![],
        luulang: vec![],
        pets: vec![],
        active_pet_stt: 0,
        shop: Default::default(),
        open_shop_id: 0,
        bank_gold: 0,
        shop_point: 0,
        horse_pet_id: 0,
        trade: Default::default(),
        talking_battle: 0,
        completed_quests: vec![],
        click_npc_id: 0,
        id_mem: [0; 4],
        id_leader: 0,
        id_qs: 0,
        quest_steps: vec![],
        warp_steps: vec![],
    };
    let found = repos
        .sessions()
        .load(nonexistent_id, &mut empty_session)
        .await
        .expect("load nonexistent");
    assert!(!found, "should return false for nonexistent character");

    // Access nonexistent
    let access = repos
        .accounts()
        .access(nonexistent_id)
        .await
        .expect("access nonexistent");
    assert_eq!(access, None);

    // Pass1 nonexistent
    let pass1 = ts_dream::db::accounts::pass1(&pool, nonexistent_id)
        .await
        .expect("pass1 nonexistent");
    assert_eq!(pass1, None);

    // Passwords nonexistent
    let passes = ts_dream::db::accounts::passwords(&pool, nonexistent_id)
        .await
        .expect("passes nonexistent");
    assert_eq!(passes, None);

    // Change pass nonexistent
    let changed = ts_dream::db::accounts::change_pass(&pool, nonexistent_id, "p1", "p2")
        .await
        .expect("change pass nonexistent");
    assert!(!changed);

    // Find id by nonexistent name
    let found_name = repos
        .characters()
        .find_id_by_name(b"NonExistentHeroName")
        .await
        .expect("find name");
    assert_eq!(found_name, None);
}

#[tokio::test]
async fn test_inventory_repository_and_storage_types() {
    use ts_dream::db::modern::model::{InventorySlot, StorageType};
    use ts_dream::db::modern::traits::InventoryRepository;
    use ts_dream::protocol::codecs::thing_data::ThingData;

    // Verify canonical StorageType enum values matching 0001_init.sql
    assert_eq!(StorageType::Bag.value(), 1);
    assert_eq!(StorageType::Bank.value(), 2);
    assert_eq!(StorageType::Secondary.value(), 4);
    assert_eq!(StorageType::Equip.value(), 8);
    assert_eq!(StorageType::Warehouse.value(), 16);

    assert_eq!(StorageType::from_value(1), Some(StorageType::Bag));
    assert_eq!(StorageType::from_value(2), Some(StorageType::Bank));
    assert_eq!(StorageType::from_value(4), Some(StorageType::Secondary));

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "invpass1", "invpass2")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos
        .characters()
        .create(account_id, b"InvHero", &seed)
        .await
        .expect("create char");

    // Save item in Bag (1)
    let bag_slot = InventorySlot {
        storage_type: StorageType::Bag,
        slot: 1,
        item: ThingData {
            item_id: 20001,
            quantity: 10,
            damage: 80,
            element: 2,
            ..Default::default()
        },
    };
    repos
        .inventories()
        .save_slot(account_id, &bag_slot, &pool.write)
        .await
        .expect("save bag slot");

    // Save item in Bank (2)
    let bank_slot = InventorySlot {
        storage_type: StorageType::Bank,
        slot: 1,
        item: ThingData {
            item_id: 30001,
            quantity: 5,
            damage: 100,
            ..Default::default()
        },
    };
    repos
        .inventories()
        .save_slot(account_id, &bank_slot, &pool.write)
        .await
        .expect("save bank slot");

    // Load storage: Bag
    let bag_items = repos
        .inventories()
        .load_storage(account_id, StorageType::Bag)
        .await
        .expect("load bag");
    assert_eq!(bag_items.len(), 1);
    assert_eq!(bag_items[0].slot, 1);
    assert_eq!(bag_items[0].item.item_id, 20001);
    assert_eq!(bag_items[0].item.quantity, 10);
    assert_eq!(bag_items[0].item.damage, 80);

    // Load storage: Bank
    let bank_items = repos
        .inventories()
        .load_storage(account_id, StorageType::Bank)
        .await
        .expect("load bank");
    assert_eq!(bank_items.len(), 1);
    assert_eq!(bank_items[0].item.item_id, 30001);

    // Load single slot
    let loaded_slot = repos
        .inventories()
        .load_slot(account_id, StorageType::Bag, 1)
        .await
        .expect("load slot")
        .expect("slot exists");
    assert_eq!(loaded_slot.item.item_id, 20001);

    // Clear slot
    repos
        .inventories()
        .clear_slot(account_id, StorageType::Bag, 1, &pool.write)
        .await
        .expect("clear slot");
    let after_clear = repos
        .inventories()
        .load_slot(account_id, StorageType::Bag, 1)
        .await
        .expect("load cleared slot");
    assert!(after_clear.is_none() || after_clear.unwrap().is_empty());
}

#[tokio::test]
async fn test_pet_repository_and_is_active() {
    use ts_dream::db::modern::model::{PetRecord, PetSkill, PetStorageType};
    use ts_dream::db::modern::traits::PetRepository;

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "petpass1", "petpass2")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 0,
        hair: 2,
        element: 3,
        map_id: 10817,
        map_x: 200,
        map_y: 200,
    };
    repos
        .characters()
        .create(account_id, b"PetMaster", &seed)
        .await
        .expect("create char");

    let pet_rec = PetRecord {
        slot: 1,
        pet_id: 15001,
        name: b"TrieuVan".to_vec(),
        level: 25,
        element: 4,
        reborn: 0,
        hp: 350,
        hp_max: 350,
        sp: 150,
        sp_max: 150,
        int_attr: 30,
        atk: 50,
        def: 45,
        hpx: 20,
        spx: 15,
        agi: 40,
        fai: 95,
        texp: 8000,
        skill_point: 3,
        thd: 60,
        skills: [
            PetSkill { id: 14001, level: 3 },
            PetSkill { id: 0, level: 0 },
            PetSkill { id: 0, level: 0 },
            PetSkill { id: 0, level: 0 },
        ],
        quest: 1,
        is_active: true,
    };

    // Save pet
    repos
        .pets()
        .save_pet(account_id, PetStorageType::Carried, &pet_rec)
        .await
        .expect("save pet");

    // Load pet storage
    let carried = repos
        .pets()
        .load_storage(account_id, PetStorageType::Carried)
        .await
        .expect("load carried");
    assert_eq!(carried.len(), 1);
    let loaded_pet = &carried[0];
    assert_eq!(loaded_pet.pet_id, 15001);
    assert_eq!(loaded_pet.name, b"TrieuVan");
    assert_eq!(loaded_pet.level, 25);
    assert_eq!(loaded_pet.thd, 60);
    assert_eq!(loaded_pet.texp, 8000);
    assert!(loaded_pet.is_active, "is_active flag must round-trip");

    // Delete pet
    repos
        .pets()
        .delete_pet(account_id, PetStorageType::Carried, 1)
        .await
        .expect("delete pet");
    let after_delete = repos
        .pets()
        .load_storage(account_id, PetStorageType::Carried)
        .await
        .expect("load after delete");
    assert!(after_delete.is_empty());
}

#[tokio::test]
async fn test_atomic_transactions() {
    use ts_dream::db::modern::model::{InventorySlot, StorageType};
    use ts_dream::db::modern::traits::InventoryRepository;
    use ts_dream::db::modern::transactions::{bank_transfer, p2p_trade, shop_buy, TxError};
    use ts_dream::protocol::codecs::thing_data::ThingData;

    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    // 1. Create player A
    let acc_a = ts_dream::db::accounts::create(&pool, "txpass1", "txpass2")
        .await
        .expect("create acc_a");
    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(acc_a, b"TraderA", &seed).await.expect("create char A");

    // Set initial money for A: gold = 1000, bank_gold = 500
    sqlx::query("UPDATE character_money SET gold = 1000, bankgold = 500 WHERE playerid = ?")
        .bind(acc_a)
        .execute(&pool.write)
        .await
        .expect("set money");

    // Deposit 300 to bank (amount > 0)
    let money = bank_transfer(&pool, acc_a, 300).await.expect("deposit");
    assert_eq!(money.gold, 700);
    assert_eq!(money.bank_gold, 800);

    // Withdraw 200 from bank (amount < 0)
    let money = bank_transfer(&pool, acc_a, -200).await.expect("withdraw");
    assert_eq!(money.gold, 900);
    assert_eq!(money.bank_gold, 600);

    // Overdraft attempt (try withdrawing 1000 when bank only has 600)
    let err = bank_transfer(&pool, acc_a, -1000).await;
    assert!(matches!(err, Err(TxError::InsufficientFunds)));

    // Shop buy test: price = 400
    let purchase = InventorySlot {
        storage_type: StorageType::Bag,
        slot: 1,
        item: ThingData {
            item_id: 40001,
            quantity: 1,
            damage: 100,
            ..Default::default()
        },
    };
    shop_buy(&pool, acc_a, 400, &purchase).await.expect("shop buy");
    let after_buy = repos.characters().load_money(acc_a).await.expect("load money");
    assert_eq!(after_buy.gold, 500);
    let bag_slot = repos.inventories().load_slot(acc_a, StorageType::Bag, 1).await.expect("load bought slot");
    assert_eq!(bag_slot.unwrap().item.item_id, 40001);

    // 2. Create player B for p2p_trade
    let acc_b = ts_dream::db::accounts::create(&pool, "txpass3", "txpass4")
        .await
        .expect("create acc_b");
    repos.characters().create(acc_b, b"TraderB", &seed).await.expect("create char B");

    // Trade slot 1 from A to B slot 1
    p2p_trade(
        &pool,
        acc_a,
        (StorageType::Bag, 1),
        acc_b,
        (StorageType::Bag, 1),
    )
    .await
    .expect("p2p trade");

    let a_slot = repos.inventories().load_slot(acc_a, StorageType::Bag, 1).await.expect("load a");
    assert!(a_slot.is_none() || a_slot.unwrap().is_empty());

    let b_slot = repos.inventories().load_slot(acc_b, StorageType::Bag, 1).await.expect("load b");
    assert_eq!(b_slot.unwrap().item.item_id, 40001);
}

#[tokio::test]
async fn test_persist_update_player_and_stt_pet() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());

    let account_id = ts_dream::db::accounts::create(&pool, "perspass1", "perspass2")
        .await
        .expect("create account");
    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 1,
        element: 1,
        map_id: 10817,
        map_x: 100,
        map_y: 100,
    };
    repos.characters().create(account_id, b"PersistHero", &seed).await.expect("create char");

    // Insert 2 pets in carried storage (1)
    sqlx::query(
        "INSERT INTO character_pets (playerid, storagetype, slot, petid, name, isactive) VALUES (?, 1, 1, 11001, 'Pet1', 0), (?, 1, 2, 11002, 'Pet2', 0)",
    )
    .bind(account_id)
    .bind(account_id)
    .execute(&pool.write)
    .await
    .expect("insert pets");

    // Test SttPetXuatchien -> sets pet 2 to active
    ts_dream::db::persist::update_player(Some(&pool), account_id as u32, "SttPetXuatchien", 2).await;

    let p1_active: i64 = sqlx::query_scalar("SELECT isactive FROM character_pets WHERE playerid = ? AND slot = 1")
        .bind(account_id)
        .fetch_one(&pool.read)
        .await
        .expect("fetch p1");
    let p2_active: i64 = sqlx::query_scalar("SELECT isactive FROM character_pets WHERE playerid = ? AND slot = 2")
        .bind(account_id)
        .fetch_one(&pool.read)
        .await
        .expect("fetch p2");
    assert_eq!(p1_active, 0);
    assert_eq!(p2_active, 1);

    // Test SttPetXuatchien = 0 -> resets all to inactive
    ts_dream::db::persist::update_player(Some(&pool), account_id as u32, "SttPetXuatchien", 0).await;
    let p2_after: i64 = sqlx::query_scalar("SELECT isactive FROM character_pets WHERE playerid = ? AND slot = 2")
        .bind(account_id)
        .fetch_one(&pool.read)
        .await
        .expect("fetch p2 after unequip");
    assert_eq!(p2_after, 0);

    // Test update_player for Pk and Texp
    ts_dream::db::persist::update_player(Some(&pool), account_id as u32, "Pk", 1).await;
    ts_dream::db::persist::update_player(Some(&pool), account_id as u32, "Texp", 9999).await;

    let (pk, texp): (i64, i64) = sqlx::query_as("SELECT pk, curexp FROM characters WHERE playerid = ?")
        .bind(account_id)
        .fetch_one(&pool.read)
        .await
        .expect("fetch pk texp");
    assert_eq!(pk, 1);
    assert_eq!(texp, 9999);
}

#[tokio::test]
async fn test_login_success_updates_lastlogin_at_and_ip() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let data = ts_dream::data::loader::GameData::default();
    let service = ts_dream::battle::service::BattleService::default();

    // 1. Tao tai khoan moi
    let account_id = ts_dream::db::accounts::create(&pool, "mypass123", "secpass123")
        .await
        .expect("create account");

    // Kiem tra ban dau: lastlogin_at va lastloginip deu la NULL
    let (init_lastlogin, init_ip): (Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT lastlogin_at, lastloginip FROM accounts WHERE playerid = ?"
    )
    .bind(account_id)
    .fetch_one(&pool.read)
    .await
    .expect("fetch account init");
    assert_eq!(init_lastlogin, None);
    assert_eq!(init_ip, None);

    // 2. Dang nhap sai mat khau voi IP 1.2.3.4 -> LOGIN_WRONG_PASS, khong duoc luu DB
    let mut conn = Conn::with_peer_ip("1.2.3.4");
    let env = ServerEnv {
        pool: Some(&pool),
        repos: Some(&repos),
        hub: None,
        sender: None,
    };

    let mut wrong_payload = (account_id as u32).to_le_bytes().to_vec();
    wrong_payload.extend_from_slice(b"VN\xBC\x00wrongpass");
    let mut out = HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x01,
        sub: 9, // lenPw = 9
        payload: &wrong_payload,
    };
    handle_login(&mut ctx).await;
    assert!(out.outgoing.iter().any(|f| f.frame == spawn::LOGIN_WRONG_PASS));

    // Kiem tra DB van giu nguyen NULL
    let (after_wrong_login, after_wrong_ip): (Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT lastlogin_at, lastloginip FROM accounts WHERE playerid = ?"
    )
    .bind(account_id)
    .fetch_one(&pool.read)
    .await
    .expect("fetch after wrong login");
    assert_eq!(after_wrong_login, None);
    assert_eq!(after_wrong_ip, None);

    // 3. Dang nhap dung mat khau voi IP 192.168.1.50 -> thanh cong, luu lastlogin_at & lastloginip vao DB
    let mut conn = Conn::with_peer_ip("192.168.1.50");
    let mut correct_payload = (account_id as u32).to_le_bytes().to_vec();
    correct_payload.extend_from_slice(b"VN\xBC\x00mypass123");
    let mut out = HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x01,
        sub: 9, // lenPw = 9
        payload: &correct_payload,
    };
    handle_login(&mut ctx).await;

    // Vi tai khoan moi chua co nhan vat, he thong tra ve LOGIN_CREATE_CHAR
    assert!(out.outgoing.iter().any(|f| f.frame == spawn::LOGIN_CREATE_CHAR));
    assert!(conn.session.authed);

    // Xac nhan DB da cap nhat dung lastlogin_at va lastloginip = "192.168.1.50"
    let (t1, ip1): (Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT lastlogin_at, lastloginip FROM accounts WHERE playerid = ?"
    )
    .bind(account_id)
    .fetch_one(&pool.read)
    .await
    .expect("fetch after valid login 1");
    assert!(t1.is_some() && t1.unwrap() > 0);
    assert_eq!(ip1.as_deref(), Some("192.168.1.50"));

    // 4. Tao nhan vat cho tai khoan nay va dang nhap lai voi IP moi 10.0.0.8
    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 1,
        element: 2,
        map_id: 12001,
        map_x: 500,
        map_y: 500,
    };
    let char_id = repos
        .characters()
        .create(account_id, b"GiaCatLuong", &seed)
        .await
        .expect("create character");
    assert_eq!(char_id, account_id);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let mut conn = Conn::with_peer_ip("10.0.0.8");
    let mut out = HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env,
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x01,
        sub: 9,
        payload: &correct_payload,
    };
    handle_login(&mut ctx).await;

    // Co nhan vat -> phai tra ve cac frame payload in-game (chuyen canh)
    assert!(conn.session.logined);
    assert!(!out.outgoing.is_empty(), "Phai tra ve payload chuyen canh in-game");

    // Xac nhan DB cap nhat lastloginip = "10.0.0.8" va lastlogin_at moi hon t1
    let (t2, ip2): (Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT lastlogin_at, lastloginip FROM accounts WHERE playerid = ?"
    )
    .bind(account_id)
    .fetch_one(&pool.read)
    .await
    .expect("fetch after valid login 2");
    assert!(t2.is_some() && t2.unwrap() >= t1.unwrap());
    assert_eq!(ip2.as_deref(), Some("10.0.0.8"));
}



