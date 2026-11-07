# 07: Guild Warfare: City Siege, Water War, and Dragon Boat Racing

**What to build:** From the player's perspective, guilds can participate in full-scale territory wars across 5 siege cities, naval battles with dedicated boat inventories and cannon bombardments, and dragon boat racing with anti-macro input pacing.

**Blocked by:** 03: Web Admin 8090 Hierarchical Lifecycle, Server Gate, and Special Events Control Plane

**Status:** ready-for-agent

- [ ] Relational SQLite schema models guilds (`guilds`, `guild_members`, `guild_ranks`, `guild_logs`, `guild_cities`) with denormalized `guild_id` on `characters` and in-memory session caching.
- [ ] Guild wire protocol handles `0x27` sub-opcodes (creation, invite, kick, leadership transfer, 288-byte badge upload) with Two-Phase State Locking on two-player operations.
- [ ] City Siege (`0x29`) manages 5 cities (Thọ Xuân, Trường An, Giang Lăng, Khúc A, Lạc Dương), syncing city ownership via table `S→C 29 0F` and 13 warfare announcements via `29 09`.
- [ ] City gate collision barriers are broadcast via `0x03` so that intact gates block attacker traversal.
- [ ] Water War (`0x3F`, `0x40`) initializes a temporary 25-slot boat inventory (`S→C 17 64`) upon entry and clears temporary items (`S→C 17 65`) on map departure.
- [ ] Naval bombardment action `C→S 40 01 [action, targetId, param]` synchronizes damage and boat visual effects via `S→C 40 01..03`.
- [ ] Dragon Boat Racing (`0x46`) accepts alternating Left/Right paddle inputs `C→S 46 01` on maps 60510..60521, enforcing a token bucket rate limiter dropping packets exceeding 5/second.
- [ ] Integration tests in `tests/guild_war_test.rs` verify guild creation/invite 2-phase locks, city siege table encoding, boat inventory lifecycle, and boat race rate limiting.
