//! Text encoding contract (Chapter 4).
//!
//! Wire encoding is **VISCII 1.1** — every byte 0x00–0xFF maps to one
//! Vietnamese character. Names are held in memory as `Vec<u8>` of VISCII
//! bytes; UTF-8 mojibake (Npcs/Items) is reversed back to VISCII car-by-char.
//!
//! Garble exceptions are replicated bug-for-bug so the Rust output diffs
//! byte-exactly against captured C# traffic (Chapter 4 §4.3/§4.6).

use std::collections::HashMap;

/// Reverse mojibake map: a mojibake Unicode char -> its VISCII byte.
///
/// From research 03 §4.1:
/// - U+0000–U+007F -> same byte (ASCII).
/// - U+0080–U+009F -> byte = codepoint (C1 pass-through).
/// - U+00A0–U+00FF -> byte = codepoint (Latin-1 = CP1252 in this range).
/// - CP1252 punctuation defined in 0x80–0x9F -> its byte.
pub fn reverse_mojibake_map() -> HashMap<u32, u8> {
    let mut m = HashMap::new();
    for cp in 0u32..=0x7F {
        m.insert(cp, cp as u8);
    }
    for cp in 0x80u32..=0xFF {
        m.insert(cp, cp as u8);
    }
    // CP1252-defined punctuation in the C1 range maps to its distinct byte.
    let cp1252: &[(u32, u8)] = &[
        (0x20AC, 0x80), // €
        (0x201A, 0x82), // ‚
        (0x0192, 0x83), // ƒ
        (0x201E, 0x84), // „
        (0x2026, 0x85), // …
        (0x2020, 0x86), // †
        (0x2021, 0x87), // ‡
        (0x02C6, 0x88), // ˆ
        (0x2030, 0x89), // ‰
        (0x0160, 0x8A), // Š
        (0x2039, 0x8B), // ‹
        (0x0152, 0x8C), // Œ
        (0x017D, 0x8E), // Ž
        (0x2018, 0x91), // ‘
        (0x2019, 0x92), // ’
        (0x201C, 0x93), // “
        (0x201D, 0x94), // ”
        (0x2022, 0x95), // •
        (0x2013, 0x96), // –
        (0x2014, 0x97), // —
        (0x02DC, 0x98), // ˜
        (0x2122, 0x99), // ™
        (0x0161, 0x9A), // š
        (0x203A, 0x9B), // ›
        (0x0153, 0x9C), // œ
        (0x017E, 0x9E), // ž
        (0x0178, 0x9F), // Ÿ
    ];
    for &(cp, byte) in cp1252 {
        m.insert(cp, byte);
    }
    m
}

/// True if a 32-bit char is the single unmappable mojibake char (U+0103 `ă`).
pub fn is_unmappable(cp: u32) -> bool {
    cp == 0x0103
}

/// Map a mojibake Unicode char to its VISCII byte.
///
/// Returns the byte for mappable chars, and for the single unmappable `ă`
/// normalizes to `0xE5` (VISCII ă) as the spec §4.4 instructs.
pub fn char_to_viscii(cp: u32, map: &HashMap<u32, u8>) -> u8 {
    if is_unmappable(cp) {
        return 0xE5;
    }
    map.get(&cp).copied().unwrap_or(0x3F) // '?' fallback
}

/// Translate a Unicode string (the in-memory mojibake text) into VISCII bytes.
pub fn to_viscii(s: &str) -> Vec<u8> {
    let map = reverse_mojibake_map();
    s.chars().map(|c| char_to_viscii(c as u32, &map)).collect()
}

/// The VISCII byte→Unicode table used for display (TextEncoder.cs:15-42), the
/// 102-entry table imported character-for-character, plus the `0xD0→Đ`,
/// `0xDD→Đ` additions from smethod_17. Bytes outside the table fall back to
/// Latin-1 pass-through (`(char)byte`), exactly like `convertToUniCode`.
pub fn viscii_to_unicode(byte: u8) -> char {
    match byte {
        0x02 => '\u{1EB2}', // Ẳ
        0x05 => '\u{1EB4}', // Ẵ
        0x06 => '\u{1EAA}', // Ẫ
        0x14 => '\u{1EF6}', // Ỷ
        0x19 => '\u{1EF8}', // Ỹ
        0x1E => '\u{1EF4}', // Ỵ
        0x80 => '\u{1EA0}', // Ạ
        0x81 => '\u{1EAE}', // Ắ
        0x82 => '\u{1EB0}', // Ằ
        0x83 => '\u{1EB6}', // Ặ
        0x84 => '\u{1EA4}', // Ấ
        0x85 => '\u{1EA6}', // Ầ
        0x86 => '\u{1EA8}', // Ẩ
        0x87 => '\u{1EAC}', // Ậ
        0x88 => '\u{1EBC}', // Ẽ
        0x89 => '\u{1EB8}', // Ẹ
        0x8A => '\u{1EBE}', // Ế
        0x8B => '\u{1EC0}', // Ề
        0x8C => '\u{1EC2}', // Ể
        0x8D => '\u{1EC4}', // Ễ
        0x8E => '\u{1EC6}', // Ệ
        0x8F => '\u{1ED0}', // Ố
        0x90 => '\u{1ED2}', // Ồ
        0x91 => '\u{1ED4}', // Ổ
        0x92 => '\u{1ED6}', // Ỗ
        0x93 => '\u{1ED8}', // Ộ
        0x94 => '\u{1EE2}', // Ợ
        0x95 => '\u{1EDA}', // Ớ
        0x96 => '\u{1EDC}', // Ờ
        0x97 => '\u{1EDE}', // Ở
        0x98 => '\u{1ECA}', // Ị
        0x99 => '\u{1ECE}', // Ỏ
        0x9A => '\u{1ECC}', // Ọ
        0x9B => '\u{1EC8}', // Ỉ
        0x9C => '\u{1EE6}', // Ủ
        0x9D => '\u{0168}', // Ũ
        0x9E => '\u{1EE4}', // Ụ
        0x9F => '\u{1EF2}', // Ỳ
        0xA0 => '\u{00D5}', // Õ
        0xA1 => '\u{1EAF}', // ắ
        0xA2 => '\u{1EB1}', // ằ
        0xA3 => '\u{1EB7}', // ặ
        0xA4 => '\u{1EA5}', // ấ
        0xA5 => '\u{1EA7}', // ầ
        0xA6 => '\u{1EA9}', // ẩ
        0xA7 => '\u{1EAD}', // ậ
        0xA8 => '\u{1EBD}', // ẽ
        0xA9 => '\u{1EB9}', // ẹ
        0xAA => '\u{1EBF}', // ế
        0xAB => '\u{1EC1}', // ề
        0xAC => '\u{1EC3}', // ể
        0xAD => '\u{1EC5}', // ễ
        0xAE => '\u{1EC7}', // ệ
        0xAF => '\u{1ED1}', // ố
        0xB0 => '\u{1ED3}', // ồ
        0xB1 => '\u{1ED5}', // ổ
        0xB2 => '\u{1ED7}', // ỗ
        0xB3 => '\u{1EE0}', // Ỡ
        0xB4 => '\u{01A0}', // Ơ
        0xB5 => '\u{1ED9}', // ộ
        0xB6 => '\u{1EDD}', // ờ
        0xB7 => '\u{1EDF}', // ở
        0xB8 => '\u{1ECB}', // ị
        0xB9 => '\u{1EF0}', // Ự
        0xBA => '\u{1EE8}', // Ứ
        0xBB => '\u{1EEA}', // Ừ
        0xBC => '\u{1EEC}', // Ử
        0xBD => '\u{01A1}', // ơ
        0xBE => '\u{1EDB}', // ớ
        0xBF => '\u{01AF}', // Ư
        0xC4 => '\u{1EA2}', // Ả
        0xC5 => '\u{0102}', // Ă
        0xC6 => '\u{1EB3}', // ẳ
        0xC7 => '\u{1EB5}', // ẵ
        0xCB => '\u{1EBA}', // Ẻ
        0xCE => '\u{0128}', // Ĩ
        0xCF => '\u{1EF3}', // ỳ
        0xD0 => 'Đ', // smethod_17 addition
        0xD1 => '\u{1EE9}', // ứ
        0xD5 => '\u{1EA1}', // ạ
        0xD6 => '\u{1EF7}', // ỷ
        0xD7 => '\u{1EEB}', // ừ
        0xD8 => '\u{1EED}', // ử
        0xDB => '\u{1EF9}', // ỹ
        0xDC => '\u{1EF5}', // ỵ
        0xDD => 'Đ', // smethod_17 addition
        0xDE => '\u{1EE1}', // ỡ
        0xDF => '\u{01B0}', // ư
        0xE4 => '\u{1EA3}', // ả
        0xE5 => '\u{0103}', // ă
        0xE6 => '\u{1EEF}', // ữ
        0xE7 => '\u{1EAB}', // ẫ
        0xEB => '\u{1EBB}', // ẻ
        0xEE => '\u{0129}', // ĩ
        0xEF => '\u{1EC9}', // ỉ
        0xF0 => '\u{0111}', // đ
        0xF1 => '\u{1EF1}', // ự
        0xF6 => '\u{1ECF}', // ỏ
        0xF7 => '\u{1ECD}', // ọ
        0xF8 => '\u{1EE5}', // ụ
        0xFB => '\u{0169}', // ũ
        0xFC => '\u{1EE7}', // ủ
        0xFE => '\u{1EE3}', // ợ
        0xFF => '\u{1EEE}', // Ữ
        b => b as char, // Latin-1 pass-through fallback
    }
}

/// Unicode→VISCII search alphabet (`@string` in smethod_17).
const SM17_UNI: &str = "áàảãạăắằẳẵặâấầẩẫậéèẻẽẹêếềểễệíìỉĩịóòỏõọôốồổỗộơớờởỡợúùủũụưứừửữựýỳỷỹỵđÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬÉÈẺẼẸÊẾỀỂỄỆÍÌỈĨỊÓÒỎÕỌÔỐỒỔỖƠỚỜỞỠÚÙỦŨỤƯỨỪỬỮỰÝỲỶỸỴĐ";

/// Unicode→VISCII encode alphabet (`str` in smethod_17) — the VISCII byte for
/// each `SM17_UNI` position, as Latin-1 chars. Same length as `SM17_UNI`.
const SM17_ENC: &str = "áàäãÕå¡¢ÆÇ£â¤¥¦ç§éèë¨©êª«¬\u{ad}®íìïî¸óòöõ÷ô¯°±²µ½¾¶·ÞþúùüûøßÑ×ØæñýÏÖÛÜðÁÀÄÃÁÅ\u{81}‚AAƒÂ„…†\u{06}‡ÉÈËˆ‰ÊŠ‹Œ\u{8d}ÊÍÌ›Î˜ÓÒ???Ô\u{8f}\u{90}‘’´•–—³ÚÙœ\u{9d}Ú¿º»¼ÿ¹ÝŸ???Ð";

/// `viscii_encode(s)` — server-authored Unicode → VISCII bytes, byte-exact
/// with the C# positional table `smethod_17` (Class5.cs:420-462). Each char's
/// position in `SM17_UNI` picks the Latin-1 / VISCII byte in `SM17_ENC`;
/// unmapped chars pass through (their low byte), and `\r`/`\n` are preserved
/// verbatim.
pub fn viscii_encode(s: &str) -> Vec<u8> {
    let uni: Vec<char> = SM17_UNI.chars().collect();
    let enc: Vec<char> = SM17_ENC.chars().collect();
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        if c == '\r' || c == '\n' {
            out.push(c as u8);
            continue;
        }
        if let Some(pos) = uni.iter().position(|&u| u == c) {
            out.push(enc[pos] as u8);
        } else {
            out.push((c as u32 & 0xFF) as u8);
        }
    }
    out
}

/// A record's wire-name garble override (Chapter 4 §4.3/§4.6).
///
/// When a mojibake name contains a codepoint > 0xFF the C# `smethod_13` emits
/// it as `AscW(ch).ToString("X2")` — a 4-digit group that the 2-byte-at-a-time
/// hex parser turns into **2 garbage bytes**, or a 3-digit group (only `ă`
/// U+0103 exists in the data) that **aborts the whole packet**. Bug-for-bug
/// replication: the in-memory name stays clean VISCII bytes while the wire
/// carries this override verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GarbleSpec {
    /// Exact hex string the C# server emits for this name on the wire.
    pub hex: String,
    /// True when the C# hex→bytes parser aborts (odd group) — the caller must
    /// drop the whole packet.
    pub abort: bool,
}

/// Compute the C# `smethod_13` wire hex for a mojibake name string.
///
/// Returns `None` for names whose codepoints are all ≤ 0xFF (the 99.9%
/// case — their wire hex equals the clean VISCII bytes). Otherwise the exact
/// hex `AscW(ch).ToString("X2")` would build, with `abort` set when any char
/// lands in 0x100..=0xFFF (a 3-digit group, e.g. the `ă` at item 48101).
pub fn compute_garble(name: &str) -> Option<GarbleSpec> {
    let mut hex = String::new();
    let mut abort = false;
    let mut has_wide = false;
    for c in name.chars() {
        let cp = c as u32;
        hex.push_str(&format!("{:02X}", cp));
        if cp > 0xFF {
            has_wide = true;
            if (0x100..=0xFFF).contains(&cp) {
                abort = true;
            }
        }
    }
    if !has_wide {
        return None;
    }
    Some(GarbleSpec { hex, abort })
}

/// Wire-name hex for a record: `None` aborts the packet; `Some` is the exact
/// hex (garble override when present, else the clean VISCII bytes).
pub fn name_wire_hex(clean: &[u8], garble: &Option<GarbleSpec>) -> Option<String> {
    match garble {
        Some(g) if g.abort => None,
        Some(g) => Some(g.hex.clone()),
        None => Some(
            clean
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<String>(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mojibake_roundtrip_simple() {
        // "D¤u Ch¤m Höi" = VISCII 44 A4 75 20 43 68 A4 6D 20 48 F6 69
        let s = "D¤u Ch¤m Höi";
        let v = to_viscii(s);
        assert_eq!(v, vec![0x44, 0xA4, 0x75, 0x20, 0x43, 0x68, 0xA4, 0x6D, 0x20, 0x48, 0xF6, 0x69]);
    }

    #[test]
    fn cp1252_punct_maps_back() {
        // „ U+201E -> byte 0x84
        assert_eq!(to_viscii("„"), vec![0x84]);
        // † U+2020 -> 0x86
        assert_eq!(to_viscii("†"), vec![0x86]);
    }

    #[test]
    fn unmappable_anh_normalized() {
        // single genuine ă U+0103 -> VISCII 0xE5
        assert_eq!(to_viscii("ă"), vec![0xE5]);
    }

    #[test]
    fn ascii_passthrough() {
        assert_eq!(to_viscii("TSVN"), b"TSVN".to_vec());
    }

    #[test]
    fn garble_two_garbage_bytes() {
        // §4.6 item 18973 "Thái „t binh pháp": „ U+201E -> "201E" -> bytes 20 1E.
        let g = compute_garble("Thái „t binh pháp").expect("garble name");
        assert!(!g.abort);
        assert_eq!(
            g.hex,
            "5468E16920201E742062696E68207068E170"
        );
        assert_eq!(
            name_wire_hex(b"", &Some(g.clone())).as_deref(),
            Some("5468E16920201E742062696E68207068E170")
        );
    }

    #[test]
    fn garble_aborts_on_three_digit_group() {
        // §4.6 item 48101 "BB Thái Văn C½ 3": ă U+0103 -> "103" (3 digits) aborts.
        let g = compute_garble("BB Thái Văn C½ 3").expect("garble name");
        assert!(g.abort);
        assert_eq!(g.hex, "4242205468E16920561036E2043BD2033");
        assert_eq!(name_wire_hex(b"", &Some(g)), None);
    }

    #[test]
    fn garble_none_for_clean_names() {
        // Item 10000 "D¤u Ch¤m Höi" — all codepoints ≤ 0xFF → no override.
        assert_eq!(compute_garble("D¤u Ch¤m Höi"), None);
        assert_eq!(
            name_wire_hex(&[0x44, 0xA4], &None).as_deref(),
            Some("44A4")
        );
    }

    #[test]
    fn viscii_display_full_table() {
        // TextEncoder.cs: 0xA4 = ấ, 0xE5 = ă, 0xF6 = ỏ, 0xD0/0xDD = Đ (smethod_17).
        assert_eq!(viscii_to_unicode(0xA4), 'ấ');
        assert_eq!(viscii_to_unicode(0xE5), 'ă');
        assert_eq!(viscii_to_unicode(0xF6), 'ỏ');
        assert_eq!(viscii_to_unicode(0xD0), 'Đ');
        assert_eq!(viscii_to_unicode(0xDD), 'Đ');
        assert_eq!(viscii_to_unicode(0x84), 'Ấ'); // VISCII 0x84 = Ấ (upper)
        assert_eq!(viscii_to_unicode(0x41), 'A');
        // Fallback = Latin-1 pass-through for bytes outside the 102-entry table.
        assert_eq!(viscii_to_unicode(0x20), ' ');
        assert_eq!(viscii_to_unicode(0xFF), 'Ữ'); // VISCII 0xFF = Ữ (upper)
        assert_eq!(viscii_to_unicode(0xE6), 'ữ'); // lower-case ữ
    }

    #[test]
    fn viscii_encode_maps_vietnamese_unicode() {
        // Đ -> 0xD0, ấ -> 0xA4 (research 03 §3.2 verified).
        assert_eq!(viscii_encode("Đ"), vec![0xD0]);
        assert_eq!(viscii_encode("ấ"), vec![0xA4]);
        // Unmappable positions collapse to '?' (0x3F) exactly like C#.
        assert_eq!(viscii_encode("Ỷ"), vec![0x3F]);
        assert_eq!(viscii_encode("Ẳ"), vec![0x41]); // -> 'A'
        // ASCII passes through unchanged; CR/LF are preserved.
        assert_eq!(viscii_encode("TSVN"), b"TSVN".to_vec());
        assert_eq!(viscii_encode("a\r\nb"), b"a\r\nb".to_vec());
        // "Th¶i gian:" = ờ is 0xB6 (Client.cs:8169 uses ¶ for the banner).
        assert_eq!(
            viscii_encode("Thời gian:"),
            vec![0x54, 0x68, 0xB6, 0x69, 0x20, 0x67, 0x69, 0x61, 0x6E, 0x3A]
        );
    }
}