//! Wire protocol layer (Chapter 2).
//!
//! Implements the exact framing and primitive encoders of the TS Online wire
//! protocol so traffic stays byte-identical to the captured originals.
//! Everything here is a pure transform and is unit-tested without a socket.

/// XOR key (Chapter 8, §2.1). Hardcoded — never configure it.
pub const XOR_KEY: u8 = 0xAD;

/// Frame header magic `F4 44`.
pub const HEADER_TS_MAGIC: [u8; 2] = [0xF4, 0x44];

/// Minimum client version (§2.3.2). Below this the connection is shut down.
pub const MIN_VERSION: u16 = 186;

/// Server/account id prefix (§1.5).
pub const ID_PREFIX: &str = "VN";

/// Server name (§1.5).
pub const SERVER_NAME: &str = "TSVN";

/// Maximum level.
pub const MAX_LEVEL: i64 = 200;

// ============================================================================
// Server Main Opcodes
// ============================================================================

/// [0x00] System warning messages / alert (S -> C only)
pub const OP_SYSTEM_ALERT: u8 = 0x00;
/// [0x01] Auth / Login / Logout / Kick / ServerList 
pub const OP_AUTH: u8 = 0x01;
/// [0x02] Chat (Normal, Team, Guild, World, Whisper) 
pub const OP_CHAT: u8 = 0x02;

/// Chat channel sub-opcodes (opcode `0x02`).
///
/// Wire layout per channel: `[02][sub][id:4B LE][msg]` (except `CHAT_SUB_SYSTEM`,
/// which carries no id). Vietnamese labels are the exact client render tags
/// verified in `.scratch/client-pseudo-op-code/opcode_02.md` §3-§5, so server
/// code must reference these names — never hardcode the byte value.
/// Channels `0x01`, `0x02`, `0x03`, `0x05`, `0x06`, `0x07` are filtered by the
/// client's per-channel flags (`TFT_ChannelForm +0x168..+0x16D`); subs `9` and
/// `0x0A` have no client branch and must never be emitted.
/// (Công bố hệ thống) — system broadcast, no channel gate.
pub const CHAT_SUB_BROADCAST: u8 = 0x00;
/// (Thần)Thiên thần — angel/GM broadcast; heavily gated client-side
/// (receiver class in [5..8] + flag `+0x16D` or magic `0xB3B6`).
pub const CHAT_SUB_ANGEL: u8 = 0x01;
/// (Gần) — near/map chat; gated by channel flag `+0x168`.
pub const CHAT_SUB_NEAR: u8 = 0x02;
/// (Thì Thầm) — whisper; gated by channel flag `+0x169`.
/// Frame id must be the *sender* id; ids 100..400 take NPC branches.
pub const CHAT_SUB_WHISPER: u8 = 0x03;
/// (GM) — GM label, no channel gate (server must gate the sender instead).
pub const CHAT_SUB_GM: u8 = 0x04;
/// (Đài) — loudspeaker/megaphone; gated by `+0x16A` and only displayed when
/// the receiver resolves the sender name.
pub const CHAT_SUB_LOUDSPEAKER: u8 = 0x05;
/// (Đoàn) — guild/party channel; gated by channel flag `+0x16B`.
pub const CHAT_SUB_GUILD: u8 = 0x06;
/// (Minh) — local/self-talk; gated by channel flag `+0x16C`.
pub const CHAT_SUB_LOCAL: u8 = 0x07;
/// Input-bar flag (not a message): empty payload, sets the client's input
/// cooldown/typing flag. Send only after verifying on a real client.
pub const CHAT_SUB_INPUT_FLAG: u8 = 0x08;
/// (Tổng Cũ) — long memo accumulated until the `"#end"` sentinel.
pub const CHAT_SUB_MEMO: u8 = 0x0B;
/// (Công bố hệ thống) — pure system line, fixed id `0`, no channel gate.
pub const CHAT_SUB_SYSTEM: u8 = 0x0C;
/// [0x03] Look / Inspect Target Player / NPC 
pub const OP_LOOK: u8 = 0x03;
/// [0x04] PlayerAppear in FOV 
pub const OP_PLAYER_APPEAR: u8 = 0x04;
/// [0x05] PlayerUpdate  / MoveConfirm 
pub const OP_PLAYER_UPDATE: u8 = 0x05;
/// [0x06] Move (Tọa độ di chuyển ô gạch Isometric) 
pub const OP_MOVE: u8 = 0x06;
/// [0x07] Teleport / PositionSync tuyệt đối + MapID (Shared, không SubOp)
pub const OP_TELEPORT: u8 = 0x07;
#[deprecated(note = "renamed to OP_TELEPORT, see .scratch/opcode-rename/spec.md")]
pub const OP_PLAYER_DETAIL: u8 = OP_TELEPORT;
/// [0x08] StatUpdate / StatPoint cộng điểm 
pub const OP_STAT_UPDATE: u8 = 0x08;
/// [0x09] CreateChar  / CreateCharResult
pub const OP_CREATE_CHAR: u8 = 0x09;
/// [0x0B] Battle (Hệ thống chiến đấu 10vs10 theo lượt) 
pub const OP_BATTLE: u8 = 0x0B;
/// [0x0C] Relocate / Warp Gate chuyển map 
pub const OP_RELOCATE: u8 = 0x0C;
/// [0x0D] Group / Team (Tổ đội người chơi) 
pub const OP_GROUP: u8 = 0x0D;
/// [0x0E] FriendInvite / SocialEvent notify (S->C chính)
pub const OP_FRIEND_INVITE: u8 = 0x0E;
#[deprecated(note = "renamed to OP_FRIEND_INVITE, see .scratch/opcode-rename/spec.md")]
pub const OP_MAIL: u8 = OP_FRIEND_INVITE;
/// [0x0F] Pet / Võ tướng đồng hành & Lòng trung thành FAI 
pub const OP_PET: u8 = 0x0F;
/// [0x10] GmManage — quiz/GM + syschat (S->C, C->S rỗng)
pub const OP_GM_MANAGE: u8 = 0x10;
#[deprecated(note = "renamed to OP_GM_MANAGE, see .scratch/opcode-rename/spec.md")]
pub const OP_NPC_MANAGE: u8 = OP_GM_MANAGE;
/// [0x13] BattlePet (Chiến thuật & Điều khiển võ tướng trong trận)
pub const OP_BATTLE_PET: u8 = 0x13;
/// [0x14] NpcEvent — NPC talk/cổng/scene-script (Shared)
pub const OP_NPC_EVENT: u8 = 0x14;
#[deprecated(note = "renamed to OP_NPC_EVENT (scene-script), see .scratch/opcode-rename/spec.md")]
pub const OP_ACTION: u8 = OP_NPC_EVENT;
/// [0x16] WorldObject — world-object sync slot 0..100 (S->C)
pub const OP_WORLD_OBJECT: u8 = 0x16;
#[deprecated(note = "renamed to OP_WORLD_OBJECT, see .scratch/opcode-rename/spec.md")]
pub const OP_SKILL_SC: u8 = OP_WORLD_OBJECT;
/// [0x17] Item & Inventory (Túi đồ, Trang bị - Chuẩn 112 Sub-Ops)
pub const OP_ITEM: u8 = 0x17;
/// [0x18] ItemInfo (tạm giữ) — stock/sync tồn kho, chưa rõ struct chi tiết — Phase3
pub const OP_ITEM_INFO: u8 = 0x18;
/// [0x19] Trade — trade P2P items/pets (Shared)
/// NOTE (opcode-rename #03): identifier OP_TRADE rotated from 0x1B to 0x19;
/// old OP_TRADE (0x1B) callers must switch to OP_NPC_SHOP.
pub const OP_TRADE: u8 = 0x19;
#[deprecated(note = "renamed to OP_TRADE, see .scratch/opcode-rename/spec.md")]
pub const OP_SCENE_MANAGE: u8 = OP_TRADE;
/// [0x1A] MoneySync — đồng bộ tiền/bộ đếm/banner (S->C)
pub const OP_MONEY_SYNC: u8 = 0x1A;
#[deprecated(note = "renamed to OP_MONEY_SYNC, see .scratch/opcode-rename/spec.md")]
pub const OP_TALK: u8 = OP_MONEY_SYNC;
/// [0x1B] NpcShop — mua/bán NPC (Shared)
/// NOTE (opcode-rename #03): identifier OP_NPC_SHOP rotated from 0x1F to 0x1B;
/// old OP_NPC_SHOP (0x1F) callers must switch to OP_PET_HOTEL.
pub const OP_NPC_SHOP: u8 = 0x1B;
/// [0x1C] Skill (Client gửi yêu cầu thi triển/học kỹ năng lên Server)
pub const OP_SKILL_CS: u8 = 0x1C;
/// [0x1D] Bank (Tiền trang / Gửi rút ngân lượng) 
pub const OP_BANK: u8 = 0x1D;
/// [0x1E] Storage (Rương Nhà Trọ & Ba Đậu Yêu Balo) 
pub const OP_STORAGE: u8 = 0x1E;
/// [0x1F] PetHotel — khách điếm pet (Shared)
pub const OP_PET_HOTEL: u8 = 0x1F;
/// [0x20] Express (Chuyển phát nhanh bưu kiện) 
pub const OP_EXPRESS: u8 = 0x20;
/// [0x21] PkSwitch — công tắc PK/Jam (Shared)
pub const OP_PK_SWITCH: u8 = 0x21;
#[deprecated(note = "renamed to OP_PK_SWITCH, see .scratch/opcode-rename/spec.md")]
pub const OP_WELCOME: u8 = OP_PK_SWITCH;
/// [0x22] GamePoints (Điểm nạp thẻ / Tích lũy tài khoản) 
pub const OP_GAME_POINTS: u8 = 0x22;
/// [0x23] Account — đổi pass/xóa char/gift-code + sendpoint/voucher (Shared)
pub const OP_ACCOUNT: u8 = 0x23;
#[deprecated(note = "renamed to OP_ACCOUNT, see .scratch/opcode-rename/spec.md")]
pub const OP_GUILD: u8 = OP_ACCOUNT;
/// [0x24] JobChange — đổi job + slot pet học skill (S->C)
pub const OP_JOB_CHANGE: u8 = 0x24;
#[deprecated(note = "renamed to OP_JOB_CHANGE, see .scratch/opcode-rename/spec.md")]
pub const OP_GUILD_INFO: u8 = OP_JOB_CHANGE;
/// [0x25] LoginComplete — announceAppear sau login (C->S chính)
pub const OP_LOGIN_COMPLETE: u8 = 0x25;
#[deprecated(note = "renamed to OP_LOGIN_COMPLETE, see .scratch/opcode-rename/spec.md")]
pub const OP_GUILD_ACTION: u8 = OP_LOGIN_COMPLETE;
/// [0x26] ExpLevel — đồng bộ exp/level trần 200 + banner (S->C)
pub const OP_EXP_LEVEL: u8 = 0x26;
#[deprecated(note = "renamed to OP_EXP_LEVEL, see .scratch/opcode-rename/spec.md")]
pub const OP_GUILD_BATTLE: u8 = OP_EXP_LEVEL;
/// [0x27] RankAnnounce — bảng rank + invite/banner + announceAppear (S->C)
pub const OP_RANK_ANNOUNCE: u8 = 0x27;
#[deprecated(note = "renamed to OP_RANK_ANNOUNCE, see .scratch/opcode-rename/spec.md")]
pub const OP_SYSTEM_MASTER: u8 = OP_RANK_ANNOUNCE;
/// [0x28] Hotkey (Cấu hình phím tắt nhanh QuickBar F1..F8)
pub const OP_HOTKEY: u8 = 0x28;
/// [0x29] Quest (Nhiệm vụ & Tiến trình kịch bản cốt truyện)
pub const OP_QUEST: u8 = 0x29;
/// [0x2A] Reset — reset skill/stat qua item (C->S)
pub const OP_RESET: u8 = 0x2A;
#[deprecated(note = "renamed to OP_RESET, see .scratch/opcode-rename/spec.md")]
pub const OP_FRIEND: u8 = OP_RESET;
/// [0x2B] Compound (Luyện đồ Mix Item & Tinh luyện khoáng thạch)
pub const OP_COMPOUND: u8 = 0x2B;
/// [0x2C] RebornPet (Chuyển sinh Võ tướng: RB1, RB2, RB3)
pub const OP_REBORN_PET: u8 = 0x2C;
/// [0x2D] Reborn (Chuyển sinh nhân vật: Chuyển Sinh CS, Trùng Sinh TS)
pub const OP_REBORN: u8 = 0x2D;
/// [0x2E] ServerSwitch — chọn server/kênh + reconnect 6414 (Shared)
pub const OP_SERVER_SWITCH: u8 = 0x2E;
#[deprecated(note = "renamed to OP_SERVER_SWITCH, see .scratch/opcode-rename/spec.md")]
pub const OP_WATER_WAR: u8 = OP_SERVER_SWITCH;
/// [0x32] BattleCommand (giữ tên) — body là battle-event record, không phải client command
pub const OP_BATTLE_COMMAND: u8 = 0x32;
/// [0x33] BattleView (giữ tên) — áp flag/value lên unit, chưa rõ spectate
pub const OP_BATTLE_VIEW: u8 = 0x33;
/// [0x34] BattleReset — reset 21 slot + snapshot Now + chạy turn engine (S->C)
pub const OP_BATTLE_RESET: u8 = 0x34;
#[deprecated(note = "renamed to OP_BATTLE_RESET, see .scratch/opcode-rename/spec.md")]
pub const OP_MOUNTAIN_THROW: u8 = OP_BATTLE_RESET;
/// [0x35] BattleEventEx — biến cố trận đánh trên battle-record, lưới 4×5 (S->C)
pub const OP_BATTLE_EVENT_EX: u8 = 0x35;
#[deprecated(note = "renamed to OP_BATTLE_EVENT_EX, see .scratch/opcode-rename/spec.md")]
pub const OP_SHIP_SKILL: u8 = OP_BATTLE_EVENT_EX;
/// [0x36] ServerStatus — bảng trạng thái form chọn server, cặp [idx 1-based][lv 0..3] (Shared)
pub const OP_SERVER_STATUS: u8 = 0x36;
#[deprecated(note = "renamed to OP_SERVER_STATUS, see .scratch/opcode-rename/spec.md")]
pub const OP_KEEPALIVE: u8 = OP_SERVER_STATUS;
/// [0x37] CafeId — form TSe_CafeIDForm submit ID (Shared)
pub const OP_CAFE_ID: u8 = 0x37;
#[deprecated(note = "renamed to OP_CAFE_ID, see .scratch/opcode-rename/spec.md")]
pub const OP_STALL: u8 = OP_CAFE_ID;
/// [0x38] Reserved — handler no-op tuyệt đối (đọc SubOp rồi bỏ)
pub const OP_RESERVED_38: u8 = 0x38;
/// [0x39] SportForm — mở/đóng form sport id 1/2/3/4/6/FF + banner (S->C, C->S rỗng)
pub const OP_SPORT_FORM: u8 = 0x39;
#[deprecated(note = "renamed to OP_SPORT_FORM, see .scratch/opcode-rename/spec.md")]
pub const OP_GACHA: u8 = OP_SPORT_FORM;
/// [0x3A] DiceBiDaXiao — kết quả xúc xắc Tài/Xỉu 3 byte c1/c2/c3 + flag + token (Shared)
pub const OP_DICE_BIDAXIAO: u8 = 0x3A;
#[deprecated(note = "renamed to OP_DICE_BIDAXIAO, see .scratch/opcode-rename/spec.md")]
pub const OP_WHEEL: u8 = OP_DICE_BIDAXIAO;
/// [0x3B] Domino — bàn Domino (mode chờ / nạp xúc xắc / set state) (Shared)
pub const OP_DOMINO: u8 = 0x3B;
#[deprecated(note = "renamed to OP_DOMINO, see .scratch/opcode-rename/spec.md")]
pub const OP_FESTIVAL: u8 = OP_DOMINO;
/// [0x3C] ZmChess — Cờ Tướng (đồng bộ bàn/nước đi/trạng thái/đồng hồ) (Shared)
pub const OP_ZM_CHESS: u8 = 0x3C;
#[deprecated(note = "renamed to OP_ZM_CHESS, see .scratch/opcode-rename/spec.md")]
pub const OP_MOUNT: u8 = OP_ZM_CHESS;
/// [0x3D] Lotto — máy Xổ số 5 số 1..42 + kênh message (Shared)
pub const OP_LOTTO: u8 = 0x3D;
#[deprecated(note = "renamed to OP_LOTTO, see .scratch/opcode-rename/spec.md")]
pub const OP_GUILD_WAR: u8 = OP_LOTTO;
/// [0x3E] LifeNotify — 2 SubOp notify cố định theo RP[1] (S->C, C->S không tồn tại)
pub const OP_LIFE_NOTIFY: u8 = 0x3E;
#[deprecated(note = "renamed to OP_LIFE_NOTIFY, see .scratch/opcode-rename/spec.md")]
pub const OP_BLISS_BAG: u8 = OP_LIFE_NOTIFY;
/// [0x3F] ArenaWaterwar — Lôi đài + Thủy chiến (Shared)
pub const OP_ARENA_WATERWAR: u8 = 0x3F;
#[deprecated(note = "renamed to OP_ARENA_WATERWAR, see .scratch/opcode-rename/spec.md")]
pub const OP_OUTFIT: u8 = OP_ARENA_WATERWAR;
/// [0x40] NavalCombat (Hải chiến quy mô lớn)
pub const OP_NAVAL_COMBAT: u8 = 0x40;
/// [0x41] Apparatus — Khí quan on/off/menu 3 slot [Slot][ItemID][Dura] (Shared)
pub const OP_APPARATUS: u8 = 0x41;
#[deprecated(note = "renamed to OP_APPARATUS, see .scratch/opcode-rename/spec.md")]
pub const OP_RANK: u8 = OP_APPARATUS;
/// [0x42] ItemMall — GM/Mall shop (gate GM giữ ở handler)
pub const OP_ITEM_MALL: u8 = 0x42;
#[deprecated(note = "renamed to OP_ITEM_MALL, see .scratch/opcode-rename/spec.md")]
pub const OP_GM_TOOL: u8 = OP_ITEM_MALL;
/// [0x43] Recommend — form Giới thiệu (đổi tên + chat, toast lỗi, close socket) (S->C, C->S rỗng)
pub const OP_RECOMMEND: u8 = 0x43;
#[deprecated(note = "renamed to OP_RECOMMEND, see .scratch/opcode-rename/spec.md")]
pub const OP_HOLE_GAME: u8 = OP_RECOMMEND;
/// [0x44] Reserved — jumptable khuyết 0x44, chưa có handler
pub const OP_RESERVED_44: u8 = 0x44;
#[deprecated(note = "renamed to OP_RESERVED_44, see .scratch/opcode-rename/spec.md")]
pub const OP_CONNECT: u8 = OP_RESERVED_44;
/// [0x45] Child — Hệ thống Con cái (block 93B, 6 slot trang bị, trưởng thành ngưỡng 400) (Shared)
pub const OP_CHILD: u8 = 0x45;
#[deprecated(note = "renamed to OP_CHILD, see .scratch/opcode-rename/spec.md")]
pub const OP_BOAT_SKILL: u8 = OP_CHILD;
/// [0x46] BoatRace — Đua thuyền rồng (sync [CharID][X][Y][state], BXH 10 mục) (Shared)
pub const OP_BOAT_RACE: u8 = 0x46;
#[deprecated(note = "renamed to OP_BOAT_RACE, see .scratch/opcode-rename/spec.md")]
pub const OP_MARK: u8 = OP_BOAT_RACE;
/// [0x47] AntiAddiction (Hệ thống kiểm soát giờ chơi 3h/5h)
pub const OP_ANTI_ADDICTION: u8 = 0x47;
/// [0x48] SlotMachine — Máy Slot (reel 1..16, trao thưởng [UID][ItemID][Count] + broadcast) (Shared)
pub const OP_SLOT_MACHINE: u8 = 0x48;
#[deprecated(note = "renamed to OP_SLOT_MACHINE, see .scratch/opcode-rename/spec.md")]
pub const OP_CITY_EX: u8 = OP_SLOT_MACHINE;
/// [0x49] WorldBoss (Sự kiện Săn Boss Thế Giới toàn cụm)
pub const OP_WORLD_BOSS: u8 = 0x49;
/// [0x4A] NpcUpgrade (Nâng cấp thuộc tính NPC)
pub const OP_NPC_UPGRADE: u8 = 0x4A;
/// [0x4B] SaleRoom (Kỳ Trân Các, Sàn đấu giá & Shop Point)
pub const OP_SALE_ROOM: u8 = 0x4B;
/// [0x4C] ExpSlot (Hũ tích lũy bình kinh nghiệm)
pub const OP_EXP_SLOT: u8 = 0x4C;
/// [0x4D] Activity (Hoạt động hàng ngày: Tháp 2k, 40 NPC)
pub const OP_ACTIVITY: u8 = 0x4D;
/// [0x4E] Astrolabe (Hệ thống Tinh Đồ / Chiêm Tinh Tướng Tinh)
pub const OP_ASTROLABE: u8 = 0x4E;
/// [0xC7] GmAnnounce — GM chat announce (tag GM/thì thầm/Thiên thần + WA0033.wav) (S->C, C->S không tồn tại)
pub const OP_GM_ANNOUNCE: u8 = 0xC7;
#[deprecated(note = "renamed to OP_GM_ANNOUNCE, see .scratch/opcode-rename/spec.md")]
pub const OP_RECONNECT: u8 = OP_GM_ANNOUNCE;

/// All main opcodes handled by the TS Online server (spec/server_main_opcode.md).
pub const SERVER_MAIN_OPCODES: &[u8] = &[
    OP_SYSTEM_ALERT, OP_AUTH, OP_CHAT, OP_LOOK, OP_PLAYER_APPEAR, OP_PLAYER_UPDATE, OP_MOVE,
    OP_TELEPORT, OP_STAT_UPDATE, OP_CREATE_CHAR, OP_BATTLE, OP_RELOCATE, OP_GROUP,
    OP_FRIEND_INVITE, OP_PET, OP_GM_MANAGE, OP_BATTLE_PET, OP_NPC_EVENT, OP_WORLD_OBJECT, OP_ITEM,
    OP_ITEM_INFO, OP_TRADE, OP_MONEY_SYNC, OP_NPC_SHOP, OP_SKILL_CS, OP_BANK, OP_STORAGE,
    OP_PET_HOTEL, OP_EXPRESS, OP_PK_SWITCH, OP_GAME_POINTS, OP_ACCOUNT, OP_JOB_CHANGE,
    OP_LOGIN_COMPLETE, OP_EXP_LEVEL, OP_RANK_ANNOUNCE, OP_HOTKEY, OP_QUEST, OP_RESET,
    OP_COMPOUND, OP_REBORN_PET, OP_REBORN, OP_SERVER_SWITCH, OP_BATTLE_COMMAND, OP_BATTLE_VIEW,
    OP_BATTLE_RESET, OP_BATTLE_EVENT_EX, OP_SERVER_STATUS, OP_CAFE_ID, OP_RESERVED_38, OP_SPORT_FORM, OP_DICE_BIDAXIAO, OP_DOMINO,
    OP_ZM_CHESS, OP_LOTTO, OP_LIFE_NOTIFY, OP_ARENA_WATERWAR, OP_NAVAL_COMBAT, OP_APPARATUS, OP_ITEM_MALL,
    OP_RECOMMEND, OP_RESERVED_44, OP_CHILD, OP_BOAT_RACE, OP_ANTI_ADDICTION, OP_SLOT_MACHINE,
    OP_WORLD_BOSS, OP_NPC_UPGRADE, OP_SALE_ROOM, OP_EXP_SLOT, OP_ACTIVITY, OP_ASTROLABE,
    OP_GM_ANNOUNCE,
];

/// Alias to SERVER_MAIN_OPCODES.
pub const MAIN_OPCODES: &[u8] = SERVER_MAIN_OPCODES;

/// Check whether the given opcode is a documented server main opcode.
pub fn is_main_opcode(opcode: u8) -> bool {
    SERVER_MAIN_OPCODES.contains(&opcode)
}

/// Retrieve the constant name of a main opcode.
pub fn opcode_name(opcode: u8) -> &'static str {
    match opcode {
        OP_SYSTEM_ALERT => "OP_SYSTEM_ALERT",
        OP_AUTH => "OP_AUTH",
        OP_CHAT => "OP_CHAT",
        OP_LOOK => "OP_LOOK",
        OP_PLAYER_APPEAR => "OP_PLAYER_APPEAR",
        OP_PLAYER_UPDATE => "OP_PLAYER_UPDATE",
        OP_MOVE => "OP_MOVE",
        OP_TELEPORT => "OP_TELEPORT",
        OP_STAT_UPDATE => "OP_STAT_UPDATE",
        OP_CREATE_CHAR => "OP_CREATE_CHAR",
        OP_BATTLE => "OP_BATTLE",
        OP_RELOCATE => "OP_RELOCATE",
        OP_GROUP => "OP_GROUP",
        OP_FRIEND_INVITE => "OP_FRIEND_INVITE",
        OP_PET => "OP_PET",
        OP_GM_MANAGE => "OP_GM_MANAGE",
        OP_BATTLE_PET => "OP_BATTLE_PET",
        OP_NPC_EVENT => "OP_NPC_EVENT",
        OP_WORLD_OBJECT => "OP_WORLD_OBJECT",
        OP_ITEM => "OP_ITEM",
        OP_ITEM_INFO => "OP_ITEM_INFO",
        OP_TRADE => "OP_TRADE",
        OP_MONEY_SYNC => "OP_MONEY_SYNC",
        OP_NPC_SHOP => "OP_NPC_SHOP",
        OP_SKILL_CS => "OP_SKILL_CS",
        OP_BANK => "OP_BANK",
        OP_STORAGE => "OP_STORAGE",
        OP_PET_HOTEL => "OP_PET_HOTEL",
        OP_EXPRESS => "OP_EXPRESS",
        OP_PK_SWITCH => "OP_PK_SWITCH",
        OP_GAME_POINTS => "OP_GAME_POINTS",
        OP_ACCOUNT => "OP_ACCOUNT",
        OP_JOB_CHANGE => "OP_JOB_CHANGE",
        OP_LOGIN_COMPLETE => "OP_LOGIN_COMPLETE",
        OP_EXP_LEVEL => "OP_EXP_LEVEL",
        OP_RANK_ANNOUNCE => "OP_RANK_ANNOUNCE",
        OP_HOTKEY => "OP_HOTKEY",
        OP_QUEST => "OP_QUEST",
        OP_RESET => "OP_RESET",
        OP_COMPOUND => "OP_COMPOUND",
        OP_REBORN_PET => "OP_REBORN_PET",
        OP_REBORN => "OP_REBORN",
        OP_SERVER_SWITCH => "OP_SERVER_SWITCH",
        OP_BATTLE_COMMAND => "OP_BATTLE_COMMAND",
        OP_BATTLE_VIEW => "OP_BATTLE_VIEW",
        OP_BATTLE_RESET => "OP_BATTLE_RESET",
        OP_BATTLE_EVENT_EX => "OP_BATTLE_EVENT_EX",
        OP_SERVER_STATUS => "OP_SERVER_STATUS",
        OP_CAFE_ID => "OP_CAFE_ID",
        OP_RESERVED_38 => "OP_RESERVED_38",
        OP_SPORT_FORM => "OP_SPORT_FORM",
        OP_DICE_BIDAXIAO => "OP_DICE_BIDAXIAO",
        OP_DOMINO => "OP_DOMINO",
        OP_ZM_CHESS => "OP_ZM_CHESS",
        OP_LOTTO => "OP_LOTTO",
        OP_LIFE_NOTIFY => "OP_LIFE_NOTIFY",
        OP_ARENA_WATERWAR => "OP_ARENA_WATERWAR",
        OP_NAVAL_COMBAT => "OP_NAVAL_COMBAT",
        OP_APPARATUS => "OP_APPARATUS",
        OP_ITEM_MALL => "OP_ITEM_MALL",
        OP_RECOMMEND => "OP_RECOMMEND",
        OP_RESERVED_44 => "OP_RESERVED_44",
        OP_CHILD => "OP_CHILD",
        OP_BOAT_RACE => "OP_BOAT_RACE",
        OP_ANTI_ADDICTION => "OP_ANTI_ADDICTION",
        OP_SLOT_MACHINE => "OP_SLOT_MACHINE",
        OP_WORLD_BOSS => "OP_WORLD_BOSS",
        OP_NPC_UPGRADE => "OP_NPC_UPGRADE",
        OP_SALE_ROOM => "OP_SALE_ROOM",
        OP_EXP_SLOT => "OP_EXP_SLOT",
        OP_ACTIVITY => "OP_ACTIVITY",
        OP_ASTROLABE => "OP_ASTROLABE",
        OP_GM_ANNOUNCE => "OP_GM_ANNOUNCE",
        _ => "OP_UNKNOWN",
    }
}

pub mod codec;
pub mod codecs;
pub mod encoder;
pub mod frame;
pub mod reader;
pub mod system_alert;
pub mod writer;

pub use codecs::{
    ehuman, BattleRoleData, BattleRoleSerializer, FriendExtra, NpcTalkCodec, PlayerCard,
    PlayerInfoCodec, QuestDontEntry, QuestSyncCodec, QuestTaskEntry, TalkLockMode, ThingData,
    ThingDataCodec, FRIEND_EXTRA_SIZE, THING_DATA_SIZE,
};
pub use reader::PacketReader;
pub use system_alert::{SystemAlert, SystemAlertReason};
pub use writer::PacketWriter;

/// Build an outgoing frame: `F444` + LE16(len) + `code` + `body`, where `len`
/// counts every byte after the 4-byte header (i.e. `code` + `body`).
///
/// This is the single place where the frame header and length are computed.
/// Every outgoing packet is built through it (directly, or via a named builder)
/// so the seam between business logic and wire format stays in one module.
pub fn frame(code: &str, body: &str) -> String {
    let total_len = (code.len() + body.len()) / 2;
    format!("F444{}{code}{body}", encoder::le16(total_len as u16))
}
