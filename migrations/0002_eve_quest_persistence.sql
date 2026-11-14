-- ============================================================================
-- TS Dream — SQLite Migration 0002 (Eve & Quest Persistence Normalization)
--
-- 1. Rebuild `character_completed_events` with PK `(playerid, scene_id, eventid)`
--    so completion counters on different maps with the same `eve_no` never
--    collide or overwrite each other.
-- 2. Add `savemap_id`, `savemap_x`, `savemap_y` columns to `characters` for
--    persistent inn/respawn point storage.
-- 3. Drop obsolete `character_quest_items` table (all Item.dat items use 0x17
--    `inventories` Bag; 0x18 is exclusively the Mark.dat Quest Ledger).
-- ============================================================================

CREATE TABLE IF NOT EXISTS character_completed_events_v2 (
    playerid        INTEGER NOT NULL,
    scene_id        INTEGER NOT NULL DEFAULT 0,
    eventid         INTEGER NOT NULL,
    completedat     INTEGER DEFAULT 0,
    completioncount INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (playerid, scene_id, eventid)
);

INSERT OR IGNORE INTO character_completed_events_v2 (playerid, scene_id, eventid, completedat, completioncount)
SELECT playerid, 0, eventid, completedat, completioncount FROM character_completed_events;

DROP TABLE character_completed_events;
ALTER TABLE character_completed_events_v2 RENAME TO character_completed_events;

ALTER TABLE characters ADD COLUMN savemap_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE characters ADD COLUMN savemap_x  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE characters ADD COLUMN savemap_y  INTEGER NOT NULL DEFAULT 0;

DROP TABLE IF EXISTS character_quest_items;
