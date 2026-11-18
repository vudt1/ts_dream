//! Consolidated special events, guild war, arena, quiz & web admin test suite.

mod common;

#[path = "events_arena/arena.rs"]
mod arena;
#[path = "events_arena/festival_battle_drops_and_exp.rs"]
mod festival_battle_drops_and_exp;
#[path = "events_arena/festival_gating_and_evacuation.rs"]
mod festival_gating_and_evacuation;
#[path = "events_arena/festival_login_and_encoder.rs"]
mod festival_login_and_encoder;
#[path = "events_arena/festival_quiz.rs"]
mod festival_quiz;
#[path = "events_arena/festival_wire_parity.rs"]
mod festival_wire_parity;
#[path = "events_arena/guild_war.rs"]
mod guild_war;
#[path = "events_arena/seasonal_festival_control.rs"]
mod seasonal_festival_control;
#[path = "events_arena/special_events_dungeon.rs"]
mod special_events_dungeon;
#[path = "events_arena/web_admin_events.rs"]
mod web_admin_events;
