# 08 — Move (op 0x06) + Expressions (op 0x20)

**What to build:** Người chơi đã spawn di chuyển quanh map (mọi người cùng map thấy `Walked`), và thể hiện cảm xúc/hành động. Vertical slice nhỏ để xác minh broadcast map hoạt động.

**Blocked by:** 07 — Login + world spawn.

**Status:** completed

- [x] Op 0x06 sub 1 Move: parse dir(1B) + x,y (LE16); **nếu đang battle (`battle_id > 0`) → ignore** (Ch2 §2.3.5 — khớp C# `FWalk.H1`); broadcast `F4440B000601`+le32(id)+dir+le16(x)+le16(y).
- [x] **Leader di chuyển theo nhóm** (C# `_My_IdLeader`/`_My_IdMem1..4`): leader di chuyển self + từng online member (`id_mem`); member đang theo leader không tự broadcast.
- [x] Op 0x20 Expressions: sub1 broadcast `F44407002001`+id+action; sub2 set động tác broadcast `F44407002002`+id+action + lưu `dongtac`; sub3 clear, không packet (Ch2 §2.3.21 — khớp C# `Update_H20`).
- [x] Fan-out **cùng map** ở server loop: frame move/expression được đẩy qua `HandleOutcome::map_broadcast` (`(subject_id, frame)`), `ServerControl::broadcast_map` gửi tới mọi client **cùng map** với nguồn (khớp C# `SendToAllClientMapid`).
- [x] Golden: move + chat/move scenario được ghi lại.

## Điều chỉnh fidelity (P1–P5, sau review đối chiếu C#)

- **P1 — không echo về chính nguồn:** C# `Walked` → `SendToAllClientMapid` bỏ qua `client._My_Id == _Id` (Server.cs:607). Frame move/expression **không còn** được `tx.send` về socket nguồn; chỉ đi vào kênh `map_broadcast`. Solo move giờ trả `0` frame — `golden/07-move.golden` đã cập nhật (trước đây khóa 1 frame echo `>>F4440B000601E1930400026400C800`, đó là diverge với C#).
- **P2 — fan-out theo chủ thể từng frame:** mỗi entry `(subject_id, frame)` được broadcast loại đúng client có `id == subject_id` (không phải chỉ loại origin). Member trong nhóm **không nhận** walk frame về chính nó (C# `SendToAllClientMapid(member, ...)` loại `client._My_Id == member`).
- **P3 — phạm vi broadcast là cùng map:** `broadcast_map` lọc theo `map_id` của nguồn từ `online_sessions`; client ở map khác không nhận move/expression (C# `client._My_MapId == nguồn._My_MapId`).
- **P4 — leader move persist vị trí member:** C# `Walked(member)` gọi `Data.PlayerUpdateDataId` (cập nhật `MapX/MapY/Gocnhin`). `handle_move` giờ cập nhật luôn session của từng member trong `online_sessions`.
- **P5 — dispatch:** op 0x05 không còn được route vào `handle_move` (spec chỉ liệt kê 0x06 là Move); dispatch `0x06 => movement::handle_move`.

## Implementation notes

- `HandleOutcome` (src/server/handler.rs) có thêm field `map_broadcast: Vec<MapBroadcast>` + type `MapBroadcast { subject, frame }` + helper `broadcast(subject, frame)`; field `outgoing` chỉ chứa frame dành riêng cho connection hiện tại.
- `handle_move` (src/server/handlers/movement.rs): guard battle + nhánh leader (self + từng `id_mem` nonzero) + persist vị trí member vào `online_sessions` (P4). Thêm field `id_leader` trong `Session` (mặc định 0).
- `handle_expressions` (src/server/handlers/expressions.rs): sub1/sub2 dùng `out.broadcast(self, ...)`; sub3 clear `dongtac`, không packet.
- Fan-out do `ServerControl::broadcast_map` (src/web/server_control.rs) đảm nhiệm: lấy `map_id` của nguồn từ `online_sessions`, gửi từng `MapBroadcast` tới các client cùng map có `id != subject`. Hàm `is_map_broadcast` cũ đã bị xoá (quyết định broadcast nằm ở handler, không phải server loop).
- **Lưu ý map-scoping:** C# `SendToAllClientMapid(id)` đánh giá theo map của **từng chủ thể `id`**; bản port này lấy map của **origin**. Trong flow party-follow các member luôn co-locate với leader nên hai tập trùng khớp; diverge chỉ xảy ra khi leader di chuyển 1 member đang ở map khác (không xảy ra trong cơ chế follow).
- Test mới (RED→GREEN, TDD):
  - `handler.rs`: `move_broadcasts_to_map` (outgoing rỗng, map_broadcast 1 entry), `move_ignored_while_in_battle`, `move_leader_moves_party_members` (3 subject), `move_member_following_leader_stays_still`, `leader_move_persists_member_positions` (P4), `op_005_is_not_routed_to_move` (P5), `expression_handling`.
  - `server_control.rs`: `broadcast_map_scopes_to_same_map_and_skips_subjects` (P2+P3), `broadcast_map_is_noop_without_online_source`.
