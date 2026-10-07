//! §6.3 Shuffle of an existing card into its owner's library — `shuffleCardInto`
//! (effects/shuffleCard.ts), the verb Classic #30 Recycle needs ("Shuffle your graveyard into your
//! deck"). SPEC §6.3, R80, R97, R311, R316. The real card's test (packages/cards/test/classic/
//! 030-recycle.test.ts) covers the same cases again through the card.
//!
//! Port of `packages/engine/test/effects-shuffle-card.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::config::LIBRARY_CAP;
use jackioh_engine::effects::shuffle_card_into;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{EngineSink, Effect};
use jackioh_engine::state::{CardInstance, GameState, find_instance_mut, new_instance};

use super::fixtures::harness::{new_game, set_library};

fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
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

fn in_graveyard(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Graveyard { player });
    state.players[player].graveyard.push(card.clone());
    card
}

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

fn shuffle_in(id: &str) -> Effect {
    shuffle_card_into(json_as(json!({ "instanceId": id })))
}

fn full_library() -> Vec<&'static str> {
    vec!["fx-1"; LIBRARY_CAP as usize]
}

mod s6_3_shuffle_card_into_c_c30_recycle {
    use super::*;

    #[test]
    fn r78_moves_a_graveyard_card_into_its_owners_library_keeping_its_id_and_its_cost_mod() {
        let mut state = new_game("shuffle-card-one", None);
        set_library(&mut state, PlayerId::P1, &["fx-1", "fx-2", "fx-3"]);
        let card = in_graveyard(&mut state, "fx-4", PlayerId::P1);
        find_instance_mut(&mut state, &card.id).expect("the graveyard card").cost_mod = -1;

        let events = run(&mut state, shuffle_in(&card.id), PlayerId::P1);

        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(state.players.p1.library.len(), 4);
        let moved = state.players.p1.library.iter().find(|c| c.id == card.id);
        assert_eq!(moved.map(|c| c.zone.clone()), Some(Zone::Library { player: PlayerId::P1 }));
        assert_eq!(moved.map(|c| c.cost_mod), Some(-1));
        assert_eq!(
            of_type(&events, "shuffledIn")
                .iter()
                .map(|event| event["instanceId"].clone())
                .collect::<Vec<_>>(),
            vec![json!(card.id)]
        );
    }

    #[test]
    fn goes_to_its_owners_library_whoever_runs_the_effect() {
        let mut state = new_game("shuffle-card-owner", None);
        set_library(&mut state, PlayerId::P2, &["fx-1"]);
        let card = in_graveyard(&mut state, "fx-4", PlayerId::P2);

        run(&mut state, shuffle_in(&card.id), PlayerId::P1);

        assert!(state.players.p2.library.iter().any(|c| c.id == card.id));
    }

    #[test]
    fn r60_the_position_is_the_match_rngs_the_same_seed_puts_it_in_the_same_place() {
        let at = |seed: &str| -> Option<usize> {
            let mut state = new_game(seed, None);
            set_library(&mut state, PlayerId::P1, &["fx-1", "fx-2", "fx-3", "fx-5", "fx-6"]);
            let card = in_graveyard(&mut state, "fx-4", PlayerId::P1);
            run(&mut state, shuffle_in(&card.id), PlayerId::P1);
            state.players.p1.library.iter().position(|c| c.id == card.id)
        };
        assert_eq!(at("shuffle-card-seed"), at("shuffle-card-seed"));
    }

    #[test]
    fn r80_r316_a_full_library_turns_it_away_it_stays_in_the_graveyard_where_it_was_with_no_second_move() {
        let mut state = new_game("shuffle-card-full", None);
        set_library(&mut state, PlayerId::P1, &full_library());
        let first = in_graveyard(&mut state, "fx-4", PlayerId::P1);
        let second = in_graveyard(&mut state, "fx-5", PlayerId::P1);

        let events = run(&mut state, shuffle_in(&first.id), PlayerId::P1);

        assert_eq!(state.players.p1.library.len(), LIBRARY_CAP as usize);
        assert_eq!(
            state.players.p1.graveyard.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
            vec![first.id.clone(), second.id.clone()]
        );
        assert_eq!(of_type(&events, "enteredGraveyard"), Vec::<Value>::new());
        assert_eq!(of_type(&events, "shuffledIn"), Vec::<Value>::new());
        assert_eq!(
            of_type(&events, "libraryOverflow"),
            vec![json!({
                "type": "libraryOverflow", "player": "p1", "instanceId": first.id, "defId": "fx-4",
                "outcome": "graveyard"
            })]
        );
    }

    #[test]
    fn r316_a_radiant_card_turned_away_is_reported_radiant() {
        let mut state = new_game("shuffle-card-full-radiant", None);
        set_library(&mut state, PlayerId::P1, &full_library());
        let card = in_graveyard(&mut state, "fx-4", PlayerId::P1);
        find_instance_mut(&mut state, &card.id).expect("the graveyard card").radiant = true;

        let events = run(&mut state, shuffle_in(&card.id), PlayerId::P1);

        assert_eq!(of_type(&events, "libraryOverflow")[0]["radiant"], json!(true));
    }

    #[test]
    fn a_card_that_no_longer_exists_is_skipped_nothing_moves_and_the_rng_is_not_drawn_from() {
        let mut state = new_game("shuffle-card-gone", None);
        set_library(&mut state, PlayerId::P1, &["fx-1"]);
        let cursor = state.rng_cursor;

        let events = run(&mut state, shuffle_in("no-such-card"), PlayerId::P1);

        assert_eq!(events, Vec::<GameEvent>::new());
        assert_eq!(state.players.p1.library.len(), 1);
        assert_eq!(state.rng_cursor, cursor);
    }
}
