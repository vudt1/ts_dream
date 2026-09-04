//! Rank.Dat loader tests — spec: `ts_mobile_client/Data/RankData.lua`
//! (`RankData.New`: string(20) + u16 honor + 4 × (u8 kind + i32 value),
//! keys `offset=9, xor1=0xFD, xor2=0xECEA, xor4=0x0B80F4B4`).

use std::path::PathBuf;
use ts_dream::data::loaders::{RankDatLoader, RankDef};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

#[test]
fn rank_loads_42_named_ranks() {
    let bytes = std::fs::read(data_dir().join("Rank.Dat")).expect("read Rank.Dat");
    // 43 records × 43 bytes; record 0 is an all-zero dummy.
    assert_eq!(bytes.len(), 43 * RankDatLoader::RECORD_SIZE);

    let ranks = RankDatLoader::load(&bytes).expect("parse Rank.Dat");
    assert_eq!(ranks.len(), 42, "dummy record 0 must be skipped");

    // Keys are 1-based record indices (Lua `rankDatas` loop counter parity).
    assert!(!ranks.contains_key(&0));
    assert!(!ranks.contains_key(&1));
    assert!(ranks.contains_key(&2));
    assert!(ranks.contains_key(&43));
}

#[test]
fn rank_first_and_last_records_match_lua_spec() {
    let bytes = std::fs::read(data_dir().join("Rank.Dat")).expect("read Rank.Dat");
    let ranks = RankDatLoader::load(&bytes).expect("parse Rank.Dat");

    let first: &RankDef = &ranks[&2];
    assert_eq!(first.name, "Giap binh");
    assert_eq!(first.honor, 15);
    assert_eq!(first.attributes[0], (207, 5));
    assert_eq!(&first.attributes[1..], &[(0, 0), (0, 0), (0, 0)]);

    // Honor thresholds ascend toward the top rank.
    let last: &RankDef = &ranks[&43];
    assert_eq!(last.honor, 2000);
    assert_eq!(
        last.attributes,
        [(207, 500), (208, 480), (211, 40), (214, 25)]
    );
}

#[test]
fn rank_game_data_wires_catalog() {
    let data = ts_dream::data::loader::GameData::load(&data_dir()).expect("GameData::load");
    assert_eq!(data.rank_defs.len(), 42);
    assert_eq!(data.rank_defs[&2].honor, 15);
}
