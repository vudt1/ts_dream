# ADR 0002 — PC server dùng mobile combat formula + bỏ ProtocolProfile enum

- **Ngày**: 2026-08-28
- **Trạng thái**: Accepted
- **Người quyết định**: Owner (grilling Q1–Q9)

## Bối cảnh

`src/battle/runner.rs:1302,1479,1488,1559` đã gọi `mobile_damage::calculate_attack` thay vì `damage::calc_physical_damage` / `damage::calc_magic_damage`. Toàn bộ tham số PC-classic (`combo_field`, `num37`, `do_manh`, `skill_tt`, `atk`, `int_stat`, `avg1`, `avg2`) bị `let _ = (...)` discard — `damage.rs` (525 dòng) chỉ còn đóng vai trò helper (`banker_round`, `get_random_*`, `get_turn`, `hit_exp`, `calc_kill_exp`).

`src/protocol/profile.rs` định nghĩa enum `ProtocolProfile { PcALogin, KotlinMobile }` cùng `FROZEN_PC_COLLISIONS = [0x19, 0x1B, 0x1F, 0x23]` và `is_frozen_pc_collision()`. `src/server/dispatcher.rs:42` mặc định `ServerEnv::none()` là `KotlinMobile`. Tại `src/server/dispatcher.rs:254-302`, 5 opcode được `match profile` chia hai nhánh; nhánh `KotlinMobile` gọi handler thật, nhánh `PcALogin` rơi vào `compat::handle` (chỉ log, không phản hồi).

`src/data/loaders/skill.rs` chỉ parse `Skill_C.dat` (mobile) — không parse `Data/Skill.Dat` (PC, 30 KB, đã có sẵn). Comment `src/data/loader.rs:307-308` ghi: *"The supplied PC Skill.Dat has a different, unverified layout. Only an explicitly supplied mobile-compatible Skill_C.dat may be parsed."* Doc string `src/data/tables.rs:205` nói *"loaded from mobile-compatible Skill.Dat"* — sai vì file thực tế là PC.

Kotlin `ts_mobile_server/` chỉ là source tham chiếu, không bao giờ được dịch ra wire từ server Rust này. Repo hướng đến server **chỉ dành riêng cho phiên bản PC** (`aLogin.exe`, port 6414).

## Quyết định

### Phần 1 — Gameplay: PC server dùng mobile combat formula

1. **Giữ `mobile_damage.rs` làm damage pipeline duy nhất** cho PC server. Công thức (1.5× element multiplier, 0.6× ATK base, 0.5× Int base, 0.1× reborn multiplier, 5–50% thunder roll, 0.5× PvP multiplier, status_infliction 50%+Int/2) là combat formula chính thức cho cả PC client lẫn mobile client.
2. **Xóa dead code trong `src/battle/damage.rs`**: `calc_physical_damage`, `calc_physical_damage_stat`, `calc_magic_damage`, `apply_buff_modifiers`, `get_damage_thuoctinh`, `get_damage_skill_int`, `get_thuoctinh_khac`, `physical_base`, `refine_base`. Giữ lại `banker_round`, `get_random_miss_attack`, `get_random_miss_troi`, `get_random_miss_flee`, `get_random_miss_combo`, `randomize_with_percent`, `randomize_array`, `get_random_skill_npc`, `get_random_drop`, `get_random_drop_slot`, `hit_exp`, `calc_kill_exp`, `calc_combo_exp`, `get_turn` (vì `runner.rs` vẫn dùng làm helper cho status rolls, exp rewards, NPC skill pick).
3. **Viết parser mới cho `Data/Skill.Dat` PC** trong `src/data/loaders/skill_pc.rs`, tham khảo cấu trúc `SkillDatLoader` mobile. Loader hiện tại (`skill.rs`) giữ nguyên cho khả năng đọc `Skill_C.dat` mobile nếu cần. `data.binary_skill_defs` sẽ chứa PC `Skill.Dat` defs thay vì mobile.

### Phần 2 — Cấu trúc: Bỏ `ProtocolProfile` enum

4. **Xóa `src/protocol/profile.rs` hoàn toàn** — kể cả `ProtocolProfile` enum, `FROZEN_PC_COLLISIONS` const, `is_frozen_pc_collision()`. Xóa dòng `pub mod profile;` ở `src/protocol/mod.rs`.
5. **Xóa field `profile: ProtocolProfile` khỏi `ServerEnv` struct** (`src/server/dispatcher.rs:32`). `ServerEnv::none()` không còn default value cho profile.
6. **Tại `dispatcher.rs:254-303`**, sửa 5 nhánh `match profile`:
   - `0x1A` PC: gọi thẳng `npc_event::handle_pc_talk(ctx).await`. Bỏ nhánh `KotlinMobile => compat::handle(ctx)`.
   - `0x19`, `0x1B`, `0x1F`, `0x23` PC: rơi vào `compat::handle(ctx)` (PC chưa có handler cho các opcode này — Trade/NPC shop/Pet stable/Guild ở PC table là opcode khác, chưa port).
7. **`compat::handle` đổi tên thành `unimplemented`** (file `src/server/handlers/compat.rs` → `src/server/handlers/unimplemented.rs`). Module name trong `handlers/mod.rs` cũng đổi. Lý do đổi tên: handler này dùng cho mọi opcode chưa implement (30+ opcode ở `dispatcher.rs:323-325` + default `_ =>`), không phải "compat" cho PC dialect nào cả. Tên `pc_compat` (gợi ý ban đầu) sẽ gây hiểu nhầm.

### Phần 3 — Rename identifier mang tên "mobile" gây hiểu nhầm (Option α)

8. **`src/battle/mobile_damage.rs` → `src/battle/combat_formula.rs`**. Module khai báo trong `src/lib.rs:59` đổi tương ứng.
9. **`MobileSkillInput` → `SkillFormulaInput`** (struct).
10. **`MobileAttackResult` → `AttackResult`** (struct).
11. **`BattleData.mobile_skills: Option<...>` → `BattleData.binary_skills: Option<...>`** (field). `BattleData::with_mobile_skills(...)` → `BattleData::with_binary_skills(...)`. Comment `runner.rs:47` đổi từ *"Optional mobile `Skill_C.dat` metadata"* → *"Optional binary skill catalog (PC `Skill.Dat` or mobile `Skill_C.dat`)"*.
12. **`mobile_skill_input` → `binary_skill_input`**, **`mobile_skill_def` → `binary_skill_def`** (private functions trong `runner.rs`).
13. **`get_pos_attack_mobile` → `get_pos_attack_by_fight_area`** trong `src/battle/targeting.rs:33` (vì nó resolve target theo fight_area, không phải "mobile-specific").
14. **Local variables** trong `runner.rs`: biến `mobile` ở line 1302, 1479, 1488, 1559 → `attack`. Biến `mobile_skill` ở line 777, 783, 790, 816 → `skill_def`.
15. **Doc comments trong `runner.rs`**: line 1363 *"Mobile Skill_C status metadata"* → *"Binary skill status metadata"*. Các comment nói "mobile formula" trong `combat_formula.rs` (sau rename) đổi thành "combat formula".
16. **Comment loader `src/data/loader.rs:307-308`**: xóa, vì PC `Skill.Dat` sẽ được parse bởi parser mới. Comment `src/data/loader.rs:21` *"Rich mobile-compatible binary skill definitions from Skill.Dat"* → *"Rich binary skill definitions from Skill.Dat"*. Doc `src/data/tables.rs:205` *"loaded from mobile-compatible Skill.Dat"* → *"loaded from PC Skill.Dat"*. Doc `src/data/loaders/skill.rs:1` *"Mobile-compatible `Skill_C.dat`/`Skill.Dat` loader"* → *"`Skill_C.dat` mobile loader (PC `Skill.Dat` parser lives in `skill_pc.rs`)"*.
17. **Các comment tham chiếu Kotlin** trong `src/eve/mod.rs:4`, `src/data/reader.rs:299,365`, `src/data/loaders/warp.rs:20,28`, `src/server/handlers/login.rs:158`, `src/server/handlers/npc_event.rs:29`, `src/server/session.rs:133`, `src/db/accounts.rs:4`, `src/server/gm.rs:3`, `src/server/dispatcher.rs:252,263,269,284,299,321`: **giữ nguyên**. Chúng nói đúng — Kotlin `ts_mobile_server/` là nguồn port, comment ghi rõ điều đó là tài liệu hữu ích, không gây hiểu nhầm về dialect runtime.

### Phần 4 — Cập nhật `CONTEXT.md`

18. **Thêm entry mới** `Combat Damage Formula` mô tả công thức mobile đang dùng cho PC server, link tới `combat_formula.rs`.
19. **Thêm entry mới** `Skill Catalog` mô tả nguồn dữ liệu kỹ năng (PC `Skill.Dat` ưu tiên, fallback mobile `Skill_C.dat` nếu có).
20. **Thu hồi entry** `ProtocolProfile` (nếu có trong tương lai) — không còn tồn tại sau refactor này. Nếu `CONTEXT.md` hiện không có entry này thì không cần xóa.
21. **Sửa entry** `ThingData` và `Reborn/Rebirth` nếu chúng có đề cập "mobile-aligned" hoặc "mobile formula" — kiểm tra tại thời điểm áp dụng ADR.

## Hậu quả

### Tích cực

- Code production không còn mang tên "mobile" gây hiểu nhầm về dialect runtime. Mọi `mobile` còn lại trong code đều là comment tham chiếu Kotlin (rõ ràng, không gây nhầm).
- `damage.rs` (525 dòng) giảm xuống còn ~300 dòng helper, không còn hàm `calc_physical_damage` / `calc_magic_damage` / `apply_buff_modifiers` mà `runner.rs` không bao giờ gọi. Dead code bị xóa, giảm tải nhận thức khi onboard dev mới.
- `dispatcher.rs` đơn giản hóa: bỏ 5 `match profile` ở `0x1A/0x19/0x1B/0x1F/0x23`. Tổng cộng ~20 dòng code giảm.
- `FROZEN_PC_COLLISIONS` const không còn — khái niệm "collision" chỉ có nghĩa khi có 2 dialect, không áp dụng cho PC-only.
- `compat::handle` → `unimplemented` giúp dev mới hiểu ngay: opcode này chưa implement, không phải "compat cho dialect X".

### Tiêu cực

- **Golden replay bị mất**: 18 file `golden/*.golden` chỉ replay được với `ProtocolProfile::KotlinMobile`. Sau khi xóa enum, không thể replay chúng. Nếu sau này cần golden regression cho PC, phải tạo fixture mới bằng cách chạy server với `aLogin.exe` thật.
- **PC `Skill.Dat` parser mới phải reverse-engineer** — không có parser mẫu trong repo, chỉ có cấu trúc mobile để tham khảo. Đây là ticket lớn, scope riêng.
- **Không có PC native combat formula** — nếu sau này community yêu cầu "damage PC chính hãng", phải viết lại `combat_formula.rs` từ `Formula.Dat`. Có thể xem là thiếu sót; ADR này chấp nhận vì mobile formula đã pass test/replay.

### Trade-off đã chọn

- **B1 vs B3**: Chọn B3 (giữ mobile formula) thay vì B1 (khôi phục PC `Formula.Dat` formula). Lý do: (i) `Formula.Dat` PC formula (`damage::calc_physical_damage` cũ) phức tạp, chưa reverse-engineered đầy đủ, nhiều tham số `num36`/`num37`/`combo_field` chưa rõ semantics; (ii) `mobile_damage.rs` đã được viết dựa trên Kotlin, có reference rõ ràng; (iii) test/replay golden đã khớp với mobile formula output, đổi sang PC formula sẽ break test mà không có lợi tức rõ ràng.
- **5.1 vs 5.2/5.3**: Chọn 5.1 (xóa enum hẳn) thay vì giữ test seam. Lý do: server là PC-only, không có kế hoạch chạy mobile client; giữ `KotlinMobile` variant chỉ để test sẽ là "code không chạy production" — dễ bị hiểu nhầm là còn dùng, tốn review effort. Golden replay có thể tạo lại khi cần.

## Phạm vi chưa bao phủ (follow-up tickets)

1. **PC `Skill.Dat` parser** (`src/data/loaders/skill_pc.rs`) — ✅ DONE 2026-08-28.
   Spec từ `SkillData.cs`/`SkillInfo.cs` (Pack=1, 86 bytes/record, 86-byte
   zero header, `DecodeItem8/16/32` XOR obfuscation, name/des reverse bytes).
   Parser tạo `PcSkillDef` (raw PC fields) + bridge sang `BinarySkillDef` qua
   `pc_to_binary` (best-effort field mapping). 352 records load được từ
   `Data/Skill.Dat`; id range 9984..=23019. Skill 10000 (basic attack) tồn
   tại với name "Nham quái".
2. **PC handlers cho 4 opcode còn rỗng** (`0x19/0x1B/0x1F/0x23`) — ticket riêng, theo PC table semantic. Hiện tại rơi vào `unimplemented::handle` (chỉ log, không phản hồi). Owner đã xác nhận: tạm thời để rỗng, xử lý sau.
3. **Golden fixtures PC** — ticket riêng, owner đã chọn 3c (bỏ qua — golden hiện tại vẫn dùng được cho test in-memory). PC golden không cần thiết vì server chỉ chạy PC dialect, replay sẽ tạo fixture mới khi có `aLogin.exe` capture.

## Cleanup 2026-08-28 — Round 2 (Q1–Q9)

Grilling session chốt scope:
- Q1: phạm vi PC-only → loại bỏ `ProtocolProfile::KotlinMobile`.
- Q2: rename scope = **Option α** (chỉ identifier gây hiểu nhầm, giữ comment tham chiếu Kotlin).
- Q3: doc `tables.rs:205` → *"loaded from PC Skill.Dat"*.
- Q4: rename scope = α.
- Q5: **Hướng 5.1** (xóa `ProtocolProfile` enum, `FROZEN_PC_COLLISIONS`).
- Q6: `compat::handle` → `unimplemented` (override đề xuất 6b ban đầu, vì handler này dùng cho mọi opcode chưa implement, không phải "PC compat" — tên `pc_compat` gây hiểu nhầm).
- Q7: viết ADR-0002 vì đủ 3 tiêu chí (hard-to-reverse, surprising, real trade-off).
- Q8: **Một ADR gộp** cả gameplay + refactor cấu trúc, vì context chung "PC-only pivot".
- Q9: `mobile_damage.rs` → `combat_formula.rs`.

## Tham chiếu

- Grilling session 2026-08-28 Q1=B3, Q2=(a) PC parser tham khảo mobile, Q3="PC Skill.Dat", Q4=α, Q5=5.1, Q6=6b→override `unimplemented`, Q7=viết ADR, Q8=(x) gộp, Q9=`combat_formula.rs`.
- Files affected: `src/battle/mobile_damage.rs`, `src/battle/damage.rs`, `src/battle/runner.rs`, `src/battle/targeting.rs`, `src/protocol/profile.rs`, `src/protocol/mod.rs`, `src/server/dispatcher.rs`, `src/server/handlers/compat.rs`, `src/server/handlers/mod.rs`, `src/data/loaders/skill.rs`, `src/data/loader.rs`, `src/data/tables.rs`, `src/lib.rs`, `CONTEXT.md`.
- Owner xác nhận: server chỉ dành cho PC, Kotlin `ts_mobile_server/` chỉ là source tham chiếu.
