//! R686: a revealed backrow Trap or Field Trap reads face-up to both players while it stays armed, so
//! the AI reads it as the person across from it does (R185 bars hidden cards, and this one is public).
//! `redact` keeps its face, a determinization leaves it where it is (B10), `evaluate` scores it from
//! its definition, and the search's signature tells it from a face-down card and a face-up one. The
//! same card unrevealed stays hidden, as `redact_live_face_down.rs` and `observe.rs` prove.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, card_by_id, register_cards};

/// C #88 Siphon Squad, (2) Field Trap: "Start of Turn: Reveal."
const SIPHON: &str = "classic-088";

/// `determinize` with TS's default options (`{}`).
fn det(public: &GameState, seat: PlayerId, rng: &mut Rng) -> GameState {
    determinize(public, seat, rng, DeterminizeOptions::default())
}

/// viewFor without its event tail, which B10 exempts.
fn view_without_events(state: &GameState, seat: PlayerId) -> Value {
    let mut view = serde_json::to_value(view_for(state, seat)).expect("a view serialises");
    if let Some(map) = view.as_object_mut() {
        map.remove("events");
    }
    view
}

/// p2's Siphon Squad.
fn siphon(state: &GameState) -> &CardInstance {
    state.players[HUMAN]
        .backrow
        .iter()
        .flatten()
        .find(|card| card.def_id == SIPHON)
        .expect("p2's Siphon Squad is set")
}

/// p1 (the AI) with Midrange Menace on the field against p2's set Siphon Squad. p2's start of turn
/// reveals it, and p1 is to act again with it still armed: revealed, not face-up.
fn revealed_siphon() -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "redact-revealed-backrow",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-019"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
        "p2": { "backrow": [{ "def": SIPHON, "faceUp": false }], "library": ["core-005", "core-016"] },
    }));
    s.end_turn(); // p2's start of turn: the card reveals itself.
    s.end_turn(); // p1's turn again.
    let state = s.state().clone();
    assert_eq!(state.active, AI);
    assert_eq!(
        siphon(&state).revealed,
        Some(true),
        "the start of turn revealed it"
    );
    assert_ne!(siphon(&state).face_up, Some(true), "revealed is not face-up");
    state
}

/// `state` with only the Siphon Squad edited.
fn with_siphon(state: &GameState, edit: impl FnOnce(&mut CardInstance)) -> GameState {
    let mut next = state.clone();
    let id = siphon(state).id.clone();
    edit(find_instance_mut(&mut next, &id).expect("the card is on the field"));
    next
}

/// The same board, the card never revealed: face-down, as p2 set it.
fn unrevealed(state: &GameState) -> GameState {
    with_siphon(state, |card| card.revealed = None)
}

/// The same board, the card flipped face-up instead.
fn flipped(state: &GameState) -> GameState {
    with_siphon(state, |card| {
        card.revealed = None;
        card.face_up = Some(true);
    })
}

mod r686_the_ai_reads_a_revealed_backrow_card {
    use super::*;

    /// R686 a revealed enemy trap is not hidden, and redact keeps its face
    #[test]
    fn r686_a_revealed_enemy_trap_is_not_hidden_and_redact_keeps_its_face() {
        let state = revealed_siphon();
        let id = siphon(&state).id.clone();
        assert!(!hidden_instance_ids(&state, AI).contains(&id));
        assert_eq!(
            card_by_id(&redact(&state, AI), &id).map(|card| card.def_id.clone()),
            Some(SIPHON.to_string())
        );
        // Unrevealed, the same card stays hidden (R33).
        let face_down = unrevealed(&state);
        assert!(hidden_instance_ids(&face_down, AI).contains(&id));
        assert_eq!(
            card_by_id(&redact(&face_down, AI), &id).map(|card| card.def_id.clone()),
            Some(HIDDEN_DEF_ID.to_string())
        );
    }

    /// R686 B10 viewFor of a determinized redaction equals the true view, events aside
    #[test]
    fn r686_b10_view_for_of_a_determinized_redaction_equals_the_true_view_events_aside() {
        let state = revealed_siphon();
        let seen = redact(&state, AI);
        for k in 0..8 {
            let world = det(
                &seen,
                AI,
                &mut create_rng(&format!("redact-revealed-backrow:{k}"), 0),
            );
            assert_eq!(
                view_without_events(&world, AI),
                view_without_events(&state, AI),
                "seed {k}"
            );
        }
    }

    /// R686 evaluate scores a revealed enemy trap from its definition, as it does a face-up one
    #[test]
    fn r686_evaluate_scores_a_revealed_enemy_trap_from_its_definition_as_it_does_a_face_up_one() {
        let state = revealed_siphon();
        let score = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &AI_EVAL);
        assert_eq!(score(&state), score(&flipped(&state)));
        assert_ne!(score(&state), score(&unrevealed(&state)));
    }

    /// R686 the search signature tells a revealed trap from a face-down one and from a face-up one
    #[test]
    fn r686_the_search_signature_tells_a_revealed_trap_from_a_face_down_one_and_from_a_face_up_one() {
        let state = revealed_siphon();
        let signature = search_signature(&state, AI);
        assert_ne!(signature, search_signature(&unrevealed(&state), AI));
        assert_ne!(signature, search_signature(&flipped(&state), AI));
    }
}
