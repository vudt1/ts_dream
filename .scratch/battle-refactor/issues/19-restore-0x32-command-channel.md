# 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction

**What to build:** Phục hồi kênh lệnh chiến đấu `0x32` C→S chuẩn wire parity từ client `aLogin.exe` (sửa sai lầm từ Ticket 01 khi xoá `0x32` và re-parent nhầm sang `0x0B Sub 3/4`). Khôi phục `OP_BATTLE_COMMAND (0x32)` trong `src/server/dispatcher.rs` để tiếp nhận frame 10 bytes:
$$\text{[32] [sub:1B] [srcGrp:1B] [srcSlot:1B] [dstGrp:1B] [dstSlot:1B] [skillOrItemId:2B LE] [rand1:1B] [rand2:1B]}$$
Đồng thời, thay thế struct phẳng cũ bằng Domain Enum `BattleAction` (Quyết định Q5: B) bao gồm `NormalAttack`, `CastSkill { skill_id, skill_lv }`, `UseItem { item_id }`, `Defend`, `Flee`, `Capture`. Trả lại `0x0B Sub 03/04` làm out-of-battle action holders (8 byte), và cập nhật toàn bộ test suite trong `tests/` sang gửi frame `0x32`.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Đăng ký lại `OP_BATTLE_COMMAND (0x32)` trong `src/server/dispatcher.rs` định tuyến tới `handlers/battle::handle_battle_command`.
- [x] Parser giải mã chuẩn xác gói tin 10 bytes: phân tách `sub 1` (kỹ năng/tấn công/thủ/chạy/bắt) và `sub 2` (vật phẩm chiến đấu), đọc tọa độ nguồn `(srcGrp, srcSlot)`, đích `(dstGrp, dstSlot)` và mã định danh 2 byte Little-Endian.
- [x] Định nghĩa Domain Enum `BattleAction` và struct `BattleCommand` mới trong `src/battle/runner.rs`, tự động tra cứu `skill_lv` từ Session nhân vật hoặc sủng vật tương ứng.
- [x] `0x0B Sub 03` và `Sub 04` không còn xử lý lệnh chiến đấu; khôi phục ngữ nghĩa lệnh thao tác bản đồ (8 bytes).
- [x] Toàn bộ các integration test trong `tests/` đang giả lập gửi lệnh qua `0x0B` được chuyển sang gửi frame `0x32` chuẩn wire.
- [x] Lệnh kiểm thử `cargo test --all-targets --no-fail-fast` vượt qua 100%.
