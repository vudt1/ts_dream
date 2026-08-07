# Cấu trúc spec & Acceptance harness

Status: resolved
Type: grilling
Blocked by: 01, 02, 03, 04, 05, 06

## Question

Quyết định cấu trúc chương cuối của `docs/rust_porting_spec.md` (các chapter: architecture, protocol, data, encoding, SQLite, battle, web, config, acceptance) và thiết kế **acceptance harness** byte-level: cách ghi capture traffic thật client↔server C# thành golden packet sets và cách diff với output server Rust. Quyết định này gộp cả chapter Config (port 6414/8090, đường dẫn Data/, db path, TOML+env). Một ticket quyết định cho phần "lắp ráp" — không phải viết spec.

## Answer

Chốt qua grilling (10 câu hỏi):

1. **Cấu trúc spec**: 9 chương giữ nguyên như ticket — Architecture, Protocol, Data files, Encoding, SQLite (→ đổi tên **Database (MySQL 8)**), Battle, Web dashboard, Config, Acceptance. Mỗi chương lấy nội dung từ research asset tương ứng + phần "lắp ráp"; thứ tự phản ánh luồng build cho executor đọc tuần tự.
2. **Golden packet sets**: thư mục riêng trong repo (`golden/`), mỗi test case 1 file text — versioned cùng repo, spec tham chiếu tên file. Không nhúng trong spec.
3. **Thu capture**: proxy TCP ghi log 2 chiều (client → proxy → server C#), tool trong repo, không sửa C#/client; log hai hướng dạng plaintext hex sau XOR.
4. **Định dạng golden**: plaintext hex, 1 frame 1 dòng, `<<` = C2S, `>>` = S2C, comment `//`, dòng trống tách nhóm (chuẩn packet.txt sẵn có). Không dùng binary.
5. **Diff**: test runner (cargo test integration) đọc golden, nối vào Rust server, gửi C2S theo thứ tự, thu S2C, so frame-by-frame với golden S2C — khớp từng byte, fail khi lệch.
6. **Quan hệ C2S/S2C**: 2 luồng tuần tự độc lập, không xen kẽ cặp thời gian thực. Test phụ thuộc timing (vd broadcast 020C từ timer) đánh dấu deterministic-only; loại frame phụ thuộc timing khi chốt golden.
7. **Phạm vi test**: ~10-15 golden scenario bao phủ nhóm nghiệp vụ: login thành công/sai pass, tạo nhân vật, di chuyển, chat, mua mall, dùng item, warp, quest (nhánh FTalk.H6), battle 1-2 mẫu, pet.
8. **Config**: TOML (`ts_dream.toml`) mặc định + env override từng key (prefix `TS_`). Keys: `game_port=6414`, `web_port=8090`, `data_dir` (Data/), `account_db_path`, `member_dir`, `template_db_path` (template binary ticket 05) → **bị thay bởi `database_url`** (`mysql://user:pass@localhost:3306/ts_dream`, env `TS_DATABASE_URL`) khi chuyển MySQL 8 — xem ticket **Thiết kế schema MySQL 8**, `perexp_default=0`.
9. **Hằng số giao thức**: KHÔNG config hóa — hardcode constants trong spec: XOR 0xAD, magic `F4 44`, IDPrefix `vn`, min version 186, tên server `TSVN`. Đổi là phá parity với client thật.
10. **Harness trong repo**: spec mô tả giao thức harness + định dạng golden + quy trình; repo chứa sẵn `golden/` + tool proxy capture + test runner. Executor chạy thẳng. Spec tiếng Việt, tool tiếng Anh.
