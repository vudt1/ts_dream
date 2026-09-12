# PHÂN TÍCH — Main OP 0x24 (36) / Case 32 / `FUN_0079346B` @ `0x0079346B`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). `switch(SubOp)` đủ `0x01..0x19` (25 nhánh), không khuyết, không default.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh **fan-out điều khiển UI/manager** — không phải kênh số (không cộng/trừ như OP 0x1A/0x23), không phải chat-log (không gọi `FUN_007ab870`, không chạm `gvar_007DA1B0`).
- 2 họ hành vi:
  - **Exec / setter cố định (15 nhánh)**: `0x01,03,0x0A,0x0B,0x0D,0x0F,0x10,0x11,0x12,0x14,0x15,0x16,0x17,0x18` — gọi `VMT+0x20` hoặc `func_0x00537d90(...,1,0)`, không đọc wire (trừ `0x0E` đọc 1 byte + pad).
  - **Passthrough blob (8 nhánh)**: `0x02,04,05,06,08,09,0x0C,0x19` — trao nguyên `RP` (gồm cả byte SubOp) cho hàm con họ `005e/0053/0052/0051`; parser nằm trong hàm con. `0x02` đặc biệt: copy `P[2..end]` vào `*(gvar_007DA07C)+0x138` rồi gọi `FUN_005e0808`.
- 3 nhánh **đọc field thật ở tầng handler**: `0x07` (clear cờ + banner có điều kiện kép 2 byte), `0x0E` (1 byte + pad), `0x13` (flag 1B + DWORD LE).
- Text duy nhất là hằng `UNK_00798A50` (SubOp 0x07, banner 2000ms). Wire không mang chuỗi biến dài.
- Chiều C→S `case 0x24: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x24 (36) → byte_table[0x78A8EE][0x24] = 0x20 (32)
                 → dword_table[0x78A9B6][32] @ 0x0078AA36 = 0x0079346B
                 → FUN_0079346b (Case 32)
```

- File chính: `ts_decompile/case_functions/functions/case_032_0079346B_FUN_0079346b.c` (188 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5564-5719` (`case 0x24:`) — khớp 1:1.
- Manifest: `case_functions/manifest.csv:34` + `jumptable_0x78A9B6_case_functions.csv:34` (`32,0x0078AA36,0x0079346B`).
- Framing/XOR/pump như `opcode_00_01.md` §2. `.asm.txt` chỉ còn prologue nên mapping xác nhận bằng `.hex`/`.csv` + manifest + bản inline.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x24`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 29-36)

```c
if (*(RP-4)==0) _BoundErr(0);   // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
switch(SubOp){ case 1: ... case 0x19: ... }
// Không default → 0x00 / ≥0x1A no-op (chỉ epilogue _LStrArrayClr/_LStrClr cuối hàm)
```

### 2.4. Codec dùng trong OP

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | **Có gọi 1 chỗ**: SubOp `0x13`, 4B → DWORD LE |
| `FUN_0077eb9c` / `FUN_0077f098` | Không gọi trong Case 32 (đối tác codec chung / double) |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi ở chiều S→C |
| `_LStrCopy(RP,2,len-1)` | SubOp `0x02`: cắt `P[2..end]` vào `+0x138` |
| `_LStrCopy(RP,3,4)` | SubOp `0x13`: cắt `P[3..6]` decode DWORD |
| Banner `(VMT+0x90)(...,2000,0,0)` | SubOp `0x07` qua `*gvar_007DA084` |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[24][01]` (2B) | không đọc | `(***gvar_007DA07C+0x20)()` |
| `0x02` | `[24][02][data...]` (biến dài) | `_LStrCopy(RP,2,len-1)` → `*(007DA07C)+0x138` | Ghi blob + `FUN_005e0808(*gvar_007DA07C)` |
| `0x03` | `[24][03]` | không đọc | `(***gvar_007DA128+0x20)()` |
| `0x04` | `[24][04][blob...]` | passthrough `RP` | `func_0x005e1c34(*gvar_007DA128, RP)` |
| `0x05` | `[24][05][blob...]` | passthrough | `func_0x005e2270(*gvar_007DA128, RP)` |
| `0x06` | `[24][06][blob...]` | passthrough | `func_0x005e20b4(*gvar_007DA128, RP)` |
| `0x07` | `[24][07][A:1B][B:1B]` (4B) | `*(007DA128+0x158)=0`; `A=RP[1]` (len≥2), `B=RP[2]` (len≥3) | `A==01 && B==07` → banner `UNK_00798A50` 2000ms; còn lại im lặng |
| `0x08` | `[24][08][blob...]` | passthrough | `func_0x005e3ed8(*gvar_007DA650, RP)` |
| `0x09` | `[24][09][blob...]` | passthrough | `func_0x0053c87c(*gvar_007DA098, RP)` |
| `0x0A` | `[24][0A]` | không đọc | `(***gvar_007DA098+0x20)()` |
| `0x0B` | `[24][0B]` | không đọc | `func_0x00537d90(*gvar_007D9C8C,1,0)` |
| `0x0C` | `[24][0C][blob...]` | passthrough | `func_0x0052e950(*gvar_007DA2B4, RP)` |
| `0x0D` | `[24][0D]` | không đọc | `(***gvar_007D9ED0+0x20)()` |
| `0x0E` | `[24][0E][v:1B][pad:1B]` (4B) | `v=RP[1]`; guard kép `len≥3` + `len≥2` | `func_0x00537d90(*gvar_007D9C8C, v)` (pad bỏ) |
| `0x0F` | `[24][0F]` | không đọc | `(***gvar_007DA164+0x20)()` |
| `0x10` | `[24][10]` | không đọc | `(***gvar_007D9F34+0x20)()` |
| `0x11` | `[24][11]` | không đọc | `(***gvar_007DA63C+0x20)()` |
| `0x12` | `[24][12]` | không đọc | `(***gvar_007DA13C+0x20)()` |
| `0x13` | `[24][13][flag:1B][A:4B LE]` (7B) | `flag=RP[1]`; `A=FUN_0077ef7c(_LStrCopy(RP,3,4))` = `P[3..6]` | `func_0x00535310(*gvar_007DA604, flag, A)` |
| `0x14` | `[24][14]` | không đọc | `(***gvar_007DA114+0x20)()` |
| `0x15` | `[24][15]` | không đọc | `(***gvar_007DA1AC+0x20)()` |
| `0x16` | `[24][16]` | không đọc | `(***gvar_007DA188+0x20)()` |
| `0x17` | `[24][17]` | không đọc | `(***gvar_007DA75C+0x20)()` |
| `0x18` | `[24][18]` | không đọc | `(***gvar_007DA1E8+0x20)()` |
| `0x19` | `[24][19][blob...]` | passthrough | `FUN_0051d914(*gvar_007DA1EC, RP)` |
| `0x00`,`≥0x1A` | — | — | no-op |

---

## 4. Chi tiết các nhánh đáng chú ý (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x02` — Ghi blob + refresh

```c
_LStrCopy(RP,2,LStrLen(RP)-1, *(007DA07C)+0x138); // P[2..end], tối thiểu [24][02] (copy rỗng)
FUN_005e0808(*gvar_007DA07C);
```
Wire biến dài, lấy tới hết, không cắt cố định.

### 4.2. SubOp `0x07` — Clear cờ + banner điều kiện kép

```c
*(007DA128+0x158) = 0;              // luôn clear trước
if (RP[1]==0x01 && RP[2]==0x07)     // guard len≥2 rồi len≥3
  banner(UNK_00798A50, 2000,0,0);
```
Đủ 4B `[24][07][01][07]` mới nổ banner; các cặp khác im lặng nhưng vẫn giữ clear.

### 4.3. SubOp `0x0E` — Set byte có pad

- Guard kép `len≥3` (`BoundErr(2)`) rồi `len≥2` (`BoundErr(1)`), đọc `v=RP[1]`, gọi `func_0x00537d90(*gvar_007D9C8C, v)`. Thực tế cần 4B `[24][0E][v][pad]`, pad bỏ (pattern giống OP 0x1F SubOp 03/05).

### 4.4. SubOp `0x13` — Nhánh số duy nhất

- `flag=RP[1]` + `A = FUN_0077ef7c(P[3..6])` (7B payload), gọi `func_0x00535310(*gvar_007DA604, flag, A)`. Thiếu byte trong A → `BoundErr` trong codec.

### 4.5. Các exec tĩnh / passthrough còn lại

- Exec `(VMT+0x20)`: 01, 03, 0A, 0D, 0F, 10, 11, 12, 14–18 (mỗi nhánh 1 object riêng, xem bảng §3).
- `0x0B`: hằng `func_0x00537d90(...,1,0)`; passthrough: 04/05/06 (cụm `007DA128`), 08 (`007DA650`), 09 (`007DA098`), 0C (`007DA2B4`), 19 (`007DA1EC` — `FUN_0051d914` có body). Parser nằm trong hàm con chưa phục hồi.

---

## 5. Chuỗi VISCII → UTF-8

- Không có payload text trên dây; nhánh số duy nhất chỉ mang số; blob do hàm con parse.
- Literal duy nhất `UNK_00798A50` (banner SubOp 07). `redump/` không có dump cho địa chỉ này → **chưa decode được**. Cần redump `.rodata` tại `0x00798A50`.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:968-969`: `case 0x24: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x24 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[24][01] / [03] / [0A] / [0D] / [0F] / [10] / [11] / [12] / [14]–[18]  exec tĩnh (2B)
[24][02][blob...]            blob P[2..end] → +0x138 + refresh
[24][04|05|06|08|09|0C|19][blob...]  passthrough (format do hàm con)
[24][07][01][07]             banner 798A50 2000ms; cặp khác im lặng (vẫn clear)
[24][0B]                     func(...,1,0)
[24][0E][v][pad]             func(...,v), đủ 4B
[24][13][flag][A u32LE]      đủ 7B
ĐỪNG GỬI: [24][00], ≥0x1A (no-op); L=1 (RangeError).
```

Độ dài: exec tĩnh 2B; Sub 07 cần `len(RP)≥3`; Sub 0E cần 4B payload; Sub 13 cần đủ DWORD `P[3..6]`; Sub 02 lấy tới hết.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_032_0079346B_FUN_0079346b.c` (188 dòng) | Handler chính đủ 25 nhánh |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5564-5719` | Bản inline đối chiếu 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x24]=0x20`) + `jumptable_dword200_0x78A9B6.hex` (`dw[32]=0x79346B`) + `.csv:34` + `manifest.csv:34` | Mapping tự parse |
| 4 | `functions/0077ef7c / 0077eb9c / 0077eb1c / 0077ee84 / 0077f098` | Codec (chỉ `EF7C` gọi ở Sub 0x13) |
| 5 | `functions/0077f414_FUN_0077F414.c:968-969` | C→S rỗng |
| 6 | `jumptable_0x78A9B6_cases.c:6557` + grep `00798a50` | Literal duy nhất, xác nhận thiếu dump |
