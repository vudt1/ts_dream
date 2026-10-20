# 05: Frame 1A 01 — cộng vàng kèm banner sau trận

**What to build:** `apply_gold` hiện chỉ cộng số dư trong session, client không hề biết → phần thưởng vàng của trận vô hình. Phát `F44406001A01{le32(delta)}` cho người nhận khi cộng vàng (client cộng số dư + hiện banner "Nhận được X Tiền", trần 9.999.999), theo `04_battle_end_rewards.md §2.7`; dùng cùng khuôn mẫu đã có ở đường shop.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] `apply_gold` phát `1A 01` byte-exact (`F44406001A01` + le32) cho chủ khi amount > 0
- [x] Số dư session vẫn được cộng; giá trị gửi khớp delta thực nhận và tôn trọng trần client 9.999.999
- [x] Test trong `tests/` mô phỏng đường cộng vàng sau trận → frames chứa `1A01` đúng số
