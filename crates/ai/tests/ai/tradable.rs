//! The Trade in the AI's search (R1838003): `legal_actions` lists it, the move order tries it after the plays
//! and answers and before the other attacks, and the AI may trade or not. Every line runs on a determinized
//! copy whose deck order is resampled (R185), so the card a Trade would draw never steers the decision. The
//! cards: Meditative #98 Showdown, Tradable, which the AI cannot play at 0 mana; Mr. Vanilla and Stockpile in
//! the decks.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, is_legal, register_cards};

const SHOWDOWN: &str = "meditative-098";
const LIBRARY: &[&str] = &[
    "core-008", "core-005", "core-011", "core-019", "core-025", "core-020",
];

/// The AI's turn with no mana, holding Showdown and a Stockpile over `LIBRARY`; the human's Mr.
/// Vanilla on the field, so the turn has a board to weigh.
fn board(seed: &str) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "mana": 0, "hand": [SHOWDOWN, "core-005"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "field": ["core-008"], "library": LIBRARY },
    }))
    .state()
    .clone()
}

fn showdown_of(state: &GameState) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == SHOWDOWN)
        .map(|card| card.id.clone())
        .expect("Showdown in the AI's hand")
}

#[test]
fn r1838003_a_trade_is_a_candidate_after_the_plays_and_before_end_turn() {
    let state = board("ai-trade-candidates");
    let trade = ActionBody::Trade {
        instance_id: showdown_of(&state),
    };
    let candidates = candidate_actions(&state, AI, Prices::NONE);
    let at = candidates
        .iter()
        .position(|action| *action == trade)
        .expect("the Trade is a candidate");
    let end = candidates
        .iter()
        .position(|action| *action == ActionBody::EndTurn)
        .expect("End Turn is a candidate");
    assert!(at < end, "the Trade is tried before End Turn: {candidates:?}");
}

#[test]
fn r1838003_the_ai_answers_a_turn_with_a_trade_on_offer_with_a_legal_action() {
    let state = board("ai-trade-decide");
    for k in ["ai-trade-k1", "ai-trade-k2"] {
        let decision = decide(&state, AI, &mut ai_options(k, AI_BUDGET)).expect("a decision");
        assert!(
            is_legal(&state, AI, &decision.action),
            "{k}: {:?}",
            decision.action
        );
        assert_ne!(decision.reason, DecisionReason::Fallback, "{k}");
        assert_eq!(decision.stats.sim_panics, 0, "{k}");
    }
}

#[test]
fn r1838003_the_card_a_trade_would_draw_never_changes_the_decision() {
    let a = board("ai-trade-hidden");
    // The same position but for the AI's own deck order, which it cannot see (R185).
    let mut b = a.clone();
    b.players[AI].library.reverse();
    assert_ne!(
        a.players[AI].library.first().map(|card| card.def_id.clone()),
        b.players[AI].library.first().map(|card| card.def_id.clone()),
        "the two decks would deal a Trade different cards"
    );
    for k in ["ai-trade-k1", "ai-trade-k2", "ai-trade-k3"] {
        assert_eq!(
            decide(&a, AI, &mut ai_options(k, AI_BUDGET)),
            decide(&b, AI, &mut ai_options(k, AI_BUDGET)),
            "{k}"
        );
    }
}

/// Meditative #11 Double Header: a (4) Field Spell, Tradable.
const DOUBLE_HEADER: &str = "meditative-011";

/// The AI's turn holding Double Header and a Mr. Vanilla with 1 mana, at `max` crystals, its started turns
/// set to match; the human's Mr. Vanilla on the field. At 1 crystal neither this turn's nor the next
/// refresh's pay for Double Header.
fn dead_board(seed: &str, max: i32) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "p1": { "mana": 1, "hand": [DOUBLE_HEADER, "core-008"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "field": ["core-008"], "library": LIBRARY },
    }))
    .state()
    .clone();
    // The crystal count the refreshes set the maximum from (R1854003) moves with it, so the next refresh
    // brings `max` + 1 and not the scenario's 4 back.
    let side = &mut state.players[AI];
    side.turns_started += max - jackioh_engine::mana_crystal_count(side);
    side.mana.max = max;
    state
}

fn double_header_of(state: &GameState) -> CardInstance {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == DOUBLE_HEADER)
        .cloned()
        .expect("Double Header in the AI's hand")
}

#[test]
fn r4154400_a_tradable_card_no_coming_crystals_pay_for_counts_at_a_discount() {
    let dead = dead_board("ai-trade-dead-eval", 1);
    let header = double_header_of(&dead);
    assert!(dead_in_hand(&dead, AI, &header));
    let blind = EvalWeights {
        dead_tradable_discount: 0.0,
        ..AI_EVAL
    };
    let score = |state: &GameState, w: &EvalWeights| evaluate(state, AI, NextSwing::Enemy, w);
    let cost =
        f64::from(jackioh_engine::own_cost(&dead, &header).expect("a cost")).min(AI_EVAL.hand_cost_cap);
    let price = AI_EVAL.hand_card + AI_EVAL.hand_per_cost * cost;
    let lost = score(&dead, &blind) - score(&dead, &AI_EVAL);
    assert!(
        (lost - AI_EVAL.dead_tradable_discount * price).abs() < 1e-9,
        "{lost} vs {}",
        AI_EVAL.dead_tradable_discount * price
    );
    // At 3 crystals the next refresh brings 4, which pays for it: no discount.
    let live = dead_board("ai-trade-dead-eval", 3);
    assert!(!dead_in_hand(&live, AI, &double_header_of(&live)));
    assert_eq!(score(&live, &blind), score(&live, &AI_EVAL));
}

#[test]
fn r4154400_the_trade_of_a_dead_card_is_tried_before_the_plays() {
    let position_of = |state: &GameState| {
        let header = double_header_of(state).id;
        let candidates = candidate_actions(state, AI, Prices::NONE);
        let trade = candidates
            .iter()
            .position(|action| matches!(action, ActionBody::Trade { instance_id } if *instance_id == header))
            .expect("the Trade is a candidate");
        let play = candidates
            .iter()
            .position(|action| matches!(action, ActionBody::Play { .. }))
            .expect("a play is a candidate");
        (trade, play)
    };
    let (trade, play) = position_of(&dead_board("ai-trade-dead-order", 1));
    assert!(trade < play, "the dead card's Trade first: {trade} vs {play}");
    // A card the next refresh pays for keeps its place after the plays (R1838003).
    let (trade, play) = position_of(&dead_board("ai-trade-dead-order", 3));
    assert!(trade > play, "after the plays: {trade} vs {play}");
}

#[test]
fn r4154400_the_ai_trades_a_card_it_cannot_play_before_its_turn_after_next() {
    let state = dead_board("ai-trade-dead-turn", 1);
    let header = double_header_of(&state).id;
    for k in ["ai-trade-dead-k1", "ai-trade-dead-k2"] {
        let turn = play_ai_turn(&state, AI, &mut ai_options(k, AI_BUDGET));
        assert!(
            turn.decisions.iter().any(
                |decision| matches!(&decision.action, ActionBody::Trade { instance_id } if *instance_id == header)
            ),
            "{k}: {:?}",
            turn.decisions.iter().map(|decision| &decision.action).collect::<Vec<_>>()
        );
    }
}

/// R4812000: `state` with the AI's crystal count the refreshes set the maximum from (R1854003) at `crystals`,
/// `perm_mod` on its maximum for the rest of the game, `stolen` crystals past the cap (R4331001) and Double
/// Header's cost moved by `cost_mod`, the maximum set as the engine sets it (`max_mana_for`).
fn lowered(state: &GameState, crystals: i32, perm_mod: i32, stolen: i32, cost_mod: i32) -> GameState {
    let mut state = state.clone();
    let side = &mut state.players[AI];
    side.turns_started += crystals - jackioh_engine::mana_crystal_count(side);
    side.mana.perm_mod = perm_mod;
    side.stolen_mana_crystals = Some(stolen).filter(|stolen| *stolen != 0);
    side.mana.max = jackioh_engine::max_mana_for(side);
    side.mana.current = side.mana.current.min(side.mana.max);
    side.hand
        .iter_mut()
        .find(|card| card.def_id == DOUBLE_HEADER)
        .expect("Double Header in the AI's hand")
        .cost_mod = cost_mod;
    state
}

/// What the engine's next refresh brings the AI of `state`: its maximum once one more turn has started.
fn next_refresh(state: &GameState) -> i32 {
    let mut next = state.players[AI].clone();
    next.turns_started += 1;
    jackioh_engine::max_mana_for(&next)
}

#[test]
fn r4812000_a_maximum_lowered_for_the_rest_of_the_game_at_the_cap_never_pays_for_a_dead_card() {
    // "Lose 1 mana for the rest of the game" (S+ #95 JC Backed, S+ #82 Endless Expedition) at the cap.
    let state = lowered(&dead_board("ai-trade-lowered", 1), 5, -1, 0, 0);
    let header = double_header_of(&state);
    assert_eq!(state.players[AI].mana.max, 3);
    assert_eq!(
        next_refresh(&state),
        3,
        "the maximum stays at 3 for the rest of the game"
    );
    assert_eq!(jackioh_engine::own_cost(&state, &header), Some(4));
    assert!(jackioh_engine::is_tradable(&state, &header));
    assert!(
        dead_in_hand(&state, AI, &header),
        "R4812000: its (4) is above the 3 its next refresh brings"
    );
}

#[test]
fn r4812000_crystals_stolen_past_the_cap_leave_the_next_refresh_growing() {
    // 2 crystals of its own and 2 stolen past the cap: the maximum is 4 and the next refresh brings 5.
    let state = lowered(&dead_board("ai-trade-stolen", 1), 2, 0, 2, 1);
    let header = double_header_of(&state);
    assert_eq!(state.players[AI].mana.max, 4);
    assert_eq!(next_refresh(&state), 5);
    assert_eq!(jackioh_engine::own_cost(&state, &header), Some(5));
    assert!(
        !dead_in_hand(&state, AI, &header),
        "R4812000: the 5 its next refresh brings pays for its (5)"
    );
}

#[test]
fn r4812000_the_dead_reading_agrees_with_the_engines_next_refresh() {
    let base = dead_board("ai-trade-grid", 1);
    for crystals in 1..=6 {
        for perm_mod in [0, -1, -2] {
            for stolen in [0, 1, 2] {
                for cost_mod in [-2, -1, 0, 1, 2] {
                    // A maximum §2.3 floors at 0 is left unmodelled (R4812000).
                    if crystals.min(jackioh_engine::config::MAX_MANA) + stolen + perm_mod < 0 {
                        continue;
                    }
                    let state = lowered(&base, crystals, perm_mod, stolen, cost_mod);
                    let header = double_header_of(&state);
                    let cost = jackioh_engine::own_cost(&state, &header).expect("a cost");
                    assert_eq!(
                        dead_in_hand(&state, AI, &header),
                        cost > next_refresh(&state),
                        "{crystals} crystals, {perm_mod} for the game, {stolen} stolen, cost {cost}"
                    );
                }
            }
        }
    }
}
