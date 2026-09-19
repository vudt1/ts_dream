# Kế hoạch: Server đáp ứng đúng yêu cầu chat của client aLogin.exe (Opcode `0x02`)

> **HANDOFF (2026-09-19) — tình trạng: G1+G3+G4 hoàn thành, suite xanh.**
> Đã xong: T1.1, T1.2, T1.3, T1.4, T1.5, T2.2, T2.3-một-phần (solo check, sub 5 chốt phương án a), T2.5, T3.1 (giữ nguyên tắc có sẵn), T3.2, T3.3, T4.1, T4.2 (5s/tin), T4.3, hằng `CHAT_SUB_*`.
> Chưa làm (cần capture live của user trước): **T0.1, T1.6 (quyết định echo), T1.7 (handler C→S mới), T2.3-registry-live, T2.4 (mới chỉ là chính sách dùng builder T1.5, chưa đấu nối lệnh nào vào sub 0x00)**. T1.0, T2.1, T3.4: không cần code (xem chi tiết từng ticket).
> File đã đổi (chưa commit — user tự commit): `src/server/handlers/chat.rs`, `src/server/spawn.rs`, `src/protocol/mod.rs`, `src/server/session.rs`, `tests/chat_test.rs` (mới, 18 test), `tests/p0_p1_p2_roadmap_test.rs` (sửa kỳ vọng cũ), `tests/db_repository_init_test.rs` (thêm field).
> Verify cuối: `cargo test --all-targets --no-fail-fast` xanh toàn bộ (chat_test 18/18).
> Đọc tiếp ở **§Tiếp theo** cuối file trước khi làm gì khác.

Nguồn chân lý: `.scratch/client-pseudo-op-code/opcode_02.md` (S→C), `client_pseudo_c/0077f414_FUN_0077f414.c` + `opcode_37.md` (C→S: `0x02`/`0x1A` rỗng; **`0x37` là CafeID submit, không phải chat**), Bear `ChatHandler.cs` (tham khảo routing + slash).
Hiện trạng Rust: `src/server/handlers/chat.rs`, `src/server/spawn.rs` (`chat_frame`/`sys_msg_frame`/`announce_frame`), `src/server/gm.rs`.
Quy ước test: mọi test mới trong `tests/` (cấm `#[cfg(test)]` trong `src/`), DB test dùng memory/tempfile, không chạm `DB/ts_dream.db`.

Nguyên tắc xuyên suốt: **S→C phát đúng layout mà client parse** (`[02][Sub][id 4B LE][msg]`, trừ `0x0C` không id và `0x08` không field); **không gửi sub không có nhánh** (`9`, `0xA`); tôn trọng gate kênh + nhánh id của client thay vì chống lại nó.

---

## G0 — Cửa C→S: tĩnh đã tới hạn, capture live là bắt buộc

**Đã chứng minh tĩnh từ `client_pseudo_c/0077f414_FUN_0077f414.c` + `opcode_37.md` (không cần capture):**
- C→S `0x02` rỗng: `case 2: break;` (`:863-864`). C→S `0x1A` rỗng: `case 0x1a: break;` (`:983-984`).
- **Đính chính: `case 0x37` KHÔNG phải chat.** `opcode_37.md §1,§6` chứng minh `gvar_007DA3B4` = `TSe_CafeIDForm`, `+0x138` = editor `"editorBG"` của form CafeID (constructor `005b0774.c:49-52`), text gửi đi là **Cafe ID đang nhập**; S→C `0x37` điều khiển form này (ẩn/banner). Giả thuyết "C→S chat = 0x37" trong bản kế hoạch trước là **sai** — đã loại.
- Các case `0x1d/0x3a/0x3b` (`:989-999, :1066-1085`) nối thêm **số** từ form globals (tiền/shop/…) → không phải chat, đã loại.
- `TSe_InputBar` (`gvar_007DA1DC`, ô nhập chat thật): mọi tham chiếu trong tập `.c` đều là UI-side (show/hide/flag, vd `0052abe4`, `0078a89c:829-830`); **không có caller nào đọc text của nó để gọi `SendCommand`** trong toàn bộ tập decompile → đường gửi chat (phím Enter trên input bar) nằm ngoài vùng đã export. Tĩnh không tiến thêm được.

**Bắt buộc capture live (T0.1 — chặn G2 channel-mapping và mọi handler C→S mới):**
1. Bảng op/sub/body khi gõ chat từng kênh (gần/thì thầm/đội/đoàn) + whisper + `/where`: op thực tế là gì, whisper có prefix target id không, encoding xác nhận.
2. Client có tự hiện text mình gõ không (quyết định echo T1.6), id người chơi ngoài dải 100..400 không (T1.0), party khác map có thấy sub 5 không (T2.3).
3. Output: bảng C→S chat thực tế + mẫu packet vàng mỗi kênh. Trong lúc chờ capture, G1 (fix S→C) vẫn làm được toàn bộ vì không phụ thuộc cửa C→S.

- **T0.1 ⏳ CHƯA LÀM — VIỆC CỦA USER (capture live).** Nội dung như 3 mục capture ở G0. Chặn: T1.6, T1.7, T2.3-registry-live và mọi handler C→S mới. Không chặn G1/G3/G4 (đã xong).

---

## G1 — Sửa emitter S→C cho khớp `opcode_02.md` (mỗi ticket ~0.5 ngày)

- **T1.0 ⚪ KHÔNG CẦN CODE.** Guard dải id NPC chuyển thành test trong T4.3 (id test đều > 400) + quy tắc verify tay. Không còn việc.
- **T1.1 ✅ XONG.** Whisper đóng sender id cả hai đầu (`chat.rs`, `CHAT_SUB_WHISPER`).
- **T1.2 ✅ XONG.** Sub 4 gate `gm_level > 0`, non-GM drop + warn log.
- **T1.3 ✅ XONG.** Sub 6 echo-only, không broadcast.
- **T1.4 ✅ XONG.** Xóa `wears_global_chat_item`; sub 2 luôn map-only.
- **T1.5 ✅ XONG.** `system_broadcast_frame` (0x00), `self_talk_frame` (0x07), `input_flag_frame` (0x08, chưa gọi ở đâu), `long_memo_frames` chunker 0x0B + `#end` (`spawn.rs`).
- **T1.6 ⏳ CHƯA CHỐT — chờ capture T0.1.** Hiện giữ `out.send` echo (hành vi cũ). Nếu capture cho thấy client tự hiện → bỏ echo sub 2/4/6.
- **T1.7 ⏳ CHƯA LÀM — chờ output T0.1.** Handler C→S chat mới (op chưa biết), parse `[op][kênh][text]`, slash intercept, tái dùng routing G1/G2. Đây là ticket mở khóa "chat aLogin hoạt động được".
- **Ngoài scope chat (ghi nhận, không làm trong kế hoạch này):** C→S `0x37` là CafeID submit (`[37][CL][cafe-id text]`, `opcode_37.md §7.2`) — server hiện drop ở nhánh `unimplemented` (đúng là chưa xử lý, nhưng đó là ticket CafeID riêng, không thuộc chat).

---

## G2 — Routing từng kênh (sau G1, mỗi ticket ~0.5 ngày)

- **T2.1 ⚪ KHÔNG CẦN CODE.** Map chat giữ `broadcast_map` + echo (echo chờ T1.6 chốt).
- **T2.2 ✅ XONG.** Whisper sender id cả hai đầu; offline → `020B`.
- **T2.3 Party chat — phương án a (sub 5) GIỮ NGUYÊN; chỉ xong một phần ✅.** Đã làm: solo/không-party → `020B`. Chưa làm ⏳: resolve từ party registry live (hiện dùng snapshot `id_leader/id_mem` của sender) — chờ capture kiểm chứng party khác map thấy sub 5 không.
- **T2.4 ⚪ CHÍNH SÁCH XONG, CHƯA ĐẤU NỐI.** Builders sub 0x00/0x0C sẵn (T1.5); chưa có lệnh/luồng nào phát sub 0x00. Khi cần loa toàn server thì đấu nối, không dùng sub 1.
- **T2.5 ✅ XONG.** C→S sub 1 drop + log, code cũ giữ comment.

---

## G3 — Slash command chặn trên sub 2 (giống Bear, sau G1)

- **T3.1 ✅ XONG (giữ nguyên tắc có sẵn).** Intercept `/` trước broadcast trên sub 2; lệnh lạ drop im cả với GM (an toàn hơn Bear).
- **T3.2 ✅ XONG.** Giữ 5 lệnh cũ + `/help`, `/offq` alias `/endtalk`; nhóm bot stub `020B` "chưa hỗ trợ"; `/warp /exchange` mặc định stub (quyết định gameplay riêng).
- **T3.3 ✅ XONG.** Giữ chặn battle + lan party leader-only + thêm announce "Sleep command executed." (parity Bear).
- **T3.4 ⚪ KHÔNG CẦN CODE.** Audit GM giữ qua `gm.rs`; slash player không audit.

---

## G4 — Giới hạn, chống spam + test (cuối cùng)

- **T4.1 ✅ XONG.** `MAX_CHAT_CHARS = 120` cho sub 2/3 (đếm `chars()` sau `viscii_decode`); vượt → log + `020B` "Tin nhan qua dai." Sub 5/6 không check (đúng kế hoạch).
- **T4.2 ✅ XONG (5s/tin).** `CHAT_COOLDOWN_MS = 5_000` + `Session::last_chat_ms` (runtime-only, không persist) + helper `chat_cooldown_ok` (`chat.rs`). Áp cho sub Gần/Thì Thầm/Đài/Đoàn; slash và GM exempt; quá nhanh → `020B` "Ban chat qua nhanh, vui long doi N giay." + warn log. Còn lại cho follow-up: mute/ban table + audit log chat nhạy cảm.
- **Hằng kênh ✅ XONG (ngoài kế hoạch).** `CHAT_SUB_*` trong `src/protocol/mod.rs` kèm nhãn client `(Gần)/(Thì Thầm)/(GM)/(Đài)/(Đoàn)/(Minh)/(Công bố hệ thống)` — `chat.rs`/`spawn.rs` không còn hardcode số sub; `tests/chat_test.rs` khóa giá trị + wire bytes.
- **T4.3 ✅ XONG.** `tests/chat_test.rs` 18 test xanh (routing/echo, whisper sender-id + offline, gate sub 4, sub 1 vô hiệu, sub 5 solo, builders, hằng kênh, cooldown + exempt + hết hạn). Sửa kèm `tests/p0_p1_p2_roadmap_test.rs` (kỳ vọng cũ assert hành vi bug) và `tests/db_repository_init_test.rs` (thêm field `last_chat_ms`).

---

## Thứ tự thực hiện và nghiệm thu (đã xong tới T4.3 trừ các mục chờ capture)

`~~T0.1~~ → ~~T1.0~~ → ~~T1.1~~ → ~~T1.2~~ → ~~T1.3~~ → ~~T1.4~~ → ~~T1.5~~ → T1.6 → T1.7 → ~~T2.1~~ → ~~T2.2~~ → T2.3-phần-còn-lại → ~~T2.4~~ → ~~T2.5~~ → ~~T3.1~~ → ~~T3.2~~ → ~~T3.3~~ → ~~T3.4~~ → ~~T4.1~~ → ~~T4.2~~ → ~~T4.3~~.`

Nghiệm thu cuối: client aLogin thật thấy đúng nhãn từng kênh (`(Gần)/(Thì Thầm)/(GM)/(Đài)/(Đoàn)/(Minh)/(Công bố hệ thống)`), whisper hiện đúng tên người gửi cả hai đầu, channel flag tắt kênh nào thì mất kênh đó, ignore-list còn tác dụng, slash không bao giờ lọt ra kênh chat, `T4.3` xanh.

---

## Tiếp theo — việc cần làm (theo thứ tự)

**Việc của user (không agent nào làm thay được):**
1. **T0.1 capture live** — gõ chat từng kênh (gần/thì thầm/đội/đoàn) + whisper + `/where`, ghi raw frame C→S: op thực tế là gì, byte kênh mỗi UI channel, whisper có prefix target id không, encoding. Kèm 3 xác nhận: client có tự hiện text mình gõ không / id người chơi ngoài 100..400 không / party khác map có thấy sub 5 không. Output bỏ vào `.scratch/op-working/` rồi báo agent.

**Việc của agent (sau khi có output T0.1):**
2. **T1.7** — handler C→S chat mới theo bảng kênh capture được (parse `[op][kênh][text]`, slash intercept, tái dùng routing G1/G2, giữ `0x02` cho Bear-dialect).
3. **T1.6** — chốt echo: nếu client tự hiện thì bỏ `out.send` ở sub 2/4/6 (giữ whisper 2 đầu + party cả team).
4. **T2.3-phần-còn-lại** — party resolve từ registry live thay vì snapshot sender (kèm kiểm chứng party khác map).
5. **T2.4-đấu-nối** — khi cần loa toàn server: đấu nối lệnh vào builder sub 0x00 (không dùng sub 1).
6. **Follow-up (không chặn nghiệm thu):** mute/ban table + audit log chat nhạy cảm; guild/army membership thật để mở lại sub 6; ticket CafeID riêng cho C→S `0x37` (hiện drop ở `unimplemented`); quyết định `/warp /exchange` mở hay stub theo roadmap gameplay.
7. **Commit thủ công** (user tự làm, agent không commit): `src/server/handlers/chat.rs`, `src/server/spawn.rs`, `src/protocol/mod.rs`, `src/server/session.rs`, `tests/chat_test.rs`, `tests/p0_p1_p2_roadmap_test.rs`, `tests/db_repository_init_test.rs`.
