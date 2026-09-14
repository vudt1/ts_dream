# PHÂN TÍCH — Main OP 0x2B (43) / Case 39 / `FUN_00794799` @ `0x00794799`

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only). 6 SubOp `0x01–0x06`, có `default` cleanup. 4/5 hàm con có body (riêng `0x05` vẫn thiếu).

> Cập nhật 2026-09-14: bổ sung phân tích từ các body/hex dump mới (theo `missing_opcode_sources.md`). **Kết quả cho OP 0x2B: không có thay đổi — callee `00758270` vẫn vắng mặt sau đợt redump** (xem §4.5).

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Kênh **quản lý party/guild** kiểu OP 0x24/0x29 — không phải kênh số cộng/trừ, không chat-log `FUN_007ab870`.
- Cùng manager cả 5 nhánh passthrough: `*gvar_007D9C20` (`+0xb3` current-ID, `+0xd4` TList, `+0xe0/+0xb7` slot).
- 2 họ:
  - **Passthrough + decode trong hàm con (5 nhánh)**: `0x01–0x05` trao nguyên `RP` cho `FUN_0075771c/0075785c/00757b70/007573a8/func_0x00758270` (05 chưa body — **vẫn chưa sau redump 2026-09-14**, §4.5).
  - **Banner 2 tầng (1 nhánh)**: `0x06` + byte thứ 2 `01/02/03` → 3 banner `DAT_00798EB4/00798F14/00798F7C` 2000ms.
- Đặc biệt: `case 3` **rơi qua `default`** nếu `*(mgr+0xb3)==0 && *(player+0x15c)==7` sai → cleanup no-op, không đi tiếp `case 6`.
- Chiều C→S `case 0x2b: break;` rỗng → client không bao giờ gửi OP này.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x2B (43) → byte_table[0x78A8EE][0x2B] = 0x27 (39)
                 → dword_table[0x78A9B6][39] @ 0x0078AA52 = 0x00794799
                 → FUN_00794799 (Case 39)
```

- File chính: `ts_decompile/case_functions/functions/case_039_00794799_FUN_00794799.c` (113 dòng)
- Bản inline: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6284-6347` — khớp 1:1.
- Framing/XOR/pump như `opcode_00_01.md` §2.

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x2B`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based. `mgr = *gvar_007D9C20`, `player = *gvar_007DA1DC`.

### 2.3. Đọc SubOp (dòng 22-29)

```c
if (*(RP-4)==0) _BoundErr(0);   // L=1 → RangeError
SubOp = (uint)*(byte*)(RP + 0); // RP[0] = P[1]
switch(SubOp){ case 1: ... case 6: ... default: cleanup; }
```

Không gọi codec trực tiếp ở dispatcher; codec gọi gián tiếp trong hàm con qua `FUN_0077ed68` (bản sao `FUN_0077ef7c`).

### 2.4. Codec dùng trong OP

| Helper | Vai trò ở OP này |
| :-- | :-- |
| `FUN_0077ed68` | 4B → DWORD LE: trực tiếp ở 01 (`Copy(RP,2,4)`), 02 (`Copy(RP,3,4)` + `Copy(RP,7,4)`), 04 (2×/record) |
| `FUN_0077ef7c` | 4B → DWORD LE: 1 chỗ ở 04 |
| `FUN_0077eb9c` / `FUN_0077f098` | Không gọi trong đường 0x2B |
| `FUN_0077eb1c` / `FUN_0077ee84` | Builder C→S, không gọi |
| Banner `(VMT+0x90)(...,ms,0,0)` | 06 (2000ms), 03 (5000ms), 02 (6000ms) |

---

## 3. Bảng tổng hợp SubOp (6 nhánh)

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[2B][01][id:4B LE]` (≥6B) | `Copy(RP,2,4)`+`ED68` = `P[2..5]` | `*(mgr+0xe0)=id` (+ đồ họa bỏ qua) |
| `0x02` | `[2B][02][sel:1B][id1:4B][id2?:4B]` (≥7B, sel==1 ≥11B) | `sel=RP[1]`; `id1=P[3..6]`; sel==1 thêm `id2=P[7..10]` → `*(mgr+0xb3)=id2` | sel 1–5 chọn hậu tố + banner 6000ms; sel==1 + `id1!=*(mgr+0xb7)` → patch list + sort |
| `0x03` | `[2B][03][flag:1B]` (3B) | `flag=RP[1]` | flag 00/01/02/FF → literal + banner 5000ms + refresh; nếu `mgr+0xb3==0 && player+0x15c==7` → `FUN_005952f4(player,2)` (mở editorBG), sai → rơi default |
| `0x04` | `[2B][04][records...]` (≥2B, record biến dài) | loop `pos=2`: `a/b/c` DWORD + `len1/s1/len2/s2/f` | Mỗi record → `FUN_00757264(mgr,a,b,f,s2,s1,c)`; xong sort + refresh. Rỗng vẫn sort; cụt → BoundErr |
| `0x05` | `[2B][05][blob...]` (≥2B) | passthrough | `func_0x00758270(mgr,RP)` — chưa body (xác nhận vẫn thiếu sau redump 2026-09-14) |
| `0x06` | `[2B][06][sub:1B]` (3B) | `sub=RP[1]` | `01→798EB4`, `02→798F14`, `03→798F7C` banner 2000ms; còn lại im lặng |
| `0x00`,`≥0x07` | — | — | no-op (default cleanup) |

---

## 4. Chi tiết từng SubOp (bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — Set `mgr+0xe0`

- `id = ED68(P[2..5])` → `*(mgr+0xe0)=id`. Tối thiểu 6B. Phần còn lại (`0063c1d0/banner 10000ms`) là đồ họa — bỏ qua.

### 4.2. SubOp `0x02` — Sel + 1/2 ID

- `sel=P[2]` (guard len≥2), `id1=P[3..6]`; `sel==1` đọc thêm `id2=P[7..10]` → `*(mgr+0xb3)=id2`, nếu `id1!=*(mgr+0xb7)` thì patch + `TList_Sort(mgr+0xd4)`. sel 2–5 chỉ chọn hậu tố (`00757AD0...`) + banner 6000ms.
- Wire: sel 2–5 cần 7B; sel 1 cần 11B.

### 4.3. SubOp `0x03` — Flag + mở editor có điều kiện

- `flag=P[2]`: 0 → đếm/scan list + clear `mgr+0xb3`, literal `0x757cfc`; 1/2/FF → `0x757d18/7d34/7d80`; luôn banner 5000ms + refresh `FUN_00571608` nếu cần.
- Sau đó: nếu `mgr+0xb3==0 && player+0x15c==7` → `FUN_005952f4(player,2)` (editorBG); sai → rơi qua default cleanup.
- Wire đúng 3B; `[2B][03]` đơn độc → `BoundErr(1)`.

### 4.4. SubOp `0x04` — Batch records

- Loop Delphi `pos=2` (1-based): mỗi iter `a=ED68`, `b=ED68`, `c=EF7C`, `len1/s1/len2/s2/f` → `FUN_00757264(mgr,a,b,f,s2,s1,c)`. Xong sort + refresh. `[2B][04]` rỗng vẫn sort; record cụt → BoundErr.

### 4.5. SubOp `0x05` — Opaque

- Chỉ `func_0x00758270(mgr,RP)`; grep toàn cây chỉ trúng dispatcher → chưa body. Mock tối thiểu 2B / replay blob live.
- **Đối chiếu lại sau đợt redump 2026-09-14** (`missing_opcode_sources.md` đã bổ sung ~245 body): vẫn **không có** file `ts_decompile/functions/00758270*` và `ts_decompile/index.csv` (6550 entry) **0 dòng** khớp `00758270`. Hai họ kề được export: `FUN_0075816c` (dừng tại `0x0075822C`) và `FUN_00758318` (`index.csv:5554-5555`) → `0x00758270` nằm gọn trong khe HOLE chưa decompile giữa hai hàm này. Gap giữ nguyên.

### 4.6. SubOp `0x06` — Banner 2 tầng

- `sub=P[2]` (guard len≥2; `[2B][06]` đơn độc → BoundErr, không phải no-op): 1/2/3 → 3 banner 2000ms; còn lại im lặng.

---

## 5. Chuỗi VISCII → UTF-8

- **Chưa decode được chuỗi nào**: `redump/` chỉ có `lit_595xxx/77F771/78A854/7A2xxx/7ABDxx` + jumptable/token — vắng toàn bộ literal OP này.
- Cần redump: `DAT_00798EB4/00798F14/00798F7C` (banner 06), `0x757AD0.../757B48` + `757AB8` (hậu tố 02), `0x757CFC/7D18/7D34/7D80` (banner 03), `0x796AB0` (tên debug), `editorBG` (`005957D8`). Redump tới null-terminator rồi map VISCII/cp1258 → UTF-8 NFC như OP 0x02.

---

## 6. Chiều Client → Server

- `ts_decompile/functions/0077f414_FUN_0077F414.c:980-981`: `case 0x2b: break;` — rỗng (kẹp giữa `0x2a` và `0x2c`).
- Kết luận: OP 0x2B S→C một chiều. Không format C→S để mock.

---

## 7. Ghi chú cho Mock Server

```
[2B][01][id u32]                  ≥6B → mgr+0xe0=id
[2B][02][01][id1][id2]            11B → mgr+0xb3=id2 + banner 6000ms
[2B][02][02..05][id1]             ≥7B → banner 6000ms
[2B][03][00|01|02|FF]             3B → banner 5000ms (+ editorBG nếu đủ điều kiện)
[2B][04][records...]              ≥2B → batch; rỗng vẫn sort
[2B][05][blob...]                 ≥2B → opaque, replay blob
[2B][06][01|02|03]                3B → banner 2000ms
ĐỪNG GỬI: [2B] (RangeError); [2B][06] cụt (BoundErr); [00],≥07 (no-op); [06][00/≥04] im lặng.
```

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_039_00794799_FUN_00794799.c` (113 dòng) | Handler chính 6 nhánh + default + fallthrough 03 |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6284-6347` + `:718-719` | Bản inline 1:1 + tên debug `0x796ab0` |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x2B]=0x27`) + `jumptable_dword200_0x78A9B6.hex` (`dw[39]`) + `.csv:41` + `manifest.csv:41` | Mapping tự parse |
| 4 | `functions/0075771c/0075785c/00757b70/007573a8` | Body 4/5 hàm con |
| 8 | `ls ts_decompile/functions/00758270*` (rỗng) + grep `00758270` `index.csv` (0/6550 dòng, kiểm 2026-09-14) + `index.csv:5554-5555` (kề `0075816c`/`00758318`) | Xác nhận gap SubOp 0x05 vẫn còn sau redump |
| 5 | `functions/0077ed68/0077ef7c/0077eb9c/0077eb1c/0077ee84/0077f098` | Codec |
| 6 | `functions/005952f4_FUN_005952f4.c:69-77` | Nhánh phụ editorBG |
| 7 | `functions/0077f414_FUN_0077F414.c:980-981` | C→S rỗng |
