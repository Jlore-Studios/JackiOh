//! The two baselines the quality gates measure the AI against (docs/polish/3-ai.md, after the
//! Hearthstone-AI Competition's random and greedy agents): SPEC §10.7's random policy, and a one-ply
//! greedy player that sees exactly what the AI sees (R185) and looks one action ahead.
//!
//! Port of `packages/ai/src/baselines.ts` (SURFACE §9).

use jackioh_engine::{ActionBody, GameState, PlayerId, Rng, mulligan_prompt_for, subsystems};

use crate::candidates::candidate_actions;
use crate::config::{AI_SEARCH, GREEDY_EVAL, GREEDY_MULLIGAN};
use crate::determinize::{DeterminizeOptions, determinize};
use crate::evaluate::{NextSwing, evaluate};
use crate::mulligan::mulligan_keep;
use crate::observe::{ai_to_act, redact, unanswered_draw_offer};
use crate::simulate::{create_node_counter, simulate};

/// §10.7's random policy for `seat` (subsystems::choose_action with AI_SKIPPED_ACTIONS, its default
/// skip set, which TS passed explicitly).
pub fn random_action(state: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    subsystems::choose_action(state, seat, rng)
}

/// One-ply greedy on one determinization of redact(state, seat): mulligan → mulligan_keep; draw offer
/// → decline; else the candidate (not endTurn) with the highest `evaluate` (with its own frozen
/// GREEDY_EVAL weights) after `simulate`, or endTurn when none beats standing still (prompts: the
/// best answer). `None` if !ai_to_act.
pub fn greedy_action(state: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    if !ai_to_act(state, seat) {
        return None;
    }

    let public = redact(state, seat);
    if unanswered_draw_offer(&public, seat) {
        return Some(ActionBody::AnswerDraw { accept: false });
    }
    if public.pending.is_none() && mulligan_prompt_for(&public, seat).is_some() {
        return Some(ActionBody::Mulligan { keep: mulligan_keep(&public, seat, GREEDY_MULLIGAN.keep_max_cost) });
    }

    // The only draws greedy takes from its rng: one determinization per decision. R762's shown-cost match
    // stays off: the gates were fixed on this sampler, as on GREEDY_EVAL, so the yardstick does not move.
    let det = determinize(&public, seat, rng, DeterminizeOptions { match_shown_cost: Some(false) });
    let candidates = candidate_actions(&det, seat);
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return candidates.into_iter().next();
    }

    let answering = det.pending.as_ref().is_some_and(|pending| pending.player_id == seat);
    // Every candidate gets its one simulation, auto-answers of the other seat's prompts included.
    let counter = create_node_counter(candidates.len() * (1 + AI_SEARCH.max_auto_answers as usize), None);

    let mut best: Option<ActionBody> = None;
    let mut best_score = f64::NEG_INFINITY;
    for action in &candidates {
        if matches!(action, ActionBody::EndTurn) {
            continue;
        }
        let Some(Ok(next)) = simulate(&det, seat, action, &counter) else {
            continue;
        };
        let score = evaluate(&next, seat, NextSwing::Enemy, &GREEDY_EVAL);
        // Strictly greater, so a tie keeps the earlier candidate in move order.
        if score > best_score {
            best = Some(action.clone());
            best_score = score;
        }
    }

    if answering {
        return best.or_else(|| candidates.first().cloned());
    }

    let Some(end_turn) = candidates.iter().find(|action| matches!(action, ActionBody::EndTurn)).cloned() else {
        return best.or_else(|| candidates.first().cloned());
    };
    if best.is_some() && best_score > evaluate(&det, seat, NextSwing::Enemy, &GREEDY_EVAL) {
        return best;
    }
    Some(end_turn)
}
