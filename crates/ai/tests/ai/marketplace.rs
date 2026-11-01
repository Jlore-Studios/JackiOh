//! The Marketplace in the AI's search (R1853004): `legal_actions` lists a buy, the move order tries it
//! after the plays, answers and Trades and before End Turn, and the AI answers a turn with one on offer
//! with a legal action. The opponent's own offers are hidden from the seat (`observe::redact`,
//! R2250002), so they never steer the decision. The state's Marketplace is written by hand: one Mr.
//! Vanilla (core-008) offer of the AI's at 40 Credits, and the AI holds 100.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, ai_options, clone, is_legal, register_cards};

const VANILLA: &str = "core-008";
const PRICE: i32 = 40;
const LIBRARY: &[&str] = &[
    "core-008", "core-005", "core-011", "core-019", "core-025", "core-020",
];

/// The AI's turn holding a Stockpile, with 100 Credits and a Marketplace offering it one Mr. Vanilla at
/// `PRICE`; the human's Mr. Vanilla on the field, so the turn has a board to weigh. `theirs` is the
/// card the human's own offer holds, ahead of the AI's in the Marketplace's list.
fn board(seed: &str, theirs: &str) -> GameState {
    priced_board(seed, theirs, PRICE)
}

/// `board` with the AI's Mr. Vanilla at `price`.
fn priced_board(seed: &str, theirs: &str, price: i32) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "p1": { "hand": ["core-005"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "field": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].market_credits = Some(100);
    let offer = |def_id: &str, owner: PlayerId| MarketOffer {
        def_id: def_id.to_string(),
        radiant: None,
        base: price,
        price,
        stock: 1,
        bought: PerPlayer::new(0, 0),
        owner: Some(owner),
        fixed: None,
    };
    state.marketplace = Some(Marketplace {
        offers: vec![offer(theirs, HUMAN), offer(VANILLA, AI)],
        quotes: Vec::new(),
    });
    state
}

#[test]
fn r1853004_a_market_buy_is_a_candidate_before_end_turn() {
    let state = board("ai-market-candidates", VANILLA);
    let buy = ActionBody::MarketBuy { offer: 0 };
    assert!(is_legal(&state, AI, &buy));
    let candidates = candidate_actions(&state, AI, Prices::NONE);
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
    let state = board("ai-market-decide", VANILLA);
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
fn r1853004_r2250002_the_opponents_offers_never_change_the_decision() {
    let a = board("ai-market-hidden", VANILLA);
    // The same position but for the human's own offer, which the AI cannot see.
    let b = board("ai-market-hidden", "core-019");
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

/// `evaluate(state, AI)` with the AI's weights.
fn eval(state: &GameState) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, &AI_EVAL)
}

#[test]
fn held_credits_are_worth_what_they_buy_on_the_own_side_only() {
    // #2389: a Credit is priced at the hand's worth of the mana it buys (R1853002).
    assert_eq!(
        AI_EVAL.market_credit,
        AI_EVAL.hand_per_cost / f64::from(MARKETPLACE_CREDITS_PER_MANA)
    );
    let held = board("ai-market-credits", VANILLA);
    let mut none = clone(&held);
    none.players[AI].market_credits = None;
    assert!((eval(&held) - eval(&none) - 100.0 * AI_EVAL.market_credit).abs() < 1e-9);
    // The human's Credits are no part of the AI's side.
    let mut theirs = clone(&held);
    theirs.players[HUMAN].market_credits = Some(100);
    assert_eq!(eval(&theirs), eval(&held));
    // The frozen baseline prices none.
    let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &GREEDY_EVAL);
    assert_eq!(greedy(&held), greedy(&none));
}

/// A price no card's place in hand is worth at `AI_EVAL.market_credit` a Credit (30 points), all the
/// Credits the AI holds.
const OVERPRICED: i32 = 1000;

#[test]
fn the_ai_keeps_its_credits_over_an_overpriced_offer() {
    let mut state = priced_board("ai-market-overpriced", VANILLA, OVERPRICED);
    state.players[AI].market_credits = Some(OVERPRICED);
    let buy = ActionBody::MarketBuy { offer: 0 };
    assert!(is_legal(&state, AI, &buy), "setup: the AI can afford it");
    for k in ["ai-market-k1", "ai-market-k2", "ai-market-k3"] {
        let decision = decide(&state, AI, &mut ai_options(k, AI_BUDGET)).expect("a decision");
        assert_ne!(decision.action, buy, "{k}: {:?}", decision.line);
    }
}

/// Stinkyscape T-gellium, a Resource (R2250000).
const GELLIUM: &str = "stinkyscape-t-gellium";

#[test]
fn a_resource_plays_for_its_quote_too() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-market-quote",
        "p1": { "hand": [GELLIUM, "core-005"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "library": LIBRARY },
    }));
    s.play(GELLIUM, json!({}));
    let played = s.state().clone();
    let quote = played.players[AI].market_credits.unwrap_or(0);
    assert!(quote > 0, "setup: the play pays its quote (R2250001)");
    let mut unpaid = clone(&played);
    unpaid.players[AI].market_credits = None;
    assert!((eval(&played) - eval(&unpaid) - f64::from(quote) * AI_EVAL.market_credit).abs() < 1e-9);
}
