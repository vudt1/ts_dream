# 06 — MÔ HÌNH TIỀN TỆ (CURRENCY MODEL) — client player record

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

Ngày: 2026-10-01 · Nguồn chính: `client_pseudo_c/` (1136 file) + đối chiếu `test/ts_decompile/`
Đối tượng: **mọi field tiền / bộ đếm tài nguyên** trong bản ghi người chơi phía client
(`**gvar_007DA7BC` = `TPlayer`/local actor record), opcode nào tác động, và hành vi tràn/âm.

> Quy ước: `P[i]` = byte thứ i payload (`P[0]` = MainOp); `RP[i] = P[i+1]`; `_LStrCopy(s,idx,cnt)` **1-based**.
> Frame: `F4 44 | Len:Word LE | payload`, XOR `0xAD` toàn frame.
> Mức bằng chứng: **(A)** = đọc body/ASM thật · **(B)** = suy luận từ call-site · **(C)** = contract Rust / suy đoán.

---

## 0. Tóm tắt nhanh (TL;DR)

Bản ghi người chơi có **ĐÚNG 3 field tiền dạng `int32`** liên tiếp + **1 nhóm bộ đếm tài nguyên `uint16/uint8`**:

| Offset | Kiểu | Tên nghiệp vụ (suy ra từ nhãn VISCII) | Trần | Set tuyệt đối | Delta ± |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `player+0x12F8` | int32 | **Tiền mang theo (kim)** — HUD `btn_001`, `Form_Bank_Money` | **9 999 999** | `1A 04` (A) | `1A 01` / `1A 02` |
| `player+0x12FC` | int32 | **Tiền trong trang (bank/store)** — `Form_Bank_Money` label trái | **9 999 999** | **`1D 04`** | **`1D 01` / `1D 02`** |
| `player+0x1300` | int32 | **Tiền mềm / điểm (soft currency)** — chỉ hiện ở HUD panel vẽ tay | **2 000 000 000** (`0x77359400`) | `1A 04` (B) | `1A 05` / `1A 06` |
| `player+0x0502` | uint16 | **Thủy binh** (resource) | động: `+0x0500` | `08 06` sel 1 | `1A 08` |
| `player+0x0508` | uint16 | (resource #2 — *chưa định danh*) | động: `+0x0506` | `08 06` sel 2 | **không có** |
| `player+0x050E` | uint16 | **Điểm đạn dược** | động: `+0x050C` | `08 06` sel 3 | `1A 0A` |
| `player+0x0513` | uint8 | **Tài bảo** | **250** (`0xFA`) | `08 06` sel 4 | `1A 09` |
| `player+0x0512` | uint8 | trần của `+0x513` (set = 250) | — | `08 07` sel 4 | — |
| `player+0x04FC` | uint32 | (counter slot 5, *chưa định danh*) | — | `08 07` sel 5 | — |

**Phát hiện lớn nhất của tài liệu này (mới):** `player+0x12FC` — field tiền-in-trang mà `opcode_1a.md` chỉ nói
"được hiển thị lại, KHÔNG bị gói nào ghi" — **thực ra có đường sync riêng hoàn chỉnh qua MainOp `0x1D`**,
với cả 3 phép: set tuyệt đối (`1D 04`), cộng (`1D 01`), trừ (`1D 02`). Chi tiết §3.

---

## 1. Kiểm kê field tiền — mọi call-site

### 1.1 `player+0x12F8` — Tiền mang theo (kim)

| # | Call-site | Phép | Trần / guard | Mức |
| :-- | :-- | :-- | :-- | :-- |
| 1 | `0072b084_FUN_0072b084.c:41` (`SUB EAX,[EDX+0x12f8]` @ `0072b084_FUN_0072b084.asm.txt:18-20`) | `cap = 9999999 - v; if (A <= cap) v += A` | **trần 9 999 999** | **A** |
| 2 | `0072b084_FUN_0072b084.c:50-53` (`asm:23-27` `CMP EAX,[EBP-8]` / `JL` / `ADD [EDX+0x12f8],EAX`) | ghi, trả cờ `1` | nếu vượt → **không ghi, trả 0** | **A** |
| 3 | `0072b084_FUN_0072b084.c:58-62` | refresh `Form_Bank_Money` + caption HUD `btn_001` | — | **A** |
| 4 | `0072b170_FUN_0072b170.c:43-46` (`asm:18-24` `CMP EAX,[EBP-8]` / `JL` / `SUB [EDX+0x12f8],EAX`) | `if (v >= A) v -= A` | **không cho âm**, trả 0 nếu thiếu | **A** |
| 5 | `0072bcf8_FUN_0072bcf8.c:51-53` | **set tuyệt đối** `v = P[2..5]` | **KHÔNG clamp** | **A** |
| 6 | `005d895c_FUN_005d895c.c:68-71` | `Format('%7d', v)` → `form+0x118` | hiển thị | **A** |
| 7 | `00632e3c_FUN_00632e3c.c:58` (đối chiếu `test/ts_decompile/functions/`) | `IntToStr(v)` vẽ HUD tại `y+0x5a` | hiển thị | **A** |
| 8 | `00541098`, `00547864` (`asm`+`.c`) | đọc để vẽ/đo | hiển thị | **B** |

**Hằng trần đọc thẳng từ ASM**: `MOV EAX,0x98967f` = **9 999 999** (`0072b084_FUN_0072b084.asm.txt:18`). **A**

> **Lưu ý quan trọng (A)**: `FUN_0072bcf8` (sub `0x04`) ghi **tuyệt đối, không clamp**. Nghĩa là server gửi
> A = 20 000 000 sẽ khiến HUD hiển thị tràn 7 chữ số `%7d` và các phép `1A 01` sau đó **vĩnh viễn fail**
> (vì `cap = 9999999 - 20000000` âm ⇒ `JL` ⇒ không ghi). ⇒ **Server phải tự clamp ≤ 9 999 999.**

### 1.2 `player+0x12FC` — Tiền trong trang (bank/store)

| # | Call-site | Phép | Trần / guard | Mức |
| :-- | :-- | :-- | :-- | :-- |
| 1 | `0072b9b8_FUN_0072b9b8.c:28-32` | `cap = 9999999 - v; if (A <= cap) v += A` | **trần 9 999 999** | **A** |
| 2 | `0072b9b8_FUN_0072b9b8.c:40` | refresh `Form_Bank_Money` | — | **A** |
| 3 | `0072ba0c_FUN_0072ba0c.c:27-30` | `if (v >= A) v -= A` | **không cho âm** | **A** |
| 4 | `0072ba0c_FUN_0072ba0c.c:35` | refresh `Form_Bank_Money` | — | **A** |
| 5 | **`0072be10_FUN_0072be10.c:37-40`** | **set tuyệt đối** `v = FUN_0077ef7c(P[2..5])` | **KHÔNG clamp** | **A** |
| 6 | `005d895c_FUN_005d895c.c:59-62` | `Format('%7d', v)` → `form+0x114` | hiển thị | **A** |
| 7 | `005d7d18_FUN_005d7d18.c:113-118` | `Format('%7d', v)` → `Form_Bank_Money+0x45` | hiển thị | **A** |

Hằng trần: `&DAT_0098967f` = **9 999 999** (`0072b9b8_FUN_0072b9b8.c:28`). **A**

> ⚠️ **Đính chính quan trọng cho `opcode_1a.md`**: mục 4.4 của tài liệu đó nói `+0x12FC` "**KHÔNG bị gói nào ghi**,
> chỉ được hiển thị lại". Điều đó **chỉ đúng trong phạm vi MainOp `0x1A`**. `FUN_0072be10` (**A**,
> `0072be10_FUN_0072be10.c:39`) ghi nó từ gói **`0x1D 04`** — xem §3.

### 1.3 `player+0x1300` — Tiền mềm / điểm (soft currency)

| # | Call-site | Phép | Trần / guard | Mức |
| :-- | :-- | :-- | :-- | :-- |
| 1 | `00745acc_FUN_00745acc.c:25-30` (`asm:15-19` `CMP EAX,0x77359400` / `JG` / `ADD [EDX+0x1300],EAX`) | `if (v + D <= 0x77359400) v += D` | **trần 2 000 000 000** | **A** |
| 2 | `00745b14_FUN_00745b14.c:26-30` | `if (v - D >= 0) v -= D` | **không cho âm** | **A** |
| 3 | `0072bcf8_FUN_0072bcf8.c:54-56` | **set tuyệt đối** `v = P[6..9]` | **KHÔNG clamp** | **A** |
| 4 | `00632e3c_FUN_00632e3c.c:93` | `IntToStr(v)` vẽ HUD tại `y+0xa6` | hiển thị | **A** |

**Hằng trần đọc thẳng từ ASM**: `CMP EAX,0x77359400` / `JG` (`00745acc_FUN_00745acc.asm.txt:15-16`)
= **2 000 000 000** (2e9). **A**

> **Phát hiện đáng chú ý (A)**: `+0x1300` là field tiền **duy nhất KHÔNG có label danh nghiệp vụ**
> (không literal nào nhắc tới), và là field **duy nhất chỉ được vẽ bằng hàm vẽ tay `00632e3c`**
> (paint trực tiếp lên canvas `gvar_007DA574` qua `FUN_007d1648`, không đi qua form control nào).
> Nó cũng **không** xuất hiện trong `Form_Bank_Money`.
> ⇒ Nghiệp vụ khả dĩ nhất: **tiền mềm / điểm mua-item-trong-cửa-hàng-đặc-biệt (cash shop)**. Mức **B**.

### 1.4 Nhóm bộ đếm tài nguyên `+0x500..+0x514`

| Offset | Kiểu | Nhãn VISCII (đã giải mã) | Trần động | Setter tuyệt đối | Incrementer |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `+0x0500` | u16 | trần của Thủy binh | — | `08 07` sel 1 (`0074cc84_FUN_0074cc84.c:57`) | — |
| `+0x0502` | u16 | **`Thủy binh`** (`0x7980DC`) | `<= +0x0500` | `08 06` sel 1 (`0074c4c8_FUN_0074c4c8.c:57`) | `1A 08` (`0074c64c…c:23-29`) |
| `+0x0506` | u16 | trần của counter #2 | — | `08 07` sel 2 (`0074cc84…c:60`) | — |
| `+0x0508` | u16 | (không nhãn) | — | `08 06` sel 2 (`0074c4c8…c:60`) | **không có** |
| `+0x050C` | u16 | trần của Điểm đạn dược | — | `08 07` sel 3 (`0074cc84…c:63`) | — |
| `+0x050E` | u16 | **`Điểm đạn dược`** (`0x798100`) | `<= +0x050C` | `08 06` sel 3 (`0074c4c8…c:63`) | `1A 0A` (`0074c6b4…c:32-40`) |
| `+0x0512` | u8 | trần của Tài bảo (set cứng 250) | — | `08 07` sel 4 (`0074cc84…c:66`) | — |
| `+0x0513` | u8 | **`Tài bảo`** (`0x7980F0`) | **250** | `08 06` sel 4 (`0074c4c8…c:70`) | `1A 09` (`0074c5f4…c:23-37`) |
| `+0x04FC` | u32 | (không nhãn) | — | `08 07` sel 5 (`0074cc84…c:69`) | — |
| `+0x0509` | u16 | mảng con của record buff (xem `00527674…c:211`) | — | — | — |
| `+0x0514`+k*4 | u8 | cờ timestamp của record buff | — | — | `00527674…c:242` |

**Quan trọng (A) — cảnh báo offset aliasing**: `+0x050C` vừa là **trần của `+0x050E` (uint16)**
(`0074c6b4_FUN_0074c6b4.c:39`) vừa là **mảng `double[4]` timestamp** ghi bởi `00527674_FUN_00527674.c:242`
(`*(double *)(player + 0x50c + k*4) = Now()`), k=1..4 ⇒ vùng `0x50C..0x51C` bị ghi đè. Đây là artefact
cấu trúc thật của client, **không phải bug decompile**. Mức **A** (nhưng ý nghĩa nghiệp vụ **C**).

**Hành vi trần (đọc thật)**:
- `+0x0502` vs `+0x0500`: `if (*(u16*)(p+0x500) < v) *(u16*)(p+0x502) = *(u16*)(p+0x500);` — **clamp xuống trần**,
  rồi `return 1` **luôn** (`0074c64c_FUN_0074c64c.c:27-40`). **A**
- `+0x0513`: `if (v < 0xFB) v += B; else *(u8*)(p+0x513) = 0xFA;` — **clamp cứng 250**, `return 1` **luôn**
  (`0074c5f4_FUN_0074c5f4.c:28-37`). **A**
- `+0x050E` vs `+0x050C`: giống `+0x0502`, `return 1` **luôn** (`0074c6b4_FUN_0074c6b4.c:39-43`). **A**

⇒ **Cả 3 bộ đếm đều LUÔN trả 1 (predicate hằng 1)** ⇒ nhánh "thất bại" của sub `1A 08/09/0A` là **code chết**. **A**

**Chi phí trừ cục bộ (không qua server) — `00527674_FUN_00527674.c`**:
- `+0x050E -= *(int*)(rec + 0xBD4)` (d.191-198) — dùng khi `local_1d ∈ {3,4}` (d.187)
- `+0x0502 -= *(int*)(rec + 0xBCC)` (d.261-269) — dùng khi `local_1d ∈ {2,4}` (d.250)
- `+0x0508 -= *(int*)(rec + 0xBD0)` (d.270-278)
⇒ Client tự trừ 1 phần tài nguyên khi dùng skill/tech. **B** (ngữ nghĩa `local_1d` chưa định danh).
⚠️ **Hệ quả thiết kế (C)**: server **không** được gửi `1A 08/09/0A` để "set" — phải dùng `08 06` set tuyệt đối,
vì các sub này chỉ **cộng**.

---

## 2. Bảng "opcode → field" đầy đủ

### 2.1 MainOp `0x1A` (S→C) — kênh tiền chính

| Sub | Payload | Field | Phép | Trần/guard | Literal hiển thị | Nguồn | Mức |
| :-: | :-- | :-- | :-- | :-- | :-- | :-- | :-: |
| `01` | `[1A][01][A:4 LE]` (6B) | `+0x12F8` | `+= A` | trần **9 999 999**, fail⇒ret 0 | OK: `0x798038`+`0x79804C` = `"Nhận được ATiền"`; FAIL: `0x79805C`+`0x798078` = `"Thất bại  đoạt được Akim"` | `case_023…c:34-65`; `0072b084…c:41-63`; `asm:18-27` | **A** |
| `02` | `[1A][02][A:4 LE]` (6B) | `+0x12F8` | `-= A` | **không âm**, fail⇒ret 0 | OK: `0x798084`+`0x79804C` = `"GiảmATiền"`; FAIL: `0x798094`+`0x798078` = `"Thất bại   giảm thiểu Akim"` | `case_023…c:67-93`; `0072b170…c:43-60`; `asm:18-24` | **A** |
| `03` | `[1A][03]` (2B) | — | banner tĩnh 2000ms | — | `0x7980B4` = `"Dung lượng tiền bạc không đủ"` | `case_023…c:95-97` | **A** |
| `04` | `[1A][04][A:4 LE][B:4 LE]` (10B) | `+0x12F8 = A`<br>`+0x1300 = B` | **set tuyệt đối cả 2** | **KHÔNG clamp** | refresh `Form_Bank_Money`; caption `btn_001` = `0x72B16C`(`"kim"`)+`A`+`0x72BDF8`(`"Tiền bạc"`) | `case_023…c:98-100`; `0072bcf8…c:51-61`; `asm:49-58` | **A** |
| `05` | `[1A][05][D:4 LE]` (6B) | `+0x1300` | `+= D` | trần **0x77359400** (2e9) | (im lặng) | `case_023…c:101-106`; `00745acc…c:25-37`; `asm:15-19` | **A** |
| `06` | `[1A][06][D:4 LE]` (6B) | `+0x1300` | `-= D` | **không âm** | (im lặng) | `case_023…c:107-112`; `00745b14…c:26-39` | **A** |
| `07` | `[1A][07][x:1B]` (3B) | — | `x==1`⇒banner 1500ms | — | `0x7467C8` = `"giao dịch bày ra thất bại"` **(mới giải mã)** | `case_023…c:113-115`; `00746774…c:22-29` | **A** |
| `08` | `[1A][08][W:2 LE]` (4B) | `+0x0502` | `+= W` | clamp `<= +0x0500`; **luôn ret 1** | `"Nhận được W"`+`0x7980DC`(`"Thủy binh"`) + `WA0014.wav` + `TLight` | `case_023…c:116-147`; `0074c64c…c:23-40` | **A** |
| `09` | `[1A][09][B:1B]` (3B) | `+0x0513` | `+= B` | clamp cứng **250**; **luôn ret 1** | `"Nhận được B"`+`0x7980F0`(`"Tài bảo"`) + `WA0014.wav` + `TLight` | `case_023…c:149-156`; `0074c5f4…c:23-40` | **A** |
| `0A` | `[1A][0A][W:2 LE]` (4B) | `+0x050E` | `+= W` | clamp `<= +0x050C`; **luôn ret 1** | `"Nhận được W"`+`0x798100`(`"Điểm đạn dược"`) + `WA0014.wav` | `case_023…c`; `0074c6b4…c:32-51` | **A** |
| `00`,`≥0B` | — | — | no-op (rơi khỏi switch) | — | — | `case_023…c:32` | **A** |

### 2.2 MainOp `0x1D` (S→C) — kênh **tiền trong trang** (MỚI, chưa có trong docs cũ)

Đường đi: `byte_table[0x78A8EE][0x1D] = 25` → `jumptable_dword200_0x78A9B6[25] = 0x00791F76` → `FUN_00791f76`
(`case_025_00791F76_FUN_00791f76.c:26-79`). Tất cả đọc thật.

| Sub | Payload | Field | Phép | Trần/guard | Literal | Nguồn | Mức |
| :-: | :-- | :-- | :-- | :-- | :-- | :-- | :-: |
| `01` | `[1D][01][A:4 LE]` | `+0x12FC` | `+= A` | trần **9 999 999** | (banner, cờ không rõ) | `case_025…c:30-45`; `0072b9b8…c:28-41` | **A** |
| `02` | `[1D][02][A:4 LE]` | `+0x12FC` | `-= A` | **không âm** | (banner) | `case_025…c:47-62`; `0072ba0c…c:27-36` | **A** |
| `03` | `[1D][03]` | — | banner tĩnh 2000ms | — | `0x798314` = `"Dung lượng tiền trang không đủ"` **(mới giải mã)** | `case_025…c:63-65` | **A** |
| `04` | `[1D][04][A:4 LE]` | `+0x12FC` | **set tuyệt đối** | **KHÔNG clamp** | refresh `Form_Bank_Money` | `case_025…c:66-68` → `FUN_0072be10`; `0072be10…c:37-40` | **A** |
| `05` | `[1D][05]` | — | `(*(*gvar_007DA46C->VMT+0x20))()` (đóng form) | — | — | `case_025…c:69-71` | **A** |
| `06` | `[1D][06]` | — | đóng form `gvar_007DA0A4` | — | — | `case_025…c:72-74` | **A** |
| `07` | `[1D][07]` | form `+0x11C = 0` | cờ form | — | — | `case_025…c:75-77` | **A** |
| `08` | `[1D][08]` | form `+0x11D = 0` | cờ form | — | — | `case_025…c:78-80` | **A** |
| `09` | `[1D][09][B:1B]` | `gvar_007DA0A4+0x418 = B` | set byte | — | — | `case_025…c:81` → `00749c30…c:27-29` | **A** |

> **Ý nghĩa (B→A)**: `0x1D` là **kênh của form `Form_Bank_Money` / `Form_Bank_Thing`** (tiền trang, lưu trữ).
> Sub `05/06` đóng form, sub `07/08` xoá 2 cờ `+0x11C/+0x11D` (khớp `005d7d18…c:120-121` vốn gán
> `local_8+0x47 = 0` và `form+0x11D = 0`). Sub `09` ghi vào **form `gvar_007DA0A4`**, không phải player.

### 2.3 MainOp `0x08` (S→C) — **setter tuyệt đối** cho bộ đếm

Đường đi: `byte_table[0x08] = 9` → `0x0078D125` (`case_009_0078D125_FUN_0078d125.c`).

| Sub | Payload | Field ghi | Nguồn | Mức |
| :-: | :-- | :-- | :-- | :-: |
| `06` | `[08][06][sel:1B][v:2 LE]` | sel1→`+0x0502`, sel2→`+0x0508`, sel3→`+0x050E`, sel4→`+0x0513` (chặn >255) | `case_009…c:181`; `0074c4c8…c:53-71` | **A** |
| `07` | `[08][07][sel:1B][v:2 LE]` | sel1→`+0x0500`, sel2→`+0x0506`, sel3→`+0x050C`, sel4→`+0x0512=250`, sel5→`+0x04FC` | `case_009…c:185`; `0074cc84…c:55-70` | **A** |

> `1D 04` là set tuyệt đối của `+0x12FC`; **không có** sub nào của `0x08` set `+0x12F8/+0x12FC/+0x1300`.

### 2.4 MainOp `0x23` sub `0x04` — ⚠️ **KHÔNG phải** frame tiền

| Sub | Payload | Field ghi | Nguồn | Mức |
| :-: | :-- | :-- | :-- | :-: |
| `04` | `[23][04][A:4][B:4][D:8]` | **`+0x146C = A*100 + B`** (DWORD) — **không phải field tiền** | `case_031…c:169-201`; `.asm` `00632e3c` không liên quan | **A** |

`0x23 04` chỉ **banner + chat-log** (`FUN_007ab870(gvar_007DA1B0, 0, msg, '\n')`) và ghi `+0x146C`.
**Không đụng bất kỳ field tiền nào.** Điều này ảnh hưởng trực tiếp tới Rust — xem §4.

### 2.5 Chiều C→S

`case 0x1a: break;` rỗng trong `FUN_0077f414` (`0077f414…c:939-940` theo `opcode_1a.md` §5) và
`case 0x1d` tương tự ⇒ **cả `0x1A` và `0x1D` là downlink-only**. **B** (theo handoff, chưa tự verify lại
`0077f414` trong phiên này).

---

## 3. Overflow / negative guard — tổng hợt

| Field | Guard | Hằng | Vị trí so sánh | Mức |
| :-- | :-- | :-- | :-- | :-: |
| `+0x12F8` cộng | `cap = 9999999 - v; if (A <= cap)` — **atomic reject, không clamp** | `0x98967f` | `0072b084…asm:18-24` | **A** |
| `+0x12F8` trừ | `if (v >= A)` — **atomic reject** | — | `0072b170…asm:18-21` | **A** |
| `+0x12F8` set | **KHÔNG guard** | — | `0072bcf8…c:53` | **A** |
| `+0x12FC` cộng | `cap = 9999999 - v; if (A <= cap)` | `0x98967f` | `0072b9b8…c:28-32` | **A** |
| `+0x12FC` trừ | `if (v >= A)` | — | `0072ba0c…c:27` | **A** |
| `+0x12FC` set | **KHÔNG guard** | — | `0072be10…c:39` | **A** |
| `+0x1300` cộng | `if (v + D <= 0x77359400)` | `0x77359400` = 2e9 | `00745acc…asm:15-16` | **A** |
| `+0x1300` trừ | `if (v - D >= 0)` | — | `00745b14…c:26-28` | **A** |
| `+0x1300` set | **KHÔNG guard** | — | `0072bcf8…c:56` | **A** |
| `+0x0502` cộng | **clamp** xuống `+0x0500` (không reject) | động | `0074c64c…c:27-29` | **A** |
| `+0x050E` cộng | **clamp** xuống `+0x050C` | động | `0074c6b4…c:39-40` | **A** |
| `+0x0513` cộng | **clamp cứng 250** | `0xFA` | `0074c5f4…c:37` | **A** |

### 3.1 Kết luận cho server

1. **Tiền (`+0x12F8`) và tiền trang (`+0x12FC`) dùng cùng trần `9 999 999`** — một hằng duy nhất `0x98967f`. **A**
2. Phép **delta là "reject", không phải "clamp"**: thiếu tiền hoặc tràn ⇒ **field KHÔNG đổi**, predicate trả 0.
   ⇒ Client **không bao giờ** tự sửa số dư sai. **Server là nguồn sự thật duy nhất.** **A**
3. Phép **set tuyệt đối (`1A 04` / `1D 04`) không clamp** ⇒ **server BẮT BUỘC tự clamp `≤ 9 999 999`** trước khi
   gửi, nếu không client sẽ hiển thị tràn và mọi `1A 01`/`1D 01` về sau sẽ **vĩnh viễn fail im lặng**. **A + C**
4. `+0x1300` trần **2e9**, lỏng hơn 200 lần so với 2 field kia. **A**
5. Bộ đếm tài nguyên **clamp** (không reject) và **luôn trả 1** ⇒ không dùng được để báo lỗi cho client. **A**
6. **Overflow `int32` chưa được bảo vệ**: `CARRY2/CARRY1/SCARRY4/SBORROW4` chỉ gọi `FUN_00402fb0` (`@IntOver`)
   ⇒ raise Delphi range-check exception, **không** phải saturate. Nhưng vì có guard ở trên nên chỉ xảy ra
   nếu server gửi giá trị vượt trần qua set tuyệt đối. **A**

---

## 4. Rust hiện tại — gap

### 4.1 Field tiền trong `Session`

| Rust field | Vị trí | Loại | Đối chiếu client |
| :-- | :-- | :-- | :-- |
| `gold: u32` | `src/server/session.rs:196` | Tiền mang theo | ✅ khớp `+0x12F8` |
| `bank_gold: u32` | `src/server/session.rs:242` | Tiền trong trang | ✅ khớp `+0x12FC` — **nhưng không có frame sync nào** |
| `shop_point: u32` | `src/server/session.rs:243` (default `1000`, d.373) | Điểm tích lũy | ⚠️ **không có field client tương ứng** — xem 4.3 |
| `god: u32` | `src/server/session.rs:197` | Điểm thần/thánh | ❌ không có field client riêng |
| `hp_store: u32` | `src/server/session.rs:198` | Kho HP | ❌ không có field client riêng |
| `sp_store: u32` | `src/server/session.rs:199` | Kho SP | ❌ không có field client riềng |
| `texp: u32` | `src/server/session.rs:195` | Kinh nghiệm | ❌ không có field client riêng |
| `TradeState.gold: u32` | `src/server/session.rs:280` | Tiền giao dịch | ✅ dùng `gold` làm backing |

**DB** (`migrations/0001_init.sql:83-87`, bảng `character_money`): chỉ **3 cột** — `gold`, `bankgold`, `shoppoint`.
Model: `src/db/modern/model.rs:97-99` (`bank_gold`, `shop_point`); đọc/ghi `src/db/modern/sqlite/session.rs:27-28,67-69,297,516-518`.
⇒ **KHÔNG có cột nào cho xu / tiền mềm (`+0x1300`) / bộ đếm `+0x0502/+0x050E/+0x0513`.**

### 4.2 `spawn.rs:561-564` — frame `1A 04`

```rust
// 12. Step 12: Gold
let mut gold_body = String::new();
gold_body.push_str(&encoder::le32(s.gold));   // A → +0x12F8
gold_body.push_str("00000000");               // B = 0 → +0x1300
frames.push(crate::protocol::frame("1A04", &gold_body));
```

⇒ **Đúng về mặt cấu trúc**: `1A 04` = `[A:4LE][B:4LE]`, `A` = gold → `+0x12F8`. **A**
⇒ **Sai về nghiệp vụ 2 điểm**:
1. **`B` luôn = 0** ⇒ `+0x1300` (tiền mềm) **bị set về 0 mỗi lần login** ⇒ HUD vẽ `0` ở `y+0xa6`. **C**
2. **`bank_gold` (`+0x12FC`) không được sync lúc login.** Rust **có** frame `1D 04`
   (`chat.rs:413`, `talk.rs:713`) nhưng **chỉ gửi khi mở form ngân hàng** (`/openbank`, H6 template
   `16080/16004/16011/16023` + `select_menu == 30`), **không** gửi trong luồng login. ⇒ Vô tới lúc
   client hiện `Form_Bank_Money` thì `+0x12FC` = 0. **C**

### 4.3 `shops.rs` — mua/bán dùng frame nào

| Vị trí | Việc | Frame gửi | Đánh giá |
| :-- | :-- | :-- | :-- |
| `shops.rs:11-13` | `gold_frame(gold)` = `F4440A001A04` + le32(gold) + `00000000` | `1A 04` | ⚠️ **luôn gửi B=0** ⇒ **ghi đè `+0x1300` về 0 mỗi lần mua/bán** |
| `chat.rs:411-416` | `/openbank` → `1D 09 00`, **`1D 04`+le32(bank_gold)**, `1D 05`, `14 09` | `1D 04` | ✅ **set tuyệt đối `+0x12FC` đúng** — khớp `FUN_0072be10` |
| `talk.rs:711-716` | H6 `select_menu==30` → cùng bộ `1D` | `1D 04` | ✅ idem |
| `shops.rs:202` | sau khi bán | `gold_frame(gold)` | idem |
| `shops.rs:245` `gold -= price` | trừ khi mua | (không gửi delta) | dùng **set tuyệt đối** thay vì `1A 02` |
| `shops.rs:262` | sau khi mua | `gold_frame(gold)` | idem |
| `shops.rs:365-366` | giao dịch shop người chơi | `gold_frame` (d.568, 576) | có clamp `.min(9_999_999)` ✅ |
| `trade_storage.rs:124-126` | `gold_status_frame` | `1A 04` + `00000000` | idem |
| `trade_storage.rs:13` | `GOLD_CAP = 9_999_999` | — | ✅ **khớp client** |
| `trade_storage.rs:723-728` | guard nạp tiền: `gold + amount <= GOLD_CAP` **và** `bank_gold + amount <= GOLD_CAP` | — | ✅ đúng |
| `system.rs:12-18` | `shop_points_frame` = `4202` + le16 + `0100` | `42 02` | ⚠️ xem 4.4 |
| `system.rs:116-167` | GM shop: `shop_point -= price` | `42 02` | ⚠️ xem 4.4 |

**Trả lời trực tiếp câu hỏi nhiệm vụ**: Rust **CÓ** gửi delta, nhưng **chỉ trong đúng 1 nơi** —
`handle_bank_gold` (`trade_storage.rs:748-751`) gửi cặp `1D 01/02` + `1A 01/02` khi rút/gửi tiền ngân hàng.
Mọi nơi khác (`shops.rs`, `trade_storage.rs:311,408`, `spawn.rs`) dùng **set tuyệt đối `1A 04`**.
`1A 05`/`1A 06` (`+0x1300`) và toàn bộ `1A 08/09/0A` (bộ đếm) — **0 call-site**. **A**

### 4.4 `battle.rs` + `use_item/books.rs`

| Vị trí | Việc | Frame | Đánh giá |
| :-- | :-- | :-- | :-- |
| `handlers/battle.rs` | (grep `gold`) | — | ❌ **KHÔNG có frame tiền nào** trong battle — thưởng tiền chưa implement |
| `spawn.rs:592-594` | `store_frame(god)`, `store_frame(hp_store)`, `store_frame(sp_store)` | `23 04` | ❌ **`23 04` KHÔNG set field tiền nào** (chỉ `+0x146C`) — xem §2.4 |
| `use_item/books.rs:186,194` | `hp_store += 10000` | (không gửi frame) | ⚠️ comment tự nhận *"deferred drift — in-memory only, no DB persist"* |

### 4.5 Bảng GAP cụ thể

| # | Gap | Mức ưu tiên | Bằng chứng |
| :-- | :-- | :-: | :-- |
| **G1** | **`bank_gold` (`+0x12FC`) không được sync lúc login.** Frame `1D 04` **đã có** (`chat.rs:413`, `talk.rs:713`) nhưng chỉ gửi khi mở form ngân hàng, không gửi trong `spawn.rs`. ⇒ `Form_Bank_Money` hiện `0` cho tới khi người chơi tự mở bank. | 🟡 TB | `spawn.rs` (0 hit `1D`) vs `chat.rs:413` |
| **G2** | **Không có field/cột cho tiền mềm `+0x1300`.** Mọi `gold_frame` gửi B=`00000000` ⇒ **ghi đè `+0x1300` = 0** ở login và sau **mỗi** giao dịch mua/bán/giao dịch. | 🔴 cao | `shops.rs:11-13`; `spawn.rs:561-564`; `0072bcf8…c:56` (A) |
| **G3** | **`23 04` bị dùng sai ý nghĩa** cho `god`/`hp_store`/`sp_store`/`gold`. Frame này không set field tiền nào, chỉ ghi `+0x146C` + banner/chat-log ⇒ đang **spam banner 10 000 ms** cho người chơi mỗi lần login. | 🔴 cao | `case_031…c:169-201` (A) vs `spawn.rs:592-594` |
| **G4** | **Không có bất kỳ frame bộ đếm tài nguyên nào** (`08 06`, `08 07`, `1A 08/09/0A`). Không có cột DB cho `+0x0502/+0x050E/+0x0513`. | 🟡 TB | `0074c4c8…c:57-70`; `0074cc84…c:57-69` (A) |
| **G5** | `shop_point` (`42 02`) **không có field client** tương ứng — là DB-only, client không hiển thị. | 🟡 TB | `migrations/0001_init.sql:87`; `system.rs:12-18` |
| **G6** | **Thiếu guard clamp phía server khi set tuyệt đối.** `trade_storage.rs:13` có `GOLD_CAP` và `handle_bank_gold` dùng nó (d.724,728) đúng, nhưng `gold_frame`/`shops.rs:11-13` và `1D 04` ở `chat.rs:413`/`talk.rs:713` **không clamp**. Gửi `gold > 9 999 999` sẽ khóa vĩnh viễn mọi `1A 01`/`1D 01` phía sau. | 🟡 TB | `shops.rs:11-13` (A) |
| **G7** | `battle.rs` **không phát thưởng tiền** dù có thưởng item. | 🟢 thấp | grep `gold` trong `battle.rs` = 0 hit |
| **G8** | `hp_store`/`sp_store` không persist DB (tự nhận trong comment) | 🟢 thấp | `use_item/books.rs:186` |
| **G9** | ✅ **KHÔNG phải gap** — `handle_bank_gold` (`trade_storage.rs:707-751`) implement **đúng** cả `1D 01/02` (delta `+0x12FC`) **và** `1A 01/02` (delta `+0x12F8`), có guard `GOLD_CAP` hai chiều và rollback qua `persist_sessions_transaction`. | ✅ | `trade_storage.rs:720-751` (A) |

---

## 5. Kết luận thiết kế cho server

### 5.1 Cần mấy loại tiền?

**Tối thiểu 3 loại tiền thật** (không tính bộ đếm tài nguyên), ánh xạ 1-1 với field client:

| # | Loại | Offset client | Kiểu | Trần | Set tuyệt đối | Delta |
| :-: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | **Gold / tiền mang theo (kim)** | `+0x12F8` | u32 | 9 999 999 | **`1A 04` (vị trí A)** | `1A 01` cộng / `1A 02` trừ |
| 2 | **Bank gold / tiền trong trang** | `+0x12FC` | u32 | 9 999 999 | **`1D 04` (DWORD đơn)** | `1D 01` / `1D 02` |
| 3 | **Tiền mềm / cash point (xu)** | `+0x1300` | u32 | 2 000 000 000 | **`1A 04` (vị trí B)** | `1A 05` / `1A 06` |

**Câu trả lời cho câu hỏi "dùng `1A 04` cho cả 2 field A/B?"**: **Có** — `1A 04` ghi **đồng thời** `+0x12F8` (A) và
`+0x1300` (B) (`0072bcf8…c:51-56`). Đây là frame **duy nhất** ghi được cả 2. Nhưng nó **không** chạm `+0x12FC`.

### 5.2 Loại nào client **chưa có** đường sync?

| Loại | Tình trạng |
| :-- | :-- |
| **Bank gold (`+0x12FC`)** | ✅ **Đã implement đầy đủ**: `1D 04` set (`chat.rs:413`, `talk.rs:713`) + `1D 01/02` delta (`trade_storage.rs:748`). Thiếu duy nhất: **gửi lúc login**. |
| **Tiền mềm (`+0x1300`)** | ✅ Client có (`1A 04` slot B / `1A 05` / `1A 06`) nhưng Rust **chưa có field DB** nào giữ nó, và `gold_frame` **ghi đè về 0**. |
| **Xu / hpoint (cash shop currency)** | ❌ **KHÔNG có field client riêng.** `+0x12F8/+0x12FC/+0x1300` đã bị chiếm bởi kim / bank / soft. `shop_point` của Rust là DB-only. ⇒ Nếu muốn hpoint phải **map vào `+0x1300`** (đã có sẵn đường set+delta+hiển thị HUD). **C** |
| **Bộ đếm tài nguyên (Thủy binh / Điểm đạn dược / Tài bảo)** | ✅ Có (`08 06` set + `1A 08/09/0A` cộng) nhưng **Rust chưa có field, chưa có cột DB, chưa có frame**. |

### 5.3 Khuyến nghị triển khai (C)

1. **Gửi `1D 04` + le32(bank_gold) trong `spawn.rs`**, cạnh `1A 04` ở d.561 ⇒ đóng G1.
2. **Sửa `gold_frame`** để nhận cả 2 tham số: `gold_frame(gold, soft)`, và **clamp `gold.min(9_999_999)`**
   (dùng `GOLD_CAP` sẵn có ở `trade_storage.rs:13`). → đóng G2 + G6.
3. **Thêm field `soft_point: u32` + cột DB `softpoint`** (migrate `character_money`), đọc/ghi cùng chỗ
   `gold` ở `sqlite/session.rs:27,297,516`. Khởi tạo từ `+0x1300` khi login.
4. **Bỏ/thay `store_frame` (`23 04`)** cho `god`/`hp_store`/`sp_store` — frame này không set field nào,
   chỉ spam banner 10 s. → đóng G3.
5. **Thêm frame `08 06`/`08 07`** nếu muốn 3 bộ đếm tài nguyên; nhớ `+0x0500/+0x050C` là **trần** phải set
   **trước** `+0x0502/+0x050E`, còn `+0x0513` tự clamp 250. → đóng G4.
6. **Dùng `1A 02`/`1A 06`/`1D 02` cho delta âm thay vì `1A 04`**: tiết kiệm 1 field trong frame và tránh
   việc set tuyệt đối cả 2 slot khi chỉ muốn đổi 1. (Đã có tiền lồ trong `handle_bank_gold` — chỉ cần mở rộng
   cho shop.) Server vẫn giữ nguồn sự thật vì client **reject** chứ không clamp.

---

## 6. CHƯA XÁC MINH

| # | Điểm | Vì sao chưa xác minh | Hướng tìm tiếp |
| :-- | :-- | :-- | :-- |
| 1 | **Tên nghiệp vụ chính xác của `+0x1300`** | Không literal nào nhắc tới; chỉ vẽ tay ở `00632e3c:93`. Suy đoán "tiền mềm/cash point" là **B** | Đọc `FUN_007d1648` (hàm vẽ text) + resource/skin của HUD panel tại `gvar_007DA574`; hoặc so `00632e3c` với form có nhãn trong `Data/` |
| 2 | **`+0x0508` / `+0x0506` / `+0x04FC` là gì** | `08 06` sel 2 và `08 07` sel 2/5 có cặp current/max như `0x502/0x500`, nhưng **không có** sub `1A` nào cộng vào `+0x0508`, và **không có** nhãn banner. `opcode_08.md` chỉ gọi chung "point/tài nguyên" | Grep `player+0x508` trong toàn bộ `test/ts_decompile/functions/`; tìm form HUD nào vẽ `0x502/0x508/0x50E` cạnh nhau |
| 3 | **`+0x12FC` có phải "tiền trong trang" hay "tiền gửi kho"?** | Nhãn `0x798314` = `"Dung lượng tiền trang không đủ"` (sub `1D 03`) gợi "tiền trang" (= túi tiền trong trang) chứ không phải bank. Nhưng nó nằm trong `Form_Bank_Money` cùng `+0x12F8` | Đọc `Form_Bank_Thing` (`005d9068`) và `Form_BankUpgrade` (`005eb530`) xem chúng đọc offset nào; tìm nơi nạp `1D 04` |
| 4 | **Ý nghĩa cờ `+0x14` trong gate `0072c0f0`** (chặn banner sub `1A 01/02`) | Chưa xác định là Visible/Modal hay gì | Đọc Delphi TForm field table cho class của `gvar_007DA46C` |
| 5 | **Chiều C→S của `0x1D`** | Chỉ dựa vào handoff `opcode_1a.md` §5 cho `0x1A`; **chưa tự verify** `case 0x1d` trong `FUN_0077f414`. *(Lưu ý: điều này **không** phủ nhận việc Rust có `handle_bank_gold` nhận `0x1D` C→S — xem `trade_storage.rs:707`; cần xác minh opcode client thật sự gửi là gì.)* | Đọc `0077f414_FUN_0077f414.c`, tìm `case 0x1d` |
| 6 | **Ý nghĩa `local_1d` trong `00527674`** (gate trừ `0x502/0x508/0x50E`) | Chỉ biết là selector 1 byte ∈ {2,3,4}; chưa rõ là loại tech/skill nào | Đọc `FUN_0052731c` (parse chuỗi → `local_1d`) |
| 7 | **`0x50C..0x51C` aliasing** | `+0x050C` vừa là trần u16 vừa là `double[4]` timestamp — chưa biết client cố ý hay lỗi thiết kế record | Đọc `opcode_08.md` §4.8 gốc + `FUN_00527674` đầy đủ; hỏi domain: 2 concept có thể dùng chung byte không |
| 8 | **`+0x146C`** (ghi bởi `23 04`) là gì | Chỉ biết công thức `A*100+B`; chưa biết hiển thị ở đâu | Grep `0x146c` trong `test/ts_decompile/functions/` |
| 9 | **`btn_001` có còn field thứ 2 không** | `1A 04` chỉ set caption `btn_001` bằng `A`; `B` (`+0x1300`) không set caption nào | Đọc `0058a964_FUN_0058a964.c:160-190` để xem `btn_002..btn_009` dùng cho gì |
| 10 | **`shop_point` (`42 02`) client có hiển thị không** | Chưa tìm thấy handler `0x42` trong `client_pseudo_c/` | Tìm `case` cho MainOp `0x42` qua `jumptable_byte200_0x78A8EE.hex` |

---

## 7. Source trail

| # | File | Dùng để |
| :-- | :-- | :-- |
| 1 | `client_pseudo_c/case_023_0079175F_FUN_0079175f.c:25-214` | Handler `0x1A`, switch SubOp 1..10, mọi phép đọc payload |
| 2 | `client_pseudo_c/case_025_00791F76_FUN_00791f76.c:26-79` | **Handler `0x1D`** — sub 1/2/3/4/5/6/7/8/9 |
| 3 | `client_pseudo_c/case_009_0078D125_FUN_0078d125.c:181,185` | Handler `0x08` sub 6/7 → setter bộ đếm |
| 4 | `client_pseudo_c/case_031_00792BAD_FUN_00792bad.c:169-201` | `0x23 04` → `+0x146C` (KHÔNG phải tiền) |
| 5 | `client_pseudo_c/0072b084_FUN_0072b084.c:41-63` + `.asm.txt:18-48` | `1A 01` cộng `+0x12F8`, trần `0x98967f`, refresh bank + caption HUD |
| 6 | `client_pseudo_c/0072b170_FUN_0072b170.c:43-60` + `.asm.txt:18-45` | `1A 02` trừ `+0x12F8` |
| 7 | `client_pseudo_c/0072bcf8_FUN_0072bcf8.c:51-61` | `1A 04` set `+0x12F8`+`+0x1300` |
| 8 | `client_pseudo_c/0072b9b8_FUN_0072b9b8.c:28-41` | `1D 01` cộng `+0x12FC` |
| 9 | `client_pseudo_c/0072ba0c_FUN_0072ba0c.c:27-36` | `1D 02` trừ `+0x12FC` |
| 10 | `client_pseudo_c/0072be10_FUN_0072be10.c:37-40` | **`1D 04` set `+0x12FC`** (phát hiện mới) |
| 11 | `client_pseudo_c/00745acc_FUN_00745acc.c:25-37` + `.asm.txt:15-19` | `1A 05` cộng `+0x1300`, trần `0x77359400` |
| 12 | `client_pseudo_c/00745b14_FUN_00745b14.c:26-39` | `1A 06` trừ `+0x1300` |
| 13 | `client_pseudo_c/0074c4c8_FUN_0074c4c8.c:53-71` | `08 06` set `+0x0502/+0x0508/+0x050E/+0x0513` |
| 14 | `client_pseudo_c/0074cc84_FUN_0074cc84.c:55-70` | `08 07` set `+0x0500/+0x0506/+0x050C/+0x0512/+0x04FC` |
| 15 | `client_pseudo_c/0074c64c_FUN_0074c64c.c:23-40` | `1A 08` cộng `+0x0502`, clamp `<=+0x0500` |
| 16 | `client_pseudo_c/0074c6b4_FUN_0074c6b4.c:32-51` | `1A 0A` cộng `+0x050E`, clamp `<=+0x050C` |
| 17 | `client_pseudo_c/0074c5f4_FUN_0074c5f4.c:23-40` | `1A 09` cộng `+0x0513`, clamp 250 |
| 18 | `client_pseudo_c/00746774_FUN_00746774.c:22-29` | `1A 07` byte + banner `0x7467C8` |
| 19 | `client_pseudo_c/0072c0f0_FUN_0072c0f0.c:24-30` | Gate banner `1A 01/02` (cờ `+0x14` 2 form) |
| 20 | `client_pseudo_c/005d895c_FUN_005d895c.c:59-71` | `Form_Bank_Money` `+0x114`←`+0x12FC`, `+0x118`←`+0x12F8`, `Format('%7d')` |
| 21 | `test/ts_decompile/functions/005d7d18_FUN_005d7d18.c:113-127,162` | `Form_Bank_Money`, `btn_Bank_Gold_R/L` |
| 22 | `test/ts_decompile/functions/00632e3c_FUN_00632e3c.c:58,93` | HUD vẽ `+0x12F8` (y+0x5a) và `+0x1300` (y+0xa6) |
| 23 | `client_pseudo_c/00527674_FUN_00527674.c:191-198,242,261-278` | Trừ chi phí `+0x050E/+0x0502/+0x0508`; aliasing `double[4]` tại `+0x050C` |
| 24 | `client_pseudo_c/jumptable_byte200_0x78A8EE.hex` + `jumptable_0x78A9B6_case_functions.csv` | Ánh xạ MainOp→Case: `0x08`→9, `0x1A`→23, `0x1D`→25, `0x23`→31 |
| 25 | `client_pseudo_c/lit_72b158/16c/238/24c/df8/72be0c/7467c8.hex` | 7 literal **mới giải mã** (mục §1, §2.1, §2.2) |
| 26 | `client_pseudo_c/lit_797ee8_798440.hex` | Giải mã toàn bộ dải nhãn `0x798038..0x7982F8` (VISCII) |
| 27 | `src/encoding.rs:127-236` (`viscii_to_unicode`) | Bảng giải mã VISCII chuẩn (dùng làm chuẩn, **không** dùng bảng viết tay) |
| 28 | `src/server/session.rs:195-199,242-243,280` | Field tiền Rust |
| 29 | `src/server/spawn.rs:325-330,561-564,592-594` | `store_frame` (`23 04`) + `1A 04` login |
| 30 | `src/server/handlers/shops.rs:11-13,42,202,239-262,351-366,568-577` | Luồng tiền shop (chỉ `1A 04`) |
| 31 | `src/server/handlers/trade_storage.rs:13,124-126,707-728` | `GOLD_CAP`, `gold_status_frame`, guard nạp tiền |
| 32 | `src/server/handlers/system.rs:12-18,116-167` | `shop_points_frame` (`42 02`), GM shop trừ `shop_point` |
| 33 | `src/server/handlers/use_item/books.rs:186-194` | `hp_store` in-memory only |
| 34 | `migrations/0001_init.sql:83-87` | Bảng `character_money`: `gold`, `bankgold`, `shoppoint` |
| 35 | `src/db/modern/model.rs:97-99`; `src/db/modern/sqlite/session.rs:27-28,67-69,297,516-518` | Model + đọc/ghi `bank_gold`/`shop_point` |
| 36 | `TS_Server_Bear/TS_Server/Client/TSCharacter.cs:76,78,515-516` | Server C# tham chiếu: **chỉ** `gold` + `gold_bank` (2 loại) |
| 37 | `src/server/handlers/chat.rs:411-416`; `talk.rs:711-716` | Rust builder `1D 04` (set `+0x12FC`) — **đã tồn tại**, chỉ gọi khi mở bank |
| 38 | `src/server/handlers/trade_storage.rs:707-751` | Rust `handle_bank_gold`: `1D 01/02` + `1A 01/02`, guard `GOLD_CAP` 2 chiều, rollback persist — **đúng spec** |
