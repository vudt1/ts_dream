-- ============================================================================
-- accounts — created exclusively through the web dashboard (Chapter 5 §5.8).
-- Passwords kept plaintext (parity with the C# server).
-- ============================================================================
CREATE TABLE IF NOT EXISTS accounts (
    player_id    BIGINT AUTO_INCREMENT PRIMARY KEY,
    pass1 VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    pass2 VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
	is_suspended TINYINT(1) NOT NULL DEFAULT 0,
	suspension_reason VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NULL,
	suspended_until BIGINT NULL,
	last_login_at BIGINT NULL,
	created_at BIGINT NOT NULL,
	updated_at BIGINT NULL,
	last_login_ip VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NULL,
	gm_level INT NOT NULL DEFAULT 0
) ENGINE = InnoDB AUTO_INCREMENT = 300000 
  DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

ALTER TABLE accounts
    ADD KEY accounts_suspended_idx (is_suspended, suspended_until),
    ADD KEY accounts_gm_level_idx (gm_level);
ALTER TABLE accounts
    ADD CONSTRAINT accounts_gm_level_nonnegative CHECK (gm_level >= 0);


CREATE TABLE IF NOT EXISTS gm_audit_log (
    id            BIGINT AUTO_INCREMENT PRIMARY KEY,
    actor_account_id BIGINT NULL,
    actor_gm_level INT NOT NULL DEFAULT 0,
    target_account_id BIGINT NULL,
    action        VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    details       TEXT CHARACTER SET latin1 COLLATE latin1_bin NULL,
    created_at    BIGINT NOT NULL,
    KEY gm_audit_actor_idx (actor_account_id, created_at),
    KEY gm_audit_target_idx (target_account_id, created_at),
    KEY gm_audit_action_idx (action, created_at)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- TS Dream — modern 3NF schema.
--
-- Normalized target relations for the runtime cutover from 0001:
-- STATUS: modern repository implementations exist, but the current live Rust
-- login/autosave path still targets selected 0001 tables. Do not remove 0001
-- until dual-read/dual-write or a verified data migration has completed.
--   accounts (0001) 1:1 characters 1:1 character_money
--     (the PC server supports exactly ONE character per account; the UNIQUE
--      key on characters.account_id enforces it at the schema level)
--   characters 1:N inventories (unifies the legacy pouches homdo / trangbi
--     into one table keyed by
--     (character_id, storage_type, slot) with the full 35-byte ThingData
--     attribute set)
--   characters 1:N character_pets across four pet storages
--     (1=Carried, 2=Cart, 3=Hotel, 4=Warehouse)
--   missions / bit flags / completed events / friends / mails round out the
--     persistence surface ticket 07's handlers will bind to.
--
-- Conventions kept from 0001:
--   - Every game-text column is CHARACTER SET latin1 COLLATE latin1_bin so
--     raw VISCII bytes (0x80-0xFF) round-trip without utf8mb4 transcoding.
--   - No FOREIGN KEY constraints and no NOT NULL beyond what a row needs to
--     be addressable (legacy Access parity; referential integrity is owned
--     by the repository layer).


-- ============================================================================
-- characters — the single playable character of one account (1:1; the PC
-- server never supports multiple avatars per account).
-- ============================================================================
CREATE TABLE IF NOT EXISTS characters (
    character_id  BIGINT NOT NULL PRIMARY KEY,
    name        VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    level       BIGINT DEFAULT 1,
    job         BIGINT DEFAULT 0,
    sex         BIGINT DEFAULT 0,
    hair        BIGINT DEFAULT 0,
    element     BIGINT DEFAULT 0,
    reborn      BIGINT DEFAULT 0,
    hp          BIGINT DEFAULT 0,
    hp_max      BIGINT DEFAULT 0,
    sp          BIGINT DEFAULT 0,
    sp_max      BIGINT DEFAULT 0,
    stat_point  BIGINT DEFAULT 0,
    skill_point BIGINT DEFAULT 0,
    int_attr    BIGINT DEFAULT 0,
    atk         BIGINT DEFAULT 0,
    def         BIGINT DEFAULT 0,
    hpx         BIGINT DEFAULT 0,
    spx         BIGINT DEFAULT 0,
    agi         BIGINT DEFAULT 0,
    map_id      BIGINT DEFAULT 0,
    map_x       BIGINT DEFAULT 0,
    map_y       BIGINT DEFAULT 0,
    newbie      BIGINT NOT NULL DEFAULT 0,
    created_at  BIGINT DEFAULT 0,
    KEY characters_name (name)
) ENGINE=InnoDB AUTO_INCREMENT = 300000
  DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- character_money — currency ledger split out of the character row so gold
-- mutations (shop buy, bank transfer, trade) stay narrow and lockable.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_money (
    character_id BIGINT PRIMARY KEY,
    gold         BIGINT DEFAULT 0,
    bank_gold    BIGINT DEFAULT 0,
    shop_point   BIGINT DEFAULT 0
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- inventories — single item storage for every container type.
-- storage_type: 1=Bag, 2=Secondary bag, 4=Bank, 8=Equip, 16=Warehouse.
-- Item columns mirror the 20 fields of the 35-byte ThingData wire struct
-- exactly (see src/protocol/codecs/thing_data.rs), Little-Endian on the wire,
-- one column per field here.
-- ============================================================================
CREATE TABLE IF NOT EXISTS inventories (
    character_id    BIGINT NOT NULL,
    storage_type    TINYINT UNSIGNED NOT NULL,
    slot            SMALLINT UNSIGNED NOT NULL,
    item_id         SMALLINT UNSIGNED DEFAULT 0,
    quantity        INT DEFAULT 0,
    damage          TINYINT UNSIGNED DEFAULT 0,
    element         TINYINT UNSIGNED DEFAULT 0,
    element_value   TINYINT UNSIGNED DEFAULT 0,
    proof_kind      TINYINT UNSIGNED DEFAULT 0,
    grow_level      TINYINT UNSIGNED DEFAULT 0,
    grow_exp        INT DEFAULT 0,
    special_kind    TINYINT UNSIGNED DEFAULT 0,
    stone_attr      TINYINT UNSIGNED DEFAULT 0,
    stone_level     TINYINT UNSIGNED DEFAULT 0,
    enhance_level   TINYINT UNSIGNED DEFAULT 0,
    delete_time     DOUBLE DEFAULT 0,
    damaged_item_id SMALLINT UNSIGNED DEFAULT 0,
    is_locked       TINYINT(1) DEFAULT 0,
    reinforced      TINYINT UNSIGNED DEFAULT 0,
    affix1          TINYINT UNSIGNED DEFAULT 0,
    affix2          TINYINT UNSIGNED DEFAULT 0,
    affix3          TINYINT UNSIGNED DEFAULT 0,
    style_level     TINYINT UNSIGNED DEFAULT 0,
    PRIMARY KEY (character_id, storage_type, slot),
    KEY inventories_item (character_id, item_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- character_pets — general (general) storage across four warehouses:
-- storage_type: 1=Carried, 2=Cart, 3=Hotel, 4=Warehouse.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_pets (
    character_id  BIGINT NOT NULL,
    storage_type  TINYINT UNSIGNED NOT NULL,
    slot          SMALLINT UNSIGNED NOT NULL,
    pet_id        SMALLINT UNSIGNED DEFAULT 0,
    name          VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin,
    level         BIGINT DEFAULT 1,
    element       BIGINT DEFAULT 0,
    reborn        BIGINT DEFAULT 0,
    hp            BIGINT DEFAULT 0,
    hp_max        BIGINT DEFAULT 0,
    sp            BIGINT DEFAULT 0,
    sp_max        BIGINT DEFAULT 0,
    int_attr      BIGINT DEFAULT 0,
    atk           BIGINT DEFAULT 0,
    def           BIGINT DEFAULT 0,
    hpx           BIGINT DEFAULT 0,
    spx           BIGINT DEFAULT 0,
    agi           BIGINT DEFAULT 0,
    fai           BIGINT DEFAULT 0,
    texp          BIGINT DEFAULT 0,
    skill_point   BIGINT DEFAULT 0,
    thd           BIGINT DEFAULT 0,
    skill1_id     BIGINT DEFAULT 0,
    skill1_level  BIGINT DEFAULT 0,
    skill2_id     BIGINT DEFAULT 0,
    skill2_level  BIGINT DEFAULT 0,
    skill3_id     BIGINT DEFAULT 0,
    skill3_level  BIGINT DEFAULT 0,
    skill4_id     BIGINT DEFAULT 0,
    skill4_level  BIGINT DEFAULT 0,
    quest         BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, storage_type, slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- character_skills / character_hotkeys — learned skills and the 10-slot
-- hotbar (parity with legacy `skill` / `skillsave`).
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_skills (
    character_id BIGINT NOT NULL,
    skill_id     BIGINT NOT NULL,
    level        BIGINT DEFAULT 1,
    sp           BIGINT DEFAULT 0,
    save_flag    BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, skill_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS character_hotkeys (
    character_id BIGINT NOT NULL,
    slot         TINYINT UNSIGNED NOT NULL,
    skill_id     BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, slot)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- Missions, permanent flags and completed events.
-- ============================================================================
CREATE TABLE IF NOT EXISTS character_missions (
    character_id BIGINT NOT NULL,
    mission_id   BIGINT NOT NULL,
    step         BIGINT DEFAULT 0,
    state        BIGINT DEFAULT 0,
    updated_at   BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, mission_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS character_mission_flags (
    character_id BIGINT NOT NULL,
    mission_id   BIGINT NOT NULL,
    flag_key     VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    flag_value   BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, mission_id, flag_key)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- Forever flags: one row per permanently-latched bit index.
CREATE TABLE IF NOT EXISTS character_bit_flags (
    character_id BIGINT NOT NULL,
    flag_index   INT UNSIGNED NOT NULL,
    set_at       BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, flag_index)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS character_completed_events (
    character_id BIGINT NOT NULL,
    event_id     BIGINT NOT NULL,
    completed_at BIGINT DEFAULT 0,
    PRIMARY KEY (character_id, event_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- friends — symmetric pair rows owned by the repository layer (no FK).
-- ============================================================================
CREATE TABLE IF NOT EXISTS friends (
    character_id BIGINT NOT NULL,
    friend_id    BIGINT NOT NULL,
    remark       VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin,
    PRIMARY KEY (character_id, friend_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

-- ============================================================================
-- mails — mailbox rows; attachments are carried as item + count until
-- claimed (mail_attachments stays out until the mail system ships).
-- ============================================================================
CREATE TABLE IF NOT EXISTS mails (
    mail_id             BIGINT AUTO_INCREMENT PRIMARY KEY,
    sender_id           BIGINT DEFAULT 0,
    receiver_id         BIGINT NOT NULL,
    title               VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin,
    body                TEXT CHARACTER SET latin1 COLLATE latin1_bin,
    gold                BIGINT DEFAULT 0,
    attachment_item_id  SMALLINT UNSIGNED DEFAULT 0,
    attachment_count    INT DEFAULT 0,
    sent_at             BIGINT DEFAULT 0,
    claimed             TINYINT(1) DEFAULT 0,
    KEY mails_receiver (receiver_id)
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
