# 13 — Learn/upgrade skills (op 0x1C) + reborn (op 0x17 sub 46 / op 0x2C)

**What to build:** Người chơi mua/học nâng-level kỹ năng (player + pet), đổi nghề (reborn/rebirth), và hồi sinh pet (reborn pet). Kết nối với bảng `skill`/`skillsave` và stat pipeline. Nền cho battle (dùng skill).

**Blocked by:** 10 — Stat skill bar; 11 — Inventory base; 12 — Use item (skill books).

**Status:** done

- [x] Op 0x1C sub 1 player: chuỗi {skill id LE16 + target level}; validate LvMax/Reborn/prereqs/SkillPoint; mỗi success → `F4440C0008016E01`+le32(lv)+le32(skill); kết → `F4440C0008012501`+le32(count)+`00000000` (Ch2 §2.3.17).
- [x] Op 0x1C sub 2 pet: `data[6]` stt, `[7–8]` skill id, `[9]` level; chỉ upgrade slot đã tồn tại; reply `F4440F00080204`+stt+`6E01`+le32(lv)+le32(skill).
- [x] Mọi write trên bảng `skill`/`skillsave` mang `player_id` (Ch5 §5.4).
- [x] Op 0x17 sub 46 reborn: yêu cầu không trang bị slot ≤ 6; update rebirth formula columns; `DELETE FROM Skill` scoped + `player_id`; reply `F44402002C01`, quest step, death/close socket (Ch2 §2.3.14 reborn).
- [x] Op 0x2C reborn pet: `stt = u16(data[6..7])`; scan slot tìm `RbPetFrom`/`RbPetTo`; recompute pet (level 1, skills từ NPC, 30/60 threshold bonuses), consume Rb; packets `F44407000F02`, `F4440C000F01`, status, `F44406001301`, `F44402002C01`; guards fail → silent (Ch2 §2.3.26).
- [x] Golden: thao tác skill/reborn được ghi lại (nếu capture có).