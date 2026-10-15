# 09: Tắt server đóng thật các kết nối đang mở

**What to build:** Khi server dừng, mọi kết nối người chơi đang mở được đóng thật sự và người chơi đang online được dọn đúng cách, thay vì chỉ xoá danh sách trong bộ nhớ và để socket mở.

**Blocked by:**
- 02: Ngắt kết nối không hoạt động

**Status:** resolved

- [x] Sau khi dừng server, không còn socket người chơi nào mở
- [x] Người chơi đang trong thế giới được lưu dữ liệu qua đường teardown
- [x] Danh sách kết nối và danh sách online rỗng sau khi dừng
- [x] Test integration với TCP thật xác nhận socket đã đóng
