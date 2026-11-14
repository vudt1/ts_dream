# ADR 0009 — Chuẩn hóa Động cơ Kịch bản Eve, Nhật ký Nhiệm vụ `0x18` (`Mark.dat`) & Bền vững Hóa Dữ liệu Quest

- **Ngày**: 2026-11-13
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn `/grill-with-docs`)

---

## 1. Bối cảnh (Context)

1. Đối chiếu trực tiếp 4 nguồn sự thật gốc — file nhị phân `Data/eve.emg` (`3,901` scene), bản dịch ngược client PC `aLogin.exe` trong `client_pseudo_c/` (`Data/Mark.dat`, `Data/Item.dat`), mã nguồn Rust trong `src/data/loaders/eve.rs`, `src/eve/`, `src/server/handlers/{talk,npc_event,quest,quest_sync}.rs`, `src/battle/service.rs` và lược đồ SQLite trong `migrations/0001_init.sql` (xem chi tiết tại `.scratch/quest-refactor/RESEARCH_AND_GAP_ANALYSIS.md`) — đã phơi bày 5 nhóm khiếm khuyết mang tính hệ thống:
   - **Lỗi giải mã nhị phân `Data/eve.emg`**: `src/data/loaders/eve.rs` đọc sai stride `win_cond` (`12B` thay vì `13B`) và đọc thừa trường `fight_limit: u32` (`0B` trong thực tế), đồng thời nuốt lỗi bằng `if let Ok(...)`, khiến `1,120 / 3,901` scene bị bỏ rơi âm thầm và dữ liệu trận đấu (`FightData`) bị lệch byte.
   - **Tắt mặc định ở production & xung đột dữ liệu mẫu `.ini`**: `EVE_EVENTS_ENABLED` mặc định `false`, `GameData::load()` không nạp `Data_Talks`/`npc_on_map`, trong khi các file `.ini` mẫu (như `12001 NPC 24`, `12136 NPC 1`) chứa dữ liệu rút gọn/sai lệch so với `Data/eve.emg`.
   - **Hiểu sai giao thức `0x18` vs `0x17` (`quest_items` giả định)**: Comment cũ trong `src/server/session.rs` và `src/server/handlers/quest_sync.rs` coi `0x18 01..04` là túi vật phẩm quest 200 ô và coi `quest_dont` (`0x18 05`) chỉ dùng `mark 1..=300`. Bằng chứng từ `aLogin.exe` (`FUN_00790ed5`, `FUN_00720ca8`, `FUN_00721088`, `FUN_0072ba54`, `FUN_005eb530`) kết hợp `Data/Mark.dat` (`2,394` bản ghi, `position` tới `1673`) chứng minh:
     - `0x18 01/02/04/06` (`LocalActor + 0x654`, 200 ô) chỉ tra cứu **`Mark.dat`** (`10001..12461`), **không bao giờ đọc `Item.dat`**.
     - `0x18 05` nhận **`bit_id` (`1..=2400`)** từ `Mark.dat.position` để bật/tắt bit `(bit_id - 1) % 8` trong mảng 300 byte tại `LocalActor + 0x8b2`, còn `0x18 07` gửi `[byte_idx (1..=300): u16][bitmask: u8]`.
     - Mọi vật phẩm `Item.dat` — kể cả đạo cụ nhiệm vụ `result_class = 1` — bắt buộc đi qua **`homdo` (`0x17`, 25 ô)**. Trên `0x14 01`, `rt=0 rc=1` chỉ hiển thị toast từ `Item.dat`.
   - **Lỗ hổng điều kiện/hiệu ứng & trình tự điều khiển**: `evaluator.rs` có lỗi fallthrough `actual = 0 -> 0 == 0 -> true`; `npc_event.rs` bỏ trống `follow_npc_ids`, `inn_npc_ids`, `cart_npc_ids`, `role_count_values`, `mark_defs` và ghép sai không gian khóa `quest_dont` với `quest_id`; `handle_talk_select_menu` thiếu lọc `DialogChoice`; `talk.rs` thiếu executor cho `NpcTeam` (`rt=0/rc=3`, phổ biến nhất với `29,858` action), `RewardPet`, `MoveEntity`, `NpcAction`, `ClickDoor`/`MeetDoor`, phát `14 2C Unlock` trước `AutoChain`, và đánh rơi `direct_messages`/`map_broadcast` trong `resume_eve_after_battle`.
   - **Mất toàn vẹn lưu trữ & Race Condition**: `SqliteSessionRepository::save` tách `save_eve_state` sang transaction thứ 2 và nuốt `Err`; `character_completed_events` thiếu `scene_id` trong khóa chính gây đụng độ `eve_no` liên bản đồ; thiếu `savemap` và `character_bit_flags`; `BattleService::battle_ended` thiếu `lock_player_operations` và thiếu `registry_update_if_online`; và ngắt kết nối giữa trận làm rớt toàn bộ hậu sự kiện Eve.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Nguồn Sự thật Duy nhất & Xử lý Mã Chưa Giải mã
- **`Data/eve.emg` làm 100% Single Source of Truth**: Bật động cơ Eve mặc định ở production (`EVE_EVENTS_ENABLED = true`), loại bỏ cơ chế ghi đè bởi file `.ini` legacy.
- **Sửa bộ nạp nhị phân `src/data/loaders/eve.rs`**: Chuẩn hóa `win_cond = 13` byte, xóa đọc `fight_limit` (`0` byte), sửa bounds guard `EveResult = 14` byte, và lan truyền lỗi nạp scene thay vì bỏ qua âm thầm để bảo đảm đủ toàn bộ `3,897` scene có nội dung.
- **Chính sách Fail-Closed với mã `UNKNOWN`**: Mọi `condition_class`, `parameter` chưa hỗ trợ trong `src/eve/evaluator.rs` phải trả về `false` kèm telemetry `warn!` (cấm rơi về `actual = 0 -> true`). Các `result_type` / `result_class` chưa giải mã bị bỏ qua an toàn kèm cảnh báo; danh sách mã `UNKNOWN` (`cc ∈ {3,4,5,11,13,15,16,17}`, `rt ∈ {7,8,10}`, `rc ∈ {0,5,6,9..22,80}`, `fight_events`/`win_conds` inner fields) được tách làm backlog kiểm chứng qua tham chiếu server/packet capture ở phiên sau.

### 2.2. Tách Bạch `0x17 homdo` (`Item.dat`) và `0x18 Quest Ledger` (`Mark.dat`)
- **Xóa bỏ khái niệm `quest_items (0x18)`**: Gỡ `Session.quest_items` và nhánh phát `0x18 01` cho item tại login/action. Toàn bộ vật phẩm `Item.dat` (bao gồm vật phẩm nhiệm vụ `result_class = 1`) thao tác duy nhất trên **`homdo` (`0x17`, 25 ô)**.
- **Pre-Flight Probe All-or-Nothing**: Trước khi áp dụng chuỗi hiệu ứng chứa cộng/trừ vật phẩm (hoặc chuỗi thưởng tổng hợp), kiểm tra trước sức chứa `homdo` (`0x17`) và sức chứa Nhật ký Nhiệm vụ (`0x18`, tối đa `200` ô) + giới hạn tiền tệ (`GOLD_CAP`/`POINT_CAP`). Nếu bất kỳ phần nào vượt giới hạn: từ chối toàn bộ chuỗi (All-or-Nothing); khi Nhật ký Nhiệm vụ `200` ô đầy thì gửi `S→C 0x18 03`.
- **Chuẩn hóa `quest_dont` & `0x18 05/07` theo bitmask `Mark.dat`**:
  - Lưu trữ trong RAM/DB tập `bit_id` hoặc byte bitmask tương ứng với `Mark.dat.position` (`1..=2400`).
  - `S→C 0x18 05` nhận `bit_id (1..=2400)`; `S→C 0x18 07` phát các bản ghi `[byte_idx (1..=300): u16 LE][bitmask: u8]`.
  - **Ràng buộc gán cờ hoàn thành**: Chỉ bật bit `quest_dont` khi nhiệm vụ thực sự hoàn tất (hoặc lệnh đánh dấu tường minh), tuyệt đối không bật ở các bước trung gian. Nạp đầy đủ bảng ánh xạ `quest_id <-> bit_id` vào `PlayerEventSnapshot` để `ConditionClass=2 pStyle=3` kiểm tra chính xác.
- **Chặn tuyệt đối `C→S 0x18 01/02`**: Từ chối toàn bộ `C→S 0x18 01/02` (phát sinh từ form dev `TMissionForm`) cho mọi kết nối bao gồm cả GM; GM thao tác quest qua lệnh chat chuyên biệt.

### 2.3. Kiến trúc Thuần Plan–Effect–Commit (`Pure Planner + EventRunner`)
- **Layer Lập kế hoạch Thuần (`src/eve/`)**: Không chạm I/O hay lock Tokio.
  - Chuẩn hóa trigger đầu vào: `ClickNpc`, `ClickDoor`, `MeetDoor`, `WalkStep`, `MenuChoice { surface_id, choice_code }`, `BattleEnd { eve_no, fight_eve_no, outcome }`.
  - Bắt buộc truyền `ChainFilter::RequireDialogChoice(surface_id, choice_code)` khi xử lý `C→S 0x14 09` để chuỗi không chứa điều kiện lựa chọn không thể đè lên lựa chọn người chơi.
  - Thủy hóa đầy đủ `PlayerEventSnapshot`: `homdo` (`0x17`), `follow_npc_ids`, `inn_npc_ids`, `cart_npc_ids`, `role_count_values`, `completed_eve_counts[(scene_id, eve_no)]`, `quest_tasks`, `quest_dont` (qua `Mark.dat`), và `bit_flags`.
  - Ánh xạ các `EveResult` được chọn (kể cả `GroupData` random/all) sang danh sách `DomainEffect` tường minh, chỉ phát `0x14 01..06` cho `result_type ∈ 0..=6` mà client hiểu, đồng thời phát kèm gói đồng bộ server-authoritative (`0x18`, `0x17`, `0x16`, `0x08`, `0x1A`, `0x0C`).
- **Bộ điều phối `EventRunner`**:
  - Một vòng lặp thống nhất thay thế các luồng rời rạc (`handle_talk_start`, `handle_talk_continue`, `handle_talk_select_menu`, `resume_eve_after_battle`, `auto_chain_after`).
  - Phát `14 2C Lock` chỉ một lần đầu chuỗi; giữ nguyên lock xuyên suốt `AutoChain` (sửa `detect_duplicate_items` hỗ trợ `p_style == 0` có `result_value` âm); đóng hội thoại sạch **trước khi** chuyển sang `StartBattle`, và chỉ phát `14 2C Unlock` + `14 08 EndTalk` một lần khi chuỗi kết thúc hoặc không nối tiếp được.
  - Bảo toàn và phát đầy đủ `direct_messages` và `map_broadcast` trong `resume_eve_after_battle`.

### 2.4. Phạm vi Tổ đội (Party) & Ràng buộc Đồng thời
- **Nhiệm vụ Eve chuẩn**: Chỉ Đội trưởng (Leader) thăng tiến bước nhiệm vụ và lưu DB; đồng bộ `quest_tasks` thành viên tổ đội chỉ giữ lại cho các sự kiện map đặc biệt hiện có trong `event_npcs.rs` (`2K`/`Nhị Kiều`/`40 NPC`), đồng thời sửa lỗi chèn `slot = 0` sang cấp phát slot hợp lệ `1..=200` qua `quest_sync`.
- **Door Warp tổ đội (`ClickDoor`/`MeetDoor`/`Warp`)**: Khi dịch chuyển cả đội, bắt buộc gọi `lock_player_operations([min(ids)..max(ids)])` trước khi đọc `registry_get`, đồng bộ `registry_update_if_online`, và persist `(mapid, x, y)` cho toàn bộ thành viên tham gia warp.
- **Vòng đời Trận đấu (`BattleService::battle_ended`)**:
  - Thu thập `sorted_ids` và giữ `lock_player_operations` trước khi đụng session; sau khi chạy `quest_lose_frames`, `battle_quest_win` hoặc `resume_eve_after_battle`, bắt buộc gọi `registry_update_if_online` cho từng người chơi online; làm mới session handle tại `BattleService::register_sender`.
  - **Ngắt kết nối giữa trận Eve**: Khi trận chiến kết thúc mà người chơi đã offline, `battle_ended` vẫn chạy `resume_eve_after_battle` (xử lý thắng/thua/chạy) và ghi kết quả thẳng xuống DB của tài khoản đó để lần đăng nhập kế tiếp giữ đúng trạng thái.

### 2.5. Lược đồ SQLite, Giao dịch Nguyên tử & Write-Through Checkpoints
- **Migration `0002_quest_eve_parity.sql`**:
  - Đổi khóa chính bảng `character_completed_events` thành `(playerid, scene_id, eventid)` (phân tách dữ liệu theo `scene_id` vì `eve_no` cục bộ theo map).
  - Thêm cột `savemap` (`savemap_id`, `savemap_x`, `savemap_y`) vào `characters`.
  - Nối `character_bit_flags` vào `Session` và `SqliteSessionRepository::load/save`; loại bỏ dead code `SqliteQuestRepository` (`quests.rs`).
  - Bảo toàn `PetStorageType` (`Cart = 2`, `Hotel = 3`, `Warehouse = 4`) khi đọc/ghi `character_pets`; sửa `SqliteCharacterRepository::delete` xóa sạch bảng quest liên quan; cập nhật `auto_save::fingerprint()` bao phủ đầy đủ các trường bền vững.
- **Một Transaction Nguyên tử duy nhất**: Gộp `save_eve_state` vào chung SQLite transaction chính trong `SqliteSessionRepository::save`; cấm nuốt lỗi (`Err` phải lan truyền để `auto_save` không đánh dấu `SaveLedger` sai lệch).
- **Write-Through Checkpoints**: Persist nguyên tử ngay lập tức tại 3 ranh giới:
  1. Kết thúc chuỗi sự kiện tạo biến đổi trạng thái (`FinishChain`).
  2. Kết thúc trận đấu (`Battle End` — cả thắng, thua, chạy và trường hợp offline).
  3. Chuyển cảnh qua cửa (`Door Warp` — cả Leader lẫn thành viên tổ đội).

---

## 3. Hệ quả & Đánh đổi (Consequences & Trade-offs)

1. **Tác động tích cực**:
   - Phục hồi toàn bộ `1,120` scene bị mất từ `Data/eve.emg`, loại bỏ sự không nhất quán của `.ini` và bug làm sai lệch `14 01` / `0x18`.
   - Ngăn chặn tình trạng ghi đè ngược reward sau battle (`registry_update_if_online`), mất tiến độ khi crash/disconnect trong trận chiến, và va chạm `eve_no` giữa các bản đồ.
   - Đồng nhất giao thức `0x17` (vật phẩm) và `0x18` (nhật ký `Mark.dat` 200 ô + bitmask 300B) đúng từng byte với `aLogin.exe`.
2. **Đánh đổi & Rủi ro cần kiểm soát**:
   - **Sức chứa 25 ô `homdo`**: Đưa mọi vật phẩm quest qua `homdo (0x17)` tuân thủ đúng 100% client `aLogin.exe` nhưng đòi hỏi người chơi chừa ô trống `homdo` trước khi nhận quest nhiều vật phẩm (giảm thiểu bằng kiểm tra Pre-flight All-or-Nothing trước khi trừ nguyên liệu/phần thưởng khác).
   - **Độ trễ DB tại 3 checkpoint**: Write-through tại `FinishChain`, `Battle End` và `Door Warp` làm tăng số lần ghi SQLite trên Dual-Pool `write` (1 kết nối), đổi lại triệt tiêu cửa sổ mất dữ liệu 180 giây của `auto_save`.
   - **Phạm vi ngoài đợt này (Out of Scope)**: Giải mã các mã raw `UNKNOWN` trong `Data/eve.emg` (`cc ∈ {3,4,5,11,13,15,16,17}`, `rt ∈ {7,8,10}`, `rc ∈ {0,5,6,9..22,80}`, `fight_events`/`win_conds` inner fields) được tách thành backlog nghiên cứu riêng tại `.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md` để đối chiếu với nguồn tham chiếu hoặc packet capture ở phiên sau; các sự kiện dùng mã này tạm thời chạy an toàn theo chế độ fail-closed. (Riêng `14 0A` và `0x1A` đã có đặc tả đầy đủ trong `.scratch/client-pseudo-op-code/opcode_14.md` và `opcode_1a.md` — `0x1A` là kênh 1 chiều S→C `OP_MONEY_SYNC`.)
