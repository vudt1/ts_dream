# Đánh Giá Kiến Trúc & Độ Phủ Quest/Mission (`Data/eve.emg` + `ts_dream` Rust Server)

**Góc nhìn**: Lead Game Engine Protocol  
**Phạm vi đối chiếu**:
- Tài liệu bàn giao Tickets `01..10`: [`HANDOFF_01_10.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/HANDOFF_01_10.md), [`spec.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/spec.md), [`ADR 0009`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md)
- Tài liệu dịch ngược client PC: [`EVE_BINARY_ANALYSIS_ALOGIN.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md), [`RESEARCH_AND_GAP_ANALYSIS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/RESEARCH_AND_GAP_ANALYSIS.md), [`UNKNOWN_EVE_CODES_RESEARCH.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md), [`005efee8_FUN_005efee8.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005efee8_FUN_005efee8.c)
- Mã nguồn Rust & dữ liệu gốc: [`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs), `src/eve/*`, `src/server/handlers/*`, `src/battle/*`, `src/data/loader.rs`, `Data/eve.emg`, `Data/Mark.dat`, `Data/Warps.txt`

---

## Executive Summary (Kết luận ngắn)

| Câu hỏi đánh giá | Phán quyết kỹ thuật | Mức độ / Con số kiểm chứng |
|---|---|---|
| **1. Kết hợp `ts_dream` server + [`src/data/loaders/eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs) đã đảm bảo hỗ trợ 100% quest/mission của client `aLogin.exe` chưa?** | **CHƯA ĐẠT 100%** (tài liệu bàn giao chỉ giải quyết phần *hạ tầng cốt lõi & persistence* của `eve.emg`, chưa giải quyết hết *ngữ nghĩa dữ liệu & biên thực thi*). | Ở mức độ event tĩnh trong `Data/eve.emg`, chỉ **`85.2%`** event (`20,168 / 23,659`) dùng toàn bộ condition/result đã hỗ trợ; xét theo **chuỗi quest hoàn chỉnh (`Mark.dat` xuất hiện trong `eve.emg`)**, chỉ **`51.0%`** (`1,096 / 2,147` quest) không đụng tới mã condition/result chưa hỗ trợ. Ngoài ra còn hổng **`47.0%` lệnh warp (`rt=2`)**, cơ chế battle win-condition/AI, map goods, các lỗi logic kích hoạt đa giai đoạn, và các hệ thống nhiệm vụ/hoạt động ngoài `eve.emg`. |
| **2. Thuật toán và kiến trúc hiện tại của module quest/mission trên server Rust đã tốt nhất chưa?** | **Kiến trúc vĩ mô TỐT, nhưng thuật toán vi mô & hot-path CHƯA TỐI ƯU (còn lỗi P0 nghiêm trọng).** | Mô hình `Pure Planner -> Pre-Flight Probe -> EventRunner -> Two-Phase Lock -> Single-Tx SQLite` rất chuẩn cho game MMO. Tuy nhiên còn bug **[P0]** ở [`should_skip_event`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L288-L310) làm gãy **`2,122` event đa giai đoạn (`1,522` quest)** và **`169` NPC có nhiều quest độc lập**; cộng nhiều lỗi **[P1]** (`rt=2` đọc `Warps.txt` thay vì `SceneInfo`, `GroupData.use_mode`, non-head `chain[1..].results`, `mark_dont_on_step1`, clone `2,394` mục `quest_marks` mỗi bước đi `0x06 01`, và `run_sync_future`). |

---

## Phần 1 — Đánh giá khả năng hỗ trợ 100% Quest / Mission của `aLogin.exe`

### 1.1. Những gì [`HANDOFF_01_10.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/HANDOFF_01_10.md) và [`eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs) đã làm đúng & hoàn thiện

Bản đối chiếu binary `aLogin.exe` (`4,373,504` bytes, SHA-256 `1487059f0508d49e89946856eee4088b0cf53704efafc2d8d57988d148ff06f4`) khẳng định chiến lược **100% `Data/eve.emg` làm nguồn sự thật trên server** là hướng đi chính xác tuyệt đối:
- Theo [`EVE_BINARY_ANALYSIS_ALOGIN.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md#L23-L35), client `aLogin.exe` tuy có parser `FUN_005efee8` đọc cả 9 section của `.eve`, nhưng `SceneInfo`, `GroupData`, `TGoods`, `TEvent/TCondition/TResult` hoàn toàn không có reader runtime trên client. Client chỉ nhận và thực thi lệnh `S->C 0x14 Sub 0x01..0x06` dài 15B qua `FUN_005eb530` (`rt ∈ 0..=6`).
- Tickets `01..10` đã xử lý đúng 5 trụ cột hạ tầng khó nhất:
  1. **Sửa stride Section 9 (`TFightData`) trong [`eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L965-L977)**: chuyển bước nhảy `win_cond_count * 13` bytes, khôi phục toàn bộ `3,897` scene hợp lệ (trên tổng số `3,901` mục lục trong `Data/eve.emg`, `9,839,580` bytes; `62007.eve` kết thúc hợp lệ tại byte `9,815,664` với 64 byte dư).
  2. **Tách rạch ròi Container `0x17` (`homdo`) và Quest Ledger/Bitmask `0x18` (`Mark.dat`)**: loại bỏ gửi nhầm item quest qua `0x18`, chuẩn hóa `0x18 05` / `0x18 07` theo bảng bit `1..=2400` (`Data/Mark.dat`, `2,394` bản ghi với `1,279` có `pos > 0` và `1,115` có `pos == 0`) và ledger `1..=200` (`0x18 06`), chặn `C->S 0x18` (`src/server/handlers/quest_sync.rs`), phát `0x18 03` khi đầy ledger.
  3. **Atomic Persistence & Two-Phase Locking**: PK `(playerid, scene_id, eventid)` cho `character_completed_events`, đồng bộ `save_eve_state` vào single-transaction SQLite, khóa `lock_player_operations([min..max])` khi party warp, xử lý `offline_eve_sessions` khi rớt mạng giữa trận.
  4. **Pure Planner + Pre-Flight Probe All-or-Nothing**: tách domain decision khỏi mutation trong [`src/eve/planner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs) và [`preflight_probe_batch`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs) để ngăn lỗi nửa vời mất đồ/mất quest.
  5. **Ghi nhận điểm vênh trong tài liệu bàn giao**: [`HANDOFF_01_10.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/HANDOFF_01_10.md) vẫn còn vài thông tin cũ cần cập nhật — tiêu đề L1 ghi `Tickets 01 through 08` (dù L181 chốt `01 through 10`), L118 ghi nhầm `Warps.dat` trong khi code thực tế dùng `Warps.txt` (`src/data/loader.rs:370-376`), L102 ghi `UNKNOWN` thay vì `DomainEffect::Unsupported`, L86-157 ghi `talk.rs` cho các hàm đã chuyển về `eve_runner.rs`, và mục L182-183 tự xác nhận còn thiếu test thực tế trên `aLogin.exe` (`0x14 01` MutateItem, `0x1A 05/06`), test multi-wave `quest_tasks[218]`, và chạy migration `0002` lên `DB/ts_dream.db`.

---

### 1.2. Năm khoảng trống khiến server CHƯA đạt 100% Quest / Mission

#### Khoảng trống 1 — Lỗi thiết kế Warp `rt=2` & `diahinh` do [`eve.rs:738`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L738) bỏ đọc `eveFileName` và phụ thuộc `Data/Warps.txt`

- **Chứng cứ từ `Data/eve.emg` và pseudo-C**:
  - Toàn bộ `Data/eve.emg` chứa **`10,336` lệnh warp (`result_type == 2`)**, và **`10,336 / 10,336` (`100%`)** tham chiếu khớp chính xác theo `(scene_id, r.parameter == SceneInfo.eve_no)` tới bảng `EveSceneInfo` (`8,882` bản ghi trong Section 6).
  - Tuy nhiên tại [`src/data/loaders/eve.rs:738`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L738), parser thực hiện:
    ```rust
    let eve_no = reader.read_u16();
    let background_no = reader.read_u16();
    reader.skip(2); // eveFileName  <-- VỨT BỎ DEST MAP ID!
    ```
    Đối chiếu trực tiếp [`client_pseudo_c/005efee8_FUN_005efee8.c:537-558`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005efee8_FUN_005efee8.c#L537-L558), 2 byte tại offset `+0x02` của mỗi `SceneInfo` (tuy chú thích tên trường là `eveFileName`) thực chất là một giá trị số `u16` (ID bản đồ/scene đích trong hệ thống scene `.eve`), đi cùng `player_appear_x`, `player_appear_y` (`i32`) và `direction` (`u8`). Dù `EveSceneInfo` đã giữ `player_appear_x/y`, cả hai trường này có **0 chỗ đọc** trong toàn bộ `src/`.
- **Hệ quả ở Runtime**:
  - Do thiếu `dest_map_id` trong `EveSceneInfo`, toàn bộ đường warp của Eve và battle ([`eve_runner.rs:926-937`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L926-L937), [`quest.rs:928-936, 981-991, 1169-1187`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L928-L936), [`battle/service.rs:553, 571`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L553-L578)) bị buộc tra bảng legacy **`data.warps` nạp từ [`Data/Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L370-L376)** (`4,978` dòng) — trong khi nhiều log/comment vẫn gọi nhầm là `"Warps.dat"`.
  - **Thống kê gãy thực tế**:
    - **`4,857 / 10,336` (`47.0%`) lệnh warp `rt=2` KHÔNG TỒN TẠI** trong `Data/Warps.txt` (chỉ có `5,479` trường hợp tìm thấy khóa `(scene_id, r.parameter)`), khiến gần một nửa số cảnh chuyển map qua sự kiện/quest bị bỏ qua lặng lẽ.
    - Ngoài ra, khóa tra cứu của `Warps.txt` theo `door_id` đụng độ ngữ nghĩa với `SceneInfo.eve_no`: trong `4,924` mục so sánh được giữa `Warps.txt` và `SceneInfo` cùng khóa số, chỉ `4,247` trùng map đích và `4,067` trùng đúng tọa độ `(x, y)` (ví dụ ở `11000.eve`, door `2` trỏ `eve 3` phát `rt=2 param=3` dẫn về `SceneInfo[3]`, trong khi `Warps.txt` tại `(11000, 3)` lại trỏ tới map `11571 (650, 370)`).
- **Nhầm lẫn `diahinh` (Battle Terrain)**:
  - [`eve_runner.rs:731-735`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L731-L735) và [`movement.rs:235-239, 288-293`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L235-L239) cùng hardcode `scene.scene_infos.get(&1).background_no` làm `diahinh` cho trận đánh — trong khi `background_no` của `SceneInfo` thực chất là thuộc tính nền/map của một điểm warp `eve_no = 1`, không phải ID địa hình chiến đấu chuyên biệt.

#### Khoảng trống 2 — Tỉ lệ hỗ trợ `EveCondition` & `EveResult` mới đạt `85.2%` Event / `51.0%` Quest

Do kiến trúc hiện tại **fail-closed** (`false` / `Unsupported` + bỏ qua) với mọi mã chưa dịch ngược hoàn chỉnh ([`evaluator.rs:19-361`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L19-L361), [`planner.rs:136, 223-232`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs#L223-L232), [`eve_runner.rs:787-797`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L787-L797)), server an toàn không crash nhưng **rất nhiều điều kiện rẽ nhánh và phần thưởng/hiệu ứng nhiệm vụ không thể kích hoạt**:

- **Độ phủ định lượng trên toàn bộ `Data/eve.emg`**:
  - Event dùng 100% condition & result đã hỗ trợ: **`20,168 / 23,659` (`85.2%`)** (`2,380` event dính condition chưa map; `1,495` event dính result chưa map).
  - Quest (`Mark.dat`) xuất hiện trong `eve.emg`: **`1,096 / 2,147` (`51.0%`)** quest sạch hoàn toàn; **`1,051` quest (`49.0%`)** có ít nhất một event chạm condition hoặc result chưa hỗ trợ.

- **Các nhóm `condition_class` bị fail-closed hoặc cắt cụt trong [`evaluator.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs)**:
  1. **8 `condition_class` hoàn toàn chưa cài đặt (`1,282` bản ghi)**: `cc=3` (`807`x), `cc=5` (`181`x), `cc=13` (`110`x), `cc=11` (`77`x), `cc=4` (`45`x), `cc=16` (`42`x), `cc=17` (`16`x), `cc=15` (`4`x), cộng `1` trường hợp `condition_ops=142` (`55100.eve, eve=5, cond_no=14`).
  2. **`cc=7` (Kiểm tra trạng thái nhân vật) thiếu nhiều `parameter`**: `evaluator.rs:124-210` chỉ hỗ trợ `param ∈ {0 (level), 2 (reborn), 3 (equip), 5 (pet slot p_style 1|2), 6 (pet ownership p_style 1..=3)}`. Bị từ chối gồm: `param=8` (`673`x), `param=16` (`178`x), `param=10` (`176`x), `param=1` (`111`x), `param=12` (`97`x), `param=7` (`61`x = `32 + 29`), và `param=2, p_style=4` (`58`x).
  3. **`cc=2` (Quest check) thiếu `p_style >= 4` (`948` bản ghi)**: `evaluator.rs:86-108` chỉ hỗ trợ `p_style ∈ 0..=3`, trả về `false` với `p_style=10` (`388`x), `9` (`320`x), `4` (`125`x), `5` (`109`x), `6` (`4`x), `7` (`2`x).
  4. **`cc=9` (NPC đồng hành / Follower) thiếu `p_style 2, 3`**: `evaluator.rs:249-268` chỉ kiểm tra `follow_npc_ids` khi `p_style ∈ {0, 1}`; bị loại gồm `p_style=2` (`259`x) và `p_style=3` (`84`x).
  5. **`cc=12` (Kiểm tra event đã làm) từ chối `param != 0` (`250` điều kiện trong `249` scene event)**: `evaluator.rs:298-319` khớp cứng `(condition_parameter == 0, p_style ∈ {0, 1})`. Mọi `cc=12` kiểm tra chéo sang event khác hoặc `p_style >= 2` đều fail-closed.
  6. **Dữ liệu đã snapshot nhưng bỏ không**: `PlayerEventState.bit_flags` ([`src/eve/state.rs:52`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/state.rs#L52)) được nạp từ `Session.bit_flags` tại `npc_event.rs:138, 187`, nhưng **không có bất kỳ dòng code nào trong `src/eve/` đọc `bit_flags`**. Tương tự `EveCondition.condition_sub_item` và `NpcEventData.to_result` được parser đọc nhưng có **0 reader** ngoài `eve.rs`.

- **Các nhóm `EveResult` chưa hỗ trợ hoặc là no-op**:
  1. **`rt=7` (`691`x), `rt=8` (`351`x), `rt=10` (`98`x)**: 100% server-only (`aLogin.exe` không xử lý), hiện map vào `DomainEffect::Unsupported` (`planner.rs:223-232`) và bị bỏ qua.
  2. **`rt=9` (`NpcAction`, riêng cặp `rc=3 param=1/2` đã có `1,599` bản ghi)**: tại [`eve_runner.rs:776-786`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L776-L786) chỉ ghi log debug rồi cho qua (no-op thực sự, không cập nhật trạng thái NPC hay phát frame).
  3. **`rt=0, rc=7, param=2` (`PlayerOp::GetArmy`, `87`x theo census)**: tại [`eve_runner.rs:506-512`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L506-L512) ghi `"Eve army result skipped: no army store wired"` — chưa nối kho lính/thủy binh.
  4. **Các `result_class` chưa map dưới `rt=0` hoặc khác**: `RESEARCH_AND_GAP_ANALYSIS.md:318` ghi nhận `rc ∈ {0, 5 (430x), 6 (9x), 9 (1x), 10 (9x), 11 (35x), 12 (21x), 13 (1x), 15..22, 80 (1x)}` chưa có ngữ nghĩa trong `planner.rs`. Ngoài ra `rt=4` (`210/210` bản ghi có `rc == 3`) cần xác nhận chuẩn đóng gói tọa độ `val XY` theo `FUN_007127bc`.

#### Khoảng trống 3 — Dữ liệu Gameplay quan trọng bị [`eve.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs) skip hoặc parse xong để không

Kiểm tra toàn bộ trường `pub` của các struct `Eve*` trong `eve.rs` trên toàn codebase `src/` cho thấy các mảng dữ liệu sau hoàn toàn chưa tham gia gameplay:

1. **Hệ thống trận đánh quest/boss (`Section 9: TFightData`)**:
   - [`eve.rs:965-977`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L965-L977) skip toàn bộ **`45,383` `TFightEvent`** và **`45,773` `TWinCondition`** (chủ yếu `wc_type=1`, nhưng có **`424` quy tắc thắng/thua đặc biệt**: `wc_type=8` `277`x, `4` `98`x, `3` `44`x, `9` `4`x, `20` `1`x). Trong `aLogin.exe`, phím tắt `[alt+W]` đọc trực tiếp `wc_type ∈ {3, 4, 8}` để hiển thị điều kiện thắng trận cho người chơi!
   - Các trường đã parse gồm `EveFightEnemy.ai`, `EveFightData.link_to_result`, `EveFightData.bk_music` đều có **0 reader** toàn `src/` ([`BattleService::start_encounter_battle`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L1233-L1382) chỉ đọc `fight_type` gán vào field cũng chưa có chỗ đọc trong `src/battle/`, cùng danh sách `left_enemies`/`right_enemies`). Điều này đồng nghĩa boss AI theo kịch bản `.eve`, chi viện theo turn (`TFightEvent`), và điều kiện thắng/thua đặc biệt (sống sót N turn, bảo vệ NPC, bắt tướng...) chưa thể hoạt động.
2. **`GoodsData` (`1,174` vật phẩm nhặt trên bản đồ tại `619` scene)**: [`skip_goods_section`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L545-L554) bỏ qua hoàn toàn `13B * count`. Mọi nhiệm vụ yêu cầu nhặt/tương tác vật phẩm đặt sẵn từ `TGoods` của `.eve` (nếu không qua `ItemOnMap`) không thể spawn từ `eve.emg`.
3. **Liên kết Shop NPC (`sale_kind`) & thuộc tính đặt NPC/Cửa**: [`parse_npc_data_section`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L476-L512) skip `sale_kind`, `roleGrid/moveOffsetGrid`, `isHide/isVisible`, `rideNpcId`, `isLie`, `innerNode/outerNode`; còn `DoorPlacement` (`close`, `style`, `start_x/y`, `end_x/y`), `NpcPlacement` (`can_cut`, `can_grow`, `trace_radius`, `motion_*`) đều chưa được bất kỳ hệ thống nào đọc.

#### Khoảng trống 4 — Các hệ thống Nhiệm vụ / Sự kiện & Tài nguyên nằm ngoài `eve.emg`

Một client đầy đủ như `aLogin.exe` không chỉ có quest từ `eve.emg`:
- **Sự kiện đặc biệt hardcode ngoài `eve.emg`** ([`src/server/handlers/event_npcs.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/event_npcs.rs#L21-L25) chạy trước Eve tại [`talk.rs:175-187`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L175-L187)): `15162` (Liên hoàn 40 NPC, `quest 218` cập nhật qua `src/battle/construction.rs`), `15312` (Giải cứu Nhị Kiều, `quest 212`), `61209` (Cửu Thiên, `quest 12309`/`12310`), `20001` (Rương tầng tháp), cùng `Bank/Inn/DailyQuest (map 12711)` hardcode trong `talk.rs`.
- **Hoạt động & sub-opcode chưa nối**: `0x14 0x2A 02/03` (đăng ký/hủy đại tỷ thí), `0x14 0x32`, `0x14 0x33`, `0x14 0x38` chưa có ở bất kỳ file nào trong `src/`; các catalog như `Data/Scene.Dat` (`1,002,454 B`) chưa có loader, còn `achievement_defs`, `dispatch_defs`, `scene_set_defs`, `warp_defs`, `city_ex`, `formula_params` vẫn để rỗng/deferred trong [`GameData::load_binary`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L54-L75); các opcode hoạt động lớn khác đi qua [`unimplemented.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/unimplemented.rs) (như `0x41 Rank` stub, `0x45 Child`, `0x46 BoatRace`, `0x49 WorldBoss`, `0x4D Activity`, `0x4E Astrolabe`...).

---

## Phần 2 — Đánh giá Thuật toán & Kiến trúc Runtime của Module Quest trên Server Rust

### 2.1. Điểm mạnh ở tầng Kiến trúc vĩ mô

Nếu nhìn từ chuẩn thiết kế Game Server Engine:
- **Pure Functional Planner (`src/eve/{evaluator,resolver,planner,group}.rs`)**: tách biệt hoàn toàn lớp ra quyết định thuần túy (`PlayerEventState` bất biến -> `Vec<DomainEffect>`) khỏi lớp I/O (`eve_runner.rs`). Đây là nền tảng rất tốt cho deterministic replay test và chống side-effect rò rỉ.
- **Pre-Flight All-or-Nothing Probe ([`preflight_probe_batch`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs))**: mô phỏng trước sức chứa `homdo` (`25` ô), trần vàng, giới hạn `4` slot pet và `200` ô Quest Ledger trước khi chạm state thật — triệt tiêu bug kinh tế kinh điển "mất nguyên liệu/hoàn thành quest mà không nhận được thưởng".
- **State Machine Resume qua trận (`EventSession` + `AwaitingBattle` + `offline_eve_sessions`)**: bảo toàn chuỗi đối thoại/thưởng liên hoàn trước và sau battle, kể cả khi người chơi mất kết nối giữa trận.
- **Two-Phase Lock Ordering + Single-Tx Write-Through**: ngăn deadlock giữa `lock_online_sessions()` và SQLite single-writer pool, đảm bảo durability ở các checkpoint quan trọng.

Tuy nhiên, **khi đi sâu vào thuật toán điều phối và vòng lặp thực thi chi tiết, server hiện có một lỗi P0 nghiêm trọng và nhiều điểm chưa tối ưu (P1/P2)**.

---

### 2.2. Các lỗ hổng Thuật toán & Hiệu năng cần xử lý ngay

#### [P0 — Nghiêm trọng] [`EveAutoChainEngine::should_skip_event`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L288-L310) phá vỡ chuỗi quest đa giai đoạn và NPC nhiều quest

Đây là lỗi thuật toán lớn nhất làm hàng nghìn quest có đủ condition/result hợp lệ vẫn **không thể chạy tiếp** trong game thực tế:
- Tại [`src/server/handlers/npc_event.rs:257`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L257) (`WalkStep`) và [`npc_event.rs:326`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L326) (`activate` cho `ClickNpc` / `ClickDoor`), server gọi **trực tiếp** `EveAutoChainEngine::should_skip_event(events, eve_no, ev_data, &state)` ngay ở **lần kích hoạt đầu tiên** (khác với [`auto_chain.rs:170-175`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/auto_chain.rs#L170-L175) có ngoại lệ cho `chain_filter`):
  1. **Rule 1 (`auto_chain.rs:303-305`)**: `if completed_count > 0 && !has_completion_condition { return true; }` (trong đó `has_completion_condition` chỉ kiểm tra có `condition_class == 12`).
     - **Sai giả định domain**: Trong `Data/eve.emg`, đại đa số chuỗi nhiệm vụ nhiều bước quay lại nói chuyện cùng một `eve_no` để phân nhánh theo trạng thái quest (`cc=2`) **KHÔNG gắn `cc=12`**, mà dùng chính `cc=2` (`step == 0`, `step == 1`, `step == 2`...) và cơ chế so điểm 3 tiêu chí của `resolver::resolve` để chọn nhánh kế tiếp! Vì mỗi `eve_no` bị đánh dấu hoàn thành ngay sau lần tương tác đầu (`step 1` nhận quest), lần quay lại giao tiếp tiếp theo gặp `completed_count > 0` + `!has_completion_condition` -> **bị skip vĩnh viễn**.
     - **Quy mô ảnh hưởng thực tế**: kiểm đếm toàn bộ `Data/eve.emg` cho thấy **`2,122` event đa giai đoạn (`>= 2` bước thay đổi state quest trong cùng `eve_no` mà không có `cc=12`)** thuộc **`1,522` quest** bị Rule 1 chặn ngay từ bước thứ hai!
  2. **Rule 2 (`auto_chain.rs:306-308`)**: `if completed_count == 0 && has_later_completed_event(events, eve_no, state) { return true; }`.
     - **Sai giả định domain**: Giả định mảng `events` gắn trên 1 NPC luôn là chuỗi tuyến tính 1 chiều (làm xong `events[j]` nghĩa là toàn bộ `events[i]` đứng trước với `i < j` đã hết hiệu lực). Thực tế trong `eve.emg`, **có `430` NPC gắn nhiều event**, trong đó **`169` NPC chứa các quest hoàn toàn độc lập nhau** (chẳng hạn nhận và trả quest ở cuối danh sách trước sẽ khóa vĩnh viễn toàn bộ các quest khác đứng đầu danh sách trên cùng NPC đó!).

#### [P1 — Đúng đắn ngữ nghĩa Engine]

1. **Bỏ qua `when_happen` (`[on_click_obj, on_stroke_obj, on_into_area, on_ser_stroke]`)**: `parse_npc_event_section` đã giải mã đủ 4 cờ kích hoạt nhưng không có bất kỳ nơi nào kiểm tra; hệ quả là sự kiện chỉ dành cho giẫm vùng (`on_into_area`) hoặc chạm va chạm (`on_stroke_obj`) có thể bị trộn lẫn nếu gắn chung danh sách hoặc không phân loại chính xác trigger.
2. **Bỏ qua `GroupData.use_mode` trong [`group.rs:71-100`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/group.rs#L71-L100) & nghi vấn khóa `group_datas`**:
   - `select_member` kiểm tra `pick_member > 0`, rồi độ dài mảng, rồi `total_weight <= 0`, **hoàn toàn không kiểm tra `use_mode == 2`** (chế độ bỏ qua chọn ngẫu nhiên / giữ toàn bộ kết quả — ảnh hưởng ít nhất `9` nhóm có `use_mode == 2` kèm `pick_member > 0` hoặc `total_weight > 0`).
   - Đồng thời [`parse_group_section` (`eve.rs:798`)](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L798) lưu `scene.group_datas` theo khóa `eve_no` (trong khi struct còn có trường `event_no` tại L802), còn [`apply_group_data` (`group.rs:39`)](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/group.rs#L39) tra trực tiếp `group_datas.get(&result.result_group_no)` — cần test đối chiếu phủ khóa trên mọi scene có `result_group_no > 0`.
3. **[`resolver::resolve`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/resolver.rs#L80-L133) bỏ rơi `results` ở các node sau đầu chuỗi AND**:
   - L88 kiểm tra `!head.results.is_empty()` và L129 chỉ trả về `winner.chain[0].results.clone()`. Trên `Data/eve.emg` tồn tại **`29` chuỗi AND** có `results` gắn ở các node `chain[1..]` bị rớt mất hoàn toàn. (Ghi chú thêm: doc comment L141 ghi `"4-tier comparison"` nhưng `is_better_than` tại L143-151 chỉ so 3 tiêu chí `step_score`, `chain.len()`, `chain[0].results.len()`, hòa thì giữ ứng viên đầu).
4. **Đánh dấu sớm `quest_dont` ở [`eve_runner.rs:339`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L339)**:
   - Với `p_style = 1` (`mark_dont_on_step1 == true`), hễ `step == 1` và trận không thua/chạy (`!matches!(battle_result, 2 | 3)`) là server set `set_quest_dont(..., 1)` luôn. Trên `Data/eve.emg`, phần lớn quest nhiều bước chỉ dùng `Mark.dat pos > 0` rất ít (`758` quest đa bước chỉ có **`7` quest** có `Mark.dat pos > 0`, gồm `10012, 11443, 11863, 12325, 12358, 12377, 12406`), nhưng chính `7` quest đó bị đánh dấu `dont = 1` quá sớm ngay ở bước nhận đầu tiên, khóa điều kiện nhận/làm tiếp theo `quest_dont`.

#### [P1 — Hot-path CPU / Allocation & Async Concurrency Anti-Patterns]

1. **Hot-path di chuyển (`0x06 01` / `0x05 01`) cực tốn kém**:
   - Tại [`src/server/handlers/movement.rs:76-91`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L76-L91), **mọi packet di chuyển 1 bước chân** đều gọi `npc_event::resolve_npc_event(..., WalkStep { x, y }, ...)`.
   - Trong [`npc_event.rs:228`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L228), hàm gọi **`snapshot_state_with_data`** **TRƯỚC** khi `activate` (`L252`) kiểm tra xem tọa độ `(x, y)` có nằm trong `scene.encounters` hay không.
   - Bên trong `snapshot_state_with_data` ([`npc_event.rs:144-151`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L144-L151)), server lặp qua **toàn bộ `2,394` phần tử `data.quest_marks`** để cấp phát và dựng một `HashMap<i32, i32>` mới cho **từng bước đi của từng người chơi**, mặc dù trên toàn bộ `3,897` scene chỉ có **`20` scene** thực sự có non-battle `WalkStep` encounter!
   - Không chỉ vậy, nếu không có encounter trúng vùng, ngay sau đó `try_trigger_random_encounter` (`movement.rs:248-249`) lại gọi `snapshot_state_with_data` thêm một lần nữa khi trúng quái.
2. **Thread/Runtime Anti-Pattern ở `run_sync_future` ([`eve_runner.rs:1025-1046`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L1025-L1046))**:
   - Khi đang ở trong Tokio runtime (`Handle::try_current().is_ok()`), `run_sync_future` dựng `std::thread::scope` để spawn hẳn một OS thread mới và tạo một Tokio `current_thread` runtime mới (`Builder::new_current_thread().enable_all().build()`) chỉ để `block_on` DB transaction! Dù đường async chính (`talk.rs`, `movement.rs`, `battle/service.rs`) dùng `*_with_pool`, các sync wrapper (`execute_event_step`, `resume_eve_after_battle_sync_full`) vẫn tiềm ẩn overhead tạo OS thread + runtime ở bất kỳ chỗ nào gọi nhầm hoặc test/adapter sync.
3. **Vi phạm Lock Ordering & Registry Sync ngoài Eve runner**:
   - Trong [`event_npcs.rs:152, 366, 444`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/event_npcs.rs#L149-L175) (2K `15312`, `15313`, `61210`) và [`battle.rs:429`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L429) (North Star quiz), code khóa thẳng `lock_online_sessions()` để ghi đè `map_id` / `battle_id` / `quest_tasks` của party member mà **không** giữ `lock_player_operations([min..max])` và ở `event_npcs.rs` cũng không persist session các member vào SQLite (`persist_sessions_transaction` chỉ truyền `&[&conn.session]`).

#### [P2 — Tối ưu cấu trúc dữ liệu & giảm cấp phát thừa]

1. **Re-plan lặp lại trong [`execute_event_step_core`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L900-L916)**: mỗi bước đối thoại hoặc action lại clone slice còn lại `&ev.results[ev.current_index..]` thành `Vec<EveResult>` rồi chạy lại `plan_results(&rem)` từ đầu, thay vì plan 1 lần khi mở `EventSession` hoặc đọc theo con trỏ index.
2. **Ép kiểu qua lại `Vec<u8>` -> `Vec<u16>` -> `Vec<u8>`**: [`SceneEveData::build_indices`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loaders/eve.rs#L310-L330) lưu index thành `Vec<u16>`, rồi [`npc_event.rs:290-307`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L290-L307) lại cấp phát `Vec<u8>` tạm cho mỗi lần click NPC/Door/Encounter rồi ép ngược `u16::from(eve_no)` ở `L323`. Nên dùng thẳng `&[u8]` (hoặc nâng `eve_no` thống nhất trong toàn engine).

---

## Phần 3 — Lộ trình Khắc phục Ưu tiên (Dành cho Implementation Tiếp Theo)

| Ưu tiên | Hạng mục kỹ thuật | Tác động trực tiếp |
|---|---|---|
| **P0** | **Gỡ / sửa lại `should_skip_event` (`auto_chain.rs:288-310`, `npc_event.rs:257, 326`)**: bỏ Rule 1 (không chặn lặp `eve_no` thiếu `cc=12`) và Rule 2 (không chặn `eve_no` đứng trước theo vị trí mảng; dùng điều kiện `cc=2`/`cc=12` và độ ưu tiên của `resolver::resolve` để điều hướng). | Mở khóa ngay **`2,122` event đa giai đoạn (`1,522` quest)** và **`169` NPC có nhiều quest độc lập** mà gần như không cần đợi thêm RE binary. |
| **P0** | **Đọc `dest_map_id` (`u16`) trong `eve.rs:738` & chuyển warp `rt=2` sang `EveSceneInfo`**: bỏ phụ thuộc `Warps.txt` cho `DomainEffect::Warp` và post-battle warp (chỉ giữ `Warps.txt` làm fallback nếu `scene_infos` không có hoặc map sang tọa độ `player_appear_x/y`). | Khôi phục **`4,857` cổng/warp quest (`47.0%`)** đang bị skip và loại bỏ lỗi lệch map/tọa độ giữa `DoorID` và `SceneInfo.eve_no`. |
| **P1** | **Tối ưu Hot-path `movement.rs` + `npc_event.rs`**: (a) kiểm tra spatial bounding box `scene.encounters` **trước** khi gọi `snapshot_state_with_data`; (b) bỏ clone `data.quest_marks` (`2,394` phần tử) mỗi lượt bằng việc giữ `Arc<GameData>` / `&HashMap<u16, u16>` trong `PlayerEventState` (tỉ lệ giảm cấp phát ở bước di chuyển ~100%). | Giảm mạnh CPU & heap pressure trên `0x06 01` ở server nhiều CCU. |
| **P1** | **Sửa ngữ nghĩa Engine & Concurrency**: check `use_mode == 2` trước trọng số trong `group.rs`; gom cả `chain[1..].results` trong `resolver.rs`; chỉ set `quest_dont` đúng ngữ nghĩa thay vì tự động ở `step == 1` (`eve_runner.rs:339`); wire `when_happen` theo trigger; sửa `lock_player_operations` cho party members trong `event_npcs.rs` / `battle.rs`. | Loại bỏ kẹt chain ở 29 chuỗi AND, 9 group `use_mode=2`, 7 quest đa bước có `Mark.dat pos > 0`, và race condition party. |
| **P1 / Research** | **Mở rộng độ phủ Quest tới ~100% (`1,051` quest còn thiếu)**: theo thứ tự ROI dịch ngược từ `Data/eve.emg` & `aLogin.exe`: (1) `cc=7` các param cao (`8, 16, 10, 1, 12, 7`), `cc=12 param != 0` (`250`x), `cc=2 ps ∈ {4,5,9,10}` (`948`x), `cc=9 ps ∈ {2,3}` (`343`x), và `cc ∈ {3, 5, 13, 11, 4, 16, 17, 15}` (`1,282`x); (2) `rt ∈ {7, 8, 9 (NpcAction thật), 10}`, `rc=5 (430x)`, `GetArmy`, `GoodsData` (`1,174`x); (3) Nạp `TFightEvent` + `TWinCondition` (`wc_type ∈ {3,4,8,...}`) + `ai` + `link_to_result` vào `BattleService`. | Đưa tỷ lệ hoàn thành chuỗi quest thực tế từ mức trần `~51%` (dù sửa P0) tiến dần lên `95–100%` bao gồm boss quest, nhặt đồ map và chuỗi điều kiện phức tạp. |
| **P2** | **Dọn runtime & tài liệu**: plan `results` 1 lần khi vào `EventSession`; thống nhất `Vec<u8>` cho `SceneEveData` index; sửa doc `HANDOFF_01_10.md` (`01..10`, `Warps.txt`, `DomainEffect::Unsupported`, `eve_runner.rs`), hoàn tất live verification trên `aLogin.exe` (`L182-183`), và áp dụng `migrations/0002` cho `DB/ts_dream.db`. | Tăng tốc thực thi `eve_runner`, sạch codebase và đồng bộ tài liệu vận hành. |
