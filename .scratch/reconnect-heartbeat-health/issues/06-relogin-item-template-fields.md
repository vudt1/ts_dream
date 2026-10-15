# 06: Sau đăng nhập lại, vật phẩm có đủ thuộc tính từ mẫu gốc

**What to build:** Vật phẩm trong túi và trang bị được khôi phục đầy đủ thuộc tính gốc (cấp, loại, thuộc tính, các chỉ số phụ) sau khi đăng nhập lại, không chỉ số lượng và độ bền. Điều kiện trang bị (equip gate) hoạt động đúng sau relog.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Vật phẩm sau load có cấp, loại, thuộc tính và giá trị thuộc tính khớp với mẫu gốc
- [x] Các chỉ số phụ tương ứng được điền đúng
- [x] Điều kiện trang bị dùng đúng dữ liệu sau relog
- [x] Dữ liệu per-instance (`khang`, `texp` của vật phẩm, `long_val`) không bị ảnh hưởng và vẫn đọc/ghi như trước
- [x] Test integration dùng DB test
