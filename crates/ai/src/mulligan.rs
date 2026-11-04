//! The AI's mulligan (SPEC §2.1, §9.9): keep the cheap cards, send back everything costing more than
//! AI_MULLIGAN.keepMaxCost plus the seat's mana bonus, within its mana cap (`mulligan_keep_bound`,
//! R2977000), and keep the cheap ones to AI_MULLIGAN_CURVE's curve (#993, `mulligan_keep_curved`).
//! The answer is R9-shaped — the ids kept — so `reduce` draws the replacements before the returned
//! cards are shuffled back in, exactly as for a human.

use jackioh_engine::config::OPENING_COINS;
use jackioh_engine::{GameState, PlayerId, find_def, handicap_of, query_cost};

use crate::config::AiMulliganCurve;
use crate::shadow_ban::{DEAL_MULLIGAN_KEEP, shaped_weight};

/// R9-shaped: the instance ids to keep; returns every hand card with queryCost <= keepMaxCost
/// (`mulligan_keep_curved` passes `mulligan_keep_bound`'s, R2977000; the greedy baseline passes its
/// own frozen GREEDY_MULLIGAN's, which no mana bonus raises).
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

/// R2977000: the dearest card `seat`'s mulligan keeps by the plain rule. `keep_max_cost` is the bound
/// of a seat with no mana bonus (a human's turn-3 play); the seat's `mana_bonus` (R181) gives it that
/// many more crystals every turn, so the bound moves up by it, never past the seat's `mana_cap` (no
/// turn's mana passes it) and never below 0. It reads the seat's handicap, never its tier.
pub fn mulligan_keep_bound(state: &GameState, seat: PlayerId, keep_max_cost: i32) -> i32 {
    let handicap = handicap_of(&state.players[seat]);
    keep_max_cost
        .saturating_add(handicap.mana_bonus)
        .min(handicap.mana_cap)
        .max(0)
}

/// `mulligan_keep` plus, at most, the one best card costing exactly `keep_max_cost + 1` whose
/// `shaped_weight` reaches DEAL_MULLIGAN_KEEP: a deck the lane's shaping dealt is deep enough that a
/// proven card one dearer than the bound is worth a slot the plain rule would send back. A seat whose
/// mana cap can never reach the card (the tutorial's three, R290) keeps nothing extra: the card would
/// be dead weight. It takes the bound as given; `mulligan_keep_curved` raises it first, so the
/// exception moves up with it (R2977000).
pub fn mulligan_keep_shaped(state: &GameState, seat: PlayerId, keep_max_cost: i32) -> Vec<String> {
    let mut keep = mulligan_keep(state, seat, keep_max_cost);
    let mana_cap = handicap_of(&state.players[seat]).mana_cap;
    let mut best: Option<(f64, &str)> = None;
    for card in &state.players[seat].hand {
        let Some(def) = find_def(Some(state), &card.def_id) else {
            continue;
        };
        if query_cost(def) != keep_max_cost.saturating_add(1) || query_cost(def) > mana_cap {
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

/// #993: filters `mulligan_keep_shaped` cheap cards to `curve` by `shaped_weight`:
/// caps first-turn plays and requires early curve targets accounting for bonus mana (R181)
/// and The Coin (R244). `keep_max_cost` is a seat with no bonus's bound: `mulligan_keep_bound` raises
/// it by the seat's mana bonus before anything reads it (R2977000).
pub fn mulligan_keep_curved(
    state: &GameState,
    seat: PlayerId,
    keep_max_cost: i32,
    curve: &AiMulliganCurve,
) -> Vec<String> {
    let keep_max_cost = mulligan_keep_bound(state, seat, keep_max_cost);
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
    let early_max_cost = curve
        .early_turn
        .saturating_add(handicap_of(side).mana_bonus)
        .saturating_add(coins);
    if !held.iter().any(|&(_, cost)| cost <= early_max_cost) {
        held.sort_by_key(|&(_, cost)| cost);
        held.truncate(curve.keep_without_early);
    }

    shaped
        .into_iter()
        .filter(|id| held.iter().any(|&(kept, _)| kept == id) || !cheap.iter().any(|&(cut, ..)| cut == id))
        .collect()
}
