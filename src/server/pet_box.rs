//! Pet roster rules: slot (`stt`) assignment — active 1..4, stable 5..10 — and
//! ownership invariants (one entry per pet id).

use crate::server::session::PetState;

/// Active (fight) pet slots.
pub const ACTIVE_SLOTS: std::ops::RangeInclusive<u8> = 1..=4;
/// Stable (stored) pet slots — scan bound `5..=10`.
pub const STABLE_SLOTS: std::ops::RangeInclusive<u8> = 5..=10;

/// The next free active slot, or `None` when all four are taken.
pub fn next_active_slot(pets: &[PetState]) -> Option<u8> {
    let used: Vec<u8> = pets.iter().map(|p| p.stt).collect();
    let mut slots = ACTIVE_SLOTS;
    slots.find(|s| !used.contains(s))
}

/// The next free stable slot, or `None` when the stable is full.
pub fn next_stable_slot(pets: &[PetState]) -> Option<u8> {
    let used: Vec<u8> = pets.iter().map(|p| p.stt).collect();
    let mut slots = STABLE_SLOTS;
    slots.find(|s| !used.contains(s))
}

/// Add a newly caught pet. Returns the assigned `stt`, or `None` when the id is
/// already owned or the roster is full.
pub fn add_caught(pets: &mut Vec<PetState>, npc_id: u16, hp_max: u16) -> Option<u8> {
    if pets.iter().any(|p| p.id == npc_id) {
        return None;
    }
    let stt = next_active_slot(pets)?;
    pets.push(PetState {
        stt,
        id: npc_id,
        level: 1,
        thuoctinh: 1,
        hp: hp_max,
        hp_max,
        ..Default::default()
    });
    Some(stt)
}
