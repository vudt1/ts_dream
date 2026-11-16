//! Consolidated Eve script, NPC talk, quest sync, door warp & random encounter test suite.

mod common;

#[path = "eve_quest/checkpoint6_parity.rs"]
mod checkpoint6_parity;
#[path = "eve_quest/door_triggers_walkstep_and_party_warp.rs"]
mod door_triggers_walkstep_and_party_warp;
#[path = "eve_quest/eve_battle_sync_registry_and_disconnect.rs"]
mod eve_battle_sync_registry_and_disconnect;
#[path = "eve_quest/eve_loader_and_default_on.rs"]
mod eve_loader_and_default_on;
#[path = "eve_quest/missing_eve_effect_executors.rs"]
mod missing_eve_effect_executors;
#[path = "eve_quest/npc_eve_e2e.rs"]
mod npc_eve_e2e;
#[path = "eve_quest/npc_eve_resolve.rs"]
mod npc_eve_resolve;
#[path = "eve_quest/npc_talk_multistep.rs"]
mod npc_talk_multistep;
#[path = "eve_quest/npc_talk_transmit.rs"]
mod npc_talk_transmit;
#[path = "eve_quest/p0_blockers.rs"]
mod p0_blockers;
#[path = "eve_quest/pure_planner_event_runner_and_preflight.rs"]
mod pure_planner_event_runner_and_preflight;
#[path = "eve_quest/quest_sync_18.rs"]
mod quest_sync_18;
#[path = "eve_quest/random_encounter.rs"]
mod random_encounter;
#[path = "eve_quest/snapshot_fail_closed_and_menu_filter.rs"]
mod snapshot_fail_closed_and_menu_filter;
#[path = "eve_quest/tickets_11_12_13.rs"]
mod tickets_11_12_13;
#[path = "eve_quest/tickets_14_15.rs"]
mod tickets_14_15;
#[path = "eve_quest/wire_codec_14_18.rs"]
mod wire_codec_14_18;
