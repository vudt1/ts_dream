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
}

#[derive(Debug, Clone, Default)]
pub struct QuestResult {
    pub dialogs: String,
    /// WarpTo — tab-separated `map x y` (Chapter 3 §3.6/§6.7 Warped(...)).
    pub warp_to: Vec<i64>,
    pub rewards: Vec<(i64, i64)>,        // item, count
    pub random_rewards: Vec<(i64, i64)>, // item, count
    pub use_items: Vec<(i64, i64)>,
    pub save_leader_quests: Vec<i64>,
    pub save_member_quests: Vec<i64>,
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

pub fn name_to_string(name: &[u8]) -> String {
    // For dashboard/log display only. Broadly lose the VISCII non-ASCII.
    name.iter()
        .map(|b| encoding::viscii_to_unicode(*b))
        .collect()
}
