# HANDOFF — Bằng chứng reverse client `aLogin.exe` cho §1.3 (`battle-refactor`)

Ngày lập: 2026-10-16 · Trạng thái: **đã giải mã tĩnh xong §1.3**, còn vài câu hỏi cần capture runtime.

> Mục đích file: chốt lại toàn bộ bằng chứng byte-exact lấy từ export Ghidra, để không mất khi context bị nén.
> Không phải đặc tả cuối cùng — đặc tả cuối cùng là `spec.md` (cần đồng bộ lại theo file này).

---

## 0. Nguồn + quy ước đọc (ĐỌC TRƯỚC KHI VIẾT SPEC)

### 0.1 Nguồn

| Mục | Đường dẫn |
|---|---|
| Target | `client_delphi/aLogin.exe` — sha256 `1487059f0508d49e89946856eee4088b0cf53704efafc2d8d57988d148ff06f4` |
| Export Ghidra (nguồn chính) | `/mnt/d/VUDT/GIT_PCC/test/ts_decompile/` — `functions/` (13 218 file `.c` + `.asm.txt`), `redump/`, `case_functions/manifest.csv`, `index.csv` |
| Round-4 manifest | `/mnt/d/VUDT/GIT_PCC/test/Round4Manifest.txt` |
| Tài liệu nghiên cứu gốc | `.scratch/client-pseudo-op-code/battle/` (01..05) — **có lỗi quy ước index, xem §0.2** |
| REA headless | không dùng được (`provider_timeout` 330 s / re-import artifact) — bỏ qua |

### 0.2 Quy ước index (đã hiệu chuẩn — nguồn gây lỗi lớn nhất của tài liệu cũ)

Gọi:

- `content` = **toàn bộ** payload frame tính từ byte opcode chính (0-based).
- `RP` = chuỗi Delphi mà handler nhận = payload **sau** opcode chính ⇒ `RP[0]` = byte sub, và `content[j] == RP[j-1]` với `j ≥ 1`.

| Kiểu đọc trong ASM | Ý nghĩa |
|---|---|
| `_LStrCopy(RP, k, n, &dst)` | `k` là **1-BASED** trên `RP` ⇒ đọc `content[k .. k+n-1]` |
| Raw byte: `MOV EAX,base; DEC; CMP; INC; MOV AL,[EDX+EAX-1]` | đọc `content[base]` (vì `[EDX+EAX-1]` = `RP[base-1]`) |

**Ba bằng chứng hiệu chuẩn** (mỗi cái độc lập, khớp byte-exact với frame Rust đã verify):

1. `08 01` — `Copy(RP,4,4)` ⇒ value ở `content[4..7]`; Rust `packets::status_update` ghi `08 01 <attr> <sign> <le32> 0000` ⇒ value đúng `content[4..7]`. ✔
2. `0B 00` — `Copy(RP,2,4)` ⇒ id `content[2..5]`, `Copy(RP,6,2)` ⇒ grp `content[6..7]`; Rust `hide_from_map` = `0B 00 <le32 id> 0000`. ✔
3. `17 05` — record 12B: `base = i*12+2`; slot đọc raw tại `content[i*12+2]`, id `Copy(i*12+3, 2)` ⇒ `content[i*12+3..4]`, texp `Copy(i*12+10, 4)` ⇒ `content[i*12+9..12]` ⇒ record = `[slot][id:2][5 byte][texp:4]`. ✔

⇒ **Lỗi của tài liệu cũ**: `04_battle_end_rewards.md` §2.9/G4 trộn lẫn 0-based của `content` với 1-based của `RP`, dẫn tới kết luận "Rust lệch 2/4 byte" **sai** (xem §4 và §7).

---

## 1. Bảng dispatch (đã xác minh)

### 1.1 S→C — dispatcher `0078d5d1` (case index **= opcode**, vì `case_011` → `0x78D5D1` = opcode `0x0B`)

`0B` sub → hàm xử lý:

| sub | handler | nội dung |
|---:|---|---|
| 0 | `00641fa4` | thoát trận (xem §2) |
| 1 | `0064bb24` | dọn ô lưới `row ≤ 3`, `col ≤ 4` |
| 3 | `00644b84` | — |
| 4 | `00642244` | batch set `+0x376`, record 8B `[type][id:4][grp:2][value]`; type 2 → `gvar_007DA300` (≤800), type 6 → `gvar_007D9FB4` (≤200), type `0x10` → `gvar_007DA10C` |
| 5 | `00644294` | cập nhật entity trong lưới (`0B 05`) |
| 6 | `0072c9e8(gvar_007D9D34)` | — |
| 7 | inline: `*(gvar_007DA51C+0xEB4)=1`, `content[2]`,`content[3]` → `FUN_00733650` + `FUN_00595a70(...,1)` | auto-battle |
| 9 | `player+0x1308 = content[2]` (+ message) | banner chat |
| `0x0A` | `0064e978(gvar_007D9CE4)` | — |
| `0x0B` | `005ba604(gvar_007D9D00)` | — |
| `0x0C` | `0060e2a8(gvar_007DA4F8)` | — |
| `0xFA` | `006438ac` | mở trận (xem §2) |
| 2, 8, `0x0D`..`0xF9` | — | **không có trong switch** |

### 1.2 C→S — builder `FUN_0077f414` (gửi qua `FUN_0051633c` = `TForm1.CY_AddSedQueue`)

Sub selector nằm ở `[EBP-6]`; nhánh theo bảng nhảy `JMP dword ptr [EAX*4+0x780440]`:

| sub | địa chỉ thân | nội dung builder (đã đọc) |
|---:|---|---|
| 0 | `0x78048D` | chưa có trong file export (xem TODO §11) |
| 1 | `0x780511` | chưa có trong file export (xem TODO §11) |
| 2 | `0x780595` | `<sub> <byte> <u32(*+0x1E)> <u16(*+0x22)>` |
| 3 | `0x780671` | như trên |
| 4 | `0x780721` | như trên |
| 5 | inline `0x7807D1` | 2 byte thô (`[EBP-5]`, `[EBP-6]`) |
| 6 | inline `0x780826` | `<b5><b6>` + `player+0x384` + `player+0xE3` (3–4 byte) |
| 7 | `0x780826`? | — |
| **8** | `0x7808E2` | `<sub 08> <kind(*+0x24)> <le32(*+0x1E)> <le16(*+0x22)>` (xác minh ở `asm:848-882`: `MOV DL,[EDX+0x24]` → `FUN_0077ee84` với `EDX=[EAX+0x1E]` → `FUN_0077eaa4` với `DX=[EAX+0x22]` → `CALL 0x404148` nối 3 mảnh → `FUN_0051633c`) |
| 9 | inline `0x7809B8` | 2 byte thô |
| **`0x0A`** | `0x780A0D` | giống sub 8 nhưng `sub=0x0A` (asm:907-939) |
| `0xFA` | `0x780ABD` | — |

---

## 2. G1 — Kind byte của record self trong `0B FA` + điều kiện reset của `0B 00`

### 2.1 Receiver `0B FA` = `006438ac`

Header + record layout (đã verify byte-exact với `0B 05` = record 24B trừ byte kind):

```
content[0]      = 0x0B
content[1]      = 0xFA          (RP[0])
content[2]      = mode          (RP[1])   ─┐ header 3 byte
content[3]      = sub           (RP[2])   ─┘
n               = (Len(RP) - 3) / 24      record bắt đầu tại content[4]
record 24B:  kind +0 | type +1 | id +2..5 (u32) | w +6..7 (u16)
              dword +8..11 (u32) | row +12 | col +13
              4×u16 +14..21 | lv +22 | elem +23
```

`switch(type)`: `case 2` (self hoặc player khác), `3`, `4`, `6/0x0B/0x0C/0x0D/0x0F`, `7`, `9`, `0x0E`, `0x10`, `0x12`.

Nhánh self (`006438ac:250-272`), điều kiện `type == 2 && id == *(gvar_007DA7BC + 4)`:

| dòng | hành động |
|---:|---|
| `:253-256` | nếu `(kind == 2 \|\| kind == 5) && *(byte*)(player+0x384) == 0x3B` → `FUN_0072b390(self, 8)` |
| `:262` | **`*(char *)(player + 0x376) = kind;`** ← byte kind do server gửi thẳng vào state |
| `:263-266` | nếu `kind == 4 \|\| kind == 7` → `*(byte*)(player+0x1307) = 1` |
| `:267-270` | nếu `kind == 4` → `*(byte*)(DAT_0098c640+0x25) = 1; FUN_0079b620(gvar_007DA0B4, 3)` (cursor trận) |
| `:271` | `*(byte*)(gvar_007DA37C + 0xC) = 2` (battleState ← 2) |

Cuối hàm (`006438ac:396-402`):

```
if (*(char*)(player + 0x376) == 4) {
    *(byte*)(*(int*)(DAT_0098c63c + 0x158) + 0x35C) = 0;
    *(byte*)(gvar_007DA37C + 0x1C) = 1;
} else {
    FUN_0058e76c(gvar_007DA530, 1);
}
```

### 2.2 Receiver `0B 00` = `00641fa4` (điều kiện reset)

```
id  = Copy(RP,2,4) → content[2..5]
grp = Copy(RP,6,2) → content[6..7]
:59-81  (grp == 0 && id == self && player[0x376] != 0):
        :62-67  nếu 0x376 != 4  → FUN_0058e76c(...,0) + ẩn 3 form
        :68     FUN_00595a70(gvar_007DA1DC, 0)            (tắt input bar)
        :69-76  CHỈ KHI 0x376 == 4:
                  battleState+0xC = 3
                  player+0x376 = 0          ★ reset duy nhất
                  FUN_0050f2f8(gvar_007DA664, 2)
                  conn+0x458 = &LAB_00644A18   (callback trễ)
                  conn+0x45C = param_1
:83-105  người khác: FUN_0070c20c(scene,id) → actor[0x376] = 0 + dội FX
```

### 2.3 Call-site C→S `0B 01` (nút "rời trận") = `00590ba8:157-163`

```
case 9:   // dispatcher action của TForm (FUN_00596378 gọi vào 8 call-site)
  if (*(char *)(*(int *)gvar_007DA7BC + 0x376) == '\x04') {
      *(undefined1 *)(*(int *)gvar_007DA5AC + 0x130) = 3;
      piVar5 = FUN_0077f414(gvar_007D9D30, 0x0B);       // builder sub = 1
  }
```

### 2.4 ✅ KẾT LUẬN G1

Chuỗi bằng chứng khép kín:

1. Server gửi `0B FA`; record self mang `kind` → client ghi thẳng vào `player+0x376` (`006438ac:262`).
2. Client **chỉ** cho bấm "rời trận" khi `0x376 == 4` (`00590ba8:159`).
3. Client **chỉ** reset `0x376 = 0` trong `0B 00` khi `0x376 == 4` (`00641fa4:62,69`).
4. Kind `4` là trạng thái *đặc biệt* mà client tự trang bị: cursor trận `FUN_0079b620(...,3)`, `DAT_0098c640+0x25 = 1`, `player+0x1307 = 1`, và ở cuối handler chuyển `state+0x1C = 1` (thay vì `FUN_0058e76c(...,1)`).

⇒ **Server phải gửi `kind = 04` cho record self** trong mọi frame `0B FA` (cả view leader lẫn view member). Rust hiện tại sai ở 2 chỗ:

| File | Hiện tại | Sửa |
|---|---|---|
| `src/battle/packets.rs:11` `battle_open_leader` | `0BFA<diahinh>` + `03` + leader_packet (record của chính leader) | `03` → `04` |
| `src/battle/construction.rs:444` `member_battle_frame` | `text.push_str("05")` + self_packet | `"05"` → `"04"` |

Ghi chú về marker của **người khác** trong cùng frame (`03` cho leader + pet của leader, `64` cho member khác, và `0B 05 03`/`0B 05 05` cho entity NPC/player):
- Client không dùng `kind` của record không phải self vào gate nào đã biết (case 2 nhánh else chỉ đưa `&kind` xuống `FUN_006469b8`/`FUN_00659358` — hai hàm này **không** đụng `+0x376`).
- Bộ giá trị `03`/`05`/`64` khớp ngữ nghĩa loại entity (03 = NPC địch, 05 = player, 64 = đồng đội) ⇒ **giữ nguyên**.
- *Câu hỏi còn mở (mức C, nên capture runtime)*: các frame `0B 04` mà Rust gửi khi show-on-map
  (`show_player_on_map` = `0B 04 02 <le32 id> 0000 03`, `show_member_on_map` → `05`, `show_pk_leader_on_map` → `02`)
  đặt `0x376` của người khác theo view. Vì gate tấn công (`0064ea20`, `0064efa0:56`) chỉ chặn `0x376 == 4`,
  có thể server thật gửi `04` cho người đang trong trận. Chưa có bằng chứng tĩnh ⇒ không đổi.

---

## 3. G3 — byte confirm của `0B 01` (C→S) + độ dài thật của `0B 01` (S→C)

### 3.1 C→S

- Call-site duy nhất tìm được: `00590ba8:157-163` (nút "rời trận") ⇒ đặt `*(byte*)(*(int*)gvar_007DA5AC + 0x130) = 3` rồi gọi builder sub `1` ⇒ **frame `0B 01 03`**.
- Điều kiện tiên quyết: `player[0x376] == 4` (xem §2.4).
- ⇒ Contract `handle_leave_battle` (`src/server/handlers/battle.rs:48-65`) với `confirm == 3` là **đúng**.
- Ghi chú: thân builder sub 1 (`0x780511`) không nằm trong export hiện tại; suy ra byte payload đọc từ `+0x130`
  dựa trên pattern sub 0/1 ghi nhận trước đó. Mức bằng chứng: **A cho giá trị 3, B cho vị trí byte** → xem TODO §11.

### 3.2 S→C `0B 01` = `0064bb24`

- `content[2] = row` (bound `≤ 3`), `content[3] = col` (bound `≤ 4`) ⇒ độ dài tối thiểu **4 byte** (`0B 01 <row> <col>`).
- **Byte thừa không được đọc** ⇒ frame 5 byte của Rust (`reposition` = `F44405000B01 r c 00`) là hợp lệ, không cần sửa.
- Frame 4 byte (`clear_pet_cell` = `F44404000B01 r^1 c`) cũng hợp lệ.

⇒ **G3 đóng tĩnh.** Không cần capture.

---

## 4. G4 — `17 06` byte-exact; **Rust hiện tại ĐÃ ĐÚNG**

Reader `0076b458` (`asm:25-182`):

| đọc | lệnh | wire offset (`content`) | biến |
|---|---|---|---|
| id | `Copy(RP,2,2)` → `FUN_0077eb9c` | `content[2..3]` (u16 LE) | `uVar6` → `EDX` của worker |
| count | `Copy(RP,4,2)` → `FUN_0077eb9c` | `content[4..5]` (u16 LE) | `uVar7` → `ECX` |
| attr1 | raw | `content[6]` | `bVar1` |
| attr2 | raw | `content[7]` | `bVar3` |
| attr3 | raw | `content[8]` | `bVar4` |
| attr4 | raw | `content[9]` | `bVar5` |
| texp | `Copy(RP,10,4)` → `FUN_0077ed68` | `content[10..13]` (u32 LE) | `local_20` |

→ payload sau `17 06` = **12 byte**, content = **14 byte**, `LEN = 0x0E` (không phải `0x10` như tài liệu cũ nghĩ).

Worker `FUN_0072b6c8(player, id, count, texp, derived, attr4, attr3, attr2, attr1)` — **thứ tự 4 byte attr bị đảo khi gọi** (`asm:103-114` push theo thứ tự attr1,2,3,4 ⇒ stack arg đọc ngược). Đối chiếu `17 05` (`0076bfc8`, record 12B `[slot][id:2][count][doben][long][giatri+100][khang][texp:4]`, vtable call `arg6..arg9 = rec7, rec6, rec5, rec4`) suy ra ánh xạ:

```
content[6] = doben (arg9 = rec4)
content[7] = long_val (arg8 = rec5)
content[8] = giatri_long + 100 (arg7 = rec6)
content[9] = khang (arg6 = rec7)
```

`derived` = `FUN_0065b580(gvar_007DA660, id, 0, texp)` — **tính trong client, không có trên wire** (giống hệt `17 05`).

So sánh Rust `src/server/handlers/inventory.rs:70-85`:

```rust
body = "{count}{00}{doben}{long}{(giatri+100)}{khang}{le32(texp)}"
full = "1706" + le16(id) + body
```

| offset | client | Rust | kết luận |
|---|---|---|---|
| `[2..3]` | id u16 | `le16(id)` | ✔ |
| `[4..5]` | count **u16** | `count` + `00` (count ≤ 0xFF nên hi byte = 0) | ✔ |
| `[6]` | attr1 = doben | doben | ✔ |
| `[7]` | attr2 = long | long_val | ✔ |
| `[8]` | attr3 = giatri+100 | giatri_long+100 | ✔ |
| `[9]` | attr4 = khang | khang | ✔ |
| `[10..13]` | texp u32 | `le32(texp)` | ✔ |

⇒ **G4 đóng: không sửa code, không sửa test** `tests/economy_wire_test.rs` (prefix `F4440E001706` là đúng).
Claim cũ "Rust đặt id tại `RP[0..1]`, cần ≥14 B payload" là **sai** do lẫn quy ước index.

---

## 5. (Mục 3 của §1.3) — Ngữ nghĩa kind của sub `08` và sub `0x0A`

### 5.1 Cấu trúc dữ liệu dùng chung

```
struct ActionHolder {  byte +0x24 = kind;  dword +0x1E = target id;  word +0x22 = index/aux; }
wire sub 08 / 0x0A = <sub> <kind:1> <le32 target> <le16 index>      (xác minh ở §1.2)
```

### 5.2 Bảng kind (mỗi dòng: hàm build + điều kiện client-side)

| kind | hàm | nguồn target | điều kiện client trước khi gửi | ngữ nghĩa |
|---:|---|---|---|---|
| `09` | `0064ea20`, `0064efa0:222` | `gvar_007D9D34+0x7B` (click) | từ chối nếu target `0x376==4`; `FUN_004c9bf0(target+4) ∉ 5..8`; `|Δx|<0x97 && |Δy|<0x97`; team (`+0x5F0`) **khác** team mình; nếu target mang cờ (`+0x35F==2`) thì target = `+0x548` (người mang cờ) | **tấn công người chơi** (PK/map-attack) |
| `0x0A` | `0064ed14` | `gvar_007D9FB4[idx].id` (≤200) | `|Δx|<0x97 && |Δy|<0x97` | **tấn công thú nuôi** (theo index) |
| `0x0B` | `0064edf8` | `gvar_007D9FB4[idx].id` | chỉ gate hộp thoại `player+0x380` | thao tác thú theo index (không check khoảng cách) |
| `0x0C` | `0064eecc` | `gvar_007D9FB4[idx].id` | như trên | thao tác thú theo index |
| `0x0E` | `0064efa0:141` | `gvar_007DA300[idx].id` (≤800) | **cùng** team mình; team `==1` hoặc `==2` → `FUN_00509cac` (mời/thách đấu), có message kèm | **thách đấu / mời cùng đội** |
| `12` | `0064f60c` | `gvar_007DA10C[idx].id` (≤10) | gate hộp thoại | thao tác nhóm thứ 3 (bảng `gvar_007DA10C` — cùng bảng mà `0B 04` type `0x10` ghi) |
| `13` | `0064f6e4` | `gvar_007DA10C[idx].id` | như trên | như trên |
| `15` | `0064f7bc` (**sub `0x0A`**) | `gvar_007DA300[idx].id` | `player+0x1458 == 2` && `*gvar_007D9D5C < 2`; loại 5..8 bị chặn; xử lý cờ như kind 9 | **cướp cờ / tương tác đặc biệt** |

Call-site: `0064ea20` ← `0070d43c` (`0070d5cf`) và `0073a310`; `0064efa0`/`0064f7bc` ← `0070d43c` (`0070d5db`, `0070d5e7`) — tất cả đều là **dispatcher click trên bản đồ**, không phải handler của lưới trận.

### 5.3 ✅ KẾT LUẬN

`0B 08` / `0B 0A` là **kênh thao tác map/PK ngoài trận** (tấn công, tấn công thú, thách đấu, cướp cờ), **không phải** lệnh trận trên lưới. Toàn bộ guard nằm phía client (team, khoảng cách `0x97`, loại mục tiêu, hộp thoại) ⇒ server chỉ cần:

1. **Nhận & bỏ qua** (không được crash/log spam) — mặc định hiện tại;
2. Nếu muốn hỗ trợ PK/cướp cờ thì mở ticket riêng, dùng `kind == 9` (tấn công player) và `kind == 0x15` (cướp cờ) làm trigger mở trận.

⇒ Không nên dùng sub 8 làm kênh "lệnh trận" trong spec/ticket.

---

## 6. G9 — Layout `0F 08` đầy đủ (reader `007a5010`)

Record base `L` (content offset) bắt đầu tại `content[2]` cho thú đầu tiên; client lặp `while L < len`.

| offset (tương đối record) | kích thước | đích trong client | ý nghĩa suy ra |
|---|---|---|---|
| `+0x00` | 1 | index vào mảng `player+0x57C[i*4]` (bound ≤ 4) | `stt` (ô thú 1..4) |
| `+0x01..0x02` | 2 | `FUN_0071c3a0(player, stt, value, 0)` | id/npc-id (u16) |
| `+0x03..0x06` | 4 | `pet+0x3FC` | **EXP** (khớp `08 01 24`) |
| `+0x07` | 1 | `pet+0x3FA` | **LEVEL** |
| `+0x08..0x09` | 2 | `pet+0x3EA` | HP |
| `+0x0A..0x0B` | 2 | `pet+0x3EC` | SP |
| `+0x0C..0x0D` | 2 | `pet+0x3EE` | int1 |
| `+0x0E..0x0F` | 2 | `pet+0x3F0` | atk |
| `+0x10..0x11` | 2 | `pet+0x3F2` | def |
| `+0x12..0x13` | 2 | `pet+0x3F4` | agi |
| `+0x14..0x15` | 2 | `pet+0x3F6` | hpx |
| `+0x16..0x17` | 2 | `pet+0x3F8` | spx |
| `+0x18` | 1 | `pet+0x55D` | byte thừa/reserved |
| `+0x19` | 1 | `pet+0x55E` | fai |
| `+0x1A` | 1 | `pet+0x55F` | quest |
| `+0x1B..0x1C` | 2 | `pet+0x400` | skill point |
| `+0x1D` | 1 | — | `nameLen` (≤ 0xFF; ghi vào `pet+9` tối đa 0x11 byte ký tự) |
| `+0x1E..+0x1D+n` | n | `pet+9` (`_PStrNCpy` 0x11) | tên |
| `+0x1E+n .. +0x20+n` | 3 | `pet+0x561..0x563` | 3 byte level kỹ năng (vòng lặp `asm:410-450`, chạy 3 lần) |
| `+0x21+n + 10*(i-1)` , `i=1..6` | 2 + 3 + 4 | `FUN_0072fa98` (worker) | **6 entry × 10 byte**: `[id:u16][b][b][b][value:u32]`; `derived = FUN_0065b580(gvar_007DA660, id, 0, value)` |
| `+0x5D+n` | 1 | `pet+0x565` | tail 1 |
| `+0x5E+n` | 1 | `pet+0x566` | tail 2 |
| `+0x5F+n .. +0x62+n` | 4 (bound ≤ 0xFFFF) | `pet+0x568` | tail value (chỉ word thấp dùng) |

**Stride record = `n + 0x63`** — `asm:758-778`: `EAX = nameLen + 0x1E + 3 + 0x3C + 1 + 1 + 4 = nameLen + 0x63`, rồi `ADD [EBP-0x14],EAX`. Tự kiểm: `0x1E` + `n` + `3` + `6×0x0A` + `6` = `n + 0x63` ✔.

### 6.1 Delta so với Rust `spawn.rs:422-464` (`pet_stat_entry`)

| phần | Rust hiện tại | client | hậu quả |
|---|---|---|---|
| id | `le32(p.id)` tại `+1..4` | `u16` tại `+1..2`, rồi `u32` EXP tại `+3..6` | mọi field từ `texp` trở đi lệch **+2 byte**; record pet thứ 2+ lệch tiếp |
| tail | `7×00` + `lv_skill[3]` + `4×00` = **12 byte** | **6 byte** (`b1`, `b2`, `le16(v)`) | record dài hơn 6 byte ⇒ lệch tích luỹ |

⇒ `pet_stat_entry` phải sửa thành:

```
stt:1 | le16(id) | le32(texp) | lv:1
| le16(hp) le16(sp) le16(int1) le16(atk) le16(def) le16(agi) le16(hpx) le16(spx)
| 00 | fai | quest | le16(skill_point) | nameLen | name
| lv_skill[0..2] (3 byte)
| 6 × encode_entry_10(slot = stt*10 + 1..6)
| b1 | b2 | le16(v)            // chọn b1 = lv_skill[3] để giữ ý nghĩa cũ, 3 byte còn lại = 0
```

Ghi chú ngữ nghĩa `b1/b2/v`: client lưu vào `pet+0x565 / +0x566 / +0x568`; `pet+0x568` được derive từ `+0x136B…+0x13E6` (bảng "học skill") ở frame `05 03` ⇒ nhiều khả năng là **tiền/số lượng skill point thú**. Mức (C) — không chặn sửa layout.

---

## 7. G8 — `05 03` self-snapshot: **Rust ĐÃ ĐÚNG** (claim "lệch 4 byte" là sai)

Reader `0072e9a0` (`0072e9a0_FUN_0072e9a0.c:150-384`) đọc (offset `content`):

| offset | size | đích |
|---|---|---|
| `0x13` | 1 | `+0x3FA` LEVEL (raw `RP[0x12]`) |
| `0x14..0x17` | 4 | `+0x3FC` EXP |
| `0x18..0x19` | 2 | `+0x400` skill point |
| `0x1A..0x1B` | 2 | `+0x402` attribute point |
| `0x1C..0x1F` | 4 | `+0x440` **tiengtam** (+ `FUN_0065b30c` → `+0x43E`) |
| **`0x20..0x21`** | 2 | **`+0x404` MaxHP** |
| **`0x22..0x23`** | 2 | **`+0x406` MaxSP** |
| `0x24..0x27` … `0x38..0x3B` | 4 × 6 | `+0x40C, +0x410, +0x414, +0x418, +0x41C, +0x420` (atk2/def2/int2/agi2/hpx2/spx2) |
| `0x3C..0x3D`, `0x3E`, `0x40`, `0x42`, `0x44` | 2 × 5 | `+0x430, +0x432, +0x434, +0x436, +0x438` |
| `0x46..0x49` | 4 | `+0x1334` (derive `+0x1330`) |
| `0x4A`, `0x4B` | 1 + 1 | `+0x1305`, `+0x1306` |
| `0x4C..0x4F` | 4 | `+0x136B` (derive `+0x1373`) |
| `0x50..0x53` | 4 | `+0x137E` (derive `+0x1386`) |
| `0x54..0x57` | 4 | `+0x1391` (derive `+0x1399`) |
| `0x58..0x5B` | 4 | `+0x13A4` (derive `+0x13AC`) |
| `0x5C..0x5F` | 4 | `+0x13CB` (derive `+0x13D3`) |
| `0x60..0x63` | 4 | `+0x13DE` (derive `+0x13E6`) |
| `0x64..0x67` | 4 | `+0x13B7` (derive `+0x13BF`) |
| `0x68..0x6B` | 4 | **`+0x5F0` = TEAM** (dùng để tính `+0x47C` qua `FUN_00759050`) |
| `0x6C` | 1 | `+0x1308` (raw `RP[0x6B]`) |
| `0x6D..0x6E` | 2 | `+0x408` |
| `0x6F..0x70` | 2 | `+0x40A` |
| **`0x71` …** | 3 × N | **danh sách kỹ năng**: `n = (Len(RP) − 0x70) / 3`, mỗi entry = `le16(skillId)` + `level:1` (`0072e9a0:332-383`) |

Rust `spawn.rs::stats()` (`~:150-212`) đặt:

```
content[2] thuoctinh:1, [3..18] 8×le16, [19] lv, [20..23] texp,
[24..25] skill_point, [26..27] point, [28..31] le32(tiengtam),
[32..33] hp_max, [34..35] sp_max, [36..59] 6×le32,
[60..69] "F401"×5 (=500 ×5), [70..112] 43×00, [113=0x71] skill list
```

⇒ khớp **100%**: `hp_max` đúng `content[0x20]`, `sp_max` đúng `content[0x22]`, header = 111 byte ⇒ skills bắt đầu đúng `0x71` với entry 3 byte (`skill_list()` ghi `le16(id)` + `lv` ✔).

Lưu ý nhỏ (mức C, chưa cần sửa): `content[0x68..0x6B]` là **TEAM** mà Rust gửi `0` — client dùng nó cho `FUN_00759050(...)+0x47C`. Nếu sau này cần hiển thị đúng phe thì phải gửi team thật.

⇒ **G8 đóng tĩnh** (không cần capture).

---

## 8. G6 + G7 — Bảng EXP `FUN_0065b0dc`

Signature thực tế: `FUN_0065b0dc(table, level /*DL→[EBP-5]*/, reborn /*CL→[EBP-6], 0..3*/)` →
`ROUND(pow(level, k[reborn])) + *(uint*)(*(*gvar_007DA6B8) + 4 + 0xA8)` (asm `:17-52, 53-86, 87-120, 121-157`).

### 8.1 Hằng `k[reborn]` (giải mã đúng thứ tự push x87)

| reborn | nhánh | hằng 80-bit | giá trị |
|---:|---|---|---|
| 0 | `0x65B101` | **đọc từ bộ nhớ**: `FLD double [EAX+0x80]` với `EAX = *(*gvar_007DA6B8)+4` ⇒ nạp từ `Data/Formula.Dat` tại payload `0x80` (`w28`, double IEEE-754: `9A 99 99 99 99 99 07 40`) | **2.9** |
| 1 | `0x65B16D` | `PUSH 0x4000 / 0xC0000000 / 0x0` ⇒ mantissa `0xC000000000000000`, exp `0x4000` | **3.0** |
| 2 | `0x65B1D5` | `PUSH 0x4000 / 0xC3333333 / 0x33333333` ⇒ mantissa `0xC333333333333333` (÷2^63 = 1.525), exp `0x4000` | **3.05** |
| 3 | `0x65B23D` | `PUSH 0x4000 / 0xC0000000 / 0x0` | **3.0** |

Cách tính: `value = (mantissa / 2^63) × 2^(exp − 0x3FFF)`.

### 8.2 ✅ Cột dữ liệu

**Cả 4 nhánh đều đọc cùng một cột `*(uint*)(table + 0xAC)`** (`asm:38, 72, 106, 140` — đều `MOV EAX,[EAX+0xA8]` sau khi `EAX = *gvar + 4`).
Dữ liệu này được client nạp trực tiếp từ `Data/Formula.Dat` tại offset payload `0xA8` (trường `extra_exp` kiểu `i32` = **`5`**).

⇒ Claim cũ "client không có `+5`" là **sai**: client có cộng `extra_exp = 5` vào EXP sau khi tính `round(pow(level, k))`.
Và claim "client có 4 cột `+0xA8..+0xB4`" là **sai**: client chỉ dùng duy nhất 1 giá trị `extra_exp` chung cho cả 4 nhánh reborn.
Rust `src/data/texps.rs` đã chuẩn hóa: `k = [2.9, 3.0, 3.05, 3.0]`, cộng `TEXP_BASE_DEFAULT = 5`, dùng rounding half-away-from-zero khớp client.

### 8.3 ✅ KẾT LUẬN G6/G7

- G6: `k = [2.9, 3.0, 3.05, 3.0]` đã được xác minh **chính xác 100%**: nhánh reborn 0 đọc trường `w28 = 2.9` từ `Data/Formula.Dat`; các nhánh reborn 1..3 là hằng inline trong mã máy `0065b0dc.asm.txt`.
- G7: **không có cột thứ 4**; cả 4 nhánh đều dùng chung giá trị `extra_exp = 5` tại `Formula.Dat` `0xA8`; `reborn.min(2)` cũ đã được sửa để reborn 3 dùng `k = 3.0` độc lập.
- Code Rust `src/data/texps.rs` hiện tại đã bám sát chính xác quy tắc này.

---

## 9. G2 — Bảng nhảy builder `0B` sub 0..4

**Đã đóng**: Round-4 export có `asm:744` (`JMP dword ptr [EAX*0x4 + 0x780440]`) và
`/mnt/d/VUDT/GIT_PCC/test/ts_decompile/redump/cs0b_780440_jmptbl_5xDWORD.hex`
(5 entry: `{0x78048D, 0x780511, 0x780595, 0x780671, 0x780721}` cho sub 0..4).
Gap G2 trong `04_battle_end_rewards.md` nói "chưa dump" → **đã lỗi thời**.

---

## 10. Bảng trạng thái các gap `G1..G11`

| ID | Nội dung | Trạng thái sau phiên này | Hành động |
|---|---|---|---|
| **G1** | kind self `0B FA` = `04`; reset `0B 00` | ✅ **ĐÓNG TĨNH** — kind = `04` (chuỗi bằng chứng §2.4) | Sửa `packets.rs:11` (`03`→`04`) + `construction.rs:444` (`05`→`04`) |
| **G2** | bảng nhảy `0x780440` | ✅ **ĐÓNG** (đã có hex Round-4) | Xoá khỏi gap list |
| **G3** | `0B 01` confirm `3` + độ dài S→C | ✅ **ĐÓNG TĨNH** — confirm `3`, S→C ≥ 4 byte, byte pad vô hại | Không sửa code |
| **G4** | `17 06` layout | ✅ **ĐÓNG — Rust đã đúng** | Bỏ ticket sửa `17 06`; giữ test `F4440E001706` |
| **G5** | `tientrang` dùng `17 1E` hay `1E 01` | ✅ **ĐÓNG TĨNH** — `1E 01` = ghi vào container `player+0xA48` (51 slot, record 12B); `17 1E` = stall homdo (`+0x9E0`, 26 slot, record 5B). Rust dùng `1E01` + `encode_entry_12` ⇒ đúng (xem §13) | Chỉ còn xác nhận số slot bằng capture |
| **G6** | hằng `k[]` | ✅ **ĐÓNG HOÀN TOÀN (4/4)** — `k = {2.9, 3.0, 3.05, 3.0}`; reborn 0 đọc từ `Formula.Dat` (+0x80) = `2.9` | Rust `texps.rs` đã khớp chuẩn |
| **G7** | cột `reborn == 3` | ✅ **ĐÓNG** — 1 cột chung `table+0xAC` (`Formula.Dat` +0xA8 = `5`), cộng `+5` mỗi cấp | Rust `texps.rs` đã hỗ trợ 4 cấp reborn |
| **G8** | tail `05 03` | ✅ **ĐÓNG — Rust đã đúng** (khớp từng offset) | Không sửa code |
| **G9** | layout `0F 08` | ✅ **ĐÓNG** — layout đầy đủ + stride `nameLen + 0x63` | Sửa `spawn.rs::pet_stat_entry` theo §6.1 |
| **G10** | không có frame "battle result" | ✅ **ĐÓNG TĨNH (bằng chứng cấu trúc)** — vòng đời trận chỉ do 4 frame điều khiển; không tồn tại sub nào mang (thắng/thua, lượt, thời gian, điểm). Xem §14 | Chỉ cần capture xác nhận thực tế |
| **G11** | `LAB_00644A18` (callback trễ) | ✅ **ĐÓNG** — body có trong export: giải phóng fight-field, `battleState = 0`, và **gửi C→S `13 02`** khi thú chết (`pet+0x3EA ≤ 1`). Xem §15 | Rust đã có handler `13 02`; thêm test |

---

## 11. TODO cho phiên tiếp theo

1. ~~Đồng bộ `spec.md` §1.3~~ — **đã làm** (xem `spec.md` §1.3/§1.4).
2. ~~Điều chỉnh ticket~~ — **đã làm**: 01, 03, 04, 07, 11, 14 đã viết lại; thêm ticket **15** (kind self = `04`).
3. **Bằng chứng còn thiếu (để nâng B → A)**:
   - Thân builder sub 0/1 (`0x78048D`, `0x780511`) — chưa có trong export; cần dump vùng `0x780480..0x780595`
     (để xác nhận byte payload của `0B 01` và call-site `005eb530`).
   - Ngữ nghĩa `0F 08 +0x01..0x02` (`FUN_0071c3a0`).
   - Wire timing của `35 03` (`force_state7`), `35 0E` (`unit_action`), `35 0F` (`battle_unit_status`).
4. **Các capture còn đáng làm** (ticket 14): giá trị `0B 04` show-on-map, kind của record không-self,
   xác nhận thực tế các frame trong luồng kết thúc trận (để phủ G10 ở mức A).
5. **Không commit source** (theo `AGENTS.md`).

---

## 12. G5 — Frame nào cho kho/tiền trang? (`1E 01` vs `17 1E`)

### 13.1 `1E 01` = ghi vào container `player+0xA48` (51 slot) — `FUN_007707a0`

- Dispatcher: opcode `0x1E` = `case_030_00792ACC` → sub `1` → `FUN_007707a0` (`007707a0:78-192`).
- `n = (Len(RP) - 1) / 12`; record 12B **giống hệt `17 05`**:

```
content[.. +0]  slot:1        (bound ≤ 0x32 = 50 ⇒ container có 51 ô, index 0..50)
content[.. +1..2] code:u16 LE
content[.. +3..7] 5 byte attr (đảo khi gọi vtable, cùng pattern 17 05)
content[.. +8..11] value:u32 LE
derived = FUN_0065b580(gvar_007DA660, code, 0, value)   (client tự tính)
target = *(int *)(player + 0xA48 + slot*4)
```

### 13.2 `17 1E` = stall/gán hàng trên homdo — `FUN_00730f4c`

- Dispatcher: opcode `0x17` = `case_020_007902FB` → sub `0x1E` → `FUN_00730f4c`.
- Layout **khác hẳn**: `nameLen:1` + tên + `1 byte` (ô stall) + N × **5 byte** `[slot:1][code:u32 LE]`,
  đích = `player + 0x9E0 + slot*4` (bound ≤ `0x19` = 25 ⇒ **26 ô = homdo**), và handler còn tạo form
  `"Stall" + itoa(slot)` (`FUN_007c9b38`), gọi `FUN_0071e164(player, name, slot)` ⇒ **UI bán hàng**, không phải kho.

### 13.3 ✅ KẾT LUẬN G5

- **Kho/tiền trang = `1E 01`** với record 12B slot-led ⇒ Rust `OPCODE_TIENTRANG = "1E01"` +
  `dump_container(..., StorageType::Bank)` → `encode_entry_12` là **đúng byte-exact**.
- `17 1E` **không** phải kho; `17 2F` (tuideo) và `17 66` (lưu lang) cùng họ record 12B slot-led.
- Số slot: client `+0xA48` nhận `slot ≤ 50` (51 ô); Rust `StorageType::Bank.capacity() = 50`
  dùng slot `1..=50` ⇒ **an toàn** (chỉ không dùng được ô 0 và ô 51). Không cần sửa.
- Câu hỏi còn lại (mức C, chỉ xác nhận bằng capture): server thật có dùng ô `0` và ô `50/51` không.

## 13. G10 — Không có frame "kết quả trận" (chứng minh bằng cấu trúc)

### 14.1 Sub map của các họ frame trận

| opcode | dispatcher | sub được xử lý |
|---:|---|---|
| `0x0B` | `0078d5d1` | `0, 1, 3, 4, 5, 6, 7, 9, 0x0A, 0x0B, 0x0C, 0xFA` (không có `2`) |
| `0x33` | `case_051_007956B9` | **chỉ sub 1** (`FUN_00547c84`) |
| `0x35` | `case_046_007952AB` | `1, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F` (`FUN_007952ab`) |
| `0x3C` | `case_053_0079575C` | **chỉ sub 1, 2, 3, 4** (`00541098`, `005408f4`, `0053fd78`, `0053f8fc`) |

*Lưu ý về nhầm lẫn Case Index vs Opcode:*
Dispatcher chính `FUN_0078a89c` dùng bảng tra 2 tầng: `byte[opcode + 0x78A8EE]` → case index → `jump[index*4 + 0x78A9B6]`.
- Opcode `0x35` có case index là **`46`** (`0x2E`), target là **`FUN_007952ab`**.
  - Sub 3: gọi `FUN_0064cfec` (`force_state7`, animation state 7).
  - Sub 0x0E: gọi `FUN_0064fbd8` (unit action / vtable `unit + 0x1c`: `[35][0E][row][col][val_u16_le]`).
  - Sub 0x0F: gọi `FUN_0064fd34` (unit status buff/debuff: `[35][0F][row][col][status_char][flag_or_turns][val_u16_le]`).
- Opcode `0x3C` có case index là **`53`** (`0x35`), target là `FUN_0079575c` (trước đây bị nhầm case 53 = opcode 0x35).

Do đó, `35 0E` và `35 0F` **có handler thật trong client**, không phải no-op.
Và không họ nào có sub mang (thắng/thua, số lượt, thời gian, điểm).

### 14.2 Bảng chuyển trạng thái trận (`battleState = *(gvar_007DA37C + 0xC)`)

| ghi | giá trị | nơi ghi | nghĩa |
|---|---:|---|---|
| `006438ac:271` (frame `0B FA`, nhánh self) | `2` | mở trận |
| `00641fa4:70` (frame `0B 00`, self & `0x376==4`) | `3` | yêu cầu thoát, cài callback trễ |
| `TExitFight.Create` (`0067e6a0:49`) | `3` | client mở hộp thoại "thoát trận" |
| `00653f18:57` (timer animation, không phải frame) | `4` | unit đang chạy animation rời lưới |
| `006540b0:29,36` (dọn ô lưới, `unit+0x24==0`) | `3` + `0x1C=1` | dọn xong |
| `00644a18:32` (callback trễ sau `0B 00`) | `0` | **thoát trận hoàn toàn** |
| `005de0e0:109`, `007994a4:104` | `0` | reset khi login/load scene |
| `0078a89c:2304` (dispatcher chính) | `1` | vào tiền trạng thái trận |
| `0078a89c:5119` (`case 10` của dispatcher con) | `6` | trạng thái phụ |

### 14.3 ✅ KẾT LUẬN G10

Client **không có** frame kết quả trận. Toàn bộ thông tin phần thưởng đi qua các kênh riêng:
`08 01 …` (stat/exp), `17 06` (vật phẩm vào túi), `1A 01`/`1A 04` (vàng), `05 0A` (LevelUP),
`0F 08` (thú), `35 04` (item rơi), `0B 09` (banner), `0B 07` (bộ đếm).
Nếu UI cần bảng tổng kết thì phải tự dựng bằng `35 0B` / kênh chat, **không** có frame chuẩn.

### 14.4 Hệ quả cho ticket 12

`35 0E` và `35 0F` có handler đầy đủ phía client. Đã xây dựng builder trong `packets.rs` (`unit_action` / `battle_unit_action` và `battle_unit_status` / `unit_status`).
Handler thực sự của `35 03` là `FUN_0064cfec` (gọi `FUN_00647708(unit, 7)`, ép state 7 chuẩn bị tấn công).

## 14. G11 — `LAB_00644A18`: callback trễ sau khi thoát trận

Body có trong export: `functions/00644a18_FUN_00644a18.c` (262 byte), được cài tại
`00641fa4:71-72` (`conn+0x458 = &FUN_00644a18`, `conn+0x45C = param_1`).

```
FUN_00644a18:
  TObject_Free(DAT_0098c63c);  DAT_0098c63c = 0;        // giải phóng fight-field
  *(byte*)(gvar_007DA37C + 0xC) = 0;                    // battleState = 0 (thoát trận)
  if (*(short*)(*(int*)(player + 0x44) + 4) != 0) {      // job/class id khác 0
      uVar2 = FUN_007746ac(gvar_007DA540, jobId);        // tra cứu job
      if (*(char*)(gvar_007DA540 + 0x35 + uVar2*0xB9) == 0x1C) {   // điều kiện job
          stt = *(byte*)(player + 0x12E5);                // ô thú đang theo (1..4)
          if (stt != 0 && *(word*)(pet[stt] + 0x3EA) <= 1) {       // pet HP ≤ 1 = chết
              *(byte*)(gvar_007D9D7C + 0x134 + stt) = 0;
              MOV CL,0x2 ; MOV DL,0x13 ; FUN_0077f414(...)          // gửi C→S `13 02`
          }
      }
  }
```

### 15.1 Quy ước builder C→S (xác nhận ở mức A)

`FUN_0077f414(manager)` nhận **CL = sub**, **DL = opcode chính**, gửi qua `FUN_0051633c`:

| call-site | CL | DL | frame |
|---|---:|---:|---|
| `00590ba8.asm:220-222` (nút rời trận) | `1` | `0x0B` | `0B 01 <byte +0x130 = 3>` |
| `00644078.asm:44-46` | `3` | `0x0B` | `0B 03 …` |
| `0064efa0.asm:320-322` | `8` | `0x0B` | `0B 08 <kind><u32><u16>` |
| `005eb530.asm:604-606` | `1` | `0x0B` | `0B 01 …` (call-site thứ 2, **không** set `+0x130` ⇒ giá trị byte chưa rõ) |
| `00644a18.asm:70-72` | `2` | `0x13` | **`13 02`** |

### 15.2 ✅ KẾT LUẬN G11

- Sau khi client thoát trận (vài trăm ms, qua callback trễ), nếu **thú đang theo đã chết** và job thỏa
  điều kiện `0x1C`, client gửi **`13 02`** = "bỏ/recall thú".
- Rust **đã có** handler: `dispatcher.rs` `OP_BATTLE_PET => pet_actions::handle_pet_summon` và
  `pet_actions.rs:369-376` sub `2` → `active_pet_stt = 0` + echo `F44402001302` ⇒ **khớp**.
- Việc còn lại: thêm test cho nhánh này + ghi chú nghĩa (sub 2 = recall, sub 1 = summon).

---

## 15. Source trail (đường dẫn + dòng)

| # | Nội dung | File / dòng |
|:-:|---|---|
| 1 | Quy ước `_LStrCopy` (hiệu chuẩn 3 frame) | `006438ac_FUN_006438ac.asm.txt:95-103`, `00641fa4_FUN_00641fa4.c:55-58`, `0076bfc8_FUN_0076bfc8.asm.txt:21-207` |
| 2 | `0B FA` record 24B + kind self | `006438ac_FUN_006438ac.c:238-402` |
| 3 | `0B 00` reset `== 4` | `00641fa4_FUN_00641fa4.c:55-105` |
| 4 | `0B 01` confirm `3` + gate `0x376==4` | `00590ba8_FUN_00590ba8.c:157-163` |
| 5 | builder `0B` (sub 8 / 0x0A) | `0077f414_FUN_0077f414.asm.txt:744, 830-883, 907-939` |
| 6 | `17 06` byte-exact | `0076b458_FUN_0076b458.asm.txt:25-182` + `0072b6c8_FUN_0072b6c8.c` |
| 7 | `17 05` record 12B (đối chiếu thứ tự attr) | `0076bfc8_FUN_0076bfc8.asm.txt` + `.c` |
| 8 | kind sub-8 (9, 0x0E) | `0064efa0_FUN_0064efa0.c:46-240` |
| 9 | kind sub-8 (0x0A, 0x0B, 0x0C) | `0064ed14`, `0064edf8`, `0064eecc` |
| 10 | kind sub-8 (0x12, 0x13) | `0064f60c`, `0064f6e4` |
| 11 | kind sub-0x0A (0x15) | `0064f7bc_FUN_0064f7bc.c:31-115` |
| 12 | `0F 08` layout + stride | `007a5010_FUN_007a5010.c:142-423` + `.asm.txt:140-807` |
| 13 | `05 03` layout + skill list | `0072e9a0_FUN_0072e9a0.c:150-383` |
| 14 | Bảng EXP (k + cột) | `0065b0dc_FUN_0065b0dc.asm.txt:1-157` |
| 15 | S→C sub map `0B` | `0078d5d1_FUN_0078d5d1.c:40-203` |
| 16 | Bảng nhảy builder | `redump/cs0b_780440_jmptbl_5xDWORD.hex` |
| 17 | Rust `17 06` | `src/server/handlers/inventory.rs:70-85` |
| 18 | Rust `0F 08` | `src/server/spawn.rs:422-464`, `pet_summary:394-418` |
| 19 | Rust `05 03` | `src/server/spawn.rs:150-221` |
| 20 | Rust marker `0B FA` | `src/battle/packets.rs:9-15`, `src/battle/construction.rs:422-492` |
| 21 | Rust EXP table | `src/data/texps.rs:1-59` |
| 22 | `1E 01` container `+0xA48` 51 slot, record 12B | `007707a0_FUN_007707a0.c:78-192` |
| 23 | `17 1E` stall homdo `+0x9E0` 26 slot, record 5B | `00730f4c_FUN_00730f4c.c:84-168` |
| 24 | Sub map `0x33` / `0x35` (G10) | `case_functions/functions/case_051_007956B9_*.c:20-29`, `case_053_0079575C_*.c:20-39` |
| 25 | Battle-state transition table (G10) | grep `gvar_007DA37C + 0xc\|0x1c` trong `functions/*.c` |
| 26 | `LAB_00644A18` + frame `13 02` (G11) | `00644a18_FUN_00644a18.c:21-64` + `.asm.txt:70-72` |
| 27 | Quy ước builder CL=sub / DL=opcode | `0077f414_FUN_0077f414.asm.txt:18` + call-site `00590ba8:220`, `00644078:44`, `0064efa0:320`, `005eb530:604`, `00644a18:70` |
| 28 | Rust `1E01` + capacity Bank 50 | `src/protocol/mod.rs:44`, `src/db/modern/model.rs:60-67`, `src/server/session.rs:615-669` |
| 29 | Rust handler `13 02` | `src/server/dispatcher.rs:283`, `src/server/handlers/pet_actions.rs:330-380` |