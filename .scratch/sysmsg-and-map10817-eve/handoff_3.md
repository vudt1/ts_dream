# Walkthrough — Khắc Phục Lỗi Live-Test & Chuẩn Hóa Kiến Trúc Tương Tác

Toàn bộ các hạng mục trong [implementation_plan.md](file:///C:/Users/tuan.vu/.gemini/antigravity-cli/brain/9e93aebe-37ba-4a63-aac9-c6ad2b6d6e63/implementation_plan.md) cùng phần sửa bổ sung cho **Map `10817` Door #1 (`eve_no = 7`)** đã được triển khai và vượt qua toàn bộ **700 tests** trên cả 11 test suite chuẩn (`cargo test --all-targets --no-fail-fast`).

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

## 6. Sửa Bổ Sung Map `10817` Door #1 (`eve_no = 7`) & `Cargo.toml`
- **Nghiên cứu capture `packet_10817_new.txt` & `packet_10817_new_2.txt`**: chi tiết đối chiếu `eve_no = 4` (NPC #4) vs `eve_no = 7` (Door #1) xem tại [research_playmovie_10817.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/research_playmovie_10817.md).
  - `packet_10817_new_2.txt` cho thấy `06 01` tới `(1302, 695)` đã mở `eve_no = 7` (`MeetDoor`, phát `06 02`, `14 2C 01`, `14 01 mean=10447`), nhưng bị client tự động gửi `14 04 01 00` và `06 02` (stop-walk echo) làm server đóng hội thoại (`14 2C 02` / `14 08`).
- [talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs): thêm helper `clear_pending_if_active_event` (phát `14 09` qua `NpcTalkCodec::build_clear_pending_hex()` khi `session.current_event_session.is_some()`), áp dụng cho `handle_talk_start` (`14 01 <npc_id>`, giữ check `payload.len() < 2`), `handle_talk_click_door` (`14 04 <door_id>`, giữ bare `14 04` làm `end_talk`), và `handle_talk_warp` (`14 08`).
- [movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs): trong `handle_move_inner` (`sub == 2`), chỉ gửi `F44402001408` khi `conn.session.current_event_session.is_none() && !conn.session.talk_locked` để không đóng UI thoại/mở khóa đi bộ khi `06 02` echo về giữa lúc đang chạy event.
- [missing_eve_effect_executors.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/missing_eve_effect_executors.rs): thêm test `map_10817_door_1_survives_14_04_and_06_02_echo_then_plays_movies` kiểm chứng `MeetDoor(1)` → `14 04 01 00` (`14 09`) → `06 02` (không phát `14 08`) → `14 01` (`14 09`) → 5x `14 06` → `PlayMovie(17)` → `PlayMovie(18)` → `Warp(10867)` → idle `06 02` (`14 08`).
- [Cargo.toml](file:///D:/VUDT/GIT_PCC/ts_dream/Cargo.toml#L5-L6): thêm `default-run = "ts_dream"` trong `[package]` để có thể chạy thẳng `cargo run`.

---

## Kết Quả Kiểm Thử (`cargo test --all-targets --no-fail-fast`)

Tất cả 11 test suite chuẩn đều pass (**700 passed, 0 failed**):

| Suite | Kết quả |
| --- | --- |
| `architecture_guardrails_test` | 8 passed, 0 failed |
| `battle_test` | 186 passed, 0 failed |
| `compound_test` | 21 passed, 0 failed |
| `data_loaders_test` | 40 passed, 0 failed |
| `db_test` | 23 passed, 0 failed |
| `economy_inventory_test` | 62 passed, 0 failed |
| `eve_quest_test` | 144 passed, 0 failed |
| `events_arena_test` | 88 passed, 0 failed |
| `server_network_test` | 108 passed, 0 failed |
| `server_stop_test` | 1 passed, 0 failed |
| `skill_job_system_test` | 19 passed, 0 failed |

---

## Hướng Dẫn Live-Test Tiếp Theo

Trước khi bật server bằng `cargo run`, cần chạy migration SQLite thủ công cho DB live/dev của bạn (theo ADR 0004):

```powershell
sqlite3 DB/ts_dream_dev.db ".read migrations/0004_character_color.sql"
$env:TS_DATABASE_URL="sqlite://DB/ts_dream_dev.db"
$env:TS_PACKET_LOGGING="true"
cargo run
```

Trong lúc live-test với `aLogin.exe`, hãy kiểm tra lại các điểm sau:
1. Bấm nút **Sắp xếp** trên `homdo` để xác nhận client gửi `17 3C` hay chuỗi `17 0A` (cả hai đều đã phát `17 05` redraw).
2. Đi bộ vào cổng `10817 Door #1 (1300, 700)` (`eve_no = 7`):
   - Xác nhận hội thoại `10447..10452` không còn bị ngắt bởi `14 04 01 00` hay `06 02` echo.
   - Xác nhận sau `PlayMovie(17)` / `PlayMovie(18)` (`rt=5`), `aLogin.exe` có tự gửi `14 06` để sang `Warp(10867)` hay đứng chờ (nếu `a076 == 0` do `MeetDoor` mở event không qua `14 0C`, kiểm tra thêm nhánh `14 04` khi `pending_meet_door.is_some()` hoặc chuyển sang timer trễ).

## Suggested Skills (Cho Phiên Làm Việc Tiếp Theo)
- `diagnosing-bugs`: nếu live-test `10817 Door #1` phát hiện `PlayMovie` bị treo chờ `14 06` hoặc xuất hiện gói bất thường từ `aLogin.exe`.
- `tdd`: viết test tái hiện capture mới trong `tests/eve_quest/` trước khi sửa handler.

---

## 7. Cập Nhật Bổ Sung — Chuẩn Hóa `shoppoint` (`1A 04`, `23 04`, `0x42`)
- **Nghiên cứu & đối chiếu capture**: [research_shoppoint_and_talk_sync.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/research_shoppoint_and_talk_sync.md), [packet_shoppoint.txt](file:///D:/VUDT/GIT_PCC/ts_dream/packet_shoppoint.txt), [implementation_plan.md](file:///C:/Users/tuan.vu/.gemini/antigravity-cli/brain/89cabf7a-0b52-4721-98db-93d60a28683e/implementation_plan.md).
- **[shops.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs)**: thêm `currency_sync_frame(gold, shop_point)` (10 byte `1A 04`: `gold` + `shop_point`), chuyển `gold_frame` thành wrapper gọi `currency_sync_frame(gold, 0)`, đổi `point_frame` thành `money_capacity_error_frame() -> "F44402001A03"` (*"Dung lượng tiền bạc không đủ"*), và cập nhật các call-site ở `shops.rs`, [eve_runner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs), [event_npcs.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/event_npcs.rs), [trade_storage.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs).
- **[spawn.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs)** & **[gm.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/gm.rs)**:
  - Step 12 thay `gold_frame` + `1A 03` bằng `currency_sync_frame(s.gold, s.shop_point)`.
  - Step 21 bỏ 3 lệnh `store_frame` (`hp_store`/`sp_store = 10000` gây `"Điểm số còn dư 10000"`), thay bằng `shop_point_banner_frame(s.shop_point)` (`23 04` với `A=0, B=shop_point, D=0` để hiện đúng `"Điểm số còn dư <shop_point>"` và giữ `player+0x146C = shop_point`) + `shop_points_frame(s.shop_point)` (`42 02`).
  - `/addgold` và `/setgold` trong `gm.rs` dùng `currency_sync_frame`.
- **[system.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs)**:
  - `shop_points_frame(points)` phát `F44406004202` + `le32(points.min(POINT_CAP))`.
  - `handle_game_points` (`0x22`) phát `shop_points_frame` + `currency_sync_frame`.
  - `handle_gm_shop` (`0x42`): bỏ gate `gm_level` cho mọi người chơi đã `authed && logined`; `42 03` trả `42 02` + `1A 04`; `42 01` kiểm tra `price > 0`, thiếu điểm gửi `F4440300420E02`, kiểm tra `can_add_item`, commit DB trước qua `persist_shop_point_and_item` rồi mới cập nhật `conn.session`, `registry_update_if_online` và phát `17 06` + `42 02` + `1A 04`.
- **[docs/client-opcodes.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/client-opcodes.md)** & **[GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)**: cập nhật `1A 04`, `1A 03`, `42 02`, và quy cách banner `0x23 01–04`.

## 8. Cập Nhật Bổ Sung — Đồng Bộ `10817 Door #1`, `PlayMovie` & Non-Blocking Talk Pacing
- **Đối chiếu capture**: [packet_10817_Door#1_full.txt](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_Door%231_full.txt) & [packet_talks_lien_tiep.txt](file:///D:/VUDT/GIT_PCC/ts_dream/packet_talks_lien_tiep.txt).
- **[quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs)**: xóa `F44402000602` và frame lỗi 3 byte `F4440300142C01` trong `DoorActionResolution::Event` của `14 08`; xóa `pending_meet_door` và `pending_talk_advance` trước `activate_event_session`.
- **[auto_chain.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs)** & **[npc_event.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs)**: thêm `EventPhase::{AwaitingTalkAck, AwaitingMovieAck}` và runtime field `last_step_sent_ms: u64`.
- **[session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs)**: thêm `TalkAdvanceKind`, `PendingTalkAdvance`, và các field runtime (`door_trigger_cooldown`, `pending_talk_advance`, `pending_legacy_dialogs`, `last_talk_sent_ms`, `min_talk_dwell_ms` default `0` trong `Session::default()`) + đồng bộ đầy đủ trong `merge_post_dispatch`.
- **[movement.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs)** & **[talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs)**:
  - Bare `14 04` khi `AwaitingMovieAck` chỉ phát `14 09`, không gọi `end_talk`.
  - Set `door_trigger_cooldown` (`1500 ms`) khi kết thúc event gắn Door trên cùng map; bỏ qua `MeetDoor` trong thời gian cooldown.
  - `handle_talk_continue` (`14 06`): không drop im lặng (tránh kẹt `a094 = 1`); đặt `pending_talk_advance` đến đủ `min_talk_dwell_ms` (`2000 ms`) nếu client tự nhảy sớm, hoặc chạy ngay nếu đủ thời gian / vừa xong `AwaitingMovieAck`.
  - `talk_messages` (`data.talks`): nạp các mảnh sau vào `pending_legacy_dialogs` khi `min_talk_dwell_ms > 0` (giữ sync cho unit test khi `min_talk_dwell_ms == 0`).
- **[eve_runner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs)**: cập nhật `AwaitingTalkAck`/`AwaitingMovieAck` + `last_step_sent_ms`, và tách bước toast `rt=0` (`MutateItem` / `NpcTeam` / `RewardPet`) khỏi `TalkStep` kế tiếp khi `min_talk_dwell_ms > 0` (`1200 ms`).
- **[server_control.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs)**: đặt `min_talk_dwell_ms = 2000` (`DEFAULT_TALK_DWELL_MS`) cho live TCP session, thêm nhánh `tokio::select!` cho `pending_talk_advance` theo Two-Phase State Lock, và bỏ `tokio::time::sleep(frame.delay_ms)` trong vòng gửi `out.outgoing`.

## 9. Kết Quả Kiểm Thử Sau Cập Nhật (`cargo test --all-targets --no-fail-fast`)
- Chạy `cargo test --all-targets --no-fail-fast` sau lần chỉnh sửa cuối cùng: **thoát với mã `0`** trên toàn bộ 11 test suites (`architecture_guardrails_test`: 8 passed, `battle_test`: 186 passed, `server_network_test`: 109 passed, `server_stop_test`: 1 passed, `skill_job_system_test`: 19 passed, và tất cả các suite còn lại đều 0 failed).
- Các test mới được thêm vào [economy_wire.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/economy_wire.rs), [missing_eve_effect_executors.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/missing_eve_effect_executors.rs), và [eve_tcp_and_sysmsg.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/eve_tcp_and_sysmsg.rs).

### Checklist Live-Test Tiếp Theo (`aLogin.exe`, `TS_PACKET_LOGGING=true`)
1. Đăng nhập nhân vật mới (`shoppoint = 1000`): xác nhận hiện đúng `"Điểm số còn dư 1000"` (không còn `10000`), mở Mall (`42 03`) thấy `1000` và mua thử (`42 01`) trừ điểm thật.
2. Đi vào `10817 Door #1 (1300, 700)`: xác nhận không còn frame lỗi `14 2C 01` và kiểm chứng flow bước 6 → `PlayMovie(17)/(18)` → `Warp(10867)`.
3. Nói chuyện NPC #2 (`10917`) và NPC dùng `data.talks`: xác nhận mỗi đoạn thoại hiển thị đủ ~2 giây.
