# 11 — Inventory base (op 0x17 subs 2/3/10/11/12)

**What to build:** Các thao tác cốt lõi với túi đồ và trang bị: nhặt vật rơi trên map (distance gate), ném bỏ vật phẩm, di chuyển/xếp stack (echo raw packet), trang bị/giáp bó. Cập nhật dumps inventory (1705/170B) đúng. Nền cho mọi thao tác item khác (ticket 12, 13, 14, 15).

**Blocked By:** 04 — DB; 07 — Login + spawn (có inventory dumps để thao tác).

**Status:** completed

- [x] Sub 2 — pick up map drop: distance gate (±150 map units), đọc drop từ map registry (`server::map_drops`, mirror `Data.ItemDropOnMap`), `HomdoAddItem` + frames `1702`/`1706` + dump.
- [x] Sub 3 — drop item (KHỞI: per-map free slot allocator, C# `HomdoDropItem` scans 1..255; `map_drops::allocate` `map_drops.rs:43`; refused→item kept, silent).
- [x] Sub 10 — move/stack: 3-byte payload (oldslot, count, newslot), thiết bị chỉ move vào slot trống, stackable merge (dst<50, total≤50), on success echo **toàn bộ raw packet** (Ch2 §2.3.14; `inventory.rs:182`).
- [x] Sub 11/12 — equip / unequip: chuyển `trangbi` ↔ `homdo`; **level gate** (player.Lv ≥ item.Lv) khi equip; **empty-slot gate** tại homdo slot đích khi không đồ; dumps `170B`, recompute stat + emission `0801` (Hp/Sp max + các `2`-stat D4/D2/D3/CF/D0/D6); broadcast `ServerSend_Equip/UnEquipItem` (`08000502`/`0501`).
- [x] Inventory dumps: `1705` (homdo), `170B` (trangbi) đúng field (Ch2 §2.4 / op 0x17).
- [x] Thao tác gắn `player_id` + DB write-through (`persist::upsert_item`) trên item tables (homdo/trangbi).
- [x] Golden: move/equip/stack scenario (`tests/golden_suite.rs` replay byte-exact — passing).

---

## Verification & fixes implemented (post-verification, with file:line)

Verification đối chiếu ticket ↔ spec ↔ `ts_server_old` C# ↔ Rust `src/`. Ticket ghi `completed` nhưng **chuỗi gear-stat thực chất là no-op** (item không mang chỉ số `_1`/`_2`/nguyên tố khi chạy) và 2 notes/deferred chưa giải. Đã mở lại và sửa như sau.

### B. Wire/stack fixes (sub 2/3/10) — `src/server/handlers/inventory.rs`
| Fix | Rust | C# reference |
|---|---|---|
| Pickup ack `1702` dùng slot **2-byte LE** (trước là 1-byte) + thêm broadcast xóa `04001702`, **bỏ dump `1705` thừa**, full-bag im lặng giữ drop | `handle_pickup` `inventory.rs:60-108` (fames 98, 103) | `PickupItemOnMap` Data.cs:3798-3800 / 3846-3865 |
| Drop dùng per-map free slot allocator (1..255), item trong map mang đúng count dropped | `handle_drop` `inventory.rs:110-141` (`map_drops::allocate`): | `HomdoDropItem` Data.cs:3511-3562; slot full→im lặng (`num3 > 255` return) |
| Stack cap 50 cho `add_item` (merge vào slot `count<50`, remainder→slot mới) | `server/inventory.rs:33` (STACK_CAP `:13`) | `HomdoAddItem` Data.cs:3291-3305 `HomdoMoveItem` 3600 |
| `add_item` giờ trả **mọi slot bị sửa** (capped merge có thể straddle 2 slot) để caller persist đủ, tránh mất increment trên reload | `server/inventory.rs:33`; `Session::add_homdo_item` `session.rs:420`; `handle_pickup` persist mọi slot `inventory.rs:92-101` | — |
| Move/stack `handle_move_stack` payload `[0]`=old `[1]`=count `[2]`=new; 3-byte; echo raw | `inventory.rs:182` | `HomdoMoveItem` Data.cs:3574 |

### C. Gear-stats (`_2` + nguyên tố) — removal of deferred note #1
- **Struct**: `InventoryItem` thêm `int2..agi2/fai2` + `giatri_thuoctinh`, `thuoctinh/long_val` đã có → `session.rs:10-40`.
- **GearCalculation**: `GearBonuses::from_gear(trangbi, player_element)` sum `_1+_2` + bonus nguyên tố theo hệt C# (mỗi field nonzero +bonus, element `== player || ==5`, cả `Long` kênh) → `character_sheet.rs:31-70`; `CharacterDisplay::recompute` thêm param `player_element` `:80`; `Session::recompute_stats` truyền `self.thuoctinh` `session.rs:318`.
- **DB round-trip**: `load_items` SELECT đủ cột (Int1..Agi2, Fai1/Fai2, Long, GiatriLong, Khang, Thuoctinh, GiatriThuoctinh, Loai) qua `ItemRow` `db/players.rs:392-440`; `upsert_item` viết đủ cột `db/persist.rs:111-170` (schema đã có sẵn các cột, `0001_init.sql`).
- **Seam**: `InventoryItem::from_template(&Data.Item, count)` `session.rs:49`; wrapper `inventory::from_template(data,id,count)` `server/inventory.rs:18`; route khởi tạo qua show: talk savemap items `talk.rs:159/180`, quest rewards `quest.rs:157/166`, NPC shop / free-bundle `shops.rs:198/208`. Static seed bổ sung `_2` `main.rs:41-56`. seam vẫn còn rải: battle/service, quest `onwin` 737, tution) — ghi chú bên dưới.
- Tests elemental: `character_sheet.rs` `gear_bonuses_include_elemental_2_stats`, `gear_element_bonus_applies_per_nonzero_nonzero_and_element_5`.

### D. Map-drop slot (deferred note #2 → đã làm trong #11)
C# cấp slot miễn phí 1..255 theo map (`SystemDropSlot`/`HomdoDropItem` only server-side, wire không mang slot — client liên hệ theo x,y rồi gửi `packet[6]`). Rust trước dùng thẳng homdo slot → 2 player cùng homodo slot trên cùng map ghi đè nhau. Fix: `map_drops::allocate` `map_drops.rs:43` quét 1..255; full-slot → im lặng không drop; static-seed (ItemOnMap) vẫn drop theo slot explicit từ dữ liệu (`ItemDropOnMap` slot đã 1..255 mỗi map). Gen cần: .NET def đã sorted.

---

**Notes / deferred (sau fix):**
- Sun mềm: trắc của player-creation starter items (`db::players::starter_rows`) vẫn mang chỉ `agi1`/`loai` (shield trangbi 19737). Gear/equip của nhân vật mới sẽ ra 0 cho chỉ số đó trừ khi khơi mạch `from_template` vào chỗ khởi tạo mới (`character.rs`/`db::players::create`). Ghi là follow-up.
- Sun mềm: item từ `use_item` (tắm/lệnh), `battle/service` reward, `shops` bán/petup (push copy from homeli — giữ stats OK), `quest::on_win` đang direct-inline. Nếu rẽ đường này vào `from_template` thì nhất quán toàn diện (follow-up tick).
- Map-drop slot allocation đã làm theo mô hình per-map 1..255 đơn lẻ (mMutex registry toàn server); việc mở rộng multi-world-state / phân bổ theo instance có thể cần ở world-state ticket như đã ghi — khởi hiện tại đủ cho nhiều map, nhiều player.