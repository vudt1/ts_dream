//! AutoSaveService: background task that batch-persists dirty sessions.
//!
//! Every cycle (default 3 minutes, mirroring the legacy auto-save cadence) the
//! service scans the online-session registry, fingerprints each authed
//! session's volatile state (gold, HP/SP, stats, bag/equipment rows, pets,
//! quest steps), and writes the sessions whose fingerprint changed since the
//! last successful save. Fingerprinting avoids both write amplification (a
//! clean player costs one hash, zero queries) and the invasiveness of
//! sprinkling dirty flags through every handler mutation site.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::db::persist;
use crate::db::pool::DbPool;
use crate::server::session::{lock_online_sessions, Session};

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
    mix_i64(&mut h, i64::from(s.map_id));
    mix_i64(&mut h, i64::from(s.map_x));
    mix_i64(&mut h, i64::from(s.map_y));
    mix(&mut h, u64::from(s.gold));
    mix(&mut h, u64::from(s.bank_gold));
    mix(&mut h, u64::from(s.hp));
    mix(&mut h, u64::from(s.hp_max));
    mix(&mut h, u64::from(s.sp));
    mix(&mut h, u64::from(s.sp_max));
    mix(&mut h, u64::from(s.texp));
    mix(&mut h, u64::from(s.level));
    mix(&mut h, u64::from(s.job));
    mix(&mut h, u64::from(s.sex));
    mix(&mut h, u64::from(s.hair));
    mix(&mut h, u64::from(s.thuoctinh));
    mix(&mut h, u64::from(s.reborn));
    mix(&mut h, u64::from(s.point));
    mix(&mut h, u64::from(s.skill_point));
    mix(&mut h, u64::from(s.int1));
    mix(&mut h, u64::from(s.atk));
    mix(&mut h, u64::from(s.def));
    mix(&mut h, u64::from(s.hpx));
    mix(&mut h, u64::from(s.spx));
    mix(&mut h, u64::from(s.agi));
    mix(&mut h, u64::from(s.tiengtam));
    mix(&mut h, u64::from(s.pk));
    mix(&mut h, u64::from(s.newbie));
    mix(&mut h, u64::from(s.ghost));
    mix(&mut h, u64::from(s.god));
    mix(&mut h, u64::from(s.guild_id));
    mix(&mut h, u64::from(s.shop_point));
    mix(&mut h, u64::from(s.savemap));
    mix(&mut h, u64::from(s.savemap_x));
    mix(&mut h, u64::from(s.savemap_y));
    mix(&mut h, u64::from(s.active_pet_stt));
    for (stype, container) in s.containers() {
        mix_i64(&mut h, i64::from(stype as u8));
        for item in container {
            if item.id == 0 {
                continue;
            }
            mix_i64(&mut h, i64::from(item.slot));
            mix_i64(&mut h, i64::from(item.id));
            mix_i64(&mut h, i64::from(item.count));
            mix_i64(&mut h, i64::from(item.doben));
            mix_i64(&mut h, i64::from(item.long_val));
            mix_i64(&mut h, i64::from(item.giatri_long));
            mix_i64(&mut h, i64::from(item.khang));
            mix(&mut h, u64::from(item.texp));
        }
    }
    for pet in &s.pets {
        if pet.id == 0 {
            continue;
        }
        mix_i64(&mut h, i64::from(pet.storage_type.value()));
        mix_i64(&mut h, i64::from(pet.stt));
        mix_i64(&mut h, i64::from(pet.id));
        for &b in &pet.name {
            mix(&mut h, u64::from(b));
        }
        mix_i64(&mut h, i64::from(pet.level));
        mix_i64(&mut h, i64::from(pet.thuoctinh));
        mix_i64(&mut h, i64::from(pet.reborn));
        mix(&mut h, u64::from(pet.hp));
        mix(&mut h, u64::from(pet.hp_max));
        mix(&mut h, u64::from(pet.sp));
        mix(&mut h, u64::from(pet.sp_max));
        mix(&mut h, u64::from(pet.int1));
        mix(&mut h, u64::from(pet.atk));
        mix(&mut h, u64::from(pet.def));
        mix(&mut h, u64::from(pet.hpx));
        mix(&mut h, u64::from(pet.spx));
        mix(&mut h, u64::from(pet.agi));
        mix(&mut h, u64::from(pet.fai));
        mix(&mut h, u64::from(pet.thd));
        mix(&mut h, u64::from(pet.texp));
        mix(&mut h, u64::from(pet.skill_point));
        mix(&mut h, u64::from(pet.quest));
        for &(skill_id, skill_lv) in &pet.skills {
            mix(&mut h, u64::from(skill_id));
            mix(&mut h, u64::from(skill_lv));
        }
    }
    for skill in &s.skills {
        mix_i64(&mut h, i64::from(skill.0));
        mix_i64(&mut h, i64::from(skill.1));
    }
    for (slot, entry) in s.hotkeys.iter().enumerate().skip(1) {
        if entry.kind > 0 && entry.id > 0 {
            mix(&mut h, slot as u64);
            mix(&mut h, u64::from(entry.kind));
            mix(&mut h, u64::from(entry.id));
        }
    }
    // Eve/quest-log state persisted by `save_eve_state_tx`. HashMap/HashSet
    // iteration order is unstable, so every collection is sorted by key first
    // — otherwise an unchanged session would fingerprint differently every pass.
    let mut quest_tasks: Vec<(u16, (u8, u8))> =
        s.quest_tasks.iter().map(|(&k, &v)| (k, v)).collect();
    quest_tasks.sort_unstable();
    for (quest_id, (slot, step)) in quest_tasks {
        mix_i64(&mut h, i64::from(quest_id));
        mix_i64(&mut h, i64::from(slot));
        mix_i64(&mut h, i64::from(step));
    }
    let mut quest_dont: Vec<u16> = s.quest_dont.iter().copied().collect();
    quest_dont.sort_unstable();
    for mark in quest_dont {
        mix_i64(&mut h, i64::from(mark));
    }
    let mut bit_flags: Vec<u32> = s.bit_flags.iter().copied().collect();
    bit_flags.sort_unstable();
    for flag in bit_flags {
        mix(&mut h, u64::from(flag));
    }
    let mut eve_counts: Vec<((u16, i32), i32)> = s
        .completed_eve_counts
        .iter()
        .map(|(&key, &count)| (key, count))
        .collect();
    eve_counts.sort_unstable();
    for ((scene_id, eve_no), count) in eve_counts {
        mix(&mut h, u64::from(scene_id));
        mix_i64(&mut h, i64::from(eve_no));
        mix_i64(&mut h, i64::from(count));
    }
    h
}

/// Last-known-persisted fingerprints, keyed by player id.
pub type SaveLedger = Mutex<HashMap<u32, u64>>;

/// One save pass over the registry. Returns how many sessions were written.
/// `pool == None` (protocol replay / DB-less boot) is a no-op success.
pub async fn run_cycle(pool: Option<&DbPool>, ledger: &SaveLedger) -> usize {
    let Some(pool) = pool else {
        return 0;
    };
    // Snapshot phase: clone candidates out of the registry so the lock is held
    // only briefly and DB I/O never blocks the dispatcher.
    let mut candidates: Vec<Session> = Vec::new();
    let mut fingerprints: Vec<u64> = Vec::new();
    {
        let last = ledger.lock().unwrap();
        let sessions = lock_online_sessions();
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

/// Persist all active authed sessions to the database immediately.
/// Typically called right before shutting down the server or during graceful exit.
pub async fn save_all_dirty(pool: &DbPool) -> usize {
    let mut candidates: Vec<Session> = Vec::new();
    {
        let sessions = lock_online_sessions();
        for s in sessions.values() {
            if !s.authed || s.id == 0 {
                continue;
            }
            candidates.push(s.clone());
        }
    }
    if candidates.is_empty() {
        return 0;
    }
    let refs: Vec<&Session> = candidates.iter().collect();
    if persist::persist_sessions_transaction(Some(pool), &refs, &SAVE_TABLES).await {
        tracing::info!("save_all_dirty saved {} session(s)", candidates.len());
        candidates.len()
    } else {
        tracing::warn!("save_all_dirty transaction failed");
        0
    }
}

static GLOBAL_SAVE_LEDGER: std::sync::OnceLock<std::sync::Arc<SaveLedger>> =
    std::sync::OnceLock::new();

pub fn global_ledger() -> &'static std::sync::Arc<SaveLedger> {
    GLOBAL_SAVE_LEDGER.get_or_init(|| std::sync::Arc::new(SaveLedger::default()))
}

/// Forget a player's ledger entry (disconnect path keeps the map bounded).
pub fn forget(ledger: &SaveLedger, player_id: u32) {
    ledger.lock().unwrap().remove(&player_id);
}

/// Forget a player from the global ledger on disconnect.
pub fn forget_player(player_id: u32) {
    if let Some(ledger) = GLOBAL_SAVE_LEDGER.get() {
        ledger.lock().unwrap().remove(&player_id);
    }
}

/// Spawn the recurring background task. Called once at server boot with the
/// live pool; the task lives for the process lifetime.
pub fn spawn(pool: DbPool) {
    let ledger = global_ledger().clone();
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

