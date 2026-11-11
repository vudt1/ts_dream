# 08: Seasonal Festivals & Static Dat Parity

**What to build:** From the player's perspective, seasonal festival activities, event calendar notifications, and festival interface dialogs open reliably and display historical festival information matching client static data.

**Blocked by:** 03: Web Admin 8090 Hierarchical Lifecycle, Server Gate, and Special Events Control Plane

**Status:** completed

- [x] Static data loader loads `Data/Festival.Dat`, parsing festival schedule records and title metadata using Delphi dual XOR key decryption (`keyA` record 8 for even records, `keyB` record 9 for odd records 1..=7).
- [x] Festival UI trigger frame `S→C 24 19` emits date DWORD `19000110` (`0x0121EAC6`, matching client `Festival.Dat` timestamp `11.0`) to trigger `Festival.Dat` client form `*gvar_007DA1EC`.
- [x] Seasonal festival broadcaster emits 74 seasonal announcements via `S→C 17 39` based on active festival calendar events.
- [x] Integration tests in `tests/festival_quiz_test.rs` verify `Festival.Dat` 7 decrypted records, date DWORD encoding parity, and `17 39` announcement payload formats.

## Resolution
- Implemented static loader `src/data/loaders/festival.rs` with exact Delphi `FUN_0051e00c` + `FUN_0051decc` XOR decryption algorithm, decoding `11.0` TDateTime, `0xB4C2` flag, and VISCII titles.
- Handled UI triggers and announcements in `src/server/handlers/festival.rs`.
- Verified with integration tests in `tests/festival_quiz_test.rs` (18/18 tests PASS).
