# Handoff: NPC Talk & Eve Script System (Opcodes 0x14 & 0x18)

## 1. Summary of Work Completed

Tài liệu thiết kế chi tiết và lộ trình tổng thể:
- Kế hoạch: [`.scratch/op-working/plan-npc-talk-eve-opcode-14-18.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/plan-npc-talk-eve-opcode-14-18.md)
- Research Checkpoint 1: [`.scratch/op-working/research-checkpoint-1-wire-spec-14-18.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-1-wire-spec-14-18.md)
- Research Checkpoint 2: [`.scratch/op-working/research-checkpoint-2-click-npc-bridge.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-2-click-npc-bridge.md)
- Research Checkpoint 3: [`.scratch/op-working/research-checkpoint-3-talk-packets.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-3-talk-packets.md)
- Research Checkpoint 4: [`.scratch/op-working/research-checkpoint-4-talk-continue-menu.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-4-talk-continue-menu.md)
- Research Checkpoint 5: [`.scratch/op-working/research-checkpoint-5-quest-sync.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-5-quest-sync.md)
- Research Checkpoint 6: [`.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md)

### Checkpoint 1 (Đã hoàn thành 100%)
- **Mục tiêu**: Xây dựng toàn bộ Wire Codec cho Opcode `0x14` (Hội thoại NPC / Lock di chuyển) và Opcode `0x18` (Đồng bộ Quest / Vật phẩm) chuẩn hóa theo decompile Ghidra C client `aLogin.exe` (`FUN_0078ec3f`, `FUN_00790ed5`).
- **Mã nguồn triển khai**:
  - [`src/protocol/codecs/npc_talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs): `EveResultWire` (chuẩn 14 bytes LE), `talk_step_packet` (`0x14 0x01`), `lock_unlock_actor_packet` (`0x14 0x2C`), `end_talk_packet` (`0x14 0x08`), enum `TalkLockMode`.
  - [`src/protocol/codecs/quest_sync.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/quest_sync.rs): 8 SubOpcodes tương ứng `0x18 Sub 0x01..0x08` (actor state, item add/remove, quest task/dont single & bulk, quest clear/full).
  - [`src/protocol/reader.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/reader.rs) & [`src/protocol/writer.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/writer.rs): Tích hợp đọc/ghi các struct nhiệm vụ và thoại.
  - [`src/protocol/codecs/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/mod.rs) & [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs): Export các module.
- **Kiểm thử**: [`tests/wire_codec_14_18_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/wire_codec_14_18_test.rs) đạt **11/11 PASS (100%)**.

### Checkpoint 2 (Đã hoàn thành 100%)
- **Mục tiêu**: đấu nối sự kiện Click NPC (`Opcode 0x14`) từ Dispatcher vào Eve Engine Resolver.
- **Mã nguồn triển khai**:
  - [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs):
    - Kiểm tra bán kính khoảng cách player - NPC ($\le 150$px). Nếu quá xa hoặc NPC không tồn tại, trả về ngay frame `0x14 Sub 0x08` (`F4 44 02 00 14 08`) để giải phóng client an toàn.
    - Xây dựng `EveStateBuilder::build_from_session()`, phân giải qua `eve_engine.resolve_npc_event(...)`.
    - Khi có kết quả: Khởi tạo và gán `current_event_session` vào `Session`, gửi packet kích hoạt mở thoại ban đầu (`0x06 Sub 0x02`).
    - Khi không khớp kịch bản: fallback sang talk thường an toàn.
  - [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs): Bổ sung `pub current_event_session: Option<ActiveEveSession>`.
- **Kiểm thử**: [`tests/npc_eve_resolve_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_eve_resolve_test.rs) đạt **3/3 PASS (100%)** với dữ liệu `eve.emg` thực tế của Trác Quận và các edge cases (NPC xa >150px, NPC không tồn tại, NPC có điều kiện thỏa mãn / không thỏa mãn).

### Checkpoint 3 (Đã hoàn thành 100%)
- **Mục tiêu**: Truyền phát packet thoại 14-byte payload (`0x14 Sub 0x01`) và khóa/mở khóa thao tác nhân vật (`0x14 Sub 0x2C`) khi bắt đầu/kết thúc thoại — đúng thứ tự wire của Bear `ClickkNpc`/`processStep`.
- **Mã nguồn triển khai**:
  - [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs):
    - Import `NpcTalkCodec`, `TalkLockMode` từ `crate::protocol::codecs::npc_talk`.
    - `handle_talk_start` (nhánh Eve match): sau khi resolve được `event_session`, build hex talk step của `results[0]` (chỉ khi `result_type == 1`) **trước khi move** vào `conn.session.current_event_session`. Thứ tự frame phát đi:
      1. `F44402000602` — mở bảng thoại
      2. `F4440700142C<char_id LE>01` — Lock actor (`NpcTalkCodec::build_talk_lock_hex(char_id, TalkLockMode::Lock)`, `char_id = conn.session.id as u32`)
      3. `F4441100140100<14B EveResult>` — talk step đầu tiên (byte-perfect từ `build_talk_step_hex`)
      - Non-talk results (`result_type` 0 Action / 2 Door / 6 Surface) đánh dấu `// TODO(CP4)` — đã được Checkpoint 4 dispatch (xem dưới).
    - `end_talk`: khi `conn.session.current_event_session.is_some()` → gửi Unlock (`...14 2C ... 02`) **trước** `F44402001408`. Legacy path (không có event session) giữ nguyên frame đơn `F44402001408` để bảo toàn golden parity.
  - [`tests/db_repository_init_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/db_repository_init_test.rs): Fix lỗi pre-existing — 3 struct literal `Session` thiếu field `current_event_session` (kế thừa từ Checkpoint 2) khiến `cargo test --all-targets` không compile → thêm `current_event_session: None`.
- **Kiểm thử**: [`tests/npc_talk_transmit_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_talk_transmit_test.rs) đạt **3/3 PASS (100%)**:
  - `test_click_npc_emits_lock_then_first_talk_step`: assert thứ tự index `0602` < lock `F4440700142CE903000001` < talk step `F4441100140100...7C28` (mean_no 10364 LE).
  - `test_end_talk_emits_unlock_before_close`: unlock `F4440700142CE903000002` xuất hiện trước `F44402001408`.
  - `test_talk_step_payload_matches_eve_result`: frame emit khớp byte-perfect với `build_talk_step_hex(&ev.results[0])`, decode đúng 21 bytes.

### Checkpoint 4 (Đã hoàn thành — 2 điểm chờ human confirm, xem §1.1)
- **Mục tiêu**: Tiếp tục thoại (`0x14 Sub 0x06`, payload C→S rỗng) và chọn menu (`0x14 Sub 0x09`, payload `[ChoiceCode]` 1 byte) — máy trạng thái duyệt `EventSession.results`, dispatch Action non-blocking, phân nhánh Surface theo `condition_class 10`.
- **Phạm vi đã chốt với user**: Door (`result_type 2`) / Battle (`result_type 3`) **chỉ kết thúc thoại an toàn** (unlock + `14 08`) + `// TODO(CP6)` — không dispatch warp/battle trong CP4.
- **Mã nguồn triển khai** (2 file src, giữ nguyên golden parity cho legacy path):
  - [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) (+321 dòng):
    - `reset_talk_context(conn)` — nửa không-packet của `end_talk`, dùng chung cho finish path; `eve_rng_seed()` — seed wall-clock ms thống nhất.
    - **`execute_event_step(conn, data, out)`** — bộ duyệt trung tâm từ `current_index`: type 1 (Talk) → gửi `build_talk_step_hex` & dừng chờ Sub 6; type 6 (Surface) → ghi `last_surface_id = result_mean_no`, `phase = AwaitingChoice`, gửi frame & dừng chờ Sub 9; type 0 (Action) → `execute_action_result` rồi `current_index += 1` & đi tiếp trong cùng lần gọi (non-blocking); type 2/3 → `end_talk` an toàn + `TODO(CP6)`; type lạ → skip + `tracing::debug` (không panic); hết queue → `finish_event_session`.
    - **`finish_event_session`** — unlock `..02` → `F44402001408` → `npc_event::auto_chain_after`: `Chained` → gán session mới + `0602` + lock `01` + gọi `execute_event_step` (đệ quy bị chặn bởi `MAX_CHAIN_DEPTH = 10` của engine); `NoMatch` → `reset_talk_context`.
    - **`execute_action_result`** — class 1 (item): dấu của `result_value` là give/take discriminator (`pstyle=0` cả 2 case trong data thật), dùng `remove_homdo_item` / `inventory::from_template` + `add_homdo_item` + sync `dump_homdo()`; class 2 (quest flag) & class 7 (exp/gold) & khác: skip + `tracing::debug` + TODO trung thực (không bịa executor — Session chưa có mission store/EXP field).
    - `handle_talk_start` — hết `TODO(CP4)`: `results[0]` là Talk giữ nguyên nhánh CP3; nếu không → gửi `0602` + lock rồi gọi `execute_event_step`.
    - `handle_talk_continue` (Sub 6) — **nhánh Eve đứng đầu hàm**: đang `AwaitingChoice` thì bỏ qua stray continue (log debug); nếu không → `current_index += 1` (index trỏ result đã giao, caller advance trước khi đi tiếp) → `execute_event_step` → `return`. Session `None` → toàn bộ guard/branch legacy giữ nguyên 100%.
    - `handle_talk_select_menu` (Sub 9) — signature mới `(conn, payload, data, out)` (call site đã cập nhật): đang `AwaitingChoice` → `choice == 0` bỏ qua; ghi `last_choice_code`/`select_menu`, `phase = Executing`, `current_index = surface_idx + 1`, dựng lại `NpcTrigger` từ `trigger_kind`/`npc_click_id`, `resolve_npc_event` lại; match → **kế thừa `last_surface_id`/`last_choice_code` vào branch session** (bắt buộc để auto-chain guard #2 chặn vòng lặp Surface→branch→Surface) + `execute_event_step`; `None` → `end_talk` an toàn. Không có session / không phải `AwaitingChoice` → legacy `select_menu = payload[0]` y như cũ.
  - [`src/server/handlers/npc_event.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs): `snapshot_state` đọc `last_surface_id`/`last_choice_code` từ `session.current_event_session` nếu có, ngược lại `(-1, -1)` — bắt buộc để `condition_class 10` match. (Thay đổi "ý thức" duy nhất ngoài `talk.rs`.)
  - `src/server/session.rs`: **không đổi** (đã có `add_homdo_item`/`remove_homdo_item` public).
- **Data `eve.emg` thật dùng cho 3 test** (khảo sát bằng test exploratory tạm, đã xóa):
  1. Multi-step: map **10817**, NPC click id **1** (template 33001 @ 590,540) — eve 1, 7 results toàn Talk (mean_no 10364/10367/10442..10446). Sub 6 × 7 → unlock + `1408`; auto-chain tái-match cùng cond → guard #1 (`matchedConditionNo` trùng) → `NoMatch`, **không** phát lại `0602`.
  2. Action: map **10817**, NPC click id **3** (template 33002 @ 950,460) — eve 3 match khi túi có item **32012**; 2 results type 0/class 1: `[32012, value -1]` rồi `[26012, value +1]` → khớp research, chạy inline rồi đóng thoại.
  3. Menu: map **10851**, NPC click id **2** (template 15050 @ 1190,200) — Surface `mean_no=3` → `AwaitingChoice`; Sub 9 `0x1E` (30) → branch `conditionNo=3` (talk đầu mean 10534); Sub 9 `0x1F` (31) → branch `conditionNo=4` (talk đầu mean 10535) — 2 nhánh khác nhau.
- **Kiểm thử** (file mới [`tests/npc_talk_multistep_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_talk_multistep_test.rs), 333 dòng):
  - `test_multistep_talk_continue_walks_to_unlock`, `test_action_results_execute_item_swap_and_close`, `test_menu_surface_and_choice_branching` — **3/3 PASS**.
  - Toàn bộ targeted suite: `npc_talk_multistep` 3/3 + `npc_talk_transmit` 3/3 + `npc_eve_resolve` 3/3 + `wire_codec_14_18` 11/11 = **20/20 PASS**, đã được coordinator chạy lại xác nhận独立 (không tin báo cáo subagent suông).
- **Kết quả**: không chạy `cargo test --all-targets` (user yêu cầu chỉ test targeted file đã sửa/ tạo mới).

#### 1.1. Deviation & kết quả confirm
1. **ChoiceCode 30/31 lệch research CP4 — ĐÃ CHỐT bằng đối chiếu mã nguồn server C#** (`TS_Server_Bear/`): research §2.2 nói 1-based `01/02/03` là **SAI** với luồng hoạt động thực tế. Chuỗi bằng chứng 3 mắt xích:
   - [`TS_Server_Bear/TS_Server/PacketHandlers/ActionHandler.cs:34-36`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/TS_Server_Bear/TS_Server/PacketHandlers/ActionHandler.cs): `case 9: client.selectMenu = data[2]` — Bear đọc **raw byte** từ `0x14 Sub 0x09`, **không normalize** 1-based → 30/31.
   - [`TS_Server_Bear/TS_Server/DataTools/EveData.cs:1301-1307`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/TS_Server_Bear/TS_Server/DataTools/EveData.cs) + [`DataTools/QuestLogics/ConditionTypeParserAdapter.cs:98-104`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/TS_Server_Bear/DataTools/QuestLogics/ConditionTypeParserAdapter.cs): condition type 10 parse `optionId` **trực tiếp từ data file** (`read16` / `bit_4`) — không quy đổi.
   - [`TS_Server_Bear/TS_Server/Client/QuestStepHelper/StepMenuSelectionHandler.cs:20`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/TS_Server_Bear/TS_Server/Client/QuestStepHelper/StepMenuSelectionHandler.cs): match `x.optionId == client.selectMenu` — **raw equality**. Data thật chứa 30/31 ⇒ client **phải** gửi 30/31 thì mới match được (client gửi 01/02 sẽ không bao giờ khớp optionId nào). Cộng chứng: [`TSClient.cs:2823,2844`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/TS_Server_Bear/TS_Server/Client/TSClient.cs) hardcode `selectMenu == 30` (lựa chọn đầu) / `40` (đóng) = đúng quy ước legacy H6.
   - $\implies$ **Implementation hiện tại đúng**: lưu raw byte ChoiceCode, match `last_surface_id` (= `idDialog`) + `last_choice_code` (= `optionId`) — tương đương server C#. **Đã cập nhật research CP4 §2.2 + §3.3** (2026-09-22) theo finding này.
2. **Sửa [`tests/npc_eve_resolve_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_eve_resolve_test.rs)** ngoài danh sách file được phép: assertion cũ `current_event_session.is_some()` với NPC 3 là stale (viết thời CP3 khi Action chưa dispatch). Đã thay bằng: session `None` (eve 3 chạy xong trong click) + có `F44402001408` + túi bị trừ 32012/cộng 26012. **User đã duyệt (2026-09-22).**
3. Kế thừa `last_surface_id`/`last_choice_code` vào branch session khi Sub 9 (spec không nói rõ, bắt buộc để guard #2 hoạt động — đã trace trên data map 10851).
4. Stray Sub 6 trong lúc `AwaitingChoice` bị ignore (spec không đề cập; an toàn hơn là nhảy qua Surface).
5. `finish_event_session` **không** auto-chain cho nhánh Door/Battle safe-end — nếu CP6 muốn chain ngay sau door thì mở lại.

### Checkpoint 5 (Đã hoàn thành 100% — 2026-09-22)
- **Mục tiêu**: Triển khai kênh đồng bộ quest & cờ trạng thái Opcode `0x18 Sub 0x01..0x08` (S→C): Session fields + session-facing handler + login bulk sync. Codec vốn đã sẵn từ CP1.
- **Mã nguồn triển khai**:
  - [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs): +3 fields — `quest_tasks: HashMap<u16, (u8, u8)>` (quest_id → slot, mark_step), `quest_dont: HashSet<u16>`, `quest_items: Vec<InventoryItem>` (túi quest tách `homdo`).
  - [`src/server/handlers/quest_sync.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest_sync.rs) (mới, ~270 dòng): hằng số biên client (`QUEST_SLOT_MAX=200`, `QUEST_DONT_MARK_MIN/MAX=1..=300`, `QUEST_ITEM_STACK_CAP=255`), allocator slot dùng chung `next_free_slot`, và các mutator 1-1 với frame: `add_quest_item` (Sub 1), `remove_quest_item` (Sub 2), `clear_quest_item` (Sub 4), `set_quest_dont` (Sub 5), `set_quest_task` (Sub 6 single), `send_actor_state_flag` (Sub 8), `sync_frames` (bulk login: Sub 6 bulk → Sub 7 bulk → Sub 1/item).
  - [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs): **Step 22** của chuỗi `Logined1` — `frames.extend(quest_sync::sync_frames(s))`, khớp thứ tự Bear `loginChar` (tasks → dont, sau inventory dumps). Trạng thái rỗng ⇒ **không frame `0x18` nào** → giữ nguyên golden parity.
  - [`src/server/handlers/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/mod.rs): export `pub mod quest_sync`.
  - [`tests/db_repository_init_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/db_repository_init_test.rs): 3 struct literal `Session` exhaustive được thêm 3 field mới (cùng lỗi CP3 đã từng sửa).
  - Dispatcher **không đổi**: `0x18` là kênh S→C thuần (aLogin không gửi `0x18` C→S), nên nhánh C→S vẫn nằm ở unimplemented boundary có kiểm soát.
- **Deviation & phát hiện mới** (chi tiết: research CP5 §5, cập nhật 2026-09-22):
  1. **Túi quest & quest log dùng CHUNG mảng 200 row của client** — `FUN_00720ca8` (Sub 1) và `FUN_0072bb6c` (Sub 6) đều ghi `LocalActor + 0x654 + slot*3`. ⇒ Server cấp slot từ **một pool dùng chung** cho cả `quest_items` và `quest_tasks` (khác Bear: `TaskQuest.Count + 1` → có thể đè row của item).
  2. **Biên client hard-fail (`_BoundErr`)**: slot `1..=200`, mark `1..=300` (mark 0 underflow), stack quest item `≤255`, và `FUN_00720df0` **từ chối** remove khi `count > owned`. Mutator đều reject trước wire để hai túi không desync.
  3. **Sub 0x03 là tín hiệu "full" duy nhất** ⇒ phát `F44402001803` khi add bị từ chối (stack tràn / hết row). Bear không bao giờ gửi Sub 1/2/3/7/8 → các nhánh này suy từ decompile, chưa có capture thật.
  4. **Lỗi hex trong research §4**: `build_quest_task_hex(1, 10801, 3)` đúng là `F4440600180601312A03` (payload 6B), research ghi `0500` là typo — đã đối chiếu `FUN_0072bb6c` stride 4 và `tests/wire_codec_14_18_test.rs`.
  5. Bear `refreshQuestTask` gửi `add16(mark)` (thừa 1 byte) — client bỏ qua nhờ tính `Len>>2`; codec của ta giữ entry 4B đúng đặc tả.
  6. Không có wire "xóa quest log" (khác item Clear Sub 4) ⇒ `quest_tasks` không có hàm remove; hoàn thành quest sẽ do CP6 (mission store) định nghĩa.
- **Kiểm thử**: [`tests/quest_sync_18_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/quest_sync_18_test.rs) (mới) **7/7 PASS**:
  `test_research_builder_hexes_match_wire` (8 sub-opcode), `test_add_merge_remove_clear_quest_item`, `test_quest_item_capacity_refuses_with_full_toast` (200 row + stack 255), `test_quest_dont_marks_and_client_bounds`, `test_quest_task_rows_share_pool_with_items` (item chiếm row 1 → task nhận row 2), `test_actor_state_flag_frame`, `test_login_sequence_syncs_quest_state_only_when_present`.
- **Regression targeted** (không chạy `--all-targets` theo yêu cầu user): `db_repository_init_test` 11/11 + `login_char_flow_test` 5/5 + 4 suite CP1–4 (`npc_talk_multistep` 3/3, `npc_talk_transmit` 3/3, `npc_eve_resolve` 3/3, `wire_codec_14_18` 11/11) + `wiring_hotkey_notice` 6/6 — **tổng 49/49 PASS**, đã tự chạy xác nhận.

---

## 2. Current State & Next Steps

Hệ thống đã sẵn sàng cho **Checkpoint 6**:

- **Checkpoint 6 (các phần còn lại)**:
  - Door/Warp (`result_type 2`) & Battle trigger (`result_type 3` → `BattleTrigger::Eve`) — hiện đang safe-end + `TODO(CP6)` trong `execute_event_step`.
  - Action class 2 (Eve mission store — `TODO(ticket 08)`) & class 7 (exp/gold selector mapping — chưa proven).
  - Nối `quest_tasks`/`quest_dont` vào `npc_event::snapshot_state` (`missions`/`raw_flags` đang là rỗng) để điều kiện Eve đọc được tiến trình quest đã sync ở CP5.
  - Mở lại auto-chain cho nhánh Door/Battle nếu cần (deviation #5 của CP4).
  - SQLite Persist: nạp `quest_tasks`/`quest_dont` khi login (trước Step 22) + ghi-through khi `has_state_changing_results()`.
  - Nghiên cứu: [`.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md).
- **Việc cần làm ngay (human/agent)**:
  1. User review & commit thủ công các file CP5: `src/server/session.rs`, `src/server/handlers/quest_sync.rs`, `src/server/handlers/mod.rs`, `src/server/spawn.rs`, `tests/quest_sync_18_test.rs`, `tests/db_repository_init_test.rs` (cùng doc: handoff này + research CP5).
  2. Các file CP4 vẫn chờ commit: `src/server/handlers/talk.rs`, `src/server/handlers/npc_event.rs`, `tests/npc_talk_multistep_test.rs`, `tests/npc_eve_resolve_test.rs`, research CP4.

---

## 3. Testing Rules Reminder
Tuân thủ tuyệt đối quy định trong [`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md):
- **Cấm** viết `#[cfg(test)]` inline trong thư mục `src/`.
- Mọi unit / integration test mới bắt buộc phải nằm ở thư mục `tests/`.
- **Ràng buộc bổ sung từ user (từ CP4 trở đi)**: **hạn chế `cargo test --all-targets`** — chỉ chạy `cargo test --test <tên_test>` với các file test đã **thay đổi hoặc tạo mới** trong lượt làm việc. Khi cần kiểm regression phạm vi hẹp, dùng 4 lệnh targeted:
  ```bash
  cargo test --test npc_talk_multistep_test
  cargo test --test npc_talk_transmit_test
  cargo test --test npc_eve_resolve_test
  cargo test --test wire_codec_14_18_test
  cargo test --test quest_sync_18_test
  cargo test --test db_repository_init_test   # sửa Session literal
  cargo test --test login_char_flow_test      # sửa chuỗi Logined1
  ```
- Không tự ý commit source — user sẽ commit thủ công.

---

## 4. Suggested Skills
Khi tiếp tục làm việc, các subagents hoặc agent kế tiếp nên sử dụng:
- `DeepCoder`: Cho các bước implement mã nguồn và chạy test suite targeted.
- `DeepInvestigator`: Khi cần tra cứu decompile Ghidra C của `aLogin.exe` hoặc cấu trúc opcode trong client (kỳ tới dùng để xác minh byte ChoiceCode client gửi — deviation #1).
- `handoff`: Khi kết thúc một checkpoint tiếp theo để cập nhật tài liệu bàn giao.
