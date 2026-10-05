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
| 1 | 🔴 **P0** | `17 0B` (trang bị) đang serialize **12 byte**, client chỉ đọc **10 byte** và **không có byte slot**. Mọi equipment record từ thứ 2 trở đi đều lệch. | `[C]` `IDIV 0x0A` tại `0x0077CC6E`; `[B]` `TSCharacter.cs:5283-5295` |
| 2 | 🔴 **P0** | `17 66` (lưu lang) **không có bất kỳ bằng chứng nào** — không có body client, Bear không hề gửi frame này. Rust vẫn đẩy nó mỗi lần login bằng layout 12B giả định. | `[C]` chỉ có call-site `0x00790D18`; `[B]` grep 65 `PacketCreator(23,n)` không có 102 |
| 3 | 🔴 **P0** | `load_items` chỉ đọc **4/22 cột** DB, `save` chỉ ghi **6/22 cột** rồi `DELETE` toàn bảng ⇒ **16 cột affix/enhance/lock mất mỗi lần auto-save**. | `[R]` `sqlite/session.rs:176-199`, `:299-314` |
| 4 | 🟠 **P1** | Template field (`loai`, `lv`, `int1..fai2`) **không được hydrate** khi load ⇒ sau relog không mặc được đồ (`loai = 0`, gate `1..=6` fail) và gear bonus = 0. Đây là nguyên nhân gốc, không chỉ là thiếu `recompute_stats()`. | `[R]` `load_items` không gọi `from_template`; `handlers/inventory.rs:292` |
| 5 | 🟠 **P1** | `08 01` prop-code mapping **lệch ngữ nghĩa**: Rust gửi `hp_max` vào `0x19`/`0x1A` (Bear quy = hp/sp **hiện tại**), và `hpx2`/`spx2` vào `0xCF`/`0xD0` (Bear quy = `hp2`/`sp2` phẳng). `0xDA`/`0xDB` đúng nghĩa chưa từng được gửi. | `[B]` `TSConstants.cs:5-67` |
| 6 | 🟠 **P1** | `1E 01` parser **không có clear pass** (khác `17 05`) ⇒ ô cũ không tự xoá sau khi rút bớt item. Cần frame `1E 08`. | `[C]` `0076BFC8` có clear loop, `007707A0` không |
| 7 | 🟠 **P1** | `05 03` tail: Rust `F401`×5 + **90 zero** + skill list; Bear `500`×5 + **43 zero** + skill list + **32 byte** block `_3`/`_4`. | `[B]` `TSCharacter.cs:5072-5098` |
| 8 | 🟡 **P2** | Mapping `storagetype` bị **nhân bản 6 nơi**; sức chứa `HOMDO_SLOTS=25` bị dùng cho cả túi 50 ô. | `[R]` `model.rs`, `persist.rs`, `sqlite/session.rs`×3 |

**Đánh giá tổng:** kiến trúc hiện tại **đúng về hình dạng** (một bảng `inventories`, 5 `storagetype`,
`dump_*` tách bạch) nhưng **sai về byte và độ tròn đầy**.

- ✅ **`17 05`** và **`1E 01`** — hai container lớn nhất — **đúng byte-for-byte**.
- 🟡 **`17 2F`** — giả định 12B có cơ sở (Bear dùng chung hàm `sendItems`).
- 🔴 **`17 0B`** — sai stride.
- 🔴 **`17 66`** — không xác minh được.

**Lộ trình tối thiểu đóng P0: 2-4 ngày, KHÔNG cần capture** (Phase 1 + 2A + 3).
Ba gap (G-02, G-06, G-07) chỉ đóng được bằng capture client thật.

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

## 5. Ba gap chỉ đóng được bằng capture

| Gap | Vì sao không giải bằng đọc code |
|---|---|
| **G-02** `17 66` | Thiếu body `FUN_0076A320`; Bear không gửi frame này |
| **G-06** `08 01` prop code | Chưa có client parser `FUN_0078D125`; bảng lấy từ Bear |
| **G-07** `05 03` tail | Chưa biết client đọc fixed-offset hay tới-cuối buffer |

Nếu không có `aLogin.exe` chạy được ⇒ dùng **phương án an toàn** cho cả ba:
tạm dừng `17 66`, giữ `05 03` hiện tại, giữ `08 01` hiện tại (đánh dấu "unresolved").

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