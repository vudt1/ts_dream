//! Battle service (ticket 21) — the server-side seam between the synchronous
//! opcode handlers and the async per-battle tasks.
//!
//! Owns the [`BattleManager`], the static [`GameData`] tables, and the online
//! player registry. Implements [`BattleSink`] so every `runner::Out` reaches
//! the right player: frames go into a per-player unbounded channel (fed to the
//! socket by the caller), DB updates mutate the shared `Session` and push the
//! `F4440C000801` status frame, drops/catches/pet-exp update the session.
//!
//! Battle lifecycle:
//! - `start_npc_battle` / `start_pk_battle` / `start_teamdef_battle` build the
//!   grid, `manager.spawn` it, register the participants, and push the
//!   `0BFA` start frames.
//! - op 0x0B sub 3/4 handlers call `submit_command`.
//! - when the task ends, `battle_ended` runs the quest-win progression for the leader if
//!   a quest talk is pending and the players won, then cleans up.

use crate::battle::construction::{Battle, StartPacket};
use crate::battle::engine::get_hp_max;
use crate::battle::manager::{BattleHandle, BattleManager, BattleSink};
use crate::battle::npc_world::NpcWorld;
use crate::battle::npc_world::{WorldPlayer, WorldSink};
use crate::battle::packets;
use crate::battle::runner::{
    BattleCommand, DbTarget, DbUpdate, Out, Outcome, PlayerSnapshot, Stat,
};
use crate::data::loader::GameData;
use crate::data::tables::TexpRow;
use crate::protocol::{encoder, GOLD_CAP};
use crate::server::handlers::quest::BattleTrigger;
use crate::server::session::Session;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// A registered online player: their shared session + frame channel.
pub struct OnlinePlayer {
    pub session: Arc<tokio::sync::RwLock<Session>>,
    pub frames: mpsc::Sender<String>,
}

impl std::fmt::Debug for OnlinePlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnlinePlayer")
            .field("session", &self.session)
            .finish_non_exhaustive()
    }
}

type OnlineMap = HashMap<i64, OnlinePlayer>;

/// Helper to acquire read lock on `OnlineMap` with backoff retries.
fn try_read_online_with_retry(
    online_lock: &tokio::sync::RwLock<OnlineMap>,
    max_retries: usize,
    retry_delay: std::time::Duration,
) -> Option<tokio::sync::RwLockReadGuard<'_, OnlineMap>> {
    for attempt in 0..max_retries {
        if let Ok(guard) = online_lock.try_read() {
            return Some(guard);
        }
        if attempt + 1 < max_retries {
            std::thread::yield_now();
            std::thread::sleep(retry_delay);
        }
    }
    None
}

/// Helper to acquire write lock on `Session` with backoff retries.
fn try_write_session_with_retry(
    session_lock: &tokio::sync::RwLock<Session>,
    max_retries: usize,
    retry_delay: std::time::Duration,
) -> Option<tokio::sync::RwLockWriteGuard<'_, Session>> {
    for attempt in 0..max_retries {
        if let Ok(guard) = session_lock.try_write() {
            return Some(guard);
        }
        if attempt + 1 < max_retries {
            std::thread::yield_now();
            std::thread::sleep(retry_delay);
        }
    }
    None
}

/// The [`BattleSink`] implementation shared by all battles of one service.
struct BattleSinkImpl {
    online: Arc<tokio::sync::RwLock<OnlineMap>>,
    /// Player ids currently in a battle (receive `broadcast`/`Out::Broadcast`).
    members: Arc<Mutex<HashSet<i64>>>,
    data: Arc<GameData>,
    /// Optional DB pool used for post-battle persistence (quest/homdo).
    /// Shared (same Arc) with the owning [`BattleService`] so `with_pool` can
    /// set it after construction, before any battle starts.
    pool: Arc<tokio::sync::RwLock<Option<crate::db::pool::DbPool>>>,
    /// Weak back-reference to the owning service, installed by
    /// [`BattleService::install_eve_backref`] right after construction. It
    /// lets `battle_ended` start a *chained* Eve battle — the sink itself has
    /// no battle-spawn machinery. Unset (bare service instances in tests) =
    /// the chained request is logged and skipped, never left parked.
    eve_service: std::sync::OnceLock<std::sync::Weak<BattleService>>,
    pub active_40_npc: Arc<Mutex<HashMap<i32, u32>>>,
    /// Participant player IDs recorded per `battle_id` at spawn/join so
    /// `battle_ended` still knows who participated even if a player disconnects.
    battle_participants: Arc<Mutex<HashMap<i32, Vec<i64>>>>,
    /// Parked Eve sessions (`phase == AwaitingBattle`) snapshotted when a player
    /// disconnects or unregisters mid-battle, keyed by `(battle_id, player_id)`.
    offline_eve_sessions: Arc<Mutex<HashMap<(i32, i64), Session>>>,
}

impl BattleSinkImpl {
    fn snapshot_offline_eve_session(&self, session: &Session) {
        if session.battle_id <= 0 {
            return;
        }
        let is_parked_eve = session
            .current_event_session
            .as_ref()
            .is_some_and(|ev| ev.phase == crate::eve::EventPhase::AwaitingBattle);
        if !is_parked_eve {
            return;
        }
        if let Ok(mut map) = self.offline_eve_sessions.lock() {
            map.insert((session.battle_id, i64::from(session.id)), session.clone());
        }
    }
}

impl BattleSink for BattleSinkImpl {
    fn send_to(&self, player: i64, frame: String) {
        push(&self.online, player, frame);
    }

    fn send_map(&self, player: i64, frame: String) {
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            for (id, p) in online.iter() {
                if *id != player {
                    let _ = p.frames.try_send(frame.clone());
                }
            }
        }
    }

    fn broadcast(&self, frame: String) {
        if let Ok(members) = self.members.lock() {
            for id in members.iter() {
                self.send_to(*id, frame.clone());
            }
        }
    }

    fn apply_db(&self, update: DbUpdate) {
        let online_opt = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2));
        if let Some(online) = online_opt {
            apply_db_update(&online, update);
        } else {
            let mut map = crate::server::session::lock_online_sessions();
            match update.target {
                DbTarget::Player(id) => {
                    if let Some(s) = map.get_mut(&(id as u32)) {
                        apply_player_stat(s, update.stat, update.value);
                    }
                }
                DbTarget::Pet { owner, stt } => {
                    if let Some(s) = map.get_mut(&(owner as u32)) {
                        if let Some(pet) = s.pets.iter_mut().find(|p| i64::from(p.stt) == stt) {
                            match update.stat {
                                Stat::Hp => pet.hp = clamp_u16(update.value),
                                Stat::Sp => pet.sp = clamp_u16(update.value),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn apply_drop(&self, drop: Out) {
        if let Out::Drop { item_id, owner, .. } = drop {
            let item = crate::server::session::InventoryItem {
                id: item_id as u16,
                count: 1,
                loai: 1,
                doben: 100,
                ..Default::default()
            };
            // The dropped item must actually reach the client bag: build the
            // single shared `17 06` add-item frame and send it to the owner.
            let frame = crate::server::inventory::item_added_frame(&item);
            if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
                if let Some(p) = online.get(&owner) {
                    if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                        s.add_homdo_item(item.clone());
                    }
                }
            }
            let mut map = crate::server::session::lock_online_sessions();
            if let Some(s) = map.get_mut(&(owner as u32)) {
                s.add_homdo_item(item);
            }
            self.send_to(owner, frame);
        }
    }

    fn apply_catch(&self, owner: i64, npc_id: i64) {
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&owner) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    add_pet_to_session(&mut s, npc_id as u16);
                }
            }
        }
        let mut map = crate::server::session::lock_online_sessions();
        if let Some(s) = map.get_mut(&(owner as u32)) {
            add_pet_to_session(s, npc_id as u16);
        }
    }

    fn apply_fled(&self, player: i64) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // 1. Remove player from self.members so they no longer receive in-battle frames
        if let Ok(mut members) = self.members.lock() {
            members.remove(&player);
        }

        let mut fled_session: Option<Session> = None;

        // 2. Update their session in self.online with retry
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&player) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    s.battle_id = 0;
                    s.last_battle_end_ms = now_ms;
                    s.encounter_steps = 0;
                    fled_session = Some(s.clone());
                    let mut map = crate::server::session::lock_online_sessions();
                    if let Some(global_s) = map.get_mut(&(player as u32)) {
                        *global_s = s.clone();
                    }
                }
            }
        }

        // 3. Update global online_sessions() fallback if not in self.online
        if fled_session.is_none() {
            let mut map = crate::server::session::lock_online_sessions();
            if let Some(s) = map.get_mut(&(player as u32)) {
                s.battle_id = 0;
                s.last_battle_end_ms = now_ms;
                s.encounter_steps = 0;
                fled_session = Some(s.clone());
            }
        }

        // 4. Persist fled session to DB so stats and pet status are saved
        if let Some(session) = fled_session {
            if let Ok(pool) = self.pool.try_read() {
                if let Some(pool) = pool.clone() {
                    tokio::spawn(async move {
                        crate::db::persist::persist_sessions_transaction(
                            Some(&pool),
                            &[&session],
                            &["stats", "pet"],
                        )
                        .await;
                    });
                }
            }
        }
    }

    fn apply_respawn(&self, npc_id: i64, map_id: i64, x: i64, y: i64) {
        // Map-wide respawn broadcast to every online client on the npc's
        // map: `F44406001603` + le16(id) + `0A00` +
        // `F44408001605` + le16(id) + le16(x) + le16(y), to every online
        // client on the npc's map.
        let frame = format!(
            "F44406001603{}0A00F44408001605{}{}{}",
            encoder::le16(npc_id as u16),
            encoder::le16(npc_id as u16),
            encoder::le16(x as u16),
            encoder::le16(y as u16)
        );
        send_to_map(&self.online, map_id, frame);
    }

    fn apply_pet_exp(&self, owner: i64, stt: i64, exp: i64) {
        // Ticket 07: the client only learns a pet's new EXP from the `0F 08`
        // record (reader `007a5010`), so build it while the updated session is
        // still in hand and push it to the owner afterwards.
        let mut frame: Option<String> = None;
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&owner) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    if let Some(pet) = s.pets.iter_mut().find(|p| i64::from(p.stt) == stt) {
                        pet.texp = pet.texp.saturating_add(exp as u32);
                    }
                    if let Some(pet) = s.pets.iter().find(|p| i64::from(p.stt) == stt) {
                        frame = Some(crate::server::spawn::pet_stat_frame(&s, pet));
                    }
                }
            }
        }
        let mut map = crate::server::session::lock_online_sessions();
        if let Some(s) = map.get_mut(&(owner as u32)) {
            if let Some(pet) = s.pets.iter_mut().find(|p| i64::from(p.stt) == stt) {
                pet.texp = pet.texp.saturating_add(exp as u32);
            }
            if frame.is_none() {
                if let Some(pet) = s.pets.iter().find(|p| i64::from(p.stt) == stt) {
                    frame = Some(crate::server::spawn::pet_stat_frame(s, pet));
                }
            }
        }
        drop(map);
        if let Some(frame) = frame {
            self.send_to(owner, frame);
        }
    }

    fn apply_gold(&self, owner: i64, amount: i64) {
        if amount <= 0 {
            return;
        }
        let grant = amount as u32;
        // The `1A 01` frame must carry the *actually credited* delta, so track
        // the pre/post balance and clamp to the client gold cap (9,999,999).
        let mut delta: u32 = 0;
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&owner) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    let before = s.gold;
                    s.gold = s.gold.saturating_add(grant).min(GOLD_CAP);
                    delta = s.gold - before;
                }
            }
        }
        let mut map = crate::server::session::lock_online_sessions();
        if let Some(s) = map.get_mut(&(owner as u32)) {
            let before = s.gold;
            s.gold = s.gold.saturating_add(grant).min(GOLD_CAP);
            if delta == 0 {
                delta = s.gold - before;
            }
        }
        drop(map);
        // The client learns about the balance change from this frame alone
        // (`F44406001A01{le32(delta)}`); the session balance was updated above.
        if delta > 0 {
            self.send_to(owner, packets::gold_gain(delta));
        }
    }

    fn battle_ended(&self, id: i32, outcome: Outcome) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let arena_40_challenger = self.active_40_npc.lock().unwrap().remove(&id);

        // 1. Gather all participant IDs for this battle (online + disconnected)
        //    and drain their parked offline Eve sessions if any.
        let mut participant_ids: Vec<i64> = Vec::new();
        let recorded_participants = self
            .battle_participants
            .lock()
            .ok()
            .and_then(|mut p| p.remove(&id));
        if let Some(ref rec) = recorded_participants {
            participant_ids.extend(rec.iter().copied());
        }

        if let Ok(mut members) = self.members.lock() {
            if let Some(ref rec) = recorded_participants {
                for pid in rec {
                    if members.remove(pid) {
                        participant_ids.push(*pid);
                    }
                }
            } else {
                participant_ids.extend(members.drain());
            }
        }

        let mut offline_parked: HashMap<i64, Session> = HashMap::new();
        if let Ok(mut offline_map) = self.offline_eve_sessions.lock() {
            let keys: Vec<(i32, i64)> = offline_map
                .keys()
                .copied()
                .filter(|&(bid, _)| bid == id)
                .collect();
            for key in keys {
                if let Some(sess) = offline_map.remove(&key) {
                    participant_ids.push(key.1);
                    offline_parked.insert(key.1, sess);
                }
            }
        }

        if let Some(cid) = arena_40_challenger {
            participant_ids.push(i64::from(cid));
        }

        if participant_ids.is_empty() {
            if let Ok(online) = self.online.try_read() {
                for (pid, p) in online.iter() {
                    if let Ok(s) = p.session.try_read() {
                        if s.talking_battle > 0 || s.battle_id == id {
                            participant_ids.push(*pid);
                        }
                    }
                }
            }
        }

        participant_ids.sort_unstable();
        participant_ids.dedup();
        let lock_ids: Vec<u32> = participant_ids
            .iter()
            .filter_map(|&pid| u32::try_from(pid).ok().filter(|&u| u > 0))
            .collect();

        // 2. Execute post-battle progression, registry sync, and atomic DB
        //    write-through under Two-Phase State Lock (`lock_player_operations`).
        let run_async = async {
            let _op_guards = crate::server::session::lock_player_operations(lock_ids).await;
            let pool_opt = self.pool.read().await.clone();
            let mut ended_sessions: Vec<Session> = Vec::new();
            let mut eve_followups: Vec<(i64, Session, u16, i32)> = Vec::new();
            let mut eve_warps: Vec<(
                i64,
                u16,
                Option<Arc<tokio::sync::RwLock<Session>>>,
                Option<mpsc::Sender<String>>,
            )> = Vec::new();

            // Snapshot online session handles once for legacy `battle_quest_win`
            // member lookups without holding `self.online` across awaits.
            let online_entries: HashMap<i64, (Arc<tokio::sync::RwLock<Session>>, mpsc::Sender<String>)> = {
                let online = self.online.read().await;
                online
                    .iter()
                    .map(|(pid, p)| (*pid, (Arc::clone(&p.session), p.frames.clone())))
                    .collect()
            };
            let legacy_member_map: HashMap<i64, Arc<tokio::sync::RwLock<Session>>> = online_entries
                .iter()
                .map(|(pid, (sess, _))| (*pid, Arc::clone(sess)))
                .collect();

            for &pid in &participant_ids {
                if let Some((sess_arc, tx)) = online_entries.get(&pid) {
                    let mut s = sess_arc.write().await;
                    s.battle_id = 0;
                    s.last_battle_end_ms = now_ms;
                    s.encounter_steps = 0;

                    if arena_40_challenger == Some(pid as u32) {
                        if let Some(reg) = crate::server::session::registry_get(pid as u32) {
                            if let Some(&(_, reg_step)) = reg.quest_tasks.get(&218) {
                                let _ = crate::server::handlers::quest_sync::upsert_quest_task_slot(
                                    &mut s,
                                    218,
                                    reg_step,
                                );
                            }
                        }
                    }

                    if outcome == Outcome::PlayerWin && arena_40_challenger == Some(pid as u32) {
                        let victory_frames = crate::server::handlers::event_npcs::resolve_40_npc_victory(
                            &mut s,
                            self.data.as_ref(),
                        );
                        for f in victory_frames {
                            let _ = tx.try_send(f);
                        }
                    }

                    let (outgoing, eve_battle, eve_warp) = self
                        .resume_eve_for_participant(pid, &mut s, outcome, pool_opt.as_ref())
                        .await;
                    for f in outgoing {
                        let _ = tx.try_send(f);
                    }
                    if let Some((fight_id, diahinh)) = eve_battle {
                        eve_followups.push((pid, s.clone(), fight_id, diahinh));
                    }
                    if let Some(warp_id) = eve_warp {
                        eve_warps.push((pid, warp_id, Some(Arc::clone(sess_arc)), Some(tx.clone())));
                    }

                    if s.talking_battle > 0 {
                        let mut frames = Vec::new();
                        if outcome == Outcome::PlayerLose {
                            crate::server::handlers::quest::quest_lose_frames(
                                &mut s,
                                self.data.as_ref(),
                                &mut frames,
                            );
                        } else {
                            let mut member = |mem: i64| legacy_member_map.get(&mem).cloned();
                            crate::server::handlers::quest::battle_quest_win(
                                &mut s,
                                self.data.as_ref(),
                                &mut frames,
                                &mut member,
                            );
                        }
                        for f in frames {
                            let _ = tx.try_send(f);
                        }
                    }

                    crate::server::session::registry_update_if_online(&s);
                    ended_sessions.push(s.clone());
                } else if let Some(mut s) = crate::server::session::registry_get(pid as u32) {
                    s.battle_id = 0;
                    s.last_battle_end_ms = now_ms;
                    s.encounter_steps = 0;
                    if arena_40_challenger == Some(pid as u32) {
                        if let Some(reg) = crate::server::session::registry_get(pid as u32) {
                            if let Some(&(_, reg_step)) = reg.quest_tasks.get(&218) {
                                let _ = crate::server::handlers::quest_sync::upsert_quest_task_slot(
                                    &mut s,
                                    218,
                                    reg_step,
                                );
                            }
                        }
                    }
                    if outcome == Outcome::PlayerWin && arena_40_challenger == Some(pid as u32) {
                        let _ = crate::server::handlers::event_npcs::resolve_40_npc_victory(
                            &mut s,
                            self.data.as_ref(),
                        );
                    }
                    let (_outgoing, eve_battle, eve_warp) = self
                        .resume_eve_for_participant(pid, &mut s, outcome, pool_opt.as_ref())
                        .await;
                    if let Some((fight_id, diahinh)) = eve_battle {
                        eve_followups.push((pid, s.clone(), fight_id, diahinh));
                    }
                    if let Some(warp_id) = eve_warp {
                        if let Some(warp) = self.data.warps.get(&(i64::from(s.map_id), i64::from(warp_id))) {
                            if warp.map2 > 0 {
                                s.map_id = warp.map2 as u16;
                                s.map_x = warp.x as u16;
                                s.map_y = warp.y as u16;
                            }
                        }
                    }
                    crate::server::session::registry_update_if_online(&s);
                    ended_sessions.push(s);
                } else if let Some(mut offline_s) = offline_parked.remove(&pid) {
                    offline_s.battle_id = 0;
                    offline_s.last_battle_end_ms = now_ms;
                    offline_s.encounter_steps = 0;
                    let (_outgoing, _eve_battle, eve_warp) = self
                        .resume_eve_for_participant(pid, &mut offline_s, outcome, pool_opt.as_ref())
                        .await;
                    if let Some(warp_id) = eve_warp {
                        if let Some(warp) = self.data.warps.get(&(i64::from(offline_s.map_id), i64::from(warp_id))) {
                            if warp.map2 > 0 {
                                offline_s.map_id = warp.map2 as u16;
                                offline_s.map_x = warp.x as u16;
                                offline_s.map_y = warp.y as u16;
                            }
                        }
                    }
                    ended_sessions.push(offline_s);
                }
            }

            if !ended_sessions.is_empty() {
                if let Some(ref pool) = pool_opt {
                    let refs: Vec<&Session> = ended_sessions.iter().collect();
                    if !crate::db::persist::persist_sessions_transaction(
                        Some(pool),
                        &refs,
                        &["stats", "homdo", "trangbi", "quest", "pet"],
                    )
                    .await
                    {
                        tracing::warn!(battle_id = id, "Battle End DB write-through failed");
                    }
                }
            }

            for (pid, warp_id, sess_arc_opt, tx_opt) in eve_warps {
                if let Some(sess_arc) = sess_arc_opt {
                    let mut warp_out = crate::server::dispatcher::HandleOutcome::default();
                    let (warped, leader_coords, member_ids) = {
                        let mut s = sess_arc.write().await;
                        let ok = crate::server::handlers::quest::perform_party_warp_for_locked_session(
                            &mut s,
                            self.data.as_ref(),
                            pool_opt.as_ref(),
                            warp_id,
                            &mut warp_out,
                        )
                        .await;
                        let coords = (s.map_id, s.map_x, s.map_y);
                        let members = if s.id_leader == s.id && s.id > 0 {
                            s.id_mem
                        } else {
                            [0; 4]
                        };
                        (ok, coords, members)
                    };
                    if warped {
                        for mem_id in member_ids {
                            if mem_id > 0 && i64::from(mem_id) != pid {
                                if let Some((mem_arc, _)) = online_entries.get(&i64::from(mem_id)) {
                                    let mut ms = mem_arc.write().await;
                                    ms.map_id = leader_coords.0;
                                    ms.map_x = leader_coords.1;
                                    ms.map_y = leader_coords.2;
                                }
                            }
                        }
                        if let Some(tx) = tx_opt {
                            for f in warp_out.outgoing {
                                let _ = tx.try_send(f.frame);
                            }
                        }
                        for (target_id, frame) in warp_out.direct_messages {
                            self.send_to(i64::from(target_id), frame);
                        }
                        for mb in warp_out.map_broadcast {
                            if let Some(target_map) = mb.map_id {
                                send_to_map(&self.online, i64::from(target_map), mb.frame);
                            } else {
                                let exclude = if mb.subject > 0 { i64::from(mb.subject) } else { pid };
                                self.send_map(exclude, mb.frame);
                            }
                        }
                    }
                }
            }

            drop(_op_guards);

            eve_followups
        };

        let eve_followups = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::MultiThread => {
                    tokio::task::block_in_place(|| handle.block_on(run_async))
                }
                _ => futures::executor::block_on(run_async),
            }
        } else {
            futures::executor::block_on(run_async)
        };

        // 3. Chained Eve battles: all participant locks and `members` locks are
        //    now free, so `start_eve_encounter` can safely spawn the next battle.
        for (player, mut session, fight_id, diahinh) in eve_followups {
            let Some(service) = self.eve_service.get().and_then(std::sync::Weak::upgrade) else {
                tracing::warn!(
                    player,
                    fight_id,
                    "chained Eve battle skipped: no BattleService backref installed"
                );
                continue;
            };
            service.start_eve_encounter(&mut session, fight_id, diahinh);
        }
    }
}

impl BattleSinkImpl {
    async fn resume_eve_for_participant(
        &self,
        pid: i64,
        session: &mut Session,
        outcome: Outcome,
        pool: Option<&crate::db::pool::DbPool>,
    ) -> (Vec<String>, Option<(u16, i32)>, Option<u16>) {
        let is_eve_leader = session.id_leader == 0 || session.id_leader == session.id;
        if is_eve_leader {
            let eve_out = crate::server::handlers::talk::resume_eve_after_battle_deferred_warp(
                session,
                outcome,
                self.data.as_ref(),
                pool,
            )
            .await;
            for (target_id, frame) in eve_out.direct_messages {
                self.send_to(i64::from(target_id), frame);
            }
            for mb in eve_out.map_broadcast {
                if let Some(target_map) = mb.map_id {
                    send_to_map(&self.online, i64::from(target_map), mb.frame);
                } else {
                    let exclude = if mb.subject > 0 { i64::from(mb.subject) } else { pid };
                    self.send_map(exclude, mb.frame);
                }
            }
            let outgoing = eve_out.outgoing.into_iter().map(|f| f.frame).collect();
            (outgoing, eve_out.eve_battle, eve_out.eve_warp)
        } else {
            if session
                .current_event_session
                .as_ref()
                .is_some_and(|ev| ev.phase == crate::eve::EventPhase::AwaitingBattle)
            {
                session.current_event_session = None;
            }
            (Vec::new(), None, None)
        }
    }
}

/// Send `frame` to the given player when they are online.
fn push(online: &tokio::sync::RwLock<OnlineMap>, player: i64, frame: String) {
    if let Some(online) = try_read_online_with_retry(online, 10, std::time::Duration::from_millis(2)) {
        if let Some(p) = online.get(&player) {
            let _ = p.frames.try_send(frame);
        }
    }
}

/// Send `frame` to every online player whose session sits on `map_id`
/// (includes the sender's own map). Shared by the
/// walk loop's wander/chase fan-out and the battle respawn broadcast.
fn send_to_map(online: &tokio::sync::RwLock<OnlineMap>, map_id: i64, frame: String) {
    if let Some(online) = try_read_online_with_retry(online, 10, std::time::Duration::from_millis(2)) {
        for (id, p) in online.iter() {
            if let Ok(s) = p.session.try_read() {
                if i64::from(s.map_id) == map_id {
                    let _ = p.frames.try_send(frame.clone());
                }
            }
            let _ = id;
        }
    }
}

/// Apply a battle DB write to the shared session and, for player targets,
/// push the matching `F4440C000801` status frame. `Hpmax`/`Spmax` lock the
/// client's `+0x404`/`+0x406` with attr `0xCD`/`0xCE`
/// (`04_battle_end_rewards.md §2.4`) so the client stops recomputing its own
/// maxima from the `0x23` level byte.
fn apply_db_update(online: &OnlineMap, update: DbUpdate) {
    match update.target {
        DbTarget::Player(id) => {
            if let Some(p) = online.get(&id) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    let frame = apply_player_stat(&mut s, update.stat, update.value);
                    if let Some(f) = frame {
                        let _ = p.frames.try_send(f);
                    }
                }
            }
            let mut map = crate::server::session::lock_online_sessions();
            if let Some(s) = map.get_mut(&(id as u32)) {
                apply_player_stat(s, update.stat, update.value);
            }
        }
        DbTarget::Pet { owner, stt } => {
            if let Some(p) = online.get(&owner) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    if let Some(pet) = s.pets.iter_mut().find(|p| i64::from(p.stt) == stt) {
                        match update.stat {
                            Stat::Hp => pet.hp = clamp_u16(update.value),
                            Stat::Sp => pet.sp = clamp_u16(update.value),
                            _ => {}
                        }
                    }
                }
            }
            let mut map = crate::server::session::lock_online_sessions();
            if let Some(s) = map.get_mut(&(owner as u32)) {
                if let Some(pet) = s.pets.iter_mut().find(|p| i64::from(p.stt) == stt) {
                    match update.stat {
                        Stat::Hp => pet.hp = clamp_u16(update.value),
                        Stat::Sp => pet.sp = clamp_u16(update.value),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn apply_player_stat(s: &mut Session, stat: Stat, value: i64) -> Option<String> {
    let (byte, apply): (Option<u8>, fn(&mut Session, i64)) = match stat {
        Stat::Hp => (Some(0x19), |s, v| s.hp = clamp_u16(v)),
        Stat::Sp => (Some(0x1A), |s, v| s.sp = clamp_u16(v)),
        Stat::Texp => (Some(0x24), |s, v| {
            s.texp = v.clamp(0, u32::MAX as i64) as u32
        }),
        Stat::Lv => (Some(0x23), |s, v| s.level = v.clamp(0, 0xFF) as u8),
        // MaxHP/MaxSP: attr `0xCD`/`0xCE` write `+0x404`/`+0x406`, locking the
        // server-computed values after the client's own recompute on `0x23`
        // (`04_battle_end_rewards.md §2.4`).
        Stat::Hpmax => (Some(0xCD), |s, v| s.hp_max = clamp_u16(v)),
        Stat::Spmax => (Some(0xCE), |s, v| s.sp_max = clamp_u16(v)),
        Stat::Point => (Some(0x26), |s, v| s.point = clamp_u16(v)),
        Stat::SkillPoint => (Some(0x25), |s, v| s.skill_point = clamp_u16(v)),
        Stat::Fai => (Some(0x40), |s, v| s.tiengtam = clamp_u16(v)),
        Stat::God => (None, |s, v| s.god = v.clamp(0, u32::MAX as i64) as u32),
    };
    apply(s, value);
    byte.map(|b| packets::status_update(b, value.clamp(i32::MIN as i64, i32::MAX as i64) as i32))
}

fn clamp_u16(v: i64) -> u16 {
    v.clamp(0, 0xFFFF) as u16
}

fn add_pet_to_session(s: &mut Session, npc_id: u16) {
    let hp_max = get_hp_max(0, 0, 1, 0) as u16;
    crate::server::pet_box::add_caught(&mut s.pets, npc_id, hp_max);
}

/// The server-side battle service.
pub struct BattleService {
    pub manager: Arc<BattleManager>,
    pub data: Arc<GameData>,
    sink: Arc<BattleSinkImpl>,
    online: Arc<tokio::sync::RwLock<OnlineMap>>,
    /// Optional DB pool used for post-battle persistence (quest/homdo);
    /// shared with the sink via `Arc` so `with_pool` is visible to battles.
    pool: Option<Arc<tokio::sync::RwLock<Option<crate::db::pool::DbPool>>>>,
    /// The shared map-NPC world (ticket 20 G1/G2), built from `npcs_on_map`.
    world: Option<Arc<std::sync::RwLock<NpcWorld>>>,
    /// Synchronous handle registry for sync handler access (op 0x0B, join).
    handles: Mutex<HashMap<i32, BattleHandle>>,
    /// Pre-rendered join cell records per battle id (the grid lives in the
    /// async task, so we capture it at spawn).
    join_cells: Mutex<HashMap<i32, (i32, String)>>,
    per_exp: i64,
    next_battle: AtomicI32,
    /// Per-turn input wait (default 21 s).
    input_timeout: std::time::Duration,
    /// Special events & North Star quiz manager
    pub events: Arc<crate::server::events::EventLifecycleManager>,
    /// Active 40-NPC Arena continuous battles (battle_id -> challenger_id)
    pub active_40_npc: Arc<Mutex<HashMap<i32, u32>>>,
}

impl Default for BattleService {
    fn default() -> Self {
        BattleService::new(Arc::new(GameData::default()))
    }
}

impl BattleService {
    pub fn new(data: Arc<GameData>) -> Self {
        let online = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let pool = Arc::new(tokio::sync::RwLock::new(None));
        // The shared world is only built when static NpcOnMap data is present
        // (golden replay uses an empty GameData → world is None).
        let world = if data.npc_on_map.is_empty() {
            None
        } else {
            Some(Arc::new(std::sync::RwLock::new(NpcWorld::new(
                &data.npc_on_map,
            ))))
        };
        let active_40_npc = Arc::new(Mutex::new(HashMap::new()));
        let sink = Arc::new(BattleSinkImpl {
            online: Arc::clone(&online),
            members: Arc::new(Mutex::new(HashSet::new())),
            data: Arc::clone(&data),
            pool: Arc::clone(&pool),
            eve_service: std::sync::OnceLock::new(),
            active_40_npc: Arc::clone(&active_40_npc),
            battle_participants: Arc::new(Mutex::new(HashMap::new())),
            offline_eve_sessions: Arc::new(Mutex::new(HashMap::new())),
        });
        BattleService {
            manager: Arc::new(BattleManager::new()),
            data,
            sink,
            online,
            pool: Some(pool),
            world,
            handles: Mutex::new(HashMap::new()),
            join_cells: Mutex::new(HashMap::new()),
            per_exp: 1,
            next_battle: AtomicI32::new(1),
            input_timeout: std::time::Duration::from_secs(21),
            events: Arc::new(crate::server::events::EventLifecycleManager::default()),
            active_40_npc,
        }
    }

    /// Attach a custom Special Events Lifecycle Manager.
    pub fn with_events(mut self, events: Arc<crate::server::events::EventLifecycleManager>) -> Self {
        self.events = events;
        self
    }

    /// Attach the DB pool used for post-battle persistence.
    pub fn with_pool(self, pool: crate::db::pool::DbPool) -> Self {
        if let Some(p) = &self.pool {
            if let Ok(mut guard) = p.try_write() {
                *guard = Some(pool);
            }
        }
        self
    }

    /// The configured post-battle persistence pool (best-effort setter used by
    /// tests); `None` when no DB is attached.
    pub fn pool(&self) -> Option<crate::db::pool::DbPool> {
        self.pool
            .as_ref()
            .and_then(|p| p.try_read().ok())
            .and_then(|p| p.clone())
    }

    /// Override the per-turn input wait (tests use a short window).
    pub fn set_input_timeout(&mut self, timeout: std::time::Duration) {
        self.input_timeout = timeout;
    }

    /// Register an online player; the returned receiver collects the frames the
    /// battle task would send to them (the socket writer drains it).
    pub fn register(
        &self,
        player: i64,
        session: Arc<tokio::sync::RwLock<Session>>,
    ) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(1024);
        if let Ok(mut online) = self.online.try_write() {
            online.insert(
                player,
                OnlinePlayer {
                    session,
                    frames: tx,
                },
            );
        }
        rx
    }

    /// Register an online player with a pre-existing frame sender channel.
    /// If the player is already registered, refreshes both the session handle
    /// and the frame sender so `BattleService` always observes the latest state.
    pub fn register_sender(
        &self,
        player: i64,
        session: Arc<tokio::sync::RwLock<Session>>,
        frames: mpsc::Sender<String>,
    ) {
        for attempt in 0..10 {
            if let Ok(mut online) = self.online.try_write() {
                if let Some(existing) = online.get_mut(&player) {
                    existing.session = session;
                    existing.frames = frames;
                } else {
                    online.insert(
                        player,
                        OnlinePlayer {
                            session,
                            frames,
                        },
                    );
                }
                break;
            }
            if attempt + 1 < 10 {
                std::thread::yield_now();
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }

    pub fn unregister(&self, player: i64) {
        let mut snapshotted = false;
        if let Ok(mut online) = self.online.try_write() {
            if let Some(existing) = online.remove(&player) {
                if let Ok(s) = existing.session.try_read() {
                    self.sink.snapshot_offline_eve_session(&s);
                    snapshotted = true;
                }
            }
        }
        if !snapshotted {
            if let Some(s) = crate::server::session::registry_get(player as u32) {
                self.sink.snapshot_offline_eve_session(&s);
            }
        }
        if let Ok(mut members) = self.sink.members.lock() {
            members.remove(&player);
        }
    }

    /// Whether any players are still registered as battle participants.
    pub fn has_members(&self) -> bool {
        self.sink
            .members
            .lock()
            .map(|m| !m.is_empty())
            .unwrap_or(false)
    }

    /// The next battle id (dedicated counter; `BattleManager` is shared).
    pub fn next_battle_id(&self) -> i32 {
        self.next_battle.fetch_add(1, Ordering::SeqCst)
    }

    /// Start an NPC battle.
    ///
    /// Returns the new battle id, or 0 if the NPC template is missing.
    pub fn start_npc_battle(&self, session: &mut Session, npc_id: i64, npc_on_map_id: i64, ground: i64) -> i32 {
        let Some(npc) = self.data.npcs.get(&npc_id).cloned() else {
            return 0;
        };
        let id = self.next_battle_id();
        let terrain = if ground > 0 { ground as i32 } else { 112 };
        let battle =
            Battle::npc_battle(id, session, i64::from(session.id), &npc, npc_on_map_id, terrain);
        let (extra_players, extra_pets) = self.party_snapshots(session);
        self.spawn_battle(
            battle,
            session,
            extra_players,
            extra_pets,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Seeded NPC battle start — deterministic RNG for golden replay.
    pub fn start_npc_battle_seeded(
        &self,
        session: &mut Session,
        npc_id: i64,
        npc_on_map_id: i64,
        s0: i32,
        s1: i32,
        s2: i32,
    ) -> i32 {
        let Some(npc) = self.data.npcs.get(&npc_id).cloned() else {
            return 0;
        };
        let id = self.next_battle_id();
        let mut battle = Battle::with_seeds(id, 112, s0, s1, s2);
        battle.add_player(session, i64::from(session.id), 3, 2);
        battle.load_leader_pets(session, i64::from(session.id), 3);
        battle.add_npc(&npc, npc_on_map_id, 0, 2, 3);
        let (extra_players, extra_pets) = self.party_snapshots(session);
        self.spawn_battle(
            battle,
            session,
            extra_players,
            extra_pets,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Start a PK battle.
    pub fn start_pk_battle(&self, session: &mut Session, opponent: i64, ground: i64) -> i32 {
        let opp = {
            let online = match self.online.try_read() {
                Ok(o) => o,
                Err(_) => return 0,
            };
            match online.get(&opponent) {
                Some(p) => p.session.clone(),
                None => return 0,
            }
        };
        let id = self.next_battle_id();
        let terrain = if ground > 0 { ground as i32 } else { 112 };
        let mut battle = Battle::new(id, terrain);
        battle.add_player(session, i64::from(session.id), 3, 2);
        battle.load_leader_pets(session, i64::from(session.id), 3);
        let opp_guard = match opp.try_read() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        battle.add_player(&opp_guard, opponent, 0, 2);
        let opponent_start = battle.member_battle_frame(terrain, opponent);
        let mut extra_players = HashMap::new();
        let mut extra_pets = HashMap::new();
        extra_players.insert(opponent, self.snapshot(&opp_guard));
        extra_pets.insert(opponent, self.pet_slots(&opp_guard));
        drop(opp_guard);
        self.spawn_battle(
            battle,
            session,
            extra_players,
            extra_pets,
            vec![opponent],
            opponent_start,
        )
    }

    /// Start a TeamDef (quest) battle from a `BattleTrigger`.
    pub fn start_teamdef_battle(&self, session: &mut Session, trigger: &BattleTrigger) -> i32 {
        let defenders: Vec<_> = trigger
            .teamdef
            .iter()
            .skip(1)
            .filter_map(|id| self.data.npcs.get(id))
            .collect();
        if defenders.is_empty() {
            return 0;
        }
        let id = self.next_battle_id();
        let battle = Battle::teamdef_battle(
            id,
            session,
            i64::from(session.id),
            &[],
            &defenders,
            trigger.diahinh,
        );
        let (extra_players, extra_pets) = self.party_snapshots(session);
        self.spawn_battle(
            battle,
            session,
            extra_players,
            extra_pets,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Seeded TeamDef battle start — deterministic RNG for the golden replay.
    /// `trigger.teamdef` = `[diahinh, npc1..npc10]`; defenders are placed at
    /// the TeamDef grid positions (0,0)..(1,4) exactly like `teamdef_battle`.
    pub fn start_teamdef_battle_seeded(
        &self,
        session: &mut Session,
        trigger: &BattleTrigger,
        s0: i32,
        s1: i32,
        s2: i32,
    ) -> i32 {
        let defenders: Vec<_> = trigger
            .teamdef
            .iter()
            .skip(1)
            .filter_map(|id| self.data.npcs.get(id))
            .cloned()
            .collect();
        if defenders.is_empty() {
            return 0;
        }
        let id = self.next_battle_id();
        let mut battle = Battle::with_seeds(id, trigger.diahinh, s0, s1, s2);
        battle.add_player(session, i64::from(session.id), 3, 2);
        battle.load_leader_pets(session, i64::from(session.id), 3);
        let positions: [(u8, u8); 10] = [
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (1, 4),
        ];
        for (i, npc) in defenders.iter().enumerate().take(10) {
            let (r, c) = positions[i];
            battle.add_npc(npc, (i + 1) as i64, r, c, 7);
        }
        let (extra_players, extra_pets) = self.party_snapshots(session);
        self.spawn_battle(
            battle,
            session,
            extra_players,
            extra_pets,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Start a wild random encounter battle from an Eve FightData record.
    ///
    /// Populates the leader, leader's pets, online party members (cols 1, 3, 0, 4)
    /// and their pets, and up to 10 enemies defined in `fight_data.left_enemies`.
    /// Install the weak self-reference the sink needs to start a *chained*
    /// Eve battle from `battle_ended` (CP6 #4). Call once right after the
    /// service is placed in its `Arc`:
    ///
    /// ```ignore
    /// let service = Arc::new(BattleService::new(data));
    /// BattleService::install_eve_backref(&service);
    /// ```
    ///
    /// Without it a chained request is logged and skipped (never parked), so
    /// the bare service instances built by tests keep working unchanged.
    pub fn install_eve_backref(this: &Arc<Self>) {
        let _ = this.sink.eve_service.set(Arc::downgrade(this));
    }

    /// Start the scripted Eve battle behind a `result_type == 3` result
    /// (CP6 #2): resolve `fight_id` in the current map's `scene.fight_datas`
    /// and spawn the same PvE encounter the random-encounter path uses, on the
    /// caller-resolved background (`diahinh`).
    ///
    /// Returns the spawned battle id, or `0` when the map has no such fight
    /// row — the caller must then close instead of parking a session on a
    /// battle that can never start.
    pub fn start_eve_encounter(&self, session: &mut Session, fight_id: u16, diahinh: i32) -> i32 {
        let Some(fight) = self
            .data
            .scene_eve_data
            .get(&u32::from(session.map_id))
            .and_then(|scene| scene.fight_datas.get(&fight_id))
        else {
            tracing::debug!(
                map_id = session.map_id,
                fight_id,
                "Eve battle start skipped: map has no FightData row"
            );
            return 0;
        };
        self.start_encounter_battle(session, fight, diahinh)
    }

    pub fn start_encounter_battle(
        &self,
        leader_session: &mut Session,
        fight_data: &crate::data::loaders::EveFightData,
        diahinh: i32,
    ) -> i32 {
        let id = self.next_battle_id();
        let terrain = if diahinh > 0 { diahinh } else { 112 };
        let mut battle = Battle::new(id, terrain);
        battle.fight_type = fight_data.fight_type;
        let lid = i64::from(leader_session.id);

        // 1. Leader & Leader Pet
        battle.add_player(leader_session, lid, 3, 2);
        battle.load_leader_pets(leader_session, lid, 3);

        // 2. Party Members & Member Pets (Columns: 1, 3, 0, 4)
        let member_cols = [1u8, 3, 0, 4];
        let mut extra_members = Vec::new();
        let mut extra_start = Vec::new();
        let mut extra_players = HashMap::new();
        let mut extra_pets = HashMap::new();

        // Sync leader session into self.online if present
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&lid) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    *s = leader_session.clone();
                    s.battle_id = id;
                }
            }
        }

        // Extract member session Arc pointers with retry, minimizing online lock duration
        let member_session_arcs: Vec<(usize, i64, Arc<tokio::sync::RwLock<Session>>)> = {
            if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
                leader_session
                    .id_mem
                    .iter()
                    .filter(|&&m| m > 0)
                    .enumerate()
                    .take(4)
                    .filter_map(|(i, &mem_id)| {
                        let mid = i64::from(mem_id);
                        online.get(&mid).map(|p| (i, mid, Arc::clone(&p.session)))
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };

        for (i, mid, session_arc) in member_session_arcs {
            if let Some(mut mem_sess) = try_write_session_with_retry(&session_arc, 10, std::time::Duration::from_millis(2)) {
                if mem_sess.battle_id != 0 {
                    continue;
                }
                // Sync with latest online_sessions() state if available
                {
                    let map = crate::server::session::lock_online_sessions();
                    if let Some(global_s) = map.get(&(mid as u32)) {
                        *mem_sess = global_s.clone();
                    }
                }
                let col = member_cols[i];
                battle.add_player(&mem_sess, lid, 3, col);
                battle.load_member_pet(&mem_sess, lid, 3, col);

                mem_sess.battle_id = id;
                extra_members.push(mid);
                extra_players.insert(mid, self.snapshot(&mem_sess));
                extra_pets.insert(mid, self.pet_slots(&mem_sess));

                // Generate open frames for member (0x0B Sub FA)
                let frames = battle.member_battle_frame(terrain, mid);
                extra_start.extend(frames);
            }
        }

        // Synchronize global online_sessions() for leader & members, including fallback for any members not in self.online
        {
            let mut map = crate::server::session::lock_online_sessions();
            if let Some(s) = map.get_mut(&leader_session.id) {
                s.battle_id = id;
            }
            for (i, &mem_id) in leader_session
                .id_mem
                .iter()
                .filter(|&&m| m > 0)
                .enumerate()
                .take(4)
            {
                let mid = i64::from(mem_id);
                if let Some(s) = map.get_mut(&mem_id) {
                    if !extra_members.contains(&mid) {
                        if s.battle_id != 0 {
                            continue;
                        }
                        s.battle_id = id;
                        let col = member_cols[i];
                        battle.add_player(s, lid, 3, col);
                        battle.load_member_pet(s, lid, 3, col);

                        extra_members.push(mid);
                        extra_players.insert(mid, self.snapshot(s));
                        extra_pets.insert(mid, self.pet_slots(s));

                        let frames = battle.member_battle_frame(terrain, mid);
                        extra_start.extend(frames);
                    } else {
                        s.battle_id = id;
                    }
                }
            }
        }

        // 3. Enemy Formation (`left_enemies` on rows 0..=1, team 2)
        for (idx, enemy) in fight_data.left_enemies.iter().enumerate().take(10) {
            if let Some(npc) = self.data.npcs.get(&i64::from(enemy.npc_id)) {
                let row = enemy.row();
                let col = enemy.col();
                battle.add_npc(npc, (idx + 1) as i64, row, col, 3);
            }
        }

        // 4. Ally Formation (`right_enemies` on rows 2..=3, team 1)
        for (idx, ally) in fight_data.right_enemies.iter().enumerate().take(10) {
            if let Some(npc) = self.data.npcs.get(&i64::from(ally.npc_id)) {
                let row = if ally.location_pos < 10 {
                    2 + ally.row()
                } else {
                    ally.row().clamp(2, 3)
                };
                let col = ally.col();
                if battle.cell(row, col).map(|c| c.id == 0).unwrap_or(false) {
                    battle.add_npc(npc, (idx + 1) as i64, row, col, 3);
                }
            }
        }

        // 4. Activate Battle Task & Notify
        self.spawn_battle(
            battle,
            leader_session,
            extra_players,
            extra_pets,
            extra_members,
            extra_start,
        )
    }

    /// Check if a battle is an active North Star quiz battle.
    pub fn is_north_star_battle(&self, battle_id: i32) -> bool {
        self.events.is_north_star_battle(battle_id)
    }

    /// Abort an ongoing battle and remove registered handles / quizzes.
    pub fn abort_battle(&self, battle_id: i32) {
        if let Ok(mut handles) = self.handles.lock() {
            handles.remove(&battle_id);
        }
        if let Ok(mut cells) = self.join_cells.lock() {
            cells.remove(&battle_id);
        }
        self.events.remove_quiz(battle_id);
    }

    /// Start a special PvE North Star Quiz battle against NPC 1643 or 1644.
    /// Emits S->C 10 01 modal question frame strictly to the Party Leader.
    pub fn start_north_star_battle(
        &self,
        leader_session: &mut Session,
        npc_id: i64,
        diahinh: i32,
    ) -> (i32, String) {
        let npc = self.data.npcs.get(&npc_id).cloned().unwrap_or_else(|| {
            let name_bytes = if npc_id == 1644 {
                crate::encoding::viscii_encode("Nam Tinh Quân")
            } else {
                crate::encoding::viscii_encode("Bắc Tinh Quân")
            };
            crate::data::tables::Npc {
                id: npc_id,
                name: name_bytes,
                lv: 100,
                thuoctinh: 1,
                hp: 10000,
                sp: 10000,
                atk: 100,
                def: 100,
                agi: 100,
                ..Default::default()
            }
        });

        let id = self.next_battle_id();
        let terrain = if diahinh > 0 { diahinh } else { 112 };
        let mut battle = Battle::new(id, terrain);
        let lid = i64::from(leader_session.id);

        battle.add_player(leader_session, lid, 3, 2);
        battle.load_leader_pets(leader_session, lid, 3);

        // Spawn North Star NPC at opposing center cell
        battle.add_npc(&npc, 0, 1, 2, 0);

        leader_session.battle_id = id;

        // Set battle_id for party members in online sessions
        {
            let mut online = crate::server::session::lock_online_sessions();
            for &mem in &leader_session.id_mem {
                if mem > 0 && mem != leader_session.id {
                    if let Some(m_sess) = online.get_mut(&mem) {
                        m_sess.battle_id = id;
                    }
                }
            }
        }

        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(lid);
            for &mem in &leader_session.id_mem {
                if mem > 0 && mem != leader_session.id {
                    members.insert(i64::from(mem));
                }
            }
        }

        self.record_join_cells(id, &battle);

        let handle = crate::battle::manager::BattleHandle::dummy(id);
        if let Ok(mut h) = self.handles.lock() {
            h.insert(id, handle);
        }

        let start = battle.npc_battle_start_packets(terrain);
        for packet in start {
            match packet {
                crate::battle::construction::StartPacket::To { player, frame } => self.sink.send_to(player, frame),
                crate::battle::construction::StartPacket::Map { player, frame } => self.sink.send_map(player, frame),
            }
        }

        // Register quiz with the EventLifecycleManager
        let (_question, modal_frame) = self.events.register_quiz(id, leader_session.id, None);

        (id, modal_frame)
    }

    /// Start continuous 40-NPC Arena battle with 40 preloaded escalating waves.
    pub fn start_40_npc_battle(
        &self,
        leader_session: &mut Session,
        waves: Vec<Vec<(crate::data::tables::Npc, u8, u8)>>,
    ) -> (i32, Vec<String>) {
        let id = self.next_battle_id();
        let terrain = 112;
        let mut battle = Battle::new(id, terrain);
        battle.challenger_id = leader_session.id;
        battle.is_40_npc = true;
        let lid = i64::from(leader_session.id);

        battle.add_player(leader_session, lid, 3, 2);
        battle.load_leader_pets(leader_session, lid, 3);

        let member_cols = [1u8, 3, 0, 4];
        let mut extra_members = Vec::new();
        let mut extra_start = Vec::new();
        let mut extra_players = HashMap::new();
        let mut extra_pets = HashMap::new();

        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&lid) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    *s = leader_session.clone();
                    s.battle_id = id;
                }
            }
        }

        {
            let mut map = crate::server::session::lock_online_sessions();
            if let Some(s) = map.get_mut(&leader_session.id) {
                s.battle_id = id;
            }
            for (i, &mem_id) in leader_session.id_mem.iter().filter(|&&m| m > 0).enumerate().take(4) {
                let mid = i64::from(mem_id);
                if let Some(s) = map.get_mut(&mem_id) {
                    if s.battle_id != 0 {
                        continue;
                    }
                    s.battle_id = id;
                    let col = member_cols[i];
                    battle.add_player(s, lid, 3, col);
                    battle.load_member_pet(s, lid, 3, col);

                    extra_members.push(mid);
                    extra_players.insert(mid, self.snapshot(s));
                    extra_pets.insert(mid, self.pet_slots(s));

                    let frames = battle.member_battle_frame(terrain, mid);
                    extra_start.extend(frames);
                }
            }
        }

        if let Some(wave1) = waves.first() {
            for (idx, (npc, row, col)) in wave1.iter().enumerate() {
                let id_on_map = (idx + 1) as i64;
                battle.add_npc(npc, id_on_map, *row, *col, 3);
            }
        }
        battle.set_waves(waves);

        self.sink.active_40_npc.lock().unwrap().insert(id, leader_session.id);

        let start = battle.npc_battle_start_packets(terrain);
        let mut leader_packets = Vec::new();
        for packet in &start {
            if let crate::battle::construction::StartPacket::To { player, frame } = packet {
                if *player == lid {
                    leader_packets.push(frame.clone());
                }
            }
        }

        self.spawn_battle(
            battle,
            leader_session,
            extra_players,
            extra_pets,
            extra_members,
            extra_start,
        );

        (id, leader_packets)
    }

    /// Register a player in an existing battle (op 0x0B sub 4 join).
    pub fn join_battle(&self, session: &mut Session, battle_id: i32) -> bool {
        if session.battle_id != 0 || !self.battle_exists(battle_id) || self.is_north_star_battle(battle_id) {
            return false;
        }
        session.battle_id = battle_id;
        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(i64::from(session.id));
        }
        if let Ok(mut participants) = self.sink.battle_participants.lock() {
            participants
                .entry(battle_id)
                .or_default()
                .push(i64::from(session.id));
        }
        let join = self.build_join_frame(session, battle_id);
        self.sink.send_to(i64::from(session.id), join);
        self.sink
            .send_to(i64::from(session.id), packets::battle_trailer());
        true
    }

    fn battle_exists(&self, battle_id: i32) -> bool {
        self.handles
            .lock()
            .map(|h| h.contains_key(&battle_id))
            .unwrap_or(false)
    }

    /// Build the op 0x0B sub-4 join frame: `0BFA` + LE16(diahinh) + `0402` +
    /// self record + the 20 grid cell records (captured at spawn).
    pub fn build_join_frame(&self, session: &Session, battle_id: i32) -> String {
        let cells = self
            .join_cells
            .lock()
            .ok()
            .and_then(|j| j.get(&battle_id).cloned());
        let (diahinh, cell_records) = cells.unwrap_or_else(|| {
            let mut empty = String::new();
            for _ in 0..20 {
                empty.push_str(&"0".repeat(48)); // 24-byte empty cell record
            }
            (112, empty)
        });
        let mut text = format!(
            "0402{}{}{}{}{}{}{:02X}{:02X}",
            encoder::le32(session.id),
            "000000000000FFFF",
            encoder::le16(session.hp_max),
            encoder::le16(session.sp_max),
            encoder::le16(session.hp),
            encoder::le16(session.sp),
            session.level,
            session.thuoctinh
        );
        text.push_str(&cell_records);
        crate::protocol::frame(&format!("0BFA{}", encoder::le16(diahinh as u16)), &text)
    }

    /// Leave the current battle (op 0x0B sub 1 or sub 5).
    pub fn leave_battle(&self, session: &mut Session) {
        self.sink.snapshot_offline_eve_session(session);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        session.battle_id = 0;
        session.last_battle_end_ms = now_ms;
        session.encounter_steps = 0;
        if let Ok(mut members) = self.sink.members.lock() {
            members.remove(&i64::from(session.id));
        }
        if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
            if let Some(p) = online.get(&i64::from(session.id)) {
                if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                    s.battle_id = 0;
                    s.last_battle_end_ms = now_ms;
                    s.encounter_steps = 0;
                }
            }
        }
        crate::server::session::registry_update_if_online(session);
    }

    /// Submit a battle command (op 0x32) to the player's battle.
    pub fn submit_command(&self, session: &Session, cmd: BattleCommand) -> bool {
        if session.battle_id == 0 {
            return false;
        }
        let handle = match self.handles.lock() {
            Ok(h) => h.get(&session.battle_id).cloned(),
            Err(_) => None,
        };
        match handle {
            Some(h) => h.command(crate::battle::manager::PlayerInput {
                player: i64::from(session.id),
                cmd,
            }),
            None => false,
        }
    }

    /// Acknowledge the battle scene from the client (0B FA ack - Ticket 20).
    pub fn ack_scene(&self, battle_id: i32) {
        if battle_id == 0 {
            return;
        }
        let handle = match self.handles.lock() {
            Ok(h) => h.get(&battle_id).cloned(),
            Err(_) => None,
        };
        if let Some(h) = handle {
            h.ack_scene();
        }
    }

    /// Broadcast a frame to every current battle participant (party).
    pub fn broadcast(&self, frame: String) {
        self.sink.broadcast(frame);
    }

    /// Push one frame to a specific online player (handlers' direct sends).
    pub fn send_to(&self, player: i64, frame: String) {
        self.sink.send_to(player, frame);
    }

    /// Push a frame to every online player on the map except `player`.
    pub fn send_map(&self, player: i64, frame: String) {
        self.sink.send_map(player, frame);
    }

    /// Trigger post-battle resolution (`Two-Phase State Lock`, Eve resume,
    /// `registry_update_if_online`, and atomic DB write-through) for `id`.
    pub fn battle_ended(&self, id: i32, outcome: Outcome) {
        self.sink.battle_ended(id, outcome);
    }

    /// Is `player` currently online (used by PK gates)?
    pub fn is_online(&self, player: i64) -> bool {
        self.online
            .try_read()
            .map(|o| o.contains_key(&player))
            .unwrap_or(false)
    }

    /// The target's PK flag (None if offline).
    pub fn target_pk(&self, player: i64) -> Option<bool> {
        let online = self.online.try_read().ok()?;
        let p = online.get(&player)?;
        let s = p.session.try_read().ok()?;
        Some(s.pk == 1)
    }

    /// The target's current battle id (None if offline).
    pub fn target_battle(&self, player: i64) -> Option<i32> {
        let online = self.online.try_read().ok()?;
        let p = online.get(&player)?;
        let s = p.session.try_read().ok()?;
        Some(s.battle_id)
    }

    /// The designated quan-su member's `Int + Int2` sum,
    /// resolved from the online registry at battle spawn. `0` when no QS is
    /// designated or the member is offline.
    fn leader_qs_int(&self, session: &Session) -> i64 {
        let qs = session.id_qs;
        if qs == 0 {
            return 0;
        }
        if let Ok(online) = self.online.try_read() {
            if let Some(p) = online.get(&i64::from(qs)) {
                if let Ok(s) = p.session.try_read() {
                    return i64::from(s.int1) + i64::from(s.int2);
                }
            }
        }
        0
    }

    fn snapshot(&self, session: &Session) -> PlayerSnapshot {
        PlayerSnapshot {
            texp: i64::from(session.texp),
            job: i64::from(session.job),
            hpx: i64::from(session.hpx),
            spx: i64::from(session.spx),
            hpx2: i64::from(session.hpx2),
            spx2: i64::from(session.spx2),
            god: session.god,
            ghost: session.ghost,
        }
    }

    fn pet_slots(&self, session: &Session) -> [i64; 4] {
        let mut slots = [0i64; 4];
        for pet in session.pets.iter().filter(|p| p.id > 0 && p.is_carried()) {
            let idx = usize::from(pet.stt - 1);
            if idx < 4 {
                slots[idx] = i64::from(pet.id);
            }
        }
        slots
    }

    /// Capture the grid cell records for join (op 0x0B sub 4) and add a battle
    /// participant. Renders one per-cell record per grid cell.
    fn record_join_cells(&self, id: i32, battle: &Battle) {
        let mut records = String::new();
        for key in &battle.keys {
            let cell = battle.list_war.get(key);
            let Some(cell) = cell else { continue };
            if cell.id <= 0 {
                records.push_str(&"0".repeat(48)); // 24-byte empty cell record
                continue;
            }
            let marker = if cell.id == cell.leader_id || cell.id_char == cell.leader_id {
                3u8
            } else {
                100u8
            };
            let id_npc = if matches!(cell.typ, 3 | 7) {
                cell.id_npc_on_map
            } else {
                0
            };
            records.push_str(&format!(
                "{:02X}{:02X}{}{}{}{:02X}{:02X}{}{}{}{}{:02X}{:02X}",
                marker,
                cell.typ,
                encoder::le32(cell.id as u32),
                encoder::le16(id_npc as u16),
                encoder::le32(cell.id_char as u32),
                cell.row,
                cell.col,
                encoder::le16(clamp_u16(cell.hp_max)),
                encoder::le16(clamp_u16(cell.sp_max)),
                encoder::le16(clamp_u16(cell.hp)),
                encoder::le16(clamp_u16(cell.sp)),
                cell.lv.clamp(0, 0xFF) as u8,
                cell.thuoctinh.clamp(0, 0xFF) as u8,
            ));
        }
        if let Ok(mut cells) = self.join_cells.lock() {
            cells.insert(id, (battle.diahinh, records));
        }
    }

    /// Snapshots + pet slots for each online party member (`id_mem`), excluding
    /// the leader (added inside [`BattleService::spawn_battle`]).
    fn party_snapshots(
        &self,
        session: &Session,
    ) -> (HashMap<i64, PlayerSnapshot>, HashMap<i64, [i64; 4]>) {
        let mut players = HashMap::new();
        let mut pets = HashMap::new();
        for mem in session.id_mem.iter().filter(|m| **m > 0) {
            if let Ok(online) = self.online.try_read() {
                if let Some(p) = online.get(&i64::from(*mem)) {
                    if let Ok(s) = p.session.try_read() {
                        players.insert(i64::from(*mem), self.snapshot(&s));
                        pets.insert(i64::from(*mem), self.pet_slots(&s));
                    }
                }
            }
        }
        (players, pets)
    }

    /// Common spawn path: register participants, spawn the battle task, push
    /// the `0BFA` start frames, and record join cells + handle.
    ///
    /// `extra_players`/`extra_pets`/`extra_members` cover participants beyond the
    /// leader (party members, PK opponents); `extra_start` holds their
    /// member-style open frames.
    fn spawn_battle(
        &self,
        mut battle: Battle,
        session: &mut Session,
        extra_players: HashMap<i64, PlayerSnapshot>,
        extra_pets: HashMap<i64, [i64; 4]>,
        extra_members: Vec<i64>,
        extra_start: Vec<StartPacket>,
    ) -> i32 {
        let id = battle.id_battle;
        session.battle_id = id;
        let lid = i64::from(session.id);

        // Snapshot the leader's quan-su designation for the per-turn SP regen
        // block (snapshotted rather than read live).
        battle.leader_id_qs = i64::from(session.id_qs);
        battle.leader_qs_int = self.leader_qs_int(session);

        if session.talking_battle > 0 {
            let dialog_id = if session.idtalking > 0 {
                session.idtalking as u16
            } else {
                session.talking_battle as u16
            };
            battle.opening_dialog = Some(dialog_id);
        }

        let mut players = HashMap::new();
        let mut pet_slots = HashMap::new();
        let mut player_skills = HashMap::new();
        players.insert(lid, self.snapshot(session));
        pet_slots.insert(lid, self.pet_slots(session));
        player_skills.insert(
            lid,
            session.skills.iter().map(|&(sk, _)| sk).collect::<Vec<u16>>(),
        );
        for mem in session.id_mem.iter().filter(|m| **m > 0) {
            if let Ok(online) = self.online.try_read() {
                if let Some(p) = online.get(&i64::from(*mem)) {
                    if let Ok(s) = p.session.try_read() {
                        player_skills.insert(
                            i64::from(*mem),
                            s.skills.iter().map(|&(sk, _)| sk).collect(),
                        );
                    }
                }
            }
        }
        players.extend(extra_players);
        pet_slots.extend(extra_pets);

        let npcs = Arc::new(self.data.npcs.clone());
        let skills = Arc::new(self.data.skills.clone());
        let binary_skills = Arc::new(self.data.binary_skill_defs.clone());
        let items = Arc::new(self.data.items.clone());
        let pet_slots = Arc::new(pet_slots);
        let players = Arc::new(players);
        let player_skills = Arc::new(player_skills);
        let texps: Arc<Vec<TexpRow>> = Arc::new(self.data.texps.clone());

        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(lid);
            members.extend(extra_members.iter().copied());
        }
        if let Ok(mut participants) = self.sink.battle_participants.lock() {
            let entry = participants.entry(id).or_default();
            entry.push(lid);
            entry.extend(extra_members);
        }
        self.record_join_cells(id, &battle);

        let diahinh = battle.diahinh;
        let start = battle.npc_battle_start_packets(diahinh);
        let manager = Arc::clone(&self.manager);
        let sink = Arc::clone(&self.sink) as Arc<dyn BattleSink>;
        let world = self.world.clone();
        let talking_battle = i64::from(session.talking_battle);
        let map_id = i64::from(session.map_id);
        let handle = manager.spawn_timeout_with_skills(
            battle,
            npcs,
            skills,
            binary_skills,
            items,
            pet_slots,
            players,
            Some(player_skills),
            texps,
            self.per_exp,
            world,
            talking_battle,
            map_id,
            self.input_timeout,
            sink,
        );
        if let Ok(mut h) = self.handles.lock() {
            h.insert(id, handle);
        }

        for packet in start.into_iter().chain(extra_start) {
            match packet {
                StartPacket::To { player, frame } => self.sink.send_to(player, frame),
                StartPacket::Map { player, frame } => self.sink.send_map(player, frame),
            }
        }
        id
    }

    /// Spawn the 900 ms `NpcOnMapWalk` task (ticket 20 G1) over the shared
    /// world. The loop re-reads the world through the `RwLock` each tick, so
    /// the battle engine's `_Delay=10` respawn writes and the walk's countdown
    /// stay race-free. No-op when the service has no world (golden replay).
    pub fn spawn_npc_walk(self: &Arc<Self>) {
        let Some(world) = self.world.clone() else {
            return;
        };
        let this = self.clone();
        tokio::spawn(async move {
            let mut tick = 0i64;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                tick += 1;
                let chase = tick >= 3;
                if let Ok(mut w) = world.write() {
                    w.walk_tick(chase, this.as_ref());
                }
                if chase {
                    tick = 0;
                }
            }
        });
    }
}

impl WorldSink for BattleService {
    fn players(&self) -> Vec<WorldPlayer> {
        let Ok(online) = self.online.try_read() else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(online.len());
        for (id, p) in online.iter() {
            if let Ok(s) = p.session.try_read() {
                out.push(WorldPlayer {
                    id: *id,
                    map_id: i64::from(s.map_id),
                    x: i64::from(s.map_x),
                    y: i64::from(s.map_y),
                    battle_id: i64::from(s.battle_id),
                    leader_id: i64::from(s.id_leader),
                    logined: s.logined,
                });
            }
        }
        out
    }

    fn send_player(&self, player: i64, frame: String) {
        self.sink.send_to(player, frame);
    }

    fn send_map(&self, map_id: i64, frame: String) {
        send_to_map(&self.online, map_id, frame);
    }

    /// Engage `player` in the SoLuong TeamDef battle (DiaHinh 4712): set their
    /// `_My_TalkingBattle`, then start the battle through the shared service.
    fn start_teamdef(&self, player: i64, npc_on_map_id: i64, teamdef: Vec<i64>) {
        let session = {
            let online = match self.online.try_read() {
                Ok(o) => o,
                Err(_) => return,
            };
            match online.get(&player) {
                Some(p) => p.session.clone(),
                None => return,
            }
        };
        let mut s = match session.try_write() {
            Ok(g) => g,
            Err(_) => return,
        };
        s.talking_battle = npc_on_map_id as i32;
        let trigger = crate::server::handlers::quest::BattleTrigger {
            teamdef,
            diahinh: 4712,
        };
        self.start_teamdef_battle(&mut s, &trigger);
    }
}
