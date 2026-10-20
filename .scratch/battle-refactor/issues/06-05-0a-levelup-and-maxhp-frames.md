# 06: Frame 05 0A LevelUP + 08 01 CD/CE khoá MaxHP/MaxSP

**What to build:** Hai frame còn thiếu của đường lên cấp: (a) khi nhân vật lên cấp sau trận, phát `F4440600050A{le32(charId)}` để client gắn nhãn "LevelUP" + phát sound; (b) `Stat::Hpmax` / `Stat::Spmax` đang map về `None` nên MaxHP/MaxSP server-side không bao giờ tới client — phát `08 01` với attr `0xCD` (MaxHP) / `0xCE` (MaxSP) sau `08 01 23` (level) để khoá giá trị server tính, tránh client tự recompute lệch.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Builder `05 0A` byte-exact (`F4440600050A` + le32 charId) và được gửi khi lv_ups > 0 (mỗi nhân vật lên cấp một frame)
- [x] `08 01 CD/CE` phát sau `08 01 23` trong pipeline finish (thứ tự: Texp → Lv → Hpmax → Hp → Spmax → Sp → Point → SkillPoint)
- [x] Attr `0xCD/0xCE` khớp bảng offset client (`04_battle_end_rewards.md §2.4`)
- [x] Test trong `tests/` cho cả hai frame + thứ tự frame khi lên cấp
