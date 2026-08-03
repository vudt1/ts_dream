Việc sử dụng Rust để xây dựng một private server cho TS Dream tích hợp cùng web dashboard quản lý là một lựa chọn xuất sắc. Cấu trúc này không chỉ đảm bảo hiệu năng cao, an toàn bộ nhớ mà còn giải quyết hoàn hảo bài toán cross-platform (Windows, Linux, MacOS) chỉ với một file chạy (single binary) duy nhất.

Dưới đây là đề xuất chi tiết về tech stack, kiến trúc tổng thể và cấu trúc thư mục cho server của bạn.

### 1. Đề xuất Tech Stack (Công nghệ sử dụng)

*   **Core Runtime:** **Tokio** (Async runtime). Hệ thống của bạn cần xử lý hàng ngàn kết nối TCP đồng thời cùng với các request HTTP, Tokio là nền tảng tiêu chuẩn và mạnh mẽ nhất trong Rust cho việc này.
*   **Giao thức Game (Network):** Sử dụng `tokio::net::TcpListener`. Dữ liệu cho thấy server TS Dream cũ chỉ sử dụng duy nhất giao thức TCP qua cổng cố định `6414` (không dùng UDP).
*   **Web Framework (Quản lý):** **Axum**. Axum là lựa chọn tối ưu nhất so với Actix-web trong trường hợp này vì nó chia sẻ cùng một Tokio runtime với TCP server, giúp việc chia sẻ state (trạng thái người chơi, thông số server) và quản lý tắt/mở (graceful shutdown) trở nên hoàn toàn tự nhiên.
*   **Frontend Web Admin:** **Askama + HTMX**. Theo nguồn dữ liệu, kết hợp Askama (template engine biên dịch thẳng vào mã máy) và HTMX (xử lý tương tác giao diện không cần viết nhiều JS) là "điểm ngọt" cho admin dashboard. Kiến trúc này giúp bạn không cần cài đặt Node.js hay tách rời frontend/backend, toàn bộ UI sẽ được nhúng vào một file binary duy nhất để dễ dàng deploy lên máy chủ Linux headless.
*   **Xử lý Gói tin (Packet):** Các thao tác bit/byte thuần túy của Rust. Giao thức TS yêu cầu giải mã XOR với key `0xAD` và phân tách frame dựa trên header `F4 44` cùng 2 byte chiều dài.

* **Tech Stack bổ sung (Cargo.toml):** Bạn sẽ cần thêm sqlx cho SQLite và bytes để xử lý mảng byte của giao thức:

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
axum = "0.7"
askama = "0.12"
bytes = "1.5"        # Xử lý Little-Endian, bóc tách frame mạng cực nhanh
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }
```

### 2. Kiến trúc tổng thể (Architecture)

Kiến trúc của bạn sẽ xoay quanh một **Shared State (Trạng thái chia sẻ)** được bọc trong `Arc<RwLock<AppState>>` hoặc `Arc<Mutex<AppState>>`. State này sẽ được truyền vào cả TCP Server (để cập nhật khi người chơi đăng nhập/di chuyển) và Web Server (để đọc và hiển thị lên Dashboard).

```
[Game Clients] 
      │ (TCP Port 6414, XOR 0xAD encrypted)
      ▼
 ┌──────────────────────┐
 │   TCP Server Module  │ ◄─── (Xử lý Packet F4 44, Opcode 0x00 -> 0x4F)
 └──────────┬───────────┘
            │ Cập nhật & Đọc
            ▼
 ┌──────────────────────┐
 │  Shared AppState     │ ◄─── (Danh sách user online, configs, metrics)
 └──────────▲───────────┘
            │ Đọc & Điều khiển
 ┌──────────┴───────────┐
 │   Web Server (Axum)  │ ◄─── (HTTP API & HTML Templates)
 └──────────▲───────────┘
            │ (HTTP Port 8090/3000)
            ▼
   [Web Admin Dashboard] (HTMX + Askama)
```

### 3. Cấu trúc Project (Project Structure)

Dựa trên hướng dẫn phân chia module, bạn có thể tổ chức thư mục mã nguồn như sau:

```text
ts_dream/
├── Cargo.toml                  # Khai báo thư viện (tokio, axum, askama, bytes...)
├── templates/                  # Chứa file HTML cho Web Dashboard (Askama)
│   ├── base.html
│   └── dashboard.html
└── src/
    ├── main.rs                 # Entry point: Khởi tạo AppState, chạy song song TCP và Web, Chạy Tokio, Axum (Web Admin), kết nối SQLite
    ├── db.rs                   # Cấu hình sqlx kết nối file SQLite (vd: ts_data.db)
    ├── config.rs               # Xử lý file cấu hình (port, db URL...)
    ├── state.rs                # Chứa `AppState` chia sẻ dữ liệu giữa Web và TCP hay AppState (Arc<RwLock<...>>) chứa danh sách online
    ├── network/                
    │   ├── tcp_server.rs       # Lắng nghe TCP cổng 6414, quản lý client connections
    │   ├── packet.rs           # Cấu trúc packet (Header F4 44, Length, Opcode, Sub, Data)
    │   └── crypto.rs           # Logic mã hóa/giải mã XOR 0xAD
    ├── game/                   
    │   ├── server.rs           # TCP Listener cổng 6414
    │   ├── crypto.rs           # Hàm giải mã/mã hóa XOR 0xAD
    │   ├── packet.rs           # Struct bóc tách F4 44, Length, Opcode, Sub
    │   ├── dispatcher.rs       # Nơi chứa lệnh `match` phân luồng 70+ Opcodes
    │   └── handlers/           # Thư mục chứa logic chi tiết từng Opcode
    │       ├── login.rs        # Opcode 0x00, 0x01 (Có check version >= 186)
    │       ├── chat.rs         # Opcode 0x02
    │       ├── move.rs         # Opcode 0x05, 0x06
    │       └── ...             # Thêm dần các opcode khác sau
    │       
    └── web/                    
        ├── routes.rs           # Định tuyến API và HTML trả về từ Axum
        ├── handlers.rs         # Logic xử lý HTTP request (start/stop server, view online)
        └── sse.rs              # (Tùy chọn) Server-Sent Events để stream log packet realtime
```

### 4. Điểm mấu chốt khi triển khai với Rust

*   **Xử lý TCP Luồng đăng nhập:** Bạn cần cài đặt chính xác luồng bắt tay ban đầu. Khi client kết nối, nó gửi `F444010000` (Opcode 0x00), server phải đáp trả `F4440300010901` trước khi nhận thông tin tài khoản (Opcode 0x01).
*   **Byte Manipulation:** Vì packet của TS dùng Little-Endian cho chiều dài (length) và một số trường dữ liệu, Rust cung cấp các thư viện như `bytes` (`BytesMut`, `Buf`, `BufMut`) cực kỳ hoàn hảo để thay thế các hàm `Substring` hoặc `smethod` thủ công của C# cũ.
*   **Chia sẻ dữ liệu:** TCP Handler của mỗi client chạy trên một Tokio Task độc lập (Thread riêng biệt). Bạn có thể dùng `tokio::sync::broadcast` để phát luồng thông tin (ví dụ: Broadcast toàn server khi có người chat Opcode 0x02) hoặc phát log thẳng lên giao diện Web Admin bằng luồng sự kiện SSE.

### 5. Một số thông tin bổ sung
**5.1. Kiến trúc xử lý đồng thời (Concurrency)**
Để xử lý nhiều người chơi cùng lúc mà không bị nghẽn, chúng ta sẽ áp dụng mô hình 1 Client = 1 Tokio Task.
+ **Tokio Task không phải là Thread của HĐH**: Nó là "green thread" rất nhẹ. Bạn có thể mở 10,000 kết nối TCP đồng thời mà chỉ tốn vài chục MB RAM.
+ Mỗi khi có một người chơi kết nối vào cổng 6414, server sẽ tokio::spawn một luồng riêng biệt để phục vụ client đó. Vòng lặp trong luồng này sẽ liên tục chờ nhận dữ liệu, giải mã XOR 0xAD, đọc Header F4 44, và bóc tách Opcode

**5.2. Xử lý các Opcodes hiệu quả (Dispatcher)**
Để tái tạo hiệu suất này trong Rust, bạn không cần viết các cấu trúc Hash Map phức tạp. Hãy sử dụng lệnh match của Rust. Khi bạn match một số nguyên (Opcode) với hàng chục case liên tiếp, trình biên dịch LLVM của Rust sẽ tự động tối ưu hóa nó thành một Jump Table (bảng nhảy mã máy) giống hệt như cách C++ hay ASM của server gốc hoạt động.

Mô phỏng Dispatcher xử lý 70+ Opcodes:

```
// Trong src/game/dispatcher.rs

use crate::game::handlers::*;

pub async fn dispatch_packet(opcode: u8, sub: u8, data: &[u8], state: &AppState) {
    // Trình biên dịch Rust sẽ biến `match` này thành Jump Table O(1)
    match opcode {
        0x00 => login::handle_hello().await, // Opcode System/Login [6]
        0x01 => login::handle_auth(data, state).await, // Opcode Auth [6, 7]
        0x02 => chat::handle_chat(sub, data, state).await, // Opcode Chat [8]
        0x03 => look::handle_look_request(sub, data).await, // Opcode Look (NTS thêm) [5, 9]
        // ... Tạm thời bỏ qua các opcode chưa làm, in ra log để debug
        _ => {
            println!("[Dispatcher] Unhandled Opcode: 0x{:02X}, Sub: 0x{:02X}", opcode, sub);
        }
    }
}
```

