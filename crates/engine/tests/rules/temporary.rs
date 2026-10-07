//! Temporary (SPEC §6.1, R637): a card that is discarded from its owner's hand at the end of their turn.
//! The card keyword, not temporary mana (§2.3).
//!
//! Port of `packages/engine/test/temporary.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, temporary_body};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, set_library};

static NONCE: AtomicU32 = AtomicU32::new(0);

fn attempt(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    reduce(state, &input.with_nonce(format!("tp{nonce}")))
}

fn act(state: &GameState, body: Value) -> (GameState, Vec<GameEvent>) {
    let result = attempt(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    (result.state, result.events)
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// The card as it stands in the state now (TS held the live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().expect("the card is in the state")
}

/// `eventsOfType(events, kind).map((event) => event[field])`, read through each event's JSON.
fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Value {
    Value::Array(
        events_of_type(events, kind)
            .into_iter()
            .map(|event| serde_json::to_value(event).expect("an event serialises")[field].clone())
            .collect(),
    )
}

/// Past the mulligans, in the main phase of turn 1.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" })).0;
    let keep = ids(&state.players.p2.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" })).0;
    state
}

mod r637_temporary {
    use super::*;

    #[test]
    fn r637_each_temporary_card_in_the_ending_players_hand_is_discarded_at_the_end_of_their_turn_in_hand_order() {
        let mut state = playing("temporary-end");
        let temporaries = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 2);
        let (first, second) = (temporaries[0].clone(), temporaries[1].clone());
        let kept = in_hand(&mut state, &plain.id, PlayerId::P1, 1)[0].clone();
        let hand_before = state.players.p1.hand.len();

        let (after, events) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        let graveyard = ids(&after.players.p1.graveyard);
        assert!(graveyard.contains(&first.id));
        assert!(graveyard.contains(&second.id));
        assert!(ids(&after.players.p1.hand).contains(&kept.id));
        assert_eq!(after.players.p1.hand.len(), hand_before - 2);
        // A discard (§6.3): one `discarded` event each, in hand order, so "whenever you discard" sees them.
        assert_eq!(
            field_of(&events, GameEventType::Discarded, "instanceId"),
            json!([first.id, second.id])
        );
    }

    #[test]
    fn r637_it_waits_for_its_owners_own_turn_end_a_temporary_card_in_the_other_players_hand_stays() {
        let mut state = playing("temporary-other-hand");
        let theirs = in_hand(&mut state, &temporary_body.id, PlayerId::P2, 1)[0].clone();

        let after_mine = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).0;
        assert!(ids(&after_mine.players.p2.hand).contains(&theirs.id));

        let (after_theirs, events) = act(&after_mine, json!({ "type": "endTurn", "playerId": "p2" }));
        assert!(ids(&after_theirs.players.p2.graveyard).contains(&theirs.id));
        assert_eq!(field_of(&events, GameEventType::Discarded, "instanceId"), json!([theirs.id]));
    }

    #[test]
    fn r637_a_temporary_card_played_before_the_end_of_the_turn_escapes_and_it_does_nothing_on_the_field() {
        let mut state = playing("temporary-played");
        let card = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 1)[0].clone();

        let mut next = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
        )
        .0;
        let top = |state: &GameState| {
            state.players.p1.units[0].as_ref().and_then(|pile| pile.first()).map(|unit| unit.id.clone())
        };
        assert_eq!(top(&next), Some(card.id.clone()));
        next = act(&next, json!({ "type": "endTurn", "playerId": "p1" })).0;
        next = act(&next, json!({ "type": "endTurn", "playerId": "p2" })).0;
        assert_eq!(top(&next), Some(card.id.clone()));
        assert!(!ids(&next.players.p1.graveyard).contains(&card.id));
    }

    #[test]
    fn r637_it_does_nothing_in_a_deck_a_temporary_card_is_drawn_like_any_other() {
        let mut state = playing("temporary-deck");
        let deck_card = set_library(&mut state, PlayerId::P1, &[temporary_body.id.clone(), plain.id.clone()])[0].clone();
        assert!(is_temporary_card(&state, &live(&state, &deck_card.id)));
        let after = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).0;
        assert!(ids(&after.players.p1.library).contains(&deck_card.id));
        assert!(!ids(&after.players.p1.graveyard).contains(&deck_card.id));
    }

    #[test]
    fn r637_a_granted_temporary_counts_and_a_vanilla_card_loses_a_printed_one_but_keeps_a_given_one_10_4() {
        let mut state = playing("temporary-granted");
        let granted = in_hand(&mut state, &plain.id, PlayerId::P1, 1)[0].clone();
        let printed_vanilla = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 1)[0].clone();
        let given_vanilla = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 1)[0].clone();
        assert!(!is_temporary_card(&state, &live(&state, &granted.id)));
        find_instance_mut(&mut state, &granted.id)
            .expect("granted")
            .granted_keywords
            .push(Keyword::Temporary);
        find_instance_mut(&mut state, &printed_vanilla.id).expect("printed").vanilla = true;
        {
            let given = find_instance_mut(&mut state, &given_vanilla.id).expect("given");
            given.vanilla = true;
            given.granted_keywords.push(Keyword::Temporary);
        }

        assert!(is_temporary_card(&state, &live(&state, &granted.id)));
        assert!(!is_temporary_card(&state, &live(&state, &printed_vanilla.id)));
        assert!(is_temporary_card(&state, &live(&state, &given_vanilla.id)));

        let after = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).0;
        let in_graveyard = ids(&after.players.p1.graveyard);
        assert!(in_graveyard.contains(&granted.id));
        assert!(in_graveyard.contains(&given_vanilla.id));
        assert!(!in_graveyard.contains(&printed_vanilla.id));
        assert!(ids(&after.players.p1.hand).contains(&printed_vanilla.id));
    }
}
