//! Migration 0004 (`characters.color`) roundtrip & normalization tests.

use ts_dream::db::modern::sqlite::session::{normalize_color_hex, DEFAULT_COLOR_HEX};
use ts_dream::db::modern::sqlite::SqliteRepositories;
use ts_dream::db::modern::traits::CharacterSeed;
use ts_dream::db::persist::update_player_text;
use ts_dream::server::session::Session;

use crate::common::test_db::setup_test_db;

#[tokio::test]
async fn color_roundtrip_create_load_save_and_update_text() {
    let pool = setup_test_db().await;
    let repos = SqliteRepositories::new(pool.clone());
    let acc = ts_dream::db::accounts::create(&pool, "pass1111", "pass2222")
        .await
        .expect("create account");

    let mut session = Session::new();
    session.id = acc as u32;
    session.color = "a1b2c3d4e5f60718".to_string();
    let seed = CharacterSeed {
        level: 1,
        sex: 1,
        hair: 5,
        element: 2,
        map_id: 10817,
        map_x: 442,
        map_y: 758,
    };

    repos
        .sessions()
        .create_and_seed(acc, b"ColorHero", &seed, &mut session, b"", b"")
        .await
        .expect("create_and_seed with color");

    // 1. Load normalizes to 16 uppercase hex chars.
    let mut loaded = Session::new();
    assert!(repos.sessions().load(acc, &mut loaded).await.unwrap());
    assert_eq!(loaded.color, "A1B2C3D4E5F60718");

    // 2. save_in_tx persists updated color; invalid/empty normalizes to DEFAULT_COLOR_HEX.
    loaded.color = "11223344aabbccdd".to_string();
    {
        let mut tx = pool.write.begin().await.unwrap();
        repos
            .sessions()
            .save_in_tx(&mut tx, &loaded)
            .await
            .expect("save_in_tx");
        tx.commit().await.unwrap();
    }

    let mut after_save = Session::new();
    assert!(repos.sessions().load(acc, &mut after_save).await.unwrap());
    assert_eq!(after_save.color, "11223344AABBCCDD");

    // 3. update_player_text updates `color` and normalizes.
    update_player_text(&pool.write, acc, "color", "deadbeef00112233").await;
    let mut after_text_update = Session::new();
    assert!(repos.sessions().load(acc, &mut after_text_update).await.unwrap());
    assert_eq!(after_text_update.color, "DEADBEEF00112233");

    // 4. Empty or invalid color falls back to 16 zeroes in DB and helper.
    update_player_text(&pool.write, acc, "Color", "not-hex").await;
    let mut after_invalid = Session::new();
    assert!(repos.sessions().load(acc, &mut after_invalid).await.unwrap());
    assert_eq!(after_invalid.color, DEFAULT_COLOR_HEX);

    assert_eq!(normalize_color_hex(""), DEFAULT_COLOR_HEX);
    assert_eq!(normalize_color_hex("1234"), DEFAULT_COLOR_HEX);
    assert_eq!(normalize_color_hex("zzzzzzzzzzzzzzzz"), DEFAULT_COLOR_HEX);
    assert_eq!(normalize_color_hex("0123456789abcdef"), "0123456789ABCDEF");
}
