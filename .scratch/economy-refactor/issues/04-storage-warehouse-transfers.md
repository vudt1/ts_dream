# 04: Storage & Warehouse Item Transfers

**What to build:** Xử lý cất đồ vào kho và rút đồ khỏi kho cá nhân / kho lưu lãng (`TThingAccountForm` qua Opcode `0x1E` và `0x17` subs 51..52). Khắc phục triệt để lỗi hardcode số lượng `0x32` trên các gói tin wire, đảm bảo truyền đúng số lượng thực tế của vật phẩm khi gửi `1E 05 <slot> <count>`, `17 09 <slot> <count>`, `17 68 <slot> <count>`. Đồng bộ danh mục kho và hành trang nguyên tử vào cơ sở dữ liệu `inventories` (phân định `storagetype = 1` cho hành trang, `storagetype = 2` cho Tiền Trang, và `storagetype = 16` cho Lưu Lãng).

**Blocked by:** 01 (NPC Shop Buy/Sell Flow & Dialogue Menu Bridge).

**Status:** completed

- [x] Sửa lỗi hardcode chuỗi cố định `"32"` tại `src/server/handlers/trade_storage.rs` (dòng 677, 684, 701), thay thế bằng format hex `{:02X}` ứng với `item.count` thực tế.
- [x] Xử lý luồng Cất đồ vào Tiền Trang (`1E 02 <slot> <count>`): Trừ đồ trong túi (`17 09 <slot> <count>`), thêm đồ vào kho (`1E 04 <12B item record>`).
- [x] Xử lý luồng Rút đồ khỏi Tiền Trang (`1E 01 <slot> <count>`): Trừ đồ trong kho (`1E 05 <slot> <count>`), thêm đồ vào túi (`17 08 <12B item record>`), kết thúc bằng `17 32`.
- [x] Xử lý luồng Cất đồ vào Hòm đồ Lưu Lãng (`17 51 <slot> <count>`): Trừ túi (`17 09`), thêm kho lưu lãng (`17 66 <12B item record>`).
- [x] Xử lý luồng Rút đồ khỏi Hòm đồ Lưu Lãng (`17 52 <slot> <count>`): Trừ kho lưu lãng (`17 68 <slot> <count>`), thêm túi (`17 08`), kết thúc bằng `17 32`.
- [x] Đảm bảo giao dịch lưu chuyển vật phẩm giữa các kho được ghi-through nguyên tử vào bảng `inventories`, không để xảy ra mất đồ hoặc nhân bản vật phẩm khi có lỗi ngắt kết nối giữa chừng.
