# HANDOFF: Chuẩn hóa Giao thức & Vòng đời Trận đấu (Battle Refactor V2)

- **Ngày cập nhật**: 2026-10-24
- **Dự án**: `ts_dream` (Rust game server cho TS Online PC `aLogin.exe`)
- **Tập tin lưu**: `.scratch/battle-refactor/handoff-battle-refactor.md` (bản sao tại `/tmp/handoff-battle-refactor.md`)
- **Phiên bản đặc tả hiện tại**: `2.0` (tham chiếu [`.scratch/battle-refactor/spec_new.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/spec_new.md))
- **Theo dõi tiến độ chi tiết**: [`.scratch/battle-refactor/HANDOFF-implementation-progress.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/battle-refactor/HANDOFF-implementation-progress.md)

---

## 1. Tóm Tắt Tiến Độ: ĐÃ HOÀN THÀNH ĐỢT 1 (Tickets 19, 20, 21) & MỘT PHẦN ĐỢT 2 (Tickets 16, 17)

Toàn bộ các ticket sau đã được thực thi và xác minh nghiêm ngặt với 100% test suite xanh (57 test binaries, 335 passed, 0 failed):
- **Đợt 1**: Ticket 19 (kênh lệnh `0x32` & domain enum `BattleAction`), Ticket 20 (barrier handshake `0B FA` & timing mở hiệp `34 01`), Ticket 21 (frame `32 02` `turn_action_finish`).
- **Đợt 2 (Phần 1)**: Ticket 16 (builder `35 06` thoại NPC / banner trận) và Ticket 17 (builder `35 0B` battle chat & system notice).

### 1.1 Chi tiết các ticket Đợt 2 vừa hoàn tất

1. **Ticket 16: Builder `35 06` (Thoại NPC / banner diễn hoạt trong trận)**:
   - **Mã máy client đối chiếu**: `FUN_0064e064` (`case 0x06` trong dispatcher Opcode `0x35` `FUN_007952ab`).
   - **Cấu trúc frame**: `F4 44 0B 00 35 06` + `[k:1B]` + `[actor_id:4B LE]` + `[tpl_id:4B LE]` (Payload 11 bytes, frame 15 bytes).
   - **Builders**:
     - `packets::battle_npc_talk(k: u8, actor_id: u32, tpl_id: u32) -> String`
     - `packets::battle_npc_dialog(dialog_id: u16) -> String` (quy ước Bear C# `PacketCreator(53, 6)` với `k=1`, `actor_id=0`, `tpl_id=dialog_id`).
   - **Hiệu ứng**: Kích hoạt `THuman.Create`, đưa vào battle HUD và hiển thị toast banner 2000ms.

2. **Ticket 17: Builder `35 0B` (Tin nhắn / bảng tin thông báo chiến đấu)**:
   - **Mã máy client đối chiếu**: `FUN_0075c518` (`case 0x0B` trong `FUN_007952ab`).
   - **Cấu trúc frame**: `F4 44` + `LE16(len)` + `35 0B` + `([type:1B][len:1B][text:len B])*n` (hỗ trợ tối đa 21 bản ghi gom nhóm).
   - **Định dạng kênh**:
     - `battle_notice_type::SYSTEM = 1`: Tiền tố Hàng 1 / thông báo hệ thống.
     - `battle_notice_type::NOTICE = 2`: Tiền tố Hàng 2 / cảnh báo.
     - `battle_notice_type::COMBAT = 3`: Tiền tố Hàng 3 / nhật ký chiến đấu.
   - **Builders**:
     - `packets::battle_chat_notice(entries: &[(u8, &str)]) -> String`
     - `packets::battle_system_message(text: &str) -> String`
     - `packets::battle_notice(msg_type: u8, text: &str) -> String`
   - **Sửa lỗi mã hóa VISCII 1.1 cốt tử**: Trong [`src/encoding.rs`](src/encoding.rs), hàm `char_to_viscii` đã được sửa để ưu tiên `unicode_to_viscii` trước map mojibake CP1252. Nhờ đó, ký tự `'Õ'` (U+00D5) được mã hóa chuẩn thành byte `0xA0` (không bị map CP1252 ghi đè thành `0xD5` / chữ `'ạ'`), đồng thời `to_viscii` tự động chuẩn hóa NFD $\rightarrow$ NFC.

### 1.2 Hồ sơ kiểm thử Đợt 2
- Tạo mới [`tests/battle_talk_notice_test.rs`](tests/battle_talk_notice_test.rs) với 15 test cases toàn diện: byte parity, endianness, bear convention, empty entries, batching tối đa 21 entries, tiếng Việt VISCII round-trip, NFD normalization.
- Chạy `cargo test --all-targets --no-fail-fast`: **57 test binaries, 335 passed, 0 failed, 0 ignored**.

---

## 2. Trạng Thái Hiện Tại & Công Việc Tiếp Theo (Immediate Frontier)

Đợt 1 và phần builder của Đợt 2 đã xong. Nhiệm vụ tiếp theo là **Ticket 18**:

```text
├── [ĐÃ XONG] ĐỢT 1: SỬA LỖI GIAO THỨC & VÒNG ĐỜI LƯỢT ĐẤU (P0)
│   ├── ✅ Ticket 19: issues/19-restore-0x32-command-channel.md
│   ├── ✅ Ticket 20: issues/20-turn-lifecycle-timing-and-34-01.md
│   └── ✅ Ticket 21: issues/21-turn-action-finish-32-02-frame.md
│
├── [ĐANG THỰC HIỆN] ĐỢT 2: HOÀN THIỆN BIẾN CỐ 0x35 & TIMING DIỄN HOẠT (P1)
│   ├── ✅ Ticket 16: issues/16-35-06-battle-npc-talk-builder.md
│   ├── ✅ Ticket 17: issues/17-35-0b-battle-chat-notice-builder.md
│   └── ⏳ Ticket 18: issues/18-battle-0x35-timing-wiring.md (TIẾP THEO)
│       └── Đấu nối timing 0x35 (35 03, 35 0F) + dynamic delay pacing (800ms - 3500ms) vào runner.rs
│
└── ĐỢT 3: BÀN CỜ ISOMETRIC & GAMEPLAY PARITY (P2)
    ├── ⏳ Ticket 22: issues/22-isometric-grid-aoe-targeting.md
    ├── ⏳ Ticket 23: issues/23-combo-rules-aoe-threshold.md
    └── ⏳ Ticket 24: issues/24-capture-validation-gates.md
```

### Nhiệm vụ tiếp theo: **TICKET 18**
- Tham chiếu: [`issues/18-battle-0x35-timing-wiring.md`](issues/18-battle-0x35-timing-wiring.md).
- Đấu nối các gói tin `0x35` (`35 03` force state 7, `35 0E` unit action, `35 0F` status update, `35 06` npc talk, `35 0B` battle notice) vào các pha thực thi trong [`src/battle/runner.rs`](src/battle/runner.rs).
- Bổ sung cấu hình dynamic animation delay pacing (800ms - 3500ms) theo từng loại chiêu thức để client có đủ thời gian vẽ hoạt ảnh đồ họa trước khi sang đòn đánh kế tiếp.

---

## 3. Các Ràng Buộc Kỹ Thuật Bất Biến (Non-Negotiable Invariants)

1. **Vị trí Test**: Mọi unit/integration test mới **bắt buộc đặt trong `tests/`**. Tuyệt đối **cấm** `#[cfg(test)]` trong `src/`.
2. **Cơ sở dữ liệu**: Không ghi đè hoặc can thiệp trực tiếp `DB/ts_dream.db`. Mọi test DB dùng in-memory SQLite (`sqlite::memory:?cache=shared`).
3. **Mã hóa chuỗi**: Chuỗi thoại và thông báo gửi client phải mã hóa **VISCII 1.1** (`crate::encoding`).
4. **Git Policy**: **Tuyệt đối không tự ý `git commit`**. Người dùng sẽ commit thủ công.

---

## 4. Suggested Skills Cho Agent Tiếp Theo

Agent ở phiên tiếp theo nên kích hoạt các skill sau:
- **`tdd`**: Áp dụng chu trình Test-Driven Development cho Ticket 16, 17, 18. Viết test assert byte-exact trước trong `tests/`, sau đó implement builder và timing wiring.
- **`reverse-engineer-anything`**: Khi cần đối chiếu lại payload của `35 06` và `35 0B` với mã máy `client_pseudo_c/0064f7b4` và `0064fa58`.
- **`code-review`**: Rà soát lại thay đổi sau khi hoàn thành Đợt 2 để đảm bảo tuân thủ tuyệt đối [ADR 0006](file:///mnt/d/VUDT/GIT_PCC/ts_dream/docs/adr/0006-battle-protocol-parity-and-turn-lifecycle.md) và [`CODING_STANDARDS.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/CODING_STANDARDS.md).
