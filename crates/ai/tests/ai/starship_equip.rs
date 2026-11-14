//! Issue #3575, fullsend 3's Starship Pieces, Equip and launches in the AI:
//!
//! - R3575400: the seat's own hand cards' Activates are priced at its root, as a card on the field is
//!   priced a turn later (`activation_prices`), so a play whose worth is the Activate it leaves reads it.
//! - R3575402: a shipyard's turn-start fuse (R3539004) is priced as its turn text (`turn_hook_prices`),
//!   outside the slumber share, since it fuses while the card sleeps.
//! - R3575404: a Dormant card's awakening Activate is priced, the launch among them (R1839002), and for
//!   that use alone the card need not be the instance left standing (R3539004).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards, unit};

/// Stinkyscape #52 Cannon v3: (2) 0/2, Starship Piece, Equip, "Activate: Deal 5 damage to an enemy Unit".
const CANNON: &str = "stinkyscape-052";
/// Stinkyscape+ #157 Phantom Shipyard: (4) 0/0, a shipyard Dormant until launched (R3539004).
const SHIPYARD: &str = "stinkyscapeplus-157";
const SORCERER: &str = "core-068";
const VANILLA: &str = "core-008";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with the AI's side `p1` and the human's `p2`, each with `LIBRARY` added.
fn board(seed: &str, mut p1: Value, mut p2: Value) -> Scenario {
    register_cards();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

fn activation_price_of(prices: &[ActivationPrice], def_id: &str, controller: PlayerId) -> Option<f64> {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .map(|entry| entry.price)
}

#[test]
fn r3575400_an_own_hand_cards_activate_is_priced_and_the_enemys_hand_never() {
    let state = board(
        "ai-r3575400-hand",
        json!({ "hand": [CANNON, VANILLA], "mana": 2 }),
        json!({ "hand": [CANNON], "field": [SORCERER] }),
    )
    .state()
    .clone();
    let sorcerer = unit_worth(&state, unit(&state, HUMAN, SORCERER), &AI_EVAL, Prices::NONE);
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(&state, AI, &counter);
    let price = activation_price_of(&prices, CANNON, AI)
        .unwrap_or_else(|| panic!("the AI's hand Cannon is priced: {prices:?}"));
    assert!(
        price >= sorcerer,
        "its shot destroys the enemy's 5/5 a turn after it is played: {price} against {sorcerer}"
    );
    assert_eq!(
        activation_price_of(&prices, CANNON, HUMAN),
        None,
        "the enemy's hand is hidden and never probed"
    );
    assert!(
        counter.used() <= AI_ACTIVATION_PROBE_NODES + AI_HAND_ACTIVATION_PROBE_NODES,
        "within the two caps: {}",
        counter.used()
    );

    // The Cannon the AI plays reads the price the root gave it.
    let mut played = board(
        "ai-r3575400-hand",
        json!({ "hand": [CANNON, VANILLA], "mana": 2 }),
        json!({ "hand": [CANNON], "field": [SORCERER] }),
    );
    played.play(CANNON, json!({ "zone": 1 }));
    let after = played.state().clone();
    let cannon = unit(&after, AI, CANNON);
    let priced = Prices {
        activations: prices,
        ..Prices::default()
    };
    let gained =
        unit_worth(&after, cannon, &AI_EVAL, &priced) - unit_worth(&after, cannon, &AI_EVAL, Prices::NONE);
    assert!(
        (gained - AI_EVAL.activation * price).abs() < 1e-9,
        "{gained} against {price}"
    );
}

#[test]
fn r3575400_a_hand_without_an_activate_spends_no_node() {
    let state = board(
        "ai-r3575400-none",
        json!({ "hand": [VANILLA], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    )
    .state()
    .clone();
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    assert_eq!(activation_prices(&state, AI, &counter), vec![]);
    assert_eq!(counter.used(), 0);
}

#[test]
fn r3575402_a_shipyards_turn_start_fuse_is_priced_in_hand_and_on_the_field() {
    let mut s = board(
        "ai-r3575402-yard",
        json!({ "hand": [SHIPYARD, VANILLA], "mana": 5 }),
        json!({ "hand": [VANILLA], "field": [VANILLA] }),
    );
    let in_hand = s.state().clone();
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let hand_prices = turn_hook_prices(&in_hand, AI, &counter);
    let hand_price = hand_prices
        .iter()
        .find(|entry| entry.def_id == SHIPYARD)
        .unwrap_or_else(|| panic!("the hand shipyard is priced: {hand_prices:?}"))
        .price;
    assert!(
        hand_price > 0.0,
        "a piece fused in is worth something: {hand_price}"
    );

    s.play(SHIPYARD, json!({ "zone": 1 }));
    let placed = s.state().clone();
    let yard = units_on_field_of(&placed, AI)
        .into_iter()
        .find(|card| card.def_id == SHIPYARD)
        .expect("the shipyard on the field")
        .clone();
    assert!(is_slumbering(&yard), "it lies Dormant until launched");
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let field_prices = turn_hook_prices(&placed, AI, &counter);
    assert!(
        field_prices
            .iter()
            .any(|entry| entry.def_id == SHIPYARD && entry.price > 0.0),
        "and on the field: {field_prices:?}"
    );
    let priced = Prices {
        turn_hooks: field_prices.clone(),
        ..Prices::default()
    };
    let price = field_prices
        .iter()
        .find(|entry| entry.def_id == SHIPYARD)
        .map(|entry| entry.price)
        .unwrap_or_default();
    let gained = evaluate_priced(&placed, AI, NextSwing::Enemy, &AI_EVAL, &priced)
        - evaluate_priced(&placed, AI, NextSwing::Enemy, &AI_EVAL, Prices::NONE);
    assert!(
        (gained - AI_EVAL.turn_hook * price).abs() < 1e-9,
        "outside the slumber share, the probe already read the hull at it: {gained} against {price}"
    );
    assert_eq!(
        evaluate_priced(&placed, AI, NextSwing::Enemy, &GREEDY_EVAL, &priced),
        evaluate(&placed, AI, NextSwing::Enemy, &GREEDY_EVAL),
        "the frozen baseline reads no price"
    );
}

#[test]
fn r3575404_a_dormant_shipyards_launch_is_priced_though_it_exiles_the_hull() {
    // Played, the shipyard lies Dormant with no piece, so its launch is refused; at the seat's next
    // turn start its yard has fused one in (R3539004) and the probe prices that launch.
    let mut s = board(
        "ai-r3575404-launch",
        json!({ "hand": [SHIPYARD, VANILLA], "mana": 5 }),
        json!({ "hand": [VANILLA], "field": [VANILLA] }),
    );
    s.play(SHIPYARD, json!({ "zone": 1 }));
    let placed = s.state().clone();
    let yard = units_on_field_of(&placed, AI)
        .into_iter()
        .find(|card| card.def_id == SHIPYARD)
        .expect("the shipyard on the field")
        .clone();
    assert!(
        acts_to_awaken(&placed, &yard),
        "it acts for its awakening alone (R1839002)"
    );
    assert!(
        !active_units_of(&placed, AI)
            .into_iter()
            .any(|unit| unit.id == yard.id),
        "and is no active unit, so R2564000's walk left it out (R1839000)"
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(&placed, AI, &counter);
    let price = activation_price_of(&prices, SHIPYARD, AI)
        .unwrap_or_else(|| panic!("the Dormant shipyard's launch is priced: {prices:?}"));
    assert!(
        price > 0.0,
        "the Starship it stands up is worth something: {price}"
    );
    let priced = Prices {
        activations: prices,
        ..Prices::default()
    };
    let gained = evaluate_priced(&placed, AI, NextSwing::Enemy, &AI_EVAL, &priced)
        - evaluate_priced(&placed, AI, NextSwing::Enemy, &AI_EVAL, Prices::NONE);
    assert!(
        (gained - AI_EVAL.activation * price).abs() < 1e-9,
        "the sleeper is worth `activation` of its launch, not the slumber share of that: the price was \
         read from the sleeping hull, so it already holds the wait: {gained} against {price}"
    );
}
