//! System & Role handlers: PK/War (0x21), Game points (0x22), Rank (0x41), GM Shop (0x42), Teleport confirm (0x0C), Account Mgmt (0x23).

use crate::db;
use crate::protocol::encoder;
use crate::server::dispatcher::OpcodeCtx;
use crate::server::session::InventoryItem;
use crate::server::spawn::{store_frame, sys_msg_frame};

/// Op 0x42 point frame: `F44406004202`+le16(points)+`0100`. Width is the spec-
/// normalized LE16 (a legacy build emitted LE32 with a malformed 4-byte length
/// header — final parity needs a real capture).
fn shop_points_frame(points: u32) -> String {
    let mut body = String::new();
    body.push_str(&encoder::le16(points as u16));
    body.push_str("0100");
    crate::protocol::frame("4202", &body)
}

/// Handle Opcode 0x21 (OP_PK_SWITCH) — PK/Jam switch (§2.3.22).
///
/// Only flags `0` and `1` are accepted; anything else is silently ignored.
/// The new value is persisted to `players.Pk` / `players.ThamChien` scoped by
/// `player_id`.
pub async fn handle_pk_war(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    if payload.is_empty() {
        return;
    }
    let flag = payload[0];
    if flag > 1 {
        return; // only 0/1 accepted — reject anything else silently.
    }

    match sub {
        1 => {
            conn.session.pk = flag;
            out.send(format!(
                "F44404002102{:02X}{:02X}",
                flag, conn.session.tham_chien
            ));
        }
        2 => {
            conn.session.tham_chien = flag;
            out.send(format!("F44404002102{:02X}{:02X}", conn.session.pk, flag));
        }
        _ => {}
    }
}

/// Handle Opcode 0x22 — Game points / God panel (§2.3.23).
///
/// Only sub 1: `F44412002304`+`le32(gold)`+12 zero bytes (the earlier
/// `le16 + 24 zero` width was wrong).
pub fn handle_game_points(ctx: &mut OpcodeCtx) {
    if ctx.sub != 1 {
        return;
    }
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    out.send(store_frame(conn.session.gold));
}

/// Handle Opcode 0x41 — Rank system (§2.3.28).
pub fn handle_rank(ctx: &mut OpcodeCtx) {
    let out = &mut ctx.out;
    let sub = ctx.sub;
    match sub {
        1 => out.send("F44402004101"),
        2 => out.send("F44402004102"),
        _ => {}
    }
}

/// Handle Opcode 0x42 — GM / Mall shop (§2.3.29).
///
/// Sub 1 reads the item and price from raw packet bytes 9..10 / 11..12; since
/// `OpcodeCtx.payload` starts at raw byte 6 that is `payload[3..5]` and
/// `payload[5..7]` (the previous `[2..3]`/`[4..5]` decode mis-parses both
/// fields). The item is materialised from the static template, `homdo` +
/// `ShopPoint` are persisted in one InnoDB transaction, and the item-add frame
/// (`1706`) is emitted before the points frame.
pub async fn handle_gm_shop(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    if !conn.session.authed
        || !conn.session.logined
        || conn.session.gm_level <= crate::server::gm::PLAYER_GM_LEVEL
    {
        out.send(sys_msg_frame("Ban khong co quyen GM."));
        if let Some(pool) = ctx.env.pool {
            let repo = crate::db::modern::sqlite::accounts::SqliteAccountRepository { pool };
            let _ = repo
                .write_gm_audit(
                    i64::from(conn.session.id),
                    conn.session.gm_level,
                    None,
                    "deny_opcode_42",
                    "normal_account_attempted_gm_shop",
                )
                .await;
        }
        return;
    }
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Buy item from GM shop
        1 => {
            if payload.len() < 7 {
                return;
            }
            let item_id = encoder::u16_le(payload[3], payload[4]);
            let price = u32::from(encoder::u16_le(payload[5], payload[6]));

            if conn.session.shop_point < price {
                return; // under-funded buys are silently ignored
            }
            let Some(template) = ctx.data.items.get(&i64::from(item_id)) else {
                return; // Unknown item id -> no grant
            };
            let item = InventoryItem::from_template(template, 1);

            if !crate::server::inventory::can_add_item(&conn.session.homdo, &item) {
                out.send("F44403001B0102"); // Inventory full
                return;
            }

            // Emit the item-add frame `F4440E001706`+id+count+9 zero bytes
            // *before* deducting the points.
            let mut add_body = String::new();
            add_body.push_str(&encoder::le16(item.id));
            add_body.push_str(&format!("{:02X}", 1));
            add_body.push_str(&"00".repeat(9));
            out.send(crate::protocol::frame("1706", &add_body));

            // Grant + deduct in memory, then persist both atomically.
            let new_points = conn.session.shop_point.saturating_sub(price);
            conn.session.add_homdo_item(item);
            conn.session.shop_point = new_points;
            let granted = conn
                .session
                .homdo
                .iter()
                .find(|i| i.id == item_id && i.count > 0)
                .cloned()
                .unwrap_or(InventoryItem {
                    id: item_id,
                    count: 1,
                    doben: 100,
                    loai: 1,
                    ..Default::default()
                });
            db::persist::persist_shop_point_and_item(
                ctx.env.pool,
                conn.session.id,
                new_points,
                &granted,
            )
            .await;
            out.send(shop_points_frame(conn.session.shop_point));
        }
        // Sub 2: no-op (§2.3.29).
        2 => {}
        // Sub 3: Query GM shop points
        3 => {
            out.send(shop_points_frame(conn.session.shop_point));
        }
        _ => {}
    }
}

/// Handle Opcode 0x0C — Teleport confirm (§2.3.9).
///
/// Sub 1 only: Relocate Loaded / Teleport Confirm (Opcode 0x0C Sub 1).
/// Sent by client after completing the map loading phase.
pub fn handle_teleport_confirm(ctx: &mut OpcodeCtx) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    if ctx.sub != 1 {
        return;
    }
    // 1. World-Ready ([05 04]) & Clear Walk Lock ([14 08])
    out.send("F44402000504F44402001408");

    conn.session.warp_finish = false;
    conn.session.talk_count = 0;
    conn.session.idtalking = 0;

    let my_id = conn.session.id;
    let map_id = conn.session.map_id;

    // 2. Synchronize current player coords in the shared online registry
    if let Some(s) = crate::server::session::online_sessions()
        .lock()
        .unwrap()
        .get_mut(&my_id)
    {
        s.map_id = map_id;
        s.map_x = conn.session.map_x;
        s.map_y = conn.session.map_y;
        s.gocnhin = conn.session.gocnhin;
    }

    // 3. Broadcast self to other players on the new map
    let color = if conn.session.color.is_empty() {
        "0000000000000000"
    } else {
        &conn.session.color
    };
    let my_appear = crate::server::spawn::player_appear(
        conn.session.id,
        conn.session.sex,
        0,
        0,
        conn.session.map_id,
        conn.session.map_x,
        conn.session.map_y,
        conn.session.gocnhin,
        conn.session.hair,
        color,
        &conn.session.equipped_ids(),
        conn.session.reborn,
        conn.session.job,
        &conn.session.name,
    );
    out.broadcast(my_id, my_appear);

    // 4. Synchronize existing in-world players on this map to this client
    let others: Vec<crate::server::session::Session> = {
        let sessions = crate::server::session::online_sessions().lock().unwrap();
        sessions
            .values()
            .filter(|s| s.id != my_id && s.map_id == map_id && s.in_world)
            .cloned()
            .collect()
    };
    for other in others {
        let o_color = if other.color.is_empty() {
            "0000000000000000"
        } else {
            &other.color
        };
        let o_appear = crate::server::spawn::player_appear(
            other.id,
            other.sex,
            0,
            0,
            other.map_id,
            other.map_x,
            other.map_y,
            other.gocnhin,
            other.hair,
            o_color,
            &other.equipped_ids(),
            other.reborn,
            other.job,
            &other.name,
        );
        out.send(o_appear);
    }

    // 5. Synchronize NPCs on this map to this client
    for npc in &ctx.data.npc_on_map {
        if npc.map_id == i64::from(map_id) {
            let npc_frame = format!("F44405001601{:02X}0000", (npc.id & 0xFF) as u8);
            out.send(npc_frame);
        }
    }

    // 6. Synchronize map item drops on this map to this client
    for drop in crate::server::map_drops::drops_on_map(map_id) {
        if drop.item.id > 0 {
            out.send(format!(
                "F44409001703{}{}{}01",
                encoder::le16(drop.item.id),
                encoder::le16(drop.map_x),
                encoder::le16(drop.map_y),
            ));
        }
    }
}

/// Handle Opcode 0x23 — Account Management (change pass, delete char, gift code).
///
/// Wire order for sub 1 is `oldPass1, newPass1, oldPass2, newPass2`.
/// All validation runs against the MySQL `accounts` and
/// `players` tables; `player_id` scoping is enforced in every repository call.
pub async fn handle_account_mgmt(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let (sub, payload) = (ctx.sub, ctx.payload);
    match sub {
        // Sub 1: Change password
        1 => {
            let Some(parts) = parse_len_strings(payload, 4) else {
                out.shutdown = true;
                return;
            };
            let (old1, new1, old2, new2) = (parts[0], parts[1], parts[2], parts[3]);
            let Some(pool) = ctx.env.pool else {
                out.shutdown = true; // MySQL is mandatory (§5.7 fail fast)
                return;
            };
            let id = i64::from(conn.session.id);
            match db::accounts::passwords(pool, id).await {
                Ok(Some((db1, db2))) => {
                    // Byte-exact old-credential check (ASCII policy keeps this
                    // equivalent to the login HEX compare; no lossy decode).
                    if old1 != db1.as_bytes() {
                        out.send("F4440300230102");
                    } else if old2 != db2.as_bytes() {
                        out.send("F4440300230103");
                    } else if !db::accounts::is_valid_password(new1)
                        || !db::accounts::is_valid_password(new2)
                    {
                        // New password violates the [8,10] printable-ASCII policy.
                        out.send("F4440300230103");
                    } else {
                        let new1_str = String::from_utf8_lossy(new1).into_owned();
                        let new2_str = String::from_utf8_lossy(new2).into_owned();
                        match db::accounts::change_pass(pool, id, &new1_str, &new2_str).await {
                            Ok(true) => out.send("F4440300230101"),
                            _ => out.send("F4440300230103"),
                        }
                    }
                }
                _ => out.shutdown = true,
            }
        }
        // Sub 2: Delete character
        2 => {
            let Some(parts) = parse_len_strings(payload, 2) else {
                out.shutdown = true;
                return;
            };
            let pass1 = parts[0];
            let pass2 = parts[1];
            let Some(pool) = ctx.env.pool else {
                out.shutdown = true; // MySQL is mandatory
                return;
            };
            let id = i64::from(conn.session.id);
            match db::accounts::passwords(pool, id).await {
                Ok(Some((db1, db2))) => {
                    let p1 = String::from_utf8_lossy(pass1).into_owned();
                    let p2 = String::from_utf8_lossy(pass2).into_owned();
                    if p1 != db1 {
                        out.send("F4440300230202");
                    } else if p2 != db2 {
                        out.send("F4440300230203");
                    } else {
                        delete_character_flow(ctx).await;
                    }
                }
                _ => out.shutdown = true,
            }
        }
        // Sub 3: Redeem item_code / gift code (Ch5 §5.5).
        // Payload = `codeLen code passLen password` (two len-prefixed strings).
        3 => {
            let Some((code, password)) = parse_gift_code(payload) else {
                out.send(sys_msg_frame("Ma qua tang sai lieu qua tang khong hop le!"));
                return;
            };
            let code_str = String::from_utf8_lossy(code).into_owned();
            let pass_str = String::from_utf8_lossy(password).into_owned();
            let Some(pool) = ctx.env.pool else {
                return; // No DB (golden replay) — no-op, never a fake "invalid".
            };

            let id = i64::from(conn.session.id);

            // The once-only TSVN123/TSVN456 gift (§5.5).
            if code_str == "TSVN123" && pass_str == "TSVN456" {
                if conn.session.newbie == 1 {
                    out.send(sys_msg_frame(
                        "Ban da nhan qua nay truoc do roi. Khong the nhan lai!",
                    ));
                    return;
                }
                let ids = [46197i64, 20711, 19711, 23549, 11001];
                let mut items = Vec::with_capacity(5);
                let mut ok = true;
                for iid in ids.iter() {
                    match ctx.data.items.get(iid) {
                        Some(t) => items.push(InventoryItem::from_template(t, 1)),
                        None => ok = false,
                    }
                }
                if !ok {
                    out.send(sys_msg_frame("Loi he thong khi nhan qua tang!"));
                    return;
                }
                match db::item_code::redeem_special_gift(pool, id, &items).await {
                    Ok(db::item_code::RedeemOutcome::Granted { .. }) => {
                        for it in items.into_iter() {
                            conn.session.add_homdo_item(it);
                        }
                        conn.session.newbie = 1;
                        out.send(sys_msg_frame("Ban da nhan qua tang thanh cong!"));
                        out.send(conn.session.dump_homdo());
                    }
                    Ok(db::item_code::RedeemOutcome::AlreadyGifted) => {
                        out.send(sys_msg_frame(
                            "Ban da nhan qua nay truoc do roi. Khong the nhan lai!",
                        ));
                    }
                    _ => {
                        out.send(sys_msg_frame("Loi he thong khi nhan qua!"));
                    }
                }
                return;
            }

            // Ordinary code: validate the reward item and bag capacity first,
            // then redeem + grant atomically in one transaction.
            let reward = db::item_code::reward_for(pool, &code_str, &pass_str).await;
            let Some((item_id, rcount)) = reward.ok().flatten() else {
                out.send(sys_msg_frame(
                    "Ma qua tang khong hop le hoac da duoc su dung!",
                ));
                return;
            };
            let count = rcount.clamp(1, 255) as u8;
            let Some(template) = ctx.data.items.get(&item_id) else {
                out.send(sys_msg_frame("Ma qua tang khong hop le!"));
                return;
            };
            let item = InventoryItem::from_template(template, count);

            let free_slot = crate::server::inventory::free_slot(&conn.session.homdo).unwrap_or(1);
            if !crate::server::inventory::can_add_item(&conn.session.homdo, &item) {
                out.send("F44403001B0102"); // Inventory full
                return;
            }

            let item_for_db = item.clone();
            match db::item_code::redeem_and_grant(
                pool,
                id,
                &code_str,
                &pass_str,
                i64::from(free_slot),
                &item_for_db,
            )
            .await
            {
                Ok(db::item_code::RedeemOutcome::Granted { .. }) => {
                    conn.session.add_homdo_item(item);
                    out.send(sys_msg_frame("Nhan ma qua tang thanh cong!"));
                    out.send(conn.session.dump_homdo());
                }
                Ok(db::item_code::RedeemOutcome::InvalidOrUsed) => {
                    out.send(sys_msg_frame(
                        "Ma qua tang khong hop le hoac da duoc su dung!",
                    ));
                }
                Ok(db::item_code::RedeemOutcome::AlreadyGifted) => {
                    out.send(sys_msg_frame("Ma qua tang da duoc su dung!"));
                }
                Err(_) => {
                    out.send(sys_msg_frame("Loi he thong khi nhan ma qua tang!"));
                }
            }
        }
        _ => {}
    }
}

/// Teardown a validated character deletion (op 0x23 sub 2).
///
/// Leaves any battle, flags the player offline, then deletes `players` + all
/// nine gameplay tables scoped by `player_id` in one transaction, removes the
/// client registry entry (hub + online-session snapshot) and closes the
/// connection. The `accounts` row is preserved — deleting a Character never
/// deletes an Account.
async fn delete_character_flow(ctx: &mut OpcodeCtx<'_>) {
    let id = ctx.conn.session.id;
    // Leave battle (clears battle_id/membership).
    ctx.service.leave_battle(&mut ctx.conn.session);
    // GiaiTanParty: drop the player's own membership, then scrub this id from
    // every other online session that still lists it as leader/member.
    ctx.conn.session.id_leader = 0;
    ctx.conn.session.id_mem = [0; 4];
    {
        let mut online = crate::server::session::online_sessions().lock().unwrap();
        for s in online.values_mut() {
            if s.id_leader == id {
                s.id_leader = 0;
            }
            for mem in s.id_mem.iter_mut() {
                if *mem == id {
                    *mem = 0;
                }
            }
        }
    }
    // Map-removal broadcast: clients on the character's map drop it.
    ctx.out
        .broadcast(id, crate::battle::packets::hide_from_map(id));
    // Delete the character data in one transaction.
    if let Some(repos) = ctx.env.repos {
        let character_id = ctx.conn.session.db_character_id;
        if character_id > 0 {
            let _ = repos.characters().delete(character_id).await;
        }
    }
    // Remove the registries and ask for the connection to close.
    crate::server::session::online_sessions()
        .lock()
        .unwrap()
        .remove(&id);
    if let Some(hub) = ctx.env.hub {
        hub.unregister_client(id).await;
    }
    ctx.conn.session.logined = false;
    ctx.out.shutdown = true;
}

/// Parse `n` length-prefixed byte strings from `payload` (`[len][bytes]*`).
///
/// Returns `None` whenever a length prefix overruns the remaining buffer — the
/// connection must then be dropped (the parse failure shuts the socket down).
pub fn parse_len_strings(payload: &[u8], n: usize) -> Option<Vec<&[u8]>> {
    let mut rest = payload;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if rest.is_empty() {
            return None;
        }
        let len = rest[0] as usize;
        rest = rest.get(1..)?;
        if len > rest.len() {
            return None;
        }
        out.push(&rest[..len]);
        rest = &rest[len..];
    }
    Some(out)
}

/// Parse the two len-prefixed `code`/`password` byte strings
/// (`[0] code_len, [1..] code, [..] pass_len, [..] password`).
fn parse_gift_code(payload: &[u8]) -> Option<(&[u8], &[u8])> {
    let parts = parse_len_strings(payload, 2)?;
    Some((parts[0], parts[1]))
}
