//! Seasonal Festival Subsystem Lifecycle Manager (ADR 0011, Ticket 01).
//!
//! Manages the 11 manual-only Seasonal Festivals independently of `EventLifecycleManager`:
//! - Exempt from the 120-minute `check_watchdog()` ceiling
//! - Supports multiple festivals `Active` simultaneously
//! - Persists `is_active` and monotonic `season_seq` in SQLite (`seasonal_festival_state`)
//! - Hydrates state from SQLite on `ServerControl::start()`

use axum::http::StatusCode;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::data::loaders::festival::{FestivalDatLoader, FestivalDef};
use crate::db::pool::DbPool;

pub const FESTIVAL_TET_NGUYEN_DAN: &str = "tet_nguyen_dan";
pub const FESTIVAL_TET_NGUYEN_TIEU: &str = "tet_nguyen_tieu";
pub const FESTIVAL_VALENTINE: &str = "valentine";
pub const FESTIVAL_CA_THANG_TU: &str = "ca_thang_tu";
pub const FESTIVAL_GIO_TO_HUNG_VUONG: &str = "gio_to_hung_vuong";
pub const FESTIVAL_PHAT_DAN: &str = "phat_dan";
pub const FESTIVAL_DOAN_NGO: &str = "doan_ngo";
pub const FESTIVAL_THAT_TICH: &str = "that_tich";
pub const FESTIVAL_TRUNG_THU: &str = "trung_thu";
pub const FESTIVAL_HALLOWEEN: &str = "halloween";
pub const FESTIVAL_GIANG_SINH: &str = "giang_sinh";

/// Static configuration profile for a Seasonal Festival (loaded from `Data/seasonal_festivals.json`
/// or built-in defaults).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FestivalProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub lore_record_id: Option<u32>,
    #[serde(default)]
    pub exp_multiplier_pct: u32,
}

/// Public snapshot of a Seasonal Festival's configuration and live state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SeasonalFestivalInfo {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub season_seq: u64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub lore_record_id: Option<u32>,
    pub unique_lore_key: Option<u32>,
    pub lore_collision: bool,
}

impl SeasonalFestivalInfo {
    pub fn badge_class(&self) -> &'static str {
        if self.is_active {
            "status-online"
        } else {
            "status-offline"
        }
    }

    pub fn status_label(&self) -> &'static str {
        if self.is_active {
            "Active"
        } else {
            "Idle"
        }
    }

    pub fn lore_status_label(&self) -> String {
        if let Some(key) = self.unique_lore_key {
            format!("Key {key}")
        } else if self.lore_collision {
            "Collision (unpatched Festival.Dat)".to_string()
        } else {
            "No record".to_string()
        }
    }

    pub fn lore_badge_class(&self) -> &'static str {
        if self.unique_lore_key.is_some() {
            "status-online"
        } else {
            "status-offline"
        }
    }
}

#[derive(Debug, Clone)]
struct FestivalEntry {
    order: usize,
    profile: FestivalProfile,
    is_active: bool,
    season_seq: u64,
    started_at: Option<i64>,
    updated_at: i64,
    unique_lore_key: Option<u32>,
    lore_collision: bool,
}

impl FestivalEntry {
    fn to_info(&self) -> SeasonalFestivalInfo {
        SeasonalFestivalInfo {
            id: self.profile.id.clone(),
            name: self.profile.name.clone(),
            is_active: self.is_active,
            season_seq: self.season_seq,
            started_at: self.started_at,
            updated_at: self.updated_at,
            lore_record_id: self.profile.lore_record_id,
            unique_lore_key: self.unique_lore_key,
            lore_collision: self.lore_collision,
        }
    }
}

/// Central Seasonal Festival Lifecycle Manager.
#[derive(Clone)]
pub struct SeasonalFestivalManager {
    entries: Arc<RwLock<HashMap<String, FestivalEntry>>>,
}

impl Default for SeasonalFestivalManager {
    fn default() -> Self {
        Self::new(Self::default_profiles())
    }
}

impl SeasonalFestivalManager {
    /// Built-in canonical profiles for the 11 Seasonal Festivals.
    pub fn default_profiles() -> Vec<FestivalProfile> {
        vec![
            FestivalProfile {
                id: FESTIVAL_TET_NGUYEN_DAN.to_string(),
                name: "Tết Nguyên Đán".to_string(),
                lore_record_id: Some(1),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_TET_NGUYEN_TIEU.to_string(),
                name: "Tết Nguyên Tiêu".to_string(),
                lore_record_id: Some(2),
                exp_multiplier_pct: 200,
            },
            FestivalProfile {
                id: FESTIVAL_VALENTINE.to_string(),
                name: "Valentine".to_string(),
                lore_record_id: Some(3),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_CA_THANG_TU.to_string(),
                name: "Ngày Cá Tháng Tư".to_string(),
                lore_record_id: Some(4),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_GIO_TO_HUNG_VUONG.to_string(),
                name: "Giỗ Tổ Hùng Vương".to_string(),
                lore_record_id: Some(5),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_PHAT_DAN.to_string(),
                name: "Lễ Phật Đản".to_string(),
                lore_record_id: Some(6),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_DOAN_NGO.to_string(),
                name: "Tết Đoan Ngọ".to_string(),
                lore_record_id: Some(7),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_THAT_TICH.to_string(),
                name: "Lễ Thất Tịch".to_string(),
                lore_record_id: Some(8),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_TRUNG_THU.to_string(),
                name: "Tết Trung Thu".to_string(),
                lore_record_id: Some(9),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_HALLOWEEN.to_string(),
                name: "Halloween".to_string(),
                lore_record_id: Some(10),
                exp_multiplier_pct: 100,
            },
            FestivalProfile {
                id: FESTIVAL_GIANG_SINH.to_string(),
                name: "Lễ Giáng Sinh".to_string(),
                lore_record_id: Some(11),
                exp_multiplier_pct: 100,
            },
        ]
    }

    pub fn new(profiles: Vec<FestivalProfile>) -> Self {
        let profiles = if profiles.is_empty() {
            Self::default_profiles()
        } else {
            profiles
        };
        let mut map = HashMap::with_capacity(profiles.len());
        for (order, profile) in profiles.into_iter().enumerate() {
            map.insert(
                profile.id.clone(),
                FestivalEntry {
                    order,
                    profile,
                    is_active: false,
                    season_seq: 0,
                    started_at: None,
                    updated_at: 0,
                    unique_lore_key: None,
                    lore_collision: false,
                },
            );
        }
        Self {
            entries: Arc::new(RwLock::new(map)),
        }
    }

    /// Load profiles from `Data/seasonal_festivals.json` if present, else use built-in defaults,
    /// and populate `unique_lore_key` / `lore_collision` from `Data/Festival.Dat` if present.
    pub fn from_data_dir(data_dir: &Path) -> Self {
        let json_path = data_dir.join("seasonal_festivals.json");
        let profiles = if json_path.exists() {
            match std::fs::read_to_string(&json_path) {
                Ok(content) => serde_json::from_str::<Vec<FestivalProfile>>(&content)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to parse seasonal_festivals.json: {e}");
                        Vec::new()
                    }),
                Err(e) => {
                    tracing::warn!("Failed to read seasonal_festivals.json: {e}");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        let mgr = Self::new(profiles);
        let dat_path = data_dir.join("Festival.Dat");
        if dat_path.exists() {
            if let Ok(bytes) = std::fs::read(&dat_path) {
                if let Ok(records) = FestivalDatLoader::load(&bytes) {
                    mgr.apply_festival_dat(&records);
                }
            }
        }
        mgr
    }

    /// Update each festival's `unique_lore_key` and `lore_collision` status from parsed `Festival.Dat` records.
    pub fn apply_festival_dat(&self, records: &HashMap<u32, FestivalDef>) {
        let mut guard = self.entries.write().unwrap();
        for entry in guard.values_mut() {
            if let Some(rec_id) = entry.profile.lore_record_id {
                if let Some(def) = records.get(&rec_id) {
                    entry.unique_lore_key = def.unique_lore_key;
                    entry.lore_collision = def.lookup_key.is_some() && def.unique_lore_key.is_none();
                } else {
                    entry.unique_lore_key = None;
                    entry.lore_collision = false;
                }
            } else {
                entry.unique_lore_key = None;
                entry.lore_collision = false;
            }
        }
    }

    /// List all seasonal festivals in canonical display order.
    pub fn list_festivals(&self) -> Vec<SeasonalFestivalInfo> {
        let guard = self.entries.read().unwrap();
        let mut items: Vec<&FestivalEntry> = guard.values().collect();
        items.sort_by_key(|e| e.order);
        items.into_iter().map(FestivalEntry::to_info).collect()
    }

    /// Get a single festival's info by ID.
    pub fn get_festival(&self, id: &str) -> Option<SeasonalFestivalInfo> {
        self.entries
            .read()
            .unwrap()
            .get(id)
            .map(FestivalEntry::to_info)
    }

    /// Check whether a given seasonal festival is currently `Active`.
    pub fn is_festival_active(&self, id: &str) -> bool {
        self.entries
            .read()
            .unwrap()
            .get(id)
            .is_some_and(|e| e.is_active)
    }

    /// Return all currently active festival IDs in canonical order.
    pub fn active_festival_ids(&self) -> Vec<String> {
        let guard = self.entries.read().unwrap();
        let mut items: Vec<&FestivalEntry> = guard.values().filter(|e| e.is_active).collect();
        items.sort_by_key(|e| e.order);
        items.into_iter().map(|e| e.profile.id.clone()).collect()
    }

    /// Hydrate `is_active`, `season_seq`, `started_at`, and `updated_at` from SQLite.
    pub async fn hydrate_from_db(&self, pool: &DbPool) -> Result<(), sqlx::Error> {
        let rows: Vec<(String, i64, i64, Option<i64>, i64)> = sqlx::query_as(
            "SELECT festival_id, is_active, season_seq, started_at, updated_at
             FROM seasonal_festival_state",
        )
        .fetch_all(&pool.read)
        .await?;

        let mut guard = self.entries.write().unwrap();
        for (festival_id, is_active, season_seq, started_at, updated_at) in rows {
            if let Some(entry) = guard.get_mut(&festival_id) {
                entry.is_active = is_active != 0;
                entry.season_seq = u64::try_from(season_seq.max(0)).unwrap_or(0);
                entry.started_at = started_at;
                entry.updated_at = updated_at;
            }
        }
        Ok(())
    }

    /// Transition a festival from `Idle -> Active`, incrementing `season_seq += 1`
    /// and persisting to SQLite. Multiple festivals may be `Active` simultaneously.
    pub async fn start_festival(
        &self,
        id: &str,
        is_server_running: bool,
        pool: Option<&DbPool>,
    ) -> Result<SeasonalFestivalInfo, (StatusCode, String)> {
        self.transition_festival(id, true, is_server_running, pool)
            .await
    }

    /// Transition a festival from `Active -> Idle`, preserving `season_seq`
    /// and persisting to SQLite.
    pub async fn stop_festival(
        &self,
        id: &str,
        is_server_running: bool,
        pool: Option<&DbPool>,
    ) -> Result<SeasonalFestivalInfo, (StatusCode, String)> {
        self.transition_festival(id, false, is_server_running, pool)
            .await
    }

    async fn transition_festival(
        &self,
        id: &str,
        target_active: bool,
        is_server_running: bool,
        pool: Option<&DbPool>,
    ) -> Result<SeasonalFestivalInfo, (StatusCode, String)> {
        if !is_server_running {
            return Err((
                StatusCode::CONFLICT,
                "Game server is stopped. Please start the server first".to_string(),
            ));
        }

        let now = chrono::Utc::now().timestamp();
        let (before, after) = {
            let mut guard = self.entries.write().unwrap();
            let entry = guard.get_mut(id).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Festival '{id}' does not exist"),
                )
            })?;

            if target_active && entry.is_active {
                return Err((
                    StatusCode::CONFLICT,
                    format!("Festival '{id}' is already active"),
                ));
            }
            if !target_active && !entry.is_active {
                return Err((
                    StatusCode::CONFLICT,
                    format!("Festival '{id}' is already idle"),
                ));
            }

            let before = entry.clone();
            entry.is_active = target_active;
            if target_active {
                entry.season_seq += 1;
                entry.started_at = Some(now);
            }
            entry.updated_at = now;
            (before, entry.to_info())
        };

        if let Some(pool) = pool {
            if let Err(e) = persist_festival_state(pool, &after).await {
                let mut guard = self.entries.write().unwrap();
                if let Some(entry) = guard.get_mut(id) {
                    *entry = before;
                }
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to persist festival state: {e}"),
                ));
            }
        }

        if target_active {
            tracing::info!(
                festival_id = id,
                season_seq = after.season_seq,
                "Seasonal festival started"
            );
        } else {
            tracing::info!(
                festival_id = id,
                season_seq = after.season_seq,
                "Seasonal festival stopped"
            );
        }
        Ok(after)
    }
}

async fn persist_festival_state(
    pool: &DbPool,
    info: &SeasonalFestivalInfo,
) -> Result<(), sqlx::Error> {
    let is_active_int: i64 = if info.is_active { 1 } else { 0 };
    let seq_int = i64::try_from(info.season_seq).unwrap_or(i64::MAX);
    sqlx::query(
        "INSERT INTO seasonal_festival_state (festival_id, is_active, season_seq, started_at, updated_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(festival_id) DO UPDATE SET
             is_active  = excluded.is_active,
             season_seq = excluded.season_seq,
             started_at = excluded.started_at,
             updated_at = excluded.updated_at",
    )
    .bind(&info.id)
    .bind(is_active_int)
    .bind(seq_int)
    .bind(info.started_at)
    .bind(info.updated_at)
    .execute(&pool.write)
    .await
    .map(|_| ())
}
