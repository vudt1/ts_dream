//! Wire protocol layer (Chapter 2).
//!
//! Implements the exact framing and primitive encoders of the TS Online wire
//! protocol so traffic stays byte-identical to the captured originals.
//! Everything here is a pure transform and is unit-tested without a socket.

/// XOR key (Chapter 8, §2.1). Hardcoded — never configure it.
pub const XOR_KEY: u8 = 0xAD;

/// Frame magic `F4 44`.
pub const MAGIC: [u8; 2] = [0xF4, 0x44];

/// Minimum client version (§2.3.2). Below this the connection is shut down.
pub const MIN_VERSION: u16 = 186;

/// Server/account id prefix (§1.5).
pub const ID_PREFIX: &str = "VN";

/// Server name (§1.5).
pub const SERVER_NAME: &str = "TSVN";

/// Maximum level.
pub const MAX_LEVEL: i64 = 200;

/// All client→server main opcodes listed in the supplied PC protocol table.
/// The table header says 69 handlers, but it contains 60 distinct opcode rows;
/// this registry follows the rows and preserves that discrepancy in the audit.
pub const DOCUMENTED_CLIENT_OPCODES: &[u8] = &[
    0x00, 0x01, 0x02, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x12, 0x13,
    0x14, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24,
    0x25, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x32, 0x36, 0x37, 0x39, 0x3A, 0x3B, 0x3C,
    0x3D, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0xC7,
];

/// All server→client main opcodes listed in the supplied PC protocol table.
pub const DOCUMENTED_SERVER_OPCODES: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
    0x13, 0x14, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24,
    0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
    0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0xC7,
];

pub fn is_documented_client_opcode(opcode: u8) -> bool {
    DOCUMENTED_CLIENT_OPCODES.contains(&opcode)
}

pub fn is_documented_server_opcode(opcode: u8) -> bool {
    DOCUMENTED_SERVER_OPCODES.contains(&opcode)
}

// (Disabled — the admin role was removed; every account is a player. Legacy
// logic treated ids below 300012 as server/admin. Account ids start at
// 300000 (`AUTO_INCREMENT`). Kept only as reference; do not re-enable
// without reintroducing a role system.)
// pub const ADMIN_ID_THRESHOLD: u32 = 300012;

pub mod codec;
pub mod codecs;
pub mod encoder;
pub mod frame;
pub mod profile;
pub mod reader;
pub mod writer;

pub use codecs::{
    ehuman, BattleRoleData, BattleRoleSerializer, FriendExtra, PlayerCard, PlayerInfoCodec,
    ThingData, ThingDataCodec, FRIEND_EXTRA_SIZE, THING_DATA_SIZE,
};
pub use reader::PacketReader;
pub use writer::PacketWriter;

/// Build an outgoing frame: `F444` + LE16(len) + `code` + `body`, where `len`
/// counts every byte after the 4-byte header (i.e. `code` + `body`).
///
/// This is the single place where the frame header and length are computed.
/// Every outgoing packet is built through it (directly, or via a named builder)
/// so the seam between business logic and wire format stays in one module.
pub fn frame(code: &str, body: &str) -> String {
    let total_len = (code.len() + body.len()) / 2;
    format!("F444{}{code}{body}", encoder::le16(total_len as u16))
}
