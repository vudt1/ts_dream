# Handoff: `sys_msg_frame` (`02 0C`) / `announce_frame` (`02 00`) & Khử Self-Deadlock `ServerControl` trên Map `10817`

- **Ngày**: 2026-11-18
- **Trạng thái**: Đã triển khai xong toàn bộ thay đổi và pass **687/687** test (`cargo test --all-targets --no-fail-fast`); thay đổi giữ nguyên ở working tree để user tự commit (`AGENTS.md`).

---

## 1. Bối cảnh & Tài liệu tham chiếu

Không lặp lại chi tiết nghiên cứu/thiết kế đã ghi đầy đủ ở các tài liệu sau:
- **Nghiên cứu gốc & giải mã 3 gói SSE**: [research.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/research.md)
- **Kế hoạch triển khai đã phê duyệt**: [implementation_plan.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/implementation_plan.md)
- **Walkthrough thay đổi & kết quả test**: [walkthrough.md](file:///C:/Users/tuan.vu/.gemini/antigravity-cli/brain/697c218c-de3d-4304-bb11-198f4d020444/walkthrough.md)
- **Chuẩn mực & ADR liên quan**: [CODING_STANDARDS.md](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L36-L41), [ADR 0007](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0007-compound-wire-parity-and-crafting-engine.md#L36), [ADR 0010](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0010-warp-door-sceneinfo-and-battlegate.md).

---

## 2. Tóm tắt thay đổi trong Working Tree

### Source & Tài liệu (`src/`, `CODING_STANDARDS.md`, `docs/`)
- [`src/server/spawn.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L346-L363): `sys_msg_frame` chuyển sang `02 0C <visc>` (`CHAT_SUB_SYSTEM`, không 4B `id`); `announce_frame` chuyển sang `02 00 00000000 <visc>` (`CHAT_SUB_BROADCAST`); xóa `text_banner`.
- [`src/server/handlers/use_item/mod.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs#L176): cập nhật doc comment sang `020C`.
- [`src/server/session.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs#L132-L422): thêm `Session.sync_revision: u64` (runtime-only), `#[derive(PartialEq)]` cho `PetState`/`ShopItem`/`PlayerShopState`/`TradeState`, cập nhật `registry_update_if_online`, và thêm `merge_post_dispatch(base, local, remote)`.
- [`src/web/server_control.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L878-L995): bỏ outer `lock_player_operations` quanh `dispatcher::dispatch(...)` để triệt tiêu self-deadlock (tại `14 08`, Bạch Lão Nhân `15009`, và các handler Two-Phase State Lock); đồng bộ `conn.session <-> online_sessions` dưới khóa ngắn trước/sau `dispatch()` với `merge_post_dispatch`.
- [`src/data/loader.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L359-L386) & [`src/main.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/main.rs#L119-L129): thêm `tracing::warn!` khi thiếu/rỗng `eve.emg` và log `eve_scenes`, `quest_marks`, `eve_events` lúc startup.
- [`CODING_STANDARDS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L36-L41) & [`docs/adr/0007-compound-wire-parity-and-crafting-engine.md`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0007-compound-wire-parity-and-crafting-engine.md#L36): cập nhật chuẩn `020C` / `0200`.

### Tests (`tests/`)
- Thêm [`tests/server_network/eve_tcp_and_sysmsg.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/eve_tcp_and_sysmsg.rs) (khai báo trong [`tests/server_network_test.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network_test.rs#L15-L16)) kiểm thử wire parity `sys_msg_frame`/`announce_frame`, `merge_post_dispatch`, và TCP end-to-end không deadlock cho `14 08 01 00` và Bạch Lão Nhân (`15009`) trên map `10817`.
- Cập nhật `sync_revision: 0` trong `tests/db/repository_init.rs` và chuyển các assertion `"020102"` / `"0201"` sang `"020C"` (cùng `"020000000000"` cho `announce_frame`) trong `tests/server_network/chat.rs`, `tests/economy_inventory/economy_wire.rs`, `tests/battle/pvp_party_and_flee.rs`, `tests/events_arena/{festival_gating_and_evacuation,festival_quiz,special_events_dungeon}.rs`, `tests/eve_quest/pure_planner_event_runner_and_preflight.rs`, `tests/compound_test.rs`, `tests/skill_job_system_test.rs`.

---

## 3. Trạng thái kiểm thử & Việc còn lại

- **Đã xác minh tự động**: `cargo test --all-targets --no-fail-fast` (chạy sau lần edit cuối cùng) — **687/687** test pass trên tất cả 11 binary test (`architecture_guardrails_test`: 8, `battle_test`: 186, `compound_test`: 21, `data_loaders_test`: 40, `db_test`: 21, `economy_inventory_test`: 59, `eve_quest_test`: 137, `events_arena_test`: 88, `server_network_test`: 107, `server_stop_test`: 1, `skill_job_system_test`: 19).
- **Chưa kiểm thử thủ công trên client thật `aLogin.exe`**:
  1. Chạy `cargo run`, xác nhận log startup hiện `eve_scenes` (> 3,800) và `eve_events=true`.
  2. Đăng nhập map `10817`: xác nhận `"Welcome to TS Dream"` hiển thị đúng kênh `(Công bố hệ thống)` không mất 3 ký tự đầu, nói chuyện Bạch Lão Nhân (`15009`) và chạm cổng `14 08 01 00` trên Live SSE Terminal phản hồi bình thường.
- **Nếu cần review diff trước khi user commit**: chạy `/code-review` trên working tree.

---

## 4. Suggested Skills

Agent ở phiên kế tiếp nên gọi `read_file` cho các skill sau tùy hướng tiếp tục:
- `code-review` (`C:\Users\tuan.vu\.gemini\config\skills\code-review\SKILL.md`): Nếu user yêu cầu rà soát diff theo `CODING_STANDARDS.md` và spec trước khi commit.
- `diagnosing-bugs` (`C:\Users\tuan.vu\.gemini\config\skills\diagnosing-bugs\SKILL.md`): Nếu khi test tay trên client `aLogin.exe` vẫn còn bất thường ở các NPC/kịch bản Eve khác trên map `10817`.
