# Handoff: NPC Talk & Eve Script System — Checkpoint 1–6 (Opcodes 0x14 & 0x18)

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

### Checkpoint 6 (Đã hoàn thành 100% — 2026-09-22)

- **Mục tiêu**: đóng nốt phần còn lại của Eve pipeline — Door/Battle dispatch trong walker trung tâm, trigger Eve battle từ server, Action executor class 2/5/7, nối quest/mission vào `snapshot_state`, auto-chain sau Door/Battle, SQLite persist cho quest/Eve state, và bộ targeted test E2E.
- **Spec & 11 quyết định design**: nằm ở [`.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/research-checkpoint-6-autochain-battle-persist.md) — tài liệu này chỉ ghi phần code đã triển khai + test, không lặp lại spec.
- **Mã nguồn triển khai** (7 file src + 1 migration mới + 1 test mới):
  - [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs) (file chính của CP6):
    - `execute_event_step` — **Door** (`result_type 2`): tra `data.warps[(map_id, parameter)]` → `quest::perform_warp` → `finish_event_session` (quyết định CP6 #11: Door tự kết thúc session — warp đã đẩy player khỏi map nên auto-chain ra `NoMatch`); warp thiếu → skip (advance + đi tiếp); party follower (`id_leader > 0 && != id`) → từ chối warp + đóng thoại an toàn. **Battle** (`result_type 3`): pre-check `scene.fight_datas[result_mean_no]` (thiếu row → skip, **không bao giờ park**), `diahinh = scene_infos[1].background_no` fallback **112**, park `phase = AwaitingBattle` với index giữ trên battle result (quy ước Talk/Sub 6), giao `out.eve_battle = (fight_id, diahinh)` rồi trả về — không gửi frame nào.
    - `execute_action_result` — **class 2** (Bear `QuestSaveHandler` parity): save gate `pStyle1 ∈ {1,2,3,10,30,50,70}` / `pStyle4` (luôn) / `pStyle2 ∈ {1,9}`; `pStyle3 + step0` → `remove_quest_task` (frame `0x18 Sub 0x04` clear-by-id); quest mới → `step.max(1)`; quest cũ → `current + step`, hoặc `current - 1` khi `battle_result ∈ {2,3}` (đọc từ `current_event_session`). **class 5** (gold, `GoldEffectHandler`): type1 `< 1000` → cộng `point` theo tỷ lệ 20:1, `≥ 1000` → cộng `gold` + frame `gold_frame`; type2 debug-skip. **class 7** (`StatBonusAndBallEffectHandler` + `GetSaveMap`): `pStyle1` → `save_map`; `param1 + pStyle2/3` → `skill_point` (stat `0x25`) / `point` (stat `0x26`); type4 (EXP) & army → deferred có log.
    - **`resume_eve_after_battle(session, outcome, data) -> (Vec<String>, Option<(u16, i32)>)`** (`pub`, đặt trong `talk.rs` vì nó drive máy trạng thái dialog của module này — `battle::service` chỉ forward): **Win** → `battle_result=1`, `phase=Executing`, index+1, walk tiếp tới điểm cần input; **Lose/Flee** → ghi 2/3 rồi `end_talk_session` (unlock + `1408`, **không** auto-chain sau trận thua); `Running` / session chưa park → no-op. `next_battle` mang eve battle phát sinh **trong lúc** walk resume (trận thứ 2 cùng session).
    - Guards mới: Sub 6 (`handle_talk_continue`) và Sub 9 (`handle_talk_select_menu`) bị **ignore** trong lúc `AwaitingBattle` — stray packet không advance qua battle result đang park (advance thuộc về resume).
    - Walker refactor: mọi eve-path fn nhận `&mut Session` (không phải `&mut Conn`) — resume chạy từ sync `BattleSink` callback nên chỉ có session guard.
  - [`src/battle/service.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/service.rs): `start_eve_encounter(&self, session, fight_id, diahinh) -> i32` (tra `scene.fight_datas`, trả `0` khi thiếu → caller không được park); `BattleSinkImpl.eve_service: OnceLock<Weak<BattleService>>` + `install_eve_backref(&Arc<Self>)`; `battle_ended` gọi resume **bên trong** vòng xoá `members` và **trước** snapshot `ended_sessions` (stat write của resume kịp persist), collect `eve_followups` rồi start **sau khi** `members` unlock; thiếu backref → `tracing::warn` + skip (không bao giờ kẹt `AwaitingBattle`). Gate legacy `battle_quest_win` bằng `talking_battle <= 0`.
  - [`src/web/server_control.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs): `BattleService::install_eve_backref(&battle_service)` ngay sau `Arc::new`; start eve battle (`start_eve_encounter` + re-insert online registry) **ngay sau khi flush xong** `out.outgoing` — FIFO của connection channel đảm bảo talk frames luôn đứng trước battle frames.
  - [`src/server/handlers/npc_event.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs): `snapshot_state` wired — `missions` từ `quest_tasks` (qid → step), `battle_result`/surface/choice từ `current_event_session` (mặc định `-1/-1/0`), `completed_eve_counts` clone, `quest_dont` → `mission_flags` (key-space assumption, inert tới khi có setter), `mark_defs` để rỗng (CP6 #6).
  - [`src/server/handlers/shops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs): `gold_frame` → `pub`.
  - [`src/server/auto_save.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/auto_save.rs): fingerprint mix thêm `quest_tasks` / `quest_dont` / `quest_items` / `completed_eve_counts` — **sort key trước khi mix** (HashMap iteration order random per instance).
  - [`src/db/modern/sqlite/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs): `load_eve_state` (**private**, hook best-effort ở cuối `load()`) + `save_eve_state` (**pub**, transaction **riêng** sau `tx.commit()` của `save()`); thiếu bảng 0002 → `tracing::debug` + skip, login/save không fail (ADR 0004).
  - [`migrations/0002_eve_persistence.sql`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/migrations/0002_eve_persistence.sql) (**mới**): `ALTER TABLE character_completed_events ADD COLUMN completioncount` + `character_quest_tasks` / `character_quest_dont` / `character_quest_items`. **`pool::migrate` là no-op (ADR 0004) ⇒ phải apply MỘT LẦN thủ công cho DB production:**
    ```bash
    sqlite3 DB/ts_dream.db < migrations/0002_eve_persistence.sql
    ```
    Trước khi apply, server vẫn chạy bình thường (best-effort degrade) — test `persistence_degrades_without_migration_0002` chứng minh.
- **Kiểm thử**: [`tests/npc_eve_e2e_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/npc_eve_e2e_test.rs) (**mới, 19 tests — 19/19 PASS**):
  - class 2 (4 test): quest mới + floor step 1 + completion count, increment, battle-backup (`resBattle` 2 → −1), remove (`pStyle3+step0`), save-gate skip — mỗi case assert đúng frame `QuestSyncCodec` + tail unlock/`1408`.
  - class 5 (1) + class 7 (1): point×20 / gold / type2 skip; skill_point `0x25`, point `0x26`, `pStyle1` → `save_map`, army+type4 deferred không đổi state.
  - Door (3): warp byte-identical (`1407` fade → `0x0C` relocate → hide broadcast → unlock/`1408`, registry cập nhật, **không** bump completion watermark), missing-warp skip, party follower từ chối.
  - Battle (3): **flush order** — walker entry 1 giao Talk frame, walker entry 2 qua `dispatch(0x14 Sub 6)` park với `eve_battle = Some((7, 55))` và 0 frame (chứng minh talk frames đứng trước battle frames); stray Sub 6 / Sub 9 lúc `AwaitingBattle` bị ignore; missing fight/scene skip; `diahinh` fallback 112.
  - resume (3): Win → advance + quest write + count bump + `next_battle=None`; Lose/Flee → unlock + `1408`, session sạch, không auto-chain; `Running` & unparked → no-op.
  - `snapshot_state` wiring (1), persistence round-trip qua `save()`/`load()` + đọc trực tiếp cột `completioncount` (1), degrade không có 0002 trên tempfile DB (1), fingerprint sensitivity + insertion-order independence (1).
  - Kỹ thuật: synthetic `GameData` (không load `Data/`), `EventSession` `eve_no=5` + `scene.npcs` rỗng → auto-chain luôn `NoMatch` tất định; frame assert so với chính codec pub server dùng.
- **Regression targeted** (từng lệnh một, không `--all-targets`): `quest_sync_18` 7/7 + `db_repository_init` 11/11 + `movement_warp` 8/8 + `create_char_atomic` 5/5 + `npc_eve_resolve` 3/3 + `npc_talk_multistep` 3/3 — cùng `npc_eve_e2e` 19/19 ⇒ **tổng 56/56 PASS**.
- **Deviation đã ghi nhận** (chi tiết research CP6 §decisions): gold bão hòa `u16::MAX` (khác Bear 1e9 — `Session.point` là `u16`); class 2 quest cũ + step0 → `current+0` (Bear outer guard là no-op); `quest_dont → mission_flags` key mark↔mission chưa có setter; `load_eve_state` giữ **private** (test round-trip đi qua hook public `save()`/`load()` thật).

---

## 2. Current State & Next Steps

**Checkpoint 6 đã hoàn thành** — toàn bộ CP1–CP6 xong, tất cả file **chưa commit** (repo không có remote, agent không tự commit).

- **Việc cần làm ngay (human)**:
  1. Apply migration 0002 thủ công cho DB production (1 lần): `sqlite3 DB/ts_dream.db < migrations/0002_eve_persistence.sql`.
  2. Review & commit thủ công. File CP6: `src/server/handlers/talk.rs`, `src/server/handlers/npc_event.rs`, `src/server/handlers/shops.rs`, `src/battle/service.rs`, `src/web/server_control.rs`, `src/server/auto_save.rs`, `src/db/modern/sqlite/session.rs`, `migrations/0002_eve_persistence.sql`, `tests/npc_eve_e2e_test.rs` (+ research CP6 + handoff này). File CP4/CP5 vẫn chờ commit: `src/server/handlers/quest_sync.rs`, `src/server/handlers/mod.rs`, `src/server/spawn.rs`, `src/server/session.rs`, `tests/quest_sync_18_test.rs`, `tests/npc_talk_multistep_test.rs`, `tests/npc_eve_resolve_test.rs`, `tests/db_repository_init_test.rs`.
  3. (Tùy chọn) Bật eve feature `TS_EVE_EVENTS=1` để smoke-test live; mặc định **tắt** để giữ golden parity (resolve trả `None` khi tắt → replay không đổi).

---

## 3. Testing Rules Reminder
Tuân thủ tuyệt đối quy định trong [`AGENTS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/AGENTS.md):
- **Cấm** viết `#[cfg(test)]` inline trong thư mục `src/`.
- Mọi unit / integration test mới bắt buộc phải nằm ở thư mục `tests/`.
- **Ràng buộc bổ sung từ user (từ CP4 trở đi)**: **hạn chế `cargo test --all-targets`** — chỉ chạy `cargo test --test <tên_test>` với các file test đã **thay đổi hoặc tạo mới** trong lượt làm việc. Khi cần kiểm regression phạm vi hẹp, dùng các lệnh targeted:
  ```bash
  cargo test --test npc_eve_e2e_test           # CP6 suite mới (19 tests)
  cargo test --test npc_talk_multistep_test
  cargo test --test npc_talk_transmit_test
  cargo test --test npc_eve_resolve_test
  cargo test --test wire_codec_14_18_test
  cargo test --test quest_sync_18_test
  cargo test --test db_repository_init_test   # sửa Session literal
  cargo test --test login_char_flow_test      # sửa chuỗi Logined1
  cargo test --test movement_warp_test        # fingerprint + warp (8)
  cargo test --test create_char_atomic_test   # sửa Session literal (5)
  ```
- Không tự ý commit source — user sẽ commit thủ công.

---

## 4. Suggested Skills
Khi tiếp tục làm việc, các agent kế tiếp nên invoke các skill có sẵn trong repo:
- `implement`: Các bước implement mã nguồn theo spec/ticket đã chốt + chạy test suite targeted.
- `diagnosing-bugs`: Khi test fail hoặc có regression sau khi sửa `talk.rs` / `service.rs` / persistence.
- `code-review`: Review thay đổi từ một commit/điểm cố định trước khi user commit thủ công (2 nhánh Standards + Spec).
- `tdd`: Khi bổ sung test cho executor/codec mới (red-green trong `tests/`, không `#[cfg(test)]` trong `src/`).
- `handoff`: Khi kết thúc một checkpoint tiếp theo để cập nhật tài liệu bàn giao.
