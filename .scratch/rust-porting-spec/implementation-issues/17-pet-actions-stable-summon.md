# 17 — Pet actions (op 0x0F) + pet stable (op 0x1F) + summon/recall (op 0x13)

**What to build:** Người chơi quản lý pet trọn vòng đời: triệu hồi/hồi lại, thả, cho vào/ra chuồng, đổi vị trí, mount/horse, đặt tên, và triệu hồi trong battle. Pets theo nhân vật trong `pet` table (`player_id`, `stt`).

**Blocked by:** 07 — Login + spawn (pet summary dumps); 04 — DB (`pet` table); 11 — Inventory (pet items equippable / pet stable slot).

**Status:** completed

- [ ] Op 0x0F: sub2 release; sub3 store → `F44405001F06`+stt+`0000`, `UpdateStatusPetWhenUseItem`, broadcast `F4440C000F01`; sub7 take-from-stable (red msg + `F44402001F09` nếu active); sub8 swap (red bx if fighting `F44402001F09F44402001F0C`); sub4 mount horse (id `18000..19000`, `F4440E000F05`); sub5 unmount `F44406000F06`; sub6 rename broadcast `F444`+len+`0F09` (Ch2 §2.3.11).
- [ ] Op 0x1F pet stable menu — ngữ nghĩa equal 0x0F sub3/7/8 nhưng reply `F44405001F060000` và menu kết `F44402001F09`/`1F0C` (Ch2 §2.3.20).
- [ ] Op 0x13 summon/recall (ngoài battle): sub1 set active `F44406001301`+id; sub2 clear + `F44402001302`; in battle (cell `_Attacked==0`): summon loads pet, removes player's battle cells, spawn pet `ChangedWar` type 4, `F4441A000B0505`+warPacket + `F44406001301`+id; recall tương tự (Ch2 §2.3.12).
- [ ] Pet statuses computed và gửi correct; thao tác `pet` mang `player_id`.
- [ ] Golden: pet scenario được ghi lại (Ch9 §9.6).

## Review parity (2026-08-08)

**Kết luận:** Chưa đáp ứng ticket. `0x0F` và summon/recall ngoài battle mới là scaffold in-memory; `0x1F` mapping và summon/recall trong battle chưa có. Giữ `Status: ready-for-agent`; checklist gốc vẫn mở.

### Thuật ngữ miền phải dùng nhất quán

- **Pet Roster Slot**: slot mang theo/chiến đấu, legacy `stt 1..4`.
- **Stable Slot**: slot chuồng, legacy C# quét `stt 5..10` tại `Client.cs:1825-1831,6041-6047`; Rust hiện khóa `5..8` tại `src/server/pet_box.rs:6-23`. Đây là discrepancy phải chốt bằng standing decision/data capture trước khi sign-off.
- **Active Summon**: `active_pet_stt`/legacy `_SttPetXuatChien`; khác với việc Pet tồn tại trong Roster và khác battle cell.
- Mọi đổi slot là đổi composite identity `(player_id, stt)` và phải chuyển cả pet equipment slots tương ứng, không chỉ sửa một field trong RAM.

### Contract correction: ticket đang đảo nghĩa subcode legacy

- C# `0x0F/sub3` và `0x1F/sub2`: request mang stable index, source thực là `packet[6] + 4`, chuyển **Stable -> Roster** (`Client.cs:1790-1812,6006-6028`).
- C# `0x0F/sub7` và `0x1F/sub3`: chuyển **Roster -> Stable**, có active-pet guard (`Client.cs:1814-1850,6030-6066`).
- C# `0x0F/sub8` và `0x1F/sub4`: swap `stable_index + 4` với roster slot (`Client.cs:1852-1878,6068-6095`).
- Vì vậy tên “store”/“take” ở checklist dòng 9 và dispatcher Rust hiện tại không thể dùng làm authority; implementation phải theo mapping C# trên, rồi cập nhật wording acceptance khi capture xác nhận.

### Ma trận acceptance

| Capability | Trạng thái | Evidence Rust | Evidence C# |
|---|---|---|---|
| `0x0F/sub2` release | Partial | `src/server/handlers/pet_actions.rs:15-28` | `Client.cs:1780-1788`; `Data.cs:2301-2312` |
| `0x0F/sub3/7/8` roster/stable | Partial, mapping sai | `pet_actions.rs:29-50,88-118` | `Client.cs:1790-1878` |
| mount/unmount/rename | Partial | `pet_actions.rs:52-87` | `Client.cs:1880-1919`; `Data.cs:2283-2297,2471-2481` |
| `0x1F` stable menu | Missing | `pet_actions.rs:123-129` | `Client.cs:6002-6097` |
| `0x13` ngoài battle | Partial | `pet_actions.rs:131-154` | `Client.cs:1926-1964` |
| `0x13` trong battle | Missing | Không có branch theo `battle_id` | `Client.cs:1966-2067` |
| status + `player_id` durability | Partial | `src/server/spawn.rs:246-345`; `src/db/persist.rs:278-304,449-504` chưa được gọi | `Data.cs:1376-1474,2102-2131,5687-5694` |
| golden pet lifecycle | Partial | `golden/14-pet.golden`; `tests/common/mod.rs:191-202` | Ch9 §9.6 |

### Vấn đề và giải pháp bắt buộc

1. **Action chỉ mutate local RAM.** Release/store/take/swap/rename/active summon không gọi persistence; reconnect phục hồi state cũ. Giải pháp: domain operation transaction theo `player_id`; slot move/swap phải rewrite/update cả hai composite keys và pet equipment, sau đó publish session state.
2. **Map broadcasts đang thành self-only.** `out.send` tại `pet_actions.rs:24-25,41-49,61-67,73-85` chỉ đi caller; C# broadcast map/all-client. Giải pháp: dùng registry/map broadcast seam, giữ self echo/order theo C#.
3. **Roster/stable flow thiếu status, summary và equipment move.** Rust chỉ sửa `PetState.stt`; `spawn::pet_status_single` tồn tại nhưng không được gọi. C# gọi `SwitchPet`, `SendStatusPet`, `UpdateStatusPetWhenUseItem`, summary và map broadcast. Giải pháp: một atomic `move_pet_slot/swap_pet_slots` cập nhật pet + 6 equipment slots rồi recompute và emit đúng sequence.
4. **`0x1F` dispatch sai.** Rust forward sub `3|7|8` thẳng sang handler `0x0F` (`pet_actions.rs:123-128`), trong khi C# dùng `2|3|4` với mapping riêng. Giải pháp: typed stable command decoder, remap đúng payload semantics, không reuse subcode bằng numeric coincidence.
5. **Mount parse sai width/guards.** C# đọc pet ID LE32, yêu cầu `18000 < id < 19000`, không phải active pet và phải đang mang theo (`Client.cs:1882-1895`). Rust đọc LE16, range inclusive, thiếu active guard và broadcast. Packet `0F05` phải mang pet LE32. Unmount chỉ phát khi đang mount.
6. **Rename/release không durable.** Rename cần giữ raw VISCII bytes, update row `(player_id, stt)`, map broadcast `0F09`; release phải clear active summon nếu cần, xóa/reset pet row và equipment ownership rồi broadcast `0F02`.
7. **Summon ngoài battle parse sai ID và thiếu gate.** Request là LE32 (`Client.cs:1934-1940`), Rust chỉ đọc 2 byte (`pet_actions.rs:139-143`). Giải pháp: parse `data[6..9]`, reject mounted pet, require roster slot `<=4`, persist Active Summon; recall chỉ ack khi active pet tồn tại.
8. **Summon/recall trong battle chưa tồn tại.** C# chỉ cho cell player `_Attacked==0`, clear companion cell, `ChangedWar` type 4, broadcast `F4441A000B0505 + warPacket`, set action consumed và active state (`Client.cs:1966-2067`). Giải pháp: command gửi vào owner battle task; battle task là authority duy nhất cho grid mutation và packet order; DB/session update sau command thành công.
9. **Live battle integration cần kiểm tra.** Battle model có seams trong `src/battle/construction.rs:142-175`, nhưng `0x13` không route vào battle manager. Không mutate battle grid trực tiếp từ connection task.
10. **Capacity không nhất quán.** C# stable scan 5..10, ticket/spec/Rust nói 5..8; add-pet item còn chặn theo `pets.len() >= 4` (`src/server/handlers/use_item/mod.rs:297-319`). Phải chốt một invariant roster/stable và dùng ở add, trade, stable, login dump.

### Tests / golden

- Unit tại `pet_actions.rs:165-218` chỉ cover mount/recall local happy path và còn seed request LE16 sai legacy.
- `golden/14-pet.golden` chỉ cover summon/recall ngoài battle với fixture Rust; không cover release, stable, swap, rename, mount, status, persistence hay battle summon.
- Acceptance cần capture C# cho từng mapping `0x0F/0x1F`, roster-full/stable-full/active guard, mount/rename, và battle summon/recall; thêm MySQL tests chứng minh mọi row có đúng `player_id` và không orphan equipment.

### Notes / deferred

- Ticket gốc không có Notes/deferred. Không defer persistence, status recompute, broadcasts, `0x1F`, hoặc battle summon/recall.
- Việc chốt Stable Slot `5..8` hay `5..10` cần capture/data decision, nhưng không miễn implementation khỏi dùng một invariant duy nhất xuyên suốt.

## Implementation record (2026-08-08)

Implemented end-to-end; subcode semantics follow the C# review correction, NOT the checklist wording:

- **`0x0F sub 3` / `0x1F sub 2`** = Stable → Roster (source `packet[6]+4`, first free roster slot 1..4; replies `F44405001F06`+src+`0000`, `F4440C000F01`+self+free+petid+`01`, trailer `F44402001F0C`).
- **`0x0F sub 7` / `0x1F sub 3`** = Roster → Stable (active-pet guard → red msg + `F44402001F09`; first free stable slot 5..10; broadcasts `F44407000F02`+self+stt; trailer `F44402001F09`).
- **`0x0F sub 8` / `0x1F sub 4`** = swap stable (`packet[6]+4`) ↔ roster (`packet[7]`) with active guard.
- `0x0F sub 4/5` mount/unmount: LE32 pet id, `18000 < id < 19000`, must-be-carried gate, `F4440E000F05`/`F44406000F06` self+map.
- `0x0F sub 6` rename: raw VISCII bytes preserved (`encoder::strhex`), broadcast `0F09`.
- `0x13` summon/recall: LE32 pet id; summon requires roster slot + not mounted; out-of-battle ack `F44406001301`+id LE32 and persists `SttPetXuatchien`; recall ack `F44402001302`. In-battle stays quiet (battle task owns the grid, ticket 17/20 seam).
- Slot moves are the composite `(player_id, stt)` op incl. relocating pet equipment (`trangbi` slots `stt*10+1..6`); persisted via `persist_sessions_transaction(["pet","trangbi"])`.

Provenance: `Client.cs:1776-2074`, `:6002-6097`; `Data.cs:2215-2346,5605-5695`; `golden/14-pet.golden` re-encoded to the LE32 summon layout.
