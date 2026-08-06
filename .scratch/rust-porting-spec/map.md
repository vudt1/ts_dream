# Map: Rust porting spec — TS Dream server

## Destination

Một bản **porting spec** hoàn chỉnh, viết bằng tiếng Việt, xuất bản tại `docs/rust_porting_spec.md` — để một executor khác (agent hoặc lập trình viên) build server Rust cho TS Dream **mà không cần đọc source C#**, với độ trung thực byte-level so với server C# hiện tại: game server TCP port 6414 (XOR 0xAD, frame `F4 44`), web admin port 8090, lưu trữ **MySQL 8** (backend ngoài binary, chạy local), một binary duy nhất (tokio + axum + sqlx + askama/HTMX).

## Notes

- **Domain**: TS Online private server. Nguồn sự thật: source C# trong `ts_server_old/`, dữ liệu game trong `ts_server_old/Data/`, schema trong `ts_server_old/CSDL/schema.sql` + `ts_server_old/Data/shopp_schema.sql`.
- **Đọc trước**: `ts_server_old/server_research_login.md` (transport + luồng login đã được trích xuất), `TS_Server_OP_Code_basic.md` (tham chiếu 69 opcode gốc — CHỈ để tham khảo, KHÔNG phải mục tiêu), `spec/codebase_design.md` (kiến trúc đích), `docs/agents/issue-tracker.md` (cách tracker local hoạt động), `Huong_Dan_Cai_Dat_MySQL_ZIP.md` (cài MySQL 8.0.46 local: port 3306, root, my.ini utf8mb4 — lưu ý tránh transcode VISCII, xem research 05).
- **Kỹ năng**: `/research` cho research tickets (AFK), `/grilling` + `/domain-modeling` cho HITL tickets.
- **Quyết định đứng vững** (đã chốt khi charting — không phải ticket):
  - **Scope**: full parity với server C# — 29 opcode C→S + battle + quests + pets + trading + ... 69-opcode doc KHÔNG phải mục tiêu.
  - **Dữ liệu**: giữ `Data/` nguyên trạng, load tại runtime; chỉ player state chuyển sang MySQL 8.
  - **DB**: **MySQL 8** (đã thay quyết định SQLite cũ — ticket 05 superseded, xem ticket **Thiết kế schema MySQL 8**): shared schema — MỘT database `ts_dream`, bảng `players` + `accounts` (shared) + 9 bảng gameplay thêm cột `player_id` làm PK kép; nhân vật mới = INSERT seed (hết template copy file); accounts giữ pass1/pass2 plaintext, **tạo thuần qua web dashboard — KHÔNG import Member.ini** (ghi đè bởi ticket **Thiết kế schema MySQL 8**, đã chốt grilling); MySQL 8 chạy local (localhost:3306), kết nối qua `database_url`. Chuỗi giữ raw byte VISCII (`VARCHAR … latin1` **khai TƯỜNG MINH** vì server default `utf8mb4` làm hỏng byte VISCII — không gợi ý đổi sang utf8, wire là VISCII).
  - **Gates**: giữ nguyên version >= 186 và prefix "vn".
  - **Battle**: port faithful, research cạn kiệt.
  - **Encoding**: một chapter riêng, round-trip byte-exact tên tiếng Việt.
  - **Player migration**: KHÔNG — fresh start.
  - **Stack**: bắt buộc tokio + axum + sqlx(**MySQL**) + askama/HTMX, single binary (MySQL 8 là dịch vụ ngoài, không nhúng — binary vẫn một file).
  - **Dashboard**: view online + start/stop server + live log packet (SSE).
  - **Fidelity**: byte-level. **Acceptance**: capture-based (ghi traffic thật client↔server C#, diff với output Rust).
  - **Spec**: một tài liệu chương hoá, tiếng Việt, tại `docs/rust_porting_spec.md`. Tiêu chí chấp nhận: executor không bao giờ đọc C#.
- **Handoff**: khi toàn bộ ticket resolved, cách đã rõ — session/người tiếp theo viết spec từ Decisions-so-far + research assets. Không có ticket "viết spec" — đó là handoff, không phải quyết định.

## Decisions so far

<!-- một dòng mỗi ticket closed: gist + link -->

- [Trích xuất giao thức đầy đủ (Protocol Reference)](issues/01-protocol-reference.md) — Toàn bộ wire protocol: 29 opcode C→S + sub-dispatch, toàn bộ packet S→C (Logined1 byte-exact, battle wire), ~120 literal hex response có nghĩa, framing XOR 0xAD/F4 44. 2 gap: FTalk.H6 (hội thoại cứng ~3000 dòng, tóm bằng template) và bảng RNG craft (0x17 sub 14).
- [Định dạng dữ liệu tĩnh (Data File Formats)](issues/02-data-file-formats.md) — 14 file: column map, delimiter, encoding thực tế, row count, cách parse trong Data.cs + quest .ini (Win32 INI semantics, sentinel "nothing", case-insensitive), key formats. EVe.txt/dictionary_0/shopp.accdb là legacy KHÔNG load — loại khỏi contract.
- [Encoding tiếng Việt (Text Encoding Contract)](issues/03-text-encoding.md) — Wire encoding là VISCII 1.1 (chứng minh 3 cách). Npcs.txt UTF-16LE+BOM (VISCII mojibake), Items.txt UTF-8 mojibake VISCII (faithful, không corrupted), Skills.txt UTF-8 sạch. TextEncoder.cs chết — port giữ raw codepoints.
- [Trích xuất Battle Engine (TheBattle.cs)](issues/04-battle-engine.md) — Lưới 20 ô, vòng Battling() + 19 skill-type dispatch, combo, damage pipeline + element table, 3 dòng RNG riêng biệt, toàn bộ packet battle byte-exact. 6 gap nhỏ cho pass 2 (checksum smethod_5, GetTurn 14013, exp curve...).
- [Thiết kế schema SQLite](issues/05-sqlite-schema.md) — 2 file: `account.db` shared (Player + accounts) + `member/vn{id}.db` (9 bảng gameplay); mọi cột số → INTEGER; đổi tên cột snake_case kèm bảng đối chiếu bắt buộc, KHÔNG thêm FK/NOT NULL; seed SkillSave 1..10; PRAGMA foreign_keys+WAL. **SUPERSEDED** — bị thay bởi quyết định MySQL 8 (ticket **Thiết kế schema MySQL 8**); giữ lại như tài liệu bản gốc.
- [Hợp đồng Web Dashboard (port 8090)](issues/06-web-dashboard.md) — Không xác thực; 1 trang + API + SSE; routes: status/start/stop(409 khi chưa chạy)/announce(020C)/accounts CRUD/npcs/online(id,name,ip)/log-stream/perexp; AppState Arc<RwLock> + broadcast; log mọi frame hex (level log/system/warning/packet/error/debug/c2s/s2c); ring buffer 500; stop đóng listener+kick, HTTP sống.
- [Cấu trúc spec & Acceptance harness](issues/07-spec-structure-acceptance.md) — 9 chương (Architecture→Acceptance); golden ở `golden/` trong repo (plaintext hex, `<<`C2S/`>>`S2C, chuẩn packet.txt); thu capture bằng proxy 2 chiều; diff bằng test runner gửi C2S so S2C frame-by-frame; C2S/S2C là 2 luồng tuần tự độc lập, test deterministic-only; ~10-15 scenario; Config TOML+env (`TS_` prefix, game_port 6414/web_port 8090/data_dir/db paths); hằng số giao thức hardcode (XOR 0xAD, F4 44, vn, 186, TSVN); harness code trong repo.
- [Xử lý hội thoại cứng FTalk.H6](issues/08-hardcoded-dialogs.md) — H6 là logic menu, không phải text (text ở Data_Talks INI). Spec transcribe thành bảng data-driven (map_id/idtalking/select_menu/action/item_in/out/packet_literal) cho 45 map case + H6 pre-dispatch rules (banker/hotel dùng chung) + exceptions đầy đủ (daily quest RNG 385-508 → nối ticket 09, pet reborn 55002/59102/59011); đặt ở phụ lục chương Protocol; generic WARP/BattleQuestWin chỉ tham chiếu.
- [Battle pass 2 — các gap còn lại](issues/09-battle-pass2.md) — Reset cả 6 gap: KHÔNG có checksum (send = hex→bytes XOR 0xAD toàn frame, `Class5.cs:132-166`); GetTurn case 14013 fall-through vào ladder `1-3→2/4-6→3/7-9→4/10→5`; heal item = `GetDataItem(id,"Hp"/"Sp")` + restore trận `_HP_Store/_SP_Store`; getHpMax/getSpMax là công thức luỹ thừa đóng, exp curve là loop `Texps[]`; BattleQuestWin full side-effect; H6 dùng `new Random()` riêng với đúng 21 draw (item `62001+num3*100`/`62101+num4*100`) — research: [`06-battle-pass2.md`](research/06-battle-pass2.md).
- [Ngoại lệ fidelity (tên garble, MySQL branch)](issues/10-fidelity-exceptions.md) — 3 quyết định: (1) TÁI HIỆN garble 122 tên byte-for-byte theo `smethod_13` (CP1252≥0x100→4 hex digit/2 byte rác, 3-digit abort) để khớp capture — mục "Ngoại lệ garble" riêng trong spec; (2) BỎ degrade `item_code` (MySQL bắt buộc fail-fast ⇒ không có 'no DB'), store functional với bind param + transaction chống redeem trùng; (3) `Title=` quest giữ opaque bytes (0xA0–0xEF), không transcode/không gửi client — chỉ server-GUI.
- [Thiết kế schema MySQL 8](issues/11-mysql-schema.md) — Thay thế "Thiết kế schema SQLite" (05 superseded). Một DB `ts_dream` (InnoDB): PK kép per-player `(player_id, slot/stt/Id/ID)` cho 9 bảng, `Quest` KHÔNG PK (giữ index QuestId). Scoping contract bắt buộc: MỌI SQL C# trên 9 bảng port kèm `player_id` (liệt kê 3 họ pattern nguy hiểm: SkillSaveGetId/UpdateId, DELETE-rồi-rebuild Skill lúc login, DELETE Quest theo MapId — FTalk). Số cột → `BIGINT`, DEFAULT y nguyên, KHÔNG FK/NOT NULL; chuỗi = `VARCHAR … latin1` + connection charset latin1 (VISCII byte-safe). `item_code` bảng riêng, redeem functional: bind param + transaction `UPDATE … WHERE … AND player_id=0` (rowcount gate) chống redeem trùng. Tạo nhân vật = MỘT transaction atomic (players + SkillSave 1..10 + Skill). Migration `sqlx::migrate!` TRƯỚC listener, fail-fast hard-exit. **Accounts KHÔNG import Member.ini — chỉ web dashboard tạo (ghi đè base cũ)**. Config `TS_DATABASE_URL`, bỏ 3 key cũ. Index: players(MapId)/Pet(IdSkill1..4)/Quest(QuestId)/SkillSave(IdSkill).

## Not yet specified

- **Thread-safety/timing khi port C#** (dispatch theo thread-per-frame, state gắn WinForms, static mutable trong `Server`/`Data`) — chưa đủ sắc để ticket, graduate sau research.
- **Ops MySQL** (tạo database/user local lần đầu, dump/backup, nâng cấp schema lúc vận hành) — phạm vi đưa vào spec hay để ngoài, quyết khi frontier chạm tới.

## Out of scope

- WinForms UI cũ (`FormServer.cs` — bị thay bởi web dashboard).
- SQLite storage (bị loại bỏ — thay bằng MySQL 8; không quay lại).
- `MySqlDbConnection.cs` (legacy MySQL, không được dùng).
- Game client.
- 69 opcode gốc vượt quá những gì server C# thực sự implement.
- Migrate player `.accdb` hiện có (fresh start — đã quyết định).
