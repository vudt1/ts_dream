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

pub const CLAIM_TYPE_LORE_SHOWN: &str = "lore_shown";

/// Canonical `period_key` for per-season claims in `character_festival_claims`.
pub fn season_period_key(season_seq: u64) -> String {
    format!("season-{season_seq}")
}

/// Canonical `period_key` (`YYYY-MM-DD` in `UTC+7`) for daily drop caps in `character_festival_claims`.
pub fn utc7_date_key(unix_secs: i64) -> String {
    let offset = chrono::FixedOffset::east_opt(7 * 3600).expect("UTC+7 offset");
    chrono::DateTime::from_timestamp(unix_secs, 0)
        .map(|dt| dt.with_timezone(&offset).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

/// Canonical `claim_type` for a seasonal bonus drop item in `character_festival_claims`.
pub fn drop_claim_type(item_id: u16) -> String {
    format!("drop_{item_id}")
}

/// Seasonal bonus drop rule for PvE `Normal` battle victories.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeasonalBonusDrop {
    pub item_id: u16,
    pub drop_rate_pct: u8,
    #[serde(default)]
    pub min_enemy_level: u16,
    #[serde(default)]
    pub daily_drop_cap: u32,
}

/// Granular gate rules for a shared map (`12000`, `12001`, `12005`, `12421`, etc.)
/// so ONLY festival-specific doors, NPCs, and quests are locked when the festival is `Idle`,
/// while normal main-story content on the same map stays open.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedMapGate {
    pub map_id: u16,
    #[serde(default)]
    pub door_ids: Vec<u16>,
    #[serde(default)]
    pub npc_place_ids: Vec<u16>,
    #[serde(default)]
    pub quest_ids: Vec<u32>,
}

/// Dynamic festival NPC spawned on a map via `S→C 16 01` when the festival is `Active`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DynamicFestivalNpc {
    pub map_id: u16,
    pub npc_place_id: u16,
    #[serde(default)]
    pub npc_id: u16,
}

/// Static configuration profile for a Seasonal Festival (loaded from `Data/seasonal_festivals.json`
/// or built-in defaults).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FestivalProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub lore_record_id: Option<u32>,
    #[serde(default)]
    pub exp_multiplier_pct: Option<u32>,
    #[serde(default)]
    pub exclusive_maps: Vec<u16>,
    #[serde(default)]
    pub shared_map_gates: Vec<SharedMapGate>,
    #[serde(default)]
    pub dynamic_npcs: Vec<DynamicFestivalNpc>,
    #[serde(default)]
    pub bonus_drops: Vec<SeasonalBonusDrop>,
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
#[derive(Debug, Clone)]
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
                exp_multiplier_pct: None,
                bonus_drops: vec![
                    SeasonalBonusDrop {
                        item_id: 46335,
                        drop_rate_pct: 10,
                        min_enemy_level: 10,
                        daily_drop_cap: 10,
                    },
                    SeasonalBonusDrop {
                        item_id: 46338,
                        drop_rate_pct: 10,
                        min_enemy_level: 10,
                        daily_drop_cap: 10,
                    },
                ],
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_TET_NGUYEN_TIEU.to_string(),
                name: "Tết Nguyên Tiêu".to_string(),
                lore_record_id: Some(2),
                exp_multiplier_pct: Some(200),
                exclusive_maps: vec![54811, 54812],
                shared_map_gates: vec![SharedMapGate {
                    map_id: 12001,
                    door_ids: Vec::new(),
                    npc_place_ids: Vec::new(),
                    quest_ids: Vec::new(),
                }],
                dynamic_npcs: Vec::new(),
                bonus_drops: Vec::new(),
            },
            FestivalProfile {
                id: FESTIVAL_VALENTINE.to_string(),
                name: "Valentine".to_string(),
                lore_record_id: Some(3),
                exp_multiplier_pct: None,
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_CA_THANG_TU.to_string(),
                name: "Ngày Cá Tháng Tư".to_string(),
                lore_record_id: Some(4),
                exp_multiplier_pct: None,
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_GIO_TO_HUNG_VUONG.to_string(),
                name: "Giỗ Tổ Hùng Vương".to_string(),
                lore_record_id: Some(5),
                exp_multiplier_pct: None,
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_PHAT_DAN.to_string(),
                name: "Lễ Phật Đản".to_string(),
                lore_record_id: Some(6),
                exp_multiplier_pct: None,
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_DOAN_NGO.to_string(),
                name: "Tết Đoan Ngọ".to_string(),
                lore_record_id: Some(7),
                exp_multiplier_pct: None,
                shared_map_gates: vec![SharedMapGate {
                    map_id: 12000,
                    door_ids: Vec::new(),
                    npc_place_ids: Vec::new(),
                    quest_ids: Vec::new(),
                }],
                bonus_drops: vec![SeasonalBonusDrop {
                    item_id: 46220,
                    drop_rate_pct: 10,
                    min_enemy_level: 10,
                    daily_drop_cap: 10,
                }],
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_THAT_TICH.to_string(),
                name: "Lễ Thất Tịch".to_string(),
                lore_record_id: Some(8),
                exp_multiplier_pct: None,
                exclusive_maps: vec![54825, 12941, 12942, 12943],
                shared_map_gates: vec![SharedMapGate {
                    map_id: 12001,
                    door_ids: Vec::new(),
                    npc_place_ids: vec![1825, 5778, 1879, 3453, 5653],
                    quest_ids: vec![10786, 11746, 11747],
                }],
                dynamic_npcs: Vec::new(),
                bonus_drops: Vec::new(),
            },
            FestivalProfile {
                id: FESTIVAL_TRUNG_THU.to_string(),
                name: "Tết Trung Thu".to_string(),
                lore_record_id: Some(9),
                exp_multiplier_pct: None,
                exclusive_maps: vec![61051, 61178],
                shared_map_gates: vec![SharedMapGate {
                    map_id: 12001,
                    door_ids: Vec::new(),
                    npc_place_ids: vec![3468, 5668, 3886],
                    quest_ids: Vec::new(),
                }],
                dynamic_npcs: Vec::new(),
                bonus_drops: Vec::new(),
            },
            FestivalProfile {
                id: FESTIVAL_HALLOWEEN.to_string(),
                name: "Halloween".to_string(),
                lore_record_id: Some(10),
                exp_multiplier_pct: None,
                exclusive_maps: {
                    let mut maps = vec![57518];
                    maps.extend(57531..=57536);
                    maps.extend(61551..=61567);
                    maps
                },
                bonus_drops: vec![SeasonalBonusDrop {
                    item_id: 46073,
                    drop_rate_pct: 10,
                    min_enemy_level: 10,
                    daily_drop_cap: 10,
                }],
                ..Default::default()
            },
            FestivalProfile {
                id: FESTIVAL_GIANG_SINH.to_string(),
                name: "Lễ Giáng Sinh".to_string(),
                lore_record_id: Some(11),
                exp_multiplier_pct: None,
                exclusive_maps: (54827..=54831).collect(),
                shared_map_gates: vec![
                    SharedMapGate {
                        map_id: 12005,
                        door_ids: Vec::new(),
                        npc_place_ids: Vec::new(),
                        quest_ids: Vec::new(),
                    },
                    SharedMapGate {
                        map_id: 12421,
                        door_ids: Vec::new(),
                        npc_place_ids: vec![2015, 2016],
                        quest_ids: vec![10044, 11542, 12174, 12175, 12352, 12353],
                    },
                ],
                dynamic_npcs: Vec::new(),
                bonus_drops: Vec::new(),
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

    /// Return the effective EXP multiplier percentage across all currently `Active` festivals
    /// (`100` = 1.0x neutral, `200` = 2.0x). Returns `100` if no festival is active.
    pub fn active_exp_multiplier_pct(&self) -> u32 {
        let guard = self.entries.read().unwrap();
        guard
            .values()
            .filter(|e| e.is_active)
            .filter_map(|e| e.profile.exp_multiplier_pct)
            .map(|m| m.max(100))
            .max()
            .unwrap_or(100)
    }

    /// Return all `(festival_id, SeasonalBonusDrop)` rules for currently `Active` festivals
    /// in canonical display order.
    pub fn active_bonus_drops(&self) -> Vec<(String, SeasonalBonusDrop)> {
        let guard = self.entries.read().unwrap();
        let mut items: Vec<&FestivalEntry> = guard.values().filter(|e| e.is_active).collect();
        items.sort_by_key(|e| e.order);
        let mut drops = Vec::new();
        for entry in items {
            for drop in &entry.profile.bonus_drops {
                drops.push((entry.profile.id.clone(), drop.clone()));
            }
        }
        drops
    }

    /// Check whether a door transition `(map_id, door_id -> target_map_id)` is allowed.
    /// Rejects if `target_map_id` is an exclusive seasonal map whose owning festivals are all `Idle`,
    /// or if `(map_id, door_id)` is a shared-map festival gate whose owning festivals are all `Idle`.
    pub fn check_door_access(
        &self,
        map_id: u16,
        door_id: u16,
        target_map_id: u16,
    ) -> Result<(), &'static str> {
        let guard = self.entries.read().unwrap();
        if target_map_id > 0 {
            let mut owned_by_any = false;
            let mut active_owner = false;
            for entry in guard.values() {
                if entry.profile.exclusive_maps.contains(&target_map_id) {
                    owned_by_any = true;
                    if entry.is_active {
                        active_owner = true;
                        break;
                    }
                }
            }
            if owned_by_any && !active_owner {
                return Err("Lễ hội chưa mở: không thể vào bản đồ này.");
            }
        }

        let mut gated_by_any = false;
        let mut active_gate = false;
        for entry in guard.values() {
            let matches_gate = entry
                .profile
                .shared_map_gates
                .iter()
                .any(|g| g.map_id == map_id && g.door_ids.contains(&door_id));
            if matches_gate {
                gated_by_any = true;
                if entry.is_active {
                    active_gate = true;
                    break;
                }
            }
        }
        if gated_by_any && !active_gate {
            return Err("Lễ hội chưa mở: cổng này tạm khóa.");
        }
        Ok(())
    }

    /// Check whether interacting with NPC `npc_place_id` on `map_id` is allowed.
    pub fn check_npc_access(&self, map_id: u16, npc_place_id: u16) -> Result<(), &'static str> {
        let guard = self.entries.read().unwrap();
        let mut map_owned = false;
        let mut map_active = false;
        let mut npc_gated = false;
        let mut npc_active = false;

        for entry in guard.values() {
            if entry.profile.exclusive_maps.contains(&map_id) {
                map_owned = true;
                if entry.is_active {
                    map_active = true;
                }
            }
            let in_shared_gate = entry
                .profile
                .shared_map_gates
                .iter()
                .any(|g| g.map_id == map_id && g.npc_place_ids.contains(&npc_place_id));
            let in_dynamic = entry
                .profile
                .dynamic_npcs
                .iter()
                .any(|n| n.map_id == map_id && n.npc_place_id == npc_place_id);
            if in_shared_gate || in_dynamic {
                npc_gated = true;
                if entry.is_active {
                    npc_active = true;
                }
            }
        }

        if map_owned && !map_active {
            return Err("Lễ hội chưa mở: không thể tương tác NPC lễ hội.");
        }
        if npc_gated && !npc_active {
            return Err("Lễ hội chưa mở: NPC này chỉ xuất hiện trong mùa lễ hội.");
        }
        Ok(())
    }

    /// Check whether a quest `quest_id` on `map_id` is allowed.
    pub fn check_quest_access(&self, map_id: u16, quest_id: u32) -> Result<(), &'static str> {
        let guard = self.entries.read().unwrap();
        let mut map_owned = false;
        let mut map_active = false;
        let mut quest_gated = false;
        let mut quest_active = false;

        for entry in guard.values() {
            if entry.profile.exclusive_maps.contains(&map_id) {
                map_owned = true;
                if entry.is_active {
                    map_active = true;
                }
            }
            let in_shared_gate = entry
                .profile
                .shared_map_gates
                .iter()
                .any(|g| g.map_id == map_id && g.quest_ids.contains(&quest_id));
            if in_shared_gate {
                quest_gated = true;
                if entry.is_active {
                    quest_active = true;
                }
            }
        }

        if map_owned && !map_active {
            return Err("Lễ hội chưa mở: nhiệm vụ lễ hội đang đóng.");
        }
        if quest_gated && !quest_active {
            return Err("Lễ hội chưa mở: nhiệm vụ lễ hội đang đóng.");
        }
        Ok(())
    }

    /// Return all dynamic festival NPCs for `map_id` belonging to currently `Active` festivals.
    pub fn active_dynamic_npcs_for_map(&self, map_id: u16) -> Vec<DynamicFestivalNpc> {
        let guard = self.entries.read().unwrap();
        let mut items: Vec<&FestivalEntry> = guard.values().filter(|e| e.is_active).collect();
        items.sort_by_key(|e| e.order);
        let mut npcs = Vec::new();
        for entry in items {
            for npc in &entry.profile.dynamic_npcs {
                if npc.map_id == map_id
                    && !npcs
                        .iter()
                        .any(|existing: &DynamicFestivalNpc| existing.npc_place_id == npc.npc_place_id)
                {
                    npcs.push(npc.clone());
                }
            }
        }
        npcs
    }

    /// Return all dynamic festival NPCs configured for `festival_id`.
    pub fn dynamic_npcs_for_festival(&self, id: &str) -> Vec<DynamicFestivalNpc> {
        self.entries
            .read()
            .unwrap()
            .get(id)
            .map(|e| e.profile.dynamic_npcs.clone())
            .unwrap_or_default()
    }

    /// Return exclusive maps for `festival_id` that are not also owned by another currently `Active` festival.
    pub fn evacuable_maps_on_stop(&self, id: &str) -> Vec<u16> {
        let guard = self.entries.read().unwrap();
        let Some(target) = guard.get(id) else {
            return Vec::new();
        };
        target
            .profile
            .exclusive_maps
            .iter()
            .copied()
            .filter(|map_id| {
                !guard.values().any(|other| {
                    other.profile.id != id
                        && other.is_active
                        && other.profile.exclusive_maps.contains(map_id)
                })
            })
            .collect()
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

/// Check whether `(player_id, festival_id, period_key, claim_type)` already exists in
/// `character_festival_claims` (reads from `pool.read`).
pub async fn has_festival_claim(
    pool: &DbPool,
    player_id: u32,
    festival_id: &str,
    period_key: &str,
    claim_type: &str,
) -> Result<bool, sqlx::Error> {
    let row: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM character_festival_claims
         WHERE playerid = ? AND festival_id = ? AND period_key = ? AND claim_type = ?
         LIMIT 1",
    )
    .bind(i64::from(player_id))
    .bind(festival_id)
    .bind(period_key)
    .bind(claim_type)
    .fetch_optional(&pool.read)
    .await?;
    Ok(row.is_some())
}

/// Query current `claim_count` for `(player_id, festival_id, period_key, claim_type)`
/// in `character_festival_claims` (reads from `pool.read`). Returns `0` when absent.
pub async fn get_festival_claim_count(
    pool: &DbPool,
    player_id: u32,
    festival_id: &str,
    period_key: &str,
    claim_type: &str,
) -> Result<u32, sqlx::Error> {
    let row: Option<i64> = sqlx::query_scalar(
        "SELECT claim_count FROM character_festival_claims
         WHERE playerid = ? AND festival_id = ? AND period_key = ? AND claim_type = ?
         LIMIT 1",
    )
    .bind(i64::from(player_id))
    .bind(festival_id)
    .bind(period_key)
    .bind(claim_type)
    .fetch_optional(&pool.read)
    .await?;
    Ok(row.and_then(|v| u32::try_from(v.max(0)).ok()).unwrap_or(0))
}

/// Insert or increment a claim in `character_festival_claims` (writes via `pool.write`).
pub async fn record_festival_claim(
    pool: &DbPool,
    player_id: u32,
    festival_id: &str,
    period_key: &str,
    claim_type: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO character_festival_claims
             (playerid, festival_id, period_key, claim_type, claim_count, claimed_at)
         VALUES (?, ?, ?, ?, 1, ?)
         ON CONFLICT(playerid, festival_id, period_key, claim_type) DO UPDATE SET
             claim_count = claim_count + 1,
             claimed_at  = excluded.claimed_at",
    )
    .bind(i64::from(player_id))
    .bind(festival_id)
    .bind(period_key)
    .bind(claim_type)
    .bind(now)
    .execute(&pool.write)
    .await
    .map(|_| ())
}
