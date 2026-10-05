# 04 — Thiết Kế DB Chuẩn Hoá

> Đặc tả cho Phase 3. Mục tiêu: **`inventories` là nguồn sự thật duy nhất cho instance data**;
> template lấy từ `Item.dat`; không mất field nào qua vòng `load → mutate → save`.
> Tuân thủ `AGENTS.md`: không `ALTER TABLE` trong `0001_init.sql`; test ở `tests/`, không
> `#[cfg(test)]` trong `src/`.

---

## 1. Schema hiện tại

`[R]` `migrations/0001_init.sql:96-123`

```sql
CREATE TABLE IF NOT EXISTS inventories (
    playerid      INTEGER NOT NULL,
    storagetype   INTEGER NOT NULL,   -- 1: HomDo, 2: TienTrang, 4: TuiDeo, 8: TrangBi, 16: LuuLang
    slot          INTEGER NOT NULL,   -- 1..25 hoặc 1..50 tùy loại túi
    itemid        INTEGER DEFAULT 0,
    quantity      INTEGER DEFAULT 0,
    damage        INTEGER DEFAULT 0,
    element       INTEGER DEFAULT 0,
    elementvalue  INTEGER DEFAULT 0,
    proofkind     INTEGER DEFAULT 0,
    growlevel     INTEGER DEFAULT 0,
    growexp       INTEGER DEFAULT 0,
    specialkind   INTEGER DEFAULT 0,
    stoneattr     INTEGER DEFAULT 0,
    stonelevel    INTEGER DEFAULT 0,
    enhancelevel  INTEGER DEFAULT 0,
    deletetime    REAL DEFAULT 0,
    damageditemid INTEGER DEFAULT 0,
    islocked      INTEGER DEFAULT 0,
    reinforced    INTEGER DEFAULT 0,
    affix1        INTEGER DEFAULT 0,
    affix2        INTEGER DEFAULT 0,
    affix3        INTEGER DEFAULT 0,
    stylelevel    INTEGER DEFAULT 0,
    PRIMARY KEY (playerid, storagetype, slot)
);
CREATE INDEX IF NOT EXISTS inventories_item ON inventories (playerid, itemid);
```

**Schema này ĐỦ.** Không cột nào thiếu. Vấn đề nằm ở chỗ **Rust chỉ đọc/ghi 4-6 cột**.

---

## 2. Nguyên tắc: tách instance data khỏi template data

`[R]` `InventoryItem` (`session.rs:9-43`) trộn hai loại field:

| Nhóm | Field | Nguồn | Lưu DB? |
|---|---|---|:-:|
| **Định danh** | `slot`, `id`, `count` | instance | ✅ |
| **Wire/instance** | `doben`, `long_val`, `giatri_long`, `khang`, `texp` | instance | ✅ |
| **Template** | `lv`, `int1..fai1`, `int2..fai2`, `loai`, `thuoctinh`, `giatri_thuoctinh` | `Item.dat` | ❌ **hydrate mỗi lần load** |

`[B-verified]` Bear làm đúng kiểu này: `TSEquipment` giữ `equipinfo` (pointer tới `ItemInfo`
từ `Item.dat`) và chỉ lưu instance fields (`duration`, `elem_type`, `elem_val`, `other_type`,
`other_val`, `anti`, `exp`, `uescout`) trong JSON.

`[R]` `InventoryItem::from_template` (`session.rs:45-75`) **đã** làm đúng việc hydrate từ
`tables::Item`. Nhưng `load_items` không gọi nó ⇒ template field = 0 sau relog.

### 2.1 Hệ quả trực tiếp

| Logic | Cần field | Sau relog | File:line |
|---|---|:-:|---|
| Equip gate `loai ∈ 1..=6` | `loai` | ❌ 0 → **không mặc được đồ** | `handlers/inventory.rs:292` |
| Equip gate `level >= lv` | `lv` | ❌ 0 → gate luôn pass | `handlers/inventory.rs:292` |
| Gear bonus | `int1..fai2` | ❌ 0 → **bonus = 0** | `character_sheet.rs:30-65` |
| Element bonus | `thuoctinh`, `giatri_thuoctinh` | ❌ 0 | `character_sheet.rs:52-58` |
| Eve predicate | `thuoctinh` | ❌ 0 | `eve/` |

⇒ **Đây là nguyên nhân gốc của G-05**, không phải thiếu `recompute_stats()`.

---

## 3. Phase 3 — Bước 1: hydrate template khi load

### 3.1 Thay đổi `load_items`

`[R]` Hiện tại `sqlite/session.rs:176-199`:
```rust
async fn load_items(&self, character_id: i64, storage_type: u8)
    -> Result<Vec<InventoryItem>, sqlx::Error>
{
    let rows = sqlx::query(
        "SELECT slot, itemid AS item_id, quantity, damage
         FROM inventories WHERE playerid = ? AND storagetype = ? AND itemid > 0 ORDER BY slot",
    )
    .bind(character_id).bind(storage_type)
    .fetch_all(&self.pool.read).await?;
    Ok(rows.into_iter().map(|r| InventoryItem {
        slot: clamp_u8(r.get("slot")),
        id: clamp_u16(r.get("item_id")),
        count: clamp_u8(r.get("quantity")),
        doben: clamp_u8(r.get("damage")),
        ..Default::default()
    }).collect())
}
```

**Sau:**
```rust
async fn load_items(&self, character_id: i64, storage: StorageType, data: &GameData)
    -> Result<Vec<InventoryItem>, sqlx::Error>
{
    let rows = sqlx::query(INVENTORY_SELECT)
        .bind(character_id)
        .bind(storage.value())
        .fetch_all(&self.pool.read)
        .await?;

    Ok(rows.iter().map(|r| {
        let id = clamp_u16(r.get("item_id"));
        let mut item = crate::server::inventory::from_template(data, id, 1);
        // Ghi đè bằng instance data đọc từ DB.
        item.slot           = clamp_u8(r.get("slot"));
        item.id             = id;
        item.count          = clamp_u8(r.get("quantity"));
        item.doben          = clamp_u8(r.get("damage"));
        item.long_val       = clamp_u8(r.get("element"));
        item.giatri_long    = clamp_u8(r.get("elementvalue"));
        item.khang          = clamp_u8(r.get("proofkind"));
        item.texp           = r.get::<i64, _>("growexp") as u32;
        item
    }).collect())
}
```

⚠️ `from_template(data, id, 1)` dùng `count=1` rồi ghi đè — kiểm tra `inventory.rs:18-27`:
hàm chỉ set `count` từ tham số, không có side-effect khác. An toàn.

⚠️ `from_template` signature hiện là `(data: &GameData, id: u16, count: u8)`. Cần xác minh
`GameData.items: HashMap<i64, Item>` có đủ field cần thiết (`data/tables.rs:39-67`).

### 3.2 Định nghĩa `INVENTORY_SELECT`

Đã có sẵn ở `sqlite/inventories.rs:16-20` — **tái sử dụng**, đừng viết lại:
```rust
const SELECT_COLUMNS: &str = "storagetype AS storage_type, slot, itemid AS item_id, quantity, damage, element, \
     elementvalue AS element_value, proofkind AS proof_kind, growlevel AS grow_level, growexp AS grow_exp, \
     specialkind AS special_kind, stoneattr AS stone_attr, stonelevel AS stone_level, \
     enhancelevel AS enhance_level, deletetime AS delete_time, damageditemid AS damaged_item_id, \
     islocked AS is_locked, reinforced, affix1, affix2, affix3, stylelevel AS style_level";
```

**Nên gom** hằng này vào `db/modern/sqlite/mod.rs` để `inventories.rs` và `session.rs` dùng
chung — hiện là `const` private của `inventories.rs:16`.

### 3.3 Truyền `GameData` xuống `load`

`[R]` `SqliteSessionRepository::load(&self, account_id, session)` (`sqlite/session.rs:19`) **không
có** tham số `GameData`. Hai đường:

| Hướng | Đánh giá |
|---|---|
| **A.** Đổi signature `load(id, session, data)` | Sạch, nhưng đụng 6 call-site (`login.rs:110`, `:207`, GM, tests) |
| **B.** Hydrate ở tầng handler sau `load` | **Khuyến nghị** — không đổi signature, 1 chỗ gọi |

⇒ Chọn **B**. Thêm hàm tái sử dụng trong `session.rs`:
```rust
impl Session {
    /// Hydrate toàn bộ template field (`lv`, `int1..fai2`, `loai`, `thuoctinh`, …)
    /// cho mọi item trong 5 container từ `Item.dat`, rồi recompute chỉ số.
    ///
    /// Instance field (`slot`, `id`, `count`, `doben`, `long_val`, `giatri_long`,
    /// `khang`, `texp`) **không** bị ghi đè — chúng đến từ DB.
    pub fn hydrate_templates(&mut self, data: &crate::data::loader::GameData) {
        for bag in [
            &mut self.homdo, &mut self.tientrang, &mut self.tuideo,
            &mut self.trangbi, &mut self.luulang,
        ] {
            for item in bag.iter_mut() {
                let Some(def) = data.items.get(&i64::from(item.id)) else { continue };
                let tpl = InventoryItem::from_template(def, item.count);
                item.lv = tpl.lv;
                item.int1 = tpl.int1;
                item.atk1 = tpl.atk1;
                item.def1 = tpl.def1;
                item.hpx1 = tpl.hpx1;
                item.spx1 = tpl.spx1;
                item.agi1 = tpl.agi1;
                item.fai1 = tpl.fai1;
                item.int2 = tpl.int2;
                item.atk2 = tpl.atk2;
                item.def2 = tpl.def2;
                item.hpx2 = tpl.hpx2;
                item.spx2 = tpl.spx2;
                item.agi2 = tpl.agi2;
                item.fai2 = tpl.fai2;
                item.item_hp = tpl.item_hp;
                item.item_sp = tpl.item_sp;
                item.loai = tpl.loai;
                item.thuoctinh = tpl.thuoctinh;
                item.giatri_thuoctinh = tpl.giatri_thuoctinh;
            }
        }
        self.recompute_stats();
    }
}
```

⚠️ **Không** dùng `*item = tpl` — sẽ mất instance field. Bắt buộc copy từng field template.

Gọi ở `handlers/login.rs` ngay sau `load` thành công (cả 2 nhánh `:110` và `:207`).

---

## 4. Phase 3 — Bước 2: ghi đủ 22 cột

### 4.1 Vấn đề hiện tại

`[R]` `sqlite/session.rs:299-314`:
```sql
DELETE FROM inventories WHERE playerid = ?
```
rồi insert 6 cột ⇒ **xoá vĩnh viễn 16 cột** mỗi lần auto-save.

`[R]` `db/persist.rs:211-225` (`upsert_inventory_tx`) — 5 cột + dòng `let _ = item;` (`:220`).

### 4.2 Cột nào ghi cho field nào

| Field Rust | Cột DB | `save` | `upsert_inventory_tx` |
|---|---|:-:|:-:|
| `slot` | `slot` | ✅ | ✅ |
| `id` | `itemid` | ✅ | ✅ |
| `count` | `quantity` | ✅ | ✅ |
| `doben` | `damage` | ✅ | ✅ |
| `long_val` | `element` | ✅ **mới** | ✅ **mới** |
| `giatri_long` | `elementvalue` | ✅ **mới** | ✅ **mới** |
| `khang` | `proofkind` | ✅ **mới** | ✅ **mới** |
| `texp` | `growexp` | ✅ **mới** | ✅ **mới** |
| *(template)* | `growlevel` | 0 | 0 |
| *(chưa dùng)* | `specialkind`, `stoneattr`, `stonelevel`, `enhancelevel`, `deletetime`, `damageditemid`, `islocked`, `reinforced`, `affix1..3`, `stylelevel` | 0 | 0 |

⚠️ **Quyết định cần chốt:** 10 cột cuối hiện **không có field Rust**. Nhiều khả năng chúng tương
ứng với `int1..fai2` (Items.txt cols 12..19 theo comment `session.rs:28`):

```
affix1 ↔ int1    affix2 ↔ atk1    affix3 ↔ def1
stylelevel ↔ loai (vị trí trang bị)
growlevel ↔ lv   (yêu cầu level)
enhancelevel ↔ agi1?
```

⚠️ **Mapping này là giả định** — phải đối chiếu `data/loaders/item.rs` + `Items.txt` +
`protocol/codecs/thing_data.rs:34-79` trước.

⇒ **An toàn:** ghi 0 cho 10 cột đó trong Phase 3. Điều này **không làm hỏng gì** vì hiện tại
`save` cũng ghi 0 (mặc định) cho chúng — chỉ là **không ghi vượt mức hiện tại**. Nâng cấp mapping
ở Phase 4/6 sau khi xác minh.

⇒ `save` giữ nguyên chiến lược `DELETE` + re-insert (đơn giản, nguyên tử) nhưng insert **23 cột**
(22 cột item + `playerid`).

### 4.3 SQL đề xuất

```rust
/// 23 cột: `playerid`, `storagetype`, `slot` + 20 cột item của bảng `inventories`.
/// Dùng chung cho `save`, `create_and_seed` và `persist::upsert_inventory_tx`.
pub const INVENTORY_COLUMNS: &str = "playerid, storagetype, slot, itemid, quantity, damage, \
     element, elementvalue, proofkind, growlevel, growexp, specialkind, stoneattr, stonelevel, \
     enhancelevel, deletetime, damageditemid, islocked, reinforced, affix1, affix2, affix3, stylelevel";
```

⚠️ SQLite không cho bind theo tên ⇒ phải sinh `VALUES (?, ?, …)` cùng độ dài. Dùng helper:
```rust
pub fn inventory_placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(", ")
}
```

---

## 5. Phase 1 (trước) — Gom mapping `storagetype`

### 5.1 Vấn đề

`[R]` **6 nơi** phải giữ đồng bộ (bảng ở `01-container-mapping.md` §3.3).

### 5.2 Nguồn sự thật duy nhất

Đặt cạnh `StorageType` trong `db/modern/model.rs`:
```rust
impl StorageType {
    /// Tên Rust của container (khớp key trong `persist::storage_type`).
    pub const fn table_name(self) -> &'static str {
        match self {
            Self::Bag => "homdo",
            Self::Bank => "tientrang",
            Self::Secondary => "tuideo",
            Self::Equip => "trangbi",
            Self::Warehouse => "luulang",
        }
    }

    /// Frame dump S→C (hex code 2 byte) của container này.
    pub const fn dump_code(self) -> &'static str {
        match self {
            Self::Bag => "1705",
            Self::Bank => "1E01",
            Self::Secondary => "172F",
            Self::Equip => "170B",
            Self::Warehouse => "1766",
        }
    }

    /// Sức chứa thực tế theo guard của client parser.
    pub const fn capacity(self) -> u16 {
        match self {
            Self::Bag | Self::Secondary => 25,
            Self::Bank | Self::Warehouse => 50,
            Self::Equip => 6,
        }
    }

    /// Format record trên wire.
    pub const fn entry_layout(self) -> EntryLayout {
        match self {
            Self::Equip => EntryLayout::Slotless10,
            _ => EntryLayout::SlotLed12,
        }
    }

    /// Parse từ tên bảng legacy (`persist::storage_type` ngược lại).
    pub fn from_table_name(name: &str) -> Option<Self> {
        match name {
            "homdo" => Some(Self::Bag),
            "tientrang" => Some(Self::Bank),
            "tuideo" => Some(Self::Secondary),
            "trangbi" => Some(Self::Equip),
            "luulang" => Some(Self::Warehouse),
            _ => None,
        }
    }
}

/// Format record inventory trên wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryLayout {
    /// 12 byte, byte slot đầu — `17 05`, `1E 01`, `17 2F`.
    SlotLed12,
    /// 10 byte, **không** byte slot — `17 0B` (client `IDIV 0x0A`).
    Slotless10,
}
```

### 5.3 Refactor các call-site

| Nơi | Trước | Sau |
|---|---|---|
| `persist.rs:135-144` | `fn storage_type(table: &str) -> Option<u8>` | `StorageType::from_table_name(table)` |
| `sqlite/session.rs:303-309` | tuple `[(1u8, &session.homdo), …]` | `session.containers()` |
| `sqlite/session.rs:542-547` | tuple lặp lại | `session.containers()` |
| `sqlite/session.rs:72-76` | `load_items(id, 1/2/4/8/16)` | `load_items(id, StorageType::…)` |
| `inventory.rs:9` | `HOMDO_SLOTS` | `StorageType::Bag.capacity()` |

Thêm vào `Session`:
```rust
/// 5 container theo thứ tự `StorageType` — nguồn duy nhất cho load / save / seed,
/// thay cho 3 tuple literal rời rạc trong `sqlite/session.rs`.
pub fn containers(&self) -> [(StorageType, &[InventoryItem]); 5] {
    [
        (StorageType::Bag, &self.homdo),
        (StorageType::Bank, &self.tientrang),
        (StorageType::Secondary, &self.tuideo),
        (StorageType::Equip, &self.trangbi),
        (StorageType::Warehouse, &self.luulang),
    ]
}
```
⚠️ Trả `&[InventoryItem]` — không mutate. Nếu cần mutate thì thêm `containers_mut()`.
Kiểm tra nhu cầu thực tế: `sqlite/session.rs` chỉ đọc + clone ⇒ `&` là đủ.

---

## 6. Clamp tiền tốc `giatri_long`

`[R]` `session.rs:461`:
```rust
(item.giatri_long as u16 + 100) as u8
```

`[B-verified]` Bear: `(byte)(100 + (byte)other_val)` — **cùng hành vi tràn**, nên đây là **chủ ý
giao thức** chứ không phải bug.

`[B-verified]` `ItemData.cs:78` có xử lý riêng cho props 65..67:
```csharp
if (value.prop1 >= 65 && value.prop1 <= 67) { value.prop1_val += 100; }
```

⇒ `+100` ở wire là **bias cố định cho giá trị forge**. Nếu `giatri_long > 155` sẽ tràn, client
hiển thị sai.

**Khuyến nghị:** guard ở tầng nghiệp vụ, **không** sửa công thức wire (khớp Bear):
```rust
/// Bias +100 theo giao thức (`TSItemContainer.sendItems`). Giá trị > 155
/// sẽ tràn u8 — chặn ở nguồn thay vì đổi công thức wire (Bear cũng tràn).
pub const GIA_TRI_LONG_BIAS: u16 = 100;
pub const GIA_TRI_LONG_MAX: u8 = 155;

pub fn biased_giatri_long(raw: u8) -> u8 {
    debug_assert!(raw <= GIA_TRI_LONG_MAX, "giatri_long {raw} > 155 sẽ tràn wire");
    ((u16::from(raw) + GIA_TRI_LONG_BIAS) & 0xFF) as u8
}
```

---

## 7. Gợi ý: `ThingData` 35 byte

`[R]` `protocol/codecs/thing_data.rs` định nghĩa struct 20 field, tổng 35 byte, khớp 1:1 với
20 cột `inventories`. Comment (`thing_data.rs:3-4`) nói nó lấy từ `Item.lua` của **Kotlin TS
Mobile reference** — không phải Bear.

**Hiện trạng:** `SqliteInventoryRepository` (`sqlite/inventories.rs`) dùng nó đúng cách, **nhưng
repo đó không được wire vào đường live** — chỉ `transactions.rs:91,109` và tests gọi.

| Lựa chọn | Ưu | Nhược |
|---|---|---|
| **A. Bỏ `ThingData`** | Xoá code chết | Mất sẵn cho công việc tương lai |
| **B. Wire `ThingData` vào đường live** | 1 nguồn sự thật | Refactor lớn: `Session` dùng `InventoryItem`, không dùng `ThingData` |
| **C. Giữ nguyên, document "chưa dùng"** | Rủi ro thấp nhất | Vẫn 2 mô hình item song song |

⇒ **Khuyến nghị C trong Phase 3**, hướng tới B ở Phase 6 nếu thực sự cần affix/ngọc khảm.
Ghi rõ trong `AGENTS.md`/`CONTEXT.md` để agent sau không hiểu nhầm là đang dùng.

---

## 8. `persist::upsert_inventory_tx` — dòng `let _ = item;`

`[R]` `db/persist.rs:220`
```rust
let _ = item;   // ← dấu hiệu refactor dở dang
sqlx::query("INSERT INTO inventories (playerid, storagetype, slot, itemid, quantity, damage) ...")
```

Dòng này không làm gì — nhiều khả năng tác giả định dùng `item` cho field khác nhưng chưa làm.

**Sau Phase 3:** đồng bộ `upsert_inventory_tx` với `save` — cùng bộ 23 cột, dùng chung hằng
`INVENTORY_COLUMNS`.

---

## 9. Bảng kiểm thử DB (đề xuất)

Tất cả ở `tests/`, dùng in-memory SQLite theo `AGENTS.md`:
```
sqlite::memory:?cache=shared
```

| Test file | Nội dung |
|---|---|
| `tests/inventory_roundtrip_test.rs` | `save` → `load` ⇒ 8 instance field × 5 container khớp |
| `tests/inventory_template_hydrate_test.rs` | `hydrate_templates` ⇒ `loai`/`lv`/`int1..fai2` khớp `Item.dat` |
| `tests/inventory_gear_bonus_test.rs` | `hydrate_templates` + `recompute_stats` ⇒ `int2..agi2` khớp `character_sheet` |
| `tests/inventory_storage_type_test.rs` | `dump_code`/`capacity`/`entry_layout` khớp `01-container-mapping.md` |

⚠️ `AGENTS.md`: **cấm** `#[cfg(test)]` trong `src/`; import qua `ts_dream::...` public API.
⇒ `hydrate_templates` phải là `pub` trên `Session`; `StorageType`/`EntryLayout` phải `pub`.

---

## 10. Tổng hợp Phase 3

| # | Việc | File | Rủi ro |
|:-:|---|---|:-:|
| D-1 | `Session::hydrate_templates()` | `session.rs` (mới) | 🟢 thấp |
| D-2 | Gọi `hydrate_templates` sau `load` | `handlers/login.rs:110,207` | 🟢 thấp |
| D-3 | `load_items` đọc đủ 8 field wire | `sqlite/session.rs:176` | 🟢 thấp |
| D-4 | `save` insert 23 cột | `sqlite/session.rs:311` | 🟡 trung bình |
| D-5 | `upsert_inventory_tx` đồng bộ | `persist.rs:221` | 🟡 trung bình |
| D-6 | Gom mapping vào `StorageType` | `model.rs` | 🟢 thấp |
| D-7 | `Session::containers()` thay 3 tuple | `session.rs` + `sqlite/session.rs` | 🟢 thấp |
| D-8 | Clamp `giatri_long ≤ 155` | `session.rs` + `inventory.rs` | 🟢 thấp |
| D-9 | Persist `tiengtam`/`god`/`color`/`hp_store`/`sp_store` | `0002_*.sql` mới | 🟡 cần migration |
| D-10 | Xác minh mapping affix↔stat | `data/loaders/item.rs` | 🔴 **chưa rõ** |

⚠️ **D-9 cần file migration mới** (`0002_*.sql`) vì `AGENTS.md` cấm thêm `ALTER` vào `0001`.
Cột thiếu: `tiengtam`, `god`, `hpstore`, `spstore`, `color`, `gocnhin`, `savemap`. Chưa biết cột
nào thực sự cần ⇒ **có thể đẩy D-9 sang Phase 4** để Phase 3 gọn.

⚠️ **D-10 nên ghi 0 tạm** trong Phase 3 (không làm hỏng gì so với hiện tại), nâng cấp ở Phase 4/6.