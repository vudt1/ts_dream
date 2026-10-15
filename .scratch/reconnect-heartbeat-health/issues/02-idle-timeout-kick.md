# 02: Ngắt kết nối không hoạt động sau thời gian chờ, có thông báo lý do

**What to build:** Kết nối không có dữ liệu trong khoảng idle timeout bị đóng. Trước khi đóng, client nhận thông báo "mất kết nối" (sub 13) và frame này phải đến được socket. Người chơi đã đăng nhập được dọn sạch (broadcast ẩn, gỡ đăng ký, lưu dữ liệu) như khi thoát bình thường.

**Blocked by:**
- 01: Nền tảng liveness

**Status:** resolved

- [x] Socket im lặng quá thời gian chờ nhận được thông báo sub 13 trước khi bị đóng
- [x] Frame thông báo được gửi xong trước khi socket đóng (không bị mất do đóng sớm)
- [x] Sau khi đóng, người chơi không còn trong danh sách kết nối và không còn online
- [x] Dữ liệu người chơi được lưu một lần qua đường teardown có sẵn
- [x] Lỗi đọc socket được ghi log ở mức debug thay vì bị nuốt
- [x] Test integration với TCP thật, dùng thời gian chờ rút ngắn
