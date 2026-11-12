# 10: North Star Quiz Live Dispatcher Routing, HP=1 Attribute Parity, Offline Timeout Persistence & Map Scope Control

**What to build:** From the player's and operator's perspective, encountering Bắc Tinh Quân in the wild and submitting an answer over a live TCP connection routes end-to-end through the packet dispatcher. Answering incorrectly or letting the 45-second timer expire reduces the leader's current HP to 1 using the exact client HP attribute code (`0x19` / `25`) and persists both `ghost = 1` and `hp = 1` to SQLite even if the player disconnected before timeout. Operators can configure the North Star map scope allowlist from Web Admin (defaulting to all wild maps while always excluding special dungeon/event maps), and players receive a 3-minute warning broadcast when an active event begins draining.

**Blocked by:** 09: Event Domain Prefactoring, Opcode Constants & Natural Multi-Session Test Flow

**Status:** done

- [x] Level-1 packet dispatcher routes `C→S 0x10` (sub-opcode `0x01`) frames to the North Star Quiz answer handler, verified by an end-to-end dispatcher test (not calling the handler function directly).
- [x] Wrong-answer and 45-second timeout penalty packets emit the current HP status update using attribute `0x19` (`25`, value `1`) rather than attribute `1`.
- [x] Active quiz state tracks the leader's persistent database character identifier (`db_character_id`) so that when the 45-second countdown expires after the player has gone offline, `ghost = 1` and `hp = 1` are reliably written to SQLite and retained upon relogin.
- [x] `EventLifecycleManager` maintains `north_star_allowed_maps: Vec<u16>` exposed via `POST /api/events/north_star/maps` (comma-separated map IDs; empty = allow all wild maps). Wild movement always excludes special dungeon/event maps (`10851..=10852`, `12911..=12928`, `49960..=49969`, `60510..=60521`) and, when the allowlist is non-empty, only triggers encounters on maps in `north_star_allowed_maps`.
- [x] Stopping an active event from Web Admin (transitioning from `Active` to `Ending`) broadcasts a 3-minute warning system message (`020102`) to online participants before the drain window closes.
- [x] Positive and negative integration tests verify live dispatcher routing, `0x19` HP=1 wire frame parity, offline timeout SQLite persistence across relogin, map scope filtering (both excluded dungeon maps and explicit allowlist), and the 3-minute drain broadcast.
