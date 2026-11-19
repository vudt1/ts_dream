# Walkthrough: Xử lý `0x14 0x08` Door Eve Event & Tặng `#19001` + `#31007` tại Quest `10037`

## 1. Thay đổi đã triển khai

### A. Giao thức `0x14 0x08` & `0x14 0x09`
- **[npc_talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/protocol/codecs/npc_talk.rs)**: Thêm hằng số `SUB_CLEAR_PENDING = 0x09` và `NpcTalkCodec::build_clear_pending_hex()` (`"F44402001409"`), giúp xóa cờ chờ `PlayerState+0xA094` trên `aLogin.exe` mà không đóng UI thoại.
- **[talk.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs)**: Trong `handle_talk_warp`, bổ sung guard `if conn.session.current_event_session.is_some()` để phát `0x14 0x09` và giữ nguyên `EventSession` đang chạy.
- **[quest.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs)**:
  - Tách helper dùng chung `collect_prioritized_door_events` ưu tiên nhánh `MeetDoor` (`when_happen[1] || when_happen[2]`) trước các event khác của Door, tái sử dụng cho cả `resolve_door_action` và `resolve_door_warp`.
  - Thêm `DoorActionResolution` (`Warp`, `Event`, `Blocked`) và `resolve_door_action` (giữ nguyên thứ tự kiểm tra `WARP_OVERRIDE_KEYS` → `try_trigger_battle_gate` → `resolve_door_action`, đồng thời tôn trọng fallback `Warps.txt` khi `!door_has_rt2`).
  - Cập nhật `handle_warp_confirm`: kích hoạt `EventSession` khi cửa có hội thoại/kịch bản (phát `06 02` + `14 2C 01` + `activate_event_session`), và phát `14 09` khi `BattleGate` hoặc `EventSession` vào `AwaitingBattle` mà chưa phát `14 01`.

### B. Loại bỏ `31007` lúc tạo nhân vật & tặng `#19001` + `#31007` ở Quest `10037`
- **[character.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/character.rs)** & **[session.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs)**: Xóa `31007 × 1` khỏi `homdo` slot `2` khi khởi tạo nhân vật (giữ nguyên `32012 × 4` ở `homdo` slot `1` và `19737 × 1` ở `trangbi` slot `2`, giữ `characters.newbie = 0`).
- **[eve_runner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs)**:
  - Thêm hằng `FLAG_TUTORIAL_31007 = 1` (lưu trong `Session::bit_flags` / `character_bit_flags`) và helper `resolve_effective_item_mutations`.
  - Tại các map tutorial `10851..=10870`, khi NPC `#15050` (*Nam Tinh Quân*, `eve_no == 3`, Quest `10037`) thưởng `19737`, server tự động đổi thành `#19001` (*Giáp Bạch Bì*) và tặng thêm `1x #31007` (*Vé đoái hoán tâm*) vào `homdo` 1 lần duy nhất mỗi nhân vật.
  - Cả `preflight_probe_batch` (All-or-Nothing) lẫn `execute_mutate_item` (phát `0x14 01`, `17 05`, `sys_msg_frame` `02 0C`, và `registry_update_if_online`) đều dùng chung helper này.

### C. Kiểm thử cập nhật & bổ sung
- **[eve_tcp_and_sysmsg.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/eve_tcp_and_sysmsg.rs)**: Cập nhật `tcp_map_10817_door_14_08_does_not_deadlock` để xác minh chuỗi `14 08 01 00` → `06 02` + `14 2C 01` + `14 01` → `14 06` → `14 2C .. 02` + `14 07` + `0x0C`, kèm kịch bản pure warp `12000 Door 14` và guard `14 09` khi `EventSession` đang mở (`MeetDoor` và `ClickNpc`).
- **[warp_door_sceneinfo_parity.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/warp_door_sceneinfo_parity.rs)**: Bổ sung kiểm chứng `10817 Door 1` mở thoại rồi mới warp `10867`, và `BattleGate` phát `14 09`.
- **[create_char_atomic.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/server_network/create_char_atomic.rs)**: Cập nhật 4 test tạo nhân vật / vào game để khẳng định không cấp `31007` lúc tạo nhân vật.
- **[npc_talk_multistep.rs](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/npc_talk_multistep.rs)**: Bổ sung `npc_15050_quest_10037_grants_19001_and_one_time_31007` kiểm tra nhận `#19001` + `#31007` trên `10867`, tính idempotent qua `bit_flags` trên `10851`, và từ chối All-or-Nothing khi `homdo` đầy `24/25` slot.

## 2. Kết quả kiểm thử

Chạy toàn bộ suite sau lần chỉnh sửa cuối cùng:
```text
cargo test --all-targets --no-fail-fast
```
- **Kết quả**: Thoát mã `0`, toàn bộ target và test đều pass (`architecture_guardrails_test`: 8/8, `battle_test`: 186/186, `eve_quest_test`: 138/138, `server_network_test`: 107/107, `server_stop_test`: 1/1, `skill_job_system_test`: 19/19, cùng các suite còn lại).
