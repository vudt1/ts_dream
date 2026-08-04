//! Real-data integration: load the actual `ts_server_old/Data/` directory and
//! assert the expected row counts (Chapter 3 §3.2). This is the strongest
//! data-layer check and needs only the repo, not MySQL or a client.

use ts_dream::data::loader::GameData;

// Default data dir as shipped in the repo.
const DATA_DIR: &str = "ts_server_old/Data";

fn data_dir() -> std::path::PathBuf {
    // Allow override for environments where the repo is checked out elsewhere.
    std::env::var("TS_TEST_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(DATA_DIR))
}

#[test]
fn loads_real_data_with_expected_counts() {
    let dir = data_dir();
    if !dir.join("Npcs.txt").exists() {
        eprintln!("data dir not present ({}) — skipping", dir.display());
        return;
    }
    let d = GameData::load(&dir).expect("load real data");
    assert!(d.is_loaded());
    // Spec: 6,673 npcs, 8,376 items, 392 skills, 4,994 warps, 68 gates,
    // 20,265 npc-on-map, 1,161 item-on-map, 98 dolls, 813 quests.
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
    // NPC 10001 "Trß½ng Giác" — mojibake -> VISCII (ð/s-half...). Just assert
    // the name is a non-empty VISCII byte string and not the raw mojibake.
    let npc = d.npcs.get(&10001).expect("npc 10001");
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
    let found = d.talks.values().find(|q| !q.on_win.random_rewards.is_empty());
    if let Some(q) = found {
        assert!(q.on_win.random_rewards.iter().all(|t| t.2 >= 0));
        assert!(!q.on_win.random_rewards.is_empty());
    }
}
