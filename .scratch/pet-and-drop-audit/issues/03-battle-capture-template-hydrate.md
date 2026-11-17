# 03: Khởi tạo đầy đủ chỉ số gốc từ Npc.dat khi bắt quái trong trận (Battle Capture Template Hydrate)

**What to build:**  
Khi người chơi sử dụng lệnh kỹ năng Bắt (`15001`) thu phục thành công một NPC trong trận chiến (Battle Capture), sủng vật mới gia nhập danh sách pet với cấp độ 1 cùng toàn bộ bộ chỉ số gốc được nạp từ bản mẫu NPC (`Data/Npc.dat` / `data.npcs`): Thuộc tính Hệ (Element), MaxHP, MaxSP, INT, ATK, DEF, AGI, HPX, SPX, độ trung thành mặc định 60 và 3-4 kỹ năng mặc định của NPC (cấp 1). Khắc phục triệt để tình trạng sủng vật mới bắt bị rỗng toàn bộ chỉ số (bằng 0).

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Cập nhật hàm `add_caught` (hoặc luồng `BattleService::apply_catch`) để tiếp nhận tham chiếu bản mẫu `Npc` thay vì chỉ nhận `npc_id` và `hp_max`.
- [ ] Gán đầy đủ thông số cho `PetState`:
  - `level = 1` (theo quyết định thiết kế đã chốt để người chơi tự luyện từ cấp 1).
  - `thuoctinh = npc.element as u8`.
  - `hp = npc.hp_max as u16`, `hp_max = npc.hp_max as u16`.
  - `sp = npc.sp_max as u16`, `sp_max = npc.sp_max as u16`.
  - `int1 = npc.int_val as u16`, `atk = npc.atk as u16`, `def = npc.def as u16`, `agi = npc.agi as u16`.
  - `hpx = npc.hpx as u16`, `spx = npc.spx as u16`.
  - `fai = 60` (độ trung thành khởi điểm mặc định).
  - Nạp 4 kỹ năng: `skill1_id..skill4_id` từ `npc.skill1..skill4`, gán `skill_level = 1` nếu có ID kỹ năng hợp lệ (> 0).
- [ ] Đảm bảo dữ liệu sủng vật đầy đủ này được write-through xuống bảng `character_pets` SQLite khi kết thúc trận đấu.
- [ ] Có unit test kiểm thử: Bắt một NPC mẫu (ví dụ Trương Giác #10001) $\rightarrow$ kiểm tra sủng vật mới tạo có level 1, mang hệ Địa (1), INT/ATK/DEF/AGI và các kỹ năng trùng khớp với dữ liệu trong `Npc.dat`.
