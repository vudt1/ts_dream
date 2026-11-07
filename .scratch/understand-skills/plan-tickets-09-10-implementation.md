# Kế Hoạch Triển Khai & Phương Án Kỹ Thuật Chi Tiết Cho Ticket 09 và Ticket 10

**Ngày tạo:** 2026-11-03  
**Phân hệ:** Kỹ Năng & Nghề (`understand-skills`)  
**Tài liệu liên quan:**
- Báo cáo hoàn thành Ticket 07 & 08: [`.scratch/understand-skills/handoff-tickets-07-08-completion.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/handoff-tickets-07-08-completion.md)
- Ticket 09: [`.scratch/understand-skills/issues/09-respec-standards-and-eve-persistence-integrity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/09-respec-standards-and-eve-persistence-integrity.md)
- Ticket 10: [`.scratch/understand-skills/issues/10-battle-holy-spirit-and-combat-cleanups.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/10-battle-holy-spirit-and-combat-cleanups.md)
- Quy chuẩn lập trình: [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md)

---

## 1. Tổng Quan Mục Tiêu

Tài liệu này tổng hợp phương án kỹ thuật chi tiết nhằm khắc phục các điểm tồn đọng được phát hiện trong phiên Code Review hai trục sau khi hoàn thành Ticket 07 và 08:
1. **Ticket 09**: Khắc phục các vi phạm quy chuẩn [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md) trong xử lý tẩy điểm (`respec.rs`), khử trùng lặp vòng lặp điều phối sự kiện Eve trong [`talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs), và bảo đảm phần thưởng kỹ năng từ kịch bản Eve được lưu trữ nguyên tử xuống DB kể cả khi kết thúc trận đấu.
2. **Ticket 10**: Chuẩn hóa chính xác hành vi giải trừ trạng thái có lợi (Buff Dispel) của Thánh Linh Thuật (`14053` Holy Spirit) trong [`runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs), chuẩn hóa công thức sát thương phép, dọn dẹp nhánh mã chết (dead code) trong nhắm mục tiêu, và trích xuất logic re-roll Sách Động (`14050`) thành phương thức dùng chung.

---

## 2. Phương Án Kỹ Thuật Chi Tiết Cho Ticket 09

### 2.1 Chuẩn hóa Frame Kho và Hằng Số Thuộc Tính trong [`src/server/handlers/respec.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs)

#### A. Vấn đề
- Dòng 203–206 và 375–378 định dạng chuỗi hex thô:
  ```rust
  ctx.out.send(format!("F44404001709{:02X}{:02X}", manual_slot, remaining_count));
  ```
  Vi phạm trực tiếp [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md) §4 ("Các chuỗi literal hex `F4440400...` phải được thay thế bằng helper").
- Dòng 43–48 trong `calculate_starting_base_stat` sử dụng literal số `1, 2, 3, 4` để biểu thị hệ nguyên tố.

#### B. Giải pháp
1. **Sử dụng helper canonical**:
   Thay toàn bộ lời gọi sinh chuỗi bằng hàm chuẩn đã có sẵn tại [`src/server/inventory.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/inventory.rs):
   ```rust
   ctx.out.send(crate::server::inventory::item_count_changed_frame(manual_slot, remaining_count));
   ```
2. **Định nghĩa Domain Constants cho Hệ nguyên tố**:
   Trong [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs) (hoặc trực tiếp trong `respec.rs` nếu mang tính cục bộ):
   ```rust
   pub const ELEMENT_EARTH: u8 = 1; // Hệ Địa
   pub const ELEMENT_WATER: u8 = 2; // Hệ Thủy
   pub const ELEMENT_FIRE: u8  = 3; // Hệ Hỏa
   pub const ELEMENT_WIND: u8  = 4; // Hệ Phong
   ```
   Cập nhật hàm `calculate_starting_base_stat`:
   ```rust
   pub fn calculate_starting_base_stat(session: &Session, attr: Attr) -> u16 {
       match (session.thuoctinh, attr) {
           (ELEMENT_EARTH, Attr::Def) => 2,
           (ELEMENT_WATER, Attr::Hpx) | (ELEMENT_WATER, Attr::Spx) => 2,
           (ELEMENT_FIRE, Attr::Atk) => 2,
           (ELEMENT_WIND, Attr::Agi) => 2,
           _ => 1,
       }
   }
   ```

---

### 2.2 Khử Nhân Bản Loop Sự Kiện & Tích Hợp DbPool Sau Trận Đấu trong [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs)

#### A. Vấn đề
- Hàm `execute_event_step` (đồng bộ, 115 dòng) và `execute_event_step_with_pool` (async, 115 dòng) gần như giống hệt nhau từng dòng mã, chỉ khác việc gọi `upsert_skill` khi gặp `EveResultClass::Skill`.
- Hàm [`resume_eve_after_battle`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L505-L519) gọi `execute_event_step(session, data, &mut out)`, truyền `pool = None`, dẫn đến nếu NPC thưởng kỹ năng sau trận đấu (như học thuật sau khi đánh bại boss sự kiện), kỹ năng không được ghi vào SQLite.
- Trong `execute_event_step_with_pool`, việc cập nhật `session.skills` trên RAM và phát frame `08 01 6E 01` diễn ra trước khi gọi `upsert_skill`. Nếu DB write thất bại, RAM bị sai lệch (state drift).

#### B. Giải pháp
1. **Hợp nhất xử lý một bước sự kiện đơn lẻ**:
   Tách hàm xử lý một `EventResult` riêng lẻ:
   ```rust
   enum StepProgress {
       Continue,
       Halt,
       SkillAwarded { pid: u32, skill_id: u16, level: u8, sp: u16 },
   }
   ```
   Hàm stepping chung `advance_event_step` sẽ chạy qua vòng lặp, xử lý các loại Talk (1), Menu (6), Door (2), Battle (4),...
2. **Plumbing DbPool vào `resume_eve_after_battle`**:
   Cập nhật chữ ký của `resume_eve_after_battle`:
   ```rust
   pub async fn resume_eve_after_battle(
       session: &mut Session,
       data: &GameData,
       pool: Option<&DbPool>,
       battle_result: u8,
   ) -> (Vec<String>, Option<EveBattleInfo>)
   ```
   Tại call site [`src/battle/service.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) trong `battle_ended`:
   Truyền `state.pool` vào lời gọi `resume_eve_after_battle`.
3. **Nguyên tử hóa trao kỹ năng Eve**:
   Khi phát hiện `SkillAwarded`:
   - Nếu có `pool`: Thực hiện `upsert_skill(Some(pool), pid, skill_id, level, sp, 0).await`.
   - Chỉ khi ghi DB thành công: ghi nhận kỹ năng vào `session.skills`, phát frame client `08 01 6E 01` và gọi `registry_update_if_online(session)`.
   - Nếu ghi DB thất bại: log `warn!`, không phát frame thành công, giữ nguyên RAM.

---

## 3. Phương Án Kỹ Thuật Chi Tiết Cho Ticket 10

### 3.1 Chuẩn Hóa Cơ Chế Giải Trừ Của Thánh Linh Thuật (`SKILL_HOLY_SPIRIT = 14053`)

#### A. Vấn đề
Tại [`src/battle/runner.rs:2928-2965`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L2928-L2965):
```rust
if hit == miss_status::ATTACK {
    byte_val = troi_byte::TYPE19;
    target.type19_id = skill; // << 14053 bị gán đè vào type19_id!
    target.type19_lv = skill_lv;
    target.type19_turn = turn;
    if skill == 13020 {
        ...
    } else if skill == i64::from(SKILL_HOLY_SPIRIT) {
        ...
        if target.type19_id > 0 { // << LUÔN TRUE vì vừa bị gán 14053 ở trên!
            ...
            out.push(Out::Broadcast(packets::purge_status(r, c, packets::PURGE_TYPE_AURA)));
        }
```
Hệ quả: Bất kỳ unit nào bị trúng Thánh Linh Thuật đều bị gửi gói tin xóa Aura và Clear Status kể cả khi mục tiêu không hề có buff Khiên hay buff Loại 19.

#### B. Giải pháp
1. **Tách biệt gán Buff Loại 19 với kỹ năng Dispel**:
   Chỉ gán `target.type19_id = skill` khi `skill != i64::from(SKILL_HOLY_SPIRIT)`:
   ```rust
   if hit == miss_status::ATTACK {
       if skill == i64::from(SKILL_HOLY_SPIRIT) {
           let (r, c) = (target.row, target.col);
           // 1. Giải trừ buff Loại 4 (Khiên ẩn thân / né tránh...)
           if target.type4_id > 0 {
               target.type4_id = 0;
               target.type4_lv = 0;
               target.type4_turn = 0;
               out.push(Out::Broadcast(packets::purge_status(r, c, packets::PURGE_TYPE_BUFF)));
           }
           // 2. Giải trừ buff Loại 15
           if target.type15_id > 0 {
               target.type15_id = 0;
               target.type15_lv = 0;
               target.type15_turn = 0;
               out.push(Out::Broadcast(packets::purge_status(r, c, packets::PURGE_TYPE_BUFF)));
           }
           // 3. Giải trừ buff Loại 19 (Thiên thần thuẫn...) CHỈ KHI MỤC TIÊU THỰC SỰ CÓ
           if target.type19_id > 0 {
               target.type19_id = 0;
               target.type19_lv = 0;
               target.type19_turn = 0;
               out.push(Out::Broadcast(packets::purge_status(r, c, packets::PURGE_TYPE_AURA)));
               out.push(Out::Broadcast(packets::clear_unit_status(r, c, packets::STATUS_DMG_BONUS_PCT)));
           }
           // 4. Sát thương thánh scaled theo INT/MATK người ra chiêu
           let matk = attacker.haint.max(1);
           let holy_dmg = ((matk / 2) + (skill_lv * 40)).max(50);
           self.apply_target_hp(target, holy_dmg, out);
           byte_val = troi_byte::HP;
           dmg = holy_dmg as u16;
       } else {
           // Kỹ năng buff Loại 19 thông thường (ví dụ: 13020, Celestial Shield...)
           byte_val = troi_byte::TYPE19;
           target.type19_id = skill;
           target.type19_lv = skill_lv;
           target.type19_turn = turn;
           if skill == 13020 {
               let bonus = (target.agi as f64 * (0.03 * skill_lv as f64)).ceil() as i64;
               ts.agi_13020 += bonus;
               target.agi += bonus;
           }
           let val = (skill_lv * 5).clamp(1, 100) as u16;
           ts.status_frames.push(packets::battle_unit_status(
               target.row,
               target.col,
               packets::STATUS_DMG_BONUS_PCT,
               turn.clamp(1, 255) as u8,
               val,
           ));
       }
   }
   ```

---

### 3.2 Dọn Dẹp Mã Chết & Trích Xuất Helper Re-roll Sách Động

#### A. Dọn dẹp dead code trong `get_skill_targets`
Tại [`src/battle/runner.rs:1342-1344`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L1342-L1344):
```rust
if skill == i64::from(SKILL_HOLY_SPIRIT) {
    return targeting::get_pos_attack_default(&cells, team, row_attack, col_attack, 1);
}
```
Do đã return sớm, xóa bỏ hoàn toàn đoạn check thừa ở dòng 1387–1389:
```rust
// XÓA BỎ:
// if skill == i64::from(SKILL_HOLY_SPIRIT) {
//     targets = targeting::get_pos_attack_default(&cells, team, row_attack, col_attack, 1);
// } else {
```

#### B. Trích xuất helper `apply_sach_dong_reroll`
Gom logic đổ xúc xắc Sách Động ở dòng 1850–1855 và dòng 2629–2634 thành một hàm duy nhất trong `impl Battle`:
```rust
impl Battle {
    /// Áp dụng cơ chế đổ xúc xắc cứu đòn của Sách Động (Instigation):
    /// Nếu đòn đánh bị tính là MISS và người tấn công có học Sách Động,
    /// có tỷ lệ bonus = (sach_dong_lv * 2)% đổi kết quả thành ATTACK.
    #[inline]
    pub fn apply_sach_dong_reroll(&mut self, attacker: &WarInfo, hit: &mut i64) {
        if *hit == miss_status::MISS as i64 && attacker.sach_dong_lv > 0 {
            let bonus = (attacker.sach_dong_lv * 2).clamp(1, 100) as i32;
            if self.rng.random_0.next_range(0, 100) < bonus {
                *hit = miss_status::ATTACK as i64;
            }
        }
    }
}
```
Tại cả 2 vị trí trên, thay thế toàn bộ khối lệnh bằng:
```rust
self.apply_sach_dong_reroll(attacker, &mut hit);
```

---

## 4. Kế Hoạch Kiểm Thử Bổ Sung (Test Plan)

Bổ sung các test case sau vào [`tests/skill_job_system_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/skill_job_system_test.rs):

1. **Test kiểm tra wire format tẩy điểm**:
   - Khẳng định gói tin cập nhật số lượng vật phẩm khi dùng Cẩm nang tâm tư hoặc Túi gấm cẩn thận khớp chuẩn 100% với `item_count_changed_frame(slot, count)`.
2. **Test Eve Skill Reward Post-Battle Persistence**:
   - Mô phỏng trận đấu sự kiện kết thúc với kết quả `battle_result = 1` (chiến thắng).
   - Gọi `resume_eve_after_battle` với `pool` hợp lệ.
   - Kiểm tra kỹ năng được lưu vào bảng SQLite và session RAM đồng bộ.
3. **Test Holy Spirit Dispel Tính Toàn Vẹn**:
   - **Case 1 (Target không buff)**: Thi triển Thánh Linh Thuật lên mục tiêu thường. Assert không có bất kỳ frame `purge_status(PURGE_TYPE_AURA)` nào được phát ra.
   - **Case 2 (Target có Celestial Shield)**: Gắn buff `14044` cho mục tiêu. Thi triển Thánh Linh Thuật. Assert nhận được frame `purge_status(PURGE_TYPE_AURA)` và `clear_unit_status`, đồng thời `target.type19_id` trở về 0.
   - **Case 3 (Damage Scaling)**: So sánh sát thương gây ra giữa người cast có INT thấp và INT cao, assert sát thương tăng tương ứng.

---

## 5. Quy Trình Chạy Kiểm Thử Chốt Chặn

Khi hoàn tất từng ticket, chạy test tập trung:
```bash
cargo test --test skill_job_system_test
```
Trước khi kết thúc toàn bộ công việc:
```bash
cargo test --all-targets --no-fail-fast
```
Đảm bảo 100% pass và `architecture_guardrails_test` không phát sinh bất kỳ cảnh báo nào.
