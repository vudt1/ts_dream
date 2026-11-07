# 08: Seasonal Festivals & Static Dat Parity

**What to build:** From the player's perspective, seasonal festival activities, event calendar notifications, and festival interface dialogs open reliably and display historical festival information matching client static data.

**Blocked by:** 03: Web Admin 8090 Hierarchical Lifecycle, Server Gate, and Special Events Control Plane

**Status:** ready-for-agent

- [ ] Static data loader loads `Data/Festival.Dat`, parsing festival schedule records and title metadata.
- [ ] Festival UI trigger frame `S→C 24 19` emits date DWORD `19000110` (`0x0121EAC6`, matching client `Festival.Dat` timestamp `11.0`) to trigger `Festival.Dat` client form `*gvar_007DA1EC`.
- [ ] Seasonal festival broadcaster emits 74 seasonal announcements via `S→C 17 39` based on active festival calendar events.
- [ ] Integration tests in `tests/festival_quiz_test.rs` verify `Festival.Dat` date DWORD encoding parity and `17 39` announcement payload formats.
