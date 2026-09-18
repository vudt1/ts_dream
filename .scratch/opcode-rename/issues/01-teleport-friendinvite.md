# 01 — Phase1: 0x07 Teleport + 0x0E FriendInvite

Status: ready-for-agent
Blocked by: none (ticket đầu chuỗi; các ticket sau rebase lên ticket này)

## Goal
Rename 2 opcode SAI rõ trong dải 0x02–0x0F.

## Evidence
- 0x07: pseudo `opcode_07.md` §0–§4 — không SubOp, payload thuần `[CharID:4B][MapID:2B][X:2B][Y:2B]`, 2 nhánh SELF/OTHER = teleport/đồng bộ vị trí tuyệt đối. Không byte thuộc-tính nào. C# không có `case 7`, Rust chưa route (rơi `unimplemented`).
- 0x0E: pseudo `opcode_0e.md` — manager lời mời bạn hữu/event-notify (`gvar_007D9F1C`, bảng pending, 7 sub-op), toast "Tiếp nhận/Cự tuyệt lời mời bạn hữu", "Thùng thư bạn hữu đã đầy". Không có nghiệp vụ mail. C# không có `case 14`.

## Rename
| Op | Cũ | Mới | Giá trị |
|---|---|---|---|
| 0x07 | OP_PLAYER_DETAIL | OP_TELEPORT | 0x07 |
| 0x0E | OP_MAIL | OP_FRIEND_INVITE | 0x0E |

## Files
- `src/protocol/mod.rs` (const + `opcode_name()` + `SERVER_MAIN_OPCODES` + alias deprecated cho tên cũ)
- `spec/server_main_opcode.md` (2 dòng)
- `src/server/dispatcher.rs` comment nếu nhắc 0x07/0x0E (không đổi số raw)

## Steps
1. Trong `mod.rs`: `pub const OP_TELEPORT: u8 = 0x07;` + `#[deprecated] pub const OP_PLAYER_DETAIL: u8 = OP_TELEPORT;` (tương tự 0x0E). Cập nhật `SERVER_MAIN_OPCODES` dùng tên mới, `opcode_name()` trả `"OP_TELEPORT"` / `"OP_FRIEND_INVITE"`.
2. Spec: `OP_TELEPORT = $07 // [0x07] Teleport / PositionSync tuyệt đối + MapID (Shared, không SubOp)`; `OP_FRIEND_INVITE = $0E // [0x0E] FriendInvite / SocialEvent notify (S->C chính)`.
3. Grep `OP_PLAYER_DETAIL|OP_MAIL` toàn repo, thay reference nội bộ sang tên mới (giữ alias chỉ cho ngoài).

## Acceptance
- `rg "OP_TELEPORT|OP_FRIEND_INVITE" src/protocol/mod.rs` có; tên cũ chỉ còn dòng `deprecated`.
- `cargo test --all-targets --no-fail-fast` xanh.

## Comments
