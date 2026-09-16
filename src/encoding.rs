//! Text encoding contract (Chapter 4).
//!
//! Wire encoding is **VISCII 1.1** — every byte 0x00–0xFF maps to one
//! Vietnamese character. Names are held in memory as `Vec<u8>` of VISCII
//! bytes; UTF-8 mojibake (Npcs/Items) is reversed back to VISCII car-by-char.
//!
//! Garble exceptions are replicated bug-for-bug so the Rust output diffs
//! byte-exactly against captured traffic (Chapter 4 §4.3/§4.6).

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

/// Convert a single CP1252 byte to its corresponding Unicode character.
pub fn cp1252_byte_to_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}', // €
        0x82 => '\u{201A}', // ‚
        0x83 => '\u{0192}', // ƒ
        0x84 => '\u{201E}', // „
        0x85 => '\u{2026}', // …
        0x86 => '\u{2020}', // †
        0x87 => '\u{2021}', // ‡
        0x88 => '\u{02C6}', // ˆ
        0x89 => '\u{2030}', // ‰
        0x8A => '\u{0160}', // Š
        0x8B => '\u{2039}', // ‹
        0x8C => '\u{0152}', // Œ
        0x8E => '\u{017D}', // Ž
        0x91 => '\u{2018}', // ‘
        0x92 => '\u{2019}', // ’
        0x93 => '\u{201C}', // “
        0x94 => '\u{201D}', // ”
        0x95 => '\u{2022}', // •
        0x96 => '\u{2013}', // –
        0x97 => '\u{2014}', // —
        0x98 => '\u{02DC}', // ˜
        0x99 => '\u{2122}', // ™
        0x9A => '\u{0161}', // š
        0x9B => '\u{203A}', // ›
        0x9C => '\u{0153}', // œ
        0x9E => '\u{017E}', // ž
        0x9F => '\u{0178}', // Ÿ
        _ => b as char,
    }
}

/// Decode raw CP1252 bytes to a Unicode string.
pub fn cp1252_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| cp1252_byte_to_char(b)).collect()
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

/// The VISCII byte→Unicode table used for display: the 102-entry table
/// imported character-for-character, plus the `0xD0→Đ`, `0xDD→Đ` additions.
/// Bytes outside the table fall back to Latin-1 pass-through (`(char)byte`),
/// matching the original decoder.
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
        0xD0 => 'Đ',        // extended-table addition
        0xD1 => '\u{1EE9}', // ứ
        0xD5 => '\u{1EA1}', // ạ
        0xD6 => '\u{1EF7}', // ỷ
        0xD7 => '\u{1EEB}', // ừ
        0xD8 => '\u{1EED}', // ử
        0xDB => '\u{1EF9}', // ỹ
        0xDC => '\u{1EF5}', // ỵ
        0xDD => '\u{00DD}', // Ý (RFC 1456 standard; replaces invalid duplicate Đ)
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
        b => b as char,     // Latin-1 pass-through fallback
    }
}

/// Decode a byte slice of VISCII 1.1 encoded characters into a Unicode UTF-8 String.
pub fn viscii_decode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| viscii_to_unicode(b)).collect()
}

/// Convert a single Unicode character to its VISCII 1.1 byte representation (RFC 1456).
/// Returns `Some(byte)` if the character exists in VISCII, or `None` if unmappable.
pub fn unicode_to_viscii(c: char) -> Option<u8> {
    match c {
        // Standard printable ASCII and C0 control characters (except the 6 C0 slots redefined by VISCII)
        c if (c as u32) <= 0x7F && !matches!(c as u8, 0x02 | 0x05 | 0x06 | 0x14 | 0x19 | 0x1E) => {
            Some(c as u8)
        }

        // 6 uppercase vowels placed in C0 control range by VISCII RFC 1456
        '\u{1EB2}' => Some(0x02), // Ẳ
        '\u{1EB4}' => Some(0x05), // Ẵ
        '\u{1EAA}' => Some(0x06), // Ẫ
        '\u{1EF6}' => Some(0x14), // Ỷ
        '\u{1EF8}' => Some(0x19), // Ỹ
        '\u{1EF4}' => Some(0x1E), // Ỵ

        // VISCII Extended Range (0x80..=0xFF)
        '\u{1EA0}' => Some(0x80), // Ạ
        '\u{1EAE}' => Some(0x81), // Ắ
        '\u{1EB0}' => Some(0x82), // Ằ
        '\u{1EB6}' => Some(0x83), // Ặ
        '\u{1EA4}' => Some(0x84), // Ấ
        '\u{1EA6}' => Some(0x85), // Ầ
        '\u{1EA8}' => Some(0x86), // Ẩ
        '\u{1EAC}' => Some(0x87), // Ậ
        '\u{1EBC}' => Some(0x88), // Ẽ
        '\u{1EB8}' => Some(0x89), // Ẹ
        '\u{1EBE}' => Some(0x8A), // Ế
        '\u{1EC0}' => Some(0x8B), // Ề
        '\u{1EC2}' => Some(0x8C), // Ể
        '\u{1EC4}' => Some(0x8D), // Ễ
        '\u{1EC6}' => Some(0x8E), // Ệ
        '\u{1ED0}' => Some(0x8F), // Ố
        '\u{1ED2}' => Some(0x90), // Ồ
        '\u{1ED4}' => Some(0x91), // Ổ
        '\u{1ED6}' => Some(0x92), // Ỗ
        '\u{1ED8}' => Some(0x93), // Ộ
        '\u{1EE2}' => Some(0x94), // Ợ
        '\u{1EDA}' => Some(0x95), // Ớ
        '\u{1EDC}' => Some(0x96), // Ờ
        '\u{1EDE}' => Some(0x97), // Ở
        '\u{1ECA}' => Some(0x98), // Ị
        '\u{1ECE}' => Some(0x99), // Ỏ
        '\u{1ECC}' => Some(0x9A), // Ọ
        '\u{1EC8}' => Some(0x9B), // Ỉ
        '\u{1EE6}' => Some(0x9C), // Ủ
        '\u{0168}' => Some(0x9D), // Ũ
        '\u{1EE4}' => Some(0x9E), // Ụ
        '\u{1EF2}' => Some(0x9F), // Ỳ
        '\u{00D5}' => Some(0xA0), // Õ
        '\u{1EAF}' => Some(0xA1), // ắ
        '\u{1EB1}' => Some(0xA2), // ằ
        '\u{1EB7}' => Some(0xA3), // ặ
        '\u{1EA5}' => Some(0xA4), // ấ
        '\u{1EA7}' => Some(0xA5), // ầ
        '\u{1EA9}' => Some(0xA6), // ẩ
        '\u{1EAD}' => Some(0xA7), // ậ
        '\u{1EBD}' => Some(0xA8), // ẽ
        '\u{1EB9}' => Some(0xA9), // ẹ
        '\u{1EBF}' => Some(0xAA), // ế
        '\u{1EC1}' => Some(0xAB), // ề
        '\u{1EC3}' => Some(0xAC), // ể
        '\u{1EC5}' => Some(0xAD), // ễ
        '\u{1EC7}' => Some(0xAE), // ệ
        '\u{1ED1}' => Some(0xAF), // ố
        '\u{1ED3}' => Some(0xB0), // ồ
        '\u{1ED5}' => Some(0xB1), // ổ
        '\u{1ED7}' => Some(0xB2), // ỗ
        '\u{1EE0}' => Some(0xB3), // Ỡ
        '\u{01A0}' => Some(0xB4), // Ơ
        '\u{1ED9}' => Some(0xB5), // ộ
        '\u{1EDD}' => Some(0xB6), // ờ
        '\u{1EDF}' => Some(0xB7), // ở
        '\u{1ECB}' => Some(0xB8), // ị
        '\u{1EF0}' => Some(0xB9), // Ự
        '\u{1EE8}' => Some(0xBA), // Ứ
        '\u{1EEA}' => Some(0xBB), // Ừ
        '\u{1EEC}' => Some(0xBC), // Ử
        '\u{01A1}' => Some(0xBD), // ơ
        '\u{1EDB}' => Some(0xBE), // ớ
        '\u{01AF}' => Some(0xBF), // Ư
        '\u{00C0}' => Some(0xC0), // À
        '\u{00C1}' => Some(0xC1), // Á
        '\u{00C2}' => Some(0xC2), // Â
        '\u{00C3}' => Some(0xC3), // Ã
        '\u{1EA2}' => Some(0xC4), // Ả
        '\u{0102}' => Some(0xC5), // Ă
        '\u{1EB3}' => Some(0xC6), // ẳ
        '\u{1EB5}' => Some(0xC7), // ẵ
        '\u{00C8}' => Some(0xC8), // È
        '\u{00C9}' => Some(0xC9), // É
        '\u{00CA}' => Some(0xCA), // Ê
        '\u{1EBA}' => Some(0xCB), // Ẻ
        '\u{00CC}' => Some(0xCC), // Ì
        '\u{00CD}' => Some(0xCD), // Í
        '\u{0128}' => Some(0xCE), // Ĩ
        '\u{1EF3}' => Some(0xCF), // ỳ
        '\u{0110}' | '\u{00D0}' => Some(0xD0), // Đ (or Eth Ð)
        '\u{1EE9}' => Some(0xD1), // ứ
        '\u{00D2}' => Some(0xD2), // Ò
        '\u{00D3}' => Some(0xD3), // Ó
        '\u{00D4}' => Some(0xD4), // Ô
        '\u{1EA1}' => Some(0xD5), // ạ
        '\u{1EF7}' => Some(0xD6), // ỷ
        '\u{1EEB}' => Some(0xD7), // ừ
        '\u{1EED}' => Some(0xD8), // ử
        '\u{00D9}' => Some(0xD9), // Ù
        '\u{00DA}' => Some(0xDA), // Ú
        '\u{1EF9}' => Some(0xDB), // ỹ
        '\u{1EF5}' => Some(0xDC), // ỵ
        '\u{00DD}' => Some(0xDD), // Ý
        '\u{1EE1}' => Some(0xDE), // ỡ
        '\u{01B0}' => Some(0xDF), // ư
        '\u{00E0}' => Some(0xE0), // à
        '\u{00E1}' => Some(0xE1), // á
        '\u{00E2}' => Some(0xE2), // â
        '\u{00E3}' => Some(0xE3), // ã
        '\u{1EA3}' => Some(0xE4), // ả
        '\u{0103}' => Some(0xE5), // ă
        '\u{1EEF}' => Some(0xE6), // ữ
        '\u{1EAB}' => Some(0xE7), // ẫ
        '\u{00E8}' => Some(0xE8), // è
        '\u{00E9}' => Some(0xE9), // é
        '\u{00EA}' => Some(0xEA), // ê
        '\u{1EBB}' => Some(0xEB), // ẻ
        '\u{00EC}' => Some(0xEC), // ì
        '\u{00ED}' => Some(0xED), // í
        '\u{0129}' => Some(0xEE), // ĩ
        '\u{1EC9}' => Some(0xEF), // ỉ
        '\u{0111}' | '\u{00F0}' => Some(0xF0), // đ (or eth ð)
        '\u{1EF1}' => Some(0xF1), // ự
        '\u{00F2}' => Some(0xF2), // ò
        '\u{00F3}' => Some(0xF3), // ó
        '\u{00F4}' => Some(0xF4), // ô
        '\u{00F5}' => Some(0xF5), // õ
        '\u{1ECF}' => Some(0xF6), // ỏ
        '\u{1ECD}' => Some(0xF7), // ọ
        '\u{1EE5}' => Some(0xF8), // ụ
        '\u{00F9}' => Some(0xF9), // ù
        '\u{00FA}' => Some(0xFA), // ú
        '\u{0169}' => Some(0xFB), // ũ
        '\u{1EE7}' => Some(0xFC), // ủ
        '\u{00FD}' => Some(0xFD), // ý
        '\u{1EE3}' => Some(0xFE), // ợ
        '\u{1EEE}' => Some(0xFF), // Ữ

        _ => None,
    }
}

/// Encode a Unicode UTF-8 string into VISCII 1.1 single-byte characters (RFC 1456).
///
/// Unmapped characters pass through their low byte (`(c as u32 & 0xFF) as u8`)
/// if ≤ 0xFF, or '?' fallback if out of 8-bit range, ensuring zero panics and
/// 100% round-trip fidelity for all 134 Vietnamese accented characters.
/// Compose a base Latin/vowel character with a Vietnamese combining diacritic into NFC.
pub fn compose_vietnamese_char(base: char, mark: char) -> Option<char> {
    match (base, mark) {
        ('A', '\u{0300}') => Some('À'),
        ('E', '\u{0300}') => Some('È'),
        ('I', '\u{0300}') => Some('Ì'),
        ('O', '\u{0300}') => Some('Ò'),
        ('U', '\u{0300}') => Some('Ù'),
        ('Y', '\u{0300}') => Some('Ỳ'),
        ('a', '\u{0300}') => Some('à'),
        ('e', '\u{0300}') => Some('è'),
        ('i', '\u{0300}') => Some('ì'),
        ('o', '\u{0300}') => Some('ò'),
        ('u', '\u{0300}') => Some('ù'),
        ('y', '\u{0300}') => Some('ỳ'),
        ('Â', '\u{0300}') => Some('Ầ'),
        ('Ê', '\u{0300}') => Some('Ề'),
        ('Ô', '\u{0300}') => Some('Ồ'),
        ('â', '\u{0300}') => Some('ầ'),
        ('ê', '\u{0300}') => Some('ề'),
        ('ô', '\u{0300}') => Some('ồ'),
        ('Ă', '\u{0300}') => Some('Ằ'),
        ('ă', '\u{0300}') => Some('ằ'),
        ('Ơ', '\u{0300}') => Some('Ờ'),
        ('ơ', '\u{0300}') => Some('ờ'),
        ('Ư', '\u{0300}') => Some('Ừ'),
        ('ư', '\u{0300}') => Some('ừ'),
        ('A', '\u{0301}') => Some('Á'),
        ('E', '\u{0301}') => Some('É'),
        ('I', '\u{0301}') => Some('Í'),
        ('O', '\u{0301}') => Some('Ó'),
        ('U', '\u{0301}') => Some('Ú'),
        ('Y', '\u{0301}') => Some('Ý'),
        ('a', '\u{0301}') => Some('á'),
        ('e', '\u{0301}') => Some('é'),
        ('i', '\u{0301}') => Some('í'),
        ('o', '\u{0301}') => Some('ó'),
        ('u', '\u{0301}') => Some('ú'),
        ('y', '\u{0301}') => Some('ý'),
        ('Â', '\u{0301}') => Some('Ấ'),
        ('Ê', '\u{0301}') => Some('Ế'),
        ('Ô', '\u{0301}') => Some('Ố'),
        ('â', '\u{0301}') => Some('ấ'),
        ('ê', '\u{0301}') => Some('ế'),
        ('ô', '\u{0301}') => Some('ố'),
        ('Ă', '\u{0301}') => Some('Ắ'),
        ('ă', '\u{0301}') => Some('ắ'),
        ('Ơ', '\u{0301}') => Some('Ớ'),
        ('ơ', '\u{0301}') => Some('ớ'),
        ('Ư', '\u{0301}') => Some('Ứ'),
        ('ư', '\u{0301}') => Some('ứ'),
        ('A', '\u{0302}') => Some('Â'),
        ('E', '\u{0302}') => Some('Ê'),
        ('O', '\u{0302}') => Some('Ô'),
        ('a', '\u{0302}') => Some('â'),
        ('e', '\u{0302}') => Some('ê'),
        ('o', '\u{0302}') => Some('ô'),
        ('Ạ', '\u{0302}') => Some('Ậ'),
        ('ạ', '\u{0302}') => Some('ậ'),
        ('Ẹ', '\u{0302}') => Some('Ệ'),
        ('ẹ', '\u{0302}') => Some('ệ'),
        ('Ọ', '\u{0302}') => Some('Ộ'),
        ('ọ', '\u{0302}') => Some('ộ'),
        ('A', '\u{0303}') => Some('Ã'),
        ('E', '\u{0303}') => Some('Ẽ'),
        ('I', '\u{0303}') => Some('Ĩ'),
        ('O', '\u{0303}') => Some('Õ'),
        ('U', '\u{0303}') => Some('Ũ'),
        ('Y', '\u{0303}') => Some('Ỹ'),
        ('a', '\u{0303}') => Some('ã'),
        ('e', '\u{0303}') => Some('ẽ'),
        ('i', '\u{0303}') => Some('ĩ'),
        ('o', '\u{0303}') => Some('õ'),
        ('u', '\u{0303}') => Some('ũ'),
        ('y', '\u{0303}') => Some('ỹ'),
        ('Â', '\u{0303}') => Some('Ẫ'),
        ('Ê', '\u{0303}') => Some('Ễ'),
        ('Ô', '\u{0303}') => Some('Ỗ'),
        ('â', '\u{0303}') => Some('ẫ'),
        ('ê', '\u{0303}') => Some('ễ'),
        ('ô', '\u{0303}') => Some('ỗ'),
        ('Ă', '\u{0303}') => Some('Ẵ'),
        ('ă', '\u{0303}') => Some('ẵ'),
        ('Ơ', '\u{0303}') => Some('Ỡ'),
        ('ơ', '\u{0303}') => Some('ỡ'),
        ('Ư', '\u{0303}') => Some('Ữ'),
        ('ư', '\u{0303}') => Some('ữ'),
        ('A', '\u{0306}') => Some('Ă'),
        ('a', '\u{0306}') => Some('ă'),
        ('Ạ', '\u{0306}') => Some('Ặ'),
        ('ạ', '\u{0306}') => Some('ặ'),
        ('A', '\u{0309}') => Some('Ả'),
        ('E', '\u{0309}') => Some('Ẻ'),
        ('I', '\u{0309}') => Some('Ỉ'),
        ('O', '\u{0309}') => Some('Ỏ'),
        ('U', '\u{0309}') => Some('Ủ'),
        ('Y', '\u{0309}') => Some('Ỷ'),
        ('a', '\u{0309}') => Some('ả'),
        ('e', '\u{0309}') => Some('ẻ'),
        ('i', '\u{0309}') => Some('ỉ'),
        ('o', '\u{0309}') => Some('ỏ'),
        ('u', '\u{0309}') => Some('ủ'),
        ('y', '\u{0309}') => Some('ỷ'),
        ('Â', '\u{0309}') => Some('Ẩ'),
        ('Ê', '\u{0309}') => Some('Ể'),
        ('Ô', '\u{0309}') => Some('Ổ'),
        ('â', '\u{0309}') => Some('ẩ'),
        ('ê', '\u{0309}') => Some('ể'),
        ('ô', '\u{0309}') => Some('ổ'),
        ('Ă', '\u{0309}') => Some('Ẳ'),
        ('ă', '\u{0309}') => Some('ẳ'),
        ('Ơ', '\u{0309}') => Some('Ở'),
        ('ơ', '\u{0309}') => Some('ở'),
        ('Ư', '\u{0309}') => Some('Ử'),
        ('ư', '\u{0309}') => Some('ử'),
        ('O', '\u{031B}') => Some('Ơ'),
        ('U', '\u{031B}') => Some('Ư'),
        ('o', '\u{031B}') => Some('ơ'),
        ('u', '\u{031B}') => Some('ư'),
        ('A', '\u{0323}') => Some('Ạ'),
        ('E', '\u{0323}') => Some('Ẹ'),
        ('I', '\u{0323}') => Some('Ị'),
        ('O', '\u{0323}') => Some('Ọ'),
        ('U', '\u{0323}') => Some('Ụ'),
        ('Y', '\u{0323}') => Some('Ỵ'),
        ('a', '\u{0323}') => Some('ạ'),
        ('e', '\u{0323}') => Some('ẹ'),
        ('i', '\u{0323}') => Some('ị'),
        ('o', '\u{0323}') => Some('ọ'),
        ('u', '\u{0323}') => Some('ụ'),
        ('y', '\u{0323}') => Some('ỵ'),
        ('Ơ', '\u{0323}') => Some('Ợ'),
        ('ơ', '\u{0323}') => Some('ợ'),
        ('Ư', '\u{0323}') => Some('Ự'),
        ('ư', '\u{0323}') => Some('ự'),
        ('É', '\u{0302}') => Some('Ế'),
        ('È', '\u{0302}') => Some('Ề'),
        ('Ẻ', '\u{0302}') => Some('Ể'),
        ('Ẽ', '\u{0302}') => Some('Ễ'),
        ('é', '\u{0302}') => Some('ế'),
        ('è', '\u{0302}') => Some('ề'),
        ('ẻ', '\u{0302}') => Some('ể'),
        ('ẽ', '\u{0302}') => Some('ễ'),
        ('Á', '\u{0302}') => Some('Ấ'),
        ('À', '\u{0302}') => Some('Ầ'),
        ('Ả', '\u{0302}') => Some('Ẩ'),
        ('Ã', '\u{0302}') => Some('Ẫ'),
        ('á', '\u{0302}') => Some('ấ'),
        ('à', '\u{0302}') => Some('ầ'),
        ('ả', '\u{0302}') => Some('ẩ'),
        ('ã', '\u{0302}') => Some('ẫ'),
        ('Á', '\u{0306}') => Some('Ắ'),
        ('À', '\u{0306}') => Some('Ằ'),
        ('Ả', '\u{0306}') => Some('Ẳ'),
        ('Ã', '\u{0306}') => Some('Ẵ'),
        ('á', '\u{0306}') => Some('ắ'),
        ('à', '\u{0306}') => Some('ằ'),
        ('ả', '\u{0306}') => Some('ẳ'),
        ('ã', '\u{0306}') => Some('ẵ'),
        ('Ó', '\u{0302}') => Some('Ố'),
        ('Ò', '\u{0302}') => Some('Ồ'),
        ('Ỏ', '\u{0302}') => Some('Ổ'),
        ('Õ', '\u{0302}') => Some('Ỗ'),
        ('ó', '\u{0302}') => Some('ố'),
        ('ò', '\u{0302}') => Some('ồ'),
        ('ỏ', '\u{0302}') => Some('ổ'),
        ('õ', '\u{0302}') => Some('ỗ'),
        ('Ó', '\u{031B}') => Some('Ớ'),
        ('Ò', '\u{031B}') => Some('Ờ'),
        ('Ỏ', '\u{031B}') => Some('Ở'),
        ('Õ', '\u{031B}') => Some('Ỡ'),
        ('Ọ', '\u{031B}') => Some('Ợ'),
        ('ó', '\u{031B}') => Some('ớ'),
        ('ò', '\u{031B}') => Some('ờ'),
        ('ỏ', '\u{031B}') => Some('ở'),
        ('õ', '\u{031B}') => Some('ỡ'),
        ('ọ', '\u{031B}') => Some('ợ'),
        ('Ú', '\u{031B}') => Some('Ứ'),
        ('Ù', '\u{031B}') => Some('Ừ'),
        ('Ủ', '\u{031B}') => Some('Ử'),
        ('Ũ', '\u{031B}') => Some('Ữ'),
        ('Ụ', '\u{031B}') => Some('Ự'),
        ('ú', '\u{031B}') => Some('ứ'),
        ('ù', '\u{031B}') => Some('ừ'),
        ('ủ', '\u{031B}') => Some('ử'),
        ('ũ', '\u{031B}') => Some('ữ'),
        ('ụ', '\u{031B}') => Some('ự'),
        _ => None,
    }
}

/// Normalize decomposed Unicode (NFD) combining diacritics into precomposed NFC characters.
pub fn normalize_vietnamese_nfc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            if let Some(prev) = out.pop() {
                if let Some(composed) = compose_vietnamese_char(prev, c) {
                    out.push(composed);
                } else {
                    out.push(prev);
                    out.push(c);
                }
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn viscii_encode(s: &str) -> Vec<u8> {
    let normalized = normalize_vietnamese_nfc(s);
    let mut out = Vec::with_capacity(normalized.len());
    for c in normalized.chars() {
        if let Some(b) = unicode_to_viscii(c) {
            out.push(b);
        } else if (c as u32) <= 0xFF {
            out.push(c as u8);
        } else {
            out.push(b'?');
        }
    }
    out
}

/// A record's wire-name garble override (Chapter 4 §4.3/§4.6).
///
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GarbleSpec {
    /// Exact hex string emitted for this name on the wire.
    pub hex: String,
    /// True when the hex→bytes parser aborts (odd group) — the caller must
    /// drop the whole packet.
    pub abort: bool,
}

/// Compute the wire hex for a mojibake name string (wide chars emit 4-digit
/// groups, min 2 digits otherwise).
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
