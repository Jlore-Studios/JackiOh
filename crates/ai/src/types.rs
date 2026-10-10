//! The AI's shared types (docs/polish/3-ai.md §Surface, SPEC §9.9). Every name here is exact: the
//! harness, the web worker and the tests write against these before the code behind them exists.
//!
//! `NodeCounter` is a trait whose methods take `&self`: one counter is shared by the lethal solver,
//! the beam, the replies and `decide` itself, a sub-counter holding its parent, so the counters keep
//! their tallies in `Cell`s (`simulate.rs`) and every function that spends nodes takes
//! `&dyn NodeCounter`. A counter that tallies failed simulations answers `Some` to
//! `sim_error_tally`, any other `None`.

use jackioh_engine::{ActionBody, Rng};
use serde::{Deserialize, Serialize};

/// A decision's budget. "Node" = one engine `reduce` call made by the AI, in any determinization.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SearchBudget {
    /// reduce() calls the whole decision may make, lethal solver and opponent auto-answers included.
    pub nodes: usize,
    /// Of `nodes`, the most the lethal solver may spend before the beam starts.
    pub lethal_nodes: usize,
    /// Determinizations sampled per decision (K).
    pub determinizations: usize,
    /// Open lines kept per depth.
    pub beam_width: usize,
    /// Children expanded at the root, after move ordering (endTurn always kept on top of this).
    pub root_branching: usize,
    /// Children expanded per open line below the root (endTurn always kept on top of this).
    pub branching: usize,
    /// Actions in one planned line, endTurn included.
    pub max_depth: usize,
    /// Best complete lines on determinization 0 that are re-scored on every other determinization.
    pub finalists: usize,
}

/// SURFACE §9: `decide`'s options; `budget` defaults to AI_BUDGET (`AiOptions::new`).
pub struct AiOptions<'a> {
    /// The AI's own stream; determinize is its only consumer. The caller reads `rng.cursor()` back
    /// afterwards.
    pub rng: Rng,
    pub budget: SearchBudget,
    /// Wall-clock safety cap, polled before every node; true = stop and answer with the best so far.
    /// The browser's clock (SURFACE §10.1): this crate never reads one.
    pub should_stop: Option<&'a dyn Fn() -> bool>,
}

impl<'a> AiOptions<'a> {
    /// The AI's stream at AI_BUDGET, with no clock.
    pub fn new(rng: Rng) -> AiOptions<'a> {
        AiOptions {
            rng,
            budget: crate::config::AI_BUDGET,
            should_stop: None,
        }
    }

    /// The AI's stream at `budget`, with no clock.
    pub fn with_budget(rng: Rng, budget: SearchBudget) -> AiOptions<'a> {
        AiOptions {
            rng,
            budget,
            should_stop: None,
        }
    }
}

/// Why `decide` answered as it did.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DecisionReason {
    #[serde(rename = "forced")]
    Forced,
    #[serde(rename = "mulligan")]
    Mulligan,
    #[serde(rename = "draw-offer")]
    DrawOffer,
    #[serde(rename = "lethal")]
    Lethal,
    #[serde(rename = "prompt")]
    Prompt,
    #[serde(rename = "search")]
    Search,
    #[serde(rename = "fallback")]
    Fallback,
}

impl DecisionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionReason::Forced => "forced",
            DecisionReason::Mulligan => "mulligan",
            DecisionReason::DrawOffer => "draw-offer",
            DecisionReason::Lethal => "lethal",
            DecisionReason::Prompt => "prompt",
            DecisionReason::Search => "search",
            DecisionReason::Fallback => "fallback",
        }
    }
}

impl std::fmt::Display for DecisionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `SearchStats["stoppedBy"]`: what ended a decision's search.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum StoppedBy {
    Exhausted,
    Budget,
    Clock,
}

impl StoppedBy {
    pub fn as_str(self) -> &'static str {
        match self {
            StoppedBy::Exhausted => "exhausted",
            StoppedBy::Budget => "budget",
            StoppedBy::Clock => "clock",
        }
    }
}

impl std::fmt::Display for StoppedBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchStats {
    pub nodes: usize,
    pub determinizations: usize,
    /// Complete lines scored on determinization 0.
    pub lines: usize,
    /// Simulated reduce calls that threw or were refused; never escape `decide`.
    pub sim_errors: usize,
    pub stopped_by: StoppedBy,
    /// Mean score of the chosen line across determinizations (0 for forced/mulligan/draw-offer).
    pub score: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub action: ActionBody,
    pub reason: DecisionReason,
    /// The planned line this action starts. Only line[0] is ever played; the AI re-plans after it.
    pub line: Vec<ActionBody>,
    pub stats: SearchStats,
}

/// Shared node accounting for one decision.
pub trait NodeCounter {
    fn used(&self) -> usize;
    fn limit(&self) -> usize;
    /// Polls shouldStop, then takes one node; false when the budget or the clock is spent.
    fn take(&self) -> bool;
    fn stopped_by(&self) -> StoppedBy;
    /// The failed simulations this counter tallies (`SearchStats.simErrors`), or `None` for a
    /// counter that tallies none.
    fn sim_error_tally(&self) -> Option<usize> {
        None
    }
    /// Sets that tally; a counter that tallies none ignores it.
    fn set_sim_error_tally(&self, _value: usize) {}
}
