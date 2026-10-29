# 22: Chuẩn hóa hình học Isometric ô lưới cho kỹ năng AoE (targeting.rs)

**What to build:** Chuẩn hóa giải thuật tính toán phạm vi mục tiêu diện rộng (AoE) trong `src/battle/targeting.rs::expand_sl_danh` theo đúng trật tự hình học Isometric của client Delphi (`00652454`). Trên bàn cờ 4×5, thứ tự cột dọc theo đường chéo Isometric từ trên-phải xuống dưới-trái là `0 -> 1 -> 2 -> 4 -> 3` (trong đó `slot 2` là cột trung tâm). Kỹ năng lan 3 mục tiêu (`sl_danh = 3`, ví dụ Ném đá `10005`, Hỏa tiễn `12003`, Nham quái `10016`) khi nhắm ô tâm `slot 2` phải mở rộng ra 2 ô lân cận theo hình học là `{1, 2, 4}` (thay vì `{1, 2, 3}` tuyến tính hiện tại).

**Blocked by:** 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction, 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01

**Status:** completed

- [x] Cập nhật nhánh `3 =>` trong `expand_sl_danh` tuân thủ đúng thứ tự kề cận Isometric: tâm `2` lấy `{1, 2, 4}`, tâm `1` lấy `{0, 1, 2}`, tâm `4` lấy `{2, 3, 4}`, tâm `0` lấy `{0, 1}`, tâm `3` lấy `{3, 4}`.
- [x] Rà soát và đồng bộ các nhánh mở rộng khác (`sl_danh = 2, 5, 6`) bảo đảm tính đối xứng qua trục Isometric.
- [x] Bộ kiểm thử trong `tests/` (ví dụ `tests/battle_isometric_targeting_test.rs`) xác nhận danh sách ô mục tiêu trúng đòn khớp 100% với vùng highlight của client.
- [x] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
