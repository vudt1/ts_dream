# 08: Quy tắc team row < 2

**What to build:** Server tính team bằng `row == 0` (3 chỗ: thêm người chơi, nạp pet leader, nạp pet thành viên) — trong khi client dùng `grp < 2` ⇒ phe 2 và `grp ≥ 2` ⇒ phe 1. Sửa về một helper chung `team_for_row` để `add_player(row=1)` (hàng địch) tính đúng team, hết latent bug.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Ba call-site dùng chung một helper; biểu thức là `row < 2 → 2`, ngược lại `→ 1`
- [x] Test pure-logic cho row 0/1/2/3
- [x] Test battle: `add_player(row=1)` → team 2; `add_player(row=2|3)` → team 1
