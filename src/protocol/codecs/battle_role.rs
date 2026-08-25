//! Battle role packet serialization and deserialization.
//!
//! Formats match `FightField.lua:RoleAppear()` and `FightManager.lua:ReciveFightAppearenceData()`,
//! as well as `BattleRoleSerializer.kt` from Kotlin TS Mobile architecture.
//!
//! Used for opcodes `0x0B` (`0x0BFA` battle open, `0x0B05` role appear) and `0x32`.

use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::protocol::reader::PacketReader;
use crate::protocol::writer::PacketWriter;

/// Character entity kind constants — matches `define.lua:EHuman`.
pub mod ehuman {
    pub const NONE: u8 = 0;
    pub const PLAYER: u8 = 1;
    pub const PLAYERS: u8 = 2;
    pub const MAP_NPC: u8 = 3;
    pub const FOLLOW_NPC: u8 = 4;
    pub const SCENE_ELM: u8 = 5;
    pub const GUARD_NPC: u8 = 6;
    pub const MINE_NPC: u8 = 7;
    pub const OWNER_NPC: u8 = 8;
    pub const DIVIDE: u8 = 9;
    pub const SOLDIER: u8 = 11;
    pub const MACHINE: u8 = 12;
    pub const CTRL_MACH: u8 = 13;
    pub const AUTO_FOLLOW: u8 = 14;
    pub const CTRL_SOLD: u8 = 15;
    pub const FOREIGN_NPC: u8 = 16;
    pub const CART_NPC: u8 = 17;
    pub const CHAOS_GOD: u8 = 18;
    pub const PET_NPC: u8 = 19;
    pub const QUESTION: u8 = 20;
    pub const WORLD_BOSS: u8 = 21;
    pub const AUTOMANUAL_PLAYER: u8 = 28;
    pub const AUTOMANUAL_NPC: u8 = 29;
    pub const HOUSE_WARRIOR: u8 = 30;
    pub const BATTLE_ROYALE_NPC: u8 = 31;
    pub const DUPLICATE: u8 = 255;
    pub const NPC: u8 = MAP_NPC;
}

/// Battle role entity serialization data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BattleRoleData {
    /// War type indicator (1B).
    pub war_type: u8,
    /// Character kind constant from `ehuman` (1B).
    pub human_kind: u8,
    /// Unique role / character ID (8B LE).
    pub role_id: u64,
    /// NPC template ID (2B LE).
    pub npc_id: u16,
    /// Master / owner character ID (8B LE).
    pub master_id: u64,
    /// Column position in the battle grid (1B).
    pub col: u8,
    /// Row position in the battle grid (1B).
    pub row: u8,
    /// Maximum HP (4B LE).
    pub max_hp: u32,
    /// Maximum SP (4B LE).
    pub max_sp: u32,
    /// Current HP (4B LE).
    pub hp: u32,
    /// Current SP (4B LE).
    pub sp: u32,
    /// Level (2B LE).
    pub level: u16,
    /// Upgrade / reborn level (1B).
    pub upgrade_lv: u8,
    /// Elemental attribute (1B).
    pub element: u8,
    /// Name string (used for player appearance and follow NPC).
    pub name: String,
    /// Gender (1B, players only).
    pub sex: u8,
    /// Face visual ID (1B, players only).
    pub face: u8,
    /// Hair visual ID (1B, players only).
    pub hair: u8,
    /// Color tint 1 (4B LE, players only).
    pub color1: u32,
    /// Color tint 2 (4B LE, players only).
    pub color2: u32,
    /// Reincarnation count (1B, players only).
    pub turn: u8,
    /// Career classification (1B, players only).
    pub career: u8,
    /// Equipped item IDs (UInt16 array, players only).
    pub equip_item_ids: Vec<u16>,
    /// Fashion / outfit item IDs (UInt16 array, players only).
    pub outfit_item_ids: Vec<u16>,
}

impl BattleRoleData {
    /// Returns true if this role kind uses player appearance format (`ReciveFightAppearenceData`).
    pub fn is_player_kind(kind: u8) -> bool {
        matches!(
            kind,
            ehuman::PLAYER | ehuman::PLAYERS | ehuman::DIVIDE | ehuman::AUTOMANUAL_PLAYER
        )
    }

    /// Returns true if this role kind uses name-only format.
    pub fn is_named_npc_kind(kind: u8) -> bool {
        matches!(kind, ehuman::FOLLOW_NPC | ehuman::AUTOMANUAL_NPC)
    }

    /// Serializes this battle role to a writer.
    pub fn encode(&self, writer: &mut PacketWriter) {
        // Base fields (38 bytes)
        writer.write_u8(self.war_type);
        writer.write_u8(self.human_kind);
        writer.write_u64_le(self.role_id);
        writer.write_u16_le(self.npc_id);
        writer.write_u64_le(self.master_id);
        writer.write_u8(self.col);
        writer.write_u8(self.row);
        writer.write_u32_le(self.max_hp);
        writer.write_u32_le(self.max_sp);
        writer.write_u32_le(self.hp);
        writer.write_u32_le(self.sp);
        writer.write_u16_le(self.level);
        writer.write_u8(self.upgrade_lv);
        writer.write_u8(self.element);

        // Conditional appearance data
        if Self::is_player_kind(self.human_kind) {
            Self::write_player_appearance(writer, self);
        } else if Self::is_named_npc_kind(self.human_kind) {
            writer.write_viscii_pascal(&self.name);
        }
    }

    /// Deserializes a battle role from a reader.
    pub fn decode(reader: &mut PacketReader<'_>) -> Result<Self> {
        let war_type = reader.read_u8()?;
        let human_kind = reader.read_u8()?;
        let role_id = reader.read_u64_le()?;
        let npc_id = reader.read_u16_le()?;
        let master_id = reader.read_u64_le()?;
        let col = reader.read_u8()?;
        let row = reader.read_u8()?;
        let max_hp = reader.read_u32_le()?;
        let max_sp = reader.read_u32_le()?;
        let hp = reader.read_u32_le()?;
        let sp = reader.read_u32_le()?;
        let level = reader.read_u16_le()?;
        let upgrade_lv = reader.read_u8()?;
        let element = reader.read_u8()?;

        let mut data = Self {
            war_type,
            human_kind,
            role_id,
            npc_id,
            master_id,
            col,
            row,
            max_hp,
            max_sp,
            hp,
            sp,
            level,
            upgrade_lv,
            element,
            name: String::new(),
            sex: 0,
            face: 0,
            hair: 0,
            color1: 0,
            color2: 0,
            turn: 0,
            career: 0,
            equip_item_ids: Vec::new(),
            outfit_item_ids: Vec::new(),
        };

        if Self::is_player_kind(human_kind) {
            Self::read_player_appearance(reader, &mut data)?;
        } else if Self::is_named_npc_kind(human_kind) {
            data.name = reader.read_viscii_pascal()?;
        }

        Ok(data)
    }

    /// Writes player appearance details (`ReciveFightAppearenceData`).
    fn write_player_appearance(writer: &mut PacketWriter, data: &BattleRoleData) {
        writer.write_viscii_pascal(&data.name);
        writer.write_u8(data.sex);
        writer.write_u8(data.element);
        writer.write_u8(data.level as u8);
        writer.write_u8(0); // bad luck god marker (1B)
        writer.write_u32_le(0); // fortune god marker (4B)
        writer.write_u16_le(0); // spirit No (2B)
        writer.write_u8(data.face);
        writer.write_u8(data.hair);
        writer.write_u8(0); // body style (1B)
        writer.write_u32_le(data.color1);
        writer.write_u32_le(data.color2);
        writer.write_u16_le(0); // PVP wins (2B)
        writer.write_u8(0); // PVP rank (1B)
        writer.write_u8(0); // NPC challenge (1B)
        writer.write_u8(data.turn);
        writer.write_u8(data.career);

        // Equipments
        writer.write_u8(data.equip_item_ids.len() as u8);
        for &item_id in &data.equip_item_ids {
            writer.write_u16_le(item_id);
        }

        // Outfits
        writer.write_u8(data.outfit_item_ids.len() as u8);
        for &item_id in &data.outfit_item_ids {
            writer.write_u16_le(item_id);
        }

        writer.write_u8(0); // team state (1B)
        writer.write_u32_le(0); // corps ID (4B)
    }

    /// Reads player appearance details (`ReciveFightAppearenceData`).
    fn read_player_appearance(reader: &mut PacketReader<'_>, data: &mut BattleRoleData) -> Result<()> {
        data.name = reader.read_viscii_pascal()?;
        data.sex = reader.read_u8()?;
        let _app_element = reader.read_u8()?;
        let _app_level = reader.read_u8()?;
        let _bad_luck = reader.read_u8()?;
        let _fortune = reader.read_u32_le()?;
        let _spirit_no = reader.read_u16_le()?;
        data.face = reader.read_u8()?;
        data.hair = reader.read_u8()?;
        let _body_style = reader.read_u8()?;
        data.color1 = reader.read_u32_le()?;
        data.color2 = reader.read_u32_le()?;
        let _pvp_wins = reader.read_u16_le()?;
        let _pvp_rank = reader.read_u8()?;
        let _npc_challenge = reader.read_u8()?;
        data.turn = reader.read_u8()?;
        data.career = reader.read_u8()?;

        // Equipments
        let equip_count = reader.read_u8()? as usize;
        data.equip_item_ids.clear();
        data.equip_item_ids.reserve(equip_count);
        for _ in 0..equip_count {
            data.equip_item_ids.push(reader.read_u16_le()?);
        }

        // Outfits
        let outfit_count = reader.read_u8()? as usize;
        data.outfit_item_ids.clear();
        data.outfit_item_ids.reserve(outfit_count);
        for _ in 0..outfit_count {
            data.outfit_item_ids.push(reader.read_u16_le()?);
        }

        let _team_state = reader.read_u8()?;
        let _corps_id = reader.read_u32_le()?;

        Ok(())
    }
}

/// Standalone serializer matching `BattleRoleSerializer` in Kotlin.
pub struct BattleRoleSerializer;

impl BattleRoleSerializer {
    /// Serializes a single `BattleRoleData` into bytes.
    pub fn serialize(data: &BattleRoleData) -> Vec<u8> {
        let mut writer = PacketWriter::with_capacity(128);
        data.encode(&mut writer);
        writer.into_bytes()
    }

    /// Serializes a slice of `BattleRoleData` into contiguous bytes.
    pub fn serialize_all(roles: &[BattleRoleData]) -> Vec<u8> {
        let mut writer = PacketWriter::with_capacity(roles.len() * 64);
        for role in roles {
            role.encode(&mut writer);
        }
        writer.into_bytes()
    }

    /// Deserializes a single `BattleRoleData` from reader.
    pub fn deserialize(reader: &mut PacketReader<'_>) -> Result<BattleRoleData> {
        BattleRoleData::decode(reader)
    }

    /// Deserializes `count` instances of `BattleRoleData` from reader.
    pub fn deserialize_all(reader: &mut PacketReader<'_>, count: usize) -> Result<Vec<BattleRoleData>> {
        let mut roles = Vec::with_capacity(count);
        for _ in 0..count {
            roles.push(BattleRoleData::decode(reader)?);
        }
        Ok(roles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_battle_role_roundtrip() {
        let role = BattleRoleData {
            war_type: 1,
            human_kind: ehuman::PLAYER,
            role_id: 300001,
            npc_id: 0,
            master_id: 0,
            col: 1,
            row: 2,
            max_hp: 1500,
            max_sp: 800,
            hp: 1450,
            sp: 790,
            level: 85,
            upgrade_lv: 1,
            element: 2,
            name: "TriệuVân".to_string(),
            sex: 1,
            face: 3,
            hair: 4,
            color1: 0x11223344,
            color2: 0x55667788,
            turn: 1,
            career: 2,
            equip_item_ids: vec![12001, 12002, 12003],
            outfit_item_ids: vec![19001],
        };

        let bytes = BattleRoleSerializer::serialize(&role);
        let mut reader = PacketReader::new(&bytes);
        let decoded = BattleRoleSerializer::deserialize(&mut reader).unwrap();
        assert_eq!(role, decoded);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_follow_npc_battle_role_roundtrip() {
        let role = BattleRoleData {
            war_type: 1,
            human_kind: ehuman::FOLLOW_NPC,
            role_id: 40001,
            npc_id: 11005,
            master_id: 300001,
            col: 0,
            row: 1,
            max_hp: 3500,
            max_sp: 1200,
            hp: 3500,
            sp: 1200,
            level: 100,
            upgrade_lv: 0,
            element: 3,
            name: "QuanVũ".to_string(),
            ..Default::default()
        };

        let bytes = BattleRoleSerializer::serialize(&role);
        let mut reader = PacketReader::new(&bytes);
        let decoded = BattleRoleSerializer::deserialize(&mut reader).unwrap();
        assert_eq!(role, decoded);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_monster_npc_battle_role_roundtrip() {
        let role = BattleRoleData {
            war_type: 1,
            human_kind: ehuman::MINE_NPC,
            role_id: 50001,
            npc_id: 14002,
            master_id: 0,
            col: 2,
            row: 0,
            max_hp: 200,
            max_sp: 50,
            hp: 200,
            sp: 50,
            level: 15,
            upgrade_lv: 0,
            element: 1,
            ..Default::default()
        };

        let bytes = BattleRoleSerializer::serialize(&role);
        // Base fields = 42 bytes exactly for NPC without extra appearance
        assert_eq!(bytes.len(), 42);

        let mut reader = PacketReader::new(&bytes);
        let decoded = BattleRoleSerializer::deserialize(&mut reader).unwrap();
        assert_eq!(role, decoded);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_serialize_all_multiple_roles() {
        let player = BattleRoleData {
            war_type: 1,
            human_kind: ehuman::PLAYER,
            role_id: 300001,
            npc_id: 0,
            master_id: 0,
            col: 1,
            row: 2,
            max_hp: 1500,
            max_sp: 800,
            hp: 1450,
            sp: 790,
            level: 85,
            upgrade_lv: 0,
            element: 2,
            name: "Player1".to_string(),
            sex: 1,
            face: 1,
            hair: 2,
            color1: 0,
            color2: 0,
            turn: 0,
            career: 0,
            equip_item_ids: vec![10001],
            outfit_item_ids: vec![],
        };

        let pet = BattleRoleData {
            war_type: 1,
            human_kind: ehuman::FOLLOW_NPC,
            role_id: 40001,
            npc_id: 11001,
            master_id: 300001,
            col: 0,
            row: 2,
            max_hp: 800,
            max_sp: 300,
            hp: 800,
            sp: 300,
            level: 50,
            upgrade_lv: 0,
            element: 1,
            name: "Pet1".to_string(),
            ..Default::default()
        };

        let enemy = BattleRoleData {
            war_type: 1,
            human_kind: ehuman::MINE_NPC,
            role_id: 50001,
            npc_id: 12001,
            master_id: 0,
            col: 3,
            row: 1,
            max_hp: 1000,
            max_sp: 200,
            hp: 1000,
            sp: 200,
            level: 60,
            upgrade_lv: 0,
            element: 4,
            ..Default::default()
        };

        let roles = vec![player, pet, enemy];
        let bytes = BattleRoleSerializer::serialize_all(&roles);
        let mut reader = PacketReader::new(&bytes);
        let decoded = BattleRoleSerializer::deserialize_all(&mut reader, 3).unwrap();
        assert_eq!(roles, decoded);
        assert_eq!(reader.remaining(), 0);
    }
}
