# 06 — Chiến Lược Test & Capture

> Đặc tả cho Phase 0 và Phase 5.
> Tuân thủ `AGENTS.md`: test **chỉ** ở `tests/`, **cấm** `#[cfg(test)]` trong `src/`;
> dùng in-memory SQLite `sqlite::memory:?cache=shared`; chạy
> `cargo test --all-targets --no-fail-fast`.

---

## 1. Phân tầng test

| Tầng | Mục đích | Loại DB | Vị trí |
|---|---|---|---|
| **T1 Unit** | Codec byte-exact, metadata thuần tuý | không | `tests/inventory_codec_test.rs` |
| **T2 Integration** | Round-trip DB, hydrate, persist | in-memory | `tests/inventory_*.rs` |
| **T3 Handler flow** | Frame đầu ra đúng chuỗi | in-memory | `tests/login_char_flow_test.rs` (mở rộng) |
| **T4 Golden** | Byte-for-byte so với capture đã duyệt | không | `golden/` + `harness.rs` |
| **T5 Capture** | Xác minh với `aLogin.exe` thật | DB test riêng | thủ công, có checklist |

---

## 2. Bảng test chi tiết

### T1 — Unit: codec byte-exact

`tests/inventory_codec_test.rs`

| # | Test | Assert |
|:-:|---|---|
| 1-1 | `encode_entry_12` đúng stride | output dài 24 hex char (12 byte) cho 1 item |
| 1-2 | `encode_entry_12` field order | byte `[0]`=slot, `[1..2]`=le16(id), `[3]`=count, `[4]`=doben, `[5]`=long_val, `[6]`=giatri+100, `[7]`=khang, `[8..11]`=le32(texp) |
| 1-3 | `encode_entry_10` đúng stride | output dài 20 hex char (10 byte) |
| 1-4 | `encode_entry_10` **không** có byte slot | byte `[0..1]` = le16(id) |
| 1-5 | `dump_container` chọn layout theo `StorageType` | `Equip` → 10B; 4 container kia → 12B |
| 1-6 | `dump_container(Equip)` sort theo slot | truyền vào slot đảo thứ tự ⇒ output tăng dần |
| 1-7 | `dump_container` bỏ qua `id == 0` | item rỗng không sinh record |
| 1-8 | `biased_giatri_long` | `0`→`100`, `55`→`155`, `155`→`255` |
| 1-9 | Frame rỗng | `dump_container(&[], Bag)` == `F44402001705` |
| 1-10 | `len` field đúng | `(len_hex - 4) == 2 + 12*n` hoặc `2 + 10*n` |

⚠️ **Test 1-10 là chốt chặn quan trọng nhất** — nó bắt được ngay lỗi `17 0B` stride.

```rust
#[test]
fn equip_frame_stride_is_ten() {
    let s = Session::default();
    s.trangbi.push(InventoryItem { slot: 1, id: 19737, count: 1, ..Default::default() });
    s.trangbi.push(InventoryItem { slot: 2, id: 19738, count: 1, ..Default::default() });
    let frame = s.dump_trangbi();
    let body_len = decode_frame_len(&frame);   // đọc len LE16 ở offset 2
    assert_eq!((body_len - 2) % 10, 0, "17 0B phải chia hết cho 10");
    assert_eq!((body_len - 2) / 10, 2);
}

#[test]
fn bag_frame_stride_is_twelve() {
    // tương tự với dump_homdo, kỳ vọng % 12 == 0
}
```

### T1b — Unit: `StorageType` metadata

`tests/inventory_storage_type_test.rs`

| # | Test | Assert |
|:-:|---|---|
| 1b-1 | `value()` khớp schema | 1/2/4/8/16 (giữ nguyên `tests/db_repository_init_test.rs:816-825`) |
| 1b-2 | `dump_code()` | `1705`/`1E01`/`172F`/`170B`/`1766` |
| 1b-3 | `capacity()` | 25/50/25/6/50 |
| 1b-4 | `entry_layout()` | `Equip`=`Slotless10`, còn lại=`SlotLed12` |
| 1b-5 | `table_name()` ↔ `from_table_name()` round-trip | 5/5 |
| 1b-6 | `from_value` từ chối giá trị lạ | `0`, `3`, `32` → `None` |

⚠️ **1b-2/1b-3/1b-4 là test chốt bảng `01-container-mapping.md`.** Khi có capture và đổi
`17 66` → `1E 09`, test này **phải đổi theo**.

### T2 — Integration: round-trip DB

`tests/inventory_roundtrip_test.rs`

| # | Test | Assert |
|:-:|---|---|
| 2-1 | save→load 1 item đầy đủ field | 8 instance field khớp |
| 2-2 | save→load 5 container | cả 5 khớp |
| 2-3 | `storagetype` không lẫn | item `slot 1` ở Bag không xuất hiện ở Bank |
| 2-4 | `count = 0` ⇒ hàng bị xoá | `load_items` filter `itemid > 0` |
| 2-5 | Item ở slot 50 của Bank | load đúng `slot = 50` |
| 2-6 | Item ở slot 6 của Equip | load đúng `slot = 6` |
| 2-7 | `upsert_inventory_tx` không giết cột khác | sau upsert, `element` cũ vẫn còn |

⚠️ **2-7 là test bắt G-04** — hiện tại `save` xoá rồi insert 6 cột nên `element` mất.

**Fixture mẫu:**
```rust
fn sample_item() -> InventoryItem {
    InventoryItem {
        slot: 3,
        id: 19737,
        count: 7,
        doben: 42,
        long_val: 3,          // Hỏa
        giatri_long: 12,
        khang: 5,
        texp: 123_456,
        ..Default::default()
    }
}
```

### T2b — Integration: template hydrate

`tests/inventory_template_hydrate_test.rs`

| # | Test | Assert |
|:-:|---|---|
| 2b-1 | `hydrate_templates` lấy `loai` từ `Item.dat` | khớp `data.items[id].loai` |
| 2b-2 | `hydrate_templates` lấy `lv` | khớp `Item.level` |
| 2b-3 | `hydrate_templates` lấy `int1..fai2` | khớp `Item` |
| 2b-4 | **Không** ghi đè instance field | `slot`/`count`/`doben`/`texp` giữ nguyên sau hydrate |
| 2b-5 | `hydrate_templates` gọi `recompute_stats` | `int2..agi2` khớp `character_sheet` |
| 2b-6 | Item không có trong `Item.dat` | không panic, giữ nguyên |

⚠️ **2b-4 là test chốt quan trọng** — bảo đảm không làm mất instance data.

### T2c — Integration: gear bonus + equip gate

`tests/inventory_gear_bonus_test.rs`

| # | Test | Assert |
|:-:|---|---|
| 2c-1 | Sau hydrate, `loai ∈ 1..=6` | equip gate pass |
| 2c-2 | `hp_max` bao gồm gear bonus | `hp_max == get_hp_max(...) + gear.hpx2` |
| 2c-3 | Element bonus đúng hệ | item `thuoctinh == player.thuoctinh` ⇒ cộng `giatri_thuoctinh` |
| 2c-4 | Element `5` = trung tính | luôn cộng |

### T3 — Handler flow

Mở rộng `tests/login_char_flow_test.rs`

| # | Test | Assert |
|:-:|---|---|
| 3-1 | Logined1 có đúng số frame | cập nhật sau Phase 2A |
| 3-2 | `1E 08` ngay trước `1E 01` | thứ tự frame |
| 3-3 | `17 0B` frame có stride 10 | `(len-2) % 10 == 0` |
| 3-4 | `17 05` frame có stride 12 | `(len-2) % 12 == 0` |
| 3-5 | Không có `17 66` khi flag off | tìm trong danh sách frame |
| 3-6 | `17 05` đứng trước `1E 08` | thứ tự |

⚠️ `[R]` `tests/login_char_flow_test.rs:234` và `:352` đang assert `21`. Sau Phase 2A
(thêm `1E 08`, bỏ `17 66` mặc định) số element **vẫn 21**. Nếu bật flag ⇒ 22.

> ⚠️ **Phân biệt "element" và "frame thật".** Test đếm `out.outgoing.len()` = số phần tử `Vec`.
> Element thứ 13 là `"F44402000504F44402000F0A"` (`spawn.rs:570`) = **2 frame gộp trong 1 chuỗi**.
> ⇒ `len() == 21` nhưng wire có **22 frame**. Khi đối chiếu capture, phải tách trước khi đếm.

`tests/equip_flow_test.rs` (mới)

| # | Test | Assert |
|:-:|---|---|
| 3c-1 | Equip 1 item | `17 11` + `17 0B` (10B) + burst `08 01` |
| 3c-2 | Burst đủ 12 frame sau 2B-1 | theo `prop_code::*` |
| 3c-3 | Unequip | `17 10` + `17 0B` + burst |
| 3c-4 | Equip rồi disconnect rồi login | trang bị còn, `loai` đúng |

`tests/storage_flow_test.rs` (mới)

| # | Test | Assert |
|:-:|---|---|
| 3s-1 | Rút 1 item từ tiền trang | `17 08` + `1E 05` + `17 32` |
| 3s-2 | Gửi 1 item vào tiền trang | `17 09` + `1E 04` |
| 3s-3 | Sức chứa đúng 50 ô | ô 51 bị từ chối |
| 3s-4 | Rút xong disconnect rồi login | item còn đúng chỗ |

### T4 — Golden

`[R]` `golden/` có 18 packet; `harness.rs` hỗ trợ diff.

| # | Hành động |
|:-:|---|
| 4-1 | Chụp golden mới **sau khi** có capture `aLogin.exe` thật |
| 4-2 | Golden hiện tại là "golden replay" — **không** phải capture thật. Ghi rõ trong `TESTS.md`. |
| 4-3 | Thêm golden cho: Logined1 đầy đủ, `17 0B` 10B, `17 05` 12B, `1E 01` 12B |

⚠️ **Không** dùng golden hiện tại làm chuẩn layout — nó được sinh từ code Rust đang
**sai** ở `17 0B`. Phải tạo lại sau khi có capture.

### T5 — Capture thật (checklist)

Xem §3 và §4 bên dưới.

---

## 3. Capture: `05 03` (ĐÃ ĐÓNG G-07 — Xác minh 100% qua decompile `aLogin.exe`)

> ✅ **Trạng thái: ĐÃ ĐÓNG (Resolved 100%)** — Không cần capture động nữa.
> Phân tích dịch ngược binary client `aLogin.exe` tại `FUN_0072E9A0` (@`0x0072E9A0`, Case 6 `FUN_0078ca56` SubOp 3) đã giải mã chính xác cấu trúc wire.

### Bằng chứng từ decompile `aLogin.exe` (`FUN_0072E9A0`)

1. **Header cố định**: Đúng 112 bytes `RestPayload` (từ byte 1 đến byte 112, tức index `0x01..0x70`).
   - Cụm `F401` × 5 (5 × u16 = 500) nằm tại offset `0x3C..0x45` (bytes 60..69).
   - Vùng padding phía sau (`0x46..0x70`, bytes 70..112) gồm các thuộc tính mở rộng, dài **đúng 43 bytes 0x00** (không phải 90 bytes).
2. **Danh sách kỹ năng (Skills list)**: Bắt đầu tại byte 113 (`0x71`). Client tính:
   $$\text{Count} = \frac{\text{Length}(\text{RestPayload}) - 112}{3}$$
   Và lặp đọc `[skill_id: Word LE][skill_level: Byte]` cho đến hết buffer.
3. **Đuôi packet (Tail)**: **KHÔNG CÓ block 32 byte `_3`/`_4`**. Client thoát vòng lặp là gọi UI refresh rồi return ngay.
4. **Hậu quả của 90 zero cũ trong Rust**: Dư 47 bytes zero $\implies$ client tính ra 15 skill rỗng (45 bytes) và dư 2 bytes ($47 \pmod 3 = 2$). 2 byte dư này làm lệch pha dòng byte của toàn bộ skill thật phía sau.

### Hành động đã thực hiện

- Sửa `src/server/spawn.rs`: Đổi `&"00".repeat(90)` $\to$ `&"00".repeat(43)`.
- Bổ sung unit test: `test_stats_frame_layout_and_g07_tail` trong `tests/login_char_flow_test.rs` kiểm tra độ dài 113B (payload không skill) và stride 3B/skill. G-07 đã đóng 100%.

---

## 4. Capture: `17 2F` / `17 66` (đóng G-02)

### Cách thu

1. Nhân vật có item trong túi đeo (`storagetype 4`) và lưu lang (`storagetype 16`)
2. Login, chụp frame `17 2F` và `17 66`
3. **Quan sát UI client**: túi đeo hiển thị đúng không? lưu lang có mở được không?

### Phân tích

| Bước | Việc |
|:-:|---|
| A | Frame `17 2F` dài bao nhiêu? `(len-2) % 12 == 0`? |
| B | Byte đầu có phải slot không? Giá trị có trong `1..=25` không? |
| C | Frame `17 66` dài bao nhiêu? Có chia hết cho 12 / 10 không? |
| D | UI hiển thị vật phẩm ở đúng ô không? |
| E | Nếu `17 66` không hiển thị gì ⇒ có thể client cần frame **mở form** trước |

⚠️ **Kịch bản quan trọng:** có thể `17 66` là frame **delta** (1 record) chứ không phải dump
(giống `17 30`/`17 08`), tức cần một frame khác để mở form lưu lãng trước.
Kiểm tra: Bear có frame nào mở lưu lãng không? → `grep` không có ⇒ khả năng cao client dùng
chung form với `1E`.

⇒ Nếu capture cho thấy `17 66` sai ⇒ chuyển sang phương án `1E 09` (xem `05-phased-roadmap.md`
§3.2 trong `03-wire-codecs.md`).

---

## 5. Capture: `08 01` prop code (đóng G-06)

### Cách thu

1. Đeo trang bị có bonus biết rõ (ví dụ `19737` `agi1 = 1`)
2. Chụp frame `08 01` sau equip
3. Ghi lại **giá trị** của từng prop code

### Bảng điền

| Prop code | Giá trị server gửi | Bear quy là | Client hiển thị ở | Khớp? |
|:-:|---|---|---|:-:|
| `0x19` | | hp | | |
| `0x1A` | | sp | | |
| `0x1B` | | mag | | |
| `0x1C` | | atk | | |
| `0x1D` | | def | | |
| `0x1E` | | agi | | |
| `0x1F` | | hpx | | |
| `0x20` | | spx | | |
| `0xCF` | | hp2 | | |
| `0xD0` | | sp2 | | |
| `0xD2` | | atk2 | | |
| `0xD3` | | def2 | | |
| `0xD4` | | mag2 | | |
| `0xD6` | | agi2 | | |
| `0xD7` | | FullHpMax | | |
| `0xD8` | | FullSpMax | | |
| `0xDA` | | hpx2 | | |
| `0xDB` | | spx2 | | |

### Cách đọc client field

`[C]` `FUN_0078D125` (case 009) sẽ cho biết mỗi prop code ghi vào offset nào của player
(ví dụ `player+0x1234`). Đối chiếu với `05 03` mapping trong `spawn.rs`.

⚠️ **Đây là capture quan trọng nhất của Phase 0** vì quyết định 3 lỗi G-06.

---

## 6. Capture: `17 20` stall close (đóng G-12)

1. Mở stall (`17 1E`), đóng stall (`17 20`)
2. Chụp frame `17 20`
3. Xác nhận độ dài và vị trí uid

Hiện tại Rust: `F44406001720` + `le32(id)` ⇒ `len = 6`, body 4 byte.
Client `FUN_00731388` đọc uid tại `payload[3..6]` ⇒ cần `len = 7`, body 5 byte.
⇒ **Thiếu 1 byte**. Capture sẽ cho biết byte đó là gì (có thể `stall_id:1` như `17 1E`).

---

## 7. Lệnh chạy test

```bash
# Toàn bộ suite
cargo test --all-targets --no-fail-fast

# Chỉ test inventory
cargo test --test inventory_codec_test --test inventory_roundtrip_test \
           --test inventory_template_hydrate_test --test inventory_gear_bonus_test \
           --test inventory_storage_type_test

# Chỉ flow login / equip / storage
cargo test --test login_char_flow_test --test equip_flow_test --test storage_flow_test

# Với DB test file riêng
TS_TEST_DB_URL=sqlite://DB/ts_dream_test.db cargo test --test inventory_roundtrip_test

# Lint
cargo clippy --all-targets -- --deny warnings
```

---

## 8. Ma trận gap ↔ test

| Gap | Test chứng minh | Loại |
|---|---|---|
| G-01 `17 0B` 12B | `equip_frame_stride_is_ten`, `encode_entry_10` | T1 |
| G-02 `17 66` | **chỉ** capture | T5 |
| G-03 `load_items` 4 cột | `inventory_roundtrip` (2-1, 2-2) | T2 |
| G-04 `save` 6 cột | `inventory_roundtrip` (2-7) | T2 |
| G-05 `recompute` khi load | `inventory_gear_bonus` (2c-2) | T2c |
| G-06 `08 01` codes | **chỉ** capture | T5 |
| G-07 `05 03` tail | `test_stats_frame_layout_and_g07_tail` (decompile `FUN_0072E9A0`) | T1/T3 (ĐÃ ĐÓNG) |
| G-08 `1E 01` clear | `login_char_flow` (3-2) + capture | T3+T5 |
| G-09 sức chứa | `storage_flow` (3s-3) | T3 |
| G-10 move persist | `inventory_move_persist_test` | T2 |
| G-13 `tuideo` | `tuideo_flow_test` | T3 |
| G-14 `pet_equip` | `pet_equip_flow_test` | T3 |
| G-15 nhân bản mapping | `inventory_storage_type` (1b-5) | T1b |
| G-18 field RAM-only | `session_persist_test` | T2 |

⚠️ **2 gap còn lại cần capture**: G-02, G-06. (G-07 đã được đóng 100% qua decompile binary `aLogin.exe` `FUN_0072E9A0` và test suite).
Nếu không có client thật ⇒ ghi "unresolved" trong `02-gap-analysis.md` và dùng phương án an toàn.

---

## 9. File test dự kiến tạo mới

```
tests/
├── inventory_codec_test.rs              (mới, T1)
├── inventory_storage_type_test.rs       (mới, T1b)
├── inventory_roundtrip_test.rs          (mới, T2)
├── inventory_template_hydrate_test.rs  (mới, T2b)
├── inventory_gear_bonus_test.rs         (mới, T2c)
├── inventory_move_persist_test.rs       (mới, T2)
├── equip_flow_test.rs                   (mới, T3)
├── storage_flow_test.rs                 (mới, T3)
├── tuideo_flow_test.rs                  (mới, T3)
├── pet_equip_flow_test.rs               (mới, T3)
├── session_persist_test.rs              (mới, T2)
└── login_char_flow_test.rs              (mở rộng, T3)
```

⚠️ Tất cả import qua `ts_dream::...` public API ⇒ các hàm cần test phải là `pub`:
`Session::hydrate_templates`, `Session::containers`, `StorageType::*`, `EntryLayout`.

⚠️ **[R]** `AGENTS.md` ghi "mọi test mới phải đặt ở `tests/`" và `tests/` hiện đã có 21 file.
Không có `#[cfg(test)]` nào trong `src/` — giữ nguyên nguyên tắc này.