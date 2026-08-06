# 03 — Static data load + bảng mã text encoding

**What to build:** Load toàn bộ dữ liệu tĩnh từ `ts_server_old/Data/` byte-identical theo quy ước từng file, trả về các bảng in-memory (`Data_Npcs`, `Data_Items`, `Data_Skills`, `Data_Warps`, `Data_Talks`, `Texps`, `Data_BattleGates`, `NpcOnMap`, `ItemOnMap`, `Data_Dolls`); triển khai đầy đủ text encoding VISCII/mojibake/garble. Sau ticket này `DataLoaded=true`.

**Blocked by:** 01 — Scaffold dự án + config + startup sequence; 02 — Protocol framing + encoders + VISCII wire path (cần quy ước byte/hex giống nhau giữa load và wire).

**Status:** ✅ **done** (implemented 2026-08-05; full suite green: 224 lib + 17 integration). Cập nhật tiếp 2026-08-06: **227 lib + 22 integration = 249 passed**, thêm bundle `Data/` + `build.rs` packaging + rename encoder (xem mục "Cập nhật 2026-08-06").

- [x] Đọc đúng convention từng loại file: BOM/encoding (Npcs UTF-16LE+BOM LF; Items UTF-8 CRLF; Skills UTF-8; còn lại ASCII), `Split('\t')`, skip dòng `//`, **termination rule** on empty line (`Warps`/`BattleGate` dừng ở `text.Length < 5`), non-numeric field trong cột numeric → load failure như C#, không default cho cột thiếu.
- [x] Nạp 8+ bảng với đúng so đồ cột Ch3 §3.2/§3.3 và `Texps` computed (0.35/2.9/3.0/3.05, MaxLevel 200) từ §3.5; `dictionary_0/1` rỗng như C# (chỉ mô phỏng absence). `Data_Client/*.DAT` `EVe.txt`/`shopp.accdb` không load.
- [x] `ItemOnMap` tạo `ItemDropOnMap` slots 1..255, spawn static drop `_Delay=999999` + broadcast `F44408001703`.
- [x] `Quests/*.ini`: Win32 INI semantics bắt buộc (absent key → sentinel `"nothing"`; case-insensitive; cap 1024 chars; `[Lose]` WarpTo đọc từ ONWIN — bug cần tái lập; `Dialogs=` hex forward verbatim split trên `"F444"`).
- [x] Bảng mã VISCII byte→Unicode (thêm `0xD0→Đ`, `0xDD→Đ` từ bảng), reverse mojibake map, Unicode→VISCII positional `viscii_encode` (trước đây `smethod_17`, rename 2026-08-06; Ch4 §4.4) — import từ research 03 character-for-character.
- [x] Garble bug-for-bug: **122 tên** (CP1252 codepoint >0xFF) replicate byte-exact — nhóm 4 hex → 2 garbage bytes; nhóm 3 hex → abort cả packet. Trong đó **3 tên abort**: item 48101 (`ă` U+0103→`103`), item 62712 (`œ` U+0153→`153`), npc 40119 (`Š` U+0160→`160`); **119 tên còn lại** = override hex 2-garbage (Ch4 §4.3/§4.6).
- [x] Tên trong memory là `Vec<u8>` VISCII; `DataLoaded=true` khi hoàn tất.

## Ghi chú triển khai (review 2026-08-05)

Các vấn đề tìm thấy khi đối chiếu C# (`Data.cs` / `Class5.cs` / `TextEncoder.cs`) đã được sửa, TDD:

1. **Npcs column map** — `_Bat`/`_Reborn` đọc nhầm Drop1/Drop2 (index 16/17 thay vì 22/23); Drop1–6 không load. Đã sửa `src/data/loader.rs` + test real-data (`npcs_parse_drop_bat_reborn_columns`).
2. **Texps** — loop sai: bỏ số hạng i=0 nên mọi ngưỡng lệch −6 và `Texps[0]=0` thay vì 6. Đã sửa `src/data/texps.rs` + test giá trị chính xác (`texps_exact_values_match_csharp`).
3. **ItemDropOnMap** — trước đây chỉ lưu `Vec<ItemOnMap>`. Đã thêm `item_drop_on_map: HashMap<(map,slot), ItemDropOnMap>` (prefill 1..255/map, spawn `_Delay=999999` kèm stats), `static_drop_frame()` cho `F44408001703`, và `seed_static_drops()` trong `main.rs` đăng ký vào runtime `map_drops` để nhặt được.
4. **Garble** — quyết định theo spec §4.3/§4.6 (user chọn): replicate byte-exact. Thêm `GarbleSpec` + `compute_garble()` (tái tạo `AscW.ToString("X2")`: 4-hex→2 garbage bytes, 3-hex→abort) trên `Npc`/`Item`, helper `wire_name_hex()`. **Sửa số liệu 2026-08-06:** abort theo số chữ số hex (không phải theo ID) nên có **3 tên abort**, không phải 1 — item 48101 (`ă` U+0103→`103`), item 62712 (`œ` U+0153→`153`), npc 40119 (`Š` U+0160→`160`); 119 tên còn lại override hex ở send-time. Prose ticket trước đây ghi "Item 48101 abort; 121 còn lại" là **chưa chính xác** — code khớp §4.6 chuẩn. Các send-site tên item/npc (on-map item list, NPC appear) thuộc ticket 11/18 sẽ gọi `wire_name_hex()`.
5. **Bảng VISCII byte→Unicode** — import đủ 102 entry từ `TextEncoder.cs:15-42` + `0xD0/0xDD→Đ` (`viscii_to_unicode`).
6. **viscii_encode** (trước đây `smethod_17`) — import bảng positional `uni`/`enc` (132 char, xác minh byte-exact từ `Class5.cs:422-423`); wire vào banner/announce/`server_name_frame`/`red_message` (Đ→0xD0 thay vì `?`).
7. **Warps skip rule** — bỏ qua dòng cột map2 rỗng (`array2[2].Length <= 0`).
8. **No-defaults contract** — `num_or_default` (default 0) → `num_at` strict (thiếu/trống cột numeric = load failure như `Conversions.ToInteger`); bỏ các guard `f.len()` cho phép skip.
9. **Quest `[REQUIRES]`** — thêm `Level/Reborn` (value+opIndex), `Thuoctinh`, `Quests`, `Wears`, `[DESCRIPTION] Title`; sửa `AddSkill` (C# flat `int[]{skillId,level}`, trước đây ra 2 cặp).
10. **Quest bugs** — `[OnLose]` WarpTo luôn đọc ONWIN (bỏ điều kiện `is_empty`); `[TEAMDEF]` Diahinh absent→0, Npcs enforce đúng 10 (≠10 → zeros).
11. **TexpGetLvUp** — giữ nguyên clamp `reborn.min(2)` (không tồn tại reborn>2); chỉ sửa bảng gốc.

**Còn lại cho ticket sau:** `TexpGetLvUp` sử dụng bảng đã sửa; `wire_name_hex()` cần được gọi ở các send-site tên item/npc (ticket 11/18); `_RemoveItemOnMap`/`ItemOnMapShow`/`NpcOnMapWalk` là runtime loop của ticket 07/11; `_DescTitle` mới lưu (dùng ở ticket 19).

## Cập nhật 2026-08-06

1. **Rename `smethod_17` → `viscii_encode`** (`src/encoding.rs`) — tên cũ khó hiểu (giữ comment provenance `Class5.cs:420-462`). Cập nhật 3 call-site: `src/server/spawn.rs` (banner/announce + `server_name_frame`), `src/server/handlers/shops.rs` (`red_message`), và các test. Tên hàm tên miền rõ nghĩa hơn; không xung đột với `to_viscii` (reverse mojibake map) vốn đã tồn tại.
2. **Bundle `Data/` vào project** — copy toàn bộ `ts_server_old/Data/` → `Data/` (byte-identical, `diff -rq` sạch): 8 bảng `.txt` + 813 file `Quests/*.ini` + file tham khảo (`EVe.txt`, `shopp.accdb`, `packet.txt`, `Member.ini`, `shopp_schema.sql`). Server TCP nạp từ thư mục này:
   - `Config::resolve_data_dir()` (`src/config.rs`) — ưu tiên (1) đường dẫn cấu hình `./Data` (CWD), (2) bundle kề executable, (3) fallback giữ nguyên. TDD: 3 test mới.
   - `main.rs` dùng `cfg.resolve_data_dir()` cho `GameData::load` + log `data_dir`; `DataLoaded` gate giữ nguyên.
3. **`build.rs` packaging** — khi `cargo build`, copy `Data/` sang `target/<profile>/Data` (suy ra profile dir từ `OUT_DIR` parent×3, kèm `cargo:rerun-if-changed=Data`) để binary đi kèm dữ liệu khi phân phối. Xác minh: `target/debug/Data` đầy đủ 813 quest.
4. **Tests** — `tests/data.rs` giờ nạp thẳng `Data/` bundled (default `DATA_DIR = "Data"`, vẫn cho phép `TS_TEST_DATA_DIR` override): 13 test real-data xanh.
5. **Số liệu test tổng** — 227 lib + 22 integration (13 data, 4 web, 3 golden, 2 golden_suite, 1 battle_golden) = **249 passed / 0 failed**.
