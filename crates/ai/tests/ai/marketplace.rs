//! The Marketplace in the AI's search (R1853004): `legal_actions` lists a buy, the move order tries it after
//! the plays, answers and Trades and before End Turn, and the AI answers a turn with one on offer with a
//! legal action. The opponent's own offers are hidden from the seat (`observe::redact`, R2250002), so they
//! never steer the decision. The state's Marketplace is written by hand: one Mr. Vanilla offer of the AI's at
//! 40 Credits, and the AI holds 100.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, ai_options, clone, close, is_legal, register_cards};

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
        buffs: None,
        nerfs: None,
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

/// Stinkyscape #89 Spice: a (1) Spell that pays its quote in Credits as it is played or discarded, as a
/// Resource does, and no longer sells from the hand (R4410000).
const SPICE: &str = "stinkyscape-089";
/// The Credits the Marketplace quotes Spice at in `spice_board`, its base face's.
const SPICE_QUOTE: i32 = 8;
/// The Credits `spice_board`'s Mr. Vanilla asks, a (1) card's (R3047000).
const SPICE_OFFER_PRICE: i32 = 16;
/// The Credits the AI holds in `spice_board`: short of the offer alone, enough with Spice's quote paid.
const SPICE_CREDITS: i32 = SPICE_OFFER_PRICE - SPICE_QUOTE;

/// The AI's turn holding Spice with `mana` mana and `SPICE_CREDITS` Credits; the Marketplace quotes Spice
/// at `SPICE_QUOTE` and offers the AI one Mr. Vanilla at `SPICE_OFFER_PRICE`. The human's Mr. Vanilla on
/// the field.
fn spice_board(seed: &str, mana: i32) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": seed,
        "p1": { "mana": mana, "hand": [SPICE], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "field": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].market_credits = Some(SPICE_CREDITS);
    state.marketplace = Some(Marketplace {
        offers: vec![MarketOffer {
            def_id: VANILLA.to_string(),
            radiant: None,
            base: SPICE_OFFER_PRICE,
            price: SPICE_OFFER_PRICE,
            stock: 1,
            bought: PerPlayer::new(0, 0),
            owner: Some(AI),
            fixed: None,
            buffs: None,
            nerfs: None,
        }],
        quotes: vec![jackioh_engine::ResourceQuote {
            def_id: SPICE.to_string(),
            base: SPICE_QUOTE,
            radiant: 2 * SPICE_QUOTE,
        }],
    });
    state
}

/// `AI_EVAL` with R4440002's discount off: the weights Spice was priced at before issue #4440.
const UNDISCOUNTED: EvalWeights = EvalWeights {
    credits_card_discount: 0.0,
    ..AI_EVAL
};

#[test]
fn r4410001_r4154401_r4440002_spice_in_hand_is_priced_as_a_1_card_at_the_credits_card_discount() {
    let held = spice_board("ai-market-spice-eval", 0);
    let mut gone = clone(&held);
    gone.players[AI].hand.retain(|card| card.def_id != SPICE);
    let price = AI_EVAL.hand_card + AI_EVAL.hand_per_cost;
    close(
        eval(&held) - eval(&gone),
        price * (1.0 - AI_EVAL.credits_card_discount),
    );
    // Without the discount, any (1) card's price (R4410001).
    let undiscounted = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &UNDISCOUNTED);
    close(undiscounted(&held) - undiscounted(&gone), price);
    // Its quote is no sale on offer: none is listed, and one is refused (R4410000).
    let spice = held.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == SPICE)
        .map(|card| card.id.clone())
        .expect("Spice in hand");
    let sell = ActionBody::MarketSell { instance_id: spice };
    assert!(!is_legal(&held, AI, &sell));
    let out = reduce(&held, &Action::new(sell, AI, "ai-market-spice-sell"));
    assert_eq!(out.error.as_deref(), Some(NOTHING_SELLS));
}

#[test]
fn r4410001_the_credits_spice_pays_as_it_is_cast_count_at_market_credit() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-market-spice-quote",
        "p1": { "hand": [SPICE, "core-005"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "library": LIBRARY },
    }));
    s.play(SPICE, json!({}));
    let pick = s.prompt_modes()[0].clone();
    s.answer(json!(pick));
    let played = s.state().clone();
    let quote = played.players[AI].market_credits.unwrap_or(0);
    assert!(
        (HAND_SALE_PRICE.sell_min..=HAND_SALE_PRICE.sell_max).contains(&quote),
        "setup: the play pays its quote (R4410000): {quote}"
    );
    let mut unpaid = clone(&played);
    unpaid.players[AI].market_credits = None;
    close(
        eval(&played) - eval(&unpaid),
        f64::from(quote) * AI_EVAL.market_credit,
    );
}

#[test]
fn r4410001_the_ai_casts_spice_it_can_afford() {
    let state = spice_board("ai-market-spice-turn", 1);
    let spice = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == SPICE)
        .map(|card| card.id.clone())
        .expect("Spice in hand");
    for k in ["ai-market-spice-k1", "ai-market-spice-k2"] {
        let turn = play_ai_turn(&state, AI, &mut ai_options(k, AI_BUDGET));
        let line: Vec<&ActionBody> = turn.decisions.iter().map(|decision| &decision.action).collect();
        assert!(
            line.iter().any(
                |action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == spice)
            ),
            "{k}: {line:?}"
        );
        assert!(
            line.iter()
                .all(|action| action.action_type() != ActionType::MarketSell),
            "{k}: {line:?}"
        );
    }
}

/// The id of the Spice in the AI's hand on `state`.
fn spice_in_hand(state: &GameState) -> String {
    state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == SPICE)
        .map(|card| card.id.clone())
        .expect("Spice in hand")
}

/// R4440002: the AI's turn holding one Vanilla Spice, its Credits kept (R4410000),
/// with one crystal, the Marketplace open, quoting it `SPICE_QUOTE` and offering the AI nothing, and no
/// Credits held: its cast is the sale alone, `SPICE_QUOTE` Credits for the card and the crystal. The
/// human's Mr. Vanilla on the field.
fn sale_board(seed: &str) -> GameState {
    let mut state = spice_board(seed, 1);
    state.players[AI].market_credits = None;
    for card in state.players[AI].hand.iter_mut() {
        card.vanilla = true;
    }
    if let Some(marketplace) = state.marketplace.as_mut() {
        marketplace.offers.clear();
    }
    state
}

#[test]
fn r4440002_a_spice_cast_for_its_credits_alone_read_as_a_loss_before_the_discount_and_reads_as_a_gain() {
    let held = sale_board("ai-market-spice-sale");
    let spice = spice_in_hand(&held);
    let play = legal_actions(&held, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == spice))
        .expect("the Spice's play is listed");
    let out = reduce(&held, &Action::new(play, AI, "ai-market-spice-sale-play"));
    assert_eq!(out.error, None);
    // Played, the Spice's Cry still opens its Discover; the pick is not the sale's, so the prompt is
    // dropped unanswered and the cast reads the sale alone.
    let mut cast = out.state;
    cast.pending = None;
    assert_eq!(
        cast.players[AI].market_credits,
        Some(SPICE_QUOTE),
        "setup: the play pays the quote"
    );
    // What the line that casts it gains over the line that holds it: the evaluation's move, and the
    // crystal it spends, which the held line ends the turn with (`unspent_mana`).
    let gain = |w: &EvalWeights| {
        evaluate(&cast, AI, NextSwing::Enemy, w) - evaluate(&held, AI, NextSwing::Enemy, w) + w.unspent_mana
    };
    assert!(
        gain(&UNDISCOUNTED) < 0.0,
        "before issue #4440: {}",
        gain(&UNDISCOUNTED)
    );
    assert!(gain(&AI_EVAL) > 0.0, "since: {}", gain(&AI_EVAL));
    close(
        gain(&AI_EVAL) - gain(&UNDISCOUNTED),
        AI_EVAL.credits_card_discount * (AI_EVAL.hand_card + AI_EVAL.hand_per_cost),
    );
}

#[test]
fn r4440002_the_ai_casts_a_spice_that_brings_only_its_credits() {
    let state = sale_board("ai-market-spice-sale-turn");
    let spice = spice_in_hand(&state);
    for k in ["ai-market-spice-sale-k1", "ai-market-spice-sale-k2"] {
        let turn = play_ai_turn(&state, AI, &mut ai_options(k, AI_BUDGET));
        let line: Vec<&ActionBody> = turn.decisions.iter().map(|decision| &decision.action).collect();
        assert!(
            line.iter().any(
                |action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == spice)
            ),
            "{k}: {line:?}"
        );
    }
}
