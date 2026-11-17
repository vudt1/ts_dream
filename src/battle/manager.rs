//! Battle manager (Chapter 1 §1.4, Chapter 6) — registry + async per-battle tasks.
//!
//! One battle runs on its own `tokio::spawn` task. Player commands (op 0x0B)
//! arrive through a per-battle `mpsc` channel; the task collects them each turn
//! (with a per-turn timeout, default 20 s), runs `Battle::run_turn`, and dispatches
//! every `runner::Out` through a `BattleSink`. The grid + RNG live only inside
//! the task, so battle state is race-free.

use crate::battle::construction::Battle;
use crate::battle::npc_world::NpcWorld;
use crate::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, DbUpdate, Out, Outcome, PlayerSnapshot,
};
use crate::data::tables::{BinarySkillDef, Item, Npc, Skill, TexpRow};
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
    /// Apply a PvE gold reward. The default is a no-op for test sinks.
    fn apply_gold(&self, _owner: i64, _amount: i64) {}
    /// The battle task finished (`PlayerWin`/`PlayerLose`/`PlayerFled`).
    fn battle_ended(&self, id: i32, outcome: Outcome);
}

/// A live battle handle (used to send commands and join/leave).
#[derive(Debug, Clone)]
pub struct BattleHandle {
    pub id: i32,
    tx: mpsc::UnboundedSender<PlayerInput>,
    ack_notify: Arc<tokio::sync::Notify>,
    locked_cells: Arc<StdRwLock<HashMap<(u8, u8), i64>>>,
}

impl BattleHandle {
    /// Submit a command for the current turn.
    pub fn command(&self, input: PlayerInput) -> bool {
        self.tx.send(input).is_ok()
    }

    /// Check synchronously (O(1)) whether `(row, col)` is currently under a
    /// Turn-Locking Status and owned by `player`.
    pub fn is_cell_locked_for(&self, row: u8, col: u8, player: i64) -> bool {
        self.locked_cells
            .read()
            .map(|g| g.get(&(row, col)).copied() == Some(player))
            .unwrap_or(false)
    }

    pub(crate) fn set_locked_cells(&self, cells: HashMap<(u8, u8), i64>) {
        if let Ok(mut g) = self.locked_cells.write() {
            *g = cells;
        }
    }

    /// Notify that the client has acknowledged the battle scene (0B FA).
    pub fn ack_scene(&self) {
        self.ack_notify.notify_one();
    }

    /// Construct a dummy battle handle (used by quiz battles where turns do not run).
    pub fn dummy(id: i32) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self {
            id,
            tx,
            ack_notify: Arc::new(tokio::sync::Notify::new()),
            locked_cells: Arc::new(StdRwLock::new(HashMap::new())),
        }
    }
}

/// The registry of live battles (id assignment precedes increment).
pub struct BattleManager {
    battles: RwLock<HashMap<i32, BattleHandle>>,
    next_id: AtomicI32,
    default_timeout: std::time::Duration,
    handshake_timeout: std::time::Duration,
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
            // Per-turn input timeout (20 s business-rule countdown; custom timeouts
            // remain available for deterministic tests and special battle modes).
            default_timeout: std::time::Duration::from_secs(20),
            handshake_timeout: std::time::Duration::from_millis(1500),
        }
    }

    pub fn set_handshake_timeout(&mut self, timeout: std::time::Duration) {
        self.handshake_timeout = timeout;
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
        binary_skills: Arc<HashMap<u16, BinarySkillDef>>,
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
            binary_skills,
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
        binary_skills: Arc<HashMap<u16, BinarySkillDef>>,
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
        self.spawn_timeout_with_skills(
            battle,
            npcs,
            skills,
            binary_skills,
            items,
            pet_slots,
            players,
            None,
            texps,
            per_exp,
            world,
            talking_battle,
            map_id,
            timeout,
            sink,
        )
    }

    /// Spawn with custom timeout and optional per-player learned skills.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_timeout_with_skills(
        self: &Arc<Self>,
        battle: Battle,
        npcs: Arc<HashMap<i64, Npc>>,
        skills: Arc<HashMap<i64, Skill>>,
        binary_skills: Arc<HashMap<u16, BinarySkillDef>>,
        items: Arc<HashMap<i64, Item>>,
        pet_slots: Arc<HashMap<i64, [i64; 4]>>,
        players: Arc<HashMap<i64, PlayerSnapshot>>,
        player_skills: Option<Arc<HashMap<i64, Vec<u16>>>>,
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
        let ack_notify = Arc::new(tokio::sync::Notify::new());
        let locked_cells = Arc::new(StdRwLock::new(HashMap::new()));
        let handle = BattleHandle {
            id,
            tx,
            ack_notify: ack_notify.clone(),
            locked_cells: locked_cells.clone(),
        };
        let handle_task = handle.clone();

        let handshake_dur = if timeout <= std::time::Duration::from_millis(500) {
            timeout.min(std::time::Duration::from_millis(50))
        } else {
            self.handshake_timeout
        };

        let manager = self.clone();
        tokio::spawn(async move {
            let mut battle = battle;
            let mut rx = rx;
            // Build the per-task data from the tables it now owns.
            let mut data = BattleData::new(
                &npcs,
                &skills,
                &items,
                &pet_slots,
                &players,
                &texps,
                world.as_ref().map(|w| w.as_ref()),
                talking_battle,
                map_id,
            )
            .with_binary_skills(&binary_skills);
            if let Some(ref ps) = player_skills {
                data = data.with_player_skills(ps.as_ref());
            }
            let mut out: Vec<Out> = Vec::new();

            // Turn 1 Handshake Barrier: wait for 0B FA ack from client with 1.5s fallback (Ticket 20)
            let _ = tokio::time::timeout(handshake_dur, ack_notify.notified()).await;

            loop {
                out.clear();

                // Compute Turn-Locking Status snapshot and living player IDs in one pass
                // before Step 1 (34 01) so synchronous 0x32 rejection and collect_commands agree 1-to-1.
                let mut locked_snapshot: HashMap<(u8, u8), i64> = HashMap::new();
                let mut player_ids: std::collections::HashSet<i64> =
                    std::collections::HashSet::new();
                for r in 0..4u8 {
                    for c in 0..5u8 {
                        if let Some(cell) = battle.cell(r, c) {
                            if let Some(owner) = owner_player_id(cell) {
                                player_ids.insert(owner);
                                if will_be_turn_locked(cell) {
                                    locked_snapshot.insert((r, c), owner);
                                }
                            }
                        }
                    }
                }
                handle_task.set_locked_cells(locked_snapshot.clone());

                // Step 1: Turn Prompt (34 01) - send to all living player-controlled fighters (Ticket 20)
                for pid in player_ids {
                    sink.send_to(pid, crate::battle::packets::your_turn());
                }

                // Step 2: Collect commands with a timeout.
                let commands = collect_commands(&mut rx, timeout, &battle, &locked_snapshot).await;
                handle_task.set_locked_cells(HashMap::new());
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

                // Step 4: Dynamic Skill Delay Pacing (Ticket 18, ADR 0006)
                // Sleep clamp(sum(delay) * 100ms, 800ms, 3500ms) after 32 02
                // so the client can finish playing animations before receiving 34 01 for the next turn.
                let delay_ms = battle.last_turn_delay_ms();
                let sleep_dur = if timeout <= std::time::Duration::from_millis(500) {
                    std::time::Duration::from_millis(delay_ms).min(std::time::Duration::from_millis(20))
                } else {
                    std::time::Duration::from_millis(delay_ms)
                };
                tokio::time::sleep(sleep_dur).await;
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
    locked_cells: &HashMap<(u8, u8), i64>,
) -> BattleCommands {
    let mut commands = BattleCommands::new();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        // Every living, unlocked player-controlled cell needs a command,
        // including follow-NPC/pet cells (typ=4).
        let waiting_cells: Vec<(u8, u8)> = (0..4u8)
            .flat_map(|r| (0..5u8).map(move |c| (r, c)))
            .filter(|&(r, c)| {
                !locked_cells.contains_key(&(r, c))
                    && battle.cell(r, c).is_some_and(is_living_player_controlled)
            })
            .collect();
        let all_ready = waiting_cells.iter().all(|cell| commands.contains_key(cell));
        if all_ready {
            return commands;
        }
        if tokio::time::Instant::now() >= deadline {
            fill_timeout_commands(&mut commands, battle, &waiting_cells);
            return commands;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(input)) => {
                // The owner id remains the authorization identity, while the
                // source cell is the command identity. Invalid ownership,
                // malformed coordinates, or turn-locked cells never satisfy readiness.
                if !locked_cells.contains_key(&(input.cmd.row, input.cmd.col))
                    && owns_commandable_cell(battle, input.player, &input.cmd)
                {
                    commands.insert((input.cmd.row, input.cmd.col), input.cmd);
                }
            }
            _ => {
                fill_timeout_commands(&mut commands, battle, &waiting_cells);
                return commands;
            }
        }
    }
}

fn fill_timeout_commands(
    commands: &mut BattleCommands,
    battle: &Battle,
    waiting_cells: &[(u8, u8)],
) {
    // Auto-confirm a basic attack for every living, non-stunned
    // player-controlled fighter that timed out. The command still uses the
    // unchanged grid source/target fields (op 0x0B sub 3/4).
    for &(row, col) in waiting_cells {
        if commands.contains_key(&(row, col)) {
            continue;
        }
        let Some(source) = battle.cell(row, col) else {
            continue;
        };
        let Some(target) = first_living_enemy(battle, source.team) else {
            continue;
        };
        commands.insert(
            (row, col),
            BattleCommand {
                row,
                col,
                target_row: target.0,
                target_col: target.1,
                action: BattleAction::NormalAttack,
            },
        );
    }
}

/// Predict whether a fighter will be under a Turn-Locking Status (Hard CC Type 3
/// or Confusion/Berserk `14021 | 20014`) in the upcoming turn, matching `turn_phase1`'s
/// round-start `-1` decrement and `turn <= 1` expiry rule 1-to-1.
pub fn will_be_turn_locked(cell: &crate::battle::engine::WarInfo) -> bool {
    let type3_active = cell.type3_id > 0 && cell.type3_turn.saturating_sub(1) > 1;
    let berserk_active =
        crate::battle::engine::is_berserk_status(cell.type15_id) && cell.type15_turn.saturating_sub(1) > 1;
    type3_active || berserk_active
}

fn owner_player_id(cell: &crate::battle::engine::WarInfo) -> Option<i64> {
    if cell.id > 0 && cell.hp > 0 {
        if cell.typ == 2 {
            Some(cell.id)
        } else if cell.typ == 4 && cell.id_char > 0 {
            Some(cell.id_char)
        } else {
            None
        }
    } else {
        None
    }
}

fn is_living_player_controlled(cell: &crate::battle::engine::WarInfo) -> bool {
    owner_player_id(cell).is_some()
}

fn owns_commandable_cell(battle: &Battle, player: i64, cmd: &BattleCommand) -> bool {
    battle.cell(cmd.row, cmd.col).is_some_and(|cell| {
        is_living_player_controlled(cell)
            && ((cell.typ == 2 && cell.id == player) || (cell.typ == 4 && cell.id_char == player))
    })
}

fn first_living_enemy(battle: &Battle, team: i64) -> Option<(u8, u8)> {
    (0..4u8)
        .flat_map(|r| (0..5u8).map(move |c| (r, c)))
        .find(|&(r, c)| {
            battle
                .cell(r, c)
                .is_some_and(|cell| cell.id > 0 && cell.hp > 0 && cell.team != team)
        })
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
            Gold { owner, amount } => sink.apply_gold(*owner, *amount),
        }
    }
}

