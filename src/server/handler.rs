//! Opcode dispatcher (Chapter 2 §2.2).
//!
//! `UpdateMainGrid_Recv` switches on byte [4] and delegates business logic to
//! specialized handlers in `crate::server::handlers`:
//! - `login.rs`: Opcode 0x00, 0x01 (version check >= 186), 0x03
//! - `chat.rs`: Opcode 0x02 (chat channels, whisper, party, slash commands)
//! - `movement.rs`: Opcode 0x05, 0x06 (movement & map position)
//! - `character.rs`: Opcode 0x09 (character creation & name check)
//! - `expressions.rs`: Opcode 0x20 (actions & expressions)

use crate::battle::service::BattleService;
use crate::data::loader::GameData;
use crate::error::Result;
use crate::protocol::encoder;
use crate::server::handlers::{
    battle, character, chat, expressions, inventory, login, movement, pet_actions, shops, skills,
    stats, system, talk, trade_storage,
};
use crate::server::session::Conn;
use crate::web::server_control::{ClientSender, ServerControl};
use sqlx::MySqlPool;

/// Live-server environment threaded into the dispatcher: the DB pool and the
/// shared client registry (double-login guard + broadcast). Absent (`none`)
/// in golden replay, where handlers run purely in-memory over a seeded session.
pub struct ServerEnv<'a> {
    pub pool: Option<&'a MySqlPool>,
    pub hub: Option<&'a ServerControl>,
    pub sender: Option<&'a ClientSender>,
}

impl<'a> ServerEnv<'a> {
    pub fn none() -> Self {
        Self {
            pool: None,
            hub: None,
            sender: None,
        }
    }
}

/// Result of handling one decoded frame.
#[derive(Debug, Default, Clone)]
pub struct HandleOutcome {
    pub outgoing: Vec<String>,
    pub shutdown: bool,
    /// If set, a TEAMDEF battle should be triggered after processing.
    pub battle_trigger: Option<crate::server::handlers::quest::BattleTrigger>,
}

impl HandleOutcome {
    pub fn send(&mut self, frame: impl Into<String>) {
        self.outgoing.push(frame.into());
    }
}

/// Everything an opcode handler may touch, bundled into one context so the
/// dispatcher and every handler share a single, uniform interface.
pub struct OpcodeCtx<'a> {
    pub conn: &'a mut Conn,
    pub data: &'a GameData,
    pub service: &'a BattleService,
    pub out: &'a mut HandleOutcome,
    pub opcode: u8,
    pub sub: u8,
    pub payload: &'a [u8],
    /// The full decoded frame (a few handlers re-parse it).
    pub decoded: &'a [u8],
    /// Live-server environment (DB + client registry); `None` in golden replay.
    pub env: ServerEnv<'a>,
}

/// Dispatch one full decoded frame (its bytes). `conn` carries session state,
/// `data` gives read tables, `service` drives the battle engine, `env` carries
/// the live DB pool + client registry (or `ServerEnv::none()` for replay).
/// Handlers run inside a silent catch. A `battle_trigger` produced by a talk
/// is processed here (the TeamDef battle is spawned after the talk's own frames).
pub async fn dispatch(
    conn: &mut Conn,
    decoded: &[u8],
    data: &GameData,
    service: &BattleService,
    env: &ServerEnv<'_>,
) -> HandleOutcome {
    let mut out = HandleOutcome::default();
    let mut ctx = OpcodeCtx {
        conn,
        data,
        service,
        out: &mut out,
        opcode: decoded.get(4).copied().unwrap_or(0),
        sub: decoded.get(5).copied().unwrap_or(0),
        payload: decoded.get(6..).unwrap_or(&[]),
        decoded,
        env: ServerEnv {
            pool: env.pool,
            hub: env.hub,
            sender: env.sender,
        },
    };
    // C# swallows handler exceptions: never propagate.
    let _ = handle(&mut ctx).await;
    let trigger = ctx.out.battle_trigger.take();
    if let Some(trigger) = trigger {
        if ctx.service.start_teamdef_battle(&mut ctx.conn.session, &trigger) > 0 {
            // The open-board frames were pushed through the service channels.
        }
    }
    out
}

async fn handle(ctx: &mut OpcodeCtx<'_>) -> Result<()> {
    match ctx.opcode {
        // Op 0x00, 0x01, 0x03 — Hello, Login, Enter game confirm
        0x00 => login::handle_hello(ctx),
        0x01 => login::handle_login(ctx).await,
        0x03 => login::handle_enter_game(ctx).await,

        // Op 0x02 — Chat & slash commands
        0x02 => chat::handle_chat(ctx).await,

        // Op 0x05, 0x06 — Move
        0x05 | 0x06 => movement::handle_move(ctx),

        // Op 0x08 — Stat allocation
        0x08 => stats::handle_stat_allocation(ctx).await,

        // Op 0x09 — Character creation & name check
        0x09 => character::handle_character(ctx).await,

        // Op 0x0B — Battle control (ticket 21)
        0x0B => battle::handle_battle(ctx),

        // Op 0x0C — Teleport confirm
        0x0C => system::handle_teleport_confirm(ctx),

        // Op 0x0F — Pet actions (release, store, mount, rename, take, swap)
        0x0F => pet_actions::handle_pet_actions(ctx),

        // Op 0x13 — Pet summon / recall
        0x13 => pet_actions::handle_pet_summon(ctx),

        // Op 0x14 — Action / Talk
        0x14 => talk::handle_talk(ctx),

        // Op 0x17 — Inventory base, use item, player shop, reborn
        0x17 => {
            if (30..=33).contains(&ctx.sub) {
                shops::handle_player_shop(ctx).await;
            } else {
                inventory::handle_inventory(ctx).await;
            }
        }

        // Op 0x19 — Trade
        0x19 => trade_storage::handle_trade(ctx),

        // Op 0x1B — NPC shop buy/sell
        0x1B => shops::handle_npc_shop(ctx).await,

        // Op 0x1C — Learn / upgrade skills
        0x1C => skills::handle_skills(ctx).await,

        // Op 0x1D — Bank gold
        0x1D => trade_storage::handle_bank_gold(ctx),

        // Op 0x1E — Storage transfer (TienTrang)
        0x1E => trade_storage::handle_storage_transfer(ctx),

        // Op 0x1F — Pet stable menu
        0x1F => pet_actions::handle_pet_stable(ctx),

        // Op 0x20 — Expressions
        0x20 => expressions::handle_expressions(ctx),

        // Op 0x21 — PK / War mode
        0x21 => system::handle_pk_war(ctx),

        // Op 0x22 — Game points / God panel
        0x22 => system::handle_game_points(ctx),

        // Op 0x23 — Account management
        0x23 => system::handle_account_mgmt(ctx),

        // Op 0x28 — Hotkey / skill bar
        0x28 => stats::handle_hotkey(ctx).await,

        // Op 0x2C — Pet reborn
        0x2C => skills::handle_pet_reborn(ctx).await,

        // Op 0x32 — Battle commands (ticket 21)
        0x32 => battle::handle_battle_command(ctx),

        // Op 0x41 — Rank system
        0x41 => system::handle_rank(ctx),

        // Op 0x42 — GM / Mall shop
        0x42 => system::handle_gm_shop(ctx),

        _ => {
            // Not yet ported / unknown: silently ignored.
        }
    }
    Ok(())
}

/// Convert a raw decoded byte frame into the hex string (for callers that
/// already have the bytes rather than the wire hex).
pub fn hex_of(decoded: &[u8]) -> String {
    encoder::hex(decoded)
}

/// Build an `OpcodeCtx` for a test that drives one handler directly.
#[cfg(test)]
pub fn test_ctx<'a>(
    conn: &'a mut Conn,
    data: &'a GameData,
    service: &'a BattleService,
    out: &'a mut HandleOutcome,
    sub: u8,
    payload: &'a [u8],
) -> OpcodeCtx<'a> {
    OpcodeCtx {
        conn,
        data,
        service,
        out,
        opcode: 0,
        sub,
        payload,
        decoded: &[],
        env: ServerEnv::none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_data() -> GameData {
        let mut data = GameData::default();
        data.skills.insert(
            10001,
            crate::data::tables::Skill {
                id: 10001,
                point: 1,
                lv_max: 10,
                ..Default::default()
            },
        );
        data
    }

    fn dummy_service() -> BattleService {
        BattleService::new(std::sync::Arc::new(GameData::default()))
    }

    #[tokio::test]
    async fn hello_replies() {
        let mut conn = Conn::new();
        // frame: F4 44 01 00 00 (opcode 0x00, length 1, no sub byte).
        let decoded = encoder::bytes("F444010000").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(out.outgoing, vec!["F4440300010901"]);
    }

    #[tokio::test]
    async fn login_version_too_low_causes_shutdown() {
        let mut conn = Conn::new();
        // Login payload with version 100 (< 186): opcode 0x01 sub 0x01 id=1 prefix="vn" ver=100 pass="123"
        let decoded = encoder::bytes("F4440B00010101000000766E6400313233").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert!(out.shutdown);
    }

    #[tokio::test]
    async fn login_wrong_password() {
        let mut conn = Conn::new();
        // ver=186 (0xBA), pass="WRONG"
        let decoded = encoder::bytes("F4440D00010101000000766EBA0057524F4E47").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(out.outgoing, vec!["F44402000106"]);
        assert!(!out.shutdown);
    }

    #[tokio::test]
    async fn create_character_name_check_and_creation() {
        let mut conn = Conn::new();

        // 1. Name check free: opcode 0x09 sub 2 name "TESTNAME"
        let name_check_decoded = encoder::bytes("F4440A000902544553544E414D45").unwrap();
        let out1 = dispatch(&mut conn, &name_check_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(out1.outgoing, vec!["F4440300090300"]);
        assert_eq!(conn.session.pending_new_char_name, b"TESTNAME");

        // 2. Create character: opcode 0x09 sub 1 with valid payload
        let mut payload = vec![0u8; 26];
        payload[0] = 1; // sex
        payload[2] = 2; // hair
        payload[12] = 3; // element
        payload[19] = 4; // pass1 len
        let mut frame_bytes = vec![0xF4, 0x44, 28, 0x00, 0x09, 0x01];
        frame_bytes.extend(payload);

        let out2 = dispatch(&mut conn, &frame_bytes, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(out2.outgoing, vec!["F44402000901"]);
    }

    #[tokio::test]
    async fn move_broadcasts_to_map() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // Move opcode 0x06 sub 1: dir=2, x=100 (0x0064), y=200 (0x00C8)
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(&mut conn, &move_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.map_x, 100);
        assert_eq!(conn.session.map_y, 200);
        assert_eq!(conn.session.gocnhin, 2);
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].starts_with("F4440B000601"));
    }

    #[tokio::test]
    async fn move_ignored_while_in_battle() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.battle_id = 7;
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(&mut conn, &move_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert!(out.outgoing.is_empty(), "move must be ignored in battle");
    }

    #[tokio::test]
    async fn move_leader_moves_party_members() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.id_leader = 300001;
        conn.session.id_mem = [300002, 300003, 0, 0];
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(&mut conn, &move_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(out.outgoing.len(), 3, "leader + 2 members each broadcast a walk");
        assert!(out.outgoing[0].starts_with("F4440B000601E1930400"));
        assert!(out.outgoing[1].starts_with("F4440B000601E2930400")); // member 300002
        assert!(out.outgoing[2].starts_with("F4440B000601E3930400")); // member 300003
    }

    #[tokio::test]
    async fn move_member_following_leader_stays_still() {
        let mut conn = Conn::new();
        conn.session.id = 300002;
        conn.session.id_leader = 300001; // not self
        let move_decoded = encoder::bytes("F44407000601026400C800").unwrap();
        let out = dispatch(&mut conn, &move_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert!(out.outgoing.is_empty(), "member does not self-broadcast");
    }

    #[tokio::test]
    async fn expression_handling() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        // Expression sub 2 action=5
        let expr_decoded = encoder::bytes("F4440300200205").unwrap();
        let out = dispatch(&mut conn, &expr_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.dongtac, 5);
        assert_eq!(out.outgoing, vec!["F44407002002E193040005"]);
    }

    #[tokio::test]
    async fn chat_slash_command_where() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.map_id = 12001;
        conn.session.map_x = 400;
        conn.session.map_y = 500;
        // Chat "/where": op 0x02 sub 2 msg="/where"
        let chat_decoded = encoder::bytes("F4440C0002022F7768657265").unwrap();
        let out = dispatch(&mut conn, &chat_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(out.outgoing.len(), 1);
        assert!(out.outgoing[0].contains("020B")); // sys msg frame
    }

    #[tokio::test]
    async fn dispatch_stat_allocation_and_hotkey() {
        let mut conn = Conn::new();
        conn.session.point = 10;
        // Op 0x08 sub 1: stat_id 27 (Int), points 3 -> hex: F444 0400 0801 1B03
        let decoded = encoder::bytes("F444040008011B03").unwrap();
        let out = dispatch(&mut conn, &decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.point, 7);
        assert_eq!(conn.session.int1, 3);
        assert_eq!(out.outgoing.len(), 2);

        // Op 0x28 sub 1: skill 10001 (0x2711), slot 5 -> hex: F444 0400 2801 1127 05
        let decoded_hotkey = encoder::bytes("F4440500280100112705").unwrap();
        let out_hk = dispatch(&mut conn, &decoded_hotkey, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.hotkeys[5], 10001);
        assert!(out_hk.outgoing.is_empty());
    }

    #[tokio::test]
    async fn dispatch_npc_shop_player_shop_and_skill_learn() {
        let mut conn = Conn::new();
        conn.session.id = 300001;
        conn.session.idtalking = 16;
        conn.session.map_id = 12002;
        conn.session.gold = 100000;
        conn.session.skill_point = 5;

        // NPC Shop buy (op 0x1B): menu 0 at map 12002 → item 20023 @ 58800 → gold 41200
        let shop_decoded = encoder::bytes("F44404001B010000").unwrap();
        let out_shop = dispatch(&mut conn, &shop_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.gold, 41200);
        assert!(conn.session.homdo.iter().any(|i| i.id == 20023));
        assert!(out_shop.outgoing.iter().any(|f| f.contains("1A04")));

        // Player shop open (op 0x17 sub 30): name "TEST" + one listing.
        let open_hex = crate::protocol::frame("171E", "045445535400");
        let open_decoded = encoder::bytes(&open_hex).unwrap();
        let out_open = dispatch(&mut conn, &open_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert!(conn.session.shop.active);
        assert!(out_open.outgoing.iter().any(|f| f.contains("171E")));

        // Player shop close (op 0x17 sub 31 / wire 171F): reply 1720 + player id.
        let close_hex = crate::protocol::frame("171F", "");
        let close_decoded = encoder::bytes(&close_hex).unwrap();
        let out_close = dispatch(&mut conn, &close_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert!(!conn.session.shop.active);
        assert!(out_close.outgoing.iter().any(|f| f.contains("1720")));

        // Skill learn skill 10001 (0x2711) lv 1 -> F444 0500 1C01 1127 01
        let skill_decoded = encoder::bytes("F44405001C01112701").unwrap();
        let out_skill = dispatch(&mut conn, &skill_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.skills.len(), 1);
        assert_eq!(conn.session.skill_point, 4);
        assert_eq!(out_skill.outgoing.len(), 2);
    }

    #[tokio::test]
    async fn dispatch_trade_bank_pk_and_pets() {
        let mut conn = Conn::new();
        conn.session.gold = 5000;
        conn.session.bank_gold = 2000;

        // Op 0x1D sub 1: withdraw 1000 gold -> F444 0400 1D01 E803
        let bank_decoded = encoder::bytes("F44404001D01E803").unwrap();
        let out_bank = dispatch(&mut conn, &bank_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.gold, 6000);
        assert_eq!(conn.session.bank_gold, 1000);
        assert_eq!(out_bank.outgoing.len(), 3);

        // Op 0x21 sub 1: set PK = 1 -> F444 0300 2101 01
        let pk_decoded = encoder::bytes("F4440300210101").unwrap();
        let out_pk = dispatch(&mut conn, &pk_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.pk, 1);
        assert_eq!(out_pk.outgoing[0], "F444040021020100");

        // Op 0x14 sub 1: talk start banker 16080 (0x3ED0) -> F444 0400 1401 D03E
        let talk_decoded = encoder::bytes("F44404001401D03E").unwrap();
        let out_talk = dispatch(&mut conn, &talk_decoded, &dummy_data(), &dummy_service(), &ServerEnv::none()).await;
        assert_eq!(conn.session.idtalking, 16080);
        assert_eq!(out_talk.outgoing.len(), 2);
    }
}
