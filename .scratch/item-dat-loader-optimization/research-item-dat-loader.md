# Nghiên cứu & Kế hoạch tối ưu `ItemDatLoader` (`Data/Item.dat`)

> **Phạm vi**: Đối chiếu trực tiếp từ mã nguồn `src/data/loaders/item.rs`, `src/data/tables.rs`, `src/data/loader.rs`, các handler/service tiêu thụ dữ liệu item trong `src/`, và test suite trong `tests/`.

---

## 1. Hiện trạng nạp `Data/Item.dat`

### 1.1 Quy mô file & cơ chế parse
- **Kích thước file**: `Data/Item.dat` có dung lượng **3,098,010 bytes** (~2.95 MiB).
- **Cấu trúc record cố định**: `ItemDatLoader::FIELD_LENGTH = 370` bytes (`src/data/loaders/item.rs:124`) → tổng cộng `3,098,010 / 370 = 8,373` records (bỏ qua record `0` theo comment tại `src/data/loaders/item.rs:159-160`, còn tối đa `8,372` records, bỏ qua các record có `id == 0` tại `:177-179`).
- **Hàm giải mã** (`src/data/loaders/item.rs:127-144`):
  - `dec8(v)`: `(v ^ 0x9A).wrapping_sub(9)`
  - `dec16(v)`: `(v ^ 0xEFC3).wrapping_sub(9)`
  - `dec32(v)`: `(v ^ 0x0B80_F4B4).wrapping_sub(9)`
  - `dec32s(v)`: `(v ^ 0x0B80_F4B4).wrapping_sub(109) as i32`

### 1.2 Lưu trữ kép trên `GameData`
Khi server khởi động (`src/main.rs:112` → `GameData::load` → `GameData::load_binary` tại `src/data/loader.rs:318-329`):
1. Đọc toàn bộ `Item.dat` vào `Vec<u8>`.
2. Gọi `ItemDatLoader::load(&item_bytes)` để tạo `self.item_defs: HashMap<u16, ItemDef>` (`src/data/loader.rs:67, 326`).
3. Duyệt toàn bộ `self.item_defs`, gọi `def.to_item()` để tạo thêm `self.items: HashMap<i64, tables::Item>` (`src/data/loader.rs:20, 327-329`).
4. **Cả hai `HashMap` đều được giữ song song trong RAM** suốt vòng đời server. Ngoài ra, `BattleService` (`src/battle/service.rs:2233`) còn gọi `.clone()` toàn bộ `self.data.items` vào `BattleData`.

---

## 2. Phân loại toàn bộ 54 fields của `ItemDef`

### Nhóm A — Thực sự được server sử dụng (18 nhóm field)

1. **Ánh xạ vào `tables::Item` (`src/data/loaders/item.rs:71-118`, `src/data/tables.rs:40-80`)**:
   - `id` (offset `22..24`, `u16`) → `Item.id`
   - `name_viscii` (offset `0..21`, `Vec<u8>`) → `Item.name` (dùng để gửi wire packet VISCII)
   - `level` (offset `83`, `u8`) → `Item.level`
   - `kind` (offset `21`, `u8`) → `Item.loai`
   - `fit_type` (offset `48`, `u8`) → `Item.equip_pos`
   - `element` (offset `98`, `u8`) → `Item.thuoctinh`
   - `element_value` (offset `99..103`, `i32`) → `Item.value`
   - `attr1_kind` (`32..34`), `attr1_value` (`38..42`), `attr2_kind` (`34..36`), `attr2_value` (`42..46`) → `Item.hp`, `sp`, `int1..fai1`, `int2..fai2`
   - `spare3` (`103..105`, `u16`) → `Item.rb_pet_from`
   - `suit_id` (`106..108`, `u16`) → `Item.rb_pet_to`
   - `role_count_index` (`109..111`, `u16`) → `Item.add_pet`
   - `compound_material` (`46`, `u8`) → `Item.compound_material`
   - `compound_rank` (`47`, `u8`) → `Item.compound_rank`
   - `garble` (hardcoded `None` ở `item.rs:271`; chỉ legacy `Items.txt` loader ở `src/data/loader.rs:642` mới tính) → `Item.garble`
   *(Lưu ý: `Item::wire_name_hex()` ở `tables.rs:77-79` hiện không có caller nào trong `src/`.)*

2. **Chỉ đọc trực tiếp từ `data.item_defs` (chưa có trong `tables::Item`)**:
   - `name` (`String` UTF-8): chỉ đọc tại `src/server/handlers/crafting.rs:237, 361` để format thông báo `sys_msg_frame` khi craft.
   - `price` (offset `84..88`, `u32`): đọc tại `src/server/handlers/shops.rs:179, 189` (`NPC_SHOP_SUB_BUY`).
   - `sell_price` (offset `88..92`, `u32`): đọc tại `src/server/handlers/shops.rs:378` (`NPC_SHOP_SUB_SELL`).
   *(Các chỗ khác đọc `item_defs` là `src/crafting/basic/validator.rs:48-55`, `src/crafting/basic/algorithm.rs:63-66, 110-115, 140-145`, `src/server/handlers/crafting.rs:413-416` — nhưng chúng chỉ dùng `id`, `kind`, `compound_material`, `compound_rank`, vốn đã có trong `tables::Item`.)*

### Nhóm B — Thuần phía Client (Visual / UI / Description) — 10 fields đang parse vô ích
Các field này được giải mã & cấp phát bộ nhớ khi load `Item.dat`, **nhưng không có bất kỳ chỗ nào trong `src/` đọc**:
- `description_viscii` (offset `115..370`, tối đa **254 bytes/record**): cấp phát `Vec<u8>`.
- `description` (offset `115..370`, tối đa **254 chars/record**): chạy vòng lặp `encoding::viscii_to_unicode` từng byte (`src/data/loaders/item.rs:262-265`) + cấp phát `String`.
- `icon_id` (`24..26`), `male_pic_id` (`28..30`), `female_pic_id` (`30..32`).
- `male_color_tints: [u32; 4]` (`50..66`), `female_color_tints: [u32; 4]` (`66..82`).
- `display_material` (`82`), `sort` (`111`), `btn_state` (`114`).
*(Ngoài ra `_large_icon_num` ở `26..28` và `_equip_limit` ở `92` cũng bị đọc rồi bỏ ngay ở `:182, :237`.)*

### Nhóm C — Hardcoded `0` / `None` trong `ItemDatLoader::load` (15 fields không có trong 370B PC `Item.dat`)
Không hề đọc từ `chunk`, chỉ gán cứng `0` tại `src/data/loaders/item.rs:301-329` nhưng vẫn chiếm bộ nhớ trong mọi `ItemDef`:
- `gender`, `restrict`, `skill_link`, `turn`, `gift_dot`, `spare2`, `direct_use`, `male_equip_switch`, `female_equip_switch`, `durable`, `furnace_kind`, `furnace_count`, `quality`, `auction_tag`, `auction_sub_tag` (và `garble: None`).
*(Lưu ý: `doben` trên `InventoryItem` hiện đang gán cứng `100` ở `quest.rs:374, 387`, `system.rs:154`, `battle/service.rs:222`.)*

### Nhóm D — Fields gameplay có trong 370B `Item.dat` nhưng `src/` chưa dùng (9 fields)
- `need_lv` (gán bằng `level` ở `:298`; `inventory.rs:347, 653` đang check `item.lv` lấy từ `Item.level`).
- `attr1_item` (`36`), `attr2_item` (`37`), `special_ability` (`49`), `open_used` (`93`), `threshold` (`94..98`), `restrict2` (`105`), `spare5` (`108`), `role_count_value` (`112..114`).

---

## 3. Đánh giá chi phí & điểm nghẽn hiện tại

1. **Lãng phí lớn nhất là `description` + `description_viscii` (`offset 115..370`)**:
   - Chiếm **255 / 370 bytes (~68.9%)** kích thước mỗi record trong `Item.dat`.
   - `ItemDatLoader::load` thực hiện ~8,372 lần: cắt slice, `to_vec()`, `.reverse()`, map `viscii_to_unicode` từng byte, cấp phát `String` + `Vec<u8>` (~16,744 heap allocations) dù server **không bao giờ** gửi hay đọc mô tả item.
2. **Nhân đôi dữ liệu giữa `GameData.item_defs` và `GameData.items`**:
   - Server giữ cả `HashMap<u16, ItemDef>` (54 fields) và `HashMap<i64, tables::Item>` (29 fields, trong đó `name_viscii` bị `.clone()` sang `Item.name`).
   - `tables::Item` dùng `i64` cho toàn bộ 27 field số (`27 × 8 = 216 bytes` chưa kể `Vec<u8>` + `Option<GarbleSpec>`), trong khi dữ liệu gốc chỉ là `u8`/`u16`/`i32`.
   - `BattleService` (`src/battle/service.rs:2233`) clone toàn bộ `HashMap<i64, Item>` cho `BattleData`.
3. **Web Admin (`src/web/`) không dùng `items` hay `item_defs`**, nên không cần `name: String` UTF-8 thường trú cho tất cả ~8,372 items — chỉ `crafting.rs` cần tên UTF-8 khi báo kết quả craft.

---

## 4. Kế hoạch tối ưu đề xuất

### Pha 1 — Cắt bỏ decode & cấp phát các cột client-only (An toàn, tác động nhỏ)
- **Mục tiêu**: Giảm mạnh CPU & heap memory khi load `Item.dat` mà chưa cần đổi cấu trúc `GameData` ở các module khác.
- **Hành động**:
  1. Tách hoặc bổ sung phương thức nạp gọn cho server (ví dụ `ItemDatLoader::load_server(bytes: &[u8])` trả về `ServerItemDef` hoặc chỉ nạp các field cần thiết), **bỏ qua hoàn toàn**:
     - `offset 115..370` (`description`, `description_viscii`) — bỏ luôn `viscii_to_unicode` loop và 2 heap allocations/record.
     - `offset 24..32` (`icon_id`, `_large_icon_num`, `male_pic_id`, `female_pic_id`).
     - `offset 50..83` (`male_color_tints`, `female_color_tints`, `display_material`).
     - `offset 111`, `114` (`sort`, `btn_state`) và các field hardcoded `0` không dùng.
  2. Giữ `ItemDatLoader::load` đầy đủ nếu cần cho test/tool offline, hoặc chỉ giữ các field mà test hiện tại kiểm tra (`tests/compound_test.rs:61-84`, `tests/economy_inventory/inventory_template_hydrate.rs:189-235`).

### Pha 2 — Hợp nhất `item_defs` và `items` trên `GameData`
- **Mục tiêu**: Loại bỏ việc giữ song song 2 `HashMap` (~8,372 entries mỗi map) và tránh clone `HashMap<i64, Item>` trong `BattleService`.
- **Hành động**:
  1. Bổ sung `price: u32` và `sell_price: u32` vào `tables::Item` (hoặc tạo struct `ItemTemplate` dùng kiểu dữ liệu gọn `u16`/`u8`/`i32` thay `i64` ở những nơi nội bộ cho phép).
  2. Thay `def.name` ở `src/server/handlers/crafting.rs:237, 361` bằng helper giải mã on-demand từ `item.name` (VISCII `&[u8]` → UTF-8 `String` chỉ tại thời điểm craft) → không cần lưu `name: String` cho cả 8,372 items.
  3. Chuyển `src/crafting/basic/validator.rs`, `src/crafting/basic/algorithm.rs`, `src/server/handlers/crafting.rs`, `src/server/handlers/shops.rs` sang đọc từ `data.items` thay vì `data.item_defs`.
  4. Xóa `item_defs` khỏi `GameData` (sau khi cập nhật các test ở `tests/compound_test.rs`, `tests/economy_inventory/economy_wire.rs`, `tests/economy_inventory/pet_equip_unequip.rs`).
  5. Dùng `Arc<HashMap<i64, Item>>` (hoặc `Arc<ItemTable>`) trong `BattleService` (`src/battle/service.rs:2233`) thay vì `self.data.items.clone()`.

### Pha 3 — Kiểm thử & đảm bảo chất lượng
- Thêm unit tests riêng cho `ItemDatLoader` tại `tests/data_loaders/` (hiện thư mục này chưa có test cho `Item.dat`).
- Chạy `cargo test --test data_loaders_test`, `cargo test --test compound_test`, `cargo test --test economy_inventory_test`, `cargo test --test architecture_guardrails_test`, và cuối cùng `cargo test --all-targets --no-fail-fast`.
- Đo lại thời gian load `Data/Item.dat` và RSS memory trước/sau khi áp dụng Pha 1 & Pha 2.
