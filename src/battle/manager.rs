//! Battle manager (Chapter 1 §1.4, Chapter 6) — registry + async per-battle tasks.
//!
//! One battle runs on its own `tokio::spawn` task. Player commands (op 0x32)
//! arrive through a per-battle `mpsc` channel; the task collects them each turn
//! (with a per-turn timeout, default ≤21 s), runs the deterministic
//! `Battle::run_turn`, and dispatches every `runner::Out` through a `BattleSink`.
//! The grid + RNG live only inside the task, so battle state is race-free.

use crate::battle::construction::Battle;
use crate::battle::npc_world::NpcWorld;
use crate::battle::runner::{BattleCommand, BattleData, DbUpdate, Out, Outcome, PlayerSnapshot};
use crate::data::tables::{Item, Npc, Skill, TexpRow};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::{mpsc, RwLock};

/// A command submitted by one player for the current turn.
#[derive(Debug, Clone, Copy)]
pub struct PlayerInput {
    pub player: i64,
    pub cmd: BattleCommand,
}

/// Sink the battle task pushes events through to reach real clients/DB.
pub trait BattleSink: Send + Sync + 'static {
    fn send_to(&self, player: i64, frame: String);
    fn send_map(&self, player: i64, frame: String);
    fn broadcast(&self, frame: String);
    fn apply_db(&self, update: DbUpdate);
    fn apply_drop(&self, drop: crate::battle::runner::Out);
    fn apply_catch(&self, owner: i64, npc_id: i64);
    fn apply_fled(&self, player: i64);
    fn apply_respawn(&self, npc_id: i64, map_id: i64, x: i64, y: i64);
    fn apply_pet_exp(&self, owner: i64, stt: i64, exp: i64);
    /// The battle task finished (`PlayerWin`/`PlayerLose`/`PlayerFled`).
    fn battle_ended(&self, id: i32, outcome: Outcome);
}

/// A live battle handle (used to send commands and join/leave).
#[derive(Debug, Clone)]
pub struct BattleHandle {
    pub id: i32,
    tx: mpsc::UnboundedSender<PlayerInput>,
}

impl BattleHandle {
    /// Submit a command for the current turn.
    pub fn command(&self, input: PlayerInput) -> bool {
        self.tx.send(input).is_ok()
    }
}

/// The registry of live battles (id assignment precedes increment).
pub struct BattleManager {
    battles: RwLock<HashMap<i32, BattleHandle>>,
    next_id: AtomicI32,
    default_timeout: std::time::Duration,
}

impl Default for BattleManager {
    fn default() -> Self {
        BattleManager::new()
    }
}

impl BattleManager {
    pub fn new() -> Self {
        BattleManager {
            battles: RwLock::new(HashMap::new()),
            next_id: AtomicI32::new(1),
            default_timeout: std::time::Duration::from_secs(21),
        }
    }

    /// The next battle id (assigns before increment).
    pub fn next_id(&self) -> i32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub async fn get(&self, id: i32) -> Option<BattleHandle> {
        self.battles.read().await.get(&id).cloned()
    }

    pub async fn len(&self) -> usize {
        self.battles.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.battles.read().await.is_empty()
    }

    /// Spawn a battle onto its own task, registering it so `get(id)` works.
    /// `sink` receives every `Out`. Builds its own `BattleData` from the given
    /// tables (which the task holds for its whole lifetime).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        self: &Arc<Self>,
        battle: Battle,
        npcs: Arc<HashMap<i64, Npc>>,
        skills: Arc<HashMap<i64, Skill>>,
        items: Arc<HashMap<i64, Item>>,
        pet_slots: Arc<HashMap<i64, [i64; 4]>>,
        players: Arc<HashMap<i64, PlayerSnapshot>>,
        texps: Arc<Vec<TexpRow>>,
        per_exp: i64,
        world: Option<Arc<StdRwLock<NpcWorld>>>,
        talking_battle: i64,
        map_id: i64,
        sink: Arc<dyn BattleSink>,
    ) -> BattleHandle {
        self.spawn_timeout(
            battle,
            npcs,
            skills,
            items,
            pet_slots,
            players,
            texps,
            per_exp,
            world,
            talking_battle,
            map_id,
            self.default_timeout,
            sink,
        )
    }

    /// Spawn with a custom per-turn input timeout (shorter makes tests fast).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_timeout(
        self: &Arc<Self>,
        battle: Battle,
        npcs: Arc<HashMap<i64, Npc>>,
        skills: Arc<HashMap<i64, Skill>>,
        items: Arc<HashMap<i64, Item>>,
        pet_slots: Arc<HashMap<i64, [i64; 4]>>,
        players: Arc<HashMap<i64, PlayerSnapshot>>,
        texps: Arc<Vec<TexpRow>>,
        per_exp: i64,
        world: Option<Arc<StdRwLock<NpcWorld>>>,
        talking_battle: i64,
        map_id: i64,
        timeout: std::time::Duration,
        sink: Arc<dyn BattleSink>,
    ) -> BattleHandle {
        let id = battle.id_battle;
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = BattleHandle { id, tx };

        let manager = self.clone();
        tokio::spawn(async move {
            let mut battle = battle;
            let mut rx = rx;
            // Build the per-task data from the tables it now owns.
            let data = BattleData::new(
                &npcs,
                &skills,
                &items,
                &pet_slots,
                &players,
                &texps,
                world.as_ref().map(|w| w.as_ref()),
                talking_battle,
                map_id,
            );
            let mut out: Vec<Out> = Vec::new();
            loop {
                out.clear();
                // Collect commands with a timeout.
                let commands = collect_commands(&mut rx, timeout, &battle).await;
                let outcome = battle.run_turn(&data, &commands, &mut out);
                dispatch(&out, sink.as_ref());
                if outcome != Outcome::Running {
                    let fled = outcome == Outcome::PlayerFled;
                    let won = outcome == Outcome::PlayerWin;
                    out.clear();
                    battle.finish(&data, per_exp, fled, won, &mut out);
                    dispatch(&out, sink.as_ref());
                    sink.battle_ended(id, outcome);
                    break;
                }
            }
            manager.battles.write().await.remove(&id);
        });

        // Register the handle eagerly.
        let manager2 = self.clone();
        let handle2 = handle.clone();
        tokio::spawn(async move {
            manager2.battles.write().await.insert(id, handle2);
        });

        handle
    }
}
async fn collect_commands(
    rx: &mut mpsc::UnboundedReceiver<PlayerInput>,
    timeout: std::time::Duration,
    battle: &Battle,
) -> HashMap<i64, BattleCommand> {
    let mut commands = HashMap::new();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        // Every living player-type cell needs a command (auto actions are handled
        // inside run_turn for NPCs, so only human players gate the wait).
        let waiting_players: Vec<i64> = (0..4u8)
            .flat_map(|r| (0..5u8).map(move |c| (r, c)))
            .filter_map(|(r, c)| {
                let cell = battle.cell(r, c)?;
                if cell.hp > 0 && cell.typ == 2 {
                    Some(cell.id)
                } else {
                    None
                }
            })
            .collect();
        let all_ready = waiting_players.iter().all(|p| commands.contains_key(p));
        if all_ready {
            return commands;
        }
        if tokio::time::Instant::now() >= deadline {
            return commands;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(input)) => {
                commands.insert(input.player, input.cmd);
            }
            _ => return commands, // channel closed / timeout
        }
    }
}

fn dispatch(out: &[Out], sink: &dyn BattleSink) {
    use Out::*;
    for o in out {
        match o {
            Broadcast(f) => sink.broadcast(f.clone()),
            ToPlayer(p, f) => sink.send_to(*p, f.clone()),
            MapBroadcast { player, frame } => sink.send_map(*player, frame.clone()),
            Db(u) => sink.apply_db(*u),
            Drop {
                item_id,
                npc_row,
                npc_col,
                row,
                col,
                owner,
            } => sink.apply_drop(Out::Drop {
                item_id: *item_id,
                npc_row: *npc_row,
                npc_col: *npc_col,
                row: *row,
                col: *col,
                owner: *owner,
            }),
            Catch { owner, npc_id } => sink.apply_catch(*owner, *npc_id),
            Fled { player } => sink.apply_fled(*player),
            Respawn {
                npc_id,
                map_id,
                x,
                y,
            } => sink.apply_respawn(*npc_id, *map_id, *x, *y),
            PetExp { owner, stt, exp } => sink.apply_pet_exp(*owner, *stt, *exp),
        }
    }
}
