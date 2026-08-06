# 05 — Golden harness: capture proxy + runner + golden skeleton

**What to build:** Bộ công cụ chấp nhận capture-based byte-level (Ch9) ship trong repo: capture proxy ghi traffic hex-after-XOR, định dạng golden file, và cargo integration test runner nối server Rust so sánh frame-by-frame. Runner mở khung chạy được placeholder; các scenario thực sẽ được fill từ ticket 23.

**Blocked by:** 02 — Protocol framing + encoders (runner cần cùng quy ước frame/XOR); 03 — Static data load (runner test dựa trên server đã nạp data).

**Status:** completed

- [x] Capture proxy TCP (client → proxy → C# server), log cả 2 chiều thành plaintext hex sau XOR, split frame trên `F444` + length, theo convention `Data/packet.txt`.
- [x] Golden file format `//` comment, `<<` client→server, `>>` server→client, line = 1 frame, blank line nhóm; **không binary** (Ch9 §9.3).
- [x] Golden runner (cargo integration test): đọc golden file, gửi `<<` C2S theo thứ tự, thu thập `>>` S2C, diff **frame-by-frame, byte-exact** — khác gì đều fail.
- [x] Mô hình C2S/S2C là 2 luồng tuần tự, độc lập; scenario phải deterministic (loại frame phụ thuộc timing như 020C timer) (Ch9 §9.2/§9.5).
- [x] Hằng số harness khớp Ch8: XOR 0xAD, magic F4 44, prefix `vn`, version 186, name TSVN.
- [x] Golden placeholder/runner chạy được: 14 golden scenario trong `golden/`; các scenario thật về sau (ticket 23).
- [x] Runner sử dụng `handler::dispatch` **async** qua `ServerEnv::none()` (không pool/hub) cho replay in-process; mọi scenario byte-exact qua `tests/golden_suite.rs` mà không cần socket/DB/wall-clock.

## Implementation notes

- `Scenario::replay`/`save`/`to_golden_text`/`regenerate` trở thành **async** (dispatch giờ async để chứa DB path — ticket 06/07); test `regenerate_goldens` chạy bằng `#[tokio::test]`
- Golden files cho tới nay là **synthetic** (regenerate từ chính handler Rust) — chưa bắt capture thật từ C# qua proxy. `04-login-success` mới được regenerate do `Logined1` chuyển từ literal sang session-driven (sửa luôn bug parity gold/store frame). Chờ capture thật để rà length frame (self-appear constant 33 vs `frame()` hiện tại).
- Runner socket thật (`GOLDEN_ADDR`) vẫn là cổng tuỳ chọn; gate mặc định (không DB) là `golden_scenarios_replay_byte_exact`.