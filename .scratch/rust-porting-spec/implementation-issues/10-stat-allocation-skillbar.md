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

**Notes / deferred:** write-through chỉ chạy khi live server có `env.pool` (golden replay in-memory).