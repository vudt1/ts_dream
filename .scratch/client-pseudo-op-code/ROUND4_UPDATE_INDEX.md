# Chỉ mục cập nhật Round-4 redump (2026-10-02)

Mỗi tài liệu dưới đây đã được chèn một mục **`§0.1`** ở đầu file (ngay trước `§0` cũ).
Tài liệu nào chưa có `§0.1` thì **không có gì thay đổi** ở Round-4.

## Tài liệu đã cập nhật

| Tài liệu | Mục | Nội dung chính |
|---|---|---|
| `guild_city_siege_flow.md` | `§0.1`–`§0.3` | 🔴 `0x27` C→S **CÓ** body (dự đoán "lỗ hổng lớn nhất" sai) · bảng 57 entry giải được, **chỉ 28/57 sub gửi** · framing `[main][sub]` chốt · nền tảng `FUN_0077F414` 42% |
| `light_dark_skill_unlock_gap.md` | `§0.1` | 🔴 **`0x1C` chỉ có sub `01/02/04`** (sub `03` là epilogue; `05`/`06` không có trong bảng) · 🔴 **BLOCKER**: `0x94B440` + `0x9AE128` ngoài vùng memory map |
| `marriage_child_feature_flow.md` | `§0.1` | ✅ `0x45` C→S **byte-exact** khớp §2.4 · ✅ framing `[main][sub]` chốt (3 nguồn) · Q10 đóng một phần |
| `reconnect_heartbeat_flow.md` | `§0.1` | ✅ `0x78A830`/`0x78A834` **byte-verify** ⇒ `F4 44 01 00 00` chính xác · byte-map 200 B đã dump |
| `battle/01_battle_start_scene.md` | `§0.1` | ❌ Gap #4 sai — **cả 14 handler `0x35` đều có body** (tài liệu tự đếm sai 13→"11") |
| `battle/02_battle_positions.md` | `§0.1` | ✅ Gap #3 đóng — `TFightField.Create` có body · hằng lưới **832×640 / ô 340×180 / mặc định 2×5** |
| `battle/03_skill_and_combo.md` | `§0.1` | 🔴 **P0 đóng** — layout sub `0..4` byte-verified · H-3 GAP → ✅ A · ❌ `FUN_00654590` có body (03 sai) |
| `battle/04_battle_end_rewards.md` | `§0.1` | ✅ G4 đóng — **`17 06` = 14 B** byte-exact · `17 05` record **stride 12 B** · G11 đóng |
| `battle/05_rust_battle_audit.md` | bảng | `0x0B` sub 1 = **2 byte** (Rust `F44405000B01{r}{c}00` thừa 1 byte) |
| `battle/README.md` | §2.3, §8 | Mục #1 P0 → **XONG** · thêm 3 việc mới |
| `economy/README.md` | `§0.1` | Bảng tổng hợp đóng/để ngỏ + blocker memory |
| `economy/01_shop_read_flow.md` | `§0.1` | `17 06` mức C → **A** |
| `economy/02_buy_sell_transaction.md` | `§0.1` | ❌ "13 file thiếu" sai (có 13 218) · ❌ HOLE sai `0x5DC96A..0x5DCE0` → `0x5DC98A..0x5DCCDF` · 4 mục §11.6-11.11 đóng |
| `economy/03_player_trade.md` | `§0.1` | Undet.3b đóng · Undet.9/10 literal đã dump |
| `economy/04_stall_vendor.md` | `§0.1` | U3/U4/U5/U14 đóng (3 handler có body) · ❌ U11/U12 "không có body" sai |
| `economy/05_special_economy.md` | `§0.1` | ✅ U9 đóng — bảng 9 nhánh `0x1E`, sub 0 = epilogue ⇒ **8 nhánh thật** |
| `economy/06,07,08` | — | Không có gì thay đổi ở Round-4 |

## Hai phát hiện nền tảng (áp dụng cho MỌI tài liệu)

### 1. `FUN_0077F414` chỉ mới được disassemble 42%

`index.csv` ghi `size_bytes = 19534`, nhưng span thật là `0x0077F414..0x0078A817`
= **46 084 B**. **26 550 B (58%) chưa bao giờ được disassemble**, nằm ở 15 khoang.
Round-4 đã thu hồi hết (trừ 2 khoang là bảng data thuần).

> ⚠️ **Đừng tin `size_bytes` khi tra địa chỉ** — mọi truy vấn trong
> `0x00784062..0x0078A89C` sẽ báo "HOLE 26 682 B" (sai). Dùng `.asm.txt` làm chuẩn.

### 2. 🔴 BLOCKER — memory đã map chỉ tới `0x7DA800`

Đo từ 2 dump đọc được một phần (`table_7da6dc_101DWORD.hex`: `0x7DA6DC` + 292 B
= `0x7DA7FF`). Mọi địa chỉ ≥ `0x7DA800` trả **0 byte**.

Bị chặn: `0x94B440` (cây kỹ năng 765 B) · `0x9AE128` (bảng `skillId→index`) ·
`0x9D1610/0x9D1620` (thang cược) · `0x948DF8`/`0x9490B0`/`0x98BE04` (Round-3 cũng fail).

**Không phải lỗi script.** Cần kiểm tra Window → Memory Map của project Ghidra;
nếu không có block phủ `0x94B440` thì phải re-import `aLogin.exe`.

## Danh sách việc phát sinh (chưa làm)

| # | Ưu tiên | Việc | File Rust |
|:-:|:--:|---|---|
| 1 | 🔴 P0 | **Sửa builder `17 06`** — hiện 11 B, phải **14 B** (`RP[1]=slot`, `id` tại `RP[2..3]`, `texp` tại `RP[10..13]`). Lệch 1 byte mọi field. | `src/server/handlers/inventory.rs:69-84`, `trade_storage.rs` |
| 2 | 🔴 P0 | Viết lại handler lệnh trận theo `0x0B` sub `02`/`08` (layout đã có) | `src/server/handlers/battle.rs:189-243` |
| 3 | ✅ XONG | ~~Dump `0x00785DA0..0x00785DBB` (27 B)~~ — **đã có sẵn** trong `cs_op1c_dispatch_cases.asm.txt`. Kết luận: `0x1C` có **7 sub**, `0`/`3` no-op, **gửi `01/02/04/05/06`**. Xem `light_dark §0.1.1` | — |
| 4 | 🟠 P1 | Đọc `0x94B440` (cây kỹ năng 765 B) + `0x9AE128` + `0x9D1610` — **không cần sửa Ghidra**: `python3 test/pe_dump.py <aLogin.exe> 0x94B440 0x765 -o out.hex` | (research) |
| 5 | 🟡 P2 | Bỏ `battle.rs:26` (`sub 5` dead code) hoặc ghi chú | `src/server/handlers/battle.rs` |
| 6 | ✅ XONG | ~~`0x0050F4CE` — disassemble thủ công `0x0050F4CE..0x00511134`~~ — **không cần**: nó nằm giữa `TForm1.Transition`, và hàm đó là **hiệu ứng chuyển cảnh màn hình**, không phải send/recv pump. Xem `reconnect §0.1.5` | — |

## Công cụ mới (2026-10-02)

| File | Dùng để |
|---|---|
| `test/pe_dump.py` | Đọc byte tại một **VA** thẳng từ file PE — **bypass hoàn toàn** blocker memory map của Ghidra. Tự in section table, image base, file overlay; `-o` ghi ra `.hex` đúng format corpus (parse được bởi `tools_decode_literals.py`) |
| `test/MemMapDiag.java` | Ghidra script **read-only**: in memory map đầy đủ + trạng thái 10 địa chỉ mục tiêu (có/không có block, đọc được mấy byte, có reference/data/function không). Chạy Script Manager, không cần arg |

> **Đính chính (2026-10-02):** `tools_decode_literals.py` **KHÔNG hỏng** — đường dẫn
> `ts_dream/client_pseudo_c/` vẫn tồn tại với **238 file `lit_*.hex`**. Vấn đề thật là nó
> **chỉ index corpus cũ**, bỏ qua **75 literal chỉ có trong `test/ts_decompile/redump/`**
> (`lit_78A854`, `lit_796f*`, `lit_7980*`, `lit_799*`, …).
> **Đã sửa** glob thành 2 thư mục. Đối chiếu byte-for-byte: **238 literal trùng tên
> giữa 2 corpus giống nhau 100%** ⇒ corpus nhất quán, không có xung đột.

## Tài liệu Round-4

| File | Nội dung |
|---|---|
| `REDUMP_ROUND4_MISSING.md` | Danh sách thiếu **trước** khi redump (154 directive) |
| `REDUMP_ROUND4_RESULTS.md` | Kết quả **sau** redump (471 ok / 1 fail, 44 144 B) |

Manifest: `test/Round4Manifest.txt` · Script: `test/Round3Redump.java` (không sửa)

*Cập nhật 2026-10-02. Mọi địa chỉ đã đối chiếu `test/ts_decompile/index.csv` và bảng
instruction thực tế trong `.asm.txt`.*
