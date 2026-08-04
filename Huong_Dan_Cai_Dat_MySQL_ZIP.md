# Hướng Dẫn Chi Tiết Cài Đặt Và Khởi Chạy MySQL Community Server (Bản ZIP / Archive)

Tài liệu này hướng dẫn từng bước chi tiết quy trình tải, cấu hình, khởi tạo không mật khẩu (`--initialize-insecure`) và đăng nhập thiết lập tài khoản quản trị cho **MySQL Community Server 8.0.x** trên hệ điều hành Windows.

---

## 📋 Mục Lục
1. [Bước 1: Tải về phiên bản MySQL ZIP](#bước-1-tải-về-phiên-bản-mysql-zip)
2. [Bước 2: Giải nén và Tạo file cấu hình `my.ini`](#bước-2-giải-nén-và-tạo-file-cấu-hình-myini)
3. [Bước 3: Khởi tạo Cơ sở dữ liệu không mật khẩu (Insecure Mode)](#bước-3-khởi-tạo-cơ-sở-dữ-liệu-không-mật-khẩu-insecure-mode)
4. [Bước 4: Cài đặt và Khởi động Windows Service](#bước-4-cài-đặt-và-khởi-động-windows-service)
5. [Bước 5: Đăng nhập và Đặt mật khẩu mới cho tài khoản root](#bước-5-đăng-nhập-và-đặt-mật-khẩu-mới-cho-tài-khoản-root)
6. [Bước 6: Thêm MySQL vào Biến môi trường Path (Tùy chọn)](#bước-6-thêm-mysql-vào-biến-môi-trường-path-tùy-chọn)
7. [Tổng kết Thông tin Kết nối](#tổng-kết-thông-tin-kết-nối)

---

## Bước 1: Tải về phiên bản MySQL ZIP

1. Truy cập trang tải chính thức của MySQL: [https://dev.mysql.com/downloads/mysql/](https://dev.mysql.com/downloads/mysql/)
2. Chọn phiên bản: **MySQL Community Server 8.0.46** (hoặc phiên bản 8.0.x mới nhất).
3. Tại mục **Operating System**, chọn **Microsoft Windows**.
4. Chọn gói **Windows (x86, 64-bit), ZIP Archive** và nhấn **Download**.
5. Chọn *"No thanks, just start my download"* để bắt đầu tải về mà không cần đăng nhập tài khoản Oracle.

---

## Bước 2: Giải nén và Tạo file cấu hình `my.ini`

1. **Giải nén:** Giải nén file `.zip` vừa tải về vào thư mục mong muốn trên máy tính.
   * *Ví dụ:* `C:\mysql-8.0.46`
2. **Tạo file cấu hình:** Tạo một file văn bản tên là `my.ini` đặt ngay trong thư mục gốc `C:\mysql-8.0.46\`.
3. **Nội dung file `my.ini`:**
   ```ini
   [mysqld]
   # Thư mục gốc chứa MySQL
   basedir=C:/mysql-8.0.46

   # Thư mục chứa dữ liệu (MySQL sẽ tự động tạo thư mục này khi khởi tạo)
   datadir=C:/mysql-8.0.46/data

   # Cấu hình Cổng kết nối (Port)
   port=3306

   # Cấu hình Bảng mã mặc định
   character-set-server=utf8mb4

   [client]
   default-character-set=utf8mb4
   ```

> **Lưu ý:**
> * Thay thế `C:/mysql-8.0.46` bằng đường dẫn thực tế trên máy tính của bạn.
> * Sử dụng dấu gạch chéo `/` hoặc hai dấu gạch chéo ngược `\\` trong đường dẫn file cấu hình `.ini`.

---

## Bước 3: Khởi tạo Cơ sở dữ liệu không mật khẩu (Insecure Mode)

Vì file `my.ini` chỉ quản lý thông số cấu hình hệ thống chứ không chứa thông tin đăng nhập, chúng ta sẽ khởi tạo thư mục dữ liệu ban đầu với tùy chọn `--initialize-insecure` để thiết lập tài khoản `root` với **mật khẩu trống**.

1. Mở **Command Prompt (CMD)** dưới quyền Quản trị viên (**Run as Administrator**).
2. Di chuyển vào thư mục `bin` của MySQL:
   ```cmd
   cd C:\mysql-8.0.46\bin
   ```
3. Chạy lệnh khởi tạo không mật khẩu:
   ```cmd
   mysqld --initialize-insecure --console
   ```
4. Chờ vài giây cho đến khi quá trình hoàn tất. Lúc này thư mục `C:\mysql-8.0.46\data` sẽ tự động được khởi tạo thành công.

---

## Bước 4: Cài đặt và Khởi động Windows Service

Vẫn tại màn hình **CMD Administrator**:

1. **Cài đặt MySQL dưới dạng Windows Service:**
   ```cmd
   mysqld --install MySQL80
   ```
   *(Thông báo `Service successfully installed.` xuất hiện là thành công).*

2. **Khởi động Service:**
   ```cmd
   net start MySQL80
   ```

---

## Bước 5: Đăng nhập và Đặt mật khẩu mới cho tài khoản root

1. **Đăng nhập vào MySQL:** 
   Do đã khởi tạo bằng chế độ `--initialize-insecure`, bạn có thể kết nối ngay mà không cần mật khẩu (nhấn Enter khi được hỏi password):
   ```cmd
   mysql -u root -P 3306 -p
   ```

2. **Đặt mật khẩu mới cho tài khoản `root`:**
   Tại giao diện dòng lệnh của MySQL (`mysql>`), thực hiện các câu lệnh SQL sau (thay `MatKhauMoi123!` bằng mật khẩu mong muốn của bạn):
   ```sql
   ALTER USER 'root'@'localhost' IDENTIFIED BY 'MatKhauMoi123!';
   FLUSH PRIVILEGES;
   ```

3. **Thoát khỏi MySQL:**
   ```sql
   EXIT;
   ```

---

## Bước 6: Thêm MySQL vào Biến môi trường Path (Tùy chọn)

Đoạn cấu hình này giúp bạn có thể gọi lệnh `mysql` hoặc `mysqld` từ bất kỳ đường dẫn nào trên CMD mà không cần chuyển thư mục về `C:\mysql-8.0.46\bin`.

1. Nhấn tổ hợp phím `Windows + R`, gõ `sysdm.cpl` và nhấn **Enter**.
2. Chuyển sang thẻ **Advanced** $\rightarrow$ Chọn **Environment Variables...**
3. Tại mục **System variables**, tìm dòng **Path** $\rightarrow$ Nhấn **Edit...**
4. Nhấn **New** và dán đường dẫn thư mục bin vào: `C:\mysql-8.0.46\bin`
5. Nhấn **OK** ở các cửa sổ để hoàn tất.

---

## 📌 Tổng kết Thông tin Kết nối

| Thông số | Giá trị |
| :--- | :--- |
| **Host / Server** | `localhost` hoặc `127.0.0.1` |
| **Port** | `3306` |
| **Username** | `root` |
| **Password** | Mật khẩu mới bạn vừa đặt ở **Bước 5** |
| **File cấu hình** | `C:\mysql-8.0.46\my.ini` |
| **Thư mục Data** | `C:\mysql-8.0.46\data` |
