# 02 — Player Stall Open & Catalog Wire Parity

Status: completed
Blocked by: 01

---

## 1. Mục tiêu
Sửa toàn diện lỗi vỡ cấu trúc wire trong tính năng Quầy sạp cá nhân (Stall - `0x17` subs `1E..22`):
- Sửa parser C→S `ITEM_SUB_STALL_OPEN` (`17 1E`): đọc 100 byte danh mục trước, sau đó mới đọc tên bảng hiệu.
- Sửa S→C phản hồi mở sạp: Gửi đúng 3 byte `F4440300171E01` (thay vì frame chuỗi dài tự chế) để client Delphi đóng form thiết lập sạp.
- Giữ nguyên cấu trúc catalog S→C `17 21` (Header 18B + records 16B) và chuẩn hóa C→S `17 21` mua hàng.

---

## 2. Đặc tả Kỹ thuật & Wire Layout

### 2.1 C→S `17 1E` (Mở sạp bày bán)
- Wire payload client gửi:
  - Byte `0..99`: Mảng 10 ô cố định (mỗi ô 10 byte):
    - Offset `+0`: `bagSlot: u8` (1..25, nếu ô trống thì bằng 0).
    - Offset `+1`: `qty: u8` (số lượng bán).
    - Offset `+2..5`: `price: u32 LE` (đơn giá vàng).
    - Offset `+6..9`: `vitality: u32 LE` (hoặc 4 byte padding/exp).
  - Byte `100`: `name_len: u8` (độ dài chuỗi tên bảng hiệu).
  - Byte `101..100+name_len`: Chuỗi tên bảng hiệu VISCII 1.1 (`boardName`).
  - Byte tiếp theo: `cardIndex: u8` và `cardPrice: u32 LE`.
- Server parse: Lấy danh sách tối đa 10 item từ byte 0..99 (bỏ qua các ô có `bagSlot == 0` hoặc `qty == 0`), sau đó trích xuất `boardName`.
- Phản hồi S→C:
  - Thành công: Gửi đúng 3 byte `F4 44 03 00 17 1E 01` (`ITEM_SUB_STALL_OPEN` + `0x01` success).
  - Thất bại (đang trong trận đánh, bận, hoặc tên vi phạm): Gửi `F4 44 03 00 17 1E 02` (hoặc `03`).

### 2.2 S→C `17 21` (Catalog người khác xem sạp) & C→S Mua hàng
- S→C `17 21`: Header 18 byte (`17 21 <vendorUid: u32 LE> 12_zeros`) + `N` records 16 byte (`[slot: u8][itemId: u16 LE][qty: u8][long_val: u8][price: u32 LE][biased_giatri_long: u8][khang: u8][pad: u8][texp: u32 LE]`).
- C→S `17 21 <vendorUid: u32 LE> <stallSlot: u8> <buyCount: u8>`:
  - `stallSlot` là chỉ số 1-based trên sạp (1..10).
  - Server tra cứu `shop_index.checked_sub(1)` trên danh sách sạp.
  - Trừ tiền người mua, cộng tiền người bán (clamp `GOLD_CAP`), chuyển item vào túi người mua.
  - Nếu số lượng listing về 0: Gỡ bỏ item khỏi sạp. Nếu sạp hết sạch hàng: Đóng sạp và phát thông báo đóng sạp `17 1F`.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. Parser `ITEM_SUB_STALL_OPEN` đọc chính xác 10 ô item từ 100 byte đầu tiên mà không bị lỗi crash do đọc nhầm `name_len`.
2. S→C phản hồi mở sạp trả về đúng 3 byte `F4440300171E01`.
3. Mua hàng từ sạp thực hiện trừ tiền và chuyển đồ nguyên tử, listing gỡ bỏ chính xác khi số lượng về 0.
4. `cargo check --all-targets` biên dịch không lỗi, không warning.

---

## Comments
- Đã viết lại parser `ITEM_SUB_STALL_OPEN` trong `src/server/handlers/shops.rs`: đọc mảng 10 ô (100 byte) trước, sau đó trích xuất `name_len`, `boardName`, và `image`.
- Chuẩn hóa phản hồi mở sạp thành frame 3 byte `F4440300171E01` (thành công) hoặc `F4440300171E02` (thất bại).
- Bổ sung kiểm tra đơn giá listing không được vượt quá `GOLD_CAP` (9.999.999) chống tràn số.
- Chuẩn hóa frame đóng sạp S->C `17 20` thành 7 byte (`F44407001720<id>00`).
- Tự động đóng sạp và phát thông báo đóng sạp khi người mua mua hết các món hàng trên quầy.
- Tích hợp Two-Phase State Lock `lock_player_operations([buyer_id, seller_id])` trong `ITEM_SUB_STALL_BUY` trước khi query và hoàn tất mua sạp.
- Cập nhật test `tests/p0_p1_p2_roadmap_test.rs` và `tests/economy_wire_test.rs`, toàn bộ test pass.
