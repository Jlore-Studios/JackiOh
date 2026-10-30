//! A unit's Death in the AI's evaluation (R2424000): at a decision's root the AI prices each Death it
//! can read, on either field and among its own hand's Units (placed first), as its controller's
//! evaluation after the sandbox's destroy less after its remove (R923003); `unit_worth` adds
//! `AI_EVAL.death` of the price and `kill_worth` takes the whole of it off a kill, so a unit that comes
//! back bigger is worth more alive and less to kill, and one whose Death hurts its owner is worth less.
//! The frozen baseline reads no price, and none is held outside a decision.
//!
//! The cards: Trigor (traininggrounds-025), (3) 3/3 Tribute 3, "Death: Summon a Trigor with this one's
//! base stats +3/+3"; Floodgates (traininggrounds-030), (3) 0/12 Armor 3, "Death: Destroy all your
//! Units"; Training Dummy (traininggrounds-036), (1) 0/3, "Death: Add 1 random Radiant Training Grounds
//! card to your hand"; and Mr. Vanilla (core-008), a (1) 4/4 with no text.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const TRIGOR: &str = "traininggrounds-025";
const FLOODGATES: &str = "traininggrounds-030";
const TRAINING_DUMMY: &str = "traininggrounds-036";
const VANILLA: &str = "core-008";

/// The AI's turn with `hand` and `field`, the human's `enemy` and a Mr. Vanilla in the human's hand.
fn board(seed: &str, hand: Value, field: Value, enemy: Value) -> GameState {
    register_cards();
    let library = json!([VANILLA, VANILLA, VANILLA, VANILLA]);
    scenario(json!({
        "seed": seed,
        "p1": { "hand": hand, "field": field, "library": library },
        "p2": { "hand": [VANILLA], "field": enemy, "library": library },
    }))
    .state()
    .clone()
}

/// The AI's Death prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<DeathPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = death_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `player`'s unit of `def_id` on the field.
fn unit<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .expect("the unit is on the field")
}

/// The price of `def_id`'s Death, which must be priced.
fn price_of(prices: &[DeathPrice], def_id: &str) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{def_id}'s Death is priced: {prices:?}"))
}

/// A 3/3 with no text in Attack Position, as `unit_worth` counts it.
fn vanilla_three_three() -> f64 {
    3.0 * AI_EVAL.attack + 3.0 * AI_EVAL.health
}

#[test]
fn r2424000_a_trigor_on_the_ai_board_is_worth_more_than_a_vanilla_3_3() {
    let state = board("ai-death-trigor", json!([]), json!([TRIGOR]), json!([]));
    let trigor = unit(&state, AI, TRIGOR);
    let body = unit_worth(&state, trigor, &AI_EVAL);
    assert!(
        (body - vanilla_three_three()).abs() < 1e-9,
        "unpriced, a Trigor is its 3/3 body: {body}"
    );
    let (prices, _) = priced(&state);
    let price = price_of(&prices, TRIGOR);
    assert!(
        price > vanilla_three_three(),
        "its Death leaves a 6/6, more than a 3/3: {price}"
    );
    let worth = with_death_prices(prices, || unit_worth(&state, trigor, &AI_EVAL));
    assert!((worth - (body + AI_EVAL.death * price)).abs() < 1e-9, "{worth}");
    assert!(worth > vanilla_three_three(), "{worth}");
}

#[test]
fn r2424000_killing_an_enemy_trigor_is_worth_less_than_killing_a_vanilla_3_3() {
    let state = board("ai-death-kill", json!([]), json!([VANILLA]), json!([TRIGOR]));
    let trigor = unit(&state, HUMAN, TRIGOR);
    let unpriced_kill = kill_worth(&state, trigor, &AI_EVAL);
    assert!(
        (unpriced_kill - vanilla_three_three()).abs() < 1e-9,
        "unpriced, killing a Trigor takes its 3/3 body: {unpriced_kill}"
    );
    let unpriced_eval = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > 0.0);
    let (kill, eval) = with_death_prices(prices, || {
        (
            kill_worth(&state, trigor, &AI_EVAL),
            evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
        )
    });
    assert!(
        kill < vanilla_three_three(),
        "the 6/6 it leaves its owner comes off the kill: {kill}"
    );
    assert!(
        eval < unpriced_eval,
        "an enemy Trigor counts for more against the AI: {eval} against {unpriced_eval}"
    );
}

#[test]
fn r2424000_a_death_that_harms_its_owner_prices_below_a_vanilla_body() {
    let state = board(
        "ai-death-floodgates",
        json!([]),
        json!([FLOODGATES, VANILLA, VANILLA]),
        json!([]),
    );
    let floodgates = unit(&state, AI, FLOODGATES);
    let body = unit_worth(&state, floodgates, &AI_EVAL);
    let (prices, _) = priced(&state);
    let price = price_of(&prices, FLOODGATES);
    assert!(price < 0.0, "its Death destroys its owner's two 4/4s: {price}");
    let worth = with_death_prices(prices, || unit_worth(&state, floodgates, &AI_EVAL));
    assert!(worth < body, "{worth} against its body's {body}");
}

#[test]
fn r2424000_a_training_dummy_is_priced_by_the_card_its_death_adds() {
    let state = board("ai-death-dummy", json!([]), json!([TRAINING_DUMMY]), json!([]));
    let (prices, _) = priced(&state);
    let price = price_of(&prices, TRAINING_DUMMY);
    assert!(price > 0.0, "a card in hand is worth something: {price}");
}

#[test]
fn r2424000_a_unit_in_the_ai_hand_is_priced_by_placing_it_first() {
    let state = board("ai-death-hand", json!([TRIGOR]), json!([VANILLA]), json!([]));
    let (prices, used) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > vanilla_three_three(), "{prices:?}");
    assert!(
        (3..=AI_DEATH_PROBE_NODES).contains(&used),
        "a spawn, a destroy and a remove at least: {used}"
    );
}

#[test]
fn r2424000_a_full_unit_row_still_prices_a_hand_unit() {
    let state = board(
        "ai-death-full",
        json!([TRIGOR]),
        json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]),
        json!([]),
    );
    assert_eq!(active_units_of(&state, AI).len(), 5);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > vanilla_three_three(), "{prices:?}");
}

#[test]
fn r2424000_a_board_without_a_death_prices_nothing_and_spends_no_node() {
    let state = board(
        "ai-death-none",
        json!([VANILLA]),
        json!([VANILLA]),
        json!([VANILLA]),
    );
    let (prices, used) = priced(&state);
    assert!(prices.is_empty(), "{prices:?}");
    assert_eq!(used, 0);
}

#[test]
fn r2424000_the_frozen_baseline_and_a_finished_decision_read_no_price() {
    assert_eq!(GREEDY_EVAL.death, 0.0);
    let state = board(
        "ai-death-decide",
        json!([VANILLA]),
        json!([TRIGOR]),
        json!([VANILLA]),
    );
    let trigor = unit(&state, AI, TRIGOR);
    let (prices, _) = priced(&state);
    let greedy = unit_worth(&state, trigor, &GREEDY_EVAL);
    let held = with_death_prices(prices.clone(), || unit_worth(&state, trigor, &GREEDY_EVAL));
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = unit_worth(&state, trigor, &AI_EVAL);
    decide(&state, AI, &mut AiOptions::new(create_rng("ai-death-decide", 0))).expect("the AI acts");
    assert_eq!(
        unit_worth(&state, trigor, &AI_EVAL),
        before,
        "the decision put its prices back"
    );
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        with_death_prices(prices, || panic!("a panic inside the decision"))
    }));
    assert!(unwound.is_err());
    assert_eq!(
        unit_worth(&state, trigor, &AI_EVAL),
        before,
        "a panic put the prices back too"
    );
}
