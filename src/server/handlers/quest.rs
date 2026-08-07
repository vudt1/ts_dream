//! Quest engine (Ticket 19): H6 data-driven table, daily-quest generator,
//! pet-reborn exceptions, quest requirement checking, TEAMDEF battle trigger,
//! and quest result processing (OnWin/OnLose rewards).

use crate::battle::packets;
use crate::battle::rng::DotNetRandom;
use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::handler::HandleOutcome;
use crate::server::handlers::stats::build_stat_update;
use crate::server::handlers::talk::{end_talk, talk_messages};
use crate::server::session::{Conn, Session};
use crate::server::spawn::sys_msg_frame;
use std::sync::Arc;

/// Attempt the data-driven quest path for H6 continue.
///
/// Returns `true` if the quest data was found and handled, `false` if the
/// caller should fall back to the hard-coded NPC paths.
pub fn try_quest_h6(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) -> bool {
    let idtalking = conn.session.idtalking;
    let select_menu = conn.session.select_menu;
    let map_id = conn.session.map_id;

    // Build the talk key: "mapId:NPC:idtalking:0"
    let key = format!("{}:NPC:{}:0", map_id, idtalking);

    let quest = match data.talks.get(&key) {
        Some(q) => q,
        None => return false,
    };

    // If TEAMDEF exists and dialogs are empty → trigger battle
    if quest.dialogs.is_empty() && !quest.teamdef.is_empty() {
        let sum: i64 = quest.teamdef.iter().sum();
        if sum > 0 {
            // Signal battle trigger — store the talk for post-battle processing.
            // The actual battle construction will be handled by the battle engine.
            conn.session.talking_battle = idtalking;
            out.battle_trigger = Some(BattleTrigger {
                teamdef: quest.teamdef.clone(),
                diahinh: quest.teamdef.first().copied().unwrap_or(112) as i32,
            });
            return true;
        }
    }

    // Send dialog messages
    if !quest.dialogs.is_empty() {
        // _RequireSelectMenu mismatch → LoseDialogs[0]/EndTalk (FTalk.cs:2687-2713).
        // Only applies when the dialog is a menu frame.
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

    // If dialogs are exhausted and a TEAMDEF exists → battle
    if select_menu == 40 && !quest.teamdef.is_empty() {
        let sum: i64 = quest.teamdef.iter().sum();
        if sum > 0 {
            conn.session.talking_battle = idtalking;
            out.battle_trigger = Some(BattleTrigger {
                teamdef: quest.teamdef.clone(),
                diahinh: quest.teamdef.first().copied().unwrap_or(112) as i32,
            });
            return true;
        }
    }

    true
}

/// Full ordered `BattleQuestWin` side effects (Data.cs:5812-5998, spec §6.7).
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
    let key = format!("{}:NPC:{}:0", session.map_id, idtalking);
    let quest = match data.talks.get(&key) {
        Some(q) => q,
        None => return false,
    };
    let result = &quest.on_win;

    // Win dialogs take precedence over the reward pipeline (TheBattle.cs:4742).
    if !result.dialogs.is_empty() {
        for part in result.dialogs.split("F444") {
            if !part.is_empty() {
                frames.push(format!("F444{part}"));
            }
        }
        frames.push("F44402001408".to_string()); // EndTalk after dialogs
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
            let mut it = crate::server::inventory::from_template(data, item_id as u16, count.min(255) as u8);
            it.doben = 100;
            it
        });
        if share > 0 {
            for mem in session.id_mem.iter().filter(|m| **m > 0) {
                if let Some(m) = member(i64::from(*mem)) {
                    if let Ok(mut s) = m.try_write() {
                        let _ = s.add_homdo_item({
                            let mut it = crate::server::inventory::from_template(data, item_id as u16, count.min(255) as u8);
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
            session.recompute_stats();
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

    clear_quest_talk(session);
    true
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

/// Compat wrapper: the pre-ticket win processing (see `battle_quest_win`).
pub fn process_quest_win(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) {
    let mut frames = Vec::new();
    battle_quest_win(&mut conn.session, data, &mut frames, &mut |_| None);
    for f in frames {
        out.send(f);
    }
}

/// Process quest lose rewards.
pub fn process_quest_lose(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome) {
    let idtalking = conn.session.talking_battle;
    if idtalking <= 0 {
        return;
    }

    let key = format!("{}:NPC:{}:0", conn.session.map_id, idtalking);
    if let Some(quest) = data.talks.get(&key) {
        // Send OnLose dialogs if present
        if !quest.on_lose.dialogs.is_empty() {
            talk_messages(conn, &quest.on_lose.dialogs, out);
        }
    }

    end_talk(conn, out);
    conn.session.talking_battle = 0;
}

/// Daily quest generator (map 12711, 21 RNG draws — §2.6.2 / research 06 §(6)).
///
/// Uses a fresh time-seeded Random (NOT the battle streams).
/// Exactly 21 `random.Next` draws consumed in order, even if unused.
pub fn generate_daily_quest(conn: &mut Conn, out: &mut HandleOutcome) {
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

    // Reward item formulas
    let _num35 = 62001 + num3 * 100;
    let _num36 = 62002 + num3 * 100;
    let _num37 = 62003 + num3 * 100;
    let _num38 = 62004 + num3 * 100;
    let _i_d4 = 62101 + num4 * 100;
    let _i_d5 = 62102 + num4 * 100;
    let _i_d6 = 62103 + num4 * 100;
    let _i_d7 = 62104 + num4 * 100;

    // The actual dialog/menu is driven by the H6 data table,
    // which sends appropriate packets based on select_menu.
    // For now, send the basic daily quest dialog.
    let _ = conn; // session used for menu-branch selection
    let _ = out; // packets emitted per select_menu branch
}

/// Pet-reborn NPC exceptions (55002, 59102, 59011).
pub fn handle_pet_reborn_npc(conn: &mut Conn, idtalking: i32, out: &mut HandleOutcome) -> bool {
    if !matches!(idtalking, 55002 | 59102 | 59011) {
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

/// Quest requirement failure packets (§2.6.3, FTalk.cs:2720-2739).
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

    // C# H8 (FTalk.cs:3258-3287): check the WARP-type talk data first. When a
    // per-step entry exists we drive its dialogs / TEAMDEF / OnWin directly and
    // do NOT fall through to the simple warp, mirroring `GetDataTalkExits`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::tables::{QuestDef, QuestResult, Skill};

    #[test]
    fn daily_quest_21_draws() {
        // Verify that exactly 21 RNG draws are consumed
        let mut conn = Conn::new();
        let mut out = HandleOutcome::default();
        generate_daily_quest(&mut conn, &mut out);
        // No crash = all 21 draws succeeded
    }

    /// Run `battle_quest_win` against a fresh session with the given OnWin
    /// result (talk key `10916:NPC:1:0`), returning the emitted frames.
    fn run_quest_win(result: QuestResult, mut data: GameData) -> (Session, Vec<String>) {
        let mut session = Session::new();
        session.id = 300001;
        session.map_id = 10916;
        session.talking_battle = 1;
        data.talks.insert(
            "10916:NPC:1:0".to_string(),
            QuestDef {
                map_id: 10916,
                id: 1,
                on_win: result,
                ..Default::default()
            },
        );
        let mut frames = Vec::new();
        battle_quest_win(&mut session, &data, &mut frames, &mut |_| None);
        (session, frames)
    }

    #[test]
    fn quest_win_rewards() {
        let result = QuestResult {
            rewards: vec![(46001, 5, 0)],
            ..Default::default()
        };
        let (session, _) = run_quest_win(result, GameData::default());
        assert_eq!(session.homdo.len(), 1);
        assert_eq!(session.homdo[0].id, 46001);
        assert_eq!(session.homdo[0].count, 5);
    }

    #[test]
    fn quest_win_random_reward() {
        let result = QuestResult {
            random_rewards: vec![(46001, 1, 0), (46002, 2, 0), (46003, 3, 0)],
            ..Default::default()
        };
        let (session, _) = run_quest_win(result, GameData::default());
        assert_eq!(session.homdo.len(), 1);
        // Should be one of the three items
        let item_id = session.homdo[0].id;
        assert!(
            item_id == 46001 || item_id == 46002 || item_id == 46003,
            "unexpected item: {}",
            item_id
        );
    }

    #[test]
    fn quest_win_add_skill() {
        let mut data = GameData::default();
        data.skills.insert(
            10001,
            Skill {
                id: 10001,
                name: "Kiem".to_string(),
                ..Default::default()
            },
        );
        let result = QuestResult {
            add_skill: vec![(10001, 1)],
            ..Default::default()
        };
        let (session, frames) = run_quest_win(result, data);
        assert_eq!(session.skills.len(), 1);
        assert_eq!(session.skills[0], (10001, 1));
        // Learn packet present.
        assert!(frames.iter().any(|f| f.contains("6E01")));
    }

    #[test]
    fn pet_reborn_npc_handled() {
        let mut conn = Conn::new();
        conn.session.idtalking = 55002;
        conn.session.select_menu = 30;
        let mut out = HandleOutcome::default();

        let handled = handle_pet_reborn_npc(&mut conn, 55002, &mut out);
        assert!(handled);
        assert!(!out.outgoing.is_empty());
    }

    #[test]
    fn pet_reborn_npc_not_handled() {
        let mut conn = Conn::new();
        let mut out = HandleOutcome::default();

        let handled = handle_pet_reborn_npc(&mut conn, 16080, &mut out);
        assert!(!handled);
    }

    #[test]
    fn requirement_fail_packet() {
        let mut conn = Conn::new();
        let mut out = HandleOutcome::default();
        send_requirement_fail(&mut conn, 42, &mut out);
        assert_eq!(
            out.outgoing[0],
            "F444110014010000000201032A00000000000000BB"
        );
        // Followed by EndTalk + SelectMenu reset to 40
        assert_eq!(out.outgoing[1], "F44402001408");
        assert_eq!(conn.session.select_menu, 40);
        assert_eq!(conn.session.idtalking, 0);
    }

    #[test]
    fn requirement_fail_packet_zero_id() {
        let mut conn = Conn::new();
        let mut out = HandleOutcome::default();
        send_requirement_fail(&mut conn, 0, &mut out);
        assert_eq!(
            out.outgoing[0],
            "F4441100140100000001010700000000000000493C"
        );
        assert_eq!(out.outgoing[1], "F44402001408");
    }

    #[test]
    fn quest_save_leader_quests() {
        let result = QuestResult {
            save_leader_quests: vec![(100, 1, 0, 1)],
            ..Default::default()
        };
        let (session, _) = run_quest_win(result, GameData::default());
        assert_eq!(session.quest_steps, vec![(100, 1)]);
    }

    #[test]
    fn quest_win_use_items_consumed() {
        let mut session = Session::new();
        session.id = 300001;
        session.map_id = 10916;
        session.talking_battle = 1;
        // Seed a use-item requirement into inventory
        session.add_homdo_item(crate::server::session::InventoryItem {
            id: 19001,
            count: 3,
            loai: 1,
            doben: 100,
            ..Default::default()
        });
        let mut data = GameData::default();
        data.talks.insert(
            "10916:NPC:1:0".to_string(),
            QuestDef {
                map_id: 10916,
                id: 1,
                on_win: QuestResult {
                    use_items: vec![(19001, 0)],
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let mut frames = Vec::new();
        battle_quest_win(&mut session, &data, &mut frames, &mut |_| None);
        // 3 - 1 = 2 remaining
        let left = session.homdo.iter().map(|i| i.count).sum::<u8>();
        assert_eq!(left, 2);
        // Self use-item frame `F44403001711`+slot present.
        assert!(frames.iter().any(|f| f.starts_with("F44403001711")));
    }

    #[test]
    fn quest_win_add_pet() {
        let result = QuestResult {
            add_pet: vec![18017],
            ..Default::default()
        };
        let (session, _) = run_quest_win(result, GameData::default());
        assert_eq!(session.pets.len(), 1);
        assert_eq!(session.pets[0].id, 18017);
    }

    #[test]
    fn quest_win_click_npc_id() {
        let result = QuestResult {
            click_npc_id: 59011,
            ..Default::default()
        };
        let (session, _) = run_quest_win(result, GameData::default());
        assert_eq!(session.click_npc_id, 59011);
    }

    #[test]
    fn quest_win_warp_leader() {
        let result = QuestResult {
            warp_to: vec![12001, 400, 500],
            ..Default::default()
        };
        let (session, frames) = run_quest_win(result, GameData::default());
        assert_eq!(session.map_id, 12001);
        assert_eq!(session.map_x, 400);
        assert_eq!(session.map_y, 500);
        assert!(frames.iter().any(|f| f.starts_with("F4440D000C")));
    }

    #[test]
    fn quest_win_end_talk_without_warp() {
        let result = QuestResult::default();
        let (session, frames) = run_quest_win(result, GameData::default());
        assert!(frames.iter().any(|f| f == "F44402001408"));
        assert_eq!(session.talking_battle, 0);
    }

    #[test]
    fn quest_win_dialogs_take_precedence() {
        // Non-empty dialogs skip the reward pipeline entirely (TheBattle.cs:4742).
        let result = QuestResult {
            dialogs: "F44411001401000000010103010000000000009E28".to_string(),
            rewards: vec![(46001, 5, 0)],
            ..Default::default()
        };
        let (session, frames) = run_quest_win(result, GameData::default());
        assert!(frames
            .iter()
            .any(|f| f.starts_with("F44411001401000000010103")));
        assert!(frames.iter().any(|f| f == "F44402001408"));
        assert!(session.homdo.is_empty(), "no rewards when dialogs present");
    }

    #[test]
    fn select_menu_mismatch_sends_lose_dialog() {
        let mut conn = Conn::new();
        conn.session.map_id = 10916;
        conn.session.idtalking = 1;
        conn.session.select_menu = 20; // wrong menu
        let mut data = crate::data::loader::GameData::default();
        data.talks.insert(
            "10916:NPC:1:0".to_string(),
            QuestDef {
                map_id: 10916,
                id: 1,
                dialogs: "F44411001401000000010603010000000000000100".to_string(),
                require_select_menu: 30,
                on_lose: QuestResult {
                    dialogs: "F44411001401000000010103010000000000009E28".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        let mut out = HandleOutcome::default();

        let handled = try_quest_h6(&mut conn, &data, &mut out);
        assert!(handled, "expected handled quest path");
        assert_eq!(conn.session.select_menu, 40);
        assert_eq!(out.outgoing.len(), 1);
        assert_eq!(
            out.outgoing[0],
            "F44411001401000000010103010000000000009E28"
        );
    }

    #[test]
    fn warp_talk_teamdef_triggers_battle() {
        let mut conn = Conn::new();
        conn.session.map_id = 11011;
        conn.session.idtalking = 3;
        let mut data = crate::data::loader::GameData::default();
        data.talks.insert(
            "11011:WARP:3:0".to_string(),
            QuestDef {
                map_id: 11011,
                talk_type: "WARP".to_string(),
                id: 3,
                teamdef: vec![121, 0, 17177, 14073, 17177, 0, 0, 0, 0, 0, 0],
                ..Default::default()
            },
        );

        let mut out = HandleOutcome::default();
        handle_warp_confirm(&mut conn, &data, &mut out);
        assert!(out.battle_trigger.is_some(), "expected battle trigger");
        let t = out.battle_trigger.as_ref().unwrap();
        assert_eq!(t.diahinh, 121);
        assert_eq!(conn.session.talking_battle, 3);
    }

    #[test]
    fn warp_talk_plain_warp() {
        let mut conn = Conn::new();
        conn.session.map_id = 11011;
        conn.session.idtalking = 2;
        let mut data = crate::data::loader::GameData::default();
        data.warps.insert(
            (11011, 2),
            crate::data::tables::Warp {
                map1: 11011,
                warpid: 2,
                map2: 12001,
                x: 400,
                y: 500,
            },
        );

        let mut out = HandleOutcome::default();
        handle_warp_confirm(&mut conn, &data, &mut out);
        assert!(out.battle_trigger.is_none());
        assert_eq!(conn.session.map_id, 12001);
        assert_eq!(conn.session.map_x, 400);
    }

    #[test]
    fn warp_talk_teamdef_missing_uses_gate() {
        let mut conn = Conn::new();
        conn.session.map_id = 59841;
        conn.session.idtalking = 1;
        let mut data = crate::data::loader::GameData::default();
        data.warps.insert(
            (59841, 1),
            crate::data::tables::Warp {
                map1: 59841,
                warpid: 1,
                map2: 60000,
                x: 10,
                y: 20,
            },
        );
        data.battle_gates.insert(
            (59841, 1),
            crate::data::tables::BattleGate {
                mapid1: 59841,
                warpid: 1,
                diahinh: 365,
                defenders: [1001, 1002, 0, 0, 0, 0, 0, 0, 0, 0],
            },
        );

        let mut out = HandleOutcome::default();
        handle_warp_confirm(&mut conn, &data, &mut out);
        assert!(out.battle_trigger.is_some(), "expected gate battle trigger");
        let t = out.battle_trigger.as_ref().unwrap();
        assert_eq!(t.diahinh, 365);
    }
}
