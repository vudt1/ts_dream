# PHÂN TÍCH — Main OP 0x38 (56) / Case 49 / FUN_00795555 @ 0x00795555

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh 100% phần vỏ (framing / dispatch / SubOp) từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler là một **no-op tuyệt đối** cho mọi SubOp: sau khi đọc `SubOp = RP[0]` nó **không hề dùng** giá trị này (không `switch`, không `if`, không truy cập toàn cục, không gọi callee). Hiệu ứng duy nhất trên wire là **`_BoundErr(0)` khi `RestPayload` rỗng** (payload `[38]`, L=1). **Không có unknown/needs-redump nào ở tầng handler.**

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò: PLACEHOLDER / NO-OP.** Main OP 0x38 đọc 1 byte `SubOp` từ `RestPayload`, cất vào biến cục bộ `unaff_EBP + -0x14`, rồi **rơi thẳng xuống epilogue dọn chuỗi** và `return`. Byte `SubOp` **không được đọc lại** ở bất kỳ đâu trong hàm → **mọi giá trị `0x00..0xFF` đều im lặng bỏ qua**.
- **Không có nhánh nghiệp vụ nào**: `case_049_00795555_FUN_00795555.c` **không chứa `switch`, không `if/else if` trên SubOp, không `default`, không truy cập `gvar_*`, không gọi hàm con** (đã kiểm tra: file không có `(**(code`, `switch`, `default`). Đây là **hàm lá (leaf) thuần trừ `_BoundErr`**.
- **Không có banner / chuỗi / VISCII** trên đường đã xác minh: handler không tham chiếu `&DAT_` / `&UNK_` literal; chỉ có `&LAB_00796408` (nhãn unwind stack, dòng 28) và các immediate `0x796364..0x7963fd` (địa chỉ return của thunk `_LStrClr`/`_LStrArrayClr`) — **tất cả là địa chỉ code, không phải dữ liệu chuỗi**.
- **Chốt chặn lỗi duy nhất**: nếu `RestPayload` rỗng (`*(int*)(RP-4) == 0`, tức L=1, payload `[38]`) → `_BoundErr(0)` → **RangeError**. Đây là guard chuẩn dùng chung cho mọi `case`, không phải nghiệp vụ.
- **Chiều C→S vắng mặt**: `ts_decompile/functions/0077f414_FUN_0077F414.c` **không có `case 0x38:`** (grep exit code 1, không match); hai case kề là `case 0x37:` (dòng 1012) và `case 0x39:` (dòng 1020). Kết luận **S→C một chiều** (§6).
- **Vì sao server lại gửi một no-op (suy luận)**: các OP lân cận 0x37 (Case 48) và 0x39 (Case 50) đều **có nghiệp vụ thực** (bật/tắt form, đổi tốc độ, phát banner). 0x38 nằm kẹp giữa chúng nhưng body **rỗng hoàn toàn**; nhiều khả năng đây là **slot đã bị lược bỏ (deprecated)** hoặc **heartbeat/ack/padding** phía server. **Đây là suy luận từ bối cảnh, KHÔNG phải hành vi quan sát được** — bản thân handler không chứng minh được mục đích.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Framing (kế thừa, áp dụng chung mọi MainOp)

```
[Token 2B: 0xF4 0x44][Length L: Word LE 2B][Payload L bytes]   ; toàn frame XOR 0xAD
Payload[0] = MainOp (0x38)   →   RestPayload = Payload[1..]   →   RP[0] = SubOp
```

- **XOR + tách khung**: `ts_decompile/functions/0050cd6c_TForm1.ClientSocket1Read.c` — dòng 67 `FUN_0050a248(DAT_009264a4, local_10, 0xad, piVar6)` (XOR khóa `0xAD`); dòng 75-76 cắt 2 byte token và so với `&DAT_0050cf48`; dòng 78-79 cắt 2 byte length giải mã bằng `FUN_0077eb9c`; dòng 86-94 cắt đúng `L` byte payload (từ vị trí 5) và đẩy vào hàng đợi `TForm1_CY_AddRevQueue`.
- **Bơm gói + tách MainOp/RestPayload**: `ts_decompile/functions/00516158_TForm1.CY_DelRevQueue.c` — dòng 74 pop payload; dòng 86 `_LStrCopy(local_c, 2, uVar3, piVar7)` (RestPayload = từ ký tự 2 trở đi); dòng 89-92 guard rỗng; dòng 93 `FUN_0078a89c(DAT_009264a0, *(undefined1 *)(iVar2 + iVar5))` (gọi dispatcher với `MainOp` = byte đầu).

### 2.2. Đường tới handler (đã xác minh đầy đủ)

```
MainOp 0x38 (56) → byte_table[0x78A8EE][0x38] = 0x31 (49)
                 → dword_table[0x78A9B6][49] @ 0x0078AA7A = 0x00795555
                 → FUN_00795555 (Case 49)
```

| Bước | Nguồn | Giá trị |
| :-- | :-- | :-- |
| Byte table | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` **hàng 4** (index `0x30..0x3F`) = `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38` | offset `0x38` (byte thứ 9) = **`0x31` = 49** |
| Dword table | `ts_decompile/redump/jumptable_0x78A9B6_case_functions.csv:51` | `49,0x0078AA7A,0x00795555,YES,"FUN_00795555",0x00795555,1` |
| Manifest | `ts_decompile/case_functions/manifest.csv:51` | `49,0x0078AA7A,0x00795555,EXPORTED,"FUN_00795555","00795555",1,"functions/case_049_00795555_FUN_00795555.c",` |
| File case | `ts_decompile/case_functions/functions/case_049_00795555_FUN_00795555.c` (58 dòng) | Tiêu đề: `Case index: 49` / `Jump table entry: 0x0078AA7A` / `Target: 0x00795555` / `Function: FUN_00795555` (dòng 1-4) |
| Bản gộp | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c:8496-8551` | Khối `Case index: 49` (dòng 8496-8500), hàm tại dòng 8503 |
| Bản inline | `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6849-6856` | `case 0x38:` khớp logic 1:1 |

Ma trận dispatcher `switch(local_9)` (MainOp) mở tại `0078a89c_FUN_0078a89c.c:579` (`local_9 = param_2`, dòng 572). `case 0x38:` của ma trận chính nằm tại **dòng 6849** (thụt lề 2 space).

### 2.3. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x38` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile, `unaff_EBP + -0xc` = con trỏ dữ liệu của `RestPayload` (Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RP - 4)` = **độ dài** chuỗi RestPayload (length prefix của AnsiString).

### 2.4. Đọc SubOp (`case_049` dòng 20-26)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);       // iVar2 = RestPayload (con trỏ data)   [dòng 20]
iVar1 = 0;                                //                                      [dòng 21]
if (*(int *)(iVar2 + -4) == 0) {          // độ dài RestPayload == 0              [dòng 22]
  iVar1 = _BoundErr(0);                   // → RangeError                         [dòng 23]
  iVar2 = extraout_EDX;                   // (đường phục hồi giả của decompiler)  [dòng 24]
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]   [dòng 26]
```

Diễn giải:
- Nếu `RestPayload` **rỗng** → `_BoundErr(0)`; nhánh `iVar2 = extraout_EDX` cùng `iVar1` chỉ là đường phục hồi giả của decompiler cho nhánh exception. **L=1 → RangeError** (giống `opcode_33.md` §2.3).
- Ngược lại `SubOp = RP[0]` được ghi vào `unaff_EBP + -0x14` (dòng 26). **Từ dòng 27 trở đi không có bất kỳ lệnh nào đọc lại biến này.**

### 2.5. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Không gọi** |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `FUN_0077eb9c` (2B → Word LE) | Chỉ dùng ở tầng framing (length), không dùng trong handler |
| `_BoundErr` | Chốt chặn `RestPayload` rỗng (dòng 22-24) |
| Callee nội bộ | **Không có** — handler không gọi hàm con nào |
| `gvar_*` | **Không truy cập** |
| `_LStrArrayClr` / `_LStrClr` (dòng 30-54) | **Dọn dẹp frame cục bộ** dùng chung cho khối case; không liên quan wire |

### 2.6. Đối chiếu file case vs bản inline (1:1)

| Bước | `case_049_...c` | `0078a89c_FUN_0078a89c.c:6849-6856` |
| :-- | :-- | :-- |
| Check rỗng | dòng 22 `if (*(int *)(iVar2 + -4) == 0)` | dòng 6851 `if (*(int *)(local_10 + -4) == 0)` |
| `BoundErr` | dòng 23 `iVar1 = _BoundErr(0)` | dòng 6854 `@BoundErr(0)` |
| Nhãn handler exception | (dùng `extraout_EDX`, dòng 24) | dòng 6852 `in_stack_ffffffd4 = (code *)&UNK_00795568` |
| Lưu SubOp | dòng 26 `*(uint*)(unaff_EBP+-0x14) = (uint)*(byte*)(iVar2+iVar1)` | **KHÔNG lưu** (rơi thẳng `break`) |
| Nhánh theo SubOp | **KHÔNG có** | **KHÔNG có** |
| Kết thúc | dòng 55 `return` | dòng 6856 `break` |

Khác biệt duy nhất: file case giữ **dead store** `SubOp` (dòng 26) mà không ai đọc; bản inline lược bỏ luôn store đó. Cả hai **tương đương ngữ nghĩa: no-op**, chỉ khác `&UNK_00795568` = nhãn exception của chính handler (địa chỉ `0x00795568`, **không phải chuỗi**).

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler **không dùng `switch`, không so sánh SubOp**, **không có `default`**.

| SubOp (RP[0]) | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| *(rỗng)* | `[38]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **RangeError** (dòng 22-24) — **duy nhất có hiệu ứng** |
| `0x00` | `[38][00]` | `SubOp = 0` | **no-op im lặng** |
| `0x01` | `[38][01]` | `SubOp = 1` | **no-op im lặng** |
| `0x02..0xFF` | `[38][xx]` | `SubOp = xx` | **no-op im lặng** |
| *(tuỳ ý, L≥2)* | `[38][..][..]` | phần đuôi `RP[1..]` | **bỏ qua hoàn toàn** (không đọc) |

**Tổng: 0 nhánh có hiệu lực nghiệp vụ. Toàn bộ 256 giá trị SubOp = no-op.** Đây là **OP rỗng duy nhất** trong dải 0x30–0x3F đã phân tích (tương phản OP 0x37/case 48 có 2 SubOp + 2 SubSubOp; OP 0x39/case 50 có 3 SubOp).

---

## 4. Chi tiết các nhánh (bỏ graphics/sound/animation)

### 4.1. Thân hàm đầy đủ — chứng minh "không nhánh"

Trích nguyên `case_049_00795555_FUN_00795555.c` dòng 20-55:

```c
iVar2 = *(int *)(unaff_EBP + -0xc);                       // 20: RestPayload
iVar1 = 0;                                                // 21
if (*(int *)(iVar2 + -4) == 0) {                          // 22: rỗng?
  iVar1 = _BoundErr(0);                                   // 23
  iVar2 = extraout_EDX;                                   // 24
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // 26: SubOp = RP[0]  ← ĐỌC RỒI BỎ
*in_FS_OFFSET = in_stack_00000000;                        // 27: epilogue unwind
puStack00000008 = &LAB_00796408;                          // 28
uStack00000004 = 0x796364;                                // 29
_LStrArrayClr((int *)(unaff_EBP + -0x56c),99);            // 30
... (dòng 31-54: chỉ còn _LStrClr/_LStrArrayClr dọn frame) ...
_LStrClr((int *)(unaff_EBP + -0xc));                      // 54
return;                                                   // 55
```

Chứng minh không nhánh:
- **Duy nhất một `if`** trong toàn hàm (dòng 22) — điều kiện là **độ dài RestPayload**, **không phải SubOp**. Không có `else`.
- **Biến `unaff_EBP + -0x14` (SubOp) ghi lần cuối ở dòng 26 và không bao giờ xuất hiện lại** trong 58 dòng còn lại (không có lần đọc thứ hai — kiểm tra bằng đọc toàn văn file).
- Không `switch`, không `default`, không `(**(code ...)` (kiểm tra grep: 0 match trong `case_049`).
- Không truy cập `gvar_*`, không gọi hàm con. Không có sub-table/indirect dispatch.
- Dòng 27-54 là **epilogue dùng chung của dispatcher** (dọn 99+14+45+35+3+21+2 phần tử chuỗi + 5 `_LStrClr`), chạy **vô điều kiện** — không phải nhánh nghiệp vụ.

### 4.2. Đối chiếu sibling CÓ nhánh (chứng minh 0x38 đặc biệt)

- **OP 0x37 / Case 48 (`case_048_007954A5_FUN_007954a5.c`)**: dòng 28 `if (... == 1)` → `(**(code**)(**(int**)gvar_007DA3B4 + 0x20))()` (method VMT+0x20 của **TSe_CafeIDForm**); dòng 31 `else if (... == 2)` → đọc `RP[1]`, dòng 38/41 rẽ nhánh `1`/`2` phát banner qua `gvar_007DA084` VMT+0x90 với `&UNK_007991d8` / `&UNK_007991ec` (2000 ms). → **có `if/else if` + đọc thêm byte.**
- **OP 0x39 / Case 50 (`case_050_00795579_FUN_00795579.c`)**: dòng 32 `if (... == 1)` → `func_0x0055374c(gvar_007D9D88, RP[1], 0)` (dòng 39) rồi đọc `RP[1]==3` (dòng 46) → đọc `RP[2]` (dòng 53/56) đặt `*(gvar_007DA778+0x38)=100/1000`; dòng 61 `else if (... == 2)` → `FUN_00553410(gvar_007D9D88)`; dòng 64 `else if (... == 3)` → banner `&UNK_00799200`/`&UNK_00799224` qua `gvar_007DA084`. → **3 SubOp + đọc 2 byte phụ.**
- **OP 0x38 / Case 49**: **không có `if`/`else if` nào trên SubOp** — đúng là trường hợp rỗng giữa hai handler "đầy đủ".

> Ghi chú bối cảnh (xác minh danh tính form, để giải thích sibling): `gvar_007DA3B4` = con trỏ **TSe_CafeIDForm** và `gvar_007DA084` = con trỏ **TSe_TalkMsgFormPlus**, theo `ts_decompile/functions/0051189c_FUN_0051189c.c:1186-1187` (`VMT_58A680_TSe_CafeIDForm` → `gvar_007DA3B4`) và dòng 1265-1266 (`VMT_63B39C_TSe_TalkMsgFormPlus` → `gvar_007DA084`).

### 4.3. Các `case 0x38:` khác trong dispatcher KHÔNG phải MainOp 0x38

Trong `0078a89c_FUN_0078a89c.c` có **5** lần xuất hiện `case 0x38:`; chỉ **1** là ma trận MainOp chính:

| Dòng | Thụt lề | Ngữ cảnh | Kết luận |
| :-- | :-- | :-- | :-- |
| 757 | 4 space | Trong `switch(uVar22)` (dòng 590) của MainOp `0x00` — gán nhãn lỗi `@LStrLAsg(...,0x796d10)` | SubOp 0x38 của OP 0x00 — **không liên quan** |
| 3603 | 4 space | Trong `switch(*(undefined1*)(iVar21+iVar20))` — SubOp 0x38 của một MainOp khác, gọi `func_0x0074e638(gvar_007DA7BC, ...)` | **không liên quan** |
| 4078 | 4 space | SubOp 0x38 của một MainOp khác, gọi `func_0x00745898(gvar_007DA7BC, ...)` | **không liên quan** |
| 6104 | 4 space | SubOp 0x38 của một MainOp khác, đặt `*(gvar_007DA7BC+0x13ca)=0` và `(**(code**)(...VMT+0x18))(...,8)` | **không liên quan** |
| **6849** | **2 space** | **`case 0x38:` của `switch(local_9)` (MainOp, mở dòng 579)** — guard `BoundErr(0)` rồi `break` | **ĐÂY LÀ handler MainOp 0x38** |

→ Nhầm lẫn dễ xảy ra: các dòng 757/3603/4078/6104 là **SubOp 0x38 của các MainOp khác**, thụt lề sâu hơn (4 space); chỉ dòng 6849 (2 space, cùng cấp `case 0x2e`/`case 0x39`) mới là handler MainOp 0x38 thật.

---

## 5. Chuỗi VISCII → UTF-8

- **Không có chuỗi nào trên đường đã xác minh.** Bằng chứng trong `case_049_00795555_FUN_00795555.c`:
  - Không có `_LStrCopy` / `_LStrCatN` / `_LStrFromString` / `_LStrLAsg`.
  - Không tham chiếu literal `.rodata`: **0 match** `&DAT_` / `&UNK_` khi grep toàn file. Chỉ có `&LAB_00796408` (dòng 28) — nhãn unwind stack.
  - Các giá trị `0x796364`, `0x79636f`, `0x79637a`, …, `0x7963fd` (dòng 29-54) được gán cho `uStack00000004` — đây là **địa chỉ return của thunk `_LStrClr`/`_LStrArrayClr`** (mã code, không phải dữ liệu), và thoải mái không được gửi ra wire.
  - Không gọi API banner `(VMT+0x90)`.
- Vì handler **không đọc** `RP[1..]`, kể cả khi client gửi thêm byte phía sau `SubOp`, phần đuôi cũng **bị bỏ qua** → **không có VISCII nào được decode**.
- **Kết luận: handler OP 0x38 không có bảng mã VISCII → UTF-8.** Không cần xử lý chuỗi khi mock.

---

## 6. Chiều Client → Server

File `ts_decompile/functions/0077f414_FUN_0077F414.c` (hàm `TFConnect.SendCommand`) có **đúng một** `switch(param_2 & 0xff) {` tại **dòng 768**, đóng tại **dòng 1078-1079** (`case 199: }`). Grep toàn file:

| Truy vấn | Kết quả |
| :-- | :-- |
| `switch` | 1 match duy nhất — dòng 768 |
| `default:` | **0 match** (switch không có `default`) |
| `case 0x38` | **0 match** (exit code 1) |
| `case 0x37:` | dòng 1012 (có code: `_LStrFromString`/`_LStrCat`/`TForm1_CY_AddSedQueue`; `break` dòng 1019) |
| `case 0x39:` | dòng 1020 (rỗng → `break;` dòng 1021) |

Đọc trực tiếp vùng lân cận dòng 1007-1021 xác nhận thứ tự `case 0x36:` (1007) → `case 0x37:` (1012) → **`0x38` bị nhảy** → `case 0x39:` (1020) → `case 0x3a:` (1022). **Không có `case 0x38:`.**

**Kết luận**: client **không bao giờ gửi Main OP 0x38**; nếu giá trị `0x38` lọt vào `SendCommand`, nó rơi khỏi `switch` (không nhánh khớp, không `default`) → no-op phía client. Vậy **OP 0x38 là kênh S→C một chiều**. Không có format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[38]                  1B  → L=1 → RP rỗng → BoundErr(0) = RangeError   (KHÔNG GỬI)
[38][00]              2B  → no-op im lặng
[38][01]              2B  → no-op im lặng
[38][xx], xx bất kỳ   2B  → no-op im lặng
[38][01][..] (L>2)   3B+  → no-op im lặng (đuôi RP[1..] bị bỏ qua)
```

- **Hiệu ứng nghiệp vụ = KHÔNG CÓ.** Không SubOp nào tạo state, không form nào bật/tắt, không banner, không log — đã chứng minh từ thân hàm (dòng 20-55).
- **Cảnh báo: KHÔNG gửi `[38]` một mình** (L=1) vì gây `RangeError` phía client (`_BoundErr(0)`, dòng 22-23).
- Nếu cần phát OP 0x38 để thăm dò/giữ nhịp kết nối, payload an toàn tối thiểu là **`[38][00]` (L=2)**; gửi thêm byte cũng vô hại vì handler không đọc `RP[1..]`.
- **Không có field nào để suy layout** — handler chỉ chạm `RP[0]` (rồi bỏ) và `*(RP-4)` (độ dài).
- Mục đích thật của 0x38 (placeholder/ack/heartbeat) **không suy ra được từ SSOT**; muốn xác minh phải quan sát timing/thứ tự gói live.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_049_00795555_FUN_00795555.c` (58 dòng; thân: 20-55) | Handler chính — đọc SubOp (dòng 26), không nhánh, epilogue (27-54), `return` (55) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:579` (MainOp switch), `:6849-6856` (`case 0x38:` inline) | Bản inline dispatcher — xác nhận 1:1 (chỉ `BoundErr` + `break`) |
| 3 | `functions/0078a89c_FUN_0078a89c.c:757,3603,4078,6104` (thụt lề 4 space) | Chứng minh các `case 0x38:` còn lại là SubOp lồng nhau của MainOp khác, không phải MainOp 0x38 |
| 4 | `redump/jumptable_byte200_0x78A8EE.hex` (hàng 4, offset `0x38` = `0x31`) + `redump/jumptable_0x78A9B6_case_functions.csv:51` + `case_functions/manifest.csv:51` | Mapping MainOp 0x38 → Case 49 → `0x00795555` |
| 5 | `case_functions/jumptable_0x78A9B6_cases.c:8496-8551` (hàm dòng 8503, thân 8515-8521, epilogue 8522-8550) | Bản gộp case 49 — trùng khớp file case |
| 6 | `functions/0050cd6c_TForm1.ClientSocket1Read.c:67,75-79,86-94` và `functions/00516158_TForm1.CY_DelRevQueue.c:74,86,89-93` | Framing XOR 0xAD / token / length / tách MainOp+RestPayload → dispatcher |
| 7 | `functions/0077f414_FUN_0077F414.c:768` (switch), `:1007-1022` (`0x36`,`0x37`,`0x39`,`0x3a`), `:1078-1079` (kết switch) + grep `case 0x38` (0 match), `default:` (0 match) | Chiều C→S — xác nhận **không có `case 0x38`** |
| 8 | Sibling: `case_functions/functions/case_048_007954A5_FUN_007954a5.c:28,31,38,41` và `case_functions/functions/case_050_00795579_FUN_00795579.c:32,39,61,64,72,75` | Đối chiếu sibling CÓ nhánh (0x37 / 0x39) |
| 9 | `functions/0051189c_FUN_0051189c.c:1186-1187` (TSe_CafeIDForm→`gvar_007DA3B4`), `:1265-1266` (TSe_TalkMsgFormPlus→`gvar_007DA084`) | Xác minh danh tính form trong bối cảnh sibling |
| 10 | Grep `00795555` / `UNK_00795568` toàn SSOT | Không có caller/duplicate nào khác ngoài case 49, bundled case 49, manifest/CSV và nhãn exception inline 6852 |

### Điểm chưa xác minh được từ SSOT (không suy đoán)

1. **Mục đích nghiệp vụ thật của Main OP 0x38** (placeholder / ack / heartbeat / slot deprecated) — **không thể suy ra từ handler** vì body rỗng. Mọi diễn giải ở §1/§4.3 là **suy luận**, không phải hành vi xác minh.
2. **Ý nghĩa phần đuôi `RP[1..]` (nếu có trên wire thực tế)** — handler bỏ qua hoàn toàn, không có cách suy layout từ SSOT.
3. **Liệu server có thực sự phát 0x38 trong phiên live hay không, và ở thời điểm nào** — cần sample/redump traffic thật để xác nhận.
4. **Quan hệ giữa 0x38 và 0x37/0x39** (ví dụ 3 OP có từng là một nhóm chức năng bị rút gọn) — chỉ là **suy luận từ thứ tự kề nhau trong bảng**, không có bằng chứng code.
