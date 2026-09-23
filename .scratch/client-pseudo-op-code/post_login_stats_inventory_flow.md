# Nghiên Cứu Luồng Sau Đăng Nhập: Xác Nhận Chỉ Số (Stats) & Túi Đồ (Hòm Đồ)

Ngày: 2026-09-23
Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Tài liệu tham chiếu:
- Client decompile: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/` (C + ASM)
- Tài liệu nền: [`login_flow_research.md`](./login_flow_research.md) (kết nối & gửi gói auth)
- Các tài liệu opcode hiện có: `opcode_03.md`, `opcode_05.md`, `opcode_08.md`, `opcode_09.md`, `opcode_17.md`, `opcode_1a.md`
- Source Rust: `src/` trong repo này
- Tham chiếu server C#: `TS_Server_Bear/TS_Server/`

---

## 1. Tổng Quan Luồng (Big Picture)

**Kết luận quan trọng nhất:** Client **KHÔNG** gửi opcode nào để "xin xác nhận chỉ số" hay "xin danh sách túi đồ".
Cả bảng chỉ số lẫn túi đồ đều là **PUSH một chiều từ Server → Client**, gửi ngay trong chuỗi
`Logined1` sau khi client xác nhận vào game bằng gói **`03 01`**.

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
       │  ──► 09 01 + payload   (TẠO NHÂN VẬT)     character.rs:138
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
       │      • 1E 01, 17 2F, 17 66, 17 0B, 1A 04, 27 09 ...
       │      • 28 01 hotkeys, 23 04 store, 18 06/07 quest sync
       │                                   │
       │  ──► 25 01             (map loaded — xong login)  login.rs:238
       │  ◄── broadcast appear + sync player/NPC khác
       │                                   │
       │  ──► 08 01 (chỉ khi PHÂN BỔ chỉ số)             stats.rs:28
       │  ◄── 08 01 (server push khi chỉ số đổi)
       │  ──► 17 xx ( thao tác túi đồ: sắp xếp, thả, dùng ) inventory.rs
       │      — KHÔNG có sub nào "xin lại toàn bộ 17 05"
```

---

## 2. Bảng Opcode Liên Quan (Đã Verify)

### 2.1. C → S (Client gửi lên)

| Main | Sub | Ý nghĩa | Payload (sau main+sub) | Bằng chứng |
|:---:|:---:|---|---|---|
| `09` | `02` | Kiểm tra tên nhân vật | string name | asm `0077f414:682-710`; server `character.rs:101-135` |
| `09` | `01` | **Tạo nhân vật** | 14 phần — xem §3.2 | asm `0077f414:502-681`; server `character.rs:39-77` |
| `03` | `01` | **Xác nhận vào game** | `[03][gself+0x63c]` (2 byte) | asm `0077f414:183-207`; server `login.rs:88` |
| `25` | `01` | Map loaded / login complete | — | server `login.rs:238-317` |
| `08` | `01` | **Phân bổ chỉ số** (KHÔNG phải "xin stats") | 10 byte: `[08][01][u16][attr][u32][byte]` | asm `0077f414:444-501`; server `stats.rs:32-44` |
| `17` | `00..3C` | Thao tác túi đồ (61 sub) | chưa dump được body | bảng sub @ `0x783405` |

> **Lưu ý quan trọng:** `opcode_08.md:267-283` ghi "OP 0x08 KHÔNG có phía gửi" là **sai một phần** —
> bảng byte `table_0x77F474` cho thấy `byte[0x08] = 0x08` → builder `0x0077FF2E` tồn tại và payload
> 10 byte đã verify khớp nguyên vẹn với parser của server tại `stats.rs:32-44`.
> Ý đúng là: client không gửi `08 01` để *yêu cầu* stats, mà chỉ gửi khi *phân bổ* điểm.

### 2.2. S → C (Server đẩy xuống) — trong chuỗi Logined1

| Main | Sub | Nội dung | Ghi chú |
|:---:|:---:|---|---|
| `14` | `08` / `21 00` | end-talk marker | `spawn.rs:73-75` |
| `03` | *(không có sub)* | **id LE32, sex, ghost, map/x/y LE16, dir, hair, color, equip list, reborn, job, name VISCII** | `spawn.rs:81-120` |
| `05` | `03` | **Snapshot chỉ số đầy đủ** (xem §3.1) | `spawn.rs:128-187` |
| `0F` | `08/14/0A/12/13` | pet summary | `spawn.rs:370-394` |
| `21` | `02` | PK / tham chiến | `spawn.rs:549` |
| `17` | `05` | **HÒM ĐỒ (danh sách túi đồ)** | `session.rs:425-427` |
| `1E` | `01` | Tiền trang | `session.rs:435-437` |
| `17` | `2F` | Túi đeo | `session.rs:440-442` |
| `17` | `66` | Lưu lang | `session.rs:445-447` |
| `17` | `0B` | Trang bị đang mặc | `session.rs:430-432` |
| `1A` | `04` | **Gold** `[A:4 LE][B:4 LE]` | `spawn.rs:561-564` |
| `27` | `09` | Server name + id LE32 | `spawn.rs:305-314` |
| `28` | `01` | Hotkeys | `session.rs:480-493` |
| `23` | `04` ×3 | store (god/hp/sp) | `spawn.rs:592-594` |
| `08` | `01` | Push chỉ số đơn lẻ (delta/attr) | **không có trong Logined1** — xem §5 gap #1 |

---

## 3. Layout Byte Chi Tiết

### 3.1. Gói `05 03` — Bảng chỉ số (Stats Snapshot)

Builder: `src/server/spawn.rs:128-187` (`fn stats()`).
Spec parser phía client: `opcode_05.md:130-151` (header cố định đến `p[0x70]`, tail là record 3 byte từ `p[0x71]`).

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

### 3.2. Gói `09 01` — Tạo nhân vật (tham chiếu đầy đủ)

Payload sau `[09][01]` (= server `data[6..]`), khớp nguyên vẹn `character.rs:39-77`:

| Off | Len | Field | Nguồn client (asm lines) |
|---:|---:|---|---|
| 0 | 1 | sex | 526–537 |
| 1 | 1 | (unused) | 541–550 |
| 2 | 1 | hair | 554–565 |
| 3 | 1 | (gap) | 569–580 |
| 4 | 4 | color A (u32 LE) | 584–590 |
| 8 | 4 | color B (u32 LE) | 591–597 |
| 12 | 1 | thuoctinh (1..4) | 599–603 |
| 13–18 | 6 | **int, atk, def, hpx, spx, agi** | 604–640 |
| 19 | 1 | pass1_len | 641–649 |
| 20.. | n | pass1 | 650–553 |
| 20+n | 1 | pass2_len | 654–662 |
| 21+n.. | m | pass2 | 663–666 |

### 3.3. Gói `17 05` — Hòm đồ (Inventory Dump)

Serializer: `src/server/session.rs:424-468` (`dump_table`).

```
Frame = [F4 44] [len LE16] [17] [05] [entry...]  (toàn frame XOR 0xAD)

Entry = 12 byte:
+--------+-----------+-------+--------+----------+-----------------+--------+-----------+
| slot:1 | itemId:2LE| cnt:1 | doben:1| long_val:1| (giatri_long+100):1 | khang:1 | texp:4LE |
+--------+-----------+-------+--------+----------+-----------------+--------+-----------+
```

Các container khác dùng **cùng entry 12 byte**: `1E 01` (tiền trang), `17 2F` (túi đéo),
`17 66` (lưu lang), `17 0B` (trang bị), `28 01` (hotkey — format riêng).

> **ThingData 35 byte KHÔNG nằm trên đường truyền này.**
> `write_thing_data` (`protocol/writer.rs:127`) và `read_thing_data` (`protocol/reader.rs:148`)
> **không có call site nào** trong toàn repo.

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

- Chỉ **MỘT** tham chiếu tới `32012` — `functions/006547a8_FUN_006547a8.c:57-58`:
  ```c
  if (id - 0x7D04 < 4) return id - 0x7D08;   // 32012→−4, 32013→−3, 32014→−2, 32015→−1
  ```
  Quét 21 slot (`local_9 == 0x15`, dòng 70) của mảng item `param_1+0x158`.
  Ý nghĩa mã trả về âm: **UNDETERMINED**.
- Grep exhaustive: `32012` (thập phân) → **0 hit** trong mọi `*.c`; `047D0000` → **0 hit** trong `*.asm.txt`.
  **Không có** tham chiếu nào về số lượng 4 hay index slot cho 32012 phía client.
- Server tham chiếu **Bear không seed 32012 כלל** (`TSCharacter.cs:733,745` chỉ có equip 19737,
  inventory rỗng `[]`; grep `32012` trong `TS_Server_Bear` = 0 hit).
- 32012 cũng được biết là **item gate của eve/NPC** (`.scratch/op-working/research-checkpoint-2-click-npc-bridge.md:22-47`).

**⇒ Con số "4 cái @ slot 2" không thể xác minh từ decompile hay Bear.
Rust hiện tại nhất quán internally (slot 1, qty 4, có test) nhưng cần đối chiếu capture/client thật.**

---

## 5. Đánh Giá Source Rust: Đã Đáp Ứng Chưa?

### 5.1. ✅ Đã cài đặt (verify)

1. **Bảng chỉ số được đẩy** — frame `05 03` chứa hp/sp/int/atk/def/agi/hpx/spx/level/exp/
   skill-point/point/danh dự/hp_max/sp_max + gear bonus (`spawn.rs:502-526`).
   Name + id + map + xy nằm ở frame `03` (`spawn.rs:83-120`); gold ở `1A 04` (`spawn.rs:561-564`).
2. **Hòm đồ được đẩy** — frame `17 05` ở Logined1 bước 10 (`spawn.rs:552`),
   layout 12 byte/entry (`session.rs:450-468`), cộng 4 container khác + `17 0B` trang bị.
3. **Seed starter nhất quán** — 32012×4 @ homdo slot 1 (1-based) + 19737 @ trangbi slot 2,
   cả in-memory lẫn DB nguyên tử, có test khẳng định (`create_char_atomic_test.rs:151-160`).
4. **Không thiếu opcode "xin stats" / "xin inventory"** — cả hai tham chiếu nói client không
   gửi `08`/`17` dạng query (`opcode_08.md:267-283`, `opcode_17.md:130-137`);
   C→S duy nhất cần xử lý sau login là `25 01` — **đã xử lý** (`login.rs:238-317`).
5. **Parse `09 01`, `09 02`, `03 01`, `08 01` khớp byte-exact** với builder phía client
   (`character.rs:39-77`, `stats.rs:32-44`, `login.rs:88`).

### 5.2. ❌ Khoảng trống / đáng ngờ

| # | Mức độ | Vấn đề | Chi tiết |
|:-:|:--:|---|---|
| 1 | **CAO** | **Không push `08 01` nào khi login** | Bear `loginChar` (`TSCharacter.cs:5100-5445`) push 10+ gói `08 01` (hpx 31, spx 32, atk 28, def 29, int 27, agi 30, hp 25, sp 26, + toàn gear bonus `0xD4/0xD2/0xD3/0xCF/0xD0/0xD6/0xDA/0xDB`). `opcode_08.md:286-303` cũng liệt kê bộ tối thiểu (class, level, hp, sp, int/atk/def/agi, exp, points). Rust Logined1 **gửi 0 gói `08 01`** → client nào dựng HUD từ `08 01` sẽ hiện số 0/stale cho tới khi equip/phân bổ điểm. |
| 2 | **TRUNG BÌNH** | **Gear bonus = 0 khi login lại từ DB** | `recompute_stats()` không được gọi trên path load (chỉ có caller từ `stats.rs`, `quest.rs`, `gm.rs`, `inventory.rs`). `SqliteSessionRepository::load` (`session.rs:19-114`) không nạp `int2/atk2/def2/hpx2/spx2/agi2` (default `session.rs:335-340`) → `05 03` gửi bonus = 0 cho nhân vật đã có trang bị. |
| 3 | **TRUNG BÌNH** | **Item load từ DB mất template fields** | `load_items` (`session.rs:176-199`) chỉ đọc `slot/itemid/quantity/damage`; `loai`, `lv`, mọi field `_1/_2` = 0. Hệ quả: gate equip `!(1..=6).contains(&loai)` (`inventory.rs:292`) **từ chối item đã lưu từ DB**; gear recompute = 0; starter 19737 cũng mất `loai:2/agi1:1` sau relog (tạo từ struct literal ở `character.rs:271-279`, không qua `from_template`). |
| 4 | **TRUNG BÌNH** | **Tail `05 03` lệch cả Bear lẫn spec client** | Rust: `F401×5` + **90** zero + skill list (`spawn.rs:182-184`). Bear: `500×5` + **43** zero + skill list + block 32 byte cuối (`hp3..spx3`, `hp4..spx4`) (`TSCharacter.cs:5072-5098`). Spec client: header cố định đến `p[0x70]`, record 3 byte từ `p[0x71]` (`opcode_05.md:131,151`). → skill list của Rust bắt đầu lệch ~47 byte so với chỗ parser decompile mong đợi, và block 32 byte tail bị thiếu. **Cần verify bằng capture thật.** |
| 5 | THẤP | Chưa validate end-to-end với `aLogin.exe` thật | `.scratch/op-working/handoff-login-handshake.md:100-102` còn open action này; test tự động duy nhất chỉ đếm 21 frame (`tests/login_char_flow_test.rs:232-236`). |
| 6 | THẤP | 32012 qty/slot không có nguồn đối chiếu | Bear không seed 32012; decompile không xác nhận qty/slot → chỉ internal consistency. |

### 5.3. Đề xuất hành động (theo thứ tự ưu tiên)

1. **Thêm push `08 01` vào cuối Logined1** — bộ tối thiểu theo `opcode_08.md:286-303`:
   class, level, hp, sp, int/atk/def/agi, exp, points; tham chiếu cách Bear làm ở
   `TSCharacter.cs:5380-5445` (`refreshChr` + `refresh(FullHpMax, 215)` / `(FullSpMax, 216)`).
2. **Gọi `recompute_stats()` ngay sau khi load session** (trong `login_db` `login.rs:169-235`
   hoặc trong `SqliteSessionRepository::load`) để `05 03` mang gear bonus đúng.
3. **Bổ sung `loai/lv/_1/_2` vào `load_items`** (`session.rs:176-199`) hoặc join với bảng template —
   fix luôn gate equip cho item đã lưu.
4. **Đối chiếu byte-exact tail `05 03`** với capture từ client thật; cân nhắc sửa
   `F401×5 + 90 zero` → layout spec (`opcode_05.md:131,151`) hoặc layout Bear.
5. **Chạy end-to-end với `aLogin.exe`** theo `handoff-login-handshake.md`.

---

## 6. Danh Sách UNDETERMINED (chưa xác định được)

1. **Parser client cho `17 05` / `17 0B` / `1E 01`** — `func_0x0076bfc8`, `func_0x0077cc2c`,
   `func_0x0077d0b8` **không có decompiled body** (`index.csv` grep `76bfc8|77cc2c|77d0b8` → 0 hit).
   → Entry 12 byte phía Rust là contract đã validate, nhưng decode phía client không xác minh được.
2. **Sub-body C→S của `0x17` (61 sub), `0x19` (≤0x29), `0x1C` (7 sub), `0x02` (8 sub)** —
   jump table tồn tại (`0x783405`, `0x7853E5`, `0x785DBB`, `0x77F883`) nhưng body không có
   trong ASM dump lẫn mọi file decompile.
3. **Hằng byte tại `0x78A834` (payload op `00`), `0x78A838..0x78A83E`, `0x78A848/854/880`** —
   không dump file nào phủ dải `0x78A8xx`, và không có `.exe` trong tree để re-dump.
4. **Xử lý qty/slot của 32012 phía client** + ý nghĩa mã trả về `id − 32016` của `FUN_006547a8`.
5. **Ý nghĩa field cuối của `08 01`** (`u32 @+0xFCF`, `byte @+0xFD3`) — server chỉ đọc 2 byte đầu.
6. **Client thật có bao giờ gửi `09 01` payload rỗng không** (server mặc định = enter-game, `character.rs:139-141`).

---

## 7. Sửa Lỗi Tài Liệu Đề Xuất (cho các file `.md` hiện có)

1. **`opcode_02.md:176-180,218` SAI**: khẳng định "client never sends 0x02; chat at 0x1A" dựa trên
   `case 2: break;`. Chứng cứ ngược: `byte[0x02] = 0x03` → builder `0x0077F86E` có bảng 8 sub
   (asm 178–182), **và chính ts_dream xử lý chat C→S tại op `0x02`** (`chat.rs:68-227`).
   Redirect "chat at 0x1A" cũng sai — `1A` subs 1–4 gửi **tiền** (`opcode_1a.md`).
2. **`opcode_3c.md:224-226`, `opcode_3d.md:24,557`**: index dword table bằng raw opcode là sai.
   Đúng: op `0x3C` → `dword[0x32] = 0x007892B9`; op `0x3D` → `dword[0x33] = 0x007893E6`;
   `0x0078A142` là builder của **op `0x48`**, `0x0078A318` là của **op `0xC7`**.
3. **Line-drift ≈ +44** với mọi citation vào `0077f414_FUN_0077f414.c`:
   `case 9` thực tế `942-943` (doc ghi 898–899), `case 2` thực tế `863-864` (ghi 819–820),
   `case 0x1a` thực tế `983`, `case 0x3d` thực tế `1088`.
4. **`opcode_08.md:267-283`**: câu "OP 0x08 KHÔNG có phía gửi" cần diễn đạt lại:
   builder C→S tồn tại và payload 10 byte verify khớp `stats.rs:32-44` — chỉ là không dùng để *query*.
5. Mọi claim "client never sends X" suy từ `case` rỗng là **chưa chứng minh** — với `byte[X] ≠ 0`
   thì `case X: break;` là lỗi recovery của Ghidra, không phải bằng chứng.

---

## 8. Bảng Mã Gọi Hàm Tóm Tắt (Execution Summary)

```
1.  Client ──► 01 [auth]                       (đã cover bởi login_flow_research.md)
2.  Server ──► danh sách nhân vật / 09 03
3.  Client ──► 09 02 + name                     character.rs:101
    Server ──► 09 03 ss
4.  Client ──► 09 01 + 14 phần (sex,hair,color,thuoctinh,int/atk/def/hpx/spx/agi,pass×2)
    Server ──► tạo DB nguyên tử: homdo 32012×4 @slot1, trangbi 19737 @slot2
                                                    character.rs:138 → session.rs:483-556
5.  Client ──► 03 01                            login.rs:88  (XÁC NHẬN VÀO GAME)
6.  Server ──► CHUỖI LOGINED1 (21 frame)        spawn.rs:471-604
    ├─ 03         id LE32 + name VISCII + map/xy + equip list
    ├─ 05 03      hp/sp/int/atk/def/agi/lv/exp + gear bonus   ◄── STATS
    ├─ 17 05      hòm đồ (entry 12B: slot,id,cnt,...)          ◄── INVENTORY
    ├─ 1E 01 / 17 2F / 17 66 / 17 0B / 1A 04 / 27 09 / 28 01 / 23 04×3
    └─ 02 0B ×2 banner, 14 08/14 21 marker, 21 02 PK, 0F xx pet
7.  Client ──► 25 01 (map loaded)               login.rs:238  → broadcast appear
8.  Về sau:  08 01 chỉ khi phân bổ điểm (C→S) / server push khi stat đổi (S→C)
             17 xx thao tác túi — KHÔNG có "xin lại 17 05"
```

---

*Tài liệu được tạo từ khảo sát decompile + source Rust ngày 2026-09-23. Mọi claim đều kèm file:line; các mục chưa xác định được liệt kê ở §6.*
