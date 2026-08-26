//! Runtime map-NPC instance registry. Serves two consumers:
//!
//! - the 900 ms wander/chase task (ticket 20 G1): moves free NPCs within their
//!   `[X_First ± Coord, Y_First ± Coord]` box every 3rd tick and triggers the
//!   DiaHinh-4712 SoLuong battle when a player is close enough; and
//! - the battle engine (ticket 20 G2): post-battle respawn reads `_Delay` and
//!   draws the respawn point through `random_2` before setting `_Delay = 10`.
//!
//! The world is shared with the per-battle task through an `Arc` (a
//! `std::sync::RwLock` so the synchronous battle engine can touch it; the walk
//! task's critical sections are equally short).

use crate::battle::rng::DotNetRandom;
use crate::data::tables::NpcOnMap;
use crate::protocol::encoder;
use std::collections::HashMap;

/// Key of one map-NPC instance: `(map_id, id)`.
pub type NpcKey = (i64, i64);

/// One live map-NPC instance.
#[derive(Debug, Clone, Default)]
pub struct NpcEntry {
    pub map_id: i64,
    pub id: i64,
    pub npc_id: i64,
    pub x_first: i64,
    pub y_first: i64,
    pub coord: i64,
    pub x: i64,
    pub y: i64,
    pub delay: i64,
    pub so_luong: i64,
    pub id_battle: i64,
}

/// A snapshot of one online player as the walk loop reads it (map id,
/// coords, battle state, leadership, logined flag).
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldPlayer {
    pub id: i64,
    pub map_id: i64,
    pub x: i64,
    pub y: i64,
    pub battle_id: i64,
    pub leader_id: i64,
    pub logined: bool,
}

/// What the walk loop does with a player it decides to engage or notify.
pub trait WorldSink {
    /// Snapshot of every online player.
    fn players(&self) -> Vec<WorldPlayer>;
    /// `client.Sendpacket(text)` — one client receives the frame.
    fn send_player(&self, player: i64, frame: String);
    /// Broadcast to every client on the map.
    fn send_map(&self, map_id: i64, frame: String);
    /// Engage `player` in the SoLuong TeamDef battle (DiaHinh 4712): set
    /// their talking-battle marker and mark the world entry `_IdBattle = 1`.
    fn start_teamdef(&self, player: i64, npc_on_map_id: i64, teamdef: Vec<i64>);
}

/// The world NPC registry (`entries` + registration-order `keys` + `random_3`).
#[derive(Debug)]
pub struct NpcWorld {
    /// All spawn entries, keyed `(map_id, id)`.
    pub entries: HashMap<NpcKey, NpcEntry>,
    /// Registration order; only `SoLuong > 0` entries join the walk/patrol list.
    pub keys: Vec<NpcKey>,
    /// The 4th world RNG stream — wander coordinates. This
    /// stream is separate from the three battle streams (§6.0).
    pub random_3: DotNetRandom,
}

impl NpcWorld {
    /// Build the world from the static `NpcOnMap.txt` rows. Delay and battle
    /// flags start cleared.
    pub fn new(rows: &[NpcOnMap]) -> Self {
        let mut entries = HashMap::new();
        let mut keys = Vec::new();
        for n in rows {
            let key = (n.map_id, n.id);
            if entries.contains_key(&key) {
                continue; // duplicate keys are skipped.
            }
            if n.so_luong > 0 {
                keys.push(key);
            }
            entries.insert(
                key,
                NpcEntry {
                    map_id: n.map_id,
                    id: n.id,
                    npc_id: n.npc_id,
                    x_first: n.x,
                    y_first: n.y,
                    coord: n.coord,
                    x: n.x,
                    y: n.y,
                    delay: 0,
                    so_luong: n.so_luong,
                    id_battle: 0,
                },
            );
        }
        NpcWorld {
            entries,
            keys,
            random_3: DotNetRandom::time_seeded(),
        }
    }

    pub fn get(&self, map_id: i64, id: i64) -> Option<&NpcEntry> {
        self.entries.get(&(map_id, id))
    }

    pub fn get_mut(&mut self, map_id: i64, id: i64) -> Option<&mut NpcEntry> {
        self.entries.get_mut(&(map_id, id))
    }

    /// Run one 900 ms walk pass over the patrol list. `chase` is true on
    /// every 3rd tick (`num >= 3`):
    /// only then does the loop chase check + wander. On the other two ticks of
    /// the cycle free NPCs do nothing and only the respawn countdown advances.
    ///
    /// Consumes `random_3` draws in the documented order (one wander pair per
    /// same-map player, chase first if it fired) so a captured run is
    /// reproducible given the same player snapshot.
    pub fn walk_tick(&mut self, chase: bool, sink: &dyn WorldSink) {
        let players = sink.players();
        for key in std::mem::take(&mut self.keys) {
            let mut entry = match self.entries.get(&key) {
                Some(v) => v.clone(),
                None => continue,
            };
            let (map_id, id) = key;
            if entry.delay == 0 && entry.id_battle == 0 {
                if chase {
                    let (lo_x, hi_x, lo_y, hi_y) = patrol_bounds(&entry);
                    let mut text = String::new();
                    for p in &players {
                        if p.map_id == map_id
                            && p.battle_id == 0
                            && (p.leader_id == 0 || p.leader_id == p.id)
                            && p.logined
                        {
                            let dx = p.x - entry.x_first;
                            let dy = p.y - entry.y_first;
                            let dist = ((dx * dx + dy * dy) as f64).sqrt().round() as i64;
                            if dist <= entry.coord {
                                // Chase: snap the npc onto the player.
                                entry.x = p.x;
                                entry.y = p.y;
                                text = npc_walk_frame(id, p.x, p.y);
                                sink.send_map(map_id, text.clone());
                                // Engage only when SoLuong is a valid 1..5 slot
                                // mapping (none outside SoLuong 1..=5); the
                                // chase frame still fires otherwise.
                                if let Some(teamdef) =
                                    teamdef_for_so_luong(entry.so_luong, entry.npc_id)
                                {
                                    entry.id_battle = 1;
                                    self.entries.insert(key, entry.clone());
                                    sink.start_teamdef(p.id, id, teamdef);
                                }
                            }
                        }
                        if p.map_id == map_id {
                            // Wander: re-roll the patrol position for this npc.
                            let nx = i64::from(self.random_3.next_range(lo_x as i32, hi_x as i32));
                            let ny = i64::from(self.random_3.next_range(lo_y as i32, hi_y as i32));
                            entry.x = nx;
                            entry.y = ny;
                            if text.is_empty() {
                                text = npc_walk_frame(id, nx, ny);
                            }
                            sink.send_player(p.id, text.clone());
                        }
                    }
                    self.entries.insert(key, entry);
                }
            } else if entry.delay >= 1 {
                entry.id_battle = 0;
                entry.delay -= 1;
                self.entries.insert(key, entry);
            }
            self.keys.push(key);
        }
    }
}

/// The patrol-box clamp (`num2 = x_first - coord; if < 0 → x_first`, plus the
/// y mirror) shared by the walk loop and the battle respawn (G1/G2).
pub fn patrol_bounds(entry: &NpcEntry) -> (i64, i64, i64, i64) {
    let mut lo_x = entry.x_first - entry.coord;
    if lo_x < 0 {
        lo_x = entry.x_first;
    }
    let hi_x = entry.x_first + entry.coord;
    let mut lo_y = entry.y_first - entry.coord;
    if lo_y < 0 {
        lo_y = entry.y_first;
    }
    let hi_y = entry.y_first + entry.coord;
    (lo_x, hi_x, lo_y, hi_y)
}

/// `F44408001602` + le16(id) + le16(x) + le16(y) — the npc walk/chase frame.
fn npc_walk_frame(id: i64, x: i64, y: i64) -> String {
    format!(
        "F44408001602{}{}{}",
        encoder::le16(id as u16),
        encoder::le16(x as u16),
        encoder::le16(y as u16)
    )
}

/// Build the TeamDefender slot vector `{diahinh, id1..id10}` for a SoLuong
/// attack: SoLuong 1 → `_id3`,
/// 2 → `_id3,_id4`, 3 → `_id2.._id4`, 4 → `_id2,_id3,_id4,_id8`,
/// 5 → `_id1.._id5`. DiaHinh is always 4712 (the ticket contract).
///
/// Returns `None` for `so_luong` outside 1..=5 — such NPCs chase
/// (frame + `_IdBattle=1`) but never enter battle.
pub fn teamdef_for_so_luong(so_luong: i64, npc_id: i64) -> Option<Vec<i64>> {
    let diahinh = 4712;
    // teamdef[0] = diahinh, teamdef[i] = `_id{i}` for i in 1..=10.
    let mut teamdef = vec![diahinh, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    match so_luong {
        1 => teamdef[3] = npc_id,
        2 => {
            teamdef[3] = npc_id;
            teamdef[4] = npc_id;
        }
        3 => {
            teamdef[2] = npc_id;
            teamdef[3] = npc_id;
            teamdef[4] = npc_id;
        }
        4 => {
            teamdef[2] = npc_id;
            teamdef[3] = npc_id;
            teamdef[4] = npc_id;
            teamdef[8] = npc_id;
        }
        5 => {
            for slot in teamdef.iter_mut().take(6).skip(1) {
                *slot = npc_id;
            }
        }
        _ => return None,
    }
    Some(teamdef)
}
