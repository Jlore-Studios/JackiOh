//! A unit's Death in the AI's evaluation (R2424000): at a decision's root the AI prices each Death it
//! can read, on either field and among its own hand's Units (placed first), as its controller's
//! evaluation after the sandbox's destroy less after its remove (R923003); `unit_worth` adds
//! `AI_EVAL.death` of the price and `kill_worth` takes the whole of it off a kill, so a unit that comes
//! back bigger is worth more alive and less to kill, and one whose Death hurts its owner is worth less.
//! The frozen baseline reads no price, and a call not passed a decision's prices reads none.
//!
//! The cards: Trigor, whose Death summons a Trigor with this one's base stats +3/+3; Floodgates, whose
//! Death destroys all your Units; Training Dummy, whose Death adds a random Radiant Training Grounds
//! card to your hand; and Mr. Vanilla, a (1) 4/4 with no text.
//!
//! R2566000: a Starship Piece's death fuses a copy of it into its owner's Dormant Starship, summoning
//! one first (R1852000), which counts as the unit it wakes as at `slumber_waiting_share` (R2380000), so
//! a piece with no Death of its own is priced too, after every Death. The cards: After Burner, a (2) 4/8
//! Starship Piece whose one hook is a "When launched" only a Starship fires; and Hit Job, "Destroy a Unit".

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const TRIGOR: &str = "traininggrounds-025";
const FLOODGATES: &str = "traininggrounds-030";
const TRAINING_DUMMY: &str = "traininggrounds-036";
const VANILLA: &str = "core-008";
const AFTER_BURNER: &str = "stinkyscape-090";
const HIT_JOB: &str = "core-016";

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

fn prices_of(deaths: Vec<DeathPrice>) -> Prices {
    Prices {
        deaths,
        ..Prices::default()
    }
}

fn unit<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .expect("the unit is on the field")
}

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
    let body = unit_worth(&state, trigor, &AI_EVAL, Prices::NONE);
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
    let worth = unit_worth(&state, trigor, &AI_EVAL, &prices_of(prices));
    assert!((worth - (body + AI_EVAL.death * price)).abs() < 1e-9, "{worth}");
    assert!(worth > vanilla_three_three(), "{worth}");
}

#[test]
fn r2424000_killing_an_enemy_trigor_is_worth_less_than_killing_a_vanilla_3_3() {
    let state = board("ai-death-kill", json!([]), json!([VANILLA]), json!([TRIGOR]));
    let trigor = unit(&state, HUMAN, TRIGOR);
    let unpriced_kill = kill_worth(&state, trigor, &AI_EVAL, Prices::NONE);
    assert!(
        (unpriced_kill - vanilla_three_three()).abs() < 1e-9,
        "unpriced, killing a Trigor takes its 3/3 body: {unpriced_kill}"
    );
    let unpriced_eval = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > 0.0);
    let prices = prices_of(prices);
    let kill = kill_worth(&state, trigor, &AI_EVAL, &prices);
    let eval = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &prices);
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
    let body = unit_worth(&state, floodgates, &AI_EVAL, Prices::NONE);
    let (prices, _) = priced(&state);
    let price = price_of(&prices, FLOODGATES);
    assert!(price < 0.0, "its Death destroys its owner's two 4/4s: {price}");
    let worth = unit_worth(&state, floodgates, &AI_EVAL, &prices_of(prices));
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
    let greedy = unit_worth(&state, trigor, &GREEDY_EVAL, Prices::NONE);
    let held = unit_worth(&state, trigor, &GREEDY_EVAL, &prices_of(prices));
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = unit_worth(&state, trigor, &AI_EVAL, Prices::NONE);
    assert!(
        (before - vanilla_three_three()).abs() < 1e-9,
        "unpriced, a Trigor is its 3/3 body: {before}"
    );
    decide(&state, AI, &mut AiOptions::new(create_rng("ai-death-decide", 0))).expect("the AI acts");
    assert_eq!(
        unit_worth(&state, trigor, &AI_EVAL, Prices::NONE),
        before,
        "a decision leaves no price behind"
    );
}

/// A Mr. Vanilla at After Burner's 4/8, by a stat override (§10.4 layer 1).
fn vanilla_four_eight() -> Value {
    json!({ "def": VANILLA, "statsOverride": { "attack": 4, "health": 8 } })
}

#[test]
fn r2566000_a_starship_piece_with_no_death_is_priced_by_the_copy_it_fuses() {
    let state = board("ai-piece-own", json!([]), json!([AFTER_BURNER]), json!([]));
    let piece = unit(&state, AI, AFTER_BURNER);
    assert!(
        death_hook_of(&state, piece).is_none(),
        "After Burner has no Death"
    );
    assert!(is_starship_piece(&state, piece));
    let body = unit_worth(&state, piece, &AI_EVAL, Prices::NONE);
    let (prices, used) = priced(&state);
    let price = price_of(&prices, AFTER_BURNER);
    assert!(
        (price - AI_EVAL.slumber_waiting_share * body).abs() < 1e-9,
        "its death leaves a Dormant Starship holding its copy, at the slumber share: {price} against {body}"
    );
    assert!(price > 0.0, "{price}");
    assert!(
        (2..=AI_DEATH_PROBE_NODES).contains(&used),
        "a destroy and a remove at least: {used}"
    );
    let worth = unit_worth(&state, piece, &AI_EVAL, &prices_of(prices));
    assert!((worth - (body + AI_EVAL.death * price)).abs() < 1e-9, "{worth}");
}

#[test]
fn r2566000_killing_an_enemy_piece_is_worth_less_than_killing_a_vanilla_of_its_stats() {
    let state = board(
        "ai-piece-kill",
        json!([]),
        json!([]),
        json!([AFTER_BURNER, vanilla_four_eight()]),
    );
    let piece = unit(&state, HUMAN, AFTER_BURNER);
    let vanilla = unit(&state, HUMAN, VANILLA);
    assert_eq!(
        unit_worth(&state, piece, &AI_EVAL, Prices::NONE),
        unit_worth(&state, vanilla, &AI_EVAL, Prices::NONE),
        "unpriced, the two 4/8s are worth the same"
    );
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, AFTER_BURNER) > 0.0, "{prices:?}");
    let prices = prices_of(prices);
    let (piece_kill, vanilla_kill) = (
        kill_worth(&state, piece, &AI_EVAL, &prices),
        kill_worth(&state, vanilla, &AI_EVAL, &prices),
    );
    assert!(
        piece_kill < vanilla_kill,
        "the copy it fuses into its owner's Starship comes off the kill: {piece_kill} against {vanilla_kill}"
    );
}

#[test]
fn r2566000_given_one_removal_the_ai_destroys_the_vanilla_and_not_the_piece() {
    register_cards();
    let library = json!([VANILLA, VANILLA, VANILLA, VANILLA]);
    for (at, enemy) in [
        ("piece-first", json!([AFTER_BURNER, vanilla_four_eight()])),
        ("vanilla-first", json!([vanilla_four_eight(), AFTER_BURNER])),
    ] {
        let seed = format!("ai-piece-removal-{at}");
        let state = scenario(json!({
            "seed": seed,
            "p1": { "hand": [HIT_JOB], "field": [], "library": library, "mana": 3 },
            "p2": { "hand": [VANILLA], "field": enemy, "library": library },
        }))
        .state()
        .clone();
        let vanilla = unit(&state, HUMAN, VANILLA).id.clone();
        let decision = decide(&state, AI, &mut AiOptions::new(create_rng(&seed, 0))).expect("the AI acts");
        assert!(
            matches!(
                &decision.action,
                ActionBody::Play { targets: Some(targets), .. }
                    if targets.first().cloned() == Some(Selection::Instance { instance_id: vanilla.clone() })
            ),
            "{at}: Hit Job takes the Vanilla, not the piece: {:?}",
            decision.action
        );
    }
}

#[test]
fn r2566000_every_death_is_probed_before_a_starship_piece() {
    let state = board(
        "ai-piece-order",
        json!([]),
        json!([AFTER_BURNER]),
        json!([TRIGOR]),
    );
    let (prices, _) = priced(&state);
    let order: Vec<&str> = prices.iter().map(|entry| entry.def_id.as_str()).collect();
    assert_eq!(
        order,
        [TRIGOR, AFTER_BURNER],
        "the enemy's Trigor first, though the AI's piece is listed ahead of it"
    );

    let (alone, nodes) = priced(&board("ai-piece-order", json!([]), json!([]), json!([TRIGOR])));
    assert_eq!(alone.len(), 1, "{alone:?}");
    let short = death_prices(&state, AI, &create_node_counter(nodes, None));
    let order: Vec<&str> = short.iter().map(|entry| entry.def_id.as_str()).collect();
    assert_eq!(
        order,
        [TRIGOR],
        "with only the Trigor's nodes, the Trigor is priced and the piece is not"
    );
    assert!(
        (price_of(&short, TRIGOR) - price_of(&prices, TRIGOR)).abs() < 1e-9,
        "the Trigor's price is the one it reads beside the piece"
    );
}
