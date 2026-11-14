Status: ready-for-agent

# Spec: Chuẩn hóa Động cơ Kịch bản Eve, Nhật ký Nhiệm vụ `0x18` & Bền vững Hóa Dữ liệu Quest

---

## Problem Statement

Hệ thống nhiệm vụ và tương tác NPC/cửa (`Kịch bản Eve`, Opcode `0x14` / `0x18`) hiện tại trên server Rust đang gặp 5 nhóm sự cố khiến chuỗi nhiệm vụ trên client `aLogin.exe` bị gãy, sai dữ liệu hoặc mất tiến độ:

1. **Mất gần 1/3 dữ liệu kịch bản gốc**: Bộ nạp nhị phân đọc sai cấu trúc `FightData` trong `Data/eve.emg` và âm thầm nuốt lỗi, làm rơi mất `1,120 / 3,901` bản đồ (scene) và làm lệch dữ liệu trận chiến ở các bản đồ còn lại; đồng thời động cơ Eve đang tắt mặc định khi chạy production.
2. **Hiểu sai ranh giới giữa Túi đồ (`0x17`) và Nhật ký Nhiệm vụ (`0x18`)**: Server giả định tồn tại kho vật phẩm nhiệm vụ 200 ô ở `0x18` (`quest_items`) và giới hạn `quest_dont` ở `1..=300`, trong khi client `aLogin.exe` dùng `0x18 01/02/04/06` cho Nhật ký Nhiệm vụ (`Mark.dat` 200 ô), dùng `0x18 05/07` cho mảng bit 300 byte (`bit_id 1..=2400` trong `Mark.dat`), và đưa 100% vật phẩm `Item.dat` qua `homdo` (`0x17`, 25 ô). Sự lệch pha này làm gãy điều kiện trả vật phẩm, ghi đè cờ hoàn thành ở bước trung gian, và chặn sai các quest có `Mark.dat position > 300`.
3. **Đánh giá điều kiện và thực thi hiệu ứng thiếu kiểm soát**: Điều kiện chưa hỗ trợ rơi về mặc định `0 == 0 -> true` thay vì thất bại an toàn; thiếu thủy hóa pet đi theo/kho/cờ; chọn menu `0x14 Surface` có thể bị chuỗi không chứa menu đè lên; thiếu xử lý nhiều action phổ biến (`NpcTeam`, `RewardPet`, `ClickDoor`/`MeetDoor`, cộng EXP lên cấp, `AutoChain` trừ đồ); và bị giật khóa UI hoặc rơi broadcast khi chuyển cảnh/sau trận đấu.
4. **Mất đồng bộ RAM – Battle – Tổ đội**: Kết thúc trận chiến chưa khóa theo quy chuẩn `Two-Phase State Lock` và không gọi `registry_update_if_online`, khiến gói tin tiếp theo của người chơi ghi đè ngược mất phần thưởng/bước quest vừa nhận; người chơi ngắt kết nối giữa trận bị mất sạch kết quả hậu sự kiện; và dịch chuyển cửa theo tổ đội không khóa hoặc không lưu trạng thái cho thành viên.
5. **Lưu trữ SQLite không nguyên tử và va chạm khóa**: Lưu trạng thái Eve tách sang giao dịch SQLite thứ hai và nuốt lỗi; bảng đếm hoàn thành sự kiện thiếu `scene_id` nên nhiệm vụ bản đồ này ghi đè bản đồ khác; thiếu lưu `savemap`, cờ bit và thiếu write-through tại các điểm chốt quan trọng.

---

## Solution

Tái cấu trúc phân hệ Quest / NPC Talk theo mô hình **`Pure Eve Planner` + `EventRunner` + Giao dịch Nguyên tử Hai Pha**, bám sát 100% hành vi nhị phân của `Data/eve.emg`, `Data/Mark.dat` và `aLogin.exe` (đã chốt trong ADR 0009 và `GLOSSARY.md`):

- **Đơn nhất hóa nguồn dữ liệu (`Single Source of Truth`)**: Nạp đầy đủ `3,897` scene có nội dung từ `Data/eve.emg`, bật động cơ Eve mặc định ở production, gỡ bỏ cơ chế ghi đè `.ini` legacy, và áp dụng fail-closed (kèm log cảnh báo) đối với các mã `UNKNOWN`.
- **Tách bạch `0x17 homdo` và `0x18 Quest Ledger`**: Đưa toàn bộ vật phẩm `Item.dat` qua `homdo` (`0x17`) với kiểm tra All-or-Nothing trước khi thực thi; chuẩn hóa `0x18` cho Nhật ký Nhiệm vụ `Mark.dat` (`1..=200` slot, đầy thì từ chối + phát `0x18 03`), chuẩn hóa bitmask `quest_dont` (`bit_id 1..=2400` trên `0x18 05`, mảng 300 byte trên `0x18 07`, chỉ đánh dấu khi thực sự hoàn thành quest), và từ chối hoàn toàn `C→S 0x18 01/02`.
- **Thực thi hợp nhất & Đồng bộ Tổ đội/Trận đấu**: Hợp nhất luồng hội thoại – `0x14 Surface` – `AutoChain` – `StartBattle` – `Warp` qua `EventRunner`; thực hiện khóa `Two-Phase State Lock` + `registry_update_if_online` khi kết thúc trận đấu hoặc warp tổ đội; vẫn giải quyết hậu sự kiện và ghi DB trực tiếp nếu người chơi mất kết nối giữa trận.
- **Bền vững hóa nguyên tử**: Hợp nhất toàn bộ việc lưu quest/Eve vào 1 SQLite transaction duy nhất, bổ sung migration đổi khóa chính theo `(playerid, scene_id, eventid)` + `savemap` + `character_bit_flags`, và thực hiện write-through ngay tại 3 checkpoint (`FinishChain`, `Battle End`, `Door Warp`).

---

## User Stories

1. Là một Quản trị viên vận hành (Operator), tôi muốn server nạp chính xác toàn bộ bản đồ từ dữ liệu kịch bản `Data/eve.emg` mà không nuốt lỗi hay đọc lệch byte trận chiến, để mọi nhiệm vụ trên tất cả các map hoạt động đầy đủ ngay khi khởi động ở chế độ mặc định.
2. Là một Người chơi, tôi muốn server sử dụng duy nhất `Data/eve.emg` làm nguồn sự thật cho mọi hội thoại và nhiệm vụ (không bị file `.ini` mẫu ghi đè sai bước), để lời thoại, nhánh chọn, chiến đấu và phần thưởng khớp 100% với dữ liệu client PC.
3. Là một Kỹ sư kiểm thử/vận hành (Developer/Operator), tôi muốn các mã điều kiện hoặc kết quả chưa giải mã (`UNKNOWN`) trong kịch bản Eve tự động thất bại an toàn (`false`/bỏ qua kèm log telemetry), không bao giờ rơi vào nhánh `0 == 0 -> true`, để người chơi không thể vô tình vượt qua điều kiện sai hoặc kích hoạt thưởng lỗi.
4. Là một Người chơi, tôi muốn khi mở một nhánh điều kiện thuộc loại chưa hỗ trợ hoặc sai tham số, chuỗi sự kiện đó bị từ chối và không tiến hành trừ đồ hay đổi bước nhiệm vụ.
5. Là một Người chơi, tôi muốn mọi tương tác chạm NPC (`ClickNpc`), nhấn vào cửa (`ClickDoor`), đi qua vùng cửa (`MeetDoor`), và sự kiện bước đi trên map (`WalkStep`) đều kích hoạt đúng kịch bản Eve tương ứng với điều kiện hiện tại của nhân vật.
6. Là một Người chơi đang trò chuyện qua `0x14 Surface` (ví dụ chuỗi khảo nghiệm Giản Ung, Tống Nhân), tôi muốn lựa chọn nút menu (`0x14 09`) chỉ khớp với đúng chuỗi điều kiện chứa cặp `(surface_id, choice_code)` đã chọn, để các chuỗi điều kiện thông thường khác trên cùng NPC không thể chen ngang đè mất lựa chọn của tôi.
7. Là một Người chơi gửi một mã lựa chọn menu (`choice_code`) không tồn tại hoặc không thỏa mãn điều kiện của bất kỳ nhánh nào trên `surface_id` hiện tại, tôi muốn server từ chối thăng tiến chuỗi lựa chọn đó và đóng/giữ an toàn phiên hội thoại.
8. Là một Người chơi có chuỗi hội thoại tự nối tiếp (`AutoChain`, bao gồm cả trường hợp trừ/nhận vật phẩm với `parameter_style == 0` có giá trị âm), tôi muốn server giữ nguyên trạng thái khóa hội thoại (`14 2C Lock`) xuyên suốt chuỗi nối tiếp và chỉ mở khóa (`14 2C Unlock` + `14 08 EndTalk`) đúng một lần khi thực sự kết thúc, để UI client không bị chớp nháy hoặc kẹt cứng.
9. Là một Người chơi gặp tình huống bước đầu tiên của chuỗi tương tác lập tức mở trận chiến (`StartBattle`), tôi muốn server không phát khung khóa thừa hay để lại hộp thoại treo khi vào bàn cờ, và phải dọn sạch UI hội thoại trước khi chuyển cảnh chiến đấu.
10. Là một Người chơi, tôi muốn khi kịch bản Eve kiểm tra điều kiện sủng vật đi theo (`FollowPet`), sủng vật trong kho/xe ngựa, số lượng vai trò, biến đếm sự kiện theo bản đồ, cờ bit hoặc cờ hoàn thành `quest_dont`, hệ thống nạp đầy đủ trạng thái thật của nhân vật vào bộ đánh giá, để các nhiệm vụ yêu cầu dắt tướng (như Sĩ Triết/Lữ Bố) hoặc kiểm tra tiền điều kiện chạy chính xác.
11. Là một Người chơi, tôi muốn khi kịch bản Eve tặng hoặc thu hồi bất kỳ vật phẩm nào thuộc `Item.dat` (bao gồm cả vật phẩm nhiệm vụ `result_class = 1`), vật phẩm đó được thêm/trừ trực tiếp trong túi chính `homdo` (`0x17`, 25 ô) đồng bộ với client, thay vì đẩy nhầm sang kênh `0x18` vốn chỉ dành cho `Mark.dat`.
12. Là một Người chơi nhận chuỗi phần thưởng có chứa vật phẩm nhưng `homdo` (`0x17`) không đủ ô trống (hoặc vượt giới hạn xếp chồng/tiền tệ `GOLD_CAP`/`POINT_CAP`), tôi muốn server kiểm tra sức chứa trước khi thực thi (`Pre-flight All-or-Nothing`) và từ chối toàn bộ bước thực thi đó (không trừ nguyên liệu đầu vào, không đánh dấu bước quest, không cộng thưởng một phần) kèm thông báo phù hợp.
13. Là một Người chơi nhận bước nhiệm vụ mới được ghi vào Nhật ký Nhiệm vụ (`Quest Ledger`, `0x18 01/02/04/06`), tôi muốn server cấp phát vị trí hợp lệ trong khoảng `1..=200` (tuyệt đối không dùng `slot = 0`), để danh sách nhiệm vụ trên giao diện client hiển thị đúng tên từ `Mark.dat` và không ghi đè lẫn nhau.
14. Là một Người chơi đang có đủ 200 ô trong Nhật ký Nhiệm vụ (`0x18`) mà kích hoạt nhận thêm nhiệm vụ mới, tôi muốn server từ chối thao tác nhận nhiệm vụ theo nguyên tắc All-or-Nothing và gửi khung báo đầy `S→C 0x18 03` cho client.
15. Là một Người chơi hoàn thành một nhiệm vụ không lặp lại có định nghĩa trong `Data/Mark.dat` (bao gồm cả các `bit_id` từ `301` đến `2400`), tôi muốn cờ hoàn thành `quest_dont` chỉ được bật đúng lúc nhiệm vụ thực sự hoàn tất (hoặc lệnh đánh dấu tường minh) qua `0x18 05` với `bit_id`, và được kiểm tra chính xác ở lần nói chuyện lại (`ConditionClass=2 pStyle=3`), để nhiệm vụ không bị khóa sớm ngay từ bước trung gian đầu tiên.
16. Là một Người chơi đăng nhập lại sau khi đã hoàn thành nhiều nhiệm vụ phân bố ở các vùng bit khác nhau (`1..=2400`), tôi muốn server đóng gói toàn bộ các bit đã bật thành từng byte `[byte_idx 1..=300: u16][bitmask: u8]` qua `S→C 0x18 07` và gửi đúng trạng thái nhiệm vụ/diễn viên (`0x18 06`, `0x18 08`), để client khôi phục nguyên vẹn dấu nhiệm vụ trên bản đồ.
17. Là một Người chơi hoặc GM gửi gói `C→S 0x18 01/02` (phát sinh từ form debug `TMissionForm` trong client), tôi muốn server từ chối hoàn toàn gói tin này và ghi log cảnh báo mà không làm thay đổi trạng thái nhiệm vụ trong RAM hay DB (GM thao tác qua lệnh chat riêng).
18. Là một Người chơi tương tác với các NPC có hành động nhóm NPC (`NpcTeam` `rc=3`), trao hoặc thu hồi võ tướng/sủng vật (`RewardPet` `rc=8`), di chuyển thực thể (`MoveEntity`), phát hoạt ảnh/phim (`Movie`), thực thi hành động NPC (`NpcAction`), học kỹ năng từ quest (`Skill` `rc=4`), cộng/trừ Kim hoặc Điểm (`Gold/Points` `rc=5`), nhận EXP dẫn tới lên cấp (`Player` `rc=7` với thăng cấp + tăng chỉ số + hồi HP/SP), hoặc lưu điểm hồi sinh `savemap`, tôi muốn tất cả hiệu ứng này hoạt động thật và phát đúng các gói đồng bộ chuẩn (`0x08`, `0x16`, `0x17`, `0x18`, `0x1A`, `0x14`, `0x0C`) cho client.
19. Là một Người chơi đánh thắng, thua hoặc bỏ chạy khỏi một trận chiến nhiệm vụ Eve (chẳng hạn trận Tống Dũng map `12001`), tôi muốn hệ thống nạp đúng đội hình `TeamDef` / `right_enemies` / `DiaHinh`, khôi phục luồng Eve tương ứng (`Win`/`Lose`/`Flee`), thực hiện đầy đủ phần thưởng/phạt và gửi đủ cả khung trực tiếp (`direct_messages`) lẫn broadcast bản đồ (`map_broadcast`).
20. Là một Người chơi tham gia trận chiến nhiệm vụ Eve và bị mất kết nối mạng (disconnect) giữa trận, tôi muốn khi trận chiến kết thúc trên server, hệ thống vẫn chạy đầy đủ hậu sự kiện chiến đấu (`Win`/`Lose`/`Flee`) và lưu trực tiếp trạng thái mới xuống DB, để khi đăng nhập lại tôi nhận đúng tiến độ/hậu quả của trận đó.
21. Là một Người chơi vừa thắng trận chiến nhiệm vụ xong và ngay lập tức gửi gói tin di chuyển hoặc nói chuyện tiếp, tôi muốn dữ liệu phần thưởng và bước nhiệm vụ vừa nhận không bị gói tin kế tiếp ghi đè mất nhờ cập nhật đồng bộ registry (`registry_update_if_online`) dưới khóa `Two-Phase State Lock`.
22. Là một Thành viên tổ đội cùng Đội trưởng thực hiện nhiệm vụ Eve chuẩn, tôi muốn chỉ Đội trưởng là người thăng tiến bước nhiệm vụ và lưu DB cho kịch bản Eve chuẩn (tránh tự động hoàn thành hộ không mong muốn), trong khi vẫn giữ đồng bộ `quest_tasks` thành viên cho các sự kiện map đặc biệt đang dùng (`2K`/`Nhị Kiều`/`40 NPC`) với `slot 1..=200` hợp lệ.
23. Là một Thành viên tổ đội đi cùng Đội trưởng qua cửa dịch chuyển (`Door Warp`), tôi muốn server khóa đồng bộ tất cả các thành viên tham gia theo đúng thứ tự ID tăng dần (`lock_player_operations`), dịch chuyển mọi người sang bản đồ đích và lưu vị trí mới xuống DB cho toàn bộ thành viên, không chỉ riêng Đội trưởng.
24. Là một Người chơi đang thực hiện sự kiện tại hai bản đồ khác nhau tình cờ có cùng chỉ số `eve_no`, tôi muốn số lần hoàn thành sự kiện (`completed_eve_counts`) được tách biệt hoàn toàn theo cặp `(scene_id, eve_no)`, để việc hoàn thành sự kiện ở bản đồ này không làm sai lệch điều kiện ở bản đồ khác.
25. Là một Người chơi vừa hoàn tất một chuỗi hội thoại/nhiệm vụ (`FinishChain`), kết thúc trận chiến (`Battle End`), hoặc đi qua cửa (`Door Warp`), tôi muốn dữ liệu được ghi ngay lập tức xuống SQLite trong một transaction nguyên tử duy nhất (write-through checkpoint), để server bị tắt đột ngột ngay sau đó cũng không làm mất tiến độ vừa đạt được.
26. Là một Quản trị viên vận hành, tôi muốn khi quá trình lưu SQLite gặp lỗi trong một transaction hợp nhất (không tách transaction phụ, không nuốt `Err`), toàn bộ transaction phải rollback sạch sẽ và `auto_save` không được phép đánh dấu nhân vật là đã lưu (`SaveLedger`), để đảm bảo tính nhất quán dữ liệu tuyệt đối.
27. Là một Người chơi có lưu điểm hồi sinh (`savemap`), có cờ bit nhiệm vụ (`character_bit_flags`), và có sủng vật đang cất ở Xe ngựa/Kho/Khách sạn (`PetStorageType` `Cart=2`, `Hotel=3`, `Warehouse=4`), tôi muốn mọi thông tin này được nạp và lưu đầy đủ qua các lần đăng nhập/tắt mở server, và bị xóa sạch không để lại dữ liệu mồ côi khi xóa nhân vật.

---

## Implementation Decisions

Các quyết định kiến trúc và thiết kế triển khai được chốt dựa trên ADR 0009 và `GLOSSARY.md`:

1. **Bộ nạp Nhị phân `Data/eve.emg` & Single Source of Truth**:
   - Chuẩn hóa bộ giải mã phần `FightData` của kịch bản Eve: đọc mỗi `win_cond` đúng `13` byte (`+0x04..+0x16`), loại bỏ hoàn toàn trường `fight_limit` (`0` byte, không đọc `u32`), chỉnh bounds guard bản ghi `EveResult` thành `14` byte, giữ nguyên trường `BKMusic` (`+0x07`, giá trị `1/2/3 -> WC0007/WC0008/WC0010.wav`) và lan truyền lỗi đọc scene thay vì nuốt lỗi âm thầm để nạp đủ toàn bộ `3,901` mục từ / `3,897` scene có dữ liệu. Riêng `62007.eve` kết thúc parse hợp lệ tại `TFight cnt=0` và có `64` byte dư ở cuối (giống `FUN_005efee8` trong `aLogin.exe`), nên parser dừng sạch sau Section 9 chứ không coi byte dư cuối `62007.eve` là lỗi.
   - Bật động cơ Eve mặc định ở production (`EVE_EVENTS_ENABLED = true`), loại bỏ cơ chế ghi đè `.ini` legacy và nhánh fallback cứng cũ.
   - Xây dựng chỉ mục tra cứu nhanh chỉ-đọc theo khóa `(scene_id, eve_no)` cùng chỉ mục theo NPC, Door và Encounter.

2. **Tách Bạch Container `0x17 homdo` & Nhật ký Nhiệm vụ `0x18` (`Mark.dat`)**:
   - Gỡ bỏ hoàn toàn khái niệm `quest_items (0x18)` khỏi mô hình `Session`, logic cộng/trừ vật phẩm, bản ghi đồng bộ lúc đăng nhập (`Logined1`), và thao tác xóa DB tương ứng. Đưa 100% vật phẩm `Item.dat` qua `homdo` (`0x17`, 25 ô).
   - Chuẩn hóa `quest_sync` cho Nhật ký Nhiệm vụ `0x18`:
     - Cấp phát slot `1..=200` cho `quest_tasks` (loại bỏ hoàn toàn mọi vị trí ghi `slot = 0` ở các sự kiện map đặc biệt/đấu trường).
     - Khi nhận quest mới vào `Quest Ledger` mà đã đủ 200 ô, từ chối All-or-Nothing và phát `S→C 0x18 03`.
     - Sửa cơ chế `quest_dont`: `S→C 0x18 05` nhận `bit_id` hợp lệ `1..=2400` (theo `Mark.dat.position`); `S→C 0x18 07` khi đăng nhập phát các byte `[byte_idx (1..=300): u16][bitmask: u8]` gom bit từ tập `bit_id` đã hoàn thành; chỉ đánh dấu `quest_dont` khi nhiệm vụ thực sự hoàn tất hoặc qua lệnh đánh dấu tường minh.
   - Tại lớp điều phối gói tin nhận từ client, từ chối hoàn toàn `C→S 0x18 01/02` cho mọi tài khoản bao gồm cả GM (chỉ ghi log cảnh báo).

3. **Lớp Lập kế hoạch Thuần (`Pure Eve Planner`)**:
   - Tách riêng hoàn toàn khỏi I/O và Tokio lock; nhận vào ngữ cảnh bất biến (`PlayerEventSnapshot`) và một `EveTrigger` có kiểu rõ ràng:

```text
EveTrigger:
  - ClickNpc(place_id)
  - ClickDoor(door_id)
  - MeetDoor(door_id)
  - WalkStep
  - MenuChoice { surface_id, choice_code }
  - BattleEnd { eve_no, fight_eve_no, outcome }
```

   - Thủy hóa đầy đủ `PlayerEventSnapshot`: `homdo` (`0x17`), `follow_npc_ids`, `inn_npc_ids`, `cart_npc_ids`, `role_count_values`, `completed_eve_counts` đánh khóa `(scene_id, eve_no)`, `quest_tasks`, ánh xạ `Mark.dat` `quest_id <-> bit_id` cho `quest_dont`, và `character_bit_flags`.
   - Sửa chuẩn hóa thứ tự đối số `compare_step` trong bộ đánh giá điều kiện; ép fail-closed (`false` + log telemetry) cho mọi `condition_class`, `parameter` hoặc `condition_ops` chưa hỗ trợ (bao gồm loại bỏ `_ => true` trong `compare_step` đối với `condition_ops = 142` ở `55100.eve eve=5 cond=14`, xóa bỏ lỗi fallthrough `actual = 0 -> true`).
   - Bộ giải quyết chuỗi bắt buộc áp dụng `ChainFilter::RequireDialogChoice(surface_id, choice_code)` khi trigger là `MenuChoice`, triệt tiêu việc chuỗi thường đè lên lựa chọn menu. Theo disassembly `aLogin.exe`, `0x14 09` có thể gửi các mã chọn hợp lệ từ callback `TSurface`: `20` (`0x14`, đóng khi không có tùy chọn), `21` (`0x15`, xác nhận/tiếp), `40` (`0x28`, hủy/thoát), `30..39` (`opt 0..9`) và `61, 62, ...` (`opt >= 10`).
   - Bộ dịch kế hoạch ánh xạ danh sách `EveResult` được chọn (qua `GroupData` random/all) thành `Vec<DomainEffect>` tường minh, hỗ trợ các nhánh `NpcTeam`, `RewardPet`, `MoveEntity` (`rt=4`, đã xác minh 100% `210/210` bản ghi đều có `rc == 3` và `>= 5` chữ số thập phân tách 4 chữ số cuối), `PlayMovie` (`rt=5`), `NpcAction` (`rt=9`), `Skill`, `Gold/Points`, `Player` (bao gồm `texp` lên cấp kèm gói `0x14 0x2A 0x01 <exp_u32_LE>` và `savemap`), chỉ cho phép gửi `0x14 01..06` đối với `result_type ∈ 0..=6` mà client xử lý được, và bỏ qua có cảnh báo đối với mã `UNKNOWN` (100% server-only, không có code xử lý trong `aLogin.exe`).

4. **Bộ điều phối Thực thi Hợp nhất (`EventRunner`) & Kiểm tra Trước (`Pre-Flight Probe`)**:
   - Thực hiện Pre-Flight Probe trên toàn bộ `DomainEffect` biến đổi tài nguyên trước khi ghi nhận bất kỳ thay đổi nào vào RAM/DB: đủ chỗ `homdo` (`0x17`), đủ slot Nhật ký Nhiệm vụ (`0x18` `1..=200`), và không vi phạm biên `GOLD_CAP`/`POINT_CAP`. Thất bại bất kỳ điều kiện nào -> từ chối All-or-Nothing.
   - Hợp nhất các nhánh xử lý nói chuyện/tiếp tục/chọn menu/nối chuỗi/hậu chiến vào một vòng lặp trạng thái thống nhất:

```text
Start / Trigger
  -> Pre-Flight Probe & Apply State Mutations (with Lock if Party)
  -> Inspect Terminating/Suspending Effect:
       * AwaitMenuChoice -> Send Lock (once) + Surface -> Wait for choice (no intermediate commit)
       * StartBattle     -> Cleanly Close/Unlock Talk UI first -> DB Checkpoint -> BattleService
       * Warp            -> Sync State + Send 0x0C (Leader & Locked Party) -> DB Checkpoint
       * FinishChain     -> Increment (scene_id, eve_no) -> Try AutoChain (with pstyle=0 fix)
                              - Matched next chain -> Continue loop (Keep Lock!)
                              - No more chain      -> DB Checkpoint -> Unlock + EndTalk (once)
```

   - Bảo toàn và phát đầy đủ `direct_messages` lẫn `map_broadcast` khi nối lại kịch bản Eve sau trận chiến.

5. **Phạm vi Tổ đội, Trận đấu (`BattleService`) & Khóa Hai Pha (`Two-Phase State Lock`)**:
   - Nạp đầy đủ `EveFightData` (`right_enemies`, `fight_type`, `background_no` từ `SceneInfo` / fallback `112`) vào khởi tạo trận chiến Eve. Đối chiếu `aLogin.exe` xác nhận client không đọc file `.ini` hay `[TEAMDEF] Diahinh`, mà nền chiến đấu hoàn toàn do gói tin server quyết định (client chỉ tra `TFight.BKMusic` để phát nhạc và đọc `TWinCondition.wc_type ∈ {3,4,8}` cho UI `[alt+W]`).
   - Nhiệm vụ Eve chuẩn chỉ Đội trưởng cập nhật tiến độ và persist DB; các sự kiện map đặc biệt giữ đồng bộ thành viên nhưng bắt buộc dùng slot hợp lệ `1..=200`.
   - Mọi Door Warp có tổ đội hoặc sự kiện chạm nhiều người chơi phải gọi `lock_player_operations` theo thứ tự ID tăng dần trước khi đọc `registry_get`, cập nhật vị trí và lưu DB cho toàn bộ thành viên tham gia.
   - Trong `BattleService::battle_ended`: khóa `lock_player_operations` cho danh sách ID tham chiến đã sắp xếp; thực thi hiệu ứng thắng/thua/chạy (`resume_eve_after_battle` và các hook chiến đấu khác); gọi `registry_update_if_online` cho từng người chơi còn online; cập nhật handle session mới nhất trong `register_sender`. Nếu người chơi đã offline khi trận kết thúc, vẫn chạy trọn vẹn `resume_eve_after_battle` và ghi kết quả thẳng xuống DB của tài khoản offline đó.

6. **Lược đồ SQLite, Transaction Nguyên tử & Write-Through Checkpoints**:
   - Bổ sung SQLx migration mới (`0002`):
     - Đổi khóa chính `character_completed_events` thành `(playerid, scene_id, eventid)` (bảo toàn hoặc migrate an toàn dữ liệu hiện có).
     - Bổ sung cột lưu `savemap` (`savemap_id`, `savemap_x`, `savemap_y`) trên bảng `characters`.
     - Liên kết đầy đủ `character_bit_flags` vào `Session` và kho lưu trữ phiên (gỡ bỏ repository dead code thừa).
     - Bảo toàn nguyên vẹn `PetStorageType` (`Cart = 2`, `Hotel = 3`, `Warehouse = 4`) khi đọc/ghi sủng vật.
   - Gộp toàn bộ lưu trạng thái Eve/quest vào **cùng 1 transaction SQLite** của hàm lưu phiên (`SqliteSessionRepository::save`); tuyệt đối không nuốt lỗi mà phải trả `Err` để `auto_save` giữ nguyên `SaveLedger` khi thất bại.
   - Sửa tác vụ xóa nhân vật dọn sạch `character_quest_tasks`, `character_quest_dont` và bỏ truy vấn bảng `character_quest_items` lỗi thời; cập nhật `auto_save::fingerprint()` bao phủ toàn bộ trường bền vững (`ghost`, `god`, `bit_flags`, `savemap`, `completed_eve_counts`, `pet.quest`, v.v.) và bỏ băm các mảng tạm không lưu.
   - Kích hoạt write-through DB nguyên tử tại đúng 3 checkpoint: `FinishChain`, `Battle End`, và `Door Warp`.

---

## Testing Decisions

Kiểm thử tuân thủ `CODING_STANDARDS.md` §3 (chỉ đặt test trong thư mục kiểm thử tích hợp, nghiêm cấm `#[cfg(test)]` trong `src/`):

- **Nguyên tắc viết test**: Mọi test chỉ kiểm chứng **hành vi bên ngoài (external behavior)** qua giao thức wire trả về (`HandleOutcome` frames, `direct_messages`, `map_broadcast`), trạng thái hiển thị của `Session`/registry online, và dữ liệu thực tế đọc lại từ SQLite — không test chi tiết cài đặt nội bộ của các struct trung gian. Mỗi User Story bắt buộc có negative test kiểm chứng từ chối nguyên tử (cả phần không biến đổi dữ liệu lẫn thông báo/gói lỗi gửi client). Khi test vật phẩm luôn test số lượng lẻ và assert các slot phụ cận trong `homdo` / `Quest Ledger` còn nguyên vẹn.
- **4 Seam kiểm thử cấp cao đã thống nhất**:
  1. **Seam Giao thức Tổng hợp (`dispatcher::dispatch`)**: Mô phỏng toàn bộ luồng client-server (`0x01`/`0x03` login sync, `0x05`/`0x06` di chuyển & `MeetDoor`, `0x14` `ClickNpc`/`ClickDoor`/Continue/`0x09` MenuChoice/EndTalk, và từ chối `C→S 0x18 01/02`) trên cả dữ liệu giả lập tối giản lẫn `Data/eve.emg` & `Data/Mark.dat` thật (ví dụ chuỗi `12001` Bắt Sơn Tặc Sĩ Triết, `12136` Giản Ung, `12001` Tống Nhân & Tống Dũng, Door Warp tổ đội).
  2. **Seam Trận đấu & Ngắt kết nối (`BattleService::battle_ended`)**: Kiểm chứng hậu quả thắng/thua/chạy trận Eve, đồng bộ `registry_update_if_online` chống ghi đè ngược khi nhận packet tiếp theo, khóa `Two-Phase State Lock` cho tổ đội, và trường hợp người chơi mất kết nối giữa trận nhưng DB vẫn cập nhật kết quả hậu sự kiện để lần đăng nhập kế tiếp giữ nguyên trạng thái.
  3. **Seam Lưu trữ & Migration (`SqliteSessionRepository::load/save`, `characters::delete` trên `common::test_db::setup_test_db()`)**: Kiểm chứng migration `0002`, khóa chính `(playerid, scene_id, eventid)` không va chạm liên bản đồ, round-trip `savemap`/`character_bit_flags`/`PetStorageType`, rollback 1 transaction duy nhất khi lỗi DB (không nuốt lỗi), write-through tại 3 checkpoint, cascade xóa nhân vật, và đồng bộ `auto_save::fingerprint`.
  4. **Seam Nạp dữ liệu (`GameData::load`)**: Xác nhận nạp đầy đủ `3,901` mục từ / `3,897` scene có nội dung từ `Data/eve.emg` và `2,394` bản ghi `Data/Mark.dat` không cắt xén dữ liệu.
- **Kế thừa & dọn dẹp test cũ (`Prior Art`)**:
  - Kế thừa cấu trúc kiểm thử E2E/Parity và Guardrail hiện có trong bộ test của repo (như suite `npc_eve_e2e`, `checkpoint6_parity`, `quest_sync_18`, `architecture_guardrails`).
  - Cập nhật lại các assertion cũ bị sai ngữ nghĩa client trong bộ test `0x18` (loại bỏ kỳ vọng `quest_items 0x18` và bỏ giới hạn cứng `mark <= 300` trên `0x18 05`, thay bằng kiểm chứng `homdo 0x17` cho item, `bit_id 1..=2400` cho `0x18 05`, byte bitmask cho `0x18 07`, `0x18 03` khi tràn 200 ô Quest Ledger, và reject `C→S 0x18 01/02`).
  - Duy trì `architecture_guardrails` test để khóa cứng quy tắc không đặt `#[cfg(test)]` trong `src/` và không sửa `online_sessions` trực tiếp trong handler.

---

## Out of Scope

- **Ngữ nghĩa thực thi phía server của các mã raw `Eve` đánh dấu `UNKNOWN`**: Đối chiếu trực tiếp `aLogin.exe` trong [`EVE_BINARY_ANALYSIS_ALOGIN.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md) đã xác nhận client **không bao giờ đọc/đánh giá `TCondition` hoặc `TResult` từ `Data/eve.emg` tại runtime** (chỉ nạp rồi giải phóng, và chỉ thực thi bản ghi `15B` của `rt ∈ 0..=6` do server gửi qua `S→C 0x14`). Do đó:
  - **Đã làm rõ từ `aLogin.exe` / quét toàn bộ `Data/eve.emg`**:
    - `result_type == 4`: `210/210` bản ghi đều có `rc == 3` và `>= 5` chữ số (`0` biến thể `rc != 3`).
    - `condition_ops == 142`: chỉ có `1` bản ghi lỗi dữ liệu tại `55100.eve, eve=5, cond_no=14` (`cc=9`), không có xử lý đặc biệt trên client (xử lý fail-closed ở `compare_step`).
    - `TWinCondition` (`13B`) & `TFight.BKMusic`: đã giải mã layout 13B (`+0x04..+0x16`), hành vi `[alt+W]` UI cho `wc_type ∈ {3, 4, 8}`, `BKMusic ∈ {1, 2, 3}`, và xác nhận `aLogin.exe` không đọc `[TEAMDEF] Diahinh` hay bất kỳ `.ini` chiến đấu nào.
    - Xác nhận nhóm mã còn lại là **100% server-only** và đã thống kê đầy đủ mẫu/phân bố trong `Data/eve.emg`.
  - **Vẫn giữ Out of Scope** (chờ nguồn server tham chiếu hoặc packet capture mới triển khai ngữ nghĩa game): `condition_class ∈ {3,4,5,11,13,15,16,17}`, biến thể `p_style` chưa ánh xạ của `cc ∈ {2,9,12}`, `result_type ∈ {7,8,10}`, `result_class ∈ {0 (ngoài rt=5/no-op),5,6,9..22,80}`, `fight_type`, `link_to_result`, và logic thực thi trận chiến cho `fight_events`/`win_conds` (`wc_type ∈ {1,3,4,8,9,20}`). Toàn bộ nhóm này chạy fail-closed theo [`UNKNOWN_EVE_CODES_RESEARCH.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/UNKNOWN_EVE_CODES_RESEARCH.md).
- **Các nhánh giao diện client ít gặp trong `aLogin.exe` (ĐÃ giải mã trong [`EVE_BINARY_ANALYSIS_ALOGIN.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md#L159-L206))**:
  - Đã phục hồi đủ body ASM/C cho: `rt == 3` (`CL=0x01` gửi `C→S [0x0B, 0x01, [TFightForm1+0x130]]`), 4 callback `TSurface` (`0x14 09` gửi `20/21/40` và `30..39` / `61+`), `0x14 0x2A` (`sub=1` ExpUP toast `[u32_LE]`, `sub=2/3` đại tỷ thí), `0x14 0x2C` (`Waiteven` sprite), `0x14 0x32` (Đoan Ngọ/Nguyên Tiêu), `0x14 0x33` (Ba Đậu Yêu/Tứ Hải), `0x14 0x38 sel=2` (timer 300s + teleport `RP[2]`), và `0x18 08 action == 2` (`65x65` sprite). *(Các phần liên quan trực tiếp đến luồng Eve như mã chọn `0x14 09`, `0x14 0x2A sub=1`, `0x0B 01` đã được đưa vào Implementation Decisions & issues; riêng việc triển khai gameplay các sự kiện mùa/minigame `0x14 0x2A sub=2/3`, `0x32`, `0x33`, `0x38 sel=2`, `0x18 08 action==2` vẫn giữ ngoài phạm vi đợt này.)* *(Lưu ý: `0x14 0x0A` và `0x1A` đã được đặc tả tại `opcode_14.md` và `opcode_1a.md`; riêng `0x1A` chỉ có chiều S→C `OP_MONEY_SYNC` cho người chơi thực tế, không phải kênh PC talk.)*
- Thiết kế lại cơ chế Vấn đáp Bắc Tinh Quân (`0x10 Quiz`) — ngoài việc giữ ranh giới rõ ràng với `0x14 Surface` trong `GLOSSARY.md`.
- Tối ưu sâu hơn ngoài phạm vi write-through 3 checkpoint đã chốt (ví dụ thay đổi kiến trúc Dual-Pool SQLite).

---

## Further Notes

- Tài liệu tham chiếu chuẩn đi kèm với spec này:
  - [`ADR 0009`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md) — toàn bộ quyết định kiến trúc đã được Owner xác nhận.
  - [`GLOSSARY.md`](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md#L390-L414) — thuật ngữ miền chuẩn hóa cho `Kịch bản Eve`, `Nhật ký Nhiệm vụ (0x18) & Mark.dat`, và `0x14 Surface vs 0x10 Quiz`.
  - [`RESEARCH_AND_GAP_ANALYSIS.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/RESEARCH_AND_GAP_ANALYSIS.md) — bảng kiểm kê định lượng đầy đủ `Data/eve.emg`, so sánh `sample_data/*.ini`, phân tích Ghidra `aLogin.exe`, và danh sách cụ thể các mã `UNKNOWN`.
  - [`EVE_BINARY_ANALYSIS_ALOGIN.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md) — kết quả disassemble trực tiếp `aLogin.exe` & quét toàn bộ `Data/eve.emg` xác minh phân giới server-only vs client, `62007.eve`, `rt=4`, `condition_ops=142`, `TWinCondition`/`BKMusic`, và giải mã toàn bộ 4 nhóm helper/callback client trong §3 `UNKNOWN_EVE_CODES_RESEARCH.md`.
- Bước tiếp theo sau khi spec này được duyệt: chia nhỏ thành từng implementation issue độc lập (`01-<slug>.md`, `02-<slug>.md`, ...) tại `.scratch/quest-refactor/issues/` theo quy chuẩn Issue Tracker của dự án trước khi tiến hành viết code và test.
