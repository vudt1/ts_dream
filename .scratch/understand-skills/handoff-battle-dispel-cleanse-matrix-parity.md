# Handoff: Chuẩn Bị Triển Khai Hệ Thống Hóa Giải & Thanh Tẩy (Battle Dispel & Cleanse Matrix Parity)

*   **Ngày hoàn thành phiên:** 2026-11-04
*   **Tác giả bàn giao:** Antigravity Agent
*   **Trạng thái phiên:** Đã rà soát ticket trước, phát hiện và phân tích toàn diện lỗi hệ thống Dispel, hoàn tất Grilling với User, cập nhật `GLOSSARY.md`, tạo Spec và bộ 4 ticket thực thi chi tiết. Sẵn sàng 100% để agent tiếp theo bắt tay vào viết code.
*   **Tài liệu Spec gốc:** [`.scratch/understand-skills/spec-battle-dispel-cleanse-matrix-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/spec-battle-dispel-cleanse-matrix-parity.md)
*   **Báo cáo Rà soát trước đó:** [`.scratch/understand-skills/research-audit-battle-buff-debuff-lifecycle.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/research-audit-battle-buff-debuff-lifecycle.md)
*   **Thuật ngữ miền chuẩn:** [`GLOSSARY.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/GLOSSARY.md#L440) (Mục *Dispel Matrix / Ma Trận Hóa Giải Trạng Thái*)

---

## 1. Tóm Tắt Toàn Bộ Công Việc Đã Thực Hiện Trong Phiên

1.  **Rà soát toàn diện ticket trước (`battle-buff-debuff-lifecycle`)**:
    *   Đối chiếu mã nguồn thực tế với đặc tả và handoff: Đạt **100% yêu cầu đặc tả (G1–G5, NFR)**.
    *   Đã chạy kiểm thử: `battle_buff_debuff_lifecycle_test` (5/5 pass, 0.00s), `skill_job_system_test` (19/19 pass), `battle_timing_wiring_test` (6/6 pass), `architecture_guardrails_test` (5/5 pass). Toàn bộ 70 suites / 438 tests đều xanh.
    *   Đã lưu báo cáo rà soát tại [`.scratch/understand-skills/research-audit-battle-buff-debuff-lifecycle.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/research-audit-battle-buff-debuff-lifecycle.md).

2.  **Khảo sát, phát hiện và làm rõ lỗi nghiêm trọng trong phân hệ Dispel/Cleanse**:
    *   Hóa giải đặc hiệu trong [`src/battle/runner.rs:1998-2106`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs#L1998-L2106) (`apply_dispel5`) đang bị lỗi nghiêm trọng:
        *   **Băng tan (`11015`)**: Bị thiếu khỏi `match skill` (lại gõ nhầm ID `11014` của Băng phong và check `type4_id == 10010` Kết giới!). Khi thi triển, `11015` rơi vào wildcard `_ =>` và xóa sạch toàn bộ 4 kênh trạng thái như một phép Đại giải trừ toàn năng!
        *   **Giải hôn (`14007`)**: Kiểm tra `type4_id == 14008`, trong khi Hôn mê nằm ở `type3_id` $\to$ luôn trả về `MISS`, không bao giờ giải được Hôn mê!
        *   **Giải độc (`14014`)**: Kiểm tra `type4_id == 14015`, trong khi Bỏ độc nằm ở `type15_id` $\to$ luôn trả về `MISS`, không bao giờ giải được Bỏ độc!
        *   **Giải hỗn loạn (`14022`)**: Kiểm tra `type4_id == 10021`, sai cả ID (`10021` thay vì `14021`) và sai cả kênh (`type4_id` thay vì `type15_id`) $\to$ luôn trả về `MISS`!
    *   **Trạng thái giải trừ (`11012`, `11025`, `11031`)**: Rơi vào wildcard `_ =>` trong `apply_dispel5`, xóa mù quáng cả 4 kênh không kiểm tra phe, làm mất sạch cả Kính (`type4`) và Aura/Thiên thần thuẫn (`type19`) của đồng đội!
    *   **Nhóm Cleanse & Heal Type 18 (`20050` Cường hiệu giải trừ, `20015` Tứ nhân trạng giải, `20019` Giải thoát)**:
        *   Bị triệt tiêu cấp độ kỹ năng bằng `let _ = skill_lv;` tại dòng 2835. Lượng hồi HP/SP bị gán cứng `int1 * 0.5`, không hề tăng khi nâng cấp từ Lv 1 lên Lv 5 hay Lv 10!
        *   Thiếu chốt chặn chống vòng lặp SP khi tự cast lên bản thân.

3.  **Thực hiện Grilling Session và chốt 4 quyết định nghiệp vụ với User**:
    *   **Q1**: Trọng giải (`11012`, `11025`, `11031`, `22005`) là **Friendly-only**; giải Debuff (Type 3 + 15), bảo toàn Buff (Type 4 + 19) của đồng đội; nhắm vào địch trả về `MISS`.
    *   **Q2**: Áp dụng chốt chặn an toàn `if attacker.id == target.id { sp = 0; }` cho Type 18: Caster tự giải nhận đủ HP và xóa debuff nhưng nhận 0 SP để chống exploit hồi SP vô hạn.
    *   **Q3**: Phân định Clean Target: Type 18 có hồi máu/SP nên dù mục tiêu không có debuff vẫn thi triển `ATTACK` và hồi HP/SP; Type 5 (không hồi máu) nếu mục tiêu không có debuff thì trả về `MISS`.
    *   **Q4**: Thống nhất tách module miền chuyên biệt `src/battle/dispel.rs` (`ts_dream::battle::dispel`) để quản lý tập trung toàn bộ ma trận Dispel.
    *   **Công thức hồi phục Type 18**:
        *   Thủy thần (`11016..=11019`): base 400..700 HP + `skill_lv * 30`, base 100..250 SP + `skill_lv * 15`.
        *   Cường hiệu giải trừ (`20050`, max Lv 5): `round(int1 * 0.40) + skill_lv * 40` HP, `round(int1 * 0.20) + skill_lv * 20` SP.
        *   Tứ nhân trạng giải (`20015`, max Lv 10) & Giải thoát (`20019`, max Lv 10): `round(int1 * 0.35) + skill_lv * 25` HP, `round(int1 * 0.15) + skill_lv * 12` SP.

4.  **Cập nhật GLOSSARY.md**:
    *   Bổ sung mục `Dispel Matrix / Ma Trận Hóa Giải Trạng Thái` tại [`GLOSSARY.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/GLOSSARY.md#L440).

5.  **Ban hành Spec và Bộ 4 Ticket**:
    *   Spec hoàn chỉnh: [`.scratch/understand-skills/spec-battle-dispel-cleanse-matrix-parity.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/spec-battle-dispel-cleanse-matrix-parity.md).
    *   Bộ 4 ticket tại `.scratch/understand-skills/issues/` (đã được User duyệt):
        1.  [`11-core-dispel-domain-and-specific-counters.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/11-core-dispel-domain-and-specific-counters.md) (Frontier, có thể làm ngay)
        2.  [`12-mass-cleanse-friendly-buff-preservation.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/12-mass-cleanse-friendly-buff-preservation.md) (Blocked by 11)
        3.  [`13-cleanse-heal-scaling-and-sp-loop-gate.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/13-cleanse-heal-scaling-and-sp-loop-gate.md) (Blocked by 11)
        4.  [`14-runner-integration-and-legacy-cleanup.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/14-runner-integration-and-legacy-cleanup.md) (Blocked by 12, 13)

---

## 2. Trạng Thái Hiện Tại Của Codebase & Working Tree

*   **Working tree**:
    *   4 tệp từ ticket trước: `src/battle/damage.rs`, `src/battle/runner.rs`, `src/data/loaders/skill_pc.rs`, `tests/battle_buff_debuff_lifecycle_test.rs` (User tự commit thủ công theo quy chuẩn repo; **Agent KHÔNG tự ý git commit**).
    *   Tệp `GLOSSARY.md` vừa được cập nhật thêm thuật ngữ Dispel Matrix.
    *   Các tệp tài liệu trong `.scratch/understand-skills/`.
*   **Trạng thái biên dịch & test**:
    *   `cargo check`: Clean.
    *   `cargo test --test battle_buff_debuff_lifecycle_test`: 5/5 pass (0.00s).
    *   Toàn bộ test suite liên quan (`skill_job_system_test`, `battle_timing_wiring_test`, `architecture_guardrails_test`): 30/30 pass.

---

## 3. Hướng Dẫn Chi Tiết Cho Agent Tiếp Theo

### Bước 1: Tiếp nhận Frontier — Ticket 11
Agent tiếp theo cần bắt đầu ngay với **[Ticket 11: Core Dispel Domain Module & Specific Status Counters](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/understand-skills/issues/11-core-dispel-domain-and-specific-counters.md)**:

1.  **Tạo module mới `src/battle/dispel.rs`**:
    *   Khai báo trong `src/battle/mod.rs`: `pub mod dispel;`.
    *   Định nghĩa struct `DispelExecutionOutcome`:
        ```rust
        pub struct DispelExecutionOutcome {
            pub hit: bool,
            pub purged_type3: bool,
            pub purged_type4: bool,
            pub purged_type15: bool,
            pub purged_type19: bool,
            pub hp_healed: i64,
            pub sp_restored: i64,
            pub broadcast_packets: Vec<String>,
        }
        ```
    *   Viết hàm xử lý Hóa giải đặc hiệu (Specific Counters):
        *   `11015` (Băng tan): kiểm tra `target.type3_id == 11014`. Nếu đúng: `target.type3_id = 0`, emit `PURGE_TYPE_DISABLE` + `STATUS_SEAL_RESIST_PCT`.
        *   `14007` (Giải hôn): kiểm tra `target.type3_id == 14008`. Nếu đúng: `target.type3_id = 0`, emit `PURGE_TYPE_DISABLE` + `STATUS_SEAL_RESIST_PCT`.
        *   `14014` (Giải độc): kiểm tra `target.type15_id == 14015`. Nếu đúng: `target.type15_id = 0`, emit `PURGE_TYPE_DEBUFF` + `STATUS_DEF_MODIFIER` / `ATK`.
        *   `14022` (Giải hỗn loạn): kiểm tra `target.type15_id == 14021`. Nếu đúng: `target.type15_id = 0`, emit `PURGE_TYPE_DEBUFF` + `STATUS_DEF_MODIFIER` / `ATK`.
        *   `10009` (Giải kết giới): kiểm tra `target.type4_id == 10010`. Nếu đúng: `target.type4_id = 0`, emit `PURGE_TYPE_BUFF` + `STATUS_DEF_MODIFIER`.
        *   `10014` (Giải trừ kính): kiểm tra `target.type4_id == 10015`. Nếu đúng: `target.type4_id = 0`, emit `PURGE_TYPE_BUFF` + `STATUS_DEF_MODIFIER`.
        *   Mọi trường hợp không khớp hoặc mục tiêu sạch debuff: trả về `hit = false` (`miss_status::MISS`) và không phát sinh broadcast packet nào.

2.  **Tạo test tích hợp mới `tests/battle_dispel_cleanse_test.rs`**:
    *   Viết test in-memory với seeded RNG (`Battle::with_seeds`).
    *   Kiểm tra 6 cặp hóa giải đặc hiệu thành công.
    *   Kiểm tra các negative test: cast lên mục tiêu không có debuff hoặc sai debuff $\to$ assert nhận frame `miss_status::MISS` và cell state không bị thay đổi.

3.  **Kiểm tra thứ tự gói tin wire (ADR 0006)**:
    *   Luôn broadcast `purge_status(0x35 0x01)` **trước** `clear_unit_status(0x35 0x0F)`.
    *   Chốt chặn: Tuyệt đối không thêm `#[cfg(test)]` vào `src/` (tuân thủ `tests/architecture_guardrails_test.rs`).

### Bước 2: Triển khai lần lượt Ticket 12 và Ticket 13
*   Sau khi Ticket 11 hoàn thành:
    *   Mở khóa Ticket 12: Trọng giải diện rộng (Friendly-only + Buff preservation).
    *   Mở khóa Ticket 13: Type 18 (Cleanse + Heal công thức scale theo `skill_lv`, chốt chặn SP loop `attacker.id == target.id`).

### Bước 3: Triển khai Ticket 14 (Tích hợp & Dọn dẹp)
*   Nối toàn bộ vào `src/battle/runner.rs`, xóa code legacy, chạy gate toàn diện `cargo test --all-targets --no-fail-fast`.

---

## 4. Suggested Skills (Kỹ Năng Đề Xuất Cho Agent Kế Tiếp)

*   **`/implement`**: Dùng để bốc Ticket 11 từ `.scratch/understand-skills/issues/11-core-dispel-domain-and-specific-counters.md` và tiến hành code.
*   **`/tdd`**: Áp dụng quy trình Red $\to$ Green $\to$ Refactor khi viết module `src/battle/dispel.rs` và `tests/battle_dispel_cleanse_test.rs`.
*   **`/code-review`**: Chạy review 2 trục (Standards & Spec) sau khi hoàn thành code trước khi bàn giao lại cho User.
*   **`/test-audit`**: Kiểm toán chất lượng các test case mới, đảm bảo runtime < 100ms, không I/O DB, không flaky.
