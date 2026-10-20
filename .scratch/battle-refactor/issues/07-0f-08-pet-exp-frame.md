# 07: Frame 0F 08 exp thú + sửa lệch layout pet_stat_entry

> **Cập nhật 2026-10-16 (HANDOFF §6, G9 ĐÃ ĐÓNG TĨNH):** layout `0F 08` đã re-derive đầy đủ từ `007a5010`; **stride record = `nameLen + 0x63`**. Rust lệch 2 chỗ: `le32(id)` và tail 12 byte thay vì 6 byte.

**Layout client (đích trong object thú = `player+0x57C[stt]`, mức A):**

| offset trong record | size | đích client | Rust field |
|---|---:|---|---|
| `+0x00` | 1 | index `stt` (≤ 4) | `p.stt` |
| `+0x01..0x02` | 2 | `FUN_0071c3a0(player, stt, v, 0)` | `le16(p.id)` ← **sửa** |
| `+0x03..0x06` | 4 | `pet+0x3FC` (EXP) | `le32(p.texp)` ← **dời lên** |
| `+0x07` | 1 | `pet+0x3FA` (LEVEL) | `p.level` |
| `+0x08..0x09` | 2 | `pet+0x3EA` (HP) | `p.hp` |
| `+0x0A..0x0B` | 2 | `pet+0x3EC` (SP) | `p.sp` |
| `+0x0C..0x0D` | 2 | `pet+0x3EE` | `p.int1` |
| `+0x0E..0x0F` | 2 | `pet+0x3F0` | `p.atk` |
| `+0x10..0x11` | 2 | `pet+0x3F2` | `p.def` |
| `+0x12..0x13` | 2 | `pet+0x3F4` | `p.agi` |
| `+0x14..0x15` | 2 | `pet+0x3F6` | `p.hpx` |
| `+0x16..0x17` | 2 | `pet+0x3F8` | `p.spx` |
| `+0x18` | 1 | `pet+0x55D` | `00` (reserved) |
| `+0x19` | 1 | `pet+0x55E` | `p.fai` |
| `+0x1A` | 1 | `pet+0x55F` | `p.quest` |
| `+0x1B..0x1C` | 2 | `pet+0x400` | `le16(p.skill_point)` |
| `+0x1D` | 1 | — | `name.len()` |
| `+0x1E..` | n | `pet+9` (tối đa 0x11 ký tự) | `p.name` |
| `+0x1E+n .. +0x20+n` | 3 | `pet+0x561..0x563` | `lv_skill[0..2]` |
| `+0x21+n + 10*(i-1)`, `i=1..6` | 10 | worker `FUN_0072fa98` (`[id:u16][b][b][b][value:u32]`) | `encode_entry_10(slot = stt*10 + i)` |
| `+0x5D+n` | 1 | `pet+0x565` | `lv_skill[3]` ← **dời từ cuối lên đây** |
| `+0x5E+n` | 1 | `pet+0x566` | `00` |
| `+0x5F+n .. +0x62+n` | 4 (bound ≤ 0xFFFF) | `pet+0x568` | `00 00` + 2 byte dự phòng |

`n + 0x63` byte/record ⇒ `pet_stat_entry` phải dài đúng `99 + name.len()`.

**What to build:** (a) sửa `pet_stat_entry` (`src/server/spawn.rs:422-464`) theo bảng trên — `le16(id)` + `le32(texp)`, tail 6 byte; giữ nguyên đường `pet_summary`/`pet_status_single`; (b) phát `0F 08` cho chủ khi thú nhận exp sau trận (`apply_pet_exp` hiện chỉ cộng session).

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] `pet_stat_entry` dài đúng `99 + name.len()`, đúng offset mọi field trong bảng (assert trong test)
- [x] Test: dựng entry từ `PetState` mẫu → parse lại theo offset client, kiểm `id`/`texp`/`level`/`hp`/`sp`/tên/6 entry 10 byte/tail 6 byte
- [x] Test: hai thú trong cùng frame phải cách nhau đúng `99 + len(name_1)`
- [x] `apply_pet_exp` phát `0F 08` cho chủ thú (không chỉ cộng session)
- [x] Test nhánh `13 02` (client recall thú chết sau trận — bằng chứng `00644a18:60` + `asm:70-72`; Rust đã có handler ở `pet_actions.rs` sub 2): `13 02` khi có active pet ⇒ `active_pet_stt = 0` + echo `F44402001302`; khi `active_pet_stt == 0` thì im lặng
- [x] Không hồi quy test pet summary / equip hiện có (các test đang assert `0F08` — cập nhật kỳ vọng theo layout mới)
- [x] Test mới trong `tests/`