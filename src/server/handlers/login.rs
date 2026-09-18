//! Login & session handlers (Opcode 0x01, 0x03).
//!
//! Live-server path (env.pool present):
//! version gate → account exists → pass1 check → double-login guard → load the
//! player row + skills/hotkeys/inventory/pets → `Logined1`. Without a pool
//! (golden replay) the handlers run in-memory over the seeded session.
//! All SQL lives in the `db` repository layer, never inline here.

use crate::protocol::encoder;
use crate::protocol::{ID_PREFIX, MIN_VERSION};
use crate::server::dispatcher::{HandleOutcome, OpcodeCtx};
use crate::server::session::Conn;
use crate::server::spawn;
use crate::web::server_control::{ClientSender, ServerControl};

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

    // ctx.sub chứa lenPw do dispatcher bóc tách từ byte thứ 5 của frame.
    // Dùng lenPw để cắt đúng độ dài mật khẩu thực tế, loại bỏ byte đệm/rác (nếu có).
    let len_pw = ctx.sub as usize;
    let password = if len_pw > 0 && payload.len() >= 8 + len_pw {
        &payload[8..8 + len_pw]
    } else {
        &payload[8..]
    };
    conn.session.id = acc_id;
    conn.session.pending_pass = password.to_vec();
    // `authed` is set only once auth succeeds — never before the password /
    // account / double-login gates (a failed or partial login must not leave a
    // connection in an authenticated state).
    conn.session.authed = false;

    match ctx.env.pool {
        Some(_) => {
            if let Some(repos) = ctx.env.repos {
                if login_db(conn, out, repos, ctx.env.hub, ctx.env.sender, password)
                    .await
                    .is_err()
                {
                    out.shutdown = true; // Handler error -> disconnect
                }
            } else {
                out.shutdown = true;
            }
        }
        None => {
            // In-memory fallback (golden replay): seeded session drives Logined1.
            if password == b"WRONG" {
                out.send(spawn::LOGIN_WRONG_PASS);
            } else if conn.session.name.is_empty() && conn.session.pending_new_char_name.is_empty()
            {
                conn.session.authed = true;
                out.send(spawn::LOGIN_CREATE_CHAR);
            } else {
                conn.session.authed = true;
                conn.session.logined = true;
                if conn.session.name.is_empty() {
                    conn.session.name = conn.session.pending_new_char_name.clone();
                }
                let seq = spawn::build_logined_sequence_session(&conn.session);
                out.outgoing.extend(
                    seq.into_iter()
                        .map(crate::server::dispatcher::OutFrame::new),
                );
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
        Some(_) => {
            // Short-circuit: this session already passed verification at login
            // (`authed`), so a blind re-verify would double `touch_login`.
            // Probe the character row first: charless → create-char screen
            // with no extra DB write; present → register + Logined1.
            if let Some(repos) = ctx.env.repos {
                let id = i64::from(conn.session.id);
                match repos.sessions().load(id, &mut conn.session).await {
                    Ok(false) => {
                        out.send(spawn::ENTER_GAME_CREATE);
                        return;
                    }
                    Ok(true) => {
                        if let (Some(hub), Some(sender)) = (ctx.env.hub, ctx.env.sender) {
                            if !hub.login_register(conn.session.id, sender).await {
                                out.send(spawn::DOUBLE_LOGIN);
                                out.shutdown = true;
                                return;
                            }
                        }
                        conn.session.logined = true;
                        let seq = spawn::build_logined_sequence_session(&conn.session);
                        out.outgoing.extend(
                            seq.into_iter()
                                .map(crate::server::dispatcher::OutFrame::new),
                        );
                        return;
                    }
                    Err(_) => {
                        out.shutdown = true;
                        return;
                    }
                }
            }
            let pass = conn.session.pending_pass.clone();
            if let Some(repos) = ctx.env.repos {
                if login_db(conn, out, repos, ctx.env.hub, ctx.env.sender, &pass)
                    .await
                    .is_err()
                {
                    out.shutdown = true;
                }
            } else {
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
                out.outgoing.extend(
                    seq.into_iter()
                        .map(crate::server::dispatcher::OutFrame::new),
                );
            }
        }
    }
}

/// Login success path: account exists → pass1 matches → double-login guard →
/// load the player → `Logined1` (or the create-char screen).
async fn login_db(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    repos: &crate::db::modern::sqlite::SqliteRepositories,
    hub: Option<&ServerControl>,
    sender: Option<&ClientSender>,
    password: &[u8],
) -> Result<(), sqlx::Error> {
    let id = i64::from(conn.session.id);

    // Account existence and byte-exact authentication through modern accounts.
    if !repos.accounts().verify_pass1(id, password).await? {
        out.send(spawn::LOGIN_WRONG_PASS);
        return Ok(());
    }
    let Some(access) = repos.accounts().access(id).await? else {
        out.send(spawn::LOGIN_WRONG_PASS);
        return Ok(());
    };
    let now_ms = chrono::Utc::now().timestamp_millis();
    if crate::db::modern::sqlite::accounts::SqliteAccountRepository::is_suspended(&access, now_ms) {
        // No verified PC/aLogin suspended-account opcode exists in the supplied
        // corpus, so do not emit a mobile numeric response on the PC dialect.
        out.send(spawn::sys_msg_frame("Tai khoan dang bi tam khoa."));
        out.shutdown = true;
        return Ok(());
    }
    conn.session.account_name = access.account_name.as_bytes().to_vec();
    conn.session.gm_level = access.gm_level.clamp(0, 99);
    repos.accounts().touch_login(id, now_ms, &conn.peer_ip).await?;

    // Xác thực tài khoản thành công. Đặt cờ authed để khi người chơi tạo nhân vật
    // và gửi gói tin xác nhận vào game (Opcode 0x03 Sub 0x01) phiên kết nối được chấp nhận.
    conn.session.authed = true;

    // Player existence: an account with no character goes to the create-char
    // screen (and is NOT registered as online — the online registry only gains
    // an entry once a character exists).
    if !repos.sessions().load(id, &mut conn.session).await? {
        out.send(spawn::LOGIN_CREATE_CHAR);
        return Ok(());
    }

    // Double-login guard: the check+register is one atomic lock so concurrent
    // logins cannot race. The Bear `[00][19]` frame goes first so the client
    // shows the "logged in elsewhere" dialog instead of hanging.
    if let (Some(hub), Some(sender)) = (hub, sender) {
        if !hub.login_register(conn.session.id, sender).await {
            out.send(spawn::DOUBLE_LOGIN);
            out.shutdown = true; // Already online elsewhere -> disconnect
            return Ok(());
        }
    }

    conn.session.logined = true;
    let seq = spawn::build_logined_sequence_session(&conn.session);
    out.outgoing.extend(
        seq.into_iter()
            .map(crate::server::dispatcher::OutFrame::new),
    );
    // The legacy login tail purged the basic `Skill` rows (Id 0..9); the
    // shared schema requires the `player_id` predicate (§5.4 note 2). This
    // would run after the stats frame so the skill list still matches the
    // pre-purge output. Currently disabled.
    // db::persist::delete_system_skills(Some(pool), conn.session.id).await;
    Ok(())
}
