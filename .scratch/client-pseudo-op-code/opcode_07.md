# PHÂN TÍCH — Main OP 0x07 (Case 8) `FUN_0078ce37` @ `0x0078CE37` — **Teleport / Đồng bộ vị trí + Map (Server ↔ Client)**

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code`
Trạng thái: **Đã xác minh từ decompile case function + dispatcher inline + handler vị trí**. Điểm quan trọng: **OP 0x07 KHÔNG có SubOp** — chỉ có 1 layout duy nhất. Mọi tài liệu ghi "SubOp của 0x07" đều là nhầm với OP 0x08.

---

## 1. Tóm tắt nghiệp vụ

OP 0x07 là gói **đồng bộ vị trí tuyệt đối + MapID**. Server dùng để:

- Teleport nhân vật chính sang map / tọa độ mới (đăng nhập, chuyển map, kéo về).
- Broadcast vị trí của 1 actor khác trong tầm nhìn cho client vẽ.
- Client cũng gửi ngược OP 0x07 lên server để báo vị trí / xin chuyển map (1 trong ít OP có 2 chiều).

| Hướng | Nội dung | Độ dài |
|---|---|---|
| S→C | `CharID:DWORD + MapID:Word + X:Word + Y:Word` | **11 bytes** (`07` + 10 rest) |
| C→S | `W1:Word + W2:Word + W3:Word` (vị trí / map hiện tại) | **7 bytes** (`07` + 6) |

---

## 2. Entry & cách đọc PacketBuffer

### 2.1. Đường vào (dispatcher chính)

1. `ClientSocket1Read` → XOR `0xAD` (`FUN_0050a248`) → deframe `[44 F4][Len:Word LE][Payload]` → `TForm1.CY_AddRevQueue`.
2. `CY_DelRevQueue` (tick ~30ms) tách `MainOp = payload[0]`, `RestPayload = Copy(payload,2,Len-1)` → `FUN_0078a89c(EAX=TFConnect, DL=MainOp, ECX=RestPayload)`.
3. Dispatcher `FUN_0078a89c` (`ts_decompile/functions/0078a89c_FUN_0078a89c.c`):
   - `switch(local_9)` tại dòng 579, với `local_9 = MainOp`.
   - `case 7:` dòng **1889–1985** → chính là `FUN_0078ce37` (byte-for-byte giống file case riêng).
   - Tra bảng: `byte_table[0x78A8EE][0x07] = 0x08` → `jumptable[0x78A9B6][8] = 0x0078CE37` → Case index 8, entry `0x0078A9D6`.

> Lưu ý dễ nhầm: tên `case_008` là **index jump-table** (đếm từ 0), không phải MainOp. `case 8` trong dispatcher (dòng 1986+) mới là MainOp `0x08` (có SubOp 1..8). OP `0x07` = dispatcher `case 7`.

### 2.2. Quy ước index (tránh off-by-one)

- `payload[k]`: byte thứ `k` của payload gốc, 0-based, `payload[0] = 0x07`.
- `ECX[r]` (0-based C) = `payload[r+1]`.
- `_LStrCopy(ECX, p, n, &tmp)` = Delphi `Copy(S,p,n)` 1-based → lấy `ECX[p-1 .. p+n-2]` = `payload[p .. p+n-1]`.
- Codec: `FUN_0077eb9c` = 2 byte → Word LE; `FUN_0077ef7c` = 4 byte → DWORD LE.

### 2.3. Cách đọc trong handler (file `case_008_0078CE37_FUN_0078ce37.c` dòng 24–35)

```c
_LStrCopy(ECX,1,4,&tmp); CharID = FUN_0077ef7c(tmp); // ECX[0..3] = payload[1..4]
_LStrCopy(ECX,5,2,&tmp); A = FUN_0077eb9c(tmp);      // ECX[4..5] = payload[5..6] (MapID)
_LStrCopy(ECX,7,2,&tmp); X = FUN_0077eb9c(tmp);      // ECX[6..7] = payload[7..8]
_LStrCopy(ECX,9,2,&tmp); Y = FUN_0077eb9c(tmp);      // ECX[8..9] = payload[9..10]
```

→ **Không có dòng nào đọc SubOp, không có `switch`.** Rest phải đủ 10 bytes, nếu thiếu sẽ `_BoundErr`.

---

## 3. Bảng tổng hợp (OP 0x07 không có SubOp)

| MainOp | SubOp | Độ dài payload | Nội dung | Handler |
|:---:|:---:|:---:|---|---|
| `0x07` | — (không có) | **11** (`07` + 10) | `CharID:DWORD LE \| MapID:Word LE \| X:Word LE \| Y:Word LE` | nhánh self: `FUN_00778510`, `FUN_0071e2b8`, `FUN_0071fae0` / nhánh other: `+ func_00722578`, `FUN_0070c20c`, `FUN_00722950`, `FUN_0070d86c`, `FUN_0071e080`, `FUN_004c9bf0` |

---

## 4. Chi tiết wire layout S→C

```
[0]=0x07 | [1..4]=CharID:DWORD LE | [5..6]=MapID:Word LE | [7..8]=X:Word LE | [9..10]=Y:Word LE
```

Rest (ECX, Delphi 1-based): `pos 1-4 = CharID`, `pos 5-6 = MapID`, `pos 7-8 = X`, `pos 9-10 = Y`.

### 4.1. Nhánh SELF — `CharID == *(player+4)` (dòng 36–47)

Điều kiện: `CharID` trùng CharID của object local `*(gvar_007DA7BC)` (offset `+4` là CharID).

1. Xóa trạng thái di chuyển cũ: `*(player+0x100)=0; *(player+0x104)=0;`
2. Ghi MapID: `*(short*)(player+0x63a) = MapID`. Bằng chứng đây là MapID: `FUN_0070d86c` đọc `*(player+0x63a)/1000`, `%1000` để phân biệt map đặc biệt (`0x3204, 0x3216, 0x348c, 0x3612...`).
3. `FUN_00778510(gvar_007D9C28, X, Y)` — cập nhật viewport/camera (`X-camX → +0xc`, `Y-camY → +0x10`, có clamp biên). Chỉ là offset nhìn, không phải vẽ.
4. `FUN_0071e2b8(player, X, Y)` — ghi vị trí chuẩn: `+0x1c=X, +0x20=Y, +0x4c=X, +0x50=Y, +0x54=X-viewX, +0x58=Y-viewY, +0xe3=0x0c, +0xe5=1`.
5. `FUN_0071fae0(player, X, Y)` — đồng bộ 5 entity con + pet: fill mảng `+0x164/0x165` (5 cặp X/Y + random), copy xuống `+0x15f[i]`, reset `+0x100/+0x104`. (Graphics/animation: chỉ 1 dòng — bám sprite con theo vị trí mới.)

→ Bản chất: **teleport local player**: đổi map + nhảy tọa độ + reset camera.

### 4.2. Nhánh OTHER — `CharID != local` (dòng 48–121)

1. `func_00722578(gvar_007D9D34, CharID)` — tra / đăng ký CharID trong manager.
2. `slot = FUN_0070c20c(gvar_007D9D34, CharID)` — CharID → slot `1..800` trong bảng `TPlayers[801]` (`gvar_007DA300`, so `+4 == CharID`). `slot==0` → bỏ qua.
3. `FUN_00722950(gvar_007D9C48, CharID, slotObj)` — "hồi sinh" object từ cache `gvar_007DA6BC[idx]`: copy `+0x04=CharID, +0x08=class, +0x3e9/+0x3fa/+0x448/+0x455/+0x7a/+0x7b/+0x7c, +0x9b[11 DWORD] tên/stats, 6 món đồ +0x2c[i], +0x462/+0x464/+0x465, +0x09[17] tên, +0x4b0/+0x4b1, +0x42c`.
4. Xóa move-state như nhánh self, rồi `FUN_0071e2b8(slotObj, X, Y)` ghi X/Y.
5. `FUN_0070d86c(slotObj)` — suy flag môi trường `+0x378` (`0/1/2/3`) + float `+0x340/+0x344` từ MapID. (Hiệu ứng map: 1 dòng.)
6. `FUN_0071e080(slotObj, +0x4b0, +0x4b1)` — xử lý cart/pet: nếu `0x4b0-1<2` thì free/create `TCartNpc` tại `+0x61c` theo `+0x79`.
7. `FUN_0071fae0(slotObj, X, Y)` như nhánh self.
8. `FUN_004c9bf0(*(slotObj+4))` — nếu trả về `5..8` (job/class siege) thì reset `*(slotObj+8)=0; *(slotObj+0x7c)=0x78`.

→ Bản chất: **spawn / di chuyển actor khác**: tra slot → nạp hồ sơ từ cache → đặt tọa độ → xử lý pet/cart.

---

## 5. Chiều Client → Server (C→S) của OP 0x07

Đối chiếu `ts_decompile/functions/0077f414_FUN_0077F414.c` (`TFConnect.SendCommand`, `switch(param_2 & 0xff)`), `case 7:` dòng 884–895:

```c
_LStrFromChar(&t, param_2);                       // byte0 = 0x07
FUN_0077eb1c(ctx, *(gvar_007DA5B0+0x138), &w1);    // Word W1
FUN_0077eb1c(ctx, *(gvar_007DA5B0+0x130), &w2);    // Word W2
FUN_0077eb1c(ctx, *(gvar_007DA5B0+0x134), &w3);    // Word W3
_LStrCatN(&out,4, ...); TForm1_CY_AddSedQueue(...);
```

- Payload C→S: `07 | W | W | W` = **7 bytes**. Không có CharID (server tự biết session).
- 3 Word lấy từ struct `007DA5B0+0x130/0x134/0x138` — vị trí / map hiện tại của client. (Thứ tự arg `_LStrCatN` trong decompile bị đảo do quy ước stack; khi mock chỉ cần gửi `07 + W1 + W2 + W3`.)
- Đây là gói **báo di chuyển / xin chuyển map**; server sẽ broadcast lại dạng S→C mục 4 cho mọi client trong tầm nhìn.

---

## 6. Ghi chú cho Mock Server

1. **Frame:** `44 F4 | Len:Word LE | payload`, toàn bộ XOR `0xAD` từng byte. Payload OP 07 S→C luôn bắt đầu `07`, dài 11.
2. **Gói teleport local player:** `07 + CharID(local) + MapID + X + Y`. Client sẽ đổi `player+0x63a`, nhảy camera, reset move-state.
3. **Gói spawn actor khác:** cùng layout nhưng `CharID` khác local; yêu cầu CharID đã có trong cache (do OP 0x03/0x14 đổ vào) nếu không `FUN_0070c20c` trả 0 → bị bỏ qua im lặng.
4. **Độ dài phải khớp:** rest thiếu 10 bytes → `_BoundErr` (exception runtime).
5. Ví dụ: `CharID=1000 (0x000003E8), Map=100 (0x0064), X=200 (0x00C8), Y=300 (0x012C)`:
   ```
   payload plain: 07 E8 03 00 00 64 00 C8 00 2C 01
   frame plain  : 44 F4 0B 00 07 E8 03 00 00 64 00 C8 00 2C 01
   frame wire (XOR AD): E9 59 A6 AD AA 45 AE AD AD C9 AD 65 AD 81 AC
   ```

---

## 7. Chuỗi hiển thị / mã hóa tiếng Việt (VISCII hay không?)

**Kết luận: OP 0x07 KHÔNG chứa bất kỳ nội dung text / VISCII / cp1258 nào — cả trong payload lẫn trong code xử lý.**

- Payload S→C và C→S đều **thuần số** (`CharID DWORD + MapID/X/Y Word`): không có trường chuỗi, không có byte text tiếng Việt trên wire.
- File case (`case_008_0078CE37_FUN_0078ce37.c` toàn file 154 dòng) **không tham chiếu hằng chuỗi nào** (`DAT_*`, `UNK_*`, `lit_*` đều vắng mặt — grep toàn `ts_decompile/` không trúng).
- Các handler (`FUN_00778510`, `FUN_0071e2b8`, `FUN_0071fae0`, `FUN_0070c20c`, `FUN_00722950`, `FUN_0070d86c`, `FUN_0071e080`, `FUN_004c9bf0`) đều là logic vị trí / slot / cache — không gọi toast `ShowMessage (+0x90)`, không gọi `SetText (FUN_007b372c)`.
- Ghi chú mã hóa: tiền lệ đã xác minh ở `opcode_02.md` mục 5 cho thấy game dùng **cp1258 (Windows Vietnamese), KHÔNG phải VISCII** (ví dụ `lit_7ABD54.hex`: `(C\xf4ng b\xaf...)` → cp1258 → "(Công bố hệ thống)"). Nên ngay cả khi sau này phát hiện text liên quan map (tên map), hướng giải mã đúng là **cp1258 → NFC**, không phải VISCII → UTF-8.

→ Không có gì để dịch / cập nhật thêm cho OP 0x07. Mock server không cần xử lý font hay chuỗi.

---

## 8. Source trail (file / địa chỉ đã dùng)

| # | Nguồn | Địa chỉ / dòng | Dùng để |
|---|---|---|---|
| 1 | `.scratch/op-code/handoff-opcode-exploration-guide.md` | dòng 16–24, 39–47, 112–141 | Framing, dispatcher, mapping `0x07 → Case 8 → 0x0078CE37`, codec helpers |
| 2 | `ts_decompile/case_functions/functions/case_008_0078CE37_FUN_0078ce37.c` | toàn file; parse 24–35; self 36–47; other 48–121 | Wire layout, 2 nhánh self/other |
| 3 | `ts_decompile/functions/0078a89c_FUN_0078a89c.c` | `switch` 579; `case 7` 1889–1985; `case 8` 1986–2128 | Xác minh inline khớp 100% + phân biệt OP 07/08 |
| 4 | `ts_decompile/functions/0077f414_FUN_0077F414.c` | `switch` 768; `case 7` 884–895 | Chiều C→S 3 Word |
| 5 | `ts_decompile/functions/00778510_FUN_00778510.c` | 19–21 | Viewport/camera |
| 6 | `ts_decompile/functions/0071e2b8_FUN_0071e2b8.c` | 26–44 | Ghi vị trí chuẩn |
| 7 | `ts_decompile/functions/0071fae0_FUN_0071fae0.c` | 40–155 | Đồng bộ entity con |
| 8 | `ts_decompile/functions/0070c20c_FUN_0070c20c.c` | 128–149 | CharID → slot bảng TPlayers |
| 9 | `ts_decompile/functions/00722950_FUN_00722950.c` | 41–194 | Nạp hồ sơ từ cache |
| 10 | `ts_decompile/functions/0070d86c_FUN_0070d86c.c` | 29–75 | MapID tại +0x63a, flag môi trường |
| 11 | `ts_decompile/functions/0071e080_FUN_0071e080.c` | 36–82 | Cart/pet |
| 12 | `ts_decompile/functions/0077eb9c / 0077ef7c / 0077eb1c` | header | Codec Word/DWORD LE |
| 13 | `docs/adr/0001-client-prediction-not-authority.md` | — | Framing, client prediction |
