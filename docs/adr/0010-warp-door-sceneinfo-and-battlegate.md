# ADR 0010 — Tách Bạch Định Danh `Door ID` vs `SceneInfo ID`, Chính Sách Override `Warps.txt` & Chuẩn Hóa `BattleGate.txt`

- **Ngày**: 2026-11-18
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn `/grill-with-docs`)
- **Tài liệu liên quan**: [ADR 0009](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md), [Báo cáo đối chiếu Warps/Eve/Warp.Dat](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/data-files-and-eve-audit/research-warps-eve-and-warp-dat.md)

---

## 1. Bối cảnh (Context)

1. Đối chiếu trực tiếp [`Data/Warps.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/Warps.txt) (`119,108` bytes, `5,043` dòng → `4,978` cặp khóa `(map1, warpid)` duy nhất), [`Data/BattleGate.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/BattleGate.txt) (`68` cổng), [`Data/eve.emg`](file:///D:/VUDT/GIT_PCC/ts_dream/Data/eve.emg) (`9,887` `Doors`, `10,336` kết quả `rt=2`, `8,882` `SceneInfos`) và mã nguồn Rust (`src/server/handlers/{quest,movement,talk,eve_runner}.rs`, `src/battle/service.rs`) cho thấy:
   - **Xung đột không gian khóa `Door.id` vs `SceneInfo.eve_no`**: Hàm [`resolve_warp_destination`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L927-L957) hiện nhận một `warp_id: u16` chung cho cả luồng kịch bản Eve (`eve_runner` truyền `SceneInfo.eve_no` từ `rt=2 parameter`) lẫn luồng `Talk-Warp 0x14 08` (`handle_warp_confirm` truyền `Door.id` từ client), rồi tra thẳng `scene.scene_infos.get(&warp_id)` trước khi fallback `data.warps`. Do `1,729 / 8,113` cửa trong `eve.emg` có `rt=2 parameter != Door.id`, cách tra thẳng làm **sai bản đồ đích (`dest_map`) ở `677 / 4,978` (`13.6%`) cổng trong `Warps.txt`** (gồm `22 / 68` cổng `BattleGate.txt`) và thiếu `54` cổng.
   - **Dư thừa & lệch tọa độ trong `Warps.txt`**: Khi giải quyết đúng theo chuỗi `Door.id → NpcEvent(rt=2) → SceneInfo`, `4,722` (`94.9%`) khớp tuyệt đối và `4,926` (`99.0%`) khớp `dest_map`; phần `Warps.txt` còn `204` dòng lệch tọa độ thu thập thủ công, `25` dòng khác `dest_map` (gồm `1` lỗi upstream `eve.emg` ở `(25303, 2)` và một số custom/legacy), và `27` dòng không có chuỗi `Door → rt=2` trong `eve.emg`. Về RAM, `GameData.warps` chỉ chiếm `~456 KB`, còn `GameData.scene_eve_data` đã luôn nạp sẵn (`~18.89 MB+`).
   - **Chồng lấn `BattleGate.txt` vs `eve.emg` (`rt=3`)**: `68 / 68` cổng `BattleGate.txt` có mặt trong `eve.doors`, trong đó **`58` cổng đã có trận chiến `rt=3` trong `eve.emg`** và chỉ còn **`10` cổng chưa có `rt=3`** (như chuỗi tháp `49901..49908`, `15481:4`, `21000:40`). Ngoài ra, `handle_warp_confirm` kích hoạt trận `BattleGate` qua `talking_battle`, vốn gọi `battle_quest_win` đọc `data.talks` (rỗng ở production), dẫn tới thắng trận không tự warp và không đánh dấu đã qua ải; đồng thời `MeetDoor` trong `movement.rs` hiện chưa kiểm tra `data.battle_gates`.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Tách Bạch Hai Đường Tra Cứu Warp
- Loại bỏ việc dùng chung `resolve_warp_destination` cho hai không gian khóa khác nhau; tách thành 2 đường định danh rõ ràng:
  1. **`resolve_scene_info_warp(data, map_id, scene_info_id)`**: Dành cho luồng Eve (`DomainEffect::Warp` từ `eve_runner.rs` và `battle/service.rs`). Tra cứu trực tiếp `scene.scene_infos.get(&scene_info_id)` theo `SceneInfo.eve_no`.
  2. **`resolve_door_warp(data, session, map_id, door_id)`** (dùng cho `Talk-Warp 0x14 08` và kiểm tra cổng theo `Door.id`): Tra cứu theo chuỗi `Door[door_id] → NpcEvent (đánh giá điều kiện đầy đủ) → result_type == 2 → SceneInfo[param]`.

### 2.2. Nguồn Sự Thật `Eve-first` + Thu Gọn `Data/Warps.txt` Thành File Override/Fallback
- **Ưu tiên `Data/eve.emg` làm nguồn sự thật chính** cho dịch chuyển cổng (nhất quán với ADR 0009), mở khóa thêm `3,162` cửa và hỗ trợ `282` cửa đa đích.
- **Đánh giá điều kiện `NpcEvent` đầy đủ trên `0x14 08`**:
  - Chuỗi `Door → NpcEvent(rt=2)` ở `0x14 08` bắt buộc đánh giá điều kiện bằng `PlayerEventSnapshot` (`src/eve/evaluator.rs`) để chọn đúng nhánh trong `282` cửa đa đích và tôn trọng điều kiện quest/menu.
  - **Không fallback khi fail điều kiện**: Nếu `Door` trong `eve.emg` có ít nhất một nhánh `rt=2` nhưng không có nhánh nào thỏa điều kiện hiện tại của người chơi, **từ chối warp** (không fallback sang `Warps.txt` để tránh nhảy cóc quest), trừ trường hợp cổng nằm trong danh sách override tường minh.
  - Chỉ fallback sang `Warps.txt` khi `Door` trong `eve.emg` không có bất kỳ `rt=2` nào (hoặc `Door`/scene vắng mặt).
- **Thu gọn `Data/Warps.txt` trên đĩa (~31 dòng hoạt động)**:
  - Xóa `~4,900+` dòng trùng khớp với `eve.emg` (và `204` dòng lệch tọa độ thu thập tay).
  - Chỉ giữ lại trong `Data/Warps.txt`:
    1. **`27` cổng thiếu chuỗi `Door → rt=2` trong `eve.emg`** (`22` cửa có `Door` nhưng thiếu `rt=2` như `11901..11910`, `12994..12999`, và `5` cửa thiếu `Door` như `11840:1`, `11000:21`, `11011:17`, `11583:4`, `12854:2`).
    2. **Fix lỗi upstream `eve.emg`**: `(25303, 2) -> (25304, 170, 790)` (trong `eve.emg` bị nhầm `dest_map_id = 35304`).
    3. **3 cổng custom/event được phê duyệt**: `(59401, 2) -> (12003, 500, 500)`, `(11000, 20) -> (58000, 490, 1790)`, `(27000, 2) -> (27851, 2310, 310)`.
  - `21` bản ghi khác `dest_map` còn lại trong `Warps.txt` cũ tuân theo chuẩn PC `Data/eve.emg`.
  - Các cổng thuộc nhóm override (`(25303, 2)`, `(59401, 2)`, `(11000, 20)`, `(27000, 2)`) được áp dụng ưu tiên trước `eve.emg` trong `resolve_door_warp` (kèm cập nhật các unit/integration test đang đọc `data.warps` như `tests/data_loaders/warps.rs`).

### 2.3. Thu Gọn `Data/BattleGate.txt` & Đồng Bộ Hậu Chiến Cửa Ải
- **Loại bỏ `58` cổng trùng khỏi `Data/BattleGate.txt`**: Giao hoàn toàn việc kích hoạt trận chiến của `58` cổng này cho kịch bản `rt=3` trong `Data/eve.emg`.
- **Chỉ giữ lại `10` cổng thiếu `rt=3` trong `Data/BattleGate.txt`**: Chuỗi tháp `49901..49908` (warp `2`), `(15481, 4)`, và `(21000, 40)`.
- **Kiểm tra đồng nhất ở cả `MeetDoor` và `0x14 08`**: Nếu một cổng nằm trong `data.battle_gates` (`10` cổng còn lại) và người chơi chưa vượt ải đó, chặn và phát động trận chiến ở cả `MeetDoor` (`movement.rs`) lẫn `handle_warp_confirm` (`0x14 08`), ngăn trường hợp `MeetDoor` của `eve.emg` (ví dụ `49902..49907`) cho đi xuyên qua không đánh quái.
- **Hành vi hậu chiến cho `BattleGate`**: Khi chiến thắng trận `BattleGate`, hệ thống **tự động dịch chuyển cả đội qua đích cổng ngay sau trận** (theo logic tương tự `eve_warp` + Two-Phase State Lock + DB write-through) và **đánh dấu trạng thái đã vượt ải** (hoặc đã chuyển tầng đối với tháp `49901..49908`) để không bị kích hoạt đánh lặp vô hạn khi người chơi tương tác lại cổng.

### 2.4. Phạm Vi Không Thay Đổi
- **`Data/Warp.Dat` và `src/data/loaders/warp.rs` giữ nguyên hiện trạng** theo chính sách *Deferred Boot* của ADR 0003 (chỉ xử lý khi triển khai tính năng truyền tống 28 thành thị / opcode `0x44` trong tương lai).

---

## 3. Hệ quả & Đánh đổi (Consequences & Trade-offs)

1. **Tác động tích cực**:
   - Chấm dứt hoàn toàn lỗi dịch chuyển nhầm bản đồ trên `677` cổng (`13.6%` `Warps.txt`, gồm `22` cổng `BattleGate`) và mở khóa thêm `3,162` cổng chuẩn PC từ `Data/eve.emg`.
   - Loại bỏ xung đột hai lần đánh trận giữa `BattleGate.txt` và `eve.emg` trên `58` cổng trùng, đồng thời đảm bảo `10` cổng tháp/ải đặc thù hoạt động nhất quán qua cả `MeetDoor` lẫn `0x14 08` và tự warp khi thắng.
   - Đưa `Data/Warps.txt` từ bảng dữ liệu thủ công 5,043 dòng chồng chéo về file patch/override tinh gọn (~31 dòng), dễ kiểm toán từng ngoại lệ.
2. **Đánh đổi & Lưu ý khi triển khai**:
   - Không còn giữ `204` tọa độ thu thập tay và `21` bản ghi khác `dest_map` cũ của `Warps.txt`; toàn bộ chuyển sang tọa độ gạch chuẩn (`tile * 20 - 10`) và đích chuẩn PC của `Data/eve.emg`.
   - Cần cập nhật các test phụ thuộc vào bảng `Warps.txt` đầy đủ cũ (đặc biệt [`tests/data_loaders/warps.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/data_loaders/warps.rs) đang assert `(49901, 2)` và `(49902, 1)` vốn đã có `rt=2` trong `eve.emg`; sau khi thu gọn cần chuyển test sang kiểm tra các bản ghi override/fallback nằm trong `Warps.txt` rút gọn như `(25303, 2)` hoặc `(11901, 2)`).
   - Việc `resolve_door_warp` kiểm tra `Warps.txt` override (4 cổng) trước `eve.emg` đòi hỏi phân biệt rõ *override* (fix lỗi/custom) với *fallback* (bù thiếu `rt=2`), tránh để `Warps.txt` fallback vượt mặt điều kiện `NpcEvent` của `eve.emg`.
