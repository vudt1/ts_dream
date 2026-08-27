//! Static data loader tests — migrated from inline #[cfg(test)] blocks (ticket 08).

// ── data::ini ──

use ts_dream::data::ini::{Ini, NOTHING};

#[test]
fn absent_key_is_nothing() {
    let ini = Ini::parse("[BASE]\nMapId=1\n");
    assert_eq!(ini.get("BASE", "Missing"), NOTHING);
    assert_eq!(ini.get_raw("BASE", "Missing"), None);
}

#[test]
fn case_insensitive_section_key() {
    let ini = Ini::parse("[OnWin]\nWarpTo=5\n");
    assert_eq!(ini.get("onwin", "warpto"), "5");
    assert_eq!(ini.get("ONWIN", "WARPTO"), "5");
}

#[test]
fn on_lose_warpto_reads_onwin() {
    // [OnLose] WarpTo must read from ONWIN — a quirk the spec keeps.
    let ini = Ini::parse("[OnWin]\nWarpTo=99");
    assert_eq!(ini.get("ONLOSE", "WarpTo"), NOTHING); // absent -> sentinel
                                                      // Executor must replicate the quirk by reading ONWIN for OnLose.WarpTo.
    assert_eq!(ini.get("ONWIN", "WarpTo"), "99");
}

#[test]
fn value_capped() {
    let long = "x".repeat(5000);
    let ini = Ini::parse(&format!("[S]\nK={}", long));
    assert_eq!(
        ini.get_raw("S", "K").unwrap().len(),
        ts_dream::data::ini::VALUE_CAP
    );
}

// ── data::reader ──

use ts_dream::data::reader::DatReader;

#[test]
fn test_basic_numeric_reads() {
    let bytes = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    let mut reader = DatReader::new(bytes);
    assert_eq!(reader.read_u8(), 0x12);
    assert_eq!(reader.read_u16(), 0x5634);
    assert_eq!(reader.read_u32(), 0xDEBC9A78);
}

#[test]
fn test_offset_and_xor() {
    // Test with offset = 3, xor1 = 211
    let val: u8 = 10;
    let encoded: u8 = (val + 3) ^ 211;
    let mut reader = DatReader::with_keys(vec![encoded], 3, 211, 0, 0);
    assert_eq!(reader.read_u8(), 10);
}

#[test]
fn test_pc_reversed_string() {
    // 1 byte count=4, 6 bytes buffer, reversed string at the end
    // String "ABCD" -> bytes [4, padding: 0x00, 0x00, 'D', 'C', 'B', 'A']
    let bytes = vec![4, 0x00, 0x00, b'D', b'C', b'B', b'A'];
    let mut reader = DatReader::new(bytes);
    let s = reader.read_string(6);
    assert_eq!(s, "ABCD");
}

// ── data::texps ──

use ts_dream::data::texps::{compute_texps, texp_get_lv_up};
use ts_dream::protocol::MAX_LEVEL;

#[test]
fn texps_monotonic() {
    let t = compute_texps();
    assert_eq!(t.len(), MAX_LEVEL as usize);
    // Cumulative thresholds strictly increase with level for each reborn.
    for r in 0..3 {
        let mut prev = 0i64;
        for row in &t {
            assert!(row.reborn[r] >= prev);
            prev = row.reborn[r];
        }
    }
}

#[test]
fn banker_round_half_even() {
    // 2.5 -> 2, 3.5 -> 4 (round-half-to-even).
    assert_eq!(2.0_f64.round_ties_even(), 2.0);
    assert_eq!(3.5_f64.round_ties_even(), 4.0);
    assert_eq!(0.5_f64.round_ties_even(), 0.0);
}

#[test]
fn lvup_at_zero_texp_is_zero() {
    let t = compute_texps();
    assert_eq!(texp_get_lv_up(&t, 1, 0, 0), 0);
}

#[test]
fn texps_exact_values() {
    // `_0(i) = _0(i-1) + (int)(Round(Pow(i+1,2.9))+5)` with round-half-to-even,
    // starting from texp=0 — so Texps[0] = 6, not a zero sentinel.
    let t = compute_texps();
    assert_eq!(t.len(), MAX_LEVEL as usize);
    assert_eq!(t[0].lv, 0);
    assert_eq!(t[0].reborn[0], 6);
    assert_eq!(t[0].reborn[1], 6);
    assert_eq!(t[0].reborn[2], 6);
    // i=1: 6 + round(2^2.9)=7 +5 = 18; 6 + round(2^3.0)=8+5 = 19; 2^3.05 same.
    assert_eq!(t[1].lv, 1);
    assert_eq!(t[1].reborn[0], 18);
    assert_eq!(t[1].reborn[1], 19);
    assert_eq!(t[1].reborn[2], 19);
    // Level-up at lv1 needs the threshold 18 (was 12 before the fix).
    assert_eq!(texp_get_lv_up(&t, 1, 0, 17), 0);
    assert_eq!(texp_get_lv_up(&t, 1, 0, 18), 1);
}

// ── data::loader ──

use std::path::Path;
use tempfile::tempdir;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::Item;

/// Render a data table into a directory. Not part of the runtime.
fn seed_temp_dir(dir: &Path, files: &[(&str, &[u8])]) {
    for (name, data) in files {
        std::fs::write(dir.join(name), data).unwrap();
    }
}

/// Minimal but structurally valid `.txt` dataset for loader tests.
/// 25-col Items row, 24-col Npcs row, 19-col Skills row.
fn write_dataset(dir: &Path) {
    std::fs::create_dir_all(dir.join("Quests")).unwrap();
    // Npcs.txt is UTF-16LE with BOM (24-col row: id..agi, skills, drops, NotPet, Reborn).
    let npc = "1\tA\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0";
    let mut npc_bytes = vec![0xFF, 0xFE];
    for u in npc.encode_utf16() {
        npc_bytes.extend_from_slice(&u.to_le_bytes());
    }
    seed_temp_dir(
        dir,
        &[
            (
                "Items.txt",
                b"//Id\tName\t...\n1\tA\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\n",
            ),
            (
                "Skills.txt",
                b"//Id\tName\t...\n1\tA\t1\t1\t1\t0\t0\t0\t0\t0\t0\t1\t1\t1\t1\t0\t0\t0\t0\n",
            ),
            (
                "BattleGate.txt",
                b"//Mapid1\tWarpId\tDiahinh\n1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\n",
            ),
            ("Dolls.txt", b"//DollId\tNpcId\n1\t2\n"),
            (
                "NpcOnMap.txt",
                b"//MapId\tId\tNpcId\tX\tY\tCoord\tSoLuong\n1\t1\t2\t3\t4\t5\t0\n",
            ),
            ("ItemOnMap.txt", b"//MapId\tId\tItemId\tX\tY\tDelay\n"),
            ("Npcs.txt", &npc_bytes),
        ],
    );
}

#[test]
fn warps_skip_empty_destination_column() {
    let dir = tempdir().unwrap();
    write_dataset(dir.path());
    seed_temp_dir(
        dir.path(),
        &[(
            "Warps.txt",
            b"//map1\twarpid\tmap2\tx\ty\n1\t2\t\t3\t4\n5\t6\t7\t8\t9\n",
        )],
    );
    let d = GameData::load_legacy_text(dir.path()).expect("load legacy fixture");
    // The row with an empty map2 column is silently dropped.
    assert_eq!(d.warps.len(), 1);
    assert!(d.warps.contains_key(&(5, 6)));
    assert!(!d.warps.contains_key(&(1, 2)));
}

#[test]
fn npcs_missing_reborn_column_is_load_failure() {
    let dir = tempdir().unwrap();
    write_dataset(dir.path());
    // 23-col row (no Reborn col 23) — a missing numeric column is a load
    // failure, not a default.
    let npc = "2\tB\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0\t0";
    let mut bytes = vec![0xFF, 0xFE];
    for u in npc.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    seed_temp_dir(dir.path(), &[("Npcs.txt", &bytes)]);
    assert!(GameData::load_legacy_text(dir.path()).is_err());
}

#[test]
fn item_drop_prefill_does_not_repeat_per_map() {
    let mut d = GameData::default();
    d.items.insert(
        31099,
        Item {
            id: 31099,
            level: 1,
            ..Default::default()
        },
    );
    let dir = tempdir().unwrap();
    let path = dir.path().join("ItemOnMap.txt");
    std::fs::write(
        &path,
        b"//MapId\tId\tItemId\tX\tY\tDelay\n10965\t1\t31099\t2228\t126\t1\n10965\t2\t31099\t10\t20\t1\n",
    )
    .unwrap();
    d.load_item_on_map(&path).expect("load item on map");
    // Pre-fill 255 slots happens once per map; both spawns land.
    assert_eq!(d.item_drop_on_map.len(), 255);
    assert_eq!(d.item_drop_on_map[&(10965, 1)].item_id, 31099);
    assert_eq!(d.item_drop_on_map[&(10965, 1)].delay, 999_999);
    assert_eq!(d.item_drop_on_map[&(10965, 2)].map_x, 10);
    assert_eq!(d.item_drop_on_map[&(10965, 255)].item_id, 0);
}

#[test]
fn require_items_survive_onwin_rebuild() {
    // Ticket 19 #4: `[REQUIRES].Items` must not be clobbered when
    // `parse_result("OnWin")` rebuilds `quest.on_win`.
    let dir = tempdir().unwrap();
    write_dataset(dir.path());
    seed_temp_dir(
        &dir.path().join("Quests"),
        &[(
            "q.ini",
            b"[BASE]\nMapId=1\nType=NPC\nId=2\nStep=0\nDialogs=0\n\
             [REQUIRES]\nSelectMenu=30\nLevel=5\t1\nItems=31044-1-0\n\
             [ONWIN]\nDialogs=0\nRewards=46001-1-0\n\
             [DESCRIPTION]\nTitle=t\n",
        )],
    );
    let q = GameData::default()
        .parse_quest_ini(&dir.path().join("Quests/q.ini"))
        .expect("parse quest ini");
    assert_eq!(q.require_select_menu, 30);
    assert_eq!(q.on_win.require_items, vec![(31044, 1, 0)]);
    assert_eq!(q.on_win.rewards, vec![(46001, 1, 0)]);
}

// ── data::loaders::eve ──

use ts_dream::data::loaders::eve::{
    EveConditionClass, EveFightEnemy, EveResultClass, EveResultType,
};

#[test]
fn test_eve_enums_and_models() {
    assert_eq!(EveConditionClass::from(0), EveConditionClass::Unconditional);
    assert_eq!(EveConditionClass::from(1), EveConditionClass::BagItem);
    assert_eq!(EveConditionClass::from(2), EveConditionClass::QuestStep);
    assert_eq!(
        EveConditionClass::from(7),
        EveConditionClass::PlayerAttribute
    );
    assert_eq!(EveConditionClass::from(8), EveConditionClass::BattleResult);
    assert_eq!(EveConditionClass::from(9), EveConditionClass::FollowPet);
    assert_eq!(EveConditionClass::from(10), EveConditionClass::DialogChoice);
    assert_eq!(
        EveConditionClass::from(12),
        EveConditionClass::SceneEventCount
    );
    assert_eq!(EveConditionClass::from(14), EveConditionClass::RoleCount);

    assert_eq!(EveResultType::from(0), EveResultType::Action);
    assert_eq!(EveResultType::from(1), EveResultType::Talk);
    assert_eq!(EveResultType::from(2), EveResultType::Door);
    assert_eq!(EveResultType::from(3), EveResultType::Battle);
    assert_eq!(EveResultType::from(5), EveResultType::Animation);
    assert_eq!(EveResultType::from(6), EveResultType::Surface);
    assert_eq!(EveResultType::from(9), EveResultType::NpcAction);

    assert_eq!(EveResultClass::from(1), EveResultClass::Item);
    assert_eq!(EveResultClass::from(2), EveResultClass::Quest);
    assert_eq!(EveResultClass::from(3), EveResultClass::NpcTeam);
    assert_eq!(EveResultClass::from(4), EveResultClass::Skill);
    assert_eq!(EveResultClass::from(7), EveResultClass::Player);
    assert_eq!(EveResultClass::from(8), EveResultClass::RewardPet);

    let enemy = EveFightEnemy {
        no: 1,
        npc_id: 10001,
        location_pos: 7, // col = 7 / 5 = 1, row = 7 % 5 = 2
        ai: 0,
    };
    assert_eq!(enemy.col(), 1);
    assert_eq!(enemy.row(), 2);
}

#[test]
fn production_loader_is_binary_only() {
    let dir = tempdir().unwrap();
    write_dataset(dir.path());
    let err = GameData::load(dir.path())
        .expect_err("text-only fixture must not be accepted by production loader");
    assert!(err.to_string().contains("binary Item.dat"));
}

#[test]
fn production_loader_inventory_accepts_only_binary_extensions() {
    let dir = tempdir().unwrap();
    let item = dir.path().join("Item.dat");
    let npc = dir.path().join("Npc.dat");
    std::fs::write(&item, b"not-a-valid-item-dat").unwrap();
    std::fs::write(&npc, b"not-a-valid-npc-dat").unwrap();
    std::fs::write(dir.path().join("ignored.txt"), b"text").unwrap();
    std::fs::write(dir.path().join("ignored.mmg"), b"legacy-map").unwrap();

    let names = GameData::binary_asset_inventory(dir.path()).expect("inventory");
    assert!(names.iter().any(|name| name == "Item.dat"));
    assert!(names.iter().any(|name| name == "Npc.dat"));
    assert!(!names.iter().any(|name| name == "ignored.txt"));
    assert!(!names.iter().any(|name| name == "ignored.mmg"));
}
