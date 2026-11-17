//! Consolidated battle engine & battle protocol test suite.

mod common;

#[path = "battle/battle_kind_and_roles.rs"]
mod battle_kind_and_roles;
#[path = "battle/buff_debuff_lifecycle.rs"]
mod buff_debuff_lifecycle;
#[path = "battle/catch_verification.rs"]
mod catch_verification;
#[path = "battle/cell_frames.rs"]
mod cell_frames;
#[path = "battle/combo_rules.rs"]
mod combo_rules;
#[path = "battle/command_channel.rs"]
mod command_channel;
#[path = "battle/debuff_turn_skip.rs"]
mod debuff_turn_skip;
#[path = "battle/dispel_cleanse.rs"]
mod dispel_cleanse;
#[path = "battle/finish_generalization.rs"]
mod finish_generalization;
#[path = "battle/frames.rs"]
mod frames;
#[path = "battle/gold_frame.rs"]
mod gold_frame;
#[path = "battle/isometric_targeting.rs"]
mod isometric_targeting;
#[path = "battle/item_frame.rs"]
mod item_frame;
#[path = "battle/levelup_frames.rs"]
mod levelup_frames;
#[path = "battle/lifecycle.rs"]
mod lifecycle;
#[path = "battle/mineral_pickaxe.rs"]
mod mineral_pickaxe;
#[path = "battle/open_leader.rs"]
mod open_leader;
#[path = "battle/open_self_kind.rs"]
mod open_self_kind;
#[path = "battle/party_pvp_flee_matrix.rs"]
mod party_pvp_flee_matrix;
#[path = "battle/pvp_party_and_flee.rs"]
mod pvp_party_and_flee;
#[path = "battle/rng_determinism.rs"]
mod rng_determinism;
#[path = "battle/state7_frame.rs"]
mod state7_frame;
#[path = "battle/status_frames.rs"]
mod status_frames;
#[path = "battle/talk_notice.rs"]
mod talk_notice;
#[path = "battle/team_row.rs"]
mod team_row;
#[path = "battle/timing_wiring.rs"]
mod timing_wiring;
