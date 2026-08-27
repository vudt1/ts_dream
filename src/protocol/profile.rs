//! Protocol dialect selection for the PC/aLogin and Kotlin/mobile contracts.

/// Main-opcode dialect used by a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolProfile {
    /// The supplied `aLogin.exe`/port-6414 contract.
    PcALogin,
    /// The Kotlin/mobile reference contract (used by replay fixtures).
    KotlinMobile,
}

/// PC meanings explicitly frozen by the operator and excluded from mobile
/// synchronization: SceneManage, Trade, NpcShop, and Guild.
pub const FROZEN_PC_COLLISIONS: [u8; 4] = [0x19, 0x1B, 0x1F, 0x23];

impl ProtocolProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PcALogin => "pc-aLogin",
            Self::KotlinMobile => "kotlin-mobile",
        }
    }

    pub const fn is_frozen_pc_collision(self, opcode: u8) -> bool {
        matches!(self, Self::PcALogin) && matches!(opcode, 0x19 | 0x1B | 0x1F | 0x23)
    }
}
