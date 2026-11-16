# Báo Cáo Kiểm Chứng & Đối Chiếu: `EVE_QUEST_100_PERCENT_EVALUATION.md`

**Mục đích**: Đối chiếu độc lập mọi nhận định từ **P0 đến P2** và **Khoảng trống 1..4** trong [`EVE_QUEST_100_PERCENT_EVALUATION.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_QUEST_100_PERCENT_EVALUATION.md) với mã nguồn Rust hiện tại (`src/`), pseudo-C (`client_pseudo_c/`), dữ liệu gốc (`Data/eve.emg`, `Data/Mark.dat`, `Data/Warps.txt`, `Data/Scene.Dat`), DB (`DB/ts_dream.db`, `migrations/`) và tài liệu bàn giao (`HANDOFF_01_10.md`).

---

## 1. Kết luận tổng quan (Executive Verdict)

Nhìn chung, **kiến trúc và các lỗ hổng P0–P2 trong `EVE_QUEST_100_PERCENT_EVALUATION.md` phản ánh ĐÚNG bản chất tình trạng server Rust hiện tại** — đặc biệt là hai vấn đề **P0** (`should_skip_event` và bỏ đọc `dest_map_id` ở `eve.rs:738`), các nút thắt hiệu năng/đồng thời **P1**, và các điểm tối ưu **P2**.

Tuy nhiên, quá trình đối chiếu trực tiếp mã nguồn và dữ liệu gốc phát hiện **05 điểm sai lệch/lỗi thời về code**, **01 mâu thuẫn về thứ tự thực thi hot-path**, **03 sai lệch số liệu thống kê nhỏ**, và **03 điểm kỹ thuật cần làm rõ thêm** trước khi triển khai sửa lỗi.

| Mức độ / Mục | Phán quyết | Tóm tắt kết quả kiểm chứng |
|---|---|---|
| **P0 — `should_skip_event`** | **Chính xác (cần lưu ý thêm scope gọi)** | `auto_chain.rs:288-310` và `npc_event.rs:257, 326` khớp hoàn toàn. Cần chú ý thêm `try_auto_chain` (`auto_chain.rs:170-175`) cũng gọi `should_skip_event` khi `battle_result == 0`. |
| **P0 (Lộ trình) — Warp `rt=2` & `diahinh`** | **Chính xác** | `eve.rs:738` skip `2` byte `u16` (`005efee8_FUN_005efee8.c:548`); `4,857/10,336` (`47.0%`) thiếu ở `Warps.txt`; `diahinh` hardcode `scene_infos.get(&1).background_no`. |
| **P1 — `when_happen`, `use_mode`, `resolver`, `mark_dont_on_step1`** | **Chính xác logic, lệch nhỏ số liệu `resolver`** | `when_happen` (0 reader, `ClickDoor`/`MeetDoor` dùng chung `door_events_by_door`), `use_mode` (9 nhóm), `mark_dont_on_step1` (7 quest `Mark.dat pos > 0`) đều đúng. Riêng `chain[1..].results` là **29 node thuộc 15 chuỗi AND** (không phải 29 chuỗi). |
| **P1 — Hot-path `movement.rs` + `npc_event.rs`** | **Chính xác phần cấp phát, SAI thứ tự thực thi** | Đúng là `snapshot_state_with_data` lặp `2,394` mục `quest_marks` trước khi lọc `scene.encounters`. Nhưng `try_trigger_random_encounter` (`movement.rs:175`) chạy **TRƯỚC** `WalkStep` (`L77`), không phải sau. |
| **P1 — `run_sync_future` & Lock ordering** | **Chính xác (bổ sung thêm vị trí)** | `run_sync_future` (`eve_runner.rs:1025-1041`) chỉ dùng trong `tests/` (0 caller production). Lock ordering vi phạm ở `event_npcs.rs:152, 366, 444` & `battle.rs:429` là thật (và còn ở `movement.rs:135-143, 161-169`, `battle/construction.rs:164-172`). |
| **P1 / Research — Độ phủ & Khoảng trống 2, 3, 4** | **Có 3 điểm sai lệch/lỗi thời quan trọng** | `rc=5` (`430`x) **đã cài đặt**; `cc=7 param=2 p_style=4` **không fail-closed** mà chạy thẳng vào `reborn_count`; `0x46 BoatRace` & `0x41 Rank` **không** đi qua `unimplemented.rs`. |
| **P2 — Re-plan, `Vec<u8>/<u16>`, Docs & DB** | **Chính xác** | `execute_event_step_core` (`eve_runner.rs:900-916`) và `Vec<u8>/<u16>` khớp hoàn toàn; các điểm vênh `HANDOFF_01_10.md` đúng; `DB/ts_dream.db` hiện là file rỗng `4,096 B` (`0` bảng). |

---

## 2. Chi tiết kiểm chứng P0 -> P2

### 2.1. [P0] `EveAutoChainEngine::should_skip_event`
- **Xác nhận khớp mã nguồn**:
  - [`src/eve/auto_chain.rs:288-310`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L288-L310): Rule 1 kiểm tra `completed_count > 0 && !has_completion_condition` (trong đó `has_completion_condition` ở L299-302 chỉ kiểm tra `condition_class == 12`). Rule 2 kiểm tra `completed_count == 0 && has_later_completed_event(events, eve_no, state)`.
  - [`src/server/handlers/npc_event.rs:257`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L257) (`WalkStep`) và [`npc_event.rs:326`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L326) (`ClickNpc`/`ClickDoor`/`MeetDoor`) đều gọi trực tiếp `should_skip_event` không có ngoại lệ (ngay cả khi `0x14 0x09` truyền `chain_filter = Some(...)` từ [`talk.rs:438-444`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L438-L444)).
- **Làm rõ điều kiện tăng `completed_eve_counts`**:
  - `completed_eve_counts` được tăng tại 2 nơi trong [`eve_runner.rs:810-816, 880-888`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L810-L816) khi `EventSession::has_state_changing_results()` (`auto_chain.rs:97-104`) trả `true`: tức không phải thua/chạy trận (`battle_result ∉ {2, 3}`) và có ít nhất một `result_type == 0` hoặc `(result_type == 3 && battle_result == 1)`.
  - Do đó, event chỉ có `rt=1` (thoại thuần) hay `rt=2` (warp thuần) không bị đánh dấu completed; nhưng **mọi event nhận quest step 1 (`rt=0, rc=2`) hoặc thắng trận đầu (`rt=3`)** đều bị đánh dấu ngay sau tương tác đầu tiên và bị Rule 1 khóa ở lần tương tác tiếp theo nếu thiếu `cc=12`.
- **Điểm cần chú ý khi sửa**:
  - [`auto_chain.rs:170-175`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L170-L175) cũng gọi `should_skip_event` trong `try_auto_chain` khi `battle_result == 0` (`filter` là `None` theo L159-163). Nếu chỉ gỡ ở `npc_event.rs` mà bỏ sót `auto_chain.rs:171`, auto-chain sau khi nhận quest vẫn bị chặn.
- **Đối chiếu số liệu**:
  - Rule 2: đúng **`430`** NPC placement có `>= 2` event, trong đó **`169`** NPC chứa các event có tập quest rời nhau.
  - Rule 1: tùy cách đếm event đa giai đoạn không có `cc=12`, dữ liệu `Data/eve.emg` cho **`1,559` event / `1,516` quest** (nhiều chuỗi `rt=0, rc=2`) hoặc **`2,281` event / `1,671` quest** (`>= 1` `rt=0, rc=2` và `>= 2` chuỗi `cc=2` đổi state), xác nhận quy mô phá vỡ `1,500+` quest thực tế.

### 2.2. [P0 trong Lộ trình] Khoảng trống 1 — Warp `rt=2` & `diahinh`
- **Xác nhận khớp 100%**:
  - [`src/data/loaders/eve.rs:736-745`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L736-L745) thực hiện `reader.skip(2); // eveFileName`, và `player_appear_x`, `player_appear_y`, `direction` đều có **0 reader** trong toàn `src/`.
  - [`client_pseudo_c/005efee8_FUN_005efee8.c:547-548`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005efee8_FUN_005efee8.c#L547-L548) xác nhận 2 byte sau `background_no` đọc nhị phân thô `u16` (không qua bộ giải mã chuỗi như `areaName`).
  - Toàn bộ đường warp ở [`eve_runner.rs:925-937`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L925-L937), [`quest.rs:928-936, 981-991, 1169-1187`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L928-L936) và [`battle/service.rs:552-610`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L552-L578) đều tra `data.warps` (nạp từ [`Data/Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L370-L376) dù nhiều log vẫn ghi nhầm `"Warps.dat"`).
  - [`eve_runner.rs:731-735`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L731-L735) và [`movement.rs:235-239, 288-293`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L235-L239) đều hardcode `scene.scene_infos.get(&1).background_no` làm `diahinh` (trong khi `Data/ground.mmg` chưa wire vào `GameData`).
- **Đối chiếu số liệu**:
  - `8,882` bản ghi `EveSceneInfo`; **`10,336 / 10,336` (`100%`)** lệnh `rt=2` khớp `(scene_id, r.parameter == SceneInfo.eve_no)`.
  - `Data/Warps.txt` nạp `4,978` khóa (từ `5,043` dòng text); **`4,857 / 10,336` (`47.0%`)** lệnh `rt=2` không tồn tại trong `Warps.txt`.
  - Trên `4,924` khóa chung giữa `Warps.txt` và `SceneInfo`: chỉ `4,247` trùng `dest_map` và `4,067` trùng cả `dest_map` lẫn tọa độ quy đổi pixel `((x * 20 - 10), (y * 20 - 10))`.
- **Điểm chưa rõ**: `aLogin.exe` không dùng `SceneInfo` ở runtime, nên trước khi chuyển toàn bộ sang `EveSceneInfo`, cần kiểm tra biên cho các bản ghi `dest_map_id == 0` hoặc `player_appear_x/y <= 0` (nếu tồn tại trong `Data/eve.emg`), cũng như xác nhận nguồn `diahinh` chuẩn (`SceneInfo` vs `ground.mmg`).

### 2.3. [P1] Đúng đắn ngữ nghĩa Engine
1. **`when_happen`**: Khớp hoàn toàn — 4 cờ được parse tại [`eve.rs:831-835`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L831-L835) nhưng có **0 reader** trong `src/`. Đáng chú ý, `ClickDoor` (`0x14 04`) và `MeetDoor` (`movement.rs:41-59`) đang dùng chung `door_events_by_door` tại [`npc_event.rs:296-300, 311`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L296-L311) nên dễ kích hoạt chéo nhau.
2. **`GroupData.use_mode` & khóa `group_datas`**:
   - Khớp: `select_member` ([`src/eve/group.rs:71-102`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/group.rs#L71-L102)) không kiểm tra `use_mode` (`9` nhóm trong `Data/eve.emg` có `use_mode == 2` kèm `pick_member > 0` hoặc `total_weight > 0`).
   - Làm rõ nghi vấn khóa: đối chiếu [`005efee8_FUN_005efee8.c:568-581`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005efee8_FUN_005efee8.c#L568-L581) cho thấy `u16` đầu tiên (`eve_no` ở [`eve.rs:798`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L798)) đúng là index mảng `GroupData` của client, nên việc khóa `group_datas` theo `eve_no` là nhất quán với binary.
3. **`resolver::resolve` & `chain[1..].results`**:
   - Khớp về code: [`src/eve/resolver.rs:79-133`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/resolver.rs#L79-L133) chỉ kiểm tra `!head.results.is_empty()` và trả về `winner.chain[0].results.clone()` (L129); doc comment L69/L141 ghi `"4-tier"` trong khi `is_better_than` (L143-151) so 3 tiêu chí tường minh và giữ ứng viên đầu khi hòa.
   - **Đính chính số liệu**: trong `Data/eve.emg` có **`29` node điều kiện** có `results` nằm ở `chain[1..]`, nhưng chúng thuộc **`15` chuỗi AND** (không phải `29` chuỗi).
4. **`mark_dont_on_step1` (`eve_runner.rs:339`)**:
   - Khớp về logic & 7 quest (`10012, 11443, 11863, 12325, 12358, 12377, 12406` trong `Mark.dat pos > 0`).
   - **Làm rõ quan trọng**: cờ `p_style = 1` ở đây là `EveResult::parameter_style == 1` (`planner.rs:301-315`), và biến `step` ở [`eve_runner.rs:339`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L339) là **bước cộng thêm** (`(result_value & 0xFF) as u8`), **không phải `next_step`**. Vì vậy mọi lệnh `rt=0, rc=2, p_style=1, val=1` ở cả các bước sau đều set `quest_dont = 1` (và `eve_runner.rs` cũng không bao giờ clear `quest_dont = 0` khi `Drop`/`Decrement`). Ngoài ra số quest đa bước theo tiêu chí `p_style=1, val=1` + `cc=2 val>=2` trên `Data/eve.emg` là `767–769` (báo cáo ghi `758`).

### 2.4. [P1] Hot-path, `run_sync_future` & Lock Ordering
1. **Hot-path `movement.rs` + `npc_event.rs`**:
   - **Khớp phần cấp phát**: mỗi bước đi `0x06 01` / `0x05 01` gọi `resolve_npc_event` (`movement.rs:77-82`) -> `snapshot_state_with_data` (`npc_event.rs:228`), lặp qua toàn bộ **`2,394`** mục `data.quest_marks` (`npc_event.rs:144-151`) **TRƯỚC** khi `activate` (`npc_event.rs:251-252`) kiểm tra `scene.encounters` (trong khi chỉ có đúng **`20` scene** có non-battle `WalkStep` encounter).
   - **Mâu thuẫn thứ tự thực thi tại `EVALUATION.md:139`**: tài liệu viết *"nếu không có encounter trúng vùng, ngay sau đó `try_trigger_random_encounter` (`movement.rs:248-249`) lại gọi `snapshot_state_with_data` thêm một lần nữa khi trúng quái"*. Thực tế [`handle_move_dispatch`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L17-L92) gọi `handle_move(ctx)` ở **L18**, và `handle_move` gọi `try_trigger_random_encounter` ở **L175** — tức `try_trigger_random_encounter` chạy **TRƯỚC** cả `MeetDoor` (`L41`) lẫn `WalkStep` (`L77`). Nếu trúng trận (`battle_id > 0`), `L21-24` thoát sớm và `WalkStep` không chạy; trường hợp gọi 2 lần chỉ xảy ra khi `try_trigger_random_encounter` gọi `snapshot_state_with_data` nhưng fail tìm `chosen_fight_id`/`fight_data` rồi rơi tiếp xuống `WalkStep`.
2. **`run_sync_future` (`eve_runner.rs:1025-1041`)**:
   - Khớp hoàn toàn về cơ chế spawn OS thread + current-thread Tokio runtime (chú ý line chính xác là `1025-1041`, còn `1044-1046` là `execute_event_step`). Hiện tại **có 0 caller production trong `src/`** (chỉ gọi trong các integration test ở `tests/`).
3. **Lock Ordering & Registry Sync (`event_npcs.rs` & `battle.rs`)**:
   - Khớp hoàn toàn: [`event_npcs.rs:152, 366, 444`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/event_npcs.rs#L148-L175) (`15312`, `15313`, `61210`) và [`battle.rs:429`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L429) khóa thẳng `online_sessions().lock()` ghi đè state party member mà không giữ `lock_player_operations`, và `event_npcs.rs` chỉ truyền `&[&conn.session]` vào `persist_sessions_transaction`.
   - **Phát hiện thêm**: cùng pattern khóa `online_sessions` không qua `lock_player_operations` còn có tại [`movement.rs:135-143, 161-169`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L135-L169) và [`battle/construction.rs:164-172`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs#L164-L172) (40-NPC `advance_wave`).

### 2.5. [P2] Tối ưu runtime & Tài liệu / DB
- **Re-plan trong `execute_event_step_core` ([`eve_runner.rs:900-916`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L900-L916))**: Chính xác — mỗi bước lại `.to_vec()` phần `&ev.results[ev.current_index..]` và gọi `plan_results(&rem)`.
- **Ép kiểu `Vec<u8>` <-> `Vec<u16>` ([`eve.rs:310-330`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L310-L330), [`npc_event.rs:290-307, 323`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L290-L323))**: Chính xác.
- **Các điểm vênh của `HANDOFF_01_10.md` (Mục 1.1.5)**: Chính xác toàn bộ (L1 ghi `01 through 08` vs L181 `01 through 10`; L118 ghi `Warps.dat`; L102 ghi `UNKNOWN`; L86-157 ghi `talk.rs`; L182-183 tự xác nhận thiếu live check & migration `0002`).
- **Trạng thái `DB/ts_dream.db`**: Kiểm tra thực tế cho thấy `DB/ts_dream.db` chỉ nặng **`4,096` bytes** với **`0` bảng** (`sqlite_master` rỗng) — tức chưa chạy cả `0001_init.sql` lẫn `0002_eve_quest_persistence.sql` (chỉ `DB/ts_dream_test.db` có đủ 24 bảng).

---

## 3. Các điểm sai lệch, lỗi thời & mâu thuẫn ở Khoảng trống 2, 3, 4

Đây là phần tập trung nhiều điểm chưa chính xác nhất trong `EVE_QUEST_100_PERCENT_EVALUATION.md` (do kế thừa lại danh sách cũ từ các tài liệu research trước đó hoặc diễn đạt nhầm tên trường):

1. **`rc=5` (`430`x) ĐÃ được cài đặt (`EVALUATION.md:76, 160` bị lỗi thời)**:
   - Tài liệu liệt kê `rc=5 (430x)` vào nhóm chưa có ngữ nghĩa trong `planner.rs` (chép từ `RESEARCH_AND_GAP_ANALYSIS.md:318`).
   - Thực tế tại [`src/eve/planner.rs:203-206, 261-299`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L203-L206) và [`eve_runner.rs:403-445`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L403-L445), `rt=0, rc=5` (`MoneyOp`) đã được map đầy đủ cho `p_style ∈ 1..=4` (StatPoints / Gold / ShopPoints).
2. **`cc=7, param=2, p_style=4` (`58`x) KHÔNG hề fail-closed (`EVALUATION.md:66`)**:
   - Tài liệu xếp `param=2, p_style=4` vào nhóm "bị từ chối".
   - Thực tế tại [`src/eve/evaluator.rs:127`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L127), nhánh `2 => compare_step(state.reborn_count, ...)` **không kiểm tra `p_style`**! Do đó `58` điều kiện này không bị trả `false` mà đang chạy thẳng vào so sánh `reborn_count` (tiềm ẩn lỗi logic âm thầm nếu `p_style=4` mang nghĩa khác).
3. **`0x46 BoatRace` và `0x41 Rank` KHÔNG đi qua `unimplemented.rs` (`EVALUATION.md:92`)**:
   - Tại [`src/server/dispatcher.rs:392`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L392), `OP_BOAT_RACE (0x46)` route tới `guild::handle_boat_race` (`src/server/handlers/guild.rs:520-568`).
   - Tại [`dispatcher.rs:371`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L371), `OP_RANK (0x41)` route tới `system::handle_rank` (`src/server/handlers/system.rs:74-82`, dù đúng là stub). Chỉ `0x45, 0x49, 0x4D, 0x4E` đi qua `unimplemented.rs`.
4. **Nhầm lẫn tên struct/trường & phạm vi reader ở Khoảng trống 2 & 3**:
   - **`NpcEventData.to_result` (`EVALUATION.md:70`)**: thực tế `pub to_result: u8` nằm trên struct **`EveCondition`** ([`eve.rs:150`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L150)), không phải `NpcEventData` (nhưng đúng là có 0 reader ngoài `eve.rs`).
   - **`DoorPlacement` / `NpcPlacement` (`EVALUATION.md:86`)**: [`EveDoorPlacement`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L186-L191) chỉ có `{ id, events, x, y }` và cả 4 đều được đọc (không có `close, style, start_x/y, end_x/y`); `close` nằm ở `EveNpcPlacement:172`, còn `can_cut` nằm ở `EveSentence:261`.
   - **`left_enemies` / `right_enemies` (`EVALUATION.md:84`)**: hai danh sách này **có được đọc** tại [`battle/service.rs:1350, 1359`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1350-L1359) để đặt NPC lên grid trận; phần có 0 reader chỉ là `EveFightEnemy.ai`, `EveFightData.link_to_result`, `EveFightData.bk_music` và `Battle.fight_type`.
   - **`0x14 0x2A` & `loader.rs:54-75` (`EVALUATION.md:92`)**: `0x14 0x2A 01` (`SUB_EXP_TOAST`) đã cài đặt ở `npc_talk.rs:22`; trong `loader.rs:54-75`, `compounds` và `festivals` đã được nạp (`L359-360`), không để rỗng như `warp_defs`, `formula_params`, v.v.
5. **Đính chính 3 con số census trong `Data/eve.emg`**:
   - **`rt=9` (`1,599`x, `EVALUATION.md:74`)**: tổng là `1,600` (`100% rc=3`); con số `1,599` là **`p_style ∈ {1, 2}`**, không phải `param=1/2` (`param=1/2` chỉ có `446` bản ghi).
   - **`cc=12 param != 0` (`EVALUATION.md:69`)**: `250` điều kiện nằm trong **`223`** scene event `(scene_id, eve_no)` duy nhất (không phải `249`).
   - **Tỷ lệ `85.2%` (`20,168 / 23,659`) vs `87.3%` (`20,651 / 23,659`)**: nếu chỉ xét `planner.rs` thuần (`Unsupported`), độ phủ event là `87.3%` (`917` event lỗi result); con số **`85.2%`** (`1,495` event lỗi result) của tài liệu là chính xác khi tính theo **hành vi thực thi đầy đủ** (gộp thêm `rt=9` no-op, `GetArmy` no-op, và `rt=0 rc=3 p_style ∈ {0,6}`). Với quest trong `Mark.dat`, script kiểm chứng đếm được `1,095 / 2,124` (`51.6%`), chênh rất nhỏ so với `1,096 / 2,147` (`51.0%`) tùy cách gom tham chiếu quest.
