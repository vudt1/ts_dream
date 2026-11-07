# ADR 0008 — Chuẩn hóa Giao thức Kỹ năng, Khung Định danh Nhân vật & Hệ thống Nghề Chuyển sinh (Skill Protocol Parity, Stats Packet & Job System)

- **Ngày**: 2026-10-31
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (qua phỏng vấn grilling /grill-with-docs)

---

## 1. Bối cảnh (Context)

1. Quá trình dịch ngược mã máy thân Client TS Online (`aLogin.exe` Borland Delphi Win32 x86) tại `.scratch/understand-skills/CLIENT_SKILLS_SPEC.md` và kiểm tra đối chiếu dữ liệu `Data/Skill.Dat`, `Data/skills.json`, `Data/eve.emg`, `Data/Item.Dat` đã làm sáng tỏ kiến trúc toàn diện của module Kỹ Năng:
   - **Không gian Wire ID chuẩn**: 100% sử dụng không gian ID từ `Data/skills.json` (`10000..23023`, 376 bản ghi). Client không có ID ngoài không gian này.
   - **Hai cấu trúc cây kỹ năng trong RAM**:
     - Bảng động `0x0094B440` (765 byte): 5 tab (Địa, Thủy, Hỏa, Phong, Tâm) × 51 slot × 3 byte (`{skillId, level}`), nạp từ đuôi gói `S→C 05 03` và cập nhật từng skill qua `S→C 08 01 6E`.
     - Cây tĩnh `TSe_SkillTreeForm`: 10 tab cố định (4 tab Ngũ hành, Tab 5 Tâm 37 kỹ năng sống, Tab 6..9 tương ứng 4 nghề Bá/Hiệp/Hiền/Tiên).
     - Bảng bóng Tái Sinh 2 `DAT_0094B3A4`: 4 Tâm pháp gốc (`idx 1..4`), Gốc nghề `14038` Đại chu thiên (`idx 5`), và 8 slot bóng Tái Sinh 2 (`idx 6..13` = `skill_rb2[0..7]`).
   - **Khung nhị phân chỉ số nhân vật `S→C 05 03`**: Header cố định 113 byte (`0x71`), trong đó `Payload[0x60]` là cấp Reborn (`player + 0x4B0`), `Payload[0x61]` là Job nghề (`player + 0x1271`), `Payload[0x05..0x06]` là Current SP, `Payload[0x18..0x19]` là Skill Points.

2. Đối chiếu mã nguồn server Rust `ts_dream` phát hiện các sai lệch và thiếu sót kiến trúc nghiêm trọng:
   - **Lỗi nạp `data.skills`**: `src/data/loader.rs` tìm file `Data/Skills.txt` không tồn tại khiến `data.skills` rỗng trong runtime, làm tê liệt `handle_player_skill_learn`.
   - **Gói `05 03` xóa trắng Reborn và Job**: `src/server/spawn.rs` điền `&"00".repeat(43)` lấp đầy offset `0x46..0x70`, gửi `reborn = 0` và `job = 0` về client, khiến client khóa cứng các tab nghề trên `TSe_SkillTreeForm`.
   - **Kịch bản Eve bỏ qua Trao kỹ năng**: `src/server/handlers/talk.rs` không xử lý `EveResultClass::Skill = 4`, khiến nhiệm vụ học kỹ năng sống hệ Tâm không thể trao kỹ năng cho người chơi.
   - **Thiếu nhánh Opcode `0x1C`**: Bỏ rơi Sub 4 (`0x1C 04`), Sub 5 (`0x1C 05`) bỏ qua danh sách Bi rồng và hoàn toàn không parse đoạn L3 (8 bóng `skill_rb2`). Thiếu gói đồng bộ `17 4D` và `17 4E`.
   - **Kỹ năng nghề trong Battle**: Chưa nạp Passive nghề vào `WarInfo`, kỹ năng nghề chủ động Type 19 rơi vào fallback rỗng không có hiệu ứng.
   - **Opcode `0x2A` (Respec)**: Bị đặt trong danh sách `unimplemented`.

---

## 2. Quyết định Thiết kế (Decisions)

### 2.1. Nguồn Dữ Liệu Kỹ Năng Server (Single Source of Truth)
- `src/data/loader.rs` chuyển sang deserialize trực tiếp từ `Data/skills.json` (376 bản ghi) vào `data.skills` (`HashMap<i64, Skill>`).
- Giữ nguyên `data.binary_skill_defs` từ `Data/Skill.Dat` để phục vụ xác thực thuộc tính nhị phân gốc khi cần.

### 2.2. Khung Dựng Chỉ Số Chuẩn Hóa `StatsPacketBuilder` (`05 03`)
- Xây dựng struct `StatsPacketBuilder` kiểm soát byte-exact toàn bộ 113 byte header của gói `05 03` gửi cho client:
  - `0x05..0x06` (`u16 LE`): `Current SP` (`player + 0x3EC`).
  - `0x18..0x19` (`u16 LE`): `Skill Points` (`player + 0x400`).
  - `0x60` (`u8`): `Reborn` (`0`=Chưa CS, `1`=CS1, `2`=TS2).
  - `0x61` (`u8`): `Job` (`1`=Hiệp, `2`=Bá, `3`=Hiền, `4`=Tiên).
  - Đuôi sau byte 113: Chuỗi `N × 3` byte `{skillId: u16 LE, level: u8}` lấy từ `session.skills`.

### 2.3. Cơ Chế Trao Kỹ Năng Kịch Bản Eve (`EveResultClass::Skill = 4`)
- Tại `src/server/handlers/talk.rs`, bổ sung nhánh `EveResultClass::Skill = 4` trong `execute_action_result`:
  - `skill_id = result.parameter`, `target_level = if result.result_value > 0 { result.result_value as u8 } else { 1 }`.
  - Hoàn toàn miễn phí `skill_point` (thưởng từ sự kiện/nhiệm vụ).
  - Cập nhật vào `session.skills` (chỉ cập nhật nếu cấp mới lớn hơn cấp hiện có), persist nguyên tử vào SQLite `character_skills`.
  - Phát gói đồng bộ `08 01 6E 01 [target_lv: u32 LE][skill_id: u32 LE]` và thông báo hệ thống / Toast cho người chơi.

### 2.4. Lưu Trữ Bi Rồng & 8 Bóng Tái Sinh 2 (`skill_rb2`)
- **Cơ sở dữ liệu SQLite**:
  - Thêm cột `dragon_balls INTEGER DEFAULT 0` (bitmask 12 bi rồng + 8-bit `ball_point`) và `rb2_slots TEXT DEFAULT ''` (JSON hoặc mảng hex 8 `u16`) vào bảng `characters`.
  - Trong `Session`: Bổ sung `ball_point: u8`, `dragon_balls: u16`, `skill_rb2: [u16; 8]`.
- **Đồng bộ S→C**:
  - Gửi `S→C 0x17 0x4D` (`ball_point` + 12 bi rồng) và `S→C 0x17 0x4E` (8 slot bóng `skill_rb2`) khi đăng nhập và ngay sau khi hoàn thành Reborn 2.
- **Cam kết `C→S 0x1C 05`**:
  - Parse đầy đủ 3 đoạn: `L1` (bi rồng gắn mới), `L2` (kỹ năng anh hùng), `L3` (các cặp `{slot: u8 (6..13), skillId: u16 LE}`). Cập nhật `session.skill_rb2` và persist vào SQLite.

### 2.5. Ràng Buộc & Thực Thi Kỹ Năng Nghề (Job Skills Gate & Combat)
- **Gate học kỹ năng (`handle_player_skill_learn` & `0x1C 04`)**:
  - 16 kỹ năng nghề (`14039..14054`) yêu cầu: `session.job` khớp chính xác với nhánh nghề (Bá=2, Hiệp=1, Hiền=3, Tiên=4), đồng thời yêu cầu kỹ năng gốc chung `14038` (Đại chu thiên) có `level >= 1`.
  - Bổ sung handler cho `C→S 0x1C 04` (chuỗi `N × 3B` `{skillId, targetLevel}`) cam kết từ `TSe_SkillTreeForm` và `TWF_TurnTwoHeroSkillForm`.
- **Thực thi trong Trận Đấu (`src/battle/`)**:
  - Khi khởi tạo thực thể tham chiến (`construction.rs:add_player`): Quét `session.skills` để áp dụng chỉ số cộng từ toàn bộ Passive nghề (`14039` Bá khí, `14041` Lực bạt sơn hề, `14043` Anh khí hộ thân, `14045` Tâm nhãn, `14047` Dụng kế, `14048` Trí thủ, `14049` Ngự địch, `14051` Tiên khiếu, `14052` Hóa Huyết Hoàn Hồn) trực tiếp vào `WarInfo` (Atk, Def, Int, Agi, HpMax, SpMax).
  - Hiện thực trọn vẹn toàn bộ 16 kỹ năng nghề cả chủ động lẫn bị động trong `runner.rs` ngay đợt này (đặc biệt `14040` Bá ý: 100% crit trong N lượt; `14044` Thiên thần thuẫn: chia sẻ/hấp thụ sát thương cho đồng đội; `14046` Ngưng khí hộ thân: giảm % sát thương nhận).

### 2.6. Hệ Thống Tẩy Điểm Respec (Opcode `0x2A`)
- Kích hoạt Level-1 Dispatcher cho `OP_RESET = 0x2A` (`src/server/dispatcher.rs`).
- Khi sử dụng vật phẩm trong `src/server/handlers/use_item/`:
  - Cẩm nang tâm tư (`46035`, `46166`): Gửi `S→C 0x2A 01` mở giao diện `TBK_SkillRegressForm`.
  - Túi gấm cẩn thận (`46203`): Gửi `S→C 0x2A 02` mở giao diện `TBK_AttrRegressForm`.
- Handler `src/server/handlers/respec.rs`:
  - `C→S 0x2A 01 { skillId: u16 LE }`: Trừ 1 cấp kỹ năng, hoàn trả điểm `skill_point`, trừ 1 Cẩm nang tâm tư trong `homdo`.
  - `C→S 0x2A 02 { attrCode: u8 }`: Trừ 1 điểm chỉ số thuộc tính, hoàn trả 1 điểm `point`, trừ 1 Túi gấm cẩn thận.
  - Tuân thủ nghiêm ngặt Two-Phase State Lock và cập nhật nguyên tử vào SQLite.

### 2.7. Ghi Chú Phạm Vi Ngoài (Out of Scope Note)
- Luồng cấp kỹ năng Quang (`22001..22022`) và Ám (`23001..23023`) (qua sách kỹ năng trong `books.rs` hoặc kịch bản Eve trao) được đánh dấu là **Out of Scope** trong đợt này và sẽ xử lý ở backlog kế tiếp.
