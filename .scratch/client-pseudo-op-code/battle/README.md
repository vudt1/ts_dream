# Nghiên Cứu Luồng Battle (Trận Chiến) — Client `aLogin.exe` ↔ Server `ts_dream`

Ngày: 2026-10-01 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Tài liệu nguồn: [`../post_login_stats_inventory_flow.md`](../post_login_stats_inventory_flow.md) (nối tiếp luồng sau login)
Nguồn chính: `client_pseudo_c/` (Ghidra pseudo-C + ASM + HEX)
Đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`
Source Rust: `src/battle/`, `src/server/handlers/battle.rs`, `src/protocol/codecs/battle_role.rs`

---

## 0. Cách đọc bộ tài liệu này

| File | Nội dung |
|:--|:--|
| [`01_battle_start_scene.md`](./01_battle_start_scene.md) | Mở trận (`0B FA`), load scene, entity, trailer |
| [`02_battle_positions.md`](./02_battle_positions.md) | Lưới trận 4×5, layout pos của player/NPC/pet |
| [`03_skill_and_combo.md`](./03_skill_and_combo.md) | Kênh lệnh trận thật, dùng skill, **kết luận về combo** |
| [`04_battle_end_rewards.md`](./04_battle_end_rewards.md) | Kết thúc trận, exp, item, gold |
| [`05_rust_battle_audit.md`](./05_rust_battle_audit.md) | Audit hiện trạng battle engine phía Rust |

**Mức bằng chứng** (giữ nguyên quy ước của tài liệu nền):

- **(A)** — đã xác minh từ client source (có body/ASM đọc byte).
- **(B)** — suy luận có cơ sở.
- **(C)** — chỉ là contract phía server Rust, chưa có client evidence.

> **Lưu ý phương pháp:** tuyệt đối không dùng `case N: break;` rỗng của Ghidra làm bằng chứng
> "client không làm gì" — đó là lỗi recovery đã được chứng minh (§7 tài liệu nền).

---

## 1. Tóm tắt điều hành

### 1.1 Bốn phát hiện quan trọng nhất

| # | Phát hiện | Mức | Ảnh hưởng |
|:-:|:--|:-:|:--|
| **H-1** | **Client KHÔNG BAO GIỜ gửi `0x32` C→S.** Kênh lệnh trận thật là **`0x0B` sub `02`/`08`**. | **A** | 🔴 **Blocker** — server chờ `0x32` nên không nhận được lệnh nào |
| **H-2** | **Combo 100% server-side.** Client không có bảng quy tắc combo, không có state combo. | **A** | 🟢 Server đã tự quyết định combo → đúng hướng |
| **H-3** | Lưới trận là **4 nhóm × 5 ô = 20 ô**; `0B FA`/`0B 05` mang pos đúng byte offset. Rust khớp 12/12 frame. | **A** | 🟢 Nền tảng pos đã đúng |
| **H-4** | **Thiếu `35 07`** (di chuyển ô) và **`33 01`** (set HP) trong frame Rust gửi. | **A** | 🔴 Ô trận client đóng băng; HP không cập nhật |

### 1.2 Timeline trận đánh (đã đối chiếu client)

```
Client                                   Server
  │                                        │
  │  click NPC / bị PK                     │
  ├─► 0B 02 <kind=2> <u32 targetId> ─────►  mở trận với NPC (kind 2)
  │                                        │
  │  ◄── 0B FA <diahinh u16> <mode> <n> <record 24B × n>   ← MỞ TRẬN
  │  ◄── 0B 05 <mode> <record 23B × n>                    ← NẠP ENTITY
  │  ◄── 0B 04 02 <u32 id> ...                            ← show on map
  │  ◄── 0B 0A 01                                           ← trailer
  │                                        │
  │  ◄── 34 01                                              ← đến lượt (your turn)
  │                                        │
  ├─► 0B 02 <kind=3|4|5> <u32> <u16> ─────►  ★ LỆNH TRẬN (dùng skill / item)
  │                                        │
  │  ◄── 32 01 ...   (kết quả hành động, combo)             │
  │  ◄── 35 xx       (biến cố: buff, dmg, di chuyển, HP)
  │  ◄── 08 01       (thay đổi chỉ số)
  │                                        │
  │  hết lượt → server tự chuyển lượt     │
  │                                        │
  ├─► 0B 01 <confirm=3> ─────────────────►  rời trận
  │  ◄── 0B 00 <selfId u32> 00 00                             ← thoát trận
  │  ◄── 08 01 <attr=0x24> <sign> <exp u32> ...              ← EXP (set tuyệt đối)
  │  ◄── 08 01 <attr=0x23> ...                               ← LEVEL
  │  ◄── 17 06 <record 12B>                                   ← ITEM nhận được
  │  ◄── 1A 01 <u32 delta>                                    ← VÀNG (cộng)
  │  ◄── 05 04 / 14 08                                        ← đóng UI trận
```

---

## 2. H-1 — Kênh lệnh trận: `0x32` là code chết 🔴

Đây là phát hiện quan trọng nhất và là **blocker** cho việc server nhận lệnh trận.

### 2.1 Bằng chứng client không gửi `0x32`

Bảng dispatch C→S `table_0x77F474` **có** entry cho `0x32` (→ builder `0x00788BCA`), nên builder
tồn tại trong code. Nhưng:

1. **Quét 116 file ASM** trong `test/ts_decompile/functions/` có call `FUN_0077f414`:
   tập `MOV DL,<mainop>` tại call-site **không chứa `0x32`**.
   ```
   21× MOV DL,0x17   17× MOV DL,0xb   11× MOV DL,0x27   7× MOV DL,0x14 ...
   → không có MOV DL,0x32
   ```
2. Với `DL=0x0B` (battle), tập `CL` (subop) thực tế là
   **{`01`,`02`,`03`,`06`,`07`,`08`,`0A`,`FA`}** — không có `04`, `05`.

### 2.2 Layout builder `0x32` (đã khôi phục byte-for-byte, mức A)

ASM `client_pseudo_c/0077f414_FUN_0077f414.asm.txt:3247-3437`:

```text
[0x32][sub]
[srcGrp  :1]   ← unit[unit[idxSelf]  ].grp    (unit+0x56E)   @ ASM 00788c9e
[srcSlot :1]   ← unit[unit[idxSelf]  ].slot   (unit+0x56F)   @ ASM 00788cf0
[dstGrp  :1]   ← unit[unit[idxTarget] ].grp    (unit+0x56E)   @ ASM 00788d48
[dstSlot :1]   ← unit[unit[idxTarget] ].slot   (unit+0x56F)   @ ASM 00788da0
[skillId :2LE] ← unit[idxSelf]+0x5A8                          @ ASM 00788dfa
[junk    :1]   ← LCG(0xDC)   \  padding deterministic         @ ASM 00788e19-00788e2b
[junk    :1]   ← LCG(0x100)  /                                @ ASM 00788e36-00788e42
```

- `idxSelf` đọc từ `battle+0xE78`, `idxTarget` từ `battle+0xEAA`; cả hai guard `<= 0x14` (20).
- Sub 2 (ASM `00788E7A`+) đọc `unit+0x5AA` thay vì `+0x5A8` → **item id** thay skill id.
- 2 byte cuối là **PRNG LCG tất định**, không mang ngữ nghĩa.

> **Kết luận (A):** layout builder là đúng như `runner.rs`/dispatcher Rust giả định
> (`row, col, row_attack, col_attack, skill_id u16`). Nhưng **không có call-site nào thực thi nó**.

### 2.3 Kênh lệnh thật: `0x0B` sub `02` / `08`

ASM `0077f414:800-843` và `00780920-007809AE` — builder `0x0B`:

```text
[0x0B][02]
[targetId :4LE] ← TFightManage+0x1E      @ ASM 0078095f (FUN_0077ee84 = encode U32)
[word     :2LE] ← TFightManage+0x22      @ ASM 00780982 (FUN_0077ef7c path)
```

`kind` byte tại `TFightManage+0x24` **không** nằm trên wire; nó chọn nhánh builder:

| kind | Ý nghĩa | Call-site |
|:-:|:--|:--|
| `2` | PK challenge | `00642c2c_FUN_00642c2c.asm.txt:383` |
| `3` | attack NPC | `00642720_FUN_00642720.asm.txt:191` |
| `4` | spectate | `00643310_FUN_00643310.asm.txt:76` |
| `5` | jam | `00643470_FUN_00643470.asm.txt:251` |
| `9`,`0A`,`0B`,`0C`,`0E` | (sub `08`) | `0064ea20`,`0064ed14`,`0064edf8`,`0064eecc`,`0064efa0` |
| `12`,`13` | (sub `08`) | `0064f60c`,`0064f6e4` |
| `15` | (sub `0A`) | `0064f7bc` |

> **⚠️ Chưa đóng:** byte bảng dispatch `0x0B` sub `00..04` tại `0x780440..0x780453` và vùng
> builder `0x78048E..0x7807D0` **không có trong corpus** → chưa xác minh được frame lệnh
> trận chi tiết. **Đây là thứ duy nhất cần redump.**

### 2.4 Hệ quả với Rust

`src/server/handlers/battle.rs:189-196` parse `0x32` (`row, col, row_attack, col_attack, skill_id`)
và `service.submit_command()`. Vì client không gửi `0x32`:

- **Không có `BattleCommand` nào tới engine** → vòng lượt không chạy.
- Cần viết lại handler theo `0x0B` sub `02`/`08`, và **redump** vùng ASM còn thiếu.

---

## 3. H-3 — Lưới trận & vị trí 🟢

### 3.1 Cấu trúc lưới (mức A, 3 nguồn độc lập)

`TFightField` = `DAT_0098C63C`:

- **4 nhóm (`grp` 0..3) × 5 ô (`slot` 0..4) = 20 ô.**
- Guard: `3 < grp` → `_BoundErr` (`0065178c.c:59-61`); `4 < slot` → `_BoundErr` (`:51-53`).
- Ô: `Ô(g,s) = battle + g*0x46 + s*14 + 0x40`, struct 14 byte:
  `+0x00 X(DWORD) +0x04 Y(DWORD) +0x08 unitId(DWORD, 0=trống) +0x0C unitIdx(BYTE) +0x0D collided`.
- Chứng minh bằng số học khớp tuyệt đối:
  `0x40 + 4×0x46 = 0x158` (offset mảng unit) → `0x158 + 21×4 = 0x1AC` (mảng FX) →
  `+0x200/0x254/0x2A8/0x2FC/0x350` (5 mảng FX phụ, stride `0x54`).
- Phe: `grp < 2` → phe 2 (địch), `grp >= 2` → phe 1 (ta) (`FUN_00646634`).

### 3.2 Layout pos (mức A)

| Frame | Offsets pos trong `P` (`P[0]`=MainOp) |
|:--|:--|
| `0B FA` (mở trận, rec 24B @ `P[5+24i]`) | **GRP=`P[17+24i]`, SLOT=`P[18+24i]`** |
| `0B 05` (rec 23B @ `P[3+23i]`) | **GRP=`P[14+23i]`, SLOT=`P[15+23i]`** |
| `32 01` | src=`P[4],P[5]`; dst=`P[10],P[11]` |
| `33 01` (rec 7B) | `P[2+7i]`, `P[3+7i]` |
| **`35 07`** | **`P[2],P[3]`=src; `P[4],P[5]`=dst — frame DUY NHẤT cập nhật lại ô** |

> **Phát hiện mới (quan trọng):** record `0B 05` (23 byte) là record `0B FA` (24 byte)
> **mất đúng byte `kind` đầu tiên** → mọi offset lệch −1. Điều này **không** có trong
> `opcode_0b.md` cũ.

### 3.3 PET (mức A)

- PET vào lưới bằng `type == 4` trong chính record `0B 05`/`0B FA`; `type` ghi vào `unit+0x79`.
- Client tra `FUN_0071d1bc(owner, id)` trong `owner->0x57C[1..4]` → **`id` phải là pet
  *template code*, không phải instance id**.
- Frame `0F` (nạp pet) phải tới **trước** `0B 05`/`0B FA`.

### 3.4 Đối chiếu Rust

**12/12 frame có pos đều khớp byte 100%** (offset, thứ tự, endian, len frame, 4×5, row/col→grp/slot).
`WarInfo::packet_hex` (`engine.rs:71-88`) khớp 23/23 byte.

**5 chỗ Rust sai:**

| # | Vấn đề |
|:-:|:--|
| 1 | ❌ **Thiếu builder `35 07`.** `reposition()` dùng `0B 01` chỉ dội FX/reset, **không** sửa `cell+0x48/+0x4C` → ô trận client đóng băng sau khi mở; unit chết vẫn chiếm ô. |
| 2 | ❌ **Thiếu builder `33 01`** ⇒ `unit+0x3EA` (HP) không bao giờ cập nhật ⇒ `FUN_00648154` (guard `HP==0`) bắn FX vào unit đã chết. `status_update` (`0x08 01`) không thay thế được. |
| 3 | ⚠️ **Latent bug `team`:** `construction.rs:93,125,140` dùng `if row == 0 {2} else {1}`; client quy tắc là `row < 2` ⇒ phe 2. Chưa lộ vì `add_npc` hard-code `team=2`, nhưng `add_player(row=1)` sẽ sai. |
| 4 | ⚠️ **Tên hàm sai:** `reposition()`/`clear_pet_cell()` không phải frame di chuyển; `troi_start()`/`troi_end()` là FX icon theo `type 1..5`. |
| 5 | ❌ **`BattleRoleData`/`BattleRoleSerializer` (`battle_role.rs`) là dead code sai** — không frame client nào đọc layout đó, và nó ghi `col` trước `row` (ngược client). Nên xoá. |

---

## 4. H-2 — Combo: 100% server-side 🟢

Kết luận chắc chắn (mức **A**, 5 lớp bằng chứng):

1. Frame lệnh C→S mang **đúng 1** mã hành động, **không** có field chain/sequence.
2. `Skill.Dat` (record 86 byte, loader `FUN_00759B2C`) **không có** field "skill kế tiếp";
   7 accessor client chỉ đọc `+0x00/+0x15/+0x16/+0x18/+0x1A/+0x21/+0x2A`.
3. `20006` **được miễn** bước validate "đã học skill" (`005ab678.c:218`:
   `if ((local_10 != 0x4e26) && ...)`) ⇒ skill ảo do server sinh.
4. `FUN_00648154` (241 entry) + 5 method nhỏ chỉ là bảng `code → FX id` thuần trình diễn;
   `20006 → FX 0x95`, `20007 → FX 0x96` (khớp Rust).
5. `unit+0x5A8` chỉ được đọc-để-truyền **đúng 1 lần**, nằm trong builder `0x32` đã chết.

**Quy tắc combo thật** chỉ có ở `runner.rs:1098-1167`:

- skill vật lý (`skill_type == 1`)
- đơn vị kế tiếp **cùng team + cùng mục tiêu**
- `|ΔAGI| <= 800` so với đầu chuỗi hoặc trung điểm
- damage `× 1.3` mỗi mắt xích
- roll 100%

Frame S→C `32 01` cho combo **khớp 100%** với `packets::combo` + `combo_footer_20007`.

> ⚠️ **Gap:** `Skill.combo` đọc từ `Skills.txt` (không tồn tại) và bị `let _ =` bỏ qua
> (`runner.rs:1301`). Cần quyết định: bỏ hẳn field này, hoặc nối vào bảng combo thật.

---

## 5. Kết thúc trận, EXP, ITEM, GOLD

### 5.1 Frame kết thúc (mức A)

- **Không tồn tại frame "battle result"** (thắng/thua/điểm/thời gian) trong client — grep toàn bộ `case_*.c` không có.
- Frame thoát trận thật là **`0B 00 <selfId u32> 00 00`** (`FUN_00641fa4:55-82`). Nó **chỉ** gán
  `player+0x376 = 0` khi giá trị đó **đúng bằng 4**; nếu ≠ 4 thì chỉ ẩn form, **không thoát mode**.
- Rust `packets::hide_from_map` **đã đúng** ✓.
- `33 01` / `34 01` **không** phải frame kết quả (chỉ là set-flag unit / reset-turn-engine).

### 5.2 EXP — chỉ qua `08 01` (mức A)

```text
08 01 <attr:1> <sign:1> <value u32 LE> <value2 u32 LE>     (12 byte)
```

| attr | Ý nghĩa | Đích trong client |
|:-:|:--|:--|
| `0x23` | **LEVEL** | byte `actor+0x3FA`; client **tự tính lại** MaxHP/MaxSP |
| `0x24` | **EXP** | DWORD `actor+0x3FC`, **ghi đè tuyệt đối** |
| `0x25` | Skill point | `actor+0x100` |
| `0x26` | Point | `actor+0x402` |

Rust `packets.rs:302-305` mapping `LV=0x23, TEXP=0x24, SKILL_POINT=0x25, POINT=0x26` — **đúng**.
`status_update` khớp byte-for-byte ✓.

### 5.3 Item & Gold

- `17 05` = dump toàn bộ hòm đồ, rec 12B, **wipe 26 ô trước khi nạp** — Rust `dump_table` khớp ✓.
- **`17 06`** = thêm 1 item (rec 12B, bỏ byte slot) — builder **đã có sẵn** ở `trade_storage.rs:27`.
- `1A 04` = set tuyệt đối gold ✓. **`1A 01` = CỘNG + banner "Nhận được X Tiền"**
  (`FUN_0072b084:41-63`: `*piVar1 = *piVar1 + local_c`) — Rust **không dùng `1A 01` ở đâu cả**.

### 5.4 Rust đang thiếu/lệch

| # | Mức | Vấn đề |
|:-:|:--:|:--|
| **M1** | 🔴 CAO | `apply_drop` (`service.rs:157-178`) chỉ mutate session + broadcast `35 04` → **item không bao giờ tới túi client**. Phải gửi `17 06`. |
| **M2** | 🔴 CAO | `apply_gold` (`service.rs:284-299`) chỉ `gold += amount` → **client không biết vàng**. Phải gửi `1A 01`. |
| **M3** | 🔴 CAO | Thiếu **`05 0A <charId>`** event LevelUP (`0072f970:52-56`, kèm `sound\WA0013.wav`). grep `050A` trong `src/` = 0 hit. |
| **M4** | 🟡 TB | `Stat::Hpmax/Spmax → byte=None` ⇒ không frame MaxHP/MaxSP; nên gửi `08 01` với `CD`/`CE`. |
| **M5** | 🟡 TB | `apply_pet_exp` không gửi gì; frame đúng là `0F 08` (`pet_status_single` đã có sẵn). |
| **M6** | 🟡 TB | Client tự recompute MaxHP bằng **bảng hệ số theo cấp** `gvar_007DA6B8`; Rust dùng `pow(lv, 0.35)` (`engine.rs:100-111`) → HP có thể vượt MaxHP client. |
| **M7** | 🟡 TB | Bảng exp Rust (`texps.rs`: k={2.9,3.0,3.05}, `+5`, banker's rounding) **chưa từng đối chiếu** với client `FUN_0065b0dc` (= `base(lv) + ROUND(lv^k)`). |

---

## 6. Đính chính tài liệu nền & điểm cần lưu ý

### 6.1 Đính chính quan trọng — `1E 01` **KHÔNG** phải stall; `17 1E` mới là stall

Tài liệu nền §3.3.1 gộp nhầm hai frame. Đã đối chiếu bảng dispatch:

| Frame | Handler | Ngữ nghĩa thật |
|:--|:--|:--|
| **`17 1E`** | `case idx 20` (main `0x17`) `case 0x1e:` → `FUN_00730F4C` | **stall/vendor** — tạo `TVenderItem`, UI `StallN` |
| **`1E 01`** | `case idx 26` (main `0x1E`) `case_026:28-29` → `func_0x007707a0` | **container 51 slot** (`player+0xA48`), record 12B |

Bằng chứng trực tiếp trong body `00730f4c_FUN_00730f4c.c:161-168`:
`_LStrCat3(&local_144, "Stall", IntToStr(stall_id))` → `FUN_007c9b38` → `StallN`.

> **Hệ quả:** layout `record 12 byte` mà Rust `dump_tientrang()` gửi kèm `1E 01` **khớp** với
> `1E 01` thật — "mismatch" của gap #6 là do **nhận nhầm opcode** khi gán nhãn stall, không phải
> do sai layout. Câu hỏi còn mở chuyển sang: `tientrang` là container nào
> (`+0xA48`/`+0x9E0`/`+0xB14`/`+0xDC0` = 51/51/26 slot)?
> Chi tiết: [`04_battle_end_rewards.md`](./04_battle_end_rewards.md) §4.4.1.

### 6.2 Đính chính khác

| # | Nội dung |
|:-:|:--|
| 1 | `case_005_0078C7DB_FUN_0078c7db.c` **≠** parser `05` — case 5 là main `0x04`. |
| 2 | `opcode_08.md` §4.2 **đã hoán `0x23`↔`0x25`**; mapping trong Rust là đúng. |
| 3 | `opcode_32.md` §6 nói `FUN_0077f414` chỉ nhận `EAX`/`DL` — **thiếu `CL`**. Đúng là 3 tham số: `EAX`=self, `DL`=MainOp, `CL`=SubOp (`asm:24-26`). |
| 4 | `opcode_0b.md` §2 SubOp `0x05` — client **không** gửi sub `04`/`05`; sub `0x05` không có call-site. |
| 5 | `0B 07` có **3 tham số** (đã chốt mức A): `MOV [EBP-0x6],CL` tại `00733650.asm.txt:13-14`. |
| 6 | `FUN_00654590` **đã có body** — `35 04` append `[b1][b2][id:u16]` vào TList `unit+0x5B0`, **không phải damage**. |
| 7 | `0B 05` record 23B **không** phải "record 23 byte của `0B FA`" — nó là record 24B **mất byte `kind` đầu**. |

---

## 7. Rủi ro cần kiểm tra live

| # | Rủi ro | Bằng chứng |
|:-:|:--|:--|
| R-1 | **`gvar_007DA51C` không có lệnh gán nào** trong toàn bộ 13 176 hàm export — chỉ có lệnh đọc. Nếu thật sự null thì `34 01` (`your_turn`) là no-op **và** client không gửi được lệnh `0x32` C→S. | grep `client_pseudo_c/*.asm.txt`: không có `MOV dword ptr [0x007da51c],*` |
| R-2 | **Kind byte record `0B FA` cho self:** Rust dùng `05` (self) / `03` (leader). Client evidence chỉ có `4` và `{2,5}`. Nếu server thật dùng `04` thì `0B 00` mới reset được mode. **Cần capture.** | `01_battle_start_scene.md` §14 |
| R-3 | **Thứ tự frame là bất biến:** `0B FA` phải trước mọi thứ, vì `0B 0A`/`32`/`33` deref `DAT_0098C63C` không null-guard. `member_battle_frame()` thiếu `battle_trailer()` so với `battle_open_leader()` → bất đối xứng. | `01_battle_start_scene.md` |

---

## 8. Danh sách việc cần làm (xếp theo ưu tiên)

| # | Ưu tiên | Việc | File Rust |
|:-:|:--:|:--|:--|
| 1 | 🔴 P0 | **Redump ASM vùng `0x780440..0x780453` + `0x78048E..0x7807D0`** — byte bảng dispatch `0B` sub `00..04` và builder lệnh trận. Không có bước này thì không thể viết đúng handler. | (decompile) |
| 2 | 🔴 P0 | **Viết lại handler lệnh trận** theo `0x0B` sub `02`/`08` thay vì `0x32`. | `src/server/handlers/battle.rs:189-243` |
| 3 | 🔴 P0 | **Thêm builder `35 07`** (di chuyển 6 byte) — sửa `cell+0x48/+0x4C`. | `src/battle/packets.rs` |
| 4 | 🔴 P0 | **Thêm builder `33 01`** (record 7B: `row, col, propId, value u32`) để cập nhật HP. `propId==0x19` + `value>0` ⇒ hồi sinh có effect. | `src/battle/packets.rs` |
| 5 | 🔴 P0 | **`apply_drop` phải gửi `17 06`.** | `src/battle/service.rs:157-178` |
| 6 | 🔴 P0 | **`apply_gold` phải gửi `1A 01`.** | `src/battle/service.rs:284-299` |
| 7 | 🟠 P1 | Thêm frame **`05 0A <charId>`** LevelUP. | `src/battle/packets.rs` |
| 8 | 🟠 P1 | Sửa `build_join_frame()` — hiện 22 byte, phải 24 (thiếu `row`/`col`). | `src/battle/service.rs:1108-1120` |
| 9 | 🟠 P1 | Sửa quy tắc `team`: `row < 2` ⇒ phe 2 (thay vì `row == 0`). | `src/battle/construction.rs:93,125,140` |
| 10 | 🟠 P1 | Chốt `tientrang` là container nào (`1E 01` rec 12B đã khớp layout; cần xác định slot table). **Không** đổi sang `17 1E`. | `src/server/session.rs:434-437` |
| 11 | 🟠 P1 | Bổ sung các sub `0x35` còn thiếu: `03` (state 7 = animation tấn công), `0E`/`0F` (set state / HP-bar). | `src/battle/packets.rs` |
| 12 | 🟡 P2 | Xoá dead code `battle_role.rs` (layout không tồn tại ở client PC). | `src/protocol/codecs/battle_role.rs` |
| 13 | 🟡 P2 | Đổi `finish` hardcode row 3 (`runner.rs:2790,2866`) → chỉ cộng exp/pet-exp cho người chơi row 3. | `src/battle/runner.rs` |
| 14 | 🟡 P2 | Sửa RNG không tất định: `runner.rs:1341,1567` tạo `time_seeded()` mới → `with_seeds`/golden replay vỡ. | `src/battle/runner.rs` |
| 15 | 🟡 P2 | `handle_use_item` trừ item **trước** khi lệnh được chấp nhận → mất item nếu `owns_commandable_cell` từ chối. | `src/server/handlers/battle.rs:216-240` |
| 16 | 🟡 P2 | Đối chiếu bảng exp Rust với client `FUN_0065b0dc`; đối chiếu hệ số MaxHP với `gvar_007DA6B8`. | `src/data/texps.rs`, `src/battle/engine.rs:100-111` |
| 17 | 🟢 P3 | Capture thật `aLogin.exe` để chốt kind byte `0B FA` self + xác nhận `1A 01`. | — |

---

## 9. Bảng tổng hợp frame battle

### 9.1 S→C (server → client)

| Frame | Ý nghĩa | Layout | Mức | Rust |
|:--|:--|:--|:-:|:--|
| `0B FA` | **Mở trận** | `diahinh u16` + `mode u8` + n×rec 24B | A | ⚠️ thiếu row/col |
| `0B 05` | Nạp entity | `mode u8` + n×rec 23B | A | ✅ |
| `0B 04 02` | Show on map | `u32 id` + tail | A | ✅ |
| `0B 00` | Thoát trận | `selfId u32` + `00 00` | A | ✅ |
| `0B 01` | Reset FX/ô | `grp` + `slot` | A | ⚠️ 3B |
| `0B 0A 01` | Trailer | — | A | ✅ |
| `0B 06` | Broadcast | `u32 id` | A | ✅ |
| `32 01` | Kết quả hành động / combo | `W u16` + rec block 9B×n | A | ✅ |
| `33 01` | Set unit prop | rec 7B: `grp,slot,propId,value u32` | A | ❌ **thiếu** |
| `34 01` | Your turn | (không đọc payload) | A | ✅ |
| `35 01` | Buff start/end | `grp,slot` | A | ✅ |
| `35 03` | State 7 (animation tấn công) | `grp,slot` | A | ❌ thiếu |
| `35 04` | Append effect list | `[b1][b2][id u16]` | A | ⚠️ dùng sai nghĩa |
| `35 05` | Acting | `grp,slot` | A | ✅ |
| `35 07` | **Di chuyển ô** | `srcGrp,srcSlot,dstGrp,dstSlot` | A | ❌ **thiếu** |
| `35 0E/0F` | Set state / HP-bar | `grp,slot` | A | ❌ thiếu |
| `05 04` | Thoát UI trận | — | A | ✅ |
| `05 0A` | **LevelUP** | `charId u32` | A | ❌ thiếu |
| `08 01` | Chỉ số (exp/level/…) | 12B, attr `0x23..0x26` | A | ✅ |
| `17 05` | Dump hòm đồ | n×rec 12B (wipe 26 ô) | A | ✅ |
| `17 06` | **Thêm 1 item** | rec 12B | B | ❌ thiếu (có builder) |
| `1A 01` | **Cộng gold + banner** | `u32 delta` | A | ❌ thiếu |
| `1A 04` | Set gold | `A u32` + `B u32` | A | ✅ |
| `0F 08` | Pet exp/status | exp u32 + level u8 + hp u16 | A | ❌ thiếu |

### 9.2 C→S (client → server) — **chỉ những gì thực sự được gửi**

| Frame | Sub | kind | Ý nghĩa | Rust |
|:--|:--|:--|:--|:--|
| `0B 01` | — | — | Rời trận (confirm) | ⚠️ check |
| `0B 02` | `02` | 2 | PK challenge | ✅ |
| `0B 02` | `02` | 3 | Attack NPC | ✅ |
| `0B 02` | `02` | 4 | Spectate | ✅ |
| `0B 02` | `02` | 5 | Jam | ⚠️ no-op |
| `0B 06` | `06` | — | Broadcast | ✅ |
| `0B 07` | `07` | — | Chế độ đánh tự động + bộ đếm | ❌ thiếu |
| `0B 08` | `08` | 9,0A,0B,0C,0E,12,13 | **Lệnh trận** | ❌ **thiếu** |
| `0B 0A` | `0A` | 15 | — | ❌ thiếu |
| `0B FA` | `FA` | — | Battle open | ✅ |
| ~~`0x32`~~ | 1/2 | — | **CODE CHẾT — client không gửi** | ❌ cần xoá/viết lại |
| ~~`0x1C`~~ | 0,3 | — | **no-op** | ⚠️ |

---

## 10. Nhật ký kiểm chứng chéo (cross-verification log)

Các lượt nghiên cứu song song có **3 điểm tranh chấp**. Tất cả đã được kiểm chứng lại
trực tiếp trên bytecode/ASM:

| # | Tranh chấp | Kết quả kiểm chứng | Cách kiểm |
|:-:|:--|:--|:--|
| 1 | Agent 04: `1E 01` không phải stall, `17 1E` mới là stall | ✅ **Agent 04 đúng** | `jumptable_byte200_0x78A8EE[0x1E]=26` ≠ `[0x17]=20`; `FUN_00730F4C` chỉ được gọi từ `case_020:149` (`case 0x1e:`). Body có `_LStrCat3(..., "Stall", ...)`. |
| 2 | Agent 03: `0x32` C→S là code chết | ✅ **Đúng** | 116 file ASM có call `FUN_0077f414`; tập `MOV DL,<op>` **không có `0x32`**. Đã khôi phục lại layout builder tại `00788BCA` byte-for-byte để đối chiếu. |
| 3 | Agent 01: `0B 07` có 3 tham số | ✅ **Đúng** | `00733650.asm.txt:13-14`: `MOV [EBP-0x6],CL` / `MOV [EBP-0x5],DL` / `MOV [EBP-0x4],EAX`. |

Hai claim **đã bị loại** sau kiểm chứng:

- ❌ "Client không gửi `0x32` vì `case 0x32: break`" — sai lý do (lỗi recovery Ghidra), nhưng
  **kết luận vẫn đúng** vì lý do khác: không có call-site nào truyền `DL=0x32`.
- ❌ "`0x32` nên bị xoá khỏi bảng dispatch" — builder `0x00788BCA` **tồn tại** và được dispatch
  qua `table_0x77F474[0x32]=0x2C`. Chỉ là không có caller.

---

*Tài liệu tổng hợp từ 5 lượt nghiên cứu song song trên `client_pseudo_c/` + đối chiếu
`test/ts_decompile/` và source Rust, ngày 2026-10-01. Mọi claim đã được gắn mức bằng chứng
(A/B/C); các điểm tranh chấp giữa các lượt nghiên cứu đã được kiểm chứng lại độc lập —
xem §10.*
