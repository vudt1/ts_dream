# PHÂN TÍCH — Main OP 0x21 (33) / Case 29 / `FUN_007928E4` @ `0x007928E4`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). Handler nhỏ (111 dòng), không codec, 2 nhánh. `func_0x00602f68` **đã có body trong bản dump mới** — xem §4.2.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Server push 2 loại không liên quan chung một MainOp, cấu trúc **2 tầng** (không phải switch phẳng):
  - Tầng 1: `T = RP[0] = P[1]` — chỉ `1` và `2` có nghĩa.
  - Tầng 2 (chỉ khi `T==1`): `K = RP[1] = P[2]` — `switch(K)` `1..8` → 8 **Toast/banner tĩnh** 2000ms qua `TSe_TalkMsgFormPlus` (`gvar_007DA084`, VMT `+0x90`). Không đọc số, không ghi state, không sound/light.
  - Nhánh `T==0x02`: **set 2 byte option** vào `TCY_OptionForm` (`gvar_007D9F74`, `+0x180/+0x181`) rồi gọi `func_0x00602f68(optionForm)` — **xác minh được từ body mới**: đây là hàm **đồng bộ nhãn 2 nút bấm** (không phải apply giá trị): với mỗi i=0..1, đọc cờ `+0x180+i`, tra tài nguyên `"btn_on"`/`"btn_off"` và ghi kết quả vào điều khiển `*(form+0x16C+i*4)+0x4C`.
- Không DWORD/Word codec, không text trên dây, không chạm bản ghi player (`gvar_007DA7BC` không xuất hiện).
- Chiều C→S `case 0x21: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x21 (33) → byte_table[0x78A8EE][0x21] = 0x1D (29)
                 → dword_table[0x78A9B6][29] = 0x007928E4
                 → FUN_007928e4 (Case 29)
```

- File chính: `ts_decompile/case_functions/functions/case_029_007928E4_FUN_007928e4.c` (111 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:5204-5274` — khớp 1:1.
- Manifest: `case_functions/manifest.csv:31` (`29, 0x0078AA2A, 0x007928E4`).
- Framing/XOR/pump như `opcode_00_01.md` §2. File `.asm.txt` dispatcher chỉ còn prologue (71 dòng) nên mapping xác nhận bằng 2 file `.hex`/`.csv` + manifest + bản inline.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x21`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`), con trỏ `*(EBP-0x0c)`, độ dài tại `*(ptr-4)`.

### 2.3. Đọc tầng 1 (dòng 23-30)

```c
if (*(RP-4) == 0) _BoundErr(0);  // RP rỗng (L=1) → ném
T = (uint)*(byte*)(RP + 0);      // T = RP[0] = P[1]
if (T == 1) {...} else if (T == 2) {...}
// T==0 hoặc >=3: no-op (chỉ epilogue _LStrArrayClr/_LStrClr cuối hàm)
```

Mọi phép đọc là dereference byte trực tiếp + guard `len`, không `_LStrCopy`/codec nào.

---

## 3. Bảng tổng hợp SubOp

| T1 `T=P[1]` | T2 `K=P[2]` | Wire (payload) | Core logic |
| :---: | :---: | :--- | :--- |
| `0x01` | `0x01` | `[21][01][01]` (3B) | Banner `UNK_00798418` 2000ms |
| `0x01` | `0x02` | `[21][01][02]` | Banner `UNK_00798448` |
| `0x01` | `0x03` | `[21][01][03]` | Banner `UNK_00798478` |
| `0x01` | `0x04` | `[21][01][04]` | Banner `UNK_007984C0` |
| `0x01` | `0x05` | `[21][01][05]` | Banner `UNK_007984F0` |
| `0x01` | `0x06` | `[21][01][06]` | Banner `UNK_00798530` |
| `0x01` | `0x07` | `[21][01][07]` | Banner `UNK_00798570` |
| `0x01` | `0x08` | `[21][01][08]` | Banner `UNK_00798594` |
| `0x02` | — | `[21][02][A:1B][B:1B]` (4B) | `*(OptionForm+0x180)=A`, `+0x181=B`, đồng bộ ảnh 2 nút `btn_on/btn_off` |
| `0x00`,`≥0x03` | — | — | no-op |
| `0x01` | `0x00`,`≥0x09` | `[21][01][K lạ]` | no-op (switch không default) |

---

## 4. Chi tiết từng nhánh (core logic, bỏ graphics/sound/animation — OP này vốn không có)

### 4.1. Nhánh `T==0x01` — 8 Toast tĩnh

- **Wire**: `P[0]=0x21, P[1]=0x01, P[2]=K (0x01..0x08)`.
- **Đọc**: `T=RP[0]` (guard `len>=1`), `K=RP[1]` (guard `len(RP)>=2` nếu không `_BoundErr(1)`), `switch(K)`.
- **Xử lý**: cả 8 case cùng khuôn `(VMT+0x90)(*gvar_007DA084, &UNK_007984xx, 2000, 0, 0)` (bản inline ghi 3-arg, bản case ghi 5-arg — cùng hàm, chênh do Ghidra mất varargs). Không đọc thêm byte, không ghi global, không gate, không `WA0014.wav`, không `TLight`. Thừa byte sau `P[2]` bị bỏ qua.

### 4.2. Nhánh `T==0x02` — Set 2 byte option

- **Wire**: `[21][02][A 1B][B 1B]` = 4 byte payload.
- **Đọc**: byte trực tiếp, không codec: `A=RP[1]` (guard `len>=2`), `B=RP[2]` (guard `len>=3`).
- **Xử lý**:
  ```c
  *(*(gvar_007D9F74) + 0x180) = A;
  *(*(gvar_007D9F74) + 0x181) = B;
  func_0x00602f68(*(gvar_007D9F74));
  ```
  `gvar_007D9F74 = TCY_OptionForm` (gán tại `0051189c:1588-1590` qua `VMT_5FA1CC`). Bằng chứng `+0x180` là cờ gate boolean: `00642c2c.c:100-104` (`if (*(OptionForm+0x180)==0){banner(...);return;}`).
- **`func_0x00602f68` — ĐÃ PHỤC HỒI** (`ts_decompile/functions/00602f68_FUN_00602f68.c`, 141B, `index.csv:6360`; chữ ký 1 tham số EAX=form). **Đính chính nhận định cũ "phần apply ở mức unknown"**: hàm **không apply giá trị nào** — nó chỉ đồng bộ hình 2 nút toggle:
  ```
  for i in 0..1:                                  (vòng lặp do..while, :50)
    if *(form + 0x180 + i) == 0: h = FUN_007C9B38(*gvar_007D9ED8, "btn_off")   (:31-32)
    else:                      h = FUN_007C9B38(*gvar_007D9ED8, "btn_on")    (:41)
    *(*(form + 0x16C + i*4) + 0x4C) = h                                        (:38/:47)
  ```
  `FUN_007C9B38` = helper tra handle ảnh theo tên (load `.bmp` nếu thiếu — như trong `opcode_20.md` §4.1.1); `gvar_007D9ED8` = resource manager toàn cục. Mảng điều khiển nút tại `form+0x16C` (2 phần tử; guard range decompiler `≤5` là artifact Delphi array — thực dụng chỉ 0..1 theo `:50`). → **Nghiệp vụ**: server bật/tắt 2 option (`A`,`B`) và form Option vẽ lại nút on/off tương ứng; không có action nào khác.

---

## 5. Chuỗi VISCII → UTF-8

- Không có payload text trên dây — mọi text là chuỗi tĩnh: `UNK_00798418/44/78/C0/F0/30/70/94`.
- `redump/` (kể cả bản mới) **vẫn không có dump** cho cả 8 địa chỉ → **không decode được từ source cho phép**. Cần redump `.rodata` tại đó (Delphi `[len:4LE][chars][00]`).
- Nhánh `T==0x02`: body mới `00602f68` chỉ lộ 2 chuỗi **ASCII tài nguyên** `"btn_off"` / `"btn_on"` (`00602f68_FUN_00602f68.c:32,41` — tên file ảnh, không phải text VISCII hiển thị).

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:962-963`: `case 0x21: break;` — rỗng hoàn toàn.
- Kết luận: OP 0x21 S→C thuần. Không format C→S để mock (caveat suy hao decompile như `opcode_1a.md` §5, nhưng không có bằng chứng dương cho hướng gửi).

---

## 7. Ghi chú cho Mock Server

```
S→C [21][01][01..08]      ; 8 toast tĩnh 2000ms
S→C [21][02][A u8][B u8]  ; set OptionForm+0x180=A,+0x181=B + vẽ lại nút btn_on/btn_off
C→S [21]: KHÔNG TỒN TẠI
```

1. Downlink-only. Frame `[F4 44][L:Word LE][payload]`, XOR `0xAD`.
2. Không gửi `T` ngoài 01/02, `K` ngoài 01..08 (no-op). Không gửi `L=1` (`_BoundErr(0)`). Nhánh 01 cần `L>=3`, nhánh 02 cần `L>=4`.
3. Muốn test yên lặng (không banner) → dùng `[21][02]`; test banner → `[21][01][K]`. Không sound/light đi kèm.
4. Nội dung 8 banner chưa đọc được — test trên client thật chỉ kiểm tra banner có hiện hay không.
5. Nhánh 02 giờ kiểm chứng được bằng mắt: 2 nút option trên `TCY_OptionForm` phải đổi ảnh on/off theo `A`/`B` (`0x00` → `btn_off`, khác 0 → `btn_on`).

---

## 9. Giới hạn còn lại (cập nhật 2026-09-14)

- [ ] 8 toast `UNK_00798418..00798594` vẫn cần redump `.rodata` (không nằm trong batch dump mới).
- [x] ~~`func_0x00602f68` chưa có body~~ → đã phục hồi (`00602f68_FUN_00602f68.c`): chỉ đồng bộ ảnh nút, không apply logic sâu hơn.
- [ ] Ý nghĩa nghiệp vụ cụ thể của 2 option `A`/`B` (label người dùng của 2 nút) chưa xác định từ mã.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_029_007928E4_FUN_007928e4.c` | Handler chính toàn bộ |
| 2 | `functions/0078a89c_FUN_0078a89c.c:5204-5274` | Bản inline đối chiếu 1:1 |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv:31` | Mapping |
| 4 | `functions/0051189c_FUN_0051189c.c:1265-1266` + `:1588-1590` | Định danh `TSe_TalkMsgFormPlus` + `TCY_OptionForm` |
| 5 | `functions/00642c2c_FUN_00642c2c.c:100-104` | Chứng minh `+0x180` là cờ option |
| 6 | `functions/0077f414_FUN_0077F414.c:962-963` | C→S rỗng |
| 7 | `functions/00602f68_FUN_00602f68.c:25-50` | **Mới**: body apply — vòng 2 lần đọc `+0x180/+0x181`, tra `btn_off`/`btn_on` (`:32,41`), ghi handle vào `*(+0x16C+i*4)+0x4C` (`:38,47`) |

---

<!-- VISCII-CORRECTION-START -->
## Bản hiệu đính VISCII → UTF-8 (2026-09-21, từ main opcode > sub opcode)

> Nguồn hiệu đính: `spec/Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C.md` §2–§3 (đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo VA + Pascal length prefix, giải mã VISCII → UTF-8). Cột **Literal gốc** giữ chứng cứ byte-string; cột **Bản hiệu đính** là câu đọc tự nhiên (không phải byte hiển thị nguyên văn của client). Phạm vi chính xác xem spec §5.

### Chú thích asm / chứng cứ (tài liệu asm kèm theo)
- Dispatcher S→C: `FUN_0078a89c` — asm `client_pseudo_c/0078a89c_FUN_0078a89c.asm.txt` (**đã copy từ `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`; file chỉ còn prologue 71 dòng tới `JMP [EAX*4+0x78a9b6]`, mapping đầy đủ xác nhận bằng `jumptable_byte200_0x78A8EE.hex` + `jumptable_dword200_0x78A9B6.hex` + `manifest.csv`**), C `client_pseudo_c/0078a89c_FUN_0078a89c.c`. Tra bảng `MOV AL,[EAX+0x78a8ee]` + `JMP [EAX*4+0x78a9b6]`.
- Handler `0x007928E4` — C `client_pseudo_c/case_029_007928E4_FUN_007928e4.c` (có); asm `client_pseudo_c/007928e4_*.asm.txt` **THIẾU** (không có trong `client_pseudo_c/` lẫn `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` — xem danh sách thiếu cuối tài liệu). Basic block trong bảng dưới là chứng cứ thay thế.

### Main opcode `0x21` — 4 literal (Sub = `body[0]`; `—` = parser/branch trực tiếp)

| Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---|---|---|---|
| — | `0x007928E4` | Đối phương chưa mở chức năng PK / PvP | Đối phương chưa mở chức năng PK / PvP. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| — | `0x007928E4` | Đối phương chưa mở chức năng Tham Chiến | Đối phương chưa mở chức năng Tham Chiến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| — | `0x007928E4` | Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến | Trong khi trả lời câu hỏi của Bắc Tinh Quân, không thể quan chiến. | Hiệu đính ngữ nghĩa/câu chữ |
| — | `0x007928E4` | Chiến đấu đặc thù không thể tham chiến | Chiến đấu đặc thù không thể tham chiến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |

### Danh sách asm thiếu (không copy được — không tồn tại ở nguồn)

- `0x007928E4`: không có `.asm.txt` trong `client_pseudo_c/` và không có trong `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` hay `case_functions/functions/` (chỉ có `.c`). Cần redump/disassemble lại từ `aLogin.exe` theo VA handler + basic block ở bảng trên.

<!-- VISCII-CORRECTION-END -->
