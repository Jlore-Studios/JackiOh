//! Reveal (R686): showing a backrow Trap or Field Trap to both players while it stays armed —
//! Classic #88 Siphon Squad's "Start of Turn: Reveal" and Classic #65 Ace in the Hole's Radiant
//! "Revealed regardless of the coin flip".

use jackioh_engine::testkit::*;

use jackioh_engine::effects::reveal;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::EngineSink;
use jackioh_engine::state::{CardInstance, GameState};
use jackioh_engine::traps::is_spent;
use jackioh_engine::view_for::view_for;

use super::fixtures::combat::plain;
use super::fixtures::field::{doom, listener, playing};
use super::fixtures::harness::{put, slot};
use crate::rules::fixtures::harness::{live, matches_object};

/// `reveal()` applied as `card`, the rng at the state's cursor; the cursor is not written back.
fn reveal_as(state: &mut GameState, card: &CardInstance) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    let mut ctx = make_context(&mut sink, Some(card), HookOptions::default());
    (reveal(Default::default()).apply)(&mut ctx);
}

/// The opponent's view of `owner`'s backrow zone `lane`, as JSON.
fn backrow_seen_by(state: &GameState, viewer: PlayerId, lane: usize) -> Value {
    let view = serde_json::to_value(view_for(state, viewer)).expect("a view serializes");
    view["opponent"]["backrow"][lane].clone()
}

mod r686_reveal {
    use super::*;

    #[test]
    fn r686_a_revealed_trap_reads_face_up_to_the_opponent_but_is_not_spent_so_it_still_fires() {
        let mut state = playing("reveal-trap");
        let trap = put(
            &mut state,
            &doom.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        reveal_as(&mut state, &trap);

        assert_eq!(live(&state, &trap.id).revealed, Some(true));
        // The opponent reads the face now, not a back.
        assert!(matches_object(
            &backrow_seen_by(&state, PlayerId::P2, 0),
            &json!({ "faceDown": false, "defId": doom.id })
        ));
        // …but it has not fired: a Trap that is merely revealed still answers.
        assert!(!is_spent(&state, &live(&state, &trap.id)));
    }

    #[test]
    fn r686_a_revealed_field_trap_stays_live_the_opponent_reads_it_and_it_keeps_firing() {
        let mut state = playing("reveal-field-trap");
        let trap = put(
            &mut state,
            &listener.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        reveal_as(&mut state, &trap);

        assert_eq!(live(&state, &trap.id).revealed, Some(true));
        assert!(matches_object(
            &backrow_seen_by(&state, PlayerId::P2, 1),
            &json!({ "faceDown": false, "defId": listener.id })
        ));
        assert!(!is_spent(&state, &live(&state, &trap.id)));
    }

    #[test]
    fn r686_reveal_touches_only_a_backrow_card_a_unit_is_unchanged() {
        let mut state = playing("reveal-unit");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        reveal_as(&mut state, &unit);

        assert_eq!(live(&state, &unit.id).revealed, None);
    }
}
