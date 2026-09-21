# Nghiên cứu Checkpoint 6: AutoChain, Battle Trigger & SQLite Persistence

## A. Tích hợp AutoChain (Tự động nhảy bước kế)

### 1. Phân tích luồng `EveAutoChainEngine`
File `src/eve/auto_chain.rs` hiện thực `EveAutoChainEngine::try_auto_chain`. Hàm này hoạt động độc lập (pure) để thử kích hoạt chuỗi sự kiện tiếp theo:
- **Input**: `completed: &EventSession`, `scene: &SceneEveData`, `state: &PlayerEventState`.
- **Output**: `AutoChainResult::Chained(EventSession)` hoặc `NoMatch`.
- **Các lớp bảo vệ (Guards)**: 
  - `detect_same_chain`: Chống kẹt vòng lặp điều kiện lặp lại liên tục.
  - `detect_re_question`: Chống hiển thị lại menu/dialog ngay khi vừa chọn.
  - `detect_re_battle`: Chống đánh liên tục một trận.
  - `detect_duplicate_items`: Chống lặp vòng lặp trao vô tận cùng 1 item.

### 2. Thiết kế tích hợp vào `Session` và `talk.rs`
Hiện tại, `EventSession` đang bị "rơi" sau khi `resolve_npc_event` trả về do `talk.rs` vẫn đang sử dụng luồng `QuestDef` / `Data_Talks` legacy (`conn.session.idtalking`, `talk_count`).
**Cần thay đổi `Session` (`src/server/session.rs`):**
```rust
pub struct Session {
    // ...
    /// Phiên làm việc Eve Script hiện tại, chứa toàn bộ trạng thái tiến trình sự kiện.
    pub active_event: Option<EventSession>,
}
```

**Luồng thực thi trong `talk.rs` (`handle_talk_continue`):**
1. Lấy kết quả từ `conn.session.active_event`.
2. Thực thi tuần tự các `EveResult` trong `active_event.results`.
3. Khi duyệt hết (`current_index >= results.len()`):
   - Đánh dấu hoàn thành event (tăng biến đếm `completed_eve_counts` lưu DB nếu `has_state_changing_results()`).
   - Gọi `auto_chain_after()` (có sẵn ở `src/server/handlers/npc_event.rs`).
   - Nếu trả về `AutoChainResult::Chained(new_session)`, gán `conn.session.active_event = Some(new_session)` và thực thi frame đầu tiên mà KHÔNG yêu cầu client gửi thêm gói tin click (chu trình tự khép kín).
   - Nếu `NoMatch`, gửi `EndTalk` (Opcode 0x14 sub 0x08) và xóa `active_event`.

## B. Battle Trigger (Kích hoạt trận đánh từ Eve)

### 1. Ý nghĩa tham số `result_type == 3`
Dựa trên kiến trúc `eve.emg` (`src/data/loaders/eve.rs`):
Khi kết quả của `resolver` trả về `result_type == 3` (Battle), cấu hình trận đấu không nằm ở Npc.dat mà nằm trực tiếp trong file eve qua **Section 9: FightData (`Eve_FightData.lua`)**.
- Mã bản đồ sự kiện chứa: `pub fight_datas: HashMap<u16, EveFightData>`.
- `EveFightData` chứa danh sách kẻ địch (`left_enemies`, `right_enemies` - kiểu `EveFightEnemy` có ID, toạ độ, AI).
- Tham số `parameter` hoặc `result_mean_no` (tuỳ thuộc kiểu dữ liệu) của `EveResult` sẽ ánh xạ với key `eve_no` của `fight_datas`.

### 2. Thiết kế Flow vào Battle Engine
Khi hàm thực thi `EveResult` gặp `result_type == 3`:
1. Chuyển `EventSession.phase = EventPhase::AwaitingBattle`.
2. Đọc `EveFightData` từ `GameData` dựa trên `parameter`.
3. Sinh `BattleTrigger` (cần tạo kiểu mới hoặc cấu trúc lại kiểu legacy `teamdef: Vec<i64>`).
   - Đề xuất bổ sung biến thể vào `BattleTrigger` ở `dispatcher.rs` để truyền trực tiếp `EveFightData` thay vì mảng `i64`.
4. Sau khi kết thúc trận (thắng/thua), hàm `process_battle_win` hoặc handler battle sẽ cập nhật `EventSession.battle_result` (1=Win, 2=Lose, 3=Flee) và gọi lại `auto_chain_after()` để chạy kịch bản sau trận (như Bear C# `BattleContinuationStepHandler.cs`).

## C. Door / Warp (Chuyển Map)

Khi `result_type == 2` (Door/Warp):
- Tham số ánh xạ trực tiếp tới `EveSceneInfo` trong **Section 8: SceneInfoData** (`src/data/loaders/eve.rs`).
- Cấu trúc `EveSceneInfo` chứa `background_no` (Map ID đích) và `player_appear_x`, `player_appear_y` (Tọa độ).
- **Thực thi**: Xóa `active_event`, gửi Opcode dịch chuyển map (0x0C warp confirm) tới `movement.rs`.

## D. SQLite Quest Persistence

### 1. Sơ đồ CSDL hiện có (`migrations/0001_init.sql` & `quests.rs`)
Đã có sẵn Repository `SqliteQuestRepository` (src/db/modern/sqlite/quests.rs):
- `character_missions` (Lưu `mission_id`, `step`, `state`)
- `character_bit_flags` (Lưu các Quest Dont / cờ bit)
- `character_completed_events` (Lưu `event_id` - đếm số lần hoàn thành sự kiện)

### 2. Thiết kế Persistence ở Session và DbPool
**Nạp khi Login:**
- Lúc đăng nhập, đọc tất cả quest, bit flags và completed events từ CSDL thông qua `pool.read`.
- Đổ vào `Session`:
  - `pub quest_tasks: HashMap<i32, i32>` (hoặc cấu trúc `MissionRow`)
  - `pub quest_dont: HashSet<u32>` (từ `character_bit_flags`)
  - `pub completed_eve_counts: HashMap<i32, i32>`
- Gắn vào `snapshot_state()` để `PlayerEventState` lấy đầy đủ thông tin tham chiếu lúc resolve condition.

**Ghi khi chạy AutoSave / Kết thúc Event:**
- `src/db/persist.rs` (Persist Layer) đang được dùng (chế độ ghi-through/Dual-Pool).
- Khi `EventSession` có `has_state_changing_results() == true`, lập tức gọi hàm ghi đè thay đổi (quest step mới, flag mới) qua channel hoặc trực tiếp xuống `pool.write` để đảm bảo độ tin cậy.

## E. Thiết kế Test Kịch bản End-to-End (`tests/npc_eve_e2e_test.rs`)

Sử dụng môi trường Test Harness và In-memory SQLite (`sqlite::memory:`):
1. **Scenario 1 (Thoại & Chuyển map)**: Giả lập player gặp cổng thành (Door trigger) $\to$ Giải quyết sự kiện $\to$ Gặp `result_type == 2` $\to$ Xác minh `Session` có `map_id` mới.
2. **Scenario 2 (Thoại $\to$ Nhiệm vụ $\to$ Trận đánh)**: NPC tân thủ Trác Quận (ClickNpc). Xác minh `auto_chain` kích hoạt $\to$ nhận quest item $\to$ sinh `BattleTrigger` $\to$ Mô phỏng thắng trận $\to$ Xác minh auto_chain nhảy tới bước nhận thưởng $\to$ Kiểm tra Db chứa mission id = X.

