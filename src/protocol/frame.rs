//! Frame codec (§2.1): concatenated-frame splitting on the wire.
//!
//! **Receiver decode:** every received byte is XOR'd with `0xAD` and
//! converted to uppercase hex. The length field is the hex at offset 4
//! (chars 4..7, little-endian u16) = byte count **after** the 4-byte header.
//! A complete frame is `4 + length` bytes = `8 + length*2` hex chars. Frames
//! are concatenated; a partial trailing frame is buffered and prepended to
//! the next chunk.
//!
//! **Send path:** a pure transform — build the hex-string packet → hex-decode
//! to bytes → XOR every byte → one write. No checksum, no trailer (research
//! 06 §(1)).

use super::encoder;
use crate::error::{Result, TsError};

/// Read up to N bytes per receive chunk.
pub const READ_BUFFER: usize = 8192;

/// A single decoded frame in hex-string form (the wire's compare unit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Uppercase hex string of the decoded (XOR-reversed) frame, `F444…`.
    pub hex: String,
}

impl Frame {
    /// The 4+N decoded payload bytes.
    pub fn payload(&self) -> Option<Vec<u8>> {
        encoder::bytes(&self.hex)
    }

    pub fn opcode(&self) -> Option<u8> {
        self.payload().and_then(|b| b.get(4).copied())
    }

    pub fn sub(&self) -> Option<u8> {
        self.payload().and_then(|b| b.get(5).copied())
    }
}

/// Wraps a receive stream: accumulates chunks and yields complete frames.
#[derive(Default)]
pub struct Decoder {
    /// Hex-encoded pending buffer (partially received frame, already the
    /// XOR-decoded hex string).
    pending: String,
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            pending: String::new(),
        }
    }

    /// Consume a raw (still-XORed) receive chunk and return any complete
    /// frames, in order, as decoded hex strings. A partial trailing frame is
    /// retained for the next chunk.
    pub fn feed(&mut self, raw: &[u8]) -> Vec<String> {
        let decoded: Vec<u8> = raw.iter().map(|b| b ^ super::XOR_KEY).collect();
        self.pending.push_str(&encoder::hex(&decoded));
        let mut frames = Vec::new();
        loop {
            if self.pending.len() < 8 {
                break;
            }
            // length field: hex chars 4..7 -> 2 bytes little-endian u16.
            let Some(lb) = u8::from_str_radix(&self.pending[4..6], 16).ok() else {
                break;
            };
            let Some(hb) = u8::from_str_radix(&self.pending[6..8], 16).ok() else {
                break;
            };
            let len = u16::from_le_bytes([lb, hb]);
            let total_chars = 8usize + (len as usize) * 2;
            if self.pending.len() < total_chars {
                break; // partial trailing frame
            }
            let frame = self.pending[..total_chars].to_string();
            self.pending = self.pending[total_chars..].to_string();
            frames.push(frame);
        }
        frames
    }

    pub fn pending(&self) -> &str {
        &self.pending
    }
}

/// Build the on-wire bytes for a decoded hex packet: hex → bytes → XOR.
pub fn encode_to_wire(frame_hex: &str) -> Result<Vec<u8>> {
    let bytes = encoder::bytes(frame_hex)
        .ok_or_else(|| TsError::Protocol(format!("bad frame hex: {frame_hex}")))?;
    Ok(encoder::xor01(&bytes))
}

/// Validate a decoded frame carries the `F4 44` magic.
pub fn check_magic(decoded: &[u8]) -> bool {
    decoded.len() >= 2 && decoded[0] == 0xF4 && decoded[1] == 0x44
}