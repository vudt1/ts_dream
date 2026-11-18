-- ============================================================================
-- TS Dream — SQLite Migration 0003 (Seasonal Festival Lifecycle & Claims)
--
-- 1. `seasonal_festival_state`: persists manual Start/Stop state and monotonic
--    `season_seq` counter across server restarts.
-- 2. `character_festival_claims`: tracks per-character per-period claims
--    (e.g. `season-<seq>` for 0x24 0x19 lore popup, `YYYY-MM-DD` for daily drops).
-- ============================================================================

CREATE TABLE IF NOT EXISTS seasonal_festival_state (
    festival_id  TEXT PRIMARY KEY,
    is_active    INTEGER NOT NULL DEFAULT 0,
    season_seq   INTEGER NOT NULL DEFAULT 0,
    started_at   INTEGER,
    updated_at   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS character_festival_claims (
    playerid     INTEGER NOT NULL,
    festival_id  TEXT NOT NULL,
    period_key   TEXT NOT NULL,
    claim_type   TEXT NOT NULL,
    claim_count  INTEGER NOT NULL DEFAULT 1,
    claimed_at   INTEGER NOT NULL,
    PRIMARY KEY (playerid, festival_id, period_key, claim_type)
);
