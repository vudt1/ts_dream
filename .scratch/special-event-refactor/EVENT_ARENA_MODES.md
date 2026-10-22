# Bảng mode lôi đài ↔ prefix chat — trích từ `FUN_0053BAB8` (0x0053BAB8, 623 byte)

> **Nguồn:** `0053bab8_FUN_0053bab8.c` / `.asm.txt` (dump Ghidra mới).
> **Caller:** `FUN_0053C87C` (manager `TMRBatterManage`).
> **Mức bằng chứng:** (A) — đọc trực tiếp ASM + giải mã VISCII literal.

---

## 1. Cấu trúc dữ liệu

`FUN_0053BAB8(self, mode, flag, x)` **đẩy 1 bản ghi 7 byte** vào mảng động
`self+0x164` (`DynArray__3` — mảng record 7 byte):

| Offset | Kiểu | Nội dung |
|---:|---|---|
| `+0` | `byte` | **mode id** (`0x60`…`0x63`, hoặc giá trị khác) |
| `+1` | `byte` | **flag** (1 = "Tỉ thí", 2 = "đấu trận" / "loạn đấu") |
| `+2` | `byte` | tham số thứ 3 (`x`) |
| `+3` | `AnsiString` (4 byte) | **nhãn chat** dựng từ mode + flag |

⇒ Client **tự lập danh sách người chơi theo chế độ thi đấu** và gắn nhãn kênh chat.

## 2. Bảng mode id → nhãn

| mode id | hex | literal | nhãn chat | ghi chú |
|---:|---:|---|---|---|
| 96 | `0x60` | `0x0053BDA0` | **`[Lôi đài tranh đấu 4]`** | |
| 97 | `0x61` | `0x0053BD80` | **`[Lôi đài tranh đấu 3]`** | flag=2 → *"Đấu trận cấp 100"* |
| 98 | `0x62` | `0x0053BD60` | **`[Lôi đài tranh đấu 2]`** | flag=2 → *"Lọan đấu"* (= Solo PK) |
| 99 | `0x63` | `0x0053BD40` | **`[Lôi đài tranh đấu 1]`** | |
| *khác* | — | `0x0053BDCC` + số + `0x0053BDC0` | dựng động từ `IntToStr(mode)` | ⚠ 2 literal này decode ra ký tự vô nghĩa (`A6 F8 AA 41 BE B9` / `B2 C4`) — **cần xác minh thêm** |

## 3. Bảng hậu tố (flag)

| flag | chuỗi ghép | literal | decode |
|---:|---|---|---|
| 1 | (mọi mode) | `0x0053BDF8` + `0x0053BDE8` + `0x0053BDDC` | **"Tỉ thí"** + **"Đòan P"** + **"  "** (2 khoảng trắng) |
| 2, mode `0x61` | | `0x0053BE1C` + `0x0053BDDC` | **"Đấu trận cấp 100"** + "  " |
| 2, mode `0x62` | | `0x0053BE08` + `0x0053BDE8` + `0x0053BDF8` | **"Lọan đấu"** (Loạn đấu) + "Đòan P" + "Tỉ thí" |
| 2, mode khác | | — | không ghép hậu tố |

> ⚠ `"Đòan P"` (`D0 F2 61 6E 20 50`) — nhiều khả năng là **"Đoàn P"** bị lỗi chính tả
> trong file chuỗi của game (byte `F2` thay cho `6F`).

## 4. Kết luận về 5 chế độ lôi đài

| Chế độ | mode id | nhãn chat | suy ra từ |
|---|---|---|---|
| **Tranh đấu 1** | `0x63` | `[Lôi đài tranh đấu 1]` | bảng mode |
| **Tranh đấu 2** | `0x62` | `[Lôi đài tranh đấu 2]` + "Lọan đấu" | bảng mode + hậu tố |
| **Tranh đấu 3** | `0x61` | `[Lôi đài tranh đấu 3]` + "Đấu trận cấp 100" | bảng mode + hậu tố |
| **Tranh đấu 4** | `0x60` | `[Lôi đài tranh đấu 4]` | bảng mode |
| **Tỉ thí / Đoàn P** | flag 1 | mọi mode + "Tỉ thí" + "Đòan P" | hậu tố |

> **Khớp với bản đồ map** (`data_event_maps.tsv`): 5 map cùng toạ độ `(128, 196)` với
> `Kind` = 8 (`Lôi đài thôn thực`), 9 (`Lôi đài 5 vs 5`), 10/11/12 (`LôiĐàiHỏiĐápNămMới`).
> ⇒ **mode id và `Kind` của map là hai mã khác nhau** — server phải ánh xạ.

## 5. Việc agent cần làm tiếp

1. Truy `FUN_0053C87C` (caller) để biết **ai** quyết định `mode`/`flag` khi người chơi
   báo danh / bắt đầu trận — nhiều khả năng là từ frame Server→Client (Main OP `0x3F`).
2. Giải mã 2 literal lạ tại `0x0053BDC0` / `0x0053BDCC` (byte thô `B2 C4` / `A6 F8 AA 41 BE B9`).
3. Đối chiếu `mode id` ↔ `Kind` map ↔ tên chế độ trong `eve.emg`
   (`data_files/eve/eve_logic.tsv`, quest `10714`/`10715` → scene `12003`).
