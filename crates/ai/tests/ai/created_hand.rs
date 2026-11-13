//! Hand cards a play makes (issue #3575, fullsend 3 of the Stinkyscape+ set). R3575500: a `costOverride`
//! an own hand card carries (S+ #5 Dingleberry Droat's fusion "costs (2)") moves its price as a `costMod`
//! does, `AI_EVAL.hand_cost_delta` a crystal (B3.4). R3575501: an own Created hand Unit (R943) is worth at
//! least its body a turn away, `slumber_turn_share` of its face less its crystals at `unspent_mana` and
//! the worth of the cheapest own units its Tribute would destroy (R3565000's rider read), so a Field
//! Spell whose start-of-turn hook adds a Tribute-free Automa card (S+ #129) and a Spell that adds a token
//! rung (S+ #133) are priced by what they give.
//!
//! Every board is a `scenario()` with the AI as p1 on 7 crystals and libraries of uncastable stand-ins.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, act, close, register_cards};

/// A card nobody can cast: the libraries' draws change nothing.
const INERT: &str = "stand-in (9) 1/1";
/// S+ #112 Fortress, (2), printed Tribute 1.
const FORTRESS: &str = "stinkyscapeplus-112";
/// S+ #129 Multiplicative Reconstructor, (3) Field Spell: "Start of turn: Add a random (2) cost Automa
/// card to your hand, it has no Tribute condition."
const MULTIPLICATIVE: &str = "stinkyscapeplus-129";
/// S+ #133 Great Paperclip Theory III, (4) Spell: adds Daglare (0) and the next rung on each first play.
const PAPERCLIP: &str = "stinkyscapeplus-133";
/// The rungs Paperclip III adds: Daglare (0), Macizon (1), Fortnith (2).
const RUNGS: [&str; 3] = [
    "stinkyscapeplus-133-1",
    "stinkyscapeplus-133-2",
    "stinkyscapeplus-133-3",
];
/// The actions a test lets one AI turn take.
const TURN_ACTIONS: usize = 12;

/// p1 to act on turn 9 with `hand`, `field` and 7 crystals, Hard's cap (R180).
fn board(hand: &[&str], field: &[&str]) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": "ai-created-hand",
        "active": "p1",
        "turn": 9,
        "p1": { "hand": hand, "field": field, "mana": 7, "library": [INERT, INERT, INERT, INERT] },
        "p2": { "hand": ["stand-in (1) 1/1"], "library": [INERT, INERT, INERT, INERT] },
    }))
    .state()
    .clone();
    state.players[AI].mana.max = 7;
    state
}

fn eval(state: &GameState, w: &EvalWeights) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, w)
}

/// `state` with its first hand card's `created` and `cost_override` set.
fn marked(state: &GameState, created: bool, cost_override: Option<i32>) -> GameState {
    let mut out = state.clone();
    out.players[AI].hand[0].created = Some(created);
    out.players[AI].hand[0].cost_override = cost_override;
    out
}

/// The AI's turn from `state`: `decide` and the reducer until the turn is no longer its own.
fn ai_turn(state: &GameState) -> Vec<String> {
    let mut options = AiOptions {
        rng: create_rng("ai-created-hand", 0),
        budget: AI_BUDGET,
        should_stop: None,
        wrap_up: None,
    };
    let mut current = state.clone();
    let mut played = Vec::new();
    while current.active == AI && current.result.is_none() && played.len() < TURN_ACTIONS {
        let decision = decide(&current, AI, &mut options).expect("the AI decides");
        if let ActionBody::Play { instance_id, .. } = &decision.action {
            let def = current.players[AI]
                .hand
                .iter()
                .find(|card| &card.id == instance_id)
                .map(|card| card.def_id.clone());
            played.extend(def);
        }
        current = act(&current, AI, &decision.action);
        if matches!(decision.action, ActionBody::EndTurn) {
            break;
        }
    }
    played
}

#[test]
fn r3575500_a_cost_override_moves_a_hand_card_as_a_cost_mod_does() {
    let printed = board(&["stand-in (4) Spell"], &[]);
    let dealt = marked(&printed, false, None);
    // (4) laid at (2): two crystals cheaper, as a costMod of −2 is.
    close(
        eval(&marked(&printed, false, Some(2)), &AI_EVAL) - eval(&dealt, &AI_EVAL),
        2.0 * AI_EVAL.hand_cost_delta,
    );
    let mut cost_mod = dealt.clone();
    cost_mod.players[AI].hand[0].cost_mod = -2;
    close(
        eval(&marked(&printed, false, Some(2)), &AI_EVAL),
        eval(&cost_mod, &AI_EVAL),
    );
    // A dearer set price costs it, the printed price changes nothing, and the frozen baseline reads none.
    close(
        eval(&marked(&printed, false, Some(6)), &AI_EVAL) - eval(&dealt, &AI_EVAL),
        -2.0 * AI_EVAL.hand_cost_delta,
    );
    close(
        eval(&marked(&printed, false, Some(4)), &AI_EVAL),
        eval(&dealt, &AI_EVAL),
    );
    close(
        eval(&marked(&printed, false, Some(2)), &GREEDY_EVAL),
        eval(&dealt, &GREEDY_EVAL),
    );
}

#[test]
fn r3575501_a_created_unit_counts_its_body_a_turn_away() {
    let w = &AI_EVAL;
    let big = board(&["stand-in (2) 9/9"], &[]);
    let dealt = eval(&marked(&big, false, None), w);
    let created = eval(&marked(&big, true, None), w);
    // The body (9 attack, 9 health) a turn away, less its two crystals; the cost price was 1 + 0.3 × 2.
    let floor = w.slumber_turn_share * (9.0 * w.attack + 9.0 * w.health - 2.0 * w.unspent_mana);
    let priced = w.hand_card + w.hand_per_cost * 2.0;
    assert!(floor > priced);
    close(created - dealt, floor - priced);
    // A Created card whose body is worth less than its cost price keeps the cost price.
    let small = board(&["stand-in (6) 1/1"], &[]);
    close(
        eval(&marked(&small, true, None), w),
        eval(&marked(&small, false, None), w),
    );
    // A Created Spell has no body to count.
    let spell = board(&["stand-in (2) Spell"], &[]);
    close(
        eval(&marked(&spell, true, None), w),
        eval(&marked(&spell, false, None), w),
    );
    // The frozen baseline reads none.
    close(
        eval(&marked(&big, true, None), &GREEDY_EVAL),
        eval(&marked(&big, false, None), &GREEDY_EVAL),
    );
}

#[test]
fn r3575501_a_created_units_tribute_costs_the_cheapest_units_it_would_destroy() {
    let w = &AI_EVAL;
    let gain = |field: &[&str]| {
        let state = board(&[FORTRESS], field);
        eval(&marked(&state, true, None), w) - eval(&marked(&state, false, None), w)
    };
    // Tribute 1 with no unit to pay it: the card is priced by its cost.
    close(gain(&[]), 0.0);
    // With units, the floor holds less the cheapest one's worth: a dearer second unit changes nothing.
    let one = gain(&["stand-in (1) 1/1"]);
    let two = gain(&["stand-in (1) 1/1", "stand-in (5) 6/6"]);
    assert!(
        one > 0.0,
        "a Tribute-1 Fortress with a 1/1 to pay it counts its body: {one}"
    );
    close(one, two);
    // A dearer unit to pay it with leaves it worth less.
    assert!(gain(&["stand-in (3) 3/3"]) < one);
}

#[test]
fn r3575501_a_start_of_turn_hook_that_adds_a_tribute_free_automa_card_is_priced_by_its_body() {
    let state = board(&[MULTIPLICATIVE], &[]);
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = turn_hook_prices(&state, AI, &counter);
    let price = prices
        .iter()
        .find(|entry| entry.def_id == MULTIPLICATIVE)
        .map(|entry| entry.price)
        .expect("the hook is priced");
    // Fortress or Zenith, (2), Tribute-free: more than a (2) card's cost price.
    assert!(
        price > AI_EVAL.hand_card + AI_EVAL.hand_per_cost * 2.0,
        "one firing adds a body worth more than a (2) card's price: {price}"
    );
    // On 7 crystals with three vanilla bodies beside it, the AI sets it.
    let hand = [
        MULTIPLICATIVE,
        "stand-in (2) 2/3",
        "stand-in (3) 3/4",
        "stand-in (4) 4/5",
    ];
    let played = ai_turn(&board(&hand, &[]));
    assert!(
        played.iter().any(|def| def == MULTIPLICATIVE),
        "played {played:?}"
    );
}

#[test]
fn r3575501_paperclip_three_is_cast_and_its_rungs_played_on_seven_crystals() {
    let hand = [PAPERCLIP, "stand-in (3) 3/4", "stand-in (4) 4/5"];
    let played = ai_turn(&board(&hand, &[]));
    assert!(played.iter().any(|def| def == PAPERCLIP), "played {played:?}");
    assert!(played.iter().any(|def| def == RUNGS[0]), "played {played:?}");
}
