# 02 — Protocol framing + primitive encoders + VISCII wire path

**What to build:** Tầng gọi là xử lý frame chính xác từng byte theo giao thức C#: một socket TCP đọc/ghi frame đúng định dạng, decode XOR 0xAD + hex split, encode gửi đi pure-transform. Nền tảng cho mọi handler và harness.

**Blocked by:** 01 — Scaffold dự án + config + startup sequence.

**Status:** done

- [x] Receiver: đọc buffer 8192B, XOR 0xAD từng byte → uppercase hex; length field = hex tại offset 4 (chars 4..7, little-endian u16) = byte count sau 4-byte header; frame = `4 + length` bytes = `8 + length*2` hex chars.
- [x] Ghép nối frame concatenated trên wire: split loop, buffering partial trailing frame, prepend vào chunk kế.
- [x] Send path: pure transform — build hex-string packet → hex-decode → XOR 0xAD từng byte → một `write`. **Không checksum, không trailer.**
- [x] Primitive encoders chính xác: `le16`, `le32`, `u16_le`, `u32_le`, `hex`, `bytes`, `xor01`, `strhex` (low byte `& 0xFF`). Name-length fields = byte counts.
- [x] Receive 0-byte → `shutdown()`; nếu session id > 0 broadcast leave-battle + offline frames (Ch2 §2.1).
- [x] VISCII wire invariance: không transcode sang UTF-8; mỗi byte 0x00–0xFF truyền verbatim (Ch4 §4.1).

## Triển khai (review G1–G6, session 2026-08-05)

- **G1 — disconnect broadcast**: `ServerControl::disconnect_player` (`src/web/server_control.rs`) — broadcast `F44408000B00`+LE32(id)+`0000` (hide/leave frame, research 04 §7.4) tới peers qua `broadcast_except`, rồi unregister + xóa online snapshot. Connection loop gọi chung trong teardown cho mọi đường thoát (0-byte, read/write error, handler shutdown).
- **G2 — test `Decoder::feed`**: 8 unit test trong `src/protocol/frame.rs` (single, concatenated, partial-across-chunks, mid-length-field split, partial trailing retained, empty, 50-frame chunk, `check_magic`).
- **G3 — `check_magic`**: được wire vào receive loop; frame không có magic `F4 44` bị drop + warn (dead code trước đây).
- **G4 — teardown hợp nhất**: loop cũ `return` ngay khi `out.shutdown` (bỏ qua cleanup); nay dùng cờ `close` → một nhánh teardown chung cho mọi lối thoát.
- **G5 — server-authored text**: `sys_msg_frame`/`announce_frame`/`server_name_frame` map qua `to_viscii` thay vì `strhex(msg.as_bytes())` (UTF-8). ASCII identity (golden không đổi); ≤0xFF Latin-1 một byte; char proper-Unicode >0xFF **hiện collapse về `'?'` 0x3F** (vd. `Đ` U+0110) — chưa đúng với `smethod_17` (vốn map `Đ→0xD0`), cần bảng đầy đủ thuộc scope **ticket 03**. Hợp đồng hiện tại chỉ đảm bảo "không bao giờ leak UTF-8 raw lên wire". `use_item.rs` bỏ hàm `spawn_sys_msg` trùng lặp → dùng `spawn::sys_msg_frame`.
- **G6 — hằng số**: `harness.rs` re-export `XOR_KEY/MIN_VERSION/SERVER_NAME/ID_PREFIX` từ `protocol::`; `FRAME_MAGIC` = `&protocol::MAGIC`.

**Còn lại ngoài scope ticket này:** frame xoá cell grid khi disconnect trong battle (`F44404000B01`+pet cell, `F44405000B01`+row+col+`00`) thuộc battle engine (ticket 21); bảng `smethod_17` đầy đủ (proper-Unicode → VISCII, gồm `Đ→0xD0`) thuộc ticket 03 — G5 hiện chỉ chặn leak UTF-8, chưa phải VISCII chuẩn cho text proper-Unicode.

**Verify:** `cargo test` — 215 unit tests (lib) + battle_golden/data/golden/golden_suite/web_dashboard đều xanh; không warning mới (clippy cảnh báo cũ giữ nguyên).
