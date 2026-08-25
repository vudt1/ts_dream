//! Integration tests for the 4-Tier Eve Script Engine (ticket 05).
//!
//! Independent suites per module: condition evaluator, AND-chain resolver,
//! GroupData probability filter, state builder and auto-chain engine.

use std::collections::{HashMap, HashSet};

use ts_dream::battle::rng::DotNetRandom;
use ts_dream::data::loaders::{
    EveCondition, EveGroupData, EveResult, EveSurfaceData, NpcEventData,
};
use ts_dream::eve::{AutoChainResult, EventPhase, EventSession, EveAutoChainEngine, MAX_CHAIN_DEPTH};
use ts_dream::eve::{compare_step, evaluate, apply_group_data};
use ts_dream::eve::{build_chains, resolve, resolve_event};
use ts_dream::eve::{EveStateBuilder, PlayerEventState, PlayerStateInputs};

// ── Shared helpers ──────────────────────────────────────────────────────────

fn result(type_: u8, mean_no: u16) -> EveResult {
    EveResult {
        result_group_no: 0,
        result_no: 1,
        result_type: type_,
        result_class: 0,
        parameter: 0,
        parameter_style: 0,
        result_value: 0,
        result_mean_no: mean_no,
    }
}

fn give_item_result(item_id: u16) -> EveResult {
    EveResult {
        result_type: 0,
        result_class: 1,
        parameter: item_id,
        parameter_style: 1,
        ..result(0, 1)
    }
}

fn cond(no: u8, cls: u8) -> EveCondition {
    EveCondition {
        condition_no: no,
        condition_class: cls,
        ..EveCondition::default()
    }
}

fn session(eve_no: i32, trigger_kind: i32, chain_depth: i32, matched_cond_no: i32) -> EventSession {
    EventSession {
        map_id: 10701,
        eve_no,
        npc_click_id: 1,
        trigger_kind,
        chain_depth,
        results: vec![result(1, 1)],
        current_index: 0,
        phase: EventPhase::Executing,
        last_surface_id: -1,
        last_choice_code: -1,
        battle_result: 0,
        matched_condition_no: matched_cond_no,
    }
}

fn event_data(eve_no: u16, conditions: Vec<EveCondition>) -> NpcEventData {
    NpcEventData {
        eve_no,
        when_happen: [false; 4],
        conditions,
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Tier 2 — EveConditionEvaluator
// ════════════════════════════════════════════════════════════════════════════

mod evaluator_tests {
    use super::*;

    fn eval_cls(cls: u8, param: u16, p_style: u8, ops: u8, value: i32, state: &PlayerEventState) -> bool {
        let mut c = cond(1, cls);
        c.condition_parameter = param;
        c.condition_parameter_style = p_style;
        c.condition_ops = ops;
        c.condition_value = value;
        evaluate(&c, state)
    }

    #[test]
    fn compare_step_all_seven_ops() {
        // ops=0 and 1 both mean equal in Eve convention.
        assert!(compare_step(5, 0, 5));
        assert!(!compare_step(5, 0, 3));
        assert!(compare_step(5, 1, 5));
        assert!(!compare_step(5, 1, 3));
        // less than
        assert!(compare_step(3, 2, 5));
        assert!(!compare_step(5, 2, 5));
        // less or equal
        assert!(compare_step(5, 3, 5));
        assert!(!compare_step(6, 3, 5));
        // greater
        assert!(compare_step(6, 4, 5));
        assert!(!compare_step(5, 4, 5));
        // greater or equal
        assert!(compare_step(5, 5, 5));
        assert!(!compare_step(4, 5, 5));
        // not equal
        assert!(compare_step(4, 6, 5));
        assert!(!compare_step(5, 6, 5));
    }

    #[test]
    fn class_0_unconditional_always_true() {
        assert!(evaluate(&cond(1, 0), &PlayerEventState::default()));
    }

    #[test]
    fn class_1_bag_item_reversed_comparison() {
        let state = PlayerEventState {
            bag_items: HashMap::from([(100, 3)]),
            ..PlayerEventState::default()
        };
        // val=0 ops=2 → 0 < qty → has item
        assert!(eval_cls(1, 100, 0, 2, 0, &state));
        // val=0 ops=5 → 0 >= qty fails when holding some
        assert!(!eval_cls(1, 100, 0, 5, 0, &state));
        // not held: qty defaults 0 → 0 >= 0 passes
        assert!(eval_cls(1, 999, 0, 5, 0, &state));
    }

    #[test]
    fn class_2_quest_steps() {
        let state = PlayerEventState {
            mission_steps: HashMap::from([(10, 2)]),
            mission_flags: HashMap::from([(10, 1)]),
            ..PlayerEventState::default()
        };
        // pStyle=1 reads step only: step=2 >= 1
        assert!(eval_cls(2, 10, 1, 5, 1, &state));
        // pStyle=3 takes max(step, flag): completed mission via flag
        let done = PlayerEventState {
            mission_flags: HashMap::from([(20, 1)]),
            ..PlayerEventState::default()
        };
        assert!(eval_cls(2, 20, 3, 5, 1, &done));
        // pStyle=3 unstarted mission: actual=0 == 0
        assert!(eval_cls(2, 99, 3, 1, 0, &PlayerEventState::default()));
    }

    #[test]
    fn class_7_player_attributes() {
        // param=0 level check — reversed like bag items: `value ops level`,
        // so a "level above T" gate is written ops=2 (<) with val=T.
        let veteran = PlayerEventState { level: 30, ..PlayerEventState::default() };
        assert!(eval_cls(7, 0, 0, 2, 24, &veteran));
        let novice = PlayerEventState { level: 10, ..PlayerEventState::default() };
        assert!(!eval_cls(7, 0, 0, 2, 24, &novice));

        // param=3 equipment possession (forward comparison)
        let equipped = PlayerEventState {
            equip_items: HashSet::from([500]),
            ..PlayerEventState::default()
        };
        assert!(eval_cls(7, 3, 0, 5, 500, &equipped));
        assert!(!eval_cls(7, 3, 0, 5, 500, &PlayerEventState::default()));

        // param=6 general storage slots by pStyle
        let pets = PlayerEventState {
            follow_npc_ids: HashSet::from([22081]),
            cart_npc_ids: HashSet::from([22082]),
            inn_npc_ids: HashSet::from([22083]),
            ..PlayerEventState::default()
        };
        assert!(eval_cls(7, 6, 1, 5, 22081, &pets));
        assert!(eval_cls(7, 6, 2, 5, 22082, &pets));
        assert!(eval_cls(7, 6, 3, 5, 22083, &pets));
        assert!(!eval_cls(7, 6, 1, 5, 22099, &pets));
        // Unknown pStyle inspects an empty set → possession never passes.
        assert!(!eval_cls(7, 6, 9, 5, 22081, &pets));
    }

    #[test]
    fn class_7_reborn_count_reversed() {
        let reborn = PlayerEventState { reborn_count: 2, ..PlayerEventState::default() };
        // "reborn more than once" reads `1 < count`.
        assert!(eval_cls(7, 2, 0, 2, 1, &reborn));
        // Threshold above actual count fails: `5 < 2` is false.
        assert!(!eval_cls(7, 2, 0, 2, 5, &reborn));
    }

    #[test]
    fn class_8_battle_results() {
        let won = PlayerEventState { battle_result: 1, ..PlayerEventState::default() };
        assert!(eval_cls(8, 0, 1, 0, 0, &won));
        assert!(!eval_cls(8, 0, 2, 0, 0, &won));
    }

    #[test]
    fn class_9_general_possession() {
        let owns = PlayerEventState {
            follow_npc_ids: HashSet::from([22081]),
            ..PlayerEventState::default()
        };
        assert!(eval_cls(9, 0, 0, 5, 22081, &owns));
        assert!(!eval_cls(9, 0, 0, 5, 22081, &PlayerEventState::default()));
    }

    #[test]
    fn class_10_dialog_choice() {
        let chose = PlayerEventState {
            last_surface_id: 100,
            last_choice_code: 30,
            ..PlayerEventState::default()
        };
        assert!(eval_cls(10, 100, 30, 0, 0, &chose));
        assert!(!eval_cls(10, 999, 30, 0, 0, &chose));
        assert!(!eval_cls(10, 100, 21, 0, 0, &chose));
    }

    #[test]
    fn class_12_completed_event_counts_forward() {
        // count=0 < 1 passes; count=1 < 1 fails
        let fresh = PlayerEventState {
            current_eve_no: 5,
            ..PlayerEventState::default()
        };
        assert!(eval_cls(12, 0, 0, 2, 1, &fresh));
        let once = PlayerEventState {
            current_eve_no: 5,
            completed_eve_counts: HashMap::from([(5, 1)]),
            ..PlayerEventState::default()
        };
        assert!(!eval_cls(12, 0, 0, 2, 1, &once));
        // equality: exactly 2 completions required
        let twice = PlayerEventState {
            current_eve_no: 1,
            completed_eve_counts: HashMap::from([(1, 2)]),
            ..PlayerEventState::default()
        };
        assert!(eval_cls(12, 0, 0, 1, 2, &twice));
        assert!(!eval_cls(12, 0, 0, 1, 2, &once));
    }

    #[test]
    fn class_14_role_counts() {
        let counted = PlayerEventState {
            role_count_values: HashMap::from([(100, 5)]),
            ..PlayerEventState::default()
        };
        assert!(eval_cls(14, 100, 0, 5, 3, &counted));
        // missing counter defaults to 0; 0 > 0 fails
        assert!(!eval_cls(14, 100, 0, 4, 0, &PlayerEventState::default()));
    }

    #[test]
    fn unknown_class_conservatively_false() {
        let mut c = cond(1, 99);
        c.and_num = 1;
        assert!(!evaluate(&c, &PlayerEventState::default()));
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Tier 3a — EveChainResolver
// ════════════════════════════════════════════════════════════════════════════

mod resolver_tests {
    use super::*;

    #[test]
    fn build_chains_single_conditions_independent() {
        let conditions = vec![cond(1, 0), cond(2, 0), cond(3, 0)];
        let chains = build_chains(&conditions);
        assert_eq!(chains.iter().map(|c| c.len()).collect::<Vec<_>>(), vec![1, 1, 1]);
    }

    #[test]
    fn build_chains_and_num_two_pairs() {
        let mut head = cond(1, 0);
        head.and_num = 2;
        let mut tail = cond(2, 0);
        tail.and_num = 0;
        let conditions = vec![head, tail, cond(3, 0)];
        let chains = build_chains(&conditions);
        assert_eq!(chains.len(), 2);
        assert_eq!(
            chains[0].iter().map(|c| c.condition_no).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(chains[1].len(), 1);
    }

    #[test]
    fn build_chains_and_num_three_groups() {
        let mut c1 = cond(1, 0);
        c1.and_num = 3;
        let mut c2 = cond(2, 0);
        c2.and_num = 0;
        let mut c3 = cond(3, 0);
        c3.and_num = 0;
        let conditions = vec![c1, c2, c3, cond(4, 0)];
        let chains = build_chains(&conditions);
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[0].len(), 3);
        assert_eq!(chains[1].len(), 1);
    }

    #[test]
    fn build_chains_empty_input() {
        assert!(build_chains(&[]).is_empty());
    }

    #[test]
    fn resolve_no_match_returns_none() {
        let mut c = cond(1, 2);
        c.condition_value = 5;
        c.results = vec![result(1, 1)];
        let resolved = resolve(&[c], &|_| false, None);
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_skips_head_without_results() {
        let bare = cond(1, 0);
        let mut with_results = cond(2, 0);
        with_results.results = vec![result(1, 99)];
        let resolved = resolve(&[bare, with_results.clone()], &|_| true, None).unwrap();
        assert_eq!(resolved.results, vec![result(1, 99)]);
        assert_eq!(resolved.matched_condition_no, 2);
    }

    #[test]
    fn resolve_and_chain_fails_when_one_condition_fails() {
        let mut head = cond(1, 0);
        head.and_num = 2;
        head.results = vec![result(1, 1)];
        let unknown_class = cond(2, 99);
        let resolved = resolve(
            &[head, unknown_class],
            &|c| c.condition_class != 99,
            None,
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_tier1_step_score_wins() {
        let mut low = cond(1, 2);
        low.condition_value = 1;
        low.results = vec![result(1, 10)];
        let mut high = cond(2, 2);
        high.condition_value = 3;
        high.results = vec![result(1, 20)];
        let resolved = resolve(&[low, high], &|_| true, None).unwrap();
        assert_eq!(resolved.results.first().unwrap().result_mean_no, 20);
        assert_eq!(resolved.matched_condition_no, 2);
        assert_eq!(resolved.step_score, 3);
    }

    #[test]
    fn resolve_tier2_chain_length_wins_on_equal_step_score() {
        let mut single = cond(1, 0);
        single.results = vec![result(1, 10)];
        let mut pair_head = cond(2, 0);
        pair_head.and_num = 2;
        pair_head.results = vec![result(1, 20)];
        let mut follower = cond(3, 0);
        follower.and_num = 0;
        let resolved = resolve(&[single, pair_head, follower], &|_| true, None).unwrap();
        assert_eq!(resolved.results.first().unwrap().result_mean_no, 20);
        assert_eq!(resolved.matched_condition_no, 2);
    }

    #[test]
    fn resolve_tier3_result_count_wins_on_full_tie() {
        let mut fewer = cond(1, 0);
        fewer.results = vec![result(1, 10)];
        let mut more = cond(2, 0);
        more.results = vec![result(1, 20), result(1, 21)];
        let resolved = resolve(&[fewer, more], &|_| true, None).unwrap();
        assert_eq!(resolved.results.len(), 2);
        assert_eq!(resolved.matched_condition_no, 2);
    }

    #[test]
    fn resolve_tier4_declaration_order_breaks_remaining_ties() {
        let mut first = cond(1, 0);
        first.results = vec![result(1, 10)];
        let mut second = cond(2, 0);
        second.results = vec![result(1, 20)];
        let resolved = resolve(&[first, second], &|_| true, None).unwrap();
        assert_eq!(resolved.results.first().unwrap().result_mean_no, 10);
        assert_eq!(resolved.matched_condition_no, 1);
    }

    #[test]
    fn chain_filter_restricts_to_matching_class() {
        let mut plain = cond(1, 0);
        plain.results = vec![result(1, 10)];
        let mut choice = cond(2, 10);
        choice.condition_parameter = 100;
        choice.condition_parameter_style = 30;
        choice.results = vec![result(1, 20)];

        let resolved = resolve(
            &[plain.clone(), choice.clone()],
            &|_| true,
            Some(&|chain| chain.iter().any(|c| c.condition_class == 10)),
        )
        .unwrap();
        assert_eq!(resolved.results.first().unwrap().result_mean_no, 20);
        assert_eq!(resolved.matched_condition_no, 2);

        // Filter excluding everything yields no match.
        assert!(resolve(
            &[plain.clone()],
            &|_| true,
            Some(&|chain| chain.iter().any(|c| c.condition_class == 8))
        )
        .is_none());
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Tier 4a — GroupData weighted random pick
// ════════════════════════════════════════════════════════════════════════════

mod group_tests {
    use super::*;

    fn grouped(group_no: u16, result_no: u8) -> EveResult {
        EveResult {
            result_group_no: group_no,
            result_no,
            ..result(0, 1)
        }
    }

    fn group(members: Vec<u8>, probs: Vec<u8>, pick: u8) -> EveGroupData {
        EveGroupData {
            eve_no: 1,
            event_no: 1,
            condition_no: 1,
            use_mode: 0,
            member_no_ay: members,
            probability_rate_ay: probs,
            pick_member: pick,
        }
    }

    #[test]
    fn all_initial_group_results_pass_through() {
        let results = vec![result(1, 1), result(1, 2)];
        let out = apply_group_data(results.clone(), &HashMap::new(), &mut DotNetRandom::new(1));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn pick_member_fixed_selection() {
        let datas = HashMap::from([(1u16, group(vec![1, 2], vec![90, 10], 2))]);
        let results = vec![grouped(1, 1), grouped(1, 2)];
        let out = apply_group_data(results, &datas, &mut DotNetRandom::new(7));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].result_no, 2);
    }

    #[test]
    fn single_member_selected_without_roll() {
        let datas = HashMap::from([(1u16, group(vec![3], vec![0], 0))]);
        let results = vec![grouped(1, 3)];
        let out = apply_group_data(results, &datas, &mut DotNetRandom::new(11));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].result_no, 3);
    }

    #[test]
    fn zero_weights_keep_every_member() {
        let datas = HashMap::from([(1u16, group(vec![1, 2], vec![0, 0], 0))]);
        let results = vec![grouped(1, 1), grouped(1, 2)];
        let out = apply_group_data(results, &datas, &mut DotNetRandom::new(3));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn missing_group_table_keeps_members() {
        let results = vec![grouped(9, 1), grouped(9, 2)];
        let out = apply_group_data(results, &HashMap::new(), &mut DotNetRandom::new(5));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn weighted_random_is_deterministic_per_seed_and_valid() {
        let datas = HashMap::from([(1u16, group(vec![1, 2], vec![60, 40], 0))]);
        let make_results = || vec![grouped(1, 1), grouped(1, 2)];

        // Same seed → identical outcome.
        let a = apply_group_data(make_results(), &datas, &mut DotNetRandom::new(42));
        let b = apply_group_data(make_results(), &datas, &mut DotNetRandom::new(42));
        assert_eq!(a, b);

        // Exactly one member survives every roll, across seeds.
        for seed in 0..64i32 {
            let out = apply_group_data(make_results(), &datas, &mut DotNetRandom::new(seed));
            assert_eq!(out.len(), 1, "seed {seed}");
            assert!(out[0].result_no == 1 || out[0].result_no == 2);
        }

        // Both faces appear over a wider sample.
        let mut seen = HashSet::new();
        for seed in 0..200i32 {
            let out = apply_group_data(make_results(), &datas, &mut DotNetRandom::new(seed));
            seen.insert(out[0].result_no);
        }
        assert!(seen.contains(&1) && seen.contains(&2));
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  State snapshot — EveStateBuilder
// ════════════════════════════════════════════════════════════════════════════

mod state_builder_tests {
    use super::*;
    use ts_dream::data::loaders::SceneEveData;

    fn inputs<'a>(
        missions: &'a [(i32, i32)],
        raw_flags: &'a [(i32, i32)],
        mark_defs: &'a HashMap<i32, i32>,
        bag_slots: &'a [(i32, i32)],
    ) -> PlayerStateInputs<'a> {
        PlayerStateInputs {
            missions,
            raw_flags,
            mark_defs,
            bag_slots,
            equips: &[],
            level: 25,
            reborn_count: 0,
            last_surface_id: -1,
            last_choice_code: -1,
            battle_result: 0,
            completed_eve_counts: HashMap::new(),
            follow_npc_ids: &[],
            inn_npc_ids: &[],
            cart_npc_ids: &[],
        }
    }

    #[test]
    fn bag_quantities_merge_across_slots() {
        let bag = [(100, 2), (100, 3), (200, 1)];
        let state = EveStateBuilder::build_player_state(&inputs(&[], &[], &HashMap::new(), &bag));
        assert_eq!(state.bag_items.get(&100), Some(&5));
        assert_eq!(state.bag_items.get(&200), Some(&1));
        assert_eq!(state.level, 25);
        assert_eq!(state.last_choice_code, -1);
    }

    #[test]
    fn mission_flags_projected_through_mark_defs_only_when_positive() {
        let missions = [(10, 2)];
        let flags = [(7, 1), (8, 0)];
        let marks = HashMap::from([(10, 7), (20, 8)]);
        let state = EveStateBuilder::build_player_state(&inputs(&missions, &flags, &marks, &[]));
        assert_eq!(state.mission_steps.get(&10), Some(&2));
        assert_eq!(state.mission_flags.get(&10), Some(&1));
        // bitId 8 exists but value 0 → not projected onto mission 20.
        assert!(!state.mission_flags.contains_key(&20));
    }

    #[test]
    fn find_fallback_talk_first_talk_with_meaningful_id() {
        let mut silent = cond(1, 0);
        silent.results = vec![result(1, 0)]; // meanNo=0 → ignored
        let mut talky = cond(2, 0);
        talky.results = vec![result(1, 77)];
        let scene = SceneEveData {
            npc_events: HashMap::from([
                (1u16, event_data(1, vec![silent])),
                (2u16, event_data(2, vec![talky])),
            ]),
            ..SceneEveData::default()
        };
        let fallback = EveStateBuilder::find_fallback_talk(&scene, &[1, 2]).unwrap();
        assert_eq!(fallback.result_mean_no, 77);
        // No events at all → none.
        assert!(EveStateBuilder::find_fallback_talk(&SceneEveData::default(), &[9]).is_none());
    }

    #[test]
    fn distance_gate_rules() {
        // Origin player bypasses validation (still initializing).
        assert!(EveStateBuilder::is_within_range(0, 0, 999, 999));
        assert!(EveStateBuilder::is_within_range(100, 100, 120, 120));
        // Exactly 200 px passes; 201 px fails.
        assert!(EveStateBuilder::is_within_range(100, 100, 300, 100));
        assert!(!EveStateBuilder::is_within_range(100, 100, 301, 100));
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Auto-chain engine
// ════════════════════════════════════════════════════════════════════════════

mod auto_chain_tests {
    use super::*;
    use ts_dream::data::loaders::{EveDoorPlacement, EveNpcPlacement, SceneEveData};

    fn surface_result(surface_id: u16) -> EveResult {
        EveResult {
            result_type: 6,
            result_mean_no: surface_id,
            ..result(0, 1)
        }
    }

    fn battle_result(mean_no: u16) -> EveResult {
        EveResult {
            result_type: 3,
            result_mean_no: mean_no,
            ..result(0, 1)
        }
    }

    #[test]
    fn should_attempt_auto_chain_rules() {
        // ClickNpc + eve>0 + depth<cap → allowed
        assert!(EveAutoChainEngine::should_attempt_auto_chain(&session(4, 1, 0, 0)));
        // MeetDoor allowed
        assert!(EveAutoChainEngine::should_attempt_auto_chain(&session(2, 8, 5, 0)));
        // fallback Talk (eve=0) never chains
        assert!(!EveAutoChainEngine::should_attempt_auto_chain(&session(0, 1, 0, 0)));
        // depth cap reached
        assert!(!EveAutoChainEngine::should_attempt_auto_chain(&session(
            4,
            1,
            MAX_CHAIN_DEPTH,
            0
        )));
        // unsupported trigger kind
        assert!(!EveAutoChainEngine::should_attempt_auto_chain(&session(4, 99, 0, 0)));
    }

    #[test]
    fn skip_event_completion_watermark() {
        let events = [1u8, 2, 3];

        // Completed once without cls=12 self-management → skip.
        let done = PlayerEventState {
            completed_eve_counts: HashMap::from([(1, 1)]),
            ..PlayerEventState::default()
        };
        assert!(EveAutoChainEngine::should_skip_event(&events, 1, &event_data(1, vec![cond(1, 0)]), &done));

        // Completed but cls=12 present → still evaluated.
        assert!(!EveAutoChainEngine::should_skip_event(
            &events,
            1,
            &event_data(1, vec![cond(1, 12)]),
            &done
        ));

        // Not completed but a later sibling already finished (watermark).
        let later_done = PlayerEventState {
            completed_eve_counts: HashMap::from([(2, 1)]),
            ..PlayerEventState::default()
        };
        assert!(EveAutoChainEngine::should_skip_event(&events, 1, &event_data(1, vec![cond(1, 0)]), &later_done));

        // Later sibling unfinished → keep evaluating.
        let earlier_done = PlayerEventState {
            completed_eve_counts: HashMap::from([(1, 1)]),
            ..PlayerEventState::default()
        };
        assert!(!EveAutoChainEngine::should_skip_event(
            &events,
            2,
            &event_data(2, vec![cond(1, 0)]),
            &earlier_done
        ));
    }

    #[test]
    fn guard_1_same_condition_chain_detection() {
        assert!(EveAutoChainEngine::detect_same_chain(
            &session(1, 1, 0, 0),
            &session(1, 1, 0, 0)
        ));
        assert!(EveAutoChainEngine::detect_same_chain(
            &session(1, 1, 0, 1),
            &session(1, 1, 0, 1)
        ));
        assert!(!EveAutoChainEngine::detect_same_chain(
            &session(1, 1, 0, 0),
            &session(1, 1, 0, 1)
        ));
    }

    #[test]
    fn guard_2_re_question_detection() {
        let new_with_surface = EventSession {
            results: vec![surface_result(5)],
            ..session(1, 1, 0, 0)
        };
        let answered = EventSession {
            last_choice_code: 20,
            ..session(1, 1, 0, 0)
        };
        assert!(EveAutoChainEngine::detect_re_question(&new_with_surface, &answered));
        let unanswered = EventSession {
            last_choice_code: -1,
            ..session(1, 1, 0, 0)
        };
        assert!(!EveAutoChainEngine::detect_re_question(&new_with_surface, &unanswered));
    }

    #[test]
    fn guard_3_re_battle_detection() {
        let new_with_battle = EventSession {
            results: vec![battle_result(9)],
            ..session(1, 1, 0, 0)
        };
        let fought = EventSession {
            battle_result: 1,
            ..session(1, 1, 0, 0)
        };
        assert!(EveAutoChainEngine::detect_re_battle(&new_with_battle, &fought));
        let untouched = EventSession {
            battle_result: 0,
            ..session(1, 1, 0, 0)
        };
        assert!(!EveAutoChainEngine::detect_re_battle(&new_with_battle, &untouched));
    }

    #[test]
    fn guard_4_duplicate_item_detection() {
        let gives_a = EventSession {
            results: vec![give_item_result(100)],
            ..session(1, 1, 0, 0)
        };
        let gives_a_again = gives_a.clone();
        let gives_b = EventSession {
            results: vec![give_item_result(200)],
            ..session(1, 1, 0, 0)
        };
        let gives_nothing = session(1, 1, 0, 0);

        assert!(EveAutoChainEngine::detect_duplicate_items(&gives_a_again, &gives_a));
        assert!(!EveAutoChainEngine::detect_duplicate_items(&gives_b, &gives_a));
        assert!(!EveAutoChainEngine::detect_duplicate_items(&gives_a_again, &gives_nothing));
    }

    /// Full flow: NPC owns events [1, 2]. Event 1 is finished (no cls=12) so
    /// it is skipped; event 2 matches and chains at depth+1.
    #[test]
    fn try_auto_chain_advances_to_next_event() {
        let mut finished = cond(1, 0);
        finished.results = vec![result(1, 10)];
        let mut next_head = cond(2, 0);
        next_head.results = vec![result(1, 20)];
        let scene = SceneEveData {
            npcs: HashMap::from([(
                1u16,
                EveNpcPlacement {
                    id: 1,
                    npc_id: 900,
                    events: vec![1, 2],
                    ..EveNpcPlacement::default()
                },
            )]),
            npc_events: HashMap::from([
                (1u16, event_data(1, vec![finished])),
                (2u16, event_data(2, vec![next_head])),
            ]),
            ..SceneEveData::default()
        };

        let completed = EventSession {
            eve_no: 1,
            results: vec![result(1, 10)],
            ..session(1, 1, 0, 1)
        };
        let state = PlayerEventState {
            completed_eve_counts: HashMap::from([(1, 1)]),
            ..PlayerEventState::default()
        };

        match EveAutoChainEngine::try_auto_chain(&scene, &completed, &state, &mut DotNetRandom::new(1)) {
            AutoChainResult::Chained(candidate) => {
                assert_eq!(candidate.eve_no, 2);
                assert_eq!(candidate.chain_depth, 1);
                assert_eq!(candidate.matched_condition_no, 2);
                assert_eq!(candidate.results.first().unwrap().result_mean_no, 20);
                assert_eq!(candidate.phase, EventPhase::Executing);
                assert_eq!(candidate.npc_click_id, 1);
            }
            AutoChainResult::NoMatch => panic!("expected chain into event 2"),
        }
    }

    /// Same-eveNo loop: re-matching the exact same condition chain terminates
    /// the chain attempt instead of looping forever.
    #[test]
    fn try_auto_chain_same_chain_loop_terminates() {
        let mut unconditional = cond(1, 0);
        unconditional.results = vec![result(1, 10)];
        let scene = SceneEveData {
            npcs: HashMap::from([(
                1u16,
                EveNpcPlacement {
                    id: 1,
                    events: vec![1],
                    ..EveNpcPlacement::default()
                },
            )]),
            npc_events: HashMap::from([(1u16, event_data(1, vec![unconditional]))]),
            ..SceneEveData::default()
        };

        let completed = EventSession {
            eve_no: 1,
            results: vec![result(1, 10)],
            ..session(1, 1, 0, 1)
        };
        // No completion record: cls=0 event stays eligible, then guard #1 fires.
        let state = PlayerEventState::default();

        assert!(matches!(
            EveAutoChainEngine::try_auto_chain(&scene, &completed, &state, &mut DotNetRandom::new(1)),
            AutoChainResult::NoMatch
        ));
    }

    /// Same-eveNo chaining is legal when a *different* condition chain wins
    /// (tutorial flows): guard #1 passes and the candidate is produced.
    #[test]
    fn try_auto_chain_same_eve_different_chain_allowed() {
        let mut chain_a = cond(1, 2);
        chain_a.condition_parameter = 1; // missionId 1
        chain_a.condition_value = 1; // step >= 1
        chain_a.results = vec![result(1, 10)];
        let mut chain_b = cond(2, 2);
        chain_b.condition_parameter = 1; // missionId 1
        chain_b.condition_value = 3; // higher step threshold
        chain_b.results = vec![result(1, 30)];
        let scene = SceneEveData {
            npcs: HashMap::from([(
                1u16,
                EveNpcPlacement {
                    id: 1,
                    events: vec![1],
                    ..EveNpcPlacement::default()
                },
            )]),
            npc_events: HashMap::from([(1u16, event_data(1, vec![chain_a, chain_b]))]),
            ..SceneEveData::default()
        };

        let completed = EventSession {
            eve_no: 1,
            results: vec![result(1, 10)],
            ..session(1, 1, 0, 1) // matched chain A
        };
        let state = PlayerEventState {
            mission_steps: HashMap::from([(1, 3)]), // step 3 satisfies both chains
            ..PlayerEventState::default()
        };

        match EveAutoChainEngine::try_auto_chain(&scene, &completed, &state, &mut DotNetRandom::new(1)) {
            AutoChainResult::Chained(candidate) => {
                assert_eq!(candidate.eve_no, 1);
                assert_eq!(candidate.matched_condition_no, 2, "higher step chain wins");
                assert_eq!(candidate.chain_depth, 1);
            }
            AutoChainResult::NoMatch => panic!("different chain must be allowed"),
        }
    }

    /// Door trigger resolves through the door event list.
    #[test]
    fn try_auto_chain_resolves_door_trigger() {
        let mut door_head = cond(1, 0);
        door_head.results = vec![result(2, 5)];
        let scene = SceneEveData {
            doors: HashMap::from([(
                7u16,
                EveDoorPlacement {
                    id: 7,
                    events: vec![3],
                    ..EveDoorPlacement::default()
                },
            )]),
            npc_events: HashMap::from([(3u16, event_data(3, vec![door_head]))]),
            ..SceneEveData::default()
        };
        let completed = EventSession {
            eve_no: 9,
            trigger_kind: 4,
            npc_click_id: 7,
            results: vec![result(2, 1)],
            ..session(9, 4, 0, 0)
        };
        match EveAutoChainEngine::try_auto_chain(&scene, &completed, &PlayerEventState::default(), &mut DotNetRandom::new(1)) {
            AutoChainResult::Chained(candidate) => {
                assert_eq!(candidate.eve_no, 3);
                assert_eq!(candidate.trigger_kind, 4);
            }
            AutoChainResult::NoMatch => panic!("door chain expected"),
        }
    }

    /// Surface re-question guard blocks chaining inside try_auto_chain.
    #[test]
    fn try_auto_chain_blocks_re_question_after_choice() {
        let mut question = cond(1, 10);
        question.condition_parameter = 100;
        question.condition_parameter_style = 30;
        question.results = vec![surface_result(100)];
        let scene = SceneEveData {
            npcs: HashMap::from([(
                1u16,
                EveNpcPlacement {
                    id: 1,
                    events: vec![1],
                    ..EveNpcPlacement::default()
                },
            )]),
            npc_events: HashMap::from([(1u16, event_data(1, vec![question]))]),
            ..SceneEveData::default()
        };

        let answered_session = EventSession {
            last_choice_code: 30,
            last_surface_id: 100,
            ..session(1, 1, 0, 1)
        };
        let state = PlayerEventState {
            last_surface_id: 100,
            last_choice_code: 30,
            ..PlayerEventState::default()
        };

        assert!(matches!(
            EveAutoChainEngine::try_auto_chain(&scene, &answered_session, &state, &mut DotNetRandom::new(1)),
            AutoChainResult::NoMatch
        ));
    }

    /// resolve_event end-to-end: cls=12 gating plus GroupData filtering.
    #[test]
    fn resolve_event_applies_count_gate_and_group_filter() {
        // Chain: never completed (count < 1) → grouped rewards.
        let mut gate = cond(1, 12);
        gate.condition_parameter = 0;
        gate.condition_ops = 2; // count < 1
        gate.condition_value = 1;
        gate.results = vec![
            EveResult { result_group_no: 1, result_no: 1, ..give_item_result(100) },
            EveResult { result_group_no: 1, result_no: 2, ..give_item_result(200) },
        ];
        let event = event_data(4, vec![gate]);

        let group = EveGroupData {
            member_no_ay: vec![1, 2],
            probability_rate_ay: vec![50, 50],
            pick_member: 2,
            ..EveGroupData::default()
        };
        let group_datas = HashMap::from([(1u16, group)]);

        let resolved = resolve_event(
            &event,
            &PlayerEventState::default(),
            &group_datas,
            &mut DotNetRandom::new(9),
            None,
        )
        .unwrap();

        // Fixed pick_member=2 keeps only reward item 200.
        assert_eq!(resolved.results.len(), 1);
        assert_eq!(resolved.results[0].parameter, 200);
        assert_eq!(resolved.matched_condition_no, 1);

        // After one completion the gate (count < 1) fails → no match.
        let completed_once = PlayerEventState {
            current_eve_no: 4,
            completed_eve_counts: HashMap::from([(4, 1)]),
            ..PlayerEventState::default()
        };
        assert!(resolve_event(
            &event,
            &completed_once,
            &group_datas,
            &mut DotNetRandom::new(9),
            None,
        )
        .is_none());
    }

    #[test]
    fn surface_data_roundtrip_in_scene_context() {
        // Sanity: surface tables coexist with engine data structures.
        let surface = EveSurfaceData {
            id: 100,
            sentence_count: 1,
            option_index: 0,
            option_count: 2,
            option_mode: 0,
            sentences: HashMap::new(),
        };
        assert_eq!(surface.id, 100);
    }
}
