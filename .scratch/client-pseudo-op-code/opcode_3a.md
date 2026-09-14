# PHÂN TÍCH — Main OP 0x3A (Case 51) — `FUN_007956b9` @ `0x007956B9` — **Đẩy kết quả phiên cá cược xúc xắc (scene "Tài Xỉu" — BiDaXiao) cho màn hình cảnh #1 (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Client: `aLogin.exe` (Delphi)
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/`). Ánh xạ jump table đã kiểm: `jumptable_byte200_0x78A8EE[0x3A] = 0x33 (=51)` → `jumptable_dword200_0x78A9B6[51]` (entry `0x0078AA82` = base + 51×4) → target `0x007956B9` = **Case 51**. Đối chiếu kép: inline trong dispatcher tổng `0078a89c_FUN_0078a89c.c:6920–6934` (marker `UNK_007956cc` / `UNK_007956ee`) khớp `case_051`.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). Ba khoảng trống lớn nhất của tài liệu này ĐÃ ĐƯỢC LẤP: (1) body RTL `@PStrNCat` xuất hiện (`00402b60__PStrNCat.c`) → semantics §6 xác minh trực tiếp; (2) nhãn Tài/Xỉu đã dump & giải mã VISCII = **"Tiểu"/"Đại"** → suy luận 比大小 chốt 100%; (3) creator `gvar_007DA42C` tìm thấy trong `0055374c_FUN_0055374c.c:29–31` → class scene #1 **xác minh đúng tên `TRE_BiDaXiao`** (VMT `VMT_547800_TRE_BiDaXiao`), không còn là suy luận từ dải method.

> Phạm vi: **core logic opcode**. Toàn bộ "trái tim" nằm ở helper `FUN_00547c84` (handler chỉ là passthrough 1 nhánh). Phần paint/sound chỉ nêu để chứng minh consumer của dữ liệu.

---

## 1. Tóm tắt nghiệp vụ

**OP 0x3A là "packet trả kết quả" của màn hình cảnh scene #1** — object đích là instance **`TRE_BiDaXiao`** (比大小 — Tài Xỉu / so lớn-nhỏ), cùng unit với `TRE_BackGround`, `TRE_BiDaXiaoHelp`, `TRE_Bet` (VMT định danh `VMT_547718_TRE_BackGround`… tại `0051189c_FUN_0051189c.c:1747–1755`). **Tên class xác minh trực tiếp từ dòng tạo**: `0055374c_FUN_0055374c.c:30–31` gọi `TRE_BiDaXiao::TRE_BiDaXiao_Create(VMT_547800_TRE_BiDaXiao, 1, param_3)` rồi gán `gvar_007DA42C` (hàm chạy từ OP 0x39 — `case_050_00795579_FUN_00795579.c:39`).

- **Chỉ có SubOp 1**. Mọi SubOp khác: **silent drop** (`case_051.c:26–29`).
- Payload sau SubOp là **fixed 8 byte**: `[c1][c2][c3][flag][token:4B LE]` — helper `FUN_00547c84` cắt đúng tới RP[9] (1-based), byte thừa ignored.
- Client: ghi 3 byte vào **3 ô hiển thị** (`f78[1..3]`), tính **tổng** giữ tại `f60` (bắt buộc ≤255, ngược lại ERangeError), dựng flag `f5` = RP[5], cất **token DWORD** `f18` = RP[6..9], rồi **chuyển state machine sang 5** (`f61:=5`) để chạy nốt hoạt ảnh.
- Ở state 6, client **vẽ banner = IntToStr(tổng f60) + nhãn, phân dải theo tổng**: `[3..10]` nối chuỗi `DAT_00548930`, `[11..18]` nối `DAT_00548940` (`0054886c_FUN_0054886c.c:40–48`). **ĐÃ XÁC MINH TỪ DUMP (2026-09-14)**: hai nhãn là **"Tiểu"** (`54 69 AC 75`, VISCII = 小 = Xỉu) và **"Đại"** (`D0 D5 69`, = 大 = Tài) — xem §7; đúng cặp Tài/Xỉu của 比大小 (BiDaXiao).
- Ở state 7 (sau chờ 1000 ms + play sound theo `f5`), client **tự-inject OP 0x1A cục bộ** với payload `[tiền-tố?][token f18:4B LE]` (`00548080.c:151–166`) — OP 0x1A đã kiểm chứng ở `opcode_1a.md` là kênh **"đồng bộ bộ đếm/tài khoản số + banner thưởng"** → `f18` nhiều khả năng là **số tiền/điểm cộng-trừ** mà server muốn áp sau phiên cược.
- **C→S CÓ THẬT**: nhà của gói này cũng chính là state machine — tại state 4 client gửi **`[0x3A][f4:1B][f0c:4B LE]`** (6 byte; byte 2 = byte biến thể bảng `obj+4`, KHÔNG phải hằng SubOp — xem §6). Tức 0x3A là cặp **request ↔ response** một-vs-một SubOp (chỉ chiều S→C dùng SubOp `0x01`).

**Nghiêm trọng cho mock:** handler **không kiểm tra nil** `gvar_007DA42C`; nếu scene #1 chưa được tạo/đã bị free (`FUN_00553410` mode 1), `FUN_00547c84` ghi `*(byte*)(0+0x60)` → **Access Violation** (`00547c84.c:53`).

---

## 2. Entry & cách đọc payload

Pipeline chung (kế thừa handoff + `opcode_36.md` §2): `TForm1.CY_DelRevQueue` (`00516158_TForm1.CY_DelRevQueue.c:86–93`) cắt `_LStrCopy(msg, 2, Len-1)` → **RP = payload bỏ MainOp**; `FUN_0078a89c(RP, 0x3A)`.

`FUN_007956b9` (`case_051_007956B9_FUN_007956b9.c:20–29`, ĐÃ ĐỌC TOÀN BỘ):
```c
// 0-based: RP[0] = SubOp;  *(int*)(RP-4) = len
if (len(RP) == 0) _BoundErr(0);            // RP rỗng ⇒ ERangeError
SubOp = RP[0];
if (SubOp == 1)
    FUN_00547c84(*(int *)gvar_007DA42C, RP);   // truyền CẢ chuỗi RP (1-based Delphi)
// không có nhánh else ⇒ SubOp khác 1: bỏ qua im lặng
```
Vòng đệm SEH cuối handler (`LAB_00796408`, immediate `0x7963xx`) là cleanup frame chuỗi dùng chung của dispatcher, không phải logic.

Helper định vị qua `*(int*)(RP-4)` (độ dài ansistring) và `_BoundErr` — mọi phép đọc chuỗi RP đều **1-based**: `RP[1]`=SubOp, `RP[2..4]`=3 byte, `RP[5]`=flag, `Copy(RP,6,4)`=`RP[6..9]`.

---

## 3. Wire layout (S→C, 10 byte tối thiểu)

| Offset payload | RP 1-based | Nội dung | Ràng buộc client (bằng chứng bounds) |
| :--- | :--- | :--- | :--- |
| `[0]` | — | `0x3A` MainOp | — |
| `[1]` | `RP[1]` | **SubOp = 0x01** | `len≥1` (`case_051.c:22–26`); ≠1 → silent drop |
| `[2]` | `RP[2]` | `c1` → `f78[1]` (ô #1) | i=1: `len≥2` (`00547c84.c:176`) |
| `[3]` | `RP[3]` | `c2` → `f78[2]` (ô #2) | i=2: `len≥3` |
| `[4]` | `RP[4]` | `c3` → `f78[3]` (ô #3) | i=3: `len≥4`; **c1+c2+c3 ≤ 255** (ERangeError, `00547c84.c:198`) |
| `[5]` | `RP[5]` | `flag` → `f5` (chọn sound + chọn prefix inject 0x1A + dấu scroll) | `len≥5` check tường minh (`00547c84.c:233–236`) |
| `[6..9]` | `RP[6..9]` | **token DWORD LE** → `f18` | `Copy(RP,6,4)` + `FUN_0077ef7c` đọc byte [0..3] ⇒ `len≥9` (`00547c84.c:238–239`) |

- Byte `[10+]`: không ai đọc → thừa vô hại.
- Tổng hợp field ghi đối tượng: `f60 := c1+c2+c3`; `f61 := 5`; `f62 := 1` (`00547c84.c:241–242`).

---

## 4. Phân tích helper `FUN_00547c84` (TRÁI TIM — file đủ: `00547c84_FUN_00547c84.c` 248 dòng + `.asm.txt` 257 dòng, ĐỌC TOÀN BỘ CẢ HAI)

Header `.c:7–19` ghi `Callers: <none>` — Ghidra không nối call-edge vì caller là jump-table (`0x7956ee`); thực tế chỉ dispatcher OP 0x3A gọi.

### 4.1. Vòng 1 (`.c:52–165`) — tách chồng lấn 3 ô kết quả
```pascal
fSum := 0;                                  // +0x60
for i := 1 to 2 do
  for j := 1 to 2 do begin
    k := ((i+j) mod 4) + ((i+j) div 4);     // cặp so sánh: (1,2)(1,3)(2,3)(2,1)
    if (Abs(Slot[i].X - Slot[k].X) < 15) and
       (Abs(Slot[i].Y - Slot[k].Y) < 15) then
      if Slot[k].X - Slot[i].X > 0 then Inc(Slot[k].X, 15)
      else                                  Dec(Slot[k].X, 15);
  end;
```
- `Slot[t] = *(obj+0x7c + t*4)^` — mảng 4 con trỏ (bounds `0..3`, index 0 không dùng), mỗi slot có **X @ +0x28, Y @ +0x2c, state @ +0x4** (state do nơi khác set — không phải opcode này).
- `iVar13 = 15` = khoảng cách chống đè; phép đẩy **chỉ theo trục X** (`.asm:110–128`: `ADD/SUB [slot+0x28], 15`).
- Đây là bước "trải 3 viên/xếp 3 ô" trước khi vẽ — không đụng payload.

### 4.2. Vòng 2 (`.c:166–230`) — nạp 3 byte payload + vẽ từng ô
```pascal
for i := 1 to 3 do begin
  f78[i]   := Byte(RP[i + 1]);              // BoundErr nếu len(RP) < 4  (RP[2..4])
  fSum     := fSum + f78[i];                // ERangeError nếu tổng > 255 ⇒ f60
  DrawSprite(Slot[i], IMG_10252, frame0, ..., X := Slot[i].X, Y := Slot[i].Y, ...,
             p15 = 6, p16 = f78[i], p17 = f78[i]);   // .asm:176–219 (hai byte cuối CÙNG một giá trị f78[i] — nhìn asm xác nhận)
end;
```
Consumer vẽ `FUN_00774220` (`00774220.c:446–479`) là wrapper sprite dùng chung toàn client (đặt `slot+4 := imgID` rồi gọi renderer `FUN_007742e4`) — **graphics, ngoài phạm vi**; chỉ kết luận: `f78[i]` đi thẳng vào tham số frame/variant của ô thứ i (hằng `0x70a3d70a/0x3fd70a3d` là constant pack render, không phải con trỏ chuỗi).

### 4.3. Đuôi helper (`.c:231–242`)
```pascal
f5     := Byte(RP[5]);                      // len<5 ⇒ _BoundErr(4) — check tường minh
tmp    := Copy(RP, 6, 4);                   // _LStrCopy(i=RP, 6, 4)
f18    := ReadDwordLE(tmp);                 // FUN_0077ef7c: b0 + b1*$100 + b2*$10000 + b3*$1000000
                                            //   (0077ef7c.c:201–241; substring <4 ký tự ⇒ ERangeError
                                            //    từng byte — tức yêu cầu cứng len(RP) ≥ 9)
f61    := 5;                                // state machine → "trượt ra / dọn kết quả"
f62    := 1;                                // flag pending-refresh
```
Lưu ý idiom `FUN_0077ef7c(*(undefined4*)gvar_007D9D30, s)`: param_1 là **rác** (không dùng trong body codec — `0077ef7c.c:162–248` chỉ chạm param_2 là chuỗi). Toàn client dùng khuôn này 40+ nơi.

### 4.4. State machine tiêu thụ kết quả — `FUN_00548080` (tick khi `f6<>0`, `00548080.c:37–283`)
| f61 | Hành vi (tóm tắt) |
| :-- | :-- |
| 0 | reset (`FUN_00547bbc`: f4,f5,f10,f18,f63:=0; slot state:=0) → 9 |
| 9 | slide-in: `f24 += 15` đến >244 → 1 |
| 1 | chờ; khi `f4 ≠ 0` gọi `FUN_00547fd8` (setup 5 vị trí `f30/f48[1..5]` theo base layout f4=1:(370,190)/f4=2:(380,540) + jitter Random(70)) → 4 |
| 4 | hiện banner "đang chờ" (`gvar_007DA084` vtable+0x90, chuỗi `DAT_00548608`); nếu banner ẩn & `f63=0`: **GỬI C→S OP 0x3A** qua `FUN_00547fbc` (`00548080.c:103`), `f63:=1` |
| 5 | **(do OP 0x3A vừa nhận set)** slide-out `f24 -= 15` → 6 |
| 6 | banner **IntToStr(f60)+nhãn dải [3..10]/[11..18]** (`FUN_0054886c`) |
| 7 | chờ 1000 ms (`FUN_007c4d30`); chưa xong → sound theo `f5` (`FUN_005487ec`: file `DAT_00548850` vs `DAT_00548860`); xong → **inject local OP 0x1A** = `Copy prefix(DAT_00548620 nếu f5=0 / DAT_00548614 nếu f5≠0) + f18:4B LE`, `f18:=0`, `FUN_00547c50` (`f10 := ±f0c`), → 0xc |
| 0xc | cuộn/tính lại `f8 += f14(=f10/33)` theo phím, chờ điều kiện → về 0 |
| 0xb | (force từ `FUN_00549978` — handler ESC/cancel, 00549978.c:21–23) inject nốt 0x1A nếu `f18<>0`, rồi **`FUN_00553818` = gửi OP 0x39 rời scene** (`00553818.c: FUN_0077f414(...,0x39)`), `f6:=0` dừng máy |

### 4.5. Chuỗi giá trị 3 byte là gì? (ĐÃ CHỐT 2026-09-14)
- `f60` tổng được **in ra banner** với hai dải nhãn — **dump + giải mã VISCII xác nhận**: `DAT_00548930` = `54 69 AC 75` → **"Tiểu"** (小, tức Xỉu); `DAT_00548940` = `D0 D5 69` → **"Đại"** (大, tức Tài) (`0054886c.c:42–46`).
- Mỗi byte `f78[i]`喂 vào thông số sprite của ô i; 3 ô + chống chồng 15px.
- **Kết luận (đã xác minh, confidence CAO):** `RP[2..4]` = **3 giá trị xúc xắc**, `RP[5]` = cờ kết quả, `RP[6..9]` = token số (tiền thưởng) echo qua OP 0x1A. Tên class scene #1 xác minh đúng `TRE_BiDaXiao` (比大小 = Đại/Tiểu — đúng bài toán 3 xúc xắc 1..6, tổng Tài ≥ 11). Không còn là suy luận.

---

## 5. Global & ngữ cảnh sống

| Global | Vai trò | Bằng chứng |
| :--- | :--- | :--- |
| `gvar_007DA42C` | **Con trỏ object scene #1 — class `TRE_BiDaXiao`** (methods `FUN_00547b94…FUN_00548e98`) — param_1 của helper. **Nơi gán ĐÃ TÌM THẤY (2026-09-14)**: `0055374c_FUN_0055374c.c:29–31` — nhánh `mode==1` gọi `TRE_BiDaXiao::TRE_BiDaXiao_Create(VMT_547800_TRE_BiDaXiao,'\x01',param_3)` rồi `*(int**)gvar_007DA42C = piVar3`; cuối hàm ghi byte mode `*(byte*)(param_1+4) = param_2` (`:58`). `FUN_0055374c` được gọi từ handler OP 0x39 (`case_050_00795579_FUN_00795579.c:39`, param_1 = `gvar_007D9D88^` = TSportManage). HOLE `0x00514C62–0x00516108` (vẫn chưa có body — đã kiểm `index.csv` bằng python: không hàm nào phủ) KHÔNG chứa creator như nghi vấn trước đây | `0055374c.c:30–31,58`; `00553410.c:32–33`; `00553840.c:35`; `0055391c.c:35` |
| `DAT_00948de4` | **Alias toàn cục khác của cùng object** (đọc `f61`, ghi `f0c` y hệ) | `005492dc.c:27,34`; `00548080.c:103` |
| Controller scene | Object có byte `+4` = **mã cảnh** (1…6,0xff); mode 1 ↔ `gvar_007DA42C`; holder `gvar_007D9D88`; free tập trung khi out-world | `00553410.c:24–52`, gọi từ `00603f20.c:227` (hàm rời world — kế thừa `opcode_36.md` §5.2) |
| Họ anh em | mode 2 `gvar_007DA0F4` = `TSBDManager`, 3 `gvar_007DA778` = `TRE_ZMChessMain`, 4 `gvar_007D9F98` = `TLottoManager`, 6 `gvar_007DA4EC` = `TMachineManager`, 0xff `gvar_007DA0A0` = `TSportDemo` — mỗi cảnh một object; **tất cả tên class xác minh từ chính dòng Create trong `0055374c_FUN_0055374c.c:25–57`** | `00553410.c:24–52`; `0055374c.c:25–57` |
| `gvar_007DA084` | Banner `TSe_TalkMsgFormPlus` (kế thừa `opcode_1a.md` §0: vtable+0x90 = ShowText(text, ms)) | `0054886c.c:51` |
| `gvar_007D9ED8` | Image Manager (kế thừa `opcode_36.md` §5.1) | `00548080.c:75,82` |
| `gvar_007D9D30` | Context vô nghĩa (rác) của codec/sender — param_1 không dùng | `0077ef7c.c:162–189` |
| Form input số | `FUN_00549458` (ctor form cha, tạo tại `0051189c.c:1747` → `gvar_007D9CA8` — **instance khác**, cùng class): 5 vị trí số, `btn_readme`, `btn_quit`(→force 0xb), edit text @+0x1a0; người dùng nhập `n`: `10 ≤ n ≤ 1000`, `n ≤ f8`, `n+f8 < 10^7` (err banner `DAT_005479c0/e4/a0`) → `f0c := n` | `00549458.c`, `005492dc.c:27–38`, `0054790c.c:9–26`, `00549370.c:44–52` |

**Field object** (offset → ý nghĩa → nơi chứng minh): xem bảng tại §4 + `00547c84.c` (f60,f78,f5,f18,f61,f62) · `00548080.c` (f6,f4,f10,f14,f24,f2c,f63,f68/f70 double Now) · `00547fd8.c` (f30/f48[1..5]) · `00547bbc.c` (f7c[1..3].+4, f8c[1..3]) · `00548944.c:50–160` (f7c[i] X@+28,Y@+2c — paint sort & draw, graphics).

---

## 6. Chiều C→S — CÓ, khôi phục được body thật

**Sender duy nhất trong SSOT:** `FUN_00547fbc` (asm `00547fbc.asm.txt`: `MOV CL,0x1; MOV DL,0x3a; CALL 0x0077f414` — **CL=1 bị builder case 0x3A bỏ quên**, chỉ DL=op được dùng; C của Ghidra `CONCAT31(…, 0x3a)` là artifact), gọi từ state 4 (`00548080.c:103`) **một lần/phiên** (gate `f63`).

`0077f414_FUN_0077f414.c:1022–1031` (`case 0x3a:`) — asm chuẩn tại `0077f414_FUN_0077f414.asm.txt:3617–3648`:
```asm
; sA[0]:=1 ; sA[1]:=DL(=op 0x3A) ; t := @PStrCopy(sA)          ; t = #1'3A'
; s2[0] := 1 ; s2[1] := [obj+4]  = f4                          ; ← MOV DL,[EDX+0x4]
; @PStrNCat(t, s2, 2)  — nối TỐI ĐA 2 ký tự TỪ DATA của s2; s2 dài 1 ⇒ chỉ nối f4
;                        ⇒ t = #2'3A f4'      (idiom hiệu chuẩn ở opcode_37.md §6.3,
;                          khớp login "Khối 1 [0x01][lenPw]" — opcode_00_01.md §5,
;                          opcode_37.md: `PStrNCat` thêm đúng 1 ký tự, KHÔNG copy length byte)
; CALL 0x0077ee84 : append DWORD LE [obj+0xc]
; CY_AddSedQueue(gvar_007DA664 = TFConnect)
```
⇒ **Body C→S = `[0x3A][f4:1B][f0c:4B LE]` (6 byte)** — byte 2 là **biến thể bảng f4** (1|2), không phải SubOp; `@LStrFromString` bỏ length byte của shortstring nên payload bắt đầu thẳng bằng `0x3A`. **`@PStrNCat` ĐÃ CÓ BODY (2026-09-14)** — HOLE `0x00402B1C–0x00402B90` đã được decompile: `00402b60__PStrNCat.c:584–606`. **Xác minh và đính chính chính xác hóa** semantics: `dest[0] += n` với `n = min(src[0], maxLen − dest[0])` (nếu `dest[0] > maxLen` hoặc phần bù = 0 thì KHÔNG nối gì); nội dung copy lấy từ `src[1..n]` — **byte độ dài `src[0]` không bao giờ được copy**; không có null-terminator. Với `dest=#1'3A'`, `src=#1'f4'`, `maxLen=2`: n = min(1, 2−1) = 1 → đúng 1 byte `f4` nối vào → `[3A][f4]`. **Khớp 100% với khuôn đã hiệu chuẩn ở `opcode_37.md §6.3` / `login_flow_research.md §3.7` — không cần suy luận chéo nữa.** Bản thân builder `FUN_0077f414` có call-site `_PStrNCat` nằm trong danh sách caller của RTL (`00402b60__PStrNCat.c:428–429`: `sub_0078a357`/`sub_0078a459` — chuỗi concat của `case 0x3d`). Gate đầu hàm: `if (*gvar_007DA3A0 = 0) → không gửi` (`0077f414.c:768`) = cờ đã kết nối. `f0c` = **số người dùng nhập (10..1000, ≤ f8)**; `f4` = byte biến thể bảng (1|2) — **nơi ghi f4 vẫn không có trong export** (grep `+ 0x4) =` trên các body mới không thấy writer cho instance `TRE_BiDaXiao`; có thể qua tham số `param_3` của `TRE_BiDaXiao_Create` — `0055374c.c:30`).

---

## 7. Chuỗi literal & encoding

- **Trong `FUN_00547c84` (.c + .asm): ZERO literal** — chỉ hằng số render/SEH (`LAB_`, `0x70a3d70a`…). ✓
- Chuỗi nghiệp vụ của **cả opcode nằm ở các helper lân cận** (code-gap). **Cập nhật 2026-09-14**: `redump/lit_548930.hex` + `lit_548940.hex` ĐÃ CÓ và được giải mã (bảng **VISCII** — lưu ý `cp1258` trong Python KHÔNG giải mã đúng các byte thanh điệu 0xAC/0xD0/0xD5, phải dùng `iconv -f VISCII`); các địa chỉ còn lại vẫn **chưa dump — không dịch**:

| Địa chỉ | Thuộc | Vai trò quan sát được |
| :--- | :--- | :--- |
| `DAT_00548614` / `DAT_00548620` | gap 0x5485bf–0x548640 | **Tiền tố khi inject local OP 0x1A** (f5≠0 / f5=0) — kỳ vọng 1 ký tự = SubOp 0x1A (0x1A sub 1/2/5/6 đọc 4B LE — khớp `opcode_1a.md` §0#4) |
| `DAT_00548608` / `DAT_0054862c` | cùng gap | Banner "đang chờ"/"hết token" (state 4 / 0xb) |
| `DAT_00548920` + `0x548930` / `0x548940` | gap sau `FUN_0054886c` | Nhãn tổng xúc xắc: hậu tố chung (`0x548920`, **chưa dump**) + **`DAT_00548930` = "Tiểu" (`54 69 AC 75` + `00`, VISCII; raw — content không header trong `lit_548930.hex`, 3 byte pad tới 0x548937)** + **`DAT_00548940` = "Đại" (`D0 D5 69`, VISCII; const AnsiString ĐẦY ĐỦ HEADER — `ff ff ff ff | 03 00 00 00` tại `0x548938–0x54893F`, thấy trong `lit_548930.hex`)** — **xác nhận Tài/Xỉu (小/大)** |
| `DAT_005479c0` / `e4` / `a0` | gap sau `FUN_0054790c` | 3 banner lỗi validate số nhập |
| `DAT_00548850` / `DAT_00548860` | sau `FUN_005487ec` | 2 tên sound theo `f5` (sound — ngoài phạm vi, 1 dòng) |

Quyền ưu tiên giải mã khi redump: cp1258→NFC (tiền lệ `opcode_02.md` §5), thử thêm VISCII nếu vô nghĩa (`opcode_13.md`). **Kinh nghiệm 2026-09-14: chính VISCII (`iconv -f VISCII`) mới cho kết quả có nghĩa cho các byte 0xAC/0xD0/0xDF/0xB6… — `cp1258` của Python ánh xạ các byte này thành ký tự Latin-1 vô nghĩa; nên dùng cả hai và chọn kết quả đọc được.**

---

## 8. Ghi chú cho Mock Server

1. **Wire S→C** (10 byte): `[0x3A][0x01][c1][c2][c3][flag][t0 t1 t2 t3]`. Ví dụ xúc xắc 4/5/6 (tổng 15 = "Tài"), flag=1, token=20000 (`0x4E20`):
   - payload: `3A 01 04 05 06 01 20 4E 00 00` → XOR `0xAD`: `97 AC B9 B8 AB AC 8D E3 AD AD`
   - frame: `F4 44 0A 00 97 AC B9 B8 AB AC 8D E3 AD AD`
2. **Chỉ gửi khi scene #1 đang sống** (object `gvar_007DA42C` ≠ nil — thường ngay sau khi client gửi request C→S bên dưới). Gửi lúc scene chưa mở ⇒ **AV crash client** (không có nil-check).
3. **Độ dài ≥ 10 byte payload**; `c1+c2+c3 ≤ 255` (mock an toàn: mỗi byte ≤ 85). vi phạm ⇒ ERangeError giữa chừng.
4. Muốn client **show kết quả + cộng tiền**: chọn `token` là delta numeric — client sẽ tự phát OP 0x1A nội bộ (prefix chưa dump; giá trị 4B LE = token). Server **không cần gửi thêm 0x1A** cho cùng sự kiện nếu đã muốn client tự inject.
5. **Nhận C→S**: `[0x3A][f4][bet:4B LE]` 6 byte (ví dụ `3A 02 F4 01 00 00` = kiểu bảng 2, cược 500; lưu ý `f4=1` sẽ trông giống SubOp nhưng là biến thể bảng) — chỉ xuất hiện 1 lần/phiên ở state 4; respond bằng đúng gói mục 1.
6. Sau khi phiên đóng (state 0xb), client gửi **OP 0x39** — server nên chuẩn bị handle leave-scene.
7. SubOp ≠ `0x01`: drop im lặng (không ack) — có thể dùng làm probe.

---

## 9. Source trail + UNKNOWN

**Đã đọc/kiểm chứng trực tiếp (SSOT):**
1. `case_functions/functions/case_051_007956B9_FUN_007956b9.c:20–58` — handler + SubOp gate.
2. `functions/00547c84_FUN_00547c84.c` (toàn bộ 248 dòng) + `.asm.txt` (toàn bộ 257 dòng) — layout RP, bounds, field offset.
3. `functions/0077ef7c_FUN_0077ef7c.c:162–248` — codec 4B→DWORD LE; `00402b90.c`, `0077ee84` (DWORD→4B), `007c4d30` (timer), `00774220.c:446–479` (wrapper render — 1 dòng).
4. `functions/00548080_FUN_00548080.c:37–283` — state machine; `00547fbc.c` — caller gửi 0x3A; `00547c50/00547fd8/00547bbc/00548640/005487ec/0054886c/00548e98` — field consumers; `00548944.c:36–318` — paint (graphics).
5. `functions/00553410.c / 00553840.c / 0055391c.c / 00553818.c` — dispatcher vòng đời scene + OP 0x39 exit; `00603f20.c:227` — free khi out-world.
6. `functions/005492dc.c:20–44`, `0054790c.c`, `00549370.c`, `00549978.c`, `00549458.c` — form nhập số/validate/hủy.
7. `functions/0077f414_FUN_0077f414.c:760–812, 1022–1031` + `.asm.txt:3617–3648` — body C→S.
8. `functions/0078a89c_FUN_0078a89c.c:6920–6934` — inline case 0x3A (xác nhận kép); `redump/jumptable_*.hex` — parse python: `byte[0x3A]=0x33`, `dword[51]=0x007956B9`.
9. `functions/0051189c_FUN_0051189c.c:1747–1756` — họ class `TRE_BackGround/TRE_BiDaXiaoHelp/TRE_Bet` (tên VMT thật).
10. `functions/0055374c_FUN_0055374c.c:25–58` — **creator toàn bộ 6 manager theo mode (`TRE_BiDaXiao`/`TSBDManager`/`TRE_ZMChessMain`/`TLottoManager`/`TMachineManager`/`TSportDemo`) + ghi byte mode `TSportManage+4`** (gọi từ OP 0x39, `case_050...c:39`).
11. `functions/00402b60__PStrNCat.c:584–606` — **body RTL `@PStrNCat` mới export** — xác minh semantics nối shortstring §6.
12. `redump/lit_548930.hex` + `lit_548940.hex` — decode VISCII → "Tiểu"/"Đại" (§4.5, §7).
13. `index.csv` (python): HOLE `0x00514C62–0x00516108` (vẫn trống, nhưng creator KHÔNG nằm ở đây), HOLE `0x547409–0x54790C` (VMT họ TRE_* — vẫn chưa dump). HOLE `0x00402B1C–0x00402B90` **ĐÃ ĐƯỢC LẤP** (`00402b60__PStrNCat.c`).
14. Kế thừa đối chiếu: `opcode_36.md` (framing, RP-cut, gvar_007DA664/ED8, out-world), `opcode_1a.md` (ngữ nghĩa 0x1A + banner vtable+0x90), `opcode_00_01.md` §5 (khuôn builder C→S), handoff S11 (sceneMode).

**CÒN THIẾU (cập nhật 2026-09-14 — cần redump/tiếp tục):**
- ~~Nơi gán `gvar_007DA42C`~~ **ĐÃ TÌM THẤY**: `0055374c_FUN_0055374c.c:30–31`, class `TRE_BiDaXiao` (VMT `VMT_547800_TRE_BiDaXiao`).
- ~~Ngữ nghĩa business (xúc xắc 3 viên, dải Tài/Xỉu)~~ **ĐÃ XÁC MINH**: dump `lit_548930/548940.hex` → "Tiểu"/"Đại" (小/大), khớp dải [3..10]/[11..18] của tổng 3×(1..6).
- VMT họ `TRE_*` HOLE `0x547409–0x54790C` — **vẫn chưa dump** (không có `vmt_5474*.hex` trong `redump/`).
- Call-site `SendCommand(0x3A)` byte `CL` — asm sender `00547fbc.asm.txt` (`MOV CL,1`) đã có nhưng **builder `case 0x3a` bỏ quên CL, dùng `[obj+4]`** (không đổi); call-site thật với `CL` ngữ nghĩa vẫn chưa thấy ngoài HOLE.
- Vùng `0x0051Bxxx` (tài nguyên scene) **chưa dump**.
- Nội dung 8 chuỗi literal §7 **còn lại** (tiền tố inject 0x1A `DAT_00548614/20`, banner chờ/hết token `DAT_00548608/2c`, 3 lỗi validate `DAT_005479c0/e4/a0`, 2 sound `DAT_00548850/60`) — code-gap, chưa dump.
- Nơi ghi `f4` (biến thể 1|2), `f8` (limit), cấp phát mảng slot `f7c[0..3]` + set state slot `+4` (đều ngoài export).
- Ý nghĩa chính xác byte `flag` (RP[5]) và các tham số render cuối của `FUN_00774220` (param_15=6, param_16/17) — graphics RTL chưa đi sâu.
