# 05 — Phase1: Cụm Account/Guild 0x23–0x27 + 0x2A Reset

Status: ready-for-agent
Blocked by: 04

## Goal
Rename cụm Guild sai toàn bộ (6 op) — ticket lớn nhất Phase 1.

## Evidence
- 0x23: Bear dec 35 = `AccountHandler` (đổi pass/xóa char/gift-code, reply `(35,1..3)`) + S→C `(35,4)` sendpoint/voucher. `CONTEXT.md:213` đã thừa nhận "Guild là nhãn sai". Pseudo S→C kênh số/notice, không guild.
- 0x24: Bear S→C `(36,11)` đổi job + `(36,12)` slot pet học skill 4. Pseudo 25 nhánh fan-out UI, không member-list.
- 0x25: Bear C→S dec 37 = `LoginCompleteHandler → announceAppear`. Pseudo setter trạng thái + chat-log.
- 0x26: pseudo `[26][01][u32]` → `player+0x1334`, tính lại level trần 200 + banner. Bear không dùng dec 38.
- 0x27: Bear S→C `(39,9)` announceAppear; pseudo 43 nhánh bảng rank + invite/banner. Không GM/master.
- 0x2A: Bear C→S dec 42 = `resetHandler` (reset skill/stat qua item). Pseudo banner/chat-log trạng thái chiến đấu.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x23 | OP_GUILD | OP_ACCOUNT |
| 0x24 | OP_GUILD_INFO | OP_JOB_CHANGE |
| 0x25 | OP_GUILD_ACTION | OP_LOGIN_COMPLETE |
| 0x26 | OP_GUILD_BATTLE | OP_EXP_LEVEL |
| 0x27 | OP_SYSTEM_MASTER | OP_RANK_ANNOUNCE |
| 0x2A | OP_FRIEND | OP_RESET |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`, `CONTEXT.md:213` (cập nhật kết luận đã chốt), comment dispatcher/handlers guild nếu có.

## Steps
1. Rename 6 const + alias deprecated + `opcode_name()` + `SERVER_MAIN_OPCODES`.
2. Spec 6 dòng với direction: 0x23 Account (Shared), 0x24 JobChange (S->C), 0x25 LoginComplete (C->S chính), 0x26 ExpLevel (S->C), 0x27 RankAnnounce (S->C), 0x2A Reset (C->S).
3. Grep `OP_GUILD|OP_SYSTEM_MASTER|OP_FRIEND` — chuyển reference nội bộ sang tên mới đúng giá trị.

## Acceptance
- 6 tên mới đúng giá trị; `rg OP_GUILD[^_]` không còn match ngoài alias deprecated. Test xanh.

## Comments
