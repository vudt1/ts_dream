# Báo cáo Nghiên cứu: `sys_msg_frame` (`0x02`) và Treo Kịch bản Eve / Bạch Lão Nhân (`15009`) tại Map `10817`

- **Ngày**: 2026-11-18
- **Phạm vi**: Nghiên cứu thuần túy đối chiếu mã nguồn Rust (`src/`), tài liệu quy chuẩn (`CODING_STANDARDS.md`, `docs/`), đặc tả opcode (`spec/`, `.scratch/client-pseudo-op-code/`), và mã dịch ngược client Delphi `aLogin.exe` (`client_pseudo_c/`). Không chỉnh sửa mã nguồn hay test.

---

## Tóm tắt điều hành (Executive Summary)

1. **Vấn đề 1 — `sys_msg_frame` (`src/server/spawn.rs:363-367`)**:
   - Hàm hiện phát `F4 44 <len> 02 01 02 <visc>`, tức đang gửi **`OP_CHAT (0x02)` + `CHAT_SUB_ANGEL (0x01)`** (`(Thần)Thiên thần`, kênh bị gate gắt phía client) thay vì kênh Hệ thống `(Công bố hệ thống)`, đồng thời chỉ nhét **1 byte `0x02`** thay cho trường `id: 4B LE` bắt buộc ở `case 1` của `FUN_0078b6fc`, khiến client ăn nhầm `0x02` + **3 byte đầu của thông điệp** làm `id` và **cắt mất 3 ký tự đầu** của nội dung.
   - Phát hiện phụ liên quan: `announce_frame` (`src/server/spawn.rs:370-372`) gọi `text_banner(CHAT_SUB_SYSTEM, msg)` (`0x02 0x0C`), chèn thừa `"00000000"` (`4` byte `0x00`) vào đầu chuỗi `msg` của `0x02 0x0C` vốn **không có trường `id`**.
   - **Chuẩn đúng của `aLogin.exe` (`FUN_0078b6fc`)**: dùng **`0x02 0x0C` (`CHAT_SUB_SYSTEM`) không kèm `id`** (`F4 44 <len> 02 0C <visc>` — khuyến nghị cho `sys_msg_frame` và sửa `announce_frame`), hoặc **`0x02 0x00` (`CHAT_SUB_BROADCAST`) kèm `id = 0` (`00000000`)** (`F4 44 <len> 02 00 00 00 00 00 <visc>`).

2. **Vấn đề 2 — Kẹt SSE tại `F444040014080100`, nói chuyện Bạch Lão Nhân (`15009`) tại map `10817` & nghi vấn chưa nạp Eve lúc startup**:
   - **Nguyên nhân gốc nghiêm trọng nhất (Self-Deadlock 100%)**: Trên kết nối TCP thật, [`handle_client_connection`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L893-L910) giữ non-reentrant `tokio::sync::Mutex` thông qua `lock_player_operations([logined_id])` **xuyên suốt** lời gọi `dispatcher::dispatch(...)`. Khi gói `c2s` `F444040014080100` (`0x14 0x08` Door `1` trên map `10817`) đi vào `talk::handle_talk_warp` → `quest::handle_warp_confirm` → [`perform_party_warp_with_resolved`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1084), hàm này **gọi lại `lock_player_operations([id])` lần thứ hai trên chính Tokio task đó**, gây **self-deadlock vĩnh viễn**.
   - **Hệ quả trực tiếp lên Bạch Lão Nhân (`id# 15009`, `MapObjectID = 4`)**:
     1. Một khi task `handle_client_connection` đã deadlock tại `14 08 01 00`, vòng `select!` không bao giờ đọc socket nữa và không phát `s2c`, đồng thời client giữ khóa bận `PlayerState+0xA094 = 1`. Mọi thao tác sau đó (click Bạch Lão Nhân `15009`) không bao giờ lên SSE Terminal.
     2. **Ngay cả khi không đụng `14 08 01 00` trước**, kịch bản Eve của Bạch Lão Nhân (`10817.eve`, `MapObjectID = 4`, `eve_no = 4`) kết thúc bằng `result_type = 2, parameter = 1` (`DomainEffect::Warp { warp_id: 1 }`), và [`eve_runner.rs:952`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L952) cũng gọi `perform_party_warp_with_resolved` → **tiếp tục self-deadlock đúng tại chỗ đó**. Lỗi này tồn tại cả ở `MeetDoor`/`WalkStep` và nhiều handler khác gọi `lock_player_operations` bên trong `dispatch()`, nhưng lọt qua bộ test vì các test gọi thẳng `dispatcher::dispatch(...)` mà không qua vòng ngoài `server_control.rs`.
   - **Trạng thái nạp Eve khi server start**: Server **CÓ nạp `Data/eve.emg`** vào `data.scene_eve_data` lúc khởi động qua `GameData::load` (`src/main.rs:112` → `src/data/loader.rs:368`) và `EVE_EVENTS_ENABLED` mặc định `true` (`src/server/handlers/npc_event.rs:28`). Tuy nhiên `src/main.rs:119-126` lại log `0 talks` (`data.talks.len()`, thuộc legacy `.ini` đã bỏ) và **không hề in `data.scene_eve_data.len()`**, khiến console lúc start trông như chưa nạp dữ liệu Eve.

---

## Phần 1: Phân tích `sys_msg_frame` trong `src/server/spawn.rs`

### 1.1. Hiện trạng cài đặt & các điểm gọi

Tại [`src/server/spawn.rs:362-367`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L362-L367):

```rust
/// Build system message frame via channel `020102` (`F444 + len + 0201 + 02 + msg`).
pub fn sys_msg_frame(msg: &str) -> String {
    let visc = crate::encoding::viscii_encode(msg);
    let body = format!("02{}", encoder::strhex(&visc));
    crate::protocol::frame("0201", &body)
}
```

Ngay phía trên và dưới hàm này còn có 2 helper liên quan trong [`src/server/spawn.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs):
- [`system_broadcast_frame(id, chat_raw)`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L304-L306): gọi `chat_frame(CHAT_SUB_BROADCAST, id, chat_raw)` → phát `02 00 <id: 4B LE> <chat_raw>`.
- [`text_banner(sub, msg)`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L350-L355) & [`announce_frame(msg)`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L370-L372): `announce_frame` gọi `text_banner(CHAT_SUB_SYSTEM, msg)` → phát `02 0C 00000000 <visc>`.

**Các call site trong `src/`**:
- Chuỗi `Logined1` lúc đăng nhập: [`src/server/spawn.rs:659,662`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L659-L664) (banner thời gian `Thoi gian: ...` và `"Welcome to TS Dream"`).
- Toàn bộ thông báo hệ thống / lỗi / phản hồi lệnh trong các handler:
  - [`src/server/gm.rs:26,140,402,454`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/gm.rs#L25-L27)
  - [`src/server/handlers/chat.rs:48,105,154,160,198,271,279,318,426`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/chat.rs#L48-L50)
  - [`src/server/handlers/battle.rs:250,254,265,269,277,281,285,423`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L250)
  - [`src/server/handlers/crafting.rs:22..290`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/crafting.rs#L22)
  - [`src/server/handlers/eve_runner.rs:201..264`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L201)
  - [`src/server/handlers/quest.rs:1256`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1256), [`src/server/handlers/respec.rs:147..296`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs#L147), [`src/server/handlers/shops.rs:703..795`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L703), [`src/server/handlers/system.rs:99..488`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs#L99), [`src/server/handlers/talk.rs:145..389`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L145), [`src/server/handlers/use_item/mod.rs:178`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs#L178), [`src/battle/service.rs:270`](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs#L270).
- Tài liệu và test đang khóa chuỗi `"020102"`:
  - [`CODING_STANDARDS.md:37`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L37): *"Kênh Chat Hệ thống `020102`: Sử dụng duy nhất hàm chuẩn hóa tại `src/server/spawn.rs::sys_msg_frame(msg)`"* (và dòng `40` cảnh báo cấm dùng `02 0B` làm thông báo chữ đỏ vì `02 0B` là form Quân Đoàn trên một số bản/suy đoán cũ, dù trên `aLogin.exe` `02 0B` là memo `#end`).
  - [`docs/adr/0007-compound-wire-parity-and-crafting-engine.md:36`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0007-compound-wire-parity-and-crafting-engine.md#L36): mô tả `02 01 02 <len> <nội dung VISCII>` (thực tế `spawn.rs:365` còn không ghi `<len>`, mà hardcode hằng `"02"`).
  - [`tests/server_network/chat.rs:194,248,268`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/chat.rs#L248): `assert!(spawn::sys_msg_frame("hi").contains("020102"))`.

### 1.2. Ý nghĩa thực sự của `02 01 02 <visc>` trên `aLogin.exe`

Đối chiếu trực tiếp hàm xử lý S→C `OP_CHAT (0x02)` của `aLogin.exe` tại [`client_pseudo_c/0078b6fc_FUN_0078b6fc.c:55-230`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078b6fc_FUN_0078b6fc.c#L55-L230), bảng vector [`spec/exhaustive_by_opcode/opcode_02_full_payloads.md:7-29`](file:///D:/VUDT/GIT_PCC/ts_dream/spec/exhaustive_by_opcode/opcode_02_full_payloads.md#L7-L29), đặc tả [`opcode_02.md`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_02.md#L71-L141) và hằng số đã khai báo tại [`src/protocol/mod.rs:74-106`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L74-L106):

- `0x02` là `OP_CHAT`. Byte kế tiếp (`RestPayload[0]`) là `SubOp` (kênh chat):
  - `0x00` = `CHAT_SUB_BROADCAST` (`(Công bố hệ thống)`)
  - `0x01` = `CHAT_SUB_ANGEL` (`(Thần)Thiên thần` — thường bị hiểu nhầm với kênh "All"/Thiên thần)
  - `0x0C` = `CHAT_SUB_SYSTEM` (`(Công bố hệ thống)`)

Khi `sys_msg_frame("Welcome to TS Dream")` phát `F4 44 ... 02 01 02 57 65 6C 63 6F 6D 65 ...`, client `FUN_0078b6fc` nhảy vào **`case 1:`** ([`0078b6fc_FUN_0078b6fc.c:79-101`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078b6fc_FUN_0078b6fc.c#L79-L101)) và gây ra **hai lỗi đồng thời**:

1. **Sai SubOp (`0x01` = `CHAT_SUB_ANGEL` thay vì Hệ thống)**:
   - Dòng `80-84` kiểm tra điều kiện gắt: `(class ∉ [5..8] || form_0x404+0xB4() != 0) && (ChannelForm+0x16D != 0 || *(short*)(PlayerRec+0x44+4) == 0xB3B6)`. Nếu không qua gate, tin bị bỏ qua hoàn toàn.
   - Nếu qua gate, dòng `96-97` gọi `FUN_007ab870(..., id, msg, '\x01')` (`tag = 1`), ghép tiền tố **`(Thần)Thiên thần`** ([`007ab870_FUN_007ab870.c:257-267`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/007ab870_FUN_007ab870.c#L257-L267)), **không phải** kênh Hệ thống `(Công bố hệ thống)`.

2. **Thiếu 3 byte trong trường `id: 4B LE` → cắt cụt 3 ký tự đầu của `msg`**:
   - Ở `case 1:` (`0078b6fc_FUN_0078b6fc.c:85-95`), client đọc **4 byte** `RestPayload[1..4]` làm `id` (`u32 LE` qua `FUN_0077ef7c`) và đọc `msg = RestPayload[5..]` (`_LStrCopy(..., 6, len - 5, ...)`).
   - Vì `sys_msg_frame` chỉ chèn **1 byte `0x02`** trước `<visc>`, client lấy `[0x02, byte0, byte1, byte2]` (tức `0x02` + 3 ký tự đầu `"Wel"`) làm `id` rác (`0x6C655702`), và hiển thị `msg` bắt đầu từ ký tự thứ 4 (`"come to TS Dream"`)!

### 1.3. Định dạng chính xác của kênh Hệ thống (`0x02`) trên `aLogin.exe`

Trong [`0078b6fc_FUN_0078b6fc.c`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078b6fc_FUN_0078b6fc.c) và [`007ab870_FUN_007ab870.c:254-256`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/007ab870_FUN_007ab870.c#L254-L256), cả hai nhánh `tag = '\0'` đều **không bị gate kênh** và đều render nhãn **`(Công bố hệ thống)`** (`DAT_007abd54`):

| Thuộc tính | `0x02 0x0C` (`CHAT_SUB_SYSTEM`) — **Khuyến nghị** | `0x02 0x00` (`CHAT_SUB_BROADCAST`) — Thay thế |
|---|---|---|
| Hằng số `src/protocol/mod.rs` | `CHAT_SUB_SYSTEM` (`0x0C`, dòng `106`) | `CHAT_SUB_BROADCAST` (`0x00`, dòng `82`) |
| Nhánh C trong `FUN_0078b6fc` | `case 0xc:` (`0078b6fc.c:220-230`) | `case 0:` (`0078b6fc.c:63-78`) |
| Layout payload chuẩn (sau `F4 44 <len>`) | **`02 0C <visc_bytes>`** (**KHÔNG có `id`**) | **`02 00 <id: 4B LE> <visc_bytes>`** (`id = 00 00 00 00`) |
| Độ dài trường tiền tố trước `<visc>` | `0` byte (`_LStrCopy(RP, 2, len - 1, &msg)`) | `4` byte (`RP[1..4]` = `00 00 00 00`) |
| `id` truyền vào `FUN_007ab870` | Hằng `0` (hardcoded) | Đọc từ `RP[1..4]` (`0` nếu là `00000000`) |
| `tag` truyền vào `FUN_007ab870` | `'\0'` (`(Công bố hệ thống)`) | `'\0'` (`(Công bố hệ thống)`) |
| Mã hóa text | VISCII 1.1 (`viscii_encode(msg)`), **không** có byte length prefix | VISCII 1.1 (`viscii_encode(msg)`), **không** có byte length prefix |

> [!WARNING]
> **Lỗi đồng thời tại `announce_frame` (`src/server/spawn.rs:370-372`)**:
> Hiện `announce_frame(msg)` gọi `text_banner(CHAT_SUB_SYSTEM, msg)` (`spawn.rs:350-355`), phát ra `02 0C 00 00 00 00 <visc>`. Theo `case 0xc:` ở `0078b6fc.c:220-230`, `02 0C` **không bỏ qua 4 byte `id`**, nên 4 byte `00 00 00 00` sẽ lọt trực tiếp vào đầu chuỗi `msg` (làm chuỗi PChar/Delphi bị coi là kết thúc bằng `\0` ngay đầu dòng!). Ngược lại, `text_banner` lại khớp 100% với `02 00` (`CHAT_SUB_BROADCAST`).

### 1.4. Phương án giải quyết Đề xuất cho Vấn đề 1

- **Phương án 1A (Khuyến nghị — Chuẩn `0x02 0x0C` `CHAT_SUB_SYSTEM`)**:
  - Sửa [`sys_msg_frame`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L363-L367) (và bỏ `00000000` khỏi `02 0C` trong [`announce_frame`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L370-L372)) để phát đúng layout `02 0C <visc>` theo [`src/protocol/mod.rs:74-75,106`](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs#L74-L106):
    ```rust
    pub fn sys_msg_frame(msg: &str) -> String {
        let visc = crate::encoding::viscii_encode(msg);
        crate::protocol::frame(&format!("02{:02X}", CHAT_SUB_SYSTEM), &encoder::strhex(&visc))
    }
    ```
  - Nếu cần phân biệt `announce_frame` (global server notice) với `sys_msg_frame` (local system notice), có thể giữ `announce_frame` dùng `text_banner(CHAT_SUB_BROADCAST, msg)` (`02 00 00000000 <visc>`) và `sys_msg_frame` dùng `02 0C <visc>` (hoặc cả hai dùng chung `02 0C <visc>`).
- **Phương án 1B (Thay thế — `0x02 0x00` `CHAT_SUB_BROADCAST`)**:
  - Cho `sys_msg_frame(msg)` gọi `text_banner(CHAT_SUB_BROADCAST, msg)` (`02 00 00000000 <visc>`), đồng thời sửa `announce_frame` không nhét `00000000` vào `02 0C`.
- **Cập nhật kèm theo bắt buộc**:
  - Cập nhật [`CODING_STANDARDS.md:37`](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L37) và [`docs/adr/0007-compound-wire-parity-and-crafting-engine.md:36`](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0007-compound-wire-parity-and-crafting-engine.md#L36) từ `020102` sang kênh chuẩn mới (`020C` / `CHAT_SUB_SYSTEM` hoặc `0200` / `CHAT_SUB_BROADCAST`).
  - Cập nhật các assertion `"020102"` trong [`tests/server_network/chat.rs:194,248,268`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/chat.rs#L248) (và các test liên quan trong `tests/` nếu có).

---

## Phần 2: Phân tích Map `10817`, Bạch Lão Nhân (`15009`), 3 Gói SSE và Nạp Dữ liệu Eve

### 2.1. Giải mã từng byte 3 gói tin trong Live SSE Terminal

Trước hết, xác nhận cơ chế ghi log của Live SSE Terminal tại [`src/web/server_control.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs):
- Chiều `c2s`: `conn.decoder.feed(&buf[..n])` giải XOR `0xAD` thành hex chuẩn `F444...` rồi mới gọi `push_log("c2s", ...)` ([`server_control.rs:865-876`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L865-L876)).
- Chiều `s2c`: gọi `push_log("s2c", ...)` trước khi `frame::encode_to_wire` XOR `0xAD` để gửi socket ([`server_control.rs:772-777,995-1000`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L995-L1000)).
→ Cả 3 chuỗi hex trên SSE Terminal đều là **decoded frame (post-XOR / pre-XOR)**.

#### Gói 1: `c2s [127.0.0.1:58262] F4440B0025013132372E302E302E31`
| Offset | Hex | Giá trị | Ý nghĩa |
|---|---|---|---|
| `0..2` | `F4 44` | Magic `0xF4 0x44` | Header chuẩn (`HEADER_TS_MAGIC`, `src/protocol/mod.rs:11`) |
| `2..4` | `0B 00` | `11` (`u16 LE`) | Độ dài payload |
| `4` | `25` | `0x25` | `OP_LOGIN_COMPLETE` (`src/protocol/mod.rs:252`) |
| `5` | `01` | `0x01` | `sub = 1` (Client báo đã load xong map / vào thế giới) |
| `6..15` | `31 32 37 2E 30 2E 30 2E 31` | `"127.0.0.1"` | Chuỗi IP client gửi lên |
- **Xử lý ở server**: [`dispatcher.rs:353-358`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L353-L358) → [`login::handle_login_complete`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/login.rs#L233-L319). Hàm kiểm tra `sub == 1 && conn.session.id != 0`, đặt `in_world = true`, phát `player_appear` cho người cùng map, gửi `27 09` (`server_name_frame`), đồng bộ người chơi khác cùng map, lặp `data.npc_on_map` (rỗng ở `load_binary`) và đồng bộ festival.

#### Gói 2: `s2c [127.0.0.1:58262] F44414002709E0930400C4000000095453204F6E6C696E65`
| Offset | Hex | Giá trị | Ý nghĩa |
|---|---|---|---|
| `0..2` | `F4 44` | Magic | Header chuẩn |
| `2..4` | `14 00` | `20` (`u16 LE`) | Độ dài payload |
| `4..5` | `27 09` | `0x27`, `0x09` | `OPCODE_SERVER_NAME` (`src/protocol/mod.rs:51,256`) |
| `6..10` | `E0 93 04 00` | `300000` (`u32 LE`, `0x000493E0`) | `my_id` (Player ID = `300000`) |
| `10..14` | `C4 00 00 00` | `196` (`u32 LE`) | Hằng `"C4000000"` trong `spawn::server_name_frame` (`src/server/spawn.rs:380`) |
| `14` | `09` | `9` (`u8`) | Độ dài tên server |
| `15..24` | `54 53 20 4F 6E 6C 69 6E 65` | `"TS Online"` | Tên server gửi từ [`login.rs:269`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/login.rs#L269) |

#### Gói 3: `c2s [127.0.0.1:58262] F444040014080100`
| Offset | Hex | Giá trị | Ý nghĩa |
|---|---|---|---|
| `0..2` | `F4 44` | Magic | Header chuẩn |
| `2..4` | `04 00` | `4` (`u16 LE`) | Độ dài payload |
| `4` | `14` | `0x14` | `OP_NPC_EVENT` (`src/protocol/mod.rs:136`) |
| `5` | `08` | `0x08` | `sub = 8` (Door / vùng dịch chuyển) |
| `6..8` | `01 00` | `1` (`u16 LE`) | `door_id = 1` (`idtalking = 1`) |
- **Nguồn phát phía `aLogin.exe`**: Theo [`asm_cs14_78301f_builders.asm.txt:61-125`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/asm_cs14_78301f_builders.asm.txt#L61-L125) và [`opcode_14.md:242`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/client-pseudo-op-code/opcode_14.md#L242), hàm `FUN_005EA3E0` gửi `[0x14][a08E=0x08][a08A: u16 LE]` khi người chơi chạm/kích hoạt cổng/vùng dịch chuyển `1` trên map `10817`, đồng thời **bật cờ bận `PlayerState+0xA094 = 1`** (`asm_cs14_78301f_builders.asm.txt:124`) chờ server phản hồi (`14 07`+`0x0C` hoặc `14 08`).
- **Xử lý phía server**: [`dispatcher.rs:304`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L304) → [`talk::handle_talk`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L97) → [`talk::handle_talk_warp`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L417-L428) (`conn.session.idtalking = 1`) → [`quest::handle_warp_confirm`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1238-L1353).

---

### 2.2. Nguyên nhân gốc rễ #1 (Nghiêm trọng): Self-Deadlock tại `lock_player_operations`

Đây là **lỗi chí mạng trực tiếp làm treo cứng toàn bộ luồng tại `F444040014080100` và làm kẹt hoàn toàn khi nói chuyện với Bạch Lão Nhân (`15009`)**.

1. **Cơ chế khóa không đệ quy (`non-reentrant`)**:
   - [`lock_player_operations`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/session.rs#L837-L864) dùng `tokio::sync::Mutex<()>`, **không hỗ trợ khóa lồng nhau (re-entrant) trên cùng một task**.
2. **Vòng kết nối TCP giữ khóa bao ngoài `dispatch()`**:
   - Tại [`src/web/server_control.rs:873-910`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L873-L910):
     1. Dòng `873-876`: ghi log `c2s [127.0.0.1:58262] F444040014080100` lên SSE.
     2. Dòng `893-894`: gọi `let _operation_guards = lock_player_operations([300000]).await;` — **chiếm khóa của player `300000`**.
     3. Dòng `910`: gọi `dispatcher::dispatch(&mut conn, &decoded, &data, &service, &env).await` trong khi **vẫn đang giữ `_operation_guards`**.
3. **Handler bên trong `dispatch()` gọi lại `lock_player_operations([300000])`**:
   - Khi xử lý `F444040014080100`:
     - `handle_warp_confirm` (`src/server/handlers/quest.rs:1331`) gọi `resolve_door_warp(data, &conn.session, 10817, 1)` → trả về `Some(resolved)` (nếu trả `None` thì dòng `1351` đã gọi `end_talk` phát `F44402001408` ra `s2c`, không im lặng).
     - Dòng `1337` gọi `perform_party_warp_with_resolved(&mut conn.session, resolved, env.pool, out).await`.
     - Tại [`src/server/handlers/quest.rs:1084`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1084), `perform_party_warp_with_resolved` gọi:
       ```rust
       let _guards = crate::server::session::lock_player_operations(participant_ids).await;
       ```
       với `participant_ids = [300000]`.
     - **Kết quả**: Task `handle_client_connection` tự chờ chính mutex mà mình đang giữ ở `server_control.rs:894` → **TREO VĨNH VIỄN (Deadlock)**.
     - Không có frame `s2c` nào được đẩy vào `tx` hay log ra SSE; vòng `tokio::select!` (`server_control.rs:830`) ngừng đọc socket (`read_half.read`) vĩnh viễn; mọi packet về sau (như click Bạch Lão Nhân) nằm kẹt trong TCP buffer, không bao giờ xuất hiện trên SSE Terminal.

4. **Đối chiếu riêng với Bạch Lão Nhân (`id# 15009`, `MapObjectID = 4`) trên Map `10817`**:
   - Theo dữ liệu `Data/eve.emg` (`10817.eve`, trích trong [`eve_full.jsonl`](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/special-event-refactor/data_files/eve/eve_full.jsonl#L2) và kiểm chứng bởi [`tests/eve_quest/npc_eve_resolve.rs:60-72`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/npc_eve_resolve.rs#L60-L72)):
     - Map `10817` có 6 NPC (`1..=6`) và 1 Door (`id = 1`).
     - **Bạch Lão Nhân (`npc_id = 15009`)** là `MapObjectID = 4`, tọa độ `(1430, 660)`, gắn `eve_no = 4` (`when_happen = OnClickObject`).
     - `eve_no = 4` gồm **7 kết quả**: 6 câu thoại (`result_type = 1`, `mean_no = 10447..10452`) và kết quả thứ 7 là **`result_type = 2, parameter = 1` (`DomainEffect::Warp { warp_id: 1 }`)**.
   - Điều này có nghĩa là **ngay cả khi không có gói `14 08 01 00` trước đó**, nếu người chơi đi lại gần Bạch Lão Nhân (`±150 px` quanh `(1430, 660)`, gate tại [`talk.rs:115-121`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L115-L121)), bấm nói chuyện (`14 01 04 00`) và bấm Tiếp tục (`14 06`) hết 6 câu thoại:
     - Ở bước thứ 7 (`DomainEffect::Warp { warp_id: 1 }`), [`eve_runner.rs:952`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L952) gọi `perform_party_warp_with_resolved` → **`quest.rs:1084` gọi `lock_player_operations([id])` → vẫn SELF-DEADLOCK y hệt!**

5. **Phạm vi ảnh hưởng rộng của cùng lỗi Deadlock này**:
   Kiểm tra toàn bộ `src/server/handlers/` cho thấy lỗi tái khóa `lock_player_operations` bên trong `dispatch()` còn xảy ra ở:
   - [`src/server/handlers/eve_runner.rs:952`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs#L952) (mọi `DomainEffect::Warp` từ `ClickNpc`, `ClickDoor`, `MeetDoor`, `WalkStep` — đáng chú ý ở [`movement.rs:38-40`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L38-L40) có comment né deadlock nội bộ `movement.rs` nhưng bỏ sót khóa bao ngoài ở `server_control.rs:894`).
   - [`src/server/handlers/event_npcs.rs:144,411,548`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/event_npcs.rs#L144) (`15312`, `14628`, `15313`, `61210`).
   - [`src/server/handlers/crafting.rs:61,296,388`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/crafting.rs#L61), [`src/server/handlers/respec.rs:121`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/respec.rs#L121), [`src/server/handlers/shops.rs:727`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs#L727), [`src/server/handlers/guild.rs:264,327,437`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/guild.rs#L264), [`src/server/handlers/battle.rs:479`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs#L479), [`src/server/trade_system.rs:177,210,241`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs#L177), [`src/server/handlers/trade_storage.rs:63,281,328,425,576,730`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs#L63).
   - **Vì sao toàn bộ test suite vẫn xanh?** Vì mọi test tích hợp trong `tests/` đều gọi thẳng `ts_dream::server::dispatcher::dispatch(...)` thay vì đi qua `ServerControl::handle_client_connection` (`server_control.rs:894`).

---

### 2.3. Các điểm cần lưu ý khác trên luồng `10817` & `Bạch Lão Nhân (15009)`

1. **Vị trí spawn tân thủ vs khoảng cách nói chuyện (`±150 px`)**:
   - Nhân vật mới tạo spawn tại `map_id = 10817, map_x = 442, map_y = 758` ([`src/server/handlers/character.rs:212-214`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/character.rs#L212-L214)).
   - Bạch Lão Nhân (`MapObjectID = 4`, `15009`) đứng tại `(1430, 660)`. Hàm [`resolve_npc`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L115-L121) kiểm tra `in_range = (-150..=150).contains(&dx) && (-150..=150).contains(&dy)`. Nếu client gửi `14 01 04 00` khi chưa cập nhật tọa độ tới gần `(1430, 660)`, server sẽ từ chối bằng `end_talk` (`F44402001408`).
2. **`data.npc_on_map` rỗng lúc login (`25 01`)**:
   - Trong [`login.rs:306-310`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/login.rs#L306-L310), `handle_login_complete` lặp `ctx.data.npc_on_map` để gửi `16 01` (`npc_appear_frame`), nhưng `GameData::load()` không nạp `NpcOnMap.txt` (`load_binary` chỉ nạp `eve.emg`). Tuy nhiên `aLogin.exe` tự nạp `TRole` (`EveNpcPlacement`) từ `Data/eve.emg` local (`EVE_BINARY_ANALYSIS_ALOGIN.md:25`) và `talk.rs:115-122` tra `data.scene_eve_data`, nên NPC vẫn hiện và click được.
3. **Khác biệt giữa `14 08 01 00` (`handle_warp_confirm`) và `MeetDoor` cho `10817.eve`**:
   - Trong `10817.eve`, `eve_no = 7` (`when_happen = [false, true, false, false]` = `OnStrokeObject`) có 6 thoại `10447..10452` + 2 animation (`rt=5`) + `rt=2 param=1`.
   - Khi đi qua [`resolve_door_warp`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1021-L1041) (từ `14 08 01 00`), `resolve_door_warp` trích thẳng `DomainEffect::Warp` và dịch chuyển ngay (bỏ qua 6 câu thoại + animation của `eve 7`), còn `MeetDoor` trong `movement.rs:118-150` chạy trọn chuỗi `EventSession`.

---

### 2.4. Kiểm chứng: Server start có nạp dữ liệu Eve không?

Kết luận từ mã nguồn khởi động: **CÓ nạp lúc start, nhưng log khởi động gây hiểu nhầm nghiêm trọng.**

1. **Đường nạp thực tế**:
   - [`src/main.rs:111-118`](file:///D:/VUDT/GIT_PCC/ts_dream/src/main.rs#L111-L118) gọi `GameData::load(&data_dir)`.
   - [`GameData::load`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L157-L163) gọi `load_binary(data_dir)` ([`src/data/loader.rs:314-374`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L314-L374)).
   - Trong `optional_binary` (`loader.rs:354,368`), `"eve.emg"` được đọc toàn bộ qua `EveDataLoader::load(&bytes)?` và gán vào `self.scene_eve_data` (`HashMap<u32, SceneEveData>`).
   - Cờ kích hoạt `EVE_EVENTS_ENABLED` tại [`src/server/handlers/npc_event.rs:28`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/npc_event.rs#L28) đã mặc định là **`true`**.
2. **Lý do khi server start trông như "chưa load dữ liệu Eve"**:
   - [`src/main.rs:119-126`](file:///D:/VUDT/GIT_PCC/ts_dream/src/main.rs#L119-L126) in:
     ```rust
     tracing::info!(
         "data loaded: {} npcs, {} items, {} skills, {} talks (data_dir={})",
         data.npcs.len(), data.items.len(), data.skills.len(), data.talks.len(), data_dir.display()
     );
     ```
     Trong đó `data.talks` chỉ dành cho legacy `.ini` nên luôn là **`0 talks`**, còn **`data.scene_eve_data.len()` hoàn toàn không được log ra**!
   - [`src/main.rs:104-107`](file:///D:/VUDT/GIT_PCC/ts_dream/src/main.rs#L104-L107) chỉ log `"Eve event engine enabled (TS_EVE_EVENTS)"` nếu có biến môi trường `TS_EVE_EVENTS=1|true`, dù mặc định engine đã bật sẵn.
   - **Rủi ro âm thầm (`optional_binary`)**: Vì `eve.emg` nằm trong `optional_binary` (`loader.rs:354-362`), nếu thư mục chạy server (`data_dir`) thiếu file `eve.emg`, server chỉ `continue;` bỏ qua mà không báo lỗi/cảnh báo; hoặc nếu `GameData::load` trả `Err`, `main.rs:114-117` fallback sang `GameData::default()` (rỗng hoàn toàn).

---

## Phần 3: Phương án Giải quyết Đề xuất cho Vấn đề 2

### 3.1. Giải quyết dứt điểm lỗi Self-Deadlock `lock_player_operations` (Ưu tiên P0)

Do `tokio::sync::Mutex` không cho phép re-entrancy và [`CODING_STANDARDS.md` §1.1](file:///D:/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md#L14-L23) yêu cầu khóa toàn bộ ID liên quan theo thứ tự tăng dần `[min..max]` trước khi đọc `registry_get`, có hai phương án kiến trúc sạch:

1. **Phương án 2A (Khuyến nghị — Bỏ vòng khóa trùng ở `server_control.rs:894` hoặc đồng bộ hóa đúng ranh giới)**:
   - Nhận xét: mỗi kết nối TCP trong [`handle_client_connection`](file:///D:/VUDT/GIT_PCC/ts_dream/src/web/server_control.rs#L788) vốn đã chạy tuần tự từng gói một trên 1 task riêng, nên việc giữ `lock_player_operations([logined_id])` quanh toàn bộ `dispatch()` thực chất chỉ để bảo vệ thao tác kéo/đẩy snapshot `conn.session ↔ online_sessions` (`server_control.rs:903-920`) khỏi bị giao dịch từ người chơi khác ghi đè.
   - Tuy nhiên, nếu giữ `lock_player_operations([logined_id])` ở `server_control.rs:894`, mọi handler bên trong gọi `lock_player_operations` đều deadlock.
   - **Cách làm chuẩn**:
     - **Hướng 2A-1 (Outer lock đầy đủ + `*_locked` bên trong)**: Mở rộng tập `operation_ids` ở `server_control.rs:878-894` (với leader đang có party trong các opcode `0x14`, `0x06`, `0x10`, ..., đưa cả `conn.session.id_mem` vào `operation_ids` trước khi khóa 1 lần duy nhất), và trong `dispatch()` gọi các biến thể `*_locked` (như [`perform_party_warp_locked_with_resolved`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1099) đã có sẵn!) — **không** gọi lại `lock_player_operations` trong handler nếu chạy dưới outer lock. *(Lưu ý: cần cẩn trọng để không làm gãy các caller từ `tests/` gọi thẳng `dispatch()` không có outer lock, hoặc tách rõ seam `dispatch_unlocked` vs `dispatch_locked`.)*
     - **Hướng 2A-2 (Khóa thuộc hoàn toàn về Handler/Service + Snapshot an toàn ở `server_control.rs`)**: Bỏ `lock_player_operations` ở `server_control.rs:893-894`, để từng handler tự gọi `lock_player_operations` theo chuẩn Two-Phase State Lock (đã có sẵn ở `quest.rs`, `crafting.rs`, `shops.rs`, `respec.rs`, `trade_system.rs`, `trade_storage.rs`, `guild.rs`, `battle.rs`, `event_npcs.rs`, và khớp 100% với cách `tests/` gọi `dispatch()`). Để không mất cập nhật chéo giữa hai người chơi (như shop buyer sửa session seller) khi `server_control.rs:903-920` đồng bộ `conn.session`: chuyển phần kéo/đẩy `online_sessions` ở `server_control.rs:903-920` vào dưới một `lock_player_operations([logined_id])` ngắn **trước** hoặc **sau** `dispatch()` — hoặc chỉ ghi lại `online_sessions` những trường kết nối cục bộ nếu session trong registry không bị giao dịch khác thay đổi, hoặc dùng cơ chế `Arc<tokio::sync::Mutex<Session>>` cho từng player online.
   - *Cảnh báo quan trọng*: Tuyệt đối **không** "vá nhanh" bằng cách chỉ bỏ `session.id` ra khỏi `perform_party_warp_with_resolved` để khóa riêng `[member_ids]` trong khi `server_control.rs:894` đang giữ `[leader_id]` — vì nếu `member_id < leader_id`, thứ tự khóa `[leader_id] -> [member_id]` sẽ vi phạm `Two-Phase State Lock` (`CODING_STANDARDS.md §1.1`) và có thể gây deadlock chéo giữa 2 leader/member.

2. **Phương án 2B (Tối thiểu hóa thay đổi nếu cần kiểm chứng nhanh trên map `10817`)**:
   - Dùng `tokio::task_local!` theo dõi tập `player_id` mà task hiện tại đang giữ lock trong `lock_player_operations`: nếu tập mới là tập con (ví dụ solo `[300000]` đã được giữ ở `server_control.rs:894`), trả về guard rỗng không lock lại; nếu cần khóa thêm ID khác (`partner_id` hoặc `member_id` có ID nhỏ hơn), vẫn bắt buộc nâng cấp hoặc tuân thủ Hướng 2A để tránh nghịch đảo thứ tự khóa. Do đó **Hướng 2A là giải pháp triệt để lâu dài**.

### 3.2. Minh bạch hóa Startup Log & Kiểm tra `eve.emg`

- Tại [`src/main.rs:119-126`](file:///D:/VUDT/GIT_PCC/ts_dream/src/main.rs#L119-L126): bổ sung `data.scene_eve_data.len()`, `data.quest_marks.len()`, và trạng thái `eve_events_enabled()` vào log khởi động (thay/bổ sung cho `0 talks` gây hiểu nhầm).
- Tại [`src/data/loader.rs:348-374`](file:///D:/VUDT/GIT_PCC/ts_dream/src/data/loader.rs#L348-L374): phát `tracing::warn!` (hoặc fail-fast tùy cấu hình) nếu `eve.emg` không tìm thấy trong `data_dir` hoặc `self.scene_eve_data.is_empty()`.

### 3.3. Kiểm thử bổ sung (khi triển khai fix)

- Hiện mọi test Eve trong `tests/eve_quest/` chỉ gọi thẳng `dispatcher::dispatch(...)` nên bỏ sót `server_control.rs:894`. Khi sửa, cần bổ sung integration test chạy qua TCP thật hoặc qua `ServerControl` loop (nằm trong `tests/server_network/` hoặc `tests/eve_quest/` theo `CODING_STANDARDS.md §3.1`) kiểm chứng:
  1. Gửi `C→S F444040014080100` trên map `10817` (qua kết nối TCP hoặc loop có outer lock) **không bị treo**, trả về `14 07` + `0x0C` hoặc `14 08` bình thường.
  2. Đi tới Bạch Lão Nhân (`MapObjectID = 4`, `15009`) tại `(1430, 660)`, gửi `14 01 04 00` → 6× `14 06` → thoát chuỗi qua `DomainEffect::Warp` không bị deadlock và chuyển map thành công.
  3. Kiểm chứng `sys_msg_frame` phát đúng `02 0C <visc>` (hoặc `02 00 00000000 <visc>`) không bị cắt 3 ký tự đầu và không mang tag `(Thần)Thiên thần`.
