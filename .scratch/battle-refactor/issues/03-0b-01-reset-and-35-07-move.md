# 03: Dọn ô 0B 01 (dọn pad thừa) + builder 35 07 di chuyển ô

> **Cập nhật 2026-10-16 (HANDOFF §3):** `0B 01` S→C — client `0064bb24` chỉ đọc `content[2]` (row ≤ 3) và `content[3]` (col ≤ 4); **byte pad `00` thừa vô hại**, độ dài tối thiểu là 4 byte. Vì vậy phần "bỏ pad" chỉ là chuẩn hoá hình thức, **không phải bug sửa layout**; phần còn lại của ticket (`35 07` + đổi tên hàm) giữ nguyên.

**What to build:** Hai việc về frame vị trí: (a) frame dọn ô `0B 01` hiện gửi kèm byte đệm `00` thừa (`F44405000B01 r c 00`) — chuẩn hoá về payload 4 byte `[0B][01][grp][slot]` cho khớp `clear_pet_cell`, và đổi tên các hàm cho đúng ngữ nghĩa (đây là dọn/reset ô lưới, không phải "di chuyển" hay "xoá pet"); (b) thêm builder `35 07` — frame DUY NHẤT cập nhật lại ô lưới sau khi trận mở (`[35][07][srcGrp][srcSlot][dstGrp][dstSlot]`), hiện chưa có nên ô phía client đóng băng. Engine chưa có call-site di chuyển unit nên ticket này ship builder + test; việc nối vào luồng di chuyển để ticket sau quyết định.

**Bằng chứng client (`0064bb24`, HANDOFF §3.2):**

```
content[0] = 0x0B, content[1] = 0x01
content[2] = row   (BoundErr nếu > 3)
content[3] = col   (BoundErr nếu > 4)
→ độ dài tối thiểu 4 byte; byte content[4..] không được đọc
```

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] `0B 01` = `F44404000B01 grp slot` (payload 4 byte, không còn `00`) — cả `reposition` lẫn `clear_pet_cell` dùng chung một builder
- [x] Comment builder ghi rõ: `row ≤ 3`, `col ≤ 4`, byte pad là tuỳ chọn (bằng chứng `0064bb24:50-62`)
- [x] Không còn hàm/builder mang tên gây hiểu nhầm "di chuyển"/"clear pet cell"
- [x] Builder `35 07` byte-exact theo `02_battle_positions.md §2.10`, kèm test độ dài 6 byte
- [x] `finish()` và các call-site cũ dùng builder mới; test hiện có vẫn xanh
- [x] Test mới trong `tests/` cho cả hai frame