# `PROMPT.md` — Prompt nghiên cứu module "KỸ NĂNG (Skills)" của client `aLogin.exe`

> **Cách dùng:** copy toàn bộ khối trong ```` ```` ```` bên dưới và dán cho agent AI
> (kèm đường dẫn tới thư mục `client_skills_handler/`).

---

````markdown
# NHIỆM VỤ — Phân tích nghiệp vụ module "KỸ NĂNG (Skills)" của client aLogin.exe (TS Online)

Bạn là chuyên gia reverse-engineering. Nhiệm vụ của bạn là **đọc mã nguồn C/ASM/HEX đã
dump từ Ghidra** của client `aLogin.exe` (client game TS Online / Thiên Sơn Online, viết
bằng Delphi 7, PE x86, image base `0x00400000`) và **tái tạo lại toàn bộ nghiệp vụ
"kỹ năng"** phía client, để có thể lập trình phía Server có thể giao tiếp đúng với client.

---

## 0. Dữ liệu đầu vào

Bundle mã nguồn nằm ở: **`client_skills_handler/`**

| Đường dẫn | Nội dung |
|---|---|
| `client_skills_handler/*.c`, `*.asm.txt` | pseudo-C (Ghidra) + assembly của các hàm liên quan kỹ năng |
| `client_skills_handler/case_XXX_*.c` | 66 file — bảng tra 65 Main OP của dispatcher S→C |
| `client_skills_handler/asm_*.asm.txt` | các khối ASM chuyên biệt (cây kỹ năng, accessor, builder `0x1C`…) |
| `client_skills_handler/cs_*.hex`, `cs_*.asm.txt` | bảng dispatch / builder C→S theo từng opcode |
| `client_skills_handler/jumptable_*.hex` | bảng tra Main OP ↔ hàm xử lý (S→C) |
| `client_skills_handler/skilltree_94b440*.hex`, `skillid2index_9ae128.hex` | ⚠ **rỗng (readable 0)** — xem `MISSING_DUMPS.md` |
| `client_skills_handler/data/*.tsv` | **11 bảng dữ liệu đã giải mã** (catalog kỹ năng, cây kỹ năng sống, nhánh nghề, Quang/Ám…) |
| ⭐ `client_skills_handler/data/viscii_strings.tsv` | **22 138 chuỗi VISCII** toàn bộ `.text` (kèm VA + hex) — dùng để tra caption |
| ⭐ `client_skills_handler/data/skill_ui_captions.tsv` | **1 475 chuỗi** trong 12 vùng module kỹ năng |
| ⭐ `client_skills_handler/data/dfm_properties.tsv` | bảng phẳng property của **6 form DFM** (chuỗi **Big5**) |
| ⭐ `client_skills_handler/data/cs_dispatch_map.tsv` | **200 dòng** — bảng dispatch S→C đầy đủ của `FUN_0077F414` (byte map `0x77F474` → dword `0x77F53C`) |
| ⭐ `client_skills_handler/reference/rsrc/` | dump **TOÀN BỘ** section `.rsrc` (83 968 B) + cây DFM đã parse |
| ⭐ `client_skills_handler/reference/data_files/` | ★★ **file dữ liệu GỐC**: `Skill.Dat` (⭐ **bảng CLIENT thật — id trên wire**, 352 record) + `skills.json` (bảng SERVER, 376 record) + `README.md` giải thích 2 không gian id |
| `client_skills_handler/tools/*.py` | ⭐ **9 script Python** (giải mã dữ liệu, dựng lại `Skill.Dat`, map id, validate manifest, **parse DFM**, **trích chuỗi VISCII**, **đồng bộ round**) |
| `client_skills_handler/SKILL_DATA_MODEL.md` | ★★★ mô hình dữ liệu đã chốt sơ bộ (đọc trước) |
| `client_skills_handler/WIRE_FORMATS.md` | ★★ khung wire đã đối chiếu byte |
| `client_skills_handler/README.md` | bản đồ opcode ↔ hàm (tiền phân tích) |
| `client_skills_handler/ROUND7_RESULTS.md` | ★★★ kết quả đợt dump Round-7 (ok=250 fail=0) + toàn bộ phát hiện mới |
| `client_skills_handler/ROUND8_RESULTS.md` | ★★★ kết quả đợt dump Round-8 (ok=127 fail=0) + toàn bộ phát hiện mới |
| `client_skills_handler/ROUND9_RESULTS.md` | ★★★ kết quả đợt dump Round-9 (ok=37 fail=0) + **LAYOUT `0x1C 05` ĐÃ CHỐT** |
| `client_skills_handler/ROUND10_RESULTS.md` | ★★★ kết quả đợt dump Round-10 (ok=162 fail=0) + **SỬA LẦM `0x1C 02`** |
| `client_skills_handler/ROUND11_RESULTS.md` | ⭐★★★ kết quả đợt dump Round-11 (ok=191 fail=0) + **`.rsrc`/DFM đã giải mã** + **form kỹ năng nạp từ file ngoài** + **đóng item `TInterForm+0x152/0x154`** |
| `client_skills_handler/ROUND12_RESULTS.md` | ⭐★★★ kết quả đợt dump Round-12 (ok=100 fail=0) + **ĐÓNG "caption tab 5"** + **4 hàm dựng cây nhánh nghề** + **bảng dispatch S→C đầy đủ** |
| `client_skills_handler/BLOCKED_DUMPS.md` | ★★ vì sao 5 file hex rỗng + cách khôi phục bảng cây kỹ năng |
| `client_skills_handler/MISSING_DUMPS.md` | ★ danh sách source còn thiếu + manifest Ghidra đề xuất |
| `client_skills_handler/Ghidra_Skills_Round7_Redump_Manifest.txt` | manifest dump đã chạy (Round-7) |
| `client_skills_handler/Ghidra_Skills_Round8_Redump_Manifest.txt` | manifest dump Round-8 (67 lệnh, đã chạy) |
| `client_skills_handler/Ghidra_Skills_Round9_Redump_Manifest.txt` | manifest dump Round-9 (31 lệnh, đã chạy) |
| `client_skills_handler/Ghidra_Skills_Round10_Redump_Manifest.txt` | manifest dump Round-10 (52 lệnh, đã chạy) |
| `client_skills_handler/Ghidra_Skills_Round11_Redump_Manifest.txt` | ★★ manifest Round-11 (62 lệnh, **đã chạy: ok=191 fail=0**) |
| `client_skills_handler/Ghidra_Skills_Round12_Redump_Manifest.txt` | ★★ manifest Round-12 (20 lệnh, **đã chạy: ok=100 fail=0**) |
| `client_skills_handler/Ghidra_Skills_Round13_Redump_Manifest.txt` | ★ manifest Round-13 kế tiếp (13 lệnh, validate 0 lỗi) |
| `client_skills_handler/reference/analysis_docs/` | 5 tài liệu phân tích cũ của repo — **coi là giả định, phải kiểm chứng lại** |
| `client_skills_handler/reference/C#_server/` | server C# tham chiếu để đối chiếu nghiệp vụ |

### Quy tắc bắt buộc về bằng chứng

1. **Mọi claim phải kèm mức độ**: ✅ VERIFIED (đã đối chiếu byte ASM/HEX) ·
   ⚠️ INFERRED (suy luận từ ≥ 2 nguồn) · ❓ OPEN (chưa đóng).
2. **Mọi claim phải kèm trích dẫn**: địa chỉ + dòng ASM (hoặc offset byte) + tên file.
3. **Khi `.c` và `.asm.txt` của cùng hàm mâu thuẫn → `.asm.txt` mới đúng.**
4. **Không được bịa.** Claim chưa đóng thì ghi ❓ OPEN + đề xuất cách đóng.
5. **Không đọc tuần tự toàn bộ corpus.** Dùng header `Callers:` / `Callees:` /
   `References to entry:` trong header file `.c` để dựng call-graph.

---

## 1. Bối cảnh đã biết (đọc `SKILL_DATA_MODEL.md` + `WIRE_FORMATS.md` trước khi bắt đầu)

### 1.1 ✅ HAI KHÔNG GIAN ID KỸ NĂNG — ĐÃ CHỐT (và đã được dựng lại cho khớp)

| | `Data/Skill.Dat` ⭐ **CANONICAL** | `Data/skills.json` | `Skill_legacy_352.Dat` (build cũ) |
|---|---|---|---|
| Số record | **376** | **376** | 352 |
| Dải id | `10000`…`23023` | `10000`…`23023` | `9980`…`141xx` (lệch ±16/±48) |

**Bằng chứng client dùng không gian `skills.json`**: hàm `FUN_006538BC`
(`006538bc_FUN_006538bc.asm.txt`) hard-code các id
`{10020, 11020, 12020, 13019, 14003…14008, 14016…14021, 14023, 14025, 14026, 14027, 14028,
14032…14052}` — **100 % tồn tại trong `skills.json`**. C# Server (`TSCharacter.cs ::
rebornChar()`, `setallskillForJob()`, `AddSkillrb2()`) cũng dùng đúng không gian này.

⭐ **2026-10-22: `Data/Skill.Dat` đã được DỰNG LẠI từ `skills.json`** ⇒ **map identity**:
* codec round-trip **byte-exact 352/352** trên file gốc,
* **33/33** id hard-code của `aLogin.exe` resolve đúng tên,
* cây `require_sk`: **217 gốc, 0 node trôi**.

> ⚠ **Việc còn lại của bạn:** *bác bỏ* hoặc *xác nhận* bằng chứng trên (đọc lại
> `tools/build_skill_dat.py` + `client_skills_handler/SKILL_DATA_MODEL.md` §2.2a),
> và **kiểm tra lại** xem còn kỹ năng nào của `skills.json` **thiếu** trong cây client
> (⇒ id nào client hard-code riêng, không có trong file).

### 1.2 Wire format đã chốt (xem `WIRE_FORMATS.md` để có chi tiết đầy đủ)

* Khung: `[F4 44][Len u16 LE][Payload]`, XOR `0xAD` toàn khung.
* `Payload[0]` = Main OP. S→C: `RP[i] = Payload[i+1]`; `_LStrCopy(RP,k,n)` = `Payload[k..k+n-1]`.
* C→S: `ShortString` → `@PStrNCat(cap=2)` → `@LStrFromString` → `@LStrCat` → `CY_AddSedQueue`.

| Frame | Hướng | Handler | Ghi chú |
|---|---|---|---|
| `05 03` | S→C | `FUN_0072E9A0` | header **113 byte** + tail **N × 3 byte** `{skillId u16 LE, level u8}`, `N = (len-0x71)/3` |
| `08 01` prop `0x6E` | S→C | `FUN_00710AB0` | 12 byte `{deltaType, value1 u32, value2 u32}` |
| `08 05` | S→C | `FUN_0074C340` | 4 slot kỹ năng tàu |
| `19 0B` | S→C | `FUN_0075B8E0` | **xoá toàn bộ 3000 ô kỹ năng** |
| `20 01` / `20 02` | S→C | `FUN_0079285A` | ⚠ **KHÔNG phải `0x1C`** (đính chính tài liệu cũ) |
| `0x1C 01/02/04/05/06` | C→S | `FUN_0077F414` @ `0x785DD7/0x785E45/0x785EB3/0x785F21/0x785F8F` | payload = **AnsiString** biến động |
| `0x1B 01/02` | C→S | `0x786011` / `0x7860C8` | ⭐ Round-11: `{ 0x1B }{ 0x01 }{ npcIndex u8 }{ npcId u16 LE }` — nguồn: `TInterForm+0x152` / `+0x154` |
| `0x32 01` | C→S | `0x788C38` | ⚠ **code chết** — client không gửi skill id khi dùng kỹ năng |

### 1.2b ⭐⭐ LAYOUT `0x1C 01` — ĐÃ CHỐT (Round-7)

✅ **VERIFIED** — builder `FUN_005990E0` (file `005990e0_FUN_005990e0.asm.txt`):

```asm
005991df: MOV EDX,dword ptr [EAX + 0xE4]     ; skillId
005991f9: CALL 0x0077eb1c                    ; -> 2 byte skillId LITTLE-ENDIAN
00599212: MOV EDX,dword ptr [EAX + 0x108]    ; level hiện tại
00599222: ADD EDX,0x1                        ; LEVEL MỤC TIÊU = level + 1
0059923d: CALL 0x004040d4                    ; @LStrCat3(dest, skillId, level)
00599249: MOV CL,0x1                         ; sub = 01
0059924b: MOV DL,0x1c                        ; main = 1C
```

> ### **Payload `0x1C 01` = `{ skillId : u16 LITTLE-ENDIAN }{ targetLevel : u8 }` — 3 byte, ID TRƯỚC VALUE**
> ⇒ **G-05 ĐÓNG** — claim cũ "client gửi `[level u8][skillId u16]`" là SAI.

Kèm theo (cùng Round-7):

| Frame | Layout | Mức |
|---|---|:-:|
| `0x1C 01` | `{ skillId u16 LE }{ targetLevel u8 }` | ✅ |
| `0x1C 02` | `{ count u8 }{ skillId u16 LE }{ targetLevel u8 }` | ⚠ INFERRED |
| `0x1C 06` | `{ skillId u16 LE }` | ✅ |
| `0x1C 05` | ✅ **ĐÃ CHỐT** (Round-9): `[1C][05]` + `u32 L1` + `L1 byte` (ball 1..12) + `u32 L2` + `L2×3{u16 skillId, u8 level}` + `u32 L3` + `L3 byte` (`{u8 slot 6..13, u16 skillId}`) | ✅ khớp 100% C# `setSkillRb2()` |

**Gate 28 id gốc hệ** (`FUN_005990E0`): id ∈ {10020-10026, 11020-11026, 12020-12026,
13021-13027} mà `player+0x4B0 == 0` ⇒ **không gửi**, hiện thông báo
*"Đẳng cấp vẫn chưa học chuyển sinh"*.

⭐ **`gvar_007D9E68` = form `TSe_SkillTreeForm`** (RTTI field table `0x577A00`: field `published`
ở `+0x25C`, `+0x288`, `+0x28C`, `+0x290`). Hàm xây payload `0x1C 04/05` = **`FUN_0057C840`**
— ✅ **Round-9 đã dump và đọc xong**: `FUN_0057C840` (1419 B) + `FUN_00583A98` (hàm lắp payload).
⇒ **`0x1C 05` ĐÃ CHỐT byte-chính-xác.**

⭐ **`gvar_007D9E68` = form `TSe_SkillTreeForm`** (RTTI field table `0x577A00`: field `published`
ở `+0x24C`, `+0x25C`, `+0x288`, `+0x28C`, `+0x290`; lớp con `TSe_NPCSkillTreeForm`).

⭐ **Calling convention bộ gửi**: `FUN_0077F414(encoder, opcode=DL, sub=CL)`.

⭐⭐ **ID WIRE = ID TRONG `Data/skills.json`** (canonical). `Data/Skill.Dat` **đã được dựng lại
từ `skills.json`** (2026-10-22) ⇒ **hai file giờ là cùng một dữ liệu** (map identity).
Bằng chứng: `aLogin.exe` hard-code chặn đúng **10020/11020/12020/13019** = 4 "Tâm pháp gốc"
của `skills.json`; **33/33** id hard-code resolve đúng; cây `require_sk` 0 node trôi.

⭐ **4 nhánh nghề — thứ tự client: BÁ – HIỆP – HIỀN – TIÊN** (tab 6/7/8/9 của
`TSe_SkillTreeForm`), khác thứ tự `job` server (Hiệp=1, Bá=2, Hiền=3, Tiên=4).
Id **wire**: Bá `14039-14042`, Hiệp `14043-14046`, Hiền `14047-14050`, Tiên `14051-14054`.

⭐⭐⭐⭐ **ROUND-12 — ĐÃ ĐÓNG "CAPTION TAB 5" VÀ "TÊN 4 NHÁNH"**:
form "Kỹ năng" tạo **5 tab** (đánh số **5..9**) trong vòng lặp `0x5AC7B2`:

```asm
005ac82c: ADD  EAX,0x4                  ; i + 4  →  5,6,7,8,9
005ac836: CALL 0x00409980                ; IntToStr(i+4)
005ac841: MOV  EDX,0x5aca04              ; "btn_02"
005ac846: CALL 0x004040d4                ; LStrCat3 → "5btn_02".."9btn_02"
```

⇒ **"Caption" của tab 5..9 KHÔNG phải text mà là TÊN ẢNH** ⇒ tên 4 nhánh nghề
được vẽ sẵn trong **ảnh** `6btn_02.bmp`…`9btn_02.bmp` ⇒ **về nguyên tắc không thể
rút từ binary**. Chi tiết: [`ROUND12_RESULTS.md`](./ROUND12_RESULTS.md) §2.

⭐⭐⭐ **ROUND-12 — 4 HÀM DỰNG CÂY 4 NHÁNH NGHỀ**: `FUN_00579E80` / `FUN_0057A858` /
`FUN_0057B2D8` / `FUN_0057BD58` — giống hệt nhau, chỉ khác hằng số **node index
0/1/2/3** trong `self->0x24C`. Gọi liên tiếp tại `0x579094..0x5790AC`.

⭐⭐ **ROUND-12 — BẢNG DISPATCH S→C ĐẦY ĐỦ**: byte map `@0x77F474` (200 byte) →
dword table `@0x77F53C`; **17 main-op** được xử lý. Xem
[`data/cs_dispatch_map.tsv`](./data/cs_dispatch_map.tsv).

⭐⭐⭐ **ROUND-11 — ĐÃ ĐÓNG "tên hiển thị 4 nhánh" (nhưng kết luận khác dự đoán)**:
`.rsrc` đã được dump **toàn bộ** + parse bằng `tools/parse_dfm.py` (PE resource +
DFM binary Delphi 7):
* `aLogin.exe` chỉ có **6 form DFM** (`TForm1..TForm4`, `TGmForm`,
  `TDelphiXDXPlayForm`) — **KHÔNG có form kỹ năng nào**.
* Chuỗi trong DFM dùng **Big5 (cp950)**, KHÔNG phải VISCII
  (`ImeName` = `A4 A4 A4 E5 20 28 C1 63 C5 E9 29 20 2D 20 AD DC BE 65`
  → big5 = `中文 (繁體) - 倉頡`).
* `FUN_0057922C` ghép tên `"{N}Form_SkillTree"` (N = 1..7) rồi gọi `FUN_007C9B38`
  (factory) — factory tìm **`<tên form>.bmp`** trong thư mục data client.
⇒ **Tên 4 nhánh + caption tab 5 KHÔNG có trong `aLogin.exe`** ⇒ cần thư mục data
gốc của client. Chi tiết: [`ROUND11_RESULTS.md`](./ROUND11_RESULTS.md) §2.

⭐⭐ **ROUND-11 — `TInterForm+0x152/+0x154/+0x156` ĐÃ ĐÓNG**:
`+0x152` = **index NPC trong danh sách** (u8, 0..255) · `+0x154` = **mã NPC**
(u16, 1..1250) · `+0x156` = cờ "đã chọn NPC" ⇒ payload `0x1B 01` =
`{ 0x1B }{ 0x01 }{ npcIndex u8 }{ npcId u16 LE }`.

⭐ **`skill_rb2` = 8 "bóng" THEO HỆ** (không phải 4 nhánh nghề × 2) — `bool[12] ballList`
+ `ushort[8] skill_rb2`.

⭐⭐ **SỬA LẦM ROUND-7 — `0x1C 02` LÀ "HỌC KỸ NĂNG CHO THÚ"** (Round-10):
byte đầu = **chỉ số thú 1..4**, KHÔNG phải count. `FUN_0071D234(player, X)` = **tìm slot thú**
theo `thú->0x55C` trong `player->0x57C[1..4]`. Khớp 100% C# `pet[data[2]-1].setSkill(...)`.

⭐ **Danh tính form** (đọc `vmtClassName` = VMT−0x2C): `gvar_007D9EB8` = **`TSe_SkillForm`**
(chủ buffer dùng chung `+0x1A8`), `gvar_007D9E68` = **`TSe_SkillTreeForm`**.

**Gate học theo hệ** (`FUN_00598944`): *"Thuộc tính Hỏa không thể học kỹ năng Địa hệ"*,
*"Thuộc tính Phong không thể học kỹ năng Thủy hệ"*, *"Thuộc tính Địa không thể học kỹ năng
Hỏa hệ"*, *"Thuộc tính Thủy không thể học kỹ năng Phong hệ"* ⇒ chặn **Địa↔Hỏa**, **Thủy↔Phong**;
**Tâm không bị chặn**.

**Hệ "Đẳng cấp" / nâng cấp**: *"Điểm học được"* ≠ *"Điểm thăng cấp"*; format string
*"Đẳng cấp từ %d nâng lên %d"*; skill *"Chuyển sinh chuyên dụng"*.
Chi tiết: [`ROUND7_RESULTS.md`](./ROUND7_RESULTS.md) + `SKILL_DATA_MODEL.md` §10.

### 1.3 ⭐ Bảng cây kỹ năng — ĐÃ ĐƯỢC ĐÍNH CHÍNH (không còn là "BLOCKER")

⚠ **Tài liệu cũ nói sai.** Đã kiểm chứng bằng header section của
`client_delphi/aLogin.exe`:

```
BSS   VA 0x003DB000  VSZ 0x002368CD  RA 0x003D9000  RSZ 0x00000000   <-- UNINITIALISED
```

* `0x0094B440` → RVA `0x54B440` → **BSS** ⇒ **không có byte nào trong file** ⇒ các file hex
  "rỗng" là **đúng bản chất**, không phải thiếu sót.
* Bảng được **dựng lúc runtime** bởi `FUN_005A0388` (gọi ở **cuối** parser `05 03`):
  `FillChar(0x94B440, 765, 0)` → đăng ký 4 gốc hard-code `10001 / 11001 / 12001 / 13001`
  → đăng ký **từng kỹ năng người chơi đã học** qua `FUN_00597954`.
* `FUN_00597954`: **tab = `elem - 1`** (Địa=0, Thủy=1, Hỏa=2, Phong=3, **Tâm=4**) và
  **byte thứ 3 của mỗi ô là LEVEL của kỹ năng** (KHÔNG phải "byte yêu cầu").
* Bảng tra ngược `0x009AE128` cũng ở BSS ⇒ do `FUN_00759B2C` dựng từ `Data\Skill.Dat`
  (3000 record) lúc runtime.

⇒ Đã khôi phục cấu trúc: `data/skill_tree_tabs.tsv` (255 dòng) +
`data/skill_tree_recovered.bin` (765 byte) — xem `tools/recover_skill_tree.py`.

**Việc còn lại của bạn:**
1. Xác nhận lại chuỗi lệnh `FUN_005A0388` và `FUN_00597954` (file `005a0388_*`, `00597954_*`).
2. **Chứng minh/thoả bác** rằng **thứ tự skill trong tail `05 03` KHÔNG ảnh hưởng** vị trí ô
   (ô được xếp theo **thứ tự record trong `Skill.Dat`**, không theo thứ tự wire) — hay ngược lại.
3. Suy thêm các quan hệ **cha → con** ngoài 5 mốc hard-code
   (10024→10023, 12024→12022/12025, 12026→12025, 13021→13020, 13024→13023)
   từ `require_sk` của `Skill.Dat`.
4. Giải nghĩa 3 bảng `0x948DF8` / `0x9490B0` / `0x98BE04` (xác minh section).

---

## 2. Phạm vi nghiên cứu — 6 nhóm (theo thứ tự ưu tiên)

### NHÓM A ⭐ — Kỹ năng sống hệ Tâm (TOÀN BỘ, không bỏ sót)

Người dùng yêu cầu **tất cả** kỹ năng sống hệ Tâm. Trong `data/skill_life_tam.tsv` và
`data/skill_life_tam_tree.tsv` đã có 38/62 kỹ năng (id server 14001-14037 + 4 nhánh nghề).

**Hãy trả lời đầy đủ:**
1. Với **TỪNG** kỹ năng sống (id, tên): client dùng nó ở đâu? (biến UI, NPC, item, script,
   battle, effect…) — dẫn chứng bằng hằng số id trong ASM (grep `0x36xx` / `0x37xx`).
2. **Đào khoáng (khoáng/thâm khoáng)** — client có xử lý riêng không? Có UI riêng không?
   Có item nhận được không? Tra `Item.dat` (`skill_link`), `Npc.dat` (`skill_link`),
   `eve.emg` (`GetAddskill`).
3. **Tinh luyện / luyện đơn / đúc luyện / chế tạo** — tra `Data/Compound.Dat`
   (công thức chế tạo) và `BlissBag.Dat`.
4. **Cưỡi ngựa (kỵ mã)** — tra `Data/Mounts.dat` + `MountsGrow.dat`
   . Client có đọc 2 file này không? Ở đâu?
   (`THorse.Create` tại `0x00720984`.)
5. **Triệu gọi (summon)** — skill id nào? (`14026` "Triệu gọi" server / `14074` "Triệu Linh
   Thuật" client; `Gọi Bạch Hổ` / `Gọi Huyền Vũ` cho Quang/Ám.) Client có class quản lý
   summoned entity không? (grep `Summon`, `Gọi`, `call`, `pet`, `npc` trong `FUN_0073D558`,
   `FUN_00716A94`.)
6. **Dung thọ (nấu ăn)** — không thấy trong `Skill.Dat`. Hãy xác minh: có skill nào tương
   ứng không? Nếu không có, nói rõ và tra các hệ thống thay thế.
7. **Mua bán / giao dịch / tịnh toạ / khai thương / thiền định** — liên quan module
   economy; tra xem client có hiện/giảm skill nào khi giao dịch.

**Kết quả mong đợi:** 1 bảng TSV `data/life_skill_usage.tsv` — mỗi dòng = 1 kỹ năng sống,
cột: `id_server, id_client, name, client_code_refs (danh sách địa chỉ hàm), UI_related,
data_file_refs, note`.

### NHÓM B ⭐ — Kỹ năng nghề sau chuyển sinh / tái sinh (Bá / Tiên / Hiền / Hiệp)

Đã xác minh 4 nhánh (server C# `setallskillForJob()`), xem `data/job_skill_branches.tsv`:

| `job` | nhánh | 4 kỹ năng |
|:-:|---|---|
| 1 | **Hiệp** | 14043, 14044, 14045, 14046 |
| 2 | **Bá** | 14039, 14040, 14041, 14042 |
| 3 | **Hiền** | 14047, 14048, 14049, 14050 |
| 4 | **Tiên** | 14051, 14052, 14053, 14054 |

(gốc chung: **14038 "Đại chu thiên"**)

**Hãy trả lời:**
1. **Tên hiển thị** của 4 nhánh nghề trong client (UI chọn nghề lúc chuyển sinh). Tìm form
   chọn nghề (grep `job`, `Reborn`, `ChuyenSinh`, `Transmigrat` trong
   `FUN_0075AF6C`, `FUN_0075AFF4`, `FUN_0075B3EC`, `FUN_0075BF7C`, `gvar_007DA0F8`
   = `TBK_AttrRegressForm`, `gvar_007DA1D8` = `TBK_SkillRegressForm`).
2. **Client có tự quyết định nhánh nghề không, hay server báo?** (đọc `05 03` / `08 01`:
   trường nào mang `job`?)
3. **"Bóng" (ball) & `skill_rb2`** — 8 slot theo hệ Kim/Thủy/Hỏa/Phong
   (`data/skill_rb2_balls.tsv`). Client có UI chọn/nâng "bóng" không? Frame nào?
4. **Layout payload `0x1C 05`** (tái sinh lần 2) — đã biết layout phía server C#
   (`TSCharacter.cs :: setSkillRb2`). Hãy **chốt layout phía client** bằng cách đọc vùng
   `0x005B347D–0x005B37C7` (nếu chưa có thì dùng `MISSING_DUMPS.md` §1 để đề xuất
   manifest) và đối chiếu với layout C#. Nếu không đóng được → ghi ❓ OPEN + phân tích
   xung đột.
5. **Reset / tẩy skill (respec)** — C# có `resetHandler.cs` (opcode `0x2A`), client có
   form `TBK_SkillRegressForm` (`gvar_007DA1D8`) và `TLH_ChildSkillDelForm`
   (`gvar_007D9E3C`). Frame nào? C→S lẫn S→C.

### NHÓM C ⭐ — Kích hoạt & học kỹ năng Quang/Ám (Light/Dark)

* Quang = `type 18` (22 skill), Ám = `type 19` (23 skill) — **không phải `elem`**;
  cả hai đều `elem = 0`. Bảng đầy đủ: `data/skill_quang_am.tsv`.
* Tài liệu cũ `reference/analysis_docs/light_dark_skill_unlock_gap.md` đã phân tích
  khá nhiều nhưng **có claim sai** (xem `WIRE_FORMATS.md` §1 — `1C 01/02` S→C thực ra là
  `0x20 01/02`). Hãy **kiểm chứng lại từ đầu**, đừng tin.

**Hãy trả lời:**
1. **"Kích hoạt" Quang/Ám diễn ra thế nào?** Client có gửi frame riêng không, hay server
   chỉ cần gửi skill id đúng?
2. **`0x36B3` (14003) là gì?** Client so sánh `gself+0xFD4+idx*3 == 0x36B3` ở
   `0x0072F428` (`FUN_0072E9A0`) và `0x007117A8` (`FUN_00710AB0`), rồi bật cờ
   `gvar_007DA108+0x13C` và `gvar_007DA530+0x1B8`. Hãy truy: `gvar_007DA108` là form
   nào (grep trong `0051189c_FUN_0051189c.c` — `Application.CreateForm` block), `+0x13C`
   là cờ gì, `+0x1B8` là gì, và cờ bị xoá ở đâu (`0x0072F328`).
3. **Quang/Ám có bị gate theo hệ Kim/Thủy/Hỏa/Phong không?** (vì `elem=0` thì không —
   hãy xác nhận bằng `FUN_0076D578`).
4. **Skill type 17** = "bắt buộc phải học" — `FUN_0075AAAC` trả `type`, `FUN_006538BC`
   so `type == 17`. Liệt kê 5 skill type 17 (id, tên) và giải thích gate.
5. **Tìm hiểu vì sao `Data/Skill.Dat` không khớp client** — client có đọc file `Skill.Dat`
   lúc nào (`FUN_00759B2C`), đường dẫn/tên file là gì (hex literal trong `.text`),
   và client build này ghép với file `Skill.Dat` nào.

### NHÓM D ⭐ — Kỹ năng triệu gọi (summon) theo hệ Thủy/Hỏa/Địa/Phong

* Skill: **14026 "Triệu gọi"** (server) / **14074 "Triệu Linh Thuật"** (client `Skill.Dat`),
  `elem = 5`, `max_lvl = 1`, `sk_point = 0`.
* Riêng Quang/Ám: "Gọi Bạch Hổ" (22011-22014) và "Gọi Huyền Vũ" (23011-23014).
* `FUN_0073D558` kiểm tra id `0x36C7` (13959) và `0x36D2` (14034) khi click NPC.

**Hãy trả lời:**
1. **Client có class/entity "summon" không?** Grep `Summon`, `Gọi`, `Call`, `Spirit`, `Ghost`,
   `Vong`, `Mat`, `Element` trong toàn bộ `*.c` của bundle + `client_pseudo_c/`.
2. **Flow triệu gọi**: click NPC / dùng item (`17 0F` — server trả `F44408000505` + npc id,
   xem `src/server/handlers/use_item/misc.rs:21`) / dùng skill trong trận?
   Client nhận entity triệu gọi bằng frame nào (`03` actor-appear? `05 05`?)?
3. **4 hệ Thủy/Hỏa/Địa/Phong** — phân biệt ở đâu (màu, `elem` của entity, NPC id,
   `Npc.dat`)? Tra `Npc.dat` (`skill_link`), `Data/Mounts.dat`.
4. **"Bạch Hổ" / "Huyền Vũ"** (Quang/Ám) — có entity tương ứng trong `Npc.dat`?
5. Cách thăng cấp cho các skill Triệu gọi ?

### NHÓM E — Nâng cấp kỹ năng / học skill / điểm kỹ năng / respec

1. **Payload `0x1C 01`** — chính xác là chuỗi gì? Đọc `gvar_007D9EB8 + 0x1A8` (dialog
   `FUN_005980A0`, form nào?) và vùng `0x005987DB–0x005999F7` (hole — xem `MISSING_DUMPS.md`).
   Chốt: là **chuỗi text** hay **record nhị phân**? Nếu nhị phân thì layout là gì
   (`{level u8, skillId u16}` như builder `0x1B 01`, hay `{skillId u16, level u8}` như C#)?
2. **Điểm kỹ năng**: `player+0x3EC` vs `player+0x400` — cái nào là "điểm học" và cái nào
   là "điểm dự trữ"? Đối chiếu `FUN_0059F090` (so `+0x3EC` với `TFSkill+0x1C`) và
   `FUN_00710AB0` (prop `0x25` ghi `+0x400`), cùng bảng offset `05 03`
   (`payload[0x05..06]→+0x3EC`, `payload[0x18..19]→+0x400`).
3. **Quy tắc giá điểm khi học** — C# `rebornChar()` dùng:
   `elem==5 hoặc elem==0 → giá gốc; elem khác hệ → ×2; nâng từ Lv1 lên LvN → + (N-1)`.
   Xác nhận phía client (client có tự tính không hay chỉ server tính?).
4. **Nâng cấp vs học mới** — client phân biệt bằng gì (level hiện tại = 0)?
5. **Respec / tẩy skill** — `TBK_SkillRegressForm` (`gvar_007DA1D8`), `TBK_AttrRegressForm`
   (`gvar_007DA0F8`), `TLH_ChildSkillDelForm` (`gvar_007D9E3C`), opcode `0x2A` (C#
   `resetHandler.cs`). Chốt frame C→S + S→C.
6. **Học skill từ sách** (`17 0F` → `Item.dat.skill_link`) — client có gửi `08 01 0x6E`
   để cập nhật không, hay chờ server?

### NHÓM F — Skill bar / hotkey / battle

1. **Thanh kỹ năng (hotkey)** — `0x28` / `0x29` C→S; `FUN_0075AF6C`, `FUN_0075AFF4`,
   `FUN_0075B3EC`, `FUN_0075BF7C`, `FUN_0075B8E0` (S→C `0x19` sub 01/02/03/0A/0B).
   Chốt layout + ý nghĩa từng sub.
2. **Dùng kỹ năng trong trận** — builder `0x32 01` là **code chết**. Vậy client gửi gì?
   (`0x0B 02/03/04` — không mang skill id.) Server phải tự suy ra skill. Xác nhận điều này
   và cho biết client **hiển thị** skill nào khi chọn (UI `unit+0x5A8`).
3. **`FUN_006538BC`** — hàm kiểm tra "đã học" + bảng nhảy 20 entry tại `0x006539D7`.
   Giải mã **toàn bộ** bảng nhảy (dùng `MISSING_DUMPS.md` §4 #14) và lập bảng
   `id → nhánh xử lý → thông báo`. Giải VISCII 3 chuỗi `0x00653E28`, `0x00653E44`,
   `0x00653EB4`.

---

## 3. Câu hỏi bắt buộc phải trả lời (checklist)

| # | Câu hỏi |
|:-:|---|
| 1 | Client có **gửi** byte hệ/hệ-số/nhóm khi học kỹ năng không? (Quang/Ám/Tâm) |
| 2 | ~~Layout byte **chính xác** của gói học/nâng cấp kỹ năng `0x1C 01` là gì?~~ | ✅ **ĐÃ CHỐT** (Round-7) |
| 3 | Layout byte **chính xác** của `0x1C 05` — ✅ **ĐÃ CHỐT** (Round-9), xem [`WIRE_FORMATS.md` §6.5](./WIRE_FORMATS.md) |
| 3b | `0x1C 02` là "học nhiều kỹ năng" hay "học kỹ năng cho THÚ"? — ✅ **HỌC CHO THÚ** (Round-10), xem [`WIRE_FORMATS.md` §6.2](./WIRE_FORMATS.md) |
| 4 | ~~Layout byte **chính xác** của `0x1B 01` là gì và nghiệp vụ là gì?~~ | ✅ **ĐÃ CHỐT (Round-11)**: `{ 0x1B }{ 0x01 }{ npcIndex u8 }{ npcId u16 LE }` — nguồn `TInterForm+0x152/+0x154` |
| 5 | `0x36B3` / `0x36C7` / `0x36D2` là gì — chúng là skill id thật, sentinel, hay flag? |
| 6 | Đã giải quyết xong: `0x94B440` / `0x9AE128` nằm ở **BSS**, dựng runtime. Câu hỏi còn lại: **thứ tự skill trong tail `05 03` có quyết định vị trí ô trong cây không?** (xem §1.3) |
| 7 | `player+0x3EC` và `player+0x400` — ý nghĩa chính xác? (Round-7 cho thấy có **2 loại điểm**: "Điểm học được" và "Điểm thăng cấp") |
| 8 | Client có **tính giá điểm học** không, hay server tính hết? |
| 9 | 4 nhánh nghề Bá/Tiên/Hiền/Hiệp — **tên hiển thị** trong client là gì? (gợi ý: skill *"Chuyển sinh chuyên dụng"* tại `0x5A8CE4` trong `FUN_005A7970`) | ⚠ **Round-12 ĐÃ ĐÓNG**: nhãn là **tên ảnh** `"6btn_02"`..`"9btn_02"` ⇒ nằm trong **ảnh** client, **không có trong binary**. Hãy **suy ra từ phía server** (`job` 1..4 + thứ tự client Bá→Hiệp→Hiền→Tiên) |
| 10 | "Triệu gọi theo hệ Thủy/Hỏa/Địa/Phong" nằm ở đâu trong client? |
| 11 | "Dung thọ" (nấu ăn) có tồn tại không? Nếu không thì hệ thống nào thay thế? |
| 12 | Frame nào xoá toàn bộ danh sách kỹ năng phía client (`19 0B`)? Khi nào server cần gửi? |
| 13 | Có frame S→C nào **khác** mà bản đồ opcode hiện tại đã bỏ sót không? (tra lại toàn bộ 65 case) |

---

## 4. Yêu cầu về sản phẩm bàn giao

### 4.1 File báo cáo chính

Tạo **MỘT** file tổng hợp, định dạng **Markdown** (hoặc HTML nếu bạn thích — nhưng phải
mở được bằng trình duyệt và tải về trực tiếp), tên:

```
client_skills_handler/ANALYSIS_SKILLS.md
```

**Nội dung bắt buộc** (tiếng Việt):

1. **Tóm tắt điều hành** — 10-20 bullet, kết luận quan trọng nhất, ưu tiên phía server.
2. **Bản đồ opcode ↔ hàm ↔ file** — đầy đủ, C→S và S→C, kèm layout byte.
3. **Bảng wire format** — mỗi frame liên quan kỹ năng: `[opcode][sub]` + định nghĩa từng
   byte + ý nghĩa + mức độ tin cậy.
4. **Mô hình dữ liệu kỹ năng** — cây kỹ năng sống hệ Tâm (toàn bộ), 4 nhánh nghề,
   Quang/Ám, triệu gọi; **kèm id ở CẢ HAI không gian** (server/client).
5. **Bảng "kỹ năng → nơi dùng trong client"** — mỗi kỹ năng sống 1 dòng.
6. **Bảng "kỹ năng nghề → nơi dùng"**.
7. **Quy tắc gate phía client** — đủ để server biết khi nào client sẽ từ chối.
10. **Câu hỏi mở** — ❓ OPEN, kèm đề xuất cách đóng (cần dump Ghidra nào / capture nào).
11. **Phụ lục** — trích dẫn ASM quan trọng (kèm địa chỉ).

### 4.1b File báo cáo bổ sung

| File | Nội dung |
|---|---|
| `client_skills_handler/ROUND8_RESULTS.md` | kết quả đợt dump Round-8 + cập nhật các claim đã đóng |
| `client_skills_handler/ROUND9_RESULTS.md` | kết quả đợt dump Round-9 + layout `0x1C 05` đã chốt |
| `client_skills_handler/ROUND10_RESULTS.md` | kết quả đợt dump Round-10 + **sửa lầm `0x1C 02`** + danh tính form |
| `client_skills_handler/ROUND11_RESULTS.md` | kết quả đợt dump Round-11 + **`.rsrc`/DFM (Big5)** + **form kỹ năng nạp từ file ngoài** + **đóng `TInterForm+0x152/0x154`** |
| `client_skills_handler/ROUND12_RESULTS.md` | kết quả đợt dump Round-12 + **đóng "caption tab 5"** + **4 hàm dựng cây nhánh nghề** + **bảng dispatch S→C đầy đủ** |
| `client_skills_handler/reference/data_files/` | file dữ liệu gốc `Skill.Dat` + `skills.json` (kiểm chứng lại catalog) |

### 4.2 File dữ liệu bàn giao (đặt trong `client_skills_handler/data/`)

| File | Nội dung |
|---|---|
| `skill_id_mapping.tsv` | ánh xạ id server ↔ id client ↔ tên (toàn bộ 376 + 352) |
| `life_skill_usage.tsv` | mỗi kỹ năng sống → nơi dùng trong client (địa chỉ hàm) |
| `job_skill_usage.tsv` | mỗi kỹ năng nghề → nơi dùng |
| `skill_wire_formats.tsv` | mỗi frame → layout byte |
| `skill_client_gates.tsv` | mỗi gate phía client → điều kiện + hàm + địa chỉ |
| `skill_gaplist.tsv` | mỗi gap → mức độ, vị trí, đề xuất |
| `skill_tree_slots.tsv` | vị trí ô thực tế của từng skill trong cây (nếu khôi phục được chính xác) |

### 4.3 Cập nhật `MISSING_DUMPS.md`

Nếu bạn tìm được thêm source (bằng cách đọc thẳng `aLogin.exe` hoặc suy luận từ ASM),
**cập nhật** `client_skills_handler/MISSING_DUMPS.md` với manifest Ghidra mới.

### 4.4 Bổ sung script vào `client_skills_handler/tools/`

Mọi script bạn viết để giải mã/đối chiếu phải đặt vào `client_skills_handler/tools/`,
chạy được bằng `python3 <script>` với **đường dẫn tuyệt đối** (không hardcode path
máy khác), và phải in ra thống kê kiểm chứng.

---

## 5. Nguyên tắc làm việc

1. **Đọc `SKILL_DATA_MODEL.md` + `WIRE_FORMATS.md` trước** — chúng đã chốt ~60 % khối lượng
   kiến thức. Bạn cần **kiểm chứng lại** chứ không cần làm lại từ đầu.
2. **Đi thẳng vào ASM khi cần byte-exact.** `.c` của Ghidra có thể sai.
3. **Ưu tiên giải quyết 2 "hole"** `0x005987DB–0x005999F7` và `0x005B347D–0x005B37C7`
   (chưa có trong bundle) — chúng quyết định layout byte gói học kỹ năng.
   Nếu Ghidra project không map được, hãy **đọc thẳng từ `client_delphi/aLogin.exe`**
   (file PE, image base `0x400000`; section `CODE` RA `0x400`, `DATA` RA `0x3D3800`; dùng
   `python3` + `pefile` hoặc `pe_dump.py` sẵn có ở `/mnt/d/VUDT/GIT_PCC/test/`).
   ⭐ **Mẹo đã dùng và rất hiệu quả:** đọc header section PE để biết một địa chỉ là
   `.text` / `.data` hay **BSS** — nếu là BSS thì **không có trong file** và phải tìm
   hàm dựng nó (xem `BLOCKED_DUMPS.md` §1 — đây là cách đã gỡ "BLOCKER" `0x94B440`).
4. **Đừng dừng lại khi gặp claim mâu thuẫn** — hãy bác bỏ claim cũ và ghi lại.
5. **Ưu tiên trả lời các câu hỏi ở §3** — đó là điều người dùng cần nhất.
6. Nếu không đóng được một mục, ghi rõ ❓ OPEN + lý do + cách đóng, **đừng bịa**.
7. ⭐⭐ **PHÂN BIỆT 2 ENCODING** khi đọc chuỗi:
   * `.text` (chuỗi runtime của game) = **VISCII 1.1** ⇒ dùng `tools/extract_strings.py`
     hoặc `data/viscii_strings.tsv`.
   * `.rsrc` / DFM (nhãn form) = **Big5 cp950** ⇒ dùng `tools/parse_dfm.py`.
   Giải mã nhầm ⇒ ra rác (ví dụ `A4 A4 A4 E5 20 28 C1 63 C5 E9 29 20 2D 20 AD DC BE 65`
   là `中文 (繁體) - 倉頡`, KHÔNG phải `ấấấă (ÁcĂé) - …`).
8. ⭐ **Tra caption bằng công cụ, không đoán địa chỉ**: `data/viscii_strings.tsv` đã có
   **mọi** chuỗi VISCII kèm VA — grep trong đó trước khi mở rộng phạm vi dump.
````

---

## Ghi chú cho người dùng

* **Ưu tiên đọc `client_skills_handler/SKILL_DATA_MODEL.md` và `WIRE_FORMATS.md` TRƯỚC** —
  2 file này đã chốt phần lớn kiến thức nền (kể cả 2 đính chính quan trọng: *hai không gian
  id kỹ năng* và *opcode S→C `0x20` chứ không phải `0x1C`*).
* Bản báo cáo cuối cùng sẽ nằm ở `client_skills_handler/ANALYSIS_SKILLS.md` +
  6 file TSV trong `client_skills_handler/data/`.
