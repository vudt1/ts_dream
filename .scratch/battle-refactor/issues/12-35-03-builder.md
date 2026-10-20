# 12: Builder 35 03 (ép state animation)

> **Cập nhật 2026-10-17 (Đính chính Dispatcher Opcode 0x35):** Dispatcher chính `0078a89c` tra bảng byte `byte[opcode + 0x78A8EE]` → `0x35` là Case 46 (`FUN_007952ab`), Case 53 là Opcode `0x3C`. `35 0E` (`FUN_0064fbd8`) và `35 0F` (`FUN_0064fd34`) **có handler thật trong client** (cùng họ `0064xxxx` với `35 01/03/04/05/07/0C`). Đã xây dựng đầy đủ builder cho `35 03`, `35 0E`, `35 0F` và `35 01` trong `packets.rs` cùng unit test trong `tests/battle_status_frames_test.rs`. Việc nối vào luồng battle runner chờ kết quả capture runtime của ticket 14.

**What to build:** Thêm builder còn thiếu của nhóm `35`:
- `35 03` = `[35][03][grp][slot]` (ép state 7 — animation tấn công, theo `FUN_0064cfec`).
- `35 0E` = `[35][0E][grp][slot][val_u16_le]` (unit action vtable 0x1c, theo `FUN_0064fbd8`).
- `35 0F` = `[35][0F][grp][slot][char][flag][val_u16_le]` (unit HUD status buff/debuff slots 'B'..'G', theo `FUN_0064fd34` & `FUN_0058faf0`).
- `35 01` = `[35][01][grp][slot][purge_type][0000]` (xóa status effects, khớp Bear C# và client `FUN_0064cd74`).

**Bảng sub Opcode 0x35 đã xác minh (Case 46 / `FUN_007952ab`):**

| sub | handler | nội dung |
|---:|---|---|
| `0x01` | `FUN_0064cd74` | Purge / clear status effects (1=disable, 2=buff, 3=debuff, 5=aura) |
| `0x03` | `FUN_0064cfec` | Ép animation state 7 (tấn công) |
| `0x04` | `FUN_0064d128` | Hiệu ứng rơi item trên map (`giveDrop`) |
| `0x05` | `FUN_0064dfa4` | Đang hành động (`acting`) |
| `0x07` | `FUN_0064e3e0` | Di chuyển ô lưới (`move_cell`) |
| `0x0E` | `FUN_0064fbd8` | Unit action / vtable 0x1c (`unit_action`) |
| `0x0F` | `FUN_0064fd34` | Cập nhật slot HUD buff/debuff ('B'..'G') & tooltip overlay |

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Builder `35 03` byte-exact (`[35][03][grp][slot]`, payload 4 byte) theo `FUN_0064cfec`
- [x] Builder `35 0E` byte-exact (`[35][0E][grp][slot][val:2]`, payload 6 byte) theo `FUN_0064fbd8`
- [x] Builder `35 0F` byte-exact (`[35][0F][grp][slot][char][flag][val:2]`, payload 8 byte) theo `FUN_0064fd34`
- [x] Builder `35 01` byte-exact (`[35][01][grp][slot][type][0000]`, payload 7 byte) theo `FUN_0064cd74`
- [x] Test trong `tests/` (`tests/battle_state7_frame_test.rs`, `tests/battle_status_frames_test.rs`)
- [x] Comment chi tiết trong code (`src/battle/packets.rs`) giải thích các loại buff/debuff 'B'..'G' và quan hệ với Bear C#

---

> **Kết quả (2026-10-17):** builder `packets::force_state7(row, col)` = `F44404003503{grp}{slot}` (payload/LEN 4 byte, frame 8 byte) đặt ngay sau `move_cell`; chưa có call-site nào (wiring chờ ticket 14). Test mới `tests/battle_state7_frame_test.rs` (5 test). `cargo test --all-targets --no-fail-fast` → 297 passed / 0 failed.
>
> **⚠️ Phát hiện cần review (mâu thuẫn bằng chứng, đã verify trên binary):** bảng sub ở `HANDOFF-client-reverse.md §14.1`/`§14.4` được suy ra từ `case_053_0079575C` nhưng **case index ≠ opcode**. Dispatcher chính `0078a89c` tra bảng byte `byte[opcode + 0x78A8EE]` rồi mới `JMP [index*4 + 0x78A9B6]` (`0078a89c.asm.txt:27-31`). Kiểm trực tiếp `client_delphi/aLogin.exe`: `byte[0x0B]=11`, `byte[0x32]=43`, `byte[0x33]=44`, `byte[0x34]=45`, `byte[0x35]=46`; `jump[46]=0x007952AB`, `jump[53]=0x0079575C`; `byte[0x3C]=53` ⇒ bảng sub `1..4 → 00541098/005408f4/0053fd78/0053f8fc` là của **opcode `0x3C`**, không phải `0x35`. Dispatcher thật của `0x35` là `case_046 → FUN_007952ab`, `switch` có route `0x0E → FUN_0064fbd8` và `0x0F → FUN_0064fd34` (cùng họ `0064xxxx` với `35 01/03/04/05/07/0C`). Hệ quả: `35 0E`/`35 0F` **không** phải no-op như ticket giả định — vẫn giữ nguyên phạm vi ticket (không ship builder cho 2 sub này, đã ghi chú trong code), nhưng cần ticket 13/14 xác nhận lại. Cũng vì lý do trên, `FUN_0064cfec` (không có caller trong export) mới đúng là handler `35 03`, khớp `02_battle_positions.md §2.7`.