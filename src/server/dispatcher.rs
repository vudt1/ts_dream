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
use crate::protocol::profile::ProtocolProfile;
use crate::server::handlers::{
    battle, character, chat, compat, expressions, inventory, login, movement, npc_event, party,
    pet_actions, shops, skills, stats, system, talk, trade_storage,
};
use crate::server::session::Conn;
use crate::web::server_control::{ClientSender, ServerControl};
use sqlx::MySqlPool;

/// Live-server environment threaded into the dispatcher: the DB pool and the
/// shared client registry (double-login guard + broadcast). Absent (`none`)
/// in golden replay, where handlers run purely in-memory over a seeded session.
pub struct ServerEnv<'a> {
    pub pool: Option<&'a MySqlPool>,
    pub repos: Option<&'a crate::db::modern::mysql::MySqlRepositories>,
    pub hub: Option<&'a ServerControl>,
    pub sender: Option<&'a ClientSender>,
    pub profile: ProtocolProfile,
}

impl<'a> ServerEnv<'a> {
    pub fn none() -> Self {
        Self {
            pool: None,
            repos: None,
            hub: None,
            sender: None,
            profile: ProtocolProfile::KotlinMobile,
        }
    }
}

/// A map-scoped broadcast frame (op 0x06 move, op 0x20 expressions). The
/// connection loop (`ServerControl::broadcast_map`) delivers `frame` to every
/// other player on `subject`'s map whose id is not `subject`, so the owner
/// never receives its own move/expression (P1/P2/P3 of issue 08).
#[derive(Debug, Clone)]
pub struct MapBroadcast {
    /// The entity the frame is about (used to exclude the owner from its own
    /// broadcast and, in the follow flow, co-locates on the origin's map).
    pub subject: u32,
    pub frame: String,
}

/// One outgoing server→client frame and its optional pacing delay.
///
/// The pacing and the frame travel as one unit (previously two parallel
/// vectors that callers had to re-index in lockstep — a data clump). `delay_ms`
/// is hinted for `TalkMessages` (500 ms between dialog fragments); golden
/// replay ignores pacing — the frame order is unchanged and byte-parity holds.
#[derive(Debug, Clone)]
pub struct OutFrame {
    pub frame: String,
    pub delay_ms: u64,
}

impl OutFrame {
    pub fn new(frame: impl Into<String>) -> Self {
        Self {
            frame: frame.into(),
            delay_ms: 0,
        }
    }

    pub fn with_delay(frame: impl Into<String>, delay_ms: u64) -> Self {
        Self {
            frame: frame.into(),
            delay_ms,
        }
    }

    pub fn contains(&self, pat: &str) -> bool {
        self.frame.contains(pat)
    }

    pub fn starts_with(&self, pat: &str) -> bool {
        self.frame.starts_with(pat)
    }

    pub fn ends_with(&self, pat: &str) -> bool {
        self.frame.ends_with(pat)
    }
}

impl PartialEq<str> for OutFrame {
    fn eq(&self, other: &str) -> bool {
        self.frame == other
    }
}

impl PartialEq<&str> for OutFrame {
    fn eq(&self, other: &&str) -> bool {
        self.frame == *other
    }
}

impl PartialEq<String> for OutFrame {
    fn eq(&self, other: &String) -> bool {
        self.frame == *other
    }
}

impl PartialEq<&String> for OutFrame {
    fn eq(&self, other: &&String) -> bool {
        self.frame == **other
    }
}

/// Result of handling one decoded frame.
#[derive(Debug, Default, Clone)]
pub struct HandleOutcome {
    pub outgoing: Vec<OutFrame>,
    pub shutdown: bool,
    /// If set, a TEAMDEF battle should be triggered after processing.
    pub battle_trigger: Option<crate::server::handlers::quest::BattleTrigger>,
    pub map_broadcast: Vec<MapBroadcast>,
}

impl HandleOutcome {
    pub fn send(&mut self, frame: impl Into<String>) {
        self.outgoing.push(OutFrame::new(frame));
    }

    /// Send `frame` after `delay_ms` (non-blocking). The first paced fragment
    /// is delayed too, matching the legacy behavior of splitting each dialog
    /// line 500 ms apart.
    pub fn send_delayed(&mut self, frame: impl Into<String>, delay_ms: u64) {
        self.outgoing.push(OutFrame::with_delay(frame, delay_ms));
    }

    /// Queue a map-scoped broadcast frame owned by `subject`. The fan-out is
    /// performed by the connection loop (`ServerControl::broadcast_map`), which
    /// never sends the frame back to `subject` or to other maps.
    pub fn broadcast(&mut self, subject: u32, frame: impl Into<String>) {
        self.map_broadcast.push(MapBroadcast {
            subject,
            frame: frame.into(),
        });
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
            repos: env.repos,
            hub: env.hub,
            sender: env.sender,
            profile: env.profile,
        },
    };
    // Handler errors are swallowed by design: never propagate to the caller.
    let _ = handle(&mut ctx).await;
    let trigger = ctx.out.battle_trigger.take();
    if let Some(trigger) = trigger {
        if ctx
            .service
            .start_teamdef_battle(&mut ctx.conn.session, &trigger)
            > 0
        {
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

        // Op 0x06 — Move
        0x06 => movement::handle_move(ctx),

        // Op 0x08 — Stat allocation
        0x08 => stats::handle_stat_allocation(ctx).await,

        // Op 0x09 — Character creation & name check
        0x09 => character::handle_character(ctx).await,

        // Op 0x0B — Battle control (ticket 21)
        0x0B => battle::handle_battle(ctx),

        // Op 0x0C — Teleport confirm
        0x0C => system::handle_teleport_confirm(ctx),
        // Op 0x0D — Party ops (quan-su designation, ticket 20 G4)
        0x0D => party::handle_party(ctx),
        // Op 0x0F — Pet actions (release, store, mount, rename, take, swap)
        0x0F => pet_actions::handle_pet_actions(ctx).await,

        // Op 0x13 — Pet summon / recall
        0x13 => pet_actions::handle_pet_summon(ctx).await,

        // Op 0x14 — Action / legacy PC talk
        0x14 => talk::handle_talk(ctx).await,

        // Op 0x1A — PC Talk/Eve selector family. This is not mobile mainKind
        // 20; it has a separate PC/aLogin payload dialect.
        0x1A => match ctx.env.profile {
            ProtocolProfile::PcALogin => npc_event::handle_pc_talk(ctx).await,
            ProtocolProfile::KotlinMobile => compat::handle(ctx),
        },

        // Op 0x17 — Inventory family; Level-2 subcode routing lives in the
        // handler module (base ops, use item, player shop, storage, reborn).
        0x17 => inventory::handle_inventory(ctx).await,

        // Op 0x19 — Trade in Kotlin/mobile; SceneManage in the PC table.
        0x19 => match ctx.env.profile {
            ProtocolProfile::KotlinMobile => trade_storage::handle_trade(ctx).await,
            ProtocolProfile::PcALogin => compat::handle(ctx),
        },

        // Op 0x1B — NPC shop in Kotlin/mobile; Trade in the PC table.
        0x1B => match ctx.env.profile {
            ProtocolProfile::KotlinMobile => shops::handle_npc_shop(ctx).await,
            ProtocolProfile::PcALogin => compat::handle(ctx),
        },

        // Op 0x1C — Learn / upgrade skills
        0x1C => skills::handle_skills(ctx).await,

        // Op 0x1D — Bank gold
        0x1D => trade_storage::handle_bank_gold(ctx).await,

        // Op 0x1E — Storage transfer (TienTrang)
        0x1E => trade_storage::handle_storage_transfer(ctx).await,

        // Op 0x1F — Pet stable in Kotlin/mobile; NPC shop in the PC table.
        0x1F => match ctx.env.profile {
            ProtocolProfile::KotlinMobile => pet_actions::handle_pet_stable(ctx).await,
            ProtocolProfile::PcALogin => compat::handle(ctx),
        },

        // Op 0x20 — Expressions
        0x20 => expressions::handle_expressions(ctx),

        // Op 0x21 — PK / War mode
        0x21 => system::handle_pk_war(ctx).await,

        // Op 0x22 — Game points / God panel
        0x22 => system::handle_game_points(ctx),

        // Op 0x23 — Account management in Kotlin/mobile; Guild in the PC table.
        0x23 => match ctx.env.profile {
            ProtocolProfile::KotlinMobile => system::handle_account_mgmt(ctx).await,
            ProtocolProfile::PcALogin => compat::handle(ctx),
        },

        // Op 0x28 — Hotkey / skill bar
        0x28 => stats::handle_hotkey(ctx).await,

        // Op 0x2C — Pet reborn
        0x2C => skills::handle_pet_reborn(ctx).await,

        // Op 0x32 — Battle commands (ticket 21)
        0x32 => battle::handle_battle_command(ctx),

        // Op 0x41 — Rank system
        0x41 => system::handle_rank(ctx),

        // Op 0x42 — GM / Mall shop
        0x42 => system::handle_gm_shop(ctx).await,

        // Documented client opcodes whose full semantics are being ported from
        // the Kotlin/mobile reference. They are deliberately routed through a
        // bounded compatibility boundary rather than silently discarded.
        0x05 | 0x0A | 0x0E | 0x10 | 0x12 | 0x16 | 0x18 | 0x24 | 0x25 | 0x26 | 0x27 | 0x29
        | 0x2A | 0x2B | 0x2D | 0x2E | 0x36 | 0x37 | 0x39 | 0x3A | 0x3B | 0x3C | 0x3D | 0x3F
        | 0x40 | 0x43 | 0x44 | 0x45 | 0x46 | 0x47 | 0x48 | 0xC7 => compat::handle(ctx),

        _ => compat::handle(ctx),
    }
    Ok(())
}

/// Convert a raw decoded byte frame into the hex string (for callers that
/// already have the bytes rather than the wire hex).
pub fn hex_of(decoded: &[u8]) -> String {
    encoder::hex(decoded)
}

/// Build an `OpcodeCtx` for a test that drives one handler directly.
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
