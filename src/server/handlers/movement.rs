//! Movement & map position handlers (Opcode 0x05, 0x06).

use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::spawn;

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
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if (sub != 1 && sub != 2) || payload.len() < 5 {
        return;
    }
    if conn.session.battle_id > 0 {
        return; // In battle → movement is ignored (Ch2 §2.3.5).
    }
    let id = conn.session.id;
    let id_leader = conn.session.id_leader;
    if id_leader > 0 && id_leader != id {
        return; // Member following a leader; the leader moves everyone.
    }
    let dir = payload[0];
    let x = encoder::u16_le(payload[1], payload[2]);
    let y = encoder::u16_le(payload[3], payload[4]);
    conn.session.gocnhin = dir;
    conn.session.map_x = x;
    conn.session.map_y = y;

    // Update current player coords in the shared online registry.
    if let Some(s) = crate::server::session::online_sessions()
        .lock()
        .unwrap()
        .get_mut(&conn.session.id)
    {
        s.gocnhin = dir;
        s.map_x = x;
        s.map_y = y;
    }

    if sub == 2 {
        // Echo response to server position lock reconciliation (Op 0x06 Sub 0x02):
        // Confirm reconciled position and release client walk lock with [14 08].
        out.send("F44402001408");
        return;
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
    try_trigger_random_encounter(ctx);
}

/// Check and trigger random encounter for leader / solo walker.
fn try_trigger_random_encounter(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let map_id = conn.session.map_id;

    // Guards:
    if conn.session.battle_id > 0 {
        return;
    }
    let id_leader = conn.session.id_leader;
    let id = conn.session.id;
    if id_leader > 0 && id_leader != id {
        return; // Only leader or solo walker triggers encounters
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if now_ms.saturating_sub(conn.session.last_battle_end_ms) < 3000 {
        return; // 3-second post-battle cooldown
    }

    let Some(scene) = ctx.data.scene_eve_data.get(&u32::from(map_id)) else {
        return;
    };
    if scene.encounters.is_empty() {
        return;
    }

    // Step counting
    conn.session.encounter_steps = conn.session.encounter_steps.saturating_add(1);
    if conn.session.encounter_threshold == 0 {
        conn.session.encounter_threshold = 20;
    }
    if conn.session.encounter_steps < conn.session.encounter_threshold {
        return;
    }

    // Hit-test encounters for current position
    let px = conn.session.map_x;
    let py = conn.session.map_y;
    let matching_encounter = scene.encounters.iter().find(|enc| enc.contains(px, py));
    let Some(encounter) = matching_encounter else {
        return;
    };

    let mut rng = crate::battle::rng::DotNetRandom::time_seeded();
    conn.session.encounter_steps = 0;
    conn.session.encounter_threshold = rng.next_range(15, 31) as u32;

    // Resolve battle from encounter's event list
    let state = crate::server::handlers::npc_event::snapshot_state(&conn.session);
    let mut chosen_fight_id: Option<u16> = None;

    for &eve_no in &encounter.events {
        let Some(event_data) = scene.npc_events.get(&u16::from(eve_no)) else {
            continue;
        };
        let Some(resolved) =
            crate::eve::resolver::resolve_event(event_data, &state, &scene.group_datas, &mut rng, None)
        else {
            continue;
        };

        for res in &resolved.results {
            if res.result_type == 3 {
                // Battle result
                let fight_id = if res.result_group_no > 0 {
                    res.result_mean_no
                } else {
                    res.result_mean_no
                };
                if fight_id > 0 {
                    chosen_fight_id = Some(fight_id);
                    break;
                }
            }
        }
        if chosen_fight_id.is_some() {
            break;
        }
    }

    let Some(fight_id) = chosen_fight_id else {
        return;
    };
    let Some(fight_data) = scene.fight_datas.get(&fight_id) else {
        return;
    };

    let diahinh = scene
        .scene_infos
        .get(&1)
        .map(|s| s.background_no as i32)
        .unwrap_or(112);

    ctx.service
        .start_encounter_battle(&mut conn.session, fight_data, diahinh);
}
