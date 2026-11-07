# 01: Character God & Ghost State, Map Appearance, and Real-Time Synchronization

**What to build:** From the player's perspective, characters can hold active God of Fortune (Thần Tài) and Ghost (Con Ma / Thần xui) statuses that correctly render over the avatar on map load, persist across relogins and map transitions, update in real time with client audio/toast feedback, and modify experience gains during PvE combat.

**Blocked by:** None (can start immediately).

**Status:** resolved

- [x] SQLite schema updates `characters` table with persistent columns `ghost INTEGER NOT NULL DEFAULT 0` and `god INTEGER NOT NULL DEFAULT 0`.
- [x] In-memory session state exposes `pub ghost: u8` alongside `pub god: u32`, hydrated from the database upon character selection.
- [x] Map appearance frame `player_appear` (`0x03`) transmits real-time values for `s.ghost` at byte offset 5 and `(s.god > 0) as u8` at byte offset 6 instead of hardcoded zeroes.
- [x] Real-time synchronizer frame builder `S→C 18 08 [playerId: u32 LE][type: u16 LE][value: u8]` emits status changes for Ghost (`type = 1`) and God (`type = 2`).
- [x] Interacting with NPC 1752 ("Bà Gọi Hồn") with at least 500 gold deducts 500 gold, clears the Ghost status (`ghost = 0`), emits `S→C 18 08 [playerId][01 00][00]`, triggering the client to play `sound\WA0006.wav` and display toast *"Thần xui đã rời xa bạn!"*. If gold is insufficient, the NPC refuses exorcism.
- [x] Consuming Item 46180 ("Cải vận phù") while ghosted consumes the item, clears the Ghost status, emits `S→C 18 08 [playerId][01 00][00]`, plays `sound\WA0006.wav` and displays the toast. Using it while not ghosted rejects consumption.
- [x] Combat formula in PvE battles applies a 1.5x experience multiplier when `god > 0`, decrementing `god` by 1 upon victory; when `god` reaches 0, emits `S→C 18 08 [playerId][02 00][00]`.
- [x] Combat formula in PvE battles applies a 0.5x experience multiplier (50% reduction) when `ghost == 1`.
- [x] Integration tests in `tests/festival_quiz_test.rs` verify appearance byte layout, persistent relogin state, item and NPC exorcism with fee validation, and combat EXP formula modifiers.

## Resolution
- Implemented and fully verified with unit & integration tests (`tests/festival_quiz_test.rs`).
- Detailed handoff available at `.scratch/special-event-refactor/handoff-tickets-01-04-05.md`.
