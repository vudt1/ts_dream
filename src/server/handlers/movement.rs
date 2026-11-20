//! Movement & map position handlers (Opcode 0x05, 0x06).

use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::spawn;

/// Async dispatcher entry for Op 0x05 (`handle_player_update`).
pub async fn handle_player_update_dispatch(ctx: &mut OpcodeCtx<'_>) {
    if ctx.sub == 1 && ctx.payload.len() >= 5 {
        handle_move_dispatch(ctx).await;
        return;
    }
    handle_player_update(ctx);
}

/// Async dispatcher entry for Op 0x06 (`handle_move`) + `MeetDoor` / `WalkStep` Eve triggers.
pub async fn handle_move_dispatch(ctx: &mut OpcodeCtx<'_>) {
    let cached_state = if (ctx.sub == 1 || ctx.sub == 2)
        && ctx.payload.len() >= 5
        && ctx.conn.session.battle_id == 0
        && ctx.conn.session.id_leader == ctx.conn.session.id
        && ctx.conn.session.id > 0
        && ctx
            .conn
            .session
            .id_mem
            .iter()
            .any(|&m| m > 0 && m != ctx.conn.session.id)
    {
        let mut participant_ids = vec![ctx.conn.session.id];
        for &m in &ctx.conn.session.id_mem {
            if m > 0 && m != ctx.conn.session.id {
                participant_ids.push(m);
            }
        }
        participant_ids.sort_unstable();
        participant_ids.dedup();
        // Hold Two-Phase State Lock only across position sync; drop before MeetDoor/WalkStep
        // so downstream `perform_party_warp_with_resolved` can re-acquire without deadlocking.
        let _guards = crate::server::session::lock_player_operations(participant_ids).await;
        handle_move_inner(ctx)
    } else {
        handle_move_inner(ctx)
    };
    if ctx.sub != 1
        || ctx.payload.len() < 5
        || ctx.conn.session.battle_id > 0
        || ctx.conn.session.current_event_session.is_some()
    {
        return;
    }
    let id = ctx.conn.session.id;
    let id_leader = ctx.conn.session.id_leader;
    if id_leader > 0 && id_leader != id {
        return;
    }

    let Some(scene) = ctx.data.scene_eve_data.get(&u32::from(ctx.conn.session.map_id)) else {
        return;
    };

    let px = ctx.conn.session.map_x;
    let py = ctx.conn.session.map_y;
    let origin = ctx.conn.session.walk_origin;

    // Fast-path: check if any door is within 50 px Euclidean (and the player is moving
    // toward the door or landing on its center) or any encounter zone contains (px, py)
    // before seeding RNG or building a PlayerEventState snapshot.
    const MEET_DOOR_RADIUS_SQ: i64 = 50 * 50;
    const MEET_DOOR_CENTER_SQ: i64 = 25 * 25;
    let mut closest_door: Option<(u16, i64)> = None;
    for door in scene.doors.values() {
        let dx = i64::from(px) - i64::from(door.x);
        let dy = i64::from(py) - i64::from(door.y);
        let dist2 = dx * dx + dy * dy;
        if dist2 <= MEET_DOOR_RADIUS_SQ {
            let ox = i64::from(origin.0) - i64::from(door.x);
            let oy = i64::from(origin.1) - i64::from(door.y);
            let origin_dist2 = ox * ox + oy * oy;
            if dist2 < origin_dist2 || dist2 <= MEET_DOOR_CENTER_SQ {
                if closest_door.map_or(true, |(_, best)| dist2 < best) {
                    closest_door = Some((door.id, dist2));
                }
            }
        }
    }
    let in_encounter_zone = scene.encounters.iter().any(|enc| enc.contains(px, py));
    if closest_door.is_none() && !in_encounter_zone {
        return;
    }

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i32)
        .unwrap_or(12345);
    let mut rng = crate::battle::rng::DotNetRandom::new(seed);

    // 1. MeetDoor: check closest door within ±60 px of new coordinates.
    if let Some((door_id, _)) = closest_door {
        let now_ms = crate::server::session::now_ms();
        let door_blocked = ctx.conn.session.talk_locked
            || ctx
                .conn
                .session
                .door_trigger_cooldown
                .is_some_and(|(m, d, until)| {
                    m == ctx.conn.session.map_id && d == door_id && now_ms < until
                });
        if !door_blocked {
            if let Some(door) = scene.doors.get(&door_id) {
                let origin = ctx.conn.session.walk_origin;
                let dx = f64::from(door.x) - f64::from(origin.0);
                let dy = f64::from(door.y) - f64::from(origin.1);
                let dist = (dx * dx + dy * dy).sqrt();
                let travel_ms =
                    ((dist / ctx.conn.session.walk_speed_px_per_sec()) * 1000.0).round() as u64;
                if travel_ms > 250 {
                    ctx.conn.session.pending_meet_door =
                        Some(crate::server::session::PendingMeetDoor {
                            map_id: ctx.conn.session.map_id,
                            door_id,
                            due_ms: now_ms.saturating_add(travel_ms),
                        });
                    return;
                }
            }
            if trigger_meet_door_for_door(
                &mut ctx.conn.session,
                door_id,
                ctx.data,
                &ctx.env,
                &mut ctx.out,
                cached_state.as_ref(),
            )
            .await
            {
                return;
            }
        }
    }

    // 2. WalkStep: scripted Eve encounter-zone events (non-random-battle).
    if in_encounter_zone {
        if let Some(event_session) =
            crate::server::handlers::npc_event::resolve_npc_event_with_filter_and_state(
                &ctx.conn.session,
                ctx.data,
                crate::server::handlers::npc_event::NpcTrigger::WalkStep { x: px, y: py },
                &mut rng,
                None,
                cached_state.as_ref(),
            )
        {
            crate::server::handlers::talk::activate_event_session(
                &mut ctx.conn.session,
                event_session,
                ctx.data,
                ctx.env.pool,
                &mut ctx.out,
            )
            .await;
        }
    }
}

/// Shared `MeetDoor` execution path for both immediate `06 01` and delayed walk arrival.
///
/// Returns `true` if the door trigger handled/terminated the step (festival block,
/// BattleGate battle, or Eve `MeetDoor` event activated).
pub async fn trigger_meet_door_for_door(
    session: &mut crate::server::session::Session,
    door_id: u16,
    data: &crate::data::loader::GameData,
    env: &crate::server::dispatcher::ServerEnv<'_>,
    out: &mut crate::server::dispatcher::HandleOutcome,
    cached_state: Option<&crate::eve::state::PlayerEventState>,
) -> bool {
    let map_id = session.map_id;
    let now_ms = crate::server::session::now_ms();
    let cooldown_active = session
        .door_trigger_cooldown
        .is_some_and(|(m, d, until)| m == map_id && d == door_id && now_ms < until);
    if session.talk_locked || cooldown_active {
        return false;
    }
    if let Some(hub) = env.hub {
        let target_map_id = crate::server::handlers::quest::resolve_door_warp(
            data,
            session,
            map_id,
            door_id,
        )
        .map(|r| r.dest_map)
        .unwrap_or(0);
        if let Err(msg) = hub
            .festivals
            .check_door_access(map_id, door_id, target_map_id)
        {
            out.send(crate::server::spawn::sys_msg_frame(msg));
            return true;
        }
    }
    if let Some(trigger) = crate::server::handlers::quest::try_trigger_battle_gate(
        session,
        data,
        map_id,
        door_id,
    ) {
        out.battle_trigger = Some(trigger);
        return true;
    }
    let seed = crate::server::session::now_ms() as i32;
    let mut rng = crate::battle::rng::DotNetRandom::new(seed);
    if let Some(event_session) =
        crate::server::handlers::npc_event::resolve_npc_event_with_filter_and_state(
            session,
            data,
            crate::server::handlers::npc_event::NpcTrigger::MeetDoor(i32::from(door_id)),
            &mut rng,
            None,
            cached_state,
        )
    {
        if let Some(hub) = env.hub {
            if let Err(msg) = crate::server::handlers::talk::check_event_session_quest_access(
                &hub.festivals,
                map_id,
                &event_session,
            ) {
                out.send(crate::server::spawn::sys_msg_frame(msg));
                return true;
            }
        }
        session.idtalking = i32::from(door_id);
        session.talk_type = "DOOR".to_string();
        session.talk_count = 0;
        crate::server::handlers::talk::activate_event_session(
            session,
            event_session,
            data,
            env.pool,
            out,
        )
        .await;
        return true;
    }
    false
}

/// Op 0x05 — Player / Actor state sync (C->S is non-movement: sub=6 form selection, sub=7 tab switch).
pub fn handle_player_update(ctx: &mut OpcodeCtx) {
    if ctx.sub == 1 && ctx.payload.len() >= 5 {
        // Legacy compatibility: tolerate synthetic 0x05 move frames while routing properly
        handle_move(ctx);
        return;
    }
    tracing::debug!(
        "received OP_PLAYER_UPDATE (0x05) sub={} len={}",
        ctx.sub,
        ctx.payload.len()
    );
    // Sub 6 (form selection e.g. death/revive) and sub 7 (tab switch) are safe no-ops in current state.
}

/// Op 0x06 — Move & remote walk / position echo (C<->S).
/// C->S payload (9 bytes total):
/// [0x06][sub: 1B][orient: 1B][destX: 2B LE][destY: 2B LE][sigA: 1B][sigB: 1B]
pub fn handle_move(ctx: &mut OpcodeCtx) {
    let _ = handle_move_inner(ctx);
}

fn handle_move_inner(ctx: &mut OpcodeCtx) -> Option<crate::eve::state::PlayerEventState> {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if (sub != 1 && sub != 2) || payload.len() < 5 {
        return None;
    }
    if conn.session.battle_id > 0 {
        return None; // In battle → movement is ignored (Ch2 §2.3.5).
    }
    let id = conn.session.id;
    let id_leader = conn.session.id_leader;
    if id_leader > 0 && id_leader != id {
        return None; // Member following a leader; the leader moves everyone.
    }
    let dir = payload[0];
    let x = encoder::u16_le(payload[1], payload[2]);
    let y = encoder::u16_le(payload[3], payload[4]);
    let now_ms = crate::server::session::now_ms();
    let prev_pos = conn.session.estimated_position_at(now_ms);
    if sub == 1 {
        conn.session.walk_origin = (
            prev_pos.0.clamp(0, i32::from(u16::MAX)) as u16,
            prev_pos.1.clamp(0, i32::from(u16::MAX)) as u16,
        );
        conn.session.walk_dest = (x, y);
        conn.session.walk_start_ms = now_ms;
        conn.session.pending_meet_door = None;
    } else if sub == 2 {
        conn.session.walk_origin = (x, y);
        conn.session.walk_dest = (x, y);
        conn.session.walk_start_ms = 0;
        conn.session.pending_meet_door = None;
    }
    conn.session.gocnhin = dir;
    conn.session.map_x = x;
    conn.session.map_y = y;

    // Update current player coords in the shared online registry.
    crate::server::session::registry_update_if_online(&conn.session);

    if sub == 2 {
        // Echo response to server position lock reconciliation (Op 0x06 Sub 0x02):
        // Release client walk lock with [14 08] ONLY when no talk/event session holds the lock;
        // otherwise [14 08] would prematurely close the active dialog UI on aLogin.exe.
        if conn.session.current_event_session.is_none() && !conn.session.talk_locked {
            out.send("F44402001408");
        }
        return None;
    }

    // Map-scoped broadcast (never echoed to the mover; the fan-out skips the
    // walk's own subject).
    out.broadcast(id, spawn::move_broadcast(id, dir, x, y));
    if id_leader == id {
        for member in conn.session.id_mem {
            if member > 0 {
                out.broadcast(member, spawn::move_broadcast(member, dir, x, y));
                // Persist the member's new position through the shared online
                // registry so later reads see the follower's real coords.
                if let Some(mem) = crate::server::session::online_sessions()
                    .lock()
                    .unwrap()
                    .get_mut(&member)
                {
                    mem.gocnhin = dir;
                    mem.map_x = x;
                    mem.map_y = y;
                }
            }
        }
    }

    // Wild random encounter step counting & battle trigger (Section 4 — Encounter/Mine Data)
    try_trigger_random_encounter(ctx)
}

/// Check and trigger random encounter for leader / solo walker.
///
/// Returns `Some(state)` if a [`PlayerEventState`] snapshot was constructed
/// during event resolution but no battle was triggered, so `handle_move_dispatch`
/// can reuse it for `MeetDoor` / `WalkStep` without building a second snapshot.
fn try_trigger_random_encounter(ctx: &mut OpcodeCtx) -> Option<crate::eve::state::PlayerEventState> {
    let conn = &mut ctx.conn;
    let map_id = conn.session.map_id;

    // Guards:
    if conn.session.battle_id > 0 {
        return None;
    }
    let id_leader = conn.session.id_leader;
    let id = conn.session.id;
    if id_leader > 0 && id_leader != id {
        return None; // Only leader or solo walker triggers encounters
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if now_ms.saturating_sub(conn.session.last_battle_end_ms) < 3000 {
        return None; // 3-second post-battle cooldown
    }

    let scene = ctx.data.scene_eve_data.get(&u32::from(map_id))?;
    if scene.encounters.is_empty() {
        return None;
    }

    // Step counting
    conn.session.encounter_steps = conn.session.encounter_steps.saturating_add(1);
    if conn.session.encounter_threshold == 0 {
        conn.session.encounter_threshold = 20;
    }
    if conn.session.encounter_steps < conn.session.encounter_threshold {
        return None;
    }

    // Hit-test encounters for current position
    let px = conn.session.map_x;
    let py = conn.session.map_y;
    let encounter = scene.encounters.iter().find(|enc| enc.contains(px, py))?;

    let mut rng = crate::battle::rng::DotNetRandom::time_seeded();
    conn.session.encounter_steps = 0;
    conn.session.encounter_threshold = rng.next_range(15, 31) as u32;

    // Special Event: North Star Quiz encounter hook (Ticket 02)
    if ctx.service.events.is_event_active(crate::server::events::EVENT_NORTH_STAR) {
        let rate = ctx.service.events.get_north_star_rate();
        let roll = rng.next_range(1, 101) as u8;
        if roll <= rate {
            let npc_id = if rng.next_range(0, 2) == 0 { 1643 } else { 1644 };
            let diahinh = ctx.data.map_diahinh(conn.session.map_id);
            let (_battle_id, modal_frame) =
                ctx.service.start_north_star_battle(&mut conn.session, npc_id, diahinh);
            ctx.out.send(modal_frame);
            return None;
        }
    }

    // Resolve battle from encounter's event list
    let state =
        crate::server::handlers::npc_event::snapshot_state_with_data(&conn.session, Some(ctx.data), None);
    let mut chosen_fight_id: Option<u16> = None;

    let mut try_resolve_fight = |eve_no: u16| -> Option<u16> {
        let event_data = scene.npc_events.get(&eve_no)?;
        let resolved = crate::eve::resolver::resolve_event(
            event_data,
            &state,
            &scene.group_datas,
            &mut rng,
            None,
        )?;
        for res in &resolved.results {
            if res.result_type == 3 && res.result_mean_no > 0 {
                return Some(res.result_mean_no);
            }
        }
        None
    };

    if let Some(eve_nos) = scene
        .encounter_events_by_id
        .get(&encounter.id)
        .filter(|v| !v.is_empty())
        .map(Vec::as_slice)
    {
        for &eve_no in eve_nos {
            if let Some(fid) = try_resolve_fight(eve_no) {
                chosen_fight_id = Some(fid);
                break;
            }
        }
    } else {
        for &eve_no in &encounter.events {
            if let Some(fid) = try_resolve_fight(u16::from(eve_no)) {
                chosen_fight_id = Some(fid);
                break;
            }
        }
    }

    let Some(fight_id) = chosen_fight_id else {
        return Some(state);
    };
    let Some(fight_data) = scene.fight_datas.get(&fight_id) else {
        return Some(state);
    };

    let diahinh = ctx.data.map_diahinh(conn.session.map_id);

    ctx.service
        .start_encounter_battle(&mut conn.session, fight_data, diahinh);
    None
}
