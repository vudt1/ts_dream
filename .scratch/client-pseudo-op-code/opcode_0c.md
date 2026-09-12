# PHÂN TÍCH — Main OP 0x0C (Case 12, `FUN_0078d84d` @ `0x0078D84D`) — **Đồng bộ vị trí / trạng thái actor (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (case function + dispatcher inline + codec + chiều C→S).

---

## 0. Kết luận quan trọng nhất

> **Main OP 0x0C KHÔNG có SubOp. Không có `switch`, không có `case label` nào trong handler.**

Toàn file `case_012_0078D84D_FUN_0078d84d.c` (175 dòng) **không chứa một lệnh `switch` nào**, không đọc `payload[1]` như SubOp. Nó parse thẳng **1 struct cố định 12 byte RestPayload** rồi rẽ 2 nhánh `id == self ? self : other`.

Bằng chứng đối chiếu: khối `case 0xc:` trong `ts_decompile/functions/0078a89c_FUN_0078a89c.c` dòng 2281–2397 **khớp từng dòng** với file case riêng (cùng thứ tự `_LStrCopy` + `FUN_0077ef7c/eb9c`, cùng 2 nhánh, cùng các call). Đây là cùng một handler, chỉ khác bối cảnh (dispatcher inline vs. file case tách ra).

Vì vậy mục "SubOp = payload[1] = ECX[0]" **không áp dụng cho OP này** — ghi rõ để tránh dựng Mock Server sai (gửi `[0C][SubOp]...` sẽ bị parse lệch thành `id`).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: server đẩy **1 bản tin định vị actor**: `id (ai) + w1 + X + Y + w4 (trạng thái/hướng?)`, client đồng bộ vào object scene.
- **2 nhánh duy nhất**:
  - `id == myId (*(gvar_007DA7BC+4))` → cập nhật **object self**, set 2 cờ, gọi `func_0x007994a4`.
  - `id != myId` → cập nhật **cache tên + actor khác**: `FUN_00729f88`, `func_0x00722578 / func_0x0072274c`, rồi nếu actor đã tồn tại trong scene (`FUN_0070c20c != 0`) thì đồng bộ tọa độ + trạng thái + hiển thị.
- **Không mang chuỗi/text nào.** Toàn bộ payload là số LE. Không có chat, không có memo.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

**Entry**: `FUN_0078d84d` @ `0x0078D84D` (Case 12, jump-target `0x0078A9E6` của bảng `0x78A9B6`).

**Dispatcher:** `FUN_0078a89c` tra bảng byte `0x78A8EE` lấy index, rồi tra bảng dword `0x78A9B6` để nhảy tới hàm xử lý (`case_001` đến `case_065`). MainOp `0x0C` → Case 12 → `FUN_0078d84d`.

**Khung mạng:** Header `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`. Toàn bộ frame XOR khóa tĩnh `0xAD`. Server speaks first.

**Mapping** (xác minh 2 nguồn):
- `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` dòng 1: `01 02 03 04 05 06 07 08 09 0A 00 0B 0C 0D 0E 0F` → `byte_table[0x0C] = 0x0C`.
- `manifest.csv` dòng 14: `12,0x0078A9E6,0x0078D84D,EXPORTED,"FUN_0078d84d"`.
- `jumptable_0x78A9B6_cases.c` mục "Case index: 12 / Target 0x0078D84D" chứa body y hệt file case riêng.

**Đầu handler** (file case dòng 27–41, dispatcher dòng 2282–2294):

```c
_LStrCopy(ECX,1,4,&tmp); id = FUN_0077ef7c(EAX,&tmp);   // ECX[0..3]
_LStrCopy(ECX,5,2,&tmp); w1 = FUN_0077eb9c(EAX,&tmp);   // ECX[4..5]
_LStrCopy(ECX,7,2,&tmp); w2 = FUN_0077eb9c(EAX,&tmp);   // ECX[6..7]
_LStrCopy(ECX,9,2,&tmp); w3 = FUN_0077eb9c(EAX,&tmp);   // ECX[8..9]
_LStrCopy(ECX,0xb,2,&tmp); w4 = FUN_0077eb9c(EAX,&tmp); // ECX[10..11]
```

**Quy ước index** (giống `opcode_02/09.md`):
- `payload[k]` 0-based, `payload[0] = 0x0C`.
- `ECX[r]` (RestPayload, đã cắt MainOp) = `payload[r+1]`.
- `_LStrCopy(ECX,p,n)` = Delphi `Copy` **1-based** → `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`.
- Codec: `FUN_0077eb9c` = **Word LE** (`b0 + b1*256`); `FUN_0077ef7c` = **DWORD LE** (`b0+b1*256+b2*65536+b3*2^24`). Mỗi codec có guard `_BoundErr` nếu chuỗi ngắn hơn số byte cần đọc.

---

## 3. Bảng tổng hợp SubOp

| SubOp | Tồn tại? | Độ dài tối thiểu |
| :--- | :--- | :--- |
| — | **KHÔNG có `switch(SubOp)`, KHÔNG có case label nào** | — |

Chỉ có **1 layout duy nhất**:

| Layout | Payload tối thiểu | Nội dung |
| :--- | :--- | :--- |
| fixed-12 | **13 bytes**: `[0C] + 12 bytes Rest` | `id:4B LE + w1:2B LE + X:2B LE + Y:2B LE + w4:2B LE` |

Gửi thiếu byte → codec gọi `_BoundErr`, không phải no-op.

---

## 4. Chi tiết wire layout + logic core

Quy ước: `P[.]` = payload gốc (`P[0]=0x0C`); `RP[.]` = RestPayload (`RP[i]=P[i+1]`). LE.

```
[0]=0x0C
[1..4]  id: DWORD LE  (_LStrCopy RP,1,4  + FUN_0077ef7c)
[5..6]  w1: Word LE   (_LStrCopy RP,5,2  + FUN_0077eb9c)
[7..8]  X:  Word LE   (_LStrCopy RP,7,2  + FUN_0077eb9c)
[9..10] Y:  Word LE   (_LStrCopy RP,9,2  + FUN_0077eb9c)
[11..12] w4: Word LE  (_LStrCopy RP,0xb,2 + FUN_0077eb9c)
```

### Nhánh A — `id == myId` (dòng 42–67): cập nhật self

```c
if (*(gvar_007DA7BC+4) == id) {
  *(EAX+4)=w1; *(EAX+8)=X; *(EAX+0xc)=Y; *(EAX+0x10)=id; *(EAX+0x14)=w4;
  *(gvar_007DA5A0+0xc)=1; *(gvar_007DA37C+0xc)=1;
  cRam009f1650='\0'; *(gvar_007D9D34+0x53fc)=0;
  if (cRam009f1650=='\0') func_0x007994a4(EAX);   // luôn chạy nhánh này
  else { *(EAX+0x18)=3; FUN_0050f2f8(form,3); ... } // dead: vừa gán '\0' ở trên
}
```

- Ghi 5 field vào object `EAX`, bật 2 cờ, gọi `func_0x007994a4(EAX)`.
- Nhánh `else` là **dead code trên thực tế** vì `cRam009f1650` vừa bị gán `'\0'` ngay trước `if`.
- Giới hạn: `func_0x007994a4` **không có file body** trong `ts_decompile/functions/` — chỉ biết signature `(obj)`.

### Nhánh B — `id != myId` (dòng 68–143): cập nhật actor khác

1. `FUN_00729f88(gvar_007D9C48, id, (short)w1)` — ghi `w1` vào `+0x1a` của record cache tên (`gvar_007DA6BC`, qua `FUN_00722508`; xem `00729f88_FUN_00729f88.c:24-30`).
2. `if (w1 == *(ushort*)(self+0x63a)) func_0x00722578(map,id) else func_0x0072274c(map,id)` — 2 hàm xử lý map/scene theo map-id, **không có body, không suy diễn**.
3. `idx = FUN_0070c20c(map, id)` — tìm actor trong mảng 800 slot `gvar_007DA300` bằng cách so `*(obj+4)==id`; `0` = chưa có → dừng (không tạo mới ở OP này).
4. Nếu `idx != 0`, với `obj = gvar_007DA300[idx]`:
   - `FUN_00722950(cache, id, obj)` — copy record cache (tên/class/byte trạng thái) vào object.
   - `FUN_0071e2b8(obj, X, Y)` — đặt tọa độ tuyệt đối + tọa độ tương đối so với camera (hiển thị: 1 dòng).
   - `FUN_0070d86c(obj)` — đặt byte trạng thái `+0x378` + 2 float theo map-id (1 dòng).
   - `FUN_0071e080(obj, obj+0x4b0, obj+0x4b1)` — ghi 2 byte liên quan, quản lý `TCartNpc` ở `+0x61c` (1 dòng).
   - `FUN_0071fae0(obj, X, Y)` — VMT `+0x18` + rải 5 điểm lân cận ngẫu nhiên quanh X/Y (1 dòng).
   - `FUN_004c9bf0(*(obj+4))` — phân loại class số; nếu `(class-5)<4` (tức class 5..8) thì `obj+8=0, obj+0x7c=0x78`.

---

## 5. Codec phụ

| Hàm | File body | Kết luận |
| :--- | :--- | :--- |
| `FUN_0077eb9c` Word LE | `0077eb9c_FUN_0077eb9c.c:127-187` | `b0 + b1*256`, guard len<2 → `_BoundErr`. **Dùng 4 lần** trong handler. |
| `FUN_0077ef7c` DWORD LE | `0077ef7c_FUN_0077ef7c.c:162-249` | `b0+b1*256+b2*65536+b3*2^24`, guard từng byte. **Dùng 1 lần** (id). |
| `FUN_0077eb1c` | `0077eb1c_FUN_0077eb1c.c:64-100` | **Encode** Word→chuỗi 2 byte (chiều C→S). Handler này không gọi. |
| `FUN_0077ee84` | `0077ee84_FUN_0077ee84.c:68-124` | **Encode** DWORD→chuỗi 4 byte (chiều C→S). Handler này không gọi. |
| `FUN_0077f098` | `0077f098_FUN_0077f098.c:28-63` | Copy tên 8 byte. Handler này không gọi. |

---

## 6. Chiều ngược lại Client → Server (C→S)

`ts_decompile/functions/0077f414_FUN_0077F414.c` dòng 911–912:

```c
case 0xc:
  break;
```

**OP 0x0C phía gửi là rỗng / no-op.** Client không bao giờ chủ động gửi OP 0x0C; chỉ nhận S→C. Không có builder payload nào cho OP này (giống OP 0x02/0x09).

---

## 7. Chuỗi hằng / tiếng Việt

- Handler **không tham chiếu bất kỳ hằng chuỗi nào** (chỉ có `gvar_*`, `cRam009f1650`, và `&UNK_007994a4` — đây là **con trỏ code callback**, không phải chuỗi).
- Do đó **không có gì để tra `ts_decompile/redump/lit_*.hex`**, không giải mã cp1258/VISCII nào cho OP này. Ghi rõ để tránh bịa đặt (khác OP 0x02 có 14 nhãn kênh đã dịch). Mã hóa đúng của game là **cp1258 → NFC** theo tiền lệ `opcode_02.md`, không phải VISCII.

---

## 8. Ghi chú cho Mock Server

1. **Không có SubOp.** Gửi đúng 13-byte payload: `[0C][id 4B][w1 2B][X 2B][Y 2B][w4 2B]`, tất cả LE.
2. **Frame**: `F4 44 | Len:Word LE (=13) | payload`, toàn bộ bytes từ Token tới payload XOR `0xAD` từng byte.
3. **Ví dụ tính sẵn** (plain XOR AD = socket):
   - Payload plain (id=1, w1=1, X=100, Y=200, w4=2): `0C 01 00 00 00 01 00 64 00 C8 00 02 00`
   - Plain frame (17B): `F4 44 0D 00 0C 01 00 00 00 01 00 64 00 C8 00 02 00`
   - Socket (XOR AD): `59 E9 A0 AD A1 AC AD AD AD AC AD C9 AD 65 AD AF AD`
4. **Kịch bản test**: gửi `id != myId` + `w1 == mapId` trước (nhánh actor khác, an toàn nếu actor chưa tồn tại → dừng ở `FUN_0070c20c==0`); rồi gửi `id == myId` (nhánh self, gọi `func_0x007994a4` — chưa rõ body, nên quan sát crash/log).
5. **Không cần** mock chiều C→S cho OP 0x0C.

---

## 9. Source trail + giới hạn trung thực (chỉ `ts_decompile/`)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `ts_decompile/case_functions/functions/case_012_0078D84D_FUN_0078d84d.c` | toàn file 175 dòng; parse d.27–41; nhánh self d.42–67; nhánh other d.68–143 | Handler chính, wire layout, logic 2 nhánh |
| 2 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 0xc:` d.2281–2397 | Đối chiếu inline khớp 1:1 với file case |
| 3 | `ts_decompile/case_functions/manifest.csv` | d.14 | Case 12, entry `0x0078A9E6` → `0x0078D84D` |
| 4 | `ts_decompile/case_functions/jumptable_0x78A9B6_cases.c` | mục Case 12 | Xác nhận mapping lần 2 |
| 5 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | hàng 1 | `byte_table[0x0C]=0x0C` |
| 6 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `case 0xc: break;` d.911–912 | C→S rỗng |
| 7 | `0077eb9c_FUN_0077eb9c.c` / `0077ef7c_FUN_0077ef7c.c` | body codec | Word LE / DWORD LE + guard |
| 8 | `0077eb1c / 0077ee84 / 0077f098` | body encode/copy | Xác nhận không dùng ở chiều S→C này |
| 9 | `00729f88 / 0070c20c / 00722950 / 0071e2b8 / 0070d86c / 0071e080 / 0071fae0 / 004c9bf0 / 0050f2f8` | body từng file | Logic core nhánh other + phân loại |

**Giới hạn (không suy diễn)**:
- `func_0x007994a4`, `func_0x00722578`, `func_0x0072274c` **không có file body** trong `ts_decompile/functions/` — chỉ biết signature từ call-site.
- `UNK_007994a4` là địa chỉ code, không phải chuỗi — chưa resolve.
- Ý nghĩa game-design của `w1` (so với map-id `+0x63a`), `w4`, `X/Y` (pixel hay tile) nằm ngoài decompile tầng case — chỉ kết luận ở mức "đồng bộ actor".
