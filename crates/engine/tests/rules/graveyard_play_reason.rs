//! R1571000 (issue #1571): a card in the viewer's own graveyard that a permission admits but
//! no play of which is listed says why (`CardView.play_reason`, §10.8), as a hand card does
//! (R979000). The reason is R979000's first refusal in a fixed order, with the price step asked
//! of the graveyard permissions (`graveyard_play::why_graveyard_price_unpaid`). The client reads
//! the field and works nothing out (CLAUDE.md rule 7).
//!
//! Fixtures reuse the play pipeline B (docs/classic-sets.md B5): Second Wind (any card), Radiant
//! Second Wind (a price of (1) or more), Corpse Plantation (Units, paid with its Plague Counters).

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    five_unit, grave_spell, grave_unit, in_graveyard, pb_playing, pb_reduce, plantation, plays_of,
    second_wind, three_spell, zero_spell,
};

/// What `viewer` reads as this graveyard card's `playReason`.
fn reason_of(state: &GameState, viewer: PlayerId, instance_id: &str) -> Option<String> {
    view_for(state, viewer)
        .you
        .graveyard
        .iter()
        .find(|card| card.instance_id == instance_id)
        .unwrap_or_else(|| panic!("{viewer:?} holds no graveyard card {instance_id}"))
        .play_reason
        .clone()
}

/// A refusal's text (`""` when the action was not refused).
fn refusal_of(state: &GameState, body: Value) -> String {
    pb_reduce(state, body).error.unwrap_or_default()
}

fn has_no_plays(state: &GameState, instance_id: &str) -> bool {
    plays_of(state, instance_id, PlayerId::P1).is_empty()
}

mod r1571000_an_unplayable_graveyard_card_says_why {
    use super::*;

    #[test]
    fn r1571000_unaffordable_graveyard_card_says_its_price_as_the_reducer_does() {
        let mut state = pb_playing("r1571000-price");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &three_spell().id, PlayerId::P1);
        state.players.p1.mana.current = 2;

        assert!(has_no_plays(&state, &spell.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &spell.id),
            Some("that card costs 3, more than your mana".to_string())
        );
        assert!(
            refusal_of(
                &state,
                json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })
            )
            .contains("that card costs 3, more than your mana")
        );
    }

    #[test]
    fn r1571000_a_price_under_the_permissions_floor_says_the_floor() {
        let mut state = pb_playing("r1571000-floor");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json_as(json!({ "radiant": true })),
        );
        let free = in_graveyard(&mut state, &zero_spell().id, PlayerId::P1);

        assert!(has_no_plays(&state, &free.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &free.id),
            Some("a card played from your graveyard must cost (1) or more".to_string())
        );
        assert!(
            refusal_of(
                &state,
                json!({ "type": "play", "instanceId": free.id, "playerId": "p1" })
            )
            .contains("must cost (1) or more")
        );
    }

    #[test]
    fn r1571000_a_plague_only_permission_says_why_its_counters_cannot_pay() {
        let mut state = pb_playing("r1571000-plague-bare");
        put(
            &mut state,
            &plantation().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        let body = in_graveyard(&mut state, &grave_unit().id, PlayerId::P1);

        assert!(has_no_plays(&state, &body.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &body.id),
            Some("there are not that many Plague Counters there".to_string())
        );

        let mut state = pb_playing("r1571000-plague-short");
        let field = put(
            &mut state,
            &plantation().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        find_instance_mut(&mut state, &field.id)
            .expect("the Plantation stands")
            .counters
            .plague = Some(2);
        let big = in_graveyard(&mut state, &five_unit().id, PlayerId::P1);
        state.players.p1.mana.current = 1;

        assert!(has_no_plays(&state, &big.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &big.id),
            Some("the rest of the price is more than your mana".to_string())
        );
        assert_eq!(
            refusal_of(
                &state,
                json!({ "type": "play", "instanceId": big.id, "plague": { "from": field.id, "tokens": 2 }, "playerId": "p1" })
            ),
            "the rest of the price is more than your mana"
        );
    }

    #[test]
    fn r1571000_full_row_says_no_free_zone() {
        let mut state = pb_playing("r1571000-zone");
        for lane in 1..=5 {
            put(
                &mut state,
                &grave_unit().id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let sixth = in_graveyard(&mut state, &grave_unit().id, PlayerId::P1);

        assert!(has_no_plays(&state, &sixth.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &sixth.id),
            Some("no free units zone".to_string())
        );
    }

    #[test]
    fn r1571000_the_price_is_said_before_the_zone() {
        let mut state = pb_playing("r1571000-order");
        for lane in 1..=5 {
            put(
                &mut state,
                &grave_unit().id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let sixth = in_graveyard(&mut state, &grave_unit().id, PlayerId::P1);
        state.players.p1.mana.current = 1;

        assert!(has_no_plays(&state, &sixth.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &sixth.id),
            Some("that card costs 2, more than your mana".to_string())
        );
    }

    #[test]
    fn r1571000_no_reason_when_a_play_is_listed_no_permission_admits_it_or_the_viewer_may_not_act() {
        // A play is listed: the card says nothing.
        let mut state = pb_playing("r1571000-listed");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        assert!(!plays_of(&state, &spell.id, PlayerId::P1).is_empty());
        assert_eq!(reason_of(&state, PlayerId::P1, &spell.id), None);

        // No permission admits the card: there is no play to explain.
        let mut state = pb_playing("r1571000-bare");
        let bare = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(reason_of(&state, PlayerId::P1, &bare.id), None);

        // The permission admits Units only: a Spell says nothing.
        let mut state = pb_playing("r1571000-units-only");
        put(
            &mut state,
            &plantation().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        assert_eq!(reason_of(&state, PlayerId::P1, &spell.id), None);

        // The viewer may not act on anyone else's turn.
        let mut state = pb_playing("r1571000-inactive");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &three_spell().id, PlayerId::P1);
        state.players.p1.mana.current = 2;
        assert!(has_no_plays(&state, &spell.id));
        state.active = PlayerId::P2;
        assert_eq!(reason_of(&state, PlayerId::P1, &spell.id), None);

        // Nor after the game.
        state.active = PlayerId::P1;
        state.result = Some(GameResult {
            winner: Winner::P1,
            reason: GameOverReason::HeroDeath,
        });
        assert!(
            view_for(&state, PlayerId::P1)
                .you
                .graveyard
                .iter()
                .all(|card| card.play_reason.is_none()),
            "a post-game graveyard carries no playReason"
        );
    }

    #[test]
    fn r1571000_no_other_seat_reads_a_graveyard_reason() {
        let mut state = pb_playing("r1571000-seats");
        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let spell = in_graveyard(&mut state, &three_spell().id, PlayerId::P1);
        state.players.p1.mana.current = 2;
        assert!(has_no_plays(&state, &spell.id));
        assert!(reason_of(&state, PlayerId::P1, &spell.id).is_some());

        put(
            &mut state,
            &second_wind().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        in_graveyard(&mut state, &three_spell().id, PlayerId::P2);

        // P2 may not act (P1's turn), and the opponent's piles carry no key either way.
        assert!(
            view_for(&state, PlayerId::P2)
                .you
                .graveyard
                .iter()
                .all(|card| card.play_reason.is_none()),
            "p2's own graveyard carries no playReason on p1's turn"
        );
        assert!(
            view_for(&state, PlayerId::P2)
                .opponent
                .graveyard
                .iter()
                .all(|card| card.play_reason.is_none()),
            "p1's graveyard carries no playReason for p2"
        );
        assert!(
            view_for(&state, PlayerId::P1)
                .opponent
                .graveyard
                .iter()
                .all(|card| card.play_reason.is_none()),
            "p2's graveyard carries no playReason for p1"
        );
    }
}
