//! Integration tests for Ticket 05:
//! - Seasonal battle bonus drops (anti-farm: min_enemy_level, DotNetRandom roll, UTC+7 daily_drop_cap)
//! - Festival EXP multiplier (`exp_multiplier_pct`) combined with `per_exp`, `FGod`, and `FGhost`
//! - Negative invariants: `PKBattle`, below `min_enemy_level`, `daily_drop_cap` reached, full bag

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use ts_dream::battle::combat_formula::apply_exp_multipliers;
use ts_dream::battle::construction::BattleKind;
use ts_dream::battle::runner::{BattleAction, BattleCommand};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Item, Npc};
use ts_dream::server::seasonal_festival::{
    drop_claim_type, get_festival_claim_count, utc7_date_key, FestivalProfile,
    SeasonalBonusDrop, SeasonalFestivalManager, FESTIVAL_DOAN_NGO, FESTIVAL_HALLOWEEN,
    FESTIVAL_TET_NGUYEN_DAN, FESTIVAL_TET_NGUYEN_TIEU,
};
use ts_dream::server::session::{
    lock_online_sessions, registry_get, registry_register_for_login, InventoryItem, Session,
};

use crate::common::test_db::setup_test_db;

fn decode_hex(hex_str: &str) -> Vec<u8> {
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).unwrap())
        .collect()
}

fn content_of(frame: &str) -> Vec<u8> {
    let raw = decode_hex(frame);
    assert_eq!(&raw[0..2], &[0xF4, 0x44]);
    raw[4..].to_vec()
}

fn is_add_item_frame_for(frame: &str, expected_item_id: u16) -> bool {
    if !frame.starts_with("F4440E001706") {
        return false;
    }
    let content = content_of(frame);
    content.len() >= 4 && u16::from_le_bytes([content[2], content[3]]) == expected_item_id
}

fn build_battle_test_data(enemy_id: i64, enemy_lv: i64) -> Arc<GameData> {
    let mut data = GameData::default();
    data.npcs.insert(
        enemy_id,
        Npc {
            id: enemy_id,
            name: b"SeasonalTestMob".to_vec(),
            lv: enemy_lv,
            hp: 1,
            sp: 100,
            ..Default::default()
        },
    );
    data.items.insert(
        46335,
        Item {
            id: 46335,
            name: b"Bao li xi".to_vec(),
            loai: 1,
            ..Default::default()
        },
    );
    Arc::new(data)
}

async fn insert_test_player(pool: &ts_dream::db::pool::DbPool, player_id: u32, name: &str) {
    sqlx::query(
        "INSERT OR REPLACE INTO characters (playerid, name, mapid) VALUES (?, ?, 12001)",
    )
    .bind(i64::from(player_id))
    .bind(name)
    .execute(&pool.write)
    .await
    .unwrap();
}

async fn run_one_turn_win(
    service: &BattleService,
    sess: &Arc<RwLock<Session>>,
    enemy_id: i64,
    kind: BattleKind,
) {
    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_with_kind(&mut guard, enemy_id, 0, 1, kind);
        assert_ne!(bid, 0, "battle must start");
    }
    let cmd = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::NormalAttack,
    };
    let snapshot = sess.read().await.clone();
    assert!(service.submit_command(&snapshot, cmd));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(2000);
    loop {
        if sess.read().await.battle_id == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() <= deadline,
            "battle did not finish in time"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

#[test]
fn test_seasonal_bonus_drop_profiles_and_utc7_date_key() {
    let profiles = SeasonalFestivalManager::default_profiles();
    let by_id: HashMap<&str, &FestivalProfile> =
        profiles.iter().map(|p| (p.id.as_str(), p)).collect();

    assert!(
        by_id[FESTIVAL_TET_NGUYEN_DAN]
            .bonus_drops
            .iter()
            .any(|d| d.item_id == 46335),
        "tet_nguyen_dan must include 46335"
    );
    assert!(
        by_id[FESTIVAL_TET_NGUYEN_DAN]
            .bonus_drops
            .iter()
            .any(|d| d.item_id == 46338),
        "tet_nguyen_dan must include 46338"
    );
    assert_eq!(by_id[FESTIVAL_TET_NGUYEN_TIEU].exp_multiplier_pct, Some(200));
    assert!(
        by_id[FESTIVAL_DOAN_NGO]
            .bonus_drops
            .iter()
            .any(|d| d.item_id == 46220),
        "doan_ngo must include 46220"
    );
    assert!(
        by_id[FESTIVAL_HALLOWEEN]
            .bonus_drops
            .iter()
            .any(|d| d.item_id == 46073),
        "halloween must include 46073"
    );

    // 1970-01-01 16:59:59 UTC == 1970-01-01 23:59:59 UTC+7
    assert_eq!(utc7_date_key(16 * 3600 + 3599), "1970-01-01");
    // 1970-01-01 17:00:00 UTC == 1970-01-02 00:00:00 UTC+7
    assert_eq!(utc7_date_key(17 * 3600), "1970-01-02");

    // EXP multiplier parity with FGod (1.5x) and FGhost (0.5x)
    assert_eq!(apply_exp_multipliers(100, 0, 0, 200), 200);
    assert_eq!(apply_exp_multipliers(100, 1, 0, 200), 300);
    assert_eq!(apply_exp_multipliers(100, 0, 1, 200), 100);
    assert_eq!(apply_exp_multipliers(100, 1, 1, 200), 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_pve_normal_victory_grants_seasonal_drop_17_06_persists_db_and_applies_exp_multiplier() {
    let pool = setup_test_db().await;
    let player_id = 8801u32;
    insert_test_player(&pool, player_id, "DropWinner").await;

    let festivals = Arc::new(SeasonalFestivalManager::new(vec![FestivalProfile {
        id: FESTIVAL_TET_NGUYEN_DAN.to_string(),
        name: "Tết Nguyên Đán".to_string(),
        exp_multiplier_pct: Some(200),
        bonus_drops: vec![SeasonalBonusDrop {
            item_id: 46335,
            drop_rate_pct: 100,
            min_enemy_level: 10,
            daily_drop_cap: 3,
        }],
        ..Default::default()
    }]));
    festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&pool))
        .await
        .unwrap();

    let data = build_battle_test_data(9101, 15);
    // Baseline battle (no festival, no god/ghost) to capture exact base EXP.
    let mut base_service = BattleService::new(Arc::clone(&data));
    base_service.set_input_timeout(std::time::Duration::from_millis(40));
    let base_sess = Arc::new(RwLock::new(Session {
        id: 8800,
        map_id: 12001,
        id_leader: 8800,
        level: 10,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        texp: 0,
        ..Default::default()
    }));
    registry_register_for_login(&base_sess.read().await.clone());
    let _base_rx = base_service.register(8800, Arc::clone(&base_sess));
    run_one_turn_win(&base_service, &base_sess, 9101, BattleKind::Normal).await;
    let base_exp = base_sess.read().await.texp;
    assert!(base_exp > 0, "baseline PvE victory must grant >0 EXP");
    lock_online_sessions().remove(&8800);

    let mut service = BattleService::new(data)
        .with_pool(pool.clone())
        .with_festivals(Arc::clone(&festivals));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: player_id,
        db_character_id: i64::from(player_id),
        map_id: 12001,
        id_leader: player_id,
        level: 10,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        god: 1,
        ghost: 0,
        texp: 0,
        ..Default::default()
    }));
    registry_register_for_login(&sess.read().await.clone());
    let mut rx = service.register(i64::from(player_id), Arc::clone(&sess));

    run_one_turn_win(&service, &sess, 9101, BattleKind::Normal).await;

    let mut got_drop_frame = false;
    while let Ok(frame) = rx.try_recv() {
        if is_add_item_frame_for(&frame, 46335) {
            got_drop_frame = true;
        }
    }
    assert!(got_drop_frame, "must emit 17 06 frame for seasonal drop 46335");

    // Verify EXP: base_exp * FGod 1.5 * Festival 2.0
    let expected_exp = u32::try_from(apply_exp_multipliers(i64::from(base_exp), 1, 0, 200)).unwrap();
    assert_eq!(sess.read().await.texp, expected_exp);

    // Verify homdo in session, registry, and SQLite
    assert!(
        sess.read().await.homdo.iter().any(|i| i.id == 46335 && i.count == 1),
        "session homdo must contain 46335"
    );
    let reg = registry_get(player_id).expect("registry must have player");
    assert!(
        reg.homdo.iter().any(|i| i.id == 46335 && i.count == 1),
        "registry homdo must contain 46335"
    );
    assert_eq!(reg.texp, expected_exp);

    let db_deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1000);
    let db_item_count: i64 = loop {
        let count: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(quantity), 0) FROM inventories WHERE playerid = ? AND itemid = 46335",
        )
        .bind(i64::from(player_id))
        .fetch_one(&pool.read)
        .await
        .unwrap();
        if count == 1 || tokio::time::Instant::now() >= db_deadline {
            break count;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };
    assert_eq!(db_item_count, 1, "SQLite homdo must persist seasonal drop");

    let today = utc7_date_key(chrono::Utc::now().timestamp());
    let claim_count = get_festival_claim_count(
        &pool,
        player_id,
        FESTIVAL_TET_NGUYEN_DAN,
        &today,
        &drop_claim_type(46335),
    )
    .await
    .unwrap();
    assert_eq!(claim_count, 1, "character_festival_claims must increment to 1");

    lock_online_sessions().remove(&player_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_pk_battle_never_drops_seasonal_items_and_never_applies_seasonal_exp() {
    let pool = setup_test_db().await;
    let player_id = 8802u32;
    insert_test_player(&pool, player_id, "PkNoSeasonal").await;

    let festivals = Arc::new(SeasonalFestivalManager::new(vec![FestivalProfile {
        id: FESTIVAL_TET_NGUYEN_DAN.to_string(),
        name: "Tết Nguyên Đán".to_string(),
        exp_multiplier_pct: Some(200),
        bonus_drops: vec![SeasonalBonusDrop {
            item_id: 46335,
            drop_rate_pct: 100,
            min_enemy_level: 1,
            daily_drop_cap: 5,
        }],
        ..Default::default()
    }]));
    festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&pool))
        .await
        .unwrap();

    let data = build_battle_test_data(9102, 50);
    let mut service = BattleService::new(data)
        .with_pool(pool.clone())
        .with_festivals(Arc::clone(&festivals));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: player_id,
        db_character_id: i64::from(player_id),
        map_id: 12001,
        id_leader: player_id,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        texp: 0,
        ..Default::default()
    }));
    registry_register_for_login(&sess.read().await.clone());
    let mut rx = service.register(i64::from(player_id), Arc::clone(&sess));

    run_one_turn_win(&service, &sess, 9102, BattleKind::Pk).await;

    while let Ok(frame) = rx.try_recv() {
        assert!(
            !is_add_item_frame_for(&frame, 46335),
            "PKBattle must NEVER emit 17 06 for seasonal drop"
        );
    }
    assert_eq!(sess.read().await.texp, 0, "PKBattle must NEVER grant EXP");
    assert!(
        !sess.read().await.homdo.iter().any(|i| i.id == 46335),
        "PKBattle must NEVER drop seasonal item"
    );

    let today = utc7_date_key(chrono::Utc::now().timestamp());
    let claim_count = get_festival_claim_count(
        &pool,
        player_id,
        FESTIVAL_TET_NGUYEN_DAN,
        &today,
        &drop_claim_type(46335),
    )
    .await
    .unwrap();
    assert_eq!(claim_count, 0);

    lock_online_sessions().remove(&player_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_enemy_below_min_level_never_drops_seasonal_items() {
    let pool = setup_test_db().await;
    let player_id = 8803u32;
    insert_test_player(&pool, player_id, "LowMobNoDrop").await;

    let festivals = Arc::new(SeasonalFestivalManager::new(vec![FestivalProfile {
        id: FESTIVAL_TET_NGUYEN_DAN.to_string(),
        name: "Tết Nguyên Đán".to_string(),
        exp_multiplier_pct: None,
        bonus_drops: vec![SeasonalBonusDrop {
            item_id: 46335,
            drop_rate_pct: 100,
            min_enemy_level: 20,
            daily_drop_cap: 5,
        }],
        ..Default::default()
    }]));
    festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&pool))
        .await
        .unwrap();

    let data = build_battle_test_data(9103, 10);
    let mut service = BattleService::new(data)
        .with_pool(pool.clone())
        .with_festivals(Arc::clone(&festivals));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: player_id,
        db_character_id: i64::from(player_id),
        map_id: 12001,
        id_leader: player_id,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        ..Default::default()
    }));
    registry_register_for_login(&sess.read().await.clone());
    let mut rx = service.register(i64::from(player_id), Arc::clone(&sess));

    run_one_turn_win(&service, &sess, 9103, BattleKind::Normal).await;

    while let Ok(frame) = rx.try_recv() {
        assert!(
            !is_add_item_frame_for(&frame, 46335),
            "enemy below min_enemy_level must NEVER drop seasonal item"
        );
    }
    assert!(!sess.read().await.homdo.iter().any(|i| i.id == 46335));

    lock_online_sessions().remove(&player_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_daily_drop_cap_enforced_per_utc7_day_and_resets_next_day() {
    let pool = setup_test_db().await;
    let player_id = 8804u32;
    insert_test_player(&pool, player_id, "DailyCapTester").await;

    let festivals = Arc::new(SeasonalFestivalManager::new(vec![FestivalProfile {
        id: FESTIVAL_TET_NGUYEN_DAN.to_string(),
        name: "Tết Nguyên Đán".to_string(),
        exp_multiplier_pct: None,
        bonus_drops: vec![SeasonalBonusDrop {
            item_id: 46335,
            drop_rate_pct: 100,
            min_enemy_level: 10,
            daily_drop_cap: 1,
        }],
        ..Default::default()
    }]));
    festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&pool))
        .await
        .unwrap();

    let data = build_battle_test_data(9104, 15);
    let mut service = BattleService::new(data)
        .with_pool(pool.clone())
        .with_festivals(Arc::clone(&festivals));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: player_id,
        db_character_id: i64::from(player_id),
        map_id: 12001,
        id_leader: player_id,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        ..Default::default()
    }));
    registry_register_for_login(&sess.read().await.clone());
    let mut rx = service.register(i64::from(player_id), Arc::clone(&sess));

    // Battle 1: drops 1st item
    run_one_turn_win(&service, &sess, 9104, BattleKind::Normal).await;
    let mut count_frames = 0;
    while let Ok(frame) = rx.try_recv() {
        if is_add_item_frame_for(&frame, 46335) {
            count_frames += 1;
        }
    }
    assert_eq!(count_frames, 1);

    // Battle 2 on same UTC+7 day: blocked by daily_drop_cap = 1
    run_one_turn_win(&service, &sess, 9104, BattleKind::Normal).await;
    while let Ok(frame) = rx.try_recv() {
        assert!(
            !is_add_item_frame_for(&frame, 46335),
            "must NOT drop after reaching daily_drop_cap on same UTC+7 day"
        );
    }
    let total_in_bag: u32 = sess
        .read()
        .await
        .homdo
        .iter()
        .filter(|i| i.id == 46335)
        .map(|i| u32::from(i.count))
        .sum();
    assert_eq!(total_in_bag, 1);

    // Simulate advancing to next UTC+7 day by moving yesterday's claim to yesterday's date key
    let now = chrono::Utc::now().timestamp();
    let today = utc7_date_key(now);
    let yesterday = utc7_date_key(now - 86_400);
    sqlx::query(
        "UPDATE character_festival_claims SET period_key = ? WHERE playerid = ? AND period_key = ?",
    )
    .bind(&yesterday)
    .bind(i64::from(player_id))
    .bind(&today)
    .execute(&pool.write)
    .await
    .unwrap();

    // Battle 3 on new UTC+7 day: drops again!
    run_one_turn_win(&service, &sess, 9104, BattleKind::Normal).await;
    let mut got_next_day = false;
    while let Ok(frame) = rx.try_recv() {
        if is_add_item_frame_for(&frame, 46335) {
            got_next_day = true;
        }
    }
    assert!(got_next_day, "must drop again on next UTC+7 day");
    let total_in_bag_after: u32 = sess
        .read()
        .await
        .homdo
        .iter()
        .filter(|i| i.id == 46335)
        .map(|i| u32::from(i.count))
        .sum();
    assert_eq!(total_in_bag_after, 2);

    lock_online_sessions().remove(&player_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_full_bag_safely_skips_seasonal_drop_without_incrementing_cap_or_failing_battle() {
    let pool = setup_test_db().await;
    let player_id = 8805u32;
    insert_test_player(&pool, player_id, "FullBagPlayer").await;

    let festivals = Arc::new(SeasonalFestivalManager::new(vec![FestivalProfile {
        id: FESTIVAL_TET_NGUYEN_DAN.to_string(),
        name: "Tết Nguyên Đán".to_string(),
        exp_multiplier_pct: None,
        bonus_drops: vec![SeasonalBonusDrop {
            item_id: 46335,
            drop_rate_pct: 100,
            min_enemy_level: 10,
            daily_drop_cap: 2,
        }],
        ..Default::default()
    }]));
    festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&pool))
        .await
        .unwrap();

    let data = build_battle_test_data(9105, 15);
    let mut service = BattleService::new(data)
        .with_pool(pool.clone())
        .with_festivals(Arc::clone(&festivals));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let mut full_bag = Vec::new();
    for slot in 1..=25u8 {
        full_bag.push(InventoryItem {
            slot,
            id: 9000 + u16::from(slot),
            count: 1,
            ..Default::default()
        });
    }

    let sess = Arc::new(RwLock::new(Session {
        id: player_id,
        db_character_id: i64::from(player_id),
        map_id: 12001,
        id_leader: player_id,
        level: 50,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        homdo: full_bag,
        ..Default::default()
    }));
    registry_register_for_login(&sess.read().await.clone());
    let mut rx = service.register(i64::from(player_id), Arc::clone(&sess));

    run_one_turn_win(&service, &sess, 9105, BattleKind::Normal).await;

    while let Ok(frame) = rx.try_recv() {
        assert!(
            !is_add_item_frame_for(&frame, 46335),
            "full bag must NOT emit 17 06"
        );
    }

    // Battle settlement still succeeded (EXP granted & battle_id cleared)
    assert_eq!(sess.read().await.battle_id, 0);
    assert!(sess.read().await.texp > 0, "battle settlement must still grant EXP");
    assert!(!sess.read().await.homdo.iter().any(|i| i.id == 46335));

    // daily_drop_cap MUST NOT be incremented when bag was full
    let today = utc7_date_key(chrono::Utc::now().timestamp());
    let claim_count = get_festival_claim_count(
        &pool,
        player_id,
        FESTIVAL_TET_NGUYEN_DAN,
        &today,
        &drop_claim_type(46335),
    )
    .await
    .unwrap();
    assert_eq!(claim_count, 0, "full bag must NOT increment daily_drop_cap");

    lock_online_sessions().remove(&player_id);
}
