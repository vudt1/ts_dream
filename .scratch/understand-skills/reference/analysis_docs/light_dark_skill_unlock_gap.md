# Luồng Mở Khóa Kỹ Năng Hệ QUANG (Light) / ÁM (Dark) — Phân Tích Client & Gap Server

Ngày: 2026-10-02
Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Nguồn: `client_pseudo_c/` (C+ASM+HEX), `test/ts_decompile/` (13k file), `Data/Skill.Dat`, `Data/Skill2.Dat`,
`TS_Server_Bear/` (server C# tham chiếu), `src/` (Rust server hiện tại)
Tài liệu nền: [`post_login_stats_inventory_flow.md`](./post_login_stats_inventory_flow.md), `battle/03_skill_and_combo.md`

> **Mức độ bằng chứng:** mọi claim ghi `✅ VERIFIED` đã đối chiếu ASM hoặc byte trong file `Data/`.
> Claim ghi `⚠️ INFERRED` là suy luận từ 2 nguồn độc lập. Claim ghi `❓` là chưa đóng.

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — §2.1 SAI về danh sách sub `0x1C`. Xem §0.1**

---

## 0.1 🔴 ĐÍNH CHÍNH SAU ROUND-4 REDUMP

### 0.1.1 §2.1 — ✅ ĐÚNG. Bảng `0x1C` có **7 entry**, không phải 5

> ⚠️ **LƯU Ý ĐỌC LẠI 2026-10-02 (sau khi tìm ra `cs_op1c_dispatch_cases.asm.txt`):**
> manifest Round-4 chỉ xin **5 dword** (`PTRTAB 0x00785DBB 5`), nên file
> `cs1c_785dbb_jmptbl_5xDWORD.hex` **bị cắt cụt** và dễ đọc nhầm thành "bảng 5 entry".
> Bảng thật là **7 entry**. Kết luận cũ trong bản đầu tiên của `§0.1` ("`0x1C` chỉ có
> sub `01/02/04`") **SAI** — đã sửa ở đây.

Dispatch head **đã có sẵn** trong `cs_op1c_dispatch_cases.asm.txt` (window
`0x00785DA6..0x0078620B`) — **không cần redump thêm**:

```asm
00785da6: XOR  EAX,EAX
00785da8: MOV  AL, byte ptr [EBP + -0x6]      ; sub
00785dab: CMP  EAX, 0x6
00785dae: JA   0x0078a4f2                      ; sub > 6 -> KHÔNG gửi
00785db4: JMP  dword ptr [EAX*0x4 + 0x785dbb]  ; sub 0..6
```

`CMP EAX,0x6 / JA` ⇒ bảng **bắt buộc có 7 entry** (`0x00..0x06`). Đọc 28 B từ
`cs_op1c_targets_0x785dbb.hex` (`0x00785DBB..0x00785DD6`):

| Entry | Builder | Nghĩa |
|:-:|:-:|---|
| `[0]` | `0x0078A4F2` | epilogue — **sub `0` KHÔNG gửi** |
| `[1]` | `0x00785DD7` | ✅ gửi |
| `[2]` | `0x00785E45` | ✅ gửi |
| `[3]` | `0x0078A4F2` | epilogue — **sub `3` KHÔNG gửi** |
| `[4]` | `0x00785EB3` | ✅ gửi |
| `[5]` | `0x00785F21` | ✅ gửi |
| `[6]` | `0x00785F8F` | ✅ gửi |

**⇒ `0x1C` có 7 sub hợp lệ (`0..6`), trong đó `0` và `3` là no-op ⇒ 5 sub thật sự gửi:
`01`, `02`, `04`, `05`, `06`.** §2.1 **đã đúng từ đầu**.

### 0.1.1b ⭐ Wire layout 5 sub gửi của `0x1C` — byte-verified

Cả 5 builder dùng cùng một khuôn: `ShortString{main}` → `@PStrNCat` cap=2 (`"1C"`,
**1 byte**) → `@LStrFromString` → `@LStrCat` (`0x00404090`) nối thêm chuỗi → `CY_AddSedQueue`.

| Sub | Builder | Payload = `[1C][sub]` ++ | Field nguồn | Mức |
|:-:|:-:|---|---|:-:|
| `1` | `0x785DD7` | `<LStr>` | `(*gvar_007D9EB8) + 0x1A8` | **A** |
| `2` | `0x785E45` | `<LStr>` | `(*gvar_007D9EB8) + 0x1A8` — **cùng nguồn sub 1** | **A** |
| `4` | `0x785EB3` | `<LStr>` | `(*gvar_007D9E68) + 0x28C` | **A** |
| `5` | `0x785F21` | `<LStr>` | `(*gvar_007D9E68) + 0x290` | **A** |
| `6` | `0x785F8F` | `<LStr>` | `(*gvar_007DA114) + 0xE8` | **A** |

ASM sub 5 (`cs_op1c_dispatch_cases.asm.txt`), bản mẫu:

```asm
00785f21: LEA  EAX,[EBP + -0x34]
00785f24: MOV  DL, byte ptr [EBP + -0x5]     ; main = 0x1C
00785f27: MOV  byte ptr [EAX + 0x1], DL
00785f2a: MOV  byte ptr [EAX], 0x1           ; len ShortString (KHONG phai len wire)
00785f4a: MOV  CL, 0x2
00785f4c: CALL 0x00402b60                    ; @PStrNCat cap=2 -> "1C"  (1 BYTE)
00785f5a: CALL 0x0040402c                    ; @LStrFromString
00785f65: MOV  EDX,[0x007d9e68]
00785f6b: MOV  EDX,[EDX]
00785f6d: MOV  EDX,[EDX + 0x290]            ; field nguon
00785f73: CALL 0x00404090                    ; @LStrCat  -> noi them chuoi
00785f85: CALL 0x0051633c                    ; TForm1_CY_AddSedQueue  -> GUI
```

> **Độ dài thay đổi** theo `LStrLen(field)` — không phải record cố định. Đây là lý do
> §2.4 ghi *"Nếu AnsiString đó đúng là blob record…"* là mức **B/C**: giờ đã xác nhận
> nó **là chuỗi**, nên layout cố định phải suy ra từ dữ liệu, không từ builder.

### 0.1.1c ⚠️ Code chết ngay sau builder sub `6` — đừng đọc nhầm

Ngay sau lệnh gửi của sub `6` là `JMP 0x0078A4F2` (epilogue), rồi tới `0x00785FFD`
xuất hiện một sub-dispatch `DEC AL / JZ / DEC AL / JZ` với **3 khối gửi nữa**:

```asm
00785ff3: CALL 0x0051633c      ; gửi packet
00785ff8: JMP  0x0078a4f2      ; <-- epilogue: KHONG bao gio quay lai
00785ffd: MOV  AL, byte ptr [EBP + -0x6]   ; (khong reachable tu 0x1C sub 6)
00786000: DEC  AL / JZ 0x00786011
00786004: DEC  AL / JZ 0x007860c8
```

⇒ **3 khối đó là dead code đối với `0x1C`** — chúng thuộc builder của opcode kế
(khoảng `0x1D`), nằm giữa bảng `0x1C` và bảng `0x1E` (`0x00786221`).
**Đừng tính 4 gói cho `1C 06`.** Chỉ **1** gói.
> **⚠️ Hệ quả cho G-19:** `skills.rs:288-340` phân tích chi tiết `1C 05` ("skill reborn 2",
> đọc `ball_point`) — **nếu `05` không tồn tại thì toàn bộ nhánh đó là code chết.**

### 0.1.2 Builder `1C 01` đã dump đầy đủ

`asm_cs1c_785dbb_jmptbl.asm.txt` (578 B, 100% coverage) chứa builder `0x00785DD7` (sub 1).
Xác nhận `marriage §0 #4` / `guild R5`: `ShortString{[EBP-5]=0x1C, [EBP-6]=0x01}` →
`_PStrNCat(cap=2)` → `_LStrFromString` → payload `1C 01` + `<AnsiString @gvar_007D9EB8+0x1A8>`.
**Byte `0x01` là độ dài ShortString, không lên wire.** Framing = `[main][sub]`.

### 0.1.3 🔴 BLOCKER — §6 #4 và §6 #5 KHÔNG THỂ LẤY

| Mục | Địa chỉ | Kết quả Round-4 |
|---|:-:|---|
| §6 #4 `TSkillTreeRow` 765 B | `0x0094B440..0x0094B6A9` | ❌ **`readable 0`** — ngoài vùng memory đã map |
| §6 #5 bảng `skillId → index` | `0x009AE128..0x009AE827` | ❌ **`readable 0`** |
| §6 #10 caption tab `0x5A02DC..0x5A039B` | trong `.text` | ✅ ASM đã dump (`asm_5a02dc_5a0387.asm.txt`, 145 B) |

**Biên memory đã map = `0x7DA800`** (đo từ `table_7da6dc_101DWORD.hex`: `0x7DA6DC` + 292 B
= `0x7DA7FF`). Mọi địa chỉ ≥ `0x7DA800` đọc được 0 byte.

Điều này **không phải lỗi script** — Round-3 đã gặp y hệt:
`table_0x948DF8_200.hex`, `table_0x9490B0_200.hex`, `table_0x98BE04_200.hex` đều `readable 0`.

**⇒ Cả 2 mục quan trọng nhất của §6 vẫn chưa lấy được.** Cần kiểm tra
Window → Memory Map trong project Ghidra; nếu không có block phủ `0x94B440` thì phải
re-import `aLogin.exe`. Đây là lý do §6 #4 từng được đánh dấu *"quan trọng nhất"* mà
vẫn chưa ai lấy được.

### 0.1.4 §6 — các mục đã đóng

| # | Mục | Kết quả |
|:-:|---|---|
| 1 | `0x005987DB..0x005999F7` (builder payload dialog kỹ năng) | ⚠️ `SCAN` xong; `FUN_005980A0` đã có, `0x598944` gap 361 B còn lại |
| 2 | `0x005B347D..0x005B37C7` (mẫu append record) | ⚠️ còn 843 B; đã có `asm_tfskill_accessors.asm.txt` (525 B) cho 8 accessor |
| 3 | `CY_AddSedQueue @ 0x0051633C` | ✅ **đã có body** (`0051633c_TForm1.CY_AddSedQueue.c`) |
| 6 | `.rodata` `0x5854E4`/`0x584384`/`LAB_0072A870`/`LAB_0072C665` | ✅ đã dump hex |
| 7 | 5 method còn lại của form kỹ năng | ✅ đã dump (`asm_59c089_*`, `asm_59c61f_*`, `asm_59d750_*`, `asm_59efe3_*`, `asm_5a02dc_*`) |
| 8 | `FUN_0076D578` gate element | ✅ **6 737 B / 6 844** đã dump (`asm_76d578_full.asm.txt`) |
| 9 | `TFSkill`: `0x00759B2C`, `0x00759DF0`, `0x00759A8C`, 8 accessor | ✅ đã có body; 4 accessor còn lại nằm trong `0x0075A948`/`0x0075A9A0` → dùng `asm_tfskill_accessors.asm.txt` |
| 10 | caption 5 tab | ⚠️ ASM có, **còn cần giải VISCII**; 3 tab chưa có `lit_*.hex` |

### 0.1.5 G-26 — chốt offset `skill_type`

`battle/03 §2.1` ghi `skill_type` ở record `+0x21`; ASM `FUN_0075A9F8` dùng hằng `0x25`.
Round-4 đã dump đủ 8 accessor (`asm_tfskill_accessors.asm.txt`, `0x0075A948..0x0075AB54`)
⇒ **đọc trực tiếp file này để chốt**, không cần redump thêm.

---

## 0. Kết luận điều hành

| # | Kết luận | Mức |
|:-:|---|:-:|
| 1 | **QUANG/ÁM không phải là `element`.** Nó là byte `type` trong `Skill.Dat`: **`type = 18` = Quang (22 skill), `type = 19` = Ám (23 skill)**. Cả hai nhóm đều có `elem = 0`. | ✅ |
| 2 | Client **không hề gửi** byte hệ/hệ-số trong gói học kỹ năng. Toàn bộ quyết định thuộc server. | ✅ |
| 3 | Gói C→S học kỹ năng là **`1C 01`** + record 3 byte `[level:1][skillId:2 LE]` × N. | ✅ frame / ⚠️ thứ tự byte |
| 4 | `05 03` parser client là **`FUN_0072E9A0`** (không phải `FUN_0078C7DB`); header **113 byte**, skill list bắt đầu tại `payload[0x71]`, **không có count** — `N = (len − 0x71)/3`. | ✅ |
| 4b | 🔴 **Mới (Round-4):** `0x1C` có **7 sub** (`0..6`) — sub `0` và `3` là epilogue (no-op), **5 sub thật sự gửi: `01`, `02`, `04`, `05`, `06`**. Layout: `[1C][sub]` ++ `@LStrCat` chuỗi tại field nguồn (độ dài **thay đổi**, không phải record cố định). Xem §0.1.1-0.1.1c. | ✅ |
| 5 | **`src/server/spawn.rs:183` ghi 90 byte 0 thay vì 43** → skill list lệch **47 byte**, client không đọc được skill nào. Đây là lỗi chặn tuyệt đối. | ✅ |
| 6 | Client lấy **cây kỹ năng từ bảng hard-code `0x94B440[5 tab][51 slot][3B]`**, chọn tab bằng byte `thuoctinh` trong `05 03`. **Server không bao giờ gửi định nghĩa cây kỹ năng.** | ✅ |
| 7 | Server Bear **không có** hệ Quang/Ám; chỉ có `element` 0..5 với `5` = trung lập. Hệ Quang/Ám phải tự thiết kế. | ✅ |
| 8 | `data.skills` (bảng `handle_player_skill_learn` đọc) **rỗng trong production** → mọi lệnh học kỹ năng hiện đều no-op. | ✅ |

---

## 1. QUANG / ÁM là gì (phát hiện mới, chưa có trong bất kỳ tài liệu nào)

`Data/Skill.Dat` — header 86 byte 0, sau đó **352 record × 86 byte** (`src/data/loaders/skill_pc.rs:6-37`).

Decode: `DecodeItem8(v) = (v ^ 0xFD) − 4`, `DecodeItem16(v) = (v ^ 0x6E90) − 4`.

### 1.1 Phân nhóm theo `type` (offset 21)

| `type` | số record | skill id | Nội dung |
|:-:|:-:|---|---|
| **18** | **22** | `21953–21963`, `22060–22070` | **HỆ QUANG**: Quang mảu thuật, Phật quang chi chưởng, Huyền Quang, Liệt diễm chi quang, Quang dư thuật, Quang chi không, Quang tuyền quán chố, Hồi quang phản chiêu, Quang tức uyên trừng, Kim tễ cương không, **Bạch Hổ** (+3 Gọi tôi/trung/giữa), Tập thức quang ân, Điếu/C.Đ/C.L quang chi oán, Chấn huyễn quang, Siêu quang bá, Quyền khuynh vân lưu |
| **19** | **23** | `22972–22975`, `22988–22995`, `22996–22998`, `22999–23003`, `23017–23019` | **HỆ ÁM**: Huyền Vọng (+3 Gọi tôi/trung/giữa), Vọng trạm, **Hắc ám liên trảm**, Cấm chiếc thiên, Chấn Vọng, Vân chiếc xuyên tâm, Cấm phi luân, Cấm nhân trậm, Cấm mê hồc chiếc trảm, Phiêu diêu lưu phong, Huyển ám luyện trậm… |
| 16 | 30 | `21004–21051` | Nhóm "Thiên Nhân" (tân thủ, phân bố theo 4 hệ Kim/Thủy/Hỏa/Phong) |
| 15 | 20 | — | Nhóm trung cấp |
| 1–5 | 215 | `9984–21014` | Kỹ năng thường theo hệ |

Histogram `type`: `0:1, 1:38, 2:36, 3:42, 4:37, 5:62, 6:3, 7:22, 8:1, 9:2, 10:1, 11:3, 12:2, 13:1, 14:1, 15:20, 16:30, 17:5, 18:22, 19:23`.

### 1.2 Đặc tính nhóm Quang/Ám

| field | offset | Giá trị | Ý nghĩa |
|---|:-:|---|---|
| `type` | 21 | **18 / 19** | nhóm Quang / Ám |
| `elem` | 26 | **0** cho tất cả 45 skill | **không bị gate theo hệ Kim/Thủy/Hỏa/Phong** |
| `sk_point` | 40 | **1 … 18** | giá điểm kỹ năng thật sự |
| `max_lvl` | 42 | 1 / 5 / 10 | |
| `require_sk` | 43 | `48` (sentinel "không có") | **không có tiền đề kỹ năng** |

> `require_sk = 48` xuất hiện ở **205/352** record — kể cả các skill thường. Đây là sentinel "no prereq" của file, không phải skill id thật (skill id hợp lệ bắt đầu từ 9984). ✅ VERIFIED

**Hệ quả:** vì `elem = 0`, nếu Rust nạp được bảng thì `can_learn_element()` (`skills.rs:10-18`) sẽ cho phép mọi người học, và `get_point_skill_add()` sẽ thu **giá gốc** (không nhân đôi) — đúng, nhưng **không có gì chặn người chưa mở khóa hệ Quang/Ám**.

### 1.3 `Data/Skill2.Dat` — bảng mô tả 22 kỹ năng (chưa ai đọc file này)

- Kích thước **1900 byte**; header 74 byte 0; phần còn lại là **stream đóng gói**: `[tên đảo ngược][u8 nameLen][zero-pad][mô tả đảo ngược][u8 desLen][zero-pad]` × 22.
- Chuỗi là **VISCII 1.1, lưu ngược** (giống `DatReader::read_string_bytes` — `src/data/reader.rs:250-269`).
- 22 cặp tên/mô tả. Tên (giải mã tay, mojibake một phần vì bảng VISCII chưa chuẩn hoá ở `0xA2/0xD5/0xDF`):

| # | Tên | Mô tả (rút gọn) |
|:-:|---|---|
| 1 | Thanh Đồng Thiên | "Mũi tên chúa Đao băng cang" |
| 2 | Thanh Thiết Thiên | "Mũi tên chúa Đao tữ thiết" |
| 3 | Thanh Cang Thiên | "Mũi tên chúa Đao băng Cang" |
| 4 | Thanh Hoa Thiên | "Mũi tên bị chết đau đớn…" |
| 5 | Côn Đẩu Cung | "Cung chúa Đao tữ gù" |
| 6 | Lưu Ly Cung | "Cung chúa Đao Tữ Thiết" |
| 7 | Tinh Cang Côn Đẩu Cung | "Côn Đẩu Cung tữ Tinh Cang" |
| 8 | Kim Nham Cung | "Đũa chúa Đao băng đá mài." |
| 9 | Đại Cung | "Đũa băng đá lớn." |
| 10 | Thiết Pháo | "Đũa pháo mài tữ thiết" |
| 11 | Lang Nha Pháo | "Đũa pháo mài tữ Tinh Cang" |
| 12 | Hoa Lưu Đẩu | "Hóa tên tữ rớm, hóa thách." |
| 13 | Chế Hắc Tranh | "Cơn điệu hẹnh học bằng giấy." |
| 14 | Hoa Hắc Tranh | "Cơn điệu hẹnh học bằng da thố." |
| 15 | Cửu Hắc Tranh | "Cơn điệu hẹnh học bằng vải." |
| 16 | Hóa Đẩu Cang | "Cái thau chơa xảy dấu nhỏ" |
| 17 | Hóa Đẩu Thương | "Dấy thương nố bộ thau dấu hóa" |
| 18 | Hóa Đẩu Mộng | "Mộng níu bằng dấy thương" |
| 19 | Pháo Tác | "Mãnh đá cắt dấy thương" |
| 20 | Pháo Liễn Đẩu | "Quạ câu sắt cắt dấy thương" |
| 21 | Tiểu Liễn Đẩu | "Quạ câu cang cắt dấy thiết." |
| 22 | Hồng Liễn Cung | "Cung của hồn vệ cang khâu." |

> ⚠️ **Đây không phải bảng kỹ năng Quang/Ám.** Đây là nhóm **cung/ném tên (Thiên Cung / Tinh Cang / Côn Đẩu / Lưu Ly)** — bao gồm nhánh **"Hắc"** (Chế/Hóa/Cửu Hắc Tranh = "Hắc Tranh", Hắc ám liên trảm) tức **nhánh Ám**. Tuy nhiên các tên này **không xuất hiện trong 352 record `Skill.Dat`** (trừ `Thiết Pháo` #211 và `Cung Tháp` #22060). → `Skill2.Dat` nhiều khả năng là bảng mô tả của **một build client khác** (Delphi server-side, theo `docs/adr/0003-rank-ground-port.md:10,47`).

---

## 2. Luồng Client (đã xác minh)

### 2.1 C→S — yêu cầu học kỹ năng

**Dispatch 2 tầng** (`client_pseudo_c/0077f414_FUN_0077f414.asm.txt:5-35`, ✅ chưa từng được tài liệu hoá):

```
byte[0x77F474 + main]  -> index
dword[0x77F53C + index*4] -> handler
```

| Main | index | handler | Ý nghĩa |
|:-:|:-:|---|---|
| `0x03` | 4 | `0x0077FB48` | xác nhận vào game |
| `0x08` | 8 | `0x0077FF2E` | phân bổ chỉ số |
| `0x17` | 0x15 | `0x007833F0` | túi đồ (61 sub) |
| `0x18` | 0x16 | `0x00785226` | ⚠️ tên file `cs_op17_first_cases.asm.txt` nhầm |
| `0x19` | 0x17 | `0x007853CA` | dispatcher 0x19 (sub 0..0x29) |
| **`0x1C`** | **0x19** | **`0x00785DA6`** | **học kỹ năng** (sub 1/2/4/5/6) |
| `0x1B` | 0x1A | `0x00785FFD` | 1 skill đơn (sub 1/2) |

**Builder `0x1C 0x01`** — `cs_op1c_dispatch_cases.asm.txt:4-51` (target `0x00785DD7`), ✅ VERIFIED:

```
prefix = [01][1C][01][01]                  ; 2 cặp, mỗi cặp = len-prefix 1 + 1 byte
payload = str(gvar_007D9EB8 + 0x1A8)       ; AnsiString dựng sẵn, XÓA SAU khi gửi
TForm1_CY_AddSedQueue(gvar_007DA664, prefix ++ payload)   @ 0x0051633C
```

Wire (sau khi `CY_AddSedQueue` bỏ các byte `0x01`):

```
[F4 44][len LE16][1C][01][ rec × N ]        XOR 0xAD toàn frame
```

**Record 3 byte** — ⚠️ INFERRED (2 nguồn độc lập):

| Nguồn | Bằng chứng |
|---|---|
| Builder anh em `1B 01` @ `0x00786011` (`0077f414.asm.txt:1821-1867`) | `[01][1B][01][01]` + `[01][level@+0x152]` + `FUN_0077EB1C(word@+0x154)` → bỏ prefix = `[1B][01][level:1][skillId_lo][skillId_hi]` — **3 byte, value trước id** |
| Server Bear `ModifySkillHandler.cs:13-18` | `for(i=2;i<len;i+=3) setSkill(read16(data,i), data[i+2])` — **id trước value** ⚠️ **NGƯỢC CHIỀU** |

> ⚠️ **Xung đột chưa đóng.** Bear đọc `[skillId u16][level u8]`; builder `1B 01` của client sinh `[level u8][skillId u16]`.
> Rust hiện đang parse `[skillId u16][target_lv u8]` (`skills.rs:114-120`) — **theo Bear**.
> Cần capture thật hoặc dump builder tại hole `0x005987DB–0x005999F7` để chốt.

**Các sub khác của `0x1C`:**

| Sub | Target | Nguồn payload | UI |
|:-:|---|---|---|
| `01` | `0x00785DD7` | `gvar_007D9EB8 + 0x1A8` | dialog NPC skill-tree (class tại `gvar_007D9EB8`, ctor `FUN_005980A0`) |
| `02` | `0x00785E45` | `gvar_007D9E68 + 0x28C` | skill thú cưng |
| `04` | `0x00785EB3` | `gvar_007D9E68 + 0x28C` | cùng field, sub khác |
| `05` | `0x00785F21` | `gvar_007D9E68 + 0x290` | skill reborn-2 / ball |
| `06` | `0x00785F8F` | `gvar_007DA114 + 0xE8` | `TShipSkillTreeForm` |

**Các UI liên quan** (từ `test/ts_decompile/functions/0051189c_FUN_0051189c.c`):

| gvar | Class | ctor |
|---|---|---|
| `gvar_007D9EB8` | *(unnamed, VMT `PTR_PTR_00589708`)* — **sở hữu payload `1C 01`** | `FUN_005980A0` |
| `gvar_007D9E30` | `TSe_NPCSkillTreeForm` | `FUN_005843AC` |
| `PTR_DAT_007da3d4` | `TSe_ChildSkillTreeForm` | `FUN_00587BA4` |
| `gvar_007DA05C` | `TSe_MiniSkillForm` | `FUN_005AC734` |
| `gvar_007DA1D8` | `TBK_SkillRegressForm` | `FUN_005E4638` |
| `gvar_007DA0F8` | `TBK_AttrRegressForm` | `FUN_005E5E9C` |
| `gvar_007D9E3C` | `TLH_ChildSkillDelForm` | `FUN_005E7A14` |
| `gvar_007DA114` | `TShipSkillTreeForm` (payload `+0xE8`, gửi `1C 06`) | `FUN_005B37C8` |
| `gvar_007DA5A8` | `TInterForm` (`+0x152` level, `+0x154` id, `+0x156` tick) → gửi `1B` | `FUN_005D6048` |

**Chuỗi gọi đầy đủ:**

```
[UI: mở cây kỹ năng NPC]            ← địa chỉ CHƯA dump (hole 0x005987DB–0x005999F7)
   │  (mở qua "follow NPC" → FUN_0079C08C / FUN_0079C0EC / FUN_0079D780 / FUN_0079D96C / FUN_005A0388)
   ▼  dựng string vào gvar_007D9EB8+0x1A8  =  chuỗi record 3 byte
FUN_0077F414(gvar_007D9D30, DL=0x1C, CL=0x01)  @ 0x0077F414
   ▼  byte[0x77F474+0x1C]=0x19 → dword[0x19] = 0x00785DA6
   ▼  JMP [0x19*4 + 0x785DBB] = 0x00785DD7
   ▼  prefix "[01][1C][01][01]" ++ payload
TForm1_CY_AddSedQueue(TForm1, …)               @ 0x0051633C
   ▼  [F4 44][len LE16][1C][01][…]  XOR 0xAD
```

> ❓ **Điểm vào luồng chưa chốt**: NPC dialogue (`0x18`/`0x14`), `17 xx` dùng vật phẩm, hay nút UI thuần.
> Bằng chứng gián tiếp mạnh: `FUN_00597954(local, 0x2EE1, 0)` hard-code **NPC id 12001**; `FUN_005D6F7C` loại trừ đúng **NPC 12001** (`0x2EE1`) và **NPC 12061** (`0x2F1D`) khỏi đường `1B` — đây là **hai NPC thầy kỹ năng**.

### 2.2 S→C — `05 03` (byte-exact)

**Parser thật là `FUN_0072E9A0`** (`client_pseudo_c/0072e9a0_FUN_0072e9a0.c:32-435` + `.asm.txt`).
`case_005_0078C7DB_FUN_0078c7db.c` là **recovery rác của Ghidra** (nó gọi `FUN_0072174c`/`TPlayers_Create`, đọc `+0x4b0/+0x4b1` = trường ngoại hình, không phải chỉ số). `FUN_0078C7DB` chỉ là **dispatcher**; sub `0x03` → `FUN_0072E9A0`.

> ✅ `opcode_05.md` §4.4 **đã đúng và byte-exact** — đối chiếu lại toàn bộ offset với `0072e9a0_FUN_0072e9a0.asm.txt`.

Dispatcher tách `MainOp = RP[0]`; `RestPayload = Copy(payload, 2, len−1)` ⇒ `RP[r] = payload[r+1]`.
`_LStrCopy(RP,k,n)` = Delphi 1-based ⇒ đọc `payload[k .. k+n−1]`.

```
payload[0]      = 0x05
payload[1]      = 0x03
payload[0x02..0x70]  = header cố định 111 byte
payload[0x71..]      = N × 3 byte skill record        N = (payloadLen − 0x71) / 3
```

**Không có count field trên wire** — client suy ra từ độ dài (ASM `0072f2e3–0072f308`):

```
MOV dword [EBP-0x10], 0x71     ; pos = 0x71
CALL LStrLen(RP) ; SUB EAX,0x70 ; IDIV ECX(ECX=3)
CMP EAX,0xFF → @BoundErr      ; guard N ≤ 255
```

| `payload[]` | Size | → `gself+` | Tên |
|---|:-:|---|---|
| `0x02` | 1 | `+0x3E9` | **thuoctinh / element** |
| `0x03..04` | 2 | `+0x3EA` | HP |
| `0x05..06` | 2 | `+0x3EC` | SP |
| `0x07..08` | 2 | `+0x3EE` | INT |
| `0x09..0A` | 2 | `+0x3F0` | ATK |
| `0x0B..0C` | 2 | `+0x3F2` | DEF |
| `0x0D..0E` | 2 | `+0x3F4` | AGI |
| `0x0F..10` | 2 | `+0x3F6` | HPX |
| `0x11..12` | 2 | `+0x3F8` | SPX |
| `0x13` | 1 | `+0x3FA` | byte job-state (kích hoạt tính lại MaxHP+MaxSP) |
| `0x14..17` | 4 | `+0x3FC` | EXP |
| `0x18..19` | 2 | `+0x400` | skill_point |
| `0x1A..1B` | 2 | `+0x402` | điểm chỉ số chưa dùng |
| `0x1C..1F` | 4 | `+0x440` | tier (`+0x43E = FUN_0065B30C(db,val)`) |
| `0x20..21` | 2 | `+0x404` | MaxHP |
| `0x22..23` | 2 | `+0x406` | MaxSP |
| `0x24..27` | 4 | `+0x40C` | ATK bonus |
| `0x28..2B` | 4 | `+0x410` | DEF bonus |
| `0x2C..2F` | 4 | `+0x414` | INT bonus |
| `0x30..33` | 4 | `+0x418` | AGI bonus |
| `0x34..37` | 4 | `+0x41C` | MaxHP rank |
| `0x38..3B` | 4 | `+0x420` | MaxSP rank |
| `0x3C..45` | 10 | `+0x430..0x438` | 5 × u16 (Rust gửi `F401`) |
| `0x46..70` | **43** | `+0x1334`, `+0x1305/6`, 7 cặp id+byte, `+0x5F0`, `+0x1308`, `+0x408`, `+0x40A` | pet/guild/adjuster |

**Skill record (3 byte):**

```
payload[0x71 + 3k .. 0x72 + 3k]   skillId : Word LE   → gself[0xFD4 + idx*3]
payload[0x73 + 3k]                level   : Byte      → gself[0xFD6 + idx*3]
idx = FUN_00759B04(skillId)        // tra cứu ngược qua DAT_009AE128[skillId], guard ≤ 0xFF
```

> ⚠️ **Client lưu `skillId` (giá trị wire) tại `+0xFD4+idx*3`, KHÔNG phải `idx`.** Mọi reader kiểm tra bất biến `gself+0xFD4+idx*3 == skillId`.
> ⇒ **Server chỉ được gửi skill id có thật trong `Data\Skill.Dat` của client**, nếu không `idx` sẽ rơi vào ô rác và client coi như chưa học.
> Trường hợp đặc biệt `skillId == 0x36B3` và level ≠ 0 ⇒ set cờ `FUN_007B0094(gvar_007DA108+0x13C, 1)` (được xoá về 0 ở `0072f328` trước mỗi lần parse `05 03`).

### 2.3 S→C — `08 01` attrCode `0x6E` (cập nhật 1 skill)

`FUN_00710AB0` (`client_pseudo_c/00710ab0_FUN_00710ab0.c:366-501`), ✅ đọc từ ASM:

```
[08][01][6E][deltaType][value1: DWORD LE][skillId: DWORD LE]      (12 byte)

if (value1 < 1)  gself[0xFD4 + idx*3] = 0                // UNLEARN
else             gself[0xFD4 + idx*3] = (Word)skillId   // LEARN
gself[0xFD6 + idx*3] = (Byte)value1                      // LEVEL
```

Sau đó, **chỉ khi** `Skill.Dat[id].type (+0x15) == 0x05` **và** `level == 1`:
phát `sound\WA0014.wav` + toast tên kỹ năng (`:429-456`) — đây là **feedback "bạn đã học được kỹ năng" phía client**.

> ⚠️ `deltaType` của `08 01` có ngữ nghĩa delta (`deltaType == 2` ⇒ client lấy đối). Với `0x6E`, `value1` là **level tuyệt đối** ⇒ phải gửi **giá trị dương tuyệt đối với `deltaType = 0`**. Gửi `deltaType = 2` sẽ **xoá level và unlearn skill**.

> ⚠️ **XUNG ĐỘT nguồn**: `case_008_0078CE37_FUN_0078ce37.c:24-47` parse `08` thành `[u32 actorId][u16 a][u16 b][u16 c]` (10 byte, **sync vị trí**), không có prop code.
> → `0x6E` (`FUN_00710AB0`) là **case `0x09` trong jumptable S→C**, tức **opcode `0x08` main với sub dispatch riêng ở tầng dưới**. Hai parser cùng đọc main `0x08` nhưng khác tầng ⇒ cần trace lại `FUN_00796347` (case 0) để chốt payload envelope chính xác. ❓

### 2.4 S→C — `1C 01` / `1C 02` (⚠️ chưa rõ ngữ nghĩa)

Dispatch: `jumptable_byte200_0x78A8EE[0x1C] = 28` → `case_028_0079285A_FUN_0079285a.c:8-37`:

```
sub 1: gself->+0x376 == 0 → FUN_0072C64C(gvar_007D9D34, RP)
       gself->+0x376 != 4 → FUN_006546E0(gvar_007DA51C, RP)
sub 2: FUN_0072F534(gvar_007D9D34, RP)
```

Cả hai parse giống nhau (`0072c64c:41-66`, `006546e0:34-60`):

```
_LStrCopy(RP, 2, 4, &tmp)          ; tmp = RP[1..4]  (4 byte)
idx = FUN_0077EF7C(gvar_007D9D30, tmp)
level = RP[5]
FUN_0072A7A8(array[idx*4], level)   ; entry+0x3D8 = level
```

Wire: `[F4 44][len][1C][01][ skillId : u32 LE ][ level : u8 ]` — record 5 byte.

- `FUN_0072C64C` → index vào `gvar_007DA300[]`, **bound 800** (danh sách NPC dạy skill).
- `FUN_006546E0` → `FUN_00654CE0(obj, id)` → index vào `obj+0x158[]`, **bound 21** = **danh sách skill đã học của nhân vật**.

> ❓ Frame này có thể vừa là *"NPC này dạy kỹ năng X"* vừa là *"bạn vừa học X"*. Bear **không bao giờ gửi opcode 28 S→C** nên không có tham chiếu. Cần trace ngược để chốt.

### 2.5 Cây kỹ năng phía client — server KHÔNG gửi

**Render: `FUN_0059F090`** (4684 byte, `test/ts_decompile/functions/0059f090_FUN_0059f090.c/.asm.txt`), gọi từ `FUN_0059D780:131`.

**Chọn tab theo element** (ASM `:27-44`) — **không phải identity**:

```
element 1 → tab 2
element 2 → tab 3
element 3 → tab 0
element 4 → tab 1
```

**Bảng cây kỹ năng hard-code** (ASM `:80-88`, lặp 22 lần):

```
EDX = tab * 0x99            ; stride 153 = 51 slot × 3 byte
MOVZX EAX, word [0x94B440 + EDX + EAX]   ; EAX = slot 0..0x32 (51)
         byte [0x94B440 + EDX + EAX + 2] ; byte yêu cầu
```

⇒ **`0x94B440` = `TSkillTreeRow[5 tab][51 slot][{u16 skillId LE, u8 requireByte}]`**, tổng `5 × 0x99 = 765` byte.

**⇒ Server KHÔNG BAO GIỜ gửi định nghĩa cây kỹ năng.** Client tự biết skill nào tồn tại; server chỉ cần:
1. gửi đúng `thuoctinh` ở `payload[0x02]` của `05 03`;
2. gửi đúng `skillId` ở tail `05 03` và `08 01 0x6E`.

Trường mỗi ô skill (tất cả ✅ từ ASM):

| Offset | Ý nghĩa |
|---|---|
| `slot+0xE4` (Word) | skillId (từ bảng `0x94B440`) |
| `slot+0x4C` | handle icon `FUN_007C9B38(graphics, IntToStr(skillId))` |
| `slot+0x108` | byte yêu cầu |
| `slot+0x136` | 1 = ẩn/grey (chưa đủ điều kiện) |
| `slot+0x10C` | 1 = **đã học** |
| `slot+0x135` | 1 = **không đủ SP** (`spCost > player+0x3EC`) |
| `slot+0x138` | child control, `+0x70/+0x74` bật click-to-learn |

Nguồn dữ liệu mỗi skill:

| Trường | Nguồn |
|---|---|
| skillId | bảng hard-code `0x94B440` — **không từ wire** |
| icon / tên / sp_cost / require-level | `Data\Skill.Dat` theo skillId |
| cờ đã học + level | `player+0xFD6+idx*3` (chỉ do `05 03` tail và `08 01 0x6E` ghi) |
| element của skill | `Skill.Dat` record `+0x1A` qua `FUN_0075AB54` |

**Accessor `Skill.Dat` (record 86 byte = 0x56)** — ✅:

| Accessor | Offset | Field |
|---|:-:|---|
| `FUN_0075AA4C` | `+0x00` | name |
| `FUN_0075AAAC` | `+0x15` | **type** |
| — | `+0x16` | id (Word) |
| `FUN_0075A9A0` | `+0x18` | sp_cost |
| `FUN_0075AB54` | `+0x1A` | **elem** |
| `FUN_0075A9F8` | `+0x25` | skill_type ⚠️ doc §2.1 ghi `+0x21` |
| `FUN_0075AB00` | `+0x2E` | level yêu cầu của tiền đề |

**Gate "phải học mới dùng"** — `FUN_006538BC` (`test/ts_decompile/functions/006538bc.c:138-166`):

```c
uVar6 = FUN_0075AAAC(gvar_007DA554, skillId);   // record+0x15 type
if ((char)(~(byte)uVar6 - 0x10) == 0) {          // ⇔ type == 0x11 (17)
    idx = FUN_00759B04(gvar_007DA554, skillId);
    if (*(ushort*)(gvar_007DA7BC + 0xfd4 + idx*3) != skillId) {
        toast("...chưa học...");
        unit+0x5A8 = 10000;                       // revert về đánh thường
    }
}
```

⇒ `type == 17` trong `Skill.Dat` = "bắt buộc phải học". Client **tự chặn** khi skillId không nằm trong mảng đã học.

**Gate element khi dùng** — `FUN_0076D578:1179-1204`:

```
skill.elem (FUN_0075AB54)
if (elem == 5) → OK (trung lập)
else if (elem != target->element) → toast reject 1500ms
```

⇒ **`elem == 5` là giá trị "không giới hạn hệ"** (khớp `character_sheet.rs:42-51`).

### 2.6 Gate level trong cây (ASM `:406-432`)

```
:406 CMP byte [player+0x376], 0        ; cờ preview-mode
:410 MOVZX EAX, word [selfRef+0x400]  ; Level
:411 CMP EAX, [EBP-0x18] / JL skip   ; level < yêu cầu → không học được
:428 MOV AL, byte [table + slot*3 + 2]; requirement byte
:431 CMP byte [EBP-0x4f], 0xF5 / JZ skip   ; sentinel 0xF5 = khoá
```

### 2.7 Âm thầm: không có bằng chứng guild/Hội-đồng

Grep `Hoi`, `DiemKyThuat`, `Phan Thien`, `Phan Tinh`, `Quang`, `Thuat`, `Hoc` trên toàn bộ `lit_*.hex` của cả hai thư mục: **0 hit**.
Móc gần nhất là `gvar_007D9C20 = TCY_OrganManage` và `+0x5F0`/`+0x47C` trong header `05 03` (tra bạn/đồng đội) — **không xác nhận được liên hệ với mở khóa kỹ năng**. ❓

### 2.8 Thứ tự frame sau login (bắt buộc)

| # | Frame | Lý do |
|:-:|---|---|
| 1 | `03` (self-appear) | phải trước `05 03`; mọi reader key off `gself+4` |
| 2 | **`05 03`** | **phải trước mọi `08 01`**; nguồn duy nhất của mảng 3-byte kỹ năng |
| 3 | `08 01` `0x3E` (tier) | HUD đổi layout khi `+0x43E > 12` |
| 4 | `08 01` `0x6E` (per-skill) | khoảnh khắc mở khóa |
| 5 | `17 05` / `1E 01` / `17 0B` / `1A 04` | độc lập kỹ năng |
| 6 | **`05 04`** (world-ready) | `scene+0x53FC = 1` mới mở khoá input. **Rust KHÔNG gửi trong Logined1** |
| 7 | `25 01` (C→S map-loaded) → broadcast | cuối |

Rust `build_logined_sequence_session` (`spawn.rs:471-604`) đã đặt `03` trước `05 03` ✅, nhưng **không gửi `08 01` và không gửi `05 04`**.

---

## 3. Đối chiếu server Rust — Gap List

### 🔴 CRITICAL

| ID | Vấn đề | Vị trí | Sửa |
|:-:|---|---|---|
| **G-01** | **`05 03` ghi 90 byte 0 thay vì 43** → skill list lệch **47 byte**, client không đọc được skill nào. Không có skill nào hiển thị, không học được gì. Comment `spawn.rs:124` đã ghi đúng "skills/2 + 113" nhưng code sai. | `src/server/spawn.rs:183` | `&"00".repeat(90)` → `&"00".repeat(43)` |
| **G-02** | **`data.skills` rỗng trong production** → `handle_player_skill_learn` `break` ngay record đầu; quest `AddSkill` không bao giờ chạy; pet upgrade bị bỏ qua. | `src/data/loader.rs:295-370` (`load_binary` không gọi `load_skills`); `skills.rs:126` | nạp `tables::Skill` từ `Skill.Dat` lúc boot (cần mapper `BinarySkillDef` → `tables::Skill`, phải có ADR vì mapping lossy) |
| **G-03** | **Không có khái niệm nhóm `type` 18/19 (Quang/Ám).** `can_learn_element()` (`skills.rs:10-18`) và `get_point_skill_add()` (`skills.rs:20-33`) chỉ biết `elem` 1..4. Nhân vật Quang/Ám không thể được tạo (`character.rs:59-62` clamp `1..=4`). | `skills.rs:10-33`, `character.rs:59-62`, `session.rs:173`, `migrations/0001_init.sql:48`, `battle/combat_formula.rs:135-146`, `protocol/codecs/thing_data.rs:45` | thêm cột `characters.skillgroup` (hoặc `system`) + cờ mở khóa; định nghĩa enum nhóm 1..19; gate học theo nhóm |
| **G-04** | **Số hiệu hệ mâu thuẫn 3 nơi**: `0001_init.sql:48` nói `5=Quang, 6=Ám, 7=Tâm`; `thing_data.rs:45` nói `5=heart, 7=light, 8=dark`; `combat_formula.rs:135-146` dùng `7↔8` là quang/ám và `5|6` trung lập; Bear C# dùng `5` = "Tâm" (trung lập). | như trên | chốt 1 bảng enum + ADR |
| **G-05** | **Thứ tự byte record `1C 01` chưa chốt.** Bear đọc `[skillId u16][lv u8]`; builder `1B 01` của client sinh `[lv u8][skillId u16]`. Rust theo Bear. | `skills.rs:114-120` | capture thật hoặc dump hole `0x005987DB–0x005999F7` |

### 🟠 HIGH

| ID | Vấn đề | Vị trí | Sửa |
|:-:|---|---|---|
| **G-06** | **Client tự kiểm tra "đã học chưa"** qua `gself+0xFD4+idx*3 == skillId` với `idx = FUN_00759B04(skillId)` tra bảng ngược `DAT_009AE128`. SkillId không có trong `Data\Skill.Dat` client ⇒ ghi rác ô ⇒ coi như chưa học. Rust phải chỉ gửi id có thật. | `spawn.rs:190-197`, `skills.rs:152` | whitelist skill id theo `Skill.Dat` trước khi ghi vào `05 03` tail |
| **G-07** | **`08 01 0x6E` phải gửi `deltaType = 0`** (level tuyệt đối). Rust hiện gửi mode khác → có thể **xoá skill đã học**. | `skills.rs:152-156`, `books.rs:90-95`, `quest.rs:495-499` | kiểm chứng & sửa byte thứ 4 |
| **G-08** | **Multi-skill book (46132/46133/46134/46136) không gửi `6E01`** → client không cập nhật danh sách. | `use_item/books.rs:103-119` | thêm frame `6E01` cho từng skill |
| **G-09** | **Sách đơn (46230..46233, 46246) không gửi feedback `1709`/`170F`** và bỏ qua mọi gate. | `use_item/books.rs:75-100` | thống nhất với `consume()` |
| **G-10** | **`0x05 04` (world-ready) không được gửi** → client đóng băng input sau login. | `spawn.rs:471-604` | thêm vào Logined1 (sau `05 03`) |
| **G-11** | **Battle không kiểm tra skill đã học**: `runner.rs:766` `2 => valid = true // player commands are trusted`; `battle.rs:269` default level 1 cho skill lạ. | `battle/runner.rs:760-774`, `battle.rs:248-270` | truyền tập skill đã học của caster vào `BattleData` và validate |
| **G-12** | **Fingerprint autosave bỏ sót `skills`**: `auto_save.rs:51` chỉ mix `skill_point`. Học skill trong RAM có thể không bao giờ flush. | `src/server/auto_save.rs:45-60` | thêm `session.skills` vào hash |
| **G-13** | **`character_skills.sp` / `saveflag` là cột chết**: `sqlite/session.rs:339` ghi cứng `0,0`; `:82` không select. `upsert_skill` ghi `sp` thật rồi autosave ghi đè bằng 0. | `src/db/modern/sqlite/session.rs:82,338-341`, `src/db/persist.rs:367-378` | `Session.skills: Vec<PlayerSkill{id,level,sp,flags}>` |

### 🟡 MEDIUM

| ID | Vấn đề | Vị trí |
|:-:|---|---|
| **G-14** | **`OP_RESET = 0x2A` đăng ký nhưng không handler** (respec bằng sách rửa skill của Bear: `resetHandler.cs` + `TSItem.uesItemResetSkillChar`) | `dispatcher.rs:22,373` |
| **G-15** | **Skill book bypass mọi gate** (element, tiền đề, `skill_point`) | `use_item/books.rs:13-34` |
| **G-16** | **`quest.rs` `AddSkill` chỉ RAM** — không gọi `persist::upsert_skill` | `handlers/quest.rs:486-505` |
| **G-17** | **`EveResultClass::Skill = 4` không có executor** — `talk.rs:381` chỉ match 1,2,5,7, rơi vào `other` sink. Đây chính là đường cấp skill miễn phí của Bear (`GetAddskill`) | `handlers/talk.rs:381-550`, `data/loaders/eve.rs:103-111` |
| **G-18** | **Vòng lặp `1C 01` `break` sớm**: một record hỏng làm hỏng cả batch (client gửi 1 frame cho nhiều skill) | `skills.rs:122-124` |
| **G-19** | **`1C 05` (skill reborn 2) không thu `skill_point`, không kiểm tra element/tiền đề; đọc `ball_count` nhưng không trừ `ball_point`** | `skills.rs:288-340` |
| **G-20** | **Item→skill map hardcode 9 item**; `Item.dat` có `skill_link` nhưng loader đặt cứng `0` | `use_item/books.rs:13-34`, `data/loaders/item.rs:41,299` |
| **G-21** | **`skill_point` khởi tạo = 0** cho nhân vật mới (`character.rs:247`) ⇒ không học được gì cho tới reborn | `handlers/character.rs:247` |
| **G-22** | **`Formula.Dat` không được nối vào boot** (`formula_params = None`); `skill_id → (level → sức mạnh)` là **thang hardcode** ~40 id trong `damage.rs:213-300` | `data/loader.rs:56-59,332-340`, `battle/damage.rs:213-300` |

### 🟢 LOW

| ID | Vấn đề | Vị trí |
|:-:|---|---|
| **G-23** | `0x28` hotkey chấp nhận mọi skill id kể cả chưa học; `kind` khác 0/2 bị drop không phản hồi | `handlers/stats.rs:142-165` |
| **G-24** | `CharacterSheet` / `PlayerStateManager` không biết skill ⇒ không recompute passive | `character_sheet.rs:78-95`, `player_state.rs:43-104` |
| **G-25** | `docs` drift: `CONTEXT.md:93` mô tả element không nói 5/6/7; `docs/adr/0002:13` + `PRODUCTION_RUNBOOK.md:23` vẫn nói `Skill.Dat` chưa xác minh (ADR 0002 §77 đã `DONE 2026-08-28`) | `CONTEXT.md`, `docs/adr/0002` |
| **G-26** | `skill_pc.rs` offset `skill_type` mâu thuẫn: doc battle ghi record `+0x21`, ASM `FUN_0075A9F8` dùng hằng `0x25` | `battle/03_skill_and_combo.md` §2.1 vs `0075a9f8` |

---

## 4. Bảng đối chiếu nhanh Rust ↔ Bear ↔ Client

| Hạng mục | Bear C# | Rust | Client (`FUN_0072E9A0` / `FUN_00710AB0`) | Khớp? |
|---|---|---|---|:-:|
| Học skill | `1C 01` | `0x1C sub 1` ✅ | `0x00785DD7` gửi `1C 01` | ✅ |
| Payload record | `[id u16][lv u8]` | `[id u16][lv u8]` | `[lv u8][id u16]` (builder `1B`) | ⚠️ |
| Ack học skill | `08 01 6E 01 <lv u32> <id u32>` | `skills.rs:152-156` | `FUN_00710AB0` `6E` 12 byte | ✅/⚠️ deltaType |
| Skill point push | `08 01 25 01 <u32> 0` | `skills.rs:224-227` | `05 03` `payload[0x18..19]` | ✅ |
| `05 03` header | 113 byte (`500×5 + addZero(43)`) | **158 byte (`F401×5 + 90 zero`)** ❌ | 113 byte | ❌ **G-01** |
| Skill list tail | `{u16 id, u8 lv}` × N, **không count** | `spawn.rs:190-197` ✅ | `N=(len−0x71)/3` ✅ | ✅ |
| Element double-cost | `elem == chr.elem ? base : base*2`, `elem==5` miễn | `skills.rs:20-33` (thiếu `elem==5`) | không có (client không tính tiền) | ⚠️ |
| `elem == 5` miễn phí | ✅ `TSCharacter.cs:4678,3850,3936,4607` | ❌ thiếu | `FUN_0076D578` chỉ dùng cho gate *dùng* | ❌ |
| Tiền đề | `require_sk` đơn + ngoại lệ `id != 13014` | `id_dk[6]`, pass nếu **bất kỳ** cái nào có | `FUN_0075AB00` `+0x2E` (level tiền đề) — chưa dùng | ⚠️ |
| Respec `0x2A` | ✅ `resetHandler.cs` | ❌ chưa handler | `TBK_SkillRegressForm` tồn tại | ❌ |
| Skill từ sách | ❌ Bear không có | ✅ `books.rs` | — | ➖ |
| Cấp skill miễn phí qua Eve | ✅ `GetAddskill` | ❌ `EveResultClass::Skill` không executor | — | ❌ |
| Quang/Ám | ❌ **không tồn tại** | ❌ | ❓ `type 18/19` trong `Skill.Dat` | ❌❌ |
| Cây kỹ năng | ❌ gửi từ server | ❌ | ✅ hard-code `0x94B440[5][51]` | ✅ client tự lo |

---

## 5. Đề xuất thứ tự hành động

1. **Sửa `spawn.rs:183`: `repeat(90)` → `repeat(43)`.** Một dòng, mở khoá toàn bộ skill window. *(kiểm chứng bằng: `05 03` với 3 skill → `len = 113 + 9 + 2 = 124` byte payload)*
2. **Thêm test `tests/`** khẳng định `05 03` header = 113 byte và `(payload.len()-2-113) % 3 == 0` (theo `AGENTS.md`: test phải đặt ở `tests/`, cấm `#[cfg(test)]` trong `src/`).
3. **Chốt layout record `1C 01`** — capture `aLogin.exe`, hoặc dump hole `0x005987DB–0x005999F7`.
4. **ADR về mô hình Quang/Ám**: nhóm `type` 18/19 như một **trục thứ 3** độc lập với `elem`; chốt enum hệ thống (5/6/7 hay 7/8); migration cột mới.
5. **Nối `tables::Skill` vào boot** từ `Skill.Dat` (ADR mapper, vì `BinarySkillDef` thiếu `id_dk[6]`/`point`/`reborn`).
6. **Thêm `05 04`** vào Logined1 sau `05 03`.
7. **Đóng các frame `6E01` còn thiếu** cho sách đa kỹ năng + sách đơn; sửa `deltaType` về 0.
8. **Thêm executor cho `EveResultClass::Skill`** — đây là đường tự nhiên để cấp Quang/Ám skill từ `eve.emg` (giống `GetAddskill` của Bear) mà không cần NPC mới.
9. **Whitelist skill id** trước khi ghi vào `05 03` tail (G-06).
10. Validate battle theo tập skill đã học (G-11).

---

## 6. Danh sách dump cần thêm (chưa có trong `client_pseudo_c/`)

| # | Địa chỉ | Kích thước | Vì sao quan trọng |
|:-:|---|:-:|---|
| 1 | **`0x005987DB–0x005999F7`** | ~4.6 KB | Chứa **builder payload** ghi `+0x1A8` + nút OK gọi `FUN_0077F414(…,0x1C)` của dialog `gvar_007D9EB8`. Song song với hole tàu đã chứng minh `0x005B347D–0x005B37C7`. |
| 2 | **`0x005B347D–0x005B37C7`** | ~0.8 KB | Chứa `0x5B3620`/`0x5B3731` (gọi `FUN_0077EB1C` = writer Word-LE) + `0x5B3638`/`0x5B374A` → **khuôn mẫu chính xác** để append record kỹ năng. |
| 3 | `TForm1.CY_AddSedQueue` @ **`0x0051633C`** | — | Chốt câu hỏi byte `0x01` length-prefix có bị strip hay không. |
| 4 | **`0x94B440..0x94B6A9`** | **765 byte** | ⭐ **Quan trọng nhất**: toàn bộ 255 skill id + requirement byte theo 5 tab. Đây là câu trả lời dứt điểm "id nào thuộc Quang, id nào thuộc Ám". |
| 5 | **`DAT_009AE128`** | — | Bảng `u16` tra ngược `skillId → index` của client (3000 slot). Xác nhận skill id nào client thật sự biết. |
| 6 | `.rodata` `0x005854E4`, `0x00584384`, `LAB_0072A870`, `LAB_0072C665` | 28 B ×4 | Chuỗi caption cửa sổ kỹ năng (duy nhất ngoài VMT). |
| 7 | `0x0059C089–0x0059C0AB`, `0x0059C61F–0x0059CB2B`, `0x0059D750–0x0059D77F`, `0x0059EFE3–0x0059F08F`, `0x005A02DC–0x005A0387` | — | Các method còn lại (sender sub 2/4/5, OK/Cancel). |
| 8 | `FUN_0076D578` body đầy đủ | 6841 B | Đã xác nhận gate `elem` ở `:1179-1204`; phần còn lại chưa đọc. |
| 9 | `TFSkill`: `0x00759B2C` (510 B), `0x00759DF0` (2891 B), `TFSkill.Create` `0x00759A8C`, 8 accessor `0x0075A948…0x0075AB54` | — | Có trong `index.csv` nhưng thiếu file `.c`/`.asm.txt`. |
| 10 | Chuỗi caption tab tại `0x5A02F0, 0x5A0300, 0x5A0314, 0x5A033C, 0x5A0354, 0x5A0378` | ~0xA0 B | Xác định tab 0 là "Quang" hay "Kim" (remap `1→2,2→3,3→0,4→1`). |

---

## 7. Sửa lỗi tài liệu (kèm theo phát hiện này)

1. **`post_login_stats_inventory_flow.md` §3.1** — phải ghi rõ `05 03` header là **113 byte** và `spawn.rs:183` đang sai (90 → 43). Gap #4 trong tài liệu đó **đã đóng** bằng `FUN_0072E9A0` + `opcode_05.md` §4.4.
2. **`case_005_0078C7DB_FUN_0078c7db.c`** không phải parser `05 03` — phải dẫn sang `FUN_0072E9A0`.
3. **`cs_op17_first_cases.asm.txt`** thực tế là **main `0x18`**, không phải `0x17` (`byte[0x77F474+0x18] = 0x16 → 0x00785226`).
4. **`cs_op1c_dispatch_cases.asm.txt`** in case đầu ở `0x785DD8` nhưng jump target thật là `0x00785DD7` (lệch 1 byte, nội dung đúng).
5. **`opcode_1b.md:121-122`** ("`case 0x1b: break` → client không gửi `0x1B`") — **SAI**. `0x00785FFD` có body thật (sub 1 và 2), và `FUN_005D6F7C` chứng minh client gửi `DL=0x1B`.
6. **`battle/03_skill_and_combo.md` §2.1** ghi `skill_type` ở record `+0x21`; ASM `FUN_0075A9F8` dùng hằng `0x25`. Cần chốt base offset trước khi dùng làm gate.
7. **`post_login_stats_inventory_flow.md:107,228`** vẫn ghi `17 1E` = case idx 20. Theo `jumptable_byte200_0x78A8EE.hex`, `byte[0x18] = 23`; handler `FUN_00730F4C` được gọi từ `case_020_007902FB.c:149` ⇒ **`18 1E`**, không phải `17 1E`.
8. **`CONTEXT.md`** — thêm mục `skill group` (type 18 = Quang, 19 = Ám) vào ubiquitous language.

---

*Tài liệu tổng hợp từ 4 lượt subagent nghiên cứu (client C→S, client S→C, audit Rust server, server Bear C#) cộng với phân tích trực tiếp `Data/Skill.Dat` (352 record) và `Data/Skill2.Dat` (1900 byte) ngày 2026-10-02. Mọi offset/byte đã tự kiểm chứng lại; hai claim sai của subagent đã bị loại (xem §3 G-01 và §2.3 note).*