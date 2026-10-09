//! The AI's mulligan (SPEC §2.1, §9.9): keep the cheap cards, send back everything costing more than
//! AI_MULLIGAN.keepMaxCost. The answer is R9-shaped — the ids kept — so `reduce` draws the
//! replacements before the returned cards are shuffled back in, exactly as for a human.
//!
//! Port of `packages/ai/src/mulligan.ts`.

use jackioh_engine::{GameState, PlayerId, find_def, handicap_of, query_cost};

use crate::shadow_ban::{DEAL_MULLIGAN_KEEP, shaped_weight};

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

/// `mulligan_keep` plus, at most, the one best card costing exactly `keep_max_cost + 1` whose
/// `shaped_weight` reaches DEAL_MULLIGAN_KEEP: a deck the lane's shaping dealt is deep enough that a
/// proven four-drop is worth a slot the plain rule would send back. A seat whose mana cap can never
/// reach the card (the tutorial's three, R290) keeps nothing extra: the card would be dead weight.
pub fn mulligan_keep_shaped(state: &GameState, seat: PlayerId, keep_max_cost: i32) -> Vec<String> {
    let mut keep = mulligan_keep(state, seat, keep_max_cost);
    let mana_cap = handicap_of(&state.players[seat]).mana_cap;
    let mut best: Option<(f64, &str)> = None;
    for card in &state.players[seat].hand {
        let Some(def) = find_def(Some(state), &card.def_id) else {
            continue;
        };
        if query_cost(def) != keep_max_cost + 1 || query_cost(def) > mana_cap {
            continue;
        }
        let weight = shaped_weight(&card.def_id);
        if weight < DEAL_MULLIGAN_KEEP {
            continue;
        }
        if best.is_none_or(|(held, _)| weight > held) {
            best = Some((weight, card.id.as_str()));
        }
    }
    if let Some((_, id)) = best {
        keep.push(id.to_string());
    }
    keep
}
