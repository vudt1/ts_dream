use ts_dream::db::modern::model::{EntryLayout, StorageType};
use ts_dream::server::inventory::{biased_giatri_long, free_slot_in, GIA_TRI_LONG_BIAS, GIA_TRI_LONG_MAX};
use ts_dream::server::session::InventoryItem;

#[test]
fn test_storage_type_metadata() {
    assert_eq!(StorageType::Bag.capacity(), 25);
    assert_eq!(StorageType::Bag.table_name(), "homdo");
    assert_eq!(StorageType::Bag.dump_code(), "1705");
    assert_eq!(StorageType::Bag.entry_layout(), EntryLayout::SlotLed12);

    assert_eq!(StorageType::Bank.capacity(), 50);
    assert_eq!(StorageType::Bank.table_name(), "tientrang");
    assert_eq!(StorageType::Bank.dump_code(), "1E01");
    assert_eq!(StorageType::Bank.entry_layout(), EntryLayout::SlotLed12);

    assert_eq!(StorageType::Secondary.capacity(), 25);
    assert_eq!(StorageType::Secondary.table_name(), "tuideo");
    assert_eq!(StorageType::Secondary.dump_code(), "172F");
    assert_eq!(StorageType::Secondary.entry_layout(), EntryLayout::SlotLed12);

    assert_eq!(StorageType::Equip.capacity(), 6);
    assert_eq!(StorageType::Equip.table_name(), "trangbi");
    assert_eq!(StorageType::Equip.dump_code(), "170B");
    assert_eq!(StorageType::Equip.entry_layout(), EntryLayout::Slotless10);

    assert_eq!(StorageType::Warehouse.capacity(), 10);
    assert_eq!(StorageType::Warehouse.table_name(), "luulang");
    assert_eq!(StorageType::Warehouse.dump_code(), "1766");
    assert_eq!(StorageType::Warehouse.entry_layout(), EntryLayout::SlotLed12);

    assert_eq!(StorageType::from_table_name("homdo"), Some(StorageType::Bag));
    assert_eq!(StorageType::from_table_name("tientrang"), Some(StorageType::Bank));
    assert_eq!(StorageType::from_table_name("tuideo"), Some(StorageType::Secondary));
    assert_eq!(StorageType::from_table_name("trangbi"), Some(StorageType::Equip));
    assert_eq!(StorageType::from_table_name("luulang"), Some(StorageType::Warehouse));
    assert_eq!(StorageType::from_table_name("unknown"), None);
}

#[test]
fn test_free_slot_in() {
    let mut items = vec![
        InventoryItem { slot: 1, id: 100, ..Default::default() },
        InventoryItem { slot: 2, id: 101, ..Default::default() },
    ];
    assert_eq!(free_slot_in(&items, 5), Some(3));

    // Fill capacity
    items.push(InventoryItem { slot: 3, id: 102, ..Default::default() });
    items.push(InventoryItem { slot: 4, id: 103, ..Default::default() });
    items.push(InventoryItem { slot: 5, id: 104, ..Default::default() });
    assert_eq!(free_slot_in(&items, 5), None);

    // Free slot in the middle
    items.remove(1); // remove slot 2
    assert_eq!(free_slot_in(&items, 5), Some(2));
}

#[test]
fn test_biased_giatri_long() {
    assert_eq!(GIA_TRI_LONG_BIAS, 100);
    assert_eq!(GIA_TRI_LONG_MAX, 155);
    assert_eq!(biased_giatri_long(0), 100);
    assert_eq!(biased_giatri_long(20), 120);
    assert_eq!(biased_giatri_long(55), 155);
    assert_eq!(biased_giatri_long(80), 180);
    assert_eq!(biased_giatri_long(155), 255);
    // Values above max must clamp safely to 255 without debug_assert panic
    assert_eq!(biased_giatri_long(156), 255);
    assert_eq!(biased_giatri_long(255), 255);
}

#[test]
fn test_dump_trangbi_10b_layout_and_pet_gear_isolation() {
    let mut session = ts_dream::server::session::Session::default();

    // Player gear: slot 3 (armor) and slot 1 (helm), inserted out-of-order
    session.trangbi.push(InventoryItem {
        slot: 3,
        id: 11003,
        count: 1,
        ..Default::default()
    });
    session.trangbi.push(InventoryItem {
        slot: 1,
        id: 11001,
        count: 1,
        ..Default::default()
    });

    // Pet gear: slot 11 (pet 1 weapon), slot 21 (pet 2 weapon)
    session.trangbi.push(InventoryItem {
        slot: 11,
        id: 22001,
        count: 1,
        ..Default::default()
    });
    session.trangbi.push(InventoryItem {
        slot: 21,
        id: 22002,
        count: 1,
        ..Default::default()
    });

    // 1. equipped_ids must only contain player gear (1..=6)
    let equipped = session.equipped_ids();
    assert_eq!(equipped.len(), 2, "equipped_ids must only return player equipment 1..=6");
    assert!(equipped.contains(&11001));
    assert!(equipped.contains(&11003));
    assert!(!equipped.contains(&22001), "pet equipment must NOT be in player equipped_ids");
    assert!(!equipped.contains(&22002), "pet equipment must NOT be in player equipped_ids");

    // 2. dump_trangbi must only encode 2 player records, sorted slot 1 then 3, 10 bytes per record
    let frame_hex = session.dump_trangbi();
    assert!(frame_hex.starts_with("F444"));
    // Frame layout: F444 [len: 2B LE] [code: 2B "170B"] [body: 10B * 2 = 20B]
    let code = &frame_hex[8..12];
    assert_eq!(code, "170B");
    let body = &frame_hex[12..];
    let body_bytes = hex::decode(body).expect("valid hex body");
    assert_eq!(body_bytes.len(), 20, "2 items * 10 bytes = 20 bytes");
    assert_eq!(body_bytes.len() % 10, 0, "17 0B body length must be a multiple of 10");

    // First record: item 11001 (slot 1)
    let item1_id = u16::from_le_bytes([body_bytes[0], body_bytes[1]]);
    assert_eq!(item1_id, 11001, "first record must be slot 1");

    // Second record: item 11003 (slot 3)
    let item2_id = u16::from_le_bytes([body_bytes[10], body_bytes[11]]);
    assert_eq!(item2_id, 11003, "second record must be slot 3");
}

#[test]
fn test_logined_sequence_clear_tientrang_and_luulang_flag() {
    let session = ts_dream::server::session::Session::default();

    // When session has no luulang items (empty)
    let seq_default = ts_dream::server::spawn::build_logined_sequence_session_ext(&session);
    let pos_clear = seq_default
        .iter()
        .position(|f| f == &ts_dream::server::spawn::clear_tientrang_frame())
        .expect("must contain 1E 08 clear frame");
    let pos_bank = seq_default
        .iter()
        .position(|f| f == &session.dump_tientrang())
        .expect("must contain 1E 01 bank frame");
    assert_eq!(pos_clear + 1, pos_bank, "1E 08 must be sent immediately before 1E 01");
    assert!(
        seq_default.iter().all(|f| !f.starts_with("F44402001766") && !f.starts_with("F44403001766")),
        "17 66 must not be present when luulang has no items"
    );

    // When session has luulang item with id == 0 (empty slot)
    let mut session_empty_slot = session.clone();
    session_empty_slot.luulang.push(ts_dream::server::session::InventoryItem {
        id: 0,
        slot: 1,
        ..Default::default()
    });
    let seq_empty_slot = ts_dream::server::spawn::build_logined_sequence_session_ext(&session_empty_slot);
    assert!(
        seq_empty_slot.iter().all(|f| !f.starts_with("F44402001766") && !f.starts_with("F44403001766")),
        "17 66 must not be present when luulang only contains empty slot (id == 0)"
    );

    // When session has luulang items (id > 0)
    let mut session_with_luulang = session.clone();
    session_with_luulang.luulang.push(ts_dream::server::session::InventoryItem {
        id: 11001,
        count: 1,
        slot: 1,
        ..Default::default()
    });
    let seq_with_luulang = ts_dream::server::spawn::build_logined_sequence_session_ext(&session_with_luulang);
    assert!(
        seq_with_luulang.iter().any(|f| f == &session_with_luulang.dump_luulang()),
        "17 66 must be present when luulang has items with id > 0"
    );
    assert_eq!(seq_with_luulang.len(), seq_default.len() + 1);

    // Verify build_logined_sequence_session delegates directly to build_logined_sequence_session_ext
    assert_eq!(
        ts_dream::server::spawn::build_logined_sequence_session(&session),
        seq_default
    );
    assert_eq!(
        ts_dream::server::spawn::build_logined_sequence_session(&session_with_luulang),
        seq_with_luulang
    );
}

#[test]
fn test_send_logined_sequence_luulang_behavior() {
    use ts_dream::server::dispatcher::HandleOutcome;
    use ts_dream::server::handlers::login::send_logined_sequence;
    use ts_dream::server::session::Conn;

    // 1. Without luulang items (22 frames including unconditional 1D 04 bank sync)
    let mut conn = Conn::default();
    let mut out = HandleOutcome::default();
    send_logined_sequence(&mut conn, &mut out);
    assert!(conn.session.logined);
    assert_eq!(out.outgoing.len(), 22);
    assert!(out.outgoing.iter().any(|f| f.frame == "F44406001D0400000000"));
    assert!(out.outgoing.iter().all(|f| !f.frame.starts_with("F44402001766") && !f.frame.starts_with("F44403001766")));

    // 2. With luulang items (id > 0) (23 frames)
    let mut conn_with_items = Conn::default();
    conn_with_items.session.luulang.push(ts_dream::server::session::InventoryItem {
        id: 11001,
        count: 1,
        slot: 1,
        ..Default::default()
    });
    let mut out_with_items = HandleOutcome::default();
    send_logined_sequence(&mut conn_with_items, &mut out_with_items);
    assert!(conn_with_items.session.logined);
    assert_eq!(out_with_items.outgoing.len(), 23);
    assert!(out_with_items.outgoing.iter().any(|f| f.frame == "F44406001D0400000000"));
    assert!(out_with_items.outgoing.iter().any(|f| f.frame == conn_with_items.session.dump_luulang()));
}

