//! T-AI-3 Hallucination's verb, `addLibraryCopies` (SPEC §8.7 row T-AI-3; R57, R60, R97, R129, R385):
//! copies of different random cards of a deck into the caster's hand, Brittle given to each that lands.
//!
//! Port of `packages/engine/test/library-copies.test.ts`.

use std::collections::BTreeSet;

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::effects::add_library_copies;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, set_library};

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// TS `run(state, effect)`: the effect applied for p1 with no card of its own, the rng cursor written
/// back; its events.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        let mut ctx = make_context(
            sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn copies(args: Value) -> Effect {
    add_library_copies(json_as(args))
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

/// The `addedToHand` events' `[instanceId, defId]`.
fn added_to_hand(events: &[GameEvent]) -> Vec<(String, String)> {
    events_of_type(events, GameEventType::AddedToHand)
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::AddedToHand {
                instance_id, def_id, ..
            } => Some((instance_id, def_id)),
            _ => None,
        })
        .collect()
}

/// TS `/fx-3[234]/`.
fn names_a_source(text: &str) -> bool {
    ["fx-32", "fx-33", "fx-34"].iter().any(|id| text.contains(id))
}

/// addLibraryCopies (T-AI-3)
mod add_library_copies_t_ai_3 {
    use super::*;

    #[test]
    fn r57_a_copy_is_a_new_card_the_caster_owns_with_its_source_s_definition_radiant_flag_stats_override_and_tuning_the_source_stays()
     {
        let mut state = game("copies-carry");
        let source = set_library(&mut state, P2, &["fx-5"])
            .into_iter()
            .next()
            .expect("a deck card");
        {
            let live = &mut state.players.p2.library[0];
            live.radiant = true;
            live.stats_override = Some(AttackHealth { attack: 2, health: 3 });
            live.tuning = Some(Tuning {
                attack: Some(1),
                ..Tuning::default()
            });
            live.cost_mod = 1;
        }
        run(&mut state, copies(json!({ "of": "enemy", "count": 1, "brittle": 2 })));
        let copy = state.players.p1.hand.last().cloned().expect("a copy in hand");
        let copy_json = serde_json::to_value(&copy).expect("serialises");
        assert!(
            matches_object(
                &copy_json,
                &json!({ "defId": "fx-5", "owner": "p1", "controller": "p1", "radiant": true, "costMod": 0 })
            ),
            "{copy_json}"
        );
        assert_ne!(copy.id, source.id);
        assert_eq!(copy.stats_override, Some(AttackHealth { attack: 2, health: 3 }));
        assert_eq!(
            copy.tuning,
            Some(Tuning {
                attack: Some(1),
                ..Tuning::default()
            })
        );
        assert_eq!(
            copy.brittle,
            Some(BrittleCounter {
                count: 2,
                since: 3,
                printed: None
            })
        );
        assert_eq!(
            state.players.p2.library.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            vec![source.id.clone()]
        );
    }

    #[test]
    fn r586_r60_the_copies_are_of_n_different_cards_and_reach_the_hand_in_the_order_drawn_never_the_deck_s() {
        let mut orders: BTreeSet<String> = BTreeSet::new();
        for seed in 1..=12 {
            let mut state = game(&format!("copies-order-{seed}"));
            let deck = set_library(&mut state, P2, &["fx-21", "fx-22", "fx-23", "fx-24", "fx-25", "fx-26"]);
            let before = state.players.p1.hand.len();
            run(&mut state, copies(json!({ "of": "enemy", "count": 2 })));
            let copies: Vec<CardInstance> = state.players.p1.hand[before..].to_vec();
            assert_eq!(copies.len(), 2);
            assert_eq!(copies.iter().map(|card| card.def_id.clone()).collect::<BTreeSet<_>>().len(), 2);
            let positions: Vec<usize> = copies
                .iter()
                .map(|copy| {
                    deck.iter()
                        .position(|card| card.def_id == copy.def_id)
                        .expect("a copy of a deck card")
                })
                .collect();
            orders.insert(if positions[0] < positions[1] { "deck order" } else { "reversed" }.to_string());
        }
        // Not the deck's order: it would tell the caster where the sources lay.
        assert_eq!(
            orders,
            ["deck order", "reversed"].into_iter().map(String::from).collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn r129_an_empty_deck_gives_nothing_and_draws_nothing_a_deck_of_no_more_than_n_cards_gives_each_with_no_draw() {
        let mut empty = game("copies-empty");
        set_library(&mut empty, P2, &[] as &[&str]);
        let cursor = empty.rng_cursor;
        let before = empty.players.p1.hand.len();
        assert_eq!(run(&mut empty, copies(json!({ "of": "enemy", "count": 2, "brittle": 2 }))), Vec::<GameEvent>::new());
        assert_eq!(empty.players.p1.hand.len(), before);
        assert_eq!(empty.rng_cursor, cursor);

        let mut short = game("copies-short");
        set_library(&mut short, P2, &["fx-30"]);
        let at = short.rng_cursor;
        run(&mut short, copies(json!({ "of": "enemy", "count": 2 })));
        assert_eq!(short.players.p1.hand.last().map(|card| card.def_id.as_str()), Some("fx-30"));
        assert_eq!(short.rng_cursor, at);
    }

    #[test]
    fn r586_a_deck_of_no_more_than_n_cards_is_copied_whole_in_definition_order_so_the_hand_never_shows_the_deck_s_order() {
        let mut state = game("copies-whole");
        set_library(&mut state, P2, &["fx-35", "fx-31"]);
        let before = state.players.p1.hand.len();
        let cursor = state.rng_cursor;
        run(&mut state, copies(json!({ "of": "enemy", "count": 2 })));
        assert_eq!(
            state.players.p1.hand[before..].iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            vec!["fx-31", "fx-35"]
        );
        assert_eq!(state.rng_cursor, cursor);
    }

    #[test]
    fn r586_s2_4_a_copy_the_hand_cap_burns_goes_to_the_caster_s_graveyard_and_takes_no_brittle() {
        let mut state = game("copies-burn");
        let room = HAND_CAP - state.players.p1.hand.len() as i32;
        in_hand(&mut state, "fx-1", P1, room);
        set_library(&mut state, P2, &["fx-31"]);
        let events = run(&mut state, copies(json!({ "of": "enemy", "count": 1, "brittle": 2 })));
        let burned = state.players.p1.graveyard.last().cloned();
        assert_eq!(burned.as_ref().map(|card| card.def_id.as_str()), Some("fx-31"));
        assert_eq!(burned.and_then(|card| card.brittle), None);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 1);
        assert_eq!(events_of_type(&events, GameEventType::CounterChanged), Vec::<GameEvent>::new());
    }

    #[test]
    fn r97_the_other_player_reads_only_that_a_card_reached_the_hand_their_deck_is_untouched() {
        let mut state = game("copies-view");
        let deck = set_library(&mut state, P2, &["fx-32", "fx-33", "fx-34"]);
        let events = run(&mut state, copies(json!({ "of": "enemy", "count": 1, "brittle": 2 })));
        state.applied = vec![AppliedAction {
            nonce: "lc".to_string(),
            events,
        }];
        let theirs = view_for(&state, P2).events;
        for (instance_id, def_id) in added_to_hand(&theirs) {
            assert_eq!(
                (instance_id, def_id),
                (HIDDEN_ID.to_string(), HIDDEN_ID.to_string())
            );
        }
        assert_eq!(events_of_type(&theirs, GameEventType::AddedToHand).len(), 1);
        assert!(!names_a_source(&serde_json::to_string(&theirs).expect("serialises")));
        assert_eq!(
            state.players.p2.library.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            deck.iter().map(|card| card.id.clone()).collect::<Vec<_>>()
        );
        let mine = added_to_hand(&view_for(&state, P1).events);
        assert!(names_a_source(&mine.first().expect("an addedToHand event").1));
    }
}
