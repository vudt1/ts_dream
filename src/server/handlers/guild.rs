//! Guild Warfare: City Siege, Water War, and Dragon Boat Racing (Ticket 07).
//!
//! Implements:
//! - Guild wire protocol (`0x27`) with Two-Phase State Locking on 2-player operations.
//! - City Siege (`0x29`) across 5 cities with ownership table (`29 0F`), announcements (`29 09`), and gate barriers (`0x03`).
//! - Water War (`0x3F`, `0x40`) with 25-slot boat inventory (`17 64` / `17 65`) and bombardment actions (`40 01..03`).
//! - Dragon Boat Racing (`0x46`) with alternating paddle inputs and token bucket rate limiter (5/sec).

use crate::encoding;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::{
    lock_online_sessions, lock_player_operations, registry_update_if_online, InventoryItem,
};
use crate::server::spawn::sys_msg_frame;
use std::time::{SystemTime, UNIX_EPOCH};

// City definitions
pub const CITY_THO_XUAN: u8 = 1;
pub const CITY_TRUONG_AN: u8 = 2;
pub const CITY_GIANG_LANG: u8 = 3;
pub const CITY_KHUC_A: u8 = 4;
pub const CITY_LAC_DUONG: u8 = 5;

// Dragon boat maps
pub const DRAGON_BOAT_MAP_MIN: u16 = 60510;
pub const DRAGON_BOAT_MAP_MAX: u16 = 60521;

// Paddle directions
pub const PADDLE_LEFT: u8 = 1;
pub const PADDLE_RIGHT: u8 = 2;

// Token bucket parameters for dragon boat racing
pub const DRAGON_BOAT_MAX_TOKENS: f64 = 5.0;
pub const DRAGON_BOAT_REFILL_RATE: f64 = 5.0; // 5 tokens per second

/// Build City Siege ownership table frame: `S→C 29 0F [cities: 5 entries]`.
pub fn build_city_ownership_raw(cities: &[(u8, u32, &str)]) -> Vec<u8> {
    let mut payload = vec![0x29, 0x0F];
    payload.push(cities.len() as u8);
    for &(city_id, guild_id, guild_name) in cities {
        payload.push(city_id);
        payload.extend_from_slice(&guild_id.to_le_bytes());
        let name_viscii = encoding::viscii_encode(guild_name);
        payload.push(name_viscii.len().min(255) as u8);
        payload.extend_from_slice(&name_viscii);
    }
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

pub fn city_ownership_frame(cities: &[(u8, u32, &str)]) -> String {
    encoder::hex(&build_city_ownership_raw(cities))
}

/// Build City Siege announcement frame: `S→C 29 09 [notice_code: 1..=13][city_id: u8]`.
pub fn build_city_siege_notice_raw(notice_code: u8, city_id: u8, extra: Option<&str>) -> Vec<u8> {
    let mut payload = vec![0x29, 0x09, notice_code, city_id];
    if let Some(extra_str) = extra {
        let viscii = encoding::viscii_encode(extra_str);
        payload.push(viscii.len().min(255) as u8);
        payload.extend_from_slice(&viscii);
    }
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

pub fn city_siege_notice_frame(notice_code: u8, city_id: u8, extra: Option<&str>) -> String {
    encoder::hex(&build_city_siege_notice_raw(notice_code, city_id, extra))
}

/// Build City Gate collision barrier broadcast via opcode `0x03`.
pub fn build_city_gate_barrier_raw(city_id: u8, gate_id: u8, intact: bool) -> Vec<u8> {
    let state_byte = if intact { 1 } else { 0 };
    vec![0xF4, 0x44, 0x04, 0x00, 0x03, city_id, gate_id, state_byte]
}

pub fn city_gate_barrier_frame(city_id: u8, gate_id: u8, intact: bool) -> String {
    encoder::hex(&build_city_gate_barrier_raw(city_id, gate_id, intact))
}

/// Build 25-slot boat inventory init frame: `S→C 17 64 [slot_count: u8]`.
pub fn build_boat_inventory_init_raw(slots: &[InventoryItem]) -> Vec<u8> {
    let mut payload = vec![0x17, 0x64, 25]; // 25 slots
    for item in slots.iter().take(25) {
        payload.push(item.slot);
        payload.extend_from_slice(&item.id.to_le_bytes());
        payload.push(item.count);
    }
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

pub fn boat_inventory_init_frame(slots: &[InventoryItem]) -> String {
    encoder::hex(&build_boat_inventory_init_raw(slots))
}

/// Build boat inventory clear frame: `S→C 17 65`.
pub fn build_boat_inventory_clear_raw() -> Vec<u8> {
    vec![0xF4, 0x44, 0x02, 0x00, 0x17, 0x65]
}

pub fn boat_inventory_clear_frame() -> String {
    encoder::hex(&build_boat_inventory_clear_raw())
}

/// Enter Water War: initialize 25-slot boat inventory and return S->C 17 64 frame.
pub fn enter_water_war(session: &mut crate::server::session::Session) -> String {
    session.boat_inventory = Vec::with_capacity(25);
    for slot in 1..=25 {
        session.boat_inventory.push(InventoryItem {
            slot: slot as u8,
            id: 0,
            count: 0,
            ..Default::default()
        });
    }
    boat_inventory_init_frame(&session.boat_inventory)
}

/// Leave Water War: clear temporary boat inventory and return S->C 17 65 frame.
pub fn leave_water_war(session: &mut crate::server::session::Session) -> String {
    session.boat_inventory.clear();
    boat_inventory_clear_frame()
}


/// Build naval bombardment damage frame: `S→C 40 01 [action, target_id: 4B LE, damage: 2B LE]`.
pub fn build_naval_damage_raw(action: u8, target_id: u32, damage: u16) -> Vec<u8> {
    let mut payload = vec![0x40, 0x01, action];
    payload.extend_from_slice(&target_id.to_le_bytes());
    payload.extend_from_slice(&damage.to_le_bytes());
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

/// Build naval visual effect frame: `S→C 40 02 [effect_id: u8, target_id: 4B LE]`.
pub fn build_naval_effect_raw(effect_id: u8, target_id: u32) -> Vec<u8> {
    let mut payload = vec![0x40, 0x02, effect_id];
    payload.extend_from_slice(&target_id.to_le_bytes());
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

/// Build naval boat status / sink frame: `S→C 40 03 [target_id: 4B LE, status: u8]`.
pub fn build_naval_status_raw(target_id: u32, status: u8) -> Vec<u8> {
    let mut payload = vec![0x40, 0x03];
    payload.extend_from_slice(&target_id.to_le_bytes());
    payload.push(status);
    let len = payload.len() as u16;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&[0xF4, 0x44]);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

/// Token bucket check and update for dragon boat racing.
/// Returns `true` if packet is accepted, `false` if dropped due to exceeding 5 packets/sec.
pub fn check_dragon_boat_rate_limit(
    tokens: &mut f64,
    last_ms: &mut u64,
    now_ms: u64,
) -> bool {
    let elapsed_sec = if *last_ms == 0 || now_ms < *last_ms {
        0.0
    } else {
        (now_ms - *last_ms) as f64 / 1000.0
    };

    *tokens = (*tokens + elapsed_sec * DRAGON_BOAT_REFILL_RATE).min(DRAGON_BOAT_MAX_TOKENS);
    *last_ms = now_ms;

    if *tokens >= 1.0 {
        *tokens -= 1.0;
        true
    } else {
        false
    }
}

/// Handle Guild operations (Opcode 0x27).
pub async fn handle_guild(ctx: &mut OpcodeCtx<'_>) {
    let my_id = ctx.conn.session.id;

    match ctx.sub {
        // 0x01: Guild Creation
        0x01 => {
            if ctx.conn.session.guild_id != 0 {
                ctx.out.send(sys_msg_frame("Ban da gia nhap quan doan"));
                return;
            }
            let name = encoding::viscii_decode(ctx.payload);
            let name = name.trim();
            if name.is_empty() {
                ctx.out.send(sys_msg_frame("Ten quan doan khong hop le"));
                return;
            }
            if let Some(pool) = ctx.env.pool {
                match crate::db::domain::create_guild(pool, name, my_id as i64).await {
                    Ok(guild_id) => {
                        ctx.conn.session.guild_id = guild_id as u32;
                        registry_update_if_online(&ctx.conn.session);
                        ctx.out.send(encoder::hex(&[0xF4, 0x44, 0x03, 0x00, 0x27, 0x01, 0x00]));
                        ctx.out.send(sys_msg_frame(&format!("Lap quan doan {} thanh cong!", name)));
                    }
                    Err(_) => {
                        ctx.out.send(sys_msg_frame("Khong the tao quan doan (ten da ton tai)"));
                    }
                }
            } else {
                ctx.conn.session.guild_id = 1;
                registry_update_if_online(&ctx.conn.session);
                ctx.out.send(encoder::hex(&[0xF4, 0x44, 0x03, 0x00, 0x27, 0x01, 0x00]));
            }
        }

        // 0x02: Guild Invite (Two-Phase State Lock on 2 players)
        0x02 => {
            if ctx.payload.len() < 4 {
                return;
            }
            let target_id = u32::from_le_bytes([ctx.payload[0], ctx.payload[1], ctx.payload[2], ctx.payload[3]]);
            if target_id == 0 || target_id == my_id {
                return;
            }

            // Phase 1: Lock ordering strictly min(id1, id2), max(id1, id2)
            let first = my_id.min(target_id);
            let second = my_id.max(target_id);
            let _guards = lock_player_operations([first, second]).await;

            // Read target session
            let target_session = lock_online_sessions().get(&target_id).cloned();
            let Some(target) = target_session else {
                ctx.out.send(sys_msg_frame("Nguoi choi khong online"));
                return;
            };

            if target.guild_id != 0 {
                ctx.out.send(sys_msg_frame("Nguoi choi da co quan doan"));
                return;
            }

            let my_guild_id = ctx.conn.session.guild_id;
            if my_guild_id == 0 {
                ctx.out.send(sys_msg_frame("Ban chua co quan doan"));
                return;
            }

            // Phase 2: Add member
            if let Some(pool) = ctx.env.pool {
                let _ = crate::db::domain::add_guild_member(pool, my_guild_id as i64, target_id as i64, 0).await;
            }

            // Update target in-memory online session
            {
                let mut online = lock_online_sessions();
                if let Some(target_sess) = online.get_mut(&target_id) {
                    target_sess.guild_id = my_guild_id;
                }
            }

            // Build invite notification frame: S→C 27 02 [my_id: 4B LE][my_guild_id: 4B LE]
            let mut invite_frame = vec![0x27, 0x02];
            invite_frame.extend_from_slice(&my_id.to_le_bytes());
            invite_frame.extend_from_slice(&my_guild_id.to_le_bytes());
            let flen = invite_frame.len() as u16;
            let mut full = vec![0xF4, 0x44];
            full.extend_from_slice(&flen.to_le_bytes());
            full.extend_from_slice(&invite_frame);

            let invite_hex = encoder::hex(&full);

            // Deliver invite to target player if hub is available
            if let Some(hub) = ctx.env.hub {
                hub.send_to(target_id, &invite_hex).await;
            }

            ctx.out.send(invite_hex);
            ctx.out.send(sys_msg_frame("Da gui loi moi gia nhap quan doan"));
        }

        // 0x04: Guild Kick
        0x04 => {
            if ctx.payload.len() < 4 {
                return;
            }
            let target_id = u32::from_le_bytes([ctx.payload[0], ctx.payload[1], ctx.payload[2], ctx.payload[3]]);
            if target_id == my_id {
                return;
            }
            let my_guild_id = ctx.conn.session.guild_id;
            if my_guild_id == 0 {
                ctx.out.send(sys_msg_frame("Ban chua co quan doan"));
                return;
            }

            if let Some(pool) = ctx.env.pool {
                let is_leader = sqlx::query_scalar::<_, i64>("SELECT leader_id FROM guilds WHERE guild_id = ?")
                    .bind(my_guild_id as i64)
                    .fetch_optional(&pool.read)
                    .await
                    .ok()
                    .flatten()
                    == Some(my_id as i64);

                if !is_leader {
                    ctx.out.send(sys_msg_frame("Chi co Quan Chu moi co quyen truc xuat thanh vien!"));
                    return;
                }

                let _ = crate::db::domain::remove_guild_member(pool, target_id as i64).await;
            }

            // Clear target's guild_id in online sessions
            {
                let mut online = lock_online_sessions();
                if let Some(target_sess) = online.get_mut(&target_id) {
                    target_sess.guild_id = 0;
                }
            }

            if let Some(hub) = ctx.env.hub {
                hub.send_to(target_id, &sys_msg_frame("Ban da bi truc xuat khoi quan doan")).await;
            }

            ctx.out.send(sys_msg_frame("Da truc xuat thanh vien khoi quan doan"));
        }

        // 0x05: Guild Leadership Transfer (Two-Phase State Lock)
        0x05 => {
            if ctx.payload.len() < 4 {
                return;
            }
            let new_leader_id = u32::from_le_bytes([ctx.payload[0], ctx.payload[1], ctx.payload[2], ctx.payload[3]]);
            if new_leader_id == my_id {
                return;
            }
            let first = my_id.min(new_leader_id);
            let second = my_id.max(new_leader_id);
            let _guards = lock_player_operations([first, second]).await;

            let my_guild_id = ctx.conn.session.guild_id;
            if my_guild_id != 0 {
                if let Some(pool) = ctx.env.pool {
                    let is_leader = sqlx::query_scalar::<_, i64>("SELECT leader_id FROM guilds WHERE guild_id = ?")
                        .bind(my_guild_id as i64)
                        .fetch_optional(&pool.read)
                        .await
                        .ok()
                        .flatten()
                        == Some(my_id as i64);

                    if !is_leader {
                        ctx.out.send(sys_msg_frame("Chi co Quan Chu moi co quyen chuyen nhuong!"));
                        return;
                    }

                    let _ = crate::db::domain::transfer_guild_leadership(pool, my_guild_id as i64, new_leader_id as i64).await;
                }
                ctx.out.send(sys_msg_frame("Chuyen nhuong chuc vu Quan Chu thanh cong!"));
            }
        }

        // 0x06: 288-byte Badge Upload
        0x06 => {
            if ctx.payload.len() >= 288 {
                let badge = &ctx.payload[..288];
                let my_guild_id = ctx.conn.session.guild_id;
                if my_guild_id != 0 {
                    if let Some(pool) = ctx.env.pool {
                        let _ = crate::db::domain::update_guild_badge(pool, my_guild_id as i64, badge).await;
                    }
                    ctx.out.send(encoder::hex(&[0xF4, 0x44, 0x03, 0x00, 0x27, 0x06, 0x00]));
                    ctx.out.send(sys_msg_frame("Cap nhat huy hieu quan doan thanh cong!"));
                }
            }
        }

        _ => {}
    }
}

/// Handle Water War requests (Opcode 0x3F).
pub async fn handle_water_war(ctx: &mut OpcodeCtx<'_>) {
    match ctx.sub {
        // Sub 0x01: Enter Water War -> initialize 25-slot boat inventory
        0x01 => {
            let frame = enter_water_war(&mut ctx.conn.session);
            registry_update_if_online(&ctx.conn.session);
            ctx.out.send(frame);
        }
        // Sub 0x02: Leave Water War -> clear boat inventory
        0x02 => {
            let frame = leave_water_war(&mut ctx.conn.session);
            registry_update_if_online(&ctx.conn.session);
            ctx.out.send(frame);
        }
        _ => {}
    }
}

/// Handle Dragon Boat Racing (Opcode 0x46).
pub async fn handle_boat_race(ctx: &mut OpcodeCtx<'_>) {
    // Only valid on dragon boat racing maps
    let map_id = ctx.conn.session.map_id;
    if !(DRAGON_BOAT_MAP_MIN..=DRAGON_BOAT_MAP_MAX).contains(&map_id) {
        return;
    }

    if ctx.sub == 0x01 && !ctx.payload.is_empty() {
        let paddle = ctx.payload[0];
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Rate limit check via token bucket
        let accepted = check_dragon_boat_rate_limit(
            &mut ctx.conn.session.boat_paddle_tokens,
            &mut ctx.conn.session.boat_last_paddle_ms,
            now_ms,
        );

        if !accepted {
            // Drop packet exceeding 5/sec rate limit
            return;
        }

        // Alternating Left / Right check
        let last = ctx.conn.session.boat_last_paddle;
        if (paddle == PADDLE_LEFT && last != PADDLE_LEFT) || (paddle == PADDLE_RIGHT && last != PADDLE_RIGHT) {
            ctx.conn.session.boat_last_paddle = paddle;
            // Advance position slightly
            ctx.conn.session.map_x = ctx.conn.session.map_x.saturating_add(5);
            registry_update_if_online(&ctx.conn.session);

            // Echo sync position to the racer
            let mut sync_payload = vec![0x46, 0x01, paddle];
            sync_payload.extend_from_slice(&ctx.conn.session.map_x.to_le_bytes());
            let flen = sync_payload.len() as u16;
            let mut full = vec![0xF4, 0x44];
            full.extend_from_slice(&flen.to_le_bytes());
            full.extend_from_slice(&sync_payload);
            let frame_hex = encoder::hex(&full);

            ctx.out.send(&frame_hex);
            // Broadcast new position to other racers on the map
            ctx.out.broadcast(ctx.conn.session.id, frame_hex);
        }
    }
}

/// Handle Naval Combat Bombardment (Opcode 0x40).
pub async fn handle_naval_combat(ctx: &mut OpcodeCtx<'_>) {
    if ctx.sub == 0x01 && ctx.payload.len() >= 5 {
        let action = ctx.payload[0];
        let target_id = u32::from_le_bytes([ctx.payload[1], ctx.payload[2], ctx.payload[3], ctx.payload[4]]);
        let param = if ctx.payload.len() >= 6 { ctx.payload[5] } else { 1 };

        let damage = (param as u16) * 50;

        let dmg_frame = encoder::hex(&build_naval_damage_raw(action, target_id, damage));
        let eff_frame = encoder::hex(&build_naval_effect_raw(action, target_id));
        let sta_frame = encoder::hex(&build_naval_status_raw(target_id, 1));

        // Send to attacker
        ctx.out.send(&dmg_frame);
        ctx.out.send(&eff_frame);
        ctx.out.send(&sta_frame);

        // Broadcast to players on map (including target)
        ctx.out.broadcast(ctx.conn.session.id, &dmg_frame);
        ctx.out.broadcast(ctx.conn.session.id, &eff_frame);
        ctx.out.broadcast(ctx.conn.session.id, &sta_frame);

        // If target is in another session, deliver directly via hub
        if let Some(hub) = ctx.env.hub {
            hub.send_to(target_id, &dmg_frame).await;
            hub.send_to(target_id, &eff_frame).await;
            hub.send_to(target_id, &sta_frame).await;
        }
    }
}
