# Spec: Tách Bạch `Door ID` vs `SceneInfo ID`, Chuẩn Hóa `Eve-first` Cho `Talk-Warp 0x14 08`, Thu Gọn `Warps.txt` & Đồng Bộ `BattleGate`

Status: `ready-for-agent`

---

## Problem Statement

Hệ thống dịch chuyển bản đồ (Warp) và cửa ải chiến đấu hiện tại của server Rust `ts_dream` đang gặp bốn vấn đề nghiêm trọng về tính đúng đắn gameplay, tính nhất quán dữ liệu và xung đột giữa kịch bản Eve với các bảng text cũ:

1. **Nhầm lẫn không gian khóa `Door ID` và `SceneInfo ID` gây sai bản đồ đích trên `677` cổng**:
   - Giao thức `Talk-Warp 0x14 08` gửi lên mã cổng đặt trên bản đồ (`Door ID`), trong khi hiệu ứng dịch chuyển của *Kịch bản Eve* (`result_type == 2`) truyền tham số trỏ tới điểm xuất hiện đích (`SceneInfo ID` / `eve_no`).
   - Do bộ giải quyết đích dịch chuyển hiện dùng chung một đường tra cứu và ưu tiên lấy `Door ID` tra thẳng vào bảng `SceneInfo` trước khi xem xét `Warps.txt`, có tới `677 / 4,978` (`13.6%`) cổng trong `Warps.txt` (bao gồm `22 / 68` *Cửa Ải Chiến Đấu / BattleGate*) bị dịch chuyển nhầm sang bản đồ của cổng khác, và `54` cổng không tìm thấy khóa trùng khớp.
2. **Bỏ sót `3,162` cửa và bỏ qua điều kiện nhiệm vụ/rẽ nhánh trên `Talk-Warp 0x14 08`**:
   - Hàng nghìn cửa thuộc các bản đồ mở rộng chỉ có trong `eve.emg` hoặc có nhiều nhánh đích phụ thuộc tiến độ nhiệm vụ / lựa chọn hội thoại (`282` cửa đa đích) chưa được giải quyết đúng chuỗi `Door ID → NpcEvent → result_type == 2 → SceneInfo ID` kèm kiểm tra điều kiện trên trạng thái người chơi.
   - Khi một cửa có điều kiện nhiệm vụ không thỏa mãn nhưng lại tồn tại dòng tĩnh trong `Warps.txt`, nguy cơ fallback sẽ cho phép người chơi vượt rào nhiệm vụ trái phép.
3. **Dư thừa và sai lệch tọa độ trong bảng `Warps.txt` 5,043 dòng**:
   - Khoảng `99%` dữ liệu `Warps.txt` là trùng lặp hoặc sai lệch tọa độ thu thập thủ công (`204` dòng lệch pixel, `7` dòng `(500, 500)`, `21` dòng khác bản đồ đích với chuẩn PC) so với nguồn chân lý `eve.emg` (ADR 0009), chỉ có khoảng `31` bản ghi thực sự đóng vai trò bù thiếu `result_type == 2` (`27` cổng), sửa lỗi dữ liệu gốc của `eve.emg` (`1` cổng) hoặc giữ đường đi custom đã duyệt (`3` cổng).
4. **Chồng lấn và đứt gãy hậu chiến ở *Cửa Ải Chiến Đấu (BattleGate)***:
   - `58 / 68` cổng trong `BattleGate.txt` đã có trận chiến kịch bản (`result_type == 3`) trong `eve.emg`, tạo ra hai luồng kích hoạt trận đấu song song và dễ xung đột giữa `MeetDoor` và `0x14 08`.
   - `10 / 68` cổng còn lại thiếu `result_type == 3` trong `eve.emg` (như chuỗi tháp `49901..49908`) hiện chỉ được kiểm tra tại `0x14 08` mà bỏ ngỏ tại `MeetDoor` (cho phép đi xuyên qua trên các tầng có `result_type == 2` chạy thẳng). Đồng thời khi thắng trận *BattleGate*, luồng hậu chiến hiện không tự dịch chuyển người chơi qua cổng và không đánh dấu đã vượt ải, khiến người chơi bị kẹt đánh lặp vô hạn.

---

## Solution

Chuẩn hóa toàn bộ kiến trúc phân giải cổng và cửa ải theo **ADR 0010**, **ADR 0009** và **GLOSSARY**:

1. **Tách bạch hai hàm/đường phân giải dịch chuyển theo đúng không gian định danh**:
   - Đường `SceneInfo ID` dành riêng cho hiệu ứng dịch chuyển của *Kịch bản Eve* (tra thẳng bảng điểm xuất hiện đích của bản đồ).
   - Đường `Door ID` dành riêng cho `Talk-Warp 0x14 08` (phân giải theo chuỗi `Door ID → NpcEvent → result_type == 2 → SceneInfo ID`, phối hợp đúng thứ tự ưu tiên với danh sách override/fallback `Warps.txt`).
2. **Áp dụng chính sách `Eve-first` kèm đánh giá điều kiện đầy đủ và thu gọn `Warps.txt`**:
   - Với `Talk-Warp 0x14 08`, ưu tiên tra cứu danh sách override tường minh (`4` cổng đã phê duyệt), tiếp theo phân giải chuỗi `Door ID → NpcEvent` bằng bộ đánh giá điều kiện đầy đủ trên hồ sơ trạng thái người chơi (`PlayerEventSnapshot`).
   - Nếu `Door` có ít nhất một nhánh `result_type == 2` trong `eve.emg` nhưng không nhánh nào thỏa điều kiện hiện tại → **từ chối dịch chuyển**, tuyệt đối không fallback sang `Warps.txt`. Chỉ fallback sang `Warps.txt` khi `Door` không có bất kỳ `result_type == 2` nào hoặc vắng mặt trong `eve.emg`.
   - Thu gọn `Warps.txt` từ `5,043` dòng xuống còn `~31` dòng hoạt động (`27` cổng thiếu `result_type == 2`, `1` bản vá lỗi gốc `(25303, 2) -> 25304`, và `3` custom override đã duyệt).
3. **Thu gọn `BattleGate.txt` xuống `10` cổng ngoại lệ, đồng bộ kiểm tra ở cả `MeetDoor` và `0x14 08`, và hoàn thiện hậu chiến**:
   - Xóa `58` cổng đã có `result_type == 3` trong `eve.emg` khỏi `BattleGate.txt`, chỉ giữ lại `10` cổng chưa có `result_type == 3` (`49901..49908` cổng `2`, `15481:4`, `21000:40`).
   - Kiểm tra chốt chặn *BattleGate* nhất quán ở cả luồng va chạm `MeetDoor` lẫn luồng `Talk-Warp 0x14 08` (bỏ qua nếu người chơi đã vượt ải đó).
   - Khi trận chiến phát động từ `BattleGate.txt` kết thúc với thắng lợi (`PlayerWin`), tự động ghi nhận trạng thái đã vượt ải cho cửa ải (nếu cần chặn lặp tại cùng bản đồ) và tự động dịch chuyển cả đội qua đích cổng theo đúng quy trình `Two-Phase State Lock` + `Write-Through Checkpoint`.

---

## User Stories

### Phân hệ 1: Tách bạch `Door ID` vs `SceneInfo ID` & Sửa lỗi sai bản đồ `677` cổng
1. Là một Người chơi đi qua bất kỳ cổng nào trong số `677` cổng trước đây có `Door ID != SceneInfo ID` bằng lệnh `Talk-Warp 0x14 08`, tôi muốn được đưa tới đúng bản đồ và tọa độ đích của cánh cửa đó, thay vì bị dịch chuyển nhầm sang bản đồ của cổng khác.
2. Là một Người chơi bước vào cổng được điều khiển bởi chuỗi *Kịch bản Eve* (`MeetDoor`, `ClickDoor`, hoặc hậu chiến `Eve`), tôi muốn hiệu ứng dịch chuyển tiếp tục tra cứu trực tiếp bằng `SceneInfo ID` mà *Kịch bản Eve* cung cấp, để các chuỗi sự kiện nhiệm vụ không bị gãy hoặc nhầm khóa với `Door ID`.
3. Là một Đội trưởng tổ đội kích hoạt dịch chuyển hợp lệ qua `0x14 08` hay *Kịch bản Eve*, tôi muốn toàn bộ thành viên tổ đội đang theo sau được cập nhật vị trí mới đồng bộ qua khóa hai pha và lưu nguyên tử xuống SQLite.
4. Là một Thành viên đang đi theo Đội trưởng (không phải Đội trưởng), tôi muốn yêu cầu tự ý dịch chuyển cổng đơn lẻ của tôi bị từ chối nhất quán ở mọi luồng cổng.

### Phân hệ 2: `Eve-first`, Đánh giá Điều kiện `NpcEvent` & Ranh giới Fallback `Warps.txt`
5. Là một Người chơi gửi `Talk-Warp 0x14 08` tại một trong `3,162` cửa hợp lệ có trong `eve.emg` nhưng trước đây vắng mặt trong `Warps.txt`, tôi muốn đi qua cổng bình thường tới đúng bản đồ đích chuẩn PC.
6. Là một Người chơi gửi `Talk-Warp 0x14 08` tại một trong `282` cửa đa đích có rẽ nhánh theo tiến độ nhiệm vụ (ví dụ bến cảng hoặc cổng sự kiện), tôi muốn hệ thống đánh giá đầy đủ điều kiện `NpcEvent` dựa trên trạng thái hiện tại của tôi để đưa tôi tới đúng nhánh đích tương ứng.
7. Là một Người chơi chưa đạt điều kiện nhiệm vụ tại một cánh cửa có `result_type == 2` bị khóa bởi điều kiện trong `eve.emg`, khi tôi gửi `0x14 08` trực tiếp, tôi muốn yêu cầu dịch chuyển bị từ chối an toàn (không fallback sang `Warps.txt`), để không thể nhảy cóc vượt qua điều kiện cốt truyện.
8. Là một Người chơi đi qua một trong `27` cánh cửa đặc thù thiếu `result_type == 2` trong `eve.emg` (như chuỗi tháp `11901..11910`, `12994..12999`, hoặc bản đồ `11840`), tôi muốn hệ thống fallback sang `Warps.txt` rút gọn để vẫn di chuyển qua lại bình thường.
9. Là một Người chơi đi qua cổng sửa lỗi `(25303, 2)` hoặc `3` cổng custom đã phê duyệt (`(59401, 2)`, `(11000, 20)`, `(27000, 2)`), tôi muốn bản vá trong `Warps.txt` được ưu tiên áp dụng trước `eve.emg` để tôi không bị kẹt ở bản đồ lỗi `35304` và giữ đúng đường đi custom mong muốn.
10. Là một Người chơi đi qua `21` cổng còn lại trước đây có `dest_map` khác biệt không có cơ sở trong `Warps.txt` cũ hoặc `204` cổng lệch tọa độ thu thập tay, tôi muốn được đưa tới đúng bản đồ và tọa độ tile chuẩn (`tile * 20 - 10`) từ `eve.emg`.
11. Là một Quản trị viên / Bảo trì viên dữ liệu, tôi muốn file `Warps.txt` chỉ giữ đúng `~31` dòng override/fallback có chú thích phân nhóm rõ ràng (lỗi upstream, custom override, thiếu `result_type == 2`), để dễ kiểm toán và không còn mang `~4,900` dòng dư thừa.

### Phân hệ 3: Thu gọn `BattleGate.txt`, Nhất quán `MeetDoor` vs `0x14 08` & Hậu chiến Cửa Ải
12. Là một Người chơi tiếp cận bất kỳ cổng nào trong `58` cổng đã có trận chiến `result_type == 3` trong `eve.emg`, tôi muốn việc kiểm tra và phát động trận chiến do *Kịch bản Eve* đảm nhiệm hoàn toàn (không bị lặp kiểm tra hay xung đột với `BattleGate.txt`), và sau khi thắng trận Eve tôi được chuyển cảnh hoặc mở khóa đúng theo kịch bản gốc.
13. Là một Người chơi bước chạm vào (`MeetDoor`) hoặc xác nhận qua (`0x14 08`) một trong `10` *Cửa Ải Chiến Đấu (BattleGate)* ngoại lệ còn lại trong `BattleGate.txt` (như các tầng tháp `49901..49908` cổng `2`, `15481:4`, `21000:40`) khi chưa vượt ải, tôi muốn trận chiến thủ ải được kích hoạt đồng nhất ở cả hai luồng, không thể đi xuyên qua bằng `MeetDoor` ở các tầng `49902..49907`.
14. Là một Người chơi chiến thắng trận chiến phát động từ `10` cổng `BattleGate.txt` nói trên, tôi muốn cả đội hình của mình được tự động dịch chuyển qua đích của cổng ngay khi trận đấu kết thúc thành công và lưu xuống cơ sở dữ liệu nguyên tử.
15. Là một Người chơi vừa đánh thắng một *Cửa Ải Chiến Đấu (BattleGate)*, khi quay lại hoặc tương tác với cổng đã vượt trong cùng phiên/tiến trình (với các cổng không tự đẩy sang bản đồ khác hoặc cho phép quay lại), tôi muốn cổng ghi nhận trạng thái đã vượt ải để cho đi qua mà không ép đánh lặp lại trận đấu đó một cách vô hạn; còn nếu thua hoặc bỏ chạy, chốt chặn vẫn giữ nguyên.
16. Là một Quản trị viên / Bảo trì viên, tôi muốn toàn bộ quy chuẩn phân giải dịch chuyển và kiểm thử nạp dữ liệu `Warps.txt` / `BattleGate.txt` phản ánh đúng cấu trúc file rút gọn mới mà không phụ thuộc vào dữ liệu legacy `Data_Talks` đã bỏ.

---

## Implementation Decisions

1. **Tách giao diện phân giải đích dịch chuyển tại tầng Handler Nhiệm vụ / Dịch chuyển**:
   - Bỏ giao diện dùng chung `resolve_warp_destination(data, map_id, warp_id)` hoặc phân tách tường minh thành hai điểm vào công khai với ngữ nghĩa tách biệt:
     - **Đường `SceneInfo ID`**: `resolve_scene_info_warp(data, map_id, scene_info_id)` — tra thẳng bảng điểm xuất hiện (`scene_infos`) của bản đồ nguồn, yêu cầu bản đồ đích và tọa độ xuất hiện dương, quy đổi tile sang pixel (`tile * 20 - 10`). Chỉ sử dụng bởi bộ thực thi *Kịch bản Eve* (`eve_runner`) và đường hậu chiến `eve_warp` của `BattleService`.
     - **Đường `Door ID`**: `resolve_door_warp(data, session, map_id, door_id)` — dành cho `Talk-Warp 0x14 08` và các kiểm tra cổng dựa trên `Door ID` (bao gồm tra đích sau khi thắng *BattleGate*).
2. **Hợp đồng thực thi của `resolve_door_warp`**:
   - **Bước 1 — Override tường minh**: Kiểm tra trước xem `(map_id, door_id)` có nằm trong tập `4` cổng override được phê duyệt (`(25303, 2)`, `(59401, 2)`, `(11000, 20)`, `(27000, 2)`) hay không; nếu có và tồn tại trong `data.warps`, trả về đích từ `data.warps`. (Có thể đánh dấu cờ `is_override` hoặc cấu hình hóa rõ ràng trên dữ liệu nạp từ `Warps.txt` hoặc tập hợp khóa được chỉ định rõ trong loader/resolver mà không trộn lẫn với fallback).
   - **Bước 2 — Phân giải chuỗi `Door → NpcEvent → rt=2 → SceneInfo` (`Eve-first`)**:
     - Tra `Door` trong `scene.doors` theo `door_id`.
     - Quét danh sách `event_id` gắn trên `Door` (đồng bộ với cơ chế lọc trigger của `NpcEvent` trên cổng): dựng `PlayerEventSnapshot` một lần cho người chơi và đánh giá toàn bộ điều kiện bằng bộ đánh giá điều kiện `eve::evaluator`.
     - Chỉ lấy kết quả `result_type == 2` thuộc nhánh điều kiện thỏa mãn để tra `scene.scene_infos` theo `parameter` (`SceneInfo ID`). Nếu một sự kiện có kết quả `result_type == 2` cùng `Group` hoặc nhánh đầu tiên thỏa điều kiện, phân giải đích từ `SceneInfo` đó.
     - **Bất biến chặn vượt rào (Fail-Closed)**: Nếu `Door` có ít nhất một sự kiện gắn kèm chứa `result_type == 2` trong `eve.emg`, nhưng không có nhánh nào thỏa điều kiện hiện tại của người chơi, hàm trả về `None` (từ chối dịch chuyển) và **không** rơi xuống fallback `data.warps`.
   - **Bước 3 — Fallback `Warps.txt`**: Chỉ khi `scene` không có, `Door` không tồn tại trong `scene.doors`, hoặc toàn bộ các sự kiện trên `Door` đều không có bất kỳ kết quả `result_type == 2` nào, mới tra cứu `data.warps` cho `(map_id, door_id)`.
3. **Đồng bộ kiểm tra và thực thi hậu chiến cho *Cửa Ải Chiến Đấu (BattleGate)***:
   - **Nhất quán giữa `MeetDoor` và `0x14 08`**:
     - Trong luồng xử lý bước chạm cổng `MeetDoor` (trình xử lý di chuyển) và luồng `Talk-Warp 0x14 08` (trình xử lý xác nhận cổng), kiểm tra `data.battle_gates.get(&(map_id, door_id))` trước khi cho dịch chuyển (đối với `MeetDoor`, kiểm tra trước hoặc phối hợp sạch với `resolve_npc_event(MeetDoor)` để không cho `49902..49907` trượt thẳng qua `result_type == 2` của `eve.emg`).
     - Bỏ qua kiểm tra *BattleGate* nếu người chơi đã đánh dấu vượt cửa ải `(map_id, door_id)` trong tập trạng thái cửa ải đã vượt trên phiên người chơi (hoặc cờ tương đương).
   - **Theo dõi ngữ cảnh trận chiến cửa ải & Hậu chiến trên `BattleService`**:
     - Thay vì gán chung chung vào cơ chế `talking_battle` phụ thuộc bảng `Data_Talks` đã bỏ, gắn thêm ngữ cảnh cửa ải chờ dịch chuyển trên phiên hoặc mô tả trận đấu (`pending_battle_gate: Option<(u16, u16)>` chứa `(map_id, door_id)` — hoặc mang thông tin đích đã giải quyết trước trận) để `BattleService::battle_ended` phân biệt rõ trận *BattleGate* với trận `Data_Talks` legacy.
     - Khi trận *BattleGate* kết thúc với kết quả `PlayerWin`:
       1. Ghi nhận `(map_id, door_id)` vào danh sách cửa ải đã vượt trên phiên người chơi (ngăn lặp vô hạn tại cổng đó, xóa ngữ cảnh chờ).
       2. Cập nhật đồng bộ trong `lock_player_operations` và xếp hàng thực hiện dịch chuyển cả đội qua `perform_party_warp_for_locked_session` (dùng `resolve_door_warp` để lấy đích cổng) với đầy đủ gói `14 07`, `0x0C`, `hide_from_map`, cập nhật `Online Session Registry` và `Write-Through Checkpoint` xuống SQLite (kể cả trường hợp thành viên/chủ đội đã ngắt kết nối giữa trận).
     - Khi trận *BattleGate* kết thúc với `PlayerLose` hoặc bỏ chạy: chỉ xóa ngữ cảnh trận đang chờ, **không** đánh dấu đã vượt ải và **không** dịch chuyển.
4. **Chuẩn hóa dữ liệu tĩnh `Warps.txt` và `BattleGate.txt`**:
   - **`Warps.txt`**: Thu gọn còn `~31` dòng hoạt động (chia nhóm comment rõ ràng: `4` dòng override được duyệt gồm fix `(25303, 2)` + `3` custom override; và `27` dòng fallback cho các cổng thiếu `result_type == 2`). Đảm bảo `GameData::load_warps` và bộ phân giải nhận diện chính xác nhóm override vs fallback (ví dụ bằng cấu trúc metadata/tập khóa override rõ ràng trong loader hoặc module phân giải).
   - **`BattleGate.txt`**: Gỡ `58` dòng trùng lặp với `eve.emg` (`result_type == 3`), giữ đúng `10` dòng thiếu `result_type == 3` (`49901..49908` warp `2`, `15481:4`, `21000:40`).
   - Không sửa đổi bất kỳ cấu trúc hay loader nào của `Warp.Dat` và `Mark.Dat`.

---

## Testing Decisions

Tuân thủ chuẩn kiểm thử của dự án (`CODING_STANDARDS.md` §3): toàn bộ kiểm thử tích hợp đặt dưới các module con của thư mục `tests/` (tuyệt đối không đặt `#[cfg(test)]` trong `src/`), chỉ kiểm thử hành vi quan sát được từ bên ngoài (gói tin wire đầu ra, thay đổi trạng thái `Session`, `Online Session Registry` và SQLite DB), không kiểm thử chi tiết cài đặt nội bộ, kèm đầy đủ các ca phủ định (negative cases) và kiểm tra chống ảnh hưởng chéo (collateral integrity).

### Các Seam Kiểm Thử Cao Nhất (Proposed Test Seams)

1. **Seam 1 — Wire Dispatcher (`dispatch` — Seam chính & cao nhất)**:
   - Đẩy trực tiếp các khung tin `0x14 08` (`Talk-Warp`), `0x06` (`Walk` → `MeetDoor`), `0x14 07` (`ClickDoor`) và `0x0C 01` (`Confirm Map`) qua hàm điều phối trung tâm `dispatch` đối với cả dữ liệu thật (`shared_game_data()`) và dữ liệu tổng hợp có kiểm soát:
     - **Sửa lỗi `677` cổng**: Gửi `0x14 08` tại các cổng có `Door ID != SceneInfo ID` (ví dụ tại `12000` cổng `14` hoặc các cổng từng bị lệch trong `677` cổng) và kiểm chứng gói `0x0C` cùng `Session.map_id` đi đúng đích chuỗi `Door → rt=2 → SceneInfo`, không bị khớp nhầm `scene_infos[Door.id]`.
     - **Mở khóa `3,162` cổng & Đa nhánh `282` cổng**: Gửi `0x14 08` tại cổng chỉ có trong `eve.emg` và cổng đa nhánh có điều kiện nhiệm vụ (`ConditionClass::QuestStep` / `DialogChoice`) — thỏa điều kiện đi đúng nhánh đích, chưa thỏa điều kiện bị từ chối sạch (không rơi về `Warps.txt`).
     - **Ranh giới Override & Fallback**: Kiểm chứng `(25303, 2)` đi tới `25304` (override thắng `eve.emg`), các cổng custom override giữ nguyên đích duyệt, `27` cổng thiếu `rt=2` (như `11901:2`) fallback qua `Warps.txt`, trong khi `21` cổng cũ khác `dest_map` không nằm trong override đi theo chuẩn `eve.emg`.
     - **Chốt chặn `MeetDoor` & `0x14 08` trên `10` cổng `BattleGate`**: Kiểm chứng bước vào `49902:2` qua cả `Walk` (`MeetDoor`) và `0x14 08` đều phát sinh `BattleTrigger` với `teamdef` của `BattleGate.txt` thay vì nhảy thẳng qua cổng; còn các cổng thuộc `58` cổng đã chuyển sang `eve.emg` do kịch bản Eve điều phối trận chiến.
     - **Kiểm tra tách biệt `SceneInfo ID` cho `Eve`**: Kiểm chứng các luồng `ClickDoor` / `MeetDoor` có `Door ID != SceneInfo ID` vẫn dịch chuyển qua đường `SceneInfo ID` đúng đích và gửi đủ gói `14 07` + `0x0C` + `hide_from_map` cho cả tổ đội.
   - *Prior art trong codebase*: `tests/server_network/movement_warp.rs`, `tests/eve_quest/tickets_11_12_13.rs`, `tests/eve_quest/door_triggers_walkstep_and_party_warp.rs`, `tests/eve_quest/npc_eve_e2e.rs`.

2. **Seam 2 — Vòng đời Kết thúc Trận đấu (`BattleService::battle_ended`)**:
   - Khởi tạo phiên trận đấu cho 1 trong `10` cổng `BattleGate` (có đủ tổ đội và SQLite DB test), kết thúc trận với các kết quả khác nhau:
     - **`PlayerWin`**: Xác minh cả đội trưởng và thành viên tổ đội được tự động dịch chuyển qua đích cổng (`14 07`, `0x0C`, cập nhật `Online Session Registry` và SQLite `characters`), đánh dấu cổng đã vượt ải, và khi gửi lại `0x14 08` / `MeetDoor` trên cổng đã vượt sẽ đi qua trực tiếp không lặp trận.
     - **`PlayerLose` / Flee**: Xác minh người chơi giữ nguyên ở bản đồ trước cổng, không đánh dấu vượt ải, và lần tương tác tiếp theo vẫn kích hoạt trận thủ ải.
   - *Prior art trong codebase*: `tests/eve_quest/tickets_11_12_13.rs` (post-battle deferred warp & persistence).

3. **Seam 3 — Nạp Dữ Liệu Production (`shared_game_data` / `GameData::load`)**:
   - Cập nhật bộ test nạp `Warps.txt` và `BattleGate.txt` trong `data_loaders` để xác minh file thu gọn nạp thành công trên production boot:
     - `data.warps` chứa đúng tập override (`(25303, 2)` -> `25304`, `3` custom override) và `27` cổng thiếu `rt=2`, không chứa `~4,900` dòng trùng lặp cũ.
     - `data.battle_gates` chứa đúng `10` cổng thiếu `rt=3` (có `49901..49908` warp `2`, `15481:4`, `21000:40`) và không chứa `58` cổng đã giao cho `eve.emg`.
   - *Prior art trong codebase*: `tests/data_loaders/warps.rs`.

---

## Out of Scope

- **Mọi thay đổi đối với `Data/Warp.Dat`, `WarpDatLoader` và `opcode 0x44` (truyền tống 28 thành thị)**: Giữ nguyên trạng thái Deferred Boot theo ADR 0003.
- **Tối ưu hóa RAM bằng Lazy Loading cho `eve.emg` hoặc strip visual fields trong `EveDataLoader`**: Không thuộc phạm vi đợt sửa lỗi định danh và chuẩn hóa dữ liệu warp này.
- **Sửa đổi bảng `Data/Mark.Dat` hoặc giao thức `0x18` (Quest Sync)**: Đã chuẩn hóa theo ADR 0009.
- **Giải mã các mã `UNKNOWN` (`cc`, `rt`, `rc`) còn lại trong `eve.emg`**: Thuộc backlog riêng của ADR 0009.
- **Lưu trữ bền vững lâu dài (DB migration mới) cho lịch sử vượt ải *BattleGate* ngoài chuỗi tháp chuyển bản đồ**: Trước mắt trạng thái đã vượt ải trong phiên (session-scoped) đủ để ngăn lặp trận vô hạn; nếu cần lưu vĩnh viễn qua restart cho các cổng đặc thù ngoài quest sẽ tách thành migration riêng sau.

---

## Further Notes

- **Cảnh báo thứ tự thực thi khi triển khai**:
  1. Cập nhật mã nguồn tách `resolve_scene_info_warp` vs `resolve_door_warp`, kiểm tra `MeetDoor` cho `BattleGate` và xử lý hậu chiến `BattleService` **trước** hoặc **đồng thời** với việc cắt gọn `Data/Warps.txt` và `Data/BattleGate.txt`, tránh giai đoạn chuyển tiếp làm gãy các cổng đang vận hành.
  2. Rà soát các test hiện hữu đang dựa trên giả định cũ (`Warps.txt` đầy đủ hoặc ưu tiên `scene_infos[warp_id]` cho `0x14 08`) để cập nhật đồng bộ theo hợp đồng mới trong ADR 0010.
  3. Theo `AGENTS.md`, `build.rs` đồng bộ `Data/` cạnh binary khi build/test, nên sau khi chỉnh `Data/Warps.txt` và `Data/BattleGate.txt` cần chạy `cargo test --all-targets --no-fail-fast` để xác nhận toàn bộ các test suite vượt qua.
