# 07: Đăng nhập lại trên cùng kết nối không làm treo client

**What to build:** Gửi lệnh đăng nhập hai lần trên cùng một socket không làm client treo. Trạng thái "đã đăng nhập" và "đang trong thế giới" được đặt lại đúng. Trận đấu gửi thông tin đến đúng kết nối sau khi phiên được tải lại.

**Blocked by:**
- 04: Đăng nhập lại khi phiên cũ đã chết

**Status:** resolved

- [x] Đăng nhập lần hai trên cùng socket trả về bộ frame đăng nhập đầy đủ, không rỗng
- [x] Cờ đã đăng nhập và đang trong thế giới được reset khi đăng nhập lại
- [x] Trận đấu gửi tin đến kết nối hiện tại, không đến kết nối cũ
- [x] Test unit cho chuỗi đăng nhập hai lần và test integration cho trận đấu sau relog
