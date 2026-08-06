# 08 — Move (op 0x06) + Expressions (op 0x20)

**What to build:** Người chơi đã spawn di chuyển quanh map (mọi người cùng map thấy `Walked`), và thể hiện cảm xúc/hành động. Vertical slice nhỏ để xác minh broadcast map hoạt động.

**Blocked by:** 07 — Login + world spawn.

**Status:** completed

- [x] Op 0x06 sub 1 Move: parse dir(1B) + x,y (LE16); **nếu đang battle (`battle_id > 0`) → ignore** (Ch2 §2.3.5 — khớp C# `FWalk.H1`); broadcast `F4440B000601`+le32(id)+dir+le16(x)+le16(y).
- [x] **Leader di chuyển theo nhóm** (C# `_My_IdLeader`/`_My_IdMem1..4`): leader di chuyển self + từng online member (`id_mem`); member đang theo leader không tự broadcast.
- [x] Op 0x20 Expressions: sub1 broadcast `F44407002001`+id+action; sub2 set động tác broadcast `F44407002002`+id+action + lưu `dongtac`; sub3 clear, không packet (Ch2 §2.3.21 — khớp C# `Update_H20`).
- [x] Fan-out toàn map ở server loop: frame op 0x06 sub1 / 0x20 được `broadcast_except` gửi tới các client khác (cùng registry `clients`).
- [x] Golden: move + chat/move scenario được ghi lại (không đổi byte sau khi bổ sung battle-check và leader-follow vì scenario mặc định không battle/không leader).

## Implementation notes

- `handle_move` (src/server/handlers/movement.rs): thêm guard battle + nhánh leader (self + từng `id_mem` nonzero). Thêm field `id_leader` trong `Session` (mặc định 0).
- Broadcast map thật sự do `server_control::handle_client_connection` đảm nhiệm: sau dispatch, frame thuộc `is_map_broadcast` (op 0x06 sub1, op 0x20) được đẩy tới mọi client trong `clients` trừ chính nguồn.
- Test mới: `move_ignored_while_in_battle`, `move_leader_moves_party_members`, `move_member_following_leader_stays_still`.