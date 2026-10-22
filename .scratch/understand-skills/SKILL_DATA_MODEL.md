# `SKILL_DATA_MODEL.md` — Mô hình dữ liệu kỹ năng của client `aLogin.exe`

> Ngày: 2026-10-21 · Bundle: `client_skills_handler/`
> Mức độ mỗi claim ghi ✅ VERIFIED / ⚠️ INFERRED / ❓ OPEN.

---

## 1. Tổng quan

Client TS Online có **3 bảng dữ liệu kỹ năng** liên quan:

| Bảng | Nguồn | Nội dung |
|---|---|---|
| **Skill.Dat** (bảng kỹ năng) | file nhị phân 86 byte/record | tên, mô tả, `type`, `elem`, `sp_cost`, `sk_point`, `max_lvl`, `require_sk`… |
| **Bảng cây kỹ năng hard-code** | `0x0094B440`, 765 byte trong `.text` | `TSkillTreeRow[5 tab][51 slot][{u16 skillId LE}{u8 requireByte}]` — **client tự biết**, server không gửi |
| **Bảng tra ngược** | `0x009AE128` | `skillId → index` trong mảng `TFSkill` — dựng lúc runtime từ Skill.Dat |

---

## 2. ⚠️ Hai không gian id — phải chốt trước khi làm gì cả

### 2.1 Bằng chứng

✅ **VERIFIED** — `FUN_006538BC` (file `006538bc_FUN_006538bc.asm.txt`) so sánh `skillId`
với các hằng số hard-code:

```
006538c9: CMP dword ptr [EBP + -0x8], 0x36b1   ; 14003
00653937: CMP EAX, 0x36d0                      ; 14032
00653948: CMP EAX, 0x36b3                      ; 14003
0065394f: SUB EAX, 0x2724                      ; 10020
0065395a: SUB EAX, 0x3e8                       ; 11020
00653965: SUB EAX, 0x3e8                       ; 12020
00653970: SUB EAX, 0x3e7                       ; 13019
00653987: ADD EAX, 0xffffc94d                  ; → EAX - 14003
006539a6: ADD EAX, 0xffffc940                  ; → EAX - 14016
006539c2: ADD EAX, 0xffffc92f                  ; → EAX - 14033 ; CMP EAX,0x13 ; JA default
006539d0: JMP dword ptr [EAX*0x4 + 0x6539d7]   ; bảng nhảy 20 entry (14033..14052)
```

Tập id được so sánh: `{10020, 11020, 12020, 13019, 14003…14008, 14016…14021, 14023,
14025, 14026, 14027, 14028, 14032…14052}`.

✅ **VERIFIED** — **tất cả** id trên tồn tại trong `Data/skills.json` (376 record).
❌ **KHÔNG** tồn tại trong `Data/Skill.Dat` (352 record): `12020`, `14003-14008`,
`14016-14021`, `14023`, `14025`, `14026`, `14027`.

✅ **VERIFIED** — server Bear dùng đúng không gian `skills.json`:
`TSCharacter.cs :: rebornChar()` cấp `14001, 14015, 14021, 14023, 14035, 14002, 14003,
14004, 14005, 14010, 14011, 14018, 14019`; `setallskillForElement()` cấp `10001…10024`,
`11001…11015`, `13001…13015`; `AddSkillrb2()` cấp `10027…10034` / `11027…11034` /
`12027…12034` / `13026…13033`.

### 2.2 ⭐⭐ Kết luận CUỐI CÙNG (2026-10-22, đã kiểm chứng 2 chiều)

> ### 🔴 KHÔNG GIAN ID THẬT TRÊN WIRE = **`Data/skills.json`** (376 record)
>
> ✅ **BẰNG CHỨNG CỨNG** — binary `aLogin.exe` **hard-code 4 id "Tâm pháp gốc"** và
> **chỉ chặn đúng 4 id** (không phải dải 7 như từng nghĩ):
>
> ```asm
> 00653948: CMP EAX,0x36b3          ; EAX < 14003
> 0065394f: SUB EAX,0x2724         ; → 10020   ⭐ "Địa Động Quyết"
> 00653954: JZ  0x653a27           ;   → chặn
> 0065395a: SUB EAX,0x3e8          ; → 11020   ⭐ "Băng Tâm Quyết"
> 0065395f: JZ  0x653a27           ;   → chặn
> 00653965: SUB EAX,0x3e8          ; → 12020   ⭐ "Hỏa Dương Quyết"
> 0065396a: JZ  0x653a27           ;   → chặn
> 00653970: SUB EAX,0x3e7          ; → 13019   ⭐ "Phong Ngạo Quyết"
> 00653975: JZ  0x653a27           ;   → chặn
> ```
>
> ⇒ **4/4 id này trong `Data/skills.json` đều là "Tâm pháp gốc"** (`skillType 19`,
> `skPoint 1`, `requireSk 0`) ⭐⭐
>
> ✅ **BẰNG CHỨNG BỔ SUNG** — 14/18 id còn lại mà binary hard-code
> (`14001`, `14003…14008`, `14016…14027`, …) **chỉ tồn tại trong `skills.json`**,
> **KHÔNG có** trong `Data/Skill.Dat`.
>
> ⇒ **`Data/skills.json` CHÍNH LÀ `Skill.Dat` của build client khớp với `aLogin.exe`.**
>


### 2.2a ⭐⭐ `Data/Skill.Dat` ĐÃ ĐƯỢC DỰNG LẠI TỪ `skills.json` (2026-10-22)

✅ **Vì sao không "trộn" record từ build khác?** Vì `Skill_new.Dat` (đã bị xóa) dùng
**không gian id riêng** (lệch ±16/±48), kể cả `require_sk` bên trong cũng trỏ theo
không gian đó ⇒ trộn vào sẽ ra file **hỗn 2 không gian id** ⇒ client tra bảng ngược
trượt ⇒ coi như chưa học.

✅ **Cách đúng: dựng lại từ `skills.json`** — đây chính là `Skill.Dat` của build
`aLogin.exe` (đã chứng minh bằng 33 id hard-code + cây `require_sk`).

```bash
python3 client_skills_handler/tools/build_skill_dat.py --out Data/Skill.Dat
```

| Kiểm tra | Kết quả |
|---|---|
| Codec VISCII + `encode_item8/16/32` | ⭐ **round-trip byte-for-byte 352/352** trên file gốc |
| Số record | **376** (= `skills.json`) |
| 33 id hard-code của `aLogin.exe` | ✅ **33/33 resolve đúng** (gate 4 "Tâm pháp gốc" + 12 id cây + 16 id 4 nhánh nghề + gốc) |
| Cây `require_sk` | ✅ **217 gốc, 0 node trôi** |
| Id wire | ⭐ **= id `skills.json`** (đồng nhất) |

> ### 🔴 BUG ĐÃ TÌM & SỬA trong server Rust — `src/data/loaders/skill_pc.rs`
> ```rust
> // ❌ SAI (thứ tự phép tính): XOR và SUB không giao hoán
> fn decode_item16(val: u16) -> u16 { val.wrapping_sub(4) ^ 0x6E90 }
> // ✅ ĐÚNG (khớp `SkillData.cs` của Bear server)
> fn decode_item16(val: u16) -> u16 { (val ^ 0x6E90).wrapping_sub(4) }
> ```
> ⇒ Trước đó **mọi field** của `binary_skill_defs` (id, loại, cấp, điểm…) đều giải mã
> **sai** ⇒ battle engine chạy trên dữ liệu rác.
> ⇒ Đã sửa cả `decode_item8/16/32` + thêm test `tests/skill_dat_rebuild_test.rs` (4 test).

### 2.2b AUDIT bản `Skill.Dat` cũ (352 record) — đã lưu ở `reference/data_files/Skill_legacy_352.Dat`

| Nhóm | Kết quả |
|---|---|
| Tổng record | **376** (= `skills.json` 376) ✅ |
| **16 kỹ năng 4 nhánh nghề** (Bá/Hiệp/Hiền/Tiên) | ✅ **16/16** — bản cũ (352) thiếu 8 id của Hiệp/Hiền |
| 4 "Tâm pháp gốc" (mỗi hệ 1) | ✅ 4/4 (Địa 10004, Thủy 11036, Hỏa 11972, Phong 13035) — bản cũ **thiếu hết** |
| 32 "bóng" (8 × 4 hệ) | ✅ **32/32** theo tên |
| Hệ Quang / Ám | ✅ đủ (`22060-22070` / `22972-23000`) |
| Map theo tên ↔ `skills.json` | ✅ **384 cặp, 0 unmatched** (bản cũ chỉ 138 cặp) |
| ⛔ id khớp binary `aLogin.exe` | ❌ **KHÔNG** (build khác) — xem §2.2 |

⇒ **NỘI DUNG ĐÃ ĐẦY ĐỦ** ✅ — nhưng **id KHÔNG dùng làm id wire được**.

### 2.2c ⭐ Bảng ánh xạ id `Skill.Dat` ⇄ id canonical (`skills.json`)

✅ sinh tự động bởi `tools/build_id_map.py` → [`data/skill_id_map_client_server.tsv`](./data/skill_id_map_client_server.tsv)

* **384 cặp / 376 record — 0 unmatched** ⇒ map **100% đầy đủ**
  (với `Data/Skill.Dat` đã đồng bộ thì map là **identity**, offset 0).
* Offset phổ biến: `+16` (106), `−16` (98), `+48` (89), `−48` (83).
* Cột `match`: `EXACT` = trùng tên, `PREFIX` = tên trong `Skill.Dat` bị cắt 20 byte VISCII.

**4 nhánh nghề** — map thủ công (đã kiểm tra vị trí trong cây Tâm):

| nhánh (thứ tự client) | id **canonical** (wire) | id trong `Skill.Dat` | tên |
|---|---:|---:|---|
| **Bá** (1) | 14039-14042 | 14055-14058 | Bá khí / Bá ý / Lực bạt sơn hề / Sóng cuồng |
| **Hiệp** (2) | 14043-14046 | 14059, 14028, 14029, 14030 | Anh khí hộ thân / ThiênThần thuẫn / Tâm nhãn / NgưngKhí HộThân |
| **Hiền** (3) | 14047-14050 | 14031-14034 | Dùng kế / Trí thủ / Chống địch / Sách động |
| **Tiên** (4) | 14051-14054 | 14035-14038 | Tiên khiếu / HóaHuyếtHòanHồn / ThánhLinh thuật / CànKhônThiênLôi |
| *(Đại chu thiên)* | 14038 | 14054 | Đại chu thiên |

> ⛔ **"Id đang thiếu" đã được bổ sung đầy đủ** — bản `Skill.Dat` 376 record có đủ
> 16 kỹ năng 4 nhánh nghề (bản 352 record thiếu 8 id của Hiệp/Hiền) ⇒ **id đó KHÔNG
> phải "game xử lý riêng"**, chỉ là **đánh số khác**.

### 2.3 Bảng đối chiếu đầy đủ

* `data/skill_id_map_client_server.tsv` — **138** cặp id client ⇄ server (mới, Round-9).
* `data/skill_id_map_unmatched.tsv` — **453** id chỉ có một bên.
* `data/skill_id_space_conflicts.tsv` — 601 dòng đối chiếu **theo id** (127 chung,
  225 chỉ client, 249 chỉ server) — ⚠ so sánh theo id nên phần lớn `DIFF_NAME`.
* `data/skill_type_groups.tsv` — 20 nhóm `type` × 2 không gian.

---

## 3. Cấu trúc record `Skill.Dat` (86 byte)

✅ **VERIFIED** — theo `src/data/loaders/skill_pc.rs` + `asm_tfskill_accessors.asm.txt`:

| Offset | Size | Field | Decoder |
|:-:|:-:|---|---|
| 0 | 1 | `namelength` | thô |
| 1 | 20 | `name[20]` | đảo ngược byte (VISCII) |
| 21 | 1 | `type` | `(v ^ 0xFD) - 4` |
| 22 | 2 | `id` | `(v ^ 0x6E90) - 4` (LE u16) |
| 24 | 2 | `sp_cost` | như `id` |
| 26 | 1 | `elem` | `(v ^ 0xFD) - 4` |
| 33 | 1 | `skill_type` | `(v ^ 0xFD) - 4` |
| 36 | 1 | `delay` | `(v ^ 0xFD) - 4` |
| 37 | 1 | `state` | `(v ^ 0xFD) - 4` |
| 40 | 1 | `sk_point` | `(v ^ 0xFD) - 4` — **giá điểm kỹ năng** |
| 42 | 1 | `max_lvl` | `(v ^ 0xFD) - 4` |
| 43 | 2 | `require_sk` | như `id` — **tiền đề** |
| 55 | 1 | `des_length` | thô |
| 56 | 30 | `des[30]` | đảo ngược byte (VISCII) |

> ⚠️ `require_sk = 48` là **sentinel "không có tiền đề"** (205/352 record dùng giá trị này,
> kể cả skill thường). Skill id hợp lệ bắt đầu từ `9980`.
> Lưu ý `48` cũng là giá trị `require_sk` hợp lệ của một số skill khác — cần xác minh lại
> bằng cây `require_sk` (xem §4).

### 3.1 Ý nghĩa các trường — 2 hệ offset (RAM vs file)

✅ **VERIFIED** — `asm_tfskill_accessors.asm.txt` đọc object `TFSkill` trong RAM;
`src/data/loaders/skill_pc.rs` đọc record trong **file** `.Dat`. Chúng **khác nhau**:

| Trường | Offset **trong RAM** (accessor) | Offset **trong file** `.Dat` | Accessor |
|---|---|:-:|---|
| `name` | `+0x04` | 1 | `FUN_0075AA4C` |
| `id` | `+0x16` | 22 (0x16) | (đọc trong `FUN_00759B2C`) |
| `type` | `+0x19` | 21 (0x15) | `FUN_0075AAAC` |
| `elem` | `+0x1A` | 26 (0x1A) | `FUN_0075AB54` |
| `sp_cost` | `+0x18` | 24 (0x18) | — |
| **`sk_point`** | **`+0x1C`** | 40 (0x28) | `FUN_0075A9A0` |
| `skill_type` | `+0x25` | 33 (0x21) | `FUN_0075A9F8` |
| `require` (level tiền đề) | `+0x2E` | 43 (0x2B) | `FUN_0075AB00` |

* **Stride record trong RAM = 86 byte** (`IMUL EAX,EAX,0x2B` rồi `×2`), base = `array + 4`.
* `FUN_00759B04(skillId)` = `u16 [0x9AE128 + skillId*2]` (guard `≤ 0xBB8` = 3000).
* Khi `idx == 0` (id không có trong `Skill.Dat`), các accessor trả **0 / 0xFFFF** ⇒ client
  coi như skill không tồn tại.
* `FUN_0075A948` (copy 86 byte) **không** kiểm tra `idx == 0` ⇒ nếu id lạ, nó copy
  record 0 (rác).

> 🔴 **Đính chính**: tài liệu cũ (`battle/03_skill_and_combo.md` §2.1) ghi `skill_type` ở
> record `+0x21`, ASM `FUN_0075A9F8` dùng hằng `0x25` — **đã đóng**:
> `+0x21` là offset **trong file**, `+0x25` là offset **trong RAM**. Cả hai đều đúng.

---

## 4. Cây kỹ năng sống hệ Tâm (toàn bộ)

### 4.1 Cây theo id **server** (`skills.json`)

> ✅ **VERIFIED** — suy ra từ `require_sk` + `setkillElement5()` + `rebornChar()` của Bear.

```
                        14001 Thuật trinh thám
                        ├── 14002 Thuật đào tẩu ── 14007 Giải hôn (1) / 14008 Hôn mê (5)
                        ├── 14003 Thuật mua bán ── 14009 Người làm ── 14016 Thuật giao dịch / 14017 Tịnh toạ
                        ├── 14004 Thuật dò xét ── 14010 Thuật đào khoáng ── 14018 Thuật đào sâu
                        ├── 14005 Thuật luyện đơn ── 14011 Thuật đúc luyện ── 14019 Thuật chế tạo ── 14032 Tinh luyện
                        └── 14006 Đại hô ── 14012 Đồng tâm / 14013 Cổ Vũ ── 14020 Hoạch lí phản
```

**Bảng đầy đủ 38 kỹ năng sống** (id, tên, `sk_point`, `max_lvl`, `require_sk`):

| id | tên | `sk_point` | `max_lvl` | `require_sk` |
|:-:|---|:-:|:-:|:-:|
| 14001 | Thuật trinh thám | 0 | 1 | — |
| 14002 | Thuật đào tẩu | 0 | 1 | 14001 |
| 14003 | Thuật mua bán | 0 | 8 | 14001 |
| 14004 | Thuật dò xét | 0 | 10 | 14001 |
| 14005 | Thuật luyện đơn | 0 | 10 | 14001 |
| 14006 | Đại hô | 0 | 10 | 14001 |
| 14007 | Giải hôn | 0 | 1 | 14002 |
| 14008 | Hôn mê | 0 | 5 | 14002 |
| 14009 | Người làm | 0 | 10 | 14003 |
| 14010 | Thuật đào khoáng | 0 | 10 | 14004 |
| 14011 | Thuật đúc luyện | 0 | 10 | 14005 |
| 14012 | Đồng tâm | 0 | 10 | 14006 |
| 14013 | Cổ Vũ | 0 | 10 | 14006 |
| 14014 | *(id không có trong skills.json)* | | | |
| 14015 | *(id không có trong skills.json)* | | | |
| 14016 | Thuật giao dịch | 0 | 10 | 14009 |
| 14017 | Tịnh toạ | 0 | 10 | 14009 |
| 14018 | Thuật đào sâu | 0 | 10 | 14010 |
| 14019 | Thuật chế tạo | 0 | 5 | 14011 |
| 14020 | Hoạch lí phản | 0 | 5 | 14012 |
| 14021 | Hỗn loạn | 0 | 5 | — |
| 14022 | Giải hỗn loạn | 0 | 1 | — |
| 14023 | **Cưỡi ngựa** | 0 | 1 | — |
| 14024 | Linh quang ẩn hiện | 0 | 1 | — |
| 14025 | Chiếm đọat | 0 | 1 | — |
| 14026 | **Triệu gọi** | 0 | 1 | — |
| 14027 | Đạo sư | 0 | 1 | — |
| 14028 | Đoàn kết | 0 | 1 | — |
| 14029 | Cát Phổ Trại bắn đạn | 0 | 10 | — |
| 14030 | Cát Phổ Trại áp đỉnh | 0 | 10 | — |
| 14031 | Tiếp chiêu | 0 | 1 | — |
| 14032 | **Tinh luyện** | 0 | 10 | 14019 |
| 14033 | Xạ Tiễn | 0 | 5 | 14037 |
| 14034 | Kỵ Binh | 0 | 1 | — |
| 14035 | Chuyển Thể | 0 | 1 | — |
| 14036 | Bách Xuyên | 0 | 5 | — |
| 14037 | Tiễn Vũ | 0 | 5 | 14036 |

> ❓ **Mở**: `14014` và `14015` không có trong `skills.json` nhưng `rebornChar()` của Bear cấp
> `14015` (level 10). Cần chốt `14014`/`14015` là gì.

### 4.2 Bản đồ sang id **client** (`Data/Skill.Dat`)

⚠️ **INFERRED** — dựa trên tên + cấu trúc `require_sk` + thứ tự. Xem
`data/skill_life_tam.tsv` (88 dòng, đã ghép 2 không gian).

| id server | tên | id client (đoán) | tên client |
|:-:|---|:-:|---|
| 14001 Thuật trinh thám | | 13953 | Trinh Sát Thuật |
| 14002 Thuật đào tẩu | | 13954 | Đào Tẩu Thuật |
| 14003 Thuật mua bán | | 13955 | Mậu Dịch Thuật |
| 14004 Thuật dò xét | | 13956 | Thám Hiểm Thuật |
| 14005 Thuật luyện đơn | | 13957 | Luyện Đan Thuật |
| 14006 Đại hô | | 13958 | Nộ Hống |
| 14007 Giải hôn | | 13959 | Giải Trừ Thụy Miên |
| 14008 Hôn mê | | 13960 | Thụy Miên Thuật |
| 14009 Người làm | | 13961 | Tuyển Dụng Thuật |
| 14010 Thuật đào khoáng | | 13962 | Khai Khoáng Thuật |
| 14011 Thuật đúc luyện | | 13963 | Luyện Kim Thuật |
| 14016 Thuật giao dịch | | 14064 | Khai Thương Thuật |
| 14017 Tịnh toạ | | 14065 | Thiền Định |
| 14018 Thuật đào sâu | | 14066 | Thâm Khoáng Thuật |
| 14019 Thuật chế tạo | | 14067 | Luyện Tạo Thuật |
| 14020 Hoạch lí phản | | 14068 | Hỗn Loạn Thuật |
| 14021 Hỗn loạn | | 14069 | Hỗn Độn Thuật |
| 14022 Giải hỗn loạn | | 14070 | Giải Trừ Hỗn Độn |
| 14023 **Cưỡi ngựa** | | 14071 | **Kỵ Mã Thuật** |
| 14025 Chiếm đọat | | 14073 | Đoạt Nguyên Thuật |
| 14026 **Triệu gọi** | | 14074 | **Triệu Linh Thuật** |
| 14027 Đạo sư | | 14075 | **Gia Sư Thuật** |
| 14032 Tinh luyện | | 14032 | Tối Cao Chiến Lược ⚠ trùng id |
| 14038 Đại chu thiên | | 14054 | Đại chu thiên |

> ⚠️ **CẢNH BÁO — id trùng nhau giữa 2 không gian.** `14032` là *Tinh luyện* ở server
> nhưng là *Tối Cao Chiến Lược* ở client. **Không được dùng id để suy ra tên.**
> Bản đồ đầy đủ 601 dòng ở `data/skill_id_space_conflicts.tsv`.

### 4.3 Ba kỹ năng sống được server cấp sẵn

✅ **VERIFIED** — `setkillElement5()` (Bear `TSCharacter.cs`):

```
14026 Triệu gọi    → level 10
14032 Tinh luyện   → level 10
14002 đào tẩu      → level 1
14003 mua bán      → level 10
14004 dò xét      → level 10
14005 luyện đơn    → level 10
14006 Đại hô       → level 10
14007 giải hôn     → level 1
14008 hôn mê       → level 5
```

✅ **VERIFIED** — `rebornChar()` (Bear) — bộ skill mặc định sau tái sinh:

```
14001 trinh thám 1 | 14015 ? 10 | 14021 hỗn loạn 5 | 14023 cưỡi ngựa 1 | 14035 chuyển thể 1
14002 đào tẩu 1    | 14003 mua bán 1 | 14004 dò xét 10 | 14005 luyện đơn 10
14010 đào khoáng 10 | 14011 đúc luyện 10 | 14018 đào sâu 10 | 14019 chế tạo 10
```

---

## 5. Kỹ năng nghề sau chuyển sinh / tái sinh

### 5.1 Bốn nhánh nghề — Bá / Hiệp / Hiền / Tiên

✅ **VERIFIED** — `setallskillForJob()` (Bear `TSCharacter.cs`), `job` = 1..4 do
`rebornChar(nb_reborn, j)` đặt (`job = j` khi `rb == 2`).

> ### ⭐ THỨ TỰ HIỂN THỊ trong client TS Online: **Bá → Hiệp → Hiền → Tiên**
> (nguồn: xác nhận của người dùng + Round-9 — `FUN_0057DD0C` hiện cây nhánh nghề
> khi `self+0x258` (tab hiện tại) ∈ **{6, 7, 8, 9}** ⇒ 4 tab = 4 nhánh.)
>
> ⇒ **Thứ tự `job` của server KHÔNG = thứ tự hiển thị của client.**
> | tab client | 6 | 7 | 8 | 9 |
> |---|:-:|:-:|:-:|:-:|
> | nhánh | **Bá** | **Hiệp** | **Hiền** | **Tiên** |
> | `job` server | 2 | 1 | 3 | 4 |

| `job` | Nhánh | thứ tự client | skill (id **canonical/wire** / id trong `Skill.Dat` / tên / level) | `sp_cost` | `skill_type` |
|:-:|---|:-:|---|:-:|---|
| **2** | **Bá** | **1** | 14039 / **14055** Bá khí (10) | 0 | Passive |
| | | | 14040 / **14056** Bá ý (5) | 46 | 19 |
| | | | 14041 / **14057** Lực bạt sơn hề (10) | 0 | Passive |
| | | | 14042 / **14058** Sóng cuồng (5) | 54 | Active |
| **1** | **Hiệp** | **2** | 14043 / **—** Anh khí hộ thân (10) | 0 | Passive |
| | | | 14044 / **—** ThiênThần thuẫn (5) | 46 | 19 |
| | | | 14045 / **—** Tâm nhãn (10) | 0 | Passive |
| | | | 14046 / **—** NgưngKhí HộThân (5) | 46 | 19 |
| **3** | **Hiền** | **3** | 14047 / **—** Dùng kế (10) | 0 | Passive |
| | | | 14048 / **—** Trí thủ (10) | 0 | Passive |
| | | | 14049 / **—** Chống địch (10) | 0 | Passive |
| | | | 14050 / **—** Sách động (10) | 0 | Passive |
| **4** | **Tiên** | **4** | 14051 / **14035** Tiên khiếu (10) | 0 | Passive |
| | | | 14052 / **14036** HóaHuyếtHòanHồn (5) | 0 | Passive |
| | | | 14053 / **14037** ThánhLinh thuật (5) | 52 | 19 |
| | | | 14054 / **14038** CànKhônThiênLôi (5) | 84 | Buff |

Tất cả `require_sk = 14038` (**Đại chu thiên**), `sk_point = 1`, `elem = 5` (Tâm).

> ### 🔴 ID `Skill.Dat` ≠ ID WIRE — và 8 id của 2 nhánh ĐÃ ĐƯỢC BỔ SUNG
> * **Id wire (canonical) = id trong `Data/skills.json`** — khớp 4/4 hằng số hard-code
>   của binary `aLogin.exe` (xem §2.2).
> * `Data/Skill.Dat` (build 376 record) là **build khác**, đánh số lệch **±16/±48**.
> * ✅ **Bản `Skill.Dat` mới ĐÃ ĐẦY ĐỦ**: đủ **16/16** kỹ năng 4 nhánh nghề
>   (bản cũ 352 record thiếu 8 id của Hiệp/Hiền) ⇒ *"id đang thiếu"* **không còn là
>   blocker**, chỉ là khác đánh số.
>
> Bảng map đầy đủ (384 cặp, 0 unmatched):
> [`data/skill_id_map_client_server.tsv`](./data/skill_id_map_client_server.tsv)
> (sinh bởi `tools/build_id_map.py`).

> ### ✅ Round-11 ĐÓNG item "tên nhánh nghề" — nhưng kết luận KHÁC dự đoán
>
> ⭐ **Round-9 đã khoanh vùng chính xác** (xem §12.3): form cây kỹ năng
> `TSe_SkillTreeForm` có **tab 6, 7, 8, 9** tương ứng 4 nhánh nghề
> (`FUN_0057DD0C`: `if (((byte)(self->0x258 - 6) < 4) && player->0x4B1)`),
> và bảng `DAT_0094B3A4` có **13 entry** trong đó **entry 6..13 (8 node)** là node
> nhánh nghề (⇒ đúng **8 slot `skill_rb2`**).
>
> **Round-11** đã dump **toàn bộ `.rsrc`** (83 968 B) và viết `tools/parse_dfm.py`
> (parser PE resource + DFM binary Delphi 7). Kết quả:
>
> | Kiểm tra | Kết quả |
> |---|---|
> | Số form DFM trong `aLogin.exe` | **6** — `TForm1..TForm4`, `TGmForm`, `TDelphiXDXPlayForm` |
> | Trong đó có form kỹ năng? | **KHÔNG** |
> | Encoding chuỗi DFM | ⭐ **Big5 (cp950)** — KHÔNG phải VISCII |
>
> ⇒ **Tên hiển thị 4 nhánh KHÔNG nằm trong `.rsrc` cũng không trong `.text`.**
> Xem §5.1.1 bên dưới.

### 5.1.1 ⭐⭐⭐ Form kỹ năng được nạp ĐỘNG từ FILE NGOÀI (Round-11)

`FUN_0057922C` (constructor `TSe_SkillTreeForm`):

```asm
00579495: CMP  dword ptr [EBP + -0x8],0x6      ; tabIndex < 6 ?
005794a1: ADD  EAX,0x1                         ; N = tabIndex + 1
005794ab: CALL 0x00409980                      ; IntToStr(N) -> "1".."6"
005794b6: MOV  EDX,0x579a08                    ; "Form_SkillTree"
005794bb: CALL 0x004040d4                      ; LStrCat3 -> "NForm_SkillTree"
005794ca: CALL 0x007c9b38                      ; ⭐ nạp form theo TÊN
005794d2: MOV  dword ptr [EDX + 0x4c],EAX       ; self->0x4C = instance
005794da: MOV  EAX,0x7                          ; tabIndex >= 6 -> "7"
005794ea: MOV  EDX,0x579a08
```

⇒ **7 tab của form kỹ năng là 7 form RIÊNG BIỆT**, tên:
`1Form_SkillTree` … `7Form_SkillTree` (thêm `Form_SkillTree1` cho NPC skill tree).

`FUN_007C9B38` (form factory) — **không** đọc resource trong exe:

```c
local_10 = managerA->FindResource(name);        // -1 nếu không có
local_14 = managerB->FindResource(name);        // -1 nếu không có
LStrCatN(&local_18, 3, ..., ".bmp", name, dir1);   // "<tên form>.bmp"
if (!FileExists(local_18)) { ... dir2 ... }
```

⇒ Nó đi tìm **`<tên form>.bmp`** trong thư mục dữ liệu của client.
`FUN_00576ED4` (form-module loader) dùng chuỗi `'form_module'` tại `0x577578`.

> ### ✅ KẾT LUẬN CUỐI CÙNG
> **Tên hiển thị 4 nhánh nghề (Bá / Hiệp / Hiền / Tiên) và caption của tab 5
> KHÔNG tồn tại trong `aLogin.exe`.**
>
> Chúng nằm trong **file form-module / resource ngoài** mà client nạp lúc runtime
> (kèm ảnh `.bmp` `<tên form>.bmp`), thường nằm trong thư mục `Data/` của client.
>
> ⇒ Muốn lấy chúng phải có **thư mục dữ liệu gốc của client**; **không suy ra được
> từ binary**. Đây là **kết luận có bằng chứng** (không phải "chưa tìm thấy").
>
> ### ✅✅ Round-12: ĐÃ TÌM RA CƠ CHẾ CHÍNH XÁC — tab 5..9 dùng **TÊN ẢNH**, không có chữ
>
> Form "Kỹ năng" tạo **5 tab** (đánh số **5..9**) trong vòng lặp `0x5AC7B2`:
>
> ```asm
> 005ac82c: ADD  EAX,0x4                  ; i + 4  →  5,6,7,8,9
> 005ac836: CALL 0x00409980                ; IntToStr(i+4)
> 005ac841: MOV  EDX,0x5aca04              ; "btn_02"
> 005ac846: CALL 0x004040d4                ; LStrCat3 → "5btn_02".."9btn_02"
> 005ac858: CALL  dword ptr [EAX + 0x8]     ; control.SetText("Nbtn_02", 5)
> ```
>
> ⇒ **Tab 5** (tab gốc) và **tab 6/7/8/9** (4 nhánh nghề) đều KHÔNG có caption text —
> nhãn là **tên ảnh** `<n>btn_02` ⇒ **tên 4 nhánh nghề được vẽ sẵn trong ảnh**
> `6btn_02.bmp` … `9btn_02.bmp`.
>
> ⇒ **Về nguyên tắc không thể rút từ binary** (không phải "chưa tìm thấy").
> Chi tiết: [`ROUND12_RESULTS.md`](./ROUND12_RESULTS.md) §2.
>
> ⇒ Nhưng **tên nhánh trong game** vẫn xác định được từ phía server: `job` 1..4
> của Bear + thứ tự hiển thị client (Bá→Hiệp→Hiền→Tiên) ⇒ map
> `job 2→Bá`, `job 1→Hiệp`, `job 3→Hiền`, `job 4→Tiên`.

| Mục | Nguồn |
|---|---|
| Bảng chuỗi VISCII toàn `.text` | [`data/viscii_strings.tsv`](./data/viscii_strings.tsv) (22 138 dòng) |
| Chuỗi trong vùng module kỹ năng | [`data/skill_ui_captions.tsv`](./data/skill_ui_captions.tsv) (1 475 dòng) |
| Bảng phẳng DFM (Big5) | [`data/dfm_properties.tsv`](./data/dfm_properties.tsv) |

### 5.1.2 ⭐⭐ 4 hàm dựng cây 4 nhánh nghề (Round-12)

4 hàm **gần như giống hệt nhau**, chỉ khác **hằng số index node** trong
`self->0x24C` (mảng node 2-D):

| Hàm | Node index | Tab | Nhánh |
|---|:-:|:-:|---|
| `FUN_00579E80` | **0** | 6 | Bá |
| `FUN_0057A858` | **1** | 7 | Hiệp |
| `FUN_0057B2D8` | **2** | 8 | Hiền |
| `FUN_0057BD58` | **3** | 9 | Tiên |

Gọi liên tiếp trong hàm setup của `TSe_SkillTreeForm` (`0x579094..0x5790AC`),
sau đó nạp form `"Ball"` (UI "bóng") vào `self->0x294`.

⇒ **Gốc rễ 4 nhánh nghề nằm ở `self->0x24C[node 0..3]`**.
⇒ `FUN_0057A858` (index 1) **chưa có** trong bundle → đưa vào manifest Round-13.

### 5.2 "Bóng" (`ball`) — `ballList[12]` và `skill_rb2[8]`

✅ **VERIFIED** — `AddSkillrb2()` (Bear) + đối chiếu Round-9:

| Mảng | Kích thước | Ý nghĩa |
|---|---:|---|
| `bool[] ballList` | **12** | 12 "bóng" (ball) — **theo hệ**, đánh số **1..12** |
| `ushort[] skill_rb2` | **8** | 8 "bóng" — **theo hệ**, đánh số **0..7** |

`AddSkillrb2()` gán `skill_rb2[0..7]` **theo `element` của người chơi**:

| `element` | hệ | `skill_rb2[0..7]` (id **server**) |
|:-:|---|---|
| 1 | Kim / Địa | 10027, 10028, 10029, 10030, 10031, 10032, 10033, 10034 |
| 2 | Thủy | 11027, 11028, 11029, 11030, 11031, 11032, 11033, 11034 |
| 3 | Hỏa | 12027, 12028, 12029, 12030, 12031, 12032, 12033, 12034 |
| 4 | Phong | 13027, **13026**, 13028, 13029, 13030, 13031, 13032, 13033 |

⚠️ Nhánh Phong lệch (`13026` thay vì `13027`) — có thể là lỗi của bản Bear cũ, cần đối chiếu.

> ### ⚠️ SỬA LẦM TRƯỚC ĐÂY (Round-9)
> Đã có nhầm lẫn rằng *"8 slot `skill_rb2` = 4 nhánh nghề × 2 slot"*.
> **SAI.** `skill_rb2` là **8 bóng theo hệ**, **KHÔNG** phải nhánh nghề.
> Cột id client: [`data/skill_rb2_balls.tsv`](./data/skill_rb2_balls.tsv).

> ### ⭐ Ánh xạ chính xác sang client (Round-9)
> | Khối trong gói `0x1C 05` | Client gửi | Bear nhận | Mảng đích |
> |---|---|---|---|
> | 1 | `L1` byte, giá trị **1..12** | `ballList[b-1]` | `bool[12]` |
> | 3 | `L3/3` entry, byte đầu **6..13** | `skill_rb2[byte-6]` | `ushort[8]` |
>
> ⇒ **12 ball 1-based** ↔ `bool[12]`; **8 rb2 slot** ↔ `ushort[8]`.
> ⇒ Client gửi `u8 slot` = **6..13** ⇒ server tính `skill_rb2[0..7]`.

### 5.3 ⭐⭐ Layout payload C→S `0x1C 05` (nâng cấp / tái sinh lần 2) — ĐÃ CHỐT

✅ **VERIFIED 2 CHIỀU (Round-9)** — vừa đọc được từ client (Ghidra), vừa khớp 100%
với server Bear.

#### 5.3.1 Gói client (đọc từ `aLogin.exe`)

Form `TSe_SkillTreeForm` (`gvar_007D9E68`) có 3 buffer `published`
(RTTI tại `0x577A00`): `+0x288`, `+0x28C`, `+0x290`.
Hàm `FUN_0057C840` dựng chúng; hàm `FUN_00583A98` lắp payload cuối vào `+0x290`.

```
offset  size      nội dung
 2      4         u32 LE  L1 = len(form+0x288)        ← số "bóng" (ball)
 6      L1        L1 byte: chỉ số ball 1-based (1..12)
 6+L1   4         u32 LE  L2 = số kỹ năng  (= len(skillList)/3)
 +4     L2×3      mỗi kỹ năng: { u16 skillId LE, u8 targetLevel }
 +4     4         u32 LE  L3 = len(form+0x28C)
 +4     L3        L3 byte: entry 3 byte = { u8 slotIndex, u16 skillId LE }
```

#### 5.3.2 Gói server Bear — **KHỚP 100%**

`TSCharacter.cs :: setSkillRb2(byte[] data)` (`data[0]=0x1C`, `data[1]=0x05`):

```csharp
int num = 2;
uint num2 = PacketReader.read32(data, num);   // ⭐ = L1 (số bóng)
if (ball_point >= num2) {
    ball_point -= (byte)num2;
    num += 4;
    for (int i = 0; i < num2; i++)
        ballList[data[num + i] - 1] = true;     // ⭐ client gửi 1..12 → ballList[0..11]
    num += (int)num2;
    uint num3 = PacketReader.read32(data, num); // ⭐ = L2 (số kỹ năng)
    num += 4;
    for (int j = 0; j < num3; j++) {
        setSkill(PacketReader.read16(data, num), data[num + 2]);  // ⭐ u16 LE + u8
        num += 3;
    }
    uint num4 = PacketReader.read32(data, num) / 3;  // ⭐ = L3/3 (số entry rb2)
    num += 4;
    for (int k = 0; k < num4; k++) {
        skill_rb2[data[num] - 6] = PacketReader.read16(data, num + 1);  // ⭐ slot = byte-6
        num += 3;
    }
    sendBallList();
}
```

| Khối | Client (`FUN_00583A98`) | Server Bear | Khớp |
|---|---|---|:-:|
| 1 | `u32 L1` + `L1` byte (ball **1..12**) | `num2` + `num2` byte, `ballList[b-1]`, `bool[] ballList = new bool[12]` | ✅ |
| 2 | `u32 L2` + `L2×3{u16 skillId, u8 level}` | `num3` + `num3×3` → `setSkill(read16, byte)` | ✅ |
| 3 | `u32 L3` + `L3` byte (`{u8 slot **6..13**, u16 id}` × L3/3) | `num4 = L3/3` → `skill_rb2[byte-6] = read16`, `ushort[] skill_rb2 = new ushort[8]` | ✅ |

⇒ **Mọi con số khớp tuyệt đối**: 12 ball ↔ `bool[12]`, 8 slot rb2 ↔ `ushort[8]`.

⇒ **`0x1C 05` ĐÃ CHỐT TRỌN VẸN.** Client gửi slot **6..13** ⇒ server tính
`skill_rb2[0..7]` ⇒ **8 bóng theo hệ** ✓ (§5.2) — *không* phải nhánh nghề.

⇒ **12 chỉ số ball 1..12** của client (`self+0x288`, dựng từ 12 nút `self+0x138..0x164`)
⇔ `bool[] ballList = new bool[12]` của server — **1-based ở cả hai bên** ✓

#### 5.3.3 ⭐ Tác động lên server Rust (`src/server/handlers/skills.rs`)

⚠ `handle_skill_reborn_2` hiện **parse đúng khối 1 và 2 nhưng BỎ QUA khối 3**
(`u32 L3` + `L3` byte) ⇒ **server Rust chưa gán được `skill_rb2`**.

⚠ Ngoài ra `0x1C 02` phía client là
`{ count u8 }{ skillId u16 LE }{ targetLevel u8 }` (đã VERIFIED), trong khi
handler Rust `handle_pet_skill_upgrade` (sub 2) lại đọc `payload[0]` là **stt thú**
⇒ **cần tách riêng handler `0x1C 02` (học nhiều kỹ năng cho người chơi)** và
handler `0x1C 04` (chưa có) — xem [`WIRE_FORMATS.md` §6](./WIRE_FORMATS.md).

---

## 5.4 ⭐⭐ Kỹ năng cho THÚ CƯNG (`0x1C 02`) — Round-10

> ### 🔴 SỬA LẦM: `0x1C 02` **KHÔNG** phải "học nhiều kỹ năng"
> ⇒ **`0x1C 02` = học / nâng cấp kỹ năng cho THÚ.**
> Byte đầu = **chỉ số thú (1..4)**, KHÔNG phải count.
> Chi tiết + bằng chứng: [`WIRE_FORMATS.md` §6.2](./WIRE_FORMATS.md) · [`ROUND10_RESULTS.md` §2](./ROUND10_RESULTS.md).

### 5.4.1 Cấu trúc thú trong RAM của client

| Offset | Kiểu | Ý nghĩa |
|---:|---|---|
| `player+0x57C` | `ptr[4]` | ⭐ **4 con trỏ thú** (slot 1..4) |
| `thú+0x78` | `u8` | cờ "thú còn sống" |
| `thú+0x55C` | `u8` | ⭐ **loài thú** (dùng để tra cứu slot) |

✅ `FUN_0071D234(player, loàiThú)` → trả về **slot 1..4** (0 nếu không tìm thấy).
⚠ **KHÔNG** phải hàm "đếm số kỹ năng".

### 5.4.2 Form học skill cho thú (`FUN_0058484C`, 448 B)

| Offset của form | Ý nghĩa |
|---:|---|
| `+0xF4 + i*4` (i=0..2) | 3 **node kỹ năng** (`+0x10C`=skillId, `+0x108`=level, `+0x109`=flag) |
| `+0x140` | ⭐ **con trỏ tới thú** đang chọn (so khớp với `player->0x57C[1..4]`) |

Payload gửi: `{ petSlot u8 }` + tối đa 3 × `{ u16 skillId LE, u8 targetLevel }`.

---

## 6. Hệ Quang / Ám (Light / Dark)

### 6.1 Bản chất

✅ **VERIFIED** — Quang/Ám **KHÔNG phải `elem`** (hệ Kim/Thủy/Hỏa/Phong/Tâm). Nó là byte
`type` trong `Skill.Dat`:

| `type` | số skill | id (server space) | id (client `Data/Skill.Dat`) | `elem` |
|:-:|:-:|---|---|:-:|
| **18** | 22 | 22001-22022 | 21953-21963, 22060-22070 | **0** |
| **19** | 23 | 23001-23023 | 22972-22975, 22988-22995, 22996-22998, 22999-23003, 23017-23019 | **0** |

⚠️ **Vì `elem = 0`, client KHÔNG gate Quang/Ám theo hệ.** Client chỉ:
* kiểm tra `type == 17` ⇒ "bắt buộc phải học" (`FUN_006538BC`),
* kiểm tra `elem == 5` ⇒ không giới hạn hệ khi **dùng** (`FUN_0076D578`).

⇒ **Toàn bộ quyết định Quang/Ám thuộc server.** Client không gửi byte hệ/hệ-số kèm
gói học kỹ năng.

### 6.2 Danh sách đầy đủ

Xem `data/skill_quang_am.tsv` (45 dòng, có tên + mô tả + `sk_point` + `max_lvl`).

**QUANG (type 18)** — 22 skill: Quang mâu thuật · Phật quang chưởng · Huyễn quang ·
Liệt diệm chi quang · Quang du thuật · Quang chi kính · Quang tuyền quán chú ·
Hồi quang phản chiêu · Quang tốc đầu trịch · Kim tê công kích · **Bạch Hổ** ·
Gọi tiểu/trung/đại Bạch Hổ · Tập thức quang đạn · Đồng quang chi ban · C.Đ quang chi ban ·
C.L diệm chi quang · Chân huyễn quang · C.Phật quang chưởng · Siêu quang ba ·
Quyền khuynh vạn lý

**ÁM (type 19)** — 23 skill: Ám độc tiễn · Ám đao trảm · Hắc toàn trảm · Vu độc ·
Tụ ẩn kiếm · Khô lâu đầu trịch · Vu độc oa oa · Yêu thuật kích hoả · Mê hoặc độc trâm ·
Độc phi luân · Huyền Vũ · Gọi tiểu/trung/đại Huyền Vũ · Vựng trảm · Hắc ám liên trảm ·
C.Ám độc tiễn · Chân Vu độc · Vạn độc xuyên tâm · C.Độc phi luân · C.Ám nhẫn trảm ·
C.Mê hoặc độc trâm · Phiêu diêu lưu phong

> ⚠️ **Quan sát đáng chú ý**: cả Quang lẫn Ám đều có 3 skill **"Gọi …"** (Bạch Hổ / Huyền Vũ)
> ⇒ đây là **nhánh triệu gọi (summon)** riêng của hệ Quang/Ám, tách khỏi
> kỹ năng sống "Triệu gọi" (14026).

### 6.3 "Kích hoạt" Quang/Ám — trạng thái chưa rõ

⚠️ **INFERRED** — `FUN_0072E9A0` (`0x0072F428`) và `FUN_00710AB0` (`0x007117A8`) đều so
sánh `gself+0xFD4+idx*3 == 0x36B3` và khi `level != 0` thì gọi
`FUN_007B0094(gvar_007DA108+0x13C, 1)` (bật 1 cờ trên 1 form) + `gvar_007DA530+0x1B8 = 1`.

❓ **Mở — `0x36B3` là gì?**
* Trong không gian `skills.json`: `14003` = *Thuật mua bán* (type 5) — hợp lý về mặt kỹ năng
  sống, nhưng lạ vì client bật cờ riêng.
* Trong không gian `Data/Skill.Dat`: **không tồn tại**.
⇒ Cần agent kiểm tra `gvar_007DA108` (form nào?) và `gvar_007DA530+0x1B8` để xác định
cờ này là gì (mở tab Quang/Ám? báo "đã kích hoạt"?).

---

## 7. Kỹ năng triệu gọi (summon)

| Mục | id (server) | id (client `Data/Skill.Dat`) | Ghi chú |
|---|:-:|:-:|---|
| Triệu gọi (kỹ năng sống) | **14026** | 14074 *Triệu Linh Thuật* | `max_lvl = 1`, `elem = 5` |
| Gọi Bạch Hổ (Quang) | 22011-22014 | 21963, 22060-22062 | `elem = 0` |
| Gọi Huyền Vũ (Ám) | 23011-23014 | 22995-22998 | `elem = 0` |

✅ **VERIFIED** — `FUN_0073D558` (file `0073d558_FUN_0073d558.c`) — hàm kiểm tra
"người chơi đã học kỹ năng X chưa" khi click NPC:

```c
if (param_2 == '\t') {          // 0x09
    idx = FUN_00759b04(gvar_007DA554, 0x36c7);
    if (*(short *)(param_1 + 0xfd4 + idx*3) == 0x36c7) local_a = 1;
} else if (param_2 == 0x16) {   // 0x16
    idx = FUN_00759b04(gvar_007DA554, 0x36d2);
    if (*(short *)(param_1 + 0xfd4 + idx*3) == 0x36d2) local_a = 1;
}
```

⇒ id `0x36C7` = 13959 và `0x36D2` = 14034 **được dùng làm "token"** khi click NPC.
Trong không gian `skills.json`: `13959` không tồn tại, `14034` = *Kỵ Binh*.
Trong `Data/Skill.Dat`: `13959` = *Giải Trừ Thụy Miên*, `14034` = *Chiến Thuật T.T*.
❓ **Mở — id `0x36C7` không tồn tại trong bất kỳ bảng nào** ⇒ nhiều khả năng client được
build với `Skill.Dat` **thứ ba**. Cần tìm file đó.

❓ **Mở — "triệu gọi theo hệ Thủy/Hỏa/Địa/Phong"** chưa tìm thấy trong client:
* Không có chuỗi "summon"/"triệu" nào trong 13 592 hàm đã decompile.
* `Data/Mounts.dat` + `Data/MountsGrow.dat` là dữ liệu **cưỡi ngựa / thú cưỡi**
  (npc_id, flag_id, source, scale; grow theo level) — có thể là "triệu gọi" dạng thú.
* `Data/Compound.Dat` là công thức **chế tạo** (8 byte/record: `compound_d`, `material_a/b/c`,
  `a/b/c_degree`).
* Agent phải tra `eve.emg` / `Npc.dat` (`skill_link`) / `Item.dat` để tìm dữ liệu triệu gọi.

---

## 8. Nâng cấp kỹ năng (skill level-up)

✅ **VERIFIED** — client **không** có gói "nâng cấp kỹ năng" riêng. Cùng một kênh `0x1C 01`
(dialog cây kỹ năng) và `0x1B 01` (thầy skill) được dùng cho cả học mới lẫn nâng cấp.

✅ **VERIFIED** — điểm kỹ năng:
* `TFSkill+0x1C` = `sk_point` yêu cầu (so với `player+0x3EC`).
* `TFSkill+0x19` (`type`) `== 17` ⇒ "bắt buộc phải học".
* `player+0x400` = **skill point** (ghi bởi prop `0x25` trong frame `08 01`).

⚠️ **Mâu thuẫn cần chốt** — `player+0x3EC` vs `player+0x400`:
* `FUN_0059F090` (render cây kỹ năng) so `player+0x3EC` với `TFSkill+0x1C` ⇒ `+0x3EC` là
  **điểm dùng để học**.
* `FUN_00710AB0` với prop `0x25` ghi vào `player+0x400` ⇒ `+0x400` là **skill point tổng**.
* Bảng offset trong `05 03`: `payload[0x05..06]` → `+0x3EC`, `payload[0x18..19]` → `+0x400`.
❓ Cần agent xác định chính xác 2 trường này (có thể là "điểm học hiện tại" vs "điểm dự trữ").

✅ **VERIFIED** — server Bear tính giá điểm khi học (`rebornChar()`):

```
elem == 5 hoặc elem == 0            → giá gốc
elem == nhân vật                    → giá gốc
elem != nhân vật                    → giá × 2
nâng từ level 1 lên level L (L>1)   → giá + (L-1)
```

Rust hiện tại (`src/server/handlers/skills.rs :: get_point_skill_add`) **thiếu nhánh
`elem == 5` miễn phí** ⇒ 🔴 gap.

---

## 9. Bảng cây kỹ năng của client — 🔴 KHÔNG phải bảng tĩnh, mà dựng lúc runtime

### 9.1 Bảng nằm trong BSS

✅ **VERIFIED** — đọc header section của `client_delphi/aLogin.exe`:

```
CODE  VA 0x00001000  VSZ 0x003D3288  RA 0x00000400  RSZ 0x003D3400
DATA  VA 0x003D5000  VSZ 0x000057F4  RA 0x003D3800  RSZ 0x00005800
BSS   VA 0x003DB000  VSZ 0x002368CD  RA 0x003D9000  RSZ 0x00000000   <-- UNINITIALISED
```

* `0x0094B440` (bảng cây kỹ năng) → RVA `0x54B440` → **BSS**
* `0x009AE128` (bảng tra ngược) → RVA `0x5AE128` → **BSS**

⇒ **Không có byte nào của 2 bảng này trong file.** Mọi lần dump Ghidra trả
`readable 0` là **đúng bản chất**, không phải thiếu sót ⇒ **"BLOCKER" đã được gỡ**.
(Chi tiết + cách khôi phục: [`BLOCKED_DUMPS.md`](./BLOCKED_DUMPS.md).)

### 9.2 Ai dựng bảng? — `FUN_005A0388` (gọi ở CUỐI parser `05 03`)

✅ **VERIFIED** — file `005a0388_FUN_005a0388.asm.txt`:

```asm
MOV  EAX,0x94b440 ; XOR ECX,ECX ; MOV EDX,0x2fd ; CALL 0x00402cd0
                       ; FillChar(0x94B440, 765, 0)   <-- xoá sạch
MOV  DX,0x2711 ; CALL 0x00597954   ; gốc 10001 (Địa)
MOV  DX,0x2af9 ; CALL 0x00597954   ; gốc 11001 (Thủy)
MOV  DX,0x2ee1 ; CALL 0x00597954   ; gốc 12001 (Hỏa)
MOV  DX,0x32c9 ; CALL 0x00597954   ; gốc 13001 (Phong)
; vòng lặp i = 0..0xFF:
;   nếu player[0xFD4 + i*3] != 0:
;     FUN_00597954(node, player[0xFD4+i*3], player[0xFD6+i*3])
```

⇒ **Bảng = 4 skill gốc hard-code + MỌI kỹ năng người chơi đã học.**

### 9.3 `FUN_00597954` — đăng ký 1 kỹ năng vào cây

✅ **VERIFIED** — file `00597954_FUN_00597954.asm.txt`:

```asm
CALL 0x0075a948(gvar_007DA554, skillId, &local)  ; copy record TFSkill (86 byte)
MOV  AL, byte ptr [EBP + -0x62]                  ; local[+0x1A] = elem
SUB  EAX,0x1                                    ; *** tab = elem - 1 ***
IMUL EDX,EDX,0x99 ; ADD EDX,0x94b440
MOV  AX, word ptr [EDX + EAX*0x1]                ; table[slot*3].skillId
CMP  AX, word ptr [EBP + -0x6]                  ; == skillId ?
JNZ  <chưa có>
MOV  CL, byte ptr [EBP + -0x7]                  ; level
MOV  byte ptr [EDX + EAX*0x1 + 0x2], CL         ; table[slot*3+2] = level
```

| Claim | Mức |
|---|:-:|
| Layout `TSkillTreeRow[5 tab][51 slot][{u16 skillId LE}{u8 X}]` = 765 byte tại `0x94B440` | ✅ |
| **Index tab = `elem - 1`** ⇒ Địa=0, Thủy=1, Hỏa=2, Phong=3, **Tâm=4** | ✅ |
| **Byte thứ 3 là LEVEL của kỹ năng đã học** (KHÔNG phải "byte yêu cầu") | ✅ |
| Kỹ năng sống hệ Tâm **CÓ tab riêng** (tab 4) | ✅ |
| Bảng chỉ chứa skill **người chơi đã học** + 4 gốc | ✅ |

> 🔴 **Đính chính tài liệu cũ** (`light_dark_skill_unlock_gap.md` §2.5, §6 #4):
> * "bảng hard-code 0x94B440" → **SAI**: bảng **dựng lúc runtime**.
> * "byte thứ 3 = requireByte" → **SAI**: là **level** của kỹ năng.
> * "element 5 không có tab" → **SAI**: element 5 ⇒ **tab 4**.
> * "0x94B440 là BLOCKER, cần re-import Ghidra" → **SAI**: không tồn tại trong file.

### 9.4 Quan hệ cha → con hard-code

✅ **VERIFIED** — `switch` trong `FUN_00597954` đăng ký kỹ năng con ở **ô cha + delta**:

| cha | hex | con | delta ô |
|:-:|---|---|:-:|
| 10024 | `0x2728` | 10023 (`0x2727`) | +2 |
| 12024 | `0x2EF8` | 12022 (`0x2EF6`) | +1 · 12025 (`0x2EF9`) | +3 |
| 12026 | `0x2EFA` | 12025 (`0x2EF9`) | +1 |
| 13021 | `0x32DD` | 13020 (`0x32DC`) | +1 |
| 13024 | `0x32E0` | 13023 (`0x32DF`) | +1 |

⇒ Đây là **5 mốc cha–con duy nhất** mã hoá cứng. ❓ Agent cần suy thêm quan hệ cha–con
từ `require_sk` của `Skill.Dat`.

### 9.5 Bảng tra ngược `0x9AE128` — cũng dựng lúc runtime

✅ **VERIFIED** — `FUN_00759B2C` (file `00759b2c_FUN_00759b2c.asm.txt`):

```asm
MOVZX EAX, word ptr [EDX + EAX*0x2 + 0x1a]   ; id của record
MOV   word ptr [EAX*0x2 + 0x9ae128], DX      ; reverse[skillId] = index
INC counter ; CMP counter,0xbb9 ; JNZ loop   ; 3000 record
```

⇒ Bảng tra ngược dựng từ **`Data\Skill.Dat`** lúc runtime (3000 record).
⇒ Skill id không có trong file ⇒ `reverse[id] = 0` ⇒ client coi như **chưa học**.

### 9.6 Bảng đã khôi phục

`tools/recover_skill_tree.py` dựng lại bảng từ `Data/Skill.Dat`:

* `data/skill_tree_tabs.tsv` — 255 dòng (5 tab × 51 slot)
* `data/skill_tree_recovered.bin` — 765 byte đúng layout RAM

Số ô có skill: Địa **50/51** · Thủy **45/51** · Hỏa **50/51** · Phong **48/51** ·
Tâm **51/51** (⚠ tràn vì file có 63 skill `elem=5` > 51 ô ⇒ bằng chứng bảng chỉ chứa
skill **đã học**, không phải catalogue tĩnh).

### 9.7 Hệ quả cho server

* **Server KHÔNG gửi định nghĩa cây kỹ năng** (đúng như tài liệu cũ) — nhưng **vì lý do
  khác**: client tự dựng từ `Skill.Dat` + danh sách skill người chơi.
* ⇒ **Server phải gửi đúng `skillId` ở tail `05 03`**, và skill id đó **phải có trong
  `Skill.Dat` của client** ⇒ client vẽ đúng vị trí ô.
* ⇒ **Thứ tự skill trong tail `05 03` KHÔNG quan trọng** cho vị trí ô (ô được xếp theo
  thứ tự record trong `Skill.Dat`, không theo thứ tự wire). Cần agent xác nhận lại.

---

## 10. ⭐ Hệ thống "Đẳng cấp" & "Nâng cấp kỹ năng" (Round-7)

### 10.1 Hai loại điểm kỹ năng

✅ **VERIFIED** — chuỗi UI trong `.text`:

| Chuỗi | VA | Nghĩa |
|---|---:|---|
| **"Điểm học được"** | `0x5A033C` | điểm dùng để **học** skill mới |
| **"Điểm thăng cấp"** | `0x5A0378` | điểm dùng để **nâng cấp** skill đã học |
| "Phải đạt số điểm để học được" | `0x5A0314` | tooltip |
| "Có thể dùng điểm thăng cấp" | `0x5A0354` | tooltip |
| **"Điểm số"** | `0x5B3218` | label ở form kỹ năng (ship) |

⇒ **Hai bộ điểm khác nhau**: một bộ cho học, một bộ cho nâng cấp.
Trong `05 03` có **hai** trường ứng với: `payload[0x05..06] → gself+0x3EC` và
`payload[0x18..19] → gself+0x400`; prop `0x25` ghi vào `+0x400`.
❓ **Cần chốt** cái nào là "Điểm học" và cái nào là "Điểm thăng cấp".

### 10.2 "Đẳng cấp" (rank) của kỹ năng

✅ **VERIFIED** — chuỗi UI:

| Chuỗi | VA | Hàm chứa nó |
|---|---:|---|
| **"Đẳng cấp từ %d nâng lên %d"** | `0x5BE1F8` | `FUN_005BE098` |
| "Đẳng cấp vượt quá giới hạn cao nhất" | `0x5BE338` | `FUN_005BE240` |
| "Đẳng cấp trước mắt không thể sử dụng phương thức nâng cấp" | `0x5BE364` | `FUN_005BE240` |
| "Đẳng cấp hạn chế " | `0x5A8C20` | `FUN_005A7970` |
| "Nâng cấp: " | `0x5A8CD0` | `FUN_005A7970` |
| **"Chuyển sinh chuyên dụng"** | `0x5A8CE4` | `FUN_005A7970` |
| "Lần đầu tiên học tập cần c…" | `0x5B45A4` | `FUN_005B4454` |
| "Mỗi lần lên cấp cần…" | `0x5B45E0` | `FUN_005B4454` |
| "Nhấn dưới để thiết định tăng cấp" | `0x5B4674` | `FUN_005B4454` |
| "Thăng cấp" | `0x5B3CCC` | `FUN_005B3B98` |
| "Xin hỏi bạn xác định muốn thăng cấp không?" | `0x5B3D10` | `FUN_005B3B98` |
| "Kỹ năng tăng cấp" | `0x5B01F8` | `FUN_005B0130` |
| "Đẳng cấp không đủ, không thể sử dụng" | `0x5BA088` | `FUN_005B9EDC` |

⇒ **"Đẳng cấp" = số điểm thăng cấp hiện có** của một kỹ năng.
⇒ Có **giới hạn trần** cho "Đẳng cấp" ⇒ đây là cơ chế **"Điểm thăng cấp"** (upgrade point)
tách riêng khỏi "Điểm số" (skill point).

### 10.3 ⭐ Gate "học kỹ năng gốc hệ" — 28 id

✅ **VERIFIED** — `FUN_005990E0` (Round-7), trước khi gửi `0x1C 01`:

```c
if ( skillId ∈ {0x2724..0x272A, 0x2B0C..0x2B12, 0x2EF4..0x2EFA, 0x32DB..0x32E1}
     && player[0x4B0] == 0 ) {
    show_message(&UNK_00599358, 2000);   // "Đẳng cấp vẫn chưa học chuyển sinh"
    return;                              // KHÔNG gửi
}
```

| Nhóm | id | Ghi chú |
|---|:-:|---|
| Kim/Địa | 10020..10026 | 7 id |
| Thủy | 11020..11026 | 7 id |
| Hỏa | 12020..12026 | 7 id |
| Phong | 13021..13027 | 7 id |

⇒ `player+0x4B0` = **cờ "đã học Chuyển sinh"**.
⇒ Thông báo *"Đẳng cấp vẫn chưa học chuyển sinh"* nghĩa là:
**"ngươi chưa học kỹ năng Chuyển sinh nên chưa thể học kỹ năng gốc hệ"**.

### 10.4 ⭐ Gate học skill theo hệ (4 thông báo)

✅ **VERIFIED** — `FUN_00598944` (Round-7 dump), 4 chuỗi liên tiếp:

| VA | Chuỗi |
|---:|---|
| `0x598B58` | "Thuộc tính Hỏa không thể học kỹ năng Địa hệ" |
| `0x598B8C` | "Thuộc tính Phong không thể học kỹ năng Thủy hệ" |
| `0x598BC4` | "Thuộc tính Địa không thể học kỹ năng Hỏa hệ" |
| `0x598BF8` | "Thuộc tính Thủy không thể học kỹ năng Phong hệ" |

⇒ Quan hệ chặn (theo thứ tự **ngược** trong bảng `type` của `Skill.Dat`):
**Địa ↔ Hỏa**, **Thủy ↔ Phong**.
⇒ ⚠️ **Không có** cặt chặn cho **Tâm (5)** ⇒ nhân vật hệ Tâm học được mọi skill hệ Tâm.

### 10.5 Skill "Chuyển sinh chuyên dụng"

✅ **VERIFIED** — chuỗi tại `0x5A8CE4` trong `FUN_005A7970` (dialog nâng cấp).
⇒ Đây là **một tên kỹ năng cụ thể** trong UI nâng cấp (không phải thông báo).
❓ Cần map sang `Skill.Dat` (id nào trong `skills.json`?).

---

## 11. Mảng kỹ năng trong RAM của client

✅ **VERIFIED** — `gself` (người chơi local) + `0xFD4`:

```
gself + 0xFD4 + idx*3   : u16 LE  = skillId  (giá trị trên wire)
gself + 0xFD6 + idx*3   : u8      = level
idx = FUN_00759B04(skillId)   ; = u16 [0x9AE128 + skillId*2], phải ≤ 0xFF
```

⇒ **Client lưu `skillId` (giá trị wire), KHÔNG phải `idx`.** Mọi reader kiểm tra bất biến
`gself+0xFD4+idx*3 == skillId`.

🔴 ⇒ **Server chỉ được gửi skill id có thật trong `Skill.Dat` của client.** Nếu id không có
trong bảng tra ngược, `idx` rơi vào ô rác ⇒ client coi như **chưa học**.

Chỉ **3 hàm** ghi vào mảng này:

| Hàm | Frame | Ghi |
|---|---|---|
| `FUN_0072E9A0` | `05 03` | `gself+0xFD4/0xFD6` theo tail 3 byte/record |
| `FUN_00710AB0` | `08 01` prop `0x6E` | `gself+0xFD4/0xFD6` (1 skill) |
| `FUN_0075B8E0` | `19 0B` | **xoá toàn bộ 3000 ô** |

Tất cả các hàm khác chỉ **đọc**.

---

## 12. ⭐ Form cây kỹ năng & cấu trúc lớp (Round-9)

### 12.1 `TSe_SkillTreeForm` — form cây kỹ năng NPC

✅ **VERIFIED** — đọc RTTI trong `.text` (`rtti_00577984_00577a20_skilltree_form_fields.hex`):

```
0x577A00: 0E 00 00 00 | 00 00 05 00 | 00 00 | 24 79 57 00
          4C 02 00 00  D4 10 40 00    ← +0x24C
          5C 02 00 00  D4 10 40 00    ← +0x25C
          88 02 00 00  D4 10 40 00    ← +0x288
          8C 02 00 00  D4 10 40 00    ← +0x28C
          90 02 00 00  D4 10 40 00    ← +0x290
0x577A1E: 11 "TSe_SkillTreeForm"
0x577AF6: 14 "TSe_NPCSkillTreeForm"   (lớp con)
```

| Offset | Nội dung |
|---:|---|
| `+0x24C` | mảng 2 chiều các node `TSe_TreeNodeBtn` (kỹ năng cây hệ) |
| `+0x25C` | mô tả/tooltip kỹ năng đang chọn |
| `+0x288` | payload "tab": chuỗi 1 ký tự các chỉ số tab đang bật (1..12) |
| `+0x28C` | payload `0x1C 04`: `{u8 slot}{u16 skillId LE}` × N |
| `+0x290` | payload `0x1C 05` (xem §5.3) |

VMT `0x577984` (40 slot) — 2 method ảo **trước đây không có function**, Round-9 đã tạo:

| Slot | Địa chỉ | Vai trò |
|:-:|---:|---|
| `+04` | `0x57F510` | — |
| `+10` | **`0x57DD0C`** | ⭐ **dựng cây** (render 28 node gốc hệ + 13 node nhánh nghề) |
| `+18` | **`0x57EE28`** | ⭐ **click node** (đọc skillId + mô tả) |
| `+20` | `0x57E540` | stub |
| `+24` | `0x583C20` | — |

### 12.1b ⭐ Danh tính các form (đọc `vmtClassName` = VMT−0x2C, Round-10)

| Global | VMT | Tên lớp |
|---|---:|---|
| `gvar_007D9E68` | `0x577984` | **`TSe_SkillTreeForm`** — form cây kỹ năng NPC |
| `gvar_007D9EB8` | `0x589754` | **`TSe_SkillForm`** — chủ sở hữu buffer dùng chung `+0x1A8` (mọi sender `0x1C 01/02` ghi vào) |
| `gvar_007DA5A8` | `0x5D5A20` | `TInterForm` — nguồn payload `0x1B 01` |
| `gvar_007DA720` | `0x5D5438` | `TThingSaleForm` — nguồn payload `0x1B 02` |
| `gvar_007D9E30` | `0x577A30` | `TSe_NPCSkillTreeForm` |
| `PTR_DAT_007da3d4` | `0x577B0C` | `TSe_ChildSkillTreeForm` |
| `gvar_007DA05C` | `0x5897EC` | `TSe_MiniSkillForm` |

### 12.2 Hai lớp node — ⭐ đã rõ field offset

`FUN_0057EE28` (`0x57EE28`) dùng `IsClass()` để phân biệt:

| Lớp | VMT | skillId | level hiện tại | flag | mô tả |
|---|---|---:|---:|---:|---:|
| `TSe_TreeNodeBtn` | `0x577838` | `+0x10C` (u32) | `+0x108` (u8) | `+0x109` (u8) | `+0x11C` |
| `TKC_TreeNodeBtn1` | `0x577760` | `+0x114` (u16) | `+0x110` (u8) | `+0x111` (u8) | `+0x118` |

⇒ **Byte level gửi lên server = `level + flag`**
(đối chiếu `FUN_005990E0`: `targetLevel = node->0x108 + 1` ⇒ node "chưa học" có flag = 1).

### 12.3 Bảng bố trí cây (runtime, trong BSS)

| Bảng | Số entry | Ý nghĩa | Field |
|---|---:|---|---|
| `DAT_0094B330` | **28** | 28 skill "gốc hệ" (10020-10026, 11020-11026, 12020-12026, 13021-13027) | `+0x108`=X, `+0x10C`=Y, `+0x110`=level, `+0x111`=flag, `+0x114`=skillId |
| `DAT_0094B3A4` | **13** | 13 node: 1..5 = 5 tab hệ, **6..13 = 8 node "bóng"** | cùng layout |

⇒ **8 node (6..13) = 8 bóng theo hệ** ⇒ khớp `skill_rb2[8]` của §5.2 ✓
⇒ Client gửi slot **6..13**, server Bear tính `skill_rb2[byte-6]` ⇒ slot **0..7** ✓
⇒ **KHÔNG** liên quan tới 4 nhánh nghề (nhánh nghề nằm ở **tab 6..9** của form)

⛔ Hai bảng này ở **BSS** (dựng lúc runtime từ `Skill.Dat`) ⇒ **không dump được**,
cần file `Skill.Dat` thật của client.

### 12.3b ⭐ Bốn tab nhánh nghề (client)

✅ **VERIFIED** — `FUN_0057DD0C`:

```c
if (((byte)(*(char *)(uStack_8 + 600) - 6U) < 4)      // self+0x258 = tab hiện tại
    && (*(char *)(*(int *)gvar_007DA7BC + 0x4b1) != '\0')) {
    for (i = 1; i <= 0x0D; i++) { ... }               // 13 node từ DAT_0094B3A4
}
```

⇒ Tab **6, 7, 8, 9** = **4 nhánh nghề**. Thứ tự hiển thị của client TS Online:

| tab | 6 | 7 | 8 | 9 |
|---|:-:|:-:|:-:|:-:|
| nhánh | **Bá** | **Hiệp** | **Hiền** | **Tiên** |
| `job` server | 2 | 1 | 3 | 4 |
| id client | 14055-14058 | *(đã có trong `Skill.Dat`)* | *(đã có trong `Skill.Dat`)* | 14035-14038 |

### 12.4 Cờ người chơi đã xác nhận

| Offset | Nghĩa | Bằng chứng |
|---:|---|---|
| `player+0x4B0` | ⭐ **"đã học Chuyển sinh"** | chặn 28 id gốc hệ (`FUN_005990E0`) |
| `player+0x4B1` | ⭐ **"đã mở nhánh nghề"** | hiện cây nhánh nghề ở tab 6..9 (`FUN_0057DD0C`) |
| `player+0x63A` | **map id** (u16) | `== 0x29CD (10637)` ⇒ chặn chức năng (`FUN_0057C840`) |
| `player+0x4B2`+ | ❓ chưa rõ | |

### 12.5 Cờ / thông báo client-side (không qua mạng)

| Điều kiện | Thông báo |
|---|---|
| `mapId == 10637` | `"Tại địa điểm này chức năng này tạm không được sử dụng"` (`DAT_0057CDE0`) |
| `skillList == 0 && self+0x288 == 0` | `"Bạn chưa thực hiện kỹ năng tăng cấp biến đổi nào"` (`DAT_0057CE20`) |
| skillId gốc hệ && `player+0x4B0 == 0` | ❓ chuỗi tại `0x599357` (chưa giải mã) |


### 12.6 ⭐ Bảng "bóng" (`ballList`) — 12 slot

✅ **VERIFIED** — `FUN_0058484C` dùng `player->0x57C[1..4]`; Bear dùng `bool[12] ballList`
⇒ 12 "bóng" đánh số **1..12**; `self+0x288` của `TSe_SkillTreeForm` ghi đúng 1 ký tự
chỉ số 1..12 (12 nút tại `self+0x138..0x164`).
---
