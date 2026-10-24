//! Cross-player trade, storage transfer, and bank operations.
#![allow(clippy::chunks_exact_to_as_chunks)]
//!
//! The settlement engine lives in [`crate::server::trade_system`]; this module
//! keeps only wire formatting, registry plumbing, and DB persistence.

use crate::db::persist;
use crate::protocol::{encoder, frame};
use crate::server::dispatcher::OpcodeCtx;
use crate::protocol::GOLD_CAP;
use crate::server::inventory;
use crate::server::session::{lock_online_sessions, InventoryItem, PetState, Session, TradeState};

/// Default trade timeout in milliseconds (60 seconds).
pub const TRADE_TIMEOUT_MS: u64 = 60_000;

/// Check if a trade state has exceeded its allowed duration.
pub fn is_trade_timed_out(trade: &TradeState, now_ms: u64) -> bool {
    trade.active && trade.started_at_ms > 0 && now_ms.saturating_sub(trade.started_at_ms) >= TRADE_TIMEOUT_MS
}

/// Expire trade state if timed out, returning true if cancelled.
pub fn expire_trade_if_timed_out(
    session: &mut Session,
    partner: &mut Session,
    now_ms: u64,
) -> bool {
    if is_trade_timed_out(&session.trade, now_ms) || is_trade_timed_out(&partner.trade, now_ms) {
        session.trade = TradeState::default();
        partner.trade = TradeState::default();
        true
    } else {
        false
    }
}

/// Helper building S->C `19 02 <code>` trade toast packet.
pub fn trade_toast_frame(code: u8) -> String {
    format!("F44403001902{:02X}", code)
}

/// Helper building S->C `19 04` trade cancelled / close trade form frame.
pub fn trade_cancelled_frame() -> &'static str {
    "F44402001904"
}

/// Cancel a timed-out trade asynchronously, sending `19 04` and `19 02 03` toast to both participants.
pub async fn cancel_timed_out_trade(
    session: &mut Session,
    out: &mut crate::server::dispatcher::HandleOutcome,
    hub: Option<&crate::web::server_control::ServerControl>,
) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if !is_trade_timed_out(&session.trade, now) {
        return false;
    }
    let partner_id = session.trade.partner_id;
    let _locks = crate::server::session::lock_player_operations(if partner_id > 0 {
        vec![session.id, partner_id]
    } else {
        vec![session.id]
    })
    .await;
    session.trade = TradeState::default();
    if let Some(mut partner) = registry_get(partner_id) {
        partner.trade = TradeState::default();
        registry_put(partner);
    }
    registry_update_if_online(session);
    out.send(trade_cancelled_frame());
    out.send(trade_toast_frame(0x03));
    if let Some(hub) = hub {
        hub.send_to(partner_id, trade_cancelled_frame()).await;
        hub.send_to(partner_id, &trade_toast_frame(0x03)).await;
    }
    true
}

fn item_wire(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}00{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.long_val,
        inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

fn storage_entry(item: &InventoryItem) -> String {
    format!(
        "{:02X}{}{:02X}{:02X}{:02X}{:02X}{:02X}{}",
        item.slot,
        encoder::le16(item.id),
        item.count,
        item.doben,
        item.long_val,
        inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

fn storage_entry_no_slot(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}{:02X}{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.doben,
        item.long_val,
        inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

fn offered_items(s: &Session) -> String {
    s.trade
        .items
        .iter()
        .filter(|i| i.id > 0)
        .map(item_wire)
        .collect()
}

fn trade_offer_frame(s: &Session) -> String {
    frame(
        "1903",
        &format!("{}{}", encoder::le32(s.trade.gold), offered_items(s)),
    )
}

fn pet_offer_frame(gold: u32, pet: &PetState) -> String {
    let name_hex = encoder::strhex(&pet.name);
    let mut padded = name_hex.clone();
    padded.truncate(28);
    while padded.len() < 28 {
        padded.push('6');
    }
    frame(
        "190C",
        &format!(
            "{}{}{:02X}{}{}{}{}{:02X}{}{:02X}000000000000{:02X}{}{}{}{}{}{}{}00000000{:02X}{:02X}",
            encoder::le32(gold),
            encoder::le16(pet.id),
            pet.level,
            encoder::le16(pet.hp_max),
            encoder::le16(pet.hp),
            encoder::le16(pet.sp_max),
            encoder::le16(pet.sp),
            pet.name.len(),
            name_hex,
            pet.fai,
            pet.name.len(),
            padded,
            encoder::le16(pet.hp),
            encoder::le16(pet.sp),
            encoder::le16(pet.int1),
            encoder::le16(pet.atk),
            encoder::le16(pet.def),
            encoder::le16(pet.agi),
            pet.level,
            pet.fai
        ),
    )
}

fn empty_pet_offer_frame(gold: u32) -> String {
    frame(
        "190C",
        &format!("{}{}", encoder::le32(gold), "00".repeat(64)),
    )
}


fn received_pet_frame(pet: &PetState) -> String {
    frame(
        "0F07",
        &format!(
            "{}{:02X}{}0000000000{:02X}{:02X}{}",
            encoder::le32(u32::from(pet.id)),
            pet.stt,
            encoder::le32(u32::from(pet.id)),
            pet.quest,
            pet.name.len(),
            encoder::strhex(&pet.name)
        ),
    )
}

fn removed_pet_frame(owner_id: u32, stt: u8) -> String {
    format!("F44407000F02{}{:02X}", encoder::le32(owner_id), stt)
}

fn pet_presence_frame(owner_id: u32, pet: &PetState) -> String {
    format!(
        "F4440C000F01{}{:02X}{}{:02X}",
        encoder::le32(owner_id),
        pet.stt,
        encoder::le32(u32::from(pet.id)),
        pet.quest
    )
}

fn add_trade_item(s: &mut Session, item: InventoryItem) -> bool {
    crate::server::trade_system::add_trade_item(s, item)
}

fn settle_trade(a: &mut Session, b: &mut Session) -> bool {
    crate::server::trade_system::settle_trade(a, b)
}

fn pet_at(s: &Session, stt: u8) -> Option<PetState> {
    crate::server::trade_system::pet_at(s, stt)
}

fn pet_trade_settle(a: &mut Session, b: &mut Session) -> Result<(), &'static str> {
    crate::server::trade_system::pet_trade_settle(a, b)
}

fn add_storage_item(
    storage: &mut Vec<InventoryItem>,
    mut item: InventoryItem,
    capacity: u16,
) -> bool {
    let Some(slot) = crate::server::inventory::free_slot_in(storage, capacity) else {
        return false;
    };
    item.slot = slot;
    storage.push(item);
    true
}

fn add_storage_to_homdo(
    homdo: &mut Vec<InventoryItem>,
    mut item: InventoryItem,
) -> Option<InventoryItem> {
    let slot = inventory::free_slot(homdo)?;
    item.slot = slot;
    homdo.push(item.clone());
    Some(item)
}

fn registry_get(id: u32) -> Option<Session> {
    lock_online_sessions().get(&id).cloned()
}
fn registry_put(s: Session) {
    lock_online_sessions().insert(s.id, s);
}
fn registry_update_if_online(s: &Session) {
    let mut sessions = lock_online_sessions();
    if sessions.contains_key(&s.id) {
        sessions.insert(s.id, s.clone());
    }
}

/// Op 0x19.
pub async fn handle_trade(ctx: &mut OpcodeCtx<'_>) {
    let id = ctx.conn.session.id;
    let sub = ctx.sub;
    let payload = ctx.payload;

    // Check timeout if session already has an active trade
    if ctx.conn.session.trade.active
        && cancel_timed_out_trade(&mut ctx.conn.session, &mut ctx.out, ctx.env.hub).await
    {
        return;
    }

    match sub {
        crate::protocol::TRADE_SUB_REQUEST => {
            if payload.len() < 4 {
                return;
            }
            let partner_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            if partner_id == 0 || partner_id == id || ctx.conn.session.trade.active {
                ctx.out.send(trade_toast_frame(0x02));
                return;
            }
            // Two-Phase State Lock: Lock both accounts in stable ID order BEFORE reading partner session
            let _locks = crate::server::session::lock_player_operations([id, partner_id]).await;
            let Some(mut partner) = registry_get(partner_id) else {
                ctx.out.send(trade_toast_frame(0x01));
                return;
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            if is_trade_timed_out(&partner.trade, now) {
                partner.trade = TradeState::default();
                registry_put(partner.clone());
            }
            if partner.trade.active {
                ctx.out.send(trade_toast_frame(0x02));
                return;
            }
            ctx.conn.session.trade = TradeState {
                active: true,
                partner_id,
                started_at_ms: now,
                ..Default::default()
            };
            partner.trade = TradeState {
                active: true,
                partner_id: id,
                started_at_ms: now,
                ..Default::default()
            };
            registry_put(partner);
            registry_update_if_online(&ctx.conn.session);
            ctx.out.send(frame("1901", &encoder::le32(partner_id)));
            if let Some(hub) = ctx.env.hub {
                hub.send_to(partner_id, &frame("1901", &encoder::le32(id)))
                    .await;
            }
        }
        crate::protocol::TRADE_SUB_PET_REQUEST => {
            if payload.len() < 4 {
                return;
            }
            let partner_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            if partner_id == 0 || partner_id == id || ctx.conn.session.trade.active {
                ctx.out.send(trade_toast_frame(0x02));
                return;
            }
            // Two-Phase State Lock: Lock both accounts in stable ID order BEFORE reading partner session
            let _locks = crate::server::session::lock_player_operations([id, partner_id]).await;
            let Some(mut partner) = registry_get(partner_id) else {
                ctx.out.send(trade_toast_frame(0x01));
                return;
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            if is_trade_timed_out(&partner.trade, now) {
                partner.trade = TradeState::default();
                registry_put(partner.clone());
            }
            if partner.trade.active {
                ctx.out.send(trade_toast_frame(0x02));
                return;
            }
            ctx.conn.session.trade = TradeState {
                active: true,
                partner_id,
                started_at_ms: now,
                ..Default::default()
            };
            partner.trade = TradeState {
                active: true,
                partner_id: id,
                started_at_ms: now,
                ..Default::default()
            };
            registry_put(partner);
            registry_update_if_online(&ctx.conn.session);
            ctx.out.send(frame("190A", &encoder::le32(partner_id)));
            if let Some(hub) = ctx.env.hub {
                hub.send_to(partner_id, &frame("190A", &encoder::le32(id)))
                    .await;
            }
        }
        crate::protocol::TRADE_SUB_OFFER_CS => {
            if payload.len() < 4 || !ctx.conn.session.trade.active {
                return;
            }
            let gold = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            if gold > ctx.conn.session.gold {
                return;
            }
            let mut items = Vec::new();
            let mut seen_slots = std::collections::HashSet::new();
            let mut cursor = 4;
            let end = payload.len().min(14);
            while cursor + 2 <= end {
                let slot = payload[cursor];
                let qty = payload[cursor + 1];
                cursor += 2;
                if slot == 0 || qty == 0 {
                    continue;
                }
                if !(1..=25).contains(&slot) || !seen_slots.insert(slot) {
                    return;
                }
                let Some(mut item) = ctx
                    .conn
                    .session
                    .homdo
                    .iter()
                    .find(|i| i.slot == slot && i.id > 0)
                    .cloned()
                else {
                    return;
                };
                if qty > item.count {
                    return;
                }
                item.count = qty;
                items.push(item);
            }
            ctx.conn.session.trade.gold = gold;
            ctx.conn.session.trade.items = items;
            registry_update_if_online(&ctx.conn.session);
            let partner_id = ctx.conn.session.trade.partner_id;
            if let Some(partner) = registry_get(partner_id) {
                if partner.trade.active {
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(partner_id, &trade_offer_frame(&ctx.conn.session))
                            .await;
                    }
                }
            }
        }
        crate::protocol::TRADE_SUB_CONFIRM_CS => {
            let Some(action) = payload.first().copied() else {
                return;
            };
            let partner_id = ctx.conn.session.trade.partner_id;
            if partner_id == 0 {
                return;
            }
            // Two-Phase State Lock: Lock both accounts in stable ID order BEFORE reading partner session
            let _locks = crate::server::session::lock_player_operations([id, partner_id]).await;
            let Some(mut partner) = registry_get(partner_id) else {
                ctx.conn.session.trade = TradeState::default();
                registry_update_if_online(&ctx.conn.session);
                ctx.out.send(trade_cancelled_frame());
                if action == 1 {
                    ctx.out.send(trade_toast_frame(0x01));
                }
                return;
            };
            if action == 0 || action == 2 {
                ctx.conn.session.trade = TradeState::default();
                partner.trade = TradeState::default();
                registry_put(partner);
                registry_update_if_online(&ctx.conn.session);
                ctx.out.send(trade_cancelled_frame());
                if let Some(hub) = ctx.env.hub {
                    hub.send_to(partner_id, "F4440300190203").await;
                    hub.send_to(partner_id, trade_cancelled_frame()).await;
                }
            } else if action == 1 && ctx.conn.session.trade.active {
                ctx.conn.session.trade.accepted = true;
                registry_update_if_online(&ctx.conn.session);
                if !partner.trade.accepted {
                    registry_put(partner);
                    return;
                }

                let current_before = ctx.conn.session.clone();
                let partner_before = partner.clone();
                let a_offers = ctx.conn.session.trade.items.clone();
                let b_offers = partner.trade.items.clone();
                let a_gold = ctx.conn.session.trade.gold;
                let b_gold = partner.trade.gold;

                if settle_trade(&mut ctx.conn.session, &mut partner) {
                    if !persist::persist_sessions_transaction(
                        ctx.env.pool,
                        &[&ctx.conn.session, &partner],
                        &["homdo"],
                    )
                    .await
                    {
                        ctx.conn.session = current_before;
                        ctx.conn.session.trade = TradeState::default();
                        let mut p = partner_before;
                        p.trade = TradeState::default();
                        registry_put(p);
                        registry_update_if_online(&ctx.conn.session);
                        ctx.out.send("F4440300190207");
                        ctx.out.send(trade_cancelled_frame());
                        if let Some(hub) = ctx.env.hub {
                            hub.send_to(partner_id, "F4440300190207").await;
                            hub.send_to(partner_id, trade_cancelled_frame()).await;
                        }
                        return;
                    }
                    registry_put(partner.clone());
                    registry_update_if_online(&ctx.conn.session);

                    // Emit gold updates
                    if a_gold < b_gold {
                        ctx.out.send(format!("F44406001A01{}", encoder::le32(b_gold - a_gold)));
                    } else if a_gold > b_gold {
                        ctx.out.send(format!("F44406001A02{}", encoder::le32(a_gold - b_gold)));
                    }
                    ctx.out.send(crate::server::handlers::shops::gold_frame(
                        ctx.conn.session.gold,
                    ));

                    // Emit item changes (17 09 remove, 17 06 add)
                    for item in &a_offers {
                        ctx.out.send(format!("F44404001709{:02X}{:02X}", item.slot, item.count));
                    }
                    for item in &b_offers {
                        ctx.out.send(crate::server::inventory::item_added_frame(item));
                    }
                    ctx.out.send("F4440300190204");
                    ctx.out.send(trade_cancelled_frame());

                    if let Some(hub) = ctx.env.hub {
                        if b_gold < a_gold {
                            hub.send_to(partner_id, &format!("F44406001A01{}", encoder::le32(a_gold - b_gold))).await;
                        } else if b_gold > a_gold {
                            hub.send_to(partner_id, &format!("F44406001A02{}", encoder::le32(b_gold - a_gold))).await;
                        }
                        hub.send_to(
                            partner_id,
                            &crate::server::handlers::shops::gold_frame(partner.gold),
                        )
                        .await;

                        for item in &b_offers {
                            hub.send_to(partner_id, &format!("F44404001709{:02X}{:02X}", item.slot, item.count)).await;
                        }
                        for item in &a_offers {
                            hub.send_to(
                                partner_id,
                                &crate::server::inventory::item_added_frame(item),
                            )
                            .await;
                        }
                        hub.send_to(partner_id, "F4440300190204").await;
                        hub.send_to(partner_id, trade_cancelled_frame()).await;
                    }
                } else {
                    ctx.conn.session.trade = TradeState::default();
                    partner.trade = TradeState::default();
                    registry_put(partner);
                    registry_update_if_online(&ctx.conn.session);
                    ctx.out.send("F4440300190207");
                    ctx.out.send(trade_cancelled_frame());
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(partner_id, "F4440300190207").await;
                        hub.send_to(partner_id, trade_cancelled_frame()).await;
                    }
                }
            }
        }
        crate::protocol::TRADE_SUB_PET_OFFER_CS => {
            if payload.len() < 5 || !ctx.conn.session.trade.active {
                return;
            }
            let stt = payload[4];
            if stt > 0 && pet_at(&ctx.conn.session, stt).is_none() {
                return;
            }
            let gold = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            if gold > ctx.conn.session.gold {
                return;
            }
            ctx.conn.session.trade.gold = gold;
            ctx.conn.session.trade.pets = if stt > 0 { vec![stt] } else { Vec::new() };
            registry_update_if_online(&ctx.conn.session);
            let partner_id = ctx.conn.session.trade.partner_id;
            if let Some(hub) = ctx.env.hub {
                let packet = pet_at(&ctx.conn.session, stt)
                    .map(|pet| pet_offer_frame(gold, &pet))
                    .unwrap_or_else(|| empty_pet_offer_frame(gold));
                hub.send_to(partner_id, &packet).await;
            }
        }
        crate::protocol::TRADE_SUB_PET_CONFIRM_CS => {
            let Some(action) = payload.first().copied() else {
                return;
            };
            let partner_id = ctx.conn.session.trade.partner_id;
            if partner_id == 0 {
                return;
            }
            // Two-Phase State Lock: Lock both accounts in stable ID order BEFORE reading partner session
            let _locks = crate::server::session::lock_player_operations([id, partner_id]).await;
            let Some(mut partner) = registry_get(partner_id) else {
                ctx.conn.session.trade = TradeState::default();
                registry_update_if_online(&ctx.conn.session);
                ctx.out.send(trade_cancelled_frame());
                if action == 1 {
                    ctx.out.send(trade_toast_frame(0x01));
                }
                return;
            };
            if action == 0 || action == 2 {
                ctx.conn.session.trade = TradeState::default();
                partner.trade = TradeState::default();
                registry_put(partner);
                registry_update_if_online(&ctx.conn.session);
                ctx.out.send("F4440300190B0F");
                ctx.out.send(trade_cancelled_frame());
                if let Some(hub) = ctx.env.hub {
                    hub.send_to(partner_id, "F4440300190B03").await;
                    hub.send_to(partner_id, trade_cancelled_frame()).await;
                }
                return;
            }
            let current_before = ctx.conn.session.clone();
            ctx.conn.session.trade.accepted = true;
            registry_update_if_online(&ctx.conn.session);
            if !partner.trade.accepted {
                registry_put(partner);
                return;
            }

            let partner_before = partner.clone();
            let current_incoming_id = partner_before
                .trade
                .pets
                .first()
                .and_then(|stt| pet_at(&partner_before, *stt))
                .map(|p| p.id);
            let partner_incoming_id = current_before
                .trade
                .pets
                .first()
                .and_then(|stt| pet_at(&current_before, *stt))
                .map(|p| p.id);
            let current_offered_stt = current_before.trade.pets.first().copied();
            let partner_offered_stt = partner_before.trade.pets.first().copied();
            match pet_trade_settle(&mut ctx.conn.session, &mut partner) {
                Ok(())
                    if persist::persist_sessions_transaction(
                        ctx.env.pool,
                        &[&ctx.conn.session, &partner],
                        &["pet"],
                    )
                    .await =>
                {
                    let partner_after = partner.clone();
                    registry_put(partner_after.clone());
                    registry_update_if_online(&ctx.conn.session);
                    ctx.out.send(crate::server::handlers::shops::gold_frame(
                        ctx.conn.session.gold,
                    ));
                    if let Some(stt) = current_offered_stt {
                        ctx.out.send(removed_pet_frame(ctx.conn.session.id, stt));
                    }
                    if let Some(pet) = current_incoming_id
                        .and_then(|id| ctx.conn.session.pets.iter().find(|p| p.id == id))
                    {
                        ctx.out.send(received_pet_frame(pet));
                        ctx.out.send(pet_presence_frame(ctx.conn.session.id, pet));
                    }
                    ctx.out.send("F4440300190B04");
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(
                            partner_id,
                            &crate::server::handlers::shops::gold_frame(partner_after.gold),
                        )
                        .await;
                        if let Some(stt) = partner_offered_stt {
                            hub.send_to(partner_id, &removed_pet_frame(partner_id, stt))
                                .await;
                        }
                        if let Some(pet) = partner_incoming_id
                            .and_then(|id| partner_after.pets.iter().find(|p| p.id == id))
                        {
                            hub.send_to(partner_id, &received_pet_frame(pet)).await;
                            hub.send_to(partner_id, &pet_presence_frame(partner_id, pet))
                                .await;
                        }
                        hub.send_to(partner_id, "F4440300190B04").await;
                    }
                }
                result => {
                    ctx.conn.session = current_before;
                    partner = partner_before;
                    let packet = if matches!(result, Err("full")) {
                        "F4440300190B0A"
                    } else {
                        "F4440300190B07"
                    };
                    ctx.conn.session.trade = TradeState::default();
                    partner.trade = TradeState::default();
                    registry_put(partner);
                    registry_update_if_online(&ctx.conn.session);
                    ctx.out.send(packet);
                    ctx.out.send(trade_cancelled_frame());
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(partner_id, packet).await;
                        hub.send_to(partner_id, trade_cancelled_frame()).await;
                    }
                }
            }
        }
        crate::protocol::TRADE_SUB_TRANSFER_ITEM => transfer_item(ctx).await,
        _ => {}
    }
}

async fn transfer_item(ctx: &mut OpcodeCtx<'_>) {
    if ctx.payload.len() < 8 {
        return;
    }
    let recipient = encoder::u32_le(
        ctx.payload[4],
        ctx.payload[5],
        ctx.payload[6],
        ctx.payload[7],
    );
    if recipient == 0 || recipient == ctx.conn.session.id {
        return;
    }
    let _locks = crate::server::session::lock_player_operations([ctx.conn.session.id, recipient]).await;
    let Some(mut target) = registry_get(recipient) else {
        return;
    };
    let sender_before = ctx.conn.session.clone();
    let mut moved: Vec<InventoryItem> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for pair in ctx.payload[8..].chunks_exact(2).take(9) {
        if pair[0] == 0 || pair[1] == 0 {
            continue;
        }
        if !seen.insert(pair[0]) {
            return;
        }
        let Some(pos) = ctx
            .conn
            .session
            .homdo
            .iter()
            .position(|i| i.slot == pair[0] && i.count >= pair[1])
        else {
            return;
        };
        let mut item = ctx.conn.session.homdo[pos].clone();
        item.count = pair[1];
        moved.push(item);
    }
    let mut probe = target.clone();
    for item in &moved {
        if !add_trade_item(&mut probe, item.clone()) {
            return;
        }
    }
    let moved_items = moved.clone();
    for item in moved.into_iter() {
        let Some(pos) = ctx
            .conn
            .session
            .homdo
            .iter()
            .position(|i| i.slot == item.slot)
        else {
            ctx.conn.session = sender_before;
            return;
        };
        if ctx.conn.session.homdo[pos].count == item.count {
            ctx.conn.session.homdo.remove(pos);
        } else {
            ctx.conn.session.homdo[pos].count -= item.count;
        }
    }
    target = probe;
    if !persist::persist_sessions_transaction(
        ctx.env.pool,
        &[&ctx.conn.session, &target],
        &["homdo"],
    )
    .await
    {
        ctx.conn.session = sender_before;
        return;
    }
    registry_put(target.clone());
    registry_update_if_online(&ctx.conn.session);
    if let Some(hub) = ctx.env.hub {
        for item in &moved_items {
            hub.send_to(recipient, &crate::server::inventory::item_added_frame(item))
                .await;
        }
    }
    ctx.out.send(ctx.conn.session.dump_homdo());
}

/// Op 0x1E and legacy op 0x17 sub 51/52 storage paths.
pub async fn handle_storage_transfer(ctx: &mut OpcodeCtx<'_>) {
    if ctx.sub == crate::protocol::STORAGE_SUB_CLEAR {
        ctx.conn.session.select_menu = 40;
        return;
    }
    if ctx.payload.is_empty() {
        return;
    }
    let (slots, requested_qty) = if (ctx.opcode == crate::protocol::OP_STORAGE
        || ctx.opcode == crate::protocol::OP_ITEM)
        && ctx.payload.len() == 2
    {
        (vec![ctx.payload[0]], Some(ctx.payload[1]))
    } else {
        (ctx.payload.to_vec(), None)
    };
    let old = ctx.conn.session.clone();
    let mut moved: Vec<(u8, InventoryItem)> = Vec::new();
    match (ctx.opcode, ctx.sub) {
        (crate::protocol::OP_STORAGE, crate::protocol::STORAGE_SUB_WITHDRAW) => {
            for slot in slots {
                if let Some(pos) = ctx
                    .conn
                    .session
                    .tientrang
                    .iter()
                    .position(|i| i.slot == slot && i.id > 0)
                {
                    let count_to_move = match requested_qty {
                        Some(qty) if qty > 0 && qty < ctx.conn.session.tientrang[pos].count => qty,
                        _ => ctx.conn.session.tientrang[pos].count,
                    };
                    let mut item = ctx.conn.session.tientrang[pos].clone();
                    item.count = count_to_move;
                    let mut probe = ctx.conn.session.homdo.clone();
                    let Some(added) = add_storage_to_homdo(&mut probe, item) else {
                        continue;
                    };
                    ctx.conn.session.homdo = probe;
                    if count_to_move == ctx.conn.session.tientrang[pos].count {
                        ctx.conn.session.tientrang.remove(pos);
                    } else {
                        ctx.conn.session.tientrang[pos].count -= count_to_move;
                    }
                    moved.push((slot, added));
                }
            }
        }
        (crate::protocol::OP_STORAGE, crate::protocol::STORAGE_SUB_DEPOSIT) => {
            for slot in slots {
                if let Some(pos) = ctx
                    .conn
                    .session
                    .homdo
                    .iter()
                    .position(|i| i.slot == slot && i.id > 0)
                {
                    let count_to_move = match requested_qty {
                        Some(qty) if qty > 0 && qty < ctx.conn.session.homdo[pos].count => qty,
                        _ => ctx.conn.session.homdo[pos].count,
                    };
                    let mut item = ctx.conn.session.homdo[pos].clone();
                    item.count = count_to_move;
                    let mut probe = ctx.conn.session.tientrang.clone();
                    if !add_storage_item(
                        &mut probe,
                        item,
                        crate::db::modern::model::StorageType::Bank.capacity(),
                    ) {
                        continue;
                    }
                    let added = probe.last().cloned().unwrap();
                    ctx.conn.session.tientrang = probe;
                    if count_to_move == ctx.conn.session.homdo[pos].count {
                        ctx.conn.session.homdo.remove(pos);
                    } else {
                        ctx.conn.session.homdo[pos].count -= count_to_move;
                    }
                    moved.push((slot, added));
                }
            }
        }
        (crate::protocol::OP_ITEM, crate::protocol::ITEM_SUB_WAREHOUSE_DEPOSIT) => {
            if !ctx
                .conn
                .session
                .pets
                .iter()
                .any(|p| p.id == 41187 || p.id == 18023)
            {
                return;
            }
            for slot in slots {
                if let Some(pos) = ctx
                    .conn
                    .session
                    .homdo
                    .iter()
                    .position(|i| i.slot == slot && i.id > 0)
                {
                    let count_to_move = match requested_qty {
                        Some(qty) if qty > 0 && qty < ctx.conn.session.homdo[pos].count => qty,
                        _ => ctx.conn.session.homdo[pos].count,
                    };
                    let mut item = ctx.conn.session.homdo[pos].clone();
                    item.count = count_to_move;
                    let mut probe = ctx.conn.session.luulang.clone();
                    if !add_storage_item(
                        &mut probe,
                        item,
                        crate::db::modern::model::StorageType::Warehouse.capacity(),
                    ) {
                        continue;
                    }
                    let added = probe.last().cloned().unwrap();
                    ctx.conn.session.luulang = probe;
                    if count_to_move == ctx.conn.session.homdo[pos].count {
                        ctx.conn.session.homdo.remove(pos);
                    } else {
                        ctx.conn.session.homdo[pos].count -= count_to_move;
                    }
                    moved.push((slot, added));
                }
            }
        }
        (crate::protocol::OP_ITEM, crate::protocol::ITEM_SUB_WAREHOUSE_WITHDRAW) => {
            if !ctx
                .conn
                .session
                .pets
                .iter()
                .any(|p| p.id == 41187 || p.id == 18023)
            {
                return;
            }
            for slot in slots {
                if let Some(pos) = ctx
                    .conn
                    .session
                    .luulang
                    .iter()
                    .position(|i| i.slot == slot && i.id > 0)
                {
                    let count_to_move = match requested_qty {
                        Some(qty) if qty > 0 && qty < ctx.conn.session.luulang[pos].count => qty,
                        _ => ctx.conn.session.luulang[pos].count,
                    };
                    let mut item = ctx.conn.session.luulang[pos].clone();
                    item.count = count_to_move;
                    let mut probe = ctx.conn.session.homdo.clone();
                    let Some(added) = add_storage_to_homdo(&mut probe, item) else {
                        continue;
                    };
                    ctx.conn.session.homdo = probe;
                    if count_to_move == ctx.conn.session.luulang[pos].count {
                        ctx.conn.session.luulang.remove(pos);
                    } else {
                        ctx.conn.session.luulang[pos].count -= count_to_move;
                    }
                    moved.push((slot, added));
                }
            }
        }
        _ => return,
    }
    if moved.is_empty() {
        return;
    }
    if !persist::persist_sessions_transaction(
        ctx.env.pool,
        &[&ctx.conn.session],
        // HomDo (storagetype = 1), TienTrang (storagetype = 2), and LuuLang (storagetype = 16)
        // are all persisted atomically via `session.containers()` in modern SQLite storage.
        &["homdo", "tientrang", "luulang"],
    )
    .await
    {
        ctx.conn.session = old;
        return;
    }
    match (ctx.opcode, ctx.sub) {
        (crate::protocol::OP_STORAGE, crate::protocol::STORAGE_SUB_WITHDRAW) => {
            for (slot, item) in moved {
                ctx.out
                    .send(format!("F4440E001708{}", storage_entry(&item)));
                ctx.out.send(format!("F44404001E05{:02X}{:02X}", slot, item.count));
            }
            ctx.out.send("F44402001732");
        }
        (crate::protocol::OP_STORAGE, crate::protocol::STORAGE_SUB_DEPOSIT) => {
            for (slot, item) in moved {
                ctx.out.send(format!("F44404001709{:02X}{:02X}", slot, item.count));
                ctx.out.send(frame("1E02", &storage_entry_no_slot(&item)));
            }
        }
        (crate::protocol::OP_ITEM, crate::protocol::ITEM_SUB_WAREHOUSE_DEPOSIT) => {
            for (slot, item) in moved {
                ctx.out.send(format!("F44404001709{:02X}{:02X}", slot, item.count));
                ctx.out.send(frame("1767", &storage_entry(&item)));
            }
        }
        (crate::protocol::OP_ITEM, crate::protocol::ITEM_SUB_WAREHOUSE_WITHDRAW) => {
            for (slot, item) in moved {
                ctx.out
                    .send(format!("F4440E001708{}", storage_entry(&item)));
                ctx.out.send(format!("F44403001768{:02X}", slot));
            }
            ctx.out.send("F44402001732");
        }
        _ => unreachable!(),
    }
}

pub async fn handle_bank_gold(ctx: &mut OpcodeCtx<'_>) {
    if ctx.payload.len() < 4 {
        return;
    }
    let amount = encoder::u32_le(
        ctx.payload[0],
        ctx.payload[1],
        ctx.payload[2],
        ctx.payload[3],
    );
    if amount == 0 {
        ctx.out.send("F44402001D03");
        if ctx.sub == crate::protocol::BANK_SUB_WITHDRAW {
            ctx.out.send("F44402001D08");
        } else {
            ctx.out.send("F44402001D07");
        }
        return;
    }
    let before = ctx.conn.session.clone();
    let valid = match ctx.sub {
        crate::protocol::BANK_SUB_WITHDRAW => {
            ctx.conn.session.bank_gold >= amount
                && ctx.conn.session.gold.saturating_add(amount) <= GOLD_CAP
        }
        crate::protocol::BANK_SUB_DEPOSIT => {
            ctx.conn.session.gold >= amount
                && ctx.conn.session.bank_gold.saturating_add(amount) <= GOLD_CAP
        }
        _ => false,
    };
    if !valid {
        ctx.out.send("F44402001D03");
        if ctx.sub == crate::protocol::BANK_SUB_WITHDRAW {
            ctx.out.send("F44402001D08");
            if ctx.conn.session.gold >= GOLD_CAP || ctx.conn.session.gold.saturating_add(amount) > GOLD_CAP {
                ctx.out.send("F44402001A06");
            }
        } else {
            ctx.out.send("F44402001D07");
        }
        return;
    }
    if ctx.sub == 1 {
        ctx.conn.session.bank_gold -= amount;
        ctx.conn.session.gold += amount;
    } else {
        ctx.conn.session.gold -= amount;
        ctx.conn.session.bank_gold += amount;
    }
    if !persist::persist_sessions_transaction(ctx.env.pool, &[&ctx.conn.session], &[]).await {
        ctx.conn.session = before;
        ctx.out.send("F44402001D03");
        if ctx.sub == 1 {
            ctx.out.send("F44402001D08");
        } else {
            ctx.out.send("F44402001D07");
        }
        return;
    }
    let code = if ctx.sub == 1 { "02" } else { "01" };
    let gold_code = if ctx.sub == 1 { "01" } else { "02" };
    ctx.out
        .send(format!("F44406001D{}{}", code, encoder::le32(amount)));
    ctx.out
        .send(format!("F44406001A{}{}", gold_code, encoder::le32(amount)));
    if ctx.sub == 1 {
        ctx.out.send("F44402001D08");
    } else {
        ctx.out.send("F44402001D07");
    }
}
