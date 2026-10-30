# 04: Tiếp Nhận & Cấp Phát Lông Vũ Form Lựa Chọn Bảo Vật

**What to build:** 
Người chơi khi tương tác với giao diện chọn bảo vật/lông vũ có thể nhấn chọn 1 trong 4 hệ nguyên tố (Địa, Thủy, Hỏa, Phong) và nhận chính xác viên Lông vũ mang hệ tương ứng vào túi đồ của mình; hệ thống từ chối an toàn kèm thông báo nhắc nhở nếu người chơi chưa chọn hệ hoặc nếu túi đồ không còn chỗ trống.

**Blocked by:** 01: Nạp & Chỉ Mục CSDL Compound và Sửa Lỗi Trường Danh Mục Vật Phẩm

**Status:** completed

- [x] Tiếp nhận và giải mã gói tin yêu cầu lựa chọn bảo vật gửi lên từ giao diện chọn lông vũ của ứng dụng người chơi.
- [x] Thẩm định giá trị lựa chọn hệ nguyên tố: hợp lệ với các mã tương ứng cho Địa, Thủy, Hỏa, Phong; từ chối xử lý nếu giá trị bằng 0.
- [x] Kiểm tra số ô trống trong hòm đồ chính của người chơi: từ chối nếu không còn ít nhất 1 ô trống.
- [x] Mở giao dịch cơ sở dữ liệu nguyên tử cấp phát đúng mã định danh của Lông vũ theo hệ đã chọn vào ô trống đầu tiên của hòm đồ.
- [x] Cập nhật đồng bộ trạng thái túi đồ trong phiên kết nối và phát gói tin thêm vật phẩm, gói âm thanh và thông báo hệ thống chúc mừng người chơi.
- [x] Có bài kiểm thử tích hợp tại thư mục kiểm thử tập trung xác minh luồng nhận lông vũ và kiểm tra các trường hợp từ chối hợp lệ.
