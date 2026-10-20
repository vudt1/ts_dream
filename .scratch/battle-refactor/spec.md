# Battle Refactor — Kế Hoạch Hoàn Thiện Giao Tiếp Trận Chiến (`ts_dream` ↔ `aLogin.exe`)

> **Nguồn**: bộ nghiên cứu `.scratch/client-pseudo-op-code/battle/` (README + 01..05, mức bằng chứng A/B/C).
> **Phạm vi**: làm cho server Rust gửi/nhận đủ frame battle để client `aLogin.exe` chơi trận trơn tru — mở trận, nhận lệnh trận, hiển thị HP/ô lưới, kết thúc trận + phần thưởng (exp/item/gold/pet-exp/level-up).
> **Đầu ra**: spec này + 15 ticket trong `issues/`, đánh số theo thứ tự phụ thuộc. Mỗi ticket tự mang test trong `tests/` — không có ticket test riêng. Không commit source; người dùng tự commit.
> **Bằng chứng ASM**: toàn bộ kết luận reverse client (mức A/B, kèm địa chỉ hàm + số dòng) nằm ở [`HANDOFF-client-reverse.md`](HANDOFF-client-reverse.md) — đọc file đó trước khi sửa layout frame.

---

## 1. Đánh giá bộ nghiên cứu

### 1.1 Đã chắc chắn (mức A) — dùng làm nền

- **H-1**: client không bao giờ gửi `0x32`; kênh lệnh trận thật là `0x0B` — tập sub hợp lệ `{0,1,2,3,4,6,7,8,9,0A,FA}`, chỉ sub 5 là no-op (`03_skill_and_combo.md §0.1.2`, `§0.1.6`).
- **H-3**: lưới 4 nhóm × 5 ô; 12/12 frame pos hiện tại của Rust khớp byte 100%; `0B FA` record 24B, `0B 05` record 23B (= 24B bỏ byte `kind`).
- **H-2**: combo 100% server-side; frame `32 01` của Rust khớp client — **không cần rework**.
- Layout các frame S→C còn thiếu đã có ASM/pseudo-C kèm theo: `33 01`, `35 07`, `35 03/0E/0F`, `05 0A`, `1A 01`, `0F 08`, `17 06` (Round-4).

### 1.2 Đã cũ — đính chính, không mở ticket

- `01_battle_start_scene.md §14.4` nói `build_join_frame()` "22 byte" — **sai**: record hiện tại đã đủ 24 byte (literal 16 ký tự hex bị đếm nhầm thành 6 byte). Không sửa gì.
- `01_battle_start_scene.md §14.3` nói `member_battle_frame()` "thiếu trailer" — **sai**: trailer `F44403000B0A01` đã có sẵn. Không sửa gì.
- `README §8` mục 8 (build_join_frame) vô hiệu theo trên; mục 20 (Ghidra memory map) — **ngoài phạm vi**, xem §4.
- `README §8` mục 10 (`tientrang`): câu hỏi đặt sai khung — **không có slot-table `tientrang`**; đây chỉ là `storage_type = 2` (`StorageType::Bank`) trong bảng `inventories`, Rust đã load/save/dump qua `StorageType::Bank`. Đã chốt — không cần capture, không refactor.

### 1.3 Đã chốt bằng ASM (2026-10-16) — chi tiết ở `HANDOFF-client-reverse.md`

| Mục | Kết luận | Bằng chứng |
|:-:|---|---|
| Kind byte self `0B FA` (G1) | Phải là **`04`**. `0B FA` ghi `kind` thẳng vào `player+0x376`; nút rời trận **và** reset của `0B 00` đều đòi `0x376 == 4`. Rust đang gửi `03`/`05` ⇒ client kẹt trong trận | `006438ac:262` · `00590ba8:159-163` · `00641fa4:62,69-76` · `006438ac:263-270, 396-402` |
| Byte confirm `0B 01` (G3) | **`3`**. S→C `0B 01` đọc `content[2]` row (≤3), `content[3]` col (≤4) ⇒ ≥ 4 byte; byte pad `00` thừa **vô hại** | `00590ba8:160` · `0064bb24:50-62` |
| Ngữ nghĩa kind sub `08`/`0A` | **Thao tác map/PK ngoài trận**, không phải lệnh trận lưới: `09` tấn công player (khác đội), `0A` tấn công thú, `0B`/`0C` thao tác thú, `0E` thách đấu cùng đội, `12`/`13` nhóm `gvar_007DA10C`, `15` (sub `0A`) cướp cờ. Guard: team `+0x5F0`, khoảng cách `< 0x97`, hộp thoại | `0064ea20` · `0064ed14` · `0064edf8` · `0064eecc` · `0064efa0` · `0064f60c` · `0064f6e4` · `0064f7bc` |
| Round-trip `17 06` (G4) | Rust **đã đúng byte-exact**: `17 06 \| id:u16 \| count:u16 \| doben \| long \| giatri+100 \| khang \| texp:u32`, `LEN = 0x0E` | `0076b458.asm.txt:25-182` · `0072b6c8` |
| Round-trip `0F 08` (G9) | Layout đầy đủ + **stride `nameLen + 0x63`**; Rust lệch 2 chỗ (`le32(id)` phải là `le16(id)` + `le32(texp)`; tail 6 byte thay vì 12) | `007a5010.asm.txt:140-807` |
| Tail `05 03` (G8) | Rust **đã đúng**: `hp_max` ở `content[0x20]`, `sp_max` ở `0x22`, header 111 byte, danh sách skill từ `0x71` (3 byte/entry). Claim "lệch 4 byte" của doc 04 là do lỗi quy ước index | `0072e9a0:150-383` |
| Hằng `k[reborn]` + cột (G6/G7) | `k = {2.9, 3.0, 3.05, 3.0}` (reborn 0 `k = 2.9` đọc từ `Formula.Dat` payload `+0x80` / RAM `table+0x84`); client có hằng `+5` đọc từ `Formula.Dat` payload `+0xA8` (`extra_exp`) cộng vào EXP mỗi cấp; cột chung `table+0xAC` — **không có cột thứ 4**; rounding = half-away-from-zero | `0065b0dc.asm.txt:17-157` · `Formula.Dat +0x80, +0xA8` |

### 1.4 Câu hỏi đã trả lời được bằng ASM (chỉ còn xác nhận runtime)

| Mục | Kết luận | Bằng chứng |
|:-:|---|---|
| **G5** — frame kho | **`1E 01`** = ghi vào container `player+0xA48` (51 ô, record 12B slot-led) ⇒ Rust `OPCODE_TIENTRANG = "1E01"` + `encode_entry_12` **đúng**. `17 1E` là **stall/gán hàng** trên homdo (`+0x9E0`, 26 ô, record 5B) | `007707a0:78-192` · `00730f4c:84-168` |
| **G10** — frame kết quả trận | **Không tồn tại.** `0x33` chỉ có sub 1; `0x35` (case 46 / `FUN_007952ab`) có các sub `1, 3, 4, 5, 6, 7, 8, 9, 0x0A..0x0F` (case 53 là Opcode `0x3C`); `0x0B` không sub nào mang (thắng/thua, lượt, thời gian, điểm). Phần thưởng đi qua `08 01` / `17 06` / `1A 01` / `05 0A` / `0F 08` / `35 04` | `case_051_007956B9` · `case_046_007952AB` · `0078d5d1` |
| **G11** — `LAB_00644A18` | Callback trễ sau `0B 00`: giải phóng fight-field, `battleState = 0`, và **gửi C→S `13 02`** khi thú đang theo đã chết (`pet+0x3EA ≤ 1`) + điều kiện job `0x1C`. Rust đã có handler (`pet_actions.rs` sub 2 = recall) | `00644a18:21-64` + `asm:70-72` |

Hệ quả phụ: `35 0E` (unit action / vtable 0x1c) và `35 0F` (unit status buff/debuff) **có handler thật trong client** (`FUN_0064fbd8` và `FUN_0064fd34` thuộc `FUN_007952ab`); đã xây dựng builder đầy đủ trong `packets.rs`.

### 1.5 Còn chờ capture (ticket 14)

- Kind của record **không phải self** (`03`/`05`/`64`) và giá trị value trong `0B 04` (show-on-map hiện gửi `03`/`05`/`02`) — có cần `04` cho người khác không (gate chặn tấn công ở `0064efa0:56`).
- Số ô thực tế server dùng trong container `+0xA48` (client nhận `slot ≤ 50`, Rust dùng `1..=50`).
- Chuỗi frame client nhận trong ~1 giây sau khi trận kết thúc (phủ G10 ở mức A, kiểm tra có frame nào bị bỏ sót).
- Ngữ nghĩa `0F 08 +0x01..0x02` (`FUN_0071c3a0`), thân builder `0B` sub 0/1 (`0x78048D`, `0x780511`).

---

## 2. Hiện trạng Rust → ticket

| # | Vấn đề hiện tại | Ticket |
|:-:|:--|:-:|
| 1 | `0x32` là code chết; sub 7/8/9/0A của `0x0B` chưa parse; sub 8 bị ép thành PK trực tiếp; item trừ trước khi lệnh được nhận | **01** |
| 2 | Thiếu `33 01` ⇒ HP/SP unit trong trận không bao giờ cập nhật ở client | **02** |
| 3 | `0B 01` thừa byte đệm; tên hàm sai ngữ nghĩa; thiếu builder `35 07` (frame di chuyển ô duy nhất) | **03** |
| 4 | `17 06` **đã đúng layout** (claim sai trong doc 04); nhưng có nhiều builder trùng lặp + `apply_drop` chỉ mutate session, item không tới túi client | **04** |
| 5 | `apply_gold` chỉ cộng session; client không biết vàng (`1A 01` không dùng ở đâu) | **05** |
| 6 | Thiếu `05 0A` LevelUP; `Stat::Hpmax/Spmax` map về `None` ⇒ không frame MaxHP/MaxSP (tail `05 03` đã verify đúng) | **06** |
| 7 | `pet_stat_entry` lệch: `le32(id)` phải là `le16(id)` + `le32(texp)`; tail 12 byte → 6 byte; `apply_pet_exp` không gửi frame | **07** |
| 8 | Team tính bằng `row == 0` thay vì `row < 2` (3 chỗ, latent bug) | **08** |
| 9 | `finish()` hardcode người chơi hàng 3 / pet hàng 2 (latent bug) | **09** |
| 10 | Runner tạo `time_seeded()` mới giữa trận ⇒ phá replay/golden determinism | **10** |
| 11 | Bảng EXP: `k = {2.9, 3.0, 3.05, 3.0}` (`k[0]=2.9` và `+5` từ `Formula.Dat`), reborn 3 k=3.0, rounding half-away-from-zero; MaxHP/SP đối chiếu `FUN_0065a554` | **11** |
| 12 | Builder `35 03` (state 7) / `35 0E` (unit action) / `35 0F` (unit status) | **12** |
| 13 | Dead code `battle_role.rs` (layout TS Mobile), magic `F4441C`, API nguy hiểm `turn_action` | **13** |
| 14 | Gap còn lại **G5 / G10 / G11** + kind của record không-self + value `0B 04` | **14** |
| 15 | Marker self trong `0B FA` phải là `04` (hiện `03`/`05`) — client không cho rời trận, `0B 00` không reset được | **15** |

---

## 3. Thứ tự thực hiện & blocking edges

- **Độc lập, làm ngay**: 01, 02, 03, 04, 05, 06, 07, 08, 10, 11, 13, **15**.
- **09 ← 06** (đường frame lên cấp mới phải xong trước khi tổng quát hoá `finish()`).
- **14 ← 01, 15** (capture cần log parse từ kênh `0x0B` thật + trạng thái client đúng để đối chiếu).
- **12**: builder ship ngay; phần nối dây vào luồng trận chờ kết quả 14 (ghi chú trong ticket, không block cứng).
- **15 nên sớm** vì không có nó thì client không rời được trận ⇒ mọi capture sau đó sẽ bị kẹt UI.

Frontier khởi đầu: chọn trong 01..08, 10, 11, 13 — ưu tiên 01 để sớm có log cho capture.

---

## 4. Ngoài phạm vi

- **`tientrang`**: đã chốt — chỉ là `storage_type = 2` (`StorageType::Bank`) trong bảng `inventories`, không phải slot-table riêng; không refactor, không cần capture.
- **Ghidra memory map** (`README §8` mục 20).
- **Combo engine**: đã đúng (H-2) — chỉ giữ nguyên.
- Các frame pos đã khớp 12/12 — không sửa.
- UI/cảm giác ngoài wire protocol.

---

## 5. Quy ước triển khai & kiểm thử

- Mọi test mới đặt trong `tests/`; **cấm** `#[cfg(test)]` trong `src/`.
- Test chạm DB dùng in-memory SQLite hoặc `TS_TEST_DB_URL=sqlite://DB/ts_dream_test.db` — tuyệt đối không đụng `DB/ts_dream.db`.
- Byte layout: khi doc mâu thuẫn quy ước index (ví dụ `17 06` §0.1.2 vs §0.1.3 ±1 byte), **re-derive từ ASM** (`0076b458`, `007a5010`, `0065b0dc`, `0065a554`) và ghi nguồn vào test/comment.
- Không commit — người dùng tự commit thủ công.
