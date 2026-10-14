# Kế Hoạch Triển Khai: Chuẩn Hoá Container Vật Phẩm & Chỉ Số — Rust Server `ts_dream`

> Ngày lập: 2026-10-03
> Nguồn tài liệu gốc: [`../post_login_stats_inventory_flow.md`](../post_login_stats_inventory_flow.md)
> Phạm vi: luồng sau đăng nhập — xác nhận chỉ số (`05 03`, `08 01`) và túi đồ (5 container `inventories`)

---

## 1. Mục đích

Bộ tài liệu này là **kế hoạch triển khai** (implementation plan), không phải tài liệu mô tả hiện
trạng. Nó gộp ba nguồn bằng chứng:

| Nguồn | Ký hiệu | Vai trò |
|---|---|---|
| `client_pseudo_c/` + `test/ts_decompile/` | `[C]` | decompile client `aLogin.exe` — **chuẩn tuyệt đối** cho layout wire |
| `TS_Server_Bear/` (C#) | `[B]` | server tham chiếu — xác nhận ngữ nghĩa, giá trị, thứ tự khung login |
| `src/` hiện tại | `[R]` | mã Rust — đối chiếu hiện trạng |
| `.scratch/client-pseudo-op-code/*` | `[D]` | tài liệu nghiên cứu đã có trong repo |

Khi `[C]` và `[B]` xung đột → **ưu tiên `[C]`** vì client là bên duy nhất quyết định byte có được
parse đúng hay không.

---

## 2. Kết luận điều hành (TL;DR)

Bảy phát hiện quan trọng nhất, xếp theo mức độ:

| # | Mức | Phát hiện | Nguồn |
|:-:|:--:|---|---|
| 1 | 🔴 **P0** | `17 0B` (trang bị) đang serialize **12 byte**, client chỉ đọc **10 byte** và **không có byte slot**. Mọi equipment record từ thứ 2 trở đi đều lệch. | `[C-verified]` `IDIV 0x0A` tại `0x0077CC6E`; `[B]` `TSCharacter.cs:5283-5295` |
| 2 | 🔴 **P0** | `17 66` (lưu lang / kho ngựa `form_Horse`) **đã được giải mã**: trỏ vào `player+0xF74`, sức chứa **10 ô** (`1..=10`, `_BoundErr` nếu > 10), stride **12 byte**, **có byte slot**, không clear pass. Sub `17 67` cập nhật 1 ô, `17 68` xóa 1 ô. | `[C-verified]` `FUN_0076a320` (`0x0076A320`), `0x0062F6FC`, `0x0076AF6C` |
| 3 | 🔴 **P0** | `load_items` chỉ đọc **4/22 cột** DB, `save` chỉ ghi **6/22 cột** rồi `DELETE` toàn bảng ⇒ **16 cột affix/enhance/lock mất mỗi lần auto-save**. | `[R]` `sqlite/session.rs:176-199`, `:299-314` |
| 4 | 🟠 **P1** | Template field (`loai`, `lv`, `int1..fai2`) **không được hydrate** khi load ⇒ sau relog không mặc được đồ (`loai = 0`, gate `1..=6` fail) và gear bonus = 0. Đây là nguyên nhân gốc, không chỉ là thiếu `recompute_stats()`. | `[R]` `load_items` không gọi `from_template`; `handlers/inventory.rs:292` |
| 5 | 🟠 **P1** | `08 01` prop-code mapping **đã được xác minh qua client**: `0x19`/`0x1A` = HP/SP hiện tại, `0xCD`/`0xCE` = MaxHP/MaxSP ghi đè trực tiếp, `0xCF`/`0xD0` = HP2/SP2 bonus phẳng, `0x1F`/`0x20` = HPX/SPX bonus điểm. | `[C-verified]` `FUN_0078d125` & `FUN_00710ab0`; `[B]` `TSConstants.cs:5-67` |
| 6 | 🟠 **P1** | `1E 01` parser **không có clear pass** (khác `17 05`) ⇒ ô cũ không tự xoá sau khi rút bớt item. Cần gửi frame `1E 08` để mass-clear trước khi dump. | `[C-verified]` `0076BFC8` có clear loop, `007707A0` không |
| 7 | 🟠 **P1** | `05 03` tail **đã làm rõ nguyên nhân lệch skill**: header cố định đúng 112 byte (chứa 5×500 + **đúng 43 byte zero**). Skill 3B bắt đầu từ byte 113. Rust gửi 90 byte zero (dư 47 byte) làm client parse lệch off-by-two toàn bộ skill list. Không có block 32B cuối. | `[C-verified]` `FUN_0072e9a0` (`0072e9a0_FUN_0072e9a0.c:310-420`) |
| 8 | 🟡 **P2** | Mapping `storagetype` bị **nhân bản 6 nơi**; sức chứa `HOMDO_SLOTS=25` bị dùng cho cả túi 50 ô và kho lưu lãng 10 ô. | `[R]` `model.rs`, `persist.rs`, `sqlite/session.rs`×3 |

**Đánh giá tổng:** kiến trúc hiện tại **đúng về hình dạng** (một bảng `inventories`, 5 `storagetype`,
`dump_*` tách bạch) nhưng **sai về byte và độ tròn đầy**.

- ✅ **`17 05`** — Hòm đồ: 25 ô, 12B, có slot byte, có clear loop.
- ✅ **`1E 01`** — Tiền trang: 50 ô, 12B, có slot byte, clear bằng `1E 08`.
- ✅ **`17 2F`** — Túi đeo: **Đã xác minh** (`FUN_00769b68`), mảng `player+0xF0C`, 25 ô, 12B, có slot byte, xóa lẻ bằng `17 31`.
- 🔴 **`17 0B`** — Trang bị: sai stride (phải là 10B, KHÔNG có byte slot).
- ✅ **`17 66`** — Lưu lãng: **Đã xác minh** (`FUN_0076a320`), mảng `player+0xF74`, 10 ô, 12B, có slot byte, xóa lẻ bằng `17 68`.

**Lộ trình tối thiểu đóng P0: 2-4 ngày, KHÔNG cần capture** (Phase 1 + 2A + 3).
Toàn bộ 3 gap từng gắn nhãn "cần capture" (G-02, G-06, G-07) nay **đều đã được giải quyết trực tiếp qua decompile binary `aLogin.exe`**.

---

## 3. Cấu trúc bộ tài liệu

| File | Nội dung | Đọc khi nào |
|---|---|---|
| [`01-container-mapping.md`](./01-container-mapping.md) | **Bảng mapping chuẩn**: khái niệm ↔ `storagetype` ↔ opcode ↔ client array ↔ stride ↔ layout byte. Kèm **ma trận 3 chiều DB ↔ Session ↔ Frame**. Đây là tài liệu chuẩn hoá mọi phase sau đều trích dẫn. | Trước khi code |
| [`02-gap-analysis.md`](./02-gap-analysis.md) | 20 gap (G-01..G-20) với hiện trạng / chuẩn / khoảng cách / mức độ. Bảng đối chiếu nhanh Rust-vs-Bear-vs-client. | Sau khi đọc 01 |
| [`03-wire-codecs.md`](./03-wire-codecs.md) | Đặc tả sửa serializer: `17 0B` 10B, `1E 08` clear, `17 1E` stall, `05 03` tail, `08 01` prop codes, `28 01` hotkey. | Khi làm Phase 2 |
| [`04-db-persistence.md`](./04-db-persistence.md) | Thiết kế DB chuẩn: 22 cột `inventories`, hydrate template từ `Item.dat`, gom `StorageType`, xử lý `ThingData`. | Khi làm Phase 3 |
| [`05-phased-roadmap.md`](./05-phased-roadmap.md) | Roadmap 6 phase, mỗi phase có task cụ thể + tiêu chí nghiệm thu + rủi ro + rollback + ước lượng effort. | Khi lên kế hoạch sprint |
| [`06-tests-and-capture.md`](./06-tests-and-capture.md) | Chiến lược test 5 tầng (T1-T5) + checklist capture để đóng 3 gap chỉ giải được bằng client thật. | Khi làm Phase 5 |

---

## 4. Quy ước bằng chứng

Mỗi claim trong bộ tài liệu đều gắn nhãn:

| Nhãn | Ý nghĩa |
|---|---|
| `[C-verified]` | Có body/ASM client, đọc được từng byte |
| `[C-call-only]` | Chỉ có call-site trong dispatcher, **chưa có body** ⇒ cấm suy ra layout |
| `[B-verified]` | Có code C# Bear (`TS_Server_Bear/`) |
| `[R]` | Quan sát trong mã Rust hiện tại (`src/`) |
| `[D]` | Tài liệu nghiên cứu đã có trong repo |
| `[ASSUMPTION]` | Giả định **cần capture** để xác nhận |

> **Nguyên tắc bất di bất dịch:** cấm dùng `case ...: break` rỗng của Ghidra làm bằng chứng client
> "không gửi" — đây là lỗi recovery đã gặp nhiều lần (`post_login_stats_inventory_flow.md` §7.5).

---

## 5. Ba gap trước đây tưởng phải capture — Nay đã đóng bằng Decompile Client

| Gap | Trạng thái cũ | Kết quả giải mã từ `aLogin.exe` |
|---|---|---|
| **G-02** `17 66` | Thiếu body `FUN_0076A320` | ✅ `FUN_0076A320` (`0x0076A320`) kết nối `form_Horse`: kho xe/ngựa 10 ô (`player+0xF74`), 12B stride, có slot byte. Xóa lẻ qua `17 68`. |
| **G-06** `08 01` prop code | Chưa có client parser `FUN_0078D125` | ✅ `FUN_0078D125` + `FUN_00710AB0`: xác nhận bảng mã prop code chính xác 100%. `0x19`/`0x1A` = HP/SP hiện tại, `0xCD`/`0xCE` = MaxHP/MaxSP, `0xCF`/`0xD0` = HP2/SP2 bonus phẳng. |
| **G-07** `05 03` tail | Chưa biết client đọc fixed hay tới cuối | ✅ `FUN_0072E9A0`: header cố định đúng 112 bytes (chứa 43 zero bytes), skill bắt đầu tại byte 113 (3B/skill). Client đọc tới cuối buffer, không có block 32B cuối. |

Toàn bộ thông tin cần thiết để viết wire codecs chuẩn đã đầy đủ mà không cần chờ capture từ server sống.

---

## 6. Thứ tự đọc đề xuất

```
01 (mapping chuẩn + ma trận 3 chiều)
  └─▶ 02 (20 gap)              ← biết cái gì đang hỏng
        ├─▶ 03 (wire codec)     ← sửa byte sai  (Phase 2A/2B)
        ├─▶ 04 (db persist)     ← sửa persist sai (Phase 3)
        └─▶ 05 (roadmap)        ← gộp thành kế hoạch sprint
              └─▶ 06 (test + capture) ← đóng gap mở
```

Phase 2A và Phase 3 **có thể chạy song song** (không cùng đụng file).
Phase 2B **phải đợi** Phase 0. Xem [`05-phased-roadmap.md`](./05-phased-roadmap.md) §6.

---

## 7. Ngoài phạm vi

- Chỉ số pet (`0F 08/14/0A/12/13`, `08 02`) — chỉ nhắc khi liên quan `17 11`
- Trade P2P (`19 01..0C`) — dùng chung `StorageType` nhưng không sửa
- NPC shop / stall economy (`economy/07_rust_economy_audit.md` G1..G40) — chỉ sửa phần liên quan container
- Quest sync (`18 01..08`) — dùng bảng `character_quest_items` riêng
- Battle engine, Eve engine, web admin

---

## 8. Liên kết tài liệu gốc

| Tài liệu | Vai trò |
|---|---|
| [`../post_login_stats_inventory_flow.md`](../post_login_stats_inventory_flow.md) | Nghiên cứu luồng sau login (đầu vào của kế hoạch này) |
| [`../opcode_1e.md`](../opcode_1e.md) | 11 subop của main `0x1E` (có body đầy đủ) |
| [`../opcode_17.md`](../opcode_17.md) | 97 subop của main `0x17` |
| [`../opcode_28.md`](../opcode_28.md) | Hotkey `28 01` |
| [`../opcode_05.md`](../opcode_05.md) | Stats `05 03` |
| [`../economy/07_rust_economy_audit.md`](../economy/07_rust_economy_audit.md) | 40 gap economy (chồng lấn G17/G18/G33) |
| [`../../spec/server_main_opcode.md`](../../spec/server_main_opcode.md) | Bảng main opcode chuẩn |
| `TS_Server_Bear/` | Server tham chiếu C# (nguồn `[B]`) |
| `client_pseudo_c/` | Decompile `aLogin.exe` (nguồn `[C]`) |