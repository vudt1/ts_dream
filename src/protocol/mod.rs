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

/// [0x00] System warning messages 
pub const OP_SYSTEM: u8 = 0x00;
/// [0x01] Auth / Login / Logout / Kick / ServerList 
pub const OP_AUTH: u8 = 0x01;
/// [0x02] Chat (Normal, Team, Guild, World, Whisper) 
pub const OP_CHAT: u8 = 0x02;
/// [0x03] Look / Inspect Target Player / NPC 
pub const OP_LOOK: u8 = 0x03;
/// [0x04] PlayerAppear in FOV 
pub const OP_PLAYER_APPEAR: u8 = 0x04;
/// [0x05] PlayerUpdate  / MoveConfirm 
pub const OP_PLAYER_UPDATE: u8 = 0x05;
/// [0x06] Move (Tọa độ di chuyển ô gạch Isometric) 
pub const OP_MOVE: u8 = 0x06;
/// [0x07] PlayerDetail (Thuộc tính chi tiết nhân vật)
pub const OP_PLAYER_DETAIL: u8 = 0x07;
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
/// [0x0E] Mail (Hòm thư bưu điện) 
pub const OP_MAIL: u8 = 0x0E;
/// [0x0F] Pet / Võ tướng đồng hành & Lòng trung thành FAI 
pub const OP_PET: u8 = 0x0F;
/// [0x10] NpcManage / Xuất hiện quái & đối thoại NPC 
pub const OP_NPC_MANAGE: u8 = 0x10;
/// [0x13] BattlePet (Chiến thuật & Điều khiển võ tướng trong trận)
pub const OP_BATTLE_PET: u8 = 0x13;
/// [0x14] Action (Động tác: Ngồi, Đứng, Quỳ, Cảm xúc) 
pub const OP_ACTION: u8 = 0x14;
/// [0x16] Skill (Kỹ năng & Thần thông Quang - Ám từ Server gửi về)
pub const OP_SKILL_SC: u8 = 0x16;
/// [0x17] Item & Inventory (Túi đồ, Trang bị - Chuẩn 112 Sub-Ops)
pub const OP_ITEM: u8 = 0x17;
/// [0x18] ItemInfo (Thông tin thuộc tính chi tiết món đồ) 
pub const OP_ITEM_INFO: u8 = 0x18;
/// [0x19] SceneManage / Map (Quản lý bản đồ, thời tiết, BGM) 
pub const OP_SCENE_MANAGE: u8 = 0x19;
/// [0x1A] Talk (Đối thoại NPC & Cây kịch bản hội thoại) 
pub const OP_TALK: u8 = 0x1A;
/// [0x1B] Trade (Giao dịch trực tiếp người chơi) 
pub const OP_TRADE: u8 = 0x1B;
/// [0x1C] Skill (Client gửi yêu cầu thi triển/học kỹ năng lên Server)
pub const OP_SKILL_CS: u8 = 0x1C;
/// [0x1D] Bank (Tiền trang / Gửi rút ngân lượng) 
pub const OP_BANK: u8 = 0x1D;
/// [0x1E] Storage (Rương Nhà Trọ & Ba Đậu Yêu Balo) 
pub const OP_STORAGE: u8 = 0x1E;
/// [0x1F] NpcShop (Cửa hàng thương nhân NPC) 
pub const OP_NPC_SHOP: u8 = 0x1F;
/// [0x20] Express (Chuyển phát nhanh bưu kiện) 
pub const OP_EXPRESS: u8 = 0x20;
/// [0x21] Welcome / MOTD Thông báo chào mừng máy chủ 
pub const OP_WELCOME: u8 = 0x21;
/// [0x22] GamePoints (Điểm nạp thẻ / Tích lũy tài khoản) 
pub const OP_GAME_POINTS: u8 = 0x22;
/// [0x23] Guild (Hệ thống Quân đoàn) 
pub const OP_GUILD: u8 = 0x23;
/// [0x24] GuildInfo (Danh sách thành viên & Kiến trúc quân đoàn)
pub const OP_GUILD_INFO: u8 = 0x24;
/// [0x25] GuildAction (Thao tác gia nhập, trục xuất, bổ nhiệm)
pub const OP_GUILD_ACTION: u8 = 0x25;
/// [0x26] GuildBattle (Chiến tranh giữa các quân đoàn)
pub const OP_GUILD_BATTLE: u8 = 0x26;
/// [0x27] SystemMaster (Hệ thống điều hành quản trị viên)
pub const OP_SYSTEM_MASTER: u8 = 0x27;
/// [0x28] Hotkey (Cấu hình phím tắt nhanh QuickBar F1..F8)
pub const OP_HOTKEY: u8 = 0x28;
/// [0x29] Quest (Nhiệm vụ & Tiến trình kịch bản cốt truyện)
pub const OP_QUEST: u8 = 0x29;
/// [0x2A] Friend (Danh sách Hảo hữu & Sổ liên lạc bạn bè)
pub const OP_FRIEND: u8 = 0x2A;
/// [0x2B] Compound (Luyện đồ Mix Item & Tinh luyện khoáng thạch)
pub const OP_COMPOUND: u8 = 0x2B;
/// [0x2C] RebornPet (Chuyển sinh Võ tướng: RB1, RB2, RB3)
pub const OP_REBORN_PET: u8 = 0x2C;
/// [0x2D] Reborn (Chuyển sinh nhân vật: Chuyển Sinh CS, Trùng Sinh TS)
pub const OP_REBORN: u8 = 0x2D;
/// [0x2E] WaterWar (Thủy chiến Xích Bích) 
pub const OP_WATER_WAR: u8 = 0x2E;
/// [0x32] BattleCommand (Mệnh lệnh điều khiển từng lượt đánh)
pub const OP_BATTLE_COMMAND: u8 = 0x32;
/// [0x33] BattleView (Chế độ xem trận đấu / Khán giả theo dõi)
pub const OP_BATTLE_VIEW: u8 = 0x33;
/// [0x34] MountainThrow (Ném đá / Công cụ ném sơn trại)
pub const OP_MOUNTAIN_THROW: u8 = 0x34;
/// [0x35] ShipSkill (Kỹ năng chiến hạm đường thủy)
pub const OP_SHIP_SKILL: u8 = 0x35;
/// [0x36] Keepalive (Nhịp tim Heartbeat Ping-Pong duy trì kết nối)
pub const OP_KEEPALIVE: u8 = 0x36;
/// [0x37] Stall (Bày bán vỉa hè người chơi) 
pub const OP_STALL: u8 = 0x37;
/// [0x39] Gacha (Quay tướng & Hòm may mắn)
pub const OP_GACHA: u8 = 0x39;
/// [0x3A] Wheel (Vòng quay may mắn kỳ bí)
pub const OP_WHEEL: u8 = 0x3A;
/// [0x3B] Festival (Sự kiện lễ hội)
pub const OP_FESTIVAL: u8 = 0x3B;
/// [0x3C] Mount (Hệ thống Thú Cưỡi & Tốc độ di chuyển)
pub const OP_MOUNT: u8 = 0x3C;
/// [0x3D] GuildWar (Sự kiện Công Thành Chiến 3 cổng thành)
pub const OP_GUILD_WAR: u8 = 0x3D;
/// [0x3E] BlissBag (Túi phúc & Hệ thống Giftcode kích hoạt quà)
pub const OP_BLISS_BAG: u8 = 0x3E;
/// [0x3F] Outfit (Thời trang, Ngoại trang & Cánh phi phong)
pub const OP_OUTFIT: u8 = 0x3F;
/// [0x40] NavalCombat (Hải chiến quy mô lớn)
pub const OP_NAVAL_COMBAT: u8 = 0x40;
/// [0x41] Rank (Bảng xếp hạng Cấp độ, Quân đoàn, PK)
pub const OP_RANK: u8 = 0x41;
/// [0x42] GmTool (Công cụ quản trị Game Master)
pub const OP_GM_TOOL: u8 = 0x42;
/// [0x43] HoleGame (Mini game đào lỗ bắt chuột)
pub const OP_HOLE_GAME: u8 = 0x43;
/// [0x44] Connect (Duy trì kênh truyền thông tin)
pub const OP_CONNECT: u8 = 0x44;
/// [0x45] BoatSkill (Kỹ năng thuyền chiến)
pub const OP_BOAT_SKILL: u8 = 0x45;
/// [0x46] Mark (Cờ hiệu & Đánh dấu tọa độ bản đồ)
pub const OP_MARK: u8 = 0x46;
/// [0x47] AntiAddiction (Hệ thống kiểm soát giờ chơi 3h/5h)
pub const OP_ANTI_ADDICTION: u8 = 0x47;
/// [0x48] CityEx (Mở rộng thành trì & Bang hội lãnh địa)
pub const OP_CITY_EX: u8 = 0x48;
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
/// [0xC7] Reconnect (Tái kết nối phiên làm việc khi rớt mạng)
pub const OP_RECONNECT: u8 = 0xC7;

/// All main opcodes handled by the TS Online server (spec/server_main_opcode.md).
pub const SERVER_MAIN_OPCODES: &[u8] = &[
    OP_SYSTEM, OP_AUTH, OP_CHAT, OP_LOOK, OP_PLAYER_APPEAR, OP_PLAYER_UPDATE, OP_MOVE,
    OP_PLAYER_DETAIL, OP_STAT_UPDATE, OP_CREATE_CHAR, OP_BATTLE, OP_RELOCATE, OP_GROUP,
    OP_MAIL, OP_PET, OP_NPC_MANAGE, OP_BATTLE_PET, OP_ACTION, OP_SKILL_SC, OP_ITEM,
    OP_ITEM_INFO, OP_SCENE_MANAGE, OP_TALK, OP_TRADE, OP_SKILL_CS, OP_BANK, OP_STORAGE,
    OP_NPC_SHOP, OP_EXPRESS, OP_WELCOME, OP_GAME_POINTS, OP_GUILD, OP_GUILD_INFO,
    OP_GUILD_ACTION, OP_GUILD_BATTLE, OP_SYSTEM_MASTER, OP_HOTKEY, OP_QUEST, OP_FRIEND,
    OP_COMPOUND, OP_REBORN_PET, OP_REBORN, OP_WATER_WAR, OP_BATTLE_COMMAND, OP_BATTLE_VIEW,
    OP_MOUNTAIN_THROW, OP_SHIP_SKILL, OP_KEEPALIVE, OP_STALL, OP_GACHA, OP_WHEEL, OP_FESTIVAL,
    OP_MOUNT, OP_GUILD_WAR, OP_BLISS_BAG, OP_OUTFIT, OP_NAVAL_COMBAT, OP_RANK, OP_GM_TOOL,
    OP_HOLE_GAME, OP_CONNECT, OP_BOAT_SKILL, OP_MARK, OP_ANTI_ADDICTION, OP_CITY_EX,
    OP_WORLD_BOSS, OP_NPC_UPGRADE, OP_SALE_ROOM, OP_EXP_SLOT, OP_ACTIVITY, OP_ASTROLABE,
    OP_RECONNECT,
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
        OP_SYSTEM => "OP_SYSTEM",
        OP_AUTH => "OP_AUTH",
        OP_CHAT => "OP_CHAT",
        OP_LOOK => "OP_LOOK",
        OP_PLAYER_APPEAR => "OP_PLAYER_APPEAR",
        OP_PLAYER_UPDATE => "OP_PLAYER_UPDATE",
        OP_MOVE => "OP_MOVE",
        OP_PLAYER_DETAIL => "OP_PLAYER_DETAIL",
        OP_STAT_UPDATE => "OP_STAT_UPDATE",
        OP_CREATE_CHAR => "OP_CREATE_CHAR",
        OP_BATTLE => "OP_BATTLE",
        OP_RELOCATE => "OP_RELOCATE",
        OP_GROUP => "OP_GROUP",
        OP_MAIL => "OP_MAIL",
        OP_PET => "OP_PET",
        OP_NPC_MANAGE => "OP_NPC_MANAGE",
        OP_BATTLE_PET => "OP_BATTLE_PET",
        OP_ACTION => "OP_ACTION",
        OP_SKILL_SC => "OP_SKILL_SC",
        OP_ITEM => "OP_ITEM",
        OP_ITEM_INFO => "OP_ITEM_INFO",
        OP_SCENE_MANAGE => "OP_SCENE_MANAGE",
        OP_TALK => "OP_TALK",
        OP_TRADE => "OP_TRADE",
        OP_SKILL_CS => "OP_SKILL_CS",
        OP_BANK => "OP_BANK",
        OP_STORAGE => "OP_STORAGE",
        OP_NPC_SHOP => "OP_NPC_SHOP",
        OP_EXPRESS => "OP_EXPRESS",
        OP_WELCOME => "OP_WELCOME",
        OP_GAME_POINTS => "OP_GAME_POINTS",
        OP_GUILD => "OP_GUILD",
        OP_GUILD_INFO => "OP_GUILD_INFO",
        OP_GUILD_ACTION => "OP_GUILD_ACTION",
        OP_GUILD_BATTLE => "OP_GUILD_BATTLE",
        OP_SYSTEM_MASTER => "OP_SYSTEM_MASTER",
        OP_HOTKEY => "OP_HOTKEY",
        OP_QUEST => "OP_QUEST",
        OP_FRIEND => "OP_FRIEND",
        OP_COMPOUND => "OP_COMPOUND",
        OP_REBORN_PET => "OP_REBORN_PET",
        OP_REBORN => "OP_REBORN",
        OP_WATER_WAR => "OP_WATER_WAR",
        OP_BATTLE_COMMAND => "OP_BATTLE_COMMAND",
        OP_BATTLE_VIEW => "OP_BATTLE_VIEW",
        OP_MOUNTAIN_THROW => "OP_MOUNTAIN_THROW",
        OP_SHIP_SKILL => "OP_SHIP_SKILL",
        OP_KEEPALIVE => "OP_KEEPALIVE",
        OP_STALL => "OP_STALL",
        OP_GACHA => "OP_GACHA",
        OP_WHEEL => "OP_WHEEL",
        OP_FESTIVAL => "OP_FESTIVAL",
        OP_MOUNT => "OP_MOUNT",
        OP_GUILD_WAR => "OP_GUILD_WAR",
        OP_BLISS_BAG => "OP_BLISS_BAG",
        OP_OUTFIT => "OP_OUTFIT",
        OP_NAVAL_COMBAT => "OP_NAVAL_COMBAT",
        OP_RANK => "OP_RANK",
        OP_GM_TOOL => "OP_GM_TOOL",
        OP_HOLE_GAME => "OP_HOLE_GAME",
        OP_CONNECT => "OP_CONNECT",
        OP_BOAT_SKILL => "OP_BOAT_SKILL",
        OP_MARK => "OP_MARK",
        OP_ANTI_ADDICTION => "OP_ANTI_ADDICTION",
        OP_CITY_EX => "OP_CITY_EX",
        OP_WORLD_BOSS => "OP_WORLD_BOSS",
        OP_NPC_UPGRADE => "OP_NPC_UPGRADE",
        OP_SALE_ROOM => "OP_SALE_ROOM",
        OP_EXP_SLOT => "OP_EXP_SLOT",
        OP_ACTIVITY => "OP_ACTIVITY",
        OP_ASTROLABE => "OP_ASTROLABE",
        OP_RECONNECT => "OP_RECONNECT",
        _ => "OP_UNKNOWN",
    }
}

pub mod codec;
pub mod codecs;
pub mod encoder;
pub mod frame;
pub mod reader;
pub mod writer;

pub use codecs::{
    ehuman, BattleRoleData, BattleRoleSerializer, FriendExtra, PlayerCard, PlayerInfoCodec,
    ThingData, ThingDataCodec, FRIEND_EXTRA_SIZE, THING_DATA_SIZE,
};
pub use reader::PacketReader;
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
