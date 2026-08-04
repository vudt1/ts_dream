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
    pub hp_store: u32,
    pub sp_store: u32,

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
            hp_store: 10000,
            sp_store: 10000,

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
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recompute gear bonus stats and max HP/SP.
    pub fn recompute_stats(&mut self) {
        let mut int2 = 0u32;
        let mut atk2 = 0u32;
        let mut def2 = 0u32;
        let mut hpx2 = 0u32;
        let mut spx2 = 0u32;
        let mut agi2 = 0u32;

        for item in &self.trangbi {
            if item.id > 0 {
                int2 += item.int1.max(0) as u32;
                atk2 += item.atk1.max(0) as u32;
                def2 += item.def1.max(0) as u32;
                hpx2 += item.hpx1.max(0) as u32;
                spx2 += item.spx1.max(0) as u32;
                agi2 += item.agi1.max(0) as u32;
            }
        }

        self.int2 = int2;
        self.atk2 = atk2;
        self.def2 = def2;
        self.hpx2 = hpx2;
        self.spx2 = spx2;
        self.agi2 = agi2;

        let computed_hp = get_hp_max(self.reborn as i64, self.job as i64, self.level as i64, self.hpx as i64) as u16 + hpx2 as u16;
        let computed_sp = get_sp_max(self.reborn as i64, self.job as i64, self.level as i64, self.spx as i64) as u16 + spx2 as u16;

        self.hp_max = computed_hp;
        self.sp_max = computed_sp;

        if self.hp > self.hp_max {
            self.hp = self.hp_max;
        }
        if self.sp > self.sp_max {
            self.sp = self.sp_max;
        }
    }

    /// Dump Homdo inventory frame (`F444` + len + `1705` + entries).
    pub fn dump_homdo(&self) -> String {
        let mut entries = String::new();
        for item in &self.homdo {
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
        let total_len = 2 + entries.len() / 2;
        format!("F444{}1705{}", encoder::le16(total_len as u16), entries)
    }

    /// Dump Trangbi equipment frame (`F444` + len + `170B` + entries).
    pub fn dump_trangbi(&self) -> String {
        let mut entries = String::new();
        for item in &self.trangbi {
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
        let total_len = 2 + entries.len() / 2;
        format!("F444{}170B{}", encoder::le16(total_len as u16), entries)
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
            let payload = format!("2801{entries}");
            let total_len = 2 + payload.len() / 2;
            format!("F444{}{}", encoder::le16(total_len as u16), payload)
        }
    }

    /// Add an item to Homdo. Returns slot index (1..25) on success, None if full.
    pub fn add_homdo_item(&mut self, mut item: InventoryItem) -> Option<u8> {
        // If stackable, find existing slot
        if item.count > 0 {
            for existing in &mut self.homdo {
                if existing.id == item.id && existing.count < 255 {
                    existing.count = existing.count.saturating_add(item.count);
                    return Some(existing.slot);
                }
            }
        }

        // Find free slot
        for slot in 1..=25 {
            if !self.homdo.iter().any(|i| i.slot == slot && i.id > 0) {
                item.slot = slot;
                self.homdo.push(item);
                return Some(slot);
            }
        }
        None
    }
}

/// Owns the incoming decode buffer for a connection.
pub struct Conn {
    pub decoder: Decoder,
    pub session: Session,
}

impl Conn {
    pub fn new() -> Self {
        Self {
            decoder: Decoder::new(),
            session: Session::new(),
        }
    }
}