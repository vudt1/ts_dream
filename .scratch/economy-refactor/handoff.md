# Handoff: Hệ Thống Kinh Tế Game (Tickets 01 - 08)

> **Thời điểm**: 2026-10-16  
> **Trạng thái tổng thể**: Hoàn thành toàn diện 8/8 tickets (01 -> 08), đã qua rà soát độc lập, thẩm định nguồn gốc bậc 1 (`08-review-verification.md`) và kiểm thử 100% pass trên 38/38 test suites của repository.

---

## 1. Tóm Tắt Trạng Thái Các Ticket

| Ticket | Tiêu đề | Trạng thái | Chi tiết triển khai & Sửa đổi |
| :--- | :--- | :---: | :--- |
| **01** | NPC Shop Buy/Sell Flow & Dialogue Menu Bridge | **Hoàn thành** | Kết nối menu choice 30/32 trong `talk.rs` gửi `1B 03` mở `TThingSaleForm`. Phân định rạch ròi nhánh Mua (`1B 01`) và Bán (`1B 02`) trong `shops.rs`. Đọc `count: u16 LE` (guard `> 255` trả `1B 01 03`, `count == 0` trả `1B 01 04`). Tra cứu `scene_eve_data` kết hợp `item_defs`, thu hẹp theo surface của NPC (`last_surface_id`/`idtalking`). Bán tính theo `sell_price * qty`. Chuỗi wire chuẩn: Mua (`1A 02` -> `1A 04` -> `17 06` -> `1B 01 00`), Bán (`1A 01` -> `1A 04` -> `17 09` -> `1B 02 00`). Giữ lại starter bundle NPC 7 Map 9999 kèm gửi đầy đủ `17 06`. Sửa comment `dispatcher.rs`. |
| **02** | Currency Model & Bank Gold Sync at Login | **Hoàn thành** | Chuẩn hóa mô hình 3 loại tiền tệ: Gold (`+0x12F8`, trần `9,999,999`), Bank Gold (`+0x12FC`, trần `9,999,999`), Shop Points (`+0x1300`, trần `2,000,000,000`). Tại `spawn.rs`, phát gói `1D 04` đồng bộ bank gold vô điều kiện (kể cả khi `bank_gold == 0`) và gửi `s.shop_point` ở DWORD thứ 2 của `1A 04`. Thống nhất clamp trần `protocol::GOLD_CAP`. |
| **03** | Bank Deposit & Withdrawal Operations | **Hoàn thành** | Xử lý C→S `1D 01` (Rút: Bank -> Túi) và `1D 02` (Nộp: Túi -> Bank). Kiểm tra trần `GOLD_CAP` và số dư, cập nhật nguyên tử DB `character_money`. Trả về `1D 02` + `1A 01` khi rút, `1D 01` + `1A 02` khi nộp, và `1D 03` khi lỗi hoặc amount = 0. Mở form Tiền Trang bằng `1D 05`. |
| **04** | Storage & Warehouse Item Transfers | **Hoàn thành** | Khắc phục triệt để lỗi hardcode `"32"` tại `src/server/handlers/trade_storage.rs`, thay bằng `{:02X}` ứng với số lượng thực tế của item trên các frame `1E 05`, `17 09`, `17 68`. Đồng bộ nguyên tử danh mục kho (`storagetype = 1` túi đồ, `2` Tiền Trang, `16` Lưu Lãng) vào DB `inventories` với slice đầy đủ `&["homdo", "tientrang", "luulang"]`. |
| **05** | Player Stall Vendor & Catalog Browsing | **Hoàn thành** | Chuẩn hóa `player_shop_catalog_frame` theo giải mã `FUN_00610290` (Level A): Header 17B (`17 21` + `seller_id` + 12B pad 0) và Records 16B (`slot`, `itemId`, `count`, `long_val`, `price`, `giatri_long` biased +100, `khang`, `pad`, `texp`). Quy ước tra cứu 1-based (`shop_index.checked_sub(1)`), loại bỏ các nhánh fuzzy fallback (sửa triệt để bug lệch slot A1). Xử lý mở/đóng quầy `17 1E`/`17 1F`. Gỡ bỏ listing khi hết hàng. |
| **06** | Player-to-Player Trade & Pet Trade Separation | **Hoàn thành** | Tách biệt hoàn toàn luồng `19 01` (Item Trade) và `19 0A` (Pet Trade) trong `trade_storage.rs`. Bổ sung gói `17 3B` (`trade_cancelled_frame()`) đóng form giao dịch an toàn khi hủy, thất bại settlement, hoặc timeout (`started_at_ms`). Trả về mã toast `19 02 01` (offline) và `19 02 02` (bận/không hợp lệ). Thực hiện hoán đổi nguyên tử kép qua transaction SQLite. |
| **07** | Economy Integration Tests | **Hoàn thành** | Bộ test tích hợp toàn diện đặt tại `tests/economy_wire_test.rs` với 9 bài test chạy độc lập trên in-memory SQLite (`sqlite::memory:?cache=shared`), không ghi đè production DB `DB/ts_dream.db`, 0 test inline trong `src/`. |
| **08** | Review Verification & Bug Fixes | **Hoàn thành** | Thẩm định và khắc phục toàn bộ các phát hiện theo `.scratch/economy-refactor/08-review-verification.md`: Sửa bug A1 (1-based slot), B1 (persist Tiền Trang / Lưu Lãng và test DB), B2 (timeout 17 3B), B3 (toast 19 02 01/02), B4 (1D 04 vô điều kiện), C1 (bảo tồn +100 wire bias thống nhất `biased_giatri_long`), C2 (reject count == 0), C3 (đổi tên stall subcode 0x1F / 0x20), D1-D5 (gộp GOLD_CAP, gold_frame, helper F44402001B03 / F4440200173B, chuẩn hóa naming seller_id), E1 (thu hẹp surface NPC), E3 (giữ effect 7 param 3 kèm doc). |

---

## 2. Chi Tiết Các Lỗi Đã Được Khắc Phục (Bug Fixes)

1. **Khắc phục Bug Logic A1 & E2 (Lệch 1 slot quầy người chơi)**:
   - *Nguyên nhân*: Catalog gửi slot 1-based (`idx = 1..N`), client echo lại đúng `idx` đó khi mua, nhưng `complete_shop_buy` dùng `seller.shop.items.get(shop_index)` (0-based) làm mua món 1 lại nhận món 2.
   - *Khắc phục*: Sửa `complete_shop_buy` sang tra cứu thuần 1-based `shop_index.checked_sub(1)`. Xóa bỏ 3 nhánh fuzzy index che giấu lỗi.
   - *Fix bổ sung*: Sửa `ShopBuyResult.item` trả về `purchase_item` mang đúng `count` đã mua, tránh phát nhầm toàn bộ stack đồ cũ của người bán cho người mua. Gỡ bỏ listing khi số lượng về 0.

2. **Khắc phục B1 (Persist kho Tiền Trang & Lưu Lãng)**:
   - *Nguyên nhân*: Comment hiểu nhầm bảng DB dẫn đến truyền thiếu table name slice.
   - *Khắc phục*: Cập nhật comment và table slice thành `&["homdo", "tientrang", "luulang"]`. Bổ sung integration test xác nhận các row SQLite lưu đúng `storagetype = 2` (Tiền Trang) và `storagetype = 16` (Lưu Lãng).

3. **Khắc phục B2 & B3 (Timeout giao dịch & Toast error code)**:
   - *Nguyên nhân*: `handle_trade` return im lặng khi đối tác bận/offline và thiếu timeout khi giao dịch bị bỏ quên.
   - *Khắc phục*: Thêm `started_at_ms` vào `TradeState`, thêm hàm `cancel_timed_out_trade` và phát gói `F4440200173B` khi hủy/timeout. Trả về mã toast `19 02 01` khi đối phương offline và `19 02 02` khi đối phương đang bận giao dịch.

4. **Khắc phục B4 (Phát 1D 04 vô điều kiện khi spawn)**:
   - *Nguyên nhân*: Điều kiện `if s.bank_gold > 0` khiến tài khoản mới hoặc số dư 0 không được xóa bộ nhớ đệm UI client (`player + 0x12FC`).
   - *Khắc phục*: Bỏ điều kiện `if s.bank_gold > 0`, luôn phát `F44406001D04<bank_gold>` (kể cả khi bằng 0). Cập nhật baseline frame count trong `login_char_flow_test.rs` và `inventory_storage_type_test.rs`.

5. **Khắc phục C2 (NPC shop buy count == 0)**:
   - *Nguyên nhân*: Server tự ý nâng `count = if count == 0 { 1 } else { count }`.
   - *Khắc phục*: Từ chối giao dịch và gửi mã lỗi `NPC_SHOP_BUY_DISRUPTED` (`1B 01 04`).

6. **Bảo tồn C1 (Wire bias +100 cho giatri_long)**:
   - *Căn cứ*: Disassembly client `FUN_00610290` (`uVar5 - 100`) chứng minh client TS Online bắt buộc trừ 100 khi đọc wire.
   - *Khắc phục*: Giữ nguyên bias `+100`, thống nhất toàn bộ các hàm dựng packet gọi qua canonical helper `crate::server::inventory::biased_giatri_long(item.giatri_long)`.

7. **Refactor Clean Code Standards (C3, D1, D2, D3, D4, D5, E1, E3)**:
   - Đổi tên `ITEM_SUB_STALL_CLOSE = 0x1F`, `ITEM_SUB_STALL_VIEW = 0x20` trong `protocol/mod.rs`.
   - Gộp `GOLD_CAP` về `ts_dream::protocol::GOLD_CAP`.
   - Dùng chung `gold_frame` giữa `shops.rs` và `trade_storage.rs`.
   - Tạo helper `open_npc_shop_frame()` (`F44402001B03`) và `trade_cancelled_frame()` (`F4440200173B`).
   - Thu hẹp `resolve_npc_shop_item` theo surface của NPC (`last_surface_id`/`idtalking`).
   - Giữ lại `talk.rs:502-503` (Eve effect 7 param 3) kèm doc comment tham chiếu `HotelShopAndSaveMapEffectHandler`.

---

## 3. Nhật Ký Kiểm Thử (Verification Record)

- **Test Tích Hợp Kinh Tế (`economy_wire_test`)**:
  ```bash
  cargo test --test economy_wire_test --no-fail-fast
  # Kết quả: 9/9 tests passed (100%)
  # Bao gồm: currency model, NPC buy/sell, starter bundle, bank deposit/withdraw,
  #          storage count fix, player stall catalog + 1-based buy, buyout removal,
  #          P2P trade / pet trade separation, storage warehouse DB persistence.
  ```
- **Toàn Bộ Test Suite Repo**:
  ```bash
  cargo test --all-targets --no-fail-fast
  # Kết quả: 38/38 test suites passed, 150+ tests passed, 0 failures.
  ```
- **Kiểm tra biên dịch & Lint**:
  ```bash
  cargo check --all-targets
  # Kết quả: 0 error, 0 warning hồi quy
  ```

---

## 4. Danh Sách Tập Tin Đã Điều Chỉnh

1. [`src/protocol/mod.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/protocol/mod.rs): Hằng số opcode/subcode kinh tế (`GOLD_CAP`, `ITEM_SUB_STALL_CLOSE = 0x1F`, `ITEM_SUB_STALL_VIEW = 0x20`, `NPC_SHOP_*`, `BANK_*`, `STORAGE_*`).
2. [`src/server/dispatcher.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs): Đính chính comment jump table opcode `0x1B`.
3. [`src/server/handlers/shops.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/shops.rs): Chuẩn hóa NPC shop buy/sell, reject `count == 0`, clamp `GOLD_CAP`, player shop catalog 17B/16B, tra cứu 1-based buy, gỡ listing khi hết hàng, dùng `biased_giatri_long`, thu hẹp surface NPC.
4. [`src/server/handlers/talk.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs): Gửi `open_npc_shop_frame()` khi mở shop, cập nhật `gold_frame(gold, shop_point)`, giữ và ghi chú Eve effect 7 param 3.
5. [`src/server/handlers/trade_storage.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs): Sửa count hex thực tế, tách item/pet trade, gửi toast `19 02 01/02`, timeout `trade_cancelled_frame()`, persist đủ kho Tiền Trang & Lưu Lãng.
6. [`src/server/spawn.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs): Đồng bộ bank gold `1D 04` vô điều kiện và shop points `1A 04` khi spawn.
7. [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs): Bổ sung `started_at_ms` trong `TradeState`.
8. [`src/server/trade_system.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs): Dùng hằng `protocol::GOLD_CAP`.
9. [`tests/economy_wire_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/economy_wire_test.rs): Suite test tích hợp cho tickets 01-08 với 9 bài test, bao gồm test 1-based slot và DB storage persistence.
10. [`tests/login_char_flow_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/login_char_flow_test.rs) & [`tests/inventory_storage_type_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/inventory_storage_type_test.rs): Cập nhật baseline frame count phản ánh gói `1D 04` login sync bắt buộc.
11. [`.scratch/economy-refactor/issues/08-economy-review-fixes.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/economy-refactor/issues/08-economy-review-fixes.md): Cập nhật trạng thái `completed`.

---

## 5. Hướng Dẫn Cho Agent Tiếp Theo (Next Steps & Suggested Skills)

### Kế hoạch tiếp theo (Next Steps):
- Đăng nhập thử nghiệm trực tiếp bằng Client game thực tế (`aLogin.exe`) để kiểm tra tương tác UI thực: mở quầy hàng (Stall), mua hàng từ quầy người chơi khác, cất/rút kho Tiền Trang và Lưu Lãng, giao dịch P2P giữa 2 client.
- Kiểm tra các trường hợp cạnh tranh (concurrency / race condition) khi 2 client cùng bấm mua 1 món hàng tại cùng một thời điểm.

### Suggested Skills:
- **Code Reviewer**: Rà soát lại lần cuối các diff trước khi người dùng thực hiện commit git thủ công.
- **reverse-engineer-anything**: Nếu cần đối chiếu thêm hành vi packet của client `aLogin.exe` hoặc các menu giao dịch khác.
