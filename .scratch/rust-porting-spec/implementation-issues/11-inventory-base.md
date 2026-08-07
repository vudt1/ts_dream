# 11 — Inventory base (op 0x17 subs 2/3/10/11/12)

**What to build:** Các thao tác cốt lõi với túi đồ và trang bị: nhặt vật rơi trên map (distance gate), ném bỏ vật phẩm, di chuyển/xếp stack (echo raw packet), trang bị/giáp bó. Cập nhật dumps inventory (1705/170B) đúng. Nền cho mọi thao tác item khác (ticket 12, 13, 14, 15).

**Blocked by:** 04 — DB; 07 — Login + spawn (có inventory dumps để thao tác).

**Status:** completed

- [x] Sub 2 — pick up map drop: distance gate (±150 map units), đọc drop từ map registry (`server::map_drops`, mirror `Data.ItemDropOnMap`), `HomdoAddItem` + frames `1702`/`1706` + dump.
- [x] Sub 3 — drop item khỏi túi: validate count, tạo drop trên map tại tọa độ người chơi, frames `1703`/`1709` + broadcast `1703` cho map.
- [x] Sub 10 — move/stack: 3-byte payload (oldslot, count, newslot), thiết bị chỉ move vào slot trống, stackable merge (dst<50, total≤50), on success echo **toàn bộ raw packet** (Ch2 §2.3.14).
- [x] Sub 11/12 — equip / unequip: chuyển `trangbi` ↔ `homdo`; **level gate** (player.Lv ≥ item.Lv) khi equip; **empty-slot gate** tại homdo slot đích khi unequip; dumps `170B`, recompute stat + emission `0801` (Hp/Sp max + các `2`-stat D4/D2/D3/CF/D0/D6); broadcast `ServerSend_Equit/UnEquitItem` (`08000502`/`0501`).
- [x] Inventory dumps: `F444`+len+`1705` (homdo), `170B` (trangbi) đúng field (Ch2 §2.4 / op 0x17).
- [x] Thao tác gắn `player_id` + DB write-through (`persist::upsert_item`) trên homdo/trangbi; in-memory + DB thống nhất.
- [x] Golden: move/equip/stack scenario ghi lại (covered bằng unit tests + golden liên quan).

**Notes / deferred:**
- Gear stat accumulate (`UpdateStatusWhenUseItem`) hiện chỉ tính `int1/atk1/def1/hpx1/spx1/agi1`; chưa gồm `Int2…Agi2` (per-item trường `_2`) và bonus `GiatriThuoctinh/GiatriLong` theo hệ nguyên tố (C# `_Thuoctinh/_Long == _My_Thuoctinh || == 5`). Cần item-data dùng hệ nguyên tố của nhân vật.
- Map-drop slot allocation đa-client: registry keyed `(map_id, slot)` với slot = homdo slot khi drop; chưa phân bổ slot riêng theo map cho nhiều người chơi (C# cấp slot 1..255 theo map). Nền ngữ cảnh world-state sẽ giải quyết sau.