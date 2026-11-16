//! Consolidated server lifecycle, socket hardening, auth, movement & chat test suite.

mod common;

#[path = "server_network/auto_save.rs"]
mod auto_save;
#[path = "server_network/chat.rs"]
mod chat;
#[path = "server_network/code_review_remediation.rs"]
mod code_review_remediation;
#[path = "server_network/create_char_atomic.rs"]
mod create_char_atomic;
#[path = "server_network/double_login.rs"]
mod double_login;
#[path = "server_network/idle_timeout.rs"]
mod idle_timeout;
#[path = "server_network/kick_reasons.rs"]
mod kick_reasons;
#[path = "server_network/liveness.rs"]
mod liveness;
#[path = "server_network/login_char_flow.rs"]
mod login_char_flow;
#[path = "server_network/movement_warp.rs"]
mod movement_warp;
#[path = "server_network/p0_p1_p2_roadmap.rs"]
mod p0_p1_p2_roadmap;
#[path = "server_network/relogin_flags.rs"]
mod relogin_flags;
#[path = "server_network/relogin_stats_items.rs"]
mod relogin_stats_items;
#[path = "server_network/socket_hardening.rs"]
mod socket_hardening;
#[path = "server_network/wiring_hotkey_notice.rs"]
mod wiring_hotkey_notice;
