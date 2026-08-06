# Hợp đồng Web Dashboard (port 8090)

Status: resolved
Type: grilling
Blocked by: 01

## Question

Quyết định hợp đồng web dashboard cho chapter Web của spec: routes/pages (danh sách online, start/stop server, log packet), SSE event schema cho log realtime, cách đọc/ghi `AppState` chia sẻ giữa TCP và HTTP (Arc<RwLock>), và các admin action phơi ra. Dựa trên quyết định đứng vững: view online + control + live logs. Liệt kê từng route, payload, và hành vi khi server chưa chạy.

## Answer

Chốt qua grilling (10 câu hỏi):

1. **Xác thực**: KHÔNG có login admin. Dashboard mở không cần đăng nhập.
2. **Cấu trúc**: 1 trang dashboard (askama/HTMX) + API JSON + SSE. Routes:
   - `GET /` — trang dashboard (online list + controls + log + account/NPC list)
   - `GET /api/server/status` — `{running: bool}`
   - `POST /api/server/start` — bind lại listener 6414 + accept
   - `POST /api/server/stop` — countdown 5s broadcast `020C` "Server will be closed in N second(s)" (giữ nguyên C# method_2), rồi đóng mọi socket client + đóng listener
   - `POST /api/server/announce` — `{text}` → opcode 0x02 sub 0x0C gửi toàn server (Button_ServerChat)
   - `POST /api/accounts` — tạo account: `{pass1, pass2}` → ghi vào bảng `accounts` MySQL, id = `AUTO_INCREMENT` (lấy id mới qua `last_insert_id()` — thay cách `max+1` cũ để tránh race khi tạo đồng thời) (Button_CreatAccount, thay Member.ini)
   - `GET /api/accounts` — danh sách `{id, pass1, pass2}` từ bảng accounts (thay ListView_Account)
   - `GET /api/npcs` — danh sách NPC từ Data_Npcs trong bộ nhớ (thay ListView_Npc/method_4)
   - `GET /api/online` — `[{id, name, ip}]` (cột ID/PlayerName/IP như ListView_Client)
   - `GET /api/log/stream` — SSE
   - `POST /api/config/perexp` — `{value}` → lưu AppState runtime (Server.PerEXP, không persist)
3. **SSE schema**: một event duy nhất `event: log`, `data: {level, ts, msg}`. Level map 8 mức Logger.cs: `log`, `system`, `warning`, `packet`, `error`, `debug`, `c2s`, `s2c`.
4. **AppState**: `Arc<RwLock<AppState>>` cho trạng thái (online list, running, perexp) + `tokio::sync::broadcast` channel cho log packet + server status event. Broadcast cho SSE đa subscriber, giảm tranh chấp lock.
5. **Live log packet**: log TOÀN BỘ frame hex đầy đủ (dạng `F444...` sau XOR, kèm hướng c2s/s2c + id player). Lưu ý: C# KHÔNG gọi Logger.C2S/S2C bao giờ — đây là feature mới của Rust theo standing decision.
6. **Log buffer**: ring buffer 500 dòng cuối trong AppState; broadcast subscriber mới nhận được lịch sử gần đây (reload/reconnect không mất hết log).
7. **PerEXP**: set qua dashboard, lưu AppState runtime, không persist (default 0 như C#).
8. **Server chưa chạy**: `GET /api/server/status` trả `{running:false}`; `POST /api/server/start` được phép; `POST /api/server/stop` khi chưa chạy → 409 + lý do; `POST /api/server/announce` khi chưa chạy → 409. HTTP luôn phục vụ dashboard để bấm Start.
9. **Stop server**: đóng listener + kick toàn bộ client; HTTP 8090 vẫn sống.
10. **PerEXP/announce khi chưa chạy**: announce → 409; perexp vẫn được (chỉ set số).

## Cập nhật (DB switch → MySQL 8)

Các item 2 chứa 'accounts SQLite (account.db)' được thay: ghi/đọc bảng `accounts` **MySQL** (ticket **Thiết kế schema MySQL 8**) thay file SQLite. Mọi route/SSE/AppState không đổi.
