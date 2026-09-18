# 06 — Phase1: Switches 0x21 PkSwitch + 0x2E ServerSwitch

Status: ready-for-agent
Blocked by: 05

## Goal
Rename 2 opcode hệ thống bị gán nhãn tính năng sai.

## Evidence
- 0x21: Bear `WelcomeHandler` dec 33: sub 1 → SwitchPk, sub 2 → SwitchJam; S→C `(33,2)/(33,3)`; `dispatcher.rs:286` đã route 0x21 → `handle_pk_war`. Không MOTD/Welcome.
- 0x2E: pseudo sub 1 chọn server/kênh + reconnect `TClientSocket` port 6414 + banner điều kiện, sub 2 banner, sub 3 dialog config. Không byte thủy chiến.

## Rename
| Op | Cũ | Mới |
|---|---|---|
| 0x21 | OP_WELCOME | OP_PK_SWITCH |
| 0x2E | OP_WATER_WAR | OP_SERVER_SWITCH |

## Files
- `src/protocol/mod.rs`, `spec/server_main_opcode.md`, comment `dispatcher.rs:286` (0x21 → pk_war đã đúng logic, chỉ sửa nhãn).

## Steps
1. Rename + alias deprecated + spec: `$21 PK_SWITCH — công tắc PK/Jam (Shared)`; `$2E SERVER_SWITCH — chọn server/kênh + reconnect 6414 (Shared)`.
2. Kiểm tra không còn comment "Welcome/MOTD" hay "Thủy chiến Xích Bích" cho 2 op này.

## Acceptance
- Tên mới đúng giá trị; test xanh. Kết thúc Phase 1.

## Comments
