# BÁO CÁO NGHIÊN CỨU & ĐÁNH GIÁ TOÀN DIỆN HỆ THỐNG KỸ NĂNG: CLIENT `aLogin.exe` vs SERVER RUST `ts_dream`

**Tác giả:** Lead Game Engine Protocol & Đội ngũ Chuyên gia Subagent  
**Ngày thực hiện:** 2026-10-31  
**Tài liệu nền:** `.scratch/understand-skills/CLIENT_SKILLS_SPEC.md`, `client_pseudo_c/`, `spec/client_skills_handler.md`, `Data/skills.json`, `Data/Skill.Dat`, `Data/eve.emg`  
**Mã nguồn đối chiếu:** `src/server/handlers/skills.rs`, `src/battle/`, `src/eve/`, `src/server/handlers/talk.rs`, `src/server/spawn.rs`, `src/data/loader.rs`

---

## I. TỔNG QUAN KIẾN TRÚC LOGIC KỸ NĂNG PHÍA CLIENT (`aLogin.exe`)

Dựa trên kết quả dịch ngược chi tiết từ binary PE x86 `aLogin.exe` (Delphi 7, Image Base `0x00400000`), hệ thống kỹ năng phía client được phân tách rõ ràng như sau:

1. **Không gian ID Wire (Wire ID Space)**:
   - Dùng 100% không gian 376 bản ghi chuẩn từ `Data/skills.json` (`10000..23023`).
   - Client không tự hard-code bất kỳ ID nào ngoài 376 bản ghi này.

2. **Hai Cấu Trúc Bảng Cây Kỹ Năng Độc Lập Trong RAM**:
   - **Bảng động `0x0094B440` (765 byte)**: Phục vụ cửa sổ danh sách kỹ năng `TSe_SkillForm` (`gvar_007D9EB8`). Bảng này có 5 tab (Địa=0, Thủy=1, Hỏa=2, Phong=3, Tâm=4), mỗi tab 51 slot × 3 byte (`{skillId: u16, level: u8}`). Bảng này được nạp động từ đuôi gói `S→C 05 03` và `08 01 6E`.
   - **Cây tĩnh `TSe_SkillTreeForm` (`gvar_007D9E68`)**: Phục vụ cửa sổ nâng điểm cây kỹ năng. Tọa độ và liên kết cha-con các node là cố định. Có 10 tab: 4 tab ngũ hành Địa/Thủy/Hỏa/Phong, Tab 5 (Hệ Tâm: 37 kỹ năng sống), Tab 6..9 (4 nhánh nghề Bá/Hiệp/Hiền/Tiên).
   - **Bảng bóng Tái Sinh 2 (`DAT_0094B3A4`)**: Chứa 4 Tâm pháp gốc (`idx 1..4`), Gốc nghề `14038` Đại chu thiên (`idx 5`), và 8 slot bóng Tái Sinh 2 (`idx 6..13` = `skill_rb2[0..7]`).

3. **Cơ chế Client Gate & Wire Format**:
   - Khi học/nâng cấp kỹ năng trên giao diện cơ bản (`TSe_SkillForm`), client gửi **`C→S 0x1C 01`** (5 byte payload: `[1C 01][skillId: u16 LE][targetLevel: u8]`).
   - Khi nâng cấp cho thú cưng, client gửi **`C→S 0x1C 02`** (6 byte payload: `[1C 02][petSlot: u8 (1..4)][skillId: u16 LE][targetLevel: u8]`).
   - Khi cam kết nâng điểm hàng loạt trên cây kỹ năng `TSe_SkillTreeForm` hoặc `TWF_TurnTwoHeroSkillForm`, client gửi **`C→S 0x1C 04`** (Payload `2 + 3×N` byte: chuỗi `[skillId: u16 LE][targetLevel: u8]`).
   - Khi cam kết cây kỹ năng Tái Sinh 2, client gửi **`C→S 0x1C 05`** (`setSkillRb2`): Layout 3 đoạn:
     `[L1: u32 LE][L1 × u8 ballIdx (1..12)][L2: u32 LE][L2 × 3B {skillId, level}][L3: u32 LE][L3 byte ({slot 6..13, skillId})]`.

---

## II. ĐỐI CHIẾU CHI TIẾT VỚI SERVER RUST `ts_dream`

### 1. Phân Hệ Đăng Nhập & Đồng Bộ Thuộc Tính (`05 03` / `spawn::stats`)

* **Client mong đợi**:
  - `Payload[0x05..0x06]` (`u16 LE`): Current SP (`player + 0x3EC`).
  - `Payload[0x18..0x19]` (`u16 LE`): Điểm kỹ năng Skill Points (`player + 0x400`).
  - `Payload[0x60]` (`u8`): Cấp Chuyển sinh / Reborn (`0`=Chưa CS, `1`=CS1, `2`=TS2) -> ghi vào `player + 0x4B0`.
  - `Payload[0x61]` (`u8`): Nghề nghiệp / Job (`1`=Hiệp, `2`=Bá, `3`=Hiền, `4`=Tiên) -> ghi vào `player + 0x1271`.
  - Đuôi sau byte 113: Chuỗi `N × 3` byte `{skillId: u16 LE, level: u8}`.
* **Server Rust hiện tại (`src/server/spawn.rs:151-211`)**:
  - Hàm `stats()` hoàn toàn không nhận tham số `reborn` và `job`.
  - Tại dòng 206–207:
    ```rust
    body.push_str("F401F401F401F401F401");
    body.push_str(&"00".repeat(43));
    ```
  - **LỖI NGHIÊM TRỌNG**: Server gửi 43 byte `0x00` liên tiếp lấp đầy offset `0x46..0x70`. Do đó `Payload[0x60]` và `Payload[0x61]` luôn là `00 00`!
  - **Hệ quả**: Dù session trong server có `reborn = 2` và `job = 2`, client vẫn ghi nhận `Reborn = 0, Job = 0`. Client lập tức khóa toàn bộ Tab 6..9 trên `TSe_SkillTreeForm` với thông báo *"Không thể học kỹ năng của nghề khác"* (`@0x57DFB8`).

---

### 2. Phân Hệ Học & Nâng Cấp Kỹ Năng (`src/server/handlers/skills.rs`)

1. **Loader Data Kỹ Năng (`src/data/loader.rs:254-256`)**:
   - `data.skills` được nạp từ `Data/Skills.txt`. Tuy nhiên trong repo `Data/` **không hề có file `Skills.txt`** (chỉ có `Skill.Dat` và `skills.json`).
   - **Hệ quả**: `data.skills` luôn rỗng trong runtime! Mọi lệnh tra cứu `data.skills.get(&skill_id)` đều trả về `None`, khiến `handle_player_skill_learn` ngắt giữa chừng và không thể nâng cấp kỹ năng.

2. **Opcode `0x1C` Handler**:
   - **Sub 1 (`handle_player_skill_learn`)**:
     - Parse đúng thứ tự wire `[skill_id: u16 LE][target_lv: u8]`.
     - Tính đúng chi phí: cấp 1 tốn `point` (nhân đôi nếu khác hệ), cấp 2 trở lên tốn 1 điểm/cấp (không nhân đôi).
     - Gói phản hồi `08 01 6E 01 [target_lv: u32 LE][skill_id: u32 LE]` khớp chính xác với mã máy của `FUN_00710AB0` trong `aLogin.exe`.
     - *Thiếu sót*: Không kiểm tra `session.job` khi học 16 kỹ năng nghề (`14039..14054`), không kiểm tra tiên quyết `14038`.
   - **Sub 2 (`handle_pet_skill_upgrade`)**:
     - Đọc byte đầu là `stt`. Trong khi client gửi `petSlot (1..4)`. Nếu `stt != slot`, server tìm sai pet.
     - Đã chặn đúng: Chỉ cho nâng cấp kỹ năng đã có sẵn trong roster của thú.
   - **Sub 4 (`0x1C 04`)**:
     - **CHƯA HỖ TRỢ**. Bị nuốt chửng bởi `_ => {}`. Khi người chơi cộng nhiều điểm trên cây kỹ năng và bấm Xác nhận, server không lưu thay đổi.
   - **Sub 5 (`handle_skill_reborn_2`)**:
     - Đọc `ball_count` (L1) nhưng bỏ qua không lưu.
     - Đọc `skill_count` (L2) và nâng cấp kỹ năng vào `session.skills`.
     - **Bỏ rơi hoàn toàn đoạn L3**: Không parse độ dài `L3` và 8 slot bóng `skill_rb2[0..7]` (`slot: u8 (6..13), skillId: u16 LE`).
   - **Sub 6 (`0x1C 06`)**: Kỹ năng thuyền chiến chưa có handler.

---

### 3. Kỹ Năng Hệ Tâm & Học Qua Kịch Bản Eve Script (`src/server/handlers/talk.rs`)

* **Client Logic**:
  - Hệ Tâm (`elem = 5`, `type = 5`) gồm 62 kỹ năng (`14001..14061`).
  - Các kỹ năng sống như Đào khoáng (`14010, 14018, 14032`), Hợp thành (`14005, 14011, 14019, 14035`), Cưỡi ngựa (`14023, 14034`), Mua bán (`14003`), Triệu gọi (`14026`) được trao thông qua sự kiện đối thoại NPC trong `Data/eve.emg`.
* **Server Rust hiện tại**:
  - `src/data/loaders/eve.rs:107` đã định nghĩa `EveResultClass::Skill = 4`.
  - Trong `src/server/handlers/talk.rs` hàm `execute_action_result` (dòng 381–555):
    - `match result.result_class` chỉ xử lý: `1` (Item), `2` (Quest), `5` (Gold/Stat), `7` (Player).
    - **Mã lớp `4` (Skill) hoàn toàn không có nhánh xử lý**, rơi vào `other => tracing::debug!(...)` và bị bỏ qua.
  - **Hệ quả**: Người chơi làm chuỗi nhiệm vụ học kỹ năng hệ Tâm trong game nhưng server không trao kỹ năng.

---

### 4. Kỹ Năng Nghề (Job Skills) & Tái Sinh 2

* **Client Logic**:
  - 4 nhánh nghề: Bá (`job=2`, `14039..14042`), Hiệp (`job=1`, `14043..14046`), Hiền (`job=3`, `14047..14050`), Tiên (`job=4`, `14051..14054`).
  - Gốc chung: `14038` Đại chu thiên (Passive, 5 cấp).
  - 12 Bi rồng (`ballList`, `1..12`) đồng bộ qua **`S→C 0x17 0x4D`**.
  - 8 Slot bóng Tái Sinh 2 (`skill_rb2[0..7]`) đồng bộ qua **`S→C 0x17 0x4E`**.
* **Server Rust hiện tại**:
  - Trong `Session`: Chưa có mảng lưu 12 bi rồng (`ball_list`), chưa có số bi rồng chưa gắn (`ball_point`), chưa có 8 slot bóng (`skill_rb2: [u16; 8]`).
  - Chưa có frame builder cho `17 4D` và `17 4E`.
  - Trong battle engine (`src/battle/runner.rs`):
    - Kỹ năng nghề chủ động Type 19 (`14040` Bá ý, `14044` Thiên thần thuẫn, `14046` Ngưng khí hộ thân, `14053` Thánh linh thuật) chỉ được gán `type19_id` vào `WarInfo`, **không có bất kỳ tác dụng nào** trong tính toán sát thương hay phòng thủ.
    - Kỹ năng nghề bị động (`14039` Bá khí, `14041` Lực bạt sơn hề, `14043` Anh khí hộ thân...) không được đọc từ `session.skills` khi vào trận (`construction.rs:add_player`), không cộng chỉ số cho nhân vật.

---

### 5. Kỹ Năng Quang - Ám (Light / Dark: 22001..22022 & 23001..23023)

* **Client Logic**:
  - Quang (`type = 18`, `elem = 0`, 22 kỹ năng), Ám (`type = 19`, `elem = 0`, 23 kỹ năng).
  - Client không cần gói mở khóa riêng. Khi server cấp kỹ năng trong `05 03` hoặc `08 01 6E`, chúng xuất hiện trong menu kỹ năng chiến đấu (`TSe_FightSkillForm`).
  - Triệu gọi Bạch Hổ (`22011`) và Huyền Vũ (`23011`) gửi lệnh đánh qua `C→S 0x32 02`, client tự phát animation `eff_pic`.
* **Server Rust hiện tại**:
  - `Session.skills` có thể chứa các ID này nếu được cấp.
  - Tuy nhiên, server **không có luồng nào để cấp**: Không có sách học Quang/Ám trong `books.rs`, không có sự kiện Eve trao.
  - Trong `src/battle/combat_formula.rs:135` (`element_multiplier`):
    - Bảng tương khắc định nghĩa `(7, 8) | (8, 7) => 1.5` (mã hệ của bản mobile).
    - Nhưng trên PC, kỹ năng Quang/Ám có `elem = 0`. Khi đánh vào mục tiêu hệ `1..4`, hàm trả về hệ số `1.0` (trung tính tuyệt đối). Quang và Ám đánh nhau cũng trả về `1.0`.

---

### 6. Kỹ Năng Sống Ngoài Chiến Đấu & Hệ Thống Tẩy Điểm (Respec)

1. **Thuật mua bán (`14003`)**:
   - Khi nhận `05 03` hoặc `08 01 6E`, client kiểm tra nếu có `14003` với `level > 0` thì mở khóa nút Bày hàng trên menu tiền.
   - Nhưng trên server, handler mở quầy hàng `0x17 Sub 30` (`shops.rs:594`) **không kiểm tra `14003` trong session**.
2. **Khai khoáng (`14010, 14018, 14032`)**:
   - Server **chưa có module đào khoáng**. Chưa có logic kiểm tra công cụ Cuốc (`Item type=39`) và cấp độ kỹ năng khai khoáng.
3. **Chế tạo / Luyện đơn (`14005, 14011, 14019, 14035`)**:
   - Thuật toán `execute_compound` trong `src/crafting/basic/algorithm.rs` là hàm thuần túy, **hoàn toàn không nhận cấp độ kỹ năng sống của người chơi** để gia tăng tỷ lệ thành công.
4. **Cưỡi ngựa (`14023, 14034`)**:
   - Handler thú cưỡi `0x0F Sub 0x04` (`pet_actions.rs:254`) không kiểm tra người chơi đã học `14023` chưa. Thú cưỡi chiến trận kind 22 đòi `14034` đang bị chặn cứng bởi điều kiện `< 19000`.
5. **Hệ thống Tẩy điểm (Opcode `0x2A`)**:
   - `OP_RESET = 0x2A` bị chuyển thẳng vào module `unimplemented` trong `src/server/dispatcher.rs:375`.
   - Các vật phẩm Cẩm nang tâm tư (`46035, 46166`), Túi gấm cẩn thận (`46203`), Túi gấm hoàn kỹ (`65025`) chưa được gắn logic gửi gói mở giao diện tẩy điểm `S→C 0x2A 01`, `0x2A 02`, `0x45 0A`.

---

## III. MA TRẬN PHÂN LOẠI CÁC ĐIỂM SERVER CẦN HỖ TRỢ (GAP MATRIX)

| Mã Phân Hệ | Mô Tả Lỗi / Thiếu Sót Trên Server | Mức Độ | File & Dòng Code Liên Quan | Ảnh Hưởng Thực Tế |
|:---|:---|:---:|:---|:---|
| **GAP-P0-01** | `spawn::stats` (`05 03`) gửi `00 00` cho `reborn` và `job` tại `0x60..0x61` | 🔴 **P0 (Blocker)** | [`src/server/spawn.rs:206-207`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L206-L207) | Client luôn nhận Reborn=0, Job=0. Khóa vĩnh viễn Tab 6..9 trên cây kỹ năng. |
| **GAP-P0-02** | `data.skills` rỗng do tìm file `Skills.txt` không tồn tại | 🔴 **P0 (Blocker)** | [`src/data/loader.rs:254-256`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L254-L256) | Không tra được định nghĩa skill trong `handle_player_skill_learn`, không học được skill. |
| **GAP-P0-03** | `talk.rs` bỏ qua `EveResultClass::Skill (4)` trong kịch bản Eve | 🔴 **P0 (Blocker)** | [`src/server/handlers/talk.rs:381-550`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L381-L550) | Kịch bản NPC không thể trao bất kỳ kỹ năng sống hệ Tâm nào cho người chơi. |
| **GAP-P1-01** | Bỏ rơi Sub 4 (`0x1C 04`) trong Level-2 skill handler | 🟡 **P1 (Critical)** | [`src/server/handlers/skills.rs:92-100`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/skills.rs#L92-L100) | Bấm Xác nhận trên `TSe_SkillTreeForm` và `TWF_TurnTwoHeroSkillForm` không có tác dụng. |
| **GAP-P1-02** | Sub 5 (`0x1C 05`) không parse đoạn L3 (8 bóng `skill_rb2`) | 🟡 **P1 (Critical)** | [`src/server/handlers/skills.rs:288-340`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/skills.rs#L288-L340) | 8 ô bóng kỹ năng Tái Sinh 2 không được lưu lại khi người chơi cam kết cây Reborn 2. |
| **GAP-P1-03** | Thiếu gói đồng bộ Bi rồng `17 4D` và 8 bóng Tái sinh `17 4E` | 🟡 **P1 (Critical)** | `src/server/spawn.rs`, `src/server/handlers/skills.rs` | Client không hiển thị số bi rồng và 8 bóng tái sinh sau khi đăng nhập / Reborn 2. |
| **GAP-P1-04** | Battle engine: Kỹ năng nghề chủ động Type 19 thiếu logic cơ chế | 🟡 **P1 (Critical)** | [`src/battle/runner.rs:2871`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L2871), `damage.rs:252` | `14040` (Bá ý) không tăng crit, `14044` (Thiên thần thuẫn) không che chắn sát thương. |
| **GAP-P1-05** | Battle engine: Không nạp Passive skills vào `WarInfo` khi vào trận | 🟡 **P1 (Critical)** | [`src/battle/construction.rs:117-142`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/construction.rs#L117-L142) | Bá khí, Lực bạt sơn hề, Anh khí hộ thân, Tiên khiếu... không có tác dụng tăng chỉ số. |
| **GAP-P1-06** | `combat_formula.rs`: Sai mã hệ tương khắc Quang - Ám | 🟡 **P1 (Critical)** | [`src/battle/combat_formula.rs:135-147`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/combat_formula.rs#L135-L147) | Quang và Ám đều có `elem = 0`, bị gán hệ số 1.0x trung tính tuyệt đối, mất tính tương khắc. |
| **GAP-P2-01** | Opcode `0x2A` (`OP_RESET`) nằm trong `unimplemented` | 🟢 **P2 (Feature)** | [`src/server/dispatcher.rs:375`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L375) | Cẩm nang tâm tư / Túi gấm cẩn thận không mở được form tẩy điểm kỹ năng / thuộc tính. |
| **GAP-P2-02** | Thiếu các gate kiểm tra kỹ năng sống (Bày sạp `14003`, Cưỡi thú `14023`) | 🟢 **P2 (Integrity)** | `src/server/handlers/shops.rs`, `pet_actions.rs` | Người chơi chưa học skill vẫn gửi packet mở shop hoặc cưỡi ngựa bình thường. |
| **GAP-P2-03** | Hợp thành `execute_compound` chưa tính cấp kỹ năng sống | 🟢 **P2 (Balance)** | [`src/crafting/basic/algorithm.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/crafting/basic/algorithm.rs) | Cấp độ Luyện đơn / Tinh luyện không giúp tăng tỷ lệ thành công của vật phẩm hợp thành. |

---

## IV. ĐỀ XUẤT LỘ TRÌNH THỰC THI (ACTION PLAN)

### Giai đoạn 1: Sửa chữa Blocker Giao thức & Dữ liệu (P0 — Ngay lập tức)
1. **Sửa `loader.rs`**: Khởi tạo `data.skills` từ `Data/skills.json` (hoặc copy từ `SkillDatLoaderPc`).
2. **Sửa `spawn.rs`**: Truyền `conn.session.reborn` vào offset `0x60` và `conn.session.job` vào offset `0x61` trong gói `05 03`.
3. **Bổ sung `talk.rs`**: Thêm nhánh `4 => { ... }` vào `execute_action_result` để nạp kỹ năng từ Eve action.

### Giai đoạn 2: Hoàn thiện Gói Kỹ Năng & Tái Sinh 2 (P1)
1. Thêm nhánh Sub 4 (`0x1C 04`) trong `handle_skills` để nhận mảng `N × 3B`.
2. Mở rộng `handle_skill_reborn_2` (`0x1C 05`): Parse `L1` (bi rồng) và `L3` (8 slot bóng `skill_rb2`).
3. Mở rộng struct `Session`: Thêm `ball_point: u8`, `ball_list: [bool; 12]`, `skill_rb2: [u16; 8]`.
4. Viết hàm gửi `S→C 0x17 0x4D` và `S→C 0x17 0x4E` khi login và sau khi Reborn 2.
5. Cập nhật `combat_formula.rs`: Kiểm tra `skill.type ∈ {18, 19}` để áp dụng tương khắc 1.5x giữa Quang và Ám.

### Giai đoạn 3: Bổ sung Cơ chế Chiến đấu Kỹ Năng Nghề & Kỹ Năng Sống (P1 & P2)
1. Cập nhật `add_player` trong `construction.rs` để cộng chỉ số từ các passive nghề vào `WarInfo`.
2. Hiện thực cơ chế buff của `14040` (Bá ý: 100% crit) và `14044` (Thiên thần thuẫn: chuyển sát thương) trong `runner.rs`.
3. Hiện thực handler `0x2A` cho Cẩm nang tâm tư (tẩy điểm kỹ năng) và Túi gấm cẩn thận (tẩy stat).
4. Thêm chốt chặn kỹ năng `14003` khi mở sạp hàng và `14023/14034` khi cưỡi thú.
