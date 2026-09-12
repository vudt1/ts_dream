# PHÂN TÍCH — Main OP 0x34 (52) / Case 45 / FUN_00795266 @ 0x00795266

Ngày: 2026-09-12 · Workspace: `/mnt/d/VUDT/GIT_PCC/test` · Feature: `op-code` · Chiều: **Server → Client (S→C) một chiều**
Trạng thái: **Đã xác minh từ mã nguồn sơ cấp** (`ts_decompile/` only).

- Handler `FUN_00795266` chỉ có **một nhánh có tác dụng duy nhất**: `SubOp == 0x01` **và** `*(int*)gvar_007DA51C != 0` → gọi `FUN_00650a24`. Mọi giá trị SubOp khác (kể cả `0x01` khi `gvar_007DA51C` null) là **no-op im lặng** (không có `default`).
- Toàn bộ nghiệp vụ thực nằm ở callee `FUN_00650a24` — **có body đầy đủ trong SSOT**.
- Chiều C→S: **không có** `case 0x34` trong `FUN_0077f414` → client không bao giờ gửi OP này.

---

## 1. Tóm tắt nghiệp vụ

- **Vai trò**: Lệnh "trigger" 1 byte. Server gửi `[34][01]` để yêu cầu client chạy một **thủ tục reset/khởi động lại trạng thái đối tượng quản lý battle/slot** (`gvar_007DA51C`). OP này **không mang dữ liệu nghiệp vụ** nào ngoài byte SubOp; mọi field đều là state cục bộ phía client.
- **Đối tượng đích**: `gvar_007DA51C` là con trỏ toàn cục tới một instance của lớp "quản lý battle/đội hình" (cùng layout với `DAT_0098c63c`): có mảng con trỏ 21 phần tử tại `+0x158` (các "slot/unit"), mảng 21 con trỏ khác tại `+0x1ac`, chỉ số slot đang chọn `+0xe78` (byte, `-1` = không chọn), và các field con `+0x569`, `+0x56a`, `+0x552`, `+0x5a8`, `+0x3ea`, `+0xdb` trên từng slot (xem mục 4, có ghi rõ phần suy luận).
- **Hành vi cốt lõi** (từ `FUN_00650a24`): dọn/refresh 21 sub-object; snapshot thời gian; copy `+0xe5a → +0xe68`; reset byte `+6`; clear cờ per-slot (`+0x569`, `+0x56a`, `+0x552`); đặt `+0xe78 = 0`; rồi rẽ nhánh theo **mode của player** (`*(char*)(*(int*)gvar_007DA7BC + 0x376)`), cuối cùng chạy engine turn `FUN_00658c30`.
- **Phân loại**: phần lõi là **state reset + turn engine**; các lời gọi `FUN_0079b620`/`FUN_0062c6f0`/`FUN_005b0640`/`FUN_0059c0ec` thiên về **UI chỉ báo / bộ chọn hiển thị** (không mang dữ liệu wire).
- Chiều C→S `case 0x34:` **không tồn tại** → OP 0x34 là một chiều S→C.

---

## 2. Entry & cách đọc PacketBuffer (S→C)

### 2.1. Đường tới handler

```
MainOp 0x34 (52) → byte_table[0x78A8EE][0x34] = 0x2D (45)   (hàng 4 của file hex)
                 → dword_table[0x78A9B6][45] @ 0x0078AA6A = 0x00795266
                 → FUN_00795266 (Case 45)
```

- File chính: `ts_decompile/case_functions/functions/case_045_00795266_FUN_00795266.c` (61 dòng; thân hàm dòng 20–58).
- Bản inline trong dispatcher: `ts_decompile/functions/0078a89c_FUN_0078a89c.c:6708-6722` (`case 0x34` của switch ngoài, switch trên **MainOp**). Khớp **1:1** với handler rời: cùng cấu trúc `if (*(local_10 + -4) == 0) → BoundErr(0)`, cùng điều kiện kép `SubOp == '\x01' && gvar_007DA51C != 0`, cùng gọi `FUN_00650a24(*(uint *)gvar_007DA51C)`. Khác duy nhất: tên nhãn `UNK_00795279` / `UNK_007952a6` (địa chỉ tiếp nối) và biến register của bản inline.
- Xác nhận switch ngoài thật sự theo MainOp: `case 0x2d:` tại dòng 6372 chứa đúng 16 nhánh banner của OP 0x2D (`FUN_00794977`), còn `case 0x34:` tại dòng 6708 chứa đúng `FUN_00650a24`. Các `case 0x34:` ở dòng 745/3568/4062/6251 nằm trong **switch lồng bên trong** handler của OP khác, không phải dispatch chính.
- Framing/XOR/pump như `opcode_00_01.md` §2 (XOR tĩnh `0xAD`).

### 2.2. Quy ước ký hiệu

- `P[i]` = byte payload (`P[0]=0x34`), `RP[i]` = byte RestPayload (`RP[i]=P[i+1]`). Delphi `_LStrCopy` 1-based.
- Trong `case_045`, `RP` chính là `local_10` / `unaff_EBP + -0xc`.

### 2.3. Đọc SubOp (dòng 20–29)

```c
iVar2 = *(int *)(unaff_EBP + -0xc);            // RestPayload
iVar1 = 0;
if (*(int *)(iVar2 + -4) == 0) {               // độ dài RestPayload == 0
  iVar1 = _BoundErr(0);                        // L=1 → RangeError
  iVar2 = extraout_EDX;
}
*(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar2 + iVar1);   // SubOp = RP[0]
if ((*(int *)(unaff_EBP + -0x14) == 1) && (*(int *)gvar_007DA51C != 0)) {
  FUN_00650a24(*(uint *)gvar_007DA51C);        // chỉ nhánh có tác dụng
}
// KHÔNG có switch, KHÔNG có default → mọi SubOp khác no-op
```

- Nếu `L == 1` (payload `[34]` không có RestPayload) → `_BoundErr(0)` (RangeError).
- Nếu `L >= 2` và `RP[0] != 1` → không làm gì.
- Nếu `RP[0] == 1` nhưng `gvar_007DA51C == 0` → không làm gì (không có guard tạo object).

### 2.4. Codec & API

| Thành phần | Vai trò ở OP này |
| :-- | :-- |
| `_BoundErr` | Kiểm tra `RP` rỗng → range error (dòng 23) |
| `gvar_007DA51C` | Con trỏ toàn cục tới đối tượng battle/slot-manager; đối số duy nhất cho callee (dòng 27–28) |
| `FUN_00650a24` | Toàn bộ nghiệp vụ (mục 4.2) |
| `_LStrArrayClr` / `_LStrClr` | Chỉ dùng ở **epilogue** (dòng 30–57) để dọn biến chuỗi tạm trên stack của handler; không tác động wire và không phải logic opcode |
| `FUN_0077ef7c` / `FUN_0077eb9c` / `FUN_0077f098` | **Không dùng** — handler không đọc byte dữ liệu nào |

---

## 3. Bảng tổng hợp SubOp

| SubOp | Wire (payload) | Đọc buffer | Core logic |
| :---: | :--- | :--- | :--- |
| `0x01` | `[34][01]` (2B) | `SubOp = RP[0] = 1` | Nếu `*(int*)gvar_007DA51C != 0` → `FUN_00650a24(*(uint*)gvar_007DA51C)`; nếu null → **no-op** |
| khác | `[34][00]`, `[34][02]`…`[34][FF]` | `SubOp = RP[0]` | **No-op** (không có `default`, không có nhánh nào khớp) |

Ghi chú: độ dài RestPayload `>= 1` là bắt buộc; `L = 1` (payload chỉ `[34]`) → `_BoundErr(0)`.

---

## 4. Chi tiết các nhánh (bỏ graphics/sound/animation)

### 4.1. SubOp `0x01` — điều kiện kép

- Điều kiện: `RP[0] == 1` **VÀ** `*(int*)gvar_007DA51C != 0`.
- Tác dụng: gọi `FUN_00650a24(param_1 = *(uint*)gvar_007DA51C)`.
- Nếu `gvar_007DA51C == 0`: im lặng (không tạo object, không log) → mock server phải gửi OP này **sau khi** đối tượng battle đã được cấp phát (thời điểm cấp phát không nằm trong SSOT — xem mục 4.4).

### 4.2. Callee `FUN_00650a24` — logic cốt lõi (có body trong SSOT)

Nguồn: `ts_decompile/functions/00650a24_FUN_00650a24.c` (131 dòng) + `...00650a24_FUN_00650a24.asm.txt` (180 dòng).
Header nguồn (dòng 1–24): Entry `00650a24`, Size 697 byte, Signature `undefined __register FUN_00650a24(uint param_1)`, `Callers: <none>` (metadata này **thiếu** caller thật là `case_045`, vì caller nằm ở thư mục `case_functions/`), `Callees:` liệt kê `@BoundErr`, `FUN_0059c0ec`, `FUN_005b0640`, `FUN_0062c6f0`, `FUN_00651094`, `FUN_00651824`, `FUN_00654898`, `FUN_00658c30`, `FUN_0079b620`, `Now`.

`param_1` là con trỏ tới đối tượng battle/slot-manager (chính là `gvar_007DA51C`). Trình tự:

1. **Dọn 21 sub-object** — `FUN_00651824(param_1)` (dòng 36):
   - `FUN_00651824` (`functions/00651824_FUN_00651824.c`, 39 dòng) loop `local_c = 0..0x14` (điều kiện dừng `!= 0x15`, tức **21 vòng**), bounds-check `0x14`, gọi `FUN_00664ffc(*(int*)(param_1 + 0x1ac + idx*4))`.
   - `FUN_00664ffc` (`functions/00664ffc_FUN_00664ffc.c`, 523 dòng) là hàm "release object theo type id": đọc `*(ushort*)(obj + 6)`, `switch` trên bảng nhảy `DAT_00665027`, `TObject_Free` các field con (`+0x3c`, `+0x40`, `+0x44`, …), gán field = 0, cuối cùng `*(ushort*)(obj + 6) = 0`.
   - Ý nghĩa: **giải phóng/tái tạo 21 phần tử** trong mảng con trỏ tại `param_1 + 0x1ac` (stride 4). Lưu ý mảng này (`+0x1ac`) **khác** mảng được clear cờ ở bước 5 (`+0x158`).
2. **Copy byte**: `*(undefined1*)(param_1 + 0xe68) = *(undefined1*)(param_1 + 0xe5a)` (dòng 37).
3. **Snapshot thời gian #1**: gọi `Now()` (`@0040acb8`, thư viện runtime, **không có body trong SSOT**), lưu kết quả double tại `param_1 + 0xe70` (dòng 38–39; asm dòng 14–17 `CALL 0040acb8` / `FSTP double ptr [EAX + 0xe70]`). Hậu tố `in_ST0`/`in_ST1` là artifact của decompiler khi hàm trả `float10` qua FPU stack.
4. **Reset byte đếm/phase**: `*(undefined1*)(param_1 + 6) = 0` (dòng 40; asm dòng 19).
5. **Clear cờ 21 slot** (dòng 41–59; asm 20–45): loop `local_c = 0` → `while (local_c != 0x15)`:
   - bounds-check `if (0x14 < local_c) _BoundErr(local_c)`;
   - `slot = *(int*)(param_1 + 0x158 + uVar2*4)` (mảng con trỏ 21 phần tử, stride 4);
   - ghi `*(slot + 0x569) = 0`, `*(slot + 0x56a) = 0`, `*(slot + 0x552) = 0`.
   - Đây là **3 cờ trạng thái per-slot** bị xoá về 0.
6. **Chọn slot đầu**: `*(undefined1*)(param_1 + 0xe78) = 0` (dòng 60). Đây là **chỉ số slot đang chọn** (byte có dấu; `-1` = không chọn).
7. **Rẽ nhánh theo mode player** — `if (*(char*)(*(int*)gvar_007DA7BC + 0x376) == '\x04')` (dòng 61):
   - **Nếu mode == 4** (dòng 61–68): chỉ cập nhật một chỉ báo UI qua `FUN_0079b620(*(int*)gvar_007DA0B4, X)`:
     - `if (*(char*)(DAT_0098c640 + 0x25) == 0)` → `X = 6`; ngược lại → `X = 3`.
   - **Nếu mode != 4** (dòng 69–127):
     - `FUN_0062c6f0(*(int*)gvar_007DA32C, 0)` — đặt bộ chọn job/class về 0 (`functions/0062c6f0_FUN_0062c6f0.c`: ghi `+0x29c = 0`, đưa sprite/actor của job 0 vào panel `+0x290`).
     - `FUN_005b0640(*(int*)gvar_007D9E5C, 0)` — tương tự, ghi `+0x38c = 0`, panel `+0x3c0`.
     - `FUN_0059c0ec(*(uint*)gvar_007D9EB8, 0)` — tương tự, ghi `+0x194 = 0`, các hàm phụ `FUN_005a0388`/`FUN_0059d780`/`FUN_00598944`.
       (Ba hàm này là **ba bộ chọn hiển thị song song** cùng được reset về index 0.)
     - Đọc `slot = *(int*)(param_1 + 0x158 + e78*4)` với `e78 = 0`; bounds-check `0x14`.
       - `if (*(short*)(slot + 0x3ea) == 0)` → `*(slot + 0x56a) = 1` và gọi `FUN_00651094(param_1)`.
     - `if (e78 != -1)`:
       - Đọc lại `slot`; `if ((byte)(*(char*)(slot + 0xdb) - 1) < 100)` (tức `slot+0xdb` thuộc `1..100`) → `*(slot + 0x56a) = 1` và `FUN_00651094(param_1)`; **else** → `FUN_00654898(param_1)`.
       - `if (e78 != -1)` (vẫn đúng):
         - `*(undefined1*)(param_1 + 0xe5b) = *(undefined1*)(param_1 + 0xe5a)`;
         - **Snapshot thời gian #2**: `Now()` → lưu double tại `param_1 + 0xe60` (decompiler gán nhầm register `in_ST1` tại dòng 103–104; asm 130–133 xác nhận `CALL 0040acb8` / `FSTP double ptr [EAX + 0xe60]`).
         - `if (*(char*)(DAT_0098c640 + 0x25) == 0 && e78 != -1)`: đọc `sVar1 = *(short*)(slot + 0x5a8)`:
           - `10000` (`0x2710`) → `FUN_0079b620(gvar_007DA0B4, 6)`;
           - `0x3a9a` (`15002`) → `FUN_0079b620(gvar_007DA0B4, 8)`;
           - còn lại → `FUN_0079b620(gvar_007DA0B4, 7)`.
         - `if (*(char*)(DAT_0098c63c + 4) == '\x02' || == '\a')` → `*(undefined1*)(param_1 + 0xe78) = 0xff` (đánh dấu "không chọn slot").
         - `FUN_00658c30(param_1)` — engine xử lý turn/battle (dòng 124).

**Các offset mà `FUN_00650a24` ghi trực tiếp**: `+0xe68` (byte), `+0xe70` (double), `+6` (byte), `+0xe78` (byte, ghi `0` rồi có thể ghi `0xff`), `+0xe5b` (byte), `+0xe60` (double). Ghi **gián tiếp** qua trỏ slot: `slot+0x569`, `slot+0x56a`, `slot+0x552`; và trong nhánh `FUN_00658c30` có thể ghi `slot+0x5a8`.

### 4.3. Phân loại core state vs UI/sound

| Nhóm | Thành phần | Vai trò |
| :-- | :-- | :-- |
| **Core state (bắt buộc để mock đúng)** | `FUN_00651824` + `FUN_00664ffc` (dọn 21 slot `+0x1ac`), clear `+0x569/+0x56a/+0x552`, ghi `+0xe68`, `+0xe70`, `+6`, `+0xe78=0`, `+0xe5b`, `+0xe60` | reset/tái tạo trạng thái battle, mốc thời gian, chọn slot 0 |
| **Core phụ thuộc (engine)** | `FUN_00651094` (đánh giá lại slot, có thể đặt `+0xe78=-1`), `FUN_00654898` (fallback), `FUN_00658c30` (turn engine, điều kiện `player+0x376` và `+0xeb4`, `+0x1306`) | quyết định trạng thái kế tiếp |
| **UI/hiển thị** | `FUN_0062c6f0`, `FUN_005b0640`, `FUN_0059c0ec` (3 bộ chọn job/class), `FUN_0079b620` (chỉ báo combo/banner), các VMT `+0x24` trong `FUN_00651094`/`FUN_00654898` | chỉ ảnh hưởng hiển thị, không đổi dữ liệu wire |
| **Thời gian** | `Now()` @ `0040acb8` | timestamp `+0xe70` / `+0xe60` |

### 4.4. Nhận dạng biến toàn cục (phần suy luận — ghi rõ)

- `gvar_007DA51C` — **suy luận**: con trỏ tới đối tượng **quản lý battle/đội hình 21 slot**. Bằng chứng từ SSOT: nó được dùng xuyên suốt với layout `+0x158` (mảng 21 con trỏ, stride 4), `+0x1ac` (mảng 21 con trỏ), `+0xe78` (chỉ số slot), `+0xe58` (cờ), `+0xeb4` (cờ); hàng trăm hàm trong dải `0x650xxx`–`0x6fxxxx` và `TWindMove.Destroy`, `TPhoenixRabid.Destroy`, `TMagicalShield.Destroy` truy cập cùng layout. Trong SSOT **không có chỗ nào gán `gvar_007DA51C` một giá trị khác 0**; chỉ có `functions/00603f20_FUN_00603f20.c:153-154` giải phóng và đặt `= 0`. Vậy điểm cấp phát (gán non-null) **không có trong SSOT (chưa decompile)**.
- `DAT_0098c63c` — **suy luận**: một con trỏ toàn cục khác tới instance **cùng lớp** battle/slot-manager (có `+0x158`, `+0x1ac`, `+0xe50`, `+0xe78`; so sánh null tại `functions/00644294_FUN_00644294.c:101`). `*(char*)(DAT_0098c63c + 4)` (đọc tại `FUN_00650a24` dòng 121) là một field trạng thái toàn cục; giá trị `2` hoặc `7` buộc bỏ chọn slot. **Ý nghĩa cụ thể của field `+4`: không xác định trong SSOT**.
- `DAT_0098c640` — **suy luận**: vùng dữ liệu toàn cục có cờ 1 byte tại `+0x25`, được `FUN_006538bc:60` set `1`, `FUN_006538bc:118` và `FUN_00654950:146` set `0`, và được kiểm tra `== 0` ở nhiều nơi (`FUN_00650a24` dòng 62/105, `FUN_00650e40`, `FUN_00651094`, `FUN_00654898`). Nhiều khả năng là cờ chế độ "đang bận/khoá" của game, nhưng **tên/mục đích chính xác không có trong SSOT**.
- `gvar_007DA7BC` — **suy luận**: đối tượng player/actor chính (theo các tài liệu khác trong repo). Ở đây dùng `player + 0x376` (byte mode/state; các giá trị đã thấy: `0` tại `FUN_00641fa4:71`, `FUN_006540b0:35`, `FUN_007450f4:248`; `4` tại `FUN_00650a24:61`) và `+0x57c` (mảng 5 con trỏ job/actor, dùng trong `FUN_0062c6f0`/`005b0640`/`0059c0ec`), `+0x151c`, `+0x1305`, `+0x1306`, `+0x644`. **`+0x376 == 4` nhiều khả năng là mode "đang ở trong battle/trận"**, nhưng đây là **suy luận** từ ngữ cảnh (nhánh này bỏ qua việc reset các bộ chọn job và chỉ cập nhật chỉ báo battle).

---

## 5. Chuỗi VISCII → UTF-8

- **Không có chuỗi VISCII nào trên đường đã xác minh.** Handler `FUN_00795266` không tham chiếu literal `.rodata` nào; `FUN_00650a24` cũng vậy (chỉ có số/offset/path code).
- Chuỗi literal duy nhất xuất hiện sâu trong cây gọi là banner UI trong `FUN_006538bc` (`functions/006538bc_FUN_006538bc.c:57` dùng `&DAT_00653e28`, dòng 80 `&DAT_00653e44`, dòng 97 `&DAT_00653e74`), nhưng `FUN_006538bc` chỉ chạm tới qua nhánh fallback `FUN_00654898` và đây là **text banner/UI**, không mang dữ liệu wire.
- `ts_decompile/redump/` **không có** dump cho `00653e28`/`00653e44`/`00653e74` → **chưa decode được** các literal này. Nếu cần, phải redump tới null-terminator rồi map VISCII/cp1258 → UTF-8 NFC như OP 0x02.

---

## 6. Chiều Client → Server

- File `ts_decompile/functions/0077f414_FUN_0077F414.c` (`TFConnect.SendCommand`), `switch(param_2 & 0xff)` bắt đầu tại **dòng 768**, kết thúc tại **dòng 1079** (dấu `}` đóng switch), **không có `default:`** (grep `default` toàn file: 0 kết quả).
- `grep "case 0x34"` trên file: **0 kết quả**. Dải case thực tế (dòng): `0xb`(902), `0xc`(911), …, `0x2c`(982), `0x2d`(984), `0x2e`(986), **`0x32`(988)**, `0x36`(1007), `0x37`(1012), `0x39`(1020), `0x3a`(1022), `0x3b`(1032), … `0x48`(1076), `199`(1078).
- Vắng mặt `0x33`, `0x34`, `0x35`, `0x38`, `0x3e`, `0x44`. Cụ thể `0x34` nằm giữa `0x2e` (dòng 986) và `0x32` (dòng 988) — không có block nào.
- Kết luận: client **không bao giờ gửi OP 0x34**; đây là OP **một chiều S→C**. Không cần định dạng C→S cho mock server.

---

## 7. Ghi chú cho Mock Server

```
[34][01]   2B  → nếu battle-manager (gvar_007DA51C) đã được cấp phát:
                 gọi FUN_00650a24 → reset 21 slot, mốc thời gian, chọn slot 0, chạy turn engine.
                 nếu gvar_007DA51C == null: KHÔNG có tác dụng (no-op).
ĐỪNG GỬI: [34] (L=1 → _BoundErr/RangeError); [34][00] và [34][02..FF] (no-op).
```

- Số nguyên LE, double 8B kiểu `F098` (dù OP 0x34 không truyền số nào — các double là `Now()` phía client).
- `FUN_00650a24` reset cả mảng `+0x1ac` (21 sub-object) và mảng `+0x158` (21 cờ), nên chỉ cần gửi 1 lần `[34][01]` khi muốn "bắt đầu/khởi động lại" trạng thái battle.
- Điều kiện tiên quyết về thứ tự: OP này vô nghĩa nếu battle-manager chưa được cấp phát (điểm cấp phát chưa có trong SSOT) → cần sample live để biết thời điểm server thực sự gửi.

---

## 8. Source trail

| # | Nguồn | Dùng để |
| :-- | :-- | :-- |
| 1 | `case_functions/functions/case_045_00795266_FUN_00795266.c` (61 dòng) | Handler chính (đọc SubOp, điều kiện kép, epilogue) |
| 2 | `functions/0078a89c_FUN_0078a89c.c:6708-6722` | Bản inline dispatcher `case 0x34` (khớp 1:1) |
| 3 | `redump/jumptable_byte200_0x78A8EE.hex` (`[0x34]=0x2D`, hàng 4) + `redump/jumptable_0x78A9B6_case_functions.csv:47` + `case_functions/manifest.csv:47` + `case_functions/jumptable_0x78A9B6_cases.c:8180-8237` | Mapping MainOp→Case→Target |
| 4 | `functions/00650a24_FUN_00650a24.c` (131 dòng) + `.asm.txt` (180 dòng) | Callee cốt lõi (toàn bộ nghiệp vụ) |
| 5 | `functions/00651824_FUN_00651824.c` + `functions/00664ffc_FUN_00664ffc.c` | Dọn 21 sub-object tại `+0x1ac` |
| 6 | `functions/00651094_FUN_00651094.c`, `functions/00654898_FUN_00654898.c`, `functions/00658c30_FUN_00658c30.c`, `functions/00650e40_FUN_00650e40.c` | Engine turn / đánh giá lại slot / fallback |
| 7 | `functions/0062c6f0_FUN_0062c6f0.c`, `functions/005b0640_FUN_005b0640.c`, `functions/0059c0ec_FUN_0059c0ec.c`, `functions/0079b620_FUN_0079b620.c` | Ba bộ chọn job/class + chỉ báo UI |
| 8 | `functions/0077e4cc_FUN_0077e4cc.c`, `functions/00603f20_FUN_00603f20.c:153-154`, `functions/006538bc_FUN_006538bc.c`, `functions/00654950_FUN_00654950.c` | Nhận dạng `gvar_007DA51C`, `DAT_0098c63c`, `DAT_0098c640` |
| 9 | `functions/0077f414_FUN_0077F414.c:768-1079` (grep `case 0x34` = 0 kết quả) | Xác nhận chiều C→S rỗng |
