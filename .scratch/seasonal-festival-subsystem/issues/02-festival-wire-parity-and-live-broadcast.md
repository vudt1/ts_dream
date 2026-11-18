# 02: Wire Parity `S→C 0x24 0x19` (`Festival.Dat` Gốc), `SeasonalNotice` (`0x17 0x39`), Pháo hoa (`0x17 0x46`) & Broadcast Trực tiếp

**What to build:** Từ góc nhìn người chơi đang online và Admin trên Dashboard, sửa đúng giao thức *Bảng Lore Lễ hội* (`S→C 0x24 0x19`) để khớp với cơ chế ép `(double)u32` của client `aLogin.exe` trên file `Festival.Dat` gốc, chuẩn hóa toàn bộ `66` nhánh *Thông báo Mùa* (`S→C 0x17 0x39`) và hiệu ứng hạt pháo hoa `TFireLight` (`S→C 0x17 0x46`), rồi nối các nút Dashboard để broadcast trực tiếp xuống toàn bộ người chơi đang kết nối.

**Blocked by:** 01: Tách `SeasonalFestivalManager`, Persistence `season_seq` & Bảng Điều khiển Lễ hội trên Web Admin

**Status:** completed

- [x] Sửa bộ nạp `Festival.Dat` đọc khóa số nguyên `lookup_key` trực tiếp từ giá trị `double @+0x00` thực tế của từng bản ghi (loại bỏ hoàn toàn hằng số sai `0x0121EAC6` / `19000110` và cập nhật lại assertion cũ tương ứng).
- [x] Phát hiện va chạm khóa trong `Festival.Dat`: khi nhiều bản ghi mang chung giá trị (như `11.0` ở cả `7` bản ghi gốc), chỉ đánh dấu bản ghi `#1` (*Tết Nguyên Đán*) là hợp lệ để phát `S→C 0x24 0x19` (`u32 LE = 11`), các bản ghi `#2..#7` bị đánh dấu chưa có khóa riêng và không bao giờ gửi `24 19` với `u32 = 11`.
- [x] Chuẩn hóa kiểu `SeasonalNotice` ánh xạ tường minh toàn bộ `66` nhánh `S→C 0x17 0x39` của client (`0x01..0x0D` và `0x15..0x4A`, tách rõ nhánh `1` byte `sub2` và nhánh `2` byte `sub2 + sub3`), không phát mã khuyết (`0x0E..0x14`) hoặc mã phi-lễ-hội từ phân hệ lễ hội; bổ sung bộ tạo khung `S→C 0x17 0x46` (`[0x17, 0x46, sub, x: u16 LE, y: u16 LE, count: u8]`).
- [x] Nối hành động `Start` / `Stop` lễ hội trên Dashboard để tự động broadcast `S→C 0x17 0x39` mở màn / kết thúc tới toàn bộ client online; bổ sung nút `Lore (24 19)` (`POST /api/festivals/{id}/lore`) và nút `Trigger Fireworks (17 46)` (`POST /api/festivals/firework`) kèm ghi `admin_audit_log`.
- [x] Hiển thị cảnh báo rõ ràng trên cột `Lore Key` của Dashboard đối với các bản ghi `#2..#7` khi chạy trên `Festival.Dat` gốc chưa patch, vô hiệu hóa nút `Lore (24 19)` tương ứng trên UI và trả HTTP `409 Conflict` nếu gọi trực tiếp API; đồng thời đảm bảo `C→S 0x24 0x19` chỉ phản hồi khi có lễ hội `Active` mang khóa hợp lệ.

## Comments

- **2026-11-18 — DeepCoder + Two-Axis Code Review (`Standards` & `Spec`)**:
  - Đã hoàn thành toàn bộ 5 mục của Ticket 02 trong working tree:
    - `src/data/loaders/festival.rs` & `src/data/loaders/mod.rs`: gỡ `FESTIVAL_PARITY_DATE_DWORD = 0x0121EAC6`, thêm `derive_lookup_key` và `unique_lore_key` (trên `Data/Festival.Dat` gốc chỉ bản ghi `#1` có `unique_lore_key = Some(11)`).
    - `src/server/handlers/festival.rs`: triển khai `enum SeasonalNotice` (`0x01..=0x0D` và `0x15..=0x4A` = 67 mã hợp lệ của client), `build_firework_effect_raw` (`0x17 0x46`), và `handle_festival` (`0x24 0x19`).
    - `src/server/seasonal_festival.rs`: gom `transition_festival`, nạp `Festival.Dat` vào `SeasonalFestivalManager::from_data_dir`, cung cấp `unique_lore_key` / `lore_collision`.
    - `src/web/app.rs` & `templates/dashboard.html`: nối broadcast `17 39` khi Start/Stop, thêm `POST /api/festivals/{id}/lore` và `POST /api/festivals/firework`, hiển thị cột `Lore Key` + cảnh báo collision và vô hiệu hóa nút Lore cho `#2..#11` khi chưa có khóa duy nhất.
    - `tests/events_arena/festival_wire_parity.rs` + `tests/events_arena/festival_quiz.rs` + `tests/events_arena/seasonal_festival_control.rs`: 7 tests mới + cập nhật assertions cũ (`events_arena_test`: 74 passed; toàn bộ 11 suites: 669 passed, 0 failed).
  - Kết quả review 2 trục: **0 lỗi chuẩn bắt buộc**, **0 sai lệch spec** (ghi chú dọn nhẹ sau: 3 lễ hội `ca_thang_tu`, `gio_to_hung_vuong`, `phat_dan` chưa có nhánh `17 39` riêng trên client nên không phát `17 39`; có thể thêm `400 Bad Request` nếu payload JSON của `/firework` bị lỗi cú pháp).
