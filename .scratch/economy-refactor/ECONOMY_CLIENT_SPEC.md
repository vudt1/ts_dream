# ĐẶC TẢ KỸ THUẬT MODULE KINH TẾ CLIENT TS ONLINE (`aLogin.exe`)

## Mục lục

- [6.1 Trang bìa & Metadata](#61-trang-bìa--metadata)
- [6.2 Tóm tắt điều hành (Executive Summary)](#62-tóm-tắt-điều-hành-executive-summary)
- [6.3 Bản đồ module & Bảng ánh xạ Opcode](#63-bản-đồ-module--bảng-ánh-xạ-opcode)
- [6.4 Đặc tả Frame Client → Server (C→S)](#64-đặc-tả-frame-client--server-cs)
- [6.5 Đặc tả Frame Server → Client (S→C)](#65-đặc-tả-frame-server--client-sc)
- [6.6 Luồng nghiệp vụ & Sequence Diagrams](#66-luồng-nghiệp-vụ--sequence-diagrams)
- [6.7 Mô hình tiền tệ & Bộ đếm tài nguyên](#67-mô-hình-tiền-tệ--bộ-đếm-tài-nguyên)
- [6.8 Bảng mã lỗi & Thông báo VISCII 1.1](#68-bảng-mã-lỗi--thông-báo-viscii-11)
- [6.9 Máy trạng thái giao diện (UI State Machine)](#69-máy-trạng-thái-giao-diện-ui-state-machine)
- [6.10 Kiểm tra hợp lệ phía Client (Client-side Validation)](#610-kiểm-tra-hợp-lệ-phía-client-client-side-validation)
- [6.11 Trả lời chi tiết 17 câu hỏi bắt buộc (§5)](#611-trả-lời-chi-tiết-17-câu-hỏi-bắt-buộc-5)
- [6.12 Điều chưa biết & Phương pháp kiểm chứng](#612-điều-chưa-biết--phương-pháp-kiểm-chứng)
- [6.13 Phụ lục (Danh mục file, Glossary, Đính chính tài liệu cũ)](#613-phụ-lục-danh-mục-file-glossary-đính-chính-tài-liệu-cũ)

## 6.1 Trang bìa & Metadata

| Thuộc tính | Chi tiết kỹ thuật |
|:--|:--|
| **Tiêu đề tài liệu** | Đặc tả Kỹ thuật Giao thức & Máy trạng thái Module Kinh tế Client TS Online (`aLogin.exe`) |
| **Ngày phát hành** | 20/10/2026 |
| **Đối tượng phân tích** | Binary thực thi `aLogin.exe` (Borland Delphi Win32 32-bit, có VMT/RTTI, SEH `FS:[0]`) |
| **Phiên bản bộ dump** | **Round-7 Complete** (25/25 handler S→C kinh tế có đủ cặp `.c` + `.asm.txt`; dispatcher C→S `0077f414_FUN_0077f414.asm.txt` 4.909 dòng; bảng nhảy `.hex` & VISCII pool `.hex`) |
| **Phạm vi nghiệp vụ** | Shop NPC (`0x14`/`0x1B`), Tiền tệ & Tài nguyên (`0x1A`/`0x23`), Túi đồ & Sạp/Stall (`0x17`), Giao dịch P2P & Thú/NPC (`0x19`), Tiền trang/Bank (`0x1D`), Kho/Container (`0x1E`), Cờ bạc/Lotto/Mini-game (`0x35`–`0x3E`, `0x18`, `0x2C`, `0x08`, `0x01`, `0x02`, `0x0C`, `0x0F`) |
| **Mã hoá đường truyền** | Frame header `F4 44 <len:u16 LE>` + Payload `P[0..len-1]`, toàn bộ frame XOR từng byte với `0xAD` |
| **Mã hoá chuỗi ký tự** | **VISCII 1.1 (RFC 1456)** single-byte precomposed (Delphi `AnsiString`: `[refcount:4][len:4][bytes...][00]`) |
| **Quy ước Offset** | **Wire offset `P[k]`**: tính từ `payload[0]` (`P[0]` = main opcode, `P[1]` = sub-opcode). **Handler offset `RP[k]`**: tính từ tham số `RP` của handler S→C (`RP[0]` = `P[1]` = sub-opcode, do dispatcher `FUN_0078a8dc` đã cắt `P[0]`). Trong Delphi `_LStrCopy(RP, idx, len)`, `idx` là 1-based nên `idx = k + 1` ứng với `RP[k]` = `P[k+1]`. |
| **Mức tin cậy tổng thể** | **Mức A (98.5%)** — Xác minh trực tiếp tới từng lệnh ASM và từng byte literal; các giả thuyết hành vi phía server gắn nhãn **Mức B/C** rõ ràng. |

## 6.2 Tóm tắt điều hành (Executive Summary)

1. **Danh mục hàng và giá Shop NPC không đi trên mạng (Mức A)**: Server mở form mua (`TInterForm`, `gvar_007DA5A8`) qua frame S→C `14 01..06` với `B2 = 6` và `W3 = shopSurfaceIndex`; client tự đọc danh sách item (stride 5 byte) từ `Data\Eve.emg` (`FUN_006df87c`) và tra giá mua (`+0x58`) / giá bán (`+0x5C`) từ `Data\Item.Dat` (`FUN_0065ab58`, record 370 byte).
2. **Frame `1B 03` là S→C mở form bán, tuyệt đối không phải C→S (Mức A)**: Tại `00791d80_FUN_00791d80.asm.txt:19-41`, khi `RP[0] == 3`, client chỉ gọi `(**(code **)(**(int **)0x007da720 + 0x20))()` để mở `TThingSaleForm`; ở chiều C→S (`0x00785FFD..0x0078600C`), `0x1B` chỉ có 2 nhánh `sub = 1` và `sub = 2`, mọi `sub >= 3` nhảy thẳng về epilogue `0x0078A4F2`.
3. **Khác biệt bất đối xứng giữa Mua (`1B 01`) và Bán (`1B 02`) (Mức A)**: `1B 01` gửi `1B 01 <index:u8> <count:u16 LE>` (5 byte, `index` **0-based** trong danh sách shop `TInterForm+0x152`, `count` tối đa 1.250 tại `0x005D7259`); ngược lại `1B 02` gửi `1B 02 <slot:u8> <qty:u8>` (4 byte, `slot` **1-based `1..25`** lấy từ `TThingSaleForm+0x14C` → `+0x160` tại `0x005DCC66..0x005DCCC4`, `qty` 1 byte tối đa 50).
4. **Đính chính chiều Ngân hàng `1D 01` vs `1D 02` (Mức A)**: Ở chiều **C→S**, `1D 01 <amount:u32 LE>` là **Rút tiền** (`FUN_005D85C4` kiểm tra `bank >= amount` và `bag < 9.999.999`) còn `1D 02 <amount:u32 LE>` là **Nộp tiền** (`FUN_005D87B4` kiểm tra `bag >= amount` và `bank < 9.999.999`); nhưng ở chiều **S→C** (`00791f76.asm.txt`), `1D 01 <Δ:u32>` là **Cộng tiền trang** (`FUN_0072b9b8`) và `1D 02 <Δ:u32>` là **Trừ tiền trang** (`FUN_0072ba0c`).
5. **Đính chính hoàn toàn cấu trúc `17 06` (Nhận vật phẩm) so với tài liệu cũ (Mức A)**: `0076b458_FUN_0076b458.c:72-104` chứng minh `17 06` dài **14 byte** (`17 06 <itemId:u16 LE> <qty:u16 LE> <b1:u8> <b3:u8> <b4:u8> <b5:u8> <exp:u32 LE>`), **không có byte `slot`** như ghi chú cũ suy đoán; client tự phân bổ vào túi qua `FUN_0072b6c8`.
6. **Đính chính hoàn toàn cấu trúc `17 21` (Danh sách hàng trên sạp người khác) (Mức A)**: `00610290_FUN_00610290.c:89-245` chứng minh `17 21` có **18 byte wire header** (`17 21 <vendorUid:u32> <card1:u32> <card2:u32> <cardPrice:u32>`) theo sau bởi `N` record **16 byte** (`[slot:u8][itemId:u16][qty:u8][b4:u8][price:u32][b9:u8][b10:u8][b11:u8][exp:u32]`), bác bỏ hoàn toàn cấu trúc 19-byte header của server C# Bear cũ.
7. **Frame C→S `17 1E` (Mở sạp bày bán) chứa giá bán từng ô và tổng cộng dài 109 byte + tên bảng hiệu (Mức A)**: Tại `0x007840A7..0x00784187`, client đọc trực tiếp 10 ô từ `TCY_VenderMenu+0x140` (mỗi ô 10 byte gồm `[bagSlot:u8][qty:u8][price:u32 LE][vitality:u32 LE]`), nối thêm chuỗi tên bảng hiệu (`edit_Board.Text`), `cardIndex:u8` (`+0x20C`) và 4 byte giá thẻ (`+0x214`).
8. **Ba trường tiền tệ có cơ chế bảo vệ tràn (overflow guard) khác nhau giữa Delta và Absolute (Mức A)**: Kim mang theo (`player+0x12F8`, trần `9.999.999`) và Tiền trang (`player+0x12FC`, trần `9.999.999`) **từ chối nguyên tử (atomic reject)** nếu phép cộng/trừ delta (`1A 01/02`, `1D 01/02`) vượt trần hoặc âm; nhưng các hàm set tuyệt đối (`1A 04` → `FUN_0072bcf8`, `1D 04` → `FUN_0072be10`, `1A 03` → `FUN_0072c070`) **ghi đè trực tiếp không clamp**.
9. **Tiền mềm / Điểm (`player+0x1300`, trần `2.000.000.000`) chỉ có frame cộng delta `1A 05` và set tuyệt đối `1A 03`, không có frame trừ S→C (Mức A)**: `0079175f_FUN_0079175f.asm.txt` chứng minh `0x1A` chỉ có 6 nhánh `sub = 1..6`, trong đó `1A 05` gọi `FUN_0072bf08` (cộng điểm, trần `0x77359400`) nhưng hàm trừ điểm `FUN_0072bf5c` không được gọi từ bất kỳ nhánh nào của `0x1A`.
10. **Giao dịch 2 người chơi (`0x19`) tách biệt rõ kênh Vật phẩm/Tiền (`19 01..04`) và kênh Võ tướng/NPC (`19 0A..0D`, `19 14..15`) (Mức A)**: `19 02` gửi tiền (`u32 LE`) + 5 cặp `(bagSlot:u8, qty:u8)` (16 byte payload); `19 03` S→C trả về tiền đối tác + tối đa 5 record **11 byte** (`[itemId:u16][qty:u8][b3..b6:4B][exp:u32]`, không có `slot`).
11. **Lỗi thiếu nhánh switch trong `0x1B` (`inner = 5` hoặc `>= 7`) gây hiện Banner rỗng (Mức A)**: Tại `00791d80.asm.txt:57-146`, `inner == 0` gán `"Giao dịch thành công"` (`0x798118`) nhưng bị chặn bởi `CMP byte [EAX+1], 0; JZ 0x00791E63` nên không hiện banner; còn `inner = 5` hoặc `>= 7` nhảy thẳng tới lệnh gọi `Banner` với chuỗi `EBP-0x2C` rỗng (`0`).
12. **Ba opcode trong `OPCODE_MAP.csv` (`0x35`, `0x38`, `0x3E`) có `c2s_builder_addr = 0x0078A4F2` là Epilogue, không có frame C→S (Mức A)**: `0x0078A4F2` là đoạn dọn dẹp SEH và `RET` cuối hàm `FUN_0077f414`; byte-map `0x0077F474` ánh xạ cả 3 opcode này về `index = 0` (`0x0078A4F2`).
13. **Đính chính `23 04`, `02 0B` và `TFWheel` (`0x2C`) (Mức A)**: `23 04 <A:u8> <B:u8>` chỉ ghi `player+0x146C = A*100 + B` (hệ số giá dịch vụ nhà trọ/thương nhân) chứ không phải store HP/SP; `02 0B` mở form tạo bang hội (`TFCreateOrganize`) và kiểm tra `player+0x12F8 >= 100.000` kim; còn `0x2C` (`TFWheel`) là vòng quay hiệu ứng kỹ năng trong trận đánh, không phải sòng bạc.

## 6.3 Bản đồ module & Bảng ánh xạ Opcode

### 6.3.1 Kiến trúc luồng dữ liệu tổng thể (ASCII)

```
+---------------------------------------------------------------------------------------------------+
|                                   CLIENT UI & LOCAL DATA LAYER                                    |
|                                                                                                   |
|  [Data\Eve.emg] (Shop Surface 5B/item) --+--> [TInterForm] (0x7DA5A8) -----> Nút Mua (0x005D6ECC) |
|  [Data\Item.Dat] (370B/rec: Giá +58/+5C)-+    [TThingSaleForm] (0x7DA720) -> Nút Bán (0x005DCC5C) |
|  [Data\RG.Dat] (Wager 0x9D1610) ------------> [TFRollEgg] (0x7DA630) ------> Đặt trứng (0x17 33)  |
|  [TMoneyAccountForm] (0x7DA46C) ------------> Nút Rút (0x5D85C4) / Nộp (0x5D87B4) / Đóng (0x5D84CC)|
|  [TThingAccountForm] (0x7DA0A4) ------------> Kéo thả kho/túi (0x1E 01..08)                       |
|  [TTradeForm] (0x7DA3C4) / [TNPCTradeForm] -> Giao dịch P2P (0x609218) / Võ tướng (0x19 01..15)   |
|  [TCY_VenderMenu] (0x7DA4F8) / [TSallForm] -> Mở sạp (0x17 1E) / Mua sạp (0x17 21) / Đóng (17 1F)|
+-------------------------------------------------------+-------------------------------------------+
                                                        |
                                                        v  Gọi FUN_0077f414(ctx, DL=op, CL=sub)
+---------------------------------------------------------------------------------------------------+
|                         C->S DISPATCHER & BUILDER (FUN_0077f414 @ 0x0077F414)                     |
|                                                                                                   |
|  1. Kiểm tra kết nối: *(byte *)(*gvar_007DA3A0) != 0 (nếu == 0 -> nhảy tới Epilogue 0x0078A4F2)   |
|  2. Tra bảng nhảy chính: idx = byte_map_0077F474[DL] (DL <= 0xC7) -> JMP dword_0077F53C[idx * 4]  |
|  3. Tra bảng nhảy phụ (nếu có):                                                                   |
|     - Op 0x17: sub <= 0x3C -> JMP dword_00783405[sub * 4]                                         |
|     - Op 0x19: sub <= 0x29 -> idx = byte_007853E5[sub] -> JMP dword_0078540F[idx * 4]             |
|     - Op 0x1E: sub <= 0x08 -> JMP dword_00786221[sub * 4]                                         |
|  4. Đóng gói số LE qua helper trên TPlayData (gvar_007D9D30):                                     |
|     - FUN_0077eb1c (u16 -> AnsiString 2B LE) / FUN_0077ee84 (u32 -> AnsiString 4B LE)             |
|  5. Gửi xuống Socket: FUN_0051633c(*gvar_007DA664, payload_AnsiString)                            |
|     -> Thêm header F4 44 <len:u16 LE> -> XOR từng byte với 0xAD -> TCP Send                       |
+-------------------------------------------------------+-------------------------------------------+
                                                        |
                                        WIRE (TCP Socket, XOR 0xAD)
                                        F4 44 <len_lo> <len_hi> <P[0]=op> <P[1]=sub> <P[2..]>
                                                        |
                                                        v
+---------------------------------------------------------------------------------------------------+
|                         S->C DISPATCHER & HANDLERS (FUN_0078a8dc @ 0x0078A8DC)                    |
|                                                                                                   |
|  1. Giải mã XOR 0xAD, tách frame theo F4 44 + len:u16 LE.                                         |
|  2. Cắt byte P[0] (main opcode), truyền phần còn lại RP = P[1..len-1] vào Handler:                |
|     idx = byte_map_0078A8EE[op - 1] -> CALL dword_0078A9B6[idx * 4]                               |
|  3. Handler đọc RP[0] (sub-opcode) và giải mã số LE qua FUN_0077eb9c (u16) / FUN_0077ed68 (u32).  |
+-------------------------------------------------------+-------------------------------------------+
                                                        |
                                                        v
+---------------------------------------------------------------------------------------------------+
|                              CLIENT MEMORY STATE & HUD / TOAST LAYER                              |
|                                                                                                   |
|  * Local Player Struct (*gvar_007DA7BC):                                                          |
|    - +0x12F8: Kim mang theo (int32, 0..9.999.999)    <-- FUN_0072b084 (+), FUN_0072b170 (-),      |
|                                                          FUN_0072bcf8 (set)                       |
|    - +0x12FC: Tiền trang / Bank (int32, 0..9.999.999)<-- FUN_0072b9b8 (+), FUN_0072ba0c (-),      |
|                                                          FUN_0072be10 (set)                       |
|    - +0x1300: Điểm / Point (int32, 0..2.000.000.000) <-- FUN_0072bf08 (+), FUN_0072c070 (set)     |
|    - +0x0502 / +0x050E / +0x0513: Thủy binh / Tài bảo / Đạn dược (int32, trần 30.000.000)         |
|    - +0x09E0 + slot*4 (slot 1..25): 25 ô túi đồ chính (TThing object, size 0x38)                  |
|    - +0x0A48 + slot*4 (slot 1..50): 50 ô Tiền trang vật phẩm (Bank Item Container)                |
|  * Container Manager (*gvar_007DA58C):                                                            |
|    - +0x04 + slot*4 (1..25): Kho nhà trọ/thương nhân | +0x6C + slot*4 (1..5): Kho phụ 5 ô         |
|  * Banner / Toast Manager (*gvar_007DA084): VMT +0x90 hiển thị thông báo VISCII trên màn hình     |
+---------------------------------------------------------------------------------------------------+
```

### 6.3.2 Bảng ánh xạ đầy đủ 25 Opcode liên quan đến Kinh tế (khớp `OPCODE_MAP.csv`)

| Opcode | Tên nhóm nghiệp vụ | Chiều | C→S Idx | C→S Builder Addr | S→C Case | S→C Handler Addr | File Handler Chuẩn (Round-7 C & ASM) | Mức |
|:--:|:--|:--:|:--:|:--:|:--:|:--:|:--|:--:|
| `0x01` | Đăng nhập / Khởi tạo session | C↔S | 2 | `0x0077F64A` | 2 | `0x0078B149` | `0078b149_FUN_0078b149.c` / `.asm.txt` | **A** |
| `0x02` | Chat / Tương tác / Mở form Tạo Quân Đoàn (`02 0B`) | C↔S | 3 | `0x0077F86E` | 3 | `0x0078B6FC` | `0078b6fc_FUN_0078b6fc.c` / `.asm.txt` | **A** |
| `0x08` | Cập nhật chỉ số nhân vật & tài nguyên (`08 01`, `08 06`) | C↔S | 8 | `0x0077FF2E` | 9 | `0x0078D125` | `case_009_0078D125_0078D4AE.c` + `0078d125.asm.txt` | **A** |
| `0x0C` | Bản đồ / Dịch chuyển / Trạng thái cảnh | C↔S | 12 | `0x00780B12` | 12 | `0x0078D84D` | `0078d84d_FUN_0078d84d.c` / `.asm.txt` | **A** |
| `0x0F` | Nhiệm vụ / Sự kiện / Hội thoại phụ | C↔S | 15 | `0x00781248` | 15 | `0x0078E367` | `0078e367_FUN_0078e367.c` / `.asm.txt` | **A** |
| `0x14` | Hội thoại NPC (`TInterForm`) & Mở Shop Mua (`B2=6`) | C↔S | 19 | `0x0078301F` | 18 | `0x0078EC3F` | `0078ec3f_FUN_0078ec3f.c` / `.asm.txt` | **A** |
| `0x17` | Túi đồ, Trang bị, Sạp (Stall `1E..22`), Trứng (`33`), Cắt hàng (`3B`) | C↔S | 21 | `0x007833F0` | 20 | `0x007902FB` | `case_020_007902FB_00790ED4.c` + `007902fb.asm.txt` | **A** |
| `0x18` | Ngân hàng Thủy binh / Chiến dịch (`TBankForm`, `gvar_007DA41C`) | C↔S | 22 | `0x00785226` | 21 | `0x00790ED5` | `00790ed5_FUN_00790ed5.c` / `.asm.txt` | **A** |
| `0x19` | Giao dịch 2 người chơi (Item/Kim & Võ tướng) & Thu thập | C↔S | 23 | `0x007853CA` | 22 | `0x0079157C` | `0079157c_FUN_0079157c.c` / `.asm.txt` | **A** |
| `0x1A` | Tiền tệ (Kim `+0x12F8`, Điểm `+0x1300`) & Phí Quân Đoàn/Thủy binh | C↔S | 24 | `0x00785B4C` | 23 | `0x0079175F` | `0079175f_FUN_0079175f.c` / `.asm.txt` | **A** |
| `0x1B` | Giao dịch Shop NPC: Mua (`1B 01`), Bán (`1B 02`), Mở form bán (`1B 03`) | C↔S | 26 | `0x00785FFD` | 24 | `0x00791D80` | `00791d80_FUN_00791d80.c` / `.asm.txt` | **A** |
| `0x1D` | Tiền trang (Bank `+0x12FC`): Rút (`01`), Nộp (`02`), Đồng bộ & Mở form | C↔S | 27 | `0x00786185` | 25 | `0x00791F76` | `00791f76_FUN_00791f76.c` / `.asm.txt` | **A** |
| `0x1E` | Kho / Container (`TThingAccountForm`): Chuyển ô, Lưu trữ, Rút nhận | C↔S | 28 | `0x0078620C` | 26 | `0x007921A6` | `case_026_007921A6_007922E5.c` + `007921a6.asm.txt` | **A** |
| `0x23` | Quản lý Quân Đoàn, Thuế, Hệ số giá nhà trọ (`23 04` → `+0x146C`) | C↔S | 33 | `0x00786CD6` | 31 | `0x00792BAD` | `00792bad_FUN_00792bad.c` / `.asm.txt` | **A** |
| `0x2C` | Vòng quay kỹ năng chiến đấu (`TFWheel`, **không phải cờ bạc**) | C↔S | 41 | `0x007887C2` | 40 | `0x00794910` | `00794910_FUN_00794910.c` / `.asm.txt` | **A** |
| `0x35` | Mini-game / Bảng thông tin phụ (chỉ nhận S→C) | **Chỉ S→C** | **0** | `0x0078A4F2` *(RET)* | 46 | `0x007952AB` | `007952ab_FUN_007952ab.c` / `.asm.txt` | **A** |
| `0x36` | Khu giải trí / Sòng bạc (`TSportManage`, `gvar_007D9D88`) | C↔S | 45 | `0x007890B8` | 47 | `0x00795494` | `00795494_FUN_00795494.c` / `.asm.txt` | **A** |
| `0x37` | Máy đánh bạc / Quay thưởng (`gvar_007DA3B4` / `gvar_007DA288`) | C↔S | 46 | `0x007890DD` | 48 | `0x007954A5` | `007954a5_FUN_007954a5.c` / `.asm.txt` | **A** |
| `0x38` | Kết quả ván cược / Thông báo sòng bạc (`gap_54f2e1_54f384`) | **Chỉ S→C** | **0** | `0x0078A4F2` *(RET)* | 49 | `0x00795555` | `00795555_FUN_00795555.c` / `.asm.txt` | **A** |
| `0x39` | Máy Slot / Đua ngựa (`TMachineManager`, `gvar_007DA4EC`) | C↔S | 47 | `0x00789151` | 50 | `0x00795579` | `00795579_FUN_00795579.c` / `.asm.txt` | **A** |
| `0x3A` | Tài Xỉu (`TRE_BiDaXiao`, `gvar_007DA42C`) | C↔S | 48 | `0x007891B1` | 51 | `0x007956B9` | `007956b9_FUN_007956b9.c` / `.asm.txt` | **A** |
| `0x3B` | Domino / Bài (`TSBDManager`, `gvar_007DA0F4`) | C↔S | 49 | `0x00789239` | 52 | `0x007956F3` | `007956f3_FUN_007956f3.c` / `.asm.txt` | **A** |
| `0x3C` | Cờ tướng (`TRE_ZMChessMain`, `gvar_007DA778`) | C↔S | 50 | `0x007892B9` | 53 | `0x0079575C` | `0079575c_FUN_0079575c.c` / `.asm.txt` | **A** |
| `0x3D` | Xổ số / Lotto (`TLottoManager`, `gvar_007D9F98`) | C↔S | 51 | `0x007893E6` | 54 | `0x007957DC` | `007957dc_FUN_007957dc.c` / `.asm.txt` | **A** |
| `0x3E` | Kết quả quay số Lotto (`TLottoManager`, chỉ nhận S→C) | **Chỉ S→C** | **0** | `0x0078A4F2` *(RET)* | 55 | `0x00795A72` | `00795a72_FUN_00795a72.c` / `.asm.txt` | **A** |

## 6.4 Đặc tả Frame Client → Server (C→S)

Mọi frame C→S đều được tạo trong hàm `FUN_0077f414` (`0077f414_FUN_0077f414.asm.txt`), sau đó truyền vào `FUN_0051633c(*gvar_007DA664, payload)` để bọc header `F4 44 <len:u16 LE>` và XOR từng byte với `0xAD`.

### 6.4.1 Nhóm Shop NPC (`0x1B`)

#### Frame C→S `1B 01` — Mua vật phẩm từ Shop NPC (`NPCShopBuy`)
- **Địa chỉ Builder**: `0x00785FFD` → nhánh `sub == 1` tại `0x00786011..0x007860C3` (`0077f414_FUN_0077f414.asm.txt:1815-1867`).
- **Điều kiện gửi**:
  1. Bấm nút **Mua (`btn_Buy`)** trên `TInterForm` (`FUN_005D6ECC` tại `asm_hole_5d6a9b_btnbuy.asm.txt:119-160`): kiểm tra `TInterForm+0x156 != 0` (đã chọn dòng hàng), chặn 2 shop đặc biệt `(mapId == 0x326F && shopId == 6)` và `(mapId == 0xD629 && shopId == 3)` (hiện banner `"Vật phẩm này tạm thời không có bán"` tại `0x005D75D0`), gọi `FUN_005D7218` để đọc số lượng (ép trần `<= 1250` và kiểm tra đủ kim `player+0x12F8`), ghi vào `word [TInterForm+0x154]`, nếu `> 0` thì gọi `FUN_0077F414(ctx, 0x1B, 0x01)`.
  2. Click mua nhanh 1 món (`FUN_005D6F7C` tại `asm_hole_5d6a9b_btnbuy.asm.txt:162-177`): gán cứng `word [TInterForm+0x154] = 1` rồi gọi `FUN_0077F414(ctx, 0x1B, 0x01)`.
- **Mức bằng chứng**: **A**.

| Wire Offset | Size | Type | Field | Giá trị / Nguồn dữ liệu trong Client | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | 1 | `u8` | `opcode` | `0x1B` (27) | **A** |
| `P[1]` | 1 | `u8` | `sub` | `0x01` (1 — Mua hàng) | **A** |
| `P[2]` | 1 | `u8` | `index` | `*(u8 *)(*gvar_007DA5A8 + 0x152)` — Chỉ số dòng hàng trong danh sách shop (**0-based**: dòng đầu tiên = `0x00`, tối đa `count-1` của surface trong `Eve.emg`) | **A** |
| `P[3..4]` | 2 | `u16 LE` | `count` | `*(u16 *)(*gvar_007DA5A8 + 0x154)` đóng gói qua `FUN_0077eb1c` — Số lượng mua (`1..1250`). Không có byte padding. | **A** |

- **Ví dụ Hex** (Mua vật phẩm ở dòng đầu tiên `index = 0`, số lượng `count = 10` = `0x000A`):
  - **Chưa XOR (Plain Wire)**: `F4 44 05 00 1B 01 00 0A 00` (`F44405001B01000A00`)
  - **Đã XOR `0xAD` (On-the-wire)**: `59 E9 A8 AD B6 AC AD A7 AD`

#### Frame C→S `1B 02` — Bán vật phẩm từ túi vào Shop NPC (`NPCShopSell`)
- **Địa chỉ Builder**: `0x007860C8..0x00786180` (`0077f414_FUN_0077f414.asm.txt:1869-1916`).
- **Điều kiện gửi**: Bấm nút **Bán (`btn_Sale`)** trên `TThingSaleForm` (`FUN_005DCC5C` tại `asm_hole_5dc98a_btnsale.asm.txt:240-269`):
  - Kiểm tra `*(u8 *)(form + 0x14C) != 0` và `<= 0x19` (`1..25`), kiểm tra ô túi `player->bag[slot]->itemId (+0x04) != 0`.
  - Gọi `FUN_005DCCE0` (`0x005DCCE0..0x005DCD7A`) đọc số lượng từ ô nhập (`form+0x120`, giới hạn 2 chữ số, `0 < qty <= slot_qty`), ghi vào `word [form + 0x162]`.
  - Nếu `qty > 0`, chép `byte [form + 0x14C]` sang `byte [form + 0x160]` rồi gọi `FUN_0077F414(ctx, 0x1B, 0x02)`.
- **Mức bằng chứng**: **A**.

| Wire Offset | Size | Type | Field | Giá trị / Nguồn dữ liệu trong Client | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | 1 | `u8` | `opcode` | `0x1B` (27) | **A** |
| `P[1]` | 1 | `u8` | `sub` | `0x02` (2 — Bán hàng) | **A** |
| `P[2]` | 1 | `u8` | `slot` | `*(u8 *)(*gvar_007DA720 + 0x160)` — Ô túi đồ (**1-based `1..25`** trong mảng `player+0x9E0+slot*4`; lưu ý: nếu tài liệu ngoài quy ước 0-based `0..24` thì trên wire client thực tế gửi giá trị `1..25` vì `0` bị `JZ 0x005DCCC4` chặn!) | **A** |
| `P[3]` | 1 | `u8` | `qty` | Byte thấp `DL` của `*(u16 *)(*gvar_007DA720 + 0x162)` — Số lượng bán (`1..50`, chỉ **1 byte `u8`**) | **A** |

- **Ví dụ Hex** (Bán vật phẩm tại ô túi số `1`, số lượng `5`):
  - **Chưa XOR (Plain Wire)**: `F4 44 04 00 1B 02 01 05` (`F44404001B020105`)
  - **Đã XOR `0xAD` (On-the-wire)**: `59 E9 A9 AD B6 AF AC A8`

### 6.4.2 Nhóm Tiền trang / Ngân hàng (`0x1D`) & Ngân hàng Thủy binh (`0x18`, `0x1A`)

#### Frame C→S `1D 01` — Rút tiền từ Tiền trang về Túi (`BankWithdraw`)
- **Địa chỉ Builder**: `0x00786185..0x00786207` (`0077f414_FUN_0077f414.asm.txt:1917-1953` — dùng chung cho mọi `sub` của `0x1D`).
- **Điều kiện gửi**: Bấm nút **Rút nhận** trên `TMoneyAccountForm` (`FUN_005D85C4` tại `asm_5d8392_5d88dc_bank_buttons.asm.txt:116-195`):
  - Kiểm tra `player+0x12F8 < 9.999.999` (`0x98967F`), đọc số tiền nhập vào `dword [form + 0x110]`, kiểm tra `player+0x12FC >= amount` và `amount > 0`, bật cờ chờ `byte [form + 0x11D] = 1`, gọi `FUN_0077F414(ctx, 0x1D, 0x01)`.
- **Mức bằng chứng**: **A**.

| Wire Offset | Size | Type | Field | Giá trị / Nguồn dữ liệu trong Client | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | 1 | `u8` | `opcode` | `0x1D` (29) | **A** |
| `P[1]` | 1 | `u8` | `sub` | `0x01` (1 — Rút tiền từ Tiền trang về người) | **A** |
| `P[2..5]` | 4 | `u32 LE` | `amount` | `*(u32 *)(*gvar_007DA46C + 0x110)` qua `FUN_0077ee84` — Số kim muốn rút (`1..9.999.999`) | **A** |

- **Ví dụ Hex** (Rút `1.000` kim = `0x000003E8`):
  - **Chưa XOR (Plain Wire)**: `F4 44 06 00 1D 01 E8 03 00 00` (`F44406001D01E8030000`)
  - **Đã XOR `0xAD` (On-the-wire)**: `59 E9 AB AD B0 AC 45 AE AD AD`

#### Frame C→S `1D 02` — Nộp tiền từ Túi vào Tiền trang (`BankDeposit`)
- **Địa chỉ Builder**: `0x00786185..0x00786207` (`0077f414_FUN_0077f414.asm.txt:1917-1953`).
- **Điều kiện gửi**: Bấm nút **Lưu trữ** trên `TMoneyAccountForm` (`FUN_005D87B4` tại `asm_5d8392_5d88dc_bank_buttons.asm.txt:199-277`):
  - Kiểm tra `player+0x12FC < 9.999.999`, đọc số tiền nhập vào `dword [form + 0x110]`, kiểm tra `player+0x12F8 >= amount` và `amount > 0`, bật cờ chờ `byte [form + 0x11C] = 1`, gọi `FUN_0077F414(ctx, 0x1D, 0x02)`.
- **Mức bằng chứng**: **A**.

| Wire Offset | Size | Type | Field | Giá trị / Nguồn dữ liệu trong Client | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | 1 | `u8` | `opcode` | `0x1D` (29) | **A** |
| `P[1]` | 1 | `u8` | `sub` | `0x02` (2 — Nộp tiền từ người vào Tiền trang) | **A** |
| `P[2..5]` | 4 | `u32 LE` | `amount` | `*(u32 *)(*gvar_007DA46C + 0x110)` qua `FUN_0077ee84` — Số kim muốn nộp (`1..9.999.999`) | **A** |

- **Ví dụ Hex** (Nộp `50.000` kim = `0x0000C350`):
  - **Chưa XOR (Plain Wire)**: `F4 44 06 00 1D 02 50 C3 00 00` (`F44406001D0250C30000`)
  - **Đã XOR `0xAD` (On-the-wire)**: `59 E9 AB AD B0 AF FD 6E AD AD`

#### Các Frame C→S `1A 01..04` & `18 01..02` — Ngân hàng Thủy binh & Đóng góp Quân Đoàn
- **Địa chỉ Builder**:
  - `1A 01..04`: `0x00785B4C..0x00785DA1` (`0077f414_FUN_0077f414.asm.txt:1615-1748`).
  - `18 01..02`: `0x00785226..0x007853C5` (`0077f414_FUN_0077f414.asm.txt:1357-1474`).
- **Mức bằng chứng**: **A**.

| Frame | Builder Addr | Size | Cấu trúc Payload (`P[0..]`) | Nguồn dữ liệu & Nghiệp vụ | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--|:--:|
| `1A 01` | `0x00785B70` | 6B | `1A 01 <amount:u32 LE>` | `*(u32 *)(*gvar_007D9C6C + 0x13C)` — Chuyển/nộp tiền từ form `0x7D9C6C` (bấm nút tại `0x0051AA26`) | `F44406001A01E8030000` | **A** |
| `1A 02` | `0x00785BF7` | 6B | `1A 02 <amount:u32 LE>` | `*(u32 *)(*gvar_007D9C6C + 0x13C)` — Rút/nhận tiền từ form `0x7D9C6C` (bấm nút tại `0x0051AB8B`) | `F44406001A02E8030000` | **A** |
| `1A 03` | `0x00785C7E` | 6B | `1A 03 <amount:u32 LE>` | `StrToInt((*gvar_007DA5C4)->edit_168.Text)` — Chuyển tài nguyên/tiền từ form `0x7DA5C4` (`0x00658482`) | `F44406001A0364000000` | **A** |
| `1A 04` | `0x00785D12` | 6B | `1A 04 <amount:u32 LE>` | `StrToInt((*gvar_007DA5C4)->edit_164.Text)` — Chuyển tài nguyên/tiền từ form `0x7DA5C4` (`0x0065836B`) | `F44406001A0464000000` | **A** |
| `18 01` | `0x0078523A` | 5B | `18 01 <w1:u16 LE> <b2:u8>` | `TBankForm` (`*gvar_007DA41C`): `w1 = StrToInt(edit_130)`, `b2 = StrToInt(edit_134)` — Giao dịch Thủy binh 1 | `F444050018010A0001` | **A** |
| `18 02` | `0x00785302` | 5B | `18 02 <w1:u16 LE> <b2:u8>` | `TBankForm` (`*gvar_007DA41C`): `w1 = StrToInt(edit_130)`, `b2 = StrToInt(edit_134)` — Giao dịch Thủy binh 2 | `F444050018020A0001` | **A** |

### 6.4.3 Nhóm Kho / Container (`0x1E`)

Tại `0x0078620C..0x0078621A` (`asm_cs1e_786221_builders.asm.txt`), dispatcher kiểm tra `sub <= 8` và nhảy qua bảng `0x00786221`:
- `sub = 0`: `0x0078A4F2` (Epilogue — không gửi).
- `sub = 1..8`: 8 builder từ `0x00786245` đến `0x0078657E`.

| Frame | Builder Addr | Size | Cấu trúc Payload (`P[0..]`) | Nguồn dữ liệu & Điều kiện gửi | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--|:--:|
| `1E 01` | `0x00786245` | `2+N` | `1E 01` + `(*gvar_007DA0A4 + 0x3F0)` | Gửi từ `TThingAccountForm` (`0x0074896A`, `0x00748BE9`): rút vật phẩm từ Tiền trang đồ (50 ô) về Túi. Chuỗi `+0x3F0` chứa `<slot:u8><qty:u8>` (4B tổng payload). | `F44404001E010101` | **A** |
| `1E 02` | `0x007862B3` | `2+N` | `1E 02` + `(*gvar_007DA0A4 + 0x3F0)` | Gửi từ `TThingAccountForm` (`0x007486B8`, `0x00748D5B`): gửi vật phẩm từ Túi vào Tiền trang đồ. Chuỗi `+0x3F0` chứa `<bagSlot:u8><qty:u8>`. | `F44404001E020105` | **A** |
| `1E 03` | `0x00786321` | 2B | `1E 03` | Không tham số (2 byte) — Yêu cầu làm mới / đóng kho phụ. | `F44402001E03` | **A** |
| `1E 04` | `0x00786376` | `2+N` | `1E 04` + `(*gvar_007DA0A4 + 0x3F0)` | Gửi từ `TThingAccountForm`: chuyển vật phẩm giữa Kho nhà trọ/thương nhân (`gvar_007DA58C+0x04`) và Túi. | `F44404001E040101` | **A** |
| `1E 05` | `0x007863E4` | `2+N` | `1E 05` + `(*gvar_007DA0A4 + 0x3F0)` | Gửi từ `TThingAccountForm`: thao tác kho loại 5 (cùng buffer `+0x3F0`). | `F44404001E050101` | **A** |
| `1E 06` | `0x00786452` | `2+N` | `1E 06` + `(*gvar_007DA63C + 0x1F8)` | Gửi từ form `gvar_007DA63C` (`0x00749FA2`, `0x0074A221`): rút vật phẩm từ Kho phụ 5 ô (`gvar_007DA58C+0x6C`). | `F44404001E060101` | **A** |
| `1E 07` | `0x007864C0` | `2+N` | `1E 07` + `(*gvar_007DA63C + 0x1F8)` | Gửi từ form `gvar_007DA63C` (`0x00749CF0`, `0x0074A393`): gửi vật phẩm vào Kho phụ 5 ô (`gvar_007DA58C+0x6C`). | `F44404001E070101` | **A** |
| `1E 08` | `0x0078652E` | 2B | `1E 08` | Gửi khi bấm nút **Đóng** trên `TMoneyAccountForm` (`0x005D84F1`), `TThingAccountForm` (`0x00749399`), hoặc form Kho phụ (`0x0074A755`) để báo server kết thúc phiên Tiền trang/Kho và bật cờ `gvar_007DA5A0+0xA096 = 1`. | `F44402001E08` | **A** |

### 6.4.4 Nhóm Giao dịch 2 người chơi & Thú/NPC (`0x19`)

Bảng nhảy `0x007853E5` (`cs_op19_dispatch_cases.asm.txt`) chứng minh chính xác **10 sub-opcode** của `0x19` có builder C→S (`01`, `02`, `03`, `0A`, `0B`, `0C`, `14`, `15`, `1F`, `29`); mọi sub khác (bao gồm `19 04`) nhảy về `0x0078A4F2` (không có C→S).

#### 1. Frame C→S `19 01` (`0x0078543B`) — Mời / Chấp nhận giao dịch vật phẩm P2P
- **Kích thước**: 6 byte payload.
- **Bố cục byte**:
  - `P[0] = 0x19`, `P[1] = 0x01`
  - `P[2..5] = targetPlayerId:u32 LE` lấy từ `*(u32 *)(*gvar_007DA700 + 0x04)` qua `FUN_0077ee84`.
- **Ví dụ Hex (Chưa XOR, `targetId = 10001` = `0x00002711`)**: `F4440600190111270000` (XOR `0xAD`: `59E9ABADB4ACBC8AADAD`). **Mức A**.

#### 2. Frame C→S `19 02` (`0x007854BF`) — Gửi danh sách vật phẩm + tiền của mình lên cửa sổ Trade (`TTradeForm`)
- **Điều kiện gửi**: Sau khi chọn đồ/tiền trên `TTradeForm` (`gvar_007DA3C4`) và bấm nút xác nhận (`00608fa0`), client đóng gói 5 ô (`+0x1BC + i*4`) vào chuỗi `TTradeForm+0x224` (10 byte = 5 cặp `<bagSlot:u8><qty:u8>`) rồi gọi `FUN_0077f414(ctx, 0x19, 0x02)`.
- **Kích thước**: **16 byte payload** (`2 + 4 + 10`).
- **Mức bằng chứng**: **A** (`cs_op19_dispatch_cases.asm.txt:85-130` & `00608fa0`).

| Wire Offset | Size | Type | Field | Giá trị / Nguồn dữ liệu trong Client | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | 1 | `u8` | `opcode` | `0x19` (25) | **A** |
| `P[1]` | 1 | `u8` | `sub` | `0x02` (2 — Đặt tiền & danh sách vật phẩm giao dịch) | **A** |
| `P[2..5]` | 4 | `u32 LE` | `gold` | `StrToInt((*gvar_007DA3C4)->edit_20C.Text)` qua `FUN_0077ee84` — Số kim đặt lên bàn giao dịch | **A** |
| `P[6..15]` | 10 | `u8[10]` | `slots_qtys` | Chuỗi `*(AnsiString *)(*gvar_007DA3C4 + 0x224)` dài 10 byte: 5 cặp `[bagSlot_i:u8, qty_i:u8]` (`i = 0..4`; nếu ô trống thì `00 00`) | **A** |

- **Ví dụ Hex** (Đặt `1.000` kim + ô túi `2` số lượng `1`, 4 ô còn lại trống):
  - **Chưa XOR**: `F4 44 10 00 19 02 E8 03 00 00 02 01 00 00 00 00 00 00 00 00` (`F44410001902E803000002010000000000000000`)

#### 3. Frame C→S `19 03` (`0x00785569`) — Huỷ hoặc Chốt giao dịch P2P (`btn_Trade` / Cancel)
- **Điều kiện gửi**:
  - Khi bấm **Giao dịch (`btn_Trade`)** (`00609218_FUN_00609218.c`): nếu `*(u8 *)(*gvar_007D9D88 + 4) == 0` (không đang chơi cờ bạc), gán `*(u8 *)(TTradeForm + 0x228) = 1` rồi gọi `FUN_0077f414(ctx, 0x19, 0x03)`.
  - Khi bấm **Huỷ / Đóng form**: gán `*(u8 *)(TTradeForm + 0x228) = 0` rồi gọi `FUN_0077f414(ctx, 0x19, 0x03)`.
- **Kích thước**: 3 byte payload: `19 03 <action:u8>` (`0x01` = đồng ý giao dịch, `0x00` = huỷ giao dịch).
- **Ví dụ Hex (Đồng ý `action = 1`)**: `F4440300190301` (XOR `0xAD`: `59E9AEADB4AEAC`). **Mức A**.

#### 4. Các Frame C→S `19 0A / 0B / 0C / 14 / 15 / 1F / 29` — Giao dịch Võ tướng (Pet/NPC) & Thu thập

| Frame | Builder Addr | Size | Cấu trúc Payload (`P[0..]`) | Nguồn dữ liệu & Nghiệp vụ | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--|:--:|
| `19 0A` | `0x007855ED` | 6B | `19 0A <targetUid:u32 LE>` | `*(u32 *)(*gvar_007DA77C + 0x04)` — Mời / Chấp nhận giao dịch Võ tướng (Pet Trade) | `F4440600190A11270000` | **A** |
| `19 0B` | `0x00785671` | 7B | `19 0B <gold:u32 LE> <petSlot:u8>` | `TNPCTradeForm` (`*gvar_007DA438`): `gold = StrToInt(edit_1DC.Text)`, `petSlot = *(u8 *)(form + 0x204)` (`1..4`) — Đặt tiền + chọn võ tướng giao dịch | `F4440700190B0000000001` | **A** |
| `19 0C` | `0x0078572D` | 3B | `19 0C <action:u8>` | `*(u8 *)(*gvar_007DA438 + 0x205)` (`1` = chốt, `0` = huỷ) — Xác nhận / Huỷ giao dịch Võ tướng | `F4440300190C01` | **A** |
| `19 14` | `0x007857B1` | 20B | `19 14 <npcId:u32 LE> <gold:u32 LE> <5 × (slot:u8, qty:u8)>` | Form `gvar_007DA32C`: giao dịch đặc biệt gồm 1 võ tướng (`npcId` tra từ bảng `gvar_007DA544` theo index `+0x3BC`), tiền (`+0x1BC`), và 5 ô vật phẩm (`+0x1A4..+0x1B4`, mỗi ô 2 byte `[+0x131, +0x108]`) | `F44414001914010000000000000000000000000000000000` | **A** |
| `19 15` | `0x007859F0` | 2B | `19 15` | Không tham số (2 byte) — Huỷ / Đóng giao dịch `19 14` | `F44402001915` | **A** |
| `19 1F` | `0x00785A45` | 6B | `19 1F <val:u32 LE>` | `*(u32 *)(*gvar_007D9E70 + 0x144)` — Gửi yêu cầu thu thập / trao đổi tài nguyên (`0x7D9E70`) | `F4440600191F01000000` | **A** |
| `19 29` | `0x00785AC8` | 3B | `19 29 <flag:u8>` | `*(u8 *)(*gvar_007DA7BC + 0x1510)` — Gửi cờ trạng thái thu thập tự động của người chơi | `F4440300192901` | **A** |

### 6.4.5 Nhóm Túi đồ, Sạp / Stall (`17 1E..22`), Trứng (`17 33`), Cắt hàng (`17 3B`)

#### 1. Frame C→S `17 1E` (`0x007840A7`) — Mở sạp bày bán (`TCY_VenderMenu`)
- **Địa chỉ Builder**: `0x007840A7..0x00784187` (`asm_cs17_783405_builders.asm.txt:904-959` & `0077f414_FUN_0077f414.c:2185-2258`).
- **Phát hiện quan trọng (Mức A)**: Trái với câu hỏi nghi ngờ "`17 1E` có chứa giá không?", pseudo-C tại `0077f414.c:2195-2255` chứng minh client copy nguyên mảng **100 byte** tại `TCY_VenderMenu + 0x140` (10 ô × 10 byte, trong đó mỗi ô được `FUN_0060dfdc` ghi `[bagSlot:u8][qty:u8][price:u32 LE][vitality:u32 LE]`), nối với tên bảng hiệu `edit_Board.Text`, 1 byte `cardIndex` (`+0x20C`), và 4 byte chuỗi `+0x214`!

| Wire Offset | Size | Type | Field | Giá trị / Nguồn dữ liệu trong `TCY_VenderMenu` (`*gvar_007DA4F8`) | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | 1 | `u8` | `opcode` | `0x17` (23) | **A** |
| `P[1]` | 1 | `u8` | `sub` | `0x1E` (30 — Bắt đầu bày sạp) | **A** |
| `P[2..101]` | 100 | `struct[10]` | `items[0..9]` | 10 record × **10 byte** lấy từ `TCY_VenderMenu + 0x140 + i*10` (`FUN_0060dfdc`):<br>• `+0`: `bagSlot:u8` (`1..25`, hoặc `0` nếu trống)<br>• `+1`: `qty:u8` (`1..50`)<br>• `+2..5`: `price:u32 LE` (giá bán mỗi đơn vị, `1..9.999.999`)<br>• `+6..9`: `vitality:u32 LE` (`exp`/độ bền của item) | **A** |
| `P[102..102+L-1]` | `L` | `VISCII` | `boardName` | `(*gvar_007DA4F8)->edit_1A8->Text` (`+0x1A0`) — Tên bảng hiệu sạp (`0..14` ký tự VISCII) | **A** |
| `P[102+L]` | 1 | `u8` | `cardIdx` | `*(u8 *)(*gvar_007DA4F8 + 0x20C)` — Chỉ số thẻ bày bán (Card index) | **A** |
| `P[103+L..106+L]` | 4 | `bytes[4]` | `cardData` | Chuỗi 4 byte tại `*(AnsiString *)(*gvar_007DA4F8 + 0x214)` | **A** |

- **Ví dụ Hex (Chưa XOR)**: Giả sử chỉ đặt ô đầu (`bagSlot=1, qty=1, price=1000=0x03E8, vit=0`), 9 ô sau = 0, `boardName="A"` (`0x41`), `cardIdx=1`, `cardData=00000000` (tổng `108` byte payload = `0x006C`):
  `F4446C00171E0101E803000000000000` + `(90 byte 00)` + `410100000000`

#### 2. Các Frame C→S `17 1F / 20 / 21 / 22 / 33 / 3B` và Quản lý Túi đồ (`17 02..14`)

| Frame | Builder Addr | Size | Cấu trúc Payload (`P[0..]`) | Nguồn dữ liệu & Nghiệp vụ | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--|:--:|
| `17 02` | `0x007834F9` | 3B | `17 02 <bagSlot:u8>` | Vứt vật phẩm khỏi túi (`*(u8 *)(*gvar_007DA768 + 0x1CC)`) | `F4440300170201` | **A** |
| `17 03` | `0x0078357F` | 4B | `17 03 <bagSlot:u8> <qty:u8>` | Vứt `qty` vật phẩm khỏi ô túi (`+0x1CC`, `+0x1CE`) | `F444040017030101` | **A** |
| `17 04` | `0x00783635` | 5B | `17 04 <srcSlot:u8> <dstSlot:u8> <qty:u8>` | Di chuyển / gộp vật phẩm giữa 2 ô trong túi | `F44405001704010201` | **A** |
| `17 0B` | `0x00783809` | 3B | `17 0B <bagSlot:u8>` | Sử dụng / Trang bị vật phẩm cho nhân vật | `F4440300170B01` | **A** |
| `17 0C` | `0x0078388A` | 3B | `17 0C <equipSlot:u8>` | Tháo trang bị từ người về túi (`1..6`) | `F4440300170C01` | **A** |
| `17 1F` | `0x0078418C` | 2B | `17 1F` | **Thu sạp / Đóng sạp của mình** (bấm nút tại `0x0060DF5A`, không tham số) | `F4440200171F` | **A** |
| `17 20` | `0x007841E1` | 6B | `17 20 <vendorUid:u32 LE>` | **Click xem sạp của người chơi khác**: `*(u32 *)(*gvar_007D9C74 + 0x04)` qua `FUN_0077ee84` | `F4440600172011270000` | **A** |
| `17 21` | `0x00784261` | 8B | `17 21 <vendorUid:u32 LE> <slot:u8> <qty:u8>` | **Mua hàng từ sạp người khác (`TSallForm`, `*gvar_007DA2F8`)**:<br>• `vendorUid`: `*(u32 *)(form + 0x130)`<br>• `slot`: `*(u8 *)(form + 0x164 + selIdx * 0x11)` (`selIdx = *(u8 *)(form + 0x15C) <= 9`)<br>• `qty`: `*(u8 *)(form + 0x134)` | `F44408001721112700000101` | **A** |
| `17 22` | `0x0078434F` | `2+N` | `17 22` + `(*gvar_007DA718 + 0x210)` | Thao tác phụ với chuỗi tại `*gvar_007DA718 + 0x210` | `F44402001722` | **A** |
| `17 33` | `0x00784D6C` | 4B | `17 33 <bagSlot:u8> <eggIdx:u8>` | **Đập trứng (`TFRollEgg`)**: gửi ô vé trong túi + chỉ số trứng chọn | `F444040017330102` | **A** |
| `17 3B` | `0x0078514D` | 3B | `17 3B <slot:u8>` | **Cắt hàng / Chia vật phẩm (`TFCutItem`, `*gvar_007DA13C`)**: gửi `*(u8 *)(form + 0xFE)` | `F4440300173B01` | **A** |

### 6.4.6 Nhóm Cờ bạc / Lotto / Mini-games (`0x36`, `0x37`, `0x39`–`0x3D`)

| Frame | Builder Addr | Size | Cấu trúc Payload (`P[0..]`) | Nguồn dữ liệu & Nghiệp vụ | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--|:--:|
| `36 <sub?>` | `0x007890B8` | **1B** | `36` *(Chỉ 1 byte `P[0]=0x36`, không ghi `sub`!)* | Tại `0x007890BE`: chỉ gọi `_LStrFromChar(0x36)` rồi gửi ngay — Thông báo rời/kết thúc bảng `TSportManage` | `F444010036` | **A** |
| `37 <sub>` | `0x007890DD` | `2+N` | `37 <sub>` + `(*gvar_007DA3B4)->ctrl_138->Text` | Gửi lựa chọn/mức cược từ form `0x7DA3B4` | `F4440300370131` | **A** |
| `39 01` | `0x00789151` | 2B | `39 01` | Bấm quay / bắt đầu trên `TMachineManager` (`sub` khác `1` bị `JNZ 0x0078A4F2` chặn) | `F44402003901` | **A** |
| `3A <side>` | `0x007891B1` | 6B | `3A <side:u8> <wager:u32 LE>` | **Đặt cược Tài Xỉu (`TRE_BiDaXiao`, `*gvar_007DA42C`)**:<br>• `P[1]` ghi đè bởi `*(u8 *)(mgr + 0x04)` (cửa đặt)<br>• `P[2..5]` = `*(u32 *)(mgr + 0x0C)` (số tiền cược) | `F44406003A0164000000` | **A** |
| `3B <sub>` | `0x00789239` | 6B | `3B <sub:u8> <val:u32 LE>` | **Domino (`TSBDManager`, `*gvar_007DA0F4`)**: gửi `sub` + `*(u32 *)(mgr + 0x28)` | `F44406003B0164000000` | **A** |
| `3C 01` | `0x007892D1` | `2+N` | `3C 01` + `(*gvar_007DA778 + 0x34)` | **Cờ tướng (`TRE_ZMChessMain`)**: gửi nước đi (`+0x34`) | `F44404003C010102` | **A** |
| `3C 02` | `0x0078933C` | 2B | `3C 02` | **Cờ tướng**: xin hoà / đầu hàng | `F44402003C02` | **A** |
| `3C 03` | `0x00789391` | 2B | `3C 03` | **Cờ tướng**: rời bàn cờ | `F44402003C03` | **A** |
| `3D <sub>` | `0x007893E6` | 7B | `3D <sub> <n1:u8> <n2:u8> <n3:u8> <n4:u8> <n5:u8>` | **Mua vé số Lotto (`TLottoManager`, `*gvar_007D9F98`)**: gửi 5 số đã chọn tại offset `+0x6A..+0x6E` | `F44407003D010102030405` | **A** |

## 6.5 Đặc tả Frame Server → Client (S→C)

Trong `FUN_0078a8dc` (`0x0078A8DC`), dispatcher đã tách byte `P[0]` (main opcode) và truyền chuỗi `RP = P[1..len-1]` vào từng handler S→C. Do đó trong code handler:
- `RP[0]` (offset `+0` từ con trỏ `RP`) chính là **sub-opcode (`P[1]`)**.
- Khi gọi `_LStrCopy(RP, idx, count)` (1-based của Delphi), `idx` tương ứng với `RP[idx - 1]` = **Wire offset `P[idx]`**.

### 6.5.1 Nhóm Hội thoại NPC & Mở Shop Mua (`0x14`) và Shop Mua/Bán (`0x1B`)

#### 1. Frame S→C `14 01..06` (với `B2 = 6`) — Mở Form Mua hàng Shop NPC (`TInterForm`)
- **Handler nhận**: `FUN_0078ec3f` (`0078ec3f_FUN_0078ec3f.c` / `.asm.txt`, case 18).
- **Loại frame**: Điều khiển mở UI (UI Trigger) + truyền chỉ số bảng hàng cục bộ.
- **Cơ chế hoạt động (Mức A)**:
  - Khi `P[1] = sub` thuộc `1..6` và byte kiểu hội thoại `B2 = 6` (`*(u8 *)(*gvar_007DA5A0 + 0x9D32) = 6`), client lưu chỉ số surface `W3:u16 LE` vào `*(u16 *)(*gvar_007DA5A0 + 0x9D34)`.
  - Hàm render `TInterForm` (`FUN_005d675c`) gọi `FUN_006df87c(*gvar_007DA660, shopSurfaceIndex)` để lấy danh sách item từ bộ nhớ đã nạp sẵn từ file cục bộ **`Data\Eve.emg`** (section Surface, mỗi dòng hàng dài **5 byte**: `[itemId:u16 LE][kind:u8][style:u8][canCut:u8]`), sau đó tra thông tin từng `itemId` trong **`Data\Item.Dat`** (`FUN_0065ab58`, mỗi record 370 byte, lấy giá mua tại offset `+0x58`).

| Wire Offset | Handler `RP` | Size | Type | Field | Ý nghĩa & Side-effect lên State/UI | Mức |
|:--:|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | *(đã cắt)* | 1 | `u8` | `opcode` | `0x14` (20) | **A** |
| `P[1]` | `RP[0]` | 1 | `u8` | `sub` | `0x01..0x06` — Nhánh hội thoại `TInterForm` | **A** |
| `P[2]` | `RP[1]` | 1 | `u8` | `dialogType` | `0x06` — Chế độ mở bảng Mua hàng Shop NPC (`PlayerState+0x9D32 = 6`) | **A** |
| `P[3..4]` | `RP[2..3]` | 2 | `u16 LE` | `surfaceIdx` | Chỉ số bảng hàng trong `Data\Eve.emg` (`PlayerState+0x9D34`), dùng cho `FUN_006df87c` | **A** |
| `P[5..]` | `RP[4..]` | var | `bytes` | `dialogData` | Tham số câu thoại / hình nền NPC đi kèm theo từng `sub` `01..06` | **A** |

#### 2. Frame S→C `1B 03` — Mở Form Bán hàng vào Shop NPC (`TThingSaleForm`)
- **Handler nhận**: `FUN_00791d80` (`00791d80_FUN_00791d80.asm.txt:19-41`, case 24).
- **Loại frame**: Điều khiển mở UI (UI Trigger).
- **Side-effect**: Kiểm tra `RP[0] == 3` (`DEC AL` × 3 → `JZ 0x00791D9A`), đọc `*gvar_007DA720` (`TThingSaleForm`) và gọi virtual method `VMT + 0x20` (`Show`). Form tự quét 25 ô túi đồ tại `player + 0x09E0 + slot*4` (`slot = 1..25`) và tra giá bán cơ sở tại `Item.Dat + 0x5C`.
- **Ví dụ Hex (Chưa XOR)**: `F4 44 02 00 1B 03` (`F44402001B03`; Đã XOR `0xAD`: `59 E9 AF AD B6 AE`). **Mức A**.

| Wire Offset | Handler `RP` | Size | Type | Field | Giá trị / Ghi chú | Mức |
|:--:|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | *(đã cắt)* | 1 | `u8` | `opcode` | `0x1B` (27) | **A** |
| `P[1]` | `RP[0]` | 1 | `u8` | `sub` | `0x03` (3 — Mở `TThingSaleForm`, tổng payload đúng **2 byte**) | **A** |

#### 3. Frame S→C `1B 01 <inner>` & `1B 02 <inner>` — Kết quả Mua / Bán Shop NPC
- **Handler nhận**: `FUN_00791d80` (`00791d80_FUN_00791d80.c` / `.asm.txt:20-165`, case 24).
- **Loại frame**: Thông báo kết quả giao dịch (Result / Toast Notification). **Không tự trừ tiền hay thêm/xoá item** — việc cập nhật tiền và túi đồ do các frame đồng bộ `1A 01/02` và `17 06/08/09` thực hiện độc lập!
- **Hành vi đặc biệt (Mức A)**:
  - Sau khi gán chuỗi thông báo vào `local_2c` (`[EBP-0x2C]`), lệnh tại `0x00791E59`: `CMP byte ptr [EAX+1], 0; JZ 0x00791E63` **bỏ qua lời gọi `Banner`** nếu `inner == 0` (thành công)!
  - Nếu `inner == 5` hoặc `inner >= 7` (không có case trong switch), `local_2c` giữ nguyên giá trị khởi tạo `NULL` (`0`), nhưng vì `inner != 0` nên client vẫn gọi `Banner(*gvar_007DA084, NULL, 2000, 0, 0)` (hiện khung thông báo rỗng 2000ms).

| Wire Offset | Handler `RP` | Size | Type | Field | Giá trị / Ghi chú | Mức |
|:--:|:--:|:--:|:--:|:--|:--|:--:|
| `P[0]` | *(đã cắt)* | 1 | `u8` | `opcode` | `0x1B` (27) | **A** |
| `P[1]` | `RP[0]` | 1 | `u8` | `outer` | `0x01` (Kết quả Mua) hoặc `0x02` (Kết quả Bán) | **A** |
| `P[2]` | `RP[1]` | 1 | `u8` | `inner` | Mã kết quả (`0x00` = Thành công, `0x01..0x04`, `0x06` = Lỗi; xem bảng chi tiết tại §6.8) | **A** |

- **Ví dụ Hex**:
  - Mua thành công (`1B 01 00`): `F44403001B0100` (XOR `0xAD`: `59E9AEADB6ACAD`).
  - Mua thất bại do thiếu vàng (`1B 01 01`): `F44403001B0101` (XOR `0xAD`: `59E9AEADB6ACAC`).

### 6.5.2 Nhóm Tiền tệ & Điểm (`0x1A`) và Tiền trang (`0x1D`)

#### 1. Các Frame S→C `1A 01..06` — Cập nhật Kim mang theo (`+0x12F8`) & Điểm (`+0x1300`)
- **Handler nhận**: `FUN_0079175f` (`0079175f_FUN_0079175f.c` / `.asm.txt`, case 23).
- **Loại frame**: Đồng bộ trạng thái tiền tệ (State Sync) kết hợp thông báo Chat Log / Banner.

| Frame | Size | Cấu trúc Payload (`P[0..]`) | Hàm nội bộ & Tác động lên State / UI | Loại cập nhật | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--|:--|:--:|:--|:--:|
| `1A 01` | 6B | `1A 01 <delta:u32 LE>` + `[opt_flag:u8]` | Gọi `FUN_0072b084(*player, delta)`: nếu `player+0x12F8 + delta <= 9.999.999` thì cộng `delta` vào `+0x12F8`, phát `WA0014.wav`, hiện dòng chữ `"Nhận được <delta> Tiền"` (nếu không có byte thứ 7 `RP[5]`). Nếu vượt `9.999.999`: **từ chối nguyên tử** (trả về `0`), hiện `"Thất bại  đoạt được"` (`0x79805C`). | **Delta (`+Δ`)** | `F44406001A01E8030000` | **A** |
| `1A 02` | 6B | `1A 02 <delta:u32 LE>` | Gọi `FUN_0072b170(*player, delta)`: nếu `player+0x12F8 >= delta` thì trừ `delta` khỏi `+0x12F8`, hiện `"Giảm <delta> kim"`. Nếu `< delta`: **từ chối nguyên tử**, hiện `"Thất bại   giảm thiểu"` (`0x798094`). | **Delta (`-Δ`)** | `F44406001A02E8030000` | **A** |
| `1A 03` | 6B | `1A 03 <point:u32 LE>` | Gọi `FUN_0072c070(*player, point)`: gán trực tiếp `*(u32 *)(*player + 0x1300) = point` (**không clamp**). | **Absolute (`=`)** | `F44406001A0300E1F505` | **A** |
| `1A 04` | 6B | `1A 04 <gold:u32 LE>` | Gọi `FUN_0072bcf8(*player, gold)`: gán trực tiếp `*(u32 *)(*player + 0x12F8) = gold` (**không clamp**). | **Absolute (`=`)** | `F44406001A0440420F00` | **A** |
| `1A 05` | 6B | `1A 05 <deltaPt:u32 LE>` | Gọi `FUN_0072bf08(*player, deltaPt)`: nếu `player+0x1300 + deltaPt <= 2.000.000.000` (`0x77359400`) thì cộng `deltaPt` vào `+0x1300` và hiện `"Nhận được <deltaPt> Điểm"`; nếu vượt trần thì từ chối và hiện `"Thất bại  đoạt được"`. | **Delta (`+Δ`)** | `F44406001A0564000000` | **A** |
| `1A 06` | 2B | `1A 06` | Hiện Banner 2000ms: `"Dung lượng tiền bạc không đủ"` (`0x007980B4`). | **Thông báo lỗi** | `F44402001A06` | **A** |

#### 2. Các Frame S→C `1D 01..09` — Cập nhật Tiền trang (`+0x12FC`) & Điều khiển Form Ngân hàng
- **Handler nhận**: `FUN_00791f76` (`00791f76_FUN_00791f76.c` / `.asm.txt`, case 25).

| Frame | Size | Cấu trúc Payload (`P[0..]`) | Hàm nội bộ & Tác động lên State / UI | Loại cập nhật | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--|:--|:--:|:--|:--:|
| `1D 01` | 6B | `1D 01 <delta:u32 LE>` | Gọi `FUN_0072b9b8(*player, delta)`: nếu `player+0x12FC + delta <= 9.999.999` thì **cộng `delta` vào Tiền trang (`+0x12FC`)**, hiện `"Lưu trữ <delta> kim"`; nếu vượt trần thì từ chối và hiện `"Thất bại lưu trữ"` (`0x7982C8`). | **Delta (`+Δ` Bank)** | `F44406001D01E8030000` | **A** |
| `1D 02` | 6B | `1D 02 <delta:u32 LE>` | Gọi `FUN_0072ba0c(*player, delta)`: nếu `player+0x12FC >= delta` thì **trừ `delta` khỏi Tiền trang (`+0x12FC`)**, hiện `"Rút nhận <delta> kim"`; nếu không đủ thì từ chối và hiện `"Thất bại rút nhận"` (`0x7982F8`). | **Delta (`-Δ` Bank)** | `F44406001D02E8030000` | **A** |
| `1D 03` | 2B | `1D 03` | Hiện Banner 2000ms: `"Dung lượng tiền trang không đủ"` (`0x00798314`). | **Thông báo lỗi** | `F44402001D03` | **A** |
| `1D 04` | 6B | `1D 04 <bankGold:u32 LE>` | Gọi `FUN_0072be10(*player, bankGold)`: gán tuyệt đối `*(u32 *)(*player + 0x12FC) = bankGold` (**không clamp**). | **Absolute (`=`)** | `F44406001D0400000000` | **A** |
| `1D 05` | 2B | `1D 05` | Gọi `(**(code **)(**gvar_007DA46C + 0x20))()` — Mở `TMoneyAccountForm` (Form Tiền trang gửi/rút kim). | **Mở Form UI** | `F44402001D05` | **A** |
| `1D 06` | 2B | `1D 06` | Gọi `(**(code **)(**gvar_007DA0A4 + 0x20))()` — Mở `TThingAccountForm` (Form Tiền trang gửi/rút vật phẩm). | **Mở Form UI** | `F44402001D06` | **A** |
| `1D 07` | 2B | `1D 07` | Xoá cờ khoá nút nộp tiền: `*(u8 *)(*gvar_007DA46C + 0x11C) = 0`. | **Mở khoá UI** | `F44402001D07` | **A** |
| `1D 08` | 2B | `1D 08` | Xoá cờ khoá nút rút tiền: `*(u8 *)(*gvar_007DA46C + 0x11D) = 0`. | **Mở khoá UI** | `F44402001D08` | **A** |
| `1D 09` | 3B | `1D 09 <mode:u8>` | Gọi `FUN_00749c30(*gvar_007DA0A4, RP[1])`: gán `*(u8 *)(TThingAccountForm + 0x418) = mode`. | **State Sync** | `F44403001D0901` | **A** |

### 6.5.3 Nhóm Đồng bộ Túi đồ (`17 05..09`), Sạp (`17 1E..22`), Trứng (`17 33`), Cắt hàng (`17 3B`)

Handler `FUN_007902fb` (`case_020_007902FB_00790ED4.c` + `007902fb_FUN_007902fb.asm.txt`, case 20) điều phối 37 nhánh con của `0x17`.

#### 1. Bốn Frame đồng bộ Túi đồ cốt lõi: `17 05`, `17 06`, `17 08`, `17 09`

| Frame | Callee Handler | Tổng Payload | Stride Record | Bố cục Byte (`P[0..]`) & Hành vi chi tiết | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--:|:--|:--|:--:|
| `17 05` | `FUN_0076bfc8` | `2 + 12×N` | **12 byte** | **Đồng bộ toàn bộ túi đồ 25 ô** (xoá sạch 25 ô `player+0x9E0+slot*4` trước, sau đó nạp `N` record 12B):<br>• `+0`: `slot:u8` (`1..25`)<br>• `+1..2`: `itemId:u16 LE`<br>• `+3`: `qty:u8`<br>• `+4`: `b4:u8` (`+0x08` độ bền/thuộc tính 1)<br>• `+5`: `b5:u8` (`+0x29`)<br>• `+6`: `b6:u8` (`+0x2A`)<br>• `+7`: `b7:u8` (`+0x2B`)<br>• `+8..11`: `exp:u32 LE` (`+0x24`, tính `doben` qua `FUN_0065b580`) | `F4440E00170501E971010000000000000000` | **A** |
| `17 06` | `FUN_0076b458` | **14 byte** | **12 byte** *(không có `slot`)* | **Thêm vật phẩm vào túi (kèm thông báo & âm thanh)**:<br>• `P[0..1]`: `17 06`<br>• `P[2..3]` (`RP[1..2]`): `itemId:u16 LE`<br>• `P[4..5]` (`RP[3..4]`): `qty:u16 LE` (**2 byte!**)<br>• `P[6..9]` (`RP[5..8]`): `b1:u8, b3:u8, b4:u8, b5:u8`<br>• `P[10..13]` (`RP[9..12]`): `exp:u32 LE`<br>Gọi `FUN_0072b6c8` tự tìm ô gộp/trống trong túi, phát `WA0014.wav` và hiện `"Nhận được <tên> <qty>"`. | `F4440E001706E9710A000000000000000000` | **A** |
| `17 08` | `FUN_0076974c` | **14 byte** | **12 byte** *(có `slot`)* | **Ghi/Cập nhật trực tiếp 1 ô túi đồ xác định**:<br>• `P[0..1]`: `17 08`<br>• `P[2]` (`RP[1]`): `slot:u8` (`1..25`)<br>• `P[3..4]` (`RP[2..3]`): `itemId:u16 LE`<br>• `P[5]` (`RP[4]`): `qty:u8` (**1 byte!**)<br>• `P[6..9]` (`RP[5..8]`): `b5:u8, b6:u8, b7:u8, b8:u8`<br>• `P[10..13]` (`RP[9..12]`): `exp:u32 LE`<br>Ghi vào `player+0x9E0+slot*4` (method `VMT+0x04`); nếu `gvar_007D9C68+0x1D2 == 0` thì hiện thông báo nhận vật phẩm. | `F4440E00170801E971050000000000000000` | **A** |
| `17 09` | `FUN_0076ae24` | **4 byte** | 2 byte | **Trừ / Xoá vật phẩm khỏi 1 ô túi đồ**:<br>• `P[0..1]`: `17 09`<br>• `P[2]` (`RP[1]`): `slot:u8` (`1..25`)<br>• `P[3]` (`RP[2]`): `qty:u8`<br>Gọi `FUN_0072b8c8(*player, slot, qty)` giảm số lượng tại ô `slot` (nếu về 0 thì xoá item), hiện thông báo mất vật phẩm nếu không bị tắt cờ. | `F444040017090105` | **A** |

*(Ghi chú bổ sung: `17 07 <slot:u8> <qty:u8>` (4 byte, `FUN_0076b9e0`) cũng gọi `FUN_0072b8c8` tương tự `17 09` khi đánh rơi/vứt vật phẩm).*

#### 2. Năm Frame Sạp / Stall (`17 1E..22`)

| Frame | Callee Handler | Tổng Payload | Bố cục Byte (`P[0..]`) & Tác động lên State / UI | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `17 1E` | `FUN_0076f92c` | **3 byte** | `17 1E <result:u8>` — Phản hồi kết quả mở sạp cho chủ sạp:<br>• `result == 1`: Mở sạp thành công (`player+0x14BC = 1`, ẩn `TCY_VenderMenu`).<br>• `result == 2` hoặc `3`: Hiện thông báo lỗi (`"Không thể bày bán tại đây"` / thiếu thẻ). | `F4440300171E01` | **A** |
| `17 1F` | `FUN_007311c8` | `2 + Σ(6+L)` | `17 1F` + chuỗi các record độ dài thay đổi `[accId:u32 LE][nameLen:u8][boardName:nameLen][stallStyle:u8]` (`007311c8_FUN_007311c8.c:61-147`) — Đồng bộ danh sách các sạp đang mở trên bản đồ hiện tại (gọi `FUN_0071e164` gắn bảng hiệu lên nhân vật). | `F4440900171F11270000014101` | **A** |
| `17 20` | Inline `0x00790A3F` | **7 byte** | `17 20 <accId:u32 LE> <state:u8>` — Cập nhật trạng thái đóng/mở sạp của 1 người chơi `accId` trên bản đồ (`FUN_0071e234`). | `F444070017201127000000` | **A** |
| `17 21` | `FUN_00610290` | `18 + 16×N` | **Mở `TSallForm` (`*gvar_007DA2F8`) hiển thị danh sách hàng của sạp người khác (`00610290_FUN_00610290.c`)**:<br>**Header 18 byte (`P[0..17]`, tức `RP[0..16]`)**:<br>• `P[0..1]`: `17 21`<br>• `P[2..5]` (`RP[1..4]`): `vendorUid:u32 LE` → `form+0x130`<br>• `P[6..9]` (`RP[5..8]`): `card1:u32` (đọc `u16 LE` qua `FUN_0077eb9c`) → `form+0x214`<br>• `P[10..13]` (`RP[9..12]`): `card2:u32` (đọc `u16 LE` qua `FUN_0077eb9c`) → `form+0x218`<br>• `P[14..17]` (`RP[13..16]`): `cardPrice:u32 LE` (qua `FUN_0077ed68`) → `form+0x21C`<br>**Theo sau bởi `N` record × 16 byte (`P[18..]`, `N = (len(RP)-17)/16`, tối đa 10 ô)**, ghi vào `form + 0x164 + i*0x11`:<br>• `+0`: `slot:u8` → `+0x164`<br>• `+1..2`: `itemId:u16 LE` → `+0x165`<br>• `+3`: `qty:u8` → `+0x167`<br>• `+4`: `b4:u8` → `+0x16C`<br>• `+5..8`: `price:u32 LE` → `+0x168`<br>• `+9`: `b9:u8` → `+0x16D`<br>• `+10`: `b10:u8` → `+0x16E`<br>• `+11`: `b11:u8` → `+0x16F`<br>• `+12..15`: `exp:u32 LE` → `+0x171` (và tính `doben:u8` lưu vào `+0x170`). | `F444220017211127000000000000000000000000000001E9710100E803000000000000000000` | **A** |
| `17 22` | `FUN_0060e3cc` | **10 byte** | **Thông báo bán được hàng / đóng sạp cho chủ sạp (`0060e3cc_FUN_0060e3cc.c`)**:<br>• `P[0..1]`: `17 22`<br>• `P[2]` (`RP[1]`): `mode:u8` (`1` = bán được 1 món, `2` = kết thúc/thu sạp)<br>• `P[3..4]` (`RP[2..3]`): `itemId:u16 LE`<br>• `P[5]` (`RP[4]`): `qty:u8`<br>• `P[6..9]` (`RP[5..8]`): `goldEarned:u32 LE` (cộng dồn vào `TCY_VenderMenu+0x218` và ghi log chat). | `F4440A00172201E97101E8030000` | **A** |

#### 3. Các Frame `17 31..34` (Đập trứng `TFRollEgg`), `17 37` (`TFBlissBag`), `17 3B` (`TFCutItem`)
- **`17 31..34` (`TFRollEgg`, `*gvar_007DA630`)**: Mở bảng đập trứng, quay trứng và nhận thưởng. Bảng giá/điểm cược của `TFRollEgg` được nạp lúc khởi động từ file `Data\RG.Dat` vào `0x009D1610 = [0, 100, 100, 100]` (`RG_DAT_DECODED.md`).
- **`17 37` (`TFBlissBag`, `*gvar_007DA434`)**: Túi phúc (dữ liệu từ `Data\BlissBag.Dat`).
- **`17 3B <res:u8>` (`TFCutItem`, `*gvar_007DA13C`)**: Kết quả cắt/tách vật phẩm (`FUN_0062e9bc`).

### 6.5.4 Nhóm Giao dịch 2 người chơi & Võ tướng (`0x19`)

Handler `FUN_0079157c` (`0079157c_FUN_0079157c.c` / `.asm.txt`, case 22) xử lý 11 nhánh S→C:

| Frame | Callee Handler | Tổng Payload | Bố cục Byte (`P[0..]`) & Side-effect lên State / UI | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `19 01` | `FUN_0075ae20` | **6 byte** | `19 01 <partnerUid:u32 LE>` — Mở cửa sổ `TTradeForm` (`*gvar_007DA3C4`), lưu UID đối tác vào `+0x21C`, kiểm tra không phải Thiên Thần (`0x75AF20`) và không trong chiến đấu (`0x75B3A0`). | `F4440600190111270000` | **A** |
| `19 02` | `FUN_0075aff4` | **3 byte** | `19 02 <resultCode:u8>` — Kết quả/Thông báo trạng thái giao dịch vật phẩm (`0075aff4_FUN_0075aff4.c`):<br>• `1`: Đối phương rớt mạng (`0x75BBEC`)<br>• `2`: Đối phương đang bận (`0x75BC20`)<br>• `3`: Đối phương huỷ bỏ giao dịch (`0x75BC58`, đóng `TTradeForm`)<br>• `4`: **Song phương giao dịch hoàn thành** (`0x75BC7C`, đóng `TTradeForm`)<br>• `5`: Song phương quang cảnh bất đồng (`0x75BCA8`)<br>• `6`: Không đủ tiền bạc, huỷ giao dịch (`0x75BCE8`) | `F4440300190204` | **A** |
| `19 03` | `FUN_0075b3ec` | `6 + 11×N` (`N <= 5`) | **Hiển thị Tiền + Vật phẩm của đối tác trên `TTradeForm` (`0075b3ec_FUN_0075b3ec.c`)**:<br>• `P[0..1]`: `19 03`<br>• `P[2..5]` (`RP[1..4]`): `partnerGold:u32 LE` → hiển thị vào label `+0x210`<br>• `P[6..]` (`RP[5..]`): `N` record × **11 byte** (`N = (len(RP)-5)/11`, tối đa 5 món), mỗi record:<br>  - `+0..1`: `itemId:u16 LE`<br>  - `+2`: `qty:u8`<br>  - `+3..6`: `b3:u8, b4:u8, b5:u8, b6:u8`<br>  - `+7..10`: `exp:u32 LE`<br>  *(Không có byte `slot`!)* Đồng thời bật cờ đối tác đã đặt đồ `*(u8 *)(TTradeForm + 0x229) = 1`. | `F44411001903E8030000E971010000000000000000` | **A** |
| `19 04` | Inline `0x00791614` | **2 byte** | `19 04` — Xoá cờ khoá giao dịch `*(u8 *)(*gvar_007DA5A0 + 0xA093) = 0` và đóng `TTradeForm` (`VMT+0x24`). | `F44402001904` | **A** |
| `19 0A` | `FUN_0075bf7c` | `6 + var` | `19 0A <partnerUid:u32 LE> ...` — Mở `TNPCTradeForm` (`*gvar_007DA438`) hoặc cập nhật thông tin võ tướng của đối tác trên cửa sổ giao dịch võ tướng (`0075bf7c_FUN_0075bf7c.c`). | `F4440600190A11270000` | **A** |
| `19 0B` | `FUN_0075b8e0` | **3 byte** | `19 0B <resultCode:u8>` — Kết quả/Thông báo giao dịch Võ tướng (`0075b8e0_FUN_0075b8e0.c`): mã `1..6` giống `19 02`, thêm mã `7..13` (`0x07..0x0D`) kiểm tra trùng võ tướng, quá 4 tướng, độ trung thành = 0, trùng xe mã (xem §6.8). | `F4440300190B04` | **A** |
| `19 0C` | Inline `0x00791656` | **2 byte** | `19 0C` — Xoá cờ `PlayerState+0xA093 = 0` và đóng `TNPCTradeForm` (`*gvar_007DA438`). | `F4440200190C` | **A** |
| `19 14` | `FUN_0075c3bc` | `2 + var` | Mở / Cập nhật form giao dịch tổng hợp `*gvar_007DA32C`. | `F44402001914` | **A** |
| `19 15` | `FUN_0075c004` | **3 byte** | `19 15 <code:u8>` — Kết quả giao dịch trên form `*gvar_007DA32C`. | `F4440300191501` | **A** |
| `19 16` | Inline `0x00791698` | **2 byte** | `19 16` — Xoá cờ `PlayerState+0xA093 = 0` và đóng form `*gvar_007DA32C`. | `F44402001916` | **A** |
| `19 1F` | `FUN_0075c738` | `2 + var` | Phản hồi thu thập / trao đổi tài nguyên (`*gvar_007D9E70`). | `F4440300191F01` | **A** |

### 6.5.5 Nhóm Kho / Container (`0x1E`)

Handler `FUN_007921a6` (`case_026_007921A6_007922E5.c` + `007921a6_FUN_007921a6.asm.txt`, case 26) xử lý 13 nhánh (`0x01..0x0D`) tác động lên 3 vùng container vật phẩm:
1. **Container A — Tiền trang vật phẩm (50 ô)**: `*(TThing **)(*gvar_007DA7BC + 0x0A48 + slot*4)` (`slot = 1..50`).
2. **Container B — Kho nhà trọ / Thương nhân (25 ô)**: `*(TThing **)(*gvar_007DA58C + 0x04 + slot*4)` (`slot = 1..25`).
3. **Container C — Kho phụ (5 ô)**: `*(TThing **)(*gvar_007DA58C + 0x6C + slot*4)` (`slot = 1..5`).

| Frame | Callee Handler | Tổng Payload | Container đích | Bố cục Byte (`P[0..]`) & Hành vi chi tiết | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--:|:--|:--|:--:|
| `1E 01` | `FUN_0076fc4c` | `2 + 12×N` | Kho B (25 ô) | Nạp danh sách `N` record **12 byte** (`[slot:u8][itemId:u16][qty:u8][b4..b7:4B][exp:u32]`) vào `gvar_007DA58C + 0x04 + slot*4`. | `F4440E001E0101E971010000000000000000` | **A** |
| `1E 02` | `FUN_0076fe88` | **13 byte** | Kho B (25 ô) | Thêm vật phẩm vào Kho B (`11B` data: `itemId:u16, qty:u8, b4..b7:4B, exp:u32`, tự tìm ô trống). | `F4440D001E02E971010000000000000000` | **A** |
| `1E 03` | `FUN_0077007c` | **4 byte** | Kho B (25 ô) | `1E 03 <slot:u8> <qty:u8>` — Trừ/xoá `qty` vật phẩm tại ô `slot` (`1..25`) của Kho B. | `F44404001E030101` | **A** |
| `1E 04` | `FUN_00770528` | **2 byte** | Kho B (25 ô) | `1E 04` — Xoá trắng toàn bộ 25 ô của Kho B (`slot = 1..25`, gọi `VMT+0x08` Clear). | `F44402001E04` | **A** |
| `1E 05` | `FUN_007707a0` | `2 + 12×N` | **Kho A (50 ô)** | **Đồng bộ Tiền trang vật phẩm 50 ô (`007707a0_FUN_007707a0.c`)**: đọc `N = (len(RP)-1)/12` record **12 byte**:<br>`[slot:u8 (1..50)][itemId:u16 LE][qty:u8][b4:u8][b5:u8][b6:u8][b7:u8][exp:u32 LE]`, ghi vào `player + 0x0A48 + slot*4`. | `F4440E001E0501E971010000000000000000` | **A** |
| `1E 06` | `FUN_007701d8` | **13 byte** | Kho A (50 ô) | Thêm vật phẩm vào Tiền trang 50 ô (`itemId:u16, qty:u8, b4..b7:4B, exp:u32`). | `F4440D001E06E971010000000000000000` | **A** |
| `1E 07` | `FUN_007703c8` | **4 byte** | Kho A (50 ô) | `1E 07 <slot:u8> <qty:u8>` — Trừ/xoá `qty` vật phẩm tại ô `slot` (`1..50`) của Tiền trang. | `F44404001E070101` | **A** |
| `1E 08` | `FUN_007706ac` | **2 byte** | Kho A (50 ô) | `1E 08` — Xoá trắng toàn bộ 50 ô Tiền trang vật phẩm (`slot = 1..50`). | `F44402001E08` | **A** |
| `1E 09` | `FUN_007709e0` | `2 + 12×N` | Kho C (5 ô) | Nạp danh sách `N` record **12 byte** vào Kho phụ 5 ô (`gvar_007DA58C + 0x6C + slot*4`, `slot = 1..5`). | `F4440E001E0901E971010000000000000000` | **A** |
| `1E 0A` | `FUN_00770c1c` | **13 byte** | Kho C (5 ô) | Thêm vật phẩm vào Kho phụ 5 ô. | `F4440D001E0AE971010000000000000000` | **A** |
| `1E 0B` | `FUN_00770e0c` | **4 byte** | Kho C (5 ô) | `1E 0B <slot:u8> <qty:u8>` — Trừ/xoá `qty` vật phẩm tại ô `slot` (`1..5`) của Kho phụ. | `F44404001E0B0101` | **A** |
| `1E 0C` | `FUN_00770f68` | **2 byte** | Kho C (5 ô) | `1E 0C` — Xoá trắng 5 ô của Kho phụ (`slot = 1..5`). | `F44402001E0C` | **A** |
| `1E 0D` | Inline `0x007922B1` | **2 byte** | UI (`0x7DA63C`) | `1E 0D` — Gọi `(**(code **)(**gvar_007DA63C + 0x20))()` để mở Form Kho phụ 5 ô. | `F44402001E0D` | **A** |

### 6.5.6 Nhóm Tài nguyên (`0x08`), Hệ số Nhà trọ (`0x23`), Cờ bạc & Lotto (`0x35`–`0x3E`)

| Frame | Handler Addr | Tổng Payload | Bố cục Byte (`P[0..]`) & Side-effect lên State / UI | Ví dụ Hex (Chưa XOR) | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `02 0B` | `0x0078B6FC` (`0078b6fc.c`) | **2 byte** | `02 0B` — Kiểm tra `player+0x12F8 >= 100.000` (`0x186A0`); nếu đủ thì mở `TFCreateOrganize` (`*gvar_007D9CC0`, tạo Quân Đoàn), nếu không đủ thì hiện Banner `"Số tiền trên người bạn không đủ nhé!"` (`0x798370`). **Không phải chat text!** | `F4440200020B` | **A** |
| `08 01` | `0x0078D125` (`case_009`) | `2 + 5×N` | Cập nhật thuộc tính nhân vật/tài nguyên theo mã `attrId:u8` + `val:u32 LE`:<br>• `attrId = 0x26` (38): Set Thủy binh (`player+0x0502`) qua `FUN_0072cbc4`<br>• `attrId = 0x27` (39): Set Tài bảo (`player+0x050E`) qua `FUN_0072cca0`<br>• `attrId = 0x28` (40): Set Đạn dược (`player+0x0513`) qua `FUN_0072cd7c` | `F4440700080126E8030000` | **A** |
| `08 06` | `0x0078D125` (`case_009`) | `2 + 5×N` | Cộng delta thuộc tính/tài nguyên (`attrId = 0x26/0x27/0x28` gọi `FUN_0072ca18`/`ca80`/`cae8`, trần `30.000.000`, kèm thông báo `"Nhận được ... Thủy binh / Tài bảo / Điểm đạn dược"`). | `F444070008062664000000` | **A** |
| `18 01..06` | `0x00790ED5` (`00790ed5.c`) | `2..10B` | Cập nhật và mở `TBankForm` (`*gvar_007DA41C`) / kho lương thực thủy binh (`*gvar_007DA7C0`). | `F44402001801` | **A** |
| `23 04` | `0x00792BAD` (`00792bad.c`) | **4 byte** | `23 04 <A:u8> <B:u8>` — Tính `*(u16 *)(*player + 0x146C) = (uint)A * 100 + (uint)B` rồi hiện Banner `"Khách hàng quý mến, có rảnh thì ghé nhé!"` (`0x0079833C`). **Tuyệt đối không phải store HP/SP!** | `F444040023040100` | **A** |
| `35 01..08` | `0x007952AB` (`007952ab.c`) | `2..var` | Cập nhật bảng trạng thái mini-game phụ (chỉ có S→C). | `F4440300350101` | **A** |
| `36 <mode>` | `0x00795494` (`00795494.c`) | **2 byte** | `36 <mode:u8>` — Gọi `FUN_004ec4e4(*gvar_007D9D88, RP[0])`: mở/đóng giao diện khu giải trí `TSportManage` (khi `*(u8 *)(*gvar_007D9D88 + 4) != 0`, client chặn giao dịch P2P tại `0x00609218`). | `F44402003601` | **A** |
| `37 01..05` | `0x007954A5` (`007954a5.c`) | `2..var` | Điều khiển máy quay thưởng `*gvar_007DA288` / `*gvar_007DA3B4`. | `F44402003701` | **A** |
| `38 <res>` | `0x00795555` (`00795555.c`) | `2..var` | Gọi `FUN_0054f254` hiển thị kết quả đặt cược (`"Đã tạm ngưng đặt cược..."`, `"Đặt cược thất bại rồi !"`, `"Đặt cược thành công chúc bạn trúng thưởng lớn!"` tại `0x0054F2EC..0x0054F340`). | `F4440300380101` | **A** |
| `39 01..0A` | `0x00795579` (`00795579.c`) | `2..var` | Điều khiển `TMachineManager` (`*gvar_007DA4EC`): mở máy slot (`39 01`), quay số, trả kết quả thắng/thua. | `F44402003901` | **A** |
| `3A 01..03` | `0x007956B9` (`007956b9.c`) | `2..var` | Điều khiển bàn **Tài Xỉu (`TRE_BiDaXiao`, `*gvar_007DA42C`)**: mở bàn (`3A 01`), xúc xắc (`3A 02`), kết quả (`3A 03`). | `F44402003A01` | **A** |
| `3B 01..03` | `0x007956F3` (`007956f3.c`) | `2..var` | Điều khiển bàn **Domino (`TSBDManager`, `*gvar_007DA0F4`)**. | `F44402003B01` | **A** |
| `3C 01..05` | `0x0079575C` (`0079575c.c`) | `2..var` | Điều khiển bàn **Cờ tướng (`TRE_ZMChessMain`, `*gvar_007DA778`)**. | `F44402003C01` | **A** |
| `3D 01..06` | `0x007957DC` (`007957dc.c`) | `2..var` | Điều khiển **Xổ số (`TLottoManager`, `*gvar_007D9F98`)**: mở bảng chọn số, cập nhật giải độc đắc (`Jackpot`), xác nhận vé. | `F44402003D01` | **A** |
| `3E 01..02` | `0x00795A72` (`00795a72.c`) | `2..var` | Công bố kết quả mở thưởng Lotto trên `TLottoManager` (`FUN_00557920` / `FUN_005579a0`). | `F44402003E01` | **A** |

## 6.6 Luồng nghiệp vụ & Sequence Diagrams

### 6.6.1 Luồng 1: Mở Shop NPC → Mua vật phẩm (`14 01..06` → `1B 01`)

```
Người chơi / UI (TInterForm)                  Client (aLogin.exe)                         Server
           |                                           |                                     |
           |--- (1) Click NPC bán hàng --------------->|                                     |
           |                                           |--- (2) C->S 14 01 <npcIdx> -------->|
           |                                           |<-- (3) S->C 14 01 [B2=6, W3=surf] --|
           |                                           |--- (4) Đọc Data\Eve.emg (surface W3)|
           |                                           |    & tra giá mua từ Data\Item.Dat   |
           |<-- (5) Hiển thị TInterForm (0x7DA5A8) ----|                                     |
           |                                           |                                     |
           |--- (6) Chọn dòng index (0-based -> +0x152)|                                     |
           |    & Bấm nút Mua (FUN_005D6ECC) --------->|                                     |
           |                                           |--- (7) Kiểm tra chặn map/shop đặc   |
           |                                           |    biệt & gọi FUN_005D7218:         |
           |                                           |    clamp count <= 1250, kiểm tra    |
           |                                           |    player+0x12F8 >= price * count   |
           |                                           |--- (8) C->S 1B 01 <idx:u8><cnt:u16>->|
           |                                           |                                     |
           |                                           |<-- (9a) S->C 1A 02 <cost:u32 LE> ---| (Trừ kim)
           |                                           |<-- (9b) S->C 17 06 <itemId><cnt>...-| (Thêm item)
           |                                           |<-- (9c) S->C 1B 01 <inner:u8> ------| (Kết quả)
           |<-- (10) Nếu inner != 0: hiện Banner lỗi --|    (Nếu inner == 0: KHÔNG hiện      |
           |         2000ms; cập nhật HUD kim & túi    |     banner vì if (RP[1] != 0))      |
```

**Timeline chi tiết**:
1. **Bước 1–5 (Mở Shop)**: Server gửi `14 01..06` với `B2 = 6` và `W3 = surfaceIdx`. Client không nhận bất kỳ danh sách item hay giá nào từ mạng mà tra `Data\Eve.emg` (`FUN_006df87c`) và `Data\Item.Dat` (`FUN_0065ab58`).
2. **Bước 6–8 (Kiểm tra & Gửi `1B 01`)**: Tại `0x005D6ECC`, client kiểm tra `TInterForm+0x156 != 0`, kiểm tra `count` trong `1..1250` và `player+0x12F8 >= price * count` (`FUN_005D7218`), sau đó gửi `1B 01 <index:u8> <count:u16 LE>` (`0x00786011`).
3. **Bước 9–10 (Phản hồi S→C)**: Các handler S→C (`0x1A`, `0x17`, `0x1B`) hoàn toàn độc lập (**Mức A**); theo chuẩn server TS Online (**Mức B**), server gửi `1A 02` (trừ kim) + `17 06` hoặc `17 08` (thêm item vào túi) + `1B 01 00` (hoặc `1B 01 <1..4, 6>` nếu thất bại).

### 6.6.2 Luồng 2: Mở Shop NPC → Bán vật phẩm (`1B 03` → `1B 02`)

```
Người chơi / UI (TThingSaleForm)              Client (aLogin.exe)                         Server
           |                                           |                                     |
           |<-- (1) Mở TThingSaleForm (VMT+0x20) ------|<-- (1) S->C 1B 03 (F44402001B03) ---|
           |    Quét 25 ô túi (player+0x9E0+slot*4),   |                                     |
           |    tra giá bán từ Data\Item.Dat (+0x5C)   |                                     |
           |                                           |                                     |
           |--- (2) Chọn ô túi (slot 1..25 -> +0x14C)  |                                     |
           |    Nhập số lượng (tối đa 2 chữ số, <=50)  |                                     |
           |    & Bấm nút Bán (FUN_005DCC5C) --------->|                                     |
           |                                           |--- (3) Kiểm tra slot in 1..25,      |
           |                                           |    itemId != 0, 0 < qty <= slotQty  |
           |                                           |    Copy +0x14C -> +0x160, qty->+162 |
           |                                           |--- (4) C->S 1B 02 <slot:u8><qty:u8>->|
           |                                           |                                     |
           |                                           |<-- (5a) S->C 17 09 <slot:u8><qty:u8>| (Trừ item)
           |                                           |<-- (5b) S->C 1A 01 <gold:u32 LE> ---| (Cộng kim)
           |                                           |<-- (5c) S->C 1B 02 <inner:u8> ------| (Kết quả)
           |<-- (6) Cập nhật túi đồ, cộng kim, --------|                                     |
           |        hiện Banner nếu inner != 0         |                                     |
```

**Timeline chi tiết**:
1. **Bước 1**: Server gửi `1B 03` (2 byte). Handler `0x00791D9A` gọi `TThingSaleForm.Show`.
2. **Bước 2–4**: Khi bấm nút Bán (`0x005DCC5C`), client kiểm tra `form+0x14C` thuộc `1..25` và `0 < qty <= slotQty` (`FUN_005DCCE0`). Client **không gửi giá bán lên server** — chỉ gửi `1B 02 <slot:u8> <qty:u8>` (4 byte). Server tự tính tiền bán.
3. **Bước 5–6**: Server gửi `17 09 <slot><qty>` (xoá/giảm item khỏi túi), `1A 01 <gold>` (cộng kim), và `1B 02 00` (hoặc mã lỗi `01..04, 06`).

### 6.6.3 Luồng 3: Giao dịch 2 người chơi — Vật phẩm & Kim (`0x19` sub `01..04`)

```
Người chơi A (Client A)                       Server                              Người chơi B (Client B)
          |                                     |                                           |
          |-- (1) C->S 19 01 <uid_B:u32> ------>|                                           |
          |<-- (2) S->C 19 01 <uid_B:u32> ------|--- (2) S->C 19 01 <uid_A:u32> ----------->|
          |   (Mở TTradeForm 0x7DA3C4)          |                                           | (Mở TTradeForm)
          |                                     |                                           |
          |-- (3) Chọn tối đa 5 ô + nhập kim,   |                                           |
          |   bấm Xác nhận danh sách (00608fa0):|                                           |
          |   Khoá sửa danh sách của mình, gửi: |                                           |
          |   C->S 19 02 <gold:u32><5x(slot,q)> |                                           |
          |------------------------------------>|--- (4) S->C 19 03 <gold:u32><N x 11B> --->|
          |                                     |    (Client B hiện đồ/tiền của A,          |
          |                                     |     bật cờ TTradeForm+0x229 = 1)          |
          |                                     |                                           |
          |<-- (5) S->C 19 03 <goldB><M x 11B> -|<-- (5) C->S 19 02 <goldB><5x(slot,q)> ----|
          |   (Client A hiện đồ/tiền của B,     |                                           |
          |    bật cờ TTradeForm+0x229 = 1)     |                                           |
          |                                     |                                           |
          |-- (6) Bấm nút Giao dịch (00609218): |                                           |
          |   Gán +0x228 = 1, gửi:              |                                           |
          |   C->S 19 03 01 ------------------->|<-- (6) C->S 19 03 01 ---------------------|
          |                                     |                                           |
          |<-- (7a) S->C 17 09 / 17 06 / 1A 01/02 (Đồng bộ túi đồ & kim cho cả 2 bên) ----->|
          |<-- (7b) S->C 19 02 04 --------------|--- (7b) S->C 19 02 04 ------------------->|
          |   ("Song phương giao dịch hoàn thành", đóng TTradeForm, xoá cờ +0xA093)         |
```

### 6.6.4 Luồng 4: Giao dịch Võ tướng (`19 0A..0C`)

```
Client A (TNPCTradeForm 0x7DA438)             Server                              Client B (TNPCTradeForm)
          |                                     |                                           |
          |-- (1) C->S 19 0A <uid_B:u32> ------>|                                           |
          |<-- (2) S->C 19 0A <uid_B + data> ---|--- (2) S->C 19 0A <uid_A + data> -------->|
          |-- (3) Chọn 1 võ tướng (+0x204)      |                                           |
          |   + nhập kim (+0x1DC), gửi:         |                                           |
          |   C->S 19 0B <gold:u32><petSlot:u8>>|--- (4) S->C 19 0A <petInfo của A> ------->|
          |-- (5) Bấm chốt (+0x205 = 1):        |                                           |
          |   C->S 19 0C 01 ------------------->|<-- (5) C->S 19 0C 01 ---------------------|
          |<-- (6) S->C 19 0B 04 (Hoàn thành) --|--- (6) S->C 19 0B 04 (Hoặc mã lỗi 01..0D)>|
          |<-- (7) S->C 19 0C (Đóng form) ------|--- (7) S->C 19 0C (Đóng form) ----------->|
```

### 6.6.5 Luồng 5: Sạp / Stall người chơi (`17 1E..22`)

```
Chủ sạp (TCY_VenderMenu 0x7DA4F8)             Server                        Khách mua (TSallForm 0x7DA2F8)
          |                                     |                                           |
          |-- (1) Chọn tối đa 10 món + nhập giá |                                           |
          |   (FUN_0060dfdc -> +0x140, 100B),   |                                           |
          |   nhập tên sạp, gửi:                |                                           |
          |   C->S 17 1E <100B><name><card><4B>>|                                           |
          |<-- (2) S->C 17 1E 01 (Thành công) --|--- (2) S->C 17 1F <list sạp trên map> --->|
          |                                     |                                           |
          |                                     |<-- (3) Click bảng hiệu: C->S 17 20 <uid> -|
          |                                     |--- (4) S->C 17 21 <18B hdr><N x 16B> ---->|
          |                                     |    (Mở TSallForm hiển thị hàng & giá)     |
          |                                     |                                           |
          |                                     |<-- (5) Chọn ô + số lượng, bấm Mua:        |
          |                                     |    C->S 17 21 <uid:u32><slot:u8><qty:u8> -|
          |<-- (6a) S->C 17 09 <slot><qty> -----|--- (6a) S->C 1A 02 <cost> + 17 06 <item> >|
          |<-- (6b) S->C 1A 01 <goldEarned> ----|--- (6b) S->C 17 21 <18B hdr><cập nhật> -->|
          |<-- (6c) S->C 17 22 01 <id><q><gold>-|                                           |
          |   (Cộng vào +0x218 & ghi log bán)   |                                           |
          |                                     |                                           |
          |-- (7) Bấm Thu sạp: C->S 17 1F ----->|--- (8) S->C 17 20 <uid><state=0> -------->|
```

### 6.6.6 Luồng 6: Tiền trang / Ngân hàng (`0x1D` + `1E 08`)

```
Người chơi / UI (TMoneyAccountForm)           Client (aLogin.exe)                         Server
           |                                           |                                     |
           |<-- (1) Mở TMoneyAccountForm (0x7DA46C) ---|<-- (1) S->C 1D 05 ------------------|
           |                                           |                                     |
           |--- (2A) Bấm "Lưu trữ" (Nộp tiền, 0x5D87B4)|                                     |
           |    Kiểm tra bank < 9.999.999,             |                                     |
           |    0 < amt <= bag (+0x12F8), +0x11C = 1 ->|--- (3A) C->S 1D 02 <amt:u32 LE> --->|
           |                                           |<-- (4A) S->C 1A 02 <amt> (Trừ túi) -|
           |                                           |<-- (4B) S->C 1D 01 <amt> (Cộng bank)|
           |                                           |<-- (4C) S->C 1D 07 (Mở khoá +0x11C)-|
           |                                           |                                     |
           |--- (2B) Bấm "Rút nhận" (Rút tiền, 0x5D85C4)                                     |
           |    Kiểm tra bag < 9.999.999,              |                                     |
           |    0 < amt <= bank (+0x12FC), +0x11D = 1 >|--- (3B) C->S 1D 01 <amt:u32 LE> --->|
           |                                           |<-- (5A) S->C 1D 02 <amt> (Trừ bank)-|
           |                                           |<-- (5B) S->C 1A 01 <amt> (Cộng túi)-|
           |                                           |<-- (5C) S->C 1D 08 (Mở khoá +0x11D)-|
           |                                           |                                     |
           |--- (6) Bấm nút Đóng (0x005D84CC) -------->|--- (7) Ẩn form, gán +0xA096 = 1,    |
           |                                           |    gửi C->S 1E 08 (F44402001E08) -->|
```

### 6.6.7 Luồng 7: Kho vật phẩm (`0x1E`) & Đặt cược Tài Xỉu / Lotto (`0x3A`, `0x3D`)

```
Client (TThingAccountForm / TRE_BiDaXiao)                                                 Server
           |                                                                                 |
           |=== [A] KHO TIỀN TRANG VẬT PHẨM (50 Ô: player+0x0A48) ===========================|
           |<-- (1) S->C 1E 05 <N x 12B> (Đồng bộ 50 ô kho) + S->C 1D 06 (Mở form) ----------|
           |--- (2) Gửi đồ vào kho: C->S 1E 02 <bagSlot:u8><qty:u8> ------------------------>|
           |<-- (3) S->C 17 09 <bagSlot><qty> (Trừ túi) + S->C 1E 06 <11B item> (Thêm kho) --|
           |--- (4) Rút đồ về túi:  C->S 1E 01 <bankSlot:u8><qty:u8> ----------------------->|
           |<-- (5) S->C 1E 07 <bankSlot><qty> (Trừ kho) + S->C 17 06 <12B item> (Thêm túi) -|
           |--- (6) Đóng form (0x00749399): C->S 1E 08 ------------------------------------->|
           |                                                                                 |
           |=== [B] CÁ CƯỢC TÀI XỈU (0x3A) & KẾT QUẢ SÒNG BẠC (0x38) ========================|
           |<-- (7) S->C 36 01 (Bật cờ TSportManage+4 = 1, khoá P2P Trade) + S->C 3A 01 -----|
           |--- (8) Chọn cửa (+0x04) & mức cược (+0x0C): C->S 3A <side:u8><wager:u32 LE> --->|
           |<-- (9) S->C 38 <res> (Thông báo đặt cược) + S->C 1A 02 <wager> (Trừ kim) -------|
           |<-- (10) S->C 3A 02/03 (Xúc xắc & kết quả) + S->C 1A 01 <winGold> (Nếu thắng) ---|
```

## 6.7 Mô hình tiền tệ & Bộ đếm tài nguyên

### 6.7.1 Bảng tổng hợp các trường tiền tệ và tài nguyên trên `*gvar_007DA7BC` (`LocalPlayer`)

| Tên tài nguyên | Offset trên `player` | Kiểu dữ liệu | Trần tối đa (Cap) | Hàm Cộng (`+Δ`) | Hàm Trừ (`-Δ`) | Hàm Gán (`=`) | Frame S→C Ghi | Hành vi khi vượt trần / âm | Mức |
|:--|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--|:--:|
| **Kim mang theo** (Gold on hand) | `+0x12F8` | `int32 LE` | `9.999.999` (`0x0098967F`) | `FUN_0072b084` | `FUN_0072b170` | `FUN_0072bcf8` | `1A 01` (`+Δ`)<br>`1A 02` (`-Δ`)<br>`1A 04` (`=`) | **Delta**: Từ chối nguyên tử (return `0`, giữ nguyên giá trị cũ), hiện `"Thất bại  đoạt được"` / `"Thất bại   giảm thiểu"`.<br>**Absolute (`1A 04`)**: Ghi đè trực tiếp, **không clamp**! | **A** |
| **Tiền trang** (Bank Gold) | `+0x12FC` | `int32 LE` | `9.999.999` (`0x0098967F`) | `FUN_0072b9b8` | `FUN_0072ba0c` | `FUN_0072be10` | `1D 01` (`+Δ`)<br>`1D 02` (`-Δ`)<br>`1D 04` (`=`) | **Delta**: Từ chối nguyên tử (return `0`), hiện `"Thất bại lưu trữ"` / `"Thất bại rút nhận"`.<br>**Absolute (`1D 04`)**: Ghi đè trực tiếp, **không clamp**! | **A** |
| **Tiền mềm / Điểm** (Soft Point) | `+0x1300` | `int32 LE` | `2.000.000.000` (`0x77359400`) | `FUN_0072bf08` | `FUN_0072bf5c` *(dead)* | `FUN_0072c070` | `1A 05` (`+Δ`)<br>`1A 03` (`=`) | **Delta (`1A 05`)**: Từ chối nguyên tử nếu `> 2.000.000.000`, hiện `"Thất bại  đoạt được"`.<br>**Absolute (`1A 03`)**: Ghi đè trực tiếp, **không clamp**! (`FUN_0072bf5c` không gắn vào `0x1A`). | **A** |
| **Thủy binh** (Water Soldiers) | `+0x0502` *(đầu mảng `+0x500`)* | `int32 LE` | `30.000.000` (`0x01C9C380`) | `FUN_0072ca18` | `FUN_0072cb50` | `FUN_0072cbc4` | `08 06` (`attr=0x26`)<br>`08 01` (`attr=0x26`) | **Delta (`08 06`)**: Từ chối nguyên tử nếu `> 30.000.000`.<br>**Absolute (`08 01`)**: Ghi đè trực tiếp. | **A** |
| **Tài bảo** (Treasure) | `+0x050E` *(cùng nhóm `+0x506/508/50C`)* | `int32 LE` | `30.000.000` (`0x01C9C380`) | `FUN_0072ca80` | `FUN_0072cc2c` | `FUN_0072cca0` | `08 06` (`attr=0x27`)<br>`08 01` (`attr=0x27`) | **Delta (`08 06`)**: Từ chối nguyên tử nếu `> 30.000.000`.<br>**Absolute (`08 01`)**: Ghi đè trực tiếp. | **A** |
| **Điểm đạn dược** (Ammo Points) | `+0x0513` *(cùng nhóm `+0x512`)* | `int32 LE` | `30.000.000` (`0x01C9C380`) | `FUN_0072cae8` | `FUN_0072cd08` | `FUN_0072cd7c` | `08 06` (`attr=0x28`)<br>`08 01` (`attr=0x28`) | **Delta (`08 06`)**: Từ chối nguyên tử nếu `> 30.000.000`.<br>**Absolute (`08 01`)**: Ghi đè trực tiếp. | **A** |
| **Hệ số giá Nhà trọ / Dịch vụ** | `+0x146C` | `u16 LE` | `65.535` | *(không)* | *(không)* | Inline `23 04` | `23 04 <A:u8><B:u8>` | Tính `*(u16 *)(*player + 0x146C) = A * 100 + B` (`00792bad.c:127`). Dùng để nhân giá dịch vụ nhà trọ/thương nhân. | **A** |

### 6.7.2 Sơ đồ luồng dịch chuyển tiền tệ & Các bẫy Overflow / Underflow

```
                    +---------------------------------------------------+
                    |           SERVER AUTHORITY (Database)             |
                    +---------------------------------------------------+
                       |                      |                      |
        Login / Sync   | 1A 04 (Set Kim)      | 1D 04 (Set Bank)     | 1A 03 (Set Point)
        (Absolute)     v                      v                      v
             +-------------------+  +-------------------+  +-------------------+
             |   KIM MANG THEO   |  |    TIỀN TRANG     |  |  TIỀN MỀM / ĐIỂM  |
             |  (player+0x12F8)  |  |  (player+0x12FC)  |  |  (player+0x1300)  |
             | Trần: 9.999.999   |  | Trần: 9.999.999   |  | Trần: 2.000.000.000|
             +-------------------+  +-------------------+  +-------------------+
               ^       |      ^               ^              ^
   1A 01 (+Δ)  |       |      |  Nộp: C->S 1D 02 / Rút: C->S 1D 01
   1A 02 (-Δ)  |       |      +---< S->C 1A 02 + 1D 01 (Khi Nộp) >---+
               |       |          < S->C 1D 02 + 1A 01 (Khi Rút) >   | 1A 05 (+Δ Point)
               |       v                                             | (Trừ Point phải
     +---------+-------+---------------------------------------------+  dùng 1A 03 Set!)
     | Mua/Bán Shop NPC (1B 01/02)
     | Giao dịch P2P & Võ tướng (19 02/03/0B)
     | Mua/Bán Sạp người chơi (17 21 / 17 22)
     | Cờ bạc Tài Xỉu / Domino / Lotto (3A, 3B, 3D)
```

**Bẫy kỹ thuật bắt buộc lưu ý cho lập trình viên Server**:
1. **Không gửi `1A 01` hoặc `1D 01` khi tổng vượt `9.999.999`**: Vì `FUN_0072b084` (`0072b084_FUN_0072b084.c:30`) kiểm tra `if (9999999 < *(int *)(param_1 + 0x12f8) + param_2) return 0;`, nếu server lỡ cộng vượt trần và gửi `1A 01`, client sẽ **bỏ qua toàn bộ frame** (số kim hiển thị trên client giữ nguyên giá trị cũ trong khi server đã cộng, gây lệch state nghiêm trọng).
2. **Trừ điểm (`player+0x1300`) bắt buộc phải gửi `1A 03 <newTotal:u32 LE>`**: Không có frame S→C `1A` nào gọi hàm trừ điểm `FUN_0072bf5c`. Khi người chơi tiêu điểm (Point), server phải tính số dư mới và gửi `1A 03 <newBalance:u32 LE>`.
3. **Phép kiểm tra `int32` có dấu**: Trong `FUN_0072b084`, phép cộng `*(int *)(player + 0x12f8) + (int)delta` là số nguyên 32-bit có dấu (`JLE` trong ASM). Nếu server gửi `delta >= 0x80000000` (số âm khi ép kiểu `int32`), tổng sẽ nhỏ hơn `9.999.999` và làm âm số kim trên client! Server phải luôn chặn `0 <= delta <= 9.999.999`.

## 6.8 Bảng mã lỗi & Thông báo VISCII 1.1

Toàn bộ chuỗi bên dưới được giải mã trực tiếp từ byte thô trong `lit_797ee8_798440.hex`, `lit_7973e0.hex`, `lit_75af20.hex`, `lit_75b3a0_75b3d0.hex`, `lit_75bbec_75bf14.hex`, `gap_5d75bb_5d7658_money_literals.hex`, `gap_74592b_745acc_trade_strings.hex`, `lit_611460_6114c0.hex`, và `gap_54f2e1_54f384_literals.hex` theo bảng mã **VISCII 1.1 (RFC 1456)**.

### 6.8.1 Bảng mã kết quả Giao dịch Shop NPC (`0x1B` — `00791d80_FUN_00791d80.c/.asm.txt`)

| Frame S→C | `outer` | `inner` | Địa chỉ Literal | Chuỗi VISCII 1.1 đã giải mã (Chính xác từng ký tự) | Kênh hiển thị & Thời lượng | Hành vi thực tế trong Client (`00791d80.asm.txt`) | Mức |
|:--:|:--:|:--:|:--:|:--|:--|:--|:--:|
| `1B 01 00` | `0x01` (Mua) | `0x00` | `0x00798118` | `"Giao dịch thành công"` | **Không hiện!** *(Bị chặn bởi `if (RP[1] != 0)`)* | Gán `local_2c = 0x798118`, nhưng tại `0x00791E59`: `CMP byte [EAX+1], 0; JZ 0x00791E63` nhảy qua lệnh gọi `Banner`! | **A** |
| `1B 01 01` | `0x01` (Mua) | `0x01` | `0x00798138` | `"Xin lỗi, vàng của bạn không đủ!"` | Banner (`*gvar_007DA084` VMT `+0x90`), `2000ms` (`0x7D0`) | Hiện thông báo lỗi giữa màn hình trong 2 giây. | **A** |
| `1B 01 02` | `0x01` (Mua) | `0x02` | `0x00798160` | `"Xin lỗi, bảng vật phẩm của bạn đã đầy!"` | Banner (`+0x90`), `2000ms` | Hiện thông báo túi đồ 25 ô đã đầy. | **A** |
| `1B 01 03` | `0x01` (Mua) | `0x03` | `0x00798190` | `"Xin lỗi, Số lượng vật phẩm bạn mua đã đầy!"` | Banner (`+0x90`), `2000ms` | Hiện thông báo vượt giới hạn chồng đồ (stack > 50). | **A** |
| `1B 01 04` | `0x01` (Mua) | `0x04` | `0x007981C4` | `"Xin lỗi, giao dịch thất bại"` | Banner (`+0x90`), `2000ms` | Hiện thông báo giao dịch mua thất bại chung. | **A** |
| `1B 01 05` | `0x01` (Mua) | `0x05` | *(Không có case)* | *(Chuỗi rỗng `NULL`)* | **Banner rỗng `2000ms` (Bug client!)** | Do `DEC AL` (từ 4 giảm 2 lần thành 6) nhảy qua `5`, `local_2c` vẫn là `0` nhưng `RP[1] != 0` nên gọi `Banner(NULL, 2000)`! | **A** |
| `1B 01 06` | `0x01` (Mua) | `0x06` | `0x007981E8` | `"Hệ thống gián đoạn giao dịch"` | Banner (`+0x90`), `2000ms` | Hiện thông báo gián đoạn giao dịch. | **A** |
| `1B 02 00` | `0x02` (Bán) | `0x00` | `0x00798118` | `"Giao dịch thành công"` | **Không hiện!** *(Bị chặn bởi `if (RP[1] != 0)`)* | Giống `1B 01 00`: gán chuỗi nhưng nhảy qua `Banner`. | **A** |
| `1B 02 01` | `0x02` (Bán) | `0x01` | `0x00798210` | `"Xin lỗi, vàng của bạn đã đầy"` | Banner (`+0x90`), `2000ms` | Hiện khi bán đồ làm kim vượt trần `9.999.999`. | **A** |
| `1B 02 02` | `0x02` (Bán) | `0x02` | `0x00798238` | `"Xin lỗi, bảng vật phẩm của bạn không có vật phẩm này!"` | Banner (`+0x90`), `2000ms` | Hiện khi ô túi gửi lên không khớp vật phẩm trên server. | **A** |
| `1B 02 03` | `0x02` (Bán) | `0x03` | `0x00798278` | `"Xin lỗi, Vật phẩm bạn bán không có nhiều như thƯ này!"` *(byte `0xBF` = `Ư`)* | Banner (`+0x90`), `2000ms` | Hiện khi số lượng bán lớn hơn số lượng thực có trong ô túi. | **A** |
| `1B 02 04` | `0x02` (Bán) | `0x04` | `0x007981C4` | `"Xin lỗi, giao dịch thất bại"` | Banner (`+0x90`), `2000ms` | Hiện thông báo giao dịch bán thất bại chung. | **A** |
| `1B 02 05` | `0x02` (Bán) | `0x05` | *(Không có case)* | *(Chuỗi rỗng `NULL`)* | **Banner rỗng `2000ms` (Bug client!)** | Không có case `5` (hoặc `>= 7`), gọi `Banner(NULL, 2000)`. | **A** |
| `1B 02 06` | `0x02` (Bán) | `0x06` | `0x007981E8` | `"Hệ thống gián đoạn giao dịch"` | Banner (`+0x90`), `2000ms` | Hiện thông báo gián đoạn giao dịch. | **A** |

### 6.8.2 Bảng mã kết quả Giao dịch P2P (`19 02`) & Giao dịch Võ tướng (`19 0B`)

Trong `FUN_0075aff4` (`19 02`, vật phẩm) và `FUN_0075b8e0` (`19 0B`, võ tướng), mã `RP[1]` (`1..13`) hiển thị qua Banner (`*gvar_007DA084` VMT `+0x90`, thời lượng `1200ms` = `0x4B0`):

| Mã `RP[1]` | Áp dụng cho | Địa chỉ Literal | Chuỗi VISCII 1.1 đã giải mã | Ý nghĩa nghiệp vụ & Side-effect | Mức |
|:--:|:--:|:--:|:--|:--|:--:|
| `0x01` (1) | `19 02`, `19 0B` | `0x0075BBEC` | `"Đối phương đã rớt mạng, không thể giao dịch"` | Huỷ giao dịch, đóng form trade, xoá cờ `+0xA093`. | **A** |
| `0x02` (2) | `19 02`, `19 0B` | `0x0075BC20` | `"Đối phương đang bận rộn, không thể giao dịch"` | Đối tác đang mở form khác / đang bận. | **A** |
| `0x03` (3) | `19 02`, `19 0B` | `0x0075BC58` | `"Đối phương hủy bỏ giao dịch"` | Đối tác bấm huỷ; đóng form trade, xoá cờ `+0xA093`. | **A** |
| `0x04` (4) | `19 02`, `19 0B` | `0x0075BC7C` | `"Song phương giao dịch hoàn thành"` | **Giao dịch thành công!** Đóng form trade, xoá cờ `+0xA093`. | **A** |
| `0x05` (5) | `19 02`, `19 0B` | `0x0075BCA8` | `"Song phương quang cảnh bất đồng, không thể giao dịch"` | Hai người chơi không đứng cùng bản đồ (map). | **A** |
| `0x06` (6) | `19 02`, `19 0B` | `0x0075BCE8` | `"Đối phương nào đó không đủ tiền bạc, hủy bỏ giao dịch"` | Một bên không đủ kim hoặc vượt trần kim `9.999.999`. | **A** |
| `0x07` (7) | Chỉ `19 0B` | `0x0075BD28` | `"Đối phương hoặc bạn đã có võ tướng trùng phúc, hủy bỏ giao dịch"` | Trùng ID võ tướng đang mang theo. | **A** |
| `0x08` (8) | Chỉ `19 0B` | `0x0075BD70` | `"Trong chiến đấu không thể giao dịch"` | Một bên đang trong trận đánh. | **A** |
| `0x09` (9) | Chỉ `19 0B` | `0x0075BD9C` | `"Tư liệu võ tướng đối chiếu có sự sai lệch, hủy bỏ giao dịch"` | Lệch dữ liệu slot võ tướng giữa client và server. | **A** |
| `0x0A` (10) | Chỉ `19 0B` | `0x0075BDE0` | `"Trên thân nhiều nhất chỉ có thể dẫn theo 4 võ tướng, hủy bỏ giao dịch"` | Bên nhận đã đầy 4 ô võ tướng mang theo. | **A** |
| `0x0B` (11) | Chỉ `19 0B` | `0x0075BE30` | `"Tự bạn hoặc đối phương có võ tướng trùng phúc ở nhà trọ, hủy bỏ giao dịch"` | Trùng ID võ tướng đang gửi trong nhà trọ. | **A** |
| `0x0C` (12) | Chỉ `19 0B` | `0x0075BE84` | `"Độ trung thành của võ tướng là 0, không thể giao dịch"` | Võ tướng có độ trung thành (Fidelity) = 0 (hoặc < ngưỡng). | **A** |
| `0x0D` (13) | Chỉ `19 0B` | `0x0075BEC4` | `"tuự mình hoặc đối phương lặp lại xe mã của võ tướng, thủ tiêu giao dịch"` | Lỗi trùng lặp xe/ngựa đi kèm võ tướng. | **A** |

### 6.8.3 Bảng các thông báo Tiền tệ (`0x1A`), Tiền trang (`0x1D`), Sạp (`0x17`), Nhà trọ (`0x23`), Sòng bạc (`0x38`)

| Ngữ cảnh / Frame | Địa chỉ Literal | Chuỗi VISCII 1.1 đã giải mã | Kênh hiển thị & Thời lượng | Điều kiện kích hoạt | Mức |
|:--:|:--:|:--|:--|:--|:--:|
| `1A 01` / `1A 05` | `0x00798038` + `0x79804C` / `0x7973E0` | `"Nhận được "` + `<số>` + `"Tiền"` / `"Điểm"` | Banner / Chat log (`2000ms`) | Nhận kim (`1A 01`) hoặc điểm (`1A 05`) thành công. | **A** |
| `1A 01` / `1A 05` lỗi | `0x0079805C` | `"Thất bại  đoạt được"` | Banner (`2000ms`) | Cộng kim vượt `9.999.999` hoặc cộng điểm vượt `2.000.000.000`. | **A** |
| `1A 02` thành công | `0x00798084` + `0x00798078` | `"Giảm"` + `<số>` + `"kim"` | Banner / Chat log (`2000ms`) | Trừ kim mang theo (`1A 02`) thành công. | **A** |
| `1A 02` lỗi | `0x00798094` | `"Thất bại   giảm thiểu"` | Banner (`2000ms`) | Trừ kim khi số dư hiện tại `< delta`. | **A** |
| `1A 06` | `0x007980B4` | `"Dung lượng tiền bạc không đủ"` | Banner (`2000ms`) | Server báo không đủ sức chứa kim mang theo. | **A** |
| `1D 01` / `1D 02` | `0x007982B8` / `0x007982E4` | `"Lưu trữ"` / `"Rút nhận"` (+ `<số> kim`) | Banner (`2000ms`) | Cộng (`1D 01`) hoặc trừ (`1D 02`) tiền trang thành công. | **A** |
| `1D 01` / `1D 02` lỗi | `0x007982C8` / `0x007982F8` | `"Thất bại lưu trữ"` / `"Thất bại rút nhận"` | Banner (`2000ms`) | Tiền trang vượt `9.999.999` hoặc rút quá số dư. | **A** |
| `1D 03` | `0x00798314` | `"Dung lượng tiền trang không đủ"` | Banner (`2000ms`) | Server báo tiền trang không đủ dung lượng. | **A** |
| `08 06` (`attr=26..28`) | `0x007980DC` / `0x7980F0` / `0x798100` | `"Thủy binh"` / `"Tài bảo"` / `"Điểm đạn dược"` | Banner (`2000ms`) | Cộng tài nguyên Quân Đoàn qua `08 06`. | **A** |
| `02 0B` thiếu kim | `0x00798370` | `"Số tiền trên người bạn không đủ nhé!"` | Banner (`2000ms`) | Khi nhận `02 0B` nhưng `player+0x12F8 < 100.000`. | **A** |
| `23 04` | `0x0079833C` | `"Khách hàng quý mến, có rảnh thì ghé nhé!"` | Banner (`2000ms`) | Khi nhận `23 04 <A><B>` cập nhật `player+0x146C`. | **A** |
| `23 05` (`00792bad.c`) | `0x007983A0` | `"Khách hàng sức khỏe rất tốt, không cần phải nghỉ ngơi nữa"` | Banner (`2000ms`) | Khi máu/SP đã đầy lúc nghỉ nhà trọ. | **A** |
| `23 05` (`00792bad.c`) | `0x007983E4` | `"Đã có một võ tướng tương tự trong nhà trọ"` | Banner (`2000ms`) | Khi gửi võ tướng trùng vào nhà trọ. | **A** |
| Shop bị khoá (`1B 01`) | `0x005D75D0` | `"Vật phẩm này tạm thời không có bán"` | Banner (`1200ms`) | Click Mua tại `(map=0x326F, shop=6)` hoặc `(map=0xD629, shop=3)`. | **A** |
| Sạp (`17 1E` lỗi) | `0x00745974` | `"Không thể bày bán tại đây"` | Banner (`1200ms`) | Mở sạp tại khu vực cấm bày bán. | **A** |
| Mua sạp (`TSallForm`) | `0x00611460` / `0x00611484` | `"Hiện giờ cấm công cụ này"` / `"Tiền bạc không đủ"` | Banner (`1200ms`) | Khi mua hàng từ sạp người khác nhưng bị cấm hoặc thiếu kim. | **A** |
| Cờ bạc (`38` S→C) | `0x0054F2EC` / `0x54F320` / `0x54F340` | `"Đã tạm ngưng đặt cược, xin mời lần sau nha!"` / `"Đặt cược thất bại rồi !"` / `"Đặt cược thành công chúc bạn trúng thưởng lớn!"` | Banner / Chat | Kết quả đặt cược trong `FUN_0054f254` (gọi từ `00795555`). | **A** |

## 6.9 Máy trạng thái giao diện (UI State Machine)

### 6.9.1 Bảng cấu trúc các Form UI Kinh tế, VMT và Offset trường quan trọng

| Tên Class (RTTI) | Biến toàn cục | VMT Addr | Size | Các Offset trường quan trọng (Field Offsets) | Frame Mở / Đóng | Mức |
|:--|:--:|:--:|:--:|:--|:--|:--:|
| `TInterForm` *(Shop Mua & Hội thoại)* | `gvar_007DA5A8` | `0x005D5484` *(chung họ)* | `0x168` | • `+0x152` (`u8`): `selectedIndex` (0-based dòng hàng đang chọn)<br>• `+0x154` (`u16`): `buyCount` (`1..1250`, gửi trong `1B 01`)<br>• `+0x156` (`u8`): `hasSelection` (`!= 0` mới cho bấm Mua) | Mở: `14 01..06` (`B2=6`)<br>Đóng: Click đóng / `14` kết thúc | **A** |
| `TThingSaleForm` *(Shop Bán)* | `gvar_007DA720` | `0x005D5484` | `0x170` | • `+0x120` (`TEdit*`): ô nhập số lượng bán (max 2 ký tự)<br>• `+0x14C` (`u8`): `selectedBagSlot` (`1..25` đang chọn trên lưới)<br>• `+0x160` (`u8`): `saleSlot` (chép từ `+0x14C` khi bấm Bán → `1B 02`)<br>• `+0x162` (`u16`): `saleQty` (`1..50`, byte thấp gửi trong `1B 02`) | Mở: S→C `1B 03`<br>Đóng: Nút đóng (`0x005DCBD0`) | **A** |
| `TMoneyAccountForm` *(Tiền trang Kim)* | `gvar_007DA46C` | `0x005D7C6C` | `0x128` | • `+0x110` (`u32`): `inputAmount` (số tiền rút/nộp gửi trong `1D 01/02`)<br>• `+0x11C` (`u8`): cờ khoá nút Lưu trữ (xoá bởi `1D 07`)<br>• `+0x11D` (`u8`): cờ khoá nút Rút nhận (xoá bởi `1D 08`) | Mở: S→C `1D 05`<br>Đóng: Nút đóng (`0x005D84CC`) → gửi C→S `1E 08` | **A** |
| `TThingAccountForm` *(Tiền trang Đồ & Kho)* | `gvar_007DA0A4` | `0x00747D80` | `0x420` | • `+0x3F0` (`AnsiString`): chuỗi `<slot:u8><qty:u8>` gửi trong `1E 01/02/04/05`<br>• `+0x418` (`u8`): `accountMode` (ghi bởi S→C `1D 09`) | Mở: S→C `1D 06`<br>Đóng: Nút đóng (`0x00749399`) → gửi C→S `1E 08` | **A** |
| `TTradeForm` *(Giao dịch P2P)* | `gvar_007DA3C4` | `0x006080B0` | `0x230` | • `+0x1BC..+0x1CC` (`5×4B`): 5 ô đồ của mình<br>• `+0x20C` (`TEdit*`): ô nhập kim của mình<br>• `+0x210`: label hiển thị kim đối tác (`19 03`)<br>• `+0x21C` (`u32`): `partnerUid` (`19 01`)<br>• `+0x224` (`AnsiString`): 10B `5×(slot,qty)` gửi trong `19 02`<br>• `+0x228` (`u8`): `commitFlag` (`1` = đồng ý, `0` = huỷ → `19 03`)<br>• `+0x229` (`u8`): `partnerReady` (`1` khi đã nhận `19 03` từ đối tác) | Mở: S→C `19 01`<br>Đóng: S→C `19 02 03/04`, `19 04`, hoặc nút Huỷ (`19 03 00`) | **A** |
| `TNPCTradeForm` *(Giao dịch Võ tướng)* | `gvar_007DA438` | `0x0060B2A0` | `0x210` | • `+0x1DC` (`TEdit*`): ô nhập kim giao dịch kèm võ tướng<br>• `+0x204` (`u8`): `selectedPetSlot` (`1..4` → gửi trong `19 0B`)<br>• `+0x205` (`u8`): `commitFlag` (`1` hoặc `0` → gửi trong `19 0C`) | Mở: S→C `19 0A`<br>Đóng: S→C `19 0B 03/04`, `19 0C` | **A** |
| `TCY_VenderMenu` *(Bày sạp của mình)* | `gvar_007DA4F8` | `0x0060D210` | `0x228` | • `+0x140` (`100B`): 10 ô × 10B `[bagSlot:u8][qty:u8][price:u32][vit:u32]`<br>• `+0x1A8` (`TEdit*`): tên bảng hiệu sạp<br>• `+0x20C` (`u8`): `cardIdx`<br>• `+0x214` (`AnsiString`): 4B dữ liệu thẻ<br>• `+0x218` (`u32`): tổng kim thu được (`17 22` cộng dồn) | Mở: Nút Bày bán trên UI<br>Đóng: S→C `17 1E 01` hoặc bấm Thu sạp (`17 1F`) | **A** |
| `TSallForm` *(Xem/Mua sạp người khác)* | `gvar_007DA2F8` | `0x0060FA90` | `0x228` | • `+0x130` (`u32`): `vendorUid` (`17 21`)<br>• `+0x134` (`u8`): `buyQty` (số lượng mua gửi trong `17 21`)<br>• `+0x15C` (`u8`): `selectedIdx` (`0..9`)<br>• `+0x164` (`10×17B`): mảng 10 entry (`stride 0x11 = 17B`)<br>• `+0x214/+218/+21C`: `card1`, `card2`, `cardPrice` | Mở: S→C `17 21`<br>Đóng: Nút đóng form | **A** |

### 6.9.2 Bảng ánh xạ Sự kiện Nút bấm UI → Frame C→S

| Form UI | Nút bấm / Thao tác | Địa chỉ Handler | Điều kiện tiên quyết tại Client | Frame C→S phát sinh | Mức |
|:--|:--|:--:|:--|:--:|:--:|
| `TInterForm` | Nút **Mua (`btn_Buy`)** | `0x005D6ECC` | `+0x156 != 0`, không phải 2 shop bị khoá, `FUN_005D7218` trả về `count > 0` (đủ kim, `<= 1250`) | `1B 01 <idx:u8><cnt:u16>` | **A** |
| `TInterForm` | Click mua nhanh 1 món | `0x005D6F7C` | Đang mở shop mua, gán `+0x154 = 1` | `1B 01 <idx:u8><01 00>` | **A** |
| `TThingSaleForm` | Nút **Bán (`btn_Sale`)** | `0x005DCC5C` | `+0x14C` trong `1..25`, ô túi có `itemId != 0`, `0 < qty <= slotQty` (`FUN_005DCCE0`) | `1B 02 <slot:u8><qty:u8>` | **A** |
| `TMoneyAccountForm` | Nút **Rút nhận** | `0x005D85C4` | `+0x11D == 0`, `bag < 9.999.999`, `0 < amt <= bank` | `1D 01 <amt:u32 LE>` | **A** |
| `TMoneyAccountForm` | Nút **Lưu trữ** | `0x005D87B4` | `+0x11C == 0`, `bank < 9.999.999`, `0 < amt <= bag` | `1D 02 <amt:u32 LE>` | **A** |
| `TMoneyAccountForm` | Nút **Đóng (`btn_Close`)** | `0x005D84CC` | Ẩn form (`VMT+0x24`), gán `PlayerState+0xA096 = 1` | `1E 08` | **A** |
| `TThingAccountForm` | Kéo/Chuyển đồ Kho ↔ Túi | `0x007486B8` / `896A` | Ô nguồn có vật phẩm, số lượng hợp lệ | `1E 01` / `1E 02` / `1E 04` / `1E 05` | **A** |
| `TThingAccountForm` | Nút **Đóng (`btn_Close`)** | `0x00749399` | Ẩn form, gán `PlayerState+0xA096 = 1` | `1E 08` | **A** |
| `TTradeForm` | Nút **Đặt đồ (`btn_OK`)** | `0x00608FA0` | Kiểm tra `gold <= player+0x12F8`, đóng gói 5 ô vào `+0x224`, khoá ô nhập | `19 02 <gold><10B items>` | **A** |
| `TTradeForm` | Nút **Giao dịch (`btn_Trade`)** | `0x00609218` | `*(u8 *)(*gvar_007D9D88 + 4) == 0` (không đang đánh bạc), gán `+0x228 = 1` | `19 03 01` | **A** |
| `TCY_VenderMenu` | Nút **Bày bán** | `0x0060DFDC` | Có ít nhất 1 món hợp lệ trong `+0x140`, có thẻ bày bán trong túi | `17 1E <100B><name><card><4B>` | **A** |
| `TCY_VenderMenu` | Nút **Thu sạp** | `0x0060DF5A` | Đang bày sạp | `17 1F` | **A** |
| `TSallForm` | Nút **Mua hàng sạp** | `0x00611080` | Chọn ô `0..9`, `0 < qty <= slotQty`, `player+0x12F8 >= price * qty` | `17 21 <uid><slot><qty>` | **A** |

## 6.10 Kiểm tra hợp lệ phía Client (Client-side Validation) & Trách nhiệm của Server

| Nghiệp vụ | Những gì Client TỰ kiểm tra trước khi gửi (kèm địa chỉ) | Những gì Client KHÔNG kiểm tra (Server BẮT BUỘC phải kiểm tra) | Mức |
|:--|:--|:--|--:|
| **Mua Shop NPC (`1B 01`)** | • Đã chọn dòng hàng (`TInterForm+0x156 != 0` tại `0x005D6EEA`)<br>• Chặn 2 shop `(map=0x326F, id=6)` và `(map=0xD629, id=3)` (`0x005D6F00`)<br>• Ép trần `count <= 1250` (`0x005D7259`: `CMP EAX, 0x4E2`)<br>• Kiểm tra `player+0x12F8 >= unitPrice * count` (`FUN_005D7218`) | • **Không kiểm tra số ô trống trong túi đồ** trước khi gửi `1B 01` (nếu đầy túi, server phải trả `1B 01 02` hoặc `1B 01 03`)!<br>• Không chống gói tin giả mạo `index` vượt quá số dòng của surface hoặc đang đứng xa NPC. | **A** |
| **Bán Shop NPC (`1B 02`)** | • `slot` thuộc `1..25` (`0x005DCC66..0x005DCC74`)<br>• Ô túi có `itemId != 0` (`0x005DCC86`)<br>• Độ dài ô nhập `<= 2` chữ số, `0 < qty <= slotQty` (`FUN_005DCCE0`) | • **Không kiểm tra tràn kim `player+0x12F8 + sellValue > 9.999.999`** trước khi gửi `1B 02` (server phải kiểm tra và trả `1B 02 01`)!<br>• Không kiểm tra cờ vật phẩm cấm bán / nhiệm vụ nếu packet bị tiêm thẳng. | **A** |
| **Tiền trang (`1D 01/02`)** | • Khi Rút (`1D 01`, `0x005D85C4`): kiểm tra `bag < 9.999.999`, `0 < amt <= bank`, cờ khoá `+0x11D == 0`.<br>• Khi Nộp (`1D 02`, `0x005D87B4`): kiểm tra `bank < 9.999.999`, `0 < amt <= bag`, cờ khoá `+0x11C == 0`. | • **Chú ý bẫy tràn**: Client chỉ kiểm tra `bag < 9.999.999` chứ không kiểm tra `bag + amt <= 9.999.999` ở đầu hàm! Nếu `bag = 9.000.000` và rút `2.000.000`, server phải chặn hoặc kẹp số tiền, nếu không `1A 01` trả về sẽ bị client từ chối nguyên tử! | **A** |
| **Giao dịch P2P (`19 02/03`)** | • Kiểm tra không trong trận đánh (`0x75B3A0`), không giao dịch với Thiên Thần (`0x75AF20`), không mở `TSportManage` (`0x00609218`).<br>• Khoá các ô chọn đồ của mình sau khi bấm `19 02` (`0x00608FA0`).<br>• Không cho chọn trùng cùng 1 `bagSlot` vào 2 ô trade. | • **Client không có timer timeout tự động** cho cửa sổ `TTradeForm` — nếu 1 bên treo hoặc ngắt kết nối, server phải chủ động timeout và gửi `19 02 01` / `19 02 03`!<br>• Server phải kiểm tra trùng `bagSlot` trong 5 cặp `(slot, qty)` của `19 02` để chống dupe vật phẩm. | **A** |
| **Sạp / Stall (`17 1E`, `17 21`)** | • Khi mở sạp (`0x0060DFDC`): kiểm tra có thẻ bày bán trong túi, giá `1..9.999.999`, tổng không vượt trần.<br>• Khi mua sạp (`0x00611080`): kiểm tra `player+0x12F8 >= price * qty` và `qty <= slotQty`. | • Server phải kiểm tra kim của chủ sạp (`vendor.gold + price * qty <= 9.999.999`), kiểm tra ô trống túi người mua, và khoá vật phẩm đang treo trên sạp không cho chủ sạp vứt/giao dịch P2P. | **A** |

## 6.11 Trả lời chi tiết 17 câu hỏi bắt buộc (§5)

1. **Danh sách hàng shop NPC có đi trên wire không? Nếu không, client lấy từ file nào, định dạng ra sao?**
   - **Trả lời (Mức A)**: **Không đi trên wire.** Khi mở shop mua, server chỉ gửi `14 01..06` với `B2 = 6` và `W3 = surfaceIdx` (`0078ec3f_FUN_0078ec3f.c`). Client dùng `surfaceIdx` gọi `FUN_006df87c` đọc bảng Surface từ file cục bộ **`Data\Eve.emg`** (mỗi mục hàng dài **5 byte**: `[itemId:u16 LE][kind:u8][style:u8][canCut:u8]`), sau đó dùng `itemId` tra trong **`Data\Item.Dat`** (`FUN_0065ab58`, mỗi record dài **370 byte** = 185 word sau 4 byte header) để lấy tên, hình ảnh, giá mua tại offset `+0x58` (88) và giá bán tại offset `+0x5C` (92). Bằng chứng thêm: handler `0x1B` (`00791d80_FUN_00791d80.asm.txt`) chỉ dài 299 byte và chỉ xử lý 3 nhánh (`1B 01` kết quả mua, `1B 02` kết quả bán, `1B 03` gọi `TThingSaleForm.Show`), hoàn toàn không có vòng lặp đọc danh sách item.
2. **`1B 01` chính xác là `index:u8 + count:u16 LE`? Index 0-based hay 1-based? Count tối đa? Có byte padding nào không?**
   - **Trả lời (Mức A)**: Chính xác tuyệt đối. Tại `0x00786011..0x007860C3` (`0077f414.asm.txt:1821-1867`), payload dài đúng **5 byte**: `1B 01 <index:u8> <count:u16 LE>`, **không có byte padding**. `index` đọc từ `*(u8 *)(*gvar_007DA5A8 + 0x152)` là **0-based** (`0` là dòng đầu tiên trong danh sách shop). `count` đọc từ `*(u16 *)(*gvar_007DA5A8 + 0x154)` (qua `FUN_0077eb1c`), được hàm kiểm tra nút Mua `FUN_005D7218` (`asm_hole_5d6a9b_btnbuy.asm.txt:253`, `0x005D7259: CMP EAX, 0x4E2`) giới hạn tối đa là **1.250** (`0x04E2`, tức 25 ô × 50 cái/ô).
3. **`1B 02` chính xác là `slot:u8 + qty:u8`? `slot` map thế nào vào túi 25 ô?**
   - **Trả lời (Mức A)**: Chính xác. Tại `0x007860C8..0x00786180` (`0077f414.asm.txt:1869-1916`), payload dài đúng **4 byte**: `1B 02 <slot:u8> <qty:u8>`. Tại hàm nút Bán `FUN_005DCC5C` (`asm_hole_5dc98a_btnsale.asm.txt:243-253`, `0x005DCC66..0x005DCC94`), client kiểm tra `CMP byte [EAX+0x14C], 0; JZ 0x005DCCC4` và `CMP EAX, 0x19` (`25`), rồi truy cập mảng túi tại `MOV EAX, [EDX + EAX*4 + 0x9E0]`. Như vậy trên wire client gửi `slot` là **1-based (`1..25`)** tương ứng trực tiếp với 25 ô túi đồ (`slot = 1` ở góc trái trên → `player + 0x09E4`, ..., `slot = 25` → `player + 0x0A44`). `qty` là 1 byte `u8` (`1..50`) lấy từ byte thấp của `TThingSaleForm+0x162`.
4. **`1B 03` do ai gửi, mang ý nghĩa gì, payload có đúng 2 byte không?**
   - **Trả lời (Mức A)**: Do **Server gửi xuống Client (S→C)**, mang ý nghĩa **mở form Bán hàng (`TThingSaleForm`)**, và payload dài đúng **2 byte** (`1B 03`, wire frame `F4 44 02 00 1B 03`). Bằng chứng: tại `00791d80_FUN_00791d80.asm.txt:19-41`, khi `RP[0] == 3`, client gọi `(**(code **)(**(int **)0x007da720 + 0x20))()`; ngược lại ở chiều C→S (`0x00785FFD..0x0078600C`), `0x1B` chỉ xử lý `sub = 1` và `sub = 2`.
5. **Thứ tự frame S→C khi mua thành công? Khi bán thành công? (trừ/cộng tiền trước hay thêm/xoá item trước?)**
   - **Trả lời (Mức A cho tính độc lập của client / Mức B cho thứ tự server)**:
     - Về phía client (**Mức A**): Ba handler `FUN_0079175f` (`0x1A` tiền), `FUN_007902fb` (`0x17` túi đồ) và `FUN_00791d80` (`0x1B` kết quả) xử lý **hoàn toàn độc lập**, không có biến trạng thái ràng buộc thứ tự giữa chúng.
     - Theo luồng nghiệp vụ chuẩn của server (**Mức B**):
       - **Khi mua thành công**: (1) `S→C 1A 02 <cost:u32 LE>` (trừ kim trước) → (2) `S→C 17 06` (hoặc `17 08`, thêm item vào túi) → (3) `S→C 1B 01 00` (kết quả mua thành công).
       - **Khi bán thành công**: (1) `S→C 17 09 <slot:u8> <qty:u8>` (trừ/xoá item khỏi túi trước) → (2) `S→C 1A 01 <gold:u32 LE>` (cộng kim) → (3) `S→C 1B 02 00` (kết quả bán thành công).
6. **`1A 01/02/03/04/05/06` khác nhau thế nào? Cái nào delta, cái nào absolute?**
   - **Trả lời (Mức A — `0079175f_FUN_0079175f.c/.asm.txt`)**:
     - `1A 01 <delta:u32 LE>`: **Delta (`+Δ`)** — Cộng kim mang theo (`player+0x12F8` qua `FUN_0072b084`, trần `9.999.999`).
     - `1A 02 <delta:u32 LE>`: **Delta (`-Δ`)** — Trừ kim mang theo (`player+0x12F8` qua `FUN_0072b170`, không âm).
     - `1A 03 <val:u32 LE>`: **Absolute (`=`)** — Gán tuyệt đối Điểm / Tiền mềm (`player+0x1300 = val` qua `FUN_0072c070`, không clamp).
     - `1A 04 <val:u32 LE>`: **Absolute (`=`)** — Gán tuyệt đối Kim mang theo (`player+0x12F8 = val` qua `FUN_0072bcf8`, không clamp).
     - `1A 05 <delta:u32 LE>`: **Delta (`+Δ`)** — Cộng Điểm / Tiền mềm (`player+0x1300` qua `FUN_0072bf08`, trần `2.000.000.000`).
     - `1A 06` (2 byte): **Thông báo lỗi** — Hiện Banner `"Dung lượng tiền bạc không đủ"` (`0x007980B4`).
7. **`1D 01/02/03/04/05/06` khác nhau thế nào? Sub nào là nộp, sub nào là rút?**
   - **Trả lời (Mức A — đảo chiều giữa C→S và S→C!)**:
     - **Chiều C→S** (`asm_5d8392_5d88dc_bank_buttons.asm.txt`):
       - **C→S `1D 01 <amount:u32 LE>` là RÚT TIỀN** từ Tiền trang về túi (`FUN_005D85C4`).
       - **C→S `1D 02 <amount:u32 LE>` là NỘP TIỀN** từ túi vào Tiền trang (`FUN_005D87B4`).
     - **Chiều S→C** (`00791f76_FUN_00791f76.asm.txt`):
       - **S→C `1D 01 <delta:u32 LE>`**: **Delta (`+Δ`)** — Cộng tiền vào Tiền trang (`player+0x12FC` qua `FUN_0072b9b8`, hiện `"Lưu trữ ... kim"`).
       - **S→C `1D 02 <delta:u32 LE>`**: **Delta (`-Δ`)** — Trừ tiền khỏi Tiền trang (`player+0x12FC` qua `FUN_0072ba0c`, hiện `"Rút nhận ... kim"`).
       - **S→C `1D 03`** (2B): Hiện Banner `"Dung lượng tiền trang không đủ"` (`0x00798314`).
       - **S→C `1D 04 <val:u32 LE>`**: **Absolute (`=`)** — Gán tuyệt đối số dư Tiền trang (`player+0x12FC = val` qua `FUN_0072be10`).
       - **S→C `1D 05`** (2B): Mở `TMoneyAccountForm` (`*gvar_007DA46C`, gửi/rút kim).
       - **S→C `1D 06`** (2B): Mở `TThingAccountForm` (`*gvar_007DA0A4`, gửi/rút vật phẩm).
       - *(Ngoài ra còn `1D 07` xoá cờ `+0x11C`, `1D 08` xoá cờ `+0x11D`, và `1D 09 <mode:u8>` ghi `TThingAccountForm+0x418`)*.
8. **`19 01/02/03/04/…` layout từng frame? Frame nào chứa item của đối tác?**
   - **Trả lời (Mức A)**:
     - **Frame chứa item và tiền của đối tác là S→C `19 03`** (`0075b3ec_FUN_0075b3ec.c`): dài `6 + 11×N` byte (`N <= 5`), gồm `19 03` + `partnerGold:u32 LE` + `N` record **11 byte** (`[itemId:u16 LE][qty:u8][b3:u8][b4:u8][b5:u8][b6:u8][exp:u32 LE]`).
     - Layout các frame khác:
       - `19 01 <uid:u32 LE>` (6B, cả C→S và S→C): Mời / Mở cửa sổ giao dịch với `uid`.
       - C→S `19 02 <myGold:u32 LE> <5 × (bagSlot:u8, qty:u8)>` (16B, `0x007854BF`): Gửi tiền và 5 ô đồ của mình lên bàn trade.
       - S→C `19 02 <resultCode:u8>` (3B, `FUN_0075aff4`): Thông báo kết quả/lỗi giao dịch (`1..6`, trong đó `4` = hoàn thành).
       - C→S `19 03 <action:u8>` (3B, `0x00785569`): Chốt giao dịch (`01`) hoặc Huỷ (`00`).
       - S→C `19 04` (2B, `0x00791614`): Đóng `TTradeForm` và mở khoá `+0xA093` (không có C→S `19 04`).
9. **Trade có chống trùng lặp/dupe không? Client khoá nút sau khi gửi? Có timeout?**
   - **Trả lời (Mức A)**:
     - **Khoá nút & chống sửa sau khi đặt đồ**: Khi bấm nút xác nhận danh sách đồ (`0x00608FA0`, gửi `19 02`), client vô hiệu hoá việc chỉnh sửa 5 ô đồ và ô tiền của mình; khi bấm nút Giao dịch (`0x00609218`, gửi `19 03 01`), client gán `TTradeForm+0x228 = 1` và bật cờ toàn cục `PlayerState+0xA093 = 1` (chỉ được xoá khi nhận `19 02 03/04` hoặc `19 04`).
     - **Chống chọn trùng ô trên UI**: Khi kéo/chọn vật phẩm từ túi vào 5 ô trade, UI kiểm tra không cho phép 2 ô trade trỏ vào cùng 1 `bagSlot`. Tuy nhiên nếu kẻ tấn công bơm thẳng packet `19 02` lặp lại cùng 1 `bagSlot` (ví dụ `01 05 01 05 ...`), **server bắt buộc phải kiểm tra tập `bagSlot` duy nhất**.
     - **Timeout**: **Client KHÔNG có bộ đếm thời gian (timer) timeout** trong `TTradeForm`. Mọi cơ chế timeout đều do server quản lý (gửi `19 02 03` hoặc `19 04` để đóng cửa sổ).
10. **`17 1E` có chứa giá không? `17 21` header bao nhiêu byte, record bao nhiêu byte?**
    - **Trả lời (Mức A)**:
      - **C→S `17 1E` CÓ chứa giá bán của cả 10 ô!** Tại `0077f414_FUN_0077f414.c:2185-2258` (`0x007840A7`), client gửi `17 1E` + **100 byte** mảng `TCY_VenderMenu+0x140` (10 record × 10 byte: `[bagSlot:u8][qty:u8][price:u32 LE][vitality:u32 LE]`) + `boardName` + `cardIdx:u8` + `4B cardData`. Còn **S→C `17 1E`** chỉ dài **3 byte** (`17 1E <result:u8>`).
      - **S→C `17 21` (`00610290_FUN_00610290.c`)**:
        - **Wire Header**: **18 byte** (`P[0..17]`, tương ứng **17 byte** trong `RP[0..16]`), gồm: `17 21` (2B) + `vendorUid:u32 LE` (4B) + `card1:u32` (4B) + `card2:u32` (4B) + `cardPrice:u32 LE` (4B).
        - **Record**: **16 byte / record** (`N = (len(RP) - 17) / 16`, bắt đầu từ `P[18]` = `RP[17]`), gồm: `[slot:u8][itemId:u16 LE][qty:u8][b4:u8][price:u32 LE][b9:u8][b10:u8][b11:u8][exp:u32 LE]`.
      - *(Lưu ý: C→S `17 21` khi khách mua hàng dài **8 byte**: `17 21 <vendorUid:u32 LE> <slot:u8> <qty:u8>` tại `0x00784261`)*.
11. **`17 05/06/08/09` khác nhau ở đâu? Stride record?**
    - **Trả lời (Mức A)**:
      - **`17 05`** (`FUN_0076bfc8`): Đồng bộ toàn bộ túi 25 ô (xoá sạch túi trước rồi nạp `N` record). **Stride = 12 byte/record**: `[slot:u8][itemId:u16 LE][qty:u8][b4:u8][b5:u8][b6:u8][b7:u8][exp:u32 LE]`.
      - **`17 06`** (`FUN_0076b458`): Nhận 1 lô vật phẩm mới vào túi (tự tìm ô trống/cộng dồn, phát `WA0014.wav`, hiện toast `"Nhận được ..."`). Tổng payload **14 byte** (data **12 byte**, **KHÔNG có byte `slot`**, nhưng **`qty` là `u16 LE` 2 byte**): `17 06 [itemId:u16 LE][qty:u16 LE][b1:u8][b3:u8][b4:u8][b5:u8][exp:u32 LE]`.
      - **`17 08`** (`FUN_0076974c`): Ghi đè/cập nhật trực tiếp 1 ô `slot` xác định trong túi. Tổng payload **14 byte** (data **12 byte**, **CÓ byte `slot:u8`**, và **`qty` là `u8` 1 byte**): `17 08 [slot:u8][itemId:u16 LE][qty:u8][b5:u8][b6:u8][b7:u8][b8:u8][exp:u32 LE]`.
      - **`17 09`** (`FUN_0076ae24`): Trừ `qty` vật phẩm khỏi ô `slot` trong túi (`FUN_0072b8c8`). Tổng payload **4 byte** (data **2 byte**): `17 09 [slot:u8][qty:u8]`.
12. **`1E 01/04/05/08/0C/0D` layout? Container nào bao nhiêu ô?**
    - **Trả lời (Mức A — `case_026_007921A6_007922E5.c` & `007707a0_FUN_007707a0.c`)**:
      - Có **3 container** với số ô lần lượt là:
        1. **Tiền trang vật phẩm (`player + 0x0A48 + slot*4`)**: **50 ô** (`slot = 1..50`).
        2. **Kho nhà trọ / Thương nhân (`*gvar_007DA58C + 0x04 + slot*4`)**: **25 ô** (`slot = 1..25`).
        3. **Kho phụ (`*gvar_007DA58C + 0x6C + slot*4`)**: **5 ô** (`slot = 1..5`).
      - Layout S→C của 6 frame được hỏi:
        - `1E 01`: `1E 01` + `N × 12B` (`[slot:u8 (1..25)][itemId:u16][qty:u8][b4..b7:4B][exp:u32]`) — Đồng bộ Kho 25 ô (`+0x04`). *(Ở chiều C→S, `1E 01 <slot:u8><qty:u8>` là rút đồ từ Tiền trang 50 ô)*.
        - `1E 04`: `1E 04` (2 byte) — Xoá trắng toàn bộ 25 ô của Kho `+0x04` (`FUN_00770528`). *(Ở chiều C→S, `1E 04 <slot:u8><qty:u8>` là chuyển đồ Kho 25 ô)*.
        - `1E 05`: `1E 05` + `N × 12B` (`[slot:u8 (1..50)][itemId:u16][qty:u8][b4..b7:4B][exp:u32]`) — Đồng bộ Tiền trang vật phẩm 50 ô (`FUN_007707a0`).
        - `1E 08`: `1E 08` (2 byte) — Ở chiều **S→C**: xoá trắng toàn bộ 50 ô Tiền trang vật phẩm (`FUN_007706ac`); ở chiều **C→S**: báo đóng form Tiền trang/Kho (`0x0078652E`).
        - `1E 0C`: `1E 0C` (2 byte, chỉ S→C) — Xoá trắng 5 ô của Kho phụ `+0x6C` (`FUN_00770f68`).
        - `1E 0D`: `1E 0D` (2 byte, chỉ S→C) — Mở form Kho phụ 5 ô (`*gvar_007DA63C`, `VMT+0x20`).
13. **Ba field tiền có trần bao nhiêu? Frame nào ghi field nào? Có clamp không?**
    - **Trả lời (Mức A)**:
      - **Kim mang theo (`player+0x12F8`)**: Trần **`9.999.999`** (`0x0098967F`). Ghi bởi `1A 01` (`+Δ`), `1A 02` (`-Δ`), `1A 04` (`=`).
      - **Tiền trang (`player+0x12FC`)**: Trần **`9.999.999`** (`0x0098967F`). Ghi bởi `1D 01` (`+Δ`), `1D 02` (`-Δ`), `1D 04` (`=`).
      - **Tiền mềm / Điểm (`player+0x1300`)**: Trần **`2.000.000.000`** (`0x77359400`). Ghi bởi `1A 05` (`+Δ`), `1A 03` (`=`).
      - **Có clamp không?**: **KHÔNG CLAMP ở bất kỳ hàm nào!** Các hàm **Absolute** (`1A 04`, `1D 04`, `1A 03`) ghi đè thẳng giá trị 32-bit vào bộ nhớ mà không kiểm tra trần; còn các hàm **Delta** (`1A 01/02/05`, `1D 01/02`) thì **từ chối nguyên tử (atomic reject)** — nếu vượt trần hoặc âm thì trả về `0` và giữ nguyên số dư cũ, tuyệt đối không tự kẹp về `9.999.999` hay `0`.
14. **Bảng thông báo `0x1B` có bao nhiêu mã? Mã nào là thành công? Mã nào không có case?**
    - **Trả lời (Mức A — `00791d80_FUN_00791d80.asm.txt`)**:
      - Có tất cả **12 nhánh case hợp lệ** (6 mã `inner` = `0, 1, 2, 3, 4, 6` cho `outer = 1` Mua, và 6 mã `inner` = `0, 1, 2, 3, 4, 6` cho `outer = 2` Bán), sử dụng **8 chuỗi VISCII phân biệt** tại `0x00798118..0x00798278`.
      - **Mã thành công**: `inner = 0` (`1B 01 00` và `1B 02 00`), gán chuỗi `"Giao dịch thành công"` (`0x00798118`), nhưng **không hiển thị Banner** do điều kiện `if (RP[1] != 0)` tại `0x00791E59`.
      - **Mã không có case**: **`inner = 5` và mọi `inner >= 7` (`7..255`)** cho cả `outer = 1` và `outer = 2` (vì sau case `4`, lệnh `SUB AL, 0x2` nhảy thẳng sang case `6`), cùng với mọi `outer == 0` hoặc `outer >= 4`. Khi nhận `inner = 5` hoặc `>= 7`, client gọi `Banner` với chuỗi rỗng (`NULL`).
15. **Những chỗ nào client tự validate (tiền, ô trống, số lượng) trước khi gửi?**
    - **Trả lời (Mức A)**: Xem bảng chi tiết tại **§6.10**:
      - Mua shop (`0x005D6ECC` & `FUN_005D7218`): kiểm tra đã chọn dòng (`+0x156 != 0`), chặn 2 shop khoá, kẹp `count <= 1250` và giảm `count` theo `player+0x12F8 / unitPrice` (**nhưng không kiểm tra ô trống trong túi**).
      - Bán shop (`0x005DCC5C` & `FUN_005DCCE0`): kiểm tra `slot` trong `1..25`, `itemId != 0`, `0 < qty <= slotQty` (**nhưng không kiểm tra tràn kim `> 9.999.999`**).
      - Ngân hàng (`0x005D85C4` / `0x005D87B4`): kiểm tra cờ khoá `+0x11C/+0x11D`, kiểm tra `0 < amt <= nguồn`, và kiểm tra đích `< 9.999.999` (**nhưng không kiểm tra `đích + amt <= 9.999.999`**).
      - Sạp (`0x0060DFDC` / `0x00611080`) và Tạo Quân Đoàn (`02 0B` kiểm tra `kim >= 100.000`).
16. **Frame nào client không bao giờ gửi? Chứng minh bằng bảng builder.**
    - **Trả lời (Mức A)**:
      1. **Cấp độ Main Opcode (`cs_bytemap_77f474_200B.hex` + `cs_builder_dword200_0x77f53c.hex`)**: Ba opcode kinh tế **`0x35`, `0x38`, `0x3E`** (cùng các opcode ngoài kinh tế `0x04, 0x11, 0x15, 0x26, 0x2F..0x31, 0x33..0x34, 0x44, 0x49..0xC6`) có `byte_map[op] = 0` → nhảy về `dword_0077F53C[0] = 0x0078A4F2` (Epilogue `RET`).
      2. **Cấp độ Sub-opcode**:
         - **`0x1B`**: Không bao giờ gửi **`1B 03`** (hoặc `1B 00`, `1B 04..FF`) — tại `0x00785FFD..0x0078600C`, chỉ có `DEC AL` cho `1` và `2`, còn lại `JMP 0x0078A4F2`.
         - **`0x1A`**: Không bao giờ gửi **`1A 05`, `1A 06`** — tại `0x00785B4C..0x00785B6B`, chỉ có 4 lần `DEC AL` (`sub = 1..4`), còn lại `JMP 0x0078A4F2`.
         - **`0x1D`**: Tuy builder `0x00786185` không lọc `sub`, chỉ có **2 call-site duy nhất** trong toàn bộ binary gọi `FUN_0077F414(0x1D, sub)` là `0x005D85C4` (`sub = 1`) và `0x005D87B4` (`sub = 2`); client không bao giờ gửi **`1D 03..09`**.
         - **`0x17`** (`cs_op17_targets_0x783405.hex`): Không bao giờ gửi **`17 00, 17 01, 17 05, 17 06, 17 07, 17 08, 17 09, 17 15..17 1D`** (tất cả đều có target `0x0078A4F2`).
         - **`0x19`** (`cs_op19_byte_map_0x7853e5.hex`): Không bao giờ gửi **`19 00, 19 04..09, 19 0D..13, 19 16..1E, 19 20..28`** (đều có `byte_map[sub] = 0` → `0x0078A4F2`).
         - **`0x1E`** (`0x0078620C`): Không bao giờ gửi **`1E 00`** hoặc **`1E 09..0D`** (`CMP EAX, 8; JA 0x0078A4F2`).
17. **Có frame nào server gửi mà client bỏ qua hoặc hiểu sai không?**
    - **Trả lời (Mức A)**: Có **5 trường hợp điển hình** đã chứng minh bằng ASM/C:
      1. **`1B 01 00` và `1B 02 00` (Thành công Shop NPC)**: Client nạp chuỗi `"Giao dịch thành công"` vào biến cục bộ nhưng **bỏ qua không hiển thị** vì lệnh `if (RP[1] != 0)` tại `0x00791E59`.
      2. **`1B 01 05` / `1B 02 05` (hoặc `inner >= 7`)**: Không có nhánh trong switch nhưng không nhảy thoát mà rơi thẳng xuống lệnh gọi `Banner` với con trỏ chuỗi `NULL` (hiện thông báo rỗng 2 giây).
      3. **`1A 01 <delta>` / `1D 01 <delta>` / `1A 05 <delta>` khi tổng vượt trần**: Client **từ chối nguyên tử** (bỏ qua hoàn toàn lượng tiền cộng thêm, không kẹp trần) và chỉ hiện thông báo `"Thất bại  đoạt được"` / `"Thất bại lưu trữ"`, gây lệch số dư giữa client và server.
      4. **`17 21` (Danh sách hàng trên sạp)**: Hai trường `card1` (`RP[5..8]`) và `card2` (`RP[9..12]`) chiếm mỗi trường **4 byte** trên wire (`_LStrCopy(RP, 6, 4)` và `_LStrCopy(RP, 10, 4)`), nhưng client lại truyền chuỗi 4 byte đó vào hàm đọc `u16` `FUN_0077eb9c` (`00610290.c:106-119`), tức **bỏ qua hoàn toàn 2 byte cao** của mỗi trường!
      5. **Mọi Opcode có `s2c_case_index = 0` trong `OPCODE_MAP.csv` (như `0x0A, 0x11, 0x12, 0x15, 0x1C, 0x2F..0x31, 0x44, 0x49..0xC6`)**: Nhảy về epilogue `0x00796347` của `FUN_0078a8dc` và bị client âm thầm bỏ qua.

## 6.12 Điều chưa biết & Phương pháp kiểm chứng

1. **Thứ tự gói tin S→C thực tế từ Server gốc (Official Chinese/Vietnamese Server) khi Mua/Bán/Trade (Mức B)**:
   - *Hiện trạng*: Code client chứng minh các handler `0x17`, `0x1A`, `0x1B`, `0x19` độc lập nhau, không bắt buộc thứ tự.
   - *Cách kiểm chứng*: Chụp pcap từ server chuẩn hoặc kiểm thử với `aLogin.exe` trong môi trường cô lập để xác nhận việc gửi `1B 01 00` trước hay sau `1A 02` + `17 06` có ảnh hưởng tới âm thanh `WA0014.wav` hay không.
2. **Ý nghĩa ngữ nghĩa chính xác của 4 byte thuộc tính phụ trong item record 12 byte (`b4, b5, b6, b7` tại `+0x08, +0x29, +0x2A, +0x2B` của `TThing`) (Mức B)**:
   - *Hiện trạng*: Đã xác định vị trí byte chính xác trong `17 05`, `17 06`, `17 08`, `17 21`, `19 03`, `1E 01`, `1E 05` (gồm thuộc tính nguyên tố, kháng/cộng thêm, trạng thái khoá/độ bền).
   - *Cách kiểm chứng*: Đối chiếu hàm render tooltip vật phẩm (`FUN_0065ab58` / `TThing` methods) với một vật phẩm có ép lông/tâm trong `Item.Dat`.
3. **Giá trị cụ thể của 4 byte `cardData` (`TCY_VenderMenu+0x214`) trong `17 1E` C→S (Mức B)**:
   - *Hiện trạng*: Builder `0x0078415F` đẩy `dword [EAX + 0x214]` (kiểu `AnsiString`) vào cuối gói `17 1E`.
   - *Cách kiểm chứng*: Đặt breakpoint hoặc log gói `17 1E` khi dùng các loại Thẻ Bày Bán khác nhau trong túi đồ.

## 6.13 Phụ lục (Danh mục file, Glossary, Đính chính tài liệu cũ)

### 6.13.1 Danh mục file đã đọc và đối chiếu (Coverage 100%)

| Nhóm tài nguyên | Danh sách File trong `client_economy_handler/` | Mức độ bao phủ |
|:--|:--|:--:|
| **Bản đồ & Ghi chú hệ thống** | `README.md`, `OPCODE_MAP.csv`, `MANIFEST.csv`, `ROUND7_NOTES.md`, `ROUND7_SUMMARY.csv`, `MISSING_DUMPS.md`, `UNRESOLVED.csv`, `VMT_MAP.csv`, `RG_DAT_DECODED.md` | **100%** |
| **Dispatcher C→S & Bảng nhảy** | `0077f414_FUN_0077f414.c`, `0077f414_FUN_0077f414.asm.txt` (4.909 dòng), `cs_bytemap_77f474_200B.hex`, `cs_builder_dword200_0x77f53c.hex`, `cs_op17_targets_0x783405.hex`, `cs_op19_byte_map_0x7853e5.hex`, `cs_op19_targets_0x78540f.hex`, `cs_op19_dispatch_cases.asm.txt`, `asm_cs17_783405_builders.asm.txt`, `asm_cs1e_786221_builders.asm.txt`, `asm_cs1eto1f_786598.asm.txt`, `asm_cs1fto20_7853e5.asm.txt`, `jumptable_byte200_0x78A8EE.hex`, `jumptable_dword200_0x78A9B6.hex` | **100%** |
| **25/25 Handler S→C Kinh tế (Round-7 C + ASM)** | `0078b149` (`0x01`), `0078b6fc` (`0x02`), `case_009_0078D125` + `0078d125.asm.txt` (`0x08`), `0078d84d` (`0x0C`), `0078e367` (`0x0F`), `0078ec3f` (`0x14`), `case_020_007902FB` + `007902fb.asm.txt` (`0x17`), `00790ed5` (`0x18`), `0079157c` (`0x19`), `0079175f` (`0x1A`), `00791d80` (`0x1B`), `00791f76` (`0x1D`), `case_026_007921A6` + `007921a6.asm.txt` (`0x1E`), `00792bad` (`0x23`), `00794910` (`0x2C`), `007952ab` (`0x35`), `00795494` (`0x36`), `007954a5` (`0x37`), `00795555` (`0x38`), `00795579` (`0x39`), `007956b9` (`0x3A`), `007956f3` (`0x3B`), `0079575c` (`0x3C`), `007957dc` (`0x3D`), `00795a72` (`0x3E`) | **100%** |
| **Callee Helpers & UI Buttons** | `0072b084_FUN_0072b084.c`, `0072bcf8_FUN_0072bcf8.c`, `0076b458_FUN_0076b458.c` (`17 06`), `0076974c_FUN_0076974c.c` (`17 08`), `0076ae24_FUN_0076ae24.c` (`17 09`), `00610290_FUN_00610290.c` (`17 21`), `007311c8_FUN_007311c8.c` (`17 1F`), `0060e3cc_FUN_0060e3cc.c` (`17 22`), `0075aff4_FUN_0075aff4.c` (`19 02`), `0075b3ec_FUN_0075b3ec.c` (`19 03`), `0075b8e0_FUN_0075b8e0.c` (`19 0B`), `0075bf7c_FUN_0075bf7c.c` (`19 0A`), `00609218_FUN_00609218.c`/`.asm.txt`, `007707a0_FUN_007707a0.c` (`1E 05`), `asm_hole_5d6a9b_btnbuy.asm.txt`, `asm_hole_5dc98a_btnsale.asm.txt`, `asm_5d8392_5d88dc_bank_buttons.asm.txt` | **100%** |
| **Literal Hex Pools (VISCII 1.1)** | `lit_797ee8_798440.hex`, `lit_7973e0.hex`, `lit_79889c_pool.hex`, `lit_75af20.hex`, `lit_75b3a0_75b3d0.hex`, `lit_75bbec_75bf14.hex`, `lit_5d75d0_pool.hex`, `lit_611460_6114c0.hex`, `gap_54f2e1_54f384_literals.hex`, `gap_5d75bb_5d7658_money_literals.hex`, `gap_74592b_745acc_trade_strings.hex`, `cs_literals_0x78a818_0x78a8ed.hex` | **100%** |

### 6.13.2 Bảng thuật ngữ & Viết tắt (Glossary)

| Thuật ngữ / Ký hiệu | Giải nghĩa |
|:--|:--|
| `P[k]` (Wire Offset) | Byte thứ `k` (0-based) của phần payload gói tin sau 4 byte header `F4 44 <len:u16 LE>`. `P[0]` = main opcode, `P[1]` = sub-opcode. |
| `RP[k]` (Handler Offset) | Byte thứ `k` (0-based) của tham số chuỗi `RP` truyền vào hàm handler S→C. Vì dispatcher đã cắt `P[0]`, `RP[k]` chính là `P[k + 1]`. |
| `_LStrCopy(RP, idx, n)` | Hàm cắt chuỗi `AnsiString` của Delphi: `idx` là **1-based**, nên `_LStrCopy(RP, idx, n)` trích `n` byte bắt đầu từ `RP[idx - 1]` = **`P[idx]`**. |
| `FUN_0077eb1c` / `FUN_0077ee84` | Hai hàm đóng gói số `u16 LE` (2 byte) và `u32 LE` (4 byte) thành `AnsiString` để nối vào payload C→S. |
| `FUN_0077eb9c` / `FUN_0077ed68` | Hai hàm giải mã `AnsiString` 2 byte (`u16 LE`) và 4 byte (`u32 LE`) trong các handler S→C. |
| `TThing` | Object đại diện cho 1 ô chứa vật phẩm (size `0x38` byte): `+0x04` = `itemId:u16`, `+0x07` = `qty:u8`, `+0x08` = `b4:u8`, `+0x24` = `exp:u32`, `+0x28` = `doben:u8`, `+0x29..+0x2B` = `b5..b7`. |

### 6.13.3 Bảng đính chính các sai lệch trong `reference_docs/` và Server C# Bear cũ

| Chủ đề | Nội dung trong `reference_docs/` hoặc Server C# Bear cũ | Sự thật trong mã nguồn `aLogin.exe` (Kèm bằng chứng Level A) |
|:--|:--|:--|
| **1. Cấu trúc gói S→C `17 06` (Thêm item vào túi)** | `ref_01_shop_read_flow.md` (ghi chú Round-4) viết rằng `17 06` có `RP[1] = slot` và `qty` 1 byte (nhầm với `17 08`). | **`0076b458_FUN_0076b458.c:72-104`** chứng minh `17 06` **KHÔNG có byte `slot`**, mà `_LStrCopy(RP, 2, 2)` là `itemId:u16 LE` (`P[2..3]`) và `_LStrCopy(RP, 4, 2)` là **`qty:u16 LE` (2 byte, `P[4..5]`)**! |
| **2. Cấu trúc gói S→C `17 21` (Danh sách hàng trên sạp)** | `ref_01_shop_read_flow.md` trích server C# Bear nói `17 21` có 19-byte header và record bắt đầu bằng `itemId:u16`. | **`00610290_FUN_00610290.c:89-245`** chứng minh wire header dài **18 byte** (`P[0..17]` = `17 21` + 4 trường 4 byte), và mỗi record **16 byte** bắt đầu bằng **`slot:u8` (`+0`), `itemId:u16` (`+1..2`), `qty:u8` (`+3`), `b4:u8` (`+4`), `price:u32` (`+5..8`)**! |
| **3. Hành vi Banner của `1B 01 00` / `1B 02 00`** | Tài liệu cũ giả định khi `inner == 0` client hiện Banner `"Giao dịch thành công"`. | **`00791d80_FUN_00791d80.asm.txt:140-141`** (`CMP byte [EAX+1], 0; JZ 0x00791E63`) chứng minh khi `inner == 0`, client **nhảy qua lệnh gọi `Banner`**, không hiển thị thông báo! |
| **4. Chỉ số `slot` của `1B 02` (Bán hàng)** | Giả định `slot` trong `1B 02` là `0..24` (0-based) giống `1B 01`. | **`asm_hole_5dc98a_btnsale.asm.txt:243-253`** (`0x005DCC66`: `CMP byte [EAX+0x14C], 0; JZ ...; CMP EAX, 0x19`) chứng minh `form+0x14C` (chép sang `+0x160` để gửi trong `1B 02`) là **1-based (`1..25`)**, giá trị `0` bị từ chối ngay tại client! |
| **5. Chiều của `1D 01` và `1D 02` (Ngân hàng)** | Nhầm lẫn `1D 01` và `1D 02` giống nhau ở cả 2 chiều C→S và S→C. | **`asm_5d8392_5d88dc_bank_buttons.asm.txt`** chứng minh ở chiều **C→S**: `1D 01` là **Rút tiền**, `1D 02` là **Nộp tiền**; ngược lại ở chiều **S→C** (`00791f76.asm.txt`): `1D 01` là **Cộng tiền trang**, `1D 02` là **Trừ tiền trang**! |
| **6. File `asm_75aee0_75af6c_trade_helper.asm.txt`** | Bị đặt tên là `trade_helper.asm.txt` như thể là hàm xử lý trade. | Thực chất `0x0075AF20..0x0075AF5F` (`lit_75af20.hex`) là **chuỗi dữ liệu VISCII** `"Không thể giao dịch với Thiên Thần"` bị disassembler dịch nhầm thành lệnh ASM! |
