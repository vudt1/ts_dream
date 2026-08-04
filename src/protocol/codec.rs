//! High-level packet builder (the `F444…` frames handlers emit).

use super::encoder;

/// Minimal Packet builder producing the `F444` + len + opcode/sub + payload
/// frames. Officially the wire is `F444` + length(LE u16 bytes-after-header)
/// + payload; `SEND(hex)` in the C# code builds this framing automatically.
#[derive(Debug, Clone, Default)]
pub struct Packet(String);

impl Packet {
    pub fn new() -> Self {
        Self(String::new())
    }

    /// Start a new packet with a given opcode/sub (bytes 4..6).
    pub fn opcode(op: u8, sub: u8) -> Self {
        let mut p = Self::new();
        p.raw_byte(0xF4);
        p.raw_byte(0x44);
        p.raw_byte(op);
        p.raw_byte(sub);
        p
    }

    /// Append raw payload hex (already uppercase hex).
    pub fn raw(mut self, hex: &str) -> Self {
        self.0.push_str(hex);
        self
    }

    /// Append a byte as 2 hex digits.
    pub fn raw_byte(&mut self, b: u8) -> &mut Self {
        self.0.push_str(&format!("{:02X}", b));
        self
    }

    /// The full frame text: `F444` + len + opcode/sub + payload, with the
    /// length field computed from the body length (bytes after the header).
    pub fn build(self) -> String {
        let body = &self.0[4..];
        let len_bytes = body.len() / 2;
        let mut out = String::from("F444");
        out.push_str(&encoder::le16(len_bytes as u16));
        out.push_str(body);
        out
    }

    /// Build as on-wire (XOR) bytes.
    pub fn build_wire(self) -> crate::error::Result<Vec<u8>> {
        super::frame::encode_to_wire(&self.build())
    }
}

/// Convenience: `F444` + len + raw hex (a complete packet pre-framed).
pub fn frame(body_hex: &str) -> String {
    let mut o = String::from("F444");
    o.push_str(&encoder::le16((body_hex.len() / 2) as u16));
    o.push_str(body_hex);
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_frame_length() {
        // opcode 0x03 sub 0x01, empty body -> F44402000301
        let p = Packet::opcode(0x03, 0x01).build();
        assert_eq!(p, "F44402000301");
    }

    #[test]
    fn full_frame_length_field() {
        // body = opcode/sub 00 00 + payload 01 00 = 4 bytes -> len 4 "0400"
        let p = Packet::opcode(0x00, 0x00).raw("0100").build();
        assert_eq!(p, "F444040000000100");
        assert_eq!(p.len(), 16);
    }
}