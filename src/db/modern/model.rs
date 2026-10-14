//! Domain models for the modern 3NF persistence layer (ticket 06).
//!
//! These types are decoupled from the legacy session snapshots: an
//! [`InventorySlot`] carries the standardized 35-byte
//! [`ThingData`](crate::protocol::codecs::thing_data::ThingData) value object,
//! so what the repository writes is byte-for-byte what the wire codec emits.

use crate::protocol::codecs::thing_data::ThingData;

/// Item storage container selector (`inventories.storage_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageType {
    Bag = 1,
    Bank = 2,
    Secondary = 4,
    Equip = 8,
    Warehouse = 16,
}

/// Format record inventory trên wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryLayout {
    /// 12 byte, byte slot đầu — `17 05`, `1E 01`, `17 2F`, `17 66`.
    SlotLed12,
    /// 10 byte, **không** byte slot — `17 0B` (client `IDIV 0x0A`).
    Slotless10,
}

impl StorageType {
    /// Numeric value stored in `inventories.storage_type`.
    #[inline]
    pub const fn value(self) -> u8 {
        self as u8
    }

    /// Tên Rust của container (khớp key trong `persist::storage_type`).
    pub const fn table_name(self) -> &'static str {
        match self {
            Self::Bag => "homdo",
            Self::Bank => "tientrang",
            Self::Secondary => "tuideo",
            Self::Equip => "trangbi",
            Self::Warehouse => "luulang",
        }
    }

    /// Frame dump S->C (hex opcode 2 byte) của container này.
    pub const fn dump_code(self) -> &'static str {
        match self {
            Self::Bag => crate::protocol::OPCODE_HOMDO,
            Self::Bank => crate::protocol::OPCODE_TIENTRANG,
            Self::Secondary => crate::protocol::OPCODE_TUIDEO,
            Self::Equip => crate::protocol::OPCODE_TRANGBI,
            Self::Warehouse => crate::protocol::OPCODE_LUULANG,
        }
    }

    /// Sức chứa thực tế theo guard của client parser:
    /// - Bag (HomDo): 25 ô (1..=25)
    /// - Bank (TienTrang): 50 ô (1..=50)
    /// - Secondary (TuiDeo): 25 ô (1..=25)
    /// - Equip (TrangBi): 6 ô (1..=6)
    /// - Warehouse (LuuLang / form_Horse): 10 ô (1..=10)
    pub const fn capacity(self) -> u16 {
        match self {
            Self::Bag | Self::Secondary => 25,
            Self::Bank => 50,
            Self::Warehouse => 10,
            Self::Equip => 6,
        }
    }

    /// Format record trên wire.
    pub const fn entry_layout(self) -> EntryLayout {
        match self {
            Self::Equip => EntryLayout::Slotless10,
            _ => EntryLayout::SlotLed12,
        }
    }

    /// Parses the raw column value. Legacy clients may send any of the five
    /// container ids; anything else is rejected rather than coerced.
    pub fn from_value(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Bag),
            2 => Some(Self::Bank),
            4 => Some(Self::Secondary),
            8 => Some(Self::Equip),
            16 => Some(Self::Warehouse),
            _ => None,
        }
    }

    /// Parse từ tên bảng (`persist::storage_type`).
    pub fn from_table_name(name: &str) -> Option<Self> {
        match name {
            "homdo" => Some(Self::Bag),
            "tientrang" => Some(Self::Bank),
            "tuideo" => Some(Self::Secondary),
            "trangbi" => Some(Self::Equip),
            "luulang" => Some(Self::Warehouse),
            _ => None,
        }
    }
}

/// General (vo tuong) storage selector (`character_pets.storage_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetStorageType {
    Carried = 1,
    Cart = 2,
    Hotel = 3,
    Warehouse = 4,
}

impl PetStorageType {
    #[inline]
    pub fn value(self) -> u8 {
        self as u8
    }

    pub fn from_value(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Carried),
            2 => Some(Self::Cart),
            3 => Some(Self::Hotel),
            4 => Some(Self::Warehouse),
            _ => None,
        }
    }
}

/// One occupied (or explicitly empty) cell of any item container.
#[derive(Debug, Clone, PartialEq)]
pub struct InventorySlot {
    pub storage_type: StorageType,
    pub slot: u16,
    pub item: ThingData,
}

impl InventorySlot {
    /// An empty-slot row: `item_id = 0` marks vacancy, mirroring the wire.
    pub fn empty(storage_type: StorageType, slot: u16) -> Self {
        Self {
            storage_type,
            slot,
            item: ThingData::empty(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.item.is_empty()
    }
}

/// Currency ledger row (`character_money`, 1:1 with a character).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Money {
    pub gold: i64,
    pub bank_gold: i64,
    pub shop_point: i64,
}

/// One learned skill row (`character_skills`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillRow {
    pub skill_id: i64,
    pub level: i64,
    pub sp: i64,
    pub save_flag: i64,
}

/// Mission progress row (`character_missions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionRow {
    pub mission_id: i64,
    pub step: i64,
    pub state: i64,
    pub updated_at: i64,
}

/// One stored general row (`character_pets`), skills included.
#[derive(Debug, Clone, PartialEq)]
pub struct PetRecord {
    pub slot: u16,
    pub pet_id: u16,
    pub name: Vec<u8>,
    pub level: i64,
    pub element: i64,
    pub reborn: i64,
    pub hp: i64,
    pub hp_max: i64,
    pub sp: i64,
    pub sp_max: i64,
    pub int_attr: i64,
    pub atk: i64,
    pub def: i64,
    pub hpx: i64,
    pub spx: i64,
    pub agi: i64,
    pub fai: i64,
    pub texp: i64,
    pub skill_point: i64,
    pub thd: i64,
    pub skills: [PetSkill; 4],
    pub quest: i64,
    pub is_active: bool,
}

/// One of the four general skill slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PetSkill {
    pub id: i64,
    pub level: i64,
}
