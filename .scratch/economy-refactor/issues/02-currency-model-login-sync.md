# 02: Currency Model & Bank Gold Sync at Login

**What to build:** Đồng bộ hóa chính xác mô hình 3 loại tiền tệ trong game giữa client `aLogin.exe` và server Rust: Vàng trên người (`+0x12F8`, trần `9,999,999`), Vàng trong Tiền Trang / Bank (`+0x12FC`, trần `9,999,999`), và Điểm Mềm / Shop Points (`+0x1300`, trần `2,000,000,000`). Khi nhân vật đăng nhập, server gửi gói `1D 04 <bank_gold: u32 LE>` để client hiển thị đúng tiền bank ngay lập tức, đồng thời frame `1A 04` truyền đúng giá trị `shop_point` từ bảng `character_money.shoppoint` thay vì ghi đè bằng 0.

**Blocked by:** None (can start immediately).

**Status:** completed

- [x] Trong `src/server/spawn.rs`, bổ sung gói `1D 04` đồng bộ số dư Bank gold (`F44406001D04<bank_gold: u32 LE>`) khi người chơi đăng nhập hoàn tất spawn.
- [x] Trong `src/server/spawn.rs` Step 12 (frame tiền tệ `1A 04`), gửi `s.shop_point` ở DWORD thứ 2 thay vì chuỗi cố định `00000000`.
- [x] Cập nhật hàm `gold_frame(gold: u32, shop_point: u32)` trong `src/server/handlers/shops.rs` và điều chỉnh tất cả các vị trí gọi (trong `shops.rs`, `talk.rs`, v.v.) để bảo toàn điểm mềm.
- [x] Cập nhật hàm `gold_status_frame` trong `src/server/handlers/trade_storage.rs` truyền đủ cả `gold` và `shop_point`.
- [x] Đảm bảo khi cộng tiền vàng (`1A 01` hoặc cộng nội bộ), giá trị luôn được clamp trần `9,999,999` để tránh lỗi kẹt client (client từ chối frame delta vượt quá 9,999,999).
- [x] Kiểm tra tính toàn vẹn dữ liệu khi đọc/ghi `character_money`: `gold`, `bankgold`, và `shoppoint` duy trì trạng thái nhất quán giữa RAM `Session` và DB SQLite.
