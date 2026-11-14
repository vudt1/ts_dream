//! Eve EventRunner, Pre-Flight Probe, and typed DomainEffect executors.
//!
//! Separated from `talk.rs` (`Smell-04`) so `talk.rs` owns only `0x14`
//! transport/sub-opcode decoding while `eve_runner.rs` owns the event
//! state-machine loop, All-or-Nothing pre-flight validation, and typed
//! [`DomainEffect`] execution (`Smell-03`).

use crate::battle::rng::DotNetRandom;
use crate::battle::runner::Outcome;
use crate::data::loader::GameData;
use crate::data::loaders::EveResult;
use crate::db::pool::DbPool;
use crate::eve::auto_chain::{AutoChainResult, EventPhase, EventSession};
use crate::eve::planner::{DomainEffect, MoneyOp, PetOp, PlayerOp, QuestOp};
use crate::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use crate::protocol::encoder;
use crate::server::dispatcher::HandleOutcome;
use crate::server::handlers::npc_event;
use crate::server::handlers::quest_sync::{remove_quest_task, set_quest_dont, set_quest_task};
use crate::server::handlers::shops::gold_frame;
use crate::server::handlers::stats::build_stat_update;
use crate::server::handlers::talk::{end_talk_session, eve_rng_seed, reset_talk_context};
use crate::server::session::Session;

/// All-or-Nothing pre-flight rejection reasons before mutating session state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreFlightError {
    HomdoFull { item_id: u16 },
    InsufficientItem { item_id: u16 },
    QuestLedgerFull { quest_id: u16 },
    PetBagFull { pet_id: u16 },
    GoldCapExceeded,
    InsufficientGold,
    PointCapExceeded,
    InsufficientPoint,
}

/// Result of processing a single [`DomainEffect`] step.
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

/// All-or-Nothing pre-flight probe over `effects`.
///
/// Walks past non-branching `TalkStep` (`rt = 1`) so that chains like
/// `[TakeItem -> TalkStep -> RewardItem]` are validated All-or-Nothing up front
/// until the chain ends or hits a true branch/transition (`AwaitMenuChoice` `rt=6`,
/// `StartBattle` `rt=3`, `Warp` `rt=2`).
fn preflight_probe_batch(
    session: &Session,
    data: &GameData,
    effects: &[DomainEffect],
) -> Result<(), PreFlightError> {
    let mut trial_homdo = session.homdo.clone();
    let mut trial_quests = session.quest_tasks.clone();
    let mut trial_pets = session.pets.clone();
    let mut trial_gold = session.gold;
    let mut trial_point = session.point;
    let mut trial_shop_point = session.shop_point;

    for effect in effects {
        match effect {
            DomainEffect::AwaitMenuChoice { .. }
            | DomainEffect::StartBattle { .. }
            | DomainEffect::Warp { .. } => break,
            DomainEffect::TalkStep(_) => continue,
            DomainEffect::MutateItem {
                item_id,
                delta,
                wire_result: _,
            } => {
                if *delta < 0 {
                    let needed = delta.unsigned_abs();
                    let removed =
                        crate::server::inventory::remove_item(&mut trial_homdo, *item_id, needed);
                    if removed < needed {
                        return Err(PreFlightError::InsufficientItem { item_id: *item_id });
                    }
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
            DomainEffect::UpdateQuest(op) => match *op {
                QuestOp::Drop { quest_id } => {
                    trial_quests.remove(&quest_id);
                }
                QuestOp::Advance { quest_id, .. } | QuestOp::Subtract { quest_id, .. }
                    if quest_id != 0 =>
                {
                    if !trial_quests.contains_key(&quest_id) {
                        let used: std::collections::HashSet<u8> = trial_quests
                            .values()
                            .map(|(s, _)| *s)
                            .filter(|&s| {
                                (1..=crate::server::handlers::quest_sync::QUEST_SLOT_MAX)
                                    .contains(&s)
                            })
                            .collect();
                        let Some(free_slot) =
                            (1..=crate::server::handlers::quest_sync::QUEST_SLOT_MAX)
                                .find(|s| !used.contains(s))
                        else {
                            return Err(PreFlightError::QuestLedgerFull { quest_id });
                        };
                        trial_quests.insert(quest_id, (free_slot, 1));
                    }
                }
                _ => {}
            },
            DomainEffect::RewardPet(op) => match op {
                PetOp::Grant { pet_id, .. } => {
                    if !trial_pets.iter().any(|p| p.id == *pet_id) {
                        let Some(stt) = crate::server::pet_box::next_active_slot(&trial_pets)
                        else {
                            return Err(PreFlightError::PetBagFull { pet_id: *pet_id });
                        };
                        trial_pets.push(crate::server::session::PetState {
                            stt,
                            id: *pet_id,
                            ..Default::default()
                        });
                    }
                }
                PetOp::Revoke { pet_id, .. } => {
                    trial_pets.retain(|p| !(p.id == *pet_id && p.is_carried()));
                }
                PetOp::NoOp => {}
            },
            DomainEffect::MutateMoney(op) => match *op {
                MoneyOp::AddStatPoints(rewarded) => {
                    if u32::from(trial_point).saturating_add(rewarded) > u32::from(u16::MAX) {
                        return Err(PreFlightError::PointCapExceeded);
                    }
                    trial_point = trial_point.saturating_add(rewarded as u16);
                }
                MoneyOp::AddGold(add) => {
                    let next = u64::from(trial_gold).saturating_add(u64::from(add));
                    if next > u64::from(crate::protocol::GOLD_CAP) {
                        return Err(PreFlightError::GoldCapExceeded);
                    }
                    trial_gold = next as u32;
                }
                MoneyOp::DeductGold(abs) => {
                    if trial_gold < abs {
                        return Err(PreFlightError::InsufficientGold);
                    }
                    trial_gold -= abs;
                }
                MoneyOp::AddShopPoints(add) => {
                    let next = u64::from(trial_shop_point).saturating_add(u64::from(add));
                    if next > u64::from(crate::protocol::POINT_CAP) {
                        return Err(PreFlightError::PointCapExceeded);
                    }
                    trial_shop_point = next as u32;
                }
                MoneyOp::DeductShopPoints(abs) => {
                    if trial_shop_point < abs {
                        return Err(PreFlightError::InsufficientPoint);
                    }
                    trial_shop_point -= abs;
                }
                MoneyOp::NoOp | MoneyOp::Unsupported { .. } => {}
            },
            DomainEffect::PlayerEffect(PlayerOp::AddStatPoint(gained)) => {
                let gained_u32 = u32::from(*gained);
                if u32::from(trial_point).saturating_add(gained_u32) > u32::from(u16::MAX) {
                    return Err(PreFlightError::PointCapExceeded);
                }
                trial_point = trial_point.saturating_add(*gained);
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
        PreFlightError::InsufficientItem { item_id } => {
            tracing::debug!(item_id, "pre-flight rejected: insufficient item in homdo");
            out.send(crate::server::spawn::sys_msg_frame(
                "Vật phẩm không đủ, không thể thực hiện.",
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

fn execute_mutate_item(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    item_id: u16,
    delta: i32,
    wire_result: &EveResult,
) {
    if delta < 0 {
        session.remove_homdo_item(item_id, delta.unsigned_abs());
    } else if delta > 0 {
        let mut rem = delta as u32;
        while rem > 0 {
            let chunk = rem.min(crate::server::inventory::STACK_CAP as u32) as u8;
            let item = crate::server::inventory::from_template(data, item_id, chunk);
            session.add_homdo_item(item);
            rem -= u32::from(chunk);
        }
    }
    if delta != 0 {
        // Emit 0x14 01 so aLogin.exe displays item gain/loss toast, plus homdo sync (0x17 05).
        out.send(NpcTalkCodec::build_eve_result_hex(wire_result));
        out.send(session.dump_homdo());
        crate::server::session::registry_update_if_online(session);
    }
}

fn compute_quest_advance_step(current: Option<u8>, step: u8, battle_result: i32) -> u8 {
    match current {
        None => step.max(1),
        Some(cur) => {
            if matches!(battle_result, 2 | 3) {
                cur.saturating_sub(1)
            } else {
                cur.saturating_add(step)
            }
        }
    }
}

fn execute_update_quest(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    op: QuestOp,
) {
    let battle_result = session
        .current_event_session
        .as_ref()
        .map(|ev| ev.battle_result)
        .unwrap_or(0);

    match op {
        QuestOp::Decrement { quest_id, dec } => {
            if let Some((_slot, current)) = session.quest_tasks.get(&quest_id).copied() {
                let next_step = current.saturating_sub(dec);
                set_quest_task(session, out, quest_id, next_step);
                crate::server::session::registry_update_if_online(session);
            }
        }
        QuestOp::Advance {
            quest_id,
            step,
            mark_dont_on_step1,
        } => {
            let current = session.quest_tasks.get(&quest_id).map(|&(_, s)| s);
            let next_step = compute_quest_advance_step(current, step, battle_result);
            set_quest_task(session, out, quest_id, next_step);
            if mark_dont_on_step1 && step == 1 && !matches!(battle_result, 2 | 3) {
                if let Some(&mark) = data.quest_marks.get(&quest_id) {
                    if mark > 0 {
                        set_quest_dont(session, out, mark, 1);
                    }
                }
            }
            crate::server::session::registry_update_if_online(session);
        }
        QuestOp::Subtract { quest_id, step } => {
            let next_step = match session.quest_tasks.get(&quest_id) {
                None => 0,
                Some((_slot, current)) => current.saturating_sub(step),
            };
            set_quest_task(session, out, quest_id, next_step);
            crate::server::session::registry_update_if_online(session);
        }
        QuestOp::Drop { quest_id } => {
            remove_quest_task(session, out, quest_id);
            crate::server::session::registry_update_if_online(session);
        }
        QuestOp::ExplicitMark {
            quest_id,
            result_value,
        } => {
            if !matches!(battle_result, 2 | 3) {
                let target_qid = if quest_id == 2 && result_value >= 10000 {
                    result_value as u16
                } else {
                    quest_id
                };
                if let Some(&mark) = data.quest_marks.get(&target_qid) {
                    if mark > 0 {
                        set_quest_dont(session, out, mark, 1);
                    }
                }
            }
            crate::server::session::registry_update_if_online(session);
        }
        QuestOp::MarkDont { quest_id } => {
            if !matches!(battle_result, 2 | 3) {
                if let Some(&mark) = data.quest_marks.get(&quest_id) {
                    if mark > 0 {
                        set_quest_dont(session, out, mark, 1);
                    }
                }
            }
            crate::server::session::registry_update_if_online(session);
        }
        QuestOp::Unsupported {
            quest_id,
            p_style,
            result_value,
        } => {
            tracing::warn!(
                quest_id,
                p_style,
                val = result_value,
                "unmapped result_class=2 p_style; skipped"
            );
        }
    }
}

fn execute_mutate_money(session: &mut Session, out: &mut HandleOutcome, op: MoneyOp) {
    match op {
        MoneyOp::AddStatPoints(rewarded) => {
            let capped = u16::try_from(rewarded).unwrap_or(u16::MAX);
            session.point = session.point.saturating_add(capped);
            out.send(build_stat_update(0x26, i32::from(session.point)));
            crate::server::session::registry_update_if_online(session);
        }
        MoneyOp::AddGold(add) => {
            session.gold = session.gold.saturating_add(add).min(crate::protocol::GOLD_CAP);
            out.send(gold_frame(session.gold));
            out.send(NpcTalkCodec::build_gold_add_hex(add));
            crate::server::session::registry_update_if_online(session);
        }
        MoneyOp::DeductGold(sub) => {
            session.gold = session.gold.saturating_sub(sub);
            out.send(gold_frame(session.gold));
            out.send(NpcTalkCodec::build_gold_sub_hex(sub));
            crate::server::session::registry_update_if_online(session);
        }
        MoneyOp::AddShopPoints(add) => {
            session.shop_point = session
                .shop_point
                .saturating_add(add)
                .min(crate::protocol::POINT_CAP);
            out.send(NpcTalkCodec::build_shop_point_add_hex(add));
            crate::server::session::registry_update_if_online(session);
        }
        MoneyOp::DeductShopPoints(sub) => {
            session.shop_point = session.shop_point.saturating_sub(sub);
            out.send(NpcTalkCodec::build_shop_point_sub_hex(sub));
            crate::server::session::registry_update_if_online(session);
        }
        MoneyOp::NoOp => {}
        MoneyOp::Unsupported { p_style, amount } => {
            tracing::warn!(
                gold_type = p_style,
                amount,
                "unmapped rc=5 Gold/Point p_style; skipped"
            );
        }
    }
}

fn execute_player_op(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    op: PlayerOp,
) {
    match op {
        PlayerOp::SaveMap => {
            crate::server::handlers::quest::save_map(session);
            crate::server::session::registry_update_if_online(session);
        }
        PlayerOp::AddSkillPoint(gained) => {
            session.skill_point = session.skill_point.saturating_add(gained);
            out.send(build_stat_update(0x25, i32::from(session.skill_point)));
            out.send(NpcTalkCodec::build_skill_point_toast_hex(
                gained.min(255) as u8,
            ));
            crate::server::session::registry_update_if_online(session);
        }
        PlayerOp::AddStatPoint(gained) => {
            session.point = session.point.saturating_add(gained);
            out.send(build_stat_update(0x26, i32::from(session.point)));
            out.send(NpcTalkCodec::build_stat_point_toast_hex(
                gained.min(255) as u8,
            ));
            crate::server::session::registry_update_if_online(session);
        }
        PlayerOp::AddExp(exp_delta) => {
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
                    .clamp(1, crate::protocol::MAX_LEVEL) as u8;
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
        PlayerOp::GetArmy { p_style, value } => {
            tracing::debug!(
                army_type = p_style,
                value,
                "Eve army result skipped: no army store wired"
            );
        }
        PlayerOp::OpenNpcShop { save_map } => {
            out.send(crate::server::handlers::shops::open_npc_shop_frame());
            if save_map {
                crate::server::handlers::quest::save_map(session);
                crate::server::session::registry_update_if_online(session);
            }
        }
        PlayerOp::Unsupported {
            parameter,
            p_style,
            result_value,
        } => {
            tracing::warn!(
                param = parameter,
                style = p_style,
                val = result_value,
                "unmapped rc=7 PlayerEffect; skipped"
            );
        }
    }
}

fn execute_reward_pet(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    op: &PetOp,
) {
    match op {
        PetOp::Grant {
            pet_id,
            wire_result,
        } => {
            let pet_id = *pet_id;
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
            out.send(NpcTalkCodec::build_talk_step_hex(wire_result));
            out.send(stat_frame);
            out.send(NpcTalkCodec::build_pet_appear_hex(session.id, stt, pet_id, 1));
            crate::server::session::registry_update_if_online(session);
        }
        PetOp::Revoke {
            pet_id,
            wire_result,
        } => {
            let pet_id = *pet_id;
            let Some(pos) = session
                .pets
                .iter()
                .position(|p| p.id == pet_id && p.is_carried())
            else {
                tracing::debug!(pet_id, "RewardPet remove skipped: player does not carry pet");
                return;
            };
            let stt = session.pets[pos].stt;
            session.pets.remove(pos);
            if session.active_pet_stt == stt {
                session.active_pet_stt = 0;
            }
            let lo = u16::from(stt) * 10 + 1;
            let hi = u16::from(stt) * 10 + 6;
            session
                .trangbi
                .retain(|i| !(lo..=hi).contains(&u16::from(i.slot)));
            out.send(NpcTalkCodec::build_talk_step_hex(wire_result));
            out.send(NpcTalkCodec::build_pet_remove_hex(session.id, stt));
            crate::server::session::registry_update_if_online(session);
        }
        PetOp::NoOp => {}
    }
}

fn process_single_domain_effect(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    effect: &DomainEffect,
) -> StepOutcome {
    match effect {
        DomainEffect::TalkStep(res) => {
            out.send(NpcTalkCodec::build_talk_step_hex(res));
            StepOutcome::Halt
        }
        DomainEffect::AwaitMenuChoice { surface_id, result } => {
            if let Some(ev) = session.current_event_session.as_mut() {
                ev.last_surface_id = *surface_id;
                ev.phase = EventPhase::AwaitingChoice;
            }
            out.send(NpcTalkCodec::build_talk_step_hex(result));
            StepOutcome::Halt
        }
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
        DomainEffect::MutateItem {
            item_id,
            delta,
            wire_result,
        } => {
            execute_mutate_item(session, data, out, *item_id, *delta, wire_result);
            StepOutcome::Continue
        }
        DomainEffect::UpdateQuest(op) => {
            execute_update_quest(session, data, out, *op);
            StepOutcome::Continue
        }
        DomainEffect::MutateMoney(op) => {
            execute_mutate_money(session, out, *op);
            StepOutcome::Continue
        }
        DomainEffect::PlayerEffect(op) => {
            execute_player_op(session, data, out, *op);
            StepOutcome::Continue
        }
        DomainEffect::Warp { .. } => StepOutcome::Halt,
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
        DomainEffect::RewardPet(op) => {
            execute_reward_pet(session, data, out, op);
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
            out.send(NpcTalkCodec::build_talk_lock_hex(
                char_id,
                TalkLockMode::Unlock,
            ));
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
    if finish_event_session_iter(session, data, out) {
        return true;
    }
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

/// Execute trailing non-blocking effects after a `Warp` and record `(scene_id, eve_no)` completion.
fn run_trailing_effects_before_warp(
    session: &mut Session,
    data: &GameData,
    out: &mut HandleOutcome,
    effects: &[DomainEffect],
) {
    for eff in effects.iter().skip(1) {
        if eff.is_blocking_or_terminal() {
            break;
        }
        match process_single_domain_effect(session, data, out, eff) {
            StepOutcome::Halt => break,
            StepOutcome::Continue => {}
            StepOutcome::SkillReward {
                skill_id,
                target_level,
                sp: _,
                name,
            } => {
                apply_skill_reward_to_session(session, out, skill_id, target_level, &name);
            }
        }
    }
    if let Some(completed) = session.current_event_session.as_ref() {
        if completed.has_state_changing_results() {
            let scene_id = u16::try_from(completed.map_id).unwrap_or(session.map_id);
            *session
                .completed_eve_counts
                .entry((scene_id, completed.eve_no))
                .or_insert(0) += 1;
        }
    }
}

/// Unified `async` EventRunner loop (`pool: Option<&DbPool>`, optional `defer_warp`).
async fn execute_event_step_core(
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

        if let DomainEffect::Warp { warp_id } = effects[0] {
            let warp_key = (i64::from(session.map_id), i64::from(warp_id));
            if !data.warps.contains_key(&warp_key) {
                tracing::debug!(
                    map_id = session.map_id,
                    warp_id,
                    "Eve Door result has no Warps.dat row; skipping"
                );
                if let Some(ev) = session.current_event_session.as_mut() {
                    ev.current_index = ev.current_index.saturating_add(1);
                }
                continue;
            }

            run_trailing_effects_before_warp(session, data, out, &effects);

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

        match process_single_domain_effect(session, data, out, &effects[0]) {
            StepOutcome::Halt => {
                if out.eve_battle.is_some() && session.db_character_id > 0 {
                    if !crate::db::persist::persist_sessions_transaction(pool, &[session], &[])
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

fn run_sync_future<F: std::future::Future + Send>(fut: F) -> F::Output
where
    F::Output: Send,
{
    let exec = || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("current_thread tokio runtime for sync Eve wrapper")
            .block_on(fut)
    };
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|s| s.spawn(exec).join().expect("sync Eve wrapper thread"))
    } else {
        exec()
    }
}

/// Thin synchronous wrapper around [`execute_event_step_core`] (`pool = None`) for sync callers/tests.
pub fn execute_event_step(session: &mut Session, data: &GameData, out: &mut HandleOutcome) {
    run_sync_future(execute_event_step_core(session, data, None, out, false));
}

pub async fn execute_event_step_with_pool(
    session: &mut Session,
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    execute_event_step_core(session, data, pool, out, false).await;
}

/// Activate an Eve `EventSession`, lock UI if the first non-Action step is not
/// `Warp`/`StartBattle`, and execute the initial step batch.
pub async fn activate_event_session(
    session: &mut Session,
    event_session: EventSession,
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
    let first_non_action = event_session.results.iter().find(|r| r.result_type != 0);
    let should_lock_ui = !matches!(first_non_action.map(|r| r.result_type), Some(2 | 3));
    let char_id = session.id;
    session.current_event_session = Some(event_session);
    if should_lock_ui {
        session.talk_locked = true;
        out.send(NpcTalkCodec::build_move_stop_hex());
        out.send(NpcTalkCodec::build_talk_lock_hex(
            char_id,
            TalkLockMode::Lock,
        ));
    }
    execute_event_step_with_pool(session, data, pool, out).await;
}

/// Advance a parked Eve event session (`phase == AwaitingBattle`) after its scripted battle ended.
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
    execute_event_step_core(session, data, pool, &mut out, defer_warp).await;
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
    run_sync_future(resume_eve_after_battle_full(session, outcome, data, None))
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
