//! Simulation through the real reducer (docs/polish/3-ai.md, SPEC §9.9). Every `reduce` call the AI
//! makes goes through `simulate` and costs one node of the decision's counter, the opponent's
//! auto-answers included, so budgets are counted in engine calls and a decision is deterministic.
//!
//! Port of `packages/ai/src/simulate.ts`. A TS counter is one object every part of a decision shares
//! (a sub-counter holds its parent while the parent is spent elsewhere too), so the counters keep
//! their tallies in `Cell`s and are passed as `&dyn NodeCounter` (`types.rs`). TS's `try`/`catch`
//! around the reducer is `catch_unwind`: an engine invariant that panics where TS threw is a failed
//! simulation, never a crash of the decision.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::{
    Action, ActionBody, GameState, PLAYER_IDS, Phase, PlayerId, active_units_of, legal_actions, opponent_of, reduce,
};
use serde::{Deserialize, Serialize};

use crate::config::{AI_EVAL, AI_SEARCH};
use crate::evaluate::{NextSwing, evaluate};
use crate::types::{NodeCounter, StoppedBy};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum LineStatus {
    Open,
    Passed,
    Yielded,
    Over,
}

impl LineStatus {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            LineStatus::Open => "open",
            LineStatus::Passed => "passed",
            LineStatus::Yielded => "yielded",
            LineStatus::Over => "over",
        }
    }
}

/// What a `CountingNodeCounter` is: a root counter of its own, or a slice of a parent.
enum CounterKind<'a> {
    Root {
        used: Cell<usize>,
        limit: usize,
        should_stop: Option<&'a dyn Fn() -> bool>,
        stopped_by: Cell<StoppedBy>,
        sim_errors: Cell<usize>,
    },
    Sub {
        parent: &'a dyn NodeCounter,
        start: usize,
        limit: usize,
        capped: Cell<bool>,
    },
}

/// A NodeCounter that also tallies failed simulations for `SearchStats.simErrors`.
pub struct CountingNodeCounter<'a> {
    kind: CounterKind<'a>,
}

impl CountingNodeCounter<'_> {
    /// TS `counter.simErrors`: a sub-counter reads its parent's tally (0 when the parent keeps none).
    pub fn sim_errors(&self) -> usize {
        match &self.kind {
            CounterKind::Root { sim_errors, .. } => sim_errors.get(),
            CounterKind::Sub { parent, .. } => parent.sim_error_tally().unwrap_or(0),
        }
    }

    /// TS `counter.simErrors = value`: a sub-counter writes its parent's tally, when it keeps one.
    pub fn set_sim_errors(&self, value: usize) {
        match &self.kind {
            CounterKind::Root { sim_errors, .. } => sim_errors.set(value),
            CounterKind::Sub { parent, .. } => {
                if parent.sim_error_tally().is_some() {
                    parent.set_sim_error_tally(value);
                }
            }
        }
    }
}

impl NodeCounter for CountingNodeCounter<'_> {
    fn used(&self) -> usize {
        match &self.kind {
            CounterKind::Root { used, .. } => used.get(),
            CounterKind::Sub { parent, .. } => parent.used(),
        }
    }

    fn limit(&self) -> usize {
        match &self.kind {
            CounterKind::Root { limit, .. } => *limit,
            CounterKind::Sub {
                parent, start, limit, ..
            } => parent.limit().min(start + *limit),
        }
    }

    fn take(&self) -> bool {
        match &self.kind {
            CounterKind::Root {
                used,
                limit,
                should_stop,
                stopped_by,
                ..
            } => {
                if stopped_by.get() != StoppedBy::Exhausted {
                    return false;
                }
                if let Some(should_stop) = should_stop
                    && should_stop()
                {
                    stopped_by.set(StoppedBy::Clock);
                    return false;
                }
                if used.get() >= *limit {
                    stopped_by.set(StoppedBy::Budget);
                    return false;
                }
                used.set(used.get() + 1);
                true
            }
            CounterKind::Sub {
                parent,
                start,
                limit,
                capped,
            } => {
                if parent.used().saturating_sub(*start) >= *limit {
                    capped.set(true);
                    return false;
                }
                parent.take()
            }
        }
    }

    fn stopped_by(&self) -> StoppedBy {
        match &self.kind {
            CounterKind::Root { stopped_by, .. } => stopped_by.get(),
            CounterKind::Sub { parent, capped, .. } => {
                if parent.stopped_by() != StoppedBy::Exhausted {
                    return parent.stopped_by();
                }
                if capped.get() {
                    StoppedBy::Budget
                } else {
                    StoppedBy::Exhausted
                }
            }
        }
    }

    fn sim_error_tally(&self) -> Option<usize> {
        Some(self.sim_errors())
    }

    fn set_sim_error_tally(&self, value: usize) {
        self.set_sim_errors(value);
    }
}

pub fn create_node_counter<'a>(limit: usize, should_stop: Option<&'a dyn Fn() -> bool>) -> CountingNodeCounter<'a> {
    CountingNodeCounter {
        kind: CounterKind::Root {
            used: Cell::new(0),
            limit,
            should_stop,
            stopped_by: Cell::new(StoppedBy::Exhausted),
            sim_errors: Cell::new(0),
        },
    }
}

/// A slice of `parent`: at most `limit` more nodes, drawn from the parent. Running out of the slice
/// does not stop the parent, so the lethal solver can hand what it left over to the beam.
pub fn create_sub_counter<'a>(parent: &'a dyn NodeCounter, limit: usize) -> CountingNodeCounter<'a> {
    CountingNodeCounter {
        kind: CounterKind::Sub {
            parent,
            start: parent.used(),
            limit,
            capped: Cell::new(false),
        },
    }
}

fn note_sim_error(counter: &dyn NodeCounter) {
    if let Some(errors) = counter.sim_error_tally() {
        counter.set_sim_error_tally(errors + 1);
    }
}

/// TS `error instanceof Error ? error.message : String(error)`, for a panic's payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "panic".to_string()
    }
}

/// `seat`'s view of a state during a line that started on `rootTurn`: "over" if state.result is set;
/// "passed" if state.turn !== rootTurn; "yielded" if no prompt is open and state.active !== seat;
/// otherwise "open".
pub fn line_status(state: &GameState, seat: PlayerId, root_turn: i32) -> LineStatus {
    if state.result.is_some() {
        return LineStatus::Over;
    }
    if state.turn != root_turn {
        return LineStatus::Passed;
    }
    if state.pending.is_none() && state.active != seat {
        return LineStatus::Yielded;
    }
    LineStatus::Open
}

/// TS `{ ok: true; state } | { ok: false; error }` (private in TS; public here, as `simulate`'s answer).
pub type SimStep = Result<GameState, String>;

/// One AI action on a determinized state: reduce (nonce `sim:${counter.used}`), then, while a prompt
/// of the other seat is open, answer it with the first entry of legalActions (each answer one more
/// node, at most AI_SEARCH.maxAutoAnswers). A throw or a refusal is `{ ok: false }`. Returns null
/// without touching anything when the counter refuses a node.
pub fn simulate(state: &GameState, seat: PlayerId, action: &ActionBody, counter: &dyn NodeCounter) -> Option<SimStep> {
    if !counter.take() {
        return None;
    }
    let run = catch_unwind(AssertUnwindSafe(|| -> Option<SimStep> {
        let first = reduce(
            state,
            &Action::new(action.clone(), seat, format!("sim:{}", counter.used())),
        );
        if let Some(error) = first.error {
            note_sim_error(counter);
            return Some(Err(error));
        }
        let mut current = first.state;
        let mut answered: usize = 0;
        loop {
            let other = match (&current.result, &current.pending) {
                (None, Some(pending)) if pending.player_id != seat => pending.player_id,
                _ => break,
            };
            if answered >= AI_SEARCH.max_auto_answers as usize {
                note_sim_error(counter);
                return Some(Err("too many opponent prompts in one step".to_string()));
            }
            let Some(reply) = legal_actions(&current, other).into_iter().next() else {
                note_sim_error(counter);
                return Some(Err("the opponent's prompt has no answer".to_string()));
            };
            if !counter.take() {
                return None;
            }
            let next = reduce(&current, &Action::new(reply, other, format!("sim:{}", counter.used())));
            if let Some(error) = next.error {
                note_sim_error(counter);
                return Some(Err(error));
            }
            current = next.state;
            answered += 1;
        }
        Some(Ok(current))
    }));
    match run {
        Ok(step) => step,
        Err(payload) => {
            note_sim_error(counter);
            Some(Err(panic_message(payload.as_ref())))
        }
    }
}

/// A passed turn's value: the evaluation, less the crystals the seat left unspent.
fn passed_score(state: &GameState, seat: PlayerId) -> f64 {
    let unspent = state.players[seat].turn_log.unspent_at_end.unwrap_or(0);
    evaluate(state, seat, NextSwing::Enemy, &AI_EVAL) - AI_EVAL.unspent_mana * f64::from(unspent)
}

/// `closeLine`'s step: the line's turn ended here, or `None` where TS handed `state` back itself
/// (which `terminalScore` tells apart by identity).
fn close_line_step(state: &GameState, seat: PlayerId, root_turn: i32, counter: &dyn NodeCounter) -> Option<GameState> {
    if line_status(state, seat, root_turn) == LineStatus::Open
        && state.pending.is_none()
        && state.active == seat
        && state.phase == Phase::Main
        && let Some(Ok(next)) = simulate(state, seat, &ActionBody::EndTurn, counter)
    {
        return Some(next);
    }
    None
}

/// Where a line stops: a line still open in the seat's main phase ends its turn here (one node, when
/// one is left). Anything else stops where it stands.
pub fn close_line(state: &GameState, seat: PlayerId, root_turn: i32, counter: &dyn NodeCounter) -> GameState {
    close_line_step(state, seat, root_turn, counter).unwrap_or_else(|| state.clone())
}

/// The static value of a line that stopped at `state`: a passed turn pays for its unspent mana.
pub fn static_score(state: &GameState, seat: PlayerId, root_turn: i32) -> f64 {
    if line_status(state, seat, root_turn) == LineStatus::Passed {
        passed_score(state, seat)
    } else {
        evaluate(state, seat, NextSwing::Enemy, &AI_EVAL)
    }
}

/// A line's value: "over"/"yielded" → evaluate; "passed" → evaluate − AI_EVAL.unspentMana ×
/// (players[seat].turnLog.unspentAtEnd ?? 0); "open" → simulate endTurn if the seat is in its main
/// phase and a node is left, then score that, else evaluate.
pub fn terminal_score(state: &GameState, seat: PlayerId, root_turn: i32, counter: &dyn NodeCounter) -> f64 {
    let status = line_status(state, seat, root_turn);
    if status == LineStatus::Open {
        return match close_line_step(state, seat, root_turn, counter) {
            None => evaluate(state, seat, NextSwing::Enemy, &AI_EVAL),
            Some(closed) => static_score(&closed, seat, root_turn),
        };
    }
    static_score(state, seat, root_turn)
}

/// A cheap identity for a position inside one turn, for the lethal solver's visited set and the
/// beam's duplicate check: both heroes, every active unit's id, damage, buffs, exertion and position,
/// the seat's hand and mana, the enemy hand size and the open prompt.
pub fn search_signature(state: &GameState, seat: PlayerId) -> String {
    let opp = opponent_of(seat);
    let mut parts: Vec<String> = vec![
        format!("h{}/{}", state.players[opp].hero.health, state.players[opp].hero.armor),
        format!("m{}/{}", state.players[seat].hero.health, state.players[seat].hero.armor),
        format!("${}", state.players[seat].mana.current),
        format!("n{}", state.players[opp].hand.len()),
        format!("q{}", state.pending.as_ref().map_or("-", |pending| pending.id.as_str())),
    ];
    for player in PLAYER_IDS {
        for unit in active_units_of(state, player) {
            let attacks = unit
                .exertion
                .attacks
                .unwrap_or(if unit.exertion.attacked { 1 } else { 0 });
            let exerted = format!("{}{}", attacks, if unit.exertion.switched { 1 } else { 0 });
            parts.push(format!(
                "{}:{}:{}/{}:{}:{}",
                unit.id,
                unit.damage,
                unit.buffs.attack,
                unit.buffs.health,
                exerted,
                unit.position.map_or("ATK", |position| position.as_str()),
            ));
        }
        let backrow = state.players[player]
            .backrow
            .iter()
            .map(|card| card.as_ref().map_or("-", |card| card.id.as_str()))
            .collect::<Vec<_>>()
            .join(",");
        parts.push(format!("b{backrow}"));
    }
    parts.push(format!(
        "H{}",
        state.players[seat]
            .hand
            .iter()
            .map(|card| card.id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    ));
    parts.join("|")
}
