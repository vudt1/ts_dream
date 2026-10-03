# Luồng S→C: BẮT ĐẦU TRẬN CHIẾN + LOAD SCENE TRẬN CHIẾN

Ngày nghiên cứu: 2026-10-01 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Nguồn chính: `client_pseudo_c/` (pseudo-C + ASM). Đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`.
Tài liệu tham chiếu: `../opcode_0b.md`, `../opcode_32.md`, `../opcode_33.md`, `../opcode_34.md`,
`../opcode_35.md`, `../opcode_3f.md`, `../opcode_40.md`, `../post_login_stats_inventory_flow.md`.
Source Rust đối chiếu: `src/battle/packets.rs`, `src/battle/construction.rs`, `src/battle/service.rs`,
`src/battle/runner.rs`, `src/server/handlers/battle.rs`, `src/protocol/codecs/battle_role.rs`.

## Quy ước (áp dụng xuyên suốt tài liệu)

- `P[i]` = byte payload gốc sau khi tách header `F4 44` + `len LE16`. `P[0]` = MainOp.
- `RP[i]` = RestPayload = `P[i+1]`. `RP[0]` = SubOp. `Len` = độ dài `RP` (Delphi AnsiString length ở `[ptr-4]`).
- Ghi `RP[a..b]` = các byte `RP[a], RP[a+1], ..., RP[b]` (bao gồm cả hai đầu).
- Delphi `_LStrCopy(S, Index, Count, Dest)` — `Index` **1-based**, nên `_LStrCopy(RP, i, n)` đọc `RP[i-1 .. i+n-2]`.
- Guard `_BoundErr(n)` = yêu cầu `Len > n` (tức cần ít nhất `n+1` byte).
- `FUN_0077eb9c` = Word LE (2 byte). `FUN_0077ef7c` = DWORD LE (4 byte). `FUN_0077f098` = double (8 byte, `StrToFloat`).
- **Mức bằng chứng**: **(A)** đã xác minh từ client source (có body/ASM đọc byte) ·
  **(B)** suy luận có cơ sở từ body/ASM nhưng chưa có capture ·
  **(C)** chỉ là contract phía server Rust, **không** có client evidence.

> **Lưu ý về phương pháp (theo `post_login_stats_inventory_flow.md` §7)**: không dùng
> `case N: break;` rỗng của Ghidra làm bằng chứng "client không làm gì". Mỗi kết luận "SubOp
> không xử lý" dưới đây đều dựa trên **cả 2 nguồn**: (1) không có nhánh trong
> `switch` của `case_NNN`, **và** (2) đối chiếu `jumptable_0x78A9B6_cases.c` + dispatcher
> inline `0078a89c_FUN_0078a89c.c` để loại trừ lỗi recovery.

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — Gap #4 SAI. Xem §0.1**

---

## 0.1 🔴 ĐÍNH CHÍNH SAU ROUND-4 REDUMP

### 0.1.1 ❌ Gap #4 (`§639-642, §646, §706`) — **`FUN_0064DFA4` ĐÃ CÓ BODY**

> §639-642: *"14 handler `0x35` … chỉ `FUN_0064cfec` có body và **11 hàm còn lại chưa có body**"*
> §646: *"`FUN_0064dfa4` **không có body** trong `client_pseudo_c/`"*

**SAI.** Đối chiếu `ts_decompile/index.csv` (13 176 function): **cả 14 handler `0x35`
đều có `.c` + `.asm.txt`**, gồm:

```
0064cd74 0064cfec 0064d128 0064dfa4 0064e064 0064e3e0 0064e8f4
0064f9f0 0074789c 0075c518 0064faa8 0076a944 0064fbd8 0064fd34
```

Tài liệu còn **tự đếm sai**: bảng liệt kê 14 dòng nhưng lại viết "11 hàm còn lại"
(thực ra là 13 dòng còn lại sau `FUN_0064cfec`).

⇒ **`35 05` (`acting()`) layout `[row][col]` nay đọc được trực tiếp** từ
`0064dfa4_FUN_0064dfa4.asm.txt`. Gap #4 **đóng**; đây là việc đọc, không cần redump.

### 0.1.2 ✅ `LAB_00644A18` (Gap G11 của `battle/04`) đã có body

`00644a18_*.{c,asm.txt}` tồn tại.

### 0.1.3 ⭐ Tập sub `0x0B` — xác nhận đầy đủ

Round-4 đã dump bảng dispatch `0x780440` + builder `0x78048D..0x7807D0`.
Chi tiết ở `battle/03_skill_and_combo.md §0.1`.

Điểm liên quan `§706` (Gap #1, kind byte) và `battle/05:373`:

| Sub | Payload sau `[0B][sub]` | Tổng |
|:-:|---|--:|
| `0`,`1` | `[b:1]` | 2 B |
| **`2`** | `[b:1]`(`TFightManage+0x24`) + `u32LE`(`+0x1E`) + `u16LE`(`+0x22`) | **9 B** |
| `3`,`4` | `u32LE` + `u16LE` | 7 B |
| `5` | **KHÔNG GỬI** (no-op) | — |
| `6` | `[0B][06]` không body | 2 B |
| `7`,`8`,`9`,`0x0A`,`0xFA` | xem `battle/03 §1.1(b)` | |

### 0.1.4 ⚠️ Gap #9 (toast `0B 03`) — 4 literal đã dump hex

> §711: *"`DAT_00644C7C/AC/F0/14` không có hex dump trong `client_pseudo_c/`"*

✅ Đã dump: `lit_644cac.hex`, `lit_644cfc.hex`, `lit_644cf0.hex`, `lit_644d14.hex`
(`lit_644c7c.hex` đã có từ trước). Dùng `tools_decode_literals.py` để giải VISCII.

### 0.1.5 Còn lại — chưa đóng

| Gap | Nội dung | Trạng thái |
|:-:|---|---|
| #1 (CAO) | kind byte record `0B FA` | ❓ **vẫn cần capture** — round-trip không thay được traffic thật |
| #2, #6, #7, #8 | xem bảng gốc | ❓ chưa đóng |
| #5 | `33 01` vs `0B 08` | ❓ vẫn cần capture |
| #10 | `TFightField.Create` | ⚠️ **đã có body** — xem `battle/02 §0.1` |

## §0. Tổng quan timeline mở trận

```
┌────────────┐                          ┌──────────────┐
│  aLogin    │                          │   ts_dream   │
│ (client)   │                          │   (server)   │
└─────┬──────┘                          └──────┬───────┘
      │                                        │
      │  ── 0B 02 02/03 ... (PK / đánh NPC)   │  handlers/battle.rs:86-163
      │     [C→S]                             │
      │  ◄── 0B 05 + LE32(id) + LE16(0000)    │  hide_from_map  (packets.rs:70)
      │        + LE16(hpmax) + LE16(spmax)    │     "0B 00" + id + 00 00
      │        + LE16(hp)   + LE16(sp)        │
      │  ◄── 0B FA  <TERRAIN LE16>  ──────────┤  ⭐ FRAME MỞ TRẬN
      │        [mode][sub] + N × rec24         │  battle_open_leader / member_battle_frame
      │                                        │     construction.rs:342, 482
      │  ── 25 01 (map loaded, client xác nhận)│  (hoặc bỏ qua)
      │                                        │
      │  ◄── 0B 0A <mode=01>                   │  battle_trailer "0B0A01"
      │        (set TFightField.mode + timer)   │  packets.rs:151
      │                                        │
      │  ◄── 0B 05 <N × rec23>                 │  entity_npc/entity_player
      │        (nạp từng entity vào lưới)       │  packets.rs:40-47
      │  ◄── 0B 04 rec8 (set +0x376)           │  (show/hide từng ô)
      │  ◄── 0B 06 <id LE32> (effect code 0x76)│
      │  ◄── 0B 00 rec (reset trạng thái)       │
      │                                        │
      │  ── 34 01 ?  ─────────────────────────►│  your_turn()  packets.rs:85
      │                                        │
      │  ◄── 34 01 (BẮT ĐẦU LƯỢT ĐẤU)  ────────┤  ⭐ START TURN (không đọc payload)
      │  ◄── 32 01 rec (skill resolve + effect)  │  turn_action / runner.rs
      │  ◄── 33 01 rec7 (set cell property)      │
      │  ◄── 35 xx (hiệu ứng lượt)              │
```

**Điểm mấu chốt (mức A):** `0B FA` là frame **duy nhất** trong toàn bộ client gọi
`TFightField_Create` (`0x0063fadc`). Bằng chứng: grep trên `client_pseudo_c/` cho
`0063fadc` chỉ trả về `006438ac_FUN_006438ac.asm.txt:56`; grep `TFightField` trong tất cả
`case_*.c` chỉ trả về `case_011`. Vì vậy `0B FA` **là** lệnh "mở trận / load scene trận".

**Thứ tự trên wire — mức (B)**: Thứ tự `0B 0A` sau `0B FA` là **suy luận** từ contract Rust
(`construction.rs` / `service.rs:1087`), không phải thứ tự client bắt buộc. Client không có
state machine thể hiện thứ tự; nó xử lý từng frame độc lập. Điều duy nhất client *buộc* là
`0B FA` phải tới trước mọi `0B 05` / `0B 04` / `0B 06` / `32` / `33` / `34` khác, vì
`DAT_0098c63c` được tạo ra ở `0B FA` và **không có null-guard** ở `0B 0A`
(`0064e978_FUN_0064e978.c:44`) cũng như ở `32 01`/`33 01`.

---

## §1. Bảng tổng hợp tất cả frame liên quan

| # | Main | Sub | Tên | Layout byte (sau `[Main][Sub]`) | Mức | Bằng chứng |
|---|:---:|:---:|---|---|:-:|---|
| F1 | `0B` | `FA` | **MỞ TRẬN / load scene** | `[mode:1][sub:1]` + N×**rec24** | **A** | `006438ac_FUN_006438ac.c:94-114` + `.asm.txt:24-56` |
| F2 | `0B` | `05` | Nạp entity vào lưới | `[tag:1]` + N×**rec23** | **A** | `00644294_FUN_00644294.c:107-245` |
| F3 | `0B` | `04` | Set byte trạng thái `+0x376` | N×**rec8** | **A** | `00642244_FUN_00642244.c:55-155` |
| F4 | `0B` | `00` | Reset trạng thái actor | `[id:4 LE][kind:2 LE]` | **A** | `00641fa4_FUN_00641fa4.c:55-58` |
| F5 | `0B` | `01` | Dọn 1 ô lưới theo (row,col) | `[row:1][col:1]` | **A** | `0064bb24_FUN_0064bb24.c:50-62` |
| F6 | `0B` | `06` | Gán effect code cho actor | `[id:4 LE]` | **A** | `0072c9e8_FUN_0072c9e8.c:39-40` |
| F7 | `0B` | `03` | Toast 4 loại | `[code:1]` | **A** | `00644b84_FUN_00644b84.c:41-61` |
| F8 | `0B` | `0A` | Set mode + nhịp lượt của trường | `[mode:1]` | **A** | `0064e978_FUN_0064e978.c:37-54` |
| F9 | `0B` | `07` | Cờ + bộ đếm byte (saturating 0xFF) | `[flag:1][delta:1]` | **A** | `00733650_FUN_00733650.asm.txt:13-40` |
| F10 | `0B` | `09` | Set `self+0x1308` + toast | `[A:1][B:1]` | **A** | `case_011_0078D5D1_FUN_0078d5d1.c:82-107` |
| F11 | `0B` | `0B` | Lịch menu `+0x118/+0x120` | `[double:8]` | **A** | `005ba604_FUN_005ba604.c:36-40` |
| F12 | `0B` | `0C` | Deadline menu `+0x230` + toast | `[double:8]` | **A** | `0060e2a8_FUN_0060e2a8.c:41-49` |
| F13 | `32` | `01` | Resolve skill + phát effect | **biến thiên** (recLen) | **A** | `00647164_FUN_00647164.c:91-263` + `.asm.txt:41-159` |
| F14 | `32` | `02` | Reset lưới / item | **biến thiên** (count×8) | **A** | `0064dd74_FUN_0064dd74.c:67-147` |
| F15 | `33` | `01` | Set property của 1 ô lưới | N×**rec7** | **A** | `0064cae0_FUN_0064cae0.c:62-191` + `.asm.txt:19-89` |
| F16 | `34` | `01` | **BẮT ĐẦU LƯỢT ĐẤU** | **không đọc payload** | **A** | `case_045_00795266_FUN_00795266.c:27-29` + `00650a24_FUN_00650a24.c:36-127` |
| F17 | `35` | `01..0F` | Effect lượt (15 sub) | tuỳ sub | **A** (dispatch) / (B) (layout) | `case_046_007952AB_FUN_007952ab.c:27-70` |
| F18 | `35` | `05` | "Đang hành động" (Rust) | `[row][col]` | **C** | `packets.rs:87-91` — **không có client evidence** |

---

## §2. F1 — `0B FA` : FRAME MỞ TRẬN (chi tiết byte-for-byte)

### 2.1 Header (3 byte trước record)

| Offset | Len | Field | Bằng chứng |
|---:|---:|---|---|
| `RP[0]` | 1 | SubOp = `0xFA` (250) | `case_011_0078D5D1_FUN_0078d5d1.c:116-118` |
| `RP[1]` | 1 | `mode` | `006438ac_FUN_006438ac.c:99` (+ guard `:94-98`) |
| `RP[2]` | 1 | `sub` | `006438ac_FUN_006438ac.c:106` (+ guard `:102-105`) |

Guard: `_BoundErr(1)` ⇒ cần `Len ≥ 2`; `_BoundErr(2)` ⇒ cần `Len ≥ 3`. Với N=0 record,
payload hợp lệ tối thiểu là `FA 00 00` (`Len = 3`).

ASM xác nhận thứ tự đọc và cách truyền tham số cho constructor:

```
006438ac_FUN_006438ac.asm.txt:24-41   ; đọc RP[1] → [EBP-0x9], RP[2] → [EBP-0xa]
006438ac_FUN_006438ac.asm.txt:51-57
  MOV AL,byte ptr [EBP + -0xa]   ; sub
  PUSH EAX
  MOV CL,byte ptr [EBP + -0x9]   ; mode
  MOV DL,0x1                     ; hằng số 1
  MOV EAX,[0x0063cc84]           ; VMT_63CC84_TFightField
  CALL 0x0063fadc                ; TFightField_Create
  MOV [0x0098c63c],EAX
```

**Ý nghĩa (mức A về cấu trúc, B về ngữ nghĩa):** `0B FA` **tạo mới** object trường
chiến đấu và ghi đè con trỏ toàn cục `DAT_0098c63c`. Hằng số `1` là tham số cố định
(có thể là cờ "không phải spectator"), `mode`/`sub` là 2 tham số nghiệp vụ.
Từ `0064e978_FUN_0064e978.c:44` (`*(DAT_0098c63c+4) := mode`) suy ra **`mode` được ghi
vào field `+0x4` của `TFightField`** — cùng offset mà `0B 0A` dùng để đặt mode. Đây là
bằng chứng **(B)** mạnh rằng `RP[1]` của `0B FA` và `RP[1]` của `0B 0A` là **cùng một field**.

### 2.2 Record 24 byte — bảng byte-offset đầy đủ

Công thức: `N = (Len - 3) / 0x18` (`006438ac_FUN_006438ac.c:107-112`, ASM `:42-50`).
Record `i` bắt đầu tại `RP[3 + 24*i]`.

**Quy ước cột "Off" = offset 0-based tính từ **đầu record** (không phải từ `RP`).**

| Off | Len | Tên biến Ghidra | Kiểu | Ý nghĩa | Bằng chứng (`.c` dòng / ASM dòng) |
|---:|---:|---|---|---|---|
| +0 | 1 | `local_12` (`char`) | byte | **`kind`** — mã lớp/nghề của đơn vị. Với `type==2` (player) được ghi thẳng vào `PlayerRec+0x376` | `:138` / `:74-82` |
| +1 | 1 | `local_f` (`byte`) | byte | **`type`** — mã loại đơn vị; khóa của `switch` | `:149` / `:83-94` |
| +2 | 4 | `local_1c` | **DWORD LE** | **`id`** — ID định danh đơn vị | `_LStrCopy(...,4)` `:156` + `FUN_0077ef7c` `:157` / `:95-108` |
| +6 | 2 | `local_26` (`ushort`) | **WORD LE** | **`w`** — chỉ số phụ (dùng làm index bảng 101 phần tử khi `type==3`) | `:164-165` / `:109-122` |
| +8 | 4 | `local_20` (`int`) | **DWORD LE** | **`dw`** — dword phụ (mã sắc/xếp hạng/skill) | `:172-173` / `:123-136` |
| +12 | 1 | `local_10` | byte | **`bA`** — giá trị 1 (client so `!= 0xFF` trước khi dùng) | `:184` / `:137-148` |
| +13 | 1 | `local_11` | byte | **`bB`** — giá trị 2 (client so `!= 0xFF`) | `:195` / `:149-160` |
| +14 | 2 | `local_30` (`ushort`) | **WORD LE** | **`w0`** — word 1 của nhóm 4 | `:202-203` / `:161-174` |
| +16 | 2 | `local_2e` | **WORD LE** | **`w1`** | `:210-211` / `:175-188` |
| +18 | 2 | `local_2c` | **WORD LE** | **`w2`** | `:218-219` / `:189-202` |
| +20 | 2 | `local_2a` | **WORD LE** | **`w3`** | `:226-227` / `:203-216` |
| +22 | 1 | `local_28` | byte | **`bC`** — byte 1 | `:238` / `:217-228` |
| +23 | 1 | `local_27` | byte | **`bD`** — byte 2 | `:249` / `:229-240` |
| | **24** | | | **tổng** | `0x18` |

**Lưu ý về nhóm 4 word (`w0..w3`):** Ghidra khai báo chúng là 4 biến `ushort` **liên tiếp
trên stack** (`local_30`, `local_2e`, `local_2c`, `local_2a` — mỗi cách nhau đúng 2 byte),
và cả 4 được truyền cho callee dưới dạng **một con trỏ duy nhất** `&local_30`
(`:259`, `:289`, `:303`, `:319`, …). Vì vậy trên wire chúng là **4 số 16-bit liên tiếp,
không xen byte**. ASM xác nhận offset đọc lần lượt là `+0xe`, `+0x10`, `+0x12`, `+0x14`
tính từ `local_18` (`.asm.txt:161-216`), tức cách đều 2 byte.

### 2.3 `switch(type)` — phân loại 13 mã loại đơn vị

Nguồn: `006438ac_FUN_006438ac.c:250-391`. Cột "callee" là hàm được gọi để nạp đơn vị.

| `type` | Callee | Tham số đáng chú ý | Dòng |
|:--:|---|---|---|
| `0x02` | `FUN_006469b8` | nếu `id == selfId` → `PlayerRec+0x376 := kind`; nếu `kind∈{4,7}` → `self+0x1307=1`; nếu `kind==4` → `DAT_0098c640+0x25=1` + `FUN_0079b620(...,3)`; `gvar_007DA37C+0xC := 2` | `:251-307` |
| `0x03` | `FUN_006469b8` | index = `w` (guard `w ≤ 100`), bảng `gvar_007DA6DC` | `:308-321` |
| `0x04` | `FUN_00646a34` | `flag=1`, `param9=4` (cứng) | `:322-325` |
| `0x06` `0x0B` `0x0C` `0x0D` `0x0F` | `FUN_00646a34` | `flag=0`, `param9 = local_f` (giá trị thô) | `:326-333` |
| `0x07` | `FUN_00646a34` | `flag=0`, `param9=7` (cứng) | `:334-337` |
| `0x09` | `FUN_006469b8`/`FUN_00659358` | giống `0x02` nhưng **không** ghi cờ self | `:338-379` |
| `0x0E` | `FUN_00646a34` | `flag=1`, `param9=0x0E` (cứng) | `:380-383` |
| `0x10` | `FUN_00646a34` | `flag=0`, `param9 = local_f` | `:384-387` |
| `0x12` | `FUN_00646a34` | `flag=0`, `param9 = local_f` | `:388-391` |

**Đối chiếu với hằng `ehuman` của server Rust** (`src/protocol/codecs/battle_role.rs:14-42`):
bảng Rust đặt `PLAYERS=2`, `MAP_NPC=3`, `FOLLOW_NPC=4`, `GUARD_NPC=6`, `MINE_NPC=7`,
`DIVIDE=9`, `SOLDIER=11`, `MACHINE=12`, `CTRL_MACH=13`, `AUTO_FOLLOW=14`, `CTRL_SOLD=15`,
`FOREIGN_NPC=16`, `PET_NPC=19`. **Con trùng khớp 1-1** với 13 mã client rẽ nhánh
(2,3,4,6,7,9,0x0B,0x0C,0x0D,0x0E,0x0F,0x10,0x12 — với `0x0E`=AUTO_FOLLOW=14 ✓,
`0x10`=FOREIGN_NPC=16 ✓). Đây là bằng chứng **(B) mạnh** rằng trường `type` của record
`0B FA` chính là `ehuman`. Lưu ý: `0x09`=DIVIDE và `0x0E`=AUTO_FOLLOW rơi vào nhánh
"giống player" (dùng `FUN_006469b8` + tra `FUN_0070c20c`) — khớp với
`BattleRoleData::is_player_kind()` (`battle_role.rs:99-104`) xếp `DIVIDE` vào nhóm player.

### 2.4 Hậu kỳ `0B FA` (ảnh hưởng UI)

`006438ac_FUN_006438ac.c:396-402`:
- Nếu `PlayerRec+0x376 == 4` → `*(first_actor + 0x35C) = 0` và `gvar_007DA37C+0x1C = 1`.
- Ngược lại → `FUN_0058e76c(gvar_007DA530, 1)` (bật UI form chiến đấu).

### 2.5 Đối chiếu với `battle_open_leader()` — ⚠️ KHỚP BYTE

`src/battle/packets.rs:9-15`:
```rust
format!("F4441C000BFA{}03{}F44403000B0A01", le16(diahinh), leader_packet)
```
Sau `0B FA`: `le16(diahinh)` chiếm `RP[1..2]` (mode, sub) ✓ ; `03` là `RP[3]` = **byte `kind`
đầu record đầu tiên** ✓ khớp `+0`; `leader_packet` = `WarInfo::packet_hex()`
(`src/battle/engine.rs:71-88`) là **23 byte**.

Vậy: `RP[3]` = kind = `0x03`, còn 23 byte của `packet_hex()` phủi đúng `+1 .. +23` của record.
`packet_hex()` (`engine.rs:71-88`) là:
```
typ(1) + id(4) + id_npc_on_map(2) + id_char(4) + row,col(2) + hp_max(2) + sp_max(2)
+ hp(2) + sp(2) + lv(1) + thuoctinh(1)   = 23 byte
```
Ánh xạ sang record 24B của client:
| record off | client field | `packet_hex()` field |
|---:|---|---|
| +0 | `kind` | *(tiền tố `03` trong builder)* |
| +1 | `type` | `typ` |
| +2 | `id` (u32) | `id` |
| +6 | `w` (u16) | `id_npc_on_map` |
| +8 | `dw` (u32) | `id_char` |
| +12 | `bA` | `row` |
| +13 | `bB` | `col` |
| +14 | `w0` | `hp_max` |
| +16 | `w1` | `sp_max` |
| +18 | `w2` | `hp` |
| +20 | `w3` | `sp` |
| +22 | `bC` | `lv` |
| +23 | `bD` | `thuoctinh` |

**⇒ `WarInfo::packet_hex()` + tiền tố 1 byte = record 24 byte của client, khớp 1-1 cả 13
field. Đây là kết luận mức (A), không phải suy đoán.**

`record_join_cells()` (`src/battle/service.rs:1273-1281`) cũng viết **đúng 24 byte** với
cùng thứ tự (`marker, typ, id, id_npc, id_char, row, col, hp_max, sp_max, hp, sp, lv,
thuoctinh`) ✓.

**Vấn đề còn lại ở `record_join_cells`:** marker dùng `3` cho leader và `100` (`0x64`)
cho member (`service.rs:1263-1268`), và `build_join_frame` dùng `0402`
(`service.rs:1114`) cho bản thân người join. Ba giá trị `kind` khác nhau này **chưa được
xác minh là đúng** — xem §Khoảng trống #2.

---

## §3. F2 — `0B 05` : nạp entity vào lưới trận (record 23 byte)

### 3.1 Header

| Offset | Len | Field | Bằng chứng |
|---:|---:|---|---|
| `RP[0]` | 1 | SubOp = `0x05` | `case_011:70-72` |
| `RP[1]` | 1 | `tag` (`local_d`, `char`) — truyền cho callee dưới dạng con trỏ `&local_d` | `:115` / guard `:110-114` |
| `RP[2..]` | | records | |

Guard: `_BoundErr(1)` ⇒ `Len ≥ 2`. `N = (Len - 2) / 0x17` (`:116-121`).

### 3.2 Record 23 byte

`iVar4 = local_30 * 0x17`; base offset của record `i` = `RP[2 + 23*i]`.

| Off | Len | Biến | Kiểu | Bằng chứng (`.c` / `.asm.txt`) |
|---:|---:|---|---|---|
| +0 | 1 | `local_e` | byte — **giá trị `type`** (khóa `switch`) | `:145` / cần đọc ASM |
| +1 | 4 | `local_14` | **DWORD LE** — `id` | `_LStrCopy(...,4)` `:152` + `FUN_0077ef7c` `:153` |
| +5 | 2 | `local_22` | **WORD LE** — `w` (index bảng NPC 101 slot khi `type==3`) | `:160-161` |
| +7 | 4 | `local_18` | **DWORD LE** — `dw` | `:168-169` |
| +11 | 1 | `local_f` | byte — `bA` | `:180` |
| +12 | 1 | `local_10` | byte — `bB` | `:191` |
| +13 | 2 | `local_2c` | **WORD LE** — `w0` | `:198-199` |
| +15 | 2 | `local_2a` | **WORD LE** — `w1` | `:206-207` |
| +17 | 2 | `local_28` | **WORD LE** — `w2` | `:214-215` |
| +19 | 2 | `local_26` | **WORD LE** — `w3` | `:222-223` |
| +21 | 1 | `local_24` | byte — `bC` | `:234` |
| +22 | 1 | `local_23` | byte — `bD` | `:245` |
| | **23** | | | `0x17` |

### 3.3 ⚠️ QUAN TRỌNG: record 23B = record 24B của `0B FA` **mất đúng byte `kind`**

Đối chiếu trực tiếp hai bảng byte-offset:

| field | `0B FA` rec24 | `0B 05` rec23 |
|---|---|---|
| `kind` | +0 | **— (vắng)** |
| `type` | +1 | +0 |
| `id` (u32) | +2 | +1 |
| `w` (u16) | +6 | +5 |
| `dw` (u32) | +8 | +7 |
| `bA` | +12 | +11 |
| `bB` | +13 | +12 |
| `w0` | +14 | +13 |
| `w1` | +16 | +15 |
| `w2` | +18 | +17 |
| `w3` | +20 | +19 |
| `bC` | +22 | +21 |
| `bD` | +23 | +22 |

Mọi offset sau đều lệch đúng −1. **Kết luận mức (A):** record 23 byte của `0B 05` là
**record 24 byte của `0B FA` đã bỏ byte `kind` ở đầu**. Cùng cấu trúc, cùng 13 trường.

**Hệ quả trực tiếp cho server Rust (mức A, cần sửa):**
`entity_npc()` / `entity_player()` (`src/battle/packets.rs:40-47`) gửi
`F4441A000B0503` + `packet_hex()`:
```
RP[0]=05, RP[1]=03, RP[2..24] = packet_hex() (23 byte)
```
`RP[1]=0x03` là byte `tag` (header, **không thuộc record**). Vậy record bắt đầu đúng ở
`RP[2]` và 23 byte của `packet_hex()` phủi đúng `+0..+22` ✓ **khớp**.

Nhưng: `entity_npc` dùng tag `0x03` còn `entity_player` dùng tag `0x05` cho **cùng một
record 23 byte** (`packets.rs:41,46`), và `construction.rs` dùng `entity_player` cho cả
**member lẫn pet** (`construction.rs:357,367,380,392,405,516`). Vì `RP[1]` **không** nằm
trong record, client nhận **hai record giống hệt nhau** về `type`/`id`/`w`/... và chỉ khác
1 byte header mà vẫn parse cả hai. Điều này **đúng** với client (tag bị bỏ qua), nhưng
việc Rust dùng `entity_player` cho pet là **quyết định thiết kế của server**, không phải
bằng chứng client. Xem §Khoảng trống #3.

---

## §4. F3 — `0B 04` : set byte trạng thái `+0x376` (record 8 byte)

`N = ((Len - 1) + 7) >> 3` tức `N = (Len-1)/8` (`:55-63`). Record `i` tại `RP[1 + 8*i]`.

| Off | Len | Biến | Kiểu | Bằng chứng |
|---:|---:|---|---|---|
| +0 | 1 | `local_1d` (`char`) | byte — **`kind`** | `:87` |
| +1 | 4 | `local_14` | **DWORD LE** — `id` | `:94-95` |
| +5 | 2 | `local_1c` | **WORD LE** — `w` | `:102-104` (có `& 0xFFFF`) |
| +7 | 1 | `local_1e` | byte — **`val`** (giá trị ghi vào `+0x376`) | `:115` |

Rẽ theo `kind` (`:116-151`):
- `kind==2` → `idx = FUN_0070c20c(gvar_007D9D34, id)`; nếu tồn tại → `actor+0x376 := val`.
- `kind==6` → `w ≤ 200` → `*(gvar_007D9FB4[w])+0x376 := val`.
- `kind==0x10` → `w ≤ 10` → `*(gvar_007DA10C[w])+0x376 := val`.
- kind khác → record bị **bỏ qua nhưng vẫn tính 8 byte** (đã ở trong vòng `for`).

**Đối chiếu `show_on_map()`** (`src/battle/packets.rs:50-52`): frame
`F4440A000B0402` + `le32(id)` + tail. Payload = `0B 04 02 <id:4> 00 00 <tail>`.
Giải mã theo rec8 (record bắt đầu tại `RP[1]`): `kind=0x02` ✓, `id` = 4 byte LE ✓,
`w` = `00 00` ✓, `val` = byte đầu của tail.
`show_player_on_map` tail = `"000003"` ⇒ `val=0x03`; `show_member_on_map` tail = `"000005"`
⇒ `val=0x05`; `show_pk_leader_on_map` tail = `"000002"` ⇒ `val=0x02`.
⇒ **Rust dùng `kind=2` (player) + `val` làm mã vai trò. Khớp cấu trúc byte hoàn toàn (A).**
Giá trị ngữ nghĩa của `val ∈ {2,3,5}` chưa có client evidence (mức C).

`hide_from_map()` (`packets.rs:70-72`) dùng SubOp `0x00` (F4), xem §5.

---

## §5. F4 — `0B 00` : reset trạng thái actor

| Offset | Len | Field | Bằng chứng |
|---:|---:|---|---|
| `RP[1..4]` | 4 | **`id`** — DWORD LE | `_LStrCopy(RP,2,4)` `:55` + `FUN_0077ef7c` `:56` |
| `RP[5..6]` | 2 | **`kind`** — WORD LE | `_LStrCopy(RP,6,2)` `:57` + `FUN_0077eb9c` `:58` |

Guard thực tế: cần `Len ≥ 7` (đọc tới `RP[6]`). Logic `:59-125`:
- `kind==0 && id==selfId` → dọn form + `FUN_00595a70(InputBar,0)`; nếu `PlayerRec+0x376==4`
  thì `gvar_007DA37C+0xC := 3`, `PlayerRec+0x376 := 0`, và callback mới gán vào
  `gvar_007DA664+0x458` (`:69-76`).
- `kind==0 && id!=selfId` → `actor+0x376 := 0` (`:90`).
- `kind!=0` → `w ≤ 200`, nếu `*(gvar_007D9FB4[kind])+4 == id` → `+0x376 := 0` (`:112-124`).

**Đối chiếu `hide_from_map()`** (`packets.rs:70-72`): `F44408000B00` + `le32(id)` + `0000`.
Payload = `0B 00 <id:4> 00 00` ⇒ `RP[1..4]`=id ✓, `RP[5..6]`=`00 00` ⇒ `kind=0` ✓.
**Khớp byte hoàn toàn (A).**

---

## §6. F5 — `0B 01` : dọn một ô lưới theo (row, col)

| Offset | Len | Field | Guard | Bằng chứng |
|---:|---:|---|---|---|
| `RP[1]` | 1 | **`row`** | `_BoundErr(row)` nếu `> 3` | `:55`, `:65-67` |
| `RP[2]` | 1 | **`col`** | `_BoundErr(col)` nếu `> 4` | `:62`, `:74-77` |

Ràng buộc độc lập của 2 byte này (`row ≤ 3`, `col ≤ 4`) **xác nhận lưới trận là 4 hàng × 5
cột** — khớp `Battle::new()` (`construction.rs:60-71`) và hằng `0x23` (stride hàng = 35)
cùng `0x07` (stride cột = 7) trong `FUN_0065178c` (`test/ts_decompile/functions/0065178c_FUN_0065178c.c:54-88`).

**Đối chiếu Rust** (`packets.rs:75-82`):
- `reposition(row,col)` → `0B 01 <row> <col> 00` ✓
- `clear_pet_cell(row,col)` → `0B 01 <row^1> <col>` ✓

Cả hai dùng tọa độ đúng chiều (row trước, col sau). **Khớp (A).**

---

## §7. F6/F7/F8/F9/F10/F11/F12 — các SubOp còn lại của `0B 0B`

### F6 — `0B 06`
`[id:4 LE]` tại `RP[1..4]` — `_LStrCopy(RP,2,4)` + `FUN_0077ef7c` (`:39-40`).
Tra `FUN_0070c20c(gvar_007D9D34, id)`; nếu có actor → `FUN_0072a7a8(*(gvar_007DA300+idx*4), 0x76)`
(`:41-48`). `0x76 = 118` là mã hiệu ứng — **ngữ nghĩa chưa xác minh**.

### F7 — `0B 03`
`[code:1]` tại `RP[1]`, guard `Len ≥ 2` (`:41-49`). `code ∈ {1,2,3,4}` → toast 1000 ms
tương ứng `DAT_00644C7C / 00644CAC / 00644CFC / 00644D14` (`:50-61`). Chuỗi **chưa có
dump hex** trong `client_pseudo_c/` ⇒ không dịch được nội dung.

### F8 — `0B 0A` — set mode + nhịp lượt
`[mode:1]` tại `RP[1]`, guard `Len ≥ 2` (`:39-43`).
```
*(DAT_0098c63c + 4) = mode                          // :44
mode ∈ {2,4,7,8,9} → *(DAT_0098c63c + 0xE5A) = 0x1E (30)   // :46-48
mode == 0x0E        → *(DAT_0098c63c + 0xE5A) = 10          // :49-51
còn lại             → *(DAT_0098c63c + 0xE5A) = 0x14 (20)   // :52-54
```
**⚠️ KHÔNG có null-guard trên `DAT_0098c63c`** (khác F5) ⇒ gửi `0B 0A` trước `0B FA`
sẽ deref null. Đây là bằng chứng cứng (A) cho luận điểm thứ tự ở §0.

**Đối chiếu `battle_trailer()`** (`packets.rs:151-153`): `F44403000B0A01` ⇒ payload
`0B 0A 01`, `mode = 0x01` ⇒ nhịp lượt 20 giây. **Khớp byte (A).** Giá trị `mode=1` và
việc client gán 20 giây là hợp lý với "bắt đầu trận, mỗi lượt 20s".

### F9 — `0B 07` — cờ + bộ đếm saturating
Tầng case yêu cầu `Len ≥ 3` (`case_011:41-43` và `:46-49`).
ASM callee **xác nhận 3 tham số** (`00733650_FUN_00733650.asm.txt:13-14`):
```
MOV byte ptr [EBP + -0x6],CL     ; ← tham số 3 = RP[2]  (đây là bằng chứng A, không phải suy đoán)
MOV byte ptr [EBP + -0x5],DL     ; ← tham số 2 = RP[1]
MOV dword ptr [EBP + -0x4],EAX   ; ← Self
```
Đây là **bằng chứng (A)** chốt lại điều `../opcode_0b.md:134` mới chỉ đánh dấu "rất có thể".
Logic (`00733650_FUN_00733650.c:48-67` + ASM `:21-40`):
```
self+0x1305 = flag                              // RP[1]
t = self[0x1306] + delta ; if t < 0x100 → self[0x1306] += delta  (byte, CARRY check)
                                else → self[0x1306] = 0xFF        // :66
```
flag `1` hoặc `2` → toast `IntToStr(self[0x1306])` trong 1500 ms.

### F10 — `0B 09` — inline hoàn toàn
`self+0x1308 = RP[1]` (`:89`); nếu `RP[2] == 1` → toast `IntToStr(RP[1])` trong 2000 ms
(`:96-107`). Guard `Len ≥ 2` và `Len ≥ 3` (`case_011:85-95`).

### F11 — `0B 0B` — lịch menu
`[double:8]` tại `RP[1..8]` — `_LStrCopy(RP,2,8)` + `FUN_0077f098` (`:36-37`).
`obj+0x118 = value; obj+0x120 = Now() + value` (`:38-40`) trên `gvar_007D9D00`.
Cần `Len ≥ 9`.

### F12 — `0B 0C` — deadline menu
`[double:8]` tại `RP[1..8]` (`:41-43`); `obj+0x230 = value`. Nếu `Now() < value` →
toast `FormatDateTime("YYYY/MM/DD hh:mm")` tiền tố `DAT_0060E39C` trong 5000 ms (`:44-49`).

> **F11/F12 không liên quan trận đấu** — chúng là menu bán hàng / máy bán. Rust hiện
> **không** sinh hai frame này; đó là đúng.

---

## §8. F13/F14 — Opcode `0x32` : hai sub đọc **record biến thiên** trên lưới trận

Tầng case: `SubOp==1` → `FUN_00647164`; `SubOp==2` → `FUN_0064dd74`
(`case_043_007951DA_FUN_007951da.c:27-32`). SubOp khác ⇒ không có nhánh, rơi thẳng
epilogue — đã đối chiếu cả `jumptable_0x78A9B6_cases.c` lẫn dispatcher inline
`0078a89c_FUN_0078a89c.c:6684-6690` (xem `../opcode_32.md:83-99`).

### 8.1 F13 — `32 01` : resolve skill + phát effect (record biến thiên)

Khởi tạo trước vòng lặp (`:84-90`, ASM `:28-37`):
```
DAT_0098c63c[0xE5B] = 0xFF
DAT_0098c63c[0xE68] = 0xFF
DAT_0098c63c[0x06]  = 0x00
FUN_00651824(DAT_0098c63c)          // flush effect cũ của cả 21 slot
self+0x1C = 1                       // TFightManage flag
```
Vòng lặp: `pos = 2; while (pos < Len) { ...; pos += recLen + 2; }` (`:91-94`, ASM `:39-44`).

| Off (từ đầu record) | Len | Biến | Kiểu | Bằng chứng (`.asm.txt` dòng) |
|---:|---:|---|---|---|
| +0 | 2 | `local_24` | **WORD LE** — **`recLen`** | `:41-56` (`_LStrCopy(idx=pos,2)` → `pos-1, pos`) |
| +2 | 1 | `local_d` | byte — **`row`** (`_BoundErr` nếu `>3`) | `:57-68` (đọc `pos+1`) |
| +3 | 1 | `local_e` | byte — **`col`** (`_BoundErr` nếu `>4`) | `:69-80` (đọc `pos+2`) |
| +4 | 2 | `local_14` | **WORD LE** — **`skillId`** | `:81-94` |
| +6 | 1 | `local_12` | byte — `b0` (truyền `FUN_00648154` tham số 4) | `:95-106` (đọc `pos+5`) |
| +7 | 1 | `local_11` | byte — `b1` (truyền `FUN_00651e3c` tham số 3) | `:107-118` (đọc `pos+6`) |
| +8 | 1 | `local_f` | byte — **`row2`** (ô đích; `_BoundErr` nếu `>3`) | `:119-130` |
| +9 | 1 | `local_10` | byte — **`col2`** (ô đích; `_BoundErr` nếu `>4`) | `:131-142` |
| +10 | `recLen-6` | `local_2c` | **string VISCII** — **tên** | `:143-154` (`_LStrCopy(pos+8, recLen-6)`) |
| | **`recLen+2`** | | | tổng |

**Chứng minh tính nhất quán (mức A — số học khớp tuyệt đối):**
header cố định = 10 byte (`+0..+9`), tên = `recLen-6` byte ⇒ tổng = `10 + (recLen-6)` =
`recLen + 4`. Nhưng ASM `:155-159` cho `pos += recLen + 2`. **Chênh 2 byte.**

→ Vậy tên **không** nằm ở `+10`. Sửa lại: ASM `:143-154` dùng
`_LStrCopy(idx = pos+8, count = recLen-6)`, mà `_LStrCopy` là **1-based** ⇒ đọc
`RP[pos+7 .. pos+recLen]` (0-based). Với record bắt đầu tại `pos-1` (0-based), offset
tương đối = `(pos+7) - (pos-1)` = **+8**. Vậy:
```
header = +0..+7  (8 byte)  ; tên bắt đầu ở +8
tên    = +8 .. +(8 + (recLen-6) - 1) = +8 .. +recLen+1
tổng    = 8 + (recLen-6) = recLen+2  ✓  == pos advance  ✓
```
Vậy **bảng byte-offset đúng** của `32 01`:

| Off | Len | Field |
|---:|---:|---|
| +0 | 2 | `recLen` (WORD LE) = `6 + nameLen` |
| +2 | 1 | `row`  (0..3) |
| +3 | 1 | `col`  (0..4) |
| +4 | 2 | `skillId` (WORD LE) |
| +6 | 1 | `b0` |
| +7 | 1 | `b1` |
| +8 | `recLen-6` | **tên (VISCII, không length prefix)** |

Và `row2`/`col2` (đọc ở `RP[pos+7]`, `RP[pos+8]`) **trùng đúng 2 byte đầu của chuỗi tên**.
Đây là điểm client thực sự dùng: `.asm.txt:249-262` tra
`DAT_0098c63c[0x4C + row2*7*2 + col2*7*2]` — tức `row2`/`col2` là **ô đích**, đọc từ
2 byte đầu của tên. Vì tên VISCII của client TS thường bắt đầu bằng byte có giá trị nhỏ,
đây là cách client mã hoá ô đích trong chuỗi tên (hoặc tên được server gửi kèm tiền tố
byte). **Mức (B)** cho ngữ nghĩa; **mức (A)** cho vị trí byte.

Logic sau khi đọc (`.c:195-263`, ASM `:162-262`):
1. `code = FUN_0065178c(DAT_0098c63c, row, col)` (ô nguồn). Nếu `== -1` → bỏ qua record.
2. Nếu `skillId == 10000` → **remap tùy loại actor**: `actor.id ∈ {0xA4B0,0xA47B,0xA47C,0xA4BA,0xA4C8}`
   → `skillId = 0x3E82`; `actor.id ∈ {0x9090,0x9091}` → `skillId = 0x3E81`
   (`.c:202-215`, ASM `:178-206`).
3. `FUN_00651e3c(actor, skillId, b1, name)` (`.c:216`, ASM `:207-213`).
4. `FUN_00648154(self, codeAt(row,col), codeAt(row2,col2), b0, skillId)` (`.c:262`, ASM `:214-262`).
   `FUN_00648154` là **bộ dispatch effect theo skillId** (12018 byte,
   `test/ts_decompile/functions/00648154_FUN_00648154.c:19-100+`): `skillId 10000 → code 1`,
   `0x2711 → 0x0F`, `0x2712 → 0x16`, `0x2713 → 0x14`, `0x2714 → 0x15`, `0x2715 → 0x12`,
   `0x2B15 → 0x3F`, `0x32E0 → 0x85`; tất cả đều gọi `FUN_00666bf0(...)`.

### 8.2 F14 — `32 02` : reset lưới / item (record biến thiên theo count)

Tiền kỳ giống F13 (`:65-66`). Vòng lặp `:67-147`:
`pos = 2; while (pos < Len) { ...; pos += count*8 + 5; }`

| Off | Len | Field | Bằng chứng |
|---:|---:|---|---|
| +0 | 1 | `local_d` — **`row`** (`_BoundErr` nếu `>3`) | `:71-77` |
| +1 | 1 | `local_e` — **`col`** (`_BoundErr` nếu `>4`) | `:78-88` |
| +2 | 2 | `local_14` — **WORD LE** | `:95-96` |
| +4 | 1 | `local_11` — **`count`** | `:107` |
| +5 | `count*8` | dữ liệu thô | (tiêu thụ để tăng `pos`) |
| | **`count*8 + 5`** | | tổng |

**Chứng minh:** header 5 byte + `count*8` = `count*8+5` ✓ khớp `pos` advance (`:138-146`).

⚠️ Ghi chú: `.c:130-137` đọc `code = FUN_0065178c(...)` và `actor = ...` nhưng **các giá trị
đó không được dùng** — Ghidra ghi cảnh báo `WARNING: Removing unreachable block (ram,0x0064def5)`
(`:16`). Vì vậy **phần thân nghiệp vụ của `32 02` chưa xác minh được**; chỉ chắc chắn
layout byte (A) và tiền kỳ reset (A).

---

## §9. F15 — Opcode `0x33` SubOp `01` : set property ô lưới (record 7 byte)

Tầng case chỉ có **một** nhánh `SubOp==1` (`case_044_0079522C_FUN_0079522c.c:27-29`).
Đối chiếu `jumptable_0x78A9B6_cases.c` + dispatcher inline xác nhận không có nhánh nào khác.

`N = (Len - 1) / 7` (`.c:62-67`, ASM `:19-26`). Record `i` tại `RP[2 + 7*i]`.

| Off | Len | Biến | Kiểu | Bằng chứng (`.asm.txt`) |
|---:|---:|---|---|---|
| +0 | 1 | `local_d` | byte — **`row`** (`_BoundErr` nếu `>3`) | `:43-51` |
| +1 | 1 | `local_e` | byte — **`col`** (`_BoundErr` nếu `>4`) | `:52-63` |
| +2 | 1 | `local_f` | byte — **`propId`** (so `== 0x19`) | `:64-75` |
| +3 | 4 | `local_14` | **DWORD LE** — **`value`** | `:76-89` (`_LStrCopy` 4 byte + `FUN_0077ef7c`) |
| | **7** | | | |

Logic (`.c:122-188`):
1. `code = FUN_0065178c(DAT_0098c63c, row, col)`; nếu `== -1` → bỏ qua.
2. Nếu `propId == 0x19` (25) **và** `FUN_007108f4(actor, 0x19) <= 0` **và** `value > 0`
   ⇒ đây là **hồi sinh / heal**: `actor+0x391 = 0`, `actor.VMT[0x18](0x10)`,
   rồi `FUN_0066e710(..., 2, 3, 1, 0x70A3D70A, 0x3FD70A3D, 0, 0, 0x32, 1, 0x1E, x+0x58, x+0x54, 2, 0x17, 0x17, 1, 0x1E, 0x279D)`
   (`.c:129-181`). `propId 0x19` = **HP** (khớp hằng `troi_byte::HP = 0x19`,
   `src/battle/packets.rs:265`).
3. `FUN_00710ab0(actor, propId, value, 0)` (`.c:187`) — set property chung.

**Đối chiếu Rust**: `status_update()` (`packets.rs:140-148`) dùng MainOp `0x08`, **không**
phải `0x33`. Không có hàm Rust nào sinh frame `33 01`. Xem §Khoảng trống #5.

---

## §10. F16 — Opcode `0x34` SubOp `01` : BẮT ĐẦU LƯỢT ĐẤU

Tầng case (`case_045_00795266_FUN_00795266.c:27-29`):
```c
if ((*(int *)(unaff_EBP + -0x14) == 1) && (*(int *)gvar_007DA51C != 0)) {
  FUN_00650a24(*(uint *)gvar_007DA51C);
}
```
**Hai điểm cực quan trọng, cả hai đều mức (A):**

1. **`FUN_00650a24` KHÔNG nhận payload.** Signature: `void FUN_00650a24(uint param_1)`
   (`.c:5`) — một tham số duy nhất là con trỏ `TFightField`. Nó **không** đọc byte nào
   từ RestPayload. Frame `34 01` là frame **1 byte payload thuần** (`34 01`), không có
   trường nào.
2. **Có null-guard `gvar_007DA51C != 0`** — khác `0B 0A`. Đây là global khác `DAT_0098c63c`.

Nội dung `FUN_00650a24` (`.c:36-127`) — đây là **state machine lượt đấu**:
```
FUN_00651824(field)                                 // :36  flush effect 21 slot
field[0xE68] = field[0xE5A]                         // :37  copy nhịp lượt
field[0xE70] = Now()                                // :38-39
field[0x06]  = 0                                    // :40
for i in 0..=0x14:                                  // :42-59  dọc 21 slot
    actor[i]+0x569 = 0 ; actor[i]+0x56A = 0 ; actor[i]+0x552 = 0
field[0xE78] = 0                                    // :60  ← chỉ số người chơi đang lượt
if (PlayerRec[0x376] == 4)                          // :61  (đã thắng → kết thúc)
    FUN_0079b620(gvar_007DA0B4, DAT_0098c640[0x25] ? 3 : 6)   // :62-68
else
    dọn 3 form (FUN_0062c6f0 / 005b0640 / 0059c0ec)           // :70-72
    actor[field[0xE78]].hp@0x3EA == 0 → set 0x56A=1 ; FUN_00651094(field)  // :77-84
    if (field[0xE78] != -1):                                      // :85
        if ((actor[.0xDB] - 1) & 0xFF) < 100 → set 0x56A=1 ; FUN_00651094   // :90-97
        else FUN_00654898(field)                                   // :99
        if (field[0xE78] != -1):                                  // :101
            field[0xE5B] = field[0xE5A] ; field[0xE60] = Now()     // :102-104
            FUN_00658c30(field)                                   // :124  vẽ UI lượt
```
**Đối chiếu `your_turn()`** (`packets.rs:85-87`): `"F44402003401"` ⇒ payload `34 01`
(len = 2). **Khớp byte tuyệt đối (A)** — và đây chính là frame **khởi động lượt đấu đầu
tiên** sau khi scene đã load xong.

---

## §11. F17 — Opcode `0x35` : effect lượt (dispatch đã xác minh, layout chưa)

`case_046_007952AB_FUN_007952ab.c:27-70` xác nhận **15 sub**: `1,3,4,5,6,7,8,9,0xA,0xB,
0xC,0xD,0xE,0xF`. **Không có `0x00` và `0x02`.**

| Sub | Handler | Sub | Handler |
|:--:|---|:--:|---|
| 1 | `FUN_0064cd74` | 9 | `FUN_0064f9f0` |
| 3 | `FUN_0064cfec` | 0xA | `FUN_0074789c` (gvar_007D9D34) |
| 4 | `FUN_0064d128` | 0xB | `FUN_0075c518` (gvar_007DA250) |
| 5 | `FUN_0064dfa4` | 0xC | `FUN_0064faa8` |
| 6 | `FUN_0064e064` | 0xD | `FUN_0076a944` (gvar_007DA0D0) |
| 7 | `FUN_0064e3e0` | 0xE | `FUN_0064fbd8` |
| 8 | `FUN_0064e8f4` | 0xF | `FUN_0064fd34` |

**Mức (A)** cho bảng dispatch. Chỉ `FUN_0064cfec` (sub 3) có body trong `client_pseudo_c/`;
nó đọc `[row:1][col:1]` tại `RP[1]`, `RP[2]` với guard `≤3` / `≤4`
(`0064cfec_FUN_0064cfec.c:47-72`) rồi `FUN_00651824(DAT_0098c63c)` — cùng hạ tầng lưới.
11 hàm còn lại **chưa có body** ⇒ layout byte của chúng **không xác minh được**.

**Đối chiếu `acting(row,col)`** (`packets.rs:87-91`): `F44404003505` + `row` + `col`
⇒ payload `35 05 <row> <col>`, len 4. Rust dùng sub `05` ⇒ `FUN_0064dfa4`.
⚠️ **`FUN_0064dfa4` không có body trong `client_pseudo_c/`** ⇒ việc `05` nhận `[row][col]`
là **contract phía server (C)**, chưa có client evidence. Xem §Khoảng trống #4.

---

## §12. Đối chiếu tổng thể với server Rust

### 12.1 Những gì Rust ĐÃ ĐÚNG (khớp byte-for-byte với client)

| Hàm Rust | Frame | Khớp với | Mức |
|---|---|---|:-:|
| `WarInfo::packet_hex()` (`engine.rs:71-88`) | 23 byte | record của `0B 05` (mất byte `kind`) **và** record-24 của `0B FA` khi bỏ byte đầu | **A** |
| `battle_open_leader()` (`packets.rs:9-15`) | `0B FA <terrain LE16> 03 <rec>` | F1 §2.2 | **A** |
| `record_join_cells()` (`service.rs:1273-1281`) | 20 × 24 byte | F1 §2.2 | **A** |
| `entity_npc()` / `entity_player()` (`packets.rs:40-47`) | `0B 05 <tag> <rec23>` | F2 §3.2 | **A** |
| `show_on_map()` (`packets.rs:50-52`) | `0B 04 02 <id> 00 00 <val>` | F3 §4 rec8 | **A** |
| `hide_from_map()` (`packets.rs:70-72`) | `0B 00 <id> 00 00` | F4 §5 | **A** |
| `reposition()` / `clear_pet_cell()` (`packets.rs:75-82`) | `0B 01 <row> <col> 00` | F5 §6 | **A** |
| `battle_trailer()` (`packets.rs:151-153`) | `0B 0A 01` | F8 §7 | **A** |
| `your_turn()` (`packets.rs:85-87`) | `34 01` | F16 §10 | **A** |
| Lưới 4 hàng × 5 cột (`construction.rs:60-71`) | — | guard `≤3`/`≤4` + stride `0x23`/`0x07` | **A** |
| Bảng `ehuman` (`battle_role.rs:14-42`) | — | 13 mã `type` của `0B FA` | **B** |

### 12.2 `BattleRoleData` / `BattleRoleSerializer` (42 byte base) là **dead code SAI**

`src/protocol/codecs/battle_role.rs` mô tả base record **42 byte**
(`war_type 1 + human_kind 1 + role_id 8 + npc_id 2 + master_id 8 + col 1 + row 1 +
max_hp 4 + max_sp 4 + hp 4 + sp 4 + level 2 + upgrade_lv 1 + element 1` — xem
`battle_role.rs:112-128`), cộng phần appearance biến thiên cho player.

**Client KHÔNG có layout này ở bất kỳ frame mở trận nào.** Bằng chứng:
- `0B FA` record = 24 byte (§2.2), `0B 05` record = 23 byte (§3.2).
- Không frame nào trong `0B`/`0x32`/`0x33`/`0x34` đọc `u64` (`FUN_0077ef7c` chỉ đọc 4 byte).
- Doc comment của file (`battle_role.rs:3-6`) tự khai báo dựa trên
  `FightField.lua` / `BattleRoleSerializer.kt` — **nguồn tham chiếu từ TS Mobile / Lua,
  không phải client Delphi `aLogin.exe`**. Đó là nguồn sai lệch.

Thêm nữa: grep toàn repo cho `BattleRoleSerializer` / `BattleRoleData` chỉ ra **4 chỗ
định nghĩa và re-export**, **không có call site sản xuất frame nào**:
`src/protocol/codecs/mod.rs:14`, `src/protocol/mod.rs:397`,
`src/protocol/reader.rs:164`, `src/protocol/writer.rs:149`.
⇒ **Không có tác động runtime**, nhưng là **bẫy cho người implement tiếp** và comment
trong `battle_role.rs:6` ("Used for opcodes 0x0B (0x0BFA battle open, 0x0B05 role appear)
and 0x32") là **sai**.

### 12.3 `battle_role.rs` cũng không khớp `0x32`

Doc comment nói `BattleRoleSerializer` dùng cho `0x32`. Nhưng §8.1 cho thấy record
`32 01` là **biến thiên** (`recLen`-prefixed, có tên VISCII), và `32 02` là
`count`-prefixed. Cả hai đều **không** phải 42 byte cố định. ⇒ Doc sai (mức A).

---

## §13. Khoảng trống & mức độ tin cậy

| # | Mức | Vấn đề | Chi tiết |
|:-:|:--:|---|---|
| 1 | **CAO** | **Giá trị `kind` byte đầu record 24B chưa xác minh** | Client dùng `kind` cho: (a) `type==2` → ghi vào `PlayerRec+0x376` (`:262`); (b) `kind==4` → `self+0x1307=1`, `DAT_0098c640+0x25=1` (`:265-270`); (c) `kind∈{2,5}` + `PlayerRec[900]==';'` → `FUN_0072b390(self,8)` (`:253-256`); (d) `kind==4` ở hậu kỳ → `first_actor+0x35C=0` (`:396-399`). Rust dùng `03` / `64` / `0402` / `05` / `05` ở 5 chỗ khác nhau. **Chỉ có client evidence cho giá trị `4`** (và tập `{2,5}` khi điều kiện `';'` thỏa). Cần capture thật để chốt tập giá trị hợp lệ. |
| 2 | **CAO** | **Thứ tự frame sau `0B FA` là suy đoán** | Client không có state machine ràng buộc thứ tự (ngoại trừ `0B 0A` phải sau `0B FA`). Rust gửi `battle_open_leader` **kèm** `battle_trailer` trong cùng chuỗi (`packets.rs:11`), còn `member_battle_frame` (`construction.rs:482-491`) **không** gửi trailer. Sự bất đối xứng này có thể là bug. |
| 3 | TRUNG BÌNH | `entity_player()` dùng cho cả pet | `construction.rs:357,367,380,392,405,516` đều gọi `entity_player()` cho pet. Vì `RP[1]` (tag) bị client bỏ qua, wire vẫn parse được, nhưng **không có bằng chứng client** rằng pet nên mang `type` như player. Cân nhắc dùng `entity_npc()` cho pet. |
| 4 | TRUNG BÌNH | `35 05` (`acting()`) là contract server | `FUN_0064dfa4` **không có body** trong `client_pseudo_c/`. Layout `[row][col]` của `acting()` là **mức (C)**. |
| 5 | TRUNG BÌNH | Server không sinh frame `33 01` | `33 01` set property ô lưới (7 byte/record), đặc biệt `propId==0x19` kích hoạt **hồi sinh có effect** (`:129-181`). `status_update()` của Rust dùng `0x08` thay vì `0x33` (`packets.rs:140`). Nếu `0x08` không đủ, cần thêm `33 01`. **Không đủ bằng chứng để kết luận `0x08` sai** — cần capture. |
| 6 | TRUNG BÌNH | Thân nghiệp vụ của `32 02` chưa xác minh | Ghidra loại unreachable block (`:16`). Chỉ chắc layout byte + tiền kỳ `FUN_00651824`. |
| 7 | TRUNG BÌNH | `32 01`: `row2`/`col2` nằm trong 2 byte đầu tên | Vị trí byte chắc chắn (A); **ngữ nghĩa** (mức B) là client mã hoá ô đích bằng 2 byte đầu chuỗi tên VISCII. Nếu server Rust (`runner.rs`) gửi `3201` + text9 không kèm 2 byte tiền tố, ô đích sẽ bị đọc sai. **Cần kiểm tra lại `runner.rs:396,424,1059,1442`.** |
| 8 | THẤP | Ngữ nghĩa `type` mã 3 (`gvar_007DA6DC`, 101 slot) | Chỉ biết là bảng 101 phần tử tra bằng `w` (guard `≤100`). Chưa rõ là NPC template hay slot index. |
| 9 | THẤP | Chuỗi toast `0B 03` chưa dịch | `DAT_00644C7C/AC/F0/14` không có hex dump trong `client_pseudo_c/`. |
| 10 | THẤP | Ngữ nghĩa effect code `0x76` (`0B 06`) | `FUN_0072a7a8` chỉ ghi state + timestamp, không cho biết `0x76` là gì. |
| 11 | THẤP | `0x32 01` remap `skillId==10000` | Remap theo `actor.id` ∈ 6 giá trị cụ thể (`:202-215`). Rust không implement. Nếu server dùng `skillId=10000` cho một trong 6 NPC id đó, hiệu ứng sẽ sai. |
| 12 | THẤP | Chưa capture từ `aLogin.exe` thật | Mọi kết luận (A) ở trên là **byte-layout verification**, chưa phải **runtime verification**. Xem `handoff-login-handshake.md`. |

### 13.1 SubOp `0B` **không** được xử lý (đã loại trừ lỗi Ghidra)

`0x02` và `0x08` không có nhánh trong `switch` (`case_011:36-118`). Để không lặp lại lỗi
`post_login_stats_inventory_flow.md` §7, đã đối chiếu:
- `case_011_0078D5D1_FUN_0078d5d1.c:36-118` — switch 3 tầng `<8`, `<0x0C`, `==0x0C/0xFA`.
- `jumptable_0x78A9B6_cases.c` — không có target cho `0x02`/`0x08` của MainOp `0x0B`.
- Dispatcher inline `0078a89c_FUN_0078a89c.c` — khớp 1:1 với file case.

⇒ Kết luận mức (A): `0B 02` và `0B 08` phía client **không hề làm gì** khi nhận từ server.
Tuy nhiên chiều **C→S** thì `case 0xb` trong `0077f414_FUN_0077f414.c:902-910` **có builder**
(tham chiếu `../opcode_0b.md:180-186`) — không mâu thuẫn, vì đó là chiều ngược lại.

---

## §14. Đề xuất sửa server Rust

### 14.1 `src/protocol/codecs/battle_role.rs` — xoá hoặc đánh dấu DEAD + SAI (ưu tiên 1)

**Vấn đề:** mô tả record 42 byte không tồn tại ở client `aLogin.exe`. Nguồn tham chiếu là
`FightField.lua` / `BattleRoleSerializer.kt` (TS Mobile), không phải client Delphi.
Doc `battle_role.rs:6` khẳng định "Used for opcodes `0x0B` (`0x0BFA` battle open, `0x0B05`
role appear) and `0x32`" — **sai hoàn toàn** theo §12.2/§12.3.

**Hành động đề xuất:**
1. Sửa doc `battle_role.rs:1-6` để ghi rõ: layout này **lấy từ TS Mobile Kotlin, KHÔNG
   phải từ client PC**, và **không dùng cho `0B`/`0x32` của protocol PC**.
2. Thêm `#[deprecated]` hoặc `#[allow(dead_code)]` + comment "chưa dùng, chưa xác minh với
   client PC" để không ai vô tình dùng khi implement `0B FA`.
3. Hoặc tốt hơn: thay thế bằng định nghĩa record đúng 24 byte đã xác minh ở §2.2.

### 14.2 Thêm codec record 24 byte / 23 byte đã xác minh (ưu tiên 2)

**Vấn đề:** `WarInfo::packet_hex()` (`engine.rs:71-88`) đúng byte cho **cả hai** dạng nhưng
đang phụ thuộc vào tiền tố byte rải rác ở 4 nơi khác nhau (`packets.rs:11` `03`;
`service.rs:1114` `0402`; `construction.rs:444` `05`; `construction.rs:449` `03`;
`construction.rs:470` `64`). Dễ lệch.

**Đề xuất cụ thể** — thêm vào `src/battle/packets.rs`:
```rust
/// Record 24 byte của `0B FA` — layout đã xác minh từ client
/// (`client_pseudo_c/006438ac_FUN_006438ac.c:120-249`).
/// Off: 0 kind | 1 type | 2 id(u32) | 6 w(u16) | 8 dw(u32) | 12 row | 13 col
///      | 14 hp_max(u16) | 16 sp_max(u16) | 18 hp(u16) | 20 sp(u16) | 22 lv | 23 tt
pub fn battle_open_record_24(
    kind: u8, typ: u8, id: u32, w: u16, dw: u32,
    row: u8, col: u8,
    hp_max: u16, sp_max: u16, hp: u16, sp: u16, lv: u8, tt: u8,
) -> String;

/// Record 23 byte của `0B 05` — CÙNG layout 24B nhưng KHÔNG có byte `kind`
/// (`client_pseudo_c/00644294_FUN_00644294.c:130-245`).
pub fn entity_record_23(
    typ: u8, id: u32, w: u16, dw: u32,
    row: u8, col: u8,
    hp_max: u16, sp_max: u16, hp: u16, sp: u16, lv: u8, tt: u8,
) -> String;
```
Rồi cho `battle_open_leader()` / `entity_*()` / `record_join_cells()` gọi chung. Điều này
làm rõ 1-1 với client và ngăn lệch byte trong tương lai.

### 14.3 `member_battle_frame()` thiếu `battle_trailer()` (ưu tiên 3)

**Vấn đề:** `construction.rs:482-491` dựng frame `0B FA` **không** kèm
`F44403000B0A01`, trong khi `battle_open_leader()` (`packets.rs:9-15`) **có**.
Hai đường vào cùng một trận gửi payload khác nhau.

**Bằng chứng phía client (A):** `0B 0A` set `DAT_0098c63c+4 = mode` và `+0xE5A = 20`
(`0064e978_FUN_0064e978.c:44-54`) — tức **bắt buộc** phải có ai đó set mode/nhịp lượt
trước khi `34 01` chạy (`00650a24_FUN_00650a24.c:37` copy `0xE5A → 0xE68`). Nếu member
không nhận `0B 0A`, `field[0xE5A]` của họ vẫn là giá trị mặc định.

**Đề xuất:** thêm `battle_trailer()` vào cuối danh sách frame của
`member_battle_frame()` (`construction.rs:522`), hoặc tốt hơn — bảo đảm mọi frame `0B FA`
đều được `spawn_battle` gửi kèm trailer một lần duy nhất (`service.rs`).

### 14.4 `build_join_frame()` dài 22 byte thay vì 24 (ưu tiên 4 — CẦN XÁC MINH)

**Vấn đề:** `service.rs:1108-1120` dựng
```
"0402" + le32(id) + "000000000000FFFF" + le16(hp_max) + le16(sp_max)
       + le16(hp) + le16(sp) + level + thuoctinh
```
Đếm: 2 + 4 + 6 + 2+2+2+2 + 1 + 1 = **22 byte**. Nhưng record 24 byte của client yêu cầu
`kind(1) + type(1) + id(4) + w(2) + dw(4) + row(1) + col(1) + 4×u16 + lv(1) + tt(1)` = 24.
⇒ **Thiếu 2 byte `row`/`col`**, hoặc `id_char`(4) bị thay bằng `"FFFF"`.

Đối chiếu chuỗi `"000000000000FFFF"` (6 byte): có thể là `w`(2 byte = `0000`) + `dw`
(4 byte = `0000FFFF`)? Nhưng `id_char` = `0x0000FFFF` = 65535 là giá trị lạ.
Hoặc 6 byte = `id_char`(4=`00000000`) + `row`(1) + `col`(1) + thừa 1? — **không khớp**.

**Đây là mismatch có khả năng cao làm hỏng frame join.** Đề xuất: viết lại bằng
`battle_open_record_24()` từ §14.2 với đầy đủ `row`/`col` từ `session`, và `dw = 0`.

### 14.5 Marker `kind` không nhất quán (ưu tiên 5)

Rust dùng `kind`: `03` (`packets.rs:11` leader, `service.rs:1265` leader),
`64`/`100` (`service.rs:1268` member, `construction.rs:470`),
`0402` (`service.rs:1114` join — chú ý đây là **2 byte** `04`+`02`, tức `kind=0x04`,
`type=0x02`), `05` (`construction.rs:444` self), `03` (`construction.rs:449`).

Client evidence duy nhất: `kind==4` vào nhánh riêng (`:267-270`, `:396-399`),
`kind∈{2,5}` + `PlayerRec[900]==';'` (`:253-256`).
⇒ `kind=0x64 (100)` **không có cơ sở**. Đề xuất: tạm thời dùng `4` cho
"người chơi chính" và `5` cho "member", rồi **capture `aLogin.exe` thật** để chốt tập giá trị.

### 14.6 `32 01`: kiểm tra 2 byte đầu chuỗi tên (ưu tiên 6)

**Vấn đề:** theo §8.1, `row2`/`col2` mà client dùng để tra ô đích nằm ở
`RP[pos+7]`, `RP[pos+8]` — tức **2 byte đầu của chuỗi tên**.
Rust `turn_action(text9)` (`packets.rs:95-99`) gửi `3201` + `text9`, và
`runner.rs:396,424,1059,1442` dựng `3201` + text. Nếu `text9` **không** bắt đầu bằng
2 byte chứa `(row2, col2)`, client sẽ tra ô đích sai.

**Đề xuất hành động:** đọc kỹ `runner.rs:380-430` và `1440-1450`, đối chiếu xem `text9`
có được tiền tố hóa bằng `(row2, col2)` không. Nếu không → cần sửa builder `3201`.
Đây là **mức (B)** cho ngữ nghĩa, **(A)** cho vị trí byte — cần đọc code runtime để chốt.

### 14.7 Cân nhắc thêm `33 01` cho hồi sinh (ưu tiên 7, cần capture)

`33 01` với `propId=0x19` + `value>0` kích hoạt chuỗi effect hồi sinh
(`0064cae0_FUN_0064cae0.c:129-181`) — thứ mà `status_update()` (`0x08`) không có.
Đề xuất: capture hồi sinh trong trận thật; nếu client không phản ứng với `0B 08` mà phản
ứng với `33 01`, thì thêm builder mới. **Hiện chưa đủ bằng chứng để sửa.**

### 14.8 Sửa doc `src/battle/engine.rs:68-70`

Comment hiện tại:
> `/// The _Packet 23-byte snapshot (ChangedWar, §6.1)`

Nên bổ sung: "**Đây chính là record 23 byte của `0B 05` (`FUN_00644294`) và là record 24
byte của `0B FA` (`FUN_006438ac`) khi bỏ byte `kind` ở đầu. Đã xác minh từ client ASM.**"
— kèm đường dẫn `client_pseudo_c/006438ac_FUN_006438ac.asm.txt:74-240` để người sau tự
kiểm chứng lại.

---

## §15. Source trail

| # | Nguồn | Dòng | Dùng để |
|---|---|---|---|
| 1 | `client_pseudo_c/case_011_0078D5D1_FUN_0078d5d1.c` | 28-118 | Dispatch `0B`, danh sách SubOp, guard |
| 2 | `client_pseudo_c/006438ac_FUN_006438ac.c` + `.asm.txt` | `.c:94-114`, `.asm.txt:24-56` | Header `0B FA` + `TFightField_Create` |
| 3 | `client_pseudo_c/006438ac_FUN_006438ac.c` + `.asm.txt` | `.c:120-249`, `.asm.txt:74-240` | **Record 24 byte** |
| 4 | `client_pseudo_c/006438ac_FUN_006438ac.c` | 250-391 | `switch(type)` 13 nhánh |
| 5 | `client_pseudo_c/006438ac_FUN_006438ac.c` | 396-402 | Hậu kỳ UI |
| 6 | `client_pseudo_c/00644294_FUN_00644294.c` | 107-245 | Record 23 byte `0B 05` |
| 7 | `client_pseudo_c/00642244_FUN_00642244.c` | 55-155 | Record 8 byte `0B 04` |
| 8 | `client_pseudo_c/00641fa4_FUN_00641fa4.c` | 55-125 | `0B 00` layout + logic |
| 9 | `client_pseudo_c/0064bb24_FUN_0064bb24.c` | 50-181 | `0B 01` (row,col) |
| 10 | `client_pseudo_c/0072c9e8_FUN_0072c9e8.c` | 39-48 | `0B 06` |
| 11 | `client_pseudo_c/00644b84_FUN_00644b84.c` | 41-61 | `0B 03` |
| 12 | `client_pseudo_c/0064e978_FUN_0064e978.c` | 37-54 | `0B 0A` mode + nhịp |
| 13 | `client_pseudo_c/00733650_FUN_00733650.asm.txt` | 13-40 | `0B 07` 3 tham số (bằng chứng A) |
| 14 | `client_pseudo_c/case_011_0078D5D1_FUN_0078d5d1.c` | 82-107 | `0B 09` inline |
| 15 | `client_pseudo_c/005ba604_FUN_005ba604.c` | 36-40 | `0B 0B` double |
| 16 | `client_pseudo_c/0060e2a8_FUN_0060e2a8.c` | 41-49 | `0B 0C` double |
| 17 | `client_pseudo_c/case_043_007951DA_FUN_007951da.c` | 27-32 | Dispatch `0x32` |
| 18 | `client_pseudo_c/00647164_FUN_00647164.c` + `.asm.txt` | `.c:84-263`, `.asm.txt:28-262` | `32 01` record biến thiên |
| 19 | `client_pseudo_c/0064dd74_FUN_0064dd74.c` | 16-147 | `32 02` record `count*8+5` |
| 20 | `client_pseudo_c/case_044_0079522C_FUN_0079522c.c` | 27-29 | Dispatch `0x33` (chỉ sub 1) |
| 21 | `client_pseudo_c/0064cae0_FUN_0064cae0.c` + `.asm.txt` | `.c:62-191`, `.asm.txt:19-89` | `33 01` record 7 byte |
| 22 | `client_pseudo_c/case_045_00795266_FUN_00795266.c` | 27-29 | Dispatch `0x34` (chỉ sub 1) |
| 23 | `client_pseudo_c/00650a24_FUN_00650a24.c` | 5, 36-127 | `34 01` state machine lượt đấu |
| 24 | `client_pseudo_c/case_046_007952AB_FUN_007952ab.c` | 27-70 | Dispatch `0x35` 15 sub |
| 25 | `client_pseudo_c/0064cfec_FUN_0064cfec.c` | 47-72 | `35 03` (row,col) |
| 26 | `test/ts_decompile/functions/0065178c_FUN_0065178c.c` | 38-91 | Xác nhận lưới 4×5, stride `0x23`/`0x07` |
| 27 | `test/ts_decompile/functions/00648154_FUN_00648154.c` | 19-100+ | Dispatch effect theo `skillId` |
| 28 | `test/ts_decompile/functions/00651824_FUN_00651824.c` | 21-37 | Flush effect 21 slot |
| 29 | `src/battle/packets.rs` | 9-153 | Đối chiếu builder |
| 30 | `src/battle/engine.rs` | 68-88 | `packet_hex()` |
| 31 | `src/battle/construction.rs` | 60-71, 327-524 | Lưới + frame ordering |
| 32 | `src/battle/service.rs` | 1098-1120, 1253-1292 | `build_join_frame`, `record_join_cells` |
| 33 | `src/protocol/codecs/battle_role.rs` | 1-135, 272-307 | Dead code 42B sai |
| 34 | `src/server/handlers/battle.rs` | 86-163 | Handler C→S |

---

*Tài liệu soạn 2026-10-01. Mọi layout byte đều kèm `file:dòng`. Phân biệt rõ (A) xác minh từ
client source, (B) suy luận có cơ sở, (C) chỉ là contract phía server Rust. Không dùng
`case N: break;` rỗng làm bằng chứng — mọi kết luận "không xử lý" đã đối chiếu jumptable +
dispatcher inline. Nguồn chính `client_pseudo_c/`; `test/ts_decompile/` chỉ dùng để đối chiếu.*