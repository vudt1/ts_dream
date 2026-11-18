# 04: Gating Bản đồ Dùng chung vs Bản đồ Thuần, Spawn NPC Động (`16 01`) & Di tản An toàn khi `Stop`

**What to build:** Từ góc nhìn người chơi đang khám phá thế giới trong và ngoài mùa lễ hội, server khóa sạch toàn bộ lối vào bản đồ phụ bản lễ hội thuần khi lễ hội đang `Idle`, đồng thời chỉ gate chi tiết theo `(scene_id, door_id / npc_place_id / quest_id)` trên `4` bản đồ dùng chung (`12000`, `12001`, `12005`, `12421`) để tuyến cốt truyện thường luôn thông suốt; khi Admin bấm `Start`, đồng bộ hiện NPC lễ hội động (`16 01`), và khi bấm `Stop`, di tản an toàn người chơi khỏi bản đồ lễ hội thuần về `savemap`.

**Blocked by:** 02: Wire Parity `S→C 0x24 0x19` (`Festival.Dat` Gốc), `SeasonalNotice` (`0x17 0x39`), Pháo hoa (`0x17 0x46`) & Broadcast Trực tiếp

**Status:** ready-for-agent

- [ ] Khai báo tường minh trong cấu hình mỗi `FestivalProfile`: danh sách `exclusive_maps` (bản đồ lễ hội thuần như `54811..54812`, `54825`, `54827..54831`, `12941..12943`, `57518`, `57531..57536`, `61051`, `61178`, `61551..61567`), danh sách `shared_map_gates` cho `4` bản đồ dùng chung (`12000`, `12001`, `12005`, `12421`, kèm `10995` nếu cần) ở mức `(door_id, npc_place_id, quest_id)`, và danh sách `dynamic_npcs` theo map.
- [ ] Áp dụng chốt chặn gating thống nhất tại các điểm tương tác NPC, va chạm cửa (`MeetDoor`), click cửa (`ClickDoor`) và `Talk-Warp 0x14 08`: từ chối bằng `sys_msg_frame` nếu mục tiêu thuộc lễ hội đang `Idle`; mở khóa bình thường khi lễ hội đang `Active`.
- [ ] Kiểm chứng Negative & Positive Gating trên bản đồ dùng chung: khi tất cả lễ hội đang `Idle`, người chơi vẫn qua lại cổng và nói chuyện NPC cốt truyện thường trên `12000`, `12001`, `12005`, `12421` bình thường, nhưng bị chặn đúng các cổng/NPC/quest lễ hội của các map đó.
- [ ] Đồng bộ spawn NPC lễ hội động (`16 01`) khi người chơi vào game / chuyển map đối với các lễ hội đang `Active`, và broadcast ngay khung hiện NPC tới người chơi đang đứng trên map liên quan khi Admin bấm `Start`.
- [ ] Khi Admin bấm `Stop` một lễ hội, di tản toàn bộ người chơi đang đứng trong `exclusive_maps` của lễ hội đó về `savemap` (fallback `12001` nếu `savemap == 0`, đồng bộ `registry_update_if_online` và lưu SQLite); nếu người chơi đang trong trận chiến (`battle_id > 0`), gắn cờ chờ và thực hiện di tản ngay khi trận đấu hiện tại kết thúc.
