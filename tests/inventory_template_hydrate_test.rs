use std::collections::HashMap;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Item;
use ts_dream::server::session::{InventoryItem, Session};

#[test]
fn test_hydrate_templates_preserves_instance_fields() {
    let mut data = GameData::default();
    let mut items = HashMap::new();
    items.insert(
        10001,
        Item {
            id: 10001,
            name: b"Dragon Saber".to_vec(),
            level: 45,
            int1: 5,
            atk1: 55,
            def1: 10,
            hpx1: 100,
            spx1: 50,
            agi1: 15,
            fai1: 0,
            int2: 2,
            atk2: 12,
            def2: 4,
            hpx2: 20,
            spx2: 10,
            agi2: 3,
            fai2: 0,
            hp: 200,
            sp: 100,
            loai: 1, // weapon
            thuoctinh: 2, // fire
            value: 10,
            ..Default::default()
        },
    );
    data.items = items;

    let mut session = Session::default();
    session.homdo.push(InventoryItem {
        slot: 1,
        id: 10001,
        count: 1,
        doben: 85,
        long_val: 5,
        giatri_long: 30,
        khang: 8,
        texp: 999,
        // template fields unpopulated
        lv: 0,
        atk1: 0,
        loai: 0,
        thuoctinh: 0,
        ..Default::default()
    });

    session.hydrate_templates(&data);

    let hydrated = &session.homdo[0];
    // Template fields populated
    assert_eq!(hydrated.lv, 45);
    assert_eq!(hydrated.atk1, 55);
    assert_eq!(hydrated.def1, 10);
    assert_eq!(hydrated.loai, 1);
    assert_eq!(hydrated.thuoctinh, 2);
    assert_eq!(hydrated.giatri_thuoctinh, 10);
    assert_eq!(hydrated.item_hp, 200);

    // Instance fields strictly preserved
    assert_eq!(hydrated.slot, 1);
    assert_eq!(hydrated.id, 10001);
    assert_eq!(hydrated.count, 1);
    assert_eq!(hydrated.doben, 85);
    assert_eq!(hydrated.long_val, 5);
    assert_eq!(hydrated.giatri_long, 30);
    assert_eq!(hydrated.khang, 8);
    assert_eq!(hydrated.texp, 999);
}

#[test]
fn test_hydrate_templates_across_all_containers() {
    let mut data = GameData::default();
    let mut items = HashMap::new();
    items.insert(
        20001,
        Item {
            id: 20001,
            name: b"Iron Armor".to_vec(),
            level: 25,
            def1: 30,
            loai: 2,
            ..Default::default()
        },
    );
    data.items = items;

    let mut session = Session::default();
    session.tientrang.push(InventoryItem {
        slot: 1,
        id: 20001,
        count: 1,
        doben: 50,
        ..Default::default()
    });
    session.trangbi.push(InventoryItem {
        slot: 2,
        id: 20001,
        count: 1,
        doben: 60,
        ..Default::default()
    });

    session.hydrate_templates(&data);

    assert_eq!(session.tientrang[0].lv, 25);
    assert_eq!(session.tientrang[0].def1, 30);
    assert_eq!(session.tientrang[0].doben, 50);

    assert_eq!(session.trangbi[0].lv, 25);
    assert_eq!(session.trangbi[0].def1, 30);
    assert_eq!(session.trangbi[0].doben, 60);
}
