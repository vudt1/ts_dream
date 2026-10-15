# 08: Mọi đường ngắt kết nối do server chủ động đều gửi lý do cho client

**What to build:** Khi server ngắt người chơi (trùng đăng nhập, bị đình chỉ, GM kick, mất kết nối không hoạt động), client nhận thông báo có lý do trước khi socket đóng, thay vì bị treo.

**Blocked by:**
- 02: Ngắt kết nối không hoạt động

**Status:** resolved

- [x] Trùng đăng nhập gửi thông báo sub tương ứng trước khi đóng
- [x] Tài khoản bị đình chỉ gửi thông báo đình chỉ trước khi đóng
- [x] Lệnh kick của GM gửi thông báo trước khi đóng
- [x] Không có đường ngắt nào đóng socket mà không gửi lý do
- [x] Test integration xác nhận frame lý do đến được socket
