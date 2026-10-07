//! The AI's mulligan (SPEC §2.1, §9.9): keep the cheap cards, send back everything costing more than
//! AI_MULLIGAN.keepMaxCost. The answer is R9-shaped — the ids kept — so `reduce` draws the
//! replacements before the returned cards are shuffled back in, exactly as for a human.
//!
//! Port of `packages/ai/src/mulligan.ts`.

use jackioh_engine::{GameState, PlayerId, find_def, query_cost};

/// R9-shaped: the instance ids to keep; returns every hand card with queryCost <= keepMaxCost
/// (callers pass AI_MULLIGAN's `keep_max_cost`, TS's default; the greedy baseline passes its own
/// frozen GREEDY_MULLIGAN's).
pub fn mulligan_keep(state: &GameState, seat: PlayerId, keep_max_cost: i32) -> Vec<String> {
    let mut keep: Vec<String> = Vec::new();
    for card in &state.players[seat].hand {
        let Some(def) = find_def(Some(state), &card.def_id) else {
            continue;
        };
        if query_cost(def) <= keep_max_cost {
            keep.push(card.id.clone());
        }
    }
    keep
}
