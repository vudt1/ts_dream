# ĐẶC TẢ `\Data\Festival.Dat` — đã giải mã & kiểm chứng byte-exact

> **Nguồn:** `aLogin.exe` (Delphi) — class `TFestival` (VMT `0x0051DE74`).
> **Hàm nạp:** `FUN_0051E00C` (`TFestival.Load`) · **Hàm giải mã:** `FUN_0051DECC` (`TFestival.Decrypt`).
> **Script giải mã:** `tools/festival_dat_decrypt.py` · **Kết quả giải mã:** `festival_dat_decrypted/rec00.bin … rec09.bin`.
> **Trạng thái:** thuật toán giải mã **đã kiểm chứng** (giải mã ra văn bản tiếng Việt có nghĩa).
> Mức bằng chứng: **(A)** — đọc trực tiếp byte + ASM.

---

## 1. Ai đọc file này

| Thành phần | Địa chỉ | Ghi chú |
|---|---|---|
| Lớp `TFestival` | VMT `0x0051DE74` | tên lớp lấy từ short-string `09TFestival` tại `0x0051DEC0`; `InstanceSize = 0xB5FA8` (745 640 byte) |
| Instance toàn cục | `DAT_009264E0` | được tạo trong `TForm1.FormCreate` (`0x0050A4A0`): `TObject_Create(VMT_51DE74_TFestival, …)` rồi `FUN_0051E00C(DAT_009264E0)` |
| Loader | `FUN_0051E00C` (495 byte) | `client_special_event_handler/0051e00c_FUN_0051e00c.c` + `.asm.txt` |
| Decryptor | `FUN_0051DECC` (237 byte) | `client_special_event_handler/0051decc_FUN_0051decc.c` + `.asm.txt` |
| Hàm tra cứu theo ngày | `FUN_0051E2DC` | quét `i = 1..count`, so sánh `*(double*)(self + 4 + i*0x7EF)` với tham số → trả về chỉ số |
| Hàm lấy đoạn văn 1 / 2 | `FUN_0051E358` / `FUN_0051E3B8` | đọc 2 khối văn bản của bản ghi |
| Handler mở form lễ hội | `FUN_0051D914` | đọc 4 byte id từ payload → `FUN_0077ED68(gvar_007D9D30, id)` → **TDateTime** → `FUN_0051E2DC` tìm bản ghi → đổ tiêu đề + 2 đoạn văn lên form |

---

## 2. Thuật toán nạp (FUN_0051E00C — đọc từ `.asm.txt`)

```
1.  path = "\Data\" + <thư mục client> + "\Festival.Dat"
2.  nếu !FileExists(path) -> ShowMessage("Festival.Dat không tồn tại")  ; thoát
3.  stream = TFileStream.Create(path, ...)          ; FUN_0040673A
4.  stream.Open(RecordSize = 0x7EF)                 ; FUN_00406CF9 -> RecordSize = 2031
5.  nRec   = GetFileSize(stream) / RecordSize       ; FUN_004069DC
6.  count  = nRec - 3                               ; -> *(word*)(self + 0xB5FA6)
7.  nBytes = (nRec - 2) * 0x7EF
8.  nếu nBytes > 0xB5FA1 -> ShowMessage("Festival.Dat size …")   ; giới hạn 745 633 byte
9.  ReadBlock(stream, self + 4, nRec - 2)           ; đọc (nRec-2) bản ghi vào self+4
10. ReadBlock(stream, &DAT_00926FE0, 2)             ; đọc 2 bản ghi cuối -> 2 khoá XOR
11. stream.Close()
12. FUN_0051DECC(self)                              ; giải mã
```

> **Lưu ý:** `FUN_00402864` xuất hiện nhiều lần trong pseudo-C nhưng thực chất là
> `System._CheckIntOverflow` (kiểm tra tràn số), **không** phải hàm đọc dữ liệu.
> Đọc `.asm.txt` để khôi phục đúng luồng.

### Kết quả với file đang dùng (`Data/Festival.Dat`, 20 310 byte)

```
FileSize     = 20310 byte
RecordSize   = 2031 (0x7EF)
Số bản ghi   = 10
count        = 10 - 3 = 7      -> TFestival + 0xB5FA6 (word)
nBytes       = (10-2) * 2031 = 16248  <= 745633  -> OK
Đọc vào self+4 : bản ghi [0] .. [7]   (8 bản ghi = 16248 byte)
Đọc vào khoá   : bản ghi [8] = keyA, bản ghi [9] = keyB
```

---

## 3. Thuật toán giải mã (FUN_0051DECC)

```
keyPtr[0] = 0x00926FE0      // = bản ghi [nRec-2] trong file
keyPtr[1] = 0x009277CF      // = bản ghi [nRec-1] trong file  (liền kề keyPtr[0], cùng 2031 byte)

for i = 1 .. count:                       // BẮT ĐẦU Ở 1 (bản ghi [0] KHÔNG được giải mã)
    rec = self + 4 + i * 0x7EF
    k   = i & 1                           // 1 nếu i lẻ, 0 nếu i chẵn
    for j = 0 .. 0x7EE:
        rec[j] ^= keyPtr[k][j]
```

* Bản ghi **chẵn** (2, 4, 6) XOR bằng `keyA` = bản ghi `[nRec-2]`.
* Bản ghi **lẻ** (1, 3, 5, 7) XOR bằng `keyB` = bản ghi `[nRec-1]`.
* `count` tối đa `0x16E` (366) — vượt thì `BoundErr`.
* Khoá XOR dài 2031 byte (đúng bằng 1 bản ghi) → mỗi byte dùng 1 byte khoá, lặp lại theo offset.

---

## 4. Layout 1 bản ghi (2031 byte = `0x7EF`)

| Offset | Kích thước | Kiểu | Nội dung (quan sát trên file đang dùng) |
|---:|---:|---|---|
| `+0x0000` | 8 | `double` (TDateTime) | `11.0` ở cả 7 bản ghi — dùng để **tra cứu theo ngày** (`FUN_0051E2DC` so sánh double này) |
| `+0x0008` | 2 | `word` | `0xB4C2` (46 274) — hằng số ở cả 7 bản ghi (có thể là màu UI `TColor` hoặc id nhóm) |
| `+0x000A` | 1 | `byte` | độ dài tiêu đề (ShortString) |
| `+0x000B` | ≤ 20 | `char[]` VISCII | **Tiêu đề** lễ hội (ShortString, kết thúc ở `0x001F`) |
| `+0x001F` | ≤ 999 | `char[]` VISCII, kết thúc `\0` | **Đoạn văn bản 1** |
| `+0x0407` | ≤ 999 | `char[]` VISCII, kết thúc `\0` | **Đoạn văn bản 2** |
| `+0x07EF` | — | — | hết bản ghi (phần đệm bằng `0x00`) |

> **Đường dẫn đọc trong code (đã kiểm chứng):**
> `FUN_0051D914` gọi `_LStrFromString(dst, festival + 0x0E + i*0x7EF)` ⇒ tiêu đề nằm ở
> `record + 0x0B` (length byte ở `record + 0x0A`), tức `festival + 4 + i*0x7EF + 0x0A`.
> `FUN_0051E358` / `FUN_0051E3B8` đọc lần lượt hai khối văn bản.

---

## 5. Nội dung 7 lễ hội trong file đang dùng (đã giải mã)

| # | Tiêu đề | Đoạn 1 (ký tự) | Đoạn 2 (ký tự) |
|---:|---|---:|---:|
| 1 | Tết Nguyên Đán | 658 | 305 |
| 2 | Tết Nguyên Tiêu | 286 | 216 |
| 3 | Valentine date | 349 | 357 |
| 4 | Ngày Cà Thang Tử (Halloween) | 228 | 479 |
| 5 | Giỗ Tổ Hùng Vương | 326 | 161 |
| 6 | Lễ Phật Đản | 155 | 449 |
| 7 | Tết Đoan Ngọ | 311 | 420 |

Bản giải mã đầy đủ nằm ở `festival_dat_decrypted/rec01.bin … rec07.bin` (mỗi file 2031 byte).
Chạy `python3 tools/festival_dat_decrypt.py` để xem lại kèm giải mã VISCII.

---

## 6. Những điều CHƯA xác minh (cần agent nghiên cứu tiếp)

1. **Vì sao `double` ngày = 11.0 ở cả 7 bản ghi?** Giả thuyết: file `.dat` trong `Data/` là
   bản "mẫu"/placeholder, ngày thật do Server gửi xuống (payload 4 byte → `FUN_0077ED68`
   tra bảng ngày ở `gvar_007D9D30` → `TDateTime`). Cần truy ngược `FUN_0077ED68` và
   `gvar_007D9D30` để xác nhận.
2. **Ý nghĩa `word` 0xB4C2 tại `+0x0008`** (màu? id? loại lễ hội?).
3. **Ai sinh ra `Festival.Dat`** (tool phía server) — cần định dạng đầu vào để server Rust
   có thể tạo file tương thích.
4. **`FUN_0051D8E4` / `FUN_0051DD3C`** — 2 hàm còn chưa rõ vai trò (ghi bảng ngày?).
5. **Form UI hiển thị lễ hội** — `FUN_0051D914` dùng `local_8[0x4E..0x53]`, `FUN_007B8060`
   (set text), `FUN_007B0094` (ẩn/hiện control). Cần map class VMT tương ứng để biết
   form nào, nút nào.

---

## 7. Ảnh hưởng tới server Rust

* Server **không** cần gửi nội dung lore (client đọc từ `Festival.Dat` cục bộ).
* Server **có** cần gửi **mốc thời gian / ngày** để client mở đúng bản ghi lễ hội.
* Nếu muốn server điều khiển lễ hội theo lịch, cần biết:
  * bản ghi nào (1..7) ứng với ngày nào → hiện client dùng `double` trong file để tra,
    nên file phải có **ngày thật** (không phải 11.0) → cần sửa/sinh lại `Festival.Dat`.
  * hoặc Server gửi trực tiếp chỉ số bản ghi theo một frame riêng (cần xác minh).
