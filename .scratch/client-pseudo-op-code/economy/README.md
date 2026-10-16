# Nghiên Cứu Kinh Tế Game — Mua / Bán / Giao Dịch / Tiền Tệ (Client `aLogin.exe` ↔ Server Rust)

Ngày: 2026-10-02 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Nguồn chính: `client_pseudo_c/` (Ghidra pseudo-C + ASM + HEX) · Đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`
Tài liệu nền (nối tiếp): [`../post_login_stats_inventory_flow.md`](../post_login_stats_inventory_flow.md) · [`../battle/README.md`](../battle/README.md)

> **Mục tiêu:** đủ bằng chứng để hoàn thiện server `ts_dream` cho chức năng mua bán, giao dịch,
> sử dụng vàng / tiền trang / tiền mềm / điểm để mua vật phẩm.

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — U3/U4/U5 đóng, 1 blocker memory**

---

## 0.1 ROUND-4 REDUMP — ghi chú cho bộ tài liệu economy

### 0.1.1 Những gì ĐÃ ĐÓNG (không cần redump thêm)

| Mục | Kết quả |
|---|---|
| `05:493` (U9) — bảng 9 nhánh C→S `0x1E` | ✅ **866 B / 866 B.** `sub 0` = epilogue ⇒ **8 nhánh thật** |
| `04:111-115` (U3/U4/U5/U14) — handler `17 1F`/`17 21`/`17 22` | ✅ đã có body |
| `05:485` (U1) — `TFRollEgg` caller `0x00772788` | ✅ đã có body |
| `04:536` (U11) — `FUN_0071E164` | ✅ đã có body (tài liệu ghi sai) |
| `04:537` (U12) — 3 class vendor `0x0075C004`/`0x007716DC`/`0x00771728` | ✅ đã có body (tài liệu ghi sai) |
| `02:29,638` — "13 file `functions/*.c` thiếu" | ❌ **SAI** — `ts_decompile/functions/` có **13 218 file** |
| `03:684` (Undet.3b) — `LAB_00609218` | ✅ đã có body |
| `03:646,694` (G9) — layout `17 06` | ✅ **14 B, byte-exact** |
| `03:691,692` (Undet.9/10) — 15 literal `19 0B`/`19 02` | ✅ đã dump hex |
| `02:608` — `DAT_005D75D0` | ✅ đã dump hex |

### 0.1.2 🔴 BLOCKER duy nhất còn lại: memory ngoài `0x7DA800`

Đo từ 2 dump đọc được một phần:

| File | Bắt đầu | Đọc được | Kết thúc |
|---|---|--:|---|
| `lvlcoef_7da6b8_wide.hex` | `0x007DA6B8` | 328 B | `0x7DA7F7` |
| `table_7da6dc_101DWORD.hex` | `0x007DA6DC` | 292 B | **`0x7DA7FF`** |

⇒ **Toàn bộ địa chỉ ≥ `0x7DA800` đọc được 0 byte.** Bị chặn:

| Mục | Địa chỉ |
|---|---|
| `05:486` (U2) — thang cược `RG.Dat` | `0x009D1610`, `0x009D1620` |
| `light_dark §6 #4` — cây kỹ năng 765 B | `0x0094B440` |
| `light_dark §6 #5` — bảng `skillId→index` | `0x009AE128` |
| Round-3 §5.4 (cũ) | `0x948DF8`, `0x9490B0`, `0x98BE04` — đều `readable 0` |

**Không phải lỗi script.** Cần kiểm tra Window → Memory Map của project Ghidra;
nếu không có block phủ `0x94B440` thì phải **re-import `aLogin.exe`**.

### 0.1.3 ⚠️ Sai lệch địa chỉ HOLE

| Tài liệu | Ghi | Thật |
|---|---|---|
| `02:281,603,605` | HOLE `0x5DC96A..0x5DCE0` | **`0x5DC98A..0x5DCCDF`** (854 B) |
| `README:243` | `0x784000..0x785000` ≈ "7 KB" | span đó chỉ **4 096 B**; 7 KB là `0x783403..0x785225` |

### 0.1.4 ⚪ Không giải được bằng redump — cần traffic thật

| # | Câu hỏi | Tài liệu |
|:-:|---|---|
| C1 | thứ tự `1A 01` / `1A 04` khi bán | `02:610` |
| C2 | client có gửi `1B 01 00` không | `02:611` |
| C3 | `17 20` có 1 byte pad thật không | `04:484` |
| C4 | sub-op C→S thật của stall | `04:485` |
| C5 | độ dài thật của prefix `17 21` | `04:486` |
| C6 | tên nghiệp vụ tiền #2 (`+0x12FC`) | `05:495` |
| C7 | `+0x1300` là tiền gì | `06:361` |
| C8 | `shop_point` (`42 02`) có hiển thị không | `06:370` |

## 0. Cách đọc bộ tài liệu này

| File | Nội dung |
|:--|:--|
| [`01_shop_read_flow.md`](./01_shop_read_flow.md) | **Dữ liệu shop đến từ đâu.** `eve.emg` + `Item.dat` cục bộ client; layout `17 21` (shop người chơi); mở/đóng form |
| [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) | **Mua/bán NPC shop**: `1B 01` / `1B 02` / `1B 03`, bảng 12 thông báo VISCII, frame túi `17 06` / `17 09`, timeline đầy đủ |
| [`03_player_trade.md`](./03_player_trade.md) | **Giao dịch 2 người** (main `0x19`), pet-trade, trade với NPC, hủy/timeout |
| [`04_stall_vendor.md`](./04_stall_vendor.md) | **Stall / gian hàng** (`17 1E`), form stall, luồng BuyVender |
| [`05_special_economy.md`](./05_special_economy.md) | **Bank** (`1D`), **vòng quay / Trân Bảo Các** (`TFBlissBag`, `TFRollEgg`), **Lotto/Xì phé/Domino/Cờ tướng** (`36`/`3A`/`3B`/`3C`), store, `1E xx` storage |
| [`06_currency_model.md`](./06_currency_model.md) | **Mô hình tiền tệ**: 3 field tiền + 3 bộ đếm, offset, trần, guard overflow |
| [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) | **Audit 100% phía Rust**: schema, state, persistence, 12 bẫy/rủi ro, kế hoạch test |
| [`08_verification_report.md`](./08_verification_report.md) | **Đối chiếu độc lập** 3 claim đảo ngược thiết kế hiện tại |

**Mức bằng chứng:** **(A)** đọc thật byte từ body/ASM client · **(B)** suy luận có cơ sở ·
**(C)** chỉ là contract phía Rust hoặc suy đoán.

> **⚠ Cảnh báo phương pháp (đã vi phạm nhiều lần trong quá khứ).** `case N: break;` rỗng của
> Ghidra **KHÔNG** phải bằng chứng "client không làm gì" — đó là lỗi recovery. Cách kiểm chứng
> đáng tin: tra bảng builder `client_pseudo_c/cs_op17_targets_0x783405.hex`,
> `cs_op19_byte_map_0x7853e5.hex`, `table_0x77F474_200B.hex` xem target có trỏ về epilogue `RET`
> hay không. Xem `08_verification_report.md` §1d — cả 3 claim sai đều bị phát hiện nhờ phương
> pháp này.

---

## 1. Tóm tắt điều hành — 10 phát hiện quan trọng nhất

| # | Phát hiện | Mức | Ảnh hưởng |
|:-:|:--|:-:|:--|
| **H-1** | **Danh sách hàng shop NPC và GIÁ không bao giờ đi trên wire.** Client tự parse `Data\Eve.emg` (5 byte/mục, `itemId` = Word LE) + `Data\Item.Dat` (`price`/`sell_price`). Server chỉ nhận `1B 01 <index:u8> <count:u16 LE>`. | **A** | 🔴 **Đảo ngược thiết kế**: bảng giá hard-code 110 dòng của Rust là sai về kiến trúc |
| **H-2** | **Rust không bao giờ gửi `1B 03`** (`grep` trong `src/` = 0 hit) → form shop NPC **không bao giờ mở**. | **A** | 🔴 Chặn toàn bộ tính năng |
| **H-3** | **Rust bỏ qua `ctx.sub`** (`shops.rs:171-173`) → `1B 02` (**bán**) bị xử như mua. | **A** | 🔴 Mất tiền / sai vật phẩm |
| **H-4** | **Rust bỏ qua `itemIndex`** (`payload[1]`) và **cứng số lượng = 1** → server không biết mua món nào, bao nhiêu. | **A** | 🔴 |
| **H-5** | **Giá bán sai hệ thống**: `gold += count` (1 vàng/món) thay vì `count × sell_price` (`shops.rs:42`). | **A** | 🔴 Sai số hệ thống |
| **H-6** | **Có 3 field tiền, không phải 2.** `+0x12F8` (kim, trần 9 999 999) · `+0x12FC` (tiền trang, trần 9 999 999, set qua `1D 04`) · `+0x1300` (tiền mềm, trần 2e9). Rust **không có field nào cho `+0x1300`** và luôn gửi `B=0` trong `1A 04` ⇒ **ghi đè tiền mềm về 0 ở login và sau mỗi giao dịch**. | **A** | 🔴 Mất tiền mềm |
| **H-7** | **`1A 04` (set tuyệt đối) KHÔNG clamp**; frame delta **từ chối** chứ không clamp. Gửi giá trị > 9 999 999 ⇒ mọi `1A 01`/`1D 01` sau đó fail **im lặng** vĩnh viễn. | **A** | 🔴 Server phải tự clamp |
| **H-8** | **`23 04` KHÔNG phải store HP/SP.** Nó chỉ ghi `player+0x146C = A*100+B` + banner 10 000 ms. Rust gửi 3× ở `spawn.rs:592-594` với `god=0, hp=10000, sp=10000` ⇒ ghi đè counter bằng giá trị vô nghĩa và spam 3 banner 10 giây mỗi lần login. | **A** | 🟡 |
| **H-9** | **Client đã có đầy đủ UI cờ bạc**: Lotto `36 01/02/03/05` (lưới 20×5 + jackpot), Xì phé `3A 01`, Domino `3B`, Cờ tướng `3C`, `TSportManage` `39`, `TFBlissBag` (Trân Bảo Các), `TFRollEgg` (vòng quay trứng), `1E 0C/0D` (thủy binh bank). Rust: **không có gì**. | **A** | 🟢 mở rộng |
| **H-10** | **`index` trong `1B 01` là 0-based** (client) — nhưng server C# Bear `NpcShopsHandler.cs:46` dùng `items[index-1]` (1-based) và `:14` đọc **1 byte** trong khi client gửi **2 byte**. **Đừng copy C# mù.** | **A** | 🔴 nếu bê nguyên xi |

---

## 2. Mô hình tiền tệ (đã kiểm chứng)

Ba field tiền liên tiếp trong `player` (`**gvar_007DA7BC`), tất cả `int32`:

| Offset | Tên nghiệp vụ | Trần | Set tuyệt đối | Delta ± | Hiển thị ở |
|:--|:--|:--|:--|:--|:--|
| `+0x12F8` | **Kim / tiền mang theo** | `9 999 999` (`0x98967F`) | `1A 04` (ô A) | `1A 01` / `1A 02` | HUD `btn_001`, `TMoneyAccountForm+0x118` |
| `+0x12FC` | **Tiền trong trang / bank** | `9 999 999` | **`1D 04`** | `1D 01` / `1D 02` | `Form_Bank_Money` (nhãn trái) |
| `+0x1300` | **Tiền mềm / cash point** | `2 000 000 000` (`0x77359400`) | `1A 04` (ô B) | `1A 05` / `1A 06` | HUD (vẽ tay) |

Ba bộ đếm tài nguyên (set tuyệt đối qua `08 06`, **chỉ cộng** qua `1A 08/09/0A`):

| Offset | Tên | Cap |
|:--|:--|:--|
| `+0x0502` | Thủy binh | `+0x0500` |
| `+0x050E` | Điểm đạn dược | `+0x050C` |
| `+0x0513` | Tài bảo | `250` (cứng) |

> Chi tiết đầy đủ (36 call-site, literal VISCII, guard): [`06_currency_model.md`](./06_currency_model.md).

---

## 3. Bảng frame kinh tế tổng hợp

### 3.1 C→S (client → server)

| Frame | Payload byte-exact | Ý nghĩa | Mức |
|:--|:--|:--|:-:|
| **`1B 01`** | `1B 01 <index:u8> <count:u16 LE>` (5 B) | **MUA** — `index` entry trong list shop (0-based), `count` số lượng | **A** |
| **`1B 02`** | `1B 02 <slot:u8> <qty:u8>` (4 B) | **BÁN** — `slot` 0..24 trong túi, `qty` | **A** |
| `1B 03` | *(không có builder)* | Client **không** gửi; đây là frame **S→C** mở form | **A** |
| `14 09` | `14 09` | Đóng form shop / `TInterForm` | **A** |
| `1D 01` | `1D 01 <amount:u32 LE>` (6 B) | Nộp tiền vào kho (nghĩa sub **chưa map xong**) | **A**/B |
| `1D 02` | `1D 02 <amount:u32 LE>` (6 B) | Rút tiền khỏi kho | **A**/B |
| `17 21` | `17 21 ?(4) <index:u8> <count:u8>` | Mua từ shop người chơi | **B** |
| `19 01` | `19 01 <partner_id:u32 LE>` | Mở form trade item với người chơi | **A** |
| `19 0A` | `19 0A <pet_id:u32 LE>` | Mở form trade **thú** | **A** |
| `19 02` | `19 02 <code>` | Toast 3000 ms (mã xem `03_player_trade.md`) | **A** |

### 3.2 S→C (server → client)

| Frame | Payload byte-exact | Ý nghĩa | Mức |
|:--|:--|:--|:-:|
| **`1B 03`** | `1B 03` (2 B) | **MỞ form shop NPC** (`TThingSaleForm`) | **A** |
| **`1B 01 <inner>`** | 3 B | Kết quả/lỗi nhóm **mua** — 6 inner đã giải mã VISCII | **A** |
| **`1B 02 <inner>`** | 3 B | Kết quả/lỗi nhóm **bán** — 6 inner đã giải mã VISCII | **A** |
| `1A 01` / `1A 02` | `1A 0x <Δ:u32 LE>` | `+0x12F8` ± Δ + banner | **A** |
| `1A 04` | `1A 04 <kim:u32> <c2:u32>` | set `+0x12F8` + `+0x1300` (**không clamp**) | **A** |
| `1D 04` | `1D 04 <bank:u32 LE>` | set `+0x12FC` | **A** |
| `1D 05` / `1D 06` | 2 B | Mở `TMoneyAccountForm` / `TThingAccountForm` | **A** |
| **`17 06`** | `17 06 <itemId:u16> <qty:u16> 00 <long> <giá> <kháng> <exp:u32>` (12 B) | **Item vào túi** sau khi mua | **B** (3 call-site C# khớp) |
| **`17 09`** | `17 09 <slot:u8> <qty:u8>` (3 B) | **Xoá item khỏi túi** sau khi bán | **B** |
| **`17 1E`** | `17 1E <nlen:1><name:nlen><stall_id:1>` + `(slot:1, id:u32 LE)×5` | **Stall listing** (KHÔNG có giá) | **A** |
| `17 21` | header 19 B + `n×16 B` | Shop người chơi **có giá** | **B** |
| `19 03` | `19 03 <gold:u32>` + `n×11 B` | Danh sách item trong trade | **A** |
| `1E 01` | `1E 01` + `n×12 B` | Container `+0xA48` (50 ô) | **A** |
| `1E 0C` | `1E 0C` + `14 B`/record | Thủy binh bank `+0xDC0` (25 ô) | **A** |
| `23 04` | `23 04 <A:u32> <B:u32> <D:double>` (17 B) | ⚠ **không phải store** — `+0x146C = A*100+B` + banner 10 s | **A** |
| `36 01/02/03/05` | `36 01 <m:u8><k1..k5:u8>` … | Lotto: lưới 20×5, jackpot, 9 banner lỗi | **A** |
| `3A 01` | `3A 01 <c1><c2><c3><flag><token:u32>` (9 B) | Xì phé | **A** |

> Bảng đầy đủ 40+ frame (kèm `1E xx`, `3B`, `3C`, `39`, `1A 08/09/0A`): [`05_special_economy.md`](./05_special_economy.md) §4.

### 3.3 Bảng 12 thông báo kết quả / lỗi của `0x1B` (đã giải mã VISCII)

| Outer | Inner | Text hiển thị |
|:-:|:-:|:--|
| 1 | `00` | **"Giao dịch thành công"** |
| 1 | `01` | "Xin lỗi, vàng của bạn không đủ!" |
| 1 | `02` | "Xin lỗi, bảng vật phẩm của bạn đã đầy!" |
| 1 | `03` | "Xin lỗi, Số lượng vật phẩm bạn mua đã đầy!" |
| 1 | `04` | "Xin lỗi, giao dịch thất bại" |
| 1 | `06` | "Hệ thống gián đoạn giao dịch" |
| 2 | `00` | "Giao dịch thành công" |
| 2 | `01` | "Xin lỗi, bảng vật phẩm của bạn không có vật phẩm này!" |
| 2 | `02` | "Xin lỗi, Vật phẩm bạn bán không có nhiều như thế này!" |
| 2 | `03` | "Xin lỗi, giao dịch thất bại" |
| 2 | `04` | "Hệ thống gián đoạn giao dịch" |
| 2 | `06` | "Xin lỗi, vàng của bạn đã đầy" |

> Toast chỉ hiện khi `Inner != 0` ⇒ **`Inner = 0` là nhánh thành công** (im lặng).
> `Inner = 0x05` / `>= 0x07`: không có `case`, không `default` ⇒ vẫn gọi banner với biến
> **chưa khởi tạo** (hành vi = A, nội dung = C). Chi tiết: [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) §7.

---

## 4. Timeline mua hàng

```
Client                                   Server (ts_dream)
  │ click NPC "cửa hàng"
  │  ◄── 14 01 <…B2=0x06…W3=shopIdx…>      (mở TInterForm — Rust ĐÃ có, shops.rs ✔)
  ├─► 1B 01 <index:u8> <count:u16 LE>  ───►  ❌ Rust hiện đang xử sai (xem §5)
  │                                        ┌─ validate: index hợp lệ trong eve.emg?
  │                                        ├─ gold >= count × price?
  │                                        ├─ túi đủ ô trống?
  │  ◄── 1B 01 03                          │ thất bại
  │  ◄── 1A 02 <count×price:u32>           │ trừ tiền
  │  ◄── 17 06 <itemId:2><qty:2>… (12 B)   │ item vào túi
  │  ◄── 1B 01 00                          │ (thành công — im lặng)
  │                                        └─ persist
```

**Timeline bán** đối xứng: `1B 03` mở form → `1B 02 <slot> <qty>` → `1A 01 <count×sell_price>`
(cộng tiền) → `17 09 <slot> <qty>` (xoá item) → `1B 02 00`.

---

## 5. GAP — Rust còn thiếu (xếp theo mức nghiêm trọng)

### 5.1 🔴 Blocker — chặn tính năng

| # | Vấn đề | `file:line` | Cần làm |
|:-:|:--|:--|:--|
| G-1 | **Thiếu `1B 03`** — form shop không mở được | grep `src/` = 0 hit; đối chiếu `case_024…c:106-107` | Gửi `F44402001B03` khi player bấm "cửa hàng" ở menu NPC |
| G-2 | **Bỏ qua `ctx.sub`** → bán bị xử như mua | `shops.rs:171-173` | Rẽ nhánh theo `ctx.sub` |
| G-3 | **Bỏ qua `itemIndex`** → không biết mua món nào | `shops.rs:172` | Mua: `payload[1]` = index; Bán: `payload[1]` = slot |
| G-4 | **Số lượng cứng = 1** | `shops.rs:240` | Mua: `payload[2..4]` Word LE; Bán: `payload[2]` u8 |
| G-5 | **Giá bán `+count` vàng/món** | `shops.rs:42` | `gold += count × ItemDef.sell_price` |
| G-6 | **Không gửi `17 06` sau khi mua** | `shops.rs:238-264` (thiếu), serializer `shops.rs:596-609` đã đúng | Gọi `equip_item_frame()` |
| G-7 | **Không gửi `17 09` sau khi bán** | `shops.rs:27-47` | Gửi `17 09 <slot> <qty>` |
| G-8 | **Không gửi `1B 01/02 + inner`** → người chơi không thấy lỗi nào | `shops.rs:216,230,242,258` | Thay `red_message()` bằng `shop_result(sub, inner)` |
| G-9 | **`red_message` sai kênh** (`02 0B` mở form, không in text) | `shops.rs:18-22` | Bỏ, thay bằng G-8 |
| G-10 | **Không có field cho `+0x1300`**, `1A 04` luôn gửi `B=0` | `shops.rs:11-13`, `spawn.rs:561-564` | Thêm field + cột DB; gửi `B` = tiền mềm thật |
| G-11 | **Bảng giá hard-code 110 dòng** lệch với `eve.emg` thật | `shops.rs:51-160` | Dùng `EveSurfaceData` + `ItemDef.price/sell_price` |

### 5.2 🟡 Nên sửa

| # | Vấn đề | `file:line` |
|:-:|:--|:--|
| G-12 | Không clamp `≤ 9 999 999` khi cộng tiền (`1A 04` không clamp ⇒ kẹt vĩnh viễn) | `shops.rs:42` |
| G-13 | Lệch 1 index ở 3 chỗ (`1B 01 02` ≠ "túi đầy") | `system.rs:125,452`; `use_item/mod.rs:109` |
| G-14 | Bán chỉ cho 2 NPC cứng + khoảng item id cứng | `shops.rs:179-185` |
| G-15 | `23 04` gửi 3× ở login: ghi đè `+0x146C` + spam 3 banner 10 s | `spawn.rs:322-330, 592-594` |
| G-16 | Comment dispatcher nói "client never sends 0x1B" — **sai** | `dispatcher.rs:307-312` |
| G-17 | `trade_storage.rs` gộp `1 \| 10` — `19 0A` là **pet-trade**, không phải item-trade | `trade_storage.rs:206` |
| G-18 | Không gửi `17 3B` (hủy/timeout trade) | `trade_storage.rs` |
| G-19 | `bank_gold` (`1D 04`) không sync lúc login | `spawn.rs` (0 hit `1D`) |

### 5.3 Bẫy/rủi ro kiến trúc (chi tiết: `07_rust_economy_audit.md` §8)

R1 hai đường ghi DB có thể đè lằn nhau (nguy hiểng nhất) · R2 `persist_shop_transaction` ghi
`DELETE storagetype=1` · R3 `online_sessions()` là snapshot-and-overwrite · R4 `shop_points_frame`
ép `as u16` mất dữ liệu · R8 `unimplemented::handle` không trả lời ⇒ treo UI client ·
R9 `character_id_tx` tra `characters.playerid` không phải `accounts.playerid` ·
R11 `db/pool.rs::migrate()` là no-op · R12 test DB có thể đụng production.

---

## 6. Lộ trình đề xuất

**Bước 1 — làm cho shop NPC chạy được (bỏ 4 blocker)**
1. Gửi `1B 03` khi bấm "cửa hàng" (G-1) — 1 dòng, mở khóa cả tính năng.
2. Parse lại `ctx.sub` + `index` + `count` (G-2/3/4).
3. Thay bảng giá hard-code bằng `EveSurfaceData` + `ItemDef` (G-11).
4. Sửa `gold += count × sell_price` + clamp 9 999 999 (G-5/G-12).

**Bước 2 — đồng bộ client**
5. Gửi `17 06` sau mua, `17 09` sau bán (G-6/7).
6. Thay `red_message` bằng `shop_result(sub, inner)` theo bảng §3.3 (G-8/9/13).

**Bước 3 — tiền tệ**
7. Thêm field + cột DB cho `+0x1300`; gửi `B` thật trong `1A 04` (G-10).
8. Gửi `1D 04` lúc login để `Form_Bank_Money` hiển thị đúng (G-19).

**Bước 4 — stall & trade**
9. Implement `17 1E` (broadcast) + `17 21` (mua) + `17 20` (đóng) — Rust đã khớp layout.
10. Tách `19 0A` (pet-trade) khỏi item-trade; thêm `17 3B` (G-17/18).

**Bước 5 — mở rộng (tuỳ chọn)**
11. Bank `1D 01/02` + `1E 0C/0D` (thủy binh bank) — đã có đặc tả.
12. Lotto `36`, Xì phé `3A`, Domino `3B`, Cờ tướng `3C` — layout đã đoánh xong.

---

## 7. Đính chính tài liệu cũ

| File | Dòng | Sai | Đúng |
|:--|:--|:--|:--|
| `opcode_1b.md` | `:122` | "client không bao giờ gửi `0x1B` (`case 0x1b` rỗng)" | Builder thật ở `0x00785FFD`; bảng `byte[0x77F474+0x1B]=26`. Đã xác minh bằng ASM |
| `opcode_1a.md` | §4.4 | (đúng, nhưng dễ nhầm) | `+0x12FC` **có** đường ghi riêng: `1D 04` — `opcode_1d.md` đã đặc tả đúng |
| `opcode_17.md` | `:132-137` | "client không gửi OP `0x17`" | Sai — artefact Ghidra; `17 21`, `17 1E`… **có** builder |
| `src/server/dispatcher.rs` | `:307-312` | comment "aLogin never sends 0x1B" | Sai — xem trên |
| `03_trade_and_stall.md` (đã xóa) | — | "sub `0x1E/1F/0x20` trỏ epilogue RET ⇒ client không gửi" | **Sai** — giải mã 244 byte cho thấy cả 3 đều có builder |

---

## 8. Việc còn mở & Kết quả giải mã

1. ~~**Body builder `0x17` sub `0x21`/`0x22`**~~ — **ĐÃ GIẢI MÃ (Level A)**:
   - Tháo mã jump table `0x00783405` và các target builder:
     - `17 1E` (`0x007840A7`): Mở quầy (`17 1E <len:u8><name><stall_id:u8><items>`).
     - `17 1F` (`0x0078418C`): Đóng quầy (`17 1F`, 2B).
     - `17 20` (`0x007841E1`): Xem quầy người khác (`17 20 <vendor_uid:u32 LE>`, 6B).
     - `17 21` (`0x00784261`): Mua từ quầy (`17 21 <vendor_uid:u32 LE> <slot:u8> <count:u8>`, 8B).
     - `17 22` (`0x0078434F`): Refresh quầy của mình (`17 22 <stall_id:u8>`).
2. ~~**`func_0x00610290` (`TCY_BuyVenderMenu`)**~~ — **ĐÃ GIẢI MÃ (Level A)**:
   - Tìm thấy decompiled body tại `test/ts_decompile/functions/00610290_FUN_00610290.c` (3656 bytes).
   - Layout byte-exact: Header 17B (`17 21` + `vendor_uid:u32 LE` + 3x4B zero pad) + n×16B records (`slot:u8`, `itemId:u16 LE`, `count:u8`, `long:u8`, `price:u32 LE`, `giatri_long:u8`, `khang:u8`, `pad:0`, `texp:u32 LE`).
3. ~~**Sub `1D 01` = nộp hay rút?**~~ — **ĐÃ GIẢI MÃ (Level A)**:
   - Tháo mã Delphi button handlers trong `aLogin.exe`:
     - `btn_Bank_Gold_R` (`0x005D85C4`): Mũi tên PHẢI (Bank → Túi) => **C→S `1D 01` = RÚT TIỀN**.
     - `btn_Bank_Gold_L` (`0x005D87B4`): Mũi tên TRÁI (Túi → Bank) => **C→S `1D 02` = NỘP TIỀN**.
   - S→C phản hồi: `1D 01 <Δ:u32>` (cộng bank), `1D 02 <Δ:u32>` (trừ bank), `1D 03` (lỗi bank đầy/không đủ), `1D 04 <val:u32>` (set bank gold), `1D 05` (mở `TMoneyAccountForm`).
4. ~~**Vòng quay / Trân Bảo Các** (`TFBlissBag`, `TFRollEgg`, `TFWheel`)~~ — **ĐÃ LÀM RÕ (Level A)**:
   - `TFWheel`: Animation sprite chiêu thức bánh xe lửa trong chiến đấu, không phải casino.
   - `TFBlissBag`: Data loader đọc `Data\BlissBag.Dat` hiển thị preview túi may mắn (`17 15`).
   - `TFRollEgg`: Loader nạp `Data\RG.Dat` vào `DAT_009d13d0` rồi tự hủy.
   - Hệ thống cờ bạc mạng thực tế quản lý bởi `TSportManage` qua Opcode `0x39`: Panel 1=Tài Xỉu (`0x3A`), Panel 2=Domino (`0x3B`), Panel 3=Cờ tướng (`0x3C`), Panel 4=Lotto (`0x36`), Panel 6=Slot machine (`0x36`).
5. ~~**Toàn bộ frame `1E 01..0D`**~~ — **ĐÃ GIẢI MÃ (Level A)**:
   - Client có 3 container: Bank 1 (`+0xA48`, 51 slot), Bank 2 (`+0xB14`, 51 slot), Thủy binh (`+0xDC0`, 26 slot).
   - Layout: `1E 01` (batch 12B records), `1E 04` (add 12B), `1E 05 <slot><count>` (remove), `1E 08` (clear), `1E 0C` (Thủy binh 14B), `1E 0D <slot><count>` (Thủy binh remove).
   - Phát hiện bug trong Rust `trade_storage.rs`: Đang hardcode số lượng `0x32` thay vì `item.count`.
6. **Chưa chạy end-to-end với `aLogin.exe` thật** — Cần test kết nối sau khi hoàn thiện code server theo kế hoạch.