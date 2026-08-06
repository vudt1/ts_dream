//! Wire protocol layer (Chapter 2).
//!
//! Implements the exact framing and primitive encoders of the C# server so
//! the Rust port produces byte-identical traffic. Everything here is a pure
//! transform and is unit-tested without a socket.

/// XOR key (Chapter 8, §2.1). Hardcoded — never configure it.
pub const XOR_KEY: u8 = 0xAD;

/// Frame magic `F4 44` (DataStructure.cs:13-18).
pub const MAGIC: [u8; 2] = [0xF4, 0x44];

/// Minimum client version (§2.3.2). Below this the connection is shut down.
pub const MIN_VERSION: u16 = 186;

/// Server/account id prefix (§1.5).
pub const ID_PREFIX: &str = "vn";

/// Server name (§1.5).
pub const SERVER_NAME: &str = "TSVN";

/// Maximum level (Data.cs:72).
pub const MAX_LEVEL: i64 = 200;

// (Disabled — the admin role was removed; every account is a player. C#
// `Client.isAdmin()` (Client.cs:10163-10170) treated ids below 300012 as
// server/admin. Account ids start at 300000 (`AUTO_INCREMENT`). Kept only as
// reference; do not re-enable without reintroducing a role system.)
// pub const ADMIN_ID_THRESHOLD: u32 = 300012;

pub mod codec;
pub mod encoder;
pub mod frame;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_builds_known_literals_byte_identical() {
        // Matches the fixed literals used across the codebase: len counts every
        // byte after the 4-byte header (code + body).
        assert_eq!(
            frame("0801", "1B0102000000000000"),
            "F4440B0008011B0102000000000000"
        );
        assert_eq!(
            frame("0601", "01000000026400C800"),
            "F4440B00060101000000026400C800"
        );
        assert_eq!(frame("1705", ""), "F44402001705");
    }
}
