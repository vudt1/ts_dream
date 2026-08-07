# 17 — Pet actions (op 0x0F) + pet stable (op 0x1F) + summon/recall (op 0x13)

**What to build:** Người chơi quản lý pet trọn vòng đời: triệu hồi/hồi lại, thả, cho vào/ra chuồng, đổi vị trí, mount/horse, đặt tên, và triệu hồi trong battle. Pets theo nhân vật trong `pet` table (`player_id`, `stt`).

**Blocked by:** 07 — Login + spawn (pet summary dumps); 04 — DB (`pet` table); 11 — Inventory (pet items equippable / pet stable slot).

**Status:** ready-for-agent

- [ ] Op 0x0F: sub2 release; sub3 store → `F44405001F06`+stt+`0000`, `UpdateStatusPetWhenUseItem`, broadcast `F4440C000F01`; sub7 take-from-stable (red msg + `F44402001F09` nếu active); sub8 swap (red bx if fighting `F44402001F09F44402001F0C`); sub4 mount horse (id `18000..19000`, `F4440E000F05`); sub5 unmount `F44406000F06`; sub6 rename broadcast `F444`+len+`0F09` (Ch2 §2.3.11).
- [ ] Op 0x1F pet stable menu — ngữ nghĩa equal 0x0F sub3/7/8 nhưng reply `F44405001F060000` và menu kết `F44402001F09`/`1F0C` (Ch2 §2.3.20).
- [ ] Op 0x13 summon/recall (ngoài battle): sub1 set active `F44406001301`+id; sub2 clear + `F44402001302`; in battle (cell `_Attacked==0`): summon loads pet, removes player's battle cells, spawn pet `ChangedWar` type 4, `F4441A000B0505`+warPacket + `F44406001301`+id; recall tương tự (Ch2 §2.3.12).
- [ ] Pet statuses computed và gửi correct; thao tác `pet` mang `player_id`.
- [ ] Golden: pet scenario được ghi lại (Ch9 §9.6).