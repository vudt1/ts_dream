# 06: Competitive Arena Tournaments, Scoreboards, and 7-Day Timed Title Revocation

**What to build:** From the player's perspective, competitive tournaments across Solo PK, Team 5v5 Đoàn P, Loạn Đấu, and Tiên Đấu feature global event broadcasts, real-time in-game scoreboards, and automatic weekly revocation of timed championship titles and rings upon reset.

**Blocked by:** 03: Web Admin 8090 Hierarchical Lifecycle, Server Gate, and Special Events Control Plane

**Status:** completed

- [x] Global alert broadcaster emits `S→C 25 03 [noticeCode: u8]` (codes 0x01..0x0A) for tournament preparation, match commencement, and conclusion across the 5 modes.
- [x] Match status updates broadcast `S→C 14 28 [state: u8]` (1 = start, 2 = end).
- [x] Tournament scoreboards encode dynamic arrays of 7-byte records (`mode`, `flag`, `x`, title string) via `S→C 24 09` and open the scoreboard interface via `S→C 24 0A`.
- [x] Weekly reset logic at Monday 00:00 scans active champions and emits `S→C 17 6F [01..03]` to revoke expired 7-day timed items (`Nhẫn loạn đấu`, `Nhẫn đoàn P`, `Văn chương vô giới`) if tenure has lapsed or ranking is lost across `homdo`, `tuideo`, and `trangbi` (with `recompute_stats()`).
- [x] Point exchange under 50 points triggers `S→C 17 6E`.
- [x] Integration tests in `tests/arena_test.rs` verify notice frame encoding, scoreboard dynamic array payloads, point exchange rejection, and weekly item revocation on Monday reset (including equipped and bag slots).

## Resolution
- Implemented handler logic in `src/server/handlers/arena.rs` and integrated into server handlers.
- Comprehensive Monday 00:00 reset scanning all 3 item containers (`trangbi`, `homdo`, `tuideo`) with stats recalculation and wire emission.
- Verified with 6 integration tests in `tests/arena_test.rs` (100% PASS).
