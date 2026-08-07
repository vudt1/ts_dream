# 12 — Use item + sách kỹ năng/stat/Texp + potion (op 0x17 sub 15, subset)

**What to build:** Người chơi dùng được vật phẩm tiêu thụ và sách: potion hồi HP/SP, sách tăng stat/skill/Texp/point, warp, add-pet, gold/FAI items, party buff, lucky-box — mỗi use kết thúc bằng frame chuẩn `…0170F` trừ branch đặc biệt. Nền cho chợ (chi phí), cường-hóa, và các happy thao tác item nâng cao.

**Blocked by:** 10 — Stat/skill bar (cần stat column để sách tác động); 11 — Inventory base (cần slot/use path).

**Status:** **completed** — **full case-15 parity** (grilling 2026-08-07 → /implement 2026-08-07).

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
- **Lucky-box** random rewards qua RNG: **đã port đầy đủ** (`src/server/handlers/use_item/rewards.rs`), `DotNetRandom` injection, 25 box + 14 fixed pack.
- Texp/god books & pet-element/pet-stat/HP-store books: **đã port** (`books.rs`).

**Source map (đối chiếu kiểm tra) — bản cập nhật 2026-08-07:**
- Rust dispatcher + helpers: `src/server/handlers/use_item/mod.rs` (`use_item` entry :210, `use_item_rng` :222, `dispatch` :268, `potion` :402). Gọi từ `src/server/handlers/inventory.rs:375-383`.
- Submodules: `rewards.rs` (lucky boxes), `books.rs` (skill/Texp/god/pet-stat/store books), `misc.rs` (dolls/dice/special/no-op/full-heal), `reborn.rs` (46170/46247-50 + `out.shutdown`).
- Persist: `src/db/persist.rs:15` `player_column` — thêm `HP_Store`/`SP_Store`/`tanthu`. Session thêm `tanthu` (`src/server/session.rs`) + load qua `src/db/players.rs`.
- RNG: `src/battle/rng.rs` `DotNetRandom` (`new(seed)`/`time_seeded()`/`next_max`/`next_range`).
- C# ground truth: `ts_server_old/Server_TS_Online/Client.cs` `case 15` **3801-5361**. `HomdoAddItem` = `Data.cs:3191` (1706 frame); `HomdoUseHPSPFAI` = `Data.cs:3638`; `PlayerUpdateDataId` = `Data.cs:233`; `PetUpdateData` = `Data.cs:2596` (frame `F4440F00080204`); `Status` types = `DataStructure.cs:959-1023`.
- Golden: `golden/10-use-item.golden` (hand-authored, single-consume — giữ nguyên).