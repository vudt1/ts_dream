//! Runtime map-NPC instance registry (C# `Data.NpcOnMap` + `NpcOnMapWalk`,
//! Data.cs:4880-5134). Serves two consumers:
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

/// One live instance (`DataStructure._NpcOnMap`, DataStructure.cs:528-551).
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

/// A snapshot of one online player as the walk loop reads it (`NpcOnMapWalk`'s
/// per-client fields: map id, coords, battle state, leadership, logined flag).
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
    /// Snapshot of every online player (`Server.ListView_Client` equivalent).
    fn players(&self) -> Vec<WorldPlayer>;
    /// `client.Sendpacket(text)` — one client receives the frame.
    fn send_player(&self, player: i64, frame: String);
    /// `Server.SendToAllMapid(map_id, text)` — every client on the map.
    fn send_map(&self, map_id: i64, frame: String);
    /// Engage `player` in the SoLuong TeamDef battle: set their
    /// `_My_TalkingBattle`, spawn TheBattle(player, teamdef, 4712), and mark
    /// the world entry's `_IdBattle = 1` (the caller already did the last).
    fn start_teamdef(&self, player: i64, npc_on_map_id: i64, teamdef: Vec<i64>);
}

/// The world NPC registry (`Data.NpcOnMap` + `_ListKeysNpcOnMap` + `random_3`).
#[derive(Debug)]
pub struct NpcWorld {
    /// All spawn entries, keyed `(map_id, id)`.
    pub entries: HashMap<NpcKey, NpcEntry>,
    /// `_ListKeysNpcOnMap` — registration order; only `SoLuong > 0` entries
    /// join the walk/patrol list (Data.cs:4913-4915).
    pub keys: Vec<NpcKey>,
    /// The 4th world RNG stream (Data.cs:46,87) — wander coordinates. This
    /// stream is separate from the three battle streams (§6.0).
    pub random_3: DotNetRandom,
}

impl NpcWorld {
    /// Build the world from the static `NpcOnMap.txt` rows. Delay and battle
    /// flags start cleared (C# `LoadDataNpcsOnMap`, Data.cs:4917-4931).
    pub fn new(rows: &[NpcOnMap]) -> Self {
        let mut entries = HashMap::new();
        let mut keys = Vec::new();
        for n in rows {
            let key = (n.map_id, n.id);
            if entries.contains_key(&key) {
                continue; // duplicate keys are skipped (C# guarded Add).
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

    /// Run one 900 ms walk pass over the patrol list (C# `NpcOnMapWalk`,
    /// Data.cs:4939-5134). `chase` is true on every 3rd tick (`num >= 3`):
    /// only then does the loop chase check + wander. On the other two ticks of
    /// the cycle free NPCs do nothing and only the respawn countdown advances.
    ///
    /// Consumes `random_3` draws in exactly the C# order (one wander pair per
    /// same-map player, chase first if it fired) so a captured run is
    /// reproducible given the same player snapshot.
    pub fn walk_tick(&mut self, chase: bool, sink: &dyn WorldSink) {
        let players = sink.players();
        for key in std::mem::take(&mut self.keys) {
            let mut value = match self.entries.get(&key) {
                Some(v) => v.clone(),
                None => continue,
            };
            let (map_id, id) = key;
            if value.delay == 0 && value.id_battle == 0 {
                if chase {
                    let mut lo_x = value.x_first - value.coord;
                    if lo_x < 0 {
                        lo_x = value.x_first;
                    }
                    let hi_x = value.x_first + value.coord;
                    let mut lo_y = value.y_first - value.coord;
                    if lo_y < 0 {
                        lo_y = value.y_first;
                    }
                    let hi_y = value.y_first + value.coord;
                    let mut text = String::new();
                    for p in &players {
                        if p.map_id == map_id
                            && p.battle_id == 0
                            && (p.leader_id == 0 || p.leader_id == p.id)
                            && p.logined
                        {
                            let dx = p.x - value.x_first;
                            let dy = p.y - value.y_first;
                            let dist = ((dx * dx + dy * dy) as f64).sqrt().round() as i64;
                            if dist <= value.coord {
                                // Chase: snap the npc onto the player and
                                // trigger the SoLuong battle.
                                value.x = p.x;
                                value.y = p.y;
                                text = npc_walk_frame(id, p.x, p.y);
                                sink.send_map(map_id, text.clone());
                                let teamdef = teamdef_for_so_luong(value.so_luong, value.npc_id);
                                value.id_battle = 1;
                                self.entries.insert(key, value.clone());
                                sink.start_teamdef(p.id, id, teamdef);
                            }
                        }
                        if p.map_id == map_id {
                            // Wander: re-roll the patrol position for this npc.
                            let nx = i64::from(self.random_3.next_range(lo_x as i32, hi_x as i32));
                            let ny = i64::from(self.random_3.next_range(lo_y as i32, hi_y as i32));
                            value.x = nx;
                            value.y = ny;
                            if text.is_empty() {
                                text = npc_walk_frame(id, nx, ny);
                            }
                            sink.send_player(p.id, text.clone());
                        }
                    }
                    self.entries.insert(key, value);
                }
            } else if value.delay >= 1 {
                value.id_battle = 0;
                value.delay -= 1;
                self.entries.insert(key, value);
            }
            self.keys.push(key);
        }
    }
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
/// attack (C# `NpcOnMapWalk`, Data.cs:5016-5083): SoLuong 1 → `_id3`,
/// 2 → `_id3,_id4`, 3 → `_id2.._id4`, 4 → `_id2,_id3,_id4,_id8`,
/// 5 → `_id1.._id5`. DiaHinh is always 4712 (the ticket contract).
pub fn teamdef_for_so_luong(so_luong: i64, npc_id: i64) -> Vec<i64> {
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
        _ => {
            for slot in teamdef.iter_mut().take(6).skip(1) {
                *slot = npc_id;
            }
        }
    }
    teamdef
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::rng::DotNetRandom;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        sent_players: Mutex<Vec<(i64, String)>>,
        sent_maps: Mutex<Vec<(i64, String)>>,
        teamdefs: Mutex<Vec<(i64, i64, Vec<i64>)>>,
        players: Vec<WorldPlayer>,
    }

    impl WorldSink for RecordingSink {
        fn players(&self) -> Vec<WorldPlayer> {
            self.players.clone()
        }
        fn send_player(&self, player: i64, frame: String) {
            self.sent_players.lock().unwrap().push((player, frame));
        }
        fn send_map(&self, map_id: i64, frame: String) {
            self.sent_maps.lock().unwrap().push((map_id, frame));
        }
        fn start_teamdef(&self, player: i64, npc_on_map_id: i64, teamdef: Vec<i64>) {
            self.teamdefs
                .lock()
                .unwrap()
                .push((player, npc_on_map_id, teamdef));
        }
    }

    fn world() -> NpcWorld {
        let rows = vec![NpcOnMap {
            map_id: 12001,
            id: 7,
            npc_id: 9001,
            x: 400,
            y: 500,
            coord: 10,
            so_luong: 3,
        }];
        let mut w = NpcWorld::new(&rows);
        w.random_3 = DotNetRandom::new(42);
        w
    }

    #[test]
    fn chase_triggers_teamdef_when_player_in_range() {
        let mut w = world();
        let sink = RecordingSink {
            players: vec![WorldPlayer {
                id: 300001,
                map_id: 12001,
                x: 405,
                y: 502,
                battle_id: 0,
                leader_id: 0,
                logined: true,
            }],
            ..Default::default()
        };
        // 3rd tick → chase enabled.
        w.walk_tick(true, &sink);
        // The npc chased the player (map-wide frame) and engaged them.
        let maps = sink.sent_maps.lock().unwrap();
        assert_eq!(maps.len(), 1);
        assert!(maps[0].1.starts_with("F44408001602"));
        let teamdefs = sink.teamdefs.lock().unwrap();
        assert_eq!(teamdefs.len(), 1);
        assert_eq!(teamdefs[0].0, 300001); // engaged player
        assert_eq!(teamdefs[0].1, 7); // npc on-map id = talking battle
        // SoLuong 3 → _id2.._id4 = 9001.
        assert_eq!(teamdefs[0].2, [4712, 0, 9001, 9001, 9001, 0, 0, 0, 0, 0, 0]);
        // The instance is now in battle (id_battle = 1).
        assert_eq!(w.get(12001, 7).unwrap().id_battle, 1);
    }

    #[test]
    fn no_chase_when_player_out_of_range() {
        let mut w = world();
        let sink = RecordingSink {
            players: vec![WorldPlayer {
                id: 300001,
                map_id: 12001,
                x: 1000,
                y: 1000,
                battle_id: 0,
                leader_id: 0,
                logined: true,
            }],
            ..Default::default()
        };
        w.walk_tick(true, &sink);
        assert!(sink.teamdefs.lock().unwrap().is_empty());
        assert_eq!(w.get(12001, 7).unwrap().id_battle, 0);
    }

    #[test]
    fn countdown_decrements_delay_and_clears_id_battle() {
        let mut w = world();
        {
            let e = w.get_mut(12001, 7).unwrap();
            e.delay = 3;
            e.id_battle = 1;
        }
        let sink = RecordingSink::default();
        w.walk_tick(false, &sink);
        let e = w.get(12001, 7).unwrap();
        assert_eq!(e.delay, 2);
        assert_eq!(e.id_battle, 0);
    }

    #[test]
    fn wander_sends_per_player_frame_on_chase_ticks() {
        let mut w = world();
        // Two players on the map, both out of range → wander frames only.
        let sink = RecordingSink {
            players: vec![
                WorldPlayer {
                    id: 300001,
                    map_id: 12001,
                    x: 900,
                    y: 900,
                    battle_id: 0,
                    leader_id: 0,
                    logined: true,
                },
                WorldPlayer {
                    id: 300002,
                    map_id: 12001,
                    x: 901,
                    y: 901,
                    battle_id: 0,
                    leader_id: 300001,
                    logined: true,
                },
            ],
            ..Default::default()
        };
        w.walk_tick(true, &sink);
        let sent = sink.sent_players.lock().unwrap();
        assert_eq!(sent.len(), 2);
        for (_, f) in sent.iter() {
            assert!(f.starts_with("F44408001602"));
        }
        // No battle triggered.
        assert!(sink.teamdefs.lock().unwrap().is_empty());
    }
}