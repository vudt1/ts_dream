//! PlayerStateManager: thread-safe in-memory management of a player's dynamic
//! attributes — base stats, HP/SP and their maxima, and the equipment bonus.
//!
//! All access funnels through the shared online-session registry, so a manager
//! update is atomic with respect to the dispatcher's own session mutations and
//! to other cross-player flows (trade, player shop). The recompute path
//! delegates the gear-bonus math to `CharacterSheet` (the same formulas the
//! login/equip flows use) and clamps current HP/SP into the new maxima.

use crate::server::session::{lock_online_sessions, Session};

/// A snapshot of the derived combat stats after a recompute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedStats {
    pub hp_max: u16,
    pub sp_max: u16,
    pub int2: u32,
    pub atk2: u32,
    pub def2: u32,
    pub hpx2: u32,
    pub spx2: u32,
    pub agi2: u32,
}

impl DerivedStats {
    fn of(session: &Session) -> Self {
        Self {
            hp_max: session.hp_max,
            sp_max: session.sp_max,
            int2: session.int2,
            atk2: session.atk2,
            def2: session.def2,
            hpx2: session.hpx2,
            spx2: session.spx2,
            agi2: session.agi2,
        }
    }
}

/// Facade over the online-session registry for stat-centric mutations.
pub struct PlayerStateManager;

impl PlayerStateManager {
    /// Point-in-time clone of a player's session, or `None` when offline.
    pub fn snapshot(player_id: u32) -> Option<Session> {
        lock_online_sessions().get(&player_id).cloned()
    }

    /// Apply `f` to the live session under the registry lock. Returns `false`
    /// when the player is offline (f never runs).
    pub fn update(player_id: u32, f: impl FnOnce(&mut Session)) -> bool {
        let mut sessions = lock_online_sessions();
        match sessions.get_mut(&player_id) {
            Some(s) => {
                f(s);
                true
            }
            None => false,
        }
    }

    /// Recompute equipment bonus + HP/SP maxima from the player's equipped
    /// items and clamp HP/SP into range. Returns the derived stats, or `None`
    /// when offline.
    pub fn recompute_stats(player_id: u32) -> Option<DerivedStats> {
        if !Self::update(player_id, |s| s.recompute_stats()) {
            return None;
        }
        Self::derived_stats(player_id)
    }

    /// Read-only derived-stats view without recomputing.
    pub fn derived_stats(player_id: u32) -> Option<DerivedStats> {
        lock_online_sessions()
            .get(&player_id)
            .map(DerivedStats::of)
    }

    /// Damage/heal helper: adjusts HP within `[0, hp_max]`. Returns the applied
    /// new HP, or `None` when offline.
    pub fn adjust_hp(player_id: u32, delta: i32) -> Option<u16> {
        if !Self::update(player_id, |s| {
            let hp = i32::from(s.hp) + delta;
            s.hp = hp.clamp(0, i32::from(s.hp_max)) as u16;
        }) {
            return None;
        }
        lock_online_sessions()
            .get(&player_id)
            .map(|s| s.hp)
    }

    /// Same as [`adjust_hp`] for SP.
    pub fn adjust_sp(player_id: u32, delta: i32) -> Option<u16> {
        if !Self::update(player_id, |s| {
            let sp = i32::from(s.sp) + delta;
            s.sp = sp.clamp(0, i32::from(s.sp_max)) as u16;
        }) {
            return None;
        }
        lock_online_sessions()
            .get(&player_id)
            .map(|s| s.sp)
    }
}
