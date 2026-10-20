# 13: Dọn dead code battle_role, magic F4441C và API nguy hiểm

**What to build:** (a) `BattleRoleData`/`BattleRoleSerializer` (kèm reader/writer tương ứng) là layout TS Mobile không tồn tại ở client PC — xoá; giữ và tái định vị `ehuman` vì vẫn đang được re-export; (b) `battle_open_leader` hardcode `F4441C` — tính length động từ payload; soát `battle_open_pk_member`, ghi chú magic `7000` có chủ đích; (c) `packets::turn_action(text9)` thiếu W — xoá hoặc sửa để không ai gọi nhầm (`02_battle_positions.md §4.1 #21`); (d) cập nhật doc comment record 23B/24B trong `engine.rs` (`01_battle_start_scene.md §14.8`). Marker kind (64/100) thuộc ticket 14 — không sửa ở đây.

**Blocked by:** None (can start immediately)

**Status:** completed

- [x] Không còn `BattleRoleData`/`BattleRoleSerializer` cùng read/write tương ứng; `ehuman` vẫn import được qua đường public (có test chứng minh)
- [x] Không còn literal `F4441C`; mọi builder `0B FA` tính length từ payload
- [x] `turn_action` không còn ở dạng nguy hiểm (xoá hoặc thêm W đúng chuẩn)
- [x] Doc comment record 24B/23B cập nhật; build/clippy không còn dead-code warning liên quan
