//! Player card and friend extra information binary codecs.
//!
//! Matches `PlayerInfo.lua:readBaseInfo / readFriendData` and `PlayerInfoCodec.kt`.
//!
//! Card wire format:
//! - nameLen (1B) + name (VISCII string)
//! - level (1B u8)
//! - element (1B u8)
//! - turn3Element (1B u8)
//! - turn (1B u8)
//! - career (1B u8)
//! - sex (1B u8)
//! - hair (1B u8)
//! - color1 (4B u32 LE)
//! - color2 (4B u32 LE)
//!
//! Total fixed part = 15 bytes + (1 + name.len()).
//!
//! Friend extra wire format (20 bytes):
//! - online (1B bool / u8)
//! - friendly (2B u16 LE)
//! - functionFlag (1B u8)
//! - addTime (8B f64 LE OADate)
//! - offlineTime (8B f64 LE OADate)

use crate::error::Result;
use crate::protocol::reader::PacketReader;
use crate::protocol::writer::PacketWriter;
use serde::{Deserialize, Serialize};

/// Total wire size of friend extra data in bytes.
pub const FRIEND_EXTRA_SIZE: usize = 20;

/// Player business card data structure (15 bytes + Pascal string).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlayerCard {
    /// Player display name (VISCII encoded).
    pub name: String,
    /// Player level (1B).
    pub level: u8,
    /// Elemental attribute (1B).
    pub element: u8,
    /// Rebirth tier 3 elemental attribute (1B).
    pub turn3_element: u8,
    /// Rebirth tier / reincarnation count (1B).
    pub turn: u8,
    /// Career / job classification (1B).
    pub career: u8,
    /// Gender (1B).
    pub sex: u8,
    /// Hair / head style (1B).
    pub hair: u8,
    /// Primary color tint (4B LE).
    pub color1: u32,
    /// Secondary color tint (4B LE).
    pub color2: u32,
}

impl PlayerCard {
    /// Serializes this player card into a `PacketWriter`.
    pub fn encode(&self, writer: &mut PacketWriter) {
        writer.write_viscii_pascal(&self.name);
        writer.write_u8(self.level);
        writer.write_u8(self.element);
        writer.write_u8(self.turn3_element);
        writer.write_u8(self.turn);
        writer.write_u8(self.career);
        writer.write_u8(self.sex);
        writer.write_u8(self.hair);
        writer.write_u32_le(self.color1);
        writer.write_u32_le(self.color2);
    }

    /// Deserializes a player card from a `PacketReader`.
    pub fn decode(reader: &mut PacketReader<'_>) -> Result<Self> {
        let name = reader.read_viscii_pascal()?;
        let level = reader.read_u8()?;
        let element = reader.read_u8()?;
        let turn3_element = reader.read_u8()?;
        let turn = reader.read_u8()?;
        let career = reader.read_u8()?;
        let sex = reader.read_u8()?;
        let hair = reader.read_u8()?;
        let color1 = reader.read_u32_le()?;
        let color2 = reader.read_u32_le()?;

        Ok(Self {
            name,
            level,
            element,
            turn3_element,
            turn,
            career,
            sex,
            hair,
            color1,
            color2,
        })
    }

    /// Serializes to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = PacketWriter::with_capacity(32);
        self.encode(&mut writer);
        writer.into_bytes()
    }

    /// Parses from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = PacketReader::new(bytes);
        Self::decode(&mut reader)
    }
}

/// Friend extra information data structure (20 bytes).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FriendExtra {
    /// Online status flag (1B).
    pub online: bool,
    /// Friendship points / intimacy (2B LE).
    pub friendly: u16,
    /// Function flag (1B).
    pub function_flag: u8,
    /// Timestamp when friend was added (8B f64 OADate).
    pub add_time: f64,
    /// Timestamp when friend last logged off (8B f64 OADate).
    pub offline_time: f64,
}

impl FriendExtra {
    /// Serializes friend extra data into a `PacketWriter` (20 bytes).
    pub fn encode(&self, writer: &mut PacketWriter) {
        writer.write_bool(self.online);
        writer.write_u16_le(self.friendly);
        writer.write_u8(self.function_flag);
        writer.write_f64_le(self.add_time);
        writer.write_f64_le(self.offline_time);
    }

    /// Deserializes friend extra data from a `PacketReader` (20 bytes).
    pub fn decode(reader: &mut PacketReader<'_>) -> Result<Self> {
        let online = reader.read_bool()?;
        let friendly = reader.read_u16_le()?;
        let function_flag = reader.read_u8()?;
        let add_time = reader.read_f64_le()?;
        let offline_time = reader.read_f64_le()?;

        Ok(Self {
            online,
            friendly,
            function_flag,
            add_time,
            offline_time,
        })
    }

    /// Converts this `FriendExtra` into a fixed 20-byte array.
    pub fn to_bytes(&self) -> [u8; FRIEND_EXTRA_SIZE] {
        let mut writer = PacketWriter::with_capacity(FRIEND_EXTRA_SIZE);
        self.encode(&mut writer);
        let bytes = writer.into_bytes();
        let mut arr = [0u8; FRIEND_EXTRA_SIZE];
        arr.copy_from_slice(&bytes[..FRIEND_EXTRA_SIZE]);
        arr
    }

    /// Parses from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = PacketReader::new(bytes);
        Self::decode(&mut reader)
    }
}

/// Standalone codec utility for PlayerCard and FriendExtra.
pub struct PlayerInfoCodec;

impl PlayerInfoCodec {
    /// Writes player card directly to writer.
    pub fn write_player_card(writer: &mut PacketWriter, card: &PlayerCard) {
        card.encode(writer);
    }

    /// Writes friend extra directly to writer.
    pub fn write_friend_extra(writer: &mut PacketWriter, extra: &FriendExtra) {
        extra.encode(writer);
    }
}
