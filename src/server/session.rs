//! Game server session: owns one socket, frames traffic, dispatches opcodes,
//! and holds per-connection player state.

use crate::battle::engine::{get_hp_max, get_sp_max};
use crate::db::modern::model::{EntryLayout, StorageType};
use crate::protocol::encoder;
use crate::protocol::frame::Decoder;

/// An item in inventory, equipment, storage, etc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryItem {
    pub slot: u8,
    pub id: u16,
    pub count: u8,
    /// Item level requirement (equip gate `player.Lv >= item.Lv`).
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
    /// Elemental `_2` stat fields (Items.txt cols 12..19).
    pub int2: i16,
    pub atk2: i16,
    pub def2: i16,
    pub hpx2: i16,
    pub spx2: i16,
    pub agi2: i16,
    pub fai2: i16,
    pub item_hp: i16,
    pub item_sp: i16,
    pub loai: u8,
    /// Item element (`_Thuoctinh`, Items.txt `element`).
    pub thuoctinh: u8,
    /// Elemental stat bonus (`_GiatriThuoctinh`, Items.txt `elem_val`).
    pub giatri_thuoctinh: u8,
}

impl InventoryItem {
    /// Materialise an inventory instance from the static item template
    /// (`Data.Item`, Items.txt), copying the base `_1`/`_2` stats, the element,
    /// and the elemental bonus from the static table. Slot and per-instance
    /// dynamic fields (doben/long/khang/texp) are left defaulted.
    pub fn from_template(def: &crate::data::tables::Item, count: u8) -> Self {
        InventoryItem {
            id: def.id as u16,
            count,
            lv: def.level as u8,
            int1: def.int1 as i16,
            atk1: def.atk1 as i16,
            def1: def.def1 as i16,
            hpx1: def.hpx1 as i16,
            spx1: def.spx1 as i16,
            agi1: def.agi1 as i16,
            fai1: def.fai1 as i16,
            int2: def.int2 as i16,
            atk2: def.atk2 as i16,
            def2: def.def2 as i16,
            hpx2: def.hpx2 as i16,
            spx2: def.spx2 as i16,
            agi2: def.agi2 as i16,
            fai2: def.fai2 as i16,
            item_hp: def.hp as i16,
            item_sp: def.sp as i16,
            loai: def.loai as u8,
            thuoctinh: def.thuoctinh as u8,
            giatri_thuoctinh: def.value as u8,
            ..Default::default()
        }
    }
}

/// One hotkey bar slot (1..=10).
/// `kind`: 0 = empty, 1 = item quickslot, 2 = skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HotkeySlot {
    pub kind: u8,
    pub id: u16,
}

impl HotkeySlot {
    pub const fn empty() -> Self {
        Self { kind: 0, id: 0 }
    }
    pub const fn item(id: u16) -> Self {
        Self { kind: 1, id }
    }
    pub const fn skill(id: u16) -> Self {
        Self { kind: 2, id }
    }
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
    pub int2: u16,
    pub atk2: u16,
    pub def2: u16,
    pub hpx2: u16,
    pub spx2: u16,
    pub agi2: u16,
    pub thd: u16,
    pub texp: u32,
    pub skill_point: u16,
    pub quest: u8,
    pub skills: [(u16, u8); 4],
}

impl PetState {
    /// Recompute bonus stats (`int2..agi2`, `hp_max`, `sp_max`) from equipped gear.
    pub fn recompute_stats(&mut self, trangbi: &[InventoryItem]) {
        let sheet = crate::server::character_sheet::PetSheet::recompute(
            self.reborn,
            self.level,
            self.hpx,
            self.spx,
            self.hp_max,
            self.sp_max,
            self.stt,
            self.thuoctinh,
            trangbi,
        );
        self.hp_max = sheet.hp_max;
        self.sp_max = sheet.sp_max;
        self.int2 = sheet.gear.int2.min(u16::MAX as u32) as u16;
        self.atk2 = sheet.gear.atk2.min(u16::MAX as u32) as u16;
        self.def2 = sheet.gear.def2.min(u16::MAX as u32) as u16;
        self.hpx2 = sheet.gear.hpx2.min(u16::MAX as u32) as u16;
        self.spx2 = sheet.gear.spx2.min(u16::MAX as u32) as u16;
        self.agi2 = sheet.gear.agi2.min(u16::MAX as u32) as u16;

        if self.hp > self.hp_max {
            self.hp = self.hp_max;
        }
        if self.sp > self.sp_max {
            self.sp = self.sp_max;
        }
    }
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
    pub image: u8,
    /// Raw VISCII bytes used by the wire protocol.
    pub name: Vec<u8>,
    pub items: Vec<ShopItem>,
}

/// Per-connection game session state.
#[derive(Debug, Clone)]
pub struct Session {
    /// Network/account identifier from the PC protocol.
    pub id: u32,
    /// Mobile-aligned account display/login name. The PC wire still uses `id`.
    pub account_name: Vec<u8>,
    /// Server-authoritative GM level; zero is a normal player.
    pub gm_level: i32,
    /// Modern `characters.id`; distinct from `id` after the 0002 cutover.
    pub db_character_id: i64,
    pub logined: bool,
    pub in_world: bool,
    pub authed: bool,
    pub idtalking: i32,
    /// Real NPC database id resolved from the on-map index (`idnpctalking`).
    pub idnpctalking: i32,
    /// Active Eve engine event session (ticket 04/05).
    pub current_event_session: Option<crate::eve::auto_chain::EventSession>,
    pub select_menu: i32,
    /// Talk counter (`talkcount`); reset by EndTalk / teleport confirm.
    pub talk_count: i32,
    /// Teleport-confirm state (`warpfinish`); reset by the 0x0C confirmation.
    pub warp_finish: bool,
    /// Talk kind: `"NPC"` or `"WARP"` (`Typetalk`).
    pub talk_type: String,
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
    /// Counter field (`newbie`), incremented by item 46238.
    pub newbie: u32,
    /// Respawn-map flag (`savemap`), set by inn-keepers (H6 SM33).
    pub savemap: u16,
    /// Equipped-colour hex string (`_My_Color`), e.g. `"0000000000000000"`.
    pub color: String,

    // Skills: list of (skill_id, level)
    pub skills: Vec<(u16, u8)>,
    // Hotkeys: 1..10
    pub hotkeys: [HotkeySlot; 11],

    // Inventory tables (by slot)
    pub homdo: Vec<InventoryItem>,
    pub trangbi: Vec<InventoryItem>,
    pub tientrang: Vec<InventoryItem>,
    pub tuideo: Vec<InventoryItem>,
    pub luulang: Vec<InventoryItem>,

    // Quest state (Opcode 0x18 sync — Checkpoint 5).
    /// Active quest log rows: `quest_id -> (slot, mark_step)`. The slot indexes
    /// the client's shared quest-entry array (`LocalActor + 0x654 + slot * 3`,
    /// 1..=200) and is allocated by `handlers::quest_sync`.
    pub quest_tasks: std::collections::HashMap<u16, (u8, u8)>,
    /// Quest "dont" marks (non-repeatable quests): mark ids 1..=300. The wire
    /// carries `(mark, flag)`; a mark present here is flag `1`, absent = `0`.
    pub quest_dont: std::collections::HashSet<u16>,
    /// Quest-only item bag (`0x18 Sub 0x01..0x04`), separate from `homdo` and
    /// stacked per quest rules (200 rows, stack <= 255 — not the homdo 25/50).
    pub quest_items: Vec<InventoryItem>,
    /// Per-event completion counters (`eveid -> count`), incremented by
    /// `finish_event_session` when the session produced state-changing results.
    pub completed_eve_counts: std::collections::HashMap<i32, i32>,

    pub pets: Vec<PetState>,
    pub active_pet_stt: u8,

    pub shop: PlayerShopState,
    /// Player whose shop we are viewing (`_Open_Shop_Id`, ops 0x17 sub 32/33).
    pub open_shop_id: u32,

    pub bank_gold: u32,
    pub shop_point: u32,
    pub horse_pet_id: u16,
    pub trade: TradeState,
    pub talking_battle: i32,
    /// Completed `(map, object/warp)` quest keys — the `[REQUIRES] Quests` gate
    /// source, populated on battle win (`quest::mark_quest_done`).
    pub completed_quests: Vec<(i64, i64)>,
    /// `[OnWin] ClickNpcId` — NPC that opens a follow-up dialog after quest win.
    pub click_npc_id: i32,
    /// Party members (`_My_IdMem1..4`), 0 = empty.
    pub id_mem: [u32; 4],
    /// Party leader id (`_My_IdLeader`); 0 = no party follow.
    pub id_leader: u32,
    /// Quan-su (quartermaster) member id (`_My_IdQS`), set by op 0x0D sub 5
    /// and cleared by sub 6. Drives the per-turn leader SP regen block.
    pub id_qs: u32,
    /// Quest step updates recorded by `BattleQuestWin` (`npcId, npcVal`).
    pub quest_steps: Vec<(i64, i64)>,
    /// Warp-step updates recorded by `BattleQuestWin` (`npcId, warpVal`).
    pub warp_steps: Vec<(i64, i64)>,
    /// Accumulated steps taken since last random encounter roll.
    pub encounter_steps: u32,
    /// Randomized step threshold for triggering wild encounter (default 20).
    pub encounter_threshold: u32,
    /// Timestamp (ms) when last battle ended, used for encounter cooldown.
    pub last_battle_end_ms: u64,
    /// Timestamp (ms) of the last accepted chat message, used for the
    /// 5-second anti-spam cooldown (`CHAT_COOLDOWN_MS` in `handlers::chat`).
    /// Runtime-only; never persisted.
    pub last_chat_ms: u64,
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
            account_name: Vec::new(),
            gm_level: 0,
            db_character_id: 0,
            logined: false,
            in_world: false,
            authed: false,
            idtalking: 0,
            idnpctalking: 0,
            current_event_session: None,
            select_menu: 0,
            talk_count: 0,
            warp_finish: false,
            talk_type: "NPC".to_string(),
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
            newbie: 0,
            savemap: 0,
            color: "0000000000000000".to_string(),

            skills: Vec::new(),
            hotkeys: [HotkeySlot::empty(); 11],

            homdo: Vec::new(),
            trangbi: Vec::new(),
            tientrang: Vec::new(),
            tuideo: Vec::new(),
            luulang: Vec::new(),

            quest_tasks: std::collections::HashMap::new(),
            quest_dont: std::collections::HashSet::new(),
            quest_items: Vec::new(),
            completed_eve_counts: std::collections::HashMap::new(),

            pets: Vec::new(),
            active_pet_stt: 0,

            shop: PlayerShopState::default(),
            open_shop_id: 0,

            bank_gold: 0,
            shop_point: 1000,
            horse_pet_id: 0,
            trade: TradeState::default(),
            talking_battle: 0,
            completed_quests: Vec::new(),
            click_npc_id: 0,
            id_mem: [0; 4],
            id_leader: 0,
            id_qs: 0,
            quest_steps: Vec::new(),
            warp_steps: Vec::new(),
            encounter_steps: 0,
            encounter_threshold: 20,
            last_battle_end_ms: 0,
            last_chat_ms: 0,
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
            self.thuoctinh,
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

        self.recompute_pet_stats();
    }

    /// Recompute bonus stats for all active carried pets (stt 1..=4).
    pub fn recompute_pet_stats(&mut self) {
        for pet in self.pets.iter_mut() {
            if (1..=4).contains(&pet.stt) && pet.id > 0 {
                pet.recompute_stats(&self.trangbi);
            }
        }
    }

    /// 5 container theo thứ tự `StorageType` — nguồn duy nhất cho load / save / seed,
    /// thay cho các tuple literal rời rạc.
    pub fn containers(&self) -> [(StorageType, &[InventoryItem]); 5] {
        [
            (StorageType::Bag, &self.homdo),
            (StorageType::Bank, &self.tientrang),
            (StorageType::Secondary, &self.tuideo),
            (StorageType::Equip, &self.trangbi),
            (StorageType::Warehouse, &self.luulang),
        ]
    }

    pub fn containers_mut(&mut self) -> [(StorageType, &mut Vec<InventoryItem>); 5] {
        [
            (StorageType::Bag, &mut self.homdo),
            (StorageType::Bank, &mut self.tientrang),
            (StorageType::Secondary, &mut self.tuideo),
            (StorageType::Equip, &mut self.trangbi),
            (StorageType::Warehouse, &mut self.luulang),
        ]
    }

    /// Hydrate toàn bộ template field (`lv`, `int1..fai2`, `loai`, `thuoctinh`, …)
    /// cho mọi item trong 5 container từ `Item.dat`, rồi recompute chỉ số.
    ///
    /// Instance field (`slot`, `id`, `count`, `doben`, `long_val`, `giatri_long`,
    /// `khang`, `texp`) **không** bị ghi đè — chúng đến từ DB.
    pub fn hydrate_templates(&mut self, data: &crate::data::loader::GameData) {
        for (_, bag) in self.containers_mut() {
            for item in bag.iter_mut() {
                if item.id == 0 {
                    continue;
                }
                let Some(def) = data.items.get(&i64::from(item.id)) else {
                    continue;
                };
                let tpl = InventoryItem::from_template(def, item.count);
                item.lv = tpl.lv;
                item.int1 = tpl.int1;
                item.atk1 = tpl.atk1;
                item.def1 = tpl.def1;
                item.hpx1 = tpl.hpx1;
                item.spx1 = tpl.spx1;
                item.agi1 = tpl.agi1;
                item.fai1 = tpl.fai1;
                item.int2 = tpl.int2;
                item.atk2 = tpl.atk2;
                item.def2 = tpl.def2;
                item.hpx2 = tpl.hpx2;
                item.spx2 = tpl.spx2;
                item.agi2 = tpl.agi2;
                item.fai2 = tpl.fai2;
                item.item_hp = tpl.item_hp;
                item.item_sp = tpl.item_sp;
                item.loai = tpl.loai;
                item.thuoctinh = tpl.thuoctinh;
                item.giatri_thuoctinh = tpl.giatri_thuoctinh;
            }
        }
        self.recompute_stats();
    }

    /// Dump Homdo inventory frame (`F444` + len + `1705` + entries).
    pub fn dump_homdo(&self) -> String {
        dump_container(&self.homdo, StorageType::Bag)
    }

    /// Dump Trangbi equipment frame (`F444` + len + `170B` + entries).
    ///
    /// **10 byte/record, KHÔNG byte slot.** Client `FUN_0077CC2C` chia `IDIV 0xA`;
    /// Bear `TSCharacter.sendEquip()` cũng ghi `add16+add8×4+add32` (10B).
    /// Thứ tự record **bắt buộc** theo `slot` tăng dần vì client định tuyến bằng
    /// body position trong `Item.dat`, không đọc byte slot.
    pub fn dump_trangbi(&self) -> String {
        dump_container(&self.trangbi, StorageType::Equip)
    }

    /// Dump TienTrang storage frame (`F444` + len + `1E01` + entries).
    pub fn dump_tientrang(&self) -> String {
        dump_container(&self.tientrang, StorageType::Bank)
    }

    /// Dump Tuideo pouch frame (`F444` + len + `172F` + entries).
    pub fn dump_tuideo(&self) -> String {
        dump_container(&self.tuideo, StorageType::Secondary)
    }

    /// Dump LuuLang storage frame (`F444` + len + `1766` + entries).
    pub fn dump_luulang(&self) -> String {
        dump_container(&self.luulang, StorageType::Warehouse)
    }

    /// Equipped item ids by slot (used in the self-appear frame, `_My_Color` side).
    /// Giới hạn ở slot trang bị người chơi (1..=6); không để lọt trang bị tướng/pet (11..=46).
    pub fn equipped_ids(&self) -> Vec<u16> {
        self.trangbi
            .iter()
            .filter(|i| i.id > 0 && (1..=6).contains(&i.slot))
            .map(|i| i.id)
            .collect()
    }

    /// Dump Hotkeys skill bar frame (`F444` + len + `2801` + entries).
    pub fn dump_hotkeys(&self) -> String {
        let mut entries = String::new();
        for slot in 1..=10 {
            let entry = self.hotkeys.get(slot as usize).copied().unwrap_or_default();
            if entry.kind > 0 && entry.id > 0 {
                entries.push_str(&format!(
                    "{:02X}{}{:02X}",
                    entry.kind,
                    encoder::le16(entry.id),
                    slot
                ));
            }
        }
        if entries.is_empty() {
            crate::protocol::frame(crate::protocol::OPCODE_HOTKEYS, "02")
        } else {
            crate::protocol::frame(crate::protocol::OPCODE_HOTKEYS, &entries)
        }
    }
}

/// Encode 1 record format 12B (slot-led): `17 05`, `1E 01`, `17 2F`, `17 66`.
/// Client chia `IDIV 0xC`. Thứ tự record KHÔNG quan trọng (có byte slot).
pub fn encode_entry_12(item: &InventoryItem) -> String {
    format!(
        "{:02X}{}{:02X}{:02X}{:02X}{:02X}{:02X}{}",
        item.slot,
        encoder::le16(item.id),
        item.count,
        item.doben,
        item.long_val,
        crate::server::inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

/// Encode 1 record format 10B (không slot): `17 0B`.
/// Client chia `IDIV 0xA` và tự định tuyến theo body position.
/// TODO(capture): doben (độ bền) không có mặt trên wire 10B; cần capture để xác nhận nếu client nhận qua frame khác.
pub fn encode_entry_10(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.long_val,
        crate::server::inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

/// Serialize container ra frame dump, chọn layout theo `StorageType`.
pub fn dump_container(items: &[InventoryItem], storage: StorageType) -> String {
    let code = storage.dump_code();
    match storage.entry_layout() {
        EntryLayout::SlotLed12 => {
            let entries: String = items
                .iter()
                .filter(|i| i.id > 0)
                .map(encode_entry_12)
                .collect();
            crate::protocol::frame(code, &entries)
        }
        EntryLayout::Slotless10 => {
            // Giới hạn ở 6 slot trang bị người chơi (1..=6); không xuất trang bị pet (11..46) vào frame 17 0B.
            let mut sorted: Vec<&InventoryItem> = items
                .iter()
                .filter(|i| i.id > 0 && (1..=6).contains(&i.slot))
                .collect();
            sorted.sort_by_key(|i| i.slot); // bắt buộc: định tuyến theo thứ tự slot tăng dần
            let entries: String = sorted.iter().map(|i| encode_entry_10(i)).collect();
            crate::protocol::frame(code, &entries)
        }
    }
}

impl Session {

    /// Add an item to Homdo. Returns the slot(s) written (a capped merge can touch
    /// two slots); empty when the bag is full.
    pub fn add_homdo_item(&mut self, item: InventoryItem) -> Vec<u8> {
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
    pub peer_ip: String,
}

/// Online session registry: authoritative per-player session snapshots, synced
/// by the connection loop on every frame. Cross-player flows (player shop op
/// 0x17 sub 32/33) read and mutate it.
pub fn online_sessions() -> &'static std::sync::Mutex<std::collections::HashMap<u32, Session>> {
    static ONLINE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, Session>>> =
        std::sync::OnceLock::new();
    ONLINE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Safely lock the online session registry, recovering from poison if a previous test or thread panicked.
pub fn lock_online_sessions() -> std::sync::MutexGuard<'static, std::collections::HashMap<u32, Session>> {
    match online_sessions().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Locks player operations in stable id order. A shop purchase locks buyer and
/// seller; ordinary frames and disconnects lock only their own player. This
/// prevents cross-player lost updates without blocking unrelated sessions.
pub async fn lock_player_operations(
    player_ids: impl IntoIterator<Item = u32>,
) -> Vec<tokio::sync::OwnedMutexGuard<()>> {
    type PlayerLock = std::sync::Arc<tokio::sync::Mutex<()>>;
    static LOCKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<u32, PlayerLock>>,
    > = std::sync::OnceLock::new();
    let locks = LOCKS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut ids: Vec<u32> = player_ids.into_iter().filter(|id| *id > 0).collect();
    ids.sort_unstable();
    ids.dedup();
    let selected: Vec<PlayerLock> = {
        let mut registry = locks.lock().unwrap();
        ids.into_iter()
            .map(|id| {
                registry
                    .entry(id)
                    .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
                    .clone()
            })
            .collect()
    };
    let mut guards = Vec::with_capacity(selected.len());
    for lock in selected {
        guards.push(lock.lock_owned().await);
    }
    guards
}

impl Conn {
    pub fn new() -> Self {
        Self {
            decoder: Decoder::new(),
            session: Session::new(),
            peer_ip: String::new(),
        }
    }

    pub fn with_peer_ip(peer_ip: impl Into<String>) -> Self {
        Self {
            decoder: Decoder::new(),
            session: Session::new(),
            peer_ip: peer_ip.into(),
        }
    }
}

impl Default for Conn {
    fn default() -> Self {
        Self::new()
    }
}
