# ĐẶC TẢ KỸ THUẬT (SPEC): CÁC QUYẾT ĐỊNH THIẾT KẾ & KHẮC PHỤC THIẾU SÓT HỆ THỐNG SỦNG VẬT VÀ DROP BATTLE

- **Feature slug:** `pet-and-drop-audit`
- **Tài liệu nghiên cứu gốc:** [research-pet-and-drop-audit.md](file:///D:/VUDT/GIT_PCC/ts_dream/.scratch/pet-and-drop-audit/research-pet-and-drop-audit.md)
- **Thuật ngữ & Ràng buộc:** [GLOSSARY.md](file:///D:/VUDT/GIT_PCC/ts_dream/GLOSSARY.md)
- **Trạng thái:** Đã chốt toàn bộ quyết định thiết kế (Design Decisions Settled via Grilling Session).

---

## 1. TỔNG HỢP CÁC QUYẾT ĐỊNH ĐÃ THỐNG NHẤT (DECISION LOG)

| STT | Vấn đề / Thiếu sót phát hiện | Quyết định đã chốt (Agreed Decision) | Vị trí áp dụng mã nguồn |
|:---:|---|---|---|
| **D1** | Giao dịch sủng vật đang ở trạng thái Xuất chiến (`active_pet_stt == a_stt`) làm desync trạng thái | **Tự động thu hồi sủng vật:** Đặt `a.active_pet_stt = 0`, cập nhật SQLite `character_pets.isactive = 0` và gửi frame recall `F44402001302` về client người bán khi giao dịch thành công. | [src/server/trade_system.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/trade_system.rs), [src/server/handlers/trade_storage.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/trade_storage.rs) |
| **D2** | Dùng thẻ item triệu hồi sủng vật (`add_pet > 10000`) không hiển thị sủng trên client cho đến khi relogin | **Đồng bộ hiển thị tức thì:** Lưu ngay vào SQLite (`persist::upsert_pet`), gửi ngay frame dữ liệu sủng `0F 07` và frame hiện diện `0F 01` về client để sủng hiển thị ngay trên UI. | [src/server/handlers/use_item/mod.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/use_item/mod.rs) |
| **D3** | Bắt quái trong trận (Battle Capture) bị rỗng chỉ số (stats = 0, kỹ năng = 0) | **Nạp đầy đủ chỉ số gốc từ `Npc.dat`:** Giữ cấp độ `level = 1`, nạp đầy đủ Element, MaxHP, MaxSP, INT, ATK, DEF, AGI và 3-4 kỹ năng mặc định từ template NPC gốc. Người chơi tự luyện từ cấp 1. | [src/server/pet_box.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/pet_box.rs), [src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) |
| **D4** | Tỷ lệ rơi đồ khi Hợp kích (Combo Kill) | **Giữ nguyên bảng tỷ lệ tĩnh:** Duy trì 6 băng xác suất `[25, 23, 20, 4, 3, 1]` trên thang 1000 (~7.6%). Giữ nguyên cơ chế mỗi ô tham gia combo thành công cấp 1 phần thưởng độc lập về túi đồ chủ nhân (`owner = caster.id_char` nếu là Pet). | [src/battle/runner.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/runner.rs) |
| **D5** | Rơi đồ trong Battle khi túi đồ (`homdo`) của người chơi đã đầy 25/25 ô | **Hủy lượt nhận đồ và báo đỏ:** Kiểm tra sức chứa hòm đồ, nếu không còn ô trống thì hủy nạp vật phẩm và gửi thông báo chữ đỏ *"Hành trang đã đầy, không thể nhận vật phẩm"*. | [src/battle/service.rs](file:///D:/VUDT/GIT_PCC/ts_dream/src/battle/service.rs) |

---

## 2. CHI TIẾT ĐẶC TẢ KỸ THUẬT & MÃ THỰC THI

### 2.1. Quyết định 1: Thu hồi Sủng vật xuất chiến khi Trade (D1)
- **Tập tin:** `src/server/trade_system.rs` & `src/server/handlers/trade_storage.rs`
- **Logic:**
  1. Trong `pet_trade_settle(a: &mut Session, b: &mut Session)`:
     ```rust
     if let Some(p) = ap {
         if a.active_pet_stt == a_stt {
             a.active_pet_stt = 0;
         }
         // ...
     }
     if let Some(p) = bp {
         if b.active_pet_stt == b_stt {
             b.active_pet_stt = 0;
         }
         // ...
     }
     ```
  2. Trong `handle_pet_confirm`:
     Khi phát hiện người chơi có pet bị chuyển đi trùng với `active_pet_stt` cũ:
     - Gửi frame recall: `ctx.out.send("F44402001302")`.
     - Cập nhật DB: `db::persist::update_player(pool, player_id, "SttPetXuatchien", 0).await`.

---

### 2.2. Quyết định 2: Đồng bộ Sủng vật tức thì khi Dùng Thẻ Item (D2)
- **Tập tin:** `src/server/handlers/use_item/mod.rs`
- **Logic:**
  Trong nhánh `if add_pet > 10000`:
  1. Sau khi push vào `session.pets`:
     ```rust
     let snap = ctx.conn.session.pets.last().cloned().unwrap();
     if let Some(pool) = ctx.pool {
         persist::upsert_pet(pool, ctx.conn.session.id, &snap).await;
     }
     ```
  2. Gửi các gói tin đồng bộ về client:
     - Frame chi tiết: `0F 07 <pet_id: 4B LE><stt: 1B><pet_id: 4B LE><quest: 1B><name_len: 1B><name>`
     - Frame hiện diện: `0F 01 <owner_id: 4B LE><stt: 1B><pet_id: 4B LE><quest: 1B>`

---

### 2.3. Quyết định 3: Khởi tạo Sủng vật bắt được từ Template `Npc.dat` (D3)
- **Tập tin:** `src/server/pet_box.rs` & `src/battle/service.rs`
- **Logic:**
  Cập nhật hàm `add_caught`:
  ```rust
  pub fn add_caught(
      pets: &mut Vec<PetState>,
      npc: &crate::data::tables::Npc,
  ) -> Option<u8> {
      let npc_id = npc.id as u16;
      if pets.iter().any(|p| p.id == npc_id) {
          return None;
      }
      let stt = next_active_slot(pets)?;
      pets.push(PetState {
          stt,
          id: npc_id,
          name: npc.name.clone(),
          level: 1, // Theo quyết định: giữ cấp độ 1 để người chơi tự luyện
          thuoctinh: npc.element as u8,
          hp: npc.hp_max as u16,
          hp_max: npc.hp_max as u16,
          sp: npc.sp_max as u16,
          sp_max: npc.sp_max as u16,
          int1: npc.int_val as u16,
          atk: npc.atk as u16,
          def: npc.def as u16,
          agi: npc.agi as u16,
          hpx: npc.hpx as u16,
          spx: npc.spx as u16,
          skill1_id: npc.skill1 as u16,
          skill1_level: if npc.skill1 > 0 { 1 } else { 0 },
          skill2_id: npc.skill2 as u16,
          skill2_level: if npc.skill2 > 0 { 1 } else { 0 },
          skill3_id: npc.skill3 as u16,
          skill3_level: if npc.skill3 > 0 { 1 } else { 0 },
          skill4_id: npc.skill4 as u16,
          skill4_level: if npc.skill4 > 0 { 1 } else { 0 },
          fai: 60, // Điểm trung thành khởi điểm mặc định
          ..Default::default()
      });
      Some(stt)
  }
  ```

---

### 2.4. Quyết định 5: Kiểm tra Sức chứa Hòm đồ khi Rơi đồ Battle (D5)
- **Tập tin:** `src/battle/service.rs` (`apply_drop`)
- **Logic:**
  ```rust
  if s.homdo.iter().filter(|i| i.slot >= 1 && i.slot <= 25 && i.id > 0).count() >= 25 {
      // Hòm đồ đã đầy: Hủy nhận và gửi cảnh báo chữ đỏ
      out.send_to(
          owner,
          crate::server::handlers::system::sys_msg_frame(
              "Hành trang đã đầy, không thể nhận vật phẩm."
          ),
      );
      return;
  }
  ```
