//! Action / Talk handler (Opcode 0x14): H1 start talk, H6 menus, H4 end talk, H8 warp talk, H9 select menu.
//!
//! Identity rules (ticket 18 review):
//! - **Map Object ID** (`map_object_id` = `idtalking`): the on-map instance
//!   from the H1 request (LE16). It keys `Data_Talks`/quests and is embedded
//!   hex-encoded in the special NPC packets.
//! - **NPC Template ID** (`npc_id`): resolved `(map, map_object_id) →
//!   NpcOnMap.NpcId`. The H1/H6 **special** branches (banker/inn/`16012`) are
//!   selected on this id, never on the object id.
//! - A **Talk Context** tracks `{talk_type, map_object_id, talk_count, select_menu}`.

use crate::data::loader::GameData;
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::handlers::stats::build_stat_update;
use crate::server::session::Conn;
use sqlx::MySqlPool;

/// EndTalk packet + reset the whole talk context.
pub fn end_talk(conn: &mut Conn, out: &mut HandleOutcome) {
    out.send("F44402001408");
    conn.session.idtalking = 0;
    conn.session.select_menu = 0;
    conn.session.talk_count = 0;
    conn.session.warp_finish = false;
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

/// Dispatch Opcode 0x14 — Action / Talk.
pub async fn handle_talk(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let data = ctx.data;
    let pool = ctx.env.pool;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        1 => handle_talk_start(conn, payload, data, out),
        4 => end_talk(conn, out),
        6 => handle_talk_continue(conn, payload, data, pool, out).await,
        8 => handle_talk_warp(conn, payload, data, out),
        9 => handle_talk_select_menu(conn, payload),
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
    let npc = data
        .npc_on_map
        .iter()
        .find(|n| n.map_id == i64::from(conn.session.map_id) && n.id == i64::from(map_object_id))?;
    let dx = i64::from(conn.session.map_x) - npc.x;
    let dy = i64::from(conn.session.map_y) - npc.y;
    let in_range = (-150..=150).contains(&dx) && (-150..=150).contains(&dy);
    Some((npc.npc_id as i32, in_range))
}

fn handle_talk_start(conn: &mut Conn, payload: &[u8], data: &GameData, out: &mut HandleOutcome) {
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

    // Generic dialog: the distance gate still applies.
    if !in_range {
        end_talk(conn, out);
        return;
    }
    let key = crate::server::handlers::quest::quest_key(
        i64::from(conn.session.map_id),
        "NPC",
        i64::from(map_object_id),
        crate::server::handlers::quest::current_step(conn),
    );
    if let Some(talk) = data.talks.get(&key) {
        out.send("F44402000602");
        let sum: i64 = talk.teamdef.iter().sum();
        if talk.dialogs.is_empty() && sum > 0 {
            crate::server::handlers::quest::trigger_teamdef(conn, &talk.teamdef, out);
            return;
        }
        talk_messages(conn, &talk.dialogs, out);
    } else {
        out.send("F44402000602");
        out.send(format!(
            "F44411001401000000010103{:02X}000000000000C830",
            map_object_id
        ));
    }
}

async fn handle_talk_continue(
    conn: &mut Conn,
    _payload: &[u8],
    data: &GameData,
    pool: Option<&MySqlPool>,
    out: &mut HandleOutcome,
) {
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
            30 => out.send("F44411001401000000010603010000000000000100"),
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => out.send("F44411001401000000010603010000000000000100"),
            33 => {
                crate::server::handlers::quest::save_map(conn);
                crate::db::persist::update_player(
                    pool,
                    conn.session.id,
                    "savemap",
                    i64::from(conn.session.savemap),
                )
                .await;
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
            30 => out.send("F44411001401000000010603010000000000000200"),
            31 => {
                conn.session.hp = conn.session.hp_max;
                conn.session.sp = conn.session.sp_max;
                out.send(build_stat_update(0x19, conn.session.hp as i32));
                out.send(build_stat_update(0x1A, conn.session.sp as i32));
                end_talk(conn, out);
            }
            32 => out.send("F44411001401000000010603010000000000000200"),
            33 => {
                crate::server::handlers::quest::save_map(conn);
                crate::db::persist::update_player(
                    pool,
                    conn.session.id,
                    "savemap",
                    i64::from(conn.session.savemap),
                )
                .await;
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

    // Generic data-driven quest path.
    if !crate::server::handlers::quest::try_quest_h6(conn, data, out) {
        end_talk(conn, out);
    }
}

fn handle_talk_warp(conn: &mut Conn, payload: &[u8], data: &GameData, out: &mut HandleOutcome) {
    if payload.len() >= 2 {
        conn.session.idtalking = encoder::u16_le(payload[0], payload[1]) as i32;
    }
    crate::server::handlers::quest::handle_warp_confirm(conn, data, out);
}

fn handle_talk_select_menu(conn: &mut Conn, payload: &[u8]) {
    if !payload.is_empty() {
        conn.session.select_menu = payload[0] as i32;
    }
}
