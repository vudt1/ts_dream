# 12 — Use item + sách kỹ năng/stat/Texp + potion (op 0x17 sub 15, subset)

**What to build:** Người chơi dùng được vật phẩm tiêu thụ và sách: potion hồi HP/SP, sách tăng stat/skill/Texp/point, warp, add-pet, gold/FAI items, party buff, lucky-box — mỗi use kết thúc bằng frame chuẩn `…0170F` trừ branch đặc biệt. Nền cho chợ (chi phí), cường-hóa, và các happy thao tác item nâng cao.

**Blocked by:** 10 — Stat/skill bar (cần stat column để sách tác động); 11 — Inventory base (cần slot/use path).

**Status:** completed (subset — lucky-box tables để sau)

- [x] Phân nhánh op 0x17 sub 15 theo item id **data-driven** qua `GameData.items` (Ch2 §2.3.14 "use item"):
  - [x] **Warp items** (bảng 46016–46070…→ map/tọa độ như C#): consume + warp (op 0x0C).
  - [x] **Add-pet** (`item.add_pet > 10000`): thêm pet nếu chưa có + còn slot.
  - [x] **Sleep item 46167** (chỉ leader): full-heal + consume.
  - [x] **Skill books** (46230/31/32/33/46): learn → packet `F4440C0008016E01`+le32(lv)+le32(skillid).
  - [x] **Point books** (50010/50011): tăng `Point`/`SkillPoint` (không consume — đúng quirk C#) + `0801 26/25`.
  - [x] **Potion restore `Hp*Sp*Fai1`** (use-type byte 0 = player, 1..4 = pet slot): cap max HP/SP, Fai cap 100.
  - [x] **Party buff** (46092 → `0B0702FF`) và special (46041/46093 → `0B09FF01`).
- [x] End feedback đúng: thường kết thúc `F44404001709`+slot+**count đã dùng (`packet[7]`)**+`F4440200170F` (C# `HomdoUseHPSPFAI`), qua helper `consume`; trừ branch đặc biệt.
- [x] Warp qua item đưa tới map tọa độ đúng (hook op 0x0C warp confirm).
- [x] In-memory + DB (`player_id`) thống nhất (`persist::upsert_item`; potion write `Hp/Sp`).
- [x] Golden: use-item scenario cập nhật (potion 30001 restore hp/sp + used-count) — `10-use-item.golden`.

**Notes / deferred:**
- **Lucky-box** random rewards (99999, 46129, 46627, 46646, 46935/34, 46395–46398, 46920, 46090, …) là bảng id→reward cứng trong C# (rất lớn, phụ thuộc dataset). Chưa port đầy đủ — các id này hiện rơi về nhánh generic (consume + end feedback). Port tiếp cần bảng reward từ `ts_server_old/Server_TS_Online/Client.cs` case 15.
- Texp/god books và party element buff cần thêm item-data hòa vào nhánh generic khi có spec rõ hiệu ứng.