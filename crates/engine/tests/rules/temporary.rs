//! Temporary (SPEC §6.1, R637): a card that is discarded from its owner's hand at the end of their turn.
//! The card keyword, not temporary mana (§2.3).

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, temporary_body};
use crate::rules::fixtures::harness::{act, act_ok, events_of_type, in_hand, live, new_game, set_library};

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Value {
    Value::Array(
        events_of_type(events, kind)
            .into_iter()
            .map(|event| serde_json::to_value(event).expect("an event serializes")[field].clone())
            .collect(),
    )
}

/// Past the mulligans, in the main phase of turn 1.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    );
    let keep = ids(&state.players.p2.hand);
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    );
    state
}

mod r637_temporary {
    use super::*;

    #[test]
    fn r637_each_temporary_card_in_the_ending_players_hand_is_discarded_at_the_end_of_their_turn_in_hand_order()
     {
        let mut state = playing("temporary-end");
        let temporaries = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 2);
        let (first, second) = (temporaries[0].clone(), temporaries[1].clone());
        let kept = in_hand(&mut state, &plain.id, PlayerId::P1, 1)[0].clone();
        let hand_before = state.players.p1.hand.len();

        let ReduceResult {
            state: after, events, ..
        } = act_ok(&state, json!({ "type": "endTurn", "playerId": "p1" }));

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

        let after_mine = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert!(ids(&after_mine.players.p2.hand).contains(&theirs.id));

        let ReduceResult {
            state: after_theirs,
            events,
            ..
        } = act_ok(&after_mine, json!({ "type": "endTurn", "playerId": "p2" }));
        assert!(ids(&after_theirs.players.p2.graveyard).contains(&theirs.id));
        assert_eq!(
            field_of(&events, GameEventType::Discarded, "instanceId"),
            json!([theirs.id])
        );
    }

    #[test]
    fn r637_a_temporary_card_played_before_the_end_of_the_turn_escapes_and_it_does_nothing_on_the_field() {
        let mut state = playing("temporary-played");
        let card = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 1)[0].clone();

        let mut next = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
        );
        let top = |state: &GameState| {
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .map(|unit| unit.id.clone())
        };
        assert_eq!(top(&next), Some(card.id.clone()));
        next = act(&next, json!({ "type": "endTurn", "playerId": "p1" }));
        next = act(&next, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(top(&next), Some(card.id.clone()));
        assert!(!ids(&next.players.p1.graveyard).contains(&card.id));
    }

    #[test]
    fn r637_it_does_nothing_in_a_deck_a_temporary_card_is_drawn_like_any_other() {
        let mut state = playing("temporary-deck");
        let deck_card = set_library(
            &mut state,
            PlayerId::P1,
            &[temporary_body.id.clone(), plain.id.clone()],
        )[0]
        .clone();
        assert!(is_temporary_card(&state, &live(&state, &deck_card.id)));
        let after = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
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
            .push(Keyword::TEMPORARY);
        find_instance_mut(&mut state, &printed_vanilla.id)
            .expect("printed")
            .vanilla = true;
        {
            let given = find_instance_mut(&mut state, &given_vanilla.id).expect("given");
            given.vanilla = true;
            given.granted_keywords.push(Keyword::TEMPORARY);
        }

        assert!(is_temporary_card(&state, &live(&state, &granted.id)));
        assert!(!is_temporary_card(&state, &live(&state, &printed_vanilla.id)));
        assert!(is_temporary_card(&state, &live(&state, &given_vanilla.id)));

        let after = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        let in_graveyard = ids(&after.players.p1.graveyard);
        assert!(in_graveyard.contains(&granted.id));
        assert!(in_graveyard.contains(&given_vanilla.id));
        assert!(!in_graveyard.contains(&printed_vanilla.id));
        assert!(ids(&after.players.p1.hand).contains(&printed_vanilla.id));
    }
}

/// R2179001: Temporary X counts down at each end of its owner's turn in their hand, and the card is
/// discarded at 0; a plain Temporary is Temporary 1.
mod r2179001_temporary_x {
    use super::*;

    fn give(state: &mut GameState, id: &str, keyword: Keyword) {
        find_instance_mut(state, id)
            .expect("the card")
            .granted_keywords
            .push(keyword);
    }

    fn temporary_of(state: &GameState, id: &str) -> Vec<Keyword> {
        card_keywords(state, &live(state, id))
            .into_iter()
            .filter(|keyword| keyword.kind() == KeywordKind::Temporary)
            .collect()
    }

    #[test]
    fn r2179001_temporary_2_outlasts_its_owners_first_end_of_turn_at_1_and_is_discarded_at_the_next() {
        let mut state = playing("temporary-2");
        let card = in_hand(&mut state, &plain.id, PlayerId::P1, 1)[0].clone();
        give(&mut state, &card.id, Keyword::Temporary { n: Some(2) });
        assert_eq!(
            temporary_of(&state, &card.id),
            vec![Keyword::Temporary { n: Some(2) }]
        );

        let after_mine = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert!(ids(&after_mine.players.p1.hand).contains(&card.id));
        assert_eq!(live(&after_mine, &card.id).temporary, Some(1));
        assert_eq!(
            temporary_of(&after_mine, &card.id),
            vec![Keyword::Temporary { n: Some(1) }]
        );

        // The opponent's end of turn counts nothing down.
        let after_theirs = act(&after_mine, json!({ "type": "endTurn", "playerId": "p2" }));
        assert!(ids(&after_theirs.players.p1.hand).contains(&card.id));
        assert_eq!(live(&after_theirs, &card.id).temporary, Some(1));

        let ReduceResult {
            state: after_next,
            events,
            ..
        } = act_ok(&after_theirs, json!({ "type": "endTurn", "playerId": "p1" }));
        assert!(ids(&after_next.players.p1.graveyard).contains(&card.id));
        assert_eq!(
            field_of(&events, GameEventType::Discarded, "instanceId"),
            json!([card.id])
        );
    }

    #[test]
    fn r2179001_a_plain_temporary_is_temporary_1_written_without_a_number() {
        assert_eq!(
            serde_json::to_value(Keyword::TEMPORARY).expect("json"),
            json!({ "kind": "Temporary" })
        );
        assert_eq!(
            serde_json::from_value::<Keyword>(json!({ "kind": "Temporary" })).expect("a keyword"),
            Keyword::TEMPORARY
        );
        assert_eq!(
            serde_json::to_value(Keyword::Temporary { n: Some(2) }).expect("json"),
            json!({ "kind": "Temporary", "n": 2 })
        );
        assert_eq!(keyword_key(&Keyword::TEMPORARY), "Temporary");
        assert_eq!(keyword_key(&Keyword::Temporary { n: Some(2) }), "Temporary 2");
        assert_eq!(temporary_number(&[Keyword::TEMPORARY]), Some(1));

        // Printed plain, it goes at its owner's first end of turn and stores no count.
        let mut state = playing("temporary-1");
        let card = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 1)[0].clone();
        let after = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        let gone = after
            .players
            .p1
            .graveyard
            .iter()
            .find(|held| held.id == card.id)
            .expect("discarded");
        assert_eq!(gone.temporary, None);
    }

    #[test]
    fn r2179001_one_temporary_is_kept_the_largest_number_and_the_count_left_stands_in_for_it() {
        let mut state = playing("temporary-merge");
        let card = in_hand(&mut state, &temporary_body.id, PlayerId::P1, 1)[0].clone();
        give(&mut state, &card.id, Keyword::Temporary { n: Some(3) });
        assert_eq!(
            temporary_of(&state, &card.id),
            vec![Keyword::Temporary { n: Some(3) }]
        );

        let once = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(
            temporary_of(&once, &card.id),
            vec![Keyword::Temporary { n: Some(2) }]
        );
        let twice = act(
            &act(&once, json!({ "type": "endTurn", "playerId": "p2" })),
            json!({ "type": "endTurn", "playerId": "p1" }),
        );
        assert_eq!(live(&twice, &card.id).temporary, Some(1));
        assert!(ids(&twice.players.p1.hand).contains(&card.id));
    }
}
