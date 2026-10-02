# 04 — Stall / Vendor (gian hàng) — MainOp `0x17` SubOp `0x1E` + BuyVender

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

Ngày: 2026-10-01 · Nguồn: decompile Ghidra `aLogin.exe` (Delphi) — `test/ts_decompile/functions/`
(raw dump tại `ts_dream/client_pseudo_c/`, `test/ts_decompile/redump/`).

> **Mục đích:** đóng gap cho Rust server `src/server/handlers/shops.rs` (`handle_player_shop` subs 30..33).

## Ghi chú phương pháp / độ tin cậy

- **Quy ước chỉ số payload.** Trong pseudo-C Delphi, dispatcher S→C truyền `RestPayload` (`RP`) =
  payload **bỏ byte main-op**, con trỏ tới `payload[2]` (byte `0x17`). `RP[k]` 1-based ⇒
  `payload[k+1]`. Vì vậy:
  - `SubOp = *(byte*)(RP + 0)` = `payload[2]` (`client_pseudo_c/case_020_007902FB_FUN_007902fb.c:29-33`)
  - `Copy(RP, p, n)` = `payload[p+1 .. p+n]` (`opcode_17.md` §1).
- Ở phía Rust: `ctx.sub = decoded[5] = payload[2]`, `ctx.payload = decoded[6..] = payload[3..] = RP[2..]`
  (`src/server/dispatcher.rs:226-228`). Nên `ctx.payload[i] ≡ RP[i+2] ≡ payload[i+3]`.
- `case N: break` trong Ghidra pseudo-C **KHÔNG phải** bằng chứng về việc subop không tồn tại —
  xem mục 5.1 (một kết luận cũ trong `opcode_17.md` §5 bị bác bỏ bằng bằng chứng thô).
- Confidence: **A** = đọc trực tiếp từ asm/pseudo-C; **B** = suy ra từ nhiều call-site;
  **C** = suy đoán chưa xác nhận.

---

## 1. Tổng quan luồng

```text
        SERVER                                        CLIENT (aLogin.exe)
          │
          │  S→C 17 1F  [b:1][uid:4][name_len:1][name][image:1]   (Confidence C — handler chưa decompile)
          │        └─► spawn "avatar + sạp" của người bán trên map
          │
          │  S→C 17 1E  [name_len:1][name:N][stall_id:1][(slot:1, item_id:4 LE)]×N
          │        └─► FUN_00730F4C  (client_pseudo_c/00730f4c_FUN_00730f4c.c:32-175)
          │             ├─ ghi item vào lưới ô actor cục bộ: cell[0x1C]=1, cell[0x20]=item_id
          │             │     (FUN_00768638, functions/00768638_FUN_00768638.c:21-23)
          │             ├─ FUN_0071E164  → tạo TVenderItem (sprite sạp), tên "Stall"+id
          │             └─ TCY_VenderMenu.Show : stall_id @ +0x20C, handle @ +0x210
          │
          │  S→C 17 21  <catalog có giá>          → FUN_00610290(TCY_BuyVenderMenu)   (B)
          │        └─► điền price vào entry[i]+0x04, icon "OnePrice", 10 dòng
          │
          │  S→C 17 20  [b:1][uid:4 LE]           → FUN_00731388
          │        └─► uid == uid mình ? VMT[9](actor cục bộ)
          │            : VMT[9](gvar_007DA300[idx])   ⇒ **đóng/xoá entity sạp**
          │
          ◄──────────────────────────────────────────────────────────
          │  C→S 17 ??  [vendor_uid:4]      (click sạp người khác)   — FUN_00771774
          │  C→S 17 ??  [mode=form[0x134]]  (nút Buy / Buy All)      — FUN_00611330
          │  C→S 17 ??  (mở/đóng sạp của mình, đặt giá)              — CHƯA ĐỊNH VỊ
          ▼
```

**Nhánh "click sạp của người khác"** — `functions/00771774_FUN_00771774.c:23-36`:

```c
FUN_007afa94(param_1);                                  // vẽ sprite
if (*(int *)(*(int *)gvar_007DA7BC + 4) == *(int *)(param_1 + 0xf0)) {
    (**(code **)(**(int **)gvar_007DA4F8 + 0x20))();    // sạp của MÌNH → mở form TCY_VenderMenu, KHÔNG gửi gói
} else {
    *(undefined4 *)(*(int *)gvar_007D9C74 + 4) = *(undefined4 *)(param_1 + 0xf0); // lưy vendor uid
    FUN_0077f414(*(undefined4 *)gvar_007D9D30, 0x17);   // ← gửi C→S main op 0x17
}
```
→ Đây chính là request mở sạp người khác; server trả lại `17 1E`. **Confidence A** (đọc thẳng).

---

## 2. Bảng frame (S→C, MainOp `0x17`)

Nguồn bảng nhảy thật: `client_pseudo_c/cs_op17_targets_0x783405.hex` (61 dword @ `0x783405`),
đọc bằng `client_pseudo_c/cs_op17_dispatch.asm.txt:5-8`. Entry mặc định = `0x78A4F2`.

| SubOp | Handler | Nhận vào | Payload (chỉ số `payload[]`, **1-based**) | Conf. | Evidence |
|---|---|---|---|---|---|
| `0x1D` | *không có* | — | rơi vào default `0x78A4F2` | **A** | `cs_op17_targets_0x783405.hex` entry `[0x1D] = F2 A4 78 00` |
| `0x1E` | `FUN_00730F4C` | `gvar_007DA7BC`, RP | `[2]=name_len`<br>`[3 .. 2+name_len]=name`<br>`[3+name_len]=stall_id`<br>`[4+name_len+i*5]=slot (≤0x19)`<br>`[5+name_len+i*5 .. 8+name_len+i*5]=item_id u32 LE`<br>`N=(len−4−name_len)/5` | **A** | `client_pseudo_c/00730f4c_FUN_00730f4c.c:89-162` |
| `0x1F` | `FUN_007311C8` *(chưa decompile)* | `gvar_007D9D34` | `[2..?]` — server Rust đoán `[b:1][uid:4][name_len][name][image]` | **C** | dispatcher `007902fb_FUN_007902fb.c:284-287`; không có body trong `test/ts_decompile/functions/` |
| `0x20` | `FUN_00731388` | `gvar_007D9D34` | `[3..6]=uid u32 LE` (đọc bằng `Copy(RP,2,4)` ⇒ **payload có 1 byte `[2]` trước uid**) | **A** | `functions/00731388_FUN_00731388.c:43-55` |
| `0x21` | `FUN_00610290` *(chưa decompile)* | `gvar_007DA2F8` = `TCY_BuyVenderMenu` | catalog kèm giá — Rust hiện gửi `17` byte 0 + mỗi món `id2 count2 price4 long giatri khang texp4 idx1` | **B/C** | dispatcher `007902fb_FUN_007902fb.c:295-299`; `src/server/handlers/shops.rs:269-293` |
| `0x22` | `FUN_0060E3CC` *(chưa decompile)* | `gvar_007DA4F8` = `TCY_VenderMenu` | làm mới form sạp của chính mình | **B** | dispatcher `007902fb_FUN_007902fb.c:300-303` |

### 2.1 Byte table `17 1E` (đã xác minh byte-by-byte)

| Off (0-based `payload`) | Size | Ý nghĩa | Nguồn |
|---|---|---|---|
| 0 | 1 | MainOp `0x17` | framing |
| 1 | 1 | SubOp `0x1E` | `case_020_007902FB_FUN_007902fb.c:33` |
| 2 | 1 | `name_len` | `00730f4c…c:89` (`*(byte*)(RP+1)`) |
| 3 | `name_len` | tên chủ sạp (byte VISCII, copy tối đa 14 ký tự) | `00730f4c…c:90-92`, `FUN_0071e164…c:56-57` (`PStrNCpy(…,0x0E)`) |
| `3+name_len` | 1 | `stall_id` (dùng để đặt tên nhãn `"Stall"+IntToStr(stall_id)`) | `00730f4c…c:103,162-168`; `FUN_0071e164…c:58-61` |
| `4+name_len+i*5` | 1 | `slot`, guard `slot ≤ 0x19 (25)` | `00730f4c…c:143,153-156` |
| `5+name_len+i*5` | 4 | `item_id` **u32 LE** (`FUN_0077EF7C` = DWORD LE) | `00730f4c…c:150-151`; `functions/0077ef7c_FUN_0077ef7c.c` |

Tổng: `len = 4 + name_len + 5·N`; `N = (len − 4 − name_len) / 5` (`00730f4c…c:104-114`).

> ⚠️ **Không có trường giá trong `17 1E`.** Giá chắc chắn đến từ frame khác (mục 5).
>
> **Bằng chứng mạnh nhất (Mức A, verify 2026-10-02):** trường 4 byte ở offset
> `5+name_len+i*5` được đưa **thẳng** vào `FUN_00768638(vendorSlotObj, item_id)`
> (`00730f4c…asm.txt:113-127`) để dựng `TVenderItem` — **cùng hàm** dùng cho item id ở mọi
> nơi khác. Nó là **`item_id`**, không phải giá. Xem thêm §2.4 (contract Bear, **mức C**,
> có mâu thuẫn — đã ghi rõ).

### 2.1b ★ GIÁ của stall đến từ đâu? (tổng hợp các frame S→C liên quan)

* **KHÔNG** trong `17 1E`. **Mức A** (xem trên)
* Các frame S→C còn lại của main `0x17` có xử lý stall
  (`case_020_007902FB_FUN_007902fb.c`):
  * `17 1F` → `func_0x007311c8(*gvar_007D9D34, RP)` — **không có body** (`:151-152`)
  * `17 20` → `FUN_00731388(*gvar_007D9D34, RP)` — **có body**
  * `17 21` → `func_0x00610290(*gvar_007DA2F8, RP)` — **không có body** (`:157-158`)
    *(`gvar_007DA2F8` = `TCY_BuyVenderMenu`, xem §3.5)*
  * `17 22` → `func_0x0060e3cc(*gvar_007DA4F8, RP)` — **không có body** (`:160-161`)
    *(`gvar_007DA4F8` = `TCY_VenderMenu`, giữ `stall_id` ở `+0x20C` và component stall ở
    `+0x210` — xem §2.2/§3.4)* ⇒ **ứng viên mạnh nhất cho "danh sách hàng + giá của stall
    đang mở"**. **Mức B**
* `FUN_00731388` (đọc thật, `00731388_FUN_00731388.c:43-57`):
  `Copy(RP,2,4)` → `FUN_0077ef7c` **u32 LE**; nếu `== *(player+4)` thì `VMT+0x24` của
  `*gvar_007DA7BC`, ngược lại `FUN_0070c20c(obj,id)` → `VMT+0x24` của phần tử trong
  `gvar_007DA300[id]`. ⇒ **`17 20 [id:4LE]` = đóng/ẩn một thực thể theo id** (id 0 = chính
  mình). **Mức A** *(verify: đúng 2 nhánh so sánh + `gvar_007DA300 + uVar1*4`)*

> **KẾT LUẬN:** GIÁ stall **không nằm trong `17 1E`**; nó phải đến từ frame riêng
> (`17 21` hoặc `17 22` theo suy luận **B**). **CHƯA XÁC MINH** — cần body của
> `0x00610290` / `0x0060e3cc` / `0x007311c8`. Riêng §4.2 lập luận mạnh hơn rằng giá **bắt
> buộc** phải đến ở `17 21` vì form mua so `actor+0x12F8` với `entry[i]+0x04`
> (`FUN_00611330…c:39`) mà `entry` chỉ được `17 21` điền.

---

### 2.2 ★ Form UI stall, `FUN_0071e164`, và state sau khi mở (bổ sung 2026-10-02)

> Hợp nhất từ `03_trade_and_stall.md` §A.2. **Giữ nguyên `file:line` + mức độ.**

* `FUN_0071e164 @ 0x0071e164` — theo call-site `00730f4c_FUN_00730f4c.asm.txt:131-138`:
  ```
  EAX = *gvar_007DA7BC            ; form/nhân vật của local player
  EDX = tên (shortstring 14 byte)
  CL  = stall_id
  CALL 0x0071e164
  ```
  ⇒ `FUN_0071e164(self, shop_name(≤14), stall_id)`. **Mức A**
  *(Lưu ý: §3.3 dưới đây dẫn `functions/0071e164_FUN_0071e164.c:54-66` — đó là file ở
  `test/ts_decompile/functions/`; trong bộ curated `client_pseudo_c/` **không có** body
  của hàm này, chỉ có call-site. Hai nguồn cùng dẫn một hàm, không mâu thuẫn.)*

* Sau đó (`asm:139-146`): `(*(**gvar_007DA4F8))[VMT+0x24]()` rồi
  `*(byte*)(*gvar_007DA4F8 + 0x20C) = stall_id`.
  ⇒ `gvar_007DA4F8` là form chính; `+0x20C` = `stall_id` đang mở; `+0x21C` **không** dùng.
  **Mức A** *(khớp §3.4)*

* Rồi (`asm:147-161`): `IntToStr(stall_id)` → `_LStrCat3(buf, "Stall", str)` →
  `FUN_007c9b38(*gvar_007D9ED8, "Stall" & IntToStr(stall_id))` →
  `*(void**)(*gvar_007DA4F8 + 0x210) = <component>`.
  ⇒ **Client tạo/lookup một component tên `"Stall<stall_id>"` (ASCII, literal tại
  `0x007311C0`, `.c:166`) và cache con trỏ nó ở `*(gvar_007DA4F8) + 0x210`.**
  **Mức A (tên component) / B (ý nghĩa "form stall")** *(khớp §3.4 dòng `+0x210`)*

* `TVenderItem` / `TVenderManage`: `client_pseudo_c/index.csv:5585`
  (`TVenderManage.Create @ 0x0075C004`), `index.csv:5677` (`TVenderItem.Create @ 0x007716DC`),
  `index.csv:5678` (`TVenderItem.Destroy @ 0x00771728`).
  ⇒ **26 ô `vendorSlot` nằm ở `**gvar_007DA7BC + 0x9E0` (26 × 4 byte), tối đa 25 vì
  guard `<= 0x19`.** **Mức A** *(khớp §3.1 dòng `+0x9E0`)*

* **Bảng `slot → item_id` được ghi vào `**gvar_007DA7BC + 0x9E0 + slot*4`**, tức là dùng
  chung vùng offset với nhóm handler `0x17 0x11` (26 ô trang bị/trạng thái). **Mức B**
  *(bổ sung cho §3.1)*

### 2.3 ★ Literal VISCII liên quan stall (đã giải mã + verify lại 2026-10-02)

Bộ giải mã: bảng `viscii_to_unicode` trong `src/encoding.rs:128-236` (VISCII 1.1 / RFC 1456).

| Địa chỉ | Nguồn | Chuỗi decode (verify) |
|---|---|---|
| `0x00745974` | `client_pseudo_c/lit_745934_7459d7.hex` off 64 | **"Không thể bày bán tại đây"** |
| `0x00745934` | cùng file, off 0 | "Không thể tái sử dụng Thăng Hoa Đan trên võ tướng này" |
| `0x00745998` | cùng file, off 100 | "Điểm kinh nghiệm đã vượt mức." |
| `0x007467C8` | `client_pseudo_c/lit_7467c8.hex` off 0 | **"giao dịch bày ra thất bại"** |
| `0x007311C0` | literal inline ASM `00730f4c…asm.txt:153` | **`"Stall"`** (ASCII, ghép `IntToStr(stall_id)`) |

Frame đi kèm (mức A):

* **`1A 07 [code:1]`** — `case_023_0079175F_FUN_0079175F.c:114` → `func_0x00746774` ;
  `00746774_FUN_00746774.c:27-29`: **chỉ khi `code == 1`** mới toast
  `"giao dịch bày ra thất bại"` trong **1500 ms** (`0x5DC`).
  ⇒ **`1A 07 01` = mở stall THẤT BẠI**; code ≠ 1 ⇒ im lặng (thành công / bị từ chối khác).
  **Mức A** *(verify: `case_023…c` `case 7:` gọi `func_0x00746774`; `00746774…c:27-29` so
  `== '\x01'` rồi toast `&DAT_007467c8` với `0x5dc`)*

* **`17 38 [code:1]`** — `case_020_…c:245-246` → `func_0x00745898` ;
  `00745898_FUN_00745898.c:28-38`: `code 1` → toast `0x745934` **2000 ms**;
  `code 2` → `0x745974` **1200 ms** (`0x4b0`); `code 3` → `0x745998` **1200 ms**.
  ⇒ `17 38` là **lỗi dùng vật phẩm**, **KHÔNG** phải stall (đính chính giả thuyết ban đầu).
  **Mức A** *(verify: `00745898…c:28-38` đúng 3 nhánh 2000/0x4b0/0x4b0)*

> **KHÔNG tìm thấy literal `Stall`/`Mua`/`Bán`/`Giá`/`Đặt` nào khác** trong 208 file
> `lit_*.hex` (đã quét toàn bộ, giải mã VISCII). Nhãn UI của form stall nhiều khả năng
> nằm trong **resource .dfm** chứ không phải hằng code ⇒ **CHƯA XÁC MINH**. **Mức A**

### 2.4 ★ Contract stall của server C# tham chiếu (MỨC C — KHÔNG phải `aLogin.exe`)

> Hợp nhất từ `03_trade_and_stall.md` §A.5. **Giữ nguyên mức C.**

`TS_Server_Bear/TS_Server/PacketProcessor.cs:120-122` → `case 23 (=0x17): new ItemHandler(...)`.
`TS_Server_Bear/TS_Server/PacketHandlers/ItemHandler.cs:306-317` → sub `30,31,32,33` → `ShopHandler`.

`TS_Server_Bear/TS_Server/PacketHandlers/ShopHandler.cs`:

| Chiều | Frame | Payload | Dòng |
|---|---|---|---|
| C→S | `17 1E` **mở stall** | `[len:1][name:len][image:1]` rồi `([slot:1][price:4 LE]) × N` stride 5, bắt đầu `data[len+4]` | `:13-39` |
| S→C | `17 1E` echo cho chủ stall | y hệt dạng trên | `:25-41` |
| S→C | `17 1F` broadcast "mở cửa" | `[owner_id:4 LE][len][name][image]` | `:42-45` |
| C→S | `17 1F` **đóng stall** | rỗng | `:48-52` |
| S→C | `17 20` broadcast "đóng" | `[owner_id:4 LE]` | `:53-55` |
| C→S | `17 20` **xem stall** | `[owner_id:4 LE]` | `:58-67` |
| S→C | `17 21` **nội dung + GIÁ** | header rỗng rồi `([itemid:2 LE][qty:2][price:4 LE][0:1][100:1][0:1][0:4][idx:1]) × N` — **16 byte/item** | `:119-148` |
| C→S | `17 21` **mua** | `data[2..5]` 4 byte (owner id) + `data[6]=idx` + `data[7]=qty` | `:72-114` |

> ⚠ **Mức C.** Đây là contract của **một client khác** (server Bear đi kèm client tiếng Thái;
> chuỗi trong C# là UTF-8 Thái). Chỉ dùng để *định hướng*, **không** làm spec cho `aLogin.exe`.
>
> ⚠ **MÂU THUẪN với bằng chứng `aLogin.exe`:** Bear coi `17 1E` là **C→S mở stall** và có
> **`price:4`**; còn `aLogin.exe` (§2.1) đọc `17 1E` là **S→C** với trường thứ 3 là
> **`stall_id`** và trường 4 byte là **`item_id`, KHÔNG phải giá** (đưa thẳng vào
> `FUN_00768638`). Hai bản **lệch nhau** — không dùng bảng Bear để suy ra layout `aLogin.exe`.

## 3. Layout đối tượng client

### 3.1 Singleton — `gvar_007DA7BC` = **actor/nhân vật cục bộ** (`*(gvar_007DA7BC)`)

| Offset | Kiểu | Nghĩa | Evidence |
|---|---|---|---|
| `+0x04` | dword | **uid người chơi** (serial) | `functions/00771774_FUN_00771774.c:25`, `functions/00731388_FUN_00731388.c:45` |
| `+0x9E0` | `array[26] of ^TItemCell` (stride 4, phần tử là **con trỏ**) | lưới ô hiển thị — dùng chung cho túi đồ lẫn hàng sạp | `functions/005335e8_FUN_005335e8.c:53-105` (vòng `1..25`, `0x19`), `functions/00768d38_FUN_00768d38.c:40-47` |
| `+0x12F8` | dword | **tiền (gold)** — so với giá khi mua | `functions/00611330_FUN_00611330.c:44` |
| `+0x5C0` | byte | `1` = đang mở sạp của chính mình | `FUN_0071e164…c:54` (set 1), `FUN_0071fa14…c:23` (set 0), đọc: `0076d578…c:617`, `0071f744…c:51` |
| `+0x5C1` | `shortstring[15]` | tên hiển thị của chủ sạp | `FUN_0071e164…c:57` (`PStrNCpy(…,0x0E)`), xoá: `FUN_0071fa14…c:24` |
| `+0x5D0` | dword | handle UI của nhãn `"Stall"+id` (lookup `FUN_007C9B38`), khởi tạo `-1` | `FUN_0071e164…c:60-61`, `FUN_0071fa14…c:25` |
| `+0x5D4` | pointer | **`TVenderItem`** — sprite sạp đặt trên map | `FUN_0071e164…c:62-66`, `FUN_0071fa14…c:26-27` |
| `+0x63A` | u16 | map id (dùng gate "được mở sạp ở đây không") | `functions/0060ecdc_FUN_0060ecdc.c:29-30` |

`gvar_007DA300[]` = mảng con trỏ actor khác (NPC/người chơi), có **cùng** nhóm field sạp
`+0x5C0/+0x5D0` (`007a3260…c:40`, `007a2d20…c:44`, `00642c2c…c:109`, `00646a34…c:349`).

**Confidence A** cho tất cả offset; **ý nghĩa nghiệp vụ** của `0x9E0` là **B** (xem 3.2).

### 3.2 `TItemCell` (phần tử của lưới `+0x9E0`) — **Confidence B**

| Offset | Kiểu | Nghĩa | Evidence |
|---|---|---|---|
| `+0x00` | dword | con trỏ class (VMT) | `functions/0076bfc8_FUN_0076bfc8.c:84` `(**(code**)(**cell+8))()` |
| `+0x04` | u16 | `item_id` (template) | `005335e8…c:93`, `00768d94…c:60` |
| `+0x07` | u8 | `count` (0 = ô trống) | `functions/00768d38_FUN_00768d38.c:46`, `005335e8…c:70` |
| `+0x10`, `+0x14` | dword | dữ liệu phụ | `005335e8…c:176`, `00533468…c:73` |
| `+0x1C` | byte | cờ "ô này là hàng của sạp" | `FUN_00768638…c:21` |
| `+0x20` | u32 | **`item_id` hàng sạp** | `FUN_00768638…c:23` |
| VMT[0] `()` | method | `Clear()` — wipe cả 26 ô | `functions/0076bfc8_FUN_0076bfc8.c:84` |
| VMT[1] `(9 tham số)` | method | `SetItem(...)` | `functions/0076bfc8_FUN_0076bfc8.c:203` |

Mô hình suy ra: khi mở sạp, client **ghi đè lưới ô hiển thị của actor cục bộ** bằng hàng của sạp
(cờ `+0x1C`, id `+0x20`), **không** đụng `+0x04/+0x07` nên không phá túi đồ; khi đóng sạp,
server phải gửi lại `17 05` để nạp lại túi. **Confidence C** (chỉ suy từ 2 call-site).

### 3.3 `TVenderItem` — sprite sạp trên map (`VMT 0x767578`)

| Offset | Kiểu | Nghĩa | Evidence |
|---|---|---|---|
| — | class | kế thừa `TSe_GrBasic` | `functions/00771728_TVenderItem.Destroy.c:30` |
| `+0x20` / `+0x24` | dword | toạ độ màn hình `x`, `y` | `FUN_007717f8…c:49,51` |
| `+0x4C` | dword | chỉ số panel UI (tra `FUN_007C9B38("Panel16")`), `-1` = không có | `FUN_007717f8…c:46-47`; khởi tạo `-1`: `functions/007716dc_TVenderItem.Create.c:44-48` |
| `+0xFC` / `+0xFD` | u8 | `x>>1`, `y>>1` (toạ độ nửa) | `FUN_007717f8…c:52-61` |
| `+0xF0` | dword | **uid chủ sạp** (so với `actor+0x04` để biết sạp này có phải của mình) | `FUN_0071e164…c:66`, `FUN_00771774…c:31` |

Vòng đời: tạo lười trong `FUN_0071E164` (`functions/0071e164_FUN_0071e164.c:62-65`),
xoá trong `FUN_0071FA14` (`functions/0071fa14_FUN_0071fa14.c:23-27`).
**Vị trí world do client tự tính** từ `actor+0x54`/`+0x58` (`functions/0071f744_FUN_0071f744.c:51-70`) ⇒
**server KHÔNG gửi toạ độ sạp** — Confidence A.

### 3.4 `TCY_VenderMenu` — `gvar_007DA4F8`, VMT `0x608368`, ctor `FUN_0060CD78`

| Offset | Kiểu | Nghĩa | Evidence |
|---|---|---|---|
| `+0xE0` | byte | chế độ (mode) form | `functions/0060f860_FUN_0060f860.c:21` |
| `+0x20C` | byte | **`stall_id`** (từ `17 1E`) | `client_pseudo_c/00730f4c…c:164` |
| `+0x210` | dword | handle nhãn `"Stall"+stall_id` | `client_pseudo_c/00730f4c…c:167-168` |

### 3.5 `TCY_BuyVenderMenu` — `gvar_007DA2F8`, VMT `0x60868C`, ctor `FUN_0060F89C`

| Offset | Kiểu | Nghĩa | Evidence |
|---|---|---|---|
| `+0x134` | byte | **chế độ mua**, gán ngay trước khi gửi C→S | `functions/00611330_FUN_00611330.c:51` |
| `+0x138` | `TCY_SelectText` | bộ chọn **11 mức** (`0..10`) = số lượng | `functions/0060f89c_FUN_0060f89c.c:143-158` |
| `+0x13C` | `TSe_ScrollButton` | thanh cuộn | `…c:136-141` |
| `+0x140` | button | `"btn_Buy"` → handler `LAB_0060FED0` | `…c:183-189` |
| `+0x144` | button | `"btn_BuyAll"` → handler `LAB_0060FF7C` | `…c:190-196` |
| `+0x148` | `TSe_Editor` | ô nhập số lượng/giá | `…c:172-182` |
| `+0x14C` | button | `"btn_close_s"` | `…c:126-135` |
| `+0x150` | button | `"btn_Close"` → `Close` (VMT[9]) | `…c:197-204` |
| `+0x154` / `+0x158` | button | `"arrow_up1"` → `LAB_0061020C` / `"arrow_dw1"` → `LAB_00610160` | `…c:160-171` |
| `+0x15C` | byte | **index hàng đang chọn**, guard `≤ 9` | `…c:186-196` không set; đọc: `FUN_00611330…c:31,51` |
| `+0x164` | `entry[10]`, **stride `0x11` = 17 byte** | mỗi dòng hàng của form mua | `FUN_00611330…c:35,59` |
| `entry[i]+0x00` | byte | loại hàng; `== 0x1A` → reset index về 1 sau khi mua | `FUN_00611330…c:59-60` |
| `entry[i]+0x04` | dword | **giá** (so với `actor+0x12F8`) | `FUN_00611330…c:39` |
| `+0x210` | dword | handle icon `"OnePrice"` | `…c:95-96` |

**Confidence A** cho offset, **B** cho ngữ nghĩa (`entry` là bảng chưa được khởi tạo bởi ctor →
do `17 21` điền).

---

## 4. Luồng mua từ stall

### 4.1 Client → Server

1. Người chơi click nút `btn_Buy` (`+0x140`) hoặc `btn_BuyAll` (`+0x144`).
   Handler lồng nhau `LAB_0060FED0` / `LAB_0060FF7C` **không được decompile**, nhưng chúng
   **tail-call** `FUN_00611330(self, mode)`:
   `References to entry` của `FUN_00611330` → `0060ff72`, `00610028`
   (`functions/00611330_FUN_00611330.c:16-17`), tức đúng trong thân hai handler kia.
2. `FUN_00611330(TCY_BuyVenderMenu, mode)` — `functions/00611330_FUN_00611330.c:30-67`:
   ```c
   if (*(char *)(*(int *)gvar_007D9D88 + 4) == '\0') {          // :31  đang giao dịch → toast
       i = form[0x15C];                                        // :37  hàng đang chọn (0..9)
       price = *(int *)(form + 0x168 + i*0x11);                 // :44  = entry[i]+0x04
       if (*(int *)(actor + 0x12F8) < price)  → toast DAT_00611484;  // :45-46  không đủ tiền
       else if (FUN_00768d38() == 0)           → toast DAT_006114A0;  // :49-50  hết ô trống
       else {
           *(byte *)(form + 0x134) = mode;                     // :51  ★ mode = sub-op
           FUN_0077f414(*(undefined4 *)gvar_007D9D30, 0x17);    // :52  ★ gửi main op 0x17
           if (form[0x164 + i*0x11] == 0x1A) form[0x15C] = 1;  // :57-58
       }
   } else toast DAT_00611460;                                   // :63-65
   ```
   - `FUN_00768D38()` = quét ô trống đầu tiên trong lưới `1..25` (`+0x9E0`, `cell+7 == 0`) —
     `functions/00768d38_FUN_00768d38.c:34-47`.
   - `FUN_00768D94(itemId)` = tìm ô chứa `itemId` — `functions/00768d94_FUN_00768d94.c:55-77`.
3. `FUN_0077F414(codec, 0x17)` = builder C→S (mục 5).

**Payload `17 ??` (buy): UNDETERMINED** — builder tại `0x00784261` (sub `0x21`) chưa decompile.
Chỉ biết chắc: **main op `0x17`**, và có **một sub-op byte** do `form[0x134]` quyết định.
Rust đang dùng sub `33 (0x21)` + `payload[4]=index, payload[5]=count` → **Confidence C, cần xác minh thực tế**.

### 4.2 Server → Client

| Bước | Frame | Nội dung | Conf. |
|---|---|---|---|
| 1 | `17 1F` | spawn avatar+sạp của người bán | C |
| 2 | `17 1E` | danh sách `(slot, item_id)` — **không giá** | **A** |
| 3 | `17 21` | catalog **có giá** → điền `entry[i]+0x04` trong form mua | **B** |

Vì `17 1E` không mang giá mà form mua lại so `actor+0x12F8` với `entry[i]+0x04`
(`FUN_00611330…c:39`), **giá bắt buộc phải đến ở frame `17 21`** → Confidence B.

### 4.3 Hết hạn sạp

`FUN_0060E2A8(TCY_VenderMenu, RP)` (`functions/0060e2a8_FUN_0060e2a8.c:40-52`):
`Copy(RP,2,8)` ⇒ **`payload[3..10]` = khối 8 byte** decode bằng `FUN_0077F098`, so với `Now()`;
nếu đã hết hạn → toast `DAT_0060E39C` + timestamp 5000 ms.
⇒ server gửi một frame **8 byte mốc thời gian** cùng cửa sổ sạp. Handler nhận là
`sub_0078d82f` (chưa xác định main op) → **Confidence C**.

---

## 5. C→S: dispatcher, subop, và một kết luận cũ bị bác bỏ

### 5.1 `FUN_0077F414(codec, mainOp)` — builder C→S

`functions/0077f414_FUN_0077f414.c:684,812` — `switch (param_2 & 0xff)` theo **main opcode**,
rồi `TForm1_CY_AddSedQueue` (`0x0051633C`, `functions/0051633c_TForm1.CY_AddSedQueue.c:141`).
Chuỗi frame được dựng bằng **AnsiString** (mỗi byte 1 ký tự):
`FUN_0077EB1C(_, u16, &s)` → Word LE 2 byte; `FUN_0077EE84(_, u32, &s)` → DWORD LE 4 byte
(`functions/0077eb1c_FUN_0077eb1c.c:47-70`, `functions/0077ee84_FUN_0077ee84.c:69-97`).

Bảng ánh xạ main-op → code C→S:
- `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex` → `byte200_cs[mainOp] = caseIndex`
- `client_pseudo_c/table_0x77F53C_dword200.hex` → `dword200_cs[caseIndex] = code address`
- ⇒ **main op `0x17` → `byte[0x17]=0x15(21)` → `dword[0x15]=0x007833F0`** — Confidence A

`client_pseudo_c/cs_op17_dispatch.asm.txt:5-8`:
```
007833f0: XOR EAX,EAX
007833f2: MOV AL,byte ptr [EBP + -0x6]      ; ← sub-op byte
007833f5: CMP EAX,0x3c
007833f8: JA  0x0078a4f2
007833fe: JMP dword ptr [EAX*0x4 + 0x783405]
```
⇒ **C→S main op `0x17` có 61 sub-op `0x02..0x3C`**, thân mỗi case nằm trong
`client_pseudo_c/cs_op17_targets_0x783405.hex`.

> 🔴 **ĐÍNH CHÍNH `opcode_17.md` §5.** Tài liệu đó kết luận *"client không bao giờ chủ động gửi
> OP 0x17"* dựa trên `case 0x17: break;` (`0077f414…c:973-974`). **Kết luận đó SAI** — đó là
> artifact của Ghidra không gán được **indirect jump** `JMP [EAX*4+0x783405]` cho `case 0x17`.
> Bằng chứng thuận (Confidence A):
> 1. `cs_dispatch_byte200_0x77f474.hex` + `table_0x77F53C_dword200.hex` → `0x17` ⇒ `0x7833F0`.
> 2. `cs_op17_dispatch.asm.txt:5-8` + `cs_op17_targets_0x783405.hex` — 61 case body thật.
> 3. **16 call-site** gọi `FUN_0077F414(..., 0x17)`: `00771774:31` (click sạp),
>    `00611330:50` (Buy/BuyAll), `0076cebc:27`, `0076cefc:197`, `0077d3b4:356,579`,
>    `0077e4cc:122,176`, `00734610:71`, `00734834:28`, `0074927c:28`, `0074bf20:250`,
>    `0074e8fc:27`, `007467e4:70`, `0076d578:1099`, `005035d0:22`, `00792174:46`,
>    `00791ed4:65`, `00792028:61`.

### 5.2 Vị trí thân các sub-op C→S (chưa decompile) — **đã giải mã lại từ hex thô 2026-10-02**

| SubOp | VA case body | Ghi chú |
|---|---|---|
| `0x02,0x03,0x04` | `0x7834F9 / 0x78357F / 0x783635` | |
| `0x05`..`0x09` | `0x78A4F2` = **default (nop)** | |
| `0x0A`..`0x14` | `0x78371E` … `0x783F8C` | 11 sub liên tục |
| `0x15`..`0x1D` | `0x78A4F2` = **default (nop)** | 9 sub |
| **`0x1E`** | **`0x7840A7`** | ← ứng viên "mở/đóng sạp" |
| **`0x1F`** | **`0x78418C`** | |
| **`0x20`** | **`0x7841E1`** | |
| **`0x21`** | **`0x784261`** | ← **ứng viên mạnh nhất cho "buy"** (§4.1) |
| `0x22`..`0x3C` | `0x78434F` … `0x7851D1` | 27 sub liên tục |

Nguồn: 244 byte của `cs_op17_targets_0x783405.hex` = **61 dword** (`sub 0x00..0x3C`),
dispatch lấy thẳng `EAX*4 + 0x783405` (`cs_op17_dispatch.asm.txt:5-8`, **không** có byte map
trung gian). **Mức A.**

`0x0078A4F2` là **default của `FUN_0077f414`** (asm `0078a4f2: XOR EAX,EAX` rồi `RET`)
⇒ default = **không phát frame**.

> 🔴 **ĐÍNH CHÍNH — claim "client KHÔNG gửi `17 1E/1F/20`" BỊ BÁC BỎ (2026-10-02).**
> `03_trade_and_stall.md` §A.4.1/§A.4.2 (nguồn hợp nhất) kết luận 3 sub đó là default ⇒
> client không gửi, và suy ra `17 1E/1F/20` không thể là frame stall C→S. **Giải mã lại
> 244 byte hex cho thấy ngược lại:** `0x1E → 0x007840A7`, `0x1F → 0x0078418C`,
> `0x20 → 0x007841E1` — **cả 3 ĐỀU CÓ builder thật**. Sub thực sự là default là
> **`0x15..0x1D`** cộng `0x00,0x01,0x05..0x09` (**16 sub**).
> ⇒ **Gap G3 ở §6.2 vẫn hợp lệ** — client **có** thể gửi `0x1E/0x1F/0x20`; chỉ là **thân
> builder chưa có trong dump** nên chưa biết payload. Cũng cần lưu ý bản gốc gán
> `0x21 → 0x007840A7` và lệch 3 sub trong nhóm `0x1E..0x3C` — bảng trên đã sửa theo hex thật.

**Thân builder vắng mặt — Mức A (về sự vắng mặt):** dải `0x00783403 … 0x00785225` **không có
trong bất kỳ file `.asm.txt` nào** của `client_pseudo_c/` (đã quét toàn bộ;
`0077f414_FUN_0077f414.asm.txt` nhảy thẳng từ `0x007833FE` sang `0x00785226`).
`test/ts_decompile/index.csv` cũng không có entry nào trong khoảng `0x783300..0x788700`
⇒ **không thể decompile thêm ngoài dump hiện có** (`aLogin.exe` không nằm trong workspace).

**Hướng tìm tiếp (cụ thể):** disassemble `FUN_0077f414` tại `0x0078340A … 0x007850C8`
(≈ 7 KB), rồi đọc block `0x007840A7` (sub `0x1E`) và `0x00784261` (sub `0x21`). Kiểm tra
xem chúng có đọc `*(byte*)(*gvar_007DA4F8 + 0x20C)` (`stall_id`) và
`*(void**)(*gvar_007DA4F8 + 0x210)` (component `"Stall"+id`, §2.2) không.

### 5.3 Mẫu "điền request struct rồi gửi"

`functions/0076cebc_FUN_0076cebc.c:23-28` — mẫu điển hình, mỗi sub-op đọc một struct riêng:
```c
obj[6] = b2; obj[7] = b3; obj[8] = b4;
FUN_0077f414(*(undefined4 *)gvar_007D9D30, 0x17);
```
⇒ server phải tự suy ra sub-op từ **struct mà client vừa điền**, không có bảng tra cứu
trong binary. Đây là lý do sub-op C→S **không đoán được từ tĩnh** — Confidence A về cơ chế.

### 5.4 Trạng thái "mở sạp của tôi"

- `FUN_0060ECDC(TCY_VenderMenu)` (`functions/0060ecdc_FUN_0060ecdc.c:18-51`) = thao tác
  **mở form sạp của chính mình**: kiểm tra cờ bận `actor+0x35F`, `actor+0x378`,
  map id `actor+0x63A`, rồi `VMT[8]()` (Show). **Không gửi gói nào.**
- `FUN_0060E2A8` = kiểm tra hạn sạp (mục 4.3).
- Cờ `actor+0x5C0` chặn một số hành động khác, ví dụ sub `0x5B` của
  `functions/0076d578_FUN_0076d578.c:617-620` → toast "đang bán hàng".

---

## 6. Gap so với Rust server + đề xuất implement

### 6.1 Frame server **phải phát** (đề xuất chuẩn hoá)

```text
S→C 17 1E   [name_len:1][name:name_len][stall_id:1][(slot:1, item_id:4 LE)]×N
S→C 17 20   [pad:1][vendor_uid:4 LE]                ← 1 byte pad BẮT BUỘC (mục 6.2 #2)
S→C 17 21   [17 × 0x00] + mỗi món: id2 count2 price4 long1 giatri1 khang1 texp4 idx1
S→C 17 1F   [pad:1][uid:4][name_len:1][name][image:1]      (chưa xác minh)
```

### 6.2 Gap cụ thể (5 gap lớn nhất)

| # | Gap | Vị trí Rust | Mức độ | Hành động đề xuất |
|---|---|---|---|---|
| **G1** | `17 1E` **không mang giá**. Rust ghi `image` + `(slot, price4)`; client thật đọc `stall_id` + `(slot, item_id4)` | `shops.rs:474-482` (S→C), `shops.rs:435-467` (C→S) | ** Cao** | Đổi tên `image → stall_id`, `price → item_id`; giá phải đi qua `17 21`. Nếu server dùng `17 1E` để đẩy giá thì form mua sẽ **luôn hiện giá 0 / so sánh `actor+0x12F8 < 0` → sai**. |
| **G2** | `17 20` **lệch 1 byte**. `FUN_00731388` đọc `Copy(RP,2,4)` = `payload[3..6]`, Rust gửi `1720`+uid4 ⇒ uid bị dịch 1 byte | `shops.rs:500` (`F4440600 1720 <uid4>`) | ** Cao** | Sửa thành `frame("1720", &format!("00{}", le32(id)))`. Cần test: xác nhận bằng packet capture. |
| **G3** | C→S sub-op thật của luồng stall **chưa biết**. Rust dùng `30/31/32/33` = `0x1E/0x1F/0x20/0x21`. `0x1E` (open my stall) **trùng** với sub-op mà client dùng cho *remote* stall view (`FUN_00771774`) | `shops.rs:421-593` | **Cao** | Xác minh bằng log thật. Đề xuất tạm: giữ nguyên nhưng thêm feature-flag và **fallback thử dải sub-op** `0x1E..0x3C`. Lưu ý sub `0x21` C→S = `0x784261` — ứng viên mạnh nhất cho "buy". **(2026-10-02: đính chính — `0x1E/1F/20` CÓ builder thật, xem §5.2; gap này vẫn đúng vì payload chưa đọc được)** |
| **G4** | `17 21` body `17 zero bytes` đầu frame là **kinh nghiệm**, không có bằng chứng | `shops.rs:270` | Trung bình | Cần chốt độ dài thật của prefix bằng capture; nếu `FUN_00610290` đọc `Copy(RP,2,len)` thì prefix phải đúng số byte. |
| **G5** | Thiếu **hạn sạp** (8-byte timestamp) và thiếu refresh giá sau khi bán hết 1 món | `shops.rs` (không có) | Trung bình | Thêm trường `expires_at` vào `ShopState`; phát frame 8 byte + `17 21` lại sau mỗi giao dịch. |

### 6.3 Đề xuất code cụ thể

```rust
// shops.rs — S→C mở sạp (đúng nghĩa client)
pub fn stall_open_frame(
    vendor_name: &[u8], stall_id: u8,
    items: &[(u8, u32)],          // (slot, item_id)
) -> String {
    let mut body = String::new();
    body.push_str(&format!("{:02X}", vendor_name.len()));
    body.push_str(&encoder::strhex(vendor_name));
    body.push_str(&format!("{:02X}", stall_id));
    for (slot, item_id) in items {
        body.push_str(&format!("{:02X}", slot));
        body.push_str(&encoder::le32(*item_id));
    }
    crate::protocol::frame("171E", &body)
}

// shops.rs — S→C đóng sạp: BẮT BUỘC 1 byte pad trước uid
pub fn stall_close_frame(vendor_uid: u32) -> String {
    crate::protocol::frame("1720", &format!("00{}", encoder::le32(vendor_uid)))
}
```

Server cần parse thêm (khi nâng cấp luồng stall hiện tại):
- `17 1E` inbound không tồn tại (S→C chỉ) — không cần parse.
- `17 20` inbound (C→S): `payload[0..4]` = vendor uid — Rust hiện dùng `payload[0..4]` ✓
  (`shops.rs:506-507`) nhưng **nếu client thật gửi sub `0x20` với 1 byte pad thì lệch** — cần capture.
- `17 21` inbound: `payload[4]` = index hàng, `payload[5]` = số lượng (`shops.rs:533-534`) — Confidence C.

---

## 7. Undetermined

| # | Câu hỏi | Vì sao chưa biết | Cần gì |
|---|---|---|---|
| U1 | Nội dung byte-exact của `C→S 17 1E/1F/20/21 …` | Thân case nằm ở `0x7840A7+`, **không có trong dump** (`index.csv` rỗng vùng `0x7833–0x7887`); không có `aLogin.exe` để disassemble lại | Re-dump Ghidra theo `cs_op17_targets_0x783405.hex`, hoặc packet capture thật |
| U2 | Sub-op C→S thật cho "mở/đóng sạp của tôi", "đặt giá", "bỏ hàng" | Cùng lý do U1 | idem |
| U3 | `17 1F` payload | Handler `FUN_007311C8` chưa decompile | idem |
| U4 | `17 21` payload chính xác (prefix length, số bản ghi) | Handler `FUN_00610290` chưa decompile | idem |
| U5 | `17 22` payload | Handler `FUN_0060E3CC` chưa decompile | idem |
| U6 | Giá trị `mode` truyền vào `FUN_00611330` từ `btn_Buy` vs `btn_BuyAll` | Nested handler `LAB_0060FED0` / `LAB_0060FF7C` chưa decompile | idem |
| U7 | Frame mang **hạn sạp** 8 byte — main op / sub op nào | Chỉ biết callee `FUN_0060E2A8`; caller là `sub_0078d82f` (chưa xác định main op) | Redump dispatcher `0x78D4AF` |
| U8 | `TItemCell` là lưới ô nào (túi đồ hay lưới sạp) | Suy từ 2 call-site | Redump `TItemCell` class (`VMT` → field table) |
| U9 | Text 3 toast của form mua (`DAT_00611484/60/4A0`) | Không có `lit_*.hex` cho `0x611460–0x6114C0` | Redump literal range `0x611460..0x6114C0` |
| U10 | Toạ độ world của sprite sạp | Client tự tính từ `actor+0x54/+0x58`; server không cần gửi | — |
| U11 | Nội dung `FUN_0071E164` (dựng form stall) trong bộ curated `client_pseudo_c/` | Không có body — chỉ có call-site `00730f4c…c:23` và `:162`; §2.2 chỉ dựa trên call-site + asm `00730f4c…asm.txt:131-138` | Re-dump `0x0071e164` (bản đầy đủ có ở `test/ts_decompile/functions/`) |
| U12 | `TVenderManage.Create @ 0x0075C004`, `TVenderItem.Create @ 0x007716DC`, `TVenderItem.Destroy @ 0x00771728` trong curated dump | Chỉ có dòng `index.csv:5585/5677/5678`, không file `.c` | Re-dump 3 hàm này |
| U13 | Cấu trúc record giá trong frame danh sách hàng (nhiều khả năng 16 byte như Bear `:119-148`) | **Mức C** — chỉ có Bear, không phải `aLogin.exe` | Body `0x00610290` (§2.4) |
| U14 | `17 1F` / `17 22` có phải frame giá không | Cả hai đều không có body | Re-dump `0x007311c8`, `0x0060e3cc` |

---

## Source trail

| Loại | File |
|---|---|
| S→C `17 1E` handler | `ts_dream/client_pseudo_c/00730f4c_FUN_00730f4c.c:32-175` (+ `.asm.txt:5-173`) |
| S→C `17 20` handler | `test/ts_decompile/functions/00731388_FUN_00731388.c:47-64` |
| Dispatcher S→C op `0x17` | `ts_dream/client_pseudo_c/007902fb_FUN_007902fb.c:284-303`, `case_020_007902FB_FUN_007902fb.c:29-33` |
| Bảng nhảy S→C | `ts_dream/client_pseudo_c/cs_op17_targets_0x783405.hex` |
| Dispatcher C→S | `test/ts_decompile/functions/0077f414_FUN_0077f414.c:684,812,973-974` |
| Bảng nhảy C→S | `cs_dispatch_byte200_0x77f474.hex` + `table_0x77F53C_dword200.hex` + `cs_op17_dispatch.asm.txt:5-8` |
| Hàng đồ client | `functions/00768638_FUN_00768638.c:26-30`, `functions/005335e8_FUN_005335e8.c:53-105` |
| Sprite sạp | `functions/0071e164_FUN_0071e164.c:54-66`, `functions/0071fa14_FUN_0071fa14.c:23-27`, `functions/007717f8_FUN_007717f8.c:40-63` |
| Click sạp (C→S) | `functions/00771774_FUN_00771774.c:23-36` |
| Form mua (ctor + buy) | `functions/0060f89c_FUN_0060f89c.c:38-214`, `functions/00611330_FUN_00611330.c:29-65` |
| Form sạp của tôi | `functions/0060ecdc_FUN_0060ecdc.c:18-51`, `functions/0060e2a8_FUN_0060e2a8.c:40-52` |
| Hạn sạp | `functions/0060e2a8_FUN_0060e2a8.c:41-49` |
| Rust server | `ts_dream/src/server/handlers/shops.rs:262-594`, `ts_dream/src/server/dispatcher.rs:194-229` |