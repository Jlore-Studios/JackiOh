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
