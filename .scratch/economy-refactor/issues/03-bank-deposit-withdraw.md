# 03: Bank Deposit & Withdrawal Operations

**What to build:** Xử lý đầy đủ hai chiều giao dịch tiền tệ tại Tiền Trang (Bank) qua Opcode `0x1D`: Rút tiền từ Tiền Trang về túi đồ (`1D 01 <amount: u32 LE>`) và Nộp tiền từ túi đồ vào Tiền Trang (`1D 02 <amount: u32 LE>`). Giao dịch kiểm tra trần `9,999,999` cho cả hai nguồn tiền, thực hiện cập nhật nguyên tử vào bảng `character_money`, và trả về các gói thông báo delta `1D 01`/`1D 02` kết hợp `1A 01`/`1A 02` cùng thông báo lỗi `1D 03` khi không hợp lệ.

**Blocked by:** 02 (Currency Model & Bank Gold Sync at Login).

**Status:** completed

- [x] Người chơi mở giao diện Tiền Trang `TMoneyAccountForm` qua gói `1D 05` khi nói chuyện với NPC Tiền Trang.
- [x] Xử lý C→S `1D 01 <amount: u32 LE>` (RÚT TIỀN): Kiểm tra `bank_gold >= amount` và `gold + amount <= 9,999,999`. Trừ `bank_gold`, cộng `gold`.
- [x] Xử lý C→S `1D 02 <amount: u32 LE>` (NỘP TIỀN): Kiểm tra `gold >= amount` và `bank_gold + amount <= 9,999,999`. Trừ `gold`, cộng `bank_gold`.
- [x] Thực hiện transaction nguyên tử cập nhật `character_money (gold, bankgold)` qua write pool SQLite. Rollback an toàn nếu có lỗi.
- [x] Gửi gói phản hồi S→C chính xác sau khi rút tiền: `1D 02 <amount: u32 LE>` (trừ bank) và `1A 01 <amount: u32 LE>` (cộng vàng túi đồ).
- [x] Gửi gói phản hồi S→C chính xác sau khi nộp tiền: `1D 01 <amount: u32 LE>` (cộng bank) và `1A 02 <amount: u32 LE>` (trừ vàng túi đồ).
- [x] Trả về gói `1D 03` khi số dư không đủ hoặc vượt quá dung lượng tối đa `9,999,999`.
