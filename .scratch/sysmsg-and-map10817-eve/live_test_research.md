# Báo cáo nghiên cứu sơ cấp: Live-Test Bugs (Map 10817 / NPC 15009, Homdo Split/Sort) & Tối ưu quy trình `cargo run`

Tài liệu này tổng hợp kết quả kiểm chứng trực tiếp từ mã nguồn Rust (`src/`), test suite (`tests/`), cấu hình build (`Cargo.toml`, `build.rs`, `ts_dream.toml`), tài liệu đặc tả (`spec/`, `docs/`, `.scratch/`), và mã dịch ngược client `aLogin.exe` (`client_pseudo_c/`). Mọi kết luận đều trích dẫn đúng file và khoảng dòng đã đọc; những điểm chưa có body hàm client hoặc capture thực tế được đánh dấu rõ ràng là **`[Giả thuyết]`** hoặc **`[Chưa có body callee]`**.

---

## Mục 1: Quest nói chuyện NPC 15009 (Bạch Lão Nhân) tại map 10817 mất animation ở bước cuối và warp ngay sang map 10867

### 1.1. Hai kịch bản Eve tại map 10817 dẫn sang map 10867

Kiểm tra các test đọc trực tiếp từ dữ liệu Eve của map `10817` cho thấy có **hai trigger khác nhau** đều có chuỗi hội thoại chuyển sang map `10867`:

1. **Click NPC 4 (`npc_id = 15009`, Bạch Lão Nhân)**:
   - `tests/eve_quest/npc_eve_resolve.rs:60-72`: `NpcTrigger::ClickNpc(4)` trên map `10817` phân giải ra `eve_no = 4` với `results.len() == 7`, trong đó phần tử cuối `results[6]` là `result_type = 2` (`Door` / `Warp`). Trong 7 result của `eve_no = 4` không có step `PlayMovie` (`result_type = 5`) — tức là 6 bước `Talk` (`result_type = 1`) nối tiếp thẳng bằng `Warp` (`result_type = 2`).
2. **Chạm / kích hoạt Door 1 (`MeetDoor` hoặc `C->S 14 08 01 00`)**:
   - `tests/eve_quest/warp_door_sceneinfo_parity.rs:706-743` và `tests/server_network/eve_tcp_and_sysmsg.rs:252-315`: Door `1` trên map `10817` phân giải ra `eve_no = 7` (`MeetDoor`), gồm chuỗi thoại `14 01` qua 6 lần `14 06`, sau đó hoàn tất bằng `14 2C .. 02` (Unlock) + `14 07` (Warp Ack) + `0x0C` (relocate sang map `10867`).
   - Dữ liệu `eve_no = 7` trên map `10817` có 6 `TalkStep` (`rt = 1`) theo sau bởi 2 step `PlayMovie` (`rt = 5`) và 1 step `Warp` (`rt = 2`, `param = 1` -> `10867`). *(Lưu ý: `AGENTS.md` cấm grep/truy cập trực tiếp `Data/`, nên chi tiết số lượng `rt=5` trong `eve_no=7` được đối chiếu theo trạng thái code và test hiện có; các step `rt=5` cũng được kiểm chứng tại `tests/eve_quest/missing_eve_effect_executors.rs:340-355`.)*

### 1.2. Nguyên nhân gốc trên Server: `PlayMovie` (`rt=5`) & `MoveEntity` (`rt=4`) không block bước tiếp theo, nên `Warp` (`rt=2`) bắn cùng batch TCP

Chuỗi xử lý trong [`src/eve/planner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/planner.rs) và [`src/server/handlers/eve_runner.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/eve_runner.rs) diễn ra như sau:

1. **Ánh xạ `EveResultType` sang `DomainEffect` (`src/eve/planner.rs:179-194`)**:
   - `result_type == 1` → `DomainEffect::TalkStep`
   - `result_type == 2` → `DomainEffect::Warp { warp_id }`
   - `result_type == 4` → `DomainEffect::MoveEntity`
   - `result_type == 5` → `DomainEffect::PlayMovie`
   - `result_type == 6` → `DomainEffect::AwaitMenuChoice`
2. **`is_blocking_or_terminal()` bỏ qua `PlayMovie` và `MoveEntity` (`src/eve/planner.rs:157-165`)**:
   ```rust
   pub fn is_blocking_or_terminal(&self) -> bool {
       matches!(
           self,
           Self::TalkStep(_)
               | Self::AwaitMenuChoice { .. }
               | Self::Warp { .. }
               | Self::StartBattle { .. }
       )
   }
   ```
3. **Vòng lặp thực thi trong `execute_event_step_core` (`src/server/handlers/eve_runner.rs:963-1069`)**:
   - Khi người chơi bấm tiếp tục hội thoại (`C->S 0x14 0x06`), [`handle_talk_continue`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L230-L250) tăng `ev.current_index` lên `1` và gọi `execute_event_step_with_pool`.
   - Tại `process_single_domain_effect` (`src/server/handlers/eve_runner.rs:830-846`):
     - `DomainEffect::MoveEntity(res)` gửi `14 01` (`NpcTalkCodec::build_talk_step_hex(res)`) và trả về **`StepOutcome::Continue`**.
     - `DomainEffect::PlayMovie(res)` gửi `14 01` (`NpcTalkCodec::build_talk_step_hex(res)`) và trả về **`StepOutcome::Continue`**.
   - Vòng `loop` trong `execute_event_step_core` tăng `ev.current_index` ngay lập tức (dòng `1065-1068`) mà **không dừng lại chờ client**.
   - Ngay vòng lặp kế tiếp, khi gặp `DomainEffect::Warp { warp_id }` (`src/server/handlers/eve_runner.rs:1001-1048`):
     - Chạy `run_trailing_effects_before_warp` (dòng `1023`, tiếp tục chạy mọi effect non-blocking sau `Warp` nếu có).
     - Gửi `14 2C <char_id> 02` (Unlock, dòng `1025-1031`).
     - Gọi [`perform_party_warp_with_resolved`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1316-L1320) (dòng `1038-1041`), phát ngay **`14 07`** (`build_warp_ack_hex`) và **`0x0C`** (`build_relocate_packet` tới `dest_map = 10867`).
   - **Hệ quả**: Toàn bộ các frame `14 01 (rt=5 / rt=4)` cuối cùng + `14 2C 02` + `14 07` + `0x0C (map 10867)` đều được push vào cùng một `HandleOutcome` của một lần `handle_talk_continue` và gửi xuống socket trong **cùng một burst TCP**.

### 1.3. Cơ chế phía Client `aLogin.exe`: tại sao animation/movie bị mất và warp ngay lập tức

Đối chiếu các handler S->C của `aLogin.exe` trong `client_pseudo_c/`:

1. **Khi nhận `14 01` với `result_type = 5` (`PlayMovie`)**:
   - [`FUN_0078ec3f`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078ec3f_FUN_0078ec3f.c#L96-L155) (`case 1..6`) copy 14 byte `EveResult` vào `*gvar_007DA5A0 + 0xa09b` và gọi `FUN_005eb530`.
   - Trong [`FUN_005eb530`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005eb530_FUN_005eb530.c#L741-L766) (`case '\x05'`):
     - Gán cờ chế độ movie: `*(gvar_007DA37C + 0x1d) = 1` (dòng `748`).
     - Khởi tạo `TMovie` (`gvar_007DA434`, dòng `750-755`) và gọi [`FUN_00618668`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/00618668_FUN_00618668.c#L26-L70) (dòng `764`) để phát hoạt cảnh/movie theo `result_value`.
   - Trong vòng lặp render chính [`FUN_005157e6`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005157e6_FUN_005157e6.c#L180-L224), client chỉ tick/phát `TMovie` theo từng frame UI khi cờ `*(DAT_009264a4 + 0x1d)` đang ở trạng thái movie (`1`, `2`, `3`).
2. **Khi nhận tiếp `14 07` và `0x0C` ngay trong cùng batch**:
   - [`FUN_0078ec3f` `case 7`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0078ec3f_FUN_0078ec3f.c#L156-L165) (`14 07`) bật `0xa097 = 1`, `0xa096 = 1`, `0xa094 = 0`.
   - Ngay sau đó [`FUN_0078d84d`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/case_012_0078D84D_FUN_0078d84d.c#L42-L66) (MainOp `0x0C`) set `*(gvar_007DA5A0 + 0xc) = 1`, `*(gvar_007DA37C + 0xc) = 1` và gọi [`FUN_007994a4`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/007994a4_FUN_007994a4.c#L102-L123) (nạp map mới `10867`, reset cờ trạng thái dialog `0xa097 = 0`, gửi lại `C->S 0x0C 0x01`, và xóa cờ `0x44f`/`0x1d` khi ở trạng thái `2`).
   - Kết quả là client chưa kịp render các frame của `TMovie` (hoặc chuyển cảnh nạp map `10867` ngay lập tức đè lên/cắt ngang hoạt cảnh cuối).
3. **Ghi chú phân biệt trigger tại 10817**:
   - **`[Đã xác minh]`**: Nếu người chơi bấm thẳng vào NPC `15009` (`ClickNpc(4)` -> `eve_no = 4`), bản thân `eve_no = 4` chỉ có 6 `TalkStep` + 1 `Warp` (không có `rt = 5` trong chain đó, theo `tests/eve_quest/npc_eve_resolve.rs:60-72`), nên bấm hết 6 câu thoại sẽ warp thẳng sang `10867`. Ngược lại, nếu kịch bản đi qua Door `1` (`MeetDoor` / `14 08 01 00` -> `eve_no = 7`), hoặc bất kỳ NPC/quest nào có `rt = 5` (`PlayMovie`) / `rt = 4` (`MoveEntity`) ngay trước `rt = 2` (`Warp`), server hiện tại chắc chắn làm mất animation cuối do `PlayMovie`/`MoveEntity` trả `StepOutcome::Continue` trước `Warp`.
   - **`[Giả thuyết]`**: Cần đối chiếu thêm bằng live packet capture (`TS_PACKET_LOGGING=true`) khi tương tác tại map `10817` xem client đang gửi `14 01 04 00` (click NPC 15009) hay `MeetDoor`/`14 08 01 00` (Door 1) và xem client có tự gửi `14 06` hay một gói kết thúc movie nào sau `14 01 (rt=5)` trước khi muốn server thực thi `Warp` (`rt=2`) hay không.

---

## Mục 2: Inventory (`homdo`) tách (split) / sắp xếp (sort) không redraw `homdo` ngay trên client

### 2.1. Hiện trạng xử lý `0x17` trên server (`ts_dream`)

Trong [`src/server/dispatcher.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/dispatcher.rs#L306-L309), toàn bộ Opcode `0x17` (`OP_ITEM`) đi vào [`handle_inventory`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs#L34-L88). Bảng `match sub` tại `src/server/handlers/inventory.rs:53-87` chỉ xử lý:
- `2` (pickup -> gửi `17 02` + `17 06` via `item_added_frame`),
- `3` (drop -> gửi `17 03` + `17 09`),
- `10` (`0x0A`, `handle_move_stack`),
- `11` (`0x0B` equip -> gửi `17 11` + `dump_trangbi`, **không** `dump_homdo`),
- `12` (`0x0C` unequip -> gửi `17 10` + `dump_trangbi` + `dump_homdo` tại dòng `448`),
- `14, 15, 17, 18, 20, 30..=33, 36, 37, 0x39, 0x3A, 45, 46, 51|52, 0x6E`,
- **`_ => {}`** (mọi subcode khác bị bỏ qua im lặng).

### 2.2. Hai/ba lỗ hổng cụ thể gây lỗi ở `split` và `sort`

#### A. Lỗi tách item (`split`) tại `handle_move_stack` (`C->S 0x17 0x0A`)
[`handle_move_stack`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs#L221-L323) nhận `payload[0] = oldslot`, `payload[1] = count`, `payload[2] = newslot`.

1. **Bug tách stack vào ô trống (`dst == None`, `src/server/handlers/inventory.rs:271-282`)**:
   ```rust
   None => {
       conn.session.homdo[src_idx].slot = newslot;
       let moved = conn.session.homdo[src_idx].clone();
       let empty = InventoryItem { slot: oldslot, ..Default::default() };
       persist::upsert_item(pool, conn.session.id, "homdo", &empty).await;
       persist::upsert_item(pool, conn.session.id, "homdo", &moved).await;
       out.send(hex_of(decoded));
   }
   ```
   - Code không hề kiểm tra `count < src_count`! Khi người chơi **tách một phần** (`count < src_count`) từ `oldslot` sang ô trống `newslot`, server di chuyển **toàn bộ stack** sang `newslot` và xóa trắng `oldslot` trong RAM + SQLite.
   - Đáng chú ý, test hiện có tại [`tests/economy_inventory/inventory_roundtrip.rs:192-249`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/economy_inventory/inventory_roundtrip.rs#L192-L249) mới chỉ kiểm tra case move trang bị (`count=1`) sang ô trống và merge toàn phần (`count=10` vào slot có sẵn `15`), chưa hề test case split `count < src_count` sang ô trống (`dst == None`).
2. **Bug khi `dst` đã có item khác loại hoặc gộp vượt trần 50 (`src/server/handlers/inventory.rs:309-320`)**:
   - Chỉ xử lý nhánh `else if count == src_count` (swap hai ô); nếu `count < src_count`, hàm kết thúc im lặng mà không gửi frame nào.
3. **Chỉ echo `17 0A <oldslot> <count> <newslot>`, không gửi frame đồng bộ lại `homdo` (`17 05` hoặc `17 08` / `17 09`)**:
   - `handle_move_stack` ở cả 3 nhánh (`equipment`, `None`, `Some`) chỉ gọi `out.send(hex_of(decoded))`, tức phát lại đúng S->C `17 0A <oldslot> <count> <newslot>`. Không hề gọi `conn.session.dump_homdo()` (`17 05`, định nghĩa ở `src/server/session.rs:662-665, 756-766`) hay `17 08` / `17 09`.

#### B. Kiểm chứng hành vi S->C `17 0A` vs `17 05` / `17 08` / `17 09` trên `aLogin.exe`
Đối chiếu trực tiếp trong `client_pseudo_c/`:
- **C->S `17 0A`**: `asm_cs17_783405_builders.asm.txt:275-335` (`0x0078371E`) build gói 5 byte `17 0A <+6> <+7> <+8>` đọc từ `*gvar_007DA0D0` (`TThingManage` tạo tại `0050a4a0_TForm1.FormCreate.c:664-665`), qua helper [`FUN_0076cebc`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0076cebc_FUN_0076cebc.c#L19-L26) (gán `+6=oldslot, +7=count, +8=newslot` rồi gọi `FUN_0077f414(..., 0x17)`).
- **S->C `17 0A`**: [`FUN_007902fb` `case 10`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/007902fb_FUN_007902fb.c#L176-L178) gọi [`FUN_0076c378`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0076c378_FUN_0076c378.c#L20-L74) (và asm tại [`0076c378_FUN_0076c378.asm.txt:44-50`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/0076c378_FUN_0076c378.asm.txt#L44-L50)). Hàm này chỉ đọc 3 byte `RP[1], RP[2], RP[3]` và gọi `FUN_0072b538(*(int *)gvar_007DA7BC, oldslot, count, newslot)`.
  - **`[Chưa có body callee]`**: `FUN_0072b538` không có file decompile trong `client_pseudo_c/`. Tuy nhiên, đối chiếu với các hàm S->C túi đồ khác cùng file `007902fb`:
    - **S->C `17 05`** (`FUN_0076bfc8`, `0076bfc8_FUN_0076bfc8.c:77-207`): Xóa sạch 25 ô `*(player + 0x9e0 + slot*4)` qua `VMT+8`, rồi nạp lại `N` record 12 byte qua `VMT+4` (`[slot][id:u16LE][qty][doben][long][giatri][khang][texp:u32LE]`). Đây chính là frame `dump_homdo()` (`src/server/session.rs:662-665, 726-738`) mà server gửi khi đăng nhập (`src/server/spawn.rs:614`) và khi tháo đồ (`handle_unequip`, `src/server/handlers/inventory.rs:448`).
    - **S->C `17 08`** (`FUN_0076974c`, `0076974c_FUN_0076974c.c:151-152`): Ghi trực tiếp 1 ô xác định vào `*(player + 0x9e0 + slot*4)` qua `VMT+4`.
    - **S->C `17 09`** (`FUN_0076ae24`, `0076ae24_FUN_0076ae24.c:62-66`): Gọi `FUN_0072b8c8` trừ số lượng theo `slot`, đồng thời kiểm tra `FUN_007686e4` để cập nhật phím tắt `FUN_005a5bc8`.
    - **S->C `17 11` / `17 17`** (`FUN_0076c248.c:52`, `FUN_0076c2d0.c:61`): Đều gọi thêm `FUN_005b0640(*gvar_007D9E5C, ...)` để refresh form UI (`TSe_StatusInfoForm`).
  - **`[Giả thuyết]`**: Khi `FUN_0076c378` (`17 0A`) chỉ gọi `FUN_0072b538` cập nhật dữ liệu `TThing` trên `LocalActor` (`gvar_007DA7BC`), nó có thể không tự kích hoạt hàm vẽ lại lưới ô của form `TCY_ThingMenu` (`0051189c_FUN_0051189c.c:1320`, không có file `.c` riêng trong `client_pseudo_c/`), hoặc khi server gửi sai `17 0A` do bug split (server dời nguyên stack trong DB/RAM còn client nhận echo `17 0A` với `count` cũ), trạng thái UI không khớp cho tới khi mở lại/relog để nhận `17 05` (`dump_homdo`). Ngoài ra, trong `handle_unequip` (`inventory.rs:448`), server phát `conn.session.dump_homdo()` (`17 05`) để ép client đồng bộ lại toàn bộ 25 ô túi đồ.

#### C. Lỗ hổng chức năng sắp xếp (`sort`) `homdo`
- Trong `src/server/handlers/inventory.rs:53-87`, **không hề có nhánh xử lý sort `homdo`**.
- Trong bảng jump-table C->S `0x17` (`cs17_783405_jmptbl_61xDWORD.hex` + `asm_cs17_783405_builders.asm.txt`), ngoài `17 0A` (`0x0078371E`), có các nhánh C->S khác không được xử lý trong `inventory.rs` như `17 04` (`0x00783635`), `17 3B` (`0x0078514D`), `17 3C` (`0x007851D1`, frame 2 byte `17 3C` không có payload thêm).
- **`[Giả thuyết]`**: Tùy cách client `aLogin.exe` thực hiện nút Sắp xếp trong `TCY_ThingMenu`:
  1. Nếu client tự chạy thuật toán sort trên 25 ô rồi bắn **chuỗi các gói C->S `17 0A`** (move/merge/swap từng cặp ô): server chỉ echo `17 0A` mà không gửi `17 05` (`dump_homdo`), đồng thời bỏ qua các cặp move/split không thỏa nhánh `handle_move_stack` → `homdo` trên UI không redraw đúng cho đến khi relog (khi `login` gọi `dump_homdo()` ở `spawn.rs:614`).
  2. Nếu nút Sort gửi một **subcode `0x17` riêng** (chẳng hạn `17 3C` hoặc một subcode khác chưa match trong `inventory.rs:53-87`), gói đó rơi thẳng vào `_ => {}` tại `src/server/handlers/inventory.rs:86` — server không sort `session.homdo`, không lưu DB, và không gửi bất kỳ frame nào về client.
  - **Cách xác nhận nhanh nhất**: Bật `TS_PACKET_LOGGING=true` (xem Mục 3) và bấm Split / Sort trên `aLogin.exe` để xem chính xác raw hex C->S mà client gửi lên server.

---

## Mục 3: Chạy `cargo run` nhanh cho live-testing khi `cargo build` chậm

### 3.1. Tại sao `cargo build` / `cargo run` trong repo này chậm?

Từ kiểm tra trực tiếp [`Cargo.toml`](file:///D:/VUDT/GIT_PCC/ts_dream/Cargo.toml), [`build.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/build.rs), và cấu trúc thư mục:

1. **Dependencies được ép biên dịch ở `opt-level = 2` ngay trong `dev` profile (`Cargo.toml:40-44`)**:
   ```toml
   [profile.dev]
   opt-level = 0

   [profile.dev.package."*"]
   opt-level = 2
   ```
   - Các crate nền nặng như `tokio` (feature `full`), `axum 0.8`, `sqlx 0.8` (`sqlite`, `runtime-tokio-rustls`, `migrate`), `askama 0.12`, `serde`, `tracing-subscriber` (`Cargo.toml:20-35`) bị compile bằng `-O2` ở lần build đầu tiên (hoặc sau `cargo clean`, hoặc khi đổi biến môi trường/feature/target).
   - Ngược lại, bản thân crate `ts_dream` vẫn compile ở `opt-level = 0`, nên **khi dependencies đã cache**, các lần sửa code trong `src/` chỉ compile lại `ts_dream` (nhanh hơn nhiều, chỉ tốn thêm bước link trên Windows).
   - Repo **không có** `.cargo/config.toml` (đã kiểm tra `Test-Path .cargo` và `.cargo/config.toml` đều trả về `False`), nên hiện chưa cấu hình linker nhanh hơn hay rustflags tùy chỉnh.
2. **`build.rs` quét `Data/` và theo dõi `DB/ts_dream.db` (`build.rs:15-59`)**:
   - `build.rs:18`: `println!("cargo:rerun-if-changed=Data");` và dòng `26-27` gọi `sync_dir(&src, &profile_dir.join("Data"), &[])` để duyệt toàn bộ **27 file (~15.3 MB)** trong `Data/` (so sánh `len` và `modified` qua `needs_copy`, dòng `94-108`) và copy sang `target/<profile>/Data/`.
   - `build.rs:35-38`: `println!("cargo:rerun-if-changed=DB/ts_dream.db");` (cố ý không watch thư mục `DB/` vì test ghi `DB/ts_dream_test.db*`, dòng `32-34`).
   - **Bẫy làm `build.rs` chạy lại mỗi lần live-test**: `ts_dream.toml:17` mặc định trỏ tới `database_url = "sqlite://DB/ts_dream.db"`. Khi chạy server live-test tại root repo, SQLite mở và ghi vào `DB/ts_dream.db` → **thay đổi mtime của `DB/ts_dream.db`** → ở lần chạy `cargo run` / `cargo build` tiếp theo, Cargo thấy `DB/ts_dream.db` đổi nên chạy lại `build.rs` (quét lại `Data/` + `DB/` rồi sync sang `target/debug/`) và có thể kích hoạt rebuild/relink crate!
3. **Có tới 2 binary và 11 integration test crates**:
   - `Cargo.toml:7-14`: có 2 `[[bin]]` là `ts_dream` (`src/main.rs`) và `patch_festival_dat` (`src/bin/patch_festival_dat.rs`). Không có `default-run`, nên chạy `cargo run` trơn sẽ báo lỗi mơ hồ vì không rõ chọn binary nào.
   - Thư mục `tests/` có **11 integration test binaries** (`architecture_guardrails_test.rs`, `battle_test.rs`, `compound_test.rs`, `data_loaders_test.rs`, `db_test.rs`, `economy_inventory_test.rs`, `eve_quest_test.rs`, `events_arena_test.rs`, `server_network_test.rs`, `server_stop_test.rs`, `skill_job_system_test.rs`). Chạy `cargo build --all-targets` hay `cargo test --all-targets` sẽ phải link tất cả các test binary đó trên Windows.

### 3.2. Cách chạy nhanh và chuẩn xác đã xác minh trong repo

1. **Chạy server live-test ở chế độ debug (incremental, tránh `-O3` của `--release`)**:
   ```powershell
   cargo run --bin ts_dream
   ```
   - `src/main.rs:102-107` & `src/server/handlers/npc_event.rs:28`: Eve-event engine (`EVE_EVENTS_ENABLED = true`) đã bật sẵn mặc định (không cần ép thêm nếu không đổi, vẫn hỗ trợ `TS_EVE_EVENTS=1`).
   - Mặc định đọc `ts_dream.toml` tại CWD (`src/config.rs:102-108`), lắng nghe Game TCP `127.0.0.1:6414` và Web Admin Dashboard `http://127.0.0.1:8090/` (`ts_dream.toml:5-11`, `src/main.rs:138-170`, `PRODUCTION_RUNBOOK.md:5,58`).
2. **Bật log gói tin C2S/S2C trực tiếp không cần sửa `ts_dream.toml`** (tránh trigger `rerun-if-changed=ts_dream.toml` ở `build.rs:53`):
   ```powershell
   $env:TS_PACKET_LOGGING="true"
   cargo run --bin ts_dream
   ```
   - Biến môi trường `TS_PACKET_LOGGING` override `ts_dream.toml` (`src/config.rs:140-141`, `ts_dream.toml:37-39`), đẩy toàn bộ frame C2S/S2C ra Live SSE Terminal trên Dashboard (`http://127.0.0.1:8090/`).
3. **Cách ly DB live-test để không chạm `DB/ts_dream.db` (tránh kích hoạt lại `build.rs`)**:
   - Sao chép `DB/ts_dream.db` thành một file riêng (ví dụ `DB/ts_dream_dev.db`) hoặc trỏ ra ngoài file được `build.rs` theo dõi (`build.rs:35-38`):
     ```powershell
     $env:TS_DATABASE_URL="sqlite://DB/ts_dream_dev.db"
     cargo run --bin ts_dream
     ```
   - `src/config.rs:145-147, 189-206` hỗ trợ override `TS_DATABASE_URL` trực tiếp; còn `TS_DATA_DIR` (`src/config.rs:142-144, 159-179`) dùng khi chạy từ thư mục khác không có `./Data`.
   - Cũng **không** chạy `cargo clean` hay `cargo update` (nhắc lại quy chuẩn `AGENTS.md`) vì sẽ xóa cache `-O2` của toàn bộ dependencies trong `target/debug/deps/`.
4. **Kiểm tra code nhanh trước khi live-test / chạy test theo domain**:
   - Kiểm tra lỗi biên dịch mà không tốn thời gian link `.exe`:
     ```powershell
     cargo check --bin ts_dream
     ```
   - Chạy đúng test domain đang sửa thay vì `--all-targets` (`AGENTS.md`):
     ```powershell
     cargo test --test eve_quest_test
     cargo test --test economy_inventory_test
     cargo test --test server_network_test
     ```
   - Chỉ khi handoff cuối mới chạy full suite: `cargo test --all-targets --no-fail-fast`.
5. **Nếu đã có sẵn binary đã build và chỉ test lại thao tác với client (không sửa Rust code)**:
   - Chạy thẳng `.\target\debug\ts_dream.exe` (do `build.rs` đã đóng gói `Data/`, `DB/`, `ts_dream.toml` cạnh binary tại `target/debug/`, và `Config::resolve_data_dir` / `resolve_database_url` trong `src/config.rs:159-206` fallback sang thư mục chứa `.exe` nếu chạy ở ngoài repo root), bỏ qua hoàn toàn thời gian gọi Cargo.

---

## Phụ lục (Bổ sung từ `packet_10817_current.txt` & 3 lỗi live-test mới)

### A. Đối chiếu `packet_10817_current.txt` (NPC 15009 / Map 10817 → 10867)
- Trong [`packet_10817_current.txt`](file:///D:/VUDT/GIT_PCC/ts_dream/packet_10817_current.txt#L1-L22):
  - Dòng 1: `c2s F444040014010400` → client gửi **`ClickNpc(4)`** (NPC `15009` *Bạch Lão Nhân*), không phải `14 08 01 00` (`Door 1`).
  - Dòng 2–14: server phát `06 02`, `14 2C .. 01` (Lock), và đúng **6 step `14 01`** với `result_type = 01` (`TalkStep`: `CF28`..`D428` = `10447`..`10452`).
  - Dòng 15–18: sau `c2s 14 06`, server phát `14 2C .. 02` (Unlock) + `14 07` + `0C .. 732A` (relocate sang map `10867`).
  - Điều này khớp hoàn toàn với [`tests/eve_quest/npc_eve_resolve.rs:60-72`](file:///D:/VUDT/GIT_PCC/ts_dream/tests/eve_quest/npc_eve_resolve.rs#L60-L72): nhánh `eve_no = 4` (`ClickNpc(4)`) trong `eve.emg` chỉ có 7 result gồm **6 `Talk` (`rt=1`) + 1 `Door/Warp` (`rt=2`)**, bản thân `eve_no = 4` **không có** step `PlayMovie` (`rt=5`) hay `MoveEntity` (`rt=4`) nào bị bỏ qua ở server. (Các step `PlayMovie`/`MoveEntity` nếu có ở event khác như `Door 1` mới gặp bug không block trước `Warp` như đã phân tích ở Mục 1.)
  - Dòng 19–22: sau khi client tải xong map `10867` (`c2s 0C 01`), server trả `05 04` + `14 08` ([`handle_teleport_confirm`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/system.rs#L181-L188)), rồi client gửi tiếp `c2s 14 06` (dòng 22) ở cuối capture.

### B. Màu tóc, màu da, áo, quần, giày không lưu xuống DB
1. **Parse từ packet tạo nhân vật (`0x09 0x01`)**:
   - [`parse_create`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/character.rs#L38-L76) đọc `hair: u16::from(payload[2])` (1 byte kiểu tóc) và `color_hex: encoder::hex(&payload[4..12])` (**8 byte = 16 hex chars** chứa bảng màu tóc/da/áo/quần/giày).
   - [`apply_to_session`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/character.rs#L238-L240) chỉ giữ `session.color = data.color_hex.clone()` **trong RAM**.
2. **Schema DB và Repository bỏ rơi hoàn toàn `session.color`**:
   - Bảng `characters` trong [`migrations/0001_init.sql:42-81`](file:///D:/VUDT/GIT_PCC/ts_dream/migrations/0001_init.sql#L42-L81) chỉ có cột `hair INTEGER DEFAULT 0` (chú thích nhầm là đã bao gồm `color1/color2`), **không có cột `color` / `color1` / `color2`**.
   - [`create_and_seed`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L555-L594) chỉ `INSERT` `seed.hair` (kiểu tóc 1 byte), **không ghi `session.color`**.
   - [`save_in_tx`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L320-L337) chỉ `UPDATE hair=?`, **không ghi `session.color`**.
   - [`load`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/modern/sqlite/session.rs#L20-L88) chỉ đọc `c.hair`, **không nạp `session.color`**, nên khi đăng nhập lại `session.color` giữ mặc định `"0000000000000000"` (`src/server/session.rs:506`) và bị gửi xuống client ở [`build_logined_sequence_session_ext`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/spawn.rs#L558-L578) cũng như [`handle_login_complete`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/login.rs#L245-L265).
   - Ngoài ra, [`handle_reborn`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/inventory.rs#L508-L512) có cập nhật `conn.session.color` trong RAM nhưng [`character_column`](file:///D:/VUDT/GIT_PCC/ts_dream/src/db/persist.rs#L24-L54) cũng không có cột lưu màu.

### C. NPC hay cổng warp chưa tới gần đã kích hoạt nói chuyện hoặc warp tức thì
1. **`C->S 0x06 0x01` nhảy trước tọa độ `session` sang điểm đích ngay lúc bắt đầu đi**:
   - Khi người chơi click di chuyển tới một điểm trên map, client gửi `06 01 <dir> <destX> <destY> ...` ngay khi bắt đầu bước đi.
   - [`handle_move_inner`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L214-L219) gán ngay `conn.session.map_x = x; conn.session.map_y = y;` bằng tọa độ đích `(destX, destY)`.
   - Ngay sau đó [`handle_move_dispatch`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/movement.rs#L67-L150) kiểm tra `MeetDoor` trong vòng `±60 px` quanh `(destX, destY)` → nếu điểm click đích gần cổng warp, server warp/mở event **tức thì** khi nhân vật trên client mới bắt đầu di chuyển.
2. **Ngưỡng khoảng cách NPC quá rộng & cổng warp click (`14 04`, `14 08`) không kiểm tra khoảng cách**:
   - [`resolve_npc`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L104-L127) (`14 01`) dùng bounding box **`-150..=150` px** (lớn hơn ngưỡng `130 px` ghi chú ở [`src/eve/state.rs:192-205`](file:///D:/VUDT/GIT_PCC/ts_dream/src/eve/state.rs#L192-L205)), cộng với việc `06 01` đã cập nhật trước `map_x/map_y` sang đích nên server cho talk khi nhân vật chưa tới nơi.
   - [`handle_talk_click_door`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L346-L401) (`14 04`) và [`handle_warp_confirm`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/quest.rs#L1376-L1543) (`14 08`) **không hề kiểm tra khoảng cách** giữa `(session.map_x, session.map_y)` và tọa độ `door` trên map.

### D. Map `#10917` NPC `#16015` (Tiểu Nhị) bị dính sang NPC `#16009` (Chủ Lò Rèn)
- Đối chiếu log packet của user với [`src/server/handlers/talk.rs`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs):
  1. `c2s F444040014010200` → click `map_object_id = 2` (`template_id = 16015`).
  2. Nhánh [`handle_talk_start`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L160-L171) match `16015` và gửi `F44411001401000000010603020000000000000100` (`parameter = 02 00`, `surface = 01 00`).
  3. Client chọn menu `0x1E` (`30`) qua `14 09 1E` và tiếp tục qua `14 06`.
  4. [`handle_talk_continue`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L296-L299) gọi `handle_inn_menu(conn, data, out, "02")`.
  5. Trong [`handle_inn_menu`](file:///D:/VUDT/GIT_PCC/ts_dream/src/server/handlers/talk.rs#L313-L334) (cả `30 =>` lẫn `32 =>`), server **hardcode `0100`** (`parameter = 1`) trong `"F4441100140100000001060301000000000000{}00"` thay vì lấy `conn.session.idtalking` (`0200`), tạo ra đúng gói `s2c F44411001401000000010603010000000000000200` trong log.
  6. Phía client [`FUN_005eb530`](file:///D:/VUDT/GIT_PCC/ts_dream/client_pseudo_c/005eb530_FUN_005eb530.c#L782-L818) dùng `parameter` (`local_50 + 6`) làm **map object ID** của NPC đang thoại (`*(gvar_007DA6DC + uVar9 * 4)` và `*(local_8 + 0xa074)`), nên `01 00` làm client chuyển ngữ cảnh thoại sang NPC object ID `1` trên map `10917` (`#16009` Chủ Lò Rèn).
  - *Ghi chú thêm*: Trong `talk.rs`, `16015` đang được ghép chung với nhóm Tiền trang ở dòng `160` (`16080 | 16004 | 16011 | 16015`, gửi `0100` ở cuối) nhưng lại tách sang `handle_inn_menu(..., "02")` ở dòng `296` (khác với `16080 | 16004 | 16011 | 16023` ở dòng `271`) — khi sửa cần rà soát lại đúng vai trò/menu của `16015` ngoài việc thay `0100` bằng `encoder::le16(conn.session.idtalking as u16)`.
