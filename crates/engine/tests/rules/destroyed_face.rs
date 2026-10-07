//! Port of `packages/engine/test/destroyed-face.test.ts`.
//!
//! R89: the `destroyed` event says what the unit was as it died, its Radiant face included — the one
//! fact a unit token that has ceased to exist can no longer tell (C+ #12.8 Frostspatula remembers each
//! Unit it killed by definition and face, R409). Absent on a base face, so nothing else moves.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::P2;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::field::playing;
use crate::rules::fixtures::harness::slot;

/// `put(state, defId, ref, { radiant })` (fixtures/harness.ts), face and all: a hand instance made
/// Radiant before it is placed, as the TS helper does.
fn put_face(state: &mut GameState, def_id: &str, at: ZoneSlot, radiant: bool) -> CardInstance {
    let (player, row, lane) = (at.player, at.row, at.lane);
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    if radiant {
        card.radiant = true;
    }
    let id = card.id.clone();
    if !place_on_field(state, card, at, Default::default()) {
        panic!("could not place {def_id} in {row} {lane}");
    }
    find_instance(state, &id).cloned().expect("placed card")
}

fn killed(radiant: bool) -> Option<GameEvent> {
    let mut state = playing(&format!("destroyed-face-{radiant}"));
    let unit = put_face(&mut state, &plain().id, slot(P2, Row::Units, 1), radiant);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
    deal_damage(
        &mut sink,
        DamageArgs {
            source: None,
            target: DamageTarget::Unit { instance: unit },
            amount: 99,
            flags: None,
        },
    );
    state_check(&mut sink);
    events
        .iter()
        .find(|event| matches!(event, GameEvent::Destroyed { .. }))
        .cloned()
}

/// `describe("R89 a destroyed unit's face")`.
mod r89_a_destroyed_unit_s_face {
    use super::*;

    #[test]
    fn r89_a_radiant_unit_s_destroyed_event_says_so() {
        let event = killed(true);
        assert!(
            matches!(event, Some(GameEvent::Destroyed { radiant: Some(true), .. })),
            "expected a destroyed event with radiant: true, got {event:?}"
        );
    }

    #[test]
    fn r89_a_base_face_unit_s_event_carries_no_flag() {
        let event = killed(false);
        assert_eq!(event.as_ref().map(|e| e.event_type()), Some(GameEventType::Destroyed));
        let carries_flag = match &event {
            Some(event) => serde_json::to_value(event).expect("event serialises").get("radiant").is_some(),
            None => false,
        };
        assert!(!carries_flag);
    }
}
