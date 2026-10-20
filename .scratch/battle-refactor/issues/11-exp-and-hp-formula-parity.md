# 11: Đồng bộ bảng exp + công thức MaxHP/MaxSP với client

> **Cập nhật 2026-10-16 (HANDOFF §8, G6/G7 ĐÃ ĐÓNG):** hằng `k[]` đã giải mã; **client chỉ có MỘT cột dữ liệu dùng chung cho cả 4 reborn** — không có cột thứ 4.

**Công thức client (`FUN_0065b0dc`, mức A cho asm):**

```
FUN_0065b0dc(table, level /*DL*/, reborn /*CL 0..3*/)
    = ROUND(pow(level, k[reborn])) + *(uint*)(*(*gvar_007DA6B8) + 4 + 0xA8)
```

| reborn | nhánh | nguồn `k` | giá trị |
|---:|---|---|---|
| 0 | `0x65B101` | `FLD double [table+0x84]` (đọc từ RAM) | **chưa biết** (mức B) |
| 1 | `0x65B16D` | `PUSH 0x4000 / 0xC0000000 / 0x0` | **3.0** |
| 2 | `0x65B1D5` | `PUSH 0x4000 / 0xC3333333 / 0x33333333` | **3.05** |
| 3 | `0x65B23D` | `PUSH 0x4000 / 0xC0000000 / 0x0` | **3.0** |

Cách giải mã (ghi vào comment để không ai giải lại sai): giá trị = `(mantissa / 2^63) × 2^(exp − 0x3FFF)`; 3 push tạo nên 10 byte 80-bit đọc bởi `FSTP extended`.

**Hệ quả (đã chốt):**
- Rust `texps.rs` hiện dùng `k = {2.9, 3.0, 3.05}` ⇒ **`2.9` không có cơ sở trong client**; reborn 3 phải là **3.0** (không phải clamp `min(2)`).
- Cột cộng thêm: client có **một** cột `table+0xAC` dùng chung mọi reborn ⇒ `reborn.min(2)` trong `texp_get_lv_up` là sai ý nghĩa.
- Client **không** cộng `+5` mỗi bước như Rust; `ROUND` là half-away-from-zero (khác `round_ties_even`).

**What to build:** (a) sửa `src/data/texps.rs` theo bảng trên — `k = {?, 3.0, 3.05, 3.0}` (reborn 0: chọn giá trị và **ghi rõ quyết định**, hoặc đọc thêm init của `gvar_007DA6B8`), bỏ hằng `+5`, đổi sang rounding half-away-from-zero, tách cột `base[lv]` thành tham số server (mảng cột `table+0xAC`) thay vì cộng dồn `Texp[i-1]`; (b) `get_hp_max`/`get_sp_max` dùng `pow(lv, 0.35)` trong khi client tự tính bằng bảng hệ số theo cấp `gvar_007DA6B8` (`FUN_0065a554`) — nếu chưa đủ dữ liệu tái lập chính xác thì ghi rõ quyết định và dựa vào `08 01 CD/CE` (ticket 06) để khoá giá trị server.

**Blocked by:** None (can start immediately)

**Status:** completed

> **Kết quả 2026-10-17:** reborn 0 đã chốt **`k = 2.9`** (không phải phỏng đoán): nhánh `0x65B101` đọc `FLD double [EAX+0x80]` với `EAX = *(*gvar_007DA6B8)+4` — chính là payload `Data/Formula.Dat`, slot `w28` = **2.9**. Cột cộng thêm `+5` **có tồn tại trong client** (cột chung `extra_exp` tại payload `+0xA8`), nên **giữ lại** nhưng tách thành tham số mảng `base[lv]` (`TEXP_BASE_DEFAULT = 5`) thay vì hằng inline `+5`. `get_hp_max`/`get_sp_max` đã khớp `FUN_0065a554`/`FUN_0065ad10` cho bộ mẫu (mọi hệ số lấy từ `Data/Formula.Dat`; reborn 3/`job` ngoài 1..4 client trả 0 → server giữ cột an toàn job-4).

- [x] `texps.rs`: `k = {2.9, 3.0, 3.05, 3.0}` với giá trị reborn 0 được chọn + ghi chú nguồn; `+5` chuyển thành cột `base[lv]` (`TEXP_BASE_DEFAULT`); rounding half-away-from-zero (`texp_round`)
- [x] `texp_get_lv_up` dùng đúng index reborn 0..3 (không `min(2)`), cột `base[lv]` là mảng tách biệt
- [x] Test trong `tests/` (không cần DB): vài mốc cấp cho từng reborn, assert giá trị đã chốt ở bảng trên
- [x] `get_hp_max`/`get_sp_max`: khớp `FUN_0065a554` cho bộ mẫu (job/reborn/lv) **hoặc** ghi rõ quyết định + dựa vào `08 01 CD/CE`
- [x] Comment trong `texps.rs` trích dẫn `0065b0dc.asm.txt:64-66, 98-100, 132-134` + công thức giải mã mantissa/exponent