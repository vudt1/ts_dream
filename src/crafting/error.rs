//! Error types for crafting and socketing system.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CraftError {
    #[error("Hợp thành cần ít nhất 2 vật phẩm")]
    NeedAtLeastTwoItems,

    #[error("Có lẫn vật phẩm không thể hợp thành")]
    InvalidItemForCompound,

    #[error("Số lượng một vật phẩm đơn lẻ không được vượt quá 50")]
    QuantityExceeded,

    #[error("Lỗi cơ sở dữ liệu hợp thành, tạo ra loại vật phẩm không được phép ({0})")]
    ForbiddenResultKind(u16),

    #[error("Cơ sở dữ liệu vật phẩm không có vật phẩm phù hợp")]
    NoMatchingItem,

    #[error("Túi đồ đã đầy")]
    BagFull,

    #[error("Người chơi không sở hữu đủ vật phẩm")]
    InsufficientItems,

    #[error("Ô trang bị không hợp lệ")]
    InvalidEquipmentSlot,

    #[error("Vật phẩm không phải trang bị có thể khảm")]
    InvalidEquipmentType,

    #[error("Lông vũ không hợp lệ")]
    InvalidFeather,

    #[error("Cấp độ lông vũ không phù hợp")]
    InvalidFeatherLevel,

    #[error("Trang bị đã đạt cấp cường hóa tối đa")]
    EquipmentMaxLevel,

    #[error("Chưa chọn hệ nguyên tố")]
    NoElementSelected,
}
