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
//! - op 0x32 handlers call `submit_command`.
//! - when the task ends, `battle_ended` runs the quest-win progression for the leader if
//!   a quest talk is pending and the players won, then cleans up.

use crate::battle::construction::{Battle, StartPacket};
use crate::battle::engine::get_hp_max;
use crate::battle::manager::{BattleHandle, BattleManager, BattleSink};
use crate::battle::npc_world::{WorldPlayer, WorldSink};
use crate::battle::npc_world::NpcWorld;
use crate::battle::packets;
use crate::battle::runner::{
    BattleCommand, DbTarget, DbUpdate, Out, Outcome, PlayerSnapshot, Stat,
};
use crate::data::loader::GameData;
use crate::data::tables::TexpRow;
use crate::protocol::encoder;
use crate::server::handlers::quest::BattleTrigger;
use crate::server::session::Session;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// A registered online player: their shared session + frame channel.
pub struct OnlinePlayer {
    pub session: Arc<tokio::sync::RwLock<Session>>,
    pub frames: mpsc::UnboundedSender<String>,
}

impl std::fmt::Debug for OnlinePlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnlinePlayer")
            .field("session", &self.session)
            .finish_non_exhaustive()
    }
}

type OnlineMap = HashMap<i64, OnlinePlayer>;

/// The [`BattleSink`] implementation shared by all battles of one service.
struct BattleSinkImpl {
    online: Arc<tokio::sync::RwLock<OnlineMap>>,
    /// Player ids currently in a battle (receive `broadcast`/`Out::Broadcast`).
    members: Arc<Mutex<HashSet<i64>>>,
    data: Arc<GameData>,
    /// Optional MySQL pool used for post-battle persistence (quest/homdo).
    /// Shared (same Arc) with the owning [`BattleService`] so `with_pool` can
    /// set it after construction, before any battle starts.
    pool: Arc<tokio::sync::RwLock<Option<sqlx::MySqlPool>>>,
}

impl BattleSink for BattleSinkImpl {
    fn send_to(&self, player: i64, frame: String) {
        push(&self.online, player, frame);
    }

    fn send_map(&self, player: i64, frame: String) {
        if let Ok(online) = self.online.try_read() {
            for (id, p) in online.iter() {
                if *id != player {
                    let _ = p.frames.send(frame.clone());
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
        if let Ok(online) = self.online.try_read() {
            apply_db_update(&online, update);
        }
    }

    fn apply_drop(&self, drop: Out) {
        if let Out::Drop { item_id, owner, .. } = drop {
            if let Ok(online) = self.online.try_read() {
                if let Some(p) = online.get(&owner) {
                    let mut s = match p.session.try_write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    s.add_homdo_item(crate::server::session::InventoryItem {
                        id: item_id as u16,
                        count: 1,
                        loai: 1,
                        doben: 100,
                        ..Default::default()
                    });
                }
            }
        }
    }

    fn apply_catch(&self, owner: i64, npc_id: i64) {
        if let Ok(online) = self.online.try_read() {
            if let Some(p) = online.get(&owner) {
                if let Ok(mut s) = p.session.try_write() {
                    add_pet_to_session(&mut s, npc_id as u16);
                }
            }
        }
    }

    fn apply_fled(&self, player: i64) {
        if let Ok(online) = self.online.try_read() {
            if let Some(p) = online.get(&player) {
                if let Ok(mut s) = p.session.try_write() {
                    s.battle_id = 0;
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
        if let Ok(online) = self.online.try_read() {
            if let Some(p) = online.get(&owner) {
                if let Ok(mut s) = p.session.try_write() {
                    if let Some(pet) = s.pets.iter_mut().find(|p| i64::from(p.stt) == stt) {
                        pet.texp = pet.texp.saturating_add(exp as u32);
                    }
                }
            }
        }
    }

    fn battle_ended(&self, _id: i32, outcome: Outcome) {
        // Clear battle state for every participant (battle is over) and
        // snapshot their sessions for post-battle persistence.
        let mut ended_sessions: Vec<Session> = Vec::new();
        if let Ok(mut members) = self.members.lock() {
            let ids: Vec<i64> = members.iter().copied().collect();
            if let Ok(online) = self.online.try_read() {
                for id in ids {
                    if let Some(p) = online.get(&id) {
                        if let Ok(mut s) = p.session.try_write() {
                            s.battle_id = 0;
                            ended_sessions.push(s.clone());
                        }
                    }
                }
            }
            members.clear();
        }
        // G3 — batch-persist the post-battle player stats (Hp/Sp/Texp/Lv/
        // HpMax/SpMax/Point/SkillPoint) + pet stats (texp) for every member on
        // win/lose/flee. `None` pool = no-op (golden replay).
        if !ended_sessions.is_empty() {
            if let Ok(pool) = self.pool.try_read() {
                if let Some(pool) = pool.clone() {
                    tokio::spawn(async move {
                        let refs: Vec<&Session> = ended_sessions.iter().collect();
                        crate::db::persist::persist_sessions_transaction(
                            Some(&pool),
                            &refs,
                            &["stats", "pet"],
                        )
                        .await;
                    });
                }
            }
        }
        if outcome != Outcome::PlayerWin {
            // Defeat: run the OnLose dialog progression for every quest-battle
            // session (ticket 19 review #8 — PlayerLose must call OnLose).
            if let Ok(online) = self.online.try_read() {
                for (player, p) in online.iter() {
                    let _ = player;
                    if let Ok(mut s) = p.session.try_write() {
                        if s.talking_battle <= 0 {
                            continue;
                        }
                        let mut frames = Vec::new();
                        crate::server::handlers::quest::quest_lose_frames(
                            &mut s,
                            self.data.as_ref(),
                            &mut frames,
                        );
                        for f in frames {
                            let _ = p.frames.send(f);
                        }
                    }
                }
            }
            return;
        }
        if let Ok(online) = self.online.try_read() {
            // Snapshot member sessions so the quest-win closure can reach them.
            let members: HashMap<i64, Arc<tokio::sync::RwLock<Session>>> = online
                .iter()
                .map(|(id, p)| (*id, p.session.clone()))
                .collect();
            for (player, p) in online.iter() {
                if let Ok(mut s) = p.session.try_write() {
                    if s.talking_battle <= 0 {
                        continue;
                    }
                    let mut frames = Vec::new();
                    let mut member = |mem: i64| members.get(&mem).cloned();
                    crate::server::handlers::quest::battle_quest_win(
                        &mut s,
                        self.data.as_ref(),
                        &mut frames,
                        &mut member,
                    );
                    for f in frames {
                        let _ = p.frames.send(f);
                    }
                    // Persist the quest-step + item mutations arising from OnWin
                    // (scoped by player_id) so a crash after battle never loses
                    // the granted rewards / advanced steps.
                    if let Ok(pool) = self.pool.try_read() {
                        if let Some(pool) = pool.clone() {
                            let session = s.clone();
                            tokio::spawn(async move {
                                crate::db::persist::persist_sessions_transaction(
                                    Some(&pool),
                                    &[&session],
                                    &["quest", "homdo", "trangbi"],
                                )
                                .await;
                            });
                        }
                    }                }
                let _ = player;
            }
        }
    }
}

fn push(online: &tokio::sync::RwLock<OnlineMap>, player: i64, frame: String) {
    if let Ok(online) = online.try_read() {
        if let Some(p) = online.get(&player) {
            let _ = p.frames.send(frame);
        }
    }
}

/// Send `frame` to every online player whose session sits on `map_id`
/// (includes the sender's own map). Shared by the
/// walk loop's wander/chase fan-out and the battle respawn broadcast.
fn send_to_map(online: &tokio::sync::RwLock<OnlineMap>, map_id: i64, frame: String) {
    if let Ok(online) = online.try_read() {
        for (id, p) in online.iter() {
            if let Ok(s) = p.session.try_read() {
                if i64::from(s.map_id) == map_id {
                    let _ = p.frames.send(frame.clone());
                }
            }
            let _ = id;
        }
    }
}

/// Apply a battle DB write to the shared session and (for player targets with a
/// status byte) push the `F4440C000801` status frame. Hpmax/Spmax mutate only.
fn apply_db_update(online: &OnlineMap, update: DbUpdate) {
    match update.target {
        DbTarget::Player(id) => {
            if let Some(p) = online.get(&id) {
                if let Ok(mut s) = p.session.try_write() {
                    let frame = apply_player_stat(&mut s, update.stat, update.value);
                    if let Some(f) = frame {
                        let _ = p.frames.send(f);
                    }
                }
            }
        }
        DbTarget::Pet { owner, stt } => {
            if let Some(p) = online.get(&owner) {
                if let Ok(mut s) = p.session.try_write() {
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

fn apply_player_stat(s: &mut Session, stat: Stat, value: i64) -> Option<String> {
    let (byte, apply): (Option<u8>, fn(&mut Session, i64)) = match stat {
        Stat::Hp => (Some(0x19), |s, v| s.hp = clamp_u16(v)),
        Stat::Sp => (Some(0x1A), |s, v| s.sp = clamp_u16(v)),
        Stat::Texp => (Some(0x24), |s, v| {
            s.texp = v.clamp(0, u32::MAX as i64) as u32
        }),
        Stat::Lv => (Some(0x23), |s, v| s.level = v.clamp(0, 0xFF) as u8),
        Stat::Hpmax => (None, |s, v| s.hp_max = clamp_u16(v)),
        Stat::Spmax => (None, |s, v| s.sp_max = clamp_u16(v)),
        Stat::Point => (Some(0x26), |s, v| s.point = clamp_u16(v)),
        Stat::SkillPoint => (Some(0x25), |s, v| s.skill_point = clamp_u16(v)),
        Stat::Fai => (Some(0x40), |s, v| s.tiengtam = clamp_u16(v)),
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
    /// Optional MySQL pool used for post-battle persistence (quest/homdo);
    /// shared with the sink via `Arc` so `with_pool` is visible to battles.
    pool: Option<Arc<tokio::sync::RwLock<Option<sqlx::MySqlPool>>>>,
    /// The shared map-NPC world (ticket 20 G1/G2), built from `npcs_on_map`.
    world: Option<Arc<std::sync::RwLock<NpcWorld>>>,
    /// Synchronous handle registry for sync handler access (op 0x32, join).
    handles: Mutex<HashMap<i32, BattleHandle>>,
    /// Pre-rendered join cell records per battle id (the grid lives in the
    /// async task, so we capture it at spawn).
    join_cells: Mutex<HashMap<i32, (i32, String)>>,
    per_exp: i64,
    next_battle: AtomicI32,
    /// Per-turn input wait (default 21 s).
    input_timeout: std::time::Duration,
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
            Some(Arc::new(std::sync::RwLock::new(NpcWorld::new(&data.npc_on_map))))
        };
        let sink = Arc::new(BattleSinkImpl {
            online: Arc::clone(&online),
            members: Arc::new(Mutex::new(HashSet::new())),
            data: Arc::clone(&data),
            pool: Arc::clone(&pool),
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
        }
    }

    /// Attach the MySQL pool used for post-battle persistence.
    pub fn with_pool(self, pool: sqlx::MySqlPool) -> Self {
        if let Some(p) = &self.pool {
            if let Ok(mut guard) = p.try_write() {
                *guard = Some(pool);
            }
        }
        self
    }

    /// The configured post-battle persistence pool (best-effort setter used by
    /// tests); `None` when no DB is attached.
    pub fn pool(&self) -> Option<sqlx::MySqlPool> {
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
    ) -> mpsc::UnboundedReceiver<String> {
        let (tx, rx) = mpsc::unbounded_channel();
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

    pub fn unregister(&self, player: i64) {
        if let Ok(mut online) = self.online.try_write() {
            online.remove(&player);
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
    pub fn start_npc_battle(&self, session: &mut Session, npc_id: i64, npc_on_map_id: i64) -> i32 {
        let Some(npc) = self.data.npcs.get(&npc_id).cloned() else {
            return 0;
        };
        let id = self.next_battle_id();
        let battle =
            Battle::npc_battle(id, session, i64::from(session.id), &npc, npc_on_map_id, 112);
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
    pub fn start_pk_battle(&self, session: &mut Session, opponent: i64) -> i32 {
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
        let mut battle = Battle::new(id, 112);
        battle.add_player(session, i64::from(session.id), 3, 2);
        battle.load_leader_pets(session, i64::from(session.id), 3);
        let opp_guard = match opp.try_read() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        battle.add_player(&opp_guard, opponent, 0, 2);
        let opponent_start = battle.member_battle_frame(112, opponent);
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

    /// Register a player in an existing battle (op 0x0B sub 4 join).
    pub fn join_battle(&self, session: &mut Session, battle_id: i32) -> bool {
        if session.battle_id != 0 || !self.battle_exists(battle_id) {
            return false;
        }
        session.battle_id = battle_id;
        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(i64::from(session.id));
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

    /// Leave the current battle (op 0x0B sub 1).
    pub fn leave_battle(&self, session: &mut Session) {
        session.battle_id = 0;
        if let Ok(mut members) = self.sink.members.lock() {
            members.remove(&i64::from(session.id));
        }
    }

    /// Submit an op 0x32 command to the player's battle.
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
        }
    }

    fn pet_slots(&self, session: &Session) -> [i64; 4] {
        let mut slots = [0i64; 4];
        for (i, pet) in session.pets.iter().take(4).enumerate() {
            slots[i] = i64::from(pet.id);
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

        let mut players = HashMap::new();
        let mut pet_slots = HashMap::new();
        players.insert(lid, self.snapshot(session));
        pet_slots.insert(lid, self.pet_slots(session));
        players.extend(extra_players);
        pet_slots.extend(extra_pets);

        let npcs = Arc::new(self.data.npcs.clone());
        let skills = Arc::new(self.data.skills.clone());
        let items = Arc::new(self.data.items.clone());
        let pet_slots = Arc::new(pet_slots);
        let players = Arc::new(players);
        let texps: Arc<Vec<TexpRow>> = Arc::new(self.data.texps.clone());

        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(lid);
            members.extend(extra_members);
        }
        self.record_join_cells(id, &battle);

        let diahinh = battle.diahinh;
        let start = battle.npc_battle_start_packets(diahinh);
        let manager = Arc::clone(&self.manager);
        let sink = Arc::clone(&self.sink) as Arc<dyn BattleSink>;
        let world = self.world.clone();
        let talking_battle = i64::from(session.talking_battle);
        let map_id = i64::from(session.map_id);
        let handle = manager.spawn_timeout(
            battle,
            npcs,
            skills,
            items,
            pet_slots,
            players,
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
