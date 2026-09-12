# PHÂN TÍCH — Main OP 0x35 (Case 46) — `FUN_007952ab` @ `0x007952AB` — **Biến cố TRẬN ĐÁNH / Battle Events (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x35] = 0x2E (=46)` → `jumptable_dword200_0x78A9B6[46]` (entry `0x0078AA6E`) → target `0x007952AB` = **Case 46**. Đối chiếu kép: bản **inline trong dispatcher tổng** `0078a89c_FUN_0078a89c.c:6723` (`case 0x35:`) + `:6733–6789` — khớp 100% với `case_046`.

> Phạm vi: **core logic opcode**; thuần graphics/FX chỉ nêu để định vị.

---

## 1. Tóm tắt nghiệp vụ

**Main OP 0x35 là kênh S→C "biến cố trận đánh"** — gần như toàn bộ 14 SubOp ủy quyền cho **`TFightManage`** (`gvar_007D9CE4`, VMT `0x63CCDC`), 3 SubOp còn lại đẩy sang các manager thực thể liên quan trận đấu (`gvar_007D9D34` registry — trùng object với OP 0x16, `TLifeManage`, `TThingManage`).

Điểm mấu chốt đã chứng minh: các handler TFightManage làm việc trên **battle-record toàn cục `DAT_0098C63C`** (con trỏ ≠ 0 **chỉ khi đang trong trận** — guard `0064cfec.c:69`, `00644294.c:101`), với **lưới trận 4 nhóm × 5 ô = 20 vị trí**.

| SubOp | Đích (self-object) | Body decompile? |
| :---: | :--- | :---: |
| `0x01` | `func_0x0064cd74` (TFightManage) | ✗ |
| **`0x03`** | `FUN_0064cfec` (TFightManage) | **✓ — phân tích đầy đủ §4.1** |
| `0x04` | `func_0x0064d128` (TFightManage) | ✗ |
| `0x05` | `func_0x0064dfa4` (TFightManage) | ✗ |
| `0x06` | `func_0x0064e064` (TFightManage) | ✗ |
| `0x07` | `func_0x0064e3e0` (TFightManage) | ✗ |
| `0x08` | `func_0x0064e8f4` (TFightManage) | ✗ |
| `0x09` | `func_0x0064f9f0` (TFightManage) | ✗ |
| `0x0A` | `func_0x0074789c` (registry `gvar_007D9D34`) | ✗ — *không* lồng trong `FUN_00747800` (asm kết thúc `RET` tại `0x747890`) |
| `0x0B` | `func_0x0075c518` (**TLifeManage** `gvar_007DA250`) | ✗ |
| `0x0C` | `func_0x0064faa8` (TFightManage) | ✗ |
| `0x0D` | `func_0x0076a944` (**TThingManage** `gvar_007DA0D0`) | ✗ |
| `0x0E` | `func_0x0064fbd8` (TFightManage) | ✗ |
| `0x0F` | `func_0x0064fd34` (TFightManage) | ✗ |

- **Không có SubOp `0x02`, không có `default`** → SubOp khác bị bỏ qua im lặng (`case_046.c:27–69`).
- **OP 0x35 thuần S→C — không tồn tại gói C→S nào** (bằng chứng quét toàn bộ §6).
- 13/14 SubOp **opaque** (helper chưa được Ghidra export — các địa chỉ nằm ở **gap giữa các hàm**, không có `.c` lẫn `.asm.txt`). Chỉ SubOp 3 khôi phục được wire.

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

| SubOp | Độ dài payload | Layout | Ý nghĩa core | Độ tin cậy |
| :---: | :---: | :--- | :--- | :---: |
| `0x03` | **4** | `[35][03][grp:1B ∈0..3][slot:1B ∈0..4]` | Ép **đơn vị đang đứng ở ô (grp,slot) của lưới trận** chuyển sang **state 7 (hoạt ảnh di chuyển/tấn công)** + **xả sạch 21 khe hiệu ứng FX** | **CAO** (body đầy đủ) |
| `0x01,0x04..0x09,0x0C,0x0E,0x0F` | ? | `[35][sub][...]` — **opaque, chưa có body** | Nghi vấn: message/biến cố trận (theo class `TFightManage` + dải address method `0x64cd..0x64fd` thuộc cùng class) | THẤP (không khẳng định wire) |
| `0x0A` | ? | `[35][0A][...]` — opaque | Thao tác registry thực thể `gvar_007D9D34` (cùng object với OP 0x16 SubOp 7/8/10) | THẤP |
| `0x0B` | ? | `[35][0B][...]` — opaque | `TLifeManage` — nghi liên quan **sự sống/hồi phục** đơn vị | THẤP |
| `0x0D` | ? | `[35][0D][...]` — opaque | `TThingManage` — nghi **vật/vật thể rơi trong trận** | THẤP |

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

### 4.2. Vì sao 13 nhánh còn lại opaque
- `func_0x0064cd74 … func_0x0064fd34`, `func_0x0074789c`, `func_0x0075c518`, `func_0x0076a944` — **tất cả nằm trong GAP giữa** các hàm đã export theo `index.csv`: vd `func_0x0064cd74` nằm lọt giữa `[0x64CADE…0x64CF60]` (giữa `FUN_0064c880` và `FUN_0064cf60`), `func_0x0064d128` nằm lọt giữa `[0x64D11B…0x64D32C]` (ngay sau `FUN_0064cfec`). Ghidra không tạo function tại đó ⇒ không `.c` lẫn `.asm.txt`.
- `func_0x0074789c` nằm sát ngay sau `FUN_00747800 [0x747800,0x747899]`; đã kiểm `.asm.txt` của nó: kết thúc bằng `RET` trước `0x74789C` ⇒ **không phải nested procedure bị gộp**.
- Suy luận chủ đề theo class **không đủ** để dựng wire — **giữ nguyên opaque**, chờ redump.

---

## 5. Bảng global

| Symbol | Vai trò | Bằng chứng |
| :--- | :--- | :--- |
| `gvar_007D9CE4` | **`TFightManage`** (VMT `0x63CCDC`) — chiến đấu manager; đã biết thêm method `FUN_0064246c` (batch snapshot của OP 0x16/0x04) | Tạo `0050a4a0_TForm1.FormCreate.c:615–616` |
| `DAT_0098C63C` | **Con trỏ battle-record toàn cục** (null = ngoài trận). Layout suy ra: `+0x48` mảng 20 ô 14B (4 nhóm stride `0x46`), `+0x158` 21 ptr unit, `+0x1AC` 21 ptr FX | guard `0064cfec.c:69`, `00644294.c:101`; bằng chứng con trỏ thật: `00648154.asm.txt:535–536` `MOV EDX,[0x98c63c]; MOV EAX,[EDX+EAX*4+0x1ac]`; **điểm cấp phát: UNKNOWN** |
| `gvar_007D9D34` | Registry thực thể/target (cùng object với OP 0x16 SubOp 7–10; field tới `+0x5544`) | **0 lần gán vế trái trong toàn dump → điểm tạo UNKNOWN** |
| `gvar_007DA250` | **`TLifeManage`** (VMT `0x75C0B4`) | `0050a4a0:631–632` |
| `gvar_007DA0D0` | **`TThingManage`** (VMT `0x767404`) | `0050a4a0:664–665` |
| `unit+0x391` | byte STATE của đơn vị chiến (đang chết=3; các state 2/6/**7**/8/10 có handler riêng) | `00647708.c:165–172` |
| `gvar_007D9D30` | codec/connection self khi gọi `FUN_0077f414`/decoder | `0074927c.asm:16` |

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
- 5 helper `0x74789c/0x75c518/0x76a944/…` nếu chứa text (toast battle) cũng **chưa có body → chưa dịch được**.

---

## 8. Ghi chú cho Mock Server

1. **Chỉ SubOp 3 đặc tả được**: gửi `[0x35][0x03][grp][slot]` (`grp∈0..3`, `slot∈0..4`) **chỉ khi client đang trong trận** (`DAT_0098C63C≠0`) và ô có đơn vị — ngoài trận client **ignore an toàn**. Hiệu ứng: FX flush + unit của ô chạy move/attack state 7.
2. **13 SubOp kia chưa thể mock** (wire chưa biết) — bắt buộc **redump 11 hàm** ở §4.2 (đặc biệt `0x64cd74` đứng đầu cluster và `0x64d128` vì đi ngay sau SubOp 3 ⇒ có thể là cặp open/close-round của trận).
3. **Thứ tự nghi vấn theo ngữ cảnh trận**: nếu mock server tự dựng engine trận, ưu tiên dump theo thứ tự `0x64cd74 → 0x64d128 → 0x64dfa4 → 0x64e064 → 0x64e3e0 → 0x64e8f4 → 0x64f9f0 → 0x64faa8 → 0x64fbd8 → 0x64fd34` (9/14 nhánh battle nằm ở đây).
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

**UNKNOWN (cần redump):** 13 helper missing (§4.2) ⇒ wire của SubOp 1,4–9,0xA–0xF; `gvar_007D9D34` class + điểm tạo; điểm cấp phát `DAT_0098C63C`; chi tiết layout 14B/ô (8 byte cuối chưa quan sát); tên state `7` chính thức; nội dung chuỗi vùng `0x796Cxx`.
