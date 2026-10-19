//! #1255: the AI prices the library cards it can name (SPEC §9.9 "Evaluation"; R185, R311). Its own
//! deck's cards it was shown going in count at `library_known_share` of their hand worth, a dealt one
//! only by what it gained since the deal (a Radiant flag and its face), so Core #42 Eugenics' flags and
//! a shuffled-in Journey Complete (with the cards its journey returns, R1246) are cards to come. A curse counts against the side whose deck
//! holds it, so one the seat shuffled into the enemy's deck counts for it. A card the seat cannot name
//! (an enemy's dealt card, an own card it was never shown, R312) counts only by number, as before.
//!
//! The weights are tuning numbers, so the sums below are written in them, never as literals.

use jackioh_ai::{AI_EVAL, EvalWeights, GREEDY_EVAL, NextSwing, evaluate};
use jackioh_engine::testkit::{GameState, Value, json};

use super::support::{AI, in_graveyard, register_cards, run_puzzle, scenario, trace};

const EUGENICS: &str = "core-042";
const JOURNEY: &str = "meditative-096";
const JOURNEY_COMPLETE: &str = "meditative-096-1";
/// Meditative #28.1: "Cast on draw: Take 7 damage."
const CURSE: &str = "meditative-028-1";
/// Meditative #30.1: "Cast on draw: Heal your hero 7."
const LOVE_BOMB: &str = "meditative-030-1";
/// (4) 7/7 Armor 7, Radiant 14/14 Armor 7, Reborn.
const BIG: &str = "core-025";
/// (1) 4/4, Radiant 12/12.
const VANILLA: &str = "core-008";
/// (1) Spell.
const STOCKPILE: &str = "core-005";
/// (2) 7/1 First Strike.
const POINTMASTER: &str = "core-020";

/// The scenario's state, the AI as p1 (support.rs's header).
fn board(setup: Value) -> GameState {
    register_cards();
    let mut options = json!({ "seed": "library-cards" });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), setup) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    scenario(options).state().clone()
}

fn eval(state: &GameState, w: &EvalWeights) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, w)
}

/// AI_EVAL with the library cards priced by number only.
fn zero() -> EvalWeights {
    EvalWeights {
        library_known_share: 0.0,
        ..AI_EVAL
    }
}

fn copies(n: usize, def: &str) -> Vec<Value> {
    vec![json!(def); n]
}

/// Jest's `toBeCloseTo(expected, digits)`: within half a unit of the last digit.
fn assert_close(received: f64, expected: f64, digits: i32) {
    assert!(
        (expected - received).abs() < 10f64.powi(-digits) / 2.0,
        "{received} is not close to {expected}"
    );
}

/// p1's library `library`, nothing else on the board but p2's three Stockpiles.
fn own_library(library: Vec<Value>) -> GameState {
    board(json!({
        "p1": { "library": library },
        "p2": { "library": copies(3, STOCKPILE) },
    }))
}

/// p2's library `library`, p1's three Stockpiles.
fn enemy_library(library: Vec<Value>) -> GameState {
    board(json!({
        "p1": { "library": copies(3, STOCKPILE) },
        "p2": { "library": library },
    }))
}

/// What a 7/7 is worth in hand: it costs 4, below the cap.
fn big_in_hand() -> f64 {
    AI_EVAL.hand_card + 4.0 * AI_EVAL.hand_per_cost
}

/// `n − 1` Vanillas and a 7/7 shuffled in since the deal.
fn vanillas_and_a_shuffled_in_big(n: usize) -> Vec<Value> {
    let mut library = copies(n - 1, VANILLA);
    library.push(json!({ "def": BIG, "created": true }));
    library
}

#[test]
fn a_dealt_own_library_card_counts_by_number_whatever_it_is() {
    let bigs = own_library(copies(4, BIG));
    let vanillas = own_library(copies(4, VANILLA));
    assert_close(eval(&bigs, &AI_EVAL), eval(&vanillas, &AI_EVAL), 9);
}

#[test]
fn a_card_shuffled_into_the_own_library_counts_at_the_share_of_its_hand_worth() {
    let shuffled = own_library(vanillas_and_a_shuffled_in_big(4));
    let vanillas = own_library(copies(4, VANILLA));
    assert_close(
        eval(&shuffled, &AI_EVAL) - eval(&vanillas, &AI_EVAL),
        AI_EVAL.library_known_share * big_in_hand(),
        9,
    );
    assert_close(eval(&shuffled, &zero()), eval(&vanillas, &zero()), 9);
}

#[test]
fn past_library_comfort_a_shuffled_in_card_counts_at_the_share_its_draws_reach() {
    for size in [12, 16] {
        let shuffled = own_library(vanillas_and_a_shuffled_in_big(size));
        let vanillas = own_library(copies(size, VANILLA));
        assert_close(
            eval(&shuffled, &AI_EVAL) - eval(&vanillas, &AI_EVAL),
            AI_EVAL.library_known_share * AI_EVAL.library_comfort / size as f64 * big_in_hand(),
            9,
        );
    }
}

#[test]
fn a_radiant_flag_set_inside_the_own_library_raises_the_score() {
    let plain = own_library(copies(4, BIG));
    let mut flagged = plain.clone();
    // As Eugenics does: inside the library, where nobody sees it, so its record stays the base face.
    flagged.players[AI].library[0].radiant = true;
    assert_eq!(
        flagged.players[AI].library[0]
            .known_as
            .as_ref()
            .map(|known| known.radiant),
        Some(false)
    );
    let w = &AI_EVAL;
    // The Radiant face adds 7 attack, 7 health and Reborn; Armor 7 is on both faces.
    let uplift = 7.0 * w.attack + 7.0 * w.health + w.keyword.reborn;
    assert_close(
        eval(&flagged, w) - eval(&plain, w),
        w.library_known_share * (w.radiant_in_hand + w.radiant_face_share * uplift),
        9,
    );
    assert_close(eval(&flagged, &zero()), eval(&plain, &zero()), 9);
}

#[test]
fn an_own_library_card_the_seat_was_never_shown_counts_only_by_number() {
    let unknown = |def: &str| {
        let mut state = own_library(copies(4, def));
        for card in &mut state.players[AI].library {
            card.known_as = None;
        }
        state
    };
    assert_close(
        eval(&unknown(BIG), &AI_EVAL),
        eval(&unknown(VANILLA), &AI_EVAL),
        9,
    );
}

#[test]
fn the_enemys_dealt_library_changes_nothing_whatever_it_holds() {
    let vanillas = enemy_library(copies(3, VANILLA));
    let mixed = enemy_library(vec![json!(BIG), json!(CURSE), json!(LOVE_BOMB)]);
    let radiant = enemy_library(vec![
        json!({ "def": BIG, "radiant": true }),
        json!({ "def": CURSE, "radiant": true }),
        json!({ "def": LOVE_BOMB, "radiant": true }),
    ]);
    let score = eval(&vanillas, &AI_EVAL);
    assert_close(eval(&mixed, &AI_EVAL), score, 9);
    assert_close(eval(&radiant, &AI_EVAL), score, 9);
}

#[test]
fn a_curse_shuffled_into_the_enemys_deck_raises_the_seats_score() {
    let before = enemy_library(copies(3, VANILLA));
    let mut library = copies(3, VANILLA);
    library.push(json!({ "def": CURSE, "created": true }));
    let after = enemy_library(library);
    assert!(eval(&after, &AI_EVAL) > eval(&before, &AI_EVAL));
    // By number alone it is one more card for the enemy.
    assert_close(
        eval(&after, &zero()),
        eval(&before, &zero()) - AI_EVAL.library_card,
        9,
    );
}

#[test]
fn a_curse_in_the_seats_own_deck_lowers_its_score() {
    let before = own_library(copies(3, VANILLA));
    let mut library = copies(3, VANILLA);
    library.push(json!({ "def": CURSE, "created": true }));
    let after = own_library(library);
    assert!(eval(&after, &AI_EVAL) < eval(&before, &AI_EVAL));
}

#[test]
fn a_journey_complete_counts_the_cards_its_journey_returns() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "library-cards-journey",
        "p1": { "hand": [JOURNEY, BIG, BIG], "mana": 2, "library": copies(3, STOCKPILE) },
        "p2": { "library": copies(2, STOCKPILE) },
    }));
    let bigs: Vec<String> = s
        .hand(AI)
        .iter()
        .filter(|card| card.def_id == BIG)
        .map(|card| card.id.clone())
        .collect();
    s.play(JOURNEY, json!({}));
    s.answer(json!(bigs));
    let after = s.state().clone();
    assert!(
        after.players[AI]
            .library
            .iter()
            .any(|card| card.def_id == JOURNEY_COMPLETE)
    );
    assert_eq!(after.players[AI].exile.len(), 2);

    let mut forgotten = after.clone();
    for card in &mut forgotten.players[AI].library {
        card.memory.shift_remove("journey");
    }
    assert!(eval(&after, &AI_EVAL) > eval(&forgotten, &AI_EVAL));
    assert_close(eval(&after, &zero()), eval(&forgotten, &zero()), 9);
}

/// A Radiant Eugenics over a deck of four Vanillas, two Stockpiles and eight 7/7s: it exiles the six
/// cheap cards and makes the 7/7s Radiant.
fn eugenics_deck() -> Vec<Value> {
    let mut library = copies(4, VANILLA);
    library.extend(copies(2, STOCKPILE));
    library.extend(copies(8, BIG));
    library
}

#[test]
fn eugenics_pays_off_through_the_share_and_never_at_a_zero_share() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "library-cards-eugenics",
        "p1": { "hand": [{ "def": EUGENICS, "radiant": true }], "mana": 2, "library": eugenics_deck() },
        "p2": { "library": copies(3, STOCKPILE) },
    }));
    let before = s.state().clone();
    s.play(EUGENICS, json!({}));
    let after = s.state().clone();
    assert_eq!(after.players[AI].library.len(), 8);
    assert!(after.players[AI].library.iter().all(|card| card.radiant));

    assert!(eval(&after, &AI_EVAL) > eval(&before, &AI_EVAL));
    assert!(eval(&after, &zero()) < eval(&before, &zero()));
}

#[test]
fn greedy_eval_prices_no_library_card_by_what_it_is() {
    let shuffled = own_library(vanillas_and_a_shuffled_in_big(4));
    let vanillas = own_library(copies(4, VANILLA));
    assert_close(eval(&shuffled, &GREEDY_EVAL), eval(&vanillas, &GREEDY_EVAL), 9);

    let before = enemy_library(copies(3, VANILLA));
    let mut library = copies(3, VANILLA);
    library.push(json!({ "def": CURSE, "created": true }));
    let after = enemy_library(library);
    assert_close(
        eval(&after, &GREEDY_EVAL),
        eval(&before, &GREEDY_EVAL) - GREEDY_EVAL.library_card,
        9,
    );
}

/// Hard is an engine handicap the search never reads (R180), so the scenario gives the mana directly.
/// Pointmaster is the other 2-mana play: a 7/1 on the board beats every Eugenics line the deck's count
/// alone prices (a zero share plays Pointmaster here), and the eight Radiant 7/7s to come beat it.
#[test]
fn the_ai_casts_eugenics_over_a_deck_of_big_radiant_faces() {
    let run = run_puzzle(
        "eugenics-1255",
        json!({
            "p1": {
                "hand": [{ "def": EUGENICS, "radiant": true }, POINTMASTER],
                "mana": 2,
                "library": eugenics_deck(),
            },
            "p2": { "library": copies(3, STOCKPILE) },
        }),
    );
    assert!(in_graveyard(&run.end, AI, EUGENICS), "{}", trace(&run.turn));
}

/// Two 7/7s the AI cannot pay for this turn: the journey sends them away and brings them back Radiant
/// (14/14 Reborn) as the Journey Complete is drawn from a deck of five. A zero share keeps them.
#[test]
fn the_ai_casts_meditative_journey_on_units_it_cannot_pay_for() {
    let run = run_puzzle(
        "journey-1255",
        json!({
            "p1": { "hand": [JOURNEY, BIG, BIG], "mana": 2, "library": copies(4, STOCKPILE) },
            "p2": { "library": copies(3, STOCKPILE) },
        }),
    );
    let me = &run.end.players[AI];
    assert!(
        me.library.iter().any(|card| card.def_id == JOURNEY_COMPLETE),
        "{}",
        trace(&run.turn)
    );
    assert!(
        me.exile.iter().any(|card| card.def_id == BIG),
        "{}",
        trace(&run.turn)
    );
}
