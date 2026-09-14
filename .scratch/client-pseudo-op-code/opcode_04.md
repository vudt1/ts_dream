# PHÂN TÍCH — Main OP 0x04 (Case 5) `FUN_0078c7db` @ `0x0078C7DB` — aLogin.exe

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C)**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** — file case riêng `case_005_0078C7DB_FUN_0078c7db.c` **khớp 100%** với khối `case 4:` inline trong dispatcher `0078a89c_FUN_0078a89c.c` (dòng 1643–1733), và cả hai jump-table (byte `0x78A8EE`, dword `0x78A9B6`) + `manifest.csv` đều xác nhận định tuyến.

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`).

---

## 0. Đính chính giả định nghiệp vụ (quan trọng — đọc trước)

1. **KHÔNG tồn tại SubOp, và KHÔNG có nhánh `switch` nào trong OP 0x04.**
   - Yêu cầu "xác định `SubOp = ECX[0]` và toàn bộ nhánh switch" **không áp dụng** cho opcode này. Bằng chứng: `grep -c switch` trên file case = **0**; thân hàm chỉ có **hai phép so sánh `if` lồng nhau** (xem mục 2.3). Byte `ECX[0]` (= `payload[1]`) **không phải SubOp** — nó là **byte thấp (LSB) của trường `charID` 4-byte Little-Endian** (`_LStrCopy(ECX,1,4)` → `FUN_0077ef7c`).
   - Cấu trúc này **giống hệt** Main OP 0x03 (Case 4) ở phía *nhánh OTHER* — đã ghi nhận trong `opcode_03.md`. Cả OP 0x03 và OP 0x04 đều là **packet hồ sơ đơn, cấu trúc cố định + biến độ dài**, không có bảng SubOp.

2. **Bản chất nghiệp vụ: "Spawn / đồng bộ MỘT actor KHÁC đi vào cảnh" (Remote-Actor Enter/Refresh).**
   - Handoff cũ (`handoff-opcode-exploration-guide.md`) **không xếp OP 0x04 vào lộ trình ưu tiên** và không phỏng đoán nghiệp vụ cho nó → không có giả định cũ nào để bác bỏ; đây là kết luận mới từ decompile.
   - OP 0x04 **chỉ xử lý actor KHÁC** (không có nhánh SELF như OP 0x03): nó luôn parse hồ sơ vào cache 2100 slot rồi **chỉ materialize actor trên map khi `MapID` của hồ sơ khớp map hiện tại của client**.

3. **Độ tin cậy định danh `gself+0x63a`:** `opcode_03.md` gọi `gself+0x63a` là **MapID/khu vực (Word)**; `opcode_08.md` gọi cùng offset là **TemplateID**. Trong ngữ cảnh OP 0x04, `0x63a` được **so sánh trực tiếp với trường MapID của hồ sơ** (`p[10..11]`, mà `FUN_0072174c` lưu vào cache `rec+0x1a`) → **ngả về cách hiểu "MapID hiện tại"** (độ tin cậy **Cao** cho vai trò cổng lọc; nhãn "TemplateID" của OP 0x08 là cách hiểu khác cho cùng field, giữ nghi vấn mức **Trung bình**).

---

## 1. Tóm tắt nghiệp vụ

Main OP `0x04` là **gói "một nhân vật/NPC khác xuất hiện hoặc làm mới trong cảnh hiện thời"**. Server đẩy gói này khi một thực thể **khác** người chơi cục bộ bước vào tầm nhìn / cần dựng lại. Mỗi gói mang **toàn bộ hồ sơ ngoại hình + trạng thái + vị trí** của thực thể đó, đủ để client tạo actor mà không hỏi thêm.

Luồng xử lý (2 tầng):

| Tầng | Hàm | Việc |
|---|---|---|
| **A. Parse & cache hồ sơ** | `FUN_0072174c(gvar_007D9C48, RestPayload)` | Đọc toàn bộ payload, cấp slot trong **cache 2100 `TWorldPlayer`** (`gvar_007DA6BC`), ghi charID/MapID/cờ/2 mã ngoại hình/N cặp item trang bị/tên. Chạy **vô điều kiện**. |
| **B. Spawn actor thật** | thân `case_005` | **Cổng lọc map**: chỉ đi tiếp nếu `p[10..11] == gself+0x63a`. Sau đó cấp slot actor 1..800 (`func_0x0070c158`), tạo `TPlayers` nếu trống, tăng bộ đếm actor, **nạp cache→actor** (`FUN_00722950`), đặt tọa độ, bật/tắt companion, neo render, ép cờ theo loại thực thể. |

**Khác biệt so với OP 0x03 (nhánh OTHER):** OP 0x04 **KHÔNG** gọi refresh Party/Quân đoàn/Hảo hữu (`FUN_00760a88/00764844/00758318`), **KHÔNG** gọi blend/anim + camera (`FUN_0070d86c`, `FUN_00778510`, `func_0x00509e84`). → OP 0x04 là phiên bản **"gọn"** của enter-scene, thuần đồng bộ dữ liệu actor từ xa. *(Bổ sung từ body mới: phần bị bỏ qua `func_0x00509e84` là routine tái khởi tạo theo map — load tài nguyên `data\<MapID>`, clear 50 gate `scene+0x540c[1..50]`, tạo lại `TBKSimServer`, phát C→S `SendCommand(0x25, CL=1)`, reset ~9 panel UI — xem `opcode_03.md` §4.1 bước 10. Việc OP 0x04 không đụng tới nó là hợp lý: gói này chỉ là spawn增量 cho actor khác, không phải sự kiện vào-map của chính client.)*

**Không có nhánh nào đọc từ client:** OP 0x04 là **100% server-push** (mục 5 chứng minh `case 4` không tồn tại ở phía gửi).

---

## 2. Entry & cách đọc PacketBuffer

### 2.1. Đường vào (dispatcher)
1. `ClientSocket1Read` → XOR `0xAD` (`FUN_0050a248`) → deframe `[44 F4][Len:Word LE][Payload]` → `TForm1.CY_AddRevQueue`.
2. `CY_DelRevQueue` (tick ~30ms) tách `MainOp = payload[0]` và `RestPayload = Copy(payload,2,Len-1)` → `FUN_0078a89c(EAX=TFConnect, DL=MainOp, ECX=RestPayload)`.
3. Dispatcher: `byte_table[0x78A8EE][0x04] = 0x05` → `jumptable[0x78A9B6][5] = 0x0078C7DB` → `FUN_0078c7db`.
   - Xác minh: `jumptable_byte200_0x78A8EE.hex` dòng 1: `01 02 03 04 05 …` → index 4 = `0x05`. `jumptable_dword200_0x78A9B6.hex` entry[5] = `DB C7 78 00` = `0x0078C7DB`. `manifest.csv`: `5,0x0078A9CA,0x0078C7DB,…,FUN_0078c7db`.

### 2.2. Quy ước index (tránh off-by-one)
- `p[k]` = byte thứ `k` của **payload gốc** (0-based), `p[0] = 0x04`.
- `ECX[r]` (0-based) = `p[r+1]`.
- `_LStrCopy(ECX, start, n, &tmp)` = Delphi `Copy` **1-based** → lấy `p[start .. start+n-1]`.
- Codec: `FUN_0077eb9c` = 2 byte → **Word LE** (`b0 + b1*256`); `FUN_0077ef7c` = 4 byte → **DWORD LE**.
- `_BoundErr`/`_IntOver` = kiểm tra biên/số học runtime Delphi, **không phải nghiệp vụ**.

### 2.3. "Phân nhánh" thực sự của handler (dòng 28–119)
```c
FUN_0072174c(*(uint*)gvar_007D9C48, ECX);                 // (A) parse & cache — VÔ ĐIỀU KIỆN
_LStrCopy(ECX, 10, 2, &t);                                // t = p[10..11]
sVar2 = FUN_0077eb9c(t);                                  // Word LE = MapID của hồ sơ
if (sVar2 == *(short*)(*(int*)gvar_007DA7BC + 0x63a)) {   // CỔNG LỌC MAP: == map hiện tại?
    _LStrCopy(ECX, 1, 4, &t2); charID = FUN_0077ef7c(t2); // charID = p[1..4]
    idx = func_0x0070c158(gvar_007D9D34, charID);         // actor đã có? trả idx cũ : slot trống đầu : 0
    if (idx != 0) {                                       // (B) SPAWN
        if (gvar_007DA300[idx] == 0)
            gvar_007DA300[idx] = TPlayers_Create(VMT_70B5D0, 1, idx);
        *(gvar_007D9D34 + 0x5c) += 1;                     // actor count++
        FUN_00722950(gvar_007D9C48, charID, gvar_007DA300[idx]);  // cache → actor
        FUN_0072a054(gvar_007D9D34);                      // recompute top-index (+0x60)
        posX = FUN_0077eb9c(_LStrCopy(ECX,12,2));         // p[12..13]
        posY = FUN_0077eb9c(_LStrCopy(ECX,14,2));         // p[14..15]
        FUN_0071e2b8(actor, posX, posY);                  // đặt tọa độ
        FUN_0071e080(actor, actor+0x4b0, actor+0x4b1);    // companion TCartNpc
        FUN_0071fae0(actor, posX, posY);                  // neo render
        if ((byte)(FUN_004c9bf0(actor+4) - 5) < 4) {      // loại thực thể ∈ [5..8]
            actor+8 = 0; actor+0x7c = 0x78;               // ép cờ
        }
    }
}
```
→ **Chỉ có 2 điều kiện rẽ nhánh**: (1) `MapID hồ sơ == map hiện tại`, (2) `idx != 0`. **Không có `default`, không có SubOp.**

### 2.4. Epilogue (dòng 120–148) — KHÔNG phải chuỗi nghiệp vụ
Các `puStack00000008 = &LAB_00796408; uStack00000004 = 0x796364/0x79636f/…` là **bảng SEH/finally + danh sách `_LStrClr`/`_LStrArrayClr`** để giải phóng ~99 AnsiString cục bộ. **Không có hằng chuỗi mô tả opcode** → OP 0x04 **không in thông báo nào cho người dùng**; chuỗi duy nhất nó chứa là **tên nhân vật** (biến độ dài ở đuôi payload).

---

## 3. Bảng tổng hợp "SubOp" (thực chất = bảng phân nhánh)

| Điều kiện | Ý nghĩa | Xử lý |
|---|---|---|
| *(luôn chạy)* | Parse hồ sơ → cache 2100 slot | `FUN_0072174c` |
| `p[10..11] == gself+0x63a` **VÀ** `idx != 0` | Actor ở **đúng map hiện tại** & còn slot → **spawn** | `TPlayers_Create` + `FUN_00722950` + tọa độ |
| `p[10..11] != gself+0x63a` | Actor ở **map khác** → **chỉ cache, không spawn** | (bỏ qua tầng B) |
| `idx == 0` | Hết/không cấp được slot actor (vượt 800) | (bỏ qua tầng B) |

Không có SubOp thứ hai, không có `default`. Toàn bộ payload hiểu theo **một layout duy nhất** (mục 4).

---

## 4. Chi tiết wire layout + logic (một packet, hai tầng)

### 4.1. Wire layout tổng thể (chỉ số `p` payload gốc, 0-based, endian LE)

| Offset `p` | Size | Đọc bằng | Gán vào cache `TWorldPlayer` (`FUN_0072174c`) | Rồi sang actor (`FUN_00722950`) |
|:---|:---:|:---|:---|:---|
| `p[0]` | 1 | — | MainOp = `0x04` | — |
| `p[1..4]` | 4 | `Copy(1,4)`+`FUN_0077ef7c` | `rec+4` = **charID** | `actor+4` (charID) |
| `p[5]` | 1 | `ECX[4]` | `rec+0x1c` | `actor+0x08` (cờ trạng thái A) |
| `p[6]` | 1 | `ECX[5]` | `rec+0x1d` | `actor+0x3e9` (**Class/Job**, xem OP 0x08) |
| `p[7]` | 1 | `ECX[6]` | `rec+0x1e` | `actor+0x3fa` (cờ job-state) |
| `p[8]` | 1 | `ECX[7]` | `rec+0x36` | `actor+0x448` (cờ) |
| `p[9]` | 1 | `ECX[8]` | `rec+0x37` | `actor+0x455` (cờ) |
| `p[10..11]` | 2 | `Copy(10,2)`+`FUN_0077eb9c` | `rec+0x1a` = **MapID** | *(dùng cho CỔNG LỌC, không copy)* |
| `p[12..13]` | 2 | `Copy(12,2)`+`FUN_0077eb9c` | *(không cache)* | **posX** → `FUN_0071e2b8` → `actor+0x1c` |
| `p[14..15]` | 2 | `Copy(14,2)`+`FUN_0077eb9c` | *(không cache)* | **posY** → `FUN_0071e2b8` → `actor+0x20` |
| `p[16]` | 1 | `ECX[15]` | `rec+0x3c` | `actor+0x7a` (cờ) |
| `p[17]` | 1 | `ECX[16]` | `rec+0x3d` | `actor+0x7b` (cờ) |
| `p[18]` | 1 | `ECX[17]` | `rec+0x3e` (Word←byte) | `actor+0x7c` (Word) |
| `p[19..22]` | 4 | `Copy(19,4)`+`FUN_0077ef7c` | `FUN_00745bc0(rec+0x42, code1, mask=-1, mode=1)` | `actor+0x9b` (ngoại hình) |
| `p[23..26]` | 4 | `Copy(23,4)`+`FUN_0077ef7c` | `FUN_00745bc0(rec+0x42, code2, mask=0xFF, mode=5)` | `actor+0x9b` (ngoại hình) |
| `p[27]` | 1 | `ECX[26]` | `N` = số cặp item trang bị | — |
| `p[28 + 2*i]` | 2·N | `Copy(2i+28,2)`+`FUN_0077eb9c` | item code → `rec+0x70[slot]` (`slot=FUN_00774af8(gvar_007DA540,code)` 0..6) | `actor+0x2c[slot]` (+4 code, +0x10 slot, +0x14 sprite) |
| `p[2N+28..29]` | 2 | `Copy(2N+28,2)`+`FUN_0077eb9c` | `rec+0x38` | `actor+0x462` (Word) |
| `p[2N+30]` | 1 | `ECX[2N+29]` | `rec+0x3a` | `actor+0x464` (cờ) |
| `p[2N+31]` | 1 | `ECX[2N+30]` | `rec+0x8c` | *(chỉ cache)* |
| `p[2N+32]` | 1 | `ECX[2N+31]` | `rec+0x3b` | `actor+0x465` (cờ) |
| `p[2N+33]` | 1 | `ECX[2N+32]` | `rec+0x8d` | *(dựng tiền tố tên ở scene login)* |
| `p[2N+34]` | 1 | `ECX[2N+33]` | `rec+0x8e` | `actor+0x4b0` (cờ companion) |
| `p[2N+35]` | 1 | `ECX[2N+34]` | `rec+0x8f` | `actor+0x4b1` (cờ companion) |
| `p[2N+36 .. end]` | biến | `Copy(2N+36, len-(2N+35))` | **tên** → `rec+8` (`_PStrNCpy` cắt `0x11`=17 byte) | `actor+9` (tên, 17 byte) |

> **Độ dài payload tối thiểu:** `36 + 2·N + len(tên)` byte. Với `N=0` và tên ≥1 ký tự → **≥ 37 byte**. Gửi thiếu → `_BoundErr` (exception runtime Delphi).

### 4.2. Tầng A — `FUN_0072174c` (parse & cache) — logic cốt lõi
- `idx = FUN_00722464(gvar_007D9C48, charID)` — **find-or-allocate** trong cache 2100 slot `gvar_007DA6BC` (so `rec+4 == charID`; không thấy thì cấp slot trống; hết → trả 0 → **bỏ qua toàn bộ**).
- Nếu `idx != 0`: `*(gvar_007D9C48+4)++` (đếm cache), `TWorldPlayer_Create(VMT_70B6CC,1,idx)` nếu slot trống, rồi ghi các field theo bảng trên.
- **Ngoại hình:** 2 DWORD `p[19..22]`/`p[23..26]` là **mã nén** → `FUN_00745bc0` tách thành các chữ số 3-3-3 ghi vào mảng byte `rec+0x42` (0x2d byte). Gửi `0` = actor "trần".
- **Trang bị:** vòng `i=0..N-1` đọc N cặp Word; mỗi code **phải tồn tại** trong CSDL item `gvar_007DA540` để `FUN_00774af8` trả ô 0..6 hợp lệ; mảng 7 slot `rec+0x70[1..6]` được reset (`+4=0`, `+0x14=-1`) trước khi nạp.
- **Tên:** nếu `FUN_00504c9c(...)` **đúng** (đang ở **scene login**, `%100 ∈ [90..99]`) → tên = `IntToStr(rec+0x8d)` nối chuỗi đuôi; **ngược lại** → copy thẳng chuỗi đuôi. Cả hai cắt còn 17 byte.

### 4.3. Tầng B — spawn actor trong `case_005` — logic cốt lõi
1. **Cổng lọc map:** `FUN_0077eb9c(Copy(10,2)) == gself+0x63a`. **Sai → không spawn** (actor chỉ nằm trong cache, chờ khi client đổi sang map đó).
2. `idx = func_0x0070c158(gvar_007D9D34, charID)` — **find-or-allocate slot actor 1..800 — xác minh được từ body mới** (`0070c158_FUN_0070c158.c`): hàm bỏ qua param_1 (EAX-artifact, đọc thẳng global `gvar_007DA300`); **bước 1** quét `i=1..800` tìm actor đã có `actor+4 == charID` → trả chính idx đó (gửi lại OP 0x04 cho actor đang sống = **cập nhật tại chỗ**, không tốn slot); **bước 2** nếu không có ai khớp thì trả slot trống đầu tiên (`gvar_007DA300[i] == 0`); mảng 800 slot đầy → trả 0 (d.31–63). `idx==0` → bỏ qua.
3. `if (gvar_007DA300[idx]==0) TPlayers_Create(VMT_70B5D0_TPlayers, 1, idx)` — tạo object actor (bên trong `TPlayers.Create` dựng sẵn 5 `TFollowNpc` tại `actor+0x15f[1..5]`, set `actor+0x79=2` = kind "TPlayers").
4. `*(gvar_007D9D34 + 0x5c) += 1` — **tăng bộ đếm actor, tăng vô điều kiện khi `idx != 0`** (guard `slot == 0` chỉ che việc `TPlayers_Create`; nếu `0070c158` trả idx của actor đã tồn tại — actor respawn — bộ đếm vẫn +1 thêm lần nữa; giảm ở OP 0x01 despawn).
5. `FUN_00722950(gvar_007D9C48, charID, actor)` — **nạp cache→actor** (bảng cột cuối mục 4.1): charID, cờ, Class `+0x3e9`, ngoại hình `+0x9b`, 7 ô trang bị `+0x2c`, tên `+9`, cờ companion `+0x4b0/+0x4b1`, `actor+0x78=1` ("đã có hồ sơ").
6. `FUN_0072a054(gvar_007D9D34)` — quét ngược mảng 800 tìm chỉ số actor đỉnh, ghi `gvar_007D9D34+0x60` (bound cho vòng render & cho `FUN_0070c20c`). *(presentation, 1 dòng)*.
7. `FUN_0071e2b8(actor, posX, posY)` — **đặt tọa độ thế giới**: `actor+0x1c=posX`, `actor+0x20=posY`, `actor+0x4c/0x50` = bản sao, `actor+0x54/0x58` = `pos - camera`, `actor+0xe3=0xc`, `actor+0xe5=1`.
8. `FUN_0071e080(actor, actor+0x4b0, actor+0x4b1)` — ghi 2 cờ companion; nếu cờ ∈ {0,1,2} thì tạo/hủy object **`TCartNpc`** tại `actor+0x61c` (xe đẩy/thú kéo đồng hành). *(1 dòng)*.
9. `FUN_0071fae0(actor, posX, posY)` — tính lại **5 neo sprite** theo `posX/posY + offset ngẫu nhiên`. *(đồ họa, 1 dòng)*.
10. `FUN_004c9bf0(actor+4)` — **phân loại thực thể theo charID** (nhóm `[5..8]` ≈ NPC/monster theo dải ID, khớp quy ước `id∈[100..400]` = NPC ở `opcode_02.md`). Nếu thuộc `[5..8]` → **ép** `actor+0x08 = 0` và `actor+0x7c = 0x78` (120).

---

## 5. Chiều Client → Server (C→S) của OP 0x04

Đối chiếu `ts_decompile/functions/0077f414_FUN_0077F414.c` (`TFConnect.SendCommand`, `switch(param_2 & 0xff)` dòng 768–1079):

```c
case 3:  ... _PStrNCat(...,2) → TForm1_CY_AddSedQueue(...)   // có (legacy, xem opcode_03)
case 5:  break;                                              // RỖNG
...
```
- **KHÔNG có `case 4:`** trong switch gửi đi. Các case hiện diện: `0,1,2,3,5,6,7,8,9,10,0xb,…,0x48,199`. **Không có `default`** tự động gửi.
- **Kết luận: OP 0x04 KHÔNG có phía gửi.** Không tìm thấy bất kỳ call-site nào gọi `SendCommand(4)`. Đây là **broadcast một chiều server→client** (giống OP 0x03/OP 0x08). Mock server **chỉ cần phát**, không cần nhận.

---

## 6. Ghi chú cho Mock Server

1. **Frame:** `44 F4 | Len:Word LE | payload`, toàn khung XOR `0xAD`. Payload OP 0x04 bắt đầu bằng `0x04` rồi **vào thẳng charID** — **đừng chèn byte SubOp**.
2. **charID:** `p[1..4]` = DWORD LE. Đây cũng là khóa tra cache/actor.
3. **BẮT BUỘC `p[10..11]` = MapID hiện tại của người nhận** (Word LE, cùng hệ giá trị với `gself+0x63a`). Nếu khác → client **chỉ cache, không spawn** (gói "im lặng" mất tác dụng).
4. **Tọa độ:** `p[12..13]=posX`, `p[14..15]=posY` (Word LE). Hai trường này **chỉ dùng lúc spawn**, không vào cache.
5. **Slot actor:** `func_0x0070c158` trả 0 khi hết 800 slot → gói bị bỏ. Đảm bảo chưa vượt ngưỡng. Lưu ý (body mới): gửi OP 0x04 **lặp cho cùng một charID đã spawn** là hợp lệ — client tái sử dụng đúng actor cũ (cập nhật tại chỗ), không tạo bản thứ hai.
6. **Ngoại hình:** `p[19..22]`/`p[23..26]` = 2 DWORD mã nén; gửi `0` nếu không cần tạo hình.
7. **Trang bị:** `p[27]=N`; mỗi item code ở `p[28+2i]` **phải có trong CSDL item client** (`gvar_007DA540`) để `FUN_00774af8` trả ô 0..6 hợp lệ. **`N` và độ dài payload phải khớp chính xác** (sai → `_BoundErr` crash).
8. **Độ dài:** `≥ 36 + 2·N + len(tên)` byte. Tên cắt còn 17 byte; ở **scene login** (`%100∈[90..99]`) client sẽ **thay tên bằng `IntToStr(p[2N+33])`** — khi mock ở map thường thì không liên quan.
9. **Cờ loại:** nếu charID rơi vào nhóm `[5..8]` của `FUN_004c9bf0`, client **ghi đè** `actor+0x08=0`, `actor+0x7c=120` bất kể giá trị gửi.
10. **Thứ tự khung cảnh:** server gửi OP 0x01 SubOp 0x09 (chuyển scene) → OP 0x03 (SELF, dựng mình) → rồi **OP 0x04** cho từng actor/NPC xung quanh (hoặc OP 0x03 OTHER — cả hai cùng layout Bảng B; OP 0x04 nhẹ hơn, không refresh party/camera).
11. **Không cần mock chiều ngược lại** cho OP 0x04.

---

## 7. Source trail (file / địa chỉ đã dùng để xác minh)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `.scratch/op-code/handoff-opcode-exploration-guide.md` | 16–24, 43, 112–141 | Framing, dispatcher, mapping `0x04 → Case 5 → 0x0078C7DB`, codec helpers |
| 2 | `.scratch/op-code/opcode_00_01.md` | 23–55 | Mẫu định dạng + quy ước đọc PacketBuffer (tick pump, `ECX`/`DL`) |
| 3 | `.scratch/op-code/opcode_03.md` | Bảng B | **Layout trùng khớp** (OP 0x04 = OP 0x03 OTHER); cổng lọc map `gself+0x63a`; `func_0x0070c158` |
| 4 | `.scratch/op-code/opcode_08.md` | 33–54, 100–134 | Stat setter `FUN_00710ab0` (offset actor `+0x3e9` Class…), nghi vấn `+0x63a` |
| 5 | `.scratch/op-code/opcode_02.md` | 59–60, 124, 190 | `gself=gvar_007DA7BC` (name `+9`), cache `gvar_007DA6BC` (name `+8`, id `+4`), `FUN_00722508`, dải id NPC `[100..400]` |
| 6 | `ts_decompile/case_functions/functions/case_005_0078C7DB_FUN_0078c7db.c` | 28–119 (logic), 120–148 (epilogue) | **Handler chính**; xác nhận **không switch** |
| 7 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `case 4:` 1643–1733 | **Cross-check byte-for-byte** với case_005 |
| 8 | `ts_decompile/functions/0072174c_FUN_0072174c.c` | 177–829 | Tầng A: parse & cache `TWorldPlayer` (`FUN_00722464`, `FUN_00745bc0`, `FUN_00774af8/6ac`, tên + prefix login `FUN_00504c9c`) |
| 9 | `ts_decompile/functions/00722950_FUN_00722950.c` | 40–196 | **Nạp cache→actor** (bảng cột cuối mục 4.1) |
| 10 | `ts_decompile/functions/00722464_FUN_00722464.c` | 20–56 | find-or-allocate cache 2100 slot `gvar_007DA6BC` |
| 11 | `ts_decompile/functions/0070c158_FUN_0070c158.c` | 31–63 | find-or-allocate slot actor 1..800 (body mới — xác minh phỏng đoán cũ: ưu tiên actor đã có theo charID, rồi slot trống đầu, đầy → 0) |
| 11b | `ts_decompile/functions/0070c20c_FUN_0070c20c.c` | 121–150 | chỉ định vị (find-by-charID, bound `+0x60`) — đối chứng |
| 12 | `ts_decompile/functions/007169b4_TPlayers.Create.c` | 30–56 | Tạo actor `TPlayers` (kind `+0x79=2`, 5 `TFollowNpc` `+0x15f`) |
| 13 | `ts_decompile/functions/0072a054_FUN_0072a054.c` | 18–38 | Recompute top-index actor `gvar_007D9D34+0x60` |
| 14 | `ts_decompile/functions/0071e2b8_FUN_0071e2b8.c` | 18–47 | Đặt tọa độ `actor+0x1c/0x20/0x4c/0x50/0x54/0x58` |
| 15 | `ts_decompile/functions/0071e080_FUN_0071e080.c` | 20–83 | Cờ companion `+0x4b0/+0x4b1` → `TCartNpc` (`actor+0x61c`) |
| 16 | `ts_decompile/functions/0071fae0_FUN_0071fae0.c` | 23–168 | Neo render 5 sprite (đồ họa) |
| 17 | `ts_decompile/functions/004c9bf0_FUN_004c9bf0.c` | 65–131 | Phân loại thực thể theo ID (nhóm `[5..8]`) |
| 18 | `ts_decompile/functions/00745bc0_FUN_00745bc0.c` | signature 44–46 | Giải mã ngoại hình (mode 1/5, mask) |
| 19 | `ts_decompile/functions/00504c9c_FUN_00504c9c.c` | 51–60 | Predicate scene login `%100∈[90..99]` |
| 20 | `ts_decompile/functions/0077eb9c / 0077ef7c` (.c) | header | Codec Word/DWORD LE |
| 21 | `ts_decompile/redump/jumptable_byte200_0x78A8EE.hex` | dòng 1 (`[4]=0x05`) | Định tuyến OP 0x04 |
| 22 | `ts_decompile/redump/jumptable_dword200_0x78A9B6.hex` | entry[5]=`DB C7 78 00` | Target `0x0078C7DB` |
| 23 | `ts_decompile/case_functions/manifest.csv` | dòng `5,…,0x0078C7DB,…,FUN_0078c7db` | Xác nhận case↔hàm |
| 24 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | 768 (switch), 819–828 (`case 3` có, `case 5` rỗng, **không `case 4`**) | Kết luận **OP 0x04 không có chiều C→S** |
| 25 | `ts_decompile/functions/0051189c_FUN_0051189c.c` | 234+ (FormCreate) | Định danh global/VMT (form, cache, DB) |

### Độ tin cậy kết luận
- **Cao:** không có SubOp/switch; wire layout (mục 4.1); cổng lọc map `p[10..11]==gself+0x63a`; charID/posX/posY; `FUN_0072174c`+`FUN_00722950` là cặp parse-cache→nạp-actor; OP 0x04 không có chiều C→S; không có chuỗi thông báo (chỉ tên); **nội hàm `func_0x0070c158` — xác minh được từ body mới (`0070c158_FUN_0070c158.c:31-63`): find theo charID trước, rồi slot trống đầu tiên, đầy → 0** (không phải "chỉ cấp slot"; không tạo object bên trong).
- **Trung bình:** nhãn ngữ nghĩa từng cờ byte (`+0x08/0x448/0x455/0x7a/0x7b/0x462/0x464/0x465`) — xác định offset & hướng ghi nhưng chưa có label chuỗi trực tiếp.
- **Thấp / nghi vấn:** `gself+0x63a` = MapID (OP 0x03) vs TemplateID (OP 0x08) — cùng offset, hai cách gọi; trong OP 0x04 ngữ cảnh so khớp MapID nghiêng về **MapID**.

> **ADR-0001:** mọi giá trị actor/đếm/tọa độ client ghi ở đây là **client prediction/presentation** — server vẫn là nguồn đúng; OP 0x04 chỉ là lệnh "dựng/đồng bộ actor từ xa" do server chủ động phát.
