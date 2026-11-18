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

#[test]
fn test_pickaxe_item_preserves_kind_39_and_equip_pos_3() {
    use ts_dream::data::tables::Item;

    let pickaxe = Item {
        id: 10534,
        loai: 39,
        equip_pos: 3,
        level: 110,
        atk1: 25,
        ..Default::default()
    };
    let tool = Item {
        id: 29335,
        loai: 27,
        equip_pos: 0,
        level: 0,
        ..Default::default()
    };

    let mut data = GameData::default();
    data.items.insert(10534, pickaxe);
    data.items.insert(29335, tool);

    let mut session = Session::default();
    session.homdo.push(InventoryItem {
        slot: 1,
        id: 10534,
        count: 1,
        ..Default::default()
    });
    session.homdo.push(InventoryItem {
        slot: 2,
        id: 29335,
        count: 1,
        ..Default::default()
    });
    // Manual legacy item without equip_pos set (only loai = 2)
    session.trangbi.push(InventoryItem {
        slot: 2,
        id: 19737,
        count: 1,
        loai: 2,
        ..Default::default()
    });

    session.hydrate_templates(&data);

    let pickaxe = &session.homdo[0];
    assert_eq!(pickaxe.loai, 39, "kind=39 must be preserved for Compound/Shop/Trade");
    assert_eq!(pickaxe.equip_pos, 3, "fit_type=3 must map to equip_pos=3");
    assert_eq!(pickaxe.effective_equip_pos(), 3);
    assert_eq!(pickaxe.atk1, 25);

    let tool = &session.homdo[1];
    assert_eq!(tool.equip_pos, 0);
    assert_eq!(tool.effective_equip_pos(), 0, "non-equippable item must have effective_equip_pos=0");

    let legacy_armor = &session.trangbi[0];
    assert_eq!(legacy_armor.effective_equip_pos(), 2, "legacy in-memory item with loai=2 must fallback to equip_pos=2");
}

#[test]
fn test_real_item_dat_pickaxes_and_tools_equip_pos() {
    use std::path::Path;
    use ts_dream::data::loaders::item::ItemDatLoader;

    let path = Path::new("Data/Item.dat");
    assert!(path.exists(), "Data/Item.dat must exist");
    let bytes = std::fs::read(path).expect("read Item.dat");
    let items = ItemDatLoader::load(&bytes).expect("load Item.dat");

    let mut data = GameData::default();
    data.items = items;

    let mut session = Session::default();
    let pickaxe_ids: [u16; 6] = [10001, 10530, 10531, 10532, 10533, 10534];
    for (idx, &id) in pickaxe_ids.iter().enumerate() {
        let item = data
            .items
            .get(&i64::from(id))
            .unwrap_or_else(|| panic!("pickaxe {id} missing from Item.dat"));
        assert_eq!(item.loai, 39, "pickaxe {id} kind in Item.dat");
        assert_eq!(item.equip_pos, 3, "pickaxe {id} fit_type in Item.dat");
        session.homdo.push(InventoryItem {
            slot: (idx + 1) as u8,
            id,
            count: 1,
            ..Default::default()
        });
    }

    // 29335 (Cuốc đào) & 31007 (Vé đoái hoán tâm) must be non-equippable (fit_type = 0)
    let tool = data.items.get(&29335).expect("29335 missing from Item.dat");
    assert_eq!(tool.equip_pos, 0, "29335 must have fit_type=0");
    let ticket = data.items.get(&31007).expect("31007 missing from Item.dat");
    assert_eq!(ticket.equip_pos, 0, "31007 must have fit_type=0");

    session.homdo.push(InventoryItem { slot: 7, id: 29335, count: 1, ..Default::default() });
    session.homdo.push(InventoryItem { slot: 8, id: 31007, count: 1, ..Default::default() });
    session.hydrate_templates(&data);

    for idx in 0..pickaxe_ids.len() {
        let item = &session.homdo[idx];
        assert_eq!(item.loai, 39, "hydrated pickaxe {} must preserve loai=39", item.id);
        assert_eq!(item.equip_pos, 3, "hydrated pickaxe {} must have equip_pos=3", item.id);
        assert_eq!(item.effective_equip_pos(), 3);
    }
    assert_eq!(session.homdo[6].effective_equip_pos(), 0, "29335 must not be equippable");
    assert_eq!(session.homdo[7].effective_equip_pos(), 0, "31007 must not be equippable");
}
