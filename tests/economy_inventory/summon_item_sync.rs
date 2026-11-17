//! Ticket 02 — Summon item instant roster sync frames.
//!
//! When a player uses an item with `add_pet > 10000`, the server must add the pet,
//! consume the item, and emit both `0F 07` (pet data) and `0F 01` (pet presence) frames
//! so the client UI updates immediately without relogging.

use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Item;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::inventory::handle_inventory;
use ts_dream::server::session::{Conn, InventoryItem};

#[tokio::test]
async fn summon_item_emits_0f07_and_0f01_frames() {
    let mut conn = Conn::default();
    conn.session.id = 5001;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 45001,
        count: 1,
        ..Default::default()
    });

    let mut data = GameData::default();
    data.items.insert(
        45001,
        Item {
            id: 45001,
            add_pet: 12005, // Grants pet 12005
            ..Default::default()
        },
    );

    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let payload = [0x01, 0x01, 0x00]; // slot 1, count 1, use_type 0 (player)

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env: ServerEnv::none(),
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 15,
        payload: &payload,
    };

    handle_inventory(&mut ctx).await;

    // 1. Pet added to session
    assert_eq!(conn.session.pets.len(), 1, "pet was added to session");
    assert_eq!(conn.session.pets[0].id, 12005);
    assert_eq!(conn.session.pets[0].stt, 1);

    // 2. Item consumed from homdo
    let item_in_slot = conn.session.homdo.iter().find(|i| i.slot == 1);
    assert!(
        item_in_slot.is_none() || item_in_slot.unwrap().id == 0,
        "item was consumed"
    );

    // 3. Outgoing frames contain both 0F07 and 0F01
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    let has_0f07 = frames.iter().any(|f| f.contains("0F07"));
    let has_0f01 = frames.iter().any(|f| f.contains("0F01"));

    assert!(has_0f07, "client must receive 0F 07 pet data frame; got: {:?}", frames);
    assert!(has_0f01, "client must receive 0F 01 pet presence frame; got: {:?}", frames);
}

#[tokio::test]
async fn summon_item_when_roster_full_rejects_and_preserves_item() {
    let mut conn = Conn::default();
    conn.session.id = 5002;
    // Roster is full: 4 pets
    for stt in 1..=4 {
        conn.session.pets.push(ts_dream::server::session::PetState {
            stt,
            id: 10000 + stt as u16,
            name: format!("Pet{}", stt).into_bytes(),
            ..Default::default()
        });
    }

    // Homdo has summon item at slot 1 and another item at slot 2
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 45001,
        count: 1,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 9999,
        count: 5,
        ..Default::default()
    });

    let mut data = GameData::default();
    data.items.insert(
        45001,
        Item {
            id: 45001,
            add_pet: 12005,
            ..Default::default()
        },
    );

    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let payload = [0x01, 0x01, 0x00]; // slot 1, count 1

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env: ServerEnv::none(),
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 15,
        payload: &payload,
    };

    handle_inventory(&mut ctx).await;

    // 1. Pet roster remains 4 (no new pet added)
    assert_eq!(conn.session.pets.len(), 4, "roster must not exceed 4 pets");
    assert!(!conn.session.pets.iter().any(|p| p.id == 12005));

    // 2. Summon item in slot 1 is NOT consumed
    let item_in_slot1 = conn.session.homdo.iter().find(|i| i.slot == 1);
    assert!(item_in_slot1.is_some());
    assert_eq!(item_in_slot1.unwrap().count, 1, "summon item count must not decrease");

    // 3. Adjacent item in slot 2 is unaffected
    let item_in_slot2 = conn.session.homdo.iter().find(|i| i.slot == 2);
    assert!(item_in_slot2.is_some());
    assert_eq!(item_in_slot2.unwrap().count, 5);

    // 4. Outgoing frames do NOT contain 0F07 or 0F01
    let frames: Vec<String> = out.outgoing.into_iter().map(|f| f.frame).collect();
    assert!(!frames.iter().any(|f| f.contains("0F07")));
    assert!(!frames.iter().any(|f| f.contains("0F01")));
}

#[tokio::test]
async fn summon_item_updates_session_registry() {
    let mut conn = Conn::default();
    conn.session.id = 5003;
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 45001,
        count: 1,
        ..Default::default()
    });

    let mut data = GameData::default();
    data.items.insert(
        45001,
        Item {
            id: 45001,
            add_pet: 12005,
            ..Default::default()
        },
    );

    // Register session in online registry
    ts_dream::server::session::lock_online_sessions().insert(5003, conn.session.clone());

    let service = ts_dream::battle::service::BattleService::default();
    let mut out = HandleOutcome::default();
    let payload = [0x01, 0x01, 0x00];

    let mut ctx = OpcodeCtx {
        conn: &mut conn,
        out: &mut out,
        env: ServerEnv::none(),
        data: &data,
        service: &service,
        decoded: &[],
        opcode: 0x17,
        sub: 15,
        payload: &payload,
    };

    handle_inventory(&mut ctx).await;

    // Registry must have been updated with the new pet via registry_update_if_online
    let reg = ts_dream::server::session::lock_online_sessions();
    let online_sess = reg.get(&5003).expect("session must exist in registry");
    assert_eq!(online_sess.pets.len(), 1, "registry must reflect new pet");
    assert_eq!(online_sess.pets[0].id, 12005);
    drop(reg);
    ts_dream::server::session::lock_online_sessions().remove(&5003);
}
