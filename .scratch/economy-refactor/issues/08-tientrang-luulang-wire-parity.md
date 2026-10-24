# 08: Tiền Trang & Lưu Lãng Wire Parity theo Binary Delphi (0x1E & 17 67)

**What to build:** 
Chuẩn hóa giao thức wire S→C cho 2 container Tiền Trang (50 ô) và Ba Đậu Yêu Lưu Lãng (10 ô) đúng 100% theo mã máy thực tế của Client Delphi `aLogin.exe` (Dispatcher `FUN_007921a6` tại VA `0x007921A6`):
1. **Tiền Trang Cất đồ**: Khi người chơi gửi đồ vào Tiền Trang (`OP_STORAGE` sub `STORAGE_SUB_DEPOSIT`), server phản hồi S→C `1E 02 <itemId: u16 LE> <qty: u8> <attrs: 4B> <exp: u32 LE>` (tổng frame 13 byte `F4440D001E02...`, đúng 11 byte item data không kèm byte slot). Client Delphi sẽ gọi `FUN_00770454` -> `FUN_0072be9c` để tự tìm ô trống bằng `FUN_0076ff98()`. Thay thế và xóa bỏ hoàn toàn việc gửi nhầm `1E 06` (vốn là No-op / Dead branch trên Delphi client). Làm rõ ranh giới: `1E 02` là lệnh thêm đồ cho Tiền Trang 50 ô, phân biệt hoàn toàn với `1E 04` vốn dành riêng cho Kho nhà trọ/thương nhân.
2. **Tiền Trang Rút đồ**: Khi người chơi rút đồ từ Tiền Trang (`OP_STORAGE` sub `STORAGE_SUB_WITHDRAW`), server phản hồi S→C `1E 05 <slot: u8> <qty: u8>` (tổng frame 4 byte `F44404001E05...`). Client Delphi sẽ gọi `FUN_0077021c` để trừ vật phẩm tại ô `slot` chỉ định trong 50 ô (`player+0x0A48`). Thay thế và xóa bỏ việc gửi `1E 07` (vốn là No-op trên Delphi client).
3. **Lưu Lãng Cất đồ**: Khi người chơi cất đồ vào Lưu Lãng (`OP_ITEM` sub `ITEM_SUB_WAREHOUSE_DEPOSIT`), server phản hồi S→C `17 67 <12B item>` (chứa byte slot) để cập nhật ô đồ theo đúng quy định tại ADR 0005 §4; bảo toàn `17 66` chỉ dùng khi dump ban đầu toàn bộ container.
4. **Tài liệu & Test Parity**: Cập nhật các bảng tham chiếu trong `.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md` và `docs/adr/0005-economy-wire-parity-and-transactions.md` phản ánh đúng bảng nhảy `0x007921D5`; cập nhật assertion kiểm thử trong `tests/economy_wire_test.rs` đạt 100% PASS.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Sửa `src/server/handlers/trade_storage.rs`: Thay thế lệnh gửi `1E 06` trong luồng deposit Tiền Trang bằng `1E 02` chứa đúng 11 byte dữ liệu vật phẩm (`itemId: u16 LE`, `qty: u8`, `doben: u8`, `long_val: u8`, `giatri_long: u8`, `khang: u8`, `texp: u32 LE`), không chứa byte slot.
- [x] Sửa `src/server/handlers/trade_storage.rs`: Thay thế lệnh gửi `1E 07` trong luồng withdraw Tiền Trang bằng `1E 05 {:02X}{:02X}` (`slot: u8`, `count: u8`).
- [x] Sửa `src/server/handlers/trade_storage.rs`: Thay thế lệnh gửi `17 66` trong luồng deposit Lưu Lãng bằng `17 67 <12B item>` theo đúng ADR 0005 §4.
- [x] Cập nhật bảng mục 6.5.5 và sequence diagram 6.6.7 trong `.scratch/economy-refactor/ECONOMY_CLIENT_SPEC.md` ghi nhận đúng subcode `1E 02` (Tiền trang 50 ô) và `1E 05` (Rút Tiền trang).
- [x] Cập nhật các assertion trong `tests/economy_wire_test.rs` kiểm tra chính xác frame `F4440D001E02...` (13 byte) và `F44404001E05...` (4 byte), cũng như `17 67` cho Lưu lãng.
- [x] `cargo check --all-targets` và `cargo test --test economy_wire_test` đạt 100% PASS (0 error, 0 warning).

## Implementation Notes
- Đã thêm helper `storage_entry_no_slot` định dạng chính xác 11-byte dữ liệu vật phẩm (`itemId: u16 LE`, `qty: u8`, `doben: u8`, `long_val: u8`, `giatri_long: u8`, `khang: u8`, `texp: u32 LE`).
- Deposit Tiền Trang phát sinh frame chuẩn 13-byte `F4440D001E02...` qua `frame("1E02", &storage_entry_no_slot(&item))` cho mỗi item.
- Withdraw Tiền Trang phát sinh frame 4-byte `F44404001E05{:02X}{:02X}`.
- Deposit Lưu Lãng phát sinh frame 14-byte `F4440E001767...` qua `frame("1767", &storage_entry(&item))` chứa slot byte.
- Đã sửa lỗi phân tích payload `handle_storage_transfer`: bóc tách cấu trúc 2-byte `[slot, qty]` chuẩn của Delphi thay vì duyệt từng byte làm slot index gây gửi nhầm slot phụ cận; hỗ trợ rút/cất số lượng lẻ theo đúng wire spec.
- Bổ sung `test_storage_partial_quantity_and_no_collateral_transfer`. Toàn bộ 18 integration tests trong `tests/economy_wire_test.rs` pass 100%.
