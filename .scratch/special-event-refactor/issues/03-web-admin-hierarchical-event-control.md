# 03: Web Admin 8090 Hierarchical Lifecycle, Server Gate, and Special Events Control Plane

**What to build:** From the operator's perspective, Web Admin (Port 8090) provides full control over the Game TCP Server (Port 6414) and the Special Events engine. Events can strictly only be triggered when the Game Server is running, duplicate triggers are prevented by strict mutexes, and stopping the Game Server initiates a cascading emergency drain across all active events before TCP shutdown.

**Blocked by:** 02: North Star Quiz In-Battle Encounter, Modal Q&A Protocol, and Anti-Spectate Lock

**Status:** completed

- [x] Central `EventLifecycleManager` is integrated directly into `ServerControl`, managing 5-phase FSM lifecycles (`Idle`, `Announcing`, `Active`, `Ending`, `Cooldown`) for all special event types.
- [x] Special event activation endpoints (`POST /api/events/{id}/start`) verify `ServerControl.is_running().await`; if false, returns `409 Conflict` with error message *"Game server is stopped. Please start the server first"*.
- [x] Triggering an event that is already in `Announcing` or `Active` state returns `409 Conflict` under the Strict Mutex policy.
- [x] Stopping the Game TCP Server (`POST /api/server/stop`) triggers an immediate cascading emergency drain across all active events (aborts combat via `S→C 00 2F` / `00 36`, clears temporary items, warps to `savemap`) prior to the 5-second network disconnect countdown.
- [x] Graceful stop endpoint (`POST /api/events/{id}/stop`) transitions an active event into a 3-minute drain period before terminating remaining instances.
- [x] Starvation watchdog enforces a maximum duration ceiling of 120 minutes on active events.
- [x] Web dashboard HTMX card on `:8090` provides an interactive Special Events management panel listing event statuses, participant counts, start/stop buttons, and a slider for North Star encounter rate (1% to 30%).
- [x] Every operator activation and stop records an entry into `admin_audit_log` with admin credentials and timestamps.
- [x] Integration tests in `tests/web_admin_events_test.rs` verify stopped server rejection gate, duplicate conflict responses, cascading emergency stop on TCP shutdown, and audit log persistence.
