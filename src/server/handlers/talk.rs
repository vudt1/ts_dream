//! NpcEvent handler (Opcode 0x14): H1 start talk, H6 menus, H4 end talk, H8 warp talk, H9 select menu.
//!
//! Identity rules (ticket 18 review):
//! - **Map Object ID** (`map_object_id` = `idtalking`): the on-map instance
//!   from the H1 request (LE16). It keys `Data_Talks`/quests and is embedded
//!   hex-encoded in the special NPC packets.
//! - **NPC Template ID** (`npc_id`): resolved `(map, map_object_id) →
//!   NpcOnMap.NpcId`. The H1/H6 **special** branches (banker/inn/`16012`) are
//!   selected on this id, never on the object id.
//! - A **Talk Context** tracks `{talk_type, map_object_id, talk_count, select_menu}`.

use crate::battle::rng::DotNetRandom;
use crate::battle::runner::Outcome;
use crate::data::loader::GameData;
use crate::data::loaders::EveResult;
use crate::eve::auto_chain::{AutoChainResult, EventPhase};
use crate::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::handlers::npc_event::{self, NpcTrigger};
use crate::server::handlers::quest_sync::{remove_quest_task, set_quest_dont, set_quest_task};
use crate::server::handlers::shops::gold_frame;
use crate::server::handlers::stats::build_stat_update;
use crate::db::pool::DbPool;
use crate::server::session::{Conn, Session};

/// Reset the legacy talk-context fields (`typetalk`, menu, warp confirm)
/// plus the active event session — the packet-less half of [`end_talk_session`],
/// shared with the CP4 event finish path (whose frames were already sent).
///
/// Session-shaped (not `Conn`-shaped) so the post-battle resume in
/// `battle::service::battle_ended` can call it with only a session guard.
fn reset_talk_context(session: &mut Session) {
    session.idtalking = 0;
    session.select_menu = 0;
    session.talk_count = 0;
    session.warp_finish = false;
    session.talk_locked = false;
    session.current_event_session = None;
}

/// EndTalk packet + reset the whole talk context, over a bare [`Session`].
///
/// When an Eve event session or talk lock is active the client expects the
/// actor-unlock frame (`0x14 Sub 0x2C`, mode 0x02) before the close-dialog frame
/// (Bear `processStep` end-of-chain order). Legacy talk paths keep the
/// bare `F44402001408` for golden parity.
pub fn end_talk_session(session: &mut Session, out: &mut HandleOutcome) {
    if session.current_event_session.is_some() || session.talk_locked {
        let char_id = session.id;
        out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
    }
    out.send("F44402001408");
    reset_talk_context(session);
}

/// [`end_talk_session`] over the connection wrapper (legacy call sites).
pub fn end_talk(conn: &mut Conn, out: &mut HandleOutcome) {
    end_talk_session(&mut conn.session, out);
}

/// Wall-clock millisecond seed for the Eve RNG (shared by the talk-start
/// bridge and the Checkpoint 4 continue / menu continuations).
fn eve_rng_seed() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i32)
        .unwrap_or(12345)
}

/// Split a dialog hex string on `F444` and emit each fragment 500 ms apart.
/// Frame order is preserved.
pub fn talk_messages(conn: &mut Conn, talk_string: &str, out: &mut HandleOutcome) {
    for part in talk_string.split("F444") {
        if !part.is_empty() {
            let frame = format!("F444{part}");
            if frame == "F44402001408" {
                conn.session.select_menu = 40;
            }
            out.send_delayed(frame, 500);
        }
    }
}

/// Dispatch Opcode 0x14 — NpcEvent (NPC talk / gate / scene-script).
pub async fn handle_talk(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let data = ctx.data;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => handle_talk_start(conn, payload, data, Some(ctx.service), pool, out).await,
        4 => handle_talk_click_door(conn, payload, data, pool, out).await,
        6 => handle_talk_continue(conn, payload, data, pool, out).await,
        8 => handle_talk_warp(conn, payload, data, &ctx.env, out).await,
        9 => handle_talk_select_menu(conn, payload, data, pool, out).await,
        _ => end_talk(conn, out),
    }
}

/// H1 identity + distance gate.
///
/// The `(map, object)` row MUST exist (its NPC template id drives the special
/// branches and its position the ±150 distance test); a talk to a missing
/// on-map instance is rejected with EndTalk (ticket 18 review: "reject
/// missing/out-of-range before any packet").
fn resolve_npc(data: &GameData, conn: &Conn, map_object_id: i32) -> Option<(i32, bool)> {
    if let Some(npc) = data
        .npc_on_map
        .iter()
        .find(|n| n.map_id == i64::from(conn.session.map_id) && n.id == i64::from(map_object_id))
    {
        let dx = i64::from(conn.session.map_x) - npc.x;
        let dy = i64::from(conn.session.map_y) - npc.y;
        let in_range = (-150..=150).contains(&dx) && (-150..=150).contains(&dy);
        return Some((npc.npc_id as i32, in_range));
    }
    if let Some(scene) = data.scene_eve_data.get(&u32::from(conn.session.map_id)) {
        if let Some(npc) = scene.npcs.get(&(map_object_id as u16)) {
            let dx = i64::from(conn.session.map_x) - i64::from(npc.x);
            let dy = i64::from(conn.session.map_y) - i64::from(npc.y);
            let in_range = (-150..=150).contains(&dx) && (-150..=150).contains(&dy);
            return Some((i32::from(npc.npc_id), in_range));
        }
    }
    if crate::server::handlers::event_npcs::is_event_npc(map_object_id) {
        return Some((map_object_id, true));
    }
    None
}

/// Central Checkpoint 4/6 walker: dispatch the active event session's result
/// queue from `current_index` until the session needs client input or runs
/// out of results.
///
/// Queue-index convention: `current_index` points at the **last dispatched**
/// result (talk-start and talk-continue leave it sitting on the delivered
/// Talk/Surface); Action results advance it inline while walking. Callers
/// re-entering after a delivered result (Sub 6 / Sub 9 / post-battle resume)
/// advance it first.
///
/// - Talk (`result_type == 1`): emit the step frame and stop — the next
///   `0x14 Sub 0x06` continues.
/// - Action (`0`): apply the state change and keep walking (non-blocking).
/// - Surface (`6`): record `last_surface_id`, switch to
///   [`EventPhase::AwaitingChoice`], emit the frame and stop — the
///   `0x14 Sub 0x09` choice answers it.
/// - Door (`2`): warp to `data.warps[(source map, parameter)]` then
///   [`finish_event_session`]. A missing warp id skips (advance + continue).
/// - Battle (`3`): pre-check `scene.fight_datas[result_mean_no]`, park the
///   session in [`EventPhase::AwaitingBattle`] and hand `out.eve_battle` to the
///   connection loop, which starts the battle **after** this outcome's frames
///   flush (so talk frames are never reordered behind battle frames).
/// - Exhausted queue: unlock + close-dialog, then auto-chain into the next
///   matching event or reset the talk context.
///
/// Takes `&mut Session` (not `&mut Conn`) because the post-battle resume in
/// `battle::service::battle_ended` reaches it through a session guard only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreFlightError {
    HomdoFull { item_id: u16 },
    QuestLedgerFull { quest_id: u16 },
    PetBagFull { pet_id: u16 },
    GoldCapExceeded,
    InsufficientGold,
    PointCapExceeded,
    InsufficientPoint,
}

/// All-or-Nothing pre-flight probe over the contiguous non-blocking
/// `DomainEffect` batch starting at `effects[0]`.
///
/// Simulates `homdo (0x17)` item mutations, `Quest Ledger (0x18)` 200-slot
/// allocations, `RewardPet` 4-slot active pet bag capacity, and `GOLD_CAP` /
/// `POINT_CAP` bounds on scratch state before any real mutation occurs.
fn preflight_probe_batch(
    session: &Session,
    data: &GameData,
    effects: &[crate::eve::planner::DomainEffect],
) -> Result<(), PreFlightError> {
    use crate::eve::planner::DomainEffect;

    let mut trial_homdo = session.homdo.clone();
    let mut trial_quests = session.quest_tasks.clone();
    let mut trial_pets = session.pets.clone();
    let mut trial_gold = session.gold;
    let mut trial_point = session.point;
    let mut trial_shop_point = session.shop_point;

    for effect in effects {
        if effect.is_blocking_or_terminal() {
            break;
        }
        match effect {
            DomainEffect::MutateItem {
                item_id,
                p_style: _,
                delta,
            } => {
                if *delta < 0 {
                    crate::server::inventory::remove_item(
                        &mut trial_homdo,
                        *item_id,
                        delta.unsigned_abs(),
                    );
                } else if *delta > 0 {
                    let mut rem = *delta as u32;
                    while rem > 0 {
                        let chunk = rem.min(crate::server::inventory::STACK_CAP as u32) as u8;
                        let item = crate::server::inventory::from_template(data, *item_id, chunk);
                        if !crate::server::inventory::can_add_item(&trial_homdo, &item) {
                            return Err(PreFlightError::HomdoFull { item_id: *item_id });
                        }
                        crate::server::inventory::add_item(&mut trial_homdo, item);
                        rem -= u32::from(chunk);
                    }
                }
            }
            DomainEffect::UpdateQuest {
                quest_id,
                p_style,
                result_value,
            } => {
                if *p_style == 3 && *result_value == 0 {
                    trial_quests.remove(quest_id);
                } else if ((*p_style == 1 && *result_value >= 0) || matches!(*p_style, 2 | 4))
                    && *quest_id != 0
                {
                    if !trial_quests.contains_key(quest_id) {
                        let used: std::collections::HashSet<u8> = trial_quests
                            .values()
                            .map(|(s, _)| *s)
                            .filter(|&s| (1..=crate::server::handlers::quest_sync::QUEST_SLOT_MAX).contains(&s))
                            .collect();
                        let Some(free_slot) = (1..=crate::server::handlers::quest_sync::QUEST_SLOT_MAX)
                            .find(|s| !used.contains(s))
                        else {
                            return Err(PreFlightError::QuestLedgerFull {
                                quest_id: *quest_id,
                            });
                        };
                        trial_quests.insert(*quest_id, (free_slot, 1));
                    }
                }
            }
            DomainEffect::RewardPet(res) => {
                let pet_id = res.parameter;
                match res.parameter_style {
                    1 if pet_id > 0 => {
                        if !trial_pets.iter().any(|p| p.id == pet_id) {
                            let Some(stt) = crate::server::pet_box::next_active_slot(&trial_pets)
                            else {
                                return Err(PreFlightError::PetBagFull { pet_id });
                            };
                            trial_pets.push(crate::server::session::PetState {
                                stt,
                                id: pet_id,
                                ..Default::default()
                            });
                        }
                    }
                    2 if pet_id > 0 => {
                        trial_pets.retain(|p| p.id != pet_id);
                    }
                    _ => {}
                }
            }
            DomainEffect::MutateMoney { p_style, amount } => match *p_style {
                1 => {
                    if *amount > 0 {
                        if *amount < 1000 {
                            let rewarded = u32::from(*amount as u16).saturating_mul(20);
                            if u32::from(trial_point).saturating_add(rewarded) > u32::from(u16::MAX) {
                                return Err(PreFlightError::PointCapExceeded);
                            }
                            trial_point = trial_point.saturating_add(rewarded as u16);
                        } else {
                            let next = u64::from(trial_gold).saturating_add(*amount as u64);
                            if next > u64::from(crate::protocol::GOLD_CAP) {
                                return Err(PreFlightError::GoldCapExceeded);
                            }
                            trial_gold = next as u32;
                        }
                    } else if *amount < 0 {
                        let abs = amount.unsigned_abs();
                        if trial_gold < abs {
                            return Err(PreFlightError::InsufficientGold);
                        }
                        trial_gold -= abs;
                    }
                }
                2 => {
                    if *amount > 0 {
                        let abs = *amount as u32;
                        if trial_gold < abs {
                            return Err(PreFlightError::InsufficientGold);
                        }
                        trial_gold -= abs;
                    }
                }
                3 => {
                    if *amount > 0 {
                        let next = u64::from(trial_shop_point).saturating_add(*amount as u64);
                        if next > u64::from(crate::protocol::POINT_CAP) {
                            return Err(PreFlightError::PointCapExceeded);
                        }
                        trial_shop_point = next as u32;
                    }
                }
                4 => {
                    if *amount > 0 {
                        let abs = *amount as u32;
                        if trial_shop_point < abs {
                            return Err(PreFlightError::InsufficientPoint);
                        }
                        trial_shop_point -= abs;
                    }
                }
                _ => {}
            },
            DomainEffect::PlayerEffect {
                parameter: 1,
                p_style: 3,
                result_value,
            } => {
                if *result_value > 0 {
                    let gained = (*result_value & 0xFFFF) as u32;
                    if u32::from(trial_point).saturating_add(gained) > u32::from(u16::MAX) {
                        return Err(PreFlightError::PointCapExceeded);
                    }
                    trial_point = trial_point.saturating_add(gained as u16);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn emit_preflight_rejection(session: &mut Session, out: &mut HandleOutcome, err: PreFlightError) {
    match err {
        PreFlightError::QuestLedgerFull { quest_id } => {
            tracing::debug!(quest_id, "pre-flight rejected: Quest Ledger (200 slots) full");
            out.send(crate::protocol::codecs::quest_sync::QuestSyncCodec::build_quest_full_hex());
        }
        PreFlightError::HomdoFull { item_id } => {
            tracing::debug!(item_id, "pre-flight rejected: homdo (25 slots) full");
            out.send(crate::server::spawn::sys_msg_frame(
                "Túi đồ đã đầy, không thể nhận thêm vật phẩm.",
            ));
        }
        PreFlightError::PetBagFull { pet_id } => {
            tracing::debug!(pet_id, "pre-flight rejected: active pet roster (4 slots) full");
            out.send(crate::server::spawn::sys_msg_frame(
                "Túi võ tướng đã đầy (tối đa 4), không thể nhận thêm.",
            ));
        }
        PreFlightError::GoldCapExceeded => {
            tracing::debug!("pre-flight rejected: GOLD_CAP exceeded");
            out.send(crate::server::spawn::sys_msg_frame(
                "Số tiền vượt quá giới hạn cho phép.",
            ));
        }
        PreFlightError::InsufficientGold => {
            tracing::debug!("pre-flight rejected: insufficient gold");
            out.send(crate::server::spawn::sys_msg_frame(
                "Không đủ tiền để thực hiện thao tác.",
            ));
        }
        PreFlightError::PointCapExceeded => {
            tracing::debug!("pre-flight rejected: POINT_CAP exceeded");
            out.send(crate::server::spawn::sys_msg_frame(
                "Số điểm vượt quá giới hạn cho phép.",
            ));
        }
        PreFlightError::InsufficientPoint => {
            tracing::debug!("pre-flight rejected: insufficient point");
            out.send(crate::server::spawn::sys_msg_frame(
                "Không đủ điểm để thực hiện thao tác.",
            ));
        }
    }
    end_talk_session(session, out);
}

#[derive(Debug)]
pub enum StepOutcome {
    Halt,
    Continue,
    SkillReward {
        skill_id: u16,
        target_level: u8,
        sp: u8,
        name: String,
    },
}

fn apply_skill_reward_to_session(
    session: &mut Session,
    out: &mut HandleOutcome,
    skill_id: u16,
    target_level: u8,
    skill_name: &str,
) {
    if let Some(entry) = session.skills.iter_mut().find(|(id, _)| *id == skill_id) {
        if target_level > entry.1 {
            entry.1 = target_level;
        }
    } else {
        session.skills.push((skill_id, target_level));
    }
    out.send(format!(
        "F4440C0008016E01{}{}",
        encoder::le32(u32::from(target_level)),
        encoder::le32(u32::from(skill_id))
    ));
    out.send(crate::server::spawn::sys_msg_frame(&format!(
        "Học được kỹ năng: {}",
        skill_name
    )));
    crate::server::session::registry_update_if_online(session);
}

fn execute_reward_pet(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    res: &EveResult,
) {
    let pet_id = res.parameter;
    if pet_id == 0 {
        return;
    }
    match res.parameter_style {
        1 => {
            if session.pets.iter().any(|p| p.id == pet_id) {
                tracing::debug!(pet_id, "RewardPet add skipped: player already owns pet");
                return;
            }
            let Some(stt) = crate::server::pet_box::next_active_slot(&session.pets) else {
                tracing::warn!(pet_id, "RewardPet add skipped: active pet roster full");
                return;
            };
            let pet = if let Some(npc) = data.npcs.get(&i64::from(pet_id)) {
                let lv = npc.lv.clamp(1, 200) as u8;
                let hp_max = if npc.hp > 0 {
                    npc.hp.min(i64::from(u16::MAX)) as u16
                } else {
                    crate::battle::engine::get_hp_max(npc.reborn, 0, i64::from(lv), npc.hpx)
                        .clamp(1, i64::from(u16::MAX)) as u16
                };
                let sp_max = if npc.sp > 0 {
                    npc.sp.min(i64::from(u16::MAX)) as u16
                } else {
                    crate::battle::engine::get_sp_max(npc.reborn, 0, i64::from(lv), npc.spx)
                        .clamp(0, i64::from(u16::MAX)) as u16
                };
                let skills: [(u16, u8); 4] = [
                    (npc.skill[0].max(0) as u16, if npc.skill[0] > 0 { 1 } else { 0 }),
                    (npc.skill[1].max(0) as u16, if npc.skill[1] > 0 { 1 } else { 0 }),
                    (npc.skill[2].max(0) as u16, if npc.skill[2] > 0 { 1 } else { 0 }),
                    (npc.skill[3].max(0) as u16, if npc.skill[3] > 0 { 1 } else { 0 }),
                ];
                crate::server::session::PetState {
                    stt,
                    id: pet_id,
                    name: npc.name.clone(),
                    level: lv,
                    thuoctinh: npc.thuoctinh.clamp(0, 255) as u8,
                    hp: hp_max,
                    hp_max,
                    sp: sp_max,
                    sp_max,
                    hpx: npc.hpx.clamp(0, i64::from(u16::MAX)) as u16,
                    spx: npc.spx.clamp(0, i64::from(u16::MAX)) as u16,
                    int1: npc.int1.clamp(0, i64::from(u16::MAX)) as u16,
                    atk: npc.atk.clamp(0, i64::from(u16::MAX)) as u16,
                    def: npc.def.clamp(0, i64::from(u16::MAX)) as u16,
                    agi: npc.agi.clamp(0, i64::from(u16::MAX)) as u16,
                    fai: 60,
                    skills,
                    storage_type: crate::db::modern::model::PetStorageType::Carried,
                    ..Default::default()
                }
            } else {
                let hp_max = crate::battle::engine::get_hp_max(0, 0, 1, 0) as u16;
                crate::server::session::PetState {
                    stt,
                    id: pet_id,
                    level: 1,
                    thuoctinh: 1,
                    hp: hp_max,
                    hp_max,
                    fai: 60,
                    storage_type: crate::db::modern::model::PetStorageType::Carried,
                    ..Default::default()
                }
            };
            let stat_frame = crate::server::spawn::pet_stat_frame(session, &pet);
            session.pets.push(pet);
            out.send(NpcTalkCodec::build_talk_step_hex(res));
            out.send(stat_frame);
            out.send(format!(
                "F4440C000F01{}{:02X}{}01",
                encoder::le32(session.id),
                stt,
                encoder::le32(u32::from(pet_id))
            ));
            crate::server::session::registry_update_if_online(session);
        }
        2 => {
            let Some(pos) = session.pets.iter().position(|p| p.id == pet_id) else {
                tracing::debug!(pet_id, "RewardPet remove skipped: player does not own pet");
                return;
            };
            let stt = session.pets[pos].stt;
            session.pets.remove(pos);
            if session.active_pet_stt == stt {
                session.active_pet_stt = 0;
            }
            let lo = u16::from(stt) * 10 + 1;
            let hi = u16::from(stt) * 10 + 6;
            session.trangbi.retain(|i| !(lo..=hi).contains(&u16::from(i.slot)));
            out.send(NpcTalkCodec::build_talk_step_hex(res));
            out.send(format!(
                "F44407000F02{}{:02X}",
                encoder::le32(session.id),
                stt
            ));
            crate::server::session::registry_update_if_online(session);
        }
        other => {
            tracing::warn!(
                pet_id,
                p_style = other,
                "unmapped RewardPet (rc=8) p_style; skipped"
            );
        }
    }
}

fn process_single_domain_effect(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    effect: &crate::eve::planner::DomainEffect,
    raw_result: &EveResult,
) -> StepOutcome {
    use crate::eve::planner::DomainEffect;

    match effect {
        // Talk: deliver the step and wait for the client's Sub 6.
        DomainEffect::TalkStep(res) => {
            out.send(NpcTalkCodec::build_talk_step_hex(res));
            StepOutcome::Halt
        }
        // Surface/menu: remember the surface id (conditionClass=10 key),
        // wait for the client's Sub 9 choice.
        DomainEffect::AwaitMenuChoice { surface_id, result } => {
            if let Some(ev) = session.current_event_session.as_mut() {
                ev.last_surface_id = *surface_id;
                ev.phase = EventPhase::AwaitingChoice;
            }
            out.send(NpcTalkCodec::build_talk_step_hex(result));
            StepOutcome::Halt
        }
        // Skill reward (free of skill_point): persist + sync `08 01 6E`.
        DomainEffect::LearnSkill {
            skill_id,
            target_level,
        } => {
            if let Some(skill) = data.skills.get(&i64::from(*skill_id)) {
                let already_has = session
                    .skills
                    .iter()
                    .any(|(id, lv)| *id == *skill_id && *lv >= *target_level);
                if !already_has {
                    return StepOutcome::SkillReward {
                        skill_id: *skill_id,
                        target_level: *target_level,
                        sp: skill.sp.min(255) as u8,
                        name: skill.name.clone(),
                    };
                }
            } else {
                tracing::warn!(skill_id, "Eve script skill reward references unknown skill");
            }
            StepOutcome::Continue
        }
        DomainEffect::MutateItem { .. }
        | DomainEffect::UpdateQuest { .. }
        | DomainEffect::MutateMoney { .. }
        | DomainEffect::PlayerEffect { .. } => {
            execute_action_result_non_skill(session, data, out, raw_result);
            StepOutcome::Continue
        }
        // Door: relocate to the scripted destination, then finish.
        DomainEffect::Warp { warp_id } => {
            let warp_key = (i64::from(session.map_id), i64::from(*warp_id));
            let Some(warp) = data.warps.get(&warp_key) else {
                tracing::debug!(
                    map_id = session.map_id,
                    warp_id,
                    "Eve Door result has no Warps.dat row; skipping"
                );
                return StepOutcome::Continue;
            };
            if session.id_leader > 0 && session.id_leader != session.id {
                tracing::debug!("Eve Door warp refused: member follows a party leader");
                end_talk_session(session, out);
                return StepOutcome::Halt;
            }
            crate::server::handlers::quest::perform_warp(session, warp, out);
            reset_talk_context(session);
            session.current_event_session = None;
            StepOutcome::Halt
        }
        // Battle: cleanly close/unlock any open talk UI first, then park the
        // session in AwaitingBattle for BattleService.
        DomainEffect::StartBattle { fight_id } => {
            let scene = data.scene_eve_data.get(&u32::from(session.map_id));
            let has_fight = scene
                .map(|s| s.fight_datas.contains_key(fight_id))
                .unwrap_or(false);
            if !has_fight {
                tracing::debug!(
                    map_id = session.map_id,
                    fight_id,
                    "Eve Battle result has no FightData row; skipping"
                );
                return StepOutcome::Continue;
            }
            if session.talk_locked {
                out.send(NpcTalkCodec::build_talk_lock_hex(
                    session.id,
                    TalkLockMode::Unlock,
                ));
                out.send(NpcTalkCodec::build_end_talk_hex());
                session.talk_locked = false;
            }
            let diahinh = scene
                .and_then(|s| s.scene_infos.get(&1))
                .map(|info| i32::from(info.background_no))
                .filter(|&bg| bg > 0)
                .unwrap_or(112);
            if let Some(ev) = session.current_event_session.as_mut() {
                ev.phase = EventPhase::AwaitingBattle;
            }
            out.eve_battle = Some((*fight_id, diahinh));
            StepOutcome::Halt
        }
        DomainEffect::NpcTeam(res) => {
            if (1..=5).contains(&res.parameter_style) {
                out.send(NpcTalkCodec::build_talk_step_hex(res));
            } else {
                tracing::warn!(
                    p_style = res.parameter_style,
                    param = res.parameter,
                    val = res.result_value,
                    "unsupported NpcTeam (rc=3) p_style; skipped"
                );
            }
            StepOutcome::Continue
        }
        DomainEffect::RewardPet(res) => {
            execute_reward_pet(session, data, out, res);
            StepOutcome::Continue
        }
        DomainEffect::MoveEntity(res) => {
            if res.result_class == 3 && res.result_value >= 10_000 {
                out.send(NpcTalkCodec::build_talk_step_hex(res));
            } else {
                tracing::warn!(
                    rc = res.result_class,
                    param = res.parameter,
                    val = res.result_value,
                    "invalid MoveEntity (rt=4) record; skipped"
                );
            }
            StepOutcome::Continue
        }
        DomainEffect::PlayMovie(res) => {
            out.send(NpcTalkCodec::build_talk_step_hex(res));
            StepOutcome::Continue
        }
        DomainEffect::NpcAction(res) => {
            // rt=9 is 100% server-side; never sent raw via S->C 0x14.
            tracing::debug!(
                rc = res.result_class,
                param = res.parameter,
                p_style = res.parameter_style,
                val = res.result_value,
                "executed server-side NpcAction (rt=9)"
            );
            StepOutcome::Continue
        }
        DomainEffect::Unsupported {
            result_type,
            result_class,
        } => {
            tracing::warn!(
                result_type,
                result_class,
                "unsupported/unknown Eve result (fail-closed, never sent raw via 0x14)"
            );
            StepOutcome::Continue
        }
    }
}

pub fn execute_event_step(session: &mut Session, data: &GameData, out: &mut HandleOutcome) {
    let mut batch_probed = false;
    loop {
        let remaining: Option<Vec<EveResult>> = session
            .current_event_session
            .as_ref()
            .and_then(|ev| ev.results.get(ev.current_index..).map(|s| s.to_vec()));
        let Some(rem) = remaining else {
            return;
        };
        if rem.is_empty() {
            if !finish_event_session_iter(session, data, out) {
                return;
            }
            batch_probed = false;
            continue;
        }

        let effects = crate::eve::planner::plan_results(&rem);
        if !batch_probed {
            if let Err(err) = preflight_probe_batch(session, data, &effects) {
                emit_preflight_rejection(session, out, err);
                return;
            }
            batch_probed = true;
        }

        match process_single_domain_effect(session, data, out, &effects[0], &rem[0]) {
            StepOutcome::Halt => return,
            StepOutcome::Continue => {
                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.current_index = ev.current_index.saturating_add(1);
                }
            }
            StepOutcome::SkillReward {
                skill_id,
                target_level,
                sp: _,
                name,
            } => {
                apply_skill_reward_to_session(session, out, skill_id, target_level, &name);
                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.current_index = ev.current_index.saturating_add(1);
                }
            }
        }
    }
}

pub async fn execute_event_step_with_pool(
    session: &mut Session,
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    execute_event_step_with_pool_opts(session, data, pool, out, false).await;
}

async fn execute_event_step_with_pool_opts(
    session: &mut Session,
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
    defer_warp: bool,
) {
    let mut batch_probed = false;
    loop {
        let remaining: Option<Vec<EveResult>> = session
            .current_event_session
            .as_ref()
            .and_then(|ev| ev.results.get(ev.current_index..).map(|s| s.to_vec()));
        let Some(rem) = remaining else {
            return;
        };
        if rem.is_empty() {
            if !finish_event_session_with_pool_iter(session, data, pool, out).await {
                return;
            }
            batch_probed = false;
            continue;
        }

        let effects = crate::eve::planner::plan_results(&rem);
        if !batch_probed {
            if let Err(err) = preflight_probe_batch(session, data, &effects) {
                emit_preflight_rejection(session, out, err);
                return;
            }
            batch_probed = true;
        }

        if let crate::eve::planner::DomainEffect::Warp { warp_id } = effects[0] {
            if session.talk_locked {
                out.send(NpcTalkCodec::build_talk_lock_hex(
                    session.id,
                    TalkLockMode::Unlock,
                ));
                session.talk_locked = false;
            }
            if defer_warp {
                out.eve_warp = Some(warp_id);
                reset_talk_context(session);
                session.current_event_session = None;
                return;
            }
            if crate::server::handlers::quest::perform_party_warp_and_persist(
                session, data, pool, warp_id, out,
            )
            .await
            {
                reset_talk_context(session);
                session.current_event_session = None;
            } else {
                end_talk_session(session, out);
            }
            return;
        }

        match process_single_domain_effect(session, data, out, &effects[0], &rem[0]) {
            StepOutcome::Halt => {
                if out.eve_battle.is_some() && session.db_character_id > 0 {
                    if !crate::db::persist::persist_sessions_transaction(
                        pool,
                        &[session],
                        &[],
                    )
                    .await
                    {
                        tracing::warn!(
                            player_id = session.id,
                            "StartBattle DB checkpoint failed"
                        );
                    }
                }
                return;
            }
            StepOutcome::Continue => {
                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.current_index = ev.current_index.saturating_add(1);
                }
            }
            StepOutcome::SkillReward {
                skill_id,
                target_level,
                sp,
                name,
            } => {
                let persist_ok = if let Some(pool) = pool {
                    match crate::db::persist::upsert_skill(
                        Some(pool),
                        session.id,
                        skill_id,
                        target_level,
                        sp,
                        0,
                    )
                    .await
                    {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::warn!("Failed to persist Eve skill reward {skill_id}: {e}");
                            false
                        }
                    }
                } else {
                    true
                };

                if persist_ok {
                    apply_skill_reward_to_session(session, out, skill_id, target_level, &name);
                }

                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.current_index = ev.current_index.saturating_add(1);
                }
            }
        }
    }
}

/// Finish an exhausted event session (`FinishChain`):
/// 1. Increment `(scene_id, eve_no)` when state-changing.
/// 2. Try `AutoChain`: if matched, keep `talk_locked` intact (no flicker!) and
///    return `true` so the `EventRunner` loop continues.
/// 3. If `NoMatch`: emit `Unlock` + `EndTalk` once, reset talk context, and
///    return `false`.
fn finish_event_session_iter(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
) -> bool {
    let char_id = session.id;
    let Some(completed) = session.current_event_session.take() else {
        return false;
    };
    if completed.has_state_changing_results() {
        let scene_id = u16::try_from(completed.map_id).unwrap_or(session.map_id);
        *session
            .completed_eve_counts
            .entry((scene_id, completed.eve_no))
            .or_insert(0) += 1;
    }
    let mut rng = DotNetRandom::new(eve_rng_seed());
    match npc_event::auto_chain_after(&completed, session, data, &mut rng) {
        AutoChainResult::Chained(new_ev) => {
            session.current_event_session = Some(new_ev);
            true
        }
        AutoChainResult::NoMatch => {
            out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
            out.send(NpcTalkCodec::build_end_talk_hex());
            reset_talk_context(session);
            crate::server::session::registry_update_if_online(session);
            false
        }
    }
}

async fn finish_event_session_with_pool_iter(
    session: &mut Session,
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) -> bool {
    let char_id = session.id;
    let Some(completed) = session.current_event_session.take() else {
        return false;
    };
    if completed.has_state_changing_results() {
        let scene_id = u16::try_from(completed.map_id).unwrap_or(session.map_id);
        *session
            .completed_eve_counts
            .entry((scene_id, completed.eve_no))
            .or_insert(0) += 1;
    }
    let mut rng = DotNetRandom::new(eve_rng_seed());
    match npc_event::auto_chain_after(&completed, session, data, &mut rng) {
        AutoChainResult::Chained(new_ev) => {
            session.current_event_session = Some(new_ev);
            true
        }
        AutoChainResult::NoMatch => {
            out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Unlock));
            out.send(NpcTalkCodec::build_end_talk_hex());
            reset_talk_context(session);
            crate::server::session::registry_update_if_online(session);
            if session.db_character_id > 0
                && !crate::db::persist::persist_sessions_transaction(pool, &[session], &[]).await
            {
                tracing::warn!(
                    player_id = session.id,
                    "FinishChain DB write-through failed"
                );
            }
            false
        }
    }
}

/// CP6 decision #4 & Phase 1 Fix Vấn đề 2 — advance a parked Eve event session
/// (`phase == AwaitingBattle`) after its scripted battle ended.
///
/// Returns `(frames, next_battle)`:
/// - **Win / Lose / Flee** records `battle_result` (1=Win, 2=Lose, 3=Flee),
///   advances `current_index` past the delivered battle, sets
///   `phase = EventPhase::Executing`, and walks `execute_event_step`.
///   Subsequent actions (e.g. quest step back-up on loss/flee, loss dialogue,
///   or post-battle auto-chain) will execute properly.
/// - `next_battle` carries an `eve_battle` requested *by the resumed walk*
///   (a second scripted battle in the same session); the caller — the battle
///   service, once its `members` lock is free — starts it.
///
/// A session that is not parked returns empty output untouched. Kept here
/// rather than in `battle::service` because it drives this module's dialog
/// state machine (and so the battle layer only forwards the outcome).
pub async fn resume_eve_after_battle_full(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
    pool: Option<&DbPool>,
) -> HandleOutcome {
    resume_eve_after_battle_opts(session, outcome, data, pool, false).await
}

pub async fn resume_eve_after_battle_deferred_warp(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
    pool: Option<&DbPool>,
) -> HandleOutcome {
    resume_eve_after_battle_opts(session, outcome, data, pool, true).await
}

async fn resume_eve_after_battle_opts(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
    pool: Option<&DbPool>,
    defer_warp: bool,
) -> HandleOutcome {
    let parked = matches!(
        session.current_event_session.as_ref(),
        Some(ev) if ev.phase == EventPhase::AwaitingBattle
    );
    if !parked {
        return HandleOutcome::default();
    }
    let battle_result: i32 = match outcome {
        Outcome::PlayerWin => 1,
        Outcome::PlayerLose => 2,
        Outcome::PlayerFled => 3,
        // Defensive: a battle still marked Running never reaches `battle_ended`.
        Outcome::Running => return HandleOutcome::default(),
    };
    {
        let Some(ev) = session.current_event_session.as_mut() else {
            return HandleOutcome::default();
        };
        ev.battle_result = battle_result;
        ev.phase = EventPhase::Executing;
        // Index sits on the delivered battle result — advance first.
        ev.current_index = ev.current_index.saturating_add(1);
    }
    let mut out = HandleOutcome::default();
    execute_event_step_with_pool_opts(session, data, pool, &mut out, defer_warp).await;
    out
}

pub async fn resume_eve_after_battle(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
    pool: Option<&DbPool>,
) -> (Vec<String>, Option<(u16, i32)>) {
    let out = resume_eve_after_battle_full(session, outcome, data, pool).await;
    let next_battle = out.eve_battle;
    let frames = out.outgoing.into_iter().map(|f| f.frame).collect();
    (frames, next_battle)
}

pub fn resume_eve_after_battle_sync_full(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
) -> HandleOutcome {
    let parked = matches!(
        session.current_event_session.as_ref(),
        Some(ev) if ev.phase == EventPhase::AwaitingBattle
    );
    if !parked {
        return HandleOutcome::default();
    }
    let battle_result: i32 = match outcome {
        Outcome::PlayerWin => 1,
        Outcome::PlayerLose => 2,
        Outcome::PlayerFled => 3,
        Outcome::Running => return HandleOutcome::default(),
    };
    {
        let Some(ev) = session.current_event_session.as_mut() else {
            return HandleOutcome::default();
        };
        ev.battle_result = battle_result;
        ev.phase = EventPhase::Executing;
        ev.current_index = ev.current_index.saturating_add(1);
    }
    let mut out = HandleOutcome::default();
    execute_event_step(session, data, &mut out);
    out
}

pub fn resume_eve_after_battle_sync(
    session: &mut Session,
    outcome: Outcome,
    data: &GameData,
) -> (Vec<String>, Option<(u16, i32)>) {
    let out = resume_eve_after_battle_sync_full(session, outcome, data);
    let next_battle = out.eve_battle;
    let frames = out.outgoing.into_iter().map(|f| f.frame).collect();
    (frames, next_battle)
}

/// Apply non-blocking Action result (`result_type == 0`) for non-skill classes.
fn execute_action_result_non_skill(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    result: &EveResult,
) {
    match result.result_class {
        // Item: parameter=itemId, result_value=signed quantity (homdo 0x17 only).
        1 => {
            let qty = result.result_value;
            if qty < 0 {
                session.remove_homdo_item(result.parameter, qty.unsigned_abs());
            } else if qty > 0 {
                let mut rem = qty as u32;
                while rem > 0 {
                    let chunk = rem.min(crate::server::inventory::STACK_CAP as u32) as u8;
                    let item = crate::server::inventory::from_template(data, result.parameter, chunk);
                    session.add_homdo_item(item);
                    rem -= u32::from(chunk);
                }
            }
            if qty != 0 {
                // Sync the mutated bag to the client (homdo 0x17).
                out.send(session.dump_homdo());
            }
        }
        // Quest-log row (`0x18 04/05/06`): parameter=questId,
        // parameter_style=pStyle, result_value=signed step/mark.
        // Bear's hardcoded save-gate is removed; all real p_styles (1..=5) are
        // supported and `quest_dont` (`0x18 05`) is set ONLY on true completion
        // (p_style=1 step=1) or explicit mark commands (p_style=3 val>0, p_style=5).
        2 => {
            let quest_id = result.parameter;
            let p_style = result.parameter_style;
            let battle_result = session
                .current_event_session
                .as_ref()
                .map(|ev| ev.battle_result)
                .unwrap_or(0);

            match p_style {
                1 => {
                    if result.result_value < 0 {
                        let dec = (result.result_value.unsigned_abs().min(255)) as u8;
                        if let Some((_slot, current)) = session.quest_tasks.get(&quest_id).copied() {
                            let next_step = current.saturating_sub(dec);
                            set_quest_task(session, out, quest_id, next_step);
                        }
                        return;
                    }
                    let step = (result.result_value & 0xFF) as u8;
                    let next_step = match session.quest_tasks.get(&quest_id) {
                        None => step.max(1),
                        Some((_slot, current)) => {
                            if matches!(battle_result, 2 | 3) {
                                current.saturating_sub(1)
                            } else {
                                current.saturating_add(step)
                            }
                        }
                    };
                    set_quest_task(session, out, quest_id, next_step);
                    // Only mark `quest_dont` when completing (step == 1) and battle was not lost/fled.
                    if step == 1 && !matches!(battle_result, 2 | 3) {
                        if let Some(&mark) = data.quest_marks.get(&quest_id) {
                            if mark > 0 {
                                set_quest_dont(session, out, mark, 1);
                            }
                        }
                    }
                }
                2 => {
                    let step = (result.result_value & 0xFF) as u8;
                    let next_step = match session.quest_tasks.get(&quest_id) {
                        None => 0,
                        Some((_slot, current)) => current.saturating_sub(step),
                    };
                    set_quest_task(session, out, quest_id, next_step);
                }
                3 => {
                    if result.result_value == 0 {
                        // pStyle 3 + val 0 = drop the quest from the log (`0x18 Sub 0x04`).
                        remove_quest_task(session, out, quest_id);
                    } else if !matches!(battle_result, 2 | 3) {
                        // Explicit completion mark (`pStyle=3, val>0`).
                        let target_qid = if quest_id == 2 && result.result_value >= 10000 {
                            result.result_value as u16
                        } else {
                            quest_id
                        };
                        if let Some(&mark) = data.quest_marks.get(&target_qid) {
                            if mark > 0 {
                                set_quest_dont(session, out, mark, 1);
                            }
                        }
                    }
                }
                4 => {
                    let step = (result.result_value & 0xFF) as u8;
                    let next_step = match session.quest_tasks.get(&quest_id) {
                        None => step.max(1),
                        Some((_slot, current)) => {
                            if matches!(battle_result, 2 | 3) {
                                current.saturating_sub(1)
                            } else {
                                current.saturating_add(step)
                            }
                        }
                    };
                    set_quest_task(session, out, quest_id, next_step);
                }
                5 => {
                    if !matches!(battle_result, 2 | 3) {
                        if let Some(&mark) = data.quest_marks.get(&quest_id) {
                            if mark > 0 {
                                set_quest_dont(session, out, mark, 1);
                            }
                        }
                    }
                }
                other => {
                    tracing::warn!(
                        quest_id,
                        p_style = other,
                        val = result.result_value,
                        "unmapped result_class=2 p_style; skipped"
                    );
                }
            }
        }
        // Gold / stat-point / shop-point reward (rc=5):
        // parameter_style=type, result_value=amount.
        5 => {
            let amount = result.result_value;
            match result.parameter_style {
                // Type 1: small positive amounts (< 1000) convert to stat points at 20:1;
                // >= 1000 is plain gold; negative deducts gold.
                1 => {
                    if amount > 0 {
                        if amount < 1000 {
                            let rewarded = u16::try_from(amount * 20).unwrap_or(u16::MAX);
                            session.point = session.point.saturating_add(rewarded);
                            out.send(build_stat_update(0x26, i32::from(session.point)));
                        } else {
                            let add = amount as u32;
                            session.gold = session.gold.saturating_add(add).min(crate::protocol::GOLD_CAP);
                            out.send(gold_frame(session.gold));
                            out.send(format!("F44406001A01{}", encoder::le32(add)));
                        }
                        crate::server::session::registry_update_if_online(session);
                    } else if amount < 0 {
                        let sub = amount.unsigned_abs();
                        session.gold = session.gold.saturating_sub(sub);
                        out.send(gold_frame(session.gold));
                        out.send(format!("F44406001A02{}", encoder::le32(sub)));
                        crate::server::session::registry_update_if_online(session);
                    }
                }
                // Type 2: deduct gold from player.
                2 => {
                    if amount > 0 {
                        let sub = amount as u32;
                        session.gold = session.gold.saturating_sub(sub);
                        out.send(gold_frame(session.gold));
                        out.send(format!("F44406001A02{}", encoder::le32(sub)));
                        crate::server::session::registry_update_if_online(session);
                    }
                }
                // Type 3: add shop points (`0x1A 05` + `0x1A 03` total).
                3 => {
                    if amount > 0 {
                        let add = amount as u32;
                        session.shop_point = session
                            .shop_point
                            .saturating_add(add)
                            .min(crate::protocol::POINT_CAP);
                        out.send(format!("F44406001A05{}", encoder::le32(add)));
                        out.send(crate::server::handlers::shops::point_frame(session.shop_point));
                        crate::server::session::registry_update_if_online(session);
                    }
                }
                // Type 4: deduct shop points (`0x1A 03` total).
                4 => {
                    if amount > 0 {
                        let sub = amount as u32;
                        session.shop_point = session.shop_point.saturating_sub(sub);
                        out.send(crate::server::handlers::shops::point_frame(session.shop_point));
                        crate::server::session::registry_update_if_online(session);
                    }
                }
                other => {
                    tracing::warn!(
                        gold_type = other,
                        amount,
                        "unmapped rc=5 Gold/Point p_style; skipped"
                    );
                }
            }
        }
        // Player-attribute effects (rc=7).
        7 => match result.parameter {
            // parameter 0 | 5: savemap when p_style == 1.
            0 | 5 => {
                if result.parameter_style == 1 {
                    crate::server::handlers::quest::save_map(session);
                    crate::server::session::registry_update_if_online(session);
                } else {
                    tracing::warn!(
                        param = result.parameter,
                        style = result.parameter_style,
                        "unmapped rc=7 savemap p_style; skipped"
                    );
                }
            }
            // parameter 1 (`GetSttBonus` / `savemap` / `texp`):
            1 => match result.parameter_style {
                1 => {
                    crate::server::handlers::quest::save_map(session);
                    crate::server::session::registry_update_if_online(session);
                }
                2 => {
                    let gained = (result.result_value & 0xFFFF) as u16;
                    session.skill_point = session.skill_point.saturating_add(gained);
                    out.send(build_stat_update(0x25, i32::from(session.skill_point)));
                    out.send(NpcTalkCodec::build_skill_point_toast_hex(gained.min(255) as u8));
                    crate::server::session::registry_update_if_online(session);
                }
                3 => {
                    let gained = (result.result_value & 0xFFFF) as u16;
                    session.point = session.point.saturating_add(gained);
                    out.send(build_stat_update(0x26, i32::from(session.point)));
                    out.send(NpcTalkCodec::build_stat_point_toast_hex(gained.min(255) as u8));
                    crate::server::session::registry_update_if_online(session);
                }
                // Type 4: EXP reward (`texp`) with level-up check, stat recompute,
                // full HP/SP heal, `0x08` stat syncs, and `0x14 0x2A 0x01 <exp_delta>`.
                4 => {
                    let exp_delta = result.result_value.max(0) as u32;
                    session.texp = session.texp.saturating_add(exp_delta);
                    out.send(build_stat_update(0x24, session.texp as i32));
                    out.send(NpcTalkCodec::build_exp_toast_hex(exp_delta));

                    let lv_ups = crate::data::texps::texp_get_lv_up(
                        &data.texps,
                        i64::from(session.level),
                        usize::from(session.reborn),
                        i64::from(session.texp),
                    );
                    if lv_ups > 0 {
                        let new_lv = (i64::from(session.level) + lv_ups)
                            .clamp(1, crate::protocol::MAX_LEVEL)
                            as u8;
                        session.level = new_lv;
                        session.recompute_stats();
                        session.hp = session.hp_max;
                        session.sp = session.sp_max;
                        session.point = session.point.saturating_add((2 * lv_ups) as u16);
                        session.skill_point = session.skill_point.saturating_add(lv_ups as u16);

                        out.send(build_stat_update(0x23, i32::from(session.level)));
                        out.send(crate::battle::packets::level_up(session.id));
                        out.send(build_stat_update(0xCD, i32::from(session.hp_max)));
                        out.send(build_stat_update(0x19, i32::from(session.hp)));
                        out.send(build_stat_update(0xCE, i32::from(session.sp_max)));
                        out.send(build_stat_update(0x1A, i32::from(session.sp)));
                        out.send(build_stat_update(0x26, i32::from(session.point)));
                        out.send(build_stat_update(0x25, i32::from(session.skill_point)));
                    }
                    crate::server::session::registry_update_if_online(session);
                }
                other => tracing::warn!(
                    stt_bonus_type = other,
                    value = result.result_value,
                    "unmapped rc=7 param=1 p_style; skipped"
                ),
            },
            // parameter 2 (`GetArmy`): no army store wired yet.
            2 => tracing::debug!(
                army_type = result.parameter_style,
                value = result.result_value,
                "Eve army result skipped: no army store wired"
            ),
            // parameter 3: open NPC shop form + optional savemap (no fallthrough).
            3 => {
                out.send(crate::server::handlers::shops::open_npc_shop_frame());
                if result.parameter_style == 1 {
                    crate::server::handlers::quest::save_map(session);
                    crate::server::session::registry_update_if_online(session);
                }
            }
            other => tracing::warn!(
                param = other,
                style = result.parameter_style,
                "unmapped rc=7 Player parameter; skipped"
            ),
        },
        other => {
            tracing::warn!(
                class = other,
                param = result.parameter,
                "unmapped Eve action result class; skipped"
            );
        }
    }
}

async fn handle_talk_start(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    service: Option<&crate::battle::service::BattleService>,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    if payload.len() < 2 {
        end_talk(conn, out);
        return;
    }
    let map_object_id = encoder::u16_le(payload[0], payload[1]) as i32;
    conn.session.idtalking = map_object_id;
    conn.session.talk_type = "NPC".to_string();
    conn.session.talk_count = 0;

    let Some((template_id, in_range)) = resolve_npc(data, conn, map_object_id) else {
        end_talk(conn, out); // missing on-map instance -> reject
        return;
    };
    conn.session.idnpctalking = template_id;

    // Special template ids embed the map object id (hex-encoded).
    // Distance gate applies before the special payload.
    if matches!(template_id, 16080 | 16004 | 16011 | 16015) {
        if !in_range {
            end_talk(conn, out);
            return;
        }
        out.send("F44402000602");
        out.send(format!(
            "F44411001401000000010603{:02X}0000000000000100",
            map_object_id
        ));
        return;
    }
    if matches!(template_id, 15002 | 16001 | 16016) {
        if !in_range {
            end_talk(conn, out);
            return;
        }
        out.send("F44402000602");
        out.send(format!(
            "F44411001401000000010603{:02X}0000000000000200",
            map_object_id
        ));
        return;
    }
    if template_id == 16012 {
        return;
    }

    // Special Event NPCs (Tickets 01, 04, 05, 09, 12, 13 -> event_npcs.rs)
    if crate::server::handlers::event_npcs::try_handle_event_npc(
        template_id,
        in_range,
        conn,
        data,
        service,
        pool,
        out,
    ).await {
        return;
    }

    // Generic dialog: the distance gate still applies.
    if !in_range {
        end_talk(conn, out);
        return;
    }
    // Eve Engine event bridge (Pure Planner + EventRunner).
    let mut rng = DotNetRandom::new(eve_rng_seed());
    if let Some(event_session) = npc_event::resolve_npc_event(
        &conn.session,
        data,
        NpcTrigger::ClickNpc(map_object_id),
        &mut rng,
    ) {
        // Do NOT emit `06 02` + `14 2C Lock` if the first non-Action step is
        // `StartBattle` (`rt=3`) or `Warp` (`rt=2`), avoiding dangling talk UI.
        let first_non_action = event_session
            .results
            .iter()
            .find(|r| r.result_type != 0);
        let should_lock_ui = !matches!(first_non_action.map(|r| r.result_type), Some(2 | 3));
        let char_id = conn.session.id;
        conn.session.current_event_session = Some(event_session);
        if should_lock_ui {
            conn.session.talk_locked = true;
            out.send("F44402000602");
            out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock));
        }
        execute_event_step_with_pool(&mut conn.session, data, pool, out).await;
        return;
    }

    end_talk(conn, out);
}

async fn handle_talk_continue(
    conn: &mut Conn,
    _payload: &[u8],
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    // Eve engine path (Checkpoint 4): an active event session owns the
    // continue — the legacy H6 guards below must not run for it.
    if conn.session.current_event_session.is_some() {
        if let Some(ev) = conn.session.current_event_session.as_mut() {
            if ev.phase == EventPhase::AwaitingChoice {
                // The pending Surface must be answered with Sub 9; a stray
                // continue must not skip the menu result.
                tracing::debug!("ignoring talk-continue while awaiting Surface choice");
                return;
            }
            if ev.phase == EventPhase::AwaitingBattle {
                // The eve battle is in flight and the queue index is parked on
                // the battle result — a stray continue must not advance past
                // it (the post-battle resume owns that advance).
                tracing::debug!("ignoring talk-continue while awaiting eve battle");
                return;
            }
            // The queue index sits on the result already delivered; advance
            // past it before walking the queue again (research CP4 §1.3).
            ev.current_index = ev.current_index.saturating_add(1);
        }
        execute_event_step_with_pool(&mut conn.session, data, pool, out).await;
        return;
    }

    // H6 pre-dispatch guards.
    if conn.session.warp_finish {
        out.send("F44402000504");
        out.send("F44402001408");
        conn.session.warp_finish = false;
        conn.session.talk_count = 0;
        conn.session.idtalking = 0;
        return;
    }
    if conn.session.idtalking == 0 && conn.session.select_menu == 40 {
        end_talk(conn, out);
        return;
    }
    if conn.session.idtalking <= 0 {
        return;
    }

    let template = conn.session.idnpctalking;

    // Banker / Store NPCs (branch on template id).
    if matches!(template, 16080 | 16004 | 16011 | 16023) {
        match conn.session.select_menu {
            30 => {
                out.send("F44403001D0900");
                out.send(format!(
                    "F44406001D04{}",
                    encoder::le32(conn.session.bank_gold)
                ));
                out.send("F44402001D05");
                out.send("F44402001409");
            }
            31 => {
                out.send("F44402001D06");
                out.send("F44402001409");
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        }
        return;
    }

    // Inn / hotel NPCs.
    if matches!(template, 15002 | 16001 | 16016 | 15118) {
        match conn.session.select_menu {
            30 => {
                out.send("F44411001401000000010603010000000000000100");
            }
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => {
                out.send(crate::server::handlers::shops::open_npc_shop_frame());
                out.send("F44411001401000000010603010000000000000100");
            }
            33 => {
                crate::server::handlers::quest::save_map(&mut conn.session);
                let item = crate::server::inventory::from_template(data, 46016, 2);
                let _ = conn.session.add_homdo_item(item);
                out.send(conn.session.dump_homdo());
                end_talk(conn, out);
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        }
        return;
    }

    // NPC 16015 — inn + gift item x2.
    if template == 16015 {
        match conn.session.select_menu {
            30 => {
                out.send("F44411001401000000010603010000000000000200");
            }
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => {
                out.send(crate::server::handlers::shops::open_npc_shop_frame());
                out.send("F44411001401000000010603010000000000000200");
            }
            33 => {
                crate::server::handlers::quest::save_map(&mut conn.session);
                let item = crate::server::inventory::from_template(data, 46016, 2);
                let _ = conn.session.add_homdo_item(item);
                out.send(conn.session.dump_homdo());
                end_talk(conn, out);
            }
            40 => end_talk(conn, out),
            _ => end_talk(conn, out),
        }
        return;
    }

    if template == 16012 {
        return;
    }

    // Daily quest map 12711 owns the whole context (21 RNG draws).
    if conn.session.map_id == 12711 {
        crate::server::handlers::quest::generate_daily_quest(conn, data, out);
        return;
    }

    end_talk(conn, out);
}

async fn handle_talk_click_door(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    // Bare `F4 44 02 00 14 04` (empty payload) is the client's end-talk frame.
    if payload.len() < 2 {
        end_talk(conn, out);
        return;
    }
    let door_id = encoder::u16_le(payload[0], payload[1]) as i32;
    if conn.session.id_leader > 0 && conn.session.id_leader != conn.session.id {
        end_talk(conn, out);
        return;
    }
    let mut rng = DotNetRandom::new(eve_rng_seed());
    if let Some(event_session) = npc_event::resolve_npc_event(
        &conn.session,
        data,
        NpcTrigger::ClickDoor(door_id),
        &mut rng,
    ) {
        let first_non_action = event_session.results.iter().find(|r| r.result_type != 0);
        let should_lock_ui = !matches!(first_non_action.map(|r| r.result_type), Some(2 | 3));
        let char_id = conn.session.id;
        conn.session.idtalking = door_id;
        conn.session.talk_type = "DOOR".to_string();
        conn.session.talk_count = 0;
        conn.session.current_event_session = Some(event_session);
        if should_lock_ui {
            conn.session.talk_locked = true;
            out.send("F44402000602");
            out.send(NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock));
        }
        execute_event_step_with_pool(&mut conn.session, data, pool, out).await;
        return;
    }
    end_talk(conn, out);
}

async fn handle_talk_warp(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    env: &crate::server::dispatcher::ServerEnv<'_>,
    out: &mut HandleOutcome,
) {
    if payload.len() >= 2 {
        conn.session.idtalking = encoder::u16_le(payload[0], payload[1]) as i32;
    }
    crate::server::handlers::quest::handle_warp_confirm(conn, data, env, out).await;
}

async fn handle_talk_select_menu(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    let Some(&choice) = payload.first() else {
        return;
    };

    // Stray menu select while the eve battle is in flight: ignore it rather
    // than fall into the legacy routing below (the parked battle result is
    // resumed by `battle::service::battle_ended`, not by Sub 9).
    let awaiting_battle = conn
        .session
        .current_event_session
        .as_ref()
        .is_some_and(|ev| ev.phase == EventPhase::AwaitingBattle);
    if awaiting_battle {
        tracing::debug!("ignoring menu select while awaiting eve battle");
        return;
    }

    // Eve menu path (Checkpoint 4): answer a pending Surface choice.
    let awaiting_choice = conn
        .session
        .current_event_session
        .as_ref()
        .is_some_and(|ev| ev.phase == EventPhase::AwaitingChoice);
    if awaiting_choice {
        // ChoiceCode 0 = "nothing selected yet" (wire default) — ignore it.
        if choice == 0 {
            tracing::debug!("ignoring menu select with default ChoiceCode 0");
            return;
        }
        // Valid TSurface choice codes from aLogin.exe callback:
        // 20 (0x14 close), 21 (0x15 confirm), 40 (0x28 cancel), 30..=39 (opt 0..9), 61.. (opt >= 10).
        if !matches!(choice, 20 | 21 | 40 | 30..=39 | 61..=u8::MAX) {
            tracing::warn!(
                choice,
                "invalid TSurface choice code in 0x14 09; closing talk safely"
            );
            end_talk(conn, out);
            return;
        }
        // Record the choice on the active session: `snapshot_state` reads it
        // back for the conditionClass=10 branch re-evaluation, and the queue
        // index skips the already-delivered Surface result.
        let (mx, my) = (conn.session.map_x, conn.session.map_y);
        let trigger_and_surface = conn
            .session
            .current_event_session
            .as_mut()
            .and_then(|ev| {
                ev.last_choice_code = i32::from(choice);
                ev.phase = EventPhase::Executing;
                ev.current_index = ev.current_index.saturating_add(1);
                let trigger = match ev.trigger_kind {
                    1 => NpcTrigger::ClickNpc(ev.npc_click_id),
                    4 => NpcTrigger::ClickDoor(ev.npc_click_id),
                    8 => NpcTrigger::MeetDoor(ev.npc_click_id),
                    16 => NpcTrigger::WalkStep { x: mx, y: my },
                    _ => return None,
                };
                Some((trigger, ev.last_surface_id))
            });
        conn.session.select_menu = i32::from(choice);

        let Some((trigger, surface_id)) = trigger_and_surface else {
            tracing::debug!("unsupported Eve trigger kind for menu select; ending talk");
            end_talk(conn, out);
            return;
        };
        // Re-resolve the NPC's event list against the updated snapshot with a
        // strict DialogChoice chain filter: only chains containing
        // conditionClass=10 matching (surface_id, choice) may win, preventing
        // unconditional/normal chains from overriding the player's menu choice.
        let mut rng = DotNetRandom::new(eve_rng_seed());
        let filter = |chain: &[crate::data::loaders::EveCondition]| {
            crate::eve::resolver::chain_has_dialog_choice(
                chain,
                surface_id,
                i32::from(choice),
            )
        };
        match npc_event::resolve_npc_event_with_filter(
            &conn.session,
            data,
            trigger,
            &mut rng,
            Some(&filter),
        ) {
            Some(mut branch) => {
                // Carry the answered Surface context into the branch session:
                // later snapshot reads (and auto-chain guard #2, "no Surface
                // right after answering") must still see the choice.
                if let Some(prev) = conn.session.current_event_session.as_ref() {
                    branch.last_surface_id = prev.last_surface_id;
                    branch.last_choice_code = prev.last_choice_code;
                }
                // The dialog frame and actor lock stay in place — deliver the
                // first step of the matched branch directly.
                conn.session.current_event_session = Some(branch);
                execute_event_step_with_pool(&mut conn.session, data, pool, out).await;
            }
            // No branch matches this choice (e.g. 40 cancel): close safely (unlock + 1408).
            None => end_talk(conn, out),
        }
        return;
    }

    // Legacy H6 path: store the raw choice byte for the talk-continue routing.
    conn.session.select_menu = i32::from(choice);
}
