//! Shared golden-suite fixtures and replay helpers (ticket 23).
//!
//! One place that owns the scenario definitions (data + session seeding) and
//! the two replay paths:
//! - synchronous dispatch replay for the feature handlers, and
//! - the seeded async battle replay for `golden/03-battle-win`.
//!
//! `run_all_goldens` is the byte-level acceptance gate: it loads every
//! `golden/*.golden`, replays it against the Rust server code, and fails on
//! any frame diff (Ch9 §7/§9.4).

use std::sync::Arc;

use ts_dream::battle::runner::BattleCommand;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, QuestResult, Skill};
use ts_dream::harness::Golden;
use ts_dream::harness::scenario::Scenario;
use ts_dream::server::session::{Conn, InventoryItem, PetState};

/// Fixed instant (unix seconds) the login-success banner is pinned to.
pub const FIXED_NOW: i64 = 1_700_000_000;

pub fn game_data() -> GameData {
    GameData::default()
}

/// Data fixture for the FTalk.H6 quest scenario (map 10916, NPC 1).
pub fn quest_data() -> GameData {
    let mut data = GameData::default();
    data.talks.insert(
        "10916:NPC:1:0".to_string(),
        ts_dream::data::tables::QuestDef {
            map_id: 10916,
            id: 1,
            dialogs: "F44411001401000000010603010000000000000100".to_string(),
            require_select_menu: 30,
            on_lose: QuestResult {
                dialogs: "F44411001401000000010103010000000000009E28".to_string(),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    data
}

/// Data fixture with a real potion (hp 100 / sp 50) for the use-item golden.
pub fn use_item_data() -> GameData {
    let mut data = GameData::default();
    data.items.insert(
        30001,
        ts_dream::data::tables::Item {
            id: 30001,
            hp: 100,
            sp: 50,
            ..Default::default()
        },
    );
    data
}

fn create_char_frame() -> String {
    let mut payload = vec![0u8; 26];
    payload[0] = 1; // sex
    payload[2] = 2; // hair
    payload[12] = 3; // element
    payload[19] = 4; // pass1 len
    let mut hex = String::from("F4441C000901");
    hex.push_str(&ts_dream::protocol::encoder::hex(&payload));
    hex
}

/// The deterministic, synchronous scenarios. Each maps 1:1 to a `golden/*`
/// file (except the two hello scaffolds and the async battle-win).
pub fn all_scenarios() -> Vec<Scenario<'static>> {
    vec![
        Scenario::new("01-hello", game_data(), vec!["F444010000".to_string()], |_| {}),
        Scenario::new(
            "02-login-scaffold",
            game_data(),
            vec!["F444010000".to_string()],
            |_| {},
        ),
        Scenario::new(
            "04-login-success",
            game_data(),
            vec!["F4440D000101E1930400766EBA003132333435".to_string()],
            |c: &mut Conn| {
                c.session.name = b"TESTNAME".to_vec();
            },
        )
        .with_now(FIXED_NOW),
        Scenario::new(
            "05-login-wrong-pass",
            game_data(),
            vec!["F4440D000101E1930400766EBA0057524F4E47".to_string()],
            |_| {},
        ),
        Scenario::new(
            "06-create-char",
            game_data(),
            vec![
                "F4440A000902544553544E414D45".to_string(),
                create_char_frame(),
            ],
            |_| {},
        ),
        Scenario::new("07-move", game_data(), vec!["F44407000601026400C800".to_string()], |c| {
            c.session.id = 300001;
        }),
        Scenario::new("08-chat", game_data(), vec!["F4440700020248454C4C4F".to_string()], |c| {
            c.session.id = 300001;
        }),
        Scenario::new(
            "09-mall-buy",
            game_data(),
            vec!["F4440600420100001127C800".to_string()],
            |_| {},
        ),
        Scenario::new(
            "10-use-item",
            use_item_data(),
            vec!["F4440400170F0102".to_string()],
            |c| {
                c.session.hp_max = 200;
                c.session.hp = 50;
                c.session.sp_max = 200;
                c.session.sp = 30;
                c.session.homdo.push(InventoryItem {
                    slot: 1,
                    id: 30001,
                    count: 5,
                    ..Default::default()
                });
            },
        ),
        Scenario::new(
            "11-warp",
            game_data(),
            vec!["F4440D0002022F77617270203132303031".to_string()],
            |c| {
                c.session.id = 300001;
            },
        ),
        Scenario::new(
            "12-quest-h6",
            quest_data(),
            vec![
                "F444040014010100".to_string(),
                "F4440300140600".to_string(),
            ],
            |c| {
                c.session.map_id = 10916;
                c.session.select_menu = 20;
            },
        ),
        Scenario::new(
            "13-battle-leave",
            game_data(),
            vec!["F44404000B0103".to_string()],
            |c| {
                c.session.id = 300001;
                c.session.battle_id = 5;
            },
        ),
        Scenario::new(
            "14-pet",
            game_data(),
            vec![
                "F44404001301993A".to_string(),
                "F4440300130200".to_string(),
            ],
            |c| {
                c.session.pets.push(PetState {
                    stt: 1,
                    id: 15001,
                    ..Default::default()
                });
            },
        ),
    ]
}

/// Scenario lookup by golden name.
pub fn scenario_for(name: &str) -> Option<Scenario<'static>> {
    all_scenarios().into_iter().find(|s| s.name == name)
}

/// The in-process dispatch frame stream for a scenario name (empty if the
/// scenario is the async battle-win).
pub async fn replay_sync(name: &str) -> Vec<String> {
    scenario_for(name).expect("scenario registered").replay().await
}

// ---- Battle-win replay (golden/03-battle-win) ----

fn battle_data() -> GameData {
    let mut data = GameData::default();
    data.npcs.insert(
        9001,
        Npc {
            id: 9001,
            name: b"Npc9001".to_vec(),
            lv: 1,
            hp: 30,
            sp: 30,
            thuoctinh: 1,
            atk: 1,
            def: 1,
            agi: 1,
            int1: 1,
            skill: [10000, 0, 0, 0],
            ..Default::default()
        },
    );
    data.skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Kiem".to_string(),
            sp: 5,
            lv_max: 10,
            skill_type: 1,
            do_manh: 10,
            sl_danh: 1,
            combo: 0,
            delay: 1000,
            ..Default::default()
        },
    );
    data.talks.insert(
        "12001:NPC:7:0".to_string(),
        ts_dream::data::tables::QuestDef {
            map_id: 12001,
            id: 7,
            on_win: QuestResult {
                rewards: vec![(46001, 5, 0)],
                message: "Win".to_string(),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    data
}

fn strong_session() -> Arc<tokio::sync::RwLock<ts_dream::server::session::Session>> {
    let s = Arc::new(tokio::sync::RwLock::new(ts_dream::server::session::Session::new()));
    {
        let mut s = s.try_write().unwrap();
        s.id = 300001;
        s.level = 50;
        s.atk = 300;
        s.def = 50;
        s.agi = 100;
        s.int1 = 100;
        s.hp_max = ts_dream::battle::engine::get_hp_max(0, 0, 50, 6) as u16;
        s.sp_max = ts_dream::battle::engine::get_sp_max(0, 0, 50, 6) as u16;
        s.hp = s.hp_max;
        s.sp = s.sp_max;
        s.map_id = 12001;
        s.talking_battle = 7;
        s.skills.push((10000, 10));
    }
    s
}

async fn drain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut count = 0usize;
    while count < 500 {
        match tokio::time::timeout(std::time::Duration::from_millis(2), rx.recv()).await {
            Ok(Some(f)) => {
                out.push(f);
                count += 1;
            }
            _ => break,
        }
    }
    out
}

/// Seeded NPC battle replay — the async S2C stream locked by `03-battle-win`.
pub async fn battle_win_frames() -> Vec<String> {
    let mut service = BattleService::new(Arc::new(battle_data()));
    service.set_input_timeout(std::time::Duration::from_millis(50));
    let service = Arc::new(service);

    let session = strong_session();
    let mut rx = service.register(300001, Arc::clone(&session));

    {
        let mut s = session.write().await;
        service.start_npc_battle_seeded(&mut s, 9001, 11, 12345, 67890, 1111);
    }
    {
        let cmd = BattleCommand {
            row: 3,
            col: 2,
            skill_id: 10000,
            skill_lv: 10,
            row_attack: 0,
            col_attack: 2,
            use_item: 0,
        };
        let s = session.read().await;
        service.submit_command(&s, cmd);
    }

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if service.manager.len().await == 0 {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("battle did not finish in time");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    drain(&mut rx).await
}

/// The byte-level acceptance gate over every golden in `golden/`.
pub async fn run_all_goldens() {
    let goldens = Golden::load_dir("golden").expect("golden/ loads");
    assert!(!goldens.is_empty(), "no golden files loaded");

    for g in &goldens {
        let expected: Vec<String> = g
            .s2c
            .iter()
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty())
            .collect();

        let got: Vec<String> = match g.name.as_str() {
            "03-battle-win" => battle_win_frames().await,
            name => replay_sync(name).await,
        };

        assert_eq!(
            got, expected,
            "golden `{}` mismatch ({} got vs {} expected)\nGOT:\n{}\nEXPECTED:\n{}",
            g.name,
            got.len(),
            expected.len(),
            got.join("\n"),
            expected.join("\n"),
        );
    }

    // Every golden is registered by a scenario (so a stray golden fails loudly).
    for g in &goldens {
        let registered = g.name == "03-battle-win" || scenario_for(&g.name).is_some();
        assert!(registered, "no scenario registered for golden `{}`", g.name);
    }
}

/// Regenerate the golden files for all synchronous scenarios (reproducible
/// re-capture when behaviour legitimately changes; Ch9 §9.2).
pub async fn regenerate(dir: &str, comment: &str) {
    for s in all_scenarios() {
        s.save(dir, comment).await.expect("golden saved");
    }
}