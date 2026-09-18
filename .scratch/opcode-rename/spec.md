# Spec: Rename opcode sai tên (Phase 1 + Phase 2)

Nguồn ground-truth: `.scratch/client-pseudo-op-code/opcode_*.md` (decompile aLogin.exe, jumptable S→C 65 case) + `TS_Server_Bear/` (C# `PacketProcessor.cs` dùng decimal = hex) + `src/server/dispatcher.rs`.
File đổi tên chính: `src/protocol/mod.rs` (const + `opcode_name()` + `SERVER_MAIN_OPCODES`) + `spec/server_main_opcode.md`.
Phát hiện quan trọng: `dispatcher.rs` và handlers hiện dùng số raw (`0x19`, `0x1A`…), KHÔNG dùng `OP_*` const — nên blast-radius của rename thấp, chủ yếu là `mod.rs` + spec + comment.

Phạm vi:
- Phase 1 (dải thấp, SAI chắc): 0x07, 0x0E, 0x10, 0x14, 0x16, 0x19, 0x1A, 0x1B, 0x1F, 0x21, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2A, 0x2E (17 op).
- Phase 2 (dải cao, SAI chắc + placeholder thiếu): 0x34, 0x35, 0x36, 0x37, 0x38 (bổ sung), 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x48, 0xC7 (20 op).
- NGOÀI phạm vi (Phase 3, chỉ comment): KHẢ_NGHI 0x03, 0x18, 0x22, 0x29, 0x2B, 0x2D, 0x32, 0x33, 0x49–0x4E. Không rename trong đợt này.

Quy ước rename áp dụng cho MỌI ticket:
1. Đổi tên const trong `src/protocol/mod.rs`, giữ giá trị u8 cũ.
2. Giữ alias cũ `#[deprecated(note = "renamed to NEW, see .scratch/opcode-rename/spec.md")] pub const OLD: u8 = NEW;` để không gãy code ngoài.
3. Cập nhật `opcode_name()` match arm + `SERVER_MAIN_OPCODES` (dùng tên mới).
4. Cập nhật `spec/server_main_opcode.md` dòng tương ứng (tên + mô tả + direction S→C / C→S / Shared theo pseudo).
5. Cập nhật comment trong `dispatcher.rs` / handler liên quan nếu nhắc tên cũ (không đổi logic số raw).
6. `cargo test --all-targets --no-fail-fast` phải xanh. Không chạy test ghi lên `DB/ts_dream.db` (dùng memory hoặc `TS_TEST_DB_URL=sqlite://DB/ts_dream_test.db` nếu cần).
7. Cấm `#[cfg(test)]` trong `src/`; test mới (nếu có) vào `tests/`.

Thứ tự thực thi: các ticket đều chạm `src/protocol/mod.rs` → chạy TUẦN TỰ theo số (mỗi ticket rebase sau ticket trước). Không claim song song.
