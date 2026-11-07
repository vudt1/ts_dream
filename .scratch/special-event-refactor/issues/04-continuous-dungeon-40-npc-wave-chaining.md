# 04: Continuous Dungeon In-Engine Wave Chaining for 40 NPC

**What to build:** From the player's perspective, challengers registering for 40 NPC (`Quest 218`) at map 12001 enter arena 10851 and battle through 40 consecutive waves of escalating NPC enemies within a single unbroken combat session, experiencing no scene reloading, screen flicker, or map warping between waves.

**Blocked by:** 01: Character God & Ghost State, Map Appearance, and Real-Time Synchronization

**Status:** resolved

- [x] Registration NPC 15162 at map 12001 validates entry prerequisites (deducts 1,000 gold or consumes item 46072 "Vé ưu đãi lôi đài") and warps the challenger to arena map 10851.
- [x] Initial wave 1 combat is launched with appropriate terrain and opponent data.
- [x] `BattleRunner` implements in-engine wave progression: upon wiping out all NPCs of wave $N$ ($1 \le N < 40$), defeated units are cleared and wave $N+1$ NPC records are populated directly into the isometric battle grid without emitting `battle_trailer` or returning to the map.
- [x] Clearing wave 40 triggers combat victory completion, distributes milestone rewards, and returns the player to the lobby.
- [x] Defeat in combat or client disconnection at any point immediately terminates the instance and evacuates the player to `savemap` on reconnect.
- [x] Integration tests in `tests/special_events_dungeon_test.rs` verify currency/ticket gating, unbroken session wave advancement from wave 1 to 40, and clean disconnection cleanup without orphan instances.

## Resolution
- Implemented and fully verified with integration tests (`tests/special_events_dungeon_test.rs`).
- Detailed handoff available at `.scratch/special-event-refactor/handoff-tickets-01-04-05.md`.
