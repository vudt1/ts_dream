//! Standardized 35-byte ThingData binary codec.
//!
//! Strictly matches `Item.lua:81-100` (read) and `Item.lua:138-158` (write),
//! as well as `ThingDataCodec.kt` in the Kotlin TS Mobile reference architecture.
//!
//! Field order (35 bytes total, Little-Endian):
//! - Id (2B u16 LE)
//! - quant (4B i32 LE)
//! - damage (1B u8)
//! - element (1B u8)
//! - elementValue (1B u8)
//! - proofKind (1B u8)
//! - growLv (1B u8)
//! - growExp (4B i32 LE)
//! - specialKind (1B u8)
//! - stoneAttr (1B u8)
//! - stoneLv (1B u8)
//! - enhanceLv (1B u8)
//! - delTime (8B f64 LE OADate)
//! - damagedItemId (2B u16 LE)
//! - isLock (1B bool)
//! - Reinforced (1B u8)
//! - affix1 (1B u8)
//! - affix2 (1B u8)
//! - affix3 (1B u8)
//! - styleLv (1B u8)

use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::protocol::reader::PacketReader;
use crate::protocol::writer::PacketWriter;

/// Total wire size of a single ThingData struct in bytes.
pub const THING_DATA_SIZE: usize = 35;

/// Item instance value object representation matching the 35-byte wire protocol.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThingData {
    /// Item definition ID (UInt16). 0 means empty slot.
    pub item_id: u16,
    /// Stack quantity (Int32).
    pub quantity: i32,
    /// Durability / damage indicator (0=perfect, 250=broken).
    pub damage: u8,
    /// Elemental attribute (0=none, 1=earth, 2=water, 3=fire, 4=wind, 5=heart, 7=light, 8=dark).
    pub element: u8,
    /// Elemental attribute value.
    pub element_value: u8,
    /// Resistance / proof kind.
    pub proof_kind: u8,
    /// Growth level (for spirit / growth weapons).
    pub grow_level: u8,
    /// Growth EXP (for spirit / growth weapons).
    pub grow_exp: i32,
    /// Special behavior flags (0=default, 1=tradable once, etc.).
    pub special_kind: u8,
    /// Socket stone elemental attribute.
    pub stone_attr: u8,
    /// Socket stone enhancement level.
    pub stone_level: u8,
    /// Exclusive weapon enhancement level.
    pub enhance_level: u8,
    /// Deletion timestamp in OADate (f64) format. 0.0 = permanent.
    pub delete_time: f64,
    /// Item definition ID prior to being damaged.
    pub damaged_item_id: u16,
    /// Locked flag.
    pub is_locked: bool,
    /// Stage / reinforcement tier.
    pub reinforced: u8,
    /// Refinement / affix 1 level.
    pub affix1: u8,
    /// Refinement / affix 2 level.
    pub affix2: u8,
    /// Refinement / affix 3 level.
    pub affix3: u8,
    /// Fashion / outfit enhancement level.
    pub style_level: u8,
}

impl ThingData {
    /// Returns `true` if this represents an empty slot (item ID == 0).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.item_id == 0
    }

    /// Creates an empty `ThingData` instance.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Serializes this `ThingData` into a `PacketWriter` (35 bytes).
    pub fn encode(&self, writer: &mut PacketWriter) {
        writer.write_u16_le(self.item_id);
        writer.write_i32_le(self.quantity);
        writer.write_u8(self.damage);
        writer.write_u8(self.element);
        writer.write_u8(self.element_value);
        writer.write_u8(self.proof_kind);
        writer.write_u8(self.grow_level);
        writer.write_i32_le(self.grow_exp);
        writer.write_u8(self.special_kind);
        writer.write_u8(self.stone_attr);
        writer.write_u8(self.stone_level);
        writer.write_u8(self.enhance_level);
        writer.write_f64_le(self.delete_time);
        writer.write_u16_le(self.damaged_item_id);
        writer.write_bool(self.is_locked);
        writer.write_u8(self.reinforced);
        writer.write_u8(self.affix1);
        writer.write_u8(self.affix2);
        writer.write_u8(self.affix3);
        writer.write_u8(self.style_level);
    }

    /// Deserializes a `ThingData` from a `PacketReader` (35 bytes).
    pub fn decode(reader: &mut PacketReader<'_>) -> Result<Self> {
        let item_id = reader.read_u16_le()?;
        let quantity = reader.read_i32_le()?;
        let damage = reader.read_u8()?;
        let element = reader.read_u8()?;
        let element_value = reader.read_u8()?;
        let proof_kind = reader.read_u8()?;
        let grow_level = reader.read_u8()?;
        let grow_exp = reader.read_i32_le()?;
        let special_kind = reader.read_u8()?;
        let stone_attr = reader.read_u8()?;
        let stone_level = reader.read_u8()?;
        let enhance_level = reader.read_u8()?;
        let delete_time = reader.read_f64_le()?;
        let damaged_item_id = reader.read_u16_le()?;
        let is_locked = reader.read_bool()?;
        let reinforced = reader.read_u8()?;
        let affix1 = reader.read_u8()?;
        let affix2 = reader.read_u8()?;
        let affix3 = reader.read_u8()?;
        let style_level = reader.read_u8()?;

        Ok(Self {
            item_id,
            quantity,
            damage,
            element,
            element_value,
            proof_kind,
            grow_level,
            grow_exp,
            special_kind,
            stone_attr,
            stone_level,
            enhance_level,
            delete_time,
            damaged_item_id,
            is_locked,
            reinforced,
            affix1,
            affix2,
            affix3,
            style_level,
        })
    }

    /// Converts this `ThingData` into a fixed 35-byte array.
    pub fn to_bytes(&self) -> [u8; THING_DATA_SIZE] {
        let mut writer = PacketWriter::with_capacity(THING_DATA_SIZE);
        self.encode(&mut writer);
        let bytes = writer.into_bytes();
        let mut arr = [0u8; THING_DATA_SIZE];
        arr.copy_from_slice(&bytes[..THING_DATA_SIZE]);
        arr
    }

    /// Parses a `ThingData` from a raw 35-byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = PacketReader::new(bytes);
        Self::decode(&mut reader)
    }
}

/// Standalone codec utility for `ThingData`.
pub struct ThingDataCodec;

impl ThingDataCodec {
    /// Encode a single `ThingData` to bytes (35 bytes).
    pub fn encode(thing: &ThingData) -> [u8; THING_DATA_SIZE] {
        thing.to_bytes()
    }

    /// Decode a single `ThingData` from bytes.
    pub fn decode(bytes: &[u8]) -> Result<ThingData> {
        ThingData::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thing_data_roundtrip() {
        let item = ThingData {
            item_id: 23145,
            quantity: 50,
            damage: 10,
            element: 3,
            element_value: 45,
            proof_kind: 2,
            grow_level: 5,
            grow_exp: 12000,
            special_kind: 1,
            stone_attr: 4,
            stone_level: 7,
            enhance_level: 10,
            delete_time: 44927.5,
            damaged_item_id: 23000,
            is_locked: true,
            reinforced: 3,
            affix1: 12,
            affix2: 15,
            affix3: 18,
            style_level: 6,
        };

        let bytes = item.to_bytes();
        assert_eq!(bytes.len(), THING_DATA_SIZE);

        let decoded = ThingData::from_bytes(&bytes).unwrap();
        assert_eq!(item, decoded);
    }

    #[test]
    fn test_empty_thing_data() {
        let empty = ThingData::empty();
        assert!(empty.is_empty());

        let bytes = empty.to_bytes();
        assert_eq!(bytes.len(), 35);
        assert_eq!(bytes, [0u8; 35]);

        let decoded = ThingData::from_bytes(&bytes).unwrap();
        assert_eq!(empty, decoded);
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_partial_bytes_error() {
        let truncated = [0u8; 30];
        assert!(ThingData::from_bytes(&truncated).is_err());
    }
}
