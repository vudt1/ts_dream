//! Login & session handlers (Opcode 0x00, 0x01, 0x03).
//!
//! Live-server path (env.pool present) mirrors C# `Update_H1`/`Update_H3`:
//! version gate → account exists → pass1 check → double-login guard → load the
//! player row + skills/hotkeys/inventory/pets → `Logined1`. Without a pool
//! (golden replay) the handlers run in-memory over the seeded session.
//! All SQL lives in the `db` repository layer, never inline here.

use crate::db;
use crate::protocol::encoder;
use crate::protocol::{ID_PREFIX, MIN_VERSION};
use crate::server::handler::{HandleOutcome, OpcodeCtx};
use crate::server::session::Conn;
use crate::server::spawn;
use crate::web::server_control::{ClientSender, ServerControl};
use sqlx::MySqlPool;

/// Op 0x00 — Hello: exact opcode 0x00 with length 1 and no sub byte.
pub fn handle_hello(ctx: &mut OpcodeCtx) {
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.is_empty() {
        out.send(spawn::HELLO_REPLY);
    }
}

/// Op 0x01 — Login (version check >= 186, auth & session initialization).
pub async fn handle_login(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let payload = ctx.payload;
    if payload.len() < 8 {
        return;
    }
    let acc_id = encoder::u32_le(payload[0], payload[1], payload[2], payload[3]);
    let prefix = &payload[4..6];
    if !prefix.eq_ignore_ascii_case(ID_PREFIX.as_bytes()) {
        return; // Prefix mismatch -> silent return
    }
    let version = encoder::u16_le(payload[6], payload[7]);
    if version < MIN_VERSION {
        out.shutdown = true; // Version gate < 186 -> disconnect
        return;
    }

    let password = &payload[8..];
    conn.session.id = acc_id;
    conn.session.pending_pass = password.to_vec();
    conn.session.authed = true;

    match ctx.env.pool {
        Some(pool) => {
            if login_db(conn, out, pool, ctx.env.hub, ctx.env.sender, password)
                .await
                .is_err()
            {
                out.shutdown = true; // C# exception -> disconnect
            }
        }
        None => {
            // In-memory fallback (golden replay): seeded session drives Logined1.
            if password == b"WRONG" {
                out.send(spawn::LOGIN_WRONG_PASS);
            } else if conn.session.name.is_empty() && conn.session.pending_new_char_name.is_empty()
            {
                out.send(spawn::LOGIN_CREATE_CHAR);
            } else {
                conn.session.logined = true;
                if conn.session.name.is_empty() {
                    conn.session.name = conn.session.pending_new_char_name.clone();
                }
                let seq = spawn::build_logined_sequence_session(&conn.session);
                out.outgoing.extend(seq);
            }
        }
    }
}

/// Op 0x03 — Enter game confirmation.
pub async fn handle_enter_game(ctx: &mut OpcodeCtx<'_>) {
    let conn = &mut ctx.conn;
    let out = &mut ctx.out;
    let sub = ctx.sub;
    if sub != 1 {
        return;
    }
    if !conn.session.authed {
        out.send(spawn::ENTER_GAME_CREATE); // Not authed -> create char screen
        return;
    }
    if conn.session.logined {
        return;
    }
    match ctx.env.pool {
        Some(pool) => {
            let pass = conn.session.pending_pass.clone();
            if login_db(conn, out, pool, ctx.env.hub, ctx.env.sender, &pass)
                .await
                .is_err()
            {
                out.shutdown = true;
            }
        }
        None => {
            if conn.session.name.is_empty() && conn.session.pending_new_char_name.is_empty() {
                out.send(spawn::ENTER_GAME_CREATE);
            } else {
                conn.session.logined = true;
                if conn.session.name.is_empty() {
                    conn.session.name = conn.session.pending_new_char_name.clone();
                }
                let seq = spawn::build_logined_sequence_session(&conn.session);
                out.outgoing.extend(seq);
            }
        }
    }
}

/// C# `Update_H1` success path: account exists → pass1 matches → double-login
/// guard → load the player → `Logined1` (or the create-char screen).
async fn login_db(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    pool: &MySqlPool,
    hub: Option<&ServerControl>,
    sender: Option<&ClientSender>,
    password: &[u8],
) -> Result<(), sqlx::Error> {
    let id = i64::from(conn.session.id);

    // Account existence (repository `accounts::pass1`).
    let Some(db_pass) = db::accounts::pass1(pool, id).await? else {
        out.shutdown = true; // Unknown account -> disconnect (spec §2.3.2)
        return Ok(());
    };
    if db_pass.as_bytes() != password {
        out.send(spawn::LOGIN_WRONG_PASS);
        return Ok(());
    }

    // Player existence: an account with no character goes to the create-char
    // screen (and is NOT registered as online — C# `Logined()` only adds the
    // client to `Server.Clients` once a character exists).
    if !db::players::load(pool, &mut conn.session).await? {
        out.send(spawn::LOGIN_CREATE_CHAR);
        return Ok(());
    }

    // Double-login guard (C# `Server.Clients.ContainsKey` + Add): the
    // check+register is one atomic lock so concurrent logins cannot race.
    if let (Some(hub), Some(sender)) = (hub, sender) {
        if !hub.login_register(conn.session.id, sender).await {
            out.shutdown = true; // Already online elsewhere -> disconnect
            return Ok(());
        }
    }

    conn.session.logined = true;
    conn.session.authed = true;
    let seq = spawn::build_logined_sequence_session(&conn.session);
    out.outgoing.extend(seq);
    Ok(())
}
