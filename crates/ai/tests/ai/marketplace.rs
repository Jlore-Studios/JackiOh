//! The Marketplace in the AI's search (R1853004): `legal_actions` lists a buy, the move order tries it
//! after the plays, answers and Trades and before End Turn, and the AI answers a turn with one on offer
//! with a legal action. What the opponent bought this cycle is hidden from the seat (`observe::redact`),
//! so it never steers the decision. No shipped card opens the Marketplace yet, so the state's is
//! written by hand: one Mr. Vanilla (core-008) lot at 40 Credits, and the AI holds 100.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, ai_options, is_legal, register_cards};

const VANILLA: &str = "core-008";
const PRICE: i32 = 40;
const LIBRARY: &[&str] = &[
    "core-008", "core-005", "core-011", "core-019", "core-025", "core-020",
];

/// The AI's turn holding a Stockpile, with 100 Credits and a Marketplace offering one Mr. Vanilla at
/// `PRICE`; the human's Mr. Vanilla on the field, so the turn has a board to weigh. `bought` is what
/// the human has bought of the lot this cycle.
fn board(seed: &str, bought: i32) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "p1": { "hand": ["core-005"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "field": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].market_credits = Some(100);
    let mut bought_by = PerPlayer::new(0, 0);
    bought_by[HUMAN] = bought;
    state.marketplace = Some(Marketplace {
        rolled_turn: state.turn,
        offers: vec![MarketOffer {
            def_id: VANILLA.to_string(),
            radiant: None,
            base: PRICE,
            price: PRICE,
            stock: 3,
            bought: bought_by,
            owner: None,
        }],
        quotes: Vec::new(),
    });
    state
}

#[test]
fn r1853004_a_market_buy_is_a_candidate_before_end_turn() {
    let state = board("ai-market-candidates", 0);
    let buy = ActionBody::MarketBuy { offer: 0 };
    assert!(is_legal(&state, AI, &buy));
    let candidates = candidate_actions(&state, AI);
    let at = candidates
        .iter()
        .position(|action| *action == buy)
        .expect("the buy is a candidate");
    let end = candidates
        .iter()
        .position(|action| *action == ActionBody::EndTurn)
        .expect("End Turn is a candidate");
    assert!(at < end, "the buy is tried before End Turn: {candidates:?}");
}

#[test]
fn r1853004_the_ai_answers_with_a_legal_action() {
    let state = board("ai-market-decide", 0);
    for k in ["ai-market-k1", "ai-market-k2"] {
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
fn r1853004_what_the_opponent_bought_never_changes_the_decision() {
    let a = board("ai-market-hidden", 0);
    // The same position but for what the human bought this cycle, which the AI cannot see.
    let b = board("ai-market-hidden", 2);
    assert_ne!(a.marketplace, b.marketplace);
    for k in ["ai-market-k1", "ai-market-k2", "ai-market-k3"] {
        assert_eq!(
            decide(&a, AI, &mut ai_options(k, AI_BUDGET)),
            decide(&b, AI, &mut ai_options(k, AI_BUDGET)),
            "{k}"
        );
    }
    assert_eq!(redact(&b, AI).marketplace, redact(&a, AI).marketplace);
}
