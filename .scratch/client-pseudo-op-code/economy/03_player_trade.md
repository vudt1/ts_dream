# 03 — GIAO DỊCH NGƯỜI CHƠI (P2P TRADE) — `MainOp 0x19`

> **Xem thêm:** [`01_shop_read_flow.md`](./01_shop_read_flow.md) · [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) · [`03_player_trade.md`](./03_player_trade.md) · [`04_stall_vendor.md`](./04_stall_vendor.md) · [`05_special_economy.md`](./05_special_economy.md) · [`06_currency_model.md`](./06_currency_model.md) · [`07_rust_economy_audit.md`](./07_rust_economy_audit.md) · [`08_verification_report.md`](./08_verification_report.md)

> Nguồn: decompile Ghidra của `aLogin.exe` (`/mnt/d/VUDT/GIT_PCC/test/ts_decompile/`),
> dump bổ sung (`redump/`), pseudo-C curated (`/mnt/d/VUDT/GIT_PCC/ts_dream/client_pseudo_c/`).
> Mọi khẳng định layout đều kèm `file:line`. Suy luận đánh dấu **Confidence C**.
> **Kết luận quan trọng (phủ định comment Rust):** `OP_TRADE (0x19)` **KHÔNG** phải
> "Bear-dialect only". xem §6.

---

## 1. Tổng quan luồng

Bộ khung: `FUN_0078a89c` = dispatcher S→C (đọc MainOp, index `byte[0x78A8EE]` → case idx →
`JMP [EAX*4+0x78A9B6]`). MainOp `0x19` → **case 22** → `0x0079157C` (`FUN_0079157c`).
Bộ khung C→S: `FUN_0077f414` (giant builder). Nhận `EAX=obj`, **`DL`=MainOp**, **`CL`=SubOp**;
dispatch `byte[EBP-5]`=MainOp qua bảng `0x77F474` → target `0x77F53C`; **case MainOp 0x19 = idx 23 →
`0x007853CA`**, phân nhánh tiếp theo **`CL`** qua bảng `0x7853E5` → `JMP [EAX*4+0x78540F]`.

```
          CLIENT (aLogin)                          SERVER
              │                                      │
 (1) mở cửa sổ giao dịch  ◄──── S→C 19 01 <id:u32>   │  (Form_Trade show, đối tượng = id)
              │                                      │
 (2) chọn vật phẩm + nhập  │                         │
     vàng, bấm btn_BothOk  ──── C→S 19 02 <gold:u32> + <slotBytes> ──►
              │                                      │
              │  ◄──── S→C 19 03 <caption:u32> + N×<item 11B>   (render vật phẩm đối phương)
              │                                      │
 (3) xác nhận / commit      ──── C→S 19 03 <state:u8>  ──────────►
              │                                      │
              │  ◄──── S→C 19 02 <status:u8>   (toast + đóng cửa sổ)
              │         status 3 = đối phương hủy
              │         status 4 = hoàn thành
              │         status 6 = đối phương không đủ tiền
              │         status 7 = đối phương hết chỗ trữ
              │                                      │
 (4) hủy (đóng form)        ──── C→S 19 01 <id:u32> ─────────────►
```

`Form_Trade` = `TCY_TradeMenu` (VMT `0x607F1C`), ctor `FUN_00608904`.
`btn_BothOk` handler = `FUN_006090a8` (`00608904…c:222-224`).
Chứa **25 `TCY_TradeImage` mỗi bên** (`00608904…c:77-132` nhóm `self[0x4c+i*4]` = `self+0x130+i*4`
nhóm 3; `00608904…c:133-188` nhóm `self[0x66+i*4]` = `self+0x198+i*4` nhóm 4; bound `0x19 < i` ⇒ i = 1..25).

---

## 2. Bảng frame

Quy ước: `P[0]` = byte MainOp trong payload; `RP` = RestPayload sau khi cắt MainOp ⇒ `RP[0]=SubOp`.
Offset trong bảng đếm từ `RP[0]` (=SubOp).

### 2.1 C→S (client gửi) — builder `FUN_0077f414`, MainOp `0x19`, index `CL` ở `0x7853E5`

| CL | Target builder | Payload (hex offset từ `RP`) | Nội dung | Conf | Evidence |
|---|---|---|---|---|---|
| `0x01` | `0x0078543B` | `RP[0]=01`, `RP[1..4]` = u32 LE | `TTradeManage[0x04]` (id vật phẩm/thú) | A | `redump/cs_op19_dispatch_cases.asm.txt:53-89`; caller `FUN_0075ad70` (`0075ad70…asm.txt:104-106`) |
| `0x02` | `0x007854BF` | `02`, u32 LE gold, rồi N×1B slot | **offer gold + danh sách slot** | A | `redump/cs_op19_dispatch_cases.asm.txt:90-130`; builder `FUN_0077ee84` cho u32LE, `FUN_00404148` cat 3 |
| `0x03` | `0x00785569` | `03`, `RP[1]` = 1 byte `Form_Trade[0x228]` | commit/lock (state byte) | C | `redump/cs_op19_dispatch_cases.asm.txt:131-166` |
| `0x0A` | `0x007855ED` | `0A`, u32 LE `TTradeFNpcManage[0x04]` | NPC-trade cancel | A | `redump/cs_op19_dispatch_cases.asm.txt:167-201`; caller `FUN_0075b66c` |
| `0x0B` | `0x00785671` | `0B`, u32 LE, 1 byte `[0x204]` | NPC shop/stall frame | C | `redump/cs_op19_dispatch_cases.asm.txt:202-281` |
| `0x0C` | `0x0078572D` | `19 0C` + 1B `Form_TradeFNpc[0x205]` (tổng 3 byte) | NPC cancel/confirm (state byte) | C | `redump/cs_op19_dispatch_cases.asm.txt:246-281` |
| `0x14` | `0x007857B1` | `19 14` + u32 LE + u32 LE + 10×1B (5 cặp byte từ inventory entry `+0x131`/`+0x108`); tổng 20 byte; **không tìm thấy caller** | inventory/transfer detail | C | `redump/cs_op19_dispatch_cases.asm.txt:282-401` |
| `0x15` | `0x007859F0` | `19 15` — **chỉ 2 byte header, không body** (`_LStrAsg` rồi enqueue) | (rỗng/khoá) | C | `redump/cs_op19_dispatch_cases.asm.txt:402-424` |
| `0x1F` | `0x00785A45` | `19 1F` + u32 LE `gvar_007D9E70[0x144]` | stall/vendor; **không tìm thấy caller** | C | `redump/cs_op19_dispatch_cases.asm.txt:425-458` |
| `0x29` | `0x00785AC8` | `29`, 1B `player[0x1510]` = `(player[0x440]&0xFF)^0x7B` | **nonce ACK** | A | `redump/cs_op19_dispatch_cases.asm.txt:459-494`; `0074df70…c:70-79` |

**Bằng chứng phủ định (ASM-level) về "aLogin chỉ có 1 frame 0x19 C→S":**
Sweep `grep -rl "MOV DL,0x19" functions/*.asm.txt` → đúng **7** file; 6 phát `CALL 0x0077f414`
(file thứ 7 `0064cae0` chỉ nạp `DL=0x19` cho mục đích khác, không gọi builder):

| File | CL | Line asm |
|---|---|---|
| `0075ad70` (TTradeManage) | `0x01` | 104–106 |
| `006090a8` (Form_Trade `btn_BothOk`) | `0x02` | 65–67 |
| `0075b66c` (TTradeFNpcManage) | `0x0A` | 104–106 |
| `0060b230` (Form_TradeFNpc) | `0x0B` | 224–226 |
| `0060b164` (Form_TradeFNpc) | `0x0C` | 12–14 |
| `0074df70` (player) | `0x29` | 29–31 |

⇒ **aLogin gửi C→S `0x19` ở 6 sub-op khác nhau** (không chỉ ACK). Comment ở
`src/server/dispatcher.rs:299-305` bị **phủ định** (xem §6).

### 2.2 S→C (server gửi) — handler `FUN_0079157c` @ `0x0079157C`, case 22

| SubOp | Handler | Payload (offset từ `RP`) | Nội dung | Conf | Evidence |
|---|---|---|---|---|---|
| `0x01` | `func_0x0075af6c` | `01`, `RP[1..4]` u32 LE | **mở Form_Trade** với object id = u32 | A | `case_022…c:28-30`; `0075af6c…c:42-44` |
| `0x02` | `FUN_0075aff4` | `02`, `RP[1]` = status u8 | toast + đóng form | A | `case_022…c:31-33`; `0075aff4…c:50-89` |
| `0x03` | `func_0x0075b3ec` | `03`, `RP[1..4]` u32, rồi N×11B item | **render danh sách vật phẩm** | A | `case_022…c:34-36`; `0075b3ec…c:65-176` |
| `0x0A` | `func_0x0075bf7c` | `0A`, `RP[1..4]` u32 | NPC: mở form NPC | A | `case_022…c:37-39`; `0075bf7c…c:37-39` |
| `0x0B` | `func_0x0075b8e0` | `0B`, `RP[1]` = status u8 | NPC: toast status | A | `case_022…c:40-42` |
| `0x0C` | `func_0x0060bbe0` | `0C`, blob dài | NPC: form chi tiết vật phẩm | C | `case_022…c:43-45` |
| `0x14` | `func_0x0062e41c` | `14`, u32, 1B flag | flag trạng thái equip | C | `case_022…c:46-48` |
| `0x15` | `func_0x00735184` | `15`, u32, … | tạo `THuman` (VMT `0x70B1C4`) | C | `case_022…c:49-51` |
| `0x16/17/18` | `0062e11c/…538/…814` | — | equip/inventory flags | C | `case_022…c:52-60` |
| `0x1F` | `func_0x00728310` | `1F`, u32, name | stall owner name | C | `case_022…c:61-63` |
| `0x20` | `FUN_007281ec` | `20`, name | stall chat line | C | `case_022…c:64-66` |
| `0x21/22` | `00742a50/00742f78` | — | misc player flags | C | `case_022…c:67-72` |
| `0x29` | `func_0x0074df70` | `29`, ≥1B | nonce ACK (nhận & gửi lại) | A | `case_022…c:73-74`; `0074df70…c:70-79` |

Bảng byte `0x7853E5` (42 byte, `CMP EAX,0x29`) chỉ map CL ∈
`{0x01,0x02,0x03,0x0A,0x0B,0x0C,0x14,0x15,0x1F,0x29}`; phần còn lại → target `0x0078A4F2` = no-op.
Evidence: `client_pseudo_c/cs_op19_byte_map_0x7853e5.hex`, `redump/subtable_0x7853E5.hex`,
`redump/cs_op19_targets_0x78540f.hex` (idx 0 = `F2 A4 78 00` = `0x0078A4F2`).

### 2.2b Cơ chế builder `FUN_0077f414` — nguồn gốc duy nhất của mọi frame C→S

> Bổ sung từ tài liệu nguồn `03_trade_and_stall.md` §0.4 (đã hợp nhất 2026-10-02).

`client_pseudo_c/0077f414_FUN_0077f414.asm.txt:8-15`
```
MOV [EBP-0x6], CL      ; byte CAO của param_2  → SubOp
MOV [EBP-0x5], DL      ; byte THẤP của param_2 → MainOp
MOV [EBP-0x4], EAX     ; codec
```
⇒ `param_2` là **Word = `MainOp | (SubOp << 8)`**. Dispatch ngoài theo MainOp
(`:29-33`), dispatch trong theo SubOp. **Mức A**

Toàn bộ frame client gửi lên đều đi qua `TForm1.CY_AddSedQueue` (`0x0051633C`). Đây là
**call-site duy nhất** của hàm gửi trong toàn bộ dump:
```
$ grep -rl "0x0051633c\|TForm1_CY_AddSedQueue" --include=*.c --include=*.asm.txt .
0077f414_FUN_0077f414.asm.txt / .c
cs_op02_dispatch_cases.asm.txt / cs_op17_first_cases / cs_op19_dispatch_cases
cs_op1c_dispatch_cases / cs_unknown_builders_0x77f634
```
⇒ **Mọi frame client gửi lên đều được sinh trong `FUN_0077f414`.** **Mức A**

Khuôn frame 2 byte (MainOp+SubOp) — ví dụ `19 01` (`cs_op19_dispatch_cases.asm.txt:110-152`):
```
0078543b: ss1 = Chr(MainOp)            ; shortstring [EBP-0x34] = [01][DL]
0078544d: Move(ss1 → buf)             ; @PStrNCpy 0x00402b90
00785552: ss2 = Chr(SubOp)
00785564: PStrCat(buf, ss2, index=2)  ; @PStrNCat 0x00402b60  → buf = "1901"
00785574: LStrAsg(buf → dst)          ; 0x0040402c
007855a3/b5: AddSedQueue(dst)
```
Rồi body thêm field tuỳ sub (xem §2.1). **Mức A**

### 2.3 Toàn map MainOp→case (S→C) — build từ jumptable

`0x19`→22, `0x17`→20, `0x1A`→23, `0x1B`→24, `0x1D`→25, `0x1E`→26, `0x1F`→27, `0x23`→31, `0x0C`→12.
Full table 0x00–0x43 ở `redump/jumptable_byte200_0x78A8EE.hex` ×
`redump/jumptable_0x78A9B6_case_functions.csv`.

---

## 3. Từng bước chi tiết

### 3.1 Bước 1 — Server mở cửa sổ: `S→C 19 01 <u32LE id>`

`FUN_0079157c` case 1 → `func_0x0075af6c(TTradeManage, RP)`:

```c
_LStrCopy(local_c, 2, 4, &local_14);                 // 0075af6c.c:42
local_10 = FUN_0077ef7c(*gvar_007D9D30, local_14);   // :43  → u32 LE
FUN_00609000(*(int **)gvar_007DA3C4, local_10);      // :44
```

`FUN_00609000` (`00609000…c:41-49`):
- `obj = FUN_0070c20c(gvar_007D9D34, id)` — tìm object theo id
- `self[0x88] = obj` (cache handle đối phương)
- `setcaption(gvar_007DA300[id*4] + 9)` — **hiện tên đối phương**
- gọi `vmt[0x20]()` → `Show`

### 3.2 Bước 2 — Client offer (gold + slot): `C→S 19 02 <u32LE gold> <slotBytes…>`

`FUN_006090a8` (btn_BothOk của Form_Trade, `00608904…c:222-224` gán handler):

```c
iVar2 = StrToInt(Form_Trade[0x20c]->Caption);          // 006090a8.c:52  ← ô vàng
if (player[0x12f8] < iVar2) → toast "…"                // :53 (0x12f8 = tiền, cap 9999999)
_LStrClr(&self[0x224]);                                // :57  xoá chuỗi slot cũ
for (i = 1; i <= 0x19; i++)                            // :58-71  ← 25 slot
    if (self[0x198 + i*4]->[0x128] != 0)               // :65   TradeImage có vật phẩm?
        AppendStr(&self[0x224], Chr(i));               // :66-67  ghép byte slot index
FUN_0077f414(gvar_007D9D30, CL=0x02);                  // :72
self[0x229] = 1;                                       // :73  khoá trạng thái
FUN_007b0094(self[0x210], 0);                          // :74  ẩn btn_BothOk
FUN_007b0094(self[0x214], 1);                          // :75  hiện btn_Trade
```

Wire build tại `0x007854BF`:
- `_LStrAsg(dest, Chr(0x19))` → 1 byte header `0x19`
- push `self[0x20c]` caption → `FUN_004099e4` (`StrToInt`) → `FUN_0077ee84` = **u32 LE encoder** (`0077ee84…c`: dựng `local_11..14` = byte0..byte3 rồi `_LStrFromString`)
- push `self[0x224]` (chuỗi slot)
- `FUN_00404148(dest, 3)` = concat 3 đoạn → payload
- `TForm1.CY_AddSedQueue` (`0x0051633C`)

⇒ **Slot index là 1 byte, 1-based, 1..25** (0x01..0x19), không phải offset túi. Conf **A**.

### 3.3 Bước 3 — Server đẩy danh sách vật phẩm: `S→C 19 03 <u32LE> <N×11B>`

`func_0x0075b3ec` (`0075b3ec…c`) — `base` dưới đây là **index 1-based của Delphi**; record `k`
chiếm `RP[0]` 0-based từ `k*11+5` đến `k*11+15`:

```c
_LStrCopy(local_c, 2, 4, &s);                          // :65  RP[1..4] (0-based) = u32
id = FUN_0077ef7c(global, s);                          // :66  u32 LE
IntToStr(id, &local_30);
FUN_007b372c(Form_Trade[0x208], local_30);             // :68  ← set caption = tên đối phương
count = (LStrLen(RP) - 5) / 0xb;                       // :74  ← (len-5)/11  ⇒ len = 5 + 11*N
slot = 1;                                              // :75
for (k = 0; k < count; k++) {
    base = k*11 + 6;                                   // :93  (Delphi 1-based)
    itemId = FUN_0077eb9c(copy(RP, base, 2));          // :93-94   → rec[0..1]  u16 LE
    b2 = RP[base+1];                                   // :105     → rec[2]
    b3 = RP[base+2];                                   // :116     → rec[3]
    b4 = RP[base+3];                                   // :127     → rec[4]
    b5 = RP[base+4];                                   // :138     → rec[5]
    b6 = RP[base+5];                                   // :149     → rec[6]
    longVal = FUN_0077ed68(copy(RP, base+7, 4));       // :156-157 → rec[7..10] u32 LE
    doben = FUN_0065b580(gvar_007DA660, itemId, 0, longVal);  // :158 ← client TỰ TÍNH lại
    FUN_00609910(Form_Trade[0x130 + slot*4], itemId, b2, longVal, doben, b6, b5, b4, b3);
    slot++;                                            // :176
}
```

Header = **5 byte**: `RP[0]` sub-op + `RP[1..4]` u32 id ⇒ công thức `count = (len-5)/11` khớp tuyệt đối
(Evidence `0075b3ec…c:74`).

`FUN_00609910` = `TCY_TradeImage.SetItem(...)` (`00609910…c:77-99`):

| field offset trong TradeImage | nguồn | ý nghĩa |
|---|---|---|
| `+0xE4` (word) | `:77` | `itemId` |
| `+0x4C` | `:79` | bitmap index (`FUN_007c9b38`) |
| `+0x108` (dword) | `:80` | `param_3` = **cnt** |
| caption | `:93` | tên vật phẩm (lookup `gvar_007DA540`) |
| `+0xE8` | `:94` | `param_9` = `b3` = RP[base+2] |
| `+0xE9` | `:95` | `param_8` = `b4` = RP[base+3] |
| `+0xEA` | `:96` | `param_7` = `b5` = RP[base+4] |
| `+0xEB` | `:97` | `param_6` = `b6` = RP[base+5] |
| `+0xEC` | `:98` | `param_5` = **`doben` tính tại client** |
| `+0xF0` (dword) | `:99` | `param_4` = **longVal** (u32) |

⇒ **11-byte record ≠ 12-byte inventory record** (`slot,itemId u16,cnt,doben,long_val,giatri,khang,texp u32`).
Đây là layout **riêng cho offered-item**, không có `slot` byte đầu, `texp` **không** có mặt,
và `doben` **không** nằm trên wire (client tự tính lại từ `itemId` + `longVal`). Conf **A**.

Record layout chuẩn hoá (đã đối chiếu 2 nguồn):
```
offset  len  type      name        (= inventory entry offset, xác nhận bằng FUN_0060936c)
0       2    u16 LE    itemId      entry+0x04  -> TradeImage+0xE4
2       1    u8        cnt         entry+0x05  -> TradeImage+0x108
3       1    u8        flagA       entry+0x08  -> TradeImage+0xE8
4       1    u8        flagB       entry+0x09  -> TradeImage+0xE9
5       1    u8        flagC       entry+0x0A  -> TradeImage+0xEA
6       1    u8        flagD       entry+0x0B  -> TradeImage+0xEB
7       4    u32 LE    longVal     entry+0x10  -> TradeImage+0xF0
```
**Không có `slot` ở đầu record** (slot do byte trong `19 02` chỉ định, xem §3.2).
**Không có `texp`.** **`doben` (entry+0x0C) KHÔNG truyền** — client tự tính.

**Đối chiếu chéo độc lập** — `FUN_0060936c` là hàm refresh **cột bên phải (của tôi)**:
 nó đọc inventory entry `player[0x9E0 + i*4]` (`i = 1..0x19`) rồi chép sang `TradeImage`:

| entry offset | kiểu | -> TradeImage | -> record byte |
|---|---|---|---|
| `+0x04` | u16 | `+0xE4` | `rec[0..1]` |
| `+0x05` | u8 | `+0x108` | `rec[2]` |
| `+0x07` | u8 | (số lượng hiển thị) | — |
| `+0x08` | u8 | `+0xE8` | `rec[3]` |
| `+0x09` | u8 | `+0xE9` | `rec[4]` |
| `+0x0A` | u8 | `+0xEA` | `rec[5]` |
| `+0x0B` | u8 | `+0xEB` | `rec[6]` |
| `+0x0C` (doben) | u8 | `+0xEC` | **KHÔNG truyền** |
| `+0x10` | u32 | `+0xF0` | `rec[7..10]` |

Hai nguồn độc lập (`0075b3ec` phía parse, `0060936c` phía ghi) khớp từng byte ⇒
`longVal` là **u32** ở 4 byte cuối, `doben` **không** có trên wire, record có **đúng 4 byte flag**.
Tên nghiệp vụ của `flagA..D` chưa chốt (Conf C) — `+0x0C` bị skip cho thấy layout entry
không chật liên tục, đừng giả định field nào kề field nào.

### 3.4 Bước 4 — Commit: `C→S 19 03 <1B state>`

`FUN_0077f414` CL=0x03 → `0x00785569`:
- header `0x19` (RP[0]) + sub `0x03` (RP[1])
- `FUN_00403fa0` = `_LStrFromChar` 1 byte từ `Form_Trade[0x228]`
- `FUN_00404148(dest, 3)` concat 3 đoạn → payload = `19 03 <state>`

**State byte = `Form_Trade[0x228]`** (cùng form với `0x229` set ở bước offer).

**Bằng chứng mạnh cho vai trò "commit"** — trong ctor `FUN_00608904`, nút thứ hai của form là
`btn_Trade` với handler `LAB_00609218` (`00608904…c:229-231`), **không nằm trong bộ export**
(`index.csv` không có entry `00609218`; không file `.asm.txt` nào phủ địa chỉ này).
Đây đúng là nút mà `FUN_006090a8` bật lên sau khi ẩn `btn_BothOk` (`006090a8…c:73-75`).
Vì CL=0x03 là builder **duy nhất** trong `FUN_0077f414` không tìm thấy caller nào,
`LAB_00609218` gần như chắc chắn là nơi gọi nó ⇒ **`19 03 <state>` = frame xác nhận (commit) của nút
`btn_Trade`**. Conf **B** (layout byte chắc A; ngữ nghĩa "commit" là suy luận từ vị trí nút + thứ tự ẩn/hiện).

### 3.5 Bước 5 — Kết quả / hủy / lỗi: `S→C 19 02 <status u8>`

`FUN_0075aff4` (`0075aff4…c:50-89`) switch trên `RP[1]` → toast 3000ms, default 1000ms,
rồi `Form_Trade.vmt[0x24]()` (đóng form). Decode VISCII từ `redump/lit_75b1e4_75b3a0.hex`:

| status | addr chuỗi | text (VISCII→NFC) | thời lượng |
|---|---|---|---|
| `0` | `0x0075b1e4` | `Hệ thống đang bận rộn, hãy giao dịch sau nhé` | 3000 |
| `1` | `0x0075b21c` | `Đối thượng đã rớt mạng, không thể giao dịch` | 3000 |
| `2` | `0x0075b250` | `Đối thượng đang bận rộn, không thể giao dịch` | 3000 |
| `3` | `0x0075b288` | `Đối thượng hủy bỏ giao dịch` | 3000 |
| `4` | `0x0075b2ac` | `Song phương giao dịch hoàn thành` | 3000 |
| `5` | `0x0075b2d8` | `Song phương quang cảnh bất đồng, không thể giao dịch` | 3000 |
| `6` | `0x0075b318` | `Đối thượng nào đó không đủ tiền bạc, hủy bỏ giao dịch` | 3000 |
| `7` | `0x0075b358` | `Đối thượng nào đó vật phẩm trữ tồn quá nhiều, hủy bỏ giao dịch` | 3000 |
| `8` | `0x0075b3a0` | (dump cắt — `lit_75b1e4_75b3a0.hex` kết thúc đúng tại `0x75b3a0`) | 3000 |
| default | `0x0075b3cc` | `Hệ thống gián đoạn giao dịch` | 1000 |

Vị trí switch arm: `0075aff4…c:50` (case 0) … `:82` (case 8), `:86-88` (default).
Chuỗi cho NPC variant (`FUN_0075b8e0`, sub `0x0B`) nằm ở `0x0075bbb4`–`0x0075bf5c`,
decode sẵn trong `redump/lit_75bbb4.hex` / `lit_75be30.hex` / `lit_75bf14.hex` / `lit_75bf5c.hex`.
→ **status là 1 byte enum, không phải text ID.** Conf **A**.

### 3.6 Bước 6 — Cancel: `C→S 19 01 <u32LE companionId>`

`FUN_0075ad70` (TTradeManage) là hàm **đóng cửa sổ giao dịch**. Nó kiểm tra nhiều điều kiện
trước khi gửi (`0075ad70…c`):
- nếu đối tượng type `== 4` → toast `LAB_0075aee8` (1500ms) — **không gửi**
- nếu `FUN_004c9bf0(TTradeManage[0x04]) ∈ [2..6)` → toast `DAT_0075af20` — không gửi
- nếu map `== 0x2AEF` hoặc `-0x3D0B` → toast `LAB_0075af4c` — không gửi
- nếu `FUN_00504c9c(...)` true → toast `LAB_0075af4c` — không gửi
- nếu `player[0x1458] != 0` → toast — không gửi
- **else** → `FUN_0077f414(..., CL=0x01)` (`0075ad70…asm.txt:104-106`)

Payload `0x0078543B`: header `0x19`, sub `0x01`, rồi
`EDX = gvar_007DA700->[0x004]` (TTradeManage field 4 = u32 id vật phẩm/thú cưng) →
`FUN_0077ee84` → u32 LE. Conf **A**.

---

### 3.7 Vàng trong giao dịch (gold offer)

- Ô vàng của Form_Trade = editor tại `self[0x20C]` (ctor: `TSe_Editor` thứ 4, `param_1[0x83]` tại
  `00608904…c:210-218`, label mask `0xFFFF` ⇒ chỉ nhận số, `FUN_007b3684(…,2)` + `FUN_007b3834(…,1)`).
  Cặp editor số thứ hai là `self[0x208]` (`00608904…c:201-209`) — ô vàng của đối phương (read-only).
  Hai editor text (mask `0x841`) là `self[0x200]` / `self[0x204]` (`00608904…c:189-200`) — tên đối phương.
- Vàng được gửi **chung trong frame `C→S 19 02`** (bước 2), u32 LE, đứng trước danh sách slot.
- Client-side cap check: `player[0x12f8]` = tiền hiện có; hằng cap `9999999` xác nhận ở
  `0072b084…c:41,45` (`&DAT_0098967f - *(param_1 + 0x12f8)`, `SBORROW4(9999999, …)`).
- **Xác nhận tổng vàng cuối cùng KHÔNG qua op 0x19.** Frame xác nhận là money-sync:
  `F4440A001A04<gold:u32LE>00000000` (MainOp `0x1A` sub `0x04`), khớp
  `trade_storage.rs:124-126` `gold_status_frame`. Ngoài ra vật phẩm nhận được đi qua
  `MainOp 0x17` (`1706` + `normal_trade_add_wire`). Conf **A** cho việc 0x19 không mang
  tổng vàng; **C** cho việc 0x17 là frame "nhận vật phẩm sau trade" (đường dẫn chỉ thấy ở Rust).

Vậy chuỗi hoàn tất phía client: `19 02 <offer>` → `19 03 <items>` → `19 03 <commit>` →
`19 02 <status 4>` + `1A04 <gold mới>` + `1706 <item mới>`.

---

### 3.8 ★ `17 3B` — toast "Hủy bỏ giao dịch" (server chủ động báo hủy)

> Bổ sung từ `03_trade_and_stall.md` §B.5 (đã hợp nhất 2026-10-02, đã verify lại).

`client_pseudo_c/case_020_007902FB_FUN_007902fb.c:254-256`
```c
case 0x3b:
  (**(code **)(**(int **)gvar_007DA084 + 0x90))(*(int **)gvar_007DA084,&UNK_00797f1c,2000,0,0);
```
Constant `0x00797F1C` nằm trong `client_pseudo_c/lit_797ee8_798440.hex`, offset `0x34`, len 16.
Decode lại bằng bảng `viscii_to_unicode` (`src/encoding.rs:128-236`):
```
48 fc 79 20 62 f6 20 67 69 61 6f 20 64 b8 63 68
 H  ủ  y     b  ỏ     g  i  a  o     d  ị  c  h
```
⇒ **"Hủy bỏ giao dịch"** (VISCII), toast **2000 ms**. **Mức A**

> ⚠ **Lưu ý về byte literal.** Bản gốc `03_trade_and_stall.md` §B.5 ghi chuỗi byte là
> `… 64 62 86 38` — **sai**. Byte thật tại `0x00797F1C` là `… 64 b8 63 68`
> (`0xB8 => '\u{1ECB}'` = `ị`, xem `src/encoding.rs:192`). Chuỗi decode `"Hủy bỏ giao dịch"`
> vẫn **đúng**, chỉ là 3 byte cuối trong bản gốc bị chép sai. Đã sửa ở đây.

Xác nhận ngữ nghĩa: đây là thông báo **server chủ động báo cho client rằng giao dịch bị hủy**
(thường do server timeout hoặc đối phương biến mất), **khác** với `19 02 [03]`
(toast 3000 ms `"Đối phương hủy bỏ giao dịch"` — có chủ ngữ "Đối phương").
**Rust hiện KHÔNG gửi `17 3B`.** **Mức A**

### 3.9 ★ Kênh báo lỗi CHI TIẾT `1B 01` / `1B 02` (main `0x1B`, case 24)

> Bổ sung từ `03_trade_and_stall.md` §B.7 (đã hợp nhất 2026-10-02).

`client_pseudo_c/case_024_00791D80_FUN_00791d80.c`:

* `SubOp == 1` (`:32-67`): `code = RP[1]`; toast **2000 ms**; nến `code != 0` mới hiện.
  Ngữ nghĩa: **lỗi phía người mua** ("vàng của bạn không đủ", "bảng vật phẩm của bạn đã đầy").
* `SubOp == 2` (`:69-104`): cùng cấu trúc, ngữ nghĩa: **lỗi phía người bán**
  ("vàng của bạn đã đầy", "không có vật phẩm này", "không có nhiều như thế này"). **Mức B**
* `SubOp == 3` (`:106-108`): `(*gvar_007DA720)[VMT+0x20]()` — đóng form nào đó. **Mức A**
* **(không có `case 5`)** ⇒ `code 5` ⇒ biến chuỗi **chưa khởi tạo** → toast rác. **Mức A**

> 📌 **Bảng decode đầy đủ 12 thông báo `1B 01`/`1B 02` (kèm địa chỉ hằng + `file:line`)
> nằm ở [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) §7** — bản đó đã verify
> lại toàn bộ và **đính chính 1 hàng sai** của tài liệu cũ. Không nhân bản ở đây.

⇒ `1B 01 <code>` / `1B 02 <code>` là kênh báo lỗi **chi tiết (2000 ms)**, tách riêng khỏi
`19 02` (3000 ms). **Mức B** (suy từ nội dung chuỗi)
**Rust hiện KHÔNG gửi `1B xx` → người chơi không thấy thông báo lỗi chi tiết nào.** **Mức A**

---

## 4. Pet trade

**Không có frame pet-in-trade riêng trong op 0x19.** Bằng chứng:

1. Sub-op `0x19` S→C = `FUN_0074df70` (nonce ACK, `0074df70…c:70-79`) — không liên quan pet.
2. Các sub-op `0x14/0x16/0x17/0x18` (S→C) dispatch vào `gvar_007DA32C` — đó là
   **inventory manager**, không phải pet box. `func_0x0062e41c` đọc `FUN_0071d1bc(player, id)`
   rồi chỉnh `player[0x1344]` và `inventory[idx][0x35C]` (`0062e41c…c:43-66`).
3. Frame cancel `19 01` mang `TTradeManage[0x04]` — đây là **u32 id vật phẩm/thú**, và
   `FUN_004c9bf0` phân loại nó thành nhóm slot-type (2,3,4,10) → tức là pet đi qua
   **cùng đường item** như một "companion item". Xem `0075ad70…c` nhánh
   `FUN_004c9bf0(*(int*)(param_1+4))`.
4. `trade_system.rs::pet_trade_settle` (`trade_system.rs:102-152`) vận hành trên
   `Session.pets` với `stt` — nhưng **không có frame nào ở 0x19 mang `stt`** (xem bảng §2.1:
   `19 02` mang u32 + slot bytes, không có trường pet).

Kết luận: **Rust `pet_trade_settle` là suy diễn Bear-dialect, chưa có đối chứng ở aLogin.**
Hướng client-side có khả năng đúng: pet là một `TradeImage` trong `19 03` với `itemId` thuộc
nhóm type mà `FUN_004c9bf0` trả về (pet/cây cảnh thường là item type trong `Item.dat`).
Cần đọc `FUN_004c9bf0` + bảng `TTradeManage` (3000 row × `0x2B`) để xác định type →
pet. Conf **C**.

Rust `pet_offer_frame` (`trade_storage.rs:82-116`) phát `MainOp 19 0C` — nhưng `0x0C` S→C
ở aLogin là **NPC form chi tiết**, không phải pet. Gap nghiêm trọng — xem §6.6.

---

## 5. Trade với NPC (`Form_TradeFNpc` / `btn_TradeNpc`)

Khác hẳn P2P — dùng **cùng MainOp 0x19** nhưng form và sub-op riêng.

| Bước | Frame | Handler | Evidence |
|---|---|---|---|
| mở form NPC | `S→C 19 0A <u32LE npcId>` | `func_0x0075bf7c` → `FUN_0060b188(Form_TradeFNpc, id)` | `0075bf7c…c:37-39` |
| báo lỗi/trạng thái | `S→C 19 0B <status u8>` | `func_0x0075b8e0` — 15 arm 0..0x0E | `0075b8e0…c:46-110` |
| form chi tiết vật phẩm NPC | `S→C 19 0C <blob>` | `func_0x0060bbe0` — ghi vào nhóm `TCY_ItemImage` thứ hai, index bound 0..6, rồi `FUN_00612174` vẽ lại | `case_022…c:43-45`; `0060bbe0…asm.txt` |
| client offer NPC | `C→S 19 0B <u32LE> + <1B tab>` (7 byte) | `FUN_0060b230` (btn_BothOk NPC) | `0060b230…c:55-161`; builder `redump/…:202-281` |
| client hủy / đóng | `C→S 19 0C <1B>` (3 byte, `self[0x205]=2`) | `FUN_0060b164` (btn_Cancel **và** btn_close_s) | `0060b164…c:22-23` |
| client hủy (từ manager) | `C→S 19 0A <u32LE>` | `FUN_0075b66c` (TTradeFNpcManage) | `0075b66c…asm.txt:104-106` |

Form NPC = `TCY_TradeFNpcMenu` (VMT `0x6080F8`), ctor `FUN_0060a7a8`, form name
`"Form_TradeFNpc"` (`0060a7a8…c:77`). Cửa sổ có `btn_BothOk` (→`FUN_0060b230`),
`btn_Trade` (→`LAB_0060b730`), `btn_Cancel` (→`FUN_0060b164`) tại
`0060a7a8…c:265-285` (btn tại :265/:273/:282, gán handler tại :261/:268/:276/:285); `btn_close_s` cũng → `FUN_0060b164` (`:99`).

NPC window có **6 slot mỗi bên** + 3 slot "detail" (`0060a7a8…c:118-205`):
`self[0x184+i*4]` và `self[0x1A0+i*4]` cho 2 bên; `self[0x1BC..0x1D4]` detail rows.
⇒ **Số slot khác P2P (25 vs 6)** — đây là dấu hiệu chẩn đoán rõ ràng để phân biệt hai họ frame
khi server nhận payload.

`FUN_0060b230` cũng validate NPC: `self[0x204]` (loại NPC) phải khớp `player[0x151C]`
(`0060b230…c:76-77`), và NPC phải bán được (`inventory[0x55F]`) / có `0x56C == 4` / giá `!= 0xFFFF`
(`0060b230…c:87-125`). Vàng NPC vẫn đi qua editor `self[0x1DC]` + cap `player[0x12f8]`
(`0060b230…c:149-153`).

---

## 6. Gap so với Rust server

### 6.0 ★ ĐÍNH CHÍNH: bảng builder C→S main `0x17` — `0x1E/0x1F/0x20` **CÓ** builder

> Nguồn: `03_trade_and_stall.md` §A.4 (hợp nhất 2026-10-02, **đã verify lại từ hex thô**).

Bảng builder C→S của main `0x17` (`client_pseudo_c/cs_op17_targets_0x783405.hex`, 61 dword,
`sub 0x00..0x3C`) **đã giải mã lại byte-exact**. Kết quả **mâu thuẫn** với một tài liệu nguồn.

**Cấu trúc dispatch** — `client_pseudo_c/cs_op17_dispatch.asm.txt:4-8`:
```
007833f0: 33C0        XOR EAX,EAX
007833f2: 8A45FA      MOV AL,byte ptr [EBP + -0x6]   ; ← SubOp
007833f5: 83F83C      CMP EAX,0x3c
007833f8: 0F87F470..  JA  0x0078a4f2                 ; sub ≥ 0x3D → default
007833fe: FF248505..  JMP dword ptr [EAX*0x4 + 0x783405]
```
`0x0078A4F2` là **default của `FUN_0077f414`** (rơi vào `XOR EAX,EAX` rồi `RET`) ⇒
default = **nop, không phát frame**.

**Bảng đã giải mã (Mức A — đọc thẳng 244 byte hex):**

| Nhóm sub | Builder | Ghi chú |
|---|---|---|
| `0x02,0x03,0x04` | `0x007834F9 / 0x0078357F / 0x00783635` | |
| `0x0A..0x14` | `0x0078371E … 0x00783F8C` | liên tục 11 sub |
| `0x1E..0x3C` | `0x007840A7 … 0x007851D1` | liên tục 31 sub |
| **`0x00,0x01,0x05..0x09,0x15..0x1D`** | **`0x0078A4F2` (default/nop)** | **16 sub** |

⇒ **Sub-op C→S khả dụng của `0x17` = `0x02..0x04`, `0x0A..0x14`, `0x1E..0x3C`** (45 sub).
**Mức A**

> 🔴 **MÂU THUẪN GHI RÕ — `03_trade_and_stall.md` §A.4.1/§A.4.2 (bản gốc) SAI.**
> Bản gốc kết luận *"client KHÔNG BAO GIỜ gửi `17 1E`, `17 1F`, `17 20`"* vì cho rằng
> 3 sub đó trỏ `0x0078A4F2`. **Giải mã lại 244 byte hex cho thấy ngược lại:**
> `0x1E → 0x007840A7`, `0x1F → 0x0078418C`, `0x20 → 0x007841E1` — **cả 3 ĐỀU CÓ builder thật**.
> Các sub thực sự là default là **`0x15..0x1D`** (9 sub) cộng `0x00,0x01,0x05..0x09` (7 sub).
> ⇒ **Kết luận "client không gửi `17 1E/1F/20`" BỊ BÁC BỎ.** Giữ bản đã verify ở trên.
>
> Lưu ý thêm: bản gốc cũng ghi sai `0x21 → 0x007840A7` và `0x3A/0x3B/0x3C` lệch — toàn bộ
> nhóm `0x1E..0x3C` trong bản gốc **lệch 3 sub** so với hex thật.

⇒ `opcode_17.md` §5 (kết luận "client không bao giờ chủ động gửi OP 0x17", dựa trên
`0077f414…c:973-974` `case 0x17: break;`) **SAI** — đó là artefact Ghidra không bóc được
jumptable gián tiếp. **Mức A**

**Về thân builder:** dải `0x00783403 … 0x00785225` **vắng mặt** trong mọi file
`.asm.txt` của `client_pseudo_c/` (đã quét toàn bộ; `0077f414_FUN_0077f414.asm.txt` nhảy thẳng
từ `0x007833FE` sang `0x00785226`) ⇒ **không đọc được payload của bất kỳ builder `0x17` sub
`0x02..0x3C` nào** từ dump hiện có. **Mức A (về sự vắng mặt)**

### 6.1 Comment dispatcher sai — `src/server/dispatcher.rs:299-305`

Comment hiện tại khẳng định:
> "aLogin never initiates trade on 0x19 (its only C→S 0x19 is the ACK after S→C 0x19/0x29)"
> "S→C trade frames are Bear-dialect (aLogin would read them as its 0x19 toast/gift bus)"

**Cả hai vế đều sai, đã refute bằng ASM sweep:**

| Vế claim | Thực tế | Evidence |
|---|---|---|
| "only 1 C→S 0x19" | **6 sub-op**: `0x01,0x02,0x0A,0x0B,0x0C,0x29` | grep `MOV DL,0x19` trong `functions/*.asm.txt` → 7 file, 6 CALL `0x0077f414` |
| "0x19 = toast/gift bus" | `0x19/1` mở Form_Trade, `0x19/3` đổ item grid, `0x19/2` enum trạng thái 0-8 | `0075af6c…c:42-44`, `0075b3ec…c:65-176`, `0075aff4…c:50-89` |
| "only ACK after 0x19/0x29" | ACK chỉ là `0x29`; 5 sub-op còn lại là trade thật | `0074df70…c:70-79` vs 6 dòng asm ở bảng §2.1 |

Vế "ACK" đúng **cho sub `0x29`** — đó là nonce `(player[0x440]&0xFF)^0x7B`. Nhưng suy rộng
sang toàn bộ `0x19` là sai. `0x19` **là** opcode giao dịch P2P của aLogin.

### 6.2 Sub-op `0x01` — Rust đọc sai semantics (cancel ≠ open)

Rust `trade_storage.rs:206-234`: sub `1 | 10` đọc `u32LE` đầu làm `partner_id` và **mở trade**.
Nhưng aLogin gửi `19 01` khi **đóng/hủy**, mang `TTradeManage[0x04]` (id vật phẩm/thú), không
phải partner id (`0x0078543B` + `FUN_0075ad70`).

⇒ Server hiện coi "hủy giao dịch" của client là "mở giao dịch mới". Sai hoàn toàn.
Cần tách: `0x01`/`0x0A` = cancel (dọn state, gửi `19 02 <status 3>` cho cả hai).
Rust cũng **thiếu** frame ACK nonce `19 29 <(code&0xFF)^0x7B>` — không có đường nào trong
`handle_trade` sinh sub `0x29`.

### 6.3 Offered-item record 11B — lệch 1 byte, sai field u32

Rust `item_wire` (`trade_storage.rs:15-25`):
```rust
format!("{}{:02X}00{:02X}{:02X}{:02X}{}", le16(id), count, long_val, giatri+100, khang, le32(texp))
```
Rust đặt: `rec[2]=cnt`, `rec[3]=00`, `rec[4]=long_val`, `rec[5]=giatri+100`, `rec[6]=khang`, `rec[7..10]=texp`.

Client đọc (`0075b3ec…c:93-176` + `00609910…c:94-99`, đối chiếu `0060936c`):
`rec[2]=cnt` ✓ · `rec[3..6]` = 4 flag byte của entry `+0x08..0x0B` ✗ · `rec[7..10]` = **u32 LE `longVal`**
(entry `+0x10`) ✗.

Hai lỗi:
1. **Lệch 1 byte**: byte `0x00` pad ở `rec[3]` đẩy `long_val`/`giatri`/`khang` lệch so với
   `entry+0x08..0x0B`. Phải là `rec[3]=entry+0x08, rec[4]=entry+0x09, rec[5]=entry+0x0A, rec[6]=entry+0x0B`.
2. **Sai field ở đuôi u32**: client coi `rec[7..10]` là `longVal` và dùng nó để **tính lại `doben`**
   qua `FUN_0065b580(itemId, 0, longVal)` (`0075b3ec…c:158`). Rust gửi `texp` ở đó ⇒ với vật phẩm có
   `long_val` (đá phúc, vật phẩm tăng kinh nghiệm thú…) client hiển thị **sai độ bền**.

`doben` không cần gửi (client tự tính) — Rust hiện đã đúng ở chỗ này, nhưng đang gửi thừa `texp`
vào chỗ mà client đọc là `longVal`.

### 6.4 Slot index ≠ bag slot — cần xác minh

Client coi byte slot trong `19 02` là **index 1..25 vào mảng `player[0x9E0 + i*4]`**
(`0060936c…c`, cùng index với loop `i = 1..0x19`), **không** phải byte slot thô của túi.
Rust (`trade_storage.rs:243-259`) tra `session.homdo.iter().find(|i| i.slot == *slot)` — tức coi
byte đó là `InventoryItem.slot` thô. Hai hệ index này **có thể trùng** (nếu server đánh số túi
liên tục từ 1) nhưng chưa có bằng chứng. Nếu túi có ô trống xen kẽ thì server sẽ nhận nhầm
vật phẩm. Conf **C** cho rủi ro; cần đối chiếu bảng inventory 12-byte của `opcode_17.md`.

### 6.5 Thiếu frame commit `19 03 <state>`

Rust `handle_trade` sub `3` (`trade_storage.rs:272-334`) coi `19 03` là action byte và
**không có bước commit hai phía rõ ràng**: nó settle ngay khi `action == 1 && partner.accepted`.
Nhưng client chỉ có **một** nút commit (`btn_Trade` sau khi `btn_BothOk` ẩn đi,
`006090a8…c:73-75`), và byte đó là `Form_Trade[0x228]`, không phải action 1/2.
Cần xác minh state byte có 2 phía hay không (chưa có caller C→S CL=0x03 trong export).

### 6.6 Sub `0x0A`/`0x0B`/`0x0C` của Rust = NPC trade, không phải P2P

Rust dùng `11 | 12` cho pet/NPC (`trade_storage.rs:334-…`) và `pet_offer_frame` phát `190C`.
Nhưng `19 0C` S→C ở aLogin = **NPC item detail form** (`func_0x0060bbe0`), `19 0B` = NPC
status enum 15 arm. Nếu Rust gửi `190C` blob pet, client sẽ render nhầm vào form NPC.
Cần tách hẳn 2 họ frame (P2P: `0x01/0x02/0x03`; NPC: `0x0A/0x0B/0x0C`) — số slot 25 vs 6
là chỉ báo phân biệt.

### 6.7 `pet_trade_settle` không có đường wire

`trade_system.rs:102-152` + `handlers/pet_actions.rs` (op `0x0F`/`0x2C`) không có frame nào
ở aLogin mang trường pet `stt` trong giao dịch. Chi tiết phân tích ở **§4**.

### 6.8 Thiếu `19 29` nonce ACK

`FUN_0074df70` (`0074df70…c:70-79`) bắt buộc client reply `19 29 <1B>`. Rust không có nhánh nào
nhận/gửi sub `0x29` ⇒ nếu server thật sự gửi `0x29`, client sẽ im. Nếu server không gửi,
`player[0x1510]` không bao giờ được set.

> ⚠ **Đính chính 1 claim từ tài liệu nguồn.** `03_trade_and_stall.md` §B.3.4/§B.6 kết luận
> **`19 29` là code chết** ("byte map không có entry cho `0x29` ⇒ rơi về default"). **SAI.**
> Giải mã lại `client_pseudo_c/cs_op19_byte_map_0x7853e5.hex` (42 byte) cho thấy
> `byte_map[0x29] = 0x0A` ⇒ **sub `0x29` CÓ builder thật** tại `0x00785AC8`, khớp đúng với
> bảng §2.1 dòng `0x29`. Kết luận này **đã bị bác bỏ**; giữ claim của §2.1/§6.8 (mức A).

### 6.10 ★ MÂU THUẪN CHƯA GIẢI QUYẾT: có "giao dịch thú cưng" ở `0x19` hay không?

> Ghi nhận mâu thuẫn, **không** xóa im lặng (theo yêu cầu hợp nhất 2026-10-02).

Hai tài liệu đọc **cùng một builder** `0x007855ED` (sub `0x0A`) nhưng kết luận ngược nhau:

| Quan điểm | Claim | Bằng chứng |
|---|---|---|
| **File này (§2.1, §4, §6.6)** | `19 0A` = **NPC trade**, đọc `TTradeFNpcManage[0x04]` (form `TCY_TradeFNpc`, 6 slot) — **không có** frame pet trade riêng | `0075b66c…asm.txt:104-106` (caller); `0060a7a8…c:118-205` (form NPC 6 slot) |
| **`03_trade_and_stall.md` §B.3.2, §B.4 Pha 4** (đã hợp nhất, **chưa bác bỏ**) | `19 0A` = **mời giao dịch THÚ CƯNG**, payload `[pet_id:4 LE]`, mở form `*gvar_007DA438` | Chiều S→C `19 0A` → `func_0x0075bf7c` → `FUN_0060b188(*gvar_007DA438, pet_id)` (`0075bf7c…c:37-39`) |

**Điểm chung:** cả hai đều dựa trên **cùng một builder `0x007855ED`** đọc **1 dword** từ
`(*gvar_007DA77C)[+4]` (C→S) và mở form `*gvar_007DA438` (S→C). Khác biệt chỉ là **đặt tên
dword đó** là `TTradeFNpcManage[0x04]` hay `pet_id`.

**Trạng thái: CHƯA XÁC MINH.** Cần đọc ctor `FUN_0060a7a8` + `FUN_0060b188` để xác định
class thật của `*gvar_007DA438` (form 6 slot hay form pet), và đọc `TTradeFNpcManage`
và `TTradeFNpc` table để xác định ý nghĩa field `+4`.
**Mức B (cho cả hai hướng).** Xem thêm §4 (phân tích pet trade của file này) và §6.6.

Tương tự, `19 0B` C→S: file này §2.1 ghi "NPC shop/stall frame" `[0x204]`, tài liệu nguồn
ghi `[gold:4 LE][pet_stt:1]`. **Cùng dựa trên builder `0x00785671` đọc `(*gvar_007DA438)+0x204`.**
Chưa xác minh tên nghiệp vụ.

> ⚠ **Lưu ý thêm về mã sub.** Tài liệu nguồn §B.3.3 gán builder `0x007857B1` cho sub
> `0x13`. **Giải mã lại `cs_op19_byte_map_0x7853e5.hex` cho thấy `byte_map[0x13] = 0x00`
> ⇒ sub `0x13` là DEFAULT (nop)**; builder `0x007857B1` thực ra là của **sub `0x14`**
> (`byte_map[0x14] = 0x07`), khớp bảng §2.1. Claim "sub `0x13` có builder" **SAI** — đã loại.

### 6.9 Bảng GAP cụ thể (Rust ↔ client) — tổng hợp

> Bổ sung từ `03_trade_and_stall.md` §PHẦN C (đã hợp nhất, đã đối chiếu §2–§6 của file này).

| # | Hạng mục | Rust làm | Client thực sự làm | Mức | Hành động |
|---|---|---|---|---|---|
| **G2** | **`19 10` (sub 10)** | gộp chung với sub 1 → coi là `partner_id`, trả `190A` | **`19 0A` = hủy/mở form NPC**, payload `[id:4 LE]` từ `TTradeFNpcManage[0x04]` | A | Tách `sub 10` ra handler riêng; **bỏ gộp `1 \| 10`** (`trade_storage.rs:206`) |
| **G3** | **Mở form giao dịch (S→C)** | `1901` + `le32(partner_id)` | `19 01` + `[u32 LE]` → mở form | A | **KHỚP — giữ nguyên** |
| **G4** | **Đề nghị item (S→C `19 03`)** | `1903` + `le32(gold)` + `item_wire`(11B) | `19 03` + `[gold:4 LE]` + `N×11B` | A | **KHỚP về layout — giữ** (xem thêm §6.3 về sai field) |
| **G5** | **Xác nhận/hủy (C→S `19 03`)** | `action 1` = ok, `2` = hủy | `19 03 [1 byte]` = `Form_Trade[0x228]` | A | **KHỚP về số byte**; cần xác minh state byte 1 phía hay 2 phía (§6.5) |
| **G6** | **Mã kết quả `19 02`** | `190204` = thành công, `190203` = hủy, `190207` = lỗi túi, `190209` = lỗi chung | `19 02 [4]`="Song phương giao dịch hoàn thành", `[3]`="Đối phương hủy bỏ giao dịch", `[7]`="…trữ tồn quá nhiều…" | A | **KHỚP về số hiệu**; nhưng **`190207` mang nghĩa "túi đầy"**, còn `190205` mới là "quang cảnh bất đồng" |
| **G7** | **Báo hủy từ server** | **KHÔNG gửi** | `17 3B` → toast 2000 ms **"Hủy bỏ giao dịch"** | A | Thêm: gửi `17 3B` khi server hủy/timeout trade (§3.8) |
| **G8** | **Báo lỗi chi tiết** | **KHÔNG gửi `1B xx`** | `1B 01 <code>` / `1B 02 <code>` → 6 message lỗi, toast 2000 ms | A | Thêm `1B 01 01` (thiếu tiền), `1B 01 02` (túi đầy), `1B 02 03` (không đủ hàng) — bảng ở [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) §7 (§3.9) |
| **G9** | **Frame trừ/cộng tiền + item sau chốt** | `1A 04 [A:4][B:4]` + `17 06 <item_wire>` | `1A 04`/`17 06` **chưa xác minh phía client** (`17 06` → `func_0x0076b458` không có body) | C | Cần body `func_0x0076b458` + handler `1A 04` |
| **G10** | **`19 14` (PATO)** | đọc `recipient = le32(payload[4..8])`, tối đa **9** cặp | builder aLogin gửi `19 14` **rỗng**; Bear đọc `read32(data,6)` + **5** cặp ở `data[10..]` | A / C | Sửa `take(9)` → `take(5)`; **CHƯA XÁC MINH** vì 2 bản client lệch nhau |

### 6.11 Rust hiện xử lý frame `0x19` nào (bảng tra nhanh)

> Bổ sung từ `03_trade_and_stall.md` §B.1.

`src/server/handlers/trade_storage.rs` + `src/server/trade_system.rs`:

| Rust | Handler | Payload Rust đọc |
|---|---|---|
| `0x19` sub `1` **hoặc** `10` | `trade_storage.rs:206-234` | `[partner_id:4 LE]` → trả `1901`/`190A` + `le32(partner_id)` cho cả 2 bên |
| `0x19` sub `2` | `:235-271` | `[gold:4 LE]` rồi **mỗi byte là 1 slot** → forward `1903` cho bên kia |
| `0x19` sub `3` | `:272-335` | `[action:1]` (`1`=đồng ý, `2`=hủy) → trả `190204` / `190203` / `190207` / `190209` |
| `0x19` sub `11` | `:336-357` | `[gold:4 LE][stt:1]` → trả `190C` (`pet_offer_frame` / `empty_pet_offer_frame`) |
| `0x19` sub `12` | `:358-453` | `[action:1]` → trả `190B04` / `190B03` / `190B0F` / `190B07` / `190B0A` |
| `0x19` sub `20` | `:459-538` | `[?4][recipient:4 LE]` rồi cặp `(slot:1, qty:1)`, `take(9)` |
| `0x1E` sub `1`,`2` | `:552-593` | kho/túi (không thuộc trade) |
| `0x17` sub `51`,`52` | `:594-652` | lưu lang (không thuộc trade) |

Rust `sub 1 | 10` bị gộp là **nguồn sai lớn nhất** (xem §6.2, §6.6, gap G2).

**Sai lệch nhỏ đã xác nhận (không phải gap lớn):**
* `normal_trade_add_wire()` (`trade_storage.rs:27-37`) khớp record 11 byte của `19 03`
  (2 LE id + count + `00` + long_val + giatri+100 + khang + le32 texp) — **Mức A**.
* `item_wire()` (`:15-25`) = 11 byte; `transferred_item_wire()` (`:39-50`) = **12 byte**
  (thêm `doben`) — chỉ dùng cho PATO; Bear PATO đọc **slot + qty** chứ không đọc item record
  ⇒ **12 byte ở đây là dư**. **Mức A (Rust) / C (contract)**

---

## 7. Undetermined

| # | Câu hỏi | Trạng thái | Cần gì |
|---|---|---|---|
| 1 | Frame **request trade** (client mở cửa sổ với đối phương) — C→S nào? | Chưa tìm thấy. `FUN_00609000` chỉ có 1 caller S→C (`0075af6c`), nghĩa là **server chủ động mở** form; có thể request đi qua op khác (`0x29`? `0x13`? menu chuột phải `FUN_0051189c` không gọi `btn_Trade`) | grep `FUN_0077f414` trong `0051189c*` và `005c0e18*`; đọc `LAB_00609218` (btn_Trade handler của Form_Trade) |
| 2 | `Form_Trade[0x228]` = state byte nào? (commit value) | Chưa có writer trong export | đọc `0060936c`/`00609e28` toàn bộ |
| 3 | 4 byte flag (record `rec[3..6]` = entry `+0x08..0x0B`) là gì? | Vị trí chắc; tên chưa | đọc `FUN_0065b580` + `FUN_007c9b38`, và đối chiếu bảng inventory 12-byte (`opcode_17.md`) |
| 3b | `LAB_00609218` (btn_Trade P2P) gọi CL nào? | Không có body trong bộ export | export thêm `0x609218`; kỳ vọng `CL=0x03` |
| 3c | Byte slot 1..25 có trùng `InventoryItem.slot` không? | Chưa | đối chiếu bảng inventory client |
| 4 | `TTradeManage` là gì? 3000 row × `0x2B`, `FUN_00759b04` lookup | Chưa định danh; nghi là bảng **loại vật phẩm được giao dịch** | đọc `FUN_00759b04` + file `Data\*.dat` tương ứng |
| 5 | Sub `0x0C` (NPC detail blob) layout đầy đủ | Đã có body `0060bbe0` nhưng chưa map từng byte | phân tích `0060bbe0…asm.txt` |
| 6 | Sub `0x14` C→S (u32+u32+10B) ai gọi? | Không caller trong export; nghi là "chuyển vật phẩm" / storage | grep thêm batch export; đọc `0x007857B1` ngữ cảnh |
| 7 | Pet trade wire format | Chưa có bằng chứng (§4) | đọc `FUN_004c9bf0` + `TTradeManage` table |
| 8 | Có phải `19 01` cũng dùng cho "unlock" (mở lại sau khoá) không? | Chưa rõ | trace `Form_Trade[0x229]` readers |
| 9 | Chuỗi `LAB_0075b3a0` (`19 02` code 8) và `DAT_0075b3cc` (default) | `lit_75b1e4_75b3a0.hex` chỉ phủ tới `0x75B3A1` | dump literal `0x75B3A0..0x75B3D0` |
| 10 | Chuỗi `19 0B` code 1–13 (NPC/pet) | `lit_75bbb4.hex` chỉ 64 byte | dump `0x75BBEC … 0x75BF14` |
| 11 | `1B 01` / `1B 02` **code 5** | không có `case 5` trong `case_024_...c` ⇒ biến chuỗi chưa khởi tạo; client sẽ toast rác (hoặc crash) | xác minh thêm; xem [`02_buy_sell_transaction.md`](./02_buy_sell_transaction.md) §7 |
| 12 | Layout `17 06` (frame Rust dùng để báo "nhận được item") | `func_0x0076b458` không có body trong dump | disassemble `0x0076b458` |
| 13 | Layout `1A 04` (frame Rust dùng để báo gold mới) | **client chưa xác minh** | trace `case_023` sub `0x04` |

---

## 8. Phụ lục — bản đồ file đã dùng

| File | Vai trò |
|---|---|
| `client_pseudo_c/case_022_0079157C_FUN_0079157c.c` | S→C 0x19 dispatcher, 16 nhánh |
| `redump/cs_op19_byte_map_0x7853e5.hex` | map CL → idx (S→C) |
| `redump/cs_op19_targets_0x78540f.hex` | idx → code address |
| `redump/cs_op19_dispatch_cases.asm.txt` | ASM 10 builder C→S 0x19 |
| `redump/cs_dispatch_byte200_0x77f474.hex` | map DL(MainOp) → idx |
| `redump/cs_builder_dword62_0x77f53c.hex` | idx → builder address |
| `functions/0077f414_FUN_0077f414.asm.txt` | dispatcher C→S gốc |
| `functions/0075af6c/7aff4/7b3ec/7bf7c/7b8e0_*.c` | 5 handler S→C P2P/NPC |
| `functions/00609000/609910/6090a8/60936c/609e28/608904_*.c` | Form_Trade (TCY_TradeMenu) |
| `functions/0060a7a8/60b164/60b188/60b230/60b9bc/60bbe0_*.c` | Form_TradeFNpc |
| `functions/0075ad70/0075b66c/0074df70_*.c` | cancel + nonce |
| `functions/004c9bf0_*.c` | phân loại item type (pet?) |
| `redump/lit_75b1e4_75b3a0.hex` | 9 chuỗi toast P2P |
| `redump/lit_75bbb4..75bf5c.hex` | chuỗi toast NPC |
| `src/server/dispatcher.rs:299-305` | comment cần sửa |
| `src/server/trade_system.rs` | settle engine (cần audit lại record) |
| `src/server/handlers/trade_storage.rs` | wire encode (sai 1 byte record) |