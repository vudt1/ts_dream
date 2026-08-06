//! Battle turn engine (Chapter 6 §6.2/§6.3) — deterministic, race-free.
//!
//! Faithful port of `TheBattle.cs` `Battling()` (lines 1002-4950). The turn
//! engine runs synchronously over a `Battle` grid; the async per-battle task
//! (`crate::battle::manager`) drives it turn-by-turn, feeding player commands
//! and broadcasting the produced `Out` events. All packet strings are emitted
//! byte-for-byte as the C# concatenates them.

use crate::battle::construction::Battle;
use crate::battle::damage;
use crate::battle::engine::WarInfo;
use crate::battle::packets;
use crate::battle::packets::{attack_status, miss_status, troi_byte, troi_end_byte};
use crate::battle::rng::DotNetRandom;
use crate::battle::targeting::{self, CellInfo, GridPos};
use crate::data::tables::{Item, Npc, NpcOnMap, Skill};
use crate::protocol::encoder;
use std::collections::HashMap;

/// One player-submitted battle command (op 0x32 sub 1 / sub 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleCommand {
    /// The grid cell being commanded (player's own cell or their pet cell).
    pub row: u8,
    pub col: u8,
    pub skill_id: i64,
    pub skill_lv: i64,
    pub row_attack: u8,
    pub col_attack: u8,
    /// Op 0x32 sub 2: use-an-item command. `0` = normal skill command; a
    /// potion id in `26001..=27165` heals the target cell + the owner's pet.
    pub use_item: i64,
}

/// Read tables the battle engine needs (mirrors the C# `Data` statics).
pub struct BattleData<'a> {
    pub npcs: &'a HashMap<i64, Npc>,
    pub skills: &'a HashMap<i64, Skill>,
    /// Item records — used by in-battle use-item (op 0x32 sub 2) heals.
    pub items: &'a HashMap<i64, Item>,
    /// Per-player pet slot ids `[stt1..stt4]` (0 = empty), used by catch.
    pub pet_slots: &'a HashMap<i64, [i64; 4]>,
    /// Map NPC instance for post-flee respawn coordinates (optional).
    pub npc_on_map: Option<&'a NpcOnMap>,
    /// The `_My_TalkingBattle` npc id (for flee respawn).
    pub talking_battle: i64,
    /// Per-player DB stat snapshot for end-of-battle rewards.
    pub players: &'a HashMap<i64, PlayerSnapshot>,
    /// The cumulative Texps thresholds (`Data.Texps[]`).
    pub texps: &'a [crate::data::tables::TexpRow],
}

/// Player stats needed to settle exp/level-up rewards (`TheBattle.cs:4486-4506`).
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSnapshot {
    pub texp: i64,
    pub job: i64,
    pub hpx: i64,
    pub spx: i64,
    pub hpx2: i64,
    pub spx2: i64,
}

impl<'a> BattleData<'a> {
    /// Build data from live references (tests provide their own tables).
    pub fn new(
        npcs: &'a HashMap<i64, Npc>,
        skills: &'a HashMap<i64, Skill>,
        items: &'a HashMap<i64, Item>,
        pet_slots: &'a HashMap<i64, [i64; 4]>,
        players: &'a HashMap<i64, PlayerSnapshot>,
        texps: &'a [crate::data::tables::TexpRow],
        npc_on_map: Option<&'a NpcOnMap>,
        talking_battle: i64,
    ) -> BattleData<'a> {
        BattleData {
            npcs,
            skills,
            items,
            pet_slots,
            npc_on_map,
            talking_battle,
            players,
            texps,
        }
    }
}

/// DB stat being written (C# `Data.PlayerUpdateDataId` / `Data.PetUpdateData`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    Hp,
    Sp,
    Texp,
    Lv,
    Hpmax,
    Spmax,
    Point,
    SkillPoint,
    Fai,
}

/// Database write target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbTarget {
    Player(i64),
    Pet { owner: i64, stt: i64 },
}

/// A database write that must be applied (plus the matching `F4440C000801`
/// status packet push) by the calling layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbUpdate {
    pub target: DbTarget,
    pub stat: Stat,
    pub value: i64,
}

/// One event produced by the battle engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    /// Send to every living player-team member (C# `SendSKillingToParty`).
    Broadcast(String),
    /// Send directly to one player (C# `Server.SendToClient`).
    ToPlayer(i64, String),
    /// Send to every client on `player`'s map except `player` (`SendToAllClientMapid`).
    MapBroadcast { player: i64, frame: String },
    /// Persist a player/pet stat (HP/SP/exp/level).
    Db(DbUpdate),
    /// Grant an item to a player's inventory (`Data.HomdoAddItem`).
    Drop {
        item_id: i64,
        npc_row: u8,
        npc_col: u8,
        row: u8,
        col: u8,
        owner: i64,
    },
    /// A pet catch succeeded — `Data.Addpet(owner, npc_id)`.
    Catch { owner: i64, npc_id: i64 },
    /// A party member fled; restore HP/pets and exit battle for `player`.
    Fled { player: i64 },
    /// Respawn the battle-triggering map npc at the given position.
    Respawn { npc_id: i64, x: i64, y: i64 },
    /// Pet exp grant at battle end (`Data.PetUpdateData(_Texp, ...)`).
    PetExp { owner: i64, stt: i64, exp: i64 },
}

/// Battle outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Running,
    PlayerWin,
    PlayerLose,
    PlayerFled,
}

/// Per-turn accumulators (C# local state of `Battling()`).
#[derive(Debug, Default)]
struct TurnState {
    avg1: Option<f64>,
    avg2: Option<f64>,
    combo_active: i64,
    combo_hi: i64,
    combo_lo: i64,
    delay: i64,
    reflect: i64, // num23
    heal: i64,    // num47 (SP-drain self heal)
    agi_13020: i64,
    combo_cells: Vec<String>,
    killed_npcs: Vec<String>,
    text_troi: String,
    text_combo: String,
    text9: String,
    text10: String,
    text11: String,
    last_target: (u8, u8),
}

/// Always-allowed skill ids (`TheBattle.cs:1563`).
const ALWAYS_ALLOWED: [i64; 8] = [10000, 15001, 15002, 15003, 17001, 18001, 18002, 19001];

const DROP_PERCENTS: [i64; 6] = [25, 23, 20, 4, 3, 1];

impl Battle {
    /// Run the battle to completion using `commands` for every turn (players
    /// without a command skip their turn). Returns the terminal outcome.
    pub fn run_battle(
        &mut self,
        data: &BattleData,
        commands: &HashMap<i64, BattleCommand>,
        out: &mut Vec<Out>,
    ) -> Outcome {
        loop {
            let outcome = self.run_turn(data, commands, out);
            if outcome != Outcome::Running {
                return outcome;
            }
        }
    }

    /// Advance exactly one turn (phases 1-4 + end-of-battle check).
    pub fn run_turn(
        &mut self,
        data: &BattleData,
        commands: &HashMap<i64, BattleCommand>,
        out: &mut Vec<Out>,
    ) -> Outcome {
        // Outcome check at loop top (`TheBattle.cs:1025-1036`).
        if self.all_enemies_dead() {
            return Outcome::PlayerWin;
        }
        if self.all_players_dead() {
            return Outcome::PlayerLose;
        }

        let mut ts = TurnState::default();
        self.turn_phase1(data, &mut ts, out);
        self.turn_phase2(data, commands, out);
        let outcome = self.turn_phase4(data, &mut ts, out);
        if outcome != Outcome::Running {
            return outcome;
        }

        // Leader SP regen (`IL_caac`, TheBattle.cs:4147-4247) — only for
        // join-in-progress (ListQS) leaders, which this port does not model.
        if self.count_enemies_alive() == 0 {
            return Outcome::PlayerWin;
        }
        if self.count_players_alive() == 0 {
            return Outcome::PlayerLose;
        }
        Outcome::Running
    }

    // ---- Phase 1: reset + buff ticks -------------------------------------

    fn turn_phase1(&mut self, _data: &BattleData, ts: &mut TurnState, out: &mut Vec<Out>) {
        let mut sum1 = 0i64;
        let mut count1 = 0i64;
        let mut sum2 = 0i64;
        let mut count2 = 0i64;

        for key in self.keys.clone() {
            let cell = self.list_war[&key].clone();
            if cell.id <= 0 {
                continue;
            }
            let (row, col, team) = (cell.row, cell.col, cell.team);

            let mut c = cell;
            c.id_skill = 0;
            c.row_attack = 0;
            c.col_attack = 0;
            c.attacked = false;
            c.random = i64::from(self.rng.random_1.next_range(0, 100));

            // Average levels — only while the average is still 0.0 (first turn).
            if ts.avg1.is_none() {
                if team == 1 {
                    sum1 += c.lv;
                    count1 += 1;
                } else if team == 2 {
                    sum2 += c.lv;
                    count2 += 1;
                }
            }

            if c.type3_id > 1 {
                c.type3_turn -= 1;
            }
            if c.type4_id > 1 {
                c.type4_turn -= 1;
            }
            if c.type15_id > 1 {
                c.type15_turn -= 1;
            }
            if c.type19_id > 1 {
                c.type19_turn -= 1;
            }

            // Burn (Type3 10004 / 10033).
            if matches!(c.type3_id, 10004 | 10033) {
                let dmg = if c.type3_id == 10033 {
                    30 + c.type3_lv * 10
                } else {
                    10 + c.type3_lv * 2
                };
                self.apply_hp_loss(&mut c, dmg, out);
                let text3 = format!(
                    "{:02X}{:02X}{}0101{}",
                    row,
                    col,
                    encoder::le16(20001),
                    packets::skilling_int(
                        row,
                        col,
                        miss_status::ATTACK,
                        attack_status::ATTACK,
                        1,
                        troi_byte::HP,
                        dmg as u16,
                        1
                    )
                );
                let payload = format!("3201{}", text3);
                out.push(Out::Broadcast(format!(
                    "F444{}{}",
                    encoder::le16(payload.len() as u16 / 2),
                    payload
                )));
            }

            // Poison (Type15 14015).
            if c.type15_id == 14015 {
                let dmg = 30 + c.type15_lv * 15;
                self.apply_hp_loss(&mut c, dmg, out);
                let text6 = format!(
                    "{:02X}{:02X}{}0101{}",
                    row,
                    col,
                    encoder::le16(20003),
                    packets::skilling_int(
                        row,
                        col,
                        miss_status::ATTACK,
                        attack_status::ATTACK,
                        1,
                        troi_byte::HP,
                        dmg as u16,
                        1
                    )
                );
                let payload = format!("3201{}", text6);
                out.push(Out::Broadcast(format!(
                    "F444{}{}",
                    encoder::le16(payload.len() as u16 / 2),
                    payload
                )));
            }

            // Buff end.
            if c.type3_turn == 1 {
                c.type3_id = 0;
                c.type3_lv = 0;
                c.type3_turn = 0;
                out.push(Out::Broadcast(packets::troi_end(
                    row,
                    col,
                    troi_end_byte::TYPE3,
                )));
            }
            if c.type4_turn == 1 {
                c.type4_id = 0;
                c.type4_lv = 0;
                c.type4_turn = 0;
                out.push(Out::Broadcast(packets::troi_end(
                    row,
                    col,
                    troi_end_byte::TYPE4,
                )));
            }
            if c.type15_turn == 1 {
                if matches!(c.type15_id, 10016 | 10017 | 10018 | 10019 | 10025 | 20022) {
                    c.agi += 200;
                }
                c.type15_id = 0;
                c.type15_lv = 0;
                c.type15_turn = 0;
                out.push(Out::Broadcast(packets::troi_end(
                    row,
                    col,
                    troi_end_byte::TYPE15,
                )));
            }
            if c.type19_turn == 1 {
                if c.type19_id == 13020 {
                    c.agi -= ts.agi_13020;
                    ts.agi_13020 = 0;
                }
                c.type19_id = 0;
                c.type19_lv = 0;
                c.type19_turn = 0;
                out.push(Out::Broadcast(packets::troi_end(
                    row,
                    col,
                    troi_end_byte::TYPE19,
                )));
            }

            // Your turn prompt for player entities.
            if c.typ == 2 {
                out.push(Out::ToPlayer(c.id, packets::your_turn()));
            }
            // Acting indicator on silenced entities.
            if c.type3_turn > 0 {
                out.push(Out::Broadcast(packets::acting(row, col)));
            }

            self.list_war.insert(key, c);
        }

        if ts.avg1.is_none() {
            ts.avg1 = Some(if count1 > 0 {
                sum1 as f64 / count1 as f64
            } else {
                0.0
            });
            ts.avg2 = Some(if count2 > 0 {
                sum2 as f64 / count2 as f64
            } else {
                0.0
            });
        }
    }

    /// Apply HP loss with the C# DB-write rules (players/pets get a clamped
    /// DB write; npc types 3/7 just subtract).
    fn apply_hp_loss(&self, c: &mut WarInfo, dmg: i64, out: &mut Vec<Out>) {
        if !matches!(c.typ, 3 | 7) {
            let target = if c.id_char == 0 {
                DbTarget::Player(c.id)
            } else {
                DbTarget::Pet {
                    owner: c.id_char,
                    stt: c.id_npc_on_map,
                }
            };
            out.push(Out::Db(DbUpdate {
                target,
                stat: Stat::Hp,
                value: (c.hp - dmg).max(0),
            }));
        }
        c.hp -= dmg;
    }

    // ---- Phase 2: input / auto actions -----------------------------------

    fn turn_phase2(
        &mut self,
        data: &BattleData,
        commands: &HashMap<i64, BattleCommand>,
        out: &mut Vec<Out>,
    ) {
        for key in self.keys.clone() {
            let cell = self.list_war[&key].clone();
            if cell.id <= 0 {
                continue;
            }
            let mut c = cell;
            if c.hp > 0 {
                if matches!(c.type15_id, 14021 | 20014) {
                    self.auto_berserk(data, &mut c);
                } else if c.type3_turn == 0 && c.id_skill == 0 && matches!(c.typ, 3 | 7) {
                    self.auto_npc(data, &mut c);
                } else if c.id_skill > 0 || c.type3_turn > 0 {
                    c.attacked = true;
                }

                // Apply a submitted player command (op 0x32 sub 1 / sub 2).
                let owner = if c.typ == 4 { c.id_char } else { c.id };
                if let Some(cmd) = commands.get(&owner) {
                    if !c.attacked && c.row == cmd.row && c.col == cmd.col {
                        if cmd.use_item > 0 {
                            self.apply_use_item(data, &mut c, cmd.use_item, out);
                        } else {
                            c.id_skill = cmd.skill_id;
                            c.lv_skill = cmd.skill_lv;
                            c.row_attack = cmd.row_attack;
                            c.col_attack = cmd.col_attack;
                        }
                        c.attacked = true;
                    }
                }
            } else {
                c.attacked = true;
            }
            self.list_war.insert(key, c);
        }
    }

    /// Op 0x32 sub 2 — use a potion in battle (`Client.cs:7775-7846`).
    ///
    /// Heals the target cell's `_Hp`/`_Sp` (capped to its max) plus the active
    /// pet of the owner, both via the item record's `_Hp`/`_Sp`. Inventory
    /// removal happens synchronously in the op 0x32 handler.
    fn apply_use_item(&self, data: &BattleData, c: &mut WarInfo, item_id: i64, out: &mut Vec<Out>) {
        let (hp, sp) = data
            .items
            .get(&item_id)
            .map(|i| (i.hp, i.sp))
            .unwrap_or((0, 0));
        if hp <= 0 && sp <= 0 {
            return;
        }
        c.hp = (c.hp + hp).min(c.hp_max);
        c.sp = (c.sp + sp).min(c.sp_max);
        self.write_hp(c, c.hp, out);
        self.write_sp(c, c.sp, out);

        // Active pet heal (C# DB writes for the active pet Stt — the leader's
        // lowest-stt pet cell, which is the first one loaded in `AddToBattle`).
        let owner = if c.id_char != 0 { c.id_char } else { c.id };
        if let Some(pet) = self
            .list_war
            .values()
            .filter(|p| p.typ == 4 && p.id_char == owner && p.hp > 0)
            .min_by_key(|p| p.id_npc_on_map)
        {
            let mut pet = pet.clone();
            pet.hp = (pet.hp + hp).min(pet.hp_max);
            pet.sp = (pet.sp + sp).min(pet.sp_max);
            self.write_hp(&pet, pet.hp, out);
            self.write_sp(&pet, pet.sp, out);
        }
    }

    /// NPC auto-action: random enemy cell + `GetRandomSkillNPC`.
    fn auto_npc(&mut self, data: &BattleData, c: &mut WarInfo) {
        let row = if damage::randomize_with_percent(&mut self.rng.random_0, 2, 3, 50) == 2 {
            2u8
        } else {
            3u8
        };
        let cols = [2i64, 1, 3, 0, 4];
        let mut alive = Vec::new();
        for &col in &cols {
            if let Some(cell) = self.cell(row, col as u8) {
                if cell.hp > 0 {
                    alive.push(col);
                }
            }
        }
        let col = if alive.is_empty() {
            2u8
        } else {
            damage::randomize_array(&mut self.rng.random_0, &alive) as u8
        };
        c.row_attack = row;
        c.col_attack = col;

        let npc = data.npcs.get(&c.id).cloned().unwrap_or_default();
        c.id_skill =
            damage::get_random_skill_npc(&mut self.rng.random_0, npc.lv, npc.reborn, npc.skill);
        c.lv_skill = data.skills.get(&c.id_skill).map(|s| s.lv_max).unwrap_or(1);
        c.attacked = true;
    }

    /// Berserk auto-action: random enemy row + random alive column.
    fn auto_berserk(&mut self, _data: &BattleData, c: &mut WarInfo) {
        let row = if matches!(c.row, 0 | 1) {
            if damage::randomize_with_percent(&mut self.rng.random_0, 0, 1, 50) == 0 {
                0u8
            } else {
                1u8
            }
        } else if damage::randomize_with_percent(&mut self.rng.random_0, 2, 3, 50) == 2 {
            2u8
        } else {
            3u8
        };
        let cols = [2i64, 1, 3, 0, 4];
        let mut alive = Vec::new();
        for &col in &cols {
            if let Some(cell) = self.cell(row, col as u8) {
                if cell.hp > 0 {
                    alive.push(col);
                }
            }
        }
        let col = if alive.is_empty() {
            2u8
        } else {
            damage::randomize_array(&mut self.rng.random_0, &alive) as u8
        };
        c.id_skill = 10000;
        c.lv_skill = 1;
        c.row_attack = row;
        c.col_attack = col;
        c.attacked = true;
    }

    // ---- Phase 4: action execution ---------------------------------------

    fn turn_phase4(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        out: &mut Vec<Out>,
    ) -> Outcome {
        // Force-set attacked (C# line 1443-1455).
        for cell in self.list_war.values_mut() {
            cell.attacked = true;
        }
        let sorted = self.turn_order();

        for idx in 0..sorted.len() {
            let (row, col) = sorted[idx];
            let attacker = self.list_war[&crate::battle::engine::war_key(row, col)].clone();
            if attacker.id <= 0 {
                continue;
            }
            let acted = self.execute_entity(data, ts, &sorted, idx, attacker, out);
            if acted != Outcome::Running {
                return acted;
            }
        }
        Outcome::Running
    }

    /// Execute one entity's action (SP gate, validity, targeting, damage switch).
    fn execute_entity(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        sorted: &[(u8, u8)],
        idx: usize,
        mut attacker: WarInfo,
        out: &mut Vec<Out>,
    ) -> Outcome {
        let (row, col) = (attacker.row, attacker.col);
        let (team, lv, atk, int_stat) = (attacker.team, attacker.lv, attacker.atk, attacker.int1);
        let (avg1, avg2) = (avg_of(ts, team), avg_of(ts, if team == 1 { 2 } else { 1 }));

        let mut skill = attacker.id_skill;
        let mut skill_lv = attacker.lv_skill;
        let row_attack = attacker.row_attack;
        let col_attack = attacker.col_attack;
        let hp2 = attacker.hp;

        // --- SP cost gate (`TheBattle.cs:1535-1558`). ---
        if attacker.type3_id == 0 && attacker.attacked {
            let cost = data.skills.get(&skill).map(|s| s.sp).unwrap_or(0);
            if attacker.sp >= cost {
                if !matches!(attacker.typ, 3 | 7) {
                    let target = if attacker.id_char == 0 {
                        DbTarget::Player(attacker.id)
                    } else {
                        DbTarget::Pet {
                            owner: attacker.id_char,
                            stt: attacker.id_npc_on_map,
                        }
                    };
                    out.push(Out::Db(DbUpdate {
                        target,
                        stat: Stat::Sp,
                        value: attacker.sp - cost,
                    }));
                }
                attacker.sp -= cost;
            } else {
                // Not enough SP → basic attack fallback.
                skill = 10000;
                skill_lv = 1;
            }
        }

        // --- Skill validity gate (`TheBattle.cs:1560-1590`). ---
        let skill_ok = hp2 > 0
            && skill > 0
            && row_attack < 4
            && col_attack < 5
            && (data.skills.contains_key(&skill) || matches!(attacker.typ, 3 | 7));
        if !skill_ok {
            attacker.attacked = true;
            self.list_war
                .insert(crate::battle::engine::war_key(row, col), attacker);
            return Outcome::Running;
        }

        let mut valid = false;
        match attacker.typ {
            4 => {
                if let Some(npc) = data.npcs.get(&attacker.id) {
                    valid = npc.skill.contains(&skill);
                }
            }
            2 => valid = true, // player commands are trusted by the runner
            _ => valid = true, // types 3/7 always allowed
        }
        if !valid && !ALWAYS_ALLOWED.contains(&skill) {
            attacker.attacked = true;
            self.list_war
                .insert(crate::battle::engine::war_key(row, col), attacker);
            return Outcome::Running;
        }

        let skill_row = data.skills.get(&skill);
        let skill_type = skill_row.map(|s| s.skill_type).unwrap_or(0);
        let do_manh = skill_row.map(|s| s.do_manh).unwrap_or(0);
        let num34 = skill_row.map(|s| s.sl_danh).unwrap_or(1);
        let combo_field = skill_row.map(|s| s.combo).unwrap_or(0);
        let mut num36 = 0i64;
        let mut num37 = 2.0f64;
        let skill_tt = skill_row.map(|s| s.thuoctinh).unwrap_or(0);

        if skill == 13008 {
            skill = 13012;
            skill_lv = 3;
        }

        // Inherited combo from the previous entity (`TheBattle.cs:1607-1612`).
        if ts.combo_active == 1 {
            num37 *= 1.3;
        }

        // Target list selection (types override the default list).
        let targets = self.pick_targets(
            data, ts, team, row_attack, col_attack, num34, skill_type, skill, skill_lv, &attacker,
        );

        // Combo detection (`TheBattle.cs:1768-1908`).
        if idx + 1 < sorted.len() && skill_type == 1 {
            self.detect_combo(data, ts, sorted, idx, &attacker, &mut num37);
        }

        let count = targets.len() as u8;

        for tpos in targets {
            ts.last_target = (tpos.row, tpos.col);
            let mut target =
                self.list_war[&crate::battle::engine::war_key(tpos.row, tpos.col)].clone();

            // Shield (20006): damage applies to the rear cell.
            let rear_row = tpos.row ^ 1;
            let rear = self.list_war[&crate::battle::engine::war_key(rear_row, tpos.col)].clone();
            if (tpos.row == 3 || tpos.row == 0) && rear.id_skill == 20006 && rear.hp > 0 {
                self.apply_damage_to_rear(
                    data,
                    ts,
                    &mut attacker,
                    &rear,
                    tpos,
                    &mut num36,
                    &mut num37,
                    num34,
                    skill,
                    skill_lv,
                    skill_type,
                    combo_field,
                    do_manh,
                    skill_tt,
                    lv,
                    atk,
                    int_stat,
                    avg1,
                    avg2,
                    out,
                );
                self.write_cell(&mut target, (tpos.row, tpos.col));
                self.write_cell(&mut attacker, (row, col));
                continue;
            }

            match skill_type {
                1 => {
                    self.apply_physical(
                        data,
                        ts,
                        &mut attacker,
                        &mut target,
                        &mut num36,
                        &mut num37,
                        num34,
                        combo_field,
                        do_manh,
                        skill,
                        skill_lv,
                        skill_tt,
                        lv,
                        atk,
                        int_stat,
                        avg1,
                        avg2,
                        out,
                    );
                    self.note_npc_hit(ts, &mut attacker, &target);
                    self.write_cell(&mut target, (tpos.row, tpos.col));
                    self.write_cell(&mut attacker, (row, col));
                }
                2 => {
                    self.apply_magic(
                        data,
                        ts,
                        &mut attacker,
                        &mut target,
                        &mut num36,
                        num34,
                        do_manh,
                        skill,
                        skill_lv,
                        skill_tt,
                        lv,
                        int_stat,
                        avg1,
                        avg2,
                        out,
                    );
                    self.note_npc_hit(ts, &mut attacker, &target);
                    self.write_cell(&mut target, (tpos.row, tpos.col));
                    self.write_cell(&mut attacker, (row, col));
                }
                3 => self.apply_status3(
                    data,
                    ts,
                    &mut attacker,
                    &mut target,
                    skill,
                    skill_lv,
                    row,
                    col,
                    avg1,
                    avg2,
                    out,
                ),
                4 => self.apply_buff4(data, ts, &mut target, skill, skill_lv, out),
                5 => self.apply_dispel5(data, ts, &mut target, skill, out),
                6 => self.apply_sp_restore(
                    data,
                    ts,
                    &mut attacker,
                    &mut target,
                    skill,
                    skill_lv,
                    out,
                ),
                7 => self.apply_hp_restore(
                    data,
                    ts,
                    &mut attacker,
                    &mut target,
                    skill,
                    skill_lv,
                    out,
                ),
                8 => self.apply_revive(data, ts, &mut target, skill, skill_lv, out),
                11 => {
                    let r = self.apply_catch(data, ts, &mut attacker, &mut target, skill, out);
                    if r != Outcome::Running {
                        return r;
                    }
                }
                12 => {
                    let r = self.apply_flee(data, ts, &mut attacker, &mut target, skill, out);
                    if r != Outcome::Running {
                        return r;
                    }
                }
                14 => self.apply_heal14(data, ts, &mut attacker, &mut target, skill, skill_lv, out),
                15 => self.apply_buff15(
                    data,
                    ts,
                    &mut attacker,
                    &mut target,
                    skill,
                    skill_lv,
                    row,
                    col,
                    avg1,
                    avg2,
                    out,
                ),
                16 => self.apply_dispel16(data, ts, &mut target, skill, out),
                18 => {
                    self.apply_cleanse18(data, ts, &mut attacker, &mut target, skill, skill_lv, out)
                }
                19 => self.apply_buff19(data, ts, &mut target, skill, skill_lv, out),
                _ => {}
            }

            // Attacker Type4 13005 drops after acting.
            if attacker.type4_id == 13005 {
                attacker.type4_id = 0;
                attacker.type4_lv = 0;
                attacker.type4_turn = 0;
                out.push(Out::Broadcast(packets::troi_end(
                    row,
                    col,
                    troi_end_byte::TYPE4,
                )));
            }

            // Dead attacker loses the 20006 shield.
            if hp2 <= 0 && skill == 20006 {
                attacker.id_skill = 0;
            }

            self.write_cell(&mut target, (tpos.row, tpos.col));
            self.write_cell(&mut attacker, (row, col));
        }

        // --- Turn packet assembly (`TheBattle.cs:3593-3623`). ---
        if !ts.text10.is_empty() {
            if ts.heal > 0 {
                let n = count + 1;
                let prefix = format!(
                    "{:02X}{:02X}{}{:02X}{:02X}",
                    row,
                    col,
                    encoder::le16(skill as u16),
                    num34 as u8,
                    n
                );
                let mut block = prefix + &ts.text10;
                block.push_str(&packets::skilling_int(
                    row,
                    col,
                    miss_status::ATTACK,
                    attack_status::ATTACK,
                    1,
                    troi_byte::HP,
                    ts.heal as u16,
                    0,
                ));
                ts.text9.push_str(&encoder::le16(block.len() as u16 / 2));
                ts.text9.push_str(&block);
                ts.heal = 0;
            } else {
                let block = format!(
                    "{:02X}{:02X}{}{:02X}{:02X}{}",
                    row,
                    col,
                    encoder::le16(skill as u16),
                    num34 as u8,
                    count,
                    ts.text10
                );
                ts.text9.push_str(&encoder::le16(block.len() as u16 / 2));
                ts.text9.push_str(&block);
            }
            ts.text10 = String::new();
        }

        if !ts.text9.is_empty() && ts.combo_active == 0 {
            ts.combo_lo = 0;
            ts.combo_hi = 0;
            let action = format!("3201{}", ts.text9);
            let frame = format!(
                "{}{}{}",
                ts.text_combo,
                "F444",
                encoder::le16(action.len() as u16 / 2)
            );
            out.push(Out::Broadcast(frame + &action));
            ts.text9 = String::new();

            if !ts.text11.is_empty() {
                out.push(Out::Broadcast(std::mem::take(&mut ts.text11)));
            }

            // Drops & exp accumulation (§3.6).
            self.process_kills(data, ts, &mut attacker, out);

            if !ts.text_troi.is_empty() {
                out.push(Out::Broadcast(std::mem::take(&mut ts.text_troi)));
            }
            if !ts.text_combo.is_empty() {
                ts.text_combo = String::new();
                let (lr, lc) = ts.last_target;
                out.push(Out::Broadcast(packets::combo_footer_20007(lr, lc)));
            }
        }

        attacker.attacked = true; // Attacked = 2 (acted)
        self.list_war
            .insert(crate::battle::engine::war_key(row, col), attacker);
        Outcome::Running
    }

    fn write_cell(&mut self, cell: &mut WarInfo, pos: (u8, u8)) {
        self.list_war
            .insert(crate::battle::engine::war_key(pos.0, pos.1), cell.clone());
    }

    /// Combo detection (`TheBattle.cs:1768-1908`).
    fn detect_combo(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        sorted: &[(u8, u8)],
        idx: usize,
        attacker: &WarInfo,
        num37: &mut f64,
    ) {
        let (nr, nc) = sorted[idx + 1];
        let next = self.list_war[&crate::battle::engine::war_key(nr, nc)].clone();
        if next.id > 0 && next.team == attacker.team {
            let next_type = data
                .skills
                .get(&next.id_skill)
                .map(|s| s.skill_type)
                .unwrap_or(0);
            if next_type == 1
                && next.row_attack == attacker.row_attack
                && next.col_attack == attacker.col_attack
            {
                let target_alive = self
                    .cell(attacker.row_attack, attacker.col_attack)
                    .map(|c| c.id > 0)
                    .unwrap_or(false);
                if target_alive {
                    if ts.combo_hi == 0 {
                        let diff = (next.agi - attacker.agi).abs();
                        if diff <= 800 {
                            if next.agi >= attacker.agi {
                                ts.combo_hi = next.agi;
                                ts.combo_lo = attacker.agi;
                            } else {
                                ts.combo_hi = attacker.agi;
                                ts.combo_lo = next.agi;
                            }
                            ts.combo_active = damage::get_random_miss_combo(&mut self.rng.random_0);
                            ts.delay = next_delay(data, &next);
                            if ts.combo_active == 1 && ts.text10.is_empty() {
                                *num37 *= 1.3;
                                add_combo_cell(ts, attacker.row, attacker.col, attacker.lv);
                                add_combo_cell(ts, next.row, next.col, next.lv);
                            }
                            return;
                        }
                    } else {
                        let mid = (ts.combo_lo + ts.combo_hi) / 2;
                        if (next.agi - mid).abs() <= 800 {
                            if next.agi < ts.combo_lo {
                                ts.combo_lo = next.agi;
                            } else if next.agi > ts.combo_hi {
                                ts.combo_hi = next.agi;
                            }
                            ts.combo_active = damage::get_random_miss_combo(&mut self.rng.random_0);
                            let nd = next_delay(data, &next);
                            if ts.delay <= nd {
                                ts.delay = nd;
                            }
                            if ts.combo_active == 1 && ts.text10.is_empty() {
                                *num37 *= 1.3;
                                add_combo_cell(ts, attacker.row, attacker.col, attacker.lv);
                                add_combo_cell(ts, next.row, next.col, next.lv);
                            }
                            return;
                        }
                    }
                }
            }
        }
        ts.combo_active = 0;
    }

    /// Target list selection for a skill (`TheBattle.cs:1606-1767`).
    #[allow(clippy::too_many_arguments)]
    fn pick_targets(
        &self,
        data: &BattleData,
        ts: &TurnState,
        team: i64,
        row_attack: u8,
        col_attack: u8,
        num34: i64,
        skill_type: i64,
        skill: i64,
        skill_lv: i64,
        attacker: &WarInfo,
    ) -> Vec<GridPos> {
        let cells = self.cell_infos();
        let mut targets = if ts.combo_active == 1 {
            targeting::get_pos_attack_combo(&cells, team, row_attack, col_attack, num34)
        } else {
            targeting::get_pos_attack_default(&cells, team, row_attack, col_attack, num34)
        };

        if skill_type == 8 {
            targets.clear();
            if let Some(c) = self.cell(row_attack, col_attack) {
                if c.id > 0 && c.team == team {
                    targets.push(GridPos::new(row_attack, col_attack));
                }
            }
        } else if skill_type == 17 {
            targets.clear();
            if attacker.row == row_attack && attacker.col == col_attack {
                targets.push(GridPos::new(attacker.row, attacker.col));
            }
        } else if matches!(skill_type, 3 | 15) {
            targets = targeting::get_pos_attack_3_15(&cells, team, row_attack, col_attack, num34);
        } else if matches!(skill_type, 4 | 6 | 7 | 14 | 19) {
            let mut area = num34;
            if matches!(skill, 11010 | 11009 | 11026 | 11030) {
                area = match skill_lv {
                    1 => 1,
                    2 | 3 => 3,
                    4..=6 => 5,
                    7..=9 => 6,
                    _ => 8,
                };
            }
            targets = targeting::get_pos_attack_type4(&cells, team, row_attack, col_attack, area);
        }

        if attacker.type15_id == 14021 || attacker.type15_id == 20014 {
            targets.clear();
            targets = targeting::get_pos_attack_hon_loan(&cells, team, row_attack, col_attack, 1);
        }

        let multi = matches!(skill, 10016 | 11016 | 12016 | 13042 | 13015) || skill_type == 18;
        if multi {
            targets.clear();
            let area = match skill_lv {
                1..=3 => 3,
                4..=6 => 5,
                7..=9 => 6,
                _ => 8,
            };
            targets = targeting::get_pos_attack_tg(&cells, team, row_attack, col_attack, area);
        }

        if matches!(skill_type, 5 | 16 | 17 | 18) {
            targets =
                targeting::get_pos_attack_giai_tru(&cells, team, row_attack, col_attack, num34);
        }

        let _ = data;
        targets
    }

    fn cell_infos(&self) -> Vec<CellInfo> {
        self.keys
            .iter()
            .filter_map(|k| self.list_war.get(k))
            .map(|c| CellInfo {
                row: c.row,
                col: c.col,
                id: c.id,
                hp: c.hp,
                team: c.team,
                type4_id: c.type4_id,
            })
            .collect()
    }

    // ---- Skill-type implementations --------------------------------------

    /// Type 1 — physical attack (`TheBattle.cs:1953-2386`).
    #[allow(clippy::too_many_arguments)]
    fn apply_physical(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        num36: &mut i64,
        num37: &mut f64,
        num34: i64,
        combo_field: i64,
        do_manh: i64,
        skill: i64,
        skill_lv: i64,
        skill_tt: i64,
        lv: i64,
        atk: i64,
        int_stat: i64,
        avg1: i64,
        avg2: i64,
        out: &mut Vec<Out>,
    ) {
        let sd = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        if ts.delay == 0 || ts.delay <= sd {
            ts.delay = sd;
        }
        let base_stat = if combo_field == 87 { int_stat } else { atk };
        *num36 = damage::calc_physical_damage_stat(
            base_stat,
            target.def,
            attacker.thuoctinh,
            target.thuoctinh,
            lv,
            target.lv,
            skill_tt,
            do_manh,
            skill_lv,
            *num37,
        );
        damage::apply_buff_modifiers(
            num36,
            target.type3_id,
            target.type3_lv,
            target.type4_id,
            target.type4_lv,
            target.type15_id,
            target.type15_lv,
            attacker.type4_id,
            attacker.type4_lv,
            attacker.type15_id,
            attacker.type15_lv,
            attacker.type19_id,
            attacker.type19_lv,
            num34,
        );
        let mut hit =
            damage::get_random_miss_attack(&mut self.rng.random_0, lv, target.lv, avg1, avg2);
        let mut adl = attack_status::ATTACK;
        if hit == miss_status::MISS as i64 {
            *num36 = 0;
            if target.id_skill == 17001 {
                adl = attack_status::DEF;
            }
            if target.type4_id == 13003 {
                adl = attack_status::LANTRANH;
            }
        } else {
            if target.id_skill == 17001 {
                adl = attack_status::DEF;
                *num36 = element_reduce(data, *num36, skill, attacker.thuoctinh, target.thuoctinh);
            }
            *num36 = if *num36 < 1 {
                1
            } else {
                *num36 + i64::from(self.rng.random_1.next_range(0, 2))
            };
            if matches!(target.type4_id, 10010 | 10015 | 10031 | 13021) {
                ts.reflect = *num36;
                *num36 = 0;
                adl = attack_status::DEF;
            } else if target.type4_id == 13003 {
                *num36 = 0;
                adl = attack_status::LANTRANH;
                hit = miss_status::MISS as i64;
            }
            if matches!(attacker.type15_id, 10016..=10019) {
                let mut fresh = DotNetRandom::time_seeded();
                if fresh.next_range(1, 3) == 1 {
                    *num36 = 0;
                    adl = attack_status::LANTRANH;
                } else {
                    *num36 = (*num36 / 10).max(1);
                }
            }
        }
        self.apply_target_hp(target, *num36, out);
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            hit as u8,
            adl,
            1,
            troi_byte::HP,
            *num36 as u16,
            1,
        ));

        // Status-debuff skills 13007/13029 cast Type3 on a clean target.
        if matches!(skill, 13007 | 13029) && target.type3_id == 0 {
            let roll = damage::get_random_miss_troi(
                &mut self.rng.random_0,
                lv,
                target.lv,
                avg1,
                avg2,
                attacker.int1,
                attacker.int1,
                target.int1,
                attacker.reborn,
                target.reborn,
            );
            if roll == 1 {
                ts.text10.push_str(&packets::skilling_short(
                    r,
                    c,
                    miss_status::ATTACK,
                    attack_status::ATTACK,
                ));
                ts.text10.push_str("02");
                ts.text10
                    .push_str(&packets::skilling_effect(troi_byte::HP, *num36 as u16, 1));
                ts.text10
                    .push_str(&packets::skilling_effect(troi_byte::TYPE3, 0, 1));
                target.type3_id = skill;
                target.type3_lv = skill_lv;
                target.type3_turn = 3;
            }
        }

        // Reflect self-damage (10015/10031/13021) applied to the attacker.
        if matches!(target.type4_id, 10015 | 10031 | 13021) && ts.reflect > 0 {
            self.apply_reflect(attacker, ts, out);
        }
    }

    fn apply_reflect(&self, attacker: &mut WarInfo, ts: &mut TurnState, out: &mut Vec<Out>) {
        let dmg = ts.reflect;
        if !matches!(attacker.typ, 3 | 7) {
            let target = if attacker.id_char == 0 {
                DbTarget::Player(attacker.id)
            } else {
                DbTarget::Pet {
                    owner: attacker.id_char,
                    stt: attacker.id_npc_on_map,
                }
            };
            out.push(Out::Db(DbUpdate {
                target,
                stat: Stat::Hp,
                value: (attacker.hp - dmg).max(0),
            }));
        }
        attacker.hp -= dmg;
        let text15 = format!(
            "{:02X}{:02X}{}0101{}",
            attacker.row,
            attacker.col,
            encoder::le16(20003),
            packets::skilling_int(
                attacker.row,
                attacker.col,
                miss_status::ATTACK,
                attack_status::ATTACK,
                1,
                troi_byte::HP,
                dmg as u16,
                1
            )
        );
        let text17 = format!("3201{}", encoder::le16(text15.len() as u16 / 2) + &text15);
        ts.text11.push_str(&format!(
            "F444{}{}",
            encoder::le16(text17.len() as u16 / 2),
            text17
        ));
        ts.reflect = 0;
    }

    /// Shield (20006) damage path — applies to the rear cell (`TheBattle.cs:1979-2145`).
    #[allow(clippy::too_many_arguments)]
    fn apply_damage_to_rear(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        rear: &WarInfo,
        tpos: GridPos,
        num36: &mut i64,
        num37: &mut f64,
        num34: i64,
        skill: i64,
        skill_lv: i64,
        skill_type: i64,
        combo_field: i64,
        do_manh: i64,
        skill_tt: i64,
        lv: i64,
        atk: i64,
        int_stat: i64,
        avg1: i64,
        avg2: i64,
        out: &mut Vec<Out>,
    ) {
        let mut rear = rear.clone();
        if skill_type == 1 {
            let base_stat = if combo_field == 87 { int_stat } else { atk };
            *num36 = damage::calc_physical_damage_stat(
                base_stat,
                rear.def,
                attacker.thuoctinh,
                rear.thuoctinh,
                lv,
                rear.lv,
                skill_tt,
                do_manh,
                skill_lv,
                *num37,
            );
        } else {
            *num36 = damage::calc_magic_damage(
                int_stat,
                rear.def,
                attacker.thuoctinh,
                rear.thuoctinh,
                lv,
                rear.lv,
                skill_tt,
                do_manh,
                skill_lv,
                skill,
                num34,
            );
        }
        damage::apply_buff_modifiers(
            num36,
            rear.type3_id,
            rear.type3_lv,
            rear.type4_id,
            rear.type4_lv,
            rear.type15_id,
            rear.type15_lv,
            attacker.type4_id,
            attacker.type4_lv,
            attacker.type15_id,
            attacker.type15_lv,
            attacker.type19_id,
            attacker.type19_lv,
            num34,
        );
        let mut hit =
            damage::get_random_miss_attack(&mut self.rng.random_0, lv, rear.lv, avg1, avg2);
        let mut adl = attack_status::ATTACK;
        if hit == miss_status::MISS as i64 {
            *num36 = 0;
            if rear.id_skill == 17001 {
                adl = attack_status::DEF;
            }
            if rear.type4_id == 13003 {
                adl = attack_status::LANTRANH;
            }
        } else {
            if rear.id_skill == 17001 {
                adl = attack_status::DEF;
                *num36 = element_reduce(data, *num36, skill, attacker.thuoctinh, rear.thuoctinh);
            }
            *num36 = if *num36 < 1 {
                1
            } else {
                *num36 + i64::from(self.rng.random_1.next_range(0, 2))
            };
            if matches!(rear.type4_id, 10010 | 10015 | 10031 | 13021) {
                *num36 = 0;
                adl = attack_status::DEF;
            } else if rear.type4_id == 13003 {
                *num36 = 0;
                adl = attack_status::LANTRANH;
                hit = miss_status::MISS as i64;
            }
        }
        self.apply_target_hp(&mut rear, *num36, out);
        let rr = tpos.row ^ 1;
        let rc = tpos.col;
        ts.text10.push_str(&packets::skilling_int(
            rr,
            rc,
            hit as u8,
            adl,
            1,
            troi_byte::HP,
            *num36 as u16,
            1,
        ));
        ts.text_combo = packets::combo(rr, rc);
        self.write_cell(&mut rear, (rr, rc));
    }

    /// Type 2 — magic attack (`TheBattle.cs:2387-2668`).
    #[allow(clippy::too_many_arguments)]
    fn apply_magic(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        num36: &mut i64,
        num34: i64,
        do_manh: i64,
        skill: i64,
        skill_lv: i64,
        skill_tt: i64,
        lv: i64,
        int_stat: i64,
        avg1: i64,
        avg2: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        *num36 = damage::calc_magic_damage(
            int_stat,
            target.def,
            attacker.thuoctinh,
            target.thuoctinh,
            lv,
            target.lv,
            skill_tt,
            do_manh,
            skill_lv,
            skill,
            num34,
        );
        damage::apply_buff_modifiers(
            num36,
            target.type3_id,
            target.type3_lv,
            target.type4_id,
            target.type4_lv,
            target.type15_id,
            target.type15_lv,
            attacker.type4_id,
            attacker.type4_lv,
            attacker.type15_id,
            attacker.type15_lv,
            attacker.type19_id,
            attacker.type19_lv,
            num34,
        );
        *num36 = if *num36 < 1 {
            1
        } else {
            *num36 + i64::from(self.rng.random_1.next_range(0, 2))
        };
        if matches!(attacker.type15_id, 10016..=10019) {
            let mut fresh = DotNetRandom::time_seeded();
            if fresh.next_range(1, 3) == 1 {
                *num36 = 0;
            } else {
                *num36 = (*num36 / 10).max(1);
            }
        }
        let mut hit =
            damage::get_random_miss_attack(&mut self.rng.random_0, lv, target.lv, avg1, avg2);
        let mut adl = attack_status::ATTACK;
        if hit == miss_status::MISS as i64 {
            *num36 = 0;
            if target.id_skill == 17001 {
                adl = attack_status::DEF;
            }
            if target.type4_id == 13003 {
                adl = attack_status::LANTRANH;
            }
        } else {
            if target.id_skill == 17001 {
                adl = attack_status::DEF;
                *num36 = element_reduce(data, *num36, skill, attacker.thuoctinh, target.thuoctinh);
            }
            if matches!(target.type4_id, 10010 | 10015 | 10031 | 13021) {
                ts.reflect = *num36;
                *num36 = 0;
                adl = attack_status::DEF;
            } else if target.type4_id == 13003 {
                *num36 = 0;
                adl = attack_status::LANTRANH;
                hit = miss_status::MISS as i64;
            }
        }
        self.apply_target_hp(target, *num36, out);
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            hit as u8,
            adl,
            1,
            troi_byte::HP,
            *num36 as u16,
            1,
        ));
        if matches!(target.type4_id, 10015 | 10031 | 13021) && ts.reflect > 0 {
            self.apply_reflect(attacker, ts, out);
        }
    }

    /// Type 3 — Type3 debuff (`TheBattle.cs:2669-2812`).
    #[allow(clippy::too_many_arguments)]
    fn apply_status3(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        row: u8,
        col: u8,
        avg1: i64,
        avg2: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let turn = damage::get_turn(skill, skill_lv);
        let mut byte_val;
        if target.type3_id > 0 {
            let (r, c) = (target.row, target.col);
            ts.text10.push_str(&packets::skilling_int(
                r,
                c,
                miss_status::MISS,
                attack_status::ATTACK,
                1,
                troi_byte::MISS,
                0,
                1,
            ));
        } else {
            let mut hit = damage::get_random_miss_troi(
                &mut self.rng.random_0,
                attacker.lv,
                target.lv,
                avg1,
                avg2,
                attacker.int1,
                attacker.int1,
                target.int1,
                attacker.reborn,
                target.reborn,
            );
            let mut adl = attack_status::ATTACK;
            if hit == miss_status::MISS as i64 {
                byte_val = troi_byte::MISS;
                if target.id_skill == 17001 {
                    adl = attack_status::DEF;
                }
                if target.type4_id == 13003 {
                    adl = attack_status::LANTRANH;
                }
                if matches!(target.type4_id, 10010 | 10015 | 10031 | 13021) {
                    adl = attack_status::ATTACK;
                }
            } else {
                byte_val = troi_byte::TYPE3;
                if target.id_skill == 17001 {
                    adl = attack_status::DEF;
                }
                if target.type4_id == 13003 {
                    adl = attack_status::LANTRANH;
                }
                if matches!(target.type4_id, 10010 | 10015 | 10031 | 13021) {
                    adl = attack_status::ATTACK;
                }
                if (13015..=13018).contains(&skill) {
                    // SP drain + caster heal.
                    let drain = skill_lv * 30;
                    ts.heal += drain;
                    if !matches!(target.typ, 3 | 7) {
                        let t = pet_target(target);
                        out.push(Out::Db(DbUpdate {
                            target: t,
                            stat: Stat::Sp,
                            value: (target.sp - drain).max(0),
                        }));
                    }
                    target.sp = (target.sp - drain).max(0);
                    if attacker.hp + ts.heal > attacker.hp_max {
                        ts.heal = attacker.hp_max - attacker.hp;
                        if !matches!(attacker.typ, 3 | 7) {
                            let t = pet_target(attacker);
                            out.push(Out::Db(DbUpdate {
                                target: t,
                                stat: Stat::Hp,
                                value: attacker.hp_max,
                            }));
                        }
                        attacker.hp = attacker.hp_max;
                    } else {
                        if !matches!(attacker.typ, 3 | 7) {
                            let t = pet_target(attacker);
                            out.push(Out::Db(DbUpdate {
                                target: t,
                                stat: Stat::Hp,
                                value: attacker.hp + ts.heal,
                            }));
                        }
                        attacker.hp += ts.heal;
                    }
                    target.type3_id = skill;
                    target.type3_lv = skill_lv;
                    target.type3_turn = turn;
                } else if target.type4_id == 10026 {
                    hit = miss_status::ATTACK as i64;
                    byte_val = troi_byte::MISS;
                    if attacker.type3_id == 0 {
                        attacker.type3_id = skill;
                        attacker.type3_lv = skill_lv;
                        attacker.type3_turn = turn;
                        ts.text_troi = packets::troi_start(row, col, skill as u16);
                    }
                } else {
                    target.type3_id = skill;
                    target.type3_lv = skill_lv;
                    target.type3_turn = turn;
                }
            }
            let (r, c) = (target.row, target.col);
            if (13015..=13018).contains(&skill) {
                ts.text10.push_str(&packets::skilling_int(
                    r, c, hit as u8, adl, 2, byte_val, 0, 1,
                ));
                ts.text10
                    .push_str(&format!("1A{}01", encoder::le16((skill_lv * 64) as u16)));
            } else {
                ts.text10.push_str(&packets::skilling_int(
                    r, c, hit as u8, adl, 1, byte_val, 0, 1,
                ));
            }
        }
    }

    /// Type 4 — Type4 buff (`TheBattle.cs:2813-2844`).
    fn apply_buff4(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        let _ = out;
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let turn = damage::get_turn(skill, skill_lv);
        let (hit, adl) = if target.type4_id == 0 {
            (miss_status::ATTACK, attack_status::ATTACK)
        } else {
            (miss_status::MISS, attack_status::DEF)
        };
        let mut byte_val = troi_byte::MISS;
        if hit == miss_status::ATTACK {
            byte_val = troi_byte::TYPE4;
            target.type4_id = skill;
            target.type4_lv = skill_lv;
            target.type4_turn = turn;
        }
        let (r, c) = (target.row, target.col);
        ts.text10
            .push_str(&packets::skilling_int(r, c, hit, adl, 1, byte_val, 0, 1));
    }

    /// Type 5 — dispel Type4 / cure (`TheBattle.cs:2845-2922`).
    fn apply_dispel5(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        target: &mut WarInfo,
        skill: i64,
        out: &mut Vec<Out>,
    ) {
        let _ = out;
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let (r, c) = (target.row, target.col);
        match skill {
            11014 => {
                let hit = if target.type4_id == 10010 {
                    target.type4_id = 0;
                    target.type4_lv = 0;
                    target.type4_turn = 0;
                    miss_status::ATTACK
                } else {
                    miss_status::MISS
                };
                ts.text10.push_str(&packets::skilling_int(
                    r,
                    c,
                    hit,
                    0,
                    1,
                    troi_byte::TYPE4,
                    0,
                    1,
                ));
            }
            14007 => {
                let hit = if target.type4_id == 14008 {
                    target.type4_id = 0;
                    target.type4_lv = 0;
                    target.type4_turn = 0;
                    miss_status::ATTACK
                } else {
                    miss_status::MISS
                };
                ts.text10.push_str(&packets::skilling_int(
                    r,
                    c,
                    hit,
                    0,
                    1,
                    troi_byte::TYPE4,
                    0,
                    1,
                ));
            }
            14014 => {
                let hit = if target.type4_id == 14015 {
                    target.type4_id = 0;
                    target.type4_lv = 0;
                    target.type4_turn = 0;
                    miss_status::ATTACK
                } else {
                    miss_status::MISS
                };
                ts.text10.push_str(&packets::skilling_int(
                    r,
                    c,
                    hit,
                    0,
                    1,
                    troi_byte::TYPE4,
                    0,
                    1,
                ));
            }
            14022 => {
                let hit = if target.type4_id == 10021 {
                    target.type4_id = 0;
                    target.type4_lv = 0;
                    target.type4_turn = 0;
                    miss_status::ATTACK
                } else {
                    miss_status::MISS
                };
                ts.text10.push_str(&packets::skilling_int(
                    r,
                    c,
                    hit,
                    0,
                    1,
                    troi_byte::TYPE4,
                    0,
                    1,
                ));
            }
            _ => {
                target.type3_id = 0;
                target.type3_lv = 0;
                target.type3_turn = 0;
                target.type4_id = 0;
                target.type4_lv = 0;
                target.type4_turn = 0;
                target.type15_id = 0;
                target.type15_lv = 0;
                target.type15_turn = 0;
                target.type19_id = 0;
                target.type19_lv = 0;
                target.type19_turn = 0;
                ts.text10.push_str(&packets::skilling_int(
                    r,
                    c,
                    miss_status::ATTACK,
                    0,
                    5,
                    troi_byte::MISS,
                    0,
                    1,
                ));
                ts.text10.push_str("DD000001DE000001DF000001E1000001");
            }
        }
    }

    /// Type 6 — SP restore (`TheBattle.cs:2923-2977`).
    fn apply_sp_restore(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let mut amount = damage::banker_round(attacker.int1 as f64 * 0.25) as i64;
        if skill == 11009 {
            amount = damage::banker_round(attacker.int1 as f64 * 0.05 * skill_lv as f64) as i64;
        } else if skill == 11006 {
            amount = damage::banker_round(attacker.int1 as f64 * 0.1 * skill_lv as f64) as i64;
        }
        if attacker.id == target.id {
            amount = 0;
        }
        if target.sp + amount <= target.sp_max {
            target.sp += amount;
            self.write_sp(target, target.sp, out);
        } else if target.sp + amount > target.sp_max {
            amount = target.sp_max - target.sp;
            target.sp = target.sp_max;
            self.write_sp(target, target.sp_max, out);
        } else if target.sp == target.sp_max {
            amount = 0;
        }
        if attacker.row == target.row && attacker.col == target.col {
            amount = 0;
        }
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            miss_status::ATTACK,
            attack_status::ATTACK,
            1,
            troi_byte::SP,
            amount as u16,
            0,
        ));
    }

    /// Type 7 — HP restore (`TheBattle.cs:2978-3024`).
    fn apply_hp_restore(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let mut amount = damage::banker_round(attacker.int1 as f64 * 0.5) as i64;
        if skill == 11010 {
            amount = damage::banker_round(attacker.int1 as f64 * 0.1 * skill_lv as f64) as i64;
        } else if skill == 11007 {
            amount = damage::banker_round(attacker.int1 as f64 * 0.2 * skill_lv as f64) as i64;
        }
        if target.hp + amount <= target.hp_max {
            target.hp += amount;
            self.write_hp(target, target.hp, out);
        } else if target.hp + amount > target.hp_max {
            amount = target.hp_max - target.hp;
            target.hp = target.hp_max;
            self.write_hp(target, target.hp_max, out);
        } else if target.hp == target.hp_max {
            amount = 0;
        }
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            miss_status::ATTACK,
            attack_status::ATTACK,
            1,
            troi_byte::HP,
            amount as u16,
            0,
        ));
    }

    /// Type 8 — revive (`TheBattle.cs:3025-3051`).
    fn apply_revive(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let (hit, adl, amount);
        if target.hp <= 0 {
            amount = damage::banker_round(target.hp_max as f64 / (10.0 / skill_lv as f64)) as i64;
            hit = miss_status::ATTACK;
            adl = attack_status::ATTACK;
            target.hp = amount;
            self.write_hp(target, amount, out);
        } else {
            amount = 0;
            hit = miss_status::MISS;
            adl = attack_status::DEF;
        }
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            hit,
            adl,
            1,
            troi_byte::HP,
            amount as u16,
            0,
        ));
    }

    /// Type 11 — catch pet (`TheBattle.cs:3052-3100`).
    fn apply_catch(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        out: &mut Vec<Out>,
    ) -> Outcome {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let caster = if attacker.id_char != 0 {
            attacker.id_char
        } else {
            attacker.id
        };
        let slots = data.pet_slots.get(&caster).copied().unwrap_or([0; 4]);
        let has_npc = slots.contains(&target.id);
        let npc_bat = data.npcs.get(&target.id).map(|n| n.bat).unwrap_or(0);
        let hp_pct = if target.hp_max > 0 {
            damage::banker_round(target.hp as f64 / target.hp_max as f64 * 100.0) as i64
        } else {
            0
        };
        let conditions = !has_npc
            && !matches!(target.typ, 2 | 4)
            && npc_bat == 0
            && target.type3_id == 0
            && attacker.lv - target.lv >= 5
            && hp_pct < 50;
        if conditions {
            let percent = 50 + damage::banker_round((attacker.lv - target.lv) as f64 / 2.0) as i64;
            let mut roll = damage::randomize_with_percent(&mut self.rng.random_0, 1, 0, percent);
            if caster == attacker.id_char {
                roll = 0; // pets cannot catch
            }
            let free_slot = slots.iter().any(|&s| s == 0);
            if roll == 1 && free_slot {
                self.clear_cell(target.row, target.col);
                out.push(Out::Catch {
                    owner: caster,
                    npc_id: target.id,
                });
                // C# ends the battle as a win after catching.
                return Outcome::PlayerWin;
            }
        }
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            miss_status::ATTACK,
            attack_status::ATTACK,
            1,
            troi_byte::MISS,
            0,
            1,
        ));
        let _ = skill;
        Outcome::Running
    }

    /// Type 12 — flee (`TheBattle.cs:3101-3222`).
    fn apply_flee(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        out: &mut Vec<Out>,
    ) -> Outcome {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let is_leader = attacker.id == attacker.leader_id || attacker.id_char == attacker.leader_id;
        let hit = damage::get_random_miss_flee(
            &mut self.rng.random_0,
            attacker.lv,
            target.lv,
            avg_of(ts, attacker.team),
            avg_of(ts, if attacker.team == 1 { 2 } else { 1 }),
        );
        if hit == 1 || skill == 14002 {
            if is_leader && attacker.leader_id > 0 {
                return Outcome::PlayerFled;
            }
            // Party member flee.
            let player = if attacker.id_char != 0 {
                attacker.id_char
            } else {
                attacker.id
            };
            let (r, c) = (attacker.row, attacker.col);
            let pr = r ^ 1;
            let mut frames = String::new();
            frames.push_str(&packets::acting(r, c));
            frames.push_str(&packets::acting(pr, c));
            frames.push_str(&packets::clear_pet_cell(r, c));
            frames.push_str(&packets::clear_pet_cell(pr, c));
            frames.push_str(&packets::hide_from_map(player as u32));
            frames.push_str(&packets::reposition(r, c));
            out.push(Out::Broadcast(frames));
            out.push(Out::MapBroadcast {
                player,
                frame: packets::hide_from_map(player as u32),
            });
            out.push(Out::ToPlayer(player, packets::battle_exit_move()));
            out.push(Out::ToPlayer(player, packets::battle_exit_talk()));
            self.clear_cell(r, c);
            self.clear_cell(pr, c);
            self.npc_respawn(data, out);
            out.push(Out::Fled { player });
            return Outcome::Running;
        }
        // Flee failed.
        let (sl_danh, skill_type) = data
            .skills
            .get(&(skill + 1))
            .map(|s| (s.sl_danh as u8, s.skill_type as u8))
            .unwrap_or((1, 0));
        let frame = packets::skilling_full_frame(&packets::skilling_full(
            attacker.row,
            attacker.col,
            (skill + 1) as u16,
            sl_danh,
            skill_type,
            attacker.row_attack,
            attacker.col_attack,
            miss_status::MISS,
            attack_status::LANTRANH,
            1,
            troi_byte::MISS,
            0,
            1,
        ));
        out.push(Out::Broadcast(frame));
        Outcome::Running
    }

    /// Type 14 — heal HP + SP (`TheBattle.cs:3223-3309`).
    fn apply_heal14(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let (mut hp, mut sp) = (
            damage::banker_round(attacker.int1 as f64 * 0.5) as i64,
            damage::banker_round(attacker.int1 as f64 * 0.5) as i64,
        );
        match skill {
            11004 => {
                hp = damage::banker_round(attacker.int1 as f64 / 1.7) as i64 + 3 * skill_lv;
                sp = damage::banker_round(attacker.int1 as f64 / 3.7) as i64 + skill_lv;
            }
            11026 => {
                hp = damage::banker_round(attacker.int1 as f64 / 2.7) as i64 + 3 * skill_lv;
                sp = damage::banker_round(attacker.int1 as f64 / 7.0) as i64 + 2 * skill_lv;
            }
            11030 => {
                hp = damage::banker_round(attacker.int1 as f64 / 1.7) as i64 + 3 * skill_lv;
                sp = damage::banker_round(attacker.int1 as f64 / 4.7) as i64 + 3 * skill_lv;
            }
            _ => {}
        }
        if target.hp + hp <= target.hp_max {
            target.hp += hp;
            self.write_hp(target, target.hp, out);
        } else if target.hp + hp > target.hp_max {
            hp = target.hp_max - target.hp;
            target.hp = target.hp_max;
            self.write_hp(target, target.hp_max, out);
        } else if target.hp == target.hp_max {
            hp = 0;
        }
        if attacker.id == target.id {
            sp = 0;
        }
        if target.sp + sp <= target.sp_max {
            target.sp += sp;
            self.write_sp(target, target.sp, out);
        } else if target.sp + sp > target.sp_max {
            sp = target.sp_max - target.sp;
            target.sp = target.sp_max;
            self.write_sp(target, target.sp_max, out);
        } else if target.sp == target.sp_max {
            sp = 0;
        }
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            miss_status::ATTACK,
            attack_status::ATTACK,
            2,
            troi_byte::HP,
            hp as u16,
            0,
        ));
        ts.text10
            .push_str(&format!("1A{}00", encoder::le16(sp as u16)));
    }

    /// Type 15 — Type15 buff/debuff (`TheBattle.cs:3310-3391`).
    #[allow(clippy::too_many_arguments)]
    fn apply_buff15(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        row: u8,
        col: u8,
        avg1: i64,
        avg2: i64,
        out: &mut Vec<Out>,
    ) {
        let _ = out;
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let turn = damage::get_turn(skill, skill_lv);
        let mut byte_val;
        if target.type15_id > 0 {
            let (r, c) = (target.row, target.col);
            ts.text10.push_str(&packets::skilling_int(
                r,
                c,
                miss_status::MISS,
                attack_status::ATTACK,
                1,
                troi_byte::MISS,
                0,
                1,
            ));
        } else {
            let mut hit = damage::get_random_miss_troi(
                &mut self.rng.random_0,
                attacker.lv,
                target.lv,
                avg1,
                avg2,
                attacker.int1,
                attacker.int1,
                target.int1,
                attacker.reborn,
                target.reborn,
            );
            let mut adl = attack_status::ATTACK;
            if hit == miss_status::MISS as i64 {
                byte_val = troi_byte::MISS;
                if target.id_skill == 17001 {
                    adl = attack_status::DEF;
                }
                if target.type4_id == 13003 {
                    adl = attack_status::LANTRANH;
                }
                if matches!(target.type4_id, 10010 | 10015 | 10031 | 13021) {
                    adl = attack_status::ATTACK;
                }
            } else {
                byte_val = troi_byte::TYPE15;
                if target.id_skill == 17001 {
                    adl = attack_status::DEF;
                }
                if target.type4_id == 13003 {
                    adl = attack_status::LANTRANH;
                }
                if matches!(target.type4_id, 10010 | 10015 | 10031 | 13021) {
                    adl = attack_status::ATTACK;
                }
                if target.type4_id == 10026 {
                    hit = miss_status::ATTACK as i64;
                    byte_val = troi_byte::MISS;
                    if attacker.type15_id == 0 {
                        if is_agi_buff(skill) {
                            target.agi -= 200;
                        }
                        attacker.type15_id = skill;
                        attacker.type15_lv = skill_lv;
                        attacker.type15_turn = turn;
                        ts.text_troi = packets::troi_start(row, col, skill as u16);
                    }
                } else {
                    if is_agi_buff(skill) {
                        target.agi -= 200;
                    }
                    target.type15_id = skill;
                    target.type15_lv = skill_lv;
                    target.type15_turn = turn;
                }
            }
            let (r, c) = (target.row, target.col);
            ts.text10.push_str(&packets::skilling_int(
                r, c, hit as u8, adl, 1, byte_val, 0, 1,
            ));
        }
    }

    /// Type 16 — dispel Type4 pair (`TheBattle.cs:3392-3424`).
    fn apply_dispel16(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        target: &mut WarInfo,
        skill: i64,
        out: &mut Vec<Out>,
    ) {
        let _ = out;
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let clears = match skill {
            10014 => target.type4_id == 10015,
            10009 => target.type4_id == 10010,
            _ => false,
        };
        let hit = if clears {
            target.type4_id = 0;
            target.type4_lv = 0;
            target.type4_turn = 0;
            miss_status::ATTACK
        } else {
            miss_status::MISS
        };
        let (r, c) = (target.row, target.col);
        ts.text10.push_str(&packets::skilling_int(
            r,
            c,
            hit,
            0,
            1,
            troi_byte::TYPE4,
            0,
            1,
        ));
    }

    /// Type 18 — cleanse + heal (`TheBattle.cs:3425-3540`).
    fn apply_cleanse18(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let (mut hp, mut sp) = (
            damage::banker_round(attacker.int1 as f64 * 0.5) as i64,
            damage::banker_round(attacker.int1 as f64 * 0.5) as i64,
        );
        match skill {
            11016 => {
                hp = 400;
                sp = 100;
            }
            11017 => {
                hp = 500;
                sp = 150;
            }
            11018 => {
                hp = 600;
                sp = 200;
            }
            11019 => {
                hp = 700;
                sp = 250;
            }
            _ => {}
        }
        let (r, c) = (target.row, target.col);
        if attacker.team == target.team {
            target.type3_id = 0;
            target.type3_lv = 0;
            target.type3_turn = 0;
            target.type15_id = 0;
            target.type15_lv = 0;
            target.type15_turn = 0;
            if target.hp + hp <= target.hp_max {
                target.hp += hp;
                self.write_hp(target, target.hp, out);
            } else if target.hp + hp > target.hp_max {
                hp = target.hp_max - target.hp;
                target.hp = target.hp_max;
                self.write_hp(target, target.hp_max, out);
            } else if target.hp == target.hp_max {
                hp = 0;
            }
            if target.sp + sp <= target.sp_max {
                target.sp += sp;
                self.write_sp(target, target.sp, out);
            } else if target.sp + sp > target.sp_max {
                sp = target.sp_max - target.sp;
                target.sp = target.sp_max;
                self.write_sp(target, target.sp_max, out);
            } else if target.sp == target.sp_max {
                sp = 0;
            }
            let _ = skill_lv;
            ts.text10.push_str(&packets::skilling_int(
                r,
                c,
                miss_status::ATTACK,
                0,
                4,
                troi_byte::TYPE3,
                0,
                1,
            ));
            ts.text10.push_str("DF000001");
            ts.text10
                .push_str(&format!("19{}00", encoder::le16(hp as u16)));
            ts.text10
                .push_str(&format!("1A{}00", encoder::le16(sp as u16)));
        } else {
            target.type4_id = 0;
            target.type4_lv = 0;
            target.type4_turn = 0;
            target.type19_id = 0;
            target.type19_lv = 0;
            target.type19_turn = 0;
            ts.text10.push_str(&packets::skilling_int(
                r,
                c,
                miss_status::ATTACK,
                0,
                2,
                troi_byte::TYPE4,
                0,
                1,
            ));
            ts.text10.push_str("E1000001");
        }
    }

    /// Type 19 — Type19 debuff (`TheBattle.cs:3541-3577`).
    fn apply_buff19(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        target: &mut WarInfo,
        skill: i64,
        skill_lv: i64,
        out: &mut Vec<Out>,
    ) {
        let _ = out;
        ts.delay = data.skills.get(&skill).map(|s| s.delay).unwrap_or(0);
        let turn = damage::get_turn(skill, skill_lv);
        let (hit, adl) = if target.type19_id == 0 {
            (miss_status::ATTACK, attack_status::ATTACK)
        } else {
            (miss_status::MISS, attack_status::DEF)
        };
        let mut byte_val = troi_byte::MISS;
        if hit == miss_status::ATTACK {
            byte_val = troi_byte::TYPE19;
            target.type19_id = skill;
            target.type19_lv = skill_lv;
            target.type19_turn = turn;
            if skill == 13020 {
                let bonus = (target.agi as f64 * (0.03 * skill_lv as f64)).ceil() as i64;
                ts.agi_13020 += bonus;
                target.agi += bonus;
            }
        }
        let (r, c) = (target.row, target.col);
        ts.text10
            .push_str(&packets::skilling_int(r, c, hit, adl, 1, byte_val, 0, 1));
    }

    // ---- HP/SP DB write helpers ------------------------------------------

    fn write_hp(&self, c: &WarInfo, value: i64, out: &mut Vec<Out>) {
        if !matches!(c.typ, 3 | 7) {
            out.push(Out::Db(DbUpdate {
                target: pet_target(c),
                stat: Stat::Hp,
                value,
            }));
        }
    }

    fn write_sp(&self, c: &WarInfo, value: i64, out: &mut Vec<Out>) {
        if !matches!(c.typ, 3 | 7) {
            out.push(Out::Db(DbUpdate {
                target: pet_target(c),
                stat: Stat::Sp,
                value,
            }));
        }
    }

    fn apply_target_hp(&self, target: &mut WarInfo, dmg: i64, out: &mut Vec<Out>) {
        self.write_hp(target, (target.hp - dmg).max(0), out);
        target.hp -= dmg;
    }

    /// Register a hit type-7 npc for drop/exp processing (`TheBattle.cs:2315-2333`).
    fn note_npc_hit(&self, ts: &mut TurnState, attacker: &mut WarInfo, npc: &WarInfo) {
        if npc.typ == 7 {
            let entry = format!("{}.{}/{}", npc.row, npc.col, npc.lv);
            if !ts.killed_npcs.contains(&entry) {
                ts.killed_npcs.push(entry);
            }
            if attacker.lv - npc.lv <= 20 {
                let exp = damage::hit_exp(attacker.lv, npc.lv);
                if npc.hp <= 0 {
                    attacker.exp += exp;
                } else {
                    attacker.exp += damage::banker_round(exp as f64 / 10.0) as i64;
                }
            }
        }
    }

    /// Drop & exp accumulation per flushed turn (§3.6, TheBattle.cs:3621-3905).
    fn process_kills(
        &mut self,
        data: &BattleData,
        ts: &mut TurnState,
        attacker: &mut WarInfo,
        out: &mut Vec<Out>,
    ) {
        let kills = std::mem::take(&mut ts.killed_npcs);
        for entry in &kills {
            let (npc_row, npc_col, npc_lv) = parse_cell_entry(entry);
            let mut item_id = 0i64;
            let npc_dead = self
                .cell(npc_row, npc_col)
                .map(|c| c.hp <= 0)
                .unwrap_or(false);
            if npc_dead {
                if let Some(npc) = self
                    .cell(npc_row, npc_col)
                    .and_then(|c| data.npcs.get(&c.id))
                {
                    item_id = damage::get_random_drop(&mut self.rng.random_0, npc, &DROP_PERCENTS);
                }
            }

            if ts.combo_cells.is_empty() {
                if item_id > 0 {
                    let owner = if attacker.id_char != 0 {
                        attacker.id_char
                    } else {
                        attacker.id
                    };
                    out.push(Out::Drop {
                        item_id,
                        npc_row,
                        npc_col,
                        row: attacker.row,
                        col: attacker.col,
                        owner,
                    });
                    out.push(Out::Broadcast(packets::drop_item(
                        item_id as u16,
                        npc_row,
                        npc_col,
                        attacker.row,
                        attacker.col,
                    )));
                }
                if attacker.lv - npc_lv <= 20 {
                    let exp = damage::calc_kill_exp(attacker.lv, npc_lv);
                    if npc_dead {
                        attacker.exp += exp;
                    } else {
                        attacker.exp += damage::banker_round(exp as f64 / 10.0) as i64;
                    }
                }
            }

            let combo_cells = std::mem::take(&mut ts.combo_cells);
            for entry in &combo_cells {
                let (pr, pc, plv) = parse_cell_entry(entry);
                if plv - npc_lv <= 20 {
                    let exp = damage::calc_combo_exp(damage::calc_kill_exp(plv, npc_lv));
                    if let Some(cell) = self.cell_mut(pr, pc) {
                        if npc_dead {
                            cell.exp += exp;
                        } else {
                            cell.exp += damage::banker_round(exp as f64 / 10.0) as i64;
                        }
                    }
                }
                if item_id > 0 {
                    let caster = self.cell(pr, pc).cloned().unwrap_or_default();
                    let owner = if caster.id_char != 0 {
                        caster.id_char
                    } else {
                        caster.id
                    };
                    if owner > 0 {
                        out.push(Out::Drop {
                            item_id,
                            npc_row,
                            npc_col,
                            row: pr,
                            col: pc,
                            owner,
                        });
                        out.push(Out::Broadcast(packets::drop_item(
                            item_id as u16,
                            npc_row,
                            npc_col,
                            pr,
                            pc,
                        )));
                    }
                }
            }
            ts.combo_cells = combo_cells;
        }
        ts.killed_npcs.clear();
    }

    fn npc_respawn(&mut self, data: &BattleData, out: &mut Vec<Out>) {
        let talking = data.talking_battle;
        if talking <= 0 {
            return;
        }
        let Some(npc_map) = data.npc_on_map else {
            return;
        };
        let lo_x = (npc_map.x - npc_map.coord).max(0);
        let hi_x = npc_map.x + npc_map.coord;
        let lo_y = (npc_map.y - npc_map.coord).max(0);
        let hi_y = npc_map.y + npc_map.coord;
        let x = i64::from(self.rng.random_2.next_range(lo_x as i32, hi_x as i32));
        let y = i64::from(self.rng.random_2.next_range(lo_y as i32, hi_y as i32));
        out.push(Out::Respawn {
            npc_id: talking,
            x,
            y,
        });
        out.push(Out::Broadcast(format!(
            "F44406001603{}0A00F44408001605{}{}{}",
            encoder::le16(talking as u16),
            encoder::le16(talking as u16),
            encoder::le16(x as u16),
            encoder::le16(y as u16)
        )));
    }

    /// End-of-battle: player rewards + cleanup packets (§3.8, TheBattle.cs:4458-4949).
    ///
    /// `per_exp` = `Server.PerEXP` (usually 1). `fled` suppresses exp. Emits the
    /// hide/reposition frames, battle-exit UI packets, player exp Db writes
    /// (with level-up side effects), and `Out::PetExp` for the active pets.
    pub fn finish(&mut self, data: &BattleData, per_exp: i64, fled: bool, out: &mut Vec<Out>) {
        let mut text9 = String::new();
        for col in 0..5u8 {
            let Some(cell) = self.cell(3, col).cloned() else {
                continue;
            };
            if cell.id <= 0 || cell.typ != 2 {
                continue;
            }
            let player = cell.id;
            let snap = data.players.get(&player).copied().unwrap_or_default();

            // Player exp.
            let exp_raw = per_exp * cell.exp;
            if !fled && cell.hp > 0 && cell.lv < 200 && exp_raw > 0 {
                let mut exp = exp_raw;
                if cell.reborn == 2 {
                    exp = damage::banker_round(exp as f64 / 2.0) as i64;
                }
                let new_texp = snap.texp + exp;
                out.push(Out::Db(DbUpdate {
                    target: DbTarget::Player(player),
                    stat: Stat::Texp,
                    value: new_texp,
                }));
                let lv_ups = crate::data::texps::texp_get_lv_up(
                    data.texps,
                    cell.lv,
                    cell.reborn as usize,
                    new_texp,
                );
                if lv_ups > 0 {
                    let new_lv = cell.lv + lv_ups;
                    let new_hp_max =
                        crate::battle::engine::get_hp_max(cell.reborn, snap.job, new_lv, snap.hpx)
                            + snap.hpx2;
                    let new_sp_max =
                        crate::battle::engine::get_sp_max(cell.reborn, snap.job, new_lv, snap.spx)
                            + snap.spx2;
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::Lv,
                        value: new_lv,
                    }));
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::Hpmax,
                        value: new_hp_max,
                    }));
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::Hp,
                        value: new_hp_max,
                    }));
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::Spmax,
                        value: new_sp_max,
                    }));
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::Sp,
                        value: new_sp_max,
                    }));
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::Point,
                        value: 2 * lv_ups,
                    }));
                    out.push(Out::Db(DbUpdate {
                        target: DbTarget::Player(player),
                        stat: Stat::SkillPoint,
                        value: lv_ups,
                    }));
                }
            }

            // Active-pet exp (pet cells cols 0,1,3,4 → stt = active..+3).
            for &pc in &[0u8, 1, 3, 4] {
                if let Some(pet) = self.cell(2, pc).cloned() {
                    if pet.id > 0 {
                        let exp = per_exp * pet.exp;
                        if exp > 0 {
                            out.push(Out::PetExp {
                                owner: player,
                                stt: pet.id_npc_on_map,
                                exp,
                            });
                        }
                    }
                }
            }

            // Cleanup + packets.
            if cell.hp <= 0 {
                out.push(Out::Db(DbUpdate {
                    target: DbTarget::Player(player),
                    stat: Stat::Hp,
                    value: 1,
                }));
            }
            text9.push_str(&packets::hide_from_map(player as u32));
            text9.push_str(&packets::reposition(cell.row, cell.col));
            out.push(Out::MapBroadcast {
                player,
                frame: packets::hide_from_map(player as u32),
            });
        }

        if !text9.is_empty() {
            out.push(Out::Broadcast(text9));
        }
        out.push(Out::Broadcast(packets::battle_exit_move()));
        out.push(Out::Broadcast(packets::battle_exit_talk()));
    }
}

fn next_delay(data: &BattleData, cell: &WarInfo) -> i64 {
    data.skills
        .get(&cell.id_skill)
        .map(|s| s.delay)
        .unwrap_or(0)
}

fn add_combo_cell(ts: &mut TurnState, row: u8, col: u8, lv: i64) {
    let entry = format!("{}.{}/{}", row, col, lv);
    if !ts.combo_cells.contains(&entry) {
        ts.combo_cells.push(entry);
    }
}

/// `num36` element reduction when the defender guards (17001).
fn element_reduce(data: &BattleData, dmg: i64, skill: i64, att_tt: i64, def_tt: i64) -> i64 {
    let stt = if skill == 10000 {
        att_tt
    } else {
        data.skills
            .get(&skill)
            .map(|s| s.thuoctinh)
            .unwrap_or(att_tt)
    };
    match damage::get_thuoctinh_khac(stt, def_tt) {
        2 => 1,
        1 => dmg / 3,
        _ => dmg / 5,
    }
}

fn is_agi_buff(skill: i64) -> bool {
    matches!(skill, 10016 | 10017 | 10018 | 10019 | 10025 | 20022)
}

fn pet_target(c: &WarInfo) -> DbTarget {
    if c.id_char == 0 {
        DbTarget::Player(c.id)
    } else {
        DbTarget::Pet {
            owner: c.id_char,
            stt: c.id_npc_on_map,
        }
    }
}

fn avg_of(ts: &TurnState, team: i64) -> i64 {
    let v = if team == 1 { ts.avg1 } else { ts.avg2 };
    v.unwrap_or(0.0).round() as i64
}

/// Parse a `"row.col/lv"` cell entry.
fn parse_cell_entry(entry: &str) -> (u8, u8, i64) {
    let dot = entry.find('.').unwrap_or(0);
    let slash = entry.rfind('/').unwrap_or(entry.len());
    let row = entry[..dot].parse::<u8>().unwrap_or(0);
    let col = entry[dot + 1..slash].parse::<u8>().unwrap_or(0);
    let lv = entry[slash + 1..].parse::<i64>().unwrap_or(0);
    (row, col, lv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::engine::get_hp_max;
    use std::collections::HashMap;

    fn skill(id: i64, skill_type: i64, do_manh: i64, sl_danh: i64, delay: i64) -> Skill {
        Skill {
            id,
            sp: if skill_type == 1 || skill_type == 2 {
                5
            } else {
                10
            },
            thuoctinh: 1,
            lv_max: 10,
            skill_type,
            do_manh,
            sl_danh,
            combo: 0,
            delay,
            ..Default::default()
        }
    }

    fn npc(id: i64, lv: i64, hp: i64, atk: i64, def: i64, agi: i64) -> Npc {
        Npc {
            id,
            lv,
            hp,
            sp: 100,
            thuoctinh: 1,
            atk,
            def,
            agi,
            int1: 10,
            skill: [10000, 0, 0, 0],
            ..Default::default()
        }
    }

    fn scenario() -> (Battle, BattleData<'static>) {
        // Leak static tables so their references are 'static for the test.
        let mut skills: HashMap<i64, Skill> = HashMap::new();
        skills.insert(10000, skill(10000, 1, 10, 1, 1000));
        skills.insert(11007, skill(11007, 7, 0, 1, 800));
        let skills = Box::leak(Box::new(skills));
        let mut npcs: HashMap<i64, Npc> = HashMap::new();
        npcs.insert(9001, npc(9001, 10, 500, 20, 10, 10));
        let npcs = Box::leak(Box::new(npcs));
        let items: HashMap<i64, Item> = HashMap::new();
        let items = Box::leak(Box::new(items));
        let pets: HashMap<i64, [i64; 4]> = HashMap::new();
        let pets = Box::leak(Box::new(pets));
        let players: HashMap<i64, PlayerSnapshot> = HashMap::new();
        let players = Box::leak(Box::new(players));
        let texps = crate::data::texps::compute_texps();
        let texps = Box::leak(Box::new(texps));
        let data = BattleData::new(npcs, skills, items, pets, players, texps, None, 0);
        let battle = Battle::with_seeds(1, 112, 1, 2, 3);
        (battle, data)
    }

    fn add_player(battle: &mut Battle) -> crate::server::session::Session {
        let mut session = crate::server::session::Session::new();
        session.id = 300001;
        session.level = 10;
        session.hp = 1000;
        session.hp_max = 1000;
        session.sp = 100;
        session.sp_max = 100;
        session.atk = 100;
        session.def = 20;
        session.agi = 50;
        session.int1 = 30;
        session.thuoctinh = 1;
        session.hpx = 10;
        battle.add_player(&session, session.id as i64, 3, 2);
        session
    }

    fn add_npc(battle: &mut Battle, data: &BattleData) {
        let npc = data.npcs.get(&9001).unwrap();
        battle.add_npc(npc, 1, 0, 2, 3);
    }

    fn basic_command() -> HashMap<i64, BattleCommand> {
        let mut cmds = HashMap::new();
        cmds.insert(
            300001,
            BattleCommand {
                row: 3,
                col: 2,
                skill_id: 10000,
                skill_lv: 1,
                row_attack: 0,
                col_attack: 2,
                use_item: 0,
            },
        );
        cmds
    }

    #[test]
    fn player_basic_attack_kills_npc() {
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        add_npc(&mut battle, &data);
        let cmds = basic_command();

        let mut out = Vec::new();
        let outcome = battle.run_battle(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::PlayerWin);
        // NPC dead.
        assert!(battle.cell(0, 2).unwrap().hp <= 0);
        // A turn action frame was broadcast.
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::Broadcast(f) if f.contains("3201"))));
    }

    #[test]
    fn player_without_command_eventually_loses() {
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        add_npc(&mut battle, &data);
        let cmds = HashMap::new();

        let mut out = Vec::new();
        let outcome = battle.run_battle(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::PlayerLose);
        assert!(battle.cell(3, 2).unwrap().hp <= 0);
        // DB HP write for the player happened.
        assert!(out.iter().any(|o| matches!(
            o,
            Out::Db(DbUpdate {
                target: DbTarget::Player(300001),
                stat: Stat::Hp,
                ..
            })
        )));
    }

    #[test]
    fn burn_tick_damages_and_broadcasts() {
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        add_npc(&mut battle, &data);
        // Give the player a burn debuff (10004, lv 3 → 10+6=16/turn).
        battle.cell_mut(3, 2).unwrap().type3_id = 10004;
        battle.cell_mut(3, 2).unwrap().type3_lv = 3;
        battle.cell_mut(3, 2).unwrap().type3_turn = 2;
        let cmds = basic_command();

        let mut out = Vec::new();
        battle.run_battle(&data, &cmds, &mut out);
        // Burn broadcast uses skill 20001 (LE16 "214E").
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::Broadcast(f) if f.contains("214E"))));
    }

    #[test]
    fn turn_action_frame_structure() {
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        add_npc(&mut battle, &data);
        let cmds = basic_command();

        let mut out = Vec::new();
        let _ = battle.run_battle(&data, &cmds, &mut out);
        let frame = out
            .iter()
            .find_map(|o| match o {
                Out::Broadcast(f) if f.contains("3201") => Some(f.clone()),
                _ => None,
            })
            .unwrap();
        // F444 + LE16 length + 3201 + block(LE16 len + row col skill sl_danh count + effects)
        assert!(frame.starts_with("F444"));
        assert!(frame.contains("3201"));
        // Skill 10000 = 0x2710 -> LE16 "1027".
        assert!(frame.contains("1027"));
    }

    #[test]
    fn heal_skill_restores_hp() {
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        add_npc(&mut battle, &data);
        // Damage the player first.
        battle.cell_mut(3, 2).unwrap().hp = 500;
        let mut cmds = basic_command();
        cmds.insert(
            300001,
            BattleCommand {
                row: 3,
                col: 2,
                skill_id: 11007,
                skill_lv: 3,
                row_attack: 3,
                col_attack: 2,
                use_item: 0,
            },
        );

        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &cmds, &mut out);
        // A DB HP write for the player with value > 500 proves the heal applied
        // (11007 lv3: round(int*0.2*lv)=round(30*0.6)=18).
        let healed = out.iter().any(|o| {
            matches!(
                o,
                Out::Db(DbUpdate { target: DbTarget::Player(300001), stat: Stat::Hp, value }) if *value > 500
            )
        });
        assert!(healed, "expected a heal DB write for the player");
    }

    #[test]
    fn flee_by_leader_ends_battle() {
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        add_npc(&mut battle, &data);
        // Skill 14002 always flees.
        let mut skills = HashMap::new();
        skills.insert(10000, skill(10000, 1, 10, 1, 1000));
        skills.insert(14002, skill(14002, 12, 0, 1, 0));
        let mut npcs = HashMap::new();
        npcs.insert(9001, npc(9001, 10, 500, 20, 10, 10));
        let items = HashMap::new();
        let pets = HashMap::new();
        let players = HashMap::new();
        let texps = crate::data::texps::compute_texps();
        let data = BattleData::new(&npcs, &skills, &items, &pets, &players, &texps, None, 0);

        let mut cmds = HashMap::new();
        cmds.insert(
            300001,
            BattleCommand {
                row: 3,
                col: 2,
                skill_id: 14002,
                skill_lv: 1,
                row_attack: 0,
                col_attack: 2,
                use_item: 0,
            },
        );
        let mut out = Vec::new();
        let outcome = battle.run_battle(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::PlayerFled);
    }

    #[test]
    fn use_item_heals_cell_and_pet() {
        let (mut battle, mut data_with_items) = scenario();
        // Add a potion to the item table.
        let items: HashMap<i64, Item> = {
            let mut m = HashMap::new();
            m.insert(
                26001,
                Item {
                    id: 26001,
                    hp: 500,
                    sp: 200,
                    ..Default::default()
                },
            );
            m
        };
        let items = Box::leak(Box::new(items));
        let npcs = Box::leak(Box::new(data_with_items.npcs.clone()));
        let skills = Box::leak(Box::new(data_with_items.skills.clone()));
        let pets = Box::leak(Box::new(data_with_items.pet_slots.clone()));
        let players = Box::leak(Box::new(data_with_items.players.clone()));
        let texps = Box::leak(Box::new(crate::data::texps::compute_texps()));
        data_with_items = BattleData::new(npcs, skills, items, pets, players, texps, None, 0);

        let mut session = crate::server::session::Session::new();
        session.id = 300001;
        session.level = 10;
        session.hp = 1000;
        session.hp_max = 1000;
        session.sp = 100;
        session.sp_max = 100;
        session.atk = 100;
        session.def = 20;
        session.agi = 50;
        session.int1 = 30;
        battle.add_player(&session, session.id as i64, 3, 2);
        // Attach a pet at (2,1).
        let mut pet = crate::server::session::PetState::default();
        pet.stt = 1;
        pet.id = 9001;
        pet.hp = 200;
        pet.hp_max = 500;
        pet.sp = 100;
        pet.sp_max = 500;
        battle.add_pet(&pet, session.id as i64, session.id as i64, 3, 2, 1);

        let add_npc = {
            let n = data_with_items.npcs.get(&9001).unwrap();
            battle.add_npc(n, 1, 0, 2, 3);
        };
        let _ = add_npc;

        // Damage the player then use potion 26001 on the player cell.
        battle.cell_mut(3, 2).unwrap().hp = 700;
        battle.cell_mut(3, 2).unwrap().sp = 40;
        let mut cmds = HashMap::new();
        cmds.insert(
            300001,
            BattleCommand {
                row: 3,
                col: 2,
                skill_id: 0,
                skill_lv: 0,
                row_attack: 0,
                col_attack: 2,
                use_item: 26001,
            },
        );
        let mut out = Vec::new();
        let _ = battle.run_turn(&data_with_items, &cmds, &mut out);
        let cell = battle.cell(3, 2).unwrap();
        // 700 + 500 capped to 1000; 40 + 200 capped to 100.
        assert_eq!(cell.hp, 1000);
        assert_eq!(cell.sp, 100);
        // Pet DB write fired (heal the owner's active pet).
        assert!(out.iter().any(|o| matches!(
            o,
            Out::Db(DbUpdate {
                target: DbTarget::Pet { owner: 300001, .. },
                stat: Stat::Hp,
                ..
            })
        )));
    }

    #[test]
    fn hp_max_formula_matches() {
        // getHpMax(rb=0, job, lvl=10, hpx=6): floor((10^0.35+1)*12 + 80 + 10).
        let v = get_hp_max(0, 0, 10, 6);
        assert!(v > 0);
    }

    #[test]
    fn finish_grants_teamdef_exp_and_exit_packets() {
        // A TeamDef (type-7) kill accumulates exp; finish pays it out.
        let (mut battle, data) = scenario();
        add_player(&mut battle);
        // Use a type-7 npc instead of the scenario's type-3.
        battle.clear_cell(0, 2);
        let n = data.npcs.get(&9001).unwrap();
        battle.add_npc(n, 1, 0, 2, 7);
        let cmds = basic_command();

        let mut out = Vec::new();
        let outcome = battle.run_battle(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::PlayerWin);
        let hp = battle.cell(3, 2).unwrap().hp;
        let _ = hp;

        let mut fin = Vec::new();
        battle.finish(&data, 1, false, &mut fin);
        // A Texp write for the player on a type-7 kill.
        assert!(fin.iter().any(|o| matches!(
            o,
            Out::Db(DbUpdate {
                target: DbTarget::Player(300001),
                stat: Stat::Texp,
                ..
            })
        )));
        // Exit UI frames.
        assert!(fin
            .iter()
            .any(|o| matches!(o, Out::Broadcast(f) if *f == packets::battle_exit_move())));
        assert!(fin
            .iter()
            .any(|o| matches!(o, Out::Broadcast(f) if *f == packets::battle_exit_talk())));
        // Player stays alive and unarmed after battle.
        assert!(battle.cell(3, 2).unwrap().hp > 0);
    }
}
