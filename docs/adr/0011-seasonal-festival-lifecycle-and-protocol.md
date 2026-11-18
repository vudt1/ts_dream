# ADR 0011 — Tách Bạch Vòng Đời Lễ Hội Theo Mùa, Cơ Chế Gating Bản Đồ Dùng Chung & Chuẩn Hóa Wire `S→C 24 19` / `Festival.Dat`

- **Ngày**: 2026-11-18
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn `/grill-with-docs`)
- **Tài liệu liên quan**: [SEASONAL_FESTIVAL_AUDIT.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/SEASONAL_FESTIVAL_AUDIT.md), [SEASONAL_FESTIVAL_SOLUTION.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/SEASONAL_FESTIVAL_SOLUTION.md), [ADR 0009](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md)

---

## 1. Bối cảnh (Context)

1. Audit trực tiếp mã dịch ngược client `aLogin.exe` (`client_pseudo_c/`) và source server Rust (`src/`, `templates/dashboard.html`, `tests/events_arena/festival_quiz.rs`) cho thấy phân hệ Sự kiện theo mùa / lễ hội hiện mới ở dạng placeholder và có **4 vấn đề cấu trúc**:
   - **Hiểu sai giao thức `S→C 24 19` (`FUN_0051d914` & `FUN_0077ed68`)**: Tài liệu trước đây giả định `FUN_0077ed68` giải mã `YYYYMMDD` sang Delphi `TDateTime` (`19000110` $\rightarrow$ `11.0`), dẫn tới `src/data/loaders/festival.rs:24` và `src/server/handlers/festival.rs:42-47` gửi cứng `FESTIVAL_PARITY_DATE_DWORD = 0x0121EAC6` (`19000110`). Thực tế ASM (`0051d914.asm.txt:25-44`, `0077ed68.c:155-216`, `0051e2dc.asm.txt:23-27`) chứng minh `FUN_0077ed68` chỉ đọc `u32 LE`, rồi `FILD qword` ép trực tiếp sang `(double)u32` (`19000110.0`) để so bằng `==` với `double @+0x00` trong `Data/Festival.Dat` (`11.0` ở cả 7 bản ghi). Hậu quả là **client không bao giờ mở form `TMR_FestivalInfoForm`**.
   - **Xung đột vòng đời với `EventLifecycleManager`**: Hiện `EVENT_SEASONAL_FESTIVAL` (`"seasonal_festival"`) nằm chung trong [`src/server/events.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/events.rs#L171) với các sự kiện đấu trường/phụ bản (chịu chung watchdog 120 phút `check_watchdog`, có trường `encounter_rate`, và không có handler gameplay nào đọc `is_event_active`).
   - **Bản đồ dùng chung (`Shared Maps`) vs Bản đồ lễ hội thuần**: Trong `Data/eve.emg`, các bản đồ `12000`, `12001`, `12005`, `12421` chứa cả tuyến cốt truyện chính lẫn nội dung lễ hội (Đoan Ngọ, Nguyên Tiêu, Thất Tịch, Nướng thịt, Giáng Sinh); gate theo `map_id` thô sẽ khóa nhầm người chơi thường.
   - **Khuyết công cụ sửa `Festival.Dat` & vòng lặp `season_seq`**: File `Data/Festival.Dat` gốc gán chung `11.0` cho cả 7 bản ghi (khiến `rec02..rec07` không thể tra được bằng `FUN_0051e2dc`), trong khi việc điều khiển thủ công không có năm/kỳ cố định đòi hỏi cách định danh đợt lễ hội để chống spam popup `24 19` khi relog nhưng vẫn mở lại được khi Admin bật đợt mới.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Tách `SeasonalFestivalManager` độc lập & Điều khiển thủ công từ Dashboard
- **Quyết định**: Tách quản lý `11` lễ hội theo mùa ra khỏi bảng `Special Events Control Plane` của `EventLifecycleManager` sang `SeasonalFestivalManager` riêng (miễn trừ watchdog 120 phút), gỡ bỏ dòng placeholder `"seasonal_festival"` khỏi danh sách 8 `SpecialEvent` cũ.
- **Kích hoạt thủ công (Manual-only)**: Không chạy cron/scheduler tự động theo lịch Âm/Dương; mọi thao tác `Start` / `Stop` từng lễ hội, phát `Lore (24 19)` và bắn `Pháo hoa (17 46)` do Admin thực hiện trực tiếp trên Web Admin Dashboard (bảo vệ bởi Server Lifecycle Gate `409 Conflict` khi TCP server đang `STOPPED` và ghi `admin_audit_log`).
- **Độc lập với `dragon_boat` (`OP 0x46`)**: Giữ `dragon_boat` vận hành độc lập trong nhóm `SpecialEvent`; bật/tắt `doan_ngo` không khóa hay ép trạng thái `dragon_boat`.

### 2.2. Wire Parity `S→C 24 19`, Giữ `Data/Festival.Dat` Gốc & Hỗ Trợ `FestivalDatEncoder`
- **Quyết định sửa parser**: `FestivalDatLoader` suy `date_dword` trực tiếp từ giá trị `double @+0x00` thực tế trong file (`dt as u32`), loại bỏ hằng số sai `0x0121EAC6` (`19000110`) và cập nhật lại test assertion tương ứng trong `tests/events_arena/festival_quiz.rs`.
- **Giữ file `Data/Festival.Dat` gốc trong repo**: Không ghi đè file nhị phân gốc trong `Data/`. Khi server chạy với file gốc (mọi bản ghi đều `11.0`):
  - Chỉ cho phép gửi `S→C 24 19` với `u32 = 11` cho `rec01` (*Tết Nguyên Đán*).
  - Bỏ qua gửi `24 19` đối với `rec02..rec07` (chỉ phát thông báo `17 39`) và hiển thị cảnh báo trên Dashboard để tránh client mở nhầm bài *Tết Nguyên Đán*.
- **Cung cấp `FestivalDatEncoder` / CLI patcher**: Hỗ trợ tái sinh file `Festival.Dat` (dual-XOR `keyA`/`keyB`, record `0x7EF` byte) với khóa phân biệt `1..=7` cho 7 lễ hội gốc, đồng thời cho phép cấu hình thêm bản ghi `8..=11` (*Thất Tịch*, *Trung Thu*, *Halloween*, *Giáng Sinh*) để đồng bộ sang client khi cần hiển thị đủ popup lore.
- **Giới hạn `24 19` khi login**: Mỗi lần đăng nhập chỉ đẩy tối đa `1` khung `24 19` chưa được nhận trong đợt (do client chỉ có 1 form singleton `TMR_FestivalInfoForm` tại `gvar_007DA1EC`).

### 2.3. Định danh Đợt Lễ Hội (`season_seq`) & Persistence SQLite
- **Quyết định**: Bảng `seasonal_festival_state` lưu `is_active` và `season_seq: u64`. Giá trị `season_seq` chỉ tăng `+1` khi Admin chuyển trạng thái từ `Idle -> Active` và được giữ nguyên khi server restart.
- Bảng `character_festival_claims` dùng `period_key = "season-<season_seq>"` cho các quyền lợi một lần mỗi đợt (như `lore_shown` cho `24 19`), và `period_key = "YYYY-MM-DD"` (theo `UTC+7`) cho giới hạn rơi đồ theo ngày (`daily_drop_cap`).

### 2.4. Gating Bản Đồ Dùng Chung vs Bản Đồ Lễ Hội Thuần & Di Tản khi `Stop`
- **Bản đồ dùng chung (`12000`, `12001`, `12005`, `12421`)**: Gate chi tiết ở cấp `(scene_id, door_id / npc_place_id / quest_id)` trong `talk.rs`, `movement.rs` và `npc_event.rs` để tuyến cốt truyện chính luôn hoạt động bình thường.
- **Bản đồ lễ hội thuần (`54811..54812`, `54821..54831`, `12941..12943`, `57535..57536`)**: Gate toàn bộ `map_id`. Khi Admin bấm `Stop` lễ hội, server phát thông báo kết thúc `S→C 17 39` và di tản người chơi đang đứng trong các map thuần này về `savemap` (hoặc `12001` nếu `savemap == 0`, chờ xong trận nếu đang ở trong battle).
- **Bonus Drop & Chống farm**: Bảng vật phẩm rơi trong trận dã ngoại (`drop_rate_pct`, `min_enemy_level`, `daily_drop_cap`) cấu hình data-driven trong `Data/seasonal_festivals.json`.

---

## 3. Hệ quả (Consequences)

- **Tương thích ngược & An toàn triển khai**: Client chưa patch `Festival.Dat` vẫn mở được lore #1 (*Tết Nguyên Đán*) đúng byte và nhận đủ toàn bộ thông báo `17 39`, `17 46`, `EVE`, `Drop` cho tất cả `11` lễ hội mà không bị lỗi hay mở nhầm bảng.
- **Ràng buộc refactor tiếp theo**: Mọi test hoặc handler liên quan tới `EventLifecycleManager` phải gỡ bỏ tham chiếu tới `EVENT_SEASONAL_FESTIVAL` cũ và chuyển sang kiểm tra `SeasonalFestivalManager`; mọi thay đổi trên `Data/Festival.Dat` phải đi qua `FestivalDatLoader` / `FestivalDatEncoder` chuẩn `0x7EF`.
