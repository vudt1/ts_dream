-- Production domain expansion for TS Dream.
-- InnoDB + latin1_bin keeps legacy VISCII bytes stable on text fields.
-- The repository layer owns cross-table invariants; this migration intentionally
-- follows the existing project convention of no foreign-key constraints.

CREATE TABLE IF NOT EXISTS data_asset_catalog (
    source_file   VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin PRIMARY KEY,
    source_sha256 CHAR(64) NOT NULL,
    byte_size     BIGINT UNSIGNED NOT NULL DEFAULT 0,
    record_count  BIGINT UNSIGNED NOT NULL DEFAULT 0,
    loaded_at     BIGINT NOT NULL DEFAULT 0,
    status        VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'loaded',
    error_message TEXT CHARACTER SET latin1 COLLATE latin1_bin
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS static_items (
    item_id       BIGINT PRIMARY KEY,
    source_file   VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    name_bytes    VARBINARY(255),
    level         BIGINT NOT NULL DEFAULT 0,
    item_type     BIGINT NOT NULL DEFAULT 0,
    element       BIGINT NOT NULL DEFAULT 0,
    value         BIGINT NOT NULL DEFAULT 0,
    raw_payload   MEDIUMBLOB,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    KEY static_items_source (source_file)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS static_npcs (
    npc_id        BIGINT PRIMARY KEY,
    source_file   VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    name_bytes    VARBINARY(255),
    level         BIGINT NOT NULL DEFAULT 0,
    element       BIGINT NOT NULL DEFAULT 0,
    hp            BIGINT NOT NULL DEFAULT 0,
    sp            BIGINT NOT NULL DEFAULT 0,
    skill1        BIGINT NOT NULL DEFAULT 0,
    skill2        BIGINT NOT NULL DEFAULT 0,
    skill3        BIGINT NOT NULL DEFAULT 0,
    skill4        BIGINT NOT NULL DEFAULT 0,
    raw_payload   MEDIUMBLOB,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    KEY static_npcs_source (source_file)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS static_skills (
    skill_id      BIGINT PRIMARY KEY,
    source_file   VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    name_bytes    VARBINARY(255),
    element       BIGINT NOT NULL DEFAULT 0,
    max_level     BIGINT NOT NULL DEFAULT 0,
    require_sp    BIGINT NOT NULL DEFAULT 0,
    raw_payload   MEDIUMBLOB,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    KEY static_skills_source (source_file)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS static_quests (
    quest_id      BIGINT NOT NULL,
    step          BIGINT NOT NULL DEFAULT 0,
    source_file   VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    map_id        BIGINT NOT NULL DEFAULT 0,
    npc_id        BIGINT NOT NULL DEFAULT 0,
    warp_id       BIGINT NOT NULL DEFAULT 0,
    raw_payload   MEDIUMBLOB,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (quest_id, step),
    KEY static_quests_map (map_id, npc_id, warp_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS static_events (
    event_id      BIGINT NOT NULL,
    scene_id      BIGINT NOT NULL DEFAULT 0,
    event_type    BIGINT NOT NULL DEFAULT 0,
    raw_payload   MEDIUMBLOB,
    source_file   VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (scene_id, event_id),
    KEY static_events_type (event_type)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS guilds (
    guild_id      BIGINT AUTO_INCREMENT PRIMARY KEY,
    name          VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    notice        TEXT CHARACTER SET latin1 COLLATE latin1_bin,
    leader_id     BIGINT NOT NULL DEFAULT 0,
    level         BIGINT NOT NULL DEFAULT 1,
    experience    BIGINT NOT NULL DEFAULT 0,
    treasury      BIGINT NOT NULL DEFAULT 0,
    status        VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'active',
    created_at    BIGINT NOT NULL DEFAULT 0,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    UNIQUE KEY guilds_name (name),
    KEY guilds_leader (leader_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS guild_members (
    guild_id      BIGINT NOT NULL,
    character_id   BIGINT NOT NULL,
    role          TINYINT UNSIGNED NOT NULL DEFAULT 0,
    contribution  BIGINT NOT NULL DEFAULT 0,
    joined_at     BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (guild_id, character_id),
    UNIQUE KEY guild_members_character (character_id),
    KEY guild_members_role (guild_id, role)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS guild_relations (
    guild_id      BIGINT NOT NULL,
    other_guild_id BIGINT NOT NULL,
    relation_type  TINYINT UNSIGNED NOT NULL DEFAULT 0,
    updated_at     BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (guild_id, other_guild_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS guild_wars (
    war_id        BIGINT AUTO_INCREMENT PRIMARY KEY,
    attacker_id   BIGINT NOT NULL,
    defender_id   BIGINT NOT NULL,
    season_id     BIGINT NOT NULL DEFAULT 0,
    state         VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'scheduled',
    start_at      BIGINT NOT NULL DEFAULT 0,
    end_at        BIGINT NOT NULL DEFAULT 0,
    result_guild_id BIGINT NOT NULL DEFAULT 0,
    KEY guild_wars_window (start_at, end_at),
    KEY guild_wars_pair (attacker_id, defender_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS mail_items (
    mail_id       BIGINT NOT NULL,
    line_no       SMALLINT UNSIGNED NOT NULL,
    item_id       BIGINT NOT NULL DEFAULT 0,
    quantity      BIGINT NOT NULL DEFAULT 0,
    claimed       TINYINT(1) NOT NULL DEFAULT 0,
    PRIMARY KEY (mail_id, line_no),
    KEY mail_items_claim (mail_id, claimed)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS trade_sessions (
    trade_id      BIGINT AUTO_INCREMENT PRIMARY KEY,
    left_character_id  BIGINT NOT NULL,
    right_character_id BIGINT NOT NULL,
    state         VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'offered',
    left_gold     BIGINT NOT NULL DEFAULT 0,
    right_gold    BIGINT NOT NULL DEFAULT 0,
    created_at    BIGINT NOT NULL DEFAULT 0,
    accepted_at   BIGINT NULL,
    completed_at  BIGINT NULL,
    KEY trade_sessions_pair (left_character_id, right_character_id),
    KEY trade_sessions_state (state)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS trade_items (
    trade_id      BIGINT NOT NULL,
    side          TINYINT UNSIGNED NOT NULL,
    line_no       SMALLINT UNSIGNED NOT NULL,
    item_id       BIGINT NOT NULL DEFAULT 0,
    quantity      BIGINT NOT NULL DEFAULT 0,
    item_snapshot BLOB,
    PRIMARY KEY (trade_id, side, line_no)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS trade_pets (
    trade_id      BIGINT NOT NULL,
    side          TINYINT UNSIGNED NOT NULL,
    line_no       SMALLINT UNSIGNED NOT NULL,
    pet_id        BIGINT NOT NULL DEFAULT 0,
    pet_snapshot  BLOB,
    PRIMARY KEY (trade_id, side, line_no)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS world_bosses (
    boss_id       BIGINT AUTO_INCREMENT PRIMARY KEY,
    npc_id        BIGINT NOT NULL,
    scene_id      BIGINT NOT NULL DEFAULT 0,
    state         VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'scheduled',
    hp            BIGINT NOT NULL DEFAULT 0,
    hp_max        BIGINT NOT NULL DEFAULT 0,
    round_limit   BIGINT NOT NULL DEFAULT 20,
    starts_at     BIGINT NOT NULL DEFAULT 0,
    ends_at       BIGINT NOT NULL DEFAULT 0,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    KEY world_bosses_state (state, starts_at, ends_at)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS world_boss_participants (
    boss_id       BIGINT NOT NULL,
    character_id  BIGINT NOT NULL,
    damage        BIGINT NOT NULL DEFAULT 0,
    reward_state  VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'pending',
    joined_at     BIGINT NOT NULL DEFAULT 0,
    updated_at    BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (boss_id, character_id),
    KEY world_boss_participants_rank (boss_id, damage)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS player_events (
    character_id  BIGINT NOT NULL,
    event_id      BIGINT NOT NULL,
    scene_id      BIGINT NOT NULL DEFAULT 0,
    phase         VARCHAR(16) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'idle',
    choice        BIGINT NOT NULL DEFAULT 0,
    result        BIGINT NOT NULL DEFAULT 0,
    started_at    BIGINT NOT NULL DEFAULT 0,
    completed_at  BIGINT NULL,
    PRIMARY KEY (character_id, event_id),
    KEY player_events_scene (scene_id, phase)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS admin_users (
    admin_id      BIGINT AUTO_INCREMENT PRIMARY KEY,
    username      VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    password_hash VARCHAR(255) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    role          VARCHAR(32) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL DEFAULT 'operator',
    enabled       TINYINT(1) NOT NULL DEFAULT 1,
    created_at    BIGINT NOT NULL DEFAULT 0,
    last_login_at BIGINT NULL,
    UNIQUE KEY admin_users_username (username)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;

CREATE TABLE IF NOT EXISTS admin_audit_log (
    audit_id      BIGINT AUTO_INCREMENT PRIMARY KEY,
    admin_id      BIGINT NULL,
    action        VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin NOT NULL,
    target_type   VARCHAR(64) CHARACTER SET latin1 COLLATE latin1_bin,
    target_id     BIGINT NULL,
    details       JSON,
    created_at    BIGINT NOT NULL DEFAULT 0,
    KEY admin_audit_time (created_at),
    KEY admin_audit_target (target_type, target_id)
) ENGINE=InnoDB DEFAULT CHARACTER SET latin1 COLLATE latin1_bin;
