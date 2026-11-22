//! R89: the `destroyed` event says what the unit was as it died, its Radiant face included — the one
//! fact a unit token that has ceased to exist can no longer tell (C+ #12.8 Frostspatula remembers each
//! Unit it killed by definition and face, R409). Absent on a base face, so nothing else moves.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::P2;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::field::playing;
use crate::rules::fixtures::harness::{put, slot};

fn killed(radiant: bool) -> Option<GameEvent> {
    let mut state = playing(&format!("destroyed-face-{radiant}"));
    let unit = put(
        &mut state,
        &plain.id,
        slot(P2, Row::Units, 1),
        json!({ "radiant": radiant }),
    );
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

mod r89_a_destroyed_unit_s_face {
    use super::*;

    #[test]
    fn r89_a_radiant_unit_s_destroyed_event_says_so() {
        let event = killed(true);
        assert!(
            matches!(
                event,
                Some(GameEvent::Destroyed {
                    radiant: Some(true),
                    ..
                })
            ),
            "expected a destroyed event with radiant: true, got {event:?}"
        );
    }

    #[test]
    fn r89_a_base_face_unit_s_event_carries_no_flag() {
        let event = killed(false);
        assert_eq!(
            event.as_ref().map(|e| e.event_type()),
            Some(GameEventType::Destroyed)
        );
        let carries_flag = match &event {
            Some(event) => serde_json::to_value(event)
                .expect("event serializes")
                .get("radiant")
                .is_some(),
            None => false,
        };
        assert!(!carries_flag);
    }
}

mod r4423000_what_a_card_died_as {
    use super::*;
    use crate::rules::fixtures::field::banner;

    fn died(def_id: &str, died_as: Option<CardType>) -> GameEvent {
        GameEvent::Destroyed {
            instance_id: "c9".into(),
            def_id: def_id.into(),
            owner: P2,
            controller: P2,
            attack: 1,
            max_health: 1,
            killer_id: None,
            radiant: None,
            animated: None,
            died_as,
            via: None,
        }
    }

    #[test]
    fn r4423000_an_ordinary_unit_s_destroyed_event_carries_no_died_as() {
        for radiant in [false, true] {
            let event = killed(radiant).expect("a destroyed event");
            let json = serde_json::to_value(&event).expect("event serializes");
            assert!(json.get("diedAs").is_none(), "{json}");
            assert!(query::is_unit_death(&playing("is-unit-death"), &event));
        }
    }

    #[test]
    fn r4423000_is_unit_death_reads_the_printed_face_unless_the_event_says_otherwise() {
        let state = playing("is-unit-death");
        assert!(query::is_unit_death(&state, &died(&plain.id, None)));
        assert!(!query::is_unit_death(
            &state,
            &died(&plain.id, Some(CardType::FieldTrap))
        ));
        assert!(!query::is_unit_death(&state, &died(&banner.id, None)));
        assert!(query::is_unit_death(
            &state,
            &died(&banner.id, Some(CardType::Unit))
        ));
        assert!(!query::is_unit_death(
            &state,
            &GameEvent::DivineShieldLost {
                instance_id: "c9".into()
            }
        ));
    }
}
