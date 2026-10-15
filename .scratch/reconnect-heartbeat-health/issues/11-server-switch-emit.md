# 11: Server chủ động chuyển người chơi sang cổng game khác (opcode 0x2E)

**What to build:** Quản trị viên có thể ra lệnh chuyển người chơi sang kết nối mới trên cổng game. Client mở lại socket và đăng nhập lại trên kết nối mới. Đây là đường reconnect do server điều khiển.

**Blocked by:**
- 04: Đăng nhập lại khi phiên cũ đã chết
- 07: Đăng nhập lại trên cùng kết nối không làm treo client
- 08: Mọi đường ngắt kết nối do server chủ động đều gửi lý do

**Status:** needs-triage

> **Deferred theo Q5:** đưa vào đợt sau, sau khi 01–08 ổn định. Không thuộc phạm vi sửa lỗi reconnect/health hiện tại.

- [ ] Frame chuyển server được phát đúng định dạng với hai byte tham số hợp lệ
- [ ] Lệnh GM kích hoạt chuyển server cho đúng người chơi mục tiêu
- [ ] Client mở lại kết nối và đăng nhập thành công trên kết nối mới (kiểm chứng thủ công với client)
- [ ] Opcode `0x2E` vẫn không được xử lý khi client gửi lên (chiều một chiều server đến client)
