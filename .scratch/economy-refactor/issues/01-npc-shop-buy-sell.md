# 01: NPC Shop Buy/Sell Flow & Dialogue Menu Bridge

**What to build:** Khi người chơi nói chuyện với NPC bán hàng (qua Opcode `0x18`) và chọn menu "Cửa hàng", client nhận được gói `1B 03` mở giao diện mua bán `TThingSaleForm`. Người chơi có thể mua vật phẩm (`1B 01 <index> <count>`) với giá và mã vật phẩm tra cứu tự động từ dữ liệu `scene_eve_data` và `item_defs`, hoặc bán vật phẩm trong túi (`1B 02 <slot> <qty>`) nhận tiền theo `sell_price`. Các phản hồi lỗi và thành công tuân thủ đúng bảng 12 mã VISCII nội bộ (`1B 01 xx`, `1B 02 xx`), cập nhật vàng bằng `1A 01`/`1A 02` và đồng bộ túi đồ bằng `17 06`/`17 09`. Giữ lại nhánh fallback đặc biệt cho NPC ID 7 Map 9999 kèm ghi chú rõ ràng.

**Blocked by:** None (can start immediately).

**Status:** completed

- [x] Người chơi bấm chọn menu "Cửa hàng" (choice 30/32 trong `talk.rs`) nhận được gói `F44402001B03` mở `TThingSaleForm`.
- [x] Phân biệt rạch ròi nhánh Mua (`ctx.sub == 1`) và Bán (`ctx.sub == 2`) trong `handle_npc_shop`.
- [x] Mua vật phẩm: Đọc đúng `item_index: u8` và `count: u16 LE` (không ép count = 1). Tra cứu `data.scene_eve_data` kết hợp `data.item_defs` để xác định `price` và `item_id`.
- [x] Bán vật phẩm: Đọc `slot: u8` và `qty: u8`. Cộng tiền theo `qty * sell_price` từ `data.item_defs` và giới hạn trần vàng `9,999,999`.
- [x] Wire sequence sau khi mua: gửi `1A 02 <cost>`, `1A 04 <gold> <shop_point>`, `17 06 <item_id> <count> ...`, và `1B 01 00` (thành công im lặng).
- [x] Wire sequence sau khi bán: gửi `1A 01 <revenue>`, `1A 04 <gold> <shop_point>`, `17 09 <slot> <qty>`, và `1B 02 00`.
- [x] Loại bỏ hoàn toàn việc gửi sai frame đỏ `02 0B` (`red_message`), thay thế bằng các mã kết quả chuẩn `1B 01 xx` và `1B 02 xx` (ví dụ: `1B 01 01` khi thiếu vàng, `1B 01 02` khi túi đầy, `1B 02 06` khi vàng đầy).
- [x] Giữ lại nhánh fallback tặng vật phẩm tân thủ cho NPC ID 7 Map 9999 kèm chú thích/doc comment chi tiết.
- [x] Cập nhật comment trong `src/server/dispatcher.rs` đính chính việc client gửi opcode `0x1B`.
