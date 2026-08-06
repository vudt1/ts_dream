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
            from_file(&path)?
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
    fn resolve_data_dir_with(data_dir: &Path, exe_dir: Option<PathBuf>) -> PathBuf {
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
}

/// Read + validate + parse a TOML file (no env overrides). Rejects the removed
/// SQLite-era keys. Extracted from [`Config::load`] so rejection is testable
/// against a single file without touching `TS_*` env vars.
fn from_file(path: &std::path::Path) -> Result<Config> {
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

    #[test]
    fn from_file_rejects_sqlite_era_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ts_dream.toml");
        for key in ["account_db_path", "template_db_path", "member_dir"] {
            std::fs::write(&path, format!("{key} = \"/old/path\"\n")).unwrap();
            let err = from_file(&path).unwrap_err();
            assert!(
                err.to_string().contains("removed"),
                "rejection message must flag the removed key; got: {err}"
            );
        }
    }

    #[test]
    fn from_file_parses_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ts_dream.toml");
        std::fs::write(&path, "game_port = 7000\nweb_port = 8100\n").unwrap();
        let cfg = from_file(&path).unwrap();
        assert_eq!(cfg.game_port, 7000);
        assert_eq!(cfg.web_port, 8100);
    }

    #[test]
    fn resolve_data_dir_prefers_existing_configured_path() {
        let dir = tempfile::tempdir().unwrap();
        // An absolute data_dir that exists is returned unchanged.
        let cfg = Config {
            data_dir: dir.path().to_path_buf(),
            ..Config::default()
        };
        assert_eq!(cfg.resolve_data_dir(), dir.path().to_path_buf());
    }

    #[test]
    fn resolve_data_dir_falls_back_to_exe_adjacent_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let exe_dir = dir.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        // The build.rs-packaged `Data/` sits next to the executable.
        let bundled = exe_dir.join("bundle");
        std::fs::create_dir_all(&bundled).unwrap();

        // The configured relative path does NOT exist in the CWD here, but the
        // exe-adjacent one does -> resolve to `exe_dir/bundle`.
        let got = Config::resolve_data_dir_with(
            &std::path::PathBuf::from("bundle"),
            Some(exe_dir),
        );
        assert_eq!(got, bundled);
    }

    #[test]
    fn resolve_data_dir_returns_configured_when_none_exist() {
        // Neither the configured path nor an exe-adjacent bundle exists ->
        // the configured path is returned unchanged (caller reports it).
        let got = Config::resolve_data_dir_with(
            &std::path::PathBuf::from("does-not-exist-data"),
            None,
        );
        assert_eq!(got, std::path::PathBuf::from("does-not-exist-data"));
    }
}