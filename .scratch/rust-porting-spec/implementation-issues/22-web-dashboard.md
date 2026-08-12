# 22 — Web dashboard (Ch7)

**What to build:** Dashboard web điều khiển hoàn chỉnh chạy trên `0.0.0.0:8090`: trang HTML server-rendered (askama/HTMX), JSON API quản lý account/server, và SSE live-log (ring 500 + broadcast). Người vận hành xem/dừng/khởi động server và tạo account trực quan.

**Blocked by:** 01 — Scaffold (web server spawn); 04 — DB (`accounts`); 05 — Golden/harness hằng số để announce (op 0x02); 07 — Login/session (online list, kick client); 20 — Battle (server state để stop an toàn).

**Status:** completed

- [ ] `GET /` — page: online list + start/stop/announce controls + live log + account fields + NPC list.
- [ ] API routes đúng app state (Ch7 §7.3):
  - `GET /api/server/status` `{"running":bool}`; `POST /api/server/start` (bind :6414 + accept nếu chưa chạy); `POST /api/server/stop` → countdown `020C` "Server will be closed in N second(s)" 5s rồi đóng socket + listener, HTTP vẫn sống, `running=false`; `POST /api/server/announce` {text} → op 0x02 sub 0x0C to all; `GET/POST /api/accounts` (list / create, trả `last_insert_id()`); `GET /api/npcs` (in-memory Data_Npcs); `GET /api/online`; `GET /api/log/stream` (SSE); `POST /api/config/perexp` {value} → set `AppState.perexp` (không persist).
  - Khi server not running: status → false; stop → **409**; announce → **409**; perexp vẫn settable; HTTP luôn up để bấm Start (Ch7 §7.3).
  - Thêm (ticket #22 user-req): `GET /api/db/status` → `{"state":"green|light|dark"}`; `GET /api/db/status/stream` (SSE event `dbstatus`, data `{"state":...}`) — live MySQL connectivity badge (Ch7 §7.5a).
- [ ] Shared `Arc<RwLock<AppState>>`: online list, `running`, `perexp`, ring buffer **500** log lines, `broadcast<LogEvent>` (Ch7 §7.2).
- [ ] SSE schema: event `log`, data `{level, ts, msg}`; 8 levels: log/system/warning/packet/error/debug/c2s/s2c (Ch7 §7.4).
- [ ] Live packet log (mới của Rust): log **mọi frame hex sau XOR** với direction + player id → ring buffer + broadcast (Ch7 §7.5).
- [ ] Account create: insert `accounts`, trả id qua `last_insert_id()`, pass1/pass2 plaintext (Ch5 §5.8).
- [ ] Dashboard + game server share AppState; stop server không phá vỡ server game. **Acceptance:** verify thủ công — dashboard hiển thị online, start/stop/announce, tạo account; golden không áp dụng cho dashboard (không đo wire byte).

---

## Notes / deferred

- **Ticket gốc không có mục Notes / deferred.** Nội dung dưới ghi lại phát hiện từ grilling + domain-modeling session (2026-08-12) và các quyết định đang chờ user.

### Discovered issue D1 — badge "dark" không thể đạt được dưới kiến trúc fail-fast hiện tại — **ĐÃ GIẢI QUYẾT**
- Spec §1.1 + `src/main.rs:79-83`: MySQL unreachable lúc boot → process **hard-exit**; dashboard chỉ được serve khi DB đã connect. `WebState.pool` luôn `Some` khi dashboard sống → indicator "dark" (chưa connect) sẽ luôn là green trừ khi thêm phát hiện runtime.
- Grep `src/` xác nhận chưa có `db_connected` / `SELECT 1` / ping / health mechanism nào → build from scratch.
- Ba trạng thái theo yêu cầu user: `green` = DB connect thành công, `dark` = chưa connect thành công, `light` = (cần định nghĩa: đang reconnect / chờ probe đầu / degraded).

### Quyết định grilling (user đã chọn — 2026-08-12)
1. **Kiến trúc:** (A) **Giữ fail-fast boot + background liveness probe** (tuân thủ §1.1; dark đạt được khi DB chết SAU khi boot).
2. **Nghĩa "light":** (A) **Đang reconnect / probe đầu** — khởi động = `light` đến khi probe đầu thành công, sau đó `green`/`dark`.
3. **Cơ chế probe & live update:** (A) **Background task ping ~5s + SSE push thay đổi.**

### Triển khai (done — `cargo test` 303 passed)
- `src/state.rs`: `enum DbStatus { Connecting, Connected, Disconnected }` + `as_str()` (`light`/`green`/`dark`); `AppState` thêm `db_status` (init `Connecting`) + `db_status_tx: broadcast::Sender<DbStatus>`.
- `src/db/pool.rs`: `spawn_liveness_probe(pool, app, interval)` — loop `SELECT 1`, set `Connected`/`Disconnected`, broadcast change; tách `probe_next_state(prev, ping_ok)`.
- `src/main.rs`: gọi `spawn_liveness_probe(pool, app_state, 5s)` sau tạo AppState (boot vẫn fail-fast).
- `src/web/app.rs`: `GET /api/db/status` → `{"state"}`; `GET /api/db/status/stream` (SSE event `dbstatus`); `DashboardTemplate` thêm `db_state`.
- `templates/dashboard.html`: header thêm dòng **Database** dưới server status (CSS `status-connected/connecting/disconnected` + `dot-green/light/dark`); JS `EventSource('/api/db/status/stream')` cập nhật live.
- Tests: `db_status_as_str_mapping`, `app_state_new_starts_connecting`, `db_status_broadcast_delivers_to_subscriber`, `probe_next_state_maps_ping_result`.

### Goals bổ sung (từ user)
- Thêm dòng **Database** dưới trạng thái server trong header dashboard, 3 màu dark/light/green. ✅
- Không commit trong `ts_server_old/` + `docs/`. ✅ (chỉ sửa `src/`, `templates/`, `.scratch/`.)
- Cập nhật đầy đủ goals + issues phát hiện được vào ticket. ✅