# 03: Gia cố socket: tắt Nagle và giới hạn hàng đợi gửi

**What to build:** Gói tin nhỏ không bị trì hoãn bởi thuật toán gộp gói của TCP. Hàng đợi gửi đi của mỗi kết nối có giới hạn, để một client bị nghẽn không làm bộ nhớ phình vô hạn.

**Blocked by:**
- 02: Ngắt kết nối không hoạt động

**Status:** resolved

- [x] Kết nối mới được tắt Nagle
- [x] Hàng đợi gửi có giới hạn; khi đầy thì kết nối bị đóng có ghi log, không chặn toàn server
- [x] Giới hạn có thể cấu hình hoặc có giá trị mặc định được ghi trong tài liệu
- [x] Test xác nhận hàng đợi đầy không làm tăng bộ nhớ không giới hạn
