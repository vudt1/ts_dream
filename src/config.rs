//! Configuration (Chapter 8).
//!
//! A TOML file (`ts_dream.toml`) with `TS_` env overrides, loaded once at
//! boot. The SQLite-era keys (`account_db_path`, `member_dir`,
//! `template_db_path`) are removed and rejected.

use crate::error::{Result, TsError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub game_port: u16,
    pub web_port: u16,
    pub data_dir: PathBuf,
    pub database_url: String,
    pub perexp_default: u32,
    pub db_auto_create: bool,
    /// Periodic SQLite WAL checkpoint interval in seconds (default: 300s / 5 mins).
    pub wal_checkpoint_interval_secs: u64,
    /// SQLite cache size in KiB per connection (default: 64000 KiB / 64 MB).
    pub sqlite_cache_size_kb: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_port: 6414,
            web_port: 8090,
            data_dir: PathBuf::from("./Data"),
            database_url: "sqlite://DB/ts_dream.db".to_string(),
            perexp_default: 0,
            db_auto_create: true,
            wal_checkpoint_interval_secs: 300,
            sqlite_cache_size_kb: 64000,
        }
    }
}

pub fn parse_u16(s: &str) -> std::result::Result<u16, String> {
    s.trim()
        .parse::<u16>()
        .map_err(|e| format!("invalid u16: {e}"))
}
fn parse_u32(s: &str) -> std::result::Result<u32, String> {
    s.trim()
        .parse::<u32>()
        .map_err(|e| format!("invalid u32: {e}"))
}
fn parse_u64(s: &str) -> std::result::Result<u64, String> {
    s.trim()
        .parse::<u64>()
        .map_err(|e| format!("invalid u64: {e}"))
}
pub fn parse_i64(s: &str) -> std::result::Result<i64, String> {
    s.trim()
        .parse::<i64>()
        .map_err(|e| format!("invalid i64: {e}"))
}
pub fn parse_bool(s: &str) -> std::result::Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        other => Err(format!("invalid bool: {other:?}")),
    }
}

/// Apply a `TS_` env var override if present, else keep the TOML value.
fn env_override<T, F>(key: &str, toml: T, parse: F) -> std::result::Result<T, String>
where
    F: Fn(&str) -> std::result::Result<T, String>,
{
    match std::env::var(key) {
        Ok(raw) => parse(&raw),
        Err(_) => Ok(toml),
    }
}

impl Config {
    /// Load `ts_dream.toml` from the current directory, then apply `TS_` env
    /// overrides. Missing file -> defaults.
    pub fn load() -> Result<Self> {
        let path = PathBuf::from("ts_dream.toml");
        let mut cfg = if path.exists() {
            from_file(&path)?
        } else {
            Config::default()
        };

        cfg.game_port = env_override("TS_GAME_PORT", cfg.game_port, parse_u16)
            .map_err(|e| TsError::Config(format!("TS_GAME_PORT: {e}")))?;
        cfg.web_port = env_override("TS_WEB_PORT", cfg.web_port, parse_u16)
            .map_err(|e| TsError::Config(format!("TS_WEB_PORT: {e}")))?;
        cfg.perexp_default = env_override("TS_PEREXP_DEFAULT", cfg.perexp_default, parse_u32)
            .map_err(|e| TsError::Config(format!("TS_PEREXP_DEFAULT: {e}")))?;
        cfg.db_auto_create = env_override("TS_DB_AUTO_CREATE", cfg.db_auto_create, parse_bool)
            .map_err(|e| TsError::Config(format!("TS_DB_AUTO_CREATE: {e}")))?;
        cfg.wal_checkpoint_interval_secs = env_override(
            "TS_WAL_CHECKPOINT_INTERVAL_SECS",
            cfg.wal_checkpoint_interval_secs,
            parse_u64,
        )
        .map_err(|e| TsError::Config(format!("TS_WAL_CHECKPOINT_INTERVAL_SECS: {e}")))?;
        cfg.sqlite_cache_size_kb = env_override(
            "TS_SQLITE_CACHE_SIZE_KB",
            cfg.sqlite_cache_size_kb,
            parse_i64,
        )
        .map_err(|e| TsError::Config(format!("TS_SQLITE_CACHE_SIZE_KB: {e}")))?;
        if let Ok(raw) = std::env::var("TS_DATA_DIR") {
            cfg.data_dir = PathBuf::from(raw);
        }
        if let Ok(raw) = std::env::var("TS_DATABASE_URL") {
            cfg.database_url = raw;
        }
        Ok(cfg)
    }

    /// Resolve the runtime static-data directory (Chapter 8 Config / §8.3).
    ///
    /// Returns the first existing candidate:
    /// 1. the configured `data_dir` as-is (absolute, or relative to the CWD —
    ///    the bundled repo-root `Data/`),
    /// 2. the same relative path next to the current executable (the
    ///    `build.rs`-packaged `Data/` shipped beside the binary),
    /// 3. otherwise the configured path unchanged (the caller reports it).
    pub fn resolve_data_dir(&self) -> PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        Self::resolve_data_dir_with(&self.data_dir, exe_dir)
    }

    /// Candidate resolution, injectable for tests (no `current_exe()`).
    pub fn resolve_data_dir_with(data_dir: &Path, exe_dir: Option<PathBuf>) -> PathBuf {
        if data_dir.exists() {
            return data_dir.to_path_buf();
        }
        if let Some(exe) = exe_dir {
            // `Path::join` with an absolute `data_dir` yields `data_dir` itself.
            let candidate = exe.join(data_dir);
            if candidate.exists() {
                return candidate;
            }
        }
        data_dir.to_path_buf()
    }

    /// Resolve the SQLite database URL candidate (similar to data_dir resolution).
    pub fn resolve_database_url(&self) -> String {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        Self::resolve_database_url_with(&self.database_url, exe_dir)
    }

    pub fn resolve_database_url_with(database_url: &str, exe_dir: Option<PathBuf>) -> String {
        if let Some(path_str) = database_url
            .strip_prefix("sqlite://")
            .or_else(|| database_url.strip_prefix("sqlite:"))
        {
            let path = Path::new(path_str);
            if path.exists() {
                return database_url.to_string();
            }
            if let Some(exe) = exe_dir {
                let candidate = exe.join(path);
                if candidate.exists() {
                    return format!("sqlite://{}", candidate.display());
                }
            }
        }
        database_url.to_string()
    }
}

/// Read + validate + parse a TOML file (no env overrides). Rejects the removed
/// SQLite-era keys. Extracted from [`Config::load`] so rejection is testable
/// against a single file without touching `TS_*` env vars.
pub fn from_file(path: &std::path::Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| TsError::Config(format!("read {}: {}", path.display(), e)))?;
    // Reject the removed SQLite keys if present.
    for key in ["account_db_path", "template_db_path", "member_dir"] {
        if raw.contains(key) {
            return Err(TsError::Config(format!(
                "{} present in ts_dream.toml — SQLite era key is removed; use `database_url`",
                key
            )));
        }
    }
    toml::from_str(&raw).map_err(|e| TsError::Config(format!("parse {}: {}", path.display(), e)))
}
