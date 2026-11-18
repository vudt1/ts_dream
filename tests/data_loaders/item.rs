//! Item.dat binary loader tests (370-byte records, server-only decode).

use std::collections::HashMap;
use std::path::PathBuf;
use ts_dream::data::loaders::ItemDatLoader;
use ts_dream::data::tables::Item;
use ts_dream::encoding;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

fn enc8(val: u8) -> u8 {
    val.wrapping_add(9) ^ 0x9A
}

fn enc16(val: u16) -> [u8; 2] {
    (val.wrapping_add(9) ^ 0xEFC3).to_le_bytes()
}

fn enc32(val: u32) -> [u8; 4] {
    (val.wrapping_add(9) ^ 0x0B80_F4B4).to_le_bytes()
}

fn enc32s(val: i32) -> [u8; 4] {
    ((val as u32).wrapping_add(109) ^ 0x0B80_F4B4).to_le_bytes()
}

#[derive(Default)]
struct SyntheticItemSpec {
    id: u16,
    name_viscii: Vec<u8>,
    kind: u8,
    attr1_kind: u16,
    attr1_value: i32,
    attr2_kind: u16,
    attr2_value: i32,
    compound_material: u8,
    compound_rank: u8,
    fit_type: u8,
    level: u8,
    price: u32,
    sell_price: u32,
    element: u8,
    element_value: i32,
    spare3: u16,
    suit_id: u16,
    role_count_index: u16,
    client_garbage_fill: Option<u8>,
}

fn encode_record(spec: &SyntheticItemSpec) -> [u8; ItemDatLoader::FIELD_LENGTH] {
    let mut rec = [0u8; ItemDatLoader::FIELD_LENGTH];
    if let Some(fill) = spec.client_garbage_fill {
        // Fill client-only ranges with garbage to prove server loader ignores them.
        rec[24..32].fill(fill);
        rec[49..83].fill(fill);
        rec[92..98].fill(fill);
        rec[111..370].fill(fill);
    }

    let name_len = spec.name_viscii.len().min(20);
    rec[0] = name_len as u8;
    if name_len > 0 {
        let mut rev = spec.name_viscii[..name_len].to_vec();
        rev.reverse();
        let start = 1 + 20 - name_len;
        rec[start..start + name_len].copy_from_slice(&rev);
    }

    rec[21] = enc8(spec.kind);
    rec[22..24].copy_from_slice(&enc16(spec.id));
    rec[32..34].copy_from_slice(&enc16(spec.attr1_kind));
    rec[34..36].copy_from_slice(&enc16(spec.attr2_kind));
    rec[38..42].copy_from_slice(&enc32s(spec.attr1_value));
    rec[42..46].copy_from_slice(&enc32s(spec.attr2_value));
    rec[46] = enc8(spec.compound_material);
    rec[47] = enc8(spec.compound_rank);
    rec[48] = enc8(spec.fit_type);
    rec[83] = enc8(spec.level);
    rec[84..88].copy_from_slice(&enc32(spec.price));
    rec[88..92].copy_from_slice(&enc32(spec.sell_price));
    rec[98] = enc8(spec.element);
    rec[99..103].copy_from_slice(&enc32s(spec.element_value));
    rec[103..105].copy_from_slice(&enc16(spec.spare3));
    rec[106..108].copy_from_slice(&enc16(spec.suit_id));
    rec[109..111].copy_from_slice(&enc16(spec.role_count_index));
    rec
}

#[test]
fn item_dat_rejects_invalid_size_and_accepts_empty() {
    let empty: HashMap<i64, Item> = ItemDatLoader::load(&[]).expect("empty slice must succeed");
    assert!(empty.is_empty());

    for bad_len in [1usize, 369, 371, 741] {
        let bytes = vec![0u8; bad_len];
        let err = ItemDatLoader::load(&bytes).expect_err("non-multiple of 370 must fail");
        assert!(
            err.to_string().contains("370"),
            "error must mention 370-byte record size: {err}"
        );
    }
}

#[test]
fn item_dat_skips_record_zero_and_zero_id_and_ignores_client_columns() {
    let mut bytes = Vec::new();

    // Record 0: even with a valid non-zero id, record 0 must always be skipped.
    bytes.extend_from_slice(&encode_record(&SyntheticItemSpec {
        id: 9999,
        name_viscii: b"DummyZero".to_vec(),
        price: 123,
        ..Default::default()
    }));

    // Record 1: id == 0 must be skipped.
    bytes.extend_from_slice(&encode_record(&SyntheticItemSpec {
        id: 0,
        name_viscii: b"ZeroId".to_vec(),
        price: 456,
        ..Default::default()
    }));

    // Record 2: valid item with garbage in all client-only regions (including desc len 0xFF).
    let viscii_name = encoding::to_viscii("Kiếm Sắt");
    bytes.extend_from_slice(&encode_record(&SyntheticItemSpec {
        id: 10534,
        name_viscii: viscii_name.clone(),
        kind: 39,
        attr1_kind: 210, // atk1
        attr1_value: 25,
        attr2_kind: 26, // sp
        attr2_value: 40,
        compound_material: 7,
        compound_rank: 4,
        fit_type: 3,
        level: 110,
        price: 58_800,
        sell_price: 14_700,
        element: 2,
        element_value: -5,
        spare3: 11001,
        suit_id: 11002,
        role_count_index: 12003,
        client_garbage_fill: Some(0xFF),
    }));

    let items: HashMap<i64, Item> = ItemDatLoader::load(&bytes).expect("parse synthetic Item.dat");
    assert_eq!(items.len(), 1);
    assert!(!items.contains_key(&9999), "record 0 must be skipped");
    assert!(!items.contains_key(&0), "id == 0 must be skipped");

    let item = items.get(&10534).expect("item 10534 must be loaded");
    assert_eq!(item.id, 10534);
    assert_eq!(item.name, viscii_name);
    assert_eq!(item.utf8_name(), "Kiếm Sắt");
    assert_eq!(encoding::viscii_encode(&item.utf8_name()), viscii_name);
    assert_eq!(item.loai, 39);
    assert_eq!(item.equip_pos, 3);
    assert_eq!(item.level, 110);
    assert_eq!(item.atk1, 25);
    assert_eq!(item.sp, 40);
    assert_eq!(item.compound_material, 7);
    assert_eq!(item.compound_rank, 4);
    assert_eq!(item.price, 58_800);
    assert_eq!(item.sell_price, 14_700);
    assert_eq!(item.thuoctinh, 2);
    assert_eq!(item.value, -5);
    assert_eq!(item.rb_pet_from, 11001);
    assert_eq!(item.rb_pet_to, 11002);
    assert_eq!(item.add_pet, 12003);
}

#[test]
fn item_dat_maps_all_server_attr_kinds_and_preserves_hp_sp_first_slot_priority() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&encode_record(&SyntheticItemSpec::default())); // record 0

    // Item 1: attr1=hp(25), attr2=hp(207) -> hp stays 100 (attr2 only writes if hp == 0)
    bytes.extend_from_slice(&encode_record(&SyntheticItemSpec {
        id: 20001,
        attr1_kind: 25,
        attr1_value: 100,
        attr2_kind: 207,
        attr2_value: 999,
        ..Default::default()
    }));

    // Item 2: attr1=fai1(64), attr2=int2(212)
    bytes.extend_from_slice(&encode_record(&SyntheticItemSpec {
        id: 20002,
        attr1_kind: 64,
        attr1_value: 12,
        attr2_kind: 212,
        attr2_value: 34,
        ..Default::default()
    }));

    let items = ItemDatLoader::load(&bytes).expect("load attrs");
    let i1 = &items[&20001];
    assert_eq!(i1.hp, 100);
    let i2 = &items[&20002];
    assert_eq!(i2.fai1, 12);
    assert_eq!(i2.int2, 34);
}

#[test]
fn item_dat_loads_real_data_with_prices_and_on_demand_utf8_name() {
    let bytes = std::fs::read(data_dir().join("Item.dat")).expect("read Data/Item.dat");
    assert_eq!(bytes.len() % ItemDatLoader::FIELD_LENGTH, 0);

    let items = ItemDatLoader::load(&bytes).expect("parse Data/Item.dat");
    assert!(
        items.len() > 8000,
        "expected >8000 items in Item.dat, got {}",
        items.len()
    );

    // Pickaxe 10534: kind=39, equip_pos=3
    let pickaxe = items.get(&10534).expect("10534 must exist");
    assert_eq!(pickaxe.loai, 39);
    assert_eq!(pickaxe.equip_pos, 3);

    // Non-zero prices exist in Item.dat
    assert!(
        items.values().any(|i| i.price > 0 && i.sell_price > 0),
        "Item.dat must populate price and sell_price"
    );

    // On-demand UTF-8 name decodes VISCII 1.1 bytes cleanly and round-trips via viscii_encode.
    let named = items.values().find(|i| !i.name.is_empty()).expect("named item");
    assert_eq!(named.utf8_name(), encoding::viscii_decode(&named.name));
    assert_eq!(encoding::viscii_encode(&named.utf8_name()), named.name);

    // Shared GameData::load populates GameData.items directly with matching count and prices.
    let shared = crate::common::shared_game_data();
    assert_eq!(shared.items.len(), items.len());
    assert_eq!(shared.items[&10534].price, pickaxe.price);
    assert_eq!(shared.items[&10534].sell_price, pickaxe.sell_price);
}
