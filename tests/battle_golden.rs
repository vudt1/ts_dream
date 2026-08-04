//! Battle-win golden replay (ticket 21, Chapter 9 §9.6).
//!
//! Drives a **seeded** NPC battle (deterministic RNG) through the
//! `BattleService`, feeding the op 0x32 skill command, and diffs the exact
//! server→client frame stream against `golden/03-battle-win.golden`.

use std::sync::Arc;
use tokio::sync::mpsc;

use ts_dream::battle::runner::BattleCommand;
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, QuestResult, Skill};
use ts_dream::server::session::Session;

fn game_data() -> GameData {
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

fn strong_session() -> Arc<tokio::sync::RwLock<Session>> {
    let s = Arc::new(tokio::sync::RwLock::new(Session::new()));
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

async fn drain(rx: &mut mpsc::UnboundedReceiver<String>) -> Vec<String> {
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

#[tokio::test]
async fn battle_win_golden_replay() {
    let mut service = BattleService::new(Arc::new(game_data()));
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

    // Wait for the battle task to finish.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let empty = service
            .manager
            .len()
            .await == 0;
        if empty {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("battle did not finish in time");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let frames = drain(&mut rx).await;

    // Compare against the golden file's S2C stream.
    let golden = ts_dream::harness::Golden::from_file("golden/03-battle-win.golden")
        .expect("golden file loads");
    let expected: Vec<String> = golden
        .s2c
        .iter()
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();

    assert_eq!(
        frames, expected,
        "battle-win frame stream mismatch ({} got vs {} expected)\nGOT:\n{}\nEXPECTED:\n{}",
        frames.len(),
        expected.len(),
        frames.join("\n"),
        expected.join("\n"),
    );

    // Leader received the quest reward.
    let s = session.read().await;
    assert!(
        s.homdo.iter().any(|i| i.id == 46001),
        "leader granted the quest reward"
    );
    assert_eq!(s.talking_battle, 0);
}
