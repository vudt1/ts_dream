//! Opcode dispatcher (Chapter 2 §2.2).
//!
//! `UpdateMainGrid_Recv` switches on byte [4] and delegates business logic to
//! specialized handlers in `crate::server::handlers`:
//! - `login.rs`: Opcode 0x01 (version check >= 186), 0x03
//! - `chat.rs`: Opcode 0x02 (chat channels, whisper, party, slash commands)
//! - `movement.rs`: Opcode 0x05, 0x06 (movement & map position)
//! - `character.rs`: Opcode 0x09 (character creation & name check)
//! - `expressions.rs`: Opcode 0x20 (actions & expressions)

use crate::battle::service::BattleService;
use crate::data::loader::GameData;
use crate::error::Result;
use crate::protocol::{
    encoder, OP_ACCOUNT, OP_ANTI_ADDICTION, OP_APPARATUS, OP_ARENA_WATERWAR, OP_AUTH, OP_BANK,
    OP_BATTLE, OP_BATTLE_COMMAND, OP_BATTLE_PET, OP_BOAT_RACE, OP_CAFE_ID, OP_CHAT, OP_CHILD,
    OP_COMPOUND, OP_CREATE_CHAR, OP_DICE_BIDAXIAO, OP_DOMINO, OP_EXP_LEVEL, OP_EXPRESS,
    OP_FRIEND_INVITE, OP_GAME_POINTS, OP_GM_ANNOUNCE, OP_GM_MANAGE, OP_GROUP, OP_HOTKEY,
    OP_ITEM, OP_ITEM_INFO, OP_ITEM_MALL, OP_JOB_CHANGE, OP_LOGIN_COMPLETE, OP_LOOK, OP_LOTTO,
    OP_MONEY_SYNC, OP_MOVE, OP_NAVAL_COMBAT, OP_NPC_EVENT, OP_NPC_SHOP, OP_PET, OP_PET_HOTEL,
    OP_PK_SWITCH, OP_PLAYER_UPDATE, OP_QUEST, OP_RANK_ANNOUNCE, OP_REBORN, OP_REBORN_PET,
    OP_RECOMMEND, OP_RELOCATE, OP_RESERVED_44, OP_RESET, OP_SERVER_STATUS, OP_SERVER_SWITCH, OP_SKILL_CS,
    OP_SLOT_MACHINE, OP_SPORT_FORM, OP_STAT_UPDATE, OP_STORAGE, OP_TRADE, OP_WORLD_OBJECT,
    OP_ZM_CHESS,
};
use crate::server::handlers::{
    battle, character, chat, expressions, inventory, login, movement, npc_event, party,
    pet_actions, shops, skills, stats, system, talk, trade_storage, unimplemented,
};
use crate::db::pool::DbPool;
use crate::server::session::Conn;
use crate::web::server_control::{ClientSender, ServerControl};

/// Live-server environment threaded into the dispatcher: the DB pool and the
/// shared client registry (double-login guard + broadcast). Absent (`none`)
/// in golden replay, where handlers run purely in-memory over a seeded session.
#[derive(Clone, Copy)]
pub struct ServerEnv<'a> {
    pub pool: Option<&'a DbPool>,
    pub repos: Option<&'a crate::db::modern::sqlite::SqliteRepositories>,
    pub hub: Option<&'a ServerControl>,
    pub sender: Option<&'a ClientSender>,
}

impl<'a> ServerEnv<'a> {
    pub fn none() -> Self {
        Self {
            pool: None,
            repos: None,
            hub: None,
            sender: None,
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

    /// Send an OP_SYSTEM_ALERT (Opcode 0x00) packet to client.
    pub fn send_system_alert(&mut self, reason: crate::protocol::SystemAlertReason) {
        self.send(reason.to_hex());
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
        // OP_AUTH (0x01), OP_LOOK (0x03) — Login, Enter game confirm
        OP_AUTH => login::handle_login(ctx).await,
        OP_LOOK => login::handle_enter_game(ctx).await,

        // OP_CHAT (0x02) — Chat & slash commands
        OP_CHAT => chat::handle_chat(ctx).await,

        // OP_PLAYER_UPDATE (0x05), OP_MOVE (0x06) — Move
        OP_PLAYER_UPDATE | OP_MOVE => movement::handle_move(ctx),

        // OP_STAT_UPDATE (0x08) — Stat allocation
        OP_STAT_UPDATE => stats::handle_stat_allocation(ctx).await,

        // OP_CREATE_CHAR (0x09) — Character creation & name check
        OP_CREATE_CHAR => character::handle_character(ctx).await,

        // OP_BATTLE (0x0B) — Battle control (ticket 21)
        OP_BATTLE => battle::handle_battle(ctx),

        // OP_RELOCATE (0x0C) — Teleport confirm
        OP_RELOCATE => system::handle_teleport_confirm(ctx),
        // OP_GROUP (0x0D) — Party ops (quan-su designation, ticket 20 G4)
        OP_GROUP => party::handle_party(ctx),
        // OP_PET (0x0F) — Pet actions (release, store, mount, rename, take, swap)
        OP_PET => pet_actions::handle_pet_actions(ctx).await,

        // OP_BATTLE_PET (0x13) — Pet summon / recall
        OP_BATTLE_PET => pet_actions::handle_pet_summon(ctx).await,

        // OP_NPC_EVENT (0x14) — NpcEvent / NPC talk / gate / scene-script
        // (NOT sitting/standing emotions — those are 0x20).
        OP_NPC_EVENT => talk::handle_talk(ctx).await,

        // OP_MONEY_SYNC (0x1A) — MoneySync per mobile-table/C# (S->C money sync from
        // GoldBankHandler); the PC/aLogin payload dialect carries talk
        // selectors instead (not mobile mainKind 20).
        OP_MONEY_SYNC => npc_event::handle_pc_talk(ctx).await,

        // OP_ITEM (0x17) — Inventory family; Level-2 subcode routing lives in the
        // handler module (base ops, use item, player shop, storage, reborn).
        OP_ITEM => inventory::handle_inventory(ctx).await,

        // OP_TRADE (0x19) — Trade P2P items/pets (Bear TransferHandler/TradeItems /
        // TradePet dialect). aLogin never initiates trade on 0x19 (its only
        // C→S 0x19 is the ACK after S→C 0x19/0x29, which falls into the
        // handler's guarded `_` arm), so this path serves Bear-dialect
        // clients; S→C trade frames are Bear-dialect (aLogin would read them
        // as its 0x19 toast/gift bus — see opcode_19.md).
        OP_TRADE => trade_storage::handle_trade(ctx).await,

        // OP_NPC_SHOP (0x1B) — NPC shop buy/sell (Bear NpcShopsHandler dialect).
        // aLogin never sends 0x1B (SendCommand case 0x1b is empty —
        // opcode_1b.md §6: 0x1B is S→C toast bus on aLogin), so this path is
        // Bear-dialect only. S→C replies here are client-safe (020B banner,
        // 1A04 money sync).
        OP_NPC_SHOP => shops::handle_npc_shop(ctx).await,

        // OP_SKILL_CS (0x1C) — Learn / upgrade skills
        OP_SKILL_CS => skills::handle_skills(ctx).await,

        // OP_BANK (0x1D) — Bank gold
        OP_BANK => trade_storage::handle_bank_gold(ctx).await,

        // OP_STORAGE (0x1E) — Storage transfer (TienTrang)
        OP_STORAGE => trade_storage::handle_storage_transfer(ctx).await,

        // OP_PET_HOTEL (0x1F) — Pet stable menu (subs 2/3/4 remap to the 0x0F sub 3/7/8
        // flows in handle_pet_stable). aLogin never sends 0x1F (SendCommand
        // case 0x1f is empty — opcode_1f.md §6), so C→S is Bear-dialect only;
        // S→C stable frames (1F09/1F0C/1F06) are already client-aligned.
        OP_PET_HOTEL => pet_actions::handle_pet_stable(ctx).await,

        // OP_EXPRESS (0x20) — Expressions
        OP_EXPRESS => expressions::handle_expressions(ctx),

        // OP_PK_SWITCH (0x21) — PK/Jam switch
        OP_PK_SWITCH => system::handle_pk_war(ctx).await,

        // OP_GAME_POINTS (0x22) — Game points / God panel
        OP_GAME_POINTS => system::handle_game_points(ctx),

        // OP_ACCOUNT (0x23) — Account management (change pass / delete char / gift code).
        // The "Guild" label in the PC opcode table is a misnomer; the C# server
        // uses 0x23 subs 1/2/3 for account management (see the VISCII string
        // report). handle_account_mgmt is the ported handler.
        OP_ACCOUNT => system::handle_account_mgmt(ctx).await,

        // OP_LOGIN_COMPLETE (0x25) — Login Complete / Map Loaded
        OP_LOGIN_COMPLETE => login::handle_login_complete(ctx).await,

        // OP_HOTKEY (0x28) — Hotkey / skill bar
        OP_HOTKEY => stats::handle_hotkey(ctx).await,

        // OP_REBORN_PET (0x2C) — Pet reborn
        OP_REBORN_PET => skills::handle_pet_reborn(ctx).await,

        // OP_BATTLE_COMMAND (0x32) — Battle commands (ticket 21)
        OP_BATTLE_COMMAND => battle::handle_battle_command(ctx),

        // OP_APPARATUS (0x41) — Rank system
        OP_APPARATUS => system::handle_rank(ctx),

        // OP_ITEM_MALL (0x42) — GM / Mall shop
        OP_ITEM_MALL => system::handle_gm_shop(ctx).await,

        // Documented client opcodes whose full semantics are being ported from
        // the Kotlin/mobile reference. They are deliberately routed through a
        // bounded unimplemented boundary rather than silently discarded.
        OP_FRIEND_INVITE
        | OP_GM_MANAGE
        | OP_WORLD_OBJECT
        | OP_ITEM_INFO
        | OP_JOB_CHANGE
        | OP_EXP_LEVEL
        | OP_RANK_ANNOUNCE
        | OP_QUEST
        | OP_RESET
        | OP_COMPOUND
        | OP_REBORN
        | OP_SERVER_SWITCH
        | OP_SERVER_STATUS
        | OP_CAFE_ID
        | OP_SPORT_FORM
        | OP_DICE_BIDAXIAO
        | OP_DOMINO
        | OP_ZM_CHESS
        | OP_LOTTO
        | OP_ARENA_WATERWAR
        | OP_NAVAL_COMBAT
        | OP_RECOMMEND
        | OP_RESERVED_44
        | OP_CHILD
        | OP_BOAT_RACE
        | OP_ANTI_ADDICTION
        | OP_SLOT_MACHINE
        | OP_GM_ANNOUNCE => unimplemented::handle(ctx),

        _ => unimplemented::handle(ctx),
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
