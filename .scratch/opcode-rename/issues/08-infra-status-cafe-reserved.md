# 08 — Phase2: Infra 0x36 ServerStatus + 0x37 CafeId + bổ sung 0x38/0x44

Status: ready-for-agent
Blocked by: 07

## Goal
Xóa nhãn Keepalive/Stall/Connect sai + lấp opcode thiếu 0x38.

## Evidence
- 0x36: không SubOp, payload dãy cặp `[idx 1-based][lv 0..3]` ghi mảng trạng thái form chọn server (`+0x150`, icon 4 frame). C→S chỉ 1 byte `[36]` request bảng. Không heartbeat/ping (`system.rs` không handler, dispatcher `unimplemented`).
- 0x37: form `TSe_CafeIDForm`: S→C SubOp 1 ẩn form / SubOp 2 + variant banner 2000ms + ẩn; C→S `[37][CL][editor.text]` submit ID. Không bày bán.
- 0x38: pseudo `opcode_38.md` — handler no-op tuyệt đối (đọc SubOp rồi bỏ, chỉ `BoundErr` khi L=1), có case 49 trong jumptable nhưng vắng trong mod.rs/spec/dispatcher.
- 0x44: jumptable nhảy 0x43→0x45 (khuyết 0x44), không `case 0x44` C→S, C# không case 44. Tên Connect zero bằng chứng.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x36 | OP_KEEPALIVE | OP_SERVER_STATUS |
| 0x37 | OP_STALL | OP_CAFE_ID |
| 0x38 | (thiếu) | OP_RESERVED_38 = 0x38 (thêm vào SERVER_MAIN_OPCODES + opcode_name) |
| 0x44 | OP_CONNECT | OP_RESERVED_44 (giữ giá trị, ghi rõ chưa có handler) |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`.

## Acceptance
- 0x38 xuất hiện trong `SERVER_MAIN_OPCODES` + `opcode_name()`; 4 tên đúng giá trị; test xanh.

## Comments
