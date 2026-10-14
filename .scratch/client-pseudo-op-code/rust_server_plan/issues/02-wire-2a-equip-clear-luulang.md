# 02 — Phase 2A: 17 0B → 10B, 1E 08 clear, 17 66 behind flag

Status: resolved
Blocked by: 01
Spec: `../05-phased-roadmap.md` Phase 2A (2A-1..2A-3), `../03-wire-codecs.md` §2, §3, §4

## What

- 2A-1 (G-01) `dump_trangbi` uses the 10-byte slotless layout, records sorted by `slot` ascending. Client evidence: `client_pseudo_c/0077cc2c_FUN_0077cc2c.*` (`IDIV 0xA`). `doben` is dropped from this frame (design decision, `03-wire-codecs.md` §2.3) — leave a TODO noting capture is needed.
- 2A-2 (G-08) Logined1 sends `1E 08` (empty body) immediately before `1E 01`. Client evidence: `client_pseudo_c/0076fdb8_FUN_0076fdb8.c`. Add a `clear_tientrang_frame()` helper.
- 2A-3 (G-02) New config `send_luulang_frame: bool` (default `false`, env/TOML like other config fields); `17 66` is only pushed into Logined1 when enabled. Keep `dump_luulang()` API.
- Update `tests/login_char_flow_test.rs` frame counts/assertions accordingly (see warning in `05-phased-roadmap.md` 2A-2 about the merged element).

## Acceptance

- `17 0B` body: `(len - 2) % 10 == 0`; test covers ordering by slot.
- Logined1 contains `1E 08` right before `1E 01`.
- No `17 66` frame when flag is off; present when on.
- `cargo test --all-targets --no-fail-fast` passes.
