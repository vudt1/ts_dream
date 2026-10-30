# 05: Động Cơ Cường Hóa Khảm Lông & Thăng Cấp Thuộc Tính Trang Bị

**What to build:** 
Tính năng cường hóa trang bị cho phép người chơi sử dụng các loại Lông vũ mang thuộc tính ngũ hành để khảm vào trang bị trong hòm đồ chính: kích hoạt hệ nguyên tố cho trang bị trắng, nâng cấp cấp độ cường hóa từ +1 đến +10 theo bảng tỷ lệ thành công lũy giảm, xử lý an toàn khi thất bại (tụt 1 cấp ở mốc cao hoặc giữ nguyên ở mốc thấp), cho phép chuyển đổi hệ nguyên tố bằng cách khảm đè lông khác hệ, và lưu trữ bền vững trạng thái thuộc tính nguyên tố cũng như cấp cường hóa xuống cơ sở dữ liệu.

**Blocked by:** 
- 02: Động Cơ Hợp Thành Căn Bản Thuần Túy & Logic Fallback Cấp 1
- 03: Tích Hợp Giao Thức Mạng Hợp Thành Căn Bản & Giao Dịch Cơ Sở Dữ Liệu

**Status:** completed

- [x] Xây dựng mô-đun nghiệp vụ tính toán cường hóa khảm lông: xác thực loại trang bị có thể khảm và mã định danh của Lông vũ hợp lệ.
- [x] Xử lý khảm lần đầu cho trang bị chưa có hệ: kích hoạt hệ nguyên tố của lông và đặt cấp cường hóa ban đầu là +1 với tỷ lệ thành công tuyệt đối 100%.
- [x] Xử lý nâng cấp cùng hệ nguyên tố: yêu cầu cấp độ của Lông vũ tương ứng với cấp thăng tiến tiếp theo và quay số ngẫu nhiên theo bảng tỷ lệ thành công chuẩn theo từng mốc cấp độ.
- [x] Xử lý kết quả khi khảm thất bại: giữ nguyên cấp cường hóa đối với các mốc cấp thấp (dưới hoặc bằng +3) và giảm 1 cấp đối với các mốc cấp cao (+4 trở lên).
- [x] Xử lý chuyển đổi hệ nguyên tố: cho phép dùng Lông khác hệ khảm đè lên trang bị đã có hệ cũ, chuyển đổi hệ trang bị sang hệ mới và đặt lại cấp cường hóa về +1.
- [x] Thực thi giao dịch cơ sở dữ liệu nguyên tử: tiêu hao 1 đơn vị Lông vũ, cập nhật thuộc tính hệ nguyên tố và giá trị cấp cường hóa vào đúng bản ghi trang bị trong bảng dữ liệu túi đồ.
- [x] Phát gói tin cập nhật thuộc tính trang bị, gói tin trừ nguyên liệu, gói âm thanh và thông báo kết quả cường hóa cho ứng dụng người chơi.
- [x] Có bộ kiểm thử toàn diện tại thư mục kiểm thử tập trung kiểm tra cả logic tính toán tỷ lệ thuần túy lẫn tính toàn vẹn của dữ liệu sau khi khảm.
