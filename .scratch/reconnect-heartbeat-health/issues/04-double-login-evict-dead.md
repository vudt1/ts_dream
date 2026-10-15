# 04: Đăng nhập lại khi phiên cũ đã chết: gỡ phiên cũ thay vì từ chối

**What to build:** Người chơi đăng nhập lại được ngay khi phiên cũ đã quá thời gian chờ không hoạt động (kết nối bị treo hoặc rớt không sạch). Phiên cũ được ngắt và lưu đúng cách trước khi phiên mới được đăng ký. Nếu phiên cũ còn hoạt động, đăng nhập mới vẫn bị từ chối với thông báo đăng nhập trùng.

**Blocked by:**
- 01: Nền tảng liveness
- 02: Ngắt kết nối không hoạt động

**Status:** resolved

- [x] Phiên cũ quá thời gian chờ bị ngắt và đăng nhập mới được chấp nhận
- [x] Phiên cũ bị ngắt đi qua đường teardown có lưu dữ liệu, không bỏ sót persist
- [x] Phiên cũ còn hoạt động vẫn nhận thông báo đăng nhập trùng và kết nối mới bị đóng
- [x] Hai đăng nhập đồng thời không gây race (kiểm tra và đăng ký là một thao tác nguyên tử)
- [x] Test integration cho cả hai nhánh: phiên cũ chết và phiên cũ còn sống
