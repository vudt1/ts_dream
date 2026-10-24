# 04 — NPC Shop Bulk Buy Parity & Menu Segregation

Status: completed
Blocked by: 01

---

## 1. Mục tiêu
Chuẩn hóa luồng Mua và Bán với Shop NPC (`0x14` và `0x1B`):
- Nâng trần mua hàng C→S `1B 01` lên tới 1.250 món (`count: u16 LE`), xóa bỏ giới hạn ép cứng `count <= 255`.
- Triển khai thuật toán tính toán ô trống **All-or-Nothing**: Tính trước `(count + 49) / 50` ô trống; nếu không đủ túi, từ chối toàn bộ với mã lỗi `NPC_SHOP_BUY_BAG_FULL` (`1B 01 02`).
- Phân tách hàng mua vào nhiều ô trong túi (mỗi ô tối đa 50 món).
- Tại hội thoại NPC (Menu 30 - Mua hàng): Chỉ phát S→C `14 01` (`B2 = 6`), loại bỏ hoàn toàn việc gửi nhầm `1B 03` (mở form bán) làm đè hai form lên nhau.

---

## 2. Đặc tả Kỹ thuật & Wire Layout

### 2.1 C→S `1B 01` (Mua hàng từ Shop NPC)
- Payload: `1B 01 <item_index: u8> <count: u16 LE>` (5 byte).
  - `item_index`: Chỉ số 0-based trong danh mục shop của surface NPC (`0..count-1`).
  - `count`: 1..1250 (nếu `count == 0` trả lỗi `NPC_SHOP_BUY_DISRUPTED` `1B 01 04`, nếu `count > 1250` trả lỗi `NPC_SHOP_BUY_QTY_FULL` `1B 01 03`).
- Quy tắc All-or-Nothing:
  - Tra cứu thông tin item từ `item_defs` theo ID trong surface. Đơn giá mua = `item.price_buy`.
  - `total_cost = price_buy * (count as u32)`.
  - Nếu `player_gold < total_cost`: Trả về S→C `F4 44 03 00 1B 01 01` (`NPC_SHOP_BUY_INSUFFICIENT_GOLD`).
  - Tính số ô cần thiết: `slots_needed = (count + 49) / 50`.
  - Đếm số ô trống trong túi đồ 25 ô: `empty_slots`. Nếu `empty_slots < slots_needed`: Trả về S→C `F4 44 03 00 1B 01 02` (`NPC_SHOP_BUY_BAG_FULL`).
- Thực thi Mua hàng:
  - Khấu trừ `player_gold -= total_cost`.
  - Phân bổ `count` vào các ô trống: Chia thành các chunk 50 món (chunk cuối lấy phần dư `count % 50`).
  - Gửi các frame S→C:
    1. Trừ vàng: `1A 02 <total_cost: u32 LE>`
    2. Cập nhật vàng tuyệt đối: `1A 04 <new_gold: u32 LE>` (6 byte)
    3. Thêm từng stack vào túi: Mỗi stack gửi 1 frame `17 06 <itemId: u16 LE> <stackCount: u16 LE> ...` (14 byte)
    4. Xác nhận thành công: `1B 01 00`

### 2.2 Phân tách Menu Hội thoại NPC
- Tại `src/server/handlers/talk.rs`: Khi người chơi click lựa chọn Menu 30 (Mua hàng):
  - Xóa dòng code gửi `open_npc_shop_frame()` (`1B 03`).
  - Chỉ gửi frame S→C `14 01` với `B2 = 6` và `W3 = shop_surface_id`. Client Delphi sẽ tự động đọc `Eve.emg` và `Item.Dat` để mở duy nhất `TInterForm`.
  - Frame `1B 03` chỉ được gửi khi mở Menu Bán hàng hoặc sự kiện bán đồ riêng biệt.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. Mua 100 món (2 ô 50) thành công khi túi có đủ 2 ô trống, phát ra 2 gói tin `17 06`.
2. Mua 100 món bị reject bằng `1B 01 02` nếu túi chỉ còn 1 ô trống duy nhất (All-or-Nothing), vàng được bảo toàn 100%.
3. Mua tới 1.250 món thành công khi túi có đủ 25 ô trống.
4. Menu 30 trong hội thoại NPC chỉ mở form mua `14 01`, không mở form bán `1B 03`.
5. `cargo check --all-targets` biên dịch không lỗi, không warning.

---

## Comments
- Implemented bulk buy in `src/server/handlers/shops.rs`: count up to 1250, All-or-Nothing slot calculation `slots_needed = (count + 49) / 50`, chunk into 50-item stacks, sending `1A 02`, `1A 04`, multiple `17 06`, and `1B 01 00`.
- Eliminated spurious `open_npc_shop_frame()` (`1B 03`) on Menu 30 selections in `src/server/handlers/talk.rs`.
- Verified clean compilation with `cargo check --all-targets`.

