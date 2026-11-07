# 05: Tower Climbing Progression for Rescue 2K & 9 Tiers of Cửu Thiên

**What to build:** From the player's perspective, teams can undertake the 18-floor climb of Đồng Tước Đài to rescue Nhị Kiều (Quest 212) with party size validation, and solo challengers can scale the 9 tiers of Cửu Thiên (Quest 12309..12319) with companion follower Hầu Tôn, checkpoint bosses, weekly entry quotas, and floor treasure chests.

**Blocked by:** 01: Character God & Ghost State, Map Appearance, and Real-Time Synchronization

**Status:** resolved

- [x] Rescue 2K entry at Diệp Thành 12003 NPC 15312 (Tả Từ) verifies party prerequisites (strictly 1 or 5 members) and transitions the team across 18 tower floors (`12911..12928`).
- [x] Climbing floors evaluates `QuestStep(212)` sequentially across 18 checkpoint fights, culminating in rescuing Đại Kiều (`12927`) and Tiểu Kiều (`12928`) to award feather 23086, bean 26051, and credentials 46049/46050.
- [x] Broadcast alerts `14 29 01` (registration closed), `14 29 02` (event started), and `14 29 03` (event ended) synchronize tournament state.
- [x] Chinh Phục Cửu Thiên entry at 33033 NPC 61209 (Hầu Tôn) verifies companion follower `FollowPet(14735)` and weekly quota `QuestStep(12310 == 0)`.
- [x] Re-entry after weekly completion requires consuming Item 46644 ("Cửu Thiên Chiến Kỳ") to reset quota.
- [x] Floor progression across maps 49960..49969 evaluates 18 checkpoint boss encounters and floor treasure chest interactions (`NPC 20001`) distributing floor rewards.
- [x] Immediate combat termination and evacuation to `savemap` is enforced on disconnect.
- [x] Integration tests in `tests/special_events_dungeon_test.rs` verify 2K party size gating and reward distribution, Cửu Thiên follower/quota checks, and flag resets via Item 46644.

## Resolution
- Implemented and fully verified with integration tests (`tests/special_events_dungeon_test.rs`).
- Detailed handoff available at `.scratch/special-event-refactor/handoff-tickets-01-04-05.md`.
