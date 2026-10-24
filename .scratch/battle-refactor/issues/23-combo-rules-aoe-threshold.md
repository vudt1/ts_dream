# 23: Chuẩn hóa quy tắc hợp kích Combo theo giới hạn mục tiêu AoE

**What to build:** Khắc phục lỗ hổng trong giải thuật phát hiện và kích hoạt liên kích (Combo) tại `src/battle/runner.rs::detect_combo`. Theo đặc tả client mục 4.3.2 ([`00647164`](../../client_pseudo_c/00647164_FUN_00647164.c) và [`005ab678`](../../client_pseudo_c/005ab678_FUN_005ab678.c)):
1. Chỉ đòn đánh thường (`10000`) và các kỹ năng có phạm vi tác dụng từ 3 mục tiêu trở xuống (`nbTarget <= 3` hoặc `fight_area <= 3`) mới được phép tham gia chuỗi Combo.
2. Các kỹ năng AoE diện rộng từ 4 mục tiêu trở lên (`nbTarget >= 4`, ví dụ Loạn kích `13013`, Phi sa tẩu thạch `10008`, Hồng thủy `11008`, Thiên băng địa liệt `10012`, Phong hỏa liêu nguyên `12014`) **bắt buộc bị vô hiệu hóa Combo hoàn toàn**.

**Blocked by:** 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction, 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01

**Status:** ready-for-agent

- [ ] Cập nhật `detect_combo` trong `runner.rs` để kiểm tra thuộc tính `nbTarget <= 3` của cả đấu sĩ ra đòn trước và đấu sĩ liên kích tiếp theo.
- [ ] Chặn tuyệt đối không tạo combo nếu kỹ năng của một trong hai bên có `nbTarget >= 4`.
- [ ] Viết integration test trong `tests/battle_combo_rules_test.rs` mô phỏng các cặp đấu sĩ cùng Agi:
  - Cặp dùng Đánh thường + Ném đá (AoE 3): kích hoạt Combo thành công.
  - Cặp dùng Đánh thường + Loạn kích (AoE 4): bị từ chối Combo, đánh độc lập.
- [ ] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
