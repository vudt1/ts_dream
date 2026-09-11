//! Server-authoritative GM commands for the PC chat path.
//!
//! The mobile reference exposes a broad command surface, but this module only
//! enables commands whose state mutation can be persisted through the current
//! modern repositories or whose PC response is an already-used safe frame.
//! There is intentionally no self-promotion, packet-spam command, or implicit
//! dashboard bootstrap.

use crate::db::modern::sqlite::SqliteRepositories;
use crate::db::persist;
use crate::db::pool::DbPool;
use crate::protocol::encoder;
use crate::server::dispatcher::HandleOutcome;
use crate::server::session::{online_sessions, Conn, InventoryItem, Session};
use crate::server::spawn;
use crate::web::server_control::ServerControl;

pub const PLAYER_GM_LEVEL: i32 = 0;
pub const HELPER_GM_LEVEL: i32 = 1;
pub const GM_LEVEL: i32 = 10;
pub const ADMIN_GM_LEVEL: i32 = 50;
pub const OWNER_GM_LEVEL: i32 = 99;
const MAX_COMMAND_GOLD: u32 = 1_000_000_000;
const MAX_COMMAND_ITEM_COUNT: u8 = 99;

fn reply(out: &mut HandleOutcome, message: &str) {
    out.send(spawn::sys_msg_frame(message));
}

fn split(msg: &str) -> (String, Vec<String>) {
    let mut parts = msg.split_whitespace();
    (
        parts.next().unwrap_or_default().to_ascii_lowercase(),
        parts.map(ToOwned::to_owned).collect(),
    )
}

fn parse_u32(input: Option<&String>) -> Option<u32> {
    input?.parse::<u32>().ok()
}

fn parse_target(args: &[String], default_id: u32) -> Option<u32> {
    args.last()
        .and_then(|s| s.parse::<u32>().ok())
        .or(Some(default_id))
}

fn is_gm_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "/gm"
            | "/gmhelp"
            | "/additem"
            | "/addgold"
            | "/setgold"
            | "/level"
            | "/tp"
            | "/goto"
            | "/summon"
            | "/broadcast"
            | "/kick"
            | "/setperm"
            | "/setgm"
            | "/battle"
            | "/quest"
            | "/setflag"
            | "/test"
    )
}

fn help(out: &mut HandleOutcome) {
    reply(
        out,
        "GM: /where /info /additem id count [player] /addgold amount [player] /setgold amount [player] /level level [player] /tp map x y [player] /broadcast text /kick player /setperm player level",
    );
}

fn find_online(target: u32) -> Option<Session> {
    online_sessions().lock().ok()?.get(&target).cloned()
}

fn update_online(target: u32, session: Session) {
    if let Ok(mut sessions) = online_sessions().lock() {
        sessions.insert(target, session);
    }
}

fn emit_item_add(out: &mut HandleOutcome, item: &InventoryItem) {
    let mut body = String::new();
    body.push_str(&encoder::le16(item.id));
    body.push_str(&format!("{:02X}", item.count));
    body.push_str(&"00".repeat(9));
    out.send(crate::protocol::frame("1706", &body));
}

async fn audit(
    pool: Option<&DbPool>,
    actor: u32,
    actor_level: i32,
    target: Option<u32>,
    action: &str,
    details: &str,
) {
    let Some(pool) = pool else {
        tracing::warn!("GM audit skipped without DB: action={action} actor={actor}");
        return;
    };
    let repo = crate::db::modern::sqlite::accounts::SqliteAccountRepository { pool };
    if let Err(error) = repo
        .write_gm_audit(
            i64::from(actor),
            actor_level,
            target.map(i64::from),
            action,
            details,
        )
        .await
    {
        tracing::error!("GM audit write failed for {action}: {error}");
    }
}

async fn mutate_target<F>(
    conn: &mut Conn,
    hub: Option<&ServerControl>,
    target: u32,
    mutator: F,
) -> Option<Session>
where
    F: FnOnce(&mut Session),
{
    let mut session = if target == conn.session.id {
        conn.session.clone()
    } else {
        find_online(target)?
    };
    mutator(&mut session);
    if target == conn.session.id {
        conn.session.clone_from(&session);
    } else {
        update_online(target, session.clone());
        if let Some(hub) = hub {
            hub.send_to(
                target,
                &spawn::sys_msg_frame("GM da cap nhat tai khoan cua ban."),
            )
            .await;
        }
    }
    Some(session)
}

/// Returns true when `msg` was a recognized GM/privileged command. Normal
/// player commands remain in `chat.rs`; recognized privileged commands are
/// always denied explicitly when the session is not a GM.
pub async fn handle(
    conn: &mut Conn,
    out: &mut HandleOutcome,
    pool: Option<&DbPool>,
    repos: Option<&SqliteRepositories>,
    hub: Option<&ServerControl>,
    data: &crate::data::loader::GameData,
    msg: &str,
) -> bool {
    let (cmd, args) = split(msg);
    if !is_gm_command(&cmd) {
        return false;
    }
    if !conn.session.authed || !conn.session.logined || conn.session.gm_level <= PLAYER_GM_LEVEL {
        // Do not reveal privileged command availability to normal clients; the
        // command is consumed and performs no state mutation.
        return true;
    }
    let actor = conn.session.id;
    let actor_level = conn.session.gm_level;

    match cmd.as_str() {
        "/gm" | "/gmhelp" => help(out),
        "/where" => reply(
            out,
            &format!(
                "MapID:{} X:{} Y:{} GM:{}",
                conn.session.map_id, conn.session.map_x, conn.session.map_y, actor_level
            ),
        ),
        "/info" => {
            let target = parse_target(&args, actor).unwrap_or(actor);
            let Some(session) =
                find_online(target).or_else(|| (target == actor).then(|| conn.session.clone()))
            else {
                reply(out, "Nguoi choi khong online.");
                return true;
            };
            reply(
                out,
                &format!(
                    "Player:{} Lv:{} Gold:{} Map:{} ({},{}) GM:{}",
                    target,
                    session.level,
                    session.gold,
                    session.map_id,
                    session.map_x,
                    session.map_y,
                    session.gm_level
                ),
            );
        }
        "/additem" => {
            let Some(item_id) = parse_u32(args.first()).filter(|v| *v <= u32::from(u16::MAX))
            else {
                reply(out, "Su dung: /additem item_id count [player_id]");
                return true;
            };
            let count = parse_u32(args.get(1))
                .filter(|v| (1..=u32::from(MAX_COMMAND_ITEM_COUNT)).contains(v));
            let Some(count) = count else {
                reply(out, "So luong phai nam trong khoang 1..99.");
                return true;
            };
            let Some(def) = data.items.get(&(item_id as i64)) else {
                reply(out, "Item khong ton tai trong static catalog.");
                return true;
            };
            let target = parse_target(&args[2..], actor).unwrap_or(actor);
            let item = InventoryItem::from_template(def, count as u8);
            let Some(session) = mutate_target(conn, hub, target, |s| {
                if s.add_homdo_item(item.clone()).is_empty() {
                    return;
                }
                s.recompute_stats();
            })
            .await
            else {
                reply(out, "Nguoi choi khong online.");
                return true;
            };
            let Some(granted) = session
                .homdo
                .iter()
                .find(|i| i.id == item.id && i.count > 0)
                .cloned()
            else {
                reply(out, "Tui do day hoac khong the cap item.");
                return true;
            };
            persist::upsert_item(pool, target, "homdo", &granted).await;
            if target == actor {
                emit_item_add(out, &granted);
            }
            audit(
                pool,
                actor,
                actor_level,
                Some(target),
                "additem",
                &format!("item_id={item_id},count={count}"),
            )
            .await;
        }
        "/addgold" | "/setgold" => {
            let Some(amount) = parse_u32(args.first()).filter(|v| *v <= MAX_COMMAND_GOLD) else {
                reply(out, "So vang khong hop le hoac vuot gioi han lenh.");
                return true;
            };
            let target = parse_target(&args[1..], actor).unwrap_or(actor);
            let Some(session) = mutate_target(conn, hub, target, |s| {
                s.gold = if cmd == "/setgold" {
                    amount
                } else {
                    s.gold.saturating_add(amount)
                };
            })
            .await
            else {
                reply(out, "Nguoi choi khong online.");
                return true;
            };
            persist::update_player(pool, target, "Gold", i64::from(session.gold)).await;
            if target == actor {
                out.send(crate::server::spawn::store_frame(session.gold));
            }
            audit(
                pool,
                actor,
                actor_level,
                Some(target),
                &cmd[1..],
                &format!("amount={amount},result={}", session.gold),
            )
            .await;
        }
        "/level" => {
            let Some(level) = parse_u32(args.first()).filter(|v| (1..=200).contains(v)) else {
                reply(out, "Cap do phai nam trong khoang 1..200.");
                return true;
            };
            let target = parse_target(&args[1..], actor).unwrap_or(actor);
            let Some(session) = mutate_target(conn, hub, target, |s| {
                s.level = level as u8;
                s.recompute_stats();
            })
            .await
            else {
                reply(out, "Nguoi choi khong online.");
                return true;
            };
            persist::update_player(pool, target, "Lv", i64::from(level)).await;
            if target == actor {
                reply(
                    out,
                    &format!("Cap do hien tai: {level}. HP/SP da tinh lai."),
                );
            }
            audit(
                pool,
                actor,
                actor_level,
                Some(target),
                "level",
                &format!("level={level}"),
            )
            .await;
            let _ = session;
        }
        "/tp" | "/goto" | "/summon" => {
            let (map, x, y, target) = if cmd == "/summon" {
                let target = parse_u32(args.first());
                (
                    Some(conn.session.map_id as u32),
                    Some(conn.session.map_x as u32),
                    Some(conn.session.map_y as u32),
                    target,
                )
            } else {
                (
                    parse_u32(args.first()),
                    parse_u32(args.get(1)),
                    parse_u32(args.get(2)),
                    parse_u32(args.get(3)),
                )
            };
            let (Some(map), Some(x), Some(y)) = (map, x, y) else {
                reply(out, "Su dung: /tp map x y [player_id].");
                return true;
            };
            if map > u32::from(u16::MAX) || x > u32::from(u16::MAX) || y > u32::from(u16::MAX) {
                reply(out, "Toa do vuot gioi han PC.");
                return true;
            }
            let target = target.unwrap_or(actor);
            let Some(_) = mutate_target(conn, hub, target, |s| {
                s.map_id = map as u16;
                s.map_x = x as u16;
                s.map_y = y as u16;
            })
            .await
            else {
                reply(out, "Nguoi choi khong online.");
                return true;
            };
            persist::update_player(pool, target, "MapId", i64::from(map)).await;
            persist::update_player(pool, target, "MapX", i64::from(x)).await;
            persist::update_player(pool, target, "MapY", i64::from(y)).await;
            if target == actor {
                reply(out, &format!("Da cap nhat vi tri {} ({},{}). Hay dung /endtalk neu client dang mo hoi thoai.", map, x, y));
            }
            audit(
                pool,
                actor,
                actor_level,
                Some(target),
                "teleport",
                &format!("map={map},x={x},y={y}"),
            )
            .await;
        }
        "/broadcast" => {
            let text = args.join(" ");
            if text.is_empty() || text.chars().count() > 120 {
                reply(out, "Noi dung broadcast rong hoac qua 120 ky tu.");
                return true;
            }
            let Some(hub) = hub else {
                reply(out, "Broadcast chi hoat dong tren live server.");
                return true;
            };
            hub.broadcast_packet(&spawn::announce_frame(&text)).await;
            audit(pool, actor, actor_level, None, "broadcast", &text).await;
        }
        "/kick" => {
            let Some(target) = parse_u32(args.first()) else {
                reply(out, "Su dung: /kick player_id.");
                return true;
            };
            if target == actor {
                reply(out, "Khong cho phep tu kick tai khoan GM hien tai.");
                return true;
            }
            let Some(hub) = hub else {
                reply(out, "Kick chi hoat dong tren live server.");
                return true;
            };
            if find_online(target).is_none() {
                reply(out, "Nguoi choi khong online.");
                return true;
            }
            hub.send_to(target, &spawn::sys_msg_frame("Ban da bi GM ngat ket noi."))
                .await;
            hub.disconnect_player(target).await;
            audit(
                pool,
                actor,
                actor_level,
                Some(target),
                "kick",
                "online_target_disconnect",
            )
            .await;
        }
        "/setperm" | "/setgm" => {
            let (Some(target), Some(level)) = (parse_u32(args.first()), parse_u32(args.get(1)))
            else {
                reply(out, "Su dung: /setperm player_id gm_level.");
                return true;
            };
            let level = level as i32;
            let Some(repos) = repos else {
                reply(out, "Doi quyen GM yeu cau MySQL live.");
                return true;
            };
            let details = format!("target={target},gm_level={level}");
            let changed = repos
                .accounts()
                .set_gm_level(
                    i64::from(actor),
                    actor_level,
                    i64::from(target),
                    level,
                    "setperm",
                    &details,
                )
                .await
                .unwrap_or(false);
            if !changed {
                reply(out, "Tu choi: can Admin (50+), khong duoc tu cap quyen, va cap duoi quyen nguoi cap.");
                return true;
            }
            if let Some(mut target_session) = find_online(target) {
                target_session.gm_level = level;
                update_online(target, target_session);
                if let Some(hub) = hub {
                    hub.send_to(
                        target,
                        &spawn::sys_msg_frame("Quyen GM cua ban da duoc cap nhat."),
                    )
                    .await;
                }
            }
            reply(out, "Da cap nhat quyen GM va ghi audit.");
        }
        "/battle" | "/quest" | "/setflag" | "/test" => {
            reply(
                out,
                "Lenh nay chua duoc bat: PC frame/persistence contract chua duoc xac minh.",
            );
            audit(pool, actor, actor_level, None, "unsupported_command", &cmd).await;
        }
        _ => help(out),
    }
    true
}
