# 13 — Learn/upgrade skills (op 0x1C) + reborn (op 0x17 sub 46 / op 0x2C)

**What to build:** Người chơi mua/học nâng-level kỹ năng (player + pet), đổi nghề (reborn/rebirth), và hồi sinh pet (reborn pet). Kết nối với bảng `skill`/`skillsave` và stat pipeline. Nền cho battle (dùng skill).

**Blocked by:** 10 — Stat skill bar; 11 — Inventory base; 12 — Use item (skill books).

**Status:** done

- [x] Op 0x1C sub 1 player: chuỗi {skill id LE16 + target level}; validate LvMax/Reborn/prereqs/SkillPoint; mỗi success → `F4440C0008016E01`+le32(lv)+le32(skill); cuối → `F4440C0008012501`+le32(count)+`00000000` (Ch2 §2.3.17).
- [x] Op 0x1C sub 2 pet: `data[6]` stt, `[7–8]` skill id, `[9]` level; chỉ upgrade slot đã tồn tại; reply `F4440F00080204`+stt+`6E01`+le32(lv)+le32(skill).
- [x] Mọi write trên bảng `skill`/`skillsave` mang `player_id` (Ch5 §5.4).
- [x] Op 0x17 sub 46 reborn: không trang bị slot ≤ 6; update rebirth formula columns; `DELETE FROM Skill` scoped + `player_id`; reply → `F44402002C01`, quest step, death/close socket (Ch2 §2.3.14 reborn).
- [x] Op 0x2C reborn pet: `stt = data[6]` (stt ≤ 8 nên 1 byte là đủ); scan slot tìm `RbPetFrom`/`RbPetTo`; recompute pet (level 1, skills từ NPC, bonus mốc 30/60), tiêu Rb; packets `F44407000F02`, `F4440C000F01`, status, `F44406001301`, `F44402002C01`; guard fail → silent (Ch2 §2.3.26).
- [x] Golden: thao tác skill được ghi `golden/18-skill-learn.golden` + scenario `18-skill-learn`.

## Mã nguồn (đối chiếu kiểm tra)

**Rust:**
- Player skill learn (op 0x1C sub 1): `src/server/handlers/skills.rs:100` (cost/prereq/element gate khớp C#)
- Pet skill upgrade (op 0x1C sub 2): `src/server/handlers/skills.rs:228`
- Pet reborn (op 0x2C): `src/server/handlers/skills.rs:274`; weighted-random stat `skills.rs:50` (`random_point_stat`); broadcast map + `pet_status_single` `skills.rs:427`
- Shared pet status (0F08/0F14/trailer): `src/server/spawn.rs:280` (`pet_stat_entry`), `:331` (`pet_status_single`)
- Player reborn (op 0x17 sub 46): `src/server/handlers/inventory.rs:390`; quest-win tail `inventory.rs:493`
- Quest-win keyed (OnWin cho reborn): `src/server/handlers/quest.rs:109` (`battle_quest_win_talk`), core `:123` (`battle_quest_win_impl`)
- Golden scenario: `tests/common/mod.rs:238` + `golden/18-skill-learn.golden`

**C# (ts_server_old/Server_TS_Online/Client.cs hoặc Data.cs — KHÔNG commit):**
- `Update_H1C` case 1 (player skill): `Client.cs:7138-7243`; case 2 (pet skill): `Client.cs:7245-7309`
- `RebornPet` (op 0x2C): `Client.cs:9860-10000`; `GetRandomPointPet` (weighted-random): `Data.cs:98-155`
- Pet status: `Data.SendStatusPet` `Data.cs:2212-2278`; pet HP/SP max `getPetHpMax/getPetSpMax` `Data.cs:5569-5603`
- `Update_H17` case 46 (player reborn + quest tail + close socket): `Client.cs:5666-5752`; `QuestGetDataNpc/QuestUpdateDataNpc` `Client.cs:8422-8445`; `BattleQuestWin(Client,key)` `Data.cs:5812-5998`

## Notes / deferred

Các lệch với C# đã được xử lý trong đợt này (theo quyết định chốt khi grill):
- **G1 (RNG bonus point pet reborn):** C# dùng `Data.GetRandomPointPet` (weighted-random theo 6 stat NPC, 7 `.NET` draw/điểm, nguồn `Data.random_0` time-seed). Rust trước dùng vòng `i % 6` deterministic → đã đổi sang `DotNetRandom::time_seeded()` + `random_point_stat` (không golden-diff được vì C# cũng time-seed — deviation đã biết, đúng semantic + RNG consumption).
- **G2 (HpMax/Spmax pet):** C# tính từ stat **gốc** (trước bonus) và map `getPetHpMax` (rb0/1→`getHpMax(0)`, rb2→`getHpMax(1)`); Rust trước tính từ stat đã cộng bonus + truyền `reborn` thẳng → đã sửa.
- **G3 (broadcast pet reborn):** C# `SendToAllMapid` gồm cả sender → Rust `out.send` (+ `hub.broadcast_except` cho map). `F44407000F02`/`F4440C000F01` giờ tới toàn map, không chỉ self.
- **G4 (frame status pet):** hand-rolled `0F08` lệch chuẩn (thiếu `00`/Fai/Quest, `le16` vs `le32` id, thiếu trailer `0F12`) → refactor dùng chung `spawn::pet_status_single`.
- **G5 (tail player reborn):** thiếu `BattleQuestWin` + `QuestUpdateDataNpc(59411,2,2)` + packet `F4441100140100000001010302000000000000F476` + `shutdown`/close socket → đã bổ sung.

**Deferred (không làm trong #13):**
- Niche edge case: reborn ≥2 của player (nhánh `else` trong C# case 46 không có branch reborn==2 — để degenerate; Rust xử như reborn 1 logic). Độ hiếm gặp (reborn tối đa 2), chưa sửa để tránh lệch thêm.
- Chia sẻ item cho thành viên party trong `battle_quest_win_talk` chạy trong luồng handler reborn: `member` closure trả `None` (không resolve member session từ handler); C# ghi trực tiếp item cho member. Ảnh hưởng chỉ khi quest reborn có `WinRewards` chia 3 — hiện không có.
- Mapping `player_id` scope trên `DELETE FROM Skill`: đã duy trì (Ch5 §5.4), trùng với C#.