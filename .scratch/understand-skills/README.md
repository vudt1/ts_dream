# `client_skills_handler/` — Bundle mã nguồn client cho module "KỸ NĂNG (Skills)"

> **Mục đích:** gom toàn bộ mã nguồn C / ASM / HEX đã dump từ Ghidra project của
> `client_delphi/aLogin.exe` (client TS Online / Thiên Sơn Online) liên quan tới
> **module kỹ năng**, để đưa cho agent AI phân tích và tái tạo nghiệp vụ phía server.
>
> **Phạm vi bao phủ (theo yêu cầu):**
> 1. **Kỹ năng sống hệ Tâm** — *toàn bộ*: trinh sát, đào tẩu, mua bán / mậu dịch, dò xét / thám hiểm,
>    luyện đơn / luyện đan, đào khoáng / khai khoáng, đào sâu / thâm khoáng, đúc luyện / luyện kim,
>    chế tạo / luyện tạo, tinh luyện, giao dịch, tịnh toạ, khai thương, thiền định,
>    nộ hống, đồng tâm, cổ vũ, hỗn loạn, hỗn độn, **cưỡi ngựa (kỵ mã)**, **triệu gọi (triệu linh)**,
>    đạo sư (gia sư), chiếm đọat, đoạt nguyên, thánh quang, hộ chủ.
> 2. **Kỹ năng nghề sau chuyển sinh / tái sinh** — 4 nhánh **Bá / Tiên / Hiền / Hiệp**
>    + kỹ năng gốc "Đại chu thiên" + "bóng" (ball) `skill_rb2` theo 4 hệ Kim/Thủy/Hỏa/Phong.
> 3. **Kích hoạt & học kỹ năng Quang/Ám** (Light/Dark) — `type` 18 / 19 trong `Skill.Dat`.
> 4. **Kỹ năng triệu gọi (summon)** — hệ Thủy / Hỏa / Địa / Phong.
> 5. **Nâng cấp kỹ năng** (skill level-up), học skill, tẩy skill (respec), skill point.
>
> **Ngày lập:** 2026-10-21 · **Repo:** `/mnt/d/VUDT/GIT_PCC/ts_dream`

---

## 1. Bắt đầu từ đâu

| Muốn làm gì? | Mở file |
|---|---|
| **Giao cho agent AI nghiên cứu** | [`PROMPT.md`](./PROMPT.md) — copy nguyên khối trong ```` ```` ```` rồi dán cho agent |
| **Hiểu mô hình dữ liệu kỹ năng** | [`SKILL_DATA_MODEL.md`](./SKILL_DATA_MODEL.md) ★★★ |
| **Hiểu cấu trúc frame C→S / S→C** | [`WIRE_FORMATS.md`](./WIRE_FORMATS.md) ★★ |
| **Biết client đang làm gì với từng id kỹ năng** | [`data/`](./data/) — 8 file TSV đã giải mã |
| ⭐ **Tự kiểm chứng lại catalog skill** | [`reference/data_files/`](./reference/data_files/) — `Skill.Dat` + `skills.json` bản gốc |
| **Hiểu vì sao bảng cây kỹ năng "rỗng"** | [`BLOCKED_DUMPS.md`](./BLOCKED_DUMPS.md) ★★ |
| **Xem kết quả đợt dump Ghidra Round-7** | [`ROUND7_RESULTS.md`](./ROUND7_RESULTS.md) ★★★ |
| **Xem kết quả đợt dump Ghidra Round-8** | [`ROUND8_RESULTS.md`](./ROUND8_RESULTS.md) ★★★ |
| **Xem kết quả đợt dump Ghidra Round-9** | [`ROUND9_RESULTS.md`](./ROUND9_RESULTS.md) ★★★ |
| **Xem kết quả đợt dump Ghidra Round-10** | [`ROUND10_RESULTS.md`](./ROUND10_RESULTS.md) ★★★ |
| ⭐ **Xem kết quả đợt dump Ghidra Round-11** | [`ROUND11_RESULTS.md`](./ROUND11_RESULTS.md) ★★★ (`.rsrc`/DFM + form skill nạp từ file ngoài) |
| ⭐⭐ **Xem kết quả đợt dump Ghidra Round-12** | [`ROUND12_RESULTS.md`](./ROUND12_RESULTS.md) ★★★★ (**caption tab 5 = tên ảnh `5btn_02`** + 4 hàm dựng cây nhánh nghề + bảng dispatch S→C đầy đủ) |
| ⭐ **Tìm mọi chuỗi VISCII trong client** | [`data/viscii_strings.tsv`](./data/viscii_strings.tsv) (22 138 dòng) · tool [`tools/extract_strings.py`](./tools/extract_strings.py) |
| ⭐ **Đọc DFM/resource của client** | [`tools/parse_dfm.py`](./tools/parse_dfm.py) · [`data/dfm_properties.tsv`](./data/dfm_properties.tsv) |
| ⭐ **Backup source về `client_pseudo_c/`** | `python3 tools/sync_to_pseudo_c.py` — copy flat 846 file `.c`/`.asm.txt`/`.hex` + cập nhật `client_pseudo_c/index.csv` |
| **Bổ sung source còn thiếu từ Ghidra** | [`MISSING_DUMPS.md`](./MISSING_DUMPS.md) + **manifest đã sẵn sàng**: [`Ghidra_Skills_Round10_Redump_Manifest.txt`](./Ghidra_Skills_Round10_Redump_Manifest.txt) |
| **Chạy dump Ghidra ngay** | `bash tools/run_skills_round13.sh` (interactive) hoặc `bash tools/run_skills_round13.sh <projDir> headless` |
| **Kiểm tra manifest trước khi chạy** | `python3 tools/validate_manifest.py Ghidra_Skills_Round13_Redump_Manifest.txt` |
| **Đọc tài liệu phân tích sẵn có của repo** | [`reference/analysis_docs/`](./reference/analysis_docs/) — 5 file `.md` |
| **Đối chiếu phía server** | [`reference/bear_server/`](./reference/bear_server/) — 5 file `.cs` (server Bear C#) |
| ⭐ **File dữ liệu gốc (Skill.Dat + skills.json)** | [`reference/data_files/`](./reference/data_files/) ★★ — 2 file nguồn + [`README.md`](./reference/data_files/README.md) giải thích |
| **Tự giải mã lại dữ liệu** | [`tools/`](./tools/) — **10** script Python |

---

## 2. Nội dung thư mục

```
client_skills_handler/
├── PROMPT.md                     ★★ PROMPT NGHIÊN CỨU (dán cho agent AI)
├── README.md                     ★ file này — chỉ mục bundle
├── SKILL_DATA_MODEL.md           ★★★ mô hình dữ liệu kỹ năng + cây kỹ năng sống + 4 nhánh nghề
├── WIRE_FORMATS.md               ★★ khung wire C→S / S→C đã đối chiếu byte
├── MISSING_DUMPS.md              ★ các vùng source còn thiếu, cần Ghidra dump bổ sung
├── BLOCKED_DUMPS.md              ★★ 5 file hex rỗng + cách khôi phục bảng cây kỹ năng
├── ROUND7_RESULTS.md             ★★★ kết quả đợt dump Round-7 (ok=250 fail=0) + phát hiện mới
├── ROUND8_RESULTS.md             ★★★ kết quả đợt dump Round-8 (ok=127 fail=0) + phát hiện mới
├── ROUND9_RESULTS.md             ★★★ kết quả đợt dump Round-9 (ok=37 fail=0) + PHÁT HIỆN LỚN
├── ROUND10_RESULTS.md            ★★★ kết quả đợt dump Round-10 (ok=162 fail=0) + PHÁT HIỆN LỚN
├── ROUND11_RESULTS.md            ★★★ kết quả đợt dump Round-11 (ok=191 fail=0) + PHÁT HIỆN LỚN
├── ROUND12_RESULTS.md            ★★★★ kết quả đợt dump Round-12 (ok=100 fail=0) + PHÁT HIỆN LỚN
├── Ghidra_Skills_Round7_Redump_Manifest.txt  ★★ manifest dump Ghidra Round-7 (84 lệnh, đã chạy)
├── Ghidra_Skills_Round8_Redump_Manifest.txt  ★★ manifest dump Ghidra Round-8 (67 lệnh, đã chạy)
├── Ghidra_Skills_Round9_Redump_Manifest.txt  ★★ manifest dump Ghidra Round-9 (31 lệnh, đã chạy)
├── Ghidra_Skills_Round10_Redump_Manifest.txt ★★ manifest dump Ghidra Round-10 (52 lệnh, đã chạy)
├── Ghidra_Skills_Round11_Redump_Manifest.txt ★★ manifest dump Ghidra Round-11 (62 lệnh, đã chạy ok=191)
├── Ghidra_Skills_Round12_Redump_Manifest.txt ★★ manifest dump Ghidra Round-12 (20 lệnh, đã chạy ok=100)
├── Ghidra_Skills_Round13_Redump_Manifest.txt ★★ manifest dump Ghidra Round-13 (12 lệnh, validate 0 lỗi)
├── round3_report_round7.txt      báo cáo gốc Round-7
├── round3_report_round8.txt      báo cáo gốc Round-8
├── round3_report_round9.txt      báo cáo gốc Round-9
├── round3_report_round10.txt     báo cáo gốc Round-10
├── round3_report_round11.txt     báo cáo gốc Round-11
├── data/
│   ├── skill_catalog_client_dat.tsv   352 skill giải mã từ Data/Skill.Dat (client_dat.tsv)
│   ├── skill_catalog_skills_json.tsv  376 skill từ Data/skills.json (KHÔNG GIAN ID THẬT)
│   ├── skill_id_space_conflicts.tsv   601 dòng — đối chiếu 2 không gian id
│   ├── skill_type_groups.tsv          20 nhóm `type` (0..19) × 2 không gian
│   ├── skill_life_tam.tsv             88 dòng — toàn bộ kỹ năng sống hệ Tâm
│   ├── skill_life_tam_tree.tsv         36 dòng — cây kỹ năng sống (id server) + nguồn cấp
│   ├── job_skill_branches.tsv         17 dòng — 4 nhánh Bá/Tiên/Hiền/Hiệp
│   ├── skill_rb2_balls.tsv            32 dòng — 8 "bóng" theo 4 hệ
│   ├── skill_quang_am.tsv             45 dòng — 22 Quang + 23 Ám
│   ├── skill_tree_tabs.tsv            255 dòng — bảng cây kỹ năng 5 tab × 51 ô (đã KHÔI PHỤC)
│   ├── skill_tree_recovered.bin       765 byte — layout RAM của bảng cây kỹ năng
│   ├── viscii_strings.tsv             ⭐ 22 138 dòng — TOÀN BỘ chuỗi VISCII trong .text (kèm VA)
│   ├── skill_ui_captions.tsv          ⭐ 1 475 dòng — chuỗi trong 12 vùng module kỹ năng
│   ├── dfm_properties.tsv             ⭐ bảng phẳng property của 6 form DFM (Big5)
│   └── cs_dispatch_map.tsv             ⭐ 200 dòng — bảng dispatch S→C đầy đủ của FUN_0077F414
├── tools/
│   ├── decode_skill_dat.py      giải mã Data/Skill.Dat + sinh 4 TSV catalog
│   ├── build_skill_tables.py    sinh các bảng nghiệp vụ (cây Tâm, nhánh nghề, rb2, Quang/Ám)
│   ├── build_skill_dat.py       ⭐ DỰNG LẠI Data/Skill.Dat từ skills.json (byte-exact) + validate
│   ├── build_id_map.py          ⭐ map id giữa 2 build Skill.Dat
│   ├── recover_skill_tree.py    khôi phục bảng cây kỹ năng 0x94B440 từ Skill.Dat
│   ├── parse_dfm.py             ⭐⭐ parser PE .rsrc + DFM binary Delphi 7 (Big5)
│   ├── extract_strings.py       ⭐ liệt kê mọi chuỗi VISCII trong PE kèm VA
│   ├── sync_round.py            ⭐ đồng bộ 1 round vào bundle (tự sinh ___tsdec / ___rN)
│   ├── sync_to_pseudo_c.py      ⭐ copy flat .c/.asm.txt/.hex về client_pseudo_c/ + cập nhật index.csv
│   ├── validate_manifest.py     kiểm tra cú pháp + dải địa chỉ của manifest
│   └── run_skills_round*.sh     chạy Ghidra redump theo manifest (interactive/headless) — Round-7..13
├── reference/
│   ├── analysis_docs/           5 tài liệu phân tích sẵn có của repo (mốc đối chiếu)
│   ├── bear_server/             5 file C# server Bear + trích đoạn TSCharacter.cs
│   ├── rsrc/                    ⭐⭐ dump TOÀN BỘ section .rsrc của aLogin.exe (Round-11)
│   │   ├── rsrc_section_full.hex    83 968 B — .rsrc (VA 0xA53000..0xA677FF)
│   │   ├── rsrc_dir_root.hex        thư mục resource gốc
│   │   ├── rsrc_dir_names.hex       thư mục tên resource
│   │   ├── rsrc_str_scan_1/2.hex    2 vùng quét chuỗi trong .rsrc
│   │   └── dfm_tree.json            cây DFM đã parse (6 form)
│   └── data_files/              ★★ file dữ liệu GỐC (Round-9)
│       ├── Skill.Dat            32 422 B — ⭐ CANONICAL (376 × 86 B, dựng lại từ skills.json)
│       ├── Skill_legacy_352.Dat  30 358 B — build cũ (352 record) để đối chiếu
│       ├── skills.json          376 record — bảng skill phía SERVER (không gian id chuẩn)
│       └── README.md            layout record, 2 không gian id, cách sinh lại TSV
├── 00xxxxxx_FUN_*.c / *.asm.txt pseudo-C + assembly của các hàm liên quan kỹ năng
├── case_XXX_*.c                 66 file — bảng tra 65 Main OP (S→C dispatcher)
├── asm_*.asm.txt                các khối ASM chuyên biệt (skill tree, accessors, builder 0x1C…)
├── cs_*.hex / cs_*.asm.txt      bảng dispatch / builder C→S
├── jumptable_*.hex              bảng tra Main OP ↔ hàm xử lý (byte 0x78A8EE, dword 0x78A9B6)
├── skilltree_94b440*.hex        dump bảng cây kỹ năng (⚠ readable 0 — vì nằm ở BSS, xem BLOCKED_DUMPS.md)
├── skillid2index_9ae128.hex     dump bảng tra ngược skillId → index (⚠ readable 0 — BSS, runtime)
└── manifest.csv / index.csv     chỉ mục dump Ghidra gốc (giữ nguyên để đối chiếu)
```

### Nguồn gốc (provenance)

| Thư mục gốc | Vai trò | Ưu tiên |
|---|---|---|
| `client_pseudo_c/` | dump Ghidra chính (header đầy đủ `Callers/Callees/References`) | ★ ưu tiên 1 |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` | dump Ghidra bổ sung (13 592 hàm) | ưu tiên 2 |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/case_functions/` | 66 file `case_XXX` | ưu tiên 2 |
| `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/redump/` | 447 file hex/asm định hướng | ưu tiên 2 |
| `client_delphi/aLogin.exe` | binary gốc (PE x86, image base `0x00400000`) | để đối chiếu byte |
| `Data/Skill.Dat`, `Data/skills.json` | file dữ liệu kỹ năng | ★ (xem §3 — 2 không gian id!) |
| `TS_Server_Bear/TS_Server/` | server C# tham chiếu | ★ để đối chiếu nghiệp vụ |

> Khi cùng tên file tồn tại ở cả `client_pseudo_c/` và `ts_decompile/functions/`,
> bản của `client_pseudo_c/` được ưu tiên. Chỉ **1 cặp file khác nhau** được giữ cả hai bản
> (đánh dấu hậu tố `___tsdec`): `006538bc_FUN_006538bc.c` / `.asm.txt` — hàm kiểm tra
> "đã học kỹ năng chưa", **quan trọng nhất cho module này**.

---

## 3. ⚠️ PHÁT HIỆN SỐ 1 — HAI KHÔNG GIAN ID KỸ NĂNG KHÁC NHAU

Đây là điều **quan trọng nhất** của bundle và là nguyên nhân nhiều tài liệu cũ sai.

| | `Data/Skill.Dat` (352 record) | `Data/skills.json` (376 record) |
|---|---|---|
| Kỹ năng sống hệ Tâm (type 5) | id `13953-13963`, `14012-14013`, `14028-14043`, `14044-14061`, `14062-14075`, `19990` | id `14001-14061` |
| Hệ QUANG (type 18) | id `21953-21963`, `22060-22070` | id `22001-22022` |
| Hệ ÁM (type 19) | id `22972-22975`, `22988-22995`, `22996-22998`, `22999-23003`, `23017-23019` | id `23001-23023` |
| Ai đọc | `src/data/loaders/skill_pc.rs` (server Rust) | **client binary `aLogin.exe`** |

**Bằng chứng client dùng không gian `skills.json`:** hàm `FUN_006538BC` (file
`006538bc_FUN_006538bc.asm.txt`) **hard-code** các id kỹ năng sau để hiện thông báo
"chưa học kỹ năng này":

```
10020, 11020, 12020, 13019,
14003, 14004, 14005, 14006, 14007, 14008,
14016, 14017, 14018, 14019, 14020, 14021,
14023, 14025, 14026, 14027, 14028,
14032 … 14052   (bảng nhảy 20 entry tại 0x006539D7, base = skillId - 0x36D1)
```

→ **100 % của các id trên tồn tại trong `Data/skills.json`, và phần lớn KHÔNG tồn tại
trong `Data/Skill.Dat`.**

Đối chiếu chéo phía server (`TS_Server_Bear/.../TSCharacter.cs`) cũng dùng đúng không gian
`skills.json`: `rebornChar()` cấp `skill.TryAdd(14001, 1) / 14015,10 / 14021,5 / 14023,1 /
14035,1 / 14002,1 / 14003,1 / 14004,10 / 14005,10 / 14010,10 / 14011,10 / 14018,10 / 14019,10`.

> 🔴 **Hệ quả cho server Rust:** `src/data/loaders/skill_pc.rs` đang nạp `Data/Skill.Dat`
> và `src/server/handlers/skills.rs` sẽ **gửi id sai cho client** nếu dùng bảng đó.
> Server phải dùng không gian `skills.json` khi nói chuyện với client.
> Xem [`SKILL_DATA_MODEL.md` §2](./SKILL_DATA_MODEL.md).

---

## 4. ⚠️ PHÁT HIỆN SỐ 2 — ĐÍNH CHÍNH BẢN ĐỒ OPCODE S→C

Tài liệu cũ `.scratch/client-pseudo-op-code/light_dark_skill_unlock_gap.md` §2.4 nói frame
`1C 01` / `1C 02` là **S→C**. **Điều này SAI.** Đã đối chiếu byte bảng bytemap
`0x0078A8EE` (200 byte, file `jumptable_byte200_0x78A8EE.hex`):

```
opcode 0x1C → byte[0x78A8EE+0x1C] = 0x00  ⇒  HOLE  ⇒ S→C 0x1C KHÔNG được xử lý
opcode 0x20 → byte[0x78A8EE+0x20] = 0x1C  ⇒  case 28 ⇒ dword[0x1C] = 0x0079285A
```

⇒ Handler `FUN_0079285A` (`case_028_0079285A_FUN_0079285a.c`) là **opcode `0x20` sub 01/02**,
không phải `0x1C`. Các frame `1C 01/02` mà server Bear gửi đi sẽ **bị client bỏ rơi**.

Bảng tra đầy đủ 65 Main OP đã được sinh lại trong `data/` và kiểm chứng lại từng dòng
(xem [`WIRE_FORMATS.md` §1](./WIRE_FORMATS.md)).

---

## 5. ⭐ PHÁT HIỆN SỐ 3 — ĐÃ CHỐT LAYOUT GÓI HỌC KỸ NĂNG `0x1C 01` (Round-7)

✅ **VERIFIED** — builder `FUN_005990E0` (file `005990e0_FUN_005990e0.asm.txt`, dump mới ở Round-7):

```
005991df: MOV EDX,dword ptr [EAX + 0xE4]     ; skillId
005991f9: CALL 0x0077eb1c                    ; -> 2 byte skillId LITTLE-ENDIAN
00599212: MOV EDX,dword ptr [EAX + 0x108]    ; level hiện tại
00599222: ADD EDX,0x1                        ; LEVEL MỤC TIÊU = level + 1
0059923d: CALL 0x004040d4                    ; @LStrCat3(dest, skillId, level)
00599249: MOV CL,0x1                         ; sub = 01
0059924b: MOV DL,0x1c                        ; main = 1C
```

> ### 🔴 **Payload `0x1C 01` = `{ skillId : u16 LITTLE-ENDIAN }{ targetLevel : u8 }` — 3 byte, ID TRƯỚC VALUE**
>
> ⇒ **KHỚP CHÍNH XÁC** server Bear (`read16(data,i)` + `data[i+2]`) và server Rust (`skills.rs:114-120`).
> ⇒ Claim cũ (G-05 trong `light_dark_skill_unlock_gap.md`): "client gửi `[level u8][skillId u16]`" là **SAI**. **G-05 ĐÓNG.**

Kèm theo (cùng Round-7):

| Phát hiện | Mức |
|---|:-:|
| `0x1C 02` (học nhiều skill) = `{ count u8 }{ skillId u16 LE }{ targetLevel u8 }` | ⚠ INFERRED |
| `0x1C 06` (kỹ năng tàu) = `{ skillId u16 LE }` | ✅ |
| **Gate 28 skill id "gốc hệ"** (`10020-10026`, `11020-11026`, `12020-12026`, `13021-13027`) bị chặn khi `player+0x4B0 == 0`, thông báo *"Đẳng cấp vẫn chưa học chuyển sinh"* | ✅ |
| 4 chuỗi gate hệ: *"Thuộc tính Hỏa không thể học kỹ năng Địa hệ"* (và 3 nhánh còn lại) | ✅ |
| Chuỗi *"Đẳng cấp từ %d nâng lên %d"* — format string của **nâng cấp kỹ năng** | ✅ |
| Chuỗi *"Chuyển sinh chuyên dụng"* — tên 1 skill trong hệ Tâm | ✅ |
| UI **"Điểm số"** vs **"Điểm thăng cấp"** — 2 loại điểm kỹ năng khác nhau | ✅ |

Chi tiết: [`ROUND7_RESULTS.md`](./ROUND7_RESULTS.md).

---

## 6. ⭐ PHÁT HIỆN SỐ 4 — ĐÃ TÌM RA HÀM XÂY PAYLOAD `0x1C 05` (Round-8)

✅ **VERIFIED** — `gvar_007D9E68` là form **`TSe_SkillTreeForm`**
(RTTI field table `0x577A00`: field `published` ở `+0x25C`, `+0x288`, `+0x28C`, `+0x290`;
tên lớp `"TSe_SkillTreeForm"` tại `0x577A1E`).

⭐ **Hàm xây payload = `FUN_0057C840`** — đọc trực tiếp từ `aLogin.exe`:

```asm
0057C860: ADD EAX,0x28C ; CALL 0x00402CF0   ; FillChar(self+0x28C,?,0)
0057C87B: ADD EAX,0x290 ; CALL 0x00402CF0   ; FillChar(self+0x290,?,0)
0057C894: CMP word [player+0x63A],0x29CD     ; map id 10637
```

⚠ **Hàm này KHÔNG có trong corpus Ghidra** ⇒ đã đưa vào **Round-9 Mục A** (P0).
Xem [`ROUND8_RESULTS.md`](./ROUND8_RESULTS.md) §2.

⇒ **⇒ Đây là lý do chính xác** vì sao `0x1C 05` (tái sinh lần 2) vẫn chưa chốt được.

---

## 7. ⭐⭐⭐ PHÁT HIỆN SỐ 5 — ĐÃ CHỐT TRỌN VẸN PAYLOAD `0x1C 05` (Round-9)

✅ **VERIFIED 2 CHIỀU** — đọc từ client (Ghidra) **và** khớp **100%** với server Bear.

⭐ **Hàm dựng payload = `FUN_0057C840`** (1419 B) — form `TSe_SkillTreeForm`
(RTTI: field `published` ở `+0x24C`, `+0x25C`, `+0x288`, `+0x28C`, `+0x290`).
⭐ **Hàm lắp payload = `FUN_00583A98`** — `numSkill = len(skillList) / 3` ⇒ **3 byte/kỹ năng**.

> ### 🔴 PAYLOAD `0x1C 05` (byte-chính-xác)
> ```
> [1C][05]
> [u32 LE] L1 = len(form+0x288)          ← số "bóng" (ball)
> [L1 byte]  chỉ số ball 1-based (1..12)
> [u32 LE] L2 = số kỹ năng
> [L2×3]     mỗi kỹ năng: { u16 skillId LE, u8 targetLevel }
> [u32 LE] L3 = len(form+0x28C)
> [L3 byte]  entry 3 byte: { u8 slotIndex (6..13), u16 skillId LE }
> ```

✅ Server Bear `TSCharacter.setSkillRb2()` đọc **đúng y hệt** (3 khối u32-LE),
và `skill_rb2[data[num] - 6] = read16(...)` ⇒ client gửi slot 6..13 ⇒ server slot 0..7
⇒ **8 slot `skill_rb2` = 4 nhánh nghề × 2 slot** ✓

⇒ **⇒ Round-9 đã đóng được "khoảng trống" lớn nhất của bundle.**
Xem [`ROUND9_RESULTS.md`](./ROUND9_RESULTS.md) §2 và [`WIRE_FORMATS.md` §6](./WIRE_FORMATS.md).

---

## 8. ⚠️ PHÁT HIỆN SỐ 6 — HAI KHÔNG GIAN ID KỸ NĂNG (đã chốt lại ở §10)

> ⚠️ **Mục này đã bị §10 thay thế.** Bản tài liệu đầu tiên (dựa trên `Data/Skill.Dat`
> bản 352 record) kết luận *"id wire = id `Skill.Dat`"*. Sau khi bạn cập nhật
> `Data/Skill.Dat` lên **376 record** và đối chiếu với **hằng số hard-code trong
> `aLogin.exe`**, kết luận đã **đảo lại**:
> ⭐ **ID wire = id trong `Data/skills.json`** ⇒ xem **§10**.

## 9. ⭐⭐ PHÁT HIỆN SỐ 7 — `0x1C 02` LÀ **HỌC KỸ NĂNG CHO THÚ**, KHÔNG PHẢI "HỌC NHIỀU" (Round-10)

> 🔴 **ĐÃ SỬA claim Round-7.** Byte đầu của `0x1C 02` **KHÔNG phải `count`** mà là
> **chỉ số thú (1..4)**.

```asm
005848c5: MOV EDX,dword ptr [EBP-0xc]     ; ⭐ petSlot
005848c8: CALL 0x00403fa0                 ; @LStrFromChar(shared, petSlot)
...
005849cf: MOV CL,0x2 / MOV DL,0x1c / CALL 0x0077f414   ; ⭐ GỬI 0x1C 02
```

✅ `FUN_0071D234(player, X)` = **tìm slot thú** theo `thú->0x55C` (loài), trong
`player->0x57C[1..4]` — **KHÔNG** phải hàm đếm.
✅ Khớp **100%** Bear `ModifySkillHandler.cs`: `pet[data[2]-1].setSkill(...)`.
✅ Server Rust `handle_pet_skill_upgrade` **đã đúng** (chỉ tài liệu là sai).

> ### 🔴 PAYLOAD `0x1C 02`
> ```
> [1C][02]
> [ petSlot : u8 ]                  ← chỉ số thú 1..4
> [ skill : u16 LE ][ lvl : u8 ]     (tối đa 3 skill / lần gửi)
> ```

---

## 10. ⭐⭐⭐ PHÁT HIỆN SỐ 8 — `Data/skills.json` MỚI LÀ `Skill.Dat` ĐÚNG BẢN `aLogin.exe` (đã đảo lại lần 3)

> ### 🔴 ID WIRE = ID TRONG `Data/skills.json` (376 record) — **KHÔNG** phải id trong `Data/Skill.Dat`
>
> **Bằng chứng cứng**: binary `aLogin.exe` **chỉ chặn đúng 4 id** (không phải dải 7):
> ```asm
> 0065394f: SUB EAX,0x2724   ; → 10020  ⭐ "Địa Động Quyết"  → chặn
> 0065395a: SUB EAX,0x3e8    ; → 11020  ⭐ "Băng Tâm Quyết"  → chặn
> 00653965: SUB EAX,0x3e8    ; → 12020  ⭐ "Hỏa Dương Quyết" → chặn
> 00653970: SUB EAX,0x3e7    ; → 13019  ⭐ "Phong Ngạo Quyết" → chặn
> ```
> ⇒ **4/4 id này trong `skills.json` đều là "Tâm pháp gốc"** (`skillType 19`, `skPoint 1`)
> ⇒ và **14/18** id còn lại mà binary hard-code **chỉ có trong `skills.json`**, không có trong `Skill.Dat`.
>
> ⇒ **`Data/Skill.Dat` là build KHÁC** (cùng 376 kỹ năng, đánh số lệch ±16/±48).

### 10.1 ⭐ AUDIT `Data/Skill.Dat` (bản 376 record — mới cập nhật)

| Nhóm | Kết quả |
|---|---|
| Tổng record | **376** (= `skills.json` 376) ✅ |
| **16 kỹ năng 4 nhánh nghề** | ✅ **16/16** — bản cũ (352) **thiếu 8 id** Hiệp/Hiền ⇒ **"id đang thiếu" ĐÃ ĐƯỢC BỔ SUNG** |
| 4 "Tâm pháp gốc" | ✅ 4/4 (Địa 10004, Thủy 11036, Hỏa 11972, Phong 13035) |
| 32 "bóng" (8 × 4 hệ) | ✅ **32/32** |
| Quang / Ám | ✅ đủ |
| Map theo tên ↔ `skills.json` | ✅ **384 cặp, 0 unmatched** (bản cũ: 138 cặp) |
| ⛔ id khớp binary | ❌ **KHÔNG** (build khác) |

⇒ **NỘI DUNG ĐÃ ĐẦY ĐỦ ✅** — chỉ là **khác đánh số id**.

### 10.2 ⭐⭐ ĐÃ DỰNG LẠI `Data/Skill.Dat` TỪ `skills.json` (thay vì "trộn" record)

✅ **Không thể "trộn"** record từ build khác: `require_sk` bên trong mỗi file trỏ theo
không gian id của chính file đó ⇒ trộn sẽ ra file hỗn 2 không gian ⇒ client tra
bảng ngược trượt.

✅ **Cách đúng:** dựng lại từ `skills.json` (chính là `Skill.Dat` của build `aLogin.exe`):

```bash
python3 client_skills_handler/tools/build_skill_dat.py --out Data/Skill.Dat
```

| Kiểm tra | Kết quả |
|---|---|
| Codec VISCII + `encode_item8/16/32` | ⭐ **round-trip byte-for-byte 352/352** |
| Số record | **376** |
| 33 id hard-code của `aLogin.exe` | ✅ **33/33 resolve đúng** |
| Cây `require_sk` | ✅ **217 gốc, 0 node trôi** |

> ### 🔴 BUG server Rust đã tìm & sửa — `src/data/loaders/skill_pc.rs`
> `decode_item8/16/32` dùng **sai thứ tự phép tính**: `v.wrapping_sub(N) ^ K` thay vì
> `(v ^ K).wrapping_sub(N)` (đúng như `SkillData.cs` của Bear server) ⇒ **mọi field** của
> `binary_skill_defs` đều sai. Đã sửa + thêm `tests/skill_dat_rebuild_test.rs` (4 test).
> ⇒ **Toàn bộ test suite vẫn xanh** sau khi sửa.
⇒ Bảng map 100% đầy đủ: [`data/skill_id_map_client_server.tsv`](./data/skill_id_map_client_server.tsv).

> ### ⚠ Tác động lên server Rust
> * `data.skills` được nạp từ **`Data/Skills.txt`** — **file này KHÔNG tồn tại** ⇒ map rỗng
>   ⇒ mọi `data.skills.get(...)` trong `handlers/skills.rs` luôn trả `None`.
> * `skill_pc.rs` nạp `Data/Skill.Dat` vào `binary_skill_defs` — ⛔ **id sai** (build khác),
>   chỉ dùng để tra **tên/mô tả**, không dùng làm id.

---

## 11. ⚠️ PHÁT HIỆN SỐ 9 — BẢNG CÂY KỸ NĂNG `0x94B440` KHÔNG PHẢI "BLOCKER"

Tài liệu cũ (`light_dark_skill_unlock_gap.md` §2.5, §6 #4) nói:

> "Client lấy cây kỹ năng từ bảng hard-code `0x94B440[5 tab][51 slot][3B]`" và
> "`0x94B440` là BLOCKER, cần re-import Ghidra".

**Cả hai đều SAI.** Đã kiểm chứng bằng header section của `client_delphi/aLogin.exe`:

* `0x0094B440` → RVA `0x54B440` → **section BSS** (uninitialised) ⇒ **không có byte nào
  trong file** ⇒ dump Ghidra trả `readable 0` là **đúng bản chất**, không phải thiếu sót.
* Bảng được **dựng lúc runtime** bởi `FUN_005A0388` (gọi ở **cuối** parser `05 03`):
  xoá 765 byte → đăng ký 4 skill gốc hard-code (10001 / 11001 / 12001 / 13001) →
  đăng ký **từng kỹ năng người chơi đã học** qua `FUN_00597954`.
* Byte thứ 3 của mỗi ô là **LEVEL của kỹ năng**, không phải "byte yêu cầu".
* **Index tab = `elem - 1`** ⇒ kỹ năng sống hệ Tâm (elem 5) **CÓ tab riêng (tab 4)** —
  tài liệu cũ nói "element 5 không có tab" là sai.

⇒ Đã **khôi phục lại** cấu trúc bảng: `tools/recover_skill_tree.py` →
`data/skill_tree_tabs.tsv` (255 dòng) + `data/skill_tree_recovered.bin` (765 byte).
Chi tiết: [`BLOCKED_DUMPS.md`](./BLOCKED_DUMPS.md).

---

## 11.5 ⭐⭐⭐ PHÁT HIỆN SỐ 10 — FORM KỸ NĂNG NẠP TỪ **FILE NGOÀI**, VÀ DFM DÙNG **BIG5** (Round-11)

Round-9/10 từng đoán: *"caption tab 5 + tên 4 nhánh nghề nằm trong DFM resource
(`.rsrc`)"*. Round-11 đã dump **toàn bộ `.rsrc`** (83 968 B) và viết
**`tools/parse_dfm.py`** (parser PE resource + DFM binary Delphi 7).

| Kiểm tra | Kết quả |
|---|---|
| Số form DFM trong `aLogin.exe` | **6** — `TForm1..TForm4`, `TGmForm`, `TDelphiXDXPlayForm` |
| Trong đó có form kỹ năng? | ⭐ **KHÔNG** |
| Encoding chuỗi trong DFM | ⭐⭐ **BIG5 (cp950)** — KHÔNG phải VISCII |
| Cách client nạp form kỹ năng | ⭐ **ĐỘNG**: `"{N}Form_SkillTree"` (N=1..7) → `FUN_007C9B38` → tìm **`<tên>.bmp`** trong thư mục data client |

**Bằng chứng encoding Big5** (property `ImeName` trong `TForm1`):

```
bytes : A4 A4 A4 E5 20 28 C1 63 C5 E9 29 20 2D 20 AD DC BE 65
big5  : 中文 (繁體) - 倉頡      ← tên IME Windows
viscii: ấấấă (ÁcĂé) - …       ← rác
```

**Bằng chứng form nạp động** (`FUN_0057922C`):

```asm
005794a1: ADD  EAX,0x1                ; N = tabIndex+1
005794b6: MOV  EDX,0x579a08           ; "Form_SkillTree"
005794bb: CALL 0x004040d4             ; -> "NForm_SkillTree"
005794ca: CALL 0x007c9b38             ; nạp form theo tên
```

⇒ ### ✅ KẾT LUẬN
> **Tên hiển thị 4 nhánh nghề (Bá / Hiệp / Hiền / Tiên) và caption tab 5
> KHÔNG tồn tại trong `aLogin.exe`.** Chúng nằm trong file form-module/resource
> **ngoài** mà client nạp lúc runtime ⇒ cần **thư mục data gốc của client**,
> không suy ra được từ binary. Đây là **kết luận có bằng chứng**.

⇒ Đồng thời Round-11 **đóng** item "`TInterForm+0x152/+0x154`":
`+0x152` = index NPC (u8, 0..255) · `+0x154` = mã NPC (u16, 1..1250) ·
`+0x156` = cờ "đã chọn NPC" ⇒ payload `0x1B 01` =
`{ 0x1B }{ 0x01 }{ npcIndex u8 }{ npcId u16 LE }`.

Chi tiết: [`ROUND11_RESULTS.md`](./ROUND11_RESULTS.md) ·
[`SKILL_DATA_MODEL.md`](./SKILL_DATA_MODEL.md) §5.1.1.

---

## 11.6 ⭐ BACKUP SOURCE VỀ `client_pseudo_c/` (2026-10-22)

Ngoài bundle, toàn bộ source dump của module kỹ năng đã được **copy flat** sang
`client_pseudo_c/` (thư mục corpus làm việc) và `index.csv` đã được cập nhật:

```bash
python3 client_skills_handler/tools/sync_to_pseudo_c.py          # xem thay đổi
python3 client_skills_handler/tools/sync_to_pseudo_c.py --dry-run
```

| Hạng mục | Kết quả |
|---|---|
| File nguồn (`.c` / `.asm.txt` / `.hex`) | **846** |
| File **mới** thêm vào `client_pseudo_c/` | **529** |
| File **trùng byte** (giữ nguyên) | **277** |
| File **chỉ khác xuống dòng** | **3** |
| File **nội dung thực sự khác** → đã cập nhật (bản cũ **không** giữ lại) | **37** |
| `index.csv` | 6 952 → **6 990** dòng (**+38**, không mất dòng nào) |

**Quy tắc chuẩn hoá** (để `client_pseudo_c/` giữ đúng convention của nó):
* `.c` → **CRLF** (100 % file `.c` sẵn có dùng CRLF)
* `.asm.txt` / `.hex` → **LF** (đa số)
* file đã tồn tại → giữ **style riêng** của nó ⇒ file không bị ghi lại vô ích.

⇒ Sau khi copy: **846/846** file khớp 100 % về nội dung (đã chuẩn hoá xuống dòng),
**0** file thiếu, **0** file khác nội dung.

> Tool **không** tạo file `___old` trừ khi chạy kèm `--keep-old`.

---

## 12. Bản đồ kỹ năng ↔ opcode ↔ hàm (tiền phân tích)

> ⚠ **Bản đồ tiền phân tích** — lập bằng xref thô trên binary, **không thay thế** cho
> kết luận cuối của agent nghiên cứu. Mọi dòng đều kèm địa chỉ để tự kiểm chứng.

### 5.1 C→S (client → server)

| Mục đích | Main OP | Sub | Builder | File trong bundle |
|---|---|:-:|---|---|
| **Học / nâng cấp kỹ năng** (skill tree NPC) | `0x1C` | `01` | `0x00785DD7` | `asm_cs1c_785dbb_jmptbl.asm.txt` |
| **Nâng cấp kỹ năng thú cưng** | `0x1C` | `02` | `0x00785E45` | ↑ |
| (dự phòng) | `0x1C` | `04` | `0x00785EB3` | ↑ |
| **Tái sinh lần 2 / chuyển sinh (bóng + skill_rb2)** | `0x1C` | `05` | `0x00785F21` | ↑ |
| **Kỹ năng tàu (ship)** | `0x1C` | `06` | `0x00785F8F` | ↑ |
| **Học 1 skill từ NPC thầy** | `0x1B` | `01` | `0x00786011` | `0077f414_FUN_0077f414.asm.txt` |
| (dự phòng) | `0x1B` | `02` | `0x007860C8` | ↑ |
| **Dùng vật phẩm** (sách kỹ năng, bùa) | `0x17` | `0F` | `0x00783B81` | `asm_cs17_783405_builders.asm.txt` |
| **Hotkey / thanh kỹ năng** | `0x28` / `0x29` | — | `0x0078820F` | `asm_cs29_788288_builders.asm.txt` |
| **Dùng kỹ năng trong trận** | `0x0B` | `02/03/04` | `0x00780595…` | `asm_cs0b_78048d_builders.asm.txt` |
| *(code chết)* dùng kỹ năng theo id | `0x32` | `01` | `0x00788C38` | `0077f414_FUN_0077f414.asm.txt` |

### 5.2 S→C (server → client)

| Mục đích | Main OP | Sub | Handler | File trong bundle |
|---|---|:-:|---|---|
| **Danh sách kỹ năng + chỉ số** | `0x05` | `03` | `FUN_0072E9A0` | `0072e9a0_FUN_0072e9a0.*` |
| **Cập nhật 1 kỹ năng** (prop `0x6E`) | `0x08` | `01` | `FUN_00710AB0` | `00710ab0_FUN_00710ab0.*` |
| Cập nhật kỹ năng của actor khác | `0x08` | `02` | `FUN_00710AB0` | `case_009_0078D125_FUN_0078d125.c` |
| Cập nhật kỹ năng theo index | `0x08` | `04` | `FUN_0073B458` | `0073b458_FUN_0073b458.*` |
| **Danh sách kỹ năng tàu (4 slot)** | `0x08` | `05` | `FUN_0074C340` | `0074c340_FUN_0074c340.*` |
| Slot bộ phận tàu | `0x08` | `06` / `07` | `FUN_0074C4C8` / `FUN_0074CC84` | ↑ |
| Thông báo lên cấp (+ điểm kỹ năng) | `0x05` | `0A` | `FUN_0072F970` | `0072f970_FUN_0072f970.c` |
| *(mơ hồ)* NPC dạy skill / bạn vừa học | `0x20` | `01` / `02` | `FUN_0079285A` | `case_028_0079285A_FUN_0079285a.c` |
| **Xoá toàn bộ 3000 ô kỹ năng** | `0x19` | `0B` | `FUN_0075B8E0` | `0075b8e0_FUN_0075b8e0.*` |

### 5.3 Logic phía client (không qua mạng)

| Mục đích | Hàm | File trong bundle |
|---|---|---|
| **Kiểm tra "đã học kỹ năng chưa"** (switch theo id kỹ năng, 20 entry) | `FUN_006538BC` | `006538bc_FUN_006538bc.asm.txt` |
| **Kiểm tra hệ khi dùng kỹ năng** (`elem==5` ⇒ không giới hạn) | `FUN_0076D578` | `asm_76d578_full.asm.txt` |
| **Tooltip kỹ năng** (gọi `FUN_00759B04` + `FUN_0075AB00` + `FUN_0075AB54`) | `FUN_0076D578` | ↑ |
| **Render cây kỹ năng** (bảng runtime `0x94B440`, 5 tab × 51 slot) | `FUN_0059F090` | `0059f090_FUN_0059f090.*` |
| ⭐ **Xây payload `0x1C 04/05`** (⚠ chưa có trong corpus) | `FUN_0057C840` | ⭐ **Round-9 Mục A** |
| ⭐ **Dialog nâng cấp kỹ năng** (4386 B) | `FUN_005A7970` | `005a7970_FUN_005a7970.*` |
| ⭐⭐ **Dựng payload `0x1C 04/05`** | `FUN_0057C840` + `FUN_00583A98` | `0057c840_*`, `asm_0057c840_*` |
| ⭐ **Dựng cây / click node** (form NPC skill) | `FUN_0057DD0C`, `FUN_0057EE28` | `0057dd0c_*`, `0057ee28_*` |
| ⭐⭐ **Học kỹ năng cho THÚ** (`0x1C 02`) | `FUN_0058484C`, `FUN_0071D234` | `0058484c_*`, `0071d234_*` |
| ⭐ **Form cây kỹ năng NPC** (`TSe_SkillTreeForm`) | `FUN_00577CCC` + 22 method | `00577ccc_*`, `0057*` |
| **Dựng lại bảng cây kỹ năng** (gọi ở cuối parser `05 03`) | `FUN_005A0388` | `005a0388_FUN_005a0388.*` ★★ |
| **Đăng ký 1 kỹ năng vào cây** (tab = elem-1) | `FUN_00597954` | `00597954_FUN_00597954.*` ★★ |
| Tra cứu skillId → index | `FUN_00759B04` | `00759b04_FUN_00759b04.*` |
| 8 accessor của record `TFSkill` (86 byte) | `0x0075A948…0x0075AB54` | `asm_tfskill_accessors.asm.txt` |
| Nạp `Skill.Dat` + dựng bảng tra ngược `0x9AE128` | `FUN_00759B2C` / `FUN_00759DF0` | `00759b2c_*`, `00759df0_*` |
| **Check "đã học" khi click NPC** (id `0x36C7`, `0x36D2`) | `FUN_0073D558` | `0073d558_FUN_0073d558.c` |
| **Danh sách kỹ năng của actor** (21 slot) | `FUN_006546E0` | `006546e0_FUN_006546e0.*` |

---

## 13. Bốn nhánh nghề sau chuyển sinh (đã xác minh)

> ⭐ **Thứ tự hiển thị trong client TS Online: BÁ → HIỆP → HIỀN → TIÊN**
> (khác thứ tự `job` của server: Hiệp=1, Bá=2, Hiền=3, Tiên=4)

Từ `TS_Server_Bear/.../TSCharacter.cs :: setallskillForJob()` (chi tiết ở
[`SKILL_DATA_MODEL.md` §4](./SKILL_DATA_MODEL.md)):

| `job` | Nhánh nghề | 4 kỹ năng (id / tên / level cấp) |
|:-:|---|---|
| **1** | **Hiệp** | 14043 *Anh khí hộ thân* (10) · 14044 *ThiênThần thuẫn* (5) · 14045 *Tâm nhãn* (10) · 14046 *NgưngKhí HộThân* (5) |
| **2** | **Bá** | 14039 *Bá khí* (10) · 14040 *Bá ý* (5) · 14041 *Lực bạt sơn hề* (10) · 14042 *Sóng cuồng* (5) |
| **3** | **Hiền** | 14047 *Dùng kế* (10) · 14048 *Trí thủ* (10) · 14049 *Chống địch* (10) · 14050 *Sách động* (10) |
| **4** | **Tiên** | 14051 *Tiên khiếu* (10) · 14052 *HóaHuyếtHòanHồn* (5) · 14053 *ThánhLinh thuật* (5) · 14054 *CànKhônThiênLôi* (5) |

Tất cả đều yêu cầu tiền đề **14038 *Đại chu thiên***.

---

## 14. Quy ước đọc file

* Mỗi file `.c` bắt đầu bằng header:
  ```
  // Function: FUN_007XXXXX
  // Entry:   007XXXXX
  // Size:    N bytes
  // Callers: ...
  // Callees: ...
  // References to entry: 007XXXXX -> 007XXXXX [UNCONDITIONAL_CALL]
  ```
  → dùng `Callers`/`Callees` để dựng call-graph, **đừng** đọc tuần tự toàn bộ corpus.
* File `.asm.txt` trong `ts_decompile/functions/` **không** có tiền tố địa chỉ (chỉ có trong
  `client_pseudo_c/`); cả hai đều dùng hex không zero-pad (`0x51b890`).
  ⇒ **Khi có cả hai bản, ưu tiên bản có tiền tố địa chỉ** (`client_pseudo_c`).
* Chuỗi literal: `lit_<start>_<end>.hex` — dòng đầu `; 0xAAAAAAA..0xBBBBBBB`, sau đó mỗi
  dòng 16 byte hex. Dùng `tools_decode_literals.py` (`.scratch/client-pseudo-op-code/`).
* Bảng tra S→C: `jumptable_byte200_0x78A8EE.hex` (Main OP → index case),
  `jumptable_dword200_0x78A9B6.hex` (index case → địa chỉ hàm).
* Bảng tra C→S: `cs_bytemap_77f474_200B.hex` / `table_0x77F474_200B.hex` (Main OP → index builder),
  `cs_builder_dword200_0x77f53c.hex` / `table_0x77F53C_dword200.hex` (index builder → địa chỉ).
* Bảng cây kỹ năng: `skilltree_94b440_765B.hex` — ⚠ **file rỗng (readable 0)**, xem `MISSING_DUMPS.md`.
* Mọi địa chỉ trong tài liệu này là **Virtual Address** (image base `0x00400000`).

---

## 15. Cảnh báo về độ tin cậy

Mọi claim trong `README.md` / `SKILL_DATA_MODEL.md` / `WIRE_FORMATS.md` đều kèm mức độ:

| Mức | Ý nghĩa |
|:-:|---|
| ✅ **VERIFIED** | đã đối chiếu byte trong ASM hoặc trong file `Data/` |
| ⚠️ **INFERRED** | suy luận từ ≥ 2 nguồn độc lập |
| ❓ **OPEN** | chưa đóng — agent nghiên cứu phải tự tra |

Tài liệu cũ ở `reference/analysis_docs/` **có chứa claim đã bị bác bỏ** (xem §3, §4).
Khi agent nghiên cứu, hãy coi mọi claim trong đó là **giả định cần kiểm chứng lại**, không
phải sự thật.
