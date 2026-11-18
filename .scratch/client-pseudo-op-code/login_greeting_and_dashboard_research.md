# Báo Cáo Nghiên Cứu Kỹ Thuật: Khắc Phục Lỗi Mở Màn Hình Đăng Nhập Client (aLogin.exe) & Tối Ưu Giao Diện Dashboard Web Admin

**Ngày lập**: 2026-11-17  
**Phân hệ**: Client Network Greeting (`aLogin.exe`), Web Admin Dashboard (`ts_dream`)  
**Tài liệu đối chiếu gốc**:
- Client Decompiled C & x86 Assembly: `ts_decompile/functions/`, `ts_decompile/case_functions/`
- Nghiên cứu sơ bộ: [`.scratch/client-pseudo-op-code/login_flow_research.md`](login_flow_research.md), [`.scratch/client-pseudo-op-code/reconnect_heartbeat_flow.md`](reconnect_heartbeat_flow.md), [`.scratch/client-pseudo-op-code/login_first_step_analysis.md`](login_first_step_analysis.md)
- Web Admin: [`templates/dashboard.html`](../../templates/dashboard.html), [`src/web/app.rs`](../../src/web/app.rs), [`src/web/server_control.rs`](../../src/web/server_control.rs), [`src/server/spawn.rs`](../../src/server/spawn.rs)

---

## 1. Vấn Đề 1: Client Báo "iD không chính xác" & Đứng Im Khi Mở Form Chọn Server

### 1.1. Hiện tượng & Nhật ký Live SSE
Khi chạy thực tế với client `aLogin.exe` kết nối đến Rust Server, trên Live SSE log của Dashboard xuất hiện:
```text
s2c - F444030001095A
c2s - F444010000
```
Sau đó, client hiển thị thông báo popup/toast: **"iD không chính xác"** và đứng im, không hiển thị form nhập tài khoản / mật khẩu.

---

### 1.2. Phân tích gói tin `c2s - F444010000`
- **Cấu trúc gói tin**:
  - Magic Header (2 bytes): `F4 44`
  - Length LE (2 bytes): `01 00` (Payload dài 1 byte)
  - Payload (1 byte): `00` (Main Opcode `0x00`)
- **Nguồn gốc trong mã Client**:
  Căn cứ theo [`.scratch/client-pseudo-op-code/reconnect_heartbeat_flow.md §4.1`](reconnect_heartbeat_flow.md#L338-L355) và assembly `005045a0_FUN_005045a0.asm.txt:115-120`:
  Ngay khi socket TCP mở kết nối thành công tới port `6414`, client thực thi:
  ```asm
  CALL 0x0046c6b0                  ; TAbstractSocket.Open
  MOV EAX,[0x007d9d30]
  XOR ECX,ECX
  XOR EDX,EDX                      ; param_2 = 0
  CALL 0x0077f414                  ; SendCommand(0) -> Gửi frame F4 44 01 00 00
  ```
- **Ý nghĩa**: Đây là frame **Liveness ACK** báo hiệu socket vừa kết nối và sẵn sàng nhận scene từ Server.
- Phía Server Rust đã xử lý frame này tại [`src/server/dispatcher.rs:340`](../../src/server/dispatcher.rs#L340) và [`src/server/handlers/system.rs:22-25`](../../src/server/handlers/system.rs#L22-L25) (`handle_system_alert` cập nhật `last_seen`).
- **Kết luận**: Gói `c2s - F444010000` hoàn toàn hợp lệ, đúng chuẩn giao thức, không phải nguyên nhân gây lỗi.

---

### 1.3. Nguyên nhân gốc rễ gây lỗi "iD không chính xác" (`s2c - F444030001095A`)

#### A. Cấu trúc gói tin Server gửi xuống:
Trong [`src/web/server_control.rs:665`](../../src/web/server_control.rs#L665) và [`src/server/spawn.rs:60`](../../src/server/spawn.rs#L60):
`LOGIN_SCENE_GREETING = "F444030001095A"`
- Main Opcode: `0x01` (`OP_AUTH`)
- Sub Opcode: `0x09` (Scene Switch)
- Tham số Cảnh (`sceneMode`): `0x5A` = 90 (decimal)

#### B. Phân tích mã nguồn Client xử lý `sceneMode`:
Khi client nhận `01 09 5A`, luồng xử lý đi qua:
`FUN_0078a89c` (MainOp 1) $\to$ `FUN_0078b149` (SubOp 9) $\to$ `FUN_0051aae0` ([`0051aae0_FUN_0051aae0.c:46-73`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/0051aae0_FUN_0051aae0.c#L46-L73)).

Tại `FUN_0051aae0`:
```c
  DAT_00926fc6 = *(byte *)(param_2 + 1); // sceneMode = 0x5A (90)
  (**(code **)(**(int **)gvar_007D9CC4 + 0x24))(); // Ẩn Server Select Form

  bVar4 = FUN_00504c9c(*(undefined4 *)gvar_007DA6EC, DAT_00926fc6);
  if (!bVar4) {
    bVar4 = FUN_00504c9c(*(undefined4 *)gvar_007DA6EC, gvar_007D7488);
    if (!bVar4) {
      (**(code **)(**(int **)gvar_007D9E7C + 0x20))(); // VMT +0x20: TFrmLogin.Show() - HIỂN THỊ FORM ĐĂNG NHẬP TRỐNG
      goto LAB_0051abcb; // Thoát ra để người dùng tự gõ phím
    }
  }

  // Nhánh khi bVar4 == TRUE: AUTO-LOGIN TỰ ĐỘNG
  gvar_007D748C = 0;
  gvar_007D7488 = 0;
  // Điền tài khoản đã lưu trong RAM (DAT_0092530c + 0x644) vào Form Login:
  FUN_007b372c(*(int **)(*(int *)gvar_007D9E7C + 0x13c), *(undefined4 **)(DAT_0092530c + 0x644));
  // Điền mật khẩu đã lưu trong RAM (DAT_0092530c + 0x648) vào Form Login:
  _LStrFromString((int *)&local_10, (byte *)(DAT_0092530c + 0x648));
  FUN_007b372c(*(int **)(*(int *)gvar_007D9E7C + 0x140), local_10);
  // TỰ ĐỘNG GỌI HÀM SUBMIT XÁC THỰC:
  FUN_007095c0(*(int **)gvar_007D9E7C);
```

#### C. Thẩm định điều kiện trong `FUN_00504c9c`:
Hàm `FUN_00504c9c` ([`00504c9c_FUN_00504c9c.c`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/00504c9c_FUN_00504c9c.c#L51-L66)):
```c
bool FUN_00504c9c(undefined4 param_1, byte param_2) {
  byte local_9 = param_2;
  if (100 < param_2) local_9 = (byte)((uint)param_2 % 100);
  return (byte)(local_9 + 0xa6) < 10; // Tương đương: local_9 in [90..99]
}
```
- Khi server gửi `sceneMode = 0x5A` (90):
  $$(90 + 166) = 256 \equiv 0 \pmod{256} \implies 0 < 10 \implies \mathbf{bVar4 = TRUE}$$
- Vì `bVar4 == TRUE`, client **BỎ QUA** nhánh `TFrmLogin.Show()` (`goto LAB_0051abcb`).
- Thay vào đó, client hiểu đây là trạng thái **Reconnect / Relog của một tài khoản đã từng đăng nhập**, nên tự động lấy chuỗi tài khoản và mật khẩu từ RAM `DAT_0092530c` điền vào form và gọi `FUN_007095c0`.

#### D. Tại sao lại báo lỗi "iD không chính xác"?
Vì người chơi vừa mới khởi động `aLogin.exe`, trong vùng nhớ `DAT_0092530c + 0x644` (Account) hoàn toàn **chưa có dữ liệu (chuỗi rỗng `""`)**.
Khi `FUN_007095c0` ([`007095c0_FUN_007095c0.c:77-142`](file:///mnt/d/VUDT/GIT_PCC/test/ts_decompile/functions/007095c0_FUN_007095c0.c#L77-L142)) được kích hoạt:
1. Nó kiểm tra tiền tố 2 ký tự đầu: Chuỗi rỗng $\to$ Sai tiền tố!
2. Nó kiểm tra độ dài: `_LStrLen < 3` $\to$ Vi phạm độ dài tối thiểu!
3. Nó kích hoạt hiển thị Toast chuỗi tĩnh tại địa chỉ `DAT_007098b8`:
   **Nội dung chuỗi `DAT_007098b8` chính là: `"iD không chính xác"`**!
4. Sau khi hiện toast, hàm dừng thực thi (`RET`), không gửi gói `SendCommand(1)`, khiến client đứng im tại màn hình chọn server.

---

### 1.4. Đề xuất giải pháp cho Vấn đề 1
- Khi Client vừa kết nối lần đầu (chưa đăng nhập), Server cần chỉ thị mở Form Đăng Nhập thủ công bằng cách gửi một `sceneMode` **nằm ngoài dải `90..99`** để hàm `FUN_00504c9c` trả về `FALSE`.
- Khi `FUN_00504c9c` trả về `FALSE`:
  Client nhảy vào nhánh:
  ```c
  (**(code **)(**(int **)gvar_007D9E7C + 0x20))(); // Show Form TFrmLogin
  goto LAB_0051abcb; // Kết thúc, chờ người dùng gõ
  ```
  Lúc này, Form Đăng Nhập của TS Online sẽ xuất hiện với 2 ô Account và Password trống, trỏ chuột nhấp nháy, người dùng tự gõ ID + Password rồi bấm nút "Đăng nhập".
- Giá trị chuẩn đã được định nghĩa ban đầu trong [`src/server/spawn.rs:44`](../../src/server/spawn.rs#L44):
  `pub const HELLO_REPLY: &str = "F4440300010901";` (`sceneMode = 0x01`).
  Với `sceneMode = 0x01`:
  $$(1 + 166) = 167 \ge 10 \implies \mathbf{bVar4 = FALSE}$$
- **Hành động khắc phục**:
  Đổi hằng số `LOGIN_SCENE_GREETING` từ `"F444030001095A"` sang `"F4440300010901"` trong `src/server/spawn.rs` (và cập nhật các test case liên quan trong `tests/server_network/packet_logging.rs`).

---

## 2. Vấn Đề 2: Tối Ưu Giao Diện Dashboard Tab "Account Manager"

Căn cứ vào `Screenshot 2026-11-17 154319.png` và mã nguồn trong [`templates/dashboard.html`](../../templates/dashboard.html), [`src/web/app.rs`](../../src/web/app.rs):

### 2.1. Khung pass1, pass2 thiếu label
#### Hiện trạng ([`templates/dashboard.html:306-312`](../../templates/dashboard.html#L306-L312)):
```html
<form hx-post="/api/accounts" hx-target="#accounts-table-body" hx-swap="afterbegin" onsubmit="setTimeout(() => this.reset(), 300)">
    <div style="display: flex; gap: 12px;">
        <input type="text" name="pass1" class="form-input" value="1111111111" maxlength="10" pattern="[!-~]{8,10}" title="8-10 ASCII characters, no spaces" required>
        <input type="text" name="pass2" class="form-input" value="1111111111" maxlength="10" pattern="[!-~]{8,10}" title="8-10 ASCII characters, no spaces" required>
        <button type="submit" class="btn btn-green">Create Account</button>
    </div>
</form>
```
- Không có thẻ `<label>` mô tả công năng của từng trường mật khẩu.
- Thiếu `placeholder` hướng dẫn người dùng.
- Nút bấm chưa căn chỉnh thẳng hàng dọc (vertical alignment) với input khi có label.

#### Đề xuất chỉnh sửa:
Sử dụng các class CSS có sẵn trong template (`.form-group`, `.form-label` tại dòng 104-105):
```html
<form hx-post="/api/accounts" hx-target="#accounts-table-body" hx-swap="afterbegin" onsubmit="setTimeout(() => this.reset(), 300)">
    <div style="display: flex; gap: 16px; align-items: flex-end;">
        <div class="form-group" style="flex: 1; margin-bottom: 0;">
            <label class="form-label" for="pass1">Mật khẩu 1 (Login Password)</label>
            <input type="text" id="pass1" name="pass1" class="form-input" value="1111111111" maxlength="10" pattern="[!-~]{8,10}" placeholder="8-10 ký tự ASCII" title="8-10 ASCII characters, no spaces" required>
        </div>
        <div class="form-group" style="flex: 1; margin-bottom: 0;">
            <label class="form-label" for="pass2">Mật khẩu 2 (Xóa nhân vật / Delete Char)</label>
            <input type="text" id="pass2" name="pass2" class="form-input" value="1111111111" maxlength="10" pattern="[!-~]{8,10}" placeholder="8-10 ký tự ASCII" title="8-10 ASCII characters, no spaces" required>
        </div>
        <button type="submit" class="btn btn-green" style="height: 42px;">Create Account</button>
    </div>
</form>
```

---

### 2.2. Cột GM Level chưa chuyển đổi giá trị 0 $\to$ no, 1 $\to$ yes

#### Hiện trạng:
1. **Trong HTML Template** ([`templates/dashboard.html:327`](../../templates/dashboard.html#L327)):
   ```html
   <td>{{ acc.gm_level }}</td>
   ```
   Đang hiển thị số nguyên thô `0` hoặc `1` (trong khi cột Suspended kế bên được format là `{% if acc.is_suspended %}yes{% else %}no{% endif %}`).
2. **Trong API HTMX Create Account** ([`src/web/app.rs:403`](../../src/web/app.rs#L403)):
   ```rust
   format!("<tr><td><strong>{}</strong></td><td>0</td><td>no</td></tr>", player_id)
   ```
   Đang hardcode `<td>0</td>` khi trả về HTML cho HTMX chèn vào đầu bảng.

#### Đề xuất chỉnh sửa:
1. **Sửa trong `templates/dashboard.html`**:
   ```html
   <td>{% if acc.gm_level > 0 %}yes{% else %}no{% endif %}</td>
   ```
   (Hoặc hiển thị tag có màu nổi bật: `{% if acc.gm_level > 0 %}<span style="color: var(--accent-green); font-weight: 600;">yes</span>{% else %}<span style="color: var(--text-muted);">no</span>{% endif %}`).
2. **Sửa trong `src/web/app.rs`**:
   ```rust
   format!("<tr><td><strong>{}</strong></td><td>no</td><td>no</td></tr>", player_id)
   ```
   Đảm bảo tính nhất quán tuyệt đối giữa dữ liệu nạp ban đầu và hàng mới tạo qua HTMX.

---

## 3. Tóm Tắt Các Bước Triển Khai Kỹ Thuật

| Hạng mục | File cần sửa | Chi tiết thay đổi |
| :--- | :--- | :--- |
| **Sửa Greeting Packet** | `src/server/spawn.rs` | Đổi `LOGIN_SCENE_GREETING` từ `"F444030001095A"` sang `"F4440300010901"` |
| **Cập nhật Test Network** | `tests/server_network/packet_logging.rs` | Đảm bảo test case kiểm tra đúng `LOGIN_SCENE_GREETING` mới |
| **Bổ sung Label Form Account** | `templates/dashboard.html` | Bọc `<label>` cho `pass1` và `pass2` với `.form-group`, `.form-label` |
| **Format GM Level Table** | `templates/dashboard.html` | Đổi `{{ acc.gm_level }}` thành `{% if acc.gm_level > 0 %}yes{% else %}no{% endif %}` |
| **Format GM Level HTMX API** | `src/web/app.rs` | Đổi `<td>0</td>` thành `<td>no</td>` trong phản hồi HTML của `create_account` |
