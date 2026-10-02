# 08 — BÁO CÁO XÁC MINH ĐỐI KHÁNG (VERIFIER) — 3 claim đảo ngược thiết kế server Rust

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

Ngày: 2026-10-02
Verifier: subagent độc lập, chỉ dùng bytes/asm thật
Nguồn chính: `client_pseudo_c/` (Ghidra pseudo-C + `.asm.txt` + `.hex`), đối chiếu `test/ts_decompile/functions/`, `TS_Server_Bear/TS_Server/`, `src/` (Rust hiện tại)

> Quy ước: `P[i]` = byte payload (`P[0]` = MainOp); `RP[i] = P[i+1]`; Delphi `_LStrCopy(s,idx,cnt)` **1-based**.
> Mức: **XÁC MINH** (đọc byte/asm thật) · **BÁC BỎ** (bytes ngược lại) · **CHƯA ĐỦ BẰNG CHỨNG**.
> Lưu ý phương pháp: **không** dùng `case N: break;` rỗng của Ghidra làm bằng chứng phủ nhận (đã kiểm tra builder C→S qua 2 bảng hex + asm thật).

---

## 0. Tóm tắt xác minh

| # | Claim | Verdict | Mức tin cậy |
| :-: | :-- | :-- | :-- |
| **1** | `0x1B` **không** mang danh sách hàng/giá; client tự parse `Data\Eve.emg` (stride 5B, itemId = **Word LE**) + `Data\Item.Dat` (price) | **XÁC MINH (phần lõi)** — nhưng **2 chi tiết sai**: (a) shop **mua** dùng `player+0x9E0[]` chứ không phải TSurface của eve; (b) `1B 01` count là **Word LE 2 byte**, index là **0-based entry index** | Cao (A) cho phần lõi; Cao (A) cho 2 sai lệch |
| **2** | `player+0x12FC` là tiền bank, set qua `0x1D` sub `0x04` | **XÁC MINH** — `FUN_0072be10.asm.txt:30` `MOV dword ptr [EDX + 0x12fc],EAX` là bằng chứng byte tuyệt đối. Tuy nhiên `opcode_1a.md` **KHÔNG sai**: câu nó viết là "gói này" (tức `1A 04`), đúng phạm vi | Rất cao (A) |
| **3** | `23 04` không set field tiền nào; Rust `store_frame()` spam banner mỗi lần login | **XÁC MINH** — `case_031…c:201` chỉ ghi `player+0x146C = A*100+B`; không đụng `0x12F8/0x12FC/0x1300` | Rất cao (A) |

**Kết luận đảo ngược thiết kế:** Claim 1 và 3 **đúng và có hàm ý kiến trúc lớn** (server không cần gửi catalogue shop NPC; `23 04` bị dùng sai ý nghĩa). Claim 2 **đúng về cơ chế nhưng sai về đánh giá tài liệu** — `opcode_1a.md` nói đúng phạm vi, và `opcode_1d.md` đã đặc tả `1D 04` từ 2026-09-14.

---

## 1. CLAIM 1 — NPC shop: danh sách hàng + giá không bao giờ do server gửi

### 1.a. Grep file `.dat`/`.emg` trong `client_pseudo_c/` — **KHÔNG có kết quả**, phải tìm ở `test/ts_decompile/`

Grep thẳng trong `client_pseudo_c/` (1136 file) trả về **0 dòng** cho `eve.emg` / `.emg` / `Item.dat` / `ItemDat` / `item.dat` / `BlissBag`. **Lý do:** `client_pseudo_c/` là tập con đã lọc (chỉ 209 hàm prefix `005*` + dispatcher + vài hàm protocol), **không** chứa các hàm loader dữ liệu (`005e8534`, `005efee8`, `005efc24`, `007746d4`, `005eb530`).

Đã tìm lại trong `test/ts_decompile/functions/` (6588 hàm):

| Tên hàm | Call-site (chỉ 1) | Bằng chứng file:line |
| :-- | :-- | :-- |
| `FUN_007746d4` — **loader `Data\Item.Dat`** | `TForm1.FormCreate @ 0x0050A4A0` | `test/…/0050a4a0_TForm1.FormCreate.asm.txt:674-675` → `MOV [0x009264bc],EAX` / `CALL 0x007746d4`; `.c:580` → `FUN_007746d4((int)DAT_009264bc)` |
| ↳ nội dung | — | `005746d4_FUN_007746d4.c:74,83` → `_LStrCat3(&local_168, local_c, "\\Data\\Item.Dat")`; `:76` `FileExists`; `:110` `if (*(int *)(local_8 + 0x58) != 2) raise`; `:125` `uVar5 = idx * 0xb9` (stride record = **0xB9 = 185**) |
| `FUN_005efc24` — **mở TFileStream dữ liệu map/eve theo mapId** | `FUN_005e83e0` | `005efc24_FUN_005efc24.c:72-83` → `_LStrLAsg(&local_20, 0x5efe80)` (root exe) + `"\\Data\\"` + mapId string; `:81-82` `piVar2 = TFileStream_Create(VMT_410A34_TFileStream, 1, local_1c, 0x20); *(int **)(local_8 + 0xa0f0) = piVar2;` |
| `FUN_005e8534` — **nạp map** (5160 B) | `FUN_005eb2ec` | `005e8534_FUN_005e8534.c:182` `puVar24 = (undefined4 *)(param_1 + 0x9d34);` · `:186` `FUN_005efee8(local_8, *(int **)(local_8 + 0xa0f0), local_30, …)` — truyền **TFileStream đã mở ở `+0xa0f0`** |
| `FUN_005efee8` — **parser nhị phân 9 section** (5632 B) | `FUN_005e8534` (`005e8534…c:186`) | `005efee8_FUN_005efee8.c:458-530` khối **Surface** (hộp thoại/shop) |

**Kết luận 1.a: XÁC MINH** — có hai loader file cục bộ thật, cả hai đều **client-side**, không liên quan frame mạng.

### 1.b. Form shop lấy danh sách + giá từ đâu?

Có **hai form shop khác nhau**, và claim gộp nhầm:

#### (i) Form MUA — `TInterForm` / `form_BuyQ_Npc` (`gvar_007DA5A8`) — **dùng TSurface của Eve.emg**

`FUN_005eb530` (paint/list build) đọc entry từ `**gvar_007DA5A0 + 0xa0d4` (con trỏ `TSurface`), offset `4 + j*5`:

```c
// 005eb530_FUN_005eb530.c:1090-1093
uVar7 = FUN_007746ac(*(undefined4 *)gvar_007DA540,
                     *(ushort *)(*(int *)(local_8 + 0xa0d4) + 4 + uVar9));
// 005eb530_FUN_005eb530.c:1137
IntToStr(*(undefined4 *)(*(int *)gvar_007DA540 + 0x58 + uVar12 * 2),(int *)&local_f0);
```

* `uVar9 = j * 5` (`:1085-1089`) ⇒ **stride 5 byte/mục** ✔ (khớp claim)
* `*(ushort *)(rec + 4 + j*5)` ⇒ **itemId = Word LE** ✔ (khớp claim)
* `*(int *)(gvar_007DA540 + 0x58 + idx*0xB9*2)` ⇒ **giá lấy từ bảng Item.Dat, offset `0x58` trong record 185-word** ✔ (khớp claim)

Nguồn `TSurface` được `FUN_005efee8` dựng khi parse eve:
```c
// 005efee8_FUN_005efee8.c:460-476
(**(code **)(*local_c + 4))(local_c,&local_1a,2);          // đọc 2B = surfaceId
uVar7 = TObject_Create(VMT_7A9E5C_TSurface, 1, …);
*(ushort *)(local_4c + 0x6e) = local_1a;                    // rec+0x6E = shopId
… (local_4c + 0x70), 0x6d, 0x72, 0x73, 0x74 …
// :487  (**(code **)(*local_c + 4))(local_c,&local_25,5);   // ĐỌC 5 BYTE / MỤC
// :505  *(undefined4 *)(local_4c + 4 + uVar7) = local_25;   // uVar7 = local_15 * 5
```
Stride 5, đọc bằng vtable `+4` (read) của TFileStream ⇒ **xác nhận mạnh: dữ liệu shop NPC nằm trong file cục bộ**.

#### (ii) Form BÁN — `TThingSaleForm` (`gvar_007DA720`, VMT `0x5D5438`) — **KHÔNG dùng eve**

Ctor (`005dc500_FUN_005dc500.c`):
```c
:116  (**(code **)(*local_8 + 8))(local_8,"Form_SaleQ_Item", …);   // tên form
:130  lVar1 = (longlong)((int)uVar2 / 5) * 0x24;                  // lưới 5×5
:161  *(int *)(local_8[uVar2 + 0x3a] + 0xb8) = local_10;          // img+0xB8 = 1..25
:204  (**(code **)(*local_20 + 8))(local_20,"btn_Sale", …);       // nút BÁN
```

Render (`FUN_005dd000`, 1904 B) — **nguồn dữ liệu là `player+0x9E0[]`, KHÔNG phải eve**:
```c
// 005dd000_FUN_005dd000.c:91
if (*(int *)(*(int *)gvar_007DA7BC + 0x9e0 + uVar2 * 4) != 0) {
// :118  uVar3 = *(uint *)(*(ushort *)(*(int *)(*(int *)gvar_007DA7BC + 0x9e0 + uVar2*4) + 4));
// :388-390  uVar1 = FUN_007746ac(gvar_007DA540, *(ushort *)(*(player+0x9e0+uVar2*4) + 4));
// :399   IntToStr(*(undefined4 *)(*(int *)gvar_007DA540 + 0x5c + uVar3 * 2), &local_2c);
// :400   _LStrCatN(&local_24, 4, __dummy_34, "kim", local_2c, " ", pcVar6);
```
* `player+0x9E0[1..25]` = 25 ô túi bán (cùng cấu trúc offset `+4/+7/+8/+0xB/+0xC/+0x10` với `player+0x2C[]` túi thường — xem `0077c64c_FUN_0077c64c.c:74,85`).
* **Giá lấy từ `Item.Dat` offset `0x5C`** (khác `0x58` của form MUA ⇒ `0x58` = giá mua, `0x5C` = giá bán). ✔ claim "Item.dat price/sell_price" — **chính xác**.

`player+0x9E0[]` được server đẩy qua `FUN_00730f4c` (main `0x17` sub `0x1E`):
```c
// 00730f4c_FUN_00730f4c.c:113-156
local_1c = (uVar3 - iVar5) / 5;                     // (len - (n+3)) / 5  ⇒ stride 5
do {
  uVar3 = iVar5 + local_18 * 5;                     // :129
  local_f = *(byte *)(local_c + uVar2);             // :135  byte index (0..25)
  _LStrCopy(local_c, uVar3 + 1, 4, &local_13c);     // :145  4 ký tự
  local_14 = FUN_0077ef7c(gvar_007D9D30, local_13c);// :146  → DWORD
  FUN_00768638(*(int *)(*(int *)gvar_007DA7BC + 0x9e0 + local_f * 4), local_14); // :156
} while (--local_24);
```

**Kết luận 1.b:**
* **XÁC MINH**: danh sách hàng + giá form MUA đến 100% từ file cục bộ (`Eve.emg` + `Item.Dat`), **không** có frame S→C nào mang nó.
* **BÁC BỎ (chi tiết phụ)**: prompt/claim gọi `TThingSaleForm` (form **BÁN**) là form shop; form BÁN **không** dùng eve mà dùng `player+0x9E0[]` (do `17 1E` push). Tuy nhiên kết luận "server không gửi danh sách" vẫn đúng cho cả hai, vì `17 1E` là **container túi bán** do người chơi tự kéo item vào (`0077c64c` là drag&drop handler), không phải catalogue NPC.

#### Về `TThingSaleForm` — render method nào?
* VMT `0x5D5438`, ctor `FUN_005dc500` — xác nhận tại `test/…/0051189c_FUN_0051189c.c:1445-1446`:
  ```c
  uVar2 = FUN_005dc500((int *)VMT_5D5438_TThingSaleForm,'\x01');
  *(undefined4 *)gvar_007DA720 = uVar2;
  ```
  (Dòng 1445-1446 trong `0051189c` — **đúng như prompt nêu**.)
* Render: `FUN_005dcfd4` (44 B) → `FUN_005dd000` (1904 B). `FUN_005dcfd4` được gọi qua con trỏ hàm `0x5D549C` (`:FUN_005dcfd4…c:11 // References to entry: 005d549c -> 005dcfd4 [DATA]`) — đúng vai trò event `OnShow`/paint.
* **`0050705c` KHÔNG phải form shop** — nó là `TMR_Slotform` (slot machine), `case 64` = main `0x48`. Đã kiểm chứng qua `opcode_48.md` + `jumptable_byte200`: `0x48 → case 64 (0x40) → 0x00796248`. Prompt gợi ý đọc `0050705c` là hướng sai.

### 1.c. NPC nào → shop nào: quyết định ở đâu?

**Bảng ánh xạ nằm trong `Eve.emg` (Surface section), client-side.** Chuỗi:

```
[14][01..06]  S→C  →  ghi W3 = index surface record vào PlayerState+0xA0A8
                      (0078a89c_FUN_0078a89c.c:3019-3093)
   ↓
FUN_005eb530(PlayerState)
   :829  *(char *)(*(int *)(local_8 + 0xa0d4) + 0x74) == '\x01'   → mode = hộp thoại/shop
   :833  *(int *)(*(int *)(local_8 + 0xa0d4) + 0xa0d4…)  = PlayerState[0x9D34 + W3*4]   ← trỏ TSurface
   :834  FUN_005d72b4(gvar_007DA5A8)  → reset list TInterForm
   :838-841  form+0x14E = rec+0x6E (shopId) ; form+0x150 = player+0x63A (mapId)
   :847  FUN_005c340c-ish: *(iVar23+0xe8) = form ; *(iVar23+0x153)=0x23 ; *(iVar23+0x152)=0x20
   ↓
Click 1 item → FUN_005d73c0 (OnChange của TBK_BiImgList)
   :103  *(char *)(local_8 + 0x152) = (char)local_c;      ← INDEX GỬI LÊN SERVER
   :105  *(undefined1 *)(local_8 + 0x156) = 1;
```

Đối chiếu Rust hiện tại (`src/server/handlers/shops.rs:51-160` `get_npc_shop_price(idtalking, map_id, menu)`) — server **tự chế bảng ánh xạ**, đúng như tài liệu `01_shop_read_flow.md` đã cảnh báo là dư.

**Đối chiếu `src/data/loaders/eve.rs` (Rust):** không có section "Surface"/shop; chỉ nạp scene/npc-encounter. ⇒ xác nhận server Rust **không** có bảng NPC→shop từ eve.

### 1.d. Kiểm tra chéo server C# — **BẰNG CHỨNG RẤT MẠNH, ủng hộ claim**

`TS_Server_Bear/TS_Server/PacketHandlers/NpcShopsHandler.cs:20`:
```csharp
List<NpcShops> list = EveData.listNpcShopOnmap[mapid]
    .FindAll((NpcShops item) => item.shopsID == client.shopid && item.indexId == index).ToList();
```
* `EveData.listNpcShopOnmap` — `DataTools/EveData.cs:96`
* Được **parse từ file eve**: `EveData.cs:1002-1044`:
  ```csharp
  ushort num38 = read16(array4, num14);   num14 += 2;      // số shop group
  for (int num40 = 0; num40 < num38; num40++) {
      byte shopsID = array4[num14];  num14 += 4;
      byte b7 = array4[num14];       num14 += 3;            // số entry
      byte b8 = array4[num14];       num14 += 1;
      if (b8 > 0)
        for (byte b9 = 0; b9 < b7 - 1; b9++) {               // ← index 0..n-2, 0-BASED
            item7.shopsID = shopsID;
            item7.indexId = b9;                              // ← 0-BASED
            item7.itemId = read16(array4, num14 + num41);    // ← WORD LE
            num41 += 5;                                      // ← STRIDE 5
        }
      num14 += 5 * b7;
  }
  ```
* **`read16` = Word LE**: `EveData.cs:130-134` → `return (ushort)(data[off] + (data[off+1] << 8));`

⇒ **C# server CŨNG parse eve.emg cục bộ, stride 5, itemId = Word LE, index 0-based.** Đây là bằng chứng độc lập mạnh nhất cho claim: nếu client **không** có data cục bộ thì C# không thể chỉ dùng `index` mà tra được `itemId`.

⚠ **Lưu ý phản chứng một phần:** C# dùng `itemId` (Word) + `ItemData.itemList[itemId].BuyingPrice` (`:23-25`), tức **C# tự tính giá từ `Item.dat`**, khớp client. Nhưng C# dùng `byte count = data[3]` (`NpcShopsHandler.cs:14`) — chỉ **1 byte**, trong khi client gửi **2 byte**. Xem 1.e.

### 1.e. `1B 01 <index> <count>` — layout byte-exact + base của index

Đã truy ngược bằng **2 bảng hex + asm thật** (KHÔNG dùng `case 0x1b: break;`):

```
0077f467: MOV AL,byte ptr [EAX + 0x77f474]     ; byte_table[opcode]
0077f46d: JMP dword ptr [EAX*0x4 + 0x77f53c]   ; dword_table[idx]
```
`client_pseudo_c/jumptable_byte200…` không phải bảng này; bảng đúng là `cs_dispatch_byte200_0x77f474.hex` + `cs_builder_dword200_0x77f53c.hex`. Giải mã (bỏ dòng `;`):
```
op 0x1B → byte[0x1B] = 0x1A (26) → dword[26] = 0x00785FFD
```

ASM builder tại `0x00785FFD` (`0077f414_FUN_0077f414.asm.txt:1809-1917`):
```asm
00785ffd: MOV AL,byte ptr [EBP + -0x6]   ; CL = SubOp
00786000: DEC AL
00786002: JZ  0x00786011                 ; SubOp==1 → MUA
00786004: DEC AL
00786006: JZ  0x007860c8                 ; SubOp==2 → BÁN
0078600c: JMP 0x0078a4f2                 ; ≥3 → KHÔNG gửi
--- SubOp 1 (MUA) ---
00786014: MOV DL,byte ptr [EBP + -0x5]   ; DL = MainOp = 0x1B
0078603a: MOV CL,0x2   ; PStrNCat nối 2 byte → [1B][01]
00786057: MOV DL,byte ptr [EDX + 0x152]   ; ← itemIndex : 1 BYTE, từ TInterForm+0x152
00786069: MOV CL,0x3   ; nối byte thứ 3
00786092: MOV DX,word ptr [EAX + 0x154]  ; ← qty : WORD
007860a0: CALL 0x0077eb1c                 ; Word → 2 RAW BYTE LE
007860be: CALL 0x0051633c                 ; TForm1.CY_AddSedQueue
--- SubOp 2 (BÁN) ---
0078610e: MOV DL,byte ptr [EDX + 0x160]   ; ← invSlot : 1 BYTE, từ TThingSaleForm+0x160
00786140: MOV DX,word ptr [EDX + 0x162]  ; ← qty : WORD
00786147: MOV byte ptr [EAX + 0x1],DL    ; ⚠ chỉ byte THẤP được nối
00786158: MOV CL,0x4
```

**Kết luận layout (byte-exact):**

| Frame | Payload | Tổng | Bằng chứng |
| :-- | :-- | :-: | :-- |
| `1B 01` (MUA) | `[1B][01][itemIndex:1][qty:2 LE raw]` | **5 B** | asm `:1821-1866` |
| `1B 02` (BÁN) | `[1B][02][invSlot:1][qty:1]` | **4 B** | asm `:1869-1916` (chỉ `DL` từ `DX` ở `:1904`) |

**Base của index: 0-BASED, là index entry trong list của NPC đó.**

Bằng chứng:
1. `FUN_005d73c0:103` gán `form+0x152 = (char)local_c` trong đó `local_c` là chỉ số dòng mà `TBK_BiImgList` báo (`FUN_007c340c(*(form+0x124), local_c)` ở `:104` — cùng `local_c`).
2. `local_c` được dùng để tra TSurface: `005d73c0:57-85`:
   ```c
   uVar3 = *(byte *)(local_14 + 0x72) + param_2;   // pageBase + row
   uVar4 = uVar3 * 5;
   local_18 = (ushort *)(local_14 + 4 + uVar4);    // itemId
   ```
   Với `pageBase = 0` (page đầu) và `param_2 = 0` ⇒ đọc `rec+4` — **entry đầu tiên**. ⇒ **0-based**.
3. Khớp C#: `EveData.cs:1029` `for (byte b9 = 0; b9 < b7 - 1; b9++) { … item7.indexId = b9; }` — `indexId` chạy **0,1,2,…** và `NpcShopsHandler.cs:20` so sánh thẳng `item.indexId == index`.

⚠ **Quan sát mâu thuẫn (C# vs client) — cần lưu ý:**
* Client `1B 02` (BÁN) gửi `invSlot` 0-based, nhưng C# `NpcShopsHandler.cs:46` đọc `inventory.items[index - 1]` — **1-based** ⇒ **C# Bear có off-by-one ở nhánh BÁN**. Đây là bug của C# tham chiếu, không phải của client.
* C# `NpcShopsHandler.cs:14` đọc `byte count = data[3]` — chỉ 1 byte, trong khi client MUA gửi **2 byte** LE ⇒ C# Bear **bỏ byte high** của qty. Cũng là giới hạn của C#.

### 1.f. Đánh giá tổng kết Claim 1

| Thành phần claim | Verdict |
| :-- | :-- |
| Main `0x1B` không mang danh sách hàng | ✅ **XÁC MINH** — `case_024_00791D80…c:30-108`: `0x1B` chỉ có 3 nhánh: sub 1/2 = **toast lỗi** (`_LStrLAsg(…,0x798118/138/160/190/1C4/1E8)`), sub 3 = **`Show(TThingSaleForm)`** (`(***gvar_007DA720 + 0x20)()`). Không đọc itemId/giá nào. |
| Danh sách từ `eve.emg`, stride 5, itemId = Word LE | ✅ **XÁC MINH** cho form MUA (TSurface): `005efee8:487` đọc 5B/mục; `005eb530:1090` đọc `ushort` tại `+4+j*5` |
| Giá từ `Item.dat` (price/sell_price) | ✅ **XÁC MINH** — offset `0x58` (mua, `005eb530:1137`) và `0x5C` (bán, `005dd000:399`) trong record 185-word của `gvar_007DA540` |
| Server chỉ nhận `1B 01 <index:1> <count:u16 LE>` | ⚠ **XÁC MINH một nửa** — index 1 byte ✔, nhưng `count` là **Word LE 2 byte** (asm `:1860` `CALL 0x0077eb1c`), tổng frame **5 byte**. Claim ghi `count:u16 LE` là **đúng**; nhưng cần nêu rõ `1B 02` chỉ 1 byte qty. |
| Form shop là `TThingSaleForm` | ❌ **BÁC BỎ** — `TThingSaleForm` = form **BÁN** (25 ô, `btn_Sale`), dữ liệu từ `player+0x9E0[]`. Form **MUA** là `TInterForm`/`form_BuyQ_Npc` (`gvar_007DA5A8`). |

---

## 2. CLAIM 2 — `player+0x12FC` là tiền bank, set qua `0x1D` sub `0x04`

### 2.a. Bảng sub-op của main `0x1D` (byte-exact)

Nguồn: `case_025_00791F76_FUN_00791f76.c` (`.asm.txt` **THIẾU** — xác nhận: không có `00791f76_*.asm.txt` trong `client_pseudo_c/` lẫn `test/ts_decompile/functions/`; đối chiếu bằng bản inline `0078a89c_FUN_0078a89c.c:4846-4914` + `jumptable_byte200_0x78A8EE.hex`).

Mapping: `byte_table[0x1D] = 0x19 (25)` → `dword_table[25] = 0x00791F76` ✔ (đã tự parse lại hex: `0x1A→23, 0x1B→24, 0x1D→25, 0x1F→27, 0x23→31`).

| Sub | Payload | Đọc | Lệnh gọi | Offset đích | file:line |
| :-: | :-- | :-- | :-- | :-- | :-- |
| `0x01` | `[1D][01][amt:4 LE]` (6B) | `_LStrCopy(RP,2,4)` + `FUN_0077ef7c` | `FUN_0072b9b8` | **`player+0x12FC += amt`** (clamp 9 999 999) | `case_025…c:30-43`; `0072b9b8…c:28-35` |
| `0x02` | `[1D][02][amt:4 LE]` (6B) | như trên | `FUN_0072ba0c` | **`player+0x12FC -= amt`** (không âm) | `case_025…c:44-57`; `0072ba0c…c:27-30` |
| `0x03` | `[1D][03]` (2B) | — | `(**(**gvar_007DA084 + 0x90))(&UNK_00798314, 2000, 0, 0)` | banner tĩnh 2000 ms | `case_025…c:58-61` |
| **`0x04`** | `[1D][04][money:4 LE]` (6B) | `Copy(RP,2,4)` **trong callee** | `FUN_0072be10` | **`player+0x12FC = money` (SET TUYỆT ĐỐI)** | `case_025…c:62-64`; `0072be10.asm.txt:30` |
| `0x05` | `[1D][05]` (2B) | — | `(**(**gvar_007DA46C + 0x20))()` | Show `TMoneyAccountForm` | `case_025…c:65-67` |
| `0x06` | `[1D][06]` (2B) | — | `(**(**gvar_007DA0A4 + 0x20))()` | Show `TThingAccountForm` | `case_025…c:68-70` |
| `0x07` | `[1D][07]` (2B) | — | — | `*(*gvar_007DA46C + 0x11C) = 0` | `case_025…c:71-73` |
| `0x08` | `[1D][08]` (2B) | — | — | `*(*gvar_007DA46C + 0x11D) = 0` | `case_025…c:74-76` |
| `0x09` | `[1D][09][x:1B]` (3B) | guard `len(RP)≥2`, `RP[1]` | `FUN_00749c30` | `**gvar_007DA0A4 + 0x418 = x` + redraw | `case_025…c:77-79` |

Không có `default` ⇒ sub `0x00` / `>0x09` là no-op.

### 2.b. Byte thật trong 4 helper tiền

#### `FUN_0072be10` — **SET `+0x12FC`** (`0072be10_FUN_0072be10.asm.txt`, 129 B — **ĐỦ ASM**)
```asm
0072be49: MOV ECX,0x4
0072be4c: MOV EDX,0x2
0072be50: MOV EAX,dword ptr [EBP + -0x8]
0072be53: CALL 0x00404290                 ; _LStrCopy(RP, 2, 4) → 4 byte P[2..5]
0072be58: MOV EDX,dword ptr [EBP + -0xc]
0072be5b: MOV EAX,[0x007d9d30]
0072be60: CALL 0x0077ef7c                 ; DWORD LE
0072be65: MOV EDX,dword ptr [0x007da7bc]
0072be68: MOV EDX,dword ptr [EDX]
0072be6a: MOV dword ptr [EDX + 0x12fc],EAX   ; ★ SET TUYỆT ĐỐI, KHÔNG CLAMP
0072be6d: MOV EAX,[0x007da46c]
0072be74: CALL 0x005d895c                 ; refresh TMoneyAccountForm
```
**Đây là byte bằng chứng trực tiếp cho claim 2.** `0x12fc` xuất hiện đúng một lần, dạng `MOV [reg+imm],EAX` (set), không phải `ADD`/`SUB`.

#### `FUN_0072b9b8` — **CỘNG `+0x12FC`, trần 9 999 999** (`.c` 84 B; **không có `.asm.txt`**)
```c
0072b9b8…c:28   uVar3 = (int)&DAT_0098967f - *(int *)(param_1 + 0x12fc);
0072b9b8…c:29   if (SBORROW4(9999999,*(int *)(param_1 + 0x12fc))) { uVar3 = _IntOver(uVar3); }
0072b9b8…c:32   if ((int)param_2 <= (int)uVar3) {
0072b9b8…c:33     piVar1 = (int *)(param_1 + 0x12fc);
0072b9b8…c:35     *piVar1 = *piVar1 + param_2;
0072b9b8…c:40     FUN_005d895c(*(int *)gvar_007DA46C);
```
Trần đọc thẳng từ hằng: `0x0098967F` = **9 999 999**. Hằng này được xác nhận chéo bằng **asm** của hàm anh em `FUN_0072b084` (cùng clamp trên `0x12F8`): `0072b084_FUN_0072b084.asm.txt:18` → `MOV EAX,0x98967f`. Trả về `bool` (1 = ghi thành công).

#### `FUN_0072ba0c` — **TRỪ `+0x12FC`** (`.c` 72 B; không asm)
```c
0072ba0c…c:27   if ((int)param_2 <= *(int *)(param_1 + 0x12fc)) {
0072ba0c…c:30     *piVar1 = *piVar1 - param_2;
0072ba0c…c:35     FUN_005d895c(*(int *)gvar_007DA46C);
```

#### `FUN_0072b084` / `FUN_0072b170` — **CỘNG/TRỪ `+0x12F8`** (có ASM đủ)
```asm
0072b084…asm:18  MOV EAX,0x98967f
0072b084…asm:19  MOV EDX,dword ptr [EBP + -0x4]
0072b084…asm:20  SUB EAX,dword ptr [EDX + 0x12f8]     ; cap = 9999999 - v
0072b084…asm:22  CMP EAX,dword ptr [EBP - 0x8] / JL   ; guard
0072b084…asm:26  ADD dword ptr [EDX + 0x12f8],EAX
; tương tự:
0072b170…asm:19  CMP EAX,dword ptr [EBP - 0x8] / JL
0072b170…asm:23  SUB dword ptr [EDX + 0x12f8],EAX
```
Hai hàm này **không** đụng `0x12FC`.

### 2.c. Đối chiếu `opcode_1a.md` / `opcode_1d.md` — claim nói tài liệu SAI, nhưng thực tế:

**`opcode_1a.md:187`** (nguyên văn):
> `form+0x114 := Format('%7d', player+0x12FC)`, `form+0x118 := Format('%7d', player+0x12F8)` … **Chú ý: `+0x12FC` KHÔNG bị gói này ghi**, chỉ được hiển thị lại.

→ Cụm **"gói này"** chỉ `1A 04` (`FUN_0072bcf8`), và điều đó **ĐÚNG**: `0072bcf8_FUN_0072bcf8.c:52-56` chỉ ghi `0x12F8` và `0x1300`. Cùng tài liệu `:274,276` **đã trích dẫn** `0072b9b8/0072ba0c` như nguồn trần của `0x12FC`. ⇒ **`opcode_1a.md` KHÔNG sai.** Claim diễn giải lại thành "không bị gói nào ghi" — đó là **misquote**.

**`opcode_1d.md`** — **đã đặc tả đúng và đầy đủ từ trước**:
* §1: "**`0x04` = SET tuyệt đối số dư `player+0x12FC`**"
* §3 bảng: `0x04 | [1D][04][money:4B LE] (6B) | … | player+0x12FC = money (SET tuyệt đối)`
* §4.4 tiêu đề: "**SET tuyệt đối số dư `player+0x12FC`**"
* §8 source-trail #6: `functions/0072be10…c:37–40` → "Body mới: SubOp 0x04 = SET `player+0x12FC`"

⇒ **Verdict phần "đính chính opcode_1a.md": BÁC BỎ.** Không có sai lệch tài liệu cần sửa ở claim 2.

Điều `opcode_1d.md` **thiếu** (CHƯA ĐỦ BẰNG CHỨNG trong tài liệu, nhưng tôi đã xác minh được): nó không nêu rõ `+0x12FC` là **tiền bank** và không nêu chiều C→S `1D` (`SendCommand` builder `0x00786185` ở `0077f414…asm.txt:1918-1948` đọc `*(*gvar_007DA46C + 0x110)` → `FUN_0077ee84` 4 byte LE).

### 2.d. Mọi call-site đọc/ghi `0x12FC` + form hiển thị

Grep `0x12fc` trong `client_pseudo_c/` — **5 vị trí, đã đọc hết**:

| # | file:line | Phép | Ngữ nghĩa |
| :-: | :-- | :-- | :-- |
| 1 | `0072be10_FUN_0072be10.asm.txt:30` / `.c:39` | `SET` | **`1D 04` — set số dư bank** |
| 2 | `0072b9b8_FUN_0072b9b8.c:28,29,33,35` | `+=` (clamp 9999999) | `1D 01` — cộng bank (nạp tiền) |
| 3 | `0072ba0c_FUN_0072ba0c.c:27,28,30` | `-=` (no âm) | `1D 02` — trừ bank (rút tiền) |
| 4 | `005d895c_FUN_005d895c.c:59` | đọc | `Format('%7d', +0x12FC)` → `form+0x114` |
| 5 | `005d895c_FUN_005d895c.c:68-70` | đọc `0x12F8` | `Format('%7d', +0x12F8)` → `form+0x118` |

**Form hiển thị: `TMoneyAccountForm` (`Form_Bank_Money`, `gvar_007DA46C`)**, không phải `Form_Bank_Money`/`TMoneyAccountForm` tách bạch:
* `FUN_005d7d18` = ctor, tạo `btn_Bank_Gold_R/L` (theo `01_npc_shop.md` §0)
* `FUN_005d895c` = hàm refresh: `form+0x114 ← '%7d' % (0x12FC)`, `form+0x118 ← '%7d' % (0x12F8)`
* Được Show bằng `1D 05` (`case_025…c:65-67`), clear cờ bằng `1D 07/08`
* Chiều C→S: `1D` builder đọc `*(*gvar_007DA46C + 0x110)` (asm `:1941`) — **field input số tiền**, khác `+0x114/+0x118` (label hiển thị)

**Xác nhận ngữ nghĩa "tiền bank" qua nhãn VISCII** (`opcode_1d.md` §5, đã có sẵn):
| Hằng | Chuỗi | Sub |
| :-- | :-- | :-: |
| `0x7982B8` | `Lưu trữ` | `1D 01` (cộng) — thành công |
| `0x7982C8` | `Thất bại lưu trữ` | `1D 01` — thất bại |
| `0x7982E4` | `Rút nhận` | `1D 02` (trừ) — thành công |
| `0x7982F8` | `Thất bại rút nhận` | `1D 02` — thất bại |
| `0x798314` | `Dung lượng tiền trang không đủ` | `1D 03` |

⇒ "Lưu trữ" (nạp) / "Rút nhận" (rút) trên **`+0x12FC`** ⇒ **XÁC MINH mạnh: đây là tiền trong trang/bank.**

### 2.e. Xác minh thêm: `1A 04` set `+0x12F8` và `+0x1300` (không phải `+0x12FC`)

`0072bcf8_FUN_0072bcf8.c` (236 B; `.asm.txt` **có**):
```c
:51  _LStrCopy(param_2, 2, 4, &local_10);                      // P[2..5]
:52  uVar1 = FUN_0077ef7c(gvar_007D9D30, local_10);
:53  *(undefined4 *)(*(int *)gvar_007DA7BC + 0x12f8) = uVar1;  // ★ tiền mang theo
:54  _LStrCopy(local_c, 6, 4, &local_14);                       // P[6..9]
:55  uVar1 = FUN_0077ef7c(gvar_007D9D30, local_14);
:56  *(undefined4 *)(*(int *)gvar_007DA7BC + 0x1300) = uVar1;  // ★ tiền mềm/điểm
:57  FUN_005d895c(*(int *)gvar_007DA46C);
```
**XÁC MINH** — `1A 04` set `0x12F8` + `0x1300`, **không** đụng `0x12FC`. (Tương xứng với `opcode_1a.md` §4.4 và `06_currency_model.md` §0.)

### 2.f. Có bao nhiêu field tiền thật?

**ĐÚNG 3 field `int32` liên tiếp** (`0x12F8`, `0x12FC`, `0x1300`) + 1 nhóm bộ đếm tài nguyên. Kiểm chứng bằng **3 nguồn độc lập**:
1. Ba hàm `SET` duy nhất: `0072bcf8` (`0x12F8`,`0x1300`), `0072be10` (`0x12FC`).
2. `FUN_005d895c` chỉ hiển thị `0x12FC` + `0x12F8` (`%7d` → 2 label). `0x1300` hiển thị ở paint routine `00632e3c_FUN_00632e3c.c:93` (`IntToStr(+0x1300)`).
3. C# `TSCharacter.sendGold()` = `PacketCreator(26,4).add32(gold).add32(gold_bank)` (`opcode_1a.md` §2.9) — 2 số dư.

---

## 3. CLAIM 3 — `23 04` không set field tiền nào

### 3.a. `0x23` sub `0x04` làm gì (byte-exact)

Nguồn: `case_031_00792BAD_FUN_00792bad.c` (338 dòng, đã đọc **toàn bộ**; `.asm.txt` **THIẾU**). Mapping: `byte_table[0x23] = 0x1F (31)` → `dword_table[31] = 0x00792BAD` ✔.

```c
// case_031_00792BAD_FUN_00792bad.c:167-203
167  case 4:
168    _LStrCopy(RP, 2, 4, &local_488);
169    iVar2 = FUN_0077ef7c(gvar_007D9D30, local_488);   // A = P[2..5]  DWORD LE
170    _LStrCopy(RP, 6, 4, &local_48c);
171    uVar3 = FUN_0077ef7c(gvar_007D9D30, local_48c);   // B = P[6..9]  DWORD LE
175    IntToStr(iVar2 + uVar3, &local_484);
176    _LStrCat3(&local_2c, &DAT_0079889c, &local_484);
178    _LStrCopy(RP, 10, 8, &local_490);
179    FUN_0077f098(gvar_007D9D30, &local_490);          // D = P[10..17] → double
180    if (in_ST0 != (float10)_DAT_007988ac) {           // _DAT_007988ac = 0.0
181      _LStrCopy(RP, 10, 8, &local_498);
182      FUN_0077f098(…);                                 // ⚠ đọc LẠI CÙNG OFFSET
183      FUN_007c56c8(&local_494, …);                      // Format double
185      _LStrCatN(&local_2c, 4);
186    }
187    (**(code **)(**(int **)gvar_007DA084 + 0x90))
188          (*gvar_007DA084, local_2c, 10000, 0, 0);      // ★ BANNER 10 000 ms
189    FUN_007ab870(*(int *)gvar_007DA1B0, 0, local_2c, '\n');  // ★ ghi 1 dòng chat-log
190    _LStrCopy(RP, 2, 4, &local_49c);
191    uVar3 = FUN_0077ef7c(gvar_007D9D30, local_49c);    // A (đọc lại)
192    iVar2 = (int)((longlong)(int)uVar3 * 100);         // ★ A * 100
196    _LStrCopy(RP, 6, 4, &local_4a0);
197    uVar3 = FUN_0077ef7c(gvar_007D9D30, local_4a0);    // B (đọc lại)
201    *(uint *)(*(int *)gvar_007DA7BC + 0x146c) = iVar2 + uVar3;   // ★★ GHI DUY NHẤT
```

**Kết luận byte-exact:**
* Wire: `[23][04][A:4 LE][B:4 LE][D:8 double]` = **18 byte payload**
* Ghi: **duy nhất `player+0x146C = A*100 + B`** (`:201`)
* Không đụng `0x12F8` / `0x12FC` / `0x1300` ✔ **XÁC MINH**
* Side-effect: banner **10 000 ms** (`:188`) + 1 dòng chat-log kênh 0 (`:189`)

⚠ **Ghi chú phản chứng (decompile noise):** dòng 181-182 đọc **lại cùng offset `RP[10..17]`** — khả năng cao là lỗi Ghidra (thiếu `.asm.txt`), nhiều khả năng offset thức là `RP[18..25]`. **Đây chỉ là caveat về payload, KHÔNG ảnh hưởng kết luận về offset ghi.**

`+0x146C` được khởi tạo 0 tại `TPlayer.Create` (`test/…/00716370_TPlayer.Create.asm.txt:190` → `MOV dword ptr [EAX + 0x146c],EDX` với `EDX=0`), và cũng được ghi bởi `00522ab8_FUN_00522ab8.c:206,220,227` (main `0x42`) — một số hiển thị, **không phải tiền tệ**.

### 3.b. Rust đang gửi gì?

`src/server/spawn.rs:325-330`:
```rust
pub fn store_frame(point: u32) -> String {
    let mut body = String::new();
    body.push_str(&encoder::le32(point));
    body.push_str(&"00".repeat(12));
    crate::protocol::frame("2304", &body)
}
```
`src/server/spawn.rs:592-594`:
```rust
frames.push(store_frame(s.god));        // 0
frames.push(store_frame(s.hp_store));   // 10000
frames.push(store_frame(s.sp_store));   // 10000
```
`src/server/session.rs:345-347`: `god: 0, hp_store: 10000, sp_store: 10000`.

**⇒ Mỗi lần login, server gửi 3 frame `23 04`. Phía client:**
* `A = point`, `B = 0`, `D = 0.0` (12 byte 0)
* **`player+0x146C` bị ghi đè 3 lần: 0 → 1 000 000 → 2 000 000** (giá trị cuối = `sp_store*100`)
* **3 banner 10 giây** hiện lên màn hình ("0", "10000", "10000" — nhãn `DAT_0079889c` chưa dump)
* **3 dòng chat-log** kênh 0

**XÁC MINH đầy đủ claim 3.** Comment trong `spawn.rs:322-324` gọi đây là "God / HP store / SP store frame: op 0x23/OP_ACCOUNT sub 0x04" — **nghĩa sai hoàn toàn**.

---

## 4. Bảng offset tiền cuối cùng (đã kiểm chứng)

| Offset | Kiểu | Tên nghiệp vụ | Trần | **SET tuyệt đối** | **Delta +** | **Delta −** | Hiển thị | Bằng chứng |
| :-- | :-- | :-- | :-- | :-- | :-- | :-- | :-- | :-- |
| **`player+0x12F8`** | int32 | **Tiền mang theo (kim)** | 9 999 999 | **`1A 04`** (P[2..5]) | `1A 01` | `1A 02` | `Form_Bank_Money+0x118`; HUD `btn_001` | `0072bcf8.c:53`; `0072b084.asm:26`; `0072b170.asm:23`; `005d895c.c:68-70` |
| **`player+0x12FC`** | int32 | **Tiền trong trang / bank** | 9 999 999 | **`1D 04`** (P[2..5]) | `1D 01` | `1D 02` | `Form_Bank_Money+0x114` | `0072be10.asm:30` ★; `0072b9b8.c:35`; `0072ba0c.c:30`; `005d895c.c:59` |
| **`player+0x1300`** | int32 | **Tiền mềm / điểm** | 2 000 000 000 (`0x77359400`) | **`1A 04`** (P[6..9]) | `1A 05` | `1A 06` | `00632e3c.c:93` | `0072bcf8.c:56`; `00745acc.c:25-30`; `00745b14.c:26-32` |
| `player+0x0502` | uint16 | Thủy binh | động `+0x0500` | `08 06` sel 1 | `1A 08` | — | HUD | `0074c64c.c` |
| `player+0x050E` | uint16 | Điểm đạn dược | động `+0x050C` | `08 06` sel 3 | `1A 0A` | — | HUD | `0074c6b4.c` |
| `player+0x0513` | uint8 | Tài bảo | 250 (`0xFA`) | `08 06` sel 4 | `1A 09` | — | HUD | `0074c5f4.c` |
| **`player+0x146C`** | uint32 | **KHÔNG phải tiền** — bộ đếm `A*100+B` | — | — | `23 04` (set, không delta) | — | paint routine | `case_031…c:201` ★; `TPlayer.Create.asm:190` |

**Chiều C→S tiền (đã kiểm chứng qua builder thật):**
| Frame | Payload | Bằng chứng |
| :-- | :-- | :-- |
| `1D` (bank input) | `[1D][DWORD LE = *(*TMoneyAccountForm + 0x110)]` = 6 B | `0077f414…asm.txt:1918-1948`, target `0x00786185`; đã map `byte[0x1D]=0x1B(27) → dword[27]=0x00786185` |

---

## 5. Sai lệch tài liệu cần sửa

| # | File:line | Nội dung hiện tại | Mức | Kết luận verifier |
| :-: | :-- | :-- | :-- | :-- |
| **1** | — | — | — | **`opcode_1a.md:187` KHÔNG cần sửa.** Cụm "KHÔNG bị gói này ghi" là đúng phạm vi (`1A 04`). Claim 2 diễn giải sai. |
| **2** | `opcode_1d.md` §1, §3, §4.4 | Đã đặc tả `1D 04 = SET +0x12FC` đúng | — | **Không cần sửa.** Thiếu: nhãn "tiền bank" cho `+0x12FC` và wire C→S `1D` — *bổ sung mức B* (`0077f414…asm.txt:1918-1948`). |
| **3** | `01_npc_shop.md` / `01_shop_read_flow.md` | "NPC→shop lấy từ Eve.emg" | — | **ĐÚNG.** Nhưng nên bổ sung: giá mua ở `Item.Dat+0x58`, giá bán ở `+0x5C` (record 185-word) — hiện tài liệu ghi `rec[84]` chưa rõ offset. |
| **4** | `01_npc_shop.md` §2.1 | `1B 01 = [1B][01][itemIdx:1][qty:2 LE]` 5 B | — | **ĐÚNG, đã xác nhận lại bằng asm.** Nên thêm: `1B 02` chỉ **1 byte qty** (4 B tổng) — hiện §2.2 đã ghi đúng. |
| **5** | `opcode_1b.md:122` | "client không bao giờ gửi OP 0x1B" | — | **SAI — cần sửa.** Đã bác bỏ bằng bảng hex + asm: `byte[0x1B]=0x1A(26) → dword[26]=0x00785FFD`, có 2 nhánh gửi thật (`0077f414…asm.txt:1809-1917`). `01_shop_read_flow.md` §1.1 đã nêu đúng, nhưng `opcode_1b.md` chưa sửa. |
| **6** | `opcode_23.md:12,73,104` | `23 04` → banner + chat + `+0x146C` | — | **ĐÚNG, đã xác nhận lại.** Không cần sửa. |
| **7** | `06_currency_model.md` §2.4, §G3 | `23 04` không set tiền, spam banner | — | **XÁC MINH đúng.** Bổ sung: giá trị cuối cùng của `+0x146C` sau login = `sp_store*100` = 2 000 000. |
| **8** | `src/server/spawn.rs:322-330` | Comment "God / HP store / SP store frame" | **A** | **SAI — cần sửa cả code lẫn comment.** `23 04` không liên quan god/hp/sp store. Nên thay bằng frame đúng (vd `1A 05`/`1A 06` cho credit, hoặc bỏ hẳn nếu không có nghiệp vụ). |

---

## 6. Hàm ý hành động cho server Rust (theo mức tin cậy)

1. **`spawn.rs:592-594` — SỬA NGAY (Claim 3, mức A).** Ba frame `23 04` mỗi lần login không set tiền nào, chỉ ghi `+0x146C` + spam 3 banner 10 s + 3 dòng chat-log. Nếu `god`/`hp_store`/`sp_store` cần hiển thị, phải tìm frame đúng; hiện **chưa xác minh được** frame nào hiển thị chúng ⇒ mức an toàn: **xoá 3 dòng này** (hoặc thay bằng `no-op` có log).
2. **`shops.rs:51-160` `get_npc_shop_price()` — CẤU TRÚC SAI (Claim 1, mức A cho cơ chế).** Client tự lấy `itemId` từ `Eve.emg` và `giá` từ `Item.Dat`; server chỉ cần tra `index → itemId` theo cùng bảng eve mà C# Bear đã parse (`EveData.cs:1002-1044`). Bảng hard-code hiện tại **lệch** với bảng eve thật ⇒ sẽ bán sai item.
3. **`spawn.rs:564` `1A04 {gold} 00000000` — ĐÚNG về `0x12F8` nhưng `0x1300` luôn = 0.** Nếu game cần điểm/credit khác 0 thì phải gửi giá trị thật ở byte `[6..9]`.
4. **`chat.rs:413` / `talk.rs:713` `1D04 {bank_gold}` — ĐÚNG** (đây là frame set `+0x12FC` duy nhất mà Rust dùng). Cần bổ sung `1D 01`/`1D 02` nếu muốn delta ± bank sau giao dịch (hiện Rust chỉ set tuyệt đối, dùng được nhưng không khớp hành vi client mong đợi sau 1 thao tác rút/nạp).
5. **Index `1B 01` là 0-based entry index** — nếu Rust sửa `get_npc_shop_price` theo bảng eve, phải dùng `indexId` 0-based như `EveData.cs:1029`, **không** trừ 1.

---

## 7. Source trail

| # | Nguồn | Địa chỉ / dòng | Dùng để |
| :-: | :-- | :-- | :-- |
| 1 | `client_pseudo_c/jumptable_byte200_0x78A8EE.hex` | `0x1A→23, 0x1B→24, 0x1D→25, 0x1F→27, 0x23→31` | Mapping MainOp → Case (tự parse lại) |
| 2 | `client_pseudo_c/case_025_00791F76_FUN_00791f76.c` | `:30-79` (switch SubOp 1..9) | Bảng sub-op `0x1D` byte-exact |
| 3 | `client_pseudo_c/0072be10_FUN_0072be10.asm.txt` | `:26-31` `MOV dword ptr [EDX + 0x12fc],EAX` | ★ BẰNG CHỨNG SET `+0x12FC` |
| 4 | `client_pseudo_c/0072b9b8_FUN_0072b9b8.c` | `:28-42` | Cộng `+0x12FC`, trần `0x0098967F` |
| 5 | `client_pseudo_c/0072ba0c_FUN_0072ba0c.c` | `:27-37` | Trừ `+0x12FC` |
| 6 | `client_pseudo_c/0072b084_FUN_0072b084.asm.txt` | `:18,26` `MOV EAX,0x98967f` / `ADD [EDX+0x12f8],EAX` | Trần 9 999 999 + cộng `0x12F8` |
| 7 | `client_pseudo_c/0072b170_FUN_0072b170.asm.txt` | `:19,23` `SUB [EDX + 0x12f8],EAX` | Trừ `0x12F8` |
| 8 | `client_pseudo_c/0072bcf8_FUN_0072bcf8.c` | `:51-57` | `1A 04` set `0x12F8` + `0x1300` |
| 9 | `client_pseudo_c/005d895c_FUN_005d895c.c` | `:59` (`0x12FC`→`form+0x114`), `:68-70` (`0x12F8`→`form+0x118`) | Form `TMoneyAccountForm`, `Format('%7d')` |
| 10 | `client_pseudo_c/case_031_00792BAD_FUN_00792bad.c` | `:167-203`, đặc biệt `:188` (banner 10000), `:201` (`MOV`-equivalent ghi `0x146C`) | ★ `23 04` không set tiền |
| 11 | `client_pseudo_c/case_024_00791D80_FUN_00791d80.c` | `:30-108` | `0x1B` chỉ toast + Show, không có catalogue |
| 12 | `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex` + `cs_builder_dword200_0x77f53c.hex` | `byte[0x1B]=0x1A(26) → dword[26]=0x00785FFD`; `byte[0x1D]=0x1B(27) → 0x00786185` | Builder C→S `1B`/`1D` (bác bỏ `case N: break`) |
| 13 | `client_pseudo_c/0077f414_FUN_0077f414.asm.txt` | `:1809-1917` (`1B 01`/`1B 02`), `:1918-1948` (`1D`) | Layout C→S byte-exact |
| 14 | `test/ts_decompile/functions/005746d4_FUN_007746d4.c` | `:74,83` `"\Data\Item.Dat"`; `:125` stride `0xB9` | Loader `Item.Dat` client-side |
| 15 | `test/ts_decompile/functions/0050a4a0_TForm1.FormCreate.asm.txt` | `:674-675` `MOV [0x009264bc],EAX` / `CALL 0x007746d4` | Call-site loader `Item.Dat` |
| 16 | `test/ts_decompile/functions/005efc24_FUN_005efc24.c` | `:72-83` root + `"\Data\"` + mapId → `TFileStream_Create` | Mở file dữ liệu map/eve |
| 17 | `test/ts_decompile/functions/005e8534_FUN_005e8534.c` | `:182,186` `PlayerState+0x9D34`, `FUN_005efee8(…, *(local_8+0xa0f0), …)` | Nạp map → parse eve |
| 18 | `test/ts_decompile/functions/005efee8_FUN_005efee8.c` | `:460-476` (tạo `TSurface`), `:487` đọc 5 B/mục, `:505` ghi `rec+4+j*5` | ★ Surface/shop record trong eve, stride 5 |
| 19 | `test/ts_decompile/functions/005eb530_FUN_005eb530.c` | `:829-851` (`rec+0x74==1` → TInterForm), `:1090` đọc `ushort`, `:1137` giá `+0x58` | ★ Form MUA lấy list từ TSurface, giá từ Item.Dat |
| 20 | `test/ts_decompile/functions/0051189c_FUN_0051189c.c` | `:1445-1446` `FUN_005dc500(VMT_5D5438_TThingSaleForm)` → `gvar_007DA720` | Định danh form BÁN (như prompt nêu) |
| 21 | `test/ts_decompile/functions/005dc500_FUN_005dc500.asm.txt` | `:117-175` tạo 25 `TFT_ItemImage`; `:161` `MOV dword ptr [EAX + 0xb8],EDX` (1..25) | Ctor form BÁN |
| 22 | `test/ts_decompile/functions/005dd000_FUN_005dd000.c` | `:91,118` đọc `player+0x9E0[]`; `:399` giá `+0x5C`; `:400` caption `"kim"` | ★ Form BÁN: nguồn = `player+0x9E0[]`, không phải eve |
| 23 | `test/ts_decompile/functions/00730f4c_FUN_00730f4c.c` | `:113-156` stride 5, `FUN_00768638(player+0x9E0+idx*4, itemId)` | `17 1E` đẩy túi bán |
| 24 | `test/ts_decompile/functions/005d73c0_FUN_005d73c0.c` | `:57-85` (pageBase + row → TSurface), `:103-105` set `form+0x152/+0x156` | ★ Index 0-based; giá từ `+0x58` |
| 25 | `TS_Server_Bear/TS_Server/PacketHandlers/NpcShopsHandler.cs` | `:13-14, 20, 46, 54` | ★ Bằng chứng chéo: C# cũng parse eve, `indexId` 0-based, giá từ `ItemData` |
| 26 | `TS_Server_Bear/TS_Server/DataTools/EveData.cs` | `:96` dict; `:1002-1044` parse shop (stride 5, `read16`, `indexId = b9` từ 0); `:130-134` `read16` = Word LE | ★ Bằng chứng chéo mạnh nhất cho Claim 1 |
| 27 | `TS_Server_Bear/TS_Server/Client/QuestStepHelper/DialogAndBattleEntryHandler.cs` | `:13-17` `shopid = GetDialogID`, `shopsnpcid = packageToSend[5]` | Cơ chế NPC→shop ở server C# |
| 28 | `TS_Server_Bear/TS_Server/PacketHandlers/GoldBankHandler.cs` | `:19-28, 32-41` — `1D 02`+`1A 01` khi rút, `1D 01`+`1A 02` khi nạp | ★ Xác nhận ngữ nghĩa `0x12FC` = tiền bank |
| 29 | `TS_Server_Bear/TS_Server/Client/QuestStepHelper/HotelShopAndSaveMapEffectHandler.cs` | `:14` `new PacketCreator(27, 3).send()` | `1B 03` payload rỗng → chỉ mở form |
| 30 | `src/server/spawn.rs` | `:322-330` (`store_frame`), `:592-594` (3 lần gọi) | ★ Rust đang spam `23 04` |
| 31 | `src/server/session.rs` | `:198-200, 345-347` `god=0, hp_store=10000, sp_store=10000` | Giá trị thực tế spam |
| 32 | `src/server/handlers/chat.rs` / `talk.rs` | `:413` / `:713` `F44406001D04{le32(bank_gold)}` | Rust dùng đúng frame set `0x12FC` |
| 33 | `src/server/handlers/shops.rs` | `:11-13` `gold_frame`, `:51-160` `get_npc_shop_price` | Rust hiện tại |
| 34 | `test/ts_decompile/functions/00716370_TPlayer.Create.asm.txt` | `:190` `MOV dword ptr [EAX + 0x146c],EDX` (EDX=0) | Khởi tạo `+0x146C` |
| 35 | `.scratch/client-pseudo-op-code/opcode_1a.md` | `:187` | Câu "gói này" — đúng phạm vi |
| 36 | `.scratch/client-pseudo-op-code/opcode_1d.md` | §1, §3, §4.4, §8#6 | Đã đặc tả `1D 04` từ trước |
| 37 | `.scratch/client-pseudo-op-code/opcode_23.md` | `:12, 73, 104` | `23 04` — đã đúng |
| 38 | `.scratch/client-pseudo-op-code/opcode_1b.md` | `:122` | **Cần sửa** (xem §5 #5) |

---

## Bản đồ độ tin cậy

* **Mức A (đọc byte/asm thật):** tất cả offset ghi tiền (`0x12F8/0x12FC/0x1300`), `0x146C`, layout C→S `1B 01`/`1B 02`/`1D`, stride 5 + Word LE của TSurface trong eve, offset giá `0x58`/`0x5C` trong `Item.Dat`, nguồn dữ liệu hai form shop, hành vi `spawn.rs`.
* **Mức B (suy luận có cơ sở):** nhãn VISCII của `0x7982B8/2C8/2E4/2F8` cho `1D 01/02` (đã có sẵn từ `opcode_1d.md` §5, tôi chỉ đối chiếu lại vị trí hằng), ý nghĩa chính xác `+0x146C`; offset thứ 2 của double trong `23 04` (do thiếu `.asm.txt`).
* **Mức C (chưa đủ bằng chứng):** nội dung text của `DAT_0079889c` (banner `23 04`), `DAT_005d75d0` (toast giá = 0), `0x5D5488`/`0x5D549C` (con trỏ hàm của TThingSaleForm), tên file cụ thể mà `FUN_005efc24` ghép (`root + "\Data\" + <string từ FUN_00409ef0>` — chưa dump được chuỗi, nên **chưa chứng minh tên là `Eve.emg` chữ-cách-chữ**, chỉ chứng minh là *một file `.dat/.emg` theo map trong `Data\`*; tên `Eve.emg` đến từ C# `EveData.cs` + doc `01_shop_read_flow.md`).

### Sai lệch cần lưu ý về chính các claim

1. **Claim 1 phần (b)**: prompt yêu cầu đọc `TThingSaleForm` như form shop — nhưng đó là form **BÁN**; form **MUA** là `TInterForm`. Cả hai đều không nhận catalogue từ server, nên **kết luận claim vẫn đúng**, nhưng lập luận phải dùng `005eb530` + `TSurface`, không phải `005dd000` + `player+0x9E0`.
2. **Claim 1 nói "strides 5 byte/mục, itemId = Word LE"** — đúng cho **TSurface trong eve** (`005efee8:487` + `005eb530:1090`) và đúng cho **C#** (`EveData.cs:1033`), nhưng **frame `17 1E`** dùng cùng stride 5 với `itemId` là **DWORD 4 byte** (`00730f4c:145-146` → `FUN_0077ef7c`). Hai thứ khác nhau, đừng lẫn.
3. **Claim 2 nói "`opcode_1a.md` nói `+0x12FC` KHÔNG bị gói nào ghi"** — đây là **misquote**. `opcode_1a.md:187` ghi "gói này" (= `1A 04`) và điều đó đúng. `opcode_1d.md` đã đặc tả `1D 04` từ trước. ⇒ **Không có tài liệu nào cần đính chính ở claim 2.**
4. **Claim 3 nói "chỉ ghi `player+0x146C` + banner 10 giây"** — thiếu side-effect thứ ba: **ghi 1 dòng vào chat-log kênh 0** (`case_031…c:189`). Không ảnh hưởng verdict.
