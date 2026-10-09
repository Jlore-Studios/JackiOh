//! The Untributable keyword (R1220, Meditative #80 Aluneth): no Tribute cost may take the card,
//! and every Sacrifice of it does nothing. It stays out of the random keyword pool, and a Degrade
//! never removes it.

use jackioh_engine::config::{RANDOM_KEYWORD_POOL, TUNE_HARMFUL_KEYWORDS};
use jackioh_engine::effects::{TuneDirection, TuneRow, applicable_changes};
use jackioh_engine::subsystems::activate::why_cannot_activate_ability;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::credit::{
    act, body, in_hand, live, playing, put, safe, safe_field, slot, tribute_one,
};
use crate::rules::fixtures::harness::sink_for;

mod r1220_untributable {
    use super::*;

    #[test]
    fn r1220_a_play_tribute_never_offers_or_takes_it() {
        let mut state = playing("untributable-play");
        let keeper = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            Value::Null,
        );
        let safe = put(
            &mut state,
            &safe().id,
            slot(PlayerId::P1, Row::Units, 2),
            Value::Null,
        );
        let dealt = in_hand(&mut state, &tribute_one().id, PlayerId::P1, 1);
        let card = live(&state, &only(&dealt));

        // The Untributable unit is never one of the Tribute's legal units, so no offered set holds it.
        let units = legal_tribute_units(&state, PlayerId::P1, &card);
        assert!(units.iter().any(|unit| unit.id == keeper.id));
        assert!(units.iter().all(|unit| unit.id != safe.id));
        assert!(
            legal_tribute_sets(&state, PlayerId::P1, &card)
                .iter()
                .all(|set| !set.contains(&safe.id))
        );

        // Naming it is refused as a unit the play cannot take.
        let play: PlayAction = json_as(json!({
            "type": "play",
            "instanceId": card.id,
            "zone": { "row": "units", "lane": 3 },
            "tributes": [safe.id],
        }));
        let refusal = why_choices_refused(&state, PlayerId::P1, &card, &play)
            .err()
            .map(|error| error.message);
        assert!(
            refusal.as_deref().is_some_and(|message| message.contains("cannot be tributed")),
            "unexpected refusal: {refusal:?}",
        );

        // Paying with the other unit works, and the safe one stands after.
        let after = act(
            &state,
            json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 3 },
                "tributes": [keeper.id],
            }),
        );
        assert!(on_field(&after, &safe.id));
        assert!(!on_field(&after, &keeper.id));
    }

    #[test]
    fn r1220_a_sacrifice_of_it_does_nothing() {
        let mut state = playing("untributable-sacrifice");
        let safe = put(
            &mut state,
            &safe().id,
            slot(PlayerId::P1, Row::Units, 1),
            Value::Null,
        );
        let other = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 2),
            Value::Null,
        );

        // A Sacrifice of the Untributable card alone does nothing.
        let target = live(&state, &safe);
        {
            let mut sink = sink_for(&mut state);
            sacrifice_now(&mut sink, &target);
        }
        assert!(on_field(&state, &safe.id));

        // In a Sacrifice together with another unit, only the other dies.
        let pair = vec![live(&state, &safe), live(&state, &other)];
        {
            let mut sink = sink_for(&mut state);
            sacrifice_together(&mut sink, &pair);
        }
        assert!(on_field(&state, &safe.id));
        assert!(!on_field(&state, &other.id));
        assert!(
            state.players.p1.graveyard.iter().any(|card| card.id == other.id),
            "the tributed body reaches the graveyard",
        );
    }

    #[test]
    fn r1220_its_tribute_when_never_fires() {
        let mut state = playing("untributable-when");
        let safe = put(
            &mut state,
            &safe_field().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Value::Null,
        );
        // Its `tributeWhen` always holds; the state check still leaves it standing.
        {
            let mut sink = sink_for(&mut state);
            state_check(&mut sink);
        }
        let card = find_instance(&state, &safe.id).expect("the field spell stands");
        assert!(matches!(card.zone, Zone::Field { .. }));
    }

    #[test]
    fn r1220_tribute_this_is_unusable() {
        let mut state = playing("untributable-self");
        let safe = put(
            &mut state,
            &safe().id,
            slot(PlayerId::P1, Row::Units, 1),
            Value::Null,
        );
        let refusal = why_cannot_activate_ability(&state, PlayerId::P1, &safe.id, Some("tribute"))
            .err()
            .map(|error| error.message);
        assert_eq!(refusal.as_deref(), Some("that card can't be Tributed"));
    }

    #[test]
    fn r1220_not_in_the_pool_and_never_degraded_away() {
        // It stays out of the random keyword pool (R21).
        assert!(!RANDOM_KEYWORD_POOL.contains(&"Untributable"));
        // And a Degrade never removes it: losing it would help the card.
        assert!(TUNE_HARMFUL_KEYWORDS.contains(&KeywordKind::Untributable));

        let mut state = playing("untributable-tune");
        let safe = put(
            &mut state,
            &safe().id,
            slot(PlayerId::P1, Row::Units, 1),
            Value::Null,
        );
        let rows = applicable_changes(&state, &live(&state, &safe), TuneDirection::Degrade);
        assert!(
            rows.iter().all(|row| *row != TuneRow::Keyword),
            "no keyword row is offered: {rows:?}",
        );
    }

    fn on_field(state: &GameState, id: &str) -> bool {
        find_instance(state, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
    }

    fn only<T: Clone>(items: &[T]) -> T {
        items.first().cloned().expect("expected at least one item")
    }
}
