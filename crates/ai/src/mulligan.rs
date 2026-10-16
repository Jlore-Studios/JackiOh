//! The AI's mulligan (SPEC §2.1, §9.9): keep the cheap cards, send back everything costing more than
//! AI_MULLIGAN.keepMaxCost, and keep the cheap ones to AI_MULLIGAN_CURVE's curve (#993,
//! `mulligan_keep_curved`). The answer is R9-shaped — the ids kept — so `reduce` draws the
//! replacements before the returned cards are shuffled back in, exactly as for a human.
//!
//! Port of `packages/ai/src/mulligan.ts`.

use jackioh_engine::config::OPENING_COINS;
use jackioh_engine::{GameState, PlayerId, find_def, handicap_of, query_cost};

use crate::config::AiMulliganCurve;
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

/// #993: `mulligan_keep_shaped` with its cheap cards (those costing at most `keep_max_cost`) kept to
/// `curve`, best first by `shaped_weight` (hand order breaking ties):
///
/// 1. at most `curve.max_first_turn` first-turn plays (costing at most `curve.first_turn_max_cost`),
///    so a hand of one-drops sends the rest back for a turn-2 or turn-3 play;
/// 2. if none of what is left is an early play (costing at most `curve.early_turn` plus the seat's
///    mana bonus, R181, and its copies of The Coin, R244, which the seat going second is dealt after
///    the mulligan), only the `curve.keep_without_early` cheapest, so the redraw digs for one.
///
/// The shaped keep's cost-4 card stays either way. With `curve.on` false this is
/// `mulligan_keep_shaped` exactly. No rng: the same state keeps the same ids, in the shaped keep's order.
pub fn mulligan_keep_curved(
    state: &GameState,
    seat: PlayerId,
    keep_max_cost: i32,
    curve: &AiMulliganCurve,
) -> Vec<String> {
    let shaped = mulligan_keep_shaped(state, seat, keep_max_cost);
    if !curve.on {
        return shaped;
    }
    let side = &state.players[seat];
    let mut cheap: Vec<(&str, i32, f64)> = Vec::new();
    for card in &side.hand {
        if !shaped.contains(&card.id) {
            continue;
        }
        let Some(def) = find_def(Some(state), &card.def_id) else {
            continue;
        };
        let cost = query_cost(def);
        if cost <= keep_max_cost {
            cheap.push((card.id.as_str(), cost, shaped_weight(&card.def_id)));
        }
    }
    // Best first; the sort is stable, so equal weights keep hand order.
    cheap.sort_by(|a, b| b.2.total_cmp(&a.2));

    let mut held: Vec<(&str, i32)> = Vec::new();
    let mut first_turn = 0;
    for &(id, cost, _) in &cheap {
        if cost <= curve.first_turn_max_cost {
            if first_turn == curve.max_first_turn {
                continue;
            }
            first_turn += 1;
        }
        held.push((id, cost));
    }

    let coins = OPENING_COINS.get(seat.seat()).copied().unwrap_or(0);
    let early_max_cost = curve.early_turn + handicap_of(side).mana_bonus + coins;
    if !held.iter().any(|&(_, cost)| cost <= early_max_cost) {
        held.sort_by_key(|&(_, cost)| cost);
        held.truncate(curve.keep_without_early);
    }

    shaped
        .into_iter()
        .filter(|id| held.iter().any(|&(kept, _)| kept == id) || !cheap.iter().any(|&(cut, ..)| cut == id))
        .collect()
}
