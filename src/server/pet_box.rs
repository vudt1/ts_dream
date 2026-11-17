//! Pet roster rules: slot (`stt`) assignment — active 1..4, stable 5..10 — and
//! ownership invariants (one entry per pet id).

use crate::data::tables::Npc;
use crate::db::modern::model::PetStorageType;
use crate::server::session::PetState;

/// Active (fight) pet slots.
pub const ACTIVE_SLOTS: std::ops::RangeInclusive<u8> = 1..=4;
/// Stable (stored) pet slots — scan bound `5..=10`.
pub const STABLE_SLOTS: std::ops::RangeInclusive<u8> = 5..=10;

/// The next free active slot, or `None` when all four are taken.
pub fn next_active_slot(pets: &[PetState]) -> Option<u8> {
    let used: Vec<u8> = pets
        .iter()
        .filter(|p| p.is_carried())
        .map(|p| p.stt)
        .collect();
    let mut slots = ACTIVE_SLOTS;
    slots.find(|s| !used.contains(s))
}

/// The next free stable slot, or `None` when the stable is full.
pub fn next_stable_slot(pets: &[PetState]) -> Option<u8> {
    let used: Vec<u8> = pets
        .iter()
        .filter(|p| p.effective_storage_type() == PetStorageType::Hotel)
        .map(|p| p.stt)
        .collect();
    let mut slots = STABLE_SLOTS;
    slots.find(|s| !used.contains(s))
}

/// Add a newly caught pet from its NPC template. Returns the assigned `stt`, or `None` when
/// the id is already owned or the roster is full.
pub fn add_caught(
    pets: &mut Vec<PetState>,
    npc: Option<&Npc>,
    npc_id: u16,
) -> Option<u8> {
    if pets.iter().any(|p| p.id == npc_id) {
        return None;
    }
    let stt = next_active_slot(pets)?;

    let pet = if let Some(n) = npc {
        let mut skills = [(0u16, 0u8); 4];
        for (i, &sk) in n.skill.iter().enumerate().take(4) {
            skills[i] = (sk as u16, if sk > 0 { 1 } else { 0 });
        }
        PetState {
            stt,
            id: npc_id,
            name: n.name.clone(),
            level: 1, // Theo quyết định thiết kế đã chốt: người chơi tự luyện từ cấp 1
            thuoctinh: n.thuoctinh as u8,
            hp: n.hp as u16,
            hp_max: n.hp as u16,
            sp: n.sp as u16,
            sp_max: n.sp as u16,
            int1: n.int1 as u16,
            atk: n.atk as u16,
            def: n.def as u16,
            agi: n.agi as u16,
            hpx: n.hpx as u16,
            spx: n.spx as u16,
            fai: 60, // Điểm trung thành mặc định
            skills,
            ..Default::default()
        }
    } else {
        PetState {
            stt,
            id: npc_id,
            level: 1,
            thuoctinh: 1,
            hp: 100,
            hp_max: 100,
            sp: 100,
            sp_max: 100,
            fai: 60,
            ..Default::default()
        }
    };

    pets.push(pet);
    Some(stt)
}
