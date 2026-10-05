# 02 — Phân Tích Khoảng Trống: Hiện Trạng vs Chuẩn

> Nguồn: đối chiếu `src/` (Rust) với [`01-container-mapping.md`](./01-container-mapping.md).
> Mỗi mục: **hiện trạng** → **chuẩn** → **khoảng cách** → **mức độ**.

---

## 1. Bảng tổng hợp

| # | Hạng mục | Mức | Rust hiện tại | Chuẩn | Phase |
|:-:|---|:-:|---|---|:-:|
| G-01 | `17 0B` stride | 🔴 P0 | 12B + byte slot | **10B, không slot** | 2 |
| G-02 | `17 66` layout | 🔴 P0 | giả định 12B | **không rõ** | 2 |
| G-03 | `load_items` cột DB | 🔴 P0 | 4/22 cột | 22 cột | 3 |
| G-04 | `save` cột DB | 🔴 P0 | 6/22 cột | 22 cột | 3 |
| G-05 | `recompute_stats` khi load | 🟠 P1 | không gọi | phải gọi | 3 |
| G-06 | `08 01` prop codes | 🟠 P1 | lệch ngữ nghĩa | theo `TSConstants` | 2 |
| G-07 | `05 03` tail | 🟠 P1 | `F401`×5 + 90 zero | `0x01F4`×5 + 43 zero + 32B | 2 |
| G-08 | `1E 01` clear state | 🟠 P1 | không | `1E 08` | 2 |
| G-09 | Sức chứa theo container | 🟠 P1 | `HOMDO_SLOTS=25` mọi nơi | 25/50/6 theo loại | 1 |
| G-10 | Move/stack không persist | 🟠 P1 | không ghi DB | upsert | 3 |
| G-11 | `17 1E` stall payload | 🟠 P1 | `le32(price)` | cần xác minh | 2 |
| G-12 | `17 20` stall uid lệch 1B | 🟠 P1 | thiếu padding | cần xác minh | 2 |
| G-13 | `tuideo` không có handler | 🟡 P2 | `1706` lặp, không `172F` | `17 2F`/`17 31` | 4 |
| G-14 | `pet_equip` không gắn item | 🟡 P2 | chỉ trừ homdo | ghi vào `trangbi` pet | 4 |
| G-15 | Nhân bản mapping `storagetype` | 🟡 P2 | 6 nơi | 1 nguồn | 1 |
| G-16 | `ThingData` 35B không dùng | 🟡 P2 | code chết | bỏ hoặc dùng | 3 |
| G-17 | Thiếu container `1E 09`/`1E 0C` | 🟢 P3 | không có | có thể thêm | 6 |
| G-18 | `tiengtam`/`god`/`color` không persist | 🟢 P3 | RAM-only | thêm cột | 4 |
| G-19 | `add_storage_item` vs `free_slot` | 🟢 P3 | 50 vs 25 | thống nhất | 1 |
| G-20 | `17 25` move túi đeo là stub | 🟢 P3 | chỉ ack | thực sự move | 4 |

---

## 2. G-01 — `17 0B` sai stride (P0)

### Hiện trạng

`[R]` `src/server/session.rs:429-432`
```rust
/// Dump Trangbi equipment frame (`F444` + len + `170B` + entries).
pub fn dump_trangbi(&self) -> String {
    self.dump_table(&self.trangbi, "170B")
}
```

`dump_table` (`:449-468`) luôn ghi **12 byte** có byte slot đầu:
```rust
fn dump_table(&self, items: &[InventoryItem], code: &str) -> String {
    let mut entries = String::new();
    for item in items {
        if item.id > 0 {
            entries.push_str(&format!(
                "{:02X}{}{:02X}{:02X}{:02X}{:02X}{:02X}{}",
                item.slot,              // ← byte slot: KHÔNG tồn tại trong 17 0B
                encoder::le16(item.id),
                item.count, item.doben, item.long_val,
                (item.giatri_long as u16 + 100) as u8,
                item.khang,
                encoder::le32(item.texp)
            ));
        }
    }
    crate::protocol::frame(code, &entries)
}
```

### Chuẩn

`[C-verified]` `client_pseudo_c/0077cc2c_FUN_0077cc2c.asm.txt`
```asm
0077cc68: MOV ECX,0xa
0077cc6b: CDQ
0077cc6e: IDIV ECX              ; n = (Len(RP)-1) / 10   ← stride 10
...
0077cc9e: ADD EAX,0x2           ; base = k*10 + 2
0077ccba: CALL 0x0077eb9c       ; Word LE tại base+2  ← itemCode ngay đầu record
```

`[B-verified]` `TS_Server_Bear/TS_Server/Client/TSCharacter.cs:5281-5297`
```csharp
PacketCreator packetCreator = new PacketCreator(23, 11);   // 23=0x17, 11=0x0B
for (int i = 0; i < 6; i++)
{
    if (equipment[i] != null)
    {
        packetCreator.add16(equipment[i].Itemid);   // không addByte slot!
        packetCreator.addByte(equipment[i].quantity);
        packetCreator.addByte(equipment[i]?.other_type ?? 0);
        packetCreator.addByte((byte)(100 + (byte)(equipment[i]?.other_val ?? 0)));
        packetCreator.addByte(equipment[i]?.anti ?? 0);
        packetCreator.add32(equipment[i]?.exp ?? 0);
    }
}
```
Tổng: 2+1+1+1+1+4 = **10 byte**. Ba nguồn độc lập cùng kết luận.

### Khoảng cách

| | Rust | Chuẩn |
|---|---|---|
| Stride | 12 | **10** |
| Byte đầu | `slot:u8` | `itemCode` byte thấp |
| Số record | `(Len-2)/12` | `(Len-2)/10` |

**Ảnh hưởng:** client đọc record thứ 2 bắt đầu tại offset 12 của Rust, nhưng client tính offset 10
→ mọi equipment từ record thứ 2 trở đi bị lệch. Với 6 ô trang bị đầy → 5/6 record sai.

### Cách sửa

Thêm serializer riêng, **không** tái sử dụng `dump_table`:
```rust
/// Frame `170B` — 10 byte/record, KHÔNG byte slot (client `IDIV 0x0A`).
pub fn dump_trangbi(&self) -> String {
    let mut entries = String::new();
    for item in self.trangbi.iter().filter(|i| i.id > 0) {
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

⚠️ **Lưu ý thứ tự:** bản sửa bỏ `doben`. Bear format 10B không có byte `doben` — byte thứ 2 là
`quantity`, byte 3 là `other_type` (= Rust `long_val`). Suy ra mapping:

| offset 10B | Ý nghĩa (Bear) | Field Rust tương đương |
|:-:|---|---|
| 0..1 | `Itemid` | `id` |
| 2 | `quantity` | `count` |
| 3 | `other_type` | `long_val` |
| 4 | `100 + other_val` | `giatri_long + 100` |
| 5 | `anti` | `khang` |
| 6..9 | `exp` | `texp` |

→ `doben` **không có chỗ** trong format 10B. Nó chỉ tồn tại ở format 12B. Phải quyết định: giữ
`doben` trong DB nhưng không gửi ở `17 0B`, hay dùng byte nào đó. Xem `03-wire-codecs.md` §2.3.

---

## 3. G-02 — `17 66` không có bằng chứng (P0)

### Hiện trạng

`[R]` `session.rs:444-447`
```rust
/// Dump LuuLang storage frame (`F444` + len + `1766` + entries).
pub fn dump_luulang(&self) -> String {
    self.dump_table(&self.luulang, "1766")
}
```
Được đẩy **mỗi lần login** tại `spawn.rs:555`.

### Bằng chứng (không)

| Nguồn | Kết quả |
|---|---|
| `[C]` call-site | `0x00790D18: CALL 0x0076A320` — **có**, nhưng body `FUN_0076A320` không có trong dump |
| `[C]` layout | ❌ không xác định được |
| `[B]` Bear | ❌ Bear **không gửi** `17 66` ở bất kỳ đâu (đã grep toàn bộ 65 call-site `PacketCreator(23,n)`) |
| `[D]` tài liệu | `post_login_stats_inventory_flow.md:213-214` ghi rõ *"layout client chưa được tách riêng"* |

### Rủi ro

Nếu `17 66` thực sự là container 51/26 ô với record 12B → layout Rust đúng. Nếu là format khác
(10B như `17 0B`, hoặc record không có slot) → **mọi record lệch**, và client có thể hiển thị
vật phẩm ở ô sai (không chỉ "không hiển thị").

### Khuyến nghị

| Bước | Hành động |
|---|---|
| 1 | **Tạm dừng gửi `17 66`** trong Logined1 (feature flag) — gửi layout sai còn nguy hiểm hơn không gửi |
| 2 | Capture `aLogin.exe` với `luulang` có dữ liệu → xác định layout thật |
| 3 | Nếu xác nhận 12B → bật lại, đóng gap |
| 4 | Nếu không xác nhận được → đổi mapping: dùng `1E 09` (51 ô, 12B, **đã có body**) cho chức năng lưu lãng, bỏ hẳn `17 66` |

Xem `05-phased-roadmap.md` §3.2 cho quyết định nhánh.

---

## 4. G-03/G-04 — Persist mất 16-18 cột (P0)

### Hiện trạng — đọc

`[R]` `src/db/modern/sqlite/session.rs:176-199`
```rust
async fn load_items(&self, character_id: i64, storage_type: u8)
    -> Result<Vec<InventoryItem>, sqlx::Error>
{
    let rows = sqlx::query(
        "SELECT slot, itemid AS item_id, quantity, damage      // ← chỉ 4 cột
         FROM inventories WHERE playerid = ? AND storagetype = ? AND itemid > 0 ORDER BY slot",
    )
    .bind(character_id).bind(storage_type)
    .fetch_all(&self.pool.read).await?;
    Ok(rows.into_iter().map(|r| InventoryItem {
        slot: clamp_u8(r.get("slot")),
        id: clamp_u16(r.get("item_id")),
        count: clamp_u8(r.get("quantity")),
        doben: clamp_u8(r.get("damage")),
        ..Default::default()                                    // ← 24 field = 0
    }).collect())
}
```

### Hiện trạng — ghi

`[R]` `sqlite/session.rs:299-314`
```rust
sqlx::query("DELETE FROM inventories WHERE playerid = ?")
    .bind(character_id).execute(&mut *tx).await?;

for (storage, items) in [
    (1u8, &session.homdo), (2, &session.tientrang), (4, &session.tuideo),
    (8, &session.trangbi), (16, &session.luulang),
] {
    for item in items.iter().filter(|i| i.id > 0) {
        sqlx::query(
            "INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage)
             VALUES (?, ?, ?, ?, ?, ?)"                          // ← chỉ 6 cột
        )
        .bind(character_id).bind(storage).bind(i64::from(item.slot))
        .bind(i64::from(item.id)).bind(i64::from(item.count))
        .bind(i64::from(item.doben)).execute(&mut *tx).await?;
    }
}
```

⚠️ **`DELETE` toàn bộ bảng rồi re-insert 6 cột** ⇒ mỗi lần auto-save **xóa vĩnh viễn** 16 cột còn
lại của mọi item. `growlevel`, `enhancelevel`, `islocked`, `affix1..3`, `stonelevel`,
`specialkind`, `reinforced`, `stylelevel`, `deletetime`, `damageditemid`… tất cả về mặc định 0.

`[R]` Đường legacy `persist::upsert_inventory_tx` (`db/persist.rs:211-225`) cũng chỉ ghi 5 cột
và có dòng `let _ = item;` (`:220`) — dấu hiệu refactor dở dang.

### Khoảng cách

| Cột `0001_init.sql` | Đọc | Ghi | Ảnh hưởng |
|---|:-:|:-:|---|
| `slot`, `itemid`, `quantity`, `damage` | ✅ | ✅ | — |
| `element`, `elementvalue`, `proofkind` | ❌ | ❌ | mất hệ nguyên tố sau relog |
| `growlevel`, `growexp` | ❌ | ❌ | mất cấp độ trưởng thành |
| `specialkind`, `stoneattr`, `stonelevel` | ❌ | ❌ | mất ngọc khảm |
| `enhancelevel`, `reinforced` | ❌ | ❌ | mất cường hóa/gia cố |
| `deletetime`, `damageditemid` | ❌ | ❌ | mất hạn dùng |
| `islocked` | ❌ | ❌ | **mất trạng thái khóa an toàn** |
| `affix1..3`, `stylelevel` | ❌ | ❌ | mất dòng phụ |

### Hệ quả kép

1. **Wire sai**: `element` → `b3` (offset 6 của record 12B) = 0 ⇒ client hiển thị sai hệ nguyên tố.
2. **Logic sai**: `equip` gate dùng `item.loai` (`inventory.rs:292`) — nhưng `loai` là **template
   field**, không phải instance field, nên nó phải được hydrate từ `Item.dat`, không lưu DB. Xem G-05.

---

## 5. G-05 — `recompute_stats` không chạy khi load (P1)

### Hiện trạng

`[R]` `Session::recompute_stats()` (`session.rs:397-422`) là nơi duy nhất ghi 6 trường `*2`:
```rust
self.int2 = sheet.gear.int2;
self.atk2 = sheet.gear.atk2;
...
self.hpx2 = sheet.gear.hpx2;
self.spx2 = sheet.gear.spx2;
self.agi2 = sheet.gear.agi2;
self.hp_max = sheet.hp_max;
self.sp_max = sheet.sp_max;
```

**9 call-site** trong `src/`:

| File:line | Ngữ cảnh |
|---|---|
| `handlers/stats.rs:104` | phân bổ `hpx` |
| `handlers/stats.rs:119` | phân bổ `spx` |
| `handlers/inventory.rs:311` | equip |
| `handlers/inventory.rs:381` | unequip |
| `handlers/inventory.rs:483` | reborn |
| `handlers/quest.rs:420` | battle win |
| `player_state.rs:66` | facade |
| `gm.rs:230` | GM `/additem` |
| `gm.rs:300` | GM `/level` |

**Vắng mặt:** `login.rs`, `sqlite/session.rs::load`, `character.rs::apply_to_session`.

### Khoảng cách

`sqlite/session.rs:56-61` đọc `int1..agi` từ DB, nhưng:
- **không có cột nào** cho `int2..agi2` (`0001_init.sql` chỉ có `baseint..baseagi`)
- không gọi `recompute_stats()`

→ Sau khi login từ DB: `int2 = atk2 = def2 = hpx2 = spx2 = agi2 = 0` ⇒ `05 03` gửi 6 số 0
(`spawn.rs:519-524`), **trong khi `hp_max`/`sp_max` lại đúng** vì đọc từ `maxhp`/`maxsp`.

**Nghịch lý:** HUD hiển thị HP tối đa có giá trị, nhưng không cộng được bonus trang bị để giải thích.

Ngoài ra: `battle/construction.rs:109-112` đọc `session.int1 + session.int2` ⇒ **sai số liệu
trong trận** cho nhân vật vừa login.

### Cách sửa

Gọi `session.recompute_stats()` ngay sau khi `load_items` xong, trong
`SqliteSessionRepository::load` (`sqlite/session.rs` sau dòng 76). Nhưng phải hydrate template
trước — xem `04-db-persistence.md` §3.

---

## 6. G-06 — `08 01` prop code lệch ngữ nghĩa (P1)

### Chuẩn

`[B-verified]` `TS_Server_Bear/TS_Server/TSConstants.cs:5-67`

| Code | Const | Ý nghĩa |
|:-:|---|---|
| `0x19` (25) | `_HP` | HP **hiện tại** |
| `0x1A` (26) | `_SP` | SP **hiện tại** |
| `0x1B` (27) | `_MAG` | INT |
| `0x1C` (28) | `_ATK` | ATK |
| `0x1D` (29) | `_DEF` | DEF |
| `0x1E` (30) | `_AGI` | AGI |
| `0x1F` (31) | `_HPX` | HPX |
| `0x20` (32) | `_SPX` | SPX |
| `0x26` (38) | `_STATS_POINT` | điểm tiềm năng |
| `0x25` (37) | `_SKILL_POINT` | điểm kỹ năng |
| `0xD7` (215) | `_FULLHPMAX` | **HP max** |
| `0xD8` (216) | `_FULLSPMAX` | **SP max** |
| `0xCF` (207) | `_HP2` | bonus HP phẳng |
| `0xD0` (208) | `_SP2` | bonus SP phẳng |
| `0xD2` (210) | `_ATK2` | bonus ATK |
| `0xD3` (211) | `_DEF2` | bonus DEF |
| `0xD4` (212) | `_MAG2` | bonus INT |
| `0xD6` (214) | `_AGI2` | bonus AGI |
| `0xDA` (218) | `_HPX2` | **bonus HPX** |
| `0xDB` (219) | `_SPX2` | **bonus SPX** |

### Hiện trạng

`[R]` `handlers/inventory.rs:314-321` (burst sau equip) và `:384-391` (sau unequip):
```rust
out.send(build_stat_update(0x19, conn.session.hp_max as i32));   // ❌ 0x19 = HP hiện tại
out.send(build_stat_update(0x1A, conn.session.sp_max as i32));   // ❌ 0x1A = SP hiện tại
out.send(build_stat_update(0xCF, conn.session.hpx2 as i32));     // ❌ 0xCF = HP2 phẳng
out.send(build_stat_update(0xD0, conn.session.spx2 as i32));     // ❌ 0xD0 = SP2 phẳng
out.send(build_stat_update(0xD2, conn.session.atk2 as i32));     // ✅
out.send(build_stat_update(0xD3, conn.session.def2 as i32));     // ✅
out.send(build_stat_update(0xD4, conn.session.int2 as i32));      // ✅
out.send(build_stat_update(0xD6, conn.session.agi2 as i32));     // ✅
```

### Khoảng cách — 3 lỗi

| Lỗi | Rust gửi | Đúng | Hậu quả |
|---|---|---|---|
| 1 | `hp_max` vào `0x19` | `hp` hiện tại vào `0x19`, `hp_max` vào `0xD7` | HUD hiển thị sai HP |
| 2 | `sp_max` vào `0x1A` | `sp` hiện tại vào `0x1A`, `sp_max` vào `0xD8` | HUD hiển thị sai SP |
| 3 | `hpx2`/`spx2` vào `0xCF`/`0xD0` | `hp2`/`sp2` vào đó; `hpx2`/`spx2` vào `0xDA`/`0xDB` | bonus sai chỗ, `0xDA`/`0xDB` chưa từng được gửi |

### 6.3 Phát hiện bổ sung: Logined1 Rust gửi **0** frame `08 01`, Bear gửi **18**

`[R]` `grep "0801\|build_stat_update" src/server/spawn.rs` → **0 kết quả**.
Toàn bộ burst `08 01` trong Rust chỉ nằm ở handler equip/unequip/talk/chat, **không** nằm trong
chuỗi login.

`[B-verified]` Bear `loginChar()` (`TSCharacter.cs:1100-1111`) gửi:

| Vị trí | Hàm | Số frame | Nội dung |
|---|---|:-:|---|
| `sendInfo()` `:1100` | `refresh(FullHpMax, 215)` + `refresh(FullSpMax, 216)` | 2 | ngay sau `05 03` |
| `refreshChr()` `:1111` | `refresh()` × 8 | 8 | `31 hpx, 32 spx, 28 atk, 29 def, 27 mag, 30 agi, 25 hp, 26 sp` |
| `refreshChr()` → `refreshBonus()` | `refresh()` × 8 | 8 | `212 mag2, 210 atk2, 211 def2, 207 hp2, 208 sp2, 214 agi2, 218 hpx2, 219 spx2` |
| | | **18** | |

⇒ Bear bù lại `05 03` bằng 18 frame delta. Rust không có bước này.

⚠️ **Không tự thêm** 18 frame này khi chưa xác minh. `05 03` đã chứa 8 chỉ số gốc + 6 bonus;
thêm `08 01` có thể thừa. Nhưng nếu `05 03` tail sai (G-07) thì `08 01` là đường **dự phòng**
duy nhất để HUD có số đúng.

⇒ **Câu hỏi capture quyết định G-07:** HUD sau login lấy số từ `05 03` hay từ `08 01`?
Trả lời "từ `05 03`" ⇒ phải sửa tail. Trả lời "từ `08 01`" ⇒ thêm burst, giữ tail hiện tại.

### 6.4 Cách sửa

Định nghĩa bảng mapping một chỗ, dùng chung:
```rust
/// Prop code `08 01` → `TSConstants.cs` (Bear). Đây là nguồn chuẩn duy nhất.
pub mod prop_code {
    pub const HP: u8 = 0x19;
    pub const SP: u8 = 0x1A;
    pub const MAG: u8 = 0x1B;
    pub const ATK: u8 = 0x1C;
    pub const DEF: u8 = 0x1D;
    pub const AGI: u8 = 0x1E;
    pub const HPX: u8 = 0x1F;
    pub const SPX: u8 = 0x20;
    pub const SKILL_POINT: u8 = 0x25;
    pub const STATS_POINT: u8 = 0x26;
    pub const HONOR: u8 = 0x3E;
    pub const HP2: u8 = 0xCF;
    pub const SP2: u8 = 0xD0;
    pub const ATK2: u8 = 0xD2;
    pub const DEF2: u8 = 0xD3;
    pub const MAG2: u8 = 0xD4;
    pub const AGI2: u8 = 0xD6;
    pub const FULL_HP_MAX: u8 = 0xD7;
    pub const FULL_SP_MAX: u8 = 0xD8;
    pub const HPX2: u8 = 0xDA;
    pub const SPX2: u8 = 0xDB;
}
```
Rồi burst equip gửi **10 frame** thay vì 8:
`0x19`(hp) `0x1A`(sp) `0xD7`(hp_max) `0xD8`(sp_max) `0xCF`(hp2) `0xD0`(sp2) `0xD2`(atk2)
`0xD3`(def2) `0xD4`(mag2) `0xD6`(agi2) `0xDA`(hpx2) `0xDB`(spx2) — thực ra 12 frame.

⚠️ **Rủi ro:** bảng trên lấy từ Bear, **chưa** xác minh bằng client parser `08 01` S→C
(`case 009` → `FUN_0078D125` chưa có body). Cần capture trước khi ship. Xem
`06-tests-and-capture.md` §3.

---

## 7. G-07 — `05 03` tail lệch (P1)

### Hiện trạng

`[R]` `src/server/spawn.rs:182-184`
```rust
// Literal `F401`×5 + 90 zero bytes + skill list.
body.push_str("F401F401F401F401F401");
body.push_str(&"00".repeat(90));
body.push_str(skills_hex);
```

### Chuẩn

`[B-verified]` `TSCharacter.cs:5072-5098`
```csharp
packetCreator.add16(500);          // 0x01F4  ← KHÔNG phải 0x01F4... đúng, nhưng Rust ghi F401 = 0x01F4 LE
packetCreator.add16(500);          // ×5
packetCreator.add16(500);
packetCreator.add16(500);
packetCreator.add16(500);
packetCreator.addZero(43);         // ← 43 byte, KHÔNG phải 90
foreach (ushort key in skill.Keys)
{
    packetCreator.add16(key);
    packetCreator.addByte(skill[key]);
}
packetCreator.add16(hp3);          // ← block _3: 8 × u16
packetCreator.add16(sp3); ... (8 field)
packetCreator.add16(hp4);          // ← block _4: 8 × u16
packetCreator.add16(sp4); ... (8 field)
```

### Khoảng cách

| Phần | Rust | Bear | Ghi chú |
|---|---|---|---|
| Header 5×u16 | `F401`×5 = 500×5 | `500`×5 | ✅ **khớp** (`F401` là LE của 0x01F4) |
| Zero padding | **90 byte** | **43 byte** | ❌ lệch 47 byte |
| Skill list | ✅ `le16(id) + u8(lv)` | ✅ tương đương | ✅ |
| Block `_3` (16B) | ❌ thiếu | ✅ có | ❌ |
| Block `_4` (16B) | ❌ thiếu | ✅ có | ❌ |

⚠️ Đính chính: `post_login_stats_inventory_flow.md:270` ghi Bear dùng "`500`×5" — đúng, và Rust
ghi `F401` cũng là 500 LE. **Vấn đề chỉ nằm ở 43 vs 90 zero và 32 byte block thiếu.**

**Nếu client đọc fixed-offset**: sau skill list, Rust gửi 90 zero → client đọc block `_3` từ
offset sai ⇒ 8 số HP3/SP3 sai. Nếu client đọc "tới hết buffer" → vô hại.

⚠️ **Cần capture** để biết client parse thế nào. Xem `06-tests-and-capture.md` §2.

---

## 8. G-08 — `1E 01` không có clear (P1)

Chi tiết ở [`01-container-mapping.md`](./01-container-mapping.md) §7.

**Tóm tắt:** `FUN_0076BFC8` (`17 05`) có vòng clear `1..=25` trước khi ghi;
`FUN_007707A0` (`1E 01`) **không có**. Ô đã gửi trước đó không tự xoá.

**Cách sửa:** gửi frame `1E 08` (mass-clear 50 ô của `player+0xA48`) **trước** `1E 01`:
```rust
// 10. Step 10: Inventory dumps
frames.push(s.dump_homdo());       // 17 05 — có clear pass nên không cần
frames.push(crate::protocol::frame("1E08", ""));  // clear +0xA48 (50 ô)
frames.push(s.dump_tientrang());   // 1E 01
```

⚠️ `17 2F` / `17 66` chưa biết có clear không → nếu không, cũng cần xử lý tương tự (chưa biết frame
nào tương đương).

---

## 9. G-09 — Sức chứa dùng chung `HOMDO_SLOTS` (P1)

### Hiện trạng

`[R]` `src/server/inventory.rs:9,29-32`
```rust
pub const HOMDO_SLOTS: u8 = 25;

/// Find the first free slot in `1..=HOMDO_SLOTS`, or `None` when full.
pub fn free_slot(bag: &[InventoryItem]) -> Option<u8> {
    (1..=HOMDO_SLOTS).find(|slot| !bag.iter().any(|i| i.slot == *slot && i.id > 0))
}
```

`[R]` `src/server/handlers/trade_storage.rs:173-181`
```rust
fn add_storage_item(storage: &mut Vec<InventoryItem>, mut item: InventoryItem) -> bool {
    let Some(slot) = (1..=50).find(|slot| !storage.iter().any(|i| i.slot == *slot && i.id > 0))
        else { return false };
    ...
}
```

Nhưng `handle_storage_transfer` gọi `inventory::free_slot` cho hướng storage→homdo
(`trade_storage.rs:187`) và `add_storage_item` cho hướng homdo→storage.

### Khoảng cách

| Container | `free_slot` | `add_storage_item` | Chuẩn |
|---|:-:|:-:|:-:|
| hòm đồ | 25 | — | 25 ✅ |
| tiền trang | **25** | **50** | 50 ❌ |
| lưu lang | **25** | **50** | 50 ❌ |
| trang bị | — | — | **6** ❌ |

### Cách sửa

Đặt sức chứa vào metadata container (Phase 1), dùng chung mọi nơi:
```rust
ContainerSpec::for_kind(ContainerKind::Bank).capacity  // 50
```

---

## 10. G-10 — Move/stack không persist (P1)

`[R]` `handlers/inventory.rs:218-273` `handle_move_stack`:
- Nhận `conn`, `payload`, `decoded`, `out`
- **Không nhận `pool`**
- **Không gọi `persist::*`**
- Ack = `out.send(hex_of(decoded))` — echo nguyên frame client

→ Di chuyển item chỉ tồn tại trong RAM tới lần auto-save kế (180s, `auto_save.rs:23`).
Disconnect trước đó ⇒ mất.

**Cách sửa:** thêm `pool` vào signature, upsert **mọi** slot bị ảnh hưởng (kể cả swap 2 slot và
split stack).

⚠️ Lưu ý `auto_save::fingerprint` (`auto_save.rs:38-120`) chỉ mix `homdo` + `trangbi`
+ `quest_items` ⇒ thay đổi ở `tientrang`/`tuideo`/`luulang` **không** kích hoạt auto-save.

---

## 11. G-11/G-12 — Frame stall (P1)

| Vấn đề | File:line | Chi tiết |
|---|---|---|
| `17 1E` payload | `shops.rs:465-466` | gửi `(slot:1, le32(price))`; client `FUN_00730F4C` đọc `item_id` DWORD rồi `FUN_00768638(slotObj, id)` ghi vào `slotObj+0x20`. Bear `ShopHandler.cs:33-34` cũng ghi `price`. **Cần xác minh client coi field này là giá hay item id.** |
| `17 20` uid | `shops.rs:500` | `F44406001720{} + le32(id)` — thiếu 1 byte; client `FUN_00731388` đọc uid tại `payload[3..6]` |

Cả hai đã được ghi nhận trong `economy/07_rust_economy_audit.md` là G17/G18 — giữ nguyên trạng
thái chờ capture.

---

## 12. G-13..G-20 — Vấn đề mức thấp

### G-13 — `tuideo` không có đường đọc đúng (P2)

`[R]` `handlers/inventory.rs:612-622`
```rust
async fn handle_auxiliary_bag(conn: &mut Conn, _payload: &[u8], out: &mut HandleOutcome) {
    for item in &conn.session.tuideo {
        if item.id > 0 {
            out.send(item_added_frame(item));   // 1706 lặp lại — KHÔNG phải 172F
        }
    }
}
```

`17 36` là **query** túi đeo theo Bear `BagHandle case 36` (`BagHandle.cs:16-97`) — Bear reply
`23 50` (mở form) + `23 9` + `23 48` (record 12B). Rust chỉ gửi `1706` → sai frame.

Bear `case 37` (`BagHandle.cs:98-168`) = bag→inventory, reply `23 6` + `23 9`. Rust `1725` là stub.

### G-14 — `pet_equip` không gắn item vào pet (P2)

`[R]` `handlers/inventory.rs:554-580` `handle_pet_equip`: trừ `homdo` count rồi gửi `1711`,
**không** ghi item vào `trangbi` với slot composite `stt*10 + sub`.
`handle_pet_unequip` (`:582-599`) là ack thuần, không đổi state.

Pet equipment thực ra dùng `trangbi` slot `stt*10+1..6` (`pet_actions.rs:38-53`,
`spawn.rs:428-438`) — nhưng `handle_pet_equip` không ghi vào đó.

⚠️ `move_pet_slot:50` dùng `u16 as u8` → tràn cho `stt*10+sub > 255`.

### G-15 — Nhân bản mapping `storagetype` (P2)

6 nơi (chi tiết bảng ở [`01-container-mapping.md`](./01-container-mapping.md) §3.3).
Sửa ở Phase 1.

### G-16 — `ThingData` 35 byte là code chết (P2)

`[R]` `write_thing_data` (`protocol/writer.rs:127`) và `read_thing_data`
(`protocol/reader.rs:148`) — **0 call site** trong toàn repo. `SqliteInventoryRepository`
dùng nó nhưng repo đó **không được wire vào đường live** (chỉ `transactions.rs:91,109` +
tests).

### G-17 — Thiếu container `1E 09` / `1E 0C` (P3)

Đã mô tả ở [`01-container-mapping.md`](./01-container-mapping.md) §5. `1E 0C` = Thủy binh bank
đã có đặc tả (`economy/05_special_economy.md:498`). Chưa implement.

### G-18 — Field RAM-only không persist (P3)

`[R]` `sqlite/session.rs::load` không đọc: `tiengtam`, `god`, `hp_store`, `sp_store`, `color`,
`gocnhin`, `dongtac`, `savemap`, `horse_pet_id`. `save` (`session.rs:284-295`) không ghi.
`persist::character_column` (`persist.rs:23-49`) không có mapping cho chúng.

⇒ `color` nhân vật **mất sau mỗi lần restart server** (chỉ có ở RAM và trong frame create).

### G-19 — `add_storage_item` vs `free_slot` (P3)

Đã nêu ở §9. Lỗi `u16 as u8` cho `move_pet_slot` liên quan tới G-14.

### G-20 — `1725` là stub (P3)

`[R]` `handlers/inventory.rs:624-636` chỉ ack, không mutate state. Theo Bear `case 37` phải
thực sự chuyển item.

---

## 13. Bảng đối chiếu nhanh: Rust vs chuẩn

| Khía cạnh | Rust | Bear | Client | Đồng thuận? |
|---|---|---|---|:-:|
| `17 05` layout | 12B slot-led | `sendItems` 12B | `IDIV 0xC` | ✅ |
| `1E 01` layout | 12B slot-led | `sendItems(30,1)` 12B | `IDIV 0xC` | ✅ |
| `17 0B` layout | **12B slot-led** | **10B no slot** | **`IDIV 0xA`** | ❌ |
| `17 2F` layout | 12B slot-led | `sendItems(23,47)` 12B | chưa có body | 🟡 |
| `17 66` layout | 12B slot-led | **không tồn tại** | chưa có body | ❌ |
| Slot base | 1-based | 1-based (`i+1`) | guard `≤0x19`/`≤0x32` | ✅ |
| Stack cap | 50 | 50 | — | ✅ |
| `05 03` header | `F401`×5 | `500`×5 | — | ✅ |
| `05 03` padding | **90 zero** | **43 zero** | chưa xác minh | ❌ |
| `05 03` tail | **thiếu 32B** | có `_3`+`_4` | chưa xác minh | ❌ |
| `08 01` codes | lệch | chuẩn | chưa xác minh | ❌ |
| `28 01` layout | 4B entry | `u8 2 + u16 + u8` | stride 4 | ✅ |
| `1A 04` layout | `le32 + 00000000` | `add32(gold) + add32(bank)` | 2× DWORD LE | ✅ |
| `17 1E` stall | header + 5B | header + 5B | header + 5B | ✅ (trừ field price/id) |