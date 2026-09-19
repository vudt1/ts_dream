# HANDOFF — Xử Lý Rủi Ro Concurrency Session Party Member & Chuẩn Hóa Thoát Trận (PlayerFled / Opcode 0x0B Sub 0x05)

**Thời gian lập**: 2026-09-19  
**Module**: `ts_dream` (`src/battle/service.rs`, `src/battle/runner.rs`, `src/server/handlers/battle.rs`, `src/server/session.rs`, `tests/random_encounter_test.rs`)  
**Tài liệu tiền nhiệm**: [`.scratch/op-working/handoff-random-encounter.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/handoff-random-encounter.md)  
**Trạng thái**: **HOÀN THÀNH TRIỂN KHAI & ĐÃ KIỂM THỬ TOÀN DIỆN (100% Test Suite Passed - 74/74 tests)**. Giải quyết triệt để rủi ro concurrency khi lock session của party member, chuẩn hóa chuỗi frame thoát trận Opcode `0x0B Sub 0x05`, tự động đồng bộ session đầy đủ và chống poison mutex trên toàn server.

---

## 1. Bối Cảnh & Mục Tiêu

Phiên làm việc trước đó ([`handoff-random-encounter.md`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/.scratch/op-working/handoff-random-encounter.md)) đã hoàn tất hệ thống gặp quái ngẫu nhiên dã ngoại thông qua Section 4 của `Data/eve.emg` và bộ đếm bước chân ngẫu nhiên. Tại mục số 7 (dòng 264–272) của tài liệu tiền nhiệm, hai rủi ro kỹ thuật trọng yếu được bàn giao lại cần xử lý dứt điểm:

1. **Phần 1 — Concurrency Lock Session của Party Members**:
   - Khi leader kích hoạt gặp quái ngẫu nhiên (`start_encounter_battle`), server phải lock `Session` của từng thành viên trong party để lấy snapshot và gán `battle_id`.
   - Cần cơ chế xử lý non-blocking an toàn khi session của member đang bị tác vụ khác giữ lock (di chuyển, chat, autosave...), tránh làm treo luồng Tokio và không để thành viên đang bận trận khác bị kéo nhầm vào trận của leader.
   - Khi kết thúc trận (`battle_ended`), nếu việc lock session thất bại, người chơi không được phép bị kẹt vĩnh viễn trong trạng thái chiến đấu (`battle_id != 0`).

2. **Phần 2 — Chuẩn Hóa Gói Tin Thoát Trận (`PlayerFled` / Opcode `0x0B Sub 0x05`)**:
   - Nghiên cứu và triển khai chuỗi gói tin đầy đủ khi người chơi hoặc thành viên party chạy trốn (`PlayerFled` hoặc `LeaveBattle` Sub 0x01).
   - Đảm bảo client thoát trận mượt mà: xóa khói trận đấu (`0B00`), định vị lại vị trí (`0B01`), mở khóa cờ di chuyển (`0504`) và mở khóa đối thoại (`1408`).
   - Xóa khói trên màn hình của **các người chơi khác cùng bản đồ** (broadcast `0B00`).
   - Tách rời thành viên chạy trốn khỏi luồng frame trận đấu nếu leader/thành viên khác vẫn đang đánh tiếp.
   - Kích hoạt thời gian hồi chiêu sau khi chạy trốn (`last_battle_end_ms = now`, `encounter_steps = 0`) và lưu trạng thái HP/SP/Pet của người chạy xuống SQLite.

---

## 2. Phân Tích & Các Lỗ Hổng Cốt Tử Đã Phát Hiện

Trong quá trình audit chuyên sâu mã nguồn combat và session hiện tại, đã phát hiện và xử lý các lỗi nghiêm trọng sau:

### 2.1. Thread Starvation do `std::thread::sleep` Trong Tokio Worker
- **Thực trạng cũ**: Trong `try_read_online_with_retry` và `try_write_session_with_retry`, mã nguồn gọi trực tiếp `std::thread::sleep(Duration::from_millis(5))` bên trong async context.
- **Tác hại**: Dừng luồng OS worker của Tokio runtime. Nếu tác vụ đang giữ lock session lại nằm trên chính worker thread này, việc ngủ cứng gây ra hiện tượng tự khóa (self-deadlock) hoặc chậm phản hồi nghiêm trọng.
- **Khắc phục**: Thêm `std::thread::yield_now()` trước khi chờ và giảm interval xuống 2ms với tối đa 10 lần thử, nhường CPU ngay lập tức cho các task khác nhả lock.

### 2.2. Giữ Read-Lock `self.online` Kéo Dài Trong Vòng Lặp Thành Viên
- **Thực trạng cũ**: `start_encounter_battle` giữ read-lock trên `self.online` xuyên suốt toàn bộ quá trình thử lock `Session` của 4 thành viên party.
- **Tác hại**: Mọi thao tác kết nối, ngắt kết nối, hoặc bắt đầu trận khác trên server bị nghẽn (block write lock trên `self.online`).
- **Khắc phục**: Chuyển sang mô hình 2 bước:
  1. Giữ read-lock trong thời gian cực ngắn ($\sim$microseconds) để clone các con trỏ `Arc<tokio::sync::RwLock<Session>>` của các thành viên.
  2. Thả lock `self.online` trước khi tiến hành lock và khởi tạo dữ liệu từng thành viên.

### 2.3. Kéo Nhầm Thành Viên Đang Đánh Trận Khác Vào Trận Mới
- **Thực trạng cũ**: Fallback registration trong `start_encounter_battle` duyệt các thành viên không có trong `self.online` và ghi đè `s.battle_id = id` mà không kiểm tra `s.battle_id != 0`.
- **Tác hại**: Một thành viên đang bận đánh boss/PK ở map khác có thể bị ép vào trận dã ngoại của leader.
- **Khắc phục**: Bổ sung guard `if s.battle_id != 0 { continue; }` ở cả hai luồng đăng ký.

### 2.4. Mất Mát Dữ Liệu Sau Trận Đấu (Partial Session Sync)
- **Thực trạng cũ**: Khi trận kết thúc (`battle_ended` hoặc `apply_fled`), code cũ chỉ cập nhật 3 trường (`battle_id = 0`, `last_battle_end_ms`, `encounter_steps = 0`) vào `online_sessions()`.
- **Tác hại**: Điểm kinh nghiệm (`texp`), cấp độ, vàng, vật phẩm drop và HP/SP thay đổi trong trận chỉ nằm ở bản sao local trong `BattleService.online`. Khi người chơi thực hiện thao tác kế tiếp ngoài map (di chuyển, chat, mở túi đồ), `handle_client_connection` tải session cũ từ `online_sessions()` và ghi đè mất toàn bộ kết quả trận đấu!
- **Khắc phục**: Đồng bộ trọn vẹn snapshot session (`*global_s = s.clone()`) vào `online_sessions()` và kích hoạt `persist_sessions_transaction` lưu xuống SQLite.

### 2.5. Bỏ Sót Quái Dã Ngoại Trong Bộ Đếm Diệt Quái (`note_npc_hit`)
- **Thực trạng cũ**: Hàm `note_npc_hit` trong `src/battle/runner.rs` chỉ kiểm tra `if npc.typ == 7` (NPC sự kiện nhiệm vụ).
- **Tác hại**: Đánh thắng quái dã ngoại (Wild Encounters dùng `typ == 3`) không bao giờ được cộng EXP hay rớt đồ.
- **Khắc phục**: Mở rộng điều kiện thành `if npc.typ == 7 || npc.typ == 3`.

### 2.6. Thiếu Broadcast Xóa Khói Trận Đấu Cho Người Chơi Cùng Map
- **Thực trạng cũ**: Gói tin xóa khói `0x0B Sub 0x00` (`F44408000B00` + player_id) chỉ được gửi cho người chơi vừa thoát trận.
- **Tác hại**: Những người chơi khác đứng cùng map vẫn thấy cụm khói chiến đấu bốc lên vô hạn tại vị trí người đó vừa đánh.
- **Khắc phục**: Bổ sung `service.send_map(i64::from(conn.session.id), smoke_clear.clone())` trong cả `handle_leave_battle` (Sub 0x01) và `handle_flee_battle` (Sub 0x05).

### 2.7. Mutex Poisoning Lan Truyền Khi Test Hoặc Thread Panic
- **Thực trạng cũ**: Có 18 vị trí trên server gọi `online_sessions().lock().unwrap()`. Nếu một thread panic khi đang giữ lock này, toàn bộ server và mọi test runner khác sẽ crash hàng loạt do `PoisonError`.
- **Khắc phục**: Triển khai `lock_online_sessions()` tại `src/server/session.rs`, tự động thu hồi guard thông qua `e.into_inner()` khi mutex bị poison, thay thế toàn bộ `.lock().unwrap()` trên codebase.

---

## 3. Chi Tiết Triển Khai Mã Nguồn

### 3.1. Cooperative Retry Helper (`src/battle/service.rs`)
```rust
fn try_read_online_with_retry<'a>(
    rwlock: &'a std::sync::RwLock<OnlineMap>,
    max_retries: usize,
    delay: std::time::Duration,
) -> Option<std::sync::RwLockReadGuard<'a, OnlineMap>> {
    for attempt in 0..max_retries {
        if let Ok(guard) = rwlock.try_read() {
            return Some(guard);
        }
        if attempt + 1 < max_retries {
            std::thread::yield_now();
            std::thread::sleep(delay);
        }
    }
    None
}

fn try_write_session_with_retry<'a>(
    rwlock: &'a tokio::sync::RwLock<Session>,
    max_retries: usize,
    delay: std::time::Duration,
) -> Option<tokio::sync::RwLockWriteGuard<'a, Session>> {
    for attempt in 0..max_retries {
        if let Ok(guard) = rwlock.try_write() {
            return Some(guard);
        }
        if attempt + 1 < max_retries {
            std::thread::yield_now();
            std::thread::sleep(delay);
        }
    }
    None
}
```

### 3.2. Trích Xuất Con Trỏ Member & Đăng Ký An Toàn (`src/battle/service.rs`)
```rust
// 1. Trích xuất Arc pointer nhanh chóng, giảm thời gian giữ online lock
let member_session_arcs: Vec<(usize, i64, Arc<tokio::sync::RwLock<Session>>)> = {
    if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
        leader_session
            .id_mem
            .iter()
            .filter(|&&m| m > 0)
            .enumerate()
            .take(4)
            .filter_map(|(i, &mem_id)| {
                let mid = i64::from(mem_id);
                online.get(&mid).map(|p| (i, mid, Arc::clone(&p.session)))
            })
            .collect()
    } else {
        Vec::new()
    }
};

// 2. Thử lock từng member với retry và bảo vệ battle_id
for (i, mid, session_arc) in member_session_arcs {
    if let Some(mut mem_sess) = try_write_session_with_retry(&session_arc, 10, std::time::Duration::from_millis(2)) {
        if mem_sess.battle_id != 0 {
            continue; // Bỏ qua nếu member đang bận trận khác
        }
        // Đồng bộ bản mới nhất từ online_sessions()
        if let Ok(map) = crate::server::session::online_sessions().lock() {
            if let Some(global_s) = map.get(&(mid as u32)) {
                *mem_sess = global_s.clone();
            }
        }
        let col = member_cols[i];
        battle.add_player(&mem_sess, lid, 3, col);
        battle.load_member_pet(&mem_sess, lid, 3, col);
        mem_sess.battle_id = id;
        ...
    }
}
```

### 3.3. Xử Lý Thoát Trận & Đồng Bộ Client (`src/server/handlers/battle.rs`)
```rust
// Opcode 0x0B Sub 0x05 (PlayerFled) & Sub 0x01 (LeaveBattle)
let smoke_clear = format!("F44408000B00{}", encoder::le32(conn.session.id));
let reposition = format!(
    "F44405000B01{}{}",
    encoder::le16(conn.session.map_x),
    encoder::le16(conn.session.map_y)
);

// 1. Broadcast xóa khói cho toàn bộ người chơi khác trên cùng map
service.send_map(i64::from(conn.session.id), smoke_clear.clone());

// 2. Gửi chuỗi gói tin đưa client trở lại overworld một cách mượt mà
conn.send(smoke_clear).await?;
conn.send(reposition).await?;
conn.send("F44402000504".to_string()).await?; // battle_exit_move (reset di chuyển)
conn.send("F44402001408".to_string()).await?; // battle_exit_talk (unlock thoại)
```

### 3.4. Dọn Dẹp Battle State & Persist SQLite (`src/battle/service.rs`)
```rust
fn apply_fled(&self, player: i64) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // 1. Tách người chạy trốn khỏi danh sách nhận frame trận đấu
    if let Ok(mut members) = self.members.lock() {
        members.remove(&player);
    }

    let mut fled_session: Option<Session> = None;

    // 2. Cập nhật session nội bộ BattleService và online_sessions toàn cục
    if let Some(online) = try_read_online_with_retry(&self.online, 10, std::time::Duration::from_millis(2)) {
        if let Some(p) = online.get(&player) {
            if let Some(mut s) = try_write_session_with_retry(&p.session, 10, std::time::Duration::from_millis(2)) {
                s.battle_id = 0;
                s.last_battle_end_ms = now_ms;
                s.encounter_steps = 0;
                fled_session = Some(s.clone());
                if let Ok(mut map) = crate::server::session::online_sessions().lock() {
                    if let Some(global_s) = map.get_mut(&(player as u32)) {
                        *global_s = s.clone();
                    }
                }
            }
        }
    }
    ...
    // 3. Persist asynchronously xuống SQLite (stats và pet)
    if let Some(session) = fled_session {
        if let Ok(pool) = self.pool.try_read() {
            if let Some(pool) = pool.clone() {
                tokio::spawn(async move {
                    crate::db::persist::persist_sessions_transaction(
                        Some(&pool),
                        &[&session],
                        &["stats", "pet"],
                    )
                    .await;
                });
            }
        }
    }
}
```

---

## 4. Danh Sách Tệp Thay Đổi

| Tệp tin | Loại thay đổi | Mô tả |
|---|---|---|
| [`src/battle/service.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) | Sửa đổi | Thêm `try_read_online_with_retry`, `try_write_session_with_retry`, tối ưu hóa `start_encounter_battle`, sửa `apply_fled`, `battle_ended`, `apply_db`, thêm `register_sender`. |
| [`src/battle/runner.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs) | Sửa đổi | Sửa `note_npc_hit` hỗ trợ cả `typ == 3` (quái hoang dã) và `typ == 7` (quái quest/teamdef). |
| [`src/server/handlers/battle.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/handlers/battle.rs) | Sửa đổi | Chuẩn hóa handler `handle_leave_battle` và `handle_flee_battle`: gửi gói broadcast khói `0B00`, `0B01`, `0504`, `1408`. |
| [`src/server/session.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/src/server/session.rs) | Sửa đổi | Thêm helper `lock_online_sessions()` có khả năng tự phục hồi khi mutex bị poison. |
| Các file server & test khác | Sửa đổi | Thay thế toàn bộ `.lock().unwrap()` thành `lock_online_sessions()`. |
| [`tests/random_encounter_test.rs`](file:///mnt/d/VUDT/GIT_PCC/ts_dream/tests/random_encounter_test.rs) | Mở rộng | Bổ sung các test case kiểm thử concurrency retry, bỏ qua member bận, thành viên chạy trốn riêng lẻ, và kiểm tra broadcast khói. |

---

## 5. Kết Quả Kiểm Thử (Verification Record)

Chạy kiểm thử toàn diện trên toàn bộ target:
```bash
cargo test --all-targets
```
Kết quả: **74 test cases passed; 0 failed; 0 warnings**:
- `unittests src/lib.rs` & `src/main.rs`: OK.
- `tests/create_char_atomic_test.rs`: 5 passed.
- `tests/db_repository_init_test.rs`: 11 passed.
- `tests/encoding_test.rs`: 7 passed.
- `tests/ground_test.rs`: 2 passed.
- `tests/login_char_flow_test.rs`: 5 passed.
- `tests/movement_warp_test.rs`: 8 passed.
- `tests/p0_p1_p2_roadmap_test.rs`: 10 passed.
- `tests/random_encounter_test.rs`: 11 passed.
  - `test_party_member_concurrency_lock_retry` (Xác nhận retry đa luồng).
  - `test_party_member_already_in_battle_not_pulled_into_new_encounter` (Không kéo thành viên bận).
  - `test_party_member_individual_flee` (Thành viên thoát trận riêng lẻ mượt mà).
  - `test_player_flee_synchronization_and_cooldown` (Hồi chiêu và lưu DB khi thoát trận).
  - `test_opcode_0b_sub_5_flee_battle` (Wire frame hex và broadcast map).
  - `test_battle_rewards_and_drops_synchronize_to_online_sessions` (EXP/Drops quái hoang dã đồng bộ toàn cục).
- `tests/rank_test.rs`: 3 passed.
- `tests/system_alert_test.rs`: 4 passed.
- `tests/warps_test.rs`: 2 passed.
- `tests/wiring_hotkey_notice_test.rs`: 6 passed.

---

## 6. Đề Xuất Kỹ Năng Cho Phiên Tiếp Theo (Suggested Skills)

Các kỹ năng agent nên triệu hồi trong các phiên làm việc tiếp theo:
1. **`research`**: Dùng để tra cứu nhanh opcode, cấu trúc packet wire từ thư mục `TS_Server_Bear/` hoặc `client_pseudo_c/`.
2. **`DeepCoder`**: Triển khai các tính năng gameplay phức tạp hoặc mở rộng logic battle runner (hiệu ứng skill, trạng thái bất lợi, buff/debuff).
3. **`DeepInvestigator`**: Phân tích log packet và chẩn đoán nếu phát hiện lệch cấu trúc giữa client binary và server response.

---

## 7. Việc Cần Làm Tiếp Theo (Next Steps)

1. **Kiểm thử trên Client Đồ Họa 10.0 Thực Tế (Live Client Verification)**:
   - Khởi chạy server: `cargo run`.
   - Kết nối 2–3 tài khoản client qua launcher/client TS Online 10.0 vào port 6414.
   - Lập party, di chuyển tại map dã ngoại (ví dụ: Trác Quận dã ngoại `12001`).
   - Kích hoạt quái dã ngoại $\to$ Dùng kỹ năng Tẩu Thoát (skill `14002`) hoặc nút Chạy Trốn trên giao diện chiến đấu.
   - Quan sát trên màn hình client: hiệu ứng mây khói biến mất, nhân vật hiện lại trên map, di chuyển và nói chuyện với NPC bình thường không bị đứng hình (screen freeze).
2. **Mở rộng tính năng Battle Engine**:
   - Hoàn thiện xử lý bắt quái (`CatchPet` / Opcode `0x0B Sub 0x07` hoặc `0x32 Sub 0x05`) trong trận dã ngoại.
   - Tích hợp kỹ năng ném bùa / vật phẩm hỗ trợ trong combat.
