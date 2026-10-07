//! §6.3 Exile at random out of a hand — `exileRandomFromHand` (effects/handExile.ts), the verb Classic
//! #15 Nose Hunter's Radiant face needs ("and a random card from their hand"). SPEC §6.3 (Exile), §3.2
//! (exile is public), R11, R55, R60. The real card's test (packages/cards/test/classic/
//! 015-nose-hunter.test.ts) covers the same cases again through the card.
//!
//! Port of `packages/engine/test/effects-hand-exile.test.ts`.

use jackioh_engine::effects::hand_exile::exile_random_from_hand;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game};

/// TS `run(state, effect, controller)`: one effect on a sink over `state`, the rng cursor written back;
/// its events.
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
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
    state.rng_cursor = rng.cursor();
    events
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match serde_json::to_value(events).expect("events serialise") {
        Value::Array(items) => items
            .into_iter()
            .map(|item| item.get(key).cloned().unwrap_or(Value::Null))
            .collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

mod exile_random_from_hand_c_15_nose_hunter {
    use super::*;

    #[test]
    fn r60_r55_exiles_one_random_card_of_the_named_player_s_hand_to_its_owner_s_exile_and_counts_it() {
        let mut state = new_game("hand-exile-one", None);
        let before: Vec<String> = in_hand(&mut state, "fx-1", PlayerId::P2, 3)
            .into_iter()
            .map(|card| card.id)
            .collect();
        in_hand(&mut state, "fx-2", PlayerId::P1, 2);

        let events = run(
            &mut state,
            exile_random_from_hand(json_as(json!({ "player": "enemy" }))),
            PlayerId::P1,
        );

        let exiled: Vec<String> = state.players[PlayerId::P2]
            .exile
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(exiled.len(), 1);
        assert!(before.contains(&exiled[0]));
        assert_eq!(hand_ids(&state, PlayerId::P2).len(), 2);
        assert!(!hand_ids(&state, PlayerId::P2).contains(&exiled[0]));
        assert_eq!(state.players[PlayerId::P1].hand.len(), 2);
        assert_eq!(state.counters.exiled, 1);
        assert_eq!(
            pluck(&events_of_type(&events, GameEventType::Exiled), "instanceId"),
            exiled.iter().map(|id| json!(id)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn r60_a_pick_of_n_takes_n_different_cards_or_the_whole_hand_when_it_holds_fewer() {
        let mut state = new_game("hand-exile-many", None);
        in_hand(&mut state, "fx-1", PlayerId::P1, 2);

        run(
            &mut state,
            exile_random_from_hand(json_as(json!({ "count": 5 }))),
            PlayerId::P1,
        );

        assert_eq!(state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(state.players[PlayerId::P1].exile.len(), 2);
        let distinct: IndexSet<String> = state.players[PlayerId::P1]
            .exile
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(distinct.len(), 2);
    }

    #[test]
    fn an_empty_hand_fizzles_nothing_moves_no_event_and_the_rng_is_not_drawn_from() {
        let mut state = new_game("hand-exile-empty", None);
        state.players[PlayerId::P2].hand = vec![];
        let cursor = state.rng_cursor;

        let events = run(
            &mut state,
            exile_random_from_hand(json_as(json!({ "player": "enemy" }))),
            PlayerId::P1,
        );

        assert_eq!(events, Vec::<GameEvent>::new());
        assert_eq!(state.players[PlayerId::P2].exile.len(), 0);
        assert_eq!(state.rng_cursor, cursor);
    }

    #[test]
    fn r60_the_pick_is_the_match_rng_s_the_same_seed_and_cursor_exile_the_same_card() {
        let pick = |seed: &str| -> String {
            let mut state = new_game(seed, None);
            let cards = in_hand(&mut state, "fx-1", PlayerId::P2, 4);
            run(
                &mut state,
                exile_random_from_hand(json_as(json!({ "player": "enemy" }))),
                PlayerId::P1,
            );
            // TS `String(cards.findIndex((card) => card.zone.z === "exile"))`: -1 when none is.
            let index = cards
                .iter()
                .position(|card| {
                    find_instance(&state, &card.id).is_some_and(|now| now.zone.z() == ZoneName::Exile)
                })
                .map_or(-1, |index| index as i64);
            index.to_string()
        };
        assert_eq!(pick("hand-exile-seed"), pick("hand-exile-seed"));
    }

    #[test]
    fn r11_a_unit_token_card_in_a_hand_ceases_to_exist_rather_than_reaching_the_exile_and_is_not_counted() {
        let mut state = new_game("hand-exile-token", None);
        let token = in_hand(&mut state, "fx-token-rush", PlayerId::P2, 1)
            .into_iter()
            .next();

        let events = run(
            &mut state,
            exile_random_from_hand(json_as(json!({ "player": "enemy" }))),
            PlayerId::P1,
        );

        assert_eq!(state.players[PlayerId::P2].hand.len(), 0);
        assert_eq!(state.players[PlayerId::P2].exile.len(), 0);
        assert_eq!(state.counters.exiled, 0);
        assert_eq!(
            pluck(&events_of_type(&events, GameEventType::Exiled), "instanceId"),
            vec![token.map_or(Value::Null, |token| json!(token.id))]
        );
    }
}
