use std::collections::HashMap;
use std::sync::Mutex;
use ts_dream::server::auto_save::{forget, forget_player, global_ledger, SaveLedger};

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
