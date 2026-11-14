//! Pure Eve Planner — zero I/O, zero Tokio locks.
//!
//! Translates resolved `EveResult`s (after `GroupData` filtering) into typed
//! [`DomainEffect`]s so the `EventRunner` can run `Pre-Flight Probe` and apply
//! state mutations + wire syncs in a single unified state machine.

use std::collections::HashMap;

use crate::battle::rng::DotNetRandom;
use crate::data::loaders::{EveGroupData, EveResult, NpcEventData};
use crate::eve::resolver::{resolve_event, ChainFilter};
use crate::eve::state::PlayerEventState;

/// Typed Eve trigger input for the Pure Planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EveTrigger {
    ClickNpc(i32),
    ClickDoor(i32),
    MeetDoor(i32),
    WalkStep,
    MenuChoice { surface_id: i32, choice_code: u8 },
    BattleEnd { eve_no: i32, fight_eve_no: i32, outcome: i32 },
}

/// Pure domain effect produced from an `EveResult`.
///
/// Only `TalkStep` (`rt=1`) and `AwaitMenuChoice` (`rt=6`) emit `S→C 0x14 01`.
/// `result_type >= 7` (except `rt=9` server-side `NpcAction`) are never sent
/// raw via `0x14` and instead become [`DomainEffect::Unsupported`].
#[derive(Debug, Clone, PartialEq)]
pub enum DomainEffect {
    /// `rt = 1`: Deliver a dialogue step (`0x14 01`) and wait for `0x14 06`.
    TalkStep(EveResult),
    /// `rt = 6`: Deliver a `TSurface` menu (`0x14 01`) and wait for `0x14 09`.
    AwaitMenuChoice { surface_id: i32, result: EveResult },
    /// `rt = 2`: Door/warp relocation (`Warps.dat` row `(map_id, warp_id)`).
    Warp { warp_id: u16 },
    /// `rt = 3`: Scripted Eve battle (`scene.fight_datas[fight_id]`).
    StartBattle { fight_id: u16 },
    /// `rt = 4`: Move NPC/entity on map (Ticket 06).
    MoveEntity(EveResult),
    /// `rt = 5`: Play movie/animation (Ticket 06).
    PlayMovie(EveResult),
    /// `rt = 9`: Server-side NPC action (Ticket 06).
    NpcAction(EveResult),
    /// `rt = 0, rc = 1`: Add or remove `Item.dat` item in `homdo (0x17)`.
    MutateItem {
        item_id: u16,
        p_style: u8,
        delta: i32,
    },
    /// `rt = 0, rc = 2`: Update `Quest Ledger (0x18)` or `quest_dont (0x18 05)`.
    UpdateQuest {
        quest_id: u16,
        p_style: u8,
        result_value: i32,
    },
    /// `rt = 0, rc = 3`: NpcTeam action (Ticket 06).
    NpcTeam(EveResult),
    /// `rt = 0, rc = 4`: Grant skill free of `skill_point` + `08 01 6E` sync.
    LearnSkill { skill_id: u16, target_level: u8 },
    /// `rt = 0, rc = 5`: Gold / stat-point reward or deduction.
    MutateMoney { p_style: u8, amount: i32 },
    /// `rt = 0, rc = 7`: Player attribute / `savemap` / `texp` / shop open.
    PlayerEffect {
        parameter: u16,
        p_style: u8,
        result_value: i32,
    },
    /// `rt = 0, rc = 8`: Add or remove pet (Ticket 06).
    RewardPet(EveResult),
    /// Unverified or server-only unknown `result_type` / `result_class` (fail-closed no-op).
    Unsupported { result_type: u8, result_class: u8 },
}

impl DomainEffect {
    /// Whether this effect suspends or terminates the current step loop.
    #[must_use]
    pub fn is_blocking_or_terminal(&self) -> bool {
        matches!(
            self,
            Self::TalkStep(_)
                | Self::AwaitMenuChoice { .. }
                | Self::Warp { .. }
                | Self::StartBattle { .. }
        )
    }
}

/// Output of planning a single `NpcEventData` chain.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedChain {
    pub eve_no: u16,
    pub matched_condition_no: i32,
    pub results: Vec<EveResult>,
    pub effects: Vec<DomainEffect>,
}

/// Translate one `EveResult` into a typed [`DomainEffect`].
#[must_use]
pub fn plan_result(result: &EveResult) -> DomainEffect {
    match result.result_type {
        1 => DomainEffect::TalkStep(result.clone()),
        6 => DomainEffect::AwaitMenuChoice {
            surface_id: i32::from(result.result_mean_no),
            result: result.clone(),
        },
        2 => DomainEffect::Warp {
            warp_id: result.parameter,
        },
        3 => DomainEffect::StartBattle {
            fight_id: result.result_mean_no,
        },
        4 => DomainEffect::MoveEntity(result.clone()),
        5 => DomainEffect::PlayMovie(result.clone()),
        9 => DomainEffect::NpcAction(result.clone()),
        0 => match result.result_class {
            1 => DomainEffect::MutateItem {
                item_id: result.parameter,
                p_style: result.parameter_style,
                delta: result.result_value,
            },
            2 => DomainEffect::UpdateQuest {
                quest_id: result.parameter,
                p_style: result.parameter_style,
                result_value: result.result_value,
            },
            3 => DomainEffect::NpcTeam(result.clone()),
            4 => {
                let target_level = if result.result_value <= 0 {
                    1
                } else {
                    result.result_value.min(255) as u8
                };
                DomainEffect::LearnSkill {
                    skill_id: result.parameter,
                    target_level,
                }
            }
            5 => DomainEffect::MutateMoney {
                p_style: result.parameter_style,
                amount: result.result_value,
            },
            7 => DomainEffect::PlayerEffect {
                parameter: result.parameter,
                p_style: result.parameter_style,
                result_value: result.result_value,
            },
            8 => DomainEffect::RewardPet(result.clone()),
            other => DomainEffect::Unsupported {
                result_type: 0,
                result_class: other,
            },
        },
        // result_type >= 7 (e.g. 7, 8, 10) are never sent raw via 0x14.
        other => DomainEffect::Unsupported {
            result_type: other,
            result_class: result.result_class,
        },
    }
}

/// Translate a `GroupData`-filtered slice of `EveResult`s into `Vec<DomainEffect>`.
#[must_use]
pub fn plan_results(results: &[EveResult]) -> Vec<DomainEffect> {
    results.iter().map(plan_result).collect()
}

/// Pure planning entry point: resolve the winning AND chain for `event_data`,
/// apply `GroupData` probability selection, and translate into `Vec<DomainEffect>`.
pub fn plan_chain(
    event_data: &NpcEventData,
    state: &PlayerEventState,
    group_datas: &HashMap<u16, EveGroupData>,
    rng: &mut DotNetRandom,
    chain_filter: Option<ChainFilter<'_>>,
) -> Option<PlannedChain> {
    let resolved = resolve_event(event_data, state, group_datas, rng, chain_filter)?;
    let effects = plan_results(&resolved.results);
    Some(PlannedChain {
        eve_no: event_data.eve_no,
        matched_condition_no: resolved.matched_condition_no,
        results: resolved.results,
        effects,
    })
}
