# 05 — MySQL 8 storage design (replaces SQLite)

Primary sources: MySQL 8.0 official docs (InnoDB, data types, charset), sqlx
docs (version 0.8 MySQL driver), `ts_server_old/CSDL/schema.sql`,
`ts_server_old/Data/shopp_schema.sql`, `ts_server_old/Server_TS_Online/Client.cs`
(opcode `0x23` sub 3 redeem-code MySQL branch), `Data/Member.ini`.

Context: the map's earlier decision (ticket 05 — "Thiết kế schema SQLite") stored
player state in two SQLite files: a shared `account.db` (Player + accounts) and
one `member/vn{id}.db` per character (9 gameplay tables), created by copying a
binary template + seeding `SkillSave`. The human redrew this: **MySQL 8 is the
official database**; **shared schema, all-in-one** database (no per-character
file/DB); MySQL runs **locally** (`localhost:3306`). This document records the
facts the MySQL schema decision (ticket 11) rests on.

## 1. sqlx MySQL driver

- Cargo feature set for async runtime: `sqlx = { version = "0.8", features =
  ["mysql", "runtime-tokio-rustls", "migrate"] }`. `migrate` enables embedded
  migrations (`sqlx::migrate!("./migrations")`) — schema is versioned in-repo,
  applied on boot.
- `MySqlPool::connect("mysql://user:pass@localhost:3306/ts_dream")`; also
  `MySqlPoolOptions::new().max_connections(n).connect(&url)`. The single
  `database_url` replaces the SQLite keys `account_db_path` / `member_dir` /
  `template_db_path` (that decision lived in ticket 07). Config key:
  `TS_DATABASE_URL` (env) / `database` in `ts_dream.toml`.
- MySQL is an external service: the "single binary" property is preserved, but
  the binary depends on a reachable mysqld. Local install assumed; ops steps
  (create database + user, dump/backup) are the human's side of ticket 11, not
  runtime work the binary must own.

## 2. Character set — byte-preservation (critical for fidelity)

- Wire protocol is VISCII bytes (research 03): the server round-trips single
  bytes; names/messages are raw VISCII, not Unicode.
- The repo's own install guide (`Huong_Dan_Cai_Dat_MySQL_ZIP.md`, MySQL 8.0.46
  ZIP, `my.ini`) sets `character-set-server=utf8mb4` as the server default —
  that default does NOT protect game columns; per-column/connection charset
  still has to be byte-preserving. The `ts_dream` database is created without
  an explicit charset, so it inherits `utf8mb4`. **Therefore the DDL MUST
  declare `CHARACTER SET latin1` (optionally `COLLATE latin1_bin`) explicitly on
  every game text column or on the table (`DEFAULT CHARACTER SET latin1`) — a
  bare `VARCHAR` falls back to utf8mb4, and VISCII high bytes (0x80–0xFF) are
  invalid UTF-8, so they would be transcoded/corrupted. `utf8/utf8mb4` is NOT a
  valid alternative for these columns: the wire encoding is VISCII, and utf8mb4
  is a byte-store only safe for metadata/dashboard, never game text. It is
  byte-preserving, not a "better Vietnamese" choice.
- MySQL's default `utf8mb4` actively **converts** and rejects invalid
  sequences; VISCII high bytes would corrupt or error. Two byte-safe storage
  strategies, to be decided in ticket 11:
  - `VARBINARY(n)` / `BINARY` — bytes as-is, no charset layer at all; async in
    sqlx reads as `Vec<u8>`.
  - `VARCHAR(n) CHARACTER SET latin1` — latin1 is 1 byte per char, no
    conversion; sqlx reads/writes opaque single-byte strings.
- Connection: with `MySqlConnectOptions` sqlx lets you set the connection
  `charset` (default `utf8mb4`); set `charset = latin1` so the client/server
  layer never transcodes the stored names.
- `Player.Color` (hex, ASCII) and all text columns share the same treatment.

## 3. Table layout — shared schema

| SQLite design (superseded, ticket 05) | MySQL 8 design (ticket 11) |
|---|---|
| `account.db`: `Player` + `accounts` | one database `ts_dream`: `players` + `accounts` |
| `member/vn{id}.db`: 9 gameplay tables | same 9 tables, shared; added `player_id` column, composite PK `(player_id, slot)` / `(player_id, stt)` / `(player_id, id)` per table |
| template binary copy + seed 1..10 `SkillSave` | character creation = one transaction of INSERTs (explicit `player_id`, seeded `SkillSave` 1..10 rows, `Skill` rebuilt at login as in C#) |
| PRAGMA `foreign_keys`/`WAL`/`busy_timeout` | InnoDB defaults; no FK (parity with Access); connection pool instead of WAL |

- Access `DOUBLE` columns (Homdo/LuuLang/Pet/TienTrang/Trangbi/Tuideo) →
  `BIGINT`, matching the SQLite decision "mọi cột số → INTEGER (i64)" — every
  observed value is integral.
- **Scoping contract (critical for shared schema)**: per-player isolation used
  to be implicit (one file per character); now every statement on the 9
  gameplay tables must carry `player_id` in its predicate or composite PK,
  or it hits other players' rows. Porting hazards if copied verbatim:
  `SkillSaveGetId` / `SkillSaveUpdateId` (`SELECT`/`UPDATE SkillSave WHERE
  Id=…`, Client.cs:8348/8360 — `SkillSave.Id` 1..10 repeats for every player),
  the login `Skill` DELETE-and-rebuild ranges (`DELETE FROM Skill WHERE Id
  >= 10001 AND Id <= 13033 …`, Client.cs:5726; `DELETE FROM Skill WHERE Id >= 0
  AND Id <= 9`, Client.cs:8193), and the `DELETE FROM Quest WHERE MapId…` sets
  (`FTalk.cs:789-955`). All must be rewritten with a `player_id` predicate.
- No FK / NOT NULL beyond Access, per old decision: C# never uses FKs
  (Homdo.Id can point at a nonexistent item, Pet.Idskill=0), adding them would
  block valid behavior.
- `accounts`: `accounts(id BIGINT AUTO_INCREMENT PRIMARY KEY, pass1 VARCHAR(...) NOT NULL, pass2 VARCHAR(...) NOT NULL)`. Import `Member.ini` (`[Account]`, `id=pass1\tpass2`) at bootstrap seeding explicit ids; auto-increment continues after the inserted max. Keep pass1/pass2 plaintext (parity, from 05).
- `SkillSave` seed 1..10 / IdSkill 0 (C# never INSERTs; it only UPDATEs — seed
  mandatory), now as seed rows at character creation.
- `item_code` table (MySQL-only origin, `Client.cs:7571-7659`, opcode `0x23` sub 3): columns observed `code`, `password`, `player_id`, `used_at` (unix seconds), `item_id`, `count`. This table was the C# MySQL surface and its branch currently "#degrade khi không có DB" (ticket 10) — with MySQL official it becomes fully functional and must exist in the migration.
- Indexes in MySQL: one index per old Access/User index kept: `Player(MapId)`,
  `Pet(IdSkill1..4)`, `Quest(QuestId)`, `SkillSave(IdSkill)`.

## 4. Concurrency / semantics differences SQLite → MySQL

- No row-level file locks: pool + InnoDB handles concurrent TCP handlers and the
  web dashboard; `autocommit` default, explicit short transactions where a
  logical unit spans multiple statements (character creation, item moves).
- `AUTO_INCREMENT` vs SQLite rowid: accounts ids are app-assigned in C#
  (`300003`, ...) but web admin creates accounts with `last()+1`. Keep explicit
  id inserts for Member.ini import; `AUTO_INCREMENT` for new accounts.
- MySQL `longtext`: Player.Color size tip — VARCHAR(16) suffices; no large
  blobs in the game schema.
- `used_at` is Unix epoch seconds (C# computes
  `(DateTime.UtcNow.Ticks - new DateTime(1970,1,1).Ticks) / 10_000_000`).