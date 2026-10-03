# 02 — HIỂN THỊ VỊ TRÍ (POS) CỦA PLAYER / NPC / PET TRONG TRẬN CHIẾN

> Nghiên cứu RE (client Delphi `aLogin.exe`). Nguồn chính: `client_pseudo_c/` (Ghidra pseudo-C + `.asm.txt`).
> Đối chiếu: `test/ts_decompile/` (bản export đầy đủ 13 176 hàm).
> Ngày: 2026-10-01. Mọi layout byte đều kèm `file:dòng`.
>
> **Mức bằng chứng dùng trong tài liệu:**
> **(A)** = đã xác minh trực tiếp từ mã client (pseudo-C *và* đối chiếu `.asm.txt` khi có ý nghĩa byte-offset).
> **(B)** = suy luận có cơ sở từ code (điều kiện ràng buộc / quy tắc byte liên tiếp), chưa có nhánh thực thi xác nhận.
> **(C)** = chỉ là contract phía server Rust (`src/battle/**`), chưa đối chiếu được với client.
>
> Quy ước chỉ số: `P[i]` = byte payload (`P[0]` = MainOp); `RP[i]` = `P[i+1]` ⇒ `RP[0]` = SubOp.
> Delphi `_LStrCopy(s,i,n)` **1-based**. Word LE = `FUN_0077eb9c`, DWORD LE = `FUN_0077ef7c`.
> Guard độ dài `_BoundErr(n)` yêu cầu `Len(RP) > n`.
>
> Tài liệu anh em: `01_battle_start_scene.md` (frame mở trận), `04_battle_end_rewards.md`, `05_rust_battle_audit.md`.
> Tài liệu nền có sẵn: `.scratch/client-pseudo-op-code/opcode_32.md`, `opcode_33.md`, `opcode_35.md`, `opcode_0b.md`.

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — Gap #3 ĐÃ ĐÓNG. Xem §0.1**

---

## 0.1 🔴 ROUND-4 REDUMP — `TFightField.Create` ĐÃ CÓ BODY

### 0.1.1 Gap #3 đóng

> §76-77, §158, §737: *"`TFightField.Create` @ `0x0063FADC` — **không có body** trong export"*
> *"HOLE `0x0063F9F3 … 0x00640984` (3 985 B)"*

**ĐÃ ĐÓNG.** `ts_decompile/functions/0063fadc_TFightField.Create.{c,asm.txt}` tồn tại
(670 dòng ASM). Ghidra đã tạo function trong lần redump này.

### 0.1.2 ⭐ Hằng số lưới — công thức pixel MỘT PHẦN đã lộ

Đọc prologue constructor (`0063fadc_TFightField.Create.asm.txt:33-56`):

| Offset | Lệnh | Ý nghĩa |
|:-:|---|---|
| `+0x08` | `MOV dword ptr [EAX+0x8], 0x340` | **832** = bề rộng canvas |
| `+0x0C` | `MOV dword ptr [EAX+0xc], 0x280` | **640** = chiều cao canvas |
| `+0x10` | `MOV dword ptr [EAX+0x10], EDX` | tham số (mode) |
| `+0x14` | `MOV dword ptr [EAX+0x14], EDX` | tham số |
| `+0x2C` | `MOV byte ptr [EAX+0x2c], 0x1` | cờ |
| `+0x30` | `MOV dword ptr [EAX+0x30], 0x154` | **340** = bề rộng 1 ô |
| `+0x34` | `MOV dword ptr [EAX+0x34], 0xb4` | **180** = chiều cao 1 ô |
| `+0x38` | `MOV dword ptr [EAX+0x38], 0x2` | **2** |
| `+0x3C` | `MOV dword ptr [EAX+0x3c], 0x5` | **5** |
| `+0xE59` | `MOV byte ptr [EDX+0xe59], AL` | copy tham số byte |
| `+0xE5B` | `MOV byte ptr [EAX+0xe5b], 0xff` | sentinel |

**Bộ hằng lưới: canvas `832 × 640`, ô `340 × 180`, mặc định `2 × 5`.**
(832 = `0x340`, 640 = `0x280`, 340 = `0x154`, 180 = `0xB4`.)

⚠️ **Lưu ý trùng tên:** `0x340` vừa là giá trị ở `+0x08` vừa là hằng tại `+0x30` —
đây là **nghịch lý giữa kích thước lưu trữ và giá trị**, cần đọc kỹ `.c` khi viết code.

### 0.1.3 ❌ Chưa thu hồi đầy đủ — công thức `cell.X` / `cell.Y`

Constructor **cấp phát và khởi tạo**, nhưng **không** chứa phép `IDIV` tính
`cell.X`/`cell.Y`. Trong 670 dòng ASM chỉ có **một** `IDIV`, và nó không liên quan lưới:

```asm
0074035d: MOV ECX, 0x6
00740362: CDQ
00740363: IDIV ECX            ; toa do / 6  —  KHONG phai luoi tran
```

⇒ **Công thức ánh xạ pixel → ô vẫn chưa recover.** Việc này **không chặn** mock server
(server không gửi pixel) nhưng vẫn chặn việc dựng lại lưới bằng toạ độ ngoài client thật.
`VMT_63CC84_TFightField` cũng đã được resolve (xem `ts_decompile/redump/round3_report.txt`,
mục `VMT`) — các method ảo sẽ chỉ ra chỗ tính toạ độ.

### 0.1.4 Đối chiếu các gap khác

| Gap | Nội dung | Trạng thái Round-4 |
|:-:|---|---|
| #1 | không có frame `35 07` (move cell) | ❓ **vẫn mở** — cần capture hoặc tìm call-site |
| #3 | `TFightField.Create` | ✅ **đóng** (§0.1.1) |
| #8 | FX ids `FUN_0066BF0C`… | ⚠️ ASM `0x0066BF0C..0x0066C258` đã dump (844 B / 100%) |
| — | `FUN_00654590` | ✅ **đã có body từ trước** — `battle/03:512` sai (xem `battle/03 §0.1.7`) |

### 0.1.5 ✅ Xác nhận `0x0B` sub 8 = **8 byte** (khác sub 2 = 9 byte)

`battle/03 §0.1.3` chứng minh sub `2` dùng `FUN_0077EB1C` (u16 → **2 byte** cho
`TFightManage+0x22`) còn sub `8` dùng `@LStrFromChar` (→ **1 byte**).
⇒ `battle.rs:35-46` (`handle_battle` sub 8) đọc `payload[1..5]` là đúng.

## §0. Sơ đồ: client render grid từ data nào

```
                    ┌──────────────────────────────────────────────┐
  S→C  0B FA ──────►│ FUN_006438ac  →  TFightField.Create @0x63CC84  │
  (mở trận)         │  gán: DAT_0098C63C = <TFightField>            │  (A)
                    │  ★ điểm cấp pháp DUY NHẤT của DAT_0098C63C      │
                    └───────────────┬──────────────────────────────┘
                                    │
   ┌────────────────────────────────▼───────────────────────────────────────┐
   │ TFightField (battle-record), layout đã SỮA LẠI (mức A)                 │
   │                                                                       │
   │  +0x00 .. +0x3F : header / cờ / bộ đếm lượt                            │
   │  +0x40 ┌────────────────── LƯỚI 4 nhóm × 5 ô = 20 ô, stride 14 byte ──┐ │
   │        │  nhóm g (0..3), gánh đều 0x46 = 70 = 5×14                     │ │
   │        │  Ô(g,s) = battle + g*0x46 + s*14 + 0x40                       │ │
   │        │    +0x00  DWORD  X        (toạ độ pixel màn hình)             │ │
   │        │    +0x04  DWORD  Y                                             │ │
   │        │    +0x08  DWORD  unitId   (0 ⇒ ô TRỐNG)                      │ │
   │        │    +0x0C  BYTE   unitIdx  (0..20, index vào mảng unit)        │ │
   │        │    +0x0D  BYTE   collided (ô đã có người, bị chồng)          │ │
   │        └───────────────────────────────────────────────────────────────┘ │
   │  +0x40 + 4*0x46 = +0x158  ← kết thúc lướI, KHỚP CHÍNH XÁC               │
   │  +0x158  mảng 21 con trỏ UNIT      (index 0..20)  ← unitIdx              │
   │  +0x1AC  mảng 21 con trỏ FX chính                                        │
   │  +0x200  mảng 21 con trỏ FX loại 1 (0x35 SubOp 1, type=1)                │
   │  +0x254  mảng 21 con trỏ FX loại 2 (type=2)                              │
   │  +0x2A8  mảng 21 con trỏ FX loại 3 (type=3)                              │
   │  +0x2FC  mảng 21 con trỏ FX loại 4 (type=4)                              │
   │  +0x350  mảng 21 con trỏ FX loại 5 (type=5)                              │
   │  +0x3A4  BYTE bộ đếm "vòng" (reset khi >0x13)                            │
   │  +0xE5A  BYTE timer; +0xE5B / +0xE68  BYTE selected-unitIdx (−1 = none)  │
   └───────────────────────────────────────────────────────────────────────┘
             │                                    │
   tra ô     │ FUN_0065178c(battle, g, s)        │ đọc vị trí pixel
   (A)        │  if cell[g][s].unitId==0 → 0xFFFFFFFF                        │
             │  else → cell[g][s].unitIdx        │ FUN_006508ec(unit,&out)
             │ 0065178c.c:51-53 (s≤4), :59-61 (g≤3), :68-70, :88            │
             ▼                                    ▼
   unit = battle->unit[unitIdx]                  out[0] = cell.X ; out[1] = cell.Y
             │                                    006508ec.c:51-105   (A)
             │  unit->0x56E = g  (grp)     ← ghi khi "đặt vào ô"
             │  unit->0x56F = s  (slot)         00646028.c:189,193 (A)
             │  unit->0x54  = cell.X            00646028.c:235 / 0064e3e0.c:344
             │  unit->0x58  = cell.Y            00646028.c:255 / 0064e3e0.c:370
             │  unit->0x1C/0x20 = 0x54/0x58     00646028.c:256-257
             │  unit->0x4C/0x50 = 0x54/0x58     00646028.c:258-259
             ▼
   RENDER: FUN_0065130c đọc unit->0x54 / unit->0x58 rồi
           FUN_007c9474(canvas, imgHandle, X - w/2 + 2, …, srcRect)
           0065130c.c:97-115  (A)
```

**Điểm mấu chốt (A):** client **không** tự tính toạ độ pixel. Nó đọc `cell.X/cell.Y`
(cặp DWORD ở `cell+0x00/+0x04`) và copy sang `unit+0x54/+0x58`. Giá trị pixel này được ghi
một lần trong `TFightField.Create` — **hàm đó không có body trong export** (`0x0063FADC` nằm
trong khoảng trống `0x0063F9F3 … 0x00640984`, xem §5). Server **không cần** gửi pixel:
chỉ cần gửi đúng `(grp, slot)`.

**Bằng chứng ba trụ cột của lưới (A, xác minh qua 3 hàm độc lập):**

| Hằng | Ý nghĩa | Nguồn |
| :-- | :-- | :-- |
| `0x23` (nhân rồi ×2 ⇒ `0x46`) | gánh đều **1 nhóm** = 70 byte = 5 ô × 14 | `0065178c.c:63`, `0064cfec.c:74`, `0064e3e0.c:116` |
| `7` (nhân rồi ×2 ⇒ 14) | gánh đều **1 ô** = 14 byte | `0065178c.c:54`, `0064cfec.c:84` |
| `0x48` / `0x4C` | offset của `unitId` / `unitIdx` trong ô (khác `0x40`/`0x44` = X/Y) | `0065178c.c:68,88` vs `0064e3e0.c:338,364` |

Tính nhất quán: `0x40 + 4×0x46 = 0x158` (đúng bằng offset mảng unit) và `0x158 + 21×4 = 0x1AC`
(đúng bằng offset mảng FX) — xác nhận lưới **20 ô, 14 byte/ô, bắt đầu tại `+0x40`**, không có
lưới 5×4 hay 25 ô.

---

## §1. Cấu trúc lưới trận

### 1.1 Kích thước & công thức index ↔ (grp, slot)

| Đại lượng | Giá trị | Mức | Bằng chứng |
| :-- | :-- | :--: | :-- |
| Số nhóm `grp` | **4** (0..3) | A | `0065178c.c:59-61` `_BoundErr` khi `3 < param_2`; `0064cfec.c:71-73`; `0064e3e0.c:113-115` |
| Số ô mỗi nhóm `slot` | **5** (0..4) | A | `0065178c.c:51-53` `_BoundErr` khi `4 < param_3`; `0064cfec.c:80-83` |
| Tổng ô | **20** | A | 4 × 5 |
| Byte mỗi ô | **14** (`0x0E`) | A | `0065178c.c:54` (`*7` ×2), `0064cfec.c:84` |
| Byte mỗi nhóm | **70** (`0x46`) | A | `0065178c.c:63` (`*0x23` ×2) |
| Offset lưới | `+0x40` (X) … `+0x4E` | A | `0064e3e0.c:338` đọc `+0x40`, `:364` đọc `+0x44`, `:180` đọc `+0x48`, `:131` đọc `+0x4C` |

**Công thức (A):**

```
Ô(g, s)      = battle + g*0x46 + s*14 + 0x40          (g ∈ 0..3, s ∈ 0..4)
Ô.unitId     = *(DWORD *)(Ô + 0x08)     // 0 ⇒ trống
Ô.unitIdx    = *(BYTE  *)(Ô + 0x0C)     // 0..20
Ô.collided   = *(BYTE  *)(Ô + 0x0D)     // 1 nếu ô đã bị chiếm khi set
Ô.X          = *(DWORD *)(Ô + 0x00)
Ô.Y          = *(DWORD *)(Ô + 0x04)

index phẳng  = g*5 + s                 ∈ 0..19
ngược lại:   g = index / 5 ;  s = index % 5
```

**Lưu ý (A):** client **không** dùng `index/5` hay `%5` ở bất kỳ đâu trong code đã export —
nó luôn dùng cặp `(grp, slot)` và nhân trực tiếp. Công thức phẳng chỉ là hệ quả số học (B cho
tên gọi "index phẳng"), nhưng rất hữu ích để hiểu thứ tự bộ nhớ `battle+0x40 … +0x158`.

### 1.2 Ngữ nghĩa `grp` (A + B)

`grp` **KHÔNG phải "hàng"** theo nghĩa đồ họa đơn giản — nó là **cặp ( phe, hàng )**:

| `grp` | phe (`unit+0x163`) | hướng sprite (`unit+0xE3`) |
| :--: | :-- | :-- |
| 0 | 2 | `0x11` |
| 1 | 2 | `0x11` |
| 2 | 1 | `0x10` |
| 3 | 1 | `0x10` |

- Phe: `FUN_00646634(grp)` trả `2` nếu `grp < 2`, `1` nếu `2 ≤ grp < 4` →
  `00646634_FUN_00646634.c:23-38` **(A)**; lưu vào `unit+0x163` tại `00646028.c:194-195` **(A)**.
- Hướng sprite: `00646028.c:196-212` — `team==1 → unit+0xE3 = 0x10`,
  `team==2 → unit+0xE3 = 0x11`, trừ khi `unit+0x2A == 0x0C` thì ghi `unit+0xE3 = 0` và
  `unit+0xFA = 0` **(A)**.
- Ngoài ra phe **thật sự** của một đơn vị không lấy từ `grp` mà từ **bản đồ của chủ**:
  `FUN_00635258(gvar_007D9C70, owner->0x5F0)` → `unit+0x16F` tại `00646028.c:117-152` **(A)**.
  `grp` chỉ quyết định hướng/row hiển thị.
- Phép đối xứng phe: khi role = `0x5F0`, ô đối ứng là `grp' = 3 - grp` tại
  `00646028.c:76-92` **(A)** ⇒ xác nhận mạnh `grp` có 4 giá trị và đối xứng 0↔3, 1↔2.

**Kết luận (B):** `grp 0,1` = phe **địch** (hàng trước + hàng sau), `grp 2,3` = phe **ta**.
Rust đang đặt player ở `row 2,3` và NPC ở `row 0,1` ⇒ **khớp**.

### 1.3 Mảng unit `battle+0x158` (21 phần tử)

| Thuộc tính | Giá trị | Mức | Bằng chứng |
| :-- | :-- | :--: | :-- |
| Offset | `+0x158` | A | `0064cfec.c:115`, `0064e3e0.c:159,313` |
| Số phần tử | **21** (index `0..20`) | A | guard `0x14 < uVar2 → _BoundErr` tại `0064cfec.c:112-114`, `0064e3e0.c:132-134,310-312`; vòng quét `i != 0x15` tại `006547a8.c:22-62`, `00651824.c:27-36` |
| Kiểu | con trỏ unit (int*) | A | `*(int **)(DAT_0098c63c + 0x158 + uVar2*4)` |
| Index `0` | **dành riêng cho chính người chơi** | A | `00645ebc.c:88-92`: nếu actor là `gvar_007DA7BC` và `kind ∈ {1,2}` và `unit[0]->0x78 == 0` ⇒ trả `0`; `00644164.c:24-27` bắt đầu quét từ **1** |
| Cấp phát | **KHÔNG có body** — constructor `0x0063FADC` | — | xem §5 |
| `unit+0x04` | DWORD id của đơn vị (dùng để tra cứu) | A | `00646664.c:93` ghi vào `cell.unitId`; `006441bc_FUN_006441bc.c:44` so khớp |
| `unit+0x24` | BYTE unitIdx (bản sao) | A | `00646664.c:117` — `cell+0x4C := (BYTE)unit->0x24` |
| `unit+0x78` | BYTE "còn sống/đang dùng" (0 = slot trống) | A | `00644164.c:33`, `00645ebc.c:88` |
| `unit+0x79` | BYTE **kind** (xem §2.2) | A | `00646a34.c:107`, `00659358.c:52` |
| `unit+0x391` | BYTE **state** (7 = đang di chuyển/đánh) | A | `00647708.c` (theo `opcode_35.md` §4.1) |
| `unit+0x56E` | BYTE **grp** hiện tại | A | `00646028.c:189`, `0064e3e0.c:313` |
| `unit+0x56F` | BYTE **slot** hiện tại | A | `00646028.c:193`, `0064e3e0.c:318` |
| `unit+0x54 / 0x58` | DWORD X / Y (ô đang đứng) | A | `00646028.c:235,255`, `0064e3e0.c:344,370` |
| `unit+0x1C/0x20` | DWORD X / Y (bản sao "render") | A | `00646028.c:256-257`, `0064e3e0.c:381,392` |
| `unit+0x4C/0x50` | DWORD X / Y (bản sao "path") | A | `00646028.c:258-259`, `0064e3e0.c:403,414` |
| `unit+0x5A8` | WORD **skill id đang chọn** (init `0x2710` = 10000) | A | `0063cda0_TFightHum.Create.asm.txt:48` (`MOV word [EAX+0x5a8],0x2710`), `00658c30_FUN_00658c30.asm.txt:91,127,163` |

**Cấp phát index (A):** `FUN_00645ebc(battle, id, kind)` —
`00645ebc.c:24-52`: nếu `battle->0x3A4 > 0x13` thì dọn 2 vòng (unit `state==7` → `FUN_006540b0`;
unit `+0x548 != 0` → `FUN_00664ffc`), nếu không có gì dọn được ⇒ trả `-1` (unit **không** được
thêm vào trận!). Nếu không thì `battle->0x3A4++` và trả `FUN_00644164(battle)` = index trống
đầu tiên trong 1..20. ⇒ **mỗi vòng đấu chỉ nhận tối đa ~20 lần `0B 05`/`0B FA`; quá số lượt
client sẽ âm thầm bỏ qua đơn vị mới.**

### 1.4 Hàm tra ô `FUN_0065178c` — đọc kỹ (A)

`test/ts_decompile/functions/0065178c_FUN_0065178c.c:38-91`

```c
uint FUN_0065178c(int battle, uint grp, uint slot) {          // :38
  uVar1 = slot;
  if (4 < slot) uVar1 = _BoundErr(slot);                     // :51-53  slot ≤ 4
  uVar2 = (uint)((longlong)(int)uVar1 * 7);                  // :54     slot*7
  if ((longlong)(int)uVar2 != (longlong)(int)uVar1 * 7) uVar2 = _IntOver(uVar2);
  uVar1 = grp;
  if (3 < grp) { uVar2 = _BoundErr(uVar2); uVar1 = extraout_EDX; }  // :59-62 grp ≤ 3
  iVar3 = (int)((longlong)(int)uVar1 * 0x23);                // :63     grp*0x23
  if ((longlong)iVar3 != (longlong)(int)uVar1 * 0x23) { … }
  if (*(int *)(battle + iVar3*2 + 0x48 + uVar2*2) == 0)       // :68     cell.unitId == 0 ?
    local_14 = 0xffffffff;                                   // :69     → ô TRỐNG
  else {                                                     // :72-88  recompute cùng công thức
    local_14 = (uint)*(byte *)(battle + iVar3*2 + 0x4c + uVar2*2);   // :88  cell.unitIdx
  }
  return local_14;
}
```

- Công thức **chính xác**: `battle + grp*0x46 + slot*14 + 0x48` (và `+0x4C`) **(A)**.
  Lưu ý base dùng ở đây là `0x48`, tức `Ô(g,s).unitId` — tức `+0x40` chỉ là `X`.
- Trả `0xFFFFFFFF` (=255) nếu ô trống; mọi caller đều so `!= 0xffffffff` trước khi index
  (`0064cfec.c:89`, `0064e3e0.c:131`, `0064cae0.c:122`, `00647164.c:200`, `0064cd74.c:112`).
- **Caller list** (`0065178c.c:7-34`): `FUN_0064b374`, `FUN_00652188`, `FUN_00653020`,
  `FUN_00663844` — cộng 16 call-site được liệt kê ở phần header. Đây là **helper tra ô duy
  nhất** của họ trận ⇒ mọi frame "theo (grp,slot)" đều đi qua đây.
- ⚠️ Hàm **không guard `battle != 0`** (deref thẳng `param_1`) ⇒ `0B 05`/`0B FA`/`33 01`/
  `32 01/02`/`35 01/04/05/07/0C/0E/0F` gửi **ngoài trận** sẽ EAccessViolation.

### 1.5 Flush FX `FUN_00651824` — đọc kỹ (A)

`test/ts_decompile/functions/00651824_FUN_00651824.c:21-37`

```c
void FUN_00651824(int battle) {
  local_c = 0;
  do {
    uVar1 = local_c;
    if (0x14 < local_c) uVar1 = _BoundErr(local_c);            // :30-32
    FUN_00664ffc(*(int *)(battle + 0x1ac + uVar1 * 4));         // :33  free FX[unitIdx]
    local_c = local_c + 1;
  } while (local_c != 0x15);                                    // :35  đúng 21 lần
}
```

- Duyệt **21** khe FX chính tại `battle+0x1AC` và `TObject.Free` chúng.
- **Không** đụng 5 mảng FX phụ `+0x200/+0x254/+0x2A8/+0x2FC/+0x350` — những mảng đó được dội
  riêng trong `FUN_0064bb24` (0x0B SubOp 1) với id `0x0B` (`0064bb24.c:113-133`).
- Caller: `00651824.c:7-9` → `FUN_0064cfec` (0x35 SubOp 3), `FUN_00650a24` (0x34 SubOp 1);
  thêm call-site `0064d098` (= `FUN_0064cd74`, 0x35 SubOp 1) và `00647164`/`0064dd74`
  (0x32 SubOp 1/2) — tức **mọi frame "bắt đầu nhịp" đều xả sạch FX trước**.
- ⚠️ Không guard null ⇒ `0x35 01`, `0x35 03`, `0x32 01`, `0x32 02` gửi ngoài trận = AV.

### 1.6 Ghi vào ô — `FUN_00646664` (A) — "occupy cell"

`test/ts_decompile/functions/00646664_FUN_00646664.c:23-120`, chữ ký
`void FUN_00646664(int battle, int unit, uint grp, uint slot)`.

| Bước | Ghi | Dòng |
| :-- | :-- | :-- |
| 1 | nếu `cell.unitId != 0` ⇒ `cell+0x4D = 1` (cờ chồng ô) | `:54-58` (set ở `:58`) |
| 2 | `cell+0x48 (DWORD) = unit->0x04` (id) | `:93` |
| 3 | `cell+0x4C (BYTE)  = unit->0x24` (unitIdx) | `:117` |

Chỉ có **một** caller: `FUN_00646028` tại `00646028.c:214-215` ⇒ **ô chỉ được chiếm đúng
một lần, trong lúc "đặt đơn vị vào trận" (`0B 05` / `0B FA`)**. Mọi frame khác chỉ *đọc* ô.

### 1.7 Bỏ ô — `0x35 SubOp 7` (`FUN_0064e3e0`) — "move unit" (A)

`client_pseudo_c/0064e3e0_FUN_0064e3e0.c`, wire = `[35][07][srcGrp][srcSlot][dstGrp][dstSlot]` (đọc ở `:82-111`).

| Bước | Ghi | Dòng |
| :-- | :-- | :-- |
| 0 | nếu unit ở ô **đích** có `state(+0x391)==7` ⇒ `FUN_006540b0(unit)` (huỷ animation) | `:112-160` |
| 1 | `dstCell+0x48 (DWORD) := srcCell+0x48` (chép **id**) | `:161-204` |
| 2 | `dstCell+0x4C (BYTE)  := srcCell+0x4C` (chép **unitIdx**) | `:205-248` |
| 3 | `srcCell+0x48 := 0` (ô nguồn trống) | `:268` |
| 4 | `srcCell+0x4C := 0` | `:288` |
| 5 | `unit->0x56E := dstGrp`, `unit->0x56F := dstSlot` | `:313`, `:318` |
| 6 | `unit->0x54 := dstCell+0x40` (X), `unit->0x58 := dstCell+0x44` (Y) | `:344`, `:370` |
| 7 | `unit->0x1C/0x20/0x4C/0x50 := unit->0x54/0x58` | `:381,392,403,414` |

⚠️ **Không guard `battle != 0`** (`0064e3e0.c:120-131` deref thẳng) ⇒ gửi `35 07` ngoài trận = AV.

> **Đây là frame DUY NHẤT cập nhật lại vị trí sau khi trận bắt đầu.** Xem §5 khoảng trống #1.

### 1.8 Ô trống / "unit chết" — cách client biết ô đã trống

`cell+0x48` **chỉ** về 0 qua 2 đường: `FUN_00646664` khi set lại, và `0x35 SubOp 7` khi
di chuyển. **Không có frame nào xoá ô khi unit chết** — `unit+0x78` mới là cờ "còn dùng".
⇒ Về mặt cấu trúc, client giữ nguyên `cell.unitId` cho tới khi server gửi `35 07` (A, suy ra
từ việc không tồn tại lệnh ghi `0` nào khác trong toàn bộ 49 hàm tham chiếu `DAT_0098C63C`).

---

## §2. Từng frame mang pos

Quy ước: `P[0]` = MainOp, `RP[0]` = `P[1]` = SubOp. Mọi record tính từ `RP`.

### 2.0 Bảng tổng hợp — frame nào mang pos

| Frame | SubOp | Mang (grp,slot)? | Byte offset trong `P` | Mức | Callee |
| :-- | :--: | :--: | :-- | :--: | :-- |
| `0B FA` | `0xFA` | ✔ **đặt đơn vị** | record 24B: `P[3+24i+12]`=grp, `P[3+24i+13]`=slot | A | `FUN_006438ac` |
| `0B 05` | `0x05` | ✔ **đặt đơn vị** | record 23B: `P[2+23i+11]`=grp, `P[2+23i+12]`=slot | A | `FUN_00644294` |
| `0B 01` | `0x01` | ✔ | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064bb24` |
| `0B 00` | `0x00` | ✘ (chỉ id) | — | A | `FUN_00641fa4` |
| `0B 04` | `0x04` | ✘ | — | A | `FUN_00642244` |
| `0B 0A` | `0x0A` | ✘ | — | A | `FUN_0064e978` |
| `32 01` | `0x01` | ✔ **nguồn + đích** | record biến thiên: `P[3]`=srcGrp, `P[4]`=srcSlot, `P[9]`=dstGrp, `P[10]`=dstSlot | A | `FUN_00647164` |
| `32 02` | `0x02` | ✔ (chỉ đọc, không dùng) | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064dd74` |
| `33 01` | `0x01` | ✔ (định danh unit) | record 7B: `P[2+7i]`=grp, `P[3+7i]`=slot | A | `FUN_0064cae0` |
| `35 01` | `0x01` | ✔ | record 5B: `P[2+5i]`=grp, `P[3+5i]`=slot | A | `FUN_0064cd74` |
| `35 03` | `0x03` | ✔ | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064cfec` |
| `35 04` | `0x04` | ✔ | `P[4]`=grp, `P[5]`=slot | A | `FUN_0064d128` |
| `35 05` | `0x05` | ✔ | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064dfa4` |
| `35 07` | `0x07` | ✔✔ **di chuyển** | `P[2]`=srcGrp,`P[3]`=srcSlot,`P[4]`=dstGrp,`P[5]`=dstSlot | A | `FUN_0064e3e0` |
| `35 0C` | `0x0C` | ✔ | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064faa8` |
| `35 0E` | `0x0E` | ✔ | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064fbd8` |
| `35 0F` | `0x0F` | ✔ | `P[2]`=grp, `P[3]`=slot | A | `FUN_0064fd34` |
| `35 06` | `0x06` | ✘ (spawn THuman ngoài lưới) | — | A | `FUN_0064e064` |
| `35 08/09/0A/0B/0D` | — | ✘ | — | A | xem `opcode_35.md` §3 |
| `34 01` | `0x01` | ✘ (reset lượt) | — | A | `FUN_00650a24` |

---

### 2.1 `0B FA` — MỞ TRẬN, đặt đơn vị (record 24 byte) **(A)**

Handler: `client_pseudo_c/case_011_0078D5D1_FUN_0078d5d1.c:116-117`
(`else if (iVar2 == 0xfa) { func_0x006438ac(*(undefined4*)gvar_007D9CE4, RP); }`).
Body: `test/ts_decompile/functions/006438ac_FUN_006438ac.c`.

```
P[0]   = 0x0B          MainOp
P[1]   = 0xFA          SubOp                       → RP[0]
P[2..3]   u16 LE  terrain / "Diahinh"              → RP[1..2]   truyền vào
P[4]   = kindOut  (byte out-param, xem ghi chú)    → RP[3]
P[5..]  record 24 byte × n,  bắt đầu tại P[5]     → RP[4]      (0-based RP[3])
```

Header đọc ở `006438ac.c:87-110`; `TFightField_Create` + gán `DAT_0098C63C` ở
`006438ac.c:114`; asm xác nhận `CALL 0x0063fadc` / `MOV [0x0098c63c],EAX` tại
`006438ac_FUN_006438ac.asm.txt:55-57`.
Số record: `n = (Len(RP) - 3) / 0x18` (`006438ac.c:112`), record `i` bắt đầu tại
0-based `RP[3 + 24i]` (`006438ac.c:129-137` + các `_LStrCopy` tại `:141-…`).

| rec off | `P` offset | Kiểu | Tên field (suy luận từ nơi dùng) | Nơi client dùng | Mức |
| :--: | :--: | :-- | :-- | :-- | :--: |
| +0 | `P[5+24i]` | u8 | **kindOut** (out-param `&local_12`) | `case 2 & id==self`: ghi vào `selfActor+0x376` (`006438ac.c:262`); `FUN_00646028.c:207-212` đọc `*param_4` | A |
| +1 | `P[6+24i]` | u8 | **kind / type** (2,3,4,6,7,9,0B,0C,0D,0E,0F,10,12) | `switch(local_f)` `006438ac.c:250`; → `unit+0x79` | A |
| +2..+5 | `P[7..10+24i]` | u32 LE | **id** (id nhân vật / NPC / **template pet**) | `FUN_0070c20c(registry,id)`; type 3 → `gvar_007DA6DC[id]`; type 4 → `FUN_0071d1bc(owner,id)` | A |
| +6..+7 | `P[11..12+24i]` | u16 LE | **idNpcOnMap** (chỉ dùng khi `type==3`) | `gvar_007DA6DC + w*4`, biên ≤100 (`:280-283`) | A |
| +8..+11 | `P[13..16+24i]` | u32 LE | **idChar** (chủ / master id) | `FUN_00646028.c:130-146` dùng để tra owner | A |
| **+12** | **`P[17+24i]`** | **u8** | **GRP** (0..3) | `FUN_00646028.c:189` → `unit+0x56E`; → `cell+0x08` qua `FUN_00646664` | **A** |
| **+13** | **`P[18+24i]`** | **u8** | **SLOT** (0..4) | `FUN_00646028.c:193` → `unit+0x56F` | **A** |
| +14..+23 | `P[19..28+24i]` | 10 B | blob → `unit+0x404`(hpMax,u16) `+0x406`(spMax,u16) `+0x3EA`(hp,u16) `+0x3EC`(sp,u16) `+0x3FA`(lv,u8) `+0x3E9`(element,u8) | `00646028_FUN_00646028.asm.txt:315-332` (copy 10 byte `MOVSD/MOVSD/MOVSW` tại `:11-13`) | A |

Từng lần đọc (mỗi lần có `_BoundErr` len trước): `local_12` `:138`, `local_f` `:149`,
`local_1c` `:157`, `local_26` `:165`, `local_20` `:173`, `local_10`=GRP `:184`,
`local_11`=SLOT `:195`, `local_30/0x2e/0x2c/0x2a` (4 Word) `:203-231`,
`local_28` `:241`, `local_27` `:250`.

**Đối chiếu byte-tay (A, khớp tuyệt đối):**
10 byte `P[19+24i .. 28+24i]` được `FUN_00646028` copy **liên tiếp 10 byte** từ `param_5`
(`.asm.txt:11-13`: `MOVSD/MOVSD/MOVSW`) rồi tách 6 field (`.asm.txt:315-332`) —
chính xác là `hp_max(2) sp_max(2) hp(2) sp(2) lv(1) element(1)`.

### 2.2 `0B 05` — NẠP ENTITY VÀO LƯỚI (record 23 byte) **(A)**

Handler: `case_011_0078D5D1_FUN_0078d5d1.c:70-71` (`case 5:` → `FUN_00644294`).
Body: `client_pseudo_c/00644294_FUN_00644294.c`.

```
P[0]  = 0x0B
P[1]  = 0x05                      → RP[0]
P[2]  = kindOut (byte out-param)  → RP[1]   ⚠ đọc MỘT LẦN trước vòng lặp
P[3..] record 23 byte × n         → RP[2 + 23i]
```

- `RP[1]` đọc **một lần duy nhất** trước vòng lặp và truyền **con trỏ** cho mọi callee
  (`00644294.c:115`; xác nhận asm `00644294_FUN_00644294.asm.txt:38-44`, và `[EBP-0x9]`
  chỉ được ghi đúng 1 lần — grep toàn file asm: `MOV byte ptr [EBP + -0x9],AL` chỉ ở dòng 42).
  Callees có thể **ghi lại** nó (`FUN_00646028.c:210-212`: `if (*param_4=='d') *param_4=5;`).
  ⇒ Với frame nhiều record, byte này là **dùng chung**, không phải per-record.
- `n = (Len(RP) - 2) / 0x17` (`00644294.c:121`); record `i` tại 0-based `RP[2 + 23i]`
  (`00644294.c:133-141`).
- Nếu `DAT_0098C63C == 0` ⇒ banner `"Error: FightManage.SupportRoleInfo"` và **return**
  (`00644294.c:101-104`) ⇒ an toàn ngoài trận (khác `35 07`).

| rec off | `P` offset | Kiểu | Tên field | Nơi client dùng | Mức |
| :--: | :--: | :-- | :-- | :-- | :--: |
| +0 | `P[3+23i]` | u8 | **kind/type** | `switch(local_e)` `00644294.c:246` | A |
| +1..+4 | `P[4..7+23i]` | u32 LE | **id** | `FUN_0070c20c(registry,id)`; type 3 → `gvar_007DA6DC[w]`; type 4 → `FUN_0071d1bc(owner,id)` | A |
| +5..+6 | `P[8..9+23i]` | u16 LE | **idNpcOnMap** (chỉ `type==3`) | `00644294.c:212-217` | A |
| +7..+10 | `P[10..13+23i]` | u32 LE | **idChar** (master/owner) | `FUN_00646028.c:130-146` | A |
| **+11** | **`P[14+23i]`** | **u8** | **GRP** (0..3) — đọc `:180` | `FUN_00646028.c:189` | **A** |
| **+12** | **`P[15+23i]`** | **u8** | **SLOT** (0..4) — đọc `:191` | `FUN_00646028.c:193` | **A** |
| +13..+22 | `P[16..25+23i]` | 10 B | blob10 → `unit+0x404/0x406/0x3EA/0x3EC/0x3FA/0x3E9` | `00646028_FUN_00646028.asm.txt:315-332` | A |

Từng lần đọc: `local_e` `:145`, `local_14`=id `:153`, `local_22`=idNpcOnMap `:161`,
`local_18`=idChar `:169`, `local_f`=GRP `:180`, `local_10`=SLOT `:191`,
4 Word `local_2c/2a/28/26` `:200-247`, `local_24` `:254`, `local_23` `:262`.

⚠️ **Lưu ý quan trọng (A):** record 23B **mất byte `kindOut`** so với record 24B của `0B FA`.
Cùng một bản ghi logic, `0B 05` dùng `RP[1]` (một lần cho cả frame), `0B FA` dùng `rec+0`
(per-record). Chi tiết này đã được `01_battle_start_scene.md` §3.3 ghi nhận.

### 2.3 `32 01` — HÀNH ĐỘNG TRẬN (record biến thiên, **có pos nguồn + đích**) **(A)**

Handler: `case_043_007951DA_FUN_007951da.c:27-28`.
Body: `client_pseudo_c/00647164_FUN_00647164.c:75-262`.

```
P[0]    = 0x32
P[1]    = 0x01                     → RP[0]
        record i bắt đầu tại 0-based RP[1 + Σ_{j<i}(W_j + 2)]
```

- `W = u16 LE` = **độ dài phần thân record** (tính từ `RP[3]`), `RP[1..2]` (`00647164.c:93-97`).
- Text: `_LStrCopy(RP, pos+8, W-6)` (`00647164.c:171-186`) ⇒ **text BẮT ĐẦU ĐÚNG TẠI `RP[9]`**,
  tức **chồng lên `dstGrp`/`dstSlot`**. Đây là hành vi nguyên văn, không phải lỗi đọc.
- Stride record: `W + 2` (`00647164.c:187-194`).

| rec off (0-based, từ `RP[1]`) | offset `P` | Kiểu | Tên | Mức |
| :--: | :--: | :-- | :-- | :--: |
| +0..+1 | `P[2..3]` | u16 LE | **W** = len thân record | A |
| +2 | `P[4]` | u8 | **SRC GRP** (0..3) → `FUN_0065178c` | **A** |
| +3 | `P[5]` | u8 | **SRC SLOT** (0..4) → `FUN_0065178c` | **A** |
| +4..+5 | `P[6..7]` | u16 LE | **code** (skill/effect id) | A |
| +6 | `P[8]` | u8 | b2 → `FUN_00648154` arg4 | A |
| +7 | `P[9]` | u8 | b3 → `FUN_00651e3c` arg3 | A |
| **+8** | **`P[10]`** | **u8** | **DST GRP** (0..3) | **A** |
| **+9** | **`P[11]`** | **u8** | **DST SLOT** (0..4) | **A** |
| +8..+W+1 | `P[10 .. W+3]` | W-6 B | **text** (đồng thời chứa dstGrp/dstSlot ở 2 byte đầu) | A |

Xác minh từng lần đọc (mỗi lần đều có `_BoundErr` len trước đó):

| biến | dòng đọc | `RP` offset |
| :-- | :-- | :--: |
| `local_24` = W | `00647164.c:96` | `RP[1..2]` |
| `local_d` = **srcGrp** | `:108` | `RP[3]` |
| `local_e` = **srcSlot** | `:119` | `RP[4]` |
| `local_14` = code | `:127` | `RP[5..6]` |
| `local_12` = b2 | `:138` | `RP[7]` |
| `local_11` = b3 | `:149` | `RP[8]` |
| `local_f` = **dstGrp** | `:160` | `RP[9]` |
| `local_10` = **dstSlot** | `:171` | `RP[10]` |
| text (`local_2c`, len `W-6`) | `:185` | `RP[9 … W+2]` |

Stride record = `W + 2` (`:186-194`). Tra ô nguồn `FUN_0065178c(battle, local_d, local_e)` tại
`:195`; tra ô đích bằng toán tử lặp lại `cell(local_f, local_10)+0x4C` tại `:216-241`;
`FUN_00648154(self, srcUnitIdx, dstUnitIdx, b2, code)` tại `:262`.

**Ý nghĩa `code` (A):** `code == 10000 (0x2710)` bị remap theo `unit->0x04`:
`0x9090/0x9091 → 0x3E81`, `0xA4B0/0xA47B/0xA47C/0xA4BA/0xA4C8 → 0x3E82`
(`00647164.c:202-214`). Đây là skill cơ bản của phe.

**`FUN_00648154` (A, `test/ts_decompile/functions/00648154_FUN_00648154.c:23-30`):**
`if (unit[src]->0x3EA == 0) return;` — **src phải còn HP** — rồi `switch(code)` dài 12 018 byte
gọi `FUN_00666bf0(battle->FX[srcUnitIdx], <fxId>, dstUnitIdx, flag)`.
⇒ `dstUnitIdx` chính là nơi FX bắn tới; **pos đích là bắt buộc**.

### 2.4 `32 02` — record 7 byte, pos **đọc nhưng KHÔNG dùng** **(A)**

Body: `client_pseudo_c/0064dd74_FUN_0064dd74.c:55-147`. Vòng record bắt đầu 0-based `RP[1]`,
stride `n*8 + 5` với `n = RP[recOff+4]` (`0064dd74.c:132-137`).

| rec off | offset `P` | Kiểu | Tên | Mức |
| :--: | :--: | :-- | :-- | :--: |
| +0 | `P[2]` | u8 | **GRP** (tra ô, `FUN_0065178c`) | **A** |
| +1 | `P[3]` | u8 | **SLOT** | **A** |
| +2..+3 | `P[4..5]` | u16 LE | w (đọc + BoundErr, **không dùng**) | A |
| +4 | `P[6]` | u8 | **n** (điều khiển stride) | A |
| +5 | `P[7]` | u8 | b (đọc, không dùng) | A |
| +6 | `P[8]` | u8 | c (đọc, không dùng) | A |

Hiệu ứng quan sát được: `FUN_00651824(battle)` (flush 21 FX) + `self+0x1C = 1` rồi parse.
Đối chiếu asm (`0064dd74_FUN_0064dd74.asm.txt`) chỉ có `CALL 0x00651824` + `CALL 0x0065178c`
+ helper RTL ⇒ **xác nhận không mất call nghiệp vụ**. Ý nghĩa nghiệp vụ **chưa kết luận** (B).

### 2.5 `33 01` — set property theo ô (record 7 byte) **(A)**

Handler `case_044_0079522C_FUN_0079522c.c:27-28`; body `client_pseudo_c/0064cae0_FUN_0064cae0.c:55-190`.
`n = (Len(RP)-1)/7` (`:67`), record `i` tại 0-based `RP[1 + 7i]` (`:86`).
Đọc: `local_d`=GRP `:91`, `local_e`=SLOT `:102`, `local_f`=flag `:113`,
`local_14`=value `:121`; tra ô `FUN_0065178c` `:122`; `FUN_00710ab0(unit,flag,value,0)` `:187`.

| rec off | offset `P` | Kiểu | Tên | Mức |
| :--: | :--: | :-- | :-- | :--: |
| +0 | `P[2+7i]` | u8 | **GRP** (0..3) | **A** |
| +1 | `P[3+7i]` | u8 | **SLOT** (0..4) | **A** |
| +2 | `P[4+7i]` | u8 | **flag** (stat id) | **A** |
| +3..+6 | `P[5..8+7i]` | u32 LE | **value** | **A** |

Nghĩa của `flag` (đọc `FUN_00710ab0` tại `0064cae0.c:187`, body `00710ab0_FUN_00710ab0.c`):

| flag | field ghi | dòng |
| :--: | :-- | :-- |
| `0x18` | `unit+0x3E9` (u8) | `:157-163` |
| **`0x19`** | **`unit+0x3EA` = HP hiện tại (u16, clamp)** | `:164-178` |
| **`0x1A`** | **`unit+0xFB` = SP hiện tại (u16)** | `:179-200` |
| `0x1B` | `unit+0x3EE` | `:201-211` |
| `0x1C` | `unit+0xFC` | `:212-222` |
| `0x1D` | `unit+0x3F2` | `:223-233` |
| `0x1E/0x1F/0x20/0x23/0x24/0x25/0x26` | `unit+0x3F4/…` | `:234-322` |
| `0x19` + `value>0` | **đặc biệt**: nếu `FUN_007108f4(unit,0x19) < 1` ⇒ `unit+0x391 = 0` (về state 0) + `VMT+0x18(unit,0x10)` + dựng FX `FUN_0066e710` tại `FX[unitIdx]` với toạ độ `unit+0x54/+0x58` | `0064cae0.c:124-180` |

⚠️ Không guard `battle != 0` (`FUN_0065178c` deref thẳng) ⇒ `33 01` có record ngoài trận = AV.
**Đây là frame DUY NHẤT cập nhật HP/SP hiển thị của unit trong trận** — xem §5 khoảng trống #2.

### 2.6 `35 01` — 5 mảng FX buff theo ô (record 5 byte) **(A)**

Handler `case_046_007952AB_FUN_007952ab.c:27-29`; body `client_pseudo_c/0064cd74_FUN_0064cd74.c:45-140`.
`n = (Len(RP)-1)/5` (`:60`), record `i` tại 0-based `RP[1 + 5i]` (`:54`).
Đọc: `local_d`=GRP `:71`, `local_e`=SLOT `:82`, `local_19`=type `:93`, `local_18`=w `:101`.

| rec off | offset `P` | Kiểu | Tên | Mức |
| :--: | :--: | :-- | :-- | :--: |
| +0 | `P[2+5i]` | u8 | **GRP** (0..3) | **A** |
| +1 | `P[3+5i]` | u8 | **SLOT** (0..4) | **A** |
| +2 | `P[4+5i]` | u8 | **type ∈ 1..5** → chọn mảng FX | **A** |
| +3..+4 | `P[5..6+5i]` | u16 LE | **w** (skill/effect id) | **A** |

`type → mảng FX` (`0064cd74.c:112-127`): `1→FUN_0064be2c` (`battle+0x200`),
`2→FUN_0064c108` (`+0x254`), `3→FUN_0064c49c` (`+0x2A8`), `4→FUN_0064c7a8` (`+0x2FC`),
`5→FUN_0064c880` (`+0x350`). Mỗi hàm là `switch(w)` → `FUN_0066d450/…/006bac4c(arr[unitIdx], fxId)`.
⚠️ **Không guard `battle != 0`** ở `FUN_00651824` đầu hàm (`:53`) ⇒ gửi ngoài trận = AV.

**Về `w` (A):** mỗi hàm đích là `switch(w)` trên **skill id**; `w` lạ ⇒ hàm `return` im lặng
(ví dụ `FUN_0064be2c.c:23-45`). `w == 0` được xử lý như "xoá" (`0064be2c.c:26-28`:
`if ((param_3 != 0x271e) && (param_3 != 0))`). Cụ thể:
`type 4` nhận `w ∈ {0, 0x4E26=20006, 0x4E27, 0x4E28, 0x36E8}` (`0064c7a8.c:24-45`),
`type 5` nhận `w ∈ {0, 0x2B04, 0x2B08..0x2B0C, 0x2B11, 0x2B17, 0x2B18, 0x2EF9, 0x32DC, 0x36D8, …}`
(`0064c880.c:23-50`).

### 2.7 `35 03` — ép state 7 cho ô (4 byte) **(A)**

`client_pseudo_c/0064cfec_FUN_0064cfec.c:45-123`.

```
P[0]=35  P[1]=03  P[2]=GRP(0..3)  P[3]=SLOT(0..4)
```
| offset `P` | Kiểu | Tên | Mức |
| :--: | :-- | :-- | :--: |
| `P[2]` | u8 | GRP | **A** |
| `P[3]` | u8 | SLOT | **A** |

Nếu `DAT_0098C63C != 0` (guard tốt, `:69`) **và** `cell+0x48 != 0` (`:89`) thì:
`FUN_00651824(battle)` (xả 21 FX) → `unit = battle->unit[cell+0x4C]` → `FUN_00647708(unit, 7)`
(`:90-115`). State 7 → `FUN_00653F18` chép path 6 điểm từ `unit+0x54/+0x58` → animation.

### 2.8 `35 04` — đẩy effect vào TList của unit theo ô **(A)**

`client_pseudo_c/0064d128_FUN_0064d128.c:45-141`.

```
P[0]=35  P[1]=04  P[2..3]=id(u16 LE, ≠0)  P[4]=GRP  P[5]=SLOT
P[6 + 2j], P[7 + 2j] = (b1, b2)  × m ,  m = (Len(RP)-5)/2
```
| offset `P` | Kiểu | Tên | Mức |
| :--: | :-- | :-- | :--: |
| `P[2..3]` | u16 LE | id (guard `id != 0`) | A |
| **`P[4]`** | u8 | **GRP** | **A** |
| **`P[5]`** | u8 | **SLOT** | **A** |
| `P[6+2j]`,`P[7+2j]` | u8,u8 | cặp (b1,b2) | A |

Đọc: `id` `_LStrCopy(RP,2,2)` `:58`, `local_d`=GRP `:66`, `local_e`=SLOT `:73`,
vòng cặp bắt đầu 1-based `local_1c = 6` ⇒ 0-based `RP[5]` `:75`,
`m = (Len(RP)-5)/2` `:81`, gọi `FUN_00654590` `:137`.
Guard `battle != 0 && id != 0` (`:74`). **Đính chính quan trọng so với `opcode_35.md` §4.3**
(vốn ghi "`FUN_00654590` chưa có body"): nay **ĐÃ CÓ body** —
`test/ts_decompile/functions/00654590_FUN_00654590.c:23-32`:
```c
void FUN_00654590(int unit, u8 b1, u8 b2, u16 id) {
  puVar1 = _GetMem(4); *puVar1 = b1; puVar1[1] = b2; *(u16*)(puVar1+2) = id;
  TList_Add(*(int **)(unit + 0x5b0), puVar1);      // :31
}
```
⇒ **KHÔNG phải damage** — nó append bản ghi 4 byte `[b1][b2][id:u16LE]` vào
`unit->0x5B0` (TList hàng đợi effect của unit). *(A)*

### 2.9 `35 05` — cờ `unit+0x552` (4 byte) **(A)**

`client_pseudo_c/0064dfa4_FUN_0064dfa4.c:30-68`. `P[2]`=GRP, `P[3]`=SLOT.
Guard `battle != 0` (`:57`), ô phải có unit (`:58-63`), ghi `unit+0x552 = 1` (`:64`).

### 2.10 `35 07` — **DI CHUYỂN Ô** (6 byte) **(A)**

`client_pseudo_c/0064e3e0_FUN_0064e3e0.c:16-420` (chi tiết §1.7).

```
P[0]=35  P[1]=07
P[2] = SRC GRP (0..3)
P[3] = SRC SLOT (0..4)
P[4] = DST GRP (0..3)
P[5] = DST SLOT (0..4)
```
Đây là **frame mang pos nhiều thông tin nhất** và **frame duy nhất thay đổi `cell+0x48/+0x4C`
sau khi trận bắt đầu**. ⚠️ Không guard null.

### 2.11 `35 0C` — cờ `unit+0x5BD` (4 byte) **(A)**

`client_pseudo_c/0064faa8_FUN_0064faa8.c:50-107`. `P[2]`=GRP, `P[3]`=SLOT.
Guard `battle != 0` (`:62`) + `cell+0x48 != 0` (`:82`), ghi `unit+0x5BD = 1` (`:105`).

### 2.12 `35 0E` — `unit.VMT+0x1C(unit, w, 1)` (6 byte) **(A)**

`client_pseudo_c/0064fbd8_FUN_0064fbd8.c:55-118`. `P[2]`=GRP (`local_d` `:58`),
`P[3]`=SLOT (`local_e` `:65`), `P[4..5]`=w (u16 LE, `_LStrCopy(RP,4,2)` `:66` → `:67`).
Guard `battle != 0` (`:69`).

### 2.13 `35 0F` — 6 cặp (byte, Word) theo ô (8 byte) **(A)**

`client_pseudo_c/0064fd34_FUN_0064fd34.c:50-175`.
Đọc: `local_d`=GRP `:57`, `local_e`=SLOT `:64`, `local_f`=mode `:71`,
`local_10`=byte `:78`, `local_12`=Word `:81`.
`switch(mode)` 6 case `'B'..'G'` (`:85-169`) ghi vào `unit+0x5BE/0x5BF`, `0x5C1/0x5C2`,
`0x5C4/0x5C5`, `0x5C7/0x5C8`, `0x5CA/0x5CB`, `0x5CD/0x5CE` và gọi
`FUN_0058f5e0(*gvar_007DA530, unitIdx, mode, w, byte)` (`:98,112,126,…`).
⚠️ Không guard null (`FUN_0065178c` deref thẳng) ⇒ gửi ngoài trận = AV.

### 2.14 `0B 01` — dọn 1 ô (3 hoặc 4 byte) **(A)**

`client_pseudo_c/0064bb24_FUN_0064bb24.c:40-190`. `P[2]`=GRP, `P[3]`=SLOT.
Nếu `cell+0x48 != 0` và `unit+0x5B4 == 0`: dội 5 mảng FX với id `0x0B` (`:113-133`), rồi
hoặc `FUN_006540b0(unit)` (nếu `state ∈ {7,8}`), hoặc `FUN_00666bf0(FX[unitIdx], 6, unitIdx, 0)`.
**Không** sửa `cell` hay `unit+0x56E/0x56F` ⇒ đây **không phải** frame di chuyển.

---

## §3. PET trong trận

### 3.1 Pet vào lưới = `0B 05` / `0B FA` với `type == 4` **(A)**

`type` (rec+0 của 23B / rec+1 của 24B) được ghi thẳng vào `unit+0x79`:
- `FUN_00646a34.c:107` — `*(char *)((int)local_20 + 0x79) = (char)param_9;`
- `FUN_00659358.c:52` — `*(undefined1 *)((int)local_24 + 0x79) = 2;` (NPC)

Bảng phân loại `type` (từ `switch` ở `006438ac.c:184` và `00644294.c:186`) **(A)**:

| `type` | `unit+0x79` | Đường tạo unit | Nguồn dữ liệu | Ghi chú |
| :--: | :--: | :-- | :-- | :-- |
| `2` | 2 | `FUN_006469b8` (nếu id==self) hoặc `FUN_00659358`/`FUN_006469b8` | registry `gvar_007DA300[idx]` | **PLAYER** |
| `3` | 2 | `FUN_006469b8` | `gvar_007DA6DC[w]` (w = `idNpcOnMap`, biên ≤100) | **NPC trên map** |
| **`4`** | **4** | **`FUN_00646a34`** | **`FUN_0071d1bc(owner, id)` → `owner->0x57C[petIdx*4]`** | **PET** |
| `6,0x0B,0x0C,0x0D,0x0F` | id | `FUN_00646a34` | `FUN_0070c20c(registry, id)` | NPC động quái |
| `7` | 7 | `FUN_00646a34` | id | TeamDef |
| `9` | id | `FUN_006469b8` | id | SOLDIER/auto |
| `0x0E,0x10,0x12` | id | `FUN_00646a34` | id | auto/chaos |

### 3.2 Pet được đặt vào ô như thế nào **(A)**

Với `type ∈ {4,6,0x0B,0x0C,0x0D,0x0F}`, `FUN_00646a34` gọi
`FUN_00646028(battle->unit[idx], tmpTHuman, grp, &kindOut, blob10, isPlayerSide, idChar, slot)`
→ `unit+0x56E = grp`, `unit+0x56F = slot`, `FUN_00646664(...)` chiếm ô. **Cùng đường với
player/NPC — không có frame pos riêng cho pet.** *(A)*

`FUN_0071d1bc(owner, templateCode)` (`test/ts_decompile/functions/0071d1bc_FUN_0071d1bc.c:118-145`):
quét `owner->0x57C[1..4]`, trả `i` nếu `pet->0x78 == 1` **và** `pet->0x04 == templateCode`,
trả `0` nếu không thấy. ⇒

> ⚠️ **Điều kiện tiên quyết (A):** với `type == 4`, byte `id` (rec+1..4 của 23B /
> rec+2..5 của 24B) phải là **template code của pet** (giá trị `pet->0x04`), **KHÔNG phải
> pet instance id**. Client tra bảng `owner->0x57C[]` để lấy **object pet** rồi copy:
> tên (17 byte @ `+9`), HP (`u16 @ +4`), byte `@ +0x10`, dword `@ +0x14`, cộng copy 0x17 byte
> từ `pet+0x56C` → `unit+0x56C` (`00646a34.c:113-190` và `:200-…`).
> Nếu `owner->0x57C[]` chưa được nạp (chưa nhận `0x0F`), `petIdx = 0` ⇒ client đọc
> `owner->0x57C[0]` — **rất có thể null ⇒ EAccessViolation hoặc pet rỗng**.

Bảng `owner->0x57C + slot*4` là **mảng 5 slot thú nuôi/đồng hành**, được đồng bộ bởi
**Main OP `0x0F`** (nhiều SubOp; xem `.scratch/client-pseudo-op-code/opcode_0f.md` §0, dòng 14 và
§4 tóm tắt quy ước "ghi vào sub-object slot `*(PlayerRec+0x57C + slot*4)`, slot guard ≤4").
⇒ **Thứ tự bắt buộc: `0x0F` (nạp pet) PHẢI tới TRƯỚC `0B 05` / `0B FA`.**

### 3.3 Pet dùng chung cơ chế pos với unit khác **(A)**

- Tra ô: `FUN_0065178c` không phân biệt loại — chỉ dựa `cell+0x08`/`cell+0x0C` (A).
- `35 07` (di chuyển) cũng vậy (A).
- Render: `FUN_0065130c` vẽ mọi unit bằng `unit+0x54/+0x58` (A).
- `FUN_0058f5e0` (`35 0F`) có nhánh đặc biệt `unit->0x79 == 4` ⇒ đổi cờ nội bộ
  (`test/ts_decompile/functions/0058f5e0_FUN_0058f5e0.asm.txt:28-32`) — xác nhận pet là
  `unit+0x79 == 4` **(A)**.
- `FUN_00745e0`/`:88f94`-class đọc `unit->0xB + k*4` (mảng 7 con trỏ "trẻ" tại `unit+0x2C`),
  được `FUN_00646a34` nạp từ pet → unit có quan hệ cha–con cho effect lan **(A, mức cấu trúc)**.

### 3.4 Pet KHÔNG có frame pos riêng — kết luận **(A)**

Grep `client_pseudo_c/` + `test/ts_decompile/functions/` cho mọi hàm chạm
`DAT_0098C63C` (49 file) cho thấy **không tồn tại** frame nào chuyên biệt cho pet: pet chỉ
đi qua `0B 05`/`0B FA` (`type=4`) rồi dùng chung `32 01`, `33 01`, `35 xx` với mọi unit khác.
*(A)*

---

## §4. Đối chiếu server Rust

### 4.1 Bảng: server gửi gì / client đọc gì / khớp hay lệch

| # | Server Rust (`src/battle/**`) | Byte thực gửi | Client đọc (`P` offset) | Kết luận |
| :--: | :-- | :-- | :-- | :-- |
| 1 | `packets::battle_open_leader` `packets.rs:9-15` = `0B FA` + le16(diahinh) + `03` + pkt23 | `P[2..3]`=diahinh, `P[4]`=kindOut, `P[5]`=pkt | `0B FA`: `RP[1..2]`→Create, `RP[3]`=kindOut, record tại `RP[3]` ⇒ `RP[4..]` = pkt23 | ✅ **KHỚP BYTE**; len `0x1C`=28 = 2+2+1+23 ✔ |
| 2 | `Battle::member_battle_frame` `construction.rs:443-487` = `0B FA` + le16 + `05`/`03`/`64` + pkt23×k | record 24B | `rec+0`=marker, `rec+1..23`=pkt23 | ✅ **KHỚP** (`text.len()/24` = số record, `n=(Len(RP)-3)/24` ✔) |
| 3 | `WarInfo::packet_hex` `engine.rs:71-88` | rec+1..23 của 24B (hoặc rec+0..22 của 23B): `typ(1) id(4) idNpcOnMap(2) idChar(4) **row** **col** hpMax(2) spMax(2) hp(2) sp(2) lv(1) element(1)` | `grp = rec+11/12`, `slot = rec+12/13`; blob10 = `hpMax..element` | ✅ **KHỚP BYTE 23/23** (đã xác minh qua `.asm.txt:315-332`) |
| 4 | `packet_hex` ghi `self.row` rồi `self.col` (`engine.rs:77`) | `row` trước `col` | client đọc `GRP` trước `SLOT` | ✅ thứ tự đúng |
| 5 | `Battle::new` `construction.rs:60-71` lặp `row 0..4`, `col 0..5` | 20 ô | `grp ≤ 3`, `slot ≤ 4` | ✅ **KHỚP** (4×5) |
| 6 | `add_player` đặt `row=3` (`construction.rs:92,568,585`) | `grp=3` | client `grp 2,3` = phe 1 (`FUN_00646634`) | ✅ khớp phe |
| 7 | `add_npc` đặt `row=0|1` (`construction.rs:596-611`) | `grp=0,1` | phe 2 | ✅ khớp |
| 8 | `add_pet` đặt `row^1` (`construction.rs:159-160`) | player row3 ⇒ pet row2 | cùng phe | ✅ khớp |
| 9 | `add_player`/`add_pet` `team = if row == 0 {2} else {1}` (`construction.rs:93,125,140`) | — | client: `team = 2` nếu `grp<2`, `1` nếu `grp≥2` | ⚠️ **LỆCH LOGIC (C)**: đúng phải là `row < 2`. Hiện chưa lộ vì player luôn row 3 và `add_npc` hard-code `team=2` (`construction.rs:203`), nhưng là **latent bug** |
| 10 | `entity_npc` = `0B 05 03` + pkt23 (`packets.rs:40-42`) | `RP[0]=05`, `RP[1]=03`, `RP[2..24]`=record 23B | `n=(Len(RP)-2)/23` = `(26-2)/23` = 1 ✔; record tại `RP[2]` ✔ | ✅ **KHỚP BYTE**, len `0x1A`=26 ✔ |
| 11 | `entity_player` = `0B 05 05` + pkt23 (`packets.rs:45-47`) | `RP[1]=05` | `FUN_00646028.c:207-209`: `if (*param_4 == 5) { unit+0x569 = 1; banner }` | ✅ **khớp** (marker 5 có tác dụng thật) |
| 12 | `WarInfo.typ` = 2 (player) / 3 (NPC) / 4 (pet) | `rec+0` | `unit+0x79` = 2 / 2 / 4 | ✅ **KHỚP** — `type 4` chính là PET, `type 3` là NPC-on-map, `type 2` là player |
| 13 | `add_pet`: `cell.id = pet.id`, `cell.id_npc_on_map = pet.stt` (`construction.rs:164-165`) | `id` = pet instance id | với `type 4`, client dùng `id` làm **template code** tra `owner->0x57C[]` (`FUN_0071d1bc`) | ⚠️ **NGHI VẤN (B/C)**: phải xác nhận `pet.id` trong DB có bằng `pet->0x04` (template) không. Nếu `pet.id` là **instance** id thì pet sẽ không dựng được |
| 14 | `add_pet`: `cell.id_char = owner_id` (`construction.rs:166`) | `idChar` | `FUN_00646028.c:130-146` dùng `param_7`(=idChar) tra owner qua `FUN_0070c20c` | ✅ khớp |
| 15 | `acting(r,c)` = `35 05` + row + col (`packets.rs:90-92`) | `P[2..3]` | `35 05`: `P[2]`=GRP, `P[3]`=SLOT, ghi `unit+0x552=1` | ✅ **KHỚP BYTE** |
| 16 | `troi_start(r,c,skill)` = `35 01` + r + c + `01` + le16 (`packets.rs:118-126`) | `P[4]`=type=1, `P[5..6]`=w | `35 01`: `type∈1..5` chọn mảng FX; `type 1 → battle+0x200` | ✅ **KHỚP BYTE**; ⚠️ **tên hàm sai**: không phải "buff start" mà là "FX icon loại 1". Ngoài ra `w` phải là skill id mà `FUN_0064be2c` có trong `switch`, nếu không sẽ `return` im lặng (`0064be2c.c:23-45`) |
| 17 | `troi_end(r,c,byte)` = `35 01` + r + c + byte + `0000` (`packets.rs:110-115`) | `type = 1..5`, `w = 0` | `FUN_0064be2c`: `param_3 == 0` được xử lý (clear) | ✅ khớp |
| 18 | `drop_item` = `35 04` + le16(item) + npcRow + npcCol + row + col (`packets.rs:128-135`) | `P[2..3]`=id, `P[4]`=npcRow, `P[5]`=npcCol, cặp `(row,col)` | `35 04`: `P[4]`=GRP, `P[5]`=SLOT, cặp byte từ `P[6]` | ✅ **KHỚP BYTE** |
| 19 | `combo`/`combo_footer_20007` = `32 01` + le16(W) + r + c + le16(code) + `01 01` + trRow + trCol + skilling (`packets.rs:103-107,243-252`) | `P[2..3]`=W, `P[4]`=r, `P[5]`=c, `P[6..7]`=code, `P[8]`=01, `P[9]`=01, `P[10]`=trRow, `P[11]`=trCol | `32 01`: `P[4]`=srcGrp, `P[5]`=srcSlot, `P[6..7]`=code, `P[8]`=b2, `P[9]`=b3, `P[10]`=dstGrp, `P[11]`=dstSlot, text = `P[10..10+W-6-1]` | ✅ **KHỚP BYTE 100%** — W = 6 + len(skilling), dst = target (row,col) (`runner.rs:1352-1361` dùng `target.row/col`) |
| 20 | `runner.rs:1059-1063` bọc `3201` + text9 (đã gồm `le16(blockLen)`) | `P[2..3]` = le16 blockLen | `RP[1..2]` = W | ✅ khớp |
| 21 | `turn_action(text9)` (`packets.rs:95-99`) = `32 01` + text9 **không W** | thiếu W | `32 01` cần W | ⚠️ **API nguy hiểm** — hàm này **không dùng** ở `runner.rs` (chỉ `runner.rs:1059` tự dựng). Nên xoá/sửa để tránh gọi nhầm |
| 22 | `reposition(r,c)` = `0B 01` + r + c + `00` (`packets.rs:75-77`) | `P[2..3]` | `0B 01`: `P[2]`=GRP, `P[3]`=SLOT | ⚠️ **TÊN SAI** — không hề di chuyển, chỉ dội FX/reset unit tại ô |
| 23 | `clear_pet_cell(r,c)` = `0B 01` + (r^1) + c (`packets.rs:80-82`) | như trên | như trên | ⚠️ **TÊN SAI** — không xoá pet, chỉ reset ô đó |
| 24 | `your_turn()` = `34 01` (`packets.rs:85-87`) | `P[1]=01` | `case_045.c:27-29` cần `gvar_007DA51C != 0` | ⚠️ **có thể no-op**: `gvar_007DA51C` **không có lệnh gán nào trong toàn bộ 13 176 hàm export** (grep `da51c` chỉ ra phép đọc). Nếu thật sự null thì `34 01` không làm gì |
| 25 | `show_on_map(id,tail)` = `0B 04 02` + le32(id) + tail3 (`packets.rs:50-67`) | record 8B: `[type=2][id:4][idx:2][val:1]` | `0B 04`: `FUN_00642244.c:63` (`n=(Len(RP)-1)/8`), record 8B `[type][DWORD id][WORD idx][BYTE val]` đọc tại `:73-110` | ✅ **KHỚP BYTE** (payload 10B ⇒ `n=(9-1)/8`=1) |
| 26 | `hide_from_map(id)` = `0B 00` + le32(id) + `0000` (`packets.rs:70-77`) | `RP[1..4]`=id, `RP[5..6]`=idx | `0B 00`: `_LStrCopy(RP,2,4)` (`00641fa4.c:55`) + `_LStrCopy(RP,6,2)` (`:57`) | ✅ **KHỚP BYTE** |
| 27 | `battle_trailer()` = `0B 0A 01` (`packets.rs:151-153`) | `P[1]=0A`, `P[2]=01` | `0B 0A`: `FUN_0064e978.c:44` ghi `battle->0x04 = RP[1]`; `02/04/07/08/09` → `battle->0xE5A = 30`, `0x0E` → `10` | ✅ khớp |
| 28 | `battle_exit_move()` = `05 04` (`packets.rs:156-158`) | — | `0x05` không mang pos | ✅ |
| 29 | `handle_skill_command` đọc `payload[0..1]`=src, `[2..3]`=dst, `[4..5]`=u16 `skill_id` (`server/handlers/battle.rs:192-197`) | — | **ASM C→S `0x32 01`** (`test/ts_decompile/functions/0077f414_FUN_0077f414.asm.txt:3284-3418`): `srcGrp(0x56E)`, `srcSlot(0x56F)`, `dstGrp(0x56E của unit[0xEAA])`, `dstSlot(0x56F)`, **`le16(unit->0x5A8)`**, `Random`, `Random(256)` | ✅ **6 byte đầu khớp tuyệt đối**; ⚠️ nhưng byte `u16` là **`unit->0x5A8` = skill id đang chọn** (init `0x2710`=10000), KHÔNG phải "skill id tùy ý" — client chỉ gửi skill nó đã chọn. 2 byte random cuối bị Rust bỏ qua (đúng) |
| 30 | `handle_use_item` đọc cùng layout + `item_id` u16 (`battle.rs:224-227`) | — | `0x32 02` (asm `:3419-3552`) dùng `+0x5AA` thay `+0x5A8` | ⚠️ field `unit+0x5AA` chưa được định danh |
| 31 | `BattleRoleData` / `BattleRoleSerializer` (`src/protocol/codecs/battle_role.rs`) | — | **không frame nào trong client đọc layout 42 byte này** | ❌ **DEAD CODE SAI** — đã xác nhận ở `01_battle_start_scene.md` §12.2; đặc biệt thứ tự `col` rồi `row` (`:119-120`) là **ngược** với client (`grp` rồi `slot`) |
| 32 | **KHÔNG có** builder cho `35 07` | — | `35 07` = frame di chuyển duy nhất | ❌ **THIẾU** (xem §5 #1) |
| 33 | **KHÔNG có** builder cho `33 01` | — | `33 01` flag `0x19`/`0x1A` = cập nhật HP/SP unit trận | ❌ **THIẾU** (xem §5 #2) |
| 34 | `status_update` = `08 01` + … (`packets.rs:140-148`) | MainOp `0x08` | `0x08` không phải frame trận (nó đẩy stat ngoài bản đồ) | ⚠️ không thay thế được `33 01` |

### 4.2 Tóm tắt mức độ khớp

- **Pos (grp, slot) trên wire: KHỚP 100%** — 12/12 frame có pos mà Rust gửi đều đúng offset,
  đúng thứ tự byte, đúng endianness, đúng độ dài frame. *(A)*
- **Kích thước lưới 4×5: KHỚP.** *(A)*
- **Mapping phe (row 0,1 = địch / row 2,3 = ta): KHỚP.** *(B)* — suy từ `FUN_00646634`
  + hướng sprite `unit+0xE3`.
- **3 lỗi/thiếu sót thật sự:**
  1. `team` tính bằng `row == 0` thay vì `row < 2` (`construction.rs:93,125,140`) — latent bug.
  2. Không có `35 07` ⇒ **ô trận phía client không bao giờ được cập nhật sau khi mở trận**.
  3. Không có `33 01` ⇒ **HP/SP hiển thị của unit trong trận không bao giờ được cập nhật**.

---

## §5. Khoảng trống

| # | Khoảng trống | Mức | Ảnh hưởng |
| :--: | :-- | :--: | :-- |
| **1** | **Không có frame `35 07`** (di chuyển/đổi ô). Client giữ nguyên `cell+0x48/0x4C` và `unit+0x56E/0x56F` từ lúc mở trận. Rust dùng `0B 01` (`reposition`) thay thế — nhưng `0B 01` **không** sửa ô (chỉ dội FX/reset state). | **A** | Unit chết vẫn "chiếm" ô ⇒ `FUN_0065178c` vẫn trả unitIdx ⇒ `32 01`, `33 01`, `35 xx` vẫn tìm thấy unit đã chết. `FUN_00648154` có guard `unit->0x3EA == 0 → return` nên FX không bắn, nhưng **`0B 01` không có guard đó**. Cần thêm builder `move_cell(src_r,src_c,dst_r,dst_c)` = `35 07` |
| **2** | **Không có frame `33 01`** ⇒ `unit+0x3EA` (HP) / `unit+0xFB` (SP) của unit trong trận không bao giờ cập nhật. | **A** | Thanh HP trên sprite + điều kiện `FUN_00648154` (`unit->0x3EA==0 ⇒ return`) sai ⇒ FX đánh vẫn bắn vào unit đã chết; `FUN_00645ebc` cũng dựa vào state khác. Cần `33 01` + record 7B `[grp][slot][0x19][hp:u32]` |
| **3** | **`TFightField.Create` @ `0x0063FADC` không có body.** Nằm trong khoảng trống `0x0063F9F3 … 0x00640984` (`index.csv`: `FUN_0063f2f0` dài 1795 B kết thúc `0x0063F9F3`, kế tiếp là `FUN_00640984`). | — | Ta **biết** client tự tính/gán `cell.X/cell.Y` ở đó nhưng **không biết công thức pixel**. Không chặn việc mock server (server không cần gửi pixel), nhưng chặn việc **dựng lại lưới bằng tọa độ** nếu muốn test ngoài client thật |
| **4** | **Mảng `battle->unit[21]` không có lệnh gán nào trong export** (grep không tìm thấy `MOV [… + 0x158], …` cho `DAT_0098C63C`). Cũng nằm trong constructor `#3`. | — | Ta biết client cấp phát trước (vì `FUN_00644164` chỉ **tìm** slot trống và `FUN_00646028` **dùng** `battle->unit[idx]` sẵn có), nhưng không thấy chỗ tạo |
| **5** | **`gvar_007DA51C` không có lệnh gán nào** trong toàn bộ 13 176 hàm (`grep -rn "da51c"` chỉ ra phép đọc `MOV EAX,[0x007da51c]; MOV EAX,[EAX]`). Nó là **con trỏ kép** (double-indirect) giống `gvar_007D9CE4`. | — | Nếu thật sự null ⇒ `0x34 01` (`your_turn`) là **no-op** (`case_045.c:27`), và builder C→S `0x32` (`0077f414.asm.txt:3296-3417`) đọc `unit->0x56E/0x56F/0x5A8` từ con trỏ khác ⇒ **client có thể không gửi được lệnh hành động**. Đây là giả thuyết mạnh cần kiểm live |
| **6** | **`gvar_0098C63C` (viết) vs `DAT_0098C63C` (đọc)** — cùng địa chỉ `0x0098C63C`; không có export nào ghi trừ `006438ac.asm.txt:57`. Nghĩa là **chỉ có `0B FA` tạo được trận**; `0B 05` ngoài trận chỉ hiện banner lỗi | A | Server **bắt buộc** phải gửi `0B FA` trước mọi frame lưới khác |
| **7** | Chưa xác minh: `id` của pet (rec+1..4) là **template** hay **instance**. | B | Nếu là instance id thì `FUN_0071d1bc` không tìm thấy pet ⇒ pet không dựng (hoặc crash). Cần đối chiếu `pet_box.rs` xem `PetState.id` là gì |
| **8** | Chưa có body cho: `FUN_00654590` — **đã có** (đính chính `opcode_35.md`); còn `FUN_0066bf0`, `FUN_0066d450/804/e37c`, `FUN_006bac4c` (FX ids), `FUN_006540b0`, `FUN_00647708` (state machine) — có body nhưng chưa khảo sát ngữ nghĩa từng `skillId` trong `FUN_0064be2c/c108/c49c/c7a8/c880` | — | Không ảnh hưởng pos; ảnh hưởng **render buff/FX** |
| **9** | Chưa xác minh: nghĩa chính xác `cell+0x40/+0x44` (X/Y) là **screen pixel** hay **world coordinate**. Bằng chứng nghiêng về screen pixel: `FUN_0065130c.c:110-115` dùng `unit->0x54 - w/2 + 2` làm toạ độ vẽ thẳng lên canvas ⇒ **screen pixel**. | B | Không chặn mock |
| **10** | Chưa rõ `unit+0x5AA` (C→S `0x32` SubOp 2 dùng thay `0x5A8`) và `battle->0x16F`/`0xE1` | — | Chỉ ảnh hưởng item-in-battle |

---

## §6. Ghi chú cho mock server (tổng hợp nhanh)

```
── MỞ TRẬN (bắt buộc đầu tiên, chỉ 1 lần) ──────────────────────────────
[0B][FA][diahinh lo][diahinh hi] ( [kindOut][rec23] × N )
   rec23 = [type][id:4][idNpcOnMap:2][idChar:4][GRP][SLOT]
           [hpMax:2][spMax:2][hp:2][sp:2][lv][element]
   GRP ∈ 0..3 (2,3 = phe ta) ; SLOT ∈ 0..4 ; type: 2=player 3=npc 4=pet 7=teamdef

── NẠP THÊM ENTITY (nhiều lần được, nhưng tổng ≤ ~20 lần/trận) ──────────
[0B][05][kindOut][rec23]

── ĐẶT / ĐỔI Ô ───────────────────────────────────────────────────────
[35][07][srcGrp][srcSlot][dstGrp][dstSlot]      ← DI CHUYỂN (chưa có builder!)
[0B][01][grp][slot][00]                         ← KHÔNG di chuyển, chỉ reset FX

── HÀNH ĐỘNG ──────────────────────────────────────────────────────────
[32][01][W:2][srcGrp][srcSlot][code:2][b2][b3][dstGrp][dstSlot][text…]
   W = 6 + len(text) ; text BẮT ĐẦU bằng dstGrp,dstSlot (chồng 2 byte đó)

── CẬP NHẬT HP/SP (chưa có builder!) ─────────────────────────────────
[33][01]([grp][slot][flag][value:4])×n     flag 0x19 = HP, 0x1A = SP

── ĐÁNH DẤU / BUFF FX ────────────────────────────────────────────────
[35][05][grp][slot]                              → unit+0x552 = 1
[35][01]([grp][slot][type 1..5][w:2])×n          → FX icon (mảng theo type)
[35][0C][grp][slot]                              → unit+0x5BD = 1
[35][0E][grp][slot][w:2]                         → unit.VMT+0x1C
[35][0F][grp][slot][mode 'B'..'G'][byte][w:2]     → 6 cặp unit+0x5BE..0x5CE
[35][04][id:2][grp][slot]([b1][b2])×m            → TList unit+0x5B0
[35][03][grp][slot]                              → ép state 7 + xả 21 FX

── MAP / UI ───────────────────────────────────────────────────────────
[0B][00][id:4][idx:2]            ẩn actor        (idx thường 0)
[0B][04][02][id:4][idx:2][val]   set actor+0x376
[0B][0A][01]                     mode trận + timer
[34][01]                         bắt đầu lượt  ⚠ cần gvar_007DA51C ≠ 0

⛔ ĐỪNG gửi 35 07 / 33 01 / 32 01 / 32 02 / 35 01 ngoài trận → EAccessViolation
⛔ ĐỪNG gửi 0B 05 trước 0B FA → banner "Error: FightManage.SupportRoleInfo", mất entity
⛔ ĐỪNG gửi type 4 (pet) trước khi 0x0F đã nạp owner->0x57C[] → pet dựng lỗi/crash
⛔ ĐỪNG để > ~20 lần 0B 05/0B FA trong một trận → client bắt đầu bỏ qua đơn vị mới
```

---

## §7. Source trail

| # | Nguồn | Dùng để |
| :--: | :-- | :-- |
| 1 | `test/ts_decompile/functions/0065178c_FUN_0065178c.c:38-91` (+ `.asm.txt`) | Công thức ô `(grp,slot)`, guard 4/3, trả `0xFFFFFFFF` |
| 2 | `test/ts_decompile/functions/00651824_FUN_00651824.c:21-37` | Flush 21 FX tại `battle+0x1AC` |
| 3 | `client_pseudo_c/0064cfec_FUN_0064cfec.c:45-123` | `35 03` (4 byte) + số học lưới |
| 4 | `client_pseudo_c/0064e3e0_FUN_0064e3e0.c:16-420` | `35 07` — di chuyển ô, `unit+0x56E/0x56F/0x54/0x58/0x1C/0x20/0x4C/0x50` |
| 5 | `test/ts_decompile/functions/00646664_FUN_00646664.c:23-120` | Chiếm ô: `cell+0x48/0x4C/0x4D` |
| 6 | `test/ts_decompile/functions/00646028_FUN_00646028.c:78-282` | `unit+0x56E/0x56F`, `unit+0x163` (phe), `unit+0xE3` (hướng), copy X/Y |
| 7 | `test/ts_decompile/functions/00646028_FUN_00646028.asm.txt:11-13, 313-332` | **Quyết định**: blob 10 byte → `unit+0x404/0x406/0x3EA/0x3EC/0x3FA/0x3E9` |
| 8 | `test/ts_decompile/functions/00646028_FUN_00646028.asm.txt:32-40` (`0064be2c.c`) | 5 mảng FX theo `type` của `35 01` |
| 9 | `test/ts_decompile/functions/00644164_FUN_00644164.c:22-40`, `00645ebc_FUN_00645ebc.c:24-95`, `006441bc_FUN_006441bc.c:22-58` | Cấp phát / tra unitIdx; giới hạn ~20 lần/trận |
| 10 | `test/ts_decompile/functions/00646634_FUN_00646634.c:23-38` | `grp → phe` (2 nếu `<2`, 1 nếu `≥2`) |
| 11 | `test/ts_decompile/functions/006508ec_FUN_006508ec.c:51-105` | Đọc toạ độ pixel từ `unit+0x56E/0x56F` |
| 12 | `test/ts_decompile/functions/006547a8_FUN_006547a8.c:22-62` | Quét 21 entry `battle+0x158` + `+0x1AC` |
| 13 | `test/ts_decompile/functions/006438ac_FUN_006438ac.c:87-420` + `.asm.txt:55-57` | `0B FA`: `TFightField.Create` + `DAT_0098C63C` + record 24B |
| 14 | `client_pseudo_c/case_011_0078D5D1_FUN_0078d5d1.c:40-120` | Dispatch `0B`: sub `0,1,3,4,5,6,7,9,0A,0B,0C,0xFA`; **không có sub 2** |
| 15 | `client_pseudo_c/00644294_FUN_00644294.c:87-300` + `00644294_FUN_00644294.asm.txt:38-44` | `0B 05`: record 23B, `RP[1]` đọc 1 lần |
| 16 | `client_pseudo_c/00647164_FUN_00647164.c:75-262` | `32 01`: record biến thiên + text chồng dstGrp/dstSlot |
| 17 | `client_pseudo_c/0064dd74_FUN_0064dd74.c:55-147` + `.asm.txt` | `32 02`: record 7B, pos đọc không dùng |
| 18 | `client_pseudo_c/0064cae0_FUN_0064cae0.c:55-190` | `33 01`: record 7B + nhánh flag `0x19` |
| 19 | `client_pseudo_c/00710ab0_FUN_00710ab0.c:111-330` | Ngữ nghĩa `flag` của `33 01` (HP `0x19`, SP `0x1A`, …) |
| 20 | `client_pseudo_c/0064cd74_FUN_0064cd74.c:45-140` | `35 01`: record 5B + switch 5 method |
| 21 | `client_pseudo_c/0064d128_FUN_0064d128.c:45-141` | `35 04`: record id+grp+slot+cặp byte |
| 22 | `test/ts_decompile/functions/00654590_FUN_00654590.c:23-32` | **Đính chính**: `35 04` append `[b1][b2][id:2]` vào `unit+0x5B0`, KHÔNG phải damage |
| 23 | `client_pseudo_c/0064dfa4_FUN_0064dfa4.c:30-68` | `35 05`: `unit+0x552` |
| 24 | `client_pseudo_c/0064faa8_FUN_0064faa8.c:50-107` | `35 0C`: `unit+0x5BD` |
| 25 | `client_pseudo_c/0064fbd8_FUN_0064fbd8.c:55-118` | `35 0E`: `VMT+0x1C` |
| 26 | `client_pseudo_c/0064fd34_FUN_0064fd34.c:50-175` | `35 0F`: 6 mode `'B'..'G'` |
| 27 | `client_pseudo_c/0064bb24_FUN_0064bb24.c:40-190` | `0B 01`: dội 5 mảng FX + reset state (KHÔNG di chuyển) |
| 28 | `test/ts_decompile/functions/00646a34_FUN_00646a34.c:107, 113-320` | `type → unit+0x79`; pet copy từ `owner->0x57C[]` |
| 29 | `test/ts_decompile/functions/0071d1bc_FUN_0071d1bc.c:118-145` | Tra pet theo **template code** trong `owner->0x57C[1..4]` |
| 30 | `test/ts_decompile/functions/00659358_FUN_00659358.c:34-60` | `type 2` cho NPC (`unit+0x79 = 2`) |
| 31 | `test/ts_decompile/functions/0065130c_FUN_0065130c.c:39-130` | Render tại `unit+0x54/0x58` (screen pixel) |
| 32 | `test/ts_decompile/functions/00648154_FUN_00648154.c:23-30` + `.asm.txt:535-538` | Guard `unit->0x3EA != 0`; `DAT_0098C63C` là con trỏ thật |
| 33 | `test/ts_decompile/functions/00651e3c_FUN_00651e3c.c:24-260` | **Đính chính**: `FUN_00651e3c` ĐÃ có body — parse text thành record 6 byte, `TList.Add` |
| 34 | `test/ts_decompile/functions/0077f414_FUN_0077f414.asm.txt:3278-3552` | **ASM C→S `0x32`**: `[grp][slot][dstGrp][dstSlot][le16(unit+0x5A8)]` + 2 random |
| 35 | `test/ts_decompile/functions/0063cda0_TFightHum.Create.asm.txt:48`, `00658c30_FUN_00658c30.asm.txt:91,127,163,241` | `unit+0x5A8` = skill id (init/reset `0x2710`, flee `0x4269`) |
| 36 | `test/ts_decompile/functions/00641fa4_FUN_00641fa4.c:27-160`, `00642244_FUN_00642244.c:30-160`, `0064e978_FUN_0064e978.c:23-50` | `0B 00` / `0B 04` / `0B 0A` (không pos) |
| 37 | `test/ts_decompile/functions/00647ad0`, `00647dfc`, `006508ec`, `006455d8` | Các hàm khác đọc `cell+0x40/+0x44` (AI/path) |
| 38 | `src/battle/packets.rs`, `src/battle/construction.rs`, `src/battle/engine.rs:71-88`, `src/server/handlers/battle.rs:165-247` | Đối chiếu phía server Rust |
| 39 | `.scratch/client-pseudo-op-code/opcode_32.md`, `opcode_33.md`, `opcode_35.md`, `opcode_0b.md`, `opcode_0f.md`, `opcode_34.md` | Nền tài liệu (có 2 chỗ **đính chính** — xem #22, #33) |
| 40 | `test/ts_decompile/index.csv:4222-4224` | Chứng minh `0x0063FADC` nằm trong khoảng trống giữa `FUN_0063f2f0` và `FUN_00640984` |
