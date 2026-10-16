# 08: Khắc Phục Các Phát Hiện Sau Code Review Hệ Thống Kinh Tế (Tickets 01–07)

**What to build:** Sửa toàn bộ các phát hiện còn tồn đọng sau khi rà soát (review) 7 ticket kinh tế đã hoàn thành. Bao gồm 1 bug logic nghiêm trọng (lệch slot quầy người chơi gây mua nhầm vật phẩm), 4 hạng mục ticket còn thiếu/một phần so với spec, các sai lệch wire so với spec, và dọn dẹp hằng số/hàm trùng lặp. Công việc đang nằm ở working tree chưa commit trên nền `HEAD` (`da25101`).

**Blocked by:** 01, 02, 03, 04, 05, 06, 07 (tất cả đã hoàn thành).

**Status:** completed

**Nguồn đối chiếu:**
- Spec: `.scratch/economy-refactor/spec.md` (và `plan_economy_reverse_engineering.md`).
- Ticket gốc: `.scratch/economy-refactor/issues/01..07`.
- Handoff (có vài tuyên bố chưa khớp thực tế, cần đính chính): `.scratch/economy-refactor/handoff.md`.

> **Bỏ qua:** mâu thuẫn giữa `TESTS.md` (`cargo test --lib ...`) và `AGENTS.md` (cấm `#[cfg(test)]` trong `src/`) — không quan trọng, không xử lý trong ticket này.

---

## A. Bug logic nghiêm trọng

- [x] **A1. Lệch 1 slot ở quầy người chơi (`17 21`) khiến người mua nhận nhầm vật phẩm.**
  - Hiện trạng: `player_shop_catalog_frame` (`src/server/handlers/shops.rs:397` hàm, dòng emit `idx` tại `shops.rs:409`) phát "slot" là `idx` tuần tự 1..N. Nhưng `handle_player_shop` sub 33 lấy `shop_index = payload[4] as usize` (`shops.rs:672`) rồi `complete_shop_buy` gọi `seller.shop.items.get(shop_index)` (0-based) tại `shops.rs:458-459`.
  - Hệ quả: client echo slot 1 (record đầu) → `get(1)` trả **record thứ 2**. Chỉ record cuối tình cờ đúng.
  - Spec §1.2 offset `+0x00` ghi rõ `slot: u8` (vị trí ô 1..10), và §2.2 "tra cứu `index` ... Lấy `data_id` làm `item_id`".
  - Yêu cầu:
    - [x] Phát **đúng `listing.slot`** thực tế thay vì `idx` tuần tự trong `player_shop_catalog_frame` (khớp spec §1.2).
    - [x] Hoặc, nếu giữ `idx` làm slot hiển thị 1..N, sửa `complete_shop_buy` để tra cứu nhất quán 1-based ↔ đúng record (không dùng `get(shop_index)` 0-based).
    - [x] Chọn **một** quy ước duy nhất cho slot catalog ↔ slot mua, xoá nhánh fuzzy (xem B1).
    - [x] Bổ sung test khẳng định: catalog phát slot X → mua slot X trả đúng `item_id` của listing X (kể cả khi `shop.items` không liền mạch/không theo thứ tự).

---

## B. Hạng mục ticket còn thiếu / một phần so với spec

- [x] **B1. Ticket 04 — Persist kho Tiền Trang / Lưu Lãng chưa đủ.**
  - Hiện trạng: `handle_storage_transfer` chỉ persist `&["homdo"]`; comment tại `src/server/handlers/trade_storage.rs:711-717` nói "TienTrang / LuuLang have no backing tables".
  - Thực tế schema **có** `inventories.storagetype` (`migrations/0001_init.sql:99`: `2=TienTrang`, `16=LuuLang`) và repository `src/db/modern/sqlite/inventories.rs` đã hỗ trợ theo `storage_type`.
  - Yêu cầu:
    - [x] Persist atomic thêm `tien_trang` (`storagetype = 2`) và `luu_lang` (`storagetype = 16`) trong cùng transaction với `homdo`.
    - [x] Đính chính lại tuyên bố sai trong `handoff.md`.
    - [x] Test: cất/rút qua Tiền Trang & Lưu Lãng, restart/đọc lại DB vẫn còn đồ (không mất/nhân bản).

- [x] **B2. Ticket 06 — Thiếu timeout `17 3B`.**
  - Spec/ticket: "hủy **hoặc quá thời gian timeout** → server phát `F4440200173B` tới cả hai client".
  - Hiện trạng: `17 3B` chỉ gửi khi hủy tay / settle thất bại (`trade_storage.rs:318,371,414,499`); không có đường timeout.
  - Yêu cầu:
    - [x] Thêm cơ chế timeout giao dịch (đề xuất: mốc thời gian bắt đầu/khoá giao dịch trong `TradeState`, kiểm tra/đóng ở tick nền hoặc khi có sự kiện) và phát `17 3B` cho cả hai bên khi hết hạn.
    - [x] Đảm bảo giải phóng khoá phiên + hoàn trả tài sản đã "đặt" trước khi đóng form.

- [x] **B3. Ticket 06 — Thiếu toast `19 02 <code>` ở nhánh đối phương đang bận.**
  - Hiện trạng: `handle_trade` sub 1 return **im lặng** khi `partner_id == id || partner.trade.active || ctx.conn.session.trade.active` (`trade_storage.rs:221`).
  - Ticket 06 checkbox 5: trả mã toast `19 02 <code>` (đối phương đang bận, quá xa, túi không đủ chỗ).
  - Yêu cầu:
    - [x] Gửi `19 02 <code>` tương ứng cho **mọi** nhánh thất bại (bận, tự giao dịch với mình, quá xa, túi đầy), không return im lặng.

- [x] **B4. Ticket 02 — `1D 04` lúc login phải gửi vô điều kiện.**
  - Spec §2.1: "`1D 04 <bank_gold: u32 LE>` (bắt buộc phải gửi khi vừa login)".
  - Hiện trạng: `src/server/spawn.rs:604` chỉ gửi `if s.bank_gold > 0`.
  - Yêu cầu:
    - [x] Luôn gửi `F44406001D04<bank_gold>` khi spawn, kể cả `bank_gold == 0`.

---

## C. Sai lệch wire so với spec

- [x] **C1. `giatri_long` cộng 100 không khớp spec.**
  - Spec §1.2 offset `+0x09`: `giatri_long: u8` là **raw u8**.
  - Hiện trạng: `shops.rs:418` và `shops.rs:754` ghi `(100u16.wrapping_add(u16::from(item.giatri_long))) & 0xFF`. (Dòng này có từ `HEAD`, nhưng nằm trong frame catalog vừa viết lại.)
  - Yêu cầu:
    - [x] Xác nhận lại với client decode (`aLogin.exe`) rồi sửa đúng raw u8 nếu spec đúng; ghi chú rõ căn cứ (nếu giữ `+100` phải có bằng chứng Level A).

- [x] **C2. NPC shop mua `count == 0` bị ép thành 1.**
  - Hiện trạng: `shops.rs:226` `let count = if count == 0 { 1 } else { count };` — trong khi player shop lại reject `count == 0` (`shops.rs:467`).
  - Yêu cầu:
    - [x] Thống nhất: `count == 0` phải trả mã lỗi phù hợp (`1B 01 xx`), không tự ý nâng lên 1.

- [x] **C3. Hằng số stall bị gán nhãn sai.**
  - Spec §1.1: `0x1F` = Close Stall, `0x20` = View người khác. Hiện trạng `protocol/mod.rs:228-229` gán `ITEM_SUB_STALL_BROADCAST = 0x1F`, `ITEM_SUB_STALL_CLOSE = 0x20` (ngược).
  - Yêu cầu:
    - [x] Đổi tên hằng đúng nghĩa spec và dùng chúng trong handler thay vì số raw `31/32`.

---

## D. Dọn dẹp standards / trùng lặp

- [x] **D1. Gộp `GOLD_CAP` (đang trùng 3 nơi).**
  - `protocol/mod.rs:29` (không được dùng), `trade_storage.rs:13`, `trade_system.rs:18`; thêm raw `9_999_999` tại `spawn.rs:605` và chỗ khác.
  - Yêu cầu: [x] Chỉ giữ **một** hằng canonical, mọi nơi import từ đó.

- [x] **D2. Gộp frame builder trùng lặp.**
  - `gold_frame` (`shops.rs:11`) và `gold_status_frame` (`trade_storage.rs:124`) là cùng `F4440A001A04{}{}`.
  - Yêu cầu: [x] Một hàm dùng chung.

- [x] **D3. Dọn hằng số chết (Speculative Generality).**
  - Các hằng 0 refs: `TRADE_SUB_*`, `MONEY_SUB_*`, `BANK_SUB_*`, `STORAGE_SUB_*`, `ITEM_SUB_*` (`protocol/mod.rs:149–236`); chỉ `NPC_SHOP_SUB_BUY/SELL` + `NPC_SHOP_BUY_SUCCESS` đang dùng.
  - Yêu cầu: [x] Hoặc dùng (thay số raw trong handler + builder) hoặc xoá. Thống nhất một hệ cơ số (hiện `TRADE_SUB_TRANSFER_ITEM = 20`, `ITEM_SUB_WAREHOUSE_DEPOSIT = 51` trộn thập phân với hex).

- [x] **D4. Đưa các frame hardcode vào helper.**
  - `"F44402001B03"` (`talk.rs:503,737,748,768,779`), `"F4440200173B"` lặp 6× (`trade_storage.rs:318,371,414,499…`).
  - Yêu cầu: [x] Tạo helper/builder (ví dụ `open_shop_frame()`, `close_trade_form_frame()`), gọi thay vì chuỗi raw.

- [x] **D5. Đặt tên `vendor_uid` không nhất quán với CONTEXT.md.**
  - CONTEXT.md:16 yêu cầu dùng `player_id`, tránh từ mơ hồ `id`.
  - Yêu cầu: [x] Đổi `vendor_uid` (`shops.rs:395,670`) thành `player_id`/`seller_id` nhất quán.

---

## E. Scope creep cần thu hẹp / xác nhận

- [x] **E1. `resolve_npc_shop_item` quét toàn bộ surface** (`shops.rs:172-186`) trong khi spec nói tra cứu surface của NPC tương ứng. Thu hẹp về đúng surface của NPC; nếu giữ fallback quét toàn cục phải ghi doc comment rõ lý do.
- [x] **E2. Fuzzy index 3 nhánh trong `complete_shop_buy`** (`shops.rs:458-466`) che giấu bug index. Xoá sau khi thống nhất quy ước slot (xem A1).
- [x] **E3. `talk.rs:502-503` `result.parameter == 3 => "1B03"`** ngoài phạm vi menu 30/32 — xác nhận có chủ đích hay bỏ.

---

## F. Kiểm thử (Ticket 07 — độ tin cậy)

- [x] **F1. Bổ sung đối chiếu byte-exact với client thực.**
  - Hiện trạng: `tests/economy_wire_test.rs` assert bằng chính `encoder::le32`/`format!` mà production dùng, **không** dùng harness `golden/`; tính "byte-exact đối chiếu `aLogin.exe`" chưa được chứng minh.
  - Yêu cầu:
    - [x] Thêm golden fixture cho các frame kinh tế (`1B 01/02`, `1D 01/02/04`, `1A 04`, `17 21`, `1E 05`) và assert literal hex cố định.
    - [x] Thêm test cho bug A1 (slot) và B1 (persist kho).

---

## Acceptance Criteria

- [x] `cargo check --all-targets` sạch, 0 warning hồi quy.
- [x] `cargo test --test economy_wire_test --no-fail-fast` pass, có test mới cho A1, B1, B2, B3, B4.
- [x] Không đặt `#[cfg(test)]` inline trong `src/`; mọi test ở `tests/`.
- [x] Cập nhật `handoff.md`: đính chính tuyên bố "Đồng bộ nguyên tử danh mục kho" (chỉ persist `homdo`) và các hạng mục thực tế chưa đủ.
- [x] Không commit source (người dùng tự commit thủ công).
