# 23: Tái cấu trúc Battle Combo Engine toàn diện theo 3 quy tắc vàng và gameplay chuẩn TS Online

**What to build:** Khắc phục triệt để các lỗ hổng nghiêm trọng trong giải thuật phát hiện, tính toán xác suất và thực thi liên kích (Combo) tại `src/battle/runner.rs::detect_combo`, `src/battle/damage.rs::get_random_miss_combo`, `src/data/loaders/skill_pc.rs` và `src/battle/combat_formula.rs`. Tham chiếu nghiên cứu chi tiết tại [`.scratch/battle-refactor/research-combo-rules.md`](../research-combo-rules.md), 3 quy tắc vàng tại [`.scratch/battle-refactor/combo_rule.md`](../combo_rule.md) và từ vựng miền tại [`GLOSSARY.md`](../../GLOSSARY.md#L103-L125).

---

## Bản Đồng Thuận Thiết Kế (Grill Session Consensus)

1. **Phạm vi Ngưỡng Agi Gap $\le 11$ điểm (Agi Span Scope)**:
   - Áp dụng trên **tập hợp các đấu sĩ thực sự tham gia cùng một chuỗi combo** đánh vào cùng 1 mục tiêu (`max(Agi_chuỗi) - min(Agi_chuỗi) <= 11`).
   - **Không áp dụng khóa cứng toàn đội (No Team-wide Lock)**: Các đấu sĩ khác trong đội hình dù có Agi lệch xa nhưng đánh mục tiêu khác hoặc hành động độc lập không làm vô hiệu hóa combo của các cặp/nhóm tham gia hợp lệ. Cho phép chia mục tiêu (split-target) thành nhiều chuỗi combo độc lập trong 1 lượt đấu.

2. **Công thức suy giảm Tỷ lệ Hợp kích theo Level Quái khi $\Delta \text{LV} < 25$**:
   - Khi $\text{LVTB}_{\text{team}} - \text{Level}_{\text{quái}} \ge 25 \implies$ Tỷ lệ đạt **$100\%$**.
   - Khi $\text{LVTB}_{\text{team}} - \text{Level}_{\text{quái}} < 25 \implies$ Tỷ lệ giảm dần theo công thức:
     $$\text{Combo Rate (\%)} = \text{clamp}\!\left( \left\lfloor \frac{\text{LVTB}_{\text{team}} - 25}{\text{Level}_{\text{quái}}} \times 100\% \right\rfloor, 0, 100 \right)$$
     *(Nếu $\text{LVTB}_{\text{team}} \le 25 \implies 0\%$. Quái cấp càng cao thì khi thiếu cấp, tỷ lệ combo rớt càng nhanh)*.

3. **Cấp độ Hiệu dụng 200 cho Nhân vật Trùng Sinh (Reborn Effective Level)**:
   - Bất kỳ đấu sĩ nào đã Trùng Sinh (`c.reborn > 0`) **luôn được tính là Level 200** khi tính toán `avg1` / `avg2` ở đầu trận.
   - **Áp dụng cho mọi thể thức trận đấu (cả PvE và PvP)**: Đảm bảo tính nhất quán của thuộc tính phe (`TurnState.avg1/avg2`) mà không phân nhánh logic.

4. **Tiêu Chí Kỹ Năng Hợp Kích: Phụ thuộc 100% vào thuộc tính `skill_type == 1` (`Active`)**:
   - Khắc phục hiểu lầm trước đây về việc "chặn AoE $\ge 4$ mục tiêu": Trong TS Online, kỹ năng **Loạn Kích (`13013`)** là kỹ năng tấn công chủ động vật lý kinh điển (`skill_type == 1`, `Active`) với 4 mục tiêu, **HOÀN TOÀN ĐƯỢC PHÉP HỢP KÍCH** (team Phong Loạn Kích là đội hình auto train/bot quái cốt lõi của game).
   - Các kỹ năng như Phi sa tẩu thạch (`10008`), Hồng thủy (`11008`), Thiên băng địa liệt (`10012`), Phong hỏa liêu nguyên (`12014`) không thể combo **KHÔNG PHẢI VÌ SỐ MỤC TIÊU**, mà vì chúng là **Phép thuật / Magic attack (`skill_type == 2`)**!
   - **Quy tắc chuẩn xác**:
     - Cho phép Combo: Đòn đánh thường (`10000`/`0`) HOẶC bất kỳ kỹ năng nào có `skill_type == 1` (`Active`), bao gồm cả Đơn thể, 2 mục tiêu (Đá lăn, Dương viêm), 3 mục tiêu (Hỏa tiễn, Ném đá) và 4 mục tiêu (**Loạn kích**).
     - Cấm Combo: Mọi kỹ năng có `skill_type != 1` (Phép/Buff `2`, Debuff `3`, Hồi máu `4`, Thủ/Hộ chủ `15..17`).

5. **Phạm vi Nhân Hệ số Sát thương $\times 1.3$**:
   - Hệ số dồn sát thương $\times 1.3$ của chuỗi Combo **chỉ áp dụng tập trung cho ô Mục tiêu chính được khóa chung** (`row_attack`, `col_attack`), nơi hoạt ảnh Hợp kích `20006/20007` hội tụ.
   - Các ô phụ bị trúng do hiệu ứng lan của kỹ năng AoE (như Loạn kích, Ném đá, Hỏa tiễn) chỉ nhận sát thương kỹ năng cơ bản.

6. **Cập nhật Từ điển Miền (Ubiquitous Language)**:
   - Đã cập nhật định nghĩa chính xác vào [`GLOSSARY.md`](../../GLOSSARY.md#L103-L125).

---

## Chi Tiết Kỹ Thuật Các Hạng Mục Triển Khai

1. **Chuẩn hóa Agi Gap $\le 11$ điểm (Quy tắc vàng 1)**:
   - Xóa bỏ hoàn toàn con số giả định `diff <= 800` tại `runner.rs:1271 & 1290`.
   - Với cặp đấu sĩ mở chuỗi: Chênh lệch Nhanh nhẹn bắt buộc $\le 11$ (`(next.agi - attacker.agi).abs() <= 11`).
   - Với đấu sĩ nối dài chuỗi (từ người thứ 3 trở đi): Khoảng cách giữa đấu sĩ có Agi cao nhất và thấp nhất trong toàn bộ chuỗi combo mở rộng không được vượt quá 11 (`max(combo_hi, next.agi) - min(combo_lo, next.agi) <= 11`).

2. **Tính toán Tỷ lệ Hợp kích theo Ngưỡng LVTB (Quy tắc vàng 2)**:
   - Thay thế việc hardcode 100% trong `damage.rs::get_random_miss_combo`.
   - Hàm nhận thêm tham số `lvtb_team: f64` và `target_lv: i64`. Áp dụng công thức suy giảm đã thống nhất ở mục trên.

3. **Quy tắc Tính Level 200 cho Nhân vật Trùng Sinh (Reborn)**:
   - Trong `runner.rs:428-437`, khi tính `avg1` và `avg2` ở lượt đấu đầu tiên, kiểm tra trường `c.reborn`.
   - Nếu `c.reborn > 0`, cấp độ hiệu dụng của nhân vật bắt buộc được tính là **Level 200**, bất kể cấp độ hiển thị thực tế là bao nhiêu. Nhân vật chưa Trùng Sinh (`c.reborn == 0`) giữ nguyên `c.lv`.

4. **Sửa lỗi Phân loại Kỹ năng Hợp lệ Tham gia Combo (Quy tắc vàng 3)**:
   - Khắc phục lỗi đọc nhầm trường: `runner.rs:886-895` và `1251-1254` đang đọc `s.kind` (`pc.type_` từ offset 21 trong `Skill.Dat` - vốn là hệ ngũ hành 0..5), dẫn đến việc chỉ kỹ năng hệ Địa (`kind == 1`) mới được combo, còn kỹ năng Active hệ Hỏa/Phong/Thủy/Vô hệ bị từ chối.
   - Sửa lại: Đọc trường loại kỹ năng thực sự từ `s.attribute` (`pc.skill_type` offset 33).
   - Bỏ rào cản nhân tạo `skill_target_count <= 3` đã viết sai trước đây.
   - Helper `is_combo_eligible_skill(data, skill_id)`:
     - `skill_id == 10000 || skill_id == 0 => true`
     - Tra cứu `skill_type`: nếu `skill_type == 1` (`Active`) => `true` (bao gồm cả Loạn kích `13013`, Hỏa tiễn `12003`, Ném đá `10005`, v.v.).
     - Ngược lại (`skill_type != 1`, ví dụ Phép `2`, Debuff `3`, Heal `4`) => `false`.

5. **Đồng bộ Dữ liệu Nhị phân `fight_area` trong `skill_pc.rs`**:
   - `skill_pc.rs:308`: Gán `fight_area = pc.nb_target` (offset 34) thay vì `pc.unk4` (offset 31, vốn luôn bằng 0) để `BinarySkillDef.fight_area` phản ánh đúng số mục tiêu kỹ năng phục vụ cho thuật toán chọn mục tiêu AoE trên lưới bàn cờ.

6. **Kích hoạt Hệ số Dồn Sát Thương $\times 1.3$ và Thưởng EXP $+8.6\%$ khi Combo**:
   - Phục hồi hệ số `num37` (hoặc cờ `is_combo`): Khi đòn đánh thuộc chuỗi combo (`ts.combo_active == 1`), sát thương đầu ra sau khi tính qua công thức chiến đấu được nhân thêm hệ số dồn $\times 1.3$ áp dụng riêng cho ô mục tiêu chính.
   - Khi tiêu diệt quái bằng combo (`ts.combo_cells` không rỗng), áp dụng `damage::calc_combo_exp` (nhân thưởng `1.086`) cho tất cả các thành viên tham gia combo.

**Blocked by:** 19: Phục hồi kênh lệnh chiến đấu 0x32 Sub 01/02 và Domain Enum BattleAction, 20: Tái cấu trúc vòng đời lượt đấu và đồng bộ timing mở hiệp 34 01

**Status:** completed

- [x] Cập nhật `src/data/loaders/skill_pc.rs`:
  - Gán `fight_area: pc.nb_target` (offset 34) để phản ánh đúng số mục tiêu kỹ năng.
  - Đảm bảo `attribute` mang giá trị `pc.skill_type` (offset 33).
- [x] Cập nhật `src/battle/damage.rs`:
  - Nâng cấp `get_random_miss_combo(rng, lvtb_team, target_lv)` tính toán tỷ lệ xác suất theo hiệu số $\text{LVTB} - \text{Level Quái}$.
- [x] Cập nhật `src/battle/runner.rs`:
  - Cập nhật công thức tính `ts.avg1` / `ts.avg2`: Đấu sĩ có `reborn > 0` tính là Level 200.
  - Tạo helper `is_combo_eligible_skill` kiểm tra `skill_type == 1` (`Active`) hoặc `10000`/`0`. Bỏ rào cản `target_count <= 3`.
  - Cập nhật `detect_combo`: Siết Agi Gap $\le 11$ cho cả mắt xích mở chuỗi và toàn bộ chuỗi nối dài (`new_hi - new_lo <= 11`).
  - Áp dụng hệ số dồn sát thương $\times 1.3$ cho mục tiêu chính khi `ts.combo_active == 1`.
  - Tích hợp `damage::calc_combo_exp` khi phân phối kinh nghiệm cho `ts.combo_cells`.
- [x] Mở rộng bộ kiểm thử tích hợp trong `tests/battle_combo_rules_test.rs`:
  - Test 1: Cặp đấu sĩ có chênh lệch Agi $\le 11$ kích hoạt combo thành công; chênh lệch Agi $\ge 12$ bị từ chối combo.
  - Test 2: Chuỗi 3 đấu sĩ với Agi `{100, 105, 111}` (span 11) combo thành công; nếu thêm đấu sĩ Agi 113 (span 13) thì đấu sĩ này bị tách ra đánh đơn.
  - Test 3: Cặp dùng Đánh thường + Ném đá (Active 10005) hoặc Hỏa tiễn (Active 12003) kích hoạt combo thành công.
  - Test 4: Cặp dùng Đánh thường + Loạn kích (Active 13013) HOẶC cặp cả 2 cùng dùng Loạn kích (13013) kích hoạt combo thành công (xác nhận kỹ năng Active AoE 4 mục tiêu combo chuẩn xác).
  - Test 5: Kỹ năng Phép thuật (`skill_type == 2` như Phi sa 10008, Hồng thủy 11008, Thiên băng 10012) bị từ chối combo.
  - Test 6: Đội hình có nhân vật Trùng Sinh level 1 đối đầu quái level 100: LVTB tính là 200, thỏa mãn $\ge 25$ cấp nên đạt 100% combo.
  - Test 7: Team LVTB 40 đánh quái level 50: Tỷ lệ combo tính theo công thức xác suất, không còn hardcode 100%.
  - Test 8: Sát thương của đòn combo cao hơn đòn đánh độc lập nhờ hệ số $\times 1.3$ trên mục tiêu chính.
- [x] `cargo test --all-targets --no-fail-fast` vượt qua 100%.
