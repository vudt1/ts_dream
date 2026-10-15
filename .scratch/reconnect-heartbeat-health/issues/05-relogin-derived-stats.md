# 05: Sau đăng nhập lại, chỉ số dẫn xuất từ trang bị được tính đúng

**What to build:** Nhân vật đăng nhập lại có đúng bonus chỉ số từ trang bị (sức mạnh, phòng thủ, v.v.) ngay khi vào game. Gói thông tin chỉ số gửi cho client phản ánh đúng trang bị đang mặc.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Sau khi tải dữ liệu từ DB, chỉ số dẫn xuất được tính lại trước khi gửi gói đăng nhập thành công
- [x] Bonus từ trang bị khác 0 khi nhân vật có trang bị
- [x] Gói cập nhật chỉ số gửi cho client khớp với trạng thái trang bị
- [x] Test integration dùng DB test, không ghi lên DB production
