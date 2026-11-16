use ts_dream::server::session::{HotkeySlot, PetState, Session};
use ts_dream::server::handlers::pet_actions::find_free;

#[test]
fn test_dump_hotkeys_kind1_and_kind2_wire_encoding() {
    let mut session = Session::default();
    session.hotkeys[1] = HotkeySlot::item(1234);
    session.hotkeys[2] = HotkeySlot::skill(5678);

    let frame = session.dump_hotkeys();
    // Frame format: F444 + len + 2801 + entries
    // entries: "01" + "D204" + "01" + "02" + "2E16" + "02"
    assert!(frame.starts_with("F444"));
    assert!(frame.contains("2801"));
    assert!(frame.contains("01D20401"));
    assert!(frame.contains("022E1602"));
}

#[test]
fn test_dump_hotkeys_empty() {
    let session = Session::default();
    let frame = session.dump_hotkeys();
    assert_eq!(frame, "F4440300280102");
}

#[test]
fn test_pet_actions_find_free_slot() {
    let pets = vec![
        PetState { stt: 1, id: 101, ..Default::default() },
        PetState { stt: 2, id: 102, ..Default::default() },
        PetState { stt: 4, id: 104, ..Default::default() },
    ];
    // Roster 1..=4 has slot 3 free
    assert_eq!(find_free(&pets, 1, 4), Some(3));

    // Stable 5..=8 has slot 5 free
    assert_eq!(find_free(&pets, 5, 8), Some(5));

    let full_roster = vec![
        PetState { stt: 1, id: 101, ..Default::default() },
        PetState { stt: 2, id: 102, ..Default::default() },
        PetState { stt: 3, id: 103, ..Default::default() },
        PetState { stt: 4, id: 104, ..Default::default() },
    ];
    assert_eq!(find_free(&full_roster, 1, 4), None);
}
