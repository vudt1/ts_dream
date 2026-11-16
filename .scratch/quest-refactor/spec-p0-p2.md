Status: ready-for-agent

# Spec: Khắc phục Toàn diện Lỗi Động cơ Kịch bản Eve (P0–P2), Warp `SceneInfo`, Địa hình Chiến đấu `Ground.MMG` & Hiệu năng/Đồng thời

> **Trạng thái**: Đã chốt toàn bộ quyết định thiết kế qua `/grill-with-docs` (Vòng 1 & Vòng 2) và đồng bộ với [`GLOSSARY.md`](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md).

---

## Problem Statement

Sau khi hoàn thành nền tảng hạ tầng và lưu trữ Eve (`Tickets 01–10`), kiểm chứng trực tiếp trên mã nguồn Rust, client PC `aLogin.exe`, `Data/eve.emg` và `Data/ground.mmg` cho thấy động cơ nhiệm vụ và thế giới vẫn còn các lỗ hổng nghiêm trọng từ **P0 đến P2** khiến hàng nghìn nhiệm vụ hợp lệ không thể hoàn thành, dịch chuyển sự kiện bị bỏ qua gần một nửa, nền chiến đấu sai lệch và hot-path di chuyển chịu áp lực cấp phát lớn:

1. **[P0] Chặn sai chuỗi nhiệm vụ đa giai đoạn & NPC nhiều nhiệm vụ (`should_skip_event`)**: Quy tắc 1 chặn vĩnh viễn mọi `eve_no` đã có lượt hoàn thành tạo biến đổi trạng thái nhưng không gắn điều kiện `condition_class == 12` (làm gãy trên `1,500` nhiệm vụ nhiều bước quay lại cùng `eve_no` rẽ nhánh theo `cc=2`); Quy tắc 2 coi mảng `events` của NPC như chuỗi tuyến tính một chiều nên khóa ngược các nhiệm vụ đứng trước khi người chơi làm xong một nhiệm vụ độc lập đứng sau (`169` NPC có nhiều nhiệm vụ độc lập).
2. **[P0] Mất `47.0%` cổng dịch chuyển sự kiện (`rt=2`) & sai lệch tọa độ**: Bộ nạp nhị phân `eve.emg` bỏ qua trường `u16` (`dest_map_id`) của `EveSceneInfo`, buộc runtime phải tra cứu bảng tĩnh legacy `Warps.txt` (thường ghi nhầm là `"Warps.dat"`), làm `4,857 / 10,336` lệnh warp `rt=2` bị bỏ qua âm thầm và gây lệch bản đồ đích hoặc tọa độ tại hàng trăm khóa trùng số giữa `DoorID` và `SceneInfo.eve_no`.
3. **[P1] Sai lệch `DiaHinh` trận đấu, tràn byte `BKMusic` & gánh nặng file `ground.mmg` (`26.7 MB`)**:
   - Runtime đang gán `SceneInfo[1].background_no` làm `DiaHinh` cho gói tin mở bàn cờ `0x0B 0xFA`, trong khi `8,882 / 8,882` bản ghi `SceneInfo.background_no` đều vượt quá `255` (mang mã Map ID). Client PC `aLogin.exe` đọc byte thấp của `DiaHinh` làm tên nền `"FField<N>"` và byte cao làm chỉ số tra `BKMusic`, dẫn tới hiển thị sai nền trận chiến và sai nhạc nền.
   - Trong khi đó, `Data/ground.mmg` chứa sẵn `geolBaseAtt` đúng dải `FField` (`109`, `112`, `129`, ... phủ `3,852 / 3,901` scene) lại chưa được đấu nối vào `GameData`. Mặt khác, `Data/ground.mmg` nặng tới `26.7 MB` (chứa dữ liệu ảnh/block của client `aLogin.exe`, vốn đã có sẵn ở client) và `GroundMmgLoader` hiện tại đang clone toàn bộ buffer cho từng map (`4,528` lần), gây tốn bộ nhớ và phình dung lượng repo/build package không cần thiết cho server.
4. **[P1] Lỗ hổng ngữ nghĩa động cơ kịch bản**:
   - Bỏ qua 4 cờ kích hoạt `when_happen` khiến `ClickDoor` và `MeetDoor` dùng chung danh sách sự kiện dễ kích hoạt chéo (trong khi cần tôn trọng dữ liệu thực tế: `3,122` sự kiện NPC chỉ bật `on_ser_stroke` và `1,835` sự kiện Encounter chỉ bật `on_click_obj`).
   - Bỏ qua `GroupData.use_mode == 2` (`9` nhóm có `pick_member > 0` hoặc `total_weight > 0` bị bốc đơn lẻ thay vì giữ toàn bộ).
   - `EveResolver` làm rơi toàn bộ `results` ở các mắt xích sau đầu chuỗi AND (`29` node thuộc `15` chuỗi AND).
   - `mark_dont_on_step1` bật quá sớm cờ `quest_dont = 1` ngay ở bước tăng `+1` cho `7` nhiệm vụ nhiều bước có `Mark.dat position > 0` (`10012, 11443, 11863, 12325, 12358, 12377, 12406`) và không bao giờ hạ cờ khi hủy/lùi bước.
5. **[P1] Nút thắt hiệu năng hot-path & vi phạm Lock Ordering**:
   - Mỗi gói tin di chuyển hợp lệ (`0x06 01` / `0x05 01`) dựng lại toàn bộ bảng `quest_marks` (`2,394` phần tử) trước khi kiểm tra xem map hiện tại có vùng `WalkStep` encounter hay không (chỉ `20` trên `3,897` bản đồ có non-battle `WalkStep` encounter); ngoài ra khi `try_trigger_random_encounter` (chạy trước `WalkStep`) trượt chọn quái sau bước snapshot, cùng một gói tin di chuyển có thể cấp phát lại snapshot lần thứ hai.
   - Các wrapper đồng bộ (`run_sync_future`) dựng OS thread và Tokio runtime riêng dù production đã chạy async; một số luồng tổ đội ngoài `EventRunner` (sự kiện NPC đặc biệt `15312`, `15313`, `61210`, trả lời đố Bắc Đẩu `0x10`, đồng bộ tọa độ di chuyển và bước `40 NPC`) khóa thẳng registry online mà không qua `Two-Phase State Lock` (`lock_player_operations`) và bỏ sót lưu SQLite của thành viên.
6. **[P2] Dư thừa cấp phát, sai lệch tài liệu & trạng thái DB vận hành**:
   - Vòng lặp thực thi sự kiện clone và chạy lại bộ lập kế hoạch trên phần kết quả còn lại ở mỗi bước thoại/hành động; bộ chỉ mục scene chuyển đổi qua lại `Vec<u8> -> Vec<u16> -> Vec<u8>`.
   - Tài liệu bàn giao `HANDOFF_01_10` còn thông tin cũ so với codebase hiện tại, và tệp SQLite mặc định `DB/ts_dream.db` hiện đang là tệp rỗng `0` bảng chưa được áp dụng các migration nền tảng.

---

## Solution

Khắc phục triệt để các lỗi **P0 → P2** theo các quyết định đã chốt (không tạo/sửa ADR, giữ `cc=7 param=2` tương thích `reborn_count` hiện tại, cập nhật `GLOSSARY.md`):

1. **Gỡ bỏ hoàn toàn `should_skip_event` (`P0`)**: Loại bỏ Quy tắc 1 và Quy tắc 2 khỏi cả đường kích hoạt trực tiếp (`NpcEventHandler`) lẫn luồng `try_auto_chain`, giao toàn quyền điều hướng nhánh cho `EveCondition` (`cc=2`, `cc=12`, ...) + cơ chế ưu tiên của `EveResolver` + 4 guard chống lặp của `AutoChain`.
2. **Dịch chuyển bằng `EveSceneInfo` từ `eve.emg` (`P0`)**: Giải mã `dest_map_id` (`u16`) trong `EveSceneInfo` và sử dụng `(dest_map_id, pixel_x = player_appear_x * 20 - 10, pixel_y = player_appear_y * 20 - 10)` làm nguồn chính cho mọi warp `rt=2` và hoãn warp hậu chiến đấu, giữ `Warps.txt` làm fallback khi thiếu khóa `SceneInfo`.
3. **Chuẩn hóa `DiaHinh` qua `ground_server.mmg` rút gọn (`P1`)**:
   - Bóc tách dữ liệu từ `ground.mmg` gốc thành tệp rút gọn chuyên cho server **`ground_server.mmg`** (~`217 KB` cho `4,528` map, giữ đúng định dạng `MapData.New` với `width`, `height`, `geolBaseAtt` và lược bỏ `pics`, `blocks`, `waves`, `elements`), sau đó xóa `Data/ground.mmg` (`26.7 MB`) khỏi repo để tinh gọn kích thước repo và bản build.
   - Sửa lỗi clone buffer trong `GroundMmgLoader` (đọc trực tiếp trên slice, không `to_vec()` theo từng map), mở rộng trích xuất kèm `width`/`height` bên cạnh `geolBaseAtt`, và đấu nối vào `GameData` (ưu tiên `ground_server.mmg`, fallback `ground.mmg` nếu có) làm nguồn `DiaHinh` (`geolBaseAtt`, fallback `112`) cho các trận Eve và ngộ địch ngẫu nhiên, bỏ hẳn việc đọc `SceneInfo[1].background_no` và đồng bộ `GLOSSARY.md`.
4. **Hoàn thiện ngữ nghĩa Engine (`P1`)**:
   - Chỉ lọc chặt `when_happen` để tách `ClickDoor` (`on_click_obj`) và `MeetDoor` (`on_stroke_obj || on_into_area`); cho phép `ClickNpc` nhận `[0] || [1] || [3]` (`on_ser_stroke`) và không lọc cứng `WalkStep` Encounter theo `when_happen`.
   - Rẽ nhánh `KEEP_ALL` cho `GroupData.use_mode == 2` (giữ `use_mode == 1` và `use_mode == 3` đi chung đường `pick_member` / weighted random hiện tại).
   - Thu thập kết quả từ tất cả các mắt xích `chain[0..]` trong `EveResolver::resolve`.
   - Bỏ hoàn toàn `mark_dont_on_step1` (`p_style=1` chỉ cập nhật `Quest Ledger`), chỉ ghi `quest_dont` qua lệnh tường minh `MarkDont` (`p_style=5`) hoặc `ExplicitMark` (`p_style=3` với `result_value > 0`).
   - Giữ nguyên `cc=7, param=2` không kiểm tra `p_style` (tiếp tục coi như kiểm tra `reborn_count` cho `p_style=4`).
5. **Tối ưu Hot-Path di chuyển, Async Runtime & Đồng thời Tổ đội (`P1`)**:
   - Kiểm tra nhanh tồn tại và vùng tọa độ `encounters` của bản đồ **trước** khi dựng `PlayerEventState`; bỏ cấp phát bản sao `2,394` mục `quest_marks` ở mỗi bước chân bằng tham chiếu bất biến từ `GameData`; tránh dựng snapshot hai lần khi `try_trigger_random_encounter` không tạo trận.
   - Chuẩn hóa `Two-Phase State Lock` (`lock_player_operations`, `registry_update_if_online`, persist đủ session thành viên) cho các sự kiện đặc biệt (`15312`, `15313`, `61210`), trả lời đố Bắc Đẩu (`0x10`), đồng bộ di chuyển và bước `40 NPC`.
   - Giữ `*_with_pool` async cho production; cô lập/thu hẹp `run_sync_future` chỉ cho ngữ cảnh kiểm thử.
6. **Dọn tối ưu P2 & Kiểm tra Schema SQLite (`P2`)**:
   - Plan `DomainEffect` một lần khi khởi tạo/thay đổi `EventSession` thay vì clone và re-plan phần dư mỗi bước; thống nhất kiểu chỉ mục `eve_no` không cấp phát vector tạm.
   - Cập nhật tài liệu bàn giao `HANDOFF_01_10` và bổ sung kiểm tra schema lúc khởi động server (vẫn giữ migrate thủ công theo ADR 0004) kèm hướng dẫn áp dụng `0001_init.sql` + `0002_eve_quest_persistence.sql` lên `DB/ts_dream.db`.

---

## User Stories

1. Là một Người chơi làm nhiệm vụ nhiều bước tại cùng một NPC (cùng một `eve_no` rẽ nhánh theo bước `cc=2` mà không có `cc=12`), tôi muốn có thể quay lại nói chuyện để nộp vật phẩm hoặc nhận bước tiếp theo mà không bị server bỏ qua sự kiện, để hoàn thành trọn vẹn hơn `1,500` chuỗi nhiệm vụ đa giai đoạn.
2. Là một Người chơi nhận và hoàn thành một nhiệm vụ nằm ở cuối danh sách sự kiện của một NPC có nhiều nhiệm vụ độc lập, tôi muốn vẫn kích hoạt được các nhiệm vụ khác đứng trước trên cùng NPC đó, để không bị khóa vĩnh viễn các nhiệm vụ còn lại.
3. Là một Người chơi có sự kiện tự nối tiếp (`AutoChain`) ngay sau khi nhận bước 1 nhiệm vụ (`battle_result == 0`), tôi muốn vòng nối tiếp không bị quy tắc bỏ qua chặn nhầm chính `eve_no` hoặc các nhánh kế tiếp hợp lệ.
4. Là một Người chơi thực hiện hành động tại cửa/NPC kích hoạt lệnh chuyển bản đồ `rt=2` (bao gồm cả trường hợp hoãn chuyển cảnh sau trận chiến), tôi muốn server đọc đúng bản đồ đích và tọa độ xuất hiện (`pixel = tile * 20 - 10`) từ `EveSceneInfo` của `eve.emg`, để toàn bộ `10,336` điểm dịch chuyển nhiệm vụ hoạt động chính xác thay vì bị bỏ qua `47.0%` như khi phụ thuộc `Warps.txt`.
5. Là một Người chơi kích hoạt một lệnh chuyển cảnh `rt=2` mà bản đồ hiện tại tình cờ có cửa `DoorID` trùng số với `SceneInfo.eve_no` trong `Warps.txt`, tôi muốn được đưa tới đúng bản đồ và tọa độ quy định bởi `EveSceneInfo`, để không bị dịch chuyển nhầm sang địa điểm khác.
6. Là một Nhà vận hành / Kỹ sư tích hợp, tôi muốn `Warps.txt` chỉ đóng vai trò fallback khi thiếu bản ghi `EveSceneInfo` (và log thống nhất đúng tên `Warps.txt` thay vì `"Warps.dat"`), để hành vi dịch chuyển thông thường và sự kiện hoàn toàn minh bạch.
7. Là một Người chơi bước vào trận chiến kích hoạt từ Eve hoặc chạm quái ngẫu nhiên trên Client PC `aLogin.exe`, tôi muốn nền chiến đấu (`FField`) và nhạc nền (`BKMusic`) hiển thị đúng địa chất bản đồ `geolBaseAtt` từ `ground_server.mmg` (fallback `112`), thay vì bị lỗi nền/nhạc do nhét Map ID `> 255` của `SceneInfo.background_no` vào `DiaHinh`.
8. Là một Người chơi thực hiện nhiệm vụ có điều kiện `cc=7, param=2, p_style=4`, tôi muốn điều kiện vẫn tiếp tục đánh giá theo `reborn_count` như hiện tại để không làm gián đoạn `58` nhánh kịch bản đang hoạt động cho tới khi có đối chiếu khác.
9. Là một Người chơi đi ngang qua vùng cửa (`MeetDoor`) hoặc bấm trực tiếp vào cửa (`ClickDoor`), tôi muốn chỉ những sự kiện có cờ `when_happen` phù hợp với kiểu kích hoạt cửa đó mới chạy, trong khi tương tác NPC (`on_ser_stroke`) và vùng chạm `WalkStep` Encounter vẫn kích hoạt bình thường không bị lọc nhầm.
10. Là một Người chơi kích hoạt sự kiện gọi nhóm kết quả có `GroupData.use_mode == 2`, tôi muốn toàn bộ các kết quả trong nhóm được thực thi đầy đủ thay vì chỉ bốc ngẫu nhiên 1 phần tử (còn `use_mode == 1` và `use_mode == 3` vẫn giữ nguyên cách chọn hiện tại).
11. Là một Người chơi kích hoạt chuỗi điều kiện AND có kết quả gắn tại các điều kiện phía sau (`chain[1..]`), tôi muốn toàn bộ phần thưởng hoặc hiệu ứng ở cả điều kiện đầu lẫn các điều kiện sau đều được thực thi, để không bị mất vật phẩm, hội thoại hoặc bước nhiệm vụ tại `15` chuỗi AND này.
12. Là một Người chơi thực hiện các nhiệm vụ nhiều bước có `Mark.dat position > 0` (như `10012`, `11443`, `11863`, `12325`, `12358`, `12377`, `12406`), tôi muốn cờ hoàn thành `quest_dont` không bị đánh dấu sớm ngay ở bước nhận đầu tiên, để tôi vẫn tiếp tục được các bước sau và chỉ đánh dấu khóa vĩnh viễn khi gặp lệnh hoàn tất tường minh (`MarkDont` hoặc `ExplicitMark`).
13. Là một Người chơi di chuyển liên tục trên bản đồ đông người (`0x06 01` / `0x05 01`), tôi muốn server bỏ qua việc dựng snapshot sự kiện trên các bản đồ không có vùng `WalkStep` encounter (hoặc khi tọa độ ngoài mọi vùng encounter) và không cấp phát lại bảng `2,394` `quest_marks` mỗi bước chân, để độ trễ di chuyển luôn ổn định và server chịu tải CCU cao mượt mà.
14. Là một Người chơi kích hoạt bước kiểm tra chạm quái ngẫu nhiên trong cùng gói tin di chuyển nhưng không tạo trận đấu, tôi muốn server tránh dựng lại snapshot trạng thái sự kiện trùng lặp một cách lãng phí.
15. Là một Thành viên trong tổ đội tham gia các sự kiện đặc biệt (`15312`, `15313`, `61210`), trả lời câu hỏi Bắc Đẩu (`0x10`) hoặc chiến đấu liên hoàn `40 NPC` (`quest_tasks[218]`), tôi muốn mọi thay đổi vị trí, trạng thái trận đấu và bước nhiệm vụ của tôi và đồng đội được bảo vệ bởi khóa theo thứ tự ID và lưu bền vững xuống SQLite, để tránh race condition và tránh mất tiến độ khi khởi động lại server.
16. Là một Kỹ sư duy trì codebase, tôi muốn vòng lặp chạy sự kiện chỉ lập kế hoạch hiệu ứng một lần cho phiên sự kiện (không clone và plan lại phần dư qua từng câu thoại), các chỉ mục sự kiện không ép kiểu `u8 <-> u16` cấp phát vector tạm ở mỗi tương tác, và wrapper đồng bộ `run_sync_future` được cô lập chỉ phục vụ kiểm thử.
17. Là một Quản trị viên vận hành server, tôi muốn tài liệu bàn giao phản ánh chính xác mã nguồn hiện tại và server kiểm tra schema SQLite lúc khởi động (cảnh báo/dừng rõ ràng nếu `DB/ts_dream.db` thiếu bảng cốt lõi từ migration `0001`/`0002`, trong khi vẫn giữ thao tác migrate thủ công theo ADR 0004).
18. Là một Quản trị viên vận hành & Kỹ sư build, tôi muốn dữ liệu địa chất/kích thước bản đồ được bóc tách sang tệp `ground_server.mmg` siêu nhẹ (~`217 KB`, giữ `width`, `height` và `geolBaseAtt`), xóa tệp `ground.mmg` `26.7 MB` của client khỏi repo và sửa lỗi clone buffer khi nạp, để repo/build gọn nhẹ và server khởi động tức thì.

---

## Implementation Decisions

### Nhóm P0 — Điều phối Sự kiện & Dịch chuyển `EveSceneInfo`

- **`EveAutoChainEngine` & `NpcEventHandler`**:
  - Gỡ bỏ hoàn toàn Quy tắc 1 và Quy tắc 2 của `should_skip_event` khỏi cả hai điểm gọi kích hoạt trực tiếp (`WalkStep` và `ClickNpc`/`ClickDoor`/`MeetDoor`, kể cả khi có `chain_filter`) lẫn điểm gọi nội bộ bên trong `try_auto_chain` (khi `battle_result == 0`).
  - Điều hướng nhánh hợp lệ dựa hoàn toàn vào `EveCondition` (`cc=2`, `cc=12`, ...) và thứ tự ưu tiên của `EveResolver`, kết hợp 4 lớp chống lặp (`SameCondition`, `ReQuestion`, `ReBattle`, `DuplicateItem`) của `EveAutoChainEngine`.
- **`EveDataLoader` (`EveSceneInfo`) & Hệ thống Warp (`EventRunner`, `QuestHandler`, `BattleService`)**:
  - Mở rộng `EveSceneInfo` để đọc và lưu trường `dest_map_id` (`u16`) thay cho thao tác bỏ qua 2 byte trước `player_appear_x`.
  - Tại các điểm xử lý `DomainEffect::Warp` và hoãn warp hậu chiến đấu (`EventRunner`, `QuestHandler`, `BattleService`), tra cứu trước tiên trong `scene_infos` của bản đồ hiện tại theo `warp_id == EveSceneInfo.eve_no`: nếu có `dest_map_id > 0` và `player_appear_x > 0, player_appear_y > 0`, dịch chuyển tới `dest_map_id` cùng tọa độ pixel quy đổi `(player_appear_x * 20 - 10, player_appear_y * 20 - 10)`; chỉ fallback sang bảng `Warps.txt` khi `scene_infos` không có khóa tương ứng.
  - Thống nhất toàn bộ thông điệp log và tài liệu chú thích nhắc `"Warps.dat"` sang đúng tên nguồn thực tế `"Warps.txt"`.

### Nhóm P1 — `DiaHinh`, `ground_server.mmg` & Ngữ nghĩa Engine

- **Bóc tách `ground_server.mmg` & Sửa `GroundMmgLoader`**:
  - Tạo công cụ trích xuất (CLI/example) đọc `ground.mmg` gốc của client để xuất ra tệp **`ground_server.mmg`** tuân thủ đúng định dạng `MapData.New` (index đuôi `29B/mục + count(u16)`): rút gọn `mapPicCount = 0`, `blockWidth = 0`, `blockHeight = 0`, `waveCount = 0`, `elementCount = 0`, `npcCount = 0` và giữ nguyên `width` (`i32`), `height` (`i32`), `geolBaseAtt` (`u8`) cho mỗi bản đồ (~`217 KB` cho `4,528` map).
  - Sau khi sinh và kiểm chứng `ground_server.mmg`, xóa tệp `ground.mmg` (`26.7 MB`) khỏi thư mục `Data` của repo (vì client đã có sẵn file gốc).
  - Sửa `GroundMmgLoader` đọc trực tiếp trên slice `&[u8]` (loại bỏ hoàn toàn thao tác clone `to_vec()` trong vòng lặp `4,528` map) và mở rộng API để cung cấp thêm metadata `(width, height, geolBaseAtt)` bên cạnh ánh xạ `map_id -> geolBaseAtt`.
- **Nguồn `DiaHinh` trong `GameData` & Trận chiến**:
  - Đấu nối `GroundMmgLoader` vào `GameData` khi nạp dữ liệu khởi động: ưu tiên nạp `ground_server.mmg` (fallback sang `ground.mmg` nếu `ground_server.mmg` không tồn tại).
  - Khi mở trận chiến qua `EventRunner` hoặc `MovementHandler` (random encounter / Bắc Đẩu), bỏ đọc `scene_infos.get(&1).background_no`; thay bằng tra `geolBaseAtt` của bản đồ hiện tại trong `GameData` (fallback về `112` khi thiếu bản ghi), bảo đảm `DiaHinh` trên gói `0x0B 0xFA` mang đúng mã `"FField<N>"` ở byte thấp và byte cao bằng `0` (không làm tràn sang `BKMusic`).
- **Chính sách `Evaluator` (`cc=7, param=2`)**:
  - Giữ nguyên logic đánh giá `reborn_count` cho `cc=7, param=2` không lọc `p_style` theo quyết định chốt của phiên thiết kế (tương thích ngược với `58` nhánh `p_style=4` hiện có).
- **Lọc Trigger theo `when_happen`**:
  - Áp dụng lọc `when_happen` riêng cho nhánh cửa (`Door`): `ClickDoor` yêu cầu `on_click_obj` (`when_happen[0]`); `MeetDoor` yêu cầu `on_stroke_obj || on_into_area` (`when_happen[1] || when_happen[2]`).
  - Với `ClickNpc`, chấp nhận `on_click_obj || on_stroke_obj || on_ser_stroke` (hoặc fallback khi cả 4 cờ đều `false`) để tương thích `3,122` sự kiện NPC `on_ser_stroke`. Với `WalkStep` Encounter, không lọc theo `when_happen` vì vùng chạm đã cô lập theo `scene.encounters` và `1,835` encounter mang cờ `on_click_obj`.
- **Hoàn thiện `GroupData.use_mode` & `EveResolver`**:
  - Trong `GroupData`, kiểm tra `use_mode == 2` trước khi xét `pick_member` hay trọng số ngẫu nhiên để trả về `KEEP_ALL` (giữ toàn bộ phần tử nhóm); giữ `use_mode == 1` và `use_mode == 3` đi chung đường `pick_member` / weighted random hiện tại.
  - Trong `EveResolver::resolve`, thu thập kết quả từ tất cả các mắt xích điều kiện trong chuỗi thắng cuộc (`winner.chain[0..]` theo đúng thứ tự chuỗi) thay vì chỉ lấy `winner.chain[0].results`, đồng thời bảo đảm tổng kết quả của chuỗi khác rỗng và cập nhật doc comment 3 tiêu chí + tie-break thứ tự khai báo.
- **Điều kiện đánh dấu `quest_dont` (`EventRunner`)**:
  - Xóa bỏ nhánh tự động đặt `quest_dont = 1` theo `mark_dont_on_step1 && step == 1` trong `EventRunner` (`p_style=1` chỉ tăng/giảm `Quest Ledger`). Chỉ gọi `set_quest_dont(..., 1)` khi xử lý `QuestOp::MarkDont` (`p_style=5`) hoặc `QuestOp::ExplicitMark` (`p_style=3` với `result_value > 0`).

### Nhóm P1 — Tối ưu Hot-Path, Runtime Async & Đồng thời Tổ đội

- **Tối ưu Hot-Path Di chuyển (`MovementHandler` & `NpcEventHandler`)**:
  - Trước khi gọi `snapshot_state_with_data` cho trigger `WalkStep`, kiểm tra nhanh `scene.encounters` có chứa `(px, py)` không; nếu không trúng vùng nào, bỏ qua hoàn toàn bước snapshot và giải quyết Eve.
  - Đổi thiết kế `EveStateBuilder` / `PlayerEventState` để chia sẻ tham chiếu bất biến (hoặc `Arc`) tới `GameData.quest_marks`, loại bỏ vòng lặp dựng `HashMap` mới `2,394` phần tử ở mỗi bước di chuyển hoặc click NPC.
  - Rà soát phối hợp giữa `try_trigger_random_encounter` và `WalkStep` để chỉ dựng snapshot khi thực sự cần kiểm tra điều kiện trận và tránh cấp phát lặp lại hai lần trong cùng gói tin di chuyển.
- **Chuẩn hóa `Two-Phase State Lock` & Persist Tổ đội**:
  - Áp dụng mẫu khóa chuẩn (`lock_player_operations` với danh sách ID thành viên đã sắp xếp tăng dần trước khi chạm `online_sessions`, kèm `registry_update_if_online` và truyền toàn bộ session bị thay đổi vào `persist_sessions_transaction`) cho các nhánh tổ đội của NPC sự kiện đặc biệt (`15312`, `15313`, `61210`), trả lời đố Bắc Đẩu (`0x10`), đồng bộ vị trí thành viên khi di chuyển (`MovementHandler`), và cập nhật bước nhiệm vụ `40 NPC` (`BattleConstruction`).
- **Tách rạch ròi Async Execution vs Test Sync Helper (`EventRunner`)**:
  - Giữ toàn bộ luồng production chạy async qua `*_with_pool(...).await`; thu hẹp và đánh dấu/cô lập `run_sync_future` cùng các sync wrapper chỉ dành cho test harness.

### Nhóm P2 — Tinh gọn Cấu trúc Dữ liệu, Tài liệu & Database

- **Loại bỏ Re-Plan lặp trong `EventRunner`**:
  - Lưu sẵn danh sách `Vec<DomainEffect>` đã được `plan_results` biên dịch một lần khi mở hoặc cập nhật `EventSession`, để `execute_event_step_core` đọc tuần tự theo `current_index` thay vì clone phần dư `EveResult` và re-plan ở mỗi bước.
- **Thống nhất kiểu chỉ mục `eve_no` (`SceneEveData` & `NpcEventHandler`)**:
  - Sử dụng nhất quán một kiểu chỉ mục (hoặc slice trực tiếp) giữa `SceneEveData` và `NpcEventHandler` để bỏ cấp phát `Vec<u8>` tạm ở mỗi tương tác.
- **Đồng bộ Tài liệu & Kiểm tra Schema Database**:
  - Cập nhật `HANDOFF_01_10` phản ánh đúng trạng thái hiện tại (`rc=5` đã hỗ trợ, `0x46 BoatRace`/`0x41 Rank` không nằm trong `unimplemented`, `Warps.txt`, `DomainEffect::Unsupported`, `EveCondition.to_result`, cùng đúng số liệu `15` chuỗi AND / `223` scene event `cc=12 param != 0` / `rt=9 p_style ∈ {1,2}`).
  - Giữ cơ chế migrate thủ công theo ADR 0004 nhưng bổ sung bước kiểm tra schema lúc khởi động server (xác nhận tồn tại các bảng/cột cốt lõi của `0001_init.sql` và `0002_eve_quest_persistence.sql`, báo lỗi rõ ràng kèm hướng dẫn migrate nếu gặp DB trống như `DB/ts_dream.db` hiện tại).

---

## Testing Decisions

Các bài kiểm thử chỉ kiểm tra **hành vi bên ngoài và hợp đồng giao thức/trạng thái** thông qua các seam công khai cao nhất (không kiểm tra chi tiết cài đặt nội bộ), tuân thủ quy chuẩn negative test, kiểm tra toàn vẹn slot lân cận và multi-session trong `CODING_STANDARDS.md`.

### Seams Kiểm thử (Ưu tiên Seam Hiện hữu)

1. **Seam 1 (Chính — End-to-End Protocol & DB)**: `dispatcher::dispatch` + `ServerEnv` + `Conn` + `common::test_db::setup_test_db()`
   - Kế thừa trực tiếp từ các suite chuẩn `npc_eve_e2e_test` và `pure_planner_event_runner_and_preflight_test` cho toàn bộ chuỗi `0x14`, `0x05`/`0x06`, `0x0B FA`, `0x0C` warp, `0x10`, multi-session tổ đội & SQLite reload.
2. **Seam 2 (Data Loader Parity)**: `EveDataLoader::load` + `GroundMmgLoader::load` + `GameData::load`
   - Kiểm chứng nạp `EveSceneInfo.dest_map_id` trên `10,336` lệnh `rt=2` của `Data/eve.emg`, nạp `geolBaseAtt` + `width`/`height` từ `ground_server.mmg` vào `GameData`, và cập nhật suite `ground_test` đọc `ground_server.mmg` thay cho `ground.mmg` đã xóa.
3. **Seam 3 (Pure Domain Decision)**: Public API của `ts_dream::eve` (`EveAutoChainEngine`, `EveResolver`, `GroupData`, `Evaluator`)
   - Kiểm chứng các case biên `when_happen`, `use_mode == 2` vs `1/3`, `chain[1..].results`, và `cc=7, param=2, p_style=4`.
4. **Seam 4 (Architecture & Concurrency Guardrails)**: `architecture_guardrails_test` + test đa phiên & startup schema check
   - Kiểm chứng tuân thủ lock ordering, hot-path di chuyển không cấp phát khi ngoài vùng `encounters`, và kiểm tra startup schema check phát hiện DB thiếu bảng.

### Kế hoạch Test Cụ thể

- **P0 — `should_skip_event`**: Positive cho chuỗi nhiều bước cùng `eve_no` thiếu `cc=12` và NPC có 2 nhiệm vụ độc lập; Negative cho nhiệm vụ một lần có `cc=12` / `cc=2` chặn và 4 guard chống lặp của `AutoChain`.
- **P0 — Warp `EveSceneInfo`**: Positive cho `rt=2` chỉ có trong `EveSceneInfo` (quy đổi `tile * 20 - 10`, cả warp trực tiếp lẫn hậu chiến đấu) và ưu tiên `EveSceneInfo` khi đụng khóa `Warps.txt`; Negative khi thiếu cả hai nguồn hoặc `dest_map_id == 0`.
- **P1 — `DiaHinh` & `ground_server.mmg`**:
  - Kiểm chứng `ground_server.mmg` giữ nguyên đủ `4,528` bản đồ, khớp 100% giá trị `geolBaseAtt`, `width`, `height` với `ground.mmg` gốc trước khi xóa `Data/ground.mmg`, và `ground_test` pass hoàn toàn với `ground_server.mmg`.
  - Xác nhận gói `0x0B 0xFA` cho trận Eve và Random Encounter mang `geolBaseAtt` của map (fallback `112` khi thiếu) và byte cao bằng `0`.
- **P1 — Ngữ nghĩa Engine**:
  - `when_happen`: `MeetDoor` bỏ qua sự kiện chỉ `on_click_obj`; `ClickDoor` bỏ qua sự kiện chỉ `on_stroke_obj`/`on_into_area`; `ClickNpc` vẫn chạy `on_ser_stroke`; `WalkStep` Encounter vẫn chạy với `on_click_obj`.
  - `GroupData.use_mode == 2`: giữ đủ phần tử nhóm bất kể `pick_member` / `probability_rate_ay` và giữ nguyên slot `homdo` phụ cận; `use_mode == 1/3` vẫn chọn 1 phần tử.
  - `EveResolver` AND chain: thực thi đầy đủ cả `chain[0].results` và `chain[1..].results`.
  - `quest_dont`: bước `Advance` (`p_style=1, step=1`) trên nhiệm vụ `Mark.dat position > 0` không bật `0x18 05`; chỉ bật khi có `MarkDont` (`p_style=5`) hoặc `ExplicitMark` (`p_style=3`).
  - `cc=7, param=2, p_style=4`: xác nhận vẫn đánh giá nhất quán theo `reborn_count`.
- **P1 — Hot-path, Async & Concurrency Tổ đội**:
  - Di chuyển `0x06 01` ngoài vùng `encounters` hoàn tất sạch không tạo side-effect; không có caller production nào gọi `run_sync_future`.
  - Multi-session party test cho NPC `15312`/`15313`/`61210`, `0x10` quiz, `MovementHandler` và `40 NPC`: đồng bộ leader + member trên `Conn`, `online_sessions` và sau khi reload SQLite.
- **P2 — EventSession Re-plan, Index & Schema Check**:
  - Chuỗi thoại nhiều bước + `AutoChain` + menu choice phát đúng trình tự `0x14`; startup schema check báo lỗi khi DB rỗng `0` bảng và pass khi đã áp dụng `0001` + `0002`.

---

## Out of Scope

- Giải mã và cài đặt toàn bộ các mã `UNKNOWN` server-only còn lại (`cc ∈ {3, 4, 5, 11, 13, 15, 16, 17}`, `cc=2 p_style >= 4`, `cc=9 p_style ∈ {2,3}`, `cc=12 param != 0`, `rt ∈ {7, 8, 9 (NpcAction thật), 10}`, `GetArmy`, `GoodsData`, `TFightEvent`/`TWinCondition` + `AI`).
- Triển khai các hệ thống gameplay ngoài `eve.emg` (`0x14 0x2A 02/03`, `0x14 0x32/33/38`, `Data/Scene.Dat`, `0x45 Child`, `0x49 WorldBoss`, `0x4D Activity`, `0x4E Astrolabe`).
- Di chuyển hay sửa đổi thư mục tham chiếu bất biến `client_pseudo_c/` hoặc các tệp `Data/` nhị phân khác ngoài việc bóc tách `ground_server.mmg` và xóa `Data/ground.mmg`.

---

## Further Notes

- Tham chiếu đối chiếu và số liệu kiểm chứng:
  - [`EVE_QUEST_100_PERCENT_EVALUATION.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_QUEST_100_PERCENT_EVALUATION.md)
  - [`EVE_QUEST_EVALUATION_REVIEW.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_QUEST_EVALUATION_REVIEW.md)
  - [`EVE_BINARY_ANALYSIS_ALOGIN.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/quest-refactor/EVE_BINARY_ANALYSIS_ALOGIN.md)
  - [`GLOSSARY.md`](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
