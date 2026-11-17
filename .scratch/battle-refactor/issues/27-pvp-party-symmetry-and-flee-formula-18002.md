# 27: Hoàn thiện PvP Party hai phe đối xứng (`0x0B 02 02`), Guard PK & Chuẩn hóa roll Chạy trốn (`14002` +20%, fail `18002`)

**What to build:** Hoàn thiện toàn bộ luồng PvP (`0x0B 02 02`) thành `PKBattle` hỗ trợ đầy đủ Đội trưởng, Thành viên tổ đội cùng bản đồ và *Sủng vật* cho cả hai phe đối xứng trên *Lưới Chiến Trận Isometric 4×5*, cấp đủ `battle_id`, kỹ năng, khung `0B 05 05`, bước dọn dẹp kết thúc trận cho cả hai phe, cùng các chốt chặn thách đấu PK trên người chơi; đồng thời chuẩn hóa công thức roll Chạy trốn theo LV (`18001` vs `14002` cộng `+20%`, clamp `[0..=100]`, bỏ bypass 100%), chặn NPC dùng nhánh Flee và sửa gói Flee thất bại luôn phát `18002`.

**Blocked by:** 26: Phân loại `BattleKind` (`Normal` vs `PKBattle`), Sửa `0x0B 02 03` PK Võ tướng & Chuẩn hóa `0B FA` (`0x03`/`0x05`/`0x04`)

**Spec:** [spec-pk-and-flee.md](../spec-pk-and-flee.md)

**Status:** ready-for-agent

## Acceptance Criteria

### 1. Hoàn thiện PvP Party hai phe đối xứng (`0x0B 02 02`) & Guard PK
- [ ] Khi thách đấu `0x0B 02 02`, server chặn các trường hợp không hợp lệ trên người chơi: tự PK chính mình, khác bản đồ, cùng tổ đội (`id_leader`), mục tiêu thuộc nhóm GM/Thiên Thần, đang là *ListQS*, hoặc đang mở *Sạp hàng người chơi*; giữ kiểm tra công tắc Option PK của cả hai bên (phát `21 01 01` nếu đối thủ chưa bật PK). Không kiểm tra danh sách bản đồ Safe Zone trên server trong ticket này.
- [ ] Trận PvP khởi tạo với `BattleKind::Pk`, nạp đầy đủ Đội trưởng, Thành viên tổ đội cùng bản đồ và *Sủng vật* của cả hai phe: Phe 1 tại hàng `3` (Player/Member) & hàng `2` (*Sủng vật*), Phe 2 đối xứng tại hàng `0` (Player/Member) & hàng `1` (*Sủng vật*) theo thứ tự cột `[2, 1, 3, 0, 4]`.
- [ ] Mọi người chơi thuộc cả Phe 1 và Phe 2 đều được cập nhật `battle_id` trong phiên trực tuyến và danh mục kỹ năng chiến đấu, nhận gói mở trận chuẩn với khung thực thể người chơi (`0B 05 05`) thay vì khung NPC (`0B 05 03`), và có thể gửi lệnh `0x32` bình thường.
- [ ] Khi trận PvP kết thúc, bước dọn dẹp duyệt qua toàn bộ *Ô chiến trận* người chơi ở **cả Phe 1 lẫn Phe 2**: hồi `Hp = 1` nếu tử trận, phát chuỗi `0B 00`, `0B 01`, `05 04`, `14 08` và không cấp thưởng/không trừ *Phúc Thần*.

### 2. Công thức Chạy trốn LV +20% (`14002`), Guard NPC & Gói thất bại `18002`
- [ ] Hàm tính tỷ lệ Flee dùng chung công thức `clamp(60 + (lv1 - lv2) + (lvtb1 - lvtb2) + spell_bonus, 0, 100)`, với `spell_bonus = +20` cho `14002` (*Thuật đào tẩu*) và `0` cho `18001` (*Chạy trốn*).
- [ ] Bỏ hoàn toàn điều kiện `|| skill == 14002` (không còn tự động thắng 100% khi dùng `14002` ở chênh lệch cấp độ bất lợi).
- [ ] Chỉ thực thể Nhân vật hoặc *Sủng vật* của người chơi (`typ ∈ {2, 4}`) mới được xử lý qua logic Flee; thực thể NPC không bao giờ kích hoạt thoát trận của phe người chơi.
- [ ] Khi Flee thất bại với cả `18001` lẫn `14002`, frame `32 01` luôn phát `skill_id = 18002` (`skill_type = 12`, `miss = 0`, `adl = 2`), tuyệt đối không phát `14003`.

### 3. Kiểm thử (`battle_test` & `eve_quest_test`)
- [ ] Bổ sung integration test PvP Party End-to-End và test các guard thách đấu PK.
- [ ] Cập nhật test `lifecycle` / `random_encounter` (gỡ mock giả `14003` và kỳ vọng `14002` auto thắng), đồng thời bổ sung test chứng minh cộng `+20%` cho `14002`, thất bại luôn phát `18002`, và chặn NPC dùng nhánh Flee.
- [ ] `cargo test --test battle_test` và `cargo test --test eve_quest_test` vượt qua 100%.
