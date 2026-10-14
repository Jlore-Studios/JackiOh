//! R874000: a trap answers an identical arrival twice per action (SPEC §10.3; the Tesla/Flicker
//! loop of issue #874).
//!
//!   * The second identical twin still answers: a Flicker re-summoning the same Unit re-offers the
//!     same `summoned` (§6.3: it counts as summoned), and the finite return — a Reborn body, a
//!     second Flicker — is answered like the first.
//!   * The third identical twin in one action's resolution is always the same stay coming back
//!     through a replacement loop, so it is declined: a Field Trap answering every twin
//!     (Tesla into a Flicker replacement) loops forever, past the settle cap's reach.
//!   * Other arrivals still answer: the same trap fires again on a different Unit's summon.
//!
//! The events are offered on one sink, as one action's resolution loop offers them: flickers of
//! the same Unit, then one settle. Fixtures: `fixtures/field.rs` (the Tesla shape),
//! `fixtures/combat.rs`.

use jackioh_engine::effects::flicker::flicker;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::field::{playing, tesla};
use crate::rules::fixtures::harness::{events_of_type, put, slot};

/// TS `sinkFor(state)`: the events and rng of a sink over `state`, the rng starting at the state's
/// cursor as reduce does. The state is lent to it call by call (`on`), so a test reads the state in
/// between as TS read its live objects.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> Sink {
    Sink {
        events: Vec::new(),
        rng: Rng::new(&state.seed, state.rng_cursor),
    }
}

impl Sink {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }

    /// `effect.apply(makeContext(sink, null, { controller }))`.
    fn apply(&mut self, state: &mut GameState, effect: Effect, controller: PlayerId) {
        let mut sink = self.on(state);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(controller),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
}

fn flicker_unit(unit: &CardInstance) -> Effect {
    flicker(json_as(
        json!({ "target": { "of": "instance", "instanceId": unit.id } }),
    ))
}

fn trap_fired_ids(events: &[GameEvent]) -> Vec<String> {
    events_of_type(events, GameEventType::TrapFired)
        .iter()
        .filter_map(|event| match event {
            GameEvent::TrapFired { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn destroyed_ids(events: &[GameEvent]) -> Vec<String> {
    events_of_type(events, GameEventType::Destroyed)
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

mod trap_answers_an_identical_arrival_twice_r874000 {
    use super::*;

    /// A 10/14 radiant stacker: it lives through two 4-damage zaps, so a declined summon below is
    /// declined by the count alone — the victim stands on the field the whole time.
    fn tough_unit(state: &mut GameState) -> CardInstance {
        put(
            state,
            &stacker.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({ "radiant": true }),
        )
    }

    fn zapper(state: &mut GameState) -> CardInstance {
        put(state, &tesla.id, slot(PlayerId::P2, Row::Backrow, 1), json!({}))
    }

    #[test]
    fn r874000_answers_two_identical_flicker_summons() {
        let mut state = playing("r874000-repeat");
        let zapper = zapper(&mut state);
        let unit = tough_unit(&mut state);
        let mut sink = sink_for(&state);
        // Two flickers of the same Unit, then one settle offering both identical summons.
        sink.apply(&mut state, flicker_unit(&unit), PlayerId::P1);
        sink.apply(&mut state, flicker_unit(&unit), PlayerId::P1);
        settle(&mut sink.on(&mut state), Default::default());

        // Finite twins both answer: two firings, two hits, and the victim still stands.
        assert_eq!(
            trap_fired_ids(&sink.events),
            vec![zapper.id.clone(), zapper.id.clone()]
        );
        assert_eq!(destroyed_ids(&sink.events), Vec::<String>::new());
    }

    #[test]
    fn r874000_declines_the_third_identical_summon() {
        let mut state = playing("r874000-third");
        let zapper = zapper(&mut state);
        let unit = tough_unit(&mut state);
        let mut sink = sink_for(&state);
        // Three flickers of the same Unit, then one settle offering three identical summons.
        sink.apply(&mut state, flicker_unit(&unit), PlayerId::P1);
        sink.apply(&mut state, flicker_unit(&unit), PlayerId::P1);
        sink.apply(&mut state, flicker_unit(&unit), PlayerId::P1);
        settle(&mut sink.on(&mut state), Default::default());

        // The first two answer; the third, with the victim alive on the field, is declined by the
        // count alone, so the Tesla/Flicker cycle ends instead of looping: two firings, not three.
        assert_eq!(
            trap_fired_ids(&sink.events),
            vec![zapper.id.clone(), zapper.id.clone()]
        );
        assert_eq!(destroyed_ids(&sink.events), Vec::<String>::new());
    }

    #[test]
    fn r874000_answers_a_different_units_summon_again() {
        let mut state = playing("r874000-different");
        let zapper = put(
            &mut state,
            &tesla.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let unit = put(
            &mut state,
            &stacker.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let other = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let mut sink = sink_for(&state);
        sink.apply(&mut state, flicker_unit(&unit), PlayerId::P1);
        sink.apply(&mut state, flicker_unit(&other), PlayerId::P1);
        settle(&mut sink.on(&mut state), Default::default());

        // Different arrivals are different answers: the guard skips third twins, nothing else.
        assert_eq!(
            trap_fired_ids(&sink.events),
            vec![zapper.id.clone(), zapper.id.clone()]
        );
    }
}
