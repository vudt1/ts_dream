# 22 — Web dashboard (Ch7)

**What to build:** Dashboard web điều khiển hoàn chỉnh chạy trên `0.0.0.0:8090`: trang HTML server-rendered (askama/HTMX), JSON API quản lý account/server, và SSE live-log (ring 500 + broadcast). Người vận hành xem/dừng/khởi động server và tạo account trực quan.

**Blocked by:** 01 — Scaffold (web server spawn); 04 — DB (`accounts`); 05 — Golden/harness hằng số để announce (op 0x02); 07 — Login/session (online list, kick client); 20 — Battle (server state để stop an toàn).

**Status:** ready-for-agent

- [ ] `GET /` — page: online list + start/stop/announce controls + live log + account fields + NPC list.
- [ ] API routes đúng app state (Ch7 §7.3):
  - `GET /api/server/status` `{"running":bool}`; `POST /api/server/start` (bind :6414 + accept nếu chưa chạy); `POST /api/server/stop` → countdown `020C` "Server will be closed in N second(s)" 5s rồi đóng socket + listener, HTTP vẫn sống, `running=false`; `POST /api/server/announce` {text} → op 0x02 sub 0x0C to all; `GET/POST /api/accounts` (list / create, trả `last_insert_id()`); `GET /api/npcs` (in-memory Data_Npcs); `GET /api/online`; `GET /api/log/stream` (SSE); `POST /api/config/perexp` {value} → set `AppState.perexp` (không persist).
  - Khi server not running: status → false; stop → **409**; announce → **409**; perexp vẫn settable; HTTP luôn up để bấm Start (Ch7 §7.3).
- [ ] Shared `Arc<RwLock<AppState>>`: online list, `running`, `perexp`, ring buffer **500** log lines, `broadcast<LogEvent>` (Ch7 §7.2).
- [ ] SSE schema: event `log`, data `{level, ts, msg}`; 8 levels: log/system/warning/packet/error/debug/c2s/s2c (Ch7 §7.4).
- [ ] Live packet log (mới của Rust): log **mọi frame hex sau XOR** với direction + player id → ring buffer + broadcast (Ch7 §7.5).
- [ ] Account create: insert `accounts`, trả id qua `last_insert_id()`, pass1/pass2 plaintext (Ch5 §5.8).
- [ ] Dashboard + game server share AppState; stop server không phá vỡ server game. **Acceptance:** verify thủ công — dashboard hiển thị online, start/stop/announce, tạo account; golden không áp dụng cho dashboard (không đo wire byte).