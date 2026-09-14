# PHÂN TÍCH — Main OP 0x35 (Case 46) — `FUN_007952ab` @ `0x007952AB` — **Biến cố TRẬN ĐÁNH / Battle Events (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x35] = 0x2E (=46)` → `jumptable_dword200_0x78A9B6[46]` (entry `0x0078AA6E`) → target `0x007952AB` = **Case 46**. Đối chiếu kép: bản **inline trong dispatcher tổng** `0078a89c_FUN_0078a89c.c:6723` (`case 0x35:`) + `:6733–6789` — khớp 100% với `case_046`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).
> **13/14 callee trước đây opaque NAY ĐÃ CÓ BODY** (`0064cd74, 0064d128, 0064dfa4, 0064e064, 0064e3e0, 0064e8f4, 0064f9f0, 0064faa8, 0064fbd8, 0064fd34, 0074789c, 0075c518, 0076a944`) — wire layout khôi phục ở §3 + chi tiết trích dẫn dòng ở §4.3.

> Phạm vi: **core logic opcode**; thuần graphics/FX chỉ nêu để định vị.

---

## 1. Tóm tắt nghiệp vụ

**Main OP 0x35 là kênh S→C "biến cố trận đánh"** — gần như toàn bộ 14 SubOp ủy quyền cho **`TFightManage`** (`gvar_007D9CE4`, VMT `0x63CCDC`), 3 SubOp còn lại đẩy sang các manager thực thể liên quan trận đấu (`gvar_007D9D34` registry — trùng object với OP 0x16, `TLifeManage`, `TThingManage`).

Điểm mấu chốt đã chứng minh: các handler TFightManage làm việc trên **battle-record toàn cục `DAT_0098C63C`** (con trỏ ≠ 0 **chỉ khi đang trong trận** — guard `0064cfec.c:69`, `00644294.c:101`), với **lưới trận 4 nhóm × 5 ô = 20 vị trí**.

| SubOp | Đích (self-object) | Body decompile? |
| :---: | :--- | :---: |
| `0x01` | `FUN_0064cd74` (TFightManage) | ✓ mới — §4.3 |
| **`0x03`** | `FUN_0064cfec` (TFightManage) | **✓ — phân tích đầy đủ §4.1** |
| `0x04` | `FUN_0064d128` (TFightManage) | ✓ mới — §4.3 |
| `0x05` | `FUN_0064dfa4` (TFightManage) | ✓ mới — §4.3 |
| `0x06` | `FUN_0064e064` (TFightManage) | ✓ mới — §4.3 |
| `0x07` | `FUN_0064e3e0` (TFightManage) | ✓ mới — §4.3 |
| `0x08` | `FUN_0064e8f4` (TFightManage) | ✓ mới — §4.3 |
| `0x09` | `FUN_0064f9f0` (TFightManage) | ✓ mới — §4.3 |
| `0x0A` | `FUN_0074789c` (registry `gvar_007D9D34`) | ✓ mới — §4.3 (*không* lồng trong `FUN_00747800`, xác nhận lại) |
| `0x0B` | `FUN_0075c518` (**TLifeManage** `gvar_007DA250`) | ✓ mới — §4.3 |
| `0x0C` | `FUN_0064faa8` (TFightManage) | ✓ mới — §4.3 |
| `0x0D` | `FUN_0076a944` (**TThingManage** `gvar_007DA0D0`) | ✓ mới — §4.3 |
| `0x0E` | `FUN_0064fbd8` (TFightManage) | ✓ mới — §4.3 |
| `0x0F` | `FUN_0064fd34` (TFightManage) | ✓ mới — §4.3 |

- **Không có SubOp `0x02`, không có `default`** → SubOp khác bị bỏ qua im lặng (`case_046.c:27–69`).
- **OP 0x35 thuần S→C — không tồn tại gói C→S nào** (bằng chứng quét toàn bộ §6).
- ~~13/14 SubOp **opaque**~~ → **(đính chính 2026-09-14)** 14/14 SubOp đã có body; wire layout khôi phục tại §3 + §4.3.

**Đính chính quan trọng:** `gvar_007DA250` ban đầu bị nghi là `TVenderManage` (vì `TVenderManage.Create` nằm kề `0x0075C004`). Kiểm tra điểm tạo thật: `0050a4a0_TForm1.FormCreate.c:631–632` → **`TLifeManage_Create(VMT_75C0B4)`**. `TVenderManage` thực sự là `gvar_007D9C74` (`:622–623`).

---

## 2. Entry & cách đọc PayloadBuffer

Framing/dispatcher đã xác minh (handoff mục 1): `Frame = [F4 44][Len:Word LE][Payload]`, XOR `0xAD`; `Payload = [MainOp][SubOp][...]`.

`FUN_007952ab` (`case_046.c:20–27`):
```c
iVar2 = *(int *)(unaff_EBP + -0xc);                     // ECX = RestPayload
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)iVar2;    // SubOp = ECX[0] = payload[1]
switch (SubOp) { case 1,3,...,0xF: FUN(self=gvar_XXX, ECX) }
```

### Quy đổi chỉ số
| Cách ghi trong mã | Ý nghĩa wire |
| :--- | :--- |
| `ECX[i]` (0-based) | `payload[1+i]` |
| `_LStrCopy(ECX, p, n)` | `n` byte từ `payload[p]` |
| `FUN_0077EB9C` = Word LE; `FUN_0077ED68`/`FUN_0077EF7C` = DWORD LE | (SubOp 3 KHÔNG dùng codec — đọc byte trực tiếp) |

---

## 3. Bảng SubOp → wire layout

(cập nhật 2026-09-14 — mọi layout dưới đây suy từ body mới; chi tiết + trích dẫn dòng ở §4.3)

| SubOp | Độ dài payload | Layout | Ý nghĩa core (theo đúng code) | Độ tin cậy |
| :---: | :---: | :--- | :--- | :---: |
| `0x01` | `2 + 5n` | `[35][01]([grp≤3][slot≤4][type∈1..5][w Word LE])×n` | tra unit tại ô → gọi 1 trong 5 method `FUN_0064be2c/c108/c49c/c7a8/c880(self, unitIdx, w)` theo `type` | **CAO** (body đủ) |
| `0x03` | **4** | `[35][03][grp:1B ∈0..3][slot:1B ∈0..4]` | Ép **đơn vị đang đứng ở ô (grp,slot) của lưới trận** chuyển sang **state 7 (hoạt ảnh di chuyển/tấn công)** + **xả sạch 21 khe hiệu ứng FX** | **CAO** (body đầy đủ) |
| `0x04` | `6 + 2m` | `[35][04][id Word LE≠0][grp≤3][slot≤4](b1 b2)×m`, `m=(L−5)/2` | cần `battle≠0 && id≠0`; với unit tại ô: `FUN_00654590(unit, b1, b2, id)` — **helper này vẫn chưa body** | CAO (wire) / THẤP (ngữ nghĩa `FUN_00654590`) |
| `0x05` | 4 | `[35][05][grp≤3][slot≤4]` | ô có unit → `*(unit+0x552)=1` (cờ trạng thái) | **CAO** |
| `0x06` | 11 | `[35][06][k][id DWORD LE][tpl DWORD LE]` | tạo `THuman` (VMT `0x70B1C4`) → `vmt+0x1c(human, tpl, 0)` → `FUN_0063c1d0(*gvar_007D9D6C, human, 0)`; dựng toast từ tên nạn nhân (`FUN_0070c20c(*gvar_007D9D34, id)` → `gvar_007DA300[idx]+9`, id 0 → tên chính mình `*gvar_007DA7BC+9`) + template literal `0x64E330`, banner 2000 ms qua `gvar_007D9D6C`+0x90 | **CAO** |
| `0x07` | 6 | `[35][07][srcGrp≤3][srcSlot≤4][dstGrp≤3][dstSlot≤4]` | **CHUYỂN Ô trên lưới**: dời record ô `[+0x48 dword][+0x4c unitIdx]` src→dst rồi xóa src; cập nhật `unit+0x56e/0x56f = dstGrp/dstSlot`, `unit+0x54/0x58 ← cell+0x40/+0x44` và `+0x1c/0x20/0x4c/0x50 := +0x54/+0x58`; nếu unit dst đang state 7 → `FUN_006540b0` | **CAO** (KHÔNG guard `battle≠0` → ngoài trận AV) |
| `0x08` | 4 | `[35][08][w1 Word LE]` | `*(*gvar_007DA7BC + 0x62E) = w1` (ghi Word vào object lớn `gvar_007DA7BC`) | **CAO** |
| `0x09` | 6 | `[35][09][w1 Word LE][w2 Word LE]` | `*(*gvar_007DA7BC + 0x630) = w1`, `+0x632 = w2` | **CAO** |
| `0x0A` | biến thiên | `[35][0A]([len][text][w Word LE])×3` | đúng 3 bản ghi: `text → *(PAnsiString)(*gvar_007D9D34 + 0x53DB + i*6)`, `w → *(Word)(… + 0x53DF + i*6)`, i=1..3 | **CAO** |
| `0x0B` | biến thiên | `[35][0B]([type∈1..3][len][text])×≤21` | `FUN_007ab870(*gvar_007DA1B0 = **TTalkMsgForm**, 0, <prefix 0x75C6A8/B8/C8 + text>, 0)` — đẩy **tin nhắn chat/thông báo** theo 3 nhóm prefix | **CAO** |
| `0x0C` | 4 | `[35][0C][grp≤3][slot≤4]` | cần trong trận + ô có unit → `*(unit+0x5BD)=1` | **CAO** |
| `0x0D` | 7 | `[35][0D][id Word LE][count byte][flag byte]` | ghi `*gvar_007DA7BC+0x1505=id`, `+0x1507=count`; count=0 → clear + `*gvar_007DA1DC+0x1BC=-1`, ngược lại tra `FUN_00775c68(*gvar_007DA540,…)` → `IntToStr` → `FUN_007c9b38(*gvar_007D9ED8, str)` → `*gvar_007DA1DC+0x1BC = index`; flag=1 → banner 2000 ms (`gvar_007DA084+0x90`, template literal `0x76AB4C/60/6C` qua `_LStrCatN` 6 tham số) | **CAO** (ngữ nghĩa "vật" vẫn suy luận) |
| `0x0E` | 6 | `[35][0E][grp≤3][slot≤4][w Word LE]` | cần trong trận + ô có unit → gọi virtual `unit.VMT+0x1C(unit, w, 1)` | **CAO** |
| `0x0F` | 8 | `[35][0F][grp≤3][slot≤4][mode 'B'..'G'][byte][w Word LE]` | tra unit tại ô; mỗi mode ghi cặp `(byte,Word)` vào `unit+0x5BE/0x5BF, 0x5C1/0x5C2, 0x5C4/0x5C5, 0x5C7/0x5C8, 0x5CA/0x5CB, 0x5CD/0x5CE` và gọi `FUN_0058f5e0(*gvar_007DA530, unitIdx, mode, w, byte)`; mode khác `'B'..'G'` → bỏ qua | **CAO** (KHÔNG guard `battle≠0` → ngoài trận AV) |

---

## 4. Chi tiết nhánh khôi phục được

### 4.1. `SubOp 0x03` — `FUN_0064cfec` (TFightManage) — `0064cfec.c:45–123` (ĐÃ ĐỌC TOÀN BỘ)

```c
grp  = RP[1];                                  // = payload[2], BoundErr nếu len(RP)<2
slot = RP[2];                                  // = payload[3], BoundErr nếu len(RP)<3
if (DAT_0098C63C != 0) {                       // CHỈ trong trận
    cell  = DAT_0098C63C + grp*0x46 + 0x48 + slot*14;   // BoundErr grp≤3, slot≤4
    if (*(int*)cell != 0) {                    // ô có đơn vị (DWORD "occupied/id")
        FUN_00651824(DAT_0098C63C);            // 1) Xả sạch 21 khe FX: lặp i=0..20, FUN_00664ffc(battle+0x1AC[i])
        u = *(byte*)(cell + 4);                // 2) unit-index (bound ≤0x14)
        FUN_00647708(*(battle+0x158 + u*4), 7);// 3) ÉP đơn vị sang STATE 7
    }
}
```

**Bản chứng diễn giải (từng dòng):**
- `0064cfec.c:53–68`: đọc **2 byte** `payload[2]`, `payload[3]` trực tiếp (không `_LStrCopy`) → **wire = 4 byte tổng cộng** `[35][03][grp][slot]`.
- `:69–89`: số học ô trận: `stride nhóm = 0x23*2 = 0x46 = 70 byte/nhóm`; `stride ô = 7*2 = 14 byte/ô`; base ô = `battle+0x48`. Ô 14 byte: `[+0x00: DWORD occupied/id][+0x04: byte unitIdx][:8 pad/flag]`. Lưới **4 nhóm × 5 ô = 20 vị trí trận** (typical formation board).
- `:90` — `FUN_00651824(battle)` (`00651824.c:28–35`): duyệt **mảng 21 con trỏ tại `battle+0x1AC`** (ngay sau mảng unit `+0x158 + 21*4 = 0x1AC` ✓), gọi `FUN_00664ffc` mỗi phần tử — giant switch 259 loại → `TObject.Free` các field ảnh/FX ⇒ **clear toàn bộ hiệu ứng đang treo** trước khi dựng nhịp mới. (Bản chất FX/sprite — chỉ ghi để hiểu trình tự.)
- `:110–115`: đọc `unitIdx = cell+4`, bound `≤ 0x14` (21 slot đơn vị), lấy `unit = *(battle+0x158+unitIdx*4)` rồi `FUN_00647708(unit, 7)`.

**`FUN_00647708` = bộ máy trạng thái đơn vị** (`00647708.c:160–189`, ĐÃ XÁC MINH TRỰC TIẾP):
```c
if (unit+0x391 == 3) FillChar(unit+0xE9, 0x30, 0);   // đang chết? reset path
unit+0x391 = state;                                   // byte STATE
switch (state) { case 2: 0x644fb4; case 6: 0x6459fc;
                 case 7: FUN_00653F18(unit,1);  case 8: 0x654490; case 10: 0x6595e4 }
```
**State 7** → `FUN_00653F18` (`00653f18.c`): chép cặp tọa độ `unit[0x15/0x16]` vào **mảng đường dẫn 6 điểm `unit+0xE9..0xF3`**, `unit+0xED=0x80`, gọi virtual `VMT+0x18(unit, 0x1A)` kèm `Now/Sleep` + test khoảng cách `<300px` ⇒ **kích hoạt sequence di chuyển/tấn công theo path đã chốt sẵn trên battle record** (animation render — bỏ qua chi tiết).

**=> Ý nghĩa SubOp 3:** *"server xác nhận nước đi/nhịp đánh cho ô (grp,slot): unit tương ứng của ô đó chạy hoạt động state 7; mọi FX cũ bị dội sạch."*

### 4.2. 13 nhánh từng opaque → ĐÃ PHỤC HỒI (2026-09-14)

Bản cũ (2026-09-12) ghi các helper này nằm ở GAP giữa các hàm đã export (`func_0x...`, không `.c` lẫn `.asm.txt`) — **đính chính**: toàn bộ 13 hàm đã được Ghidra tạo function và decompile trong đợt redump (`missing_opcode_sources.md`); bảng §4.2 cũ giữ làm lai lịch. Kết quả đọc từng body ở §4.3 dưới đây. Ghi chú kỹ thuật còn đúng: `func_0x0074789c` **không** lồng trong `FUN_00747800` (header body ghi caller duy nhất `sub_007953d8` = case 46 — `0074789c_FUN_0074789c.c:8,13`).

### 4.3. Chi tiết 13 nhánh khôi phục (wire 0-based `RP[0]=SubOp`; mọi BoundErr là guard index Delphi)

- **SubOp 1 — `FUN_0064cd74`** (`0064cd74_FUN_0064cd74.c`): flush FX đầu hàm (dòng 53); `n = (len(RP)−1)/5` (dòng 55-61) record 5 byte `[grp][slot][type][w:2]` (dòng 64-102); chỉ `type ∈ 1..5` (dòng 108) và `battle≠0` (dòng 109): tra unit → switch `type` gọi `FUN_0064be2c` (1) / `FUN_0064c108` (2) / `FUN_0064c49c` (3) / `FUN_0064c7a8` (4) / `FUN_0064c880` (5) — `(self, unitIdx, w)` (dòng 112-127). 5 method đích đều **đã có body** trong `functions/` (chưa khảo sát ngữ nghĩa từng cái trong tài liệu này).
- **SubOp 4 — `FUN_0064d128`** (`0064d128_FUN_0064d128.c`): `[id Word RP1..2][grp RP3][slot RP4]` + `m = (len−5)/2` cặp byte từ `RP5` (dòng 58-81); guard `battle≠0 && id≠0` (dòng 74); mỗi cặp: unitIdx tại ô `(grp,slot)` (byte `cell+0x4C`, dòng 112-131) → `FUN_00654590(unit, b1, b2, id)` (dòng 137). **`FUN_00654590` vẫn chưa có body** → bản chất cặp byte (damage? hiệu ứng?) **chưa kết luận được**.
- **SubOp 5 — `FUN_0064dfa4`** (`0064dfa4_FUN_0064dfa4.c`): `[grp RP1][slot RP2]`; guard battle (dòng 57); ô có unit → `*(unit+0x552)=1` (dòng 64). Cờ đơn phương, không tham số phụ.
- **SubOp 6 — `FUN_0064e064`** (`0064e064_FUN_0064e064.c`): `[k RP1][id DWORD RP2..5][tpl DWORD RP6..9]` (dòng 59-63); tạo `THuman_Create(VMT_70B1C4)` + virtual `+0x1c(human, tpl, 0)` + `FUN_0063c1d0(*gvar_007D9D6C, human, 0)` (dòng 64-66 — `gvar_007D9D6C` = instance thứ hai của **TSe_TalkMsgFormPlus**, `0051189c_FUN_0051189c.c:1295-1296`); nhánh toast: `id<1` → literal `0x64E308` (dòng 67-71); `k==1` → `Random(2)` chọn `0x64E2E4`/`0x64E308` (dòng 73-81); ngược lại tên = `*(gvar_007DA7BC+9)` nếu `FUN_0070c20c(*gvar_007D9D34, id)==0` hoặc `*(gvar_007DA300 + idx*4 + 9)` (biên idx ≤ 800, dòng 83-93), ghép template: `Random(1)` **luôn = 0** (helper LCG `[0,n)`, `00402cf0_FUN_00402cf0.c:760-765`) ⇒ chỉ case 0 chạy: `_LStrCat3(msg, name, DAT_0064E330)`; các case 1..5 (`0x64E350..0x64E3B4`) **không tới được trong build này** (dòng 94-113); banner `gvar_007D9D6C+0x90`, 2000 ms (dòng 116). ⇒ "human mới (THuman) được spawn + toast mang tên ai đó" — ngữ nghĩa `tpl` **chưa kết luận**.
- **SubOp 7 — `FUN_0064e3e0`** (`0064e3e0_FUN_0064e3e0.c`): `[srcGrp RP1][srcSlot RP2][dstGrp RP3][dstSlot RP4]` (dòng 90-111). Nếu **unit tại ô ĐÍCH** đang `state(+0x391)==7` → `FUN_006540b0(unit)` (dòng 112-159). Sau đó **dời ô**: `dst.cell+0x48 (DWORD) := src.cell+0x48` (dòng 161-204), `dst.cell+0x4C (byte) := src.cell+0x4C` (dòng 205-248), `src.cell+0x48 := 0` (dòng 268), `src.cell+0x4C := 0` (dòng 288); unit mới: `+0x56E := dstGrp`, `+0x56F := dstSlot` (dòng 313,318), `+0x54 := cell+0x40`, `+0x58 := cell+0x44` (dòng 338-370) và `+0x1C/+0x20/+0x4C/+0x50 := +0x54/+0x58` (dòng 371-414). ⇒ **nghi thức "đơn vị dời từ ô src sang ô dst" + cập nhật bộ đệm tọa độ**. **Không guard `battle≠0`** (deref dòng 131/180/204…) → ngoài trận = EAccessViolation. Khớp đối xứng với SubOp 3 (`0064cfec` ép state 7 trên ô) — cặp **move/commit** chính là suy đoán "open/close-round" bản cũ, **nay xác minh được từ body mới** ở mức cơ chế (tên ngữ nghĩa vẫn là suy luận).
- **SubOp 8 — `FUN_0064e8f4`** (`.c:37-39`): `[w RP1..2]` → `*(*gvar_007DA7BC + 0x62E) = w`. SubOp 9 — **`FUN_0064f9f0`** (`.c:39-44`): hai Word → `+0x630`, `+0x632`. Không đi qua `self` TFightManage (chỉ nhận RP làm payload, `self` bị bỏ trống — param_1 không dùng) — ghi trực tiếp vào object `gvar_007DA7BC`.
- **SubOp 0xA — `FUN_0074789c`** (`0074789c_FUN_0074789c.c`): **đúng 3** vòng lặp `i=1..3` (dòng 54-58, 130) trên `self = *gvar_007D9D34`: `[len RPpos][text len byte][w Word]` → `_LStrCopy` **thẳng vào bộ nhớ object** tại `self + 0x53DB + i*6` (dòng 82-90) và `w` vào `self + 0x53DF + i*6` (dòng 103-115); pos += len+3 (dòng 116-125); `pos > len` thì kết thúc sớm (dòng 62-66). ⇒ 3 slot chuỗi-Word ghi chồng tại rìa `+0x53DB` của registry (nhất quán với ghi chú "field tới `+0x5544`").
- **SubOp 0xB — `FUN_0075c518`** (`0075c518_FUN_0075c518.c`): tối đa **0x15=21** record `[type RP+0][len][text]` (dòng 52-113); `type 1/2/3` → ghép prefix literal `DAT_0075C6A8/0x75C6B8/0x75C6C8` + text (đây là **3 chuỗi nằm trong vùng code chưa dump**) rồi `FUN_007ab870(*gvar_007DA1B0, 0, msg, 0)` (dòng 90-107) — `gvar_007DA1B0` = **`TTalkMsgForm` (VMT `0x7AB774`**, `0051189c_FUN_0051189c.c:1585-1586`) ⇒ **đẩy tin nhắn chat/bảng tin**, không phải "hồi phục đơn vị" như nghi vấn bản cũ — **đính chính**: không có thao tác unit/sức sống nào trong body.
- **SubOp 0xC — `FUN_0064faa8`** (`.c:54-107`): `[grp][slot]`, guard battle (dòng 62) + ô có `cell+0x48 ≠ 0` (dòng 82) → `*(unit+0x5BD)=1`. Cờ.
- **SubOp 0xD — `FUN_0076a944`** (`0076a944_FUN_0076a944.c`): `[id Word RP1..2]` → `*gvar_007DA7BC+0x1505`; `[count RP3]` → `+0x1507`; `[flag RP4]` (dòng 56-72). `count==0` → `FillChar(+0x1505, 3, 0)` + `*gvar_007DA1DC+0x1BC = -1` (dòng 73-76); ngược lại: `FUN_00775c68(*gvar_007DA540, id, rec)` → ushort `rec+0x18` → `IntToStr` → `FUN_007c9b38(*gvar_007D9ED8, str)` → `*gvar_007DA1DC+0x1BC = index` (dòng 78-81); `flag==1` → đọc tiếp `FUN_00774a84(*gvar_007DA540, id, &name)` + `IntToStr(count)` + `_LStrCatN(&msg, 6)` với literals `0x76AB4C/0x76AB60/0x76AB6C` → **banner 2000 ms** `gvar_007DA084+0x90` (dòng 82-97). ⇒ "chọn/đặt một vật (id) với số lượng, tra bảng tên `gvar_007D9ED8`" — chủ đề **vật thể** của `TThingManage` **xác minh được một phần** (có bảng tên + count), nhưng là vật rơi hay vật phẩm vẫn suy luận.
- **SubOp 0xE — `FUN_0064fbd8`** (`.c:58-114`): `[grp][slot][w Word]`; guard battle + ô có unit (dòng 68-88) → `unit.VMT+0x1C(unit, w, 1)` (dòng 114).
- **SubOp 0xF — `FUN_0064fd34`** (`0064fd34_FUN_0064fd34.c`): `[grp][slot][mode char RP3][byte RP4][w Word RP5..6]` (dòng 57-81); tra unit theo ô (dòng 82); `switch(mode)` đúng 6 case `'B','C','D','E','F','G'` (dòng 86-169), mỗi case ghi `(byte, Word)` vào 6 ô nhớ liền kề của unit: `0x5BE/0x5BF, 0x5C1/0x5C2, 0x5C4/0x5C5, 0x5C7/0x5C8, 0x5CA/0x5CB, 0x5CD/0x5CE` và gọi `FUN_0058f5e0(*gvar_007DA530, unitIdx, mode, w, byte)` (vd `'B'`: dòng 90-98). Không có default → mode khác bị bỏ. **Không guard `battle≠0`** → ngoài trận AV.

---

## 5. Bảng global

| Symbol | Vai trò | Bằng chứng |
| :--- | :--- | :--- |
| `gvar_007D9CE4` | **`TFightManage`** (VMT `0x63CCDC`) — chiến đấu manager; đã biết thêm method `FUN_0064246c` (batch snapshot của OP 0x16/0x04) | Tạo `0050a4a0_TForm1.FormCreate.c:615–616` |
| `DAT_0098C63C` | **Con trỏ battle-record toàn cục** (null = ngoài trận). Layout suy ra: `+0x06 byte`, ô trận = `[groupBase + 0x40 + slot*14]` size 14B = `+0x00 X:DWORD][+0x04 Y:DWORD][+0x08 occupied:DWORD][+0x0C unitIdx:byte]` (4 nhóm stride `0x46` — **đính chính nhỏ**: bản cũ lấy gốc ô là `+0x48` vì helper SubOp 3 chỉ đụng `occupied/unitIdx`; hai field `+0x40/+0x44` xuất hiện từ body `FUN_0064e3e0` §4.3), `+0x158` 21 ptr unit, `+0x1AC` 21 ptr FX, thêm 5 mảng ptr 21 phần tử tại `+0x200/+0x254/+0x2A8/+0x2FC/+0x350` (dội FX id `0xb` — `0064bb24_FUN_0064bb24.c:113-133`), cờ `+0xE5B/+0xE68` (reset bởi `FUN_00647164` — `opcode_32.md` §4.2) | guard `0064cfec.c:69`, `00644294.c:101`; bằng chứng con trỏ thật: `00648154.asm.txt:535–536`; **điểm cấp phát: VẪN UNKNOWN** — grep re-export 2026-09-14 không tìm thấy lệnh gán slot `gvar_0098C63C/DAT_0098c63c` nào (chỉ đọc + qua con trỏ) |
| `gvar_007D9D34` | Registry thực thể/target (cùng object với OP 0x16 SubOp 7–10; field tới `+0x5544`; **3 slot ghi mới `+0x53DB..+0x53F5` từ OP 0x35 SubOp 0xA**, §4.3) | **0 lần gán vế trái trong toàn dump (kể cả re-export 2026-09-14) → điểm tạo UNKNOWN** |
| `gvar_007DA250` | **`TLifeManage`** (VMT `0x75C0B4`) | `0050a4a0:631–632` |
| `gvar_007DA0D0` | **`TThingManage`** (VMT `0x767404`) | `0050a4a0:664–665` |
| `unit+0x391` | byte STATE của đơn vị chiến (đang chết=3; các state 2/6/**7**/8/10 có handler riêng) | `00647708.c:165–172` |
| `gvar_007D9D30` | codec/connection self khi gọi `FUN_0077f414`/decoder | `0074927c.asm:16` |
| `gvar_007DA7BC` | (mới, 2026-09-14) object toàn cục mà SubOp 8/9/0x0D **ghi trực tiếp**: `+0x62E`, `+0x630/+0x632`, `+0x1505/+0x1507`; SubOp 6 đọc `+9` làm **tên chính mình** khi id tra không thấy (`0064e064_FUN_0064e064.c:85`) — nhất quán nhãn "player/avatar" ở `opcode_39.md` | `0064e8f4_FUN_0064e8f4.c:39`, `0064f9f0_FUN_0064f9f0.c:41-44`, `0076a944_FUN_0076a944.c:58-65` |

---

## 6. Chiều C→S

**KHÔNG TỒN TẠI gói C→S với MainOp 0x35.** Chuỗi bằng chứng:
1. `0077f414_FUN_0077F414.c` **không có** `case 0x35` (jump-table artifact).
2. Prologue ASM builder (`0077f414.asm.txt:22–34`): `MainOp = DL` (`[EBP-0x5]`), `SubSel = CL` (`[EBP-0x6]`) ⇒ muốn gửi 0x35 phải có `MOV DL,0x35` tại call-site.
3. Quét **toàn bộ 6302 file asm** đã export bằng python: **0 hit `MOV DL,0x35`**. Hai site duy nhất chứa hằng `0x35` gần `CALL 0x77f414` là `0074927c.asm:18–20` và `0074e8fc.asm:18–20`: `MOV CL,0x35; MOV DL,0x17` ⇒ gửi **MainOp 0x17 với SubSel 0x35** (OP 0x17 = túi đồ), không phải 0x35.
4. `MOV DX,0x35` duy nhất (`00648154.asm:538`) = hằng số 53 làm tham số `CALL 0x666bf0`, không liên quan opcode.
⇒ **OP 0x35 là kênh server-push một chiều.** (Ghi chú: các hành động người chơi trong trận đi bằng opcode khác, ví dụ 0x17/0x20 — ngoài phạm vi file này.)

---

## 7. Chuỗi literal & encoding (VISCII→UTF-8)

- `case_046`, `FUN_0064cfec` và các helper `00651824/00647708/00653f18`: **không có chuỗi hiển thị nào** — thuần số học con trỏ/byte; các `UNK_007952xx`/`LAB_00796408` là **nhãn SEH/frame cleanup**, không phải text.
- Bảng tên-opcode-debug trong `0078a89c.c` (vd `case 0x36 → @LStrLAsg(..., 0x796CAC)`) trỏ tới vùng string `0x00796Cxx`, nhưng **`redump/` không có dump phủ vùng này** ⇒ tên chữ của 0x35/0x36: **chưa có dump → không dịch được** (không bịa).
- ~~5 helper `0x74789c/0x75c518/0x76a944/…` nếu chứa text (toast battle) cũng chưa có body → chưa dịch được.~~ **(cập nhật 2026-09-14)** các body mới **có chứa literal thật**, nhưng là **AnsiString nhúng trong vùng code** chưa hề được redump — bổ sung vào danh sách dump: `0x64E2E4, 0x64E308, 0x64E330, 0x64E350, 0x64E364, 0x64E378, 0x64E390, 0x64E3B4` (toast SubOp 6, `0064e064_FUN_0064e064.c:68-112`), `0x75C6A8/0x75C6B8/0x75C6C8` (prefix chat SubOp 0xB, `0075c518_FUN_0075c518.c:92-104`), `0x76AB4C/0x76AB60/0x76AB6C` (banner SubOp 0xD, `0076a944_FUN_0076a944.c:85-94`). **Chưa dịch được nội dung** — chỉ đã biết *vai trò* từng literal theo ngữ cảnh ghép chuỗi.

---

## 8. Ghi chú cho Mock Server

1. **Chỉ SubOp 3 đặc tả được** *(cập nhật 2026-09-14: **cả 14 SubOp đều đã đặc tả được** theo bảng §3)*: gửi `[0x35][0x03][grp][slot]` (`grp∈0..3`, `slot∈0..4`) **chỉ khi client đang trong trận** (`DAT_0098C63C≠0`) và ô có đơn vị — ngoài trận client **ignore an toàn** (riêng SubOp 7 và 0xF **KHÔNG guard** → gửi ngoài trận gây EAccessViolation, tuyệt đối tránh). Hiệu ứng: FX flush + unit của ô chạy move/attack state 7.
2. ~~**13 SubOp kia chưa thể mock**~~ → **đã mock được** (wire §3, §4.3).Ràng buộc chung: các SubOp chạm lưới trận (`0x01,0x03,0x04,0x05,0x07,0x0C,0x0E,0x0F`) chỉ phát khi trong trận; SubOp `0x08/0x09/0x0A/0x0B/0x0D` chỉ ghi global/registry/chat → dùng được cả ngoài trận.
3. Cặp nghi vấn "open/close-round" bản cũ (`0x64cd74` ↔ `0x64d128`): body cho thấy `0x64cd74` = **batch 5 loại thao tác theo ô** và `0x64d128` = **áp cặp byte + id Word lên unit của một ô** — cơ chế đã rõ, nhưng có đúng là mở/kết thúc vòng đấu hay không vẫn **chưa kết luận được** (cần traffic đối chứng).
4. Đừng nhầm với OP 0x16: 0x16 = world-object sync **ngoài/khung cảnh**; 0x35 = **lưới trận đấu 4×5** (battle-record riêng `DAT_0098C63C`, không phải `gvar_007DA6DC`).

---

## 9. Source trail + UNKNOWN

**Đã đọc/kiểm chứng:**
1. `case_functions/functions/case_046_007952AB_FUN_007952ab.c:20–97` — handler + switch.
2. `functions/0078a89c_FUN_0078a89c.c:6723,6733–6789` — inline dispatcher (xác nhận kép).
3. `functions/0064cfec_FUN_0064cfec.c:45–123` — **SubOp 3 (toàn bộ body)**.
4. `functions/00651824_FUN_00651824.c:28–35`; `functions/00647708_FUN_00647708.c:160–189` (state machine, **đã tự đọc**); `00653f18` (state 7); `00664ffc` (FX free, đọc qua agent).
5. `functions/0050a4a0_TForm1.FormCreate.c:615–616,622–623,631–632,664–665` — danh tính 4 gvar (**đã tự đọc**).
6. `functions/0077f414_FUN_0077F414.asm.txt:22–34` + quét python 6302 asm (C→S).
7. `functions/00648154_FUN_00648154.asm.txt:535–538` — `DAT_0098C63C` là con trỏ.
8. `redump/jumptable_byte200_0x78A8EE.hex`, `jumptable_dword200_0x78A9B6.hex` — dispatch.
9. **(mới 2026-09-14)** `functions/0064cd74/0064d128/0064dfa4/0064e064/0064e3e0/0064e8f4/0064f9f0/0064faa8/0064fbd8/0064fd34_FUN_*.c`, `functions/0074789c/0075c518/0076a944_FUN_*.c` — 13 body khôi phục (§4.3); `functions/0065178c_FUN_0065178c.c:50-77` (tra ô); `functions/0051189c_FUN_0051189c.c:1295-1296,1585-1586` (danh tính `gvar_007D9D6C` = TSe_TalkMsgFormPlus #2, `gvar_007DA1B0` = TTalkMsgForm).

**UNKNOWN (còn lại — cập nhật 2026-09-14):**
- ~~13 helper missing ⇒ wire của SubOp 1,4–9,0xA–0xF~~ → **ĐÃ KHÔI PHỤC** (§3, §4.3).
- Ngữ nghĩa sâu: 5 method `FUN_0064be2c/c108/c49c/c7a8/c880` (SubOp 1) và đặc biệt `FUN_00654590` (SubOp 4) + `FUN_0058f5e0` (SubOp 0xF) — **`FUN_00654590` đến nay vẫn chưa có body** (toàn bộ danh sách §4.3 chỉ thiếu nó).
- `gvar_007D9D34` class + điểm tạo (vẫn 0 write-slot sau re-export); điểm cấp phát `DAT_0098C63C` (vẫn không có lệnh gán slot trong dump).
- Tên state `7` chính thức; ý nghĩa cờ unit mới quan sát: `+0x552`, `+0x5BD`, các cặp `0x5BE..0x5CE`; bảng tên `0x796Cxx` và các literal code-embed `0x64E2E4../0x75C6A8../0x76AB4C..` — **cần redump** (§7).
