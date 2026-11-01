//! A unit's Activate in the AI's evaluation (R2564000): at a decision's root the AI ends its own turn
//! and then the enemy's on a copy, tries each visible unit's Activate at the start of its controller's
//! next turn as the opponent's reply tries one, and prices the best use as its controller's evaluation
//! after it less before it; `unit_worth` adds `AI_EVAL.activation` of the price for the seat's units
//! and the enemy's alike. A face-down or hand card is never probed, so hidden cards change no decision
//! (R185), the frozen baseline reads no price, and a call not passed a decision's prices reads none.
//! R2603000 prices a face-up backrow card's Activate, a Field Spell's, the same way.
//!
//! Cannon v3 is a Starship Piece: when it dies, a copy of it fuses into its owner's Dormant Starship
//! (R1852000), which can fire it again. So the removal below is a transform, which is no death, and the
//! tribute is made beside a Voidwalker, whose aura exiles the tributed card (R461).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, on_field, register_cards};

const CANNON: &str = "stinkyscape-052";
const POLYMORPH: &str = "meditative-024";
const THE_ROCK: &str = "core-066";
const POSTDOC: &str = "core-061";
const SORCERER: &str = "core-068";
const VOIDWALKER: &str = "classic-050";
const MENACE: &str = "core-019";
const VANILLA: &str = "core-008";
const SHEEPISH: &str = "core-041";
const MY_PAWN: &str = "core-096";
const PUNISH: &str = "classic-020";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with the AI's side `p1` and the human's `p2`, each with `LIBRARY` added.
fn board(seed: &str, mut p1: Value, mut p2: Value) -> GameState {
    register_cards();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// The AI's activation prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<ActivationPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `activations` as a decision passes them, with no Death or turn-hook price.
fn prices_of(activations: Vec<ActivationPrice>) -> Prices {
    Prices {
        activations,
        ..Prices::default()
    }
}

fn unit<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .expect("the unit is on the field")
}

/// The price of `controller`'s `def_id`, which must be priced.
fn price_of(prices: &[ActivationPrice], def_id: &str, controller: PlayerId) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{controller}'s {def_id}'s Activate is priced: {prices:?}"))
}

/// The AI's Sorcerer and `hand` against the human's Cannon v3, Mr. Vanilla and `backrow`.
fn cannon_against_vanilla(seed: &str, hand: Value, backrow: Value) -> GameState {
    board(
        seed,
        json!({ "hand": hand, "field": [SORCERER] }),
        json!({ "hand": [VANILLA], "field": [CANNON, VANILLA], "backrow": backrow }),
    )
}

/// `state` with the AI's Sorcerer just summoned, so it cannot attack this turn.
fn sorcerer_resting(mut state: GameState) -> GameState {
    let sorcerer = unit(&state, AI, SORCERER).id.clone();
    let turn = state.turn;
    find_instance_mut(&mut state, &sorcerer)
        .expect("the Sorcerer is on the field")
        .summoned_turn = Some(turn);
    state
}

#[test]
fn r2564000_an_own_cannon_is_priced_by_the_unit_its_shot_destroys() {
    let state = board(
        "ai-activation-own",
        json!({ "field": [CANNON] }),
        json!({ "field": [SORCERER] }),
    );
    let cannon = unit(&state, AI, CANNON);
    let sorcerer = unit_worth(&state, unit(&state, HUMAN, SORCERER), &AI_EVAL, Prices::NONE);
    let body = unit_worth(&state, cannon, &AI_EVAL, Prices::NONE);
    let (prices, used) = priced(&state);
    let price = price_of(&prices, CANNON, AI);
    assert!(
        price >= sorcerer,
        "its shot destroys the enemy's 5/5: {price} against {sorcerer}"
    );
    assert!(
        (3..=AI_ACTIVATION_PROBE_NODES).contains(&used),
        "two turn ends and a use at least: {used}"
    );
    let worth = unit_worth(&state, cannon, &AI_EVAL, &prices_of(prices));
    assert!(
        (worth - (body + AI_EVAL.activation * price)).abs() < 1e-9,
        "{worth} against {body} and {price}"
    );
}

#[test]
fn r2564000_an_enemy_cannon_is_worth_more_to_destroy_than_a_bigger_vanilla() {
    let state = cannon_against_vanilla("ai-activation-enemy", json!([]), json!([]));
    let (cannon, vanilla) = (unit(&state, HUMAN, CANNON), unit(&state, HUMAN, VANILLA));
    let (unpriced_cannon, unpriced_vanilla) = (
        kill_worth(&state, cannon, &AI_EVAL, Prices::NONE),
        kill_worth(&state, vanilla, &AI_EVAL, Prices::NONE),
    );
    assert!(
        unpriced_cannon < unpriced_vanilla,
        "unpriced, the 0/4 is the smaller kill: {unpriced_cannon} against {unpriced_vanilla}"
    );
    let unpriced_eval = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, CANNON, HUMAN) > 0.0);
    let prices = prices_of(prices);
    let cannon_kill = kill_worth(&state, cannon, &AI_EVAL, &prices);
    let vanilla_kill = kill_worth(&state, vanilla, &AI_EVAL, &prices);
    let eval = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &prices);
    assert!(
        cannon_kill > vanilla_kill,
        "the shot it fires each turn counts: {cannon_kill} against {vanilla_kill}"
    );
    assert!(
        eval < unpriced_eval,
        "an enemy Cannon counts for more against the AI: {eval} against {unpriced_eval}"
    );
}

#[test]
fn r2564000_a_board_without_an_activate_prices_nothing_and_spends_no_node() {
    let state = board(
        "ai-activation-none",
        json!({ "hand": [CANNON], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2564000_given_one_removal_the_ai_removes_the_enemy_cannon() {
    let state = sorcerer_resting(cannon_against_vanilla(
        "ai-activation-removal",
        json!([POLYMORPH]),
        json!([]),
    ));
    let cannon = unit(&state, HUMAN, CANNON).id.clone();
    let polymorph = state.players[AI].hand[0].id.clone();
    let turn = play_ai_turn(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-removal", 0)),
    );
    let targets = turn.actions.iter().find_map(|action| match &action.body {
        ActionBody::Play {
            instance_id, targets, ..
        } if *instance_id == polymorph => Some(targets.clone()),
        _ => None,
    });
    assert_eq!(
        targets,
        Some(Some(vec![Selection::Instance { instance_id: cannon }])),
        "{:?}",
        turn.actions
    );
}

#[test]
fn r2564000_the_ai_tributes_its_vanilla_before_its_cannon() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-activation-tribute",
        "p1": {
            "hand": [THE_ROCK],
            "field": [CANNON, POSTDOC, { "def": MENACE, "position": "DEF" }],
            "library": LIBRARY,
        },
        "p2": { "hand": [VANILLA], "field": [SORCERER, VOIDWALKER], "library": LIBRARY },
    }));
    let target = s.unit_at(HUMAN, 1).id.clone();
    s.activate(
        CANNON,
        json!({ "ability": "cannon", "targets": pick::id(&target) }),
    );
    let state = s.state().clone();
    assert_eq!(
        active_units_of(&state, HUMAN).len(),
        1,
        "the use is spent on the Sorcerer and the Voidwalker is left"
    );
    let postdoc = unit(&state, AI, POSTDOC).id.clone();
    let rock = state.players[AI].hand[0].id.clone();
    let turn = play_ai_turn(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-tribute", 0)),
    );
    let tributes = turn.actions.iter().find_map(|action| match &action.body {
        ActionBody::Play {
            instance_id,
            tributes,
            ..
        } if *instance_id == rock => Some(tributes.clone()),
        _ => None,
    });
    assert_eq!(tributes, Some(Some(vec![postdoc])), "{:?}", turn.actions);
    assert!(on_field(&turn.state, AI, CANNON), "the Cannon stays");
}

#[test]
fn r2564000_a_face_down_card_is_never_probed_and_hidden_cards_change_no_decision() {
    let hidden = board(
        "ai-activation-hidden",
        json!({ "field": [SORCERER] }),
        json!({ "hand": [CANNON], "backrow": [SHEEPISH], "field": [VANILLA] }),
    );
    assert_eq!(
        priced(&hidden),
        (vec![], 0),
        "a Cannon in the enemy's hand and its face-down Trap are never probed"
    );

    let a = sorcerer_resting(cannon_against_vanilla(
        "ai-activation-r185",
        json!([POLYMORPH]),
        json!([SHEEPISH]),
    ));
    let mut b = a.clone();
    b.players[HUMAN].hand[0].def_id = CANNON.to_string();
    let trap = b.players[HUMAN]
        .backrow
        .iter_mut()
        .flatten()
        .next()
        .expect("the enemy's Trap is set");
    trap.def_id = MY_PAWN.to_string();
    let decide_on = |state: &GameState| {
        decide(
            state,
            AI,
            &mut AiOptions::new(create_rng("ai-activation-r185", 0)),
        )
        .expect("the AI acts")
    };
    let (on_a, on_b) = (decide_on(&a), decide_on(&b));
    assert_eq!(on_a.action, on_b.action);
    assert_eq!(on_a.line, on_b.line);
}

#[test]
fn r2564000_the_probes_stay_within_their_cap_and_the_decision_within_its_budget() {
    let state = board(
        "ai-activation-budget",
        json!({ "field": [CANNON, { "def": CANNON, "radiant": true }] }),
        json!({ "field": [CANNON, { "def": CANNON, "radiant": true }, SORCERER] }),
    );
    let (_, used) = priced(&state);
    assert!((2..=AI_ACTIVATION_PROBE_NODES).contains(&used), "{used}");
    let decision = decide(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-budget", 0)),
    )
    .expect("the AI acts");
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

#[test]
fn r2564000_the_frozen_baseline_and_a_finished_decision_read_no_price() {
    assert_eq!(GREEDY_EVAL.activation, 0.0);
    let state = board(
        "ai-activation-decide",
        json!({ "hand": [VANILLA], "field": [CANNON] }),
        json!({ "field": [SORCERER] }),
    );
    let cannon = unit(&state, AI, CANNON);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, CANNON, AI) > 0.0);
    let prices = prices_of(prices);
    let greedy = unit_worth(&state, cannon, &GREEDY_EVAL, Prices::NONE);
    let held = unit_worth(&state, cannon, &GREEDY_EVAL, &prices);
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = unit_worth(&state, cannon, &AI_EVAL, Prices::NONE);
    assert!(
        unit_worth(&state, cannon, &AI_EVAL, &prices) > before,
        "passed the prices, a Cannon counts its Activate over its unpriced {before}"
    );
    decide(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-decide", 0)),
    )
    .expect("the AI acts");
    assert_eq!(
        unit_worth(&state, cannon, &AI_EVAL, Prices::NONE),
        before,
        "a decision leaves no price behind"
    );
}

#[test]
fn r2603000_an_own_power_to_punish_raises_the_evaluation_by_its_price() {
    let state = board(
        "ai-activation-field-spell-own",
        json!({ "backrow": [PUNISH] }),
        json!({ "hand": [VANILLA], "field": [SORCERER] }),
    );
    let unpriced = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, used) = priced(&state);
    let price = price_of(&prices, PUNISH, AI);
    assert!(price > 0.0, "a use punishes the enemy: {price}");
    assert!(
        (3..=AI_ACTIVATION_PROBE_NODES).contains(&used),
        "two turn ends and a use at least: {used}"
    );
    let prices = prices_of(prices);
    let eval = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &prices);
    assert!(
        (eval - (unpriced + AI_EVAL.activation * price)).abs() < 1e-9,
        "{eval} against {unpriced} and {price}"
    );
    assert_eq!(
        evaluate_priced(&state, AI, NextSwing::Enemy, &GREEDY_EVAL, &prices),
        evaluate(&state, AI, NextSwing::Enemy, &GREEDY_EVAL),
        "the frozen baseline reads no price"
    );
}

#[test]
fn r2603000_an_enemy_power_to_punish_lowers_the_evaluation_by_its_price() {
    let state = board(
        "ai-activation-field-spell-enemy",
        json!({ "hand": [VANILLA], "field": [SORCERER] }),
        json!({ "backrow": [PUNISH] }),
    );
    let unpriced = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, _) = priced(&state);
    let price = price_of(&prices, PUNISH, HUMAN);
    assert!(price > 0.0, "a use punishes the AI: {price}");
    let eval = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &prices_of(prices));
    assert!(
        (eval - (unpriced - AI_EVAL.activation * price)).abs() < 1e-9,
        "{eval} against {unpriced} and {price}"
    );
}

#[test]
fn r2603000_an_enemy_face_down_card_is_never_probed_and_changes_no_price() {
    let a = board(
        "ai-activation-field-spell-hidden",
        json!({ "backrow": [PUNISH] }),
        json!({ "hand": [VANILLA], "field": [SORCERER], "backrow": [SHEEPISH] }),
    );
    let mut b = a.clone();
    b.players[HUMAN]
        .backrow
        .iter_mut()
        .flatten()
        .next()
        .expect("the enemy's Trap is set")
        .def_id = MY_PAWN.to_string();
    let (on_a, on_b) = (priced(&a), priced(&b));
    assert!(price_of(&on_a.0, PUNISH, AI) > 0.0);
    assert!(
        on_a.0.iter().all(|entry| entry.controller == AI),
        "only the AI's Power to Punish is priced: {on_a:?}"
    );
    assert_eq!(
        on_a, on_b,
        "the enemy's face-down Trap changes no price and no node"
    );
}
