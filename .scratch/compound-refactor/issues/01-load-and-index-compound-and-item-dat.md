# 01: Nạp & Chỉ Mục CSDL Compound và Sửa Lỗi Trường Danh Mục Vật Phẩm

**What to build:** 
Hệ thống nạp dữ liệu nhị phân của Server đọc và nhận diện chính xác mã chất liệu hợp thành (`compound_material`, giá trị 1..37) và cấp phẩm chất (`compound_rank`, giá trị 1..10+) cho toàn bộ vật phẩm trong game, đồng thời phân tích trọn vẹn 1.393 bản ghi công thức hợp thành (bỏ qua phần header metadata) và xây dựng chỉ mục tìm kiếm nhị phân theo khóa số nguyên để phục vụ cho việc tra cứu công thức tức thì.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Đọc và lưu trữ chính xác trường mã chất liệu hợp thành (`compound_material`) từ bản ghi nhị phân của từng vật phẩm.
- [x] Đọc và lưu trữ chính xác trường cấp phẩm chất hợp thành (`compound_rank`) từ bản ghi nhị phân của từng vật phẩm.
- [x] Khử xung đột tên gọi đối với trường chất liệu hiển thị phụ trợ cũ để tránh nhầm lẫn trong toàn bộ codebase.
- [x] Bỏ qua đúng 8 byte header metadata ở đầu tệp dữ liệu công thức hợp thành để nạp đủ và đúng 1.393 bản ghi công thức.
- [x] Chuẩn hóa trường loại trang bị kết quả và lập chỉ mục mảng công thức được sắp xếp theo khóa số nguyên 32-bit (ghép từ các mã chất liệu).
- [x] Có bài kiểm thử tự động tại thư mục kiểm thử tập trung xác minh tính toàn vẹn của danh mục vật phẩm và danh mục công thức sau khi nạp.
