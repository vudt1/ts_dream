# 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01

**What to build:** Khắc phục lỗi đảo lộn thời điểm phát `34 01` (lỗi timing inversion khiến client bị đơ menu chọn lệnh ở Hiệp 1). Tái cấu trúc luồng vòng lặp hiệp đấu trong `src/battle/manager.rs`:
1. **Turn 1 Handshake Barrier**: Thiết lập rào chắn chờ gói xác nhận `0B FA` (2 bytes) từ client chủ phòng sau khi nạp xong scene (áp dụng fallback timeout an toàn 1.5s).
2. **Turn Prompt**: Phát gói tin `34 01` (`packets::your_turn()`) ở **ĐẦU MỖI HIỆP ĐẤU** tới các đấu sĩ còn sống phe ta trước khi bắt đầu lắng nghe lệnh, kích hoạt client reset ô cờ, mở menu điều khiển và chạy đồng hồ đếm lùi 20s.
3. **Timeout Fallback**: Khi hết 20 giây mà người chơi chưa ra lệnh, tự động gán hành động Tấn công thường `10000` nhắm vào mục tiêu quái địch gần nhất (Quyết định Q6: A).

**Blocked by:** 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction

**Status:** completed

- [x] Task trận đấu trong `manager.rs` chờ gói ack `0B FA` ở đầu trận (với timeout fallback 1.5s) trước khi phát `34 01` hiệp 1.
- [x] Gói tin `34 01` (`F44402003401`) được gửi tới mọi người chơi phe ta ở bước đầu tiên của mỗi hiệp, không còn bị sinh muộn bên trong `run_turn`.
- [x] Hàm `collect_commands` gom lệnh qua kênh `0x32` trong khoảng thời gian đếm lùi; khi timeout tự động gán `BattleAction::NormalAttack` (Skill 10000).
- [x] Bộ kiểm thử trong `tests/` mô phỏng trật tự vòng đời hiệp đấu xác nhận `34 01` luôn đi trước việc thu thập lệnh và thực thi đòn đánh.
- [x] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
