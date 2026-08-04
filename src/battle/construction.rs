//! Battle construction (Chapter 6 §6.1).
//!
//! Creates a new battle grid, populates cells with players/NPCs/pets, and
//! manages the IdBattle counter. Faithful port of `TheBattle.cs` `CreatNewBattle`
//! (line 32), `ChangedWar` (73), `AddToBattle` (116) and `AddNPCToBattle` (424).

use crate::battle::engine::{war_key, WarInfo};
use crate::battle::packets;
use crate::battle::rng::BattleRng;
use crate::data::tables::Npc;
use crate::server::session::{PetState, Session};
use std::collections::HashMap;

/// One battle-start packet, tagged with its recipient routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartPacket {
    /// Send directly to `player` (`Server.SendToClient`).
    To { player: i64, frame: String },
    /// Send to every client on `player`'s map, excluding `player` (`SendToAllClientMapid`).
    Map { player: i64, frame: String },
}

/// A live battle instance.
#[derive(Debug)]
pub struct Battle {
    /// Unique battle id (assigned from `IdBattleCount` before increment).
    pub id_battle: i32,
    /// Terrain id.
    pub diahinh: i32,
    /// The 20-cell grid keyed by `row-col` hex string.
    pub list_war: HashMap<String, WarInfo>,
    /// Cell keys in creation order (row 0..3, col 0..4).
    pub keys: Vec<String>,
    /// Three independent RNG streams.
    pub rng: BattleRng,
    /// Spectator/join slots (1..50).
    pub list_qs: HashMap<i32, i32>,
}

impl Battle {
    /// Create a new battle with grid cells.
    pub fn new(id_battle: i32, diahinh: i32) -> Self {
        let mut battle = Battle {
            id_battle,
            diahinh,
            list_war: HashMap::with_capacity(20),
            keys: Vec::with_capacity(20),
            rng: BattleRng::new(),
            list_qs: HashMap::with_capacity(50),
        };

        // Create 20 cells (row 0..3, col 0..4)
        for row in 0..4u8 {
            for col in 0..5u8 {
                let key = war_key(row, col);
                let cell = WarInfo {
                    row,
                    col,
                    ..Default::default()
                };
                battle.list_war.insert(key.clone(), cell);
                battle.keys.push(key);
            }
        }

        // Init ListQS slots 1..50
        for i in 1..=50 {
            battle.list_qs.insert(i, 0);
        }

        battle
    }

    /// Create with explicit RNG seeds (for testing).
    pub fn with_seeds(id_battle: i32, diahinh: i32, s0: i32, s1: i32, s2: i32) -> Self {
        let mut b = Self::new(id_battle, diahinh);
        b.rng = BattleRng::with_seeds(s0, s1, s2);
        b
    }

    /// Add a player to the battle grid.
    ///
    /// `row` is 3 for team 1, 0 for team 2. `col` is the column (2 = leader).
    /// `team = (row == 0) ? 2 : 1`.
    pub fn add_player(&mut self, session: &Session, leader_id: i64, row: u8, col: u8) {
        let key = war_key(row, col);
        let team = if row == 0 { 2 } else { 1 };

        if let Some(cell) = self.list_war.get_mut(&key) {
            cell.typ = 2; // player type
            cell.id = session.id as i64;
            cell.id_char = 0;
            cell.id_npc_on_map = 0;
            cell.hp_max = session.hp_max as i64;
            cell.sp_max = session.sp_max as i64;
            cell.hp = session.hp as i64;
            cell.sp = session.sp as i64;
            cell.lv = session.level as i64;
            cell.thuoctinh = session.thuoctinh as i64;
            cell.leader_id = leader_id;
            cell.team = team;
            cell.int1 = session.int1 as i64 + session.int2 as i64;
            cell.atk = session.atk as i64 + session.atk2 as i64;
            cell.def = session.def as i64 + session.def2 as i64;
            cell.agi = session.agi as i64 + session.agi2 as i64;
            cell.reborn = session.reborn as i64;
            cell.row = row;
            cell.col = col;
        }
    }

    /// Load the leader's up-to-4 battle pets.
    ///
    /// C# `AddToBattle` (TheBattle.cs:135-238): leader pets at `(row^1, col)` for
    /// cols `1,3,0,4` using pet Stt `active .. active+3`. Later member pets
    /// overwrite overlapping cells (dict insertion order wins).
    pub fn load_leader_pets(&mut self, session: &Session, leader_id: i64, row: u8) {
        let team = if row == 0 { 2 } else { 1 };
        let base = session.active_pet_stt as i64;
        let cols = [1u8, 3, 0, 4];
        for (i, &col) in cols.iter().enumerate() {
            let stt = base + i as i64;
            let pet = session.pets.iter().find(|p| i64::from(p.stt) == stt);
            if let Some(pet) = pet {
                self.add_pet(pet, session.id as i64, leader_id, row, col, team);
            }
        }
    }

    /// Load one battle pet for a party member at `(row^1, col)` when the member's
    /// active pet Stt is in 1..=4 (C# AddToBattle member block, line 251-283).
    pub fn load_member_pet(&mut self, session: &Session, leader_id: i64, row: u8, col: u8) {
        let team = if row == 0 { 2 } else { 1 };
        let stt = session.active_pet_stt as i64;
        if (1..=4).contains(&stt) {
            if let Some(pet) = session.pets.iter().find(|p| i64::from(p.stt) == stt) {
                self.add_pet(pet, session.id as i64, leader_id, row, col, team);
            }
        }
    }

    /// Add a pet to the battle grid at the pet row (row ^ 1, col).
    pub fn add_pet(
        &mut self,
        pet: &PetState,
        owner_id: i64,
        leader_id: i64,
        row: u8,
        col: u8,
        team: i64,
    ) {
        let pet_row = row ^ 1;
        let key = war_key(pet_row, col);

        if let Some(cell) = self.list_war.get_mut(&key) {
            cell.typ = 4; // pet type
            cell.id = pet.id as i64;
            cell.id_npc_on_map = pet.stt as i64;
            cell.id_char = owner_id;
            cell.hp_max = pet.hp_max as i64;
            cell.sp_max = pet.sp_max as i64;
            cell.hp = pet.hp as i64;
            cell.sp = pet.sp as i64;
            cell.lv = pet.level as i64;
            cell.thuoctinh = pet.thuoctinh as i64;
            cell.leader_id = leader_id;
            cell.team = team;
            cell.int1 = pet.int1 as i64;
            cell.atk = pet.atk as i64;
            cell.def = pet.def as i64;
            cell.agi = pet.agi as i64;
            cell.reborn = pet.reborn as i64;
            cell.row = pet_row;
            cell.col = col;
        }
    }

    /// Add an NPC to the battle grid.
    ///
    /// `npc_type`: 3 = hostile NPC, 7 = TeamDef NPC.
    pub fn add_npc(&mut self, npc: &Npc, id_npc_on_map: i64, row: u8, col: u8, npc_type: u8) {
        let key = war_key(row, col);

        if let Some(cell) = self.list_war.get_mut(&key) {
            cell.typ = npc_type;
            cell.id = npc.id;
            cell.id_npc_on_map = id_npc_on_map;
            cell.id_char = 0;
            cell.hp_max = npc.hp;
            cell.sp_max = npc.sp;
            cell.hp = npc.hp;
            cell.sp = npc.sp;
            cell.lv = npc.lv;
            cell.thuoctinh = npc.thuoctinh;
            cell.leader_id = 0;
            cell.team = 2;
            cell.int1 = npc.int1;
            cell.atk = npc.atk;
            cell.def = npc.def;
            cell.agi = npc.agi;
            cell.reborn = npc.reborn;
            cell.row = row;
            cell.col = col;
        }
    }

    /// Remove a cell entirely (zeros its entity data, keeps the cell).
    pub fn clear_cell(&mut self, row: u8, col: u8) {
        let key = war_key(row, col);
        if let Some(cell) = self.list_war.get_mut(&key) {
            cell.typ = 0;
            cell.id = 0;
            cell.id_npc_on_map = 0;
            cell.id_char = 0;
            cell.hp_max = 0;
            cell.sp_max = 0;
            cell.hp = 0;
            cell.sp = 0;
            cell.lv = 0;
            cell.thuoctinh = 0;
            cell.leader_id = 0;
            cell.team = 0;
            cell.int1 = 0;
            cell.atk = 0;
            cell.def = 0;
            cell.agi = 0;
            cell.reborn = 0;
            cell.attacked = false;
            cell.random = 0;
            cell.exp = 0;
            cell.id_skill = 0;
            cell.lv_skill = 0;
        }
    }

    /// Check if all enemies (rows 0-1) are dead.
    pub fn all_enemies_dead(&self) -> bool {
        for row in 0..2u8 {
            for col in 0..5u8 {
                let key = war_key(row, col);
                if let Some(cell) = self.list_war.get(&key) {
                    if cell.id > 0 && cell.hp > 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Check if all players (rows 2-3) are dead.
    pub fn all_players_dead(&self) -> bool {
        for row in 2..4u8 {
            for col in 0..5u8 {
                let key = war_key(row, col);
                if let Some(cell) = self.list_war.get(&key) {
                    if cell.id > 0 && cell.hp > 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Count living entities (id>0 && hp>0) in the enemy rows (0-1).
    pub fn count_enemies_alive(&self) -> i64 {
        (0..2u8)
            .flat_map(|r| (0..5u8).map(move |c| (r, c)))
            .filter(|&(r, c)| {
                self.cell(r, c)
                    .map(|x| x.id > 0 && x.hp > 0)
                    .unwrap_or(false)
            })
            .count() as i64
    }

    /// Count living entities (id>0 && hp>0) in the player rows (2-3).
    pub fn count_players_alive(&self) -> i64 {
        (2..4u8)
            .flat_map(|r| (0..5u8).map(move |c| (r, c)))
            .filter(|&(r, c)| {
                self.cell(r, c)
                    .map(|x| x.id > 0 && x.hp > 0)
                    .unwrap_or(false)
            })
            .count() as i64
    }

    /// Get a cell reference by (row, col).
    pub fn cell(&self, row: u8, col: u8) -> Option<&WarInfo> {
        self.list_war.get(&war_key(row, col))
    }

    /// Get a mutable cell reference by (row, col).
    pub fn cell_mut(&mut self, row: u8, col: u8) -> Option<&mut WarInfo> {
        let key = war_key(row, col);
        self.list_war.get_mut(&key)
    }

    /// Build the turn order: sort by Attacked DESC, Agi DESC, Random DESC.
    /// Returns list of (row, col) in execution order.
    pub fn turn_order(&self) -> Vec<(u8, u8)> {
        let mut entries: Vec<(u8, u8, bool, i64, i64)> = Vec::with_capacity(20);
        for key in &self.keys {
            if let Some(cell) = self.list_war.get(key) {
                entries.push((cell.row, cell.col, cell.attacked, cell.agi, cell.random));
            }
        }
        // Sort: Attacked DESC (true > false), Agi DESC, Random DESC
        entries.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)).then(b.4.cmp(&a.4)));
        entries.iter().map(|e| (e.0, e.1)).collect()
    }

    /// Generate the battle-start packets for an NPC battle (leader view + each
    /// member's own open frame), mirroring `BattleNpc` (`TheBattle.cs:676-712`).
    ///
    /// Returns packets tagged with their recipient (`StartPacket::To`) or the
    /// "show on map" broadcast (`StartPacket::Map`).
    pub fn npc_battle_start_packets(&self, diahinh: i32) -> Vec<StartPacket> {
        let mut out: Vec<StartPacket> = Vec::new();

        // Leader at (3,2)
        let Some(leader) = self.cell(3, 2) else {
            return out;
        };
        if leader.id <= 0 {
            return out;
        }
        let leader_packet = leader.packet_hex();

        // Open frame (leader)
        out.push(StartPacket::To {
            player: leader.id,
            frame: packets::battle_open_leader(diahinh as u16, &leader_packet),
        });

        // Leader on map
        out.push(StartPacket::Map {
            player: leader.id,
            frame: packets::show_player_on_map(leader.id as u32),
        });

        // Leader's pet at (2,2)
        if let Some(pet) = self.cell(2, 2) {
            if pet.id > 0 {
                out.push(StartPacket::To {
                    player: leader.id,
                    frame: packets::entity_player(&pet.packet_hex()),
                });
            }
        }

        // Members at cols 1, 3, 0, 4
        for &col in &[1u8, 3, 0, 4] {
            if let Some(member) = self.cell(3, col) {
                if member.id > 0 {
                    let pkt = member.packet_hex();
                    out.push(StartPacket::To {
                        player: leader.id,
                        frame: packets::entity_player(&pkt),
                    });
                    out.push(StartPacket::To {
                        player: leader.id,
                        frame: packets::show_member_on_map(member.id as u32),
                    });

                    // Member's pet
                    if let Some(pet) = self.cell(2, col) {
                        if pet.id > 0 {
                            out.push(StartPacket::To {
                                player: leader.id,
                                frame: packets::entity_player(&pet.packet_hex()),
                            });
                        }
                    }

                    // Member's own battle frame (SendBattleMem1 equivalent)
                    out.extend(self.member_battle_frame(diahinh, member.id));
                } else if let Some(pet) = self.cell(2, col) {
                    // Member absent: still send their (leader-row) pet cell if set.
                    if pet.id > 0 {
                        out.push(StartPacket::To {
                            player: leader.id,
                            frame: packets::entity_player(&pet.packet_hex()),
                        });
                    }
                }
            }
        }

        // Enemy entities (rows 0-1)
        for row in 0..2u8 {
            for col in 0..5u8 {
                if let Some(enemy) = self.cell(row, col) {
                    if enemy.id > 0 {
                        out.push(StartPacket::To {
                            player: leader.id,
                            frame: packets::entity_npc(&enemy.packet_hex()),
                        });
                    }
                }
            }
        }

        out
    }

    /// The member's own `0BFA` battle frame (`SendBattleMem1`, TheBattle.cs:756-1000).
    ///
    /// Packet order mirrors the C#: show leader → show other members → the `0BFA`
    /// frame → map broadcast of self → own pet entity → enemy entities.
    /// Markers inside the frame: `05`+self, `03`+leader(+leader pet),
    /// `64`+each other member(+their pet).
    pub fn member_battle_frame(&self, diahinh: i32, member_id: i64) -> Vec<StartPacket> {
        let mut out: Vec<StartPacket> = Vec::new();
        let Some(cell) = self.list_war.values().find(|c| c.id == member_id) else {
            return out;
        };
        let row = cell.row;
        let col = cell.col;
        let opp = row ^ 1;

        let self_packet = cell.packet_hex();

        // Show leader on map (to member)
        if let Some(lc) = self.cell(row, 2) {
            if lc.id > 0 {
                out.push(StartPacket::To {
                    player: member_id,
                    frame: packets::show_player_on_map(lc.id as u32),
                });
            }
        }

        let mut text = String::new();
        text.push_str("05");
        text.push_str(&self_packet);

        // Leader block: (row,2) then (row^1,2)
        if let Some(lc) = self.cell(row, 2) {
            text.push_str("03");
            text.push_str(&lc.packet_hex());
        }
        if let Some(lp) = self.cell(opp, 2) {
            if lp.id > 0 {
                text.push_str("03");
                text.push_str(&lp.packet_hex());
            }
        }

        // Member blocks for cols 1,3,0,4 (excluding self) + their show packets.
        for &c in &[1u8, 3, 0, 4] {
            if c == col {
                continue;
            }
            if let Some(m) = self.cell(row, c) {
                if m.id > 0 {
                    out.push(StartPacket::To {
                        player: member_id,
                        frame: packets::show_member_on_map(m.id as u32),
                    });
                    text.push_str("64");
                    text.push_str(&m.packet_hex());
                    if let Some(p) = self.cell(opp, c) {
                        if p.id > 0 {
                            text.push_str("64");
                            text.push_str(&p.packet_hex());
                        }
                    }
                }
            }
        }

        let frame = format!(
            "F444{}0BFA{}{}F44403000B0A01",
            crate::protocol::encoder::le16(4 + text.len() as u16 / 2),
            crate::protocol::encoder::le16(diahinh as u16),
            text
        );
        out.push(StartPacket::To {
            player: member_id,
            frame,
        });

        // Show-on-map for the member itself (map broadcast)
        out.push(StartPacket::Map {
            player: member_id,
            frame: packets::show_player_on_map(member_id as u32),
        });

        // Own pet entity
        if let Some(p) = self.cell(opp, col) {
            if p.id > 0 {
                out.push(StartPacket::To {
                    player: member_id,
                    frame: packets::entity_player(&p.packet_hex()),
                });
            }
        }

        // Enemy entities
        for r in 0..2u8 {
            for c in 0..5u8 {
                if let Some(e) = self.cell(r, c) {
                    if e.id > 0 {
                        out.push(StartPacket::To {
                            player: member_id,
                            frame: packets::entity_npc(&e.packet_hex()),
                        });
                    }
                }
            }
        }

        out
    }
}

/// Global battle counter (starts at 1, assignment before increment).
#[derive(Debug, Default)]
pub struct BattleCounter {
    next_id: i32,
}

impl BattleCounter {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    /// Get the next battle ID (assigns before incrementing).
    pub fn next(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Diahinh constants for the four trigger types (Ch6 §6.1).
pub mod trigger_diahinh {
    /// PK challenge (`Client.cs:1300`) and NPC attack (`Client.cs:1323`).
    pub const PK_AND_NPC: i32 = 112;
    /// Active-NPC (so-luong) battle (`Data.cs:5026-5080`).
    pub const ACTIVE_NPC: i32 = 4712;
    /// Quest/TeamDef battle — Diahinh comes from `dataTalkTeamDefs[0]`.
    pub const TEAMDEF: i32 = 0;
}

impl Battle {
    /// Build a player-vs-NPC battle (`TheBattle(IdLeader, IdNpc, IdNpcOnMap, 112)`,
    /// TheBattle.cs:485). Enemy is a hostile NPC (Type 3) at (0,2).
    pub fn npc_battle(
        id_battle: i32,
        leader: &Session,
        leader_id: i64,
        npc: &Npc,
        id_npc_on_map: i64,
        diahinh: i32,
    ) -> Battle {
        let mut battle = Battle::new(id_battle, diahinh);
        battle.add_player(leader, leader_id, 3, 2);
        battle.load_leader_pets(leader, leader_id, 3);
        battle.add_npc(npc, id_npc_on_map, 0, 2, 3);
        battle
    }

    /// Build a TeamDef battle (`TheBattle(IdLeader, TeamDeffender, DiaHinh)`,
    /// TheBattle.cs:526). `defenders` are placed at rows 0-1 in the C# order
    /// with Type 7. Pass `members` to include party members (leader-only if empty).
    pub fn teamdef_battle(
        id_battle: i32,
        leader: &Session,
        leader_id: i64,
        members: &[&Session],
        defenders: &[&Npc],
        diahinh: i32,
    ) -> Battle {
        let mut battle = Battle::new(id_battle, diahinh);
        battle.add_player(leader, leader_id, 3, 2);
        battle.load_leader_pets(leader, leader_id, 3);
        let member_cols = [1u8, 3, 0, 4];
        for (i, member) in members.iter().enumerate() {
            if i >= 4 {
                break;
            }
            let col = member_cols[i];
            battle.add_player(member, leader_id, 3, col);
            battle.load_member_pet(member, leader_id, 3, col);
        }
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
        battle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::session::Session;

    #[test]
    fn create_battle_grid() {
        let battle = Battle::new(1, 112);
        assert_eq!(battle.list_war.len(), 20);
        assert_eq!(battle.keys.len(), 20);
        assert_eq!(battle.list_qs.len(), 50);
    }

    #[test]
    fn add_player_and_npc() {
        let mut battle = Battle::new(1, 112);
        let session = Session::new();

        battle.add_player(&session, session.id as i64, 3, 2);

        let cell = battle.cell(3, 2).unwrap();
        assert_eq!(cell.typ, 2);
        assert_eq!(cell.team, 1);
        assert_eq!(cell.row, 3);
        assert_eq!(cell.col, 2);

        let npc = Npc {
            id: 1001,
            hp: 500,
            sp: 200,
            lv: 10,
            thuoctinh: 1,
            atk: 50,
            def: 30,
            agi: 20,
            int1: 15,
            ..Default::default()
        };
        battle.add_npc(&npc, 1, 0, 2, 3);

        let enemy = battle.cell(0, 2).unwrap();
        assert_eq!(enemy.typ, 3);
        assert_eq!(enemy.team, 2);
        assert_eq!(enemy.hp, 500);
        assert_eq!(enemy.id, 1001);
    }

    #[test]
    fn leader_pets_loaded_at_expected_cells() {
        let mut battle = Battle::new(1, 112);
        let mut session = Session::new();
        session.id = 300001;
        session.active_pet_stt = 1;
        for stt in 1..=4u8 {
            let mut pet = PetState::default();
            pet.stt = stt;
            pet.id = 9000 + u16::from(stt);
            pet.hp_max = 100;
            pet.hp = 100;
            pet.level = 5;
            session.pets.push(pet);
        }
        battle.add_player(&session, session.id as i64, 3, 2);
        battle.load_leader_pets(&session, session.id as i64, 3);

        // Leader pets at (2,1), (2,3), (2,0), (2,4) with Stt 1,2,3,4.
        for (i, col) in [1u8, 3, 0, 4].iter().enumerate() {
            let cell = battle.cell(2, *col).unwrap();
            assert_eq!(cell.typ, 4, "pet cell (2,{col})");
            assert_eq!(cell.id, 9000 + i as i64 + 1);
            assert_eq!(cell.id_char, 300001);
            assert_eq!(cell.id_npc_on_map, i as i64 + 1);
        }
    }

    #[test]
    fn member_pet_overwrites_leader_pet() {
        let mut battle = Battle::new(1, 112);
        let mut leader = Session::new();
        leader.id = 300001;
        leader.active_pet_stt = 1;
        let mut pet = PetState::default();
        pet.stt = 1;
        pet.id = 9001;
        leader.pets.push(pet);

        let mut member = Session::new();
        member.id = 300002;
        member.active_pet_stt = 1;
        let mut mp = PetState::default();
        mp.stt = 1;
        mp.id = 8002;
        member.pets.push(mp);

        battle.add_player(&leader, leader.id as i64, 3, 2);
        battle.load_leader_pets(&leader, leader.id as i64, 3);
        battle.add_player(&member, leader.id as i64, 3, 1);
        battle.load_member_pet(&member, leader.id as i64, 3, 1);

        // Member processed later overwrites the leader's pet at (2,1).
        let cell = battle.cell(2, 1).unwrap();
        assert_eq!(cell.id, 8002);
    }

    #[test]
    fn teamdef_npcs() {
        let mut battle = Battle::new(1, 100);
        let npc = Npc {
            id: 2001,
            hp: 300,
            sp: 100,
            lv: 5,
            thuoctinh: 2,
            ..Default::default()
        };

        // TeamDef: id1→(0,0), id2→(0,1), ..., id6→(1,0), etc.
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
        for (i, &(r, c)) in positions.iter().enumerate() {
            battle.add_npc(&npc, (i + 1) as i64, r, c, 7);
        }

        for &(r, c) in &positions {
            let cell = battle.cell(r, c).unwrap();
            assert_eq!(cell.typ, 7);
            assert_eq!(cell.team, 2);
        }
    }

    #[test]
    fn win_lose_checks() {
        let mut battle = Battle::new(1, 112);

        // Initially all empty → both sides "dead"
        assert!(battle.all_enemies_dead());
        assert!(battle.all_players_dead());

        // Add a player
        let mut session = Session::new();
        session.id = 300001;
        session.hp = 100;
        session.hp_max = 100;
        battle.add_player(&session, 1, 3, 2);
        assert!(!battle.all_players_dead());
        assert!(battle.all_enemies_dead());

        // Add an NPC
        let npc = Npc {
            id: 1,
            hp: 100,
            ..Default::default()
        };
        battle.add_npc(&npc, 1, 0, 2, 3);
        assert!(!battle.all_enemies_dead());
        assert_eq!(battle.count_enemies_alive(), 1);
        assert_eq!(battle.count_players_alive(), 1);
    }

    #[test]
    fn battle_counter() {
        let mut counter = BattleCounter::new();
        assert_eq!(counter.next(), 1);
        assert_eq!(counter.next(), 2);
        assert_eq!(counter.next(), 3);
    }

    #[test]
    fn turn_order_sorts_correctly() {
        let mut battle = Battle::with_seeds(1, 112, 42, 43, 44);

        if let Some(cell) = battle.cell_mut(3, 2) {
            cell.id = 1;
            cell.agi = 100;
            cell.attacked = true;
            cell.random = 50;
        }
        if let Some(cell) = battle.cell_mut(0, 2) {
            cell.id = 2;
            cell.agi = 200;
            cell.attacked = true;
            cell.random = 30;
        }

        let order = battle.turn_order();
        assert_eq!(order[0], (0, 2)); // agi=200 higher
        assert_eq!(order[1], (3, 2));
    }

    #[test]
    fn npc_battle_start_packets() {
        let mut battle = Battle::new(1, 112);
        let mut session = Session::new();
        session.id = 300001;
        session.hp = 100;
        session.hp_max = 100;
        session.sp = 50;
        session.sp_max = 50;
        session.level = 10;
        session.thuoctinh = 1;
        battle.add_player(&session, session.id as i64, 3, 2);

        let npc = Npc {
            id: 1001,
            hp: 500,
            sp: 200,
            lv: 10,
            thuoctinh: 1,
            ..Default::default()
        };
        battle.add_npc(&npc, 1, 0, 2, 3);

        let packets = battle.npc_battle_start_packets(112);
        assert!(packets.len() >= 3); // open + show on map + enemy entity
        match &packets[0] {
            StartPacket::To { frame, .. } => assert!(frame.starts_with("F4441C000BFA")),
            _ => panic!("first packet must be the leader open frame"),
        }
    }

    #[test]
    fn npc_battle_constructor() {
        let leader = Session::new();
        let npc = Npc {
            id: 1001,
            hp: 500,
            lv: 10,
            thuoctinh: 1,
            ..Default::default()
        };
        let battle = Battle::npc_battle(1, &leader, 300001, &npc, 7, trigger_diahinh::PK_AND_NPC);
        assert_eq!(battle.diahinh, 112);
        assert_eq!(battle.cell(3, 2).unwrap().id, leader.id as i64);
        assert_eq!(battle.cell(0, 2).unwrap().typ, 3);
    }

    #[test]
    fn teamdef_battle_constructor() {
        let mut leader = Session::new();
        leader.id = 300001;
        let mut member = Session::new();
        member.id = 300002;
        let npcs: Vec<Npc> = (1..=3)
            .map(|i| Npc {
                id: 1000 + i,
                hp: 300,
                lv: 5,
                thuoctinh: 2,
                ..Default::default()
            })
            .collect();
        let defs: Vec<&Npc> = npcs.iter().collect();
        let battle = Battle::teamdef_battle(1, &leader, 300001, &[&member], &defs, 4712);
        // Defenders at (0,0),(0,1),(0,2) Type 7.
        assert_eq!(battle.cell(0, 0).unwrap().typ, 7);
        assert_eq!(battle.cell(0, 0).unwrap().id, 1001);
        assert_eq!(battle.cell(0, 2).unwrap().id, 1003);
        // Member placed at col 1.
        assert_eq!(battle.cell(3, 1).unwrap().id, 300002);
    }

    #[test]
    fn member_frame_has_markers() {
        let mut battle = Battle::new(1, 112);
        let mut leader = Session::new();
        leader.id = 300001;
        battle.add_player(&leader, leader.id as i64, 3, 2);

        let mut member = Session::new();
        member.id = 300002;
        member.hp = 100;
        member.hp_max = 100;
        battle.add_player(&member, leader.id as i64, 3, 1);

        let frames = battle.member_battle_frame(112, 300002);
        assert!(!frames.is_empty());
        // Find the 0BFA open frame among the member packets.
        let open = frames
            .iter()
            .filter_map(|p| match p {
                StartPacket::To { frame, .. } => Some(frame),
                _ => None,
            })
            .find(|f| f.contains("0BFA"))
            .expect("member open frame present");
        assert!(open.contains("05")); // self marker
        assert!(open.contains("03")); // leader marker
        assert!(open.ends_with("F44403000B0A01"));
    }
}