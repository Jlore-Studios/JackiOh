//! Simulation through the real reducer (docs/polish/3-ai.md, SPEC §9.9). Every engine `reduce`
//! costs one node of the decision's counter, ensuring deterministic search budgets.
//!
//! Node counters use interior mutability (`&dyn NodeCounter`, `types.rs`). Engine panics are
//! caught via `catch_unwind` and tracked in `SearchStats.simPanics` (#1429).

use std::cell::{Cell, OnceCell};
use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::{
    Action, ActionBody, CardInstance, GameEvent, GameState, PLAYER_IDS, Phase, PlayerId, ReduceResult, Rng,
    TurnLog, active_units_of, effective_life, find_instance, is_sick, legal_actions, max_mana_for,
    opponent_of, owed_mana_of, reduce_unpiloted, self_def_ids, slot_of, subsystems, unit_view,
    units_on_field_of,
};
use serde::{Deserialize, Serialize};

use crate::candidates::action_key;
use crate::config::{AI_EVAL, AI_SEARCH};
use crate::evaluate::{NextSwing, Prices, evaluate_priced};
use crate::learned::{line_score, open_score};
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
        sim_panics: Cell<usize>,
        first_sim_panic: OnceCell<String>,
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

    /// #1429: the panics of `sim_errors`; a sub-counter reads its parent's tally (0 when the parent
    /// keeps none).
    pub fn sim_panics(&self) -> usize {
        match &self.kind {
            CounterKind::Root { sim_panics, .. } => sim_panics.get(),
            CounterKind::Sub { parent, .. } => parent.sim_panic_tally().unwrap_or(0),
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

    fn sim_panic_tally(&self) -> Option<usize> {
        Some(self.sim_panics())
    }

    fn first_sim_panic(&self) -> Option<String> {
        match &self.kind {
            CounterKind::Root { first_sim_panic, .. } => first_sim_panic.get().cloned(),
            CounterKind::Sub { parent, .. } => parent.first_sim_panic(),
        }
    }

    fn record_sim_panic(&self, message: &str) {
        match &self.kind {
            CounterKind::Root {
                sim_panics,
                first_sim_panic,
                ..
            } => {
                sim_panics.set(sim_panics.get() + 1);
                if first_sim_panic.get().is_none() {
                    let _ = first_sim_panic.set(message.to_string());
                }
            }
            CounterKind::Sub { parent, .. } => parent.record_sim_panic(message),
        }
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
            sim_panics: Cell::new(0),
            first_sim_panic: OnceCell::new(),
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

/// #1429: a caught panic is a failed simulation, counted apart too, its message kept when first;
/// returns the message.
pub(crate) fn note_sim_panic(counter: &dyn NodeCounter, payload: &(dyn std::any::Any + Send)) -> String {
    note_sim_error(counter);
    let message = panic_message(payload);
    counter.record_sim_panic(&message);
    message
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

/// Up-front node surcharge for heavy actions (expensive scorers §10.7, turn-granting traps §8.5 core-096,
/// watching players, or R186 sweep flags; #1240).
pub fn surcharge_nodes(state: &GameState, actor: PlayerId, action: &ActionBody) -> usize {
    match action {
        ActionBody::Play { instance_id, .. } => {
            let expensive = find_instance(state, instance_id)
                .filter(|card| stands_for_any(state, &card.def_id, AI_SEARCH.expensive_def_ids))
                .map_or(0, |_| AI_SEARCH.expensive_def_nodes.max(0) as usize);
            let judged = if subsystems::watching(state, actor) {
                AI_SEARCH.judged_card_nodes.max(0) as usize * state.players[actor].hand.len()
            } else {
                0
            };
            expensive + judged
        }
        ActionBody::Attack { attacker_id, .. } => {
            let defender = opponent_of(actor);
            // Trapped lethal-gate estimate against effective_life (R964000; never if cannot lose, R2201000).
            let lethalish = jackioh_engine::can_lose(state, defender)
                && find_instance(state, attacker_id)
                    .is_some_and(|unit| unit_view(state, unit).attack >= effective_life(state, defender));
            let trapped = state.players[defender]
                .backrow
                .iter()
                .flatten()
                .any(|card| stands_for_any(state, &card.def_id, AI_SEARCH.ai_turn_trap_ids));
            if lethalish && trapped {
                AI_SEARCH.ai_turn_trap_nodes.max(0) as usize
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Post-step surcharge for unannounced expensive card resolves like random/graveyard casts (#1347).
pub fn announced_surcharge_nodes(state: &GameState, action: &ActionBody, stepped: &ReduceResult) -> usize {
    let ids = AI_SEARCH.expensive_def_ids;
    let announced = stepped
        .events
        .iter()
        .filter(|event| {
            matches!(event, GameEvent::CardAnnounced { def_id, .. } if stands_for_any(&stepped.state, def_id, ids))
        })
        .count();
    let prepaid = match action {
        ActionBody::Play { instance_id, .. } => usize::from(
            find_instance(state, instance_id).is_some_and(|card| stands_for_any(state, &card.def_id, ids)),
        ),
        _ => 0,
    };
    announced.saturating_sub(prepaid) * AI_SEARCH.expensive_def_nodes.max(0) as usize
}

/// Takes a step's `announced_surcharge_nodes` from `counter`, as many as it still gives: the step's
/// work is done and kept, and a spent counter stops the search at its next node.
pub(crate) fn charge_announced(
    counter: &dyn NodeCounter,
    state: &GameState,
    action: &ActionBody,
    stepped: &ReduceResult,
) {
    for _ in 0..announced_surcharge_nodes(state, action, stepped) {
        if !counter.take() {
            return;
        }
    }
}

/// #1240: whether `def_id` is one of `ids`, or a fused definition (R77) one of whose ingredients is
/// (`self_def_ids`, fused ingredients included), since a fused card runs every ingredient's script.
fn stands_for_any(state: &GameState, def_id: &str, ids: &[&str]) -> bool {
    self_def_ids(Some(state), def_id)
        .iter()
        .any(|id| ids.contains(&id.as_str()))
}

/// Which answers of a prompt `best_auto_answer` tries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnswersTried {
    /// The first one legal_actions lists: the greedy baseline's (`simulate_first_answers`).
    First,
    /// AI_SEARCH.max_auto_answer_options of the whole answer set (`auto_answers`).
    Drawn,
}

/// The answers `best_auto_answer` tries of `answerer`'s prompt (excluding Concede).
/// `Drawn` (#1382) sorts by `action_key` and samples up to `AI_SEARCH.max_auto_answer_options` (R185).
fn auto_answers(state: &GameState, answerer: PlayerId, tried: AnswersTried) -> Vec<ActionBody> {
    let answers = legal_actions(state, answerer)
        .into_iter()
        .filter(|answer| !matches!(answer, ActionBody::Concede));
    if tried == AnswersTried::First {
        return answers.take(1).collect();
    }
    let mut keyed: Vec<(String, ActionBody)> = answers.map(|answer| (action_key(&answer), answer)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let ordered: Vec<ActionBody> = keyed.into_iter().map(|(_, answer)| answer).collect();
    let cap = AI_SEARCH.max_auto_answer_options.max(1) as usize;
    if ordered.len() <= cap {
        return ordered;
    }
    let prompt = state.pending.as_ref().map_or("", |pending| pending.id.as_str());
    let stream = format!("{}:{}:{prompt}", AI_SEARCH.auto_answer_seed, state.seed);
    let mut drawn = Rng::new(&stream, 0).shuffle(&ordered);
    drawn.truncate(cap);
    drawn
}

/// Finds best auto-answer by evaluating from the answerer's perspective (#891, R185).
/// Costs one node per answer tried; returns `None` on counter exhaustion.
pub(crate) fn best_auto_answer(
    state: &GameState,
    answerer: PlayerId,
    tried: AnswersTried,
    counter: &dyn NodeCounter,
    prefix: &str,
    prices: &Prices,
) -> Option<SimStep> {
    let answers = auto_answers(state, answerer, tried);
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
        let next = match catch_unwind(AssertUnwindSafe(|| reduce_unpiloted(state, &action))) {
            Ok(next) => next,
            Err(payload) => {
                failure = note_sim_panic(counter, payload.as_ref());
                continue;
            }
        };
        if let Some(error) = next.error {
            note_sim_error(counter);
            failure = error;
            continue;
        }
        charge_announced(counter, state, &action.body, &next);
        let score = if scored {
            let swing = if next.state.active == answerer {
                NextSwing::Seat
            } else {
                NextSwing::Enemy
            };
            evaluate_priced(&next.state, answerer, swing, &AI_EVAL, prices)
        } else {
            0.0
        };
        if best.as_ref().is_none_or(|(_, best_score)| score > *best_score) {
            best = Some((next.state, score));
        }
    }
    Some(best.map(|(state, _)| state).ok_or(failure))
}

/// #1112: what a state waiting on `seat`'s own prompt is worth once its prompts are answered, which is
/// how the beam ranks a child that waits there (search.rs): every prompt open on the way, the seat's
/// and any the other seat holds, gets its holder's best of AI_SEARCH.max_auto_answer_options answers
/// drawn from all of them (`best_auto_answer`, nonce `ahead:${counter.used}`, each answer tried one
/// node of `counter`), at most AI_SEARCH.max_auto_answers prompts, and the state reached is scored by
/// `open_score` as the beam scores a child. Only a score: the child stays at its prompt, so the beam
/// still picks its real answers. `None` when no prompt of the seat's is open, when the counter
/// refuses a node, and when no answer is accepted, the prompts outrun the cap or the engine panics
/// (a failed simulation, as in `simulate`): the child then keeps its plain evaluation.
pub fn own_prompt_score(
    state: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<f64> {
    let holder = |state: &GameState| match (&state.result, &state.pending) {
        (None, Some(pending)) => Some(pending.player_id),
        _ => None,
    };
    if holder(state) != Some(seat) {
        return None;
    }
    let run = catch_unwind(AssertUnwindSafe(|| -> Option<f64> {
        let mut current = state.clone();
        let mut answered: usize = 0;
        while let Some(answerer) = holder(&current) {
            if answered >= AI_SEARCH.max_auto_answers as usize {
                return None;
            }
            current =
                best_auto_answer(&current, answerer, AnswersTried::Drawn, counter, "ahead", prices)?.ok()?;
            answered += 1;
        }
        Some(open_score(&current, seat, NextSwing::Enemy, prices))
    }));
    run.unwrap_or_else(|payload| {
        note_sim_panic(counter, payload.as_ref());
        None
    })
}

/// Simulates one AI action on determinized state and auto-answers subsequent opponent prompts.
/// Returns `None` when node counter exhausts.
pub fn simulate(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<SimStep> {
    simulate_answering(state, seat, action, counter, AnswersTried::Drawn, prices)
}

/// `simulate` variant using first legal action for opponent prompts (baseline before #891).
pub fn simulate_first_answers(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
) -> Option<SimStep> {
    simulate_answering(state, seat, action, counter, AnswersTried::First, Prices::NONE)
}

fn simulate_answering(
    state: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
    tried: AnswersTried,
    prices: &Prices,
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
        let first = reduce_unpiloted(
            state,
            &Action::new(action.clone(), seat, format!("sim:{}", counter.used())),
        );
        if let Some(error) = first.error {
            note_sim_error(counter);
            return Some(Err(error));
        }
        charge_announced(counter, state, action, &first);
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
            current = match best_auto_answer(&current, other, tried, counter, "sim", prices)? {
                Ok(next) => next,
                Err(error) => return Some(Err(error)),
            };
            answered += 1;
        }
        Some(Ok(current))
    }));
    match run {
        Ok(step) => step,
        Err(payload) => Some(Err(note_sim_panic(counter, payload.as_ref()))),
    }
}

/// A passed turn's value: the line's score (`line_score`), less the crystals the seat left unspent.
fn passed_score(state: &GameState, seat: PlayerId, prices: &Prices) -> f64 {
    let unspent = state.players[seat].turn_log.unspent_at_end.unwrap_or(0);
    line_score(state, seat, NextSwing::Enemy, prices) - AI_EVAL.unspent_mana * f64::from(unspent)
}

/// `closeLine`'s step: the line's turn ended here, or `None` where TS handed `state` back itself
/// (which `terminalScore` tells apart by identity).
fn close_line_step(
    state: &GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<GameState> {
    if line_status(state, seat, root_turn) == LineStatus::Open
        && state.pending.is_none()
        && state.active == seat
        && state.phase == Phase::Main
        && let Some(Ok(next)) = simulate(state, seat, &ActionBody::EndTurn, counter, prices)
    {
        return Some(next);
    }
    None
}

/// Where a line stops: a line still open in the seat's main phase ends its turn here (one node, when
/// one is left). Anything else stops where it stands.
pub fn close_line(
    state: &GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> GameState {
    close_line_step(state, seat, root_turn, counter, prices).unwrap_or_else(|| state.clone())
}

/// The static value of a line that stopped at `state`: a passed turn pays for its unspent mana.
pub fn static_score(state: &GameState, seat: PlayerId, root_turn: i32, prices: &Prices) -> f64 {
    if line_status(state, seat, root_turn) == LineStatus::Passed {
        passed_score(state, seat, prices)
    } else {
        line_score(state, seat, NextSwing::Enemy, prices)
    }
}

/// A line's value: "over"/"yielded" → `line_score`; "passed" → `line_score` − AI_EVAL.unspentMana ×
/// (players[seat].turnLog.unspentAtEnd ?? 0); "open" → simulate endTurn if the seat is in its main
/// phase and a node is left, then score that, else `line_score` (R1012002).
pub fn terminal_score(
    state: &GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> f64 {
    let status = line_status(state, seat, root_turn);
    if status == LineStatus::Open {
        return match close_line_step(state, seat, root_turn, counter, prices) {
            None => line_score(state, seat, NextSwing::Enemy, prices),
            Some(closed) => static_score(&closed, seat, root_turn, prices),
        };
    }
    static_score(state, seat, root_turn, prices)
}

/// `items` sorted and joined by commas: a part of `search_signature` that two orders of the same moves
/// must sign alike.
fn sorted(items: impl Iterator<Item = String>) -> String {
    let mut items: Vec<String> = items.collect();
    items.sort();
    items.join(",")
}

/// What rides a card through its zones and changes how it plays later (#898): its memory, keys sorted
/// so the order a script wrote them in splits nothing, and its tuning (B3.4, R386). Each only when it
/// holds something, so a card without them signs as before.
fn rides_signature(card: &CardInstance) -> String {
    let mut signature = String::new();
    if !card.memory.is_empty() {
        let memory = sorted(card.memory.iter().map(|(key, value)| format!("{key}={value}")));
        signature.push_str(&format!(";m{memory}"));
    }
    if let Some(tuning) = &card.tuning {
        signature.push_str(&format!(";t{tuning:?}"));
    }
    signature
}

/// What a card off the field is, for `search_signature` (B5 E39, #898).
fn card_signature(card: &CardInstance) -> String {
    let enchantments = match &card.enchantments {
        Some(enchantments) if !enchantments.is_empty() => format!(";e{enchantments:?}"),
        _ => String::new(),
    };
    format!(
        "{}={}{}{}/{:?}{}{enchantments}",
        card.id,
        card.def_id,
        if card.radiant { "*" } else { "" },
        card.cost_mod,
        card.cost_override,
        rides_signature(card),
    )
}

/// Unit signature for search duplicate detection, incorporating layers (§10.4), #898, and freeze thaw (R1174000).
fn unit_play_signature(state: &GameState, unit: &CardInstance) -> String {
    let view = unit_view(state, unit);
    let frozen = unit.frozen.map(|thaw| format!(":f{thaw}")).unwrap_or_default();
    format!(
        "{}{}{}:{}/{}/{}/{}:{:?}:{:?}/{:?}:{}:{:?}:{:?}{}{frozen}",
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
        rides_signature(unit),
    )
}

/// Non-default player parts for `search_signature` (mana, riders R844, locks, fatigue, graveyard, R1854003, R1224, #898).
fn push_side_parts(state: &GameState, player: PlayerId, parts: &mut Vec<String>) {
    let side = &state.players[player];
    let mana = &side.mana;
    if mana.max != max_mana_for(side) || mana.next_turn_mod != 0 || mana.perm_mod != 0 {
        parts.push(format!(
            "M{player}{}/{}/{}",
            mana.max, mana.next_turn_mod, mana.perm_mod
        ));
    }
    // R844: a lost refresh still to come, only while one is.
    if let Some(through) = side.lost_refresh_through {
        parts.push(format!("R{player}{through}"));
    }
    // R1854003: mana crystals a steal moved, which every later refresh reads, only while some are.
    if let Some(crystals) = side.mana_crystals {
        parts.push(format!("C{player}{crystals}"));
    }
    if side
        .locks
        .units
        .iter()
        .chain(&side.locks.backrow)
        .any(|locked| *locked)
    {
        parts.push(format!(
            "L{player}{:?}/{:?}",
            side.locks.units, side.locks.backrow
        ));
    }
    if !side.mods.is_empty() {
        let mods = sorted(
            side.mods
                .iter()
                .map(|modifier| format!("{:?}{:?}", modifier.expiry, modifier.kind)),
        );
        parts.push(format!("X{player}{mods}"));
    }
    // The log's counts, not `unspentAtEnd`: the close of the other player's last turn stays in their
    // log all through the seat's turn (`turn.rs`), the same in every position a search compares.
    let log = &side.turn_log;
    let quiet = TurnLog {
        unspent_at_end: log.unspent_at_end,
        ..TurnLog::default()
    };
    if *log != quiet {
        let mut costs = log.costs_paid.clone().unwrap_or_default();
        costs.sort_unstable();
        parts.push(format!(
            "T{player}{}:{}:{costs:?}:{}:{:?}/{:?}/{:?}",
            log.cards_played,
            sorted(log.played_ids.iter().cloned()),
            sorted(
                log.played_by_type
                    .iter()
                    .flatten()
                    .map(|(kind, count)| format!("{kind:?}{count}"))
            ),
            log.mana_borrowed,
            log.borrowed_parts,
            log.mana_locked,
        ));
    }
    if side.fatigue_count != 0 {
        parts.push(format!("F{player}{}", side.fatigue_count));
    }
    if let Some(jade) = side.jade {
        parts.push(format!("J{player}{jade}"));
    }
    if let Some(extra) = side.extra_turns {
        parts.push(format!("E{player}{extra}"));
    }
    if let Some(cap) = side.hand_cap {
        parts.push(format!("C{player}{cap}"));
    }
    if !side.graveyard.is_empty() {
        parts.push(format!(
            "G{player}{}",
            sorted(side.graveyard.iter().map(card_signature))
        ));
    }
    if !side.exile.is_empty() {
        parts.push(format!(
            "Y{player}{}",
            sorted(side.exile.iter().map(card_signature))
        ));
    }
    if let Some(piles) = &side.backrow_piles
        && piles.iter().any(|pile| !pile.is_empty())
    {
        let piles = piles
            .iter()
            .map(|pile| pile.iter().map(card_signature).collect::<Vec<_>>().join(","))
            .collect::<Vec<_>>()
            .join("/");
        parts.push(format!("P{player}{piles}"));
    }
}

/// Position signature for beam deduplication and lethal visited set (R1224, #898, #1797).
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
            // R1829000: a card a Stack pile uncovers keeps its id and is summoning sick again, so the
            // sickness is part of the position, or two children that differ only in it would merge.
            let exerted = format!(
                "{}{}{}",
                attacks,
                if unit.exertion.switched { 1 } else { 0 },
                if is_sick(state, unit) { "z" } else { "" }
            );
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
        // R1839000: a Dormant card is no active unit, yet it fills its zone and counts down: it is part of
        // the position, said only when there is one, so every other signature stays as it was.
        for unit in units_on_field_of(state, player) {
            if let Some(slumber) = unit.slumber {
                parts.push(format!("z{}:{:?}", unit.id, slumber.turns));
            }
            // R1852000: and a Dormant Starship is the pieces fused into it, which its definition names.
            if let Some(ship) = unit.starship.as_ref().filter(|_| unit.slumber.is_some()) {
                parts.push(format!("s{}:{}:{}", unit.id, unit.def_id, ship.pieces.len()));
            }
        }
        let backrow = state.players[player]
            .backrow
            .iter()
            .map(|card| {
                card.as_ref().map_or("-".to_string(), |card| {
                    // R686: a revealed card shows its face but is not face-up (a Trap of it still fires).
                    let face = if card.face_up == Some(true) {
                        "^"
                    } else if card.revealed == Some(true) {
                        "~"
                    } else {
                        ""
                    };
                    format!("{}{face}:{:?}", card_signature(card), card.counters)
                })
            })
            .collect::<Vec<_>>()
            .join(",");
        parts.push(format!("b{backrow}"));
        push_side_parts(state, player, &mut parts);
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
    // #898: the game's delayed effects and marks, by the same rule — only when there are some, sorted,
    // and without the sequence ids the engine names them by.
    if !state.delayed.is_empty() {
        let delayed = sorted(state.delayed.iter().map(|entry| {
            format!(
                "{}{:?}{:?}{:?}{:?}{:?}",
                entry.owner, entry.at, entry.not_before, entry.resume, entry.watch, entry.hand_watch
            )
        }));
        parts.push(format!("D{delayed}"));
    }
    if let Some(marks) = &state.marks
        && !marks.is_empty()
    {
        let marks = sorted(
            marks
                .iter()
                .map(|mark| format!("{}:{}:{}", mark.instance_id, mark.mark, mark.color)),
        );
        parts.push(format!("K{marks}"));
    }
    // R1853004: an open Marketplace's deals change the seat's Credits and its stock, which are part of
    // the position only while it is open, so every other signature stays as it was (a card bought or
    // sold is in the hand's part).
    if let Some(market) = &state.marketplace {
        let side = &state.players[seat];
        parts.push(format!(
            "W{}:{:?}",
            side.market_credits.unwrap_or(0),
            market
                .offers
                .iter()
                .map(|offer| offer.bought[seat])
                .collect::<Vec<_>>()
        ));
    }
    parts.join("|")
}

/// Every active unit's lane, each side's in `active_units_of` order, the sides in `PLAYER_IDS` order
/// (#1797): what `search_signature` leaves out. The beam's duplicate check reads the two together, so
/// two placements of one play that differ only in lane stay two positions, while two orders of the
/// same moves into the same lanes stay one.
pub fn unit_lanes(state: &GameState) -> String {
    PLAYER_IDS
        .into_iter()
        .map(|player| {
            active_units_of(state, player)
                .into_iter()
                .map(|unit| slot_of(state, unit).map_or(0, |slot| slot.lane).to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("/")
}
