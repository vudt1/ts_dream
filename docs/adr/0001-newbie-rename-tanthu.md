# ADR 0001 — Rename `tanthu` → `newbie` và bổ sung cột `characters.newbie`

- **Ngày**: 2026-08-27
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (grilling Q1–Q5)

## Bối cảnh

- `migrations/0001_init.sql:71` định nghĩa `characters` không có cột `tanthu`/`newbie`, trong khi toàn bộ runtime (`src/db/players.rs:312`, `src/db/item_code.rs:116`, `src/db/persist.rs:49`, `src/db/modern/mysql/session.rs:25`, `src/server/session.rs:198`) đọc/ghi `tanthu`.
- DB chưa từng được tạo (owner xác nhận), nên drift chưa gây lỗi production nhưng sẽ fail ngay lần `SELECT ... newbie` đầu tiên sau khi tạo DB.
- Thuật ngữ `tanthu` là tiếng Việt không dấu của "tân thủ" (newbie), không nhất quán với Ubiquitous Language tiếng Anh của codebase và gây nhầm với các counter khác.

## Quyết định

1. **Edit thẳng `0001_init.sql`** (thay vì tạo `0004_add_newbie`): thêm `newbie BIGINT NOT NULL DEFAULT 0` sau `map_y` (`migrations/0001_init.sql:95`). Lý do: chưa có DB nào chạy migration, edit giữ lịch sử gọn. Nếu repo đã share cho máy đã migrate, phải đổi sang migration mới — được ghi rõ trong grilling.
2. **Rename toàn codebase `tanthu` → `newbie`**: `src/db/item_code.rs:30` (`RedeemOutcome::Granted { newbie }`), `src/db/persist.rs:49` (`"newbie" => "newbie"`), `src/db/players.rs:312` (giữ để deprecated path đồng nhất, dù bảng `players` sẽ bỏ), `src/db/modern/mysql/session.rs:25,79,261`, `src/server/session.rs:198`, `src/server/handlers/system.rs:285`, `src/server/handlers/use_item/books.rs:154`.
3. **Cập nhật `CONTEXT.md`**: thêm glossary `Tân thủ / Newbie` với invariant `0=chưa nhận, 1=đã nhận TSVN, >1=tích lũy sách 46238`.
4. **Defer drift còn lại** (Q5=a): không gộp fix `characters`/`inventories`/`character_pets`/`character_missions` drift trong ticket này. Xuất báo cáo drift làm follow-up ticket.

## Hậu quả

- Mọi `SELECT newbie FROM characters` và `UPDATE characters SET newbie` khớp migration (`src/db/item_code.rs:117,142`).
- Naming thống nhất, tránh phải alias `tanthu AS newbie`.
- Drift lớn vẫn tồn tại — được liệt kê ở phần dưới, xử lý ở ticket riêng để không phình scope.

## Drift còn lại (deferred)

**`characters` thiếu** (`src/db/modern/mysql/session.rs:25`): `int2, atk2, def2, hpx2, spx2, agi2, texp, god, tiengtam, gocnhin, pk, tham_chien, hp_store, sp_store, savemap, color, fight_npc_id, title_id`.

**`inventories` thiếu** (`src/db/modern/mysql/session.rs:137`, `src/db/persist.rs:203`): `item_level, int1, atk1, def1, hpx1, spx1, agi1, fai1, int2, atk2, def2, hpx2, spx2, agi2, fai2, item_hp, item_sp, item_type, item_element, item_element_value` (migration hiện định nghĩa `damage/element/element_value/proof_kind/grow_level/...` sai tên).

**`character_pets` thiếu**: `int2, atk2, def2, hpx2, spx2, agi2`.

**`character_missions` thiếu**: `npc_id, warp_id`.

**Bảng legacy không tồn tại trong migration nhưng code query**: `players, pet, skill, skillsave, quest, homdo, trangbi` (`src/db/players.rs:3`, `src/db/quest.rs`) — Q4 quyết định bỏ, chỉ giữ tables trong `0001`+`0003`.

## Cleanup 2026-08-28 — Round 1 (Q5=a follow-up)

Grilling Q1–Q5 chốt scope:
- Q1: scope = **toàn bộ deferred list (A)**.
- Q2: **xóa Rust, KHÔNG sửa migration (B)**.
- Q3: chấp nhận mất toàn bộ runtime, **giữ `newbie`** vì cần cho TSVN gift guard.
- Q4: sách 46238 **chỉ giữ `newbie`**, bỏ Spx2/SpMax/stat frame 0xD0.
- Q5: **xóa quest persist + talk menu 33 savemap persist** (in-memory only, restart mất).

Đã xóa:
- `src/db/players.rs` (orphan, 0 caller ngoài self-ref) — file deleted.
- `src/db/quest.rs` (orphan chain) — file deleted.
- `src/db/mod.rs::pub mod players/quest` — module declarations removed.
- 16 mapping deferred trong `src/db/persist.rs::character_column` (giữ `"newbie" => "newbie"`).
- 20 cột ThingData khỏi `persist::upsert_inventory_tx` (chỉ giữ `item_id, quantity, damage`).
- `int2/atk2/def2/hpx2/spx2/agi2` khỏi `persist::upsert_pet` INSERT.
- 18 cột deferred khỏi `session.rs::load()` SELECT + assignments (giữ `newbie`).
- 20 cột ThingData khỏi `session.rs::load_items()`.
- `int2/atk2/.../agi2` khỏi `session.rs::load_pets()`.
- Tất cả cột deferred khỏi `session.rs::save()` UPDATE/INSERT (giữ `newbie`).
- `character_missions` SELECT/INSERT (vì thiếu `npc_id/warp_id`).
- `update_player("Pk"/"ThamChien")` trong `system.rs:39,47` (in-memory only).
- `update_player("Texp")` × 5 sites: `books.rs:136`, `reborn.rs:41,80`, `inventory.rs:489`.
- `update_player("God")` trong `books.rs:125`.
- `update_player("HP_Store"/"SP_Store")` trong `books.rs:200-218`.
- `update_player("Spx2"/"Hpx2"/"Int2"/"HpMax"/"SpMax")` trong `misc.rs:136-140`.
- `update_player("savemap")` trong `talk.rs:212,244` × 2.
- Sách 46238: chỉ giữ `newbie += 1` + persist; bỏ `spx2/sp_max` increment + stat frame 0xD0.
- `Session.quest_steps`/`Session.warp_steps` (giờ là in-memory only — restart mất).

Cập nhật `CONTEXT.md`:
- Entry `Tân thủ/Newbie`: invariant đơn giản hóa còn `0/1` (bỏ `>1` tích lũy sách).
- Entry `Reborn/Rebirth`: bỏ `Texp=13` khỏi invariants, đổi thành "Texp reset in-memory".
- Entry `ThingData`: thêm "Phạm vi persist" ghi rõ 3/20 cột còn persist.
- Entry mới `Deferred Drift (Đã Cleanup Sau ADR 0001)`: liệt kê field còn in-memory, restart mất gì.

## Tham chiếu

- Grilling session 2026-08-27 Q1=sửa 0001, Q2=NOT NULL DEFAULT 0, Q3=toàn codebase, Q4=bỏ legacy players, Q5=defer.
- Grilling session 2026-08-28 Q1=scope A, Q2=hướng B (xóa Rust, không sửa migration), Q3=giữ newbie, Q4=sách 46238 chỉ giữ newbie, Q5=xóa quest persist + talk menu 33 savemap.
- Files: `migrations/0001_init.sql:95`, `CONTEXT.md` (Tân thủ/Newbie + Deferred Drift), `src/db/modern/mysql/session.rs:25`
