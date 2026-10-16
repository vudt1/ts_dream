# Báo Cáo Thẩm Định Tính Chính Xác Của Tài Liệu Review Hệ Thống Kinh Tế (08-economy-review-fixes.md)

> **Người thực hiện:** Research Agent (Chuyên gia thẩm định và nghiên cứu mã nguồn)  
> **Thời điểm thẩm định:** 2026-10-16  
> **Tài liệu thẩm định:** `.scratch/economy-refactor/issues/08-economy-review-fixes.md`  
> **Tình trạng git baseline:** HEAD (`da25101`), working tree chưa commit  

---

## 1. Tóm Tắt Kết Quả Thẩm Định

Tổng số hạng mục thẩm định: **17 mục** (A1, B1, B2, B3, B4, C1, C2, C3, D1, D2, D3, D4, D5, E1, E2, E3, F1).

| Kết quả thẩm định | Số lượng | Danh sách các mục |
| :--- | :---: | :--- |
| **ĐÚNG (Verified Correct)** | **14** | **A1**, **B2**, **B3**, **B4**, **C2**, **C3**, **D1**, **D2**, **D3**, **D4**, **D5**, **E1**, **E2**, **F1** |
| **ĐÚNG MỘT PHẦN (Partially Correct)** | **1** | **B1** (Comment sai lệch và rủi ro kiến trúc là đúng; runtime thực tế `persist_sessions_transaction` đã lưu toàn bộ containers do bỏ qua `_tables`) |
| **SAI / CẦN ĐIỀU CHỈNH (Incorrect / Clarified)** | **2** | **C1** (Review nói spec raw u8 là đúng, yêu cầu bỏ `+100` là SAI; Disassembly client chứng minh client trừ 100 khi decode), **E3** (Xác minh code có chủ đích từ kịch bản Eve trong Bear `HotelShopAndSaveMapEffectHandler.cs:13-15`, không phải scope creep vô căn cứ) |

---

## 2. Thẩm Định Chi Tiết Từng Mục Đối Chiếu Nguồn Bậc 1 (Primary Sources)

---

### Nhóm A: Bug Logic Nghiêm Trọng

#### A1. Lệch 1 slot ở quầy người chơi (`17 21`) khiến người mua nhận nhầm vật phẩm
- **Đánh giá thẩm định:** **ĐÚNG 100% (Critical Bug)**
- **Nguồn bậc 1 đối chiếu:**
  1. *Server Emit Catalog (`src/server/handlers/shops.rs:401, 409, 421`):*
     ```rust
     let mut idx = 1u8;
     for listing in seller.shop.items.iter().take(10) {
         ...
         body.push_str(&format!("{:02X}", idx));
         ...
         idx = idx.wrapping_add(1);
     }
     ```
     Server phát trường slot của từng record là `idx = 1, 2, 3...` (1-based index tuần tự).
  2. *Client Decompiled C (`client_pseudo_c/00610290_FUN_00610290.c:227, 330`):*
     Hàm `FUN_00610290` (xử lý S→C `17 21` Catalogue Shop) đọc byte `+0x00` của mỗi record (`uVar15`) và gán thẳng vào struct dòng hiển thị tại `+0x164`:
     ```c
     *(char *)((int)local_8 + uVar6 + 0x164) = (char)(uVar4 >> 0x18);
     ```
  3. *Client Asm Builder (`client_pseudo_c/asm_cs17_783405_builders.asm.txt:1046-1055` tại `0x00784261`):*
     Khi người chơi chọn một dòng `row` trên giao diện và bấm Mua:
     ```assembly
     007842d4: MOVZX EDX, byte ptr [EDX + 0x15c]          ; EDX = row được chọn (0..9)
     007842e5: IMUL  EDX, EDX, 0x11                       ; stride 17 bytes (0x11)
     007842f7: MOV   DL, byte ptr [ECX + EDX*0x1 + 0x164] ; Lấy lại chính byte đã lưu từ server!
     007842fe: CALL  0x00403fa0                           ; Đưa byte này vào payload C→S 17 21
     ```
     Client echo nguyên vẹn byte `idx` (1-based: 1, 2, 3...) mà server đã gửi trong catalog!
  4. *Server Buy Handler (`src/server/handlers/shops.rs:680` & `458-466`):*
     ```rust
     let shop_index = payload[4] as usize; // Client gửi 1 -> shop_index = 1
     // Trong complete_shop_buy:
     let (actual_idx, listing) = if let Some(l) = seller.shop.items.get(shop_index).cloned() {
         (shop_index, l) // get(1) trên Vec 0-based trả về PHẦN TỬ THỨ 2!
     } else if shop_index > 0 && shop_index <= seller.shop.items.len() {
         (shop_index - 1, seller.shop.items[shop_index - 1].clone())
     ...
     ```
- **Hệ quả thực tế:**
  - Người mua bấm mua món hàng thứ 1 trên UI (client gửi `1`), server gọi `seller.shop.items.get(1)` -> Người mua nhận món thứ 2!
  - Người mua bấm món thứ 2 -> Nhận món thứ 3!
  - Chỉ khi mua món hàng cuối cùng (ví dụ danh sách có 3 món, mua món 3), `items.get(3)` trả về `None`, code mới lọt vào nhánh `else if` (`shop_index - 1 = 2`), tình cờ mua đúng món cuối.
- **Đề xuất xử lý:**
  - Quy ước chuẩn hóa: Quầy bán hàng người chơi có tối đa 10 ô (1..10). Trong catalog, phát `slot` (1-based index 1..N hoặc `idx`).
  - Trong `complete_shop_buy`, loại bỏ hoàn toàn `seller.shop.items.get(shop_index)` 0-based. Tra cứu trực tiếp theo 1-based index: `shop_index.checked_sub(1).and_then(|i| seller.shop.items.get(i))`.
  - Xóa bỏ toàn bộ nhánh fuzzy fallback tại `shops.rs:458-466`.

---

### Nhóm B: Hạng Mục Ticket Còn Thiếu / Một Phần So Với Spec

#### B1. Ticket 04 — Persist kho Tiền Trang / Lưu Lãng chưa đủ
- **Đánh giá thẩm định:** **ĐÚNG MỘT PHẦN (Partially Correct)**
- **Nguồn bậc 1 đối chiếu:**
  1. *Comment và code tại `src/server/handlers/trade_storage.rs:711-717`:*
     ```rust
     if !persist::persist_sessions_transaction(
         ctx.env.pool,
         &[&ctx.conn.session],
         // TienTrang / LuuLang have no backing tables (0001 usage audit) —
         // their pouches persist in Session memory only, so just homdo here.
         &["homdo"],
     )
     ```
     Developer ghi chú rõ ràng rằng Tiền Trang và Lưu Lãng "không có backing tables" và chỉ persist trong bộ nhớ session. Nhận định này của developer trong code là **HOÀN TOÀN SAI**.
  2. *Schema SQLite (`migrations/0001_init.sql:91-100`):*
     Bảng `inventories` có cột `storagetype`: `1: HomDo`, `2: TienTrang`, `4: TuiDeo`, `8: TrangBi`, `16: LuuLang`. Cả 5 loại kho đều có cấu trúc lưu trữ hoàn chỉnh.
  3. *Implementation thực tế tại `src/db/persist.rs:295-309` & `src/db/modern/sqlite/session.rs:314-352`:*
     ```rust
     pub async fn persist_sessions_transaction(
         pool: Option<&DbPool>,
         sessions: &[&Session],
         _tables: &[&str], // <--- Bị gạch dưới, HOÀN TOÀN KHÔNG ĐƯỢC DÙNG!
     ) -> bool {
         ...
         repo.save(session).await ...
     }
     ```
     Trong `repo.save(session)`, repository gọi `session.containers()`:
     ```rust
     for (storage, items) in session.containers() { // Bao gồm Bag(1), Bank(2), Secondary(4), Equip(8), Warehouse(16)
         for item in items.iter().filter(|i| i.id > 0) {
             sqlx::query(&insert_inventory_sql)...
         }
     }
     ```
- **Phân tích:**
  - Về mặt runtime hiện tại: `persist_sessions_transaction` thực tế **đã lưu** toàn bộ 5 container (trong đó có cả `tientrang` và `luulang`) do cơ chế lưu toàn diện của `session.save()`.
  - Tuy nhiên, sự sai lệch nằm ở **nhận thức sai của developer**: Developer tưởng rằng DB không hỗ trợ nên chỉ truyền `&["homdo"]`, dẫn đến:
    - Nếu sau này `persist_sessions_transaction` được tối ưu hóa chỉ persist các table được truyền trong slice (granular updates), dữ liệu Tiền Trang và Lưu Lãng sẽ bị mất sạch!
    - Thiếu hoàn toàn unit/integration test kiểm chứng việc cất/rút kho Tiền Trang và Lưu Lãng có persist thật vào SQLite hay không.
- **Đề xuất xử lý:**
  - Sửa comment tại `trade_storage.rs:714-716`.
  - Truyền đầy đủ danh sách `&["homdo", "tientrang", "luulang"]`.
  - Thêm integration test kiểm chứng cất đồ vào Tiền Trang/Lưu Lãng, reload session từ DB và assert item vẫn tồn tại đúng `storagetype`.

#### B2. Ticket 06 — Thiếu timeout `17 3B`
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  1. *Ticket 06 (`.scratch/economy-refactor/issues/06-player-trade-pet-trade-separation.md:12`):*
     Đặc tả yêu cầu: "Khi một trong hai bên bấm nút Hủy **hoặc bị timeout**, server phát gói tin `F4440200173B` tới cả hai client để giải phóng trạng thái giao dịch và đóng form giao diện sạch sẽ."
  2. *Hiện trạng code (`src/server/handlers/trade_storage.rs`):*
     Các vị trí gửi `17 3B` (`lines 318, 371, 414, 499`): Chỉ xuất hiện khi người chơi chủ động bấm nút Hủy (`sub == 2` hoặc `sub == 6`), hoặc khi settle giao dịch thất bại. Hoàn toàn không có bất kỳ dòng code nào kiểm tra thời gian trôi qua, không có trường timestamp `trade_started_at` trong `TradeState`.
  3. *Client Disassembly (`client_pseudo_c/case_020_007902FB_FUN_007902fb.c:254-256` & `03_player_trade.md §3.8`):*
     Opcode `17 3B` hiển thị toast VISCII `0x00797F1C` ("Hủy bỏ giao dịch") thời lượng 2000 ms, là kênh server chủ động đóng form khi giao dịch bị gián đoạn/timeout.
- **Hệ quả thực tế:** Nếu người chơi mở cửa sổ trade rồi treo máy, phiên giao dịch sẽ bị kẹt vĩnh viễn ở trạng thái `trade.active = true`, khóa người chơi không thể giao dịch tiếp.
- **Đề xuất xử lý:** Bổ sung timestamp `trade_start_time` vào `TradeState`. Khi có tick kiểm tra (hoặc khi có tương tác mới quá thời gian cho phép, ví dụ 60s không thao tác), tự động hủy giao dịch, giải phóng trạng thái và gửi `17 3B` cho cả 2 bên.

#### B3. Ticket 06 — Thiếu toast `19 02 <code>` ở nhánh đối phương đang bận
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  1. *Hiện trạng code (`src/server/handlers/trade_storage.rs:217-222`):*
     ```rust
     let Some(mut partner) = registry_get(partner_id) else {
         return; // Return im lặng!
     };
     if partner_id == id || partner.trade.active || ctx.conn.session.trade.active {
         return; // Return im lặng!
     }
     ```
  2. *Ticket 06 (`.scratch/economy-refactor/issues/06-player-trade-pet-trade-separation.md:13`):*
     Yêu cầu: "Trả về các mã toast giao dịch tương ứng theo bảng đặc tả `19 02 <code>` (ví dụ: đối phương đang bận, khoảng cách quá xa, túi đồ không đủ chỗ)." Ticket đã đánh dấu `[x]` (completed) nhưng thực tế chưa làm.
  3. *Client Disassembly Bảng Mã Toast (`client_pseudo_c/0075aff4_FUN_0075aff4.c:50-89` & `03_player_trade.md §3.5`):*
     - `19 02 01`: "Đối tượng đã rớt mạng, không thể giao dịch" (khi `partner` offline).
     - `19 02 02`: "Đối tượng đang bận rộn, không thể giao dịch" (khi `partner.trade.active`).
     - `19 02 05`: "Song phương quang cảnh bất đồng, không thể giao dịch" (khi khác map hoặc quá xa).
     - `19 02 07`: "Đối tượng nào đó vật phẩm trữ tồn quá nhiều, hủy bỏ giao dịch" (khi túi đầy).
- **Hệ quả thực tế:** Khi người chơi gửi lời mời giao dịch tới người đang bận hoặc offline, client hoàn toàn không nhận được phản hồi, không có thông báo toast, gây trải nghiệm người dùng xấu.
- **Đề xuất xử lý:** Thay các lệnh `return;` im lặng bằng việc gửi frame `19 02 <code>` tương ứng (`19 02 01` khi offline, `19 02 02` khi partner đang bận trade).

#### B4. Ticket 02 — `1D 04` lúc login phải gửi vô điều kiện
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  1. *Hiện trạng code (`src/server/spawn.rs:604-606`):*
     ```rust
     if s.bank_gold > 0 {
         frames.push(format!("F44406001D04{}", encoder::le32(s.bank_gold.min(9_999_999))));
     }
     ```
  2. *Spec Nghiệp Vụ (`spec.md:143`):*
     "Đồng bộ tuyệt đối: `1D 04 <bank_gold: u32 LE>` (bắt buộc phải gửi khi vừa login)."
  3. *Client Disassembly (`client_pseudo_c/0072be10_FUN_0072be10.c:38-39` & `06_currency_model.md:22`):*
     Frame `1D 04` ghi trực tiếp giá trị vào ô nhớ `player + 0x12FC` (Bank Gold). Client không tự động reset biến này về 0 nếu server không gửi gói tin `1D 04`.
- **Hệ quả thực tế:** Với nhân vật mới tạo hoặc nhân vật có `bank_gold == 0`, nếu đổi nhân vật (relogin trong cùng phiên game client), client sẽ giữ lại số dư `bank_gold` của nhân vật trước đó trong bộ nhớ UI.
- **Đề xuất xử lý:** Bỏ điều kiện `if s.bank_gold > 0`, luôn luôn phát frame `1D 04` mang giá trị `s.bank_gold` (kể cả khi bằng 0).

---

### Nhóm C: Sai Lệch Wire So Với Spec

#### C1. `giatri_long` cộng 100 không khớp spec
- **Đánh giá thẩm định:** **SAI (REVIEW HIỂU NHẦM / SPEC THIẾU THÔNG TIN WIRE)**
- **Nguồn bậc 1 đối chiếu:**
  1. *Client Decompiled C (`client_pseudo_c/00610290_FUN_00610290.c:663-668` và `683-688`):*
     Tại hàm `FUN_00610290` (chính là hàm decode và hiển thị quầy bán hàng `17 21`):
     ```c
     uVar5 = (uint)*(byte *)((int)local_8 + uVar5 + 0x16e); // Đọc byte thuộc tính ép lông/ngọc
     uVar6 = uVar5 - 100;                                   // <--- CLIENT BẮT BUỘC TRỪ ĐI 100!
     if (SBORROW4(uVar5, 100)) {
         uVar6 = _IntOver(uVar6);
     }
     IntToStr(uVar6, (int *)&local_98);                     // Chuyển thành chuỗi để in lên tooltip
     _LStrCat((int *)&local_44, local_98);
     ```
  2. *C# Server Gốc (`TS_Server_Bear/TS_Server/Client/TSItemContainer.cs:153`):*
     ```csharp
     packetCreator.addByte((byte)(100 + (byte)(items[i].equip?.other_val ?? 0)));
     ```
  3. *Code chuẩn hóa hiện tại của server (`src/server/inventory.rs:17-23`):*
     ```rust
     pub const GIA_TRI_LONG_BIAS: u16 = 100;
     pub fn biased_giatri_long(raw: u8) -> u8 {
         let clamped = raw.min(GIA_TRI_LONG_MAX);
         ((u16::from(clamped) + GIA_TRI_LONG_BIAS) & 0xFF) as u8
     }
     ```
- **Phân tích:**
  - Spec §1.2 ghi `giatri_long: u8` là tên trường dữ liệu trừu tượng.
  - Trên Wire, giao thức client TS Online quy định các giá trị thuộc tính ép lông/ngọc có bias `+100`. Phía client khi hiển thị luôn trừ đi 100 (`value - 100`).
  - Nếu sửa theo yêu cầu của Review C1 (bỏ `+100`, phát raw u8), một món đồ có `giatri_long = 0` sẽ bị client giải mã thành `0 - 100 = -100` (hoặc tràn số `156`), làm sai lệch hoàn toàn giao diện game.
- **Đề xuất xử lý:**
  - **GIỮ NGUYÊN logic `+100`**.
  - Đính chính tài liệu Spec §1.2: Ghi rõ trường wire `giatri_long` mang giá trị `biased_giatri_long` (`raw + 100`).
  - Trong `shops.rs:416` và `shops.rs:754`, thay vì tự viết công thức `(100u16.wrapping_add(...)) & 0xFF`, hãy gọi hàm chuẩn `crate::server::inventory::biased_giatri_long(item.giatri_long)` để đồng nhất codebase.

#### C2. NPC shop mua `count == 0` bị ép thành 1
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  1. *Hiện trạng code (`src/server/handlers/shops.rs:228`):*
     ```rust
     let count = if count == 0 { 1 } else { count };
     ```
  2. *Đối chiếu Player Shop (`src/server/handlers/shops.rs:467-469`):*
     ```rust
     if count == 0 {
         return Err(ShopBuyError::InvalidIndex);
     }
     ```
  3. *Bảng mã lỗi client `1B 01` (`02_buy_sell_transaction.md §7.2` & `case_024_00791D80_FUN_00791d80.c`):*
     Client có các mã lỗi `1B 01 03` ("Số lượng vật phẩm bạn mua đã đầy") và `1B 01 04` ("Hệ thống gián đoạn giao dịch").
- **Hệ quả thực tế:** Nếu packet truyền `count == 0`, server tự động ép mua 1 món và trừ tiền người chơi.
- **Đề xuất xử lý:** Loại bỏ dòng ép `count = 1`. Nếu `count == 0`, từ chối giao dịch và gửi mã lỗi `NPC_SHOP_BUY_DISRUPTED` (`1B 01 04`).

#### C3. Hằng số stall bị gán nhãn sai (0x1F vs 0x20)
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  1. *Hiện trạng code (`src/protocol/mod.rs:228-229`):*
     ```rust
     pub const ITEM_SUB_STALL_BROADCAST: u8 = 0x1F; // 31
     pub const ITEM_SUB_STALL_CLOSE: u8 = 0x20;     // 32
     ```
  2. *Spec Nghiệp Vụ (`spec.md:16-19`):*
     - `17 1F` (`0x0078418C`): Đóng quầy bán hàng (Close Stall).
     - `17 20` (`0x007841E1`): Xem quầy của người khác (View Stall).
  3. *Handler Thực Tế (`src/server/handlers/shops.rs:640-666`):*
     Handler match case 31 (`0x1F`) để đóng shop (`shop.active = false`) và match case 32 (`0x20`) để mở catalogue shop của người khác (`open_shop_id = vendor_uid`).
- **Hệ quả thực tế:** Hằng số trong `protocol/mod.rs` bị đảo ngược ý nghĩa hoàn toàn so với binary client và logic của handler. Do tên bị sai, developer trong `shops.rs` không dám dùng hằng số mà phải dùng số raw `31`, `32`.
- **Đề xuất xử lý:**
  - Đổi tên `ITEM_SUB_STALL_CLOSE = 0x1F;` (31) và `ITEM_SUB_STALL_VIEW = 0x20;` (32).
  - Sử dụng trực tiếp các hằng số này trong `shops.rs` thay cho số raw.

---

### Nhóm D: Dọn Dẹp Standards / Trùng Lặp

#### D1. Gộp `GOLD_CAP` (đang trùng 3 nơi)
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - Khai báo 1: `src/protocol/mod.rs:29` (`pub const GOLD_CAP: u32 = 9_999_999;`).
  - Khai báo 2: `src/server/handlers/trade_storage.rs:13` (`pub(crate) const GOLD_CAP: u32 = 9_999_999;`).
  - Khai báo 3: `src/server/trade_system.rs:18` (`pub(crate) const GOLD_CAP: u32 = 9_999_999;`).
  - `shops.rs:12, 354, 506` phải tham chiếu dài dòng qua `crate::server::handlers::trade_storage::GOLD_CAP`.
  - `spawn.rs:599, 605` dùng literal hardcode `9_999_999`.
- **Đề xuất xử lý:** Giữ duy nhất `ts_dream::protocol::GOLD_CAP` làm hằng canonical. Mọi nơi khác import và sử dụng hằng này.

#### D2. Gộp frame builder trùng lặp (`gold_frame` vs `gold_status_frame`)
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - `src/server/handlers/shops.rs:11-18`:
    ```rust
    pub fn gold_frame(gold: u32, shop_point: u32) -> String {
        let clamped_gold = gold.min(crate::server::handlers::trade_storage::GOLD_CAP);
        format!("F4440A001A04{}{}", encoder::le32(clamped_gold), encoder::le32(shop_point))
    }
    ```
  - `src/server/handlers/trade_storage.rs:124-130`:
    ```rust
    fn gold_status_frame(gold: u32, shop_point: u32) -> String {
        format!("F4440A001A04{}{}", encoder::le32(gold.min(GOLD_CAP)), encoder::le32(shop_point))
    }
    ```
- **Đề xuất xử lý:** Xóa `gold_status_frame` trong `trade_storage.rs`, sử dụng chung `ts_dream::server::handlers::shops::gold_frame` (hoặc chuyển vào `protocol::codecs`).

#### D3. Dọn hằng số chết (Speculative Generality)
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - Trong `src/protocol/mod.rs:149-236`: Các khối hằng `TRADE_SUB_*`, `BANK_SUB_*`, `STORAGE_SUB_*`, `ITEM_SUB_*` có 0 lượt tham chiếu bên ngoài file khai báo.
  - Đồng thời trộn lẫn hệ cơ số: `TRADE_SUB_TRANSFER_ITEM = 20;`, `ITEM_SUB_WAREHOUSE_DEPOSIT = 51;` (thập phân) nằm cạnh `0x0A`, `0x1E` (hexadecimal).
- **Đề xuất xử lý:** Thống nhất định dạng hexadecimal (`0x14`, `0x33`...), thay thế toàn bộ số raw trong các match arm của `shops.rs` và `trade_storage.rs` bằng các hằng số này.

#### D4. Đưa các frame hardcode vào helper
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - Chuỗi `"F44402001B03"` lặp lại 5 lần trong `src/server/handlers/talk.rs` (dòng 503, 737, 748, 768, 779).
  - Chuỗi `"F4440200173B"` lặp lại 8 lần trong `src/server/handlers/trade_storage.rs` (dòng 318, 321, 371, 374, 414, 417, 499, 502).
- **Đề xuất xử lý:** Tạo các helper function: `open_npc_shop_frame()` và `trade_cancelled_frame()`.

#### D5. Đặt tên `vendor_uid` không nhất quán với CONTEXT.md
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - `CONTEXT.md:16`: Yêu cầu chuẩn hóa thuật ngữ định danh, sử dụng `player_id`, tránh các từ mơ hồ hoặc không thuộc ubiquitous language.
  - `src/server/handlers/shops.rs:395, 670`: Đặt tên `vendor_uid`.
- **Đề xuất xử lý:** Đổi tên biến thành `seller_id` hoặc `player_id`.

---

### Nhóm E: Scope Creep & Fuzzy Logic

#### E1. `resolve_npc_shop_item` quét toàn bộ surface
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - `src/server/handlers/shops.rs:173-183`:
    ```rust
    for surface in scene.surface_datas.values() {
        if let Some(sentence) = surface.sentences.get(&item_index) {
            if sentence.data_kind == 2 {
                ...
                return Some((item_id, price));
            }
        }
    }
    ```
- **Hệ quả thực tế:** Khi tương tác mua hàng, nếu không tìm thấy trong surface hiện tại, server lặp qua toàn bộ các surface khác trong map. Nếu trên cùng một bản đồ có nhiều NPC bán hàng (ví dụ tiệm thuốc và tiệm vũ khí), việc quét toàn bộ `surface_datas.values()` theo thứ tự ngẫu nhiên sẽ dẫn tới việc người chơi mua nhầm vật phẩm của NPC khác trên cùng map.
- **Đề xuất xử lý:** Thu hẹp phạm vi tìm kiếm: Chỉ tra cứu theo surface gắn với NPC đang tương tác (`conn.session.current_event_session.last_surface_id` hoặc surface tương ứng với `conn.session.idtalking`). Loại bỏ vòng lặp fallback quét mù toàn map.

#### E2. Fuzzy index 3 nhánh trong `complete_shop_buy`
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  - `src/server/handlers/shops.rs:458-466`:
    Nhánh 1: `seller.shop.items.get(shop_index)` (0-based)
    Nhánh 2: `seller.shop.items[shop_index - 1]` (1-based)
    Nhánh 3: `find(|(_, l)| l.slot == shop_index as u8)` (slot matching)
- **Phân tích:** Nhánh 1 nuốt chửng chỉ số 1-based của client, trực tiếp gây ra bug A1.
- **Đề xuất xử lý:** Xóa bỏ toàn bộ 3 nhánh fuzzy. Thay bằng 1 phép tra cứu duy nhất theo 1-based index sau khi đã chuẩn hóa ở A1.

#### E3. `talk.rs:502-503` `result.parameter == 3 => "1B03"`
- **Đánh giá thẩm định:** **XÁC MINH: ĐÂY LÀ CODE CÓ CHỦ ĐÍCH (Intentional), KHÔNG PHẢI SCOPE CREEP VÔ CĂN CỨ**
- **Nguồn bậc 1 đối chiếu:**
  - `src/server/handlers/talk.rs:501-504`:
    ```rust
    // parameter 3: open NPC shop form (Bear HotelShopAndSaveMapEffectHandler)
    if result.parameter == 3 {
        out.send("F44402001B03");
    }
    ```
  - C# Server Gốc (`TS_Server_Bear/TS_Server/Client/QuestStepHelper/HotelShopAndSaveMapEffectHandler.cs:9-15`):
    ```csharp
    if (packMode.GetHotel_NpcShops) {
        switch (packMode.GetHotel_NpcShops_Type) {
            case 3:
                client.reply(new PacketCreator(27, 3).send()); // 27 = 0x1B, 3 = 0x03 -> 1B 03!
                break;
    ```
- **Phân tích:** Trong kịch bản Eve của game, kết quả sự kiện loại 7 (Eve effect 7) với parameter 3 là lệnh kích hoạt mở form shop trực tiếp mà không cần qua menu đối thoại thông thường.
- **Đề xuất xử lý:** **GIỮ LẠI** nhánh logic này. Chuyển sang gọi helper `open_npc_shop_frame()` và bổ sung doc comment dẫn chiếu nguồn gốc từ `HotelShopAndSaveMapEffectHandler` để tránh hiểu nhầm trong các lần review sau.

---

### Nhóm F: Kiểm Thử (Ticket 07 — Độ Tin Cậy)

#### F1. Bổ sung đối chiếu byte-exact với client thực
- **Đánh giá thẩm định:** **ĐÚNG 100%**
- **Nguồn bậc 1 đối chiếu:**
  1. *Hiện trạng `tests/economy_wire_test.rs`:*
     - Dòng 73-76, 84, 100: Sử dụng chính `format!("F4440A001A04{}{}", encoder::le32(...))` để assert kết quả (code test tự sinh chuỗi bằng cùng encoder với production rồi tự so sánh với chính nó).
     - Dòng 856: Trong test mua shop người chơi, test gọi:
       `complete_shop_buy(&mut buyer, &mut seller, 0, 2)` -> Test tự truyền index `0` (0-based) trực tiếp vào hàm nội bộ, bypass hoàn toàn luồng wire opcode `17 21` từ client (vốn gửi 1-based), che giấu bug A1.
     - Thiếu test cho việc persist kho Tiền Trang (`storagetype = 2`) và Lưu Lãng (`storagetype = 16`) sau khi cất/rút đồ.
- **Đề xuất xử lý:**
  - Bổ sung các test assert bằng literal hex cố định (golden packets).
  - Viết test end-to-end cho opcode `17 21`: Client gửi payload wire `17 21 <seller_id> 01 02` (slot 1, count 2) -> Khẳng định mua đúng vật phẩm của slot 1.
  - Viết test kiểm tra tính toàn vẹn của bảng `inventories` trong SQLite in-memory sau các thao tác cất/rút Tiền Trang và Lưu Lãng.

---

## 3. Khuyến Nghị Hành Động Cho Developer Thực Hiện Fixes

1. **Khắc phục Bug Logic A1 & Xóa Fuzzy E2:**
   - Trong `shops.rs`: Sửa `complete_shop_buy` tra cứu nhất quán 1-based index (`shop_index - 1`). Xóa sạch 3 nhánh fuzzy.
2. **Bảo tồn Wire Encoding C1:**
   - **Tuyệt đối không bỏ `+100`** của `giatri_long`. Dùng `crate::server::inventory::biased_giatri_long`.
3. **Bổ sung các phản hồi wire còn thiếu (B2, B3, B4, C2):**
   - B4: Bỏ `if s.bank_gold > 0`, luôn phát `1D 04`.
   - C2: Reject `count == 0` khi mua shop NPC bằng `1B 01 04`.
   - B3: Gửi `19 02 01` và `19 02 02` khi đối phương offline hoặc đang bận trade.
   - B2: Thêm thời gian timeout giao dịch và phát `17 3B` khi hết hạn.
4. **Chuẩn hóa Hằng số & Refactor Clean Code (C3, D1, D2, D3, D4, D5, E1, E3):**
   - Đổi tên `ITEM_SUB_STALL_CLOSE = 0x1F`, `ITEM_SUB_STALL_VIEW = 0x20`.
   - Gộp `GOLD_CAP` về `protocol::GOLD_CAP`.
   - Dùng chung `gold_frame`.
   - Tạo helper `open_npc_shop_frame()` và `trade_cancelled_frame()`.
   - Giữ lại `talk.rs:502-503` (Eve effect 7 param 3) kèm doc comment.
5. **Nâng cấp Test Suite `tests/economy_wire_test.rs` (F1):**
   - Thêm test case cho bug A1 (client gửi slot 1 -> mua đúng item 1).
   - Thêm test persist DB cho Tiền Trang (`storagetype = 2`) và Lưu Lãng (`storagetype = 16`).
