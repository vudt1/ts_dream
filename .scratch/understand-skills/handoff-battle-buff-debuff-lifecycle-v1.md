# Handoff: Triển Khai Ticket Battle Buff & Debuff Lifecycle (Băng Phong / Kính / Bỏ Độc / Hỗn Loạn)

**Ngày hoàn thành:** 2026-11-03  
**Nhánh:** working tree (người dùng tự commit thủ công theo quy chuẩn repo; agent KHÔNG commit)  
**Trạng thái:** Triển khai xong toàn bộ ticket, full test suite xanh (70 suites / 438 tests). Chưa commit — chờ user duyệt & commit.  
**Tài liệu tham chiếu:**
- Ticket gốc (đã approve): [`.scratch/understand-skills/battle-buff-debuff-lifecycle.md`](.scratch/understand-skills/battle-buff-debuff-lifecycle.md)
- Code review 2 trục đã chạy: xem mục 6 (không lưu report riêng)
- ADR liên quan: [`docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md`](../../docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md), [`docs/adr/0002-pc-server-mobile-formula.md`](../../docs/adr/0002-pc-server-mobile-formula.md)

---

## 1. Tóm Tắt Kết Quả Triển Khai

Ticket khắc phục lỗi duration bị hỏng do `pc.state` (visual sprite ID của Delphi client) bị nạp nhầm vào `BinarySkillDef.round` (Băng phong 11014 chỉ trụ 1 lượt, Kính 10015 tới 103 lượt, Bỏ độc 14015 tới 171 lượt). Giải pháp (Alternative A trong ticket):

- **Loader PC** (`src/data/loaders/skill_pc.rs`): `pc_to_binary` pin `round: 0` + comment giải thích; doc mapping cập nhật.
- **SSoT duration** (`src/battle/damage.rs`): `get_turn(skill_id, lv)` là nguồn chân lý duy nhất; wildcard fallback `_ => 3` thêm `tracing::debug!` (ticket Open Question 1).
- **Runner** (`src/battle/runner.rs`): `skill_round` thêm `.filter(|&r| r > 0)`; `binary_skill_input` resolve `round` qua `damage::get_turn(skill_id, 1)` khi `def.round == 0` (bảo vệ secondary status infliction trong `combat_formula.rs:107`).
- **Test mới** (`tests/battle_buff_debuff_lifecycle_test.rs`): đúng 5 test case theo ticket (table parity + loader hygiene, anti-stack MISS ×3 kênh, cross-channel concurrency, attack secondary status, countdown → DOT → auto-dispel). Chạy in-memory, seeded `Battle::with_seeds`, không DB/I-O.

Mutation check: revert 2 hunk trong `runner.rs` → 3/5 test FAIL như kỳ vọng; khôi phục lại → toàn xanh.

## 2. Danh Mục Tệp Thay Đổi (Changeset Manifest)

| Tệp tin | Loại thay đổi | Chi tiết |
|---|---|---|
| `src/data/loaders/skill_pc.rs` | Cập nhật | `round: pc.state` → `round: 0` + comment; cập nhật doc mapping (`state` = visual sprite ID, không bao giờ là round) |
| `src/battle/damage.rs` | Cập nhật | Wildcard `_ => 3` kèm `tracing::debug!` |
| `src/battle/runner.rs` | Cập nhật | `skill_round` filter `round > 0`; `binary_skill_input` fallback `get_turn(id, 1)` khi `def.round == 0` |
| `tests/battle_buff_debuff_lifecycle_test.rs` | Tạo mới | 5 test case (xem ticket §Detailed Implementation Plan #4) |

Kiểm tra diff: `git diff HEAD -- src/battle/damage.rs src/battle/runner.rs src/data/loaders/skill_pc.rs`

Lưu ý: working tree còn nhiều thay đổi **KHÔNG thuộc ticket này** (Data/, .scratch/, docs/, spec/, TESTS.md deleted…) — tuyệt đối không gộp vào commit của ticket. Chỉ stage đúng 4 tệp trên.

## 3. Kết Quả Kiểm Thử (Verification Summary)

- `cargo check`: clean.
- `cargo test --test battle_buff_debuff_lifecycle_test`: **5/5 pass** (< 100ms, NFR đạt).
- `cargo test --test skill_job_system_test --test battle_timing_wiring_test --test architecture_guardrails_test --test skill_dat_rebuild_test --test battle_status_frames_test --test battle_lifecycle_test --test battle_combo_rules_test --test battle_rng_determinism_test`: tất cả pass.
- **Full gate**: `cargo test --all-targets --no-fail-fast` → **70 suites, 438 tests, 0 failed** (exit 0).
- Byte-exact wire parity cho `0x35 0x01` / `0x35 0x0F` đã có ở `tests/battle_status_frames_test.rs`; purge-trước-clear-ordering assert trong test mới + `battle_timing_wiring_test.rs`.

## 4. Cơ Chế & Bẫy Môi Trường Cho Agent Kế Tiếp

1. **`run_turn` trả sớm `Outcome::PlayerWin`** khi rows 0–1 (phe 2) không có unit sống nào — Phase 1 (decrement/DOT/dispel) KHÔNG chạy. Mọi battle fixture phải có ít nhất 1 unit team-2 còn sống, nếu không test sẽ "pass" sai (turn không bao giờ giảm).
2. **Phase 1 chạy TRƯỚC khi xử lý lệnh cast trong cùng round**: decrement `typeX_turn -= 1` xảy ra trước khi skill được thi triển. Khi seed trạng thái sẵn có, phải seed `typeX_turn >= 3` nếu không muốn auto-dispel (`turn <= 1`) purge trước khi cast — nếu không, anti-stack test sẽ sai kết quả.
3. **`hit_status` là `u8`** (mã status nhỏ của PC client; `6` = ICE_BOUND/frozen, xem `ICE_BOUND_STATUS_ID` trong `src/battle/combat_formula.rs`) — không bao giờ chứa skill id (vd 11014). Với debuff thuần, để `hit_status: 0` thì apply_status3 dùng `unwrap_or(skill)` làm `type3_id`.
4. **PC skill catalog cho test**: dùng `BattleData::new(...).with_binary_skills(&map)` + `BinarySkillDef { round: 0, attribute: <skill_type>, ..Default::default() }`. `attribute` quyết định nhánh xử lý: 1=physical, 3=Type3, 4=Type4, 15=Type15, 19=Type19. Landing roll 100% khi `atk/int1` đủ lớn (formula: `30 + max/30`, clamp 100).
5. **`Battle::with_seeds(id, diahinh, s0, s1, s2)`** cho RNG deterministic; `random_0` (troi rolls) và `random_damage` (combat formula: hit→factor→thunder→status) là các stream riêng. `turn_order` sort theo `attacked DESC, agi DESC, random DESC`.
6. **DOT chạy TRƯỚC dispel check** trong Phase 1: round mà `turn` giảm về `<= 1` vẫn gây damage/DOT rồi mới purge (Bỏ độc: `30 + lv * 15`; Cây tinh 10004: `10 + lv*2`; 10033: `30 + lv*10`).
7. **`typeX_id > 1` mới decrement, nhưng dispel guard `typeX_id > 0`** — trạng thái id == 1 sẽ không bao giờ giảm turn/hết hạn (xem mục 5-2).
8. Test canvas chuẩn: `battle.cell_mut(r, c)` để seed, `battle.cell(r, c)` để assert; frame assert qua `Out::Broadcast` (helper `broadcast_frames` trong test file); `skilling_int(...)` cho payload nhúng trong khung `32 01`.

## 5. Việc Tồn Đọng / Cần User Triage

1. **`binary_skill_input` hardcode `get_turn(skill_id, 1)`** (đúng verbatim spec đã approve): secondary status của attack skill luôn lấy duration theo Lv 1, không theo `skill_lv`. Với các skill có thang level (vd 14021: 2→6 lượt), debuff kèm đòn đánh sẽ không scale theo cấp. Fix = thread `skill_lv` vào `binary_skill_input` (đổi signature ~4 call sites) — **cần user quyết vì lệch spec đã duyệt**.
2. **Bất đối xứng pre-existing**: decrement guard `typeX_id > 1` vs dispel guard `> 0` (`src/battle/runner.rs` turn_phase1, ~L455-468 vs ~L529-605). Status có id == 1 không bao giờ expire. Ngoài phạm vi ticket — đề xuất ticket riêng.
3. **G4 (auto-dispel broadcast) cho Type 3/4/19** chỉ được cover gián tiếp qua `battle_timing_wiring_test.rs` (test phase1 purge/clear cả 4 kênh); test mới chỉ assert chi tiết cho Type 15 (đúng phạm vi ticket case 5). Nếu muốn, bổ sung 3 case nhỏ tương tự — **không tự ý thêm khi chưa được duyệt** (ticket quy định đúng 5 test).
4. Các judgement-call của reviewer đã cân nhắc và bỏ qua (có lý do): extract `resolve_round` helper (spec quy định 2 spelling), table-drive fixture (đúng idiom repo), `contains()` cho payload nhúng trong `32 01` (frame rời đã assert byte-exact).

## 6. Kết Quả Code Review 2 Trục (Tóm Tắt)

- **Standards:** không vi phạm documented standard (tests/ đúng chỗ, không `#[cfg(test)]` trong src/, không DB, wire parity ADR 0006 giữ nguyên — purge trước clear). 1 đề xuất "probable bug" = mục 5-1 (spec-mandated, chuyển user quyết).
- **Spec:** G1 ✓, G2 ✓ (11014→5, 10004→3, 10015→2/3/4, 14015→3, 14021→2..6, 10010→2..5 — assert literal trong test), G3 ✓ (MISS + không refresh/overwrite; cross-channel 4 trạng thái cùng tồn tại, purge đúng phạm vi Type 15), G4 ~ (Type 15 đầy đủ; 3/4 đã có ở timing-wiring test), G5 ✓ (đúng 5 test, in-memory, seeded). Scope creep: không.

## 7. Lệnh Xác Minh Nhanh

```bash
cargo test --test battle_buff_debuff_lifecycle_test -- --nocapture
cargo test --test battle_timing_wiring_test --test skill_job_system_test --test architecture_guardrails_test
cargo test --all-targets --no-fail-fast   # gate trước handoff (CODING_STANDARDS §3.6)
```

## 8. Suggested Skills

- **`/code-review`** — chạy lại nếu có thêm thay đổi từ mục 5 trước khi user commit.
- **`/implement`** — nếu user duyệt mục 5-1 (thread `skill_lv`) hoặc ticket follow-up mục 5-2.
- **`/tdd`** — cho ticket fix mục 5-1/5-2 nếu đi theo red-green-refactor.
- **`/test-audit`** — nếu nghi ngờ chất lượng test hiện có (vd fixture boilerplate ×4) hoặc trước khi thêm case G4 (mục 5-3).
- **`/diagnosing-bugs`** — nếu phát hiện hồi quy duration/DOT sau khi merge.
- **`/handoff`** — tạo handoff tiếp theo sau khi user commit ticket này.
