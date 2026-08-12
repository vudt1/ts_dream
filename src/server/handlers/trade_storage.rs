//! Cross-player trade, storage transfer, and bank operations.

use crate::db::persist;
use crate::protocol::{encoder, frame};
use crate::server::dispatcher::OpcodeCtx;
use crate::server::inventory;
use crate::server::session::{online_sessions, InventoryItem, PetState, Session, TradeState};

const GOLD_CAP: u32 = 9_999_999;

fn item_wire(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}00{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.long_val,
        item.giatri_long.wrapping_add(100),
        item.khang,
        encoder::le32(item.texp)
    )
}

fn normal_trade_add_wire(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}0000{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.long_val,
        item.giatri_long.wrapping_add(100),
        item.khang,
        encoder::le32(item.texp)
    )
}

fn transferred_item_wire(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}00{:02X}{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.doben,
        item.long_val,
        item.giatri_long.wrapping_add(100),
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
        item.giatri_long.wrapping_add(100),
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

fn gold_status_frame(gold: u32) -> String {
    format!("F4440A001A04{}00000000", encoder::le32(gold))
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

fn remove_offers(s: &mut Session) -> bool {
    for offered in &s.trade.items {
        if !s
            .homdo
            .iter()
            .any(|i| i.slot == offered.slot && i.id == offered.id && i.count >= offered.count)
        {
            return false;
        }
    }
    for offered in s.trade.items.clone() {
        if let Some(pos) = s.homdo.iter().position(|i| i.slot == offered.slot) {
            if s.homdo[pos].count == offered.count {
                s.homdo.remove(pos);
            } else {
                s.homdo[pos].count -= offered.count;
            }
        }
    }
    true
}

fn add_trade_item(s: &mut Session, mut item: InventoryItem) -> bool {
    if (1..=6).contains(&item.loai) {
        let Some(slot) = inventory::free_slot(&s.homdo) else {
            return false;
        };
        item.slot = slot;
        s.homdo.push(item);
        true
    } else {
        inventory::can_add_item(&s.homdo, &item)
            && !inventory::add_item(&mut s.homdo, item).is_empty()
    }
}

fn add_storage_item(storage: &mut Vec<InventoryItem>, mut item: InventoryItem) -> bool {
    let Some(slot) = (1..=50).find(|slot| !storage.iter().any(|i| i.slot == *slot && i.id > 0))
    else {
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

fn settle_trade(a: &mut Session, b: &mut Session) -> bool {
    if a.trade.gold > a.gold || b.trade.gold > b.gold {
        return false;
    }
    let mut a_probe = a.clone();
    let mut b_probe = b.clone();
    if !remove_offers(&mut a_probe) || !remove_offers(&mut b_probe) {
        return false;
    }
    for item in a.trade.items.clone() {
        if !add_trade_item(&mut b_probe, item) {
            return false;
        }
    }
    for item in b.trade.items.clone() {
        if !add_trade_item(&mut a_probe, item) {
            return false;
        }
    }
    a_probe.gold = a_probe.gold - a.trade.gold + b.trade.gold;
    b_probe.gold = b_probe.gold - b.trade.gold + a.trade.gold;
    if a_probe.gold > GOLD_CAP || b_probe.gold > GOLD_CAP {
        return false;
    }
    a_probe.trade = TradeState::default();
    b_probe.trade = TradeState::default();
    *a = a_probe;
    *b = b_probe;
    true
}

fn pet_at(s: &Session, stt: u8) -> Option<PetState> {
    s.pets.iter().find(|p| p.stt == stt && p.id > 0).cloned()
}

fn pet_trade_settle(a: &mut Session, b: &mut Session) -> Result<(), &'static str> {
    if a.trade.gold > a.gold || b.trade.gold > b.gold {
        return Err("gold");
    }
    let a_stt = a.trade.pets.first().copied().unwrap_or(0);
    let b_stt = b.trade.pets.first().copied().unwrap_or(0);
    let ap = if a_stt > 0 { pet_at(a, a_stt) } else { None };
    let bp = if b_stt > 0 { pet_at(b, b_stt) } else { None };
    if ap
        .as_ref()
        .is_some_and(|p| b.pets.iter().any(|x| x.id == p.id))
        || bp
            .as_ref()
            .is_some_and(|p| a.pets.iter().any(|x| x.id == p.id))
    {
        return Err("duplicate");
    }
    if ap.is_some() && b.pets.len() >= 8 || bp.is_some() && a.pets.len() >= 8 {
        return Err("full");
    }
    if let Some(p) = ap {
        a.pets.retain(|x| x.stt != a_stt);
        let mut p = p;
        p.stt = crate::server::pet_box::next_active_slot(&b.pets)
            .or_else(|| crate::server::pet_box::next_stable_slot(&b.pets))
            .ok_or("full")?;
        b.pets.push(p);
    }
    if let Some(p) = bp {
        b.pets.retain(|x| x.stt != b_stt);
        let mut p = p;
        p.stt = crate::server::pet_box::next_active_slot(&a.pets)
            .or_else(|| crate::server::pet_box::next_stable_slot(&a.pets))
            .ok_or("full")?;
        a.pets.push(p);
    }
    a.gold = a.gold - a.trade.gold + b.trade.gold;
    b.gold = b.gold - b.trade.gold + a.trade.gold;
    if a.gold > GOLD_CAP || b.gold > GOLD_CAP {
        return Err("gold");
    }
    a.trade = TradeState::default();
    b.trade = TradeState::default();
    Ok(())
}

fn registry_get(id: u32) -> Option<Session> {
    online_sessions().lock().unwrap().get(&id).cloned()
}
fn registry_put(s: Session) {
    online_sessions().lock().unwrap().insert(s.id, s);
}

/// Op 0x19.
pub async fn handle_trade(ctx: &mut OpcodeCtx<'_>) {
    let id = ctx.conn.session.id;
    let sub = ctx.sub;
    let payload = ctx.payload;
    match sub {
        1 | 10 => {
            if payload.len() < 4 {
                return;
            }
            let partner_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            let Some(mut partner) = registry_get(partner_id) else {
                return;
            };
            if partner_id == id || partner.trade.active || ctx.conn.session.trade.active {
                return;
            }
            ctx.conn.session.trade = TradeState {
                active: true,
                partner_id,
                ..Default::default()
            };
            partner.trade = TradeState {
                active: true,
                partner_id: id,
                ..Default::default()
            };
            registry_put(partner);
            let code = if sub == 1 { "1901" } else { "190A" };
            ctx.out.send(frame(code, &encoder::le32(partner_id)));
            if let Some(hub) = ctx.env.hub {
                hub.send_to(partner_id, &frame(code, &encoder::le32(id)))
                    .await;
            }
        }
        2 => {
            if payload.len() < 4 || !ctx.conn.session.trade.active {
                return;
            }
            let gold = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
            if gold > ctx.conn.session.gold {
                return;
            }
            let mut items = Vec::new();
            for slot in &payload[4..] {
                let Some(item) = ctx
                    .conn
                    .session
                    .homdo
                    .iter()
                    .find(|i| i.slot == *slot && i.id > 0)
                    .cloned()
                else {
                    return;
                };
                if items.iter().any(|i: &InventoryItem| i.slot == item.slot) {
                    return;
                }
                items.push(item);
            }
            ctx.conn.session.trade.gold = gold;
            ctx.conn.session.trade.items = items;
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
        3 => {
            let Some(action) = payload.first().copied() else {
                return;
            };
            let partner_id = ctx.conn.session.trade.partner_id;
            let Some(mut partner) = registry_get(partner_id) else {
                return;
            };
            if action == 2 {
                ctx.conn.session.trade = TradeState::default();
                partner.trade = TradeState::default();
                registry_put(partner);
                ctx.out.send("F4440300190209");
                if let Some(hub) = ctx.env.hub {
                    hub.send_to(partner_id, "F4440300190203").await;
                }
            } else if action == 1 && ctx.conn.session.trade.active {
                let current_before = ctx.conn.session.clone();
                ctx.conn.session.trade.accepted = true;
                if !partner.trade.accepted {
                    registry_put(partner);
                    return;
                }
                let partner_before = partner.clone();
                let incoming = partner.trade.items.clone();
                let partner_incoming = ctx.conn.session.trade.items.clone();
                if settle_trade(&mut ctx.conn.session, &mut partner) {
                    if !persist::persist_sessions_transaction(
                        ctx.env.pool,
                        &[&ctx.conn.session, &partner],
                        &["homdo"],
                    )
                    .await
                    {
                        ctx.conn.session = current_before;
                        registry_put(partner_before);
                        return;
                    }
                    registry_put(partner.clone());
                    ctx.out.send(gold_status_frame(ctx.conn.session.gold));
                    for item in &incoming {
                        ctx.out.send(frame("1706", &normal_trade_add_wire(item)));
                    }
                    ctx.out.send("F4440300190204");
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(partner_id, &gold_status_frame(partner.gold))
                            .await;
                        for item in &partner_incoming {
                            hub.send_to(partner_id, &frame("1706", &normal_trade_add_wire(item)))
                                .await;
                        }
                        hub.send_to(partner_id, "F4440300190204").await;
                    }
                } else {
                    ctx.conn.session.trade = TradeState::default();
                    partner.trade = TradeState::default();
                    registry_put(partner);
                    ctx.out.send("F4440300190207");
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(partner_id, "F4440300190207").await;
                    }
                }
            }
        }
        11 => {
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
            let partner_id = ctx.conn.session.trade.partner_id;
            if let Some(hub) = ctx.env.hub {
                let packet = pet_at(&ctx.conn.session, stt)
                    .map(|pet| pet_offer_frame(gold, &pet))
                    .unwrap_or_else(|| empty_pet_offer_frame(gold));
                hub.send_to(partner_id, &packet).await;
            }
        }
        12 => {
            let Some(action) = payload.first().copied() else {
                return;
            };
            let partner_id = ctx.conn.session.trade.partner_id;
            let Some(mut partner) = registry_get(partner_id) else {
                return;
            };
            if action == 2 {
                ctx.conn.session.trade = TradeState::default();
                partner.trade = TradeState::default();
                registry_put(partner);
                ctx.out.send("F4440300190B0F");
                if let Some(hub) = ctx.env.hub {
                    hub.send_to(partner_id, "F4440300190B03").await;
                }
                return;
            }
            let current_before = ctx.conn.session.clone();
            ctx.conn.session.trade.accepted = true;
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
                    ctx.out.send(gold_status_frame(ctx.conn.session.gold));
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
                        hub.send_to(partner_id, &gold_status_frame(partner_after.gold))
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
                    ctx.out.send(packet);
                    if let Some(hub) = ctx.env.hub {
                        hub.send_to(partner_id, packet).await;
                    }
                }
            }
        }
        20 => transfer_item(ctx).await,
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
    if let Some(hub) = ctx.env.hub {
        for item in &moved_items {
            hub.send_to(recipient, &frame("1706", &transferred_item_wire(item)))
                .await;
        }
    }
    ctx.out.send(ctx.conn.session.dump_homdo());
}

/// Op 0x1E and legacy op 0x17 sub 51/52 storage paths.
pub async fn handle_storage_transfer(ctx: &mut OpcodeCtx<'_>) {
    if ctx.sub == 8 {
        ctx.conn.session.select_menu = 40;
        return;
    }
    let slots: Vec<u8> = ctx.payload.to_vec();
    if slots.is_empty() {
        return;
    }
    let old = ctx.conn.session.clone();
    let mut moved: Vec<(u8, InventoryItem)> = Vec::new();
    match (ctx.opcode, ctx.sub) {
        (0x1E, 1) => {
            for slot in slots {
                if let Some(pos) = ctx
                    .conn
                    .session
                    .tientrang
                    .iter()
                    .position(|i| i.slot == slot && i.id > 0)
                {
                    let item = ctx.conn.session.tientrang[pos].clone();
                    let mut probe = ctx.conn.session.homdo.clone();
                    let Some(added) = add_storage_to_homdo(&mut probe, item) else {
                        continue;
                    };
                    ctx.conn.session.homdo = probe;
                    ctx.conn.session.tientrang.remove(pos);
                    moved.push((slot, added));
                }
            }
        }
        (0x1E, 2) => {
            for slot in slots {
                if let Some(pos) = ctx
                    .conn
                    .session
                    .homdo
                    .iter()
                    .position(|i| i.slot == slot && i.id > 0)
                {
                    let item = ctx.conn.session.homdo[pos].clone();
                    let mut probe = ctx.conn.session.tientrang.clone();
                    if !add_storage_item(&mut probe, item) {
                        continue;
                    }
                    let added = probe.last().cloned().unwrap();
                    ctx.conn.session.tientrang = probe;
                    ctx.conn.session.homdo.remove(pos);
                    moved.push((slot, added));
                }
            }
        }
        (0x17, 51) => {
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
                    let item = ctx.conn.session.homdo[pos].clone();
                    let mut probe = ctx.conn.session.luulang.clone();
                    if !add_storage_item(&mut probe, item) {
                        continue;
                    }
                    let added = probe.last().cloned().unwrap();
                    ctx.conn.session.luulang = probe;
                    ctx.conn.session.homdo.remove(pos);
                    moved.push((slot, added));
                }
            }
        }
        (0x17, 52) => {
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
                    let item = ctx.conn.session.luulang[pos].clone();
                    let mut probe = ctx.conn.session.homdo.clone();
                    let Some(added) = add_storage_to_homdo(&mut probe, item) else {
                        continue;
                    };
                    ctx.conn.session.homdo = probe;
                    ctx.conn.session.luulang.remove(pos);
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
        &["homdo", "tientrang", "luulang"],
    )
    .await
    {
        ctx.conn.session = old;
        return;
    }
    match (ctx.opcode, ctx.sub) {
        (0x1E, 1) => {
            for (slot, item) in moved {
                ctx.out
                    .send(format!("F4440E001708{}", storage_entry(&item)));
                ctx.out.send(format!("F44404001E05{:02X}32", slot));
            }
            ctx.out.send("F44402001732");
        }
        (0x1E, 2) => {
            let mut entries = String::new();
            for (slot, item) in moved {
                ctx.out.send(format!("F44404001709{:02X}32", slot));
                entries.push_str(&storage_entry(&item));
            }
            ctx.out.send(frame("1E04", &entries));
        }
        (0x17, 51) => {
            let mut entries = String::new();
            for (slot, item) in moved {
                ctx.out.send(format!("F44404001709{:02X}32", slot));
                entries.push_str(&storage_entry(&item));
            }
            ctx.out.send(frame("1766", &entries));
        }
        (0x17, 52) => {
            for (slot, item) in moved {
                ctx.out
                    .send(format!("F4440E001708{}", storage_entry(&item)));
                ctx.out.send(format!("F44404001768{:02X}32", slot));
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
        return;
    }
    let before = ctx.conn.session.clone();
    let valid = match ctx.sub {
        1 => {
            ctx.conn.session.bank_gold >= amount
                && ctx.conn.session.gold.saturating_add(amount) <= GOLD_CAP
        }
        2 => {
            ctx.conn.session.gold >= amount
                && ctx.conn.session.bank_gold.saturating_add(amount) <= GOLD_CAP
        }
        _ => false,
    };
    if !valid {
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
        return;
    }
    let code = if ctx.sub == 1 { "02" } else { "01" };
    let gold_code = if ctx.sub == 1 { "01" } else { "02" };
    ctx.out
        .send(format!("F44406001D{}{}", code, encoder::le32(amount)));
    ctx.out
        .send(format!("F44406001A{}{}", gold_code, encoder::le32(amount)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::server::dispatcher::{test_ctx, HandleOutcome};
    use crate::server::session::Conn;
    use std::sync::Arc;

    fn item(slot: u8, id: u16, count: u8) -> InventoryItem {
        InventoryItem {
            slot,
            id,
            count,
            ..Default::default()
        }
    }

    fn dependencies() -> (GameData, BattleService) {
        (
            GameData::default(),
            BattleService::new(Arc::new(GameData::default())),
        )
    }

    #[tokio::test]
    async fn accepted_trade_exchanges_both_players_items_and_gold() {
        let (data, service) = dependencies();
        let mut conn = Conn::new();
        conn.session.id = 390_001;
        conn.session.gold = 1_000;
        conn.session.homdo = vec![item(1, 1001, 1)];
        conn.session.trade = TradeState {
            active: true,
            partner_id: 390_002,
            gold: 100,
            items: conn.session.homdo.clone(),
            ..Default::default()
        };
        let mut partner = Session::new();
        partner.id = 390_002;
        partner.gold = 2_000;
        partner.homdo = vec![item(1, 2002, 1)];
        partner.trade = TradeState {
            active: true,
            partner_id: 390_001,
            accepted: true,
            gold: 200,
            items: partner.homdo.clone(),
            ..Default::default()
        };
        registry_put(partner);
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 3, &[1]);
        handle_trade(&mut ctx).await;
        let partner = registry_get(390_002).unwrap();
        assert_eq!((conn.session.gold, partner.gold), (1_100, 1_900));
        assert_eq!(conn.session.homdo[0].id, 2002);
        assert_eq!(partner.homdo[0].id, 1001);
        assert!(out.outgoing[0].contains("1A04"));
        assert!(out.outgoing[1].contains("1706"));
        assert_eq!(out.outgoing[2], "F4440300190204");
        online_sessions().lock().unwrap().remove(&390_002);
    }

    #[tokio::test]
    async fn full_trade_destination_is_reported_without_losing_source_item() {
        let (data, service) = dependencies();
        let mut conn = Conn::new();
        conn.session.id = 390_011;
        conn.session.homdo = vec![item(1, 1001, 1)];
        conn.session.trade = TradeState {
            active: true,
            partner_id: 390_012,
            items: conn.session.homdo.clone(),
            ..Default::default()
        };
        let mut partner = Session::new();
        partner.id = 390_012;
        partner.homdo = (1..=25)
            .map(|slot| item(slot, 2000 + u16::from(slot), 1))
            .collect();
        partner.trade = TradeState {
            active: true,
            partner_id: 390_011,
            accepted: true,
            ..Default::default()
        };
        registry_put(partner);
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 3, &[1]);
        handle_trade(&mut ctx).await;
        assert_eq!(conn.session.homdo, vec![item(1, 1001, 1)]);
        assert_eq!(out.outgoing, vec!["F4440300190207"]);
        online_sessions().lock().unwrap().remove(&390_012);
    }

    #[tokio::test]
    async fn transfer_uses_requested_count_and_updates_recipient() {
        let (data, service) = dependencies();
        let mut conn = Conn::new();
        conn.session.id = 390_021;
        conn.session.homdo = vec![item(3, 3003, 5)];
        let mut recipient = Session::new();
        recipient.id = 390_022;
        registry_put(recipient);
        let mut payload = vec![0; 4];
        payload.extend_from_slice(&390_022u32.to_le_bytes());
        payload.extend_from_slice(&[3, 2]);
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 20, &payload);
        handle_trade(&mut ctx).await;
        assert_eq!(conn.session.homdo[0].count, 3);
        assert_eq!(registry_get(390_022).unwrap().homdo[0].count, 2);
        online_sessions().lock().unwrap().remove(&390_022);
    }

    #[tokio::test]
    async fn storage_and_luulang_validate_destination_and_access_pet() {
        let (data, service) = dependencies();
        let mut conn = Conn::new();
        conn.session.tientrang = vec![item(1, 4001, 1), item(2, 4002, 1)];
        conn.session.homdo = (1..=24)
            .map(|slot| item(slot, 5000 + u16::from(slot), 1))
            .collect();
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 1, &[1, 2]);
        ctx.opcode = 0x1E;
        handle_storage_transfer(&mut ctx).await;
        assert!(conn.session.homdo.iter().any(|i| i.id == 4001));
        assert!(conn.session.tientrang.iter().any(|i| i.id == 4002));
        conn.session.homdo = vec![item(1, 6001, 1)];
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 51, &[1]);
        ctx.opcode = 0x17;
        handle_storage_transfer(&mut ctx).await;
        assert!(conn.session.luulang.is_empty());
        conn.session.pets.push(PetState {
            stt: 1,
            id: 41187,
            ..Default::default()
        });
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 51, &[1]);
        ctx.opcode = 0x17;
        handle_storage_transfer(&mut ctx).await;
        assert_eq!(conn.session.luulang[0].id, 6001);
        let luulang_slot = conn.session.luulang[0].slot;
        let mut out = HandleOutcome::default();
        let luulang_payload = [luulang_slot];
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 52, &luulang_payload);
        ctx.opcode = 0x17;
        handle_storage_transfer(&mut ctx).await;
        assert!(conn.session.luulang.is_empty());
        assert_eq!(conn.session.homdo[0].id, 6001);
    }

    #[tokio::test]
    async fn accepted_pet_trade_moves_ownership_and_gold() {
        let (data, service) = dependencies();
        let mut conn = Conn::new();
        conn.session.id = 390_031;
        conn.session.gold = 500;
        conn.session.pets = vec![PetState {
            stt: 1,
            id: 7001,
            ..Default::default()
        }];
        conn.session.trade = TradeState {
            active: true,
            partner_id: 390_032,
            gold: 50,
            pets: vec![1],
            ..Default::default()
        };
        let mut partner = Session::new();
        partner.id = 390_032;
        partner.gold = 800;
        partner.pets = vec![PetState {
            stt: 1,
            id: 7002,
            ..Default::default()
        }];
        partner.trade = TradeState {
            active: true,
            partner_id: 390_031,
            accepted: true,
            gold: 100,
            pets: vec![1],
            ..Default::default()
        };
        registry_put(partner);
        let mut out = HandleOutcome::default();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 12, &[1]);
        handle_trade(&mut ctx).await;
        let partner = registry_get(390_032).unwrap();
        assert!(conn.session.pets.iter().any(|p| p.id == 7002));
        assert!(partner.pets.iter().any(|p| p.id == 7001));
        assert_eq!((conn.session.gold, partner.gold), (550, 750));
        assert!(out.outgoing.iter().any(|f| f.contains("1A04")));
        assert!(out.outgoing.iter().any(|f| f.contains("0F02")));
        assert!(out.outgoing.iter().any(|f| f.contains("0F07")));
        assert!(out.outgoing.iter().any(|f| f.contains("0F01")));
        assert_eq!(out.outgoing.last().unwrap(), "F4440300190B04");
        online_sessions().lock().unwrap().remove(&390_032);
    }

    #[tokio::test]
    async fn bank_parses_le32_and_enforces_bank_cap() {
        let (data, service) = dependencies();
        let mut conn = Conn::new();
        conn.session.gold = 100_000;
        conn.session.bank_gold = 9_900_000;
        let mut out = HandleOutcome::default();
        let rejected = 100_000u32.to_le_bytes();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &rejected);
        handle_bank_gold(&mut ctx).await;
        assert!(out.outgoing.is_empty());
        let mut out = HandleOutcome::default();
        let accepted = 99_999u32.to_le_bytes();
        let mut ctx = test_ctx(&mut conn, &data, &service, &mut out, 2, &accepted);
        handle_bank_gold(&mut ctx).await;
        assert_eq!(conn.session.bank_gold, 9_999_999);
        assert_eq!(
            out.outgoing,
            vec!["F44406001D019F860100", "F44406001A029F860100"]
        );
    }
}
