# 10 — Stat allocation (op 0x08) + hotkey/skill bar (op 0x28)

**What to build:** Người chơi phân bổ points vào 8 chỉ số và thấy stat được recompute cập nhật lại trên màn hình client; thiết lập skill bar hotkey theo slot. Nền cho dùng kỹ năng/skill (ticket 13).

**Blocked by:** 04 — DB schema; 07 — Login + spawn (cần player loaded + stat dumps).

**Status:** completed

- [x] Op 0x08 sub 1, gate `Point >= points && points > 0`; stat id → column + recompute (Ch2 §2.3.6):
  - [x] 25 Hpmax (`getHpMax(reborn, job, lv, Hpx+n) + Hpx2`); 26 Spmax; 27 Int; 28 Atk; 29 Def; 30 Agi (cap 400); 31 Hpx (cap 400 + Hpmax recompute); 32 Spx (cap 400).
- [x] Mọi change flow qua `PlayerUpdateDataId` → emit op 0x08 stat packet (type/sign/abs value) đúng định dạng Ch2 §2.4 (`F4440C000801`+type+sign+le32+`00000000`).
- [x] Cập nhật cả DB + in-memory thống nhất (battle trạng thái riêng biệt). DB write-through qua `server::persist::update_player` (`Point/Int/Atk/Def/Agi/Hpx/Spx/Hp/Sp/HpMax/SpMax`). Case 25/26 không trừ point và không tăng Hpx — **faithful** với C# gốc.
- [x] Op 0x28 hotkey: `data[7..8]` skill id LE16, `data[9]` slot 1..10 → `SkillSaveUpdateId(slot, skill)`; **no response** (Ch2 §2.3.25). DB write-through qua `persist::update_skillsave`. Thêm xử lý slot 0 = clear (no-op).
- [x] Skill bar packet `F444`+len+`2801`+`02`+le16(skill)+slot được gửi ở login dump (`dump_hotkeys`); handler giờ là async.

## Références mã nguồn (đối chiếu)

### Rust (port)
| Chức năng | File:line |
|---|---|
| Opcode 0x08 handler `handle_stat_allocation` (parse `payload[2]`=stat id, `payload[3]`=points; gate `pts==0 \|\| point < pts`; match 25..32) | `src/server/handlers/stats.rs:29` |
| Case 25 Hpmax → `build_stat_update(0x19, new_hp)` | `src/server/handlers/stats.rs:53-65` |
| Case 26 Spmax → `build_stat_update(0x1A, new_sp)` | `src/server/handlers/stats.rs:66-78` |
| Case 27 Int → `0x26` Point + `0x1B` Int | `src/server/handlers/stats.rs:79-90` |
| Case 28 Atk → `0x26` + `0x1C` | `src/server/handlers/stats.rs:91-102` |
| Case 29 Def → `0x26` + `0x1D` | `src/server/handlers/stats.rs:103-114` |
| Case 30 Agi → `0x26` + `0x1E` | `src/server/handlers/stats.rs:115-126` |
| Case 31 Hpx → `0x26` Point + `0x1F` Hpx (recompute Hpmax in-memory, no packet) | `src/server/handlers/stats.rs:127-143` |
| Case 32 Spx → `0x26` + `0x20` (recompute Spmax in-memory) | `src/server/handlers/stats.rs:144-160` |
| Stat packet builder `build_stat_update` (`F4440C000801`+type+sign+le32+`00000000`; sign `01`/`02`) | `src/server/handlers/stats.rs:9-23` |
| `get_hp_max` / `get_sp_max` | `src/battle/engine.rs:96` / `src/battle/engine.rs:113-126` |
| `recompute` (gear bonuses + max) | `src/server/character_sheet.rs:48-64` |
| `recompute_stats` (clamp hp/sp) | `src/server/session.rs:274-297` |
| DB write-through `update_player` (whitelist cột: Point/Int/Atk/Def/Agi/Hpx/Spx/Hp/Sp/HpMax/SpMax) | `src/db/persist.rs:44-59` |
| DB write-through `update_skillsave` | `src/db/persist.rs:63-75` |
| Login dump hotkey/skill bar `dump_hotkeys` (`F444`+`len`+`2801`+`02`+le16+slot; empty → `F4440300280102`) | `src/server/session.rs:355-368` |
| Handlers routed trong dispatcher | `src/server/handler.rs:154` (0x08), `src/server/handler.rs:214` (0x28) |
| Login dump build `build_logined_sequence_session` (step 20: `dump_hotkeys`) | `src/server/spawn.rs:327` / `445` |
| Unit tests | `src/server/handlers/stats.rs:190-285` |

### C# nguyên gốc (`ts_server_old/Server_TS_Online/`)

| Chức năng | File:line |
|---|---|
| Opcode 0x08 dispatch `case 8: Update_H8(...)` | `Client.cs:880` |
| Opcode 0x28 dispatch `case 40: Update_H28(...)` | `Client.cs:940` |
| `Update_H8` (gate `_My_Point > 0) & (_My_Point >= num2)`; switch 25..32) | `Client.cs:1064-1141` |
| `Update_H28` (`data[7..8]` skill, `data[9]` slot → `SkillSaveUpdateId`) | `Client.cs:7669-7684` |
| `SkillSaveUpdateId` (UPDATE SkillSave) | `Client.cs:8358-8363` |
| `PlayerUpdateDataId` (emit `F4440C000801`; nhánh `_Hpmax`/`_Spmax` chỉ set in-memory — no packet) | `Data.cs:233`; Hpmax tại `Data.cs:275-278`; Spmax `Data.cs:284-287` |
| `getHpMax` / `getSpMax` | `Data.cs:5537-5539` / `Data.cs:5553` |

**Notes / deferred:** write-through chỉ chạy khi live server có `env.pool` (golden replay in-memory).