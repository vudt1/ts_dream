//! Integration tests for Ticket 25 — Debuff Turn-Skip & Input Blocking.
//!
//! Covers the 3 pre-agreed seams:
//! 1. Battle Engine (`Battle::run_turn`): Type 3 priority over `14021 | 20014`,
//!    all 6 actions blocked under Type 3, soft debuffs unrestricted, `id == 1`
//!    turn decrement, and wire parity (`35 01` / `35 0F` / `32 01`).
//! 2. Turn Lifecycle (`BattleManager` + `BattleSink`): auto-skip locked cells
//!    with `0ms` exit while still sending `34 01`, waiting for unlocked Pet,
//!    and unlocking on the exact turn a debuff expires.
//! 3. Dispatcher & Service (`0x32 sub 1/2`): synchronous rejection without
//!    `35 05` or item consumption, collateral item safety, and `sys_msg_frame`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use ts_dream::battle::construction::Battle;
use ts_dream::battle::engine::WarInfo;
use ts_dream::battle::manager::{BattleManager, BattleSink, PlayerInput};
use ts_dream::battle::packets::{self, troi_byte};
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleCommands, BattleData, DbUpdate, Out, Outcome, PlayerSnapshot,
};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{BinarySkillDef, Item, Npc, Skill, TexpRow};
use ts_dream::protocol::encoder;
use ts_dream::server::dispatcher::{HandleOutcome, OpcodeCtx, ServerEnv};
use ts_dream::server::handlers::battle as battle_handler;
use ts_dream::server::session::{Conn, InventoryItem, PetState};
use ts_dream::server::spawn::sys_msg_frame;

static EMPTY_ITEMS: LazyLock<HashMap<i64, Item>> = LazyLock::new(HashMap::new);

fn create_test_data<'a>(
    npcs: &'a HashMap<i64, Npc>,
    skills: &'a HashMap<i64, Skill>,
    pet_slots: &'a HashMap<i64, [i64; 4]>,
    players: &'a HashMap<i64, PlayerSnapshot>,
    texps: &'a [TexpRow],
    binary_skills: &'a HashMap<u16, BinarySkillDef>,
) -> BattleData<'a> {
    BattleData::new(npcs, skills, &EMPTY_ITEMS, pet_slots, players, texps, None, 0, 0)
        .with_binary_skills(binary_skills)
}

fn command(row: u8, col: u8, target_row: u8, target_col: u8, action: BattleAction) -> BattleCommand {
    BattleCommand { row, col, target_row, target_col, action }
}

fn seed_unit(
    battle: &mut Battle,
    row: u8,
    col: u8,
    id: i64,
    typ: u8,
    team: i64,
    lv: i64,
    hp: i64,
    agi: i64,
) {
    let cell = battle.cell_mut(row, col).unwrap();
    *cell = WarInfo::default();
    cell.id = id;
    cell.typ = typ;
    cell.row = row;
    cell.col = col;
    cell.team = team;
    cell.lv = lv;
    cell.hp = hp;
    cell.hp_max = hp;
    cell.sp = 500;
    cell.sp_max = 500;
    cell.atk = 200;
    cell.int1 = 200;
    cell.def = 10;
    cell.agi = agi;
}

fn broadcast_frames(out: &[Out]) -> Vec<String> {
    out.iter()
        .filter_map(|o| match o {
            Out::Broadcast(frame) => Some(frame.clone()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Slice 1 — Engine Seam: Type 3 priority over 14021/20014, 6 actions blocked,
// soft debuffs unrestricted, and status id == 1 turn decrement.
// ---------------------------------------------------------------------------

#[test]
fn type3_overrides_berserk_and_blocks_all_six_actions_for_player_and_pet() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(
        10001,
        BinarySkillDef {
            id: 10001,
            name: "Fire Strike".to_string(),
            kind: 1,
            require_sp: 10,
            element: 3,
            numerical: 100,
            attribute: 1,
            fight_area: 0,
            round: 0,
            hit_status: 0,
            how_much_times: 1,
            max_lv: 9,
            ..Default::default()
        },
    );
    let mut items = HashMap::new();
    items.insert(
        26001,
        Item {
            id: 26001,
            name: b"Heal Pill".to_vec(),
            hp: 100,
            ..Default::default()
        },
    );
    let data = BattleData::new(&npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0)
        .with_binary_skills(&binary_skills);

    let all_actions = [
        BattleAction::NormalAttack,
        BattleAction::Defend,
        BattleAction::UseItem { item_id: 26001 },
        BattleAction::CastSkill { skill_id: 10001, skill_lv: 1 },
        BattleAction::Capture,
        BattleAction::Flee,
    ];

    // Both Player (3, 2, typ=2) and Pet (2, 2, typ=4) under Type 3 AND Berserk/Confusion (14021 / 20014).
    for (unit_row, unit_typ, berserk_id) in [(3u8, 2u8, 14021i64), (2u8, 4u8, 20014i64)] {
        for action in all_actions {
            let mut battle = Battle::with_seeds(100, 0, 42, 7, 99);
            seed_unit(&mut battle, unit_row, 2, 101, unit_typ, 1, 20, 1000, 200);
            if let Some(u) = battle.cell_mut(unit_row, 2) {
                u.id_char = 101;
                u.hp = 500; // below hp_max so UseItem would visibly heal if not blocked
                u.type3_id = 11014; // Freeze (Type 3)
                u.type3_turn = 5;
                u.type15_id = berserk_id; // Simultaneous Berserk/Confusion
                u.type15_turn = 5;
            }
            seed_unit(&mut battle, 0, 2, 501, 3, 2, 10, 5000, 10);

            let mut cmds = BattleCommands::new();
            cmds.insert((unit_row, 2), command(unit_row, 2, 0, 2, action));
            let mut out = Vec::new();
            let outcome = battle.run_turn(&data, &cmds, &mut out);

            let enemy = battle.cell(0, 2).unwrap();
            let actor = battle.cell(unit_row, 2).unwrap();
            assert_eq!(
                enemy.hp, 5000,
                "Type 3 must override berserk ({berserk_id}) and block {action:?} for typ={unit_typ}"
            );
            assert_eq!(actor.hp, 500, "UseItem must not heal under Type 3");
            assert_eq!(actor.sp, 500, "CastSkill must not spend SP under Type 3");
            assert_ne!(
                outcome,
                ts_dream::battle::runner::Outcome::PlayerFled,
                "Flee must not succeed under Type 3"
            );
        }
    }

    // Soft debuffs (14015 Poison, 10016-10019 / 10025 / 20022, 13011) must NOT block actions.
    for soft_id in [14015i64, 10016, 13011] {
        let mut battle = Battle::with_seeds(101, 0, 42, 7, 99);
        seed_unit(&mut battle, 3, 2, 101, 2, 1, 20, 1000, 200);
        if let Some(u) = battle.cell_mut(3, 2) {
            u.type15_id = soft_id;
            u.type15_turn = 5;
        }
        seed_unit(&mut battle, 0, 2, 501, 3, 2, 10, 5000, 10);

        let mut cmds = BattleCommands::new();
        cmds.insert((3, 2), command(3, 2, 0, 2, BattleAction::NormalAttack));
        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &cmds, &mut out);

        let enemy = battle.cell(0, 2).unwrap();
        assert!(
            enemy.hp < 5000,
            "Soft debuff {soft_id} must allow NormalAttack to deal damage"
        );
    }
}

#[test]
fn status_id_one_decrements_and_expires() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let binary_skills = HashMap::new();
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    let mut battle = Battle::with_seeds(102, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 101, 2, 1, 20, 1000, 200);
    if let Some(u) = battle.cell_mut(3, 2) {
        u.type3_id = 1;
        u.type3_turn = 3;
        u.type15_id = 1;
        u.type15_turn = 3;
    }
    seed_unit(&mut battle, 0, 2, 501, 3, 2, 10, 5000, 10);

    // Turn 1: 3 -> 2
    let mut out1 = Vec::new();
    let _ = battle.run_turn(&data, &BattleCommands::new(), &mut out1);
    let u1 = battle.cell(3, 2).unwrap();
    assert_eq!(u1.type3_turn, 2, "type3_id == 1 must decrement from 3 to 2");
    assert_eq!(u1.type15_turn, 2, "type15_id == 1 must decrement from 3 to 2");

    // Turn 2: 2 -> 1 (<= 1) -> expired!
    let mut out2 = Vec::new();
    let _ = battle.run_turn(&data, &BattleCommands::new(), &mut out2);
    let u2 = battle.cell(3, 2).unwrap();
    assert_eq!(u2.type3_id, 0, "type3_id == 1 must expire when turn <= 1");
    assert_eq!(u2.type3_turn, 0);
    assert_eq!(u2.type15_id, 0, "type15_id == 1 must expire when turn <= 1");
    assert_eq!(u2.type15_turn, 0);
    let frames = broadcast_frames(&out2);
    assert!(frames.contains(&packets::purge_status(3, 2, packets::PURGE_TYPE_DISABLE)));
    assert!(frames.contains(&packets::purge_status(3, 2, packets::PURGE_TYPE_DEBUFF)));
}

// ---------------------------------------------------------------------------
// Slice 2 — Engine Seam: Wire parity 35 01 on direct Type 3/15 cast, and
// 32 01 Type3 + 35 01 + 35 0F on physical/magic secondary status infliction.
// ---------------------------------------------------------------------------

#[test]
fn direct_type3_and_type15_cast_emits_troi_start_35_01() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(
        11014,
        BinarySkillDef {
            id: 11014,
            name: "Bang Phong".to_string(),
            kind: 1,
            require_sp: 10,
            element: 2,
            numerical: 0,
            attribute: 3,
            fight_area: 0,
            round: 0,
            hit_status: 0,
            how_much_times: 1,
            max_lv: 9,
            ..Default::default()
        },
    );
    binary_skills.insert(
        14015,
        BinarySkillDef {
            id: 14015,
            name: "Bo Doc".to_string(),
            kind: 1,
            require_sp: 10,
            element: 1,
            numerical: 0,
            attribute: 15,
            fight_area: 0,
            round: 0,
            hit_status: 0,
            how_much_times: 1,
            max_lv: 9,
            ..Default::default()
        },
    );
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    // Direct Type 3 cast (11014)
    let mut b1 = Battle::with_seeds(103, 0, 42, 7, 99);
    seed_unit(&mut b1, 3, 2, 101, 2, 1, 50, 1000, 200);
    b1.cell_mut(3, 2).unwrap().int1 = 2000;
    seed_unit(&mut b1, 0, 2, 501, 3, 2, 10, 5000, 10);
    b1.cell_mut(0, 2).unwrap().int1 = 0;

    let mut cmds1 = BattleCommands::new();
    cmds1.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: 11014, skill_lv: 5 }),
    );
    let mut out1 = Vec::new();
    let _ = b1.run_turn(&data, &cmds1, &mut out1);
    assert_eq!(b1.cell(0, 2).unwrap().type3_id, 11014);
    let frames1 = broadcast_frames(&out1);
    assert!(
        frames1.contains(&packets::troi_start(0, 2, 11014)),
        "Direct Type 3 cast must emit 35 01 troi_start for target, got: {frames1:?}"
    );

    // Direct Type 15 cast (14015)
    let mut b2 = Battle::with_seeds(104, 0, 42, 7, 99);
    seed_unit(&mut b2, 3, 2, 101, 2, 1, 50, 1000, 200);
    b2.cell_mut(3, 2).unwrap().int1 = 2000;
    seed_unit(&mut b2, 0, 2, 501, 3, 2, 10, 5000, 10);
    b2.cell_mut(0, 2).unwrap().int1 = 0;

    let mut cmds2 = BattleCommands::new();
    cmds2.insert(
        (3, 2),
        command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: 14015, skill_lv: 5 }),
    );
    let mut out2 = Vec::new();
    let _ = b2.run_turn(&data, &cmds2, &mut out2);
    assert_eq!(b2.cell(0, 2).unwrap().type15_id, 14015);
    let frames2 = broadcast_frames(&out2);
    assert!(
        frames2.contains(&packets::troi_start(0, 2, 14015)),
        "Direct Type 15 cast must emit 35 01 troi_start for target, got: {frames2:?}"
    );
}

#[test]
fn attack_and_magic_secondary_status_emits_32_01_type3_35_01_and_35_0f() {
    let npcs = HashMap::new();
    let skills = HashMap::new();
    let pet_slots = HashMap::new();
    let players = HashMap::new();
    let texps: Vec<TexpRow> = Vec::new();
    let mut binary_skills = HashMap::new();
    binary_skills.insert(
        19001,
        BinarySkillDef {
            id: 19001,
            name: "Phys Freeze".to_string(),
            kind: 1,
            require_sp: 0,
            element: 1,
            numerical: 150,
            attribute: 1, // physical -> apply_attack
            fight_area: 0,
            round: 0,
            hit_status: 6, // ICE_BOUND
            how_much_times: 1,
            max_lv: 9,
            ..Default::default()
        },
    );
    binary_skills.insert(
        19002,
        BinarySkillDef {
            id: 19002,
            name: "Magic Freeze".to_string(),
            kind: 1,
            require_sp: 0,
            element: 2,
            numerical: 150,
            attribute: 2, // magic -> apply_magic
            fight_area: 0,
            round: 0,
            hit_status: 6, // ICE_BOUND
            how_much_times: 1,
            max_lv: 9,
            ..Default::default()
        },
    );
    let data = create_test_data(&npcs, &skills, &pet_slots, &players, &texps, &binary_skills);

    for (skill_id, expected_lv) in [(19001u16, 3i64), (19002u16, 1i64)] {
        let mut battle = Battle::with_seeds(105, 0, 42, 7, 99);
        seed_unit(&mut battle, 3, 2, 101, 2, 1, 20, 2000, 200);
        {
            let a = battle.cell_mut(3, 2).unwrap();
            a.atk = 500;
            a.int1 = 500;
            a.thuoctinh = 1;
        }
        seed_unit(&mut battle, 0, 2, 501, 3, 2, 1, 9999, 0);

        let mut cmds = BattleCommands::new();
        cmds.insert(
            (3, 2),
            command(3, 2, 0, 2, BattleAction::CastSkill { skill_id: i64::from(skill_id), skill_lv: 3 }),
        );
        let mut out = Vec::new();
        let _ = battle.run_turn(&data, &cmds, &mut out);

        let victim = battle.cell(0, 2).unwrap();
        assert_eq!(victim.type3_id, 6, "skill {skill_id} must inflict Type 3 status 6");

        let frames = broadcast_frames(&out);
        let expected_35_01 = packets::troi_start(0, 2, 6);
        let expected_35_0f = packets::battle_unit_status(
            0,
            2,
            packets::STATUS_SEAL_RESIST_PCT,
            victim.type3_turn.clamp(1, 255) as u8,
            (expected_lv * 10).clamp(1, 100) as u16,
        );
        let type3_effect_marker = packets::skilling_effect(troi_byte::TYPE3, 0, 1);
        let skill_le = encoder::le16(skill_id);
        let has_32_01_type3 = frames.iter().any(|f| {
            f.starts_with("F444") && f.contains("3201") && f.contains(&skill_le) && f.contains(&type3_effect_marker)
        });

        assert!(has_32_01_type3, "skill {skill_id} must emit 32 01 with TYPE3 effect, got: {frames:?}");
        assert!(frames.contains(&expected_35_01), "skill {skill_id} must emit 35 01 troi_start, got: {frames:?}");
        assert!(frames.contains(&expected_35_0f), "skill {skill_id} must emit 35 0F battle_unit_status, got: {frames:?}");
    }
}

// ---------------------------------------------------------------------------
// Slice 3 — Turn Lifecycle Seam: BattleManager + BattleSink
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CapturingSink {
    frames: Mutex<Vec<(Option<i64>, String)>>,
    ended: Mutex<Option<(i32, Outcome)>>,
}

impl BattleSink for CapturingSink {
    fn send_to(&self, player: i64, frame: String) {
        self.frames.lock().unwrap().push((Some(player), frame));
    }
    fn send_map(&self, _player: i64, _frame: String) {}
    fn broadcast(&self, frame: String) {
        self.frames.lock().unwrap().push((None, frame));
    }
    fn apply_db(&self, _update: DbUpdate) {}
    fn apply_drop(&self, _drop: Out) {}
    fn apply_catch(&self, _owner: i64, _npc_id: i64) {}
    fn apply_fled(&self, _player: i64) {}
    fn apply_respawn(&self, _npc_id: i64, _map_id: i64, _x: i64, _y: i64) {}
    fn apply_pet_exp(&self, _owner: i64, _stt: i64, _exp: i64) {}
    fn battle_ended(&self, id: i32, outcome: Outcome) {
        *self.ended.lock().unwrap() = Some((id, outcome));
    }
}

fn spawn_lifecycle_battle(
    battle: Battle,
    timeout: Duration,
) -> (
    ts_dream::battle::manager::BattleHandle,
    Arc<CapturingSink>,
    Arc<BattleManager>,
) {
    let sink = Arc::new(CapturingSink::default());
    let manager = Arc::new(BattleManager::new());
    let handle = manager.spawn_timeout(
        battle,
        Arc::new(HashMap::<i64, Npc>::new()),
        Arc::new(HashMap::<i64, Skill>::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        1,
        None,
        0,
        0,
        timeout,
        sink.clone(),
    );
    (handle, sink, manager)
}

async fn wait_for_frame(sink: &CapturingSink, target_frame: &str, max_wait: Duration) -> bool {
    let start = tokio::time::Instant::now();
    while start.elapsed() < max_wait {
        if sink
            .frames
            .lock()
            .unwrap()
            .iter()
            .any(|(_, f)| f == target_frame)
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test]
async fn manager_skips_all_locked_cells_immediately_while_sending_34_01() {
    let mut battle = Battle::with_seeds(201, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 100, 2, 1, 20, 5000, 20);
    if let Some(p) = battle.cell_mut(3, 2) {
        p.type3_id = 11014; // Hard CC (Freeze)
        p.type3_turn = 5;
    }
    seed_unit(&mut battle, 2, 2, 200, 4, 1, 20, 5000, 15);
    if let Some(pet) = battle.cell_mut(2, 2) {
        pet.id_char = 100;
        pet.type15_id = 14021; // Confusion/Berserk
        pet.type15_turn = 5;
    }
    seed_unit(&mut battle, 0, 2, 1001, 3, 2, 10, 5000, 5);

    let (handle, sink, _mgr) = spawn_lifecycle_battle(battle, Duration::from_secs(3));
    handle.ack_scene();

    let finish_frame = packets::turn_action_finish();
    let got_finish = wait_for_frame(&sink, &finish_frame, Duration::from_millis(300)).await;
    assert!(
        got_finish,
        "Turn with all player cells locked must skip 3s timeout and emit 32 02 immediately"
    );

    let frames = sink.frames.lock().unwrap().clone();
    assert!(
        frames
            .iter()
            .any(|(to, f)| *to == Some(100) && f == &packets::your_turn()),
        "34 01 turn prompt must still be sent to living player even when all cells are locked"
    );
}

#[tokio::test]
async fn manager_waits_for_unlocked_pet_when_player_is_locked() {
    let mut battle = Battle::with_seeds(202, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 100, 2, 1, 20, 5000, 20);
    if let Some(p) = battle.cell_mut(3, 2) {
        p.type3_id = 11014; // Locked player
        p.type3_turn = 5;
    }
    seed_unit(&mut battle, 2, 2, 200, 4, 1, 20, 5000, 15);
    battle.cell_mut(2, 2).unwrap().id_char = 100; // Unlocked pet
    seed_unit(&mut battle, 0, 2, 1001, 3, 2, 10, 5000, 5);

    let (handle, sink, _mgr) = spawn_lifecycle_battle(battle, Duration::from_secs(3));
    handle.ack_scene();

    tokio::time::sleep(Duration::from_millis(80)).await;
    let finish_frame = packets::turn_action_finish();
    {
        let frames = sink.frames.lock().unwrap();
        assert!(
            frames
                .iter()
                .any(|(to, f)| *to == Some(100) && f == &packets::your_turn()),
            "34 01 must be sent"
        );
        assert!(
            !frames.iter().any(|(_, f)| f == &finish_frame),
            "Must wait for unlocked Pet before resolving turn"
        );
    }

    assert!(handle.command(PlayerInput {
        player: 100,
        cmd: command(2, 2, 0, 2, BattleAction::NormalAttack),
    }));

    let got_finish = wait_for_frame(&sink, &finish_frame, Duration::from_millis(300)).await;
    assert!(
        got_finish,
        "Turn must resolve immediately after unlocked Pet submits command without waiting for locked Player"
    );
}

#[tokio::test]
async fn manager_prompts_and_waits_when_lock_expires_at_turn_start() {
    let mut battle = Battle::with_seeds(203, 0, 42, 7, 99);
    seed_unit(&mut battle, 3, 2, 100, 2, 1, 20, 5000, 20);
    if let Some(p) = battle.cell_mut(3, 2) {
        p.atk = 500;
        p.type3_id = 11014;
        p.type3_turn = 2; // turn_phase1 will decrement 2 -> 1 (<= 1) and clear!
    }
    seed_unit(&mut battle, 0, 2, 1001, 3, 2, 5, 1, 5);

    let (handle, sink, _mgr) = spawn_lifecycle_battle(battle, Duration::from_secs(3));
    handle.ack_scene();

    tokio::time::sleep(Duration::from_millis(80)).await;
    let finish_frame = packets::turn_action_finish();
    assert!(
        !sink
            .frames
            .lock()
            .unwrap()
            .iter()
            .any(|(_, f)| f == &finish_frame),
        "Must NOT skip player when Type 3 expires at upcoming turn_phase1 (turn == 2 -> 1 <= 1)"
    );

    assert!(handle.command(PlayerInput {
        player: 100,
        cmd: command(3, 2, 0, 2, BattleAction::NormalAttack),
    }));
    let got_finish = wait_for_frame(&sink, &finish_frame, Duration::from_millis(300)).await;
    assert!(got_finish, "Turn must resolve after newly-freed player submits command");
}

// ---------------------------------------------------------------------------
// Slice 4 — Dispatcher & Service Seam: 0x32 sub 1/2 synchronous rejection
// ---------------------------------------------------------------------------

fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a BattleService,
    out: &'a mut HandleOutcome,
    opcode: u8,
    sub: u8,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        data,
        service,
        out,
        opcode,
        sub,
        payload,
        decoded: &[],
        env: ServerEnv::none(),
    }
}

async fn setup_turn2_with_locked_cell(
    skill_id: i64,
    attribute: u8,
) -> (
    GameData,
    BattleService,
    Conn,
    tokio::sync::mpsc::Receiver<String>,
    (u8, u8),
) {
    let mut data = GameData::default();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"CC Boss".to_vec(),
            lv: 80,
            hp: 5000,
            hpx: 5000,
            sp: 1000,
            spx: 1000,
            int1: 5000,
            agi: 500,
            skill: [skill_id, skill_id, skill_id, skill_id],
            ..Default::default()
        },
    );
    data.npcs.insert(
        2001,
        Npc {
            id: 2001,
            name: b"Pet".to_vec(),
            lv: 20,
            hp: 5000,
            hpx: 5000,
            sp: 500,
            spx: 500,
            agi: 10,
            ..Default::default()
        },
    );
    data.binary_skill_defs.insert(
        skill_id as u16,
        BinarySkillDef {
            id: skill_id as u16,
            name: format!("Skill {skill_id}"),
            kind: 1,
            require_sp: 0,
            element: 2,
            numerical: 0,
            attribute,
            fight_area: 0,
            round: 0,
            hit_status: 0,
            how_much_times: 1,
            max_lv: 9,
            ..Default::default()
        },
    );

    let mut service = BattleService::new(Arc::new(data.clone()));
    service.set_input_timeout(Duration::from_millis(500));

    let (tx, mut rx) = tokio::sync::mpsc::channel(256);

    let mut conn = Conn::default();
    conn.session.id = 700;
    conn.session.level = 20;
    conn.session.hp = 500;
    conn.session.hp_max = 500;
    conn.session.sp = 500;
    conn.session.sp_max = 500;
    conn.session.agi = 10;
    conn.session.active_pet_stt = 1;
    conn.session.pets.push(PetState {
        stt: 1,
        id: 2001,
        name: b"Pet".to_vec(),
        level: 20,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        agi: 10,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 1,
        id: 26001,
        count: 5,
        ..Default::default()
    });
    conn.session.homdo.push(InventoryItem {
        slot: 2,
        id: 26002,
        count: 3,
        ..Default::default()
    });
    service.register_sender(
        700,
        Arc::new(tokio::sync::RwLock::new(conn.session.clone())),
        tx,
    );

    let bid = service.start_npc_battle_seeded(&mut conn.session, 1001, 0, 7, 13, 99);
    assert_ne!(bid, 0);
    service.ack_scene(bid);

    let prompt = packets::your_turn();
    let finish = packets::turn_action_finish();
    let t32 = packets::troi_start(3, 2, skill_id as u16);
    let t21 = packets::troi_start(2, 1, skill_id as u16);
    let mut locked_cell = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);

    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(f)) = tokio::time::timeout(Duration::from_millis(20), rx.recv()).await {
            if f == t32 {
                locked_cell = Some((3, 2));
            } else if f == t21 {
                locked_cell = Some((2, 1));
            } else if f == prompt {
                if locked_cell.is_some() {
                    // We are now at the start of the turn AFTER the debuff landed!
                    break;
                }
                let _ = service.submit_command(&conn.session, command(3, 2, 3, 2, BattleAction::Defend));
                let _ = service.submit_command(&conn.session, command(2, 1, 2, 1, BattleAction::Defend));
            } else if f == finish {
                // Turn finished; loop continues to catch troi_start/prompt.
            }
        }
    }
    let locked_cell = locked_cell.expect("NPC must inflict status on (3, 2) or (2, 1)");
    (data, service, conn, rx, locked_cell)
}

#[tokio::test]
async fn handler_0x32_sub1_and_sub2_reject_locked_cell_and_protect_inventory() {
    let expected_notice = sys_msg_frame("Đang trong trạng thái khống chế, không thể hành động lượt này!");

    // Test both Hard CC Type 3 (11014) and Confusion/Berserk Type 15 (14021).
    for (skill_id, attr) in [(11014i64, 3u8), (14021i64, 15u8)] {
        let (data, service, mut conn, mut rx, (lr, lc)) =
            setup_turn2_with_locked_cell(skill_id, attr).await;

        // 1. Test 0x32 sub 1 (NormalAttack 10000, Defend 17001, Flee 18001, Capture 15001, Skill 10001)
        for code in [10000u16, 17001, 18001, 15001, 10001] {
            let mut out = HandleOutcome::default();
            let payload = [lr, lc, 0, 2, (code & 0xFF) as u8, (code >> 8) as u8, 0, 0];
            let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x32, 1, &payload);
            battle_handler::handle_battle_command(&mut ctx);

            assert!(
                out.outgoing.iter().any(|f| f == &expected_notice),
                "0x32 sub 1 (code {code}, status {skill_id}) on locked cell ({lr},{lc}) must send sys_msg_frame"
            );
        }

        // Verify NO 35 05 acting indicator was broadcast to rx.
        let acting_frame = format!("F44404003505{:02X}{:02X}", lr, lc);
        while let Ok(f) = rx.try_recv() {
            assert_ne!(f, acting_frame, "Rejected 0x32 sub 1 must NOT broadcast 35 05");
        }

        // 2. Test 0x32 sub 2 (UseItem 26001) — must not consume 26001 or collateral 26002.
        let mut out2 = HandleOutcome::default();
        let payload2 = [lr, lc, lr, lc, (26001u16 & 0xFF) as u8, (26001u16 >> 8) as u8, 0, 0];
        let mut ctx2 = test_ctx(&mut conn, &data, &service, &mut out2, 0x32, 2, &payload2);
        battle_handler::handle_battle_command(&mut ctx2);

        assert!(
            out2.outgoing.iter().any(|f| f == &expected_notice),
            "0x32 sub 2 (status {skill_id}) on locked cell must send sys_msg_frame"
        );
        let count_26001 = conn.session.homdo.iter().find(|i| i.id == 26001).map(|i| i.count).unwrap_or(0);
        let count_26002 = conn.session.homdo.iter().find(|i| i.id == 26002).map(|i| i.count).unwrap_or(0);
        assert_eq!(count_26001, 5, "Rejected 0x32 sub 2 must NOT consume item 26001");
        assert_eq!(count_26002, 3, "Collateral item 26002 must remain untouched");
    }
}

#[tokio::test]
async fn handler_0x32_allows_soft_debuff_cell_and_consumes_item() {
    let (data, service, mut conn, _rx, (lr, lc)) = setup_turn2_with_locked_cell(14015, 15).await;

    let mut out = HandleOutcome::default();
    let payload = [lr, lc, lr, lc, (26001u16 & 0xFF) as u8, (26001u16 >> 8) as u8, 0, 0];
    let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 0x32, 2, &payload);
    battle_handler::handle_battle_command(&mut ctx);

    assert!(
        out.outgoing.is_empty(),
        "Soft debuff (14015 Poison) must NOT trigger turn-lock sys_msg_frame"
    );
    let count_26001 = conn.session.homdo.iter().find(|i| i.id == 26001).map(|i| i.count).unwrap_or(0);
    let count_26002 = conn.session.homdo.iter().find(|i| i.id == 26002).map(|i| i.count).unwrap_or(0);
    assert_eq!(count_26001, 4, "Accepted 0x32 sub 2 under soft debuff must consume 1 item");
    assert_eq!(count_26002, 3, "Collateral item 26002 must remain untouched");
}
