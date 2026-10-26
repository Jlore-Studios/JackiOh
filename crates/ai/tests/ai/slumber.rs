//! Dormant in the AI's reads (R1839000): the candidates come from `legal_actions`, so a Dormant card's
//! attack, switch and abilities, and an attack on one, are never among them, and neither `decide` nor the
//! random policy ever acts through it; the evaluation counts it as no Unit and no threat until it wakes
//! (`active_units_of` leaves it out); and the search signature tells a position holding one apart from
//! one without it.
//!
//! No shipped card is Dormant yet (Stinkyscape's come with the set), so each board puts a unit on the
//! field to sleep by writing its status, as `slumber::start_slumber_on_field` would. The cards: Mr.
//! Vanilla (core-008) 4/4 and Tempo Timmy (core-011) 3/3.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// The AI's Mr. Vanilla ready on the field with a card in hand, and the human's Tempo Timmy; the AI's
/// turn 9.
fn board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "ai-dormant",
        "p1": { "hand": ["core-011"], "field": ["core-008"], "library": LIBRARY },
        "p2": { "hand": ["core-011"], "field": ["core-011"], "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// `player`'s first unit on the field, put to sleep for two of its controller's turns.
fn sleep(state: &mut GameState, player: PlayerId) -> String {
    let id = active_units_of(state, player)[0].id.clone();
    find_instance_mut(state, &id).expect("the unit").slumber = Some(Slumber { turns: Some(2) });
    id
}

/// `state` with `player`'s lane-1 unit zone emptied.
fn emptied(state: &GameState, player: PlayerId) -> GameState {
    let mut out = state.clone();
    out.players[player].units[0] = None;
    out
}

fn names(action: &ActionBody, id: &str) -> bool {
    serde_json::to_string(action)
        .expect("serialises")
        .contains(&format!("\"{id}\""))
}

#[test]
fn r1839000_a_dormant_card_is_never_in_a_candidate_as_actor_or_target() {
    let mut state = board();
    let own = active_units_of(&state, AI)[0].id.clone();
    let theirs = active_units_of(&state, HUMAN)[0].id.clone();
    assert!(
        candidate_actions(&state, AI)
            .iter()
            .any(|action| names(action, &own))
    );
    assert!(
        candidate_actions(&state, AI)
            .iter()
            .any(|action| names(action, &theirs))
    );
    sleep(&mut state, AI);
    sleep(&mut state, HUMAN);
    assert!(
        !candidate_actions(&state, AI)
            .iter()
            .any(|action| names(action, &own) || names(action, &theirs)),
        "neither the AI's Dormant card nor the human's is in any candidate"
    );
}

#[test]
fn r1839000_decide_and_the_random_policy_never_act_through_a_dormant_card() {
    let mut state = board();
    let own = sleep(&mut state, AI);
    let theirs = sleep(&mut state, HUMAN);
    let decision = decide(&state, AI, &mut AiOptions::new(create_rng("ai-dormant", 0))).expect("a decision");
    assert!(
        !names(&decision.action, &own) && !names(&decision.action, &theirs),
        "{:?}",
        decision.action
    );
    for cursor in 0..32 {
        let mut rng = create_rng("ai-dormant-policy", cursor);
        let action = subsystems::ai_policy::choose_action(&state, AI, &mut rng).expect("an action");
        assert!(!names(&action, &own) && !names(&action, &theirs), "{action:?}");
    }
}

#[test]
fn r1839000_the_evaluation_counts_a_dormant_card_as_no_unit_and_no_threat_until_it_wakes() {
    let awake = board();
    let mut dormant = awake.clone();
    sleep(&mut dormant, HUMAN);
    let gone = emptied(&awake, HUMAN);
    // The human's Timmy threatens the AI's face awake, and nothing asleep.
    assert!(face_threat(&awake, HUMAN) > 0);
    assert_eq!(face_threat(&dormant, HUMAN), 0);
    // To the AI's evaluation the sleeping Timmy is as if its zone were empty, and better for the AI than
    // the awake one.
    let eval = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &AI_EVAL);
    assert!(
        (eval(&dormant) - eval(&gone)).abs() < 1e-9,
        "{} vs {}",
        eval(&dormant),
        eval(&gone)
    );
    assert!(eval(&dormant) > eval(&awake));
    // Its own Dormant card counts for nothing either, until it wakes.
    let mut own = awake.clone();
    sleep(&mut own, AI);
    assert!((eval(&own) - eval(&emptied(&awake, AI))).abs() < 1e-9);
}

#[test]
fn r1839000_two_positions_that_differ_only_in_a_dormant_card_sign_apart() {
    let awake = board();
    let mut dormant = awake.clone();
    let id = sleep(&mut dormant, HUMAN);
    let gone = emptied(&awake, HUMAN);
    assert_ne!(search_signature(&dormant, AI), search_signature(&gone, AI));
    assert_ne!(search_signature(&dormant, AI), search_signature(&awake, AI));
    // Its countdown is part of the position too.
    let mut later = dormant.clone();
    find_instance_mut(&mut later, &id).expect("the unit").slumber = Some(Slumber { turns: Some(1) });
    assert_ne!(search_signature(&dormant, AI), search_signature(&later, AI));
}
