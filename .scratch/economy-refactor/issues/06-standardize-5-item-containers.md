# 06 — Standardize 5 Item Containers

Status: completed
Blocked by: 01

---

## 1. Mục tiêu
Chuẩn hóa kiến trúc và giao thức truyền thông của 5 Container vật phẩm trong game theo đúng kết quả giải mã mã máy Client Delphi:
1. **Hòm đồ (`homdo`, storagetype = 1)**: Túi chính 25 ô (`1..=25`).
2. **Tiền trang (`tientrang`, storagetype = 2)**: Rương kho lớn 50 ô (`1..=50`).
3. **Túi đeo (`tuideo`, storagetype = 4)**: Ba Đậu Yêu Balo 25 ô (`1..=25`).
4. **Lưu Lãng (`luulang`, storagetype = 16)**: Ba Đậu Yêu Lưu Lãng / Kho xe ngựa 10 ô (`1..=10`).
5. **Trang bị (`trangbi`, storagetype = 8)**: Đồ đang mặc 6 ô (`1..=6`).

---

## 2. Đặc tả Kỹ thuật & Wire Layout

### 2.1 Tiền trang đồ (`tientrang`, 50 ô)
- **Mass-Clear `1E 08`**: Do handler client `FUN_007707a0` không có clear pass tự động, server bắt buộc phải gửi frame `F4 44 02 00 1E 08` để dọn sạch 50 ô trước khi dump danh sách `1E 01`.
- **Rút đồ khỏi Tiền trang**: Gửi S→C `F4 44 04 00 1E 07 <slot: u8> <qty: u8>` (thay thế lệnh nhầm `1E 05`).
- **Thêm đồ vào Tiền trang**: Gửi S→C `1E 06 <itemId: u16 LE> <qty: u8> <attrs: 4B> <exp: u32 LE>` (thay thế lệnh nhầm `1E 04`).

### 2.2 Ba Đậu Yêu Lưu Lãng (`luulang`, 10 ô)
- **Giới hạn cứng**: Sức chứa tối đa đúng **10 ô (`1..=10`)**, mảng client `player+0xF74` (`form_Horse`). Cấm dùng `HOMDO_SLOTS=25` cho Lưu lãng (nếu gửi slot > 10 client sẽ gọi `_BoundErr` gây crash).
- **Giao thức Cất / Rút**:
  - C→S `17 33` (Cất vào Lưu lãng): Túi trừ `17 09 <slot> <qty>`, Lưu lãng cập nhật qua S→C `17 66` (hoặc `17 67`).
  - C→S `17 34` (Rút khỏi Lưu lãng): Túi thêm `17 08 <item>`, Lưu lãng trừ qua S→C `F4 44 03 00 17 68 <slot: u8>`.
- Khẳng định: `17 66`, `17 67`, `17 68` là opcode chuẩn của Client Delphi, giữ lại và bảo đảm đúng stride 12 byte có byte slot.

### 2.3 Trang bị (`trangbi`, 6 ô)
- Record serialization dài đúng **10 byte, không có byte slot**:
  `[itemId: u16 LE][qty: u8][long_val: u8][biased_giatri_long: u8][khang: u8][texp: u32 LE]`.
- Danh sách trang bị phải được sắp xếp theo thứ tự `slot` tăng dần (1..6) trước khi xuất wire.

---

## 3. Tiêu chí Nghiệm thu (Acceptance Criteria)
1. Dump Tiền trang đồ luôn phát frame `F44402001E08` trước `1E 01`.
2. Thao tác rút đồ Tiền trang phát `1E 07`, không phát `1E 05`.
3. Sức chứa Lưu Lãng không bao giờ vượt quá 10 ô; thao tác rút đồ Lưu lãng phát `17 68 <slot>`.
4. Trang bị serialize đúng 10 byte/record và không chứa byte slot.
5. `cargo check --all-targets` biên dịch không lỗi, không warning.

---

## Comments
- Updated `src/server/handlers/trade_storage.rs`:
  - `add_storage_item` now enforces container capacity (50 for tientrang, 10 for luulang).
  - TienTrang withdraw emits `1E 07 <slot> <qty>`, TienTrang deposit emits `1E 06 <entries>`.
  - LuuLang withdraw emits exact 3-byte `F44403001768<slot: u8>`.
- Updated `src/server/response.rs`: `send_storage_items` sends `1E 08` mass-clear immediately before `1E 01` dump.
- Confirmed `dump_trangbi` serializes 10 bytes/record without slot byte, sorted by slot ascending (1..=6).

