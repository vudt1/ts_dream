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
//! - when the task ends, `battle_ended` runs `BattleQuestWin` for the leader if
//!   a quest talk is pending and the players won, then cleans up.

use crate::battle::construction::{Battle, StartPacket};
use crate::battle::engine::get_hp_max;
use crate::battle::manager::{BattleHandle, BattleManager, BattleSink};
use crate::battle::packets;
use crate::battle::runner::{BattleCommand, DbTarget, DbUpdate, Out, Outcome, PlayerSnapshot, Stat};
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

    fn apply_respawn(&self, _npc_id: i64, _x: i64, _y: i64) {
        // Map NPC instance state is not modelled in-memory yet.
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
        // Clear battle state for every participant (battle is over).
        if let Ok(mut members) = self.members.lock() {
            let ids: Vec<i64> = members.iter().copied().collect();
            if let Ok(online) = self.online.try_read() {
                for id in ids {
                    if let Some(p) = online.get(&id) {
                        if let Ok(mut s) = p.session.try_write() {
                            s.battle_id = 0;
                        }
                    }
                }
            }
            members.clear();
        }
        if outcome != Outcome::PlayerWin {
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
                }
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
        Stat::Texp => (Some(0x24), |s, v| s.texp = v.clamp(0, u32::MAX as i64) as u32),
        Stat::Lv => (Some(0x23), |s, v| s.level = v.clamp(0, 0xFF) as u8),
        Stat::Hpmax => (None, |s, v| s.hp_max = clamp_u16(v)),
        Stat::Spmax => (None, |s, v| s.sp_max = clamp_u16(v)),
        Stat::Point => (Some(0x26), |s, v| s.point = clamp_u16(v)),
        Stat::SkillPoint => (Some(0x25), |s, v| s.skill_point = clamp_u16(v)),
        Stat::Fai => (Some(0x40), |s, v| s.tiengtam = clamp_u16(v)),
    };
    apply(s, value);
    byte.map(|b| {
        packets::status_update(b, value.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    })
}

fn clamp_u16(v: i64) -> u16 {
    v.clamp(0, 0xFFFF) as u16
}

fn add_pet_to_session(s: &mut Session, npc_id: u16) {
    if s.pets.iter().any(|p| p.id == npc_id) {
        return;
    }
    let stt = (s.pets.len() as u8 + 1).max(1);
    let hp_max = get_hp_max(0, 0, 1, 0) as u16;
    s.pets.push(crate::server::session::PetState {
        stt,
        id: npc_id,
        level: 1,
        thuoctinh: 1,
        hp: hp_max,
        hp_max,
        ..Default::default()
    });
}

/// The server-side battle service.
pub struct BattleService {
    pub manager: Arc<BattleManager>,
    pub data: Arc<GameData>,
    sink: Arc<BattleSinkImpl>,
    online: Arc<tokio::sync::RwLock<OnlineMap>>,
    /// Synchronous handle registry for sync handler access (op 0x32, join).
    handles: Mutex<HashMap<i32, BattleHandle>>,
    /// Pre-rendered join cell records per battle id (the C# join loop renders
    /// the grid; the grid lives in the async task, so we capture it at spawn).
    join_cells: Mutex<HashMap<i32, (i32, String)>>,
    per_exp: i64,
    next_battle: AtomicI32,
    /// Per-turn input wait (default 21 s, mirrors the C# ≤21 s poll).
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
        let sink = Arc::new(BattleSinkImpl {
            online: Arc::clone(&online),
            members: Arc::new(Mutex::new(HashSet::new())),
            data: Arc::clone(&data),
        });
        BattleService {
            manager: Arc::new(BattleManager::new()),
            data,
            sink,
            online,
            handles: Mutex::new(HashMap::new()),
            join_cells: Mutex::new(HashMap::new()),
            per_exp: 1,
            next_battle: AtomicI32::new(1),
            input_timeout: std::time::Duration::from_secs(21),
        }
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
            online.insert(player, OnlinePlayer { session, frames: tx });
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

    /// The next battle id (dedicated counter; `BattleManager` is shared).
    pub fn next_battle_id(&self) -> i32 {
        self.next_battle.fetch_add(1, Ordering::SeqCst)
    }

    /// Start an NPC battle (`TheBattle(leader, npcId, onMap, 112)`).
    ///
    /// Returns the new battle id, or 0 if the NPC template is missing.
    pub fn start_npc_battle(&self, session: &mut Session, npc_id: i64, npc_on_map_id: i64) -> i32 {
        let Some(npc) = self.data.npcs.get(&npc_id).cloned() else {
            return 0;
        };
        let id = self.next_battle_id();
        let battle = Battle::npc_battle(
            id,
            session,
            i64::from(session.id),
            &npc,
            npc_on_map_id,
            112,
        );
        self.launch(battle, session)
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
        self.launch(battle, session)
    }

    /// Start a PK battle (`TheBattle(leader, opponent, 112)`).
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
        {
            let opp = opp.try_read();
            match opp {
                Ok(o) => battle.add_player(&o, opponent, 0, 2),
                Err(_) => return 0,
            }
        }
        self.launch_pk(battle, session, opponent)
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
        self.launch(battle, session)
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
        self.sink.send_to(i64::from(session.id), packets::battle_trailer());
        true
    }

    fn battle_exists(&self, battle_id: i32) -> bool {
        self.handles.lock().map(|h| h.contains_key(&battle_id)).unwrap_or(false)
    }

    /// Build the op 0x0B sub-4 join frame: `0BFA` + LE16(diahinh) + `0402` +
    /// self record + the 20 grid cell records (captured at spawn).
    fn build_join_frame(&self, session: &Session, battle_id: i32) -> String {
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
        format!(
            "F444{}0BFA{}{}",
            encoder::le16((4 + text.len() / 2) as u16),
            encoder::le16(diahinh as u16),
            text
        )
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
    /// participant. Renders one per-cell record per the C# join loop.
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

    /// Common spawn path: register participants, spawn the battle task, push
    /// the `0BFA` start frames, and record join cells + handle.
    fn launch(&self, battle: Battle, session: &mut Session) -> i32 {
        let id = battle.id_battle;
        session.battle_id = id;

        let mut players = HashMap::new();
        let mut pet_slots = HashMap::new();
        let lid = i64::from(session.id);
        players.insert(lid, self.snapshot(session));
        pet_slots.insert(lid, self.pet_slots(session));
        // Include party members' snapshots where registered.
        for mem in session.id_mem.iter().filter(|m| **m > 0) {
            if let Ok(online) = self.online.try_read() {
                if let Some(p) = online.get(&i64::from(*mem)) {
                    if let Ok(s) = p.session.try_read() {
                        players.insert(i64::from(*mem), self.snapshot(&s));
                        pet_slots.insert(i64::from(*mem), self.pet_slots(&s));
                    }
                }
            }
        }

        let npcs = Arc::new(self.data.npcs.clone());
        let skills = Arc::new(self.data.skills.clone());
        let items = Arc::new(self.data.items.clone());
        let pet_slots = Arc::new(pet_slots);
        let players = Arc::new(players);
        let texps: Arc<Vec<TexpRow>> = Arc::new(self.data.texps.clone());

        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(lid);
        }
        self.record_join_cells(id, &battle);

        let diahinh = battle.diahinh;
        let start = battle.npc_battle_start_packets(diahinh);
        let manager = Arc::clone(&self.manager);
        let sink = Arc::clone(&self.sink) as Arc<dyn BattleSink>;
        let handle = manager.spawn_timeout(
            battle,
            npcs,
            skills,
            items,
            pet_slots,
            players,
            texps,
            self.per_exp,
            self.input_timeout,
            sink,
        );
        if let Ok(mut h) = self.handles.lock() {
            h.insert(id, handle);
        }

        for packet in start {
            match packet {
                StartPacket::To { player, frame } => self.sink.send_to(player, frame),
                StartPacket::Map { player, frame } => self.sink.send_map(player, frame),
            }
        }
        id
    }

    fn launch_pk(&self, battle: Battle, session: &mut Session, opponent: i64) -> i32 {
        let id = battle.id_battle;
        session.battle_id = id;
        let lid = i64::from(session.id);

        let mut players = HashMap::new();
        let mut pet_slots = HashMap::new();
        players.insert(lid, self.snapshot(session));
        pet_slots.insert(lid, self.pet_slots(session));
        if let Ok(online) = self.online.try_read() {
            if let Some(p) = online.get(&opponent) {
                if let Ok(o) = p.session.try_read() {
                    players.insert(opponent, self.snapshot(&o));
                    pet_slots.insert(opponent, self.pet_slots(&o));
                }
            }
        }

        if let Ok(mut members) = self.sink.members.lock() {
            members.insert(lid);
            members.insert(opponent);
        }
        self.record_join_cells(id, &battle);

        let start = battle.npc_battle_start_packets(112);
        // The opponent gets their own member-style open frame (PK view).
        let opponent_start = battle.member_battle_frame(112, opponent);
        let npcs = Arc::new(self.data.npcs.clone());
        let skills = Arc::new(self.data.skills.clone());
        let items = Arc::new(self.data.items.clone());
        let pet_slots = Arc::new(pet_slots);
        let players = Arc::new(players);
        let texps: Arc<Vec<TexpRow>> = Arc::new(self.data.texps.clone());
        let manager = Arc::clone(&self.manager);
        let sink = Arc::clone(&self.sink) as Arc<dyn BattleSink>;
        let handle = manager.spawn_timeout(
            battle,
            npcs,
            skills,
            items,
            pet_slots,
            players,
            texps,
            self.per_exp,
            self.input_timeout,
            sink,
        );
        if let Ok(mut h) = self.handles.lock() {
            h.insert(id, handle);
        }
        let all_start = start
            .into_iter()
            .chain(opponent_start)
            .collect::<Vec<_>>();
        for packet in all_start {
            match packet {
                StartPacket::To { player, frame } => self.sink.send_to(player, frame),
                StartPacket::Map { player, frame } => self.sink.send_map(player, frame),
            }
        }
        id
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::tables::{Npc, Skill};
    use crate::server::session::Session;

    fn game_data() -> GameData {
        let mut data = GameData::default();
        data.npcs.insert(
            9001,
            Npc {
                id: 9001,
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
                sp: 5,
                lv_max: 10,
                skill_type: 1,
                do_manh: 10,
                sl_danh: 1,
                delay: 1000,
                ..Default::default()
            },
        );
        data
    }

    fn strong_session(id: u32) -> Arc<tokio::sync::RwLock<Session>> {
        let s = Arc::new(tokio::sync::RwLock::new(Session::new()));
        {
            let mut s = s.try_write().expect("lock");
            s.id = id;
            s.level = 50;
            s.hp = 5000;
            s.hp_max = 5000;
            s.sp = 500;
            s.sp_max = 500;
            s.atk = 300;
            s.def = 50;
            s.agi = 100;
            s.int1 = 100;
            s.hp_max = crate::battle::engine::get_hp_max(0, 0, 50, 6) as u16;
            s.hp = s.hp_max;
            s.skills.push((10000, 10));
        }
        s
    }

    /// Drain the receiver (up to 200 frames, up to 300 ms) and return them.
    async fn drain(rx: &mut mpsc::UnboundedReceiver<String>) -> Vec<String> {
        let mut out = Vec::new();
        let mut collected = 0usize;
        while collected < 200 {
            match tokio::time::timeout(
                std::time::Duration::from_millis(1),
                rx.recv(),
            )
            .await
            {
                Ok(Some(f)) => {
                    out.push(f);
                    collected += 1;
                }
                _ => break,
            }
        }
        out
    }

    async fn wait_no_members(service: &Arc<BattleService>) {
        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            let empty = service
                .sink
                .members
                .lock()
                .map(|m| m.is_empty())
                .unwrap_or(false);
            if empty {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("battle did not end in time");
    }

    #[tokio::test]
    async fn npc_battle_runs_to_win_and_clears_members() {
        let mut service = BattleService::new(Arc::new(game_data()));
        service.set_input_timeout(std::time::Duration::from_millis(50));
        let service = Arc::new(service);

        let session = strong_session(300001);
        let mut rx = service.register(300001, Arc::clone(&session));

        let start_frames = {
            let mut s = session.write().await;
            service.start_npc_battle(&mut s, 9001, 11)
        };
        assert!(start_frames > 0);

        // Actor command: leader at (3,2) basic-attacks the NPC at (0,2).
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
            assert!(service.submit_command(&s, cmd), "command accepted");
        }

        // The battle task ends; collect its frames.
        wait_no_members(&service).await;
        let frames = drain(&mut rx).await;

        // Open board + turn-action frames are emitted (acting `F44404003505`
        // is sent by the op 0x32 *handler*, not the battle task).
        assert!(
            frames.iter().any(|f| f.starts_with("F4441C000BFA")),
            "open board frame expected: {frames:?}"
        );
        assert!(
            frames.iter().any(|f| f.contains("3201")),
            "turn-action frame expected"
        );

        // The participant's battle id was cleared and exit frames sent.
        let s = session.read().await;
        assert_eq!(s.battle_id, 0, "battle id cleared after end");
        drop(s);
    }

    #[tokio::test]
    async fn join_frame_has_24byte_cell_records() {
        let mut service = BattleService::new(Arc::new(game_data()));
        service.set_input_timeout(std::time::Duration::from_millis(50));
        let service = Arc::new(service);

        let session = strong_session(300001);
        let mut rx = service.register(300001, Arc::clone(&session));
        let battle_id = {
            let mut s = session.write().await;
            service.start_npc_battle_seeded(&mut s, 9001, 11, 1, 2, 3)
        };
        assert!(battle_id > 0);
        // Build the join frame and verify the length header matches the payload.
        let join = {
            let s = session.read().await;
            service.build_join_frame(&s, battle_id)
        };
        assert!(join.starts_with("F444"));
        let len_bytes = encoder::u16_le(
            hex_u8(&join[4..6]),
            hex_u8(&join[6..8]),
        ) as usize;
        let payload = &join[8..];
        assert_eq!(
            len_bytes,
            payload.len() / 2,
            "join frame length header must match payload (frame={join})"
        );
        // Payload = 0BFA + diahinh + 0402 + self(22B) + 20 × 24-byte cells.
        assert_eq!(payload.len() / 2, 2 + 2 + 2 + 22 + 20 * 24);
        let _ = rx.try_recv();
        let _ = battle_id;
    }

    fn hex_u8(hex: &str) -> u8 {
        u8::from_str_radix(hex, 16).unwrap()
    }

    #[tokio::test]
    async fn battle_end_triggers_quest_win() {
        let mut data = game_data();
        data.talks.insert(
            "12001:NPC:7:0".to_string(),
            crate::data::tables::QuestDef {
                map_id: 12001,
                id: 7,
                on_win: crate::data::tables::QuestResult {
                    rewards: vec![(46001, 5, 0)],
                    message: "Win".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let mut service = BattleService::new(Arc::new(data));
        service.set_input_timeout(std::time::Duration::from_millis(50));
        let service = Arc::new(service);

        let session = strong_session(300001);
        {
            let mut s = session.write().await;
            s.map_id = 12001;
            s.talking_battle = 7;
        }
        let mut rx = service.register(300001, Arc::clone(&session));

        {
            let mut s = session.write().await;
            service.start_npc_battle(&mut s, 9001, 11);
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
        wait_no_members(&service).await;
        let frames = drain(&mut rx).await;

        // Reward granted to the leader.
        let s = session.read().await;
        assert!(
            s.homdo.iter().any(|i| i.id == 46001 && i.count == 5),
            "quest reward item granted: {:?}",
            s.homdo
        );
        // Red message frame emitted.
        assert!(frames.iter().any(|f| f.contains("020B")));
        assert_eq!(s.talking_battle, 0, "quest talk cleared");
    }
}
