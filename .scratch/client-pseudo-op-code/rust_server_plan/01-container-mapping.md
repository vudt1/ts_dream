# 01 — Bảng Mapping ChuẨn: Container Vật Phẩm ↔ storagetype ↔ Opcode

> **Đây là tài liệu chuẩn hoá (canonical).** Mọi phase triển khai sau đều phải trích dẫn bảng này.
> Khi có bằng chứng mới (capture `aLogin.exe`), cập nhật tại đây trước, rồi mới sửa code.

---

## 1. Bảng tổng quan — 5 container đang hoạt động

| # | Khái niệm (tiếng Việt) | Tên Rust | `storagetype` | Opcode dump S→C | Sức chứa thực tế | Bằng chứng layout |
|:-:|---|---|:-:|:--:|---|---|
| 1 | **Hòm đồ** (túi chính, hành trang) | `homdo` | `1` | `17 05` | 25 ô (`1..=25`) | ✅ `[C-verified]` stride 12B, có byte slot, có clear loop |
| 2 | **Tiền trang** (rương kho) | `tientrang` | `2` | `1E 01` | 50 ô (`1..=50`) | ✅ `[C-verified]` stride 12B, có byte slot, clear bằng `1E 08` |
| 3 | **Túi đeo** (túi phụ) | `tuideo` | `4` | `17 2F` | 25 ô (`1..=25`) | ✅ `[C-verified]` stride 12B, có byte slot, mảng `player+0xF0C` |
| 4 | **Lưu lang** (kho xe / túi yên ngựa) | `luulang` | `16` | `17 66` | **10 ô** (`1..=10`) | ✅ `[C-verified]` stride 12B, có byte slot, mảng `player+0xF74` (`form_Horse`) |
| 5 | **Trang bị** (đang mặc) | `trangbi` | `8` | `17 0B` | 6 ô (`1..=6`) | ✅ `[C-verified]` stride **10B, KHÔNG byte slot** |

---

## 2. Bảng chi tiết theo container

### 2.1 HÒM ĐỒ — `storagetype = 1` — opcode `17 05`

| Thuộc tính | Giá trị | Bằng chứng |
|---|---|---|
| Handler client | `FUN_0076BFC8` | `[C-verified]` `0076bfc8_FUN_0076bfc8.c:87-203` |
| Mảng đích client | `**player + 0x9E0` | `[C-verified]` `0x0076C1F5: MOV EAX,[EDX + EAX*0x4 + 0x9e0]` |
| Guard slot | `slot <= 0x19` (25) | `[C-verified]` `0x0076C1E3: CMP EAX,0x19` → `_BoundErr` |
| Công thức số record | `n = (Len(P) - 2) / 12` | `[C-verified]` `0x0076C03B: IDIV 0xC` |
| Clear pass | ✅ **CÓ** — quét `1..=25` gọi `VMT+8` xoá trước khi ghi | `[C-verified]` `0x0076BFF7-0x0076C021` |
| Bear tương đương | `inventory.sendItems(23, 5)` → `PacketCreator(23,5)` | `[B-verified]` `TSCharacter.cs:1103` |
| Sức chứa Bear | `new TSItemContainer(this, 25)` | `[B-verified]` `TSCharacter.cs:365` |
| Emitter Rust | `Session::dump_homdo()` | `[R]` `session.rs:425-427` |
| Trạng thái | ✅ **KHỚP byte-for-byte** | — |

> **Vì `17 05` có clear pass**, server chỉ cần gửi các ô **đang có đồ**; ô trống không cần
> record trống. Đây là lý do `dump_table` bỏ qua `id == 0` là đúng.

---

### 2.2 TIỀN TRANG — `storagetype = 2` — opcode `1E 01`

| Thuộc tính | Giá trị | Bằng chứng |
|---|---|---|
| Handler client | `FUN_007707A0` | `[C-verified]` `007707a0_FUN_007707a0.c:78-200` |
| Mảng đích client | `**player + 0xA48` | `[C-verified]` `:182: MOV EDX,[0x007da7bc]; MOV EAX,[EDX + 0xa48 + uVar7*4]` |
| Guard slot | `slot <= 0x32` (50) | `[C-verified]` `:179: CMP EAX,0x32` |
| Công thức số record | `n = (Len(P) - 2) / 12` | `[C-verified]` `:85: MOV EAX, local_28 / 0xC` |
| Clear pass | ❌ **KHÔNG có** | `[C-verified]` không có vòng lặp clear như `0076BFC8` |
| Bear tương đương | `storage.sendItems(30, 1)` = `1E 01` | `[B-verified]` `TSCharacter.cs:1105` |
| Sức chứa Bear | `new TSItemContainer(this, 250)`, wire clamp `Math.Min(len, 50)` | `[B-verified]` `TSCharacter.cs:367`, `TSItemContainer.cs:143` |
| Emitter Rust | `Session::dump_tientrang()` | `[R]` `session.rs:435-437` |
| Trạng thái | ✅ **KHỚP layout 12B** | — |

> ⚠️ **Phát hiện quan trọng:** vì `1E 01` **không có clear pass**, nếu server từng gửi 50 ô và
> lần sau chỉ gửi 20 ô, 30 ô cũ **vẫn còn trên client**. Xem §7 — cần frame `1E 08` hoặc reset.

---

### 2.3 TÚI ĐEO — `storagetype = 4` — opcode `17 2F`

| Thuộc tính | Giá trị | Bằng chứng |
|---|---|---|
| Handler client | `FUN_00769B68` | `[C-verified]` `0x00769B68` |
| Call-site xác nhận | `0x00790A1F: CALL 0x00769B68` với `gvar_007DA0D0` | `[C-verified]` `007902fb_FUN_007902fb.asm.txt:346` |
| Mảng đích client | `**player + 0xF0C + slot*4` | `[C-verified]` `0x00769D63: MOV EAX,[EDX + EAX*4 + 0xf0c]` |
| Guard slot | `slot <= 0x19` (25) | `[C-verified]` `0x00769D51: CMP EAX, 0x19` → `_BoundErr` |
| Công thức số record | `n = (Len(P) - 2) / 12` | `[C-verified]` `0x00769BA9: IDIV 0xC` (stride 12B, có byte slot) |
| Clear pass | ❌ **KHÔNG có** (xóa lẻ qua subop `17 31`) | `[C-verified]` `00769B68` không có loop clear |
| Bear tương đương | `bag.sendItems(23, 47)` = `17 2F` | `[B-verified]` `TSCharacter.cs:1104` |
| Sức chứa Bear | `new TSItemContainer(this, 25)` | `[B-verified]` `TSCharacter.cs:366` |
| Emitter Rust | `Session::dump_tuideo()` | `[R]` `session.rs:440-442` |
| Trạng thái | ✅ **ĐÃ XÁC MINH: 12B, 25 ô** | — |

> **Cập nhật & xóa 1 ô:**
> - `17 30` (`FUN_007699D4`): Cập nhật 1 ô trong túi đeo.
> - `17 31` (`FUN_0076AECC`): Xóa 1 ô trong túi đeo (`[17][31][slot: 1B]`, gọi `VMT+8` tại `player+0xF0C[slot]`).

---

### 2.4 LƯU LANG — `storagetype = 16` — opcode `17 66`

| Thuộc tính | Giá trị | Bằng chứng |
|---|---|---|
| Handler client | `FUN_0076A320` | `[C-verified]` `0076a320_FUN_0076a320.c` tại `0x0076A320` |
| Call-site xác nhận | `0x00790D18: CALL 0x0076A320` với `gvar_007DA0D0` | `[C-verified]` `007902fb_FUN_007902fb.asm.txt:537` |
| Mảng đích client | `**player + 0xF74 + slot*4` (`form_Horse`) | `[C-verified]` `0x0076A51B`, `0x0062F6FC` (kho hàng xe/ngựa) |
| Guard slot | **`slot <= 0x0A` (10 ô)** | `[C-verified]` `0076a320.c:185: if (10 < uVar8) _BoundErr` |
| Công thức số record | `n = (Len(P) - 2) / 12` | `[C-verified]` `0076a320.c:81: uVar2 = (int)uVar2 / 0xc` |
| Clear pass | ❌ **KHÔNG có** (xóa lẻ qua subop `17 68`) | `[C-verified]` không có clear loop |
| Layout record | **12 bytes có byte slot** (`slot:1B, id:2B, b1..b5:5B, texp:4B`) | `[C-verified]` `0076a320.c:89-192` |
| Bear tương đương | ❌ Bear không dùng chức năng này | `[B]` Bear dùng storage mở rộng khác |
| Emitter Rust | `Session::dump_luulang()` | `[R]` `session.rs:445-447` |
| Trạng thái | ✅ **ĐÃ XÁC MINH: 12B, 10 Ô** (Cần sửa sức chứa 10 ô) | — |

> **Cập nhật & xóa 1 ô:**
> - `17 67` (`FUN_0076A18C`): Cập nhật 1 ô trong Lưu lãng.
> - `17 68` (`FUN_0076AF6C`): Xóa 1 ô trong Lưu lãng (`[17][68][slot: 1B]`, gọi `VMT+8` tại `player+0xF74[slot]`).

**Kết luận:** Client thực sự parse `17 66` với layout 12B có byte slot, nhưng sức chứa giới hạn cứng là **10 ô**. Server chỉ cần giới hạn số ô `luulang <= 10` là an toàn tuyệt đối.

---

### 2.5 TRANG BỊ — `storagetype = 8` — opcode `17 0B` 🔴 **BUG ĐÃ XÁC NHẬN**

| Thuộc tính | Giá trị | Bằng chứng |
|---|---|---|
| Handler client | `FUN_0077CC2C` | `[C-verified]` `0077cc2c_FUN_0077cc2c.asm.txt` |
| **Stride** | **10 byte** — `IDIV 0xA` | `[C-verified]` `0x0077CC6E: IDIV 0xA` |
| **Byte slot đầu** | ❌ **KHÔNG CÓ** — record bắt đầu bằng `itemCode` | `[C-verified]` `:0077CCBA: LStrCopy(RP, base+2, 2)` — offset 0 |
| Định tuyến | Theo **body position** (`FUN_0072FA98`), không theo slot byte | `[C-verified]` `:0077CDE7` |
| Mảng đích client | `**player + 0x2C` (7 ô) qua `FUN_0072FA98` | `[C-verified]` guard `6 < uVar2 → _BoundErr` |
| Bear tương đương | `sendEquip()` → `PacketCreator(23, 11)` | `[B-verified]` `TSCharacter.cs:5281-5297` |
| Bear ghi | `add16(Itemid) + add8(quantity) + add8(other_type) + add8(100+other_val) + add8(anti) + add32(exp)` = **10B, không slot** | `[B-verified]` `TSCharacter.cs:5288-5293` |
| Emitter Rust | `Session::dump_trangbi()` → **dùng `dump_table` 12B** | `[R]` `session.rs:430-432` |
| Trạng thái | 🔴 **SAI — lệch 2 byte mỗi record** | — |

**Ví dụ hỏng cụ thể** (1 trang bị):

```
Rust gửi (12B):  05  C7 4E  01  00  00  64  00  00 00 00 00
                 ↑slot ↑id  ↑cnt ↑dob ↑long ↑gia ↑khang  ↑texp(4)

Client đọc (10B):    C7 4E  01  00  00  64  00  00 00 00
                     ↑id(2) ↑b1 ↑b2 ↑b3 ↑b4   ↑q(4)
```

Client đọc `q` 4 byte từ vị trí `khang`+`texp[0..2]` → sai hoàn toàn.

---

## 3. Bảng mapping `storagetype` ↔ container

### 3.1 Định nghĩa enum hiện tại

`[R]` `src/db/modern/model.rs:14-21`

```rust
pub enum StorageType {
    Bag = 1,        // Hòm đồ      → 17 05
    Bank = 2,       // Tiền trang  → 1E 01
    Secondary = 4,  // Túi đeo     → 17 2F
    Equip = 8,      // Trang bị    → 17 0B
    Warehouse = 16, // Lưu lang    → 17 66
}
```

**Ghi chú quan trọng:** comment `model.rs:12-13` nói *"Bit values match the Kotlin reference"* —
đây là **giả định không có nguồn**. Bear **không hề có** `StorageType`/`storagetype`; nó phân biệt
container bằng **field C# nào giữ dữ liệu** (`inventory` / `bag` / `storage` / `equipment`) và
**opcode nào đẩy**.

### 3.2 Bảng tra cứu 2 chiều

| `storagetype` | Tên Rust | `StorageType` | Opcode dump | Handler client | Layout |
|:-:|---|---|:--:|---|---|
| `1` | `homdo` | `Bag` | `17 05` | `FUN_0076BFC8` | 12B slot-led (25 ô, có clear pass) |
| `2` | `tientrang` | `Bank` | `1E 01` | `FUN_007707A0` | 12B slot-led (50 ô, clear bằng `1E 08`) |
| `4` | `tuideo` | `Secondary` | `17 2F` | `FUN_00769B68` | 12B slot-led (25 ô, xóa lẻ `17 31`) |
| `8` | `trangbi` | `Equip` | `17 0B` | `FUN_0077CC2C` | **10B không slot** (6 ô) |
| `16` | `luulang` | `Warehouse` | `17 66` | `FUN_0076A320` | **12B slot-led** (10 ô, xóa lẻ `17 68`) |

### 3.3 Nơi ánh xạ này đang bị **nhân bản** (code smell)

| Nơi | File:line | Ghi chú |
|---|---|---|
| Enum định nghĩa | `db/modern/model.rs:14-21` | nguồn duy nhất *có type safety* |
| Hardcode tuple lúc load | `db/modern/sqlite/session.rs:72-76` | `load_items(id, 1)`, `(id, 2)`, `(id, 4)`, `(id, 8)`, `(id, 16)` |
| Hardcode tuple lúc save | `db/modern/sqlite/session.rs:303-309` | `(1, &session.homdo)`, … |
| Hardcode tuple lúc seed | `db/modern/sqlite/session.rs:542-547` | lặp lại lần nữa |
| String→u8 legacy | `db/persist.rs:135-144` | `"homdo"=>1, "tientrang"=>2, "trangbi"=>8, "tuideo"=>4, "luulang"=>16` |
| Schema comment | `migrations/0001_init.sql:92-93, 98` | ghi chú song song |

→ **6 nơi** phải giữ đồng bộ. Đây là lý do `05-phased-roadmap.md` Phase 1 đề xuất gom về một
bảng metadata duy nhất.

---

## 4. Layout byte chi tiết

### 4.1 Format A — **12 byte, slot-led** (`17 05`, `1E 01`, `17 2F`)

```
offset  len  trường            ghi chú
──────  ───  ────────────────  ──────────────────────────────────
  0      1   slot:u8            1-based; guard 0x19 (17 05) hoặc 0x32 (1E 01)
  1      2   itemCode:u16 LE    đọc bằng FUN_0077EB9C (Word LE)
  3      1   b1                 Bear: quantity
  4      1   b2                 Bear: 0 (unused)
  5      1   b3                 Bear: other_type  (Rust: long_val)
  6      1   b4                 Bear: 100 + other_val (Rust: giatri_long + 100)
  7      1   b5                 Bear: anti (Rust: khang)
  8      4   q:u32 LE          đọc bằng FUN_0077ED68 (DWORD LE); Bear: exp (Rust: texp)
──────  ───
        12
```

`[B-verified]` ánh xạ ngữ nghĩa từ `TSItemContainer.sendItems()` (`TSItemContainer.cs:148-155`):
```csharp
packetCreator.addByte((byte)(i + 1));                                        // slot
packetCreator.add16(items[i].Itemid);                                        // itemCode
packetCreator.addByte(items[i].quantity);                                   // b1 = quantity
packetCreator.addByte(0);                                                    // b2 = 0
packetCreator.addByte(items[i].equip?.other_type ?? 0);                     // b3
packetCreator.addByte((byte)(100 + (byte)(items[i].equip?.other_val ?? 0)));// b4
packetCreator.addByte(items[i].equip?.anti ?? 0);                           // b5
packetCreator.add32(items[i].equip?.exp ?? 0);                              // q
```

> ⚠️ **Bias +100 tại b4:** `(giatri_long as u16 + 100) as u8` — nếu `giatri_long > 155` sẽ **tràn
> vòng**. Bear có **cùng** hành vi (`(byte)(100 + (byte)other_val)`) nên đây là **chủ ý giao thức**,
> không phải bug. Nhưng cần clamp `giatri_long ≤ 155` ở tầng nghiệp vụ.

### 4.2 Format B — **10 byte, không slot** (`17 0B` trang bị)

```
offset  len  trường            ghi chú
──────  ───  ────────────────  ──────────────────────────────────
  0      2   itemCode:u16 LE    KHÔNG có byte slot
  2      1   b1                 Bear: quantity
  3      1   b2                 Bear: other_type
  4      1   b3                 Bear: 100 + other_val
  5      1   b4                 Bear: anti
  6      4   q:u32 LE          Bear: exp
──────  ───
        10
```

`[B-verified]` `TSCharacter.sendEquip()` (`TSCharacter.cs:5288-5293`):
```csharp
packetCreator.add16(equipment[i].Itemid);
packetCreator.addByte(equipment[i].quantity);
packetCreator.addByte(equipment[i]?.other_type ?? 0);
packetCreator.addByte((byte)(100 + (byte)(equipment[i]?.other_val ?? 0)));
packetCreator.addByte(equipment[i]?.anti ?? 0);
packetCreator.add32(equipment[i]?.exp ?? 0);
```

> **Slot định tuyến bằng body position** (`Item.dat` row byte `+0x34`, giá trị `0..6`).
> Vì vậy **thứ tự record trong `17 0B` phải là thứ tự vị trí trang bị 1..6** — không tùy ý.

### 4.3 So sánh hai format

| | Format A (12B) | Format B (10B) |
|---|---|---|
| Byte slot | ✅ có | ❌ không |
| Stride | 12 | 10 |
| Dùng cho | `17 05`, `1E 01`, `17 2F`, `17 05` delta, `1E 0C` | `17 0B` |
| Slot → vị trí | byte `slot` truyền tường minh | thứ tự array / body position |
| Bear | `sendItems()` | `sendEquip()` |

---

## 5. Container client **chưa** có trong Rust

`[C-verified]` — ba mảng slot client không được Rust đụng tới:

| Opcode | Handler | Mảng đích | Sức chứa | Layout | Bear? | Nghiệp vụ suy đoán |
|:--:|---|---|:-:|---|---|---|
| `1E 09` | `FUN_007714A0` | `**player + 0xB14` | 51 ô (`≤ 0x32`) | 12B batch | ❌ | **Ba Đậu Yêu Balo** — `spec/server_main_opcode.md:28` ghi `0x1E = "Rương Nhà Trọ & Ba Đậu Yêu Balo"` |
| `1E 0C` | `FUN_0077B72C` | `**player + 0xDC0` | 26 ô (`≤ 0x19`) | 12B single | ❌ | **Thủy binh bank** — `economy/05_special_economy.md:498` đã xác định |
| `1E 08` | `FUN_0076FDB8` | `**player + 0xA48` | quét 1..50 | không payload | ❌ | **mass-clear tiền trang** — xem §7 |

Ngoài ra còn các subop delta của `0x1E` mà Rust chưa dùng:

| Subop | Payload | Tác dụng `[C-verified]` |
|:-:|---|---|
| `1E 02` | `[code:2][b1..b5][q:4]` (11B, không slot) | `FUN_0072BE9C` — tự đặt vào ô trống `+0xA48` |
| `1E 03` | `[code:2][b:1]` (5B) | `FUN_0072C038` |
| `1E 04` | 1× 12B record | ghi 1 ô của `+0xA48` |
| `1E 05` | `[slot:1][n:1]` (4B) | trừ `n` khỏi counter `slotObj+7` của `+0xA48[slot]` |
| `1E 0A` | 11B | `FUN_00749C78` |
| `1E 0B` | `[slot:1][n:1]` | trừ counter trên `+0xB14[slot]` |
| `1E 0D` | `[slot:1][n:1]` | trừ counter trên `+0xDC0[slot]` |

---

## 6. Bảng tổng hợp mọi frame `0x17` / `0x1E` do Rust phát

### 6.1 Frame **đúng**

| Frame | File:line | Layout | Khớp client? |
|---|---|---|---|
| `17 05` dump | `session.rs:425-427` → `:450-468` | 12B | ✅ |
| `1E 01` dump | `session.rs:435-437` → `:450-468` | 12B | ✅ |
| `17 2F` dump | `session.rs:440-442` → `:450-468` | 12B | 🟡 giả định |
| `17 05` delta | `inventory.rs:69-85` (`item_added_frame`) | 12B không slot | ✅ |
| `17 09` delta | `inventory.rs:189-194` | `slot:1 + qty:1` | ✅ |
| `28 01` hotkey | `session.rs:480-493` | `02 + le16(id) + slot` (4B) | ✅ xác minh ở `05ab3f8` |

### 6.2 Frame **sai hoặc nghi ngờ**

| Frame | File:line | Vấn đề | Mức |
|---|---|---|:-:|
| `17 0B` dump | `session.rs:430-432` | 12B thay vì 10B, thêm byte slot | 🔴 |
| `17 66` dump | `session.rs:445-447` | layout không có bằng chứng | 🔴 |
| `17 1E` stall | `shops.rs:465-466`, `:482` | gửi `le32(price)`; client `FUN_00768638` ghi vào `slotObj+0x20` là **item id** → cần xem lại (xem `03-wire-codecs.md` §5) | 🟠 |
| `17 20` stall close | `shops.rs:500` | uid lệch 1 byte — `economy` gap G18 | 🟠 |

---

## 7. Vấn đề clear-state (phát hiện mới)

`[C-verified]` So sánh hai parser:

| Parser | Có clear pass? | Hệ quả |
|---|:---:|---|
| `FUN_0076BFC8` (`17 05`) | ✅ có | quét `1..=25` gọi `VMT+8` xoá trước → gửi ô trống không cần record trống |
| `FUN_007707A0` (`1E 01`) | ❌ **không** | ô đã có trên client **không tự biến mất** khi server ngừng gửi |

**Kịch bản lỗi:** người chơi có 40 vật phẩm trong tiền trang → login lần 1 nhận 40 record.
Sau đó rút 20 vật phẩm ra (`1E 02`) → lần login sau chỉ còn 20 record gửi đi, nhưng **20 ô cũ vẫn
hiển thị trên client**.

**Hai cách khắc phục:**

1. **Frame `1E 08`** — `[C-verified]` `FUN_0076FDB8` quét `1..=50` và gọi
   `FUN_0077250C(slotObj, 50)` cho từng ô; hàm này set counter về 0 và gọi `VMT+8` (clear).
   → Gửi `1E 08` **trước** `1E 01` trong Logined1.
2. **Gửi record "trống"** cho mọi ô không có đồ — nhưng format 12B không có cách biểu diễn
   "ô trống" rõ ràng (itemCode = 0 chưa chắc client chấp nhận).

→ **Khuyến nghị: dùng cách 1 (`1E 08`)**, vì nó là con đường protocol chính thức.

Tương tự, cần kiểm tra `17 2F` và `17 66` có clear pass hay không — **hiện chưa biết** vì thiếu
body. Nếu không có, chúng **cũng** cần cơ chế clear tương tự trước khi dump.

---

## 8. Quy ước slot (1-based)

`[B-verified]` Bear xác nhận rõ ràng:

```csharp
// TSItemContainer.cs:148
packetCreator.addByte((byte)(i + 1));   // array index i → wire byte i+1
// TSItem.cs:13-15
public byte slot;         // 1-based
public byte index_slot;   // slot - 1
```

`[R]` Rust tuân thủ: `free_slot` quét `1..=25` (`inventory.rs:31`), PK schema là
`(playerid, storagetype, slot)` (`0001_init.sql:120`), `add_storage_item` quét `1..=50`
(`trade_storage.rs:174`).

**Kết luận: slot 1-based trên cả wire lẫn DB.** Không cần chuyển đổi.

---

## 9. Ma trận client container ↔ offset bộ nhớ

`[C-verified]` Bảng đầy đủ, từ `FUN_0078A89C` dispatcher + các body:

| Offset | Sức chứa | Guard | Handler ghi vào | Subop tương ứng |
|---|:-:|---|---|---|
| `player+0x2C` | 7 | `≤ 6` | `FUN_0072FA98` | `17 0B`, `17 0C`, `17 15` |
| `player+0x9E0` | 26 | `≤ 0x19` | `FUN_0076BFC8`, `FUN_0076974C`, `FUN_00730F4C` | `17 05`, `17 08`, `17 1E` |
| `player+0xA48` | 51 | `≤ 0x32` | `FUN_007707A0`, `FUN_007705D8`, `FUN_0076FDB8` | `1E 01`, `1E 04`, `1E 08` |
| `player+0xB14` | 51 | `≤ 0x32` | `FUN_007714A0`, `FUN_007700BC` | `1E 09`, `1E 0B` |
| `player+0xDC0` | 26 | `≤ 0x19` | `FUN_0077B72C`, `FUN_0077B938` | `1E 0C`, `1E 0D` |
| `player+0x57C` | 5 | `≤ 4` | `FUN_0077D290` | `17 15` (bộ trang bị) |
| `petmgr+0x158` | 21 | `≤ 0x14` | `case_017` | `13 01/02` |

---

## 10. Đề xuất chuẩn hoá lưu trữ

Xem chi tiết ở [`04-db-persistence.md`](./04-db-persistence.md). Tóm tắt:

```
storagetype (DB)          wire opcode          layout
─────────────            ───────────          ──────
1  Bag         ─────────▶ 17 05                12B slot-led   ✅ chuẩn
2  Bank        ─────────▶ 1E 01                12B slot-led   ✅ chuẩn
4  Secondary   ─────────▶ 17 2F                12B slot-led   ⚠ cần capture
8  Equip       ─────────▶ 17 0B                10B KHÔNG slot  🔴 đang sai
16 Warehouse   ─────────▶ 17 66                ❓ chưa biết     🔴 cần capture
```

**Khuyến nghị chiến lược cho `17 66`** (xem `05-phased-roadmap.md` §3.2):

- **Không** xoá `17 66` ngay — có thể client cần nó để dựng UI.
- **Tạm dừng gửi** `17 66` trong Logined1 cho tới khi có capture, vì gửi layout sai còn tệ hơn
  không gửi (client có thể hiển thị vật phẩm ma ở ô sai).
- Chạy song song hai cấu hình qua feature flag, chờ capture quyết định.

---

## 11. Ma trận liên kết 3 chiều: DB ↔ Opcode ↔ Frame

Bảng tra cứu nhanh cho developer: cột DB nào sinh ra frame nào, và ngược lại.
`Session` field ở giữa là điểm nối.

```
┌─────────────────────────┬──────────────┬──────────────┬──────────────┬──────────┐
│ DB: inventories         │ Session      │ storagetype  │ Wire opcode  │ Layout   │
├─────────────────────────┼──────────────┼──────────────┼──────────────┼──────────┤
│ storagetype = 1         │ session.homdo │ Bag = 1      │ 17 05        │ 12B/slot │
│ storagetype = 2         │ session.     │ Bank = 2     │ 1E 01        │ 12B/slot │
│                         │ tientrang    │              │              │          │
│ storagetype = 4         │ session.     │ Secondary= 4 │ 17 2F        │ 12B/slot │
│                         │ tuideo       │              │              │ (chưa    │
│                         │              │              │              │  xác minh│
│ storagetype = 8         │ session.     │ Equip = 8    │ 17 0B        │ 10B/không│
│                         │ trangbi      │              │              │  slot ⚠  │
│ storagetype = 16        │ session.     │ Warehouse=16 │ 17 66        │ ❓ chưa   │
│                         │ luulang      │              │              │  biết ❌  │
└─────────────────────────┴──────────────┴──────────────┴──────────────┴──────────┘
```

### 11.1 Đường đi của DB → RAM (đọc)

`[R]` `sqlite/session.rs:72-76`
```rust
session.homdo      = self.load_items(session.db_character_id, 1).await?;
session.tientrang  = self.load_items(session.db_character_id, 2).await?;
session.tuideo     = self.load_items(session.db_character_id, 4).await?;
session.trangbi    = self.load_items(session.db_character_id, 8).await?;
session.luulang    = self.load_items(session.db_character_id, 16).await?;
```

| Bước | Dữ liệu | Hiện trạng |
|:-:|---|---|
| 1 | `storagetype` (bind param) | ✅ đủ |
| 2 | `slot`, `itemid`, `quantity`, `damage` | ✅ đủ |
| 3 | `element`, `elementvalue`, `proofkind`, `growexp` | ❌ **bỏ qua** |
| 4 | 16 cột còn lại | ❌ bỏ qua (chấp nhận tạm) |
| 5 | Template từ `Item.dat` | ❌ **không hydrate** ⇒ `loai`/`lv`/`int1..fai2` = 0 |

### 11.2 Đường đi của RAM → DB (ghi)

`[R]` `sqlite/session.rs:303-314`
```rust
for (storage, items) in [
    (1u8, &session.homdo), (2, &session.tientrang), (4, &session.tuideo),
    (8, &session.trangbi), (16, &session.luulang),
] {
    for item in items.iter().filter(|i| i.id > 0) {
        sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage) VALUES (?,?,?,?,?,?)")
    }
}
```

⚠️ `DELETE FROM inventories WHERE playerid = ?` (`:299`) chạy trước ⇒ mọi cột không nằm trong
danh sách INSERT **bị xoá vĩnh viễn** mỗi lần auto-save.

### 11.3 Đường đi của RAM → Frame (gửi)

| Session field | Emitter | Frame | Cột DB đích |
|---|---|---|---|
| `homdo` | `dump_homdo()` `session.rs:425` | `17 05` | st=1 |
| `tientrang` | `dump_tientrang()` `session.rs:435` | `1E 01` | st=2 |
| `tuideo` | `dump_tuideo()` `session.rs:440` | `17 2F` | st=4 |
| `trangbi` | `dump_trangbi()` `session.rs:430` | `17 0B` | st=8 |
| `luulang` | `dump_luulang()` `session.rs:445` | `17 66` | st=16 |

### 11.4 Đường đi của Frame → Session (nhận từ client)

| Frame C→S | Handler | Mutate Session field | Persist qua |
|---|---|---|---|
| `17 02` pick up | `inventory.rs:87-145` | `homdo` | `persist::upsert_item("homdo")` |
| `17 03` drop | `inventory.rs:147-216` | `homdo` (giảm) | `upsert_item("homdo")` |
| `17 10` move | `inventory.rs:218-273` | `homdo` (đổi slot) | ❌ **không** |
| `17 11` equip | `inventory.rs:275-335` | `homdo`, `trangbi` | `upsert_item` × 2 |
| `17 12` unequip | `inventory.rs:337-404` | `homdo`, `trangbi` | `upsert_item` × 3 |
| `17 15` use item | `use_item/` | `homdo` (+`recompute_stats`) | `upsert_item("homdo")` |
| `17 25` move túi đeo | `inventory.rs:624-636` | ❌ stub | ❌ |
| `1E 01` rút kho | `trade_storage.rs:553-572` | `tientrang`, `homdo` | `persist_sessions_transaction` |
| `1E 02` gửi kho | `trade_storage.rs:573-593` | `homdo`, `tientrang` | `persist_sessions_transaction` |
| `17 51` → lưu lang | `trade_storage.rs:594-623` | `homdo`, `luulang` | `persist_sessions_transaction` |
| `17 52` ← lưu lang | `trade_storage.rs:624-652` | `luulang`, `homdo` | `persist_sessions_transaction` |
| `17 36/37` túi đeo | `inventory.rs:612-636` | ❌ stub | ❌ |
| `17 17` equip pet | `inventory.rs:554-580` | `homdo` (chỉ trừ) | ❌ chưa gắn vào pet |

⚠️ `trade_storage.rs:658-669` có comment sai:
```rust
// TienTrang / LuuLang have no backing tables (0001 usage audit) —
// their pouches persist in Session memory only, so just homdo here.
&["homdo"],
```
⇒ **Sai**: `inventories` **có** cột cho storagetype 2 và 16. Và tham số `_tables` bị
`persist.rs:264-278` **bỏ qua hoàn toàn** — `persist_sessions_transaction` luôn gọi full snapshot
`save()` cho mọi session nên cả 5 túi đều được ghi. Comment phải sửa (đã ghi ở
`02-gap-analysis.md` G-15).

### 11.5 Frame không gắn với `storagetype` nào

| Frame | Ý nghĩa | Session liên quan |
|---|---|---|
| `17 06` | thêm item vào túi (delta, không slot) | `homdo` |
| `17 08` | cập nhật 1 ô của hòm đồ | `homdo` |
| `17 09` | giảm số lượng / xoá ô | mọi container |
| `17 30` | thêm item vào túi đeo (delta) | `tuideo` |
| `17 32` | ack dung lượng túi đeo | `tuideo` |
| `17 48` | ghi 1 ô túi đeo | `tuideo` |
| `17 50` | ack mở form túi đeo | `tuideo` |
| `17 66` | ? | `luulang` (?) |
| `1E 04` | cập nhật 1 ô tiền trang | `tientrang` |
| `1E 05` | giảm số lượng 1 ô kho | `tientrang` |
| `17 1E` | stall mở (self) | `shop` (không phải `inventories`) |
| `17 1F` | stall mở (broadcast) | `shop` |
| `17 20` | stall đóng | `shop` |
| `17 21` | catalog người mua thấy | `shop` |
| `19 03` | offer trade P2P | `homdo` (cả 2 bên) |

⚠️ `17 66` nằm giữa `17 64`/`17 65` (đều gọi `gvar_007DA0D0`, cùng nhóm item-page với `17 05`)
và `17 67`/`17 68` — **cùng họ item-page**, khác với `17 30`/`17 31`/`17 32` (nhóm túi đeo theo
Bear `BagHandle`). Điều này **ủng hộ** giả thuyết `17 66` là frame delta trên item page, không
phải container dump.

---

## 12. Bảng tổng hợp: mọi thứ cần sửa trong file này

| Mục | Nơi định nghĩa | Phase xử lý |
|---|---|---|
| `17 0B` → 10B | [`03-wire-codecs.md`](./03-wire-codecs.md) §2 | Phase 2A |
| `17 66` → tạm dừng | [`03-wire-codecs.md`](./03-wire-codecs.md) §3 | Phase 2A |
| Gộp 6 bản sao `storagetype` | [`04-db-persistence.md`](./04-db-persistence.md) §5 | Phase 1 |
| `1E 08` clear frame | [`03-wire-codecs.md`](./03-wire-codecs.md) §4 | Phase 2A |
| Sửa comment sai ở `trade_storage.rs:658` | §11.4 | Phase 1 |
| Thêm `1E 09` / `1E 0C` | §5 | Phase 6 |
| Clamp `giatri_long ≤ 155` | [`04-db-persistence.md`](./04-db-persistence.md) §6 | Phase 3 |
| Xác minh `17 64..68` (item-page family) | §11.5 | Phase 0 |