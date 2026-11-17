//! Ticket 01 — Trade active pet auto-recall.
//!
//! When a player trades away their currently active pet (`active_pet_stt == a_stt`),
//! the settlement resets `active_pet_stt = 0` so the player is never left pointing
//! at an empty or recycled pet slot.

use ts_dream::server::session::{PetState, Session};
use ts_dream::server::trade_system::pet_trade_settle;

#[test]
fn pet_trade_settle_resets_active_pet_stt_when_active_pet_traded() {
    let mut a = Session::default();
    a.id = 1001;
    a.active_pet_stt = 1;
    a.pets.push(PetState {
        stt: 1,
        id: 11001,
        level: 30,
        ..Default::default()
    });
    a.trade.pets.push(1);

    let mut b = Session::default();
    b.id = 1002;
    b.active_pet_stt = 0;

    let res = pet_trade_settle(&mut a, &mut b);
    assert!(res.is_ok(), "settlement succeeds: {:?}", res.err());
    assert_eq!(a.active_pet_stt, 0, "seller active_pet_stt must be reset to 0");
    assert_eq!(a.pets.len(), 0, "seller pet list no longer has the pet");
    assert_eq!(b.pets.len(), 1, "buyer receives the pet");
}

#[test]
fn pet_trade_settle_preserves_active_pet_stt_when_inactive_pet_traded() {
    let mut a = Session::default();
    a.id = 1001;
    a.active_pet_stt = 2; // Pet 2 is active, trading pet 1
    a.pets.push(PetState {
        stt: 1,
        id: 11001,
        level: 30,
        ..Default::default()
    });
    a.pets.push(PetState {
        stt: 2,
        id: 11002,
        level: 40,
        ..Default::default()
    });
    a.trade.pets.push(1);

    let mut b = Session::default();
    b.id = 1002;
    b.active_pet_stt = 0;

    let res = pet_trade_settle(&mut a, &mut b);
    assert!(res.is_ok());
    assert_eq!(a.active_pet_stt, 2, "seller active_pet_stt preserved when trading inactive pet");
}
