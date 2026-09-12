# PHÂN TÍCH — Main OP 0x0B (Case 11, `FUN_0078d5d1` @ `0x0078D5D1`) — chiều Server→Client

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server→Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (case function + dispatcher inline + C→S `FUN_0077f414` + 4/12 hàm con có body). Các hàm con còn lại **không có body — không suy diễn**.

---

## 0. Tóm tắt nghiệp vụ (1 đoạn)

**OP 0x0B là kênh "đồng bộ menu/form + trạng thái nhân vật" dạng fan-out:** 1 byte SubOp chọn 1 trong 12 đường xử lý, mỗi đường **pass nguyên RestPayload cho 1 form/hàm quản lý khác nhau**, tầng case này **không tự parse Word/DWORD nào** (ngoại trừ SubOp 7 và 9 đọc 1–2 byte cờ). 3 đường đọc được body cho thấy: SubOp 0 = reset/xóa trạng thái hiệu ứng theo `(id DWORD, kind Word)`; SubOp 3 = toast 4 loại theo 1 byte mã; SubOp 5 = đồng bộ danh sách "hỗ trợ chiến đấu" theo record 23 byte; SubOp 9 = ghi 2 byte cờ vào `PlayerRec+0x1308` + toast số khi byte thứ 2 = 1. 8 đường còn lại pass-through nên nghiệp vụ nằm trong hàm con chưa decompile.

---

## 1. Entry & cách đọc PacketBuffer

**Entry:** `FUN_0078d5d1` @ `0x0078D5D1` (Case 11, jump-entry `0x0078A9E2` của bảng `0x78A9B6`).

**Mapping MainOp 0x0B → Case 11 (3 bằng chứng độc lập):**
1. `manifest.csv` dòng 13: `11,0x0078A9E2,0x0078D5D1,EXPORTED,FUN_0078d5d1`.
2. `redump/jumptable_byte200_0x78A8EE.hex` dòng 1: `01 02 03 04 05 06 07 08 09 0A 00 0B ...` — index byte `0x0B` = giá trị `0x0B` (MainOp 0x0B → case index 11).
3. `redump/jumptable_dword200_0x78A9B6.hex` dòng 3: entry thứ 12 (index 11) = `D1 D5 78 00` = `0x0078D5D1` (little-endian).
4. Đối chiếu inline: khối `case 0xb:` tại `0078a89c_FUN_0078a89c.c:2179–2280` khớp từng nhánh với file case riêng.

**Dispatcher (bộ điều phối S→C):** `FUN_0078a89c` (`ts_decompile/functions/0078a89c_FUN_0078a89c.asm.txt`) tra bảng byte `0x78A8EE` lấy index, rồi tra bảng dword `0x78A9B6` để nhảy tới hàm xử lý (`case_001` đến `case_065`). MainOp `0x0B` → Case 11 → `FUN_0078d5d1`.

**Khung mạng (framing):** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame cả 2 chiều XOR khóa tĩnh `0xAD` qua `FUN_0050a248` / `FUN_0050a2fc`. Server luôn phát tín hiệu trước (Server speaks first).

**Cách đọc SubOp (đầu file case, dòng 28–35):**
```c
iVar2 = *(int *)(unaff_EBP + -0xc);          // ECX = RestPayload (payload đã cắt MainOp)
if (*(int *)(iVar2 + -4) == 0) _BoundErr(0); // guard Delphi: RestLen >= 1
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + 0); // SubOp = ECX[0]
```
- `unaff_EBP + -0xc` = **RestPayload** = toàn bộ payload trừ byte MainOp (quy ước `_LStrCopy(payload,2,len-1)`).
- Nên `ECX[0]` (0-based C) = `payload[1]` = **SubOp**, đọc bằng **1 byte thường, không codec**.
- `*(iVar2-4)` = độ dài Delphi AnsiString (guard `_BoundErr(n)` = yêu cầu `Len > n`).

**Quy ước `_LStrCopy` 1-based (Delphi `Copy`):**
| Phép đọc | Ý nghĩa trên wire |
|---|---|
| `_LStrCopy(ECX, 1, 4, &t)` | `ECX[0..3]` = `payload[1..4]` |
| `_LStrCopy(ECX, 2, 4, &t)` + `FUN_0077ef7c` | `ECX[1..4]` = `payload[2..5]` = **DWORD LE** |
| `_LStrCopy(ECX, 6, 2, &t)` + `FUN_0077eb9c` | `ECX[5..6]` = `payload[6..7]` = **Word LE** |
| `*(byte*)(ECX+n)` trực tiếp | `payload[n+1]`, 1 byte thường |

**Codec helper (đã đọc body, xác minh 100%):**
- `FUN_0077eb9c` (`0077eb9c_FUN_0077eb9c.c:166–174`): `b0 + b1*0x100` = **Word LE**.
- `FUN_0077ef7c` (`0077ef7c_FUN_0077ef7c.c:207–240`): `b0 + b1*0x100 + b2*0x10000 + b3*0x1000000` = **DWORD LE**.
- `FUN_0077eb1c`: encode U16 → chuỗi 2 byte LE (chỉ dùng chiều C→S).
- `FUN_0077ee84`: encode U32 → chuỗi 4 byte LE (chỉ dùng chiều C→S).
- `FUN_0077f098`: `_LStrToString + copy 8 byte` — **không được handler này gọi**.

> Quan trọng: **bản thân `FUN_0078d5d1` không gọi bất kỳ hàm Word/DWORD LE nào.** Mọi decode số nằm trong các hàm con (chỉ 3 hàm có body mới thấy được).

---

## 2. Bảng tổng hợp toàn bộ SubOp

`switch(SubOp)` phân 3 tầng: `<8`, `<0x0C`, `==0x0C` / `==0xFA`. **Các case tồn tại: `0,1,3,4,5,6,7,9,10,11(0x0B),12(0x0C),250(0xFA)`. Không có `2, 8`, không có `default`.**

| SubOp | Độ dài payload tối thiểu (tầng case) | Wire | Handler (arg1 = object, arg2 = RestPayload) | Tình trạng body |
|---|---|---|---|---|
| `0x00` | **2** (`0B 00`) | pass nguyên ECX | `FUN_00641fa4(*(gvar_007D9CE4), ECX)` | **có body** |
| `0x01` | **2** | pass nguyên ECX | `func_0x0064bb24(*(gvar_007D9CE4), ECX)` | **không có body, không suy diễn** |
| `0x03` | **2** | pass nguyên ECX | `FUN_00644b84(*(gvar_007D9CE4), ECX)` | **có body** |
| `0x04` | **2** | pass nguyên ECX | `func_0x00642244(*(gvar_007D9CE4), ECX)` | **không có body** |
| `0x05` | **2** | pass nguyên ECX | `FUN_00644294(*(gvar_007D9CE4), ECX)` | **có body** |
| `0x06` | **2** | pass nguyên ECX | `func_0x0072c9e8(*(gvar_007D9D34), ECX)` | **không có body** (file `0072c988` tồn tại nhưng là địa chỉ khác) |
| `0x07` | **4** (`0B 07 B1 B2`) — guard `Len<3 → BoundErr(2)` + `Len<2 → BoundErr(1)`, chỉ dùng `B1=ECX[1]` | `[0B][07][B1][B2?]` | `func_0x00733650(*(gvar_007DA7BC=self), B1)` + cờ UI | **không có body** |
| `0x09` | **4** (`0B 09 A B`) — guard `Len<2` và `Len<3` | `[0B][09][A=ECX1][B=ECX2]` | inline: `PlayerRec+0x1308 := A`; nếu `B==1` thì toast số | inline toàn bộ |
| `0x0A` (10) | **2** | pass nguyên ECX | `func_0x0064e978(*(gvar_007D9CE4), ECX)` | **không có body** |
| `0x0B` (11) | **2** | pass nguyên ECX | `func_0x005ba604(*(gvar_007D9D00), ECX)` | **không có body** (file `005ba68c` tồn tại nhưng là địa chỉ khác) |
| `0x0C` (12) | **2** | pass nguyên ECX | `func_0x0060e2a8(*(gvar_007DA4F8), ECX)` | **không có body** |
| `0xFA` (250) | **2** | pass nguyên ECX | `func_0x006438ac(*(gvar_007D9CE4), ECX)` | **không có body** |
| `0x02`, `0x08`, các giá trị khác | — | rơi qua switch, chỉ cleanup chuỗi | no-op | — |

---

## 3. Chi tiết từng SubOp

Quy ước: `P[i]` = byte payload gốc (`P[0]=0x0B`, `P[1]=SubOp`); `R[i]` = RestPayload (`R[0]=P[1]`).

### SubOp `0x00` — `FUN_00641fa4` (CÓ BODY — reset hiệu ứng theo id+kind)
- Wire (tầng case): `[0B][00][rest...]` (≥2 byte, pass nguyên). Parse thật nằm trong `00641fa4_FUN_00641fa4.c:55–58`:
  - `id = DWORD LE của Copy(R,2,4)` = `P[2..5]`; `kind = Word LE của Copy(R,6,2)` = `P[6..7]` → **thực tế cần payload ≥ 8 byte** (`0B 00 id[4] kind[2]`), nếu thiếu thì `_BoundErr` bên trong con.
- Logic core (`00641fa4.c:59–125`):
  - `kind==0` + `id==selfId (*(gvar_007DA7BC+4))` → nhánh self: gọi `FUN_00595a70(InputBar, 0)` (tắt cờ thanh nhập) + các call VMT hiển thị (1 dòng: đóng/refresh form).
  - `kind==0` + `id!=self` → tra actor `FUN_0070c20c(mapScene, id)`, set `actor+0x376=0` (xóa trạng thái), có thể gọi hiệu ứng `FUN_00666bf0` (graphics — 1 dòng).
  - `kind!=0` → tra bảng `gvar_007D9FB4[kind]` (giới hạn `kind ≤ 200`), nếu `slot+4==id` thì `slot+0x376=0`.
- Tóm 1 dòng graphics: mọi hiển thị chỉ qua VMT `+0x24` và `FUN_00666bf0/FUN_00595a70`.

### SubOp `0x01` — `func_0x0064bb24` (KHÔNG BODY)
- Wire: `[0B][01][rest...]`, pass nguyên `ECX` + object `*(gvar_007D9CE4)`.
- Giới hạn: **không có file `0064bb24*` trong `ts_decompile/functions/`** → không biết field, không suy diễn.

### SubOp `0x03` — `FUN_00644b84` (CÓ BODY — toast 4 loại theo 1 byte mã)
- Wire (tầng case): `[0B][03][rest...]` pass nguyên. Parse thật trong `00644b84_FUN_00644b84.c:41–49`: guard `RestLen<2 → BoundErr(1)`, đọc `code = R[1]` (= `P[2]`, 1 byte thường) → **thực tế cần payload ≥ 3 byte**.
- Logic core (dòng 50–61): `code==1/2/3/4` → toast `DAT_00644c7c / 00644cac / 00644cf0 / 00644d14` qua `(gvar_007DA084+0x90)(...,1000,0,0)` (hiển thị 1000ms); các giá trị khác → không làm gì.
- Chuỗi: 4 địa chỉ `0x00644C7C...` **không có file `lit_644c*.hex` trong `redump/` → chưa dịch được, không bịa nội dung.**

### SubOp `0x04` — `func_0x00642244` (KHÔNG BODY)
- Wire: `[0B][04][rest...]`, pass nguyên + object `*(gvar_007D9CE4)`. Không có file body → không suy diễn.

### SubOp `0x05` — `FUN_00644294` (CÓ BODY — đồng bộ danh sách hỗ trợ chiến đấu, record 23 byte)
- Wire (tầng case): `[0B][05][rest...]` pass nguyên. Parse thật trong `00644294_FUN_00644294.c:107–245`:
  - `local_d = R[1]` (= `P[2]`, 1 byte mã loại); số record `n = (RestLen-2)/0x17` (23 thập phân), loop từng record 23 byte từ offset `R[2]`:
  - Mỗi record: `local_e = +0`, `id = DWORD LE +1 (4B)`, `local_22 = Word LE +5 (2B)`, `local_18 = DWORD LE +7 (4B)`, `local_f = +11 (1B)`, `local_10 = +12 (1B)`, 4×Word LE `+13/+15/+17/+19`, `local_24/local_23 = +21/+22 (1B+1B)`.
- Logic core: `switch(local_e)` rẽ `2/3/4/6/7/9/0xB/0xC/0xD/0xE/0xF/0x10/0x12` gọi `FUN_006469b8 / FUN_00646a34 / FUN_00659358` với `(DAT_0098c63c, actor hoặc cache tên, ...)` — đều là cập nhật bảng quản lý chiến đấu + tra actor (`FUN_0070c20c`) / cache tên (`FUN_00722508`). Hiển thị/effect chỉ trong các hàm đó (ngoài phạm vi, tóm 1 dòng).

### SubOp `0x06` — `func_0x0072c9e8` (KHÔNG BODY)
- Wire: `[0B][06][rest...]`, pass nguyên + object `*(gvar_007D9D34)` (object map/scene dùng chung nhiều OP).
- Giới hạn: thư mục chỉ có `0072c988_*` (địa chỉ khác) → **coi như không có body.**

### SubOp `0x07` — cờ UI + lệnh 1 byte cho self (một phần inline, hàm chính KHÔNG BODY)
- Wire: `[0B][07][B1][B2?]`; code yêu cầu `RestLen ≥ 3` nhưng chỉ dùng `B1 = R[1]` (= `P[2]`).
- Logic inline:
  1. Nếu `*(gvar_007DA51C)!=0` thì `*(+0xEB4)=1` (dựng cờ, object chưa định danh — ghi rõ giới hạn).
  2. `func_0x00733650(*(gvar_007DA7BC=self), B1)` — **không có file `00733650*` → không suy diễn B1.**
  3. Nếu `*(gvar_007DA51C)!=0`: `*(InputBar gvar_007DA1DC +0x18C)=0`; `FUN_00595a70(InputBar, 1)`.
- `FUN_00595a70` (có body, `00595a70_FUN_00595a70.c:46–119`): setter cờ `param_1+0x18C := param_2`; đổi skin/button `btn_021/btn_024` qua `FUN_007c9b38+007b0628` và bật/tắt 7 nút `+0x104..+0x114` (UI thuần — tóm 1 dòng theo yêu cầu).

### SubOp `0x09` — ghi cờ + toast số (INLINE TOÀN BỘ, không cần hàm con)
- Wire: `[0B][09][A][B]` (payload đúng 4 byte tối thiểu; `A=R[1]=P[2]`, `B=R[2]=P[3]`).
- Logic:
  - `*(gvar_007DA7BC self +0x1308) := A` (1 byte).
  - Nếu `B==0x01`: `IntToStr(A) + _LStrCatN(3 phần: UNK_00796f1c, số, UNK_00796f44)` → toast `(gvar_007DA084+0x90)(...,2000)` 2000ms. Nếu `B!=1` → chỉ ghi cờ, không toast.
- Chuỗi: `UNK_00796F1C`, `UNK_00796F44` **không có `lit_796f*.hex` trong `redump/` → ghi rõ địa chỉ + chưa dịch được, không bịa.**

### SubOp `0x0A` / `0x0B` / `0x0C` / `0xFA` — pass-through (KHÔNG BODY)
- `[0B][0A][rest...]` → `func_0x0064e978(*(gvar_007D9CE4), ECX)`.
- `[0B][0B][rest...]` → `func_0x005ba604(*(gvar_007D9D00), ECX)`; `gvar_007D9D00` = `TLH_ApparatusMenu` (xác minh `0051189c.c:1774–1775`).
- `[0B][0C][rest...]` → `func_0x0060e2a8(*(gvar_007DA4F8), ECX)`; `gvar_007DA4F8` = `TCY_VenderMenu` (`0051189c.c:1363–1364`).
- `[0B][FA][rest...]` → `func_0x006438ac(*(gvar_007D9CE4), ECX)`.
- Cả 4 đều **không có file body** trong `ts_decompile/functions/` → tầng case chỉ kết luận "forward nguyên rest cho đúng form", mọi parse nằm trong hàm con.

---

## 4. Đối chiếu chiều Client→Server trong `FUN_0077F414`

`0077f414_FUN_0077F414.c:902–910` (`switch(param_2 & 0xFF)`):
```c
case 0xb:
  FUN_00402b90(...); _PStrNCat(...,2); FUN_00402b90(...); _PStrNCat(...,3);
  _LStrFromString(...); TForm1_CY_AddSedQueue(*(gvar_007DA664), ...);
  break;
```
- **Case 11 CÓ builder** (không phải `break` rỗng như OP 0x02/0x09). Nó dựng chuỗi gửi từ các `ShortString` nội bộ rồi đẩy vào hàng đợi gửi `CY_AddSedQueue`. Vì decompile không giữ tên biến nguồn nên chỉ kết luận: **client có thể chủ động gửi OP 0x0B (gói ngắn, không field nghiệp vụ rõ ràng ở tầng này)** — ngược với OP 0x02/0x09 là one-way S→C.

---

## 5. Chuỗi hiển thị / mã hóa tiếng Việt

Tiền lệ đã xác minh ở `opcode_02.md` mục 5: các nhãn kênh chat dump từ `ts_decompile/redump/lit_7ABD*.hex` (Delphi ansistring `[len:4LE][chars]`) giải mã đúng bằng **cp1258 → NFC** (không phải VISCII). Mọi chuỗi toast của OP 0x0B (nếu dump được) cũng phải giải mã theo **cp1258 → NFC**, không phải VISCII → UTF-8.

| Địa chỉ | Nơi tham chiếu | Trạng thái |
|---|---|---|
| `DAT_00644C7C / AC / F0 / 14` | `FUN_00644b84` (SubOp 3, toast 1000ms) | **Chưa có dump `lit_644c*.hex` → chưa dịch được** |
| `UNK_00796F1C / UNK_00796F44` | SubOp 9 inline (toast 2000ms) | **Chưa có dump `lit_796f*.hex` → chưa dịch được** |

Payload trên wire của OP 0x0B **không mang text trực tiếp** (chỉ id/kind/cờ số) — text nằm ở **hằng số trong binary**. Không suy đoán nội dung tiếng Việt khi chưa có bytes (tránh bịa đặt). Cần dump Delphi ansistring tại các địa chỉ trên từ binary gốc rồi giải mã `bytes.decode('cp1258')` + `unicodedata.normalize('NFC', s)`.

---

## 6. Ghi chú cho Mock Server

1. **Frame:** `[F4 44][Len:Word LE][payload]`, `Len` = độ dài payload (không tính 4 byte header); toàn bộ bytes trên socket = plain XOR `0xAD` từng byte.
2. **Bảng frame tính sẵn:**

| Test | Payload plain | Plain frame | Socket (XOR AD) |
|---|---|---|---|
| SubOp 0 min | `0B 00` | `F4 44 02 00 0B 00` | `59 E9 AF AD A6 AD` |
| SubOp 9 (A=5,B=1 → toast) | `0B 09 05 01` | `F4 44 04 00 0B 09 05 01` | `59 E9 A9 AD A6 A4 A8 AC` |
| SubOp 7 (B1=1) | `0B 07 01 00` | `F4 44 04 00 0B 07 01 00` | `59 E9 A9 AD A6 AA AC AD` |

3. **Độ dài an toàn:** SubOp 7/9 bắt buộc đủ 4 byte payload (thiếu → `_BoundErr` crash client ở tầng case). Các SubOp pass-through chỉ cần 2 byte ở tầng case, nhưng hàm con có thể đòi dài hơn: SubOp 0 thực tế cần ≥8 byte (`id`+`kind`), SubOp 3 cần ≥3 byte, SubOp 5 cần `2+23*n` byte. Với 8 SubOp không body, mock nên gửi đúng 2 byte trước (`0B xx`), rồi tăng dần rest nếu client không phản ứng.
4. **Thứ tự test gợi ý:** `0B 09 05 00` (chỉ ghi cờ, an toàn nhất) → `0B 09 05 01` (toast số) → `0B 03 <code 1..4>` (toast 1000ms) → `0B 00 + id/kind` → các pass-through còn lại.
5. **Không gửi SubOp `0x02`/`0x08`** — không có nhánh, rơi qua switch (vô hại nhưng vô nghĩa).

---

## 7. Source trail (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_011_0078D5D1_FUN_0078d5d1.c` | toàn file 150 dòng | Handler chính, danh sách SubOp, guard độ dài |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0xb:` d.2179–2280 | Đối chiếu inline 1:1 với file case |
| 3 | `ts_decompile/case_functions/manifest.csv` | dòng 13 | Case 11, entry `0x0078A9E2` → `0x0078D5D1` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | Case 11 | Xác nhận target |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | dòng 1 | `MainOp 0x0B → case 11` |
| 6 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | dòng 3 | `case 11 → 0x0078D5D1` |
| 7 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | switch d.768; `case 0xb` d.902–910 | Chiều C→S có builder |
| 8 | `ts_decompile/functions/0077eb9c_FUN_0077eb9c.c`, `0077ef7c_FUN_0077ef7c.c` | codec | Word LE / DWORD LE |
| 9 | `ts_decompile/functions/00641fa4_FUN_00641fa4.c` | SubOp 0: `id`+`kind`, core reset | Body SubOp 0 |
| 10 | `ts_decompile/functions/00644b84_FUN_00644b84.c` | SubOp 3: 1 byte mã → 4 toast | Body SubOp 3 |
| 11 | `ts_decompile/functions/00644294_FUN_00644294.c` | SubOp 5: record 23B | Body SubOp 5 |
| 12 | `ts_decompile/functions/00595a70_FUN_00595a70.c` | SubOp 0/7: setter cờ `+0x18C` | Cờ UI |
| 13 | `ts_decompile/functions/0051189c_FUN_0051189c.c` | d.1363–1364, 1774–1775 | `TCY_VenderMenu`, `TLH_ApparatusMenu` |
| 14 | Giới hạn ghi rõ: `0064bb24, 00642244, 0072c9e8, 00733650, 0064e978, 0060e2a8, 006438ac, 005ba604` không có file; `lit_796f1c/796f44/644c*` không có dump | — | Không suy diễn / chưa dịch |

*Ghi chú trung thực: mọi kết luận về 8 SubOp pass-through dừng ở mức "forward nguyên ECX cho đúng object". Nội dung record, chuỗi và toast cần dump thêm từ binary gốc mới dịch được (cp1258→NFC như tiền lệ `opcode_02`), hiện không bịa.*
