# 23 — Golden scenario suite + byte-level acceptance gate (Ch9)

**What to build:** Bộ capture-based golden đầy đủ các scenario deterministic (login/create-char/move/chat/mall-buy/use/warp/quest/1–2 battle/pet) để runner diff byte-exact từng frame — cổng chấp nhận cuối cho mọi feature handler. Các feature trên đã để sẵn chỗ golden; ticket này thu thập hoàn chỉnh và bật như gate CI.

**Blocked by:** 05 — Harness (proxy/runner/định dạng); và các feature có golden: 06 (create-char), 07 (login/wrong-pass), 08 (move), 09 (chat), 11 (inv/equip), 12 (use/warp), 14 (mall-buy), 16 (rank/points), 18–19 (talk/quest), 20–21 (battle), 17 (pet).

**Status:** completed

- [x] ~10–15 scenario golden file dưới `golden/`, mỗi cái versioned: login success / wrong password; create character; move; chat; buy from mall; use item; warp; quest (FTalk.H6 branch); 1–2 battle samples; pet (Ch9 §9.6).
- [x] Chỉ chọn scenario **deterministic**, loại/mark những frame phụ thuộc timing (020C timer) khỏi golden-lock (Ch9 §9.2/§9.5).
- [x] Runner xanh toàn bộ golden trên server Rust (cargo integration test) — **bất kỳ diff frame nào fail**.
- [x] Hằng số harness (XOR/magic/prefix/version/name) khớp Ch8.
- [x] Capture helper reproducible: chạy lại capture khi có regression; proxy validate định dạng.

## Comments

- 14 golden files under `golden/` (01-hello, 02-login-scaffold, 03-battle-win
  cũ; 04-login-success, 05-login-wrong-pass, 06-create-char, 07-move,
  08-chat, 09-mall-buy, 10-use-item, 11-warp, 12-quest-h6, 13-battle-leave,
  14-pet mới). Mỗi file là một scenario: `<<` C2S + `>>` S2C byte-exact.
- Runner: `tests/golden_suite.rs` (`golden_scenarios_replay_byte_exact`) —
  cargo integration test tự động xanh, không cần server/DB ngoài: replay qua
  `handler::dispatch` cho các feature handler và `BattleService` seeded cho
  battle-win. Bất kỳ diff frame nào fail. Định nghĩa scenario + replay dùng
  chung ở `tests/common/mod.rs`.
- Determinism: frame "Thoi gian" trong Logined1 là timing-dependent (Ch9
  §9.2) → thêm seam `spawn::override_now`/`reset_now` (UTC fixed instant)
  để login-success lock được toàn bộ chuỗi Logined1; còn lại mọi scenario
  đều pure function của session seed + GameData.
- Hằng số harness (XOR 0xAD, magic F4 44, prefix `vn`, version 186, name
  `TSVN`) đã có sẵn tại `src/harness.rs` (§8.2/§9.7) — không đổi.
- Capture lại khi có regression: `cargo test --test golden_suite -- --ignored
  regenerate_goldens` viết lại các golden sync từ code hiện tại; proxy capture
  đã có trong `harness::proxy` (ticket 05).