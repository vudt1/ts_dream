use ts_dream::encoding::{unicode_to_viscii, viscii_decode, viscii_encode, viscii_to_unicode};

#[test]
fn test_viscii_roundtrip_all_bytes() {
    for b in 0u8..=255 {
        let ch = viscii_to_unicode(b);
        let mapped_b = unicode_to_viscii(ch);
        assert_eq!(
            mapped_b,
            Some(b),
            "Byte 0x{:02X} mapped to Unicode char {:?} (U+{:04X}) did not round-trip",
            b,
            ch,
            ch as u32
        );
    }
}

#[test]
fn test_viscii_c0_upper_vowels() {
    // VISCII places 6 least-used uppercase vowels into C0 control slots (RFC 1456 §3)
    let c0_vowels = [
        (0x02, '\u{1EB2}', "Ẳ"),
        (0x05, '\u{1EB4}', "Ẵ"),
        (0x06, '\u{1EAA}', "Ẫ"),
        (0x14, '\u{1EF6}', "Ỷ"),
        (0x19, '\u{1EF8}', "Ỹ"),
        (0x1E, '\u{1EF4}', "Ỵ"),
    ];

    for (b, ch, label) in c0_vowels {
        assert_eq!(viscii_to_unicode(b), ch, "Mismatch on {label}");
        assert_eq!(unicode_to_viscii(ch), Some(b), "Mismatch encode on {label}");
    }
}

#[test]
fn test_viscii_y_and_d_corrections() {
    // 0xD0 is Đ
    assert_eq!(viscii_to_unicode(0xD0), 'Đ');
    assert_eq!(unicode_to_viscii('Đ'), Some(0xD0));

    // 0xDD is Ý (RFC 1456 Table 1 column Dx row xD = Y')
    assert_eq!(viscii_to_unicode(0xDD), 'Ý');
    assert_eq!(unicode_to_viscii('Ý'), Some(0xDD));

    // 0xF0 is đ
    assert_eq!(viscii_to_unicode(0xF0), 'đ');
    assert_eq!(unicode_to_viscii('đ'), Some(0xF0));
}

#[test]
fn test_vietnamese_names_encode_decode() {
    let test_names = [
        "Nguyễn Văn Ý",
        "Vũ Thị Mộng Mơ",
        "Trần Quốc Toản",
        "Lê Quý Đôn",
        "Phan Bội Châu",
        "Hồ Chí Minh",
        "Đinh Bộ Lĩnh",
        "Ỷ Lan",
        "Ngô Quyền",
        "Lý Thường Kiệt",
        "Quang Trung",
        "Bà Triệu",
        "Hai Bà Trưng",
    ];

    for &name in &test_names {
        let encoded = viscii_encode(name);
        let decoded = viscii_decode(&encoded);
        assert_eq!(
            decoded, name,
            "Failed round-trip for Vietnamese name: {name}"
        );
        // Ensure none of the characters were replaced with '?'
        assert!(
            !encoded.contains(&b'?'),
            "Encoded name '{name}' contains '?'"
        );
    }
}

#[test]
fn test_all_134_vietnamese_accented_characters() {
    let all_vn = concat!(
        "AÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬ",
        "aáàảãạăắằẳẵặâấầẩẫậ",
        "EÉÈẺẼẸÊẾỀỂỄỆ",
        "eéèẻẽẹêếềểễệ",
        "IÍÌỈĨỊ",
        "iíìỉĩị",
        "OÓÒỎÕỌÔỐỒỔỖỘƠỚỜỞỠỢ",
        "oóòỏõọôốồổỗộơớờởỡợ",
        "UÚÙỦŨỤƯỨỪỬỮỰ",
        "uúùủũụưứừửữự",
        "YÝỲỶỸỴ",
        "yýỳỷỹỵ",
        "Đđ"
    );

    for ch in all_vn.chars() {
        let opt_byte = unicode_to_viscii(ch);
        assert!(
            opt_byte.is_some(),
            "Character {ch} (U+{:04X}) missing from unicode_to_viscii",
            ch as u32
        );
        let b = opt_byte.unwrap();
        let decoded_ch = viscii_to_unicode(b);
        assert_eq!(
            decoded_ch, ch,
            "Round-trip failed for character {ch} (0x{b:02X} -> {decoded_ch})"
        );
    }
}

#[test]
fn test_whitespace_and_ascii_preservation() {
    let text = "Hello, World! 12345 @ # $ % ^ & * ( ) _ + - = [ ] { } ; ' : \" , . / < > ? \r\n\t";
    let encoded = viscii_encode(text);
    let decoded = viscii_decode(&encoded);
    assert_eq!(decoded, text);
}

#[test]
fn test_nfd_decomposed_vietnamese_normalization_and_encoding() {
    // Decomposed Vietnamese strings with combining diacritics
    // "Việt Nam": 'V', 'i', 'e', '\u{0323}' (dot below), '\u{0302}' (circumflex), 't', ' ', 'N', 'a', 'm'
    let nfd_viet_nam = "Vie\u{0323}\u{0302}t Nam";
    let nfc_viet_nam = "Việt Nam";

    let enc_nfd = viscii_encode(nfd_viet_nam);
    let enc_nfc = viscii_encode(nfc_viet_nam);
    assert_eq!(
        enc_nfd, enc_nfc,
        "NFD 'Việt Nam' must encode identically to NFC"
    );
    assert!(!enc_nfd.contains(&b'?'), "Must not contain '?' replacement byte");
    assert_eq!(viscii_decode(&enc_nfd), nfc_viet_nam);

    // "Nguyễn Văn Ý": 'N', 'g', 'u', 'y', 'e', '\u{0303}', '\u{0302}', 'n' ...
    let nfd_nguyen = "Nguye\u{0303}\u{0302}n Va\u{0306}n Y\u{0301}";
    let nfc_nguyen = "Nguyễn Văn Ý";
    let enc_nfd_nguyen = viscii_encode(nfd_nguyen);
    let enc_nfc_nguyen = viscii_encode(nfc_nguyen);
    assert_eq!(enc_nfd_nguyen, enc_nfc_nguyen);
    assert!(!enc_nfd_nguyen.contains(&b'?'));
    assert_eq!(viscii_decode(&enc_nfd_nguyen), nfc_nguyen);

    // "Đinh Bộ Lĩnh": 'D', '\u{0335}' (or Đ directly), 'B', 'o', '\u{0323}', '\u{0302}', ' ', 'L', 'i', '\u{0303}', 'n', 'h'
    let nfd_dinh = "Đinh Bo\u{0323}\u{0302} Li\u{0303}nh";
    let nfc_dinh = "Đinh Bộ Lĩnh";
    assert_eq!(viscii_encode(nfd_dinh), viscii_encode(nfc_dinh));

    // "Ỷ Lan": 'Y', '\u{0309}', ' ', 'L', 'a', 'n'
    let nfd_y_lan = "Y\u{0309} Lan";
    let nfc_y_lan = "Ỷ Lan";
    assert_eq!(viscii_encode(nfd_y_lan), viscii_encode(nfc_y_lan));
}
