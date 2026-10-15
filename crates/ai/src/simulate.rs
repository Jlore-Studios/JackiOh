//! Simulation through the real reducer (docs/polish/3-ai.md, SPEC §9.9). Every `reduce` call the AI
//! makes goes through `simulate` and costs one node of the decision's counter, the opponent's
//! auto-answers included (every answer `best_auto_answer` tries), so budgets are counted in engine
//! calls and a decision is deterministic.
//!
//! Port of `packages/ai/src/simulate.ts`. A TS counter is one object every part of a decision shares
//! (a sub-counter holds its parent while the parent is spent elsewhere too), so the counters keep
//! their tallies in `Cell`s and are passed as `&dyn NodeCounter` (`types.rs`). TS's `try`/`catch`
//! around the reducer is `catch_unwind`: an engine invariant that panics where TS threw is a failed
//! simulation, never a crash of the decision.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::{
    Action, ActionBody, CardInstance, GameState, PLAYER_IDS, Phase, PlayerId, active_units_of, find_instance,
    legal_actions, opponent_of, owed_mana_of, reduce, unit_view,
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

pub fn create_node_counter<'a>(
    limit: usize,
    should_stop: Option<&'a dyn Fn() -> bool>,
) -> CountingNodeCounter<'a> {
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

/// TS `error instanceof Error ? error.message : String(error)`, for a panic's payload. The one reader
/// of a panic's message: the AI's simulations and match runner, the server's match tasks and the
/// tools' agent, fuzz and arena all call it.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
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

/// The extra nodes an action is charged for the work its `reduce` really does (AI_SEARCH's
/// expensive_def_* and ai_turn_trap_*): a Play of a card whose resolve ranks the whole catalog for
/// the state (the §10.7 scorer) or an Attack that could set off a trap which hands the attacker a
/// whole random turn (§8.5 core-096). One reduce of either costs the wall clock a hundred cheap
/// sims do, so the step is budgeted its share up front — otherwise a hand or backrow holding one
/// burns a decision's 2 s on a few sims (R186's sweep flag) as easily as on a few cheap ones.
pub fn surcharge_nodes(state: &GameState, actor: PlayerId, action: &ActionBody) -> usize {
    match action {
        ActionBody::Play { instance_id, .. } => find_instance(state, instance_id)
            .filter(|card| AI_SEARCH.expensive_def_ids.contains(&card.def_id.as_str()))
            .map_or(0, |_| AI_SEARCH.expensive_def_nodes.max(0) as usize),
        ActionBody::Attack { attacker_id, .. } => {
            let defender = opponent_of(actor);
            // The trap's `when` asks a would-be-lethal declaration at its controller's hero; the
            // surcharge reads the same gate approximately — the unit's attack against the hero's
            // health — so a chip attack at an armed board is not charged the playout's price.
            let lethalish = find_instance(state, attacker_id)
                .is_some_and(|unit| unit_view(state, unit).attack >= state.players[defender].hero.health);
            let trapped = state.players[defender]
                .backrow
                .iter()
                .flatten()
                .any(|card| AI_SEARCH.ai_turn_trap_ids.contains(&card.def_id.as_str()));
            if lethalish && trapped {
                AI_SEARCH.ai_turn_trap_nodes.max(0) as usize
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// One auto-answer (#891): of the first `options` answers `answerer`'s prompt lists (legal_actions
/// less Concede, which ends the game rather than answering), the one whose result `evaluate` likes
/// best from the answerer's own seat, so a world does not assume the answerer takes option zero. The
/// score reads the determinization only, as everything here does (R185). Each answer tried is one
/// node (nonce `${prefix}:${counter.used}`); strictly greater wins, so ties keep legal_actions order,
/// and a lone answer is committed unscored. A throw or a refusal is a failed simulation and the next
/// answer is tried; `Some(Err)` when none is left. `None` when the counter refuses a node.
pub(crate) fn best_auto_answer(
    state: &GameState,
    answerer: PlayerId,
    options: usize,
    counter: &dyn NodeCounter,
    prefix: &str,
) -> Option<SimStep> {
    let answers: Vec<ActionBody> = legal_actions(state, answerer)
        .into_iter()
        .filter(|answer| !matches!(answer, ActionBody::Concede))
        .take(options)
        .collect();
    if answers.is_empty() {
        note_sim_error(counter);
        return Some(Err("the prompt has no answer".to_string()));
    }
    let scored = answers.len() > 1;
    let mut best: Option<(GameState, f64)> = None;
    let mut failure = String::new();
    for answer in answers {
        if !counter.take() {
            return None;
        }
        let action = Action::new(answer, answerer, format!("{prefix}:{}", counter.used()));
        let next = match catch_unwind(AssertUnwindSafe(|| reduce(state, &action))) {
            Ok(next) => next,
            Err(payload) => {
                note_sim_error(counter);
                failure = panic_message(payload.as_ref());
                continue;
            }
        };
        if let Some(error) = next.error {
            note_sim_error(counter);
            failure = error;
            continue;
        }
        let score = if scored {
            let swing = if next.state.active == answerer {
                NextSwing::Seat
            } else {
                NextSwing::Enemy
            };
            evaluate(&next.state, answerer, swing, &AI_EVAL)
        } else {
            0.0
        };
        if best.as_ref().is_none_or(|(_, best_score)| score > *best_score) {
            best = Some((next.state, score));
        }
    }
    Some(best.map(|(state, _)| state).ok_or(failure))
}

/// One AI action on a determinized state: reduce (nonce `sim:${counter.used}`), then, while a prompt
/// of the other seat is open, answer it with the answerer's best of its first
/// AI_SEARCH.max_auto_answer_options answers (`best_auto_answer`: each answer tried one more node, at
/// most AI_SEARCH.maxAutoAnswers prompts). A throw or a refusal is `{ ok: false }`. Returns null
/// without touching anything when the counter refuses a node.
pub fn simulate(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
) -> Option<SimStep> {
    simulate_answering(
        state,
        seat,
        action,
        counter,
        AI_SEARCH.max_auto_answer_options.max(1) as usize,
    )
}

/// `simulate` as it stood before #891, for the greedy baseline: every prompt of the other seat gets
/// the first entry of legalActions, one node each, and nothing is scored with AI_EVAL, so the
/// yardstick the gates measure the AI against stays put (docs/polish/3-ai.md, "A baseline that
/// stays put").
pub fn simulate_first_answers(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
) -> Option<SimStep> {
    simulate_answering(state, seat, action, counter, 1)
}

fn simulate_answering(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
    options: usize,
) -> Option<SimStep> {
    if !counter.take() {
        return None;
    }
    // An action whose surcharge the slice cannot pay must not drain it node by node and then
    // return `None`: `None` is the callers' "the budget is spent, stop the wave" signal, so one
    // expensive step would stop every cheaper sibling beside it. Refuse the step like any refused
    // action instead (Some(Err): the beam's `continue`, reply's "skip this action").
    let surcharge = surcharge_nodes(state, seat, action);
    if counter.used() + surcharge > counter.limit() {
        note_sim_error(counter);
        return Some(Err(
            "the step's surcharge does not fit the budget's slice".to_string()
        ));
    }
    for _ in 0..surcharge {
        if !counter.take() {
            return None;
        }
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
            current = match best_auto_answer(&current, other, options, counter, "sim")? {
                Ok(next) => next,
                Err(error) => return Some(Err(error)),
            };
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
fn close_line_step(
    state: &GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
) -> Option<GameState> {
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

/// What a backrow card or an own hand card is, for `search_signature`: its id, its definition (a
/// Transform or a discovered card keeps the id), its Radiant face, and its cost changes.
fn card_signature(card: &CardInstance) -> String {
    format!(
        "{}={}{}{}/{:?}",
        card.id,
        card.def_id,
        if card.radiant { "*" } else { "" },
        card.cost_mod,
        card.cost_override,
    )
}

/// What changes how an active unit plays beyond its id, damage, buffs, exertion and position: its
/// definition and Radiant face, a Vanilla, its stats and keywords through the layers (§10.4), its stat
/// and armor overrides, a spent Divine Shield, and its counters and enchantments.
fn unit_play_signature(state: &GameState, unit: &CardInstance) -> String {
    let view = unit_view(state, unit);
    format!(
        "{}{}{}:{}/{}/{}/{}:{:?}:{:?}/{:?}:{}:{:?}:{:?}",
        unit.def_id,
        if unit.radiant { "*" } else { "" },
        if unit.vanilla { "~" } else { "" },
        view.attack,
        view.max_health,
        view.health,
        view.armor,
        view.keywords,
        unit.stats_override,
        unit.armor_override,
        u8::from(unit.divine_shield_spent == Some(true)),
        unit.counters,
        unit.enchantments,
    )
}

/// A cheap identity for a position inside one turn, for the lethal solver's visited set and the
/// beam's duplicate check: both heroes, the open prompt whole, both library sizes, every active unit's
/// id, damage, buffs, exertion and position and what else changes how it plays (`unit_play_signature`),
/// every backrow card (`card_signature`, and whether it is face up), the seat's hand (`card_signature`)
/// and mana, and the enemy hand size. A unit's lane is left out, so the two lanes `candidate_actions`
/// keeps for one play stay one position.
pub fn search_signature(state: &GameState, seat: PlayerId) -> String {
    let opp = opponent_of(seat);
    let mut parts: Vec<String> = vec![
        format!(
            "h{}/{}",
            state.players[opp].hero.health, state.players[opp].hero.armor
        ),
        format!(
            "m{}/{}",
            state.players[seat].hero.health, state.players[seat].hero.armor
        ),
        format!("${}", state.players[seat].mana.current),
        format!("n{}", state.players[opp].hand.len()),
        format!(
            "q{}",
            state
                .pending
                .as_ref()
                .map_or("-".to_string(), |pending| format!("{pending:?}"))
        ),
        format!(
            "l{}/{}",
            state.players[seat].library.len(),
            state.players[opp].library.len()
        ),
    ];
    for player in PLAYER_IDS {
        for unit in active_units_of(state, player) {
            let attacks = unit
                .exertion
                .attacks
                .unwrap_or(if unit.exertion.attacked { 1 } else { 0 });
            let exerted = format!("{}{}", attacks, if unit.exertion.switched { 1 } else { 0 });
            parts.push(format!(
                "{}:{}:{}/{}:{}:{}:{}",
                unit.id,
                unit.damage,
                unit.buffs.attack,
                unit.buffs.health,
                exerted,
                unit.position.map_or("ATK", |position| position.as_str()),
                unit_play_signature(state, unit),
            ));
        }
        let backrow = state.players[player]
            .backrow
            .iter()
            .map(|card| {
                card.as_ref().map_or("-".to_string(), |card| {
                    let face = if card.face_up == Some(true) { "^" } else { "" };
                    format!("{}{face}:{:?}", card_signature(card), card.counters)
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        parts.push(format!("b{backrow}"));
    }
    parts.push(format!(
        "H{}",
        state.players[seat]
            .hand
            .iter()
            .map(card_signature)
            .collect::<Vec<_>>()
            .join(",")
    ));
    // R1224: a debt is part of the position — but only when one is owed, so signatures without debt
    // stay exactly as they were.
    let owed = owed_mana_of(state, seat);
    if owed > 0 {
        parts.push(format!("d{owed}"));
    }
    parts.join("|")
}
