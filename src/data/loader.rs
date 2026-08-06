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
#[derive(Debug, Default, Clone)]
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
    /// Spawned static drops (C# `Data.ItemDropOnMap`), keyed by
    /// `(map_id, slot)`. Pre-filled empty slots 1..255 per map, then each
    /// ItemOnMap.txt row spawns a `_Delay=999999` static drop.
    pub item_drop_on_map: HashMap<(i64, i64), ItemDropOnMap>,
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

/// Strict column read (spec §3.1 "no defaults"): a missing or empty numeric
/// column is a load failure, exactly like the C# `Conversions.ToInteger`
/// throwing `IndexOutOfRangeException`/`FormatException`.
fn num_at(idx: usize, f: &[&str], file: &str) -> Result<i64> {
    let field = f
        .get(idx)
        .ok_or_else(|| TsError::Data(format!("missing column {idx} in {file}")))?;
    num(field, file)
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
    /// Column map (Data.cs:4060-4083): 0-11 id..agi, 12-15 Skill1-4, 16-21
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
    /// column (`array2[2].Length <= 0`, Data.cs:4514).
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
            for i in 0..10 {
                defenders[i] = num_at(3 + i, &f, "BattleGate.txt")?;
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
    /// `_Delay=999999` (C# `CreatMapItem` Data.cs:5347-5412 + `SystemDropItem`
    /// 5278-5345). The C# broadcast `F44408001703` fires with no clients at
    /// load time (no-op); `static_drop_frame` exposes the same frame for maps
    /// with live clients.
    fn load_item_on_map(&mut self, path: &Path) -> Result<()> {
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
            // Duplicate `(mapId, itemId, x, y)` rows are skipped (C# guard
            // `if (!ItemOnMap.ContainsKey(key))`, Data.cs:5390) — no re-spawn.
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
            // Spawn the static drop (C# `SystemDropItem(mapid, slot, x, y,
            // itemId, 999999)`): copies the item's full stats, `_Delay=999999`,
            // `_Gold=3` (Data.cs:5278-5345).
            let item = self
                .items
                .get(&item_id)
                .ok_or_else(|| TsError::Data(format!("ItemOnMap references unknown item {item_id}")))?;
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

    /// Broadcast frame for a spawned static drop (C# `SystemDropItem`):
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
            // C# `genTalkInfoTeamDefDiahinh`: absent -> 0.
            let diahinh = ini.get("TEAMDEF", "Diahinh");
            v.push(if diahinh == NOTHING || diahinh.trim().is_empty() {
                0
            } else {
                num(&diahinh, &file)?
            });
            // C# `genTalkInfoTeamDefNpcs(text, '\t')`: absent or not exactly 10
            // elements -> int[10] zeros; else the 10 parsed ids.
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

        // [REQUIRES] — entry conditions (C# Data.cs:4612-4619).
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
            q.on_win.require_items = parse_tuples(&ini.get("REQUIRES", "Items"), &file)?;
        }

        q.on_win = self.parse_result(&ini, "OnWin", &file)?;
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
        // [OnLose].WarpTo is read from ONWIN (C# copy-paste bug, Data.cs:4649):
        // `_LoseWarpTo` always equals `_WinWarpTo`.
        let mut on_lose = self.parse_result(&ini, "OnLose", &file)?;
        on_lose.warp_to = q.on_win.warp_to.clone();
        q.on_lose = on_lose;
        // [DESCRIPTION] Title — server-GUI requirement messages (Data.cs:4650).
        let title = ini.get("DESCRIPTION", "Title");
        if title != NOTHING {
            q.desc_title = title;
        }
        Ok(q)
    }

    fn parse_result(&self, ini: &Ini, section: &str, file: &str) -> Result<QuestResult> {
        let mut r = QuestResult::default();
        r.dialogs = ini.get(section, "Dialogs");
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
        let b = parts.get(1).copied().unwrap_or("0")
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

/// `[REQUIRES] Level/Reborn` — `value\top`; operator index per C#
/// `genTalkInfoCondition` (Data.cs:6054-6067): `["=",">=",">","<=","<","!="]`
/// → 0..5. Absent key → `None` (C# returns the empty `int[0]` = no condition,
/// NOT a `= 0` requirement).
fn parse_condition(s: &str, file: &str) -> Result<Option<(i64, i64)>> {
    if s == NOTHING || s.trim().is_empty() {
        return Ok(None);
    }
    let mut it = s.split('\t');
    let value = it.next().unwrap_or("").trim().parse::<i64>().map_err(|_| {
        TsError::Data(format!("bad condition `{s}` in {file}"))
    })?;
    let op = it.next().unwrap_or("").trim();
    let ops = ["=", ">=", ">", "<=", "<", "!="];
    let op_index = ops.iter().position(|&o| o == op).map(|i| i as i64).unwrap_or(-1);
    Ok(Some((value, op_index)))
}

/// `[REQUIRES] Quests` — tab-separated `mapId-npcId-warpId-step` tuples
/// (C# `genTalkInfoListInt` with intSplit `-`).
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
        let b = parts.get(1).copied().unwrap_or("0")
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
        let target = parts.get(1).copied().unwrap_or("0")
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

/// AddSkill — `skillId\tlevel` (C# `_WinAddSkill =
/// Array.ConvertAll(genTalkInfoDialog(..., '\t'), int.Parse)` — a flat int
/// array; the first two elements are the skill id and its level).
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal but structurally valid `.txt` dataset for loader unit tests.
    /// 25-col Items row, 24-col Npcs row, 19-col Skills row.
    fn write_dataset(dir: &std::path::Path) {
        std::fs::create_dir_all(dir.join("Quests")).unwrap();
        std::fs::write(dir.join("Items.txt"), b"//Id\tName\t...\n1\tA\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\n").unwrap();
        std::fs::write(dir.join("Skills.txt"), b"//Id\tName\t...\n1\tA\t1\t1\t1\t0\t0\t0\t0\t0\t0\t1\t1\t1\t1\t0\t0\t0\t0\n").unwrap();
        std::fs::write(dir.join("BattleGate.txt"), b"//Mapid1\tWarpId\tDiahinh\n1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\n").unwrap();
        std::fs::write(dir.join("Dolls.txt"), b"//DollId\tNpcId\n1\t2\n").unwrap();
        std::fs::write(dir.join("NpcOnMap.txt"), b"//MapId\tId\tNpcId\tX\tY\tCoord\tSoLuong\n1\t1\t2\t3\t4\t5\t0\n").unwrap();
        std::fs::write(dir.join("ItemOnMap.txt"), b"//MapId\tId\tItemId\tX\tY\tDelay\n").unwrap();
        // Npcs.txt is UTF-16LE with BOM (24-col row: id..agi, skills, drops, NotPet, Reborn).
        let npc = "1\tA\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0";
        let mut bytes = vec![0xFF, 0xFE];
        for u in npc.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        std::fs::write(dir.join("Npcs.txt"), bytes).unwrap();
    }

    #[test]
    fn warps_skip_empty_destination_column() {
        let dir = tempfile::tempdir().unwrap();
        write_dataset(dir.path());
        std::fs::write(
            dir.path().join("Warps.txt"),
            b"//map1\twarpid\tmap2\tx\ty\n1\t2\t\t3\t4\n5\t6\t7\t8\t9\n",
        )
        .unwrap();
        let d = GameData::load(dir.path()).expect("load");
        // The row with an empty map2 column is silently dropped (C# Data.cs:4514).
        assert_eq!(d.warps.len(), 1);
        assert!(d.warps.contains_key(&(5, 6)));
        assert!(!d.warps.contains_key(&(1, 2)));
    }

    #[test]
    fn npcs_missing_reborn_column_is_load_failure() {
        let dir = tempfile::tempdir().unwrap();
        write_dataset(dir.path());
        // 23-col row (no Reborn col 23) — C# `Conversions.ToInteger(array2[23])`
        // throws IndexOutOfRangeException → load failure, not a default.
        let npc = "2\tB\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0";
        let mut bytes = vec![0xFF, 0xFE];
        for u in npc.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        std::fs::write(dir.path().join("Npcs.txt"), bytes).unwrap();
        assert!(GameData::load(dir.path()).is_err());
    }

    #[test]
    fn item_drop_prefill_does_not_repeat_per_map() {
        let mut d = GameData::default();
        d.items.insert(
            31099,
            Item {
                id: 31099,
                level: 1,
                ..Default::default()
            },
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ItemOnMap.txt");
        std::fs::write(
            &path,
            b"//MapId\tId\tItemId\tX\tY\tDelay\n10965\t1\t31099\t2228\t126\t1\n10965\t2\t31099\t10\t20\t1\n",
        )
        .unwrap();
        d.load_item_on_map(&path).expect("load item on map");
        // Pre-fill 255 slots happens once per map; both spawns land.
        assert_eq!(d.item_drop_on_map.len(), 255);
        assert_eq!(d.item_drop_on_map[&(10965, 1)].item_id, 31099);
        assert_eq!(d.item_drop_on_map[&(10965, 1)].delay, 999_999);
        assert_eq!(d.item_drop_on_map[&(10965, 2)].map_x, 10);
        assert_eq!(d.item_drop_on_map[&(10965, 255)].item_id, 0);
    }
}
