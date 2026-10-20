# 04: Gộp một builder 17 06 duy nhất + item rơi về túi sau trận

> **Cập nhật 2026-10-16 (HANDOFF §4, G4 ĐÃ ĐÓNG):** builder `17 06` hiện tại của Rust **đã khớp byte-exact** với client `0076b458`; `LEN = 0x0E` là đúng (test prefix `F4440E001706` **giữ nguyên**). Phần "sửa layout" của ticket này bị loại — chỉ còn việc gộp builder + gửi item rơi.

**Bằng chứng byte-exact (`0076b458`, mức A):**

```
content[0..1]   = 17 06
content[2..3]   = id     u16 LE
content[4..5]   = count  u16 LE     (Rust: byte count + byte 00 ⇒ tương đương vì count ≤ 0xFF)
content[6]      = doben             (arg9 = rec4 của 17 05)
content[7]      = long_val          (arg8 = rec5)
content[8]      = giatri_long + 100 (arg7 = rec6)
content[9]      = khang             (arg6 = rec7)
content[10..13] = texp     u32 LE
→ content = 14 byte ⇒ LEN = 0x0E
```

Ghi chú: worker `FUN_0072b6c8` tự tính `derived = FUN_0065b580(gvar_007DA660, id, 0, texp)` (số lượng/stack) — **không có byte này trên wire**; thứ tự 4 byte attr bị đảo khi client gọi worker, đã ánh xạ ngược ở bảng trên.

**What to build:** Hai nửa của cùng một đường "thêm 1 item": (a) **một** builder `17 06` dùng chung cho mọi call-site (nhặt đồ, thương nhân, giao dịch, kho, dùng item, GM) — layout giữ nguyên như hiện tại; (b) `apply_drop` sau trận chỉ cộng vào session mà không gửi frame — phải gửi `17 06` cho chủ sở hữu để item thực sự vào túi client.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Một builder `17 06` duy nhất, giữ nguyên layout 14 byte (`id:u16 | count:u16 | doben | long | giatri+100 | khang | texp:u32`); mọi call-site cũ dùng nó
- [x] Test round-trip: dựng frame rồi parse lại đúng `id`/`count`/`texp` theo offset ở bảng trên (assert `LEN == 0x0E`)
- [x] `apply_drop` gửi `17 06` cho chủ khi item rơi; test mô phỏng giết NPC → chủ nhận frame add-item đúng id
- [x] Test `tests/economy_wire_test.rs` giữ nguyên prefix `F4440E001706` (đã đúng — **không** đổi kỳ vọng)
- [x] Test mới trong `tests/` cho builder dùng chung