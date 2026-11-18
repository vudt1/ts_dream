# Spec: Phân Hệ Sự Kiện Theo Mùa / Lễ Hội (`SeasonalFestival`) — Wire Parity, `Festival.Dat` Encoder, Gating & Điều Khiển Thủ Công

Status: `ready-for-agent`

---

## Problem Statement

Hệ thống server Rust `ts_dream` hiện chưa hỗ trợ vận hành thực tế cho phân hệ **Lễ hội Theo mùa (`SeasonalFestival`)** tương thích với client `aLogin.exe`, dẫn tới bốn vấn đề lớn cho người vận hành (Admin) và người chơi:

1. **Nút Start/Stop lễ hội trên Web Admin không có tác dụng trong game và bị watchdog ép tắt nhầm**:
   - Giao diện quản trị hiện gộp một dòng placeholder chung chung vào bảng *Sự kiện Đặc biệt (`SpecialEvent`)* chịu giới hạn watchdog 120 phút. Khi Admin bấm Start/Stop, server chỉ đổi trạng thái hiển thị trong bộ nhớ và ghi log, không phát thông báo mùa (`S→C 0x17 0x39`), không spawn NPC lễ hội, không mở thưởng dã ngoại, và tự động tắt sau 120 phút nếu không tách vòng đời.
2. **Giao thức mở *Bảng Lore Lễ hội* (`S→C 0x24 0x19` & `Festival.Dat`) bị gãy hoàn toàn với client**:
   - Server hiện phát cứng khóa `19000110` (`0x0121EAC6`) do nhầm lẫn client chuyển đổi `YYYYMMDD` sang `TDateTime`. Thực tế client đọc số nguyên 32-bit rồi ép trực tiếp thành số thực `(double)u32` để so bằng với trường khóa trong `Festival.Dat` (vốn đặt `11.0` ở cả `7` bản ghi gốc). Kết quả là client không bao giờ mở được hộp thoại `TMR_FestivalInfoForm`.
   - Ngoài ra, file `Festival.Dat` gốc đặt chung `11.0` cho cả `7` bản ghi nên các lễ hội từ `#2` đến `#7` không thể mở đúng bài viết riêng nếu chưa có công cụ tái sinh file với khóa phân biệt.
3. **Kênh *Thông báo & Hiệu ứng Mùa* (`S→C 0x17 0x39` & `0x17 0x46`) chưa khớp wire và chưa có caller**:
   - Builder hiện tại chỉ nhận một byte `1..=74` và không được gọi ở bất kỳ luồng runtime nào, trong khi client chia `66` nhánh (`0x01..0x0D`, `0x15..0x4A`, khuyết `0x0E..0x14`), nhiều nhánh bắt buộc phải có byte tham số thứ ba (`payload[2]`) như Niên Thú (`0x44`), Giáng Sinh (`0x2B`/`0x3B`/`0x43`), Thất Tịch (`0x35`), Trung Thu (`0x39`), Đoan Ngọ (`0x33`/`0x42`), và chưa hỗ trợ hiệu ứng pháo hoa (`S→C 0x17 0x46`).
4. **Nội dung lễ hội trong *Kịch bản Eve* (`eve.emg`) mở quanh năm không có rào chắn, thiếu phần thưởng & lưu vết**:
   - Các bản đồ phụ bản lễ hội thuần (`54811..54831`, `12941..12943`, `57535..57536`) và các tuyến cửa/NPC lễ hội trên 4 bản đồ dùng chung với cốt truyện chính (`12000`, `12001`, `12005`, `12421`) chưa bị chặn khi lễ hội đang tắt; người chơi không nhận được bonus drop theo mùa hoặc thông báo nhắc lễ hội khi đăng nhập, và server mất trạng thái lễ hội sau khi khởi động lại.

---

## Solution

Xây dựng phân hệ **Lễ hội Theo mùa (`SeasonalFestival`)** độc lập, chuẩn wire parity với client `aLogin.exe`, tuân thủ **ADR 0011** và **GLOSSARY**:

1. **Tách `SeasonalFestivalManager` độc lập & Điều khiển thủ công trên Web Admin**:
   - Gỡ bỏ placeholder lễ hội khỏi bảng `SpecialEvent` (giữ `8` sự kiện đặc biệt còn lại và giữ Đua Thuyền Rồng `dragon_boat` vận hành độc lập với `doan_ngo`).
   - Quản lý `11` lễ hội theo mùa (*Tết Nguyên Đán*, *Tết Nguyên Tiêu*, *Valentine*, *Ngày Cá Tháng Tư*, *Giỗ Tổ Hùng Vương*, *Lễ Phật Đản*, *Tết Đoan Ngọ*, *Lễ Thất Tịch*, *Tết Trung Thu*, *Halloween*, *Lễ Giáng Sinh*) trong bảng điều khiển riêng trên Web Admin Port 8090, miễn trừ watchdog 120 phút, lưu trạng thái và bộ đếm đợt (`season_seq`) bền vững trong SQLite.
2. **Sửa Wire Parity `S→C 0x24 0x19`, Chuẩn hóa `S→C 0x17 0x39` / `0x17 0x46` & Cung cấp `FestivalDatEncoder`**:
   - Sửa loader `Festival.Dat` đọc khóa số nguyên thực tế từ trường `double` của từng bản ghi (loại bỏ `0x0121EAC6`). Khi chạy với `Festival.Dat` gốc (`11.0` ở cả 7 bản ghi), chỉ cho phép đẩy `24 19` cho bản ghi `#1` (*Tết Nguyên Đán*) và cảnh báo trên Dashboard đối với `#2..#7`.
   - Bổ sung encoder/CLI tái sinh `Festival.Dat` (mã hóa XOR hai khóa chuẩn `0x7EF` byte) gán khóa phân biệt `1..=7` cho 7 lễ hội gốc và tùy chọn bổ sung bản ghi `8..=11` cho các lễ hội còn lại.
   - Chuẩn hóa bộ sinh khung `S→C 0x17 0x39` theo đúng `66` nhánh và `S→C 0x17 0x46` tạo hạt pháo hoa `TFireLight`.
3. **Gating Bản đồ dùng chung vs Bản đồ lễ hội thuần, Di tản an toàn & Phần thưởng Dã ngoại**:
   - Trên `4` bản đồ dùng chung (`12000`, `12001`, `12005`, `12421`), chỉ khóa chi tiết theo cặp `(scene_id, door_id / place_id / quest_id)` của lễ hội đang tắt để tuyến chính hoạt động bình thường.
   - Trên bản đồ phụ bản lễ hội thuần (`54811..54831`, `12941..12943`, `57535..57536`), khóa toàn bộ lối vào khi tắt và tự động phát thông báo kết thúc kèm di tản người chơi bên trong về điểm lưu (`savemap`) khi Admin bấm Stop.
   - Kích hoạt thông báo/popup khi đăng nhập (tối đa `1` popup `24 19` chưa nhận cho mỗi lần đăng nhập theo `season-<seq>`), spawn NPC động (`16 01`), và rớt vật phẩm mùa chống farm trong trận PvE `Normal` theo cấu hình `Data/seasonal_festivals.json`.

---

## User Stories

### Phân hệ 1: Vòng đời Độc lập & Điều khiển Thủ công trên Web Admin

1. Là một Quản trị viên (Admin) trên Web Admin (Port 8090), tôi muốn thấy riêng card **Seasonal Festivals Control Plane** liệt kê `11` lễ hội theo mùa tách biệt hoàn toàn với `8` *Sự kiện Đặc biệt (`SpecialEvent`)*, để tôi không nhầm lẫn giữa hai cơ chế vận hành.
2. Là một Admin, tôi muốn dòng placeholder lễ hội cũ được gỡ khỏi bảng *Special Events Control Plane*, để bảng phụ bản/đấu trường chỉ còn đúng `8` sự kiện đặc biệt chịu watchdog 120 phút.
3. Là một Admin, tôi muốn bấm `Start` một lễ hội cụ thể khi Game TCP Server đang chạy để mở khóa toàn bộ nội dung lễ hội đó, tăng bộ đếm đợt `season_seq` thêm `1`, lưu trạng thái xuống SQLite và phát thông báo `S→C 0x17 0x39` tới toàn bộ người chơi đang online.
4. Là một Admin, tôi muốn bấm `Start` đồng thời nhiều lễ hội (ví dụ Tết Nguyên Tiêu và Valentine) mà không bị khóa chéo (mutex), để có thể tổ chức nhiều hoạt động cùng lúc.
5. Là một Admin, tôi muốn lễ hội đã bật vẫn giữ nguyên trạng thái `Active` qua nhiều ngày và không bị watchdog 120 phút tự động tắt, cho tới khi tôi chủ động bấm `Stop`.
6. Là một Admin, tôi muốn khi Game TCP Server khởi động lại, server tự động khôi phục trạng thái `is_active` và `season_seq` của từng lễ hội từ SQLite, để đợt lễ hội đang chạy không bị mất hoặc reset bộ đếm đợt.
7. Là một Admin, tôi muốn mọi thao tác `Start`, `Stop`, gửi `Lore (24 19)` và bắn `Pháo hoa (17 46)` bị từ chối với HTTP `409 Conflict` khi Game TCP Server đang ở trạng thái `STOPPED`, để không phát sinh trạng thái mồ côi khi socket game chưa mở.
8. Là một Admin, tôi muốn bấm `Start` trên lễ hội đã `Active` hoặc `Stop` trên lễ hội đã `Idle` trả về `409 Conflict`, để tránh tăng nhầm `season_seq` hoặc gửi lặp thông báo kết thúc.
9. Là một Kiểm toán viên, tôi muốn mọi thao tác `Start`, `Stop`, gửi `Lore` và bắn `Pháo hoa` của Admin đều ghi bản ghi vào `admin_audit_log`, để truy vết đầy đủ lịch sử vận hành.
10. Là một Admin, tôi muốn bật/tắt lễ hội `doan_ngo` hoàn toàn độc lập với sự kiện `dragon_boat` (`OP 0x46`) trên bảng *Special Events*, để có thể tổ chức Đua Thuyền Rồng riêng lẻ hoặc song hành tùy ý.

### Phân hệ 2: Wire Parity `S→C 0x24 0x19`, `Festival.Dat` Loader & Encoder

11. Là một Người chơi mở bảng thông tin Tết Nguyên Đán trên client `aLogin.exe` chưa chỉnh sửa `Festival.Dat`, tôi muốn server gửi `S→C 0x24 0x19` mang giá trị `u32 LE = 11` (`0B 00 00 00`), để client ép thành `11.0`, khớp đúng bản ghi `#1` và hiển thị đầy đủ form `TMR_FestivalInfoForm` hai trang.
12. Là một Người chơi đang tham gia các lễ hội `#2..#7` khi server vẫn dùng `Festival.Dat` gốc (mọi bản ghi đều mang `11.0`), tôi muốn server **không** gửi `S→C 0x24 0x19` với `u32 = 11`, để tôi không bị bật nhầm bài viết của Tết Nguyên Đán.
13. Là một Admin nhìn vào cột `Lore Key` trên Dashboard khi server dùng `Festival.Dat` gốc, tôi muốn thấy cảnh báo rõ ràng rằng file client chưa patch khóa phân biệt (chỉ hỗ trợ lore `#1`) và nút `Lore (24 19)` của các lễ hội `#2..#7` bị vô hiệu hóa an toàn.
14. Là một Kỹ thuật viên / Admin muốn kích hoạt đầy đủ bảng lore cho cả `7` lễ hội gốc và tối đa `4` lễ hội mở rộng (`#8..#11`: Thất Tịch, Trung Thu, Halloween, Giáng Sinh), tôi muốn dùng bộ `FestivalDatEncoder` / CLI tool để xuất ra file `Festival.Dat` đã mã hóa XOR hai khóa với khóa số phân biệt `1..=11`, mà không làm hỏng file `Data/Festival.Dat` gốc trong kho lưu trữ.
15. Là một Người chơi sử dụng client có file `Festival.Dat` đã được patch khóa phân biệt `1..=11`, khi lễ hội tương ứng được kích hoạt, tôi muốn nhận đúng khung `S→C 0x24 0x19` mang `u32` tương ứng (`1..=11`) để đọc đúng nội dung văn hóa/lịch sử của lễ hội đó.
16. Là một Admin muốn kiểm tra nhanh giao diện lore trên client hoặc phát lại cho người chơi đang online, tôi muốn bấm nút `Lore (24 19)` của một lễ hội hợp lệ trên Dashboard để server broadcast `S→C 0x24 0x19` tới toàn bộ người chơi đang kết nối.

### Phân hệ 3: Kênh Thông báo Mùa `S→C 0x17 0x39` & Hiệu ứng Pháo hoa `0x17 0x46`

17. Là một Người chơi đang online hoặc vừa đăng nhập khi lễ hội được bật, tôi muốn nhận đúng khung `S→C 0x17 0x39` khớp sub-opcode và tham số `payload[2]` mà client yêu cầu (như Niên Thú `44 01/02`, Noel `2B 01/02`, Thất Tịch `35 01/02`, Trung Thu `39 01..06`, Đoan Ngọ `33 01..03`), để khung chat hệ thống và bảng nổi hiển thị đúng thông điệp tiếng Việt của lễ hội.
18. Là một Nhà phát triển bảo trì giao thức, tôi muốn mọi khung `S→C 0x17 0x39` được sinh qua kiểu định danh tường minh (`SeasonalNotice`), để không thể gửi nhầm sub-opcode thiếu `payload[2]`, mã khuyết (`0x0E..0x14`), hoặc mã phi-lễ-hội (`0x07`, `0x2D..0x32`, `0x40`, `0x41`).
19. Là một Admin đang tổ chức sự kiện Tết hoặc Lễ Tình Nhân, tôi muốn bấm nút `Trigger Fireworks (17 46)` trên Dashboard để toàn bộ người chơi online nhìn thấy hiệu ứng hạt pháo hoa `TFireLight` (`S→C 0x17 0x46`) trên màn hình.

### Phân hệ 4: Đăng nhập, Gating Bản đồ/EVE & Di tản An toàn

20. Là một Người chơi vừa hoàn tất đăng nhập vào game trong lúc có lễ hội đang `Active`, tôi muốn nhận thông báo `S→C 0x17 0x39` cho các lễ hội đang mở, và tự động bật tối đa `1` hộp thoại lore `S→C 0x24 0x19` nếu nhân vật chưa từng nhận trong đợt `season-<seq>` hiện tại.
21. Là một Người chơi relog nhiều lần trong cùng một đợt `season-<seq>` đã nhận popup lore, tôi muốn server không bật lại hộp thoại `S→C 0x24 0x19` nữa, để tránh bị làm phiền mỗi lần đăng nhập.
22. Là một Người chơi quay lại game ở đợt lễ hội mới sau khi Admin đã `Stop` rồi `Start` lại (`season_seq` tăng lên), tôi muốn được nhận lại hộp thoại `S→C 0x24 0x19` một lần cho đợt mới đó.
23. Là một Người chơi đi qua cửa hoặc nói chuyện với NPC cốt truyện thường trên `4` bản đồ dùng chung (`12000`, `12001`, `12005`, `12421`) khi mọi lễ hội đều đang `Idle`, tôi muốn các nhiệm vụ và cổng dịch chuyển cốt truyện chính vẫn hoạt động bình thường không bị chặn nhầm.
24. Là một Người chơi cố tình tương tác NPC, cửa cổng hoặc kích hoạt chuỗi `eve.emg` thuộc về một lễ hội đang `Idle` (cả trên cổng/NPC lễ hội của `4` bản đồ dùng chung lẫn trên bản đồ lễ hội thuần), tôi muốn server từ chối truy cập gọn gàng bằng thông báo hệ thống (`sys_msg_frame`) thay vì cho vào bản đồ hoặc phát thưởng trái phép.
25. Là một Người chơi đang đứng trên bản đồ có NPC lễ hội động (như Niên Thú, Tà Lão Đầu, Ông Già Noel, Nguyệt Lão) khi Admin bấm `Start`, tôi muốn nhận ngay gói đồng bộ hiện NPC (`16 01`) mà không phải chuyển map hay relog; và người chơi mới bước vào map cũng thấy đầy đủ các NPC này.
26. Là một Người chơi đang đứng trong một bản đồ phụ bản lễ hội thuần (`54811..54812`, `54825`, `54827..54831`, `12941..12943`, `57518`, `57531..57536`, `61051`, `61178`, `61551..61567`) khi Admin bấm `Stop` lễ hội đó, tôi muốn nhận thông báo kết thúc `17 39` và được server di tản an toàn về `savemap` (hoặc `12001` nếu chưa đặt `savemap`).
27. Là một Người chơi đang ở giữa trận chiến bên trong bản đồ lễ hội thuần khi Admin bấm `Stop`, tôi muốn server chờ trận chiến kết thúc sạch sẽ rồi mới di tản tôi về `savemap`, để không gây lỗi treo giao diện chiến đấu phía client.

### Phân hệ 5: Phần thưởng Chiến đấu Dã ngoại, Chống Farm & Hiệu ứng Tuyết/EXP

28. Là một Người chơi chiến thắng trận chiến PvE `Normal` trên bản đồ dã ngoại đủ điều kiện trong lúc một lễ hội có cấu hình `bonus_drops` đang `Active`, tôi muốn có cơ hội nhận vật phẩm mùa (như *Bao lì xì năm mới* `46335`, *Túi Đoan Ngọ* `46220`, *Hộp kẹo phá đám* `46073`) vào túi đồ (`homdo`) qua khung `17 06` và lưu bền vững xuống SQLite.
29. Là một Người chơi tham gia trận `PKBattle` (PvP hoặc PK NPC bằng con trỏ PK), tôi muốn hệ thống tuyệt đối không kích hoạt rớt vật phẩm mùa hay cộng thưởng lễ hội, giữ đúng bất biến của `PKBattle`.
30. Là một Người chơi đánh quái cấp thấp dưới ngưỡng `min_enemy_level` cấu hình trong lễ hội, tôi muốn không rớt vật phẩm thưởng lễ hội, để ngăn chặn farm tràn lan tại bản đồ tân thủ.
31. Là một Người chơi đã nhận đủ số lượng `daily_drop_cap` trong ngày (tính theo múi giờ `UTC+7`) cho một vật phẩm mùa, tôi muốn server ngừng rớt thêm vật phẩm đó trong phần còn lại của ngày và mở lại vào ngày hôm sau.
32. Là một Người chơi đánh thắng trận PvE `Normal` khi lễ hội có cấu hình `exp_multiplier_pct` (ví dụ `200%` trong Tết Nguyên Tiêu) đang `Active`, tôi muốn điểm kinh nghiệm thưởng được nhân theo hệ số cấu hình phối hợp chuẩn xác với `perexp`, *Thần Tài (`FGod`)* và *Con Ma (`FGhost`)*.

---

## Implementation Decisions

Các quyết định triển khai dưới đây tuân thủ chặt **ADR 0011**, **ADR 0009**, **CODING_STANDARDS** (`Two-Phase State Lock`, `registry_update_if_online`, `sys_msg_frame`), và **GLOSSARY**:

1. **Tách `SeasonalFestivalManager` khỏi `EventLifecycleManager`**:
   - Loại bỏ hằng số và bản ghi `"seasonal_festival"` khỏi `EventLifecycleManager`, thu gọn bảng `SpecialEvent` về đúng `8` sự kiện đặc biệt.
   - Tạo `SeasonalFestivalManager` quản lý danh mục `11` `FestivalProfile` nạp từ `Data/seasonal_festivals.json` (có tập mặc định built-in tương tự ngân hàng câu hỏi Bắc Tinh Quân).
   - `SeasonalFestivalManager` được giữ trong trạng thái trung tâm của `ServerControl` và truyền vào `BattleService` / môi trường xử lý gói tin để mọi luồng gameplay đều kiểm tra được mà không qua biến toàn cục rải rác.
   - Không đăng ký `SeasonalFestival` vào vòng lặp `check_watchdog()` 120 phút.

2. **Schema SQLite & Trạng thái Đợt Lễ hội (`season_seq`)**:
   - Thêm migration mới tạo 2 bảng `seasonal_festival_state` và `character_festival_claims`:
     ```sql
     CREATE TABLE IF NOT EXISTS seasonal_festival_state (
         festival_id  TEXT PRIMARY KEY,
         is_active    INTEGER NOT NULL DEFAULT 0,
         season_seq   INTEGER NOT NULL DEFAULT 0,
         started_at   INTEGER,
         updated_at   INTEGER NOT NULL DEFAULT 0
     );

     CREATE TABLE IF NOT EXISTS character_festival_claims (
         playerid     INTEGER NOT NULL,
         festival_id  TEXT NOT NULL,
         period_key   TEXT NOT NULL,
         claim_type   TEXT NOT NULL,
         claim_count  INTEGER NOT NULL DEFAULT 1,
         claimed_at   INTEGER NOT NULL,
         PRIMARY KEY (playerid, festival_id, period_key, claim_type)
     );
     ```
   - `season_seq` chỉ tăng `+1` tại thời điểm chuyển trạng thái `Idle -> Active`; giữ nguyên khi `Stop` hoặc khi server restart (hydrate từ `seasonal_festival_state` lúc `ServerControl::start()`).
   - Toàn bộ thao tác ghi đi qua `write_pool` (`max_connections = 1`), đọc qua `read_pool`.

3. **Loader & Encoder `Festival.Dat` + Parity `S→C 0x24 0x19`**:
   - Bộ nạp `FestivalDatLoader` giữ nguyên giải mã dual-XOR (`keyA = rec[n-2]`, `keyB = rec[n-1]`, `RECORD_SIZE = 2031`), nhưng suy `lookup_key: Option<u32>` trực tiếp từ giá trị `double @+0x00` nguyên dương trong file thay vì gán cứng `0x0121EAC6`.
   - Tính bản đồ `unique_lore_keys`: nếu nhiều bản ghi dùng chung một giá trị `double` (như `11.0` ở `Data/Festival.Dat` gốc), chỉ bản ghi có `id` nhỏ nhất (`id == 1`) được phép phát `S→C 0x24 0x19`; các bản ghi trùng sau đó đánh dấu không khả dụng để tránh mở nhầm bài viết.
   - Bổ sung `FestivalDatEncoder` + CLI/binary helper cho phép sinh file `Festival.Dat` mới với khóa `1.0..=7.0` (và tùy chọn `8.0..=11.0`) từ dữ liệu đầu vào JSON/VISCII mà không ghi đè file `Data/Festival.Dat` gốc trong repo.
   - Khi nhận `C→S 0x24 0x19`, chỉ trả `S→C 0x24 0x19` nếu xác định được một lễ hội đang `Active` có `unique_lore_key` hợp lệ.

4. **Type-Safe `SeasonalNotice` (`S→C 0x17 0x39`) & Pháo hoa (`S→C 0x17 0x46`)**:
   - Định nghĩa `enum SeasonalNotice` đại diện tường minh cho từng biến thể thông báo mùa (phân biệt rõ nhánh `1` byte `sub2` và nhánh `2` byte `sub2 + sub3`), thay thế hàm nhận `u8` thô `1..=74`.
   - Bổ sung bộ tạo khung `build_firework_effect_raw(sub, x, y, count)` cho `S→C 0x17 0x46` theo cấu trúc `[0x17, 0x46, sub, x: u16 LE, y: u16 LE, count: u8]`.

5. **Granular Gating trên Bản đồ Dùng chung vs Bản đồ Thuần & Di tản**:
   - Mỗi `FestivalProfile` khai báo tách bạch:
     - `exclusive_maps: Vec<u16>` (bản đồ lễ hội thuần — chặn toàn map khi `Idle`, di tản về `savemap` khi `Stop`).
     - `shared_map_gates: Vec<SharedMapGate>` gồm `(map_id, door_ids, npc_place_ids, quest_ids)` cho các map dùng chung (`12000`, `12001`, `12005`, `12421`, `10995`...) — chỉ chặn đúng đối tượng thuộc lễ hội khi `Idle`.
   - Điểm kiểm tra đặt thống nhất tại chốt chặn tương tác NPC, va chạm cửa (`MeetDoor`), click cửa (`ClickDoor`), và xác nhận dịch chuyển (`0x14 08`).
   - Khi Admin bấm `Stop`, các session đang ở `exclusive_maps` nếu `battle_id == 0` được di tản ngay về `savemap` (fallback `12001`) kèm cập nhật `registry_update_if_online` và ghi DB; nếu đang trong trận thì gắn cờ hoãn di tản để chuyển ngay khi trận kết thúc.

6. **Login Sync, Dynamic NPC Spawn & Battle Bonus Drop**:
   - Luồng đăng nhập hoàn tất (`handle_login_complete`) và đồng bộ bản đồ kiểm tra `SeasonalFestivalManager`: phát `17 39` cho mọi lễ hội đang `Active`, chọn tối đa `1` lễ hội đủ điều kiện chưa có claim `lore_shown` cho `season-<seq>` để đẩy `24 19` và ghi nhận claim, đồng thời phát khung `16 01` cho các NPC lễ hội động thuộc map hiện tại.
   - Sau trận PvE `Normal` thắng lợi, module phần thưởng kiểm tra từng lễ hội đang `Active`: đối chiếu `min_enemy_level`, roll xác suất `drop_rate_pct` qua `DotNetRandom`, kiểm tra và tăng `daily_drop_cap` trong `character_festival_claims` (theo ngày `UTC+7`), sau đó cấp vật phẩm qua quy trình túi đồ chuẩn (All-or-Nothing + `17 06` + `registry_update_if_online`).

7. **Web Admin HTTP/HTMX Contract**:
   - Giữ card `Special Events Control Plane` còn `8` sự kiện đặc biệt; thêm card `Seasonal Festivals Control Plane` trên giao diện dashboard.
   - Các endpoint quản trị lễ hội (`GET /api/festivals`, `POST /api/festivals/{id}/start`, `POST /api/festivals/{id}/stop`, `POST /api/festivals/{id}/lore`, `POST /api/festivals/firework`) tuân theo Server Lifecycle Gate (`409 Conflict` nếu server chưa chạy), trả về HTML fragment HTMX hoặc JSON tùy `HX-Request`, và ghi `admin_audit_log`.

---

## Testing Decisions

### Nguyên tắc & Testing Seams (Highest-Level Seams)
Mọi bài test chỉ kiểm thử **hành vi bên ngoài (external behavior)** qua các seam công khai cấp cao nhất sẵn có của hệ thống, **không** tạo production seam chỉ dành cho test và **không** viết `#[cfg(test)]` nội tuyến trong `src/`:

1. **Seam 1 — Data & Wire Codec (`FestivalDatLoader` / `FestivalDatEncoder` / `SeasonalNotice`)**:
   - Đọc trực tiếp file nhị phân thật `Data/Festival.Dat` và round-trip encode → decode qua bộ nhớ đệm để kiểm chứng layout `0x7EF`, dual-XOR, phát hiện xung đột khóa `11.0`, và byte frame chuẩn xác cho `S→C 0x24 0x19`, `0x17 0x39`, `0x17 0x46`.
2. **Seam 2 — Protocol Dispatcher & Gameplay Handlers (`handle_packet` / `BattleService`)**:
   - Kích hoạt thật qua luồng login, map transfer, talk NPC (`0x14`), di chuyển/va chạm cửa (`0x05`/`0x06`), và kết thúc trận chiến PvE `Normal` vs `PKBattle` để xác minh gating bản đồ dùng chung, di tản map thuần (kể cả khi đang trong battle), giới hạn `1` popup `24 19` mỗi lần login, chống lặp `season_seq`, và giới hạn `min_enemy_level` / `daily_drop_cap`.
3. **Seam 3 — Web Admin Router + SQLite (`web::app::router` qua `tower::ServiceExt::oneshot` & `ServerControl` + `setup_test_db().await`)**:
   - Gọi HTTP request thật tới các route `/api/festivals/*` và kiểm tra phản hồi `200`/`409`, HTML render trên dashboard, trạng thái bền vững qua restart của `seasonal_festival_state`, bộ đếm `season_seq`, `character_festival_claims`, và `admin_audit_log` trên `DB/ts_dream_test.db`.

### Phạm vi & Tiền lệ Kiểm thử (Prior Art)
- Đặt toàn bộ bài test mới thành submodule trong suite chuẩn `tests/events_arena/` (chạy qua `cargo test --test events_arena_test`), kế thừa mô hình test đã có tại phần lễ hội/quiz và web control trong `tests/events_arena/` cùng helper DB test `setup_test_db().await`.
- Mọi tính năng mới đều đi kèm **Negative Tests** tương ứng: bấm `Start`/`Stop`/`Lore` khi TCP server `STOPPED` (`409`), gọi `Lore (24 19)` cho `#2..#7` trên file `Festival.Dat` gốc bị chặn an toàn, vào cổng/NPC lễ hội khi `Idle` bị từ chối nhưng cổng cốt truyện trên cùng map dùng chung vẫn qua được, trận `PKBattle` hoặc quái dưới `min_enemy_level` hoặc vượt `daily_drop_cap` không rơi đồ lễ hội.
- Kết thúc bằng chốt chặn `cargo test --all-targets --no-fail-fast` và kiểm tra `tests/architecture_guardrails_test.rs`.

---

## Out of Scope

- Lập lịch tự động theo ngày Dương lịch / Âm lịch (cron/scheduler chuyển trạng thái không cần thao tác tay của Admin) — mọi thao tác bật/tắt lễ hội đều thực hiện thủ công từ Dashboard.
- Sửa đổi trực tiếp file nhị phân `Data/Festival.Dat` gốc lưu trong repository (chỉ cung cấp công cụ/encoder xuất file đã patch cho client khi người vận hành cần).
- Viết lại các cơ chế riêng của Đua Thuyền Rồng (`OP 0x46`: đếm ngược, BXH 10 người) hoặc các sự kiện đặc biệt khác trong bảng `SpecialEvent` (`arena`, `guild_siege`, `water_war`, `world_boss`).
- Soạn mới nội dung văn chương cho `4` lễ hội mở rộng (`#8..#11`) ngay trong repository nếu chưa có dữ liệu đầu vào từ người vận hành (chỉ cung cấp schema/encoder hỗ trợ).

---

## Further Notes

- Tham chiếu chi tiết bằng chứng ASM/C của `FUN_00722cd4` (`66` nhánh `17 39`), `FUN_0051d914` (`24 19`), `FUN_0062024c` (`17 46`), và danh sách ID map/NPC/item/quest đã trích xuất tại `SEASONAL_FESTIVAL_AUDIT.md`.
- Lưu ý trong quá trình triển khai cần cập nhật các assertion cũ trong `tests/events_arena/` vốn đang assert hằng số sai `0x0121EAC6` (`19000110`) và đang kiểm tra `9` sự kiện trong `EventLifecycleManager` (sau khi gỡ `"seasonal_festival"` còn `8` sự kiện).
