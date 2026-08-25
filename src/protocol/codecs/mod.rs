//! Standardized domain binary codecs.
//!
//! Provides byte-exact serializers and deserializers for core TS Online entities:
//! - `ThingData` (35 bytes item wire format)
//! - `PlayerCard` & `FriendExtra` (Player card and social extra data)
//! - `BattleRoleData` & `BattleRoleSerializer` (Battle entity appearances for opcodes `0x0B` / `0x32`)

pub mod battle_role;
pub mod player_info;
pub mod thing_data;

pub use battle_role::{ehuman, BattleRoleData, BattleRoleSerializer};
pub use player_info::{FriendExtra, PlayerCard, PlayerInfoCodec, FRIEND_EXTRA_SIZE};
pub use thing_data::{ThingData, ThingDataCodec, THING_DATA_SIZE};
