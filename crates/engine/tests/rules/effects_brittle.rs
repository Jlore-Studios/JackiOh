//! Brittle X's verbs (docs/classic-sets.md B3.3 rule 4; R385, R440, R441): "Give Brittle N" sets the
//! count and starts it now, "gain +N Brittle" adds to it, on a named card anywhere or over a card scope,
//! and what each reports to whom.
//!
//! Port of `packages/engine/test/effects-brittle.test.ts`.

use jackioh_engine::effects::{gain_brittle, give_brittle};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{brittle_trap, brittle_unit, instance_game};

fn game() -> GameState {
    let mut state = instance_game("brittle-verbs", None);
    state.turn = 5;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// TS `run(state, effect, { self, ...hook })`: the effect applied for p1 (unless `hook` names another
/// controller) on a sink over `state`, the rng cursor written back; its events.
fn run(
    state: &mut GameState,
    effect: Effect,
    self_: Option<CardInstance>,
    hook: HookOptions,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let controller = hook.controller.unwrap_or(PlayerId::P1);
        let mut ctx = make_context(
            &mut sink,
            self_.as_ref(),
            HookOptions {
                controller: Some(controller),
                ..hook
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn chosen(id: &str) -> HookOptions {
    HookOptions {
        targets: Some(vec![Selection::Instance {
            instance_id: id.to_string(),
        }]),
        ..Default::default()
    }
}

fn brittle_of(state: &GameState, id: &str) -> Option<BrittleCounter> {
    find_instance(state, id)
        .expect("the card is in the state")
        .brittle
}

fn as_json(events: &[GameEvent]) -> Value {
    serde_json::to_value(events).unwrap()
}

mod b3_3_rule_4_give_and_gain_r385 {
    use super::*;

    #[test]
    fn r385_give_brittle_n_sets_the_count_to_n_from_now_whatever_the_card_had() {
        let mut state = game();
        let unit = put(
            &mut state,
            &brittle_unit.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        assert_eq!(
            brittle_of(&state, &unit.id),
            Some(BrittleCounter {
                count: 2,
                since: 5,
                printed: Some(true)
            })
        );
        state.turn = 7;
        let events = run(
            &mut state,
            give_brittle(json_as(json!({ "instanceId": unit.id, "n": 5 }))),
            None,
            HookOptions::default(),
        );
        assert_eq!(
            brittle_of(&state, &unit.id),
            Some(BrittleCounter {
                count: 5,
                since: 7,
                printed: None
            })
        );
        assert_eq!(
            as_json(&events),
            json!([{ "type": "counterChanged", "instanceId": unit.id, "counter": "brittle", "value": 5 }])
        );
    }

    #[test]
    fn r385_gain_n_adds_to_the_count_in_force_and_keeps_when_it_started() {
        let mut state = game();
        let unit = put(
            &mut state,
            &brittle_unit.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        state.turn = 8;
        let self_ = find_instance(&state, &unit.id).cloned();
        run(
            &mut state,
            gain_brittle(json_as(json!({ "target": { "of": "self" }, "n": 1 }))),
            self_,
            HookOptions::default(),
        );
        assert_eq!(
            brittle_of(&state, &unit.id),
            Some(BrittleCounter {
                count: 3,
                since: 5,
                printed: Some(true)
            })
        );
    }

    #[test]
    fn r385_a_named_card_in_a_hand_or_a_deck_takes_a_count_too_reported_to_whoever_may_read_it_r97() {
        let mut state = game();
        let Some(held) = in_hand(&mut state, &plain.id, PlayerId::P1, 1).into_iter().next() else {
            panic!("no card");
        };
        let Some(deck) = set_library(&mut state, PlayerId::P1, std::slice::from_ref(&plain.id))
            .into_iter()
            .next()
        else {
            panic!("no card");
        };
        let events = run(
            &mut state,
            give_brittle(json_as(json!({ "target": { "of": "chosen" }, "n": 2 }))),
            None,
            chosen(&held.id),
        );
        run(
            &mut state,
            give_brittle(json_as(json!({ "instanceId": deck.id, "n": 2 }))),
            None,
            HookOptions::default(),
        );
        assert_eq!(
            brittle_of(&state, &held.id),
            Some(BrittleCounter {
                count: 2,
                since: 5,
                printed: None
            })
        );
        assert_eq!(
            brittle_of(&state, &deck.id),
            Some(BrittleCounter {
                count: 2,
                since: 5,
                printed: None
            })
        );
        state.applied = vec![AppliedAction {
            nonce: "t".to_string(),
            events,
        }];
        assert_eq!(
            as_json(&view_for(&state, PlayerId::P1).events),
            json!([{ "type": "counterChanged", "instanceId": held.id, "counter": "brittle", "value": 2 }])
        );
        // The other player may not read the card, so neither its id nor its count (R385).
        assert_eq!(
            as_json(&view_for(&state, PlayerId::P2).events),
            json!([{ "type": "counterChanged", "instanceId": HIDDEN_ID, "counter": "brittle", "value": -1 }])
        );
    }

    #[test]
    fn r440_over_a_scope_a_card_of_a_hidden_pile_takes_its_count_silently_a_public_one_is_reported() {
        let mut state = game();
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let trap = put(
            &mut state,
            &brittle_trap.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let hand = in_hand(&mut state, &plain.id, PlayerId::P1, 3);
        let events = run(
            &mut state,
            give_brittle(json_as(
                json!({ "scope": { "zones": ["field", "hand"] }, "n": 2 }),
            )),
            None,
            HookOptions::default(),
        );
        let mut cards = vec![unit.clone(), trap.clone()];
        cards.extend(hand);
        for card in &cards {
            assert_eq!(
                brittle_of(&state, &card.id),
                Some(BrittleCounter {
                    count: 2,
                    since: 5,
                    printed: None
                })
            );
        }
        assert_eq!(
            as_json(&events),
            json!([{ "type": "counterChanged", "instanceId": unit.id, "counter": "brittle", "value": 2 }])
        );
    }

    #[test]
    fn r385_a_card_that_has_ceased_to_exist_takes_nothing() {
        let mut state = game();
        let Some(mut held) = in_hand(&mut state, &plain.id, PlayerId::P1, 1).into_iter().next() else {
            panic!("no card");
        };
        state.players[PlayerId::P1].hand = vec![];
        held.zone = Zone::Gone { player: PlayerId::P1 };
        assert_eq!(
            run(
                &mut state,
                give_brittle(json_as(json!({ "target": { "of": "chosen" }, "n": 2 }))),
                None,
                chosen(&held.id)
            ),
            Vec::<GameEvent>::new()
        );
        // TS read the detached object, which nothing can reach: it is in no zone of the state.
        assert!(find_instance(&state, &held.id).is_none());
        assert_eq!(held.brittle, None);
    }
}
