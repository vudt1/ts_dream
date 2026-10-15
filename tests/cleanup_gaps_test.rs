use std::collections::HashMap;
use std::sync::Mutex;
use ts_dream::server::auto_save::{forget, forget_player, global_ledger, SaveLedger};
use ts_dream::server::session::{lock_online_sessions, Session};

#[test]
fn test_save_ledger_forget_player() {
    let ledger: SaveLedger = Mutex::new(HashMap::new());
    let player_id = 999999;

    // Simulate recording saves in instance ledger
    ledger.lock().unwrap().insert(player_id, 123456789);
    assert!(ledger.lock().unwrap().contains_key(&player_id));

    // Test forget on instance
    forget(&ledger, player_id);
    assert!(!ledger.lock().unwrap().contains_key(&player_id));

    // Simulate in global ledger
    global_ledger().lock().unwrap().insert(player_id, 987654321);
    assert!(global_ledger().lock().unwrap().contains_key(&player_id));

    // Global forget_player helper cleans global ledger
    forget_player(player_id);
    assert!(!global_ledger().lock().unwrap().contains_key(&player_id));
}

#[test]
fn test_online_sessions_dynamic() {
    let player_id = 888123;

    // Insert a dummy session into the global online sessions map
    let mut session = Session::default();
    session.id = player_id;
    session.name = b"TestPlayer".to_vec();
    session.map_id = 12001;
    session.map_x = 500;
    session.map_y = 400;
    session.hp = 100;
    session.hp_max = 100;
    session.sp = 50;
    session.sp_max = 50;
    session.level = 10;

    lock_online_sessions().insert(player_id, session);

    {
        let sessions = lock_online_sessions();
        assert!(sessions.contains_key(&player_id));
        let sess = sessions.get(&player_id).unwrap();
        assert_eq!(sess.name, b"TestPlayer");
        assert_eq!(sess.map_id, 12001);
        assert_eq!(sess.map_x, 500);
        assert_eq!(sess.map_y, 400);
    }

    // Clean up
    lock_online_sessions().remove(&player_id);
    assert!(!lock_online_sessions().contains_key(&player_id));
}
