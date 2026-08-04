//! Static data loading (Chapter 3). Loads the `Data/` directory byte-identical
//! into the in-memory tables, following each file's encoding + row convention.
//! `Loaded()` sets `DataLoaded=true` (the TCP accept gate).

use crate::data::ini::{Ini, NOTHING};
use crate::data::tables::*;
use crate::data::texps::compute_texps;
use crate::encoding;
use crate::error::{Result, TsError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The complete loaded static dataset (mirrors the C# `Data` statics).
#[derive(Debug, Default)]
pub struct GameData {
    pub npcs: HashMap<i64, Npc>,
    pub items: HashMap<i64, Item>,
    pub skills: HashMap<i64, Skill>,
    pub warps: HashMap<(i64, i64), Warp>,
    pub battle_gates: HashMap<(i64, i64), BattleGate>,
    pub dolls: HashMap<i64, Doll>,
    pub talks: HashMap<String, QuestDef>,
    pub texps: Vec<TexpRow>,
    pub npc_on_map: Vec<NpcOnMap>,
    pub item_on_map: Vec<ItemOnMap>,
    pub loaded: bool,
}

/// Render a data table into a temp dir for tests. Not part of the runtime.
#[doc(hidden)]
#[cfg(test)]
pub fn seed_temp_dir(dir: &Path, files: &[(&str, &[u8])]) {
    for (name, data) in files {
        std::fs::write(dir.join(name), data).unwrap();
    }
}

fn num(field: &str, file: &str) -> Result<i64> {
    field
        .trim()
        .parse::<i64>()
        .map_err(|_| TsError::Data(format!("non-numeric field `{field}` in {file}")))
}

fn num_or_default(idx: usize, f: &[&str], dflt: i64, file: &str) -> Result<i64> {
    match f.get(idx) {
        Some(s) if !s.trim().is_empty() => num(s, file),
        _ => Ok(dflt),
    }
}

impl GameData {
    /// Load the entire dataset under `data_dir`.
    pub fn load(data_dir: &Path) -> Result<Self> {
        let mut d = Self::default();
        for name in ["Npcs.txt", "Items.txt", "Skills.txt"] {
            let p = data_dir.join(name);
            if !p.exists() {
                return Err(TsError::Data(format!("missing data file: {}", p.display())));
            }
        }
        d.load_npcs(&data_dir.join("Npcs.txt"))?;
        d.load_items(&data_dir.join("Items.txt"))?;
        d.load_skills(&data_dir.join("Skills.txt"))?;
        d.load_warps(&data_dir.join("Warps.txt"))?;
        d.load_battle_gates(&data_dir.join("BattleGate.txt"))?;
        d.load_dolls(&data_dir.join("Dolls.txt"))?;
        d.load_npc_on_map(&data_dir.join("NpcOnMap.txt"))?;
        d.load_item_on_map(&data_dir.join("ItemOnMap.txt"))?;
        d.load_talks(&data_dir.join("Quests"))?;
        d.texps = compute_texps();
        d.loaded = true;
        Ok(d)
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Npcs.txt — UTF-16LE+BOM, LF. Mojibake decoded back to VISCII names.
    fn load_npcs(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        let offset = if bytes.starts_with(&[0xFF, 0xFE]) { 2 } else { 0 };
        let u16s: Vec<u16> = bytes[offset..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let decoded = String::from_utf16_lossy(&u16s);
        for line in decoded.split('\n') {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.trim().starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 14 {
                continue;
            }
            let npc = Npc {
                id: num(f[0], "Npcs.txt")?,
                name: encoding::to_viscii(f[1]),
                lv: num(f[2], "Npcs.txt")?,
                thuoctinh: num(f[3], "Npcs.txt")?,
                hp: num(f[4], "Npcs.txt")?,
                sp: num(f[5], "Npcs.txt")?,
                hpx: num(f[6], "Npcs.txt")?,
                spx: num(f[7], "Npcs.txt")?,
                int1: num(f[8], "Npcs.txt")?,
                atk: num(f[9], "Npcs.txt")?,
                def: num(f[10], "Npcs.txt")?,
                agi: num(f[11], "Npcs.txt")?,
                skill: [
                    num(f[12], "Npcs.txt")?,
                    num_or_default(13, &f, 0, "Npcs.txt")?,
                    num_or_default(14, &f, 0, "Npcs.txt")?,
                    num_or_default(15, &f, 0, "Npcs.txt")?,
                ],
                item: [0; 6],
                bat: num_or_default(16, &f, 0, "Npcs.txt")?,
                reborn: num_or_default(17, &f, 0, "Npcs.txt")?,
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
            if line.trim().is_empty() || line.trim().starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 19 {
                continue;
            }
            let item = Item {
                id: num(f[0], "Items.txt")?,
                name: encoding::to_viscii(f[1]),
                level: num(f[2], "Items.txt")?,
                hp: num(f[3], "Items.txt")?,
                sp: num(f[4], "Items.txt")?,
                int1: num(f[5], "Items.txt")?,
                atk1: num(f[6], "Items.txt")?,
                def1: num(f[7], "Items.txt")?,
                hpx1: num(f[8], "Items.txt")?,
                spx1: num(f[9], "Items.txt")?,
                agi1: num(f[10], "Items.txt")?,
                fai1: num(f[11], "Items.txt")?,
                int2: num(f[12], "Items.txt")?,
                atk2: num(f[13], "Items.txt")?,
                def2: num(f[14], "Items.txt")?,
                hpx2: num(f[15], "Items.txt")?,
                spx2: num(f[16], "Items.txt")?,
                agi2: num(f[17], "Items.txt")?,
                fai2: num(f[18], "Items.txt")?,
                thuoctinh: num_or_default(19, &f, 0, "Items.txt")?,
                value: num_or_default(20, &f, 0, "Items.txt")?,
                loai: num_or_default(21, &f, 0, "Items.txt")?,
                rb_pet_from: num_or_default(22, &f, 0, "Items.txt")?,
                rb_pet_to: num_or_default(23, &f, 0, "Items.txt")?,
                add_pet: num_or_default(24, &f, 0, "Items.txt")?,
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
            if line.trim().is_empty() || line.trim().starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 5 {
                continue;
            }
            let skill = Skill {
                id: num(f[0], "Skills.txt")?,
                name: f[1].to_string(),
                sp: num(f[2], "Skills.txt")?,
                point: num(f[3], "Skills.txt")?,
                thuoctinh: num(f[4], "Skills.txt")?,
                id_dk: [
                    num_or_default(5, &f, 0, "Skills.txt")?,
                    num_or_default(6, &f, 0, "Skills.txt")?,
                    num_or_default(7, &f, 0, "Skills.txt")?,
                    num_or_default(8, &f, 0, "Skills.txt")?,
                    num_or_default(9, &f, 0, "Skills.txt")?,
                    num_or_default(10, &f, 0, "Skills.txt")?,
                ],
                lv_max: num_or_default(11, &f, 0, "Skills.txt")?,
                skill_type: num_or_default(12, &f, 0, "Skills.txt")?,
                do_manh: num_or_default(13, &f, 0, "Skills.txt")?,
                sl_danh: num_or_default(14, &f, 0, "Skills.txt")?,
                reborn: num_or_default(15, &f, 0, "Skills.txt")?,
                combo: num_or_default(16, &f, 0, "Skills.txt")?,
                delay: num_or_default(17, &f, 0, "Skills.txt")?,
                troi_buff: num_or_default(18, &f, 0, "Skills.txt")?,
            };
            self.skills.insert(skill.id, skill);
        }
        Ok(())
    }

    /// Warps.txt — ASCII, terminator `text.Length < 5`.
    fn load_warps(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with("//") {
                continue;
            }
            if line.trim().len() < 5 {
                break;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 5 {
                continue;
            }
            let warp = Warp {
                map1: num(f[0], "Warps.txt")?,
                warpid: num(f[1], "Warps.txt")?,
                map2: num(f[2], "Warps.txt")?,
                x: num(f[3], "Warps.txt")?,
                y: num(f[4], "Warps.txt")?,
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
            if line.trim().is_empty() || line.starts_with("//") {
                continue;
            }
            if line.trim().len() < 5 {
                break;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 3 {
                continue;
            }
            let mut defenders = [0i64; 10];
            for i in 0..10 {
                defenders[i] = num_or_default(3 + i, &f, 0, "BattleGate.txt")?;
            }
            let gate = BattleGate {
                mapid1: num(f[0], "BattleGate.txt")?,
                warpid: num(f[1], "BattleGate.txt")?,
                diahinh: num(f[2], "BattleGate.txt")?,
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
            if line.trim().is_empty() || line.starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 2 {
                continue;
            }
            let doll = Doll {
                doll_id: num(f[0], "Dolls.txt")?,
                npc_id: num(f[1], "Dolls.txt")?,
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
            if line.trim().is_empty() || line.starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 7 {
                continue;
            }
            self.npc_on_map.push(NpcOnMap {
                map_id: num(f[0], "NpcOnMap.txt")?,
                id: num(f[1], "NpcOnMap.txt")?,
                npc_id: num(f[2], "NpcOnMap.txt")?,
                x: num(f[3], "NpcOnMap.txt")?,
                y: num(f[4], "NpcOnMap.txt")?,
                coord: num(f[5], "NpcOnMap.txt")?,
                so_luong: num(f[6], "NpcOnMap.txt")?,
            });
        }
        Ok(())
    }

    /// ItemOnMap.txt — ASCII.
    fn load_item_on_map(&mut self, path: &Path) -> Result<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with("//") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 6 {
                continue;
            }
            self.item_on_map.push(ItemOnMap {
                map_id: num(f[0], "ItemOnMap.txt")?,
                id: num(f[1], "ItemOnMap.txt")?,
                item_id: num(f[2], "ItemOnMap.txt")?,
                x: num(f[3], "ItemOnMap.txt")?,
                y: num(f[4], "ItemOnMap.txt")?,
                delay: num(f[5], "ItemOnMap.txt")?,
            });
        }
        Ok(())
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

    fn parse_quest_ini(&self, path: &Path) -> Result<QuestDef> {
        let bytes = std::fs::read(path)
            .map_err(|e| TsError::Data(format!("read {}: {}", path.display(), e)))?;
        let s = String::from_utf8_lossy(&bytes);
        let ini = Ini::parse(&s);
        let file = path.to_string_lossy().to_string();

        let mut q = QuestDef::default();
        q.map_id = num(&ini.get("BASE", "MapId"), &file)?;
        q.talk_type = ini.get("BASE", "Type");
        q.id = num(&ini.get("BASE", "Id"), &file)?;
        q.step = num(&ini.get("BASE", "Step"), &file)?;
        q.dialogs = ini.get("BASE", "Dialogs");

        if ini.has_section("TEAMDEF") {
            let mut v = Vec::with_capacity(11);
            v.push(num(&ini.get("TEAMDEF", "Diahinh"), &file)?);
            let npcs = ini.get("TEAMDEF", "Npcs");
            // Npcs are separated by tabs (sometimes commas like tuples).
            if npcs != NOTHING {
                for tok in npcs.split(['\t', ',']) {
                    let t = tok.trim();
                    if !t.is_empty() {
                        v.push(num(t, &file)?);
                    }
                }
            }
            q.teamdef = v;
        }

        q.on_win = self.parse_result(&ini, "OnWin", &file)?;
        // [OnLose].WarpTo is read from ONWIN (C# copy-paste bug).
        let mut on_lose = self.parse_result(&ini, "OnLose", &file)?;
        let win_warp = ini.get("ONWIN", "WarpTo");
        if on_lose.warp_to.is_empty() && win_warp != NOTHING {
            on_lose.warp_to = parse_warp(&win_warp, &file)?;
        }
        q.on_lose = on_lose;
        Ok(q)
    }

    fn parse_result(&self, ini: &Ini, section: &str, file: &str) -> Result<QuestResult> {
        let mut r = QuestResult::default();
        r.dialogs = ini.get(section, "Dialogs");
        let warp = ini.get(section, "WarpTo");
        if warp != NOTHING {
            r.warp_to = parse_warp(&warp, file)?;
        }
        r.rewards = parse_tuples(&ini.get(section, "Rewards"), file)?;
        r.random_rewards = parse_tuples(&ini.get(section, "RandomRewards"), file)?;
        r.use_items = parse_tuples(&ini.get(section, "UseItems"), file)?;
        r.save_leader_quests = parse_int_list(&ini.get(section, "SaveLeaderQuests"));
        r.save_member_quests = parse_int_list(&ini.get(section, "SaveMemberQuests"));
        r.player_enhance_data = parse_enhance(&ini.get(section, "PlayerEnhanceData"), file)?;
        r.add_skill = parse_add_skill(&ini.get(section, "AddSkill"), file)?;
        r.add_pet = parse_add_pet(&ini.get(section, "AddPet"), file)?;
        r.click_npc_id = num_or(&ini.get(section, "ClickNpcId"), file)?;
        Ok(r)
    }
}

fn parse_tuples(s: &str, file: &str) -> Result<Vec<(i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING {
        return Ok(out);
    }
    for tok in s.split(',') {
        let mut it = tok.trim().split('-');
        let a = it.next().map(str::trim);
        let b = it.next().map(str::trim);
        if let (Some(a), Some(b)) = (a, b) {
            if !a.is_empty() && !b.is_empty() {
                out.push((
                    a.parse::<i64>()
                        .map_err(|_| TsError::Data(format!("bad tuple `{tok}` in {file}")))?,
                    b.parse::<i64>()
                        .map_err(|_| TsError::Data(format!("bad tuple `{tok}` in {file}")))?,
                ));
            }
        }
    }
    Ok(out)
}

fn parse_int_list(s: &str) -> Vec<i64> {
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != NOTHING)
        .filter_map(|t| t.parse::<i64>().ok())
        .collect()
}

/// WarpTo is tab-separated `map x y` (sometimes the `-` tuple form).
fn parse_warp(s: &str, file: &str) -> Result<Vec<i64>> {
    let mut out = Vec::new();
    if s == NOTHING {
        return Ok(out);
    }
    for tok in s.split(|c| c == '\t' || c == ',') {
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
    s.trim().parse::<i64>().map_err(|_| {
        TsError::Data(format!("non-numeric field `{s}` in {file}"))
    })
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

/// AddSkill — `skillId\tlevel` (tabs; comma form also tolerated).
fn parse_add_skill(s: &str, file: &str) -> Result<Vec<(i64, i64)>> {
    let mut out = Vec::new();
    if s == NOTHING {
        return Ok(out);
    }
    for tok in s.split(['\t', ',']) {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        let mut parts = t.split('-');
        let a = parts.next().unwrap_or("").trim();
        let b = parts.next().unwrap_or("1").trim();
        if a.is_empty() {
            continue;
        }
        out.push((
            a.parse::<i64>()
                .map_err(|_| TsError::Data(format!("bad AddSkill `{t}` in {file}")))?,
            b.parse::<i64>()
                .map_err(|_| TsError::Data(format!("bad AddSkill `{t}` in {file}")))?,
        ));
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