//! Warps.txt talk-warp gate tests — production `GameData::load` wires
//! `data.warps` (the only text asset in production boot; consumer:
//! `quest.rs` talk-warp lookup keyed `(map_id, warpid)`).

use std::path::PathBuf;
use ts_dream::data::loader::GameData;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

#[test]
fn warps_wired_into_production_boot() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    assert!(
        !data.warps.is_empty(),
        "Warps.txt must wire data.warps in production boot"
    );
    // Sample round-trip pair from the source file header.
    let w = &data.warps[&(49901, 2)];
    assert_eq!((w.map2, w.x, w.y), (49902, 522, 495));
    let back = &data.warps[&(49902, 1)];
    assert_eq!((back.map2, back.x, back.y), (49901, 222, 295));
}

#[test]
fn battle_gates_wired_into_production_boot() {
    let data = GameData::load(&data_dir()).expect("GameData::load");
    assert!(
        !data.battle_gates.is_empty(),
        "BattleGate.txt must wire data.battle_gates in production boot"
    );
    // Sample from the source file header: map 60011 warp 2, diahinh 171.
    let gate = &data.battle_gates[&(60011, 2)];
    assert_eq!(gate.diahinh, 171);
    assert_eq!(gate.defenders[0], 27061);
}
