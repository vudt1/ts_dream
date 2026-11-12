//! Special Event NPC Handlers (Tickets 01, 04, 05, 09).
//!
//! Handles interactions with event NPCs extracted from `talk.rs`:
//! - 1752: Exorcist / Bà Gọi Hồn (Ghost state removal)
//! - 15162: 40-NPC Arena Registrar (Challenge registration)
//! - 15312: 2K Rescue Registrar (Tower entry)
//! - 61209: Nine Heavens / Cửu Thiên Registrar (Entry gate)
//! - 20001: Floor Reward Chest (Continuous dungeons)
//! - 14628: 40-NPC Wave NPC
//! - 15313: 2K Rescue Floor NPC
//! - 61210: Nine Heavens Floor NPC

use crate::data::loader::GameData;
use crate::db::pool::DbPool;
use crate::server::dispatcher::HandleOutcome;
use crate::server::handlers::shops::gold_frame;
use crate::server::handlers::talk::end_talk;
use crate::server::session::{Conn, InventoryItem};

/// Check whether the template ID is an event NPC handled by this module.
pub fn is_event_npc(template_id: i32) -> bool {
    matches!(
        template_id,
        1752 | 15162 | 15312 | 61209 | 20001 | 14628 | 15313 | 61210
    )
}

/// Try handling an event NPC interaction. Returns `true` if handled, `false` otherwise.
pub async fn try_handle_event_npc(
    template_id: i32,
    in_range: bool,
    conn: &mut Conn,
    data: &GameData,
    service: Option<&crate::battle::service::BattleService>,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) -> bool {
    if !is_event_npc(template_id) {
        return false;
    }

    if template_id == 1752 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        if conn.session.ghost == 1 {
            if conn.session.gold >= 500 {
                conn.session.gold -= 500;
                conn.session.ghost = 0;
                out.send(gold_frame(conn.session.gold));
                let sync = crate::server::spawn::state_sync_frame(conn.session.id, 1, 0);
                out.send(sync.clone());
                out.broadcast(conn.session.id, &sync);
                let _ = crate::db::persist::persist_sessions_transaction(
                    pool,
                    &[&conn.session],
                    &["stats"],
                )
                .await;
                crate::server::session::registry_update_if_online(&conn.session);
                out.send(crate::server::spawn::sys_msg_frame("Da hoa giai ma theo!"));
            } else {
                out.send(crate::server::spawn::sys_msg_frame("Khong du tien"));
            }
        } else {
            out.send(crate::server::spawn::sys_msg_frame("Ban khong bi ma theo"));
        }
        end_talk(conn, out);
        return true;
    }

    if template_id == 15162 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        if conn.session.id_leader > 0 && conn.session.id_leader != conn.session.id {
            out.send(crate::server::spawn::sys_msg_frame("Chi doi truong moi co the dang ky"));
            end_talk(conn, out);
            return true;
        }
        let ticket_count = conn.session.remove_homdo_item(46072, 1);
        let allowed = if ticket_count > 0 {
            out.send(conn.session.dump_homdo());
            true
        } else if conn.session.gold >= 1000 {
            conn.session.gold -= 1000;
            out.send(gold_frame(conn.session.gold));
            true
        } else {
            false
        };

        if allowed {
            crate::server::handlers::quest_sync::set_quest_task(&mut conn.session, out, 218, 1);
            let old_map = conn.session.map_id;
            conn.session.map_id = 10851;
            conn.session.map_x = 400;
            conn.session.map_y = 400;
            crate::server::session::registry_update_if_online(&conn.session);
            out.send("F44402001407");
            out.send(crate::server::spawn::build_relocate_packet(conn.session.id, 10851, 400, 400, 0));
            out.broadcast_to_map(conn.session.id, old_map, crate::battle::packets::hide_from_map(conn.session.id));
            let _ = crate::db::persist::persist_sessions_transaction(
                pool,
                &[&conn.session],
                &["stats"],
            )
            .await;
        } else {
            out.send(crate::server::spawn::sys_msg_frame("Khong du tien hoac ve loi dai"));
        }
        end_talk(conn, out);
        return true;
    }

    if template_id == 15312 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        if conn.session.id_leader > 0 && conn.session.id_leader != conn.session.id {
            out.send(crate::server::spawn::sys_msg_frame("Chi doi truong moi co the dang ky"));
            end_talk(conn, out);
            return true;
        }
        let party_count = 1 + conn.session.id_mem.iter().filter(|&&m| m > 0 && m != conn.session.id).count();
        if party_count != 1 && party_count != 5 {
            out.send(crate::server::spawn::sys_msg_frame("Chi cho phep to doi 1 nguoi hoac 5 nguoi"));
            end_talk(conn, out);
            return true;
        }
        crate::server::handlers::quest_sync::set_quest_task(&mut conn.session, out, 212, 1);
        let alert_frame = crate::protocol::codecs::NpcTalkCodec::build_event_alert_hex(2);
        out.send(alert_frame.clone());
        out.broadcast(conn.session.id, alert_frame);

        let old_map = conn.session.map_id;
        conn.session.map_id = 12911;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        crate::server::session::registry_update_if_online(&conn.session);
        out.send("F44402001407");
        out.send(crate::server::spawn::build_relocate_packet(conn.session.id, 12911, 400, 400, 0));
        out.broadcast_to_map(conn.session.id, old_map, crate::battle::packets::hide_from_map(conn.session.id));

        if conn.session.id_leader == conn.session.id && conn.session.id > 0 {
            for member in conn.session.id_mem {
                if member > 0 && member != conn.session.id {
                    let member_old_map = {
                        let mut online = crate::server::session::lock_online_sessions();
                        if let Some(mem) = online.get_mut(&member) {
                            let m_old = mem.map_id;
                            mem.map_id = 12911;
                            mem.map_x = 400;
                            mem.map_y = 400;
                            m_old
                        } else {
                            old_map
                        }
                    };
                    out.send_to(member, "F44402001407");
                    out.send_to(member, crate::server::spawn::build_relocate_packet(member, 12911, 400, 400, 0));
                    out.broadcast_to_map(member, member_old_map, crate::battle::packets::hide_from_map(member));
                }
            }
        }
        let _ = crate::db::persist::persist_sessions_transaction(
            pool,
            &[&conn.session],
            &["stats"],
        )
        .await;
        end_talk(conn, out);
        return true;
    }

    if template_id == 61209 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        if conn.session.id_leader > 0 && conn.session.id_leader != conn.session.id {
            out.send(crate::server::spawn::sys_msg_frame("Chi doi truong moi co the dang ky"));
            end_talk(conn, out);
            return true;
        }
        let has_companion = conn.session.active_pet_stt > 0
            && conn.session.pets.iter().any(|p| p.stt == conn.session.active_pet_stt && p.id == 14735);
        if !has_companion {
            out.send(crate::server::spawn::sys_msg_frame("Can phai co Hau Ton di theo"));
            end_talk(conn, out);
            return true;
        }
        let quota_step = conn.session.quest_tasks.get(&12310).map(|&(_, s)| s).unwrap_or(0);
        if quota_step != 0 {
            out.send(crate::server::spawn::sys_msg_frame("Hien tai van chua den luc..."));
            end_talk(conn, out);
            return true;
        }
        crate::server::handlers::quest_sync::set_quest_task(&mut conn.session, out, 12309, 1);
        let old_map = conn.session.map_id;
        conn.session.map_id = 49960;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        crate::server::session::registry_update_if_online(&conn.session);
        out.send("F44402001407");
        out.send(crate::server::spawn::build_relocate_packet(conn.session.id, 49960, 400, 400, 0));
        out.broadcast_to_map(conn.session.id, old_map, crate::battle::packets::hide_from_map(conn.session.id));
        let _ = crate::db::persist::persist_sessions_transaction(
            pool,
            &[&conn.session],
            &["stats"],
        )
        .await;
        end_talk(conn, out);
        return true;
    }

    if template_id == 20001 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }

        // Floor Reward Chest (20001) Gating:
        // 12927 requires QuestStep(212) >= 17, 12928 requires QuestStep(212) >= 18
        // 49960..=49969: floor M requires QuestStep(12309) >= (M - 49960) + 1
        if conn.session.map_id == 49969 {
            let step = conn.session.quest_tasks.get(&12309).map(|&(_, s)| s).unwrap_or(0);
            if step < 10 {
                out.send(crate::server::spawn::sys_msg_frame("Chua du dieu kien mo ruong tang nay!"));
                end_talk(conn, out);
                return true;
            }
            crate::server::handlers::quest_sync::set_quest_task(&mut conn.session, out, 12310, 1);
            if let Some(item_def) = data.items.get(&26051) {
                let item = InventoryItem::from_template(item_def, 1);
                conn.session.add_homdo_item(item);
            }
            if let Some(item_def) = data.items.get(&23086) {
                let item = InventoryItem::from_template(item_def, 1);
                conn.session.add_homdo_item(item);
            }
            out.send(conn.session.dump_homdo());
            out.send(crate::server::spawn::sys_msg_frame("Chuc mung da vuot qua Cuu Thien!"));
        } else if (49960..=49968).contains(&conn.session.map_id) {
            let required_step = ((conn.session.map_id - 49960) + 1) as u8;
            let step = conn.session.quest_tasks.get(&12309).map(|&(_, s)| s).unwrap_or(0);
            if step < required_step {
                out.send(crate::server::spawn::sys_msg_frame("Chua du dieu kien mo ruong tang nay!"));
                end_talk(conn, out);
                return true;
            }
            if let Some(item_def) = data.items.get(&26051) {
                let item = InventoryItem::from_template(item_def, 1);
                conn.session.add_homdo_item(item);
                out.send(conn.session.dump_homdo());
            }
            out.send(crate::server::spawn::sys_msg_frame("Nhan duoc phan thuong tu Thung Bau Vat!"));
        } else if matches!(conn.session.map_id, 12927 | 12928) {
            let required_step = if conn.session.map_id == 12927 { 17 } else { 18 };
            let step = conn.session.quest_tasks.get(&212).map(|&(_, s)| s).unwrap_or(0);
            if step < required_step {
                out.send(crate::server::spawn::sys_msg_frame("Chua du dieu kien mo ruong tang nay!"));
                end_talk(conn, out);
                return true;
            }
            if let Some(item_def) = data.items.get(&23086) {
                let item = InventoryItem::from_template(item_def, 1);
                conn.session.add_homdo_item(item);
            }
            if let Some(item_def) = data.items.get(&26051) {
                let item = InventoryItem::from_template(item_def, 1);
                conn.session.add_homdo_item(item);
            }
            let cred_id = if conn.session.map_id == 12927 { 46049 } else { 46050 };
            if let Some(item_def) = data.items.get(&cred_id) {
                let item = InventoryItem::from_template(item_def, 1);
                conn.session.add_homdo_item(item);
            }
            out.send(conn.session.dump_homdo());
            out.send(crate::server::spawn::sys_msg_frame("Giai cuu thanh cong!"));
        }
        crate::server::session::registry_update_if_online(&conn.session);
        let _ = crate::db::persist::persist_sessions_transaction(
            pool,
            &[&conn.session],
            &["homdo", "quest"],
        )
        .await;
        end_talk(conn, out);
        return true;
    }

    // 14628: 40-NPC Arena Wave NPC (Ticket 12)
    if template_id == 14628 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        let quest_step = conn.session.quest_tasks.get(&218).map(|&(_, s)| s).unwrap_or(0);
        if quest_step == 0 {
            out.send(crate::server::spawn::sys_msg_frame("Chua dang ky tham gia loi dai 40 NPC!"));
            end_talk(conn, out);
            return true;
        }

        let waves = build_40_npc_waves(data);
        if let Some(service) = service {
            let (_battle_id, start_frames) = service.start_40_npc_battle(&mut conn.session, waves);
            for f in start_frames {
                out.send(f);
            }
        }
        crate::server::session::registry_update_if_online(&conn.session);
        end_talk(conn, out);
        return true;
    }

    // 15313: 2K Floor Transition NPC (Ticket 13)
    if template_id == 15313 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        let map_id = conn.session.map_id;
        if !(12911..=12927).contains(&map_id) {
            end_talk(conn, out);
            return true;
        }

        // 1. Production Eve engine priority
        if data.scene_eve_data.contains_key(&(map_id as u32)) {
            return false;
        }

        // 2. Deterministic Test Fallback
        let current_stage = ((map_id - 12911) + 1) as u8; // 1..=17
        let quest_step = conn.session.quest_tasks.get(&212).map(|&(_, s)| s).unwrap_or(0);
        if quest_step != current_stage {
            out.send(crate::server::spawn::sys_msg_frame("Chua hoan thanh thu thach tang nay!"));
            end_talk(conn, out);
            return true;
        }

        let next_step = current_stage + 1;
        crate::server::handlers::quest_sync::set_quest_task(&mut conn.session, out, 212, next_step);

        let next_map = map_id + 1;
        let old_map = conn.session.map_id;
        conn.session.map_id = next_map;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        crate::server::session::registry_update_if_online(&conn.session);

        out.send("F44402001407");
        out.send(crate::server::spawn::build_relocate_packet(conn.session.id, next_map, 400, 400, 0));
        out.broadcast_to_map(conn.session.id, old_map, crate::battle::packets::hide_from_map(conn.session.id));

        if conn.session.id_leader == conn.session.id && conn.session.id > 0 {
            for member in conn.session.id_mem {
                if member > 0 && member != conn.session.id {
                    let member_old_map = {
                        let mut online = crate::server::session::lock_online_sessions();
                        if let Some(mem) = online.get_mut(&member) {
                            let m_old = mem.map_id;
                            mem.map_id = next_map;
                            mem.map_x = 400;
                            mem.map_y = 400;
                            mem.quest_tasks.insert(212, (0, next_step));
                            m_old
                        } else {
                            old_map
                        }
                    };
                    out.send_to(member, "F44402001407");
                    out.send_to(member, crate::server::spawn::build_relocate_packet(member, next_map, 400, 400, 0));
                    out.broadcast_to_map(member, member_old_map, crate::battle::packets::hide_from_map(member));
                }
            }
        }

        let _ = crate::db::persist::persist_sessions_transaction(
            pool,
            &[&conn.session],
            &["stats", "quest"],
        )
        .await;

        end_talk(conn, out);
        return true;
    }

    // 61210: Nine Heavens Floor Transition NPC (Ticket 13)
    if template_id == 61210 {
        if !in_range {
            end_talk(conn, out);
            return true;
        }
        let map_id = conn.session.map_id;
        if !(49960..=49968).contains(&map_id) {
            end_talk(conn, out);
            return true;
        }

        // 1. Production Eve engine priority
        if data.scene_eve_data.contains_key(&(map_id as u32)) {
            return false;
        }

        // 2. Deterministic Test Fallback
        let current_stage = ((map_id - 49960) + 1) as u8; // 1..=9
        let quest_step = conn.session.quest_tasks.get(&12309).map(|&(_, s)| s).unwrap_or(0);
        if quest_step != current_stage {
            out.send(crate::server::spawn::sys_msg_frame("Chua hoan thanh thu thach tang nay!"));
            end_talk(conn, out);
            return true;
        }

        let next_step = current_stage + 1;
        crate::server::handlers::quest_sync::set_quest_task(&mut conn.session, out, 12309, next_step);

        let next_map = map_id + 1;
        let old_map = conn.session.map_id;
        conn.session.map_id = next_map;
        conn.session.map_x = 400;
        conn.session.map_y = 400;
        crate::server::session::registry_update_if_online(&conn.session);

        out.send("F44402001407");
        out.send(crate::server::spawn::build_relocate_packet(conn.session.id, next_map, 400, 400, 0));
        out.broadcast_to_map(conn.session.id, old_map, crate::battle::packets::hide_from_map(conn.session.id));

        if conn.session.id_leader == conn.session.id && conn.session.id > 0 {
            for member in conn.session.id_mem {
                if member > 0 && member != conn.session.id {
                    let member_old_map = {
                        let mut online = crate::server::session::lock_online_sessions();
                        if let Some(mem) = online.get_mut(&member) {
                            let m_old = mem.map_id;
                            mem.map_id = next_map;
                            mem.map_x = 400;
                            mem.map_y = 400;
                            mem.quest_tasks.insert(12309, (0, next_step));
                            m_old
                        } else {
                            old_map
                        }
                    };
                    out.send_to(member, "F44402001407");
                    out.send_to(member, crate::server::spawn::build_relocate_packet(member, next_map, 400, 400, 0));
                    out.broadcast_to_map(member, member_old_map, crate::battle::packets::hide_from_map(member));
                }
            }
        }

        let _ = crate::db::persist::persist_sessions_transaction(
            pool,
            &[&conn.session],
            &["stats", "quest"],
        )
        .await;

        end_talk(conn, out);
        return true;
    }

    false
}

/// Sourcing 40 escalating wave formations from `Data/eve.emg` (`scene_eve_data[10851]`)
/// and falling back to deterministic templates when empty (e.g. in unit tests).
pub fn build_40_npc_waves(data: &GameData) -> Vec<Vec<(crate::data::tables::Npc, u8, u8)>> {
    // 1. Check if scene_eve_data has map 10851 with at least 40 fight_datas
    if let Some(scene) = data.scene_eve_data.get(&10851) {
        if scene.fight_datas.len() >= 40 {
            let mut sorted_fights: Vec<_> = scene.fight_datas.iter().collect();
            sorted_fights.sort_by_key(|(k, _)| **k);
            let mut waves = Vec::with_capacity(40);
            for (_, fight) in sorted_fights.into_iter().take(40) {
                let mut wave_npcs = Vec::new();
                for enemy in &fight.left_enemies {
                    let npc = if let Some(n) = data.npcs.get(&i64::from(enemy.npc_id)) {
                        n.clone()
                    } else {
                        let mut t = crate::data::tables::Npc::default();
                        t.id = i64::from(enemy.npc_id);
                        t.hp = 100;
                        t.lv = 10;
                        t
                    };
                    wave_npcs.push((npc, enemy.row(), enemy.col()));
                }
                if !wave_npcs.is_empty() {
                    waves.push(wave_npcs);
                }
            }
            if waves.len() == 40 {
                return waves;
            }
        }
    }

    // 2. Deterministic fallback templates when scene_eve_data is empty (unit tests)
    let mut waves = Vec::with_capacity(40);
    for wave_idx in 1..=40 {
        let mut wave_npcs = Vec::new();
        let npc_id = 30000 + wave_idx as i64;
        let lv = (wave_idx as i64 * 3).min(200);
        let hp = 100 + wave_idx as i64 * 50;
        let npc = if let Some(existing) = data.npcs.get(&npc_id) {
            existing.clone()
        } else {
            let mut template = crate::data::tables::Npc::default();
            template.id = npc_id;
            let name_str = format!("Arena NPC Wave {}", wave_idx);
            template.name = crate::encoding::viscii_encode(&name_str);
            template.lv = lv;
            template.hp = hp;
            template.sp = 100;
            template.atk = 50 + wave_idx as i64 * 5;
            template.thuoctinh = (wave_idx % 4 + 1) as i64;
            template
        };
        wave_npcs.push((npc, 0u8, 2u8));
        waves.push(wave_npcs);
    }
    waves
}

/// Conclude 40-NPC Arena battle on wave 40 victory:
/// Awards milestone rewards (26051 Đậu Dũng Khí, 23086 Lông Vũ, 10,000 EXP),
/// resets QuestStep(218 = 0), and warps the challenger back to lobby map 12001.
pub fn resolve_40_npc_victory(session: &mut crate::server::session::Session, data: &GameData) -> Vec<String> {
    let mut frames = Vec::new();

    // 1. Milestone rewards: 26051 (Đậu Dũng Khí), 23086 (Lông Vũ), 10,000 EXP
    let item_dau = if let Some(def) = data.items.get(&26051) {
        InventoryItem::from_template(def, 1)
    } else {
        InventoryItem {
            id: 26051,
            count: 1,
            ..Default::default()
        }
    };
    session.add_homdo_item(item_dau);

    let item_long = if let Some(def) = data.items.get(&23086) {
        InventoryItem::from_template(def, 1)
    } else {
        InventoryItem {
            id: 23086,
            count: 1,
            ..Default::default()
        }
    };
    session.add_homdo_item(item_long);

    session.texp = session.texp.saturating_add(10_000);

    // 2. Reset QuestStep(218 = 0)
    session.quest_tasks.insert(218, (0, 0));

    // 3. Warp back to lobby map 12001
    let _old_map = session.map_id;
    session.map_id = 12001;
    session.map_x = 400;
    session.map_y = 400;

    frames.push(session.dump_homdo());
    frames.push("F44402001407".to_string());
    frames.push(crate::server::spawn::build_relocate_packet(session.id, 12001, 400, 400, 0));
    frames.push(crate::server::spawn::sys_msg_frame("Chuc mung da vuot qua 40 NPC Loi Dai!"));
    frames
}
