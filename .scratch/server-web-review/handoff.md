# Handoff — Web Dashboard / Account / Login review → Password policy + op 0x23

**Date:** 2026-09-07
**Branch:** `ss2` — **2 commits ahead of `origin/ss2`, NOT pushed**: `29aa55a` (password_hash cleanup) + `38719dd` (password policy + op 0x23).
**Session arc:** `/code-review` (flow review) → schema cross-check → opcode 0x01 login wire trace → drop dead `accounts.password_hash` (`29aa55a`) → `/grilling` on the encoding-parity risk → implement ASCII password policy + wire op 0x23 account-management (`38719dd`).
**Next-session focus (per user):** continue reviewing the rest of the web module / login path; verify the newly-live op 0x23 sub-flows against a live DB.

---

## 1. Flow as analysed (reference — do not re-derive)

- **Boot → dashboard:** `src/main.rs:71-83` (config + MySQL bootstrap fail-fast + `migrate`) → `src/main.rs:151-171` binds `0.0.0.0:8090` (`web_port` default `src/config.rs:29`, env `TS_WEB_PORT`) → `axum::serve`.
- **Create account:** form `templates/dashboard.html` → `hx-post /api/accounts` → `create_account` `src/web/app.rs:359` → `db::accounts::create` `src/db/accounts.rs:38` → `INSERT INTO accounts (pass1, pass2, created_at, updated_at)`. `player_id` = MySQL `AUTO_INCREMENT` from **300000** (`0001_init.sql:6,18`). Passwords **plaintext**.
- **Login (opcode 0x01):** `src/server/handlers/login.rs:25`. Payload = `decoded[6..]` (`dispatcher.rs:189`): `[0..4]` acc_id u32 LE · `[4..6]` prefix `"VN"` (`ID_PREFIX`) · `[6..8]` version u16 LE (gate `>= MIN_VERSION 186`) · `[8..]` **password = raw bytes to end of packet** (no length prefix / terminator). Frame XOR `0xAD` on wire, decoded pre-dispatch. Verify: `login.rs:147` → `src/db/modern/mysql/accounts.rs:150` → `HEX(pass1)=HEX(?)` byte-exact. `pass2` is NOT a login gate.
- **Change password (opcode 0x23 sub 1):** `src/server/handlers/system.rs:199` `handle_account_mgmt`. Payload = 4 length-prefixed strings `old1,new1,old2,new2` (`parse_len_strings`, `system.rs:430`). Responses: `F4440300230101` success / `…02` old1 wrong / `…03` old2-wrong-or-generic-fail.
- **Golden evidence:** `golden/04-login-success.golden` = id 300001 / `vn` / 186 / `12345`; `golden/05-login-wrong-pass.golden` = `WRONG`; `golden/06-create-char.golden` create-char packet carries **empty** `pass1` (pass1_len=0). (Test fixtures, not real credentials.)

---

## 2. Facts established this session

- **`accounts.password_hash` was dead** (0 refs in `src/`, all named-column queries) → removed in `29aa55a`. `admin_users.password_hash` (`0003:213`) left intact (different table, admin-auth design).
- **Character creation does NOT set the password** — `create_and_seed` (`src/db/modern/mysql/session.rs:263`) only inserts the character + saves session; the old `character.rs` comment claiming it wrote `accounts.pass1/pass2` was false (fixed in `38719dd`).
- **op 0x23 was unreachable on PC** — `dispatcher.rs` routed it to `unimplemented` because the PC opcode table labels 0x23 "Guild". The C# string report (`spec/Bản hiệu đính…md:210-215`) proves 0x23 sub 1/2/3 = change-password / delete-char / gift-code. "Guild" is a misnomer → now wired (see §3).

---

## 3. Grilling outcome — resolved design tree (decisions, not re-derivable)

Password policy applies to **both `pass1` and `pass2`**:
- Length **8–10 bytes**, every byte in **`0x21..=0x7E`** (printable ASCII; no space/control; never UTF-8/VISCII) → guarantees the `HEX(pass)=HEX(?)` login compare round-trips.
- Default at creation = **`1111111111`** (10 ASCII chars).
- Change mechanism = **op 0x23 sub 1** (wired into the dispatcher).
- Validation enforced **server-side at BOTH** the dashboard create handler **and** op 0x23 change; client-side `maxlength`/`pattern` is UX only.
- Invalid NEW password (op 0x23) → reuse generic fail `F4440300230103` (no banner; the PC "too short" result byte is not recoverable from the repo).
- `handle_account_mgmt` old-cred compare switched to **byte-exact** (dropped `String::from_utf8_lossy`).
- **No** force-change-on-first-login (accepted residual risk: shared default is guessable until the player changes it).
- Docs: `CONTEXT.md` updated (Account policy + 0x23 reassign); **no ADR** (user declined).

---

## 4. Changes committed

### `29aa55a` — drop unused accounts.password_hash; fix stale migration refs
- `migrations/0001_init.sql` — removed dead `password_hash` column (edited 0001 directly; DB is fresh, user-authorised).
- `src/db/accounts.rs:1-6` — fixed stale "Migration 0007" comment.
- `AGENTS.md` — rewrote `0001_init.sql` tree line; deleted phantom `0002_modern_schema.sql` line.

### `38719dd` — enforce [8,10] ASCII password policy; wire op 0x23 account-management
- `src/db/accounts.rs:58` — new `is_valid_password(&[u8])` (`[8,10]` bytes, `0x21..=0x7E`).
- `src/web/app.rs:374` — `create_account` validates both passwords; invalid → **HTTP 400** (HTMX red `<tr>` / JSON `{"error"}`).
- `templates/dashboard.html:282-283` — pre-fill `value="1111111111"` + `maxlength="10"` + `pattern="[!-~]{8,10}"`.
- `src/server/dispatcher.rs:290` — `0x23 => system::handle_account_mgmt(ctx).await` (was `unimplemented`).
- `src/server/handlers/system.rs:216-238` — byte-exact `old1/old2` compare; validate `new1/new2` after old passes; invalid → `F4440300230103`.
- `src/server/handlers/character.rs:3-9` — corrected comment (create-char does not write the password).
- `CONTEXT.md:15` — password policy in Account invariants; `:208` — 0x23 removed from unimplemented list, reclassified as account-management.
- `golden/19-change-password.golden` — new wire-contract doc (see §6 caveat).

---

## 5. Verification performed

- `cargo check --all-targets` — clean after each commit (`29aa55a` and `38719dd`).
- Greps: `is_valid_password` defined + used in web & server; no `from_utf8_lossy(old…)` left; `0x23 =>` routes to handler; dashboard prefill present.
- No `cargo test` / no MySQL started (per `AGENTS.md` — DB-live checks are manual).

---

## 6. Open items / risks for next agent

- **Side effect of wiring 0x23 — VERIFY:** routing all of 0x23 to `handle_account_mgmt` also makes **sub 2 (delete character)** and **sub 3 (gift-code / TSVN123·TSVN456 newbie redeem)** live on PC for the first time. Confirm these behave correctly against a live DB (they were previously unreachable). `sub 3` writes items + flips `newbie`; `sub 2` tears down a character.
- **`golden/19` is doc-only, not runnable in-memory:** `handle_account_mgmt` shuts down when `pool` is absent and needs a logged-in `conn.session.id`, so the socketless scenario harness yields a shutdown, not the frames. Reproduce only against a live DB with an authenticated session.
- **Shared default password guessable** (Q11 accepted, no force-change): every new account is `1111111111` until the player changes it. Revisit if this becomes a real concern.
- **Deferred Standards — INTENTIONAL, do NOT "fix" unless the user reverses:** no auth/CSRF on the 8090 dashboard, plaintext passwords, error-swallowing elsewhere, no pagination on `list`. Spec `spec/production_architecture.md:51,70` requires admin auth + CSRF — accepted gap.
- **Manual DB verification pending:** fresh DB → `DESCRIBE accounts;` (no `password_hash`) → create account via dashboard (default + a custom `[8,10]` ASCII, and one invalid to see the 400) → login client → change password in-game via 0x23 → re-login with the new password.
- **CONTEXT.md drift (carried over):** "Deferred Drift" (~line 176) says 0001 must not be edited; this session edited it under fresh-DB authorisation. Consider an ADR/note.
- **Stale comment in 0001 (~lines 41-50):** still references legacy `characters.account_id`/pouches (from `.scratch/repository-check/handoff.md` §5). Untouched.
- **Unrelated working-tree changes remain uncommitted:** `Data/BattleGate.txt`, `Data/NpcDropLoader.kt`, `Data/Warps.txt`, `spec/TS_Server_OP_Code.md` (modified) + untracked `ts_mobile_server/`, `examples/`, `.scratch/repository-check/`, etc. Deliberately excluded from both commits.
- **Not pushed.**

---

## 7. References (read directly — not duplicated here)

- Commits: `git show 29aa55a`, `git show 38719dd`, `git log --oneline -5`.
- Web: `src/web/app.rs`, `src/web/server_control.rs`, `templates/dashboard.html`.
- Accounts: `src/db/accounts.rs`, `src/db/modern/mysql/accounts.rs`.
- Login / account-mgmt: `src/server/handlers/login.rs`, `src/server/handlers/system.rs`, `src/server/dispatcher.rs`, `src/protocol/mod.rs`.
- Schema: `migrations/0001_init.sql`, `migrations/0003_production_domain.sql`.
- Spec/standards: `spec/production_architecture.md` (§51, §70), `spec/Bản hiệu đính…md` (0x23 strings), `AGENTS.md`, `CONTEXT.md`.
- Prior handoffs: `.scratch/repository-check/handoff.md`, `.scratch/refactor-ts-server/handoff.md`.

---

## 8. Suggested skills for next session

- **`code-review`** — two-axis review of the newly-live op 0x23 sub-flows (delete-char, gift redeem) and the rest of the web module (`server_control.rs` lifecycle/stop-countdown, SSE `log_stream`/`db_status_stream`, `FormOrJson` extractor). Fixed point: `38719dd` (or `HEAD~2` for the whole session).
- **`domain-modeling`** — record the "0x23 = account-management (not Guild)" reassignment and the ASCII-password policy as ADRs under `docs/adr/` if the user later wants them formalised; reconcile the `CONTEXT.md` "Deferred Drift" edit-0001 exception.
- **`diagnosing-bugs`** — if the live-DB verification of op 0x23 sub 2/3 surfaces unexpected behaviour.
- **`grilling`** — if the shared-default-password risk or the deferred auth/CSRF gap is promoted into real work.
- **`handoff`** — again if the review continues in further sessions.

Do **not** scaffold tests or start MySQL unless the user lifts the `AGENTS.md` "no test / manual DB" constraint.

---

## 9. How to continue

1. `git log --oneline -3; git status --short` — confirm `38719dd` is HEAD and unrelated changes are still unstaged.
2. Run the live-DB verification in §6, prioritising the op 0x23 sub 2/3 side effects.
3. If promoting any deferred Standards item, confirm with the user first (intentional), then route via `to-spec`/`to-tickets`.
4. Push only when explicitly asked.
