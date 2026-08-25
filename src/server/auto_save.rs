//! AutoSaveService: background task that batch-persists dirty sessions.
//!
//! Every cycle (default 3 minutes, mirroring the C# auto-save cadence) the
//! service scans the online-session registry, fingerprints each authed
//! session's volatile state (gold, HP/SP, stats, bag/equipment rows, pets,
//! quest steps), and writes the sessions whose fingerprint changed since the
//! last successful save. Fingerprinting avoids both write amplification (a
//! clean player costs one hash, zero queries) and the invasiveness of
//! sprinkling dirty flags through every handler mutation site.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use sqlx::MySqlPool;

use crate::db::persist;
use crate::server::session::{online_sessions, Session};

/// Save cadence: 3 minutes (spec §3 AutoSaveService).
pub const AUTO_SAVE_INTERVAL: Duration = Duration::from_secs(180);

/// Tables rewritten for a saved session: full volatile snapshot.
const SAVE_TABLES: [&str; 5] = ["stats", "homdo", "trangbi", "quest", "pet"];

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

fn mix(h: &mut u64, value: u64) {
    *h ^= value;
    *h = h.wrapping_mul(0x0000_0100_0000_01b3);
}

fn mix_i64(h: &mut u64, value: i64) {
    mix(h, value as u64);
}

/// Stable fingerprint of every persisted-volatile field of a session. Any
/// handler mutation of gold, stats, items, pets, or quest steps changes it.
pub fn fingerprint(s: &Session) -> u64 {
    let mut h = FNV_OFFSET;
    mix(&mut h, u64::from(s.id));
    mix(&mut h, u64::from(s.gold));
    mix(&mut h, u64::from(s.bank_gold));
    mix(&mut h, u64::from(s.hp));
    mix(&mut h, u64::from(s.sp));
    mix(&mut h, u64::from(s.texp));
    mix(&mut h, u64::from(s.level));
    mix(&mut h, u64::from(s.point));
    mix(&mut h, u64::from(s.skill_point));
    mix(&mut h, u64::from(s.god));
    mix(&mut h, u64::from(s.shop_point));
    mix(&mut h, u64::from(s.savemap));
    for item in s.homdo.iter().chain(s.trangbi.iter()) {
        if item.id == 0 {
            continue;
        }
        mix_i64(&mut h, i64::from(item.slot));
        mix_i64(&mut h, i64::from(item.id));
        mix_i64(&mut h, i64::from(item.count));
        mix_i64(&mut h, i64::from(item.doben));
        mix_i64(&mut h, i64::from(item.long_val));
        mix_i64(&mut h, i64::from(item.khang));
        mix(&mut h, u64::from(item.texp));
    }
    for pet in &s.pets {
        if pet.id == 0 {
            continue;
        }
        mix_i64(&mut h, i64::from(pet.stt));
        mix_i64(&mut h, i64::from(pet.id));
        mix_i64(&mut h, i64::from(pet.level));
        mix(&mut h, u64::from(pet.hp));
        mix(&mut h, u64::from(pet.texp));
        mix_i64(&mut h, i64::from(pet.reborn));
    }
    for skill in &s.skills {
        mix_i64(&mut h, i64::from(skill.0));
        mix_i64(&mut h, i64::from(skill.1));
    }
    for (npc, step) in &s.quest_steps {
        mix_i64(&mut h, *npc);
        mix_i64(&mut h, *step);
    }
    h
}

/// Last-known-persisted fingerprints, keyed by player id.
pub type SaveLedger = Mutex<HashMap<u32, u64>>;

/// One save pass over the registry. Returns how many sessions were written.
/// `pool == None` (protocol replay / DB-less boot) is a no-op success.
pub async fn run_cycle(pool: Option<&MySqlPool>, ledger: &SaveLedger) -> usize {
    let Some(pool) = pool else {
        return 0;
    };
    // Snapshot phase: clone candidates out of the registry so the lock is held
    // only briefly and DB I/O never blocks the dispatcher.
    let mut candidates: Vec<Session> = Vec::new();
    let mut fingerprints: Vec<u64> = Vec::new();
    {
        let last = ledger.lock().unwrap();
        let sessions = online_sessions().lock().unwrap();
        for s in sessions.values() {
            if !s.authed || s.id == 0 {
                continue;
            }
            let fp = fingerprint(s);
            if last.get(&s.id).copied() != Some(fp) {
                candidates.push(s.clone());
                fingerprints.push(fp);
            }
        }
    }
    if candidates.is_empty() {
        return 0;
    }
    let refs: Vec<&Session> = candidates.iter().collect();
    if persist::persist_sessions_transaction(Some(pool), &refs, &SAVE_TABLES).await {
        let mut last = ledger.lock().unwrap();
        for (s, fp) in candidates.iter().zip(fingerprints) {
            last.insert(s.id, fp);
        }
        tracing::debug!("auto-save wrote {} session(s)", candidates.len());
        candidates.len()
    } else {
        tracing::warn!("auto-save transaction failed; will retry next cycle");
        0
    }
}

/// Forget a player's ledger entry (disconnect path keeps the map bounded).
pub fn forget(ledger: &SaveLedger, player_id: u32) {
    ledger.lock().unwrap().remove(&player_id);
}

/// Spawn the recurring background task. Called once at server boot with the
/// live pool; the task lives for the process lifetime.
pub fn spawn(pool: MySqlPool) {
    let ledger: std::sync::Arc<SaveLedger> = std::sync::Arc::default();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(AUTO_SAVE_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await; // first tick fires immediately; skip it
        loop {
            ticker.tick().await;
            run_cycle(Some(&pool), &ledger).await;
        }
    });
}
