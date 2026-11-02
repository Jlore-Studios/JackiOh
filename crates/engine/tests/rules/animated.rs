//! Animated (docs/classic-sets.md B3.1, R383, R445): a Field Spell, Trap or Field Trap that steps into a unit
//! zone as a Unit, its home zone held.

use serde::Serialize;

use jackioh_engine::effects::steal::steal;
use jackioh_engine::effects::targets::cards_in_scope;
use jackioh_engine::effects::{TuneDirection, TuneRow, applicable_changes, grant_keyword};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::field::{
    asker, banner, ears, flush, golem, listener, mote, notes_of, playing, spatula, springer, tesla, tower,
    wisp,
};
use crate::rules::fixtures::harness::{
    act, act_result, events_of_type, in_hand, json_of, live, matches_object, put, sink_for, slot,
};

/// An `ActionInput` from a JSON object (SURFACE §8).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn expect_match(actual: impl Serialize, expected: Value) {
    let actual = json_of(actual);
    assert!(
        matches_object(&actual, &expected),
        "{actual} does not match {expected}"
    );
}

fn instance_ids(events: impl Serialize) -> Vec<String> {
    json_of(events)
        .as_array()
        .map(|list| {
            list.iter()
                .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, at).map(|card| card.id.clone())
}

fn graveyard_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .graveyard
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

fn unit_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    active_units_of(state, player)
        .iter()
        .map(|unit| unit.id.clone())
        .collect()
}

struct Played {
    state: GameState,
    events: Vec<GameEvent>,
    unit: String,
}

/// p1 plays a fresh plain 3/3 from hand into the unit zone of `lane`.
fn play_plain(state: &mut GameState, lane: i32) -> Played {
    let card = in_hand(state, &plain.id, P1, 1)
        .into_iter()
        .next()
        .expect("no card");
    flush(state, P1, 10);
    let result = act_result(
        state,
        input(json!({
            "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": lane }, "playerId": "p1"
        })),
    );
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    Played {
        state: result.state,
        events: result.events,
        unit: card.id,
    }
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn fill_units(state: &mut GameState, player: PlayerId) {
    for lane in 1..=5 {
        put(
            state,
            &plain.id,
            slot(player, Row::Units, lane),
            Default::default(),
        );
    }
}

fn json_round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

mod r383_b3_1_animated_traps {
    use super::*;

    #[test]
    fn r383_an_animated_field_trap_fires_then_animates_into_its_lanes_unit_zone_in_the_position_its_text_names()
     {
        let mut start = playing("animated-tesla");
        let zapper = put(
            &mut start,
            &tesla.id,
            slot(P2, Row::Backrow, 2),
            Default::default(),
        );
        let played = play_plain(&mut start, 1);
        let state = &played.state;
        let tesla2 = by_id(state, &zapper.id);

        // The arrival was zapped for 4 (a 3/3 dies), and then the trap stepped into p2's unit zone 2.
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.id == played.unit)
        );
        assert_eq!(id_at(state, slot(P2, Row::Units, 2)), Some(zapper.id.clone()));
        assert!(card_at(state, slot(P2, Row::Backrow, 2)).is_none());
        assert_eq!(tesla2.position, Some(Position::Def));
        assert_eq!(tesla2.face_up, Some(true));
        assert_eq!(tesla2.summoned_turn, Some(state.turn));
        assert_eq!(
            json_of(events_of_type(&played.events, GameEventType::Animated)),
            json!([{
                "type": "animated", "player": "p2", "instanceId": zapper.id, "defId": tesla.id,
                "backrowLane": 2, "unitLane": 2
            }])
        );
        // Firing ended by animating, so the trap was not consumed into the graveyard.
        assert!(state.players.p2.graveyard.is_empty());
    }

    #[test]
    fn r383_an_animated_field_trap_keeps_firing_from_its_unit_zone_and_a_unit_already_does_not_move_or_change_position()
     {
        let mut state = playing("animated-tesla-turret");
        let zapper = put(
            &mut state,
            &tesla.id,
            slot(P2, Row::Backrow, 2),
            Default::default(),
        );
        state = play_plain(&mut state, 1).state;
        by_id_mut(&mut state, &zapper.id).position = Some(Position::Atk);
        let second = play_plain(&mut state, 3);
        let state = second.state;

        assert_eq!(
            instance_ids(events_of_type(&second.events, GameEventType::TrapFired)),
            vec![zapper.id.clone()]
        );
        assert!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.id == second.unit)
        );
        assert_eq!(id_at(&state, slot(P2, Row::Units, 2)), Some(zapper.id.clone()));
        assert_eq!(by_id(&state, &zapper.id).position, Some(Position::Atk));
        assert!(events_of_type(&second.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r383_an_animated_field_trap_fires_in_r68s_order_among_the_unit_lanes_before_the_backrows_traps() {
        let mut state = playing("animated-order");
        let turret = put(
            &mut state,
            &ears.id,
            slot(P2, Row::Backrow, 4),
            Default::default(),
        );
        let ear = put(
            &mut state,
            &listener.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        let first = play_plain(&mut state, 1);
        let mut state = first.state;
        // Both in the backrow at first: lane 1, then lane 4.
        assert_eq!(
            instance_ids(events_of_type(&first.events, GameEventType::TrapFired)),
            vec![ear.id.clone(), turret.id.clone()]
        );
        assert_eq!(id_at(&state, slot(P2, Row::Units, 4)), Some(turret.id.clone()));
        let fired = play_plain(&mut state, 2);
        // p2 is the opponent: its unit lanes (the animated turret) come before its backrow (the listener).
        assert_eq!(
            instance_ids(events_of_type(&fired.events, GameEventType::TrapFired)),
            vec![turret.id.clone(), ear.id.clone()]
        );
        assert_eq!(
            notes_of(find_instance(&fired.state, &ear.id)),
            vec!["heard", "heard"]
        );
        assert_eq!(
            notes_of(find_instance(&fired.state, &turret.id)),
            vec!["heard", "heard"]
        );
    }

    #[test]
    fn r383_a_plain_animated_trap_animates_on_firing_is_spent_and_never_reaches_the_graveyard() {
        let mut state = playing("animated-springer");
        let trap = put(
            &mut state,
            &springer.id,
            slot(P2, Row::Backrow, 5),
            Default::default(),
        );
        state = play_plain(&mut state, 1).state;
        let card = by_id(&state, &trap.id);
        assert!(is_animated(&state, card));
        assert_eq!(id_at(&state, slot(P2, Row::Units, 5)), Some(trap.id.clone()));
        assert!(is_spent(&state, card));
        // A Trap fires once: the next play finds it spent.
        let again = play_plain(&mut state, 2);
        assert!(events_of_type(&again.events, GameEventType::TrapFired).is_empty());
        assert_eq!(notes_of(find_instance(&again.state, &trap.id)), vec!["sprang"]);
    }

    #[test]
    fn r383_with_no_open_unit_zone_an_animated_trap_stays_where_it_is_face_up_spent_out_of_the_graveyard() {
        let mut state = playing("animated-no-room");
        fill_units(&mut state, P2);
        let trap = put(
            &mut state,
            &springer.id,
            slot(P2, Row::Backrow, 3),
            Default::default(),
        );
        let played = play_plain(&mut state, 1);
        let state = played.state;
        let card = by_id(&state, &trap.id);
        assert_eq!(id_at(&state, slot(P2, Row::Backrow, 3)), Some(trap.id.clone()));
        assert_eq!(card.face_up, Some(true));
        assert!(is_spent(&state, card));
        assert!(state.players.p2.graveyard.is_empty());
        assert!(events_of_type(&played.events, GameEventType::Animated).is_empty());
        // Both players read it now, as a fired Field Trap is read (R33).
        expect_match(
            &view_for(&state, P1).opponent.backrow[2],
            json!({ "faceDown": false, "defId": springer.id }),
        );
    }

    #[test]
    fn r445_animating_is_not_a_summon_it_emits_animated_never_summoned_and_a_tesla_does_not_answer_it() {
        let mut state = playing("animated-not-summon");
        let zapper = put(
            &mut state,
            &tesla.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let trap = put(
            &mut state,
            &springer.id,
            slot(P2, Row::Backrow, 2),
            Default::default(),
        );
        let played = play_plain(&mut state, 3);
        let state = played.state;
        assert!(is_animated(&state, by_id(&state, &trap.id)));
        let summons = instance_ids(events_of_type(&played.events, GameEventType::Summoned));
        assert_eq!(summons, vec![played.unit.clone()]);
        assert!(!summons.contains(&trap.id));
        assert_eq!(
            instance_ids(events_of_type(&played.events, GameEventType::TrapFired)),
            vec![trap.id.clone()]
        );
        assert_eq!(by_id(&state, &zapper.id).face_up, None);
        assert_eq!(id_at(&state, slot(P1, Row::Backrow, 1)), Some(zapper.id.clone()));
    }
}

mod r383_b3_1_an_animated_card_is_a_unit_for_every_rule {
    use super::*;

    #[test]
    fn r383_answers_unit_for_its_type_where_it_stands_joins_the_unit_walks_fights_and_dies_like_a_unit() {
        let mut state = playing("animated-unit-rules");
        let trap = put(
            &mut state,
            &springer.id,
            slot(P2, Row::Backrow, 4),
            Default::default(),
        );
        state = play_plain(&mut state, 1).state;
        let card = live(&state, &trap.id);

        // Its face is still a Trap; where it stands, it is a Unit (B2.7's reader, R383).
        assert_eq!(face_type_of(&state, &card), CardType::Trap);
        assert_eq!(card_type_of(&state, &card), CardType::Unit);
        assert!(unit_ids(&state, P2).contains(&trap.id));
        {
            let mut sink = sink_for(&mut state);
            let ctx = make_context(&mut sink, None, by(P1));
            assert!(ids(&cards_in_scope(&ctx, &json_as(json!({ "side": "enemy" })))).contains(&trap.id));
            assert!(
                !ids(&cards_in_scope(
                    &ctx,
                    &json_as(json!({ "side": "enemy", "rows": ["backrow"] }))
                ))
                .contains(&trap.id)
            );
        }

        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p1" })));
        state = act(&state, input(json!({ "type": "endTurn", "playerId": "p2" })));
        let attacker = card_at(&state, slot(P1, Row::Units, 1)).cloned();
        assert!(attacker.is_some());
        let Some(attacker) = attacker else { return };
        by_id_mut(&mut state, &trap.id).damage = 2;
        assert!(can_attack(
            &state,
            &attacker,
            &AttackTarget::Unit {
                instance: live(&state, &trap.id)
            }
        ));
        state = act(
            &state,
            input(
                json!({ "type": "attack", "attackerId": attacker.id, "targetId": trap.id, "playerId": "p1" }),
            ),
        );
        assert!(graveyard_ids(&state, P2).contains(&trap.id));
        assert!(card_at(&state, slot(P2, Row::Units, 4)).is_none());
    }

    #[test]
    fn r383_entering_the_unit_zone_is_entering_it_on_that_turn_summoning_sick_a_fresh_exertion_and_it_cannot_attack_yet()
     {
        let mut state = playing("animated-sick");
        let card = put(
            &mut state,
            &springer.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        {
            let spent = by_id_mut(&mut state, &card.id);
            spent.face_up = Some(true);
            spent.exertion = Exertion {
                attacked: true,
                switched: true,
                attacks: None,
            };
        }
        let mut sink = sink_for(&mut state);
        let now = live(sink.state, &card.id);
        assert!(animate_card(&mut sink, &now, Default::default()));
        let after = live(sink.state, &card.id);
        assert_eq!(after.summoned_turn, Some(sink.state.turn));
        assert_eq!(
            after.exertion,
            Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            }
        );
        assert!(attack_targets(sink.state, &after).is_empty());
    }
}

mod r383_b3_1_animated_field_spells_and_animated_on_your_turn {
    use super::*;

    #[test]
    fn r383_an_animated_field_spell_animates_as_it_enters_the_field_and_holds_no_home() {
        let mut state = playing("animated-golem");
        let Some(card) = in_hand(&mut state, &golem.id, P1, 1).into_iter().next() else {
            return;
        };
        flush(&mut state, P1, 10);
        let result = act_result(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 3 }, "playerId": "p1"
            })),
        );
        assert_eq!(result.error, None);
        let next = &result.state;
        assert_eq!(id_at(next, slot(P1, Row::Units, 3)), Some(card.id.clone()));
        assert!(card_at(next, slot(P1, Row::Backrow, 3)).is_none());
        assert!(!is_reserved(next, slot(P1, Row::Backrow, 3)));
        let order: Vec<GameEventType> = result
            .events
            .iter()
            .map(|event| event.event_type())
            .filter(|kind| *kind == GameEventType::Summoned || *kind == GameEventType::Animated)
            .collect();
        assert_eq!(order, vec![GameEventType::Summoned, GameEventType::Animated]);
        expect_match(
            &view_for(next, P2).opponent.units[2],
            json!({ "defId": golem.id, "animated": {} }),
        );
    }

    #[test]
    fn r383_an_on_your_turn_card_played_on_its_controllers_turn_animates_at_once_and_holds_its_backrow_zone_for_its_return()
     {
        let mut state = playing("animated-spatula-play");
        let Some(card) = in_hand(&mut state, &spatula.id, P1, 1).into_iter().next() else {
            return;
        };
        flush(&mut state, P1, 10);
        let mut next = act(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1"
            })),
        );
        assert_eq!(id_at(&next, slot(P1, Row::Units, 2)), Some(card.id.clone()));
        assert_eq!(
            json_of(home_of(&next, &card.id)),
            json!({ "instanceId": card.id, "zone": { "player": "p1", "row": "backrow", "lane": 2 } })
        );
        assert!(is_reserved(&next, slot(P1, Row::Backrow, 2)));
        for viewer in [P1, P2] {
            let view = view_for(&next, viewer);
            let side = if viewer == P1 { &view.you } else { &view.opponent };
            assert_eq!(side.reserved.backrow, vec![false, true, false, false, false]);
            expect_match(
                &side.units[1],
                json!({ "defId": spatula.id, "animated": { "home": 2 } }),
            );
        }
        // Nothing else may enter the held zone.
        let Some(other) = in_hand(&mut next, &golem.id, P1, 1).into_iter().next() else {
            return;
        };
        flush(&mut next, P1, 10);
        let refused = act_result(
            &next,
            input(json!({
                "type": "play", "instanceId": other.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1"
            })),
        );
        assert!(refused.error.is_some());
    }

    #[test]
    fn r383_r1062_its_end_of_turn_text_runs_while_it_is_a_unit_and_cleanup_sends_it_home_its_next_start_of_turn_animates_it_again_not_summoning_sick()
     {
        let mut state = playing("animated-spatula-cycle");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 4),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 4)), Some(card.id.clone()));

        // §2.2: the end-of-turn hooks, then cleanup's return.
        run_hooks_in_trigger_order(&mut sink, HookName::EndOfTurn, Some(P1));
        return_at_cleanup(&mut sink, P1);
        assert_eq!(notes_of(find_instance(sink.state, &card.id)), vec!["units"]);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 4)),
            Some(card.id.clone())
        );
        assert_eq!(by_id(sink.state, &card.id).position, None);
        assert!(home_of(sink.state, &card.id).is_none());
        assert_eq!(
            json_of(events_of_type(sink.events, GameEventType::Deanimated)),
            json!([{
                "type": "deanimated", "player": "p1", "instanceId": card.id, "defId": spatula.id,
                "unitLane": 4, "backrowLane": 4
            }])
        );

        // On the opponent's turn, back in the backrow: no unit walk finds it, a backrow effect does (rule 7).
        assert!(!unit_ids(sink.state, P1).contains(&card.id));
        {
            let ctx = make_context(&mut sink, None, by(P2));
            assert!(!ids(&cards_in_scope(&ctx, &json_as(json!({ "side": "enemy" })))).contains(&card.id));
            assert!(
                ids(&cards_in_scope(
                    &ctx,
                    &json_as(json!({ "side": "enemy", "rows": ["backrow"] }))
                ))
                .contains(&card.id)
            );
        }
        assert_eq!(
            card_type_of(sink.state, &live(sink.state, &card.id)),
            CardType::FieldSpell
        );
        sink.state.turn += 2;
        animate_at_turn_start(&mut sink, P1);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 4)), Some(card.id.clone()));
        assert!(!is_sick(sink.state, &live(sink.state, &card.id)));
    }

    #[test]
    fn r383_moving_is_not_leaving_the_field_damage_buffs_counters_and_memory_stay_and_no_departure_is_counted()
     {
        let mut state = playing("animated-keeps");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        {
            let kept = by_id_mut(&mut state, &card.id);
            kept.damage = 1;
            kept.buffs = AttackHealth { attack: 2, health: 2 };
            kept.memory.insert("kept".to_string(), json!("yes"));
            kept.counters.plague = Some(2);
        }
        let mut sink = sink_for(&mut state);
        let exits = sink.state.field_exits.as_ref().map_or(0, |exits| exits.count);
        animate_at_turn_start(&mut sink, P1);
        return_at_cleanup(&mut sink, P1);
        animate_at_turn_start(&mut sink, P1);
        let after = by_id(sink.state, &card.id);
        assert_eq!(after.damage, 1);
        assert_eq!(after.buffs, AttackHealth { attack: 2, health: 2 });
        assert_eq!(after.memory.get("kept"), Some(&json!("yes")));
        assert_eq!(after.counters.plague, Some(2));
        assert_eq!(
            sink.state.field_exits.as_ref().map_or(0, |exits| exits.count),
            exits
        );
    }

    #[test]
    fn r383_with_no_open_unit_zone_an_on_your_turn_card_stays_in_the_backrow_and_a_face_down_one_never_animates_at_turn_start()
     {
        let mut state = playing("animated-turn-start-limits");
        fill_units(&mut state, P1);
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 1)),
            Some(card.id.clone())
        );
        assert!(home_of(sink.state, &card.id).is_none());
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r688_rule_6_a_lock_on_its_home_since_no_longer_stops_the_return_the_return_is_a_move_not_a_play() {
        let mut state = playing("animated-home-locked");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 3),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        lock_zone(sink.state, slot(P1, Row::Backrow, 3));
        return_at_cleanup(&mut sink, P1);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 3)),
            Some(card.id.clone())
        );
        assert!(home_of(sink.state, &card.id).is_none());
    }

    #[test]
    fn r383_rule_6_a_card_that_changed_sides_has_no_home_on_the_new_side_going_to_its_controllers_leftmost_open_backrow_zone()
     {
        let mut state = playing("animated-stolen");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        put(
            sink.state,
            &banner.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        (steal(json_as(
            json!({ "target": { "of": "instance", "instanceId": card.id } }),
        ))
        .apply)(&mut make_context(&mut sink, None, by(P2)));
        assert_eq!(by_id(sink.state, &card.id).controller, P2);
        return_at_cleanup(&mut sink, P2);
        // p2's backrow lane 1 is taken, so lane 2; the old home on p1's side is let go.
        assert_eq!(
            id_at(sink.state, slot(P2, Row::Backrow, 2)),
            Some(card.id.clone())
        );
        assert!(!is_reserved(sink.state, slot(P1, Row::Backrow, 2)));
        assert!(home_of(sink.state, &card.id).is_none());
    }

    #[test]
    fn r383_rule_6_a_card_dormant_under_a_stack_does_not_return_and_its_home_stays_held() {
        let mut state = playing("animated-buried");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 5),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        let top = put(
            sink.state,
            &stacker.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut stacked = top.clone();
        place_on_field(
            sink.state,
            &mut stacked,
            slot(P1, Row::Units, 5),
            json_as(json!({ "stack": true })),
        );
        return_at_cleanup(&mut sink, P1);
        let pile: Option<Vec<String>> = sink.state.players.p1.units[4]
            .as_ref()
            .map(|pile| pile.iter().map(|c| c.id.clone()).collect());
        assert_eq!(pile, Some(vec![top.id.clone(), card.id.clone()]));
        assert!(is_reserved(sink.state, slot(P1, Row::Backrow, 5)));
        let buried = live(sink.state, &card.id);
        assert!(!return_home(&mut sink, &buried));
    }

    #[test]
    fn r383_a_vanilla_card_has_lost_animated_where_it_stands_it_stays_a_unit_and_its_home_is_let_go() {
        let mut state = playing("animated-vanilla");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        by_id_mut(sink.state, &card.id).vanilla = true;
        assert!(animated_kind_of(sink.state, by_id(sink.state, &card.id)).is_none());
        return_at_cleanup(&mut sink, P1);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 1)), Some(card.id.clone()));
        assert!(home_of(sink.state, &card.id).is_none());
        assert!(!is_reserved(sink.state, slot(P1, Row::Backrow, 1)));
    }

    #[test]
    fn r383_a_card_leaving_the_field_from_its_unit_zone_lets_its_home_go() {
        let mut state = playing("animated-dies");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        state.turn += 1;
        state.active = P2;
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P2);
        by_id_mut(sink.state, &card.id).damage = 5;
        settle(&mut sink, SettleOptions::default());
        assert!(graveyard_ids(sink.state, P2).contains(&card.id));
        assert!(home_of(sink.state, &card.id).is_none());
        assert!(!is_reserved(sink.state, slot(P2, Row::Backrow, 1)));
    }

    #[test]
    fn r657_r2280000_an_animated_card_with_no_stats_no_longer_animates_or_fights_as_a_0_1() {
        let mut state = playing("animated-wisp");
        let mut sink = sink_for(&mut state);
        // Printed Animated, no stats on either face: it does not animate, no 0/1 read (R657, superseded).
        let base = put(
            sink.state,
            &wisp.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let now = live(sink.state, &base.id);
        assert!(!animate_card(&mut sink, &now, Default::default()));
        assert_eq!(unit_view(sink.state, by_id(sink.state, &base.id)).attack, 0);
        assert_eq!(unit_view(sink.state, by_id(sink.state, &base.id)).max_health, 0);
        let radiant = put(
            sink.state,
            &wisp.id,
            slot(P1, Row::Backrow, 3),
            json_as(json!({ "radiant": true })),
        );
        let now = live(sink.state, &radiant.id);
        assert!(!animate_card(&mut sink, &now, Default::default()));
        assert_eq!(
            unit_view(sink.state, by_id(sink.state, &radiant.id)).max_health,
            0
        );
        // Granted Animated on a stat-less card is the same.
        let granted = put(
            sink.state,
            &tower.id,
            slot(P1, Row::Backrow, 4),
            Default::default(),
        );
        by_id_mut(sink.state, &granted.id)
            .granted_keywords
            .push(Keyword::Animated);
        let now = live(sink.state, &granted.id);
        assert!(!animate_card(&mut sink, &now, Default::default()));
        assert_eq!(
            unit_view(sink.state, by_id(sink.state, &granted.id)).max_health,
            0
        );
        // All three stay in their backrow zones through the state check, no Unit to die.
        settle(&mut sink, SettleOptions::default());
        for (lane, card) in [(2, &base), (3, &radiant), (4, &granted)] {
            assert_eq!(
                id_at(sink.state, slot(P1, Row::Backrow, lane)),
                Some(card.id.clone())
            );
        }
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());
    }
}

mod r1062_animated_on_your_turn_at_start_of_turn_after_an_earlier_entry {
    use super::*;

    #[test]
    fn r1062_animated_at_its_start_of_turn_after_an_earlier_entry_it_is_not_summoning_sick_and_its_rush_reaches_the_hero()
     {
        let mut state = playing("r1062-not-sick");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let turn = state.turn;
        by_id_mut(&mut state, &card.id).summoned_turn = Some(turn);
        state.turn += 2;
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        let now = live(sink.state, &card.id);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 2)), Some(card.id.clone()));
        assert!(!is_sick(sink.state, &now));
        assert!(attack_targets(sink.state, &now).contains(&DamageTarget::Hero { player: P2 }));
    }

    /// D14: the Meditative set has shipped, so R1062 holds in every game, nothing previewed.
    #[test]
    fn r1062_holds_with_nothing_previewed_since_the_meditative_set_shipped_in_v0_3_5() {
        assert!(set_ships(SetName::Meditative));
        let mut state = playing("r1062-shipped");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let turn = state.turn;
        by_id_mut(&mut state, &card.id).summoned_turn = Some(turn);
        state.turn += 2;
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        let now = live(sink.state, &card.id);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 2)), Some(card.id.clone()));
        assert!(!is_sick(sink.state, &now));
        assert!(attack_targets(sink.state, &now).contains(&DamageTarget::Hero { player: P2 }));
    }

    #[test]
    fn r1062_one_that_came_to_its_side_this_turn_is_still_sick() {
        let mut state = playing("r1062-still-sick");
        let card = put(
            &mut state,
            &spatula.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        by_id_mut(&mut state, &card.id).summoned_turn = Some(state.turn);
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        let now = live(sink.state, &card.id);
        assert!(is_sick(sink.state, &now));
        assert!(!attack_targets(sink.state, &now).contains(&DamageTarget::Hero { player: P2 }));
    }

    #[test]
    fn r1062_r383_its_animation_as_it_enters_the_field_is_still_an_entry() {
        let mut state = playing("r1062-entry");
        let Some(card) = in_hand(&mut state, &spatula.id, P1, 1).into_iter().next() else {
            return;
        };
        flush(&mut state, P1, 10);
        let next = act(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1"
            })),
        );
        assert!(is_sick(&next, &live(&next, &card.id)));
    }
}

mod r113_r383_b3_1_a_pause_inside_an_animated_traps_firing {
    use super::*;

    /// p2's asking trap (backrow lane 1) fires at p1's play into unit lane 2 and asks: the state, its id.
    fn paused(seed: &str) -> (GameState, String) {
        let mut start = playing(seed);
        let trap = put(
            &mut start,
            &asker.id,
            slot(P2, Row::Backrow, 1),
            Default::default(),
        );
        let played = play_plain(&mut start, 2);
        (played.state, trap.id)
    }

    fn answer_hero_p1(state: &GameState) -> GameState {
        let choice_id = state
            .pending
            .as_ref()
            .map(|pending| pending.id.clone())
            .unwrap_or_default();
        act(
            state,
            input(json!({
                "type": "answer", "choiceId": choice_id, "selection": [{ "pick": "hero", "player": "p1" }],
                "playerId": "p2"
            })),
        )
    }

    #[test]
    fn r383_a_trap_whose_list_asks_animates_only_once_the_answer_has_run_the_rest_of_it() {
        let (state, trap) = paused("animated-pause");
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(P2));
        assert_eq!(id_at(&state, slot(P2, Row::Backrow, 1)), Some(trap.clone()));
        assert!(!state.work.is_empty());
        let next = answer_hero_p1(&state);
        assert_eq!(notes_of(find_instance(&next, &trap)), vec!["answered", "tail"]);
        assert_eq!(id_at(&next, slot(P2, Row::Units, 1)), Some(trap.clone()));
        assert!(next.work.is_empty());
    }

    #[test]
    fn r383_the_paused_firing_survives_a_json_round_trip_and_a_replay_to_the_same_state() {
        let run = |seed: &str, round_trip: bool| -> (String, String) {
            let (state, _) = paused(seed);
            let paused_hash = hash_state(&state);
            let from = if round_trip {
                json_round_trip(&state)
            } else {
                state
            };
            assert_eq!(hash_state(&from), paused_hash);
            let next = answer_hero_p1(&from);
            (paused_hash, hash_state(&next))
        };
        let live = run("animated-pause-replay", false);
        let revived = run("animated-pause-replay", true);
        let replayed = run("animated-pause-replay", false);
        assert_eq!(revived, live);
        assert_eq!(replayed, live);
    }

    #[test]
    fn r383_r2734000_hidden_information_the_opponent_reads_the_fired_trap_in_its_zone_then_the_animated_card_in_full_the_prompt_is_its_controllers_alone()
     {
        let (state, trap) = paused("animated-pause-view");
        let opponent_view = view_for(&state, P1);
        let fired = json_of(events_of_type(&opponent_view.events, GameEventType::TrapFired));
        let last = fired
            .as_array()
            .and_then(|list| list.last())
            .cloned()
            .unwrap_or(Value::Null);
        // Paused inside its firing (R113), the trap stands face-up in its zone (R33), public to both
        // seats, so its `trapFired` names it to the opponent too (R2734000).
        expect_match(
            &last,
            json!({ "instanceId": trap, "defId": asker.id, "row": "backrow", "lane": 1 }),
        );
        assert_eq!(
            json_of(&opponent_view.pending),
            json!({ "forYou": false, "pendingFor": "p2" })
        );
        expect_match(&view_for(&state, P2).pending, json!({ "forYou": true }));

        let next = answer_hero_p1(&state);
        for viewer in [P1, P2] {
            let view = view_for(&next, viewer);
            assert_eq!(
                json_of(events_of_type(&view.events, GameEventType::Animated)),
                json!([{
                    "type": "animated", "player": "p2", "instanceId": trap, "defId": asker.id,
                    "backrowLane": 1, "unitLane": 1
                }])
            );
            let side = if viewer == P2 { &view.you } else { &view.opponent };
            expect_match(
                &side.units[0],
                json!({ "instanceId": trap, "defId": asker.id, "animated": {} }),
            );
        }
    }
}

/// Fuses `ingredient` into `target`, the card the fusion keeps (R77), as a Fuse effect's verb does.
fn fuse_into(sink: &mut EngineSink<'_>, ingredient: &CardInstance, target: &CardInstance) -> CardInstance {
    fuse(
        sink,
        FuseArgs {
            ingredients: vec![live(sink.state, &ingredient.id)],
            target: Some(live(sink.state, &target.id)),
            ..FuseArgs::default()
        },
    )
    .expect("the fusion")
}

fn owed(state: &GameState, id: &str) -> bool {
    by_id(state, id).memory.contains_key(ANIMATE_OWED_KEY)
}

// R2280000 (issue #2280): a backrow card becomes a Unit only with both the Animated keyword and stats.
mod r2280000_a_backrow_card_becomes_a_unit_only_with_animated_and_stats {
    use super::*;

    #[test]
    fn r2280000_a_statless_animated_field_spell_enters_and_stays_in_the_backrow() {
        let mut state = playing("animated-statless-entry");
        let card = in_hand(&mut state, &wisp.id, P1, 1).remove(0);
        flush(&mut state, P1, 10);
        let result = act_result(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1"
            })),
        );
        assert_eq!(result.error, None);
        assert_eq!(
            id_at(&result.state, slot(P1, Row::Backrow, 2)),
            Some(card.id.clone())
        );
        assert!(card_at(&result.state, slot(P1, Row::Units, 2)).is_none());
        assert!(events_of_type(&result.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r2280000_a_statless_animated_trap_fires_and_stays_face_up_in_its_zone() {
        let mut state = playing("animated-statless-trap");
        let trap = put(
            &mut state,
            &mote.id,
            slot(P2, Row::Backrow, 3),
            Default::default(),
        );
        let played = play_plain(&mut state, 1);
        let state = played.state;
        assert_eq!(notes_of(find_instance(&state, &trap.id)), vec!["moted"]);
        assert_eq!(id_at(&state, slot(P2, Row::Backrow, 3)), Some(trap.id.clone()));
        assert_eq!(by_id(&state, &trap.id).face_up, Some(true));
        assert!(state.players.p2.graveyard.is_empty());
        assert!(events_of_type(&played.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r2280000_a_statless_on_your_turn_card_stays_at_its_start_of_turn() {
        let mut state = playing("animated-statless-turn");
        let card = put(
            &mut state,
            &tower.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        by_id_mut(&mut state, &card.id)
            .granted_keywords
            .push(Keyword::AnimatedOnYourTurn);
        let mut sink = sink_for(&mut state);
        animate_at_turn_start(&mut sink, P1);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 1)),
            Some(card.id.clone())
        );
        assert!(home_of(sink.state, &card.id).is_none());
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r2280000_a_field_spell_holding_a_fused_unit_stays_a_backrow_card() {
        let mut state = playing("animated-fused-banner");
        let flag = put(
            &mut state,
            &banner.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 3), Default::default());
        let mut sink = sink_for(&mut state);
        fuse_into(&mut sink, &unit, &flag);
        state_check(&mut sink);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 1)),
            Some(flag.id.clone())
        );
        assert!(has_stat_line(sink.state, by_id(sink.state, &flag.id)));
        assert!(!unit_ids(sink.state, P1).contains(&flag.id));
        assert!(!owed(sink.state, &flag.id));
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r2280000_buffs_and_auras_give_a_statless_card_no_stats() {
        let mut state = playing("animated-buffed-wisp");
        let card = put(
            &mut state,
            &wisp.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        // Banner's aura lends +2 attack to its controller's cards on the field.
        put(
            &mut state,
            &banner.id,
            slot(P1, Row::Backrow, 3),
            Default::default(),
        );
        by_id_mut(&mut state, &card.id).buffs = AttackHealth { attack: 2, health: 2 };
        let mut sink = sink_for(&mut state);
        let now = live(sink.state, &card.id);
        assert!(!has_stat_line(sink.state, &now));
        assert!(!animate_card(&mut sink, &now, Default::default()));
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 2)),
            Some(card.id.clone())
        );
    }

    #[test]
    fn r2280000_a_buff_skips_a_statless_animated_card() {
        let mut state = playing("animated-tune-reach");
        let statless = put(
            &mut state,
            &wisp.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let stated = put(
            &mut state,
            &golem.id,
            slot(P1, Row::Backrow, 3),
            Default::default(),
        );
        let rows =
            |state: &GameState, id: &str| applicable_changes(state, by_id(state, id), TuneDirection::Upgrade);
        assert!(!rows(&state, &statless.id).contains(&TuneRow::Stats));
        assert!(rows(&state, &stated.id).contains(&TuneRow::Stats));
        assert!(
            !applicable_changes(&state, by_id(&state, &statless.id), TuneDirection::Degrade)
                .contains(&TuneRow::Stats)
        );
    }

    #[test]
    fn r2280000_a_card_that_loses_animated_or_its_stats_stays_a_unit_where_it_stands() {
        let mut state = playing("animated-loses");
        let card = put(
            &mut state,
            &wisp.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        by_id_mut(&mut state, &card.id).stats_override = Some(AttackHealth { attack: 2, health: 2 });
        let stated = put(
            &mut state,
            &golem.id,
            slot(P1, Row::Backrow, 3),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        let now = live(sink.state, &card.id);
        assert!(animate_card(&mut sink, &now, Default::default()));
        let now = live(sink.state, &stated.id);
        assert!(animate_card(&mut sink, &now, Default::default()));

        // The wisp loses its stats (a buff keeps it alive), the golem its Animated (a Vanilla).
        let wisp_now = by_id_mut(sink.state, &card.id);
        wisp_now.stats_override = None;
        wisp_now.buffs = AttackHealth { attack: 0, health: 2 };
        by_id_mut(sink.state, &stated.id).vanilla = true;
        state_check(&mut sink);
        assert!(!has_stat_line(sink.state, by_id(sink.state, &card.id)));
        assert!(animated_kind_of(sink.state, by_id(sink.state, &stated.id)).is_none());
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 2)), Some(card.id.clone()));
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Units, 3)),
            Some(stated.id.clone())
        );
        assert!(is_animated(sink.state, by_id(sink.state, &card.id)));
        assert!(is_animated(sink.state, by_id(sink.state, &stated.id)));
    }
}

// R2280001: a face-up backrow card that first comes to have both animates at that state check.
mod r2280001_a_backrow_card_that_comes_to_have_both_animates_at_the_state_check {
    use super::*;

    #[test]
    fn r2280001_a_unit_fused_into_a_statless_animated_card_animates_it_at_the_state_check() {
        let mut state = playing("animated-owed");
        let card = put(
            &mut state,
            &wisp.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 4), Default::default());
        let turn = state.turn;
        let mut sink = sink_for(&mut state);
        fuse_into(&mut sink, &unit, &card);
        // Owed, not yet moved: the fusion itself animates nothing.
        assert!(owed(sink.state, &card.id));
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 2)),
            Some(card.id.clone())
        );
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());

        state_check(&mut sink);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 2)), Some(card.id.clone()));
        let view = unit_view(sink.state, by_id(sink.state, &card.id));
        assert_eq!((view.attack, view.health), (3, 3));
        assert_eq!(events_of_type(sink.events, GameEventType::Animated).len(), 1);
        // An entry, as every animation from the backrow is (R83, R171).
        assert_eq!(by_id(sink.state, &card.id).summoned_turn, Some(turn));
        assert!(!owed(sink.state, &card.id));
    }

    #[test]
    fn r2280001_with_no_open_unit_zone_it_stays_and_a_zone_opening_later_does_not_animate_it() {
        let mut state = playing("animated-owed-no-room");
        fill_units(&mut state, P1);
        let card = put(
            &mut state,
            &wisp.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let unit = in_hand(&mut state, &plain.id, P1, 1).remove(0);
        let mut sink = sink_for(&mut state);
        fuse_into(&mut sink, &unit, &card);
        state_check(&mut sink);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 2)),
            Some(card.id.clone())
        );
        assert!(!owed(sink.state, &card.id));

        // A unit zone opens: the card had both already, so nothing more is owed it.
        let first = id_at(sink.state, slot(P1, Row::Units, 1)).expect("a unit in zone 1");
        by_id_mut(sink.state, &first).damage = 99;
        state_check(&mut sink);
        assert!(card_at(sink.state, slot(P1, Row::Units, 1)).is_none());
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 2)),
            Some(card.id.clone())
        );
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r2280001_a_face_down_trap_never_animates_before_it_fires() {
        let mut state = playing("animated-owed-face-down");
        let trap = put(
            &mut state,
            &mote.id,
            slot(P1, Row::Backrow, 2),
            Default::default(),
        );
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 4), Default::default());
        let mut sink = sink_for(&mut state);
        fuse_into(&mut sink, &unit, &trap);
        assert!(!owed(sink.state, &trap.id));
        state_check(&mut sink);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 2)),
            Some(trap.id.clone())
        );
        assert!(is_face_down(sink.state, by_id(sink.state, &trap.id)));
        assert!(events_of_type(sink.events, GameEventType::Animated).is_empty());
    }

    #[test]
    fn r2280001_an_animated_keyword_granted_to_a_backrow_card_with_stats_animates_it() {
        let mut state = playing("animated-owed-granted");
        let flag = put(
            &mut state,
            &banner.id,
            slot(P1, Row::Backrow, 1),
            Default::default(),
        );
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 3), Default::default());
        let mut sink = sink_for(&mut state);
        fuse_into(&mut sink, &unit, &flag);
        state_check(&mut sink);
        assert_eq!(
            id_at(sink.state, slot(P1, Row::Backrow, 1)),
            Some(flag.id.clone())
        );

        (grant_keyword(json_as(json!({
            "target": { "of": "instance", "instanceId": flag.id },
            "keyword": { "kind": "Animated" },
        })))
        .apply)(&mut make_context(&mut sink, None, by(P1)));
        assert!(owed(sink.state, &flag.id));
        state_check(&mut sink);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 1)), Some(flag.id.clone()));
        assert_eq!(events_of_type(sink.events, GameEventType::Animated).len(), 1);
    }
}
