# Thiết kế schema SQLite

> **SUPERSEDED** — quyết định này bị thay thế bởi quyết định **dùng MySQL 8** (chủ map redraw). Schema MySQL 8 được chốt ở ticket [Thiết kế schema MySQL 8](11-mysql-schema.md). Phân tích bên dưới giữ nguyên như bản gốc (tài liệu tham khảo: bảng đối chiếu cột, quyết định kiểu/seed/naming vẫn mang triết lý sang MySQL).

Status: resolved
Type: grilling
Blocked by: 03

## Question

Quyết định schema SQLite cho spec: mapping từ `schema.sql` (10 bảng: `Player`, `Homdo`, `LuuLang`, `Pet`, `Quest`, `Skill`, `SkillSave`, `TienTrang`, `Trangbi`, `Tuideo`) + `shopp` sang SQLite — kiểu dữ liệu (Access `DOUBLE` → REAL/INTEGER?), bố cục một-file-mỗi-member (`member/vn{id}.db` từ template NewChar), bảng `accounts` (import từ `Member.ini`, giữ pass1/pass2 plaintext), chỉ mục, và PRAGMA cần thiết (foreign_keys, WAL). Kèm so sánh phương án schema chuẩn hoá vs giữ nguyên cột Access.

## Answer

Chốt qua grilling (từng quyết định có lý do trong các câu hỏi):

1. **File layout**: 2 loại file. `account.db` (shared, 1 file cho mọi account) chứa bảng `Player` + `accounts`. File per-member `member/vn{id}.db` chứa 9 bảng gameplay (Homdo, LuuLang, Pet, Quest, Skill, SkillSave, TienTrang, Trangbi, Tuideo) — khớp cấu trúc C# (Account.accdb vs member/*.accdb).
2. **`shopp`**: loại khỏi contract. `shopp.accdb` không được bất kỳ code C# nào tham chiếu (chỉ tồn tại `Data/shopp_schema.sql` làm tài liệu); nhất quán với ticket 02 đã loại nó khỏi dữ liệu tĩnh.
3. **Kiểu dữ liệu**: mọi cột số (kể cả cột Access `DOUBLE`) → SQLite `INTEGER` (i64). Mọi giá trị quan sát đều nguyên, C# chỉ đọc qua `Conversions.ToInteger`; tránh lỗi float. Cột chuỗi giữ `TEXT` (Player.Color).
4. **Chuẩn hoá tên cột**: ĐỔI tên cột Access sang snake_case rõ nghĩa (vd `Int`/`Int2` → tên mới do spec đặt). Bắt buộc kèm **bảng đối chiếu Access → SQLite** cho TẤT CẢ cột, trong spec — executor phải map được từng tên. **KHÔNG** thêm FK / NOT NULL ngoài Access (C# không dùng FK, thêm sẽ chặn hành vi hợp lệ, vd Homdo.Id trỏ item không tồn tại, Pet.Idskill=0).
5. **Bảng `accounts`**: `accounts(id INTEGER PRIMARY KEY, pass1 TEXT NOT NULL, pass2 TEXT NOT NULL)` trong `account.db`. Import từ `Data/Member.ini` (section `[Account]`, key = id, value = `pass1\tpass2`); giữ plaintext. Không có cột mở rộng.
6. **Thời điểm tạo member.db**: tạo member/vn{id}.db tại lúc tạo nhân vật trong game (opcode 0x09 sub 1), KHÔNG phải lúc tạo account qua web admin. Web admin chỉ tạo dòng trong bảng `accounts`.
7. **Cơ chế tạo**: copy một **file template SQLite binary** đóng gói trong repo (tương đương NewChar.accdb) — 9 bảng + seed sẵn `SkillSave` Id 1..10 / IdSkill 0 (C# không bao giờ INSERT SkillSave, chỉ UPDATE, nên seed bắt buộc). Spec phải mô tả nội dung template + seed. (Bảng `Skill` trong template bị DELETE + tái tạo lúc login nên nội dung seed không quan trọng.)
8. **PRAGMA**: bật `foreign_keys`, `journal_mode=WAL`, `busy_timeout` cho cả account.db và member/*.db — WAL cần cho TCP handler + web dashboard đọc/ghi đồng thời qua sqlx pool.
9. **Index**: giữ nguyên index từ schema.sql: PK duy nhất mỗi bảng, `Player(MapId)`, `Pet(Idskill1..4)`, `Quest(QuestId)`, `SkillSave(IdSkill)`.
10. **DEFAULT**: giữ y nguyên DEFAULT của Player (`ShopPoint 0`, `SP_Store/HP_Store 10000`, `DTT/TLP/TCP/TTP/savemap/tanthu/phien/PTS 0`) — C# INSERT (Client.cs:1200) không chèn các cột này nên phụ thuộc DEFAULT.

Lưu ý còn lại: bảng `Quest` (QuestId INTEGER NULL — C# INSERT INTO Quest không chèn QuestId), `SkillSave.ID` là PK do template seed. Tên file: `account.db` + `member/vn{id}.db` (thay `.accdb`).
