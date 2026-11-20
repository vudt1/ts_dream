# ADR 0012 — Nội Suy Bước Đi (Walk Interpolation), Chuẩn Hóa Proximity 130 px & Lưu Trữ Màu Ngoại Hình (`characters.color`)

- **Ngày**: 2026-11-20
- **Trạng thái**: Accepted
- **Người quyết định**: Owner & Engineering Team
- **Tài liệu liên quan**: [`GLOSSARY.md`](../../GLOSSARY.md), [ADR 0010](0010-warp-door-sceneinfo-and-battlegate.md), [`migrations/0004_character_color.sql`](../../migrations/0004_character_color.sql)

---

## 1. Bối cảnh (Context)

Qua phân tích mã dịch ngược client `aLogin.exe` (`client_pseudo_c/`) và kiểm thử thực tế (live-test), phát hiện **3 vấn đề kiến trúc**:

1. **Client gửi tọa độ đích ngay khi click (`0x06 0x01`)**:
   - Khi người chơi click di chuyển trên bản đồ, `aLogin.exe` gửi ngay gói `0x06 0x01` chứa tọa độ đích `(dest_x, dest_y)` lên server, sau đó mới chạy hoạt ảnh nhân vật đi bộ từ vị trí hiện tại tới đích trên client.
   - Nếu server cập nhật `session.map_x`/`session.map_y` ngay lập tức bằng tọa độ đích và dùng trực tiếp cho `MeetDoor`, `14 01` (gửi lời mời lập nhóm), `14 04` (chấp nhận nhóm) hay `14 08` (xin vào nhóm), server sẽ thấy nhân vật "dịch chuyển tức thời" tới đích trước khi client thực sự bước tới, gây sai lệch điều kiện khoảng cách và kích hoạt cửa chuyển cảnh (`MeetDoor`) quá sớm.
2. **Ngưỡng khoảng cách không đồng nhất giữa các handler lập nhóm**:
   - Các nhánh kiểm tra khoảng cách cho `14 01`, `14 04`, `14 08` trước đây dùng ngưỡng khác nhau (hộp `±150 px` hoặc bán kính `200 px`), không khớp với tầm tương tác thực tế của client.
3. **Mất màu ngoại hình nhân vật sau khi restart / relog**:
   - Trong `0x09 0x01` (tạo nhân vật), `payload[4..12]` chứa **8 byte** màu ngoại hình (tóc, da, áo, quần, giày — mã hóa 16 ký tự hex), tách biệt hoàn toàn với `hair` (1 byte kiểu tóc tại `payload[2]`). Lệnh `reborn` cũng cập nhật lại chuỗi màu này.
   - Sau ADR 0001 cleanup, `session.color` chỉ được giữ trong RAM (deferred drift) và không có cột trong `characters`, khiến nhân vật khi relog hoặc restart server bị rơi về màu mặc định `0000000000000000` trong gói `0x03`.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Nội suy bước đi tuyến tính (Linear Walk Interpolation) & `MeetDoor` Timer Hủy Được
- **Quyết định**: Lưu cặp điểm `(walk_origin, walk_dest, walk_start_ms)` trên phiên khi nhận `0x06 0x01` và tính vị trí tức thời bằng nội suy tuyến tính theo tốc độ chuẩn của client:
  - Bản đồ thường: `160 px/s` (`0.16 px/ms`).
  - Bản đồ thế giới (`map_id % 1000 == 0`): `128 px/s` (`0.128 px/ms`).
- **Hẹn giờ `MeetDoor` hủy được (Cancellable `MeetDoor` Timer)**: Thay vì kiểm tra `MeetDoor` ngay tại thời điểm nhận `0x06 0x01`, server tính thời gian di chuyển dự kiến tới đích và đặt timer hủy được; nếu người chơi đổi hướng hoặc dừng trước khi tới cửa, timer cũ bị hủy bỏ.
- **Đánh đổi (Trade-off)**: Nội suy tuyến tính bỏ qua vật cản vi mô (client tự pathfind theo lưới), nhưng đủ chính xác để đồng bộ thời điểm tới đích với sai số nhỏ mà không phải chạy tick vật lý nặng cho toàn bộ map.

### 2.2. Chuẩn hóa Proximity Gate bán kính Euclidean `130 px`
- **Quyết định**: Chuẩn hóa một hàm kiểm tra khoảng cách dùng chung với bán kính **Euclidean `130 px`** ($\sqrt{\Delta x^2 + \Delta y^2} \le 130$), thay thế các phép thử hộp `±150 px` và bán kính `200 px`.
- **Phạm vi áp dụng đồng nhất**: Áp dụng thống nhất cho `14 01`, `14 04`, `14 08` (lập nhóm) dựa trên vị trí nội suy hiện tại của cả hai người chơi.

### 2.3. Khôi phục lưu trữ `characters.color` tách biệt khỏi `hair`
- **Quyết định**: Bổ sung cột `color TEXT NOT NULL DEFAULT '0000000000000000'` vào bảng `characters` qua [`migrations/0004_character_color.sql`](../../migrations/0004_character_color.sql), kiểm tra sự tồn tại của cột tại `verify_schema()` lúc khởi động.
- **Chuẩn hóa dữ liệu (`normalize_color_hex`)**: Mọi đường đọc/ghi (`load`, `save_in_tx`, `create_and_seed`, `update_player_text`) chuẩn hóa chuỗi màu về đúng **16 ký tự hex viết hoa** (`0-9A-F`); các giá trị rỗng, sai độ dài hoặc không phải hex được đưa về `"0000000000000000"`.
- **Theo dõi thay đổi**: `auto_save::fingerprint` trộn `s.color.as_bytes()` vào hash FNV-1a để AutoSave tự động ghi nhận thay đổi màu khi reborn hoặc chỉnh sửa ngoại hình.

---

## 3. Hệ quả (Consequences)

- **Tương thích ngược**: CSDL cũ cần chạy thêm `migrations/0004_character_color.sql` (nếu thiếu, `verify_schema()` báo lỗi rõ ràng hướng dẫn chạy migration 0004 theo ADR 0004). Các nhân vật cũ tự động nhận giá trị mặc định `'0000000000000000'`.
- **Ràng buộc triển khai**: `hair` (1 byte kiểu tóc) và `color` (8 byte / 16 hex màu ngoại hình) được tách bạch hoàn toàn từ DB tới wire `0x03`; không gộp byte màu vào `hair`.
