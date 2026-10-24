# 24: Hoàn thiện 8 bước kiểm định thu phục võ tướng (apply_catch)

**What to build:** Bổ sung đầy đủ 8 rào chắn kiểm tra điều kiện thu phục / bắt NPC (`Skill 15001`) trong `src/battle/runner.rs::apply_catch` theo đúng logic của client tại `0063cf58_FUN_0063cf58.c` (dòng 362–486 và 631–638):
1. **Chủ tướng hành động**: Chỉ Nhân vật chính (`caster.typ == 2`) mới được phép bắt. Sủng vật không thể bắt NPC.
2. **Quy tắc chênh lệch cấp độ (Lệch LV $\le 5$)**: Độ chênh lệch cấp độ giữa nhân vật và võ tướng/NPC không được vượt quá 5 cấp (`target.lv - caster.lv <= 5`). Nếu cấp độ của NPC cao hơn người chơi quá 5 cấp thì từ chối bắt (ở phía client UI cũng có thêm rào cản chặn cứng nếu lệch $> 40$ cấp).
3. **Loại thực thể hợp lệ**: Không được bắt người chơi khác, sủng vật đã có chủ (`typ == 4`), hoặc khí cụ công thành (`typ == 9`).
4. **Cờ cấm bắt trong Npc.dat**: Thuộc tính `bat == 0` (được phép bắt). Nếu `bat == 1` thì từ chối.
5. **Chống trùng lặp**: Người chơi chưa sở hữu sủng vật có cùng `template_id` trong 4 ô sủng vật mang theo.
6. **Sức chứa túi sủng vật**: Túi sủng vật phải còn ít nhất 1 ô trống (< 4 con).
7. **Kỹ năng Triệu gọi (Skill 14026)**: Nếu NPC có thuộc tính đặc biệt `FUN_00625128 == 0x16`, người chơi bắt buộc phải đã học kỹ năng Triệu gọi `14026`.
8. **Ngưỡng máu sinh lực**: HP hiện tại của mục tiêu phải dưới 50% MaxHP.

**Blocked by:** 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction, 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01

**Status:** ready-for-agent

- [ ] Cập nhật `apply_catch` trong `runner.rs` tích hợp đầy đủ 8 điều kiện kiểm định trên trước khi tính toán tỷ lệ xác suất thành công.
- [ ] Xử lý phản hồi gói tin thất bại hoặc thành công chuẩn hóa, không gây desync trạng thái giữa server và client.
- [ ] Viết bộ test trong `tests/battle_catch_verification_test.rs` mô phỏng đầy đủ 8 ca kiểm thử biên (lệch level > 5, túi đầy, trùng tướng, thiếu chiêu Triệu gọi...).
- [ ] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
