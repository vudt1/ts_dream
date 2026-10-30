# 03: Tích Hợp Giao Thức Mạng Hợp Thành Căn Bản & Giao Dịch Cơ Sở Dữ Liệu

**What to build:** 
Tính năng hợp thành hoạt động trơn tru từ giao diện người chơi đến cơ sở dữ liệu: tiếp nhận gói tin yêu cầu hợp thành gửi lên từ bảng giao diện của túi đồ, kiểm tra quyền sở hữu và số lượng thực tế trong hòm đồ chính, từ chối an toàn khi túi đồ đầy, thực hiện trừ nguyên liệu và cấp phát vật phẩm mới trong một giao dịch cơ sở dữ liệu nguyên tử theo mô hình khóa hai pha, đồng thời phát đầy đủ các gói tin phản hồi chuẩn về âm thanh rèn đúc, cập nhật ô chứa túi đồ và thông báo hệ thống chúc mừng người chơi.

**Blocked by:** 02: Động Cơ Hợp Thành Căn Bản Thuần Túy & Logic Fallback Cấp 1

**Status:** completed

- [x] Điều phối chính xác mã lệnh gói tin hợp thành từ bộ phân luồng chính sang trình xử lý hợp thành chuyên biệt.
- [x] Ràng buộc phạm vi ô chứa nghiêm ngặt: chỉ chấp nhận các ô thuộc hòm đồ chính của nhân vật (từ 1 đến 25), từ chối mọi ô ngoài phạm vi.
- [x] Áp dụng cơ chế kiểm tra ô trống trước khi trừ nguyên liệu: nếu túi đồ đầy 25/25 ô, từ chối giao dịch ngay lập tức với thông báo lỗi thích hợp.
- [x] Thực thi cơ chế khóa hai pha (Two-Phase State Lock) và mở giao dịch ghi nguyên tử trên cơ sở dữ liệu: trừ/xóa các ô nguyên liệu đã tiêu hao và đặt thành phẩm mới vào ô trống đầu tiên.
- [x] Phát gói tin âm thanh rèn đúc chuẩn để ứng dụng người chơi phát hiệu ứng âm thanh đặc trưng.
- [x] Phát gói tin cập nhật/xóa số lượng ô nguyên liệu và gói tin cấp phát thành phẩm mới cho ứng dụng người chơi.
- [x] Gửi thông báo chữ qua kênh thông điệp hệ thống chuẩn thông báo tên món đồ người chơi vừa tạo thành.
- [x] Có bài kiểm thử tích hợp mạng tại thư mục kiểm thử tập trung mô phỏng trọn vẹn luồng từ gói tin yêu cầu đến các gói tin phản hồi và xác minh trạng thái cơ sở dữ liệu.
