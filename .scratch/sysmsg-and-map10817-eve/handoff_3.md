# Walkthrough — Khắc Phục Lỗi Live-Test & Chuẩn Hóa Kiến Trúc Tương Tác

Toàn bộ các hạng mục trong [implementation_plan.md](file:///C:/Users/tuan.vu/.gemini/antigravity-cli/brain/9e93aebe-37ba-4a63-aac9-c6ad2b6d6e63/implementation_plan.md) đã được triển khai qua 3 subagent song song và vượt qua toàn bộ **699 tests** trên cả 11 test suite chuẩn (`cargo test --all-targets --no-fail-fast`).

---

## 1. Domain Model & Kiến Trúc (`GLOSSARY.md`, ADR `0012`)
- Cập nhật [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md): gỡ `color` khỏi danh sách `Deferred Drift`, bổ sung định nghĩa **Appearance Color (Màu ngoại hình)** và **Proximity Gate & Walk Interpolation**.
- Tạo [0012-walk-interpolation-proximity-and-color-persistence.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0012-walk-interpolation-proximity-and-color-persistence.md) ghi nhận đánh đổi kiến trúc cho nội suy bước đi `0x06 0x01`, `PendingMeetDoor`, bán kính `130 px` Euclidean, và cột `characters.color`.

## 2. Bền Vững Hóa Màu Ngoại Hình (`characters.color`, Migration `0004`)
- Tạo [0004_character_color.sql](file:///D:/VUDT/GIT_PCC/ts_dream/migrations/0004_character_color.sql) (`ALTER TABLE characters ADD COLUMN color TEXT NOT NULL DEFAULT '0000000000000000';`).
- [pool.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/pool.rs): bổ sung kiểm tra cột `"color"` trong `verify_schema()`, báo lỗi `TsError::Migrate` trỏ tới `migrations/0004_character_color.sql` nếu DB chưa migrate.
- [session.rs (sqlite)](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs): thêm `normalize_color_hex()`, đồng bộ `color` trong `load`, `save_in_tx`, và `create_and_seed`.
- [persist.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/persist.rs), [inventory.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs) (`handle_reborn`), và [auto_save.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/auto_save.rs): thêm `update_player_text` cho `handle_reborn` và trộn `s.color.as_bytes()` vào `fingerprint()`.
- [test_db.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/common/test_db.rs), [sqlite_migration_0004_color.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/db/sqlite_migration_0004_color.rs), [startup_schema_check.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/db/startup_schema_check.rs), và [create_char_atomic.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/create_char_atomic.rs): bổ sung test roundtrip & kiểm tra frame `0x03`.

## 3. Hòm Đồ `homdo` (`0x17 0x0A` Split & `0x17 0x3C` Sort)
- [mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs): thêm `ITEM_SUB_HOMDO_DUMP` (`0x05`), `ITEM_SUB_MOVE_STACK` (`0x0A`), `ITEM_SUB_SORT` (`0x3C`).
- [inventory.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs):
  - `handle_move_stack`: hỗ trợ tách một phần (`count < src_count`) sang ô trống (`dst == None`), từ chối sạch nếu `count < src_count` không thể gộp, và luôn phát `conn.session.dump_homdo()` (`17 05`) sau echo `17 0A` ở mọi nhánh thành công để client vẽ lại ngay.
  - `handle_sort_homdo`: xử lý `0x17 0x3C`, giữ riêng trang bị (`1..=6`), gộp các vật phẩm xếp chồng trùng `id` và thuộc tính động tối đa `STACK_CAP` (`50`), sắp xếp `(id, Reverse(count), slot)`, lưu nguyên tử qua `persist_sessions_transaction` (kèm rollback nếu lỗi DB), đồng bộ registry và phát `17 05`.
- [inventory_roundtrip.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/inventory_roundtrip.rs): thêm test `test_move_stack_partial_split_to_empty_slot`, `test_move_stack_partial_split_negative_rejects`, và `test_sort_homdo_merges_and_sorts`.

## 4. Proximity Gate (`130 px`), Walk Interpolation & Delayed `MeetDoor`
- [state.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/state.rs): đặt `INTERACT_RADIUS_PX = 130` (`MAX_INTERACT_DISTANCE_SQ = 130 * 130`).
- [session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs): thêm `PendingMeetDoor`, `now_ms()`, `walk_origin`, `walk_dest`, `walk_start_ms`, `walk_speed_px_per_sec()` (`128.0` khi `map_id % 1000 == 0`, ngược lại `160.0`), `estimated_position_at()`, và `clear_walk_tracking()`.
- [movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs) & [server_control.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs): ghi nhận hành trình ở `06 01`/`06 02`, hoãn `MeetDoor` qua `pending_meet_door` khi `travel_ms > 250` ms, thêm nhánh `tokio::select!` gọi `trigger_meet_door_for_door()` dưới `lock_player_operations`, và hủy khi có `06 01`/`06 02`/battle mới.
- [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) & [quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs): áp dụng `estimated_position_at()` + `130 px` cho `resolve_npc` (`14 01`), `handle_talk_click_door` (`14 04`), và `handle_warp_confirm` (`14 08`) (im lặng gửi `14 08` EndTalk khi ngoài tầm); gọi `clear_walk_tracking()` khi warp party.

## 5. Map `10917 #16015`, Quán Trọ / Tiền Trang (`rt=7, rc=7`) & `PlayMovie`/`MoveEntity` Blocking
- [planner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs) & [eve_runner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs):
  - Thêm `NpcServiceOp` & `DomainEffect::NpcService` cho `result_type == 7 && result_class == 7` (`ps = 1, 2, 4, 5, 15`), không bao giờ gửi thô `rt >= 7` qua `0x14 01`.
  - Đánh dấu `PlayMovie` (`rt=5`) và `MoveEntity` (`rt=4`) là blocking (`StepOutcome::Halt` chờ `14 06`).
- [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs): ưu tiên `npc_event::resolve_npc_event` trước các nhánh Banker/Inn hardcoded fallback, và thay `"0100"` hardcoded trong `handle_inn_menu` bằng `encoder::le16(conn.session.idtalking as u16)`.

---

## Kết Quả Kiểm Thử (`cargo test --all-targets --no-fail-fast`)

Tất cả 11 test suite chuẩn đều pass (**699 passed, 0 failed**):

| Suite | Kết quả |
| --- | --- |
| `architecture_guardrails_test` | 8 passed, 0 failed |
| `battle_test` | 186 passed, 0 failed |
| `compound_test` | 21 passed, 0 failed |
| `data_loaders_test` | 40 passed, 0 failed |
| `db_test` | 23 passed, 0 failed |
| `economy_inventory_test` | 62 passed, 0 failed |
| `eve_quest_test` | 143 passed, 0 failed |
| `events_arena_test` | 88 passed, 0 failed |
| `server_network_test` | 108 passed, 0 failed |
| `server_stop_test` | 1 passed, 0 failed |
| `skill_job_system_test` | 19 passed, 0 failed |

---

## Hướng Dẫn Live-Test Tiếp Theo

Trước khi bật server bằng `cargo run --bin ts_dream`, cần chạy migration SQLite thủ công cho DB live/dev của bạn (theo ADR 0004):

```powershell
sqlite3 DB/ts_dream_dev.db ".read migrations/0004_character_color.sql"
$env:TS_DATABASE_URL="sqlite://DB/ts_dream_dev.db"
$env:TS_PACKET_LOGGING="true"
cargo run --bin ts_dream
```

Trong lúc live-test với `aLogin.exe`, hãy kiểm tra lại 2 điểm đã lưu ý trong kế hoạch:
1. Bấm nút **Sắp xếp** trên `homdo` để xác nhận client gửi `17 3C` hay chuỗi `17 0A` (cả hai đều đã phát `17 05` redraw).
2. Đi qua cổng `10817 Door 1` (`eve_no=7`) để xác nhận sau `PlayMovie` (`rt=5`), client có gửi `14 06` hay đứng chờ (nếu client không gửi `14 06`, sẽ chuyển bước tiếp theo sang timer trễ).
