//! Integration tests for Ticket 03:
//! - FestivalDatEncoder round-trip (7 records 1..=7 and 11 records 1..=11, VISCII 1.1 preservation)
//! - Login sync (17 39 + at most 1x 24 19 per season_seq) & character_festival_claims persistence

use std::sync::Arc;
use tokio::sync::RwLock;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::loaders::festival::{
    FestivalDatEncoder, FestivalDatLoader, FestivalDef, FestivalRecordInput,
};
use ts_dream::server::dispatcher::{dispatch, ServerEnv};
use ts_dream::server::handlers::festival::{festival_ui_trigger_frame, SeasonalNotice};
use ts_dream::server::seasonal_festival::{
    FESTIVAL_TET_NGUYEN_DAN, FESTIVAL_TET_NGUYEN_TIEU,
};
use ts_dream::server::session::Conn;
use ts_dream::state::AppState;
use ts_dream::web::server_control::ServerControl;

use crate::common::test_db::setup_test_db;

#[test]
fn test_festival_dat_encoder_roundtrip_7_and_11_records_and_max_limit() {
    let orig_bytes = std::fs::read("Data/Festival.Dat").expect("read Data/Festival.Dat");
    let orig_records = FestivalDatLoader::load(&orig_bytes).expect("parse original Festival.Dat");
    assert_eq!(orig_records.len(), 7);

    // 1. Patch keys 1..=7 from Data/Festival.Dat
    let patched_7 = FestivalDatEncoder::patch_festival_dat(&orig_bytes, &[])
        .expect("patch 7 records");
    assert_eq!(
        patched_7.len(),
        (7 + 3) * FestivalDatLoader::RECORD_SIZE,
        "7 records + dummy0 + keyA + keyB = 10 records"
    );

    let loaded_7 = FestivalDatLoader::load(&patched_7).expect("reload patched 7");
    assert_eq!(loaded_7.len(), 7);
    assert!(
        !FestivalDatLoader::has_lore_collisions(&loaded_7),
        "patched 7-record Festival.Dat must have no lore collisions"
    );
    for id in 1..=7u32 {
        let orig = &orig_records[&id];
        let patched = &loaded_7[&id];
        assert_eq!(patched.date_time, id as f64);
        assert_eq!(patched.lookup_key, Some(id));
        assert_eq!(patched.unique_lore_key, Some(id));
        assert_eq!(patched.word_flag, orig.word_flag);
        assert_eq!(patched.title, orig.title, "title mismatch at id={id}");
        assert_eq!(patched.text1, orig.text1, "text1 mismatch at id={id}");
        assert_eq!(patched.text2, orig.text2, "text2 mismatch at id={id}");
    }

    // 2. Append records 8..=11 (Thất Tịch, Trung Thu, Halloween, Giáng Sinh)
    let extra = vec![
        FestivalRecordInput {
            key: 8,
            word_flag: 0xB4C2,
            title: "Lễ Thất Tịch".to_string(),
            text1: "Ngày mùng 7 tháng 7 âm lịch, Ngưu Lang Chức Nữ gặp nhau.".to_string(),
            text2: "Cầu duyên và ngắm sao Chức Nữ trên bầu trời đêm.".to_string(),
        },
        FestivalRecordInput {
            key: 9,
            word_flag: 0xB4C2,
            title: "Tết Trung Thu".to_string(),
            text1: "Rằm tháng tám trăng tròn vành vạnh, trẻ em rước đèn phá cỗ.".to_string(),
            text2: "Thưởng thức bánh nướng, bánh dẻo bên gia đình.".to_string(),
        },
        FestivalRecordInput {
            key: 10,
            word_flag: 0xB4C2,
            title: "Halloween".to_string(),
            text1: "Đêm hội hóa trang cuối tháng mười với đèn bí ngô.".to_string(),
            text2: "Cho kẹo hay bị ghẹo cùng bạn bè khắp thành thị.".to_string(),
        },
        FestivalRecordInput {
            key: 11,
            word_flag: 0xB4C2,
            title: "Lễ Giáng Sinh".to_string(),
            text1: "Đêm đông an lành cùng cây thông và tiếng chuông ngân.".to_string(),
            text2: "Ông già Noel phát quà cho các lữ khách phương xa.".to_string(),
        },
    ];

    let patched_11 = FestivalDatEncoder::patch_festival_dat(&orig_bytes, &extra)
        .expect("patch 11 records");
    assert_eq!(
        patched_11.len(),
        (11 + 3) * FestivalDatLoader::RECORD_SIZE,
        "11 records + dummy0 + keyA + keyB = 14 records"
    );

    let loaded_11 = FestivalDatLoader::load(&patched_11).expect("reload patched 11");
    assert_eq!(loaded_11.len(), 11);
    assert!(
        !FestivalDatLoader::has_lore_collisions(&loaded_11),
        "patched 11-record Festival.Dat must have no lore collisions"
    );
    for id in 1..=11u32 {
        let rec = &loaded_11[&id];
        assert_eq!(rec.date_time, id as f64);
        assert_eq!(rec.lookup_key, Some(id));
        assert_eq!(rec.unique_lore_key, Some(id));
    }
    for (idx, exp) in extra.iter().enumerate() {
        let id = 8 + idx as u32;
        let rec = &loaded_11[&id];
        assert_eq!(rec.word_flag, exp.word_flag);
        assert_eq!(rec.title, exp.title);
        assert_eq!(rec.text1, exp.text1);
        assert_eq!(rec.text2, exp.text2);
    }

    // 3. Negative test: > MAX_RECORDS (366) must fail
    let too_many: Vec<FestivalDef> = (1..=367u32)
        .map(|id| FestivalDef {
            id,
            date_time: id as f64,
            word_flag: 0xB4C2,
            title: format!("F{id}"),
            ..Default::default()
        })
        .collect();
    let zero_key = [0u8; FestivalDatLoader::RECORD_SIZE];
    assert!(
        FestivalDatEncoder::encode(&too_many, &zero_key, &zero_key).is_err(),
        "encoding > 366 records must return Err"
    );
}

fn is_17_39(frame: &str) -> bool {
    frame.len() >= 12 && &frame[8..12] == "1739"
}

fn is_24_19(frame: &str) -> bool {
    frame.len() >= 12 && &frame[8..12] == "2419"
}

#[tokio::test]
async fn test_login_sync_emits_17_39_and_max_1_lore_24_19_per_season_seq() {
    let db = setup_test_db().await;
    let orig_bytes = std::fs::read("Data/Festival.Dat").expect("read Data/Festival.Dat");
    let patched_bytes =
        FestivalDatEncoder::patch_festival_dat(&orig_bytes, &[]).expect("patch Festival.Dat");

    let mut data = GameData::default();
    data.festivals = FestivalDatLoader::load(&patched_bytes).expect("load patched Festival.Dat");
    let data_arc = Arc::new(data);

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;
    let control = Arc::new(ServerControl::new(
        0,
        app,
        Some(data_arc.clone()),
        Some(db.clone()),
    ));
    let service = BattleService::default();
    let env = ServerEnv {
        pool: Some(&db),
        repos: None,
        hub: Some(&control),
        sender: None,
    };

    let login_complete_pkt = [0xF4, 0x44, 0x02, 0x00, 0x25, 0x01];

    // 1. Negative: all festivals Idle -> no 17 39, no 24 19
    let mut conn = Conn::default();
    conn.session.id = 5001;
    conn.session.map_id = 12001;
    let out_idle = dispatch(&mut conn, &login_complete_pkt, &data_arc, &service, &env).await;
    assert!(
        !out_idle.outgoing.iter().any(|f| is_17_39(&f.frame)),
        "idle festivals must not emit 17 39 on login"
    );
    assert!(
        !out_idle.outgoing.iter().any(|f| is_24_19(&f.frame)),
        "idle festivals must not emit 24 19 on login"
    );

    // 2. Start 2 eligible festivals (tet_nguyen_dan key=1, tet_nguyen_tieu key=2)
    control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&db))
        .await
        .unwrap();
    control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_TIEU, true, Some(&db))
        .await
        .unwrap();

    // First login: emits BOTH 17 39 notices, and AT MOST 1x 24 19 (key=1)
    let mut conn1 = Conn::default();
    conn1.session.id = 5001;
    conn1.session.map_id = 12001;
    let out1 = dispatch(&mut conn1, &login_complete_pkt, &data_arc, &service, &env).await;
    let notices1: Vec<&str> = out1
        .outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .filter(|f| is_17_39(f))
        .collect();
    assert_eq!(notices1.len(), 2, "expected 2 active 17 39 notices");
    assert!(notices1.contains(&SeasonalNotice::TetCountdownStart.to_frame().frame.as_str()));
    assert!(notices1.contains(&SeasonalNotice::LanternFestival(1).to_frame().frame.as_str()));

    let lores1: Vec<&str> = out1
        .outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .filter(|f| is_24_19(f))
        .collect();
    assert_eq!(lores1, vec![festival_ui_trigger_frame(1).as_str()]);

    // Verify claim persisted in character_festival_claims for season-1
    let claim_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM character_festival_claims
         WHERE playerid = ? AND festival_id = ? AND period_key = ? AND claim_type = ?",
    )
    .bind(5001i64)
    .bind(FESTIVAL_TET_NGUYEN_DAN)
    .bind("season-1")
    .bind("lore_shown")
    .fetch_one(&db.read)
    .await
    .unwrap();
    assert_eq!(claim_count, 1);

    // Second login in same season-1: emits 17 39 for both, and 24 19 for the second unclaimed festival (key=2)
    let mut conn2 = Conn::default();
    conn2.session.id = 5001;
    conn2.session.map_id = 12001;
    let out2 = dispatch(&mut conn2, &login_complete_pkt, &data_arc, &service, &env).await;
    assert_eq!(
        out2.outgoing.iter().filter(|f| is_17_39(&f.frame)).count(),
        2
    );
    let lores2: Vec<&str> = out2
        .outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .filter(|f| is_24_19(f))
        .collect();
    assert_eq!(lores2, vec![festival_ui_trigger_frame(2).as_str()]);

    // Third login in same season-1: all active lore_shown claimed -> 0x 24 19
    let mut conn3 = Conn::default();
    conn3.session.id = 5001;
    conn3.session.map_id = 12001;
    let out3 = dispatch(&mut conn3, &login_complete_pkt, &data_arc, &service, &env).await;
    assert_eq!(
        out3.outgoing.iter().filter(|f| is_17_39(&f.frame)).count(),
        2
    );
    assert_eq!(
        out3.outgoing.iter().filter(|f| is_24_19(&f.frame)).count(),
        0,
        "relogging in same season_seq after all claimed must NOT re-emit 24 19"
    );

    // 3. Stop then Start tet_nguyen_dan -> season_seq becomes 2 -> next login receives 24 19 once for season-2
    control
        .festivals
        .stop_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&db))
        .await
        .unwrap();
    let restarted = control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_DAN, true, Some(&db))
        .await
        .unwrap();
    assert_eq!(restarted.season_seq, 2);

    let mut conn4 = Conn::default();
    conn4.session.id = 5001;
    conn4.session.map_id = 12001;
    let out4 = dispatch(&mut conn4, &login_complete_pkt, &data_arc, &service, &env).await;
    let lores4: Vec<&str> = out4
        .outgoing
        .iter()
        .map(|f| f.frame.as_str())
        .filter(|f| is_24_19(f))
        .collect();
    assert_eq!(lores4, vec![festival_ui_trigger_frame(1).as_str()]);

    // Relogging again in season-2 -> 0x 24 19
    let mut conn5 = Conn::default();
    conn5.session.id = 5001;
    conn5.session.map_id = 12001;
    let out5 = dispatch(&mut conn5, &login_complete_pkt, &data_arc, &service, &env).await;
    assert_eq!(
        out5.outgoing.iter().filter(|f| is_24_19(&f.frame)).count(),
        0
    );
}

#[tokio::test]
async fn test_login_sync_colliding_festival_on_unpatched_dat_emits_17_39_without_24_19() {
    let db = setup_test_db().await;
    let orig_bytes = std::fs::read("Data/Festival.Dat").expect("read Data/Festival.Dat");

    let mut data = GameData::default();
    data.festivals = FestivalDatLoader::load(&orig_bytes).expect("load unpatched Festival.Dat");
    let data_arc = Arc::new(data);

    let app = Arc::new(RwLock::new(AppState::new(1)));
    app.write().await.running = true;
    let control = Arc::new(ServerControl::new(
        0,
        app,
        Some(data_arc.clone()),
        Some(db.clone()),
    ));
    control
        .festivals
        .start_festival(FESTIVAL_TET_NGUYEN_TIEU, true, Some(&db))
        .await
        .unwrap();

    let service = BattleService::default();
    let env = ServerEnv {
        pool: Some(&db),
        repos: None,
        hub: Some(&control),
        sender: None,
    };

    let mut conn = Conn::default();
    conn.session.id = 5002;
    conn.session.map_id = 12001;
    let login_complete_pkt = [0xF4, 0x44, 0x02, 0x00, 0x25, 0x01];
    let out = dispatch(&mut conn, &login_complete_pkt, &data_arc, &service, &env).await;

    assert!(
        out.outgoing
            .iter()
            .any(|f| f.frame == SeasonalNotice::LanternFestival(1).to_frame().frame),
        "active colliding festival must still emit 17 39 on login"
    );
    assert!(
        !out.outgoing.iter().any(|f| is_24_19(&f.frame)),
        "colliding festival on unpatched Festival.Dat must NOT emit 24 19 on login"
    );
}
