//! Shared helpers for integration tests (`mod common;` in each test file).
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::session::Conn;

pub mod test_db;

pub fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

/// Shared read-only `GameData` loaded once per test binary.
///
/// Avoids re-parsing ~23 MB of static game data across dozens of tests.
pub fn shared_game_data() -> &'static Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| {
        Arc::new(GameData::load(&data_dir()).expect("GameData::load must succeed"))
    })
}

/// Build an `OpcodeCtx` for tests that drive a handler directly.
pub fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a BattleService,
    out: &'a mut HandleOutcome,
    opcode: u8,
    sub: u8,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        data,
        service,
        out,
        opcode,
        sub,
        payload,
        decoded: &[],
        env: ServerEnv::none(),
    }
}

/// Process-wide lock for tests that temporarily mutate `std::env`.
pub fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
