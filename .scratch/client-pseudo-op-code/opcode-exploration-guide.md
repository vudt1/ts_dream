# HANDOFF — Hướng Dẫn Khai Phá Các Main OP, Sub OP & Rest Payload Tiếp Theo (aLogin.exe)

---

## 1. Tóm Tắt Ngữ Cảnh Đã Hoàn Thành (Current State)

### Các kết luận cốt lõi không cần khảo sát lại:
1. **Khung mạng (Framing)**:
   * Header: `[Token 2B: 0xF4 0x44] [Length L: Word LE 2B] [Payload L Bytes]`.
   * Mã hóa: Toàn bộ frame cả 2 chiều XOR khóa tĩnh `0xAD` qua [`FUN_0050a248`](../../client_pseudo_c/0050a248_FUN_0050a248.c) / [`FUN_0050a2fc`](../../client_pseudo_c/005163e4_TForm1.CY_DelSedQueue.c#L55).
2. **Quyền nói trước**: Server luôn phát tín hiệu trước (**Server speaks first**). Client không gửi gì sau khi bắt tay TCP thành công.
3. **Dispatcher (Bộ điều phối S $	o$ C)**:
   * [`FUN_0078a89c`](../../client_pseudo_c/0078a89c_FUN_0078a89c.c) tra bảng byte `0x78A8EE` lấy index, rồi tra bảng dword `0x78A9B6` để nhảy tới hàm xử lý (`case_001` đến `case_065`).
4. **Main OP 0x00 & 0x01**:
   * `0x00` $	o$ Case 1 [`FUN_0078aabe`](../../client_pseudo_c/case_001_0078AABE_FUN_0078aabe.c): System Error Notice / Disconnect Dialog (SubOp `0x01..0x38`).
   * `0x01` $	o$ Case 2 [`FUN_0078b149`](../../client_pseudo_c/case_002_0078B149_FUN_0078b149.c): Player Despawn (`SubOp 0x01`), Scene Switch / Auto-Login (`SubOp 0x09`), Toasts (`0x05, 0x06, 0x07`), Countdown (`0x08`).

---

## 2. Bốn Nguồn Tài Nguyên Cốt Lõi Để Khai Phá Các Opcode Còn Lại

Khi agent mới bắt đầu làm việc với bất kỳ Opcode nào, **chỉ cần làm theo đúng 4 nguồn sau**:

### Nguồn 1: Bản Đồ Tra Cứu Trung Tâm (Main OP tới File C)
* **File bảng ánh xạ byte**: [`../../client_pseudo_c/jumptable_byte200_0x78A8EE.hex`](../../client_pseudo_c/jumptable_byte200_0x78A8EE.hex)
* **File danh mục mục lục**: [`../../client_pseudo_c/manifest.csv`](../../client_pseudo_c/manifest.csv)
* **File gộp toàn bộ 65 case**: [`../../client_pseudo_c/jumptable_0x78A9B6_cases.c`](../../client_pseudo_c/jumptable_0x78A9B6_cases.c)

#### Bảng Ánh Xạ Toàn Diện 65 Active Main OPs:
```
MainOp 0x00 (  0) -> Case  1 -> Target 0x0078AABE | FUN_0078aabe | case_001_0078AABE_FUN_0078aabe.c
MainOp 0x01 (  1) -> Case  2 -> Target 0x0078B149 | FUN_0078b149 | case_002_0078B149_FUN_0078b149.c
MainOp 0x02 (  2) -> Case  3 -> Target 0x0078B6FC | FUN_0078b6fc | case_003_0078B6FC_FUN_0078b6fc.c
MainOp 0x03 (  3) -> Case  4 -> Target 0x0078BC95 | FUN_0078bc95 | case_004_0078BC95_FUN_0078bc95.c
MainOp 0x04 (  4) -> Case  5 -> Target 0x0078C7DB | FUN_0078c7db | case_005_0078C7DB_FUN_0078c7db.c
MainOp 0x05 (  5) -> Case  6 -> Target 0x0078CA56 | FUN_0078ca56 | case_006_0078CA56_FUN_0078ca56.c
MainOp 0x06 (  6) -> Case  7 -> Target 0x0078CB99 | FUN_0078cb99 | case_007_0078CB99_FUN_0078cb99.c
MainOp 0x07 (  7) -> Case  8 -> Target 0x0078CE37 | FUN_0078ce37 | case_008_0078CE37_FUN_0078ce37.c
MainOp 0x08 (  8) -> Case  9 -> Target 0x0078D125 | FUN_0078d125 | case_009_0078D125_FUN_0078d125.c
MainOp 0x09 (  9) -> Case 10 -> Target 0x0078D4AF | FUN_0078d4af | case_010_0078D4AF_FUN_0078d4af.c
MainOp 0x0B ( 11) -> Case 11 -> Target 0x0078D5D1 | FUN_0078d5d1 | case_011_0078D5D1_FUN_0078d5d1.c
MainOp 0x0C ( 12) -> Case 12 -> Target 0x0078D84D | FUN_0078d84d | case_012_0078D84D_FUN_0078d84d.c
MainOp 0x0D ( 13) -> Case 13 -> Target 0x0078DBAE | FUN_0078dbae | case_013_0078DBAE_FUN_0078dbae.c
MainOp 0x0E ( 14) -> Case 14 -> Target 0x0078E01C | FUN_0078e01c | case_014_0078E01C_FUN_0078e01c.c
MainOp 0x0F ( 15) -> Case 15 -> Target 0x0078E367 | FUN_0078e367 | case_015_0078E367_FUN_0078e367.c
MainOp 0x10 ( 16) -> Case 16 -> Target 0x0078E593 | FUN_0078e593 | case_016_0078E593_FUN_0078e593.c
MainOp 0x13 ( 19) -> Case 17 -> Target 0x0078EADC | FUN_0078eadc | case_017_0078EADC_FUN_0078eadc.c
MainOp 0x14 ( 20) -> Case 18 -> Target 0x0078EC3F | FUN_0078ec3f | case_018_0078EC3F_FUN_0078ec3f.c
MainOp 0x16 ( 22) -> Case 19 -> Target 0x0078FEAF | FUN_0078feaf | case_019_0078FEAF_FUN_0078feaf.c
MainOp 0x17 ( 23) -> Case 20 -> Target 0x007902FB | FUN_007902fb | case_020_007902FB_FUN_007902fb.c
MainOp 0x18 ( 24) -> Case 21 -> Target 0x00790ED5 | FUN_00790ed5 | case_021_00790ED5_FUN_00790ed5.c
MainOp 0x19 ( 25) -> Case 22 -> Target 0x0079157C | FUN_0079157c | case_022_0079157C_FUN_0079157c.c
MainOp 0x1A ( 26) -> Case 23 -> Target 0x0079175F | FUN_0079175f | case_023_0079175F_FUN_0079175f.c
MainOp 0x1B ( 27) -> Case 24 -> Target 0x00791D80 | FUN_00791d80 | case_024_00791D80_FUN_00791d80.c
MainOp 0x1D ( 29) -> Case 25 -> Target 0x00791F76 | FUN_00791f76 | case_025_00791F76_FUN_00791f76.c
MainOp 0x1E ( 30) -> Case 26 -> Target 0x007921A6 | FUN_007921a6 | case_026_007921A6_FUN_007921a6.c
MainOp 0x1F ( 31) -> Case 27 -> Target 0x007922E6 | FUN_007922e6 | case_027_007922E6_FUN_007922e6.c
MainOp 0x20 ( 32) -> Case 28 -> Target 0x0079285A | FUN_0079285a | case_028_0079285A_FUN_0079285a.c
MainOp 0x21 ( 33) -> Case 29 -> Target 0x007928E4 | FUN_007928e4 | case_029_007928E4_FUN_007928e4.c
MainOp 0x22 ( 34) -> Case 30 -> Target 0x00792ACC | FUN_00792acc | case_030_00792ACC_FUN_00792acc.c
MainOp 0x23 ( 35) -> Case 31 -> Target 0x00792BAD | FUN_00792bad | case_031_00792BAD_FUN_00792bad.c
MainOp 0x24 ( 36) -> Case 32 -> Target 0x0079346B | FUN_0079346b | case_032_0079346B_FUN_0079346b.c
MainOp 0x25 ( 37) -> Case 33 -> Target 0x007937C6 | FUN_007937c6 | case_033_007937C6_FUN_007937c6.c
MainOp 0x26 ( 38) -> Case 34 -> Target 0x00793889 | FUN_00793889 | case_034_00793889_FUN_00793889.c
MainOp 0x27 ( 39) -> Case 35 -> Target 0x007938C3 | FUN_007938c3 | case_035_007938C3_FUN_007938c3.c
MainOp 0x28 ( 40) -> Case 36 -> Target 0x007943BF | FUN_007943bf | case_036_007943BF_FUN_007943bf.c
MainOp 0x29 ( 41) -> Case 37 -> Target 0x007943F9 | FUN_007943f9 | case_037_007943F9_FUN_007943f9.c
MainOp 0x2A ( 42) -> Case 38 -> Target 0x00794719 | FUN_00794719 | case_038_00794719_FUN_00794719.c
MainOp 0x2B ( 43) -> Case 39 -> Target 0x00794799 | FUN_00794799 | case_039_00794799_FUN_00794799.c
MainOp 0x2C ( 44) -> Case 40 -> Target 0x00794910 | FUN_00794910 | case_040_00794910_FUN_00794910.c
MainOp 0x2D ( 45) -> Case 41 -> Target 0x00794977 | FUN_00794977 | case_041_00794977_FUN_00794977.c
MainOp 0x2E ( 46) -> Case 42 -> Target 0x00795171 | FUN_00795171 | case_042_00795171_FUN_00795171.c
MainOp 0x32 ( 50) -> Case 43 -> Target 0x007951DA | FUN_007951da | case_043_007951DA_FUN_007951da.c
MainOp 0x33 ( 51) -> Case 44 -> Target 0x0079522C | FUN_0079522c | case_044_0079522C_FUN_0079522c.c
MainOp 0x34 ( 52) -> Case 45 -> Target 0x00795266 | FUN_00795266 | case_045_00795266_FUN_00795266.c
MainOp 0x35 ( 53) -> Case 46 -> Target 0x007952AB | FUN_007952ab | case_046_007952AB_FUN_007952ab.c
MainOp 0x36 ( 54) -> Case 47 -> Target 0x00795494 | FUN_00795494 | case_047_00795494_FUN_00795494.c
MainOp 0x37 ( 55) -> Case 48 -> Target 0x007954A5 | FUN_007954a5 | case_048_007954A5_FUN_007954a5.c
MainOp 0x38 ( 56) -> Case 49 -> Target 0x00795555 | FUN_00795555 | case_049_00795555_FUN_00795555.c
MainOp 0x39 ( 57) -> Case 50 -> Target 0x00795579 | FUN_00795579 | case_050_00795579_FUN_00795579.c
MainOp 0x3A ( 58) -> Case 51 -> Target 0x007956B9 | FUN_007956b9 | case_051_007956B9_FUN_007956b9.c
MainOp 0x3B ( 59) -> Case 52 -> Target 0x007956F3 | FUN_007956f3 | case_052_007956F3_FUN_007956f3.c
MainOp 0x3C ( 60) -> Case 53 -> Target 0x0079575C | FUN_0079575c | case_053_0079575C_FUN_0079575c.c
MainOp 0x3D ( 61) -> Case 54 -> Target 0x007957DC | FUN_007957dc | case_054_007957DC_FUN_007957dc.c
MainOp 0x3E ( 62) -> Case 55 -> Target 0x00795A72 | FUN_00795a72 | case_055_00795A72_FUN_00795a72.c
MainOp 0x3F ( 63) -> Case 56 -> Target 0x00795AC4 | FUN_00795ac4 | case_056_00795AC4_FUN_00795ac4.c
MainOp 0x40 ( 64) -> Case 57 -> Target 0x00795C7B | FUN_00795c7b | case_057_00795C7B_FUN_00795c7b.c
MainOp 0x41 ( 65) -> Case 58 -> Target 0x00795CE4 | FUN_00795ce4 | case_058_00795CE4_FUN_00795ce4.c
MainOp 0x42 ( 66) -> Case 59 -> Target 0x00795DB6 | FUN_00795db6 | case_059_00795DB6_FUN_00795db6.c
MainOp 0x43 ( 67) -> Case 60 -> Target 0x00795F48 | FUN_00795f48 | case_060_00795F48_FUN_00795f48.c
MainOp 0x45 ( 69) -> Case 61 -> Target 0x00795F82 | FUN_00795f82 | case_061_00795F82_FUN_00795f82.c
MainOp 0x46 ( 70) -> Case 62 -> Target 0x007960BD | FUN_007960bd | case_062_007960BD_FUN_007960bd.c
MainOp 0x47 ( 71) -> Case 63 -> Target 0x007961C1 | FUN_007961c1 | case_063_007961C1_FUN_007961c1.c
MainOp 0x48 ( 72) -> Case 64 -> Target 0x00796248 | FUN_00796248 | case_064_00796248_FUN_00796248.c
MainOp 0xC7 (199) -> Case 65 -> Target 0x007962FC | FUN_007962fc | case_065_007962FC_FUN_007962fc.c
```

---

### Nguồn 2: Thư Mục Mã Nguồn Chi Tiết Chiều Client nhận gói tin từ Server (S > C)
Thư mục: 📂 **[`../../client_pseudo_c/`](../../client_pseudo_c/)**

Mỗi file C trong thư mục này đại diện cho 1 Main OP hoàn chỉnh. Khi phân tích bất kỳ file nào:
1. **Đọc SubOp**: Nằm ở phần đầu hàm:
   ```c
   iVar6 = *(int *)(unaff_EBP + -0xc); // RestPayload (ECX)
   *(uint *)(unaff_EBP + -0x14) = (uint)*(byte *)(iVar6 + 0); // SubOp = ECX[0]
   ```
2. **Liệt kê Sub Codes**: Nhìn vào khối lệnh `switch(*(undefined4 *)(unaff_EBP + -0x14))` để lấy chính xác toàn bộ danh sách `case 1:`, `case 2:`...
3. **Đọc cấu trúc Payload Fields**:
   * Delphi 1-based string index: `_LStrCopy(ECX, 2, 4, &out)` nghĩa là cắt 4 bytes tính từ byte thứ 2 của `ECX` (tức `payload[2..5]`).
   * Xem biến `out` được truyền vào hàm chuyển đổi nào để xác định kiểu dữ liệu.

---

### Nguồn 3: Mã Nguồn Chiều Client gửi đến Server (C > S)
File: 📄 [`../../client_pseudo_c/0077f414_FUN_0077F414.c`](../../client_pseudo_c/0077f414_FUN_0077F414.c)

* Hàm `FUN_0077f414` (`TFConnect.SendCommand`) nhận `param_2` là Opcode chiều gửi đi.
* Xem lệnh `switch(param_2 & 0xff)` từ dòng **768** đến dòng **1180**:
  * Mỗi `case <Op>:` thể hiện chính xác cách Client tạo và ghép các trường (`_LStrCatN`) trước khi gọi `TForm1_CY_AddSedQueue`.
  * Điều này cho phép đối chiếu 2 chiều đối xứng giữa phản hồi từ Server và hành động của Client.

---

### Nguồn 4: Thư Viện Các Hàm Helper Giải Mã Nhị Phân (Codec Primitives)
Các hàm helper chuẩn được gọi xuyên suốt trong toàn bộ các case:
* [`FUN_0077eb9c`](../../client_pseudo_c/0077eb9c_FUN_0077eb9c.c): Cắt 2 bytes chuỗi $	o$ `Word Little-Endian` (`b0 + b1 * 256`).
* [`FUN_0077ef7c`](../../client_pseudo_c/0077ef7c_FUN_0077ef7c.c): Cắt 4 bytes chuỗi $	o$ `DWORD Little-Endian` (`b0 + b1*256 + b2*65536 + b3*16777216`).
* [`FUN_0077eb1c`](../../client_pseudo_c/0077eb1c_FUN_0077eb1c.c): Chuyển `Word` $	o$ chuỗi nhị phân 2 bytes LE.
* [`FUN_0077ee84`](../../client_pseudo_c/0077ee84_FUN_0077ee84.c): Chuyển `DWORD` $	o$ chuỗi nhị phân 4 bytes LE.
* [`FUN_0077f098`](../../client_pseudo_c/0077f098_FUN_0077f098.c): Tách chuỗi cấu trúc cố định `[8-byte name][2 x dword]`.
