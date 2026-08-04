//! Battle construction (Chapter 6 §6.1).
//!
//! Creates a new battle grid, populates cells with players/NPCs/pets,
//! and manages the IdBattle counter.

use crate::battle::engine::{war_key, WarInfo};
use crate::battle::rng::BattleRng;
use crate::data::tables::Npc;
use crate::server::session::Session;
use std::collections::HashMap;

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

    /// Add a pet to the battle grid at the pet row (row ^ 1, col).
    pub fn add_pet(
        &mut self,
        pet: &crate::server::session::PetState,
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

    /// Generate the battle start packets for an NPC battle (leader view).
    pub fn npc_battle_start_packets(&self, diahinh: i32) -> Vec<String> {
        let mut out = Vec::new();

        // Leader at (3,2)
        if let Some(leader) = self.cell(3, 2) {
            if leader.id <= 0 {
                return out;
            }
            let leader_packet = leader.packet_hex();

            // Open frame
            out.push(crate::battle::packets::battle_open_leader(
                diahinh as u16,
                &leader_packet,
            ));

            // Leader on map
            out.push(crate::battle::packets::show_player_on_map(leader.id as u32));

            // Leader's pet at (2,2)
            if let Some(pet) = self.cell(2, 2) {
                if pet.id > 0 {
                    out.push(crate::battle::packets::entity_player(&pet.packet_hex()));
                }
            }

            // Members at cols 1, 3, 0, 4
            for &col in &[1u8, 3, 0, 4] {
                if let Some(member) = self.cell(3, col) {
                    if member.id > 0 {
                        let pkt = member.packet_hex();
                        out.push(crate::battle::packets::entity_player(&pkt));
                        out.push(crate::battle::packets::show_member_on_map(member.id as u32));

                        // Member's pet
                        if let Some(pet) = self.cell(2, col) {
                            if pet.id > 0 {
                                out.push(crate::battle::packets::entity_player(&pet.packet_hex()));
                            }
                        }
                    }
                }
            }

            // Enemy entities (rows 0-1)
            for row in 0..2u8 {
                for col in 0..5u8 {
                    if let Some(enemy) = self.cell(row, col) {
                        if enemy.id > 0 {
                            out.push(crate::battle::packets::entity_npc(&enemy.packet_hex()));
                        }
                    }
                }
            }
        }

        out
    }
}

/// Global battle counter (starts at 1, assignment before increment).
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

        // All enemy cells should be populated
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

        // Add two cells with different agi
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
        // First entry should be (0,2) with agi=200 (higher)
        assert_eq!(order[0], (0, 2));
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
        assert!(packets[0].starts_with("F4441C000BFA")); // battle open
    }
}
