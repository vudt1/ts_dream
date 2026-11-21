# Kế Hoạch Triển Khai — Sửa `invert_emg_ops(4)` Cho Thương Nhân BDY `#16024` (Map `#12003`) & Eve Evaluator

## Mục tiêu (Goal Description)
Khắc phục lỗi **Thương nhân BDY `#16024`** tại map `#12003` (`place_id = 8`) không thể đổi **vé Đoái hoán tâm `#31007`** lấy sủng vật **Ba Đậu Yêu `#22028`** dù vé có sẵn trong `homdo`.

Nguyên nhân gốc (xem chi tiết tại [research_map12003_bdy_and_ngumagiam.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/sysmsg-and-map10817-eve/research_map12003_bdy_and_ngumagiam.md)):
- Hàm [`invert_emg_ops`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L46-L54) đang ánh xạ `4 => 2` (`actual < expected`), khiến mọi điều kiện `condition_ops = 4` trong `Data/eve.emg` (với `condition_class = 1` BagItem và `condition_class = 7, param ∈ {0, 2}` Level/Reborn) bị đảo thành `<` thay vì `>=`.
- Tại map `12003`, `eve_no = 8` (`#31007`) dùng `ops = 4, val = 1` nên bị đánh giá `false` khi người chơi đang có `1x #31007`. Sau đó `auto_chain_after` nhảy nhầm sang `eve_no = 19` (`#31023`, `0 < 1 = true`), mở Surface `9` và báo lỗi `"Vật phẩm không đủ, không thể thực hiện."` khi chọn `14 09 1E`.

Thay đổi này chuẩn hóa `invert_emg_ops(4) => 5` (`actual >= expected`), cập nhật ghi chú đặc tả vào ADR `0009`, và bổ sung test hồi quy cho cả `#16024` lẫn `cc = 7, param = 0`.

---

## Quyết định Đã Chốt (Qua `/grill-with-docs`)
- **Phạm vi sửa toán tử**: Đổi `4 => 5` (`actual >= expected`) trong `invert_emg_ops` cho cả `cc = 1` (`BagItem`) và `cc = 7, param ∈ {0, 2}` (`Level`/`Reborn`).
- **Không thêm special-case**: Giữ nguyên `EveAutoChainEngine` và `talk.rs`, không viết nhánh riêng cho NPC `#16024`.
- **Tài liệu**: Giữ nguyên `GLOSSARY.md` (không đưa số opcode vào glossary) và không tạo ADR mới; bổ sung ghi chú bảng ánh xạ `invert_emg_ops` vào [0009-eve-quest-engine-and-persistence.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md) và docstring của [`src/eve/evaluator.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L39-L54).

---

## Thay Đổi Đề Xuất (Proposed Changes)

### 1. Eve Condition Evaluator (`src/eve/`)

#### [MODIFY] [evaluator.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/evaluator.rs#L39-L54)
Cập nhật `invert_emg_ops` để ánh xạ `4 => 5` (`actual >= expected`) và ghi rõ docstring quy ước thực tế của `Data/eve.emg`:

```rust
/// Normalize a `conditionOps` operator for `Data/eve.emg` classes (`cc = 1` BagItem
/// and `cc = 7, param = 0 | 2` Level/Reborn):
/// - `2` -> `4` (`expected < actual`  <=> `actual > expected`)
/// - `3` -> `5` (`expected <= actual` <=> `actual >= expected`)
/// - `4` -> `5` (`Data/eve.emg` encodes `actual >= expected` directly, e.g. `val = 1, 5, 15`)
/// - `5` -> `3` (`expected >= actual` <=> `actual <= expected`)
#[must_use]
fn invert_emg_ops(ops: u8) -> u8 {
    match ops {
        2 => 4,
        3 => 5,
        4 => 5,
        5 => 3,
        other => other,
    }
}
```

---

### 2. Tài Liệu Kiến Trúc (`docs/adr/`)

#### [MODIFY] [0009-eve-quest-engine-and-persistence.md](file:///D:/VUDT/GIT_PCC/ts_dream/docs/adr/0009-eve-quest-engine-and-persistence.md#L28)
Bổ sung ghi chú dưới mục `2.1` về bảng chuẩn hóa `condition_ops` trong `src/eve/evaluator.rs` cho `cc = 1` và `cc = 7 (param ∈ {0, 2})` (`2 → >`, `3 → >=`, `4 → >=`, `5 → <=`) để tránh hiểu nhầm ngữ nghĩa của `ops = 4` khi đọc mã nguồn sau này.

---

### 3. Kiểm Thử Hồi Quy (`tests/eve_quest/`)

#### [MODIFY] [missing_eve_effect_executors.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/missing_eve_effect_executors.rs#L1535)
Thêm 2 test mới ở cuối file:
1. `evaluator_invert_emg_ops_4_treats_bag_item_and_level_as_gte`:
   - Kiểm tra `cc = 1, ops = 4, val = 1` (`qty = 0` → `false`, `qty = 1` → `true`, `qty = 2` → `true`) và `val = 5` (`qty = 4` → `false`, `qty = 5` → `true`).
   - Kiểm tra `cc = 7, param = 0, ops = 4, val = 15` (`level = 14` → `false`, `level = 15` → `true`).
2. `map_12003_bdy_16024_trades_ticket_31007_for_pet_22028`:
   - Dựng scene `12003` với cấu trúc `eve_no = 8` và `eve_no = 19` của `place_id = 8` tại `(690, 860)` (hoặc nạp trực tiếp từ `Data/eve.emg`).
   - Đặt nhân vật trong tầm `130 px`, mang `1x #31007` trong `homdo` và `< 4` pet.
   - Thực hiện chuỗi gói thật: `14 01 08 00` → 2x `14 06` → xác nhận server mở **Surface `8`** (`14 01 ... 08 00`, không nhảy sang Surface `9` của `eve_no = 19`) → gửi `14 09 1E` → xác nhận trừ `1x #31007` khỏi `homdo` và thêm pet `#22028` vào `follow_pets`.

---

## Kế Hoạch Kiểm Chứng (Verification Plan)

### Automated Tests
1. Chạy suite `eve_quest_test` sau khi sửa:
   ```powershell
   cargo test --test eve_quest_test
   ```
2. Chạy toàn bộ 11 test suite chuẩn trước khi handoff:
   ```powershell
   cargo test --all-targets --no-fail-fast
   ```

### Manual Verification (Live-Test với `aLogin.exe`)
1. Vào map `#12003`, đảm bảo `homdo` có `1x #31007` (*Vé đoái hoán tâm*) và đội hình `< 4` pet (chưa có `#22028`).
2. Click Thương nhân BDY `#16024` (`(690, 860)`), bấm tiếp qua 2 đoạn thoại chào (`12438`, `15963`).
3. Xác nhận client mở bảng chọn **Surface `8`**, chọn xác nhận đổi (`14 09 1E`), kiểm tra `#31007` bị trừ và pet Ba Đậu Yêu `#22028` xuất hiện trong danh sách sủng vật.
