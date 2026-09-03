# TS Dream Production Runbook

## Runtime contract

The game listener uses TCP port **6414** by default. The web administration dashboard uses HTTP port **8090** by default. Frames use the decoded magic header `F4 44`, little-endian length, and XOR key `0xAD` on the wire. The production live listener selects the **PC/aLogin profile**; golden replay tests use the Kotlin/mobile profile explicitly through `ServerEnv::none()`.

The protocol constants are `MIN_VERSION=186`, `ID_PREFIX=VN`, `SERVER_NAME=TSVN`, and `MAX_LEVEL=200`. The runtime rejects a missing or invalid production data bundle by leaving `data_loaded=false`; the server-control accept gate then refuses to start the game listener. This is safer than silently starting with an empty catalog.

## MySQL

Set `TS_DATABASE_URL` to a MySQL 8-compatible URL, for example:

```text
mysql://ts_user:change-me@127.0.0.1:3306/ts_dream
```

The default `TS_DB_AUTO_CREATE=true` attempts to create the database and uses the ordered migrations in `migrations/`. For managed environments, provision the database and set `TS_DB_AUTO_CREATE=false`. The schema keeps VISCII-bearing text columns in `latin1_bin`; do not convert those columns to `utf8mb4` without a wire-encoding migration. Migrations `0004_schema_runtime_mode.sql` and `0006_cutover_drop_legacy.sql` record and execute the schema cutover. After 0006, `active_backend=modern_0002`, `target_backend=modern_0002`, and `legacy_drop_safe=1`; the seven legacy tables are dropped only after the INSERT...SELECT copy in 0006.

## Data bundle

Set `TS_DATA_DIR` when the `Data/` directory is not adjacent to the process working directory. Production `GameData::load` admits only root-level files with `.dat`, `.emg`, or `.mng` extensions and requires `Item.dat` and `Npc.dat`. It does not load `.txt`, `.ini`, `.mmg`, or other legacy text/map assets in the production path. Accepted assets are hashed with SHA-256 and persisted to `data_asset_catalog`; the dashboard exposes them at `/api/data/assets`.

The typed mobile-compatible ports cover Skill, Mark, Mounts, MountsGrow, AchievementData, Dispatch, DispatchBonus, LeaderboardInfo, SceneSet, and TeachInfo. These new loaders use `StrictDatReader` and reject truncated records or invalid UTF-16LE lengths. The shipped PC `Skill.Dat` remains format-unverified relative to mobile `Skill_C.dat`; production activates the mobile decoder only for an explicitly supplied `Skill_C.dat`, never by guessing against `Skill.Dat`. `NpcDropLoader` is recorded as unavailable because its mobile source consumes JSON and no accepted PC binary asset exists. Ground `.mmg` remains excluded by policy.

## Build on Linux

```bash
rustup toolchain install stable
rustup component add rustfmt clippy
cargo fmt --all
cargo test --all-targets --no-fail-fast -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --target x86_64-unknown-linux-gnu
./target/x86_64-unknown-linux-gnu/release/ts_dream
```

## Build on Windows

Using the MSVC toolchain is recommended on a native Windows host:

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup component add rustfmt clippy --toolchain stable-x86_64-pc-windows-msvc
cargo build --release
```

A Linux host can produce the tested GNU Windows artifact with the MinGW linker:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

The server binary is `target/x86_64-pc-windows-gnu/release/ts_dream.exe`.

## Dashboard

Open `http://127.0.0.1:8090/`. The dashboard provides lifecycle controls, online-player status, account operations, announcements, NPC inspection, binary data-integrity hashes, guild read models, and world-boss read models. JSON endpoints include `/api/server/status`, `/api/accounts`, `/api/npcs`, `/api/data/assets`, `/api/guilds`, `/api/world-bosses`, and `/api/db/status`.

## Schema cutover status

The live Rust path now uses the injected modern repository bundle and `MySqlSessionRepository` for login, character lifecycle, autosave, battle-end persistence, item updates, pets, skills, hotkeys, and quests. Migration 0006 removes the seven legacy tables after copying their rows. A backup and a real MySQL 8 migration rehearsal remain mandatory before production execution.

## Checkpoint resume

Checkpoints are under `work/checkpoints/` in the project workspace. Resume from the highest numbered checkpoint only after reading its `Known limitations` and `Resume point` sections. Checkpoint `011_mobile_loader_ports.md` records the latest typed-loader scope. Evidence logs are under `work/evidence/`; the latest gate logs are `release_serial_tests.log`, `release_clippy.log`, `release_linux_build.log`, and `release_windows_build.log`. A real MySQL 8 rehearsal of destructive migration `0006_cutover_drop_legacy.sql` is still mandatory before production execution.
