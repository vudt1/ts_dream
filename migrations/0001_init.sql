-- ============================================================================
-- TS Dream — SQLite Database Schema (Migration 0001)
-- Converted from 0001_init_mysql.sql with SQLite 3 standard conventions.
-- Contains full domain column comments for future coding agents.
-- ============================================================================

-- ============================================================================
-- accounts — Quản lý tài khoản đăng nhập người chơi.
-- Tạo độc quyền qua Web Admin Dashboard. Mật khẩu lưu plaintext (chuẩn C# server).
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounts (
    playerid         INTEGER PRIMARY KEY AUTOINCREMENT,                 -- Mã ID tài khoản định danh duy nhất (bắt đầu từ 300000)
    pass1            TEXT NOT NULL,                                     -- Mật khẩu chính đăng nhập vào game (8..10 ký tự ASCII)
    pass2            TEXT NOT NULL,                                     -- Mật khẩu cấp 2 dùng xác nhận đổi mật khẩu / xóa nhân vật (op 0x23)
    issuspended      INTEGER NOT NULL DEFAULT 0,                        -- Cờ khóa tài khoản: 0 = Bình thường, 1 = Đang bị đình chỉ/khóa
    suspensionreason TEXT NULL,                                         -- Lý do khóa tài khoản (nếu bị khóa)
    suspendeduntil   INTEGER NULL,                                      -- Thời điểm hết hạn khóa tài khoản (timestamp unix; NULL nếu vô thời hạn)
    lastlogin_at     INTEGER NULL,                                      -- Thời điểm đăng nhập gần nhất (timestamp unix)
    createdat        INTEGER NOT NULL,                                  -- Thời điểm khởi tạo tài khoản (timestamp unix)
    updatedat        INTEGER NULL,                                      -- Thời điểm cập nhật thông tin tài khoản gần nhất (timestamp unix)
    lastloginip      TEXT NULL,                                         -- Địa chỉ IP của phiên đăng nhập gần nhất
    gmlevel          INTEGER NOT NULL DEFAULT 0 CHECK (gmlevel >= 0)    -- Cấp bậc quyền quản trị viên: 0 = Người chơi thường, >= 1 = GM/Admin
);

CREATE INDEX IF NOT EXISTS accounts_suspended_idx ON accounts (issuspended, suspendeduntil);
CREATE INDEX IF NOT EXISTS accounts_gm_level_idx ON accounts (gmlevel);

-- Khởi tạo giá trị tự tăng ban đầu để tài khoản đầu tiên tạo ra sẽ có playerid = 300000
INSERT OR IGNORE INTO sqlite_sequence (name, seq) VALUES ('accounts', 299999);

-- ============================================================================
-- characters — Bảng chứa dữ liệu nhân vật người chơi (Quan hệ 1:1 với accounts).
-- Mỗi tài khoản TS Online chỉ chứa duy nhất 1 nhân vật đại diện.
-- ============================================================================
CREATE TABLE IF NOT EXISTS characters (
    playerid    INTEGER NOT NULL PRIMARY KEY,   -- Mã ID định danh nhân vật (shared PK 1:1 với accounts.playerid)
    name        BLOB NOT NULL UNIQUE,           -- Tên nhân vật (dữ liệu byte mã VISCII 1.1 nguyên bản, tối đa 16 bytes)
    level       INTEGER DEFAULT 1,              -- Đẳng cấp nhân vật (1..200)
    gender      INTEGER DEFAULT 0,              -- Giới tính nhân vật: 0 = Nữ, 1 = Nam
    hair        INTEGER DEFAULT 0,              -- Kiểu và màu sắc tóc của nhân vật (bao gồm color1/color2 từ packet tạo nhân vật)
    element     INTEGER DEFAULT 0,              -- Hệ nguyên tố: 1=Địa, 2=Thủy, 3=Hỏa, 4=Phong, 5=Quang, 6=Ám, 7=Tâm
    rebornstage INTEGER DEFAULT 0,              -- Giai đoạn chuyển sinh: 0=Chưa, 1=Chuyển sinh (CS), 2=Tái sinh (TS)
    curhp       INTEGER DEFAULT 0,              -- Sinh lực (Máu / HP) hiện tại của nhân vật
    maxhp       INTEGER DEFAULT 0,              -- Sinh lực (Máu / HP) tối đa của nhân vật
    cursp       INTEGER DEFAULT 0,              -- Nội lực (Mana / SP) hiện tại của nhân vật
    maxsp       INTEGER DEFAULT 0,              -- Nội lực (Mana / SP) tối đa của nhân vật
    curexp      INTEGER DEFAULT 0,              -- Điểm kinh nghiệm tích lũy hiện tại trong cấp độ
    nextexp     INTEGER DEFAULT 0,              -- Điểm kinh nghiệm cần đạt để thăng cấp độ tiếp theo
    freepoints  INTEGER DEFAULT 0,              -- Điểm tiềm năng khả dụng chưa phân bổ (stat points)
    skillpoint  INTEGER DEFAULT 0,              -- Điểm kỹ năng khả dụng chưa phân bổ (skill points)
    baseatk     INTEGER DEFAULT 0,              -- Điểm tấn công vật lý cơ bản (ATK gốc)
    baseint     INTEGER DEFAULT 0,              -- Điểm trí lực cơ bản (INT gốc)
    basedef     INTEGER DEFAULT 0,              -- Điểm phòng thủ cơ bản (DEF gốc)
    basehpx     INTEGER DEFAULT 0,              -- Điểm tiềm năng tăng máu gốc (HPX gốc)
    basespx     INTEGER DEFAULT 0,              -- Điểm tiềm năng tăng mana gốc (SPX gốc)
    baseagi     INTEGER DEFAULT 0,              -- Điểm nhanh nhẹn cơ bản (AGI gốc - quyết định thứ tự lượt đánh)
    equipatk    INTEGER DEFAULT 0,              -- Điểm tấn công vật lý cộng thêm từ trang bị mang trên người
    equipdef    INTEGER DEFAULT 0,              -- Điểm phòng ngự cộng thêm từ trang bị mang trên người
    equipint    INTEGER DEFAULT 0,              -- Điểm trí lực cộng thêm từ trang bị mang trên người
    equipagi    INTEGER DEFAULT 0,              -- Điểm nhanh nhẹn cộng thêm từ trang bị mang trên người
    equiphpx    INTEGER DEFAULT 0,              -- Máu cộng thêm từ trang bị mang trên người
    equipspx    INTEGER DEFAULT 0,              -- Mana cộng thêm từ trang bị mang trên người
    fai         INTEGER DEFAULT 0,              -- Điểm trung thành / Tâm tính người chơi (0..100)
    pk          INTEGER DEFAULT 0,              -- Điểm sát khí / Tội ác PK khi giết người chơi khác
    mapid       INTEGER DEFAULT 0,              -- Mã bản đồ hiện tại nhân vật đang đứng (theo Warp.Dat / CityEx.Dat)
    mapx        INTEGER DEFAULT 0,              -- Tọa độ trục X của nhân vật trên bản đồ hiện tại
    mapy        INTEGER DEFAULT 0,              -- Tọa độ trục Y của nhân vật trên bản đồ hiện tại
    jobtype     INTEGER DEFAULT 0,              -- Nghề nghiệp: 0=Dân thường, 1=Hiệp sĩ, 2=Nho gia, 3=Hiền triết, 4=Bá vương
    newbie      INTEGER DEFAULT 0               -- Đánh dấu tân thủ nhận quà đặc biệt (0 = Chưa nhận, 1 = Đã nhận)
);

-- ============================================================================
-- character_money — Sổ cái quản lý tiền tệ của nhân vật (Tách riêng khỏi characters
-- để tối ưu lock khi giao dịch, mua bán shop, chuyển khoản ngân hàng).
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_money (
    playerid  INTEGER PRIMARY KEY,  -- Mã ID nhân vật người chơi (shared PK 1:1)
    gold      INTEGER DEFAULT 0,    -- Lượng tiền vàng (Gold) mang theo trong hành trang người chơi
    bankgold  INTEGER DEFAULT 0,    -- Lượng tiền vàng gửi trong Tiền trang / Ngân hàng
    shoppoint INTEGER DEFAULT 0     -- Điểm tích lũy / nạp dùng giao dịch tại Kỳ Trân Các / Shop đặc biệt
);

-- ============================================================================
-- inventories — Quản lý toàn bộ vật phẩm trong tất cả các loại túi / rương chứa.
-- storagetype: 1=Hành trang chính (Bag), 2=Tiền trang (Bank), 4=Túi phụ (Secondary),
--              8=Trang bị trên người (Equip), 16=Hòm đồ lưu lãng (Warehouse).
-- Các cột ánh xạ khớp 1:1 với struct ThingData 35-byte giao thức wire (Little-Endian).
-- ============================================================================
CREATE TABLE IF NOT EXISTS inventories (
    playerid      INTEGER NOT NULL,     -- Mã ID nhân vật sở hữu vật phẩm
    storagetype   INTEGER NOT NULL,     -- Loại kho chứa (1: HomDo, 2: TienTrang, 4: TuiDeo, 8: TrangBi, 16: LuuLang)
    slot          INTEGER NOT NULL,     -- Vị trí ô chứa trong túi/hòm đồ (1..25 hoặc 1..50 tùy loại túi)
    itemid        INTEGER DEFAULT 0,    -- Mã ID định danh vật phẩm theo Item.dat (0 = ô trống)
    quantity      INTEGER DEFAULT 0,    -- Số lượng vật phẩm trong ô (chồng vật phẩm / stack count)
    damage        INTEGER DEFAULT 0,    -- Độ hao mòn / Độ bền hiện tại của trang bị
    element       INTEGER DEFAULT 0,    -- Thuộc tính nguyên tố của trang bị (1=Địa, 2=Thủy, 3=Hỏa, 4=Phong...)
    elementvalue  INTEGER DEFAULT 0,    -- Giá trị thuộc tính nguyên tố cộng thêm
    proofkind     INTEGER DEFAULT 0,    -- Loại bùa hộ mệnh / loại chứng nhận bảo hộ trang bị
    growlevel     INTEGER DEFAULT 0,    -- Cấp độ trưởng thành / phát triển của trang bị
    growexp       INTEGER DEFAULT 0,    -- Điểm kinh nghiệm tích lũy phát triển của trang bị
    specialkind   INTEGER DEFAULT 0,    -- Thuộc tính / hiệu ứng đặc biệt của trang bị
    stoneattr     INTEGER DEFAULT 0,    -- Thuộc tính loại ngọc / đá đã khảm nạm vào trang bị
    stonelevel    INTEGER DEFAULT 0,    -- Cấp độ ngọc / đá đã khảm nạm vào trang bị
    enhancelevel  INTEGER DEFAULT 0,    -- Cấp độ tinh luyện / cường hóa trang bị
    deletetime    REAL DEFAULT 0,       -- Thời hạn sử dụng / thời điểm vật phẩm tự hủy (timestamp unix; 0 = vĩnh viễn)
    damageditemid INTEGER DEFAULT 0,    -- Mã ID vật phẩm phế liệu biến đổi thành khi trang bị bị hỏng vỡ
    islocked      INTEGER DEFAULT 0,    -- Trạng thái khóa an toàn trang bị: 0 = Mở khóa, 1 = Đã khóa bảo vệ
    reinforced    INTEGER DEFAULT 0,    -- Cấp độ gia cố / tăng viện của trang bị
    affix1        INTEGER DEFAULT 0,    -- Thuộc tính bổ sung / dòng phụ thứ 1
    affix2        INTEGER DEFAULT 0,    -- Thuộc tính bổ sung / dòng phụ thứ 2
    affix3        INTEGER DEFAULT 0,    -- Thuộc tính bổ sung / dòng phụ thứ 3
    stylelevel    INTEGER DEFAULT 0,    -- Cấp độ thời trang / hiển thị ngoại trang
    PRIMARY KEY (playerid, storagetype, slot)
);

CREATE INDEX IF NOT EXISTS inventories_item ON inventories (playerid, itemid);

-- ============================================================================
-- character_pets — Quản lý danh sách Võ Tướng / Thú Nuôi (Pet) của người chơi.
-- storagetype: 1=Mang theo (Carried 1..4), 2=Xe ngựa (Cart), 
--              3=Nhà nghỉ (Hotel), 4=Kho quân doanh (Warehouse).
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_pets (
    playerid     INTEGER NOT NULL,      -- Mã ID nhân vật sở hữu võ tướng
    storagetype  INTEGER NOT NULL,      -- Vị trí lưu trữ võ tướng: 1=Mang theo, 2=Xe ngựa, 3=Nhà nghỉ, 4=Kho
    slot         INTEGER NOT NULL,      -- Vị trí ô chứa võ tướng trong danh sách tương ứng (1..4 hoặc mở rộng)
    petid        INTEGER DEFAULT 0,     -- Mã ID nguyên mẫu NPC gốc trong Npc.dat (0 = ô trống)
    name         BLOB,                  -- Tên riêng của Võ Tướng / Pet (dữ liệu byte mã VISCII 1.1)
    level        INTEGER DEFAULT 1,     -- Đẳng cấp hiện tại của Võ Tướng (1..200)
    element      INTEGER DEFAULT 0,     -- Hệ nguyên tố: 1=Địa, 2=Thủy, 3=Hỏa, 4=Phong, 5=Quang, 6=Ám
    rebornstage  INTEGER DEFAULT 0,     -- Cấp độ chuyển sinh võ tướng: 0=Thường, 1=Tái sinh 1 (RB1), 2=Tái sinh 2 (RB2)
    curhp        INTEGER DEFAULT 0,     -- Sinh lực (Máu / HP) hiện tại của võ tướng
    maxhp        INTEGER DEFAULT 0,     -- Sinh lực (Máu / HP) tối đa của võ tướng
    cursp        INTEGER DEFAULT 0,     -- Nội lực (Mana / SP) hiện tại của võ tướng
    maxsp        INTEGER DEFAULT 0,     -- Nội lực (Mana / SP) tối đa của võ tướng
    curexp       INTEGER DEFAULT 0,     -- Điểm kinh nghiệm tích lũy hiện tại của võ tướng
    nextexp      INTEGER DEFAULT 0,     -- Điểm kinh nghiệm cần đạt để thăng cấp võ tướng tiếp theo
    baseatk      INTEGER DEFAULT 0,     -- Điểm tấn công vật lý cơ bản (ATK gốc) của võ tướng
    baseint      INTEGER DEFAULT 0,     -- Điểm trí lực cơ bản (INT gốc) của võ tướng
    basedef      INTEGER DEFAULT 0,     -- Điểm phòng thủ cơ bản (DEF gốc) của võ tướng
    basehpx      INTEGER DEFAULT 0,     -- Điểm tiềm năng tăng máu gốc (HPX) của võ tướng
    basespx      INTEGER DEFAULT 0,     -- Điểm tiềm năng tăng mana gốc (SPX) của võ tướng
    baseagi      INTEGER DEFAULT 0,     -- Điểm nhanh nhẹn cơ bản (AGI gốc) của võ tướng
    equipatk     INTEGER DEFAULT 0,     -- Tấn công cộng thêm từ trang bị võ tướng mang
    equipdef     INTEGER DEFAULT 0,     -- Phòng thủ cộng thêm từ trang bị võ tướng mang
    equipint     INTEGER DEFAULT 0,     -- Trí lực cộng thêm từ trang bị võ tướng mang
    equipagi     INTEGER DEFAULT 0,     -- Nhanh nhẹn cộng thêm từ trang bị võ tướng mang
    equiphpx     INTEGER DEFAULT 0,     -- Máu cộng thêm từ trang bị võ tướng mang
    equipspx     INTEGER DEFAULT 0,     -- Mana cộng thêm từ trang bị võ tướng mang
    fai          INTEGER DEFAULT 0,     -- Điểm trung thành / Thân mật của tướng (0..100; <60 có nguy cơ bỏ trốn)
    skillpoint   INTEGER DEFAULT 0,     -- Điểm kỹ năng khả dụng chưa phân bổ của võ tướng
    skill1_id    INTEGER DEFAULT 0,     -- Mã ID kỹ năng chiến đấu thứ 1
    skill1_level INTEGER DEFAULT 0,     -- Cấp độ kỹ năng thứ 1
    skill2_id    INTEGER DEFAULT 0,     -- Mã ID kỹ năng chiến đấu thứ 2
    skill2_level INTEGER DEFAULT 0,     -- Cấp độ kỹ năng thứ 2
    skill3_id    INTEGER DEFAULT 0,     -- Mã ID kỹ năng chiến đấu thứ 3
    skill3_level INTEGER DEFAULT 0,     -- Cấp độ kỹ năng thứ 3
    skill4_id    INTEGER DEFAULT 0,     -- Mã ID kỹ năng chiến đấu thứ 4
    skill4_level INTEGER DEFAULT 0,     -- Cấp độ kỹ năng thứ 4
    isactive     INTEGER DEFAULT 0,     -- Cờ trạng thái xuất chiến: 1=Đang chọn làm Pet chiến đấu chính, 0=Nghỉ ngơi
    thd          INTEGER DEFAULT 0,     -- Điểm thuần phục (THD) của võ tướng
    texp         INTEGER DEFAULT 0,     -- Điểm kinh nghiệm TEXP của võ tướng
    quest        INTEGER DEFAULT 0,     -- Cờ nhiệm vụ / sự kiện của võ tướng
    PRIMARY KEY (playerid, storagetype, slot)
);

-- ============================================================================
-- character_skills — Kỹ năng đã học của nhân vật người chơi.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_skills (
    playerid INTEGER NOT NULL,          -- Mã ID nhân vật người chơi
    skillid  INTEGER NOT NULL,          -- Mã ID kỹ năng đã học (theo file dữ liệu Skill.dat)
    level    INTEGER DEFAULT 1,         -- Đẳng cấp của kỹ năng (1..10)
    sp       INTEGER DEFAULT 0,         -- Lượng nội lực (SP) tiêu hao khi thi triển kỹ năng
    saveflag INTEGER DEFAULT 0,         -- Cờ đánh dấu lưu trữ thuộc tính đặc thù kỹ năng
    PRIMARY KEY (playerid, skillid)
);

-- ============================================================================
-- character_hotkeys — Danh sách phím tắt gán kỹ năng nhanh (Hotbar 10 ô).
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_hotkeys (
    playerid INTEGER NOT NULL,          -- Mã ID nhân vật người chơi
    slot     INTEGER NOT NULL,          -- Vị trí ô phím tắt nhanh trên thanh hotbar (1..10)
    skillid  INTEGER DEFAULT 0,         -- Mã ID kỹ năng hoặc biểu cảm gán vào ô phím tắt (0 = để trống)
    PRIMARY KEY (playerid, slot)
);

-- ============================================================================
-- character_missions — Tiến độ thực hiện nhiệm vụ (Quest) của nhân vật.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_missions (
    playerid  INTEGER NOT NULL,         -- Mã ID nhân vật người chơi
    missionid INTEGER NOT NULL,         -- Mã ID nhiệm vụ (theo cấu trúc kịch bản nhiệm vụ)
    step      INTEGER DEFAULT 0,        -- Bước / Giai đoạn hiện tại của nhiệm vụ đang thực hiện
    state     INTEGER DEFAULT 0,        -- Trạng thái: 0=Chưa nhận, 1=Đang làm, 2=Đã hoàn thành, 3=Thất bại
    updatedat INTEGER DEFAULT 0,        -- Thời điểm cập nhật trạng thái nhiệm vụ gần nhất (timestamp unix)
    PRIMARY KEY (playerid, missionid)
);

-- ============================================================================
-- character_mission_flags — Cờ trạng thái chi tiết theo từng nhiệm vụ.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_mission_flags (
    playerid  INTEGER NOT NULL,         -- Mã ID nhân vật người chơi
    missionid INTEGER NOT NULL,         -- Mã ID nhiệm vụ liên quan
    flagkey   TEXT NOT NULL,            -- Tên khóa định danh của cờ tiến trình (vd: "kill_count", "talk_step")
    flagvalue INTEGER DEFAULT 0,        -- Giá trị số nguyên lưu trạng thái của cờ
    PRIMARY KEY (playerid, missionid, flagkey)
);

-- ============================================================================
-- character_bit_flags — Cờ bit vĩnh viễn (Mỗi hàng đại diện cho 1 bit đã bật).
-- Dùng đánh dấu hoàn thành sự kiện hoặc nhận thưởng 1 lần duy nhất trong đời nhân vật.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_bit_flags (
    playerid  INTEGER NOT NULL,         -- Mã ID nhân vật người chơi
    flagindex INTEGER NOT NULL,         -- Chỉ số index của bit cờ (0..n)
    set_at    INTEGER DEFAULT 0,        -- Thời điểm bit cờ được kích hoạt bật lên 1 (timestamp unix)
    PRIMARY KEY (playerid, flagindex)
);

-- ============================================================================
-- character_completed_events — Lịch sử ghi nhận các sự kiện kịch bản Eve đã hoàn thành.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_completed_events (
    playerid    INTEGER NOT NULL,       -- Mã ID nhân vật người chơi
    eventid     INTEGER NOT NULL,       -- Mã ID sự kiện kịch bản trong eve.emg đã tham gia
    completedat INTEGER DEFAULT 0,      -- Thời điểm hoàn thành sự kiện (timestamp unix)
    PRIMARY KEY (playerid, eventid)
);

-- ============================================================================
-- friends — Danh sách hảo hữu / Bạn bè (Quan hệ đối xứng giữa 2 nhân vật).
-- ============================================================================
CREATE TABLE IF NOT EXISTS friends (
    playerid INTEGER NOT NULL,          -- Mã ID nhân vật người chơi sở hữu danh sách
    friendid INTEGER NOT NULL,          -- Mã ID người chơi bạn bè kết giao
    remark   TEXT,                      -- Biệt hiệu / Ghi chú gợi nhớ đặt cho người bạn này
    PRIMARY KEY (playerid, friendid)
);

-- ============================================================================
-- mails — Hòm thư tín của người chơi (Hỗ trợ gửi thư hệ thống, thư kèm tiền và vật phẩm).
-- ============================================================================
CREATE TABLE IF NOT EXISTS mails (
    mailid           INTEGER PRIMARY KEY AUTOINCREMENT,     -- Mã ID định danh thư tự tăng duy nhất
    senderid         INTEGER DEFAULT 0,                     -- Mã ID người chơi gửi thư (0 = Thư tự động từ Hệ thống/NPC)
    receiverid       INTEGER NOT NULL,                      -- Mã ID người chơi nhận thư
    title            TEXT,                                  -- Tiêu đề của bức thư
    body             TEXT,                                  -- Nội dung chi tiết bức thư
    gold             INTEGER DEFAULT 0,                     -- Lượng tiền vàng (Gold) gửi đính kèm theo thư
    attachmentitemid INTEGER DEFAULT 0,                     -- Mã ID vật phẩm đính kèm theo thư (theo Item.dat; 0 = không có)
    attachmentcount  INTEGER DEFAULT 0,                     -- Số lượng vật phẩm đính kèm tương ứng
    sentat           INTEGER DEFAULT 0,                     -- Thời điểm gửi thư đi (timestamp unix)
    claimed          INTEGER DEFAULT 0                      -- Trạng thái nhận thư: 0 = Chưa nhận/chưa đọc, 1 = Đã nhận quà
);

CREATE INDEX IF NOT EXISTS mails_receiver ON mails (receiverid);

-- ============================================================================
-- item_code — Hệ thống mã quà tặng (Giftcode / Redeem Code) - Opcode 0x23 sub 3.
-- ============================================================================
CREATE TABLE IF NOT EXISTS item_code (
    code     TEXT NOT NULL,             -- Chuỗi mã giftcode người chơi nhập (vd: TSVN123)
    password TEXT NOT NULL,             -- Chuỗi mật khẩu bảo mật đi kèm mã quà (secret pass)
    playerid INTEGER NOT NULL DEFAULT 0,-- Mã ID người chơi đã sử dụng giftcode này (0 = Chưa có ai dùng)
    usedat   INTEGER NULL,              -- Thời điểm mã quà được đổi thành công (timestamp unix; NULL nếu chưa dùng)
    itemid   INTEGER DEFAULT 0,         -- Mã ID vật phẩm nhận được khi đổi mã quà (theo Item.dat)
    count    INTEGER DEFAULT 0          -- Số lượng vật phẩm được tặng khi đổi mã quà
);

CREATE INDEX IF NOT EXISTS item_code_code ON item_code (code);
CREATE INDEX IF NOT EXISTS item_code_redeem ON item_code (code, password, playerid);

-- ============================================================================
-- gm_audit_log — Nhật ký ghi lại các thao tác phân quyền quản trị viên GM.
-- ============================================================================
CREATE TABLE IF NOT EXISTS gm_audit_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_account_id  INTEGER NOT NULL,
    actor_gm_level    INTEGER NOT NULL,
    target_account_id INTEGER NULL,
    action            TEXT NOT NULL,
    details           TEXT NOT NULL,
    created_at        INTEGER NOT NULL
);

