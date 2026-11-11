### 1. Giải đáp: Rust có tải dependencies về local repo không?

**Có.** Tương tự như Maven dùng `.m2/repository` (hay `D:\maven_repo` trên máy bạn), trình quản lý gói **Cargo** của Rust tải và lưu tập trung toàn bộ dependencies về một thư mục cục bộ được điều khiển bởi biến môi trường `CARGO_HOME` (đã được trỏ về **`D:\rust_repo`**):

- **`D:\rust_repo\registry\index\`**: Lưu chỉ mục (metadata/version index) từ `crates.io`.
- **`D:\rust_repo\registry\cache\`**: Lưu các gói nén `.crate` đã tải về (tương tự file `.jar` trong Maven). Hiện đã tải sẵn **258 gói** từ [`Cargo.toml`](file:///D:/VUDT/GIT_PCC/ts_dream/Cargo.toml) (`tokio`, `axum`, `sqlx`, `askama`, `serde`, v.v.).
- **`D:\rust_repo\registry\src\`**: Lưu mã nguồn đã giải nén của từng crate để `rustc` biên dịch và dùng chung cho mọi project trên máy mà không cần tải lại.
- **`D:\rust_repo\bin\`**: Chứa các file thực thi của bộ công cụ Cargo/Rust (`cargo.exe`, `rustc.exe`, `rustup.exe`, `rustfmt.exe`, `cargo-clippy.exe`) và các công cụ cài thêm qua `cargo install`.

---

### 2. Thông tin các thành phần đã cài đặt

| Thành phần | Đường dẫn | Phiên bản / Chi tiết |
| :--- | :--- | :--- |
| **Rust SDK (`RUSTUP_HOME`)** | `D:\VUDT\rustup` | Toolchain `stable-x86_64-pc-windows-gnu` (`rustc 1.99.1`, `cargo 1.99.1`, `rustfmt`, `clippy`) |
| **Local Crate Repo (`CARGO_HOME`)** | `D:\rust_repo` | Đã `cargo fetch` toàn bộ **258 crates** của dự án `ts_dream` |
| **C/C++ Compiler & Linker** | `D:\VUDT\w64devkit` | `gcc 16.2.0` (MinGW-w64) — dùng để liên kết binary trên Windows và biên dịch các thư viện C nhúng trong [`Cargo.toml`](file:///D:/VUDT/GIT_PCC/ts_dream/Cargo.toml) như `libsqlite3-sys` (`sqlx`) và `ring` (`rustls`) |

---

### 3. Biến môi trường User đã được cấu hình tự động

Tôi đã thiết lập sẵn vào **User Environment Variables** trên Windows:
- `RUSTUP_HOME` = `D:\VUDT\rustup`
- `CARGO_HOME` = `D:\rust_repo`
- `Path` đã được bổ sung `D:\rust_repo\bin` và `D:\VUDT\w64devkit\bin`

> **Lưu ý:** Bạn chỉ cần mở một cửa sổ Terminal / PowerShell mới (hoặc khởi động lại IDE) là có thể chạy trực tiếp các lệnh `cargo build`, `cargo test`, `cargo run`, `rustup` ở bất kỳ thư mục nào.