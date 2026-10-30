# 02: Động Cơ Hợp Thành Căn Bản Thuần Túy & Logic Fallback Cấp 1

**What to build:** 
Động cơ nghiệp vụ hợp thành căn bản thuần túy cho phép tính toán kết quả khi người chơi kết hợp từ 2 đến 3 nguyên liệu (khoáng thạch, dược phẩm, trang bị phôi) với nhau theo chuẩn nguyên bản của trò chơi: tính toán phẩm chất trung bình theo phương pháp làm tròn chẵn Banker's Rounding, quay số thăng cấp phẩm chất mục tiêu, tra cứu bảng công thức bằng khóa số nguyên, tự động kích hoạt cơ chế an toàn hạ về trang bị rác cấp 1 khi trượt công thức để người chơi không bao giờ bị mất trắng đồ, và bảo đảm số lượng thành phẩm tạo ra tuân thủ giới hạn loại trang bị.

**Blocked by:** 01: Nạp & Chỉ Mục CSDL Compound và Sửa Lỗi Trường Danh Mục Vật Phẩm

**Status:** completed

- [x] Thẩm định các điều kiện hợp lệ đầu vào: ít nhất 2 vật phẩm, dải mã định danh hợp lệ, số lượng một ô không vượt quá 50, và từ chối các loại vật phẩm thuộc nhóm cấm.
- [x] Tính bậc phẩm chất trung bình bằng phương pháp Banker's Rounding (IEEE-754 Round to Even) và quay số ngẫu nhiên cấp phẩm chất mục tiêu trong khoảng từ 1 đến cấp trung bình cộng 1 (kẹp trần tối đa là 10).
- [x] Ghép các mã chất liệu đã sắp xếp tăng dần thành khóa số nguyên và tìm kiếm công thức khớp trong danh mục công thức đã lập chỉ mục.
- [x] Kích hoạt cơ chế Fallback nguyên bản khi không có công thức khớp: chọn ngẫu nhiên loại của một trong các nguyên liệu đầu vào và ép bậc phẩm chất mục tiêu về 1 (không báo lỗi thất bại hay làm mất trắng đồ).
- [x] Áp dụng quy tắc số lượng: ép số lượng thành phẩm về 1 nếu là trang bị vũ khí/giáp; giữ nguyên số lượng theo giá trị nhỏ nhất của nguyên liệu đầu vào nếu là vật phẩm tiêu hao/khoáng thạch.
- [x] Cho phép truyền seed số ngẫu nhiên cố định để phục vụ cho các bài kiểm thử tự động deterministic.
- [x] Có bộ kiểm thử độc lập tại thư mục kiểm thử tập trung xác minh tính đúng đắn của giải thuật với các công thức mẫu từ cẩm nang và trường hợp trượt công thức.
