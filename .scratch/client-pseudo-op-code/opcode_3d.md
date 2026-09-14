# PHÂN TÍCH — Main OP 0x3D (61) / Case 54 / FUN_007957dc @ 0x007957DC

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)** là chiều chính, **và C→S CÓ THẬT** (`case 0x3d` trong `FUN_0077f414`, xem §6).
Trạng thái: **Đã xác minh phần vỏ (framing / dispatch / SubOp / nhánh xử lý) từ SSOT** (`ts_decompile/` only).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). **SÁU callee ĐÃ CÓ BODY** (`0054f18c`, `0054f230`, `0054f818`, `00728598`, `00728734`, `007287e4` + `0054f384`) → wire S→C sau SubOp khôi phục ở §4. **CHÍN chuỗi banner ĐÃ DUMP + GIẢI MÃ VISCII** (§5) — toàn bộ là **const AnsiString có header `[FF FF FF FF][len:4LE]`** (trả lời đúng câu hỏi PChar-vs-AnsiString của bản cũ); nội dung là **thông báo máy Xổ số (Lotto)**. **Call-site C→S `0x3D` ĐÃ TÌM THẤY** (đính chính "0 hit"): `0054e4f0_FUN_0054e4f0.asm.txt:12–15` — `MOV CL,0x1; MOV DL,0x3d` với guard `Self+0x6f == 5`; field `+0x6a..+0x6e` **xác minh là 5 số đã chọn (1..42)**, `+0x6f` = bộ đếm chọn. **2/3 gvar đích chốt tên class từ dòng gán mới**: `gvar_007D9F98 = TLottoManager`, `gvar_007DA4EC = TMachineManager` (`0055374c_FUN_0055374c.c:25–27,47–49`) — đoán "họ `TMachineManager`" cũ: ĐÚNG một nửa (type 6), ĐÍNH CHÍNH type 4 thành `TLottoManager`; `gvar_007D9C48` vẫn chưa có creator.

> Phạm vi: framing, dispatch, SubOp, layout `RestPayload`, lõi logic, wire format. Bỏ qua graphics / sound / animation / chi tiết render form.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: OP 0x3D là opcode **điều khiển đa mục đích cho các "máy"/bảng minigame** (machine) và một **bảng UI game dùng chung**. Đây là handler lớn nhất trong cụm `0x3A–0x3D` (133 dòng, 10 nhánh SubOp + 1 `switch` lồng 9 nhánh).
- **Ba đối tượng đích** (dispatch theo SubOp) — **tên class của 2/3 đã chốt bằng dòng gán (2026-09-14, `0055374c.c:25–27/47–49`; creator chạy từ OP 0x39)**:
  - `gvar_007D9F98` — **`TLottoManager`** (VMT `VMT_54D580_TLottoManager`, machine type 4) — nhận SubOp `1, 2, 3` → **máy Xổ số (Lotto)**; họ hàng gần: widget `TLottoMachineMain` (VMT `VMT_54D5F0`, ctor `0054f384`).
  - `gvar_007DA4EC` — **`TMachineManager`** (VMT `VMT_54D6D8`, machine type 6) — nhận SubOp `7, 8, 9` (đoán cũ "họ TMachineManager" **xác minh đúng cho type 6**).
  - `gvar_007D9C48` — object UI/game dùng chung (họ hàm `FUN_0072xxxx`); nhận SubOp `4, 10, 11`. **Creator vẫn chưa tìm thấy** — và phát hiện mới: **cả 3 callee SubOp 4/10/11 KHÔNG hề dùng param_1** (`00728598.c:56`, `00728734.c:37`, `007287e4.c:43` — `local_8 = param_1` rồi không đọc nữa) — chúng format text rồi bắn vào kênh message `FUN_007ab870(gvar_007DA1B0^, 0, text, ·)` (§4.3/4.8).
- **SubOp `5`** là nhánh duy nhất **không dùng form đích** mà phát **banner/thông báo nổi** trên `gvar_007DA084` (`TSe_TalkMsgFormPlus`, `vtable+0x90`) — 9 biến thể theo `RP[1] = 1..9`.
- **Cấu trúc chung**: `SubOp = RP[0]`; các nhánh `1,2,3,4,5,7,8,9,10,11` — **KHÔNG có `default`, KHÔNG có `case 6`**. Mọi giá trị khác (`0x00`, `0x06`, `>= 0x0C`) là **no-op im lặng**.
- **Hai cổng chặn độ dài**:
  - `RestPayload` rỗng (payload `[3D]`, L=1) → `_BoundErr(0)` = **ERangeError**.
  - `SubOp 5` mà `RestPayload` chỉ dài 1 (payload `[3D][05]`, L=2) → `_BoundErr(1)` = **ERangeError**.
- **C→S CÓ THẬT — ĐÃ TÌM THẤY CALL-SITE (2026-09-14)**: client gửi payload **7 byte** `[3D][CL=1][obj+0x6a][obj+0x6b][obj+0x6c][obj+0x6d][obj+0x6e]` qua `TFConnect.SendCommand` (`0077f414_FUN_0077F414.c:1044-1059`, asm `3785-3854`; builder xác nhận bằng bảng đỏ: `byte[0x3D]=0x33`, `dword[0x3D]=0x0078A318` — `redump/table_0x77F474_200B.hex`/`table_0x77F53C_dword200.hex`, parse python). Call-site: **`FUN_0054e4f0`** (`0054e4f0_FUN_0054e4f0.asm.txt:12–15`: `MOV CL,0x1; MOV DL,0x3d`) với điều kiện `Self+0x6f == 5` — đính chính nhận định "`MOV DL,0x3D` 0 hit" của bản cũ (hàm này nằm trong đợt decompile mới). **`CL` chốt = 1**; 5 byte cuối = **5 số người chơi chọn (mỗi số 1..42)**, `+0x6f` = bộ đếm số đã chọn (§6.5).
- **Bối cảnh (XÁC MINH 2026-09-14)**: `gvar_007D9F98 = TLottoManager` — **máy Xổ số**; 9 banner SubOp 5 đã decode (§5) toàn bộ là thông báo xổ số/đối chiếu giải thưởng ("Xổ số kỳ này bạn không trúng thưởng", "Thời gian đối chiếu giải thưởng là ... 20:00 ... trước 18:00!"…), payload C→S là **5 số chọn trong 1..42**, SubOp 9 nạp 6 "ball" 1..42 vào `TMachineManager` ⇒ cụm `0x3D` = **kênh điều khiển máy Xổ số (keno-style 42 số) + kênh message/banner dùng chung**. `gvar_007D9C48` vẫn giữ nhãn "object UI dùng chung" (creator chưa thấy).

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler (đã kiểm lại từng mắt xích)

```
MainOp 0x3D (61) → byte_table[0x78A8EE][0x3D] = 0x36 (54)
                 → dword_table[0x78A9B6][54] @ 0x0078AA8E = 0x007957DC
                 → FUN_007957dc (Case 54)
```

- **Bảng byte 200**: `redump/jumptable_byte200_0x78A8EE.hex` — **hàng 4** ứng với index `0x30..0x3F`: `00 00 2B 2C 2D 2E 2F 30 31 32 33 34 35 36 37 38`. Byte thứ 14 của hàng (offset `0x3D`) = **`0x36` = 54** (khớp `Case 54`).
- **Bảng dword**: `redump/jumptable_dword200_0x78A9B6.hex` **hàng 7** (entries 48–55): entry #54 = bytes `DC 57 79 00` → **`0x007957DC`** (LE).
- **CSV**: `redump/jumptable_0x78A9B6_case_functions.csv:56` → `54,0x0078AA8E,0x007957DC,YES,"FUN_007957dc",0x007957DC,1`.
- **Manifest**: `case_functions/manifest.csv:56` → `54,0x0078AA8E,0x007957DC,EXPORTED,"FUN_007957dc","007957dc",1,"functions/case_054_007957DC_FUN_007957dc.c",`.
- **File case chính**: `case_functions/functions/case_054_007957DC_FUN_007957dc.c` (133 dòng); SubOp read tại **dòng 21-28**, switch tại **dòng 28-101**.
- **Bản gộp jump table**: `case_functions/jumptable_0x78A9B6_cases.c:8869` (header "Case index: 54"), hàm tại dòng **8875**, lõi logic tại **dòng 8888-8967**.
- **Bản inline trong dispatcher tổng**: `functions/0078a89c_FUN_0078a89c.c:6987-7081` (nhãn `case 0x3d:`) — **khớp 1:1** (§2.5).
- Framing/XOR/pump/`L`/`RestPayload` giống các OP khác; xem quy ước tại `opcode_00_01.md` §2 và `opcode_2d.md` §2. Frame: `[Token 2B: F4 44][Length L: Word LE][Payload]`, toàn khung XOR `0xAD`.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0] = 0x3D` là MainOp), `RP[i]` = byte RestPayload (`RP[i] = P[i+1]`), tức `RP[0] = SubOp`.
- Trong mã decompile: `unaff_EBP + -0xc` = con trỏ dữ liệu `RestPayload` (Delphi AnsiString), `unaff_EBP + -0x14` = biến lưu SubOp.
- `*(int*)(RP-4)` = **độ dài** `RestPayload` (length prefix của AnsiString).
- `_LStrCopy(s, index, count, &out)` là **1-based**: `_LStrCopy(RP, 3, 4)` = `RP[2..5]` = `P[3..6]`.
- `gvar_007D9F98`, `gvar_007DA4EC`, `gvar_007D9C48` là con trỏ `void**` tới instance (dereference 2 lần: `*(int*)gvar` = `Self`).

### 2.3. Đọc SubOp & `switch` (`case_054` dòng 21-28)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // iVar2 = RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {             // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                      // → ERangeError (payload [3D], L=1)
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
switch(*(undefined4 *)(unaff_EBP + -0x14)) {
case 1: ...
```

Diễn giải:
- `RestPayload` rỗng → `_BoundErr(0)`; `iVar2 = extraout_EDX`/`iVar1` là đường phục hồi giả của decompiler cho nhánh exception (thực tế là lỗi range). **L=1 → ERangeError.**
- `SubOp = RP[0]`; decompile dùng **`switch` thật** (khác OP 0x37 dùng `if/else if`).
- **Không có `case 6`, không có `default`**.

### 2.4. Codec & API

| Helper / API | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` (4B → DWORD LE) | **Gián tiếp** qua callee `FUN_0054f900` (dòng 72, 80) — giải mã 2 dword từ RP |
| `FUN_0077eb9c` (2B → Word LE) | **Không gọi** trực tiếp ở handler |
| `FUN_0077f098` (8B → double) | **Không gọi** |
| `_LStrCopy` | **Gián tiếp** qua callee (`FUN_0054f900:71,79`) |
| `_BoundErr` | Chốt chặn độ dài `RestPayload` (dòng 24 và 45) |
| `gvar_007D9F98` | Instance "machine" (SubOp 1,2,3) |
| `gvar_007DA4EC` | Instance "machine" (SubOp 7,8,9) |
| `gvar_007D9C48` | Object UI/game dùng chung (SubOp 4,10,11) |
| `gvar_007DA084` | Con trỏ `TSe_TalkMsgFormPlus`; method `vtable+0x90` = banner (SubOp 5) |
| `DAT_00799254 … DAT_00799450` | 9 literal `.data` truyền làm text banner; **không có dump** (§5) |
| `switchD_00792e22::caseD_0()` | **Artifact decompiler** = nhánh cleanup/epilogue dùng chung (đi kèm `return`), **không phải logic wire** |
| `_LStrArrayClr/_LStrClr` (dòng 105-129) | **Dọn dẹp frame cục bộ** của dispatcher (danh sách rất dài: 99, 14, 45, 35, … phần tử), **không liên quan wire** |

### 2.5. Đối chiếu file case vs bản inline vs bản gộp (1:1)

| Vị trí | `case_054_...c` | `0078a89c_FUN_0078a89c.c` |
| :-- | :-- | :-- |
| Lấy RestPayload | dòng 21 | dòng 6989 `iVar21 = local_10` |
| Check rỗng | dòng 23 | dòng 6991 |
| SubOp | dòng 27-28 `switch` | dòng 6997 `switch` |
| SubOp1 call | dòng 30 `FUN_0054e2f8` | dòng 7000 (giống) |
| SubOp5 check | dòng 44 `if (*(uint*)(iVar2+-4) < 2)` | dòng 7017 (giống) |
| SubOp5 case 1 | dòng 50 banner `DAT_00799254,0x898,0,0` | dòng 7024 banner `DAT_00799254,0x898` |
| SubOp5 case 9 | dòng 82 banner `DAT_00799450,0x4b0,0,0` | dòng 7056 banner `DAT_00799450,0x4b0` |
| SubOp7/8/9 | dòng 88/91/94 | dòng 7063/7067/7071 |
| SubOp10/11 | dòng 97/100 | dòng 7075/7079 |

Khác biệt **duy nhất**:
1. Tên biến tạm của decompiler (`iVar1/iVar2/extraout_EDX*` vs `iVar20/iVar21/extraout_EDX_x0018x*`) và nhãn SEH marker (`&UNK_007957ef…`).
2. Bản inline hiển thị **3 tham số** cho banner `(self, &literals, duration)`, bản case hiển thị **5 tham số** `(self, &literals, duration, 0, 0)` — **artifact giải mã** (2 tham số `0,0` cuối là hằng 0 dùng chung stack), **không phải khác biệt hành vi**.

**Logic trùng khớp hoàn toàn.** Bản gộp `jumptable_0x78A9B6_cases.c:8875-8998` cũng trùng 1:1.

---

## 3. Bảng tổng hợp SubOp

`SubOp = RP[0]`. Handler dùng `switch`; **không có `default`, không có `case 6`**. Nhánh `5` có **`switch` lồng** trên `RP[1]`.

| SubOp (RP[0]) | Đích | Payload S→C (P[0]=0x3D) | Điều kiện đọc | Core logic | SSOT callee |
| :---: | :-- | :-- | :-- | :--- | :--- |
| *(rỗng)* | — | `[3D]` (L=1) | `*(int*)(RP-4) == 0` | `_BoundErr(0)` → **ERangeError** (dòng 23-26) | — |
| `0x01` | `gvar_007D9F98` | `[3D][01][f1][b0..b4]` (L=8) | `RP ≥ 7` (dòng 51-55, 70-73) | `FUN_0054e2f8`: tăng đếm `+0x68` (<0x15), ghi `+0x69=f1`, ghi 5 byte `RP[2..6]` vào slot `(count-1)*5` | **có body** |
| `0x02` | `gvar_007D9F98` | `[3D][02][D:4B LE]` (L≥6) | `_LStrCopy(RP,2,4)` (callee) | **`FUN_0054f18c`**: DWORD `RP[1..4]` → giải mã qua `FUN_0077ef7c` → ghi `obj+0x16c` (`0054f18c.c:41–44`) — **xác minh từ body mới** | **có body** |
| `0x03` | `gvar_007D9F98` | `[3D][03][cmd]` (L=3) | `RP ≥ 2`; `cmd ∈ {1,2,3}` (else im lặng) | **`FUN_0054f230`**: cmd 1 → banner `DAT_0054f2ec` + `obj+0x170 := 1`; cmd 2 → banner `DAT_0054f320`; cmd 3 → `obj+0x6f := 0; obj+0x68 := 0` + banner `DAT_0054f340` (`0054f230.c:27–45`) — **xác minh từ body mới** | **có body** |
| `0x04` | `gvar_007D9C48`*(param không dùng)* | `[3D][04][n1..n5][m]` (L≥8) | BoundErr khi RP thiếu byte | **`FUN_00728598`**: `IntToStr` từng byte `RP[1..5]` + `RP[6]`, ghép vào template text `0x7286F4` (code-gap, chưa dump) rồi gửi qua **`FUN_007ab870(gvar_007DA1B0^, 0, msg, 0)`** = kênh message/chat (`00728598.c:57–99`) | **có body** |
| `0x05` | `gvar_007DA084` | `[3D][05][variant]` (L=3) | `RP ≥ 2` (dòng 44-47) | `switch(RP[1]) 1..9` → **9 banner** (xem §4.4). `RP[1]` khác 1..9 → fall-through → no-op | **có body** (banner) |
| `0x06` | — | `[3D][06]` (L=2) | — | **Không có case** → no-op im lặng | — |
| `0x07` | `gvar_007DA4EC` | `[3D][07][x][v1..v5]` (L≥8) | counter `+0xae < 0x15`; byte `RP[1]`, `RP[2..6]` | **`FUN_0054f818`**: tăng counter `+0xae` (max 21), `+0xaf := RP[1]`, 5 byte `RP[2..6]` → slot `(count-1)*5` của mảng 5B/record (khuôn y hệt SubOp 1 nhưng trên máy type-6) (`0054f818.c:31–83`) — **xác minh từ body mới** | **có body** |
| `0x08` | `gvar_007DA4EC` | `[3D][08][slot][A:4B LE][B:4B LE]` (L=11) | `RP ≥ 2` (dòng 58-63); đọc RP[2..9] | `FUN_0054f900`: lưu `A` vào `+100+(slot+1)*8`, `B` vào `+0x68+(slot+1)*8`, `%` vào `+0x90+(slot+1)*4` | **có body** |
| `0x09` | `gvar_007DA4EC` | `[3D][09][ball1..ball6]` (L=8) | đọc RP[1..6], mỗi byte 1..42 (dòng 53) | `FUN_0054f764`: ghi 6 byte vào `+0xa8..+0xad`, rồi `FUN_0054fac0` | **có body** |
| `0x0A` | `gvar_007D9C48`*(param không dùng)* | `[3D][0A][text...]` (L≥3) | `_LStrCopy(RP,2,len)` | **`FUN_00728734`**: lấy **toàn bộ byte sau SubOp làm text thô** (`RP[1..]`), ghép template `&DAT_007287C8` (code-gap, chưa dump) phía trước → `FUN_007ab870(gvar_007DA1B0^, 0, msg, 0)` = **tin nhắn tự do vào kênh message/chat** (`00728734.c:39–42`) | **có body** |
| `0x0B` | `gvar_007D9C48`*(param không dùng)* | `[3D][0B][k][v][text...]` (L≥5) | `RP ≥ 2`; `k ∈ {1,2}` | **`FUN_007287e4`**: `k := RP[1]`, `v := RP[2]`, text = `RP[3..]`; `k=1` → ghép `&DAT_0072892C + IntToStr(v) + &DAT_00728950 + default 0x72885D` gửi kênh message; `k=2` → mẫu tương tự với `&DAT_00728974` (`007287e4.c:60–75`; templates code-gap chưa dump) | **có body** |
| `>= 0x0C` | — | `[3D][xx]` (L=2) | — | **Không khớp case** → no-op im lặng | — |

**Tổng: 10 nhánh có hiệu lực** (`1,2,3,4,5,7,8,9,10,11`) + **3 nhánh đọc field có cấu trúc xác minh được** (`1,8,9` — bản cũ) **+ 6 nhánh còn lại (`2,3,4,7,10,11`) NAY CŨNG XÁC MINH ĐƯỢC TỪ BODY MỚI (§4.2–4.8)** + **9 banner** trong nhánh `5` đã decode (§5) + **2 dạng gây ERangeError**. Byte dư sau trường cuối **bị bỏ qua**.

---

## 4. Chi tiết các nhánh (bỏ graphics/sound/animation)

### 4.1. `SubOp == 0x01` → `FUN_0054e2f8` (thêm 1 bản ghi 5 byte)

```c
case 1:
  FUN_0054e2f8(*(uint *)gvar_007D9F98,*(int *)(unaff_EBP + -0xc));   // case_054 dòng 30
  break;
```

Body: `functions/0054e2f8_FUN_0054e2f8.c:19-99`:

```c
pcVar1 = (char *)(param_1 + 0x68);
cVar2 = *pcVar1;
*pcVar1 = *pcVar1 + '\x01';                 // tăng bộ đếm +0x68
...
if (*(byte *)(param_1 + 0x68) < 0x15) {     // chỉ xử lý khi count < 21
  if (*(uint *)(param_2 + -4) < 2) { iVar4 = _BoundErr(1); }   // cần RP[1]
  *(undefined1 *)(param_1 + 0x69) = *(undefined1 *)(iVar7 + iVar4);   // +0x69 = RP[1]
  local_10 = 1;
  do {
    ... bound-check RP[local_10 + 1] (tức RP[2..6]) ...
    iVar7 = param_1 + (count-1)*5;
    *(char *)(iVar7 + (local_10 - 1)) = (char)uVar6;   // slot[count-1][0..4] = RP[2..6]
    local_10++;
  } while (local_10 != 6);
}
```

- **State change**: tăng bộ đếm `+0x68` (guard `< 0x15` = 21); ghi `RP[1]` vào `+0x69`; ghi 5 byte `RP[2..6]` vào slot thứ `count-1` của mảng **5 byte/slot**.
- **Wire layout SubOp 1**:
  ```
  P[0]=0x3D  P[1]=0x01  P[2]=f1 (→+0x69)  P[3..7]=b0..b4 (→slot)
  Tối thiểu để đọc đủ 1 slot: L = 8
  ```
- **Giới hạn (bounds)**:
  - `RP` phải dài ≥ 2 để đọc `RP[1]`; nếu không → `_BoundErr(1)` (dòng 51-55).
  - Vòng lặp bound-check từng byte `RP[2..6]` → cần `RP` dài ≥ 7 để ghi đủ 5 byte (dòng 70-73).
- **Suy luận nghiệp vụ**: xây **danh sách/bản ghi 5 byte** (mỗi bản ghi 5 byte, tối đa ~20 bản ghi). Ý nghĩa ngữ nghĩa của 5 byte **chưa xác minh** (không có tên trường).

### 4.2. `SubOp == 0x02` & `0x03` → `FUN_0054f18c` / `FUN_0054f230` (ĐÃ CÓ BODY — xác minh từ body mới)

**`FUN_0054f18c` — SubOp 2 (ghi 1 DWORD vào `obj+0x16c`)** — `0054f18c_FUN_0054f18c.c:16–46`:
```c
_LStrCopy(RP, 2, 4, &tmp);                                   // RP[1..4]
*(uint*)(obj + 0x16c) = FUN_0077ef7c(gvar_007D9D30^, tmp);  // 4B → DWORD LE
```
- **Wire**: `[3D][02][D0 D1 D2 D3]` = 6 byte; `D` = DWORD LE ghi nguyên vào field `+0x16c` của `TLottoManager` (cặp với `+0x170` bên dưới — bộ field Lotto).
- Không bound-check RP trước (LStrCopy cắt mềm); hàm chạy trong 2 lớp SEH (`:29–34`) — lỗi chuỗi swallow.

**`FUN_0054f230` — SubOp 3 (3 lệnh banner/trạng thái theo `RP[1]`)** — `0054f230_FUN_0054f230.c:16–47`:
- `RP[1] == 1` → banner `&DAT_0054f2ec` 1200 ms + `obj+0x170 := 1`.
- `RP[1] == 2` → banner `&DAT_0054f320` 1200 ms.
- `RP[1] == 3` → **`obj+0x6f := 0` và `obj+0x68 := 0`** (reset bộ đếm chọn + bộ đếm record — đúng 2 field mà SubOp 1 và sender C→S đụng tới, §4.1/§6.5) + banner `&DAT_0054f340` 1200 ms.
- Khác 1/2/3: im lặng. **Wire**: `[3D][03][cmd]` = 3 byte.
- Chuỗi banner `0x54f2ec/0x54f320/0x54f340` nằm trong code-gap của chính vùng này — **chưa redump** (giới hạn, §8).

### 4.3. `SubOp == 0x04` → `FUN_00728598` (ĐÃ CÓ BODY — formatter tin nhắn số)

`00728598_FUN_00728598.c:16–103`: nhận `(param_1 = gvar_007D9C48^ — KHÔNG DÙNG, local_8 chỉ được gán không đọc)`. Đọc 5 byte `RP[1..5]` + `RP[6]` (bound `_BoundErr` từng byte → yêu cầu `len(RP) ≥ 7`, `:60–95`), mỗi byte `IntToStr` và ghép (`_LStrCatN`) vào chuỗi mẫu khởi tạo tại `0x7286F4` (code-gap, chưa dump), cuối cùng gửi **`FUN_007ab870(gvar_007DA1B0^, 0, msg, 0)`** (`:99`) — helper message 1181 byte (`007ab870_FUN_007ab870.c`) dùng chung toàn client (67+ callers, trong đó có các handler OP khác). **Wire**: `[3D][04][n1 n2 n3 n4 n5 m]` = 8 byte — 5 số nguyên nhỏ + 1 byte.

### 4.4. `SubOp == 0x05` → 9 biến thể banner trên `gvar_007DA084`

```
+----+--------------+----------+---------------------------------------+
| RP[1] | Literal  | Duration | Gọi thêm                              |
| (variant) |       | (ms)     |                                       |
+----+--------------+----------+---------------------------------------+
| 1  | DAT_00799254   | 0x898 (2200) | —                                 |
| 2  | DAT_007992d0   | 0x4b0 (1200) | —                                 |
| 3  | DAT_00799300   | 0x4b0 (1200) | —                                 |
| 4  | LAB_0079932c   | 0x4b0 (1200) | —                                 |
| 5  | DAT_00799350   | 0x4b0 (1200) | —                                 |
| 6  | DAT_00799384   | 0x4b0 (1200) | —                                 |
| 7  | DAT_007993d0   | 0x4b0 (1200) | —                                 |
| 8  | DAT_00799410   | 0x4b0 (1200) | —                                 |
| 9  | DAT_00799450   | 0x4b0 (1200) | —                                 |
| khác | —           | —            | fall-through → no-op               |
+----+--------------+----------+---------------------------------------+
```

```c
case 5:
  iVar1 = 1;
  if (*(uint *)(iVar2 + -4) < 2) { iVar1 = _BoundErr(1); ... }        // dòng 44-47
  switch(*(undefined1 *)(iVar2 + iVar1)) {                           // = RP[1]
  case 1:
    (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&DAT_00799254,0x898,0,0);  // dòng 50
    switchD_00792e22::caseD_0();                                     // artifact → epilogue
    return;
  ...
  }
  break;
```

- **`gvar_007DA084`** = instance `TSe_TalkMsgFormPlus` (đã xác minh ở các OP anh em: `functions/0051189c_FUN_0051189c.c:1265-1277`).
- **`vtable+0x90`** = method phát banner dùng chung `(self, textPtr, duration, 0, 0)` — **thân method VẪN KHÔNG có trong SSOT** (đợt dump 2026-09-14 không có VMT `0x63B39C` — `redump/` chỉ có 2 file `vmt_*.hex` khác; giữ nguyên ghi chú giới hạn).
- **Thứ tự giảm dần độ dài**: biến thể `1` (2200 ms) là dài nhất, còn lại đều `1200 ms`.
- **`switchD_00792e22::caseD_0()`** là **artifact decompiler** (nhảy tới epilogue dùng chung) — tương đương `goto cleanup; return;`, **không phải lời gọi hàm nghiệp vụ**.
- **Fall-through khi `RP[1]` khác 1..9**: sau `switch` trong cùng `case 5` có `break;` (dòng 86) → thoát switch ngoài → **no-op im lặng** (không ERangeError vì đã đủ 2 byte).
- **Bounds**: `RP` dài 1 (payload `[3D][05]`, L=2) → `_BoundErr(1)` = **ERangeError** (dòng 44-47).

### 4.5. `SubOp == 0x07` → `FUN_0054f818` (ĐÃ CÓ BODY — bản sao SubOp 1 trên máy type-6)

`0054f818_FUN_0054f818.c:16–86`: **khuôn giống hệt `FUN_0054e2f8` (§4.1) nhưng trên field khác**: tăng bộ đếm `+0xae` (guard `< 0x15` = 21), `+0xaf := RP[1]`, 5 byte `RP[2..6]` → mảng record **5 byte/slot** (`obj + count*5 + (i-1)`, bounds `count ≤ 20`). **Wire**: `[3D][07][x][v1 v2 v3 v4 v5]` = 8 byte; `RP` thiếu → `_BoundErr` swallow, không ghi. **Xác minh từ body mới** — 3 callee `0054e2f8/0054f18c/0054f818` cùng kiểu "danh sách 5 byte" cho hai máy Lotto (SubOp 1) và Machine (SubOp 7).

### 4.6. `SubOp == 0x08` → `FUN_0054f900` (cập nhật chỉ số current/max/%)

```c
case 8:
  FUN_0054f900(*(int *)gvar_007DA4EC,*(int *)(unaff_EBP + -0xc));   // case_054 dòng 91
  break;
```

Body: `functions/0054f900_FUN_0054f900.c:22-135`:

```c
if (*(uint *)(param_2 + -4) < 2) { iVar3 = _BoundErr(1); ... }   // dòng 58-63
local_d = *(byte *)(param_2 + iVar3);                            // RP[1] = slot index
if ((byte)(local_d - 1) < 5) {                                   // slot 1..5
  _LStrCopy(local_c,3,4,&local_14);                              // RP[2..5]
  uVar4 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30,local_14);   // → DWORD LE
  *(uint *)(local_8 + 100 + (slot+1)*8) = uVar4;                 // +100+(slot+1)*8 = A
  _LStrCopy(local_c,7,4,&local_18);                              // RP[6..9]
  uVar4 = FUN_0077ef7c(*(undefined4 *)gvar_007D9D30,local_18);   // → DWORD LE
  *(uint *)(local_8 + 0x68 + (slot+1)*8) = uVar4;                // +0x68+(slot+1)*8 = B
  ...
  if (A < 1) *(+0x90+(slot+1)*4) = 0;
  else       *(+0x90+(slot+1)*4) = B / A;                        // tỉ lệ %
}
```

- **Field layout SubOp 8** (P[0]=0x3D):

| Byte | Trường | Kiểu | Ghi vào |
| :-- | :-- | :-- | :-- |
| `P[0]` | MainOp `0x3D` | 1B | — |
| `P[1]` | SubOp `0x08` | 1B | — |
| `P[2]` | slot index | 1B (1..5) | chọn slot |
| `P[3..6]` | giá trị A | DWORD LE | `+100+(slot+1)*8` |
| `P[7..10]` | giá trị B | DWORD LE | `+0x68+(slot+1)*8` |

- **State change**: cập nhật mảng giá trị `A`, mảng `B`, và ô tỉ lệ `%` = `B/A` (0 nếu `A<1`).
- **Bounds**: check `RP ≥ 2` cho `RP[1]` (dòng 58-63); mọi truy cập sau dùng `_LStrCopy`/index **có kiểm biên nội bộ trong callee** (`_BoundErr` tại dòng 74-77, 82-85, 88-90, 99-101, 104-106, 111-113, 116-118, 122-124).
- **Wire tối thiểu để có ý nghĩa**: L = 11 (`[3D][08][slot][A 4B][B 4B]`).

### 4.7. `SubOp == 0x09` → `FUN_0054f764` (nạp 6 "ball")

```c
case 9:
  FUN_0054f764(*(int *)gvar_007DA4EC,*(int *)(unaff_EBP + -0xc));   // case_054 dòng 94
  break;
```

Body: `functions/0054f764_FUN_0054f764.c:20-79`:

```c
local_10 = 1;
do {
  ... bound-check RP[local_10] (RP[1..6]) ...
  if ((byte)(RP[local_10] - 1U) < 0x2a) {        // giá trị 1..42
    *(char *)(param_1 + (local_10 - 1) + 0xa8) = (char)uVar3;   // +0xa8.. = balls
  }
  local_10++;
} while (local_10 != 7);
FUN_0054fac0(param_1, ...);                      // hậu xử lý (body có trong SSOT)
```

- **Field layout SubOp 9** (P[0]=0x3D):

| Byte | Trường | Kiểu | Ghi vào |
| :-- | :-- | :-- | :-- |
| `P[0]` | MainOp `0x3D` | 1B | — |
| `P[1]` | SubOp `0x09` | 1B | — |
| `P[2..7]` | 6 "ball" | 6×1B (mỗi byte ∈ 1..42) | `+0xa8..+0xad` |

- **Bằng chứng "ball 1..42"**: `TMachineManager.Create` tạo các resource tên `"Ball01".."Ball42"` và `"LTable01"/"LTable02"` (`functions/0054f500_TMachineManager.Create.c:87-111`). Cận `0x2a` = 42 khớp đúng.
- **Bounds**: vòng lặp bound-check `RP[1..6]` → cần `RP` dài ≥ 6 (payload L ≥ 7) để đọc đủ; byte ngoài 1..42 **bị bỏ qua** (không ghi).
- **Wire tối thiểu**: L = 8 (`[3D][09][b1..b6]`).

### 4.8. `SubOp == 0x0A` & `0x0B` → `FUN_00728734` / `FUN_007287e4` (ĐÃ CÓ BODY — kênh tin nhắn text)

- **`FUN_00728734`** (`00728734_FUN_00728734.c:16–48`): `_LStrCopy(RP, 2, len(RP))` → **toàn bộ `RP[1..]` là text thô**; `_LStrCat3(msg, &DAT_007287C8, text)` (template prefix code-gap — chưa dump); `FUN_007ab870(gvar_007DA1B0^, 0, msg, 0)` → message. **Wire**: `[3D][0A][byte-string...]` — payload là text (encoding của text do server chọn — nhiều khả năng VISCII, chưa xác minh trực tiếp).
- **`FUN_007287e4`** (`007287e4_FUN_007287e4.c:16–92`): `k := RP[1]` (cần `len(RP) ≥ 2`), `v := RP[2]` (cần `≥ 3`), text = `RP[3..]` (`_LStrCopy(RP,4,len)` — 1-based, offset 3 0-based); `k == 1` → `msg = DAT_0072892C + IntToStr(v) + text + DAT_00728950 (+default 0x72885D)` gửi message; `k == 2` → tương tự với `DAT_00728974`; `k` khác → im lặng. **Wire**: `[3D][0B][k][v][text...]`.
- **Đính chính nhẹ**: bản cũ ghi "case 0xb không có `break` → fall-through xuống epilogue" — vẫn đúng nhưng **vô hại**: cả hai callee tự về epilogue sau khi gửi message (§4.8). `gvar_007D9C48^` **không được đụng tới** trong cả 3 SubOp — tham số chỉ là self thừa (xem §1).

### 4.9. Epilogue & artifact `switchD_00792e22`

- Dòng 102-130: `*in_FS_OFFSET = ...; _LStrArrayClr(...); _LStrClr(...); ... return;` — **dọn dẹp stack cục bộ** dùng chung cho mọi case (giống `case_047`/`case_048`/`case_050`). **Không phải logic nghiệp vụ.**
- `switchD_00792e22::caseD_0()` (dòng 51,55,…,83) là **artifact decompiler** trỏ tới epilogue dùng chung; nó buộc `return` sớm trong các nhánh `case 5` (khác `break`). **Không phải call-site thật.**

### 4.10. Danh tính các gvar đích — bằng chứng & giới hạn

| gvar | Nhóm hàm | Bằng chứng | Kết luận |
| :-- | :-- | :-- | :-- |
| `gvar_007D9F98` | `0054e2f8`, `0054f18c`, `0054f230`, `0054f818?` | **DÒNG GÁN: `0055374c_FUN_0055374c.c:25–27` — `TLottoManager_Create(VMT_54D580_TLottoManager,'\x01') → *(gvar_007D9F98)`; free type-4 `00553410.c:28-29`; tick/render mode-4 `00553840.c:32 → FUN_0054e51c`** | **`TLottoManager` (ĐÃ XÁC MINH)** — máy Xổ số, vị trí type 4. Widget đồng hệ: `TLottoMachineMain` (VMT `VMT_54D5F0`, ctor `FUN_0054f384` — tạo nút `"Btn_Lexit"` tại `0054f384.c:36-45`, caller `0051189c.c:1711`) |
| `gvar_007DA4EC` | `0054f818`, `0054f900`, `0054f764` | **DÒNG GÁN: `0055374c.c:47–49` — `TMachineManager_Create(VMT_54D6D8_TMachineManager) → *(gvar_007DA4EC)`; free type-6 `00553410.c:45-46`; tick `00553840.c:45 → FUN_0054fbac` (bộ 8 hàm vẽ: `0054fbf8/54fd38/55000c/550148/5503ac/55042c/55069c/55070c`)** | **`TMachineManager` (ĐÃ XÁC MINH — đúng như đoán cũ)** — type 6; ctor `0054f500_TMachineManager.Create.c` load `"machine"`, `"LTable01/02"`, `"Ball01".."Ball42"` (`:77,89,102`) |
| `gvar_007D9C48` | `00728598`, `00728734`, `007287e4` (KHÔNG dùng self); đọc rộng khắp (`FUN_00722508/0072288c/007228d4` tra entity theo ID — vd `0051fb6c.c:84`) | **grep `DAT_007D9C48 =` toàn `functions/*.c` 2026-09-14: KHÔNG có dòng gán creator** — chỉ có ghi field `*(gvar_007D9C48^+4) := 0` tại `00603f20.c:233` (reset out-world) | **vẫn UNKNOWN tên class**; chắc chắn là object tra-bảng theo ID toàn client, KHÔNG phải đích thực sự của SubOp 4/10/11 (các callee chỉ mượn con trỏ) |

- **Bằng chứng "họ máy"**: `functions/0054f500_TMachineManager.Create.c:27` định nghĩa `TMachineManager.Create`; hàm này tạo resource `"machine"`, `"LTable01/02"`, `"Ball01".."Ball42"` (`:77,89,102`).
- **ĐÍNH CHÍNH (2026-09-14)** dòng cũ "Không tìm thấy hàm khởi tạo gán 3 gvar — không thể chốt tên lớp": **Hàm creator tìm thấy**: `0055374c_FUN_0055374c.c:25–27,47–49` (dispatch theo mode trong OP 0x39) — type 4 = **`TLottoManager`**, type 6 = **`TMachineManager`** (đoán cũ chỉ đúng cho type 6 — type 4 là class anh em khác, không phải cùng `TMachineManager`). **Riêng `gvar_007D9C48` vẫn chưa có dòng gán** (grep `007d9c48` trên functions/*.c: chỉ đọc + 1 ghi field-free `00603f20.c:233`) → tên class `gvar_007D9C48` = UNKNOWN, cần redump tiếp.

---

## 5. Chuỗi VISCII → UTF-8 — **ĐÃ DUMP + ĐÃ DECODE (2026-09-14)**

`redump/lit_799254/7992d0/799300/79932c/799350/799384/7993d0/799410/799450.hex` hiện đã có (mỗi file 512B bắt đầu tại **đúng address của content** — con trỏ `DAT_007992xx` trỏ thẳng vào byte đầu của text). Decode bằng **VISCII** (`iconv -f VISCII`; cp1258 của Python cho kết quả vô nghĩa với các byte thanh điệu — xem ghi chú ở `opcode_3a.md §7`).

**Định dạng: const AnsiString Delphi, KHÔNG phải PChar thô** — ngay trước mỗi content (trong cửa sổ dump của chuỗi liền trước) có header `[FF FF FF FF][len:4 LE]`:

| # (`RP[1]`) | Content @ | Header | len (LE) | Bytes (hex, tới `00`) | Text (VISCII → UTF-8) |
| :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | `0x00799254` | `FF FF FF FF 70 00 00 00` @ `0x79924C` (thấy trong `lit_799200.hex`) | 112 | `54 68 B6 69 20 67 69 61 6E 20 F0 AF 69 20 63 68 ...` (112B) | **"Thời gian đối chiếu giải thưởng là ngày mở giải thưởng lúc 20:00 đến ngày mở giải thưởng lần sau là trước 18:00!"** |
| 2 | `0x007992D0` | `FF FF FF FF 25 00 00 00` @ `0x7992C8` | 37 | `42 D5 6E 20 6D 61 6E 67 20 74 68 65 6F 20 62 EA 6E 20 6D EC 6E 68 20 72 A4 74 20 6E 68 69 AB 75 20 74 69 AB 6E` | **"Bạn mang theo bên mình rất nhiều tiền"** |
| 3 | `0x00799300` | `FF FF FF FF 23 00 00 00` @ `0x7992F8` | 35 | `58 B1 20 73 AF 20 6B CF 20 6E E0 79 20 62 D5 6E 20 6B 68 F4 6E 67 20 74 72 FA 6E 67 20 74 68 DF B7 6E 67` | **"Xổ số kỳ này bạn không trúng thưởng"** |
| 4 | `0x0079932C` | `FF FF FF FF 1B 00 00 00` @ `0x799324` | 27 | `4B CF 20 6E E0 79 20 63 FB 6E 67 20 63 68 B0 61 20 6D B7 20 74 68 DF B7 6E 67 21` | **"Kỳ này cũng chưa mở thưởng!"** (byte `0xB0` — iconv VISCII render 'ồ'; theo ngữ cảnh là 'ư') |
| 5 | `0x00799350` | `FF FF FF FF 2B 00 00 00` @ `0x799348` | 43 | `D0 E3 20 74 D5 6D 20 6E 67 DF 6E 67 20 F0 A3 74 20 63 DF FE 63 2C 20 ...` | **"Đã tạm ngưng đặt cược, xin mời lần sau nha!"** |
| 6 | `0x00799384` | `FF FF FF FF 40 00 00 00` @ `0x79937C` | 64 | `D0 E3 20 6B 68 F4 6E 67 20 74 68 AC 20 F0 AF 69 20 63 68 69 AA 75 20 ...` | **"Đã không thể đối chiếu xổ số rồi, xin hãy đợi mở thưởng lần sau!"** |
| 7 | `0x007993D0` | `FF FF FF FF 36 00 00 00` @ `0x7993C8` | 54 | `58 69 6E 20 68 E3 79 20 F0 AF 69 20 63 68 69 AA 75 20 78 B1 20 73 AF ...` | **"Xin hãy đối chiếu xổ số kỳ trước rồi mới đặt cược nha!"** |
| 8 | `0x00799410` | `FF FF FF FF 36 00 00 00` @ `0x799408` | 54 | `4C FA 63 20 6E E0 79 20 6C E0 20 6D E1 79 20 64 75 79 20 74 72 EC ...` | **"Lúc này là máy duy trì thời gian tồn tại, xin chờ đợi!"** |
| 9 | `0x00799450` | `FF FF FF FF 2F 00 00 00` @ `0x799448` | 47 | `56 EC 20 62 E4 6E 67 20 76 A7 74 20 70 68 A6 6D 20 F0 E3 20 F0 A5 79 ...` | **"Vì bảng vật phẩm đã đầy, ngày khác bổ sung thêm"** |

- **Kết luận đóng goal cũ**: ~~"chưa xác định PChar hay AnsiString"~~ → **AnsiString const có header `[FF FF FF FF][len:4LE]` + content + `00`** (9/9 header quan sát trực tiếp — 8 trong `lit_799254.hex`, 1 trong `lit_799200.hex`; parse python offset = addr−base).
- **Ngữ nghĩa đã chốt**: bảng thông báo **máy Xổ số** — khớp `gvar_007D9F98 = TLottoManager` (§4.10): lịch đối chiếu thưởng (#1), ví tiền (#2), không trúng (#3), chưa mở thưởng (#4), tạm ngưng cược (#5), không đối chiếu được (#6), phải đối chiếu kỳ trước (#7), máy bảo trì (#8), túi đồ đầy (#9).
- Call-site ghi nhận cũ không đổi: `case_054_...c:50–82`; `jumptable_...cases.c:8917–8949`; `0078a89c...c:7024–7056`.

---

## 6. Chiều Client → Server

### 6.1. Vị trí

`functions/0077f414_FUN_0077F414.c:1044-1059` — `case 0x3d:` của `switch(param_2 & 0xff)`:

```c
case 0x3d:
  FUN_00402b90(&stack0xffffffc4,&stack0xffffffc8);
  _PStrNCat(&stack0xffffffc4,&stack0xffffffc0,2);
  FUN_00402b90(local_80,&stack0xffffffc4);
  _PStrNCat(local_80,&stack0xffffffc0,3);
  FUN_00402b90(local_d4,local_80);
  _PStrNCat(local_d4,&stack0xffffffc0,4);
  FUN_00402b90(local_dc,local_d4);
  _PStrNCat(local_dc,&stack0xffffffc0,5);
  FUN_00402b90(local_e4,local_dc);
  _PStrNCat(local_e4,&stack0xffffffc0,6);
  FUN_00402b90(local_410,local_e4);
  _PStrNCat(local_410,&stack0xffffffc0,7);
  _LStrFromString((int *)local_834,local_410);
  TForm1_CY_AddSedQueue(*(undefined4 *)gvar_007DA664,local_834[0]);
  break;
```

### 6.2. Dịch ngược từng bước (đối chiếu ASM)

ASM thật cùng case: `functions/0077f414_FUN_0077F414.asm.txt:3768-3854`; prologue lưu tham số ẩn tại **dòng 17-18** (`MOV [EBP-6],CL` = tham số thứ 3; `MOV [EBP-5],DL` = MainOp).

```asm
3768 LEA EAX,[EBP + -0x34]          ; buf A
3769 MOV DL,byte ptr [EBP + -0x5]   ; DL = MainOp (= 0x3D)
3770 MOV byte ptr [EAX + 0x1],DL    ; A[1] = 0x3D
3771 MOV byte ptr [EAX],0x1         ; A[0] = length 1  → shortstring [01][3D]
3772-3774                            ; B(-0x38) = copy(A)
3775 LEA EAX,[EBP + -0x3c]          ; buf C
3776 MOV DL,byte ptr [EBP + -0x6]   ; DL = CL (tham số 3 ẩn)
3777 MOV byte ptr [EAX + 0x1],DL    ; C[1] = CL
3778 MOV byte ptr [EAX],0x1         ; C[0] = length 1  → [01][CL]
3779-3782                            ; @PStrNCat(B, C, 2)   → B = [3D][CL]
3783-3785                            ; D(-0x7c) = copy(B)
3786 LEA EAX,[EBP + -0x3c]
3787 MOV EDX,[0x007d9f98]
3788 MOV EDX,[EDX]                  ; Self = *(gvar_007D9F98)
3789 MOV DL,byte ptr [EDX + 0x6a]   ; DL = Self.field_6a
3790 MOV byte ptr [EAX + 0x1],DL
3791 MOV byte ptr [EAX],0x1         ; C = [01][+0x6a]
3792-3795                            ; @PStrNCat(D, C, 3)   → D = [3D][CL][6a]
3799 MOV DL,byte ptr [EDX + 0x6b]   ; ... maxlen 4 → [3D][CL][6a][6b]
3812 MOV DL,byte ptr [EDX + 0x6c]   ; ... maxlen 5
3825 MOV DL,byte ptr [EDX + 0x6d]   ; ... maxlen 6
3838 MOV DL,byte ptr [EDX + 0x6e]   ; ... maxlen 7 → 7 byte
3848 LEA EDX,[EBP + 0xfffffbf4]
3849 LEA EAX,[EBP + 0xfffff7d0]
3850 CALL 0x0040402c                  ; @LStrFromString (bỏ byte độ dài)
3851-3854                            ; TForm1.CY_AddSedQueue(queue, payload)
```

### 6.3. Từng helper

- **`FUN_00402b90(dest, src)` — copy chuỗi dài lượng ngắn** (body: `functions/00402b90_FUN_00402b90.c:392-411`): copy đúng `*src + 1` byte (byte độ dài + nội dung). Ở đây dùng để nhân bản buffer shortstring qua từng bước.
- **`_PStrNCat(dest, src, maxLen)` — `@PStrNCat` @ `0x00402B60`** (RTL Borland, **không có body trong SSOT**): **nối (append) `src` vào `dest`, giới hạn độ dài đích = `maxLen`**. Vì `maxLen` tăng dần `2→3→4→5→6→7` và mỗi lần `src` là **1 byte khác nhau**, payload lớn dần đúng 1 byte mỗi bước.
- **`_LStrFromString(dest, src)` — `@LStrFromString` @ `0x0040402C`** (RTL, không body): chuyển **Pascal shortstring** (byte [0] = độ dài) sang **Delphi AnsiString** → bỏ byte độ dài.
- **`TForm1.CY_AddSedQueue(queue, payload)`** (body: `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181`): tính `L = _LStrLen(payload)`, mã hoá `L` thành **2 byte LE** qua `FUN_0077eb1c`, rồi ghép khung `[Token F4 44][L Word LE][payload]` và đẩy vào queue. → **payload case 0x3d chính là nội dung giữa Length và Token sau khi ghép**.

### 6.4. Wire C→S (suy ra)

```
Payload C→S = [0x3D][CL][ *(byte*)(*(gvar_007D9F98)+0x6a) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6b) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6c) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6d) ]
              [ *(byte*)(*(gvar_007D9F98)+0x6e) ]        (tổng 7 byte)
```

| Byte | Ý nghĩa | Nguồn |
| :-- | :-- | :-- |
| `P[0]` | `0x3D` (MainOp) | `[EBP-5] = DL` (asm dòng 18) |
| `P[1]` | `CL` — **byte tham số ẩn #3** (`SendCommand` chỉ hiện 2 tham số decompile) | `[EBP-6] = CL` (asm dòng 17) |
| `P[2]` | field `+0x6a` của `gvar_007D9F98` | asm 3787-3790 |
| `P[3]` | field `+0x6b` | asm 3800-3803 |
| `P[4]` | field `+0x6c` | asm 3813-3816 |
| `P[5]` | field `+0x6d` | asm 3826-3829 |
| `P[6]` | field `+0x6e` | asm 3839-3842 |

- **Đối xứng với S→C**: `gvar_007D9F98` chính là đối tượng nhận SubOp `1/2/3` ở chiều S→C (dòng 30/33/36). Vậy `0x3D` là **request (C→S) → response (S→C)** trên cùng đối tượng máy.
- **ĐÃ XÁC MINH (2026-09-14)** — 5 field `+0x6a..+0x6e` + `+0x6f` là **bộ chọn số của người chơi trên `TLottoManager`** (xem §6.5): `FUN_0054e3d4.c:56–82` thêm số khi chưa trùng & chưa đủ 5 (ghi `+0x6a+count`, tăng `+0x6f`), `FUN_0054e830.c:28–48` bỏ số (clear slot + giảm `+0x6f`), `FUN_0054f230` cmd 3 xóa `+0x6f/+0x68` (server reset), `FUN_0054e008.c:27–36` clear cả 5 + `+0x6f` sau khi gửi. `+0x68/+0x69` = counter/flag của mảng record SubOp 1 (§4.1) — **không ghi đè `+0x6a..0x6e`**.
- **`CL` và call-site** — **ĐÍNH CHÍNH (2026-09-14)**: bản cũ "grep `MOV DL,0x3D` → 0 hit" **nay có 1 hit**: `0054e4f0_FUN_0054e4f0.asm.txt:14` (`MOV CL,0x1; MOV DL,0x3d; CALL 0x0077f414`). Hàm `FUN_0054e4f0` (43B, mới decompile — `0054e4f0_FUN_0054e4f0.c:21–27`): guard `if (Self+0x6f == 5)` → gửi → `FUN_0054e008(Self)` clear; caller duy nhất `0x54db56` (vùng `TMachineManager`-lân-cận, chưa có hàm cha export — chi tiết trigger chưa kết luận được).

### 6.5. Ngữ nghĩa C→S 0x3D (xác minh từ body mới)

```
[0x3D][0x01][n1 n2 n3 n4 n5]   — gửi đúng khi 5 số đã chọn (obj+0x6f == 5)
                                  ni = byte tại obj+0x6a..+0x6e (mỗi số 1..42)
```
- **`+0x6a..+0x6e` = danh sách 5 số người dùng chọn** (0 = ô trống), **`+0x6f` = số lượng đang chọn**; các phép ghi chứng minh:
  - `FUN_0054e830.c:28–48` (bỏ chọn): `if (value-1 < 0x2a)` ⇒ **1..42**, tìm value trong `+0x6a..0x6e`, clear, `+0x6f--`.
  - `FUN_0054e3d4.c:56–82` (thêm chọn): chống trùng qua `+0x6a..`, ghi `+0x6a+count`, `+0x6f++`.
  - `FUN_0054e008.c:27–36` (reset): clear `+0x6a..0x6e` (5 phần tử) + `+0x6f := 0` — chạy NGAY SAU khi gửi 0x3D (bên trong `FUN_0054e4f0`).
  - `FUN_0054f230.c:35–43` (SubOp 3 cmd 3, S→C): server xóa `+0x6f/+0x68` ⇒ server điều khiển vòng chọn.
- Khớp ngữ nghĩa SubOp 9 (6 ball 1..42 vào `TMachineManager`) + 9 banner xổ số (§5) ⇒ **0x3D C→S = "submit 5 số đã chọn của vé Lotto"**; `CL=1` hằng.

---

## 7. Ghi chú cho Mock Server

### 7.1. S→C (server gửi xuống client)

Payload sau MainOp; khung ngoài = `[Token F4 44][Length L: Word LE][Payload]`, **toàn khung XOR 0xAD** (L tính từ byte MainOp `0x3D`):

```
[3D]                              L=1  → ERangeError (BoundErr(0))        — ĐỪNG GỬI
[3D][01]                          L=2  → ERangeError (BoundErr(1) trong FUN_0054e2f8)  — ĐỪNG GỬI
[3D][01][f1][b0..b4]              L=8  → thêm 1 bản ghi 5 byte vào gvar_007D9F98
[3D][02][D:4B LE]                L=6  → TLottoManager+0x16c := D              (§4.2)
[3D][03][cmd]                    L=3  → cmd 1: banner+obj+0x170:=1; cmd 2: banner; cmd 3: clear +0x6f/+0x68 + banner (§4.2)
[3D][04][n1..n5][m]              L=8  → message "5 số + 1 byte" vào kênh gvar_007DA1B0 (§4.3)
[3D][05]                          L=2  → ERangeError (BoundErr(1))        — ĐỪNG GỬI
[3D][05][01]                      L=3  → banner "Thời gian đối chiếu giải thưởng ... 20:00 ... trước 18:00!" (2200 ms)
[3D][05][02..09]                  L=3  → 8 banner Lotto còn lại (1200 ms) — text đầy đủ §5
[3D][05][xx>09]                   L=3  → no-op im lặng
[3D][06]                          L=2  → no-op im lặng (không có case 6)
[3D][07][x][v1..v5]               L=8  → record 5 byte +0xae/+0xaf trên TMachineManager (§4.5)
[3D][08][slot][A 4B LE][B 4B LE]  L=11 → cập nhật chỉ số current/max/% trên gvar_007DA4EC
[3D][09][b1..b6]                  L=8  → nạp 6 "ball" (1..42) vào gvar_007DA4EC
[3D][0A][text...]                 L≥3  → message text tự do + template 0x7287C8 (§4.8)
[3D][0B][k][v][text...]           L≥5  → message mẫu k=1/2 với số v (§4.8)
[3D][xx>=0C]                      L=2  → no-op im lặng
```

**Ví dụ khung hoàn chỉnh** (đã XOR `0xAD`):

```
# [3D][05][01] banner dài 2200 ms:
payload  = 3D 05 01
L (LE)   = 03 00
pre-XOR  = F4 44 03 00 3D 05 01
on-wire  = 59 E9 AE AD 90 A8 AC

# [3D][08][slot=1][A=100][B=200]:
payload  = 3D 08 01 64 00 00 00 C8 00 00 00
L (LE)   = 0B 00
pre-XOR  = F4 44 0B 00 3D 08 01 64 00 00 00 C8 00 00 00
on-wire  = 59 E9 A6 AD 90 A5 AC C9 AD AD AD 65 AD AD AD

# [3D][09][balls = 05 0A 0F 14 1A 2A]:
payload  = 3D 09 05 0A 0F 14 1A 2A
L (LE)   = 08 00
pre-XOR  = F4 44 08 00 3D 09 05 0A 0F 14 1A 2A
on-wire  = 59 E9 A5 AD 90 A4 A8 A7 A2 B9 B7 87
```

- **Bắt buộc**:
  - Không gửi `[3D]` (L=1) — ERangeError.
  - Với SubOp `01`, gửi đủ `L ≥ 8` để tránh `_BoundErr(1)` trong `FUN_0054e2f8`.
  - Không gửi `[3D][05]` (L=2) — ERangeError.
- **Nội dung 9 banner ĐÃ ĐỌC ĐƯỢC (§5)**: mock có thể kiểm tra text client hiển thị theo `RP[1] = 01..09`; duration 2200/1200 ms.
- **Cả 10 SubOp đều có cấu trúc wire xác minh được từ body (§3/§4)** — không còn nhánh "chỉ gửi ≥2 byte để dò"; 4 chuỗi message-template của SubOp 4/10/11 (`0x7286F4/7287C8/72885D/72892C/728950/728974`) và 3 chuỗi banner SubOp 3 (`0x54F2EC/54F320/54F340`) vẫn **chưa dump** → text hiển thị của các nhánh đó chưa kiểm được.

### 7.2. C→S (client gửi lên — server phải nhận)

```
Payload nhận được (sau khi bỏ Token/Length và giải XOR):
  [0x3D][0x01][n1 n2 n3 n4 n5]        (7 byte)   — 5 số đã chọn, mỗi số 1..42
```

- Server nên **đọc `0x3D` là "nộp vé 5 số của máy Lotto"** (§6.5): `CL` **chốt = 0x01** (hằng trong sender `FUN_0054e4f0`), 5 byte = `TLottoManager+0x6a..+0x6e` — mỗi ô `0` = chưa chọn, và gói chỉ được gửi khi cả 5 ô đã đầy (`+0x6f == 5`); client tự clear `+0x6a..+0x6e` ngay sau khi gửi (`FUN_0054e008`).
- Response hợp lệ: `[3D][09][6 ball]` quay số, `[3D][01|02|08]` cập nhật bảng/xỉ số, `[3D][05][k]` banner lý do (§5), `[3D][03][3]` reset vòng chọn.
- **Không có tiền tố độ dài cho chuỗi**: payload cố định 7 byte do khung `L` quyết định.

---

## 8. Source trail

| # | Nguồn (trong `ts_decompile/`) | Dùng để |
| :-- | :-- | :-- |
| 1 | `redump/jumptable_byte200_0x78A8EE.hex` (hàng 4, index `0x3D` = `0x36` = 54) | Byte table MainOp 0x3D → 54 |
| 2 | `redump/jumptable_dword200_0x78A9B6.hex` (hàng 7, entry#54 = `DC 57 79 00`); `redump/jumptable_0x78A9B6_case_functions.csv:56`; `case_functions/manifest.csv:56` | Dword table entry #54 → `0x007957DC` |
| 3 | `case_functions/functions/case_054_007957DC_FUN_007957dc.c:21-101` (handler, 133 dòng) | Lõi logic S→C |
| 4 | `case_functions/jumptable_0x78A9B6_cases.c:8869-8998` | Bản gộp case 54 |
| 5 | `functions/0078a89c_FUN_0078a89c.c:6987-7081` | Bản inline dispatcher — xác nhận 1:1 |
| 6 | `functions/0054e2f8_FUN_0054e2f8.c:41-99` | Body SubOp 1 (thêm bản ghi 5 byte) |
| 7 | `functions/0054f900_FUN_0054f900.c:54-134` + `:71-72,79-80` (`FUN_0077ef7c`) | Body SubOp 8 (current/max/%) |
| 8 | `functions/0054f764_FUN_0054f764.c:36-78` | Body SubOp 9 (6 "ball" 1..42) |
| 9 | `functions/0054f500_TMachineManager.Create.c:27,77,87-111` | Bằng chứng họ `TMachineManager` + `"Ball01".."Ball42"` |
| 10 | `functions/00553410_FUN_00553410.c:28-29,45-46`; `functions/00553840_FUN_00553840.c:32,45`; `functions/0055391c_FUN_0055391c.c:32,45` | `gvar_007D9F98` = machine type 4; `gvar_007DA4EC` = machine type 6 |
| 11 | `functions/007281ec_FUN_007281ec.c:56-60` | `gvar_007D9C48` = object UI tra cứu entity theo ID 4B (họ `FUN_0072xxxx`) |
| 12 | ~~6 callee trong khe trống~~ **NAY ĐÃ DECOMPILE**: `functions/0054f18c/0054f230/0054f818/00728598/00728734/007287e4_FUN_*.c` (+`0054f384`, `0054e4f0`, `0054e008/0054e3d4/0054e830`) | Wire SubOp 2/3/4/7/10/11 + cơ chế chọn số C→S (§4, §6.5) |
| 13 | ~~9 literal banner chưa dump~~ **ĐÃ DUMP**: `redump/lit_799254/7992d0/799300/79932c/799350/799384/7993d0/799410/799450.hex` (+ `lit_799200.hex` chứa header của #1) | Decode VISCII đầy đủ §5 |
| 14 | **DÒNG GÁN gvar tìm thấy**: `functions/0055374c_FUN_0055374c.c:25–27` (`TLottoManager_Create → gvar_007D9F98`), `:47–49` (`TMachineManager_Create → gvar_007DA4EC`); grep `007d9c48` toàn `functions/*.c` — vẫn không có creator | Chốt tên 2/3 class; `gvar_007D9C48` vẫn unknown |
| 15 | `functions/0077f414_FUN_0077F414.c:1044-1059` + `.asm.txt:17-18,3768-3854`; **bảng đỏ `redump/table_0x77F474_200B.hex` byte[0x3D]=`0x33`, `table_0x77F53C_dword200.hex` dword[0x3D]=`0x0078A318`** | Chiều C→S (case 0x3d) — payload 7 byte; builder xác nhận bằng bảng |
| 16 | `functions/00402b90_FUN_00402b90.c:392-411`; **`functions/00402b60__PStrNCat.c:584–606` (body RTL mới — xác minh `_PStrNCat` nối `min(src[0], maxLen−dest[0])` ký tự data, không copy length byte)** | Giải thích từng bước builder §6.2 |
| 17 | `functions/0051633c_TForm1.CY_AddSedQueue.c:141-181` | Đóng gói khung `[Token][L 2B LE][payload]` bằng `FUN_0077eb1c` |
| 18 | `functions/0051189c_FUN_0051189c.c:1265-1277` | `gvar_007DA084` = `TSe_TalkMsgFormPlus` |
| 19 | ~~grep `MOV DL,0x3D` → 0 hit~~ **NAY 1 HIT**: `functions/0054e4f0_FUN_0054e4f0.asm.txt:12–15` (`CL=1`); caller `0054e4f0.c` header: `0x54db56` | Call-site C→S 0x3D + `CL` chốt (§6.4–6.5) |

### Đã ĐÓNG (2026-09-14)
- ~~1. Thân 6 callee~~ **XONG** (§4.2/4.3/4.5/4.8) — wire SubOp 2/3/4/7/10/11 khôi phục; `0054f818` là bản sao SubOp 1 trên `TMachineManager` (`+0xae/+0xaf`, mảng 5B/slot).
- ~~2. Tên lớp `gvar_007D9F98`/`gvar_007DA4EC`~~ **XONG**: `TLottoManager` / `TMachineManager` (`0055374c.c:25–27,47–49`). **Riêng `gvar_007D9C48` vẫn chưa có creator**.
- ~~3. 9 literal banner~~ **XONG** — AnsiString header đầy đủ, text VISCII tại §5.
- ~~5. Người ghi `+0x6a..+0x6e`~~ **XONG** — UI Lotto `FUN_0054e3d4/0054e830` (§6.5), không phải SubOp 2/3.
- ~~6. `CL`/call-site 0x3D~~ **XONG** — `FUN_0054e4f0`, `CL = 1` (§6.4).
- **Ý nghĩa SubOp 5** (9 banner): **XONG** — bảng thông báo máy Xổ số (§5).
- **`FUN_0054f384`** (mới, không phải handler): constructor widget **`TLottoMachineMain`** (`VMT_54D5F0_TLottoMachineMain`, `0051189c.c:1711`) — tạo nút `"Btn_Lexit"` (`0054f384.c:41–45`), handler `LAB_0054f490`; bằng chứng đồng họ Lotto cho `gvar_007D9F98`.

### Điểm chưa xác minh được (cập nhật)
1. **Ngữ nghĩa 5 byte `RP[2..6]` của SubOp 1** và **2 dword `A`/`B` của SubOp 8** (chỉ biết ghi vào mảng trạng thái): **chưa có tên trường**.
2. **Thân method `vtable+0x90` của `TSe_TalkMsgFormPlus`** (banner): **VẪN chưa có trong SSOT** — không có dump VMT `0x63B39C` (redump chỉ có `vmt_613160_TGmManage.hex` + `vmt_75C0B4_TLifeManage.hex`); các hàm `0063b494/0063b4cc/0063bac8/0063bb3c` chưa map được vào slot. Giữ nguyên ghi chú §4.4.
3. **9 chuỗi message/banner còn lại**: templates `0x7286F4/7287C8/72885D/72892C/728950/728974` (SubOp 4/10/11) + `0x54F2EC/54F320/54F340` (SubOp 3) — code-gap, chưa dump → text hiển thị của các nhánh này chưa đọc được.
4. **Class `gvar_007D9C48`** + giá trị `0`/`6` của SubOp 11 (chỉ 1/2 có nhánh).
5. **Ý nghĩa từng mã SubOp 3** (ba banner 1200 ms của `TLottoManager`) — shape đã biết, text thiếu.
6. **`FUN_0054f230` cmd 1 ghi `+0x170 := 1`** dùng để làm gì (field đọc lại ở đâu): chưa lần ra trong export.
