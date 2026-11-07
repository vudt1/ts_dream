# Báo Cáo Rà Soát Triển Khai (Research Audit): Battle Buff & Debuff Lifecycle

*   **Người rà soát:** Antigravity Research Agent
*   **Ngày thực hiện:** 2026-11-04
*   **Tài liệu đối chiếu:**
    *   Tài liệu đặc tả (Spec): [`.scratch/understand-skills/battle-buff-debuff-lifecycle.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/battle-buff-debuff-lifecycle.md)
    *   Tài liệu bàn giao (Handoff): [`.scratch/understand-skills/handoff-battle-buff-debuff-lifecycle.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/handoff-battle-buff-debuff-lifecycle.md)
*   **Mã nguồn thực tế (Primary Sources):**
    *   [`src/data/loaders/skill_pc.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/data/loaders/skill_pc.rs)
    *   [`src/battle/damage.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs)
    *   [`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs)
    *   [`src/battle/combat_formula.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/combat_formula.rs)
    *   [`tests/battle_buff_debuff_lifecycle_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_buff_debuff_lifecycle_test.rs)
    *   [`tests/battle_timing_wiring_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_timing_wiring_test.rs)
    *   [`tests/architecture_guardrails_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/architecture_guardrails_test.rs)

---

## 1. Kết Luận Tổng Quan (Executive Summary)

Sau khi rà soát toàn diện giữa **Đặc tả kỹ thuật (Spec)**, **Báo cáo bàn giao (Handoff)** và **Mã nguồn thực tế (Primary Sources)**:

> **KẾT LUẬN: ĐẠT 100% YÊU CẦU ĐẶC TẢ (FULL CONVERGENCE)**  
> Agent trước đó đã triển khai **chính xác từng dòng mã, đúng từng cấu trúc dữ liệu, thỏa mãn toàn bộ 5 Mục tiêu (G1–G5), 3 Kịch bản người dùng (CUJ 1–3) và các ràng buộc phi chức năng (NFR)**.  
> Không phát hiện bất kỳ sai lệch nào ngoài ý muốn (accidental deviation) hay vi phạm quy chuẩn dự án (`CODING_STANDARDS.md`).

---

## 2. Ma Trận Đối Chiếu Chi Tiết 5 Mục Tiêu (G1 – G5)

| Mục tiêu đặc tả | Nội dung yêu cầu trong Spec | Hiện trạng mã nguồn thực tế | Đánh giá |
|---|---|---|---|
| **G1: SSoT Restoration & Fallback** | 1. `pc_to_binary`: gán `round: 0`, không leak `pc.state` (visual ID).<br>2. `skill_round`: thêm `.filter(\|&r\| r > 0)`, fallback sang `damage::get_turn(skill_id, skill_lv)`.<br>3. `binary_skill_input`: gán `round` qua fallback `damage::get_turn(skill_id, 1)` khi `def.round == 0`. | • [`skill_pc.rs:309`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/data/loaders/skill_pc.rs#L309): gán `round: 0` kèm ghi chú kỹ thuật rõ ràng.<br>• [`runner.rs:123-128`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L123-L128): `skill_round` có `.filter(\|&r\| r > 0)`.<br>• [`runner.rs:174-178`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L174-L178): `binary_skill_input` fallback `damage::get_turn(skill_id, 1)`. | **KHỚP 100%** |
| **G2: Duration Parity** | 1. Băng phong (11014): 5 lượt.<br>2. Cây tinh (10004): 3 lượt.<br>3. Kính (10015): 2..4 lượt.<br>4. Bỏ độc (14015): 3 lượt.<br>5. Hỗn loạn (14021): 2..6 lượt.<br>6. Fallback `_ => 3` phát `tracing::debug!`. | • [`damage.rs:226-316`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs#L226-L316): Bảng tra cứu `get_turn` khớp chính xác từng giá trị.<br>• [`damage.rs:312-315`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/damage.rs#L312-L315): Có `tracing::debug!(skill_id = id_skill, ...)`.<br>• Test case 1 assert literal từng skill & level. | **KHỚP 100%** |
| **G3: Anti-Stacking & Multi-Channel** | 1. Không ghi đè / không làm mới lượt khi đã dính hiệu ứng cùng kênh (Type 3, 4, 15), trả về `MISS`.<br>2. Cho phép đa trạng thái tồn tại song song giữa các kênh khác nhau (Type 3 + 4 + 15 + 19). | • [`runner.rs:1826-1837`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L1826-L1837) (Type 3), [`runner.rs:1956-1960`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L1956-L1960) (Type 4), [`runner.rs:2600-2611`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L2600-L2611) (Type 15): Kiểm tra `id > 0` trả về `miss_status::MISS`.<br>• Test case 2 & 3 kiểm thử độc lập cho cả 3 kênh và đa kênh. | **KHỚP 100%** |
| **G4: Auto-Dispel Broadcast Parity** | Khi `turn <= 1` ở đầu lượt:<br>1. Reset ID, Lv, Turn về 0.<br>2. Hoàn trả chỉ số AGI nếu có.<br>3. Broadcast `0x35 0x01` (Purge) trước, sau đó `0x35 0x0F` (Clear HUD). | • [`runner.rs:534-610`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L534-L610): Reset fields, emit `purge_status` rồi `clear_unit_status`.<br>• [`battle_buff_debuff_lifecycle_test.rs:467-476`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/battle_buff_debuff_lifecycle_test.rs#L467-L476): Assert thứ tự Purge trước Clear.<br>• Phủ toàn diện cả Type 3/4/19 trong `battle_timing_wiring_test.rs`. | **KHỚP 100%** |
| **G5: Comprehensive Test Suite** | Tạo tệp `tests/battle_buff_debuff_lifecycle_test.rs` với 5 test case xác định (deterministic), in-memory, không I/O DB, runtime < 100ms. | • Tệp `tests/battle_buff_debuff_lifecycle_test.rs` chứa đúng 5 test case.<br>• Dùng `Battle::with_seeds` (RNG cố định).<br>• Thời gian chạy thực tế: **0.00s** (< 100ms NFR).<br>• Kết quả chạy: **5/5 pass**. | **KHỚP 100%** |

---

## 3. Kiểm Tra Mã Nguồn Chi Tiết Theo Từng Tệp

### 3.1. Static Loader: `src/data/loaders/skill_pc.rs`
*   **Spec yêu cầu (Mục 4.A & Plan 1.2)**: `round: 0`, giải thích rõ `pc.state` là mã diễn hoạt thị giác (visual sprite animation ID) của Delphi client (`0x35 0x01`), không phải số lượt chiến đấu.
*   **Thực tế mã nguồn**:
    ```rust
    // src/data/loaders/skill_pc.rs:309-312
    // Note: PC Skill.Dat record has no round field. pc.state represents the client
    // visual sprite animation ID (e.g. 1=Ice, 3=Tree, 103=Mirror, 171=Poison). Do NOT map
    // pc.state to round; combat turn durations are determined by damage::get_turn.
    round: 0,
    ```
    Tại docstring hàm `pc_to_binary` (dòng 281-285) cũng đã được cập nhật đồng bộ, xóa bỏ giả định sai trước đó về `state -> round`.

### 3.2. SSoT Duration Rules: `src/battle/damage.rs`
*   **Spec yêu cầu (Mục 4.B, Open Question 1, Plan 1.3)**: `damage::get_turn` đóng vai trò nguồn chân lý duy nhất (SSoT). Nhánh wildcard `_ => 3` phải thêm log `tracing::debug!`.
*   **Thực tế mã nguồn**:
    ```rust
    // src/battle/damage.rs:312-315
    _ => {
        tracing::debug!(skill_id = id_skill, "uncataloged skill hit default turn fallback (3)");
        3
    }
    ```
    Giữ nguyên các nhóm chuẩn (GROUP_a .. GROUP_f) và các nhánh thang bậc (14021 Hỗn loạn: Lv 1..5 -> 2..6 lượt).

### 3.3. Battle Execution Engine: `src/battle/runner.rs`
*   **Spec yêu cầu (Mục 4.C & Plan 1.1)**:
    1. `skill_round`:
       ```rust
       fn skill_round(data: &BattleData<'_>, skill_id: i64, skill_lv: i64) -> i64 {
           binary_skill_def(data, skill_id)
               .map(|skill| i64::from(skill.round))
               .filter(|&r| r > 0)
               .unwrap_or_else(|| damage::get_turn(skill_id, skill_lv))
       }
       ```
    2. `binary_skill_input`:
       ```rust
       round: if def.round > 0 {
           i64::from(def.round)
       } else {
           damage::get_turn(skill_id, 1)
       },
       ```
*   **Thực tế mã nguồn**:
    Cả 2 vị trí ([`runner.rs:123-128`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L123-L128) và [`runner.rs:174-178`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L174-L178)) khớp nguyên văn (verbatim) với mã nguồn mẫu trong đặc tả.

### 3.4. Bộ Kiểm Thử: `tests/battle_buff_debuff_lifecycle_test.rs`
Spec yêu cầu đúng 5 test cases cụ thể:
1. `test_skill_round_table_parity`: Kiểm tra bảng tra cứu canonical và kiểm tra loader không leak `state`.
2. `test_anti_stacking_enforcement`: Kiểm tra chống dồn trạng thái kênh 3, kênh 4, kênh 15; trả về gói `MISS`.
3. `test_cross_channel_concurrency`: Kiểm tra tồn tại đồng thời 4 trạng thái (Băng phong + Kính + Bỏ độc + Bá ý), poison rút HP mỗi lượt, khi poison hết hạn chỉ purge riêng Type 15.
4. `test_attack_secondary_status_infliction`: Kỹ năng tấn công gây sát thương đồng thời gán hiệu ứng phụ (`hit_status > 0`) nhận đúng số lượt từ SSoT dù binary mang `round: 0`.
5. `test_turn_countdown_dot_and_auto_dispel_lifecycle`: Vòng đời từng lượt của Bỏ độc (Round 1: cast không mất máu; Round 2: giảm lượt và trừ 60 HP; Round 3: trừ 60 HP, auto-dispel phát `0x35 0x01` trước `0x35 0x0F`).

*Thực tế*: Cả 5 test case đều được cài đặt chuẩn xác, không có test phụ dư thừa, không có test bị bỏ sót.

---

## 4. Xác Minh Các Kịch Bản Người Dùng (CUJs)

*   **CUJ 1 (Casting Freeze & Auto-Dispel)**: Thể hiện tại `test_turn_countdown_dot_and_auto_dispel_lifecycle` và `test_skill_round_table_parity`: Băng phong nhận đúng 5 lượt, giảm trừ lượt theo vòng đời, giải trừ khi `turn <= 1`.
*   **CUJ 2 (Anti-Stacking Rejection)**: Thể hiện tại `test_anti_stacking_enforcement`: Mục tiêu đang chịu Cây tinh bị dính tiếp Băng phong trả về `miss_status::MISS`, không bị ghi đè hay làm mới lượt.
*   **CUJ 3 (Cross-Channel Debuff Concurrency)**: Thể hiện tại `test_cross_channel_concurrency`: Một thực thể nhận cùng lúc 4 hiệu ứng thuộc 4 kênh độc lập; khi kênh Độc hết hạn và bị thanh tẩy, 3 kênh còn lại vẫn duy trì nguyên vẹn.

---

## 5. Đánh Giá Các Ghi Chú & Bẫy Kỹ Thuật Nêu Trong Handoff

Tài liệu Handoff đã phản ánh rất trung thực hai điểm lưu ý kiến trúc:

1.  **Vấn đề `binary_skill_input` hardcode `damage::get_turn(skill_id, 1)`**:
    *   *Nguồn gốc*: Đúng nguyên văn yêu cầu đặc tả tại Mục G1 và Mục Technical Plan 4.C.
    *   *Hệ quả*: Kỹ năng đánh vật lý/phép có kèm hiệu ứng phụ (secondary status) sẽ luôn tính thời gian hiệu lực theo cấp độ 1 của debuff đó.
    *   *Đánh giá của Auditor*: Agent đã làm **đúng 100% theo bản đặc tả đã được phê duyệt**. Quyết định không tự ý sửa chữ ký của `binary_skill_input` (ảnh hưởng 4 call sites) khi chưa có ý kiến của User là hoàn toàn chuẩn xác theo quy tắc kỷ luật kỹ thuật (Scope discipline).
2.  **Bất đối xứng pre-existing `typeX_id > 1` vs `> 0`**:
    *   Trong `runner.rs` vòng lặp `turn_phase1`: Việc giảm lượt kiểm tra `typeX_id > 1`, trong khi dispel kiểm tra `typeX_id > 0`. Đây là mã nguồn có sẵn từ trước, không phải do agent tạo ra. Agent không tự ý can thiệp là hoàn toàn đúng phạm vi ticket.

---

## 6. Kết Quả Chạy Kiểm Thử Thực Tế

Đã thực hiện chạy kiểm thử trực tiếp trên môi trường:

```bash
cargo test --test battle_buff_debuff_lifecycle_test
```
*Kết quả:*
```text
running 5 tests
test test_skill_round_table_parity ... ok
test test_attack_secondary_status_infliction ... ok
test test_anti_stacking_enforcement ... ok
test test_cross_channel_concurrency ... ok
test test_turn_countdown_dot_and_auto_dispel_lifecycle ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Đã chạy kiểm thử hồi quy các phân hệ liên quan:
```bash
cargo test --test skill_job_system_test --test battle_timing_wiring_test --test architecture_guardrails_test
```
*Kết quả:* **30/30 passed; 0 failed.** Không có bất kỳ hồi quy nào.

---

## 7. Khuyến Nghị Tiếp Theo Cho User

1.  **Phê duyệt và Commit**: Toàn bộ thay đổi mã nguồn trong working tree cho 4 tệp (`src/data/loaders/skill_pc.rs`, `src/battle/damage.rs`, `src/battle/runner.rs`, `tests/battle_buff_debuff_lifecycle_test.rs`) đã hoàn chỉnh, sạch sẽ và sẵn sàng để commit thủ công.
2.  **Đề xuất Ticket Tiếp Theo (Tùy chọn)**:
    Nếu muốn hiệu ứng phụ của đòn đánh (secondary status) tỷ lệ theo cấp độ kỹ năng thay vì cố định cấp 1, mở ticket mới: *"Refactor `binary_skill_input` to pass `skill_lv` for dynamic status round calculation"*.
