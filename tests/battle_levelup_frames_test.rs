//! Ticket 06 — post-battle level-up frames.
//!
//! Two frames were missing from the level-up path
//! (`.scratch/client-pseudo-op-code/battle/04_battle_end_rewards.md`):
//!
//! * `05 0A <charId LE32>` — the "LevelUP" event. Client handler
//!   `FUN_0072f970` attaches the `"LevelUP"` nameplate label and, when the id
//!   is the local player, plays `sound\WA0013.wav` (`§2.8`, evidence class A).
//!   Without it the level number changes but no label/sound appears.
//! * `08 01 CD` / `08 01 CE` — MaxHP/MaxSP. Attr `0xCD` writes `+0x404` and
//!   `0xCE` writes `+0x406` (`§2.4`). They must be pushed *after* `08 01 23`
//!   (level), which makes the client recompute its own maxes, so the
//!   server-computed values win.
//!
//! Required finish pipeline order:
//! `Texp → Lv → LevelUP → Hpmax → Hp → Spmax → Sp → Point → SkillPoint`.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use ts_dream::battle::construction::Battle;
use ts_dream::battle::packets;
use ts_dream::battle::runner::{
    BattleAction, BattleCommand, BattleData, DbTarget, Out, PlayerSnapshot, Stat,
};
use ts_dream::battle::service::BattleService;
use ts_dream::data::loader::GameData;
use ts_dream::data::tables::{Npc, Skill, TexpRow};
use ts_dream::data::texps::compute_texps;
use ts_dream::server::session::{lock_online_sessions, Session};

/// `08 01` attr code at hex offset 12 (content[2], after `F444` + LEN + `0801`).
fn attr_byte(frame: &str) -> &str {
    &frame[12..14]
}

/// ---- Builder: byte-exact `05 0A` -----------------------------------------

#[test]
fn level_up_frame_is_byte_exact() {
    // F444 + LE16(6) + [05][0A] + le32(1234 = 0x4D2).
    assert_eq!(packets::level_up(1234), "F4440600050AD2040000");
    assert_eq!(packets::level_up(0), "F4440600050A00000000");

    let bytes = hex::decode(packets::level_up(1234)).expect("valid hex");
    assert_eq!(&bytes[0..2], &[0xF4, 0x44], "TS magic header");
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0x06, "LEN = 6");
    assert_eq!(&bytes[4..], &[0x05, 0x0A, 0xD2, 0x04, 0x00, 0x00]);
}

#[test]
fn level_up_encodes_full_u32() {
    // The charId is a little-endian u32 (`Copy(RP,2,4)` in FUN_0072f970).
    assert_eq!(packets::level_up(0x0102_0304), "F4440600050A04030201");
}

/// ---- finish(): frame order when a character levels up ---------------------

/// Two-row Texps table: threshold 18 at index 1, effectively unreachable at
/// index 2, so exactly one level-up is granted for a texp of 20.
fn texps_one_level() -> Vec<TexpRow> {
    vec![
        TexpRow {
            lv: 0,
            reborn: [0, 0, 0, 0],
        },
        TexpRow {
            lv: 1,
            reborn: [18, 18, 18, 18],
        },
        TexpRow {
            lv: 2,
            reborn: [1_000_000, 1_000_000, 1_000_000, 1_000_000],
        },
    ]
}

/// A lone player cell (row 3, col 2) with 20 gained exp and a clean snapshot.
fn leveling_battle_and_data() -> (
    Battle,
    HashMap<i64, Npc>,
    HashMap<i64, Skill>,
    HashMap<i64, ts_dream::data::tables::Item>,
) {
    let mut battle = Battle::new(1, 0);
    {
        let c = battle.cell_mut(3, 2).unwrap();
        c.typ = 2;
        c.id = 7;
        c.leader_id = 7;
        c.hp = 100;
        c.hp_max = 100;
        c.sp = 50;
        c.sp_max = 100;
        c.lv = 1;
        c.team = 1;
        c.exp = 20;
    }
    (battle, HashMap::new(), HashMap::new(), HashMap::new())
}

#[test]
fn finish_level_up_emits_frames_in_contract_order() {
    let mut players = HashMap::new();
    players.insert(
        7,
        PlayerSnapshot {
            texp: 0,
            job: 0,
            hpx: 0,
            spx: 0,
            hpx2: 0,
            spx2: 0,
        },
    );
    let texps = texps_one_level();
    let pet_slots = HashMap::new();

    let (mut battle, npcs, skills, items) = leveling_battle_and_data();
    let data = BattleData::new(
        &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
    );

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    // 1. The `08 01` stat pipeline for the player, in exact order.
    let stat_seq: Vec<Stat> = out
        .iter()
        .filter_map(|o| match o {
            Out::Db(u) if u.target == DbTarget::Player(7) => Some(u.stat),
            _ => None,
        })
        .collect();
    assert_eq!(
        stat_seq,
        vec![
            Stat::Texp,
            Stat::Lv,
            Stat::Hpmax,
            Stat::Hp,
            Stat::Spmax,
            Stat::Sp,
            Stat::Point,
            Stat::SkillPoint,
        ],
        "finish() must emit Texp → Lv → Hpmax → Hp → Spmax → Sp → Point → SkillPoint"
    );

    // 2. The `05 0A` LevelUP event sits between Lv and Hpmax, carrying this
    //    character's id.
    let lv_idx = out
        .iter()
        .position(|o| matches!(o, Out::Db(u) if u.stat == Stat::Lv))
        .expect("Lv frame");
    let levelup_idx = out
        .iter()
        .position(|o| *o == Out::Broadcast(packets::level_up(7)))
        .expect("05 0A LevelUP frame");
    let hpmax_idx = out
        .iter()
        .position(|o| matches!(o, Out::Db(u) if u.stat == Stat::Hpmax))
        .expect("Hpmax frame");
    assert!(
        lv_idx < levelup_idx && levelup_idx < hpmax_idx,
        "LevelUP must be emitted after Lv and before the MaxHP lock"
    );

    // 3. The locked maxima equal the server-computed values, and HP/SP are set
    //    to them (full heal on level-up).
    let hp_max = ts_dream::battle::engine::get_hp_max(0, 0, 2, 0);
    let sp_max = ts_dream::battle::engine::get_sp_max(0, 0, 2, 0);
    let db_value = |stat: Stat| {
        out.iter()
            .find_map(|o| match o {
                Out::Db(u) if u.target == DbTarget::Player(7) && u.stat == stat => Some(u.value),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(db_value(Stat::Lv), 2);
    assert_eq!(db_value(Stat::Hpmax), hp_max);
    assert_eq!(db_value(Stat::Hp), hp_max);
    assert_eq!(db_value(Stat::Spmax), sp_max);
    assert_eq!(db_value(Stat::Sp), sp_max);
    assert_eq!(db_value(Stat::Point), 2, "2 attribute points per level");
    assert_eq!(db_value(Stat::SkillPoint), 1, "1 skill point per level");
}

#[test]
fn finish_without_level_up_emits_no_levelup_frame() {
    let mut players = HashMap::new();
    players.insert(
        7,
        PlayerSnapshot {
            texp: 0,
            job: 0,
            hpx: 0,
            spx: 0,
            hpx2: 0,
            spx2: 0,
        },
    );
    let texps = texps_one_level();
    let pet_slots = HashMap::new();

    let (mut battle, npcs, skills, items) = leveling_battle_and_data();
    // Not enough exp to cross the level-1 threshold.
    battle.cell_mut(3, 2).unwrap().exp = 5;
    let data = BattleData::new(
        &npcs, &skills, &items, &pet_slots, &players, &texps, None, 0, 0,
    );

    let mut out = Vec::new();
    battle.finish(&data, 1, false, false, &mut out);

    assert!(
        !out.iter()
            .any(|o| matches!(o, Out::Broadcast(f) if f.starts_with("F4440600050A"))),
        "no level-up ⇒ no 05 0A frame, got {out:?}"
    );
    assert!(
        !out.iter()
            .any(|o| matches!(o, Out::Db(u) if u.stat == Stat::Hpmax || u.stat == Stat::Spmax)),
        "no level-up ⇒ no MaxHP/MaxSP lock, got {out:?}"
    );
}

/// ---- End-to-end: the frames reach the wire in order -----------------------

/// Run one seeded Player-vs-NPC battle where the level-1 player kills a level-2
/// NPC (exp 20 ≥ the real level-1 threshold), then collect every frame the
/// owner received.
async fn run_leveling_battle() -> Vec<String> {
    let mut data = GameData::default();
    data.texps = compute_texps();
    data.npcs.insert(
        1001,
        Npc {
            id: 1001,
            name: b"LvNPC".to_vec(),
            lv: 2,
            hp: 1,
            sp: 10,
            agi: 1,
            ..Default::default()
        },
    );
    data.skills.insert(
        10000,
        Skill {
            id: 10000,
            name: "Tan Cong".to_string(),
            skill_type: 1,
            ..Default::default()
        },
    );

    let mut service = BattleService::new(Arc::new(data));
    service.set_input_timeout(std::time::Duration::from_millis(40));

    let sess = Arc::new(RwLock::new(Session {
        id: 9002,
        map_id: 1,
        id_leader: 9002,
        level: 1,
        reborn: 0,
        texp: 0,
        hpx: 0,
        spx: 0,
        hpx2: 0,
        spx2: 0,
        agi: 1000,
        hp: 5000,
        hp_max: 5000,
        sp: 500,
        sp_max: 500,
        ..Default::default()
    }));
    let mut rx = service.register(9002, sess.clone());
    lock_online_sessions().insert(9002, sess.read().await.clone());

    {
        let mut guard = sess.write().await;
        let bid = service.start_npc_battle_seeded(&mut guard, 1001, 0, 4, 1, 1);
        assert_ne!(bid, 0, "seeded NPC battle must start");
    }

    let cmd = BattleCommand {
        row: 3,
        col: 2,
        target_row: 0,
        target_col: 2,
        action: BattleAction::NormalAttack,
    };
    let snapshot = sess.read().await.clone();
    assert!(service.submit_command(&snapshot, cmd));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1500);
    loop {
        if sess.read().await.battle_id == 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() <= deadline,
            "battle did not finish in time"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let mut frames = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        frames.push(frame);
    }
    lock_online_sessions().remove(&9002);
    frames
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn battle_level_up_reaches_the_wire_in_order() {
    let frames = run_leveling_battle().await;

    // Keep only the level-up pipeline frames: the `08 01` stat pushes and the
    // `05 0A` event. In-battle frames (e.g. an SP regen `08 01 1A`) can precede
    // the finish burst, so start at the Texp push (attr `0x24`), which is the
    // first frame `finish()` emits for a leveling cell.
    let frames: Vec<String> = frames
        .into_iter()
        .filter(|f| f.starts_with("F4440C000801") || f.starts_with("F4440600050A"))
        .collect();
    let start = frames
        .iter()
        .position(|f| attr_byte(f) == "24")
        .expect("finish() must emit the Texp frame");
    let pipeline = &frames[start..];

    let attrs: Vec<String> = pipeline
        .iter()
        .map(|f| {
            if f.starts_with("F4440600050A") {
                "050A".to_string()
            } else {
                attr_byte(f).to_string()
            }
        })
        .collect();
    assert_eq!(
        attrs,
        vec!["24", "23", "050A", "CD", "19", "CE", "1A", "26", "25"],
        "wire order must be Texp → Lv → LevelUP → Hpmax → Hp → Spmax → Sp → Point → SkillPoint"
    );

    // The LevelUP frame carries the leveled character's id.
    assert_eq!(pipeline[2], packets::level_up(9002));

    // MaxHP (`08 01 CD`) / MaxSP (`08 01 CE`) lock the server-computed values.
    let value_at = |frame: &str| {
        let bytes = hex::decode(frame).expect("valid hex");
        u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]])
    };
    let hp_max = ts_dream::battle::engine::get_hp_max(0, 0, 2, 0) as u32;
    let sp_max = ts_dream::battle::engine::get_sp_max(0, 0, 2, 0) as u32;
    assert_eq!(value_at(&pipeline[3]), hp_max, "08 01 CD must carry MaxHP");
    assert_eq!(value_at(&pipeline[5]), sp_max, "08 01 CE must carry MaxSP");
}
