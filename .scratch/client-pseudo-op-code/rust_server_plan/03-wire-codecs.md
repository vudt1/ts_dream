# 03 — Đặc Tả Wire Codec: Sửa Serializer

> Bản đặc tả byte-level cho Phase 2. Mọi thay đổi ở đây đều **độc lập DB** ⇒ có thể chạy song
> song với Phase 3 (`04-db-persistence.md`).
> Quy ước: `F444` + `len LE16` + opcode + sub + body, toàn frame XOR `0xAD`.
> `len` đếm mọi byte sau header 4 byte, tức `opcode + sub + body`.

---

## 1. Frame primitive

`[R]` `src/protocol/mod.rs:411-414`
```rust
pub fn frame(code: &str, body: &str) -> String {
    let total_len = (code.len() + body.len()) / 2;
    format!("F444{}{code}{body}", encoder::le16(total_len as u16))
}
```

`code` là chuỗi hex: 2 ký tự = 1 byte. `frame("17", body)` = main không có sub;
`frame("1705", body)` = main + sub. Frame rỗng: `frame("1705", "")` = `F44402001705`.

---

## 2. `17 0B` — Trang bị: 12 byte → 10 byte (P0)

### 2.1 Vấn đề

Hiện tại `dump_trangbi` gọi `dump_table` (`session.rs:450-468`) xuất record 12 byte kèm byte slot.
Client `FUN_0077CC2C` chia `IDIV 0xA`.

### 2.2 Đặc tả mới

```
Frame = [F4 44] [len LE16] [17] [0B] [entry × N]

entry (10 byte) — không có byte slot:
  off  len  trường        nguồn Bear        nguồn Rust
  ───  ───  ────────────  ────────────────  ───────────────────
   0    2   itemCode LE    add16(Itemid)     item.id
   2    1   quantity       add8(quantity)    item.count
   3    1   other_type     add8(other_type)  item.long_val
   4    1   100+other_val  add8(100+val)    (item.giatri_long + 100) as u8
   5    1   anti           add8(anti)        item.khang
   6    4   exp LE         add32(exp)        item.texp
```

Bằng chứng byte-level `[C-verified]` `0077cc2c_FUN_0077cc2c.asm.txt`:
```asm
0077cc68: MOV ECX,0xa
0077cc6b: CDQ
0077cc6e: IDIV ECX                      ; n = (Len(RP)-1)/10
0077cc93: IMUL EAX,[EBP-0x10],0xa        ; base = k*10
0077cc9e: ADD EAX,0x2                   ; base+2 (bỏ qua byte sub ở vị trí 0)
0077ccba: CALL 0x0077eb9c               ; Word LE tại base+2 → itemCode
...
0077ccdd5: PUSH 0x0                     ; param_4 = 0
0077cde7: MOV EAX,[0x007da7bc]           ; player
0077cdee: MOV CL,[EBP-0x19]              ; R+2 → byte thứ 3
0077cdf1: CALL 0x0072fa98               ; định tuyến theo body position
```

### 2.3 Quyết định thiết kế: `doben` đi đâu?

Rust có 9 field instance (`doben`, `long_val`, `giatri_long`, `khang`, `texp`) nhưng format 10B
chỉ có **4 byte** sau `itemCode` trước `exp`.

Bear format 10B **không** truyền `duration`/`damage` ở frame base. Nó gửi thêm frame
single-entry `23 11` cho gear có "soul" (`TSCharacter.cs:1390-1409 expitem()`), thay
`other_type`/`other_val` bằng `elem_type`/`elem_val`.

**Khuyến nghị:**

| Field Rust | Đưa vào 10B | Giữ trong DB | Ghi chú |
|---|:-:|:-:|---|
| `id` | ✅ offset 0-1 | ✅ | |
| `count` | ✅ offset 2 | ✅ | |
| `doben` | ❌ **không** | ✅ `damage` | client không có chỗ; giữ DB để dùng sau |
| `long_val` | ✅ offset 3 (`other_type`) | ✅ `element` | |
| `giatri_long` | ✅ offset 4 (+100) | ✅ `elementvalue` | |
| `khang` | ✅ offset 5 (`anti`) | ✅ `proofkind` | |
| `texp` | ✅ offset 6-9 | ✅ `growexp` | |

⚠️ **Quyết định cần chốt:** `doben` bị bỏ khỏi wire `17 0B`. Nếu client cần hiển thị độ bền ở ô
trang bị, phải dùng đường khác (ví dụ `17 0C` `FUN_0077D0B9` đọc `player+0x2C[slot]+0x4`).
**Ghi nhận là quyết định thiết kế, cần xác nhận khi capture.**

### 2.4 Thứ tự record

Vì không có byte slot, **thứ tự record = thứ tự vị trí trang bị**. Client định tuyến qua
`FUN_0072FA98(player, code, R+2, 0, q, grade, R+5, R+4, R+3)` — nó tự tra `Item.dat` để biết
`code` thuộc vị trí nào (`equippos` byte `+0x34`, giá trị `0..6`).

⇒ Rust **phải** sắp xếp `trangbi` theo `slot` tăng dần trước khi serialize. `dump_table` hiện
giữ thứ tự `Vec` (thứ tự chèn), **không** bảo đảm.

### 2.5 Code đề xuất

```rust
/// Frame `170B` — trang bị đang mặc.
///
/// **10 byte/record, KHÔNG byte slot** (client `FUN_0077CC2C` chia `IDIV 0xA`;
/// Bear `TSCharacter.sendEquip()` `add16+add8×4+add32`). Thứ tự record phải
/// theo `slot` tăng dần vì client định tuyến bằng body position trong Item.dat.
pub fn dump_trangbi(&self) -> String {
    let mut equipped: Vec<&InventoryItem> = self.trangbi.iter().filter(|i| i.id > 0).collect();
    equipped.sort_by_key(|i| i.slot);
    let mut entries = String::new();
    for item in equipped {
        entries.push_str(&format!(
            "{}{:02X}{:02X}{:02X}{:02X}{}",
            encoder::le16(item.id),
            item.count,
            item.long_val,
            (item.giatri_long as u16 + 100) as u8,
            item.khang,
            encoder::le32(item.texp)
        ));
    }
    crate::protocol::frame("170B", &entries)
}
```

⚠️ **Không** dùng chung `dump_table` nữa. Xem `05-phased-roadmap.md` §3.1 về việc tách
`dump_table` thành `encode_entry_12` / `encode_entry_10` để tránh tái sử dụng sai.

---

## 3. `17 66` — Lưu lang: tạm dừng (P0)

### 3.1 Vì sao không xoá hẳn

| Phương án | Rủi ro |
|---|---|
| Xoá hẳn khỏi code | Nếu client cần `17 66` để dựng UI lưu lãng → UI không mở được, người chơi mất chức năng |
| **Tạm dừng gửi** (khuyến nghị) | Client giữ UI ở trạng thái rỗng; chức năng tạm không dùng được nhưng **không hiển thị sai** |
| Gửi sai layout (hiện tại) | Client có thể hiển thị vật phẩm ở ô sai → mất vật phẩm người chơi |

⇒ Chọn **tạm dừng**. Frame vẫn được sinh ra (giữ API), nhưng không đẩy vào Logined1.

### 3.2 Cấu hình

Thêm vào `Session` một cờ feature (tốt nhất: đọc từ `config.rs`):
```rust
// config.rs
/// Gửi frame `17 66` (kho lưu lãng) trong Logined1.
///
/// Mặc định `false`: layout `17 66` chưa được xác minh bằng decompile client
/// (thiếu body `FUN_0076A320`) và Bear không hề gửi frame này. Bật lên chỉ
/// sau khi capture `aLogin.exe` xác nhận layout. Xem
/// `.scratch/client-pseudo-op-code/rust_server_plan/01-container-mapping.md` §2.4.
pub send_luulang_frame: bool,
```

Rồi trong `spawn.rs:551-556`:
```rust
// 10. Step 10: Inventory dumps
frames.push(s.dump_homdo());
frames.push(crate::protocol::frame("1E08", ""));   // G-08: clear +0xA48
frames.push(s.dump_tientrang());
frames.push(s.dump_tuideo());
if cfg.send_luulang_frame {
    frames.push(s.dump_luulang());
}
frames.push(s.dump_trangbi());
```

### 3.3 Nhánh thay thế nếu capture không khả thi

`[C-verified]` có **một** container 51 ô khác đã có body đầy đủ: `1E 09` (`FUN_007714A0`,
`player+0xB14`, guard `≤ 0x32`, record 12B — giống hệt `1E 01`).

⇒ Nếu không xác minh được `17 66`, **chuyển chức năng lưu lãng sang `1E 09`** với
`storagetype = 16`. Chi tiết ở `05-phased-roadmap.md` §3.2.

---

## 4. `1E 08` — Clear tiền trang (P1)

### 4.1 Bằng chứng

`[C-verified]` `client_pseudo_c/0076fdb8_FUN_0076fdb8.c:25-33`
```c
local_c = 1;
do {
  if (0x32 < local_c) { uVar1 = _BoundErr(local_c); }        // guard ≤ 50
  FUN_0077250c(*(int **)(*(int *)gvar_007DA7BC + 0xa48 + uVar1 * 4), 0x32);
} while (local_c != 0x33);                                    // 1..50
```

`[C-verified]` `test/ts_decompile/functions/0077250c_FUN_0077250c.c:29-52`
```c
uVar3 = *(byte *)((int)param_1 + 7) - param_2;   // counter -= 50
if ((int)uVar3 < 1) { *(byte*)((int)param_1+7) = 0; }
else { *(byte*)((int)param_1+7) -= (byte)param_2; }
if (*(byte*)((int)param_1+7) == 0) { (**(code **)(*param_1 + 8))(); }  // VMT+8 = clear
```

⇒ `1E 08` quét 50 ô của `player+0xA48`, trừ 50 khỏi counter mỗi ô; counter về 0 thì gọi `VMT+8`
(xoá ô). Về mặt quan sát được: **xoá sạch tiền trang**.

⚠️ **Lưu ý đính chính:** `opcode_1e.md` §4.7 gọi đây là "trừ 50" chứ không phải "clear". Đúng về
mặt cơ chế, nhưng **hiệu ứng quan sát được là clear** (counter ≤ 50 nên luôn về 0). Frame này
an toàn để dùng làm reset.

### 4.2 Code

```rust
/// Frame `1E 08` — mass-clear container `player+0xA48` (50 ô, tiền trang).
///
/// Client `FUN_0076FDB8` quét `1..=50` và gọi `FUN_0077250C(slotObj, 50)`; hàm đó
/// set counter về 0 rồi gọi `VMT+8` (clear ô). Vì `FUN_007707A0` (`1E 01`)
/// **không có clear pass**, frame này là cách duy nhất xoá ô cũ trước khi dump.
pub fn clear_tientrang_frame() -> String {
    crate::protocol::frame("1E08", "")
}
```

Đặt trong Logined1 **trước** `1E 01`.

### 4.3 `17 2F` và `17 66` có cần clear không?

**Chưa biết** — cần body `FUN_00769B68` / `FUN_0076A320`. Nếu chúng không có clear pass tương
tự `FUN_0076BFC8`, thì:
- `17 2F` cần một frame clear tương đương — Bear không có ⇒ có thể phải dùng `17 09` (decrement)
  cho từng ô, hoặc chấp nhận giới hạn.
- `17 66` đã tạm dừng nên không cần.

Ghi vào backlog. Xem `06-tests-and-capture.md` §4.

---

## 5. `17 1E` — Stall: kiểm chứng field 4-byte

### 5.1 Layout client

`[C-verified]` `client_pseudo_c/00730f4c_FUN_00730f4c.c:89-168`
```c
local_d  = iVar5[iVar1];                    // RP[1] = name_len
_LStrCopy(local_c, 3, local_d, &local_138); // RP[2..1+N] = name
_LStrToString(local_134, local_138, 0xff);
_PStrNCpy(local_33, local_134, 0xe);        // tên hiển thị, cắt 14 ký tự

uVar2    = local_d + 3;
local_e  = iVar5[uVar2];                    // RP[N+2] = stall id / image
uVar3    = Len(RP) - (N+3);
local_1c = (int)uVar3 / 5;                  // n = (Len(RP)-(N+3))/5

uVar2 = local_d + 4;
for (k = 0; k < local_1c; k++) {
  local_f = iVar5[(k*5) + uVar2];            // slot
  local_14 = FUN_0077ef7c(local_13c);        // ★ DWORD LE
  FUN_00768638(*(int *)(*(int*)gvar_007DA7BC + 0x9e0 + local_f*4), local_14);
}

FUN_0071e164(*gvar_007DA7BC, local_33, local_e);   // mở UI vendor
_LStrCat3(&local_144, "Stall", IntToStr(local_e));  // caption "Stall<N>"
```

Wire:
```
[17] [1E]
[name_len:1]
[name: name_len]
[stall_id:1]
[(slot:1, value:4 LE) × N]        N = (Len(P) - (3 + name_len)) / 5
```

### 5.2 Tranh chấp: `value` là giá hay item id?

| Nguồn | Nội dung |
|---|---|
| `[B]` `ShopHandler.cs:33-34` | `packetCreator2.addByte((byte)num6); packetCreator2.add32(num7);` — `num7` là **đơn giá** (`PacketReader.read32`) |
| `[C]` `FUN_00768638` | ghi `local_14` vào `slotObj+0x20`; `FUN_00768638:21-22` set `slotObj+0x1C = 1` và `slotObj+0x20 = itemid` |
| `[C]` tên hàm | `FUN_00768638` được mô tả là ghi vào slot object dùng chung với `17 05` |

⇒ **Xung đột.** Bear gửi **giá**; client ghi giá trị vào field tên `+0x20` mà docs gọi là `itemId`.

**Phân giải khả dĩ:** `slotObj+0x20` thực chất là **giá bán** (client đã biết item id từ
`player+0x9E0[slot]` vì `17 1E` **tái sử dụng** mảng hòm đồ, không phải gửi item mới). Nghĩa là
Rust **đang đúng** khi gửi `le32(price)`.

⚠️ **Xác minh thêm:** `asm_7716dc_venderitem_create.asm.txt` cho thấy `TVenderItem.Create` gọi
`TSe_itemObject` ctor rồi `self+0x4C = -1`. Cần đọc `FUN_00768638` body để chốt tên field.

**Hành động:** giữ `le32(price)`, đánh dấu `economy` gap G17 là **false positive** có khả năng
cao, cập nhật lại tài liệu sau khi đọc body `FUN_00768638`.

### 5.3 `17 20` — Stall close

`[R]` `shops.rs:500`:
```rust
out.send(format!("F44406001720{}{}", "F4440600" /*…*/, encoder::le32(id)));
// → F4440600 1720 <le32 id>   len=6 nhưng body chỉ có 4 byte sau sub
```

Client `FUN_00731388` đọc uid tại `payload[3..6]` ⇒ cần 1 byte padding giữa sub và uid.
**Chưa xác minh byte đó là gì** (có thể `stall_id:1` giống `17 1E`). Giữ backlog, chờ capture.

---

## 6. `05 03` — Stats snapshot: cân nhắc tail

### 6.1 Layout hiện tại

`[R]` `spawn.rs:180-186`
```rust
body.push_str("F401F401F401F401F401");   // 5 × u16 = 500
body.push_str(&"00".repeat(90));         // 90 byte zero
body.push_str(skills_hex);               // (le16 id + u8 lv) × N
```

### 6.2 Layout Bear

`[B-verified]` `TSCharacter.cs:5072-5098`
```csharp
packetCreator.add16(500);  // ×5
packetCreator.addZero(43);
foreach (ushort key in skill.Keys) { packetCreator.add16(key); packetCreator.addByte(skill[key]); }
packetCreator.add16(hp3);  packetCreator.add16(sp3);  packetCreator.add16(mag3);
packetCreator.add16(atk3); packetCreator.add16(def3); packetCreator.add16(agi3);
packetCreator.add16(hpx3); packetCreator.add16(spx3);
packetCreator.add16(hp4);  packetCreator.add16(sp4);  packetCreator.add16(mag4);
packetCreator.add16(atk4); packetCreator.add16(def4); packetCreator.add16(agi4);
packetCreator.add16(hpx4); packetCreator.add16(spx4);
```

Block `_3` và `_4`: 8 × u16 mỗi block. Bear không có field nào tên ý nghĩa rõ — có thể là
thống kê `_3`/`_4` của item system (đồng bộ với `_1`/`_2` mà `InventoryItem` đang giữ).

### 6.3 Quyết định

| Bước | Hành động |
|---|---|
| 1 | **GIỮ NGUYÊN** `90` byte zero + không có block `_3`/`_4` cho tới khi có capture |
| 2 | Capture `05 03` từ `aLogin.exe` (server Bear nếu có, hoặc so sánh 2 server) |
| 3 | Đọc client parser `05` S→C (`case_005` → `FUN_0078C7DB`) để biết đọc fixed hay tới-cuối |
| 4 | Nếu fixed-offset → sửa thành `43` + thêm 32 byte `_3`/`_4` (giá trị 0 trước) |
| 5 | Nếu tới-cuối → giữ nguyên, ghi chú "90 zero là padding vô hại" |

⚠️ **Không tự sửa theo Bear** khi chưa biết client parse thế nào. `post_login_stats_inventory_flow.md`
§5.2 gap #4 đã cảnh báo đúng điều này.

---

## 7. `08 01` — Bảng prop code chuẩn

### 7.1 Wire format

`[R]` `stats.rs:8-22` — đã khớp Bear:
```rust
pub fn build_stat_update(stat_type: u8, val: i32) -> String {
    let (sign, abs_val) = if val >= 0 { ("01", val as u32) } else { ("02", (-val) as u32) };
    let body = format!("{:02X}{}{}{}", stat_type, sign, encoder::le32(abs_val), "00000000");
    crate::protocol::frame("0801", &body)
}
```
`[B-verified]` `TSCharacter.refresh():5413-5445` — `PacketCreator(8) + addByte(1) + addByte(code)
+ addByte(sign) + add32(val) + add32(0)`. Khớp byte-for-byte. ✅

### 7.2 Bảng mapping

Xem chi tiết ở `02-gap-analysis.md` §6. Tóm tắt những chỗ cần sửa:

| Tình huống | Rust hiện tại | Đúng |
|---|---|---|
| equip burst | `0x19`←`hp_max` | `0x19`←`hp`, `0xD7`←`hp_max` |
| equip burst | `0x1A`←`sp_max` | `0x1A`←`sp`, `0xD8`←`sp_max` |
| equip burst | `0xCF`←`hpx2` | `0xCF`←`hp2`; `0xDA`←`hpx2` |
| equip burst | `0xD0`←`spx2` | `0xD0`←`sp2`; `0xDB`←`spx2` |
| stats.rs | `0x1B`←`int1` | ✅ đúng (MAG) |
| stats.rs | `0x26`←`point` | ✅ đúng (STATS_POINT) |
| `talk.rs:508` | `0x25`←`skill_point` | ✅ đúng |

⚠️ **Session chưa có field `hp2`/`sp2`** (bonus HP/SP phẳng). Bear có 8 accumulator:
`hp2 sp2 atk2 def2 mag2 agi2 hpx2 spx2`. Rust chỉ có 6: `int2 atk2 def2 hpx2 spx2 agi2`.

⇒ Cần thêm 2 field `hp2`/`sp2` vào `Session` + `GearBonuses` + `character_sheet`, và cột DB.
Đây là thay đổi lan toả — xem `04-db-persistence.md` §4.

### 7.3 Cải thiện `GearBonuses`

`[R]` `character_sheet.rs:30-65` hiện cộng `int1 + int2` cho **mọi** accumulator:
```rust
b.int2 += item.int1.max(0) as u32 + item.int2.max(0) as u32;
b.atk2 += item.atk1.max(0) as u32 + item.atk2.max(0) as u32;
// ...
```

`[B-verified]` Bear chỉ cộng theo **prop code** cụ thể (`TSCharacter.cs:1682-1718`):
```csharp
public void addEquipBonus(ushort prop, int prop_val, int type)
{
    int num = (type == 0) ? prop_val : -prop_val;
    switch (prop)
    {
        case 207: hp2  += num; break;   // 207 = HP2
        case 208: sp2  += num; break;
        case 210: atk2 += num; break;
        case 211: def2 += num; break;
        case 212: mag2 += num; break;
        case 214: agi2 += num; break;
        case 218: hpx2 += num; break;
        case 219: spx2 += num; break;
        case 209: case 213: case 215: case 216: case 217: break;  // bị bỏ qua
    }
}
```

⇒ Rust cần **bảng `Item.dat` prop → accumulator**, không phải "cộng vào tất cả".
Kiểm tra `Item.dat` loader (`data/loaders/item.rs`, `data/tables.rs:39-67`) xem có field
`prop1`/`prop2` chưa. Đây là việc Phase 2 scope creep — có thể tách sang Phase 4 nếu cần.

---

## 8. `28 01` — Hotkey: đã đúng, chỉ cần test

### 8.1 Layout

`[R]` `session.rs:479-493`
```rust
for slot in 1..=10 {
    let skill_id = self.hotkeys.get(slot as usize).copied().unwrap_or(0);
    if skill_id > 0 { entries.push_str(&format!("02{}{:02X}", encoder::le16(skill_id), slot)); }
}
```

`[C-verified]` `client_pseudo_c/005ab3f8_FUN_005ab3f8.asm.txt`
```asm
005ab401: SUB EAX,0x1
005ab408: ADD EAX,0x3
005ab40b: SAR EAX,0x2          ; N = (Len(RP)-1) >> 2   ← stride 4
...
005ab45b: MOV AL,[EBP-0xa]    ; R+0 = type
005ab477: CALL 0x00404290     ; LStrCopy 2 byte
005ab486: CALL 0x0077eb9c     ; Word LE → id
005ab4a8: MOV AL,[EBP-0xb]    ; R+3 = slot
005ab4bb: INC byte [EBP-0xd]  ; cursor += 4
```

`type == 1` → `FUN_005ABCF0` (gán **item** vào quick slot)
`type == 2` → `FUN_005AB678` (gán **skill** vào quick slot)

⇒ ✅ Rust khớp: `type=2`, `le16(skill_id)`, `slot`.

### 8.2 Phát hiện bổ sung

Client hỗ trợ `type == 1` = **item quickslot** (scan `player+0x9E0[1..0x19]` tìm `itemCode`).
Rust `handle_hotkey` (`stats.rs:142-165`) chỉ xử lý `kind 0` (xoá) và `kind 2` (gán skill).

⇒ Bổ sung `kind 1` trong backlog (Phase 4).

---

## 9. `1A 04` — Gold: đã đúng

`[R]` `spawn.rs:561-564`
```rust
body.push_str(&encoder::le32(s.gold));
body.push_str("00000000");
```

`[B-verified]` `TSCharacter.sendGold():5299-5305`
```csharp
PacketCreator packetCreator = new PacketCreator(26, 4);
packetCreator.add32(gold);
packetCreator.add32(gold_bank);
```

`[C-verified]` `client_pseudo_c/0072bcf8_FUN_0072bcf8.c:51-61`
```c
_LStrCopy(param_2, 2, 4, &local_10);
*(undefined4 *)(*(int *)gvar_007DA7BC + 0x12f8) = FUN_0077ef7c(...);   // field A
_LStrCopy(local_c, 6, 4, &local_14);
*(undefined4 *)(*(int *)gvar_007DA7BC + 0x1300) = FUN_0077ef7c(...);   // field B
```

⚠️ **Phát hiện mới:** field thứ 2 **không phải** "unused". `FUN_00745ACC` (`1A 05`) và
`FUN_00745B14` (`1A 06`) đọc/ghi `player+0x1300` như một **currency pool riêng**, trần
`0x77359400` (2 000 000 000). Rust luôn gửi 0 ⇒ mất một loại tiền/thanh điểm thứ hai.

⇒ Đưa vào backlog Phase 4 (ngoài phạm vi hiện tại nhưng ảnh hưởng `1A 04`).

---

## 10. Tổng hợp thay đổi wire của Phase 2

| # | Thay đổi | File | Rủi ro |
|:-:|---|---|:-:|
| W-1 | `dump_trangbi` → 10B, sort theo slot | `session.rs:429-432` | 🟡 trung bình — có thể `doben` cần wire |
| W-2 | `1E 08` clear trước `1E 01` | `spawn.rs:551-556` | 🟢 thấp — additive frame |
| W-3 | `17 66` sau feature flag | `spawn.rs:555`, `config.rs` | 🟢 thấp — chỉ bỏ 1 frame |
| W-4 | Tách `dump_table` → `encode_12` / `encode_10` | `session.rs:449-468` | 🟢 thấp — refactor |
| W-5 | Bảng `prop_code` + sửa burst | `stats.rs`, `inventory.rs:314-321,384-391` | 🔴 **cao** — cần capture |
| W-6 | Thêm `hp2`/`sp2` vào Session | `session.rs`, `character_sheet.rs` | 🟡 trung bình |
| W-7 | `05 03` tail `43` + block `_3`/`_4` | `spawn.rs:182-184` | 🔴 **cao** — **HOÃN**, chờ capture |
| W-8 | `GearBonuses` theo prop code | `character_sheet.rs:30-65` | 🟡 trung bình |

**Đề xuất chia 2 lô:**
- **Lô 2A (làm ngay):** W-1, W-2, W-3, W-4 — không cần capture, rủi ro thấp.
- **Lô 2B (chờ capture):** W-5, W-7, W-8 — cần đọc client parser `08`/`05`.