# 04 — KẾT THÚC TRẬN CHIẾN & PHẦN THƯỞNG (EXP / ITEM / VÀNG)

Ngày: 2026-10-01 · Workspace: `/mnt/d/VUDT/GIT_PCC/ts_dream`
Nguồn chính: `client_pseudo_c/` (Ghidra pseudo-C + ASM của `aLogin.exe`)
Đối chiếu: `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`
Tài liệu tham chiếu: `../post_login_stats_inventory_flow.md`, `../opcode_0b.md`, `../opcode_05.md`,
`../opcode_08.md`, `../opcode_17.md`, `../opcode_1a.md`, `../opcode_32.md`, `../opcode_33.md`,
`../opcode_34.md`, `../opcode_35.md`, `../opcode_1e.md`
Source Rust: `src/battle/{service,runner,packets,manager,engine,construction}.rs`,
`src/server/handlers/battle.rs`, `src/server/spawn.rs`, `src/data/texps.rs`, `src/server/session.rs`

> **Mức bằng chứng dùng trong tài liệu**
> **(A)** xác minh trực tiếp từ body/ASM client (có `file:line`).
> **(B)** suy luận có cơ sở từ body client + tương quan byte/offset.
> **(C)** chỉ là contract phía server Rust (chưa có bằng chứng client).
>
> **Cấm dùng `case N: break;` rỗng của Ghidra làm bằng chứng** (xem `../post_login_…md` §7).
> Mọi claim "client không gửi X" dưới đây đều đã đối chiếu bảng target C→S
> `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex` + `cs_builder_dword200_0x77f53c.hex`.

---

> **🔴 CẬP NHẬT 2026-10-02 (Round-4 redump) — G4 (`17 06`) ĐÃ ĐÓNG. Xem §0.1**

---

## 0.1 🔴 ROUND-4 REDUMP — `17 06` ĐÃ CÓ BODY, LAYOUT BYTE-EXACT

### 0.1.1 G4 đóng — `func_0x0076B458` nay có `.c` + `.asm.txt`

> §2.6 / §G4: *"`func_0x0076B458` **KHÔNG có file** trong `client_pseudo_c/`"*
> *"cần redump `0x0076b458` để chốt byte-for-byte"*

**ĐÃ ĐÓNG.** `ts_decompile/functions/0076b458_FUN_0076b458.{c,asm.txt}` tồn tại
(HOLE `0x0076AF6A..0x0076B84C` = 2 274 B trước đây).

### 0.1.2 ⭐ Wire layout `17 06` — byte-exact

Dispatcher `case_020_007902FB_FUN_007902fb.c:50-51`:
```c
case 6: func_0x0076B458(*(undefined4 *)gvar_007DA0D0, *(undefined4 *)(unaff_EBP + -0xc)); break;
```
⇒ `RP` (chuỗi payload) **bắt đầu tại byte sub** ⇒ `RP[0] == 0x06`.

Đọc tay ASM (`0076b458_FUN_0076b458.asm.txt`), quy ước `_LStrCopy(dst, k, n, src)` =
Delphi 1-based ⇒ đọc `RP[k .. k+n-1]`:

| ASM offset | Đọc | Giá trị |
|---|---|---|
| `0x76b4b2` `MOV ECX,0x2` / `0x76b4b7` `MOV EDX,0x2` → `_LStrCopy(_,2,2,_)` | `RP[2..3]` → `FUN_0077EB9C` | **`u16 LE` → `uVar6`** |
| `0x76b4b9` `MOV ECX,0x2` (offset 4) | `RP[4..5]` → `FUN_0077EB9C` | `u16 LE` → `uVar7` |
| `0x76b4d7` `MOV EAX,0x6` + bound check | `RP[5]` | `bVar1` |
| `0x76b4f2` `MOV EAX,0x7` | `RP[6]` | `bVar3` |
| `0x76b50d` `MOV EAX,0x8` | `RP[7]` | byte0 của `local_1c` |
| `0x76b528` `MOV EAX,0x9` | `RP[8]` | byte3 — **bị `CONCAT31` ghi đè, không dùng** |
| `0x76b547` `MOV ECX,0x4` / `0x76b54c` `MOV EDX,0xa` | `RP[10..13]` → `FUN_0077ED68` | **`u32 LE` → `local_20`** |

Sau đó `FUN_0065B580(*gvar_007DA660, uVar6, 0, local_20)` tra 1 byte (`CMP EAX,0xFF; _BoundErr`)
rồi `CONCAT31` thay byte3, và:
```c
FUN_0072B6C8(*gvar_007DA7BC, uVar6, uVar7, local_20, uVar9, bVar3, bVar4, bVar5, bVar1);
```

⇒ **Layout wire (`RP[0..13]`, tối thiểu 14 byte):**

```
RP[0]      = 0x06                      sub
RP[1]      = ?                         KHONG doc
RP[2..3]   = u16 LE                    item id  -> uVar6
RP[4..5]   = u16 LE                    -> uVar7
RP[5]      = u8                        -> bVar1 (trùng high byte của uVar7)
RP[6]      = u8                        -> bVar3
RP[7]      = u8                        -> byte0 local_1c
RP[8]      = u8                        doc roi, BI ghi de  (nen co the = 0)
RP[9]      = ?                         KHONG doc
RP[10..13] = u32 LE                    texp
```

Frame đầy đủ = `F4 44 10 00 17 06 …` (payload **16 B**).

### 0.1.3 ✅ `17 05` — record stride **12 byte** (đối chiếu)

Handler `17 05` = `FUN_0076BFC8` (đã có sẵn body). ASM:

```asm
0076c035: MOV ECX,0xc / CDQ / IDIV ECX     ; <- STRIDE = 12, count = (len-1)/12
...
0076c0a2: MOV ECX,0x2  + base+1            ; u16 LE -> id
0076c0b9: CALL 0x0077eb9c
0076c182: MOV ECX,0x4  + base+8            ; u32 LE -> texp
0076c199: CALL 0x0077ed68
0076c1b2: CALL 0x0065b580                  ; cung tra byte, guard <= 0xFF
```

⇒ Record `17 05` = **`[slot:1][id:2][×5 byte][texp:4]` = 12 B**, và
**`17 06` = byte `0x06` + chính record `17 05` đó** (`RP[1..13]`).

> ⚠️ **Rust đang lệch.** `inventory.rs:69-84 item_added_frame` ghi
> `"1706" + le16(id) + [count, doben, long, giatri+100, khang, le32(texp)]` = **11 B**,
> đặt `id` ngay tại `RP[0..1]`. Client đọc `id` tại **`RP[2..3]`** và cần **≥14 B**.
> ⇒ **Lệch 1 byte trên mọi field + thiếu byte `RP[1]` (slot) + thiếu `RP[9]`.**
>
> Cần một test round-trip: build `17 06` 14 byte theo layout §0.1.2 rồi đối chiếu
> `FUN_0072B6C8` trong client.

### 0.1.4 G11 đóng — `LAB_00644A18` nay có body

`ts_decompile/functions/00644a18_*.{c,asm.txt}` tồn tại (HOLE `0x00644A16..0x00644B20`
= 266 B trước đây).

### 0.1.5 ⚠️ `FUN_0064DFA4` (`35 05` `acting()`) — ĐÃ CÓ BODY (ngược với `battle/01`)

`battle/01_battle_start_scene.md:646` và `:706` (Gap #4) ghi
*"`FUN_0064DFA4` không có body trong `client_pseudo_c/`"*.
**SAI** — `0064dfa4_FUN_0064dfa4.{c,asm.txt}` đã tồn tại (và `battle/02:565` đã trích dẫn nó).

⇒ **`35 05` layout `[row][col]` nay đã có cơ sở đọc trực tiếp** — xem
`0064dfa4_FUN_0064dfa4.asm.txt`. Đây là việc đọc, không cần redump.

## 0. TIMELINE KẾT THÚC TRẬN (ASCII client ↔ server)

```
                 ┌──────────────┐                                ┌──────────────┐
                 │ aLogin.exe   │                                │ ts_dream     │
                 └──────┬───────┘                                └──────┬───────┘
                        │                                               │
  (trong trận)         │◄── 34 01                       "your turn" ────┤
                        │    (FUN_00650a24: reset 21 slot + turn engine) │
                        │◄── 32 01 / 35 xx / 35 04 / 08 01 (hp/sp trong lượt)
                        │◄── 08 01 19/1A … (F4440C000801 …)  stat delta  │
                        │                                               │
                        │            … hết vòng cuối …                  │
                        │                                               │
                        │◄── [A] 08 01 24 <sign> <texp LE32> 00000000  │  apply_db(Stat::Texp)
                        │        (FUN_00710ab0 case 0x24 → actor+0x3FC)  │
                        │        nếu lên cấp:                           │
                        │◄── [B] 08 01 23 00 <lv LE32> 00000000         │  level byte → +0x3FA
                        │        (case 0x23 → +0x3FA + TỰ tính lại MaxHP/MaxSP)
                        │◄── [C] (KHÔNG có frame cho Hpmax/Spmax)       │  ← MISMATCH #4
                        │◄── [D] 08 01 19 00 <hp  LE32> 00000000        │
                        │◄── [E] 08 01 1A 00 <sp  LE32> 00000000        │
                        │◄── [F] 08 01 26 00 <point> 00000000           │  +0x402
                        │◄── [G] 08 01 25 00 <skillP> 00000000          │  +0x400
                        │        (KHÔNG có frame pet-exp)               │  ← MISMATCH #5
                        │        (KHÔNG có 05 0A LevelUP)              │  ← MISMATCH #3
                        │                                               │
                        │◄── [H] 0B 00 <selfId LE32> 00 00   (mỗi thành viên)
                        │    (FUN_00641fa4: tắt form, player+0x376 …)  │
                        │◄── [I] 0B 01 <row> <col> 00         (dọn ô lưới) │
                        │◄── [J] 0B 01 <row^1> <col>          (dọn ô pet) │
                        │◄── [K] 05 04                 (world-ready gate) │
                        │◄── [L] 14 08                 (TRANSITION-END)  │
                        │                                               │
                        │        (KHÔNG có frame item rơi vào túi)       │  ← MISMATCH #1
                        │        (KHÔNG có frame vàng)                  │  ← MISMATCH #2
                        │                                               │
                        │──► 0B 01 <confirm byte>   (chưa xác minh)    │  handle_leave_battle
                        │    hoặc tự đóng form → không cần gửi          │
```

Thứ tự **thực tế trên wire** hiện tại của Rust (do `runner::finish()` + `manager::dispatch()` quyết định, xem §4.1):

```
mỗi thành viên P (col 0..5, dòng `runner.rs:2789`):
   08 01 24 …            (Texp — chỉ khi exp>0, hp>0, lv<200, !fled)
   nếu lên cấp:
       08 01 23 …        (Lv)
       — (Hpmax: KHÔNG frame)
       08 01 19 …        (Hp)
       — (Spmax: KHÔNG frame)
       08 01 1A …        (Sp)
       08 01 26 …        (Point)
       08 01 25 …        (SkillPoint)
   nếu hp<=0: 08 01 19 01… (Hp=1)
   MapBroadcast 0B 00 <id> 00 00   (người khác thấy)
sau vòng lặp:
   Out::Respawn (nếu thắng/trốn)   16 03/16 05
   Broadcast: 0B00|0B01|0B01 cho MỌI thành viên (kể cả chính mình)
   Broadcast: 05 04
   Broadcast: 14 08
```

---

## 1. BẢNG TỔNG HỢP FRAME KẾT THÚC / EXP / ITEM / VÀNG

| # | Frame (plain payload) | Nội dung | Mức | Bằng chứng client |
|---|---|---|---|---|
| F1 | `0B 00` + `<id:4LE>` + `<kind:2LE>` (`kind=0`, `id==self`) | **THOÁT TRẬN / xoá trạng thái**: tắt 3 form, `FUN_00595a70(input,0)`, nếu `player+0x376==4` → `battleState=3`, **`player+0x376=0`**, `FUN_0050f2f8(conn,2)`, cài callback `LAB_00644A18`; ẩn thêm 4 form | **(A)** cơ chế / **(B)** nhãn "kết thúc trận" | `00641fa4_FUN_00641fa4.c:55-58` (parse), `:59-82` (self), `:69-76` (reset mode) |
| F2 | `0B 01 <row≤3> <col≤4> [00]` | dọn dòng/ô đơn vị trên `TFightField` (`DAT_0098c63c`) | **(A)** | `0064bb24_FUN_0064bb24.c:50-62` (guard), `:64-83`, `:103-108` |
| F3 | `0B FA <mode> <sub> (record 24B × n)` | **MỞ** trận: `DAT_0098c63c := TFightField_Create(mode, sub)`; record 24B/ô | **(A)** | `006438ac_FUN_006438ac.c:94-114`, `:122-249`, `:250-391` |
| F4 | `34 01` | reset 21 slot + `FUN_00658c30` (turn engine) — **KHÔNG phải frame kết quả trận** | **(A)** | `00650a24_FUN_00650a24.c:36-61`, `:121`, `:124`; `../opcode_34.md:16-17` |
| F5 | `33 01` + record 7B × n (`[grp][slot][flag][dword LE]`) | áp `flag/value` lên unit trong lưới — **KHÔNG phải kết quả trận** | **(A)** | `0064cae0_FUN_0064cae0.c:62-67`, `:120-121`, `:187` |
| F6 | **`08 01 <attr> <sign> <value LE32> 00000000`** | **PUSH 1 CHỈ SỐ CỦA NHÂN VẬT LOCAL** — đây là kênh EXP/Level duy nhất | **(A)** | `case_009_0078D125_FUN_0078d125.c:41` (sub), `:50` (sign=P3), `:51-53` (v1), `:63-64` (v2), `:65-71` (attr=P2 → `FUN_00710ab0`) |
| F7 | `08 01 24 …` (attr `0x24`) | ghi `actor+0x3FC` = **EXP** (DWORD, ghi đè tuyệt đối, không clamp) | **(A)** | `00710ab0_FUN_00710ab0.c:293-295`; getter `007108f4_FUN_007108f4.c:86-88` |
| F8 | `08 01 23 …` (attr `0x23`) | ghi `actor+0x3FA` = **LEVEL (byte)** + **tự tính lại MaxHP `+0x404` và MaxSP `+0x406`** | **(A)** | `00710ab0_FUN_00710ab0.c:271-292`; `007108f4_FUN_007108f4.c:83-85`; **bằng chứng `0x3FA` = level**: `test/ts_decompile/functions/0065a300_FUN_0065a300.c:72-77` (truyền `actor[0x3FA]` làm tham số *level* cho hệ số MaxHP) |
| F9 | `08 01 25 …` / `08 01 26 …` | `+0x400` / `+0x402` (Word) — **skill point / attribute point** | **(A)** ghi đích / **(B)** nhãn | `00710ab0_FUN_00710ab0.c:296-322`; `007108f4_FUN_007108f4.c:89-94` |
| F10 | `08 01 19/1A/CD/CE …` | `+0x3EA` HP / `+0x3EC` SP / `+0x404` MaxHP / `+0x406` MaxSP | **(A)** | `00710ab0_FUN_00710ab0.c:164-200`, `:352-358`, `:557-563` |
| F11 | `05 03` (113B + skill list) | snapshot login chứa exp/level — **không phải** đường exp sau trận | **(A)** parser `FUN_0072e9a0` | `../opcode_05.md:130-151`; `client_pseudo_c/0072e9a0_FUN_0072e9a0.c:154` (`+0x3FA`), `:139-142` (`+0x3FC`), `:143-150` (`+0x400/+0x402`) |
| F12 | `05 0A <charId LE32>` | **EVENT LÊN CẤP**: `FUN_0071e004(actor,"LevelUP",0)` + `sound\WA0013.wav` | **(A)** | `0072f970_FUN_0072f970.c:52-56`, `:60-66` |
| F13 | `17 05` + record 12B × n | **DUMP TOÀN BỘ HÒM ĐỒ**; **wipe 26 ô trước khi nạp** | **(A)** | `case_020_007902FB_FUN_007902fb.c:47-48` → `0076bfc8_FUN_0076bfc8.c:77-86` (wipe), `:87-99` (n=(len−1)/12), `:101-203` (record) |
| F14 | `17 06` + `[id:2LE][count][doben][long][giatri+100][khang][texp LE32]` | **THÊM 1 ITEM VÀO TÚI** — layout = record `17 05` **bỏ byte slot đầu** | **(C)** server contract (Rust dùng ở inventory/shops/trade/system) / **(B)** suy ra từ `17 05` | handler `func_0x0076b458` (`case_020…c:53-54`) **KHÔNG có body trong dump** |
| F15 | `17 10 <src≤6> <dst≤25>` | chuyển item **từ ô trang bị `player+0x2C[1..6]`** sang ô hòm đồ `player+0x9E0[1..25]` (VMT+4 với `count=1`) — **KHÔNG phải nhận item** | **(A)** | `0077c64c_FUN_0077c64c.c:57-69`, `:74`, `:103-109` |
| F16 | `17 0C <idx≤6>` | `FUN_00730870`/`FUN_0077c120` trên `player+0x2C[idx]` — thao tác ô trang bị | **(A)** | `0077d0b9_FUN_0077d0b9.c:41-58` |
| F17 | **`1A 04 <gold LE32> <cash LE32>`** | **SET TUYỆT ĐỐI VÀNG** → `player+0x12F8 = gold`, `+0x1300 = cash` | **(A)** | `0072bcf8_FUN_0072bcf8.c:51-56` |
| F18 | **`1A 01 <amount LE32>`** | **CỘNG VÀNG** (`player+0x12F8 += amount`, trần 9 999 999) + banner "Nhận được X Tiền" + chuông + `TLight` | **(A)** | `0072b084_FUN_0072b084.c:41-63` |
| F19 | `1A 02 <amount LE32>` | trừ vàng + banner "Giảm X Tiền" | **(A)** | `../opcode_1a.md:3` §3 (sub `0x02` → `FUN_0072b170`) |
| F20 | `0F 08 <pet record>` | per-pet stat: `[stt:1][id:2LE][texp LE32][lv:1][hp:2][sp:2]…` → ghi `+0x3FC`(exp) / `+0x3FA`(lv) / `+0x3EA`(hp) | **(A)** | `case_015_0078E367_FUN_0078e367.c:47-49` → `test/ts_decompile/functions/007a5010_FUN_007a5010.c:178-198` (đọc exp rồi level rồi hp — **thứ tự khớp `pet_stat_entry` của Rust**) |
| F21 | `05 04` | world-ready gate: `scene+0x53FC=1`, stamp tick, `FUN_0079b620(TMouseInfo,0)` — mở lại điều khiển chuột/bàn phím | **(A)** | `../opcode_05.md:100-110` |
| F22 | `14 08` | TRANSITION-END: xoá `a095/076/097/096/094`, `scene+0x1D=0`, `a085=0`, `conn+0x44F=0` | **(A)** | `../opcode_14.md:76` |
| F23 | `35 04 <id LE16> <npcRow> <npcCol> <row> <col>` | item rơi **hiển thị trên lưới trận** (không phải vào túi) | **(B)** | `../opcode_35.md:60` (`35 04` = `[id Word][grp≤3][slot≤4](b1,b2)×m` → `FUN_00654590(unit,b1,b2,id)`) |
| F24 | `08 01 08 <sign> <value LE32> 0000` (attr `0x40`) | chỉ số **follower** `actor+0x55E` (chỉ khi `actor[0x79]==4`) | **(A)** | `00710ab0_FUN_00710ab0.c:110-153` |

---

## 2. PHÂN TÍCH CHI TIẾT TỪNG FRAME

### 2.1 `0B FA` — MỞ TRẬN (để đối chiếu khi kết thúc)

**(A)** `006438ac_FUN_006438ac.c`:

| Dòng | Nội dung |
|---|---|
| `:94-106` | guard `Len(RP) < 2 → BoundErr(1)`, `< 3 → BoundErr(2)` |
| `:99`, `:106` | `mode = RP[1]`, `sub = RP[2]` |
| `:107-112` | `n = (Len(RP) − 3) / 0x18` (record **24 byte**) |
| `:114` | `DAT_0098c63c = TFightField_Create(VMT_63CC84, 1, mode, sub)` |
| `:138`, `:149`, `:156-157`, `:164-165`, `:172-173`, `:184`, `:195`, `:202-227`, `:238`, `:249` | record 24B: `kind=+0`, `type=+1`, `id=+2..5` (LE32), `w=+6..7` (LE16, guard ≤100 khi type==3), `dword=+8..11` (LE32), `row=+12`, `col=+13`, 4×Word `+14..21`, `lv=+22`, `elem=+23` |
| `:250-391` | `switch(type)`: `2` = player/người chơi (khi `id==myId` → **`player+0x376 = kind`** tại `:262`; kind `∈{4,7}` → `+0x1307=1`; kind `4` → `DAT_0098c640+0x25=1` + `FUN_0079b620(TMouseInfo,3)`; `battleState=2` tại `:271`); `3/4/6/7/9/0xB/0xC/0xD/0xE/0xF/0x10/0x12` = NPC/pet theo họ khác |
| `:396-402` | sau vòng lặp: nếu `player+0x376==4` → `unit[0]+0x35C=0`, `battleState=1`; ngược lại `FUN_0058e76c(...,1)` |

**Đối chiếu Rust (`src/battle/construction.rs:449-483`, `src/battle/engine.rs:71-88`)**:
Rust dựng record = `marker(1B) + packet_hex(23B)` = 24B; `marker` đóng vai trò `kind`,
`packet_hex[0] = typ` đóng vai trò `type`. Ánh xạ byte cho byte:

```
record[0]  = kind    ← Rust marker: "05" (self) / "03" (leader) / "64" (member khác)
record[1]  = type    ← WarInfo.typ
record[2..6]= id      ← le32(war.id)
record[6..8]= w       ← le16(war.id_npc_on_map)
record[8..12]= dword ← le32(war.id_char)
record[12] = row     ← war.row      → client local_10
record[13] = col     ← war.col      → client local_11
record[14..16]= Word ← le16(hp_max) → client local_30
record[16..18]= Word ← le16(sp_max) → client local_2e
record[18..20]= Word ← le16(hp)     → client local_2c
record[20..22]= Word ← le16(sp)     → client local_2a
record[22] = lv      ← war.lv       → client local_28
record[23] = elem    ← war.thuoctinh→ client local_27
```

→ **(B) layout record `0B FA` của Rust ĐÚNG** (mode/sub ở `RP[1..2]`, record từ `RP[3]`, stride 24; payload length `4 + text.len()/2` ⇒ `n = text.len()/48` = số block).
`battle_open_leader` (`packets.rs:9-15`) dùng marker `03` cho leader — nhưng client chỉ reset `player+0x376 = 0` khi giá trị đó **đúng bằng 4** (xem §2.2). Xem §5 gap G1.

### 2.2 `0B 00 <id LE32> 00 00` — FRAME KẾT THÚC TRẬN (self)

**(A)** `00641fa4_FUN_00641fa4.c`:

```
:55-58   id   = Copy(RP,2,4)  → P[2..5]   (DWORD LE, FUN_0077ef7c)
         kind = Copy(RP,6,2)  → P[6..7]   (Word  LE, FUN_0077eb9c)
:59      if (kind != 0)  → nhánh follower  :107-124  (kind ≤ 200, so `gvar_007D9FB4[kind]`)
:60      if (id == *(gvar_007DA7BC + 4))          // ← bản thân mình
:61         if (player[0x376] != 0) {
:62-67          if (player[0x376] != 4) { FUN_0058e76c(...,0); ẩn 3 form (VMT+0x24) }
:68             FUN_00595a70(gvar_007DA1DC, 0);            // tắt input bar
:69-76          if (player[0x376] == 4) {
                    *(gvar_007DA37C + 0x0C) = 3;          // battleState ← 3
                    *(player + 0x376) = 0;                 // ★ thoát trận
                    FUN_0050f2f8(gvar_007DA664, 2);       // reset state conn
                    conn+0x458 = &LAB_00644A18;           // callback trễ
                    conn+0x45C = param_1 (TFightManage)   // tham số callback
                }
:77-81       ẩn thêm 4 form (VMT+0x24)
             }
:83-105  else  // người khác: idx = FUN_0070c20c(scene,id) → actor[0x376] = 0 + dội FX
```

**Kết luận (B):** `0B 00 <selfId> 0000` là frame **"rời trận / xoá trạng thái actor"**.
Nó là frame thoát DUY NHẤT ghi `player+0x376 = 0` (grep toàn `client_pseudo_c/*.c`: chỉ `00641fa4:71`).
Điều kiện tiên quyết: `player[0x376]` phải bằng **4** mới reset được.

**FUN_0050f2f8 (được gọi ở `:72`)** — `0050f2f8_FUN_0050f2f8.c:55-64`:
nếu `conn+0x45A != 0` → gọi callback `conn+0x458(conn+0x45C)` rồi xoá; sau đó
`conn+0x450 = 2`, `conn+0x44E = 0`, `conn+0x441 = 0x0F`, `conn+0x430 = <double toàn cục>`.
Đây là hàm **reset state kết nối/timer** (có 25 call-site, gồm cả `TForm1.ClientSocket1Disconnect`,
`TForm1.Button24Click`) — **không** nên gọi là "disconnect" (mức **(B)**).

### 2.3 `0B 01 <i1≤3> <i2≤4> [pad]` — DỌN Ô LƯỚI

**(A)** `0064bb24_FUN_0064bb24.c:50-62` (guard `i1 ≤ 3`, `i2 ≤ 4`), `:63` null-guard `DAT_0098c63c`,
`:64-83` tra `record[i1]` (stride `0x46` = 70 B) → `+0x48 + i2*14`, `:103` đọc unitIdx tại `+0x4C + i2*14`.
⇒ Guard `≤3 / ≤4` **trùng khớp** `row ∈ 0..3`, `col ∈ 0..4` của lưới trận Rust → **(A) `reposition()` /
`clear_pet_cell()` của Rust (`packets.rs:74-82`) đúng về khoảng chỉ số**.

### 2.4 `08 01 <attr> <sign> <v1 LE32> <v2 LE32>` — KÊNH EXP/LEVEL DUY NHẤT

**(A)** `case_009_0078D125_FUN_0078d125.c`:

| Dòng | Wire | Ghi chú |
|---|---|---|
| `:37-41` | `SubOp = RP[0] = P[1]` | `Len(RP)==0 → BoundErr(0)` |
| `:43-78` | `case 1:` | `deltaType = P[3]` (`:50`, guard `Len<3`) |
| `:51-53` | `value1 = Copy(RP,4,4) = P[4..7]` DWORD LE | |
| `:54-62` | `if (deltaType == 2) value1 = -value1` | `sign`: `0/1` = tuyệt đối, `2` = âm |
| `:63-64` | `value2 = Copy(RP,8,4) = P[8..11]` DWORD LE | |
| `:65-71` | `attrCode = P[2]` → `FUN_00710ab0(gvar_007DA7BC, attrCode, value1, value2)` | |

⇒ Payload **12 byte**: `08 01 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11`.

**Rust `packets::status_update(byte, value)`** (`packets.rs:140-148`) =
`F4440C0008 01 <byte> <sign> <le32 value> 00000000` → payload 12B,
`attrCode = status_byte`, `deltaType = sign`, `value1 = le32(value)`, `value2 = 0`.
⇒ **(A) KHỚP BYTE-FOR-BYTE.**

**Bảng attrCode → offset (verify trực tiếp từ `00710ab0_FUN_00710ab0.c`)**:

| attrCode | offset | kiểu | clamp | dòng | tên (mức) |
|---:|---:|---|---|---|---|
| `0x19` | `+0x3EA` | Word | `[0..0xFFFF]` | `:164-178` | HP **(A/B)** |
| `0x1A` | `+0x3EC` | Word | `[0..0xFFFF]` | `:179-200` | SP **(A/B)** |
| `0x1B..0x1E` | `+0x3EE..+0x3F4` | Word ×4 | `≤0xFFFF` | `:201-244` | INT/ATK/DEF/AGI **(A/B)** |
| `0x1F` | `+0x3F6` | Word | `≤0xFFFF` | `:245-257` | HPX + recompute MaxHP **(A/B)** |
| `0x20` | `+0x3F8` | Word | `≤0xFFFF` | `:258-270` | SPX + recompute MaxSP **(A/B)** |
| **`0x23`** | **`+0x3FA`** | **Byte** | `≤0xFF` | `:271-292` | **LEVEL** — **sau khi ghi sẽ gọi `FUN_0065b6ac`+`FUN_0065b6e8` để TỰ tính lại `+0x404`/`+0x406`** **(A cơ chế / (B) nhãn level, đã củng cố bằng `0065a300.c:72-77`)** |
| **`0x24`** | **`+0x3FC`** | **DWORD** | không clamp, **ghi đè** | `:293-295` | **EXP** **(A)** |
| `0x25` | `+0x400` | Word | `≤0xFFFF` | `:296-314` | **skill point** — sau khi ghi refresh panel `gvar_007D9EB8` **(A cơ chế / (B) nhãn)** |
| `0x26` | `+0x402` | Word | `≤0xFFFF` | `:316-322` | **attribute point** **(A cơ chế / (B) nhãn)** |
| `0x32` | `+0x42E + aux*2` | Word[6] | `aux ≤ 5` | `:323-334` | 6 thanh phụ **(A)** |
| `0x3E` | `+0x43E` | Byte | `≤0xFF` | `:335-341` | layout HUD **(A/B)** |
| `0xCD` | `+0x404` | Word | `≤0xFFFF` | `:557-563` | MaxHP **(A/B)** |
| `0xCE` | `+0x406` | Word | `≤0xFFFF` | `:352-358` | MaxSP **(A/B)** |
| `0xCF` | `+0x41C` | DWORD | — | `:565-573` | + recompute MaxHP **(A)** |
| `0xD0` | `+0x420` | DWORD | — | `:574-582` | + recompute MaxSP **(A)** |
| `0xDA`/`0xDB` | `+0x424`/`+0x428` | DWORD | — | `:648-664` | HP/SP base + recompute **(A)** |
| `0x40` | `+0x55E` | Byte | `≤0xFF` | `:110-153` | follower stat, **chỉ khi `actor[0x79]==4`** |

> **ĐÍNH CHÍNH QUAN TRỌNG — `../opcode_08.md` §4.2 (bảng mapping) SAI 2 DÒNG.**
> `opcode_08.md:111` ghi `0x23 → +0x3FA` là "Cờ chế-nghiền công thức HP/SP (job-state)" và
> `opcode_08.md:113` ghi `0x25 → +0x400` là "Level". Thực tế:
> * `+0x3FA` là **LEVEL**: `test/ts_decompile/functions/0065a300_FUN_0065a300.c:72-77` truyền
>   `actor[0x3FA]` (1 byte) làm **tham số level** cho `FUN_0065a554` (bảng hệ số theo cấp),
>   và `FUN_00710ab0` recompute MaxHP/MaxSP ngay sau khi ghi nó — đúng hành vi "lên cấp".
> * `test/ts_decompile/functions/007a5010_FUN_007a5010.c:181→198` (`0F 08`) đọc
>   **DWORD exp vào `+0x3FC` rồi 1 byte level vào `+0x3FA`** — thứ tự `exp → level → hp`
>   trùng khớp `pet_stat_entry()` của Rust (`spawn.rs:405-421`).
> * `0F 08` tương tự: `+0x400` đi kèm sau `+0x55D/55E/55F` — nhóm điểm, không phải level.
>
> **Hệ quả: mapping attrCode của Rust (`service.rs:542-558`) là ĐÚNG** — `Lv→0x23`, `Texp→0x24`,
> `SkillPoint→0x25`, `Point→0x26`. *(Đây là điểm ngược lại với dự đoán ban đầu khi đọc `opcode_08.md`.)*

### 2.5 `17 05` — DUMP HÒM ĐỒ (đường "vật phẩm nhận được" đã xác minh)

**(A)** `case_020_007902FB_FUN_007902fb.c:47-48` → `FUN_0076bfc8`:

| Dòng | Nội dung |
|---|---|
| `:77-86` | **WIPE 26 ô** `player+0x9E0[1..25]` qua VMT `+8` **trước khi nạp** ⇒ `17 05` là lệnh **thay thế toàn bộ**, không phải thêm |
| `:87-99` | `n = (Len(RP) − 1) / 0x0C` (RP gồm cả byte sub) |
| `:101-116` | record[0] = `slot` (guard `≤ 0x19` = 25) |
| `:117-124` | record[1..2] = `itemId` Word LE |
| `:125-179` | record[3]…record[7] = 5 byte trạng thái |
| `:180-187` | record[8..11] = DWORD qua **`FUN_0077ed68`** (`0077ed68` = `b0+b1·256+b2·65536+b3·2^24`, giống hệt `FUN_0077ef7c`) |
| `:188-192` | `derived = FUN_0065b580(table, itemId, 0, dword)` (byte) |
| `:199-203` | `(**(VMT+4))(slotObj, itemId, rec3, dword, derived, rec7, rec6, rec5, rec4)` |

Rust `dump_table()` (`session.rs:450-468`) ghi đúng 12 byte
`slot(1) + le16(id) + count(1) + doben(1) + long_val(1) + (giatri_long+100)(1) + khang(1) + le32(texp)(4)`
⇒ **(A) layout `17 05` của Rust khớp client.**

### 2.6 `17 06` — THÊM 1 ITEM (đường Rust dùng, chưa có body client)

**(C)** Rust: `inventory.rs:69-85 item_added_frame()` và `trade_storage.rs:27-37 normal_trade_add_wire()`:
```
17 06 | le16(id) | count | 00 | doben | long_val | (giatri_long+100) | khang | le32(texp)
     = 13 byte payload
```
**(B)** suy luận: đây chính là record 12 byte của `17 05` **bỏ byte `slot` đầu tiên** ⇒ client tự chọn ô trống
đầu tiên trong 26 ô `player+0x9E0`. Handler `case 020 :6` = `func_0x0076b458(gvar_007DA0D0 /*TThingManage*/, RP)`
(`case_020_007902FB_FUN_007902fb.c:50-51`) — **`func_0x0076b458` KHÔNG có file trong `client_pseudo_c/`**
⇒ mức tối đa **(B)/(C)**; cần redump `0x0076b458` để chốt byte-for-byte.
*(Ghi chú: `gvar_007DA0D0` = `TThingManage` — đúng object quản lý vật phẩm, khớp ngữ nghĩa "thêm vật".)*

### 2.7 `1A 04` / `1A 01` — VÀNG

**(A)** `1A 04` = `0072bcf8_FUN_0072bcf8.c:51-56`:
`Copy(RP,2,4) → P[2..5]` = gold → `player+0x12F8`;
`Copy(RP,6,4) → P[6..9]` = cash → `player+0x1300`;
refresh `Form_Bank_Money` + caption `btn_001` của HUD.
Rust `spawn.rs:559-563` gửi `1A04` + `le32(gold)` + `00000000` ⇒ **khớp (A)**.

**(A)** `1A 01` = `0072b084_FUN_0072b084.c:41-63`: `player[0x12F8] += amount` (trần `9 999 999`),
refresh form, ghép chuỗi "Nhận được X Tiền" → caption nút. Đây là frame **báo nhận vàng**,
phục vụ cho phần thưởng trận. Rust **không dùng `1A 01` ở đâu cả** (grep: `1A04` chỉ ở
`spawn.rs`, `shops.rs:12`, `trade_storage.rs:125`).

### 2.8 `05 0A` — EVENT LÊN CẤP

**(A)** `0072f970_FUN_0072f970.c:52-56`:
`Copy(RP,2,4)` = charId; nếu `charId == *(gvar_007DA7BC+4)` →
`FUN_0071e004(self, "LevelUP", 0)` + phát `sound\WA0013.wav`; nếu là actor khác và tra được
(`FUN_0070c20c`) → gắn nhãn `"LevelUP"` nhưng **không** phát tiếng (`:60-66`).
`FUN_0071e004` ghi `actor+0x490 = "LevelUP"`, `+0x48C = 0`, `+0x498 = Now()`, `+0x4A0 = 0`
(`../opcode_05.md:116`) → nhãn sự kiện có timestamp cho nameplate.
Rust **không có** builder/tài liệu cho `05 0A` → grep `050A` trong `src/` = 0 hit.

### 2.9 `0F 08` — EXP/LEVEL CỦA THÚ NUÔI

**(A)** `case_015_0078E367_FUN_0078e367.c:47-49` → `FUN_007a5010` (`test/ts_decompile/functions/007a5010_FUN_007a5010.c`).
Record (offset 0-based trong RP, sau byte sub):
`[slot:1 ≤4]` `:143-151` · `[w:2]` `:158-163` · `[dword → slotobj+0x3FC = EXP]` `:178-181` ·
`[byte → slotobj+0x3FA = LEVEL]` `:190-198` · `[w:2 → +0x3EA = HP]` `:207-214` · … `[w:2 → +0x400]` `:~390`.

Rust `pet_stat_entry()` (`spawn.rs:398-441`) ghi:
`stt(1) | le32(id)(4) | le32(texp)(4) | lv(1) | le16(hp)(2) | le16(sp)(2) | …`

⚠️ **Lệch 2 byte**: client đọc `[w:2]` ở offset 1 rồi **exp DWORD ở offset 3**, còn Rust đặt
`le32(id)` (4 byte) ở offset 1 → mọi field từ `texp` trở đi lệch 2 byte.
⇒ **(B) nghi vấn `0F 08` lệch layout** (ngoài phạm vi bài này nhưng ảnh hưởng pet-exp sau trận).
Sửa đúng có lẽ là `id` chỉ nên là **Word LE 2 byte** (pet npc-id vừa khít u16).

---

## 3. BẢNG EXP & CÔNG THỨC CẤP EXP

### 3.1 Bảng ngưỡng EXP phía client

**(A)** `test/ts_decompile/functions/0065b0dc_FUN_0065b0dc.c:44-131` + `.asm.txt:10-40, 55-80, 89-114, 123-148`:

```
FUN_0065b0dc(table, level /*byte*/, reborn /*0..3*/) =
        *(uint*)( record(level) + 0xA8 )            // base tích lũy của cấp `level`
      + ROUND( level ^ k[reborn] )                  // System._Pow + @ROUND
```

* `record(level)` = `(*gvar_007DA6B8) + 4*level` → **mảng record 16 byte / cấp**;
  4 cột DWORD tại `+0xA8, +0xAC, +0xB0, +0xB4` tương ứng reborn `0..3`
  (asm đọc `+0xA8`; pseudo-C gộp thành `+0xAC` do mất `extraout`).
* Hằng số `k[reborn]` là **hằng 80-bit extended nhúng trong code**, 3 nhánh quan sát được tại
  `.asm.txt:64-66`, `:98-100`, `:132-134`:
  * reborn 0 → `PUSH 0x4000 / PUSH 0xC0000000 / PUSH 0x0`
  * reborn 1 → `PUSH 0x4000 / PUSH 0xC3333333 / PUSH 0x33333333`
  * reborn 2 → `PUSH 0x4000 / PUSH 0xC0000000 / PUSH 0x0`
  → **(C) chưa giải mã chính xác giá trị số của `k[]`; không khẳng định `2.9/3.0/3.05`.**
* Cùng bảng `gvar_007DA6B8` còn được dùng cho **hệ số MaxHP/MaxSP**:
  `test/ts_decompile/functions/0065a554_FUN_0065a554.c:88-90` đọc `*(word*)(gvar_007DA6B8 + 0xB0)`
  rồi cộng `ROUND(level ^ k)` ⇒ **bảng hệ số theo cấp**, không phải công thức `pow(lv, k)` giải tích.

### 3.2 Công thức exp Rust

**(C)** `src/data/texps.rs:16-31`:
```
Texp[i] = Texp[i-1] + round_ties_even( (i+1) ^ k[reborn] ) + 5,   k = {2.9, 3.0, 3.05}
Texp[0] = 6   (vì round(1^e)=1 rồi +5)
texp_get_lv_up(texps, lv, reborn, texp)  // đi từ hàng `lv`, dừng khi texp < threshold
```

### 3.3 So khớp / lệch

| Hạng mục | Client **(A)** | Rust **(C)** | Kết luận |
|---|---|---|---|
| Dạng bảng | mảng record/cấp, **tích lũy** `base(lv) + ROUND(lv^k)` | `Vec<TexpRow>` tích lũy, `lv` bắt đầu từ 1 | ✅ cùng hình dạng |
| Số cột | 4 (reborn 0..3) | 3 (`reborn[3]`, `reborn.min(2)` ở `texp_get_lv_up`) | ⚠ reborn 3 bị bỏ |
| Hằng `+5` | không thấy trong client | có | ⚠ **lệch số** |
| Hằng `k` | 80-bit constant trong code, chưa giải mã | `{2.9, 3.0, 3.05}` (từ spec) | ⚠ **chưa xác minh** |
| Rounding | `@ROUND` ( Delphi `_ROUND` = round-half-away-from-zero) | `round_ties_even` (banker's) | ⚠ **khác nhau ở .5** |
| Công thức cấp exp | **không tìm thấy** ở client | `reward_exp(lv,npc)`: `floor(npc_lv*10*mod)`, `mod ∈ {1.2,1.0,0.7,0.3}` theo `npc_lv - lv` (`runner.rs:2947-2959`) | **(C) thuần server-side**; client không kiểm chứng |
| `hit_exp` | — | `damage::hit_exp(lv,npc)` dùng cho **đánh** (`runner.rs:2622-2629`) | **(C)** |
| Cỡ nhân exp | — | `per_exp = 1` (`service.rs:626`, `runner.rs:2800`) | **(C)** |

**Kết luận §3 (mức B):** phần **cộng dồn exp và ghi xuống client** là đúng kênh (`08 01 24` + giá trị
tuyệt đối), nhưng **bảng ngưỡng lên cấp** của Rust là một *tái cài đặt* độc lập, chưa từng được
đối chiếu byte với bảng `gvar_007DA6B8` của client. Rủi ro: client tự vẽ thanh EXP bằng bảng của nó
(`FUN_007a6008` → `FUN_0065b0dc`) trong khi server dùng bảng khác ⇒ **thanh EXP và level-up ở client
có thể lệch với server**.

---

## 4. ĐỐI CHIẾU SERVER RUST + MISMATCH

### 4.1 Thứ tự frame server gửi khi kết thúc trận (đọc từ code)

`manager.rs:188-196` (task) → `battle.finish(...)` → `dispatch(&out, sink)` → `sink.battle_ended(id, outcome)`.

`runner.rs:2780-2909 finish()`:

| Bước | `Out` | Frame | `packets.rs` |
|---:|---|---|---|
| 1 | `Db(Texp)` | `08 01 24 <sign> <texp LE32> 0000` | `:140-148` |
| 2 | `Db(Lv)` *(chỉ khi `lv_ups>0`)* | `08 01 23 00 <lv> 0000` | `:140-148` |
| 3 | `Db(Hpmax)` | **không frame** (`byte=None`) | `service.rs:550` |
| 4 | `Db(Hp)` | `08 01 19 00 <hp> 0000` | `:140-148` |
| 5 | `Db(Spmax)` | **không frame** | `service.rs:551` |
| 6 | `Db(Sp)` | `08 01 1A 00 <sp> 0000` | `:140-148` |
| 7 | `Db(Point)` | `08 01 26 00 <2*lv_ups> 0000` | `:140-148` |
| 8 | `Db(SkillPoint)` | `08 01 25 00 <lv_ups> 0000` | `:140-148` |
| 9 | `PetExp{..}` | **không frame** (chỉ `session.pets[i].texp += exp`) | `service.rs:266-282` |
| 10 | `Db(Hp=1)` nếu chết | `08 01 19 01 …` | `:140-148` |
| 11 | `MapBroadcast` | `0B 00 <id LE32> 00 00` | `packets.rs:69-72` |
| 12 | `Respawn` *(thắng/trốn)* | `16 03` + `16 05` | `service.rs:256-262` |
| 13 | `Broadcast(text9)` | cho **mọi** thành viên: `0B00 <id> 0000` ‖ `0B 01 <row><col> 00` ‖ `0B 01 <row^1><col>` (lặp mỗi người) | `:69-82` |
| 14 | `Broadcast` | `05 04` | `packets.rs:155-158` |
| 15 | `Broadcast` | `14 08` | `packets.rs:160-163` |

Sau đó `battle_ended()` (`service.rs:301-470`) chỉ:
resume Eve / `quest_lose_frames` / `battle_quest_win` (frame `18 xx` + `17 06` + `1A 04` của trade/quest),
persist DB — **không gửi frame exp/item/gold nào**.

### 4.2 Frame KHỚP (không cần sửa)

| Frame server | Bằng chứng client | Mức |
|---|---|---|
| `08 01 <attr> <sign> <v LE32> 0000` | `case_009:41,50,51-53,63-71` | **(A)** |
| `08 01 24` (EXP) → `+0x3FC` | `00710ab0:293-295` | **(A)** |
| `08 01 23` (LEVEL) → `+0x3FA` + tự recompute MaxHP/MaxSP | `00710ab0:271-292` + `0065a300:72-77` | **(A)** |
| `08 01 25/26` (`+0x400/+0x402`) | `00710ab0:296-322` | **(A)** |
| `08 01 19/1A` (`+0x3EA/+0x3EC`) | `00710ab0:164-200` | **(A)** |
| `0B 00 <id> 00 00` (`hide_from_map`) | `00641fa4:55-58,59-82` | **(A)** |
| `0B 01 <row≤3> <col≤4> [00]` | `0064bb24:50-62` | **(A)** |
| `05 04` | `../opcode_05.md:100-110` | **(A)** |
| `14 08` | `../opcode_14.md:76` | **(A)** |
| `17 05` (nếu server dùng `dump_homdo()`) | `0076bfc8:77-203` | **(A)** |
| `1A 04` (nếu server dùng `gold_status_frame`) | `0072bcf8:51-56` | **(A)** |

### 4.3 MISMATCH / THIẾU (theo mức độ)

| # | Mức | Vấn đề | Bằng chứng | Đề xuất |
|:-:|:-:|---|---|---|
| **M1** | **CAO** | **Item rơi sau trận KHÔNG bao giờ tới túi client.** `apply_drop()` chỉ `s.add_homdo_item()` + `MapBroadcast(35 04 …)`; **không** gửi `17 06` / `17 05` cho chủ. | `service.rs:157-178`; `runner.rs:2673-2688`, `:2711-2726` | Trong `apply_drop`, gửi `frame("1706", &normal_trade_add_wire(&item))` cho `owner` (đã có sẵn builder ở `trade_storage.rs:27-37`), hoặc gom rồi `s.dump_homdo()` (đã có sẵn `session.rs:426`) |
| **M2** | **CAO** | **Vàng trận KHÔNG tới client.** `apply_gold()` chỉ `s.gold += amount`; không `1A 01`/`1A 04`. | `service.rs:284-299`; `runner.rs:2666-2671` | Gửi `F44406001A01{le32(amount)}` (đã xác minh ở `0072b084:41-63`) — hoặc `1A 04` tuyệt đối |
| **M3** | **CAO** | **Thiếu frame EVENT lên cấp `05 0A <charId LE32>`.** Server chỉ push `08 01 23` (giá trị), client sẽ đổi số level nhưng **không** gắn nhãn `"LevelUP"` + không phát `sound\WA0013.wav`. grep `050A` trong `src/` = 0 hit. | `0072f970:52-56,60-66` | Thêm builder `F4440600050A{le32(charId)}` khi `lv_ups > 0` |
| **M4** | **TRUNG BÌNH** | **MaxHP/MaxSP không có frame.** `Stat::Hpmax/Spmax → byte = None`; client chỉ nhận `+0x404/+0x406` qua attr `0xCD/0xCE` hoặc qua `0xCF/0xD0/0xDA/0xDB` (recompute). Rust dựa vào việc client tự recompute khi nhận `0x23`. | `service.rs:550-551`; `00710ab0:271-292`, `:352-358`, `:557-582` | Gửi thêm `08 01 CD` / `08 01 CE` **sau** `08 01 23` để khóa giá trị server |
| **M5** | **TRUNG BÌNH** | **Exp thú KHÔNG tới client.** `apply_pet_exp()` chỉ cộng trong session. Frame đúng hình dạng là `0F 08` (F20). | `service.rs:266-282`; `007a5010:178-198` | `spawn::pet_status_single(&s, stt)` đã có sẵn (`spawn.rs:452-464`) — gọi lại sau khi cộng exp |
| **M6** | **TRUNG BÌNH** | **Công thức MaxHP lệch.** Client tự tính `MaxHP` bằng **bảng hệ số theo cấp** `gvar_007DA6B8` (`FUN_0065a554`), Rust dùng công thức giải tích `pow(lv,0.35)` (`engine.rs:100-111`). Client nhận `0x23` → tự recompute MaxHP → rồi nhận `0x19` với HP do server tính ⇒ **HP có thể vượt MaxHP mà client tự tính**. | `test/ts_decompile/functions/0065a300_FUN_0065a300.c:72-77`, `0065a554:88-90` vs `src/battle/engine.rs:100-111`; `service.rs:2818-2850` | Chốt công thức client (`FUN_0065a554` + bảng `gvar_007DA6B8`) rồi đồng bộ `get_hp_max/get_sp_max`, **hoặc** gửi `0xCD/0xCE` để ghi đè MaxHP/MaxSP của client |
| **M7** | **TRUNG BÌNH** | **Bảng ngưỡng lên cấp + công thức exp lệch/không kiểm chứng** (§3.3). | `texps.rs:16-59` vs `0065b0dc:44-131` | Dump/giải mã hằng `k[]` tại `0065b0dc.asm.txt:64-66,98-100,132-134`; đối chiếu `@ROUND` vs `round_ties_even` |
| **M8** | **THẤP** | `08 01 40` (`Stat::Fai`) bị client bỏ qua vì chỉ áp dụng khi `actor[0x79]==4` (TFollowNpc), còn `gvar_007DA7BC` là TPlayer. | `00710ab0:111-113` | Bỏ `stat_byte::FAI` khỏi đường battle, hoặc dùng cơ chế riêng |
| **M9** | **THẤP** | Builder `battle_toast_frame` (`0B 03`), `battle_flag_frame` (`0B 09`), `battle_counter_frame` (`0B 07`) đã viết nhưng **không nơi nào gọi** — 3 sub-op của `0B` bị bỏ trống. | `spawn.rs:340-362`; grep không có call-site | Xác minh rồi nối vào luồng battle (đếm lượt, mode) |
| **M10** | **THẤP** | `Out::Catch` (`apply_catch`) cũng không gửi frame pet mới. | `service.rs:180-192` | Cần frame nhận thú (`0F 07`?) — xem `../opcode_0f.md` |

### 4.4 Đính chính tài liệu liên quan (phát hiện phụ, ảnh hưởng "vật phẩm")

1. **`post_login_stats_inventory_flow.md` §3.3.1 & §5.2 gap #6: `1E 01` KHÔNG phải stall/vendor.**
   `FUN_00730F4C` (parser stall, tạo `TVenderItem` + UI `StallN`) chỉ được gọi từ
   `case_020_007902FB_FUN_007902fb.c:149` — tức **MainOp `0x17`, SubOp `0x1E`** ⇒ frame là **`17 1E`**
   (grep `0x00730f4c`: chỉ `007902fb`, `case_020`, dispatcher `0078a89c:3957`, `jumptable_0x78A9B6_cases:4409`).
   Ngược lại **MainOp `0x1E` SubOp `0x01`** là `case_026:28-29 → func_0x007707a0` =
   **batch ghi vào `player+0xA48` (51 slot), record 12 byte `[slot:1][code:Word LE][5 byte][q:4 LE]`**
   (`../opcode_1e.md:60`; `test/ts_decompile/functions/007707a0_FUN_007707a0.c:70-77` `n = (Len(RP)−1)/12`).
   ⇒ **Hệ quả: `dump_tientrang()` của Rust (`1E01` + record 12B `dump_table`) về mặt layout là KHỚP `1E 01`**;
   "mismatch" được nêu trong gap #6 là do nhận nhầm opcode. Câu hỏi còn mở chuyển sang:
   `tientrang` có thực sự là container `+0xA48` hay `+0x9E0`/`+0xB14`/`+0xDC0` (51/51/26 slot).
2. **`opcode_08.md` §4.2 bảng mapping**: sửa `0x23 → Level (byte)`, `0x25 → skill point (Word)`, `0x26 → attribute point (Word)`
   (bằng chứng §2.4). Nhãn "job-state" và "Level" đã bị hoán đổi.
3. **`opcode_0b.md` §3 SubOp 0 / opcode_11.md §2.3**: ghi rõ thêm rằng `0B 00 <selfId> 0000`
   **ghi `player+0x376 = 0` chỉ khi giá trị đó đúng bằng 4** — điều kiện tiên quyết quan trọng cho mock.
4. **`case_005_0078C7DB_FUN_0078c7db.c` KHÔNG phải parser `05`** — đó là **Case index 5 = MainOp `0x04`**
   (`jumptable_byte200_0x78A8EE.hex:1`: `byte[0x04] = 0x05`). Parser `05` thật là `case_006_0078CA56`
   / `FUN_0072e9a0` (`../opcode_04.md`, `../opcode_05.md`). `post_login_…md:120-123` cần sửa.

---

## 5. KHOẢNG TRỐNG (chưa đóng được)

| ID | Khoảng trống | Vì sao chưa đóng | Cách đóng |
|---|---|---|---|
| **G1** | **Kind byte của record `0B FA` cho bản thân mình nên là `04` hay `05`?** | `0B 00` chỉ reset `player+0x376 = 0` khi `== 4` (`00641fa4:69-71`). Rust dùng marker `05` cho bản thân (`construction.rs:449`) và `03` cho leader (`packets.rs:11`). Nếu giá trị thật của server là `4`, thì sau trận `player+0x376` **vẫn = 5/3 ≠ 0** ⇒ các gate `if (player[0x376] != 0)` ở `0077c64c:121`, `FUN_0064faa8`, `opcode_16` vẫn tưởng đang trận | Chụp traffic thật, hoặc suy từ `0x376 ∈ {0,2,3,4,5,7}` + logic `34 01` (`0x376==4` thì **bỏ qua** reset selector → nghĩa là 4 là trạng thái *đặc biệt*, không phải trận thường) |
| **G2** | Layout `0B 01` phía C→S (SubSel 1..4) | Bảng nhảy con `0x780440` (`0077f414_FUN_0077f414.asm.txt:744`) **chưa được dump**; không có `.hex` phủ `0x780440`. Đã xác minh `SubSel 6/7/8/9/0x0A/0xFA` (asm `:758`, `:781`, `:830`, `:884`, `:907`, `:955`) | Dump `0x780440..0x780454` (20 byte) |
| **G3** | `0B 01` + byte confirm = `3` (contract của `handle_leave_battle`) | Chưa tìm được call-site `SendCommand(MainOp=0x0B, SubSel=1)` trong các `.asm.txt` đã dump; `005eb530:680` chỉ gửi `0B` không rõ SubSel | Tìm `MOV CL,1; MOV DL,0xb` trong asm đầy đủ, hoặc hook socket |
| **G4** | Body `func_0x0076b458` (handler `17 06`) | Không có file trong `client_pseudo_c/` | Redump `0x0076B458` |
| **G5** | Body `func_0x00730F4C` cho `17 1E` — đã có nhưng **chưa** dùng để đối chiếu `tientrang` | `00730f4c_FUN_00730f4c.c:32-175` có sẵn; cần xác định `tientrang` Rust nên dùng `17 1E` (stall) hay `1E 01` (container 51 slot) hay `17 2F`/`17 66` | Đọc `00730f4c` + so sánh số slot client (`+0x9E0`=25, `+0xA48`=51, `+0xB14`=51, `+0xDC0`=26) với `free_slot` Rust (homdo 25, storage 50) |
| **G6** | Hằng số số học `k[reborn]` của bảng EXP (`0065b0dc.asm.txt:64-66, 98-100, 132-134`) | Hằng 80-bit trong code, chưa giải mã chắc chắn (thứ tự push x87 dễ sai) | Giải mã 10 byte theo đúng stack layout `FSTP` + 3×`PUSH`, đối chiếu với giá trị `2.9/3.0/3.05` của spec |
| **G7** | Cột `reborn == 3` | Client có 4 cột (`+0xA8..+0xB4`), Rust chỉ 3 (`reborn.min(2)`) | Xác minh cột thứ 4 dùng cho hồi sinh lần 4 |
| **G8** | `05 03` tail sau `tiengtam` | Client `FUN_0072e9a0` đọc `p[0x1C..0x1F]→+0x440`, `p[0x20..0x23]→+0x404/+0x406`; Rust gửi `hp_max/sp_max` sớm hơn 4 byte ⇒ lệch (gap #4 của `post_login_…md`) | Đối chiếu byte-exact tail bằng capture |
| **G9** | Layout `0F 08` lệch 2 byte (§2.9) | Rust `le32(id)` 4 byte ở offset 1; client đọc `Word` ở offset 1 rồi DWORD exp ở offset 3 | Đọc kỹ `FUN_007a5010` toàn bộ 20+ field, sửa `pet_stat_entry()` |
| **G10** | Không có **frame báo kết quả trận** (thắng/thua, số lượt, thời gian, điểm tiêu diệt) | Grep toàn bộ `case_*.c` + `../opcode_*.md` không tìm thấy frame nào mang tập (thắng/thua, exp, điểm, thời gian). Kết luận: **client TS Online không có frame "battle result" riêng** — thông tin phần thưởng được đẩy rải rác qua `08 01 …` / `17 06` / `1A 01` / `05 0A` | Chấp nhận kết luận này; nếu UI client cần bảng tổng kết thì phải dùng kênh chat `02`/`35 0B` |
| **G11** | `LAB_00644A18` (callback trễ cài sau khi thoát trận) | Không có body trong dump | Redump `0x00644A18` |

---

## 6. SOURCE TRAIL

| # | File | Dòng / offset | Dùng cho |
|:-:|---|---|---|
| 1 | `client_pseudo_c/jumptable_byte200_0x78A8EE.hex` | `:1-5` | MainOp → Case index (lưu ý lệch 1 ở MainOp < 0x0B) |
| 2 | `client_pseudo_c/case_009_0078D125_FUN_0078d125.c` | `:35-72` | **layout `08 01`** |
| 3 | `client_pseudo_c/00710ab0_FUN_00710ab0.c` | `:46-677` (cụ thể `:110-153,164-200,245-334,352-358,557-582,648-664`) | **stat setter — bảng attrCode→offset** |
| 4 | `client_pseudo_c/007108f4_FUN_007108f4.c` | `:40-111` | getter đối xứng, chốt size/boundary |
| 5 | `client_pseudo_c/00641fa4_FUN_00641fa4.c` | `:55-124` | **`0B 00` = thoát trận** |
| 6 | `client_pseudo_c/006438ac_FUN_006438ac.c` | `:94-402` | **`0B FA` = mở trận**, record 24B |
| 7 | `client_pseudo_c/0064bb24_FUN_0064bb24.c` | `:50-108` | `0B 01` guard row/col |
| 8 | `client_pseudo_c/0064cae0_FUN_0064cae0.c` | `:62-191` | `33 01` record 7B |
| 9 | `client_pseudo_c/00650a24_FUN_00650a24.c` | `:36-131` | `34 01` reset + turn engine |
| 10 | `client_pseudo_c/case_020_007902FB_FUN_007902fb.c` | `:36-149` | map SubOp `0x17` → handler (`17 05`@47-48, `17 06`@50-51, `17 0B`@65-66, `17 1E`@148-149) |
| 11 | `client_pseudo_c/0076bfc8_FUN_0076bfc8.c` | `:77-206` | **`17 05` layout 12B + wipe** |
| 12 | `client_pseudo_c/0077c64c_FUN_0077c64c.c` | `:54-142` | `17 10` = trang bị → hòm đồ |
| 13 | `client_pseudo_c/0077d0b9_FUN_0077d0b9.c` | `:41-84` | `17 0C` |
| 14 | `client_pseudo_c/00730f4c_FUN_00730f4c.c` | `:32-175` | `17 1E` = stall/vendor (KHÔNG phải `1E 01`) |
| 15 | `client_pseudo_c/0072bcf8_FUN_0072bcf8.c` | `:51-61` | **`1A 04`** |
| 16 | `client_pseudo_c/0072b084_FUN_0072b084.c` | `:41-63` | **`1A 01` cộng vàng** |
| 17 | `client_pseudo_c/0072f970_FUN_0072f970.c` | `:52-66` | **`05 0A` LevelUP** |
| 18 | `client_pseudo_c/0072e9a0_FUN_0072e9a0.c` | `:88-413` (`:139-156`) | `05 03` snapshot self (exp/level/point) |
| 19 | `client_pseudo_c/case_026_007921A6_FUN_007921a6.c` | `:20-60` (`case 1` @ `:28-29`) | MainOp `0x1E` → SubOp 1 = `func_0x007707a0` (container 51 slot) |
| 20 | `client_pseudo_c/case_015_0078E367_FUN_0078e367.c` | `:47-49` | `0F 08` → `FUN_007a5010` |
| 21 | `client_pseudo_c/0077f414_FUN_0077f414.asm.txt` | `:34-35` (dispatch), `:737-744` (builder `0B`), `:758-780` (SubSel 6), `:781-829` (SubSel 7), `:830-883` (SubSel 8), `:884-906` (SubSel 9), `:907-954` (SubSel 0x0A), `:955-983` (SubSel 0xFA) | **C→S builder `0B`** |
| 22 | `client_pseudo_c/cs_dispatch_byte200_0x77f474.hex` | `:4` (`byte[0x0B]=0x0B`) | C→S dispatch index |
| 23 | `client_pseudo_c/cs_builder_dword200_0x77f53c.hex` | `:3` (index 11 → `0x780420`) | C→S builder `0B` |
| 24 | `test/ts_decompile/functions/0065b0dc_FUN_0065b0dc.c` + `.asm.txt` | `:44-131` / `:10-148` | **bảng EXP theo cấp** |
| 25 | `test/ts_decompile/functions/0065a300_FUN_0065a300.c` | `:72-77` | **chứng minh `0x3FA` = LEVEL** |
| 26 | `test/ts_decompile/functions/0065a554_FUN_0065a554.c` | `:88-90` | hệ số MaxHP theo cấp (bảng) |
| 27 | `test/ts_decompile/functions/007a5010_FUN_007a5010.c` | `:143-214`, `:390-393` | `0F 08` pet record |
| 28 | `test/ts_decompile/functions/007707a0_FUN_007707a0.c` | `:70-77` | `1E 01` record 12B / container `+0xA48` |
| 29 | `src/battle/runner.rs` | `:2615-2732`, `:2774-2909`, `:2947-2959` | drop/exp/gold + `finish()` |
| 30 | `src/battle/service.rs` | `:157-178`, `:266-299`, `:301-470`, `:542-558` | `apply_drop/pet_exp/gold`, `battle_ended`, `stat→attrCode` |
| 31 | `src/battle/manager.rs` | `:188-196`, `:303-336` | thứ tự dispatch `Out` |
| 32 | `src/battle/packets.rs` | `:9-15`, `:69-82`, `:84-92`, `:128-137`, `:140-163` | builder `0BFA`, `0B00`, `0B01`, `3504`, `0801`, `0504`, `1408` |
| 33 | `src/battle/construction.rs` | `:325-500` | marker `05/03/64` + `packet_hex` |
| 34 | `src/battle/engine.rs` | `:71-111` | `packet_hex()` + `get_hp_max` |
| 35 | `src/data/texps.rs` | `:16-59` | bảng EXP Rust |
| 36 | `src/server/spawn.rs` | `:340-362`, `:398-464`, `:498-566` | `0B03/09/07` builders, pet frames, Logined1 |
| 37 | `src/server/session.rs` | `:424-468` | `dump_table()` 12B |
| 38 | `src/server/handlers/trade_storage.rs` | `:27-37`, `:124-126`, `:311-321` | `17 06` / `1A 04` builders (contract) |
| 39 | `src/server/handlers/inventory.rs` | `:69-85` | `item_added_frame()` (`17 06`) |
| 40 | `src/server/handlers/battle.rs` | `:48-65` | `0B 01` + confirm `3` |

---

*Tài liệu soạn 2026-10-01. Mọi layout byte đều kèm `file:line`. Claim nào suy luận đều đánh dấu (B);
claim nào chỉ dựa trên source Rust đánh dấu (C). Đã loại bỏ mọi suy luận dựa trên
`case N: break;` rỗng của Ghidra. Nguồn chính `client_pseudo_c/`, đối chiếu
`test/ts_decompile/`. Những điểm sửa tài liệu cũ nêu ở §4.4 nên được đồng bộ ngược về
`post_login_stats_inventory_flow.md`, `opcode_08.md`, `opcode_0b.md`, `opcode_17.md`, `opcode_1e.md`.*