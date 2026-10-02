# Luồng Client: Reconnect / Heartbeat / Health Check (Kết nối lại & kiểm tra tình trạng)

Ngày nghiên cứu: 2026-10-01 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Nguồn chính: `client_pseudo_c/` (C + ASM + HEX) · Đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`
Tài liệu nền: [`post_login_stats_inventory_flow.md`](./post_login_stats_inventory_flow.md),
[`login_flow_research.md`](./login_flow_research.md), [`opcode_00_01.md`](./opcode_00_01.md)
Source Rust: `src/` (dispatcher, handlers, web/server_control, protocol)

> **Phương pháp:** 2 subagent độc lập (một cho client decompile, một cho gap Rust) + đối chiếu
> byte-level trực tiếp bởi tác giả đối với mọi claim then chốt. Những claim nào hai nguồn
> **xung đột** đã được tác giả kiểm chứng lại và ghi rõ ở §3 và §7.

---

## 1. Kết Luận Quan Trọng Nhất (Đọc Trước)

| # | Kết luận | Mức bằng chứng |
|:-:|---|---|
| **1** | **Client KHÔNG có heartbeat định kỳ.** Không có timer nào phát sinh frame mạng định kỳ. Hằng `GetTickCount` duy nhất liên quan socket là ngưỡng **100 ms** dùng để giãn tốc độ gửi (pacing), không phải keepalive. | ASM đã kiểm |
| **2** | Client có **đúng một frame health-check**: opcode `0x00` với payload **1 byte `0x00`**, wire `F4 44 01 00 00` (XOR: `59 E9 AC AD AD`). Nó **không** phải ping định kỳ — nó là **ACK** do client gửi khi (a) vừa mở socket game port 6414, và (b) ngay trước khi hiện hộp thoại "mất kết nối". | ASM + hex literal, tự verify |
| **3** | **Sai lệch quan trọng với tài liệu cũ:** `opcode_00_01.md:223-225` nói frame này là header rỗng `F4 44 00 00`. **Sai.** Đúng là `F4 44 01 00 00`. | Đã tự verify hex |
| **4** | **Client không có auto-reconnect.** Toàn bộ dump chỉ có **một** call-site `TAbstractSocket.Open`, và nó được kích hoạt bởi **server** gửi `2E 01` (server-switch). Không có retry, không có timer reconnect. | grep toàn dump |
| **5** | **Server KHÔNG có idle timeout / health check nào.** Vòng `select!` chỉ có 2 nhánh (read, write). Không có `last_seen`. Socket half-open → task treo vô hạn, entry trong `clients` không bao giờ được dọn. | `server_control.rs:377-483` |
| **6** | **Chặn reconnect lớn nhất:** double-login guard (`server_control.rs:238-240`). Nếu connection cũ không EOF sạch, entry `clients` còn sót → reconnect bị từ chối với `F44402000013`. | Đã verify |
| **7** | **Server đã có sẵn `SystemAlertReason` đầy đủ 57 mã** (bao gồm `ConnectionLost=13`, `DuplicateLoginOtherLocation=19`) nhưng `send_system_alert` **chưa được gọi ở đâu cả**. Đây là cơ chế kick đúng nhưng chưa ai dùng. | grep |
| **8** | Sự kiện **reconnect thật sự** của client là: đóng socket → client về màn login → server gửi `01 09 <sceneMode 90..99>` → client **tự điền form và tự submit lại auth** (`FUN_007095c0`). Đây là **re-login mới**, không phải reconnect trong suốt. | Đã verify |

---

## 2. Framing — Baseline (nền tảng cho mọi claim)

Wire là **binary** (Delphi `AnsiString`), không phải hex-ASCII.

### 2.1 Frame layout

```
[F4 44] [len:2 LE] [payload: len bytes]     ← toàn frame XOR 0xAD khi truyền
```

- **Token = `F4 44`** (không phải `44 F4`). Bằng chứng: `token_send.hex` và `token_recv.hex`
  đều dump `F4 44` tại offset data; khớp `HEADER_TS_MAGIC = [0xF4, 0x44]` tại `src/protocol/mod.rs:11`.
- **Length là LE16**: `FUN_0077eb1c` (`0077eb1c_FUN_0077eb1c.asm.txt:10-22`) ghi byte thấp trước
  rồi `SHR EAX,0x8` ghi byte cao sau. Chiều nhận: `FUN_0077eb9c`
  (`0077eb9c_FUN_0077eb9c.asm.txt:27-54`) `EAX = byte[0] + (byte[1] << 8)`.
- **XOR 0xAD áp lên TOÀN frame** (kể cả header), phía gửi:
  `TForm1.CY_DelSedQueue.asm.txt:52-60` (`MOV CL,0xad` → `CALL 0x0050a2fc` → `SendText`).
  Phía nhận: `TForm1.ClientSocket1Read.asm.txt:26-29` (`MOV CL,0xad` → `CALL 0x0050a248`).

### 2.2 Opcode KHÔNG do framing layer thêm

`TForm1_CY_AddSedQueue` chỉ gắn `token + len + payload`. **Byte opcode đầu tiên của payload
do chính thân `case` body ghi vào.** Bằng chứng đối chiếu `case 1` (auth):

```
cs_unknown_builders_0x77f634.asm.txt:94-96   MOV byte ptr [EAX + 0x1],DL   ; DL = opcode
                                            MOV byte ptr [EAX],0x1        ; ShortString{1, chr(opcode)}
...
:149  PUSH [playerID 4B LE]
:150  PUSH [account prefix 2 chars]
:152  MOV DX,0xbc ; :154 CALL 0x0077eb1c    ; LE16(0x00BC)
:159  PUSH [raw password]
:162  CALL 0x00404148                        ; LStrCatN(&dest,5,...)
:166  CALL 0x0051633c                        ; AddSedQueue
```
→ payload = `[01][lenPw][playerID:4LE][prefix:2][BC 00][pw]` — đúng layout auth đã biết.

---

## 3. Frame Health-Check C→S: Opcode `0x00` — Sửa Sai Tài Liệu Cũ

### 3.1 Định tuyến dispatcher

`FUN_0077f414` (SendCommand) là dispatcher C→S:

```
0077f414_FUN_0077f414.asm.txt:27-31
  27: MOV AL,byte ptr [EBP + 0xfb]      ; DL = low byte của param_2
  28: CMP EAX,0xc7
  29: JA  0x0078a4f2                    ; > 0xC7 → thoát, KHÔNG gửi
  30: MOV AL,byte ptr [EAX + 0x77f474]  ; byte_map[opcode]
  31: JMP dword ptr [EAX*0x4 + 0x77f53c]
```

Từ `table_0x77f474_200B.hex:1`: `byte_map[0x00] = 0x01`.
Từ `table_0x77f53C_dword200.hex:1`: `jt[1] = 0x0077f634` → **`case 0` nằm ở `0x0077F634`**.

Body của `case 0`:

```
0077f414_FUN_0077f414.asm.txt:36-40
  36: MOV EAX,[0x007da664]
  37: MOV EAX,dword ptr [EAX]
  38: MOV EDX,0x78a834          ; ← hằng chuỗi payload
  39: CALL 0x0051633c           ; TForm1.CY_AddSedQueue(form, <0x78a834>)
  40: JMP 0x0078a4f2            ; thoát
```

### 3.2 Payload KHÔNG rỗng — nó là 1 byte `0x00`

Ghidra in `TForm1_CY_AddSedQueue(..., "")` tại `0077f414_FUN_0077f414.c:815`, nhưng
**đó là recovery artifact**. Hằng Delphi `AnsiString` có layout
`[refcount:4 @ ptr-8][length:4LE @ ptr-4][chars @ ptr][NUL]` (chính là dòng header 3 của
`cs_literals_0x78a818_0x78a8ed.hex`).

Đọc `cs_literals_0x78a818_0x78a8ed.hex` (base `0x78a818`, 16 byte/hàng):

| offset | bytes | ý nghĩa |
|---|---|---|
| `0x78a82C` | `FF FF FF FF` | refcount = −1 (hằng compile-time) |
| `0x78a830` | `01 00 00 00` | **length = 1** |
| `0x78a834` | `00` | **chars[0] = 0x00** ← payload của `case 0` |

Xác thực chéo bằng 3 hằng liền kề có ngữ nghĩa đã biết:

| Địa chỉ | ptr−4 (len) | chars | nội dung |
|---|---|---|---|
| `0x78a834` (`asm:38`) | `01 00 00 00` → **1** | `00` | **payload 1 byte `0x00`** |
| `0x78a848` (`.c:828`) | `02 00 00 00` → 2 | `20 20` | `"  "` — fallback account prefix |
| `0x78a854` (`.c:820,846`) | `01 00 00 00` → 1 | `30` | `"0"` — prefix cho `StrToInt` |
| `0x78a860` (`.c:860`) | `08 00 00 00` → 8 | `"12345678"` | chuỗi scramble mật khẩu |

Layout đúng ở cả 4 hằng ⇒ kết luận chắc chắn: **`LStrLen(0x78a834) = 1`**.

### 3.3 Byte wire chính xác

| tầng | bytes |
|---|---|
| payload | `00` — **1 byte = main opcode `0x00`, không có body** |
| length field | `01 00` (LE16 của 1) |
| **frame thuần (5 byte)** | **`F4 44 01 00 00`** |
| hex-string | `F444010000` |
| **trên wire (XOR `0xAD`)** | **`59 E9 AC AD AD`** |

Tính: `F4^AD=59`, `44^AD=E9`, `01^AD=AC`, `00^AD=AD`, `00^AD=AD`.

**⇒ Không phải frame header rỗng 4 byte.** Nó là frame 5 byte có 1 payload byte, và byte
đó chính là **main opcode `0x00`** (`OP_SYSTEM_ALERT` tại `src/protocol/mod.rs:30`).

### 3.4 Server hiện đang xử lý thế nào

| bước | vị trí | kết quả |
|---|---|---|
| Decode | `src/protocol/frame.rs:60-85` | `len = le16(hex[4..8]) = 1`; `total_chars = 8+2 = 10` → tách đúng frame `F444010000` |
| Magic check | `src/protocol/frame.rs:100-102` | `F4 44` ✓ |
| Dispatch | `src/server/dispatcher.rs:226-228` | `opcode = decoded[4] = 0x00`; `sub = None → 0`; `payload = &[]` |
| Handle | `src/server/dispatcher.rs:394` | **không có arm `0x00`** → rơi vào `_ => unimplemented::handle(ctx)` |
| `unimplemented` | `src/server/handlers/unimplemented.rs` | `is_main_opcode(0)` = true → chỉ `tracing::debug!`, **không trả lời, không shutdown** |

⇒ Server **hiện đã chịu được** frame ACK này một cách âm thầm. Điều thiếu là khả năng
dùng nó làm tín hiệu liveness (§6).

---

## 4. Ai Kích Hoạt Frame `0x00`? — Toàn Bộ Call-Site

Grep `FUN_0077f414` toàn dump cho thấy ~215 call-site, nhưng **chỉ 2 call-site truyền
`param_2 & 0xff == 0`**. Không cái nào là timer định kỳ.

### 4.1 Call-site #1 — Kết nối mới tới game server (sau khi login OK)

```
005045a0_FUN_005045a0.asm.txt:92-120
  92: CALL 0x00603f20            ; teardown / clear world
  93: MOV EAX,[0x007da664]
  95: MOV EAX,dword ptr [EAX + 0x3e8]   ; ClientSocket1
  96: MOV EDX,0x190e                   ; = 6414  ← GAME PORT
  97: CALL 0x0046c604                  ; TClientSocket.SetPort(6414)
 :109: CALL 0x0046c504                  ; TClientSocket.SetAddress(...)
 :112: MOV EAX,[0x007da664]
 :115: CALL 0x0046c6b0                  ; TAbstractSocket.Open
 :116: MOV EAX,[0x007d9d30]
 :118: XOR ECX,ECX
 :119: XOR EDX,EDX                      ; ← param_2 = 0
 :120: CALL 0x0077f414                  ; → frame F4 44 01 00 00
```

Chuỗi nhân quả đầy đủ (đây là **kết nối lại ở mức protocol**):

```
S→C MainOp 0x2E (OP_SERVER_SWITCH), SubOp 0x01
   ↓ dispatcher S→C FUN_0078a89c: byte_map[0x2E] = 0x2A (=42)
   ↓   jt[42] = 0x00795171  = FUN_00795171
   ↓   case_042_00795171_FUN_00795171.c:28-30 →  if (SubOp == 1) FUN_005045a0(...)
   ↓
FUN_005045a0: SetPort(6414) → SetAddress → Open() → send `F4 44 01 00 00`
   ↓
Server nhận `00` → đây là tín hiệu "socket vừa mở, tôi sẵn sàng nhận scene"
```

> **Đây chính là cơ chế reconnect native của protocol.** Server chủ động bảo client
> chuyển sang game port, client mở socket và báo hiệu sẵn sàng bằng `00`.

Các nhánh anh em cùng hàm: `SubOp == 2 → FUN_00504f54`, `SubOp == 3 → FUN_00505104`
(`case_042…:31-36`) — nhánh báo lỗi cùng màn hình, hiện toast 2000 ms.

### 4.2 Call-site #2 — ACK trước hộp thoại "mất kết nối"

```
case_001_0078AABE_FUN_0078aabe.c:239-241
 :239  (**(code **)(**(int **)gvar_007DA258 + 0x20))();   ; ShowMessage (modal)
 :240  FUN_0077f414(*(undefined4 *)(unaff_EBP + -4),0);   ; ← ACK: F4 44 01 00 00
 :241  FUN_0050f2f8(*(int *)gvar_007DA664,1);             ; chuyển UI state
```

Đây là handler S→C MainOp `0x00` (`byte_map[0]=1 → jt[1]=0x0078AABE`).

> **Sửa lỗi thứ 2 trong tài liệu cũ:** `opcode_00_01.md:139` ghi bước 6 là
> *"Ngắt kết nối socket client: `FUN_0050f2f8`"* — **sai**. `FUN_0050f2f8`
> (`0050f2f8_FUN_0050f2f8.c:55-110`) chỉ ghi vài field UI
> (`param_1+0x441/0x442/0x44E/0x450`, `param_1+0x430`) và gọi 1 VMT slot.
> **Không có call socket nào.** Socket teardown đến từ event
> `TForm1.ClientSocket1Disconnect` (`0x0050D030`) khi **server đóng TCP**.

⇒ **Vòng đời đúng:** Server đóng TCP → client nhận `Disconnect` → `01 00` sub hiển thị
modal → **client ACK `00`** → UI về màn login.

### 4.3 Không có call-site nào khác

Các call-site còn lại đều truyền low byte khác 0 (spot-check: `0x01,0x03,0x05,0x06,0x09,0x0A,
0x0B,0x0C,0x0D,0x0E,0x0F,0x10,0x14,0x16,0x17,0x19,0x1B,0x1C,0x1E,0x23,0x25,0x27,0x28,0x29,
0x2A,0x2B,0x2C,0x2D,0x2F,0x32,0x37,0x39,0x3A,0x3B,0x3C,0x3D,0x40,0x41,0x42,0x46,0x47,0x48`).

---

## 5. Client KHÔNG Có Heartbeat Định Kỳ

### 5.1 Send pump là event-driven, không phải heartbeat producer

```
005163e4_TForm1.CY_DelSedQueue.asm.txt:19-36,52-60,84-86
  19: MOV EAX,[0x00926e8c]              ; SedQueue
  21: CALL dword ptr [EDX + 0x14]        ; Count
  22: TEST EAX,EAX
  23: JZ  0x005164f2                    ; rỗng → return, KHÔNG gửi gì
  29: MOV EAX,[0x009264a4]
  30: MOV EAX,dword ptr [EAX + 0x10]    ; lastSendTick
  33: MOV EAX,0x64                      ; ★ ngưỡng = 100 (ms)
  34: CALL 0x007c4d7c                   ; (now - lastTick) >= 100 ?
  35: TEST AL,AL
  36: JZ  0x005164f2                    ; chưa đủ 100ms → return
  52: MOV EDX,[EBP + 0x24]              ; pop queue[0]
  53: MOV CL,0xad
  55: CALL 0x0050a2fc                   ; XOR 0xAD cả frame
  60: CALL 0x0046bd78                   ; SendText
  84: CALL 0x00407930                   ; GetTickCount
  86: MOV dword ptr [EDX + 0x10],EAX    ; lastSendTick = now
```

`FUN_007c4d7c` là hàm thuần "elapsed ≥ threshold" (`007c4d7c_FUN_007c4d7c.asm.txt:9-25`:
`GetTickCount` → `FSUB [EBP+8]` → `FCOMPP` → `JA`).

⇒ **100 ms chỉ là sàn pacing tốc độ gửi, KHÔNG phải keepalive interval.** Ở đây
**không có frame nào được phát sinh** — chỉ có frame đã xếp sẵn trong queue được gửi đi.

### 5.2 Toàn bộ `GetTickCount` trong dump — không cái nào là network timer

| Call-site | Mục đích |
|---|---|
| `00509e84_FUN_00509e84.c:97` | lưu `DAT_00923214+0x484`; `.c:110` gửi opcode `0x25`. **One-shot** world/scene init, từ `sub_0078c45f` trong `FUN_0078bc95` (S→C MainOp `0x03`). Không phải heartbeat |
| `005163e4…:77` | **sàn pacing gửi** (§5.1) |
| `0078a89c…:1763` (= `0078ca56…:64`) | S→C MainOp `0x05` SubOp `4` → `self+0x348 = GetTickCount()` (cờ mở khóa chuột) |
| `0078a89c…:6244` | S→C battle sub `0x32` → `+0x1430 = GetTickCount()` |
| `00734610…:76,78,124`, `0073439c…:38` | accumulator `+60000` ms cho **hiệu ứng ánh sáng** (`TLight_Create`) |
| `0074e638…:28`, `0074e82c…:32` | `self+0x1518 = 300000` — thời gian hiệu ứng pet/mount |
| `0073c524`, `0074def4`, `0074fe94`, `00749198`, `007508e4`, `00756590`, `00759680`, `005965e8`, `case_037` | timestamp UI/scene/effect |

**Không có magic interval 30000 / 60000 / `0x7530` / `0xEA60` nào được dùng làm network timer.**

> **Cảnh báo recovery artifact:** `TForm1.CY_DelSedQueue.c:8` ghi `Callers: <none>` rồi
> `:20` cho `005157e6 → 005163e4`. `TForm1.CY_DelRevQueue.c:20` cho `005157ee → 00516158`.
> Cả hai địa chỉ nằm trong **khoảng trống chưa decompile** `0x00514C5B … 0x00516108`.
> ⇒ **Hàm sở hữu pump là UNDETERMINED.** Con số "~30 ms tick" trong tài liệu cũ
> (`login_flow_research.md:40,108`, `opcode_00_01.md:40`) là **văn xuôi, không có hằng
> số nào trong binary backing nó** — không nên dùng làm căn cứ thiết kế.

### 5.3 "Keepalive" gần nhất trong client — nhưng không phải

`TForm1.Button33Click` gọi thẳng `CY_AddSedQueue`, bypass `FUN_0077f414`:

```
00519fe0_TForm1.Button33Click.asm.txt:20-43
  20: PUSH 0x51a090           ; literal "2\x01"
  22: MOV DX,0x2710           ; 10000
  24: CALL 0x0077eb1c        ; LE16(10000)
  27: CALL 0x00402cf0        ; Random(256)
  32: CALL 0x00402cf0        ; Random(256)
  40: CALL 0x00404148        ; LStrCatN(&dest,4,...)
  43: CALL 0x0051633c        ; AddSedQueue
```

Payload = `32 01 10 27 RR RR` (opcode `0x32`, sub `0x01`, LE16 10000, 2 byte random).
Đây là **handler nút debug/cheat ẩn** (`Callers: <none>`, chỉ bind qua DFM).
`.scratch/.../opcode_1a.md:21` gọi nó là *"khung keepalive"* — **không có cơ sở**;
payload random không hợp với keepalive cố định.

### 5.4 Client không có idle timeout, không tự abort

- **Chỉ 1 call-site** `TAbstractSocket.Open` (`0x0046c6b0`) trong toàn dump →
  `005045a0…asm.txt:115`. (grep `CALL 0x0046c6b0` = 1 hit.)
- **Không có** `TAbstractSocket.Close/Abort` nào cho game socket.
- Kết nối login khác đi qua `TAbstractSocket.SetActive(socket,1)` trong
  `TForm1.FormCreate` (`0050a4a0_…c:695-713`): `SetPort(0x1910 = 6416)`, đọc `Setup.ini`.
- ⇒ **Nếu server im lặng, client không làm gì cả** — vẫn render, vẫn di chuyển.
  **Server buộc phải tự đóng TCP** mới kích hoạt được `ClientSocket1Disconnect`.
- Không có handler `ClientSocket1Error` trong dump ⇒ hành vi khi connection-refused/reset
  là **UNDETERMINED**.

### 5.5 Không có S→C ping để client đáp

- **MainOp `0x0A`** map `byte_map[0x0A] = 0x00` → `jt[0] = 0x00796347` =
  `case_000_00796347_caseD_0.c:8-46`, là **pure epilogue / no-op** (chỉ `LStrArrayClr`/`LStrClr`).
  ⇒ **Client hoàn toàn BỎ QUA S→C `0x0A`.** Server không thể dùng nó làm probe.
- **MainOp `0x38`** → `case_064_00796248`; `opcode_38.md:18,220,244` tự đánh dấu body rỗng
  và mục đích là **suy đoán** ("placeholder / heartbeat / ack / padding").
- Server (`src/web/server_control.rs`) không có task định kỳ nào per-connection; các task
  định kỳ chỉ là WAL checkpoint (`db/pool.rs:115`), AutoSave (`auto_save.rs:199`),
  và NPC wander 900 ms (`battle/service.rs:1396-1415`).

---

## 6. Vòng Đời Reconnect Thật Sự Của Client

### 6.1 Không có auto-reconnect theo timer

Grep `CALL 0x0046c6b0` (socket Open) = **1 hit**, kích hoạt bởi S→C `2E 01` (§4.1).
⇒ **Không có re-Open tự động ở bất kỳ đâu.** Socket rớt → client về UI login và chờ người chơi.

### 6.2 Auto-relogin CÓ, nhưng do server điều khiển

`FUN_007095c0` (submit login) có **đúng 2** caller:

| Caller | Kích hoạt |
|---|---|
| `00708f38_FUN_00708f38.c:91` — `piVar2[0x14] = (int)FUN_007095c0;` | nút login `OnClick` (thao tác người) |
| `0051aae0_FUN_0051aae0.c:71` — `FUN_007095c0(*(int **)gvar_007D9E7C);` | **S→C MainOp `0x01`, SubOp `0x09`** |

```c
// 0051aae0_FUN_0051aae0.c:50-72
DAT_00926fc6 = *(byte *)(param_2 + 1);              // sceneMode = RestPayload[1]
(**(code **)(**(int **)gvar_007D9CC4 + 0x24))();     // ẩn form chọn server
bVar4 = FUN_00504c9c(gvar_007DA6EC, DAT_00926fc6);   // sceneMode % 100 ∈ [90..99]?
if (!bVar4) { /* hiện form login */ }
else {
  FUN_007b372c(loginForm + 0x13c, DAT_0092530c + 0x644);   // tài khoản đã lưu
  _LStrFromString(&local_10, DAT_0092530c + 0x648);        // mật khẩu đã lưu
  FUN_007b372c(loginForm + 0x140, local_10);
  FUN_007095c0(*(int **)gvar_007D9E7C);                    // ★ TỰ SUBMIT
}
```
→ `FUN_007095c0_…c:142-153` → `FUN_0077f414(gvar_007D9D30, 1)` (gói auth).

⇒ **Con đường auto-reauth duy nhất:** server gửi `01 09 <sceneMode>` với
`sceneMode % 100 ∈ [90..99]` **và** client đã lưu account/password ⇒ client tự điền form
và tự gửi lại auth. Đây là **re-login mới trên socket mới**, không phải reconnect trong suốt.

### 6.3 Sơ đồ đầy đủ

```
┌──────────────────────────────────────────────────────────────────────┐
│ Nhánh A — KẾT NÓI LẠI DO SERVER CHỈ ĐỘNG (native reconnect)          │
├──────────────────────────────────────────────────────────────────────┤
│ Server ──► 2E 01 <c1> <c2>        (server-switch; c1∈{1,2} c2∈{2,3}   │
│                                        hoặc c1=0 c2=7)               │
│ Client: FUN_005045a0 → SetPort(6414) → SetAddress → Open             │
│ Client ──► 00                    = F4 44 01 00 00  ← "socket đã mở"  │
│ Server: (hiện chưa có arm 0x00 → no-op)                            │
│ Client ──► 01 [auth] → 03 01 → Logined1 → 25 01                     │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│ Nhánh B — MẤT KẾT NỐI RỒI TỰ LOGIN LẠI                              │
├──────────────────────────────────────────────────────────────────────┤
│ (rớt mạng / server đóng socket)                                     │
│ Client: ClientSocket1Disconnect → về UI login (0x0050D030)           │
│   — hoặc —                                                          │
│ Server ──► 00 <sub>              (system-alert, modal "mất kết nối") │
│ Client: ShowMessage → ACK ──► 00  → UI về login                      │
│ Server (lần sau): ──► 01 09 <sceneMode 90..99>                      │
│ Client: tự điền form + tự submit ──► 01 [auth]                      │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│ Nhánh C — IDLE / HEALTH CHECK                                        │
├──────────────────────────────────────────────────────────────────────┤
│ Client KHÔNG gửi gì cả. Không có timer.                            │
│ Server phải tự phát hiện (idle timeout) → gửi 00 <sub> → đóng TCP  │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 7. Server Rust: Khoảng Trống Cho Reconnect & Health Check

Mọi claim dưới đây đã được tác giả đối chiếu trực tiếp `src/`.

### 7.1 Vòng đời connection hiện tại

| Bước | Vị trí | Chi tiết |
|---|---|---|
| Bind | `src/web/server_control.rs:103-111` | `TcpListener::bind(0.0.0.0:game_port)` |
| Accept | `:129-154` | `select!` trên `listener.accept()` vs `shutdown_rx.recv()`. **Không `set_nodelay`, không `set_keepalive`** (grep `nodelay\|keepalive` trong `src/` = **0 match**) |
| Spawn | `:139-141` | `tokio::spawn(handle_client_connection(...))`, task tách rời |
| Handler | `:344-372` | log, `split()` (`:363`), `mpsc` **không giới hạn** (`:364`), `Conn::with_peer_ip` (`:366`), buffer 8 KiB (`:367`), `logined_id = 0` (`:368`) |
| Greeting | `:372` | `LOGIN_SCENE_GREETING` = `F444030001095A` (`src/server/spawn.rs:60`) |
| Read loop | `:377-483` | `select!` **chỉ 2 nhánh**: socket read, outbound channel. **Không có nhánh timer** |

Grep `timeout\|idle\|Instant\|last_seen\|heartbeat\|Duration::from_secs` trong `src/**/*.rs`:
- `server_control.rs:172` — `sleep(1s)` — **chỉ đếm ngược shutdown**
- `battle/manager.rs:216-243`, `battle/service.rs:588,628` — deadline trận đấu
- `db/pool.rs:49` — `busy_timeout` SQLite
- `server/auto_save.rs:20` — `AUTO_SAVE_INTERVAL = 180s`

**Không có field `last_seen`/`last_active`/`connected_at` ở bất kỳ đâu** (grep = 0 match).
`Session` (`src/server/session.rs:131-273`) không mang timestamp nào ngoài
`last_battle_end_ms` (`:268`) và `last_chat_ms` (`:272`) — đều là cooldown gameplay.
`Conn` (`src/server/session.rs:508-512`) chỉ có `{ decoder, session, peer_ip }`.

### 7.2 Teardown — đường duy nhất

| Sự kiện | Vị trí | Hành vi |
|---|---|---|
| EOF sạch `Ok(0)` | `:381` | `close = true` |
| Lỗi read `Err(_)` | `:472` | `close = true` — **`Err(_) => close = true`, KHÔNG có dòng log nào** |
| Lỗi write | `:477-479` | `close = true` |
| `out.shutdown` từ handler | `:462-469` | push log + `close = true` |

Teardown (`:485-493`) → `disconnect_player` (`:323-338`): broadcast `session_offline_frame`
(`spawn.rs:319-321`), `unregister_client` (`:326`), xoá snapshot (`:327`),
persist qua `persist_sessions_transaction` với `["stats","homdo","trangbi","quest","pet"]` (`:330-336`).

> ⚠️ **Không có gì gửi `0x00` system-alert.** Và **không bị giới hạn thời gian**.
> Một TCP half-open (rút cáp, NAT timeout, kill client) → socket nằm vô hạn trong `read()`,
> task không bao giờ yield, entry `clients` không bao giờ được dọn, autosave tiếp tục
> "fingerprint" một bóng ma.

### 7.3 Reconnect xảy ra thế nào hôm nay

**Có một phần đúng:** TCP mới ⇒ `Conn` mới (`:366`) ⇒ `Session::default()` nên
`logined=false` (`session.rs:294`), `authed=false` (`:296`), `in_world=false` (`:295`),
`logined_id=0` (`:368`). Cờ per-connection reset đúng. ✅

**`login_db` (`src/server/handlers/login.rs:169-235`) — đã verify:**

| Bước | Dòng | Ghi chú |
|---|---|---|
| `verify_pass1` | `:180` | sai → `LOGIN_WRONG_PASS`, **không shutdown** — socket vẫn mở, chưa auth |
| suspended check | `:184-195` | `sys_msg_frame` + `shutdown = true` |
| `touch_login` | `:198` | ghi last-login + IP |
| `authed = true` | `:202` | |
| **`repos.sessions().load()`** | `:207` | ✅ reload nhân vật, tiền, 5 túi, pet, skill, hotkey, quest/Eve |
| **double-login guard** | `:215-221` | ❌ xem §7.4 |
| `logined = true` | `:223` | |
| `build_logined_sequence_session` | `:224-228` | ✅ **chuỗi Logined1 22 bước được phát lại** |

### 7.4 Danh sách khoảng trống

| # | Mức | Vấn đề | Trích dẫn |
|:-:|:-:|---|---|
| **G1** | **CAO** | **Không idle timeout / read deadline.** `select!` không có nhánh timer. Half-open socket treo vô hạn, `clients` + `online_sessions` leak, `disconnect_player` không bao giờ chạy. | `server_control.rs:377-483` |
| **G2** | **CAO** | **Double-login guard chặn reconnect hợp lệ.** Entry `clients` tồi tàn từ zombie connection ⇒ `DOUBLE_LOGIN` = `F44402000013` + đóng socket. Entry chỉ bị xoá bởi `unregister_client` (`:78-81`), chỉ gọi từ `disconnect_player` (`:326`) hoặc `stop()` (`:182`). | `server_control.rs:238-240`; `login.rs:216-219`; `spawn.rs:56` |
| **G3** | **CAO** | **Không có arm `0x00` trong dispatcher.** Rơi vào `unimplemented`. Không thể dùng ACK làm tín hiệu liveness vì chưa có `last_seen`. | `dispatcher.rs:252-397` (thiếu arm `0x00`), wildcard `:394` |
| **G4** | **CAO** | **`recompute_stats()` không bao giờ được gọi sau load** ⇒ `int2..agi2 = 0` ⇒ frame `05 03` báo gear bonus = 0 cho nhân vật đã có trang bị. Đã verify: grep `recompute_stats` trong `login.rs` và `sqlite/session.rs` = **0 match**. | `login.rs:207`; `sqlite/session.rs:19-114`; `session.rs:398-422`; `spawn.rs:519-524` |
| **G5** | **CAO** | **`load_items` chỉ đọc `slot,itemid,quantity,damage`** (`sqlite/session.rs:176-199`). `lv`, `loai`, `thuoctinh`, `giatri_thuoctinh`, mọi `_1`/`_2` đều = 0 → hỏng equip gate + gear recompute sau relog. | `sqlite/session.rs:176-199`; `session.rs:9-43` |
| **G6** | **CAO** | **Không `0x00` system-alert khi kick.** `HandleOutcome::send_system_alert` tồn tại (`dispatcher.rs:187-189`) nhưng **không nơi nào gọi** (grep `src/server/handlers/` + `src/server/spawn.rs` = 0 match). Kick im lặng ⇒ client treo không hiểu lý do. | `gm.rs:407` chỉ gửi `sys_msg_frame` (op `0x02`) |
| **G7** | **TRUNG BÌNH** | `OP_SERVER_STATUS (0x36)` → `unimplemented`. Client gửi `[0x36]` khi mở form chọn server. | `dispatcher.rs:377` |
| **G8** | **TRUNG BÌNH** | `OP_SERVER_SWITCH (0x2E)` → `unimplemented`. **Lưu ý: `0x2E` là S→C-only**, client không gửi. "Implement" = **phát ra**, không phải xử lý inbound. | `dispatcher.rs:376`; `protocol/mod.rs:182` |
| **G9** | **TRUNG BÌNH** | Không có `last_seen`/`connected_at`. `Conn` chỉ 3 field. | `session.rs:508-512` |
| **G10** | **TRUNG BÌNH** | Không có surface cấu hình timeout. `Config` 8 field, không field nào là duration; không `TS_*` override cho timeout. | `config.rs:11-24` |
| **G11** | **TRUNG BÌNH** | Lỗi read bị nuốt không log — không phân biệt được "chết do socket error" với "close sạch". | `server_control.rs:472` |
| **G12** | **TRUNG BÌNH** | Không TCP keepalive/nodelay; `mpsc` không giới hạn ⇒ reader bị stall làm channel phình vô hạn. | `server_control.rs:33,62,364` |
| **G13** | **TRUNG BÌNH** | `battle_service.register_sender` được seed từ snapshot **cũ** ở `login_register` (`:242-248`), trước khi session mới load được ghi lại ở `:425-428`. | `server_control.rs:242-253` vs `:425-428` |
| **G14** | **TRUNG BÌNH** | Logined1 không push `08 01` (gap đã biết từ tài liệu nền). | `spawn.rs:471-604` |
| **G15** | **TRUNG BÌNH** | `stop()` gọi `clients.clear()` (`:181-183`) chỉ drop sender, **không đóng socket, không chạy `disconnect_player`**. | `server_control.rs:181-183` |
| **G16** | THẤP | `handle_login` reset `authed` (`login.rs:48`) nhưng **không reset `logined`**. Gửi `0x01` hai lần trên một socket ⇒ `handle_enter_game:99-101` return 0 frame ⇒ **client treo**. | `login.rs:48,99-101` |
| **G17** | THẤP | `in_world` không reset khi re-login. | `login.rs:245` |
| **G18** | THẤP | Field session không persist mất mỗi lần reconnect: `color`, `gocnhin`, `dongtac`, `tham_chien`, `tiengtam`, `god`, `hp_store`, `sp_store`, `savemap`, `quest_steps`, `warp_steps`, `completed_quests`, `click_npc_id`, `id_mem/id_leader/id_qs`, `shop`, `trade`, `horse_pet_id`. | `session.rs:206,216-218,244-261` |
| **G19** | THẤP | `characters.fai` được `save()` ghi nhưng `load()` không đọc; `Session` không có field `fai`. | `sqlite/session.rs:210,271,285,500`; `session.rs:95` |
| **G20** | THẤP | `SaveLedger::forget()` (`auto_save.rs:193-195`) không có caller ⇒ map phình vô hạn. | `auto_save.rs:193-195` |
| **G21** | THẤP | `/api/online` đọc `AppState.online` — vốn **không nơi nào populate** (chỉ `.clear()` ở `:199`) ⇒ luôn trả `[]`. | `web/app.rs:227-229,445-447,167` |

### 7.5 Coverage test

21 file test trong `tests/`. **Không test nào chạm connection loop, `ServerControl`,
`login_register`, hay bất kỳ đường health/idle nào.**

| Đường | Coverage |
|---|---|
| `handle_client_connection` | **Không.** Hàm `async` private trong module, không `pub`. `src/harness.rs` là packet-capture proxy, không phải client simulator. |
| `ServerControl` | Chỉ `tests/movement_warp_test.rs:445` — dựng 1 cái rồi gọi `disconnect_player` (`:463`). **Không test `login_register`/double-login.** |
| double-login | `tests/create_char_atomic_test.rs:250-260` chỉ assert **chuỗi hằng** `DOUBLE_LOGIN == "F44402000013"`. Không kích hoạt guard. |
| `handle_login` | `tests/login_char_flow_test.rs:86-245` dùng `ServerEnv::none()` (`:91`) — **không pool, không hub, không guard**. Chỉ nhánh golden/in-memory. |
| `system_alert` | `tests/system_alert_test.rs` chỉ test enum thuần (opcode, `from_u8`, `to_hex`). **Không round-trip dispatcher.** |
| `0x2E`/`0x36` | **Không.** |
| `recompute_stats` sau load | **Không.** |

---

## 8. Đề Xuất Hiện Thực

### 8.1 (a) Idle timeout / health check phía server

**Config** — `src/config.rs:11-24` + `Default` `:26-39`:
- `idle_timeout_secs: u64` (mặc định `300`)
- wire `TS_IDLE_TIMEOUT_SECS` qua khối `env_override` sẵn có (`:91-116`)

**State** — `src/server/session.rs`: thêm `pub last_seen: std::time::Instant` vào
`struct Conn` (`:508-512`), **không** thêm vào `Session` — vì `Session` bị clone vào registry
toàn cục **mỗi frame** (`server_control.rs:426`); `Instant` ở đó vô nghĩa và phá vỡ
hợp đồng `fingerprint`. Khởi tạo ở `Conn::new` (`:564-570`) và `with_peer_ip` (`:572-578`).

**Vòng timeout** — `src/web/server_control.rs:378-482`: thêm **nhánh thứ ba** vào `select!`.
Vì các arm của `select!` được tạo lại mỗi vòng, dạng đúng là giữ `tokio::time::Sleep` pinned
ngoài vòng lặp, dùng `&mut sleep` trong arm, và gọi `sleep.as_mut().reset(Instant::now() + idle)`
trong nhánh `Ok(n)` (`:382`) và nhánh ACK.

Khi fire:
1. `out.send_system_alert(SystemAlertReason::ConnectionLost)` — sub `13` ("Đã mất kết nối.")
   là khớp ngữ nghĩa nhất. Builder: `system_alert.rs:269-271`; helper: `dispatcher.rs:187-189`.
2. **Phải yield để frame thực sự tới socket** — `rx.recv()` cần chạy thêm một lần trước khi
   `close = true`. Cùng lớp bug như `out.shutdown` ở `:462-469` hiện break **trước** khi drain.
3. `push_log("warn", ...)` — đồng thời vá G11.
4. `close = true` → rơi vào teardown sẵn có `:485-490` (đã chạy `disconnect_player`: broadcast
   hide, unregister, persist).

**Hardening cùng file:** sau `listener.accept()` (`:132-142`), thêm
`stream.set_nodelay(true)` và `stream.set_keepalive(Some(Duration::from_secs(60)))`.

### 8.2 (b) Đường reconnect sạch

Ba sửa đổi phối hợp:

1. **Dọn entry cũ trước khi guard.** Trong `login_db` (`login.rs:207-221`), `load()` ở `:207`
   đã thành công. Nếu `hub.login_register(...)` trả `false` (`:216`), thay vì từ chối vô điều kiện,
   kiểm tra incumbent còn sống không: thêm `ServerControl::is_client_live(player_id) -> bool`
   (cạnh `register_client:73`) dựa trên `clients.contains_key` **và** idle timestamp; rồi
   (i) `disconnect_player(old)` + thử `login_register` lại, hoặc (ii) giữ `DOUBLE_LOGIN`.
   **(i) là thứ thực sự gỡ tắc người chơi hôm nay**, và an toàn vì `disconnect_player` đã
   broadcast hide + persist.

2. **`recompute_stats()` ngay sau load.** Thêm `conn.session.recompute_stats();` tại `login.rs:208`
   (sau `load`, trước khi build Logined1 ở `:224`) — vá G4 bằng **một dòng**. Tốt hơn: đặt ở cuối
   `SqliteSessionRepository::load` (`sqlite/session.rs:112`, ngay trước `Ok(true)` ở `:113`) để
   **mọi** caller đều hưởng — đó là điểm hydrate duy nhất.

3. **Khôi phục field template item.** `SqliteSessionRepository::load_items`
   (`sqlite/session.rs:176-199`) phải merge template `Data.Item` vào mỗi row.
   `InventoryItem::from_template` (`session.rs:50-74`) đã tồn tại và copy `lv`, `loai`,
   `thuoctinh`, `giatri_thuoctinh` và mọi `_1`/`_2`. Hai lựa chọn:
   - truyền `&GameData` (hoặc index template `&HashMap<i64, Item>`) vào `SqliteSessionRepository`
     (`sqlite/session.rs:12-14`), dựng ở `handle_client_connection` (`server_control.rs:357-361`)
     nơi `data` đã có sẵn; **hoặc**
   - post-process trong `login_db` sau `load`: duyệt `conn.session.homdo/trangbi/tientrang/tuideo/luulang`
     và điền field thiếu từ `ctx.data.items`. Cách này giữ được bất biến "repository chỉ làm SQL"
     (`login.rs:7`) và diff nhỏ hơn.

   `save()` (`sqlite/session.rs:299-314`) chỉ persist `slot/itemid/quantity/damage` ⇒ các field
   template **suy ra được**, không cần migration. `long_val`/`giatri_long`/`khang`/`texp` là
   per-instance thực sự và **đang mất**; `khang`/`texp` chưa có cột ⇒ LOW, chấp nhận mất
   hoặc thêm cột.

4. **Reset `logined`/`in_world` khi re-login.** Tại `login.rs:43-48`, cạnh `authed = false`,
   thêm `conn.session.logined = false; conn.session.in_world = false;` (vá G16/G17).

### 8.3 (c) Xử lý ACK `0x00` của client

Client ACK là frame `[F4 44][Len: 00 00]`… **không** — đã chứng minh ở §3.3 là
`[F4 44][Len: 01 00][00]`, tức `F444010000`, payload rỗng, sub = 0.

Decode path: `Decoder::feed` (`frame.rs:60-85`) đọc `len = 1` ⇒ `total_chars = 10` ⇒ tách đúng
frame. `check_magic` (`:100-102`) pass. `dispatcher::dispatch:226` đọc `decoded.get(4)` = `0x00`
⇒ rơi vào wildcard.

Sửa đổi là **thêm một arm** trong `src/server/dispatcher.rs::handle` (`:252-397`), **trên**
wildcard `:394`:

```rust
OP_SYSTEM_ALERT => system::handle_system_alert(ctx),
```

Hàm mới `pub fn handle_system_alert(ctx: &mut OpcodeCtx<'_>)` trong
`src/server/handlers/system.rs` (file này đã sở hữu `0x0C`/`0x21`/`0x22`/`0x23`/`0x41`/`0x42`):
- coi mọi sub là **no-op** (client chỉ gửi ACK trần; `SystemAlertReason::from_u8`
  tại `system_alert.rs:205-266` đã map unknown → `UnknownReason`);
- hành vi **có ích** là **liveness**: `ctx.conn.last_seen = Instant::now();` — `OpcodeCtx`
  (`dispatcher.rs:194-206`) đã giữ `&mut Conn`, nên **không cần đổi signature**;
- **ngoài ra** coi mọi frame đọc được là liveness ngầm ở `server_control.rs:382`
  (`Ok(n) => { conn.last_seen = … }`), để heartbeat hoạt động ngay cả trước khi có arm handler;
- log ở mức `trace`, không phải `warn`, để không spam.

**Không cần trả lời.** Client không có ACK-of-ACK, và không có mã "keepalive-ok" nào trong
`SystemAlertReason`.

### 8.4 (d) `0x2E` sub-1 — server switch (đường reconnect native)

**Đính chính framing của yêu cầu:** `0x2E` là **S→C-only**. Client không gửi nó. Nên
"implement `0x2E` sub-1" nghĩa là **phát ra** frame, không phải xử lý inbound.

- Thêm builder cạnh `DOUBLE_LOGIN` (`src/server/spawn.rs:44-60`) và `announce_frame` (`:299-302`):
  ```rust
  pub fn server_switch_frame(c1: u8, c2: u8) -> String {
      crate::protocol::frame("2E01", &format!("{c1:02X}{c2:02X}"))
  }
  ```
  → `F444 04 00 2E 01 <c1> <c2>`
- Cặp hợp lệ: `c1∈{1,2} ∧ c2∈{2,3}`, hoặc `c1=0 ∧ c2=7` (`005045a0_FUN_005045a0.c:55-77`).
- Trigger: lệnh GM/ops, cạnh `/kick` trong `src/server/gm.rs:390-419`. Dạng:
  `hub.send_to(target, &spawn::server_switch_frame(0, 7)).await;` rồi
  `hub.disconnect_player(target).await;`. Client đọc `Server.ini`, mở lại `TClientSocket`
  trên port **6414** (`0x190E`), rồi chạy lại `0x01` → `0x03` trên connection mới.
- Giữ nguyên `dispatcher.rs:376` (`OP_SERVER_SWITCH => unimplemented`) — nó ghi đúng
  rằng inbound `0x2E` không được mong đợi.
- **`0x36` mới là hướng inbound đáng implement** (G7): client gửi `[0x36]` trần khi mở form
  chọn server. Trả lời `[36][idx1][lv1]...`, `idx` 1-based, `lv ∈ 0..3`,
  **tổng payload phải chẵn** (độ dài lẻ gây `ERangeError` phía client).

### 8.5 Thứ tự ưu tiên đề xuất

| # | Việc | File | Lý do |
|:-:|---|---|---|
| 1 | Thêm arm `0x00` + `last_seen` trên `Conn` | `dispatcher.rs`, `session.rs`, `system.rs` | Nền cho mọi health check; rẻ, đúng đắn |
| 2 | Idle timeout + kick `00 13` + đóng TCP | `server_control.rs`, `config.rs` | Vá G1 — chặn leak `clients`/ghost session |
| 3 | Dọn entry cũ trước double-login guard | `server_control.rs`, `login.rs` | Vá G2 — **gỡ tắc reconnect thật** |
| 4 | `recompute_stats()` sau load | `sqlite/session.rs:112` hoặc `login.rs:208` | Vá G4 — 1 dòng, sửa `05 03` sai sau relog |
| 5 | Merge item template trong `load_items` | `sqlite/session.rs` hoặc `login.rs` | Vá G5 — phục hồi equip gate + gear bonus |
| 6 | Reset `logined`/`in_world` khi re-login | `login.rs:43-48` | Vá G16/G17 — chống treo |
| 7 | `send_system_alert` cho mọi đường kick | `login.rs`, `gm.rs`, `character.rs` | Vá G6 — client hiện thông báo lý do |
| 8 | `set_nodelay` + `set_keepalive` + log lỗi read | `server_control.rs` | Vá G11/G12 |
| 9 | Builder `server_switch_frame` + lệnh GM | `spawn.rs`, `gm.rs` | Đường reconnect native (§8.4) |
| 10 | `0x36` server-list response | `spawn.rs`, `dispatcher.rs` | G7 |

---

## 9. Sửa Lỗi Tài Liệu Đề Xuất

1. **`opcode_00_01.md:223-225`** — nói frame ACK là header rỗng `[TOKEN 2B][Length 00 00]`
   (tức `F4 44 00 00`). **Sai.** Đúng là `F4 44 01 00 00`: payload 1 byte `0x00`, và byte đó
   **chính là main opcode `0x00`**. Nguồn: `0077f414_FUN_0077f414.asm.txt:38` +
   `cs_literals_0x78a818_0x78a8ed.hex` (`0x78a830` = length 1).
2. **`opcode_00_01.md:139`** — nói `FUN_0050f2f8` là "Ngắt kết nối socket client".
   **Sai.** `0050f2f8_FUN_0050f2f8.c:55-110` không có call socket nào; nó chỉ chuyển UI state.
   Socket teardown đến từ event `ClientSocket1Disconnect` khi **server đóng TCP**.
3. **`opcode_1a.md:21`** — gọi payload `32 01 10 27 RR RR` của `TForm1.Button33Click` là
   "khung keepalive". **Không có cơ sở**: đó là handler nút debug ẩn với 2 byte random.
4. **Token framing:** một số tài liệu viết token là `44 F4`. Đúng là **`F4 44`**
   (hex-string `F444`).
5. **Con số "~30 ms tick"** (`login_flow_research.md:40,108`, `opcode_00_01.md:40`) là văn xuôi
   suy đoán; hằng số duy nhất tìm được là **100 ms pacing** tại
   `005163e4_TForm1.CY_DelSedQueue.asm.txt:33`. Hàm sở hữu pump còn nằm trong vùng chưa
   decompile `0x00514C5B…0x00516108` ⇒ **UNDETERMINED**.

---

## 10. UNDETERMINED

1. **Hàm/timer sở hữu send+recv pump.** Call-site `0x005157e6` (→ `CY_DelSedQueue`) và
   `0x005157ee` (→ `CY_DelRevQueue`) nằm trong khoảng trống chưa decompile.
   Interval thật của pump là UNDETERMINED.
2. **Kích hoạt cụ thể của `FUN_00509e84`** (`GetTickCount` ở `:97` + gửi `0x25` ở `:110`).
   Chắc chắn **one-shot** (re-init toàn world) nên không phải heartbeat, nhưng điều kiện
   sub-opcode không recover được.
3. **Ý nghĩa server-side của `0x2E 01`** ngoài "chuyển sang port 6414".
4. **`ClientSocket1Error`** — không tồn tại handler trong dump ⇒ hành vi khi
   connection-refused/reset là UNDETERMINED.
5. **Wire layout `0x2E 01` chi tiết hơn** điều kiện `b[2]∈{1,2} ∧ b[3]∈{2,3}` hoặc
   `b[2]==0 ∧ b[3]==7` (`005045a0_FUN_005045a0.c:55-77`); chuỗi IP được build bởi
   `FUN_00504ce4` (chưa dump).
6. **Hai literal `0x796d38` (default) và `0x796d10` (sub `0x38`)** chưa có dump
   (`opcode_00_01.md:6,130`) ⇒ nghĩa chưa xác nhận.
7. **Client có phân biệt S→C `0x00` sub `0` không.** Ghidra render `default:` → `0x796d38`
   (`case_001_…c:30-33`), nhưng bảng VISCII cho thấy block *"Mất kết nối với Server"* tại
   `0x0078AF97` cho sub `0` và `39` (`opcode_00_01.md:253-254,293-294`) ⇒ một `case 0:` đã bị
   Ghidra gộp vào `default:`. **Không dựa vào nhãn `case` của Ghidra ở handler này.**
8. **Server có cần trả lời ACK `0x00` không.** Không có bằng chứng trong bất kỳ binary nào về
   reply S→C cho MainOp `0x00` từ phía client. Khuyến nghị: **im lặng** (hành vi Rust hiện tại
   đã đúng).

---

*Tài liệu nghiên cứu bởi 2 subagent + đối chiếu byte-level trực tiếp của tác giả (2026-10-01).
Mọi claim "đã xác minh" đều có `file:line` và trích dẫn asm/hex cụ thể; những điểm hai nguồn
xung đột đã được tác giả tự kiểm chứng và ghi rõ tại §3 và §9. Nguồn chính `client_pseudo_c/`,
`test/ts_decompile/` chỉ dùng để đối chiếu.*
