# 02 — GIAO DỊCH MUA / BÁN CỬA HÀNG NPC (`0x1B`) — open → buy → sell → đồng bộ túi & tiền

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

> **Phần đọc dữ liệu shop & layout `17 21` xem [`01_shop_read_flow.md`](./01_shop_read_flow.md); offset tiền xem [`06_currency_model.md`](./06_currency_model.md).**
>
> Tài liệu này **hợp nhất** hai nguồn trước đó: `01_npc_shop.md` (luồng NPC shop, bảng 12 thông báo VISCII)
> và `01_shop_read_flow.md` (nguồn dữ liệu shop, layout `17 21`). Phần layout C→S `1B 01/02`,
> frame `1B 03`, kênh lỗi `1B 01/02 + inner` là **độc quyền của file này**.

---

## Quy ước

* `P[i]` = byte thứ *i* của **payload sau deframe** (đã bỏ magic `F4 44` + `Len:Word LE`). `P[0] = MainOp`.
* `RP[i] = P[i+1]` = "RestPayload" (Delphi `AnsiString`) mà handler đọc; độ dài ở `*(int *)(RP-4)`.
* `_LStrCopy(RP, Index, Count, Dst)` là **1-BASED** → byte 0-based tương ứng là `RP[Index-1 .. Index+Count-2]`.
* Codec client: `FUN_0077eb9c` = **Word LE decode** · `FUN_0077ef7c` = **DWORD LE decode** ·
  `FUN_0077eb1c(out, w)` = **Word → 2 raw byte LE** · `FUN_0077ee84(out, d)` = **DWORD → 4 raw byte LE**.
* `VMT+0x20` của form = **Show**, `VMT+0x24` = **Hide**
  (`005d7000_FUN_005d7000.asm.txt:19-21` — `btn_Close` gọi `[EDX+0x24]`).
* **Mức bằng chứng**: **(A)** đọc body/ASM thật · **(B)** suy luận có cơ sở · **(C)** contract Rust / suy đoán.
* **Hai cây nguồn khác nhau — đọc kỹ trước khi tra `file:line`:**
  1. `client_pseudo_c/` — **có trong repo này** (`case_*.c`, `FUN_*.c/.asm.txt`, `*.hex`). Mọi claim
     mức (A) trong tài liệu này đã **tự verify lại trên cây này**.
  2. `test/ts_decompile/functions/` + `test/ts_decompile/redump/` — cây decompile Ghidra đầy đủ của
     agent khác, **KHÔNG có trong checkout hiện tại**. Các trích dẫn dạng `functions/005d73c0…c:95`
     thuộc cây này; giữ nguyên vì chúng **bổ sung** những gì `client_pseudo_c/` không có.
     → Danh sách file thiếu: `005d6f7c`, `005d7000`, `005d6048`, `005d73c0`, `005dc500`, `005dcce0`,
     `005dd000`, `005d7218`, `007746d4`, `007746ac`, `005efee8`, `005e8534`, `0051189c`, `redump/*`.

---

## §0. Tóm tắt điều hành — 5 phát hiện lớn nhất

| # | Phát hiện | Mức | Ảnh hưởng |
| :-: | :-- | :-: | :-- |
| 1 | **Danh sách hàng + giá của shop NPC KHÔNG có trên wire.** Client tự parse `Data\Eve.emg` (surface record, entry 5 byte `{itemId:2, kind, style, canCut}`) và `Data\Item.Dat` (giá mua = offset 84, giá bán = offset 88). Server chỉ gửi `1B 03` để mở form rồi **validate** `index` theo cùng bảng `eve.emg`. | **(A)** `005efee8…c:458-530` · `005d73c0…c:86,95,115` · `007746d4…c:74-119`; xem [`01_shop_read_flow.md`](./01_shop_read_flow.md) §2.3/§3.1 | Bảng giá hard-code 110 dòng trong `src/server/handlers/shops.rs:51-160` là **thừa + sai kiến trúc**. Nguồn giá đúng đã có sẵn trong `src/data/loaders/item.rs`. |
| 2 | **Frame `1B 03` = "mở khung BÁN" (`TThingSaleForm`), không phải "mở shop NPC".** Payload đúng **2 byte**, handler không đọc thêm gì. | **(A)** `case_024_00791D80_FUN_00791d80.c:106-108` | Rust **không hề gửi `1B 03`** → `TThingSaleForm` không bao giờ mở → người chơi không thấy màn hình bán. Gap chặn tính năng. |
| 3 | **Bảng 12 thông báo lỗi/kết quả đã bị SAI LỆCH 1 HÀNG so với tài liệu cũ.** `Outer=1, Inner=0` là **`Giao dịch thành công`** (hằng `0x798118`), *không phải* "vàng không đủ". Nhóm text đã được decode lại bằng **bảng VISCII trong `src/encoding.rs`** (`viscii_to_unicode`) chứ không phải bảng viết tay. | **(A)** `case_024…c:40-57, 76-94` + `client_pseudo_c/lit_797ee8_798440.hex` + `src/encoding.rs:126-236` | Tài liệu cũ (`01_npc_shop.md` §2.5) đã gán sai `0x798118`. Toàn bộ bảng ở **§7** đã viết lại + verify. `Inner=0` còn là **nhánh thành công** (toast chỉ hiện khi `Inner != 0`). |
| 4 | **`ctx.sub` bị bỏ qua ⇒ `1B 02` (bán) bị xử như mua.** Thêm nữa `payload[1]` (itemIndex) bị bỏ hoàn toàn, và giá bán được tính là `+count` vàng thay vì `count * ItemDef.sell_price`. | **(A)** `shops.rs:172-173` (parse) · `shops.rs:42` (`gold += count`) · `shops.rs:238-264` (mọi request vào nhánh mua) | Bán hàng **mất tiền người chơi** và **trừ nhầm tiền của người mua**. 🔴 |
| 5 | **Sau giao dịch, client cần đúng 3 frame: `17 06` (item vào túi) / `17 09` (item giảm) + `1A 04` (đặt lại số dư) [+ `1A 01` khi bán].** `17 06` = **12 byte** `itemId:2 LE, qty:2 LE, 0, long, giatri, khang, exp:4` — **cùng hình dạng** với record `17 05` (stride 12) nhưng **không có byte slot đầu**. `17 09` = **4 byte** `slot:1, qty:1`. | **(B)** layout `(A)` phía server C# (`TSItem.cs:894-901`, `FormServer.cs:4749-4756`, `TSItemContainer.cs:365-366`), thân client `func_0x0076b458`/`func_0x0076ae24` **không có** trong dump | Rust NPC shop **không gửi frame túi nào** sau khi mua/bán → túi phía client **không đổi** dù DB đã đổi. 🔴 |

---

## §1. Frame inventory

| Frame | Chiều | Payload byte-exact | Ý nghĩa | Nguồn `file:line` | Mức |
| :-- | :-: | :-- | :-- | :-- | :-: |
| `1B 01 <i> <q:2>` | C→S | `1B 01 i q_lo q_hi` (5 B) | **MUA** — `i` = index entry trong list shop, `q` = số lượng (Word LE) | `0077f414_FUN_0077f414.asm.txt:1815-1867`; `NpcShopsHandler.cs:13-30` | **(A)** |
| `1B 02 <s> <q:1>` | C→S | `1B 02 s q` (4 B) | **BÁN** — `s` = ô túi 0..24, `q` = số lượng (**chỉ byte thấp** của Word) | `0077f414…asm.txt:1869-1917`; `NpcShopsHandler.cs:44-57` | **(A)** |
| `1B 03` | S→C | `1B 03` (2 B) | **MỞ `TThingSaleForm`** (khung BÁN) — `(**gvar_007DA720 + 0x20)()` | `case_024_00791D80_FUN_00791d80.c:106-108`; `HotelShopAndSaveMapEffectHandler.cs:13-14` | **(A)** |
| `1B 01 <inner>` | S→C | `1B 01 inner` (3 B) | Kết quả/lỗi nhóm **MUA** — toast `Banner(text, 2000, 0, 0)`, chỉ hiện khi `inner != 0` | `case_024…c:32-68` | **(A)** |
| `1B 02 <inner>` | S→C | `1B 02 inner` (3 B) | Kết quả/lỗi nhóm **BÁN** — cùng cơ chế | `case_024…c:69-105` | **(A)** |
| `14 01..06 <15 B>` | S→C | `14 01 P0 W1 B1 **B2** B3 W2 B4 D1 **W3**` (17 B) | Nạp map + mở hộp thoại/shop; `B2 = 6` → shop/dialog, `W3` = index surface record | `0078a89c_FUN_0078a89c.c:3035-3093` | **(A)** |
| `14 08` | S→C | `14 08` (2 B) | **Đóng `TInterForm`** — `a095 = 0; (**gvar_007DA5A8 + 0x24)()` | `0078a89c…c:3105-3110` | **(A)** |
| `14 09` | C→S | `14 09` (2 B) | Player bấm `btn_Close` → đóng khung shop (`+0xA104 = 0x28`) | `functions/005d7000_FUN_005d7000.asm.txt:8-15` | **(A)** |
| `17 06 <12 B>` | S→C | `17 06 itemId:2 qty_lo qty_hi 00 long giatri khang exp:4` (14 B) | **Item mới vào túi** (mua xong / nhặt / rút kho) | `case_020_007902FB_FUN_007902fb.c:50-51`; `TSItem.cs:894-901`; `FormServer.cs:4749-4756`; `BagHandle.cs:126-131` | **(B)** dispatch (A) / layout (A) |
| `17 05 <N×12>` | S→C | `17 05` + N × `[slot:1][itemId:2][qty:1][doben:1][long:1][giatri:1][khang:1][exp:4]` | **Đồng bộ toàn bộ túi** (stride **12**), `N = (len(RP)−1)/12` | `case_020…c:47-48`; `FUN_0076bfc8.c:79-207` | **(A)** |
| `17 08 <12 B>` | S→C | `17 08 [slot:1][itemId:2][qty:1][0][long][giatri][khang][exp:4]` (14 B) | Refresh **một** ô túi (identical với record `17 05`) | `case_020…c:56-57`; `TSItem.cs:943-954` | **(A)** dispatch / **(B)** layout |
| `17 09 <s> <q>` | S→C | `17 09 slot qty` (4 B) | **Giảm/xoá item trong túi** (dùng sau BÁN, và sau dùng item) | `case_020…c:59-60`; `TSItemContainer.cs:365-366`; `TSItem.cs:878` | **(A)** dispatch / **(B)** layout |
| `17 07` | S→C | **chưa biết** | Client `func_0x0076ad78` — **không có call-site nào trong C# tham chiếu** | `case_020…c:53-54` | **(A)** dispatch / **(C)** layout |
| `1A 01 <g:4>` | S→C | `1A 01 gain:4 LE` (6 B) | **Cộng** `+0x12F8` (trần 9 999 999, atomic reject) + banner + `WA0014.wav` + `TLight` | `case_023…c:34-65`; `0072b084_FUN_0072b084.c:43-56`; `NpcShopsHandler.cs:58-61` | **(A)** |
| `1A 02 <g:4>` | S→C | `1A 02 amt:4 LE` (6 B) | **Trừ** `+0x12F8` (atomic reject nếu không đủ) + banner `"Giảm … Tiền"` | `case_023…c:67-93`; `0072b170_FUN_0072b170.c:41-58` | **(A)** |
| `1A 03` | S→C | `1A 03` (2 B) | Banner tĩnh `"Dung lượng tiền bạc không đủ"` (`0x7980B4`) | `case_023…c:95-97`; `lit_797ee8_798440.hex` | **(A)** |
| `1A 04 <g:4> <p:4>` | S→C | `1A 04 gold:4LE points:4LE` (10 B) | **Set tuyệt đối** `+0x12F8` **và** `+0x1300` (không clamp) + refresh 2 nhãn ngân hàng + caption HUD | `case_023…c:98-100`; `0072bcf8_FUN_0072bcf8.c:51-61`; `005d895c_FUN_005d895c.c:55-70` | **(A)** |
| `1D 04 <b:4>` | S→C | `1D 04 bank:4 LE` (6 B) | **Set tuyệt đối** `+0x12FC` (tiền trong trang) | `0072be10_FUN_0072be10.c:33-40` | **(A)** |
| `1B 01 01` | S→C | 3 B | **"túi đầy"** — `Xin lỗi, bảng vật phẩm của bạn đã đầy!` | `case_024…c:43-45` | **(A)** |

> ⚠ **Đính chính quan trọng:** byte `[6..9]` của `1A 04` ghi vào `+0x1300` (**điểm**), **KHÔNG** phải
> `bank_gold`. Số dư ngân hàng (`+0x12FC`) chỉ set bằng `1D 04`. Server C# `sendGold()`
> (`TSCharacter.cs:5299-5307`) truyền `gold_bank` vào ô thứ 2 ⇒ C# đang lấp đầy ô **điểm** bằng số
> dư ngân hàng. Chi tiết: [`06_currency_model.md`](./06_currency_model.md) §2.1–§2.2.

---

## §2. Timeline MUA (click NPC → form → chọn hàng → `1B 01` → item + tiền)

```text
  SERVER                                        CLIENT (aLogin.exe)
    │
    │ ① player bấm NPC / đi đến map
    ├────────── [14][01 P0 W1 B1 B2=6 B3 W2 B4 D1 W3] ───►  FUN_005EB530
    │                                                            │
    │        (KHÔNG có frame "đẩy danh sách hàng")                │ rec = PlayerState[0x9D34 + W3*4]
    │                                                            │ đọc entry 5 byte từ Data\Eve.emg
    │                                                            │ giá = Data\Item.Dat rec[84]
    │                                                            │ điền TInterForm + Show
    │ ② player click 1 item trong list                             │
    │        guard: TInterForm+0x156 != 0                          │
    │        và (map,shopId) ∉ {(0x2EE1,0x16),(0x2F1D,0x05)}      │
    │        TInterForm+0x154 = 1                                  │
    │        OnChange(TBK_BiImgList) = FUN_005d6f7c               │
    │◄── [1B][01][itemIdx:1][qty:2 LE] ────────────────────────   MUA
    │
    │ ③ validate: entry.kind == 2 (client đã chặn, .c:71)
    │            price = ItemDef.price (item.rs offset 84)
    │            total  = price × qty ;  gold −= total
    │            thêm item vào homdo
    │
    ├── [17 06 itemId:2 qty:2 00 long giatri khang exp:4] ──────►  item vào túi (×N stack)
    ├── [1A 04 gold:4LE points:4LE] ───────────────────────────►  đặt lại số dư + refresh ngân hàng + HUD
    │   (không bắt buộc) [1B][01][00] = "Giao dịch thành công" → Inner==0 ⇒ KHÔNG hiện toast
    │
    │ ④ nhánh lỗi (thay cho ③):
    │    gold không đủ / túi đầy / vượt số lượng cho phép
    ├── [1B][01][01] "…bảng vật phẩm của bạn đã đầy!"  ────────►  Banner 2000 ms
    ├── [1B][01][02] "…Số lượng vật phẩm bạn mua đã đầy!" ─────►
    ├── [1B][01][06] "…vàng của bạn đã đầy"                  ─────►
    ├── [1B][01][03] "Xin lỗi, giao dịch thất bại"             ─────►
    ├── [1B][01][04] "Hệ thống gián đoạn giao dịch"           ─────►
    │    (hoặc [1A 03] "Dung lượng tiền bạc không đủ")
    │
    │ ⑤ player bấm btn_Close
    │◄── [14][09] ────────────────────────────────────────────   đóng TInterForm
    │    (hoặc server ép đóng: [14][08] → Hide(TInterForm))
```

**Điểm then chốt (mức A):** bước ① **không có** frame danh sách hàng; danh sách + giá **100% client-side**.
Xem [`01_shop_read_flow.md`](./01_shop_read_flow.md) §2.3 và §3.1.

---

## §3. Timeline BÁN (`1B 03` → chọn ô → `1B 02` → `1A 01` + `17 09`)

```text
  SERVER                                        CLIENT
    │
    │ ① player bấm nút "bán" ở menu NPC
    ├────────── [1B][03] ───────────────────────────────►  (**gvar_007DA720 + 0x20)()  = Show TThingSaleForm
    │                                                    25 ô lấy từ LocalActor+0x9E0 + idx*4
    │                                                    caption mỗi ô = "kim" + sell_price + " " + tên
    │                                                    editor số lượng +0x150 (MaxLength 2 ⇒ ≤ 50)
    │ ② player chọn ô (slot) + số lượng
    │        validate phía client: 0 < n ≤ stack[+7]  (005dcce0…c:23-45)
    │        btn_Sale → +0x14C (lựa chọn), +0x160 (slot gửi), +0x162 (số lượng gửi)
    │◄── [1B][02][slot:1][qty:1] ─────────────────────►   BÁN   (4 byte — KHÔNG có byte cao của qty)
    │
    │ ③ validate: slot có hàng && qty ≤ stack
    │            gold += qty × ItemDef.sell_price (item.rs offset 88)
    │            giảm/xoá item trong homdo
    │
    ├── [1A 01 gain:4 LE] ─────────────────────────────►  cộng +0x12F8 + banner "Nhận được … Tiền"
    │                                                        + WA0014.wav + TLight   (C# dùng để xác nhận bán)
    ├── [1A 04 gold:4LE points:4LE] ───────────────────►  đặt lại số dư (sendGold)
    ├── [17 09 slot:1 qty:1] ──────────────────────────►  giảm/xoá item trong túi phía client
    │   (không bắt buộc) [1B][02][00] "Giao dịch thành công" → Inner==0 ⇒ KHÔNG hiện toast
    │
    │ ④ nhánh lỗi (thay cho ③):
    ├── [1B][02][01] "…bảng vật phẩm của bạn không có vật phẩm này!" ─► Banner 2000 ms
    ├── [1B][02][02] "…Vật phẩm bạn bán không có nhiều như thế này!"   ─►
    ├── [1B][02][06] "…vàng của bạn đã đầy"                            ─►
    ├── [1B][02][03] "Xin lỗi, giao dịch thất bại"                     ─►
    ├── [1B][02][04] "Hệ thống gián đoạn giao dịch"                   ─►
```

> ⚠ **Thứ tự `1A 01` vs `1A 04`**: C# gọi `sendGold()` (`1A 04`) **trước** rồi mới gửi `1A 01`
> (`NpcShopsHandler.cs:55-61`). Nếu gửi `1A 01` **sau** `1A 04` thì client cộng delta lần nữa vào số
> dư đã đúng ⇒ **hiển thị sai**. Phải theo đúng thứ tự C#, hoặc chỉ dùng `1A 04` + bỏ `1A 01`.
> Mức: **(A)** cho thứ tự trong C#; **(B)** cho hệ quả "cộng hai lần".

---

## §4. `1B 01` — C→S MUA

### 4.1 Layout byte-exact

| off (`P`) | len | kiểu | tên | nguồn ASM |
| :-: | :-: | :-- | :-- | :-- |
| 0 | 1 | u8 | `MainOp = 0x1B` | `0077f414…asm.txt:1822-1824` (`MOV DL,[EBP-0x5]`) |
| 1 | 1 | u8 | `SubOp = 0x01` | `…:1837-1841` (`MOV DL,[EBP-0x6]`) → `@PStrNCat(CL=2)` |
| 2 | 1 | u8 | `itemIndex` = `byte[TInterForm+0x152]` | `…:1842` (`MOV EDX,[0x7DA5A8]; MOV DL,[EDX+0x152]`) |
| 3 | 2 | u16 LE **raw** | `qty` = `Word[TInterForm+0x154]` | `…:1857` (`MOV DX,word [EAX+0x154]`) → `FUN_0077eb1c` tại `…:1860` |
| — | **5** | | **tổng payload** | |

```
P[0]=1B  P[1]=01  P[2]=idx  P[3]=qty_lo  P[4]=qty_hi
```

### 4.2 Builder C→S và dispatch (mức **A**)

* **Bảng dispatch** (verify lại trên cây `client_pseudo_c/`):
  `cs_dispatch_byte200_0x77f474.hex` → `byte[0x77F474 + 0x1B] = 26 (0x1A)`;
  `table_0x77F53C_dword200.hex` → `dword[0x77F53C + 26*4] = 0x00785FFD`.
  ⇒ target thật của MainOp `0x1B` là `0x00785FFD`, **không phải** `case 0x1b: break;`
  (đó là artefact Ghidra — xem [`01_shop_read_flow.md`](./01_shop_read_flow.md) §1.1).
* **Sub-dispatch**: `0077f414…asm.txt:1815-1819`
  ```asm
  00785ffd: MOV AL,byte ptr [EBP + -0x6]
  00786000: DEC AL
  00786002: JZ  0x00786011        ; sub==1 → MUA
  00786004: DEC AL
  00786006: JZ  0x007860c8        ; sub==2 → BÁN
  0078600c: JMP 0x0078a4f2        ; sub>=3 → KHÔNG gửi
  ```
* **Thân builder MUA**: `0077f414…asm.txt:1821-1867` — 3 đoạn `@LStrLAsg`/`@PStrNCat`
  (`CL = 2, 3, 4`) + `FUN_0040402c` (`LStrCopy 4`) + `FUN_0077eb1c` (Word → 2 raw byte)
  + `CALL 0x0051633c` (`TForm1.CY_AddSedQueue` = **gửi đi**, `:1867`).
* **Thứ tự byte** khi nối shortstring: `@PStrNCat` nối **đuôi** (`00402b60__PStrNCat.c:584-606`).
* **`FUN_0077eb1c` = Word → 2 raw byte LE**, không phải chuỗi số
  (`0077eb1c_FUN_0077eb1c.asm.txt:19-46`).

### 4.3 Ý nghĩa 2 field cuối (mức **A**)

| Field | Nguồn ghi | Kết luận |
| :-- | :-- | :-- |
| `+0x152` | set = chỉ số entry vừa click: `functions/005d73c0_FUN_005d73c0.c:103` | **index entry trong list shop**, KHÔNG phải "menu" |
| `+0x154` | `Word`, builder đọc qua `FUN_0077eb1c` (Word→2 raw byte) ⇒ buộc là số lượng | **số lượng** |
| cả hai + `+0x156` | **cùng reset về 0 khi đóng form**: `functions/005d7000_FUN_005d7000.asm.txt:39,41,46` | đây là **state của một lần giao dịch**, không phải layout form |

* **Giá trị `qty` thực tế = 1** trên đường click-item: `FUN_005d6f7c` gán `+0x154 = 1` **ngay trước**
  khi gọi `SendCommand` (`functions/005d6f7c_FUN_005d6f7c.asm.txt:30-36`).
* **Server C# khớp 1:1**: `byte index = data[2]` (`NpcShopsHandler.cs:13`), `byte count = data[3]`
  (`:14`, byte cao bị bỏ) — `switch (data[1])` (`:15`).

---

## §5. `1B 02` — C→S BÁN

### 5.1 Layout byte-exact

| off (`P`) | len | kiểu | tên | nguồn ASM |
| :-: | :-: | :-: | :-- | :-- |
| 0 | 1 | u8 | `MainOp = 0x1B` | `0077f414…asm.txt:1872-1874` |
| 1 | 1 | u8 | `SubOp = 0x02` | `…:1885-1888` |
| 2 | 1 | u8 | `invSlot` (0..24) = `byte[TThingSaleForm+0x160]` | `…:1890` (`MOV EDX,[0x7DA720]; MOV DL,[EDX+0x160]`) |
| 3 | 1 | u8 | `qty` = **byte thấp** của `Word[TThingSaleForm+0x162]` | `…:1903` (`MOV DX,word [EDX+0x162]`) → chỉ `DL` được ghép (`…:1904`) |
| — | **4** | | **tổng payload** | |

```
P[0]=1B  P[1]=02  P[2]=slot  P[3]=qty
```

### 5.2 Builder (mức **A**)

* Cùng entry `0x00785FFD`, nhánh `0x007860C8`: `0077f414…asm.txt:1869-1916`.
  `@PStrNCat(CL = 2, 3, 4)` → `[1B][02][slot][qty]`, rồi `CALL 0x0051633c` (`:1916`).
* **Không có byte high của `qty`** (chỉ `DL` được đẩy) ⇒ payload 4 byte.

### 5.3 Khớp server C# (mức **A**)

`NpcShopsHandler.cs:9-11` — `data[1]` = sub, `data[2]` = index, `data[3]` = count.
`:46` dùng `inventory.items[index - 1]` ⇒ **index là 1-based** *(mức **B**: suy từ C#, chưa xác minh
từ client vì không có call-site nào ghi `TThingSaleForm+0x160` trong code decompiled)*.
`:50` kiểm `quantity >= count`, `:54` `num = count * value.SellingPrice`.

---

## §6. `1B 03` — S→C mở form `TThingSaleForm`

### 6.1 Layout

```
P[0] = 1B
P[1] = 03
(tổng 2 byte — handler KHÔNG đọc thêm byte nào)
```

### 6.2 Bằng chứng

`client_pseudo_c/case_024_00791D80_FUN_00791d80.c:105-108`
```c
else if (iVar2 == 3) {
  (**(code **)(**(int **)gvar_007DA720 + 0x20))();
}
```
* `iVar2` đọc từ `RP[0]` (`case_024…c:30-32`) ⇒ `iVar2 == 3` ⇔ `P[1] == 3` ⇒ **wire = `1B 03`**.
* `gvar_007DA720` = **`TThingSaleForm`**; `VMT+0x20` = **Show** (đối chiếu `VMT+0x24` = Hide tại
  `functions/005d7000_FUN_005d7000.c:26`).
* **Đối chiếu C# hoàn hảo**: `HotelShopAndSaveMapEffectHandler.cs:13-14`
  `client.reply(new PacketCreator(27, 3).send());` — `PacketCreator(byte, byte)` đặt
  `data[0]=op, data[1]=sub, pos=2` (`PacketCreator.cs:41-45`) ⇒ payload đúng 2 byte.
  (Khớp 3 nguồn độc lập: ASM handler + C# handler + Rust contract.)

### 6.3 Cấu trúc form (mức **A/B**)

| Offset của `TThingSaleForm` | Ý nghĩa | Nguồn |
| :-: | :-- | :-- |
| `+0x9E0 + idx*4` (qua `LocalActor`) | 25 ô đồ trong túi | `functions/005dc500_FUN_005dc500.c:170-176` |
| `+0x150` | Editor số lượng (MaxLength 2 ⇒ ≤ 50) | `…c:177-180` |
| `+0x55` | `btn_Sale` (handler `LAB_005dcc5c`, nằm trong **HOLE** decompile) | `…c:200-202` |
| `+0x14C` | slot đang chọn | — |
| `+0x160` | slot **gửi lên server** | đọc bởi builder `1B 02` (`asm:1890`) |
| `+0x162` (Word) | số lượng **gửi lên server** | đọc bởi builder `1B 02` (`asm:1903`) |
| — | caption mỗi ô = `"kim" + Item.Dat.sell_price + " " + <tên item>` | `functions/005dd000_FUN_005dd000.c:399-406` |

* **Validate số lượng bán (client-side)**: `0 < n ≤ stack[+7]` —
  `functions/005dcce0_FUN_005dcce0.c:23-45`. **A**

---

## §7. Bảng 12 thông báo lỗi / kết quả (VISCII) — ĐÃ VERIFY LẠI

### 7.1 Cách verify (quan trọng)

1. **Đọc lại từng hằng literal** trong `client_pseudo_c/case_024_00791D80_FUN_00791d80.c:39-57`
   (Outer 1) và `:76-94` (Outer 2).
2. **Giải mã VISCII bằng bảng trong `src/encoding.rs` của chính repo `ts_dream`**
   (`viscii_to_unicode`, `src/encoding.rs:126-236`) — **KHÔNG** dùng bảng viết tay hay `iconv`.
3. Nguồn byte: `client_pseudo_c/lit_797ee8_798440.hex` (pool `0x797EE8..0x798440`).
   Bố cục hằng Delphi `AnsiString` trong pool này:
   `[FF FF FF FF][len:4 LE][bytes…][00]` — nên địa chỉ literal trỏ vào **byte dữ liệu đầu tiên**,
   `len` nằm ở `addr-4`.

> ⚠ **ĐÍNH CHÍNH SAI LỆCH QUAN TRỌNG.** Bảng ở `01_npc_shop.md` §2.5 cũ gán
> `Outer=1, Inner=0 → 0x798118 = "Xin lỗi, vàng của bạn không đủ!"`.
> Decode thật cho thấy **`0x798118` = `"Giao dịch thành công"`** và
> `"Xin lỗi, vàng của bạn không đủ!"` nằm ở **`0x798138`** (tức `Inner=1`).
> Toàn bộ 12 dòng bên dưới đã viết lại từ đầu.

### 7.2 Bảng đầy đủ

| Outer (`P[1]`) | Inner (`P[2]`) | Wire | Text (decode bằng `src/encoding.rs`) | len | Hằng | Hiện toast? | `case_024…c` |
| :-: | :-: | :-- | :-- | :-: | :-- | :-: | :-- |
| 1 | 0 | `1B 01 00` | **`Giao dịch thành công`** | 20 | `0x798118` | ❌ (`inner == 0`) | `:41` |
| 1 | 1 | `1B 01 01` | `Xin lỗi, vàng của bạn không đủ!` | 31 | `0x798138` | ✅ | `:44` |
| 1 | 2 | `1B 01 02` | `Xin lỗi, bảng vật phẩm của bạn đã đầy!` | 38 | `0x798160` | ✅ | `:47` |
| 1 | 3 | `1B 01 03` | `Xin lỗi, Số lượng vật phẩm bạn mua đã đầy!` | 42 | `0x798190` | ✅ | `:50` |
| 1 | 4 | `1B 01 04` | `Hệ thống gián đoạn giao dịch` | 28 | `0x7981C4` | ✅ | `:53` |
| 1 | 6 | `1B 01 06` | `Xin lỗi, vàng của bạn đã đầy` | 28 | `0x7981E8` | ✅ | `:56` |
| 2 | 0 | `1B 02 00` | **`Giao dịch thành công`** | 20 | `0x798118` | ❌ (`inner == 0`) | `:78` |
| 2 | 1 | `1B 02 01` | `Xin lỗi, vàng của bạn đã đầy` | 28 | `0x798210` | ✅ | `:81` |
| 2 | 2 | `1B 02 02` | `Xin lỗi, bảng vật phẩm của bạn không có vật phẩm này!` | 53 | `0x798238` | ✅ | `:84` |
| 2 | 3 | `1B 02 03` | `Xin lỗi, Vật phẩm bạn bán không có nhiều như thƠ này!` | 53 | `0x798278` | ✅ | `:87` |
| 2 | 4 | `1B 02 04` | `Hệ thống gián đoạn giao dịch` | 28 | `0x7981C4` | ✅ | `:90` |
| 2 | 6 | `1B 02 06` | `Xin lỗi, vàng của bạn đã đầy` | 28 | `0x7981E8` | ✅ | `:93` |

**Ghi chú về text `Outer=2, Inner=3`**: byte VISCII `0xBF` trong literal `0x798278` decode thành
`U+01AF 'Ư'` theo bảng `src/encoding.rs:189` (`0xBF => '\u{01AF}'`). Đây là **nội dung thật của
literal trong binary** (text trong game ghi "thƠ" thay vì "thế"), không phải lỗi decode.
Byte thô: `… 74 68 BF 20 6E E0 79 21` = `"…t" "h" 0xBF " " "n" "à" "y" "!"`.

### 7.3 Cột `inner` chưa có `case` nào

| `inner` | Wire | Hành vi client | Mức |
| :-: | :-- | :-- | :-: |
| `0x00` | `1B 0x 00` | Gán text `"Giao dịch thành công"` nhưng **KHÔNG** hiện toast (điều kiện `inner != 0`) | **(A)** `:40-42, 58-67` |
| `0x05` | `1B 0x 05` | **Không có `case`, không có `default`** ⇒ biến cục bộ `local_2c` **chưa khởi tạo**; điều kiện `inner != 0` vẫn đúng nên **vẫn gọi `Banner`** với **chuỗi rác / rỗng** | **(A)** cho hành vi gọi Banner; **(C)** cho nội dung hiển thị (Ghidra không cho biết giá trị khởi tạo của local) |
| `>= 0x07` | `1B 0x 07..FF` | Y hệt `0x05` | **(A)/(C)** |
| `0x01..0x04, 0x06` | — | Có `case` → text ở §7.2 | **(A)** |

### 7.4 Cơ chế hiển thị

* Toast = `(**gvar_007DA084 + 0x90)(gvar_007DA084, text, 2000, 0, 0)`
  (`:65-66` cho Outer 1, `:102-103` cho Outer 2).
* `gvar_007DA084` = `TSe_TalkMsgFormPlus`, `VMT+0x90` = **`Banner(text, ms, …)`**.
* Điều kiện: `if (RP[1] != 0)` — tức `P[2] != 0` (`:64`, `:101`).
* ⇒ **`inner == 0` là nhánh "thành công"**: server muốn báo công thì gửi `[1B][01][00]`
  (client bỏ qua, chỉ dùng làm ACK logic ở phía server).

---

## §8. Nguồn danh sách hàng + giá — KHÔNG có trên wire

> Chi tiết đầy đủ: [`01_shop_read_flow.md`](./01_shop_read_flow.md) §2.3 (form tự dựng danh sách) và
> §3.1 (bảng "có/không trên wire").

| Câu hỏi | Trả lời | Mức | Nguồn |
| :-- | :-- | :-: | :-- |
| Frame S→C mang danh sách hàng shop NPC? | **KHÔNG TỒN TẠI** | **(A)** | `case_024…c:32-108` chỉ có 3 nhánh (`Outer` 1/2/3), không vòng lặp, không đọc record; không frame nào ghi vào `TInterForm+0x13C..0x14A` (vùng layout list) |
| Danh sách hàng đến từ đâu? | `Data\Eve.emg`, section **Surface**: record `{+0x6E shopId, +0x6D nEntry, +0x72 pageBase, +0x73 nValid, +0x74 mode}`, mỗi entry **5 byte** `{itemId:2 LE, kind:1, style:1, canCut:1}` | **(A)** | `functions/005efee8_FUN_005efee8.c:458-530`; Rust tương đương `src/data/loaders/eve.rs:255-275, 600-661` (`parse_surface_section`) — khớp 1:1 |
| Client biết shop nào khi click? | `W3` (P[15..16]) → `rec = PlayerState[0x9D34 + W3*4]` | **(A)** | `0078a89c…c:3086-3087`; `functions/005eb530_FUN_005eb530.c:823-828` |
| Giá mua? | `Data\Item.Dat` record offset **84** (`ItemDef.price`), record size `0x172 = 370`, 10 001 record | **(A)** | `functions/005d73c0_FUN_005d73c0.c:86,95,115`; `functions/007746d4_FUN_007746d4.c:91,105-119`; Rust `src/data/loaders/item.rs:226-228` |
| Giá bán? | `Data\Item.Dat` offset **88** (`ItemDef.sell_price`) | **(A)** | `functions/005dd000_FUN_005dd000.c:399`; Rust `src/data/loaders/item.rs:229-231` |
| Loại tiền dùng trong giao dịch? | **Chỉ 1 loại: `+0x12F8` (kim)**. Không có trường "loại tiền" trên wire | **(A)** | `NpcShopsHandler.cs:26,28,55` đều đụng `gold` |
| Có frame "giới hạn số lượng mua"? | **KHÔNG.** Client tự clamp (`editor ≤ 1250`, `FUN_005d7218`); server chỉ nhận `count` rồi tự kiểm | **(A)** | `NpcShopsHandler.cs:26` |

**⇒ Kết luận kiến trúc cho server:** bảng `get_npc_shop_price()` hard-code trong
`src/server/handlers/shops.rs:51-160` là **thừa và sai**. Thay bằng:

```rust
let rec    = surface_record(W3);                        // đã có: src/data/loaders/eve.rs
let entry  = rec.sentences[index];                       // 0-based
if entry.data_kind != 2 { /* lỗi [1B][01][03] */ }
let unit   = ctx.data.items[&entry.data_id];
let total  = unit.price  * qty;                          // mua  (offset 84)
let earn   = unit.sell_price * qty;                      // bán  (offset 88)
```

---

## §9. Frame cập nhật túi đồ + tiền sau giao dịch (NGHIÊN CỨU MỚI)

### 9.1 `17 05` — đồng bộ toàn bộ túi: layout byte-exact, stride 12 (mức **A**)

Handler: `case_020_007902FB_FUN_007902fb.c:47-48` → `func_0x0076bfc8(gvar_007DA0D0, RP)`.
Thân hàm có trong cây `client_pseudo_c/`: `client_pseudo_c/0076bfc8_FUN_0076bfc8.c:79-207`.

1. `:79-86` — **xoá sạch** 25 slot hiển thị: `for slot = 1 .. 0x19 { (**(…(gvar_007DA7BC+0x9E0+slot*4) + 8))(); }`
2. `:87-99` — `N = (LStrLen(RP) − 1) / 12` ⇒ **stride = 12**, record bắt đầu tại `RP[1]`
3. `:101-206` — mỗi record đọc (đã quy đổi `_LStrCopy` 1-based → 0-based):

| Offset trong record (bắt đầu tại `RP[1 + 12*i]`) | len | kiểu | biến | dòng |
| :-: | :-: | :-- | :-- | :-- |
| `+0` | 1 | u8 | `local_d` = **slot hiển thị** (1..25) | `:105-116` |
| `+1` | 2 | u16 LE | `local_1a` = **`itemId`** (qua `FUN_0077eb9c`) | `:118-124` |
| `+3` | 1 | u8 | `local_e` | `:125-135` |
| `+4` | 1 | u8 | `local_f` | `:136-146` |
| `+5` | 1 | u8 | `local_10` | `:147-157` |
| `+6` | 1 | u8 | `local_11` | `:158-168` |
| `+7` | 1 | u8 | `local_12` | `:169-179` |
| `+8` | 4 | u32 LE | `local_18` (qua `FUN_0077ed68`) | `:181-187` |
| **tổng** | **12** | | | `:100-206` |

* `FUN_0065b580(gvar_007DA660, itemId, 0, exp)` → slot logic, clamp `0xFF` (`:188-192`).
* Gọi `(**(*obj + 4))(obj, itemId, b3, exp, slot, b7, b6, b5, b4)` (`:202-203`).

**⇒ Byte-exact:**
```
17 05 | [slot:1][itemId:2 LE][b3][b4][b5][b6][b7][exp:4 LE]  × N
```
**Rust `dump_table(..., "1705")` (`src/server/session.rs:450-467`) KHỚP BYTE-EXACT** (mức **A**):
`slot → doben → long_val → 100+giatri → khang → le32(texp)` = 12 byte, đúng thứ tự trên.

### 9.2 `17 06` — item mới vào túi (sau khi MUA) (mức **B**)

Handler: `case_020_007902FB_FUN_007902fb.c:50-51` → `func_0x0076b458(gvar_007DA0D0, RP)`.

> ⚠ **Thân `func_0x0076b458` KHÔNG có** trong `client_pseudo_c/` (đã grep toàn bộ 1136 file, không có
> file nào chứa địa chỉ này) ⇒ **layout `17 06` phải dựng lại từ server C#**, không đọc được từ client.
> Đây là hạn chế thật, ghi rõ ở §11.

**Ba nguồn C# độc lập, tất cả cùng chiều dài 12 byte sau `sub`:**

| Nguồn | `dòng` | Cách dựng |
| :-- | :-- | :-- |
| `TSItem.sendNewItem(qty)` | `TSItem.cs:894-901` | `PacketCreator(23,6)` + `add16(Itemid)` + `add8(qty)` + `addZero(9)` |
| `FormServer` (rút kho → túi) | `FormServer.cs:4749-4756` | `PacketCreator(23,6)` + `add16(Itemid)` + `add16(quantity)` + `addByte(0)` + `add8(other_type)` + `add8(100+other_val)` + `add8(anti)` + `add32(exp)` |
| `BagHandle` case `0x25` | `BagHandle.cs:126-131` | giống `FormServer` |

**Hai nguồn đầu **khớp nhau 100%** ở byte `+2` (`qty_lo`) và `+3` (`qty_hi = 0`).
⇒ **Layout byte-exact của `17 06`** (mức **B** — suy từ 3 call-site C# độc lập, dispatch phía client là (A)):

| off (`P`) | len | kiểu | tên |
| :-: | :-: | :-- | :-- |
| 0 | 1 | u8 | `MainOp = 0x17` |
| 1 | 1 | u8 | `SubOp = 0x06` |
| 2 | 2 | u16 LE | `itemId` |
| 4 | 1 | u8 | `qty` (byte thấp) |
| 5 | 1 | u8 | `qty >> 8` (luôn `0`) |
| 6 | 1 | u8 | `0` |
| 7 | 1 | u8 | `long_val` / `other_type` |
| 8 | 1 | u8 | `(100 + giatri_long) & 0xFF` |
| 9 | 1 | u8 | `khang` / `anti` |
| 10 | 4 | u32 LE | `texp` / `exp` |
| — | **14** | | **tổng payload** |

### 9.3 `17 06` có cùng stride với `17 05` không? → **CÓ, cùng hình dạng 12 byte, lệch 1 byte**

| Frame | Cấu trúc 12 byte sau `sub` | Có byte `slot` đầu? |
| :-- | :-- | :-: |
| `17 05` record | `slot:1, itemId:2, qty:1, doben:1, long:1, giatri:1, khang:1, exp:4` | **Có** |
| `17 08` (`TSItem.getSlotItemBytes`) | `slot:1, itemId:2, qty:1, 0:1, long:1, giatri:1, khang:1, exp:4` | **Có** — **giống hệt record `17 05`** (`TSItem.cs:943-954`) |
| `17 06` | `itemId:2, qty:2, 0:1, long:1, giatri:1, khang:1, exp:4` | **Không** — mọi field lệch **−1 byte** |

⇒ `17 06` = record `17 05` **bỏ byte slot đầu, và `qty` nới từ 1 byte lên 2 byte**. Về mặt kỹ thuật
client nhiều khả năng dùng **cùng một hàm phân tích struct** cho cả hai (đều là object `gvar_007DA0D0`),
chỉ khác điểm neo offset — **mức (B)** vì chưa đọc được thân `func_0x0076b458`.

**Đối chiếu Rust — CẢ HAI builder `1706` đều ĐÚNG layout:**

| Rust | `file:line` | Sinh ra |
| :-- | :-- | :-- |
| `item_added_frame()` | `src/server/handlers/inventory.rs:72-85` | `le16(id) + count + 00 + doben + long + giatri + khang + le32(texp)` = **12 B** ✔ byte-compatible với `FormServer.cs:4749` |
| `equip_item_frame()` | `src/server/handlers/shops.rs:596-609` | `le16(id) + count + 0000 + long + giatri + khang + le32(texp)` = **12 B** ✔ (`0000` = `qty_hi`+`0`) |
| `system.rs` GM shop | `src/server/handlers/system.rs:130-135` | `le16(id) + 01 + 00×9` = **12 B** ✔ khớp `TSItem.sendNewItem` |

* Sự khác biệt duy nhất: `inventory.rs:76` đặt `item.doben` vào offset `+4` trong khi C# hardcode `0`
  (`FormServer.cs:4753`). Vì client đọc offset này làm `other_type` của **trang bị**, mà `doben` ở Rust
  là *item level requirement* ⇒ **nên zero hóa** để khớp C#. Mức: **(C)**.

### 9.4 Item bị xoá/giảm khỏi túi khi BÁN → **`17 09 <slot:1> <qty:1>`** (mức **A/B**)

* Client dispatch: `case_020_007902FB_FUN_007902fb.c:59-60` → `func_0x0076ae24(gvar_007DA0D0, RP)`
  (thân hàm **không có** trong dump ⇒ layout lấy từ C#, mức **B**).
* **Nguồn C# trực tiếp cho luồng BÁN** — `TSItemContainer.delForSallItem()`:
  ```csharp
  if (tSItem.quantity == qty) { items[init_slot - 1] = null; nextSlot(); }
  else                       { tSItem.quantity -= qty; }
  owner.reply(new PacketCreator(new byte[4] { 23, 9, init_slot, qty }).send());
  ```
  `TS_Server_Bear/TS_Server/Client/TSItemContainer.cs:347-366` — đúng **4 byte**:
  `P[0]=17, P[1]=09, P[2]=slot, P[3]=qty`.
* Đối chiếu thêm 2 call-site cùng dạng: `TSItem.cs:878` (dùng item) và
  `BagHandle.cs:47-50` (`PacketCreator(23,9).addByte(slot).add16(quantity)` — 4 byte, dialect
  `slot:1 + qty:2`, tức `qty` là **Word LE** ở đây).
  ⚠ **`17 09` có 2 dialect**: `slot:1 + qty:1` (4 B, `delForSallItem`) và
  `slot:1 + qty:2` (5 B, `BagHandle`). `BagHandle.cs:38-40` còn dùng `PacketCreator(23,9).add16(slot)`
  (chỉ slot, 3 B). ⇒ **server Rust nên theo dialect `delForSallItem`** (khớp đường BÁN của C#).
* **Rust đã dùng đúng dialect này**: `src/server/handlers/inventory.rs:195`
  `F44404001709{slot}{rem}`, `src/server/handlers/use_item/mod.rs:166`,
  `src/server/handlers/trade_storage.rs:682,690`.
  ⇒ **NHƯNG NPC shop KHÔNG dùng** (xem §10).

### 9.5 `17 07` và `17 08` — có liên quan tới shop không?

| Sub | Client callee | Nguồn | Kết luận |
| :-: | :-- | :-- | :-- |
| `0x07` | `func_0x0076ad78` | `case_020…c:53-54` — thân **không có** trong dump | **KHÔNG có call-site nào trong server C# tham chiếu** (`grep "PacketCreator(23, *7"` → 0 hit, chỉ có `(23,77)`/`(23,78)` ở `TSCharacter.cs:5110,5120`). ⇒ **không dùng cho shop**. Mức: **(A)** cho "C# không dùng", **(C)** cho layout |
| `0x08` | `func_0x0076974c` | `case_020…c:56-57` — thân **không có** trong dump | = `TSItem.sendSlotItem` / `getSlotItemBytes` (`TSItem.cs:937-955`): **refresh một ô túi**, 12 byte, layout **giống hệt record `17 05`** (`slot:1 + itemId:2 + qty:1 + 0 + long + giatri + khang + exp:4`). Không dùng cho shop |

### 9.6 Tiền: trừ sau mua / cộng sau bán

| Mục đích | Frame | Layout | Guard phía client | Nguồn | Mức |
| :-- | :-- | :-- | :-- | :-- | :-: |
| **Đặt lại số dư (C# dùng cho cả 2 chiều)** | `1A 04` | `[1A][04][gold:4 LE][points:4 LE]` (10 B) → `+0x12F8`, `+0x1300` | **KHÔNG clamp** ⇒ server phải tự clamp `≤ 9 999 999` | `case_023…c:98-100`; `0072bcf8_FUN_0072bcf8.c:51-61`; `NpcShopsHandler.cs:29,56` (`sendGold()` = `TSCharacter.cs:5299-5307`) | **(A)** |
| **Cộng tiền sau BÁN** | `1A 01` | `[1A][01][gain:4 LE]` (6 B) → `+0x12F8 += gain` | `cap = 9999999 − v; if (gain <= cap)` ⇒ **atomic reject**; fail ⇒ banner `"Thất bại  đoạt được … kim"` | `case_023…c:34-65`; `0072b084_FUN_0072b084.c:43-56`; `NpcShopsHandler.cs:58-61` | **(A)** |
| **Trừ tiền (delta)** | `1A 02` | `[1A][02][amt:4 LE]` (6 B) → `+0x12F8 −= amt` | `if (amt <= v)` ⇒ atomic reject; OK ⇒ banner `"Giảm … Tiền"` | `case_023…c:67-93`; `0072b170_FUN_0072b170.c:41-58` | **(A)** |
| **Số dư ngân hàng** | `1D 04` | `[1D][04][bank:4 LE]` (6 B) → `+0x12FC` | không clamp | `0072be10_FUN_0072be10.c:33-40`; `src/server/handlers/talk.rs:711-716` | **(A)** |

**Trả lời trực tiếp 3 câu hỏi của đề bài:**

1. **`1A 02 <u32 LE>` có phải là frame trừ tiền sau mua không?** — **Frame trừ delta CÓ tồn tại và chạy
   đúng** (`0072b170…c:41`): atomic reject nếu không đủ, refresh `TMoneyAccountForm`, caption HUD
   (`"Giảm" + n + "Tiền"`, `0x798084`+`0x79804C`), nhưng **server C# tham chiếu KHÔNG dùng nó trong
   shop** — C# dùng `sendGold()` (`1A 04`, set tuyệt đối) cho cả mua lẫn bán
   (`NpcShopsHandler.cs:29,56`). Khuyến nghị: theo C# (`1A 04`), vì `1A 04` là set tuyệt đối nên
   idempotent và không bị double-count. **Mức (A)** cho tồn tại, **(C)** cho khuyến nghị.
2. **`1A 01 <u32 LE>` có phải là frame cộng tiền sau bán không?** — **CÓ, và C# dùng đúng frame này
   làm xác nhận bán** (`NpcShopsHandler.cs:58-61`). ⚠ C# `PacketCreator(26,1).add32(num).addZero(8)`
   ⇒ payload **12 byte**, trong khi client chỉ đọc 4 byte rồi bỏ phần đệm ⇒ **vẫn chạy được**.
   **Mức (A)** cho frame, **(A)** cho sự dư 8 byte đệm.
3. **Có frame nào báo "cân nặng túi đầy" không?** — **CÓ, nhưng nó nằm trong kênh `1B`, không phải
   kênh túi**: `[1B][01][01]` = `"Xin lỗi, bảng vật phẩm của bạn đã đầy!"` (`case_024…c:43-45`).
   Không có sub nào của `0x17` mang nghĩa "túi đầy" — client chỉ dùng `1A 03`
   (`"Dung lượng tiền bạc không đủ"`) cho phần tiền. **Mức (A)**.

### 9.7 Bộ khung trả lời chuẩn (copy-paste được)

```text
── MUA thành công (N stack được tạo/cộng dồn) ──────────────────────
  17 06 <itemId:2> <qty:2> 00 <long> <giatri> <khang> <exp:4>   × N
  1A 04 <gold:4 LE> <points:4 LE>
  1B 01 00                      # "Giao dịch thành công" — client KHÔNG hiện toast (inner==0)

── MUA lỗi ─────────────────────────────────────────────────────────
  1B 01 01                      # túi đầy            (hoặc 06 = vàng đã đầy)
  1B 01 03                      # giao dịch thất bại
  1B 01 04                      # hệ thống gián đoạn giao dịch

── BÁN thành công ──────────────────────────────────────────────────
  1A 01 <gain = qty × sell_price : 4 LE>     # banner + âm thanh
  1A 04 <gold:4 LE> <points:4 LE>            # set tuyệt đối (đặt SAU 1A 01 như C#)
  17 09 <slot:1> <qty:1>                     # giảm/xoá item trong túi
  1B 02 00                      # "Giao dịch thành công" (không hiện toast)

── BÁN lỗi ─────────────────────────────────────────────────────────
  1B 02 01                      # không có vật phẩm này
  1B 02 02                      # không có đủ số lượng
  1B 02 03                      # giao dịch thất bại
  1B 02 06                      # vàng đã đầy
```

---

## §10. Rust gap

Đọc `src/server/handlers/shops.rs` (609 dòng), `src/server/session.rs`,
`src/server/spawn.rs`, `src/server/handlers/inventory.rs`, `src/server/dispatcher.rs`.

| # | Vấn đề | Rust đang làm | Client cần | `file:line` | Mức |
| :-: | :-- | :-- | :-- | :-- | :-: |
| i | **Thiếu frame mở form bán `1B 03`** | Không có đường nào gửi `1B 03` (`grep -rn "1B03\|1b03" src/` → 0 hit) | `F44402001B03` khi player bấm "bán" ở menu NPC | thiếu ở toàn bộ `src/`; đối chiếu `case_024…c:106-108` | 🔴 |
| ii | **Bỏ qua `ctx.sub` ⇒ `1B 02` (bán) bị xử như mua** | `let menu = ctx.payload.first()`, `let count = ctx.payload.get(1)`; `ctx.sub` **không** được đọc → mọi request rơi vào nhánh mua | `sub==1` → mua; `sub==2` → bán (slot + qty 1 byte) | `shops.rs:171-173` vs `shops.rs:238-264` | 🔴 |
| ii.1 | **`itemIndex` (`payload[1]`) bị bỏ hoàn toàn** | `count` đọc từ `payload[1]` — nhưng đó là **index**; server không biết người chơi đang mua item nào, chỉ đoán theo `(idtalking, map_id, menu)` | Mua: `index` = entry trong list shop (0-based) · Bán: `slot` 0..24 | `shops.rs:172-173`; layout đúng ở §4/§5 | 🔴 |
| ii.2 | **Số lượng mua luôn = 1, bỏ qua `Word LE`** | `from_template(data, item_id, 1)` — cứng 1 | `payload[2..4]` là Word LE số lượng | `shops.rs:240`; `shops.rs:173` | 🔴 |
| iii | **Không gửi frame kết quả `1B 01/02 + inner`** | Mọi nhánh lỗi/im lặng đều `return` không gửi gì (`shops.rs:216, 230, 242, 258`) | `1B 01 <inner>` / `1B 02 <inner>` theo bảng §7 | `shops.rs:216,230,242,258`; `case_024…c:32-105` | 🔴 |
| iv | **`red_message` dùng `02 0B` — sai kênh** | `F444 + len + "020B" + "00000000" + VISCII text` | Lỗi shop phải qua `1B 01/02 + inner`; `02 0B` theo `case_002_0078B149_FUN_0078b149.c:188-193` **mở 1 form khác**, không in text | `shops.rs:18-22, 201, 233, 261` | 🔴 |
| iv.1 | **Lệch 1 index ở 3 chỗ khác** | `out.send("F44403001B0102")` với comment `// Inventory full` | `[1B][01][02]` = `"Số lượng vật phẩm bạn mua đã đầy!"`; **túi đầy** là `[1B][01][01]` | `src/server/handlers/system.rs:125, 452`; `src/server/handlers/use_item/mod.rs:109` | 🟡 |
| v | **Bảng giá hard-code 110 dòng** | `get_npc_shop_price(idtalking, map_id, menu) -> Option<(u16,u32)>` | Giá = `ItemDef.price` (offset 84) / `sell_price` (offset 88), đã có trong `GameData` | `shops.rs:51-160`; `src/data/loaders/item.rs:226-231` | 🔴 |
| v.1 | **`get_npc_shop_price` trả `None` khi trượt ⇒ im lặng** | `shops.rs:157` `return None` ⇒ client bấm mua, không phản hồi gì | `1B 01 03` ("giao dịch thất bại") | `shops.rs:157`; C# `NpcShopsHandler.cs:35` có `announce(...)` | 🟡 |
| vi | **Tiền: dùng set tuyệt đối `1A 04`, không có delta** | `gold_frame(gold)` = `F4440A00 1A04 {le32(gold)} 00000000`; ô thứ 2 luôn `0` ⇒ **ghi đè `+0x1300` (điểm) về 0 mỗi giao dịch** | `1A 04` set `+0x12F8` **và** `+0x1300`; sau bán C# còn gửi thêm `1A 01 <gain>` | `shops.rs:11-13, 202, 262`; `0072bcf8…c:51-61`; `NpcShopsHandler.cs:58-61` | 🟡 |
| vi.1 | **Giá bán sai hệ thống: `+count` vàng/món** | `conn.session.gold = gold.saturating_add(u32::from(count))` | `gold += count × ItemDef.sell_price` | `shops.rs:42`; `NpcShopsHandler.cs:54` | 🔴 |
| vi.2 | **Không clamp `≤ 9 999_999`** khi cộng (đường bán) | `saturating_add` (chỉ chặn tràn `u32`) | `+0x12F8` trần `9 999 999`; `1A 04` **không clamp** ⇒ client hiển thị tràn và mọi `1A 01` sau đó fail im lặng | `shops.rs:42`; `0072b084…c:43-47`; `06_currency_model.md` §3.1 | 🟡 |
| vii | **`17 06` layout ĐÚNG — nhưng NPC shop không gửi** | `equip_item_frame()` (12 B) **khớp byte-exact** layout client/C# | `17 06 itemId:2 qty:2 00 long giatri khang exp:4` | `shops.rs:596-609` ✔ vs §9.2; nhưng `handle_npc_shop` **không** gọi nó (`shops.rs:238-264`) | 🔴 |
| viii | **Không gửi `17 09` sau khi bán ⇒ túi phía client không đổi** | `try_sell_range()` chỉ `remove_homdo_item()` + cộng tiền, **không** gửi frame túi | `17 09 <slot:1> <qty:1>` | `shops.rs:27-47` (thiếu); `TSItemContainer.cs:365-366` | 🔴 |
| ix | **Bán chỉ cho 2 NPC cứng + khoảng item id cứng** | `idnpctalking == 16005 \|\| 99999` → `26001..=26455`; `16002 \|\| 99999` → `27001..=27165` | Bán **bất kỳ** item nào có `sell_price > 0` trong túi | `shops.rs:179-185` | 🔴 |
| x | **Bỏ qua `kind == 2`** (client chỉ cho mua entry loại 2) | Không validate `data_kind` | `FUN_005d73c0…c:71` chỉ cho mua `rec[6 + idx*5] == 2` | `shops.rs:238`; `functions/005d73c0_FUN_005d73c0.c:71` | 🟡 |
| xi | **Không gửi `1D 04` ⇒ `bank_gold` không bao giờ hiển thị** | `gold_frame` chỉ động tới `+0x12F8`/`+0x1300` | `1D 04 <bank:4 LE>` → `+0x12FC` | `shops.rs:11-13`; `0072be10…c:33-40`; đúng ở `talk.rs:711-716` | 🟢 |
| xii | **`dispatcher.rs` còn comment sai** | Comment: *"aLogin never sends 0x1B (SendCommand `case 0x1b` is empty…)"* | `byte[0x77F474+0x1B]=26 → dword[0x77F53C+26*4]=0x00785FFD`, body thật ở `asm:1815-1917` | `src/server/dispatcher.rs:307-312` | 🟡 |
| xiii | **Frame mở shop `14 01` — `B2` đã đúng** | `F44411001401000000010603{:02X}0000000000000100` ⇒ `P[6] = 0x06` = **KIND shop/dialog** ✔ | `B2 = 6` → `FUN_005EB530:823-851` (mode 1 → `TInterForm`) | `talk.rs:578, 590, 650, 732, 740, 757, 765` ✔ vs `0078a89c…c:3063-3073` (`puStackY_70[5] = RP[5]` = `P[6]`) | 🟢 *(đính chính: tài liệu cũ §5.8 nói `B2 = 1` — **sai**, đã hex-verify lại)* |

### 10.1 Thứ tự ưu tiên sửa

1. **i** — thêm `1B 03` (1 dòng code, mở khóa cả tính năng bán).
2. **ii / ii.1 / ii.2** — parse lại `ctx.sub` + `index` + `count` (Word LE cho mua, 1 byte cho bán).
3. **vii / viii** — gửi `17 06` sau khi mua, `17 09` sau khi bán.
4. **iii / iv** — thay `red_message()` trong `handle_npc_shop` bằng `shop_result(sub, inner)`.
5. **vi / vi.1 / vi.2** — dùng `sell_price`, clamp `9_999_999`, thêm `1A 01` sau bán.
6. **v / v.1 / ix / x** — thay bảng giá hard-code bằng `EveSurfaceData` + `ItemDef`.

---

## §11. CHƯA XÁC MINH

| # | Câu hỏi mở | Vì sao chưa biết | Hướng tìm tiếp |
| :-: | :-- | :-- | :-- |
| 1 | **Thân `func_0x0076b458`** (handler `17 06`) | Không có trong `client_pseudo_c/` (grep toàn bộ 1136 file) | Disassemble `aLogin.exe` tại `VA 0x0076B458`; grep `FUN_0077eb9c` (Word) + `FUN_0077ed68` (DWORD) để xác nhận có đúng **1 Word + 1 DWORD** và offset neo (có/không byte `slot`) |
| 2 | **Thân `func_0x0076ae24`** (handler `17 09`) | Không có trong dump | Disassemble `VA 0x0076AE24`; xác nhận đọc `slot:1 + qty:1` hay `qty:2` (2 dialect C#) |
| 3 | **Thân `func_0x0076ad78`** (handler `17 07`) | Không có trong dump; C# không dùng | Disassemble `VA 0x0076AD78` để xác định ý nghĩa (xóa ô / refresh ô) |
| 4 | **Layout `17 08`** | Thân `func_0x0076974c` không có; layout lấy từ `TSItem.getSlotItemBytes` | Disassemble `VA 0x0076974C`; đối chiếu 12 byte |
| 5 | **`index` của `1B 01` là 0-based hay 1-based?** | Chưa tìm được chỗ **client** gán `TInterForm+0x152` ngoài `FUN_005d73c0` (chỉ có `functions/005d73c0…c:103`, không đọc được vì file thiếu trong checkout) | Đọc `test/ts_decompile/functions/005d73c0_FUN_005d73c0.c` đầy đủ; đối chiếu `pageBase` (`rec+0x72`) — nghiêng về **0-based** trong list đang hiện |
| 6 | **`slot` của `1B 02` là 0-based hay 1-based?** | `TThingSaleForm+0x160` — không có call-site ghi trong code decompiled | Đọc `functions/005dc500_FUN_005dc500.c` (`btn_Sale` `LAB_005dcc5c` **trong HOLE** `0x5DC96A..0x5DCE0`); C# dùng `items[index-1]` ⇒ nghiêng **1-based** (mức **B**) |
| 7 | **Body `btn_Buy` (`LAB_005d6ecc`)** của `form_BuyQ_Npc` | HOLE decompile `0x5D6A9B..0x5D6F7C`, **không có `.asm.txt`** | Disassemble `0x005D6A9B..0x005D6F7C`; tìm `CALL 0x0077f414` với `DL=0x1b` (call-site MUA thứ 2 có thể tồn tại) |
| 8 | **Body `btn_Sale` (`LAB_005dcc5c`)** | HOLE decompile `0x5DC96A..0x5DCE0` | Disassemble; xác nhận nó gán `+0x160`/`+0x162` rồi gọi `SendCommand(0x1B, 2, …)` |
| 9 | **2 cặp `(map, shopId)` bị chặn** `{(0x2EE1,0x16), (0x2F1D,0x05)}` | `functions/005d6f7c…asm.txt:14-29` cho thấy client **không gửi** yêu cầu mua cho 2 shop này, nhưng không rõ server xử lý thay thế bằng cách gì | Đọc `functions/005eb530_FUN_005eb530.c:823-931` xem có nhánh UI riêng cho `mode == 2` hay không |
| 10 | **`TThingBuyQForm` / `TThingSaleQForm`** (`form_BuyQuant_Npc`) | Không có bất kỳ call-site `Show`/`Hide` nào trong code decompiled — có thể là form legacy | Đọc ctor `functions/005d7674…c` / `functions/005dd794…c` và grep `VMT_5D55D4` / `VMT_5D5510` toàn bộ `.asm.txt` |
| 11 | **Text của `DAT_005D75D0`** (toast client-side khi `price == 0`) | Chưa có hex dump (`lit_5d75*.hex` không tồn tại) | Dump `0x005D75D0` (pool literal) |
| 12 | **`TMoneyAccountForm+0x110`** (4 raw byte gửi kèm ở C→S `[1D][sub][4 B]`) | Không tìm ra chỗ ghi (`0077f414…asm.txt:1942-1951` đọc nó; grep `007da46c` không file nào ghi `+0x110`) | Đọc `functions/005d7d18_FUN_005d7d18.c` + ctor `functions/0051189c…c:1170-1180` |
| 13 | **Thứ tự `1A 01` vs `1A 04` khi bán** | C# rõ ràng gọi `sendGold()` rồi `1A 01`, nhưng chưa chứng minh client "cộng hai lần" | Bắt traffic thật; hoặc đọc `FUN_0072c0f0` (gate hiển thị) để hiểu điều kiện chống spam |
| 14 | **Có client nào thật sự gửi `1B 01 00` (inner = 0) không?** | `case_024…c` cho thấy nhánh đó **gán text rồi bỏ**, nên về mặt client là no-op | Chỉ an toàn nếu server C#/protocol khác xử lý; xem `HotelShopAndSaveMapEffectHandler.cs` và `DialogAndBattleEntryHandler.cs` xem có nhánh nào gửi `PacketCreator(27, 1/2)` kèm byte 0 không |
| 15 | **`17 09` dialect nào đúng cho NPC shop?** | C# có 3 cách dựng `23,9` (`TSItemContainer.cs:365`, `TSItem.cs:878`, `BagHandle.cs:38/47`) | Đọc thân `func_0x0076ae24` (mục 2) là dứt điểm |

---

## §12. Source trail

| # | Nguồn | Vị trí | Dùng để |
| :-: | :-- | :-- | :-- |
| 1 | `client_pseudo_c/0077f414_FUN_0077f414.asm.txt` | `1815-1819` (sub-dispatch), `1821-1867` (builder MUA), `1869-1916` (builder BÁN) | **Layout C→S `1B 01` / `1B 02`** (§4, §5) — (A) |
| 2 | `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex` + `client_pseudo_c/table_0x77F53C_dword200.hex` | `byte[0x77F474+0x1B] = 26`; `dword[0x77F53C+26*4] = 0x00785FFD` | Chứng minh builder `0x1B` **tồn tại** (verify lại) — (A) |
| 3 | `client_pseudo_c/00402b60__PStrNCat.c` + `.asm.txt` | `:584-606` / `:4-25` | Thứ tự byte khi nối shortstring (nối **đuôi**) — (A) |
| 4 | `client_pseudo_c/0077eb1c_FUN_0077eb1c.c` / `.asm.txt` | `:5` / `:19-46` | Word → 2 raw byte LE — (A) |
| 5 | `client_pseudo_c/case_024_00791D80_FUN_00791d80.c` | `:30-32, 39` (Outer/Inner), `:40-57, 76-94` (12 hằng), `:58-67, 95-104` (điều kiện toast), `:105-108` (`1B 03`) | Kênh toast lỗi + mở form bán — (A) |
| 6 | `client_pseudo_c/lit_797ee8_798440.hex` | pool `0x797EE8..0x798440`, hằng `0x798118…0x7982AC` | Nội dung 12 text — decode bằng `src/encoding.rs` — (A) |
| 7 | `src/encoding.rs` | `:126-236` (`viscii_to_unicode`) | **Bảng VISCII 1.1 dùng để decode literal** (không dùng bảng viết tay) — (A) |
| 8 | `client_pseudo_c/case_020_007902FB_FUN_007902fb.c` | `:47-48` (`17 05`), `:50-51` (`17 06`), `:53-54` (`17 07`), `:56-57` (`17 08`), `:59-60` (`17 09`), `:157-158` (`17 21`) | Dispatch frame túi — (A) |
| 9 | `client_pseudo_c/0076bfc8_FUN_0076bfc8.c` | `:79-86` (xoá 25 ô), `:87-99` (`N=(len−1)/12`), `:100-207` (12 field/record), `:188-203` (gọi VMT+4) | **Layout byte-exact + stride 12 của `17 05`** — (A) |
| 10 | `client_pseudo_c/case_023_0079175F_FUN_0079175f.c` | `:34-65` (`1A 01`), `:67-93` (`1A 02`), `:95-97` (`1A 03`), `:98-100` (`1A 04`), `:101-112` (`1A 05/06`) | Layout + hướng phép của `0x1A` — (A) |
| 11 | `client_pseudo_c/0072b084_FUN_0072b084.c` / `.asm.txt` | `:43-56` | `1A 01` cộng `+0x12F8`, trần `9999999`, atomic reject — (A) |
| 12 | `client_pseudo_c/0072b170_FUN_0072b170.c` / `.asm.txt` | `:41-58` | `1A 02` trừ `+0x12F8`, atomic reject — (A) |
| 13 | `client_pseudo_c/0072bcf8_FUN_0072bcf8.c` | `:51-61` | `1A 04` set `+0x12F8` **và** `+0x1300` — (A) |
| 14 | `client_pseudo_c/0072be10_FUN_0072be10.c` | `:33-40` | `1D 04` set `+0x12FC` — (A) |
| 15 | `client_pseudo_c/0078a89c_FUN_0078a89c.c` | `:3019-3093` (op `0x14` sub 1..6 + layout 15 B), `:3105-3110` (sub 8 → Hide), `:4884-4885` (op `0x1D`) | Mở/đóng form shop — (A) |
| 16 | `client_pseudo_c/005d895c_FUN_005d895c.c` | `:55-70` | `form+0x114`(`+0x12FC`), `form+0x118`(`+0x12F8`) dạng `%7d` — (A) |
| 17 | `client_pseudo_c/0050705c_FUN_0050705c.c` | `:88-92` (16 ô), `:93-94` (`FUN_00774a84` tên), `:131` (giá `+0x108`), `:107-133` (fallback NPC) | Form shop NPC tự dựng danh sách từ dữ liệu cục bộ — (A) |
| 18 | `client_pseudo_c/lit_72b16c.hex` · `lit_72b24c.hex` · `lit_72bdf8.hex` · `lit_7467c8.hex` | `0x72B16C`=`"kim"`, `0x72BDF8`=`"Tiền bạc"`, `0x7467C8`=`"giao dịch bày ra thất bại"` | Caption HUD + banner `1A 07` — (A) |
| 19 | `test/ts_decompile/functions/` (**cây ngoài repo**) | `005d6f7c…asm:7-36` · `005d7000…asm:8-15,19-30,39-46` · `005d6048…c:169-171` · `005d73c0…c:57-128` · `005dc500…c:170-202` · `005dcce0…c:23-45` · `005dd000…c:399-406` · `005d7218…c:20-45` · `007746d4…c:74-119` · `007746ac…asm:10-11` · `005efee8…c:458-530` · `005e8534…c:182-193` · `005eb530…c:216,823-931` · `0051189c…c:1170-1180,1228-1258,1354-1367,1445-1448,1492-1507` | GUI form, nguồn `eve.emg`/`Item.Dat`, định danh tên class Delphi — (A) |
| 20 | `TS_Server_Bear/TS_Server/PacketHandlers/NpcShopsHandler.cs` | `:9-11` (parse), `:17-43` (mua), `:44-69` (bán), `:54` (`count*SellingPrice`), `:56-61` (`sendGold` + `PacketCreator(26,1).add32(num).addZero(8)`) | Đối chiếu layout + nghiệp vụ mua/bán — (A)/(B) |
| 21 | `TS_Server_Bear/TS_Server/Client/TSItem.cs` | `:894-901` (`sendNewItem` → `17 06`), `:937-955` (`getSlotItemBytes` → `17 08`), `:878` (`17 09`) | Layout `17 06` / `17 08` / `17 09` — (A) |
| 22 | `TS_Server_Bear/TS_Server/Client/TSItemContainer.cs` | `:347-366` (`delForSallItem` → `17 09 <slot> <qty>`) | **Frame xoá item sau BÁN** — (A) |
| 23 | `TS_Server_Bear/TS_Server/FormServer.cs` | `:4749-4756` (`17 06` dạng đầy đủ 12 B) | Xác nhận `17 06` qty là **Word LE** — (A) |
| 24 | `TS_Server_Bear/TS_Server/PacketHandlers/BagHandle.cs` | `:38-40, 47-50, 91-93` (dialect `17 09`), `:126-131` (`17 06`) | Ba dialect `17 09` — (A) |
| 25 | `TS_Server_Bear/TS_Server/Client/TSCharacter.cs` | `:5299-5307` (`sendGold()` = `[1A][04]`), `:5110,5120` (`17 4D`, `17 4E` — **không** phải `17 07/08`) | Frame tiền + loại trừ nhầm `17 07/08` — (A) |
| 26 | `TS_Server_Bear/TS_Server/PacketCreator.cs` | `:41-45` (ctor 2-opcode), `:55-78` (framing `F4 44` + XOR `0xAD`) | Xác nhận payload 2 byte của `1B 03` — (A) |
| 27 | `TS_Server_Bear/TS_Server/Client/QuestStepHelper/HotelShopAndSaveMapEffectHandler.cs` | `:13-14` (`PacketCreator(27,3)`) | Frame mở form bán — (A) |
| 28 | `TS_Server_Bear/TS_Server/PacketProcessor.cs` | `:120-121` (`case 27` → `NpcShopsHandler`) | Dispatch phía server C# — (A) |
| 29 | `src/server/handlers/shops.rs` | `:11-13` (`gold_frame`), `:18-22` (`red_message`), `:27-47` (`try_sell_range`, `:42` `gold += count`), `:51-160` (`get_npc_shop_price`), `:166-265` (`handle_npc_shop`, `:171-173` parse), `:179-185` (NPC bán cứng), `:238-264` (mua), `:596-609` (`equip_item_frame` `1706`) | **§10 gap (i)–(ix)** — (A) |
| 30 | `src/server/session.rs` | `:196` (`gold`), `:214` (`homdo`), `:242` (`bank_gold`), `:425-427` (`dump_homdo`), `:450-467` (`dump_table` record 12 B) | Đối chiếu `17 05` + field tiền — (A) |
| 31 | `src/server/spawn.rs` | `:552-557` (`dump_*`), `:560-564` (`1A 04`) | Frame lúc login — (A) |
| 32 | `src/server/handlers/inventory.rs` | `:72-85` (`item_added_frame` `1706`), `:187-195` (`1709`) | Đối chiếu `17 06` / `17 09` — (A) |
| 33 | `src/server/dispatcher.rs` | `:228-235` (`ctx.sub = decoded[5]`, `ctx.payload = decoded[6..]`), `:307-312` (comment sai) | Quy ước payload + gap xii — (A) |
| 34 | `src/server/handlers/talk.rs` | `:578, 590, 650, 732, 740, 757, 765` (frame `14 01` với `B2=0x06`), `:711-716` (`1D 09`/`1D 04` ngân hàng) | Mở shop (đúng) + bank frame (đúng) — (A) |
| 35 | `src/server/handlers/system.rs` · `use_item/mod.rs` | `:125, 452` · `:109` — `F44403001B0102` "Inventory full" | Gap iv.1 — (A) |
| 36 | `src/data/loaders/item.rs` | `:34-35`, `:226-231` (`price` @84, `sell_price` @88), `FIELD_LENGTH = 370` | Nguồn giá thay bảng hard-code — (A) |
| 37 | `src/data/loaders/eve.rs` | `:255-275` (`EveSentence`), `:371-379` (9 section), `:486-496` (`skip_goods_section`), `:600-661` (`parse_surface_section`) | Nguồn danh sách hàng — (A) |
| 38 | `.scratch/client-pseudo-op-code/economy/01_shop_read_flow.md` | §1.1, §2.3, §3.1, §6 | Tài liệu nguồn dữ liệu shop (giữ nguyên, không xoá) |
| 39 | `.scratch/client-pseudo-op-code/economy/06_currency_model.md` | §2.1, §2.2, §3.1, §4.3 | Bảng offset tiền + guard trần (đã kiểm chứng) |

---

*Ngày hợp nhất: 2026-10-02. Tài liệu cũ `01_npc_shop.md` đã được `git mv` (thực tế là `mv`, thư mục
`.scratch/` chưa được track) thành file này.*
