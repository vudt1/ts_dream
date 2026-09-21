# PHÂN TÍCH — Main OP 0x23 (35) / Case 31 / `FUN_00792BAD` @ `0x00792BAD`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). File case 338 dòng, switch khuyết `case 5` — ghi nhận nguyên trạng.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh notice/số + chat-log + ghi state. `switch(SubOp)` có **10 nhánh**: `0x01,02,03,04,06,07,08,09,0A,0x0B` — **khuyết `0x05`** (no-op), `0x00`/`≥0x0C` no-op, không default.
- SubOp `0x01/02/03`: banner tĩnh 2000ms theo mã `K = RP[1]` (6/4/8 nhánh; nhánh 01/K=1 ghi thêm `player+0x648/0x64C`, nhánh 02/K=1 gọi `FUN_00766360` ×2 duyệt file).
- SubOp `0x04`: nhánh phức tạp nhất — `A=P[2..5]` + `B=P[6..9]` DWORD LE, `D=P[10..17]` 8B → double; banner 10000ms `nhãn + IntToStr(A+B) [+ format(D) nếu D!=0]` + ghi 1 dòng chat-log kênh 0 + `player+0x146C = A*100+B`.
- SubOp `0x06`: ẩn form (`FUN_005afd98`); `0x07`: ghi 2 mẫu chat-log tĩnh theo K=1/2; `0x08`: banner số thực nếu `D!=0`; `0x09`: banner `A+B` kèm/không kèm `D`; `0x0A`: chat-log số thực, delimiter = byte thấp của `D`; `0x0B`: exec object `(**gvar_007DA15C+0x20)()`.
- Mọi text hiển thị là **hằng `.rodata`** (`DAT_007985f0…DAT_007989ec`, ~30 địa chỉ) — wire chỉ mang số (DWORD/double/byte mã), không có field chuỗi biến dài.
- Chiều C→S `case 0x23: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x23 (35) → byte_table[0x78A8EE][0x23] = 0x1F (31)
                 → dword_table[0x78A9B6][31] @ 0x0078AA32 = 0x00792BAD
                 → FUN_00792bad (Case 31)
```

- File chính: `ts_decompile/case_functions/functions/case_031_00792BAD_FUN_00792bad.c` (338 dòng, header Case 31)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5306-5563` (`case 0x23:`) — khớp 1:1 (lộ `0x33323106`, `0.0`, thứ tự `_LStrCatN`).
- Framing/XOR/pump như `opcode_00_01.md` §2. `.asm.txt` chỉ còn prologue nên mapping xác nhận bằng `.hex`/`.csv` + manifest + bản inline.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x23`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.

### 2.3. Đọc SubOp (dòng 45-53)

```c
if (*(RP-4)==0) _BoundErr(0);       // RP rỗng (L=1) → RangeError
SubOp = (uint)*(byte*)(RP + 0);     // RP[0] = P[1], 1 byte thường
switch(SubOp){ case 1,2,3,4,6,7,8,9,10,0xB ... }
// Không case 5, không default → 0x00/0x05/≥0x0C no-op (chỉ epilogue)
```

- Khối `_LStrArrayClr/_LStrClr` cuối hàm (dòng 307-335) là epilogue chung dispatcher.
- Biến `bVar5 = 0` từ dòng 45 không bao giờ đổi → artefact (mọi index nhân với nó đều = 0, tức ghi đúng 1 slot).

### 2.4. Codec & API dùng trong OP

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ef7c` | 4B → **DWORD LE** (SubOp 4, 9) |
| `FUN_0077f098` | 8B chunk → **double** qua FPU (SubOp 4, 8, 9, 10); hằng so sánh `_DAT_007988ac` = `0.0` |
| `FUN_0077eb9c` | **Không gọi** trong Case 31 |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi ở chiều S→C này |
| `IntToStr` | DWORD → chuỗi thập phân |
| `FUN_007c56c8` | Format double → AnsiString |
| `FUN_007ab870(form, channel, text, delim)` | Nạp 1 dòng vào chat-log `TTalkMsgForm` (`*gvar_007DA1B0`); channel `0` hoặc `*(player+4)`; delim `'\n'` hoặc byte thấp của double (SubOp 0A) |
| Banner `(VMT+0x90)(form,text,ms,0,0)` | Banner/marquee `TSe_TalkMsgFormPlus`, ms = 2000 (Sub 1–3) hoặc 10000 (Sub 4/8/9) |
| `FUN_00766360` | Duyệt file theo pattern (`FindFirst/Next/Close`); chỉ gọi ở SubOp 02/K=1 |
| `FUN_005afd98` | Ẩn form (`p+0x43=1, p+0x33=0`, Hide 2 control, VMT+0x20); gọi với `*gvar_007D9CC0` ở SubOp 06 |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[23][01][K:1B]` (3B, K=0–5) | `K=RP[1]`, guard `len>=2` | Banner tĩnh 2000ms; K=1 ghi thêm `player+0x648/0x64C` |
| `0x02` | `[23][02][K:1B]` (3B, K=1–4) | `K=(char)RP[1]` | Banner tĩnh; K=1 + `FUN_00766360`×2 |
| `0x03` | `[23][03][K:1B]` (3B, K=1–8) | `K=RP[1]` | Banner tĩnh 8 nhánh |
| `0x04` | `[23][04][A:4B][B:4B][D:8B]` (18B) | `_LStrCopy(RP,2,4/6,4)`+`EF7C`, `_LStrCopy(RP,10,8)`+`F098` | Banner+chat-log `A+B [+Fmt(D)]` 10000ms + `player+0x146C=A*100+B` |
| `0x06` | `[23][06]` (2B) | không đọc | Ẩn form `*gvar_007D9CC0` |
| `0x07` | `[23][07][K:1B]` (3B, K=1/2) | `K=RP[1]` | Chat-log dòng tĩnh `7988e0/798900` (kênh `*(player+4)`) |
| `0x08` | `[23][08][D:8B]` (10B) | `_LStrCopy(RP,2,8)`+`F098` | `D!=0` → banner `nhãn+Fmt(D)` 10000ms; `D==0` → no-op |
| `0x09` | `[23][09][A:4B][B:4B][D:8B]` (18B, đọc D trước) | như SubOp 04 | Banner `A+B [+Fmt(D)]` 10000ms (2 nhánh D==0 / !=0) |
| `0x0A` | `[23][0A][D:8B]` (10B) | `_LStrCopy(RP,2,8)`+`F098` | Chat-log `nhãn+Fmt(D)`, delim = byte thấp của D |
| `0x0B` | `[23][0B]` (2B) | không đọc | `(***gvar_007DA15C+0x20)()` |
| `0x00`,`0x05`,`≥0x0C` | — | — | no-op |

---

## 4. Chi tiết từng SubOp (core logic, bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Banner 6 nhánh + ghi field ở K=1

- `K=0`: banner `DAT_007985f0`. `K=1`: banner `DAT_0079860c` + ghi `*(player+0x648)=0x33323106` (bytes `06 33 32 31` thô từ `.rodata`, bản inline dòng 5323) + `+0x64C/0x64E` (index ×`bVar5=0` → đúng 1 slot). `K=2/3/4/5`: banner `00798630/4c/6c/84`.

### 4.2. SubOp `0x02` — Banner 4 mode + reload file ở K=1

- `K=1`: banner `007986c4` + `FUN_00766360(*gvar_007D9CC8)` + `FUN_00766360(*gvar_007DA70C)`. `K=2/3/4`: banner `007986ec/00798718/00798748`. Khác: no-op.

### 4.3. SubOp `0x03` — Banner 8 nhánh, không field

- `K=1..8` → `0079876c/88/a4/c4/dc/f8/8824/8864`, đều banner 2000ms. Không đọc thêm, không ghi state.

### 4.4. SubOp `0x04` — Số cộng + chat-log + ghi `player+0x146C`

1. `S=IntToStr(A+B)` (check tràn `SCARRY4/_IntOver`).
2. `Msg = DAT_0079889c + S`.
3. Nếu `D!=0.0`: `T=FUN_007c56c8(D)`, `Msg = Msg + ... + T` (`_LStrCatN(Msg,4)` — Ghidra mất 3 varargs, chỉ biết tổng 4 mảnh).
4. `banner(Msg,10000)` + `FUN_007ab870(chatlog, 0, Msg, '\n')` — vừa banner vừa chat-log kênh 0.
5. `player+0x146C = A*100+B` (check tràn `*100`).

### 4.5. SubOp `0x06` — Ẩn form

- `[23][06]`, `FUN_005afd98(*gvar_007D9CC0)`.

### 4.6. SubOp `0x07` — 2 mẫu chat-log

- `K=1` → `FUN_007ab870(chatlog, *(player+4), DAT_007988e0, '\n')`; `K=2` → `DAT_00798900`. Không banner.

### 4.7. SubOp `0x08` — Banner số thực có điều kiện

- `D==0.0` → no-op; `D!=0.0` → `Msg = DAT_00798978 + FUN_007c56c8(D)`, banner 10000ms.

### 4.8. SubOp `0x09` — Banner tổng A+B kèm/không kèm D

- `D==0.0`: `Msg = DAT_007988b8 + IntToStr(A+B) + 0x7988c4`, banner 10000ms.
- `D!=0.0`: thêm `FUN_007c56c8(D)`, `_LStrCatN(5 mảnh)`, banner 10000ms.

### 4.9. SubOp `0x0A` — Chat-log số thực, delimiter động

- `Msg = DAT_007989ec + FUN_007c56c8(D)`; `FUN_007ab870(chatlog, 0, Msg, (char)D)` — delim là byte thấp của double.

### 4.10. SubOp `0x0B` — Exec object

- `[23][0B]`, `(***gvar_007DA15C+0x20)()`. Không đọc payload.

---

## 5. Chuỗi VISCII → UTF-8

- **Không decode được từ source cho phép, và không có text trên dây**: mọi text là hằng `.rodata` (`DAT_007985f0…DAT_007989ec`, ~30 địa chỉ); `redump/` không có `lit_7985xx/7986xx/7988xx/7989xx.hex` nên không có byte nguồn.
- Wire chỉ mang số (DWORD/double/byte mã); chuỗi ghép thêm đều sinh từ số (`IntToStr`, `FUN_007c56c8`). Không có field chuỗi biến dài.
- Cần redump `.rodata` `0x007985F0–0x007989EC` mới đọc được banner/chat-log tĩnh.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:966-967`: `case 0x23: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x23 S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[23][01][K 00–05]                 banner tĩnh (K=01 ghi player+0x648/0x64C)
[23][02][K 01–04]                 banner tĩnh (K=01 + reload file ×2)
[23][03][K 01–08]                 banner tĩnh
[23][04][A u32LE][B u32LE][D 8B]  banner+chat-log + player+0x146C=A*100+B
[23][06]                          ẩn form
[23][07][K 01|02]                 chat-log tĩnh (kênh *(player+4))
[23][08][D 8B]                    D!=0 → banner; D==0 → im lặng
[23][09][A u32LE][B u32LE][D 8B]  banner(A+B [+Fmt(D)])
[23][0A][D 8B]                    chat-log, delim = byte thấp D
[23][0B]                          exec object
ĐỪNG GỬI: [23][05], [23][00], SubOp ≥0x0C (no-op); L=1 (RangeError).
```

Độ dài chính xác: Sub 1/2/3/7 cần `len(RP)>=2`; Sub 8/10 đủ 8B từ `P[2]`; Sub 4/9 đủ 18B payload. Thừa byte bị cắt cố định.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_031_00792BAD_FUN_00792bad.c` | Handler chính toàn file |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5306-5563` | Bản inline: `0x33323106`, `0.0`, thứ tự CatN |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x23]=0x1F`) + `jumptable_0x78A9B6_case_functions.csv:33` + `manifest.csv:33` | Mapping `0x23→31→0x00792BAD` |
| 4 | `functions/0077ef7c / 0077f098 / 0077eb9c / 0077eb1c / 0077ee84` | Codec DWORD/double/Word + builder C→S |
| 5 | `functions/007ab870 / 007c56c8 / 00766360 / 005afd98` | Chat-log, format double, duyệt file, ẩn form |
| 6 | `functions/0077f414_FUN_0077F414.c:966-967` | C→S rỗng |

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x00792BAD` — C `client_pseudo_c/case_031_00792BAD_FUN_00792bad.c` (có); asm `client_pseudo_c/00792bad_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x23` — 23 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| `1` | `0x00792C0C` | Sửa đổi thất bại | Sửa đổi thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x00792C0C` | Sửa đổi thành công | Sửa đổi thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x00792C0C` | Mật mã cũ sai lầm | Mật mã cũ sai lầm. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x00792C0C` | Mã cá nhân cũ sai lầm | Mã cá nhân cũ sai lầm. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x00792C0C` | Mật mã quá ngắn | Mật mã quá ngắn. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `1` | `0x00792C0C` | Không thể sửa đổi mật mã và mã cá nhân trên server này | Không thể sửa đổi mật mã và mã cá nhân trên máy chủ này. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `2` | `0x00792D2F` | Loại trừ nhân vật thành công | Loại trừ nhân vật thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `2` | `0x00792D2F` | Mật mã loại trừ nhân vật sai lệch | Mật mã loại trừ nhân vật sai lệch. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `2` | `0x00792D2F` | Mã cá nhân loại trừ nhân vât sai lệch | Mã cá nhân loại trừ nhân vât sai lệch. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `2` | `0x00792D2F` | Loại trừ nhân vật thất bại | Loại trừ nhân vật thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x00792E00` | Lưu trữ thành công | Lưu trữ thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x00792E00` | Lưu trữ thất bại | Lưu trữ thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x00792E00` | Tài khoản thẻ sai lầm | Tài khoản thẻ không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `3` | `0x00792E00` | Mật mã sai lệch | Mật mã không khớp. | Hiệu đính ngữ nghĩa/câu chữ |
| `3` | `0x00792E00` | Thẻ đã sử dụng qua | Thẻ đã sử dụng qua. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `3` | `0x00792E00` | Đã lưu trữ điểm số thẻ khởi động | Đã lưu trữ điểm số thẻ khởi động. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `4` | `0x00792F5D` | Điểm số còn dư | Điểm số còn dư. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `4` | `0x00792F5D` | Kỳ hạn có thể chơi | Kỳ hạn có thể chơi. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `7` | `0x00793113` | Giới thiệu thành công | Giới thiệu thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `7` | `0x00793113` | Giới thiệu thất bại! Hãy xác định lại tư cách người giới thiệu và người được giới thiệu có phù hợp hay không! | Giới thiệu thất bại! Hãy xác định lại tư cách người giới thiệu và người được giới thiệu có phù hợp hay không! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `8` | `0x0079317C` | Được giới thiệu 5 lần được thưởng số điểm là Kỳ hạn có thể chơi | Được giới thiệu 5 lần được thưởng số điểm là Kỳ hạn có thể chơi. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `9` | `0x00793224` | Điểm số được giới thiệu đạt đến | Điểm số được giới thiệu đạt đến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `10` | `0x007933F0` | Do quy định số thời gian online, lần sau bạn có thể đăng nhập thời gian tiến hành game là | Do quy định số thời gian online, lần sau bạn có thể đăng nhập thời gian tiến hành game là. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x00792BAD`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
