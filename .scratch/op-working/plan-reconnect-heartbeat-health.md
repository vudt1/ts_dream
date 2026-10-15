# Kế Hoạch Cải Thiện Server Rust: Reconnect, Heartbeat & Health Check

> **Trạng thái**: Đã chốt quyết định (Q1, Q2, Q3, Q5, Q6) — sẵn sàng triển khai theo §5. Chưa sửa code.
> **Tài liệu nguồn**:
> - Nghiên cứu client: [`.scratch/client-pseudo-op-code/reconnect_heartbeat_flow.md`](../client-pseudo-op-code/reconnect_heartbeat_flow.md)
> - Decompile/ASM đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/` (`0077f414`, `005045a0`, `0050d030`, `0051aae0`, `007095c0`)
> - Nền tảng liên quan: [`post_login_stats_inventory_flow.md`](../client-pseudo-op-code/post_login_stats_inventory_flow.md), [`login_flow_research.md`](../client-pseudo-op-code/login_flow_research.md), [`opcode_00_01.md`](../client-pseudo-op-code/opcode_00_01.md)
> - Quy ước: test đặt trong `tests/`, không `#[cfg(test)]` trong `src/` (`AGENTS.md`).

---

## 1. Tóm Tắt Điều Hành

Client **không có heartbeat định kỳ** và **không có auto-reconnect theo timer**. Frame duy nhất client gửi liên quan kết nối là ACK `F4 44 01 00 00` (payload 1 byte `0x00`, wire `59 E9 AC AD AD`), gửi đúng hai lúc: sau khi mở socket game port 6414 (`005045a0` → `0077f414(...,0)`) và ngay trước hộp thoại "mất kết nối" (`0078aabe`).

Phía server Rust hiện có ba lỗ hổng lớn:

1. **Không có cơ chế phát hiện connection chết.** `select!` ở `server_control.rs:377-483` chỉ có 2 nhánh, không có idle timer, không có `last_seen`. Socket half-open treo vô hạn, entry `clients` không được dọn.
2. **Double-login guard không phân biệt zombie với phiên sống** (`login.rs:215-221`), nên reconnect hợp lệ bị chặn bằng `F44402000013`.
3. **Dữ liệu sau relog bị sai**: `recompute_stats()` không được gọi sau `load` và `load_items` không khôi phục các trường template (`lv`, `loai`, `thuoctinh`, `_1`/`_2`).

Mục tiêu: sau kế hoạch này, server (a) tự phát hiện và dọn connection chết trong thời gian cấu hình, (b) cho phép reconnect sạch, (c) trả lại đúng gear bonus và trang bị sau relog, (d) báo lý do kick cho client.

---

## 2. Xác Nhận Lại Trạng Thái Hiện Tại

Đã đối chiếu trực tiếp `src/` (không chỉ tin tài liệu nghiên cứu):

| Claim | Kết quả | Vị trí |
|---|---|---|
| Không có `last_seen`, `set_nodelay`, `set_keepalive` | Đúng | grep toàn `src/` |
| `Instant::now`/timeout chỉ có trong battle và DB | Đúng | `battle/manager.rs`, `battle/service.rs`, `db/pool.rs` |
| `send_system_alert` không có caller ngoài wrapper | Đúng | `dispatcher.rs:187`, `response.rs:49` |
| Dispatcher không có arm `0x00`, rơi vào wildcard | Đúng | `dispatcher.rs:394` |
| `recompute_stats` không được gọi trong `login.rs` | Đúng | caller hiện có: `gm.rs`, `handlers/inventory.rs`, `quest.rs`, `stats.rs` |
| Double-login guard trả `DOUBLE_LOGIN` và `shutdown` | Đúng | `handlers/login.rs:215-221` |
| `select!` chỉ có 2 nhánh | Đúng | `server_control.rs:377` |

**Ghi chú về phạm vi xác nhận:** một số claim phía client (ACK `F4 44 01 00 00`, pacing 100 ms, không có timer) đã được tài liệu nghiên cứu ghi nhận từ ASM. Lần này tôi chưa đọc lại hex gốc của `0078aabe` (file `.asm.txt` không tìm thấy trong thư mục decompile). Các claim đó cần được xác minh lại trước khi làm gap 5 (xử lý ACK).

---

## 3. Danh Sách Gap

Mức: **Cao** = chặn vận hành hoặc gây leak tài nguyên · **Trung bình** = sai hành vi hoặc thiếu bảo vệ · **Thấp** = dọn dẹp, không ảnh hưởng người chơi ngay.

| # | Gap | Mức | Vị trí | Hậu quả |
|:-:|---|:-:|---|---|
| G1 | Không có idle timeout / read deadline | Cao | `server_control.rs:377-483` | Half-open socket treo vô hạn; `clients` và `online_sessions` leak; `disconnect_player` không bao giờ chạy |
| G2 | Double-login guard chặn reconnect khi entry cũ còn sót | Cao | `login.rs:215-221`, `server_control.rs:238-240` | Người chơi không vào được game cho đến khi restart |
| G3 | `recompute_stats()` không chạy sau load | Cao | `login.rs:207`, `sqlite/session.rs:19-114` | Gear bonus = 0 sau relog; frame `05 03` sai |
| G4 | `load_items` chỉ đọc `slot,itemid,quantity,damage` | Cao | `sqlite/session.rs:176-199`, `session.rs:9-43` | Mất `lv`, `loai`, `thuoctinh`, `_1`/`_2`; hỏng equip gate |
| G5 | Không có arm `0x00` trong dispatcher | Trung bình | `dispatcher.rs:252-397` | ACK của client rơi vào `unimplemented`; không có tín hiệu liveness |
| G6 | Không gửi `00` system-alert khi kick | Trung bình | `login.rs:216-219`, `gm.rs:407` | Client không biết lý do bị ngắt, treo UI |
| G7 | Không có `set_nodelay` / `set_keepalive`; `mpsc` không giới hạn | Trung bình | `server_control.rs:33,62,364` | Latency gói nhỏ; reader bị stall làm channel phình vô hạn |
| G8 | Lỗi read bị nuốt không log (`Err(_) => close = true`) | Thấp | `server_control.rs:472` | Khó chẩn đoán khi socket lỗi |
| G9 | `stop()` chỉ `clients.clear()`, không đóng socket | Thấp | `server_control.rs:181-183` | Socket vẫn mở sau shutdown nội bộ |
| G10 | `logined`/`in_world` không reset khi re-login | Thấp | `login.rs:43-48`, `login.rs:245` | Gửi `0x01` hai lần trên một socket có thể làm client treo |
| G11 | `battle_service.register_sender` seed từ snapshot cũ | Trung bình | `server_control.rs:242-253` vs `:425-428` | Battle gửi tới sender lệch sau relog |
| G12 | Field session không persist mất khi reconnect | Thấp | `session.rs:206,216-218,244-261` | `color`, `gocnhin`, `hp_store`, `quest_steps`, ... reset |
| G13 | `characters.fai` được ghi nhưng không đọc | Thấp | `sqlite/session.rs:210,271,285,500` | Dữ liệu ghi xuống không dùng |
| G14 | `SaveLedger::forget()` không có caller | Thấp | `auto_save.rs:193-195` | Map phình vô hạn |
| G15 | `/api/online` luôn trả `[]` | Thấp | `web/app.rs:227-229,445-447,167` | Dashboard không phản ánh người online |
| G16 | `0x2E` (server switch) và `0x36` (server list) chưa implement | Thấp | `dispatcher.rs:376-377` | Không có đường reconnect native do server chủ động |
| G17 | Logined1 không push `08 01` | Trung bình | `spawn.rs:471-604` | Gap đã biết từ tài liệu nền, ngoài phạm vi reconnect |

**Không có test nào** chạm `handle_client_connection`, `ServerControl::login_register`, hoặc đường idle/health. Double-login chỉ được kiểm bằng hằng chuỗi (`tests/create_char_atomic_test.rs:250-260`).

---

## 4. Hướng Giải Quyết Chi Tiết

### 4.1 Nền tảng liveness (G5, G9 phần liveness)

**Config** — `src/config.rs:11-24`, `Default` `:26-39`:
- Thêm `idle_timeout_secs: u64` (mặc định `300`).
- Thêm `TS_IDLE_TIMEOUT_SECS` qua khối `env_override` sẵn có (`:91-116`).

**State** — `src/server/session.rs:508-512`:
- Thêm `pub last_seen: std::time::Instant` vào `struct Conn`, **không** vào `Session`. `Session` được clone vào registry toàn cục mỗi frame (`server_control.rs:426`), `Instant` ở đó vô nghĩa và phá hợp đồng `fingerprint`.
- Khởi tạo trong `Conn::new` (`:564-570`) và `with_peer_ip` (`:572-578`).

**Handler** — `src/server/dispatcher.rs:252-397` và `src/server/handlers/system.rs`:
- Thêm arm `OP_SYSTEM_ALERT => system::handle_system_alert(ctx)` trước wildcard `:394`.
- Hàm mới: cập nhật `ctx.conn.last_seen = Instant::now()`, log `trace` (không `warn` để tránh spam). Không trả lời: client không có ACK-of-ACK.
- `OpcodeCtx` (`dispatcher.rs:194-206`) đã giữ `&mut Conn`, không cần đổi signature.

**Liveness ngầm** — `server_control.rs:382`:
- Mọi lần `Ok(n)` đọc được byte đều cập nhật `last_seen`, để heartbeat hoạt động ngay cả khi frame không có handler.

### 4.2 Idle timeout (G1, G6)

Vòng `select!` ở `server_control.rs:377-483` có thêm nhánh thứ ba.

**Kỹ thuật:** các arm của `select!` được tạo lại mỗi vòng, nên phải giữ `tokio::time::Sleep` pinned **ngoài** vòng lặp, dùng `&mut sleep` trong arm, và gọi `sleep.as_mut().reset(Instant::now() + idle)` trong nhánh `Ok(n)` (`:382`) và nhánh ACK.

**Khi hết hạn:**
1. `out.send_system_alert(SystemAlertReason::ConnectionLost)` — sub `13` ("Đã mất kết nối."). Builder: `system_alert.rs:269-271`.
2. **Phải yield để frame tới socket** trước khi đóng: chạy `rx.recv()`/flush thêm một lần. Cùng lớp bug với `out.shutdown` ở `:462-469`, hiện break trước khi drain.
3. `push_log("warn", ...)` để vá G8 tại cùng điểm.
4. `close = true` → teardown sẵn có `:485-490` (đã chạy `disconnect_player`: broadcast hide, unregister, persist).

**Hardening cùng file (G7):** sau `listener.accept()` (`:132-142`), thêm `stream.set_nodelay(true)` và keepalive. Keepalive của `std`/`tokio` không có API trực tiếp; dùng `socket2` nếu cần, hoặc để idle timeout ở tầng ứng dụng đảm nhiệm (khuyến nghị: idle timeout ở tầng ứng dụng là đủ, keepalive là lớp phụ).

**Kick có lý do (G6):** trước mọi `out.shutdown = true` ở đường kick, gọi `out.send_system_alert(reason)`. Các đường cần rà: double-login (`login.rs:216-219`), suspended (`login.rs:184-195`), GM kick (`gm.rs:390-419`). Helper có sẵn: `dispatcher.rs:187-189`.

### 4.3 Double-login và reconnect sạch (G2, G15)

Sửa trong `login_db` (`login.rs:207-221`):

1. `load()` ở `:207` đã thành công. Nếu `hub.login_register(...)` trả `false`, **không** từ chối ngay.
2. Thêm `ServerControl::is_client_live(player_id) -> bool` (cạnh `register_client:73`), dựa trên `clients.contains_key` **và** `last_seen` của incumbent.
3. Nếu incumbent đã chết (quá `idle_timeout`): gọi `disconnect_player(old)` (đã broadcast hide + persist), rồi thử `login_register` lại.
4. Nếu incumbent còn sống: giữ `DOUBLE_LOGIN` + gửi kèm lý do (xem 4.2).

Đây là sửa đổi **gỡ tắc người chơi hôm nay** và an toàn vì `disconnect_player` đã persist trước khi unregister.

`stop()` (G9): gọi `disconnect_player` cho từng client (hoặc gửi shutdown qua sender) trước khi `clients.clear()`, để không để lại socket mở.

### 4.4 Dữ liệu sau relog (G3, G4)

**G3 — `recompute_stats`:**
- Cách đơn giản: thêm `conn.session.recompute_stats();` sau `hydrate_templates` (`login.rs:207-209`), trước khi build Logined1 (`:224`).
- Cách bền hơn (khuyến nghị): gọi ở cuối `SqliteSessionRepository::load` (`sqlite/session.rs:112`, ngay trước `Ok(true)`), để mọi caller được hưởng. Đây là điểm hydrate duy nhất.

**G4 — `load_items`:**
- `InventoryItem::from_template` (`session.rs:50-74`) đã copy `lv`, `loai`, `thuoctinh`, `giatri_thuoctinh` và `_1`/`_2`.
- Hai lựa chọn:
  - (a) Truyền `&GameData` (hoặc index template) vào `SqliteSessionRepository` (`sqlite/session.rs:12-14`), dựng ở `server_control.rs:357-361`.
  - (b) Post-process trong `login_db` sau `load`: duyệt `homdo/trangbi/tientrang/tuideo/luulang` và điền field thiếu từ `ctx.data.items`. Giữ được bất biến "repository chỉ làm SQL" (`login.rs:7`), diff nhỏ hơn.
- `save()` (`sqlite/session.rs:299-314`) chỉ persist `slot/itemid/quantity/damage`, nên các field template **suy ra được**, không cần migration.
- **Phạm vi G4 đã được thu hẹp.** Kiểm tra `src/db/modern/sqlite/session.rs:176-215` và schema cho thấy `khang` (cột `proofkind`, là loại chứng nhận bảo hộ, không phải % kháng) và `texp` (cột `growexp`, EXP phát triển của trang bị) **đã được đọc và ghi lại** — không mất sau relog. Chỉ các trường template (`lv`, `loai`, `thuoctinh`, `giatri_thuoctinh`, `_1`/`_2`) là thực sự thiếu.
- Nhân vật `Session.texp` map sang `characters.curexp` (`session.rs:27,302`) — đã persist, không cần xử lý.
- `long_val`/`giatri_long` (cột `element`, `element_value`) cũng đã được load.

### 4.5 Reset trạng thái khi re-login (G10)

Tại `login.rs:43-48`, cạnh `authed = false`, thêm `conn.session.logined = false;` và `conn.session.in_world = false;`. Vá cả G10 và G17.

### 4.6 Đồng bộ battle sender (G11)

`server_control.rs:242-253` seed `battle_service.register_sender` từ snapshot cũ trước khi session mới được load và ghi lại ở `:425-428`. Di chuyển `register_sender` xuống sau `:425-428`, hoặc cập nhật lại sender ngay khi `login_register` thành công.

### 4.7 Đường server switch và server list (G16)

Chỉ làm sau khi 4.1–4.4 ổn định:
- Builder `server_switch_frame(c1, c2)` ở `spawn.rs` cạnh `DOUBLE_LOGIN` (`:44-60`) và `announce_frame` (`:299-302`): `crate::protocol::frame("2E01", &format!("{c1:02X}{c2:02X}"))`.
- Cặp hợp lệ: `c1∈{1,2} ∧ c2∈{2,3}`, hoặc `c1=0 ∧ c2=7` (`005045a0_FUN_005045a0.c:55-77`).
- Trigger: lệnh GM cạnh `/kick` (`gm.rs:390-419`). Client đọc `Server.ini`, mở lại socket port 6414 (`0x190E`), rồi chạy lại `0x01` → `0x03`.
- **Giữ nguyên** `dispatcher.rs:376` (`OP_SERVER_SWITCH => unimplemented`): `0x2E` là S→C-only, client không gửi.
- `0x36` (server list, C→S) là hướng inbound đáng implement: trả `[36][idx1][lv1]...`, `idx` 1-based, `lv ∈ 0..3`, **tổng payload phải chẵn** (độ dài lẻ gây `ERangeError` phía client).

### 4.8 Đính chính tài liệu nghiên cứu

Các điểm sau nên sửa trong tài liệu nguồn trước khi dùng làm spec:

1. `opcode_00_01.md:223-225`: ghi ACK là `F4 44 00 00`. Đúng là `F4 44 01 00 00`.
2. `opcode_00_01.md:139`: ghi `FUN_0050f2f8` là "ngắt socket". Sai: nó chỉ chuyển UI state. Teardown đến từ `ClientSocket1Disconnect` (`0x0050D030`) khi server đóng TCP.
3. `opcode_1a.md:21`: gọi `32 01 10 27 RR RR` là "keepalive". Sai: đó là nút debug ẩn `Button33Click` với 2 byte random.
4. Token framing là `F4 44`, không phải `44 F4`.
5. "~30 ms tick" (`login_flow_research.md:40,108`, `opcode_00_01.md:40`) không có hằng số nào trong binary. Chỉ có 100 ms pacing tại `005163e4`.

---

## 5. Thứ Tự Thực Hiện

| Bước | Việc | Gap | File chính | Ghi chú |
|:-:|---|---|---|---|
| 1 | Config `idle_timeout_secs` + `Conn.last_seen` + arm `0x00` | G5 | `config.rs`, `session.rs`, `dispatcher.rs`, `system.rs` | Nền cho mọi health check |
| 2 | Nhánh idle timeout trong `select!` + kick `ConnectionLost` + yield trước close | G1, G6, G8 | `server_control.rs` | Chặn leak `clients`/ghost session |
| 3 | `set_nodelay` + log lỗi read + giới hạn channel | G7, G8 | `server_control.rs` | Hardening |
| 4 | Dọn incumbent chết trước double-login guard | G2 | `login.rs`, `server_control.rs` | Gỡ tắc reconnect thật |
| 5 | `recompute_stats()` sau load | G3 | `sqlite/session.rs:112` hoặc `login.rs:207` | 1 dòng, sửa gear bonus |
| 6 | Merge template item trong load | G4 | `sqlite/session.rs`, `login.rs` | Khôi phục equip gate |
| 7 | Reset `logined`/`in_world`; sửa `register_sender` | G10, G11, G17 | `login.rs`, `server_control.rs` | Chống treo |
| 8 | `system_alert` cho mọi đường kick | G6 | `login.rs`, `gm.rs`, `character.rs` | Client hiện lý do |
| 9 | `stop()` đóng socket thật | G9 | `server_control.rs` | Shutdown sạch |
| 10 | `server_switch_frame` + lệnh GM | G16 | `spawn.rs`, `gm.rs` | Đường reconnect native |
| 11 | Server list `0x36` | G16 | `spawn.rs`, `dispatcher.rs` | Chỉ khi client cần |
| 12 | Các gap THẤP còn lại (G12-G15) | G12-G15 | `session.rs`, `auto_save.rs`, `web/app.rs` | Dọn dẹp |

Bước 1–4 là phạm vi tối thiểu để giải quyết vấn đề reconnect/health ban đầu. Bước 5–6 độc lập và có thể làm song song.

---

## 6. Kế Hoạch Kiểm Thử

Theo `AGENTS.md`: mọi test mới đặt trong `tests/`. Không chạy test ghi đè `DB/ts_dream.db`; dùng `sqlite::memory:?cache=shared` hoặc `TS_TEST_DB_URL=sqlite://DB/ts_dream_test.db`.

| Test | Đường kiểm | Loại |
|---|---|---|
| `idle_timeout_kicks_silent_connection` | Socket không gửi byte nào quá `idle_timeout` → nhận `F4 44 .. 00 13`, sau đó EOF, `clients` rỗng | Integration (TCP thật, `ServerControl`) |
| `system_alert_flushed_before_close` | Frame kick đến được socket trước khi đóng | Integration |
| `zero_byte_ack_refreshes_liveness` | Frame `F444010000` cập nhật `last_seen`, không sinh frame trả lời | Unit (dispatcher) |
| `double_login_evicts_dead_incumbent` | Incumbent quá hạn → login mới được chấp nhận, incumbent bị `disconnect_player` | Integration |
| `double_login_rejects_live_incumbent` | Incumbent còn sống → vẫn `DOUBLE_LOGIN` | Integration |
| `relogin_restores_gear_bonus` | Sau `load`, `int2..agi2` khác 0 khi có trang bị; frame `05 03` đúng | Integration (DB test) |
| `relogin_restores_item_template_fields` | `lv`, `loai`, `thuoctinh` đúng sau load | Integration (DB test) |
| `relogin_resets_logined_flag` | Gửi `0x01` hai lần không làm client treo | Unit |
| `stop_closes_sockets` | `stop()` đóng socket thật | Integration |

**Đối chiếu wire:** dùng `golden/` (18 packet) để diff frame kick và frame ACK sau khi sửa.

**Lệnh chạy:**
```bash
cargo test --all-targets --no-fail-fast
```

---

## 7. Quyết Định Còn Mở (Cần Xác Nhận Trước Khi Triển Khai)

Theo quy trình grilling, các câu hỏi dưới đây cần người dùng trả lời trước khi sửa code. Mỗi câu có khuyến nghị.

**Q1 — Giá trị mặc định của `idle_timeout_secs`?** ✅ ĐÃ CHỐT: `300` giây, cấu hình qua `TS_IDLE_TIMEOUT_SECS`.
Client không gửi heartbeat, nên timeout đủ dài để không kick người chơi AFK nhưng vẫn tránh leak lâu dài.

**Q2 — Idle timeout có phải là nguồn duy nhất đánh dấu "còn sống"?** ✅ ĐÃ CHỐT: Có. Mọi byte đọc được (kể cả ACK `00` và mọi opcode khác) làm mới `last_seen`.

**Q3 — Khi incumbent chết, dọn bằng cách nào?** ✅ ĐÃ CHỐT: Gọi `disconnect_player(old)` rồi `login_register` lại (gỡ tắc ngay).
Từ chối và chờ timeout bị loại vì người chơi sẽ bị chặn thêm tối đa `idle_timeout`.

**Q4 — (ĐÃ XÓA)** Câu hỏi về `khang`/`texp` dựa trên nhận định sai là các trường này bị mất sau relog. Đã kiểm tra: cả hai đều được load và save (xem §4.4). Không còn câu hỏi.

**Q5 — Có làm `0x2E` server switch và `0x36` server list ngay trong đợt này không?** ✅ ĐÃ CHỐT: Không. Đưa vào đợt sau, sau khi bước 1–7 ổn định.

**Q6 — Có bật keepalive ở tầng TCP (`socket2`) hay chỉ dùng idle timeout ở tầng ứng dụng?** ✅ ĐÃ CHỐT: Chỉ idle timeout ở tầng ứng dụng, không thêm dependency.

---

## 8. Rủi Ro

| Rủi ro | Mức | Giảm thiểu |
|---|:-:|---|
| Idle timeout kick nhầm người chơi AFK lâu | Trung bình | Timeout mặc định 300 s; cấu hình được; ghi log `warn` để theo dõi |
| `disconnect_player` trên incumbent đang ghi DB dở | Thấp | Persist là transaction nguyên tử (`persist_sessions_transaction`) |
| Frame kick không tới socket do đóng quá sớm | Trung bình | Yield/flush trước khi `close = true` (bước 2) |
| `recompute_stats` đổi chỉ số đã hiển thị | Thấp | Đây là sửa đúng; kiểm bằng `relogin_restores_gear_bonus` |
| ACK `F4 44 01 00 00` chưa xác minh lại hex | Thấp | Xác minh trước bước 1 (xem §2) |

---

## 9. Phạm Vi Ngoài Kế Hoạch

- Chuyển đổi tài liệu nghiên cứu sang dạng spec chính thức (chỉ làm §4.8).
- Đọc lại hex gốc `0078aabe` và `0050f2f8` (không có file `.asm.txt` trong thư mục decompile hiện tại).
- Logined1 `08 01`: đã nêu là G17 trong bảng §3 của plan này. Gap này nằm ngoài phạm vi reconnect/health; chỉ theo dõi.
