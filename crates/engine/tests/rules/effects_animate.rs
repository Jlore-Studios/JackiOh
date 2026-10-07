//! The `animate` verb (docs/classic-sets.md B3.1 rules 2 and 4, R383, R445): what an Animated trap's list
//! ends with. The whole system — firing, turn start, cleanup, home zones — is `animated.test.ts`; this is
//! the verb's own surface: its target, its position, and every card it leaves alone.
//!
//! Port of `packages/engine/test/effects-animate.test.ts`.

use jackioh_engine::effects::animate;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::field::{banner, cover, golem, playing, springer, tower};
use crate::rules::fixtures::harness::{events_of_type, put, slot};

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. Every context
/// `f` builds on it shares its events, which come back.
fn sink_run(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>)) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink);
    events
}

fn as_player(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, at).map(|card| card.id.clone())
}

mod animate_b3_1_r383 {
    use super::*;

    #[test]
    fn r383_animates_the_card_running_the_script_into_its_lane_s_unit_zone_in_attack_position_by_default() {
        let mut state = playing("animate-self");
        let card = put(
            &mut state,
            &springer.id,
            slot(PlayerId::P2, Row::Backrow, 3),
            json!({}),
        );
        let events = sink_run(&mut state, |sink| {
            let mut ctx = make_context(sink, Some(&card), as_player(PlayerId::P2));
            (animate(Default::default()).apply)(&mut ctx);
        });
        assert_eq!(
            id_at(&state, slot(PlayerId::P2, Row::Units, 3)),
            Some(card.id.clone())
        );
        assert_eq!(live(&state, &card.id).position, Some(Position::Atk));
        assert_eq!(events_of_type(&events, GameEventType::Animated).len(), 1);
        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
    }

    #[test]
    fn r383_animates_a_named_card_in_the_position_the_text_gives_else_the_leftmost_open_unit_zone() {
        let mut state = playing("animate-named");
        let card = put(
            &mut state,
            &golem.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let events = sink_run(&mut state, |sink| {
            let mut ctx = make_context(sink, None, as_player(PlayerId::P1));
            (animate(json_as(
                json!({ "target": { "of": "instance", "instanceId": card.id }, "position": "DEF" }),
            ))
            .apply)(&mut ctx);
        });
        assert_eq!(
            id_at(&state, slot(PlayerId::P1, Row::Units, 1)),
            Some(card.id.clone())
        );
        assert_eq!(live(&state, &card.id).position, Some(Position::Def));
        let animated: Vec<(i32, i32)> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Animated {
                    backrow_lane,
                    unit_lane,
                    ..
                } => Some((*backrow_lane, *unit_lane)),
                _ => None,
            })
            .collect();
        assert_eq!(animated, vec![(2, 1)]);
    }

    #[test]
    fn r383_leaves_alone_a_card_in_a_hand_one_dormant_under_a_backrow_pile_and_a_unit_a_carrier_holds() {
        let mut state = playing("animate-refusals");
        let held = new_instance(
            &mut state,
            &golem.id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        state.players[PlayerId::P1].hand.push(held.clone());
        let buried = put(
            &mut state,
            &golem.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let mut top = new_instance(
            &mut state,
            &cover.id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P1, Row::Backrow, 1),
            json_as(json!({ "stack": true })),
        );
        put(
            &mut state,
            &tower.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let mut rider = new_instance(
            &mut state,
            &plain.id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        place_on_field(
            &mut state,
            &mut rider,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        let events = sink_run(&mut state, |sink| {
            for card in [&held, &buried, &rider] {
                let mut ctx = make_context(sink, None, as_player(PlayerId::P1));
                (animate(json_as(
                    json!({ "target": { "of": "instance", "instanceId": card.id } }),
                ))
                .apply)(&mut ctx);
            }
        });
        assert_eq!(events, Vec::<GameEvent>::new());
        assert_eq!(live(&state, &held.id).zone.z(), ZoneName::Hand);
        assert_eq!(
            id_at(&state, slot(PlayerId::P1, Row::Backrow, 1)),
            Some(top.id.clone())
        );
    }

    #[test]
    fn r383_a_card_that_is_a_unit_already_does_not_move_or_change_position() {
        let mut state = playing("animate-already");
        let card = put(
            &mut state,
            &springer.id,
            slot(PlayerId::P2, Row::Backrow, 4),
            json!({}),
        );
        let events = sink_run(&mut state, |sink| {
            let mut ctx = make_context(sink, Some(&card), as_player(PlayerId::P2));
            (animate(json_as(json!({ "position": "DEF" }))).apply)(&mut ctx);
            // TS passed the same live object again; here the card is read back as it now stands.
            let now = find_instance(sink.state, &card.id).cloned();
            let mut ctx = make_context(sink, now.as_ref(), as_player(PlayerId::P2));
            (animate(json_as(json!({ "position": "ATK" }))).apply)(&mut ctx);
        });
        assert_eq!(
            id_at(&state, slot(PlayerId::P2, Row::Units, 4)),
            Some(card.id.clone())
        );
        assert_eq!(live(&state, &card.id).position, Some(Position::Def));
        assert_eq!(events_of_type(&events, GameEventType::Animated).len(), 1);
    }

    #[test]
    fn r383_with_every_unit_zone_taken_the_card_stays_in_the_backrow_and_nothing_is_reported() {
        let mut state = playing("animate-full");
        for lane in 1..=5 {
            put(
                &mut state,
                &plain.id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let card = put(
            &mut state,
            &banner.id,
            slot(PlayerId::P1, Row::Backrow, 5),
            json!({}),
        );
        let events = sink_run(&mut state, |sink| {
            let mut ctx = make_context(sink, Some(&card), as_player(PlayerId::P1));
            (animate(Default::default()).apply)(&mut ctx);
        });
        assert_eq!(
            id_at(&state, slot(PlayerId::P1, Row::Backrow, 5)),
            Some(card.id.clone())
        );
        assert_eq!(events, Vec::<GameEvent>::new());
    }
}
