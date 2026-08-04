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
    Lv                BIGINT,
    Hp                BIGINT,
    HpMax             BIGINT,
    Sp                BIGINT,
    SpMax             BIGINT,
    Point             BIGINT,
    SkillPoint        BIGINT,
    `Int`             BIGINT,
    Atk               BIGINT,
    Def               BIGINT,
    Hpx               BIGINT,
    Spx               BIGINT,
    Agi               BIGINT,
    Int2              BIGINT,
    Atk2              BIGINT,
    Def2              BIGINT,
    Hpx2              BIGINT,
    Spx2              BIGINT,
    Agi2              BIGINT,
    Texp              BIGINT,
    MapId             BIGINT,
    MapX              BIGINT,
    MapY              BIGINT,
    Reborn            BIGINT,
    Job               BIGINT,
    Sex               BIGINT,
    Hair              BIGINT,
    Thuoctinh         BIGINT,
    Ghost             BIGINT,
    God               BIGINT,
    Color             VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin,
    Gold              BIGINT,
    Tiengtam          BIGINT,
    Gocnhin           BIGINT,
    SttPetXuatchien   BIGINT,
    Pk                BIGINT,
    ThamChien         BIGINT,
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
    Id        BIGINT,
    `Count`   BIGINT,
    Lv        BIGINT,
    DoBen     BIGINT,
    Int1      BIGINT,
    Atk1      BIGINT,
    Def1      BIGINT,
    Hpx1      BIGINT,
    Spx1      BIGINT,
    Agi1      BIGINT,
    Fai1      BIGINT,
    Int2      BIGINT,
    Atk2      BIGINT,
    Def2      BIGINT,
    Hpx2      BIGINT,
    Spx2      BIGINT,
    Agi2      BIGINT,
    Fai2      BIGINT,
    Hp        BIGINT,
    Sp        BIGINT,
    `Long`    BIGINT,
    GiatriLong BIGINT,
    Khang     BIGINT,
    Thuoctinh BIGINT,
    GiatriThuoctinh BIGINT,
    Loai      BIGINT,
    Texp      BIGINT,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- LuuLang (storage).
CREATE TABLE IF NOT EXISTS luulang (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id         BIGINT, `Count` BIGINT, Lv BIGINT, DoBen BIGINT,
    Int1 BIGINT, Atk1 BIGINT, Def1 BIGINT, Hpx1 BIGINT, Spx1 BIGINT, Agi1 BIGINT, Fai1 BIGINT,
    Int2 BIGINT, Atk2 BIGINT, Def2 BIGINT, Hpx2 BIGINT, Spx2 BIGINT, Agi2 BIGINT, Fai2 BIGINT,
    Hp BIGINT, Sp BIGINT, `Long` BIGINT, GiatriLong BIGINT, Khang BIGINT,
    Thuoctinh BIGINT, GiatriThuoctinh BIGINT, Loai BIGINT, Texp BIGINT,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- Pet.
CREATE TABLE IF NOT EXISTS pet (
    player_id BIGINT NOT NULL,
    Stt       BIGINT NOT NULL,
    Id        BIGINT,
    Name      VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin,
    Lv        BIGINT,
    Thuoctinh BIGINT,
    Reborn    BIGINT,
    Hp        BIGINT,
    HpMax     BIGINT,
    Sp        BIGINT,
    SpMax     BIGINT,
    `Int`     BIGINT,
    Atk       BIGINT,
    Def       BIGINT,
    Hpx       BIGINT,
    Spx       BIGINT,
    Agi       BIGINT,
    Fai       BIGINT,
    Texp      BIGINT,
    Int2      BIGINT,
    Atk2      BIGINT,
    Def2      BIGINT,
    Hpx2      BIGINT,
    Spx2      BIGINT,
    Thd       BIGINT,
    SkillPoint BIGINT,
    Quest     BIGINT,
    Idskill1  BIGINT, LvSkill1 BIGINT,
    IdSkill2  BIGINT, LvSkill2 BIGINT,
    IdSkill3  BIGINT, LvSkill3 BIGINT,
    IdSkill4  BIGINT, LvSkill4 BIGINT,
    Agi2      BIGINT,
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
    QuestId   BIGINT,
    MapId     BIGINT,
    NpcId     BIGINT,
    WarpId    BIGINT,
    Step      BIGINT,
    KEY quest_questid (QuestId)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- Skill.
CREATE TABLE IF NOT EXISTS skill (
    player_id BIGINT NOT NULL,
    Id        BIGINT NOT NULL,
    Lv        BIGINT,
    Sp        BIGINT,
    Save      BIGINT,
    PRIMARY KEY (player_id, Id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- SkillSave (hotbar). Saed rows 1..10 / IdSkill 0 at character creation (C#
-- never INSERTs; it only UPDATEs).
CREATE TABLE IF NOT EXISTS skillsave (
    player_id BIGINT NOT NULL,
    ID        BIGINT NOT NULL,
    IdSkill   BIGINT,
    KEY skillsave_idskill (IdSkill),
    PRIMARY KEY (player_id, ID)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- TienTrang (storage), Trangbi (equip), Tuideo (other pouch).
CREATE TABLE IF NOT EXISTS tientrang (
    player_id BIGINT NOT NULL,
    Slot BIGINT NOT NULL,
    Id BIGINT, `Count` BIGINT, Lv BIGINT, DoBen BIGINT,
    Int1 BIGINT, Atk1 BIGINT, Def1 BIGINT, Hpx1 BIGINT, Spx1 BIGINT, Agi1 BIGINT, Fai1 BIGINT,
    Int2 BIGINT, Atk2 BIGINT, Def2 BIGINT, Hpx2 BIGINT, Spx2 BIGINT, Agi2 BIGINT, Fai2 BIGINT,
    Hp BIGINT, Sp BIGINT, `Long` BIGINT, GiatriLong BIGINT, Khang BIGINT,
    Thuoctinh BIGINT, GiatriThuoctinh BIGINT, Loai BIGINT, Texp BIGINT,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS trangbi (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id BIGINT, `Count` BIGINT, Lv BIGINT, DoBen BIGINT,
    Int1 BIGINT, Atk1 BIGINT, Def1 BIGINT, Hpx1 BIGINT, Spx1 BIGINT, Agi1 BIGINT, Fai1 BIGINT,
    Int2 BIGINT, Atk2 BIGINT, Def2 BIGINT, Hpx2 BIGINT, Spx2 BIGINT, Agi2 BIGINT, Fai2 BIGINT,
    Hp BIGINT, Sp BIGINT, `Long` BIGINT, GiatriLong BIGINT, Khang BIGINT,
    Thuoctinh BIGINT, GiatriThuoctinh BIGINT, Loai BIGINT, Texp BIGINT,
    PRIMARY KEY (player_id, Slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS tuideo (
    player_id BIGINT NOT NULL,
    Slot      BIGINT NOT NULL,
    Id BIGINT, `Count` BIGINT, Lv BIGINT, DoBen BIGINT,
    Int1 BIGINT, Atk1 BIGINT, Def1 BIGINT, Hpx1 BIGINT, Spx1 BIGINT, Agi1 BIGINT, Fai1 BIGINT,
    Int2 BIGINT, Atk2 BIGINT, Def2 BIGINT, Hpx2 BIGINT, Spx2 BIGINT, Agi2 BIGINT, Fai2 BIGINT,
    Hp BIGINT, Sp BIGINT, `Long` BIGINT, GiatriLong BIGINT, Khang BIGINT,
    Thuoctinh BIGINT, GiatriThuoctinh BIGINT, Loai BIGINT, Texp BIGINT,
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
    item_id    BIGINT,
    `count`    BIGINT,
    KEY item_code_code (code),
    KEY item_code_redeem (code, password, player_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;