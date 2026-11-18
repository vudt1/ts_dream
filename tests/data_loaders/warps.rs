//! Warps.txt & BattleGate.txt production boot tests (ADR 0010):
//! - `Warps.txt` is compacted to 31 rows (4 overrides + 27 fallbacks missing `rt=2` in `eve.emg`).
//! - `BattleGate.txt` is compacted to 10 rows missing `rt=3` in `eve.emg`.

#[test]
fn warps_wired_into_production_boot() {
    let data = crate::common::shared_game_data();
    assert_eq!(
        data.warps.len(),
        31,
        "Compacted Warps.txt must load exactly 31 entries (4 overrides + 27 fallbacks)"
    );

    // 4 explicit overrides
    let fix = &data.warps[&(25303, 2)];
    assert_eq!((fix.map2, fix.x, fix.y), (25304, 170, 790));
    assert_eq!((data.warps[&(59401, 2)].map2, data.warps[&(59401, 2)].x, data.warps[&(59401, 2)].y), (12003, 500, 500));
    assert_eq!((data.warps[&(11000, 20)].map2, data.warps[&(11000, 20)].x, data.warps[&(11000, 20)].y), (58000, 490, 1790));
    assert_eq!((data.warps[&(27000, 2)].map2, data.warps[&(27000, 2)].x, data.warps[&(27000, 2)].y), (27851, 2310, 310));

    // Sample fallbacks missing rt=2 in eve.emg
    let tower = &data.warps[&(11901, 2)];
    assert_eq!((tower.map2, tower.x, tower.y), (11902, 670, 1230));
    let no_door = &data.warps[&(11840, 1)];
    assert_eq!((no_door.map2, no_door.x, no_door.y), (11000, 970, 1490));

    // Redundant rows already covered by eve.emg must not be in Warps.txt
    assert!(!data.warps.contains_key(&(49901, 2)));
    assert!(!data.warps.contains_key(&(49902, 1)));
}

#[test]
fn battle_gates_wired_into_production_boot() {
    let data = crate::common::shared_game_data();
    assert_eq!(
        data.battle_gates.len(),
        10,
        "Compacted BattleGate.txt must load exactly 10 entries missing rt=3 in eve.emg"
    );

    for map_id in 49901..=49908 {
        assert!(
            data.battle_gates.contains_key(&(map_id, 2)),
            "Tower gate ({map_id}, 2) must remain in BattleGate.txt"
        );
    }
    assert!(data.battle_gates.contains_key(&(15481, 4)));
    assert!(data.battle_gates.contains_key(&(21000, 40)));

    // Gates with rt=3 in eve.emg must be removed from BattleGate.txt
    assert!(!data.battle_gates.contains_key(&(60011, 2)));
}
