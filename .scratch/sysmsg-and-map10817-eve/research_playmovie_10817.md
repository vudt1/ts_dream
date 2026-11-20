# Báo cáo Nghiên cứu: `PlayMovie` (`rt=5`), `MoveEntity` (`rt=4`) & Luồng Warp Map `10817` → `10867` trong Live Test

- **Ngày**: 2026-11-20
- **Phạm vi**: Đối chiếu trực tiếp capture thực tế [`packet_10817_new.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_new.txt) với mã nguồn server (`src/`), test suite (`tests/`), đặc tả opcode (`spec/`, `.scratch/client-pseudo-op-code/`), và dữ liệu kịch bản đã trích xuất ([`eve_full.jsonl`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_full.jsonl), [`eve_doors.tsv`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_doors.tsv)). Không chỉnh sửa mã nguồn hay test.

---

## Tóm tắt Kết luận (Executive Summary)

1. **Nguyên nhân chính khiến `PlayMovie` (`rt=5`) không xuất hiện trong [`packet_10817_new.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_new.txt) và client warp thẳng sang `10867`**:
   - Gói mở đầu ở dòng `1` là **`c2s F444040014010400`** (`0x14 0x01`, `map_object_id = 4`), tức là người chơi đã **Click NPC #4** (Bạch Lão Nhân, `npc_id = 15009`, tọa độ `(1430, 660)`) chứ **không** kích hoạt **Door #1** (`MeetDoor` hoặc `14 08 01 00` tại `(1300, 700)`).
   - Theo dữ liệu `10817.eve` ([`eve_full.jsonl:18`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_full.jsonl#L18), kiểm chứng tại [`tests/eve_quest/npc_eve_resolve.rs:60-72`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/npc_eve_resolve.rs#L60-L72)):
     - **`eve_no = 4`** (`ClickNpc(4)`) chỉ có đúng **7 kết quả**: **6 câu thoại `Talk` (`rt = 1`, `mean_no = 10447..10452`)** nối thẳng bởi **1 `Warp` (`rt = 2, rc = 7, param = 1`)** sang map `10867`. **`eve_no = 4` hoàn toàn KHÔNG có `PlayMovie` (`rt = 5`) hay `MoveEntity` (`rt = 4`).**
     - **`eve_no = 7`** (`OnStrokeObject` gắn vào **Door #1**) mới là kịch bản có **2 bước `PlayMovie` (`rt = 5`, `val = 17` và `val = 18`)** nằm giữa 6 câu thoại `10447..10452` và `Warp` (`rt = 2, param = 1`).
   - Do đó, sau 6 lần `c2s 14 06` (dòng `5, 7, 9, 11, 13, 15`), bước kế tiếp (`idx = 6`) trong `eve_no = 4` là `DomainEffect::Warp { warp_id: 1 }` nên server thực hiện mở khóa (`14 2C .. 02`), `14 07` và `0x0C` chuyển sang map `10867` là **đúng theo dữ liệu `eve_no = 4`**.

2. **Tại sao có gói `c2s F44402001406` ở dòng `22` (sau `05 04` + `14 08`)**:
   - Ở dòng `17`, server gửi `s2c F44402001407` (`SUB_WARP_ACK`). Theo đặc tả `FUN_0078EC3F` (`case 0x07`) trong [`opcode_14.md:75,158-162,243`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L75), gói `14 07` đặt `a094 = 0`, `a096 = 1`, `a097 = 1` và cấp cờ **"vé tự quay về cổng" `TFConnect+0x44F = 1`**.
   - Game-tick pump `FUN_005EE30C` của `aLogin.exe` tự động phát **`C→S 0x14 0x06`** khi `TFConnect+0x44F != 0` và `a076 != 0, a094 == 0, a095 == 0` ([`opcode_14.md:243`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L243)). Gói `14 06` ở dòng `22` được client phát ra trong nhịp chuyển cảnh trước khi `14 08` (dòng `21`) dọn cờ `0x44F` / `a076`; phía server [`handle_talk_continue`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L288) thấy `current_event_session == None`, `warp_finish == false`, `idtalking <= 0` nên bỏ qua an toàn ở dòng `286-288`.

3. **Trạng thái hiện tại của `PlayMovie` (`rt=5`) & `MoveEntity` (`rt=4`) trên Server (`src/eve/planner.rs` & `src/server/handlers/eve_runner.rs`)**:
   - Đã được sửa thành **blocking** (`DomainEffect::is_blocking_or_terminal()` tại [`src/eve/planner.rs:170-180`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L170-L180) và `StepOutcome::Halt` tại [`src/server/handlers/eve_runner.rs:830-847`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L830-L847)), đồng thời được kiểm chứng bằng unit/integration test tại [`tests/eve_quest/missing_eve_effect_executors.rs:835-854`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/missing_eve_effect_executors.rs#L835-L854) cho kịch bản `10817 Door 1 (eve_no = 7)`.
   - Tuy nhiên, nếu người chơi thực sự đi vào **Door #1** (`eve_no = 7`) trong live test, cần xác nhận thêm hành vi của `aLogin.exe` khi nhận `14 01 (rt=5)` đơn lẻ (không kèm `14 07` ở cuối): theo [`opcode_14.md:151`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L151), `FUN_005EB530` `case 5` bật `gvar_007DA37C+0x1D = 1` (movie mode) và chạy `TMovie`; nếu client không tự gửi `14 06` khi kết thúc `TMovie` (vì `0x44F` chưa bật hoặc `a076 == 0`), server sẽ chờ `14 06` vô thời hạn trừ khi chuyển sang cơ chế timer trễ (như đã lưu ý trong [`handoff_3.md:72`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/handoff_3.md#L72)).

---

## Phần 1: Giải mã Toàn bộ Capture [`packet_10817_new.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_new.txt)

Tất cả 22 dòng trong [`packet_10817_new.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_new.txt) là frame đã giải/chưa XOR `0xAD` (cùng định dạng log SSE của [`server_control.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs)). Cấu trúc `S→C 14 01` tuân theo [`NpcTalkCodec::build_talk_step_body`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs#L56-L119): `F4 44 11 00 14 01 00 [group:2B LE] [no:1B] [rt:1B] [rc:1B] [param:2B LE] [ps:1B] [val:4B LE] [mean_no:2B LE]`.

| Dòng | Chiều | Raw Hex | Giải mã & Ý nghĩa | Nguồn xử lý / đối chiếu |
|---:|:---:|---|---|---|
| `1` | `c2s` | `F444040014010400` | `OP_NPC_EVENT (0x14)` sub `0x01`, `map_object_id = 04 00` (**4**) — **Click NPC #4** (`npc_id = 15009`, Bạch Lão Nhân @ `(1430, 660)`). | [`talk.rs:94,158-226`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L158-L226), [`eve_full.jsonl:18`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_full.jsonl#L18) |
| `2` | `s2c` | `F44402000602` | `0x06 0x02` (`build_move_stop_hex`) — dừng di chuyển nhân vật trước hội thoại. | [`eve_runner.rs:1193`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1193), [`npc_talk.rs:144-146`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs#L144-L146) |
| `3` | `s2c` | `F4440700142CE093040001` | `0x14 0x2C`, `char_id = E0 93 04 00` (**300000**), `mode = 01` (`TalkLockMode::Lock`) — khóa thao tác actor. | [`eve_runner.rs:1194-1197`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1194-L1197), [`npc_talk.rs:186-191`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs#L186-L191) |
| `4` | `s2c` | `F4441100140100000001010700000000000000CF28` | `0x14 0x01`: `grp=0, no=1, rt=1 (Talk), rc=7, param=0, ps=0, val=0, mean=CF 28` (**10447**) — câu thoại #1 của `eve_no = 4`. | [`eve_runner.rs:730-733`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L730-L733) |
| `5` | `c2s` | `F44402001406` | `0x14 0x06` (`SUB_TALK_CONTINUE`) — người chơi bấm tiếp tục từ bước `idx=0` → `idx=1`. | [`talk.rs:251-271`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L271) |
| `6` | `s2c` | `F4441100140100000002010304000000000000D028` | `0x14 0x01`: `grp=0, no=2, rt=1 (Talk), rc=3, param=4, mean=D0 28` (**10448**) — câu thoại #2 của `eve_no = 4`. | [`eve_runner.rs:730-733`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L730-L733) |
| `7` | `c2s` | `F44402001406` | `0x14 0x06` — tiếp tục `idx=1` → `idx=2`. | [`talk.rs:251-271`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L271) |
| `8` | `s2c` | `F4441100140100000003010700000000000000D128` | `0x14 0x01`: `grp=0, no=3, rt=1 (Talk), rc=7, param=0, mean=D1 28` (**10449**) — câu thoại #3 của `eve_no = 4`. | [`eve_runner.rs:730-733`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L730-L733) |
| `9` | `c2s` | `F44402001406` | `0x14 0x06` — tiếp tục `idx=2` → `idx=3`. | [`talk.rs:251-271`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L271) |
| `10` | `s2c` | `F4441100140100000004010304000000000000D228` | `0x14 0x01`: `grp=0, no=4, rt=1 (Talk), rc=3, param=4, mean=D2 28` (**10450**) — câu thoại #4 của `eve_no = 4`. | [`eve_runner.rs:730-733`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L730-L733) |
| `11` | `c2s` | `F44402001406` | `0x14 0x06` — tiếp tục `idx=3` → `idx=4`. | [`talk.rs:251-271`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L271) |
| `12` | `s2c` | `F4441100140100000005010700000000000000D328` | `0x14 0x01`: `grp=0, no=5, rt=1 (Talk), rc=7, param=0, mean=D3 28` (**10451**) — câu thoại #5 của `eve_no = 4`. | [`eve_runner.rs:730-733`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L730-L733) |
| `13` | `c2s` | `F44402001406` | `0x14 0x06` — tiếp tục `idx=4` → `idx=5`. | [`talk.rs:251-271`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L251-L271) |
| `14` | `s2c` | `F4441100140100000006010304000000000000D428` | `0x14 0x01`: `grp=0, no=6, rt=1 (Talk), rc=3, param=4, mean=D4 28` (**10452**) — câu thoại #6 (cuối) của `eve_no = 4`. | [`eve_runner.rs:730-733`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L730-L733) |
| `15` | `c2s` | `F44402001406` | `0x14 0x06` — tiếp tục `idx=5` → `idx=6` (`DomainEffect::Warp { warp_id: 1 }`). | [`eve_runner.rs:1031-1079`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1031-L1079) |
| `16` | `s2c` | `F4440700142CE093040002` | `0x14 0x2C`, `char_id = 300000`, `mode = 02` (`TalkLockMode::Unlock`) — mở khóa trước khi warp. | [`eve_runner.rs:1055-1061`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1055-L1061) |
| `17` | `s2c` | `F44402001407` | `0x14 0x07` (`build_warp_ack_hex`) — xác nhận warp (client đặt `a097=1, a094=0, a096=1, 0x44F=1`). | [`quest.rs:873`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L873), [`opcode_14.md:75`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L75) |
| `18` | `s2c` | `F4440D000CE0930400732A7A03FE010100` | `0x0C` (`build_relocate_packet`): `id = 300000`, `map = 73 2A` (**10867**), `x = 7A 03` (**890**), `y = FE 01` (**510**), `warp_id = 01`, `00`. | [`spawn.rs:274-283`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L274-L283), [`quest.rs:876-878`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L876-L878) |
| `19` | `c2s` | `F44402001730` | `0x17 0x30` (`48`) — client gửi khi chuyển cảnh; server hiện không map sub `48` trong `handle_inventory` (vào nhánh `_ => {}`). | [`inventory.rs:53-87`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs#L53-L87), [`opcode_17_full_payloads.md:55`](file:///D:/VUDT/GIT_PCC/ts_dream/spec/exhaustive_by_opcode/opcode_17_full_payloads.md#L55) |
| `20` | `c2s` | `F44402000C01` | `0x0C 0x01` — client xác nhận đã nạp xong map `10867`. | [`system.rs:181-188`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs#L181-L188) |
| `21` | `s2c` | `F44402000504F44402001408` | `0x05 0x04` (World-Ready) + `0x14 0x08` (EndTalk: dọn cờ `a095/076/097/096/094`, xóa `0x44F`). | [`system.rs:188`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs#L188), [`opcode_14.md:76`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L76) |
| `22` | `c2s` | `F44402001406` | `0x14 0x06` (`SUB_TALK_CONTINUE`) phát muộn từ tick pump `FUN_005EE30C` sau `14 07`; server nhận khi `idtalking == 0` nên bỏ qua tại `talk.rs:286-288`. | [`opcode_14.md:243`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L243), [`talk.rs:286-288`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L286-L288) |

---

## Phần 2: Đối chiếu `eve_no = 4` (NPC #4) vs `eve_no = 7` (Door #1) tại Map `10817`

Trích xuất từ [`eve_full.jsonl:18`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_full.jsonl#L18) và [`eve_doors.tsv:18`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_doors.tsv#L18):

| Thuộc tính | `eve_no = 4` (Bạch Lão Nhân — `NPC id=4`, `npc_id=15009`) | `eve_no = 7` (Cổng ra — `Door id=1`) |
|---|---|---|
| Tọa độ | `(1430, 660)` | `(1300, 700)` |
| `when_happen` | `[0]` (`OnClickObject`) | `[1]` (`OnStrokeObject`) |
| Trigger từ client | `C→S 14 01 04 00` (`ClickNpc(4)`) | `MeetDoor` (`06 01` vào vùng cổng) hoặc `C→S 14 08 01 00` |
| Bước `1..6` | `rt=1` (`mean = 10447..10452`) | `rt=1` (`mean = 10447..10452`) |
| Bước `7` | **`rt=2, rc=7, param=1`** (`Warp` → `10867`) | **`rt=5, rc=0, param=0, ps=2, val=17`** (`PlayMovie`) |
| Bước `8` | *(không có)* | **`rt=5, rc=0, param=0, ps=1, val=18`** (`PlayMovie`) |
| Bước `9` | *(không có)* | **`rt=2, rc=7, param=1`** (`Warp` → `10867`) |

**Kết luận trực tiếp**:
- Trong [`packet_10817_new.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_new.txt), client gửi `14 01 04 00` (`ClickNpc(4)`) nên server chạy **`eve_no = 4`**, dẫn tới không hề có bước `PlayMovie` (`rt=5`) nào được lập kế hoạch hay phát ra, và ngay sau câu thoại thứ 6 (`D4 28 = 10452`), server warp thẳng sang `10867`.
- Kiểm tra ngược trên `eve_runner.rs`: hàm [`execute_event_step_core`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L993-L1137) chỉ trả về `DomainEffect::Warp` ở bước thứ 7 của `eve_no = 4`; không có bất kỳ lỗi skip `rt=5` nào xảy ra trong phiên capture này.

---

## Phần 3: Kiểm tra Cơ chế `PlayMovie` (`rt=5`) & `MoveEntity` (`rt=4`) và Giao thức `14 06` trên `aLogin.exe`

Mặc dù capture [`packet_10817_new.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_new.txt) không đi qua `eve_no = 7`, cần ghi nhận rõ trạng thái hiện tại của `PlayMovie` (`rt=5`) và `MoveEntity` (`rt=4`) trong code:

1. **Phía Server (`src/eve/planner.rs` & `src/server/handlers/eve_runner.rs`)**:
   - [`DomainEffect::is_blocking_or_terminal`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L170-L180) đã bao gồm `Self::MoveEntity(_)` và `Self::PlayMovie(_)`.
   - [`process_single_domain_effect`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L830-L847) gửi `NpcTalkCodec::build_talk_step_hex(res)` (`F4 44 11 00 14 01 00 [14B EveResult]`) và trả về `StepOutcome::Halt` cho cả `MoveEntity` (`rc == 3 && result_value >= 10_000`) và `PlayMovie`.
   - [`tests/eve_quest/missing_eve_effect_executors.rs:835-854`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/missing_eve_effect_executors.rs#L835-L854) đã xác nhận `10817 Door 1 (eve_no = 7)` dừng ở `PlayMovie (rt=5)` chờ `14 06` trước khi phát `14 07` (`Warp`).

2. **Phía Client `aLogin.exe` theo [`opcode_14.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md)**:
   - Khi nhận `14 01` (`rt = 5`), `FUN_0078EC3F` xóa `a094 = 0` và gọi `FUN_005EB530` `case 5` ([`opcode_14.md:140,151`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L140)): mở khóa `a096` theo `B4` (`parameter_style`), đặt `gvar_007DA37C+0x1D = 1` và phát `TMovie` với `D1 = result_value`.
   - Trong khi đó, tick pump `FUN_005EE30C` chỉ tự động gửi **`C→S 0x14 0x06`** khi `TFConnect+0x44F != 0` (chỉ được cấp ở `S→C 14 07`) và `a076 != 0, a094 == 0, a095 == 0` ([`opcode_14.md:243`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L243)).
   - **Hệ quả cần lưu ý khi test thật `10817 Door 1 (eve_no = 7)`**: nếu client không tự gửi `14 06` sau khi phát xong `TMovie` (hoặc không có thao tác click tiếp từ người chơi), `EventSession` sẽ dừng vĩnh viễn ở bước `PlayMovie` (`rt=5`) vì `StepOutcome::Halt` đang chờ `14 06`. Đây chính là điểm cần kiểm chứng bằng live test đi qua **Door #1 (`1300, 700`)** thay vì click NPC #4 (`1430, 660`).
