//! System Alert protocol definitions and builder (Opcode 0x00).
//!
//! Pure S → C protocol frames for system warnings, disconnect notices, and error dialogs.
//! Client never sends Opcode 0x00 to the Server.
//!
//! Wire format: `F4 44 02 00 00 <sub_opcode>` (Header: F4 44, Len: 2B LE = 0x0002, Opcode: 0x00, SubOp: 1B).

use super::OP_SYSTEM_ALERT;

/// Sub-opcode status codes for `OP_SYSTEM_ALERT` (0x00).
/// Status codes 0..=56 correspond to client-side switch cases and localized strings,
/// with code 57 representing `UnknownReason` (client default fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SystemAlertReason {
    /// 0 - Mất kết nối với máy chủ.
    Disconnected = 0,
    /// 1 - Dữ liệu quá lớn; kết nối đã bị ngắt.
    DataTooLarge = 1,
    /// 2 - Trả lời sai 3 lần; kết nối sẽ bị ngắt.
    ThreeWrongAnswers = 2,
    /// 3 - Đăng nhập sai 3 lần.
    ThreeWrongLogins = 3,
    /// 4 - Do máy chủ gặp sự cố.
    ServerError = 4,
    /// 5 - Phát hiện hành vi vi phạm (mã 1); kết nối đã bị ngắt.
    RuleViolation1 = 5,
    /// 6 - Phát hiện hành vi vi phạm (mã 2); kết nối đã bị ngắt.
    RuleViolation2 = 6,
    /// 7 - Không tìm thấy sự kiện cần thực hiện; kết nối đã bị ngắt.
    EventNotFound = 7,
    /// 8 - Không thể thiết lập kết nối; kết nối đã bị ngắt.
    CannotConnect8 = 8,
    /// 9 - Không thể thiết lập kết nối; kết nối đã bị ngắt.
    CannotConnect9 = 9,
    /// 10 - Kết nối bị ngắt do phiên bản không tương thích.
    VersionMismatch = 10,
    /// 11 - Kết nối gặp sự cố; đã ngắt kết nối.
    ConnectionError = 11,
    /// 12 - Phát hiện sử dụng chương trình không hợp lệ.
    IllegalProgram = 12,
    /// 13 - Đã mất kết nối.
    ConnectionLost = 13,
    /// 14 - Kết nối bị ngắt do phát hiện chương trình bên thứ ba.
    ThirdPartyProgram = 14,
    /// 15 - Đã gỡ bỏ thành công. Vui lòng khởi động lại máy.
    RemovedSuccessfully = 15,
    /// 16 - Kết nối bị ngắt do địa chỉ IP đăng nhập không hợp lệ.
    IllegalIpLogin = 16,
    /// 17 - Phiên bản không tương thích. Vui lòng cập nhật phiên bản mới.
    UpdateVersionRequired = 17,
    /// 18 - Dữ liệu đã thay đổi; kết nối đã bị ngắt.
    DataChanged = 18,
    /// 19 - Kết nối bị ngắt do tài khoản đã đăng nhập ở nơi khác.
    DuplicateLoginOtherLocation = 19,
    /// 20 - Hệ thống gặp sự cố bất thường.
    SystemAbnormal = 20,
    /// 21 - Lỗi lưu trữ dữ liệu; kết nối đã bị ngắt.
    StorageError = 21,
    /// 22 - Kết nối bị ngắt do định dạng dữ liệu không hợp lệ.
    InvalidDataFormat = 22,
    /// 23 - Lỗi đổi tên; kết nối đã bị ngắt.
    RenameError = 23,
    /// 24 - Mật khẩu quá ngắn; kết nối đã bị ngắt.
    PasswordTooShort = 24,
    /// 25 - Tên đã tồn tại; kết nối đã bị ngắt.
    DuplicateName = 25,
    /// 26 - Phát hiện sự kiện vi phạm; kết nối đã bị ngắt.
    ViolationEvent = 26,
    /// 27 - Kết nối bị ngắt do thông tin đăng nhập không hợp lệ.
    InvalidLoginInfo = 27,
    /// 28 - Kết nối bị ngắt bởi cơ chế bảo vệ.
    ProtectionDisconnect = 28,
    /// 29 - Dữ liệu quá lớn.
    DataTooLargeNotice = 29,
    /// 30 - Tài khoản đã bị khóa; kết nối đã bị ngắt.
    AccountLocked = 30,
    /// 31 - Không thể sử dụng ID này.
    CannotUseId = 31,
    /// 32 - Khung cảnh chiến đấu xảy ra lỗi.
    BattleSceneError = 32,
    /// 33 - Ký hiệu và khung cảnh không tương thích; kết nối đã bị ngắt.
    SceneSymbolMismatch = 33,
    /// 34 - Đăng nhập lại thông qua máy chủ.
    ReloginViaServer = 34,
    /// 35 - Quy ước/phiên đăng nhập.
    LoginProtocol = 35,
    /// 36 - ID nằm ngoài phạm vi hợp lệ.
    IdOutOfRange = 36,
    /// 37 - Mất kết nối do không cùng khung cảnh.
    DifferentSceneDisconnect = 37,
    /// 38 - Mục tiêu khung cảnh không hợp lệ; kết nối đã bị ngắt.
    InvalidSceneTarget = 38,
    /// 39 - Mất kết nối với máy chủ.
    Disconnected39 = 39,
    /// 40 - Sửa đổi tệp dữ liệu nhân vật; kết nối đã bị ngắt.
    CharacterFileModified = 40,
    /// 41 - Lưu lại sau khi đăng nhập.
    SaveAfterLogin = 41,
    /// 42 - Dữ liệu chiến đấu đã bị chỉnh sửa.
    BattleDataModified = 42,
    /// 43 - Dữ liệu trạng thái chiến đấu của người chơi không khớp.
    BattleStateMismatch = 43,
    /// 44 - Sự kiện và khung cảnh không tương thích.
    SceneEventMismatch = 44,
    /// 45 - Tài khoản của bạn đã tạm khóa do vi phạm quy định.
    AccountSuspendedViolation = 45,
    /// 46 - Phát hiện gian lận trong phần vấn đáp của Bắc Đẩu Quân.
    PolarisQuizCheating = 46,
    /// 47 - Sự kiện kết thúc trước khi trận chiến hoàn tất.
    EventEndedBeforeBattle = 47,
    /// 48 - Sử dụng trái phép kỹ năng Triệu Hồi.
    IllegalSummonSkill = 48,
    /// 49 - Người chơi chưa đủ 18 tuổi không được phép tham gia.
    UnderageRestriction = 49,
    /// 50 - Đăng nhập máy chủ thi đấu chuyên dụng.
    TournamentServerLogin = 50,
    /// 51 - Không thể đăng nhập máy chủ thi đấu chuyên dụng.
    CannotLoginTournament51 = 51,
    /// 52 - Không thể đăng nhập máy chủ thi đấu chuyên dụng.
    CannotLoginTournament52 = 52,
    /// 53 - Chưa đăng nhập vào máy chủ.
    NotLoggedInYet = 53,
    /// 54 - Sự kiện đấu trường đã kết thúc.
    ArenaEventEnded = 54,
    /// 55 - Mục tiêu lưu dữ liệu chuyển máy chủ không hợp lệ.
    MigrationTargetError = 55,
    /// 56 - Máy chủ đang bận. Vui lòng chờ một lát.
    ServerBusy = 56,
    /// 57 - Lý do không xác định (client default fallback / 0xFF).
    UnknownReason = 57,
}

impl SystemAlertReason {
    /// Return the numeric sub-opcode byte (0..=57).
    pub const fn sub_code(self) -> u8 {
        self as u8
    }

    /// Return the Vietnamese description according to the rectified VISCII -> UTF-8 spec.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Disconnected => "Mất kết nối với máy chủ.",
            Self::DataTooLarge => "Dữ liệu quá lớn; kết nối đã bị ngắt.",
            Self::ThreeWrongAnswers => "Trả lời sai 3 lần; kết nối sẽ bị ngắt.",
            Self::ThreeWrongLogins => "Đăng nhập sai 3 lần.",
            Self::ServerError => "Do máy chủ gặp sự cố.",
            Self::RuleViolation1 => "Phát hiện hành vi vi phạm (mã 1); kết nối đã bị ngắt.",
            Self::RuleViolation2 => "Phát hiện hành vi vi phạm (mã 2); kết nối đã bị ngắt.",
            Self::EventNotFound => "Không tìm thấy sự kiện cần thực hiện; kết nối đã bị ngắt.",
            Self::CannotConnect8 => "Không thể thiết lập kết nối; kết nối đã bị ngắt.",
            Self::CannotConnect9 => "Không thể thiết lập kết nối; kết nối đã bị ngắt.",
            Self::VersionMismatch => "Kết nối bị ngắt do phiên bản không tương thích.",
            Self::ConnectionError => "Kết nối gặp sự cố; đã ngắt kết nối.",
            Self::IllegalProgram => "Phát hiện sử dụng chương trình không hợp lệ.",
            Self::ConnectionLost => "Đã mất kết nối.",
            Self::ThirdPartyProgram => "Kết nối bị ngắt do phát hiện chương trình bên thứ ba.",
            Self::RemovedSuccessfully => "Đã gỡ bỏ thành công. Vui lòng khởi động lại máy.",
            Self::IllegalIpLogin => "Kết nối bị ngắt do địa chỉ IP đăng nhập không hợp lệ.",
            Self::UpdateVersionRequired => "Phiên bản không tương thích. Vui lòng cập nhật phiên bản mới.",
            Self::DataChanged => "Dữ liệu đã thay đổi; kết nối đã bị ngắt.",
            Self::DuplicateLoginOtherLocation => "Kết nối bị ngắt do tài khoản đã đăng nhập ở nơi khác.",
            Self::SystemAbnormal => "Hệ thống gặp sự cố bất thường.",
            Self::StorageError => "Lỗi lưu trữ dữ liệu; kết nối đã bị ngắt.",
            Self::InvalidDataFormat => "Kết nối bị ngắt do định dạng dữ liệu không hợp lệ.",
            Self::RenameError => "Lỗi đổi tên; kết nối đã bị ngắt.",
            Self::PasswordTooShort => "Mật khẩu quá ngắn; kết nối đã bị ngắt.",
            Self::DuplicateName => "Tên đã tồn tại; kết nối đã bị ngắt.",
            Self::ViolationEvent => "Phát hiện sự kiện vi phạm; kết nối đã bị ngắt.",
            Self::InvalidLoginInfo => "Kết nối bị ngắt do thông tin đăng nhập không hợp lệ.",
            Self::ProtectionDisconnect => "Kết nối bị ngắt bởi cơ chế bảo vệ.",
            Self::DataTooLargeNotice => "Dữ liệu quá lớn.",
            Self::AccountLocked => "Tài khoản đã bị khóa; kết nối đã bị ngắt.",
            Self::CannotUseId => "Không thể sử dụng ID này.",
            Self::BattleSceneError => "Khung cảnh chiến đấu xảy ra lỗi.",
            Self::SceneSymbolMismatch => "Ký hiệu và khung cảnh không tương thích; kết nối đã bị ngắt.",
            Self::ReloginViaServer => "Đăng nhập lại thông qua máy chủ.",
            Self::LoginProtocol => "Quy ước/phiên đăng nhập.",
            Self::IdOutOfRange => "ID nằm ngoài phạm vi hợp lệ.",
            Self::DifferentSceneDisconnect => "Mất kết nối do không cùng khung cảnh.",
            Self::InvalidSceneTarget => "Mục tiêu khung cảnh không hợp lệ; kết nối đã bị ngắt.",
            Self::Disconnected39 => "Mất kết nối với máy chủ.",
            Self::CharacterFileModified => "Sửa đổi tệp dữ liệu nhân vật; kết nối đã bị ngắt.",
            Self::SaveAfterLogin => "Lưu lại sau khi đăng nhập.",
            Self::BattleDataModified => "Dữ liệu chiến đấu đã bị chỉnh sửa.",
            Self::BattleStateMismatch => "Dữ liệu trạng thái chiến đấu của người chơi không khớp.",
            Self::SceneEventMismatch => "Sự kiện và khung cảnh không tương thích.",
            Self::AccountSuspendedViolation => "Tài khoản của bạn đã tạm khóa do vi phạm quy định.",
            Self::PolarisQuizCheating => "Phát hiện gian lận trong phần vấn đáp của Bắc Đẩu Quân.",
            Self::EventEndedBeforeBattle => "Sự kiện kết thúc trước khi trận chiến hoàn tất.",
            Self::IllegalSummonSkill => "Sử dụng trái phép kỹ năng Triệu Hồi.",
            Self::UnderageRestriction => "Người chơi chưa đủ 18 tuổi không được phép tham gia.",
            Self::TournamentServerLogin => "Đăng nhập máy chủ thi đấu chuyên dụng.",
            Self::CannotLoginTournament51 => "Không thể đăng nhập máy chủ thi đấu chuyên dụng.",
            Self::CannotLoginTournament52 => "Không thể đăng nhập máy chủ thi đấu chuyên dụng.",
            Self::NotLoggedInYet => "Chưa đăng nhập vào máy chủ.",
            Self::ArenaEventEnded => "Sự kiện đấu trường đã kết thúc.",
            Self::MigrationTargetError => "Mục tiêu lưu dữ liệu chuyển máy chủ không hợp lệ.",
            Self::ServerBusy => "Máy chủ đang bận. Vui lòng chờ một lát.",
            Self::UnknownReason => "Lý do không xác định.",
        }
    }

    /// Parse a u8 status code into a `SystemAlertReason`. Unknown codes default to `UnknownReason`.
    pub const fn from_u8(code: u8) -> Self {
        match code {
            0 => Self::Disconnected,
            1 => Self::DataTooLarge,
            2 => Self::ThreeWrongAnswers,
            3 => Self::ThreeWrongLogins,
            4 => Self::ServerError,
            5 => Self::RuleViolation1,
            6 => Self::RuleViolation2,
            7 => Self::EventNotFound,
            8 => Self::CannotConnect8,
            9 => Self::CannotConnect9,
            10 => Self::VersionMismatch,
            11 => Self::ConnectionError,
            12 => Self::IllegalProgram,
            13 => Self::ConnectionLost,
            14 => Self::ThirdPartyProgram,
            15 => Self::RemovedSuccessfully,
            16 => Self::IllegalIpLogin,
            17 => Self::UpdateVersionRequired,
            18 => Self::DataChanged,
            19 => Self::DuplicateLoginOtherLocation,
            20 => Self::SystemAbnormal,
            21 => Self::StorageError,
            22 => Self::InvalidDataFormat,
            23 => Self::RenameError,
            24 => Self::PasswordTooShort,
            25 => Self::DuplicateName,
            26 => Self::ViolationEvent,
            27 => Self::InvalidLoginInfo,
            28 => Self::ProtectionDisconnect,
            29 => Self::DataTooLargeNotice,
            30 => Self::AccountLocked,
            31 => Self::CannotUseId,
            32 => Self::BattleSceneError,
            33 => Self::SceneSymbolMismatch,
            34 => Self::ReloginViaServer,
            35 => Self::LoginProtocol,
            36 => Self::IdOutOfRange,
            37 => Self::DifferentSceneDisconnect,
            38 => Self::InvalidSceneTarget,
            39 => Self::Disconnected39,
            40 => Self::CharacterFileModified,
            41 => Self::SaveAfterLogin,
            42 => Self::BattleDataModified,
            43 => Self::BattleStateMismatch,
            44 => Self::SceneEventMismatch,
            45 => Self::AccountSuspendedViolation,
            46 => Self::PolarisQuizCheating,
            47 => Self::EventEndedBeforeBattle,
            48 => Self::IllegalSummonSkill,
            49 => Self::UnderageRestriction,
            50 => Self::TournamentServerLogin,
            51 => Self::CannotLoginTournament51,
            52 => Self::CannotLoginTournament52,
            53 => Self::NotLoggedInYet,
            54 => Self::ArenaEventEnded,
            55 => Self::MigrationTargetError,
            56 => Self::ServerBusy,
            _ => Self::UnknownReason,
        }
    }

    /// Build the wire frame hex string (e.g. `"F4440200000A"`).
    pub fn to_hex(self) -> String {
        format!("F4440200{:02X}{:02X}", OP_SYSTEM_ALERT, self.sub_code())
    }
}

/// Helper struct providing convenient factory methods for constructing `OP_SYSTEM_ALERT` payload hex strings.
pub struct SystemAlert;

impl SystemAlert {
    /// Build wire hex string for a given alert reason.
    pub fn payload(reason: SystemAlertReason) -> String {
        reason.to_hex()
    }

    /// 0 - Mất kết nối với máy chủ.
    pub fn disconnected() -> String {
        SystemAlertReason::Disconnected.to_hex()
    }

    /// 1 - Dữ liệu quá lớn; kết nối đã bị ngắt.
    pub fn data_too_large() -> String {
        SystemAlertReason::DataTooLarge.to_hex()
    }

    /// 2 - Trả lời sai 3 lần; kết nối sẽ bị ngắt.
    pub fn three_wrong_answers() -> String {
        SystemAlertReason::ThreeWrongAnswers.to_hex()
    }

    /// 3 - Đăng nhập sai 3 lần.
    pub fn three_wrong_logins() -> String {
        SystemAlertReason::ThreeWrongLogins.to_hex()
    }

    /// 4 - Do máy chủ gặp sự cố.
    pub fn server_error() -> String {
        SystemAlertReason::ServerError.to_hex()
    }

    /// 5 - Phát hiện hành vi vi phạm (mã 1); kết nối đã bị ngắt.
    pub fn rule_violation_1() -> String {
        SystemAlertReason::RuleViolation1.to_hex()
    }

    /// 6 - Phát hiện hành vi vi phạm (mã 2); kết nối đã bị ngắt.
    pub fn rule_violation_2() -> String {
        SystemAlertReason::RuleViolation2.to_hex()
    }

    /// 7 - Không tìm thấy sự kiện cần thực hiện; kết nối đã bị ngắt.
    pub fn event_not_found() -> String {
        SystemAlertReason::EventNotFound.to_hex()
    }

    /// 8 - Không thể thiết lập kết nối; kết nối đã bị ngắt.
    pub fn cannot_connect() -> String {
        SystemAlertReason::CannotConnect8.to_hex()
    }

    /// 10 - Kết nối bị ngắt do phiên bản không tương thích.
    pub fn version_mismatch() -> String {
        SystemAlertReason::VersionMismatch.to_hex()
    }

    /// 11 - Kết nối gặp sự cố; đã ngắt kết nối.
    pub fn connection_error() -> String {
        SystemAlertReason::ConnectionError.to_hex()
    }

    /// 12 - Phát hiện sử dụng chương trình không hợp lệ.
    pub fn illegal_program() -> String {
        SystemAlertReason::IllegalProgram.to_hex()
    }

    /// 13 - Đã mất kết nối.
    pub fn connection_lost() -> String {
        SystemAlertReason::ConnectionLost.to_hex()
    }

    /// 14 - Kết nối bị ngắt do phát hiện chương trình bên thứ ba.
    pub fn third_party_program() -> String {
        SystemAlertReason::ThirdPartyProgram.to_hex()
    }

    /// 15 - Đã gỡ bỏ thành công. Vui lòng khởi động lại máy.
    pub fn removed_successfully() -> String {
        SystemAlertReason::RemovedSuccessfully.to_hex()
    }

    /// 16 - Kết nối bị ngắt do địa chỉ IP đăng nhập không hợp lệ.
    pub fn illegal_ip_login() -> String {
        SystemAlertReason::IllegalIpLogin.to_hex()
    }

    /// 17 - Phiên bản không tương thích. Vui lòng cập nhật phiên bản mới.
    pub fn update_version_required() -> String {
        SystemAlertReason::UpdateVersionRequired.to_hex()
    }

    /// 18 - Dữ liệu đã thay đổi; kết nối đã bị ngắt.
    pub fn data_changed() -> String {
        SystemAlertReason::DataChanged.to_hex()
    }

    /// 19 - Kết nối bị ngắt do tài khoản đã đăng nhập ở nơi khác.
    pub fn duplicate_login_other_location() -> String {
        SystemAlertReason::DuplicateLoginOtherLocation.to_hex()
    }

    /// 20 - Hệ thống gặp sự cố bất thường.
    pub fn system_abnormal() -> String {
        SystemAlertReason::SystemAbnormal.to_hex()
    }

    /// 21 - Lỗi lưu trữ dữ liệu; kết nối đã bị ngắt.
    pub fn storage_error() -> String {
        SystemAlertReason::StorageError.to_hex()
    }

    /// 22 - Kết nối bị ngắt do định dạng dữ liệu không hợp lệ.
    pub fn invalid_data_format() -> String {
        SystemAlertReason::InvalidDataFormat.to_hex()
    }

    /// 23 - Lỗi đổi tên; kết nối đã bị ngắt.
    pub fn rename_error() -> String {
        SystemAlertReason::RenameError.to_hex()
    }

    /// 24 - Mật khẩu quá ngắn; kết nối đã bị ngắt.
    pub fn password_too_short() -> String {
        SystemAlertReason::PasswordTooShort.to_hex()
    }

    /// 25 - Tên đã tồn tại; kết nối đã bị ngắt.
    pub fn duplicate_name() -> String {
        SystemAlertReason::DuplicateName.to_hex()
    }

    /// 26 - Phát hiện sự kiện vi phạm; kết nối đã bị ngắt.
    pub fn violation_event() -> String {
        SystemAlertReason::ViolationEvent.to_hex()
    }

    /// 27 - Kết nối bị ngắt do thông tin đăng nhập không hợp lệ.
    pub fn invalid_login_info() -> String {
        SystemAlertReason::InvalidLoginInfo.to_hex()
    }

    /// 28 - Kết nối bị ngắt bởi cơ chế bảo vệ.
    pub fn protection_disconnect() -> String {
        SystemAlertReason::ProtectionDisconnect.to_hex()
    }

    /// 29 - Dữ liệu quá lớn.
    pub fn data_too_large_notice() -> String {
        SystemAlertReason::DataTooLargeNotice.to_hex()
    }

    /// 30 - Tài khoản đã bị khóa; kết nối đã bị ngắt.
    pub fn account_locked() -> String {
        SystemAlertReason::AccountLocked.to_hex()
    }

    /// 31 - Không thể sử dụng ID này.
    pub fn cannot_use_id() -> String {
        SystemAlertReason::CannotUseId.to_hex()
    }

    /// 32 - Khung cảnh chiến đấu xảy ra lỗi.
    pub fn battle_scene_error() -> String {
        SystemAlertReason::BattleSceneError.to_hex()
    }

    /// 33 - Ký hiệu và khung cảnh không tương thích; kết nối đã bị ngắt.
    pub fn scene_symbol_mismatch() -> String {
        SystemAlertReason::SceneSymbolMismatch.to_hex()
    }

    /// 34 - Đăng nhập lại thông qua máy chủ.
    pub fn relogin_via_server() -> String {
        SystemAlertReason::ReloginViaServer.to_hex()
    }

    /// 35 - Quy ước/phiên đăng nhập.
    pub fn login_protocol() -> String {
        SystemAlertReason::LoginProtocol.to_hex()
    }

    /// 36 - ID nằm ngoài phạm vi hợp lệ.
    pub fn id_out_of_range() -> String {
        SystemAlertReason::IdOutOfRange.to_hex()
    }

    /// 37 - Mất kết nối do không cùng khung cảnh.
    pub fn different_scene_disconnect() -> String {
        SystemAlertReason::DifferentSceneDisconnect.to_hex()
    }

    /// 38 - Mục tiêu khung cảnh không hợp lệ; kết nối đã bị ngắt.
    pub fn invalid_scene_target() -> String {
        SystemAlertReason::InvalidSceneTarget.to_hex()
    }

    /// 40 - Sửa đổi tệp dữ liệu nhân vật; kết nối đã bị ngắt.
    pub fn character_file_modified() -> String {
        SystemAlertReason::CharacterFileModified.to_hex()
    }

    /// 41 - Lưu lại sau khi đăng nhập.
    pub fn save_after_login() -> String {
        SystemAlertReason::SaveAfterLogin.to_hex()
    }

    /// 42 - Dữ liệu chiến đấu đã bị chỉnh sửa.
    pub fn battle_data_modified() -> String {
        SystemAlertReason::BattleDataModified.to_hex()
    }

    /// 43 - Dữ liệu trạng thái chiến đấu của người chơi không khớp.
    pub fn battle_state_mismatch() -> String {
        SystemAlertReason::BattleStateMismatch.to_hex()
    }

    /// 44 - Sự kiện và khung cảnh không tương thích.
    pub fn scene_event_mismatch() -> String {
        SystemAlertReason::SceneEventMismatch.to_hex()
    }

    /// 45 - Tài khoản của bạn đã tạm khóa do vi phạm quy định.
    pub fn account_suspended() -> String {
        SystemAlertReason::AccountSuspendedViolation.to_hex()
    }

    /// 46 - Phát hiện gian lận trong phần vấn đáp của Bắc Đẩu Quân.
    pub fn polaris_quiz_cheating() -> String {
        SystemAlertReason::PolarisQuizCheating.to_hex()
    }

    /// 47 - Sự kiện kết thúc trước khi trận chiến hoàn tất.
    pub fn event_ended_before_battle() -> String {
        SystemAlertReason::EventEndedBeforeBattle.to_hex()
    }

    /// 48 - Sử dụng trái phép kỹ năng Triệu Hồi.
    pub fn illegal_summon_skill() -> String {
        SystemAlertReason::IllegalSummonSkill.to_hex()
    }

    /// 49 - Người chơi chưa đủ 18 tuổi không được phép tham gia.
    pub fn underage_restriction() -> String {
        SystemAlertReason::UnderageRestriction.to_hex()
    }

    /// 50 - Đăng nhập máy chủ thi đấu chuyên dụng.
    pub fn tournament_server_login() -> String {
        SystemAlertReason::TournamentServerLogin.to_hex()
    }

    /// 51 - Không thể đăng nhập máy chủ thi đấu chuyên dụng.
    pub fn cannot_login_tournament() -> String {
        SystemAlertReason::CannotLoginTournament51.to_hex()
    }

    /// 53 - Chưa đăng nhập vào máy chủ.
    pub fn not_logged_in_yet() -> String {
        SystemAlertReason::NotLoggedInYet.to_hex()
    }

    /// 54 - Sự kiện đấu trường đã kết thúc.
    pub fn arena_event_ended() -> String {
        SystemAlertReason::ArenaEventEnded.to_hex()
    }

    /// 55 - Mục tiêu lưu dữ liệu chuyển máy chủ không hợp lệ.
    pub fn migration_target_error() -> String {
        SystemAlertReason::MigrationTargetError.to_hex()
    }

    /// 56 - Máy chủ đang bận. Vui lòng chờ một lát.
    pub fn server_busy() -> String {
        SystemAlertReason::ServerBusy.to_hex()
    }

    /// 57 - Lý do không xác định.
    pub fn unknown_reason() -> String {
        SystemAlertReason::UnknownReason.to_hex()
    }
}
