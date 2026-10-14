use ts_dream::server::session::{InventoryItem, Session};

#[test]
fn test_recompute_stats_applies_gear_bonus() {
    let mut session = Session {
        level: 10,
        int1: 20,
        atk: 30,
        def: 25,
        agi: 15,
        hpx: 10,
        spx: 10,
        ..Default::default()
    };
    session.recompute_stats();
    let base_hp_max = session.hp_max;
    let base_sp_max = session.sp_max;

    // Equip weapon in slot 1 with stats
    session.trangbi.push(InventoryItem {
        slot: 1, // weapon
        id: 1001,
        count: 1,
        atk1: 25,
        atk2: 5,
        int1: 10,
        int2: 2,
        hpx1: 50,
        spx1: 20,
        ..Default::default()
    });

    // Equip armor in slot 2
    session.trangbi.push(InventoryItem {
        slot: 2, // armor
        id: 1002,
        count: 1,
        def1: 15,
        def2: 4,
        ..Default::default()
    });

    // Add pet equipment in slot 11 (pet 1 weapon). Must NOT affect player!
    session.trangbi.push(InventoryItem {
        slot: 11,
        id: 1003,
        count: 1,
        atk1: 999,
        atk2: 999,
        ..Default::default()
    });

    session.recompute_stats();

    assert_eq!(session.atk2, 30);
    assert_eq!(session.def2, 19);
    assert_eq!(session.int2, 12);
    assert!(session.hp_max > base_hp_max);
    assert!(session.sp_max > base_sp_max);
}
