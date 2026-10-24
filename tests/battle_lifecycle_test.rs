//! Ticket 20 & 21 — Battle Turn Lifecycle Timing and Turn Action Finish (32 02).
//!
//! Tests:
//! 1. Turn 1 Handshake Barrier: waits for scene ack 0B FA from client, with fallback.
//! 2. Turn Prompt 34 01: emitted at turn start to living players BEFORE command collection.
//! 3. Command timeout fallback: auto-assigns NormalAttack (10000) when timer expires.
//! 4. Frame order: 34 01 -> action 32 01 -> 32 02 (turn_action_finish).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ts_dream::battle::construction::Battle;
use ts_dream::battle::manager::{BattleManager, BattleSink};
use ts_dream::battle::packets;
use ts_dream::battle::runner::{BattleAction, BattleCommand, DbUpdate, Outcome};
use ts_dream::data::tables::{Npc, Skill};

#[derive(Default)]
struct CapturingSink {
    frames: Mutex<Vec<(Option<i64>, String)>>,
    ended: Mutex<Option<(i32, Outcome)>>,
}

impl BattleSink for CapturingSink {
    fn send_to(&self, player: i64, frame: String) {
        self.frames.lock().unwrap().push((Some(player), frame));
    }
    fn send_map(&self, _player: i64, _frame: String) {}
    fn broadcast(&self, frame: String) {
        self.frames.lock().unwrap().push((None, frame));
    }
    fn apply_db(&self, _update: DbUpdate) {}
    fn apply_drop(&self, _drop: ts_dream::battle::runner::Out) {}
    fn apply_catch(&self, _owner: i64, _npc_id: i64) {}
    fn apply_fled(&self, _player: i64) {}
    fn apply_respawn(&self, _npc_id: i64, _map_id: i64, _x: i64, _y: i64) {}
    fn apply_pet_exp(&self, _owner: i64, _stt: i64, _exp: i64) {}
    fn battle_ended(&self, id: i32, outcome: Outcome) {
        *self.ended.lock().unwrap() = Some((id, outcome));
    }
}

fn create_test_battle() -> (Battle, Arc<CapturingSink>) {
    let mut battle = Battle::new(1, 0);
    // Player at (3, 2)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 100;
        c.hp = 100;
        c.hp_max = 100;
        c.sp = 50;
        c.sp_max = 50;
        c.team = 1;
        c.lv = 10;
        c.atk = 50;
        c.def = 20;
        c.agi = 20;
    }
    // Enemy at (0, 2)
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.hp = 1;
        c.hp_max = 1;
        c.team = 2;
        c.lv = 5;
        c.def = 10;
        c.agi = 5;
    }
    (battle, Arc::new(CapturingSink::default()))
}

#[tokio::test]
async fn turn1_handshake_barrier_releases_on_ack() {
    let (battle, sink) = create_test_battle();
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::<i64, Npc>::new()),
        Arc::new(HashMap::<i64, Skill>::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(200),
        sink.clone(),
    );

    // Prompt 34 01 should NOT be sent before scene ack or timeout
    // Immediate check
    tokio::time::sleep(Duration::from_millis(5)).await;
    {
        let frames = sink.frames.lock().unwrap();
        assert!(
            !frames.iter().any(|(_, f)| f == &packets::your_turn()),
            "34 01 must not be sent before handshake ack"
        );
    }

    // Now send scene ACK (0B FA)
    handle.ack_scene();

    // After ack, 34 01 prompt should be sent immediately to living player (100)
    let deadline = tokio::time::Instant::now() + Duration::from_millis(100);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames.iter().any(|(p, f)| *p == Some(100) && f == &packets::your_turn()) {
                break;
            }
        }
        assert!(tokio::time::Instant::now() < deadline, "expected 34 01 after scene ack");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn turn1_handshake_barrier_fallback_on_timeout() {
    let (battle, sink) = create_test_battle();
    let manager = Arc::new(BattleManager::new());
    // Spawn with 50ms timeout so test runs fast
    let _handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::<i64, Npc>::new()),
        Arc::new(HashMap::<i64, Skill>::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(50),
        sink.clone(),
    );

    // Without ack, fallback timeout should still release the barrier and send 34 01
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames.iter().any(|(p, f)| *p == Some(100) && f == &packets::your_turn()) {
                break;
            }
        }
        assert!(tokio::time::Instant::now() < deadline, "expected 34 01 after fallback timeout");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn turn_prompt_34_01_precedes_command_and_turn_finish_32_02() {
    let (battle, sink) = create_test_battle();
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::<i64, Npc>::new()),
        Arc::new(HashMap::<i64, Skill>::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(500),
        sink.clone(),
    );

    // Ack scene
    handle.ack_scene();

    // Wait for 34 01 to arrive
    let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames.iter().any(|(p, f)| *p == Some(100) && f == &packets::your_turn()) {
                break;
            }
        }
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // Submit normal attack command via handle
    handle.command(ts_dream::battle::manager::PlayerInput {
        player: 100,
        cmd: BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::NormalAttack,
        },
    });

    // Wait for turn actions to complete
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames.iter().any(|(_, f)| f == &packets::turn_action_finish()) {
                break;
            }
        }
        assert!(tokio::time::Instant::now() < deadline, "expected 32 02 turn_action_finish");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Verify ordering: 34 01 index < 32 02 index
    let frames = sink.frames.lock().unwrap();
    let idx_3401 = frames
        .iter()
        .position(|(p, f)| *p == Some(100) && f == &packets::your_turn())
        .expect("34 01 must be present");
    let idx_3202 = frames
        .iter()
        .position(|(_, f)| f == &packets::turn_action_finish())
        .expect("32 02 must be present");
    assert!(idx_3401 < idx_3202, "34 01 must strictly precede 32 02 in turn lifecycle");
}

#[tokio::test]
async fn timeout_fallback_assigns_normal_attack() {
    let (battle, sink) = create_test_battle();
    let manager = Arc::new(BattleManager::new());
    // Short 50ms timeout so fallback triggers promptly
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::<i64, Npc>::new()),
        Arc::new(HashMap::<i64, Skill>::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(50),
        sink.clone(),
    );

    handle.ack_scene();

    // Do NOT submit any command. The engine should timeout and auto-assign NormalAttack (10000).
    // The enemy has 50 HP, player has 50 ATK vs 10 DEF, enemy dies in 1-2 turns.
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1500);
    loop {
        if sink.ended.lock().unwrap().is_some() {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "battle should end by timeout auto-attack");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let ended = sink.ended.lock().unwrap();
    assert!(
        matches!(*ended, Some((_, Outcome::PlayerWin))),
        "timeout auto-attack should defeat the enemy, got {:?}",
        *ended
    );
}

#[tokio::test]
async fn turn1_handshake_barrier_releases_on_immediate_ack() {
    let (battle, sink) = create_test_battle();
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::<i64, Npc>::new()),
        Arc::new(HashMap::<i64, Skill>::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(200),
        sink.clone(),
    );

    // Immediate ack without sleep - must not miss the notification (notify_one permit)
    handle.ack_scene();

    let deadline = tokio::time::Instant::now() + Duration::from_millis(100);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames.iter().any(|(p, f)| *p == Some(100) && f == &packets::your_turn()) {
                break;
            }
        }
        assert!(tokio::time::Instant::now() < deadline, "expected 34 01 promptly on immediate ack");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn turn_prompt_34_01_sent_to_player_when_character_is_dead_but_pet_is_alive() {
    let mut battle = Battle::new(1, 0);
    // Dead player at (3, 2)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 100;
        c.hp = 0;
        c.hp_max = 100;
        c.team = 1;
    }
    // Living pet at (2, 2)
    {
        let c = battle.cell_mut(2, 2).unwrap();
        c.typ = 4;
        c.id = 5001;
        c.id_char = 100;
        c.hp = 50;
        c.hp_max = 50;
        c.team = 1;
    }
    // Living enemy at (0, 2)
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.hp = 50;
        c.hp_max = 50;
        c.team = 2;
    }
    let sink = Arc::new(CapturingSink::default());
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(200),
        sink.clone(),
    );
    handle.ack_scene();

    // Verify player 100 receives 34 01 because their pet is alive and needs orders
    let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
    loop {
        {
            let frames = sink.frames.lock().unwrap();
            if frames.iter().any(|(p, f)| *p == Some(100) && f == &packets::your_turn()) {
                break;
            }
        }
        assert!(tokio::time::Instant::now() < deadline, "expected 34 01 for dead player with living pet");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn flee_command_executes_apply_flee_when_using_battle_action_flee() {
    let mut battle = Battle::new(1, 0);
    // High-level leader at (3, 2)
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 100;
        c.leader_id = 100;
        c.hp = 100;
        c.hp_max = 100;
        c.lv = 150;
        c.team = 1;
    }
    // Low-level enemy at (0, 2)
    {
        let c = battle.cell_mut(0, 2).unwrap();
        c.typ = 3;
        c.id = 1001;
        c.hp = 100;
        c.hp_max = 100;
        c.lv = 1;
        c.team = 2;
    }
    let sink = Arc::new(CapturingSink::default());
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        Duration::from_millis(500),
        sink.clone(),
    );
    handle.ack_scene();

    // Submit Flee command
    handle.command(ts_dream::battle::manager::PlayerInput {
        player: 100,
        cmd: BattleCommand {
            row: 3,
            col: 2,
            target_row: 0,
            target_col: 2,
            action: BattleAction::Flee,
        },
    });

    let deadline = tokio::time::Instant::now() + Duration::from_millis(1500);
    loop {
        if sink.ended.lock().unwrap().is_some() {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "battle should end with flee");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let ended = sink.ended.lock().unwrap();
    assert!(
        matches!(*ended, Some((_, Outcome::PlayerFled))),
        "BattleAction::Flee should resolve to PlayerFled, got {:?}",
        *ended
    );
}
