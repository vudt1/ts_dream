# 10: RNG trận tất định (bỏ time_seeded trong runner)

**What to build:** Runner tạo `DotNetRandom::time_seeded()` mới ở hai nhánh resolve (skill 10016..10019) → kết quả mỗi lần chạy khác nhau, phá `with_seeds`/golden replay. Thay bằng RNG tất định thuộc battle (`BattleRng` hoặc stream riêng khởi tạo từ seed của battle), giữ nguyên thứ tự draw của các stream hiện có (drop/AI/damage), để hai trận cùng seed cho ra chuỗi frame giống hệt.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Không còn `time_seeded()` trên đường chạy battle runner
- [x] Nhánh 10016..10019 lấy số từ stream tất định của battle; ghi chú rõ dùng stream nào và không xê dịch draw của stream drop/AI/damage hiện có
- [x] Test determinism trong `tests/`: 2 run cùng seed → chuỗi frame giống hệt, gồm cả nhánh 10016..10019

**Ghi chú triển khai (ticket 10):**
- Thêm stream `BattleRng::random_skill`, khởi tạo tất định từ seed battle (`with_seeds` → `DotNetRandom::new(s0 + 12953)`; production `new()` vẫn time-seeded như các stream khác). Hai call-site `apply_physical`/`apply_magic` trong `runner.rs` dùng `self.rng.random_skill.next_range(1, 3)`.
- Stream `random_skill` hoàn toàn mới → seed và thứ tự draw của `random_0` (drop/skill pick), `random_1` (AI tie-breaker), `random_2` (respawn), `random_damage` (combat formula) **không đổi**. Test `existing_streams_keep_their_draw_order` ghim điều này (draw xen kẽ `random_skill` không làm lệch 4 stream cũ).
- Test: `tests/battle_rng_determinism_test.rs` — 2 trận cùng seed (nhánh 10016) ra chuỗi `Out` byte-identical; có thêm test đối chứng chứng minh nhánh 10016..10019 thực sự được chạy, và test guard source `runner.rs` không còn `time_seeded`.
- `cargo test --all-targets --no-fail-fast` → 277 passed / 0 failed.
