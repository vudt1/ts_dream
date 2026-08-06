-- TS Dream — MySQL 8 schema (Chapter 5). Shared database `ts_dream`.
-- Every game-text column is explicitly CHARACTER SET latin1 (COLLATE latin1_bin)
-- so VISCII byte names (0x80–0xFF) round-trip without utf8mb4 transcoding.
-- No FOREIGN KEY / NOT NULL beyond the legacy Access schema (parity).

-- ============================================================================
-- accounts — created exclusively through the web dashboard (Chapter 5 §5.8).
-- Passwords kept plaintext (parity with the C# server).
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounts (
    player_id    BIGINT AUTO_INCREMENT PRIMARY KEY,
    pass1 VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    pass2 VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL
) ENGINE = InnoDB AUTO_INCREMENT = 300000 
  DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- players — one row per created character; its id doubles as the account id.
-- Numeric Access DOUBLE -> BIGINT; defaults kept verbatim.
-- ============================================================================
CREATE TABLE IF NOT EXISTS players (
    player_id         BIGINT PRIMARY KEY,
    Name              VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin,
    Lv                BIGINT DEFAULT 1,
    Hp                BIGINT DEFAULT 0,
    HpMax             BIGINT DEFAULT 0,
    Sp                BIGINT DEFAULT 0,
    SpMax             BIGINT DEFAULT 0,
    Point             BIGINT DEFAULT 0,
    SkillPoint        BIGINT DEFAULT 0,
    `Int`             BIGINT DEFAULT 0,
    Atk               BIGINT DEFAULT 0,
    Def               BIGINT DEFAULT 0,
    Hpx               BIGINT DEFAULT 0,
    Spx               BIGINT DEFAULT 0,
    Agi               BIGINT DEFAULT 0,
    Int2              BIGINT DEFAULT 0,
    Atk2              BIGINT DEFAULT 0,
    Def2              BIGINT DEFAULT 0,
    Hpx2              BIGINT DEFAULT 0,
    Spx2              BIGINT DEFAULT 0,
    Agi2              BIGINT DEFAULT 0,
    Texp              BIGINT DEFAULT 0,
    MapId             BIGINT DEFAULT 0,
    MapX              BIGINT DEFAULT 0,
    MapY              BIGINT DEFAULT 0,
    Reborn            BIGINT DEFAULT 0,
    Job               BIGINT DEFAULT 0,
    Sex               BIGINT DEFAULT 0,
    Hair              BIGINT DEFAULT 0,
    Thuoctinh         BIGINT DEFAULT 0,
    Ghost             BIGINT DEFAULT 0,
    God               BIGINT DEFAULT 0,
    Color             VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin,
    Gold              BIGINT DEFAULT 0,
    Tiengtam          BIGINT DEFAULT 0,
    Gocnhin           BIGINT DEFAULT 0,
    SttPetXuatchien   BIGINT DEFAULT 0,
    Pk                BIGINT DEFAULT 0,
    ThamChien         BIGINT DEFAULT 0,
    ShopPoint         BIGINT DEFAULT 0,
    SP_Store          BIGINT DEFAULT 10000,
    HP_Store          BIGINT DEFAULT 10000,
    DTT               BIGINT DEFAULT 0,
    TLP               BIGINT DEFAULT 0,
    TCP               BIGINT DEFAULT 0,
    TTP               BIGINT DEFAULT 0,
    savemap           BIGINT DEFAULT 0,
    tanthu            BIGINT DEFAULT 0,
    phien             BIGINT DEFAULT 0,
    PTS               BIGINT DEFAULT 0,
    KEY players_mapid (MapId)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- 9 gameplay tables (shared schema, per-player composite PK incl. player_id).
-- ============================================================================

-- Homdo — inventory slots.
CREATE TABLE IF NOT EXISTS homdo (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id        BIGINT DEFAULT 0,
    `Count`   BIGINT DEFAULT 0,
    Lv        BIGINT DEFAULT 0,
    DoBen     BIGINT DEFAULT 0,
    Int1      BIGINT DEFAULT 0,
    Atk1      BIGINT DEFAULT 0,
    Def1      BIGINT DEFAULT 0,
    Hpx1      BIGINT DEFAULT 0,
    Spx1      BIGINT DEFAULT 0,
    Agi1      BIGINT DEFAULT 0,
    Fai1      BIGINT DEFAULT 0,
    Int2      BIGINT DEFAULT 0,
    Atk2      BIGINT DEFAULT 0,
    Def2      BIGINT DEFAULT 0,
    Hpx2      BIGINT DEFAULT 0,
    Spx2      BIGINT DEFAULT 0,
    Agi2      BIGINT DEFAULT 0,
    Fai2      BIGINT DEFAULT 0,
    Hp        BIGINT DEFAULT 0,
    Sp        BIGINT DEFAULT 0,
    `Long`    BIGINT DEFAULT 0,
    GiatriLong BIGINT DEFAULT 0,
    Khang     BIGINT DEFAULT 0,
    Thuoctinh BIGINT DEFAULT 0,
    GiatriThuoctinh BIGINT DEFAULT 0,
    Loai      BIGINT DEFAULT 0,
    Texp      BIGINT DEFAULT 0,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- LuuLang (storage).
CREATE TABLE IF NOT EXISTS luulang (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id         BIGINT DEFAULT 0, 
	`Count` BIGINT DEFAULT 0,
	Lv BIGINT DEFAULT 0, 
	DoBen BIGINT DEFAULT 0,
    Int1 BIGINT DEFAULT 0,
	Atk1 BIGINT DEFAULT 0,
	Def1 BIGINT DEFAULT 0,
	Hpx1 BIGINT DEFAULT 0,
	Spx1 BIGINT DEFAULT 0,
	Agi1 BIGINT DEFAULT 0,
	Fai1 BIGINT DEFAULT 0,
    Int2 BIGINT DEFAULT 0,
	Atk2 BIGINT DEFAULT 0,
	Def2 BIGINT DEFAULT 0,
	Hpx2 BIGINT DEFAULT 0,
	Spx2 BIGINT DEFAULT 0,
	Agi2 BIGINT DEFAULT 0,
	Fai2 BIGINT DEFAULT 0,
    Hp BIGINT DEFAULT 0,
	Sp BIGINT DEFAULT 0,
	`Long` BIGINT DEFAULT 0,
	GiatriLong BIGINT DEFAULT 0,
	Khang BIGINT DEFAULT 0,
    Thuoctinh BIGINT DEFAULT 0,
	GiatriThuoctinh BIGINT DEFAULT 0,
	Loai BIGINT DEFAULT 0,
	Texp BIGINT DEFAULT 0,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- Pet.
CREATE TABLE IF NOT EXISTS pet (
    player_id BIGINT NOT NULL,
    Stt       BIGINT NOT NULL,
    Id        BIGINT DEFAULT 0,
    Name      VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin,
    Lv        BIGINT DEFAULT 0,
    Thuoctinh BIGINT DEFAULT 0,
    Reborn    BIGINT DEFAULT 0,
    Hp        BIGINT DEFAULT 0,
    HpMax     BIGINT DEFAULT 0,
    Sp        BIGINT DEFAULT 0,
    SpMax     BIGINT DEFAULT 0,
    `Int`     BIGINT DEFAULT 0,
    Atk       BIGINT DEFAULT 0,
    Def       BIGINT DEFAULT 0,
    Hpx       BIGINT DEFAULT 0,
    Hpx       BIGINT DEFAULT 0,
    Hpx       BIGINT DEFAULT 0,
    Spx       BIGINT DEFAULT 0,
    Agi       BIGINT DEFAULT 0,
    Fai       BIGINT DEFAULT 0,
    Texp      BIGINT DEFAULT 0,
    Int2      BIGINT DEFAULT 0,
    Atk2      BIGINT DEFAULT 0,
    Def2      BIGINT DEFAULT 0,
    Hpx2      BIGINT DEFAULT 0,
    Spx2      BIGINT DEFAULT 0,
    Thd       BIGINT DEFAULT 0,
    SkillPoint BIGINT DEFAULT 0,
    Quest     BIGINT DEFAULT 0,
    Idskill1  BIGINT DEFAULT 0,
	LvSkill1 BIGINT DEFAULT 0,
    IdSkill2  BIGINT DEFAULT 0,
	LvSkill2 BIGINT DEFAULT 0,
    IdSkill3  BIGINT DEFAULT 0,
	LvSkill3 BIGINT DEFAULT 0,
    IdSkill4  BIGINT DEFAULT 0,
	LvSkill4 BIGINT DEFAULT 0,
    Agi2      BIGINT DEFAULT 0,
    KEY pet_idskill1 (Idskill1),
    KEY pet_idskill2 (Idskill2),
    KEY pet_idskill3 (Idskill3),
    KEY pet_idskill4 (Idskill4),
    PRIMARY KEY (player_id, Stt)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- Quest — no PK (Access has none); keep a KEY on QuestId. Do NOT invent
-- NOT NULL / UNIQUE.
CREATE TABLE IF NOT EXISTS quest (
    player_id BIGINT NOT NULL,
    QuestId   BIGINT DEFAULT 0,
    MapId     BIGINT DEFAULT 0,
    NpcId     BIGINT DEFAULT 0,
    WarpId    BIGINT DEFAULT 0,
    Step      BIGINT DEFAULT 0,
    KEY quest_questid (QuestId)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- Skill.
CREATE TABLE IF NOT EXISTS skill (
    player_id BIGINT NOT NULL,
    Id        BIGINT NOT NULL,
    Lv        BIGINT DEFAULT 1,
    Sp        BIGINT DEFAULT 0,
    Save      BIGINT DEFAULT 0,
    PRIMARY KEY (player_id, Id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- SkillSave (hotbar). Saed rows 1..10 / IdSkill 0 at character creation (C#
-- never INSERTs; it only UPDATEs).
CREATE TABLE IF NOT EXISTS skillsave (
    player_id BIGINT NOT NULL,
    ID        BIGINT NOT NULL,
    IdSkill   BIGINT DEFAULT 0,
    KEY skillsave_idskill (IdSkill),
    PRIMARY KEY (player_id, ID)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- TienTrang (storage), Trangbi (equip), Tuideo (other pouch).
CREATE TABLE IF NOT EXISTS tientrang (
    player_id BIGINT NOT NULL,
    Slot BIGINT NOT NULL,
    Id BIGINT DEFAULT 0,
	`Count` BIGINT DEFAULT 0,
	Lv BIGINT DEFAULT 0,
	DoBen BIGINT DEFAULT 0,
    Int1 BIGINT DEFAULT 0,
	Atk1 BIGINT DEFAULT 0,
	Def1 BIGINT DEFAULT 0,
	Hpx1 BIGINT DEFAULT 0,
	Spx1 BIGINT DEFAULT 0,
	Agi1 BIGINT DEFAULT 0,
	Fai1 BIGINT DEFAULT 0,
    Int2 BIGINT DEFAULT 0,
	Atk2 BIGINT DEFAULT 0,
	Def2 BIGINT DEFAULT 0,
	Hpx2 BIGINT DEFAULT 0,
	Spx2 BIGINT DEFAULT 0,
	Agi2 BIGINT DEFAULT 0,
	Fai2 BIGINT DEFAULT 0,
    Hp BIGINT DEFAULT 0,
	Sp BIGINT DEFAULT 0,
	`Long` BIGINT DEFAULT 0,
	GiatriLong BIGINT DEFAULT 0,
	Khang BIGINT DEFAULT 0,
    Thuoctinh BIGINT DEFAULT 0,
	GiatriThuoctinh BIGINT DEFAULT 0,
	Loai BIGINT DEFAULT 0,
	Texp BIGINT DEFAULT 0,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS trangbi (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id BIGINT DEFAULT 0, 
	`Count` BIGINT DEFAULT 0,
	Lv BIGINT DEFAULT 0 ,
	DoBen BIGINT DEFAULT 0,
    Int1 BIGINT DEFAULT 0,
	Atk1 BIGINT DEFAULT 0, 
	Def1 BIGINT DEFAULT 0,
	Hpx1 BIGINT DEFAULT 0,
	Spx1 BIGINT DEFAULT 0,
	Agi1 BIGINT DEFAULT 0,
	Fai1 BIGINT DEFAULT 0,
    Int2 BIGINT DEFAULT 0,
	Atk2 BIGINT DEFAULT 0,
	Def2 BIGINT DEFAULT 0,
	Hpx2 BIGINT DEFAULT 0,
	Spx2 BIGINT DEFAULT 0,
	Agi2 BIGINT DEFAULT 0,
	Fai2 BIGINT DEFAULT 0,
    Hp BIGINT DEFAULT 0,
	Sp BIGINT DEFAULT 0,
	`Long` BIGINT DEFAULT 0,
	GiatriLong BIGINT DEFAULT 0,
	Khang BIGINT DEFAULT 0,
    Thuoctinh BIGINT DEFAULT 0,
	GiatriThuoctinh BIGINT DEFAULT 0,
	Loai BIGINT DEFAULT 0,
	Texp BIGINT DEFAULT 0,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS tuideo (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id BIGINT DEFAULT 0,
	`Count` BIGINT DEFAULT 0,
	Lv BIGINT DEFAULT 0,
	DoBen BIGINT DEFAULT 0,
    Int1 BIGINT DEFAULT 0,
	Atk1 BIGINT DEFAULT 0,
	Def1 BIGINT DEFAULT 0,
	Hpx1 BIGINT DEFAULT 0,
	Spx1 BIGINT DEFAULT 0,
	Agi1 BIGINT DEFAULT 0,
	Fai1 BIGINT DEFAULT 0,
    Int2 BIGINT DEFAULT 0,
	Atk2 BIGINT DEFAULT 0,
	Def2 BIGINT DEFAULT 0,
	Hpx2 BIGINT DEFAULT 0,
	Spx2 BIGINT DEFAULT 0,
	Agi2 BIGINT DEFAULT 0,
	Fai2 BIGINT DEFAULT 0,
    Hp BIGINT DEFAULT 0,
	Sp BIGINT DEFAULT 0,
	`Long` BIGINT DEFAULT 0,
	GiatriLong BIGINT DEFAULT 0,
	Khang BIGINT DEFAULT 0,
    Thuoctinh BIGINT DEFAULT 0,
	GiatriThuoctinh BIGINT DEFAULT 0,
	Loai BIGINT DEFAULT 0,
	Texp BIGINT DEFAULT 0,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- item_code — redeemable gift codes (op 0x23 sub 3). Fully functional.
-- ============================================================================
CREATE TABLE IF NOT EXISTS item_code (
    code       VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    password   VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    player_id  BIGINT NOT NULL DEFAULT 0,
    used_at    BIGINT NULL,
    item_id    BIGINT DEFAULT 0,
    `count`    BIGINT DEFAULT 0,
    KEY item_code_code (code),
    KEY item_code_redeem (code, password, player_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;