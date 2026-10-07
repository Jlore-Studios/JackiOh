//! The AI's one entry point (SPEC §9.9). The order below is the contract:
//!
//!   1. ai_to_act / redact — the only two reads of the true state (R185).
//!   2. an unanswered draw offer is declined at once (R188).
//!   3. the mulligan keeps the cheap cards.
//!   4. a single candidate is played with no search: listed on a throwaway determinization, since
//!      the seat's own legal actions never depend on the hidden cards, so `options.rng` is untouched.
//!   5. the exact lethal solver, verified on every determinization.
//!   6. the beam on determinization 0; its best lines are scored after the opponent's reply
//!      (reply.rs), the best first actions are re-scored the same way on the other determinizations,
//!      and the best mean wins.
//!   7. a fallback when nothing could be scored.
//!
//! Only the first action of the chosen line is played; the caller asks again after it, so the AI
//! re-plans after every action and a plan that only one sampled world liked never gets past step one.
//! `decide` never panics out: every internal failure becomes a fallback and is counted in simErrors.
//!
//! Port of `packages/ai/src/decide.ts` (SURFACE §9). TS's `try`/`catch` is `catch_unwind` (an engine
//! invariant is a panic in Rust, SURFACE §4.4.9). TS made the node counter after the determinizations;
//! here it is made first, so the counter outlives the guarded body. Nothing differs: a counter is only
//! read through `take()`, which nothing calls before the lethal solver, so its stats are the same.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexMap;
use jackioh_engine::{ActionBody, GameState, PlayerId, Rng, mulligan_prompt_for};

use crate::candidates::{action_key, candidate_actions};
use crate::config::{AI_EVAL, AI_MULLIGAN, AI_REPLY, AI_SEARCH};
use crate::determinize::{DeterminizeOptions, determinize};
use crate::lethal::find_lethal;
use crate::mulligan::mulligan_keep;
use crate::observe::{ai_to_act, redact, unanswered_draw_offer};
use crate::reply::{hidden_card_ids, reply_score};
use crate::search::{Line, beam_search, score_line};
use crate::simulate::{CountingNodeCounter, create_node_counter, create_sub_counter};
use crate::types::{AiOptions, Decision, DecisionReason, NodeCounter, SearchBudget, SearchStats, StoppedBy};

fn quiet_stats() -> SearchStats {
    SearchStats {
        nodes: 0,
        determinizations: 0,
        lines: 0,
        sim_errors: 0,
        stopped_by: StoppedBy::Exhausted,
        score: 0.0,
    }
}

fn immediate(action: ActionBody, reason: DecisionReason) -> Decision {
    Decision { line: vec![action.clone()], action, reason, stats: quiet_stats() }
}

/// Step 7: endTurn when it is a candidate, else the first candidate, else endTurn regardless.
fn fallback_action(candidates: &[ActionBody]) -> ActionBody {
    candidates
        .iter()
        .find(|action| matches!(action, ActionBody::EndTurn))
        .or_else(|| candidates.first())
        .cloned()
        .unwrap_or(ActionBody::EndTurn)
}

/// Lines in order, at most `perAction` for any one first action, `count` in all.
fn shortlist_lines(found: &[Line], count: usize) -> Vec<&Line> {
    let mut taken: IndexMap<String, i32> = IndexMap::new();
    let mut out: Vec<&Line> = Vec::new();
    for line in found {
        let Some(first) = line.actions.first() else {
            continue;
        };
        let key = action_key(first);
        let already = taken.get(&key).copied().unwrap_or(0);
        if already >= AI_SEARCH.lines_per_action {
            continue;
        }
        taken.insert(key, already + 1);
        out.push(line);
        if out.len() >= count {
            break;
        }
    }
    out
}

/// One line and the score it was given.
#[derive(Clone, Copy)]
struct Scored<'a> {
    line: &'a Line,
    score: f64,
}

/// The best-scoring line for each distinct first action, best first, at most `count` of them.
fn best_per_first_action<'a>(scored: &[Scored<'a>], count: usize) -> Vec<Scored<'a>> {
    let mut best: IndexMap<String, Scored<'a>> = IndexMap::new();
    for entry in scored {
        let Some(first) = entry.line.actions.first() else {
            continue;
        };
        let key = action_key(first);
        match best.get_mut(&key) {
            None => {
                best.insert(key, *entry);
            }
            Some(held) => {
                if entry.score > held.score {
                    *held = *entry;
                }
            }
        }
    }
    // The map keeps first-seen order (TS's `order` list).
    let mut entries: Vec<Scored<'a>> = best.into_values().collect();
    // Stable, so equal scores keep the beam's order.
    entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    entries.truncate(count);
    entries
}

/// What the guarded body decided, before the stats are read off the counter.
enum Plan {
    /// Answered at once, with quiet stats.
    Immediate(ActionBody, DecisionReason),
    /// Searched: the stats are the counter's, with this score and, when given, this stop reason.
    Searched {
        action: ActionBody,
        reason: DecisionReason,
        line: Vec<ActionBody>,
        score: f64,
        stopped_by: Option<StoppedBy>,
    },
}

/// What the guarded body leaves behind for the fallback when it panics (TS's outer `let`s).
struct Scratch {
    candidates: Vec<ActionBody>,
    determinizations: usize,
    lines: usize,
}

/// Steps 1–7 on the redacted copy; every number `stats` reads beside the counter goes in `scratch`.
fn plan(
    state: &GameState,
    seat: PlayerId,
    budget: SearchBudget,
    rng: &mut Rng,
    counter: &dyn NodeCounter,
    scratch: &mut Scratch,
) -> Plan {
    // 1. Everything below reads the redacted copy.
    let public = redact(state, seat);

    // 2. R188.
    if unanswered_draw_offer(&public, seat) {
        return Plan::Immediate(ActionBody::AnswerDraw { accept: false }, DecisionReason::DrawOffer);
    }

    // 3. The mulligan: its own, at once, whether or not the other seat has answered (R265).
    if public.pending.is_none() && mulligan_prompt_for(&public, seat).is_some() {
        let keep = mulligan_keep(&public, seat, AI_MULLIGAN.keep_max_cost);
        return Plan::Immediate(ActionBody::Mulligan { keep }, DecisionReason::Mulligan);
    }

    // 4. Forced: a throwaway world lists the candidates without touching options.rng.
    let probe = determinize(&public, seat, &mut Rng::new(AI_SEARCH.probe_seed, 0), DeterminizeOptions::default());
    scratch.candidates = candidate_actions(&probe, seat);
    if scratch.candidates.len() == 1
        && let Some(only) = scratch.candidates.first()
    {
        return Plan::Immediate(only.clone(), DecisionReason::Forced);
    }

    // The only draws decide ever takes from options.rng.
    let k = budget.determinizations.max(1);
    let mut dets: Vec<GameState> = Vec::new();
    for _ in 0..k {
        dets.push(determinize(&public, seat, rng, DeterminizeOptions::default()));
    }
    scratch.determinizations = k;

    // 5. Lethal, verified on every determinization.
    if let Some(lethal) = find_lethal(&dets, seat, counter, budget.lethal_nodes)
        && let Some(first_lethal) = lethal.first().cloned()
    {
        return Plan::Searched {
            action: first_lethal,
            reason: DecisionReason::Lethal,
            line: lethal,
            score: AI_EVAL.win - f64::from(public.turn),
            stopped_by: None,
        };
    }

    // 6. The beam on determinization 0, keeping enough nodes back to score the finalists after the
    // opponent's reply on every determinization.
    let Some(det0) = dets.first() else {
        let action = fallback_action(&scratch.candidates);
        return Plan::Searched { line: vec![action.clone()], action, reason: DecisionReason::Fallback, score: 0.0, stopped_by: None };
    };
    let root_turn = det0.turn;
    let remaining = budget.nodes.saturating_sub(counter.used());
    let finalist_count = budget.finalists.max(1);
    let per_finalist = (AI_SEARCH.lines_per_action * AI_REPLY.reserve_steps) as usize
        + (k - 1) * (budget.max_depth + 1 + AI_REPLY.reserve_steps as usize);
    let reserve = (remaining / 2).min(finalist_count * per_finalist);
    // The beam's own slice of the counter. Its stop reason is read as the beam ends: nothing takes a
    // node through the slice afterwards, so it is the reason TS read off it at the end.
    let (found, beam_stopped_by) = {
        let beam_counter = create_sub_counter(counter, remaining - reserve);
        let found = beam_search(det0, seat, &beam_counter, budget);
        (found, beam_counter.stopped_by())
    };
    scratch.lines = found.len();

    if found.is_empty() {
        let action = fallback_action(&scratch.candidates);
        let stopped_by =
            if counter.stopped_by() != StoppedBy::Exhausted { counter.stopped_by() } else { beam_stopped_by };
        return Plan::Searched {
            line: vec![action.clone()],
            action,
            reason: DecisionReason::Fallback,
            score: 0.0,
            stopped_by: Some(stopped_by),
        };
    }

    // Only a line's first action is ever played. The best lines (at most AI_SEARCH.linesPerAction
    // for any one first action) are scored after the opponent's reply on determinization 0; each
    // first action keeps its best line; the best `finalists` first actions are scored again on every
    // other determinization, and the best mean wins.
    let shortlist = shortlist_lines(&found, finalist_count * AI_SEARCH.lines_per_action as usize);
    let hidden0 = hidden_card_ids(det0, seat);
    let mut replied: Vec<Scored<'_>> = Vec::new();
    for line in shortlist {
        let Some(score) = reply_score(&line.end, seat, root_turn, counter, &hidden0) else {
            break;
        };
        replied.push(Scored { line, score });
    }
    let unreplied: Vec<Scored<'_>> = found.iter().map(|line| Scored { line, score: line.score }).collect();
    let finalists =
        best_per_first_action(if !replied.is_empty() { &replied } else { &unreplied }, finalist_count);

    let mut totals: Vec<f64> = finalists.iter().map(|entry| entry.score).collect();
    let mut scored_on: i32 = 1;
    for world in dets.iter().skip(1) {
        let hidden = hidden_card_ids(world, seat);
        let mut row: Vec<f64> = Vec::new();
        for entry in &finalists {
            let Some(score) =
                score_line(world, seat, &entry.line.actions, counter, !replied.is_empty(), Some(&hidden))
            else {
                break;
            };
            row.push(score);
        }
        // A world the counter could not finish is left out for every finalist alike.
        if row.len() < finalists.len() {
            break;
        }
        for (index, score) in row.into_iter().enumerate() {
            if let Some(total) = totals.get_mut(index) {
                *total += score;
            }
        }
        scored_on += 1;
    }

    let mut best_index = 0usize;
    let mut best_mean = totals.first().copied().unwrap_or(0.0) / f64::from(scored_on);
    for index in 1..finalists.len() {
        let mean = totals.get(index).copied().unwrap_or(0.0) / f64::from(scored_on);
        if mean > best_mean {
            best_mean = mean;
            best_index = index;
        }
    }

    let chosen = finalists.get(best_index).map(|entry| entry.line);
    let Some((chosen, action)) = chosen.and_then(|line| line.actions.first().map(|action| (line, action.clone())))
    else {
        let fallback = fallback_action(&scratch.candidates);
        return Plan::Searched {
            line: vec![fallback.clone()],
            action: fallback,
            reason: DecisionReason::Fallback,
            score: 0.0,
            stopped_by: None,
        };
    };
    let stopped_by = if counter.stopped_by() != StoppedBy::Exhausted { counter.stopped_by() } else { beam_stopped_by };
    let reason = if public.pending.as_ref().is_some_and(|pending| pending.player_id == seat) {
        DecisionReason::Prompt
    } else {
        DecisionReason::Search
    };
    Plan::Searched { action, reason, line: chosen.actions.clone(), score: best_mean, stopped_by: Some(stopped_by) }
}

/// The AI's one entry point. `None` when !ai_to_act(state, seat). Never panics out.
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision> {
    match catch_unwind(AssertUnwindSafe(|| ai_to_act(state, seat))) {
        Ok(true) => {}
        _ => return None,
    }

    let budget = options.budget;
    let counter = create_node_counter(budget.nodes, options.should_stop);
    let mut scratch = Scratch { candidates: Vec::new(), determinizations: 0, lines: 0 };
    let rng = &mut options.rng;

    let outcome = catch_unwind(AssertUnwindSafe(|| plan(state, seat, budget, rng, &counter, &mut scratch)));

    let stats = |counter: &CountingNodeCounter, thrown: usize, score: f64, stopped_by: Option<StoppedBy>| SearchStats {
        nodes: counter.used(),
        determinizations: scratch.determinizations,
        lines: scratch.lines,
        sim_errors: counter.sim_errors() + thrown,
        stopped_by: stopped_by.unwrap_or_else(|| counter.stopped_by()),
        score,
    };

    match outcome {
        Ok(Plan::Immediate(action, reason)) => Some(immediate(action, reason)),
        Ok(Plan::Searched { action, reason, line, score, stopped_by }) => {
            Some(Decision { action, reason, line, stats: stats(&counter, 0, score, stopped_by) })
        }
        Err(_) => {
            let action = fallback_action(&scratch.candidates);
            Some(Decision {
                line: vec![action.clone()],
                action,
                reason: DecisionReason::Fallback,
                stats: stats(&counter, 1, 0.0, None),
            })
        }
    }
}
