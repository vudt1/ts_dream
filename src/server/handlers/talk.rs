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
use crate::data::loader::GameData;
use crate::db::pool::DbPool;
use crate::eve::auto_chain::{EventPhase, EventTriggerKind};
use crate::protocol::codecs::npc_talk::{NpcTalkCodec, TalkLockMode};
use crate::protocol::encoder;
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::handlers::npc_event::{self, NpcTrigger};
use crate::server::handlers::stats::build_stat_update;
use crate::server::session::{Conn, Session};

pub use super::eve_runner::{
    activate_event_session, execute_event_step, execute_event_step_with_pool,
    resume_eve_after_battle, resume_eve_after_battle_deferred_warp, resume_eve_after_battle_full,
    resume_eve_after_battle_sync, resume_eve_after_battle_sync_full, PreFlightError, StepOutcome,
};

/// Reset the legacy talk-context fields (`typetalk`, menu, warp confirm)
/// plus the active event session — the packet-less half of [`end_talk_session`],
/// shared with the CP4 event finish path (whose frames were already sent).
pub(crate) fn reset_talk_context(session: &mut Session) {
    session.idtalking = 0;
    session.select_menu = 0;
    session.talk_count = 0;
    session.warp_finish = false;
    session.talk_locked = false;
    session.current_event_session = None;
}

/// EndTalk packet + reset the whole talk context, over a bare [`Session`].
pub fn end_talk_session(session: &mut Session, out: &mut HandleOutcome) {
    if session.current_event_session.is_some() || session.talk_locked {
        let char_id = session.id;
        out.send(NpcTalkCodec::build_talk_lock_hex(
            char_id,
            TalkLockMode::Unlock,
        ));
    }
    out.send("F44402001408");
    reset_talk_context(session);
}

/// [`end_talk_session`] over the connection wrapper (legacy call sites).
pub fn end_talk(conn: &mut Conn, out: &mut HandleOutcome) {
    end_talk_session(&mut conn.session, out);
}

/// Wall-clock millisecond seed for the Eve RNG.
pub(crate) fn eve_rng_seed() -> i32 {
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
        end_talk(conn, out);
        return;
    };
    conn.session.idnpctalking = template_id;

    if matches!(template_id, 16080 | 16004 | 16011 | 16015) {
        if !in_range {
            end_talk(conn, out);
            return;
        }
        out.send(NpcTalkCodec::build_move_stop_hex());
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
        out.send(NpcTalkCodec::build_move_stop_hex());
        out.send(format!(
            "F44411001401000000010603{:02X}0000000000000200",
            map_object_id
        ));
        return;
    }
    if template_id == 16012 {
        return;
    }

    if crate::server::handlers::event_npcs::try_handle_event_npc(
        template_id,
        in_range,
        conn,
        data,
        service,
        pool,
        out,
    )
    .await
    {
        return;
    }

    if !in_range {
        end_talk(conn, out);
        return;
    }

    let mut rng = DotNetRandom::new(eve_rng_seed());
    if let Some(event_session) = npc_event::resolve_npc_event(
        &conn.session,
        data,
        NpcTrigger::ClickNpc(map_object_id),
        &mut rng,
    ) {
        activate_event_session(&mut conn.session, event_session, data, pool, out).await;
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
    if conn.session.current_event_session.is_some() {
        if let Some(ev) = conn.session.current_event_session.as_mut() {
            if ev.phase == EventPhase::AwaitingChoice {
                tracing::debug!("ignoring talk-continue while awaiting Surface choice");
                return;
            }
            if ev.phase == EventPhase::AwaitingBattle {
                tracing::debug!("ignoring talk-continue while awaiting eve battle");
                return;
            }
            ev.current_index = ev.current_index.saturating_add(1);
        }
        execute_event_step_with_pool(&mut conn.session, data, pool, out).await;
        return;
    }

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
            _ => end_talk(conn, out),
        }
        return;
    }

    if matches!(template, 15002 | 16001 | 16016 | 15118) {
        handle_inn_menu(conn, data, out, "01");
        return;
    }

    if template == 16015 {
        handle_inn_menu(conn, data, out, "02");
        return;
    }

    if template == 16012 {
        return;
    }

    if conn.session.map_id == 12711 {
        crate::server::handlers::quest::generate_daily_quest(conn, data, out);
        return;
    }

    end_talk(conn, out);
}

fn handle_inn_menu(conn: &mut Conn, data: &GameData, out: &mut HandleOutcome, item_tag: &str) {
    match conn.session.select_menu {
        30 => {
            out.send(format!(
                "F4441100140100000001060301000000000000{}00",
                item_tag
            ));
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
            out.send(format!(
                "F4441100140100000001060301000000000000{}00",
                item_tag
            ));
        }
        33 => {
            crate::server::handlers::quest::save_map(&mut conn.session);
            let item = crate::server::inventory::from_template(data, 46016, 2);
            let _ = conn.session.add_homdo_item(item);
            out.send(conn.session.dump_homdo());
            end_talk(conn, out);
        }
        _ => end_talk(conn, out),
    }
}

async fn handle_talk_click_door(
    conn: &mut Conn,
    payload: &[u8],
    data: &GameData,
    pool: Option<&DbPool>,
    out: &mut HandleOutcome,
) {
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
        conn.session.idtalking = door_id;
        conn.session.talk_type = "DOOR".to_string();
        conn.session.talk_count = 0;
        activate_event_session(&mut conn.session, event_session, data, pool, out).await;
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

    let awaiting_battle = conn
        .session
        .current_event_session
        .as_ref()
        .is_some_and(|ev| ev.phase == EventPhase::AwaitingBattle);
    if awaiting_battle {
        tracing::debug!("ignoring menu select while awaiting eve battle");
        return;
    }

    let awaiting_choice = conn
        .session
        .current_event_session
        .as_ref()
        .is_some_and(|ev| ev.phase == EventPhase::AwaitingChoice);
    if awaiting_choice {
        if choice == 0 {
            tracing::debug!("ignoring menu select with default ChoiceCode 0");
            return;
        }
        if !matches!(choice, 20 | 21 | 40 | 30..=39 | 61..=u8::MAX) {
            tracing::warn!(
                choice,
                "invalid TSurface choice code in 0x14 09; closing talk safely"
            );
            end_talk(conn, out);
            return;
        }
        let trigger_and_surface = conn
            .session
            .current_event_session
            .as_mut()
            .map(|ev| {
                ev.last_choice_code = i32::from(choice);
                ev.phase = EventPhase::Executing;
                ev.current_index = ev.current_index.saturating_add(1);
                let trigger = match ev.trigger_kind {
                    EventTriggerKind::ClickNpc => NpcTrigger::ClickNpc(ev.npc_click_id),
                    EventTriggerKind::ClickDoor => NpcTrigger::ClickDoor(ev.npc_click_id),
                    EventTriggerKind::MeetDoor => NpcTrigger::MeetDoor(ev.npc_click_id),
                    EventTriggerKind::WalkStep => NpcTrigger::WalkStepEncounter(ev.npc_click_id),
                };
                (trigger, ev.last_surface_id)
            });
        conn.session.select_menu = i32::from(choice);

        let Some((trigger, surface_id)) = trigger_and_surface else {
            end_talk(conn, out);
            return;
        };
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
                if let Some(prev) = conn.session.current_event_session.as_ref() {
                    branch.last_surface_id = prev.last_surface_id;
                    branch.last_choice_code = prev.last_choice_code;
                }
                conn.session.current_event_session = Some(branch);
                execute_event_step_with_pool(&mut conn.session, data, pool, out).await;
            }
            None => end_talk(conn, out),
        }
        return;
    }

    conn.session.select_menu = i32::from(choice);
}
