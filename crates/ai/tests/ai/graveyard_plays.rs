//! #2928: the evaluation counts a card in a graveyard its owner may still play from there
//! (`playable_from_graveyard`, R454, R65) at `graveyard_playable` of its hand price, either side's,
//! since a graveyard is public (R185). The permissions: a card's own Flashback (Stinkyscape #21
//! Moonberry, R783012), gone once spent (R783013); the one S #152 Sanctum lends every card in its
//! controller's graveyard and no other (R1869003); Classic #28 Second Wind's for every type, and #74
//! Corpse Plantation's for Units while its Activate has armed it. A graveyard card with none counts
//! nothing, and the frozen baseline (`GREEDY_EVAL`) keeps 0.
//!
//! Every board puts the AI in p1's seat (support's header). The cards: Mr. Vanilla, a (1) Unit;
//! Stockpile, a (1) Spell; Sheepish, a (1) Trap.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards};

const MOONBERRY: &str = "stinkyscape-021";
const SANCTUM: &str = "stinkyscape-152";
const SECOND_WIND: &str = "classic-028";
const PLANTATION: &str = "classic-074";
const VANILLA: &str = "core-008";
const STOCKPILE: &str = "core-005";
const SHEEPISH: &str = "core-041";

const LIBRARY: [&str; 4] = [VANILLA, VANILLA, VANILLA, VANILLA];

const COUNTED: EvalWeights = EvalWeights {
    graveyard_playable: 0.5,
    ..AI_EVAL
};
const IGNORED: EvalWeights = EvalWeights {
    graveyard_playable: 0.0,
    ..AI_EVAL
};

fn board(seed: &str, mut p1: Value, mut p2: Value) -> GameState {
    register_cards();
    for side in [&mut p1, &mut p2] {
        side["library"] = json!(LIBRARY);
    }
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

fn score(state: &GameState, w: &EvalWeights) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, w)
}

/// What the term reads, alone: the board counted less the board ignored.
fn term(state: &GameState) -> f64 {
    score(state, &COUNTED) - score(state, &IGNORED)
}

/// A plain card's hand price at this cost (`hand_card_value`).
fn price(cost: f64) -> f64 {
    AI_EVAL.hand_card + AI_EVAL.hand_per_cost * cost
}

fn backrow_id(state: &GameState, player: PlayerId, def_id: &str) -> String {
    state.players[player]
        .backrow
        .iter()
        .flatten()
        .find(|card| card.def_id == def_id)
        .expect("the backrow card")
        .id
        .clone()
}

#[test]
fn a_playable_moonberry_in_the_own_graveyard_beats_the_same_one_exiled() {
    let buried = board(
        "ai-graveyard-moonberry",
        json!({ "graveyard": [MOONBERRY] }),
        json!({}),
    );
    let exiled = board(
        "ai-graveyard-moonberry",
        json!({ "exile": [MOONBERRY] }),
        json!({}),
    );
    assert!(playable_from_graveyard(&buried, &buried.players[AI].graveyard[0]));
    close(
        score(&buried, &COUNTED) - score(&exiled, &COUNTED),
        0.5 * price(1.0),
    );
    close(score(&buried, &IGNORED), score(&exiled, &IGNORED));
}

#[test]
fn a_spent_flashback_counts_nothing() {
    let mut spent = board(
        "ai-graveyard-spent",
        json!({ "graveyard": [MOONBERRY] }),
        json!({}),
    );
    let exiled = board("ai-graveyard-spent", json!({ "exile": [MOONBERRY] }), json!({}));
    let id = spent.players[AI].graveyard[0].id.clone();
    find_instance_mut(&mut spent, &id)
        .expect("Moonberry")
        .flashback_spent = Some(true);
    assert!(!playable_from_graveyard(&spent, &spent.players[AI].graveyard[0]));
    close(score(&spent, &COUNTED), score(&exiled, &COUNTED));
}

#[test]
fn a_graveyard_card_with_no_permission_counts_nothing() {
    let buried = board(
        "ai-graveyard-vanilla",
        json!({ "graveyard": [VANILLA] }),
        json!({}),
    );
    let exiled = board("ai-graveyard-vanilla", json!({ "exile": [VANILLA] }), json!({}));
    assert!(!playable_from_graveyard(
        &buried,
        &buried.players[AI].graveyard[0]
    ));
    close(score(&buried, &COUNTED), score(&exiled, &COUNTED));
}

#[test]
fn sanctum_raises_the_worth_of_its_controllers_graveyard() {
    let lent = board(
        "ai-graveyard-sanctum",
        json!({ "backrow": [SANCTUM], "graveyard": [VANILLA, STOCKPILE] }),
        json!({}),
    );
    assert!(
        lent.players[AI]
            .graveyard
            .iter()
            .all(|card| playable_from_graveyard(&lent, card))
    );
    close(term(&lent), 0.5 * 2.0 * price(1.0));

    let without = board(
        "ai-graveyard-sanctum",
        json!({ "graveyard": [VANILLA, STOCKPILE] }),
        json!({}),
    );
    close(term(&without), 0.0);
}

#[test]
fn sanctum_does_not_reach_the_enemy_graveyard() {
    let state = board(
        "ai-graveyard-sanctum-enemy",
        json!({ "backrow": [SANCTUM] }),
        json!({ "graveyard": [VANILLA] }),
    );
    assert!(!playable_from_graveyard(
        &state,
        &state.players[HUMAN].graveyard[0]
    ));
    close(term(&state), 0.0);
}

#[test]
fn a_playable_card_in_the_opponents_graveyard_counts_for_the_opponent() {
    let buried = board(
        "ai-graveyard-enemy",
        json!({}),
        json!({ "graveyard": [MOONBERRY] }),
    );
    let exiled = board("ai-graveyard-enemy", json!({}), json!({ "exile": [MOONBERRY] }));
    assert!(playable_from_graveyard(
        &buried,
        &buried.players[HUMAN].graveyard[0]
    ));
    // Removing it raises the AI's evaluation by what it counted for the enemy.
    close(
        score(&exiled, &COUNTED) - score(&buried, &COUNTED),
        0.5 * price(1.0),
    );

    let lent = board(
        "ai-graveyard-enemy-sanctum",
        json!({}),
        json!({ "backrow": [SANCTUM], "graveyard": [VANILLA] }),
    );
    let lent_exiled = board(
        "ai-graveyard-enemy-sanctum",
        json!({}),
        json!({ "backrow": [SANCTUM], "exile": [VANILLA] }),
    );
    close(
        score(&lent, &COUNTED) - score(&lent_exiled, &COUNTED),
        -0.5 * price(1.0),
    );
}

#[test]
fn second_wind_and_an_armed_corpse_plantation_admit_their_graveyard_cards() {
    // The Radiant face: the base face's replacement would exile the Vanilla on its way in.
    let wind = board(
        "ai-graveyard-second-wind",
        json!({ "backrow": [{ "def": SECOND_WIND, "radiant": true }], "graveyard": [VANILLA] }),
        json!({}),
    );
    assert!(playable_from_graveyard(&wind, &wind.players[AI].graveyard[0]));
    close(term(&wind), 0.5 * price(1.0));

    let mut plantation = board(
        "ai-graveyard-plantation",
        json!({ "backrow": [PLANTATION], "graveyard": [VANILLA, STOCKPILE] }),
        json!({}),
    );
    close(term(&plantation), 0.0);
    let id = backrow_id(&plantation, AI, PLANTATION);
    let turn = plantation.turn;
    find_instance_mut(&mut plantation, &id)
        .expect("Corpse Plantation")
        .memory
        .insert("armed".to_string(), json!(turn));
    // Units only: the Vanilla counts, the Stockpile does not.
    let graveyard = &plantation.players[AI].graveyard;
    assert!(playable_from_graveyard(&plantation, &graveyard[0]));
    assert!(!playable_from_graveyard(&plantation, &graveyard[1]));
    close(term(&plantation), 0.5 * price(1.0));
}

#[test]
fn the_term_reads_only_what_the_seat_sees() {
    let state = board(
        "ai-graveyard-redacted",
        json!({ "graveyard": [MOONBERRY, MOONBERRY] }),
        json!({ "graveyard": [MOONBERRY], "hand": [VANILLA, STOCKPILE], "backrow": [SHEEPISH] }),
    );
    let public = redact(&state, AI);
    close(term(&public), term(&state));
    close(term(&state), 0.5 * price(1.0));
}

#[test]
fn the_frozen_baseline_keeps_zero() {
    assert_eq!(GREEDY_EVAL.graveyard_playable, 0.0);
    let buried = board(
        "ai-graveyard-moonberry",
        json!({ "graveyard": [MOONBERRY] }),
        json!({}),
    );
    let exiled = board(
        "ai-graveyard-moonberry",
        json!({ "exile": [MOONBERRY] }),
        json!({}),
    );
    close(score(&buried, &GREEDY_EVAL), score(&exiled, &GREEDY_EVAL));
}
