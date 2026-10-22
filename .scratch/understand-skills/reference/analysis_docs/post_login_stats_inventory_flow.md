# Nghiên Cứu Luồng Sau Đăng Nhập: Xác Nhận Chỉ Số (Stats) & Túi Đồ (Hòm Đồ)

Ngày: 2026-09-23; cập nhật xác minh: 2026-09-24; **đính chính `1E 01`/`17 1E`: 2026-10-01**
Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Tài liệu tham chiếu:
- Client decompile chính: `client_pseudo_c/` (C + ASM + HEX, đã bổ sung sau đợt dump Round-3)
- Client decompile đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/` (C + ASM)
- Tài liệu nền: [`login_flow_research.md`](./login_flow_research.md) (kết nối & gửi gói auth)
- Các tài liệu opcode hiện có: `opcode_03.md`, `opcode_05.md`, `opcode_08.md`, `opcode_09.md`, `opcode_17.md`, `opcode_1a.md`
- Source Rust: `src/` trong repo này
- Tham chiếu server C#: `TS_Server_Bear/TS_Server/`

> **Phạm vi bằng chứng mới:** `client_pseudo_c/` đã có body/ASM cho parser `17 05`, `17 0B`, parser `1E 01` (`FUN_007707a0`), stall `17 1E` (`FUN_00730F4C`), các dispatcher C→S `02/17/19/1C`, literal `0x78A8xx`, hàm `FUN_006547a8`, và ASM `09 01` empty-payload. Những claim cũ trong §6 về việc các artifact này chưa tồn tại đã được sửa.
>
> **🛠 Đính chính 2026-10-01 (sau nghiên cứu battle):** bản gốc gán `1E 01` = stall/vendor
> dựa trên `FUN_00730F4C`. **Sai** — `FUN_00730F4C` được gọi từ main `0x17` (tức `17 1E`).
> `1E 01` thật là container 51 ô, record 12 byte, **khớp** `dump_table()` của Rust.
> Chi tiết: §3.3.1.

---

## 1. Tổng Quan Luồng (Big Picture)

**Kết luận cập nhật:** Trong các builder/handler đã dump, không tìm thấy opcode C→S chuyên để
“xin” toàn bộ chỉ số hoặc xin lại toàn bộ túi đồ. `05 03` và `17 05` là các frame push từ
Server → Client trong `Logined1`; `08 01` là đường phân bổ chỉ số, không phải query.
Tuy nhiên, các claim cũ còn dùng pseudo-C `case ...: break` làm bằng chứng phải bị loại bỏ:
ASM hiện có đã chứng minh có các builder C→S `02/17/19/1C`, đồng thời xác nhận một nhánh gửi
`09 01` với payload rỗng. Full create payload `09 01` mà server parse trong `character.rs`
vẫn là contract phía server; chưa có builder client độc lập tương ứng trong dump hiện tại.

```
┌─────────────┐   01 .. (auth)      ┌─────────────┐
│    Client    │ ──────────────────► │    Server    │
│  (aLogin)    │                     │  (ts_dream)  │
└──────┬──────┘                     └──────┬───────┘
       │                                   │
       │  ◄── 01 09 sceneMode (mở màn login)      [login_flow_research.md]
       │  ──► 01 [auth payload]                   [đã研究 trong login_flow_research]
       │  ◄── danh sách nhân vật / kết quả login
       │                                   │
       │  ──► 09 02 + name      (kiểm tra tên)     character.rs:101
       │  ◄── 09 03 ss          (ss=1 trùng, ss=2 sai)
       │  ──► 09 01 + payload   (TẠO NHÂN VẬT; server contract) character.rs:39-143
       │  ◄── 09 03 00          (tạo OK)
       │                                   │
       │  ──► 03 01             (XÁC NHẬN VÀO GAME) login.rs:88
       │                                   │
       │  ◄── ═══ CHUỖI LOGINED1 (21–22 frame) ═══  spawn.rs:471-604
       │      • 14 08 / 14 21 00   (end-talk marker)
       │      • 03                 (id + name + map/xy + equip)   ← ID & NAME
       │      • 05 03              (hp/sp/int/atk/def/agi/lv/exp) ← STATS
       │      • 0F xx pet, 21 02 PK
       │      • 17 05              (HÒM ĐỒ)                       ← INVENTORY
       │      • 1E 01 (container 51 ô), 17 2F, 17 66, 17 0B, 1A 04, 27 09 ...
       │      • 28 01 hotkeys, 23 04 store, 18 06/07 quest sync
       │                                   │
       │  ──► 25 01             (map loaded — xong login)  login.rs:238
       │  ◄── broadcast appear + sync player/NPC khác
       │                                   │
       │  ──► 08 01 (chỉ khi PHÂN BỔ chỉ số)             stats.rs:28
       │  ◄── 08 01 (server push khi chỉ số đổi)
       │  ──► 17 xx ( thao tác túi đồ: sắp xếp, thả, dùng ) inventory.rs
       │      — KHÔNG có bằng chứng cho sub "xin lại toàn bộ 17 05"
```

> **Phân biệt hai path `09 01`:** ASM `client_pseudo_c/0077f414_FUN_0077f414.asm.txt:711-736`
> xác nhận một call-site gửi frame chỉ có main/sub `09 01`, không đọc body. Layout create đầy đủ
> trong §3.2 là contract của `src/server/handlers/character.rs`; dump client hiện tại chưa
> cung cấp builder full-payload độc lập để đối chiếu byte-for-byte. Không được dùng
> `case 9: break` trong pseudo-C để phủ nhận path empty này.

## 2. Bảng Opcode Liên Quan (Đã Verify)

### 2.1. C → S (Client gửi lên)

| Main | Sub | Ý nghĩa | Payload (sau main+sub) | Bằng chứng |
|:---:|:---:|---|---|---|
| `09` | `02` | Kiểm tra tên nhân vật | string name | `character.rs:101-135`; client `0077f414`/ASM hiện có các call-site liên quan |
| `09` | `01` | **Tạo nhân vật** (server contract) | 14 phần — xem §3.2 | `character.rs:39-77`; full client builder chưa có body độc lập trong dump |
| `09` | `01` | **Empty send path đã xác minh** | không có body sau main/sub | `client_pseudo_c/0077f414_FUN_0077f414.asm.txt:711-736`; server `character.rs:137-143` |
| `03` | `01` | **Xác nhận vào game** | `[03][gself+0x63c]` (2 byte) | server `login.rs:88`; client builder liên quan trong `0077f414` |
| `25` | `01` | Map loaded / login complete | — | server `login.rs:238-317` |
| `08` | `01` | **Phân bổ chỉ số** (KHÔNG phải "xin stats") | `[08][01][u16 @+0xFCC][u8 @+0xFCE][u32 @+0xFCF][u8 @+0xFD3]` | ASM `0077f414:444-501`; server `stats.rs:32-44` |
| `02` | `00..07` | Các thao tác C→S của nhóm `0x02` | đã có ASM cho 8 target | `cs_op02_dispatch_cases.asm.txt:1-231`; target `0x77F883` |
| `17` | `00..3C` | Thao tác túi đồ (61 sub) | dispatcher + target table + body sub `01/02`; chưa có body riêng cho toàn bộ 61 sub | `cs_op17_dispatch.asm.txt`, `cs_op17_first_cases.asm.txt`, `cs_op17_targets_0x783405.hex` |
| `19` | `00..29` | Dispatcher C→S theo byte-map | đã có ASM cho các target hợp lệ; một số sub map về default | `cs_op19_dispatch_cases.asm.txt`, `cs_op19_byte_map_0x7853e5.hex` |
| `1C` | `00..06` | Dispatcher C→S theo target table | sub `00/03` là default; sub `01/02/04/05/06` có body ASM | `cs_op1c_dispatch_cases.asm.txt`, `cs_op1c_targets_0x785dbb.hex` |

> **Lưu ý quan trọng:** `opcode_08.md:267-283` ghi “OP 0x08 KHÔNG có phía gửi” là
> **sai một phần**. Bảng byte `table_0x77F474` cho thấy `byte[0x08] = 0x08` → builder
> `0x0077FF2E` tồn tại. ASM hiện tại xác nhận client gửi thêm hai field trailing
> (`u32` tại `+0xFCF`, `byte` tại `+0xFD3`) sau `u16 + u8`; server hiện chỉ parse
> `stat_id` và `target_val` (`stats.rs:32-44`). Ý đúng là: client gửi `08 01` khi
> **phân bổ chỉ số**, không phải để *yêu cầu* snapshot.

### 2.2. S → C (Server đẩy xuống) — trong chuỗi Logined1

| Main | Sub | Nội dung | Ghi chú |
|:---:|:---:|---|---|
| `14` | `08` / `21 00` | end-talk marker | `spawn.rs:73-75` |
| `03` | *(không có sub)* | **id LE32, sex, ghost, map/x/y LE16, dir, hair, color, equip list, reborn, job, name VISCII** | `spawn.rs:81-120` |
| `05` | `03` | **Snapshot chỉ số đầy đủ** (xem §3.1) | `spawn.rs:128-187` |
| `0F` | `08/14/0A/12/13` | pet summary | `spawn.rs:370-394` |
| `21` | `02` | PK / tham chiến | `spawn.rs:549` |
| `17` | `05` | **HÒM ĐỒ (danh sách túi đồ)** | Rust `session.rs:425-427`; client parser `FUN_0076BFC8` xác nhận stride 12B |
| `1E` | `01` | **Container 51 ô** (`player+0xA48`) | `n = (Len(RP)−1)/12` record 12B: `[slot:1][itemCode:2LE][b3][b4][b5][b6][b7][qty:4LE]` | Client `case_026:28-29 → FUN_007707a0`; **KHÔNG phải stall** — xem §3.3.1 (đã đính chính 2026-10-01) |
| `17` | `1E` | **Stall/vendor listing** | `[1E][name_len:1][name:name_len][stall_id:1][(slot:1,id:4LE)...]` | Client `case_020:149 → FUN_00730F4C` tạo `TVenderItem` + UI `StallN` (chuỗi `"Stall"`) |
| `17` | `2F` | Túi đeo | Rust `session.rs:440-442`; client layout chưa tách riêng trong dump hiện tại |
| `17` | `66` | Lưu lang | Rust `session.rs:445-447`; client layout chưa tách riêng trong dump hiện tại |
| `17` | `0B` | Trang bị đang mặc | Client parser `FUN_0077CC2C` dùng stride 10B, **không giống** 12B của `dump_table()` |
| `1A` | `04` | **Gold** `[A:4 LE][B:4 LE]` | `spawn.rs:561-564` |
| `27` | `09` | Server name + id LE32 | `spawn.rs:305-314` |
| `28` | `01` | Hotkeys | `session.rs:480-493` |
| `23` | `04` ×3 | store (god/hp/sp) | `spawn.rs:592-594` |
| `08` | `01` | Push chỉ số đơn lẻ (delta/attr) | **không có trong Logined1**; client parser S→C `case_008` có body, xem §5 gap #1 |

---

## 3. Layout Byte Chi Tiết

### 3.1. Gói `05 03` — Bảng chỉ số (Stats Snapshot)

Builder: `src/server/spawn.rs:128-187` (`fn stats()`).
Tài liệu `opcode_05.md:130-151` mô tả header/tail, nhưng đây là **spec thứ cấp**, không
phải parser client đầy đủ đã được dump. `client_pseudo_c/case_005_0078C7DB_FUN_0078c7db.c`
xác nhận handler `05` xử lý dữ liệu actor, nhưng không đủ để chứng minh toàn bộ tail
`05 03` byte-for-byte. Vì vậy claim “header cố định đến `p[0x70]`” chỉ giữ mức tham chiếu
tài liệu, chưa phải bằng chứng capture.

```
[05] [03]
[thuoctinh:1]            // hệ (element)
[hp:2 LE] [sp:2 LE]
[int1:2 LE] [atk:2 LE] [def:2 LE] [agi:2 LE]
[hpx:2 LE] [spx:2 LE]
[lv:1]
[texp:4 LE]              // exp
[skill_point:2 LE] [point:2 LE]
[tiengtam:4 LE]          // danh dự
[hp_max:2 LE] [sp_max:2 LE]
[atk2:4 LE] [def2:4 LE] [int2:4 LE] [agi2:4 LE] [hpx2:4 LE] [spx2:4 LE]   // gear bonus
[F4 01] × 5
[90 byte zero]           // ← LECH so với Bear/client spec, xem §5 gap #4
[skill list]
```

> **`05 03` KHÔNG chứa name và KHÔNG chứa id.**
> - `id` + `name` nằm ở frame **`03`** (`spawn.rs:99` id LE32, `spawn.rs:114` name VISCII).
> - `id` lại xuất hiện trong **`27 09`** (`spawn.rs:309`).
> - `gold` ở **`1A 04`**, `map/x/y` ở **`03`**.

### 3.2. Gói `09 01` — Server contract create + empty client path

Payload create sau `[09][01]` (= server `data[6..]`) được parser Rust xác định trong
`src/server/handlers/character.rs:39-77`. Đây là **contract phía server**; các dòng asm
cũ từng được dùng để gán cho client không còn là bằng chứng đầy đủ, vì pseudo-C hiện tại
còn cảnh báo Ghidra loại bỏ nhiều unreachable block.

Độc lập với contract create, ASM `client_pseudo_c/0077f414_FUN_0077f414.asm.txt:711-736`
xác nhận một call-site gửi `09 01` **không có body**, chỉ tạo hai byte main/sub rồi gọi
`TForm1_CY_AddSedQueue` (`0x0051633C`). Server hiện map `payload.is_empty()` sang
`handle_enter_game` (`character.rs:137-143`).

| Off | Len | Field | Độ xác minh |
|---:|---:|---|---|
| 0 | 1 | sex | server parser `character.rs:39-77` |
| 1 | 1 | (unused) | server parser `character.rs:39-77` |
| 2 | 1 | hair | server parser `character.rs:39-77` |
| 3 | 1 | (gap) | server parser `character.rs:39-77` |
| 4 | 4 | color A (u32 LE) | server parser `character.rs:39-77` |
| 8 | 4 | color B (u32 LE) | server parser `character.rs:39-77` |
| 12 | 1 | thuoctinh (1..4) | server parser `character.rs:39-77` |
| 13–18 | 6 | **int, atk, def, hpx, spx, agi** | server parser `character.rs:39-77` |
| 19 | 1 | pass1_len | server parser `character.rs:39-77` |
| 20.. | n | pass1 | server parser `character.rs:39-77` |
| 20+n | 1 | pass2_len | server parser `character.rs:39-77` |
| 21+n.. | m | pass2 | server parser `character.rs:39-77` |

> **Kết luận:** full create layout đã được xác minh ở boundary server, nhưng chưa được
> xác minh byte-for-byte bằng một client builder riêng trong dump hiện tại. Empty `09 01`
> là một call-site khác và đã được xác minh bằng ASM.

### 3.3. Gói `17 05` — Hòm đồ (Inventory Dump)

Server serializer: `src/server/session.rs:424-468` (`dump_table`).

```
Frame = [F4 44] [len LE16] [17] [05] [entry...]  (toàn frame XOR 0xAD)

17 05 client entry = 12 byte:
+--------+-----------+-------+--------+----------+-----------------+--------+-----------+
| slot:1 | itemId:2LE| cnt:1 | doben:1| long_val:1| (giatri_long+100):1 | khang:1 | texp:4LE |
+--------+-----------+-------+--------+----------+-----------------+--------+-----------+
```

Bằng chứng client: `client_pseudo_c/0076bfc8_FUN_0076bfc8.c:87-203` và
`0076bfc8_FUN_0076bfc8.asm.txt:35-180`; hàm tính số entry bằng `IDIV 0x0C`,
đọc `slot`, `itemId`, các byte trạng thái và 4 byte `texp`.

**Không được suy rộng layout 12B cho mọi container:**

- `17 05`: parser `FUN_0076BFC8`, stride **12B**, có byte slot.
- `17 0B`: parser `FUN_0077CC2C`, stride **10B**, không có byte slot đầu tiên trong
  entry client (`0077cc2c_FUN_0077cc2c.c:64-144`, ASM tương ứng). Đây là mismatch
  nghiêm trọng với `dump_trangbi()` hiện vẫn gọi `dump_table()` 12B.
- `1E 01`: **container 51 ô** (`player+0xA48`), record **12B** — parser `FUN_007707a0`
  (`client_pseudo_c/007707a0_FUN_007707a0.c:78-200`), `n = (Len(RP)−1)/12`. Layout này
  **khớp** `dump_table()` 12B của Rust. Xem §3.3.1.a.
  *(Đính chính 2026-10-01: bản trước gán nhầm frame này là stall/vendor.)*
- `17 1E`: **stall/vendor** — header `name_len/name/stall_id` + record 5 byte
  `(slot, u32 id)`, tạo `TVenderItem`/UI `StallN`. Xem §3.3.1.b. Rust chưa implement.
- `17 2F`, `17 66`: layout client chưa được tách riêng trong dump hiện tại; Rust vẫn dùng
  cùng `dump_table()` 12B, đây là giả định cần kiểm tra thêm.
- `28 01`: format hotkey riêng, không dùng entry inventory.

> **ThingData 35 byte KHÔNG nằm trên đường truyền này.**
> `write_thing_data` (`protocol/writer.rs:127`) và `read_thing_data` (`protocol/reader.rs:148`)
> **không có call site nào** trong toàn repo.

### 3.3.1. Gói `1E 01` — Container 51 ô (ĐÃ ĐÍNH CHÍNH 2026-10-01: KHÔNG phải stall)

> **⚠️ Đính chính.** Bản trước của mục này ghi `1E 01` = stall/vendor dựa trên
> `FUN_00730F4C`. **Gán nhãn sai.** Đã đối chiếu lại bảng dispatch:
>
> | Frame | Handler | Ngữ nghĩa thật |
> |:--|:--|:--|
> | **`17 1E`** | `case idx 20` (main `0x17`) `case 0x1e:` → `FUN_00730F4C` | **stall/vendor** |
> | **`1E 01`** | `case idx 26` (main `0x1E`) `case_026:28-29` → `FUN_007707a0` | **container 51 ô** |
>
> Bằng chứng: `jumptable_byte200_0x78A8EE[0x1E] = 26` ≠ `[0x17] = 20`, và
> `FUN_00730F4C` chỉ được gọi từ `client_pseudo_c/007902fb_FUN_007902fb.c:149`
> (grep `0x00730f4c` trong `client_pseudo_c/`: chỉ `007902fb` + `case_020`).

#### 3.3.1.a `1E 01` — layout thật (mức A)

Client đọc `n = (Len(RP) − 1) / 12` record, mỗi record **12 byte**
(`client_pseudo_c/007707a0_FUN_007707a0.c:78-200`):

```text
[1E] [01]
[record:12] × N
  ├─ slot       :1     // guard <= 0x32 (50); index vào player+0xA48 + slot*4
  ├─ itemCode   :2 LE  // FUN_0077eb9c (Word LE) → local_1a
  ├─ b3         :1     // local_e
  ├─ b4         :1     // local_f
  ├─ b5         :1     // local_10
  ├─ b6         :1     // local_11
  ├─ b7         :1     // local_12
  └─ qty        :4 LE  // FUN_0077ed68 (DWORD LE) → local_18
```

Client dùng 5 byte giữa (`local_e..local_12`) cộng với `FUN_0065b580(itemCode, 0, qty)`
để gọi một **virtual method** trên object tại `player+0xA48` (`007707a0_FUN_007707a0.c:172-179`).

> **Hệ quả cho contract Rust:** `src/server/session.rs:434-437` `dump_tientrang()` gửi
> `1E 01` với **record 12 byte** → **khớp về layout**. "Mismatch" ở gap #6 của §5.2 trước đây
> là do **nhận nhầm opcode** khi gán nhãn stall, **không phải** do sai layout.
> Điểm còn mở chuyển sang câu hỏi ngữ nghĩa: `tientrang` (storagetype 2) có đúng là container
> `+0xA48` 51 ô không, hay thuộc `+0xB14` (51 ô) / `+0xDC0` (26 ô).
> Migration `migrations/0001_init.sql:99` ghi slot `1..50` cho TienTrang — **khớp** guard 51 ô.
> Xem thêm [`opcode_1e.md`](./opcode_1e.md) §4 (đã mô tả đúng `0x1E` là nhóm bảng slot
> `+0xA48`/`+0xB14`/`+0xDC0`, **không** phải stall).

#### 3.3.1.b `17 1E` — Stall/vendor (layout đã xác minh, mức A)

`FUN_00730F4C` được dispatcher S→C gọi tại `client_pseudo_c/007902fb_FUN_007902fb.c:149`.
Body đã dump:

- `client_pseudo_c/00730f4c_FUN_00730f4c.c:32-175`
- `client_pseudo_c/00730f4c_FUN_00730f4c.asm.txt:5-173`
- Đối chiếu callee: `test/ts_decompile/functions/00768638_FUN_00768638.c:18-23`

```text
[17] [1E]
[name_len:1]                         // A; độ dài tên/stall
[name:name_len]                      // A byte, client copy tối đa 14 byte
[stall_id:1]                         // B; dùng để tạo/cập nhật StallB
[record:5] × N
  ├─ slot:1                          // client kiểm tra <= 0x19 (25)
  └─ item_id:4 LE                   // đọc bởi FUN_0077ef7c
```

Công thức: `N = (len − (A + 3)) / 5`. Sau khi đọc record, client gọi `FUN_0071e164` với
tên đã cắt ngắn và `stall_id` → tạo `TVenderItem`, cập nhật `StallN`. Bằng chứng trực tiếp
cho nhãn "stall": `_LStrCat3(&local_144, "Stall", IntToStr(stall_id))` tại
`00730f4c_FUN_00730f4c.c:161-168`.

> **Rust chưa có frame `17 1E`** — đây là gap thật (stall chưa được implement), khác với
> `1E 01` vốn đã đúng layout. Chi tiết: [`battle/README.md`](./battle/README.md) §6.1.

### 3.4. Quy ước Slot — 1-based trên wire

- Wire slot là **1-based**: `free_slot` quét `1..=25` (`inventory.rs:30-32`),
  schema `slot (1..25)` PK `(playerid, storagetype, slot)` (`migrations/0001_init.sql:99,120`).
- Tham chiếu Bear xác nhận: `TSItemContainer.cs:148` ghi `(byte)(i + 1)` (index 0 → byte `01`),
  parsing map `items[slot - 1]` (`TSItemContainer.cs:34,63`).

---

## 4. Item 32012 × 4 Tại Slot 2 — Kết Quả Đối Chiếu

**Câu hỏi:** server phải tạo sẵn item `32012` số lượng `4` ở **slot thứ 2** của túi đồ.

### 4.1. Source Rust hiện tại

| Hạng mục | Giá trị | Vị trí |
|---|---|---|
| Seed homdo khi tạo nhân vật | `slot: 1, id: 32012, count: 4` | `src/server/handlers/character.rs:264-270` |
| Comment tài liệu | "Homdo type 1: 32012×4; Trangbi type 8 **slot 2**: 19737" | `character.rs:8`, `character.rs:230-232` |
| Seed trang bị | `slot: 2, id: 19737, count: 1, agi1: 1, loai: 2` | `character.rs:271-279` |
| Ghi DB nguyên tử | insert đúng `slot 1 / slot 2` ở trên | `src/db/modern/sqlite/session.rs:539-552` |
| Test khẳng định | `slot == 1 && id == 32012 && count == 4` | `tests/create_char_atomic_test.rs:151-160` |
| `0x7D04` trong repo | **0 occurrences** | — |

### 4.2. Kết luận: "slot thứ 2" nghĩa là gì?

Vì wire slot là **1-based**, có 2 cách diễn giải "vị trí slot thứ 2":

| Diễn giải | Giá trị Rust | Khớp? |
|---|---|:---:|
| "Slot thứ 2" = ô **thứ 2** của túi (1-based, tức byte `02`) | Rust ghi byte `01` | ❌ |
| "Slot thứ 2" = **index 1** trong mảng 0-based → wire byte `01` | Rust ghi byte `01` | ✅ |
| "Slot thứ 2" = nhầm với **trangbi slot 2** (item 19737) | hai thứ khác nhau | ⚠️ |

**Verdict:** Rust seed ở **slot byte `01` = array index 0 (ô đầu tiên của túi)**.
Không có giá trị slot `2` nào cho 32012. Comment "slot 2" trong `character.rs:230-232`
ám chỉ **trang bị 19737 ở trangbi slot 2**, **không phải** hòm đồ.

### 4.3. Bằng chứng phía Client (decompile)

- `client_pseudo_c/006547a8_FUN_006547a8.c:55-70` và ASM tương ứng xác nhận client có
  đúng một điểm kiểm tra ID trong bộ entry nội bộ:
  ```c
  if (iVar2 - 0x7D04U < 4) {
    return iVar2 + -0x7D08;
  }
  ```
  Vì vậy mapping là `32012 → -4`, `32013 → -3`, `32014 → -2`, `32015 → -1`.
- Cùng hàm quét `local_9` từ `0` đến `0x14` (`21` entry) và truy cập
  `param_1 + 0x158 + local_9*4`. Đây là **mảng object nội bộ của client**, không phải
  bằng chứng về wire slot.
- Hàm không đọc field quantity/count; vì vậy decompile không xác nhận `count = 4`.
- Hàm cũng không map `local_9` sang wire slot `01` hay `02`; không thể dùng hàm này để
  kết luận 32012 nằm ở ô thứ hai.
- Server tham chiếu Bear không seed 32012 (`TSCharacter.cs:733,745`; inventory mặc định
  rỗng), nên Bear cũng không cung cấp bằng chứng cho qty/slot này.
- 32012 vẫn được ghi nhận là item gate của eve/NPC tại
  `.scratch/op-working/research-checkpoint-2-click-npc-bridge.md:22-47`.

**Kết luận sau xác minh:** client xác nhận ID `32012` tồn tại và được `FUN_006547a8`
map thành `-4`; client **không** xác nhận quantity `4` hoặc wire slot `02`. Rust hiện
seed `32012 × 4` ở wire slot byte `01`, phù hợp với test nội bộ nhưng vẫn là quyết
định thiết kế của server, chưa phải dữ liệu được client xác nhận.


---

## 5. Đánh Giá Source Rust: Đã Đáp Ứng Chưa?

### 5.1. ✅ Đã cài đặt / xác minh được

1. **Bảng chỉ số được đẩy** — frame `05 03` chứa hp/sp/int/atk/def/agi/hpx/spx/level/exp/
   skill-point/point/danh dự/hp_max/sp_max + gear bonus (`spawn.rs:502-526`).
   Name + id + map + xy nằm ở frame `03` (`spawn.rs:83-120`); gold ở `1A 04`
   (`spawn.rs:561-564`).
2. **Hòm đồ `17 05` được đẩy và đã đối chiếu parser client** — frame `17 05` ở Logined1
   bước 10 (`spawn.rs:552`), layout 12 byte/entry (`session.rs:450-468`), parser client
   `FUN_0076BFC8` xác nhận cùng stride/field order. **Không áp dụng layout này cho `17 0B`:**
   parser `FUN_0077CC2C` dùng stride 10B, trong khi `dump_trangbi()` hiện vẫn serialize 12B.
   **Cũng không áp dụng layout này cho `17 1E` (stall):** `FUN_00730F4C` đọc header
   `name_len/name/stall_id` và record 5 byte, đồng thời cập nhật `TVenderItem`/UI
   `StallN`; xem §3.3.1.b. (`1E 01` thì **có** dùng entry 12B — xem §3.3.1.a.)
3. **Các dispatcher C→S đã có bằng chứng ASM** — `02` có 8 target body; `17` có target table
   61 sub và body sub `01/02`; `19` có byte-map/target table và ASM các target hợp lệ;
   `1C` có target table và ASM sub `01/02/04/05/06` (sub `00/03` là default).
4. **Seed starter nhất quán trong source Rust** — 32012×4 @ homdo slot 1 (1-based) +
   19737 @ trangbi slot 2, cả in-memory lẫn DB nguyên tử, có test khẳng định
   (`create_char_atomic_test.rs:151-160`). Client chỉ xác nhận mapping ID `32012→-4`,
   không xác nhận qty/slot.
5. **Không có bằng chứng query stats/inventory trong các path đã dump** — `05 03`/`17 05`
   là push S→C; `08 01` là builder phân bổ C→S; `17`/`02`/`19`/`1C` là dispatcher hành động,
   không phải request snapshot. `25 01` là handshake map-loaded đầu tiên sau Logined1 và đã
   được xử lý (`login.rs:238-317`); sau đó vẫn có các opcode hành động `08/17/...`.
6. **Các builder `02/08/09 01 empty` đã có ASM trực tiếp** — không nên dùng
   `case ...: break` của Ghidra làm bằng chứng phủ nhận chúng.

### 5.2. ❌ Khoảng trống / đáng ngờ sau đợt xác minh

| # | Mức độ | Vấn đề | Chi tiết |
|:-:|:--:|---|---|
| 1 | **CAO** | **Logined1 không push `08 01`** | Rust `spawn.rs` không gửi frame `08 01` trong chuỗi login. Bear `loginChar` (`TSCharacter.cs:5100-5445`) có bộ push lớn; tuy nhiên chưa có capture chạy `aLogin.exe` để kết luận HUD client bắt buộc cần toàn bộ các frame đó ngay lúc login. Không push là khác biệt protocol cần xử lý/verify, không phải bằng chứng chắc chắn client sẽ hiện 0/stale. |
| 2 | **TRUNG BÌNH** | **Gear bonus có thể bằng 0 khi login lại từ DB** | `recompute_stats()` không được gọi trên path load; `SqliteSessionRepository::load` (`src/db/modern/sqlite/session.rs:19-114`) không nạp `int2/atk2/def2/hpx2/spx2/agi2` (default `Session`) → `05 03` gửi bonus = 0 cho nhân vật đã có trang bị. |
| 3 | **TRUNG BÌNH** | **Item load từ DB có thể mất template fields** | `SqliteSessionRepository::load_items` (`src/db/modern/sqlite/session.rs:176-199`) chỉ đọc `slot/itemid/quantity/damage`; `loai`, `lv`, các field `_1/_2` mặc định 0. Hệ quả gate equip và gear recompute có thể sai sau relog. |
| 4 | **TRUNG BÌNH** | **Tail `05 03` vẫn chưa đối chiếu byte-exact** | Rust: `F401×5` + 90 zero + skill list (`spawn.rs:182-184`); Bear: `500×5` + 43 zero + skill list + block 32 byte cuối; spec `opcode_05.md` mô tả parser fixed header/tail khác. Đợt dump mới không cung cấp capture `aLogin.exe`, nên chưa thể chọn layout chuẩn. |
| 5 | **CAO** | **`17 0B` đang có khả năng lệch layout** | Client `FUN_0077CC2C` dùng stride 10B/no leading slot; Rust `dump_trangbi()` dùng `dump_table()` 12B. Cần sửa serializer hoặc xác minh lại frame trước khi coi `17 0B` là đúng. |
| 6 | ~~**CAO**~~ → **ĐÃ ĐÓNG** | ~~**`1E 01` không dùng layout inventory 12B**~~ | **Sai — do nhận nhầm opcode.** `FUN_00730F4C` (stall) được gọi từ main `0x17` (`17 1E`), không phải `1E 01`. `1E 01` thật là container 51 ô với record **12B** (`FUN_007707a0`), **khớp** `dump_table()` của Rust. Xem §3.3.1. Gap còn lại (mức thấp) là xác định `tientrang` có đúng là container `+0xA48` không. |
| 7 | **TRUNG BÌNH** | **Full create `09 01` chưa có client builder độc lập** | ASM xác nhận empty `09 01`; layout 14 phần vẫn chỉ được xác minh tại server parser `character.rs`. Không được ghi “client builder byte-exact” như bản cũ. |
| 8 | THẤP | **Chưa validate end-to-end với `aLogin.exe` thật** | `handoff-login-handshake.md` còn open action; test tự động chủ yếu đếm frame, chưa kiểm tra client UI/session state. |
| 9 | THẤP | **32012 qty/slot chưa có nguồn đối chiếu** | `FUN_006547a8` xác nhận ID mapping `32012→-4` và quét 21 entry nội bộ, nhưng không đọc quantity hoặc wire slot. |

### 5.3. Đề xuất hành động (theo thứ tự ưu tiên)

1. **Tách serializer `17 0B` khỏi `dump_table()` 12B** — lấy client `FUN_0077CC2C` làm
   chuẩn 10B, hoặc chụp/đối chiếu frame thật trước khi sửa. Đây là mismatch có thể làm
   lệch toàn bộ equipment record.
2. ~~**Đối chiếu lại contract `1E 01`**~~ — **ĐÃ ĐÓNG 2026-10-01.** `1E 01` là container
   51 ô với record 12 byte, **khớp** `dump_table()`; gap cũ là do nhận nhầm với `17 1E`.
   Việc còn lại (mức thấp): xác nhận `tientrang` (storagetype 2) có đúng là container
   `player+0xA48` không — xem §3.3.1.a. Việc thật còn thiếu: **implement frame `17 1E`**.
3. **Thêm push `08 01` hoặc chứng minh HUD không cần bộ push đó** — tham chiếu Bear
   `loginChar`/`refreshChr`, rồi đối chiếu bằng capture `aLogin.exe`; không chỉ suy từ
   thiếu frame trong Rust.
4. **Gọi `recompute_stats()` ngay sau khi load session** (trong `login_db` hoặc
   `SqliteSessionRepository::load`) để `05 03` mang gear bonus đúng.
5. **Bổ sung `loai/lv/_1/_2` vào `load_items`** hoặc join với bảng template để giữ
   được gate equip và gear bonus sau relog.
6. **Đối chiếu byte-exact tail `05 03`** bằng capture từ client thật; chưa tự sửa layout
   dựa trên Bear hoặc Ghidra khi hai nguồn đang lệch nhau.
7. **Tìm builder full create `09 01`** nếu cần tuyên bố byte-exact client↔server; hiện
   chỉ có server contract và empty client send path.
8. **Chạy end-to-end với `aLogin.exe`** theo `handoff-login-handshake.md`.


---

## 6. Danh Sách UNDETERMINED (sau đợt xác minh `client_pseudo_c/`)

Các mục dưới đây đã được phân loại lại thành **đã xác minh**, **xác minh một phần**, hoặc
**còn mở**; không còn dùng danh sách cũ như một khối claim “chưa có bằng chứng”.

1. **Parser client `17 05` / `17 0B` / `1E 01` — layout đã đóng; gap `1E 01` đã đóng**
   - `17 05`: `client_pseudo_c/0076bfc8_FUN_0076bfc8.c/.asm.txt` có body; stride 12B,
     có slot → layout 12B của Rust được xác minh.
   - `17 0B`: `0077cc2c_FUN_0077cc2c.c/.asm.txt` có body; stride 10B, không có leading
     slot → layout 12B của `dump_trangbi()` **không còn được coi là đúng**.
   - `1E 01`: `007707a0_FUN_007707a0.c` đã có body; layout là **container 51 ô**
     (`player+0xA48`), record 12 byte `(slot, itemCode u16 LE, 5 byte, qty u32 LE)`,
     `n = (Len(RP)−1)/12` → **khớp** `dump_table()` 12B của Rust.
     *(Đính chính 2026-10-01: bản trước gán frame này là stall/vendor do nhầm với `17 1E`.)*
   - `17 1E`: `00730f4c_FUN_00730f4c.c/.asm.txt` đã có body; layout là header
     `name_len/name/stall_id` + record 5 byte `(slot, u32 id)`; `FUN_0071e164` tạo
     `TVenderItem` và UI `StallN`. **Rust chưa implement frame này** — gap thật.
   - `0077d0b9` là parser `17 0C`, không phải parser `1E 01`.

2. **C→S body `0x02` / `0x17` / `0x19` / `0x1C` — claim cũ đã sửa**
   - `02`: đã dump ASM 8 sub/target trong `cs_op02_dispatch_cases.asm.txt`.
   - `17`: đã có dispatcher, target table 61 sub và body sub `01/02`; chưa có source
     C/ASM riêng cho toàn bộ 61 sub.
   - `19`: đã có byte-map, target table và ASM các target hợp lệ; các sub không map
     về target hợp lệ rơi vào default.
   - `1C`: đã có target table và ASM sub `01/02/04/05/06`; sub `00/03` là default.
   Vì vậy không còn được viết rằng “chỉ có jump table, không có body”; phần còn thiếu là
   **coverage đầy đủ từng sub**, không phải bằng chứng rằng client không gửi các opcode này.

3. **Literal/data `0x78A8xx` — đã dump, không còn là gap**
   - `cs_literals_0x78a818_0x78a8ed.hex` phủ `0x78A818..0x78A8ED` (214 byte).
   - `cs_lit_0x78a834.hex`, `cs_lit_0x78a848.hex`, `cs_lit_0x78a854.hex`,
     `cs_lit_0x78a860.hex`, `cs_lit_0x78a880.hex` đã có trong `client_pseudo_c/`.
   - Call-site trong `0077f414` xác nhận đây là Delphi string/data constants và buffer
     metadata, không phải raw wire fields. `0x78A834` được dùng trong path main `00`;
     `0x78A838..0x3E` được copy vào object; `0x78A848/854/860/880` được dùng trong các
     builder liên quan. Semantic đầy đủ của từng constant vẫn cần call-site/UI context,
     nhưng claim “không có file dump” đã sai.

   | Địa chỉ | Bytes quan sát được | Call-site/ý nghĩa tối thiểu |
   |---|---|---|
   | `0x78A834` | `00 00 00 00 06 "123123" 00 ...` | được đưa vào queue ở path main `00`; không phải raw payload field |
   | `0x78A838..0x3E` | `FF FF FF FF 01 00 00` | copy 7 byte vào object `+0x648`; là buffer/metadata runtime |
   | `0x78A848` | `20 20 00 00 ...` | chuỗi fallback khi độ dài tên < 2 |
   | `0x78A854` | `30 00 00 00 ...` | chuỗi `"0"` dùng trong `StrToInt` ở builder liên quan |
   | `0x78A860` | `31 32 33 34 35 36 37 38 00` | chuỗi `"12345678"` dùng làm dữ liệu test/reset trong builder |
   | `0x78A880` | metadata + `31 32 33 34 35 36 37 38 39 30 00` | chuỗi `"1234567890"` và buffer liên quan; vùng này xen kẽ data/code |

4. **32012 qty/slot — mapping ID đã xác minh, wire layout chưa**
   - `FUN_006547a8` xác nhận `32012..32015 → -4..-1` và quét 21 entry nội bộ.
   - Không có bằng chứng quantity `4` hoặc wire slot `02`; client không đọc count/slot
     trong hàm này. Rust `slot:1, count:4` vẫn là seed nội bộ của server.

5. **Field trailing của `08 01` — wire presence đã xác minh, semantic chưa**
   - ASM `0077f414_FUN_0077f414.asm.txt:469-500` xác nhận gửi `u16`, `u8`, `u32`, `u8`
     sau `08 01`; field cuối đọc từ `+0xFCF` và `+0xFD3`.
   - Server `stats.rs:37-44` chỉ parse `stat_id` và `target_val`; chưa có tên nghiệp vụ
     chắc chắn cho `u32`/byte trailing. Gap còn lại là **semantic**, không phải “client có
     gửi hay không”.

6. **`09 01` empty — đã xác minh một call-site; full create vẫn chưa đóng**
   - ASM `0077f414_FUN_0077f414.asm.txt:711-736` tạo rồi gửi frame chỉ có main/sub,
     không đọc body; server map `payload.is_empty()` sang enter-game.
   - Layout create 14 phần ở §3.2 vẫn là server contract. Chưa tìm thấy client builder
     full-payload tương ứng trong dump hiện tại, nên không được tuyên bố full create đã
     byte-exact hai chiều.


---

## 7. Sửa Lỗi Tài Liệu Đề Xuất (đã cập nhật theo ASM mới)

1. **`opcode_02.md:176-180,218`**: claim “client never sends 0x02; chat at 0x1A” dựa trên
   `case 2: break` là **sai**. `cs_op02_dispatch_cases.asm.txt` xác nhận dispatcher 8 sub
   và có body gửi thực tế; server cũng xử lý chat C→S tại `chat.rs:68-227`. `1A` không
   phải redirect của chat.
2. **`opcode_3c.md:224-226`, `opcode_3d.md:24,557`**: index dword table bằng raw opcode là sai.
   Đúng: op `0x3C` → `dword[0x32] = 0x007892B9`; op `0x3D` → `dword[0x33] = 0x007893E6`;
   `0x0078A142` là builder của op `0x48`, `0x0078A318` là builder của op `0xC7`.
3. **Line-drift citations**: các line number cũ trong `0077f414_FUN_0077f414.c` phải được
   đối chiếu lại; pseudo-C có nhiều `WARNING: Removing unreachable block`, nên các
   `case ...: break` không phải bằng chứng thiếu code.
4. **`opcode_08.md:267-283`**: sửa thành “builder C→S `08 01` tồn tại và dùng cho phân bổ
   điểm; không phải query”. ASM mới xác nhận thêm `u32 @+0xFCF` và `byte @+0xFD3`; server
   hiện bỏ qua các field này.
5. **Mọi claim “client never sends X” suy từ `case` rỗng**: phải bị loại bỏ khi
   `byte[X] != 0` hoặc ASM dispatch đã có target. Đây là lỗi recovery của Ghidra, không
   phải bằng chứng protocol.
6. **`0x17`/inventory docs**: không còn ghi mọi container dùng cùng entry 12B. `17 05` là
   12B; `17 0B` là 10B; `1E 01` là container 51 ô record 12B (§3.3.1.a); stall là
   **`17 1E`** với header + record 5 byte `(slot, u32 id)` (§3.3.1.b).
   *(Đính chính 2026-10-01: bản trước gán `1E 01` = stall — sai.)*
8. **`post_login_stats_inventory_flow.md` §2.2/§3.3.1/§5.2 gap #6/§6.1**: đã đính chính
   `1E 01` (container 51 ô) và tách `17 1E` (stall). Chi tiết ở §3.3.1 và
   [`battle/README.md`](./battle/README.md) §6.1.
7. **`0077d0b9`**: phải ghi là parser `17 0C`, không dùng nhầm làm bằng chứng cho `1E 01`.


---

## 8. Bảng Mã Gọi Hàm Tóm Tắt (Execution Summary)

```
1.  Client ──► 01 [auth]                       (đã cover bởi login_flow_research.md)
2.  Server ──► danh sách nhân vật / 09 03
3.  Client ──► 09 02 + name                     character.rs:101
    Server ──► 09 03 ss
4.  Client ──► 09 01 + full create payload      server contract: character.rs:39-77
    (client full builder chưa có body độc lập; empty 09 01 đã xác minh tại ASM 007803C0)
    Server ──► tạo DB nguyên tử: homdo 32012×4 @slot1, trangbi 19737 @slot2
                                                    character.rs:138 → session.rs:483-556
5.  Client ──► 03 01                            login.rs:88  (XÁC NHẬN VÀO GAME)
6.  Server ──► CHUỖI LOGINED1 (21 frame)        spawn.rs:471-604
    ├─ 03         id LE32 + name VISCII + map/xy + equip list
    ├─ 05 03      hp/sp/int/atk/def/agi/lv/exp + gear bonus   ◄── STATS
    ├─ 17 05      hòm đồ (client entry 12B: slot,id,cnt,...) ◄── INVENTORY
    ├─ 1E 01 / 17 2F / 17 66 / 1A 04 / 27 09 / 28 01 / 23 04×3
    │              1E01: container 51 ô, record 12B (đã khớp dump_table)
    │              17 1E: stall/vendor (header + record 5B) — Rust chưa implement
    │              17 2F/17 66: layout client chưa đủ bằng chứng
    ├─ 17 0B      trang bị; client parser stride 10B — Rust hiện còn 12B, cần sửa/verify
    └─ 02 0B ×2 banner, 14 08/14 21 marker, 21 02 PK, 0F xx pet
7.  Client ──► 25 01 (map loaded)               login.rs:238  → broadcast appear
8.  Về sau:
    ├─ 08 01 C→S = phân bổ điểm; ASM xác nhận u16+u8+u32+u8, server dùng 2 field đầu
    ├─ 08 01 S→C = push delta theo handler
    └─ 17/02/19/1C = các dispatcher hành động; không có bằng chứng query snapshot
```


---

*Tài liệu được tạo từ khảo sát decompile + source Rust ngày 2026-09-23 và cập nhật xác minh ngày 2026-09-24. Các claim đã phân biệt rõ **đã xác minh**, **xác minh một phần**, và **còn mở**; nguồn chính là `client_pseudo_c/`, với `test/ts_decompile/` chỉ dùng để đối chiếu.*
