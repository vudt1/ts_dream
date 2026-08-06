//! In-memory static data tables (Chapter 3 §3.2/§3.3).

use crate::encoding;

/// An NPC record (`Data_Npcs`), from Npcs.txt.
#[derive(Debug, Clone, Default)]
pub struct Npc {
    pub id: i64,
    pub name: Vec<u8>, // VISCII bytes
    pub lv: i64,
    pub thuoctinh: i64,
    pub hp: i64,
    pub sp: i64,
    pub hpx: i64,
    pub spx: i64,
    pub int1: i64,
    pub atk: i64,
    pub def: i64,
    pub agi: i64,
    pub skill: [i64; 4],
    pub item: [i64; 6],
    pub bat: i64,
    pub reborn: i64,
    /// Bug-for-bug garble override (Chapter 4 §4.3/§4.6) — 23 NPC names.
    pub garble: Option<crate::encoding::GarbleSpec>,
}

impl Npc {
    /// Wire-name hex: `None` aborts the packet (3-digit garble); `Some` is the
    /// exact hex the C# emits (garble override, else clean VISCII bytes).
    pub fn wire_name_hex(&self) -> Option<String> {
        crate::encoding::name_wire_hex(&self.name, &self.garble)
    }
}

/// An item record (`Data_Items`), from Items.txt.
#[derive(Debug, Clone, Default)]
pub struct Item {
    pub id: i64,
    pub name: Vec<u8>, // VISCII bytes
    pub level: i64,
    pub hp: i64,
    pub sp: i64,
    pub int1: i64,
    pub atk1: i64,
    pub def1: i64,
    pub hpx1: i64,
    pub spx1: i64,
    pub agi1: i64,
    pub fai1: i64,
    pub int2: i64,
    pub atk2: i64,
    pub def2: i64,
    pub hpx2: i64,
    pub spx2: i64,
    pub agi2: i64,
    pub fai2: i64,
    pub thuoctinh: i64,
    pub value: i64,
    pub loai: i64,
    pub rb_pet_from: i64,
    pub rb_pet_to: i64,
    pub add_pet: i64,
    /// Bug-for-bug garble override (Chapter 4 §4.3/§4.6) — 99 item names.
    pub garble: Option<crate::encoding::GarbleSpec>,
}

impl Item {
    /// Wire-name hex: `None` aborts the packet (3-digit garble, e.g. item 48101);
    /// `Some` is the exact hex the C# emits (garble override, else clean bytes).
    pub fn wire_name_hex(&self) -> Option<String> {
        crate::encoding::name_wire_hex(&self.name, &self.garble)
    }
}

/// A skill record (`Data_Skills`), from Skills.txt (UTF-8, GUI-only names).
#[derive(Debug, Clone, Default)]
pub struct Skill {
    pub id: i64,
    pub name: String,
    pub sp: i64,
    pub point: i64,
    pub thuoctinh: i64,
    pub id_dk: [i64; 6],
    pub lv_max: i64,
    pub skill_type: i64,
    pub do_manh: i64,
    pub sl_danh: i64,
    pub reborn: i64,
    pub combo: i64,
    pub delay: i64,
    pub troi_buff: i64,
}

/// A warp record (`Data_Warps`), keyed `(map1, warpid)`.
#[derive(Debug, Clone, Default)]
pub struct Warp {
    pub map1: i64,
    pub warpid: i64,
    pub map2: i64,
    pub x: i64,
    pub y: i64,
}

/// A battle gate record (`Data_BattleGates`).
#[derive(Debug, Clone, Default)]
pub struct BattleGate {
    pub mapid1: i64,
    pub warpid: i64,
    pub diahinh: i64,
    pub defenders: [i64; 10],
}

/// A doll record (`Data_Dolls`).
#[derive(Debug, Clone, Default)]
pub struct Doll {
    pub doll_id: i64,
    pub npc_id: i64,
}

/// A quest-ini dialog entry (`Data_Talks`).
#[derive(Debug, Clone, Default)]
pub struct Talk {
    /// Raw `Dialogs=` hex value (prebuilt `F444…` packet hex).
    pub dialogs: String,
    // The extended [OnWin];[OnLose];[TEAMDEF];[REQUIRES] fields are kept in
    // a separate `QuestDef` as data is parsed.
}

/// Win32 INI quest definition (parsed from a `Quests/*.ini` file).
#[derive(Debug, Clone, Default)]
pub struct QuestDef {
    pub map_id: i64,
    pub talk_type: String, // "NPC" | "WARP"
    pub id: i64,
    pub step: i64,
    pub dialogs: String,
    pub teamdef: Vec<i64>, // int[11] {diahinh, n1..n10}
    pub on_win: QuestResult,
    pub on_lose: QuestResult,
    /// `[REQUIRES] SelectMenu` — the menu choice that must be set (0 if absent).
    pub require_select_menu: i64,
    /// `[REQUIRES] Level` — `Some((value, opIndex))`; op 0 `=` 1 `>=` 2 `>`
    /// 3 `<=` 4 `<` 5 `!=`. `None` = key absent = no requirement (C# `int[0]`).
    pub require_level: Option<(i64, i64)>,
    /// `[REQUIRES] Reborn` — `Some((value, opIndex))`, `None` when absent.
    pub require_reborn: Option<(i64, i64)>,
    /// `[REQUIRES] Thuoctinh` — element 1..4 (0 if absent).
    pub require_thuoctinh: i64,
    /// `[REQUIRES] Quests` — `(mapId, npcId, warpId, step)` tuples.
    pub require_quests: Vec<(i64, i64, i64, i64)>,
    /// `[REQUIRES] Wears` — `(itemId, playerOrPet)` tuples.
    pub require_wears: Vec<(i64, i64)>,
    /// `[DESCRIPTION] Title` — server-GUI requirement message only.
    pub desc_title: String,
}

#[derive(Debug, Clone, Default)]
pub struct QuestResult {
    pub dialogs: String,
    /// WarpTo — tab-separated `map x y` (Chapter 3 §3.6/§6.7 Warped(...)).
    pub warp_to: Vec<i64>,
    /// Red banner shown on win (`_Message`).
    pub message: String,
    /// Guaranteed rewards: `(itemId, count, shareToParty)`.
    pub rewards: Vec<(i64, i64, i64)>,
    /// Random rewards: `(itemId, count, shareToParty)` — one fresh-RNG pick.
    pub random_rewards: Vec<(i64, i64, i64)>,
    /// UseItems: `(itemId, target)` target 0 = self, else active pet path.
    pub use_items: Vec<(i64, i64)>,
    /// SaveLeaderQuests: `(npcId, npcVal, warpVal, plus)`.
    pub save_leader_quests: Vec<(i64, i64, i64, i64)>,
    /// SaveMemberQuests: `(npcId, npcVal, warpVal, plus)`.
    pub save_member_quests: Vec<(i64, i64, i64, i64)>,
    /// RequireItems consumed on win: `(itemId, count, remove)`.
    pub require_items: Vec<(i64, i64, i64)>,
    /// PlayerEnhanceData — tab-separated `Stat-Δ` boosts (Point, SkillPoint).
    pub player_enhance_data: Vec<(String, i64)>,
    /// AddSkill — skill id + target level (tabs). Vec for safety.
    pub add_skill: Vec<(i64, i64)>,
    pub add_pet: Vec<i64>,
    pub click_npc_id: i64,
}

/// `Data_Texps` — computed cumulative EXP thresholds (no file). Indexed by
/// level (0..MaxLevel-1); each has reborn 0/1/2 thresholds.
#[derive(Debug, Clone, Default)]
pub struct TexpRow {
    pub lv: i64,
    pub reborn: [i64; 3],
}

/// A map NPC spawn entry (NpcOnMap.txt).
#[derive(Debug, Clone, Default)]
pub struct NpcOnMap {
    pub map_id: i64,
    pub id: i64,
    pub npc_id: i64,
    pub x: i64,
    pub y: i64,
    pub coord: i64,
    pub so_luong: i64,
}

/// A static map drop entry (ItemOnMap.txt).
#[derive(Debug, Clone, Default)]
pub struct ItemOnMap {
    pub map_id: i64,
    pub id: i64,
    pub item_id: i64,
    pub x: i64,
    pub y: i64,
    pub delay: i64,
}

/// A spawned static drop (`Data.ItemDropOnMap`), created by `CreatMapItem`.
///
/// Pre-filled as empty slots 1..255 per map; each ItemOnMap.txt row spawns one
/// with a full copy of the item's stats (C# `SystemDropItem`, Data.cs:5278-5345)
/// and `_Delay = 999999` (never auto-removed). Keyed `(map_id, slot)`.
#[derive(Debug, Clone, Default)]
pub struct ItemDropOnMap {
    pub map_id: i64,
    pub slot: i64,
    pub item_id: i64,
    pub map_x: i64,
    pub map_y: i64,
    pub delay: i64,
    pub count: i64,
    /// The spawned item carries a copy of `Data_Items` stats (C# `_ItemDropOnMap`).
    pub lv: i64,
    pub doben: i64,
    pub int1: i64,
    pub atk1: i64,
    pub def1: i64,
    pub hpx1: i64,
    pub spx1: i64,
    pub agi1: i64,
    pub fai1: i64,
    pub int2: i64,
    pub atk2: i64,
    pub def2: i64,
    pub hpx2: i64,
    pub spx2: i64,
    pub agi2: i64,
    pub fai2: i64,
    pub hp: i64,
    pub sp: i64,
    pub long_val: i64,
    pub giatri_long: i64,
    pub khang: i64,
    pub thuoctinh: i64,
    pub giatri_thuoctinh: i64,
    pub loai: i64,
    pub texp: i64,
    /// C# always sets `_Gold = 3` on spawned drops (Data.cs:5341).
    pub gold: i64,
}

pub fn name_to_string(name: &[u8]) -> String {
    // For dashboard/log display only. Broadly lose the VISCII non-ASCII.
    name.iter()
        .map(|b| encoding::viscii_to_unicode(*b))
        .collect()
}
