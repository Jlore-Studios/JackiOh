//! The AI's mulligan (SPEC §2.1, §9.9): keep the cheap cards, send back everything costing more than
//! AI_MULLIGAN.keepMaxCost plus the seat's mana bonus, within its mana cap (`mulligan_keep_bound`,
//! R2977000), and keep the cheap ones to AI_MULLIGAN_CURVE's curve (#993, `mulligan_keep_curved`),
//! ranked by the evaluation's price of a card in hand (R4110000).
//! The answer is R9-shaped — the ids kept — so `reduce` draws the replacements before the returned
//! cards are shuffled back in, exactly as for a human.
//! A card's cost here is its `mulligan_cost`, which counts a Tribute card's units (R3936000); the greedy
//! baseline's `mulligan_keep` reads mana alone.

use jackioh_engine::config::OPENING_COINS;
use jackioh_engine::{
    CardDef, CardInstance, GameState, PlayerId, find_def, handicap_of, query_cost, tribute_cost_of,
};

use crate::config::{AI_EVAL, AI_MULLIGAN_TRIBUTE_ALLOWANCE, AiMulliganCurve};
use crate::evaluate::hand_card_value;

/// R9-shaped: the instance ids to keep; returns every hand card with queryCost <= keepMaxCost, mana
/// alone: the greedy baseline's keep, at its own frozen GREEDY_MULLIGAN's bound, which no mana bonus
/// raises (R2977000) and no Tribute moves (R3936000: the AI's own keep, `mulligan_keep_plain`, reads
/// `mulligan_cost`).
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

/// R3936000: what a hand card costs the AI's mulligan: its mana cost, or, while it asks for X units to
/// sacrifice (§6.3 Tribute X, as Nerfs, Buffs and Floats left it, R386, R1841003), the larger of that and
/// X plus AI_MULLIGAN_TRIBUTE_ALLOWANCE. It reads only the card, one of the seat's own.
pub fn mulligan_cost(state: &GameState, card: &CardInstance, def: &CardDef) -> i32 {
    let cost = query_cost(def);
    let tribute = tribute_cost_of(state, card);
    if tribute <= 0 {
        return cost;
    }
    cost.max(tribute.saturating_add(AI_MULLIGAN_TRIBUTE_ALLOWANCE))
}

/// R2977000, R3936000, R4110000: exactly the hand cards whose `mulligan_cost` is at most
/// `keep_max_cost`; nothing dearer is kept. It takes the bound as given; `mulligan_keep_curved` raises
/// it first.
pub fn mulligan_keep_plain(state: &GameState, seat: PlayerId, keep_max_cost: i32) -> Vec<String> {
    let mut keep: Vec<String> = Vec::new();
    for card in &state.players[seat].hand {
        let Some(def) = find_def(Some(state), &card.def_id) else {
            continue;
        };
        if mulligan_cost(state, card, def) <= keep_max_cost {
            keep.push(card.id.clone());
        }
    }
    keep
}

/// #993: filters `mulligan_keep_plain`'s cards, priced by `mulligan_cost` (R3936000), to `curve`, ranked
/// by `hand_card_value` under AI_EVAL (R4110000):
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
    let plain = mulligan_keep_plain(state, seat, keep_max_cost);
    if !curve.on {
        return plain;
    }
    let side = &state.players[seat];
    let mut cheap: Vec<(&str, i32, f64)> = Vec::new();
    for card in &side.hand {
        if !plain.contains(&card.id) {
            continue;
        }
        let Some(def) = find_def(Some(state), &card.def_id) else {
            continue;
        };
        cheap.push((
            card.id.as_str(),
            mulligan_cost(state, card, def),
            hand_card_value(state, card, &AI_EVAL),
        ));
    }
    // R4110000: best first by the evaluation's price of a card in hand; the sort is stable, so equal
    // prices keep hand order.
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

    plain
        .into_iter()
        .filter(|id| held.iter().any(|&(kept, _)| kept == id))
        .collect()
}
