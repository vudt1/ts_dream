# 01 — Luồng "đọc dữ liệu shop" (mở shop NPC → client nhận danh sách hàng)

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

> **Phần giao dịch (layout C→S `1B 01`/`1B 02`, frame `1B 03`, 12 thông báo lỗi VISCII, frame cập
> nhật túi `17 06`/`17 09` + tiền `1A 01`/`1A 04`) xem [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md).**
> Offset tiền xem [`06_currency_model.md`](./06_currency_model.md).

Ngày: 2026-10-01 · Nguồn chính: `client_pseudo_c/` (Ghidra pseudo-C + ASM + HEX của `aLogin.exe`)
Đối chiếu: `TS_Server_Bear/TS_Server/` (server C# tham chiếu), `src/` (server Rust hiện tại)

> Quy ước: `P[i]` = byte thứ i của payload (`P[0]` = MainOp). `RP[i]` = byte thứ i của
> RestPayload (`RP[i] = P[i+1]`, `RP[0]` = SubOp). Frame = `F4 44 | Len:Word LE | payload`,
> toàn frame XOR `0xAD`. Delphi `_LStrCopy(s,i,cnt)` là **1-BASED**.
> Mức bằng chứng: **(A)** đọc body/ASM thật · **(B)** suy luận có cơ sở · **(C)** contract Rust / suy đoán.

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump)**

---

## 0.1 ROUND-4

| Mục | Kết quả |
|---|---|
| §335 `17 06` — "chưa có đối chiếu client" (mức **C**) | ✅ **ĐÃ CHỐT byte-exact, mức A** — xem `battle/04_battle_end_rewards.md §0.1.2`. Layout **14 B**: `RP[1]=slot`, `RP[2..3]=id`, `RP[10..13]=texp` |
| §368 handler `17 21` = `func_0x00610290` — "không có file `.c`/`.asm.txt`" | ✅ **ĐÃ ĐÓNG** — `00610290_FUN_00610290.{c,asm.txt}` tồn tại. Đọc `_LStrCopy(_, 2, 4, _)` ⇒ `RP[2..5]` |
| §262-263 `PTR_DAT_007d9d60` / `PTR_DAT_007da7c4` | ✅ `GVAR` directive đã chạy — đọc `round3_report.txt` |
| §265-266 `gvar_007DA2F8` / `gvar_007DA4F8` | ✅ `GVAR` đã chạy |

⚠️ **Hệ quả cho `shops.rs:596-608`** (`purchase_item` gửi `17 06`): builder hiện tại
sai layout — xem `battle/04 §0.1.3`.

## 0. KẾT LUẬN QUAN TRỌNG NHẤT (đọc trước mọi thứ khác)

> ### Không tồn tại frame S→C nào mang DANH SÁCH HÀNG của shop NPC.
> ### Danh sách hàng shop NPC nằm **100% trong file dữ liệu cục bộ `Data/eve.emg` của chính client**.

Server chỉ gửi **một frame 2-byte** để bảo client **mở form shop**, rồi client tự đọc
`eve.emg` để dựng danh sách. Client gửi lại **chỉ số hàng + số lượng** (không gửi item id,
vì client đã tự biết).

Đây là hệ quả trực tiếp của bằng chứng sau, mỗi mục đều có `file:line` ở §1–§3.

Hệ quả thực tế cho server Rust: **bảng `get_npc_shop_price()` hard-code 100+ dòng trong
`src/server/handlers/shops.rs:51-160` là thừa và sai kiến trúc** — client không cần
server báo giá, giá đến từ `Item.dat` (cùng file client đọc). Server chỉ cần **validate**
index↔shopid theo cùng bảng `eve.emg`.

---

## 1. Câu 1 — Client gửi gì C→S khi thao tác với shop NPC?

### 1.1 ĐÍNH CHÍNH: `case 0x1b: break;` trong pseudo-C là LỖI RECOVERY CỦA GHIDRA

Tài liệu `opcode_1b.md:122` kết luận "client không bao giờ gửi OP 0x1B" dựa trên
`0077f414_FUN_0077f414.c:985-986`:

```c
case 0x1b:
  break;
```

**Kết luận đó SAI.** Bảng nhảy thật của builder C→S nằm ngay trong thân hàm:

- `client_pseudo_c/0077f414_FUN_0077f414.asm.txt:30-35`
  ```
  0077f457: XOR EAX,EAX
  0077f459: MOV AL,byte ptr [EBP + -0x5]     ; AL = opcode (DL) 
  0077f45c: CMP EAX,0xc7
  0077f461: JA  0x0078a4f2
  0077f467: MOV AL,byte ptr [EAX + 0x77f474] ; bảng byte 200 phần tử
  0077f46d: JMP dword ptr [EAX*0x4 + 0x77f53c] ; bảng dword 200 phần tử
  ```
- Đối chiếu 2 bảng hex (mỗi phần tử 4 byte LE) cho kết quả:
  `op 0x17 → idx 21 → 0x007833F0` · `op 0x1A → idx 24 → 0x00785B4C` ·
  **`op 0x1B → idx 26 → 0x00785FFD`** · `op 0x1D → idx 27 → 0x00786185` ·
  `op 0x1E → idx 28 → 0x0078620C` · `op 0x23 → idx 33 → 0x00786CD6`

→ **Target thật của OP `0x1B` trong builder C→S là `0x00785FFD`**, không phải `break`.
Đây là bằng chứng (A). (Việc `case N: break;` rỗng là artefact Ghidra đã được cảnh báo
ở `post_login_stats_inventory_flow.md` §7 —nay áp dụng cho đúng `0x1B`.)

### 1.2 Body thật của builder `0x1B` — MUA (`sub=1`) và BÁN (`sub=2`)

`client_pseudo_c/0077f414_FUN_0077f414.asm.txt:1815-1868` (nhánh `sub == 1`) và
`:1869-1917` (nhánh `sub == 2`):

```asm
00785ffd: MOV AL,byte ptr [EBP + -0x6]   ; sub-op (đối số thứ 3, CL)
00786000: DEC AL
00786002: JZ  0x00786011                  ; sub == 1  → MUA
00786004: DEC AL
00786006: JZ  0x007860c8                  ; sub == 2  → BÁN
0078600c: JMP 0x0078a4f2                  ; sub khác  → no-op
```

Khuôn dựng payload (nhánh MUA, `:1821-1867`) — **3 đoạn nối shortstring**:

```asm
; (1) shortstring{len=1, ch=opcode}
00786014: MOV DL,byte ptr [EBP - 0x5]     ; DL = MainOp = 0x1B
00786017: MOV byte ptr [EAX+0x1],DL
0078601a: MOV byte ptr [EAX],0x1
00786023: CALL 0x00402b90                 ; @LStrLAsg  → dest
; (2) shortstring{len=1, ch=sub}
0078602b: MOV DL,byte ptr [EBP - 0x6]     ; DL = sub
00786031: MOV byte ptr [EAX+0x1],DL
0078603a: CALL 0x00402b60                 ; @PStrNCat(CL=2) → 2 byte: [1B][sub]
; (3) shortstring{len=1, ch=**gvar_007DA5A8 + 0x152}
0078604f: MOV EDX,dword ptr [0x007da5a8]  ; TInterForm
00786057: MOV DL,byte ptr [EDX + 0x152]   ; ← chỉ số hàng (1 byte)
0078606b: CALL 0x00402b60                 ; @PStrNCat(CL=3) → 3 byte
; (4) Word LE
00786092: MOV DX,word ptr [EDX + 0x154]   ; ← số lượng (word)
007860a0: CALL 0x0077eb1c                 ; Word → 2 byte
007860be: CALL 0x0051633c                 ; TForm1.CY_AddSedQueue  ← GỬI
```

Byte order chính xác của `@PStrNCat` đã đọc body tại
`client_pseudo_c/00402b60__PStrNCat.c:584-606` + `.asm.txt:4-25`:
nối **dest-data ← src-data** (dest là shortstring, byte `[0]` = độ dài), tức nối **đuôi**.
Vì `dest` được gán từ đoạn (1) trước, thứ tự là **`[0x1B][sub][idx][w_lo][w_hi]`**.

### 1.3 Wire layout C→S shop NPC (byte-exact)

```
Offset  Kích thước  Ý nghĩa                       Nguồn            Mức
------  ---------  -----------------------------  ---------------  ---
P[0]         1 B   MainOp = 0x1B                  asm:1815-1835    (A)
P[1]         1 B   SubOp: 1=MUA, 2=BÁN            asm:1816-1820    (A)
P[2]         1 B   index hàng (chọn trên form)    asm:1842/1857    (A)
P[3..4]      2 B   số lượng — Word LE             asm:1857 +       (A)
                                  FUN_0077eb1c.c
Tổng: 5 byte payload.
```

Đối chiếu chéo với server C# `TS_Server_Bear/TS_Server/PacketHandlers/NpcShopsHandler.cs:13-15`
```csharp
byte index = data[2];   // ← P[2]  ✔ khớp
byte count = data[3];   // ← P[3] = byte thấp của Word ✔ khớp (byte cao bị server bỏ)
switch (data[1])        // ← P[1]  ✔ khớp: case 1 = mua, case 2 = bán
```
`PacketProcessor.cs:120-121` xác nhận `case 27:` (27 = 0x1B) → `NpcShopsHandler`.

### 1.4 Client KHÔNG gửi gì để "mở form shop"

Builder `0x1B` chỉ có **2** nhánh (`sub=1`, `sub=2`), tất cả sub khác rơi thẳng vào
`JMP 0x0078a4f2` (exit, `asm:1820`). Không có `sub=3` hay bất kỳ sub nào mang
"shopid"/"mở cửa hàng". ⇒ **Việc mở form shop do server ra lệnh, không phải client yêu cầu.**
(Ngược lại với giả định ban đầu trong đề bài.)

### 1.5 Danh sách hàng shop NPC đến từ đâu — `eve.emg`

Bằng chứng server C# **tự parse cùng một file** `eve.emg` để lấy danh sách hàng
(`TS_Server_Bear/TS_Server/DataTools/EveData.cs:1001-1044`):

```csharp
ushort num38 = read16(array4, num14);          // số shop trong scene
for (int num39 = 0; num39 < num38; num39++) {
    num14 += 4;
    byte b6 = array4[num14];                  // số mục hàng
    num14 += 3;  num14++;
    num14 += 5 * b6;                          // ← stride 5 byte / mục hàng
}
```
và bên trong mỗi mục: `item7.itemId = read16(array4, num14 + num41); num41 += 5;`
(`EveData.cs:1029-1032`) ⇒ **record hàng = 5 byte, item id là Word LE ở offset 0**.

Repo Rust đã có loader cùng cấu trúc section: `src/data/loaders/eve.rs:486-496`
(`skip_goods_section`, `count*13` — section 2 *GoodsData*, khác section shop;
`src/data/loaders/eve.rs:371-379` liệt kê 9 section của 1 scene).

⇒ **Cả client và server C# đều đọc `eve.emg` cục bộ; không ai gửi danh sách hàng shop NPC
qua mạng.** Mức (A) cho hành vi, (B) cho kết luận "không có frame danh sách".

---

## 2. Câu 2 — Frame S→C nào mang danh sách hàng shop?

### 2.1 Kiểm chứng từng giả thuyết

| Giả thuyết | Kết quả kiểm chứng | Mức |
|---|---|---|
| Main `0x1B` (Case 24) đẩy danh sách hàng | **SAI.** `case_024_00791D80_FUN_00791d80.c:32-108` chỉ có 3 nhánh: Outer `1`/`2` = gán literal + **banner 2000 ms** (`:65-66`, `:102-103`), Outer `3` = **`(**gvar_007DA720 + 0x20)()`** (`:107`). Không có vòng lặp, không đọc record, không có `_LStrCopy`. | (A) |
| Main `0x17` sub `0x30`–`0x33` là player shop | **ĐÚNG, nhưng là shop NGƯỜI CHƠI, không phải NPC.** `case_020_007902FB_FUN_007902fb.c:221-232` → `func_0x007699d4` / `func_0x0076aecc` / `FUN_0062f390` / `func_0x0076ad20`. Định danh form: `0078a89c_FUN_0078a89c.c:4266` (`gvar_007DA0A4`) và `FUN_00730f4c.c:164` nối chuỗi **`"Stall"`**. | (A) |
| Main `0x0C` (Case 12) | **SAI.** `case_012_0078D84D_FUN_0078d84d.c` không có `switch` nào; parse struct cố định 12 byte = đồng bộ vị trí actor (`opcode_0c.md:11-15`). | (A) |
| Main `0x17` sub `0x21` | **ĐÚNG — đây là frame danh sách hàng shop NGƯỜI CHƠI.** `case_020…c:157-158` → `func_0x00610290(gvar_007DA2F8, RP)`. Định danh form: `test/ts_decompile/functions/0051189c_FUN_0051189c.c:1366` → `VMT_60868C_TCY_BuyVenderMenu`. | (A) |

### 2.2 Frame S→C thật sự mở form shop NPC: `1B 03`

| Chiều | Payload | Ý nghĩa | Nguồn | Mức |
|---|---|---|---|---|
| S→C | `1B 03` (2 byte) | **MỞ FORM SHOP NPC** — `(**gvar_007DA720 + 0x20)()`, `gvar_007DA720` = **`TThingSaleForm`** (`test/ts_decompile/functions/0051189c_FUN_0051189c.c:1445-1446`, `VMT_5D5438_TThingSaleForm`) | `case_024_00791D80_FUN_00791d80.c:106-107` | **(A)** |

Đối chiếu **hoàn hảo** với server C#:
`TS_Server_Bear/TS_Server/Client/QuestStepHelper/HotelShopAndSaveMapEffectHandler.cs:13-14`
```csharp
case 3:
    client.reply(new PacketCreator(27, 3).send());   // 27 = 0x1B, payload = 2 byte
```
`PacketCreator(byte, byte)` đặt `data[0]=opcode, data[1]=sub, pos=2`
(`PacketCreator.cs:41-45`) ⇒ payload đúng **2 byte `1B 03`**. Khớp 3 nguồn độc lập.

### 2.3 Form shop NPC tự dựng danh sách từ dữ liệu cục bộ

`client_pseudo_c/0050705c_FUN_0050705c.c` là hàm render của `TThingSaleForm`:

| Dòng | Nội dung | Mức |
|---|---|---|
| `:88-92` | `uVar5 = local_10>>16 & 0xFF; if (0x10 < uVar5) _BoundErr;` → **16 ô hàng** (`0x00..0x0F`); đọc `*(int *)(local_8 + 0x150 + uVar5*4) + 0xe4` | (A) |
| `:93-94` | `FUN_00774a84(gvar_007DA540, itemId, &out)` → **tra tên vật phẩm** trong bảng item của client; ghi vào `form+0x1B0` | (A) |
| `:131` | `IntToStr(*(uint *)(... + 0x108))` → **hiển thị giá** từ field `+0x108`; nối vào `form+0x328` | (A) |
| `:107-133` | Nhánh fallback khi `form+0x1B0 == 0`: lấy `*(... + 0x108)` từ mảng NPC `gvar_007DA32C + 0xE8 + i*4` (i ≤ 0x19) | (A) |
| `:168-213` | Hiện/ẩn 3 nhóm control theo `form+0x1CE` / `form+0x1CF` (cột chọn) | (A) |

⇒ **record hàng shop NPC nằm trong object nội bộ form, không nằm trên wire.**
`itemId` ở `+0xE4`, `giá` ở `+0x108` — offset trong object, không phải offset wire.

### 2.4 Layout record hàng shop NGƯỜI CHƠI (`17 21`) — byte-exact

Nguồn: `TS_Server_Bear/TS_Server/PacketHandlers/ShopHandler.cs`, hàm `refreshShop`
(dòng 108-140). Client chỉ nhận về `func_0x00610290` (thân hàm **không có** trong
`client_pseudo_c/` → xem §7 CHƯA XÁC MINH).

```
Offset  Size  Field                Ghi chú                          Mức
------  ----  --------------------  -------------------------------  ----
P[0]      1   MainOp = 0x17        PacketCreator(23, 33)             (B)
P[1]      1   SubOp = 0x21         PacketCreator(23, 33)             (B)
P[2..18] 17   zero                 addZero(17)                       (B)
--- record lặp, stride = 16 byte, N = (len - 19) / 16 ---
+0       2   itemId   Word LE     add16(tSItem.Itemid)              (B)
+2       2   quantity Word LE     add16(quantity)                   (B)
+4       4   price    DWORD LE    add32(b)                          (B)
+8       1   0                     addByte(0)                        (B)
+9       1   100                   addByte(100)                      (B)
+10      1   0                     addByte(0)                        (B)
+11      4   0                     add32(0u)                         (B)
+15      1   index (1-based)       addByte(num++)                    (B)
```

`N = (len − 19) / 16` với `len` = độ dài payload.

---

## 3. Câu 3 — Layout byte-exact của record hàng shop

### 3.1 Shop NPC — **KHÔNG có record trên wire**

| Câu hỏi | Trả lời | Mức |
|---|---|---|
| Item id u16 hay u32 trên wire? | **Không có trên wire.** Trong `eve.emg` là `Word LE` (`EveData.cs:1029`), trong form là field `+0xE4` u32 | (A) |
| Giá 1 hay 2 trường (mua + bán)? | **Không có trên wire.** Giá hiển thị đến từ `Item.dat` field `price` / `sell_price` (`src/data/loaders/item.rs:34-35, 226-229`; C# `ItemInfo.cs:51,53` `BuyingPrice`/`SellingPrice`). Server C# dùng `value2.BuyingPrice` khi mua (`NpcShopsHandler.cs:25`) và `value.SellingPrice` khi bán (`:54`). | (A) |
| Số lượng tối đa mỗi lần mua? | **Không có trường max trên wire.** Client tự tính từ `Item.dat`/UI; server chỉ nhận `count` u16 từ client và **tự kiểm** (`NpcShopsHandler.cs:26` `gold >= count*BuyingPrice`) | (A) |
| Loại tiền (kim/xu/điểm/hpoint)? | **Chỉ 1 loại: vàng (kim).** Cả buy và sell đều đụng `client.getChar().gold` (`NpcShopsHandler.cs:26,28,55`). Không có trường loại tiền trên wire. | (A) |
| Có tên hiển thị không? | **Không trên wire.** Tên do client tra bảng `gvar_007DA540` qua `FUN_00774a84` (`0050705c_FUN_0050705c.c:93,98,114,124`) | (A) |
| Có icon/index không? | Icon: client tự tra theo `itemId`. Index: `form+0x1CE/0x1CF` là **cột đang chọn**, không phải field wire. | (A) |

### 3.2 Shop người chơi (`17 21`) — có record trên wire

Xem bảng §2.4. Tổng hợp: **1 item id (u16) + 1 số lượng (u16) + 1 giá (u32) + 6 byte
metadata/đệm + 1 index**, stride **16 byte**. Không có tên, không có loại tiền (luôn là
vàng), không có trường "mua/bán".

---

## 4. Câu 4 — Phân biệt shop NPC và shop người chơi

**CÓ — 2 họ form UI hoàn toàn khác nhau.** Định danh từ tên VMT trong
`test/ts_decompile/functions/0051189c_FUN_0051189c.c` (mỗi form được `FUN_005b075c`
đăng ký vào `gvar_007DA234`):

| Global | Tên class Delphi (VMT) | Dòng | Vai trò |
|---|---|---|---|
| `gvar_007DA720` | **`TThingSaleForm`** | `:1445-1446` | **Form shop NPC** (mở bởi `1B 03`) |
| `PTR_DAT_007d9d60` | **`TThingSaleQForm`** | `:1448` | Dialog hỏi số lượng khi **bán** |
| `PTR_DAT_007da7c4` | **`TThingBuyQForm`** | `:1503` | Dialog hỏi số lượng khi **mua** |
| `gvar_007DA5A8` | **`TInterForm`** | `:1506-1507` | Form giao dịch; `+0x152` = index hàng, `+0x154` = số lượng mà builder `1B/1` đọc |
| `gvar_007DA2F8` | **`TCY_BuyVenderMenu`** | `:1366-1367` | **Form xem danh sách shop người chơi** (nhận `17 21`) |
| `gvar_007DA4F8` | **`TCY_VenderMenu`** | `:1363-1364` | Form bảng "các gian hàng" (`17 1E`); nối chuỗi `"Stall"` tại `00730f4c.c:164,166` |
| `gvar_007DA0A4` | **`TThingAccountForm`** | `:1492-1493` | Form túi đồ (`17 20` đóng shop → refresh) |
| `gvar_007DA718` | **`TCY_WareMenu`** | `:1354-1355` | Form kho (`17 2E` → `case_020…c:216`) |

Bằng chứng bổ sung về tính chất 2 loại shop:

- **Shop NPC**: dữ liệu tĩnh trong `eve.emg`, không đổi theo người chơi, không có tên
  người bán; form không có trường "tên cửa hàng".
- **Shop người chơi**: `ShopHandler.cs:15-18` đọc `myShopName` + `myShopImage` từ client,
  `ShopHandler.cs:139-140` bảng "các gian hàng" gắn nhãn `"Stall" + <id>`; frame
  `17 1F` mang `accID` (DWORD) để client biết chủ shop.

---

## 5. Câu 5 — Rust hiện tại làm gì, sai/thiếu gì

### 5.1 Shop NPC — `src/server/handlers/shops.rs:166-265` (`handle_npc_shop`)

| # | Hạng mục | Đánh giá | Chi tiết |
|---|---|---|---|
| 1 | **Gửi frame mở form shop `1B 03`** | **LỆCH — THIẾU NGHIÊM TRỌNG** | `timeout 30 grep -rn '1B03\|1b03' src/` → **0 kết quả**. Server Rust **không bao giờ** mở form shop NPC ⇒ người chơi không thể mở cửa hàng. Đây là gap lớn nhất. |
| 2 | **Phân biệt MUA (`sub=1`) vs BÁN (`sub=2`)** | **LỆCH** | `shops.rs:172-173` đọc `menu = payload.first()`, `count = payload.get(1)` và **bỏ qua `ctx.sub`**. Mọi request (kể cả `1B 02` = bán) đều đi vào nhánh mua ở `shops.rs:238`. |
| 3 | **Bảng giá hard-code 100+ dòng** | **LỆCH về kiến trúc** | `shops.rs:51-160` `get_npc_shop_price(idtalking, map_id, menu)`. Client tự đọc giá từ `Item.dat`; server chỉ cần **validate index**. Bảng này (a) sai về nguồn dữ liệu, (b) `menu` ở Rust là `index` từ client — tên biến gây hiểu nhầm. |
| 4 | **Giá bán = 1 vàng / món** | **LỆCH — SAI SỐ** | `shops.rs:42`: `conn.session.gold = ...saturating_add(u32::from(count))` — cộng đúng `count` vàng, bỏ qua `SellingPrice` (`NpcShopsHandler.cs:54` dùng `count * value.SellingPrice`). |
| 5 | **Bán chỉ cho 2 NPC cứng** | **LỆCH** | `shops.rs:179-185`: `idnpctalking == 16005 \|\| 16002 \|\| 99999` + khoảng item id cứng. C# áp giá cho **mọi** shop có `b4 > 0` trong `eve.emg`. |
| 6 | **Frame kết quả `red_message` dùng opcode `0x02 0x0B`** | **LỆCH (diện rộng)** | `shops.rs:18-22` dựng `02 0B 00000000 <text>`. Nhưng `case_002_0078B149_FUN_0078b149.c:188-193` cho thấy `0x02/0x0B` **mở 1 form khác** (`(**gvar_007D9F58 + 0x20)()`), **không** in text. Text shop đúng phải qua `1B 01/02 + inner` (banner 2000 ms, `case_024…c:65-66`). |
| 7 | `gold_frame` = `1A 04 <gold:4> <0:4>` | **KHỚP** | `shops.rs:11-13` vs C# `TSCharacter.cs:5301-5303` `PacketCreator(26,4) + add32(gold) + add32(goldBank)`. Cả hai 10 byte payload. (Bank luôn 0 ở Rust — chấp nhận được nếu bank chưa làm.) |
| 8 | Bảng `get_npc_shop_price` trả `None` khi trượt | **LỆCH (im lặng)** | `shops.rs:157` `return None` ⇒ client bấm mua mà server im → không có phản hồi nào. C# `NpcShopsHandler.cs:35` gửi `announce(...)` khi item không tồn tại. |

### 5.2 Shop người chơi — `src/server/handlers/shops.rs:269-609`

| # | Hạng mục | Đánh giá | Chi tiết |
|---|---|---|---|
| 9 | `player_shop_catalog_frame` = `1721` + 17 zero + N×16 | **KHỚP** | `shops.rs:270` `"0".repeat(34)` = 17 byte zero ✔; record `shops.rs:279-289` = `le16 id` + `le16 count` + `le32 price` + `1B long_val` + `1B giatri` + `1B khang` + `le32 texp` + `1B idx` = **16 byte** ✔ khớp `refreshShop`. |
| 10 | `171E` (self) / `171F` (broadcast open) | **KHỚP** | `shops.rs:482,484-492` vs `ShopHandler.cs:29-38` (`PacketCreator(23,30)` self, `PacketCreator(23,31)` + accID broadcast). |
| 11 | `1720` (đóng shop) | **KHỚP** | `shops.rs:500` `F4440600 1720 <accID:4>` vs `ShopHandler.cs:57-59` `PacketCreator(23,32) + add32(accID)`. |
| 12 | Parse sub `0x20` (view shop) | **KHỚP** | `shops.rs:507` đọc `u32` ở `payload[0..3]` = `P[2..5]` ✔ vs `ShopHandler.cs:51` `read32(data,2)`. |
| 13 | Parse sub `0x21` (buy) | **KHỚP** | `shops.rs:533-534` `payload[4]`/`payload[5]` = `P[6]`/`P[7]` ✔ vs `ShopHandler.cs:63-64` `data[6]`/`data[7]`. |
| 14 | Parse sub `0x1E` (mở shop) | **KHỚP** | `shops.rs:424-435` `name_len=payload[0]`, name `payload[1..]`, image `payload[name_len+1]`, listing stride 5 ✔ vs `ShopHandler.cs:13-30`. |
| 15 | Frame `1706` (gửi item vừa mua cho buyer) | **CHƯA BIẾT** | `shops.rs:598-608`. Không có trong `ShopHandler.cs`; C# dùng `transferItem` (`ShopHandler.cs:79`). Chưa đối chiếu được với client. |
| 16 | Giá bán trần `9_999_999` | **KHỚP** | `shops.rs:366` ✔ vs `opcode_1a.md:93` "clamp trần 9 999 999" trên `player+0x12F8`. |

### 5.3 Gap lớn nhất (xếp theo mức nghiêm trọng)

1. **Thiếu frame `1B 03`** → shop NPC không mở được (chặn toàn bộ tính năng).
2. **Không phân biệt `sub=1` (mua) / `sub=2` (bán)** → bán hàng bị xử như mua, gây mất tiền/sai vật phẩm.
3. **Giá bán sai số hệ thống** (`+1 vàng/món` thay vì `SellingPrice`).
4. **Bảng giá mua hard-code** thay vì validate theo `eve.emg` như C#.
5. **Kết quả giao dịch dùng sai opcode** (`02 0B` thay vì `1B 01/02`).

---

## 6. Câu 6 — Bảng tổng hợp frame

| Frame | Chiều | Payload (byte-exact) | Ý nghĩa | Nguồn | Mức |
|---|---|---|---|---|---|
| `1B 03` | S→C | `1B 03` (2 B) | **Mở form shop NPC** (`TThingSaleForm`) | `case_024…c:106-107`; `HotelShopAndSaveMapEffectHandler.cs:14` | **A** |
| `1B 01 <i> <c:2>` | C→S | `1B 01 i c_lo c_hi` (5 B) | **MUA** — index `i`, số lượng `c` (Word LE) | `0077f414.asm.txt:1815-1867`; `NpcShopsHandler.cs:13-17,26-30` | **A** |
| `1B 02 <i> <c:2>` | C→S | `1B 02 i c_lo c_hi` (5 B) | **BÁN** — index `i`, số lượng `c` (Word LE) | `0077f414.asm.txt:1869-1917`; `NpcShopsHandler.cs:44-57` | **A** |
| `1B 01 <inner>` | S→C | `1B 01 inner` (3 B) | **Banner kết quả 2000 ms** — nhóm lỗi/thành công 1 | `case_024…c:32-68` | **A** |
| `1B 02 <inner>` | S→C | `1B 02 inner` (3 B) | **Banner kết quả 2000 ms** — nhóm lỗi/thành công 2 | `case_024…c:69-105` | **A** |
| — | — | — | *(không tồn tại)* frame danh sách hàng **shop NPC** | `case_024…c` không có vòng lặp/record; `EveData.cs:1001-1044` cho thấy server C# tự parse `eve.emg` | **A** |
| `17 1E` | C→S | `17 1E nameLen name[] image (slot:1 price:4)*` | Mở gian hàng của tôi | `cs_op17_targets` sub 30 → `0x007840A7`; `ShopHandler.cs:11-30` | **A** (dispatch) / **B** (layout) |
| `17 1F` | S→C | `17 1F accID:4 nameLen name[] image` | Broadcast mở gian hàng | `ShopHandler.cs:35-39` | **B** |
| `17 20` | S→C | `17 20 accID:4` | Broadcast đóng gian hàng | `ShopHandler.cs:56-60` | **B** |
| `17 21` | S→C | `17 21 zero[17] + N×16` | **Danh sách hàng shop người chơi** | `case_020…c:157-158` → `func_0x00610290`; `ShopHandler.cs:108-140` | **A** (dispatch) / **B** (layout) |
| `17 20` | C→S | `17 20 sellerAccID:4` | Mở xem shop người chơi | `cs_op17_targets` sub 32 → `0x007841E1`; `ShopHandler.cs:50-56` | **A**/**B** |
| `17 21` | C→S | `17 21 ?(4) index:1 count:1` | Mua từ shop người chơi | `cs_op17_targets` sub 33 → `0x00784261`; `ShopHandler.cs:63-64` | **A**/**B** |
| `1A 04 <g:4> <b:4>` | S→C | `1A 04 gold:4 goldBank:4` (10 B) | Đồng bộ số dư | `opcode_1a.md:96,181-191`; `TSCharacter.cs:5299-5305` | **A** |
| `17 06` | S→C | `17 06 id:2 cnt:2 00 00 <1> <1> <1> texp:4` (15 B) | Gửi item vừa mua cho buyer | `shops.rs:598-608` (Rust) — **chưa có đối chiếu client** | **C** |

### 6.1 Bảng offset record `17 21` (chi tiết)

| Offset | Size | Kiểu | Tên | Ghi chú |
|---|---|---|---|---|
| 0 | 1 | u8 | MainOp | `0x17` |
| 1 | 1 | u8 | SubOp | `0x21` |
| 2 | 17 | — | zero | `addZero(17)` |
| 19 + 16·i + 0 | 2 | u16 LE | `itemId` | |
| 19 + 16·i + 2 | 2 | u16 LE | `quantity` | |
| 19 + 16·i + 4 | 4 | u32 LE | `price` | vàng |
| 19 + 16·i + 8 | 1 | u8 | `0` | đệm |
| 19 + 16·i + 9 | 1 | u8 | `100` | hằng |
| 19 + 16·i + 10 | 1 | u8 | `0` | đệm |
| 19 + 16·i + 11 | 4 | u32 LE | `0` | đệm |
| 19 + 16·i + 15 | 1 | u8 | `index` | 1-based, tăng dần |

**`N = (len − 19) / 16`**

### 6.2 Bảng offset frame `1B 01/02` (chi tiết)

| Offset | Size | Kiểu | Tên | Nguồn |
|---|---|---|---|---|
| 0 | 1 | u8 | MainOp `0x1B` | `asm:1815-1835` |
| 1 | 1 | u8 | SubOp `1`=mua / `2`=bán | `asm:1816-1820` |
| 2 | 1 | u8 | `index` hàng | `asm:1842` / `asm:1857` |
| 3 | 2 | u16 LE | `count` | `asm:1857` + `FUN_0077eb1c` |

---

## 7. CHƯA XÁC MINH

1. **Thân `func_0x00610290` (handler `17 21`)** — không có file `.c`/`.asm.txt` trong
   `client_pseudo_c/` lẫn `test/ts_decompile/functions/`. ⇒ **layout record `17 21` mới
   chỉ dựa trên server C#, chưa được xác nhận từ phía client.**
   *Hướng tìm:* disassemble `aLogin.exe` tại `VA 0x00610290` (dùng cho `gvar_007DA2F8`
   = `TCY_BuyVenderMenu`); grep `FUN_0077eb9c` / `FUN_0077ef7c` trong thân để xác nhận
   có đúng 2 lần Word-LE + 1 lần DWORD-LE mỗi record.

2. **Builder C→S `0x17` sub 30-33** (`0x007840A7`, `0x0078418C`, `0x007841E1`, `0x00784261`)
   — **ASM dump `0077f414_FUN_0077f414.asm.txt` bị THIẾU đoạn `0x00784000-0x00785000`**
   (đã kiểm: `awk '{print substr($1,1,6)}' … | grep '^00784'` → rỗng). ⇒ byte order trong
   frame `17 1E` / `17 21` (C→S) hiện chỉ suy từ server C#.
   *Hướng tìm:* disassemble lại `FUN_0077f414` trong khoảng `0x007840A7..0x0078434F`.

3. **Ai ghi `TInterForm+0x152` / `+0x154` và `TThingSaleForm+0x160` / `+0x162`?** — đây là
   2 field mà builder `1B/1` và `1B/2` đọc để gửi `index` + `count`. Đã grep
   `test/ts_decompile/functions/*.c` với pattern `gvar_007DA5A8 + 0x152|0x154` và
   `gvar_007DA720 + 0x160|0x162` → **0 kết quả**; các offset tìm được (`005eb530…c:832-850`)
   là offset của **control con** (`*(…gvar_007DA5A8 + 0x124) + 0x152`), không phải offset
   của form. ⇒ **Chưa xác minh `index` là 0-based hay 1-based, và `count` là số mua hay số
   còn lại.** Đối chiếu sơ bộ: C# dùng `inventory.items[index-1]` cho bán
   (`NpcShopsHandler.cs:46`) ⇒ **nghiêng về 1-based**, nhưng đây là suy luận (B), không phải
   bằng chứng client.
   *Hướng tìm:* tìm hàm gọi `FUN_005d6048` (constructor `TInterForm`) / `FUN_005dc500`
   (constructor `TThingSaleForm`) và các hàm `Button*Click` ghi vào offset này; hoặc
   disassemble `0x005d6048`–`0x005dd800`.

4. ~~**Ý nghĩa chính xác 12 literal của `0x1B 01/02`**~~ — **ĐÃ GIẢI QUYẾT** ✅
   Bảng đầy đủ (12 dòng, đã verify từng hằng `case_024…c:40-57, 76-94` và decode bằng bảng
   VISCII `src/encoding.rs:126-236`) nằm ở
   [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) §7.
   **⚠ Kết quả đảo ngược giả định cũ:** `Inner = 0` ⇔ `0x798118` = **"Giao dịch thành công"**
   (toast **không** hiện vì điều kiện là `Inner != 0`); `"Xin lỗi, vàng của bạn không đủ!"` nằm ở
   `0x798138` ⇔ `Inner = 1`; `"…bảng vật phẩm của bạn đã đầy!"` ở `0x798160` ⇔ `Inner = 2`.

5. **Section shop trong `eve.emg` phía Rust** — `src/data/loaders/eve.rs:486-496` hiện
   **skip** section 2 (GoodsData, `count*13`) và **không có** parser cho section shop mà
   `EveData.cs:1001-1044` đọc. Vị trí chính xác section shop trong 9 section của 1 scene
   **chưa xác định** trong loader Rust.
   *Hướng tìm:* so `num14` của C# với từng `parse_*_section` trong `eve.rs:371-379`; đoán
   section shop nằm ngay **trước** block Warp (C# đọc shop xong rồi mới đọc `num42` warp).

6. **`shopid` của client lấy từ đâu cụ thể** — C# set `client.shopid = (byte)packMode.GetDialogID`
   khi eve script mở dialog (`DialogAndBattleEntryHandler.cs:14`). Client chắc lấy từ
   `eve.emg` cục bộ. **Chưa xác minh** client có gửi `shopid` lên server hay không —
   bằng chứng cho thấy **KHÔNG** (frame `1B 01/02` chỉ 5 byte, không có shopid), nên server
   buộc phải suy ra từ state `idtalking`/`idnpctalking` như C# và như Rust hiện tại.

---

## 8. Source trail

| # | Nguồn | Dùng để |
|---|---|---|
| 1 | `client_pseudo_c/0077f414_FUN_0077f414.asm.txt:30-35, 1815-1917` | Dispatch C→S thật + body builder `0x1B` sub 1/2 — **bác bỏ `case 0x1b: break`** |
| 2 | `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex` + `cs_builder_dword200_0x77f53c.hex` | Ánh xạ MainOp → target builder (200 phần tử, `asm:32` xác nhận `CMP EAX,0xC7`) |
| 3 | `client_pseudo_c/00402b60__PStrNCat.c:584-606` + `.asm.txt:4-25` | Thứ tự byte khi nối shortstring (nối **đuôi**) |
| 4 | `client_pseudo_c/0077eb1c_FUN_0077eb1c.c:5` | `Word → 2 byte` cho trường `count` |
| 5 | `client_pseudo_c/case_024_00791D80_FUN_00791d80.c:32-108` | Handler S→C `0x1B`: 3 nhánh, **không có** frame danh sách hàng |
| 6 | `client_pseudo_c/case_020_007902FB_FUN_007902fb.c:148-158, 215-232` | Handler S→C `0x17`: `0x1E/0x1F/0x20/0x21` (stall) và `0x30..0x33` (player shop) |
| 7 | `client_pseudo_c/0050705c_FUN_0050705c.c:88-133, 168-213` | Form `TThingSaleForm`: 16 ô, tên từ `FUN_00774a84`, giá ở `+0x108` |
| 8 | `client_pseudo_c/00730f4c_FUN_00730f4c.c:164-166` | Chuỗi `"Stall"` → form bảng gian hàng là shop **người chơi** |
| 9 | `test/ts_decompile/functions/0051189c_FUN_0051189c.c:1354-1367, 1445-1448, 1492-1507` | **Định danh tên class form** (TThingSaleForm / TCY_BuyVenderMenu / TCY_VenderMenu / TInterForm) |
| 10 | `TS_Server_Bear/TS_Server/PacketHandlers/NpcShopsHandler.cs:13-73` | Layout `1B 01/02` + logic mua/bán + nguồn giá |
| 11 | `TS_Server_Bear/TS_Server/Client/QuestStepHelper/HotelShopAndSaveMapEffectHandler.cs:13-14` | **Frame mở shop `PacketCreator(27,3)`** |
| 12 | `TS_Server_Bear/TS_Server/PacketProcessor.cs:120-121` | `case 27` → `NpcShopsHandler` |
| 13 | `TS_Server_Bear/TS_Server/PacketHandlers/ShopHandler.cs:11-140` | Layout `17 1E/1F/20/21` + `refreshShop` record 16 byte |
| 14 | `TS_Server_Bear/TS_Server/PacketCreator.cs:41-45, 55-70` | Constructor 2-opcode + framing `F4 44` + XOR `0xAD` |
| 15 | `TS_Server_Bear/TS_Server/DataTools/EveData.cs:1001-1044, 655-700` | **Shop list đến từ `eve.emg`**, stride 5 byte/mục, `itemId` = Word LE |
| 16 | `TS_Server_Bear/TS_Server/DataTools/ItemInfo.cs:51,53` + `ItemData.cs:101-102` | `BuyingPrice` / `SellingPrice` từ `Item.dat` |
| 17 | `TS_Server_Bear/TS_Server/Client/TSCharacter.cs:5299-5305` | `sendGold` = `1A 04 <gold:4> <bank:4>` |
| 18 | `src/server/handlers/shops.rs:11-13, 51-160, 166-265, 269-293, 412-594` | Đối chiếu Rust |
| 19 | `src/server/dispatcher.rs:194-206, 228` | Quy ước `ctx.sub` = `decoded[5]`, `ctx.payload` = `decoded[6..]` |
| 20 | `src/data/loaders/item.rs:34-35, 226-229` | `Item.dat` có `price` + `sell_price` |
| 21 | `src/encoding.rs:128-236` | Bảng VISCII 1.1 dùng để giải mã literal `lit_*.hex` |
