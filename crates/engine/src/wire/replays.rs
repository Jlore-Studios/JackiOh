//! What a replay shows (SPEC §9.2, §9.3, §10.8, R768, issue #508): a finished game folded once,
//! then any seat's view at any step, a page at a time.
//!
//! `replay::replay_open` checks the game's log against its catalog version and its recorded final
//! hash and answers `ReplayOpen`; `replay::replay_page` reads steps out of the `ReplayCheckpoints`
//! it handed back. Step 0 is the state after `begin_game`, step k the state after the k-th action
//! `reduce` accepted. A step is `view_for(state_k, seat)` and nothing else (R97).

use serde::{Deserialize, Serialize};

use crate::state::GameState;
use crate::wire::actions::Action;
use crate::wire::string_union;
use crate::wire::view::PlayerView;

string_union! {
    /// R768: why a replay is refused, with no step given.
    pub enum ReplayRefusal {
        /// The game was played on another catalog version than this build's.
        EarlierPatch = "earlier_patch",
        /// The log folds to another final hash than the one recorded: a rule has changed since.
        RulesChanged = "rules_changed"
    }
}

/// R768: what a finished game recorded about itself, which its replay is held to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRecord {
    /// The catalog version the game was played on.
    pub catalog_version: String,
    /// `hash_state` of the game's final state.
    pub final_hash: String,
}

/// R768: an opened replay's folded log. `states[i]` is the state at step `i * REPLAY_CHECKPOINT_EVERY`
/// and `accepted` the log's accepted actions, so any step is at most `REPLAY_CHECKPOINT_EVERY`
/// `reduce` calls from a state held here.
///
/// It holds whole states, hidden cards included: only the server or the practice worker may hold
/// one, and what leaves them is a `ReplayStep`'s `view_for` (CLAUDE.md rule 7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ReplayCheckpoints {
    /// The log's actions `reduce` accepted, in order; a refused one is not here.
    pub accepted: Vec<Action>,
    /// The state every `REPLAY_CHECKPOINT_EVERY` accepted actions, step 0's first.
    pub states: Vec<GameState>,
}

/// R768: the answer to opening a replay.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ReplayOpen {
    /// The log folds to the recorded hash on this build's catalog: `steps` steps, `0..steps`.
    Ready {
        steps: usize,
        checkpoints: ReplayCheckpoints,
    },
    /// No step is given.
    Refused { reason: ReplayRefusal },
}

/// R768: one step of a replay as one seat saw it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ReplayStep {
    /// The step's index: 0 after `begin_game`, k after the k-th accepted action.
    pub step: usize,
    /// The step's state's turn.
    pub turn: i32,
    /// `view_for(state_k, seat)`.
    pub view: PlayerView,
}

/// R768: steps `[from, from + steps.len())` of a replay, and what reaching them cost.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPage {
    /// The first step asked for.
    pub from: usize,
    /// The steps, in order: at most `REPLAY_PAGE_STEPS`, fewer at the last step or when the page
    /// ended early at the `reduce` budget (the next page starts at `from + steps.len()`).
    pub steps: Vec<ReplayStep>,
    /// The `reduce` calls the page made, at most `REPLAY_CHECKPOINT_EVERY` (a pure crate keeps no
    /// counter, so the function reports it).
    pub reduces: usize,
}
