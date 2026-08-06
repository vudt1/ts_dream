//! Game server session: owns one socket, frames traffic, dispatches opcodes,
//! and holds per-connection player state.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::protocol::encoder;
use crate::protocol::frame::Decoder;

/// An item in inventory, equipment, storage, etc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryItem {
    pub slot: u8,
    pub id: u16,
    pub count: u8,
    /// Item level requirement (equip gate `player.Lv >= item.Lv`; C# `_Lv`).
    pub lv: u8,
    pub doben: u8,
    pub long_val: u8,
    pub giatri_long: u8,
    pub khang: u8,
    pub texp: u32,
    pub int1: i16,
    pub atk1: i16,
    pub def1: i16,
    pub hpx1: i16,
    pub spx1: i16,
    pub agi1: i16,
    pub fai1: i16,
    pub loai: u8,
    pub thuoctinh: u8,
}

/// One pet entry owned by player.
#[derive(Debug, Clone, Default)]
pub struct PetState {
    pub stt: u8, // 1..4 (active list), 5..8 (stable)
    pub id: u16,
    pub name: Vec<u8>,
    pub level: u8,
    pub thuoctinh: u8,
    pub reborn: u8,
    pub hp: u16,
    pub hp_max: u16,
    pub sp: u16,
    pub sp_max: u16,
    pub int1: u16,
    pub atk: u16,
    pub def: u16,
    pub hpx: u16,
    pub spx: u16,
    pub agi: u16,
    pub fai: u16,
    pub texp: u32,
    pub skill_point: u16,
    pub skills: [(u16, u8); 4],
}

/// An item listed in player shop.
#[derive(Debug, Clone, Default)]
pub struct ShopItem {
    pub slot: u8,
    pub item_id: u16,
    pub count: u8,
    pub price: u32,
}

/// Player shop state.
#[derive(Debug, Clone, Default)]
pub struct PlayerShopState {
    pub active: bool,
    pub name: String,
    pub items: Vec<ShopItem>,
}

/// Per-connection game session state (mirrors C# `Client`).
#[derive(Debug, Clone)]
pub struct Session {
    pub id: u32,
    pub logined: bool,
    pub authed: bool,
    pub idtalking: i32,
    /// Real NPC database id resolved from the on-map index (`idnpctalking`).
    pub idnpctalking: i32,
    pub select_menu: i32,
    pub battle_id: i32,
    pub pending_pass: Vec<u8>,
    pub pending_new_char_name: Vec<u8>,
    pub name: Vec<u8>,
    pub map_id: u16,
    pub map_x: u16,
    pub map_y: u16,
    pub gocnhin: u8,
    pub dongtac: u8,
    pub pk: u8,
    pub tham_chien: u8,

    // Character stats
    pub level: u8,
    pub reborn: u8,
    pub job: u8,
    pub sex: u8,
    pub hair: u16,
    pub thuoctinh: u8,
    pub hp: u16,
    pub hp_max: u16,
    pub sp: u16,
    pub sp_max: u16,
    pub point: u16,
    pub skill_point: u16,
    pub int1: u16,
    pub atk: u16,
    pub def: u16,
    pub hpx: u16,
    pub spx: u16,
    pub agi: u16,

    // Equipment bonus stats
    pub int2: u32,
    pub atk2: u32,
    pub def2: u32,
    pub hpx2: u32,
    pub spx2: u32,
    pub agi2: u32,

    pub texp: u32,
    pub gold: u32,
    pub tiengtam: u16,
    pub god: u32,
    pub hp_store: u32,
    pub sp_store: u32,
    /// Equipped-colour hex string (`_My_Color`), e.g. `"0000000000000000"`.
    pub color: String,

    // Skills: list of (skill_id, level)
    pub skills: Vec<(u16, u8)>,
    // Hotkeys: 1..10
    pub hotkeys: [u16; 11],

    // Inventory tables (by slot)
    pub homdo: Vec<InventoryItem>,
    pub trangbi: Vec<InventoryItem>,
    pub tientrang: Vec<InventoryItem>,
    pub tuideo: Vec<InventoryItem>,
    pub luulang: Vec<InventoryItem>,

    pub pets: Vec<PetState>,
    pub active_pet_stt: u8,

    pub shop: PlayerShopState,
    /// Player whose shop we are viewing (`_Open_Shop_Id`, C# case 32/33).
    pub open_shop_id: u32,

    pub bank_gold: u32,
    pub shop_point: u32,
    pub horse_pet_id: u16,
    pub gift_code_redeemed: bool,
    pub trade: TradeState,
    pub talking_battle: i32,
    pub completed_quests: Vec<i64>,
    /// `[OnWin] ClickNpcId` — NPC that opens a follow-up dialog after quest win.
    pub click_npc_id: i32,
    /// Party members (`_My_IdMem1..4`), 0 = empty.
    pub id_mem: [u32; 4],
    /// Party leader id (`_My_IdLeader`); 0 = no party follow.
    pub id_leader: u32,
    /// Quest step updates recorded by `BattleQuestWin` (`npcId, npcVal`).
    pub quest_steps: Vec<(i64, i64)>,
    /// Warp-step updates recorded by `BattleQuestWin` (`npcId, warpVal`).
    pub warp_steps: Vec<(i64, i64)>,
}

#[derive(Debug, Clone, Default)]
pub struct TradeState {
    pub active: bool,
    pub partner_id: u32,
    pub accepted: bool,
    pub gold: u32,
    pub items: Vec<InventoryItem>,
    pub pets: Vec<u8>,
}

impl Default for Session {
    fn default() -> Self {
        let hp_max = get_hp_max(0, 0, 1, 0) as u16;
        let sp_max = get_sp_max(0, 0, 1, 0) as u16;
        Self {
            id: 0,
            logined: false,
            authed: false,
            idtalking: 0,
            idnpctalking: 0,
            select_menu: 0,
            battle_id: 0,
            pending_pass: Vec::new(),
            pending_new_char_name: Vec::new(),
            name: Vec::new(),
            map_id: 12001,
            map_x: 400,
            map_y: 500,
            gocnhin: 0,
            dongtac: 0,
            pk: 0,
            tham_chien: 0,

            level: 1,
            reborn: 0,
            job: 0,
            sex: 0,
            hair: 0,
            thuoctinh: 1,
            hp: hp_max,
            hp_max,
            sp: sp_max,
            sp_max,
            point: 0,
            skill_point: 0,
            int1: 0,
            atk: 0,
            def: 0,
            hpx: 0,
            spx: 0,
            agi: 0,

            int2: 0,
            atk2: 0,
            def2: 0,
            hpx2: 0,
            spx2: 0,
            agi2: 0,

            texp: 6,
            gold: 0,
            tiengtam: 1,
            god: 0,
            hp_store: 10000,
            sp_store: 10000,
            color: "0000000000000000".to_string(),

            skills: Vec::new(),
            hotkeys: [0; 11],

            homdo: Vec::new(),
            trangbi: Vec::new(),
            tientrang: Vec::new(),
            tuideo: Vec::new(),
            luulang: Vec::new(),

            pets: Vec::new(),
            active_pet_stt: 0,

            shop: PlayerShopState::default(),
            open_shop_id: 0,

            bank_gold: 0,
            shop_point: 1000,
            horse_pet_id: 0,
            gift_code_redeemed: false,
            trade: TradeState::default(),
            talking_battle: 0,
            completed_quests: Vec::new(),
            click_npc_id: 0,
            id_mem: [0; 4],
            id_leader: 0,
            quest_steps: Vec::new(),
            warp_steps: Vec::new(),
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recompute gear bonus stats and max HP/SP.
    pub fn recompute_stats(&mut self) {
        let sheet = crate::server::character_sheet::CharacterSheet::recompute(
            i64::from(self.reborn),
            i64::from(self.job),
            i64::from(self.level),
            i64::from(self.hpx),
            i64::from(self.spx),
            &self.trangbi,
        );
        self.int2 = sheet.gear.int2;
        self.atk2 = sheet.gear.atk2;
        self.def2 = sheet.gear.def2;
        self.hpx2 = sheet.gear.hpx2;
        self.spx2 = sheet.gear.spx2;
        self.agi2 = sheet.gear.agi2;
        self.hp_max = sheet.hp_max;
        self.sp_max = sheet.sp_max;
        if self.hp > self.hp_max {
            self.hp = self.hp_max;
        }
        if self.sp > self.sp_max {
            self.sp = self.sp_max;
        }
    }

    /// Dump Homdo inventory frame (`F444` + len + `1705` + entries).
    pub fn dump_homdo(&self) -> String {
        self.dump_table(&self.homdo, "1705")
    }

    /// Dump Trangbi equipment frame (`F444` + len + `170B` + entries).
    pub fn dump_trangbi(&self) -> String {
        self.dump_table(&self.trangbi, "170B")
    }

    /// Dump TienTrang storage frame (`F444` + len + `1E01` + entries).
    pub fn dump_tientrang(&self) -> String {
        self.dump_table(&self.tientrang, "1E01")
    }

    /// Dump Tuideo pouch frame (`F444` + len + `172F` + entries).
    pub fn dump_tuideo(&self) -> String {
        self.dump_table(&self.tuideo, "172F")
    }

    /// Dump LuuLang storage frame (`F444` + len + `1766` + entries).
    pub fn dump_luulang(&self) -> String {
        self.dump_table(&self.luulang, "1766")
    }

    /// Serialise one inventory table into a frame (`F444` + len + `code` + entries).
    fn dump_table(&self, items: &[InventoryItem], code: &str) -> String {
        let mut entries = String::new();
        for item in items {
            if item.id > 0 {
                entries.push_str(&format!(
                    "{:02X}{}{:02X}{:02X}{:02X}{:02X}{:02X}{}",
                    item.slot,
                    encoder::le16(item.id),
                    item.count,
                    item.doben,
                    item.long_val,
                    (item.giatri_long as u16 + 100) as u8,
                    item.khang,
                    encoder::le32(item.texp)
                ));
            }
        }
        crate::protocol::frame(code, &entries)
    }

    /// Equipped item ids by slot (used in the self-appear frame, `_My_Color` side).
    pub fn equipped_ids(&self) -> Vec<u16> {
        self.trangbi
            .iter()
            .filter(|i| i.id > 0)
            .map(|i| i.id)
            .collect()
    }

    /// Dump Hotkeys skill bar frame (`F444` + len + `2801` + entries).
    pub fn dump_hotkeys(&self) -> String {
        let mut entries = String::new();
        for slot in 1..=10 {
            let skill_id = self.hotkeys.get(slot as usize).copied().unwrap_or(0);
            if skill_id > 0 {
                entries.push_str(&format!("02{}{:02X}", encoder::le16(skill_id), slot));
            }
        }
        if entries.is_empty() {
            "F4440300280102".to_string()
        } else {
            crate::protocol::frame("2801", &entries)
        }
    }

    /// Add an item to Homdo. Returns slot index (1..25) on success, None if full.
    pub fn add_homdo_item(&mut self, item: InventoryItem) -> Option<u8> {
        crate::server::inventory::add_item(&mut self.homdo, item)
    }

    /// Remove up to `count` of `item_id` from inventory; returns the removed count.
    pub fn remove_homdo_item(&mut self, item_id: u16, count: u32) -> u32 {
        crate::server::inventory::remove_item(&mut self.homdo, item_id, count)
    }
}

/// Owns the incoming decode buffer for a connection.
pub struct Conn {
    pub decoder: Decoder,
    pub session: Session,
}

/// Online session registry (C# `Server.Clients`): authoritative per-player
/// session snapshots, synced by the connection loop on every frame. Cross-player
/// flows (player shop op 0x17 sub 32/33) read and mutate it.
pub fn online_sessions() -> &'static std::sync::Mutex<std::collections::HashMap<u32, Session>> {
    static ONLINE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, Session>>> =
        std::sync::OnceLock::new();
    ONLINE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

impl Conn {
    pub fn new() -> Self {
        Self {
            decoder: Decoder::new(),
            session: Session::new(),
        }
    }
}
