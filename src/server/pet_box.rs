//! Pet roster rules: slot (`stt`) assignment — active 1..4, stable 5..8 — and
//! ownership invariants (one entry per pet id).

use crate::server::session::PetState;

/// Active (fight) pet slots.
pub const ACTIVE_SLOTS: std::ops::RangeInclusive<u8> = 1..=4;
/// Stable (stored) pet slots.
pub const STABLE_SLOTS: std::ops::RangeInclusive<u8> = 5..=8;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn pet(stt: u8, id: u16) -> PetState {
        PetState {
            stt,
            id,
            ..Default::default()
        }
    }

    #[test]
    fn add_caught_assigns_next_active_slot() {
        let mut pets = vec![pet(1, 1001)];
        assert_eq!(add_caught(&mut pets, 1002, 30), Some(2));
        assert_eq!(pets[1].stt, 2);
        assert_eq!(pets[1].level, 1);
    }

    #[test]
    fn add_caught_rejects_duplicate_id() {
        let mut pets = vec![pet(1, 1001)];
        assert_eq!(add_caught(&mut pets, 1001, 30), None);
        assert_eq!(pets.len(), 1);
    }

    #[test]
    fn slots_respect_ranges() {
        let pets: Vec<PetState> = (1..=4).map(|s| pet(s, 2000 + u16::from(s))).collect();
        assert_eq!(next_active_slot(&pets), None);
        assert_eq!(next_stable_slot(&pets), Some(5));
    }
}
