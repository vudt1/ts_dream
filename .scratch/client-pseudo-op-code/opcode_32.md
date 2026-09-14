# PHÂN TÍCH — Main OP 0x32 (50) / Case 43 / FUN_007951da @ 0x007951DA

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)** (kèm khảo sát đối chiếu C→S)
Trạng thái: **Đã xác minh phần khung + dispatch từ mã sơ cấp** (`ts_decompile/` only). Handler chỉ là lớp passthrough 2 nhánh `SubOp 1/2`; **cả 2 callee `00647164`/`0064dd74` ĐÃ có body từ bản redump 2026-09-14** (HOLE `0x00647162→0x00647524` đã decompile) — xem §4.2/§4.3.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh `S→C` mỏng ở tầng handler — handler **không tự parse bất kỳ field nào**. Nó chỉ đọc `SubOp = RP[0]`, rồi chuyển nguyên con trỏ RestPayload cho 1 trong 2 hàm thuộc **TFightManage** (`gvar_007D9CE4`):
  - `SubOp == 1` → `FUN_00647164(*gvar_007D9CE4, RP)`
  - `SubOp == 2` → `FUN_0064dd74(*gvar_007D9CE4, RP)`
  - Tất cả SubOp khác → **im lặng (no-op)**, không có `default`.
- **Hệ quả cho reverse-engineering** *(cập nhật 2026-09-14)*: layout field của payload **đã suy ra được từ 2 callee mới decompile** (§4.2/§4.3): cả hai là bộ parse **chuỗi record trên lưới trận 4×5** (`DAT_0098c63c`), dùng chung hạ tầng với OP 0x35 (cùng flush FX `FUN_00651824`, cùng tra ô `FUN_0065178c`, cùng mảng unit `battle+0x158`).
- **Đối xứng C→S**: `ts_decompile/functions/0077f414_FUN_0077F414.c` có `case 0x32:` tương ứng, nhưng bản C bị **cắt cụt** (xem §6). Bản asm cùng hàm (`0077f414_FUN_0077F414.asm.txt`) cho thấy C→S 0x32 **có** builder thật, tự dispatch tiếp theo tham số phụ `[EBP-0x6]` đúng 2 nhánh `==1` và `==2` — **trùng khớp 1↔1 với SubOp 1/2 của chiều S→C**.
- **Chiều C→S có gọi `TForm1.CY_AddSedQueue` (0x0051633c)** ở cả 2 nhánh con (asm dòng 3417 và 3551), nhưng bản C của `case 0x32` không hiển thị — đây là **lỗi/thiếu sót của bộ decompile**, không phải bằng chứng "không build packet".

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x32 (50) → byte_table[0x78A8EE][0x32] = 0x2B (43)
                 → dword_table[0x78A9B6][43] @ 0x0078AA62 = 0x007951DA
                 → FUN_007951da (Case 43)
```

- Bảng byte: `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex:4` (hàng 4 = index 48..63): `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Phần tử offset `0x32-48=2` = `0x2B` = 43.
- Bảng dword: `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex:11` (hàng 11 = index 40..43): `10 49 79 00 | 77 49 79 00 | 71 51 79 00 | DA 51 79 00`. Entry 43 = `DA 51 79 00` = `0x007951DA`. Địa chỉ entry = `0x78A9B6 + 43*4 = 0x0078AA62`.
- Danh mục: `ts_decompile/case_functions/manifest.csv:45` (`43,0x0078AA62,0x007951DA,EXPORTED,"FUN_007951da",...`); `ts_decompile/redump/jumptable_0x78A9B6_case_functions.csv:45` (`43,0x0078AA62,0x007951DA,YES,"FUN_007951da",...`).
- File chính: `ts_decompile/case_functions/functions/case_043_007951DA_FUN_007951da.c` (64 dòng).
- Bản inline trong dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6674-6692` — **logic 1:1**, chỉ khác hình thức do bộ decompile flatten (xem §2.5).
- Bản gộp jumptable: `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:8052-8113` (case index 43, dòng code bắt đầu 8071).
- Framing/XOR/pump định nghĩa như `opcode_00_01.md` §2 (XOR 0xAD, header `F4 44` + length LE).

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x32`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`, tức `RP = P[1..]`). Delphi `_LStrCopy` 1-based.
- `RP_len = *(int*)(RP_ptr - 4)` (độ dài Delphi AnsiString nằm ở offset `-4`).

### 2.3. Đọc SubOp (handler dòng 20-32)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);        // dòng 20: con trỏ RestPayload
iVar1 = 0;                                  // dòng 21
if (*(int *)(iVar2 + -4) == 0) {            // dòng 22: RP_len == 0 ?
  iVar1 = _BoundErr(0);                     // dòng 23: RangeError, L=1 là bất hợp lệ
  iVar2 = extraout_EDX;                     // dòng 24
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1); // dòng 26: SubOp = RP[0]
if (*(int *)(unaff_EBP + -0x14) == 1) {     // dòng 27
  func_0x00647164(*(undefined4 *)gvar_007D9CE4, *(undefined4 *)(unaff_EBP + -0xc)); // dòng 28
}
else if (*(int *)(unaff_EBP + -0x14) == 2) { // dòng 30
  func_0x0064dd74(*(undefined4 *)gvar_007D9CE4, *(undefined4 *)(unaff_EBP + -0xc)); // dòng 31
}
```

- SubOp được lưu vào `*(uint*)(unaff_EBP + -0x14)` (dòng 26), rồi so sánh **int == 1 / == 2**.
- **Không có `default`** → SubOp `0x00` hoặc `>= 0x03` chỉ đi thẳng xuống epilogue, không làm gì.
- Điều kiện biên duy nhất: `RP_len == 0` → `_BoundErr(0)`. Nếu `RP_len >= 1`, hàm không kiểm tra thêm độ dài vì nó **không đọc field nào khác ngoài byte 0** — mọi thứ còn lại do callee tự kiểm.

### 2.4. Codec & API

| Thành phần | Vai trò ở OP này |
| :-- | :-- |
| `_BoundErr(0)` | Chặn `L=1` (RestPayload rỗng) — dòng 23 |
| `gvar_007D9CE4` | Con trỏ tới instance **TFightManage** (xem §2.6) |
| `FUN_00647164` | Callee nhánh SubOp 1 — **body ĐÃ có** từ 2026-09-14 (`functions/00647164_FUN_00647164.c`, 860 B) |
| `FUN_0064dd74` | Callee nhánh SubOp 2 — **body ĐÃ có** từ 2026-09-14 (`functions/0064dd74_FUN_0064dd74.c`, 462 B) |
| `FUN_0077ef7c` / `FUN_0077eb9c` / `FUN_0077f098` | **Không dùng** ở handler này (không parse DWORD/Word/double tại chỗ) |
| `TForm1_CY_AddSedQueue` (0x0051633c) | Hàm gửi C→S; chỉ xuất hiện ở chiều C→S (§6) |

### 2.5. Đối chiếu 1:1 giữa file handler và bản inline dispatcher

Bản inline `0078a89c_FUN_0078a89c.c:6674-6692`:

```c
case 0x32:
  iVar20 = 0; iVar21 = local_10; puStack_20 = &stack0xfffffffc;
  if (*(int *)(local_10 + -4) == 0) {                       // 6678: RP_len == 0
    in_stack_ffffffd4 = (code *)&UNK_007951ed;
    iVar20 = @BoundErr(0); iVar21 = extraout_EDX_x00164;    // 6681-6682
  }
  if (*(char *)(iVar21 + iVar20) == '\x01') {               // 6684: SubOp == 1
    in_stack_ffffffd4 = (code *)&UNK_00795213;
    func_0x00647164(*(undefined4 *)gvar_007D9CE4,local_10); // 6686
  }
  else if (*(char *)(iVar21 + iVar20) == '\x02') {          // 6688: SubOp == 2
    in_stack_ffffffd4 = (code *)&UNK_00795227;
    func_0x0064dd74(*(undefined4 *)gvar_007D9CE4,local_10); // 6690
  }
  break;                                                    // 6692
```

Kết luận so khớp: **logic 1:1** (cùng `BoundErr(0)`, cùng `SubOp=RP[0]`, cùng `1→00647164`, `2→0064dd74`, không default). Khác biệt chỉ là hình thức biên dịch:
- File handler lưu SubOp ra biến `+(-0x14)` rồi so sánh `int`; dispatcher so sánh trực tiếp `char`.
- Dispatcher chèn các marker `UNK_007951ed / UNK_00795213 / UNK_00795227` (địa chỉ unwind/SEH) mà file handler không thấy.
- File handler còn có epilogue dọn biến chuỗi cục bộ (dòng 33-61: `_LStrArrayClr` / `_LStrClr`); dispatcher dọn ở epilogue dùng chung nên không lặp trong case.

Phần epilogue dòng 33-61 (handler) là **code dọn AnsiString do compiler sinh**, KHÔNG phải logic wire — bỏ qua khi phân tích giao thức.

### 2.6. Định danh `gvar_007D9CE4` (đã xác minh)

`ts_decompile/functions/0050a4a0_TForm1.FormCreate.c:615-617`:

```c
piVar6 = TFightManage_Create((int *)VMT_63CCDC_TFightManage,'\x01',extraout_ECX_28);
*(int **)gvar_007D9CE4 = piVar6;
FUN_006449b4(*(int *)gvar_007D9CE4,0xffffffff);
```

→ `gvar_007D9CE4` là **ô biến toàn cục chứa con trỏ tới instance `TFightManage`** (VMT `VMT_63CCDC_TFightManage`). Đây là **định nghĩa trực tiếp, xác minh được** từ FormCreate.

**Xác minh được từ body mới (2026-09-14)**: `FUN_00647164`/`FUN_0064dd74` đúng là họ method `0x0064xxxx` làm việc trực tiếp trên battle-record `DAT_0098c63c` và nhận `Self` là instance lưu tại `gvar_007D9CE4` (`00647164_FUN_00647164.c:76,90`; `0064dd74_FUN_0064dd74.c:55,66`) — cùng đơn vị với `TFightManage.Create @ 0x00641f14` và chuỗi lỗi `"Error: FightManage.SupportRoleInfo"` (`00644294_FUN_00644294.c:104`). Suy luận "method của TFightManage" nay **khớp toàn bộ bằng chứng hành vi**, dù Ghidra vẫn chưa đặt tên chính thức.

OP dùng chung `gvar_007D9CE4` (cùng "họ FightManage"):
- Main OP 0x33 / Case 44: `case_044_0079522C_FUN_0079522c.c:28` → `func_0x0064cae0`.
- Main OP 0x35 / Case 46: `case_046_007952AB_FUN_007952ab.c:29-68` → nhiều `func_0x0064xxxx` (`cd74, cfec, d128, dfa4, e064, e3e0, e8f4, f9f0, faa8, fbd8, fd34`) cùng vài gvar khác (`gvar_007D9D34`, `gvar_007DA250`, `gvar_007DA0D0`).
- Ngoài ra OP 0x11 (Case 11: `0058D5D1...c:59-117`) và OP 0x16 (Case 19: `0078FEAF...c:130`) cũng gọi họ `0x0064xxxx` này.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[32][01]` + chuỗi record (xem §4.2) | `SubOp = RP[0]`; `RP` (gồm cả byte 0x01) truyền nguyên cho callee | `FUN_00647164(*gvar_007D9CE4, RP)` — reset cờ battle-record, flush FX, parse chuỗi record `[W][srcGrp][srcSlot][code][…][dstGrp][dstSlot][text]` và áp lên unit (body mới, §4.2) |
| `0x02` | `[32][02]` + chuỗi record 7 byte (xem §4.3) | `SubOp = RP[0]`; `RP` truyền nguyên cho callee | `FUN_0064dd74(*gvar_007D9CE4, RP)` — flush FX + cờ `self+0x1C`, parse/kiểm biên chuỗi record; **không thấy tiêu thụ field nào** (§4.3) |
| `0x00`, `0x03..0xFF` | bất kỳ | chỉ đọc `RP[0]` | **Im lặng — no-op** (không `default`, chỉ chạy epilogue) |
| — (biên) | `[32]` (L=1, RestPayload rỗng) | `*(RP-4)==0` | `_BoundErr(0)` → RangeError phía client |

Ghi chú: handler **không** gọi `FUN_0077eb9c`/`FUN_0077ef7c`/`FUN_0077f098`; **callee** `FUN_00647164`/`FUN_0064dd74` mới là nơi giải mã Word LE (qua `FUN_0077eb9c`) và cắt record (qua `_LStrCopy`) — xem §4.2/§4.3.

---

## 4. Chi tiết các nhánh chính (bỏ graphics/sound/animation)

### 4.1. Kiến trúc passthrough — điểm mấu chốt

Toàn bộ thân hàm chỉ gồm 3 việc:

1. Kiểm tra `RP_len == 0` → `BoundErr(0)` (dòng 22-25).
2. Đọc `SubOp = RP[0]` vào local (dòng 26).
3. Rẽ nhánh `SubOp == 1` / `SubOp == 2`, đẩy `(self, RP)` cho callee (dòng 27-32).

Handler **không** cắt chuỗi con, **không** đọc Word/DWORD/double, **không** ghi state, **không** gọi banner. Vì thế mọi thông tin protocol thực nằm ở 2 callee.

### 4.2. Nhánh `SubOp == 0x01` — `FUN_00647164` (body ĐÃ có — 2026-09-14)

- Call-site: `case_043_007951DA_FUN_007951da.c:28`; `0078a89c_FUN_0078a89c.c:6686`.
- File body: `ts_decompile/functions/00647164_FUN_00647164.c` (276 dòng, 860 byte, entry `00647164` trong `index.csv`) — **HOLE `0x00647162→0x00647524` đã được decompile**; call-site trong header file khớp đúng handler case 43 (`00647164_FUN_00647164.c:8,13`).
- Chữ ký: `void __register FUN_00647164(int param_1, int param_2)` — `param_1` = Self `TFightManage` (EAX), `param_2` = RestPayload (EDX, AnsiString; `local_8 = param_1` dòng 76).
- **Việc mở đầu** (dòng 84-90), tác động thẳng lên battle-record `DAT_0098c63c` (gọi tắt `battle`):
  - `*(byte*)(battle+0xE5B) = 0xFF`, `*(byte*)(battle+0xE68) = 0xFF`, `*(byte*)(battle+6) = 0` (dòng 84-86);
  - `FUN_00651824(battle)` — **flush 21 khe FX** tại `battle+0x1AC` (cùng helper với SubOp 3 OP 0x35; `00651824_FUN_00651824.c` deref tham số ngay, không guard null);
  - `*(byte*)(self+0x1C) = 1` (dòng 90).
  - ⚠️ **Không có guard `battle != 0`** (khác OP 0x35 SubOp 3): gửi SubOp 1 **ngoài trận** (pointer null) → deref `0x0+0xE5B` / `FUN_00651824(0)` → **EAccessViolation**. Chỉ an toàn khi `DAT_0098C63C ≠ 0`.
- **Wire layout từng record** (vòng lặp `pos` Delphi 1-based bắt đầu tại 2, điều kiện `2 < len(RP)` — dòng 92-93; stride = `W + 2` — dòng 186-194). Quy ra 0-based `RP[0] = SubOp`:

```
RP[1..2]   W      Word LE                    — độ dài phần thân record (tính từ RP[3])
RP[3]      srcGrp byte (BoundErr nếu > 3)
RP[4]      srcSlot byte (BoundErr nếu > 4)
RP[5..6]   code   Word LE
RP[7]      b2     byte (đọc dòng 138, truyền arg 4 ở dưới)
RP[8]      b3     byte (đọc dòng 149, truyền arg 3 FUN_00651e3c)
RP[9]      dstGrp byte (BoundErr nếu > 3)
RP[10]     dstSlot byte (BoundErr nếu > 4)
RP[9 .. W+2] text AnsiString — _LStrCopy(RP, pos+8, W-6) (dòng 185)
             lưu ý: code COPY BẮT ĐẦU từ chính byte dstGrp, tức 2 byte
             dstGrp/dstSlot nằm chồng lên đầu chuỗi text — hành vi nguyên văn của body.
```

- **Xử lý mỗi record** (dòng 195-262):
  1. `u = FUN_0065178c(battle, srcGrp, srcSlot)` — trả byte `cell+0x4C` (unit index) hoặc `0xFFFFFFFF` nếu ô trống (`0065178c_FUN_0065178c.c:68-70`; ô = `battle + grp*0x46 + slot*14 + 0x48`).
  2. Nếu tìm thấy: `unit = *(int*)(battle + 0x158 + u*4)` (mảng 21 con trỏ unit — dòng 201; cùng cơ chế với OP 0x35).
  3. Nếu `code == 10000`: remap theo id unit `*(unit+4)`: `0x9090/0x9091 → 0x3E81 (16001)`; `0xA4B0/0xA47B/0xA47C/0xA4BA/0xA4C8 → 0x3E82 (16002)` (dòng 202-214).
  4. `FUN_00651e3c(unit, code, b3, text)` (dòng 216) — **vẫn KHÔNG có body** (không có file `00651e3c_*`) ⇒ tác động cụ thể của `code`/`text` lên unit: **chưa kết luận được**.
  5. `FUN_00648154(self, unitIdx(src cell), unitIdx(dst cell), b2, code)` (dòng 217-262) — `FUN_00648154` đã biết ở `opcode_35.md` (consumer FX/animation của `DAT_0098C63C`).
- **Tổng nghĩa theo code**: luồng record "**unit tại ô (srcGrp,srcSlot) thực hiện hành động `code` kèm chuỗi `text`, có liên kết tới ô (dstGrp,dstSlot)**" — cùng họ biến cố trận với OP 0x35 (lưới 4 nhóm × 5 ô).

### 4.3. Nhánh `SubOp == 0x02` — `FUN_0064dd74` (body ĐÃ có — 2026-09-14)

- Call-site: `case_043_007951DA_FUN_007951da.c:31`; `0078a89c_FUN_0078a89c.c:6690`.
- File body: `ts_decompile/functions/0064dd74_FUN_0064dd74.c` (159 dòng, 462 byte) — khe trống `0x0064dc44 → 0x0064ea20` đã decompile.
- **Việc mở đầu** giống hệt §4.2 trừ 3 byte cờ battle: `FUN_00651824(battle)` flush FX + `*(byte*)(self+0x1C) = 1` (dòng 65-66); cũng **không guard `battle != 0`** → cùng ràng buộc "chỉ trong trận".
- **Vòng record** (dòng 69-147), `pos` 1-based bắt đầu 2, stride = `n*8 + 5` (dòng 138-143), với mỗi record 7 byte đầu:

```
RP[1] grp (tra ô), RP[2] slot, RP[3..4] Word LE (decode, không dùng),
RP[5] n (byte điều khiển stride), RP[6], RP[7] (đọc + kiểm biên, không dùng)
```

  - Mỗi record: `u = FUN_0065178c(battle, grp, slot)`; nếu tìm thấy, `unit = *(battle+0x158+u*4)` được lưu `local_24` (dòng 130-137) nhưng **không được dùng trong bất kỳ lời gọi/ghi nào** — dead store; Word `RP[3..4]` và `RP[6..7]` cũng chỉ đọc + BoundErr.
- **Đối chiếu asm** (`0064dd74_FUN_0064dd74.asm.txt`): danh sách `CALL` trong asm chỉ gồm `FUN_00651824` (dòng 27), `FUN_0065178c` (dòng 113) và các helper RTL chuỗi (`0x00402fa8/0x00402fb0/0x00403df8/0x00404088/0x0040423c/0x00404290`) — **xác nhận C không bị mất call nghiệp vụ**; Ghidra có cảnh báo `Removing unreachable block (ram,0x0064def5)` (`.c:16`) là code chết.
- **Hiệu ứng quan sát được của SubOp 2**: flush FX + đặt cờ `self+0x1C` + parse/kiểm biên chuỗi record. **Ý nghĩa nghiệp vụ từng field record: chưa kết luận được** (field được server gửi nhưng client thân này không tiêu thụ — có thể dành cho phiên bản logic khác hoặc bị loại bỏ).

### 4.4. Nhánh SubOp khác & biên

- `SubOp == 0x00` hoặc `>= 0x03`: rơi xuống cuối khối `if/else if`, không có nhánh xử lý → **im lặng**.
- `RP_len == 0` (payload chỉ có `[32]`): `_BoundErr(0)` → client ném RangeError. Mock server **không được** gửi frame kiểu này.
- Với mọi SubOp hợp lệ, handler **không** tự đọc thêm byte, nên không có nguy cơ BoundErr do thiếu field ở tầng này — rủi ro nằm trong callee.

---

## 5. Chuỗi VISCII → UTF-8

- Trong đường đã xác minh (handler `case_043` + bản inline `0078a89c`:6674-6692), **không có bất kỳ chuỗi literal nào** và **không có lời gọi `_LStrFromChar`/`_LStrCatN`/`_LStrLAsg`**. Các lời gọi `_LStrArrayClr`/`_LStrClr` (handler dòng 36-60) chỉ là **dọn biến AnsiString cục bộ do compiler sinh**, không mang dữ liệu.
- Kết luận: **Không có VISCII cần decode trong path OP 0x32 S→C.** Hai callee mới (`00647164_FUN_00647164.c`, `0064dd74_FUN_0064dd74.c`) **không chứa literal chuỗi tĩnh nào** — payload chỉ được cắt bằng `_LStrCopy` rồi truyền vào `FUN_00651e3c(unit, code, b3, text)` dưới dạng **binary blob**; nội dung `text` có phải văn bản VISCII hay không phụ thuộc body `FUN_00651e3c` (vẫn chưa có trong SSOT — `index.csv` không có entry `00651e3c`).

---

## 6. Chiều Client → Server

### 6.1. Bản C của `case 0x32` (bị cắt cụt) — dẫn chứng

`ts_decompile/functions/0077f414_FUN_0077F414.c:986-1006` (hàm `FUN_0077f414`, tên "SendCommand" theo handoff; SSOT chỉ có `FUN_0077f414`):

```c
986:    case 0x2e:
987:      break;
988:    case 0x32:
989:      local_24 = &stack0xfffffffc;
990:      uVar4 = FUN_00402cf0(0xdc);
991:      uVar7 = uVar4 == 0xff;
992:      if (0xff < uVar4) { _BoundErr(uVar4); }
995:      _LStrCmp(*(uint **)(*(int *)gvar_007DA37C + 0x24),*(uint **)(*(int *)gvar_007DA37C + 0x28));
996:      if ((bool)uVar7) {
997:        iVar5 = FUN_00402cf0(0x38);
998:        uVar4 = iVar5 + 200;              /* +0xc8 */
...
1005:      }
1006:      break;
```

Quan sát trực tiếp: khối này **không** gọi `TForm1_CY_AddSedQueue`, **không** build chuỗi, và có một `if ((bool)uVar7)` gần như không bao giờ đúng (`uVar4` là `Random(0xdc)` nên luôn `< 0xdc < 0xff`).

**Kiểm tra lại 2026-09-14 (bản re-export)**: `0077f414_FUN_0077F414.c` nay dài 1208 dòng; `case 0x32:` vẫn nằm ở dòng 988 và **vẫn kết thúc bằng `break;` tại dòng 1006** — không một lần xuất hiện `0051633c`/`TForm1_CY_AddSedQueue`/hằng `0x158` trong toàn file C (grep: các `CY_AddSedQueue` gần nhất là dòng 1010/1018, thuộc `case 0x36`/`case 0x37`). ⇒ **Mâu thuẫn C/asm ĐÃ được xác nhận là lỗi cắt cụt của bản C, không phải hành vi**: phần builder thật của case 0x32 **vẫn chỉ có trong asm**.

### 6.2. Bản asm cùng hàm (dẫn chứng ngược lại bản C)

Trong `ts_decompile/functions/0077f414_FUN_0077F414.asm.txt` (không phải file khác):

- **Preamble của case 0x32** (dòng 3255-3283):
  - `3255-3257`: `MOV EAX,0xdc; CALL 0x00402cf0; CMP EAX,0xff`
  - `3258`: `JBE 0x00788be0` (nếu `<= 0xff` thì bỏ qua error-path)
  - `3261-3267`: nạp `[0x007da37c]+0x24` và `+0x28`, gọi `CALL 0x00404198` (= `_LStrCmp`)
  - `3268`: `JNZ 0x00788c24`
  - `3269-3277`: `MOV EAX,0x38; CALL 0x00402cf0; ADD EAX,0xc8; ... CMP EAX,0xff` (random `0x38` + 200, kèm kiểm biên)
  - `3278-3283`: **dispatch phụ trên byte tham số thứ hai** `[EBP-0x6]`:
    ```
    3278: MOV AL,byte ptr [EBP + -0x6]
    3279: DEC AL
    3280: JZ 0x00788c38      ; [EBP-0x6] == 1
    3281: DEC AL
    3282: JZ 0x00788e7a      ; [EBP-0x6] == 2
    3283: JMP 0x0078a4f2      ; khác → kết thúc (no-op)
    ```
- **Nhánh con `== 1`** (`0x00788c38`, asm 3284-3418): dựng AnsiString bằng `CALL 0x00402b90`, `CALL 0x00402b60` *(kiểm tra lại 2026-09-14: `0x00402b60` nay là **`@PStrNCat`** với body trong `functions/00402b60__PStrNCat.c` — HOLE `0x00402B1C–0x00402B90` đã resolved; chữ ký `@PStrNCat(dest, src, maxLen)` nối shortstring có cắt gọn theo `maxLen` — dòng 584-606 của file)*; đọc bảng actor `[0x007da51c] + 0x158 + idx*4` với `idx = MOVSX byte [obj+0xe78]`/`[obj+0xeaa]` (kiểm biên `0x14`) rồi lấy `+0x56e/+0x56f/+0x5a8`; cuối cùng `CALL 0x0051633c` (= **`TForm1.CY_AddSedQueue`**, asm dòng **3417**) rồi `JMP 0x0078a4f2`.
- **Nhánh con `== 2`** (`0x00788e7a`, asm 3419-3552): cấu trúc tương tự (dùng `+0x5aa` thay `+0x5a8`), kết thúc bằng `CALL 0x0051633c` (asm dòng **3551**) rồi `JMP 0x0078a4f2`.
- Cả hai nhánh con còn gọi lại `FUN_00402cf0` để sinh byte ngẫu nhiên đưa vào packet:
  - `3399-3400`: `MOV AL,byte ptr [EBP + -0x19]; CALL 0x00402cf0`
  - `3405-3406`: `MOV EAX,0x100; CALL 0x00402cf0`
  (tương tự ở nhánh 2 dòng 3533-3534 và 3539-3540).

### 6.3. `FUN_00402cf0` là gì (body CÓ trong SSOT)

`ts_decompile/functions/00402cf0_FUN_00402cf0.c` — 767 dòng, **body thật nằm ở dòng 760-765**:

```c
undefined4 FUN_00402cf0(uint param_1)
{
  DAT_007db044 = DAT_007db044 * 0x8088405 + 1;
  return (int)((ulonglong)param_1 * (ulonglong)DAT_007db044 >> 0x20);
}
```

- **Chữ ký**: `undefined4 __register FUN_00402cf0(uint param_1)`, entry `00402cf0`, size 22 bytes (header dòng 2-5 của file).
- **Bản chất**: PRNG kiểu **LCG nhân** (multiplier `0x8088405`, seed `DAT_007db044`), trả về `floor(param_1 * seed / 2^32)` → **số nguyên ngẫu nhiên trong `[0, param_1)`**. Tương đương `Random(param_1)` của Delphi.
- Vì vậy `FUN_00402cf0(0xdc)` ∈ `[0, 0xdb]` → điều kiện `0xff < uVar4` **luôn sai**, và `uVar7 = (uVar4 == 0xff)` **luôn false**. Đây là cơ sở để kết luận `if ((bool)uVar7)` trong bản C là **biểu diễn sai của bộ decompile**.

### 6.4. Kết luận chiều C→S (dán nhãn rõ)

- **XÁC MINH ĐƯỢC (từ asm SSOT `0077f414_FUN_0077F414.asm.txt`)**:
  1. `case 0x32` C→S **là một builder thật**, không phải fall-through rỗng: sau preamble random + `_LStrCmp`, hàm dispatch tiếp theo tham số phụ `[EBP-0x6]` với đúng 2 nhánh `1` và `2`, và **cả 2 nhánh đều gọi `TForm1.CY_AddSedQueue` (0x0051633c)** (asm 3417, 3551).
  2. Việc "không thấy `TForm1_CY_AddSedQueue`" trong khối C dòng 988-1006 là do **bộ decompile cắt mất phần sau của case** (không phải bằng chứng branch rỗng). Toàn bộ vùng `gvar_007DA51C` / `+0x158` / `+0xe78` / `+0x56e` (asm 3303-3551) **không xuất hiện một lần nào trong file C** `0077f414_FUN_0077F414.c` → xác nhận decompile thiếu.
  3. `SubOp` phụ của C→S 0x32 chỉ nhận `1` và `2`, **đối xứng đúng** với SubOp `1/2` của handler S→C `FUN_007951da`. Giá trị khác → kết thúc im lặng (asm 3283).
  4. **Re-check 2026-09-14 trên asm re-export**: dispatch hai nhánh xác nhận nguyên văn — `MOV AL,[EBP-0x6]; DEC AL; JZ 0x00788c38; DEC AL; JZ 0x00788e7a; JMP 0x0078a4f2` tại `0077f414_FUN_0077F414.asm.txt:3278-3283` (JZ lần lượt dòng 3280/3282), và `CALL 0x0051633c` vẫn ở đúng dòng 3417/3551. Trong vùng 3284-3552 còn có: `CALL 0x0077eb1c` (2 lần, từ dòng 3396), `CALL 0x00403fa0` (5 lần, từ 3403), `CALL 0x00404148` (2 lần, từ 3413) — **xác thực chuỗi builder đã nêu ở §7**.
- **CHƯA XÁC MINH ĐƯỢC**:
  1. Ý nghĩa ngữ nghĩa các field trong packet C→S 0x32 (các byte `0x01`, `[EBP-0x5]`, `[EBP-0x6]`, chỉ số bảng actor, `+0x56e/+0x56f/+0x5a8/+0x5aa`, các byte random). Bảng dispatch `0x77f474`/`0x77f53c` **không có trong `redump/`**, nên việc gán "vùng asm 0x00788bd9 = opcode 0x32" dựa vào **nhãn `case 0x32` của bộ decompile + preamble trùng khớp tuyệt đối**; chưa đối chiếu độc lập bằng hex.
  2. Tên/class thực của `gvar_007DA51C` — chỉ biết nó là object có mảng `+0x158` (index `0..0x14` = 0..20) và các cờ `+0xe78`, `+0xeaa`; **nhãn "actor/skill" là suy luận**. *Bổ sung 2026-09-14*: các field builder đọc (`unit+0x56e`, `unit+0x56f`) đúng bằng các field mà **OP 0x35 SubOp 7** ghi khi chuyển ô (`*(unit+0x56e)=dstGrp`, `*(unit+0x56f)=dstSlot` — `0064e3e0_FUN_0064e3e0.c:313,318`) ⇒ hệ số nhất quán: phần tử mảng `+0x158` của `[0x007DA51C]` là **unit trận đấu** (cùng layout với mảng `battle+0x158` ở §4.2) — vẫn chưa có tên class chính thức.
  3. Việc C→S "thật sự" gửi opcode nào trên wire (byte đầu builder là `0x01` + `[EBP-0x5]`, chưa rõ `TForm1_CY_AddSedQueue` có prepend header/opcode hay không) — **unknown**.

---

## 7. Ghi chú cho Mock Server

```
S→C:
[32][01][W lo][W hi][srcGrp][srcSlot][code lo][code hi][b2][b3][dstGrp][dstSlot][text...]  × N record
        → FUN_00647164(selfFightManage, RP)  (reset cờ battle-record, flush FX, áp hành động lên unit)
[32][02][grp][slot][w lo][w hi][n][x][y]  × N record   → FUN_0064dd74(selfFightManage, RP)
        (chỉ flush FX + cờ self+0x1C; field record không tiêu thụ — §4.3)
[32][00] / [32][03..FF]     → no-op im lặng
ĐỪNG GỬI: [32] (L=1, RestPayload rỗng) → BoundErr(0) phía client.
ĐỪNG GỬI SubOp 1/2 NGOÀI TRẬN (DAT_0098C63C = 0): cả hai callee deref battle-record
không guard → EAccessViolation.
```

- **Update 2026-09-14**: SubOp 1/2 **đã mock được** theo layout §4.2/§4.3 (ghi chú: `text` trong SubOp 1 bắt đầu chồng lên 2 byte dstGrp/dstSlot, độ dài `W-6`; payload `[32][01]` hoặc `[32][02]` một mình (L=2) chỉ chạy phần mở đầu, không record nào). Tác động cuối của SubOp 1 lên unit còn đi qua `FUN_00651e3c` (**chưa có body**) nên hiệu ứng hiển thị chính xác vẫn cần theo dõi live.
- C→S 0x32 đúng là builder thật (asm), gồm preamble random + dispatch `[EBP-0x6] ∈ {1,2}` → gửi 2 loại packet khác nhau, cả hai kết thúc bằng `CY_AddSedQueue`.
- `Số nguyên/Word` trong builder C→S dùng `FUN_0077eb1c` (Word→2B LE), `0x00403fa0` và `0x00404148`. **Đính chính 2026-09-14**: hai helper này đã được index.csv đặt tên RTL — `0x00403fa0 = @LStrFromChar` (biến **1 ký tự** (Char trong EAX) thành AnsiString, qua `_LStrFromPCharLen(dest, &ch, 1)` — `00403fa0__LStrFromChar.c:308-314`; KHÔNG phải "Int→str" như bản cũ suy đoán), và `0x00404148 = @LStrCatN` (ghép `Count` AnsiString, **duyệt tham số ngược từ cuối về đầu** ⇒ nối NGƯỢC thứ tự liệt kê, xác nhận quy ước đã hiệu chuẩn ở `opcode_06.md` §5.1 — `00404148__LStrCatN.c:714-744`). Int→str thật sự là helper `IntToStr` (thấy ở nơi khác, vd `0076a944_FUN_0076a944.c:79`), không xuất hiện trong 2 hàm vừa nêu. Hai helper `0x00402fa8`/`0x00402fb0` (CALL kề sau `Random` trong builder) vẫn **chưa có body** — chưa kết luận được.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_043_007951DA_FUN_007951da.c` (64 dòng) | Handler chính: BoundErr + SubOp 1/2 + epilogue |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6674-6692` | Bản inline dispatcher (đối chiếu 1:1) |
| 3 | `case_functions/jumptable_0x78A9B6_cases.c:8052-8113` | Bản gộp case 43 |
| 4 | `redump/jumptable_byte200_0x78A8EE.hex:4` (`[0x32]=0x2B`) + `redump/jumptable_dword200_0x78A9B6.hex:11` + `.csv:45` + `manifest.csv:45` | Mapping Main OP → Case → Target |
| 5 | `functions/0050a4a0_TForm1.FormCreate.c:615-617` | Định danh `gvar_007D9CE4` = TFightManage |
| 6 | `case_functions/functions/case_044_0079522C_FUN_0079522c.c:28`, `case_046_007952AB_FUN_007952ab.c:29-68` | OP 0x33/0x35 dùng chung TFightManage (họ FightManage) |
| 7 | ~~grep `00647164` / `0064dd74` toàn `ts_decompile/`~~ | *(lỗi thời — 2026-09-14: hai callee đã có body, xem dòng 12-15)* |
| 8 | `functions/0077f414_FUN_0077F414.c:988-1006` | Bản C chiều C→S (**vẫn cắt cụt** sau re-export — kiểm lại 2026-09-14) |
| 9 | `functions/0077f414_FUN_0077F414.asm.txt:3255-3552` | Bản asm chiều C→S (builder thật + `CY_AddSedQueue` 3417/3551; dispatch `[EBP-0x6]` 3278-3283) |
| 10 | `functions/00402cf0_FUN_00402cf0.c:760-765` | Body `FUN_00402cf0` = LCG Random |
| 11 | `functions/0051633c_TForm1.CY_AddSedQueue.c` (index.csv:2815) | Hàm gửi C→S (`0x0051633c`) |
| 12 | `functions/00647164_FUN_00647164.c` (276 dòng) | Body SubOp 1: reset battle-flag, flush FX, record `[W][srcGrp][srcSlot][code][b2][b3][dstGrp][dstSlot][text]`, `FUN_00651e3c` + `FUN_00648154` |
| 13 | `functions/0064dd74_FUN_0064dd74.c` (159 dòng) + `.asm.txt` (danh sách CALL) | Body SubOp 2: flush FX + parse record 7B, stride `n*8+5`, field không tiêu thụ |
| 14 | `functions/00403fa0__LStrFromChar.c:308-314`, `functions/00404148__LStrCatN.c:714-744`, `functions/00402b60__PStrNCat.c:584-606` | RTL confirm cho chuỗi builder C→S 0x32 (`@LStrFromChar`, `@LStrCatN` nối ngược, `@PStrNCat` maxLen) |
| 15 | `functions/0065178c_FUN_0065178c.c:50-77`; `functions/0064e3e0_FUN_0064e3e0.c:313,318` | Tra ô → unitIdx `cell+0x4C`; đối chiếu field `+0x56e/+0x56f` với OP 0x35 SubOp 7 |
