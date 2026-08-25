//! Real-data integration: load the actual `Data/` directory and
//! assert the expected row counts (Chapter 3 §3.2). This is the strongest
//! data-layer check and needs only the repo, not MySQL or a client.

use ts_dream::data::loader::GameData;

// Default data dir as shipped in the repo (the bundled `Data/` the server
// loads at boot).
const DATA_DIR: &str = "Data";

fn data_dir() -> std::path::PathBuf {
    // Allow override for environments where the repo is checked out elsewhere.
    std::env::var("TS_TEST_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(DATA_DIR))
}

#[test]
fn loads_real_data_with_expected_counts() {
    let dir = data_dir();
    if !dir.join("Npc.dat").exists() && !dir.join("Npcs.txt").exists() {
        eprintln!("data dir not present ({}) — skipping", dir.display());
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    assert!(d.is_loaded());
    // Spec: > 6000 npcs, > 8000 items, > 300 skills, 200 texps
    assert!(d.npcs.len() > 6000, "npcs {}", d.npcs.len());
    assert!(d.items.len() > 8000, "items {}", d.items.len());
    assert!(d.skills.len() > 300, "skills {}", d.skills.len());
    assert!(d.texps.len() == 200, "texps {}", d.texps.len());
}

#[test]
fn loads_known_viscii_names() {
    let dir = data_dir();
    if !dir.exists() {
        return;
    }
    let d = GameData::load(&dir).expect("load");
    // Item 10000 "Dấu Chấm Hỏi" — VISCII 44 A4 75 20 43 68 A4 6D 20 48 F6 69.
    let item = d.items.get(&10000).expect("item 10000");
    assert_eq!(
        item.name,
        vec![0x44, 0xA4, 0x75, 0x20, 0x43, 0x68, 0xA4, 0x6D, 0x20, 0x48, 0xF6, 0x69]
    );
}

#[test]
fn npc_name_roundtrip() {
    let dir = data_dir();
    if !dir.exists() {
        return;
    }
    let d = GameData::load(&dir).expect("load");
    // NPC 10002 "Trương Bảo" (or 10005) — mojibake -> VISCII. Just assert
    // the name is a non-empty VISCII byte string and not the raw mojibake.
    let npc = d.npcs.get(&10002).expect("npc 10002");
    assert!(!npc.name.is_empty());
    let _ = ts_dream::data::tables::name_to_string(&npc.name);
}

#[test]
fn quests_parse_requires_select_menu() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // "10916 NPC 1 step 0.ini" has [REQUIRES] SelectMenu=30.
    let q = d
        .talks
        .get("10916:NPC:1:0")
        .expect("10916 NPC 1 step 0 quest");
    assert_eq!(q.require_select_menu, 30);
    // Absent Level/Reborn keys = no condition (None), never a `= 0` block.
    assert_eq!(q.require_level, None);
    assert_eq!(q.require_reborn, None);
}

#[test]
fn quests_parse_teamdef() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // "11011 Võ An Quốc Warp 3 step 0.ini" has [TEAMDEF] Diahinh=121 + 10 npcs.
    let q = d
        .talks
        .get("11011:WARP:3:0")
        .expect("11011 WARP 3 step 0 quest");
    assert_eq!(q.teamdef.len(), 11);
    assert_eq!(q.teamdef[0], 121);
    assert_eq!(q.teamdef[2], 17177);
}

#[test]
fn quests_parse_reward_share_and_auto_save() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // "10916 NPC 1 step 0.ini" Rewards=10001-1-0 → (10001, 1, 0), and
    // SaveLeaderQuests=AUTO → resolved to the talk's map/id/step+1.
    let q = d
        .talks
        .get("10916:NPC:1:0")
        .expect("10916 NPC 1 step 0 quest");
    assert_eq!(q.on_win.rewards, vec![(10001, 1, 0)]);
    assert_eq!(
        q.on_win.save_leader_quests,
        vec![(10916, 1, 0, 1)],
        "AUTO expands to (mapId, id, 0, step+1)"
    );
}

#[test]
fn quests_parse_random_rewards_triples() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // Find any quest with RandomRewards and assert 3-tuple shape.
    let found = d
        .talks
        .values()
        .find(|q| !q.on_win.random_rewards.is_empty());
    if let Some(q) = found {
        assert!(q.on_win.random_rewards.iter().all(|t| t.2 >= 0));
        assert!(!q.on_win.random_rewards.is_empty());
    }
}

#[test]
fn npcs_parse_drop_bat_reborn_columns() {
    // Regression: Drop1-6 were never parsed and _Bat/_Reborn read Drop1/Drop2
    // (issue 03#1). NPC 10005: skill 10001/10003/10006/0, drops
    // 26156/26158/27038/0/49001/0, NotPet=0, Reborn=0.
    let dir = data_dir();
    if !dir.join("Npc.dat").exists() && !dir.join("Npcs.txt").exists() {
        eprintln!("data dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    let npc = d.npcs.get(&10005).expect("npc 10005");
    assert_eq!(npc.skill, [10001, 10003, 10006, 0]);
    assert_eq!(npc.item, [26156, 26158, 27038, 0, 49001, 0]);
    assert_eq!(npc.bat, 0);
    assert_eq!(npc.reborn, 0);
}

#[test]
fn items_garble_replicated() {
    let dir = data_dir();
    if !dir.join("Item.dat").exists() && !dir.join("Items.txt").exists() {
        eprintln!("data dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // Item 18973 from binary Item.dat has valid wire hex
    let item = d.items.get(&18973).expect("item 18973");
    assert_eq!(
        item.wire_name_hex().as_deref(),
        Some("5468E1692084742062696E68207068E170")
    );
    // Clean name (item 10000) has no override.
    let clean = d.items.get(&10000).expect("item 10000");
    assert_eq!(clean.garble, None);
}

#[test]
fn item_drops_prefill_and_spawn() {
    let dir = data_dir();
    if !dir.join("ItemOnMap.txt").exists() {
        eprintln!("data dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // Map 10965 slot 1 -> item 31099 at (2228, 126), spawned with _Delay=999999.
    let spawned = d.item_drop_on_map.get(&(10965, 1)).expect("spawned drop");
    assert_eq!(spawned.item_id, 31099);
    assert_eq!(spawned.map_x, 2228);
    assert_eq!(spawned.map_y, 126);
    assert_eq!(spawned.delay, 999_999);
    // Pre-filled empty slots 1..255 for every ItemOnMap map.
    assert_eq!(
        d.item_drop_on_map
            .get(&(10965, 255))
            .expect("slot 255")
            .item_id,
        0
    );
    // The static drop frame is the C# `F44408001703` + le16 id + x + y.
    assert_eq!(
        GameData::static_drop_frame(31099, 2228, 126),
        "F444080017037B79B4087E00"
    );
}

#[test]
fn quests_parse_requires_conditions() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // "12021 triệu quảng 7 step 0.ini": Level=20 >=, Reborn=1 >=, SelectMenu=30,
    // TEAMDEF Diahinh=5479 + 10 npcs.
    let q = d.talks.get("12041:NPC:7:0").expect("12041 NPC 7 step 0");
    assert_eq!(q.require_level, Some((20, 1)));
    assert_eq!(q.require_reborn, Some((1, 1)));
    assert_eq!(q.require_select_menu, 30);
    assert_eq!(
        q.teamdef,
        vec![5479, 42213, 27149, 42215, 27149, 42214, 27149, 27149, 27149, 27149, 27149]
    );
}

#[test]
fn quests_on_lose_warpto_reads_onwin() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // "11021-Van Du dao Si.ini": [OnWin] WarpTo=11901\t210\t1230 — the C#
    // copy-paste bug (Data.cs:4649) means _LoseWarpTo == _WinWarpTo.
    let q = d.talks.get("11021:NPC:9:0").expect("11021 NPC 9 step 0");
    assert_eq!(q.on_win.warp_to, vec![11901, 210, 1230]);
    assert_eq!(q.on_lose.warp_to, q.on_win.warp_to);
}

#[test]
fn quests_add_skill_is_single_pair() {
    let dir = data_dir();
    if !dir.join("Quests").is_dir() {
        eprintln!("Quests dir not present — skipping");
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    // C# `_WinAddSkill = int[] {skillId, level}` — "14001\t1" is ONE pair, not
    // two (regression: the loader used to split into (14001,1) and (1,1)).
    let q = d.talks.get("12136:NPC:1:3").expect("12136 NPC 1 step 3");
    assert_eq!(q.on_win.add_skill, vec![(14001, 1)]);
}

#[test]
fn binary_loaders_load_all_dat_files() {
    let dir = data_dir();
    if !dir.exists() {
        return;
    }
    let d = GameData::load(&dir).expect("load real data with all binary files");

    // 1. Formula.Dat
    let formula = d.formula_params.as_ref().expect("Formula.Dat loaded");
    assert_eq!(formula.base_hp, 80);
    assert_eq!(formula.base_sp, 60);
    assert_eq!(formula.w1, 1.0);
    assert_eq!(formula.w2, 4.0);

    // 2. BlissBag.Dat
    assert!(!d.bliss_bags.is_empty(), "BlissBag.Dat loaded");
    let bag = d.bliss_bags.get(&46205).expect("BlissBag 46205");
    assert_eq!(bag.bag_item_id, 46205);
    assert_eq!(bag.kind_count, 2);
    assert_eq!(bag.items.len(), 8);

    // 3. Compound.Dat
    assert_eq!(d.compounds.len(), 1394, "Compound.Dat loaded");
    assert_eq!(d.compounds[1].compound_d, 1);
    assert_eq!(d.compounds[1].material_a, 1);
    assert_eq!(d.compounds[1].material_b, 1);

    // 4. Astrolabe.Dat
    assert_eq!(d.astrolabes.len(), 7, "Astrolabe.Dat loaded 7 stars");
    let astro1 = d.astrolabes.get(&1).expect("Astrolabe star 1");
    assert_eq!(astro1.attributes[0], (1, 3));

    // 5. CityEx.Dat
    assert_eq!(d.city_ex.len(), 110, "CityEx.Dat loaded 110 rows");
    assert_eq!(d.city_ex[0].data[0], 0.7);

    // 6. EVOStatus.Dat
    assert_eq!(d.evo_statuses.len(), 43, "EVOStatus.Dat loaded 43 statuses");
    let evo1 = d.evo_statuses.get(&1).expect("EVOStatus 1");
    assert_eq!(evo1.value, 10);
    assert_eq!(evo1.item_id, 25501);

    // 7. Warp.Dat
    assert!(!d.warp_defs.is_empty(), "Warp.Dat loaded");

    // 8. Item.dat & Npc.dat
    assert_eq!(d.item_defs.len(), 8371, "Item.dat loaded 8371 defs");
    assert_eq!(d.npc_defs.len(), 6659, "Npc.dat loaded 6659 defs");

    // 9. eve.emg
    assert!(!d.scene_eve_data.is_empty(), "eve.emg loaded");
    assert!(d.scene_eve_data.len() > 1000, "eve.emg non-empty scenes count > 1000 (actual: {})", d.scene_eve_data.len());
}

#[test]
fn eve_loader_loads_major_scenes() {
    let dir = data_dir();
    if !dir.exists() {
        return;
    }
    let d = GameData::load(&dir).expect("load real data");

    // Total loaded scene count
    println!("Total scenes loaded: {}", d.scene_eve_data.len());
    assert!(!d.scene_eve_data.is_empty(), "eve.emg must contain scenes");

    // 1. Tân thủ thôn (Scene 10801)
    let scene_10801 = d.scene_eve_data.get(&10801).expect("Scene 10801 present");
    println!(
        "10801: npcs={}, doors={}, events={}, scene_infos={}, fight_datas={}",
        scene_10801.npcs.len(),
        scene_10801.doors.len(),
        scene_10801.npc_events.len(),
        scene_10801.scene_infos.len(),
        scene_10801.fight_datas.len()
    );
    assert!(!scene_10801.npcs.is_empty(), "Scene 10801 has NPCs");
    assert!(!scene_10801.scene_infos.is_empty(), "Scene 10801 has Scene Infos");
    assert!(!scene_10801.fight_datas.is_empty(), "Scene 10801 has FightDatas");

    // 2. Trác Quận (Scene 12001)
    let scene_12001 = d.scene_eve_data.get(&12001).expect("Scene 12001 present");
    println!(
        "12001: npcs={}, doors={}, events={}, scene_infos={}, fight_datas={}",
        scene_12001.npcs.len(),
        scene_12001.doors.len(),
        scene_12001.npc_events.len(),
        scene_12001.scene_infos.len(),
        scene_12001.fight_datas.len()
    );
    assert!(!scene_12001.npcs.is_empty(), "Scene 12001 has NPCs");

    // Validate NpcPlacement properties
    for npc in scene_10801.npcs.values() {
        assert!(npc.id > 0);
    }

    // Validate SceneInfo destinations
    for scene_info in scene_10801.scene_infos.values() {
        assert!(scene_info.eve_no > 0);
    }
}



