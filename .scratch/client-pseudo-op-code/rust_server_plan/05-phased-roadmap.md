# 05 — Roadmap Triển Khai Theo Phase

> Kế hoạch thực thi. Mỗi phase có: **mục tiêu**, **task cụ thể**, **tiêu chí nghiệm thu**,
> **rủi ro + rollback**.
> Ưu tiên theo `02-gap-analysis.md` §1.

---

## 0. Tổng quan

```
Phase 0  Capture & Xác minh        ──►  chặn W-5, W-7, G-02, G-11, G-12
   │
Phase 1  Nền: Container Metadata    ──►  🟢 độc lập, mở đường cho Phase 3
   │
   ├── Phase 2A  Wire lỗ (an toàn)  ──►  🟢 độc lập DB, không cần capture
   ├── Phase 2B  Wire nghi ngờ      ──►  🟡 cần capture từ Phase 0
   │
Phase 3  DB persist + template      ──►  🟢 độc lập Phase 2
   │
Phase 4  Handler & nghiệp vụ        ──►  phụ thuộc Phase 3
   │
Phase 5  Test toàn diện             ──►  phụ thuộc tất cả
   │
Phase 6  Mở rộng container mới       ──►  tùy
```

**Quy tắc ưu tiên:** Phase 2A và Phase 3 **có thể chạy song song** (không cùng đụng file).
Phase 2B **phải đợi** Phase 0.

---

## Phase 0 — Capture & Xác minh (chặn)

### Mục tiêu

Đóng các gap **không thể quyết bằng đọc code**. Không có phase này, W-5/W-7/G-02 mãi mở.

### Task

| # | Task | Đóng gap | Cần gì |
|:-:|---|---|---|
| P0-1 | Capture `05 03` byte-exact từ `aLogin.exe` | G-07, W-7 | Server chạy + client thật |
| P0-2 | Đọc client parser `05` S→C (`case_005` → `FUN_0078C7DB`) | G-07 | Dump đầy đủ `aLogin.exe` |
| P0-3 | Đọc client parser `08 01` S→C (`case_009` → `FUN_0078D125`) | G-06, W-5 | Như P0-2 |
| P0-4 | Đọc body `FUN_00769B68` (`17 2F`) | G-02 (túi đeo) | Như P0-2 |
| P0-5 | Đọc body `FUN_0076A320` (`17 66`) | G-02 (lưu lang) | Như P0-2 |
| P0-6 | Đọc body `FUN_00768638` (ghi vào slotObj) | G-11 (stall price vs id) | Như P0-2 |
| P0-7 | Capture `17 20` stall close + uid offset | G-12 | Client thật |
| P0-8 | Capture hành vi `1E 01` khi không clear (ô cũ còn không) | G-08 | Client thật |

### Tiêu chí nghiệm thu

Mỗi task tạo 1 entry trong `.scratch/client-pseudo-op-code/` với:
- Địa chỉ hàm + file `.c`/`.asm.txt` đã lưu vào `client_pseudo_c/`
- Byte layout dạng bảng offset
- Ghi rõ `[C-verified]`

### Rủi ro

| Rủi ro | Giảm thiểu |
|---|---|
| Không có `aLogin.exe` / không chạy được client | Chấp nhận G-02/W-7 đóng "unresolved"; dùng phương án an toàn (tạm dừng `17 66`, giữ `05 03` hiện tại) |
| Đọc `FUN_0078A89C` dispatcher không đủ | Cần dump **function body**, không chỉ xref |

---

## Phase 1 — Nền: Container Metadata

### Mục tiêu

Tạo **nguồn sự thật duy nhất** cho mapping `storagetype ↔ opcode ↔ capacity ↔ layout`.
Xoá 6 bản sao. Không đổi **byte nào** trên wire.

### Task

| # | Task | File | Mức |
|:-:|---|---|:-:|
| 1-1 | Thêm `EntryLayout` enum + `table_name`/`dump_code`/`capacity`/`entry_layout`/`from_table_name` vào `impl StorageType` | `db/modern/model.rs:14-42` | 🟢 |
| 1-2 | Thêm `Session::containers()` | `session.rs` | 🟢 |
| 1-3 | Thay 3 tuple literal bằng `containers()` | `sqlite/session.rs:303-309`, `:542-547` | 🟢 |
| 1-4 | `load_items` nhận `StorageType` thay `u8` | `sqlite/session.rs:176-199` | 🟢 |
| 1-5 | `persist::storage_type` → `StorageType::from_table_name` | `persist.rs:135-144` | 🟢 |
| 1-6 | Thay `HOMDO_SLOTS` bằng `StorageType::capacity()` | `inventory.rs:9`, `trade_storage.rs:174` | 🟢 |
| 1-7 | Tách `dump_table` → `encode_entry_12` + `encode_entry_10` | `session.rs:449-468` | 🟢 |

### 1-6 chi tiết — sửa G-09 (sức chứa)

```rust
// Trước — inventory.rs:30-32
pub fn free_slot(bag: &[InventoryItem]) -> Option<u8> {
    (1..=HOMDO_SLOTS).find(|slot| !bag.iter().any(|i| i.slot == *slot && i.id > 0))
}
```

Đổi thành nhận capacity:
```rust
/// Ô trống đầu tiên trong `1..=capacity`, hoặc `None` khi đầy.
/// `capacity` phải khớp guard của client parser — xem
/// `rust_server_plan/01-container-mapping.md` §2.
pub fn free_slot_in(bag: &[InventoryItem], capacity: u16) -> Option<u8> {
    (1..=capacity).find(|slot| !bag.iter().any(|i| i.slot == *slot && i.id > 0))
        .and_then(|s| u8::try_from(s).ok())
}

/// Giữ hàm cũ cho hòm đồ (25 ô) để không phá call-site hiện có.
pub fn free_slot(bag: &[InventoryItem]) -> Option<u8> {
    free_slot_in(bag, StorageType::Bag.capacity())
}
```
Rồi trong `trade_storage.rs:187` dùng `free_slot_in(…, StorageType::Bank.capacity())`.

### 1-7 chi tiết — tách encoder

```rust
/// Encode 1 record format 12B (slot-led): `17 05`, `1E 01`, `17 2F`.
/// Client chia `IDIV 0xC`. Thứ tự record KHÔNG quan trọng (có byte slot).
fn encode_entry_12(item: &InventoryItem) -> String {
    format!(
        "{:02X}{}{:02X}{:02X}{:02X}{:02X}{:02X}{}",
        item.slot,
        encoder::le16(item.id),
        item.count,
        item.doben,
        item.long_val,
        crate::server::inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

/// Encode 1 record format 10B (không slot): `17 0B`.
/// Client chia `IDIV 0xA` và tự định tuyến theo body position.
fn encode_entry_10(item: &InventoryItem) -> String {
    format!(
        "{}{:02X}{:02X}{:02X}{:02X}{}",
        encoder::le16(item.id),
        item.count,
        item.long_val,
        crate::server::inventory::biased_giatri_long(item.giatri_long),
        item.khang,
        encoder::le32(item.texp)
    )
}

/// Serialize container ra frame dump, chọn layout theo `StorageType`.
pub fn dump_container(items: &[InventoryItem], storage: StorageType) -> String {
    let code = storage.dump_code();
    match storage.entry_layout() {
        EntryLayout::SlotLed12 => {
            let entries: String = items.iter().filter(|i| i.id > 0)
                .map(encode_entry_12).collect();
            crate::protocol::frame(code, &entries)
        }
        EntryLayout::Slotless10 => {
            let mut sorted: Vec<&InventoryItem> = items.iter().filter(|i| i.id > 0).collect();
            sorted.sort_by_key(|i| i.slot);   // bắt buộc: định tuyến theo thứ tự
            let entries: String = sorted.iter().map(|i| encode_entry_10(i)).collect();
            crate::protocol::frame(code, &entries)
        }
    }
}
```

### Tiêu chí nghiệm thu

- ✅ `cargo build` sạch, không warning mới
- ✅ `cargo test --all-targets --no-fail-fast` pass (golden frames **không đổi**)
- ✅ `grep -rn "1u8, &session.homdo"` → 0 kết quả (đã gom)
- ✅ Test mới `tests/inventory_storage_type_test.rs` assert 5 mapping đúng bảng `01`

### Rủi ro & rollback

| Rủi ro | Mức | Rollback |
|---|:-:|---|
| Đổi `free_slot` làm lệch sức chứa túi | 🟡 | Giữ `free_slot` cũ, chỉ dùng `free_slot_in` ở call-site mới |
| Tách encoder làm lệch byte | 🟢 | Golden test bắt được; revert commit |
| `containers()` trả `&` không đủ cho `save` | 🟡 | Thêm `containers_mut()` |

---

## Phase 2A — Wire lỗi an toàn (không cần capture)

### Mục tiêu

Sửa 4 lỗi wire **đã có bằng chứng độc lập** từ client + Bear. Rủi ro thấp.

### Task

| # | Task | Gap | File | Rủi ro |
|:-:|---|---|---|:-:|
| 2A-1 | `dump_trangbi` → 10B, sort theo slot | G-01 | `session.rs:429-432` | 🟡 |
| 2A-2 | `1E 08` clear trước `1E 01` | G-08 | `spawn.rs:551-556` | 🟢 |
| 2A-3 | `17 66` sau feature flag | G-02 | `spawn.rs:555`, `config.rs` | 🟢 |
| 2A-4 | Xác minh lại `17 1E` stall (đọc `FUN_00768638`) | G-11 | không sửa nếu chưa capture | 🟢 |

### 2A-1 — chi tiết

Sửa `session.rs:429-432` để dùng `encode_entry_10` (thiết lập ở Phase 1):
```rust
/// Dump Trangbi equipment frame (`F444` + len + `170B` + entries).
///
/// **10 byte/record, KHÔNG byte slot.** Client `FUN_0077CC2C` chia `IDIV 0xA`;
/// Bear `TSCharacter.sendEquip()` cũng ghi `add16+add8×4+add32` (10B).
/// Thứ tự record **bắt buộc** theo `slot` tăng dần vì client định tuyến bằng
/// body position trong `Item.dat`, không đọc byte slot.
pub fn dump_trangbi(&self) -> String {
    dump_container(&self.trangbi, StorageType::Equip)
}
```

⚠️ **Quyết định cần chốt:** `doben` **không** còn chỗ ở format 10B. Xem `03-wire-codecs.md` §2.3.
Nếu cần độ bền trên wire ⇒ phải tìm frame khác (`17 0C`?). **Chưa rõ** → ghi TODO + capture.

⚠️ **Số frame Logined1 không đổi** (vẫn 1 frame `17 0B`) ⇒ `tests/login_char_flow_test.rs:234`
vẫn pass, chỉ **nội dung frame đổi** ⇒ cần cập nhật golden/assert nội dung.

### 2A-2 — chi tiết

```rust
// spawn.rs, step 10
// 10. Step 10: Inventory dumps
frames.push(s.dump_homdo());
// `FUN_007707A0` (`1E 01`) KHÔNG có clear pass, khác `FUN_0076BFC8` (`17 05`).
// Gửi `1E 08` để xoá 50 ô cũ trước khi dump, tránh ô "ma" từ lần login trước.
frames.push(crate::protocol::frame("1E08", ""));
frames.push(s.dump_tientrang());
frames.push(s.dump_tuideo());
if cfg.send_luulang_frame {
    frames.push(s.dump_luulang());
}
frames.push(s.dump_trangbi());
```
⚠️ **Số element Logined1: 21 → 21** (thêm `1E 08` +1, bỏ `17 66` −1 khi flag off).
Nếu bật `send_luulang_frame = true` ⇒ **22**.

> **Cảnh báo khi đếm frame.** `[R]` `tests/login_char_flow_test.rs:234,352` đếm
> `out.outgoing.len()` — tức **số phần tử của `Vec`**, KHÔNG phải số frame thật trên wire.
> Element thứ 13 là `"F44402000504F44402000F0A"` (`spawn.rs:570`) = **2 frame gộp vào 1 chuỗi**.
> ⇒ `outgoing.len() == 21` nhưng thực tế là **22 frame**. `login_start()` (`spawn.rs:73-75`)
> thì trả 2 chuỗi riêng biệt.
>
> Khi so với capture `aLogin.exe`, phải tách chuỗi gộp trước khi đếm, nếu không sẽ lệch 1.
> Test nên tách thành 2 element riêng để khớp frame thật — nhưng đó là **thay đổi breaking**
> cho golden, để Phase 5.

### 2A-3 — chi tiết

`config.rs`:
```rust
/// Gửi frame `17 66` (kho lưu lãng) trong Logined1. Mặc định `false`.
/// Layout `17 66` **chưa được xác minh** (thiếu body `FUN_0076A320`) và Bear
/// không hề gửi frame này. Bật lên chỉ sau capture — xem
/// `.scratch/client-pseudo-op-code/rust_server_plan/01-container-mapping.md` §2.4.
pub send_luulang_frame: bool,
```

### Tiêu chí nghiệm thu

- ✅ `17 0B` sinh ra **10 × n** byte body, test assert `(len-2) % 10 == 0`
- ✅ Logined1 có frame `1E 08` ngay trước `1E 01`
- ✅ Không có frame `17 66` khi `send_luulang_frame = false`
- ✅ Golden test cập nhật

### Rủi ro & rollback

| Rủi ro | Mức | Xử lý |
|---|:-:|---|
| `17 0B` 10B làm client mất trang bị hiển thị | 🟡 | Rollback 1 dòng; capture `17 0C`/`expitem()` để hiểu `doben` |
| `1E 08` không hoạt động như mong đợi | 🟢 | Frame vô hại; bỏ nếu thấy lỗi |
| Bỏ `17 66` làm mất chức năng lưu lãng | 🟡 | Bật flag nếu nghiệp vụ cần gấp |

---

## Phase 3 — DB persist + template hydrate

### Mục tiêu

Không mất field nào qua `load → mutate → save`; template field luôn đúng sau relog.

### Task

Đặc tả đầy đủ: [`04-db-persistence.md`](./04-db-persistence.md).

| # | Task | Gap | Phụ thuộc |
|:-:|---|---|---|
| 3-1 | `Session::hydrate_templates()` | G-05 | Phase 1 |
| 3-2 | Gọi sau `load` ở cả 2 nhánh login | G-05 | 3-1 |
| 3-3 | `load_items` đọc đủ 8 field wire | G-03 | Phase 1 |
| 3-4 | `save` insert 23 cột | G-04 | 3-6 |
| 3-5 | `upsert_inventory_tx` đồng bộ 23 cột | G-04 | 3-6 |
| 3-6 | Hằng `INVENTORY_COLUMNS` dùng chung | — | Phase 1 |
| 3-7 | Persist `handle_move_stack` (sub 10) | G-10 | 3-4 |
| 3-8 | `auto_save::fingerprint` mix cả 5 container | G-10 | — |

### 3-7 chi tiết — sửa G-10

`[R]` `handlers/inventory.rs:218-273` `handle_move_stack(conn, payload, decoded, out)`
không có `pool` ⇒ không persist.

```rust
async fn handle_move_stack(
    conn: &mut Conn,
    payload: &[u8],
    decoded: &[u8],
    out: &mut HandleOutcome,
    pool: Option<&DbPool>,
) {
    // … logic cũ, nhưng thu thập `touched: Vec<InventoryItem>` mỗi khi mutate …
    // Cuối hàm:
    for item in &touched {
        persist::upsert_item(pool, conn.session.id, "homdo", item).await;
    }
    out.send(hex_of(decoded));
}
```
⚠️ Phải persist **mọi** slot bị ảnh hưởng: swap = 2 slot; split stack = 2 slot;
`add_item` có thể trả 2 slot (`inventory.rs:40-72` ghi rõ).

### 3-8 chi tiết

`[R]` `auto_save.rs:38-120` `fingerprint` chỉ mix `homdo` + `trangbi` + `quest_items`
(`:55-66`, `:102-108`) ⇒ đổi ở `tientrang`/`tuideo`/`luulang` **không** kích hoạt auto-save.

Thêm `tientrang`, `tuideo`, `luulang` vào hash.

### Tiêu chí nghiệm thu

- ✅ `tests/inventory_roundtrip_test.rs`: save→load giữ nguyên 8 instance field × 5 container
- ✅ `tests/inventory_template_hydrate_test.rs`: sau `hydrate_templates`, `loai` khớp `Item.dat`
- ✅ `tests/inventory_gear_bonus_test.rs`: sau hydrate + recompute, `int2..agi2` khớp `character_sheet`
- ✅ `handle_move_stack` → disconnect → reconnect ⇒ giữ nguyên layout
- ✅ Auto-save kích hoạt khi chỉ sửa `tientrang`

### Rủi ro & rollback

| Rủi ro | Mức | Xử lý |
|---|:-:|---|
| `hydrate_templates` chậm (5 túi × N item × hash lookup) | 🟢 | Chỉ chạy 1 lần / login; có thể cache |
| `load_items` đọc 22 cột chậm hơn 4 cột | 🟢 | Bù bằng index `inventories(playerid, storagetype)`; cân nhắc thêm |
| Ghi 23 cột làm `save` chậm | 🟡 | Đo trước/sau; `DELETE`+re-insert vốn đã là nghẽn |
| Mapping affix sai | 🔴 | **Ghi 0 tạm** (không tệ hơn hiện tại); xác minh ở Phase 4/6 |

---

## Phase 2B — Wire nghi ngờ (cần capture)

### Mục tiêu

Sửa 3 lỗi **cần bằng chứng client** mới làm được.

### Task

| # | Task | Gap | Chặn bởi |
|:-:|---|---|---|
| 2B-1 | Bảng `prop_code` + sửa equip burst (`0x19`←hp, `0xD7`←hp_max, `0xDA`/`0xDB`←hpx2/spx2) | G-06 | P0-3 |
| 2B-2 | Thêm `hp2`/`sp2` vào `Session` + `GearBonuses` + DB | G-06 | 2B-1 |
| 2B-3 | `05 03` tail → `43` zero + block `_3`/`_4` 32 byte | G-07 | P0-1, P0-2 |
| 2B-4 | `GearBonuses` theo prop code thay vì "cộng vào tất cả" | G-06 | P0-3 |
| 2B-5 | `17 66` → `1E 09` nếu không xác minh được | G-02 | P0-5 |
| 2B-6 | `17 1E`/`17 20` stall field | G-11, G-12 | P0-6, P0-7 |

### 2B-1 chi tiết

Định nghĩa bảng một chỗ (`stats.rs` hoặc `protocol/mod.rs`):
```rust
/// Prop code frame `08 01` — nguồn chuẩn: `TS_Server_Bear/TS_Server/TSConstants.cs:5-67`
/// và `TSCharacter.addEquipBonus():1682-1718`. **Đã xác minh bằng capture client.**
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
Rồi burst equip/unequip gửi **12 frame** (thay vì 8):
```rust
out.send(build_stat_update(prop_code::HP, conn.session.hp as i32));
out.send(build_stat_update(prop_code::SP, conn.session.sp as i32));
out.send(build_stat_update(prop_code::FULL_HP_MAX, conn.session.hp_max as i32));
out.send(build_stat_update(prop_code::FULL_SP_MAX, conn.session.sp_max as i32));
out.send(build_stat_update(prop_code::HP2, conn.session.hp2 as i32));     // Phase 2B-2
out.send(build_stat_update(prop_code::SP2, conn.session.sp2 as i32));
out.send(build_stat_update(prop_code::ATK2, conn.session.atk2 as i32));
out.send(build_stat_update(prop_code::DEF2, conn.session.def2 as i32));
out.send(build_stat_update(prop_code::MAG2, conn.session.int2 as i32));
out.send(build_stat_update(prop_code::AGI2, conn.session.agi2 as i32));
out.send(build_stat_update(prop_code::HPX2, conn.session.hpx2 as i32));
out.send(build_stat_update(prop_code::SPX2, conn.session.spx2 as i32));
```

### 2B-3 chi tiết — rủi ro cao

⚠️ **Không tự sửa `05 03` khi chưa biết client đọc fixed-offset hay tới-cuối.**
Nếu client đọc tới-cuối ⇒ `90` zero hiện tại **vô hại**, sửa thành `43` cũng vô hại.
Nếu client đọc fixed ⇒ `90` **sai**, phải sửa.

Quy trình: capture → xác định parser → sửa → test.

### Tiêu chí nghiệm thu

- ✅ Mỗi frame `08 01` trong `src/` dùng hằng `prop_code::*`, không literal rời rạc
- ✅ `grep -rn "build_stat_update(0x" src/` → chỉ còn trong `prop_code` module
- ✅ `05 03` khớp capture byte-for-byte

### Rủi ro & rollback

| Rủi ro | Mức | Rollback |
|---|:-:|---|
| Sửa `08 01` làm HUD sai | 🔴 | Bật feature flag `stat_push_v2`; giữ đường cũ |
| Thêm `hp2`/`sp2` lan toả nhiều file | 🟡 | Tách commit riêng |
| Sửa `05 03` phá golden test | 🟡 | Cập nhật golden; nếu vẫn sai ⇒ revert |

---

## Phase 4 — Handler & nghiệp vụ

### Mục tiêu

Sửa các gap mức P2/P3 sau khi wire + DB đã ổn định.

### Task

| # | Task | Gap | File |
|:-:|---|---|---|
| 4-1 | `17 36` trả `17 32` + `17 09` + `17 30` (không phải `1706` lặp) | G-13 | `handlers/inventory.rs:612-622` |
| 4-2 | `17 37` move túi đeo thực sự | G-20 | `handlers/inventory.rs:624-636` |
| 4-3 | `pet_equip` ghi vào `trangbi[stt*10+sub]` | G-14 | `handlers/inventory.rs:554-580` |
| 4-4 | `pet_unequip` thực sự gỡ | G-14 | `handlers/inventory.rs:582-599` |
| 4-5 | Sửa `u16 as u8` tràn ở `move_pet_slot` | G-14 | `handlers/pet_actions.rs:50,96-97` |
| 4-6 | `kind 1` hotkey = item quickslot | — | `handlers/stats.rs:142-165` |
| 4-7 | Persist `tiengtam`/`god`/`color`/`hp_store`/`sp_store` | G-18 | `0002_*.sql` |
| 4-8 | `1A 04` field thứ 2 = currency pool thứ 2 | — | `spawn.rs:561-564` |
| 4-9 | Xác minh + map affix↔stat | G-03/D-10 | `data/loaders/item.rs` |
| 4-10 | `clear_items` / `replace_inventory_tx` dùng `StorageType` | G-15 | `persist.rs:146,280` |

### 4-1 chi tiết — theo Bear

`[B-verified]` `BagHandle.cs:16-97` (`case 36` = inventory → bag):
```csharp
packetCreator2 = new PacketCreator(23, 50);   client.reply(...);   // 17 32: mở form
packetCreator2 = new PacketCreator(23, 9);    // 17 09
packetCreator2.add16(data[n + 2]);           // slot
client.reply(...);
… thực hiện chuyển …
packetCreator2 = new PacketCreator(23, 48);   // 17 30: record 12B
packetCreator2.add16(Itemid); add16(quantity); addByte(0);
packetCreator2.addByte(other_type); addByte(100+other_val);
packetCreator2.addByte(anti); add32(exp);
```
⚠️ Bear dùng `add16(quantity)` — **12B record không có byte slot**, khác `sendItems`.
Đây là frame **delta 1 ô**, không phải container dump. Cần xác minh client `17 30` xử lý thế nào.

### Tiêu chí nghiệm thu

- ✅ `pet_equip` → `disconnect` → `reconnect` ⇒ pet vẫn mặc đồ
- ✅ `17 36` trả frame đúng, không còn `1706` lặp
- ✅ `1A 04` field 2 có giá trị ≠ 0 sau khi cài

### Rủi ro

| Rủi ro | Mức | Ghi chú |
|---|:-:|---|
| 4-1/4-2 sai vì chưa xác minh client `17 30`/`17 32` | 🔴 | Cần P0 bổ sung; hoãn nếu chưa capture |
| 4-3 đổi cấu trúc pet equip | 🟡 | Test kỹ với pet thật |
| 4-7 cần migration | 🟡 | Tạo `0002_*.sql`, không sửa `0001` |

---

## Phase 5 — Test toàn diện

### Mục tiêu

Chốt bằng chứng mọi fix không hồi quy.

### Task

Đặc tả: [`06-tests-and-capture.md`](./06-tests-and-capture.md).

| # | Task |
|:-:|---|
| 5-1 | Unit test codec: 12B/10B encode, round-trip |
| 5-2 | Unit test `StorageType` metadata |
| 5-3 | Integration test round-trip DB |
| 5-4 | Integration test hydrate + gear bonus |
| 5-5 | Cập nhật `tests/login_char_flow_test.rs` (số frame) |
| 5-6 | Cập nhật golden packets |
| 5-7 | Test `handle_move_stack` persist |
| 5-8 | Test `auto_save` fingerprint đủ 5 container |

### Tiêu chí nghiệm thu

```bash
cargo test --all-targets --no-fail-fast
# 0 failed
```

---

## Phase 6 — Mở rộng (tùy chọn)

| # | Task | Ghi chú |
|:-:|---|---|
| 6-1 | Thêm container `1E 09` (Ba Đậu Yêu Balo, 51 ô, 12B) | Body client **đã có** — sẵn sàng |
| 6-2 | Thêm container `1E 0C` (Thủy binh bank, 26 ô) | Body client **đã có**; đặc tả `economy/05_special_economy.md:498` |
| 6-3 | Wire `ThingData` 35B vào đường live | Xem `04-db-persistence.md` §7 |
| 6-4 | Affix / ngọc khảm (`17 26` dice, `17 0E` craft) | Stub hiện tại |
| 6-5 | Slot machine `0x48`, Kỳ Trân Các `0x4B` | `economy` gap G39, G40 |

⚠️ **6-1/6-2 cần `storagetype` mới** → migration `0002_*.sql`.
Đề xuất: `1E 09` → `storagetype = 32` (Ba Đậu Yêu Balo), `1E 0C` → `storagetype = 64`
(Thủy binh bank) — tiếp tục chuỗi bit 1/2/4/8/16/32/64.

---

## 6. Ma trận phụ thuộc

| Task | Phase 1 | 2A | 2B | 3 | 4 |
|---|:-:|:-:|:-:|:-:|:-:|
| 1-1..1-5 | — | | | | |
| 1-6 (`free_slot_in`) | — | | | | |
| 1-7 (tách encoder) | — | | | | |
| 2A-1 (`17 0B` 10B) | ✅ 1-7 | | | | |
| 2A-2 (`1E 08`) | — | | | | |
| 2A-3 (`17 66` flag) | — | | | | |
| 2B-1..2B-4 | ✅ P0-3 | | — | ✅ 2B-2 | |
| 3-1..3-3 | ✅ 1-2, 1-4 | | | | |
| 3-4, 3-5 | ✅ 1-6, 3-3 | | | | |
| 3-7, 3-8 | | | | | |
| 4-3..4-5 | | | | ✅ 3-1 | |

---

## 7. Ước lượng effort

| Phase | Task | Ước lượng | Phụ thuộc capture? |
|---|---|---|:-:|
| 0 | 8 capture | 1-3 ngày | — (cần client) |
| 1 | 7 refactor | 0.5-1 ngày | ❌ |
| 2A | 4 fix wire | 0.5 ngày | ❌ |
| 3 | 8 task DB | 1-2 ngày | ❌ |
| 2B | 6 fix wire | 1-2 ngày | ✅ |
| 4 | 10 task handler | 2-3 ngày | một phần |
| 5 | 8 task test | 1 ngày | ❌ |
| 6 | 5 task mở rộng | 3-5 ngày | một phần |

**Lộ trình tối thiểu đóng P0:** Phase 1 + 2A + 3 ⇒ **2-4 ngày**, không cần capture.
**Lộ trình đóng hết P0+P1:** + Phase 0 + 2B ⇒ **5-8 ngày**.

---

## 8. Definition of Done

| # | Tiêu chí |
|:-:|---|
| 1 | `17 0B` body luôn `(len - 2) % 10 == 0` |
| 2 | `17 05` / `1E 01` body luôn `(len - 2) % 12 == 0` |
| 3 | `grep -rn "1u8, &session.homdo" src/` → 0 |
| 4 | `grep -rn "build_stat_update(0x" src/` → chỉ trong `prop_code` |
| 5 | save → load → so sánh: 8 instance field × 5 container khớp 100% |
| 6 | Sau relog: `loai` khớp `Item.dat`; `int2..agi2` khớp `character_sheet` |
| 7 | `handle_move_stack` → disconnect → reconnect ⇒ giữ layout |
| 8 | `cargo test --all-targets --no-fail-fast` pass |
| 9 | `01-container-mapping.md` cập nhật theo kết quả capture |
| 10 | `AGENTS.md` + `CONTEXT.md` ghi chú feature flag mới |

---

## 9. Cập nhật tài liệu sau mỗi phase

| Sau phase | Cập nhật |
|---|---|
| 0 | `01-container-mapping.md` §2.4, §5 (điền body mới); `02-gap-analysis.md` đóng gap |
| 1 | `04-db-persistence.md` §5 (đánh dấu hoàn thành) |
| 2A | `02-gap-analysis.md` §2, §8 đóng G-01/G-02/G-08; `03-wire-codecs.md` §2,4 hoàn thành |
| 2B | `03-wire-codecs.md` §6,7 hoàn thành |
| 3 | `02-gap-analysis.md` §4,5,10 đóng G-03/G-04/G-05/G-10 |
| 4 | `02-gap-analysis.md` §12 đóng G-13..G-20 |
| 5 | `06-tests-and-capture.md` §2 bảng test đầy đủ |

⚠️ **Quy tắc:** không đánh dấu gap "đã đóng" chỉ vì code đã sửa — phải có **test** hoặc
**capture** làm bằng chứng.