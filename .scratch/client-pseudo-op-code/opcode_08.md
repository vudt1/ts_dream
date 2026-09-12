# PHÂN TÍCH — Main OP 0x08 (Case 9) `FUN_0078d125` @ `0x0078D125` — **Stats / Chỉ số nhân vật (Server → Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code`
Trạng thái: **Đã xác minh từ decompile case function + dispatcher inline + các helper ghi struct nhân vật**. Giả định "Stats" trong handoff là **ĐÚNG cho 4/8 sub-op (1,2,3,4)**; 4 sub-op còn lại (5,6,7,8) là **nhóm bộ đếm tài nguyên / thanh HUD phụ + thông báo dạng số**, tức OP 0x08 thực chất là **"Nhân vật & chỉ số" (Stats + counters + notice)**, không phải thuần Stats.

---

## 1. Tóm tắt nghiệp vụ

| Nhánh | Bản chất | Có phải "Stats"? |
|---|---|---|
| SubOp `0x01` | Ghi 1 chỉ số của **chính nhân vật local** (HP/SP/INT/ATK/DEF/AGI/EXP/Level/MaxHP…) | **CÓ** (core) |
| SubOp `0x02` | Ghi 1 chỉ số cho **1 thành viên tổ đội** (`player+0x57c[slot]`, 5 slot) — gate `type==4` | **CÓ** |
| SubOp `0x03` | Ghi **bản sao chỉ số** của một actor trong **cache 2100 slot** (tìm theo charID) — dùng cho nameplate/tooltip | **CÓ** |
| SubOp `0x04` | Ghi 1 chỉ số cho **follower/NPC đi theo** (`gvar_007D9FB4[slot]`, 201 slot), có gate điều kiện | **CÓ** |
| SubOp `0x05` | Đồng bộ **4 bản ghi 12 byte** tại `player+0x515/0x521/0x52d/0x539` (Word id + Byte) → vẽ lại icon trên panel `gvar_007DA238` | KHÔNG (counter/buff/tech) |
| SubOp `0x06` | Ghi **1 trong 4 giá trị** tại `player+0x502 / 0x508 / 0x50e / 0x513` | KHÔNG (point/tài nguyên) |
| SubOp `0x07` | Ghi **1 trong 5 giá trị** tại `player+0x500 / 0x506 / 0x50c / 0x512(=250) / 0x4fc` | KHÔNG (max-cap của 6 trên) |
| SubOp `0x08` | In **1 trong 5 mẫu thông báo có số** vào log/chat (`gvar_007DA1B0`) | KHÔNG (notice) |

Không có nhánh nào **đọc** từ client: OP 0x08 là **100% server-push** (xem mục 5).

---

## 2. Entry & cách đọc PacketBuffer

### 2.1. Đường vào
1. `ClientSocket1Read` → XOR `0xAD` (`FUN_0050a248`) → deframe `[44 F4][Len:Word LE][Payload]` → `TForm1.CY_AddRevQueue`.
2. `CY_DelRevQueue` (tick ~30ms) tách `MainOp = payload[0]` (1-based char 1) và `RestPayload = Copy(payload,2,Len-1)` → `FUN_0078a89c(EAX=TFConnect, DL=MainOp, ECX=RestPayload)`.
3. Dispatcher: `byte_table[0x78A8EE][0x08] = 0x09` → `jumptable[0x78A9B6][9] = 0x0078D125` → `FUN_0078d125`.
   (Xác minh chéo: khối `case 9:` trong `ts_decompile/functions/0078a89c_FUN_0078a89c.c` **inline đúng** logic của file case riêng — hai bản khớp nhau 100%.)

### 2.2. Quy ước index (RẤT QUAN TRỌNG — tránh off-by-one)
- `payload[k]`: byte thứ `k` của payload gốc, **0-based**, `payload[0] = 0x08`.
- `ECX[r]` (0-based) = `payload[r+1]`.
- `_LStrCopy(ECX, p, n, &tmp)` = `Delphi Copy(S, p, n)` **1-based** → lấy `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`.
- `*(byte*)(ECX + r)` đi kèm `_BoundErr` với điều kiện `Length < r+1` → đọc `ECX[r]` = `payload[r+1]`.
- Codec: `FUN_0077eb9c` = 2 byte → **Word LE** (`b0 + b1*256`); `FUN_0077ef7c` = 4 byte → **DWORD LE**.

### 2.3. Đọc SubOp trong handler (dòng 35–42)
```c
iVar7 = *(int *)(unaff_EBP + -0xc);                 // ECX = RestPayload
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar7 + 0);  // SubOp = ECX[0] = payload[1]
switch (*(undefined4 *)(unaff_EBP + -0x14)) { case 1..8 }
```
→ **`SubOp = payload[1]`**, chỉ có `0x01..0x08`; không có `default` (mã lạ → rơi xuống epilogue `_LStrArrayClr` dọn chuỗi, không làm gì).

### 2.4. Mechanism "dấu" chung cho SubOp 1/2/3
Cả 3 nhánh đều đọc 1 byte `deltaType` ngay sau `attrCode`, rồi:
```c
cVar1 = deltaType + -2;
if (cVar1 == 0) value1 = -value1;   // deltaType == 2  ⇒ đảo dấu (two's complement)
```
Nghĩa là: **server gửi độ lớn + cờ hướng**; `deltaType == 2` = "giảm". Với stat kiểu Word, `FUN_00710ab0` clamp `< 0 → 0`, `> 0xFFFF → 0xFFFF` (nên giá trị âm chỉ có nghĩa với stat DWORD).

---

## 3. Bảng tổng hợp SubOp

| SubOp | Hàm xử lý | Độ dài payload | Nội dung | Đối tượng ghi |
|:---:|---|:---:|---|---|
| `0x01` | inline + `FUN_00710ab0` | **12** | `attrCode, deltaType, value1, value2` | **player local** `*(gvar_007DA7BC)` |
| `0x02` | inline + `FUN_00710ab0` | **15** | `type=4, slot:Word, attrCode, deltaType, value1, value2` | **party slot 0..4** `player+0x57c[slot]` |
| `0x03` | inline + `FUN_00722508` + `FUN_00731c0c` | **16** | `charID, attrCode, deltaType, value1, value2(loại)` | **cache actor** `gvar_007DA6BC[idx]` |
| `0x04` | `FUN_0073b458` + `FUN_00710ab0` | **8** | `slot:0..200, attrCode, value1` | **follower** `gvar_007D9FB4[slot]` |
| `0x05` | `FUN_0074c340` | **14** | 4 × `[value:Word LE][flag:Byte]` | `player+0x515/0x521/0x52d/0x539` + HUD |
| `0x06` | `FUN_0074c4c8` | **5** | `sel:1..4, value:Word LE` | `player+0x502/0x508/0x50e/0x513` |
| `0x07` | `FUN_0074cc84` | **5** | `sel:1..5, value:Word LE` | `player+0x500/0x506/0x50c/0x512/0x4fc` |
| `0x08` | `FUN_0074cfd4` | **4** | `template:1..5, number:Byte` | log/chat `gvar_007DA1B0` |

---

## 4. Chi tiết từng SubOp

### 4.1. SubOp `0x01` — Ghi chỉ số cho **nhân vật local** (CORE STATS)

**Wire layout (12 bytes)**
```
[0]=0x08  [1]=0x01  [2]=attrCode:1B  [3]=deltaType:1B  [4..7]=value1:DWORD LE  [8..11]=value2:DWORD LE
```
**Cách đọc (case_009 @ 0x0078D140..0x0078D218)**
| Trường | Code | Suy ra |
|---|---|---|
| `deltaType` | `iVar2=2; *(EBP-0x41) = ECX[2]` | `payload[3]` |
| `value1` | `_LStrCopy(ECX,4,4)` → `FUN_0077ef7c` | `payload[4..7]` (đảo dấu nếu `deltaType==2`) |
| `value2` | `_LStrCopy(ECX,8,4)` → `FUN_0077ef7c` | `payload[8..11]` |
| `attrCode` | `iVar2=1; *(byte*)(ECX+1)` | `payload[2]` |

**Gọi:** `FUN_00710ab0(*(int**)gvar_007DA7BC /*player*/, attrCode, value1 /*param_3*/, value2 /*param_4*/)`
**Sau khi ghi:** `if (0xc < *(byte*)(player + 0x43e)) FUN_00596a7c(gvar_007DA1DC, 1);` → nếu byte chỉ-số-ở-`0x43e` > 12 thì HUD đổi layout ô nhập sang bitmap `"main_input_3"` (thay vì `"main_input_1"`), reset 6 ô con `+0x138[i] → 0x22e`. (Chỉ 1 dòng: **presentation, không phải logic**.)

---

### 4.2. `FUN_00710ab0` — "Stat Setter" (bảng mapping offset → chỉ số)

Signature: `FUN_00710ab0(int *actor, byte attrCode, uint value, uint aux)`.
Byte `+0x79` của actor = kind: `1 = TPlayer`, `4 = TFollowNpc` (VMT `0x70B740` / `0x70B354`).
Toàn bộ ánh xạ `attrCode → offset actor` (đã đối chiếu bằng getter đối xứng `FUN_007108f4` @ `0x007108F4`):

| attrCode | Offset actor | Kích thước | Tên suy luận (bằng chứng) | Ghi chú logic |
|:---:|---|:---:|---|---|
| `0x18` | `0x3e9` | Byte | **Class/Job ID** (giá trị 1..4) | `0057922c/0077d3b4/0059f090` so sánh với 1..4 để lọc skill-list theo nghề |
| `0x19` | `0x3ea` | Word | **HP hiện tại** | `FUN_006313d0`: `IntToStr(@0x3ea) + "/" + IntToStr(@0x404)` gán nhãn `"HP"`; clamp `[0..0xFFFF]` |
| `0x1a` | `0x3ec` | Word | **SP (MP) hiện tại** | `"SP"` = `@0x3ec / @0x406`; refresh form `gvar_007D9EB8` |
| `0x1b` | `0x3ee` | Word | **INT (cơ bản)** | `FUN_005e64d8` → `icon_INT`; tổng = `0x3ee + 0x414` |
| `0x1c` | `0x3f0` | Word | **ATK (cơ bản)** | → `icon_ATK`; tổng = `0x3f0 + 0x40c` |
| `0x1d` | `0x3f2` | Word | **DEF (cơ bản)** | → `icon_DEF`; tổng = `0x3f2 + 0x410` |
| `0x1e` | `0x3f4` | Word | **AGI (cơ bản)** | → `icon_AGI`; tổng = `0x3f4 + 0x418 (+FUN_007507c8)` |
| `0x1f` | `0x3f6` | Word | **HPX (HP bonus)** | → `icon_HPX`; **tính lại MaxHP** (`FUN_0065b6ac`) |
| `0x20` | `0x3f8` | Word | **SPX (SP bonus)** | → `icon_SPX`; **tính lại MaxSP** (`FUN_0065b6e8`) |
| `0x23` | `0x3fa` | Byte | Cờ chế-nghiền công thức HP/SP (job-state) | **tính lại CẢ MaxHP + MaxSP**; set `*(gvar_007DA530+0x1b9)=1` |
| `0x24` | `0x3fc` | DWORD | **EXP hiện tại** *(suy luận)* | panel `FUN_00593428` hiển thị ở slot 4 & 0xd; `007a6008` nạp từ bảng EXP-theo-Level `FUN_0065b0dc` |
| `0x25` | `0x400` | Word | **Level** *(suy luận)* | `0059f090:560` dùng `actor+0x400` để xét "đủ cấp dùng skill"; mở/refresh form `gvar_007D9EB8` |
| `0x26` | `0x402` | Word | **Điểm thuộc tính chưa dùng** *(suy luận)* | panel slot 5 |
| `0x32` | `0x42e + aux*2` | Word[6] | Mảng 6 thanh phụ (aux = index 0..5) | panel sao chép sang `0x400 + idx*2` |
| `0x3e` | `0x43e` | Byte | Chỉ số kích hoạt đổi HUD layout (handler so `> 12`) *(suy luận: cấp/tier)* | panel slot 0xe |
| `0x3f` | `0x440` | DWORD | Chỉ số mở rộng *(không đọc được UI)* | — |
| `0x40` | `0x55e` | Byte | Chỉ số **follower** (intimacy/energy) | chỉ khi kind==4; sinh toast "tăng/giảm N" (`DAT_007118ac/007118c8`) |
| `0x42` | `0x43c` | Word | **TemplateID** dùng tra bảng definition | `FUN_004deb28(gvar_007D9FCC, @0x43c, 0xCF/0xD0)` trong công thức HP/SP |
| `0x6e` | `0xfd4 + aux*3` | Word+Byte | Bảng 256 slot trang bị/kỹ năng (id + flag) | chỉ kind==1; item `0x36b3` → sound `sound\WA0014.wav` |
| `0xcd` | `0x404` | Word | **MaxHP (trực tiếp)** | `0x3ea/0x404` = cặp "HP x/y" |
| `0xce` | `0x406` | Word | **MaxSP (trực tiếp)** | `0x3ec/0x406` = cặp "SP x/y" |
| `0xcf` | `0x41c` | DWORD | Số hạng MaxHP → **tính lại `0x404`** | |
| `0xd0` | `0x420` | DWORD | Số hạng MaxSP → **tính lại `0x406`** | |
| `0xd1` | `0x42c`=aux, `0x42d`=value | Byte×2 | Buff tạm: tag `0xDA`→áp vào HP, `0xDB`→áp vào SP | 2 getter `0x404/0x406` đọc `0x42c/0x42d` |
| `0xd2` | `0x40c` | DWORD | Bonus ATK | |
| `0xd3` | `0x410` | DWORD | Bonus DEF | |
| `0xd4` | `0x414` | DWORD | Bonus INT | |
| `0xd6` | `0x418` | DWORD | Bonus AGI | |
| `0xd7` | `0x408` | Word | Tham số hiệu chỉnh MaxHP → **tính lại `0x404`** | |
| `0xd8` | `0x40a` | Word | Tham số hiệu chỉnh MaxSP → **tính lại `0x406`** | |
| `0xda` | `0x424` | DWORD | **HP base** → **tính lại `0x404`** | panel slot 0x19 |
| `0xdb` | `0x428` | DWORD | **SP base** → **tính lại `0x406`** | panel slot 0x1a |

**Công thức tính lại MaxHP (`FUN_0065a300` @ `0x0065A300`) — evidence cho cặp 0x404/0x3f6:**
```
tmp   = dword[+0x424]                       // HP base (code 0xda)
if byte[+0x42c] == 0xDA: tmp += byte[+0x42d]
hp0   = word[+0x3f6] + tmp   (clamp >= 0)   // HPX (code 0x1f)
bonus = Table[gvar_007D9FCC][word[+0x43c]][0xCF] + dword[+0x41c]
MaxHP = FUN_0065a554(engine, byte[+0x3fa], hp0, byte[+0x4b0], byte[+0x4b1], word[+0x408], bonus)
→ word[+0x404]
```
MaxSP (`FUN_0065aabc`) đối xứng: `0x428 / 0x3f8 / 0x420 / 0x40a`, bảng code `0xD0`, → `word[+0x406]`.

> Lưu ý ADR-0001: mọi con số trên là **client prediction / presentation** — client không tự quyết, server vẫn là nguồn đúng.

---

### 4.3. SubOp `0x02` — Ghi chỉ số cho **thành viên tổ đội**

**Wire layout (15 bytes)**
```
[0]=0x08 [1]=0x02 [2]=0x04(=target-type, bắt buộc) [3..4]=slot:Word LE [5]=attrCode:1B
[6]=deltaType:1B [7..10]=value1:DWORD LE [11..14]=value2:DWORD LE
```
**Logic:** gate `ECX[1] == 4`, nếu khác → bỏ qua (rơi epilogue). `slot` clamp `0..4` (`if (4 < uVar3) _BoundErr`).
`piVar4 = *(int**)(player + 0x57c + slot*4)` → chính là **object actor của thành viên** cùng layout `TPlayer` (chứng minh: `007a6008` ghi `player+0x57c[i] → +0x3fa/+0x3fc/+0x3ee`, `0052e1b8` đọc tên tại `+9`).
Sau đó `FUN_00710ab0(member, ECX[4] /*payload[5] attrCode*/, value1, value2)` — dùng **chung bảng stat ở 4.2**.

---

### 4.4. SubOp `0x03` — Cập nhật **bản sao chỉ số actor trong cache** (nameplate)

**Wire layout (16 bytes)**
```
[0]=0x08 [1]=0x03 [2..5]=charID:DWORD LE [6]=attrCode:1B [7]=deltaType:1B
[8..11]=value1:DWORD LE [12..15]=value2:DWORD LE (đọc nhưng BỊ BỎ)
```
**Logic:**
1. `idx = FUN_00722508(*(gvar_007D9C48), charID)` → quét **2100 slot** `gvar_007DA6BC[1..0x834]`, so `*(actor+4) == charID` → trả về index, `0` nếu không tìm thấy (→ nhánh `break`, không ghi gì).
2. Nếu tìm thấy: `bVar8 = ECX[5] /*payload[6] attrCode*/`, `value1` đã đảo dấu nếu `deltaType==2`; `payload[12..15]` được giải mã qua `FUN_0077ef7c` nhưng **không dùng** (field dự phòng của server).
3. `FUN_00731c0c(gvar_007DA6BC[idx] /*cache entry*/, attrCode, value1)` — ghi vào **struct mirror** nhỏ hơn:

| attrCode | Offset entry | Kích thước | Mirror của actor |
|:---:|---|:---:|---|
| `0x19` | `0x20` | Word (clamp `[0..0xFFFF]`) | HP hiện tại (`actor+0x3ea`) |
| `0x1a` | `0x22` | Word | SP hiện tại (`actor+0x3ec`) |
| `0x1f` | `0x24` | Word | HPX (`actor+0x3f6`) |
| `0x20` | `0x26` | Word | SPX (`actor+0x3f8`) |
| `0x23` | `0x1e` | Byte | cờ `0x3fa` — **đồng thời** ghi lan vào cache `gvar_007DA6E8` (stride `0x4e`, so khớp ID tại `+0`) |
| `0xcf` | `0x28` | DWORD | `actor+0x41c` |
| `0xd0` | `0x2c` | DWORD | `actor+0x420` |
| `0xd7` | `0x30` | Word | `actor+0x408` |
| khác | — | — | **bỏ qua im lặng** |

→ Bản chất: sub-op 3 **không vẽ UI**, chỉ giữ snapshot HP/SP của actor khác để nameplate/tooltip đọc.

---

### 4.5. SubOp `0x04` — Ghi chỉ số cho **follower / NPC đi theo** (`FUN_0073b458` @ `0x0073B458`)

**Wire layout (8 bytes)**
```
[0]=0x08 [1]=0x04 [2]=slotIdx:1B (0..200) [3]=attrCode:1B [4..7]=value1:DWORD LE
```
**Gate bắt buộc:** `FUN_00634f3c(*(gvar_007D9C70), *(ushort*)(player + 0x63a), 3) != 0`
→ hàm này tra bảng definition (stride `0xab`, tại `DAT_0098be0e`) xem **TemplateID `player+0x63a`** có thuộc nhóm flag `0x03` hay không. Nếu không, **toàn bộ sub-op bị bỏ qua**.
**Ghi:** `if (gvar_007D9FB4[slotIdx] != 0) FUN_00710ab0(ThatFollower, attrCode /*ECX[2]*/, value1, 0)`
→ `param_4 = 0` nên các attrCode dùng aux (`0x32`, `0x6e`, `0xd1`) sẽ ghi index 0. Danh sách `gvar_007D9FB4` (201 con trỏ) chính là tập follower/NPC (chung layout với `TFollowNpc`, thấy ở `006352c8` đọc `+0x550/+0x55a`).

---

### 4.6. SubOp `0x05` — 4 bản ghi counter (`FUN_0074c340` @ `0x0074C340`)

**Wire layout (14 bytes) = 4 record × 3 byte**
```
[0]=0x08 [1]=0x05
[2..3]=id1:Word LE  [4]=flag1:Byte
[5..6]=id2:Word LE  [7]=flag2:Byte
[8..9]=id3:Word LE  [10]=flag3:Byte
[11..12]=id4:Word LE [13]=flag4:Byte
```
**Ghi (player struct, mảng 4 phần tử stride 12 byte, base `0x509`):**
```
for k = 1..4:
    *(short*)(player + 0x509 + k*12) = id_k      // 0x515, 0x521, 0x52d, 0x539
    *(char *) (player + 0x50b + k*12) = flag_k   // +2 trong record
    if (id_k != 0) FUN_005ab678(gvar_007DA238, k, id_k, 0);   // cập nhật icon/nhãn ô thứ k
FUN_005b3f94(gvar_007DA114);                     // refresh panel cha
```
Bằng chứng record này có thêm trường thời gian/số học: `00527674` ghi `*(double*)(player + 0x50c + k*12)` từ `Now()` và set byte `+0x514 + k*12 = 1` ⇒ record = `{Word id, ?, double timestamp, byte dirty}` ⇒ **nhóm buff/tech/counter có timestamp**, **không phải HP/EXP**.

---

### 4.7. SubOp `0x06` — 1 trong 4 bộ đếm (`FUN_0074c4c8` @ `0x0074C4C8`)

```
[0]=0x08 [1]=0x06 [2]=sel:1B [3..4]=value:Word LE
```
| sel | Ghi | Loại |
|:---:|---|---|
| 1 | `*(ushort*)(player+0x502) = value` | current (cặp với `0x500`) |
| 2 | `*(ushort*)(player+0x508) = value` | current (cặp với `0x506`) |
| 3 | `*(ushort*)(player+0x50e) = value` | current (cặp với `0x50c`) |
| 4 | `*(char*)(player+0x513) = (char)value` | byte state |

---

### 4.8. SubOp `0x07` — 1 trong 5 giá trị "gốc" (`FUN_0074cc84` @ `0x0074CC84`)

```
[0]=0x08 [1]=0x07 [2]=sel:1B [3..4]=value:Word LE
```
| sel | Ghi | Ý nghĩa |
|:---:|---|---|
| 1 | `*(ushort*)(player+0x500) = value` | max/cap #1 |
| 2 | `*(ushort*)(player+0x506) = value` | max/cap #2 |
| 3 | `*(ushort*)(player+0x50c) = value` | max/cap #3 |
| 4 | `*(char*)(player+0x512) = 0xFA` (**bỏ qua value**) | reset = 250 |
| 5 | `*(uint*)(player+0x4fc) = value` | selector/index đang chọn |

Bằng chứng cặp `(0x500,0x502) (0x506,0x508) (0x50c,0x50e)`: `0074b7c8:268` `0x508 := 0x506`, `0x50e := 0x50c`, `0x502 := 0x500` (khi **reset đầy**), và `00527674` **trừ chi phí** vào `0x502 / 0x508 / 0x50e` khi mua/đổi (costs lấy từ `+0xbcc/0xbd0/0xbd4`) ⇒ **nhóm point/resource của HUD**, không thuộc khối HP/SP (HP/SP nằm ở `0x3ea/0x3ec`, Max ở `0x404/0x406`).

---

### 4.9. SubOp `0x08` — Thông báo dạng số (`FUN_0074cfd4` @ `0x0074CFD4`)

```
[0]=0x08 [1]=0x08 [2]=template:1B (1..5) [3]=number:Byte
```
Ghép chuỗi `DAT_0074d1f4 + IntToStr(number) + <suffix[template]>` với suffix tại `DAT_0074d1bc / 0x21c / 0x254 / 0x28c / 0x2c4`, rồi:
`FUN_007ab870(*(gvar_007DA1B0), 0, str, 0)` → **đẩy message vào cửa sổ log/chat** (cùng helper dùng bởi OP chat 0x02). Không ghi struct nhân vật. `param_1 = *(gvar_007D9D34)` được truyền nhưng **không dùng**.

---

## 5. Chiều Client → Server (C→S) của OP 0x08

Đối chiếu `ts_decompile/functions/0077f414_FUN_0077F414.c` (`TFConnect.SendCommand`, `switch(param_2 & 0xff)` dòng 768–1079):

```c
case 7:   ... FUN_0077eb1c ×3 + _LStrCatN(...) → TForm1_CY_AddSedQueue(...)   // có payload
case 8:
  break;                                                                        // RỖNG
```

**Kết luận: OP 0x08 KHÔNG có phía gửi.** Chi tiết:
- `case 8:` **rỗng hoàn toàn** (dòng 896–897), không gọi `TForm1_CY_AddSedQueue`, và sau `switch` cũng **không có nhánh `default`** nào tự động gửi opcode.
- Nơi duy nhất gọi `FUN_0077f414(..., 8)` là `FUN_005fd72c` @ `0x005FD72C` (dòng 32 và 38) — hàm *prediction* "dùng item hồi HP/SP": nó tự set `player+0xfcc/0xfce/0xfcf/0xfd3` (bản sao thanh bar) rồi gọi `SendCommand(8)`; với builder hiện tại lệnh này **không phát sinh frame nào** (code thừa/đã bị loại ở phía server, hoặc opcode đã đổi sang OP khác ở case 0x17/0x27).
- Toàn bộ `attrCode` (0x19/0x1a/…) phía client chỉ được **tự ghi local** qua `FUN_00653ecc` ("AddStat": `old = FUN_007108f4(actor,code); FUN_00710ab0(actor,code,old±delta,0)`) từ `FUN_00652454` (vòng hiệu ứng kỹ năng) — **không có packet gửi đi**.

⇒ Mock server chỉ cần **một chiều phát** cho OP 0x08.

---

## 6. Ghi chú cho Mock Server

1. **Frame:** `44 F4 | Len:Word LE | payload`, toàn bộ XOR `0xAD`. Payload OP 0x08 luôn bắt đầu `08 | SubOp`.
2. **Gói tối thiểu đưa player vào game (thứ tự gợi ý):** các `SubOp 1` riêng lẻ cho từng `attrCode`:
   - `08 01 18 00 <class>00000000 00000000` — Class/Job (byte, `0x3e9`)
   - `08 01 3e 00 <level>00000000 00000000` — byte `0x43e` (lưu ý: `> 12` sẽ **đổi layout HUD** → chủ ý set sớm)
   - `08 01 25 00 <level>00000000 00000000` — word `0x400` (Level, dùng check "đủ cấp")
   - `08 01 1f 00 <hpx:4> 00000000` rồi `08 01 da 00 <basehp:4> 00000000` → client **tự tính MaxHP `0x404`**
   - `08 01 20 00 <spx:4> 00000000` rồi `08 01 db 00 <basesp:4> 00000000` → tự tính MaxSP `0x406`
   - `08 01 19 00 <hp>000000 00000000` / `08 01 1a 00 <sp>000000 00000000` → thanh HP/SP (0x1a còn refresh form stats `gvar_007D9EB8`)
   - `08 01 1b/1c/1d/1e …` → INT/ATK/DEF/AGI (bonus: `0xd4/0xd2/0xd3/0xd6`)
   - `08 01 24 00 <exp:4> 00000000` → EXP; `08 01 26 00 <pts>…` → điểm phân bổ
   Ví dụ: **HP hiện tại = 100** → payload 12B `08 01 19 00 64 00 00 00 00 00 00 00`.
3. **Độ dài payload phải khớp**: client `_BoundErr` khi chuỗi ngắn hơn index cần đọc → exception runtime. Luôn gửi đủ số byte ở mục 3 (12/15/16/8/14/5/5/4).
4. **Quy ước dấu:** đặt `deltaType = 0` khi gửi giá trị tuyệt đối; `deltaType = 2` nghĩa là "số này là âm" (client sẽ `value = -value`, Word → clamp về 0). Đừng gửi `deltaType=2` cho HP/SP nếu muốn giữ giá trị.
5. **Stat đã clamp:** Word (`0x19,0x1a,0x1b..0x20,0x25,0x26,0xcd,0xce,0xd7,0xd8,0x42`) kẹp `[0..0xFFFF]`; Byte (`0x18,0x23,0x3e,0x40`) kẹp `[0..0xFF]`; DWORD (`0x24,0x3f,0xcf,0xd0,0xd2..0xd6,0xda,0xdb,0x440`) không clamp.
6. **Gate cần biết:** `SubOp 0x02` **bắt buộc** `payload[2] == 0x04`, slot ≤ 4; `SubOp 0x04` bị **nuốt** nếu `FUN_00634f3c(table, player+0x63a, 3) == 0` và slot phải trỏ vào `gvar_007D9FB4` đang khác 0; `SubOp 0x03` yêu cầu charID **đã có trong cache 2100 slot** (do OP 0x03/0x14 đổ vào) — nếu không, gói bị im lặng bỏ qua.
7. **Không cần** mock chiều ngược lại cho OP 0x08; nếu muốn client "xin làm mới", không có cơ chế — server tự push khi chỉ số đổi.
8. **Vẽ bar (bỏ qua chi tiết):** HP bar = `(0x3ea / 0x404)`, SP = `(0x3ec / 0x406)`, EXP = `(0x3fc / bảng theo level)`; chỉ cần ghi 1 dòng — client tự repaint qua `FUN_0059d780` / `FUN_005e64d8` / `FUN_00528bd0` (`icon_HP1/icon_SP1/icon_EXP1`).

---

## 7. Kết luận kiểm chứng giả định "Stats"

- **ĐÚNG một phần trọng tâm:** sub-op `0x01/0x02/0x04` đích thực là **hệ thống chỉ số** (HP, SP, MaxHP/MaxSP có công thức Recompute, INT/ATK/DEF/AGI, Level, EXP, điểm thuộc tính, ClassID) với **bảng attrCode dùng chung** `FUN_00710ab0`; sub-op `0x03` là bản sao chỉ số cho cache actor.
- **KHÔNG ĐÚNG nếu cho rằng cả OP là Stats:** `0x05/0x06/0x07` là **nhóm counter/point tài nguyên** ở khối `player+0x4fc..0x539` (có timestamp, bị trừ khi giao dịch) và `0x08` là **notice dạng số** → OP 0x08 = *"Nhân vật: chỉ số + bộ đếm + thông báo"*.
- Tên `EXP` (`0x3fc`) và `Level` (`0x400`) là **suy luận loại mạnh** (từ bảng EXP-theo-level `FUN_0065b0dc`, check "đủ cấp dùng skill", panel `icon_LV`/`icon_HP_SP_EXP`), chưa có chuỗi label trực tiếp vì region `0x0074D1xx`/`0x00796Dxx` không có trong redump.

---

## 8. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `.scratch/op-code/handoff-opcode-exploration-guide.md` | dòng 16–24, 39–47, 112–141 | Framing, dispatcher, mapping `0x08 → Case 9 → 0x0078D125`, codec helpers |
| 2 | `.scratch/op-code/opcode_00_01.md` | dòng 23–55 | Mẫu định dạng + quy ước đọc PacketBuffer (tick pump, `ECX`/`DL`) |
| 3 | `ts_decompile/case_functions/functions/case_009_0078D125_FUN_0078d125.c` | 35–42 (SubOp), 43–78 (1), 79–126 (2), 127–171 (3), 172–192 (4–8) | Switch, wire layout, thứ tự `_LStrCopy` |
| 4 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 9:` inline ~ dòng 1990–2160 | Xác minh chéo byte-offset của sub-op 1/2/3/4..8 |
| 5 | `ts_decompile/functions/00710ab0_FUN_00710ab0.c` | 45–664 | **Stat setter**: attrCode → offset actor |
| 6 | `ts_decompile/functions/007108f4_FUN_007108f4.c` | 44–109 | Getter đối xứng → chốt kích thước/boundary từng offset |
| 7 | `ts_decompile/functions/006313d0_FUN_006313d0.c` | 84–168 | Chuỗi `"HP"`, `"SP"` → `0x3ea/0x404`, `0x3ec/0x406`; công thức cộng bonus |
| 8 | `ts_decompile/functions/005e64d8_FUN_005e64d8.c` | 88–245 | `icon_INT/ATK/DEF/AGI` ↔ code `0x1b..0x1e`; `icon_HPX/SPX` ↔ `0x1f/0x20` |
| 9 | `ts_decompile/functions/0065a300 / 0065aabc / 0065b6ac / 0065b6e8` (.c) | toàn bộ | Công thức MaxHP/MaxSP, vai trò `0x3f6/0x3f8/0x41c/0x420/0x424/0x428/0x42c/0x42d/0x408/0x40a/0x43c` |
| 10 | `ts_decompile/functions/00731c0c_FUN_00731c0c.c` | 26–101 | Mirror struct cache actor (sub-op 3) |
| 11 | `ts_decompile/functions/0073b458_FUN_0073b458.c` | 52–84 | Sub-op 4: gate `FUN_00634f3c`, mảng `gvar_007D9FB4` |
| 12 | `ts_decompile/functions/00634f3c_FUN_00634f3c.c` | 76–115 | Bảng definition stride `0xab`, field `+0x0e` ↔ flag 3 |
| 13 | `ts_decompile/functions/0074c340 / 0074c4c8 / 0074cc84 / 0074cfd4` | như đã dẫn | Sub-op 5,6,7,8 |
| 14 | `ts_decompile/functions/005ab678 + 005b3f94` (.c) | `icon_sk` | UI refresh của nhóm counter |
| 15 | `ts_decompile/functions/0074b7c8_FUN_0074b7c8.c` | 241–300 | Quan hệ `0x500→0x502`, `0x506→0x508`, `0x50c→0x50e`, `0x512=0xFA` (reset đầy) |
| 16 | `ts_decompile/functions/00527674_FUN_00527674.c` | 191–278 | Trừ chi phí vào `0x502/0x508/0x50e`, timestamp `Now()` vào record `0x509+k*12` |
| 17 | `ts_decompile/functions/00722508_FUN_00722508.c` | 89–115 | Cache 2100 slot `gvar_007DA6BC`, so `+4 == charID` |
| 18 | `ts_decompile/functions/00593428 + 00593b54` (.c) | 99–154 | Panel chỉ số: thứ tự field ↔ offset (`0x3ea,0x3ec,0x3fc,0x402,0x3ee..0x3f8,0x43e,0x3e9,0x40c..0x418,0x424,0x428,0x42e[]`) |
| 19 | `ts_decompile/functions/0056d56c_FUN_0056d56c.c` | 100–105 | Chuỗi `icon_LV`, `icon_HP_SP_EXP`, `icon_exp1` → tồn tại panel "LV / HP SP EXP" |
| 20 | `ts_decompile/functions/00528bd0_FUN_00528bd0.c` | 205, 276, 314 | `icon_HP1`, `icon_SP1`, `icon_EXP1` (bar — chỉ ghi chú 1 dòng) |
| 21 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | 768 (switch), 884–897 (`case 7` có payload, `case 8` rỗng), 1076–1080 (không default) | Kết luận **OP 0x08 không có chiều C→S** |
| 22 | `ts_decompile/functions/005fd72c_FUN_005fd72c.c` | 25–39 | Call-site duy nhất `SendCommand(8)` → no-op builder |
| 23 | `ts_decompile/functions/00653ecc_FUN_00653ecc.c` | 29–44 | Client-side "AddStat" (prediction), không gửi gói |
| 24 | `ts_decompile/functions/00596a7c_FUN_00596a7c.c` | 32–70 | Hệ quả `byte[player+0x43e] > 12` → đổi `main_input_1/3` |
| 25 | `ts_decompile/functions/0065b0dc_FUN_0065b0dc.c` | 42–80 | Bảng EXP-theo-level (cơ sở suy luận `0x3fc`) |
| 26 | `ts_decompile/functions/0077eb9c / 0077ef7c / 0077eb1c / 0077ee84` | header | Codec Word/DWORD LE |
| 27 | `docs/adr/0001-client-prediction-not-authority.md` | — | Framing: mọi giá trị client tính chỉ là prediction |
