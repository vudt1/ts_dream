//! Ticket 18 tests: Startup SQLite schema sanity-check (`db::pool::verify_schema`).

use sqlx::sqlite::SqlitePoolOptions;
use ts_dream::db::pool::verify_schema;

async fn memory_pool() -> sqlx::SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite")
}

#[tokio::test]
async fn ticket18_verify_schema_rejects_empty_zero_table_db_with_adr0004_hint() {
    let pool = memory_pool().await;
    let err = verify_schema(&pool)
        .await
        .expect_err("0-table DB must fail startup schema sanity-check");
    let msg = err.to_string();
    assert!(
        msg.contains("0 table(s)") && msg.contains("ADR 0004"),
        "expected clear 0-table + ADR 0004 message, got: {msg}"
    );
    assert!(
        msg.contains("0001_init.sql") && msg.contains("0002_eve_quest_persistence.sql"),
        "expected manual migration instructions, got: {msg}"
    );
}

#[tokio::test]
async fn ticket18_verify_schema_rejects_0001_only_missing_0002() {
    let pool = memory_pool().await;
    let sql_0001 = include_str!("../../migrations/0001_init.sql");
    sqlx::raw_sql(sql_0001)
        .execute(&pool)
        .await
        .expect("apply 0001_init.sql");

    let err = verify_schema(&pool)
        .await
        .expect_err("DB missing 0002_eve_quest_persistence.sql must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("savemap_id") && msg.contains("0002_eve_quest_persistence.sql"),
        "expected savemap_id / 0002 error message, got: {msg}"
    );
}

#[tokio::test]
async fn ticket18_verify_schema_rejects_missing_0004_color_column() {
    let pool = memory_pool().await;
    let sql_0001 = include_str!("../../migrations/0001_init.sql");
    let sql_0002 = include_str!("../../migrations/0002_eve_quest_persistence.sql");
    sqlx::raw_sql(sql_0001)
        .execute(&pool)
        .await
        .expect("apply 0001_init.sql");
    sqlx::raw_sql(sql_0002)
        .execute(&pool)
        .await
        .expect("apply 0002_eve_quest_persistence.sql");

    let err = verify_schema(&pool)
        .await
        .expect_err("DB missing 0004_character_color.sql must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("color") && msg.contains("0004_character_color.sql"),
        "expected color / 0004 error message, got: {msg}"
    );
}

#[tokio::test]
async fn ticket18_verify_schema_accepts_full_0001_0002_and_0004_schema() {
    let pool = memory_pool().await;
    let sql_0001 = include_str!("../../migrations/0001_init.sql");
    let sql_0002 = include_str!("../../migrations/0002_eve_quest_persistence.sql");
    let sql_0004 = include_str!("../../migrations/0004_character_color.sql");
    sqlx::raw_sql(sql_0001)
        .execute(&pool)
        .await
        .expect("apply 0001_init.sql");
    sqlx::raw_sql(sql_0002)
        .execute(&pool)
        .await
        .expect("apply 0002_eve_quest_persistence.sql");
    sqlx::raw_sql(sql_0004)
        .execute(&pool)
        .await
        .expect("apply 0004_character_color.sql");

    verify_schema(&pool)
        .await
        .expect("full 0001 + 0002 + 0004 schema must pass verify_schema");
}
