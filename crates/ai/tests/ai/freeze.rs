//! Frozen in the AI's reads (R1174000): the candidates come from `legal_actions`, so a Frozen Unit's
//! attack and switch are never among them; the evaluation leaves a Unit still Frozen next turn out of
//! the face damage it counts (`face_threat_with`, through `can_swing`); and the search signature tells
//! two positions apart that differ only in a freeze.
//!
//! No card freezes yet (M+ #24 Bob is the first, #1185), so each board freezes a unit on the field
//! by writing its thaw, as `freeze::freeze_unit` would. The cards: Mr. Vanilla (core-008) 4/4 and
//! Tempo Timmy (core-011) 3/3.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

/// The AI's Mr. Vanilla ready on the field with a card in hand, and the human's Tempo Timmy; the AI's
/// turn 9.
fn board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "ai-frozen",
        "p1": { "hand": ["core-011"], "field": ["core-008"], "library": LIBRARY },
        "p2": { "hand": ["core-011"], "field": ["core-011"], "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// `player`'s first unit on the field, frozen to thaw at the cleanup of its controller's turn `thaw`
/// or later (`CardInstance.frozen`).
fn freeze(state: &mut GameState, player: PlayerId, thaw: i32) -> String {
    let id = active_units_of(state, player)[0].id.clone();
    find_instance_mut(state, &id).expect("the unit").frozen = Some(thaw);
    id
}

#[test]
fn r1174000_a_frozen_units_attack_and_switch_are_never_candidates() {
    let mut state = board();
    let unit = active_units_of(&state, AI)[0].id.clone();
    let acts = |state: &GameState| {
        candidate_actions(state, AI).iter().any(|action| match action {
            ActionBody::Attack { attacker_id, .. } => *attacker_id == unit,
            ActionBody::SwitchPosition { instance_id } => *instance_id == unit,
            _ => false,
        })
    };
    assert!(acts(&state), "a ready unit attacks or switches");
    let turn = state.turn;
    freeze(&mut state, AI, turn);
    assert!(!acts(&state), "a Frozen one does neither");
}

#[test]
fn r1174000_face_threat_leaves_out_a_unit_still_frozen_next_turn() {
    let state = board();
    let ready = face_threat(&state, AI);
    assert!(ready > 0, "the AI's Mr. Vanilla threatens the human's face");

    // Frozen through its controller's next turn: no swing to count.
    let mut frozen = state.clone();
    let turn = frozen.turn;
    freeze(&mut frozen, AI, turn + 1);
    assert_eq!(face_threat(&frozen, AI), 0);

    // Thawing at this turn's cleanup, its controller's: it swings next turn.
    let mut thawing = state.clone();
    freeze(&mut thawing, AI, turn);
    assert_eq!(face_threat(&thawing, AI), ready);

    // A Frozen unit of the player not on turn is Frozen through that player's next turn, whatever
    // its stamp says.
    let human = face_threat(&state, HUMAN);
    assert!(human > 0);
    let mut theirs = state.clone();
    freeze(&mut theirs, HUMAN, turn);
    assert_eq!(face_threat(&theirs, HUMAN), 0);
}

#[test]
fn r1174000_two_positions_that_differ_only_in_a_freeze_sign_apart() {
    let state = board();
    let mut frozen = state.clone();
    let turn = frozen.turn;
    freeze(&mut frozen, HUMAN, turn + 1);
    assert_ne!(search_signature(&state, AI), search_signature(&frozen, AI));
}
