//! Quest engine (Ticket 19): H6 data-driven table, daily-quest generator,
//! pet-reborn exceptions, quest requirement checking, TEAMDEF battle trigger,
//! and quest result processing (OnWin/OnLose rewards).

use crate::battle::packets;
use crate::battle::rng::DotNetRandom;
use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::dispatcher::HandleOutcome;
use crate::server::handlers::stats::build_stat_update;
use crate::server::handlers::talk::{end_talk, talk_messages};
use crate::server::session::{Conn, Session};
use crate::server::spawn::sys_msg_frame;
use std::sync::Arc;

/// Canonical quest key (ticket 19): `{map_id, talk_type, map_object_id, step}`.
///
/// `talk_type` is `"NPC"` or `"WARP"`; `map_object_id` is the on-map instance
/// (`idtalking`), NOT the NPC template id; `step` comes from the quest table.
/// A bare `talking_battle: i32` is never sufficient to disambiguate these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestKey {
    pub map_id: i64,
    pub talk_type: String,
    pub map_object_id: i64,
    pub step: i64,
}

impl QuestKey {
    pub fn new(map_id: i64, talk_type: impl Into<String>, map_object_id: i64, step: i64) -> Self {
        Self {
            map_id,
            talk_type: talk_type.into(),
            map_object_id,
            step,
        }
    }

    /// The `Data_Talks` HashMap key (`mapId:Type:Id:Step`).
    pub fn to_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.map_id, self.talk_type, self.map_object_id, self.step
        )
    }
}

/// Build the successful-format quest key string (used by data lookups).
pub fn quest_key(map_id: i64, talk_type: &str, map_object_id: i64, step: i64) -> String {
    QuestKey::new(map_id, talk_type, map_object_id, step).to_key()
}

/// The current quest step for a given talk object id (the `player_id`-scoped
/// quest snapshot); `0` when no quest row exists yet.
pub fn step_for_object(conn: &Session, map_object_id: i64) -> i64 {
    conn.quest_steps
        .iter()
        .find(|(npc, _)| *npc == map_object_id)
        .map(|(_, step)| *step)
        .unwrap_or(0)
}

/// The current quest step for the session's NPC talk (`player_id`-scoped);
/// `0` when no quest row exists yet.
pub fn current_step(conn: &Conn) -> i64 {
    step_for_object(&conn.session, i64::from(conn.session.idtalking))
}

/// Set the pending `talking_battle` context and emit a battle trigger.
pub fn trigger_teamdef(conn: &mut Conn, teamdef: &[i64], out: &mut HandleOutcome) {
    if teamdef.is_empty() || teamdef.iter().sum::<i64>() <= 0 {
        return;
    }
    conn.session.talking_battle = conn.session.idtalking;
    out.battle_trigger = Some(BattleTrigger {
        teamdef: teamdef.to_vec(),
        diahinh: teamdef.first().copied().unwrap_or(112) as i32,
    });
}

/// `savemap` canonical seam: refresh the session's respawn point from the
/// current map. Deferred drift: the `savemap` column is not persisted (in-memory
/// only), so a server restart loses the saved respawn point.
pub fn save_map(conn: &mut Conn) {
    conn.session.savemap = conn.session.map_id;
}

/// Attempt the data-driven quest path for H6 continue.
///
/// Returns `true` if the quest data was found and handled, `false` if the
/// caller should fall back to the hard-coded NPC paths.
pub fn try_quest_h6(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) -> bool {
    let select_menu = conn.session.select_menu;
    let map_id = i64::from(conn.session.map_id);
    let map_object_id = i64::from(conn.session.idtalking);

    // Pet-reborn exceptions keyed `(map, object)` -> compiled table behavior.
    if is_pet_reborn_key(map_id, map_object_id) {
        return handle_pet_reborn_npc(conn, map_id, map_object_id, out);
    }

    // Build the talk key from the full QuestKey (map, type, object, step).
    let step = current_step(conn);
    let key = quest_key(map_id, "NPC", map_object_id, step);

    let quest = match data.talks.get(&key) {
        Some(q) => q,
        // Fall back to step-less lookup only for legacy keys (see ticket 19
        // note: the loader keys step 0 for old fixtures).
        None => match data.talks.get(&quest_key(map_id, "NPC", map_object_id, 0)) {
            Some(q) => q,
            None => return false,
        },
    };

    // Requirement gate: Level/Reborn/Thuoctinh/Quests/Wears/Items.
    if let Some(fail) = evaluate_requirements(conn, quest) {
        out.send(fail);
        end_talk(conn, out);
        conn.session.select_menu = 40;
        return true;
    }

    // If TEAMDEF exists and dialogs are empty → trigger battle
    if quest.dialogs.is_empty() && !quest.teamdef.is_empty() {
        let sum: i64 = quest.teamdef.iter().sum();
        if sum > 0 {
            conn.session.talking_battle = map_object_id as i32;
            out.battle_trigger = Some(BattleTrigger {
                teamdef: quest.teamdef.clone(),
                diahinh: quest.teamdef.first().copied().unwrap_or(112) as i32,
            });
            return true;
        }
    }

    // Send dialog messages
    if !quest.dialogs.is_empty() {
        let is_menu = quest.dialogs.contains("F444110014010000000106");
        if is_menu
            && quest.require_select_menu > 0
            && select_menu != quest.require_select_menu as i32
        {
            let lose = &quest.on_lose.dialogs;
            if !lose.is_empty() {
                talk_messages(conn, lose, out);
                conn.session.select_menu = 40;
            } else {
                end_talk(conn, out);
            }
            return true;
        }
        talk_messages(conn, &quest.dialogs, out);
    }

    // If dialogs exhaust and TEAMDEF exists → battle
    if select_menu == 40 && !quest.teamdef.is_empty() {
        let sum: i64 = quest.teamdef.iter().sum();
        if sum > 0 {
            conn.session.talking_battle = map_object_id as i32;
            out.battle_trigger = Some(BattleTrigger {
                teamdef: quest.teamdef.clone(),
                diahinh: quest.teamdef.first().copied().unwrap_or(112) as i32,
            });
            return true;
        }
    }

    true
}

/// Evaluate `[REQUIRES]` conditions (Level/Reborn/Thuoctinh/Quests/Wears/Items)
/// against the session. Returns the failure packet or `None` (pass).
///
/// The failure packet follows §2.6.3:
/// - missing item -> `F444110014010000000101070000000000000077A7`;
/// - a `Dict`-style id>0 fail -> `F44411001401000000020103`+id+`…BB`.
pub fn evaluate_requirements(conn: &Conn, quest: &crate::data::tables::QuestDef) -> Option<String> {
    // Level requirement.
    if let Some((value, op)) = quest.require_level {
        if !cmp_op(i64::from(conn.session.level), value, op) {
            return Some("F444110014010000000101070000000000000077A7".to_string());
        }
    }
    if let Some((value, op)) = quest.require_reborn {
        if !cmp_op(i64::from(conn.session.reborn), value, op) {
            return Some("F444110014010000000101070000000000000077A7".to_string());
        }
    }
    if quest.require_thuoctinh > 0 && conn.session.thuoctinh != quest.require_thuoctinh as u8 {
        return Some("F444110014010000000101070000000000000077A7".to_string());
    }
    // Require quests: every listed `(map, npc, warp, step)` must be completed
    // (a WARP-tagged quest is satisfied by its warp id; an NPC one by its
    // object id — both scoped to the map). `completed_quests` is populated on
    // battle win in `battle_quest_win_impl`.
    for &(map, npc, warp, _step) in &quest.require_quests {
        let done = conn.session.completed_quests.contains(&(map, npc))
            || (warp > 0 && conn.session.completed_quests.contains(&(map, warp)));
        if !done {
            return Some("F444110014010000000101070000000000000077A7".to_string());
        }
    }
    // Require wears: one equipped item (`trangbi`) per `(itemId, playerOrPet)`;
    // the equip table is the same registered-gear source for both targets.
    for &(item_id, _target) in &quest.require_wears {
        let worn = conn
            .session
            .trangbi
            .iter()
            .any(|i| i.id == item_id as u16 && i.count > 0);
        if !worn {
            return Some("F444110014010000000101070000000000000077A7".to_string());
        }
    }
    // Require items to possess at entry: the
    // `[OnWin] RequireItems` tuples read from the `[REQUIRES].Items` line.
    for &(item_id, count, _remove) in &quest.on_win.require_items {
        if item_id <= 0 || count <= 0 {
            continue;
        }
        let have: u32 = conn
            .session
            .homdo
            .iter()
            .filter(|i| i.id == item_id as u16)
            .map(|i| u32::from(i.count))
            .sum();
        if have < count as u32 {
            return Some("F444110014010000000101070000000000000077A7".to_string());
        }
    }
    None
}

/// Operator comparison for `[REQUIRES]` conditions (`0` `=`, `1` `>=`, `2` `>`,
/// `3` `<=`, `4` `<`, `5` `!=`).
fn cmp_op(a: i64, b: i64, op: i64) -> bool {
    match op {
        0 => a == b,
        1 => a >= b,
        2 => a > b,
        3 => a <= b,
        4 => a < b,
        5 => a != b,
        _ => true, // unknown operator: treated as no-op
    }
}

/// Full ordered battle-win side effects (spec §6.7).
///
/// Runs when a quest/TeamDef battle ends with a player win and the leader has
/// a pending `talking_battle`. `member` resolves a party member's shared
/// session (for `shareToParty` grants and member warps); return `None` for
/// offline/unregistered members. All emitted server→client frames are appended
/// to `frames`. Returns `true` if the talk existed and was processed.
pub fn battle_quest_win(
    session: &mut Session,
    data: &GameData,
    frames: &mut Vec<String>,
    member: &mut dyn FnMut(i64) -> Option<Arc<tokio::sync::RwLock<Session>>>,
) -> bool {
    let idtalking = session.talking_battle;
    if idtalking <= 0 {
        return false;
    }
    let quest = resolve_quest_for_session(session, i64::from(idtalking), data);
    battle_quest_win_impl(session, data, frames, member, quest)
}

/// Same OnWin runner as [`battle_quest_win`], but resolved against the *current
/// talk* NPC (`idtalking`) — the entry used by the player-reborn flow.
pub fn battle_quest_win_talk(
    session: &mut Session,
    idtalking: i32,
    data: &GameData,
    frames: &mut Vec<String>,
    member: &mut dyn FnMut(i64) -> Option<Arc<tokio::sync::RwLock<Session>>>,
) -> bool {
    if idtalking <= 0 {
        return false;
    }
    let quest = resolve_quest_for_session(session, i64::from(idtalking), data);
    battle_quest_win_impl(session, data, frames, member, quest)
}

/// Resolve the `Data_Talks` entry for a battle outcome from the session's full
/// quest key `{map_id, talk_type, map_object_id, step}` — never a bare
/// integer. A step-aware lookup is tried first, then the step-0 fallback used
/// by legacy fixtures.
fn resolve_quest_for_session<'a>(
    session: &Session,
    map_object_id: i64,
    data: &'a GameData,
) -> Option<&'a crate::data::tables::QuestDef> {
    let map_id = i64::from(session.map_id);
    let step = step_for_object(session, map_object_id);
    let typed = quest_key(map_id, &session.talk_type, map_object_id, step);
    let npc0 = quest_key(map_id, "NPC", map_object_id, 0);
    let warp = format!("{}:WARP:{}:{}", map_id, map_object_id, step);
    data.talks
        .get(&typed)
        .or_else(|| data.talks.get(&npc0))
        .or_else(|| data.talks.get(&warp))
}

fn battle_quest_win_impl(
    session: &mut Session,
    data: &GameData,
    frames: &mut Vec<String>,
    member: &mut dyn FnMut(i64) -> Option<Arc<tokio::sync::RwLock<Session>>>,
    quest: Option<&crate::data::tables::QuestDef>,
) -> bool {
    let Some(quest) = quest else {
        return false;
    };
    let result = &quest.on_win;

    // Win dialogs take precedence over the reward pipeline.
    if !result.dialogs.is_empty() {
        for part in result.dialogs.split("F444") {
            if !part.is_empty() {
                frames.push(format!("F444{part}"));
            }
        }
        frames.push("F44402001408".to_string()); // EndTalk after dialogs
        mark_quest_done(session);
        clear_quest_talk(session);
        return true;
    }

    // 1. Consume required items (`_RequireItems`).
    for &(item_id, count, remove) in &result.require_items {
        if remove <= 0 {
            continue;
        }
        let have: u32 = session
            .homdo
            .iter()
            .filter(|i| i.id == item_id as u16)
            .map(|i| u32::from(i.count))
            .sum();
        if have >= count.max(0) as u32 {
            session.remove_homdo_item(item_id as u16, remove.max(0) as u32);
        }
    }

    // 2. Red message (`_Message`).
    if !result.message.is_empty() {
        frames.push(crate::server::spawn::sys_msg_frame(&result.message));
    }

    // 3 + 4. Guaranteed `WinRewards` + one random `WinRandomRewards` via a
    // fresh independent time-seeded RNG (NOT the battle streams).
    let mut list: Vec<(i64, i64, i64)> = result.rewards.clone();
    if !result.random_rewards.is_empty() {
        let mut rng = DotNetRandom::time_seeded();
        let idx = rng.next_range(0, result.random_rewards.len() as i32) as usize;
        list.push(result.random_rewards[idx]);
    }

    // 5. Grant {item, count} to leader + (shareToParty) each member.
    for &(item_id, count, share) in &list {
        if item_id <= 0 || count <= 0 {
            continue;
        }
        let _ = session.add_homdo_item({
            let mut it =
                crate::server::inventory::from_template(data, item_id as u16, count.min(255) as u8);
            it.doben = 100;
            it
        });
        if share > 0 {
            for mem in session.id_mem.iter().filter(|m| **m > 0) {
                if let Some(m) = member(i64::from(*mem)) {
                    if let Ok(mut s) = m.try_write() {
                        let _ = s.add_homdo_item({
                            let mut it = crate::server::inventory::from_template(
                                data,
                                item_id as u16,
                                count.min(255) as u8,
                            );
                            it.doben = 100;
                            it
                        });
                    }
                }
            }
        }
    }

    // 6. Use items (`_WinUseItems`): self → equip packet + consume + recompute;
    // else the active-pet path.
    for &(item_id, target) in &result.use_items {
        if item_id <= 0 {
            continue;
        }
        let slot = session
            .homdo
            .iter()
            .find(|i| i.id == item_id as u16 && i.count > 0)
            .map(|i| i.slot);
        let Some(slot) = slot else { continue };
        if target == 0 {
            // Self: `F44403001711`+slot, equip broadcast, consume, recompute.
            frames.push(format!("F44403001711{:02X}", slot));
            frames.push(format!(
                "F44408000502{}{}",
                encoder::le32(session.id),
                encoder::le16(item_id as u16)
            ));
            session.remove_homdo_item(item_id as u16, 1);
            // Capture pre-recompute HP/SP so the status burst can resync the
            // client when an old value exceeds its new max.
            let old_hp = session.hp;
            let old_sp = session.sp;
            session.recompute_stats();
            // Wire-fidelity status burst: Int2/Atk2/Def2/Hpx2/Spx2/Agi2
            // (always) then Hp/Sp only when the old value exceeded the freshly
            // recomputed max. Hpmax/Spmax are client-only stores that emit no
            // packet.
            frames.push(build_stat_update(0xD4, session.int2 as i32));
            frames.push(build_stat_update(0xD2, session.atk2 as i32));
            frames.push(build_stat_update(0xD3, session.def2 as i32));
            frames.push(build_stat_update(0xCF, session.hpx2 as i32));
            frames.push(build_stat_update(0xD0, session.spx2 as i32));
            frames.push(build_stat_update(0xD6, session.agi2 as i32));
            if old_hp > session.hp_max {
                frames.push(build_stat_update(0x19, session.hp_max as i32));
            }
            if old_sp > session.sp_max {
                frames.push(build_stat_update(0x1A, session.sp_max as i32));
            }
        } else {
            // Pet path: `F44404001717`+stt+slot, consume, apply item to pet.
            let stt = session.active_pet_stt;
            if stt > 0 && session.pets.iter().any(|p| p.stt == stt) {
                frames.push(format!("F44404001717{:02X}{:02X}", stt, slot));
                session.remove_homdo_item(item_id as u16, 1);
                if let Some(item) = data.items.get(&item_id) {
                    if let Some(pet) = session.pets.iter_mut().find(|p| p.stt == stt) {
                        pet.atk = pet.atk.saturating_add(item.atk1.max(0) as u16);
                        pet.def = pet.def.saturating_add(item.def1.max(0) as u16);
                        pet.int1 = pet.int1.saturating_add(item.int1.max(0) as u16);
                        pet.hpx = pet.hpx.saturating_add(item.hpx1.max(0) as u16);
                        pet.spx = pet.spx.saturating_add(item.spx1.max(0) as u16);
                        pet.agi = pet.agi.saturating_add(item.agi1.max(0) as u16);
                        pet.fai = pet.fai.saturating_add(item.fai1.max(0) as u16);
                    }
                }
            }
        }
    }

    // 7. Save leader quests (`_WinSaveLeaderQuests`).
    for &(npc, npc_val, warp_val, plus) in &result.save_leader_quests {
        if npc_val > 0 {
            session.quest_steps.push((npc, npc_val));
        }
        if warp_val > 0 {
            session.warp_steps.push((npc, warp_val));
        }
        let _ = plus;
    }

    // 8. Player enhance delta (`_WinPlayerEnhanceData`).
    for (stat, delta) in &result.player_enhance_data {
        match stat.as_str() {
            "Point" => {
                session.point = (i64::from(session.point) + delta).clamp(0, 0xFFFF) as u16;
                frames.push(build_stat_update(0x26, i32::from(session.point)));
            }
            "SkillPoint" => {
                session.skill_point =
                    (i64::from(session.skill_point) + delta).clamp(0, 0xFFFF) as u16;
                frames.push(build_stat_update(0x25, i32::from(session.skill_point)));
            }
            _ => {}
        }
    }

    // 9. Add skill (`_WinAddSkill`) — learn packet + skillpoint + red message.
    for &(skill_id, lv) in &result.add_skill {
        let known = session
            .skills
            .iter()
            .any(|&(sid, _)| sid == skill_id as u16);
        if data.skills.contains_key(&skill_id) && skill_id > 0 && !known {
            session.skills.push((skill_id as u16, lv.min(255) as u8));
            let skill_name = &data.skills[&skill_id].name;
            frames.push(sys_msg_frame(&format!("Hoc duoc ky nang {}", skill_name)));
            frames.push(format!(
                "F4440C0008016E01{}{}",
                encoder::le32(lv.clamp(0, u32::MAX as i64) as u32),
                encoder::le32(skill_id.clamp(0, u32::MAX as i64) as u32)
            ));
            frames.push(format!(
                "F4440C0008012501{}00000000",
                encoder::le32(u32::from(session.skill_point))
            ));
        }
    }

    // 10. Add pet (`_WinAddPet`).
    for &pet_id in &result.add_pet {
        if pet_id > 0 {
            add_pet_to_quest(session, pet_id as u16);
        }
    }

    // 11. Warp/end (`_WinWarpTo` → Warped leader + members; else EndTalk).
    session.click_npc_id = result.click_npc_id as i32;
    let warp = &result.warp_to;
    if warp.first().copied().unwrap_or(0) > 0 {
        let map = warp.first().copied().unwrap_or(0) as u16;
        let x = warp.get(1).copied().unwrap_or(0) as u16;
        let y = warp.get(2).copied().unwrap_or(0) as u16;
        warp_leader(session, map, x, y, frames);
        for mem in session.id_mem.iter().filter(|m| **m > 0) {
            if let Some(m) = member(i64::from(*mem)) {
                if let Ok(mut s) = m.try_write() {
                    warp_member(&mut s, map, x, y, i64::from(session.id), frames);
                }
            }
        }
    } else {
        frames.push("F44402001408".to_string());
    }

    mark_quest_done(session);
    clear_quest_talk(session);
    true
}

/// Record `(map, object)` as a completed quest — the `[REQUIRES] Quests` gate
/// source. A WARP talk's object id is the warp id, so the same pair satisfies
/// both an NPC and a WARP requirement on that map.
fn mark_quest_done(session: &mut Session) {
    if session.talking_battle <= 0 {
        return;
    }
    let key = (i64::from(session.map_id), i64::from(session.talking_battle));
    if !session.completed_quests.contains(&key) {
        session.completed_quests.push(key);
    }
}

fn add_pet_to_quest(session: &mut Session, pet_id: u16) {
    if session.pets.iter().any(|p| p.id == pet_id) {
        return;
    }
    let stt = (session.pets.len() as u8 + 1).max(1);
    let hp_max = crate::battle::engine::get_hp_max(0, 0, 1, 0) as u16;
    session.pets.push(crate::server::session::PetState {
        stt,
        id: pet_id,
        level: 1,
        thuoctinh: 1,
        hp: hp_max,
        hp_max,
        ..Default::default()
    });
}

fn warp_leader(session: &mut Session, map: u16, x: u16, y: u16, frames: &mut Vec<String>) {
    let old_id = session.id;
    session.map_id = map;
    session.map_x = x;
    session.map_y = y;
    // Warp start + the 0x0C goto-map frame + hide on the old map.
    frames.push("F44402001407".to_string());
    frames.push(format!(
        "F4440D000C{}{}{}{}0000",
        encoder::le32(old_id),
        encoder::le16(map),
        encoder::le16(x),
        encoder::le16(y)
    ));
    frames.push(packets::hide_from_map(old_id));
    frames.push("F44402000504".to_string());
}

fn warp_member(
    session: &mut Session,
    map: u16,
    x: u16,
    y: u16,
    leader: i64,
    frames: &mut Vec<String>,
) {
    let id = session.id;
    session.map_id = map;
    session.map_x = x;
    session.map_y = y;
    frames.push(format!("F4440700142C{}01", encoder::le32(leader as u32)));
    frames.push(format!(
        "F4440D000C{}{}{}{}0000",
        encoder::le32(id),
        encoder::le16(map),
        encoder::le16(x),
        encoder::le16(y)
    ));
    frames.push(packets::hide_from_map(id));
    frames.push("F44402000504".to_string());
}

fn clear_quest_talk(session: &mut Session) {
    session.talking_battle = 0;
    session.idtalking = 0;
    session.select_menu = 0;
}

/// Emit the OnLose dialog progression for a battle defeat (frames vector form
/// used by the battle sink; the request-time path is `end_talk` free).
///
/// Scope (ticket 19 review #8): OnLose is dialog progression + EndTalk — not
/// the WinRewards pipeline. The key is resolved through the session's full
/// QuestKey (map/type/object/step).
pub fn quest_lose_frames(session: &mut Session, data: &GameData, frames: &mut Vec<String>) {
    let object = session.talking_battle;
    if object <= 0 {
        return;
    }
    if let Some(quest) = resolve_quest_for_session(session, i64::from(object), data) {
        if !quest.on_lose.dialogs.is_empty() {
            for part in quest.on_lose.dialogs.split("F444") {
                if !part.is_empty() {
                    frames.push(format!("F444{part}"));
                }
            }
        }
    }
    frames.push("F44402001408".to_string());
    clear_quest_talk(session);
}

/// Daily quest generator (map 12711, 21 RNG draws — §2.6.2 / research 06 §(6)).
///
/// Uses a fresh time-seeded Random (NOT the battle streams).
/// Exactly 21 draws consumed in order, even if unused. The menu
/// actions are data-driven: `add pet from item`, `exchange 65517 ×20 → skill
/// book`, `level/skillpoint boosts` — keyed by `(idtalking, select_menu)` per
/// the H6 table.
pub fn generate_daily_quest(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) {
    let mut rng = DotNetRandom::time_seeded();

    // 21 draws in exact order
    let num3 = rng.next_range(0, 7); // 1: 0..6
    let num4 = rng.next_range(0, 6); // 2: 0..5
    let _num5 = rng.next_range(0, 4); // 3: 0..3
    let _num6 = rng.next_range(0, 9); // 4: 0..8
    let _num7 = rng.next_range(0, 150); // 5: 0..149

    let _id = rng.next_range(47028, 47369); // 6
    let _id2 = rng.next_range(48031, 48104); // 7
    let _id3 = rng.next_range(47028, 47369); // 8
    let _id4 = rng.next_range(48031, 48104); // 9
    let _id5 = rng.next_range(47028, 47369); // 10
    let _id6 = rng.next_range(61029, 61091); // 11
    let _id7 = rng.next_range(61097, 61223); // 12
    let _id8 = rng.next_range(61029, 61091); // 13
    let _id9 = rng.next_range(61097, 61223); // 14
    let _id10 = rng.next_range(46184, 46204); // 15
    let _i_d = rng.next_range(62838, 62845); // 16
    let _i_d2 = rng.next_range(46900, 46907); // 17
    let _id11 = rng.next_range(14283, 14286); // 18
    let _num8 = rng.next_range(46395, 46399); // 19
    let _i_d3 = rng.next_range(46395, 46399); // 20
    let _num10 = rng.next_range(0, 7); // 21

    // Reward item formulas (used by the exchange branches below).
    let item_a = 62001 + num3 * 100; // slot-A equipment
    let _item_b = 62002 + num3 * 100;
    let item_c = 62101 + num4 * 100; // 65517-exchange book
    let _ = item_c;

    let idtalking = conn.session.idtalking;
    let select_menu = conn.session.select_menu;

    // Map 12711 — pet-shop rows.
    if idtalking == 1 {
        let pet_items = [
            (30, 31044, 18016),
            (30, 31138, 18020),
            (31, 31094, 18017),
            (31, 31184, 18046),
            (32, 31095, 18018),
            (32, 31185, 18047),
            (33, 31096, 18019),
            (33, 31186, 18048),
        ];
        for &(menu, item_id, pet_id) in &pet_items {
            if select_menu == menu {
                let has = conn.session.homdo.iter().any(|i| i.id == item_id as u16);
                if has {
                    conn.session.pets.push(crate::server::session::PetState {
                        stt: conn.session.pets.len() as u8 + 1,
                        id: pet_id as u16,
                        ..Default::default()
                    });
                    crate::server::inventory::remove_item(
                        &mut conn.session.homdo,
                        item_id as u16,
                        1,
                    );
                    out.send(conn.session.dump_homdo());
                    end_talk(conn, out);
                }
            }
        }
    }
    if idtalking == 6 || idtalking == 7 {
        // `65517 ×10` for each 62705..62712 medal item.
        let medal = match (idtalking, select_menu) {
            (6, 30) => Some(62705),
            (6, 31) => Some(62706),
            (6, 32) => Some(62707),
            (6, 33) => Some(62708),
            (7, 30) => Some(62709),
            (7, 31) => Some(62710),
            (7, 32) => Some(62711),
            (7, 33) => Some(62712),
            _ => None,
        };
        if let Some(medal_item) = medal {
            let has = conn.session.homdo.iter().any(|i| i.id == medal_item as u16);
            if has {
                conn.session
                    .add_homdo_item(crate::server::inventory::from_template(data, 65517, 10));
                crate::server::inventory::remove_item(
                    &mut conn.session.homdo,
                    medal_item as u16,
                    1,
                );
                out.send(conn.session.dump_homdo());
                end_talk(conn, out);
            }
        }
    }
    // idtalking 8..10: exchange 65517×20 for a skill book (6210x + num4*100).
    if matches!(idtalking, 8..=10) && select_menu == 31 {
        let has = conn.session.homdo.iter().any(|i| i.id == 65517);
        if has {
            conn.session
                .add_homdo_item(crate::server::inventory::from_template(
                    data,
                    item_a as u16,
                    1,
                ));
            crate::server::inventory::remove_item(&mut conn.session.homdo, 65517, 20);
            out.send(conn.session.dump_homdo());
            end_talk(conn, out);
        }
    }
    if idtalking == 11 {
        // Map 12711 idtalking 11: level/skillpoint/point boosts (the other
        // daily map's case 12003 keeps both maps on 21 draws).
        match select_menu {
            30 => {
                conn.session.level = conn.session.level.saturating_add(1).min(200);
                conn.session.skill_point = conn.session.skill_point.saturating_add(1);
                conn.session.point = conn.session.point.saturating_add(2);
            }
            31 => {
                conn.session.level = conn.session.level.saturating_add(5).min(200);
                conn.session.skill_point = conn.session.skill_point.saturating_add(5);
                conn.session.point = conn.session.point.saturating_add(10);
            }
            _ => {}
        }
    }
}

/// Pet-reborn NPC exceptions — keyed `(map_id, map_object_id)` per the ticket
/// 19 review: `55002/59102/59011` are **map ids**, not template ids; the right
/// keys are `(55002,3)`, `(59102,1)`, `(59011,1)` (map 12711's pet shop rows).
pub fn handle_pet_reborn_npc(
    conn: &mut Conn,
    map_id: i64,
    map_object_id: i64,
    out: &mut HandleOutcome,
) -> bool {
    if !is_pet_reborn_key(map_id, map_object_id) {
        return false;
    }

    let select_menu = conn.session.select_menu;
    match select_menu {
        30 => {
            // Show pet reborn dialog/menu
            out.send("F44411001401000000010603010000000000000100");
        }
        31 => {
            // Execute pet reborn (handled by pet_actions/skills handler)
            end_talk(conn, out);
        }
        40 => end_talk(conn, out),
        _ => end_talk(conn, out),
    }
    true
}

/// Runtime key-signature check for pet-reborn map rows (data-driven guard).
pub fn is_pet_reborn_key(map_id: i64, map_object_id: i64) -> bool {
    (map_id == 55002 && map_object_id == 3)
        || (map_id == 59102 && map_object_id == 1)
        || (map_id == 59011 && map_object_id == 1)
}

/// Quest requirement failure packets (§2.6.3).
///
/// When `idtalking > 0`: `F44411001401000000020103` + id:X2 + `00000000000000BB`.
/// When `idtalking <= 0`: `F4441100140100000001010700000000000000493C`.
pub fn send_requirement_fail(conn: &mut Conn, id: i32, out: &mut HandleOutcome) {
    let packet = if id > 0 {
        format!("F44411001401000000020103{:02X}00000000000000BB", id)
    } else {
        "F4441100140100000001010700000000000000493C".to_string()
    };
    out.send(packet);
    end_talk(conn, out);
    conn.session.select_menu = 40;
}

/// A battle trigger from quest/TEAMDEF.
#[derive(Debug, Clone)]
pub struct BattleTrigger {
    /// TeamDef: [diahinh, npc1..npc10].
    pub teamdef: Vec<i64>,
    /// Terrain ID.
    pub diahinh: i32,
}

/// Handle warp-talk (H8) completion: confirm warp into 0x0C flow.
pub fn handle_warp_confirm(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) {
    let map_id = conn.session.map_id;
    let idtalking = conn.session.idtalking;

    // Check the WARP-type talk data first. When a per-step entry exists we
    // drive its dialogs / TEAMDEF / OnWin directly and do NOT fall through to
    // the simple warp.
    let talk_key = format!("{}:WARP:{}:0", map_id, idtalking);
    if let Some(quest) = data.talks.get(&talk_key) {
        if !quest.dialogs.is_empty() {
            // _RequireSelectMenu mismatch → LoseDialogs[0]/EndTalk.
            let is_menu = quest.dialogs.contains("F444110014010000000106");
            if is_menu
                && quest.require_select_menu > 0
                && conn.session.select_menu != quest.require_select_menu as i32
            {
                let lose = &quest.on_lose.dialogs;
                if !lose.is_empty() {
                    talk_messages(conn, lose, out);
                    conn.session.select_menu = 40;
                } else {
                    end_talk(conn, out);
                }
                return;
            }
            talk_messages(conn, &quest.dialogs, out);
            return;
        }

        // No dialogs: requirement gate, then a WARP TEAMDEF battle or OnWin.
        if quest.require_select_menu > 0
            && conn.session.select_menu != quest.require_select_menu as i32
        {
            let lose = &quest.on_lose.dialogs;
            if !lose.is_empty() {
                talk_messages(conn, lose, out);
                conn.session.select_menu = 40;
            } else {
                send_requirement_fail(conn, idtalking, out);
            }
            return;
        }

        let sum: i64 = quest.teamdef.iter().sum();
        if !quest.teamdef.is_empty() && sum > 0 {
            conn.session.talking_battle = idtalking;
            out.battle_trigger = Some(BattleTrigger {
                teamdef: quest.teamdef.clone(),
                diahinh: quest.teamdef.first().copied().unwrap_or(112) as i32,
            });
            return;
        }

        // OnWin with no TEAMDEF → apply rewards + end.
        if !quest.on_win.dialogs.is_empty() {
            talk_messages(conn, &quest.on_win.dialogs, out);
        } else {
            let mut frames = Vec::new();
            battle_quest_win(&mut conn.session, data, &mut frames, &mut |_| None);
            for f in frames {
                out.send(f);
            }
        }
        end_talk(conn, out);
        return;
    }

    // Lookup warp data
    let warp_key = (map_id as i64, idtalking as i64);
    if let Some(warp) = data.warps.get(&warp_key) {
        // Check if this warp has a battle gate
        if let Some(gate) = data.battle_gates.get(&warp_key) {
            if gate.diahinh > 0 {
                // Trigger battle gate battle
                let mut teamdef = vec![gate.diahinh];
                teamdef.extend_from_slice(&gate.defenders);
                conn.session.talking_battle = idtalking;
                out.battle_trigger = Some(BattleTrigger {
                    teamdef,
                    diahinh: gate.diahinh as i32,
                });
                return;
            }
        }

        // Normal warp
        conn.session.map_id = warp.map2 as u16;
        conn.session.map_x = warp.x as u16;
        conn.session.map_y = warp.y as u16;
        out.send("F44402000504");
        end_talk(conn, out);
    } else {
        end_talk(conn, out);
    }
}
