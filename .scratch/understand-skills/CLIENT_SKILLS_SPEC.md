# BÁO CÁO PHÂN TÍCH NGHIỆP VỤ MODULE "KỸ NĂNG (SKILLS)" — CLIENT `aLogin.exe` (TS ONLINE)

**Đối tượng phân tích:** Client `aLogin.exe` (Delphi 7, PE x86, Image Base `0x00400000`)
**Dữ liệu đối chiếu:** Bundle Ghidra C/ASM/HEX (`client_skills_handler/`, qua 12 đợt redump Round 1..12), `Data/Skill.Dat` (376 bản ghi canonical & 352 bản ghi legacy), `Data/skills.json` (376 bản ghi), `Data/Item.Dat` (8.372 vật phẩm), `Data/Npc.dat` (6.659 NPC), `Data/eve.emg` (2.777 scene sự kiện), và mã nguồn C# Server tham chiếu (`TSCharacter.cs`, `Packet1CHandler.cs`, `resetHandler.cs`).

**Quy ước mức độ bằng chứng:**
* ✅ **VERIFIED**: Đã đối chiếu khớp tuyệt đối tới từng lệnh ASM / byte offset trong `.asm.txt`, `.hex`, hoặc binary `.dat`.
* ⚠️ **INFERRED**: Suy luận vững chắc từ ≥ 2 nguồn độc lập (call-graph + cấu trúc dữ liệu + đối chiếu C# server).
* ❓ **OPEN**: Hạng mục nằm ngoài file thực thi `aLogin.exe` (như chữ vẽ trong file ảnh `.bmp` ngoài) hoặc hàm phụ chưa cần thiết, kèm đề xuất đóng cụ thể.

## 1. Tóm tắt điều hành (Executive Summary — Ưu tiên phía Server)

* **Không gian ID kỹ năng trên đường truyền (Wire ID Space) là 100% `Data/skills.json` (376 bản ghi)** (✅ **VERIFIED**): Đã chứng minh trên **5 nguồn độc lập** (`aLogin.exe` tại `FUN_006538BC` / `FUN_005990E0`, `Data/Item.Dat` trường `skill_link` `+0x67`, `Data/Npc.dat` trường kỹ năng `+0x42..+0x46`, `Data/eve.emg` hành động `Action -> Skill`, và C# Server `TSCharacter.cs`). Toàn bộ đều dùng hệ ID `10000..23023` của `skills.json`. Bản `Skill_legacy_352.Dat` cũ bị lệch `±16`/`±48` là của bản build khác; `Data/Skill.Dat` hiện đã được dựng lại từ `skills.json` (376 bản ghi, map identity 1:1, 0 node mồ côi).
* **Gói học/nâng cấp kỹ năng nhân vật `C→S 0x1C 01` có độ dài 3 byte `{ skillId: u16 LE }{ targetLevel: u8 }`** (✅ **VERIFIED** — `FUN_005990E0` tại `0x005991DF..0x0059924F`, file `005990e0_FUN_005990e0.asm.txt`): `skillId` đứng **trước** `targetLevel` (`targetLevel = currentLevel + 1`). Bác bỏ hoàn toàn giả định cũ `[level:u8][skillId:u16]`.
* **Gói `C→S 0x1C 02` là "Học/nâng cấp kỹ năng cho THÚ CƯNG (`TFollowNpc`)" (4 byte)** (✅ **VERIFIED** — `FUN_005990E0` tại `0x00599170` & `FUN_0071D234` tại `0x0071D234`, file `0071d234_FUN_0071d234.asm.txt`): Layout chính xác là `{ petSlot: u8 (1..4) }{ skillId: u16 LE }{ targetLevel: u8 }`, trong đó `petSlot` là vị trí thú trong `player->0x57C[1..4]`. Khớp 100% với `pet[data[2]-1].setSkill(...)` của C# server.
* **Gói cam kết cây kỹ năng Tái Sinh 2 `C→S 0x1C 05` khớp 100% với `TSCharacter.setSkillRb2()`** (✅ **VERIFIED** — `FUN_0057C840` tại `0x0057CDB6` & `FUN_00583A98` tại `0x00583A98`, file `0057c840_FUN_0057c840.asm.txt`): Layout gồm 3 đoạn có độ dài tiền tố `u32 LE`: `[L1: u32 LE][L1 × u8 ballIdx (1..12)][L2: u32 LE][L2 × 3B {skillId: u16 LE, level: u8}][L3: u32 LE][L3 byte ({slot: u8 (6..13), skillId: u16 LE})]`. **Lưu ý cực kỳ quan trọng cho Server**: `L2` là **số lượng kỹ năng** (tổng số byte đoạn 2 là `L2 × 3`), trong khi `L3` là **tổng số byte** của đoạn 3 (số bóng = `L3 / 3`, với `slot` trên wire là `6..13` tương ứng `skill_rb2[0..7]` trên server).
* **Gói cam kết học kỹ năng thường trên cây `C→S 0x1C 04` có layout `N × { skillId: u16 LE, targetLevel: u8 }`** (✅ **VERIFIED** — `FUN_0057C840` tại `0x0057CB92` & `FUN_00586AAC` trên `TWF_TurnTwoHeroSkillForm`): Gửi chuỗi bội số 3 byte không có header đếm, khi người chơi nâng nhiều cấp/kỹ năng cùng lúc trên `TSe_SkillTreeForm` mà không thay đổi bi rồng/bóng tái sinh.
* **Client KHÔNG gửi kèm bất kỳ byte hệ (`elem`), hệ số nhân giá, hay nhóm (`type`) nào khi học kỹ năng** (✅ **VERIFIED** — `FUN_005990E0`, `FUN_0057C840`): Dù học kỹ năng Ngũ hành, hệ Tâm, nghề Chuyển sinh hay Quang/Ám, client chỉ gửi `skillId (u16 LE)` và `targetLevel (u8)`. Server phải tự tra bảng kỹ năng để trừ đúng số điểm kỹ năng.
* **Giải quyết dứt điểm `player+0x3EC` vs `player+0x400` và 2 trường giá điểm trong `TFSkill`** (✅ **VERIFIED** — `FUN_0059F090` tại `0x0059F090`, `FUN_0057D0C8` tại `0x0057D0C8`, `FUN_00710AB0` tại `0x007110E0`):
  * `player+0x3EC` (`u16`) là **Nội lực / SP hiện tại (`Current SP`)**, dùng để so sánh với `TFSkill.require_sp (+0x1C RAM)` nhằm tô đỏ nút kỹ năng hoặc báo *"SP không đủ"* (`@0x598EF3`).
  * `player+0x400` (`u16`) mới là **Điểm kỹ năng còn lại (`Skill Points`)**, nhận từ `05 03` (`payload[0x18..0x19]`) và `08 01` prop `0x25`.
  * Hai khái niệm *"Điểm học được"* và *"Điểm thăng cấp"* nằm trong cấu trúc `TFSkill`: **`TFSkill+0x2C` (`sk_point`)** là giá học từ cấp `0 → 1` (nhân `×2` nếu khác hệ nhân vật), còn **`TFSkill+0x2D` (`grade_point`)** là giá nâng mỗi cấp từ `1 → 2..max_lvl` (luôn bằng `grade_point`, thường là `1`, **không** nhân đôi khi khác hệ).
* **Bác bỏ hoàn toàn giả thuyết cũ về `0x36B3` (`14003`), `S→C 19 0B`, và `S→C 20 01/02`** (✅ **VERIFIED**):
  * **`0x36B3` (`14003` Thuật mua bán)** KHÔNG phải cờ mở khóa Quang/Ám! Tại `0051189c_FUN_0051189c.c:1347,1576`, `gvar_007DA108` là form **`TCY_MoneyMenu`** và `+0x13C` là nút **"Bày hàng"**; `gvar_007DA530` là **`TSe_MainStatus`** và `+0x1B8` là cờ nhấp nháy nút menu tiền. Khi nhận `14003` (`level > 0`) qua `05 03` hoặc `08 01 prop 0x6E`, client bật nút Bày hàng; khi bấm dùng `14003` trong `TSe_SkillForm`, `FUN_00598C28` gọi `FUN_005C182C()` để mở sập hàng.
  * **`S→C 19 0B` (`FUN_0075B8E0`)** KHÔNG xóa 3000 ô kỹ năng! Opcode `0x19` là nhóm gói Giao dịch (`TCY_TradeMenu` / `TCY_TradeFNpcMenu`); `19 0B` hiển thị thông báo hủy/lỗi giao dịch võ tướng với tham số `3000` là **3000 ms thời gian hiện thông báo**. Việc xóa sạch 3000 byte bảng kỹ năng (`player+0xFD4`, 1000 slot × 3B) thực chất nằm ở đầu hàm `FUN_0072E9A0` (`S→C 05 03` tại `0x0072F328`).
  * **`S→C 20 01 / 20 02` (`case_028_0079285A`)** là gói biểu cảm động tác nhân vật (**`TCY_ActPose`**), hoàn toàn không liên quan kỹ năng Quang/Ám.
* **Cơ chế kích hoạt và học kỹ năng Quang/Ám (`type=18` Quang, `type=19` Ám, `elem=0`)** (✅ **VERIFIED** — `FUN_00597954`, `FUN_00598944`, `FUN_0076D578`): Không có gói mạng riêng để "mở khóa" Quang/Ám. Vì `elem = 0`, chúng không bị chặn bởi hàm khắc hệ `FUN_00598944`, nhưng cũng không thuộc 5 tab `0..4` (`elem 1..5`) của `TSe_SkillForm`. Để cấp kỹ năng Quang/Ám cho người chơi, Server chỉ cần gửi `{skillId, level}` trong đuôi `05 03` hoặc qua `08 01` prop `0x6E` (`deltaType=1`).
* **Giải nghĩa hoàn toàn `0x36C7` (`14023` Cưỡi ngựa) và `0x36D2` (`14034` Kỵ Binh)** (✅ **VERIFIED** — `FUN_0073D558` tại `0x0073D5B6..0x0073D5E8`, `FUN_005A1620` tại `0x005A177C`, `FUN_0063CF58` tại `0x0063EC5A`): Khi cưỡi thú có `Npc.dat b15 == 9` (72 ngựa/thú cưỡi thường, ID `18001..18072`), client bắt buộc nhân vật đã học `14023` (**Cưỡi ngựa**). Khi cưỡi thú có `Npc.dat b15 == 22` (`0x16` — 6 Chiến Tượng/chiến mã, ID `31001..31006`), client bắt buộc nhân vật đã học `14034` (**Kỵ Binh**), nếu chưa học sẽ chặn và báo *"Phải học thuật kỵ binh"* (`@0x5A191C`).
* **Thứ tự 4 nhánh nghề trên Client (`Bá – Hiệp – Hiền – Tiên`) ngược vị trí 1↔2 so với mã `job` của Server (`1=Hiệp, 2=Bá, 3=Hiền, 4=Tiên`)** (✅ **VERIFIED** — `FUN_00579E80`, `FUN_0057A858`, `FUN_0057B2D8`, `FUN_0057BD58` & `FUN_0057D0C8` tại `0x0057DF4E`): Trong `S→C 05 03`, server gửi `job` tại `payload[0x61]` (lưu vào `player+0x1271`), và `reborn` tại `payload[0x60]` (lưu vào `player+0x4B0`). Client ánh xạ: Tab 6 (`self->0x24C[0]`, `14039..14042` **Bá**) ↔ `job=2`; Tab 7 (`self->0x24C[1]`, `14043..14046` **Hiệp**) ↔ `job=1`; Tab 8 (`self->0x24C[2]`, `14047..14050` **Hiền**) ↔ `job=3`; Tab 9 (`self->0x24C[3]`, `14051..14054` **Tiên**) ↔ `job=4`.
* **Phát hiện 2 gói `S→C 0x17 0x4D` và `S→C 0x17 0x4E` điều khiển Bi rồng & 8 Bóng Tái Sinh 2 (`skill_rb2`)** (✅ **VERIFIED** — `case_020_007902FB_FUN_007902fb.c:308-313`, `FUN_00749FD4` tại `0x00749FD4`):
  * `S→C 0x17 0x4D` (`FUN_00748FAC`): Gửi `{ ball_point: u8 }{ activeBalls: K × u8 (1..12) }` ghi vào `player+0x1480` và mảng `player+0x12D4[1..12]`.
  * `S→C 0x17 0x4E` (`FUN_00749FD4`): Khi `player->0x4B0 >= 2`, đọc `M` bộ 3 byte `{ slot: u8 (1..8), skillId: u16 LE }` ghi vào `DAT_0094B3A4[slot + 5]` (chỉ số `6..13`) và vẽ lại `TSe_SkillTreeForm`.
  * Ngoài ra, client có form riêng **`TWF_TurnTwoHeroSkillForm`** (`gvar_007DA2C4`, resource `"form_HeroSkill"`, `FUN_00586370` / `FUN_005876F0`) hiển thị lưới 32 kỹ năng bóng tái sinh (`10027..13033`) để chọn/nâng cấp và gửi qua `0x1C 04` (`FUN_00586AAC`).
* **Cơ chế Triệu gọi 4 hệ Địa/Thủy/Hỏa/Phong (`10016`, `11016`, `12016`, `13015`) và 8 Ngọc/Văn Chương triệu gọi** (✅ **VERIFIED** — `FUN_0057E55C` tại `0x0057EC94`, `FUN_005A0388` tại `0x005A058A`, `FUN_0058E0BC` tại `0x0058E220`):
  * Trên `TSe_SkillTreeForm`, mỗi tab hệ `0..3` có một node triệu gọi mặc định mang ID placeholder **`0x36CA` (`14026` Triệu gọi)**. Khi người chơi đã học kỹ năng triệu gọi của hệ đó (trong bảng `gvar_007D78C0`, gồm 3 bậc mỗi hệ: `10016..10018`, `11016..11018`, `12016..12018`, `13015..13017`), `FUN_0057E55C` tự động thay thế icon và ID của node `0x36CA` bằng kỹ năng triệu gọi đã học!
  * Đối với Thú cưng (`TFollowNpc`), nếu thú sở hữu kỹ năng triệu gọi (`10016/11016/12016/13015`) nhưng vật phẩm đang đeo ở slot phụ kiện của thú (`pet->0x44->id`) **không phải** là 1 trong 8 vật phẩm Ngọc/Văn Chương triệu gọi (`23086..23089` hoặc `23135..23138` trong `Item.Dat`), client sẽ ẩn kỹ năng triệu gọi thật thành `14026` trên cây kỹ năng thú và khóa nút phím tắt chiến đấu của thú!
* **Xác minh "Dung thọ (nấu ăn)" KHÔNG tồn tại trong TS Online** (✅ **VERIFIED**): Quét toàn bộ `Skill.Dat`, `skills.json`, và 22.138 chuỗi VISCII của `aLogin.exe` xác nhận không có kỹ năng "Dung thọ". Việc chế biến thức ăn hồi HP/SP và đơn dược được thực hiện qua hệ thống **Hợp thành / Chế tạo (`TCY_MixMenu`, `gvar_007DA374`, gửi `C→S 0x1A 04`)** dựa trên chuỗi 4 kỹ năng sống hệ Tâm: `14005` (Thuật luyện đơn) → `14011` (Thuật đúc luyện) → `14019` (Thuật tinh luyện) → `14035` (Đúc thành), cùng với Lò sự kiện (`C→S 0x33 01`).
* **Hệ thống Tẩy điểm (`0x2A`) và Xóa kỹ năng con cái (`0x45 0A`)** (✅ **VERIFIED** — `FUN_005E4C14`, `FUN_005E68B0`, `case_038_00794719`, `case_061_00795F82`):
  * Khi dùng **Cẩm nang tâm tư** (`46035` / `46166`), server gửi `S→C 0x2A 01` mở `TBK_SkillRegressForm` (`gvar_007DA1D8`); khi người chơi chọn kỹ năng và bấm OK, `FUN_005E4C14` kiểm tra không còn kỹ năng con có `level > 0` rồi gửi **`C→S 0x2A 01 { skillId: u16 LE }`**.
  * Khi dùng **Túi gấm hoàn kỹ** (`65025`), server gửi `S→C 0x45 0A` mở `TLH_ChildSkillDelForm` (`gvar_007D9E3C`); khi bấm OK, client gửi **`C→S 0x2A 04 { childSlot: u8 }{ skillId: u16 LE }`**.
* **Sử dụng kỹ năng trong chiến đấu gửi qua `C→S 0x32 02` (6 byte)** (✅ **VERIFIED** — `TFightHum::FUN_0063CF58` tại `0x0063EE0B`, file `0063cf58_FUN_0063cf58.asm.txt`): Builder `0x32 01` (`0x00788C38`) là **code chết** (0 caller). Trong thực tế, khi người chơi chọn kỹ năng từ menu chiến đấu (`unit+0x5A8`) và click mục tiêu, `FUN_0063CF58` gọi `FUN_00788E06` để gửi **`C→S 0x32 02 { srcRow: u8 }{ srcCol: u8 }{ dstRow: u8 }{ dstCol: u8 }{ skillId: u16 LE }`**.

## 2. Trả lời trực tiếp 13 câu hỏi bắt buộc (Checklist §3)

| # | Câu hỏi | Trạng thái | Câu trả lời tổng hợp & Bằng chứng |
|:-:|---|:-:|---|
| **1** | Client có gửi byte hệ/hệ-số/nhóm khi học kỹ năng không? (Quang/Ám/Tâm) | ✅ **VERIFIED** | **KHÔNG.** Mọi gói học kỹ năng (`0x1C 01`, `0x1C 02`, `0x1C 04`, `0x1C 05`, `0x1C 06`) chỉ gửi `skillId (u16 LE)` và `targetLevel (u8)` (kèm `petSlot` ở `1C 02` hoặc `ball/slot` ở `1C 05`). Không có byte `elem`, `type` hay hệ số nhân giá trên wire (`005990e0_FUN_005990e0.asm.txt`, `0057c840_FUN_0057c840.asm.txt`). |
| **2** | Layout byte chính xác của gói `0x1C 01` là gì? | ✅ **VERIFIED** | **3 byte: `{ skillId: u16 LE }{ targetLevel: u8 }`** (`targetLevel = currentLevel + 1`). Xây bởi `FUN_005990E0` tại `0x005991DF..0x0059924F` và gửi qua `FUN_00785DD7`. |
| **3** | Layout byte chính xác của gói `0x1C 05` là gì? | ✅ **VERIFIED** | **`[u32 L1][L1×u8 ballIdx (1..12)][u32 L2][L2×3B {skillId u16 LE, lvl u8}][u32 L3][L3 byte ({slot u8 6..13, skillId u16 LE})]`**. Xây bởi `FUN_0057C840` (`0x0057CDB6`) + `FUN_00583A98`, khớp 100% `TSCharacter.setSkillRb2()`. |
| **3b** | `0x1C 02` là "học nhiều kỹ năng" hay "học kỹ năng cho THÚ"? | ✅ **VERIFIED** | **Học kỹ năng cho THÚ CƯNG (4 byte): `{ petSlot: u8 (1..4) }{ skillId: u16 LE }{ targetLevel: u8 }`**. Byte đầu là kết quả `FUN_0071D234(player, npcId)` tìm vị trí thú trong `player->0x57C[1..4]`. |
| **4** | Layout byte chính xác của `0x1B 01` là gì và nghiệp vụ là gì? | ✅ **VERIFIED** | **3 byte: `{ npcIndex: u8 }{ npcId: u16 LE }`**. Nguồn từ `TInterForm+0x152` (`u8` chỉ số NPC trong danh sách) và `TInterForm+0x154` (`u16` mã NPC `1..1250`), gửi tại `FUN_00562624` (`0x00562772`) qua `FUN_00786011` khi tương tác NPC dạy kỹ năng/nhiệm vụ. |
| **5** | `0x36B3` / `0x36C7` / `0x36D2` là gì — skill id thật, sentinel, hay flag? | ✅ **VERIFIED** | **Cả 3 đều là Skill ID thật** trong không gian canonical `skills.json`:<br>• **`0x36B3` = `14003` (Thuật mua bán)**: Khi học (`level > 0`), client bật nút **Bày hàng** (`gvar_007DA108+0x13C` trong `TCY_MoneyMenu`) và nhấp nháy `gvar_007DA530+0x1B8` (`TSe_MainStatus`). **KHÔNG phải cờ Quang/Ám!**<br>• **`0x36C7` = `14023` (Cưỡi ngựa)**: Kiểm tra tại `FUN_0073D558` khi cưỡi 72 ngựa thường (`Npc.dat b15=9`, ID `18001..18072`).<br>• **`0x36D2` = `14034` (Kỵ Binh)**: Kiểm tra tại `FUN_0073D558`, `FUN_005A1620`, `FUN_0063CF58` khi cưỡi 6 Chiến Tượng/chiến mã (`Npc.dat b15=22`, ID `31001..31006`). |
| **6** | Thứ tự skill trong tail `05 03` có quyết định vị trí ô trong cây không? | ✅ **VERIFIED** | **CÓ đối với bảng động `0x94B440` (`TSe_SkillForm`), và KHÔNG đối với cây tĩnh `TSe_SkillTreeForm` (`0x94B330` / `0x24C`)**:<br>• Trong `0x94B440` (`5 tab × 51 slot × 3B`), `FUN_005A0388` chỉ cố định 4 gốc `10001/11001/12001/13001` ở `slot 0` của mỗi tab `0..3`, sau đó duyệt tuần tự `player+0xFD4[0..999]` (theo đúng thứ tự tail `05 03`) và gọi `FUN_00597954` nhét từng skill vào **ô trống đầu tiên** của tab `elem - 1`.<br>• Ngược lại, trên `TSe_SkillTreeForm`, vị trí các node được dựng cố định bởi `FUN_00577CCC..FUN_0057BD58`, không phụ thuộc thứ tự wire. |
| **7** | `player+0x3EC` và `player+0x400` — ý nghĩa chính xác? | ✅ **VERIFIED** | • **`player+0x3EC` (`u16`) = Nội lực / SP hiện tại (`Current SP`)**, được `FUN_0059F090`, `FUN_0058E0BC`, `FUN_00598C28` so sánh với `TFSkill.require_sp (+0x1C RAM)`.<br>• **`player+0x400` (`u16`) = Điểm kỹ năng còn lại (`Skill Points`)**, nhận từ `05 03` (`+0x18`) và `08 01` prop `0x25`.<br>• Hai loại chi phí *"Điểm học được"* (`sk_point`, tốn khi học cấp `0→1`) và *"Điểm thăng cấp"* (`grade_point`, tốn khi nâng cấp `1→2..max`) nằm ở **`TFSkill+0x2C`** và **`TFSkill+0x2D`** trong RAM. |
| **8** | Client có tính giá điểm học không, hay server tính hết? | ✅ **VERIFIED** | **Client CÓ tự tính và kiểm tra chặn (gate) trước khi gửi!** Trong `FUN_0059F090` (`TSe_SkillForm`) và `FUN_0057D0C8` (`TSe_SkillTreeForm`): nếu `level == 0`, giá = `sk_point` (cùng hệ) hoặc `sk_point × 2` (khác hệ); nếu `level > 0`, giá = `grade_point` (`+0x2D`, thường bằng 1, **không** nhân đôi khi khác hệ). Nếu `player->0x400 < cost`, client ẩn nút `+` hoặc báo *"Điểm kỹ năng không đủ"* (`@0x57DF10`). |
| **9** | 4 nhánh nghề Bá/Tiên/Hiền/Hiệp — tên hiển thị và thứ tự trong client là gì? | ✅ **VERIFIED** | • Nhãn của 5 tab `5..9` là **tên ảnh BMP** `"5btn_02"`..`"9btn_02"` (`FUN_005AC75C` tại `0x005AC82C`), vẽ trong file ảnh ngoài.<br>• Thứ tự 4 tab nhánh nghề trong client (`FUN_00579E80` / `0057A858` / `0057B2D8` / `0057BD58` và `Item.Dat` `23257..23272`) là: **Tab 6 = Bá (`14039..14042`, `job=2`) · Tab 7 = Hiệp (`14043..14046`, `job=1`) · Tab 8 = Hiền (`14047..14050`, `job=3`) · Tab 9 = Tiên (`14051..14054`, `job=4`)**. |
| **10** | "Triệu gọi theo hệ Thủy/Hỏa/Địa/Phong" nằm ở đâu trong client? | ✅ **VERIFIED** | Nằm tại bảng hằng **`gvar_007D78C0`** (4 hệ × 3 bậc: Địa `10016..10018`, Thủy `11016..11018`, Hỏa `12016..12018`, Phong `13015..13017`), được xử lý tại:<br>1) `FUN_0057E55C` (`0x0057EC94`): thay thế node placeholder `0x36CA` (`14026` Triệu gọi) trên 4 tab hệ của `TSe_SkillTreeForm` bằng kỹ năng triệu gọi đã học.<br>2) `FUN_005A0388` (`0x005A058A`), `FUN_00584AD8` (`0x00584D24`), `FUN_0058E0BC` (`0x0058E220`): kiểm tra thú cưng có đeo 1 trong 8 Ngọc/Văn Chương triệu gọi (`Item.Dat` `23086..23089`, `23135..23138`) không. |
| **11** | "Dung thọ" (nấu ăn) có tồn tại không? Nếu không thì hệ thống nào thay thế? | ✅ **VERIFIED** | **KHÔNG tồn tại** trong `Skill.Dat`, `skills.json` lẫn `aLogin.exe`. Được thay thế bởi hệ thống **Hợp thành / Chế tạo (`TCY_MixMenu`, `gvar_007DA374`, gửi `C→S 0x1A 04`)** dựa trên 4 kỹ năng: `14005` (Thuật luyện đơn), `14011` (Thuật đúc luyện), `14019` (Thuật tinh luyện), `14035` (Đúc thành) và Lò sự kiện (`C→S 0x33 01`). |
| **12** | Frame nào xóa toàn bộ danh sách kỹ năng phía client (`19 0B`)? Khi nào server cần gửi? | ✅ **VERIFIED** | **Đính chính quan trọng: `19 0B` KHÔNG xóa kỹ năng!** `19 0B` (`FUN_0075B8E0`) là gói báo hủy giao dịch võ tướng (`TCY_TradeFNpcMenu`, `3000` là `3000 ms` hiện thông báo). Frame thực sự xóa sạch 3000 byte kỹ năng (`_FillChar(player+0xFD4, 3000, 0)`) là **`S→C 05 03`** (`FUN_0072E9A0` tại `0x0072F328`). Khi đăng nhập hoặc sau khi Chuyển sinh / Tái sinh / Reset toàn bộ skill, server gửi lại `05 03` (hoặc xóa từng skill bằng `08 01` prop `0x6E` với `deltaType=2`, level về `0`). |
| **13** | Có frame S→C nào khác mà bản đồ opcode hiện tại đã bỏ sót không? | ✅ **VERIFIED** | **CÓ — đã phát hiện thêm 7 nhóm gói S→C quan trọng** qua quét toàn bộ 65 case dispatcher:<br>1) **`08 02` (sel `04`)**: Cập nhật skill (`0x6E`) và skill point (`0x25`) cho **Thú cưng** (`case_009_0078D125`).<br>2) **`17 15 / 16 / 17`**: Làm mới form kỹ năng Thú cưng (`case_020_007902FB`).<br>3) **`17 4D`**: Gửi `ball_point` (`+0x1480`) và danh sách 12 bi rồng (`+0x12D4`) (`case_020_007902FB` → `FUN_00748FAC`).<br>4) **`17 4E`**: Gửi 8 slot kỹ năng bóng Tái Sinh 2 (`skill_rb2`) vào `DAT_0094B3A4[6..13]` (`FUN_00749FD4`).<br>5) **`24 14`**: Mở `TShipSkillTreeForm` (`case_032_0079346B`).<br>6) **`2A 01 / 02 / 03 / 04`**: Mở/phản hồi tẩy điểm kỹ năng & thuộc tính (`case_038_00794719`).<br>7) **`45 0A`**: Mở/phản hồi xóa kỹ năng Võ tướng đời 2 (`TLH_ChildSkillDelForm`, `case_061_00795F82`). |

## 3. Bản đồ Opcode ↔ Hàm ↔ File (C→S và S→C)

### 3.1 Chiều Client → Server (C→S Builders & Callsites)

Mọi gói C→S đều đi qua hàm đóng khung và mã hóa `FUN_0077F414(encoder=EAX, mainOp=DL, subOp=CL)` (`0077f414_FUN_0077f414.asm.txt`), sau đó đẩy vào hàng đợi gửi `CY_AddSedQueue` (`FUN_0051633C`).

| Main OP | Sub OP | Builder VA (trong `0x785000..0x789000`) | Hàm nghiệp vụ gọi (Caller VA & File) | Form / Đối tượng UI nguồn | Ý nghĩa nghiệp vụ (✅ VERIFIED) |
|:-:|:-:|---|---|---|---|
| `0x1C` | `0x01` | `0x00785DD7` (`cs_1C_785dd7_785fff.asm.txt`) | `FUN_005990E0` @ `0x0059924F` (`005990e0_FUN_005990e0.asm.txt`) | `TSe_SkillForm` (`gvar_007D9EB8`, buffer `+0x1A8`) | Học mới hoặc nâng 1 cấp kỹ năng cho **Nhân vật** (`{skillId:u16 LE, targetLvl:u8}`). |
| `0x1C` | `0x02` | `0x00785E45` (`cs_1C_785dd7_785fff.asm.txt`) | `FUN_005990E0` @ `0x005991CA` (`005990e0_FUN_005990e0.asm.txt`)<br>`FUN_0058484C` / `FUN_00585B20` (`0058484c_FUN_0058484c.c`) | `TSe_SkillForm` (`gvar_007D9EB8`) / `TSe_NPCSkillTreeForm` (`gvar_007D9E30`) | Học mới hoặc nâng cấp kỹ năng cho **Thú cưng (`TFollowNpc`)** (`{petSlot:u8, skillId:u16 LE, targetLvl:u8}`). |
| `0x1C` | `0x04` | `0x00785EB3` (`cs_1C_785dd7_785fff.asm.txt`) | `FUN_0057C840` @ `0x0057CB92` (`0057c840_FUN_0057c840.c`)<br>`FUN_00586AAC` (`00586370_FUN_00586370.c:111`) | `TSe_SkillTreeForm` (`gvar_007D9E68`, `+0x290`)<br>`TWF_TurnTwoHeroSkillForm` (`gvar_007DA2C4`) | Cam kết học/nâng cấp hàng loạt kỹ năng trên cây kỹ năng thường hoặc bảng kỹ năng anh hùng Tái Sinh 2 (`N × {skillId:u16 LE, targetLvl:u8}`). |
| `0x1C` | `0x05` | `0x00785F21` (`cs_1C_785dd7_785fff.asm.txt`) | `FUN_0057C840` @ `0x0057CDB6` + `FUN_00583A98` (`00583a98_FUN_00583a98.c`) | `TSe_SkillTreeForm` (`gvar_007D9E68`, `+0x288/+0x28C/+0x290`) | Cam kết cây kỹ năng **Tái Sinh 2 (Reborn 2)**: gửi 3 danh sách `L1` (bi rồng), `L2` (skill + level), `L3` (8 slot bóng `skill_rb2`). |
| `0x1C` | `0x06` | `0x00785F8F` (`cs_1C_785dd7_785fff.asm.txt`) | `FUN_00785F8F` (wrapper gọi `FUN_0077F414`) | Dialog nâng cấp đặc biệt | Mở khóa/nâng cấp kỹ năng đặc biệt theo `skillId: u16 LE` (2 byte). |
| `0x1B` | `0x01` | `0x00786011` (`cs_1B_786011_78612f.asm.txt`) | `FUN_00562624` @ `0x00562772` (`00562624_FUN_00562624.c`) | `TInterForm` (`gvar_007DA5A8`, `+0x152/+0x154`) | Chọn NPC trong danh sách tương tác (`{npcIndex:u8, npcId:u16 LE}`). |
| `0x1B` | `0x02` | `0x007860C8` (`cs_1B_786011_78612f.asm.txt`) | `FUN_007860C8` | `TThingSaleForm` (`gvar_007DA720`, `+0x160/+0x162`) | Tương tác NPC phụ theo `npcId: u16 LE`. |
| `0x2A` | `0x01` | `0x0078848A` (`asm_cs2a_78848a_builders.asm.txt`) | `FUN_005E4C14` @ `0x005E54EE` (`005e4c14_FUN_005e4c14.c`) | `TBK_SkillRegressForm` (`gvar_007DA1D8`) | Tẩy (hạ 1 cấp) kỹ năng `skillId: u16 LE` bằng Cẩm nang tâm tư (`46035`/`46166`). |
| `0x2A` | `0x02` | `0x007884F8` (`asm_cs2a_78848a_builders.asm.txt`) | `FUN_005E68B0` (`005e5e9c_FUN_005e5e9c.c:118`) | `TBK_AttrRegressForm` (`gvar_007DA0F8`) | Tẩy điểm thuộc tính (`attrCode: u8`) bằng Túi gấm cẩn thận (`46203`). |
| `0x2A` | `0x03` | `0x00788566` (`asm_cs2a_78848a_builders.asm.txt`) | `FUN_005E4638` / `FUN_005E5E9C` | `TBK_SkillRegressForm` / `TBK_AttrRegressForm` | Báo đóng cửa sổ tẩy điểm (`{flag: u8}`). |
| `0x2A` | `0x04` | `0x007885D4` (`asm_cs2a_78848a_builders.asm.txt`) | `FUN_005E7F90` (`005e7a14_FUN_005e7a14.c`) | `TLH_ChildSkillDelForm` (`gvar_007D9E3C`) | Xóa 1 kỹ năng của Võ tướng đời 2 (`{childSlot:u8, skillId:u16 LE}`) bằng Túi gấm hoàn kỹ (`65025`). |
| `0x28` | `0x01` | `0x00785CC2` (`cs_28_29_785cc2_785dd6.asm.txt`) | `FUN_005AB678` @ `0x005AB7FA` (`005ab678_FUN_005ab678.c`) | `TSe_QuickSkillForm` (`gvar_007D9EC0`) | Gán kỹ năng vào thanh phím tắt F1..F10 (`{keyIdx:u8, isPet:u8, skillId:u16 LE}`). |
| `0x29` | `0x01` | `0x007888E0` (`cs_29_7888e0_78896f.asm.txt`) | `FUN_0058ED70` @ `0x0058F1AC` (`0058ed70_FUN_0058ed70.c`) | `TSe_FightHotKeyForm` | Gán kỹ năng vào thanh phím tắt chiến đấu (`{slotIdx:u8, unitType:u8, skillId:u16 LE}`). |
| `0x32` | `0x01` | `0x00788C38` (`cs_32_788c38_788e9f.asm.txt`) | **Không có caller (DEAD CODE)** | — | Hàm mồ côi trong bảng builder, client hoàn toàn không gọi. |
| `0x32` | `0x02` | `0x00788E06` (`cs_32_788c38_788e9f.asm.txt`) | `TFightHum::FUN_0063CF58` @ `0x0063EE0B` (`0063cf58_FUN_0063cf58.asm.txt`) | `TFightHum` / `TSe_FightSkillForm` (`unit+0x5A8`) | Gửi lệnh xuất chiêu / dùng kỹ năng trong trận (`{srcR:u8, srcC:u8, dstR:u8, dstC:u8, skillId:u16 LE}`). |
| `0x17` | `0x0F` | Qua `FUN_0077F414(op=0x17, sub=0x0F)` | `FUN_005A1B10` (sau khi qua gate `FUN_0076D578`) | `TSe_BagForm` / `TSe_EquipForm` | Dùng vật phẩm trong túi (Sách kỹ năng `type=65/66`, Cẩm nang tâm tư `46035/46166`, Túi gấm hoàn kỹ `65025`). |
| `0x0F` | `0x05/06` | Qua `FUN_0077F414(op=0x0F, sub=5/6)` | `FUN_005A1620` @ `0x005A181D` / `0x005A1878` (`005a1620_FUN_005a1620.c`) | `TSe_PetForm` / `TSe_SkillForm` (`14023`) | Cưỡi ngựa (`sub=05`, kiểm tra `14034` nếu là Chiến Tượng) hoặc xuống ngựa (`sub=06`, khi bấm dùng skill `14023`). |

### 3.2 Chiều Server → Client (S→C Dispatcher & Handlers)

Hàm giải mã và phân phối gói S→C chính đọc `Main OP = Payload[0]` (nhảy qua jumptable 65 nhánh `case_000..case_064`) và `Sub OP = Payload[1]` (`RP[1]`).

| Main OP | Sub OP | Case File (Dispatcher) | Hàm Handler (VA & File) | Cấu trúc RAM / UI được cập nhật | Ý nghĩa nghiệp vụ (✅ VERIFIED) |
|:-:|:-:|---|---|---|---|
| `0x05` | `0x03` | `case_006_0078CE88_FUN_0078ce88.c` | `FUN_0072E9A0` (`0072e9a0_FUN_0072e9a0.c`) | `player+0x3EC` (SP), `+0x400` (SkillPts), `+0x4B0` (Reborn), `+0x1271` (Job), `+0xFD4` (3000B skills), `0x94B440` | Nạp toàn bộ thông tin nhân vật + xóa sạch 3000B bảng skill + nạp `N` kỹ năng + bật nút Bày hàng nếu có `14003` + gọi `FUN_005A0388` dựng lại cây kỹ năng. |
| `0x08` | `0x01` (`0x6E`) | `case_009_0078D125_FUN_0078d125.c` | `FUN_00710AB0` @ `0x0071158C` (`00710ab0_FUN_00710ab0.c`) | `player+0xFD4`, `TCY_MoneyMenu+0x13C`, `0x94B440`, `TSe_SkillForm`, `TSe_SkillTreeForm` | Thêm mới, tăng cấp, giảm cấp, hoặc xóa (khi level về 0) 1 kỹ năng của **Nhân vật**; tự động gọi `FUN_005A0388` dựng lại cây. |
| `0x08` | `0x01` (`0x25`) | `case_009_0078D125_FUN_0078d125.c` | `FUN_00710AB0` @ `0x007110E0` (`00710ab0_FUN_00710ab0.c`) | `player+0x400` (`u16`) | Cập nhật **Điểm kỹ năng còn lại** của nhân vật (`deltaType`: 1=Cộng, 2=Trừ, 3=Gán). |
| `0x08` | `0x02` (`sel 04`) | `case_009_0078D125_FUN_0078d125.c:86` | `FUN_00710AB0(player->0x57C[slot], ...)` | `pet+0xFD4` (skills), `pet+0x400` (skill points) | Cập nhật kỹ năng (`prop=0x6E`) hoặc điểm kỹ năng (`prop=0x25`) cho **Thú cưng** tại slot `1..4`. |
| `0x08` | `0x05` | `case_009_0078D125_FUN_0078d125.c:140` | `FUN_0074C340` (`0074c340_FUN_0074c340.c`) | `player+0x12AE[0..3]`, `TShipSkillForm`, `TShipSkillTreeForm` | Cập nhật 4 slot kỹ năng chiến thuyền (Thủy chiến). |
| `0x17` | `0x15..0x17` | `case_020_007902FB_FUN_007902fb.c:97` | `FUN_0059F090` / `FUN_00584AD8` | `TSe_SkillForm`, `TSe_NPCSkillTreeForm` | Làm mới giao diện kỹ năng của Thú cưng sau khi học/thay đổi. |
| `0x17` | `0x4D` | `case_020_007902FB_FUN_007902fb.c:308` | `FUN_00748FAC` | `player+0x1480` (`ball_point`), `player+0x12D4[1..12]` (`ballList`) | Gửi số điểm bi rồng và danh sách các viên bi rồng (`1..12`) đã kích hoạt (Tái Sinh 2). |
| `0x17` | `0x4E` | `case_020_007902FB_FUN_007902fb.c:311` | `FUN_00749FD4` (`00749fd4_FUN_00749fd4.c`) | `DAT_0094B3A4[6..13]`, `TSe_SkillTreeForm` (`FUN_00580DEC`) | Gửi danh sách kỹ năng bóng Tái Sinh 2 (`skill_rb2`, slot `1..8` → ghi vào `DAT_0094B3A4[6..13]`). |
| `0x19` | `0x01/02/03/0A` | `case_022_0079157C_FUN_0079157c.c` | `FUN_0075AF6C` / `0075AFF4` / `0075B3EC` / `0075BF7C` | `TSe_QuickSkillForm` (`gvar_007D9EC0`) | Đồng bộ cấu hình 10 phím tắt kỹ năng F1..F10. |
| `0x19` | `0x0B` | `case_022_0079157C_FUN_0079157c.c` | `FUN_0075B8E0` (`0075b8e0_FUN_0075b8e0.c`) | `TCY_TradeFNpcMenu` (`gvar_007DA438`) | **Đính chính:** Báo lỗi/hủy giao dịch võ tướng (hiện thông báo 3000 ms), **không phải** xóa kỹ năng. |
| `0x20` | `0x01/0x02` | `case_028_0079285A_FUN_0079285a.c` | `FUN_0072C64C` / `FUN_0072F534` | `TCY_ActPose` | **Đính chính:** Gói biểu cảm động tác nhân vật, **không phải** mở khóa Quang/Ám. |
| `0x24` | `0x14` | `case_032_0079346B_FUN_0079346b.c:140` | `FUN_005AC1F0` | `TShipSkillTreeForm` (`gvar_007DA114`) | Mở cửa sổ cây kỹ năng chiến thuyền. |
| `0x2A` | `0x01..0x04` | `case_038_00794719_FUN_00794719.c` | `FUN_005E59C0` / `005E5A38` / `005E799C` / `005E7A14` | `TBK_SkillRegressForm` (`0x2A 01`), `TBK_AttrRegressForm` (`0x2A 02`) | Mở form hoặc trả kết quả tẩy điểm kỹ năng / thuộc tính. |
| `0x45` | `0x0A` | `case_061_00795F82_FUN_00795f82.c:55` | `FUN_005E7A90` (`005e7a14_FUN_005e7a14.c`) | `TLH_ChildSkillDelForm` (`gvar_007D9E3C`) | Mở form xóa kỹ năng Võ tướng đời 2 khi dùng Túi gấm hoàn kỹ (`65025`). |

## 4. Bảng Wire Format Chi Tiết (Byte-Exact Specification)

Mọi khung truyền giữa Client và Server đều được bọc trong header `[0xF4, 0x44][Len: u16 LE][Payload]` và XOR với `0xAD` toàn bộ khung. Dưới đây là định nghĩa byte của phần `Payload` sau khi giải mã XOR `0xAD`.

### 4.1 Chi tiết các gói Client → Server (C→S)

#### (1) `C→S 0x1C 01` — Học / nâng 1 cấp kỹ năng Nhân vật (`✅ VERIFIED`)
* **Hàm tạo:** `FUN_005990E0` (`005990e0_FUN_005990e0.asm.txt` tại `0x005991DF..0x0059924F`) → gọi `FUN_00785DD7`.
* **Kích thước Payload:** 5 byte (gồm 2 byte opcode `1C 01` + 3 byte dữ liệu).

| Offset | Kiểu | Tên trường | Giá trị / Ý nghĩa | Bằng chứng ASM |
|:-:|:-:|---|---|---|
| `+0` | `u8` | `mainOp` | `0x1C` | `0059924b: MOV DL, 0x1c` |
| `+1` | `u8` | `subOp` | `0x01` | `00599249: MOV CL, 0x1` |
| `+2..+3` | `u16 LE` | `skillId` | Mã kỹ năng (`10001..23023`) từ `btn->0xE4` | `005991df: MOV EDX, [EAX+0xE4]` → `CALL 0x0077eb1c` |
| `+4` | `u8` | `targetLevel` | Cấp mục tiêu = `currentLevel (btn->0x108) + 1` | `00599212: MOV EDX, [EAX+0x108]` → `ADD EDX, 1` |

#### (2) `C→S 0x1C 02` — Học / nâng 1 cấp kỹ năng cho Thú cưng (`✅ VERIFIED`)
* **Hàm tạo:** `FUN_005990E0` (`005990e0_FUN_005990e0.asm.txt` tại `0x00599170..0x005991CF`) và `FUN_0058484C` (`0058484c_FUN_0058484c.c`) → gọi `FUN_00785E45`.
* **Kích thước Payload:** 6 byte (2 byte opcode `1C 02` + 4 byte dữ liệu).

| Offset | Kiểu | Tên trường | Giá trị / Ý nghĩa | Bằng chứng ASM |
|:-:|:-:|---|---|---|
| `+0` | `u8` | `mainOp` | `0x1C` | `005991c5: MOV DL, 0x1c` |
| `+1` | `u8` | `subOp` | `0x02` | `005991c3: MOV CL, 0x2` |
| `+2` | `u8` | `petSlot` | Vị trí thú trong `player->0x57C[1..4]` (`1..4`) do `FUN_0071D234` trả về | `0059914c: CALL 0x0071d234` |
| `+3..+4` | `u16 LE` | `skillId` | Mã kỹ năng cần học của thú | `0059917c: CALL 0x0077eb1c` |
| `+5` | `u8` | `targetLevel` | Cấp mục tiêu (`currentLevel + 1`) | `00599195: MOV EDX, [EAX+0x108]` → `ADD EDX, 1` |

#### (3) `C→S 0x1C 04` — Cam kết học kỹ năng trên cây `TSe_SkillTreeForm` / `TWF_TurnTwoHeroSkillForm` (`✅ VERIFIED`)
* **Hàm tạo:** `FUN_0057C840` (`0057c840_FUN_0057c840.c` tại `0x0057CB92`) và `FUN_00586AAC` → gọi `FUN_00785EB3`.
* **Kích thước Payload:** `2 + 3 × N` byte (`N` là số kỹ năng có thay đổi cấp trên cây).

| Offset | Kiểu | Tên trường | Giá trị / Ý nghĩa | Bằng chứng ASM |
|:-:|:-:|---|---|---|
| `+0` | `u8` | `mainOp` | `0x1C` | `00785ef6: MOV DL, 0x1c` |
| `+1` | `u8` | `subOp` | `0x04` | `00785ef4: MOV CL, 0x4` |
| `+2 + 3*i` | `u16 LE` | `skillId[i]` | Mã kỹ năng thứ `i` (`i = 0..N-1`) | `FUN_00583A98` ghi `u16 LE` + `u8` vào `self->0x290` |
| `+4 + 3*i` | `u8` | `targetLevel[i]` | Cấp mục tiêu sau khi cộng trên cây | `node->0xFC` (cấp mới trên cây) |

#### (4) `C→S 0x1C 05` — Cam kết cây kỹ năng Tái Sinh 2 (`setSkillRb2`) (`✅ VERIFIED`)
* **Hàm tạo:** `FUN_0057C840` (`0057c840_FUN_0057c840.asm.txt` tại `0x0057CC30..0x0057CDB6`) kết hợp `FUN_00583A98` (`00583a98_FUN_00583a98.asm.txt`) → gọi `FUN_00785F21`.
* **Kích thước Payload:** `2 + 4 + L1 + 4 + (L2 × 3) + 4 + L3` byte.

| Offset | Kiểu | Tên trường | Giá trị / Ý nghĩa | Đối chiếu C# `TSCharacter.setSkillRb2()` |
|:-:|:-:|---|---|---|
| `+0..+1` | `u8[2]` | `op, sub` | `0x1C, 0x05` | `Packet1CHandler.cs` case `0x05` |
| `+2..+5` | `u32 LE` | `L1` | Số byte (= số viên bi rồng mới gắn trong `self->0x288`) | `int length = packet.readInt32(2);` |
| `+6..+5+L1` | `u8[L1]` | `balls[0..L1-1]` | Chỉ số viên bi rồng (`1..12`) | `ballList[packet[6 + i] - 1] = true;` |
| `+6+L1..+9+L1` | `u32 LE` | `L2` | **Số lượng kỹ năng** trong `self->0x28C` (**KHÔNG phải số byte!** Tổng số byte đoạn 2 là `L2 × 3`) | `int num = packet.readInt32(6 + length);` (vòng lặp `i < num`, bước nhảy `3*i`) |
| `+10+L1..` | `3B × L2` | `skills[0..L2-1]` | Mỗi phần tử 3 byte: `{ skillId: u16 LE, targetLevel: u8 }` | `ushort id = readUInt16(...); byte lvl = ...;` |
| `Next 4B` | `u32 LE` | `L3` | **Tổng số byte** của đoạn 3 (`self->0x290`, bội số của 3; số bóng = `L3 / 3`) | `int num4 = packet.readInt32(...) / 3;` |
| `Next L3 B` | `3B × (L3/3)` | `rb2Slots` | Mỗi phần tử 3 byte: `{ slot: u8 (6..13), skillId: u16 LE }` | `skill_rb2[packet[pos] - 6] = readUInt16(pos + 1);` |

#### (5) Các gói C→S bổ trợ khác (`0x1B 01`, `0x2A 01/02/03/04`, `0x28 01`, `0x29 01`, `0x32 02`)
* **`0x1B 01` (3B data, `✅ VERIFIED`)**: `{ npcIndex: u8 (TInterForm+0x152) }{ npcId: u16 LE (TInterForm+0x154) }`.
* **`0x2A 01` (2B data, `✅ VERIFIED`)**: `{ skillId: u16 LE }` — Tẩy 1 cấp kỹ năng trên `TBK_SkillRegressForm`.
* **`0x2A 04` (3B data, `✅ VERIFIED`)**: `{ childSlot: u8 }{ skillId: u16 LE }` — Xóa 1 kỹ năng của Võ tướng đời 2 trên `TLH_ChildSkillDelForm`.
* **`0x28 01` (4B data, `✅ VERIFIED`)**: `{ hotKeyIdx: u8 (1..10) }{ isPet: u8 (1=Player, 2=Pet) }{ skillId: u16 LE }` — Lưu phím tắt F1..F10.
* **`0x29 01` (4B data, `✅ VERIFIED`)**: `{ slotIdx: u8 (0..5) }{ unitType: u8 }{ skillId: u16 LE }` — Lưu phím tắt chiến đấu.
* **`0x32 02` (6B data, `✅ VERIFIED`)**: `{ srcRow: u8 }{ srcCol: u8 }{ dstRow: u8 }{ dstCol: u8 }{ skillId: u16 LE }` — Xuất chiêu kỹ năng trong trận (`FUN_0063CF58` @ `0x0063EE0B`).

### 4.2 Chi tiết các gói Server → Client (S→C)

#### (1) `S→C 0x05 03` — Khởi tạo Nhân vật & Danh sách kỹ năng (`✅ VERIFIED`)
* **Hàm xử lý:** `FUN_0072E9A0` (`0072e9a0_FUN_0072e9a0.c`, `0072e9a0_FUN_0072e9a0.asm.txt`).
* **Cấu trúc:** Header cố định **113 byte (`0x71` byte, tính cả `05 03`)** + Đuôi **`N × 3` byte** (`N = (PayloadLen - 0x71) / 3`).
* **Các trường liên quan trực tiếp đến Kỹ năng trong Header 113 byte:**
  * `Payload[0x05..0x06]` (`u16 LE`) → ghi vào **`player + 0x3EC` (`Current SP` / Nội lực hiện tại)**.
  * `Payload[0x18..0x19]` (`u16 LE`) → ghi vào **`player + 0x400` (`Skill Points` / Điểm kỹ năng còn lại)**.
  * `Payload[0x60]` (`u8`) → ghi vào **`player + 0x4B0` (`Reborn` / Cấp chuyển sinh: `0`=Chưa CS, `1`=Chuyển sinh 1, `2`=Tái sinh 2)**.
  * `Payload[0x61]` (`u8`) → ghi vào **`player + 0x1271` (`Job` / Nghề chuyển sinh: `1`=Hiệp, `2`=Bá, `3`=Hiền, `4`=Tiên)**.
* **Xử lý phần Đuôi (`N × 3` byte `{ skillId: u16 LE, level: u8 }`):**
  1. Tại `0x0072F319`: Gọi `FUN_007B0094(*(int *)(gvar_007DA108 + 0x13C), 0)` để **tắt nút Bày hàng** trên `TCY_MoneyMenu`.
  2. Tại `0x0072F328`: Gọi `_FillChar(player + 0xFD4, 3000, 0)` để **xóa sạch 1000 ô kỹ năng nhân vật**.
  3. Vòng lặp `idx = 0..N-1`: Ghi `skillId` (`u16 LE`) vào `player + 0xFD4 + idx*3` và `level` (`u8`) vào `player + 0xFD6 + idx*3`. Nếu `skillId == 0x36B3` (`14003` Thuật mua bán) và `level > 0`, bật nút Bày hàng `gvar_007DA108 + 0x13C` và bật cờ nhấp nháy `gvar_007DA530 + 0x1B8 = 1`.
  4. Tại `0x0072F50D` (cuối hàm): Gọi **`FUN_005A0388()`** để dựng lại toàn bộ bảng cây kỹ năng `0x94B440`, `0x94B740`, `0x94B770`.

#### (2) `S→C 0x08 01` prop `0x6E` & `0x25` (Nhân vật) và `S→C 0x08 02` sel `0x04` (Thú cưng) (`✅ VERIFIED`)
* **Hàm xử lý:** `FUN_00710AB0(targetUnit, propCode, deltaType, val1, val2)` (`00710ab0_FUN_00710ab0.c`).
  * Với `08 01`: `targetUnit = *(int *)gvar_007DA054` (Nhân vật chính). Payload: `{ 0x08, 0x01, propCode: u8, deltaType: u8, val1: u32 LE, val2: u32 LE }` (12 byte).
  * Với `08 02`: Nếu `RP[1] == 0x04`, `targetUnit = player->0x57C[RP[2]]` (Thú cưng ở slot `1..4`). Payload: `{ 0x08, 0x02, 0x04, petSlot: u8, propCode: u8, deltaType: u8, val1: u32 LE, val2: u32 LE }` (14 byte).
* **Nhánh `propCode == 0x6E` (`0x0071158C`):**
  * `val1` (`u32 LE`) = `skillId`; `val2` (`u32 LE`) = `deltaLevel`.
  * Nếu `deltaType == 0x01` (Thêm / Tăng cấp): Nếu đã có `skillId` trong `targetUnit + 0xFD4`, cộng thêm `val2` vào level hiện tại; nếu chưa có, tìm ô trống đầu tiên (`skillId == 0`) ghi `{skillId, val2}`.
  * Nếu `deltaType == 0x02` (Giảm cấp / Xóa): Trừ `val2` khỏi level hiện tại; nếu level mới `<= 0`, xóa ô đó (`skillId = 0, level = 0`) và dồn mảng `targetUnit + 0xFD4` lên để không bị lỗ rỗng!
  * Cuối nhánh `0x6E`: Tự động kiểm tra `0x36B3` (`14003`), gọi `FUN_005A0388()` dựng lại cây `0x94B440`, và làm mới `TSe_SkillForm` / `TSe_SkillTreeForm` / `TSe_NPCSkillTreeForm`.

#### (3) `S→C 0x17 0x4D` (Bi rồng) & `S→C 0x17 0x4E` (8 Bóng `skill_rb2`) (`✅ VERIFIED`)
* **`S→C 0x17 0x4D`** (`case_020_007902FB_FUN_007902fb.c:308` → `FUN_00748FAC`):
  * `RP[1]` (`u8`) → ghi vào `player + 0x1480` (`ball_point` — số bi rồng chưa gắn).
  * Các byte tiếp theo `RP[2..len]` (`u8`, giá trị `1..12`) → đánh dấu `*(byte *)(player + 0x12D4 + ballId) = 1`.
* **`S→C 0x17 0x4E`** (`00749fd4_FUN_00749fd4.c`):
  * Kiểm tra `*(byte *)(player + 0x4B0) >= 2` (phải đã Tái Sinh 2).
  * Xóa `DAT_0094B3A4[6..13]` về `0`, sau đó đọc `M = (len - 1) / 3` bộ 3 byte `{ slot: u8 (1..8), skillId: u16 LE }`, ghi `DAT_0094B3A4[slot + 5] = skillId`, rồi gọi `FUN_00580DEC(gvar_007D9E68)` để vẽ lại 8 bóng trên `TSe_SkillTreeForm`.

## 5. Mô Hình Dữ Liệu Kỹ Năng & Bảng Cây Kỹ Năng Runtime

### 5.1 Chứng minh Không gian ID Canonical (`skills.json`) trên 5 nguồn độc lập (`✅ VERIFIED`)

Để kiểm chứng xem client `aLogin.exe` thực sự dùng không gian ID nào (và còn kỹ năng nào bị thiếu hay hard-code riêng không), chúng tôi đã viết script giải mã trực tiếp toàn bộ `Data/Item.Dat` (khoá `(w ^ 0xEFC3) - 9`), `Data/Npc.dat` (khoá `(w ^ 0x5209) - 1`), và `Data/eve.emg`:

| Nguồn dữ liệu | Phương pháp kiểm chứng | Kết quả đối chiếu với `Data/skills.json` (376 bản ghi) | Mức độ |
|---|---|---|---|
| **1. Code ASM `aLogin.exe`** | Quét toàn bộ hằng số skill ID trong `FUN_006538BC`, `FUN_005990E0`, `FUN_005A0388`, `FUN_00597954`, `FUN_00577CCC..0057BD58`, `FUN_0058E0BC`, `FUN_0073D558` | **100% khớp tuyệt đối** (ví dụ: 4 Tâm pháp gốc `10020, 11020, 12020, 13019`; gốc nghề `14038` Đại chu thiên; 16 kỹ năng nghề `14039..14054`; cung tên `14033, 14036, 14037`; cưỡi ngựa `14023, 14034`; mua bán `14003`; triệu gọi `14026, 10016, 11016, 12016, 13015`). | ✅ **VERIFIED** |
| **2. `Data/Item.Dat` (8.372 item)** | Giải mã trường `skill_link` tại offset `+0x67` (`+0x60` RAM) của 135 Sách kỹ năng (`type=65`, ID `65029..65189`), 131 Ngọc tăng uy lực (`type=14`, ID `23030..23276`), 86 Bùa (`type=16`) | **100% khớp tuyệt đối với `skills.json`!** Ví dụ: `65030 Sách lôi mộc → 10002`, `65087 Sách địa động quyết → 10020`, `65108 Sách phong ngạo quyết → 13019`, `65115 Sách Đại chu thiên → 14038`, `65116..65146 → 10027..13033` (31 sách bóng tái sinh), `23257..23272 → 14039..14054` (16 Linh thạch 4 nhánh nghề Bá/Hiệp/Hiền/Tiên), `65155 Sách giải độc → 14014`. | ✅ **VERIFIED** |
| **3. `Data/Npc.dat` (6.659 NPC)** | Giải mã 3 slot kỹ năng NPC tại offset `+0x42, +0x44, +0x46` của toàn bộ 6.659 NPC | **100% khớp với `skills.json`** trên toàn bộ 199 kỹ năng mà NPC sở hữu (ví dụ `NPC#10001 Trương Giác` mang `10006 Địa nha, 10008 Phi sa tẩu thạch, 10013 Vạn mã phi đằng`; `NPC#35029 Thanh Long` mang `13015 Thanh long`). | ✅ **VERIFIED** |
| **4. `Data/eve.emg` (2.777 scene)** | Quét các sự kiện trao kỹ năng (`Action → Skill`) và điều kiện kiểm tra kỹ năng (`cond_class=Unknown`) | **100% khớp với `skills.json`** cho toàn bộ 22 kỹ năng sống được trao qua nhiệm vụ (như `14001` tại map `12136`, `14003` tại `12031`, `14023` tại `12001`, `14026` tại `11569`, `14032` tại `12106`, `14034` tại `12093`). *(Chỉ duy nhất 1 map phụ `14695` có 2 điều kiện cũ `14064/14067` sót lại từ bản legacy).* | ✅ **VERIFIED** |
| **5. `Data/Skill.Dat` (đã dựng lại)** | Kiểm tra `FUN_00759B2C` (`00759b2c_FUN_00759b2c.c`) và `tools/build_skill_dat.py` | `FUN_00759B2C` mở `\Data\Skill.Dat` (86 byte/record, kiểm tra sentinel `*(int *)(base+0x1F) == 2`), giải mã qua `FUN_00759DF0` và điền bảng tra ngược `0x009AE128[skillId] = recordIndex`. Bản `Skill.Dat` 376 record dựng từ `skills.json` đạt độ khớp 100%, **không có bất kỳ ID kỹ năng nào trong `aLogin.exe` nằm ngoài 376 bản ghi này**. | ✅ **VERIFIED** |

### 5.2 Ánh xạ Byte-Exact giữa Record `Skill.Dat` (86B) và `TFSkill` trong RAM (`✅ VERIFIED`)

Trong `FUN_00759B2C` (`00759b2c_FUN_00759b2c.c:88`), toàn bộ file `Skill.Dat` được đọc thẳng vào bộ nhớ bắt đầu từ `skillTableBase + 4` (`local_8 + 4`). Do đó, **mọi offset của một record trong RAM (`skillTableBase + recordIdx * 86 + offset_RAM`) đều bằng `offset_file + 4`**:

| Trường | Offset trong File `.Dat` | Offset từ `skillTableBase` (RAM) | Kiểu | Giải mã (`FUN_00759DF0`) | Ý nghĩa & Hàm truy cập (Accessor) |
|---|:-:|:-:|:---:|---|---|
| `name` | `+0x00..+0x14` | `+0x04..+0x18` | `ShortString[20]` | Đảo ngược chuỗi VISCII | Tên kỹ năng (`FUN_0075AA58`) |
| `type` | `+0x15` (21) | `+0x19` (25) | `u8` | `(b ^ 0xAC) - 1` | Nhóm kỹ năng (`0..19`: `1..4`=Đ/T/H/P, `5`=Tâm, `17`=Bắt buộc, `18`=Quang, `19`=Ám; `FUN_0075AAAC`) |
| `id` | `+0x16..+0x17` (22) | `+0x1A..+0x1B` (26) | `u16 LE` | `(w ^ 0xFD98) - 1` | Mã kỹ năng (`10000..23023`; dùng làm khóa tạo `0x009AE128`) |
| `require_sp` | `+0x18..+0x19` (24) | `+0x1C..+0x1D` (28) | `u16 LE` | `(w ^ 0xFD98) - 1` | **SP tiêu hao khi xuất chiêu** (so sánh với `unit+0x3EC` trong `FUN_0059F090`, `FUN_00598C28`) |
| `elem` | `+0x1A` (26) | `+0x1E` (30) | `u8` | `(b ^ 0xAC) - 1` | Ngũ hành (`0`=Vô/Quang/Ám, `1`=Địa, `2`=Thủy, `3`=Hỏa, `4`=Phong, `5`=Tâm) |
| `atk_type` | `+0x1B` (27) | `+0x1F` (31) | `u8` | `(b ^ 0xAC) - 1` | Kiểu mục tiêu / tầm đánh (ở record 0 sentinel là `int32 = 2`) |
| `nb_target` | `+0x20` (32) | `+0x24` (36) | `u8` | `(b ^ 0xAC) - 1` | Số mục tiêu tối đa |
| `skill_type` | `+0x21` (33) | `+0x25` (37) | `u8` | `(b ^ 0xAC) - 1` | Loại tác dụng (`0`=Passive, `1`=Active, `2`=Buff, `3`=Debuff, `4`=Heal dùng được ngoài trận, `5`=Giải trạng thái) |
| `delay` | `+0x23` (35) | `+0x27` (39) | `u8` | `(b ^ 0xAC) - 1` | Độ trễ / hồi chiêu |
| `unk_gate` | `+0x27` (39) | `+0x2B` (43) | `i8` | `(b ^ 0xAC) - 1` | Nếu bằng `-11` (`0xF5`), `FUN_0059F090` ẩn nút `+` nâng cấp |
| **`sk_point`** | **`+0x28` (40)** | **`+0x2C` (44)** | **`u8`** | `(b ^ 0xAC) - 1` | **"Điểm học được"** — Số điểm kỹ năng tốn khi học cấp `0 → 1` (nhân `×2` nếu khác hệ) |
| **`grade_point`** | **`+0x29` (41)** | **`+0x2D` (45)** | **`u8`** | `(b ^ 0xAC) - 1` | **"Điểm thăng cấp"** — Số điểm kỹ năng tốn cho mỗi cấp từ `1 → 2..max_lvl` (luôn bằng `1`, không nhân đôi) |
| `max_lvl` | `+0x2A` (42) | `+0x2E` (46) | `u8` | `(b ^ 0xAC) - 1` | Cấp tối đa của kỹ năng (`1`, `5`, hoặc `10`) |
| `des` | `+0x2B..+0x52` (43) | `+0x2F..+0x56` (47) | `ShortString[39]` | Đảo ngược chuỗi VISCII (hoặc Big5 với Quang/Ám) | Mô tả kỹ năng (`FUN_0075AB1C`) |
| `require_sk` | `+0x53..+0x54` (83) | `+0x57..+0x58` (87) | `u16 LE` | `(w ^ 0xFD98) - 1` | Kỹ năng tiền đề (cha) trong `Skill.Dat` (`FUN_0075AB3C`) |

### 5.3 Hai Cơ Chế Cây Kỹ Năng Runtime: Bảng Động `0x94B440` vs Cây Tĩnh `TSe_SkillTreeForm` (`✅ VERIFIED`)

Client `aLogin.exe` duy trì **hai cấu trúc dữ liệu cây kỹ năng hoàn toàn tách biệt** cho hai cửa sổ UI khác nhau:

#### A. Bảng động `0x0094B440` (765 byte BSS) phục vụ cửa sổ danh sách kỹ năng `TSe_SkillForm` (`gvar_007D9EB8`)
* **Hàm dựng:** `FUN_005A0388` (`005a0388_FUN_005a0388.c`) gọi `FUN_00597954` (`00597954_FUN_00597954.c`).
* **Cấu trúc bộ nhớ:**
  * `0x0094B440`: `5 tab (Địa=0, Thủy=1, Hỏa=2, Phong=3, Tâm=4) × 51 slot × 3 byte = 765 byte`. Mỗi ô 3 byte gồm `{ skillId: u16 LE, learnedLevel: u8 }`.
  * `0x0094B740`: `4 thú cưng × 4 slot × 3 byte = 48 byte` (3 kỹ năng gốc + 1 `special_skill` của từng thú trong `player->0x57C[1..4]`).
  * `0x0094B770`: `4 slot × 3 byte = 12 byte` (4 kỹ năng của Võ tướng đời 2 / con cái tại `player->0x58C`).
* **Trả lời câu hỏi §1.3 #2 ("Thứ tự skill trong tail `05 03` có ảnh hưởng vị trí ô không?"):**
  * **CÓ đối với `0x94B440`!** Trong `FUN_005A0388`:
    1. Xóa trắng `0x94B440` (`765` byte), `0x94B740` (`48` byte), `0x94B770` (`12` byte).
    2. Đăng ký cố định 4 kỹ năng gốc `10001` (Địa), `11001` (Thủy), `12001` (Hỏa), `13001` (Phong) với `level = 0` vào **`slot 0`** của từng tab `0..3` (`0x005A040F..0x005A0451`).
    3. Duyệt vòng lặp `i = 0..999` trên mảng kỹ năng đã học `player + 0xFD4 + i*3` (**chính là thứ tự các cặp `{skillId, level}` mà Server gửi trong đuôi gói `05 03`**). Với mỗi kỹ năng có `skillId != 0`, gọi `FUN_00597954(skillId, level)`.
    4. Trong `FUN_00597954`: Tra `elem` của `skillId` (`tab = elem - 1`). Nếu `skillId` đã có sẵn trong tab (như 4 gốc `10001..13001`), ghi đè `level` vào byte thứ 3; nếu chưa có, tìm **ô trống đầu tiên (`slot` đầu tiên có `skillId == 0`)** trong tab đó để ghi `{skillId, level}`! Nếu kỹ năng có `require_sk != 0` (hoặc thuộc 5 nhánh cha-con hard-code: `10024→10023`, `12024→12022/12025`, `12026→12025`, `13021→13020`, `13024→13023`) mà kỹ năng cha chưa có trong tab, `FUN_00597954` tự động chèn kỹ năng cha với `level = 0` vào ô trống trước rồi mới chèn kỹ năng con.
  * **Kết luận cho Server:** Để danh sách kỹ năng trong `TSe_SkillForm` hiển thị gọn và nhất quán, Server nên sắp xếp danh sách kỹ năng trong đuôi `05 03` tăng dần theo `skillId`.

#### B. Cây kỹ năng tĩnh trên `TSe_SkillTreeForm` (`gvar_007D9E68`)
* **Hàm dựng:** `FUN_00577CCC`, `FUN_00579640` (Tab 5 Tâm), `FUN_00579E80` (Tab 6 Bá), `FUN_0057A858` (Tab 7 Hiệp), `FUN_0057B2D8` (Tab 8 Hiền), `FUN_0057BD58` (Tab 9 Tiên), và cập nhật trạng thái qua `FUN_0057E55C` / `FUN_00580DEC`.
* **Cấu trúc:**
  * `DAT_0094B330` (`4 hệ × 14 u16 = 112 byte`): Lưu các kỹ năng nhánh nâng cao/chuyển sinh của 4 tab hệ `0..3`.
  * `DAT_0094B3A4` (`14 u16 = 28 byte`): Lưu 4 Tâm pháp gốc (`idx 1..4` = `10020, 11020, 12020, 13019`), gốc nghề (`idx 5` = `14038` Đại chu thiên), và **8 slot kỹ năng bóng Tái Sinh 2 (`idx 6..13` = `skill_rb2[0..7]`)**.
  * `self->0x24C[0..3]`: 4 cây nhánh nghề Bá (`0`), Hiệp (`1`), Hiền (`2`), Tiên (`3`), mỗi cây có đúng 5 node cố định (node 0 = `14038`, node 1..4 = 4 kỹ năng nghề).
  * `self->0x24C[4]`: Cây kỹ năng sống hệ Tâm (Tab 5), gồm 37 node cố định (`14001..14037`).
* Vị trí tọa độ `(X, Y)` và chỉ số node trên `TSe_SkillTreeForm` là **cố định tuyệt đối theo code dựng form**, hoàn toàn không bị ảnh hưởng bởi thứ tự gói `05 03`.

### 5.4 Giải nghĩa 3 địa chỉ BSS `0x948DF8`, `0x9490B0`, `0x98BE04` (§1.3 #4 — `✅ VERIFIED`)

Đối chiếu PE Section Header (`BSS VA 0x007DB000..0x00A118CD`) và báo cáo cross-reference `round3_report_round7.txt:2088-2115`:
1. **`0x00948DF8`** (được trỏ bởi `gvar_007DA308`): Chỉ được truy cập tại `0x0054A384..0x0054BE9A` thuộc lớp **`TMR_WaterBattleInFor`** (`0051189c_FUN_0051189c.c`) — bảng thông tin sự kiện Thủy chiến, **không liên quan đến kỹ năng**.
2. **`0x009490B0`**: Chỉ được truy cập duy nhất tại `FUN_0054C29C` (`0x0054C2CB`, thuộc cụm Thủy chiến / Mini-game), **không liên quan đến kỹ năng**.
3. **`0x0098BE04`** (được trỏ bởi `gvar_007DA638`): Được khởi tạo và tra cứu tại `FUN_00634F3C` (`0x00635023`, bảng giới hạn bản đồ/cảnh) và `0x006355C8..0x00637D69` (`TSe_EquipForm2`), **không liên quan đến cây kỹ năng**.

## 6. Phân Tích Chuyên Sâu 6 Nhóm Nghiệp Vụ (Nhóm A → Nhóm F)

### 6.1 NHÓM A ⭐ — Toàn Bộ Kỹ Năng Sống Hệ Tâm (`elem = 5`, `type = 5`)

Trong `Data/Skill.Dat` và `Data/skills.json`, hệ Tâm (`type = 5`) có tổng cộng **62 kỹ năng** (`14001..14061` và `20006` Hộ chủ), cộng thêm **4 kỹ năng Boss/NPC mang `elem = 5`** (`20033, 20047, 20051, 20052`). Toàn bộ 66 kỹ năng này (cùng dòng xác minh "Dung thọ") đã được xuất chi tiết trong file `client_skills_handler/data/life_skill_usage.tsv`. Dưới đây là giải đáp chuyên sâu cho 7 câu hỏi của Nhóm A:

#### (1) Đào khoáng (`14010` Thuật đào khoáng, `14018` Khai khoáng, `14032` Thâm khoáng / Đào sâu) (`✅ VERIFIED`)
* **Trên Client:** Cả 3 kỹ năng `14010` (`require_sk=14004`), `14018` (`require_sk=14010`), `14032` (`require_sk=14018`) đều là kỹ năng **Bị động (`skill_type = 0`, `max_lvl = 10`, `sk_point = 1`)** hiển thị trên Tab 5 (`self->0x24C[4]`) của `TSe_SkillTreeForm`. Client **không có cửa sổ UI đào khoáng riêng** và chặn click dùng trực tiếp (`FUN_00598C28` báo *"Không thể dùng trong trạng thái thường"*, `FUN_006538BC` chặn dùng trong trận tại `0x00653A2B`).
* **Cơ chế hoạt động & Vật phẩm liên quan (`Item.Dat` & `Eve.Emg`):**
  * Để đào khoáng, người chơi trang bị công cụ **Cuốc** (`type = 39` trong `Item.Dat`: `10001 Cuốc`, `10497 Hoàng Cân Sừ Đầu`, `10530 Cuốc chân kim`, `10531 Cuốc tân truyền`, `10532 Cuốc hòa điền`, `10533 Cuốc đá vũ hoa`, `10534 Cuốc nham thạch đỏ`) và đi lại tại các bản đồ mỏ khoáng. Server dựa vào cấp độ của `14010` / `14018` / `14032` để quyết định tỷ lệ rơi khoáng thạch cùng cấp trong trận đánh mỏ hoặc sự kiện.
  * Trong `Eve.Emg` (`eve_logic.tsv`): Kỹ năng đào khoáng được trao và kiểm tra trực tiếp tại:
    * `map 12404` (**Đỉnh Từ Vô Sơn**, `eve=2`): Kiểm tra có `14001` và vật phẩm `18001 Nhập Môn Lục` → trao kỹ năng `14005` / nhánh khoáng.
    * `map 14425` (**Đỉnh Chung Nam Sơn**, `eve=4`): Kiểm tra đã học `14004` (Thuật dò xét) → trao **`14010` (Thuật đào khoáng)**.
    * `map 12106` (**Đồng Tước đài**, `eve=2`): Kiểm tra `14005 == 10`, `14011 == 10`, `14019 == 5` → trao **`14032` (Đào sâu / Thâm khoáng)**.

#### (2) Tinh luyện / Luyện đơn / Đúc luyện / Chế tạo (`14005`, `14011`, `14019`, `14035`) (`✅ VERIFIED`)
* **Chuỗi 4 kỹ năng trên cây Tab 5 (`FUN_00579640`):**
  1. `14005` (**Thuật luyện đơn**, `require_sk=14001`, `max_lvl=10`, Passive) — Hợp thành sơ cấp.
  2. `14011` (**Thuật đúc luyện**, `require_sk=14005`, `max_lvl=10`, Passive) — Hợp thành trung cấp (học tại `map 11424` Lang Gia cung, `eve=2`).
  3. `14019` (**Thuật tinh luyện**, `require_sk=14011`, `max_lvl=10`, Passive) — Hợp thành cao cấp (học tại `map 18102` Kiến Nghiệp nhai đạo, `eve=4`).
  4. `14035` (**Đúc thành / Luyện Tạo Thuật**, `require_sk=14019`, `max_lvl=10`, Passive) — Hợp thành tổng hợp cấp 4 (kiểm tra tại `map 59301` Thông đạo Thiên Đình, `eve=4`).
* **UI và Opcode phía Client:**
  * Client **không đọc** file `Data/Compound.Dat` hay `Data/BlissBag.Dat` (hai file này là bảng công thức phía **Server**; quét toàn bộ chuỗi trong `aLogin.exe` xác nhận không có chuỗi `Compound.Dat` hay `BlissBag.Dat`).
  * Khi người chơi mở giao diện **Hợp thành (`TCY_MixMenu`, `gvar_007DA374`)**, đặt nguyên liệu vào các ô và bấm nút Hợp thành, hàm `FUN_005C0288` gửi gói **`C→S 0x1A 04`** chứa vị trí các ô nguyên liệu lên Server. Server tra `Compound.Dat` và cấp độ `14005/14011/14019/14035` của người chơi để tính ra vật phẩm hợp thành và trả kết quả về qua `S→C 0x1A`.

#### (3) Cưỡi ngựa (`14023` Thuật kỵ mã / Cưỡi ngựa & `14034` Kỵ Binh) và lớp `THorse` (`✅ VERIFIED`)
* **Kiểm tra file `Mounts.dat` / `MountsGrow.dat`:** Client `aLogin.exe` **không đọc** `Mounts.dat` hay `MountsGrow.dat` (0 kết quả trong `viscii_strings.tsv`). Toàn bộ dữ liệu ngựa được client đọc từ **`Data/Npc.dat`**!
* **Lớp `THorse` (`00720984_THorse.Create.c`, RTTI tại `0x0070B82C`):** Được khởi tạo tại `FUN_005E8534` và `FUN_0071DAE0` để quản lý animation/sprite thú cưỡi gắn vào nhân vật.
* **Sự khác biệt giữa `14023` (`0x36C7` Cưỡi ngựa) và `14034` (`0x36D2` Kỵ Binh):**
  * Trong `Data/Npc.dat`, trường byte tại offset `+0x0F` (`b15`, giải mã `(b ^ 0xC8) - 1`) phân loại thú cưỡi thành 2 nhóm:
    1. **`b15 == 9` (Ngựa/thú cưỡi thường — 72 loài, ID `18001..18072`)**: Gồm `Tuấn Mã (18001)`, `Ô Duy Mã (18002)`, `Đích Lô mã (18010)`, `Hạn Huyết Bửu Mã (18013)`, `Côn Lôn Thần Câu (18016..18020)`, `Lạc đà (18024)`, `Sói (18026)`, `Gấu (18028)`, `Báo (18030)`, `Hổ (18032)`, `Sư tử (18034)`, `Cân Đẩu Vân (18064)`, `Xích Thố Thiết Kỵ (18067)`... Tại `FUN_0073D558` (`0073d558_FUN_0073d558.c:32-44`), khi `param_2 == 9`, client kiểm tra nhân vật có kỹ năng **`0x36C7` (`14023` Cưỡi ngựa)** không. Ngoài ra, khi người chơi bấm dùng kỹ năng `14023` trong `TSe_SkillForm` (`FUN_00598C28` @ `0x00598D57`) hoặc trên thanh phím tắt F1..F10 (`FUN_005A94C0` / `FUN_005AA09C`), client gọi `FUN_005A1620(0)` để gửi **`C→S 0x0F 06` (Xuống ngựa)**!
    2. **`b15 == 22` (`0x16` — Chiến mã / Chiến Tượng — 6 loài, ID `31001..31006`)**: Gồm 6 loại `Chiến Tượng` (`31001..31006`). Tại **3 hàm độc lập** (`FUN_0073D558` @ `0x0073D5E8`, `FUN_005A1620` @ `0x005A177C` ngoài trận, và `TFightHum::FUN_0063CF58` @ `0x0063EC5A` trong trận), khi `Npc.dat b15 == 0x16`, client gọi `FUN_0073393C(player, 0x36D2)` kiểm tra nhân vật đã học **`0x36D2` (`14034` Kỵ Binh)** chưa (trừ khi đang đứng ở map đặc biệt `12001..12003` hoặc `12825`). Nếu chưa học `14034`, client chặn cưỡi và hiện thông báo `@0x5A191C`: ***"Phải học thuật kỵ binh"***.

#### (4) Dung thọ (Nấu ăn) — Xác minh không tồn tại & Hệ thống thay thế (`✅ VERIFIED`)
* Quét toàn bộ 376 bản ghi `skills.json`, 352 bản ghi `Skill_legacy_352.Dat`, và 22.138 chuỗi VISCII trong `aLogin.exe` xác nhận **không có kỹ năng nào tên là "Dung thọ" hay "Nấu ăn"**.
* Trong TS Online, việc chế biến thức ăn hồi HP/SP (như `Bánh bao hấp`, `Gà nướng trui`, `Đĩnh Tống`...) được tích hợp vào:
  1. **Hệ thống Hợp thành (`TCY_MixMenu`, `gvar_007DA374`, opcode `C→S 0x1A 04`)** dùng kỹ năng `14005` (Thuật luyện đơn) và `14011` (Thuật đúc luyện).
  2. **Lò nấu / Chế tạo sự kiện (`TSe_InteractiveForm`, opcode `C→S 0x33 01`)** dùng trong các dịp lễ hội (như làm bánh Đoan Ngọ, Nguyên Tiêu).

#### (5) Mua bán / Giao dịch / Tịnh tọa / Khai thương / Thiền định (`✅ VERIFIED`)
* **`14003` (`0x36B3` Thuật mua bán)**:
  * Khi nhận `05 03` (`FUN_0072E9A0` @ `0x0072F428`) hoặc `08 01` prop `0x6E` (`FUN_00710AB0` @ `0x007117A8`), nếu nhân vật có `14003` với `level > 0`, client gọi `FUN_007B0094(*(int *)(gvar_007DA108 + 0x13C), 1)` để **mở khóa nút "Bày hàng"** trong menu tiền `TCY_MoneyMenu` (`gvar_007DA108`), đồng thời đặt `*(byte *)(gvar_007DA530 + 0x1B8) = 1` để nhấp nháy biểu tượng menu tiền trên `TSe_MainStatus`.
  * Khi người chơi bấm trực tiếp vào kỹ năng `14003` trên `TSe_SkillForm` (`FUN_00598C28` @ `0x00598D3F`) hoặc trên thanh phím tắt F1..F10 (`FUN_005A94C0` / `FUN_005AA09C`), client gọi thẳng **`FUN_005C182C()`** để mở giao diện Bày hàng cá nhân (Player Stall / Vendor)!
  * Ngoài ra, cấp độ `14003` (`1..8`) còn quyết định mức giảm giá khi mua/tăng giá khi bán tại shop NPC (xử lý ở module Economy).
* **`14001` (Thuật trinh thám / Tịnh tọa gốc)**, **`14016` (Tu dưỡng)**, **`14017` (Khai thương)**, **`14020` (Thiền Định)**: Đều là các kỹ năng bị động (`skill_type = 0`) trên Tab 5 (`self->0x24C[4]`), bị chặn dùng trong trận tại `FUN_006538BC` (`0x00653A2B`). Server đọc cấp độ của `14016` / `14020` để tăng tốc độ hồi HP/SP khi ngồi nghỉ, và đọc cấp độ của `14017` (Khai thương, `max_lvl = 5`) để mở rộng số ô chứa đồ/thú tại Tiền trang / Khách sạn.

### 6.2 NHÓM B ⭐ — Kỹ Năng Nghề Sau Chuyển Sinh / Tái Sinh (Bá / Hiệp / Hiền / Tiên) & Bóng `skill_rb2`

#### (1) Thứ tự 4 nhánh nghề và Ánh xạ `job` Server ↔ Tab Client (`✅ VERIFIED`)
* **Gốc chung:** Kỹ năng **`14038` ("Đại chu thiên")** (`elem = 5`, `type = 5`, `sk_point = 1`, `max_lvl = 5`, `skill_type = 0` Passive). Được lưu tại `DAT_0094B3A4[5]` và là **node 0** của cả 4 cây nhánh nghề `self->0x24C[0..3]`.
* **16 kỹ năng thuộc 4 nhánh nghề:** Được dựng bởi 4 hàm song sinh `FUN_00579E80`, `FUN_0057A858`, `FUN_0057B2D8`, `FUN_0057BD58` (gọi liên tiếp tại `0x00579094..0x005790AC` trong constructor `FUN_00577CCC`). Kết hợp với kiểm tra `player + 0x1271` (`job`) tại `FUN_0057D0C8` (`0057d0c8_FUN_0057d0c8.c:518-534`) và danh sách Linh thạch nghề trong `Item.Dat` (`23257..23272`), bảng ánh xạ chính xác tuyệt đối như sau:

| Thứ tự Tab trên `TSe_SkillTreeForm` | Tên file ảnh nút (`FUN_005AC75C`) | Chỉ số mảng `self->0x24C[idx]` | Hàm dựng cây (Builder VA) | Nhánh nghề | Mã `job` trên Server (`player+0x1271`, `05 03` `+0x61`) | 4 Skill ID (Canonical & Wire) | Linh thạch tăng hiệu quả trong `Item.Dat` (`type=14`) |
|:-:|:-:|:-:|:-:|---|:-:|---|---|
| **Tab 6** | `"6btn_02"` | `0x24C[0]` | `FUN_00579E80` | **BÁ** | **`2`** | `14039` (Bá khí - Passive)<br>`14040` (Bá ý - Active)<br>`14041` (Lực bạt sơn hề - Passive)<br>`14042` (Nộ đào chiến đạp - Active) | `23257..23260` |
| **Tab 7** | `"7btn_02"` | `0x24C[1]` | `FUN_0057A858` | **HIỆP** | **`1`** | `14043` (Anh khí hộ thân - Passive)<br>`14044` (Thiên Thần thuẫn - Active)<br>`14045` (Tâm nhãn - Passive)<br>`14046` (Ngưng Khí Hộ Thân - Active) | `23261..23264` |
| **Tab 8** | `"8btn_02"` | `0x24C[2]` | `FUN_0057B2D8` | **HIỀN** | **`3`** | `14047` (Dụng kế - Passive)<br>`14048` (Trí thủ - Passive)<br>`14049` (Ngự địch - Passive)<br>`14050` (Sách động - Active) | `23265..23268` |
| **Tab 9** | `"9btn_02"` | `0x24C[3]` | `FUN_0057BD58` | **TIÊN** | **`4`** | `14051` (Tiên khiếu - Passive)<br>`14052` (Hóa Huyết Hoàn Hồn - Passive)<br>`14053` (Thánh Linh thuật - Active)<br>`14054` (Càn Khôn Thiên Lôi - Active) | `23269..23272` |

* **Client có tự quyết định nhánh nghề không hay Server báo? (`✅ VERIFIED`)**:
  * **Server quyết định hoàn toàn!** Server gửi mã `job` (`1..4`) trong gói `S→C 05 03` tại `Payload[0x61]` (lưu vào `*(byte *)(player + 0x1271)`).
  * Khi mở `TSe_SkillTreeForm` (`FUN_0057E55C` tại `0057e55c_FUN_0057e55c.c:216-247`) hoặc bấm vào một node trên Tab 6..9 (`FUN_0057D0C8` tại `0057d0c8_FUN_0057d0c8.c:518-534`), client kiểm tra `player->0x1271`: nếu người chơi bấm sang tab của nghề khác với `player->0x1271`, client chặn ngay và hiện thông báo `@0x57DFB8` (*"Không thể học kỹ năng của nghề khác"*).

#### (2) "Bóng" (`skill_rb2`) & Bi rồng (`ballList`) trong Tái Sinh 2 (`✅ VERIFIED`)
* **12 Bi rồng (`ballList`):** Lưu tại `player + 0x12D5 .. +0x12E0` (`12 byte`, chỉ số `1..12`), số bi rồng chưa gắn lưu tại `player + 0x1480` (`u8`). Được Server đồng bộ xuống qua gói **`S→C 0x17 0x4D`** (`FUN_00748FAC`). Khi người chơi gắn bi rồng trên `TSe_SkillTreeForm`, chỉ số viên bi (`1..12`) được đẩy vào chuỗi `self->0x288` để gửi lên trong đoạn `L1` của gói **`C→S 0x1C 05`**.
* **8 Slot Bóng Tái Sinh (`skill_rb2`) và 32 Kỹ năng Anh hùng (`10027..13033`):**
  * 8 slot bóng được lưu tại **`DAT_0094B3A4[6..13]`** (2 slot cho mỗi hệ Địa/Thủy/Hỏa/Phong). Server đồng bộ 8 slot này xuống client qua gói **`S→C 0x17 0x4E`** (`FUN_00749FD4`).
  * Có **32 kỹ năng bóng tái sinh** (8 kỹ năng mỗi hệ: Địa `10027..10034`, Thủy `11027..11034`, Hỏa `12027..12034`, Phong `13026..13033`).
  * **UI chọn/nâng cấp kỹ năng bóng:** Ngoài việc gắn bóng trên `TSe_SkillTreeForm` (`FUN_0057D0C8` ghi `{slot 6..13, skillId}` vào `self->0x290` để gửi trong đoạn `L3` của `0x1C 05`), client còn có form chuyên biệt **`TWF_TurnTwoHeroSkillForm`** (`gvar_007DA2C4`, resource `"form_HeroSkill"`, khởi tạo tại `FUN_00586370`, vẽ tại `FUN_005876F0`) hiển thị lưới 32 kỹ năng anh hùng này và gửi gói `0x1C 04` qua `FUN_00586AAC`.

#### (3) Hệ thống Tẩy điểm / Respec (`0x2A`) & Xóa kỹ năng con cái (`0x45 0A`) (`✅ VERIFIED`)
* **Tẩy điểm kỹ năng nhân vật (`TBK_SkillRegressForm`, `gvar_007DA1D8`):**
  * Kích hoạt khi dùng vật phẩm **Cẩm nang tâm tư** (`Item#46035` hoặc `Item#46166`, `type=38`) → Server gửi **`S→C 0x2A 01`** (`case_038_00794719` → `FUN_005E59C0`) mở `TBK_SkillRegressForm`.
  * Khi người chơi chọn kỹ năng cần hạ cấp và bấm OK, `FUN_005E4C14` (`005e4c14_FUN_005e4c14.c`) kiểm tra **3 điều kiện chặn (GATE-11)**:
    1. Nếu chọn 1 trong 4 kỹ năng gốc (`10001, 11001, 12001, 13001`), mọi kỹ năng khác trong cùng tab hệ phải đã về `level == 0` (nếu còn, báo `@0x5E55C8`).
    2. Nếu kỹ năng đang ở `level == 1` (tức giảm về `0`), mọi kỹ năng con trực tiếp (`child node` trong cây `TSe_SkillTreeForm`) phải có `level == 0` (nếu còn, báo `@0x5E55F8`).
    3. Nếu chọn `14038` (Đại chu thiên) khi `level == 1`, cả 4 kỹ năng nhánh nghề phải có `level == 0` (nếu còn, báo `@0x5E5630`).
  * Nếu hợp lệ, `FUN_005E4C14` gọi `FUN_0078848A` gửi **`C→S 0x2A 01 { skillId: u16 LE }`**. Server trừ 1 cấp kỹ năng (gửi `S→C 08 01` prop `0x6E` `deltaType=2`), cộng trả điểm kỹ năng (`08 01` prop `0x25` `deltaType=1`), và gửi phản hồi `S→C 0x2A 01`.
* **Tẩy điểm thuộc tính (`TBK_AttrRegressForm`, `gvar_007DA0F8`):** Mở bởi `S→C 0x2A 02` khi dùng **Túi gấm cẩn thận** (`Item#46203`); gửi **`C→S 0x2A 02 { attrCode: u8 }`**.
* **Xóa kỹ năng Võ tướng đời 2 (`TLH_ChildSkillDelForm`, `gvar_007D9E3C`):** Mở bởi **`S→C 0x45 0A`** (`case_061_00795F82` → `FUN_005E7A90`) khi dùng **Túi gấm hoàn kỹ** (`Item#65025`); khi bấm OK (`FUN_005E7F90`), client gọi `FUN_007885D4` gửi **`C→S 0x2A 04 { childBagSlot: u8 }{ skillId: u16 LE }`**.

### 6.3 NHÓM C ⭐ — Kích Hoạt & Học Kỹ Năng Quang / Ám (Light / Dark) & Kỹ Năng `type = 17`

#### (1) Bản chất kỹ năng Quang (`type = 18`) và Ám (`type = 19`) (`✅ VERIFIED`)
* Trong `Data/Skill.Dat` và `Data/skills.json`:
  * **Quang (`type = 18`)**: Gồm **22 kỹ năng** (`22001..22022`), đều có **`elem = 0`**.
  * **Ám (`type = 19`)**: Gồm **23 kỹ năng** (`23001..23023`), đều có **`elem = 0`**.
* **Phát hiện thú vị về chuỗi mô tả (`des`) của 45 kỹ năng Quang/Ám (`✅ VERIFIED`):** Trong `Skill.Dat` / `skills.json`, tên (`name`) của 45 kỹ năng này đã được dịch sang tiếng Việt VISCII (`Quang mâu thuật`, `Phật quang chưởng`, `Bạch Hổ`, `Huyền Vũ`...), nhưng trường mô tả (`des`) vẫn giữ nguyên **byte tiếng Trung Phồn thể Big5 (`cp950`)** chưa dịch! Khi bộ giải mã cũ đọc bằng bảng VISCII đã tạo ra chuỗi rác (`ầHầúấậ...`). Khi giải mã đúng bằng `big5`, toàn bộ 45 mô tả hiện ra rõ ràng (ví dụ: `22001` = `以光之矛傷害3位敵人` *"Dùng giáo ánh sáng sát thương 3 kẻ địch"*, `22011` = `召喚白虎攻擊` *"Triệu hồi Bạch Hổ tấn công"*, `23011` = `召喚玄武攻擊` *"Triệu hồi Huyền Vũ tấn công"* — đã lưu đầy đủ trong `client_skills_handler/data/skill_id_mapping.tsv`).

#### (2) Đính chính tài liệu cũ: Kích hoạt Quang/Ám diễn ra thế nào? (`✅ VERIFIED`)
* Tài liệu cũ (`reference/analysis_docs/light_dark_skill_unlock_gap.md`) từng suy đoán rằng `0x36B3` (`14003`) là cờ mở khóa Quang/Ám và `S→C 20 01/02` là gói kích hoạt Quang/Ám. **Cả hai giả định đó đều SAI:**
  1. `0x36B3` (`14003`) là **Thuật mua bán**, bật nút Bày hàng tại `TCY_MoneyMenu + 0x13C`.
  2. `S→C 20 01/02` là gói biểu cảm động tác nhân vật (**`TCY_ActPose`**).
* **Cơ chế thật sự trong Client (`✅ VERIFIED`):**
  * Kỹ năng Quang/Ám có `elem = 0`. Khi kiểm tra khắc hệ tại `FUN_00598944` (`00598944_FUN_00598944.c`), hàm chỉ chặn các cặp `(1↔3)` và `(2↔4)`, nên **`elem = 0` hoàn toàn KHÔNG bị gate theo hệ Địa/Thủy/Hỏa/Phong** (xác nhận thêm tại `FUN_0076D578` khi đọc sách kỹ năng).
  * Tuy nhiên, trong `FUN_00597954` (`00597954_FUN_00597954.c`), bảng cây `0x94B440` của `TSe_SkillForm` chỉ nhận các kỹ năng có `tab = elem - 1` thuộc `0..4` (`elem = 1..5`). Vì Quang/Ám có `elem = 0` (`tab = -1`), chúng không nằm trên 5 tab Ngũ hành/Tâm của `TSe_SkillForm`, nhưng vẫn nằm trong mảng kỹ năng nhân vật **`player + 0xFD4`** và hiển thị đầy đủ trong **menu kỹ năng chiến đấu (`TFightHum` / `TSe_FightSkillForm`)**!
  * Do đó, **Client không gửi gói kích hoạt riêng nào cho Quang/Ám**. Phía Server chỉ cần gửi `{skillId, level}` (ví dụ `22001..22022` hoặc `23001..23023`) trong đuôi gói `S→C 05 03` hoặc qua `S→C 08 01` prop `0x6E` (`deltaType=1`) là nhân vật sử dụng được ngay trong chiến đấu!

#### (3) Danh sách 5 kỹ năng `type = 17` ("Bắt buộc phải học / Bị động trang bị") và Gate trong `FUN_006538BC` (`✅ VERIFIED`)
Trong `FUN_006538BC` (`006538bc_FUN_006538bc.asm.txt` tại `0x00653978..0x00653992`), client gọi `FUN_0075AAAC(skillId)` để lấy `TFSkill.type (+0x19 RAM)`. Nếu **`type == 0x11` (`17`)**, hàm hiện thông báo `@0x00653E28` (**`"Chiến dịch lại đến"`** / kỹ năng tự động kích hoạt khi vào trận) và trả về `0` (không cho chọn làm lệnh đánh chủ động). Danh sách 5 kỹ năng `type = 17` trong `Skill.Dat` / `skills.json` gồm:

| Skill ID (Canonical) | Skill ID (Legacy 352) | Tên kỹ năng | Hệ (`elem`) | `sp_cost` | `max_lvl` | Mô tả trong `Skill.Dat` |
|:-:|:-:|---|:-:|:-:|:-:|---|
| **`10037`** (`0x2735`) | `9989` (`0x2705`) | **Cự Thú** (`Chấn Ba`) | `1` (Địa) | `41` | `10` | Dùng lời ếm dụ địch thủ |
| **`11037`** (`0x2B1D`) | `11053` (`0x2B2D`) | **Băng Vũ** (`Thiên Băng Vũ`) | `2` (Thủy) | `44` | `10` | Băng đao hồi thiên bay lượn |
| **`12037`** (`0x2F05`) | `11989` (`0x2ED5`) | **Hỏa Quyền** | `3` (Hỏa) | `38` | `10` | Giáng cú đấm lửa xuống |
| **`13036`** (`0x32EC`) | `13052` (`0x32FC`) | **Băng Lôi** | `4` (Phong) | `42` | `10` | Sấm sét liên tục giáng xuống |
| **`14056`** (`0x36E8`) | `14104` (`0x3718`) | **Tiên khí hộ thân** | `5` (Tâm) | `0` | `1` | Tiên khí tràn ngập, miễn trừ các hiệu quả bất lợi (gắn với `Item#25086 Thẻ tiên khí hộ thân`) |

### 6.4 NHÓM D ⭐ — Kỹ Năng Triệu Gọi (Summon) Theo 4 Hệ Địa / Thủy / Hỏa / Phong & Quang / Ám

#### (1) Danh sách 16 kỹ năng triệu gọi 4 hệ và Cơ chế Node Placeholder `0x36CA` (`14026`) (`✅ VERIFIED`)
* Trong `Skill.Dat` / `skills.json`, ngoài kỹ năng gốc **`14026` (`0x36CA` "Triệu gọi" / "Triệu Linh Thuật")** (`elem=5`, `max_lvl=1`, `sk_point=0`), mỗi hệ Địa/Thủy/Hỏa/Phong có **4 ID kỹ năng triệu gọi** (1 ID chính có thể học/dùng + 3 ID hiệu ứng theo bậc Tiểu/Trung/Đại dùng trong trận):

| Hệ (`elem`) | Skill ID chính (trong `gvar_007D78C0`) | Tên kỹ năng | 3 Skill ID bậc hiệu ứng (Tiểu / Trung / Đại) | Sách học kỹ năng (`Item.Dat type=65`) | Ngọc / Văn Chương cần đeo (`Item.Dat type=14`) |
|:-:|:-:|---|---|---|---|
| **Địa (`1`)** | **`10016`** (`0x2720`) | **Nham quái** | `10017` (Tiểu), `10018` (Trung), `10019` (Đại) | `65147` (*Sách Triệu Gọi N.Quái*) | `23086` (*Ngọc Nham Quái*)<br>`23135` (*Nham Quái Văn*) |
| **Thủy (`2`)** | **`11016`** (`0x2B08`) | **Thủy thần** | `11017` (Tiểu), `11018` (Trung), `11019` (Đại) | `65148` (*Sách Triệu Gọi T.Thần*) | `23087` (*Ngọc Thủy Thần*)<br>`23136` (*Thủy Thần Văn*) |
| **Hỏa (`3`)** | **`12016`** (`0x2EF0`) | **Phụng hoàng** | `12017` (Tiểu), `12018` (Trung), `12019` (Đại) | `65149` (*Sách Triệu Gọi P.Hoàng*) | `23088` (*Ngọc Phụng Hoàng*)<br>`23137` (*Phụng Hoàn Văn*) |
| **Phong (`4`)** | **`13015`** (`0x32D7`) | **Thanh long** | `13016` (Tiểu), `13017` (Trung), `13018` (Đại) | `65150` (*Sách Triệu Gọi T.Long*) | `23089` (*Ngọc Thanh Long*)<br>`23138` (*Thanh Long Văn*) |

* **Cơ chế thay thế Node Placeholder `0x36CA` (`14026`) trên `TSe_SkillTreeForm` (`FUN_0057E55C` @ `0x0057EC94`, file `0057e55c_FUN_0057e55c.c:395-458`):**
  * Trên mỗi tab hệ `0..3` của `TSe_SkillTreeForm`, node triệu gọi ban đầu được khởi tạo với `node->0x10C = 0x36CA` (`14026` Triệu gọi).
  * Khi cập nhật cây kỹ năng, `FUN_0057E55C` kiểm tra bảng `gvar_007D78C0[tab][0..2]`: nếu người chơi đã học kỹ năng triệu gọi của hệ đó (`param_2` ∈ `{10016..10018, 11016..11018, 12016..12018, 13015..13017}`), hàm **ghi đè `node->0x10C = param_2`**, nạp lại ảnh icon `icon_sk<param_2>` và cập nhật cấp độ của node đó!
* **Cơ chế khóa Triệu gọi trên Thú cưng khi chưa đeo Ngọc/Văn Chương (`FUN_0058E0BC`, `FUN_005A0388`, `FUN_00584AD8`):**
  * Trong `FUN_005A0388` (`005a0388_FUN_005a0388.c:189-201, 295-307`) và `FUN_00584AD8` (`00584ad8_FUN_00584ad8.c:233-240`): Khi nạp kỹ năng thứ 4 (`TSpecialSkill`) của thú cưng, nếu kỹ năng đó là triệu gọi (`FUN_00585874(skillId) != 0`), client kiểm tra vật phẩm thú đang đeo (`pet->0x44->id`) qua `FUN_005B1934`. Nếu thú **chưa đeo** Ngọc/Văn Chương triệu gọi, client đổi ID hiển thị về `0x36CA` (`14026` Triệu gọi).
  * Trong `FUN_0058E0BC` (`0058e0bc_FUN_0058e0bc.c:186-290` — cập nhật thanh phím tắt chiến đấu của thú): Client kiểm tra trực tiếp:
    * Nếu `skillId == 0x2720` (`10016` Nham quái): `pet->0x44->id` phải là **`0x5A2E` (`23086` Ngọc Nham Quái)** hoặc **`0x5A5F` (`23135` Nham Quái Văn)**.
    * Nếu `skillId == 0x2B08` (`11016` Thủy thần): `pet->0x44->id` phải là **`0x5A2F` (`23087` Ngọc Thủy Thần)** hoặc **`0x5A60` (`23136` Thủy Thần Văn)**.
    * Nếu `skillId == 0x2EF0` (`12016` Phụng hoàng): `pet->0x44->id` phải là **`0x5A30` (`23088` Ngọc Phụng Hoàng)** hoặc **`0x5A61` (`23137` Phụng Hoàn Văn)**.
    * Nếu `skillId == 0x32D7` (`13015` Thanh long): `pet->0x44->id` phải là **`0x5A31` (`23089` Ngọc Thanh Long)** hoặc **`0x5A62` (`23138` Thanh Long Văn)**.
    * Nếu không khớp, client đặt `btn->0x135 = 1` (vô hiệu hóa nút kỹ năng triệu gọi của thú trong trận)!

#### (2) Flow triệu gọi trong trận, "Bạch Hổ / Huyền Vũ" (Quang/Ám), và Cách thăng cấp kỹ năng Triệu gọi (`✅ VERIFIED`)
* **Client có tạo entity/NPC riêng khi dùng kỹ năng triệu gọi trong trận không?**
  * **KHÔNG!** Khi xuất chiêu `10016` (Nham quái), `11016` (Thủy thần), `12016` (Phụng hoàng), `13015` (Thanh long), hoặc **`22011..22014` (Bạch Hổ / Gọi tiểu-trung-đại Bạch Hổ)** và **`23011..23014` (Huyền Vũ / Gọi tiểu-trung-đại Huyền Vũ)** trong trận, client gửi lệnh đánh bình thường qua **`C→S 0x32 02`** với `skillId` tương ứng (`10016`, `11016`, `12016`, `13015`, `22011`, `23011`).
  * Server tính sát thương và gửi gói diễn hoạt chiến đấu (`S→C 0x32` / `0x0B`) về client; client phát animation triệu hồi thần thú theo trường `eff_pic` của kỹ năng đó trong `Skill.Dat` (chứ không sinh thêm `TFollowNpc` bằng gói `05 05`). *(Trong `Npc.dat`, các NPC tên `Nham Quái #35020`, `Nữ Thủy Thần #35021`, `Thanh Long #35029`, `Bí Hí #35032`, `Bạch Hổ #35054/#42239` có `b15 = 26/33` là các **Boss sự kiện/nhiệm vụ** đứng trên bản đồ, không phải thực thể do người chơi triệu hồi ra).*
* **Cách thăng cấp cho các kỹ năng Triệu gọi (`10016`, `11016`, `12016`, `13015`, `22011`, `23011`):**
  * Trong `Skill.Dat` / `skills.json`: `14026` (Triệu gọi gốc) có `max_lvl = 1, sk_point = 0` (học từ nhiệm vụ Động Quỷ Thần `map 11569, eve=3`).
  * Các kỹ năng triệu gọi 4 hệ (`10016, 11016, 12016, 13015`) và Quang/Ám (`22011 Bạch Hổ`, `23011 Huyền Vũ`) đều có **`max_lvl = 10`**, **`sk_point = 0`**, **`grade_point (+0x2D RAM) = 1`**.
  * Khi nhân vật đã được Server cấp cấp 1 (`level = 1`) của kỹ năng triệu gọi (qua nhiệm vụ Cự Thú / Rồng Thiêng hoặc học từ Sách `65147..65150`), người chơi có thể bấm nút `+` trên `TSe_SkillForm` (gửi `0x1C 01`) hoặc bấm tăng cấp trên `TSe_SkillTreeForm` (gửi `0x1C 04` / `0x1C 05`) để nâng từ cấp `1 → 10` với chi phí **1 điểm kỹ năng / 1 cấp** (`grade_point = 1`), hoặc tăng cấp tự động theo cơ chế độ thuần thục/sự kiện phía Server rồi đồng bộ về qua `S→C 08 01` prop `0x6E`.

### 6.5 NHÓM E — Nâng Cấp Kỹ Năng, Điểm Kỹ Năng (`+0x3EC` vs `+0x400`), Quy Tắc Giá Điểm & Sách Kỹ Năng

1. **Phân biệt `player+0x3EC` và `player+0x400` (`✅ VERIFIED`):**
   * `*(ushort *)(unit + 0x3EC)` = **SP hiện tại (`Current SP`)**. Được đọc tại `FUN_0059F090:654`, `FUN_0059D988:652`, `FUN_0058E0BC:311`, `FUN_00598C28:96` để so sánh với `TFSkill.require_sp (+0x1C RAM)`.
   * `*(ushort *)(unit + 0x400)` = **Điểm kỹ năng còn lại (`Skill Points`)**. Được cập nhật bởi `05 03` (`payload[0x18..0x19]`) và `08 01` prop `0x25` (`FUN_00710AB0` @ `0x007110E0`).
2. **Quy tắc tính giá điểm khi học mới vs nâng cấp (`✅ VERIFIED`):**
   * Trong `FUN_0059F090` (`0059f090_FUN_0059f090.c:406-608`) và `FUN_0057D0C8` (`0057d0c8_FUN_0057d0c8.c:372-642`):
     * **Khi `currentLevel == 0` (Học mới cấp 0 → 1):** Dùng trường **`TFSkill.sk_point (+0x2C RAM / +0x28 .Dat)`** (*"Điểm học được"*). Nếu `(player->0x3E9 - 1) == tab` (cùng hệ Ngũ hành với nhân vật), `cost = sk_point`; nếu khác hệ, `cost = sk_point * 2`. *(Lưu ý: đối với `DAT_0094B3A4` trong `FUN_005836C4:120-138`, mặt nạ bit `DAT_005839BC` bật sẵn bit 5 cho hệ Tâm `elem=5`, nên kỹ năng gốc nghề `14038` Đại chu thiên luôn tính hệ số `×1` = 1 điểm; các kỹ năng sống hệ Tâm `14001..14037` học qua nhiệm vụ hoặc có `sk_point=1` trên cây `0x24C[4]`).*
     * **Khi `currentLevel > 0` (Nâng cấp từ Lv 1 → Lv 2..max):** Dùng trường **`TFSkill.grade_point (+0x2D RAM / +0x29 .Dat)`** (*"Điểm thăng cấp"*, luôn bằng `1`). Đặc biệt: khi nâng cấp (`level > 0`), client **KHÔNG nhân đôi** `grade_point` ngay cả khi kỹ năng khác hệ! Tổng điểm tốn để học từ Lv 0 lên Lv `N` trên client là: `(1 hoặc 2) × sk_point + (N - 1) × grade_point`, **khớp 100% công thức trong `TSCharacter.rebornChar()` của C# Server**!
3. **Học kỹ năng từ Sách (`Item.Dat type=65/66`) (`✅ VERIFIED`):**
   * Khi người chơi double-click dùng vật phẩm trong túi đồ, client gọi `FUN_0076D578(itemId, targetUnit)` (`0076d578_FUN_0076d578.c`).
   * Nếu `item->type == 0x41` (`65` — 135 Sách kỹ năng Võ tướng đời 2 / Anh hùng `65029..65189`) hoặc `0x42` (`66`): `FUN_0076D578` đọc `skillId = *(ushort *)(item + 0x60)` (`+0x67` trong `Item.Dat`), kiểm tra đối tượng chưa học `skillId`, không bị khắc hệ (`FUN_00598944`), và nếu `skillId ∈ {14033, 14036, 14037}` thì Võ tướng đời 2 phải thuộc nghề Cung (`child->0x0A == 4`).
   * Nếu vượt qua gate, client chỉ gửi **`C→S 0x17 0F` (Use Item)** lên Server và **chờ Server gửi `S→C 08 01` prop `0x6E` (hoặc cập nhật con cái)** về chứ client không tự cộng skill trong RAM.

### 6.6 NHÓM F — Thanh Kỹ Năng (Hotkey), Chiến Đấu & Bảng Nhảy `FUN_006538BC`

#### (1) Thanh kỹ năng phím tắt ngoài trận (`0x28 01` / `S→C 0x19`) và trong trận (`0x29 01`) (`✅ VERIFIED`)
* **Ngoài trận (`TSe_QuickSkillForm`, `gvar_007D9EC0` — phím F1..F10):**
  * Khi kéo/gán kỹ năng vào ô F1..F10, `FUN_005AB678` (`005ab678_FUN_005ab678.c:109`) gửi **`C→S 0x28 01 { keyIdx: u8 (1..10), isPet: u8 (1=Nhân vật, 2=Thú), skillId: u16 LE }`**.
  * Server đồng bộ xuống qua **`S→C 0x19`** (`case_022_0079157C`):
    * `19 01` (`FUN_0075AF6C`), `19 02` (`FUN_0075AFF4`), `19 03` (`FUN_0075B3EC`): Cập nhật 1 ô phím tắt `{ keyIdx: u8 (1..10), isPet: u8, skillId: u16 LE }` vào `TSe_QuickSkillForm + 0x125..0x127`.
    * `19 0A` (`FUN_0075BF7C`): Đồng bộ toàn bộ **10 ô phím tắt** cùng lúc (`10 × 3 byte = 30 byte` `{ isPet: u8, skillId: u16 LE }`).
* **Trong trận (`TSe_FightHotKeyForm` — 6 ô phím tắt chiến đấu):**
  * Khi gán kỹ năng trong trận, `FUN_0058ED70` (`0058ed70_FUN_0058ed70.c:146`) gửi **`C→S 0x29 01 { slotIdx: u8 (0..5), unitType: u8, skillId: u16 LE }`**.

#### (2) Dùng kỹ năng trong trận (`C→S 0x32 02`, `unit+0x5A8`) (`✅ VERIFIED`)
* Builder `0x32 01` (`0x00788C38`) không có bất kỳ caller nào (code chết).
* Khi người chơi chọn 1 kỹ năng trong bảng phép chiến đấu (`TSe_FightSkillForm` hoặc phím tắt chiến đấu), mã kỹ năng được ghi vào **`*(ushort *)(unit + 0x5A8)`**.
* Khi người chơi click chọn mục tiêu trên bàn cờ trận đánh, hàm `TFightHum::FUN_0063CF58` (`0063cf58_FUN_0063cf58.asm.txt` tại `0x0063EDF6..0x0063EE0B`) đọc `*(ushort *)(EBX + 0x5A8)` và gọi **`FUN_00788E06` (`C→S 0x32 02`)** gửi đủ 6 byte: **`{ srcRow: u8, srcCol: u8, dstRow: u8, dstCol: u8, skillId: u16 LE }`**. Nhờ đó, **Server nhận được chính xác `skillId` (`u16 LE`) ở 2 byte cuối của gói `0x32 02`**!

#### (3) Giải mã Toàn Bộ Hàm `FUN_006538BC` & Bảng Nhảy 20 Entry tại `0x006539D7` (`✅ VERIFIED`)
Hàm `FUN_006538BC(unit, skillId)` (`006538bc_FUN_006538bc.asm.txt`) được gọi khi hiển thị hoặc chọn kỹ năng trong trận. Trình tự kiểm tra của hàm:
1. Kiểm tra `unit` đã học `skillId` chưa (`level > 0`). Nếu chưa học → trả về `0`.
2. Gọi `FUN_0075AAAC(skillId)` lấy `TFSkill.type`. Nếu **`type == 17` (`0x11`)** → hiện thông báo **`0x00653E28`** và trả về `0`.
3. Kiểm tra các nhánh `SUB` / `JE` trước bảng nhảy (`0x00653996..0x006539D5`):
   * Nếu `skillId` thuộc **`{ 10020 (0x2724), 11020 (0x2B0C), 12020 (0x2EF4), 13019 (0x32DB), 14003..14008 (0x36B3..0x36B8), 14016..14021 (0x36C0..0x36C5), 14023 (0x36C7), 14025..14028 (0x36C9..0x36CC) }`** → nhảy tới `0x00653A2B` (hiện thông báo **`0x00653E44`** và trả về `0`).
4. Với `skillId` trong dải **`0x36D1..0x36E4` (`14033..14052`, 20 entry)**, trừ `0x36D1` và nhảy qua bảng `0x006539D7` (`switch_table_6539d7_skill_id_jumptable.hex`):

| Chỉ số (`skillId - 0x36D1`) | Skill ID (`Dec` / `Hex`) | Tên kỹ năng (`skills.json`) | Địa chỉ đích trong bảng `0x006539D7` | Hành vi xử lý & Thông báo |
|:-:|:-:|---|---|---|
| `0` | **`14033`** (`0x36D1`) | **Xạ Tiễn** | `0x00653A0E` | Kiểm tra vũ khí đang đeo `equip[4]->type == 13` (Cung). Nếu không phải Cung → hiện **`0x00653EB4`** và trả về `0`. |
| `1` | **`14034`** (`0x36D2`) | **Kỵ Binh** | `0x00653A2B` | Chặn dùng trong trận → hiện **`0x00653E44`**, trả về `0`. |
| `2` | **`14035`** (`0x36D3`) | **Đúc thành** | `0x00653A2B` | Chặn dùng trong trận → hiện **`0x00653E44`**, trả về `0`. |
| `3` | **`14036`** (`0x36D4`) | **Bách Xuyên** | `0x00653A0E` | Kiểm tra đeo Cung (`equip[4]->type == 13`), nếu sai → hiện **`0x00653EB4`**, trả về `0`. |
| `4` | **`14037`** (`0x36D5`) | **Tiễn Vũ** | `0x00653A0E` | Kiểm tra đeo Cung (`equip[4]->type == 13`), nếu sai → hiện **`0x00653EB4`**, trả về `0`. |
| `5` | **`14038`** (`0x36D6`) | **Đại chu thiên** | `0x00653A2B` | Chặn dùng trong trận (Passive gốc nghề) → hiện **`0x00653E44`**, trả về `0`. |
| `6` | **`14039`** (`0x36D7`) | **Bá khí** (Bá 1) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `7` | **`14040`** (`0x36D8`) | **Bá ý** (Bá 2) | `0x00653DFB` (`default`) | **CHO PHÉP dùng trong trận** (tiếp tục kiểm tra trạng thái/SP, trả về `1`). |
| `8` | **`14041`** (`0x36D9`) | **Lực bạt sơn hề** (Bá 3) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `9` | **`14042`** (`0x36DA`) | **Nộ đào chiến đạp** (Bá 4) | `0x00653DFB` (`default`) | **CHO PHÉP dùng trong trận**. |
| `10` | **`14043`** (`0x36DB`) | **Anh khí hộ thân** (Hiệp 1) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `11` | **`14044`** (`0x36DC`) | **Thiên Thần thuẫn** (Hiệp 2) | `0x00653DFB` (`default`) | **CHO PHÉP dùng trong trận**. |
| `12` | **`14045`** (`0x36DD`) | **Tâm nhãn** (Hiệp 3) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `13` | **`14046`** (`0x36DE`) | **Ngưng Khí Hộ Thân** (Hiệp 4) | `0x00653DFB` (`default`) | **CHO PHÉP dùng trong trận**. |
| `14` | **`14047`** (`0x36DF`) | **Dụng kế** (Hiền 1) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `15` | **`14048`** (`0x36E0`) | **Trí thủ** (Hiền 2) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `16` | **`14049`** (`0x36E1`) | **Ngự địch** (Hiền 3) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `17` | **`14050`** (`0x36E2`) | **Sách động** (Hiền 4) | `0x00653DFB` (`default`) | **CHO PHÉP dùng trong trận**. |
| `18` | **`14051`** (`0x36E3`) | **Tiên khiếu** (Tiên 1) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |
| `19` | **`14052`** (`0x36E4`) | **Hóa Huyết Hoàn Hồn** (Tiên 2) | `0x00653A2B` | Chặn dùng trong trận (Passive) → hiện **`0x00653E44`**, trả về `0`. |

*(Hai kỹ năng Tiên 3 `14053 Thánh Linh thuật` và Tiên 4 `14054 Càn Khôn Thiên Lôi` nằm ngoài `0x36E4` nên đi thẳng vào nhánh `default 0x00653DFB` → **CHO PHÉP dùng trong trận**!).*

* **Giải mã VISCII 3 chuỗi thông báo của `FUN_006538BC` (`data/viscii_strings.tsv` — `✅ VERIFIED`):**
  1. **`0x00653E28`**: `"Đặc kỹ này là bị động, không cần sử dụng!"` (Hex: `80 64 63 20 6b fe 20 6e e0 79 20 6c e0 20 62 69 20 64 f4 6e 67 ...` — dùng khi `TFSkill.type == 17`).
  2. **`0x00653E44`**: `"Đặc kỹ này không thể dùng trong chiến đấu!"` (Dùng tại nhánh `0x00653A2B` cho toàn bộ kỹ năng sống và kỹ năng nghề bị động).
  3. **`0x00653EB4`**: `"Cần phải phối bị cung tên mới được dùng đặc kỹ này!"` (Dùng tại nhánh `0x00653A0E` cho 3 kỹ năng cung `14033, 14036, 14037` khi không đeo Cung `type == 13`).

## 7. Bảng Tổng Hợp 12 Quy Tắc Gate Phía Client (`skill_client_gates.tsv`)

Server bắt buộc phải đồng bộ 12 quy tắc kiểm tra dưới đây để không xảy ra lệch trạng thái với client:

| Mã Gate | Tên quy tắc chặn | Hàm & Địa chỉ VA | Điều kiện chặn chi tiết | Phản ứng của Client |
|:-:|---|---|---|---|
| **GATE-01** | Khắc hệ Ngũ hành (Địa↔Hỏa, Thủy↔Phong) | `FUN_00598944` (`0x00598944`) | `(player_elem==1 & skill_elem==3) \| (3 & 1) \| (2 & 4) \| (4 & 2)` | Báo *"Thuộc tính X không thể học kỹ năng Y hệ"* (`@0x598B68..0x598C04`), chặn học. Tâm (`5`) và Quang/Ám (`0`) không bị chặn. |
| **GATE-02** | 28 kỹ năng Chuyển sinh 1 | `FUN_005990E0` (`0x005990E0`) | `skillId ∈ {10020..10026, 11020..11026, 12020..12026, 13021..13027}` và `player->0x4B0 == 0` | Báo *"Đẳng cấp vẫn chưa học chuyển sinh"* (`@0x599278`). *(Lưu ý bug client: hệ Phong kiểm tra `13021..13027` thay vì `13019..13025`)*. |
| **GATE-03** | Điểm kỹ năng & Cấp tối đa trên `TSe_SkillForm` | `FUN_0059F090` / `FUN_0059D988` | `level >= max_lvl` hoặc `TFSkill+0x2B == -11` hoặc `player->0x400 < cost` (`Lv0`: `1×/2× sk_point`; `Lv>0`: `grade_point`) | Ẩn nút `+` nâng cấp (`btn->0x154 = 0`). |
| **GATE-04** | Điều kiện nâng node trên `TSe_SkillTreeForm` | `FUN_0057D0C8` (`0x0057D0C8`) | Khắc hệ; `self->0x254 < cost`; `level >= max_lvl`; `parent_level == 0`; khác nghề `player->0x1271` trên Tab 6..9; chưa học đủ 36 skill cơ bản của hệ khi bấm `10020/11020/12020/13019` | Hiện thông báo lỗi tương ứng (`@0x57DF10..0x57E094`), không tăng cấp node. |
| **GATE-05** | Điều kiện bấm OK lưu cây `TSe_SkillTreeForm` | `FUN_0057C840` (`0x0057C840`) | Còn `ball_point (player+0x1480) > 0` chưa gắn hết, hoặc mở bóng `skill_rb2` mà chưa chọn skill, hoặc không có thay đổi nào | Hiện cảnh báo (`@0x57CEAC` / `@0x57CF18`), không gửi `0x1C 04/05`. |
| **GATE-06** | Chặn kỹ năng bị động / ngoài trận khi đang chiến đấu | `FUN_006538BC` (`0x006538BC`) | `TFSkill.type == 17` hoặc `skillId` nằm trong danh sách 33 kỹ năng sống/nghề bị động | Báo `@0x653E28` hoặc `@0x653E44`, không cho chọn. |
| **GATE-07** | Bắt buộc đeo Cung (`type=13`) khi bắn tên | `FUN_006538BC` (`0x00653A0E`) | `skillId ∈ {14033, 14036, 14037}` và `equip[4]->type != 13` | Báo `@0x653EB4` (*"Cần phải phối bị cung tên..."*), trả về `0`. |
| **GATE-08** | Điều kiện dùng kỹ năng ngoài chiến đấu | `FUN_00598C28` / `005A94C0` / `005AA09C` | `unit->0x3EC (SP) < require_sp` (`@0x598EF3`); trừ `14003, 14023, 14009`, mọi skill khác phải có `skill_type == 4` (Heal) | Báo *"SP không đủ"* hoặc *"Không thể dùng trong trạng thái thường"*. |
| **GATE-09** | Điều kiện cưỡi ngựa thường (`14023`) & Chiến Tượng (`14034`) | `FUN_0073D558` / `005A1620` / `0063CF58` | `Npc.dat b15 == 9` cần `14023` (Cưỡi ngựa); `b15 == 22` cần `14034` (Kỵ Binh), trừ map `12001..12003, 12825` | Báo `@0x5A191C` (*"Phải học thuật kỵ binh"*), chặn cưỡi. |
| **GATE-10** | Điều kiện dùng Triệu gọi trên Thú cưng | `FUN_0058E0BC` / `005A0388` / `00584AD8` | Thú có `10016/11016/12016/13015` nhưng `pet->0x44->id` không thuộc `{23086..23089, 23135..23138}` | Đổi hiển thị thành `14026` trên cây thú và khóa phím tắt chiến đấu của thú. |
| **GATE-11** | Điều kiện tẩy kỹ năng (`TBK_SkillRegressForm`) | `FUN_005E4C14` (`0x005E4C14`) | Tẩy gốc hệ khi còn skill trong hệ; hạ Lv 1→0 khi còn skill con `level > 0`; hạ `14038` Lv 1→0 khi còn skill nghề `level > 0` | Báo lỗi `@0x5E55C8..0x5E5630`, chặn gửi `0x2A 01`. |
| **GATE-12** | Điều kiện dùng Sách học kỹ năng (`type=65/66`) | `FUN_0076D578` (`0x0076D578`) | Đã học `skill_link`, khắc hệ, chưa đủ cấp chuyển sinh, hoặc con cái không phải nghề Cung khi học `14033/14036/14037` | Báo lỗi từ chối, chặn gửi `0x17 0F`. |

## 8. Bảng Dữ Liệu Kỹ Năng Sống & Kỹ Năng Nghề (Trích Xuất Từ TSV)

*(Toàn bộ dữ liệu chi tiết nằm trong 7 file TSV tại `client_skills_handler/data/`. Dưới đây là bảng tổng hợp trực tiếp để tra cứu nhanh).*

### 8.1 Bảng Kỹ năng sống hệ Tâm → Nơi dùng trong Client (`data/life_skill_usage.tsv`)

| ID Server | ID Client (Canonical / Legacy) | Tên kỹ năng | Hàm & Địa chỉ tham chiếu trong Client | Giao diện UI liên quan | Tham chiếu File dữ liệu (`Item.Dat` / `Npc.dat` / `Eve.Emg`) | Ghi chú nghiệp vụ |
|:-:|---|---|---|---|-|---|
| 14001 | 14001 (legacy:-) | Thuật trinh thám | FUN_00579640@0x005796C0..0x00579CE0 (TSe_SkillTreeForm.BuildLifeTree root); FUN_00597954@0x00597954 (Tab 4 Tâm) | TSe_SkillTreeForm Tab 5 (5btn_02, node gốc 0); TSe_SkillForm Tab 4 (Tâm) | Data/Skill.Dat; Data/skills.json; Eve(map=12136:Đường lớn Trác Quận,eve=2); Eve(map=54901:Khu chuẩn bị lôi đài,eve=85); Eve(map=54901:Khu chuẩn bị lôi đài,eve=89); EveCond(map=12031:Nhà Lưu Bị,eve=5,op=5,val=0); EveCond(map=12244:,eve=4,op=4,val=1); EveCond(map=12404:Đỉnh Từ Vô Sơn,eve=2,op=4,val=1) | Gốc nhánh sống hệ Tâm (sk_point=0, max_lvl=1); tiền đề của 14002..14006; học qua nhiệm vụ Eve (map 12136/54901) |
| 14002 | 14002 (legacy:-) | Thuật đào tẩu | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu (TSe_FightSkillForm) | Data/Skill.Dat; Data/skills.json | Thuật đào tẩu — chủ động trong trận (skill_type=12, sp=10); tiền đề của 14007 (Giải hôn), 14008 (Hôn mê) |
| 14003 | 14003 (legacy:-) | Thuật mua bán | FUN_0072E9A0@0x0072F319,0x0072F428,0x0072F46A; FUN_00710AB0@0x007117A8,0x007117F6; FUN_00598C28@0x00598D3F (->FUN_005C182C); FUN_006538BC@0x00653A2B (chặn trong trận) | TCY_MoneyMenu (gvar_007DA108+0x13C nút Bày hàng); TSe_MainStatus (gvar_007DA530+0x1B8 nhấp nháy); TSe_SkillForm (click gọi FUN_005C182C mở sập hàng) | Data/Skill.Dat; Data/skills.json; Eve(map=12031:Nhà Lưu Bị,eve=5) | Thuật mua bán (0x36B3) — ✅ VERIFIED KHÔNG PHẢI cờ Quang/Ám! Khi học (level>0) bật nút Bày hàng trong TCY_MoneyMenu; dùng ngoài trận mở giao diện bày hàng; bị chặn trong trận (@0x653E44) |
| 14004 | 14004 (legacy:-) | Thuật dò xét | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Eve(map=12244:,eve=4); EveCond(map=14425:Đỉnh Chung Nam Sơn,eve=4,op=5,val=0); EveCond(map=14425:Đỉnh Chung Nam Sơn,eve=4,op=4,val=1) | Thuật dò xét (0x36B4) — bị động (type=0), tiền đề của 14009 (Thuật ẩn mình), 14010 (Thuật đào khoáng); server dùng khi bắt thú/dò xét; bị chặn trong trận (@0x653E44) |
| 14005 | 14005 (legacy:-) | Thuật luyện đơn | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Eve(map=12404:Đỉnh Từ Vô Sơn,eve=2); EveCond(map=11424:Lang Gia cung,eve=2,op=5,val=10); EveCond(map=12106:Đồng Tước đài,eve=2,op=5,val=10); EveCond(map=12107:Đồng Tước đài,eve=2,op=5,val=10) | Thuật luyện đơn (0x36B5) — bị động (max_lvl=10), tiền đề của 14011 (Thuật đúc luyện); gửi/xử lý chế tạo qua C→S 0x1A 04 (FUN_005C0288 trong TCY_MixMenu gvar_007DA374); bị chặn trong trận |
| 14006 | 14006 (legacy:-) | Đại hô | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23273:Đại hát huy hiệu(type=14); Item#65151:Sách đại hát(type=65); Npc.dat(81 NPC); Eve(map=13446:Trại lớn Quan Độ,eve=4); EveCond(map=14173:Làng Thiễm Bắc,eve=3,op=5,val=0); EveCond(map=14173:Làng Thiễm Bắc,eve=3,op=2,val=0) | Đại hô (0x36B6) — trong bảng client/server là nhánh dẫn tới 14012 (Đồng tâm), 14013 (Cổ Vũ); bị chặn trong trận tại FUN_006538BC (@0x653E44); có Huy hiệu 23273 & Sách 65151 |
| 14007 | 14007 (legacy:-) | Giải hôn | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_00598C28@0x00598EE0 / FUN_005A94C0 / FUN_005AA09C (dùng ngoài trận chỉ có hiệu quả với skill_type=4); FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#65189:Sách giải hôn thụy(type=65) | Giải hôn (0x36B7) — yêu cầu 14002; có Sách giải hôn thụy (Item#65189); lưu ý FUN_006538BC hard-code chặn 14003..14008 |
| 14008 | 14008 (legacy:-) | Hôn mê | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23275:Hôn thụy huy hiệu(type=14); Item#65152:Sách hôn thụy(type=65); Npc.dat(26 NPC) | Hôn mê (0x36B8) — yêu cầu 14002; có Huy hiệu 23275 & Sách 65152; nằm trong dải 14003..14008 của FUN_006538BC |
| 14009 | 14009 (legacy:-) | Người làm | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_00598C28@0x00598DA0 (dùng ngoài trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json | Thuật ẩn mình (0x36B9) — yêu cầu 14004; tiền đề của 14014 (Giải độc), 14015 (Bỏ độc) |
| 14010 | 14010 (legacy:-) | Thuật đào khoáng | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_00598C28@0x00598EE0 (chặn click dùng ngoài trận vì skill_type=0 != 4) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 (bị động) | Data/Skill.Dat; Data/skills.json; Eve(map=14425:Đỉnh Chung Nam Sơn,eve=4) | Thuật đào khoáng (0x36BA) — bị động (max_lvl=10); tiền đề của 14018 (Khai khoáng); KHÔNG có UI đào khoáng riêng ở client, đào khoáng thực hiện bằng trang bị Cuốc (Item#10001,10497,10530..10534 type=39/55) tại map mỏ |
| 14011 | 14011 (legacy:-) | Thuật đúc luyện  | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_005C0288 (TCY_MixMenu gửi C→S 0x1A 04) | TSe_SkillTreeForm Tab 5; TCY_MixMenu (gvar_007DA374) | Data/Skill.Dat; Data/skills.json; Eve(map=11424:Lang Gia cung,eve=2); EveCond(map=12106:Đồng Tước đài,eve=2,op=5,val=10); EveCond(map=12107:Đồng Tước đài,eve=2,op=5,val=10); EveCond(map=18102:Kiến Nghiệp nhai đạo,eve=4,op=2,val=9) | Thuật đúc luyện (0x36BB) — bị động (max_lvl=10), yêu cầu 14005; tiền đề của 14019 (Thuật tinh luyện); tăng tỷ lệ hợp thành đồ trong TCY_MixMenu (server tính xác suất) |
| 14012 | 14012 (legacy:14060) | Đồng tâm | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu (TSe_FightSkillForm) | Data/Skill.Dat; Data/skills.json; Item#65153:Sách đồng tâm(type=65); Npc.dat(16 NPC); Eve(map=14173:Làng Thiễm Bắc,eve=3) | Đồng tâm (0x36BC) — chủ động trong trận (Buff, sp=22, max_lvl=5), yêu cầu 14006; có Sách đồng tâm (Item#65153) |
| 14013 | 14013 (legacy:14061) | Cổ Vũ | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu (TSe_FightSkillForm) | Data/Skill.Dat; Data/skills.json; Item#65154:Sách cổ vũ(type=65); Npc.dat(16 NPC) | Cổ Vũ (0x36BD) — chủ động trong trận (Debuff/Buff, sp=25, max_lvl=5), yêu cầu 14006; có Sách cổ vũ (Item#65154) |
| 14014 | 14014 (legacy:-) | Giải độc | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#65155:Sách giải độc(type=65); Npc.dat(1 NPC); Eve(map=18836:,eve=2) | Giải độc (0x36BE) — chủ động trong trận (skill_type=5 giải trạng thái, sp=10, max_lvl=1), yêu cầu 14009; có Sách giải độc (Item#65155) |
| 14015 | 14015 (legacy:-) | Bỏ độc | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#23274:Thi độc huy hiệu(type=14); Item#56019:Thuốcdiệttrùng(type=56); Item#65157:Sách thi độc(type=65); Npc.dat(29 NPC); Eve(map=15174:Thôn Tiểu Bái,eve=2) | Bỏ độc / Thi độc (0x36BF) — chủ động trong trận (Active, sp=35, max_lvl=10), yêu cầu 14009; tiền đề của 14020..14022; có Huy hiệu 23274, Thuốc diệt trùng 56019, Sách 65157 |
| 14016 | 14016 (legacy:-) | Thuật giao dịch | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json | Tu dưỡng (0x36C0) — bị động hồi HP/SP (max_lvl=5), yêu cầu 14003; bị chặn trong trận tại FUN_006538BC (@0x653E44) |
| 14017 | 14017 (legacy:-) | Tịnh toạ | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json | Khai thương (0x36C1) — bị động mở rộng tiền trang/kho (max_lvl=5), yêu cầu 14016; tiền đề của 14017->14016; bị chặn trong trận (@0x653E44) |
| 14018 | 14018 (legacy:-) | Thuật đào sâu | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json | Khai khoáng (0x36C2) — bị động đào khoáng trung cấp (max_lvl=10), yêu cầu 14010; tiền đề của 14032 (Thâm khoáng); bị chặn trong trận (@0x653E44) |
| 14019 | 14019 (legacy:-) | Thuật chế tạo | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_005C0288 (TCY_MixMenu); FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 5; TCY_MixMenu (gvar_007DA374) | Data/Skill.Dat; Data/skills.json; Eve(map=18102:Kiến Nghiệp nhai đạo,eve=4); EveCond(map=12106:Đồng Tước đài,eve=2,op=5,val=5); EveCond(map=12107:Đồng Tước đài,eve=2,op=5,val=5); EveCond(map=18102:Kiến Nghiệp nhai đạo,eve=4,op=5,val=0) | Thuật tinh luyện (0x36C3) — bị động hợp thành trung cấp (max_lvl=10), yêu cầu 14011; tiền đề của 14035 (Đúc thành); bị chặn trong trận (@0x653E44) |
| 14020 | 14020 (legacy:-) | Hoạch lí phản | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json | Thiền Định (0x36C4) — bị động kháng trạng thái / hồi SP (max_lvl=5), yêu cầu 14015; tiền đề của 14021; bị chặn trong trận (@0x653E44) |
| 14021 | 14021 (legacy:-) | Hỗn loạn | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#23276:Hỗn loạn huy hiệu(type=14); Item#65158:Sách hỗn loạn(type=65); Npc.dat(79 NPC); Eve(map=11047:Thanh Châu,eve=3) | Hỗn loạn (0x36C5) — kỹ năng gây hỗn loạn (sp=25, max_lvl=5), yêu cầu 14015; có Huy hiệu 23276 & Sách hỗn loạn 65158; lưu ý FUN_006538BC hard-code 0x36C5 trong dải 14016..14021 |
| 14022 | 14022 (legacy:-) | Giải hỗn loạn | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#65159:Sách giải hỗn loạn(type=65); Npc.dat(1 NPC) | Giải hỗn loạn (0x36C6) — chủ động trong trận (skill_type=5, sp=10, max_lvl=1), yêu cầu 14015; có Sách giải hỗn loạn 65159 |
| 14023 | 14023 (legacy:-) | Cưỡi ngựa | FUN_0073D558@0x0073D5B6 (kiểm tra cưỡi ngựa thường param_2==9); FUN_006538BC@0x00653A2B (chặn trong trận); FUN_00598C28@0x00598D57 (->FUN_005A1620 xuống ngựa); FUN_005A94C0@0x005A9780; FUN_005AA09C@0x005AA1E0 | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4; TSe_QuickSkillForm (thanh phím tắt F1..F10) | Data/Skill.Dat; Data/skills.json; Eve(map=12001:U Châu,eve=38); EveCond(map=18301:Đường lớn KiếnNghiệp,eve=15,op=2,val=0) | Thuật kỵ mã / Cưỡi ngựa (0x36C7) — ✅ VERIFIED: kiểm tra tại FUN_0073D558 khi cưỡi 72 ngựa thường (Npc.dat b15=9, id 18001..18072); dùng ngoài trận/hotkey gọi FUN_005A1620 gửi C→S 0x0F 06 (xuống ngựa); bị chặn trong trận (@0x653E44) |
| 14024 | 14024 (legacy:-) | Linh quang ẩn hiện | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_00598C28@0x00598EE0 (chặn ngoài trận); FUN_006538BC (cho phép trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json | Thánh linh / Thánh Quang (0x36C8) — kỹ năng xem thông tin kỹ năng trong trận (max_lvl=1) |
| 14025 | 14025 (legacy:-) | Chiếm đọat | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#65160:Sách kinh đoạt(type=65); Npc.dat(5 NPC) | Chiếm đoạt / Đoạt Nguyên Thuật (0x36C9) — bị động nhân đôi vật phẩm rơi (max_lvl=1); có Sách kinh đoạt 65160; bị chặn trong trận (@0x653E44) |
| 14026 | 14026 (legacy:-) | Triệu gọi | FUN_0057E55C@0x0057EC94 (node placeholder 0x36CA trong 4 tab Địa/Thủy/Hỏa/Phong của TSe_SkillTreeForm); FUN_005A0388@0x005A058A,0x005A0710 (mask pet summon skill khi chưa đeo văn chương); FUN_00584AD8@0x00584D24; FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 0..3 (ô placeholder 0x36CA được thay bằng 10016/11016/12016/13015 khi học) & Tab 5; TSe_NPCSkillTreeForm | Data/Skill.Dat; Data/skills.json; Eve(map=11569:Động Quỷ Thần,eve=3) | Triệu gọi / Triệu Linh Thuật (0x36CA) — ✅ VERIFIED: 1) Kỹ năng gốc mở khóa nhánh triệu gọi 4 hệ; 2) Mã placeholder (0x36CA) trên 4 tab hệ và trên thú cưng khi thú chưa đeo Ngọc/Văn Chương triệu gọi (23086..23089, 23135..23138); bị chặn dùng trực tiếp trong trận (@0x653E44) |
| 14027 | 14027 (legacy:-) | Đạo sư | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Eve(map=11411:Thanh Châu,eve=12) | Cố hương / Gia Sư Thuật (0x36CB) — bị động tăng EXP khi dẫn người mới (max_lvl=1); học qua nhiệm vụ Eve (map 11411, eve=12); bị chặn trong trận (@0x653E44) |
| 14028 | 14028 (legacy:-) | Đoàn kết | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Eve(map=12842:Ký Châu,eve=6) | Tổ đội / Đoàn Kết Thuật (0x36CC) — bị động tăng EXP khi lập đội (max_lvl=1); học qua nhiệm vụ Eve (map 12842, eve=6); bị chặn trong trận (@0x653E44) |
| 14029 | 14029 (legacy:-) | Cát Phổ Trại bắn đạn | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Npc.dat(3 NPC) | Kỳ Môn Độn (0x36CD) — chủ động trong trận giảm AGI đối phương (Debuff, sp=25, max_lvl=10) |
| 14030 | 14030 (legacy:-) | Cát Phổ Trại áp đỉnh | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Npc.dat(5 NPC) | Càn Khôn Thiên / Kỳ Môn Xung (0x36CE) — chủ động trong trận phản đòn (Buff, sp=25, max_lvl=10) |
| 14031 | 14031 (legacy:-) | Tiếp chiêu | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC (cho phép dùng trong trận) | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Npc.dat(1 NPC) | Giải Trừ / Giải Trừ Công Kích (0x36CF) — chủ động trong trận xóa trạng thái (skill_type=5, sp=25, max_lvl=1) |
| 14032 | 14032 (legacy:-) | Tinh luyện | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Eve(map=12106:Đồng Tước đài,eve=2) | Thâm khoáng / Đào sâu (0x36D0) — bị động đào khoáng cao cấp (max_lvl=10), yêu cầu 14018; học qua Eve (map 12106, eve=2); bị chặn trong trận (@0x653E44) |
| 14033 | 14033 (legacy:-) | Xạ Tiễn | FUN_006538BC@0x00653A0E (nhánh cung tên: kiểm tra vũ khí đang đeo có type==13 Cung không); FUN_00579640@0x005796C0 | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#65163:Sáchxuyêntâmthuấnthỉ(type=65); Npc.dat(35 NPC); Eve(map=15863:Rừng Hạ Thái,eve=2) | Xạ Tiễn (0x36D1) — ✅ VERIFIED: chủ động trong trận (Active, sp=15, max_lvl=10); tại FUN_006538BC bắt buộc nhân vật đeo vũ khí Cung (equip[4].type == 13), nếu không báo @0x653EB4 'Cần phải phối bị cung tên mới được dùng đặc kỹ này!'; tiền đề của 14036, 14037; có Sách 65163 |
| 14034 | 14034 (legacy:-) | Kỵ Binh | FUN_0073D558@0x0073D5E8 (kiểm tra param_2==0x16 chiến mã); FUN_005A1620@0x005A177C (kiểm tra FUN_00625128==0x16 -> FUN_0073393C(0x36D2) báo @0x5A191C 'Phải học thuật kỵ binh'); FUN_0063CF58@0x0063EC5A (trong trận); FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 5; TSe_SkillForm Tab 4; TSe_PetForm (nút Cưỡi) | Data/Skill.Dat; Data/skills.json; Eve(map=12093:Thông đạo Đổng Tước,eve=4) | Kỵ Binh / Kỵ Thú Thuật (0x36D2) — ✅ VERIFIED: bắt buộc phải có khi cưỡi Chiến Tượng / chiến mã (Npc.dat b15=22, id 31001..31006) ở cả ngoài trận (FUN_005A1620, FUN_0073D558) và trong trận (FUN_0063CF58); học qua Eve (map 12093, eve=4); bị chặn dùng như phép trong trận (@0x653E44) |
| 14035 | 14035 (legacy:-) | Chuyển Thể | FUN_00579640@0x005796C0 (TSe_SkillTreeForm Tab 5); FUN_005C0288 (TCY_MixMenu); FUN_006538BC@0x00653A2B | TSe_SkillTreeForm Tab 5; TCY_MixMenu (gvar_007DA374) | Data/Skill.Dat; Data/skills.json; Npc.dat(1 NPC); EveCond(map=59301:Thông đạo Thiên Đình,eve=4,op=2,val=0) | Đúc thành / Luyện Tạo Thuật (0x36D3) — bị động hợp thành cao cấp (max_lvl=10), yêu cầu 14019; kiểm tra điều kiện Eve tại map 59301 (Thông đạo Thiên Đình); bị chặn trong trận (@0x653E44) |
| 14036 | 14036 (legacy:14052) | Bách Xuyên | FUN_006538BC@0x00653A0E (nhánh cung tên: bắt buộc equip[4].type == 13); FUN_00579640@0x005796C0 | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#65161:Sáchbáchbộxuyêndương(type=65); Npc.dat(35 NPC); Eve(map=23807:Trại Vân Mộng,eve=2); EveCond(map=14873:Rừng Tân Phong,eve=2,op=2,val=0); EveCond(map=21713:Trại Đông Ngô XBích,eve=8,op=2,val=0); EveCond(map=25310:Đường lớn Lạc Thành,eve=2,op=2,val=0) | Bách Xuyên / Bách Bộ Xuyên Dương (0x36D4) — chủ động trong trận (Active, sp=25, max_lvl=10), yêu cầu 14033; bắt buộc đeo Cung (type=13); học qua Eve (map 23807); có Sách 65161 |
| 14037 | 14037 (legacy:14053) | Tiễn Vũ  | FUN_006538BC@0x00653A0E (nhánh cung tên: bắt buộc equip[4].type == 13); FUN_00579640@0x005796C0 | TSe_SkillTreeForm Tab 5; Menu kỹ năng chiến đấu | Data/Skill.Dat; Data/skills.json; Item#65162:Sách tiễnvũcửuthiên(type=65); Npc.dat(56 NPC); Eve(map=15435:Đỉnh Giáp Thạch Sơn,eve=5); EveCond(map=15304:Từ Châu nhai đạo,eve=3,op=2,val=0) | Tiễn Vũ / Tiễn Vũ Cửu Thiên (0x36D5) — chủ động trong trận (Active AoE, sp=32, max_lvl=10), yêu cầu 14033; bắt buộc đeo Cung (type=13); học qua Eve (map 15435); có Sách 65162 |
| 14038 | 14038 (legacy:14054) | Đại chu thiên | FUN_00579E80/0057A858/0057B2D8/0057BD58 (gốc của cả 4 nhánh nghề Bá/Hiệp/Hiền/Tiên trong self->0x24C[0..3]); FUN_006538BC@0x00653A2B (chặn trong trận) | TSe_SkillTreeForm Tab 6..9 (6btn_02..9btn_02, node gốc 0 của mỗi tab nghề) | Data/Skill.Dat; Data/skills.json; Item#65115:Sách Đại chu thiên(type=65); Npc.dat(82 NPC) | Đại chu thiên (0x36D6) — kỹ năng gốc chung (sk_point=1, max_lvl=5, Passive) của cả 4 nhánh nghề sau chuyển sinh/tái sinh; có Sách Đại chu thiên (Item#65115); bị chặn trong trận (@0x653E44) |
| 14039 | 14039 (legacy:14055) | Bá khí | FUN_00579E80 (dựng cây nghề Bá); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 6 (6btn_02 - Nhánh Bá); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23257:Bá khí linh thạch(type=14); Npc.dat(19 NPC) | Kỹ năng nhánh nghề Bá (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14040 | 14040 (legacy:-) | Bá ý | FUN_00579E80 (dựng cây nghề Bá); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 6 (6btn_02 - Nhánh Bá); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23258:Bá ý linh thạch(type=14); Npc.dat(9 NPC) | Kỹ năng nhánh nghề Bá (require_sk=14038 Đại chu thiên); Active (skill_type=19, sp=46) — dùng được trong trận |
| 14041 | 14041 (legacy:-) | Lực bạt sơn hề | FUN_00579E80 (dựng cây nghề Bá); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 6 (6btn_02 - Nhánh Bá); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23259:LựcBạtSơnHềLinhThạch(type=14) | Kỹ năng nhánh nghề Bá (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14042 | 14042 (legacy:14058) | Sóng cuồng | FUN_00579E80 (dựng cây nghề Bá); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 6 (6btn_02 - Nhánh Bá); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23260:NộđàochiếnđLinhThạch(type=14); Npc.dat(19 NPC) | Kỹ năng nhánh nghề Bá (require_sk=14038 Đại chu thiên); Active (skill_type=19, sp=46) — dùng được trong trận |
| 14043 | 14043 (legacy:-) | Anh khí hộ thân | FUN_0057A858 (dựng cây nghề Hiệp); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 7 (7btn_02 - Nhánh Hiệp); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23261:A.KhíHộThầnLinhThạch(type=14); Npc.dat(18 NPC) | Kỹ năng nhánh nghề Hiệp (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14044 | 14044 (legacy:-) | ThiênThần thuẫn | FUN_0057A858 (dựng cây nghề Hiệp); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 7 (7btn_02 - Nhánh Hiệp); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23262:ThiênTThuẫnLinhThạch(type=14); Npc.dat(1 NPC) | Kỹ năng nhánh nghề Hiệp (require_sk=14038 Đại chu thiên); Active (skill_type=19, sp=46) — dùng được trong trận |
| 14045 | 14045 (legacy:-) | Tâm nhãn | FUN_0057A858 (dựng cây nghề Hiệp); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 7 (7btn_02 - Nhánh Hiệp); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23263:Tâm Nhãn Linh Thạch(type=14); Npc.dat(38 NPC) | Kỹ năng nhánh nghề Hiệp (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14046 | 14046 (legacy:-) | NgưngKhí HộThân | FUN_0057A858 (dựng cây nghề Hiệp); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 7 (7btn_02 - Nhánh Hiệp); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23264:N.KhíHộThânLinhThạch(type=14); Npc.dat(17 NPC) | Kỹ năng nhánh nghề Hiệp (require_sk=14038 Đại chu thiên); Active (skill_type=19, sp=46) — dùng được trong trận |
| 14047 | 14047 (legacy:-) | Dùng kế | FUN_0057B2D8 (dựng cây nghề Hiền); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 8 (8btn_02 - Nhánh Hiền); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23265:Dụng kế linh thạch(type=14) | Kỹ năng nhánh nghề Hiền (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14048 | 14048 (legacy:-) | Trí thủ | FUN_0057B2D8 (dựng cây nghề Hiền); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 8 (8btn_02 - Nhánh Hiền); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23266:Trí thủ linh thạch(type=14) | Kỹ năng nhánh nghề Hiền (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14049 | 14049 (legacy:-) | Chống địch | FUN_0057B2D8 (dựng cây nghề Hiền); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 8 (8btn_02 - Nhánh Hiền); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23267:Ngự địch linh thạch(type=14) | Kỹ năng nhánh nghề Hiền (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14050 | 14050 (legacy:-) | Sách động | FUN_0057B2D8 (dựng cây nghề Hiền); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 8 (8btn_02 - Nhánh Hiền); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23268:Sách động linh thạch(type=14) | Kỹ năng nhánh nghề Hiền (require_sk=14038 Đại chu thiên); Active (skill_type=19, sp=56) — dùng được trong trận |
| 14051 | 14051 (legacy:14035) | Tiên khiếu | FUN_0057BD58 (dựng cây nghề Tiên); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 9 (9btn_02 - Nhánh Tiên); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23269:Tiênkhiếu linh thạch(type=14); Npc.dat(10 NPC) | Kỹ năng nhánh nghề Tiên (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14052 | 14052 (legacy:14036) | HóaHuyếtHòanHồn | FUN_0057BD58 (dựng cây nghề Tiên); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 9 (9btn_02 - Nhánh Tiên); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23270:HHuyếtHHồn linhthạch(type=14) | Kỹ năng nhánh nghề Tiên (require_sk=14038 Đại chu thiên); Passive — FUN_006538BC@0x00653A2B chặn trong trận |
| 14053 | 14053 (legacy:14037) | ThánhLinh thuật | FUN_0057BD58 (dựng cây nghề Tiên); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 9 (9btn_02 - Nhánh Tiên); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23271:ThánhLinh linh thạch(type=14); Npc.dat(5 NPC) | Kỹ năng nhánh nghề Tiên (require_sk=14038 Đại chu thiên); Active (skill_type=1, sp=51) — dùng được trong trận |
| 14054 | 14054 (legacy:14038) | CànKhônThiênLôi | FUN_0057BD58 (dựng cây nghề Tiên); FUN_0057D0C8 (click nâng cấp); FUN_0057C840 (gửi 0x1C 04/05); FUN_006538BC | TSe_SkillTreeForm Tab 9 (9btn_02 - Nhánh Tiên); TSe_SkillForm Tab 4 | Data/Skill.Dat; Data/skills.json; Item#23272:C.KhônT.LôiLinhThạch(type=14); Npc.dat(3 NPC) | Kỹ năng nhánh nghề Tiên (require_sk=14038 Đại chu thiên); Active (skill_type=1, sp=72) — dùng được trong trận |
| 14055 | 14055 (legacy:-) | Tổn huyết | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json | Kỹ năng đặc biệt hệ Tâm (Tổn huyết, skill_type=Passive, max_lvl=0, sk_point=0) |
| 14056 | 14056 (legacy:-) | Tiên khí hộ thân | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json; Item#25086:Thẻ tiên khí hộ thân(type=16) | Kỹ năng đặc biệt hệ Tâm (Tiên khí hộ thân, skill_type=5, max_lvl=0, sk_point=0) |
| 14057 | 14057 (legacy:14041) | Vật lý giảm nữa | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json; Item#25087:Chướng lệ chi hoàn(type=19); Item#25088:Chướng lệ chi thạch(type=19); Item#25089:Chướng lệ chi ngọc(type=19) | Kỹ năng đặc biệt hệ Tâm (Vật lý giảm nữa, skill_type=Passive, max_lvl=10, sk_point=1) |
| 14058 | 14058 (legacy:14042) | Thiết bố sam | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json | Kỹ năng đặc biệt hệ Tâm (Thiết bố sam, skill_type=Passive, max_lvl=10, sk_point=1) |
| 14059 | 14059 (legacy:14043) | Đằng thuật giáp | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json | Kỹ năng đặc biệt hệ Tâm (Đằng thuật giáp, skill_type=Passive, max_lvl=10, sk_point=1) |
| 14060 | 14060 (legacy:14012) | Toàn thể Thiên Lôi | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json | Kỹ năng đặc biệt hệ Tâm (Toàn thể Thiên Lôi, skill_type=Buff, max_lvl=5, sk_point=1) |
| 14061 | 14061 (legacy:14013) | Thiên Lôi Thần Thuẫn | FUN_00759B2C (nạp từ Skill.Dat vào 0x9AE128); FUN_00597954 (xếp vào Tab 4 Tâm nếu học); FUN_006538BC (kiểm tra type==17 nếu là 14056) | TSe_SkillForm Tab 4 (Tâm) / Kỹ năng đặc biệt trang bị & NPC | Data/Skill.Dat; Data/skills.json | Kỹ năng đặc biệt hệ Tâm (Thiên Lôi Thần Thuẫn, skill_type=Passive, max_lvl=10, sk_point=1) |
| 20006 | 20006 (legacy:19990) | Hộ chủ | FUN_00759B2C (nạp từ Skill.Dat); FUN_006538BC (kiểm tra trong trận) | Menu chiến đấu NPC / Chiến Tượng / Xe công thành | Data/Skill.Dat; Data/skills.json; Npc.dat(39 NPC) | Kỹ năng hệ Tâm đặc biệt (Hộ chủ, id=20006) |
| 20033 | 20033 (legacy:-) | Kiếm tuyệt vọng | FUN_00759B2C (nạp từ Skill.Dat); FUN_006538BC (kiểm tra trong trận) | Menu chiến đấu NPC / Chiến Tượng / Xe công thành | Data/Skill.Dat; Data/skills.json; Npc.dat(75 NPC) | Kỹ năng hệ Tâm đặc biệt (Kiếm tuyệt vọng, id=20033) |
| 20047 | 20047 (legacy:20063) | Cường hiệu thi độc | FUN_00759B2C (nạp từ Skill.Dat); FUN_006538BC (kiểm tra trong trận) | Menu chiến đấu NPC / Chiến Tượng / Xe công thành | Data/Skill.Dat; Data/skills.json | Kỹ năng hệ Tâm đặc biệt (Cường hiệu thi độc, id=20047) |
| 20051 | 20051 (legacy:20067) | Cửu vỉ hồ mi hoặc | FUN_00759B2C (nạp từ Skill.Dat); FUN_006538BC (kiểm tra trong trận) | Menu chiến đấu NPC / Chiến Tượng / Xe công thành | Data/Skill.Dat; Data/skills.json | Kỹ năng hệ Tâm đặc biệt (Cửu vỉ hồ mi hoặc, id=20051) |
| 20052 | 20052 (legacy:-) | Kiếm tuyệt vọng 4 | FUN_00759B2C (nạp từ Skill.Dat); FUN_006538BC (kiểm tra trong trận) | Menu chiến đấu NPC / Chiến Tượng / Xe công thành | Data/Skill.Dat; Data/skills.json | Kỹ năng hệ Tâm đặc biệt (Kiếm tuyệt vọng 4, id=20052) |
| NONE | NONE | Dung thọ (Nấu ăn — KHÔNG TỒN TẠI) | Không có hàm/hằng số trong aLogin.exe (0 kết quả grep VISCII & Skill.Dat) | Thay thế bởi: TCY_MixMenu (gvar_007DA374, C→S 0x1A 04) + TSe_InteractiveForm (lò nấu/chế tạo sự kiện C→S 0x33 01) | Data/Skill.Dat (0 match); Data/skills.json (0 match); Item.Dat (vật phẩm thức ăn hồi HP/SP chế từ Thuật luyện đơn 14005 / Thuật đúc luyện 14011) | ✅ VERIFIED: Trong TS Online KHÔNG có kỹ năng 'Dung thọ'. Chế biến thức ăn/đơn dược dùng hệ thống Hợp thành (TCY_MixMenu, opcode 0x1A 04) kết hợp chuỗi kỹ năng Thuật luyện đơn (14005) -> Thuật đúc luyện (14011) -> Thuật tinh luyện (14019) -> Đúc thành (14035) và Lò sự kiện (0x33 01). |

## 9. Bảng Kỹ Năng Nghề & Bóng Tái Sinh → Nơi Dùng (`data/job_skill_usage.tsv`)

| ID Server | ID Legacy | Tên kỹ năng | Phân nhóm | Nhánh nghề / Hệ | Mã `job` Server | Tab / Slot Client | Yêu cầu (`require_sk`) | Cấp tối đa | Dùng trong trận | Hàm & Địa chỉ Client | Vật phẩm liên kết (`Item.Dat`) |
|:-:|:-:|---|---|---|:-:|:-:|:-:|:-:|:-:|---|---|
| 14038 | 14054 | Đại chu thiên | Job Root (Gốc chung 4 nghề) | Tất cả (Bá/Hiệp/Hiền/Tiên) | 1..4 | Tab 6..9 (node 0) | 0 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_00579E80/0057A858/0057B2D8/0057BD58; FUN_005836C4; FUN_0057D0C8; FUN_0057C840 | Item#65115:Sách Đại chu thiên(type=65) |
| 14039 | 14055 | Bá khí | Nhánh nghề Chuyển sinh | Bá (Tab 6 - 6btn_02) | 2 | Tab 6 (node 1, self->0x24C[0]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_00579E80@0x00579E80; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23257:Bá khí linh thạch(type=14) |
| 14040 | - | Bá ý | Nhánh nghề Chuyển sinh | Bá (Tab 6 - 6btn_02) | 2 | Tab 6 (node 2, self->0x24C[0]) | 14038 | 5 | YES (Active skill_type=19) | FUN_00579E80@0x00579E80; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23258:Bá ý linh thạch(type=14) |
| 14041 | - | Lực bạt sơn hề | Nhánh nghề Chuyển sinh | Bá (Tab 6 - 6btn_02) | 2 | Tab 6 (node 3, self->0x24C[0]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_00579E80@0x00579E80; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23259:LựcBạtSơnHềLinhThạch(type=14) |
| 14042 | 14058 | Sóng cuồng | Nhánh nghề Chuyển sinh | Bá (Tab 6 - 6btn_02) | 2 | Tab 6 (node 4, self->0x24C[0]) | 14038 | 5 | YES (Active skill_type=19) | FUN_00579E80@0x00579E80; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23260:NộđàochiếnđLinhThạch(type=14) |
| 14043 | - | Anh khí hộ thân | Nhánh nghề Chuyển sinh | Hiệp (Tab 7 - 7btn_02) | 1 | Tab 7 (node 1, self->0x24C[1]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057A858@0x0057A858; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23261:A.KhíHộThầnLinhThạch(type=14) |
| 14044 | - | ThiênThần thuẫn | Nhánh nghề Chuyển sinh | Hiệp (Tab 7 - 7btn_02) | 1 | Tab 7 (node 2, self->0x24C[1]) | 14038 | 5 | YES (Active skill_type=19) | FUN_0057A858@0x0057A858; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23262:ThiênTThuẫnLinhThạch(type=14) |
| 14045 | - | Tâm nhãn | Nhánh nghề Chuyển sinh | Hiệp (Tab 7 - 7btn_02) | 1 | Tab 7 (node 3, self->0x24C[1]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057A858@0x0057A858; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23263:Tâm Nhãn Linh Thạch(type=14) |
| 14046 | - | NgưngKhí HộThân | Nhánh nghề Chuyển sinh | Hiệp (Tab 7 - 7btn_02) | 1 | Tab 7 (node 4, self->0x24C[1]) | 14038 | 5 | YES (Active skill_type=19) | FUN_0057A858@0x0057A858; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23264:N.KhíHộThânLinhThạch(type=14) |
| 14047 | - | Dùng kế | Nhánh nghề Chuyển sinh | Hiền (Tab 8 - 8btn_02) | 3 | Tab 8 (node 1, self->0x24C[2]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057B2D8@0x0057B2D8; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23265:Dụng kế linh thạch(type=14) |
| 14048 | - | Trí thủ | Nhánh nghề Chuyển sinh | Hiền (Tab 8 - 8btn_02) | 3 | Tab 8 (node 2, self->0x24C[2]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057B2D8@0x0057B2D8; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23266:Trí thủ linh thạch(type=14) |
| 14049 | - | Chống địch | Nhánh nghề Chuyển sinh | Hiền (Tab 8 - 8btn_02) | 3 | Tab 8 (node 3, self->0x24C[2]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057B2D8@0x0057B2D8; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23267:Ngự địch linh thạch(type=14) |
| 14050 | - | Sách động | Nhánh nghề Chuyển sinh | Hiền (Tab 8 - 8btn_02) | 3 | Tab 8 (node 4, self->0x24C[2]) | 14038 | 10 | YES (Active skill_type=19) | FUN_0057B2D8@0x0057B2D8; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23268:Sách động linh thạch(type=14) |
| 14051 | 14035 | Tiên khiếu | Nhánh nghề Chuyển sinh | Tiên (Tab 9 - 9btn_02) | 4 | Tab 9 (node 1, self->0x24C[3]) | 14038 | 10 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057BD58@0x0057BD58; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23269:Tiênkhiếu linh thạch(type=14) |
| 14052 | 14036 | HóaHuyếtHòanHồn | Nhánh nghề Chuyển sinh | Tiên (Tab 9 - 9btn_02) | 4 | Tab 9 (node 2, self->0x24C[3]) | 14038 | 5 | NO (Passive, chặn tại FUN_006538BC@0x00653A2B) | FUN_0057BD58@0x0057BD58; FUN_0057D0C8; FUN_0057C840; FUN_006538BC@0x00653A2B | Item#23270:HHuyếtHHồn linhthạch(type=14) |
| 14053 | 14037 | ThánhLinh thuật | Nhánh nghề Chuyển sinh | Tiên (Tab 9 - 9btn_02) | 4 | Tab 9 (node 3, self->0x24C[3]) | 14038 | 5 | YES (Active skill_type=1) | FUN_0057BD58@0x0057BD58; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23271:ThánhLinh linh thạch(type=14) |
| 14054 | 14038 | CànKhônThiênLôi | Nhánh nghề Chuyển sinh | Tiên (Tab 9 - 9btn_02) | 4 | Tab 9 (node 4, self->0x24C[3]) | 14038 | 5 | YES (Active skill_type=1) | FUN_0057BD58@0x0057BD58; FUN_0057D0C8; FUN_0057C840; FUN_006538BC | Item#23272:C.KhônT.LôiLinhThạch(type=14) |
| 10027 | - | Vô Gian Chấn Ba | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #1 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10011 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23224:Vôgianchấnba huyhiệu(type=14); Item#65116:Sách vô gian chấn ba(type=65) |
| 10028 | - | Cát Thiên Hồn Phủ | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #2 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10021 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23225:Hồn Phủ Huy Hiệu(type=14); Item#65117:Sách Hồn Phủ(type=65) |
| 10029 | - | Lưu Tinh Thiết Pháo | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #3 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10005 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23226:LưuTinhT.PháoHuyHiệu(type=14); Item#65118:Sáchlưutinhthiếtpháo(type=65) |
| 10030 | - | Lang Tinh Pháo | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #4 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10008 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23227:LangTinhPháo HuyHiệu(type=14); Item#65119:Sách lang tinh pháo(type=65) |
| 10031 | 9983 | Kim Chung Tráo | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #5 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10015 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23228:KimChungTráo HuyHiệu(type=14); Item#65120:Sách kim chung tráo(type=65) |
| 10032 | - | ChiếnTượngTrùngPhong | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #6 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10013 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23229:ChiếnTượngXP HuyHiệu(type=14); Item#65121:Sách chiếntượngXP(type=65) |
| 10033 | - | Sa Bộc Cầm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #7 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10025 | 5 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23230:Sa bạo cầm huy hiệu(type=14); Item#65122:Sách sa bộc cầm(type=65) |
| 10034 | - | Đồ Nham Vương Sát | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 1 (Địa / Kim) - Bóng #8 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 0 | 10024 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23231:Vương Sát HuyHiệu(type=14); Item#65123:Sách Vương Sát(type=65) |
| 11027 | - | Thiên Hồi Băng Vũ | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #1 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11014 | 5 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23233:T.HồiBăngVũ HuyHiệu(type=14); Item#65124:Sách thiên băng vũ(type=65) |
| 11028 | - | Sương Lôi Quyền | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #2 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11014 | 5 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23234:SươngLôiQuyềnHuyHiệu(type=14); Item#65125:Sách sương lôi quyền(type=65) |
| 11029 | - | Siêu Hồi Ma Thuật | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #3 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11009 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23235:Siêu hồi ma huy hiệu(type=14); Item#65126:Sách siêu hồi ma(type=65) |
| 11030 | - | Thiên Giáng Cam Lâm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #4 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11010 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23236:Cam Lâm Huy Hiệu(type=14); Item#65127:Sách t.giáng cam lâm(type=65) |
| 11031 | - | SiêuTrạng Giải Thuật | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #5 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11025 | 1 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#65128:Sách Giải Thuật(type=65) |
| 11032 | - | Minh Hà Vô Tưởng | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #6 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11024 | 5 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23237:Minh Hà Vô Tưởng HH(type=14); Item#65129:Sách minhhạvôtưởng(type=65) |
| 11033 | - | Trảm Kỵ Nhất Thiểm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #7 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11022 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23238:Nhất Thiểm Huy Hiệu(type=14); Item#65130:Sách Nhất Thiểm(type=65) |
| 11034 | - | Lục Hồi Thiên BăngVũ | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 2 (Thủy) - Bóng #8 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 1 | 11023 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23239:Lục Băng Vũ Huy Hiệu(type=14); Item#65131:Sách Lục băng vũ(type=65) |
| 12027 | - | Hỏa Bạo Nhất Kích | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #1 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12006 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23240:Nhất Kích Huy Hiệu(type=14); Item#65132:Sách hỏabạo nhấtkích(type=65) |
| 12028 | - | Liệt Thuật Hào Hỏa | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #2 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12011 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23241:Hào Hỏa Huy Hiệu(type=14); Item#65133:Sách liệtthuậthàohỏa(type=65) |
| 12029 | 12077 | Xích Long Cự | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #3 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12004 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23242:XíchLongCự Huy Hiệu(type=14); Item#65134:Sách xích long cự(type=65) |
| 12030 | - | Lôi Hỏa Thiệu Lâm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #4 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12010 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23243:LôiHỏaC.Lâm HuyHiệu(type=14); Item#65135:Sách lôihỏachiêulâm(type=65) |
| 12031 | 12079 | Dương Viêm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #5 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12015 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23244:Dương viêm huy hiệu(type=14); Item#65136:Sách dương viêm(type=65) |
| 12032 | - | Chúc Dung Phần Dã | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #6 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12014 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23245:ChúcDungP.Dã HuyHiệu(type=14); Item#65137:Sách chúcdungphầndã(type=65) |
| 12033 | - | Long Bá Liệt Trảm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #7 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12022 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23246:T.LongBáLiệt HuyHiệu(type=14); Item#65138:Sách tướclongbáliệt(type=65) |
| 12034 | - | Long Vương Chấn Hề | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 3 (Hỏa) - Bóng #8 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 2 | 12026 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23247:L Vương Chấn Hề HH(type=14); Item#65139:Sách longvươngchấnhề(type=65) |
| 13027 | - | Phong Huyễn Thần Ly | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #1 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13006 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23250:PhongThầnLy Huy Hiệu(type=14); Item#65141:Sách phonghuyềntđiêu(type=65) |
| 13026 | 13010 | Băng Lôi | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #2 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13009 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23249:Băng lôi huy hiệu(type=14); Item#65140:Sách băng lôi(type=65) |
| 13028 | - | Trảm Kỵ Phi Kiếm | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #3 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13004 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23251:Trảmkỵphikiếmhuyhiệu(type=14); Item#65142:Sách trảmkỵphikiếm(type=65) |
| 13029 | - | Sơn Lam | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #4 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13024 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23252:Sơn lam huy hiệu(type=14); Item#65143:Sách sơn lam(type=65) |
| 13030 | - | Thức Qủy Hấp Hồn | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #5 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13014 | 5 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23253:Thựcquỷhấphồnhuyhiệu(type=14); Item#65144:Sách hấp hồn(type=65) |
| 13031 | - | Phong Kình Lôi Minh | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #6 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13023 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23254:Lôi Minh Huy Hiệu(type=14); Item#65145:Sách Lôi Minh(type=65) |
| 13032 | - | Ẩn Phân Thân Thuật | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #7 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13005 | 5 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23255:ẨnPhânThân Huy Hiệu(type=14) |
| 13033 | - | Chấn Điện | Kỹ năng Bóng Tái Sinh (skill_rb2) | Hệ 4 (Phong) - Bóng #8 | 1..4 (yêu cầu Reborn 2: player->0x4B0 >= 2) | DAT_0094B3A4[6..13] (8 slot bóng) & TSe_SkillTreeForm Tab 3 | 13013 | 10 | YES (Active/Buff Reborn 2) | FUN_00586370/005876F0 (TWF_TurnTwoHeroSkillForm); FUN_005836C4 (kiểm tra mở khóa bóng trong DAT_0094B3A4); FUN_00749FD4 (S→C 17 4E); FUN_0057C840/00583A98 (C→S 1C 05) | Item#23256:Chấn điện huy hiệu(type=14); Item#65146:Sách chấn điện(type=65) |

## 10. Câu Hỏi Mở (`❓ OPEN`) & Đề Xuất Cách Đóng

Sau 12 đợt dump và đợt kiểm chứng chéo toàn diện này, **100% câu hỏi cốt lõi phục vụ lập trình Server (13/13 câu hỏi ở §3) đã được đóng ở mức `✅ VERIFIED`**. Chỉ còn 2 hạng mục phụ thuộc tài nguyên ngoài hoặc mang tính lưu trữ:

1. **❓ OPEN-01 — Chữ vẽ bên trong các file ảnh BMP ngoài của Tab 5..9 (`5btn_02.bmp`..`9btn_02.bmp`) và nền `1Form_SkillTree.bmp`..`7Form_SkillTree.bmp`:**
   * **Bằng chứng đã chốt:** Vòng lặp `0x005AC7B2..0x005AC846` trong `FUN_005AC75C` tạo 5 nút tab `5..9` bằng tên tài nguyên ảnh `"5btn_02"`..`"9btn_02"` (không gán chuỗi text caption). Tương tự, `FUN_0057922C` nạp nền form từ `"1Form_SkillTree.bmp"`..`"7Form_SkillTree.bmp"`.
   * **Trạng thái nghiệp vụ:** Về mặt logic, đã xác định chắc chắn **Tab 5 = Kỹ năng sống hệ Tâm (`14001..14037`)**, **Tab 6 = Bá (`14039..14042`)**, **Tab 7 = Hiệp (`14043..14046`)**, **Tab 8 = Hiền (`14047..14050`)**, **Tab 9 = Tiên (`14051..14054`)**.
   * **Cách đóng nếu cần xem chữ hiển thị trên ảnh:** Lấy thư mục ảnh `pic/` hoặc `data/` của bộ cài TS Online client đầy đủ, mở trực tiếp `5btn_02.bmp`..`9btn_02.bmp`.
2. **❓ OPEN-02 — Lưu trữ file `.c` cho 8 hàm helper nhỏ (Tùy chọn — Ghidra Round-14):**
   * Hành vi của 8 hàm helper (`FUN_00582A7C`, `FUN_005810B4`, `FUN_00585874`, `FUN_005B1934`, `FUN_00586AAC`, `FUN_00748FAC`, `FUN_005E5A38`, `FUN_005E7F90`) đã được xác định từ nơi gọi (callsites) và đối chiếu server C#. Nếu muốn lưu đủ 100% file decompile `.c` trong bundle, chỉ cần chạy manifest 12 lệnh đã cập nhật tại Mục 12 của `client_skills_handler/MISSING_DUMPS.md`.

## 11. Phụ Lục — Trích Dẫn ASM Quan Trọng (Kèm Địa Chỉ VA)

### 11.1 Builder `C→S 0x1C 01` & `0x1C 02` (`FUN_005990E0` — `005990e0_FUN_005990e0.asm.txt`)

```asm
; --- Nhánh thú cưng (0x1C 02): petSlot (1..4) + skillId (u16 LE) + targetLevel (u8) ---
0059914c: E8E3401800      CALL 0x0071d234               ; EAX = petSlot (1..4) trong player->0x57C
00599160: E8CF4F1E00      CALL 0x0077e134               ; u8 -> AnsiString (1 byte petSlot)
00599165: 8B45FC          MOV  EAX,dword ptr [EBP-0x4]  ; EAX = skill button
00599168: 8B90E4000000    MOV  EDX,dword ptr [EAX+0xE4] ; EDX = skillId
0059917c: E89B591E00      CALL 0x0077eb1c               ; u16 -> AnsiString (2 byte skillId LE)
00599195: 8B9008010000    MOV  EDX,dword ptr [EAX+0x108]; EDX = currentLevel
005991a5: 83C201          ADD  EDX,0x1                  ; EDX = currentLevel + 1 (targetLevel)
005991a8: E8874F1E00      CALL 0x0077e134               ; u8 -> AnsiString (1 byte targetLevel)
005991c3: B102            MOV  CL,0x2                   ; subOp = 0x02
005991c5: B21C            MOV  DL,0x1c                  ; mainOp = 0x1C
005991ca: E845621E00      CALL 0x0077f414               ; gửi C->S 0x1C 02

; --- Nhánh nhân vật (0x1C 01): skillId (u16 LE) + targetLevel (u8) ---
005991df: 8B90E4000000    MOV  EDX,dword ptr [EAX+0xE4] ; EDX = skillId
005991f9: E81E591E00      CALL 0x0077eb1c               ; -> 2 byte skillId LE
00599212: 8B9008010000    MOV  EDX,dword ptr [EAX+0x108]; EDX = currentLevel
00599222: 83C201          ADD  EDX,0x1                  ; EDX = currentLevel + 1 (targetLevel)
0059923d: E892AECEFF      CALL 0x004040d4               ; @LStrCat3(dest, skillId_2B, level_1B)
00599249: B101            MOV  CL,0x1                   ; subOp = 0x01
0059924b: B21C            MOV  DL,0x1c                  ; mainOp = 0x1C
0059924f: E8C0611E00      CALL 0x0077f414               ; gửi C->S 0x1C 01
```

### 11.2 Hàm đóng gói payload `C→S 0x1C 05` (`FUN_00583A98` — `00583a98_FUN_00583a98.asm.txt`)

```asm
; Ghép 3 đoạn: [u32 L1][self->0x288] + [u32 L2][self->0x28C] + [u32 L3][self->0x290]
00583ac6: 8B8388020000    MOV  EAX,dword ptr [EBX+0x288]; chuỗi L1 (danh sách bi rồng 1..12)
00583acc: E83705E8FF      CALL 0x00404008               ; @LStrLen(self->0x288) -> L1 byte
00583ad3: E894B01F00      CALL 0x0077eb6c               ; u32 -> 4 byte LE
00583b05: 8B838C020000    MOV  EAX,dword ptr [EBX+0x28c]; chuỗi L2 (danh sách cặp 3B {skillId, lvl})
00583b0b: E8F804E8FF      CALL 0x00404008               ; @LStrLen(self->0x28c)
00583b10: B903000000      MOV  ECX,0x3
00583b15: 99              CDQ
00583b16: F7F9            IDIV ECX                      ; L2 = len(self->0x28c) / 3 (SỐ PHẦN TỬ!)
00583b1a: E84DB01F00      CALL 0x0077eb6c               ; u32 -> 4 byte LE
00583b4c: 8B8390020000    MOV  EAX,dword ptr [EBX+0x290]; chuỗi L3 (danh sách 3B {slot 6..13, skillId})
00583b52: E8B104E8FF      CALL 0x00404008               ; @LStrLen(self->0x290) -> L3 (SỐ BYTE!)
00583b59: E80EB01F00      CALL 0x0077eb6c               ; u32 -> 4 byte LE
```

### 11.3 Xử lý `0x36B3` (`14003` Thuật mua bán) và Xóa 3000B bảng skill trong `S→C 05 03` (`FUN_0072E9A0` — `0072e9a0_FUN_0072e9a0.asm.txt`)

```asm
0072f312: A108A17D00      MOV  EAX,dword ptr [0x007da108] ; EAX = TCY_MoneyMenu
0072f317: 8B00            MOV  EAX,dword ptr [EAX]
0072f319: 8B803C010000    MOV  EAX,dword ptr [EAX+0x13c]  ; nút Bày hàng (TCY_MoneyMenu+0x13C)
0072f31f: 33D2            XOR  EDX,EDX                    ; EDX = 0 (disable)
0072f321: E86E0D0800      CALL 0x007b0094                 ; tắt nút Bày hàng trước khi đọc 05 03
0072f328: BAF80B0000      MOV  EDX,0xbb8                  ; 3000 byte (1000 slot * 3B)
0072f32d: 33C9            XOR  ECX,ECX
0072f32f: E86839CDFF      CALL 0x00402c9c                 ; @_FillChar(player+0xFD4, 3000, 0)
...
0072f428: 6681BC02D40F0000B336 CMP word ptr [EDX+EAX+0xfd4],0x36b3 ; skillId == 14003 (Thuật mua bán)?
0072f46a: 8B803C010000    MOV  EAX,dword ptr [EAX+0x13c]  ; TCY_MoneyMenu+0x13C
0072f470: B201            MOV  DL,0x1                     ; DL = 1 (enable)
0072f472: E81D0C0800      CALL 0x007b0094                 ; bật nút Bày hàng!
```

### 11.4 Xuất chiêu kỹ năng trong trận qua `C→S 0x32 02` (`TFightHum::FUN_0063CF58` — `0063cf58_FUN_0063cf58.asm.txt`)

```asm
0063edf6: 668B83A8050000  MOV  AX,word ptr [EBX+0x5a8]    ; AX = selectedSkillId (unit+0x5A8)
0063edfd: 50              PUSH EAX                        ; arg5: skillId (u16 LE)
0063edfe: 8A4DF7          MOV  CL,byte ptr [EBP-0x9]      ; arg4: dstCol
0063ee01: 51              PUSH ECX
0063ee02: 8A4DF8          MOV  CL,byte ptr [EBP-0x8]      ; arg3: dstRow
0063ee05: 8A5309          MOV  DL,byte ptr [EBX+0x9]      ; arg2: srcCol
0063ee08: 8A4308          MOV  AL,byte ptr [EBX+0x8]      ; arg1: srcRow
0063ee0b: E8F6BE1400      CALL 0x00788e06                 ; gửi C->S 0x32 02 (6 byte)
```
