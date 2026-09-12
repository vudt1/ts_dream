# PHÂN TÍCH — Main OP 0x32 (50) / Case 43 / FUN_007951da @ 0x007951DA

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)** (kèm khảo sát đối chiếu C→S)
Trạng thái: **Đã xác minh phần khung + dispatch từ mã sơ cấp** (`ts_decompile/` only). Handler chỉ là lớp passthrough 2 nhánh `SubOp 1/2`; **toàn bộ ruột logic nằm trong 2 callee KHÔNG có body trong SSOT**.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh `S→C` mỏng nhất trong nhóm — handler **không tự parse bất kỳ field nào**. Nó chỉ đọc `SubOp = RP[0]`, rồi chuyển nguyên con trỏ RestPayload cho 1 trong 2 hàm thuộc **TFightManage** (`gvar_007D9CE4`):
  - `SubOp == 1` → `func_0x00647164(*gvar_007D9CE4, RP)`
  - `SubOp == 2` → `func_0x0064dd74(*gvar_007D9CE4, RP)`
  - Tất cả SubOp khác → **im lặng (no-op)**, không có `default`.
- **Hệ quả cho reverse-engineering**: layout field của payload **không thể suy ra từ SSOT hiện tại**, vì bị đẩy xuống 2 callee chưa decompile. Cần redump `0x00647164` và `0x0064dd74` để có bảng field.
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
| `func_0x00647164` | Callee nhánh SubOp 1 — **không có body trong SSOT** |
| `func_0x0064dd74` | Callee nhánh SubOp 2 — **không có body trong SSOT** |
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

**Suy luận (chưa xác minh bằng tên hàm)**: các hàm được gọi trên `*gvar_007D9CE4` đều nằm dải `0x0064xxxx` — cùng dải với `TFightManage.Create @ 0x00641f14` và phương thức có chuỗi lỗi `"Error: FightManage.SupportRoleInfo"` (`ts_decompile/functions/00644294_FUN_00644294.c:104`). Do đó `func_0x00647164` (0x00647164) và `func_0x0064dd74` (0x0064dd74) **rất có thể là method của TFightManage**, nhưng SSOT không có tên chính thức → chỉ dừng ở mức suy luận.

OP dùng chung `gvar_007D9CE4` (cùng "họ FightManage"):
- Main OP 0x33 / Case 44: `case_044_0079522C_FUN_0079522c.c:28` → `func_0x0064cae0`.
- Main OP 0x35 / Case 46: `case_046_007952AB_FUN_007952ab.c:29-68` → nhiều `func_0x0064xxxx` (`cd74, cfec, d128, dfa4, e064, e3e0, e8f4, f9f0, faa8, fbd8, fd34`) cùng vài gvar khác (`gvar_007D9D34`, `gvar_007DA250`, `gvar_007DA0D0`).
- Ngoài ra OP 0x11 (Case 11: `0058D5D1...c:59-117`) và OP 0x16 (Case 19: `0078FEAF...c:130`) cũng gọi họ `0x0064xxxx` này.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[32][01][...blob...]` (≥2B) | `SubOp = RP[0]`; `RP` (gồm cả byte 0x01) truyền nguyên cho callee | `func_0x00647164(*gvar_007D9CE4, RP)` — **body KHÔNG có trong SSOT (chưa decompile)**; layout field **chưa xác định** |
| `0x02` | `[32][02][...blob...]` (≥2B) | `SubOp = RP[0]`; `RP` truyền nguyên cho callee | `func_0x0064dd74(*gvar_007D9CE4, RP)` — **body KHÔNG có trong SSOT (chưa decompile)**; layout field **chưa xác định** |
| `0x00`, `0x03..0xFF` | bất kỳ | chỉ đọc `RP[0]` | **Im lặng — no-op** (không `default`, chỉ chạy epilogue) |
| — (biên) | `[32]` (L=1, RestPayload rỗng) | `*(RP-4)==0` | `_BoundErr(0)` → RangeError phía client |

Ghi chú: handler **không** gọi `FUN_0077eb9c`/`FUN_0077ef7c`/`FUN_0077f098`; nghĩa là với OP này SSOT **không cho biết** kiểu dữ liệu của bất kỳ field nào sau SubOp.

---

## 4. Chi tiết các nhánh chính (bỏ graphics/sound/animation)

### 4.1. Kiến trúc passthrough — điểm mấu chốt

Toàn bộ thân hàm chỉ gồm 3 việc:

1. Kiểm tra `RP_len == 0` → `BoundErr(0)` (dòng 22-25).
2. Đọc `SubOp = RP[0]` vào local (dòng 26).
3. Rẽ nhánh `SubOp == 1` / `SubOp == 2`, đẩy `(self, RP)` cho callee (dòng 27-32).

Handler **không** cắt chuỗi con, **không** đọc Word/DWORD/double, **không** ghi state, **không** gọi banner. Vì thế mọi thông tin protocol thực nằm ở 2 callee.

### 4.2. Nhánh `SubOp == 0x01` — `func_0x00647164`

- Call-site: `case_043_007951DA_FUN_007951da.c:28`; `0078a89c_FUN_0078a89c.c:6686`.
- Tham số: `*(undefined4*)gvar_007D9CE4` (instance TFightManage) + `RP` (con trỏ RestPayload, gồm cả byte `0x01`).
- **Nội bộ: KHÔNG có body trong SSOT (chưa decompile).** Đã grep toàn bộ `ts_decompile/` cho chuỗi `00647164`: chỉ có 2 call-site (case file + bản inline dispatcher) và 2 dòng trong bản gộp `jumptable_0x78A9B6_cases.c:8079/8082`; **không có** file `ts_decompile/functions/00647164_*`. `grep` trong `ts_decompile/index.csv` cũng không có entry `00647164`.
- Bằng chứng bổ sung về "lỗ hổng dump": `index.csv` có `FUN_00647140` (`index.csv:4269`, size 34 bytes → kết thúc `0x00647162`), entry kế tiếp là `FUN_00647524` (`index.csv:4270`); `0x00647164` nằm ngay sau `0x00647162` trong khe trống `0x00647162 → 0x00647524` → **chưa được decompile**.
- → Toàn bộ logic bên trong (ý nghĩa SubOp 1, field payload) là **unknown / cần redump**.

### 4.3. Nhánh `SubOp == 0x02` — `func_0x0064dd74`

- Call-site: `case_043_007951DA_FUN_007951da.c:31`; `0078a89c_FUN_0078a89c.c:6690`.
- Tham số y hệt nhánh 1.
- **Nội bộ: KHÔNG có body trong SSOT (chưa decompile).** Trong `index.csv`, `FUN_0064d438` (`index.csv:4292`, size 2060 bytes → kết thúc `0x0064dc44`) là entry được dump gần nhất trước `0x0064dd74`; entry kế tiếp là `FUN_0064ea20` (`index.csv:4293`). `0x0064dd74` nằm trong khe trống `0x0064dc44 → 0x0064ea20` → chưa decompile.
- → Logic bên trong **unknown / cần redump**.

### 4.4. Nhánh SubOp khác & biên

- `SubOp == 0x00` hoặc `>= 0x03`: rơi xuống cuối khối `if/else if`, không có nhánh xử lý → **im lặng**.
- `RP_len == 0` (payload chỉ có `[32]`): `_BoundErr(0)` → client ném RangeError. Mock server **không được** gửi frame kiểu này.
- Với mọi SubOp hợp lệ, handler **không** tự đọc thêm byte, nên không có nguy cơ BoundErr do thiếu field ở tầng này — rủi ro nằm trong callee.

---

## 5. Chuỗi VISCII → UTF-8

- Trong đường đã xác minh (handler `case_043` + bản inline `0078a89c`:6674-6692), **không có bất kỳ chuỗi literal nào** và **không có lời gọi `_LStrFromChar`/`_LStrCatN`/`_LStrLAsg`**. Các lời gọi `_LStrArrayClr`/`_LStrClr` (handler dòng 36-60) chỉ là **dọn biến AnsiString cục bộ do compiler sinh**, không mang dữ liệu.
- Kết luận: **Không có VISCII cần decode trong path OP 0x32 S→C từ SSOT hiện tại.** Nếu callee `func_0x00647164` / `func_0x0064dd74` (chưa decompile) có string literal, phải redump mới xác định — hiện chưa có `redump/lit_0064*.hex` trong `ts_decompile/redump/`.

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
- **Nhánh con `== 1`** (`0x00788c38`, asm 3284-3418): dựng AnsiString bằng `CALL 0x00402b90`, `CALL 0x00402b60`; đọc bảng actor `[0x007da51c] + 0x158 + idx*4` với `idx = MOVSX byte [obj+0xe78]`/`[obj+0xeaa]` (kiểm biên `0x14`) rồi lấy `+0x56e/+0x56f/+0x5a8`; cuối cùng `CALL 0x0051633c` (= **`TForm1.CY_AddSedQueue`**, asm dòng **3417**) rồi `JMP 0x0078a4f2`.
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
- **CHƯA XÁC MINH ĐƯỢC**:
  1. Ý nghĩa ngữ nghĩa các field trong packet C→S 0x32 (các byte `0x01`, `[EBP-0x5]`, `[EBP-0x6]`, chỉ số bảng actor, `+0x56e/+0x56f/+0x5a8/+0x5aa`, các byte random). Bảng dispatch `0x77f474`/`0x77f53c` **không có trong `redump/`**, nên việc gán "vùng asm 0x00788bd9 = opcode 0x32" dựa vào **nhãn `case 0x32` của bộ decompile + preamble trùng khớp tuyệt đối**; chưa đối chiếu độc lập bằng hex.
  2. Tên/class thực của `gvar_007DA51C` — chỉ biết nó là object có mảng `+0x158` (index `0..0x14` = 0..20) và các cờ `+0xe78`, `+0xeaa`; **nhãn "actor/skill" là suy luận**.
  3. Việc C→S "thật sự" gửi opcode nào trên wire (byte đầu builder là `0x01` + `[EBP-0x5]`, chưa rõ `TForm1_CY_AddSedQueue` có prepend header/opcode hay không) — **unknown**.

---

## 7. Ghi chú cho Mock Server

```
S→C:
[32][01][...blob...]   ≥2B → func_0x00647164(selfFightManage, RP)   [body KHÔNG có trong SSOT]
[32][02][...blob...]   ≥2B → func_0x0064dd74(selfFightManage, RP)   [body KHÔNG có trong SSOT]
[32][00] / [32][03..FF]     → no-op im lặng
ĐỪNG GỬI: [32] (L=1, RestPayload rỗng) → BoundErr(0) phía client.
```

- Không thể build payload đúng nghĩa cho SubOp 1/2 cho tới khi redump `0x00647164` và `0x0064dd74`. Test an toàn hiện tại: `[32][00]` (no-op) hoặc `[32][03]`.
- C→S 0x32 đúng là builder thật (asm), gồm preamble random + dispatch `[EBP-0x6] ∈ {1,2}` → gửi 2 loại packet khác nhau; nhưng field layout cụ thể chưa giải mã được từ C (bị cắt) và cần đọc asm/bảng actor để hoàn thiện.
- `Số nguyên/Word` trong builder C→S dùng `FUN_0077eb1c` (Word→2B LE) và `FUN_00403fa0` (Int→str) / `FUN_00404148` (ghép mảng) — không phải codec của nhánh S→C (vì nhánh S→C không parse field).

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
| 7 | grep `00647164` / `0064dd74` toàn `ts_decompile/` + `index.csv` | Xác nhận callee KHÔNG có body (chỉ call-site) |
| 8 | `functions/0077f414_FUN_0077F414.c:988-1006` | Bản C chiều C→S (bị cắt) |
| 9 | `functions/0077f414_FUN_0077F414.asm.txt:3255-3552` | Bản asm chiều C→S (builder thật + `CY_AddSedQueue` 3417/3551) |
| 10 | `functions/00402cf0_FUN_00402cf0.c:760-765` | Body `FUN_00402cf0` = LCG Random |
| 11 | `functions/0051633c_TForm1.CY_AddSedQueue.c` (index.csv:2815) | Hàm gửi C→S (`0x0051633c`) |
