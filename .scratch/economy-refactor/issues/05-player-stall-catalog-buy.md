# 05: Player Stall Vendor & Catalog Browsing

**What to build:** Xử lý toàn diện tính năng mở quầy bán hàng (Stall / Vendor) và mua hàng giữa người chơi qua Opcode `0x17`: Mở quầy (`17 1E`), Đóng quầy (`17 1F`), Xem quầy (`17 20`), Mua từ quầy (`17 21 <vendor_uid: u32 LE> <slot: u8> <count: u8>`), và Làm mới quầy (`17 22`). Khung tin danh mục S→C `17 21` tuân thủ chính xác cấu trúc Header 17 bytes và các Record 16 bytes giải mã từ `FUN_00610290`. Giao dịch mua hàng thực hiện trừ tiền người mua, cộng tiền người bán (clamp `9,999,999`), chuyển vật phẩm nguyên tử và gửi đầy đủ gói cập nhật cho cả hai phiên kết nối.

**Blocked by:** 01 (NPC Shop Buy/Sell Flow & Dialogue Menu Bridge), 02 (Currency Model & Bank Gold Sync at Login).

**Status:** completed

- [x] Chuẩn hóa hàm `player_shop_catalog_frame` trong `src/server/handlers/shops.rs`: Header 17B (`17 21` + `vendor_uid: u32 LE` + 12B pad zero) và danh sách tối đa 10 vật phẩm, mỗi vật phẩm đúng 16B (`slot`, `itemId`, `count`, `long`, `price`, `giatri_long`, `khang`, `pad`, `texp`).
- [x] Xử lý C→S `17 20 <vendor_uid: u32 LE>`: Khi người chơi click vào người đang mở quầy, server phát gói S→C `17 21` mở giao diện `TCY_BuyVenderMenu` trên client.
- [x] Xử lý C→S `17 21 <vendor_uid: u32 LE> <slot: u8> <count: u8>`:
  - Kiểm tra quầy người bán còn mở, còn hàng, và người mua đủ vàng.
  - Kiểm tra trần vàng người bán không bị vượt quá `9,999,999`.
  - Thực hiện giao dịch nguyên tử 2 người chơi (cập nhật DB `character_money` và `inventories` cho cả 2 tài khoản).
- [x] Gửi gói cập nhật cho người mua: `gold_frame(gold, shop_point)`, làm mới danh mục `player_shop_catalog_frame`, và gói thêm đồ vào túi `equip_item_frame(17 06)`.
- [x] Gửi gói cập nhật cho người bán: `gold_frame(seller_gold, seller_shop_point)` và gói gỡ/trừ đồ trên quầy `17 09 <slot> <count>`.
- [x] Xử lý C→S `17 1E` (Mở quầy broadcast trong bản đồ) và `17 1F` (Đóng quầy).
