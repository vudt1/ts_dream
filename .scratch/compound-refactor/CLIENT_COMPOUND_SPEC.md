# Phân Tích Kỹ Thuật Chi Tiết Module "Mix Đồ" (合成 / Compound) — Client `aLogin.exe` (TS Online)

- **Tài liệu phục vụ**: Cài đặt lại chính xác (clean-room re-implementation) logic nạp dữ liệu, giải mã, kiểm tra hợp lệ, tra cứu công thức và tính tỷ lệ hợp thành vật phẩm (Mix / Compound) phía Server (Rust).  
- **Đối tượng phân tích**: Binary `client_delphi/aLogin.exe` (PE32 GUI, Intel 80386, Borland Delphi, `ImageBase = 0x00400000`, `CODE` section tại RVA `0x1000` / file offset `0x400`) và bộ 157 file trích xuất trong thư mục `client_pseudo_c/`.  
- **Quy ước mức độ tin cậy**:  
  - `[ĐÃ XÁC MINH]`: Khẳng định có chứng cứ trực tiếp từ mã máy/disassembly (`.asm.txt`), dữ liệu nhị phân (`.Dat`, `.hex.txt`), hoặc pseudo-C (`.c`) đã đối chiếu với assembly.  
  - `[SUY LUẬN]`: Kết luận hợp lý rút ra từ việc ghép nối nhiều bằng chứng trong binary nhưng không có nhãn/symbol trực tiếp.  
  - `[CHƯA RÕ — cần kiểm chứng]`: Thông tin nằm ngoài phạm vi 157 file trích xuất hoặc cần kiểm chứng thêm trên runtime/file khác.

---

## 1\. TL;DR (Tóm Tắt Nhanh)

1. **`Compound.Dat` không lưu `item_id`** `[ĐÃ XÁC MINH]`: File `\Data\Compound.Dat` (`11.152` byte \= `1.394` slot × `8` byte, gồm 1 slot header \+ `1.393` công thức) lưu **`result_kind`** (`u16` ∈ `1..52`, ánh xạ vào `item + 0x19`) và 3 mã chất liệu **`matA, matB, matC`** (`u8` ∈ `0..37`, ánh xạ vào `item + 0x32` tức `ratA`), kèm 3 hệ số tỷ lệ nguyên liệu **`degA, degB, degC`** (`u8` ∈ `{0, 1}`). (`0079a890_FUN_0079a890.asm.txt:109-123, 368-378, 495-538`).  
2. **Giải mã XOR \+ trừ offset 3** `[ĐÃ XÁC MINH]`: Hàm `FUN_005F2DD0` (`0x005F2DD0`) giải mã tại chỗ các slot `1..1393`: trường `u16` theo công thức `(raw ^ 0xFBBC) - 3`, và 6 trường `u8` theo công thức `((raw ^ 0xD3) & 0xFF) - 3` (`005f2dd0_FUN_005f2dd0.asm.txt:25-153`).  
3. **Không có lỗi off-by-one khi đọc file** `[ĐÃ XÁC MINH]`: Hàm `FUN_005F2528` gọi `FUN_004069DC` (`FileSize`) hai lần — lần 1 trừ `1` lưu `count = 1393` vào `inst + 0xC74E`, lần 2 giữ nguyên `1394` để `ReadFile` đọc trọn `11.152` byte vào `inst + 0x0004`. Slot `0` là header (`u16 == 2`), vòng lặp giải mã và dựng bảng chạy từ `i = 1..1393` (`005f2528_FUN_005f2528.asm.txt:55-120`).  
4. **Bảng tra cứu thứ cấp (stride 9 byte) và khóa `u32`** `[ĐÃ XÁC MINH]`: Với mỗi công thức, 3 mã chất liệu `(matA, matB, matC)` và `(degA, degB, degC)` được sắp xếp tăng dần theo `mat` (`0` đứng đầu nếu chỉ có 2 nguyên liệu), rồi ghép chuỗi thập phân `StrToInt(IntToStr(m0) + IntToStr(m1) + IntToStr(m2))` tạo khóa `u32` lưu tại `inst + 0x5DCE + i*9`. Toàn bộ `1.393` mục sau đó được sắp xếp tăng dần theo khóa `u32` (`005f2528_FUN_005f2528.asm.txt:155-351`).  
5. **Ý nghĩa thật của 6 ô nhập trên Form `0x0079EE08`** `[ĐÃ XÁC MINH]`: 3 ô trên (`Self + 0x134..0x13C`) nhập **`item_id` (`i32`)**, 3 ô dưới (`Self + 0x140..0x148`) nhập **số lượng nguyên liệu `qty` (`u8`)** chứ không đơn thuần là cờ bật/tắt. Hàm `FUN_0079A890` dùng `qty` để tìm số lượng nhỏ nhất khác 0 (`min_qty` → số lượng thành phẩm `Num:`) và tính bậc nguyên liệu đầu vào `input_deg[k] = min(5, qty[k] div min_qty)` (`0079a890_FUN_0079a890.asm.txt:234-275`).  
6. **Luật kiểm tra đầu vào (`FUN_0079A5E4`)** `[ĐÃ XÁC MINH]`: Phải có ít nhất 2 ô hợp lệ (`item_id > 0` và `qty > 0`); mọi `item_id` phải thuộc `[10001, 50000]`, tồn tại trong `TFItem`, có `qty <= 50`, và `item.kind` (`item + 0x19`) không thuộc `{20, 21, 22}` (`0079a5e4_FUN_0079a5e4.asm.txt:20-169`).  
7. **Công thức bậc phẩm chất (`target_rank` / `ratB`)** `[ĐÃ XÁC MINH]`: Trung bình cộng trường `ratB` (`item + 0x33`) của các nguyên liệu đầu vào: \$R\_{\\text{avg}} \= \\max(1, \\text{ROUND}(\\sum \\text{ratB}*k / N))\$, sau đó quay số ngẫu nhiên \$R*{\\text{target}} \= \\min(10, \\text{Random}(R\_{\\text{avg}} \+ 1\) \+ 1)\$. Xác suất tăng 1 bậc (\$R\_{\\text{avg}} \+ 1\$, tối đa 10\) là \$\\frac{1}{R\_{\\text{avg}} \+ 1}\$ (`0079a890_FUN_0079a890.asm.txt:135-156`).  
8. **Cơ chế Fallback khi không khớp công thức** `[ĐÃ XÁC MINH]`: Nếu không có công thức nào trong `Compound.Dat` khớp khóa `u32` và `degree`, `FUN_0079A890` **không trả về mã lỗi `1`** mà tự động fallback: chọn ngẫu nhiên 1 nguyên liệu đầu vào, lấy `result_kind = input_item.kind` (`item + 0x19`), và ép bậc phẩm chất về `target_rank = 1`. Nhánh xử lý mã trả về `1` (`: 合成資料庫無此合成法`) trong handler `0x0079F3C8` thực chất là dead code (`0079a890_FUN_0079a890.asm.txt:382-424`).  
9. **Tra ngược từ `(result_kind, target_rank)` ra `item_id` kết quả** `[ĐÃ XÁC MINH]`: Sau khi chọn được `result_kind` (tối đa 10 ứng viên công thức), hàm quét toàn bộ bảng `TFItem` tìm các vật phẩm có `(item + 0x19) == result_kind` và `(item + 0x33) == target_rank` (tối đa 10 ứng viên item), rồi chọn ngẫu nhiên 1 vật phẩm theo bước chia `ROUND(100.0 / K)` (`0079a890_FUN_0079a890.asm.txt:463-576`).  
10. **Bản chất của Form `0x0079EE08` và Giao thức** `[ĐÃ XÁC MINH]`: Form `0x0079EE08` và handler `0x0079F3C8` là **trình mô phỏng / kiểm thử công thức hợp thành nội bộ (offline dev simulator)** được nhúng trong client, thực thi 100% cục bộ và không gửi bất kỳ gói tin mạng nào. Trong gameplay thực tế, client gửi yêu cầu mix qua Action Code `AC 29` (`Compound`) và Server phải thực thi toàn bộ thuật toán của `FUN_0079A5E4` \+ `FUN_0079A890`.

---

## 2\. Sơ Đồ Kiến Trúc Tổng Thể

\+-----------------------------------------------------------------------------------+

|                        KHỞI TẠO DỮ LIỆU KHI MỞ CLIENT                             |

|  TForm1.FormCreate                                                                |

|    ├──► TFItem.Create (0x00774634)                                                |

|    │      └──► FUN\_007746D4 ("\\Data\\Item.dat", stride 370B \= 0x172)               |

|    │             ├──► FUN\_00774BA8 (Giải mã XOR Item.dat: offset=9, keys=9A/EFC3) |

|    │             └──► Bảng ánh xạ \[0x009D162C\]: item\_id (u16) ──► item\_idx (u16)  |

|    │                  & \[0x007DA754\] \-\> TFItem instance                           |

|    └──► TFCompound.Create (0x005F24B0) ──► lưu con trỏ tại \[0x007DA7E8\]           |

|           └──► FUN\_005F2528 ("\\Data\\Compound.Dat", stride 8B, 1394 slot)          |

|                  ├──► Kiểm tra Header: slot\[0\].u16 \== 2                           |

|                  ├──► FUN\_005F2DD0: Giải mã XOR slot 1..1393 (offset=3, FBBC/D3)  |

|                  ├──► Sắp xếp (matA,matB,matC) & (degA,degB,degC) tăng theo mat   |

|                  ├──► Tạo khóa u32 \= StrToInt(IntToStr(m0)+IntToStr(m1)+IntToStr(m2))

|                  └──► Sắp xếp bảng thứ cấp (inst+0x5DCC, stride 9B) tăng theo u32 |

\+-----------------------------------------------------------------------------------+

                                         │

                                         ▼

\+-----------------------------------------------------------------------------------+

|                     LUỒNG UI & XỬ LÝ NÚT "合成" (MIX HANDLER)                     |

|  FUN\_0051189C (Form Factory)                                                      |

|    └──► FUN\_0079EE08 (Constructor Form Mix Simulator)                             |

|           ├──► 3 ô Item ID : Self+0x134 ("10001"), \+0x138 ("17001"), \+0x13C       |

|           ├──► 3 ô Số lượng: Self+0x140 ("1"),     \+0x144 ("1"),     \+0x148 ("1") |

|           ├──► 2 hộp danh sách TSe\_SelectText: Self+0x14C (KQ), Self+0x150 (Dump) |

|           └──► Nút TSe\_FixedButton "合成" (OnClick \= 0x0079F3C8)                  |

|                                         │                                         |

|                                         ▼                                         |

|  Handler 0x0079F3C8 (Khi bấm nút "合成")                                          |

|    ├──► 1\. Đọc 6 ô TSe\_Editor (+0x120 text) qua StrToInt \-\> local entries\[0..2\]   |

|    ├──► 2\. Gọi FUN\_0079A5E4(\&entries) ──► Validate đầu vào                        |

|    │         ├─ Lỗi 1: \< 2 nguyên liệu          ──► ": 合成至少要二樣物品"        |

|    │         ├─ Lỗi 2: ID ngoài 10001..50000,   ──► ": 參雜有不能合成的物品"      |

|    │         │         không có trong DB, hoặc kind ∈ {20,21,22}                  |

|    │         ├─ Lỗi 3: Số lượng qty \> 50        ──► ": 單項物品數量不可超過50"    |

|    │         └─ Hợp lệ (0): Tiếp tục                                              |

|    ├──► 3\. Dump vào hộp phải (Self+0x150):                                        |

|    │         ├─ Toàn bộ 1393 dòng Compound.Dat ("Key  Id  D1  D2  D3")            |

|    │         └─ Thông số 3 nguyên liệu ("Mate  kind  ratA  ratB") từ TFItem       |

|    └──► 4\. Gọi FUN\_0079A890(\&entries, \&out\_id, \&out\_qty)                          |

|              ├─ Tính target\_rank từ trung bình ratB (+0x33) \+ Random              |

|              ├─ Tạo khóa u32 từ 3 trường ratA (+0x32) đã sắp xếp                  |

|              ├─ Tính min\_qty (= out\_qty) & input\_deg\[k\] \= min(5, qty\[k\]/min\_qty)  |

|              ├─ Quét bảng inst+0x5DCC tìm tối đa 10 recipe khớp khóa & degree     |

|              ├─ Nếu 0 recipe khớp: Fallback lấy kind (+0x19) của 1 nguyên liệu,   |

|              │                     ép target\_rank \= 1                             |

|              ├─ Chọn ngẫu nhiên 1 result\_kind; nếu kind ∈ 1..14 thì out\_qty \= 1   |

|              ├─ Quét TFItem tìm tối đa 10 item có kind==result\_kind & ratB==rank  |

|              └─ Chọn ngẫu nhiên 1 out\_id (+0x1A) ──► In vào hộp trái (Self+0x14C) |

\+-----------------------------------------------------------------------------------+

---

## 3\. Bảng Ánh Xạ Hàm (Function Map)

Mọi địa chỉ và quan hệ gọi hàm dưới đây đều đã được kiểm chứng từ các file `.asm.txt` và `.c` trong `client_pseudo_c/` (`MANIFEST.md`, `COVERAGE.md`).

| Địa chỉ VA | Tên hàm / Symbol | Nhóm | Vai trò chính | Caller tiêu biểu | Callee chính | Độ tin cậy |
| :---- | :---- | :---: | :---- | :---- | :---- | :---: |
| `0x005F24B0` | `TFCompound.Create` | G1 | Constructor lớp `TFCompound` (size `0xC750`), gọi nạp `Compound.Dat` | `TForm1.FormCreate` (`0x00511092`) | `0x0040304C` (`_ClassCreate`), `0x005F2528` | `[ĐÃ XÁC MINH]` |
| `0x005F2528` | `FUN_005F2528` | G1 | Mở `\Data\Compound.Dat`, kiểm tra header `== 2`, gọi giải mã, sắp xếp nguyên liệu, tạo khóa `u32` và sắp xếp bảng thứ cấp | `0x005F24B0` | `0x0040673A`, `0x00406CF9`, `0x00406BF8`, `0x004069DC`, `0x004067DC`, `0x004068A4`, `0x005F2DD0`, `0x00409980`, `0x004099E4` | `[ĐÃ XÁC MINH]` |
| `0x005F2DD0` | `FUN_005F2DD0` | G1 | Giải mã XOR \+ trừ `3` tại chỗ cho `count` record trong `inst + 0x0004` | `0x005F2528` (`0x005F25E9`) | `0x00402FA8` (`_BoundErr`), `0x00402FB0` (`_IntOver`) | `[ĐÃ XÁC MINH]` |
| `0x005F2420` | `FUN_005F2420` | G1 | Hàm kiểm tra tồn tại vật phẩm kết quả có `kind == result_id` và `ratB == 1` (tiện ích kiểm tra DB) | Không gọi trực tiếp từ `0x0079F3C8` | `0x007746AC`, `0x00402FA8` | `[ĐÃ XÁC MINH]` |
| `0x005F3048` | `FUN_005F3048` | G1 | Giải phóng bộ nhớ của instance `TFCompound` (`TObject.Free`) | `0x005F30EC` | `0x0040300C` (`TObject.Free`) | `[ĐÃ XÁC MINH]` |
| `0x005F3068` | `FUN_005F3068` | G1 | Khởi tạo và nạp bảng tỉ lệ phụ `Compound2.Dat` (15x15 byte, XOR `0xFD`, trừ `5`) | `0x005F30D8` | `0x0040673A..0x004068A4` | `[ĐÃ XÁC MINH]` |
| `0x0040673A` | `FUN_0040673A` | G2 | `@AssignFile` — Gán đường dẫn chuỗi vào cấu trúc `TFileRec` | `0x005F2528`, `0x007746D4` | `0x00406704`, `0x004040C8` | `[ĐÃ XÁC MINH]` |
| `0x00406CF9` | `FUN_00406CF9` | G2 | `@ResetFile` — Đặt `FileMode` và `RecSize` (`[EAX+8] = EDX`), gọi mở file | `0x005F2528`, `0x007746D4` | `0x00406BD0` (`CreateFileA`) | `[ĐÃ XÁC MINH]` |
| `0x00406BF8` | `FUN_00406BF8` | G2 | Lấy mã lỗi I/O gần nhất (`InOutRes`) | `0x005F2528` | Không có | `[ĐÃ XÁC MINH]` |
| `0x004069DC` | `FUN_004069DC` | G2 | `@FileSize` — Trả về `GetFileSize(handle) div RecSize` | `0x005F2528`, `0x007746D4` | `SetFilePointer` (`0x004012EC`) | `[ĐÃ XÁC MINH]` |
| `0x004067DC` | `FUN_004067DC` | G2 | `@BlockRead` — Đọc `count * RecSize` byte từ file vào buffer qua `ReadFile` | `0x005F2528`, `0x007746D4` | `ReadFile` (`0x0040129C`) | `[ĐÃ XÁC MINH]` |
| `0x004068A4` | `FUN_004068A4` | G2 | `@CloseFile` — Đóng file handle trong `TFileRec` | `0x005F2528`, `0x007746D4` | `0x004066C0` (`CloseHandle`) | `[ĐÃ XÁC MINH]` |
| `0x0079EE08` | `FUN_0079EE08` | G3 | Constructor dựng Form Mix Simulator (6 `TSe_Editor`, 2 `TSe_SelectText`, 1 `TSe_ScrollButton`, 1 `TSe_FixedButton`) | `0x0051189C` (`0x00511CA5`) | `0x007B0ECC`, `0x007B34FC`, `0x007B2E80`, `0x007BDC60`, `0x007B7928` | `[ĐÃ XÁC MINH]` |
| `0x0079F3C8` | `mix_handler` | G3 | Event handler `OnClick` của nút `合成`: đọc 6 ô nhập, gọi validate, dump bảng công thức & nguyên liệu, gọi tra cứu và in kết quả | Gắn vào `[btn + 0x50]` tại `0x0079F305` | `0x004099E4`, `0x0079A5E4`, `0x0079A890`, `0x007746AC`, `0x007BEA20`, `0x007BD3BC` | `[ĐÃ XÁC MINH]` |
| `0x0051189C` | `FUN_0051189C` | G3 | Form Factory khởi tạo các cửa sổ UI con của client (trong đó có `FUN_0079EE08`) | Startup / Form Manager | `0x0079EE08` (cùng hàng chục form khác) | `[ĐÃ XÁC MINH]` |
| `0x0079A5AC` | `FUN_0079A5AC` | G4 | Hàm khởi tạo/xóa cấu trúc dữ liệu của module `0x0079A5xx` | Khởi tạo module | `0x00402920` (`FillChar`) | `[ĐÃ XÁC MINH]` |
| `0x0079A5E4` | `FUN_0079A5E4` | G4 | Kiểm tra tính hợp lệ của 3 cặp `(item_id, qty)` đầu vào; trả về `0` (OK) hoặc `1/2/3` | `0x0079F3C8` (`0x0079F4D4`) | `0x007746AC`, `0x00402FA8`, `0x00402FB0` | `[ĐÃ XÁC MINH]` |
| `0x0079A890` | `FUN_0079A890` | G4 | Trái tim của module: tính bậc phẩm chất (`target_rank`), tra bảng `TFCompound`, xử lý fallback, quét `TFItem` chọn `out_item_id` và `out_qty` | `0x0079F3C8` (`0x0079F74A`) | `0x007746AC`, `0x00402B1C` (`ROUND`), `0x00402CF0` (`Random`), `0x00409980`, `0x004099E4` | `[ĐÃ XÁC MINH]` |
| `0x007746AC` | `FUN_007746AC` | G5 | Tra cứu chỉ số 1-based (`1..count`) trong bảng `TFItem` từ `item_id` qua mảng tĩnh `[0x009D162C + item_id * 2]` | `0x0079A5E4`, `0x0079A890`, `0x0079F3C8` | `0x00402FA8` (`_BoundErr`) | `[ĐÃ XÁC MINH]` |
| `0x00774634` | `TFItem.Create` | G5 | Constructor lớp `TFItem`, gọi `FUN_007746D4` nạp `\Data\Item.dat` | `TForm1.FormCreate` | `0x007746D4` | `[ĐÃ XÁC MINH]` |
| `0x007746D4` | `FUN_007746D4` | G5 | Đọc `\Data\Item.dat` (record size `370` byte \= `0x172`), gọi `FUN_00774BA8` giải mã và điền bảng `[0x009D162C]` | `0x00774634` | `0x0040673A..0x004068A4`, `0x00774BA8` | `[ĐÃ XÁC MINH]` |
| `0x00774BA8` | `FUN_00774BA8` | G5 | Giải mã XOR cho từng record của `Item.dat` (`offset = 9`, `u8 ^ 0x9A`, `u16 ^ 0xEFC3`, `u32 ^ 0x0B80F4B4`) | `0x007746D4` | `0x00402FA8`, `0x00402FB0` | `[ĐÃ XÁC MINH]` |
| `0x005FAA80` | `FUN_005FAA80` | G6 | Khởi tạo bảng tra cứu kỹ năng / thuật (chứa các hằng chuỗi `合成`, `挖礦` ở vùng dữ liệu liền kề `0x005FC4F0..0x005FC818`) | Khởi tạo hệ thống Skill | Các hàm xử lý chuỗi RTL | `[SUY LUẬN]` |
| `0x00402864` | `FUN_00402864` | G7 | `@_IOTest` — Kiểm tra `InOutRes != 0` sau lệnh I/O; bảo toàn `EAX/EDX/ECX` bằng `PUSH/POP` (**không có giá trị trả về**) | `0x005F2528`, `0x007746D4` | `0x00402838` (`_RunError`) | `[ĐÃ XÁC MINH]` |
| `0x00402B1C` | `ROUND` | G7 | `@ROUND` — Làm tròn đỉnh stack FPU `ST(0)` thành số nguyên 64-bit (`FISTP qword ptr [ESP]`, chuẩn IEEE-754 round-to-even) | `0x0079A890` | Không có | `[ĐÃ XÁC MINH]` |
| `0x00402CF0` | `FUN_00402CF0` | G7 | `@RandInt` (`Random(range)`) — Bộ sinh số giả ngẫu nhiên LCG của Delphi (`seed = seed * 0x08088405 + 1`), trả về `[0, EAX - 1]` | `0x0079A890` | Không có | `[ĐÃ XÁC MINH]` |
| `0x007BEA20` | `FUN_007BEA20` | G10 | `TSe_SelectText.AddLine` — Thêm một dòng văn bản vào hộp danh sách (tối đa 500 dòng tại `Self + 0xAC .. +0x884`) | `0x0079F3C8` | `0x00403E98` (`@LStrAsg`) | `[ĐÃ XÁC MINH]` |
| `0x007BD3BC` | `FUN_007BD3BC` | G10 | Cập nhật thanh cuộn và vẽ lại (`Refresh`) danh sách `TSe_SelectText` | `0x0079F3C8` | `0x007BD180` | `[ĐÃ XÁC MINH]` |

---

## 4\. Định Dạng `Compound.Dat` & Thuật Toán Giải Mã XOR

### 4.1. Cấu trúc vật lý của `Compound.Dat` `[ĐÃ XÁC MINH]`

Theo `Compound.Dat`, `compound_dat_head.hex.txt` và `005f2528_FUN_005f2528.asm.txt`:

- **Tổng kích thước file**: `11.152` byte.  
- **Kích thước mỗi bản ghi (Record Size)**: `8` byte (`MOV EDX, 0x8; CALL 0x00406CF9` tại `005f2528_FUN_005f2528.asm.txt:57-58`).  
- **Tổng số slot trong file**: `11.152 / 8 = 1.394` slot (đánh số `0..1393`):  
  - **Slot `0` (File offset `0x0000..0x0007`)**: Header 8 byte. Giá trị thô là `02 00 00 00 00 00 00 00`. Hai byte đầu ở dạng Little-Endian (`u16`) bằng `2` (version/magic number). Hàm `FUN_005F2528` kiểm tra `CMP word ptr [EAX + 0x4], 0x2` (`005f2528_FUN_005f2528.asm.txt:90`) **trước khi** gọi giải mã; nếu khác `2` sẽ ném ngoại lệ qua `0x0040CAA0` / `0x0040380C`. Sau khi giải mã xong các slot `1..1393`, slot `0` được xóa về `0` (`MOV word ptr [EAX + 0x4], 0x0` tại `005f2528_FUN_005f2528.asm.txt:100`).  
  - **Slot `1..1393` (File offset `0x0008..0x2B8F`)**: `1.393` bản ghi công thức hợp thành. Số lượng `1393` (`0x0571`) được lưu tại trường `u16` ở `inst + 0xC74E` (`005f2528_FUN_005f2528.asm.txt:69`).

### 4.2. Bảng trường của một Record `Compound.Dat` (8 byte) `[ĐÃ XÁC MINH]`

> **Lưu ý bẫy Decompiler Ghidra**: Trong `005f2528_FUN_005f2528.c` và `005f2dd0_FUN_005f2dd0.c`, biến `param_1` là con trỏ `Self` (`TFCompound`). Do `4` byte đầu của `TFCompound` (`+0x00..+0x03`) là con trỏ VMT, mảng buffer bắt đầu tại `Self + 0x04`. Vì vậy trong assembly (`005f2dd0_FUN_005f2dd0.asm.txt:25-153`), trường đầu của record thứ `i` (`EAX = 1..1393`) nằm ở `[EDX + EAX*8 + 0x4]`, tương ứng đúng với offset `+0` tính từ đầu record\!

| Offset trong Record | Biểu thức địa chỉ trong ASM (`EDX = Self`, `EAX = i`) | Kiểu dữ liệu | Tên trường | Phạm vi giá trị sau giải mã | Ý nghĩa thực sự trong logic game (`FUN_0079A890`) |
| :---: | :---- | :---: | :---- | :---: | :---- |
| `+0..+1` | `word ptr [EDX + EAX*8 + 0x4]` | `u16` (LE) | `result_kind` | `1 .. 52` (36 giá trị) | **Mã loại vật phẩm kết quả (`item.kind` tại `item + 0x19`)**, KHÔNG phải `item_id` (`0079a890_FUN_0079a890.asm.txt:371, 495`) |
| `+2` | `byte ptr [EDX + EAX*8 + 0x6]` | `u8` | `matA` | `1 .. 37` | Mã chất liệu nguyên liệu 1 (`item.ratA` tại `item + 0x32`) |
| `+3` | `byte ptr [EDX + EAX*8 + 0x7]` | `u8` | `matB` | `1 .. 37` | Mã chất liệu nguyên liệu 2 (`item.ratA` tại `item + 0x32`) |
| `+4` | `byte ptr [EDX + EAX*8 + 0x8]` | `u8` | `matC` | `0 .. 37` | Mã chất liệu nguyên liệu 3 (`0` \= công thức chỉ dùng 2 nguyên liệu) |
| `+5` | `byte ptr [EDX + EAX*8 + 0x9]` | `u8` | `degA` | `1` | Hệ số/bậc tỷ lệ yêu cầu của nguyên liệu A (luôn `= 1`) |
| `+6` | `byte ptr [EDX + EAX*8 + 0xA]` | `u8` | `degB` | `1` | Hệ số/bậc tỷ lệ yêu cầu của nguyên liệu B (luôn `= 1`) |
| `+7` | `byte ptr [EDX + EAX*8 + 0xB]` | `u8` | `degC` | `0` hoặc `1` | Hệ số/bậc tỷ lệ yêu cầu của nguyên liệu C (`0` khi và chỉ khi `matC == 0`) |

### 4.3. Thuật toán giải mã chính xác của `FUN_005F2DD0` `[ĐÃ XÁC MINH]`

Đọc trực tiếp từ `005f2dd0_FUN_005f2dd0.asm.txt:12-160`:

// Tái hiện chính xác 100% logic của FUN\_005F2DD0 @ 0x005F2DD0

pub fn decode\_compound\_record(raw: &\[u8; 8\]) \-\> CompoundRawRecord {

    let raw\_id \= u16::from\_le\_bytes(\[raw\[0\], raw\[1\]\]);

    // ASM 0x005F2DF7..0x005F2E14:

    // MOVZX EAX, word ptr \[EDX \+ EAX\*8 \+ 0x4\]

    // XOR EAX, 0xFBBC

    // SUB EAX, 0x3

    let result\_kind \= (raw\_id ^ 0xFBBC).checked\_sub(3).expect("BoundErr u16");

    // ASM 0x005F2E1A..0x005F2F36 (lặp tương tự cho \+0x6, \+0x7, \+0x8, \+0x9, \+0xA, \+0xB):

    // XOR AL, 0xD3

    // AND EAX, 0xFF

    // SUB EAX, 0x3

    let decode\_u8 \= |b: u8| \-\> u8 {

        (b ^ 0xD3).checked\_sub(3).expect("BoundErr u8")

    };

    CompoundRawRecord {

        result\_kind,

        mat: \[decode\_u8(raw\[2\]), decode\_u8(raw\[3\]), decode\_u8(raw\[4\])\],

        deg: \[decode\_u8(raw\[5\]), decode\_u8(raw\[6\]), decode\_u8(raw\[7\])\],

    }

}

- **Vì sao `u16` dùng khóa `0xFBBC` (`64444`), `u8` dùng khóa `0xD3` (`211`), và cùng trừ `3`?** `[ĐÃ XÁC MINH]` Trong định dạng `.Dat` của TS Online, công cụ đóng gói dữ liệu (packer) của nhà phát triển áp dụng cơ chế làm rối (obfuscation) thống nhất cho mọi kiểu số nguyên theo độ rộng kiểu: \$\$\\text{encoded\_u8} \= ((\\text{plain\_u8} \+ \\text{offset}) \\bmod 256\) \\oplus K\_8\$\$ \$\$\\text{encoded\_u16} \= ((\\text{plain\_u16} \+ \\text{offset}) \\bmod 65536\) \\oplus K\_{16}\$\$ \$\$\\text{encoded\_u32} \= ((\\text{plain\_u32} \+ \\text{offset}) \\bmod 2^{32}) \\oplus K\_{32}\$\$ Mỗi file `.Dat` được cấu hình một bộ hằng số `(offset, K_8, K_16, K_32)` riêng biệt:  
  - `Compound.Dat` (`FUN_005F2DD0`): `offset = 3`, `K_8 = 0xD3` (`211`), `K_16 = 0xFBBC` (`64444`), `K_32 = 0` — hoàn toàn trùng khớp với lời gọi `DatReader::with_keys(bytes, 3, 211, 64444, 0)` trong `src/data/reader.rs` phía Server Rust.  
  - Đối chiếu các file `.Dat` khác trong cùng client:  
    - `Item.Dat` (`00774ba8_FUN_00774ba8.asm.txt`): `offset = 9`, `K_8 = 0x9A` (`154`), `K_16 = 0xEFC3` (`61379`), `K_32 = 0x0B80F4B4`.  
    - `WeapBody.Dat` / `Suit.Dat` / `Compound2.Dat` (`004ca43c_FUN_004ca43c.c`, `004dbaa0_FUN_004dbaa0.c`, `005f3068_FUN_005f3068.c`): `offset = 5`, `K_8 = 0xFD` (`253`), `K_16 = 0xECEA` (`60650`).

### 4.4. Kiểm chứng trên 3 Record thật từ `Compound.Dat` `[ĐÃ XÁC MINH]`

Đối chiếu giữa `compound_dat_head.hex.txt` và `compound_dat_decoded_sample.txt`:

| Thứ tự Record | File Offset | Hex thô (8 byte) | Giải mã từng trường | Sau khi sắp xếp `mat` tăng dần & tạo khóa `u32` |
| :---- | :---: | :---- | :---- | :---- |
| **Record \#1** (Slot `1`) | `0x0008` | `B8 FB D7 D7 D0 D7 D7 D0` | • `result_kind`: `0xFBB8 ^ 0xFBBC = 0x0004` \$\\rightarrow 4 \- 3 \= \\mathbf{1}\$ • `matA`: `0xD7 ^ 0xD3 = 4` \$\\rightarrow 4 \- 3 \= \\mathbf{1}\$ • `matB`: `0xD7 ^ 0xD3 = 4` \$\\rightarrow 4 \- 3 \= \\mathbf{1}\$ • `matC`: `0xD0 ^ 0xD3 = 3` \$\\rightarrow 3 \- 3 \= \\mathbf{0}\$ • `degA, degB, degC`: `0xD7, 0xD7, 0xD0` \$\\rightarrow \\mathbf{(1, 1, 0)}\$ | • `mat_sorted = (0, 1, 1)` • `deg_sorted = (0, 1, 1)` • Chuỗi: `"0" + "1" + "1" = "011"` • Khóa `u32`: `StrToInt("011") =` **`11`** |
| **Record \#2** (Slot `2`) | `0x0010` | `B8 FB D7 D6 D0 D7 D7 D0` | • `result_kind`: `0xFBB8 ^ 0xFBBC - 3 = \mathbf{1}$<br>•` matA`:` 0xD7 ^ 0xD3 \- 3 \= \\mathbf{1}\$ • `matB`: `0xD6 ^ 0xD3 = 5` \$\\rightarrow 5 \- 3 \= \\mathbf{2}\$ • `matC`: `0xD0 ^ 0xD3 - 3 = \mathbf{0}$<br>•` degA, degB, degC\`: \$\\mathbf{(1, 1, 0)}\$ | • `mat_sorted = (0, 1, 2)` • `deg_sorted = (0, 1, 1)` • Chuỗi: `"0" + "1" + "2" = "012"` • Khóa `u32`: `StrToInt("012") =` **`12`** |
| **Record \#1393** (Slot `1393`) | `0x2B88` | `8B FB F4 F4 F4 D7 D7 D7` | • `result_kind`: `0xFB8B ^ 0xFBBC = 0x0037 (55)` \$\\rightarrow 55 \- 3 \= \\mathbf{52}\$ • `matA,B,C`: `0xF4 ^ 0xD3 = 0x27 (39)` \$\\rightarrow 39 \- 3 \= \\mathbf{36}\$ • `degA,B,C`: \`0xD7 ^ 0xD3 \- 3 \= \\mathbf{(1, 1, 1)}\$ | • `mat_sorted = (36, 36, 36)` • `deg_sorted = (1, 1, 1)` • Chuỗi: `"36" + "36" + "36" = "363636"` • Khóa `u32`: `StrToInt("363636") =` **`363636`** |

---

## 5\. Cấu Trúc RAM Của `TFCompound` & Thuật Toán Tạo Khóa / Sắp Xếp

### 5.1. Bảng ánh xạ bộ nhớ (Memory Layout) của đối tượng `TFCompound` `[ĐÃ XÁC MINH]`

Lớp `TFCompound` có VMT đặt tại `0x005F2458`. Tại `0x005F2458 - 40 = 0x005F2430`, kích thước instance được khai báo là `0x0000C750` (`51.024` byte). Con trỏ toàn cục trỏ tới instance duy nhất của `TFCompound` nằm tại `0x007DA7E8` (`0079a890_FUN_0079a890.asm.txt:292`).

| Offset trên `TFCompound` | Kích thước | Kiểu dữ liệu tương đương (Delphi) | Vai trò | Chứng cứ ASM (`005f2528.asm.txt`) |
| :---: | :---: | :---- | :---- | :---- |
| `+0x0000 .. +0x0003` | `4` B | `Pointer` | Con trỏ VMT (`0x005F2458`) | `005f24b0_TFCompound.Create.asm.txt:15` |
| `+0x0004 .. +0x000B` | `8` B | `TCompoundRawRec` (Slot `0`) | Header đọc từ file (`u16 = 2`), bị xóa về `0` ở dòng `100` | `CMP word ptr [EAX+0x4], 0x2` (dòng `90`, `100`) |
| `+0x000C .. +0x5DCB` | `24.000` B | `array[1..3000] of TCompoundRawRec` (stride `8` B) | Bảng 1 (thô/đã giải mã tại chỗ): công thức `i` nằm tại `inst + 0x4 + i*8` | `005f2dd0_FUN_005f2dd0.asm.txt:25-153` |
| `+0x5DCC .. +0x5DD4` | `9` B | `TCompoundIndexRec` (Slot `0`) | Phần tử `0` của mảng thứ cấp (không dùng vì mảng 1-based `1..count`) | Công thức địa chỉ `inst + 0x5DCC + i*9` với `i >= 1` |
| `+0x5DD5 .. +0xC74C` | `27.000` B | `array[1..3000] of packed record` (stride `9` B) | **Bảng 2 (tra cứu đã sắp xếp)**: • `inst + 0x5DCC + i*9` (`u16`): `result_kind` • `inst + 0x5DCE + i*9` (`u32`): `lookup_key` • `inst + 0x5DD2 + i*9` (`u8[3]`): `deg[0..2]` | Dòng `141`: `MOV [EDX+EAX+0x5DCC], CX` Dòng `236`: `MOV [EDX+EAX+0x5DCE], ECX` Dòng `153`: `MOV [EAX+EDX+0x5DD2], CL` |
| `+0xC74D` | `1` B | `u8` (padding) | Byte căn chỉnh chẵn (2-byte alignment) của trình biên dịch Delphi | `3001 * 9 = 0x6981`; `0x5DCC + 0x6981 = 0xC74D` |
| `+0xC74E .. +0xC74F` | `2` B | `u16` | `recipe_count` \= tổng số công thức hợp lệ (`1.393`, tối đa `3.000 = 0xBB8`) | Dòng `69`: `MOV [EDX + 0xC74E], AX` |

- **Tại sao `TFCompound` lưu hai bảng trong RAM?** `[ĐÃ XÁC MINH]` Bảng 1 (`+0x0004`, stride `8` byte) khớp 1-1 với cấu trúc vật lý của `Compound.Dat` để `ReadFile` (`FUN_004067DC`) đọc thẳng toàn bộ file vào bộ nhớ chỉ bằng một syscall duy nhất, sau đó `FUN_005F2DD0` giải mã tại chỗ. Tuy nhiên, khi tra cứu, client không so sánh 3 byte `matA/B/C` rời rạc mà gộp chúng thành một khóa `u32` (4 byte) và sắp xếp lại toàn bộ bảng theo khóa này. Vì mỗi bản ghi tra cứu cần `2 (result_kind) + 4 (key u32) + 3 (deg[0..2]) = 9` byte (lớn hơn `8` byte của Bảng 1), dev đã khai báo mảng thứ hai tại `+0x5DCC` (stride `9` byte) để chứa dữ liệu đã chuẩn hóa và sắp xếp.

### 5.2. Thuật toán sắp xếp 2 tầng & Tạo khóa tra cứu `u32` `[ĐÃ XÁC MINH]`

Trong `FUN_005F2528` (`005f2528_FUN_005f2528.asm.txt:108-355`), quá trình chuẩn hóa bảng thứ cấp gồm 3 bước:

1. **Bước 1 — Sắp xếp nội bộ 3 nguyên liệu trong từng record (`0x005F27F4..0x005F295C`)**:  
   - Sao chép 3 byte `matA, matB, matC` ra mảng cục bộ `m[0..2]` (`[EBP-0x23..-0x21]`) và 3 byte `degA, degB, degC` vào `inst + 0x5DD2 + i*9 + (0..2)`.  
   - Chạy hai vòng lặp lồng nhau `outer = 0..2`, `inner = 0..2`: nếu `outer < inner` và `m[inner] < m[outer]`, hoán đổi đồng thời `m[outer] <-> m[inner]` **và** `deg[outer] <-> deg[inner]`.  
   - **Hệ quả quan trọng**: Với công thức chỉ có 2 nguyên liệu (`matC = 0, degC = 0`), sau khi sắp xếp tăng dần, giá trị `0` luôn bị đẩy về đầu mảng: `m[0] = 0` và `deg[0] = 0`\!  
2. **Bước 2 — Tạo khóa `u32` bằng ghép chuỗi thập phân (`0x005F295E..0x005F29F5`)**:  
   - Chuyển từng số `m[0]`, `m[1]`, `m[2]` thành chuỗi thập phân không có số `0` đệm đầu qua `IntToStr` (`0x00409980`).  
   - Nối 3 chuỗi lại qua `@LStrCatN(3)` (`0x00404148`): `s = IntToStr(m[0]) + IntToStr(m[1]) + IntToStr(m[2])`.  
   - Đổi chuỗi `s` thành số nguyên 32-bit có dấu qua `StrToInt` (`0x004099E4`), lấy trị tuyệt đối (`CDQ; XOR EAX, EDX; SUB EAX, EDX`), kiểm tra không âm rồi lưu vào `u32` tại `inst + 0x5DCE + i*9`.  
3. **Bước 3 — Sắp xếp toàn bảng thứ cấp theo khóa `u32` tăng dần (`0x005F2A40..0x005F2D05`)**:  
   - Dùng thuật toán Exchange Sort \$O(N^2)\$ với 2 vòng lặp `i = 1..count`, `j = 1..count`: nếu `i < j` và `table[j].key < table[i].key`, hoán đổi trọn bộ 3 thành phần (`key`, `result_kind`, `deg[0..2]`) giữa vị trí `i` và `j`.  
   - **Ảnh hưởng tới tra cứu**: Nhờ bảng thứ cấp đã được sắp xếp tăng dần theo `key`, hàm tra cứu `FUN_0079A890` (`0079a890_FUN_0079a890.asm.txt:299`) dùng lệnh nhảy thoát sớm `JA 0x0079B036` ngay khi gặp `table[i].key > search_key`, giúp giảm số phép so sánh trung bình từ \$1.393\$ xuống chỉ còn một phần nhỏ bảng.

### 5.3. Phân tích giới hạn tràn số & Hiện tượng va chạm khóa (Key Collision) `[ĐÃ XÁC MINH]`

- **Giới hạn tràn số (`IntOver`)**: Vì `m[k]` là kiểu `u8` (`0..255`), chuỗi ghép dài nhất có thể tạo ra là `"255255255"`, tương ứng với số nguyên `255.255.255` (\$\\approx 2,55 \\times 10^8\$), nhỏ hơn rất nhiều so với giới hạn `i32::MAX` (`2.147.483.647`) của `StrToInt`. Do đó **không bao giờ xảy ra tràn số 32-bit** với mọi giá trị `u8`. Trong dữ liệu thật (`m[k] <= 37`), khóa lớn nhất trong `Compound.Dat` là `363636` (hoặc tối đa lý thuyết `373737`).  
- **Phân tích toán học về va chạm khóa (Collision)**:  
  - **Trường hợp cả 3 ô đều có nguyên liệu (\$1 \\le m\_0 \\le m\_1 \\le m\_2 \\le 99\$)**: Vì \$m\_0 \\le m\_1 \\le m\_2\$, số chữ số thập phân của 3 thành phần cũng đơn điệu không giảm (\$\\text{len}(m\_0) \\le \\text{len}(m\_1) \\le \\text{len}(m\_2) \\in {1, 2}\$). Do đó tổng độ dài chuỗi \$L \\in {3, 4, 5, 6}\$ xác định **duy nhất** cấu trúc độ dài từng số: \$L=3 \\Rightarrow (1,1,1)\$; \$L=4 \\Rightarrow (1,1,2)\$; \$L=5 \\Rightarrow (1,2,2)\$; \$L=6 \\Rightarrow (2,2,2)\$. Vì vậy, trên miền \$1 \\le m\_0 \\le m\_1 \\le m\_2 \\le 99\$, phép ghép chuỗi thập phân **đơn ánh 100% (không thể va chạm)**.  
  - **Trường hợp 2 nguyên liệu (\$m\_0 \= 0\$) vs 3 nguyên liệu (\$m\_0 \\ge 1\$)**: Khi \$m\_0 \= 0\$, chuỗi `"0" + IntToStr(m1) + IntToStr(m2)` có chữ số `'0'` ở đầu, và hàm `StrToInt` sẽ bỏ qua chữ số `'0'` này\! Khi đó, một bộ 2 nguyên liệu \$(0, m\_1, m\_2)\$ có thể sinh ra cùng số nguyên `u32` với một bộ 3 nguyên liệu \$(m'\_0, m'\_1, m'\_2)\$\!  
    - **Ví dụ va chạm lý thuyết**: Bộ 2 nguyên liệu \$(0, 12, 34\) \\rightarrow \\text{"01234"} \\rightarrow 1234\$, trùng khít khóa `1234` với bộ 3 nguyên liệu \$(1, 2, 34\) \\rightarrow \\text{"1234"} \\rightarrow 1234\$.  
    - **Vì sao trong `Compound.Dat` không bị trộn lẫn công thức khi xảy ra va chạm khóa?** Trong `FUN_0079A890` (`0079a890_FUN_0079a890.asm.txt:310-354`), sau khi khớp `key`, client kiểm tra tiếp mảng `deg[0..2]`: bộ 2 nguyên liệu có `input_deg[0] = 0` và `recipe.deg[0] = 0`, trong khi bộ 3 nguyên liệu \$(1, 2, 34)\$ có `recipe.deg[0] = 1` và `input_deg[0] >= 1` (khi `input_deg[0] >= 1`, vòng lặp `d = input_deg[0] downto 1` chỉ chấp nhận `recipe.deg[0] >= 1`, loại bỏ mọi công thức có `recipe.deg[0] == 0`). Tuy nhiên, ở chiều ngược lại (người chơi bỏ 2 nguyên liệu \$(0, 12, 34)\$ nên `input_deg[0] == 0`), nhánh `k == 0` tại `0x0079AF04` lại bỏ qua kiểm tra `recipe.deg[0]` và cho khớp luôn\! Kiểm tra thực tế trên toàn bộ `1.393` bản ghi của `Compound.Dat` cho thấy không có cặp công thức 2-nguyên-liệu và 3-nguyên-liệu nào trùng khóa `u32` (các khóa 2 nguyên liệu đều \$\\le 3737\$, trong khi công thức 3 nguyên liệu có khóa \$\\le 3737\$ chỉ gồm các bộ \$(1..3, 1..7, 1..37)\$ không trùng với các cặp 2 nguyên liệu hiện có).

---

## 6\. Chuỗi Đọc File RTL & Giải Thích Cái Bẫy `count - 1` Của Decompiler

### 6.1. Sơ đồ 6 hàm RTL I/O khi nạp `Compound.Dat` `[ĐÃ XÁC MINH]`

Trong `FUN_005F2528` (`005f2528_FUN_005f2528.asm.txt:52-88`), client dùng hệ thống Untyped File chuẩn của Delphi RTL (`TFileRec` đặt trên stack tại `[EBP - 0x168]`):

1. **`FUN_0040673A(&fileRec, str_Data_Compound_Dat)` (`@AssignFile`)**: Khởi tạo `TFileRec`, copy đường dẫn `\Data\Compound.Dat` (`0x005F2D2C`) vào buffer tên file tại `fileRec + 0x0C` (`0040673a_FUN_0040673a.asm.txt`).  
2. **`FUN_00406CF9(&fileRec, 8)` (`@ResetFile`)**: Đặt kích thước mỗi bản ghi `fileRec.RecSize = 8` (`MOV [EAX + 0x08], EDX`) và gọi `0x00406BD0` (`CreateFileA`) mở file ở chế độ đọc. **Hàm này hoàn toàn KHÔNG seek qua header** (`00406cf9_FUN_00406cf9.asm.txt:16-19`).  
3. **`FUN_00406BF8()` (`IOResult`)**: Đọc và xóa biến toàn cục `InOutRes` (`[0x007C9064]`). Nếu khác `0` (file không tồn tại hoặc lỗi mở file), thoát sớm khỏi `FUN_005F2528` (`00406bf8_FUN_00406bf8.c`).  
4. **`FUN_004069DC(&fileRec)` (`@FileSize` — gọi lần 1 tại `0x005F2599`)**: Lấy kích thước file (`11.152` byte) chia nguyên cho `RecSize` (`8`), trả về `EAX = 1394`. Ngay sau đó:  
   - `DEC EAX` (`1394 - 1 = 1393`)  
   - `MOV [EDX + 0xC74E], AX` (Lưu `recipe_count = 1393` vào `inst + 0xC74E`).  
5. **`FUN_004069DC(&fileRec)` (`@FileSize` — gọi lần 2 tại `0x005F25B7`) & `FUN_004067DC` (`@BlockRead`)**:  
   - Lần gọi thứ hai trả về `ECX = EAX = 1394` (**không có lệnh `DEC EAX`**).  
   - Truyền `ECX = 1394`, `EDX = inst + 0x0004`, `EAX = &fileRec` vào `FUN_004067DC` (`0x005F25CE`).  
   - `FUN_004067DC` nhân `1394 * 8 = 11.152` byte và gọi `ReadFile` đọc từ offset `0` của file vào `inst + 0x0004 .. inst + 0x2B93` (`004067dc_FUN_004067dc.c`).  
6. **`FUN_004068A4(&fileRec)` (`@CloseFile`)**: Đóng handle của file vừa đọc (`004068a4_FUN_004068a4.c`).

### 6.2. Vì sao Pseudo-C của Ghidra gây hiểu lầm là đọc `count - 1` record? `[ĐÃ XÁC MINH]`

Trong `005f2528_FUN_005f2528.c:139-143`, Ghidra sinh ra đoạn mã giả cực kỳ dễ đánh lừa:

uVar2 \= FUN\_004069dc(auStack\_168);

uVar2 \= FUN\_00402864();

\*(short \*)(\*piVar9 \+ 0xc74e) \= (short)uVar2 \+ \-1;

uVar2 \= FUN\_004069dc(auStack\_168);

iVar3 \= FUN\_00402864();

FUN\_004067dc(auStack\_168,(void \*)(\*piVar9 \+ 4),iVar3);

Đối chiếu với `005f2528_FUN_005f2528.asm.txt:62-82` và `00402864_FUN_00402864.asm.txt`:

005f2599: CALL 0x004069dc       ; EAX \= FileSize(\&fileRec) \= 1394

005f259e: CALL 0x00402864       ; @\_IOTest: bên trong PUSH EAX/EDX/ECX rồi POP ECX/EDX/EAX \-\> giữ nguyên EAX \= 1394\!

005f25a3: DEC  EAX              ; EAX \= 1393

005f25a4: JO   0x005f25ab

005f25a6: CMP  EAX, 0xffff

005f25ab: JBE  0x005f25b2

005f25ad: CALL 0x00402fa8       ; @\_BoundErr

005f25b2: MOV  EDX, \[EBP-0x4\]

005f25b5: MOV  \[EDX+0xc74e\], AX ; inst-\>recipe\_count \= 1393

005f25b7: LEA  EAX, \[EBP-0x168\]

005f25bd: CALL 0x004069dc       ; EAX \= FileSize(\&fileRec) \= 1394 (GỌI LẦN 2\)

005f25c2: CALL 0x00402864       ; @\_IOTest: giữ nguyên EAX \= 1394

005f25c7: MOV  ECX, EAX         ; ECX \= 1394 (số record cần đọc)

005f25c9: MOV  EDX, \[EBP-0x4\]

005f25cc: ADD  EDX, 0x4         ; EDX \= inst \+ 0x0004 (buffer đích)

005f25cf: LEA  EAX, \[EBP-0x168\]

005f25d5: CALL 0x004067dc       ; @BlockRead(\&fileRec, inst \+ 4, 1394\)

- **Nguyên nhân lỗi của Ghidra**: Hàm `FUN_00402864` (`@_IOTest`) thực chất chỉ kiểm tra `CMP dword ptr [0x007C9064], 0` và bọc `PUSH EAX / PUSH EDX / PUSH ECX ... POP ECX / POP EDX / POP EAX` để **không làm hỏng thanh ghi trả về `EAX` của hàm đứng trước nó**. Ghidra không biết quy ước bảo toàn thanh ghi đặc thù này của Delphi RTL nên tưởng `FUN_00402864` là một hàm chuẩn trả về kết quả trong `EAX` (`uVar2 = FUN_00402864()`, `iVar3 = FUN_00402864()`), làm đứt luồng dữ liệu từ `FUN_004069DC` sang `DEC EAX` và sang `MOV ECX, EAX`.  
- **Đối chiếu với `src/data/loaders/compound.rs` (Server Rust)**: Nếu phía Rust bỏ qua 8 byte đầu (header `02 00 ...`) rồi đọc tiếp `1.393` bản ghi 8 byte tiếp theo thì **kết quả dữ liệu hoàn toàn giống hệt** với việc Client đọc cả `1.394` slot vào mảng `0..1393` rồi chỉ xử lý từ chỉ số `1` đến `1393`.  
- **Dung lượng tối đa của Buffer trong `TFCompound`**:  
  - Vùng buffer thứ nhất `inst + 0x0004 .. inst + 0x5DCB` có kích thước `0x5DC8 = 24.000` byte (cộng 8 byte slot `0` ở `+0x0004..+0x000B` là `24.008` byte \= `3.001` slot × `8` byte).  
  - Trong `FUN_005F2DD0` (`005f2dd0_FUN_005f2dd0.asm.txt:20`) và `FUN_005F2528` (`005f2528_FUN_005f2528.asm.txt:116`), mọi truy cập mảng đều kiểm tra `CMP EAX, 0xBB8; JBE ...; CALL 0x00402FA8` (`0xBB8 = 3000`).  
  - **Cảnh báo tràn bộ nhớ đệm (Buffer Overflow)**: Nếu file `Compound.Dat` trên đĩa có nhiều hơn `3.001` slot (tức kích thước file \$\> 24.008\$ byte), lệnh `FUN_004067DC` tại `0x005F25D5` **không kiểm tra trần `3000` trước khi gọi `ReadFile`**, nên sẽ ghi đè trực tiếp sang bảng thứ hai (`+0x5DCC`), sang `recipe_count` (`+0xC74E`), và tràn ra ngoài heap của `TFCompound`\!

---

## 7\. Luồng Giao Diện (UI Flow) & Cấu Trúc Form Mix (`0x0079EE08`)

### 7.1. Bảng Widget của Form Mix Simulator (`FUN_0079EE08`) `[ĐÃ XÁC MINH]`

Được kiểm chứng độc lập từ `0079ee08_FUN_0079ee08.asm.txt` và `hex_79f31b_79f3c4_ui_strings.txt`:

| Offset trên `Self` | Tên Class (tại `[VMT - 44]`) | VMT / ClassRef | Hàm tạo (Creator) | Tọa độ `(X, Y, W, H)` | Giá trị mặc định / Thuộc tính | Vai trò trên UI |
| :---: | :---- | :---: | :---: | :---: | :---- | :---- |
| `Self + 0x120` | `TSe_Form` | `[0x007AD3B0]` | `0x007B0ECC` | `(50, 50, 597, 369)` | Nền `0x00804000`, viền khung | Khung nền của Form Mix |
| `Self + 0x134` | `TSe_Editor` | `[0x007B11C4]` | `0x007B34FC` | `(75, 19, 44, 14)` | Text `= "10001"` (`0x0079F338`) | **Ô nhập 1 (trên): `item_id` nguyên liệu 1 (`i32`)** |
| `Self + 0x138` | `TSe_Editor` | `[0x007B11C4]` | `0x007B34FC` | `(167, 19, 44, 14)` | Text `= "17001"` (`0x0079F348`) | **Ô nhập 2 (trên): `item_id` nguyên liệu 2 (`i32`)** |
| `Self + 0x13C` | `TSe_Editor` | `[0x007B11C4]` | `0x007B34FC` | `(264, 19, 44, 14)` | Text `= "10001"` (`0x0079F338`) | **Ô nhập 3 (trên): `item_id` nguyên liệu 3 (`i32`)** |
| `Self + 0x140` | `TSe_Editor` | `[0x007B11C4]` | `0x007B34FC` | `(75, 42, 44, 14)` | Text `= "1"` (`0x0079F358`) | **Ô nhập 4 (dưới ô 1): Số lượng `qty` nguyên liệu 1 (`u8`)** |
| `Self + 0x144` | `TSe_Editor` | `[0x007B11C4]` | `0x007B34FC` | `(167, 42, 44, 14)` | Text `= "1"` (`0x0079F358`) | **Ô nhập 5 (dưới ô 2): Số lượng `qty` nguyên liệu 2 (`u8`)** |
| `Self + 0x148` | `TSe_Editor` | `[0x007B11C4]` | `0x007B34FC` | `(264, 42, 44, 14)` | Text `= "1"` (`0x0079F358`) | **Ô nhập 6 (dưới ô 3): Số lượng `qty` nguyên liệu 3 (`u8`)** |
| `Self + 0x14C` | `TSe_SelectText` | `[0x007BACC0]` | `0x007B2E80` | `(16, 65, 290, 290)` | Gắn ScrollButton `Self + 0x154` | **Hộp danh sách TRÁI**: In thông báo lỗi hoặc kết quả vật phẩm hợp thành (`Id: ... Num:...`) |
| `Self + 0x150` | `TSe_SelectText` | `[0x007BACC0]` | `0x007B2E80` | `(326, 65, 250, 290)` | Gắn ScrollButton cục bộ | **Hộp danh sách PHẢI**: Dump toàn bộ bảng `Compound.Dat` và thông số `Mate kind ratA ratB` của 3 nguyên liệu |
| `Self + 0x154` | `TSe_ScrollButton` | `[0x007BAADC]` | `0x007BDC60` | `(307, 65, 16, 290)` | Gắn với `Self + 0x14C` | Thanh cuộn dọc của hộp danh sách trái (`Self + 0x14C`) |
| *(Biến cục bộ)* | `TSe_ScrollButton` | `[0x007BAADC]` | `0x007BDC60` | `(577, 65, 16, 290)` | Gắn với `Self + 0x150` | Thanh cuộn dọc của hộp danh sách phải (`Self + 0x150`) |
| *(Biến cục bộ)* | `TSe_FixedButton` | `[0x007B567C]` | `0x007B7928` | `(330, 17, 42, 20)` | Caption `= "合成"` (`0x0079F3C0`), `OnClick = 0x0079F3C8` | **Nút bấm thực thi Mix (`合成`)** |

### 7.2. Chứng cứ ASM cho từng bước trong Luồng UI `[ĐÃ XÁC MINH]`

1. **Gắn caption `"合成"` và sự kiện `OnClick = 0x0079F3C8` vào nút `TSe_FixedButton`** (`0079ee08_FUN_0079ee08.asm.txt:394-409`):  
     
   0079f2ec: CALL 0x007b7928                   ; EAX \= TSe\_FixedButton.Create(...)  
     
   0079f2f1: MOV  EBX, EAX                     ; EBX \= button instance  
     
   0079f2f8: LEA  EAX, \[EBX \+ 0x84\]            ; button.Caption (AnsiString @ \+0x84)  
     
   0079f2fe: MOV  EDX, 0x0079f3c0              ; literal "合成" (Big5: A6 X4 A6 X8)  
     
   0079f303: CALL 0x00403e98                   ; @LStrAsg(button.Caption, "合成")  
     
   0079f305: MOV  dword ptr \[EBX \+ 0x50\], 0x0079f3c8 ; button.OnClick.Code \= 0x0079F3C8  
     
   0079f30c: MOV  dword ptr \[EBX \+ 0x54\], ESI  ; button.OnClick.Data \= Self (Form)  
     
2. **Đọc 6 ô `TSe_Editor` trong Handler `0x0079F3C8`: 3 ô đầu là `item_id` (`i32`), 3 ô sau là `qty` (`u8`)** (`0079f3c8_mix_handler.asm.txt:29-67`):  
     
   - **Xác nhận kèm hiệu chỉnh quan trọng**: Đúng là về kiểu dữ liệu máy, 3 ô đầu (`+0x134, +0x138, +0x13C`) được lưu thành **3 số nguyên `i32`** (mỗi phần tử chiếm 4 byte đầu của struct 8-byte tại `[EBP - 0x20]`, `[EBP - 0x18]`, `[EBP - 0x10]`), còn 3 ô sau (`+0x140, +0x144, +0x148`) được kiểm tra `CMP EAX, 0xFF; JBE ...` rồi lưu thành **1 byte `u8`** tại offset `+4` của mỗi struct (`[EBP - 0x1C]`, `[EBP - 0x14]`, `[EBP - 0x0C]`). Tuy nhiên, về **ngữ nghĩa nghiệp vụ**, byte `u8` ở 3 ô sau **KHÔNG chỉ là cờ bật/tắt (0/1)** mà là **số lượng vật phẩm (`qty`, từ `0` đến `50`)** của từng ô\! Bằng chứng:  
     - `FUN_0079A5E4` kiểm tra nếu byte này `> 50` (`0x32`) thì trả về mã lỗi `3` ứng với thông báo `: 單項物品數量不可超過50` (*"Số lượng một vật phẩm đơn lẻ không được vượt quá 50"*).  
     - `FUN_0079A890` dùng byte này để tính số lượng thành phẩm đầu ra (`min_qty`) và tỷ lệ phối trộn `input_deg[k] = min(5, qty[k] div min_qty)`.

   

   0079f3ef: MOV  EAX, \[EBX \+ 0x134\]     ; Editor 1 (item\_id 1\)

   

   0079f3f5: MOV  EAX, \[EAX \+ 0x120\]     ; Editor.Text (AnsiString @ \+0x120)

   

   0079f3fb: CALL 0x004099e4             ; StrToInt

   

   0079f400: MOV  \[EBP \- 0x20\], EAX      ; entries\[0\].item\_id \= EAX (i32)

   

   ...

   

   0079f432: MOV  EAX, \[EBX \+ 0x140\]     ; Editor 4 (qty 1\)

   

   0079f438: MOV  EAX, \[EAX \+ 0x120\]     ; Editor.Text

   

   0079f43e: CALL 0x004099e4             ; StrToInt

   

   0079f443: CMP  EAX, 0xff              ; Kiểm tra tràn kiểu Byte (0..255)

   

   0079f448: JBE  0x0079f44f

   

   0079f44a: CALL 0x00402fa8             ; @\_BoundErr

   

   0079f44f: MOV  \[EBP \- 0x1c\], AL       ; entries\[0\].qty \= AL (u8)

   

3. **Phân vai 2 hộp danh sách `TSe_SelectText`: `Self + 0x14C` (Trái) và `Self + 0x150` (Phải)** `[ĐÃ XÁC MINH]`:  
     
   - **Hộp trái (`Self + 0x14C`)**: Nhận toàn bộ **thông báo lỗi validate** (`0x0079F4EC`, `0x0079F539`, `0x0079F586`), **thông báo lỗi tra cứu** (`0x0079F761`, `0x0079F7AA`, `0x0079F7F3`) và **kết quả vật phẩm hợp thành thành công** (`Id: <id> <tên> Num:<số_lượng>` tại `0x0079F899`).  
   - **Hộp phải (`Self + 0x150`)**: Khi đầu vào vượt qua bước validate (`FUN_0079A5E4 == 0`), handler xóa trắng hộp `Self + 0x150` (`MOV byte ptr [EAX + 0x896], 0` tại `0x0079F5D8`), rồi:  
     1. In tiêu đề `"Key  Id  D1  D2  D3"` (`0x0079F93C`) và duyệt `1..recipe_count` để in toàn bộ `1.393` dòng của bảng thứ cấp `TFCompound` vào `Self + 0x150` (`0x0079F5EC..0x0079F68D`).  
     2. In dòng phân cách, tiêu đề `"Mate  kind  ratA  ratB"` (`0x0079F968`), và duyệt 3 nguyên liệu đầu vào để in 4 trường của từng nguyên liệu lấy từ `TFItem`: `item_id` (`+0x1A`), `kind` (`+0x19`), `ratA` (`+0x32`), `ratB` (`+0x33`) vào `Self + 0x150` (`0x0079F692..0x0079F73B`).

---

## 8\. Luật Nghiệp Vụ Kiểm Tra Đầu Vào (`FUN_0079A5E4`)

### 8.1. Bảng điều kiện, mã trả về và thông báo lỗi `[ĐÃ XÁC MINH]`

Đối chiếu từ `0079a5e4_FUN_0079a5e4.asm.txt`, `0079f3c8_mix_handler.asm.txt:76-152` và `hex_79f8bc_79fa40_msg_strings.txt`:

| Thứ tự kiểm tra | Điều kiện trong `FUN_0079A5E4` | Mã trả về (`AL`) | Địa chỉ chuỗi lỗi trong `0x0079F3C8` | Chuỗi gốc (Big5) | Nghĩa tiếng Việt |
| :---: | :---- | :---: | :---: | :---- | :---- |
| **1** | Số ô có `(item_id > 0 && qty > 0)` nhỏ hơn `2` (`valid_count < 2`) | **`1`** | `0x0079F8C4` | `: 合成至少要二樣物品` | Hợp thành cần ít nhất 2 vật phẩm |
| **2a** | Tồn tại ô có `item_id > 0` nhưng `item_id < 10001` (`0x2711`) hoặc `item_id > 50000` (`0xC350`) | **`2`** | `0x0079F8E0` | `: 參雜有不能合成的物品` | Có lẫn vật phẩm không thể dùng để hợp thành |
| **2b** | Tồn tại ô có `item_id > 0` nhưng `FUN_007746AC(item_id) == 0` (không tồn tại trong `Item.dat`) | **`2`** | `0x0079F8E0` | `: 參雜有不能合成的物品` | Có lẫn vật phẩm không thể dùng để hợp thành |
| **2c** | Tồn tại ô có `item_id > 0` nhưng `item.kind` (`item + 0x19`) \$\\in {20, 21, 22}\$ | **`2`** | `0x0079F8E0` | `: 參雜有不能合成的物品` | Có lẫn vật phẩm không thể dùng để hợp thành |
| **3** | Tồn tại ô có `item_id > 0` nhưng `qty > 50` (`0x32`) | **`3`** | `0x0079F900` | `: 單項物品數量不可超過50` | Số lượng một vật phẩm đơn lẻ không được vượt quá 50 |
| **4** | Vượt qua toàn bộ các kiểm tra trên | **`0`** | *(Không báo lỗi)* | *(Tiếp tục dump & gọi `FUN_0079A890`)* | Đầu vào hợp lệ |

> **Lưu ý tinh tế từ ASM (`0079a5e4_FUN_0079a5e4.asm.txt:39-57`)**: Ở vòng lặp thứ nhất, nếu một ô có `item_id > 0` nhưng `qty == 0`, biến `valid_count` không tăng (và ô đó được giữ nguyên, không bị xóa `item_id`), nhưng sang các vòng lặp 2, 3, 4, điều kiện kiểm tra chỉ là `CMP dword ptr [EBP + EAX*8 - 0x1C], 0; JLE ...` (tức chỉ kiểm tra `item_id > 0`). Tuy nhiên, nếu một ô có `item_id <= 0` hoặc `qty == 0` khiến nhánh `else` ở dòng `49` chạy, lệnh `MOV dword ptr [EBP + EAX*8 - 0x1C], 0` và `MOV byte ptr [EBP + EAX*8 - 0x18], 0` sẽ chuẩn hóa cả `item_id = 0` và `qty = 0` cho ô đó\! Nhờ vậy, sau vòng lặp 1, mọi ô không hợp lệ đều đã bị ép `item_id = 0, qty = 0`.

### 8.2. Pseudo-code sạch của `FUN_0079A5E4` `[ĐÃ XÁC MINH]`

\#\[derive(Clone, Copy, Debug)\]

pub struct MixInputSlot {

    pub item\_id: i32, // offset \+0..+3

    pub qty: u8,      // offset \+4 (cộng 3 byte padding \+5..+7 \= stride 8 byte)

}

pub fn validate\_mix\_inputs(slots: &\[MixInputSlot; 3\], item\_db: \&ItemDatabase) \-\> u8 {

    let mut local \= \*slots; // Copy 24 byte vào stack frame (REP MOVSD, ECX=6)

    let mut valid\_count: u8 \= 0;

    // 1\. Đếm số ô hợp lệ và xóa sạch các ô có item\_id \<= 0 hoặc qty \== 0

    for s in \&mut local {

        if s.item\_id \> 0 && s.qty \> 0 {

            valid\_count \+= 1;

        } else {

            s.item\_id \= 0;

            s.qty \= 0;

        }

    }

    if valid\_count \< 2 {

        return 1; // ": 合成至少要二樣物品"

    }

    // 2\. Kiểm tra dải ID \[10001, 50000\]

    for s in \&local {

        if s.item\_id \> 0 && (s.item\_id \< 10001 || s.item\_id \> 50000\) {

            return 2; // ": 參雜有不能合成的物品"

        }

    }

    // 3\. Kiểm tra số lượng tối đa mỗi ô \<= 50 (0x32)

    for s in \&local {

        if s.item\_id \> 0 && s.qty \> 50 {

            return 3; // ": 單項物品數量不可超過50"

        }

    }

    // 4\. Kiểm tra tồn tại trong Item.dat và cấm kind ∈ {20, 21, 22}

    for s in \&local {

        if s.item\_id \> 0 {

            let Some(item) \= item\_db.get\_by\_id(s.item\_id as u16) else {

                return 2; // FUN\_007746AC trả về 0 \-\> ": 參雜有不能合成的物品"

            };

            // ASM 0x0079A71A: ADD AL, 0xEC; SUB AL, 0x3; JC 0x0079A722

            if (20..=22).contains(\&item.kind) {

                return 2; // ": 參雜有不能合成的物品"

            }

        }

    }

    0 // Hợp lệ

}

### 8.3. Ý nghĩa ngưỡng `10001..50000` và `item.kind ∈ {20, 21, 22}` `[ĐÃ XÁC MINH]` & `[SUY LUẬN]`

- **Ngưỡng `10001 <= item_id <= 50000`**:  
  - Trong `items.json` (trích xuất từ `Item.dat`), mọi vật phẩm trong game đều có `id >= 10001` (vật phẩm đầu tiên là `10001` — 布帽 / Mũ vải). Không có vật phẩm thông thường nào có `id <= 10000`.  
  - Các vật phẩm trong dải `10001..50000` bao gồm toàn bộ trang bị (mũ, giáp, vũ khí, hộ uyển, giày, đặc thủ: `10xxx..24xxx`), khoáng thạch, vật liệu, dược phẩm và vật phẩm tiêu hao thông thường (`25xxx..49xxx`).  
  - Các vật phẩm có `id > 50000` (`50001..65526`) là vật phẩm nhiệm vụ đặc biệt, vật phẩm sự kiện/Cash Shop (Point Mall), bùa chú đặc biệt hoặc đạo cụ hệ thống không được phép đưa vào lò hợp thành.  
- **Vì sao `item.kind ∈ {20, 21, 22}` bị cấm làm nguyên liệu (và cấm làm kết quả)?**:  
  - Trong cấu trúc `TFItem`, byte tại offset `+0x19` là mã phân loại vật phẩm (`kind` / `type` trong `items.json`).  
  - Kiểm tra `FUN_0079A890` (`0079a890_FUN_0079a890.asm.txt:458-462`) cho thấy `kind ∈ 1..14` là **Vũ khí & Trang bị** (khi合 ra `kind ∈ 1..14`, số lượng thành phẩm luôn bị ép về `1` vì trang bị không xếp chồng / non-stackable).  
  - Các `kind` từ `23..52` (và `15..19`) là vật phẩm tiêu hao, khoáng thạch, vải, da, gỗ, giấy, nguyên liệu chế tạo.  
  - Ba mã `kind ∈ {20, 21, 22}` bị chặn cứng ở cả đầu vào (`FUN_0079A5E4`) lẫn đầu ra (`FUN_0079A890` trả lỗi `2`: *"Lỗi cơ sở dữ liệu hợp thành, hợp thành ra loại không được phép"*) vì đây là 3 nhóm vật phẩm đặc thù không thuộc hệ thống chế tạo (như vật phẩm nhiệm vụ/tín vật, sách/cuộn đặc biệt hoặc bảo vật không thể tổng hợp) `[SUY LUẬN — nhãn tên cụ thể của 3 mã 20/21/22 cần đối chiếu thêm bảng enum ItemKind đầy đủ của server]`.

---

## 9\. Giải Quyết Câu Hỏi Mở (§2.2) & Thuật Toán Tra Cứu Công Thức (`FUN_0079A890`)

### 9.1. Kết luận dứt điểm về ngữ nghĩa các trường (`result_id`, `material`, `degree`, `ratA`, `ratB`) `[ĐÃ XÁC MINH]`

Bằng cách lần theo từng lệnh truy cập bộ nhớ trong `0079a890_FUN_0079a890.asm.txt` và đối chiếu với `0079f3c8_mix_handler.asm.txt:210-258`, ta xác định **chính xác tuyệt đối** mối liên hệ giữa `Compound.Dat` và `Item.dat` (`TFItem`):

1. **Bốn trường cốt lõi của bản ghi `TFItem` (stride `0x172 = 370` byte, 1-based index `idx = FUN_007746AC(item_id)`)**:  
     
   - `item + 0x19` (`u8`): **`kind`** (loại vật phẩm, tương ứng trường `"type"` trong `items.json`, giá trị `1..65`).  
   - `item + 0x1A` (`u16`): **`item_id`** (mã định danh vật phẩm thực sự `10001..65526`).  
   - `item + 0x32` (`u8`): **`ratA`** (mã **chất liệu / Material class** của vật phẩm, giá trị `1..37`).  
   - `item + 0x33` (`u8`): **`ratB`** (cấp **phẩm chất / bậc hợp thành / Compound Rank** của vật phẩm, giá trị `1..10+`).  
   - *Chứng cứ trực tiếp*: Đoạn code dump thông số nguyên liệu trong `0x0079F3C8` (`0079f3c8_mix_handler.asm.txt:210-252`) in tiêu đề `"Mate  kind  ratA  ratB"` (`0x0079F968`) rồi lần lượt lấy:  
     - `Mate` \= `[EBP + ESI*8 - 0x20]` (`entries[k].item_id`)  
     - `kind` \= `byte ptr [EDX + EAX*2 + 0x19]` (`item + 0x19`)  
     - `ratA` \= `byte ptr [EDX + EAX*2 + 0x32]` (`item + 0x32`)  
     - `ratB` \= `byte ptr [EDX + EAX*2 + 0x33]` (`item + 0x33`)

   

2. **`material A/B/C` (`0..37`) trong `Compound.Dat` là gì?** \$\\rightarrow\$ **Chính là `item.ratA` (`item + 0x32`)\!** `[ĐÃ XÁC MINH]`  
     
   - *Chứng cứ*: Tại `0079a890_FUN_0079a890.asm.txt:109-123`, với mỗi nguyên liệu đầu vào `k = 0..2`, client tra `idx = FUN_007746AC(item_id)` rồi đọc `AL = byte ptr [EDX + EAX*2 + 0x32]` (`item.ratA`) lưu vào `[EBP + k - 0x1F]`, sau đó copy sang `[EBP + k - 0x48]`, sắp xếp tăng dần 3 byte `ratA` này, ghép chuỗi thập phân tạo khóa `search_key` (`[EBP - 0x50]`) và đem so sánh trực tiếp với khóa `u32` của `Compound.Dat` tại `inst + 0x5DCE + i*9` (`asm:297`)\!

   

3. **`result id` (`1..52`) trong `Compound.Dat` là gì?** \$\\rightarrow\$ **Chính là `item.kind` (`item + 0x19`) của vật phẩm đầu ra\!** `[ĐÃ XÁC MINH]`  
     
   - *Chứng cứ 1*: Khi không tìm thấy công thức nào trong `Compound.Dat` (`local_40 == 0` tại `asm:382`), nhánh fallback chọn ngẫu nhiên 1 nguyên liệu đầu vào và đọc `byte ptr [EDX + EAX*2 + 0x19]` (`item.kind`) gán thẳng vào mảng ứng viên kết quả `auStack_80[0]` (`[EBP - 0x7C]`, `asm:420-424`) — cùng mảng chứa `result_id` lấy từ `Compound.Dat` (`inst + 0x5DCC + i*9`, `asm:371-378`)\!  
   - *Chứng cứ 2*: Sau khi chọn 1 giá trị `result_kind` (`[EBP - 0x5C]`) từ mảng `auStack_80`, vòng lặp quét `TFItem` (`asm:487-538`) tìm các vật phẩm thỏa mãn đồng thời 2 điều kiện:  
     - `byte ptr [EDX + i*0x172 + 0x19] == result_kind` (`asm:495-499`)  
     - `byte ptr [EDX + i*0x172 + 0x33] == target_rank` (`asm:510-514`) Khi cả hai khớp, nó lấy **`word ptr [EDX + i*0x172 + 0x1A]`** (`item_id` thật) đưa vào danh sách ứng viên thành phẩm `auStack_94` (`[EBP + idx*2 - 0x90]`, `asm:531-536`)\!

   

4. **`degree A/B/C` (`0` hoặc `1`) trong `Compound.Dat` là gì?** `[ĐÃ XÁC MINH]`  
     
   - `degree` là **hệ số tỷ lệ số lượng tối thiểu** của từng thành phần nguyên liệu sau khi đã chuẩn hóa theo ước số lượng nhỏ nhất (`min_qty`).  
   - Cụ thể (`asm:234-275`): Giả sử người chơi đặt vào 3 ô với số lượng `(qty_0, qty_1, qty_2)` (đã hoán vị đi kèm theo thứ tự sắp xếp của `ratA`). Hàm tìm giá trị nhỏ nhất **khác 0** trong 3 số lượng đó gọi là `min_qty` (nếu cả 3 bằng 0 thì `min_qty = 1`).  
   - Bậc tỷ lệ đầu vào của từng ô được tính bằng phép chia nguyên: \$\$\\text{input\_deg}\[k\] \= \\min\\left(5, \\left\\lfloor \\frac{\\text{qty}\[k\]}{\\text{min\_qty}} \\right\\rfloor\\right) \\in {0, 1, 2, 3, 4, 5}\$\$  
   - Ô trống (`qty[k] == 0`) sẽ có `input_deg[k] = 0`. Ô có nguyên liệu sẽ có `input_deg[k] \in 1..5`.  
   - Trong `Compound.Dat` hiện tại, mọi công thức đều có `deg = 1` cho các ô có nguyên liệu và `deg = 0` cho ô trống (`mat == 0`). Cấu trúc code hỗ trợ công thức yêu cầu tỷ lệ `deg` lên tới `5` (ví dụ tỷ lệ `1:2:1`), và điều kiện khớp cho từng ô `k` là:  
     - Nếu `k == 0` và `input_deg[0] == 0` (công thức 2 nguyên liệu): khớp.  
     - Nếu `input_deg[k] >= 1`: công thức phải có `1 <= recipe.deg[k] <= input_deg[k]` (vòng lặp `d` chạy lùi từ `input_deg[k]` về `1` tại `asm:323-348`).

### 9.2. Giải thích cảnh báo của Ghidra: `"Removing unreachable block (ram,0x0079af0d)"` `[ĐÃ XÁC MINH]`

Trong `0079a890_FUN_0079a890.c:4`, Ghidra cảnh báo đã xóa block tại `0x0079AF0D`. Đối chiếu `0079a890_FUN_0079a890.asm.txt:311-319`:

0079af00: CMP  dword ptr \[EBP \- 0x30\], 0x0      ; Kiểm tra k \== 0?

0079af04: JNZ  0x0079af1f                       ; Nếu k \!= 0 \-\> nhảy tới 0x0079AF1F

0079af06: MOV  EAX, dword ptr \[EBP \- 0x30\]      ; Ở đây chắc chắn EAX \== 0\!

0079af09: CMP  EAX, 0x2                         ; Kiểm tra biên mảng 0..2 của Delphi (0 \<= 2 luôn đúng\!)

0079af0c: JBE  0x0079af12                       ; Luôn nhảy tới 0x0079AF12\!

0079af0e: CALL 0x00402fa8                       ; \<--- Block 0x0079AF0D/0x0079AF0E: CALL @\_BoundErr (DEAD CODE)

0079af12: CMP  byte ptr \[EBP \+ EAX \- 0x53\], 0x0 ; Kiểm tra input\_deg\[0\] \== 0?

0079af17: JNZ  0x0079af1f

0079af19: MOV  byte ptr \[EBP \- 0x45\], 0x1       ; slot\_matched \= true

0079af1d: JMP  0x0079af98

- **Kết luận**: Trình biên dịch Delphi tự động sinh mã kiểm tra chỉ số mảng (`CMP EAX, 2; JBE ...; CALL @_BoundErr`) khi truy cập `input_deg[k]` trong biểu thức `if (k = 0) and (input_deg[k] = 0)`. Vì ngay trước đó đã có điều kiện `k == 0`, nhánh gọi `@_BoundErr` tại `0x0079AF0E` không bao giờ có thể xảy ra. Ghidra chỉ loại bỏ lệnh gọi `@_BoundErr` thừa đó, **không làm mất bất kỳ nhánh logic nghiệp vụ nào** trong `.c`.

### 9.3. Pseudo-code đầy đủ và chuẩn xác của `FUN_0079A890` `[ĐÃ XÁC MINH]`

/// Trả về:

/// \- Ok((out\_item\_id, out\_qty)) tương ứng mã trả về 0 của FUN\_0079A890

/// \- Err(2): ": 合成資料庫錯誤,合成出不允許的類別" (result\_kind ∈ {20, 21, 22})

/// \- Err(3): ": 物品資料庫無此合成物品" (không có item nào trong Item.dat khớp result\_kind & target\_rank)

/// Lưu ý: Mã trả về 1 (": 合成資料庫無此合成法") KHÔNG BAO GIỜ XẢY RA do có cơ chế Fallback ở Bước 5\.

pub fn execute\_compound\_lookup(

    slots: &\[MixInputSlot; 3\],

    compound\_db: \&CompoundDatabase,

    item\_db: \&ItemDatabase,

    rng: \&mut DelphiRng,

) \-\> Result\<(u16, u8), u8\> {

    // \--- BƯỚC 1: Chuẩn hóa 3 ô đầu vào và tra ratA (+0x32), ratB (+0x33) từ TFItem \---

    let mut local\_slots \= \*slots;

    let mut active\_count: u8 \= 3; // local\_27 (\[EBP \- 0x23\])

    let mut mat\_rat\_a \= \[0u8; 3\]; // \[EBP \- 0x1F .. \-0x1D\]

    let mut rank\_rat\_b \= \[0u8; 3\]; // \[EBP \- 0x22 .. \-0x20\]

    for k in 0..3 {

        if local\_slots\[k\].item\_id \<= 0 || local\_slots\[k\].qty \== 0 {

            local\_slots\[k\].item\_id \= 0;

            local\_slots\[k\].qty \= 0;

        }

        if local\_slots\[k\].item\_id \> 0 {

            if let Some(item) \= item\_db.get\_by\_id(local\_slots\[k\].item\_id as u16) {

                mat\_rat\_a\[k\] \= item.rat\_a;  // item \+ 0x32

                rank\_rat\_b\[k\] \= item.rat\_b; // item \+ 0x33

            } else {

                local\_slots\[k\].item\_id \= 0;

                local\_slots\[k\].qty \= 0;

                active\_count \-= 1;

            }

        } else {

            active\_count \-= 1;

        }

    }

    // \--- BƯỚC 2: Tính bậc phẩm chất mục tiêu (target\_rank / local\_5c) \---

    let sum\_rat\_b: u16 \= rank\_rat\_b.iter().map(|\&x| x as u16).sum();

    // ASM 0x0079A994..0x0079A9AC: FILD sum\_rat\_b; FDIV active\_count; CALL @ROUND

    let mut avg\_rank \= delphi\_round((sum\_rat\_b as f64) / (active\_count as f64)) as i32;

    if avg\_rank \<= 0 {

        avg\_rank \= 1;

    }

    // ASM 0x0079A9BA..0x0079A9D5: Random(avg\_rank \+ 1\) \+ 1, kẹp trần 10

    let mut target\_rank \= (rng.random\_int(avg\_rank \+ 1\) \+ 1\) as u8;

    if target\_rank \== 11 {

        target\_rank \= 10;

    }

    // \--- BƯỚC 3: Sắp xếp cặp (mat\_rat\_a\[k\], qty\[k\]) tăng dần theo mat\_rat\_a \---

    let mut sorted\_mat \= mat\_rat\_a;

    let mut sorted\_qty \= \[local\_slots\[0\].qty, local\_slots\[1\].qty, local\_slots\[2\].qty\];

    for i in 0..3 {

        for j in (i \+ 1)..3 {

            if sorted\_mat\[j\] \< sorted\_mat\[i\] {

                sorted\_mat.swap(i, j);

                sorted\_qty.swap(i, j);

            }

        }

    }

    // Tạo khóa tra cứu u32 bằng ghép chuỗi thập phân

    let key\_str \= format\!("{}{}{}", sorted\_mat\[0\], sorted\_mat\[1\], sorted\_mat\[2\]);

    let search\_key: u32 \= key\_str.parse::\<u32\>().unwrap();

    // \--- BƯỚC 4: Tính số lượng thành phẩm (out\_qty) và bậc nguyên liệu input\_deg\[0..2\] \---

    let orig\_qty \= sorted\_qty; // \[EBP \- 0x56 .. \-0x54\]

    let mut asc\_qty \= sorted\_qty;

    for i in 0..3 {

        for j in (i \+ 1)..3 {

            if asc\_qty\[j\] \< asc\_qty\[i\] {

                asc\_qty.swap(i, j);

            }

        }

    }

    let mut min\_qty: u8 \= 0;

    for \&q in \&asc\_qty {

        if q \> 0 {

            min\_qty \= q;

            break;

        }

    }

    if min\_qty \== 0 {

        min\_qty \= 1;

    }

    let mut out\_qty: u8 \= min\_qty; // \[EBP \- 0x57\] (local\_5b)

    let mut input\_deg \= \[0u8; 3\]; // \[EBP \- 0x53 .. \-0x51\]

    for k in 0..3 {

        let mut d \= orig\_qty\[k\] / min\_qty;

        if d \> 5 {

            d \= 5;

        }

        input\_deg\[k\] \= d;

    }

    // \--- BƯỚC 5: Quét bảng thứ cấp TFCompound (đã sắp xếp theo key) tìm tối đa 10 ứng viên \---

    let mut recipe\_candidates \= \[0u16; 10\]; // auStack\_80 (\[EBP \- 0x7C\])

    let mut cand\_count: usize \= 0;          // local\_40 (\[EBP \- 0x3C\])

    for rec in \&compound\_db.sorted\_table {

        if rec.key \> search\_key {

            break; // Bảng đã sort tăng dần theo key \-\> thoát sớm (JA 0x0079B036)

        }

        if rec.key \== search\_key {

            let mut slot\_matched \= false;

            for k in 0..3 {

                slot\_matched \= false;

                if k \== 0 && input\_deg\[0\] \== 0 {

                    slot\_matched \= true;

                } else if input\_deg\[k\] \>= 1 {

                    let mut last\_d \= 0u8;

                    for d in (1..=input\_deg\[k\]).rev() {

                        last\_d \= d;

                        if rec.deg\[k\] \== d {

                            slot\_matched \= true;

                            break;

                        }

                    }

                    if last\_d \== 1 && \!slot\_matched {

                        break; // Không khớp slot k \-\> dừng kiểm tra record này

                    }

                }

            }

            if slot\_matched {

                recipe\_candidates\[cand\_count\] \= rec.result\_kind;

                cand\_count \+= 1;

                if cand\_count \> 10 {

                    // Lưu ý ASM 0x0079AFC1: INC byte \[EBP-0x3C\]; CMP byte \[EBP-0x3C\], 0xA; JA break

                    // Khi cand\_count tăng lên 10 vẫn chưa JA, nhưng mảng auStack\_80 chỉ có 10 phần tử.

                    break;

                }

            }

        }

    }

    // \--- BƯỚC 6: Cơ chế Fallback khi không khớp công thức nào (cand\_count \== 0\) \---

    if cand\_count \== 0 {

        let mut valid\_input\_ids \= \[0u16; 3\];

        let mut valid\_n: usize \= 0;

        for k in 0..3 {

            if local\_slots\[k\].item\_id \> 0 {

                valid\_input\_ids\[valid\_n\] \= local\_slots\[k\].item\_id as u16;

                valid\_n \+= 1;

            }

        }

        let pick \= rng.random\_int(valid\_n as i32) as usize;

        let chosen\_input\_id \= valid\_input\_ids\[pick\];

        let chosen\_item \= item\_db.get\_by\_id(chosen\_input\_id).unwrap();

        recipe\_candidates\[0\] \= chosen\_item.kind as u16; // Lấy item \+ 0x19 làm result\_kind\!

        cand\_count \= 1;

        target\_rank \= 1; // Ép bậc phẩm chất về 1\!

    }

    // \--- BƯỚC 7: Chọn ngẫu nhiên 1 result\_kind từ danh sách ứng viên công thức \---

    let r1 \= rng.random\_int(100) as u8; // 0..99

    let step1 \= delphi\_round(100.0 / (cand\_count as f64)) as u8;

    let mut idx1 \= (r1 / step1) as usize;

    if idx1 \>= cand\_count {

        idx1 \= cand\_count \- 1;

    }

    let chosen\_kind \= recipe\_candidates\[idx1\];

    // Kiểm tra loại kết quả có bị cấm không ({20, 21, 22})

    if (20..=22).contains(\&chosen\_kind) {

        return Err(2); // ": 合成資料庫錯誤,合成出不允許的類別"

    }

    // Nếu loại kết quả là Vũ khí / Trang bị (1..14), ép số lượng thành phẩm \= 1

    if (1..=14).contains(\&chosen\_kind) {

        out\_qty \= 1;

    }

    // \--- BƯỚC 8: Quét bảng TFItem tìm tối đa 10 vật phẩm có (kind \== chosen\_kind && ratB \== target\_rank) \---

    let mut item\_candidates \= \[0u16; 10\]; // auStack\_94 (\[EBP \- 0x90\])

    let mut item\_cand\_count: usize \= 0;

    for item in \&item\_db.records {

        if (item.kind as u16) \== chosen\_kind && item.rat\_b \== target\_rank {

            item\_candidates\[item\_cand\_count\] \= item.id; // item \+ 0x1A

            item\_cand\_count \+= 1;

            if item\_cand\_count \>= 10 {

                break;

            }

        }

    }

    if item\_cand\_count \== 0 {

        return Err(3); // ": 物品資料庫無此合成物品"

    }

    // \--- BƯỚC 9: Chọn ngẫu nhiên 1 item\_id từ danh sách ứng viên vật phẩm \---

    let r2 \= rng.random\_int(100) as u8;

    let step2 \= delphi\_round(100.0 / (item\_cand\_count as f64)) as u8;

    let mut idx2 \= (r2 / step2) as usize;

    if idx2 \>= item\_cand\_count {

        idx2 \= item\_cand\_count \- 1;

    }

    Ok((item\_candidates\[idx2\], out\_qty))

}

---

## 10\. Công Thức Tỷ Lệ & Xác Suất Thành Công (Dạng Toán Học)

### 10.1. Công thức xác suất bậc phẩm chất (`target_rank` / `ratB`) `[ĐÃ XÁC MINH]`

Gọi \$N \\in {2, 3}\$ là số ô nguyên liệu hợp lệ, và \$b\_1, \\dots, b\_N \\in {1, \\dots, 10}\$ là giá trị trường `ratB` (`item + 0x33`) của từng nguyên liệu.

1. **Bậc phẩm chất trung bình làm tròn chẵn (`ROUND` — Banker's Rounding của x87 FPU)**: \$\$R\_{\\text{avg}} \= \\max\\left(1, \\text{ROUND}\\left(\\frac{1}{N} \\sum\_{k=1}^{N} b\_k\\right)\\right)\$\$ *(Trong đó \$\\text{ROUND}(x)\$ làm tròn tới số nguyên gần nhất; nếu phần thập phân đúng bằng \$0{,}5\$ như \$2{,}5\$ hoặc \$3{,}5\$, lệnh `FISTP` làm tròn về **số nguyên chẵn** gần nhất: \$\\text{ROUND}(2{,}5) \= 2\$, \$\\text{ROUND}(3{,}5) \= 4\$).*  
     
2. **Phân phối xác suất của bậc phẩm chất đầu ra \$R\_{\\text{target}}\$**: Biến ngẫu nhiên \$U \\sim \\text{Uniform}{0, 1, \\dots, R\_{\\text{avg}}}\$ được sinh bởi `Random(R_avg + 1)` (gồm \$R\_{\\text{avg}} \+ 1\$ giá trị đồng khả năng), sau đó: \$\$R\_{\\text{target}} \= \\min(10, U \+ 1\) \\in {1, 2, \\dots, \\min(10, R\_{\\text{avg}} \+ 1)}\$\$  
     
   - Khi \$R\_{\\text{avg}} \< 10\$: \$\$\\mathbb{P}(R\_{\\text{target}} \= r) \= \\frac{1}{R\_{\\text{avg}} \+ 1}, \\quad \\forall r \\in {1, 2, \\dots, R\_{\\text{avg}} \+ 1}\$\$  
     - **Xác suất "Thăng cấp" (vượt bậc trung bình, \$R\_{\\text{target}} \= R\_{\\text{avg}} \+ 1\$)**: \$\$\\mathbb{P}(\\text{Upgrade}) \= \\frac{1}{R\_{\\text{avg}} \+ 1}\$\$  
     - **Xác suất "Giữ nguyên bậc trung bình" (\$R\_{\\text{target}} \= R\_{\\text{avg}}\$)**: \$\$\\mathbb{P}(\\text{Keep}) \= \\frac{1}{R\_{\\text{avg}} \+ 1}\$\$  
     - **Xác suất "Tụt bậc" (\$R\_{\\text{target}} \< R\_{\\text{avg}}\$)**: \$\$\\mathbb{P}(\\text{Downgrade}) \= \\frac{R\_{\\text{avg}} \- 1}{R\_{\\text{avg}} \+ 1}\$\$  
   - Khi \$R\_{\\text{avg}} \= 10\$: vì cả \$U \= 9\$ (\$U+1=10\$) và \$U \= 10\$ (\$\\min(10, 11\) \= 10\$) đều cho \$R\_{\\text{target}} \= 10\$, nên \$\\mathbb{P}(R\_{\\text{target}} \= 10\) \= \\frac{2}{11}\$, còn mỗi bậc \$r \\in {1, \\dots, 9}\$ có xác suất \$\\frac{1}{11}\$.

### 10.2. Công thức chọn ứng viên khi có \$M\$ công thức hoặc \$K\$ vật phẩm cùng khớp `[ĐÃ XÁC MINH]`

Tại `0079a890_FUN_0079a890.asm.txt:426-449` và `544-568`, để chọn 1 phần tử từ \$M \\in {1, \\dots, 10}\$ ứng viên (đánh chỉ số \$0 \\dots M-1\$):

1. Quay số \$r \\sim \\text{Uniform}{0, 1, \\dots, 99}\$ (`Random(100)`).  
2. Tính bước nhảy: \$s \= \\text{ROUND}(100{,}0 / M)\$ (hằng số `float32` tại `0x0079B4B4` là `100.0f`).  
3. Chỉ số được chọn: \$\$\\text{idx} \= \\min\\left(M \- 1, \\left\\lfloor \\frac{r}{s} \\right\\rfloor\\right)\$\$

>   
> **Hệ quả lệch xác suất ở ứng viên cuối (\$M-1\$) do làm tròn `step`**:  
> 

> - Nếu \$M \= 3\$: \$s \= \\text{ROUND}(33{,}333\\dots) \= 33\$. Khi đó:  
>   - \$\\text{idx} \= 0\$ khi \$r \\in \[0, 32\]\$ (\$33%\$);  
>   - \$\\text{idx} \= 1\$ khi \$r \\in \[33, 65\]\$ (\$33%\$);  
>   - \$\\text{idx} \= 2\$ khi \$r \\in \[66, 98\]\$ **và** \$r \= 99\$ (\$\\lfloor 99/33 \\rfloor \= 3 \\to \\text{kẹp về } 2\$), tổng cộng **\$34%\$**\!  
> - Nếu \$M \= 6\$: \$s \= \\text{ROUND}(16{,}666\\dots) \= 17\$. Các chỉ số \$0, 1, 2, 3, 4\$ mỗi chỉ số chiếm \$17\$ giá trị (\$17% \\times 5 \= 85%\$), còn chỉ số cuối \$\\text{idx} \= 5\$ chỉ nhận \$r \\in \[85, 99\]\$ tức **\$15%\$**\! Server muốn tái tạo **chính xác 100% phân phối của client** phải giữ nguyên phép chia nguyên cho `ROUND(100.0 / M)` thay vì dùng `Random(M)`.

### 10.3. Ví dụ số cụ thể từng bước `[ĐÃ XÁC MINH]`

Giả sử người chơi đặt vào 3 ô nguyên liệu:

- Ô 1: `item_id = 10001`, `qty = 4`, có `ratA = 1`, `ratB = 2`.  
- Ô 2: `item_id = 17001`, `qty = 2`, có `ratA = 2`, `ratB = 3`.  
- Ô 3: Bỏ trống (`item_id = 0`, `qty = 0`).  
1. **Bước Validate (`FUN_0079A5E4`)**: Có \$N \= 2 \\ge 2\$ ô hợp lệ, ID thuộc `[10001, 50000]`, `qty <= 50`, `kind` không thuộc `{20, 21, 22}` \$\\rightarrow\$ Trả về `0` (OK).  
2. **Bước Tính `target_rank`**:  
   - Tổng `ratB`: \$S \= 2 \+ 3 \+ 0 \= 5\$, số ô hoạt động \$N \= 2\$.  
   - Trung bình: \$5 / 2 \= 2{,}5\$. Theo luật làm tròn chẵn (`FISTP` của `@ROUND`), \$\\text{ROUND}(2{,}5) \= 2 \\Rightarrow R\_{\\text{avg}} \= 2\$.  
   - Quay `Random(2 + 1) + 1 = Random(3) + 1` \$\\in {1, 2, 3}\$, mỗi bậc có xác suất \$\\frac{1}{3} \\approx 33{,}33%\$. Giả sử quay được \$R\_{\\text{target}} \= 3\$ (thăng cấp).  
3. **Bước Tạo khóa `u32` và `input_deg`**:  
   - Mảng `(ratA, qty)` ban đầu: `[(1, 4), (2, 2), (0, 0)]`.  
   - Sau khi sắp xếp tăng dần theo `ratA`: `sorted_mat = [0, 1, 2]`, `sorted_qty = [0, 4, 2]`.  
   - Khóa `search_key`: `StrToInt("0" + "1" + "2") = StrToInt("012") = 12`.  
   - Số lượng nhỏ nhất khác 0: `min_qty = 2` \$\\Rightarrow\$ `out_qty = 2`.  
   - Bậc tỷ lệ đầu vào: `input_deg = [0/2, 4/2, 2/2] = [0, 2, 1]`.  
4. **Bước Tra bảng `Compound.Dat`**:  
   - Khóa `12` khớp với **Record \#2** (`key = 12`, `result_kind = 1`, `deg = [0, 1, 1]`) và các record cùng khóa `12` (`result_kind = 2, 3, 4, 5, 6, 7, 8, 9, 10...` trong `compound_dat_decoded_sample.txt`).  
   - Kiểm tra `degree`: với `k = 0`, `input_deg[0] == 0` (khớp); với `k = 1`, `recipe.deg[1] = 1 <= input_deg[1] = 2` (khớp); với `k = 2`, `recipe.deg[2] = 1 <= input_deg[2] = 1` (khớp).  
   - Thu thập 10 ứng viên `result_kind` đầu tiên có khóa `12` vào `auStack_80`, chọn ngẫu nhiên 1 loại (ví dụ `chosen_kind = 1` — Vũ khí/Trang bị loại 1).  
   - Vì `chosen_kind = 1` thuộc `1..14`, `out_qty` bị ép từ `2` về **`1`**.  
5. **Bước Quét `TFItem`**: Tìm tối đa 10 vật phẩm có `kind == 1` và `ratB == 3`, rồi chọn ngẫu nhiên 1 vật phẩm trả về `(out_item_id, 1)`.

---

## 11\. Bảng Message & Chuỗi Giao Diện (Big5 / Hán-Việt / Tiếng Việt)

Toàn bộ các hằng chuỗi dưới đây nằm liền kề sau mã máy của Constructor `0x0079EE08` (`hex_79f31b_79f3c4_ui_strings.txt`) và Handler `0x0079F3C8` (`hex_79f8bc_79fa40_msg_strings.txt`) `[ĐÃ XÁC MINH]`:

| Địa chỉ VA | Độ dài | Hex gốc (Big5 / ASCII) | Chuỗi giải mã (cp950) | Phiên âm Hán-Việt | Nghĩa tiếng Việt & Ngữ cảnh sử dụng |
| :---: | :---: | :---- | :---- | :---- | :---- |
| `0x0079F338` | 5 | `31 30 30 30 31` | `10001` | — | Giá trị mặc định của ô `TSe_Editor` 1 (`+0x134`) và 3 (`+0x13C`) |
| `0x0079F348` | 5 | `31 37 30 30 31` | `17001` | — | Giá trị mặc định của ô `TSe_Editor` 2 (`+0x138`) |
| `0x0079F358` | 1 | `31` | `1` | — | Giá trị mặc định của 3 ô số lượng `TSe_Editor` 4, 5, 6 (`+0x140..+0x148`) |
| `0x0079F3C0` | 4 | `A6 X4 A6 X8` (`a6 58 a6 a8`) | `合成` | Hợp thành | Nhãn (Caption) của nút bấm `TSe_FixedButton` kích hoạt `0x0079F3C8` |
| `0x0079F8C4` | 18 | `3A 20 A6 58 A6 A8 A6 DC A4 D6 AC 6E A4 47 B9 CBl AB ~` | `: 合成至少要二樣物品` | Hợp thành chí thiểu yếu nhị dạng vật phẩm | Báo lỗi যখন `FUN_0079A5E4 == 1`: Hợp thành cần ít nhất 2 vật phẩm |
| `0x0079F8E0` | 20 | `3A 20 B0 D1 C2 F8 A6 B3 A4 A3 AF E0 A6 58 A6 A8 AA BA AB ~` | `: 參雜有不能合成的物品` | Tham tạp hữu bất năng hợp thành đích vật phẩm | Báo lỗi khi `FUN_0079A5E4 == 2`: Có lẫn vật phẩm không thể hợp thành (ID ngoài `10001..50000`, không tồn tại, hoặc `kind ∈ {20,21,22}`) |
| `0x0079F900` | 24 | `3A 20 B3 E6 B6 B5 AB ~ C9 6E B6 q A4 A3 A5 i B6 W B9 L 35 30` | `: 單項物品數量不可超過50` | Đơn hạng vật phẩm số lượng bất khả siêu quá 50 | Báo lỗi khi `FUN_0079A5E4 == 3`: Số lượng một vật phẩm đơn lẻ không được vượt quá 50 |
| `0x0079F920` | 20 | `3A 20 A6 58 A6 A8 A5 CB B1 6F AD P B5 L A6 B9 A6 58 A6 A8` | `: 合成失敗導致無此合成` | Hợp thành thất bại đạo trí vô thử hợp thành | Chuỗi cảnh báo dự phòng (không được tham chiếu trực tiếp trong `0x0079F3C8`) |
| `0x0079F93C` | 17 | `4B 65 79 20 20 49 64 20 20 44 31 20 20 44 32 20 20 44 33` | `Key  Id  D1  D2  D3` | — | Tiêu đề bảng dump `Compound.Dat` trong hộp phải (`Self + 0x150`) |
| `0x0079F968` | 20 | `4D 61 74 65 20 20 6B 69 6E 64 20 20 72 61 74 41 20 20 72 61 74 42` | `Mate  kind  ratA  ratB` | — | Tiêu đề bảng dump thông số 3 nguyên liệu trong hộp phải (`Self + 0x150`) |
| `0x0079F988` | 20 | `3A 20 A6 58 A6 A8 B8 EA AE C6 AE w B5 L A6 B9 A6 58 A6 A8 AA k` | `: 合成資料庫無此合成法` | Hợp thành tư liệu khố vô thử hợp thành pháp | Nhánh `FUN_0079A890 == 1`: DB hợp thành không có công thức này (**Unreachable / Dead code** do đã có fallback) |
| `0x0079F9A4` | 32 | `3A 20 A6 58 A6 A8 B8 EA AE C6 AE w BF F9 BB ~ 2C A6 58 A6 A8 A5 X A4 A3 A4 B9 B3 \ AA BA C3 FE A7 O` | `: 合成資料庫錯誤,合成出不允許的類別` | Hợp thành tư liệu khố sai ngộ, hợp thành xuất bất doãn hứa đích loại biệt | Nhánh `FUN_0079A890 == 2`: Lỗi DB hợp thành, tạo ra loại vật phẩm không được phép (`result_kind ∈ {20,21,22}`) |
| `0x0079F9CC` | 20 | `3A 20 AB ~ B8 EA AE C6 AE w B5 L A6 B9 A6 58 A6 A8 AB ~` | `: 物品資料庫無此合成物品` | Vật phẩm tư liệu khố vô thử hợp thành vật phẩm | Nhánh `FUN_0079A890 == 3`: DB vật phẩm (`Item.dat`) không có vật phẩm nào khớp `(result_kind, target_rank)` |
| `0x0079F9EC` | 15 | `3A 20 A6 58 A6 A8 A5 X AA BA AB ~ 20 49 64 3A 20` | `: 合成出的物品 Id:`  | Hợp thành xuất đích vật phẩm Id: | Tiền tố in kết quả thành công vào hộp trái (`Self + 0x14C`) |
| `0x0079FA04` | 5 | `20 4E 75 6D 3A` |  `Num:` | — | Hậu tố in số lượng thành phẩm (`out_qty`) khi hợp thành thành công |

---

## 12\. Phân Loại Tính Năng Người Dùng: Mix Item, Mix Khoáng & Cường Hóa Lông/Tâm

1. **"Mix khoáng / quặng (Mineral / Ore / Quartz)" vs "Mix item với item"** `[ĐÃ XÁC MINH]`:  
     
   - **Về mặt code**: Client **KHÔNG có nhánh code hay thuật toán riêng** để phân biệt "mix khoáng với khoáng" và "mix trang bị với trang bị". Cả hai đều chạy qua **duy nhất** một hàm `FUN_0079A890` và tra cứu trên cùng một bảng `Compound.Dat`\!  
   - **Vì sao cùng một hàm lại xử lý được cả hai?**:  
     - Mỗi loại khoáng thạch (Kim, Mộc, Thủy, Hỏa, Thổ, Thiết, Đồng, Bạc, Vàng, Ngọc...) trong `Item.dat` được gán một mã chất liệu `ratA` (`item + 0x32`, thuộc `1..37`) và một cấp độ khoáng `ratB` (`item + 0x33`, thuộc `1..10`).  
     - Khi người chơi ghép 2 hoặc 3 cục khoáng cùng loại hoặc khác loại, `FUN_0079A890` lấy trung bình `ratB` để quay xác suất lên khoáng cấp cao hơn (\$R\_{\\text{target}} \= R\_{\\text{avg}} \+ 1\$ với xác suất \$\\frac{1}{R\_{\\text{avg}}+1}\$), đồng thời tra khóa `u32` trong `Compound.Dat` để biết tổ hợp chất liệu đó tạo ra `result_kind` là khoáng hay trang bị\!  
     - Điểm khác biệt duy nhất trong `FUN_0079A890` nằm ở dòng `0x0079B13D` (`0079a890_FUN_0079a890.asm.txt:458-462`): nếu `result_kind ∈ 1..14` (trang bị/vũ khí), số lượng đầu ra `out_qty` bị ép về `1`; nếu `result_kind >= 15` (khoáng thạch, vật liệu, thuốc...), `out_qty` giữ nguyên bằng `min_qty` (cho phép mix hàng loạt tối đa 50 viên khoáng cùng lúc\!).  
   - **Về chuỗi `"挖礦"` (Đào khoáng) tại `0x005FC720` trong `FUN_005FAA80`** (`005faa80_FUN_005faa80.c`): Đây là hàm khởi tạo danh sách tên các **kỹ năng thu thập / chế tạo của nhân vật** (Thuật Đào khoáng `挖礦`, Thuật Hợp thành `合成` tại `0x005FC708`, Thuật Tu luyện `修練`, v.v.), dùng để hiển thị tên kỹ năng chứ không chứa logic tính toán mix khoáng.

   

2. **"Mix lông / tâm (Feather / Heart) vào trang bị để cường hóa thuộc tính"** `[ĐÃ XÁC MINH]` & `[SUY LUẬN]`:  
     
   - **Không nằm trong `Compound.Dat` / `FUN_0079A890`** `[ĐÃ XÁC MINH]`: Trong `FUN_0079A890`, kết quả đầu ra chỉ là một `item_id` tĩnh lấy từ `Item.dat` (`item + 0x1A`) và `out_qty`, hoàn toàn không có phép cộng thuộc tính nguyên tố (`elemType` / `elemVal`) hay sửa đổi chỉ số động của vật phẩm.  
   - **Nằm ở các lớp/form riêng biệt trong client** `[ĐÃ XÁC MINH]`:  
     - Trong Nhóm G8 (`004ca580_FUN_004ca580.c`, `004ca43c_FUN_004ca43c.c`, `004dbaa0_FUN_004dbaa0.c`), hai lớp `TFWeapBody` (VMT `0x004CA3A0`, đọc `\Data\WeapBody.dat`) và `TFSuit` (VMT `0x004DBA08`, đọc `\Data\Suit.dat`) chỉ quản lý **dữ liệu hình ảnh / bảng màu sprite hiển thị trên nhân vật** (vũ khí và trang phục), không phải logic mix lông/tâm.  
     - Ngược lại, tính năng khảm/cường hóa Lông (`羽毛` \- Feather) vào trang bị có một Form UI chuyên biệt trong client là **`TFT_SelectFeatherForm`** (VMT `0x0058A3EC`, constructor `0x005B0BFC`, được liệt kê trong `README.md`). Khi người chơi dùng Lông/Tâm trong gameplay thực tế, thao tác này được gửi lên server qua gói tin riêng chứ không đi qua `TFCompound` `[SUY LUẬN — cần disassemble thêm constructor 0x005B0BFC của TFT_SelectFeatherForm nếu muốn lấy opcode khảm lông]`.

---

## 13\. Giao Thức Mạng (C→S / S→C) & Phân Định Trách Nhiệm Client – Server

### 13.1. Tại sao Handler `0x0079F3C8` không gửi bất kỳ gói tin mạng nào? `[ĐÃ XÁC MINH]`

Kiểm tra `0079f3c8_mix_handler.calls.txt` cho thấy toàn bộ 11 hàm được gọi bên trong `0x0079F3C8` chỉ gồm:

- Xử lý chuỗi & số RTL: `0x004099E4` (`StrToInt`), `0x00409980` (`IntToStr`), `0x00403E98` (`@LStrAsg`), `0x00404148` (`@LStrCatN`), `0x00403BD0` (`@LStrClr`), `0x00404518` (`@HandleFinally`), `0x00402FA8` (`@_BoundErr`).  
- Nghiệp vụ & DB cục bộ: `0x0079A5E4` (validate), `0x0079A890` (tra cứu & tính kết quả), `0x007746AC` (tra `TFItem`).  
- Cập nhật UI: `0x007BEA20` (`TSe_SelectText.AddLine`), `0x007BD3BC` (`TSe_SelectText.Refresh`).

**Kết luận**: Form `0x0079EE08` (với các ô nhập `item_id` thô mặc định `"10001"`, `"17001"` và bảng dump `"Key  Id  D1  D2  D3"`, `"Mate  kind  ratA  ratB"`) là **công cụ kiểm thử hợp thành nội bộ (Developer Mix Simulator)** của đội ngũ phát triển Chinesegamer để test bảng `Compound.Dat` và `Item.dat` ngay trên client. Trong chế độ simulator này, client tự chạy `FUN_0079A5E4` và `FUN_0079A890` hoàn toàn offline.

### 13.2. Giao thức C→S và S→C khi người chơi Mix đồ trong Gameplay thực tế `[ĐÃ XÁC MINH]`

Đối chiếu với bảng giao thức `OPCODE_MAP.csv`, `05_special_economy.md` và `07_rust_economy_audit.md`:

| Chiều | Action Code (AC) | Sub-code | Tên nhóm giao thức | Vai trò trong luồng Mix đồ thực tế |
| :---: | :---: | :---: | :---- | :---- |
| **C → S** | **`AC 29`** (`0x1D`) | `sub 1..N` | `Compound` | Client gửi danh sách các slot hòm đồ (`bag_slot`, `qty`) mà người chơi bỏ vào giao diện Thuật Hợp Thành (`合成`) lên Server để yêu cầu thực hiện mix. |
| **S → C** | **`AC 29`** (`0x1D`) | `sub 1..N` | `Compound` | Server phản hồi trạng thái đóng/mở giao diện hợp thành hoặc kết quả hiệu ứng hợp thành (thành công / thất bại). |
| **S → C** | **`AC 17`** (`0x11`) | `sub 3, 4, 8, 9` | `Inventory` (Remove / Add Item) | Server trừ các nguyên liệu đã tiêu hao khỏi túi đồ của người chơi và thêm vật phẩm kết quả `(out_item_id, out_qty)` vào túi đồ. |
| **C ↔ S** | **`AC 01`** (`0x01`) | `sub 9` (`0x09`) | `Selection` (`Send_Select9`) | Dùng khi hợp thành qua NPC Thợ Rèn / NPC Chế Tạo trong thành (người chơi chọn phương án chế tạo trong hội thoại NPC). |

### 13.3. Phân định rõ: Phần nào Client tính vs Phần nào Server bắt buộc phải tính `[ĐÃ XÁC MINH]`

| Thành phần xử lý | Trong Dev Simulator (`0x0079EE08`) | Trong Gameplay thực tế (Production) | Lý do kỹ thuật & Bảo mật |
| :---- | :---: | :---: | :---- |
| Kiểm tra số lượng nguyên liệu \$\\ge 2\$, `qty <= 50`, `kind ∉ {20,21,22}` (`FUN_0079A5E4`) | Client tự kiểm tra | **Server bắt buộc kiểm tra lại 100%** (Client chỉ chặn sơ bộ ở UI) | Chống packet injection gửi `item_id` cấm hoặc số lượng âm/vượt quá số lượng thực có trong túi đồ. |
| Giải mã `Compound.Dat` & dựng bảng tra khóa `u32` (`FUN_005F2528`) | Client nạp vào `TFCompound` | **Server nạp vào RAM khi khởi động** | Server cần bảng công thức để quyết định loại vật phẩm đầu ra (`result_kind`). |
| Tính bậc phẩm chất `target_rank` (`ROUND` \+ `Random(R_avg + 1) + 1`) | Client tự quay số | **Server bắt buộc phải tính hoàn toàn** | Nếu để client quyết định `Random`, người chơi có thể hack client để luôn đạt xác suất thăng cấp \$R\_{\\text{avg}} \+ 1\$. |
| Tra cứu ứng viên `result_kind`, xử lý Fallback, và quét `Item.dat` chọn `out_item_id` (`FUN_0079A890`) | Client tự chọn | **Server bắt buộc phải tính hoàn toàn** | Server giữ quyền tối cao sinh vật phẩm mới và cập nhật túi đồ (`AC 17`). |

---

## 14\. Đối Chiếu Với Server Rust (`src/data/loaders/compound.rs`, `item.rs`, `reader.rs`)

Đối chiếu giữa logic thực tế của Client vừa giải phẫu với thiết kế hiện tại của Server Rust (tham chiếu qua `PROMPT.md`, `compound_dat_decoded_sample.txt` và `items.json`):

| Hạng mục đối chiếu | Phía Client Delphi (`aLogin.exe`) | Phía Server Rust hiện tại (`src/data/...`) | Đánh giá & Chỗ lệch cần sửa ngay |
| :---- | :---- | :---- | :---- |
| **1\. Giải mã `Compound.Dat`** | `FUN_005F2DD0`: `u16 = (raw ^ 0xFBBC) - 3`, `u8 = (raw ^ 0xD3) - 3`. | `DatReader::with_keys(bytes, 3, 211, 64444, 0)` trong `src/data/reader.rs`. | **KHỚP 100%**. (`0xFBBC == 64444`, `0xD3 == 211`, `offset == 3`). |
| **2\. Đọc Header & Số lượng Record** | Đọc `1.394` slot × `8` byte; kiểm tra `slot[0].u16 == 2`, giải mã `1.393` slot từ `1..1393`. | Bỏ qua 8 byte header đầu file, đọc `1.393` record 8 byte tiếp theo. | **KHỚP VỀ DỮ LIỆU**. Không có lỗi mất record cuối như ghi chú cũ từng nghi ngờ. |
| **3\. Ngữ nghĩa trường `result_id` và `matA/B/C` trong `compound.rs`** | • `result_id` (`1..52`) là **`item.kind` (`item + 0x19`)**. • `matA/B/C` (`0..37`) là **`item.ratA` (`item + 0x32`)**. | Nhiều tài liệu/struct cũ đặt tên là `result_item_id` hoặc `material_item_id` và tưởng nhầm là `item_id` hoặc `item.kind`. | **LỆCH NGHIÊM TRỌNG NẾU DÙNG NHẦM TRƯỜNG**: Trong `compound.rs`, phải đổi tên rõ thành `result_kind: u16` và `materials: [u8; 3]` (so khớp với `item.rat_a`, tuyệt đối không so với `item.id` hay `item.kind`). |
| **4\. Trường `ratA` (`+0x32`) và `ratB` (`+0x33`) trong `item.rs` / `items.json`** | `FUN_00774BA8` giải mã hai byte tại offset `+0x32` và `+0x33` của mỗi record `370` byte trong `Item.dat`. | File `items.json` hiện chỉ xuất: `id`, `name`, `type`, `level`, `equippos`, `props`, `elemType`, `elemVal`, `sellPrice`, `buyPrice` — **hoàn toàn thiếu `ratA` (`+0x32`) và `ratB` (`+0x33`)\!** | **THIẾU DỮ LIỆU BẮT BUỘC**: Nếu `src/data/loaders/item.rs` chưa parse 2 byte tại offset `+0x32` (`rat_a: u8`) và `+0x33` (`rat_b: u8`) của bản ghi `Item.dat` (370 byte), server **không thể** tạo khóa tra cứu `Compound.Dat` và **không thể** tính `target_rank` hay lọc vật phẩm đầu ra\! |
| **5\. Cơ chế Fallback khi không khớp công thức** | Không bao giờ báo lỗi "không có công thức": tự động lấy `kind` của 1 nguyên liệu ngẫu nhiên và ép `target_rank = 1`. | Thường trả về `None` / báo lỗi thất bại nếu không tìm thấy recipe trong `Compound.Dat`. | **LỆCH HÀNH VI GAMEPLAY**: Trong TS Online gốc, ghép bừa 2–3 món không có trong bảng công thức vẫn ra một món đồ cấp 1 (`ratB = 1`) thuộc cùng loại (`kind`) với một trong các nguyên liệu đầu vào\! |

---

## 15\. Khoảng Trống & Đề Xuất Cho Kỹ Sư Server

1. **Bổ sung ngay 2 trường `rat_a` (`+0x32`) và `rat_b` (`+0x33`) vào `src/data/loaders/item.rs`**:  
   - Trong `00774ba8_FUN_00774ba8.asm.txt:154-169`, hai byte tại offset `+0x32` và `+0x33` của mỗi record `Item.dat` (kích thước `370` byte, tính từ đầu record sau khi trừ offset `+0x4` của `TFItem`) được giải mã bằng công thức chuẩn của `Item.dat`: `decoded_u8 = ((raw_u8 ^ 0x9A) - 9)`.  
   - Hãy trích xuất thêm `rat_a` và `rat_b` từ `Item.dat` vào struct `ItemData` của Rust (và xuất lại `items.json` nếu server đọc từ JSON).  
2. **Xác minh cấu trúc payload của gói tin `AC 29` (`Compound`) trong Gameplay thực tế**:  
   - Như đã chứng minh ở Mục 13, form `0x0079EE08` là Dev Simulator offline. Để nối hàm `execute_compound_lookup` (Mục 9.3) vào handler mạng của server Rust (`src/protocol/mod.rs`), cần đối chiếu handler gửi `AC 29` của client để biết chính xác thứ tự byte của gói C→S (thường gồm danh sách cặp `(bag_slot: u8, qty: u8)`), sau đó server ánh xạ `bag_slot -> item_id` từ túi đồ thật của nhân vật rồi truyền vào `validate_mix_inputs` và `execute_compound_lookup`.  
3. **Kiểm chứng thêm `Compound2.Dat` (`FUN_005F3068` @ `0x005F3068`)**:  
   - Trong Nhóm G1 có hàm `FUN_005F3068` đọc file `\Data\Compound2.Dat` gồm 15 record × 15 byte (ma trận \$15 \\times 15\$, giải mã `((raw ^ 0xFD) - 5)`). Bảng này **không** được gọi bởi `FUN_0079A890`, nhưng rất có thể được dùng cho một cơ chế phụ (ví dụ: bảng tương khắc hoặc ma trận kết hợp giữa 14 loại trang bị `kind 1..14`). Cần kiểm tra xem `Compound2.Dat` có tồn tại trong thư mục `\Data\` của bản client mục tiêu hay không.

---

## 16\. Phụ Lục: Bảng Hằng Số & Cấu Trúc Record

### 16.1. Bảng hằng số giải mã các file `.Dat` liên quan `[ĐÃ XÁC MINH]`

| Tên file `.Dat` | Hàm giải mã (VA) | Kích thước Record | Header Slot 0 | `number_offset` | `u8_key` | `u16_key` | `u32_key` |
| :---- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `\Data\Compound.Dat` | `0x005F2DD0` | `8` byte | `u16 == 2` | **`3`** | **`0xD3`** (`211`) | **`0xFBBC`** (`64444`) | `0` |
| `\Data\Compound2.Dat` | `0x005F3068` | `15` byte | `u8 == 2` | **`5`** | **`0xFD`** (`253`) | — | — |
| `\Data\Item.dat` | `0x00774BA8` | `370` byte (`0x172`) | `u16 == 5` | **`9`** | **`0x9A`** (`154`) | **`0xEFC3`** (`61379`) | **`0x0B80F4B4`** |
| `\Data\WeapBody.dat` | `0x004CA43C` | `642` byte (`0x282`) | `u16 == 1` | **`5`** | **`0xFD`** (`253`) | **`0xECEA`** (`60650`) | — |
| `\Data\Suit.dat` | `0x004DBAA0` | `642` byte (`0x282`) | `u16 == 1` | **`5`** | **`0xFD`** (`253`) | **`0xECEA`** (`60650`) | — |

### 16.2. Các offset quan trọng trong bản ghi `TFItem` (`Item.dat`, stride `0x172 = 370` byte) `[ĐÃ XÁC MINH]`

*(Lưu ý: Trong ASM của `TFItem`, bản ghi thứ `i` (`1..count`) bắt đầu tại `inst + i*0x172 + 0x04`. Vì vậy offset trong file `Item.dat` cho bản ghi thứ `i` bằng `offset_ASM - 4`):*

| Offset trong ASM (`inst + i*370 + ...`) | Offset tương đối trong Record `Item.dat` (`0..369`) | Kiểu dữ liệu | Tên trường | Vai trò trong Module Mix |
| :---: | :---: | :---: | :---- | :---- |
| `+0x04` | `+0x00` (`0`) | `ShortString[20]` | `name` | Tên vật phẩm (21 byte: 1 byte độ dài \+ 20 byte ký tự Big5), dùng để in kết quả Mix (`0079f3c8_mix_handler.asm.txt:308`) |
| `+0x19` | `+0x15` (`21`) | `u8` | **`kind`** (`type`) | Loại vật phẩm (`1..14` \= trang bị, `20..22` \= cấm mix). So khớp với `result_kind` của `Compound.Dat` |
| `+0x1A` | `+0x16` (`22`) | `u16` (LE) | **`item_id`** | Mã ID vật phẩm (`10001..65526`), dùng để dựng bảng tra `[0x009D162C]` và trả về `out_item_id` |
| `+0x32` | `+0x2E` (`46`) | `u8` | **`ratA`** (`material`) | Mã chất liệu của vật phẩm (`1..37`). Dùng để ghép chuỗi tạo khóa `u32` tra bảng `Compound.Dat` |
| `+0x33` | `+0x2F` (`47`) | `u8` | **`ratB`** (`rank`) | Cấp bậc/phẩm chất hợp thành của vật phẩm (`1..10+`). Dùng để tính `target_rank` và lọc vật phẩm đầu ra |

### 16.3. Hàm làm tròn chuẩn Delphi (`delphi_round`) và Bộ sinh số ngẫu nhiên (`DelphiRng`) bằng Rust

/// Tái tạo chính xác lệnh FISTP qword ptr \[ESP\] trong @ROUND (0x00402B1C):

/// Làm tròn về số nguyên gần nhất; nếu phần lẻ đúng bằng 0.5 thì làm tròn về số nguyên CHẴN (Round half to even).

pub fn delphi\_round(val: f64) \-\> i64 {

    val.round\_ties\_even() as i64

}

/// Bộ sinh số giả ngẫu nhiên LCG chuẩn của Borland Delphi (@RandInt @ 0x00402CF0)

pub struct DelphiRng {

    pub rand\_seed: i32, // Biến toàn cục tại \[0x007C9004\]

}

impl DelphiRng {

    pub fn new(seed: i32) \-\> Self {

        Self { rand\_seed: seed }

    }

    /// Trả về số nguyên ngẫu nhiên trong đoạn \[0, range \- 1\] (nếu range \> 0\)

    pub fn random\_int(\&mut self, range: i32) \-\> i32 {

        // ASM 0x00402CF0:

        // IMUL EDX, \[0x007C9004\], 0x08088405

        // INC  EDX

        // MOV  \[0x007C9004\], EDX

        // MUL  EDX  (nhân không dấu EAX \= range với EDX \= new\_seed \-\> kết quả 64-bit EDX:EAX)

        // MOV  EAX, EDX (lấy 32 bit cao)

        let new\_seed \= (self.rand\_seed as u32)

            .wrapping\_mul(0x0808\_8405)

            .wrapping\_add(1);

        self.rand\_seed \= new\_seed as i32;

        let prod \= (new\_seed as u64) \* (range as u32 as u64);

        (prod \>\> 32\) as i32

    }

}