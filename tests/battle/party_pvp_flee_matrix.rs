//! Ticket 28 — 4-role Party/PvP Flee matrix, Zombie Cell fix, and
//! `battle_participants` synchronization.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use ts_dream::battle::construction::{Battle, BattleKind};
use ts_dream::battle::packets;
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, Out, Outcome,
};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{EveFightData, EveFightEnemy, Npc, Skill};
use ts_dream::server::session::{lock_online_sessions, PetState, Session};

fn make_skills() -> HashMap<i64, Skill> {
    let mut skills = HashMap::new();
    skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Attack".to_string(),
            sl_danh: 1,
            skill_type: 1,
            delay: 5,
            ..Default::default()
        },
    );
    skills.insert(
        18001,
        Skill {
            id: 18001,
            name: "Chay Tron".to_string(),
            sl_danh: 1,
            skill_type: 12,
            delay: 5,
            ..Default::default()
        },
    );
    skills.insert(
        18002,
        Skill {
            id: 18002,
            name: "Chay Tron That Bai".to_string(),
            sl_danh: 1,
            skill_type: 12,
            delay: 5,
            ..Default::default()
        },
    );
    skills
}

fn make_flee_cmd(row: u8, col: u8, target_row: u8, target_col: u8) -> BattleCommand {
    BattleCommand {
        row,
        col,
        target_row,
        target_col,
        action: BattleAction::CastSkill {
            skill_id: 18001,
            skill_lv: 1,
        },
    }
}

fn make_attack_cmd(row: u8, col: u8, target_row: u8, target_col: u8) -> BattleCommand {
    BattleCommand {
        row,
        col,
        target_row,
        target_col,
        action: BattleAction::NormalAttack,
    }
}

fn setup_session(id: u32, level: u8, agi: u16, hp: u16) -> Session {
    Session {
        id,
        level,
        hp,
        hp_max: hp.max(100),
        sp: 100,
        sp_max: 100,
        agi,
        atk: 10,
        def: 10,
        map_id: 12001,
        ..Default::default()
    }
}

fn setup_pet(id: u16, level: u8, agi: u16, hp: u16) -> PetState {
    PetState {
        stt: 1,
        id,
        level,
        hp,
        hp_max: hp.max(100),
        sp: 100,
        sp_max: 100,
        agi,
        atk: 10,
        def: 10,
        ..Default::default()
    }
}

#[test]
fn pve_and_pvp_leader_or_leader_pet_flee_ends_battle_with_player_fled() {
    let skills = make_skills();
    let binary_skills = HashMap::new();
    let npcs = HashMap::new();
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 12001,
    };

    // 1. PvE Leader (character) flees -> Outcome::PlayerFled.
    {
        let mut battle = Battle::with_seeds(9, 112, 1, 1, 1);
        let leader = setup_session(1001, 200, 300, 500);
        let member = setup_session(1002, 50, 10, 500);
        battle.add_player(&leader, 1001, 3, 2);
        battle.add_player(&member, 1001, 3, 1);
        {
            let e = battle.cell_mut(0, 2).unwrap();
            e.typ = 3;
            e.id = 41001;
            e.team = 2;
            e.lv = 1;
            e.hp = 500;
            e.hp_max = 500;
        }
        let mut cmds = BattleCommands::default();
        cmds.insert((3, 2), make_flee_cmd(3, 2, 0, 2));
        cmds.insert((3, 1), make_attack_cmd(3, 1, 0, 2));
        let mut out = Vec::new();
        assert_eq!(
            battle.run_turn(&data, &cmds, &mut out),
            Outcome::PlayerFled,
            "PvE leader flee must end the battle with PlayerFled"
        );
    }

    // 2. PvE Leader's Pet flees -> Outcome::PlayerFled.
    {
        let mut battle = Battle::with_seeds(10, 112, 1, 1, 1);
        let leader = setup_session(1001, 200, 10, 500);
        let leader_pet = setup_pet(41001, 200, 300, 500);
        battle.add_player(&leader, 1001, 3, 2);
        battle.add_pet(&leader_pet, 1001, 1001, 3, 2, 1);
        {
            let e = battle.cell_mut(0, 2).unwrap();
            e.typ = 3;
            e.id = 41001;
            e.team = 2;
            e.lv = 1;
            e.hp = 500;
            e.hp_max = 500;
        }
        let mut cmds = BattleCommands::default();
        cmds.insert((2, 2), make_flee_cmd(2, 2, 0, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(
            outcome,
            Outcome::PlayerFled,
            "PvE leader's pet flee must end the battle with PlayerFled"
        );
    }

    // 2. PvP Team 1 Leader flees -> Outcome::PlayerFled.
    {
        let mut battle = Battle::with_seeds(11, 112, 1, 1, 1).with_kind(BattleKind::Pk);
        let l1 = setup_session(1001, 200, 300, 500);
        let m1 = setup_session(1002, 50, 10, 500);
        let l2 = setup_session(2001, 1, 10, 500);
        battle.add_player(&l1, 1001, 3, 2);
        battle.add_player(&m1, 1001, 3, 1);
        battle.add_player(&l2, 2001, 0, 2);
        let mut cmds = BattleCommands::default();
        cmds.insert((3, 2), make_flee_cmd(3, 2, 0, 2));
        cmds.insert((3, 1), make_attack_cmd(3, 1, 0, 2));
        cmds.insert((0, 2), make_attack_cmd(0, 2, 3, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(
            outcome,
            Outcome::PlayerFled,
            "PvP Team 1 leader flee must end the battle with PlayerFled"
        );
    }

    // 3. PvP Team 2 Leader's Pet flees -> Outcome::PlayerFled.
    {
        let mut battle = Battle::with_seeds(12, 112, 1, 1, 1).with_kind(BattleKind::Pk);
        let l1 = setup_session(1001, 1, 10, 500);
        let l2 = setup_session(2001, 200, 10, 500);
        let l2_pet = setup_pet(41002, 200, 300, 500);
        let m2 = setup_session(2002, 50, 10, 500);
        battle.add_player(&l1, 1001, 3, 2);
        battle.add_player(&l2, 2001, 0, 2);
        battle.add_pet(&l2_pet, 2001, 2001, 0, 2, 2);
        battle.add_player(&m2, 2001, 0, 1);
        let mut cmds = BattleCommands::default();
        cmds.insert((1, 2), make_flee_cmd(1, 2, 3, 2));
        cmds.insert((0, 2), make_attack_cmd(0, 2, 3, 2));
        cmds.insert((0, 1), make_attack_cmd(0, 1, 3, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(
            outcome,
            Outcome::PlayerFled,
            "PvP Team 2 leader's pet flee must end the battle with PlayerFled"
        );
    }

    // 3b. PvP Team 2 Leader (character) flees -> Outcome::PlayerFled.
    {
        let mut battle = Battle::with_seeds(14, 112, 1, 1, 1).with_kind(BattleKind::Pk);
        let l1 = setup_session(1001, 1, 10, 500);
        let l2 = setup_session(2001, 200, 300, 500);
        let m2 = setup_session(2002, 50, 10, 500);
        battle.add_player(&l1, 1001, 3, 2);
        battle.add_player(&l2, 2001, 0, 2);
        battle.add_player(&m2, 2001, 0, 1);
        let mut cmds = BattleCommands::default();
        cmds.insert((0, 2), make_flee_cmd(0, 2, 3, 2));
        cmds.insert((0, 1), make_attack_cmd(0, 1, 3, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        assert_eq!(
            battle.run_turn(&data, &cmds, &mut out),
            Outcome::PlayerFled,
            "PvP Team 2 leader flee must end the battle with PlayerFled"
        );
    }

    // 4. Edge case: Member's pet whose NPC template ID happens to equal leader_id
    //    must still be treated as a MEMBER's pet (not leader!).
    {
        let mut battle = Battle::with_seeds(13, 112, 1, 1, 1);
        let leader = setup_session(1001, 50, 10, 500);
        let member = setup_session(1002, 200, 10, 500);
        let member_pet = setup_pet(1001, 200, 300, 500); // pet.id == leader_id!
        battle.add_player(&leader, 1001, 3, 2);
        battle.add_player(&member, 1001, 3, 1);
        battle.add_pet(&member_pet, 1002, 1001, 3, 1, 1);
        {
            let e = battle.cell_mut(0, 2).unwrap();
            e.typ = 3;
            e.id = 41001;
            e.team = 2;
            e.lv = 1;
            e.hp = 500;
            e.hp_max = 500;
        }
        let mut cmds = BattleCommands::default();
        cmds.insert((2, 1), make_flee_cmd(2, 1, 0, 2));
        cmds.insert((3, 1), make_attack_cmd(3, 1, 0, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(
            outcome,
            Outcome::Running,
            "Member's pet whose NPC ID equals leader_id must NOT end the battle"
        );
        assert_eq!(battle.cell(3, 1).unwrap().id, 0, "member cell cleared");
        assert_eq!(battle.cell(2, 1).unwrap().id, 0, "member pet cell cleared");
        assert_eq!(battle.cell(3, 2).unwrap().id, 1001, "leader remains");
    }
}

#[test]
fn party_member_or_member_pet_flee_in_pve_and_pvp_extracts_only_owner_cells_without_duplicate_reset_or_respawn() {
    let skills = make_skills();
    let binary_skills = HashMap::new();
    let mut npcs = HashMap::new();
    npcs.insert(
        41001,
        Npc {
            id: 41001,
            name: b"Wolf".to_vec(),
            hp: 500,
            lv: 1,
            ..Default::default()
        },
    );
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 12001,
    };

    // 1. PvE Party Member (with pet) flees.
    {
        let mut battle = Battle::with_seeds(20, 112, 1, 1, 1);
        let leader = setup_session(1001, 50, 10, 500);
        let member = setup_session(1002, 200, 300, 500);
        let member_pet = setup_pet(41002, 200, 20, 500);
        battle.add_player(&leader, 1001, 3, 2);
        battle.add_player(&member, 1001, 3, 1);
        battle.add_pet(&member_pet, 1002, 1001, 3, 1, 1);
        {
            let e = battle.cell_mut(0, 2).unwrap();
            e.typ = 3;
            e.id = 41001;
            e.team = 2;
            e.lv = 1;
            e.hp = 500;
            e.hp_max = 500;
        }

        let mut cmds = BattleCommands::default();
        cmds.insert((3, 1), make_flee_cmd(3, 1, 0, 2));
        cmds.insert((2, 1), make_attack_cmd(2, 1, 0, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::Running);

        // Both member and pet cells are cleared.
        assert_eq!(battle.cell(3, 1).unwrap().id, 0);
        assert_eq!(battle.cell(2, 1).unwrap().id, 0);
        // Leader and enemy remain.
        assert_eq!(battle.cell(3, 2).unwrap().id, 1001);
        assert_eq!(battle.cell(0, 2).unwrap().id, 41001);

        // No Respawn in member flee.
        assert!(
            !out.iter().any(|o| matches!(o, Out::Respawn { .. })),
            "Member flee must not emit Out::Respawn"
        );

        // Check Broadcast frames: acting + reset_cell exactly ONCE per cleared cell + 0B 00.
        let broadcasts: String = out
            .iter()
            .filter_map(|o| match o {
                Out::Broadcast(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        let reset_member = packets::reset_cell(3, 1);
        let reset_pet = packets::reset_cell(2, 1);
        assert_eq!(
            broadcasts.matches(&reset_member).count(),
            1,
            "reset_cell(3, 1) must be emitted exactly once, got: {broadcasts}"
        );
        assert_eq!(
            broadcasts.matches(&reset_pet).count(),
            1,
            "reset_cell(2, 1) must be emitted exactly once, got: {broadcasts}"
        );
        assert!(broadcasts.contains(&packets::acting(3, 1)));
        assert!(broadcasts.contains(&packets::acting(2, 1)));
        assert!(broadcasts.contains(&packets::hide_from_map(1002)));

        // ToPlayer: 05 04 (battle_exit_move) and 14 08 (battle_exit_talk) for 1002.
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::ToPlayer(1002, f) if f == &packets::battle_exit_move())));
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::ToPlayer(1002, f) if f == &packets::battle_exit_talk())));
    }

    // 2. PvP Team 2 Member's Pet flees -> extracts Team 2 Member + Pet, keeps Team 1 & Team 2 Leaders.
    {
        let mut battle = Battle::with_seeds(21, 112, 1, 1, 1).with_kind(BattleKind::Pk);
        let l1 = setup_session(1001, 50, 10, 500);
        let l2 = setup_session(2001, 50, 10, 500);
        let m2 = setup_session(2002, 200, 20, 500);
        let m2_pet = setup_pet(41002, 200, 300, 500);
        battle.add_player(&l1, 1001, 3, 2);
        battle.add_player(&l2, 2001, 0, 2);
        battle.add_player(&m2, 2001, 0, 1);
        battle.add_pet(&m2_pet, 2002, 2001, 0, 1, 2);

        let mut cmds = BattleCommands::default();
        cmds.insert((1, 1), make_flee_cmd(1, 1, 3, 2));
        cmds.insert((0, 1), make_attack_cmd(0, 1, 3, 2));
        cmds.insert((0, 2), make_attack_cmd(0, 2, 3, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(outcome, Outcome::Running);

        assert_eq!(battle.cell(0, 1).unwrap().id, 0, "Team 2 member cleared");
        assert_eq!(battle.cell(1, 1).unwrap().id, 0, "Team 2 pet cleared");
        assert_eq!(battle.cell(0, 2).unwrap().id, 2001, "Team 2 leader stays");
        assert_eq!(battle.cell(3, 2).unwrap().id, 1001, "Team 1 leader stays");

        let broadcasts: String = out
            .iter()
            .filter_map(|o| match o {
                Out::Broadcast(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(broadcasts.matches(&packets::reset_cell(0, 1)).count(), 1);
        assert_eq!(broadcasts.matches(&packets::reset_cell(1, 1)).count(), 1);
        assert!(broadcasts.contains(&packets::hide_from_map(2002)));
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::ToPlayer(2002, f) if f == &packets::battle_exit_move())));
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::ToPlayer(2002, f) if f == &packets::battle_exit_talk())));
    }

    // 3. PvP Team 1 Member flees -> extracts Team 1 Member + Pet, keeps Team 1 & Team 2 Leaders.
    {
        let mut battle = Battle::with_seeds(22, 112, 1, 1, 1).with_kind(BattleKind::Pk);
        let l1 = setup_session(1001, 50, 10, 500);
        let m1 = setup_session(1002, 200, 300, 500);
        let m1_pet = setup_pet(41002, 200, 20, 500);
        let l2 = setup_session(2001, 50, 10, 500);
        battle.add_player(&l1, 1001, 3, 2);
        battle.add_player(&m1, 1001, 3, 1);
        battle.add_pet(&m1_pet, 1002, 1001, 3, 1, 1);
        battle.add_player(&l2, 2001, 0, 2);

        let mut cmds = BattleCommands::default();
        cmds.insert((3, 1), make_flee_cmd(3, 1, 0, 2));
        cmds.insert((2, 1), make_attack_cmd(2, 1, 0, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        cmds.insert((0, 2), make_attack_cmd(0, 2, 3, 2));
        let mut out = Vec::new();
        assert_eq!(battle.run_turn(&data, &cmds, &mut out), Outcome::Running);
        assert_eq!(battle.cell(3, 1).unwrap().id, 0, "Team 1 member cleared");
        assert_eq!(battle.cell(2, 1).unwrap().id, 0, "Team 1 pet cleared");
        assert_eq!(battle.cell(3, 2).unwrap().id, 1001, "Team 1 leader stays");
        assert_eq!(battle.cell(0, 2).unwrap().id, 2001, "Team 2 leader stays");
    }
}

#[test]
fn member_flee_does_not_create_zombie_cell_across_multiple_turns_and_ends_when_last_survivor_flees()
{
    let skills = make_skills();
    let binary_skills = HashMap::new();
    let mut npcs = HashMap::new();
    npcs.insert(
        41001,
        Npc {
            id: 41001,
            name: b"Wolf".to_vec(),
            hp: 5000,
            lv: 1,
            ..Default::default()
        },
    );
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let data = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 12001,
    };

    // Case A: Multi-turn after member flee — no Zombie Cell across turns 1..=3.
    {
        let mut battle = Battle::with_seeds(30, 112, 1, 1, 1);
        let leader = setup_session(1001, 50, 10, 500);
        let member = setup_session(1002, 200, 300, 500);
        let member_pet = setup_pet(41002, 200, 20, 500);
        battle.add_player(&leader, 1001, 3, 2);
        battle.add_player(&member, 1001, 3, 1);
        battle.add_pet(&member_pet, 1002, 1001, 3, 1, 1);
        {
            let e = battle.cell_mut(0, 2).unwrap();
            e.typ = 3;
            e.id = 41001;
            e.team = 2;
            e.lv = 1;
            e.hp = 5000;
            e.hp_max = 5000;
        }

        // Turn 1: member flees.
        let mut cmds1 = BattleCommands::default();
        cmds1.insert((3, 1), make_flee_cmd(3, 1, 0, 2));
        cmds1.insert((2, 1), make_attack_cmd(2, 1, 0, 2));
        cmds1.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
        let mut out = Vec::new();
        assert_eq!(battle.run_turn(&data, &cmds1, &mut out), Outcome::Running);
        assert_eq!(battle.cell(3, 1).unwrap().id, 0);
        assert_eq!(battle.cell(2, 1).unwrap().id, 0);

        // Turns 2 & 3: leader and enemy continue fighting; fled cells remain cleared.
        for turn in 2..=3 {
            let mut cmds = BattleCommands::default();
            cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
            let mut turn_out = Vec::new();
            assert_eq!(battle.run_turn(&data, &cmds, &mut turn_out), Outcome::Running);
            assert_eq!(
                battle.cell(3, 1).unwrap().id,
                0,
                "Zombie cell appeared at (3, 1) on turn {turn}"
            );
            assert_eq!(
                battle.cell(2, 1).unwrap().id,
                0,
                "Zombie cell appeared at (2, 1) on turn {turn}"
            );
        }
    }

    // Case B: PvE last surviving member flees while leader is dead -> Outcome::PlayerLose.
    {
        let mut battle = Battle::with_seeds(31, 112, 1, 1, 1);
        let leader = setup_session(1001, 50, 10, 0);
        let member = setup_session(1002, 200, 300, 500);
        let member_pet = setup_pet(41002, 200, 20, 500);
        battle.add_player(&leader, 1001, 3, 2);
        battle.add_player(&member, 1001, 3, 1);
        battle.add_pet(&member_pet, 1002, 1001, 3, 1, 1);
        {
            let e = battle.cell_mut(0, 2).unwrap();
            e.typ = 3;
            e.id = 41001;
            e.team = 2;
            e.lv = 1;
            e.hp = 500;
            e.hp_max = 500;
        }
        let mut cmds = BattleCommands::default();
        cmds.insert((3, 1), make_flee_cmd(3, 1, 0, 2));
        cmds.insert((2, 1), make_attack_cmd(2, 1, 0, 2));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(
            outcome,
            Outcome::PlayerLose,
            "Last surviving member fleeing while leader is dead must end with PlayerLose"
        );
    }

    // Case C: PvP Team 2 last surviving member flees while Team 2 leader is dead -> Outcome::PlayerWin.
    {
        let mut battle = Battle::with_seeds(32, 112, 1, 1, 1).with_kind(BattleKind::Pk);
        let l1 = setup_session(1001, 50, 10, 500);
        let l2 = setup_session(2001, 50, 10, 0);
        let m2 = setup_session(2002, 200, 300, 500);
        let m2_pet = setup_pet(41002, 200, 20, 500);
        battle.add_player(&l1, 1001, 3, 2);
        battle.add_player(&l2, 2001, 0, 2);
        battle.add_player(&m2, 2001, 0, 1);
        battle.add_pet(&m2_pet, 2002, 2001, 0, 1, 2);

        let mut cmds = BattleCommands::default();
        cmds.insert((0, 1), make_flee_cmd(0, 1, 3, 2));
        cmds.insert((1, 1), make_attack_cmd(1, 1, 3, 2));
        cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 1));
        let mut out = Vec::new();
        let outcome = battle.run_turn(&data, &cmds, &mut out);
        assert_eq!(
            outcome,
            Outcome::PlayerWin,
            "PvP Team 2 last surviving member fleeing while Team 2 leader is dead must end with PlayerWin"
        );
    }
}

#[tokio::test]
async fn service_member_flee_removes_from_participants_skips_34_01_and_battle_ended_does_not_overwrite()
{
    let mut data = GameData::default();
    data.npcs.insert(
        41001,
        Npc {
            id: 41001,
            name: b"Wolf".to_vec(),
            hp: 50,
            sp: 100,
            lv: 1,
            ..Default::default()
        },
    );
    data.skills = make_skills();

    let fight = EveFightData {
        eve_no: 504,
        fight_type: 0,
        link_to_result: 0,
        left_enemies: vec![EveFightEnemy {
            no: 1,
            npc_id: 41001,
            location_pos: 2,
            ai: 0,
        }],
        right_enemies: Vec::new(),
        bk_music: 0,
    };

    let mut service = BattleService::new(Arc::new(data));
    service.set_input_timeout(std::time::Duration::from_millis(60));

    let leader_sess = Arc::new(AsyncRwLock::new(Session {
        id: 7101,
        level: 50,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        atk: 5,
        agi: 10,
        map_id: 12001,
        id_leader: 7101,
        id_mem: [7102, 0, 0, 0],
        ..Default::default()
    }));
    let member_sess = Arc::new(AsyncRwLock::new(Session {
        id: 7102,
        level: 200,
        hp: 500,
        hp_max: 500,
        sp: 100,
        sp_max: 100,
        agi: 300,
        map_id: 12001,
        id_leader: 7101,
        ..Default::default()
    }));

    let _rx_leader = service.register(7101, leader_sess.clone());
    let mut rx_member = service.register(7102, member_sess.clone());
    lock_online_sessions().insert(7101, leader_sess.read().await.clone());
    lock_online_sessions().insert(7102, member_sess.read().await.clone());

    let mut leader_guard = leader_sess.write().await;
    let battle_id = service.start_encounter_battle(&mut leader_guard, &fight, 112);
    assert!(battle_id > 0);
    drop(leader_guard);

    // Wait for Turn 1 prompt, drain member rx, and submit Flee for member + NormalAttack for leader.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    while rx_member.try_recv().is_ok() {}

    let member_snap = member_sess.read().await.clone();
    assert!(service.submit_command(&member_snap, make_flee_cmd(3, 1, 0, 2)));
    let leader_snap = leader_sess.read().await.clone();
    assert!(service.submit_command(&leader_snap, make_attack_cmd(3, 2, 0, 2)));

    // Wait for member to finish fleeing (`battle_id == 0`).
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1000);
    loop {
        if member_sess.read().await.battle_id == 0 {
            break;
        }
        if tokio::time::Instant::now() > deadline {
            panic!("Member did not flee within timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Drain Turn 1 exit frames and stamp a sentinel on `member_sess.last_battle_end_ms`.
    // If `battle_ended` still has member in `battle_participants`, it will overwrite
    // `last_battle_end_ms` (and/or `encounter_steps`).
    while rx_member.try_recv().is_ok() {}
    {
        let mut mem = member_sess.write().await;
        mem.last_battle_end_ms = 42;
        mem.encounter_steps = 7;
    }

    // Wait for Turn 2 to prompt Leader and finish battle (Leader kills 50-HP enemy or times out).
    // Give Turn 2 time to emit `34 01` and finish.
    for _ in 0..3 {
        let lsnap = leader_sess.read().await.clone();
        if lsnap.battle_id == 0 {
            break;
        }
        let _ = service.submit_command(&lsnap, make_attack_cmd(3, 2, 0, 2));
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    // Wait for Leader's battle to end.
    let deadline2 = tokio::time::Instant::now() + std::time::Duration::from_millis(1000);
    loop {
        if leader_sess.read().await.battle_id == 0 {
            break;
        }
        if tokio::time::Instant::now() > deadline2 {
            panic!("Leader battle did not end within timeout");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Member must NOT have received `34 01` (turn prompt) in Turn 2+.
    let mut member_later_frames = Vec::new();
    while let Ok(f) = rx_member.try_recv() {
        member_later_frames.push(f);
    }
    let has_turn_prompt = member_later_frames
        .iter()
        .any(|f| f.contains("3401"));
    assert!(
        !has_turn_prompt,
        "Fled member must NOT receive 34 01 turn prompt in subsequent turns, got: {member_later_frames:?}"
    );

    // `battle_ended` must NOT have overwritten the fled member's session state.
    let mem_after = member_sess.read().await;
    assert_eq!(
        mem_after.last_battle_end_ms, 42,
        "battle_ended must not overwrite fled member's last_battle_end_ms"
    );
    assert_eq!(
        mem_after.encounter_steps, 7,
        "battle_ended must not overwrite fled member's encounter_steps"
    );

    // Also verify `Out::Fled { battle_id, player }` carries `battle_id`.
    let mut battle = Battle::with_seeds(99, 112, 1, 1, 1);
    let l = setup_session(1001, 50, 10, 500);
    let m = setup_session(1002, 200, 300, 500);
    battle.add_player(&l, 1001, 3, 2);
    battle.add_player(&m, 1001, 3, 1);
    {
        let e = battle.cell_mut(0, 2).unwrap();
        e.typ = 3;
        e.id = 41001;
        e.team = 2;
        e.lv = 1;
        e.hp = 500;
        e.hp_max = 500;
    }
    let skills = make_skills();
    let binary_skills = HashMap::new();
    let npcs = HashMap::new();
    let items = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let bdata = BattleData {
        npcs: &npcs,
        skills: &skills,
        binary_skills: Some(&binary_skills),
        items: &items,
        pet_slots: &pet_slots,
        players: &players,
        player_skills: None,
        texps: &[],
        world: None,
        talking_battle: 0,
        map_id: 12001,
    };
    let mut cmds = BattleCommands::default();
    cmds.insert((3, 1), make_flee_cmd(3, 1, 0, 2));
    cmds.insert((3, 2), make_attack_cmd(3, 2, 0, 2));
    let mut out = Vec::new();
    battle.run_turn(&bdata, &cmds, &mut out);
    assert!(
        out.iter()
            .any(|o| matches!(o, Out::Fled { battle_id: 99, player: 1002 })),
        "Out::Fled must carry battle_id: 99 and player: 1002"
    );
}
