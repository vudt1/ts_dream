//! Static data loading (Chapter 3). Loads the `Data/` directory byte-identical
//! into the in-memory tables, following each file's encoding + row convention.
#![allow(clippy::chunks_exact_to_as_chunks)]
//! `Loaded()` sets `DataLoaded=true` (the TCP accept gate).

use crate::data::ini::{Ini, NOTHING};
use crate::data::tables::*;
use crate::data::texps::compute_texps;
use crate::encoding;
use crate::error::{Result, TsError};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The complete loaded static dataset.
#[derive(Debug, Default, Clone)]
pub struct GameData {
    pub npcs: HashMap<i64, Npc>,
    pub items: HashMap<i64, Item>,
    pub skills: HashMap<i64, Skill>,
    /// Rich binary skill definitions from Skill.Dat (PC `Skill.Dat` once the
    /// dedicated parser in `skill_pc.rs` lands; mobile `Skill_C.dat` until then).
    pub binary_skill_defs: HashMap<u16, BinarySkillDef>,
    /// Mission-mark definitions and the mobile reverse bit index.
    pub mark_defs: HashMap<u16, MarkDef>,
    pub bit_to_mission_id: HashMap<u16, u16>,
    pub mount_defs: HashMap<u16, MountDef>,
    pub mount_grow_defs: HashMap<u8, MountGrowDef>,
    pub achievement_defs: HashMap<u16, AchievementDef>,
    pub dispatch_defs: HashMap<u8, HashMap<u8, DispatchDef>>,
    pub dispatch_bonus_defs: HashMap<u8, DispatchBonusDef>,
    pub leaderboard_defs: HashMap<u8, LeaderboardDef>,
    pub scene_set_defs: HashMap<u16, SceneSetDef>,
    /// Military-rank definitions from PC `Rank.Dat`, keyed by 1-based
    /// record index (mirrors the Lua `rankDatas` loop counter).
    pub rank_defs: HashMap<u16, RankDef>,
    pub guide_last_bit_flag_id: HashMap<u8, u16>,
    pub guide_mark_flag_ids: HashMap<u8, u16>,
    pub mark_flag_to_guide_id: HashMap<u16, u8>,
    pub warps: HashMap<(i64, i64), Warp>,
    pub battle_gates: HashMap<(i64, i64), BattleGate>,
    pub dolls: HashMap<i64, Doll>,
    pub talks: HashMap<String, QuestDef>,
    pub texps: Vec<TexpRow>,
    pub npc_on_map: Vec<NpcOnMap>,
    pub item_on_map: Vec<ItemOnMap>,
    /// Spawned static drops (ItemOnMap.txt), keyed by
    /// `(map_id, slot)`. Pre-filled empty slots 1..255 per map, then each
    /// ItemOnMap.txt row spawns a `_Delay=999999` static drop.
    pub item_drop_on_map: HashMap<(i64, i64), ItemDropOnMap>,
    // Binary .Dat tables
    pub formula_params: Option<FormulaParams>,
    pub bliss_bags: HashMap<u16, BlissBagDef>,
    pub compounds: Vec<CompoundDef>,
    pub astrolabes: HashMap<u8, AstrolabeDef>,
    pub evo_statuses: HashMap<u8, EVOStatusDef>,
    pub city_ex: Vec<CityExDef>,
    pub item_defs: HashMap<u16, ItemDef>,
    pub npc_defs: HashMap<u16, NpcDef>,
    pub warp_defs: HashMap<usize, WarpDef>,
    pub scene_eve_data: HashMap<u32, SceneEveData>,
    /// Raw bytes for every accepted binary asset. Typed loaders project the
    /// known catalogs above; untyped catalogs remain available for the next
    /// typed port without ever admitting text files into production boot.
    pub raw_binary_assets: HashMap<String, Vec<u8>>,
    pub loaded: bool,
}

/// Deterministic metadata for an accepted binary asset.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BinaryAssetMeta {
    pub source_file: String,
    pub byte_size: usize,
    pub sha256: String,
}

/// Helper to locate a file in `data_dir`, checking exact name, lowercase/uppercase extension, and `_C.dat` variants.
pub fn resolve_data_file(data_dir: &Path, file_name: &str) -> Option<PathBuf> {
    let p = data_dir.join(file_name);
    if p.exists() {
        return Some(p);
    }
    if let Some((stem, ext)) = file_name.rsplit_once('.') {
        let p_lower = data_dir.join(format!("{}.{}", stem, ext.to_lowercase()));
        if p_lower.exists() {
            return Some(p_lower);
        }
        let p_upper = data_dir.join(format!("{}.{}", stem, ext.to_uppercase()));
        if p_upper.exists() {
            return Some(p_upper);
        }
        let p_stem_lower = data_dir.join(format!("{}.{}", stem.to_lowercase(), ext.to_lowercase()));
        if p_stem_lower.exists() {
            return Some(p_stem_lower);
        }
        let p_stem_capital = data_dir.join(format!(
            "{}{}.{}",
            stem[..1].to_uppercase(),
            stem[1..].to_lowercase(),
            ext.to_lowercase()
        ));
        if p_stem_capital.exists() {
            return Some(p_stem_capital);
        }
        let p_dat = data_dir.join(format!("{}.Dat", stem));
        if p_dat.exists() {
            return Some(p_dat);
        }
        let p_dat_lower = data_dir.join(format!("{}.dat", stem));
        if p_dat_lower.exists() {
            return Some(p_dat_lower);
        }
        let p_c_dat = data_dir.join(format!("{}_C.dat", stem));
        if p_c_dat.exists() {
            return Some(p_c_dat);
        }
    }
    // Also check CompreseData/ subfolder (Mobile convention)
    let comp_p = data_dir.join("CompreseData").join(file_name);
    if comp_p.exists() {
        return Some(comp_p);
    }
    None
}

fn num(field: &str, file: &str) -> Result<i64> {
    field
        .trim()
        .parse::<i64>()
        .map_err(|_| TsError::Data(format!("non-numeric field `{field}` in {file}")))
}

/// Strict column read (spec §3.1 "no defaults"): a missing or empty numeric
/// column is a load failure rather than a silent default.
fn num_at(idx: usize, f: &[&str], file: &str) -> Result<i64> {
    let field = f
        .get(idx)
        .ok_or_else(|| TsError::Data(format!("missing column {idx} in {file}")))?;
    num(field, file)
}

impl GameData {
    /// Load the production dataset. Only binary assets with `.dat`, `.emg`, or
    /// `.mng` extensions are considered; text fallbacks are intentionally not
    /// part of the production boot path.
    pub fn load(data_dir: &Path) -> Result<Self> {
        let mut d = Self::default();
        d.load_binary(data_dir)?;
        d.texps = compute_texps();
        d.loaded = true;
        Ok(d)
    }

    /// Load the legacy text-compatible dataset for migration fixtures only.
    /// Production code must call [`GameData::load`] instead.
    pub fn load_legacy_text(data_dir: &Path) -> Result<Self> {
        let mut d = Self::default();

        // 1. Items: Item.dat (binary) preferred, fallback to Items.txt
        if let Some(p) = resolve_data_file(data_dir, "Item.dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.item_defs = ItemDatLoader::load(&bytes)?;
            for def in d.item_defs.values() {
                d.items.insert(def.id as i64, def.to_item());
            }
        } else if let Some(p) = resolve_data_file(data_dir, "Items.txt") {
            d.load_items(&p)?;
        } else {
            return Err(TsError::Data(format!(
                "missing item data file (Item.dat / Items.txt) in {}",
                data_dir.display()
            )));
        }

        // 2. NPCs: Npc.dat (binary) preferred, fallback to Npcs.txt
        if let Some(p) = resolve_data_file(data_dir, "Npc.dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.npc_defs = NpcDatLoader::load(&bytes)?;
            for def in d.npc_defs.values() {
                d.npcs.insert(def.id as i64, def.to_npc());
            }
        } else if let Some(p) = resolve_data_file(data_dir, "Npcs.txt") {
            d.load_npcs(&p)?;
        } else {
            return Err(TsError::Data(format!(
                "missing NPC data file (Npc.dat / Npcs.txt) in {}",
                data_dir.display()
            )));
        }

        // 3. Formula.Dat
        if let Some(p) = resolve_data_file(data_dir, "Formula.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.formula_params = Some(FormulaDatLoader::load(&bytes)?);
        }

        // 4. BlissBag.Dat
        if let Some(p) = resolve_data_file(data_dir, "BlissBag.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.bliss_bags = BlissBagDatLoader::load(&bytes)?;
        }

        // 5. Compound.Dat
        if let Some(p) = resolve_data_file(data_dir, "Compound.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.compounds = CompoundDatLoader::load(&bytes)?;
        }

        // 6. Astrolabe.Dat
        if let Some(p) = resolve_data_file(data_dir, "Astrolabe.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.astrolabes = AstrolabeDatLoader::load(&bytes)?;
        }

        // 7. CityEx.Dat
        if let Some(p) = resolve_data_file(data_dir, "CityEx.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.city_ex = CityExDatLoader::load(&bytes)?;
        }

        // 8. EVOStatus.Dat
        if let Some(p) = resolve_data_file(data_dir, "EVOStatus.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.evo_statuses = EVOStatusDatLoader::load(&bytes)?;
        }

        // 9. Warp.Dat / Warps.txt
        if let Some(p) = resolve_data_file(data_dir, "Warp.Dat") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.warp_defs = WarpDatLoader::load(&bytes)?;
        }
        if let Some(p) = resolve_data_file(data_dir, "Warps.txt") {
            d.load_warps(&p)?;
        }

        // 10. Skills.txt
        if let Some(p) = resolve_data_file(data_dir, "Skills.txt") {
            d.load_skills(&p)?;
        }

        // 11. Optional game text files
        if let Some(p) = resolve_data_file(data_dir, "BattleGate.txt") {
            d.load_battle_gates(&p)?;
        }
        if let Some(p) = resolve_data_file(data_dir, "Dolls.txt") {
            d.load_dolls(&p)?;
        }
        if let Some(p) = resolve_data_file(data_dir, "NpcOnMap.txt") {
            d.load_npc_on_map(&p)?;
        }
        if let Some(p) = resolve_data_file(data_dir, "ItemOnMap.txt") {
            d.load_item_on_map(&p)?;
        }
        let quests_dir = data_dir.join("Quests");
        if quests_dir.is_dir() {
            d.load_talks(&quests_dir)?;
        }

        // 12. eve.emg (binary quest and scene events container)
        if let Some(p) = resolve_data_file(data_dir, "eve.emg") {
            let bytes = std::fs::read(&p)
                .map_err(|e| TsError::Data(format!("read {}: {}", p.display(), e)))?;
            d.scene_eve_data = EveDataLoader::load(&bytes)?;
        }

        d.texps = compute_texps();
        d.loaded = true;
        Ok(d)
    }

    fn load_binary(&mut self, data_dir: &Path) -> Result<()> {
        self.load_binary_asset_inventory(data_dir)?;

        let item_path = resolve_data_file(data_dir, "Item.dat").ok_or_else(|| {
            TsError::Data(format!("missing binary Item.dat in {}", data_dir.display()))
        })?;
        let item_bytes = std::fs::read(&item_path)
            .map_err(|e| TsError::Data(format!("read {}: {}", item_path.display(), e)))?;
        self.item_defs = ItemDatLoader::load(&item_bytes)?;
        for def in self.item_defs.values() {
            self.items.insert(def.id as i64, def.to_item());
        }

        let npc_path = resolve_data_file(data_dir, "Npc.dat").ok_or_else(|| {
            TsError::Data(format!("missing binary Npc.dat in {}", data_dir.display()))
        })?;
        let npc_bytes = std::fs::read(&npc_path)
            .map_err(|e| TsError::Data(format!("read {}: {}", npc_path.display(), e)))?;
        self.npc_defs = NpcDatLoader::load(&npc_bytes)?;
        for def in self.npc_defs.values() {
            self.npcs.insert(def.id as i64, def.to_npc());
        }

        let optional_binary: [(&str, &str); 20] = [
            ("Formula.Dat", "formula"),
            ("BlissBag.Dat", "bliss_bag"),
            ("Compound.Dat", "compound"),
            ("Astrolabe.Dat", "astrolabe"),
            ("CityEx.Dat", "city_ex"),
            ("EVOStatus.Dat", "evo_status"),
            ("Warp.Dat", "warp"),
            // PC `Skill.Dat` is the authoritative source (preferred per ADR
            // 0002). `Skill_C.dat` is the mobile variant; if PC is absent
            // and mobile is present, fall back to the mobile parser.
            ("Skill.dat", "skill_pc"),
            ("Skill_C.dat", "skill"),
            ("Mark.Dat", "mark"),
            ("Mounts.Dat", "mount"),
            ("MountsGrow.Dat", "mount_grow"),
            ("AchievementData.Dat", "achievement"),
            ("Dispatch.Dat", "dispatch"),
            ("DispatchBonus.Dat", "dispatch_bonus"),
            ("LeaderboardInfo.Dat", "leaderboard"),
            ("SceneSet.Dat", "scene_set"),
            ("Rank.Dat", "rank"),
            ("TeachInfo.Dat", "teach_info"),
            ("eve.emg", "eve"),
        ];
        for (file_name, _) in optional_binary {
            let Some(path) = resolve_data_file(data_dir, file_name) else {
                continue;
            };
            let bytes = std::fs::read(&path)
                .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
            match file_name.to_ascii_lowercase().as_str() {
                "formula.dat" => self.formula_params = Some(FormulaDatLoader::load(&bytes)?),
                "blissbag.dat" => self.bliss_bags = BlissBagDatLoader::load(&bytes)?,
                "compound.dat" => self.compounds = CompoundDatLoader::load(&bytes)?,
                "astrolabe.dat" => self.astrolabes = AstrolabeDatLoader::load(&bytes)?,
                "cityex.dat" => self.city_ex = CityExDatLoader::load(&bytes)?,
                "evostatus.dat" => self.evo_statuses = EVOStatusDatLoader::load(&bytes)?,
                "warp.dat" => self.warp_defs = WarpDatLoader::load(&bytes)?,
                "skill.dat" => self.binary_skill_defs = SkillDatLoaderPc::load(&bytes)?,
                "skill_c.dat" => self.binary_skill_defs = SkillDatLoader::load(&bytes)?,
                "mark.dat" => {
                    let (defs, reverse) = MarkDatLoader::load(&bytes)?;
                    self.mark_defs = defs;
                    self.bit_to_mission_id = reverse;
                }
                "mounts.dat" => self.mount_defs = MountDatLoader::load(&bytes)?,
                "mountsgrow.dat" => self.mount_grow_defs = MountGrowDatLoader::load(&bytes)?,
                "achievementdata.dat" => {
                    self.achievement_defs = AchievementDatLoader::load(&bytes)?;
                }
                "dispatch.dat" => self.dispatch_defs = DispatchDatLoader::load_dispatch(&bytes)?,
                "dispatchbonus.dat" => {
                    self.dispatch_bonus_defs = DispatchDatLoader::load_bonus(&bytes)?;
                }
                "leaderboardinfo.dat" => {
                    self.leaderboard_defs = LeaderboardDatLoader::load(&bytes)?;
                }
                "sceneset.dat" => self.scene_set_defs = SceneSetDatLoader::load(&bytes)?,
                "rank.dat" => self.rank_defs = RankDatLoader::load(&bytes)?,
                "teachinfo.dat" => {
                    let result = TeachInfoDatLoader::load(&bytes)?;
                    self.guide_last_bit_flag_id = result.guide_last_bit_flag_id;
                    self.guide_mark_flag_ids = result.guide_mark_flag_ids;
                    self.mark_flag_to_guide_id = self
                        .guide_mark_flag_ids
                        .iter()
                        .map(|(&guide_id, &mark_flag_id)| (mark_flag_id, guide_id))
                        .collect();
                }
                "eve.emg" => self.scene_eve_data = EveDataLoader::load(&bytes)?,
                _ => unreachable!("optional binary list contains unknown file"),
            }
        }
        Ok(())
    }

    /// Return accepted binary asset names without parsing their record format.
    /// This is used by diagnostics and tests; production boot still calls
    /// [`GameData::load`] and validates the required typed catalogs.
    pub fn binary_asset_inventory(data_dir: &Path) -> Result<Vec<String>> {
        Ok(Self::binary_asset_metadata_from_dir(data_dir)?
            .into_iter()
            .map(|asset| asset.source_file)
            .collect())
    }

    /// Compute sorted SHA-256 metadata for accepted root-level binary assets.
    pub fn binary_asset_metadata_from_dir(data_dir: &Path) -> Result<Vec<BinaryAssetMeta>> {
        let mut data = Self::default();
        data.load_binary_asset_inventory(data_dir)?;
        Ok(data.binary_asset_metadata())
    }

    pub fn binary_asset_metadata(&self) -> Vec<BinaryAssetMeta> {
        let mut assets: Vec<_> = self
            .raw_binary_assets
            .iter()
            .map(|(source_file, bytes)| {
                let digest = Sha256::digest(bytes);
                BinaryAssetMeta {
                    source_file: source_file.clone(),
                    byte_size: bytes.len(),
                    sha256: format!("{digest:x}"),
                }
            })
            .collect();
        assets.sort_by(|a, b| a.source_file.cmp(&b.source_file));
        assets
    }

    fn load_binary_asset_inventory(&mut self, data_dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(data_dir)
            .map_err(|e| TsError::Data(format!("read data dir {}: {}", data_dir.display(), e)))?;
        for entry in entries {
            let entry = entry.map_err(|e| {
                TsError::Data(format!("read data entry {}: {}", data_dir.display(), e))
            })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let accepted = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "dat" | "emg" | "mng"))
                .unwrap_or(false);
            if !accepted {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    TsError::Data(format!("non-UTF8 binary filename: {}", path.display()))
                })?
                .to_string();
            let bytes = std::fs::read(&path)
                .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
            self.raw_binary_assets.insert(name, bytes);
        }
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Npcs.txt — UTF-16LE+BOM, LF. Mojibake decoded back to VISCII names.
    /// Column map: 0-11 id..agi, 12-15 Skill1-4, 16-21
    /// Drop1-6, 22 NotPet(_Bat), 23 Reborn.
    fn load_npcs(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        let offset = if bytes.starts_with(&[0xFF, 0xFE]) {
            2
        } else {
            0
        };
        let u16s: Vec<u16> = bytes[offset..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let decoded = String::from_utf16_lossy(&u16s);
        for line in decoded.split('\n') {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.trim().starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let name = f.get(1).copied().unwrap_or("");
            let npc = Npc {
                id: num_at(0, &f, "Npcs.txt")?,
                name: encoding::to_viscii(name),
                lv: num_at(2, &f, "Npcs.txt")?,
                thuoctinh: num_at(3, &f, "Npcs.txt")?,
                hp: num_at(4, &f, "Npcs.txt")?,
                sp: num_at(5, &f, "Npcs.txt")?,
                hpx: num_at(6, &f, "Npcs.txt")?,
                spx: num_at(7, &f, "Npcs.txt")?,
                int1: num_at(8, &f, "Npcs.txt")?,
                atk: num_at(9, &f, "Npcs.txt")?,
                def: num_at(10, &f, "Npcs.txt")?,
                agi: num_at(11, &f, "Npcs.txt")?,
                skill: [
                    num_at(12, &f, "Npcs.txt")?,
                    num_at(13, &f, "Npcs.txt")?,
                    num_at(14, &f, "Npcs.txt")?,
                    num_at(15, &f, "Npcs.txt")?,
                ],
                item: [
                    num_at(16, &f, "Npcs.txt")?,
                    num_at(17, &f, "Npcs.txt")?,
                    num_at(18, &f, "Npcs.txt")?,
                    num_at(19, &f, "Npcs.txt")?,
                    num_at(20, &f, "Npcs.txt")?,
                    num_at(21, &f, "Npcs.txt")?,
                ],
                bat: num_at(22, &f, "Npcs.txt")?,
                reborn: num_at(23, &f, "Npcs.txt")?,
                garble: encoding::compute_garble(name),
            };
            self.npcs.insert(npc.id, npc);
        }
        Ok(())
    }

    /// Items.txt — UTF-8 no BOM, CRLF, CP1252-mojibake → VISCII.
    fn load_items(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.trim().starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let name = f.get(1).copied().unwrap_or("");
            let item = Item {
                id: num_at(0, &f, "Items.txt")?,
                name: encoding::to_viscii(name),
                level: num_at(2, &f, "Items.txt")?,
                hp: num_at(3, &f, "Items.txt")?,
                sp: num_at(4, &f, "Items.txt")?,
                int1: num_at(5, &f, "Items.txt")?,
                atk1: num_at(6, &f, "Items.txt")?,
                def1: num_at(7, &f, "Items.txt")?,
                hpx1: num_at(8, &f, "Items.txt")?,
                spx1: num_at(9, &f, "Items.txt")?,
                agi1: num_at(10, &f, "Items.txt")?,
                fai1: num_at(11, &f, "Items.txt")?,
                int2: num_at(12, &f, "Items.txt")?,
                atk2: num_at(13, &f, "Items.txt")?,
                def2: num_at(14, &f, "Items.txt")?,
                hpx2: num_at(15, &f, "Items.txt")?,
                spx2: num_at(16, &f, "Items.txt")?,
                agi2: num_at(17, &f, "Items.txt")?,
                fai2: num_at(18, &f, "Items.txt")?,
                thuoctinh: num_at(19, &f, "Items.txt")?,
                value: num_at(20, &f, "Items.txt")?,
                loai: num_at(21, &f, "Items.txt")?,
                rb_pet_from: num_at(22, &f, "Items.txt")?,
                rb_pet_to: num_at(23, &f, "Items.txt")?,
                add_pet: num_at(24, &f, "Items.txt")?,
                garble: encoding::compute_garble(name),
            };
            self.items.insert(item.id, item);
        }
        Ok(())
    }

    /// Skills.txt — UTF-8 proper Unicode; names GUI-only, never in packets.
    fn load_skills(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.trim().starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let skill = Skill {
                id: num_at(0, &f, "Skills.txt")?,
                name: f.get(1).copied().unwrap_or("").to_string(),
                sp: num_at(2, &f, "Skills.txt")?,
                point: num_at(3, &f, "Skills.txt")?,
                thuoctinh: num_at(4, &f, "Skills.txt")?,
                id_dk: [
                    num_at(5, &f, "Skills.txt")?,
                    num_at(6, &f, "Skills.txt")?,
                    num_at(7, &f, "Skills.txt")?,
                    num_at(8, &f, "Skills.txt")?,
                    num_at(9, &f, "Skills.txt")?,
                    num_at(10, &f, "Skills.txt")?,
                ],
                lv_max: num_at(11, &f, "Skills.txt")?,
                skill_type: num_at(12, &f, "Skills.txt")?,
                do_manh: num_at(13, &f, "Skills.txt")?,
                sl_danh: num_at(14, &f, "Skills.txt")?,
                reborn: num_at(15, &f, "Skills.txt")?,
                combo: num_at(16, &f, "Skills.txt")?,
                delay: num_at(17, &f, "Skills.txt")?,
                troi_buff: num_at(18, &f, "Skills.txt")?,
            };
            self.skills.insert(skill.id, skill);
        }
        Ok(())
    }

    /// Warps.txt — ASCII, terminator `text.Length < 5`, skip empty destination
    /// column.
    fn load_warps(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.starts_with("//") {
                continue;
            }
            if line.trim().len() < 5 {
                break;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.get(2).map(|v| v.is_empty()).unwrap_or(true) {
                continue; // destination map column empty -> silently dropped
            }
            let warp = Warp {
                map1: num_at(0, &f, "Warps.txt")?,
                warpid: num_at(1, &f, "Warps.txt")?,
                map2: num_at(2, &f, "Warps.txt")?,
                x: num_at(3, &f, "Warps.txt")?,
                y: num_at(4, &f, "Warps.txt")?,
            };
            self.warps.insert((warp.map1, warp.warpid), warp);
        }
        Ok(())
    }

    /// BattleGate.txt — ASCII, terminator `text.Length < 5`.
    fn load_battle_gates(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.starts_with("//") {
                continue;
            }
            if line.trim().len() < 5 {
                break;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let mut defenders = [0i64; 10];
            for (i, defender) in defenders.iter_mut().enumerate() {
                *defender = num_at(3 + i, &f, "BattleGate.txt")?;
            }
            let gate = BattleGate {
                mapid1: num_at(0, &f, "BattleGate.txt")?,
                warpid: num_at(1, &f, "BattleGate.txt")?,
                diahinh: num_at(2, &f, "BattleGate.txt")?,
                defenders,
            };
            self.battle_gates.insert((gate.mapid1, gate.warpid), gate);
        }
        Ok(())
    }

    /// Dolls.txt — ASCII.
    fn load_dolls(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let doll = Doll {
                doll_id: num_at(0, &f, "Dolls.txt")?,
                npc_id: num_at(1, &f, "Dolls.txt")?,
            };
            self.dolls.insert(doll.doll_id, doll);
        }
        Ok(())
    }

    /// NpcOnMap.txt — ASCII (map spawns/patrol list).
    fn load_npc_on_map(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            self.npc_on_map.push(NpcOnMap {
                map_id: num_at(0, &f, "NpcOnMap.txt")?,
                id: num_at(1, &f, "NpcOnMap.txt")?,
                npc_id: num_at(2, &f, "NpcOnMap.txt")?,
                x: num_at(3, &f, "NpcOnMap.txt")?,
                y: num_at(4, &f, "NpcOnMap.txt")?,
                coord: num_at(5, &f, "NpcOnMap.txt")?,
                so_luong: num_at(6, &f, "NpcOnMap.txt")?,
            });
        }
        Ok(())
    }

    /// ItemOnMap.txt — ASCII. First appearance of a MapId pre-fills empty
    /// slots 1..255 in `ItemDropOnMap`; each row spawns a static drop with
    /// `_Delay=999999`. The load-time broadcast `F44408001703` fires with no
    /// clients connected (no-op); `static_drop_frame` exposes the same frame
    /// for maps with live clients.
    pub fn load_item_on_map(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        let mut seen_maps: std::collections::HashSet<i64> = Default::default();
        let mut seen_keys: std::collections::HashSet<(i64, i64, i64, i64)> = Default::default();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() {
                break;
            }
            if line.starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let map_id = num_at(0, &f, "ItemOnMap.txt")?;
            if seen_maps.insert(map_id) {
                for slot in 1..=255 {
                    self.item_drop_on_map.insert(
                        (map_id, slot),
                        ItemDropOnMap {
                            map_id,
                            slot,
                            ..Default::default()
                        },
                    );
                }
            }
            let slot = num_at(1, &f, "ItemOnMap.txt")?;
            let item_id = num_at(2, &f, "ItemOnMap.txt")?;
            let x = num_at(3, &f, "ItemOnMap.txt")?;
            let y = num_at(4, &f, "ItemOnMap.txt")?;
            let delay = num_at(5, &f, "ItemOnMap.txt")?;
            // Duplicate `(mapId, itemId, x, y)` rows are skipped — no re-spawn.
            if !seen_keys.insert((map_id, item_id, x, y)) {
                continue;
            }
            self.item_on_map.push(ItemOnMap {
                map_id,
                id: slot,
                item_id,
                x,
                y,
                delay,
            });
            // Spawn the static drop `(mapid, slot, x, y, itemId, delay=999999)`:
            // copies the item's full stats, `_Delay=999999`, `_Gold=3`.
            let item = self.items.get(&item_id).ok_or_else(|| {
                TsError::Data(format!("ItemOnMap references unknown item {item_id}"))
            })?;
            let drop = ItemDropOnMap {
                map_id,
                slot,
                item_id,
                map_x: x,
                map_y: y,
                delay: 999_999,
                count: 1,
                lv: item.level,
                doben: 0,
                int1: item.int1,
                atk1: item.atk1,
                def1: item.def1,
                hpx1: item.hpx1,
                spx1: item.spx1,
                agi1: item.agi1,
                fai1: item.fai1,
                int2: item.int2,
                atk2: item.atk2,
                def2: item.def2,
                hpx2: item.hpx2,
                spx2: item.spx2,
                agi2: item.agi2,
                fai2: item.fai2,
                hp: item.hp,
                sp: item.sp,
                long_val: 0,
                giatri_long: 0,
                khang: 0,
                thuoctinh: item.thuoctinh,
                giatri_thuoctinh: item.value,
                loai: item.loai,
                texp: 0,
                gold: 3,
            };
            self.item_drop_on_map.insert((map_id, slot), drop);
        }
        Ok(())
    }

    /// Broadcast frame for a spawned static drop:
    /// `F44408001703` + le16(itemId) + le16(x) + le16(y).
    pub fn static_drop_frame(item_id: i64, x: i64, y: i64) -> String {
        format!(
            "F44408001703{}{}{}",
            crate::protocol::encoder::le16(item_id as u16),
            crate::protocol::encoder::le16(x as u16),
            crate::protocol::encoder::le16(y as u16)
        )
    }

    /// Quests/*.ini — 813 files, Win32 INI semantics.
    fn load_talks(&mut self, quest_dir: &Path) -> Result<()> {
        if !quest_dir.is_dir() {
            return Ok(());
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(quest_dir)
            .map_err(|e| TsError::Data(format!("read dir {}: {}", quest_dir.display(), e)))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "ini").unwrap_or(false))
            .collect();
        files.sort();
        for path in files {
            let q = self.parse_quest_ini(&path)?;
            let key = format!("{}:{}:{}:{}", q.map_id, q.talk_type, q.id, q.step);
            self.talks.insert(key, q);
        }
        Ok(())
    }

    pub fn parse_quest_ini(&self, path: &Path) -> Result<QuestDef> {
        let bytes = std::fs::read(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        let s = String::from_utf8_lossy(&bytes);
        let ini = Ini::parse(&s);
        let file = path.to_string_lossy().to_string();

        let mut q = QuestDef {
            map_id: num(&ini.get("BASE", "MapId"), &file)?,
            talk_type: ini.get("BASE", "Type"),
            id: num(&ini.get("BASE", "Id"), &file)?,
            step: num(&ini.get("BASE", "Step"), &file)?,
            dialogs: ini.get("BASE", "Dialogs"),
            ..Default::default()
        };

        if ini.has_section("TEAMDEF") {
            let mut v = Vec::with_capacity(11);
            // Diahinh: absent -> 0.
            let diahinh = ini.get("TEAMDEF", "Diahinh");
            v.push(if diahinh == NOTHING || diahinh.trim().is_empty() {
                0
            } else {
                num(&diahinh, &file)?
            });
            // Npcs: absent or not exactly 10 elements -> int[10] zeros; else
            // the 10 parsed ids.
            let npcs = ini.get("TEAMDEF", "Npcs");
            let mut npc_ids = [0i64; 10];
            if npcs != NOTHING {
                let toks: Vec<&str> = npcs.split('\t').map(str::trim).collect();
                if toks.len() == 10 && toks.iter().all(|t| !t.is_empty()) {
                    for (i, t) in toks.iter().enumerate() {
                        npc_ids[i] = num(t, &file)?;
                    }
                }
            }
            v.extend_from_slice(&npc_ids);
            q.teamdef = v;
        }

        // [REQUIRES] — entry conditions.
        let mut required_items: Option<Vec<(i64, i64, i64)>> = None;
        if ini.has_section("REQUIRES") {
            let rm = ini.get("REQUIRES", "SelectMenu");
            q.require_select_menu = if rm == NOTHING || rm.trim().is_empty() {
                0
            } else {
                num(&rm, &file)?
            };
            q.require_level = parse_condition(&ini.get("REQUIRES", "Level"), &file)?;
            q.require_reborn = parse_condition(&ini.get("REQUIRES", "Reborn"), &file)?;
            let thuoctinh = ini.get("REQUIRES", "Thuoctinh");
            q.require_thuoctinh = if thuoctinh == NOTHING || thuoctinh.trim().is_empty() {
                0
            } else {
                num(&thuoctinh, &file)?
            };
            q.require_quests = parse_quest_tuples(&ini.get("REQUIRES", "Quests"), &file)?;
            q.require_wears = parse_wear_tuples(&ini.get("REQUIRES", "Wears"), &file)?;
            // Items consumed on win (`_RequireItems`): itemId-count-remove.
            required_items = Some(parse_tuples(&ini.get("REQUIRES", "Items"), &file)?);
        }

        // `parse_result` rebuilds `OnWin` and would clobber `require_items`
        // (ticket 19 #4: the INI writes them under [REQUIRES], but they act on
        // win) — re-apply after.
        q.on_win = self.parse_result(&ini, "OnWin", &file)?;
        if let Some(items) = required_items {
            q.on_win.require_items = items;
        }
        // SaveLeaderQuests / SaveMemberQuests need map_id/type/id/step for AUTO.
        let win_qs = ini.get("ONWIN", "SaveLeaderQuests");
        if win_qs != NOTHING {
            q.on_win.save_leader_quests =
                parse_save_quest(&win_qs, &file, q.map_id, &q.talk_type, q.id, q.step);
        }
        let win_ms = ini.get("ONWIN", "SaveMemberQuests");
        if win_ms != NOTHING {
            q.on_win.save_member_quests =
                parse_save_quest(&win_ms, &file, q.map_id, &q.talk_type, q.id, q.step);
        }
        // [OnLose].WarpTo is read from ONWIN (a quirk the spec keeps):
        // OnLose.WarpTo always equals OnWin.WarpTo.
        let mut on_lose = self.parse_result(&ini, "OnLose", &file)?;
        on_lose.warp_to = q.on_win.warp_to.clone();
        q.on_lose = on_lose;
        // [DESCRIPTION] Title — server-GUI requirement messages.
        let title = ini.get("DESCRIPTION", "Title");
        if title != NOTHING {
            q.desc_title = title;
        }
        Ok(q)
    }

    fn parse_result(&self, ini: &Ini, section: &str, file: &str) -> Result<QuestResult> {
        let mut r = QuestResult {
            dialogs: ini.get(section, "Dialogs"),
            ..Default::default()
        };
        let warp = ini.get(section, "WarpTo");
        if warp != NOTHING {
            r.warp_to = parse_warp(&warp, file)?;
        }
        let msg = ini.get(section, "Message");
        if msg != NOTHING {
            r.message = msg;
        }
        r.rewards = parse_tuples(&ini.get(section, "Rewards"), file)?;
        r.random_rewards = parse_tuples(&ini.get(section, "RandomRewards"), file)?;
        r.use_items = parse_use_items(&ini.get(section, "UseItems"), file)?;
        r.player_enhance_data = parse_enhance(&ini.get(section, "PlayerEnhanceData"), file)?;
        r.add_skill = parse_add_skill(&ini.get(section, "AddSkill"), file)?;
        r.add_pet = parse_add_pet(&ini.get(section, "AddPet"), file)?;
        r.click_npc_id = num_or(&ini.get(section, "ClickNpcId"), file)?;
        Ok(r)
    }
}

fn parse_tuples(s: &str, file: &str) -> Result<Vec<(i64, i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING || s.trim().is_empty() {
        return Ok(out);
    }
    // listSplit = '\t'; each tuple is `a-b-c` or `a-b`.
    for tok in s.split('\t') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        let parts: Vec<&str> = t.split('-').map(str::trim).collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let a = parts[0]
            .parse::<i64>()
            .map_err(|_| TsError::Data(format!("bad tuple `{tok}` in {file}")))?;
        let b = parts
            .get(1)
            .copied()
            .unwrap_or("0")
            .parse::<i64>()
            .map_err(|_| TsError::Data(format!("bad tuple `{tok}` in {file}")))?;
        let c = match parts.get(2) {
            Some(x) if !x.trim().is_empty() => x
                .trim()
                .parse::<i64>()
                .map_err(|_| TsError::Data(format!("bad tuple `{tok}` in {file}")))?,
            _ => 0,
        };
        out.push((a, b, c));
    }
    Ok(out)
}

/// `[REQUIRES] Level/Reborn` — `value\top`; operator index over
/// `["=",">=",">","<=","<","!="]` → 0..5. Absent key → `None`
/// (= no condition, NOT a `= 0` requirement).
fn parse_condition(s: &str, file: &str) -> Result<Option<(i64, i64)>> {
    if s == NOTHING || s.trim().is_empty() {
        return Ok(None);
    }
    let mut it = s.split('\t');
    let value = it
        .next()
        .unwrap_or("")
        .trim()
        .parse::<i64>()
        .map_err(|_| TsError::Data(format!("bad condition `{s}` in {file}")))?;
    let op = it.next().unwrap_or("").trim();
    let ops = ["=", ">=", ">", "<=", "<", "!="];
    let op_index = ops
        .iter()
        .position(|&o| o == op)
        .map(|i| i as i64)
        .unwrap_or(-1);
    Ok(Some((value, op_index)))
}

/// `[REQUIRES] Quests` — tab-separated `mapId-npcId-warpId-step` tuples.
fn parse_quest_tuples(s: &str, file: &str) -> Result<Vec<(i64, i64, i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING || s.trim().is_empty() {
        return Ok(out);
    }
    for tok in s.split('\t') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        let parts: Vec<i64> = t
            .split('-')
            .map(|p| {
                p.trim()
                    .parse::<i64>()
                    .map_err(|_| TsError::Data(format!("bad quest tuple `{tok}` in {file}")))
            })
            .collect::<Result<Vec<_>>>()?;
        if parts.is_empty() {
            continue;
        }
        let mut v = [0i64; 4];
        for (i, p) in parts.iter().take(4).enumerate() {
            v[i] = *p;
        }
        out.push((v[0], v[1], v[2], v[3]));
    }
    Ok(out)
}

/// `[REQUIRES] Wears` — tab-separated `itemId-playerOrPet` tuples.
fn parse_wear_tuples(s: &str, file: &str) -> Result<Vec<(i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING || s.trim().is_empty() {
        return Ok(out);
    }
    for tok in s.split('\t') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        let parts: Vec<&str> = t.split('-').map(str::trim).collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let a = parts[0]
            .parse::<i64>()
            .map_err(|_| TsError::Data(format!("bad wears tuple `{tok}` in {file}")))?;
        let b = parts
            .get(1)
            .copied()
            .unwrap_or("0")
            .parse::<i64>()
            .map_err(|_| TsError::Data(format!("bad wears tuple `{tok}` in {file}")))?;
        out.push((a, b));
    }
    Ok(out)
}

/// UseItems — `itemId-target-?` tuples (listSplit `\t`, intSplit `-`).
fn parse_use_items(s: &str, file: &str) -> Result<Vec<(i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING || s.trim().is_empty() {
        return Ok(out);
    }
    for tok in s.split('\t') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        let parts: Vec<&str> = t.split('-').map(str::trim).collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let item_id = parts[0]
            .parse::<i64>()
            .map_err(|_| TsError::Data(format!("bad UseItems `{tok}` in {file}")))?;
        let target = parts
            .get(1)
            .copied()
            .unwrap_or("0")
            .parse::<i64>()
            .map_err(|_| TsError::Data(format!("bad UseItems `{tok}` in {file}")))?;
        out.push((item_id, target));
    }
    Ok(out)
}

/// SaveLeaderQuests/SaveMemberQuests — `npcId-npcVal-warpVal-plus` tuples,
/// with the `AUTO` token expanded to `mapId-id-step+1` (id goes in the npc
/// column for `Type=NPC`, in the warp column for `Type=WARP`).
fn parse_save_quest(
    s: &str,
    _file: &str,
    map_id: i64,
    talk_type: &str,
    id: i64,
    step: i64,
) -> Vec<(i64, i64, i64, i64)> {
    let mut out = Vec::new();
    if s == NOTHING || s.trim().is_empty() {
        return out;
    }
    for tok in s.split('\t') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        if t == "AUTO" {
            let (npc_val, warp_val) = if talk_type == "WARP" {
                (0, id)
            } else {
                (id, 0)
            };
            out.push((map_id, npc_val, warp_val, step + 1));
            continue;
        }
        let parts: Vec<i64> = t.split('-').filter_map(|p| p.trim().parse().ok()).collect();
        let mut v = [0i64; 4];
        for (i, p) in parts.iter().take(4).enumerate() {
            v[i] = *p;
        }
        out.push((v[0], v[1], v[2], v[3]));
    }
    out
}

/// WarpTo is tab-separated `map x y` (sometimes the `-` tuple form).
fn parse_warp(s: &str, file: &str) -> Result<Vec<i64>> {
    let mut out = Vec::new();
    if s == NOTHING {
        return Ok(out);
    }
    for tok in s.split(['\t', ',']) {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        for n in t.split('-') {
            let n = n.trim();
            if n.is_empty() {
                continue;
            }
            out.push(
                n.parse::<i64>()
                    .map_err(|_| TsError::Data(format!("bad WarpTo `{tok}` in {file}")))?,
            );
        }
    }
    Ok(out)
}

fn num_or(s: &str, file: &str) -> Result<i64> {
    if s == NOTHING || s.trim().is_empty() {
        return Ok(0);
    }
    s.trim()
        .parse::<i64>()
        .map_err(|_| TsError::Data(format!("non-numeric field `{s}` in {file}")))
}

/// PlayerEnhanceData — tab-separated `Stat-Δ` pairs.
fn parse_enhance(s: &str, file: &str) -> Result<Vec<(String, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING {
        return Ok(out);
    }
    for tok in s.split(['\t', ',']) {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(eq) = t.find('-') {
            let stat = t[..eq].to_string();
            let delta = t[eq + 1..]
                .trim()
                .parse::<i64>()
                .map_err(|_| TsError::Data(format!("bad enhance `{t}` in {file}")))?;
            out.push((stat, delta));
        }
    }
    Ok(out)
}

/// AddSkill — `skillId\tlevel` parsed from a flat int list; the first two
/// elements are the skill id and its level.
fn parse_add_skill(s: &str, file: &str) -> Result<Vec<(i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING || s.trim().is_empty() {
        return Ok(out);
    }
    let ints: Vec<i64> = s
        .split(['\t', ','])
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            t.parse::<i64>()
                .map_err(|_| TsError::Data(format!("bad AddSkill `{s}` in {file}")))
        })
        .collect::<Result<Vec<_>>>()?;
    if ints.len() >= 2 {
        out.push((ints[0], ints[1]));
    }
    Ok(out)
}

/// AddPet — comma/tab-separated npc ids.
fn parse_add_pet(s: &str, _file: &str) -> Result<Vec<i64>> {
    let mut out = Vec::new();
    if s == NOTHING {
        return Ok(out);
    }
    for tok in s.split(['\t', ',']) {
        let t = tok.trim();
        if !t.is_empty() {
            if let Ok(n) = t.parse::<i64>() {
                out.push(n);
            }
        }
    }
    Ok(out)
}
