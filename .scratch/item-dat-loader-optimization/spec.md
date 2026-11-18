Status: ready-for-agent

# Đặc tả Tối ưu hóa Nạp Dữ liệu `Item.dat` cho Server

## Problem Statement

Hiện tại, khi Game Server khởi động, bộ nạp dữ liệu vật phẩm đọc toàn bộ tệp nhị phân `Item.dat` (~2.95 MiB, 8,373 bản ghi × 370 bytes) và giải mã đầy đủ tất cả các cột dữ liệu của Client — bao gồm phần mô tả dài tới 254 bytes (`description` và `description_viscii`), icon, hình ảnh nam/nữ, bảng màu tint, thông tin giao diện/sắp xếp, cùng 15 trường không tồn tại trong định dạng PC 370-byte được gán cứng giá trị `0`. Việc giải mã từng ký tự VISCII sang Unicode và cấp phát hàng chục nghìn `String` / `Vec<u8>` không bao giờ được server sử dụng gây lãng phí CPU và bộ nhớ RAM lúc khởi động.

Đồng thời, hệ thống dữ liệu trung tâm (`GameData`) đang lưu giữ song song hai bảng tra cứu cho cùng một tập vật phẩm: bảng định nghĩa thô 54 trường (`item_defs`) và bảng bản ghi in-memory 29 trường (`items`), trong đó `items` dùng kiểu `i64` cho hầu hết các trường số và sao chép lại tên VISCII. Bên cạnh đó, mỗi khi khởi tạo trận đấu, `BattleService` lại nhân bản toàn bộ bảng `items` sang ngữ cảnh trận đấu.

## Solution

Chuyển bộ nạp `Item.dat` của server sang mô hình **chỉ giải mã các cột phục vụ nghiệp vụ server** và **hợp nhất lưu trữ về một nguồn sự thật duy nhất (single source of truth)**:

1. **Bỏ qua hoàn toàn các cột thuần Client**: Không giải mã, không chuyển đổi bảng mã và không cấp phát bộ nhớ cho mô tả vật phẩm (`description`, `description_viscii`), tài nguyên đồ họa (`icon_id`, `male_pic_id`, `female_pic_id`, `male_color_tints`, `female_color_tints`, `display_material`) và trạng thái giao diện (`sort`, `btn_state`).
2. **Hợp nhất `item_defs` và `items` trên `GameData`**: Bổ sung hai trường giá mua/bán (`price`, `sell_price`) vào bản ghi vật phẩm dùng chung của server, giải mã tên UTF-8 theo nhu cầu (on-demand) từ byte VISCII tại các điểm thông báo hệ thống thay vì lưu sẵn `String` cho mọi vật phẩm, và chuyển toàn bộ hệ thống Crafting, NPC Shop sang đọc từ một bảng vật phẩm thống nhất.
3. **Chia sẻ bất biến thay vì nhân bản**: Chia sẻ bảng vật phẩm dùng chung qua tham chiếu đếm (shared immutable reference) cho `BattleService` thay vì clone toàn bộ `HashMap` cho mỗi trận đấu.

---

## User Stories

1. As a server operator, I want the server to skip decoding client-only visual and UI columns from `Item.dat` at boot, so that server startup is faster and consumes significantly less RAM.
2. As a server operator, I want the server to skip allocating and converting 254-byte item descriptions (`description` and `description_viscii`) for all ~8,372 items, so that over 16,000 unnecessary heap allocations are eliminated at startup.
3. As a server operator, I want `GameData` to maintain a single unified item lookup table instead of storing both `item_defs` and `items` simultaneously, so that in-memory item metadata is not duplicated.
4. As a server operator, I want `BattleService` to share the read-only item lookup table via reference counting rather than cloning the entire map, so that battle initialization avoids unnecessary memory copies.
5. As a player, I want NPC Shop buy and sell transactions to continue using the exact `price` and `sell_price` values from `Item.dat`, so that economy wire parity and pricing remain unchanged.
6. As a player, I want item crafting (Compound and feather socketing) to continue validating item `kind`, `compound_material`, and `compound_rank` identically, so that crafting success rates and restrictions behave the same.
7. As a player, I want crafting result notifications (`sys_msg_frame`) to continue displaying the item's UTF-8 name accurately, even though UTF-8 `String` names are no longer pre-allocated for every item at boot.
8. As a player, I want inventory template hydration, item usage (HP/SP potions, add-pet items), pet reborn (`rb_pet_from`, `rb_pet_to`), quest rewards, and static map item drops to preserve all item stats and attributes (`level`, `loai`, `equip_pos`, `thuoctinh`, `value`, `hp`, `sp`, `int1..fai2`) without regression.
9. As a developer, I want a clean loader interface for server item data that does not introduce legacy field aliases, so that the codebase stays compliant with architecture guardrails.
10. As a developer, I want dedicated automated tests covering `Item.dat` binary parsing, edge cases (record 0 skip, `id == 0` skip, invalid file length, stat offsets for `attr1_kind`/`attr2_kind` `65..=67`), and unified `GameData` consumers, so that regressions in binary decoding or downstream handlers are caught immediately.

---

## Implementation Decisions

### 1. Tối ưu hóa Bộ nạp nhị phân `Item.dat`
- **Tách ranh giới dữ liệu server vs full-client**: Bổ sung phương thức nạp gọn cho server (hoặc thu gọn đầu ra của bộ nạp mặc định) chỉ đọc các offset thực sự cần cho nghiệp vụ server từ mỗi record 370 bytes:
  - Header & định danh: `namelen` + `name` (VISCII bytes), `kind`, `id`.
  - Thuộc tính chiến đấu & trang bị: `attr1_kind`, `attr2_kind`, `attr1_value`, `attr2_value` (giữ nguyên quy tắc `+100` khi `attr*_kind` nằm trong `65..=67`), `fit_type`, `level`, `element`, `element_value`.
  - Thuộc tính chế tạo & sủng vật: `compound_material`, `compound_rank`, `spare3` (`rb_pet_from`), `suit_id` (`rb_pet_to`), `role_count_index` (`add_pet`).
  - Thuộc tính kinh tế: `price`, `sell_price`.
- **Không giải mã các vùng dữ liệu thuần Client**: Bỏ qua hoàn toàn việc đọc, đảo byte, và chuyển đổi bảng mã tại vùng `description` (115..370), các mã hình ảnh (`icon_id`, `_large_icon_num`, `male_pic_id`, `female_pic_id`), bảng màu (`male_color_tints`, `female_color_tints`), `display_material`, `sort`, `btn_state`, và các trường chưa dùng/hardcoded `0`.
- **Tên UTF-8 giải mã theo nhu cầu (On-demand)**: Không lưu trường `name: String` (UTF-8) trong bảng tra cứu thường trú. Chỉ lưu `name` dạng `Vec<u8>` (VISCII 1.1) phục vụ đóng gói wire packet, và cung cấp hàm helper chuyển đổi sang UTF-8 `String` khi cần hiển thị thông báo hệ thống (như toast khi craft).

### 2. Hợp nhất mô hình dữ liệu trên `GameData`
- **Bổ sung trường kinh tế vào bảng `Item` dùng chung**: Thêm `price: u32` và `sell_price: u32` vào cấu trúc bản ghi vật phẩm chuẩn của server (`tables::Item`).
- **Xóa bỏ lưu trữ kép (`item_defs`) khỏi `GameData`**: `GameData` chỉ giữ một bảng tra cứu vật phẩm duy nhất (`items`). Toàn bộ luồng khởi tạo nhị phân và khởi tạo từ dữ liệu text legacy đều nạp trực tiếp vào bảng thống nhất này (giữ nguyên tính năng nạp `Items.txt` fallback khi cần).
- **Chuyển đổi các phân hệ tiêu thụ**:
  - Phân hệ **Crafting** (validator, algorithm, và crafting handler) đọc `id`, `kind`, `compound_material`, `compound_rank` và tên UTF-8 (qua helper) trực tiếp từ `data.items`.
  - Phân hệ **NPC Shops** đọc `price` và `sell_price` trực tiếp từ `data.items`.
- **Tuân thủ Architecture Guardrails**: Không thêm các alias/field tương thích ngược mang tên cũ vào `ItemDef` hay `Item`; cập nhật thẳng các điểm gọi trong `src/` và `tests/`.

### 3. Chia sẻ dữ liệu trong `BattleService`
- Thay đổi trường lưu trữ bảng vật phẩm trong `GameData` hoặc `BattleData` để `BattleService` chia sẻ tham chiếu bất biến (`Arc<HashMap<i64, Item>>`) thay vì `.clone()` toàn bộ bảng vật phẩm mỗi khi khởi tạo `BattleData`.

### 4. Đường nạp tương thích cho Test / Công cụ ngoại tuyến (nếu cần)
- Nếu vẫn giữ hàm nạp đầy đủ 54 trường cho mục đích kiểm tra dữ liệu gốc hoặc công cụ offline, tách biệt rõ ràng với đường nạp server mặc định để runtime production không bao giờ trả chi phí cho `ItemDef` đầy đủ.

---

## Testing Decisions

### Nguyên tắc kiểm thử & Seam đề xuất
- **Seam 1 (Binary Loader Seam)**: Kiểm thử trực tiếp qua API công khai của bộ nạp `Item.dat` với dữ liệu nhị phân tổng hợp (synthetic 370-byte records) và dữ liệu thật `Data/Item.dat`. Đây là seam cao nhất cho tầng nạp dữ liệu mà không phụ thuộc chi tiết cấp phát nội bộ.
  - Xác minh: từ chối kích thước không chia hết cho 370; bỏ qua record `0` và `id == 0`; giải mã đúng `name` VISCII, `price`, `sell_price`, `compound_material`, `compound_rank`, `fit_type`, `kind`, và phép cộng `+100` cho `attr*_kind` trong `65..=67`.
- **Seam 2 (GameData & Domain Handler Seam)**: Kiểm thử hành vi đầu cuối (external behavior) của các phân hệ tiêu thụ dữ liệu vật phẩm thông qua `GameData` và các handler tương ứng:
  - **NPC Shop**: Gửi lệnh mua/bán vật phẩm qua shop handler với `GameData.items` chứa `price`/`sell_price` và xác nhận số tiền giao dịch.
  - **Crafting**: Chạy validator/algorithm và crafting handler với `GameData.items` để xác nhận kiểm tra `kind`, `compound_material`, `compound_rank` và chuỗi UTF-8 trong `sys_msg_frame`.
  - **Inventory Hydration & Battle**: Kiểm tra `hydrate_templates` với cuốc đào khoáng (`kind == 39`, `fit_type == 3`) và luồng dùng vật phẩm hồi HP/SP trong trận đấu.

### Phân hệ & Prior Art trong repo
- **Data Loaders** (`tests/data_loaders/`): Tạo test module mới cho `Item.dat` theo mẫu các loader test hiện có (như `warps.rs`, `ground.rs`).
- **Compound & Economy/Inventory** (`tests/compound_test.rs`, `tests/economy_inventory/economy_wire.rs`, `tests/economy_inventory/inventory_template_hydrate.rs`, `tests/economy_inventory/pet_equip_unequip.rs`): Cập nhật các test dựng dữ liệu giả từ `item_defs` sang dựng trực tiếp vào `items`.
- **Architecture Guardrails** (`tests/architecture_guardrails_test.rs`): Chạy để đảm bảo không vi phạm quy chuẩn cấu trúc mã nguồn.

---

## Out of Scope

- Thay đổi định dạng tệp nhị phân `Data/Item.dat` trên đĩa hoặc chỉnh sửa công cụ build/client Delphi.
- Thu hẹp kiểu dữ liệu của toàn bộ các trường số hiện có trên `tables::Item` từ `i64` xuống `u8`/`u16`/`i32` (có thể thực hiện như một đợt tối ưu độc lập sau khi đã hợp nhất xong `item_defs` và `items`).
- Triển khai logic server cho các trường PC/Mobile đang chưa dùng trong `src/` (như `durable`, `gender`, `restrict`, `direct_use`, `special_ability`, `skill_link`).
- Thay đổi cơ chế nạp các tệp `.dat` khác (`Npc.dat`, `Skill.dat`, `Talk.dat`, `Scene.dat`).

---

## Further Notes

- Dữ liệu `Data/Item.dat` hiện tại có `3,098,010` bytes (`8,373` records). Chỉ riêng việc loại bỏ phần `description` (255 bytes/record) và bảng `item_defs` trùng lặp sẽ cắt giảm phần lớn lượng heap allocation khi khởi động server.
- Toàn bộ thay đổi cần được kiểm chứng bằng `cargo test --all-targets --no-fail-fast` trước khi hoàn tất.
