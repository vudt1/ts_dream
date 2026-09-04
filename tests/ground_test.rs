//! ground.mmg loader tests — spec: `GroundMmgLoader.kt`, itself a port of
//! `ts_mobile_client/Logic/DataManager.lua::OnLoadMapData` (tail index:
//! `count(u16) + count × 29B` entries of name(1+20B `.map`) + pos(i32) +
//! size(i32); per-body walk to the `geolBaseAtt(u8)` field).

use std::path::PathBuf;
use ts_dream::data::loaders::GroundMmgLoader;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Data")
}

#[test]
fn ground_loads_reference_terrains() {
    let bytes = std::fs::read(data_dir().join("ground.mmg")).expect("read ground.mmg");
    let terrains = GroundMmgLoader::load(&bytes).expect("parse ground.mmg");
    // 4528 indexed maps; only bodies whose walk yields geolBaseAtt > 0 land here.
    assert!(
        terrains.len() > 3000,
        "expected most indexed maps to resolve, got {}",
        terrains.len()
    );
    assert!(
        terrains.values().all(|&g| g > 0),
        "zero geolBaseAtt entries must be dropped (Kotlin parity)"
    );
}

#[test]
fn ground_degrades_on_garbage() {
    assert!(GroundMmgLoader::load(&[]).expect("empty").is_empty());
    assert!(GroundMmgLoader::load(&[0x00]).expect("1 byte").is_empty());
    // Declared count overshoots the file: negative index start → empty.
    assert!(
        GroundMmgLoader::load(&[0xFF, 0xFF, 0x10, 0x00])
            .expect("bogus")
            .is_empty()
    );
}
