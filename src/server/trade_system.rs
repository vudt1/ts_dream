//! TradeSystem: domain system for two-player item / pet trades.
//!
//! Two halves:
//! - The pure settlement engine ([`settle_trade`], [`pet_trade_settle`]) moved
//!   out of the wire handler so it is unit-testable and reusable.
//! - A cross-session coordinator ([`TradeSystem`]) that drives the shared
//!   online-session registry atomically: both players are locked in stable id
//!   order via [`lock_player_operations`], offers are validated against the
//!   live bags (an offer whose items vanished is rejected), and settlement is
//!   probe-first so a full bag on either side rolls the whole exchange back.

use crate::server::inventory;
use crate::server::session::{
    lock_online_sessions, lock_player_operations, InventoryItem, PetState, Session, TradeState,
};

/// Maximum gold either side may hold after a trade settles.
pub(crate) const GOLD_CAP: u32 = 9_999_999;

/// Remove every offered item from `s`'s bag. Verifies each offer still exists
/// in sufficient quantity first; any missing offer fails the whole removal
/// without partial mutation (anti-duplicate guard).
pub(crate) fn remove_offers(s: &mut Session) -> bool {
    for offered in &s.trade.items {
        if !s
            .homdo
            .iter()
            .any(|i| i.slot == offered.slot && i.id == offered.id && i.count >= offered.count)
        {
            return false;
        }
    }
    for offered in s.trade.items.clone() {
        if let Some(pos) = s.homdo.iter().position(|i| i.slot == offered.slot) {
            if s.homdo[pos].count == offered.count {
                s.homdo.remove(pos);
            } else {
                s.homdo[pos].count -= offered.count;
            }
        }
    }
    true
}

/// Place one traded item into a session's bag. Equip-class items always take a
/// fresh slot; stackables merge when possible. Fails cleanly on a full bag.
pub(crate) fn add_trade_item(s: &mut Session, mut item: InventoryItem) -> bool {
    if (1..=6).contains(&item.loai) {
        let Some(slot) = inventory::free_slot(&s.homdo) else {
            return false;
        };
        item.slot = slot;
        s.homdo.push(item);
        true
    } else {
        inventory::can_add_item(&s.homdo, &item)
            && !inventory::add_item(&mut s.homdo, item).is_empty()
    }
}

/// Atomic two-sided item+gold settlement. Both sides are cloned into probes;
/// only when every removal, insertion, and the gold cap check pass do the
/// probes replace the real sessions. Either failure leaves both untouched.
pub(crate) fn settle_trade(a: &mut Session, b: &mut Session) -> bool {
    if a.trade.gold > a.gold || b.trade.gold > b.gold {
        return false;
    }
    let mut a_probe = a.clone();
    let mut b_probe = b.clone();
    if !remove_offers(&mut a_probe) || !remove_offers(&mut b_probe) {
        return false;
    }
    for item in a.trade.items.clone() {
        if !add_trade_item(&mut b_probe, item) {
            return false;
        }
    }
    for item in b.trade.items.clone() {
        if !add_trade_item(&mut a_probe, item) {
            return false;
        }
    }
    a_probe.gold = a_probe.gold - a.trade.gold + b.trade.gold;
    b_probe.gold = b_probe.gold - b.trade.gold + a.trade.gold;
    if a_probe.gold > GOLD_CAP || b_probe.gold > GOLD_CAP {
        return false;
    }
    a_probe.trade = TradeState::default();
    b_probe.trade = TradeState::default();
    *a = a_probe;
    *b = b_probe;
    true
}

pub(crate) fn pet_at(s: &Session, stt: u8) -> Option<PetState> {
    s.pets.iter().find(|p| p.stt == stt && p.id > 0).cloned()
}

/// Pet-trade settlement with duplicate-pet rejection (`Err("duplicate")`),
/// capacity guards (`Err("full")`) and the same gold-cap rules as items.
pub(crate) fn pet_trade_settle(a: &mut Session, b: &mut Session) -> Result<(), &'static str> {
    if a.trade.gold > a.gold || b.trade.gold > b.gold {
        return Err("gold");
    }
    let a_stt = a.trade.pets.first().copied().unwrap_or(0);
    let b_stt = b.trade.pets.first().copied().unwrap_or(0);
    let ap = if a_stt > 0 { pet_at(a, a_stt) } else { None };
    let bp = if b_stt > 0 { pet_at(b, b_stt) } else { None };
    if ap
        .as_ref()
        .is_some_and(|p| b.pets.iter().any(|x| x.id == p.id))
        || bp
            .as_ref()
            .is_some_and(|p| a.pets.iter().any(|x| x.id == p.id))
    {
        return Err("duplicate");
    }
    if ap.is_some() && b.pets.len() >= 8 || bp.is_some() && a.pets.len() >= 8 {
        return Err("full");
    }
    if let Some(p) = ap {
        a.pets.retain(|x| x.stt != a_stt);
        let mut p = p;
        p.stt = crate::server::pet_box::next_active_slot(&b.pets)
            .or_else(|| crate::server::pet_box::next_stable_slot(&b.pets))
            .ok_or("full")?;
        b.pets.push(p);
    }
    if let Some(p) = bp {
        b.pets.retain(|x| x.stt != b_stt);
        let mut p = p;
        p.stt = crate::server::pet_box::next_active_slot(&a.pets)
            .or_else(|| crate::server::pet_box::next_stable_slot(&a.pets))
            .ok_or("full")?;
        a.pets.push(p);
    }
    a.gold = a.gold - a.trade.gold + b.trade.gold;
    b.gold = b.gold - b.trade.gold + a.trade.gold;
    if a.gold > GOLD_CAP || b.gold > GOLD_CAP {
        return Err("gold");
    }
    a.trade = TradeState::default();
    b.trade = TradeState::default();
    Ok(())
}

/// Outcome of a coordinated [`TradeSystem`] operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeOutcome {
    /// Both sessions were mutated to completion.
    Settled,
    /// The exchange was rejected; both sessions are untouched.
    Rejected,
    /// One or both players are offline; nothing was done.
    Offline,
}

/// Cross-session trade coordinator over the shared online-session registry.
///
/// Every operation takes the per-player locks in stable id order, so two
/// concurrent trades can never interleave their registry reads/writes, and
/// settlement is all-or-nothing against duplicate items.
pub struct TradeSystem;

impl TradeSystem {
    /// Open a trade request from `initiator` to `partner`. Succeeds only when
    /// both players are online, distinct, and neither is already trading.
    pub async fn begin(initiator: u32, partner: u32) -> TradeOutcome {
        if initiator == partner {
            return TradeOutcome::Rejected;
        }
        let _locks = lock_player_operations([initiator, partner]).await;
        let mut sessions = lock_online_sessions();
        let partner_free = sessions.get(&partner).map(|p| !p.trade.active);
        let initiator_free = sessions.get(&initiator).map(|a| !a.trade.active);
        let Some(partner_free) = partner_free else {
            return TradeOutcome::Offline;
        };
        let Some(initiator_free) = initiator_free else {
            return TradeOutcome::Offline;
        };
        if !partner_free || !initiator_free {
            return TradeOutcome::Rejected;
        }
        if let Some(a) = sessions.get_mut(&initiator) {
            a.trade = TradeState {
                active: true,
                partner_id: partner,
                ..TradeState::default()
            };
        }
        if let Some(p) = sessions.get_mut(&partner) {
            p.trade = TradeState {
                active: true,
                partner_id: initiator,
                ..TradeState::default()
            };
        }
        TradeOutcome::Settled
    }

    /// Tear down an active trade involving `player`, clearing both sides
    /// (cancel path: rejection, disconnect, timeout).
    pub async fn cancel(player: u32) -> TradeOutcome {
        let _locks = lock_player_operations([player]).await;
        let (partner_id, snapshot) = {
            let mut sessions = lock_online_sessions();
            let Some(s) = sessions.get_mut(&player) else {
                return TradeOutcome::Offline;
            };
            if !s.trade.active {
                return TradeOutcome::Rejected;
            }
            let pid = s.trade.partner_id;
            s.trade = TradeState::default();
            (pid, s.clone())
        };
        let mut sessions = lock_online_sessions();
        if let Some(p) = sessions.get_mut(&partner_id) {
            if p.trade.partner_id == player && p.trade.active {
                p.trade = TradeState::default();
            }
        }
        drop(sessions);
        let _ = snapshot;
        TradeOutcome::Settled
    }

    /// Attempt final settlement of the trade `player` belongs to. Both players
    /// must be online and have accepted; the pure engine decides whether the
    /// exchange applies. On success both sessions carry the swapped goods and
    /// cleared trade state; on [`TradeOutcome::Rejected`] both are untouched
    /// except that the trade state is torn down (the wire layer reports the
    /// failure frames).
    pub async fn accept(player: u32) -> (TradeOutcome, Option<Session>, Option<Session>) {
        let _locks = lock_player_operations([player]).await;
        let (partner_id, a_snapshot, b_accepted, b_snapshot) = {
            let mut sessions = lock_online_sessions();
            let Some(a) = sessions.get_mut(&player) else {
                return (TradeOutcome::Offline, None, None);
            };
            if !a.trade.active {
                return (TradeOutcome::Rejected, None, None);
            }
            a.trade.accepted = true;
            let pid = a.trade.partner_id;
            let a_snap = a.clone();
            let b_read = sessions.get(&pid);
            match b_read {
                None => {
                    // Partner vanished: roll the acceptance flag back.
                    if let Some(a) = sessions.get_mut(&player) {
                        a.trade.accepted = false;
                    }
                    return (TradeOutcome::Offline, None, None);
                }
                Some(b) => (pid, a_snap, b.trade.accepted, Some(b.clone())),
            }
        };
        let Some(b_snapshot) = b_snapshot else {
            return (TradeOutcome::Offline, None, None);
        };
        if !b_accepted {
            return (TradeOutcome::Rejected, Some(a_snapshot), Some(b_snapshot));
        }
        // Probe-settle on the snapshots first so the registry is never left
        // half-mutated by a failed engine pass.
        let mut a = a_snapshot.clone();
        let mut b = b_snapshot.clone();
        if settle_trade(&mut a, &mut b) {
            {
                let mut sessions = lock_online_sessions();
                sessions.insert(a.id, a.clone());
                sessions.insert(b.id, b.clone());
            }
            return (TradeOutcome::Settled, Some(a), Some(b));
        }
        // Rejected: tear down the trade state on both sides but keep bags/gold.
        {
            let mut sessions = lock_online_sessions();
            if let Some(a) = sessions.get_mut(&player) {
                a.trade = TradeState::default();
            }
            if let Some(b) = sessions.get_mut(&partner_id) {
                b.trade = TradeState::default();
            }
        }
        (TradeOutcome::Rejected, Some(a_snapshot), Some(b_snapshot))
    }

    /// Whether `player` currently has an active trade session.
    pub fn is_trading(player: u32) -> bool {
        lock_online_sessions()
            .get(&player)
            .is_some_and(|s| s.trade.active)
    }
}
