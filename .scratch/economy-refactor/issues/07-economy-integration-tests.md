# 07: Economy Integration Tests & Wire Diffing Harness

**What to build:** Bộ kiểm thử tích hợp toàn diện (Integration Test Suite) cho toàn bộ hệ thống Kinh tế Game (Shop NPC, Tiền Tệ, Tiền Trang, Kho Bãi, Shop Người Chơi, và Giao Dịch 1-1) đặt tập trung tại `tests/economy_wire_test.rs`. Tuyệt đối không dùng `#[cfg(test)]` inline trong `src/`. Sử dụng cơ sở dữ liệu in-memory SQLite (`sqlite::memory:?cache=shared`) độc lập, cách ly hoàn toàn với production database `DB/ts_dream.db`. Kiểm tra tính khớp nối byte-exact của các gói tin nhị phân wire protocol đối chiếu với client `aLogin.exe`.

**Blocked by:** 01, 02, 03, 04, 05, 06.

**Status:** completed

- [x] Tạo file test tập trung `tests/economy_wire_test.rs` tuân thủ nghiêm ngặt quy tắc kiểm thử của dự án (cấm `#[cfg(test)]` trong `src/`).
- [x] Test NPC Shop Mua/Bán: Mô phỏng gửi gói `1B 01` (Mua) và `1B 02` (Bán); kiểm tra server trả về đúng chuỗi wire frame `1A 01/02`, `1A 04`, `17 06/09`, và mã kết quả `1B 01/02 xx`.
- [x] Test Mô Hình Tiền Tệ: Kiểm tra gói `spawn_player_self` phát đúng `1D 04` (Bank gold) và `1A 04` (bảo toàn `shop_point` khác 0); kiểm tra trần clamp `9,999,999`.
- [x] Test Tiền Trang: Kiểm tra nộp tiền (`1D 02`) và rút tiền (`1D 01`), xác minh cập nhật số dư nguyên tử trong `character_money` và các gói phản hồi delta tương ứng.
- [x] Test Chuyển Kho: Kiểm tra cất/rút đồ giữa hành trang và Tiền Trang/Lưu Lãng, xác minh gói `1E 05` và `17 09` mang số lượng thực tế của item (không bị hardcode `"32"`).
- [x] Test Quầy Hàng (Stall): Kiểm tra gói danh mục S→C `17 21` đúng cấu trúc 17B header + 16B records; kiểm tra người mua trả vàng và nhận item, người bán nhận vàng và trừ đồ trên quầy.
- [x] Chạy kiểm thử thành công qua lệnh `cargo test --test economy_wire_test --no-fail-fast` với 0 lỗi biên dịch và 0 cảnh báo hồi quy.
