//! Configuration (Chapter 8).
//!
//! A TOML file (`ts_dream.toml`) with `TS_` env overrides, loaded once at
//! boot. The SQLite-era keys (`account_db_path`, `member_dir`,
//! `template_db_path`) are removed and rejected.

use crate::error::{Result, TsError};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub game_port: u16,
    pub web_port: u16,
    pub data_dir: PathBuf,
    pub database_url: String,
    pub perexp_default: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_port: 6414,
            web_port: 8090,
            data_dir: PathBuf::from("./Data"),
            database_url: "mysql://user:pass@localhost:3306/ts_dream".to_string(),
            perexp_default: 0,
        }
    }
}

fn parse_u16(s: &str) -> std::result::Result<u16, String> {
    s.trim()
        .parse::<u16>()
        .map_err(|e| format!("invalid u16: {e}"))
}
fn parse_u32(s: &str) -> std::result::Result<u32, String> {
    s.trim().parse::<u32>().map_err(|e| format!("invalid u32: {e}"))
}

/// Apply a `TS_` env var override if present, else keep the TOML value.
fn env_override<T, F>(
    key: &str,
    toml: T,
    parse: F,
) -> std::result::Result<T, String>
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
            let raw = std::fs::read_to_string(&path)
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
            toml::from_str(&raw)
                .map_err(|e| TsError::Config(format!("parse {}: {}", path.display(), e)))?
        } else {
            Config::default()
        };

        cfg.game_port = env_override("TS_GAME_PORT", cfg.game_port, parse_u16).map_err(|e| {
            TsError::Config(format!("TS_GAME_PORT: {e}"))
        })?;
        cfg.web_port = env_override("TS_WEB_PORT", cfg.web_port, parse_u16).map_err(|e| {
            TsError::Config(format!("TS_WEB_PORT: {e}"))
        })?;
        cfg.perexp_default =
            env_override("TS_PEREXP_DEFAULT", cfg.perexp_default, parse_u32).map_err(|e| {
                TsError::Config(format!("TS_PEREXP_DEFAULT: {e}"))
            })?;
        if let Ok(raw) = std::env::var("TS_DATA_DIR") {
            cfg.data_dir = PathBuf::from(raw);
        }
        if let Ok(raw) = std::env::var("TS_DATABASE_URL") {
            cfg.database_url = raw;
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.game_port, 6414);
        assert_eq!(cfg.web_port, 8090);
        assert_eq!(cfg.perexp_default, 0);
        assert!(cfg.database_url.contains("ts_dream"));
    }

    #[test]
    fn parse_u16_rejects_bad() {
        assert_eq!(parse_u16("6414").unwrap(), 6414);
        assert!(parse_u16("abc").is_err());
    }
}