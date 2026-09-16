# Bản hiệu đính VISCII → UTF-8 cho báo cáo call graph S → C

> **Mục tiêu hiệu đính.** Bản này đối chiếu literal Delphi trực tiếp từ `aLogin.exe` theo địa chỉ VA và độ dài prefix, giải mã bằng VISCII → UTF-8, loại bỏ kết quả giả do quét nhầm hằng điều khiển, rồi hiệu đính tiếng Việt theo nghĩa sát nhất có thể. Cột **Literal gốc** giữ chứng cứ; cột **Bản hiệu đính** là bản đọc tự nhiên. Khi nghĩa game-specific chưa đủ ngữ cảnh, bản hiệu đính chỉ sửa chính tả/thuật ngữ và không suy diễn. [1] [2]

## 1. Các thay đổi chính

| Vấn đề cũ | Cách sửa |
|---|---|
| Literal bị dính hoặc có khoảng trắng thừa | Chuẩn hóa whitespace từ byte VISCII gốc. |
| Chuỗi rác do hằng code/jump table bị xem là text | Chỉ nhận `mov reg, imm32`/`push imm32` truyền literal trực tiếp, sau đó kiểm tra Pascal length prefix. |
| `Server`, `version`, `files` lẫn tiếng Việt | Chuẩn hóa thành **máy chủ**, **phiên bản**, **tệp** khi ngữ cảnh rõ ràng. |
| Dịch máy như “bạn chơi”, “quang cảnh”, “Tạo Vật” | Chuyển sang **người chơi**, **khung cảnh**, **dữ liệu nhân vật** khi không làm đổi ý nghĩa code. |
| Cụm mơ hồ thuộc domain game | Giữ gần literal và ghi chú cần ngữ cảnh helper/packet để khẳng định. |

## 2. Opcode `0x00` — System Alert: status đã hiệu đính

Handler thực tế `0x0078AABE` nhận status `body[0]` trong `0..56`. Các dòng sau liên kết status với basic block và literal gốc; `0x0078AF97` là đường default chung cho status 0 và 39. [1]

| Status | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Nhóm ý nghĩa | Cách xử lý |
|---:|---:|---|---|---|---|
| `0` | `0x0078AF97` | Mất kết nối với Server | Mất kết nối với máy chủ. | Ngắt kết nối | Hiệu đính ngữ nghĩa/câu chữ |
| `1` | `0x0078ABD1` | Dữ liệu quá nhiều bị mất kết nối | Dữ liệu quá lớn; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `2` | `0x0078ABE3` | Trả lời sai 3 lần sẽ bị mất kết nối | Trả lời sai 3 lần; kết nối sẽ bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `3` | `0x0078ABF5` | Đăng nhập sai 3 lần | Đăng nhập sai 3 lần. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `4` | `0x0078AC07` | vì Server gặp trục trặc | Do máy chủ gặp sự cố. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `5` | `0x0078AC19` | Phát hiện sự kiện phạm luật bị mất kết nối 1 | Phát hiện hành vi vi phạm (mã 1); kết nối đã bị ngắt. | Bảo vệ/anti-cheat | Hiệu đính ngữ nghĩa/câu chữ |
| `6` | `0x0078AC2B` | Phát hiện sự kiện phạm luật bị mất kết nối 2 | Phát hiện hành vi vi phạm (mã 2); kết nối đã bị ngắt. | Bảo vệ/anti-cheat | Hiệu đính ngữ nghĩa/câu chữ |
| `7` | `0x0078AC3D` | Tra không ra sự kiện phải làm, mất kết nối | Không tìm thấy sự kiện cần thực hiện; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `8` | `0x0078AC4F` | Không kết nối được, mất kết nối | Không thể thiết lập kết nối; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `9` | `0x0078AC61` | Không kết nối được, mất kết nối | Không thể thiết lập kết nối; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `10` | `0x0078AC73` | mất kết nối do version không phù hợp | Kết nối bị ngắt do phiên bản không tương thích. | Tương thích phiên bản | Hiệu đính ngữ nghĩa/câu chữ |
| `11` | `0x0078AC85` | Kết nối gặp trục trặc, mất kết nối | Kết nối gặp sự cố; đã ngắt kết nối. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `12` | `0x0078AC97` | Sử dụng chương trình bất hợp pháp | Phát hiện sử dụng chương trình không hợp lệ. | Từ chối/lỗi nghiệp vụ | Hiệu đính ngữ nghĩa/câu chữ |
| `13` | `0x0078ACA9` | Mất kết nối | Đã mất kết nối. | Ngắt kết nối | Hiệu đính ngữ nghĩa/câu chữ |
| `14` | `0x0078ACBB` | Mất kết nối do sử dụng thêm chương trình khác | Kết nối bị ngắt do phát hiện chương trình bên thứ ba. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `15` | `0x0078ACCD` | Loại bỏ thành công, xin khởi động lại máy | Đã gỡ bỏ thành công. Vui lòng khởi động lại máy. | Kết quả thành công | Hiệu đính ngữ nghĩa/câu chữ |
| `16` | `0x0078ACDF` | mất kết nối do đăng nhập IP bất hợp pháp | Kết nối bị ngắt do địa chỉ IP đăng nhập không hợp lệ. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `17` | `0x0078ACF1` | Version không phù hợp, xin cập nhật version mới | Phiên bản không tương thích. Vui lòng cập nhật phiên bản mới. | Tương thích phiên bản | Hiệu đính ngữ nghĩa/câu chữ |
| `18` | `0x0078AD03` | Dữ liệu thay đổi,mất kết nối | Dữ liệu đã thay đổi; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `19` | `0x0078AD15` | Mất kết nối do có sự đăng nhập khác | Kết nối bị ngắt do tài khoản đã đăng nhập ở nơi khác. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `20` | `0x0078AD27` | Hệ thống gặp trục trặc bất thường | Hệ thống gặp sự cố bất thường. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `21` | `0x0078AD39` | Gặp trục trặc với việc lưu trữ, mất kết nối | Lỗi lưu trữ dữ liệu; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `22` | `0x0078AD4B` | Mất kết nối do dạng dữ liệu không phù hợp | Kết nối bị ngắt do định dạng dữ liệu không hợp lệ. | Từ chối/lỗi nghiệp vụ | Hiệu đính ngữ nghĩa/câu chữ |
| `23` | `0x0078AD5D` | Đổi tên, mất kết nối | Lỗi đổi tên; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `24` | `0x0078AD6F` | Mật khẩu quá ngắn, mất kết nối | Mật khẩu quá ngắn; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `25` | `0x0078AD81` | Trùng lập tên, mất kết nối | Tên đã tồn tại; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `26` | `0x0078AD93` | Sự kiện phạm luật, mất kết nối | Phát hiện sự kiện vi phạm; kết nối đã bị ngắt. | Bảo vệ/anti-cheat | Hiệu đính ngữ nghĩa/câu chữ |
| `27` | `0x0078ADA5` | Mất kết nối do đăng nhập sai | Kết nối bị ngắt do thông tin đăng nhập không hợp lệ. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `28` | `0x0078ADB7` | Phòng vệ mất kết nối | Kết nối bị ngắt bởi cơ chế bảo vệ. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `29` | `0x0078ADC9` | Dữ liệu quá nhiều | Dữ liệu quá lớn. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `30` | `0x0078ADDB` | Khóa tài khoản, mất kết nối | Tài khoản đã bị khóa; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `31` | `0x0078ADED` | Không thể sử dụng ID này | Không thể sử dụng ID này. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `32` | `0x0078ADFF` | Cảnh chiến đấu bị lỗi | Khung cảnh chiến đấu xảy ra lỗi. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `33` | `0x0078AE11` | Ký hiệu và quang cảnh không phù hợp, mất kết nối | Ký hiệu và khung cảnh không tương thích; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `34` | `0x0078AE23` | Đăng nhập lại qua Server | Đăng nhập lại thông qua máy chủ. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `35` | `0x0078AE35` | Hiệp định đăng nhập | Quy ước/phiên đăng nhập. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `36` | `0x0078AE47` | Phạm vi - ID không phù hợp | ID nằm ngoài phạm vi hợp lệ. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `37` | `0x0078AE59` | Rớt mạng do không cùng quang cảnh | Mất kết nối do không cùng khung cảnh. | Ngắt kết nối | Hiệu đính ngữ nghĩa/câu chữ |
| `38` | `0x0078AE6B` | Mục đích quang cảnh không phù hợp nên rớt mạng | Mục tiêu khung cảnh không hợp lệ; kết nối đã bị ngắt. | Từ chối/lỗi nghiệp vụ | Hiệu đính ngữ nghĩa/câu chữ |
| `39` | `0x0078AF97` | Mất kết nối với Server | Mất kết nối với máy chủ. | Ngắt kết nối | Hiệu đính ngữ nghĩa/câu chữ |
| `40` | `0x0078AE7D` | Sửa đổi files Tạo Vật bị rớt mạng | Sửa đổi tệp dữ liệu nhân vật; kết nối đã bị ngắt. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `41` | `0x0078AE8F` | Lưu lại sau khi đăng nhập | Lưu lại sau khi đăng nhập. | Xác thực/phiên | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `42` | `0x0078AEA1` | Sửa đổi tư liệu chiến đấu | Dữ liệu chiến đấu đã bị chỉnh sửa. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `43` | `0x0078AEB3` | Độ sai lệch hình thái chiến đấu của bạn chơi 0 | Dữ liệu trạng thái chiến đấu của người chơi không khớp. | Từ chối/lỗi nghiệp vụ | Hiệu đính ngữ nghĩa/câu chữ |
| `44` | `0x0078AEC5` | Sự kiện và quang cảnh xẩy ra không phù hợp | Sự kiện và khung cảnh không tương thích. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `45` | `0x0078AED7` | Tài khoản sử dụng của bạn đã bị tạm khóa do phạm luật | Tài khoản của bạn đã tạm khóa do vi phạm quy định. | Bảo vệ/anti-cheat | Hiệu đính ngữ nghĩa/câu chữ |
| `46` | `0x0078AEE9` | Gian xảo trong vấn đáp của Bắc Đẩu Quân | Phát hiện gian lận trong phần vấn đáp của Bắc Đẩu Quân. | Bảo vệ/anti-cheat | Hiệu đính ngữ nghĩa/câu chữ |
| `47` | `0x0078AEFB` | Sự kiện kết thúc trước khi chiến đấu kết thúc | Sự kiện kết thúc trước khi trận chiến hoàn tất. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `48` | `0x0078AF0D` | Phi pháp sử dụng kỹ năng Kêu Gọi | Sử dụng trái phép kỹ năng Triệu Hồi. | Bảo vệ/anti-cheat | Hiệu đính ngữ nghĩa/câu chữ |
| `49` | `0x0078AF1F` | Cấm vận gia nhập đối với bạn chưa đủ 18 | Người chơi chưa đủ 18 tuổi không được phép tham gia. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `50` | `0x0078AF2E` | Đăng nhập thi đấu chuyên thuộc Server | Đăng nhập máy chủ thi đấu chuyên dụng. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `51` | `0x0078AF3D` | Không thể đăng nhập thi đấu chuyên thuộc Server | Không thể đăng nhập máy chủ thi đấu chuyên dụng. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `52` | `0x0078AF4C` | Không thể đăng nhập thi đấu chuyên thuộc Server | Không thể đăng nhập máy chủ thi đấu chuyên dụng. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `53` | `0x0078AF5B` | Chưa đăng nhập vào Server | Chưa đăng nhập vào máy chủ. | Xác thực/phiên | Hiệu đính ngữ nghĩa/câu chữ |
| `54` | `0x0078AF6A` | Hoạt động lôi đài đấu trận kết thúc | Sự kiện đấu trường đã kết thúc. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |
| `55` | `0x0078AF79` | Mục đích trong lưu trữ tư liệu di dân server | Mục tiêu lưu dữ liệu chuyển máy chủ không hợp lệ. | Từ chối/lỗi nghiệp vụ | Hiệu đính ngữ nghĩa/câu chữ |
| `56` | `0x0078AF88` | Server đang bận xin chờ tý xíu. | Máy chủ đang bận. Vui lòng chờ một lát. | Thông báo UI/trạng thái | Hiệu đính ngữ nghĩa/câu chữ |

## 3. Literal ngoài Login đã hiệu đính theo handler/subopcode

Các hàng dưới chỉ gồm chuỗi có **tham chiếu literal trực tiếp trong basic block**. `Sub` là `body[0]` đối với subdispatcher; `—` là parser/branch trực tiếp. [1]

| Opcode | Handler | Sub | Basic block | Literal VISCII gốc | Bản tiếng Việt hiệu đính | Ghi chú dịch |
|---|---:|---:|---:|---|---|---|
| `0x00` | `0x0078AABE` | `0` | `0x0078AF97` | Mất kết nối | Đã mất kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `0` | `0x0078AF97` | Mất kết nối với Server | Mất kết nối với máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `1` | `0x0078ABD1` | Dữ liệu quá nhiều bị mất kết nối | Dữ liệu quá lớn; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `2` | `0x0078ABE3` | Trả lời sai 3 lần sẽ bị mất kết nối | Trả lời sai 3 lần; kết nối sẽ bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `3` | `0x0078ABF5` | Đăng nhập sai 3 lần | Đăng nhập sai 3 lần. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `4` | `0x0078AC07` | vì Server gặp trục trặc | Do máy chủ gặp sự cố. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `5` | `0x0078AC19` | Phát hiện sự kiện phạm luật bị mất kết nối 1 | Phát hiện hành vi vi phạm (mã 1); kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `6` | `0x0078AC2B` | Phát hiện sự kiện phạm luật bị mất kết nối 2 | Phát hiện hành vi vi phạm (mã 2); kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `7` | `0x0078AC3D` | Tra không ra sự kiện phải làm, mất kết nối | Không tìm thấy sự kiện cần thực hiện; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `8` | `0x0078AC4F` | Không kết nối được, mất kết nối | Không thể thiết lập kết nối; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `9` | `0x0078AC61` | Không kết nối được, mất kết nối | Không thể thiết lập kết nối; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `10` | `0x0078AC73` | mất kết nối do version không phù hợp | Kết nối bị ngắt do phiên bản không tương thích. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `11` | `0x0078AC85` | Kết nối gặp trục trặc, mất kết nối | Kết nối gặp sự cố; đã ngắt kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `12` | `0x0078AC97` | Sử dụng chương trình bất hợp pháp | Phát hiện sử dụng chương trình không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `13` | `0x0078ACA9` | Mất kết nối | Đã mất kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `14` | `0x0078ACBB` | Mất kết nối do sử dụng thêm chương trình khác | Kết nối bị ngắt do phát hiện chương trình bên thứ ba. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `15` | `0x0078ACCD` | Loại bỏ thành công, xin khởi động lại máy | Đã gỡ bỏ thành công. Vui lòng khởi động lại máy. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `16` | `0x0078ACDF` | mất kết nối do đăng nhập IP bất hợp pháp | Kết nối bị ngắt do địa chỉ IP đăng nhập không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `17` | `0x0078ACF1` | Version không phù hợp, xin cập nhật version mới | Phiên bản không tương thích. Vui lòng cập nhật phiên bản mới. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `18` | `0x0078AD03` | Dữ liệu thay đổi,mất kết nối | Dữ liệu đã thay đổi; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `19` | `0x0078AD15` | Mất kết nối do có sự đăng nhập khác | Kết nối bị ngắt do tài khoản đã đăng nhập ở nơi khác. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `20` | `0x0078AD27` | Hệ thống gặp trục trặc bất thường | Hệ thống gặp sự cố bất thường. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `21` | `0x0078AD39` | Gặp trục trặc với việc lưu trữ, mất kết nối | Lỗi lưu trữ dữ liệu; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `22` | `0x0078AD4B` | Mất kết nối do dạng dữ liệu không phù hợp | Kết nối bị ngắt do định dạng dữ liệu không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `23` | `0x0078AD5D` | Đổi tên, mất kết nối | Lỗi đổi tên; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `24` | `0x0078AD6F` | Mật khẩu quá ngắn, mất kết nối | Mật khẩu quá ngắn; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `25` | `0x0078AD81` | Trùng lập tên, mất kết nối | Tên đã tồn tại; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `26` | `0x0078AD93` | Sự kiện phạm luật, mất kết nối | Phát hiện sự kiện vi phạm; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `27` | `0x0078ADA5` | Mất kết nối do đăng nhập sai | Kết nối bị ngắt do thông tin đăng nhập không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `28` | `0x0078ADB7` | Phòng vệ mất kết nối | Kết nối bị ngắt bởi cơ chế bảo vệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `29` | `0x0078ADC9` | Dữ liệu quá nhiều | Dữ liệu quá lớn. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `30` | `0x0078ADDB` | Khóa tài khoản, mất kết nối | Tài khoản đã bị khóa; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `31` | `0x0078ADED` | Không thể sử dụng ID này | Không thể sử dụng ID này. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `32` | `0x0078ADFF` | Cảnh chiến đấu bị lỗi | Khung cảnh chiến đấu xảy ra lỗi. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `33` | `0x0078AE11` | Ký hiệu và quang cảnh không phù hợp, mất kết nối | Ký hiệu và khung cảnh không tương thích; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `34` | `0x0078AE23` | Đăng nhập lại qua Server | Đăng nhập lại thông qua máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `35` | `0x0078AE35` | Hiệp định đăng nhập | Quy ước/phiên đăng nhập. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `36` | `0x0078AE47` | Phạm vi - ID không phù hợp | ID nằm ngoài phạm vi hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `37` | `0x0078AE59` | Rớt mạng do không cùng quang cảnh | Mất kết nối do không cùng khung cảnh. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `38` | `0x0078AE6B` | Mục đích quang cảnh không phù hợp nên rớt mạng | Mục tiêu khung cảnh không hợp lệ; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `39` | `0x0078AF97` | Mất kết nối | Đã mất kết nối. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `39` | `0x0078AF97` | Mất kết nối với Server | Mất kết nối với máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `40` | `0x0078AE7D` | Sửa đổi files Tạo Vật bị rớt mạng | Sửa đổi tệp dữ liệu nhân vật; kết nối đã bị ngắt. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `41` | `0x0078AE8F` | Lưu lại sau khi đăng nhập | Lưu lại sau khi đăng nhập. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x00` | `0x0078AABE` | `42` | `0x0078AEA1` | Sửa đổi tư liệu chiến đấu | Dữ liệu chiến đấu đã bị chỉnh sửa. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `43` | `0x0078AEB3` | Độ sai lệch hình thái chiến đấu của bạn chơi 0 | Dữ liệu trạng thái chiến đấu của người chơi không khớp. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `44` | `0x0078AEC5` | Sự kiện và quang cảnh xẩy ra không phù hợp | Sự kiện và khung cảnh không tương thích. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `45` | `0x0078AED7` | Tài khoản sử dụng của bạn đã bị tạm khóa do phạm luật | Tài khoản của bạn đã tạm khóa do vi phạm quy định. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `46` | `0x0078AEE9` | Gian xảo trong vấn đáp của Bắc Đẩu Quân | Phát hiện gian lận trong phần vấn đáp của Bắc Đẩu Quân. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `47` | `0x0078AEFB` | Sự kiện kết thúc trước khi chiến đấu kết thúc | Sự kiện kết thúc trước khi trận chiến hoàn tất. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `48` | `0x0078AF0D` | Phi pháp sử dụng kỹ năng Kêu Gọi | Sử dụng trái phép kỹ năng Triệu Hồi. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `49` | `0x0078AF1F` | Cấm vận gia nhập đối với bạn chưa đủ 18 | Người chơi chưa đủ 18 tuổi không được phép tham gia. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `50` | `0x0078AF2E` | Đăng nhập thi đấu chuyên thuộc Server | Đăng nhập máy chủ thi đấu chuyên dụng. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `51` | `0x0078AF3D` | Không thể đăng nhập thi đấu chuyên thuộc Server | Không thể đăng nhập máy chủ thi đấu chuyên dụng. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `52` | `0x0078AF4C` | Không thể đăng nhập thi đấu chuyên thuộc Server | Không thể đăng nhập máy chủ thi đấu chuyên dụng. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `53` | `0x0078AF5B` | Chưa đăng nhập vào Server | Chưa đăng nhập vào máy chủ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `54` | `0x0078AF6A` | Hoạt động lôi đài đấu trận kết thúc | Sự kiện đấu trường đã kết thúc. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `55` | `0x0078AF79` | Mục đích trong lưu trữ tư liệu di dân server | Mục tiêu lưu dữ liệu chuyển máy chủ không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x00` | `0x0078AABE` | `56` | `0x0078AF88` | Server đang bận xin chờ tý xíu. | Máy chủ đang bận. Vui lòng chờ một lát. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `1` | `0x0078B1A8` | Đối tượng thì thầm | Đối tượng trò chuyện riêng. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `1` | `0x0078B1A8` | Rời mạng | Đã ngắt kết nối mạng. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `5` | `0x0078B464` | Không thể kết nối với server, xin bạn đợi sau đó thử lại lần nữa | Không thể kết nối đến máy chủ. Vui lòng chờ rồi thử lại. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `6` | `0x0078B4CB` | Số liệu mật mã sai lệch | Dữ liệu mã hóa không khớp. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `7` | `0x0078B532` | Đối tượng server chưa được liên mạng,xin bạn đợi sau đó thử lại lần nữa | Đối tượng máy chủ chưa kết nối mạng. Vui lòng chờ rồi thử lại. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `8` | `0x0078B599` | Account này hiện đang bị khóa | Tài khoản này hiện đang bị khóa. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x01` | `0x0078B149` | `10` | `0x0078B674` | Server đang bận xin chờ tý xíu. | Máy chủ đang bận. Vui lòng chờ một lát. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x09` | `0x0078D4AF` | — | `0x0078D4AF` | Tên bị trùng lập, hãy lập lại tên mới | Tên đã tồn tại. Vui lòng chọn tên mới. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x09` | `0x0078D4AF` | — | `0x0078D4AF` | Tên không hợp lệ, hãy lấy tên khác | Tên không hợp lệ. Vui lòng chọn tên khác. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x0D` | `0x0078DBAE` | `2` | `0x0078DC29` | Đối phương đang bận rộn | Đối phương đang bận rộn. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x0E` | `0x0078E01C` | `1` | `0x0078E06B` | sound\m004.wav | sound\m004.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x0E` | `0x0078E01C` | `3` | `0x0078E193` | Bạn chơi | Bạn chơi. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x0E` | `0x0078E01C` | `3` | `0x0078E193` | Tiếp nhận lời mời bạn hữu | Tiếp nhận lời mời bạn hữu. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x0E` | `0x0078E01C` | `3` | `0x0078E193` | Cự tuyệt gia nhễp bạn hũu | Cự tuyệt gia nhễp bạn hũu. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x0E` | `0x0078E01C` | `3` | `0x0078E193` | Không hồi ứng | Không hồi ứng. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x0E` | `0x0078E01C` | `6` | `0x0078E331` | Thùng thư bạn hữu đã đầy | Thùng thư bạn hữu đã đầy. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x10` | `0x0078E593` | `2` | `0x0078E7D9` | Thật thông minh...Chúc mừng bạn đã trả lời đúng!! | Thật thông minh...Chúc mừng bạn đã trả lời đúng!! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x10` | `0x0078E593` | `2` | `0x0078E7D9` | Ồ...bạn trả lời sai rồi! | Ồ...bạn trả lời sai rồi! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x10` | `0x0078E593` | `2` | `0x0078E7D9` | Ồ... Bạn đã sai đến lần thứ 3 rồi! | Ồ... Bạn đã sai đến lần thứ 3 rồi! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x10` | `0x0078E593` | `2` | `0x0078E7D9` | Ồ,...Trả lời sai quá nhiều rồi! | Ồ, ...Trả lời sai quá nhiều rồi! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x10` | `0x0078E593` | `8` | `0x0078E943` | Giải trừ lệnh cấm 15 phút cấm nói | Giải trừ lệnh cấm 15 phút cấm nói. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x10` | `0x0078E593` | `13` | `0x0078E9BB` | Giải trừ cấm Kênh GM 15 phút | Giải trừ cấm Kênh GM 15 phút. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x17` | `0x007902FB` | `15` | `0x007905F2` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x17` | `0x007902FB` | `25` | `0x00790811` | Vật phẩm này tạm thời không thể nhặt lên | Tạm thời không thể nhặt vật phẩm này. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x17` | `0x007902FB` | `36` | `0x007908E7` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x17` | `0x007902FB` | `59` | `0x00790B02` | Hủy bỏ giao dịch | Hủy bỏ giao dịch. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x17` | `0x007902FB` | `109` | `0x00790D97` | Tặng 1 Chuyển Đản ngoài định mức | Tặng 1 Chuyển Đản ngoài định mức. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x17` | `0x007902FB` | `114` | `0x00790E01` | có được %s%d cái | có được %s%d cái | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x18` | `0x00790ED5` | `3` | `0x007912C2` | Dung lượng nhiệm vụ đã đầy | Dung lượng nhiệm vụ đã đầy. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `1` | `0x007917BA` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `1` | `0x007917BA` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `1` | `0x007917BA` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `2` | `0x007918E3` | Thất bại giảm thiểu | Thất bại giảm thiểu. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `3` | `0x007919AF` | Dung lượng tiền bạc không đủ | Dung lượng tiền bạc không đủ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `8` | `0x00791A82` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `8` | `0x00791A82` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `8` | `0x00791A82` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `8` | `0x00791A82` | Thủy binh | Thủy binh. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `9` | `0x00791B93` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `9` | `0x00791B93` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `9` | `0x00791B93` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `9` | `0x00791B93` | Tài bảo | Tài bảo. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `10` | `0x00791C93` | sound\WA0014.wav | sound\WA0014.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `10` | `0x00791C93` | Nhận được | Nhận được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `10` | `0x00791C93` | Thất bại đoạt được | Thất bại đoạt được. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1A` | `0x0079175F` | `10` | `0x00791C93` | Điểm đạn dược | Điểm đạn dược. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1B` | `0x00791D80` | — | `0x00791D80` | Giao dịch thành công | Giao dịch thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1B` | `0x00791D80` | — | `0x00791D80` | Xin lỗi, vàng của bạn không đủ! | Xin lỗi, bạn không đủ vàng. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x1B` | `0x00791D80` | — | `0x00791D80` | Xin lỗi, bảng vật phẩm của bạn đã đầy! | Xin lỗi, túi vật phẩm của bạn đã đầy. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x1B` | `0x00791D80` | — | `0x00791D80` | Xin lỗi, Số lượng vật phẩm bạn mua đã đầy! | Xin lỗi, số lượng vật phẩm bạn mua đã đạt giới hạn. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x1B` | `0x00791D80` | — | `0x00791D80` | Xin lỗi, giao dịch thất bại | Xin lỗi, giao dịch thất bại. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x1B` | `0x00791D80` | — | `0x00791D80` | Hệ thống gián đoạn giao dịch | Hệ thống gián đoạn giao dịch. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1D` | `0x00791F76` | `1` | `0x00791FCD` | Lưu trữ | Lưu trữ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1D` | `0x00791F76` | `1` | `0x00791FCD` | Thất bại lưu trữ | Thất bại lưu trữ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1D` | `0x00791F76` | `2` | `0x0079206A` | Rút nhận | Rút nhận. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1D` | `0x00791F76` | `2` | `0x0079206A` | Thất bại rút nhận | Thất bại rút nhận. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1D` | `0x00791F76` | `3` | `0x00792107` | Dung lượng tiền trang không đủ | Dung lượng tiền trang không đủ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1F` | `0x007922E6` | `1` | `0x00792351` | Khách hàng quý mến, có rảnh thì ghé nhé! | Khách hàng quý mến, có rảnh thì ghé nhé! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1F` | `0x007922E6` | `1` | `0x00792351` | Số tiền trên người bạn không đủ nhé! | Số tiền trên người bạn không đủ nhé! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1F` | `0x007922E6` | `1` | `0x00792351` | Khách hàng sức khỏe rất tốt, không cần phải nghỉ ngơi nữa | Khách hàng sức khỏe rất tốt, không cần phải nghỉ ngơi nữa. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x1F` | `0x007922E6` | `13` | `0x00792824` | Đã có một võ tướng tương tự trong nhà trọ | Đã có một võ tướng tương tự trong nhà trọ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x21` | `0x007928E4` | — | `0x007928E4` | Đối phương chưa mở chức năng PK / PvP | Đối phương chưa mở chức năng PK / PvP. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x21` | `0x007928E4` | — | `0x007928E4` | Đối phương chưa mở chức năng Tham Chiến | Đối phương chưa mở chức năng Tham Chiến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x21` | `0x007928E4` | — | `0x007928E4` | Trong khi trả lời câu hỏi Bắc Tinh Quân không thể quan chiến | Trong khi trả lời câu hỏi của Bắc Tinh Quân, không thể quan chiến. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x21` | `0x007928E4` | — | `0x007928E4` | Chiến đấu đặc thù không thể tham chiến | Chiến đấu đặc thù không thể tham chiến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x22` | `0x00792ACC` | — | `0x00792ACC` | Điểm số còn lại của bạn hiện tại là | Điểm số còn lại của bạn hiện tại là. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `1` | `0x00792C0C` | Sửa đổi thất bại | Sửa đổi thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `1` | `0x00792C0C` | Sửa đổi thành công | Sửa đổi thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `1` | `0x00792C0C` | Mật mã cũ sai lầm | Mật mã cũ sai lầm. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `1` | `0x00792C0C` | Mã cá nhân cũ sai lầm | Mã cá nhân cũ sai lầm. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `1` | `0x00792C0C` | Mật mã quá ngắn | Mật mã quá ngắn. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `1` | `0x00792C0C` | Không thể sửa đổi mật mã và mã cá nhân trên server này | Không thể sửa đổi mật mã và mã cá nhân trên máy chủ này. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `2` | `0x00792D2F` | Loại trừ nhân vật thành công | Loại trừ nhân vật thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `2` | `0x00792D2F` | Mật mã loại trừ nhân vật sai lệch | Mật mã loại trừ nhân vật sai lệch. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `2` | `0x00792D2F` | Mã cá nhân loại trừ nhân vât sai lệch | Mã cá nhân loại trừ nhân vât sai lệch. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `2` | `0x00792D2F` | Loại trừ nhân vật thất bại | Loại trừ nhân vật thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `3` | `0x00792E00` | Lưu trữ thành công | Lưu trữ thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `3` | `0x00792E00` | Lưu trữ thất bại | Lưu trữ thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `3` | `0x00792E00` | Tài khoản thẻ sai lầm | Tài khoản thẻ không hợp lệ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x23` | `0x00792BAD` | `3` | `0x00792E00` | Mật mã sai lệch | Mật mã không khớp. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x23` | `0x00792BAD` | `3` | `0x00792E00` | Thẻ đã sử dụng qua | Thẻ đã sử dụng qua. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `3` | `0x00792E00` | Đã lưu trữ điểm số thẻ khởi động | Đã lưu trữ điểm số thẻ khởi động. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `4` | `0x00792F5D` | Điểm số còn dư | Điểm số còn dư. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `4` | `0x00792F5D` | Kỳ hạn có thể chơi | Kỳ hạn có thể chơi. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `7` | `0x00793113` | Giới thiệu thành công | Giới thiệu thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `7` | `0x00793113` | Giới thiệu thất bại! Hãy xác định lại tư cách người giới thiệu và người được giới thiệu có phù hợp hay không! | Giới thiệu thất bại! Hãy xác định lại tư cách người giới thiệu và người được giới thiệu có phù hợp hay không! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `8` | `0x0079317C` | Được giới thiệu 5 lần được thưởng số điểm là Kỳ hạn có thể chơi | Được giới thiệu 5 lần được thưởng số điểm là Kỳ hạn có thể chơi. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `9` | `0x00793224` | Điểm số được giới thiệu đạt đến | Điểm số được giới thiệu đạt đến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x23` | `0x00792BAD` | `10` | `0x007933F0` | Do quy định số thời gian online, lần sau bạn có thể đăng nhập thời gian tiến hành game là | Do quy định số thời gian online, lần sau bạn có thể đăng nhập thời gian tiến hành game là. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x24` | `0x0079346B` | `7` | `0x0079359F` | Bị cấm gửi lời nhắn | Bạn bị cấm gửi tin nhắn. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x27` | `0x007938C3` | `2` | `0x007939F6` | Hãy chú ý, Quân đoàn của bạn do chưa đủ số lượng 10 đoàn viên ở cấp 15, nên sẽ | Hãy chú ý, Quân đoàn của bạn do chưa đủ số lượng 10 đoàn viên ở cấp 15, nên sẽ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `2` | `0x007939F6` | yyyy/m/d | yyyy/m/d | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `2` | `0x007939F6` | Giải tán | Giải tán. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `5` | `0x00793B08` | sound\m004.wav | sound\m004.wav | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `16` | `0x00793EC2` | Hãy chú ý, Quân đoàn của bạn do chưa đủ số lượng 10 đoàn viên ở cấp 15, nên sẽ | Hãy chú ý, Quân đoàn của bạn do chưa đủ số lượng 10 đoàn viên ở cấp 15, nên sẽ. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `16` | `0x00793EC2` | yyyy/m/d | yyyy/m/d | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `16` | `0x00793EC2` | Giải tán | Giải tán. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `16` | `0x00793EC2` | Số lượng đoàn viên đủ 10 người đạt đẳng cấp 15 nên quân đoàn của bạn sẽ không bị giải tán | Số lượng đoàn viên đủ 10 người đạt đẳng cấp 15 nên quân đoàn của bạn sẽ không bị giải tán. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x27` | `0x007938C3` | `20` | `0x00793FC3` | Tổ chức thùng thư đã đầy! | Hộp thư của tổ chức đã đầy. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x27` | `0x007938C3` | `29` | `0x007940DC` | Trong chiến tranh công kích thành trì không thể chọn chức năng rời khỏi | Trong chiến tranh công thành, không thể chọn chức năng rời khỏi. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x27` | `0x007938C3` | `29` | `0x007940DC` | Trong chiến tranh công kích thành trì không thể chọn chức năng xóa bỏ | Trong chiến tranh công thành, không thể chọn chức năng xóa bỏ. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x27` | `0x007938C3` | `29` | `0x007940DC` | Trong chiến tranh công kích thành trì không thể chọn chức năng giải tán | Trong chiến tranh công thành, không thể chọn chức năng giải tán. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x27` | `0x007938C3` | `29` | `0x007940DC` | Trong chiến tranh công kích thành trì bạn chơi không thể xóa bỏ nhân vật | Trong chiến tranh công thành, người chơi không thể xóa nhân vật. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x27` | `0x007938C3` | `29` | `0x007940DC` | Trong chiến tranh công kích thành trì không thể thiết lập quân đoàn | Trong chiến tranh công thành, không thể thành lập quân đoàn. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2B` | `0x00794799` | `6` | `0x00794881` | Trong chiến tranh công kích thành trì không thể chọn chức năng liên minh với người khác | Trong chiến tranh công kích thành trì không thể chọn chức năng liên minh với người khác. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2B` | `0x00794799` | `6` | `0x00794881` | Trong chiến tranh công kích thành trì không thể chọn chức năng hồi ứng liên minh với người khác | Trong chiến tranh công kích thành trì không thể chọn chức năng hồi ứng liên minh với người khác. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2B` | `0x00794799` | `6` | `0x00794881` | Trong chiến tranh công kích thành trì không thể chọn chức năng rời khỏi liên minh | Trong chiến tranh công kích thành trì không thể chọn chức năng rời khỏi liên minh. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `1` | `0x007949EA` | Kết hôn thành công | Kết hôn thành công. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2D` | `0x00794977` | `1` | `0x007949EA` | Không thể đồng giới tính | Không thể đồng giới tính. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `1` | `0x007949EA` | Có người không đủ đẳng cấp | Có người không đủ đẳng cấp. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `2` | `0x00794B9D` | Sai lầm | Sai lầm. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `2` | `0x00794B9D` | Ly hôn thành công | Ly hôn thành công. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2D` | `0x00794977` | `2` | `0x00794B9D` | Bạn chơi độc thân | Bạn chơi độc thân. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `6` | `0x00794D3D` | Sai lầm | Sai lầm. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `6` | `0x00794D3D` | Tặng lễ kim thành công | Tặng lễ kim thành công. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2D` | `0x00794977` | `6` | `0x00794D3D` | Đối phương không có trên mạng | Đối phương không trực tuyến. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2D` | `0x00794977` | `6` | `0x00794D3D` | Đối phương đã hoàn hôn | Đối phương đã kết hôn. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2D` | `0x00794977` | `6` | `0x00794D3D` | Bạn không có nhiều tiền như vậy để phát | Bạn không có nhiều tiền như vậy để phát. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `8` | `0x00794E3C` | Đối phương không đồng ý lấy bạn | Đối phương không đồng ý kết hôn. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x2D` | `0x00794977` | `9` | `0x00794E7E` | Số tiền thu người đến | Số tiền thu người đến. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x2D` | `0x00794977` | `9` | `0x00794E7E` | Lễ kim | Lễ kim. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x37` | `0x007954A5` | — | `0x007954A5` | Thành công | Thành công. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x37` | `0x007954A5` | — | `0x007954A5` | Thất bại | Thất bại. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x3D` | `0x007957DC` | `5` | `0x0079588B` | Thời gian đối chiếu giải thưởng là ngày mở giải thưởng lúc 20:00 đến ngày mở giải thưởng lần sau là trước 18:00! | Thời gian đối chiếu giải thưởng là ngày mở giải thưởng lúc 20:00 đến ngày mở giải thưởng lần sau là trước 18:00! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x3D` | `0x007957DC` | `5` | `0x0079588B` | Bạn mang theo bên mình rất nhiều tiền | Bạn mang theo bên mình rất nhiều tiền. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x3D` | `0x007957DC` | `5` | `0x0079588B` | Xổ số kỳ này bạn không trúng thưởng | Bạn không trúng thưởng ở kỳ xổ số này. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x3D` | `0x007957DC` | `5` | `0x0079588B` | Kỳ này cũng chồa mở thưởng! | Kỳ này cũng chồa mở thưởng! | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |
| `0x3D` | `0x007957DC` | `5` | `0x0079588B` | Đã tạm ngưng đặt cược, xin mời lần sau nha! | Đặt cược đã tạm dừng. Vui lòng thử lại vào lần sau. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x3D` | `0x007957DC` | `5` | `0x0079588B` | Đã không thể đối chiếu xổ số rồi, xin hãy đợi mở thưởng lần sau! | Chưa thể đối chiếu kết quả xổ số. Vui lòng chờ kỳ mở thưởng tiếp theo. | Hiệu đính ngữ nghĩa/câu chữ |
| `0x47` | `0x007961C1` | — | `0x007961C1` | Đã mở cơ chế Phòng thẩm mật | Đã mở cơ chế Phòng thẩm mật. | Chuẩn hóa thuật ngữ/chính tả; giữ sát literal |

## 4. Chuỗi giữ nguyên có điều kiện

| Literal | Cách thể hiện | Lý do |
|---|---|---|
| `sound\...wav` | Giữ nguyên đường dẫn asset | Không phải câu VISCII cần dịch. |
| `yyyy/m/d` | Giữ nguyên format ngày | Đây là format string. |
| `%s`, `%d` | Giữ nguyên placeholder | Thay thế tại runtime. |
| Tên skill/NPC/game-specific như `Bắc Đẩu Quân`, `Triệu Hồi` | Việt hóa nhất quán nhưng giữ tên riêng | Không đủ localization table để đổi tên chính xác hơn. |
| Cụm “mục tiêu lưu dữ liệu chuyển máy chủ” | Hiệu đính thận trọng | Cần format packet/helper để biết field chính xác. |

## 5. Phạm vi chính xác

Bản hiệu đính phân biệt rõ **literal được giải mã** và **câu tiếng Việt đọc tự nhiên**. Literal/VISCII gốc, VA, basic block và status/subopcode là chứng cứ A. Bản tiếng Việt hiệu đính là diễn đạt B/C nhằm làm rõ ý nghĩa, không phải byte string được client hiển thị nguyên văn. [1]

## Tài liệu tham chiếu

[1]: aLogin.exe "Mẫu PE do người dùng cung cấp; literal Delphi, VISCII, VA, basic block và status được phân tích tĩnh"
[2]: Bao_cao_dien_giai_ngu_nghia_call_graph_S_to_C.md "Tài liệu người dùng gửi để hiệu đính; được dùng làm bố cục và đối chiếu nội dung"
