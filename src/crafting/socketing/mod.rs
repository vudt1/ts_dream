//! Socketing and feather enhancement module.

pub mod feather;
pub mod rate_table;

pub use feather::{parse_feather, socket_feather, FeatherInfo, SocketOutcome};
pub use rate_table::socket_success_rate;
