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

/// Server/admin id threshold (Client.cs): ids below are treated as admin.
pub const ADMIN_ID_THRESHOLD: u32 = 300012;

pub mod codec;
pub mod encoder;
pub mod frame;