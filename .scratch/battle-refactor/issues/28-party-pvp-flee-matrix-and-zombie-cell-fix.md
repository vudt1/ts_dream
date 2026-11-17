# 28: Chạy trốn Tổ đội / PvP (4 vai trò), Sửa Zombie Cell & Đồng bộ người tham gia

**What to build:** Hoàn thiện xử lý phạm vi Chạy trốn cho đủ 4 vai trò tổ đội trên cả PvE và PvP hai phe đối xứng (Đội trưởng / *Sủng vật* của Đội trưởng ở bất kỳ phe nào trốn thành công thì toàn trận kết thúc sạch; Thành viên / *Sủng vật* của Thành viên trốn thành công thì chỉ chủ sở hữu đó cùng các *Sủng vật* của họ thoát trận); sửa triệt để bug Zombie Cell ghi đè lại *Ô chiến trận* đã xóa, bỏ `npc_respawn` và khung xóa ô trùng lặp ở nhánh Thành viên, đồng thời gỡ người vừa trốn khỏi danh sách người tham gia cuối trận.

**Blocked by:** 27: Hoàn thiện PvP Party hai phe đối xứng (`0x0B 02 02`), Guard PK & Chuẩn hóa roll Chạy trốn (`14002` +20%, fail `18002`)

**Spec:** [spec-pk-and-flee.md](../spec-pk-and-flee.md)

**Status:** completed

## Acceptance Criteria

- [x] Khi Đội trưởng hoặc *Sủng vật* của Đội trưởng (thuộc Phe người chơi PvE, hoặc thuộc một trong hai phe trong trận PvP Party) chạy trốn thành công, trận đấu trả `Outcome::PlayerFled` và toàn bộ các bên tham chiến thoát sạch về bản đồ (không thưởng EXP/Drop).
- [x] Khi Thành viên tổ đội hoặc *Sủng vật* của Thành viên (ở bất kỳ phe nào) chạy trốn thành công, toàn bộ *Ô chiến trận* thuộc chủ sở hữu đó (gồm cả Nhân vật và các *Sủng vật* của họ) được xóa khỏi lưới, phát `acting`, `reset_cell` (chỉ 1 lần mỗi ô), `0B 00`, `05 04`, `14 08` đúng đối tượng và giữ nguyên trận cho những người còn lại.
- [x] Xác minh qua nhiều hiệp liên tiếp sau khi Thành viên trốn thành công: vòng thực thi không ghi đè lại bản sao *Ô chiến trận* cũ lên lưới (không tạo Zombie Cell), không phát `34 01` cho người đã trốn, và nếu người sống sót cuối cùng trốn trong khi Đội trưởng đã gục thì trận kết thúc đúng với `Outcome::PlayerLose`.
- [x] Nhánh Thành viên chạy trốn không gọi `npc_respawn` giữa trận; sự kiện `Out::Fled { battle_id, player }` gỡ người trốn khỏi cả danh sách `members` lẫn `battle_participants` để `battle_ended` sau đó không ghi đè trạng thái hoặc kích hoạt lại Eve cho người đã thoát.
- [x] Đủ integration tests cho cả 4 vai trò tổ đội (PvE + PvP) và hồi quy Zombie Cell trong `battle_test`, và toàn bộ `cargo test --all-targets --no-fail-fast` vượt qua 100%.
