# Kế hoạch Sửa đổi Chi tiết: Chuẩn hóa `shoppoint` (`1A 04` / `23 04` / `0x42`) & Đồng bộ Trạng thái Talk (`10817 Door #1`, `PlayMovie`, `Talks` Pacing)

## Tổng quan & Căn cứ Kiểm chứng
Kế hoạch này dựa trên kết quả đối chiếu trực tiếp 3 bản capture live ([`packet_shoppoint.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_shoppoint.txt), [`packet_10817_Door#1_full.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_Door%231_full.txt), [`packet_talks_lien_tiep.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_talks_lien_tiep.txt)), báo cáo nghiên cứu [`research_shoppoint_and_talk_sync.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/research_shoppoint_and_talk_sync.md), và các yêu cầu bạn đã xác nhận:
1. **Vấn đề 1 (`shoppoint`)**: Loại bỏ thông báo giả `"Điểm số còn dư 10000"` từ `23 04` khi đăng nhập, nạp đúng `shop_point = 1000` qua `1A 04` (10 byte) và `42 02` (`u32 LE`), mở `42 03` (truy vấn điểm) và `42 01` (mua đồ Mall) cho mọi người chơi đã đăng nhập (có chống gian lận giá `0` và rollback DB nguyên tử).
2. **Vấn đề 2a (`10817 Door #1` & `PlayMovie`)**: Xóa frame lỗi 3 byte `F4440300142C01` + `06 02` thừa ở [`quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1522-L1525), bổ sung `EventPhase::AwaitingTalkAck` / `AwaitingMovieAck`, xóa `pending_meet_door` khi mở event, chặn bare `14 04` đóng nhầm phiên đang `PlayMovie`, và thêm `door_trigger_cooldown` chống re-trigger vùng cửa `(1300, 700)`.
3. **Vấn đề 2b (`Talks` liên tiếp trôi nhanh)**:
   - Client `aLogin.exe` tự phát `C->S 14 06` ở NPC #2 mà không chờ click. Vì client đặt `PlayerState+0xA094 = 1` mỗi lần gửi `14 06`, server **không được drop im lặng** `14 06` (sẽ gây kẹt `a094 = 1`). Thay vào đó, server dùng bộ hẹn giờ **non-blocking `tokio::select!`** (`pending_talk_advance`) đảm bảo mỗi bước thoại hiển thị tối thiểu **`MIN_TALK_DWELL_MS = 2000` ms** nếu client tự nhảy quá nhanh, nhưng trả lời ngay (`0 ms` trễ thêm) nếu người chơi bấm chậm hơn 2 giây.
   - Tách riêng bước toast `rt=0` (`MutateItem`), `NpcTeam`, `RewardPet` khỏi `TalkStep` kế tiếp khi dùng pacing live (`min_talk_dwell_ms > 0`).
   - Chuyển `talk_messages` (`data.talks`) từ `send_delayed(500)` + `tokio::time::sleep(500)` sang hàng đợi `pending_legacy_dialogs` chạy bằng timer non-blocking trong `tokio::select!`, không bao giờ block `read_half.read`.

---

## Phần A: Vấn đề 1 — Chuẩn hóa `shoppoint`, Gỡ `23 04` & Mở `0x42`

### A.1. Chuẩn hóa `1A 04` (10 byte) & gỡ `1A 03` sai nghĩa
- **[MODIFY] [`src/server/handlers/shops.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L12-L26)**
  - Bổ sung `pub fn currency_sync_frame(gold: u32, shop_point: u32) -> String` phát khung 10 byte chuẩn `FUN_0079175f` `case 4`:
    `F4440A001A04` + `le32(gold.min(GOLD_CAP))` + `le32(shop_point.min(POINT_CAP))`.
  - Cập nhật `gold_frame(gold: u32)` (hoặc thay các call-site có `Session` bằng `currency_sync_frame(session.gold, session.shop_point)`) để không call-site nào phát `1A 04` thiếu 4 byte `P[6..9]` hay ghi đè `player+0x1300 = 0`:
    - Các file cần thay call-site truyền đủ `(gold, shop_point)`: [`src/server/handlers/shops.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L279), [`src/server/handlers/eve_runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L488-L494), [`src/server/handlers/event_npcs.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/event_npcs.rs#L51), [`src/server/handlers/trade_storage.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L491).
  - Bỏ dùng `point_frame` (`1A 03`) cho mục đích đồng bộ số dư; đổi/giữ helper `pub fn money_capacity_error_frame() -> &'static str { "F44402001A03" }` đúng nghĩa lỗi *"Dung lượng tiền bạc không đủ"* (`opcode_1a.md#L164-L170`).

### A.2. Sửa chuỗi Login & các chỗ gọi nhầm `23 04` (`store_frame`)
- **[MODIFY] [`src/server/spawn.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L625-L664)**
  - Step 12 (`L626-L633`): thay `gold_frame(s.gold)` + `point_frame(s.shop_point)` bằng một frame `currency_sync_frame(s.gold, s.shop_point)` (`1A 04` 10 byte).
  - Step 21 (`L660-L663`): **xóa** 3 lời gọi `store_frame(s.god)`, `store_frame(s.hp_store)`, `store_frame(s.sp_store)` (`23 04`) — triệt tiêu hoàn toàn spam `"Điểm số còn dư 10000"` và việc ghi đè `player+0x146C = 1_000_000`. Thay vào đó phát `crate::server::handlers::system::shop_points_frame(s.shop_point)` (`42 02 <le32(shop_point)>`) để khởi tạo `player+0x146C = 1000` ngay khi login.
- **[MODIFY] [`src/server/gm.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/gm.rs#L274-L276)**
  - Trong `/addgold` và `/setgold`, thay `store_frame(session.gold)` bằng `currency_sync_frame(session.gold, session.shop_point)`.
- **[MODIFY] [`src/server/handlers/system.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs#L64-L71)**
  - Trong `handle_game_points` (`0x22`), thay `store_frame(conn.session.gold)` bằng `shop_points_frame(conn.session.shop_point)` + `currency_sync_frame(conn.session.gold, conn.session.shop_point)`.

### A.3. Sửa `42 02` (`u32 LE`) và mở `0x42 01/03` cho mọi người chơi
- **[MODIFY] [`src/server/handlers/system.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs#L9-L175)**
  - Làm `pub fn shop_points_frame(points: u32) -> String`: phát `F44406004202` + `encoder::le32(points.min(POINT_CAP))` (6 byte payload `[42][02][points:4B LE]`, chuẩn `FUN_00522ab8` Mode 2 ghi `player+0x146C`).
  - Trong `handle_gm_shop`:
    - Chỉ kiểm tra `!conn.session.authed || !conn.session.logined` (bỏ gate `gm_level <= PLAYER_GM_LEVEL` cho cả `sub == 3` và `sub == 1`).
    - **`sub == 3` (`42 03` — truy vấn điểm)**: gửi `shop_points_frame(conn.session.shop_point)` và `currency_sync_frame(conn.session.gold, conn.session.shop_point)`.
    - **`sub == 1` (`42 01` — mua đồ Mall)**:
      - Kiểm tra `payload.len() >= 7`, `item_id > 0`, tồn tại trong `ctx.data.items`.
      - **Chống gian lận giá `0` / tràn điểm**: bắt buộc `price > 0` (nếu `price == 0`, từ chối giao dịch; lưu ý ghi rõ trong comment rằng `price` do client gửi nếu chưa có bảng giá Mall phía server). Nếu `conn.session.shop_point < price`, gửi `F4440300420E02` (`"Điểm số không đủ, mời bạn bổ sung điểm số"`, theo `opcode_42.md#L178`).
      - Kiểm tra `can_add_item`, chuẩn bị `homdo` mới + `new_points = shop_point - price`, **commit DB trước qua `persist_shop_point_and_item` (kiểm tra kết quả `bool`)**; chỉ khi DB thành công mới áp dụng vào `conn.session`, cập nhật `registry_update_if_online`, rồi phát `17 06` + `shop_points_frame(conn.session.shop_point)` + `currency_sync_frame(conn.session.gold, conn.session.shop_point)` (đảm bảo Two-Phase State Lock). Nếu DB thất bại, giữ nguyên RAM và không phát `17 06`.
- **[MODIFY] Tài liệu & Test liên quan**:
  - Cập nhật [`docs/client-opcodes.md`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/client-opcodes.md#L21) & [`GLOSSARY.md`](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md) phần `1A 04` / `1A 03` / `42 02`.
  - Cập nhật [`tests/economy_inventory/economy_wire.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/economy_wire.rs#L54-L125), [`tests/eve_quest/checkpoint6_parity.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/checkpoint6_parity.rs#L132-L148), [`tests/eve_quest/npc_eve_e2e.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/npc_eve_e2e.rs#L367-L379).

---

## Phần B: Vấn đề 2a — Đồng bộ Trạng thái Talk cho `10817 Door #1` & `PlayMovie`

### B.1. Dọn bug `14 2C 01` lỗi & xóa `pending_meet_door`
- **[MODIFY] [`src/server/handlers/quest.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1522-L1545)**
  - Trong nhánh `DoorActionResolution::Event(event_session)` của `handle_warp_confirm` (`14 08`):
    - **Xóa** 2 dòng `out.send("F44402000602")` và `out.send("F4440300142C01")` (dư thừa + thiếu `CharID: 4B LE`, gây out-of-bounds trong `FUN_0074641C`).
    - Xóa `conn.session.pending_meet_door = None;` và `conn.session.pending_talk_advance = None;` trước khi gọi `activate_event_session`.

### B.2. Mở rộng `EventPhase` & chống re-trigger / bare `14 04` trong `PlayMovie`
- **[MODIFY] [`src/eve/auto_chain.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L20-L120)**
  - Bổ sung vào `EventPhase`:
    - `AwaitingTalkAck` (đang hiển thị `TalkStep` chờ `14 06`)
    - `AwaitingMovieAck` (đang chạy `PlayMovie` / `MoveEntity` chờ `14 06`)
  - Thêm runtime field (default `0`, không lưu DB) vào `EventSession`: `pub last_step_sent_ms: u64`.
- **[MODIFY] [`src/server/session.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs)**
  - Thêm các field runtime vào `Session` (mặc định `None` / `0` / rỗng trong `Session::default()`):
    - `pub door_trigger_cooldown: Option<(u16, u16, u64)>` — `(map_id, door_id, until_ms)` đặt trong `1500 ms` khi một event gắn Door kết thúc/đóng mà không warp khỏi map.
    - `pub pending_talk_advance: Option<PendingTalkAdvance>` — `{ due_ms: u64, kind: TalkAdvanceKind }`.
    - `pub pending_legacy_dialogs: std::collections::VecDeque<String>` — hàng đợi các frame còn lại của `talk_messages`.
    - `pub last_talk_sent_ms: u64` và `pub min_talk_dwell_ms: u64` (mặc định `0` ở `Session::default()` để unit test đồng bộ hiện có không thay đổi timing; được set = `2000` khi kết nối live TCP khởi tạo trong `server_control.rs`).
  - Đăng ký đầy đủ các field mới vào `merge_post_dispatch` ([`src/server/session.rs#L924-L1037`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs#L924-L1037)) theo quy chuẩn Two-Phase State Lock.
- **[MODIFY] [`src/server/handlers/talk.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs)**
  - Trong `handle_talk_click_door` (`14 04`, [`L389-L395`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L389-L395)): nếu `payload.len() < 2` (bare `14 04`) nhưng `current_event_session.as_ref().is_some_and(|ev| ev.phase == EventPhase::AwaitingMovieAck)`, **không** gọi `end_talk`; chỉ phát `14 09` (`clear_pending_if_active_event`) để tránh đóng nhầm `EventSession` giữa lúc `PlayMovie` rồi kích hoạt lại bước 1.
  - Khi `end_talk_session` / `finish_event_session_iter` kết thúc event thuộc Door (`EventTriggerKind::MeetDoor | ClickDoor`) trên cùng `map_id` (không warp), set `session.door_trigger_cooldown = Some((session.map_id, door_id, now_ms + 1500))`.
- **[MODIFY] [`src/server/handlers/movement.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L45-L120)**
  - Bỏ qua `MeetDoor` nếu `conn.session.talk_locked` hoặc `door_trigger_cooldown` còn hiệu lực cho `(map_id, door_id)` tại `now_ms`.
  - Khi `sub == 2` (`L286-L294`): giữ guard `current_event_session.is_none() && !talk_locked` trước khi gửi `14 08`.

---

## Phần C: Vấn đề 2b — Non-Blocking Talk Pacing cho `eve_runner.rs` & Legacy `data.talks`

### C.1. Pacing cho nhánh Eve (`eve_runner.rs` & `talk.rs`)
- **[MODIFY] [`src/server/handlers/eve_runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs)**
  - Trong `activate_event_session`: xóa `session.pending_meet_door = None;`, `session.pending_talk_advance = None;`, `session.pending_legacy_dialogs.clear();`.
  - Trong `process_single_domain_effect`:
    - `DomainEffect::TalkStep`: set `ev.phase = EventPhase::AwaitingTalkAck`, `ev.last_step_sent_ms = now_ms()`.
    - `DomainEffect::PlayMovie` / `MoveEntity`: set `ev.phase = EventPhase::AwaitingMovieAck`, `ev.last_step_sent_ms = now_ms()`.
    - **Tách `rt=0` (`MutateItem` / `NpcTeam` / `RewardPet`) khỏi `TalkStep` kế tiếp**: trong `execute_event_step_core`, sau khi chạy một hiệu ứng non-blocking có phát `14 01` (như `MutateItem` ở `packet_talks_lien_tiep.txt` dòng 10), nếu `session.min_talk_dwell_ms > 0` và bước kế tiếp vẫn tồn tại (chưa hết chuỗi và không phải `Warp`/`StartBattle`), dừng batch hiện tại (`StepOutcome::Halt` / đặt `pending_talk_advance` sau `1200 ms`) để client kịp hiện toast nhận vật phẩm trước khi `TalkStep` kế tiếp ghi đè `gvar_007D9D6C`/`PlayerState`.
- **[MODIFY] [`src/server/handlers/talk.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L264-L285)**
  - Trong `handle_talk_continue` (`14 06`):
    - Giữ nguyên guard `AwaitingChoice` & `AwaitingBattle`.
    - Nếu `conn.session.pending_talk_advance.is_some()` → trả về an toàn (chống double `14 06` tăng `current_index` 2 lần).
    - Nếu `conn.session.min_talk_dwell_ms > 0 && ev.phase == EventPhase::AwaitingTalkAck`:
      - Tính `elapsed = now_ms().saturating_sub(ev.last_step_sent_ms)`.
      - Nếu `elapsed < conn.session.min_talk_dwell_ms`: đặt `conn.session.pending_talk_advance = Some(PendingTalkAdvance { due_ms: ev.last_step_sent_ms + conn.session.min_talk_dwell_ms, kind: TalkAdvanceKind::EveStep })` rồi return (giữ `a094 = 1` tạm thời, **không drop** và **không tăng `current_index` hai lần**).
      - Nếu `elapsed >= conn.session.min_talk_dwell_ms` (hoặc `ev.phase == EventPhase::AwaitingMovieAck` vừa xong `PlayMovie`): tăng `ev.current_index += 1` và gọi `execute_event_step_with_pool` **ngay lập tức**.

### C.2. Chuyển `talk_messages` (`data.talks`) & `server_control.rs` sang Non-Blocking `tokio::select!`
- **[MODIFY] [`src/server/handlers/talk.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L71-L83)**
  - Trong `talk_messages(conn, talk_string, out)`:
    - Tách các mảnh `F444...`. Mảnh đầu tiên gửi ngay (`out.send(first)`), cập nhật `conn.session.last_talk_sent_ms = now_ms()`.
    - Nếu còn các mảnh sau:
      - Nếu `conn.session.min_talk_dwell_ms == 0` (unit test đồng bộ): gửi ngay theo thứ tự cũ để bảo toàn tương thích test đồng bộ.
      - Nếu `conn.session.min_talk_dwell_ms > 0` (live TCP): nạp các mảnh còn lại vào `conn.session.pending_legacy_dialogs`, và đặt `conn.session.pending_talk_advance = Some(PendingTalkAdvance { due_ms: now_ms + conn.session.min_talk_dwell_ms, kind: TalkAdvanceKind::LegacyDialog })`. (Khi gửi đến `F44402001408` cuối hàng đợi, cập nhật `select_menu = 40` tương ứng).
- **[MODIFY] [`src/web/server_control.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L830-L1100)**
  - Khởi tạo `conn.session.min_talk_dwell_ms = 2000` (hoặc hằng số `DEFAULT_TALK_DWELL_MS = 2000`) cho phiên TCP live (lưu ý set lại sau `login` snapshot load).
  - Thêm nhánh `tokio::select!` mới cho `conn.session.pending_talk_advance` (cùng mẫu `Two-Phase State Lock` như `pending_meet_door` ở `L831-L945`):
    - Khi `due_ms` tới:
      - Nếu `TalkAdvanceKind::EveStep`: tăng `ev.current_index += 1`, gọi `execute_event_step_with_pool`, đồng bộ `merge_post_dispatch`, đẩy `out.outgoing` (và kích hoạt `eve_battle`/`battle_trigger` nếu bước đó mở trận).
      - Nếu `TalkAdvanceKind::LegacyDialog`: pop mảnh tiếp theo từ `pending_legacy_dialogs`, gửi qua `tx`; nếu vẫn còn mảnh phía sau thì hẹn tiếp `due_ms = now_ms + min_talk_dwell_ms`.
  - Bỏ hoàn toàn `tokio::time::sleep(frame.delay_ms)` cho thoại trong vòng `for frame in &out.outgoing` (`L920-L923`, `L1094-L1097`) hoặc chỉ giữ cho trường hợp không phải thoại nếu còn dùng, giúp socket reader không bao giờ bị treo giữa chừng.

---

## Kế hoạch Kiểm thử (`cargo test`) & Xác minh Live-Test

1. **Test đơn vị & tích hợp mới (thêm vào đúng submodule theo `AGENTS.md`, không tạo `*_test.rs` lẻ)**:
   - [`tests/economy_inventory/economy_wire.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/economy_wire.rs):
     - Kiểm tra `build_logined_sequence_session` phát `1A 04` đúng 10 byte (`gold` + `shop_point = 1000`), không còn `1A 03` và không còn `23 04` (`store_frame`).
     - Kiểm tra `OP_ITEM_MALL (0x42)`: người chơi thường (`gm_level == 0`) gọi `42 03` nhận `42 02 <le32(shop_point)>`; gọi `42 01` mua thành công trừ DB + RAM và phát `17 06` + `42 02` + `1A 04`; từ chối `price == 0` và thiếu điểm (`42 0E 02`); rollback RAM nếu DB lỗi.
   - [`tests/eve_quest/missing_eve_effect_executors.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/missing_eve_effect_executors.rs) & [`tests/server_network/eve_tcp_and_sysmsg.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/eve_tcp_and_sysmsg.rs):
     - Kiểm tra `10817 Door #1` qua `14 08 01 00`: **không** phát `06 02` thừa và **không** phát frame lỗi `F4440300142C01`; bare `14 04` trong `AwaitingMovieAck` không đóng `EventSession`; kết thúc event có `door_trigger_cooldown` chặn re-trigger tức thời.
     - Kiểm tra pacing khi `min_talk_dwell_ms = 2000`: `14 06` gửi sau `50 ms` đặt `pending_talk_advance` (chưa nhảy bước ngay, không mất lượt); `14 06` gửi sau `2100 ms` trả `14 01` ngay lập tức; `MutateItem (rt=0)` tách nhịp khỏi `TalkStep` kế tiếp; `talk_messages` với `min_talk_dwell_ms = 2000` nạp hàng đợi `pending_legacy_dialogs` thay vì `delay_ms = 500`.
     - Kiểm tra TCP thật qua `ServerControl` xác nhận `pending_talk_advance` nhả bước kế sau dwell time mà vẫn đọc socket bình thường.
2. **Chạy toàn bộ test suite trước handoff**:
   - `cargo test --all-targets --no-fail-fast` (xác nhận toàn bộ 11 suite chuẩn pass).
3. **Live-test lại với `aLogin.exe` (`TS_PACKET_LOGGING=true`)**:
   - Xác nhận lúc login không còn `"Điểm số còn dư 10000"`, mở `0x42` hiển thị đúng `1000` (và mua đồ trừ điểm thật).
   - Đi vào `10817 Door #1 (1300, 700)` xác nhận hết frame lỗi `14 2C 01` và kiểm chứng hành vi bước 6 → `PlayMovie(17)/(18)` → `Warp(10867)`.
   - Nói chuyện NPC #2 (`10917`) và NPC dùng `data.talks` xác nhận từng câu thoại dừng đủ ~2 giây để đọc.
