//! Backrow piles and carriers (docs/classic-sets.md B5 E21; R446, R447): a backrow card with Stack tops
//! an occupied backrow zone, only the top acts (a face-down trap under a pile never fires, an aura under
//! one is off), a pile travels whole; and a carrier (Classic+ #33 Ivory Tower) holds one Unit played on
//! top of it — a Unit for every rule that can neither attack nor be attacked, stepping down into a unit
//! zone when its zone stops carrying it; a carrier that fuses its Unit (Classic+ #33 Ivory Tower, R653)
//! takes one a stay. Pauses, a round trip, a replay and each seat's view included.
//! Fixtures: `fixtures/field.ts`.
//!
//! Port of `packages/engine/test/backrow-piles.test.ts`.

use serde::Serialize;

use jackioh_engine::effects::destroy::destroy_all;
use jackioh_engine::effects::move_::bounce_card;
use jackioh_engine::effects::swap::swap_board;
use jackioh_engine::effects::targets::cards_in_scope;
use jackioh_engine::effects::transform::transform;
use jackioh_engine::subsystems::activate::why_cannot_activate_ability;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{indestructible, plain, taunter};
use crate::rules::fixtures::field::{
    act, act_result, banner, cover, flush, fuser, mourner, notes_of, playing, tower, watcher, wrecker,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, sink_for, slot};
use crate::rules::fixtures::scripts::heroic_power;

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively; an
/// array matches element by element and in length; anything else is equal.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

fn expect_match(actual: impl Serialize, expected: Value) {
    let actual = json_of(actual);
    assert!(matches_object(&actual, &expected), "{actual} does not match {expected}");
}

/// `events.map((event) => event.instanceId)`, read off the events' JSON.
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

/// The refusal a `why…` check gives, or `None` when it allows (TS `string | null`).
fn refusal(check: Result<(), EngineError>) -> Option<String> {
    check.err().map(|error| error.message)
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, at).map(|card| card.id.clone())
}

fn beneath_ids(state: &GameState, at: ZoneSlot) -> Vec<String> {
    beneath_at(state, at).iter().map(|card| card.id.clone()).collect()
}

fn graveyard_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].graveyard.iter().map(|card| card.id.clone()).collect()
}

fn zone(row: Row, lane: i32) -> ZoneChoice {
    ZoneChoice { row, lane }
}

/// The first `cardPlayed` event's instance id, or "".
fn first_played(events: &[GameEvent]) -> String {
    instance_ids(events_of_type(events, GameEventType::CardPlayed))
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// TS `JSON.parse(JSON.stringify(state))`.
fn json_round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// The card as it stands in `state` now, owned (TS held the live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    by_id(state, id).clone()
}

/// Stack a new card of `top_def` onto the zone `under` stands in, as a Stack play or summon would (B5 E21).
fn stack_onto(state: &mut GameState, top_def: &str, under: &CardInstance) -> CardInstance {
    let Zone::Field { player, lane, .. } = by_id(state, &under.id).zone.clone() else {
        panic!("not on the field")
    };
    let mut top = new_instance(state, top_def, player, Zone::Hand { player });
    if !place_on_field(state, &mut top, slot(player, Row::Backrow, lane), json_as(json!({ "stack": true }))) {
        panic!("no stack");
    }
    live(state, &top.id)
}

/// p1 plays a fresh `def_id` from hand, into `zone` when given (`{ row, lane }`).
fn play_from(state: &mut GameState, def_id: &str, zone: Option<Value>) -> ReduceResult {
    let card = in_hand(state, def_id, P1, 1).into_iter().next().expect("no card");
    flush(state, P1, 10);
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": "p1" });
    if let Some(zone) = zone {
        body["zone"] = zone;
    }
    act_result(state, input(body))
}

mod r447_b5_e21_backrow_piles {
    use super::*;

    #[test]
    fn r447_a_backrow_card_with_stack_may_be_played_onto_an_occupied_backrow_zone_the_card_beneath_goes_dormant() {
        let mut state = playing("piles-play");
        let flag = put(&mut state, &banner.id, slot(P1, Row::Backrow, 1), Default::default());
        let Some(card) = in_hand(&mut state, &cover.id, P1, 1).into_iter().next() else { return };
        flush(&mut state, P1, 10);
        assert!(plays_on_stack(&state, &card));
        assert!(legal_zones_for(&state, P1, &card, None).contains(&zone(Row::Backrow, 1)));
        let mut next = act(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 1 }, "playerId": "p1"
            })),
        );
        assert_eq!(id_at(&next, slot(P1, Row::Backrow, 1)), Some(card.id.clone()));
        assert_eq!(beneath_ids(&next, slot(P1, Row::Backrow, 1)), vec![flag.id.clone()]);
        assert!(is_buried(&next, by_id(&next, &flag.id)));
        for viewer in [P1, P2] {
            let view = view_for(&next, viewer);
            let side = if viewer == P1 { &view.you } else { &view.opponent };
            expect_match(&side.backrow[0], json!({ "defId": cover.id, "buried": 1 }));
        }
        // A card without Stack still needs an empty zone.
        let Some(plain_card) = in_hand(&mut next, &banner.id, P1, 1).into_iter().next() else { return };
        assert!(!legal_zones_for(&next, P1, &plain_card, None).contains(&zone(Row::Backrow, 1)));
    }

    #[test]
    fn r447_an_aura_under_a_pile_is_off_and_on_again_once_the_top_leaves() {
        let mut state = playing("piles-aura");
        let flag = put(&mut state, &banner.id, slot(P1, Row::Backrow, 2), Default::default());
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        assert_eq!(view_for(&state, P1).you.units[0].as_ref().map(|unit| unit.attack), Some(5));
        let top = stack_onto(&mut state, &cover.id, &flag);
        assert_eq!(view_for(&state, P1).you.units[0].as_ref().map(|unit| unit.attack), Some(3));
        // The top leaves: the banner resumes acting.
        let mut sink = sink_for(&mut state);
        (destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P1),
        ));
        settle(&mut sink);
        assert!(graveyard_ids(sink.state, P1).contains(&top.id));
        assert_eq!(id_at(sink.state, slot(P1, Row::Backrow, 2)), Some(flag.id.clone()));
        assert_eq!(view_for(sink.state, P1).you.units[0].as_ref().map(|unit| unit.attack), Some(5));
        assert_eq!(by_id(sink.state, &unit.id).zone.z(), ZoneName::Field);
    }

    #[test]
    fn r447_a_face_down_trap_under_a_pile_never_fires_once_uncovered_it_answers_only_what_happens_after() {
        let mut state = playing("piles-trap");
        state.active = P2;
        let trap = put(&mut state, &watcher.id, slot(P2, Row::Backrow, 3), Default::default());
        let top = stack_onto(&mut state, &cover.id, &trap);
        state.active = P1;
        let first = play_from(&mut state, &plain.id, Some(json!({ "row": "units", "lane": 1 })));
        state = first.state;
        assert!(events_of_type(&first.events, GameEventType::TrapFired).is_empty());
        assert!(notes_of(find_instance(&state, &trap.id)).is_empty());

        // Take the top off: the trap resumes, face-down, and answers the next play.
        let mut sink = sink_for(&mut state);
        (destroy_all(json_as(json!({ "side": "enemy", "rows": ["backrow"] }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P1),
        ));
        settle(&mut sink);
        assert_eq!(id_at(sink.state, slot(P2, Row::Backrow, 3)), Some(trap.id.clone()));
        assert!(graveyard_ids(sink.state, P2).contains(&top.id));
        let second = play_from(&mut state, &plain.id, Some(json!({ "row": "units", "lane": 2 })));
        assert_eq!(
            instance_ids(events_of_type(&second.events, GameEventType::TrapFired)),
            vec![trap.id.clone()]
        );
    }

    #[test]
    fn r447_r212_a_card_that_resumes_answers_nothing_of_the_removal_that_uncovered_it_and_what_comes_after() {
        let mut state = playing("piles-uncovered");
        let under = put(&mut state, &mourner.id, slot(P1, Row::Backrow, 4), Default::default());
        let top = stack_onto(&mut state, &cover.id, &under);
        let unit = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        let mut sink = sink_for(&mut state);
        (destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P1),
        ));
        settle(&mut sink);
        assert_eq!(id_at(sink.state, slot(P1, Row::Backrow, 4)), Some(under.id.clone()));
        assert!(graveyard_ids(sink.state, P1).contains(&top.id));
        assert!(notes_of(find_instance(sink.state, &under.id)).is_empty());
        by_id_mut(sink.state, &unit.id).damage = 3;
        settle(&mut sink);
        assert_eq!(notes_of(find_instance(sink.state, &under.id)), vec!["mourned"]);
    }

    #[test]
    fn r447_hidden_information_a_face_down_card_beneath_is_a_count_to_both_seats_never_an_identity() {
        let mut state = playing("piles-hidden");
        let trap = put(&mut state, &watcher.id, slot(P2, Row::Backrow, 1), Default::default());
        stack_onto(&mut state, &cover.id, &trap);
        let mine = view_for(&state, P2);
        let theirs = view_for(&state, P1);
        expect_match(
            &theirs.opponent.backrow[0],
            json!({ "faceDown": false, "defId": cover.id, "buried": 1 }),
        );
        expect_match(&mine.you.backrow[0], json!({ "defId": cover.id, "buried": 1 }));
        let text = serde_json::to_string(&theirs).expect("serialises");
        assert!(!text.contains(&trap.id));
        assert!(!text.contains(&watcher.id));
        // A face-down top shows its back and the count.
        let deep = put(&mut state, &watcher.id, slot(P2, Row::Backrow, 4), Default::default());
        let upper = stack_onto(&mut state, &watcher.id, &deep);
        assert_eq!(
            json_of(&view_for(&state, P1).opponent.backrow[3]),
            json!({ "faceDown": true, "cost": 1, "buried": 1 })
        );
        assert_ne!(upper.id, deep.id);
    }

    #[test]
    fn r447_a_locked_zone_a_held_zone_and_a_zone_carrying_a_unit_take_no_stack_card() {
        let mut state = playing("piles-refusals");
        put(&mut state, &banner.id, slot(P1, Row::Backrow, 1), Default::default());
        put(&mut state, &banner.id, slot(P1, Row::Backrow, 2), Default::default());
        put(&mut state, &tower.id, slot(P1, Row::Backrow, 3), Default::default());
        lock_zone(&mut state, slot(P1, Row::Backrow, 1));
        reserve_zone(&mut state, slot(P1, Row::Backrow, 2));
        let mut rider = put(&mut state, &plain.id, slot(P1, Row::Units, 1), Default::default());
        state.players.p1.units[0] = None;
        assert!(place_on_field(&mut state, &mut rider, slot(P1, Row::Backrow, 3), Default::default()));
        let Some(card) = in_hand(&mut state, &cover.id, P1, 1).into_iter().next() else { return };
        let zones = legal_zones_for(&state, P1, &card, None);
        assert!(!zones.contains(&zone(Row::Backrow, 1)));
        assert!(!zones.contains(&zone(Row::Backrow, 2)));
        assert!(!zones.contains(&zone(Row::Backrow, 3)));
        assert!(zones.contains(&zone(Row::Backrow, 4)));
    }

    #[test]
    fn r447_a_board_swap_carries_a_backrow_pile_whole_top_on_top_to_the_other_side() {
        let mut state = playing("piles-swap");
        let flag = put(&mut state, &banner.id, slot(P1, Row::Backrow, 5), Default::default());
        let top = stack_onto(&mut state, &cover.id, &flag);
        let mut sink = sink_for(&mut state);
        (swap_board().apply)(&mut make_context(sink.reborrow(), None, by(P1)));
        assert_eq!(id_at(sink.state, slot(P2, Row::Backrow, 5)), Some(top.id.clone()));
        assert_eq!(beneath_ids(sink.state, slot(P2, Row::Backrow, 5)), vec![flag.id.clone()]);
        assert_eq!(by_id(sink.state, &flag.id).controller, P2);
        assert!(sink.state.players.p1.backrow_piles.is_none());
    }

    #[test]
    fn r447_a_card_dormant_under_a_backrow_pile_does_not_act_a_buried_heroic_power_cannot_be_used() {
        let mut state = playing("piles-power");
        let power = put(&mut state, &heroic_power().id, slot(P1, Row::Backrow, 3), Default::default());
        by_id_mut(&mut state, &power.id)
            .memory
            .insert("power".to_string(), json!("ping"));
        flush(&mut state, P1, 10);
        assert_eq!(refusal(why_cannot_activate_ability(&state, P1, &power.id, None)), None);
        stack_onto(&mut state, &cover.id, &power);
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &power.id, None)),
            Some("that card is under a pile and does not act".to_string())
        );
        assert!(!legal_actions(&state, P1).iter().any(|action| matches!(
            action,
            ActionBody::Activate { instance_id, .. } if *instance_id == power.id
        )));
    }

    #[test]
    fn r447_a_json_round_trip_keeps_the_pile_and_a_game_with_no_pile_carries_no_pile_field() {
        let plain_state = playing("piles-json-none");
        assert!(plain_state.players.p1.backrow_piles.is_none());
        assert!(plain_state.players.p1.carried.is_none());
        let mut state = playing("piles-json");
        let flag = put(&mut state, &banner.id, slot(P1, Row::Backrow, 1), Default::default());
        stack_onto(&mut state, &cover.id, &flag);
        let round = json_round_trip(&state);
        assert_eq!(hash_state(&round), hash_state(&state));
        assert!(is_buried(&round, by_id(&round, &flag.id)));
    }
}

mod r446_b5_e21_a_carrier_and_the_unit_it_holds {
    use super::*;

    /// What `towerGame` hands back: the state after p1 played a plain body onto a Tower in backrow lane
    /// 2, the Tower, and the carried Unit's id.
    struct TowerGame {
        state: GameState,
        tower: CardInstance,
        rider: String,
    }

    fn tower_game(seed: &str) -> TowerGame {
        let mut state = playing(seed);
        let holder = put(&mut state, &tower.id, slot(P1, Row::Backrow, 2), Default::default());
        let result = play_from(&mut state, &plain.id, Some(json!({ "row": "backrow", "lane": 2 })));
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        let rider = first_played(&result.events);
        TowerGame {
            state: result.state,
            tower: holder,
            rider,
        }
    }

    fn destroy_own_backrow(state: &mut GameState) -> Vec<GameEvent> {
        let mut sink = sink_for(state);
        (destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P1),
        ));
        settle(&mut sink);
        sink.events.clone()
    }

    #[test]
    fn r446_a_unit_may_be_played_on_top_of_a_carrier_offered_placed_there_with_the_carrier_acting_beneath() {
        let mut state = playing("carrier-play");
        put(&mut state, &tower.id, slot(P1, Row::Backrow, 2), Default::default());
        let Some(card) = in_hand(&mut state, &plain.id, P1, 1).into_iter().next() else { return };
        flush(&mut state, P1, 10);
        assert!(legal_zones_for(&state, P1, &card, None).contains(&zone(Row::Backrow, 2)));
        assert!(legal_actions(&state, P1).contains(&json_as(json!({
            "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 2 }
        }))));
        let result = act_result(
            &state,
            input(json!({
                "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1"
            })),
        );
        assert_eq!(result.error, None);
        let mut next = result.state;
        assert_eq!(
            carried_at(&next, slot(P1, Row::Backrow, 2)).map(|unit| unit.id.clone()),
            Some(card.id.clone())
        );
        assert_eq!(
            card_at(&next, slot(P1, Row::Backrow, 2)).map(|holder| holder.def_id.clone()),
            Some(tower.id.clone())
        );
        assert!(is_carried(&next, by_id(&next, &card.id)));
        assert!(
            active_units_of(&next, P1)
                .iter()
                .map(|unit| unit.id.clone())
                .any(|id| id == card.id)
        );
        expect_match(
            events_of_type(&result.events, GameEventType::Summoned),
            json!([{ "instanceId": card.id, "row": "backrow", "lane": 2 }]),
        );
        // The carrier's aura keeps working: the Unit in hand has Stack.
        let Some(held) = in_hand(&mut next, &plain.id, P1, 1).into_iter().next() else { return };
        assert!(plays_on_stack(&next, &held));
        // Only a carrier's zone: a plain backrow card takes no Unit.
        put(&mut next, &banner.id, slot(P1, Row::Backrow, 4), Default::default());
        let refused = act_result(
            &next,
            input(json!({
                "type": "play", "instanceId": held.id, "zone": { "row": "backrow", "lane": 4 }, "playerId": "p1"
            })),
        );
        assert!(refused.error.is_some());
    }

    #[test]
    fn r446_a_carried_unit_can_neither_attack_nor_be_attacked_and_its_taunt_binds_no_attacker() {
        let TowerGame { mut state, rider, .. } = tower_game("carrier-combat");
        state.turn += 2;
        assert!(attack_targets(&state, by_id(&state, &rider)).is_empty());
        let enemy = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
        by_id_mut(&mut state, &enemy.id).summoned_turn = Some(0);
        state.active = P2;
        assert!(!can_attack(
            &state,
            by_id(&state, &enemy.id),
            &AttackTarget::Unit { instance: live(&state, &rider) }
        ));
        let taunt = taunter.base.keywords.first().cloned().unwrap_or(Keyword::Taunt);
        by_id_mut(&mut state, &rider).granted_keywords.push(taunt);
        assert!(can_attack(&state, by_id(&state, &enemy.id), &AttackTarget::Hero { player: P1 }));
        let names: Vec<String> = attack_targets(&state, by_id(&state, &enemy.id))
            .iter()
            .map(|target| match target {
                AttackTarget::Hero { .. } => "hero".to_string(),
                AttackTarget::Unit { instance, .. } => instance.id.clone(),
            })
            .collect();
        assert_eq!(names, vec!["hero"]);
        // It may still switch position, a unit's own action.
        state.active = P1;
        state.phase = Phase::Main;
        assert!(legal_actions(&state, P1).contains(&json_as(json!({ "type": "switchPosition", "instanceId": rider }))));
        let switched = act(&state, input(json!({ "type": "switchPosition", "instanceId": rider, "playerId": "p1" })));
        assert_eq!(by_id(&switched, &rider).position, Some(Position::Def));
    }

    #[test]
    fn r446_it_is_a_unit_for_every_rule_all_units_reach_it_and_backrow_effects_find_the_carrier_instead() {
        let TowerGame { mut state, tower: holder, rider } = tower_game("carrier-scopes");
        {
            let ctx = make_context(sink_for(&mut state), None, by(P2));
            assert!(ids(&cards_in_scope(&ctx, &json_as(json!({ "side": "enemy" })))).contains(&rider));
            assert_eq!(
                ids(&cards_in_scope(&ctx, &json_as(json!({ "side": "enemy", "rows": ["backrow"] })))),
                vec![holder.id.clone()]
            );
        }
        let view = view_for(&state, P2).opponent;
        expect_match(
            json_of(&view.carried)[1].clone(),
            json!({ "instanceId": rider, "defId": plain.id }),
        );
        expect_match(&view.backrow[1], json!({ "defId": tower.id }));
    }

    #[test]
    fn r446_a_transform_replaces_a_carried_unit_where_it_stands_on_its_carrier() {
        let TowerGame { mut state, tower: holder, rider } = tower_game("carrier-transform");
        let mut sink = sink_for(&mut state);
        (transform(json_as(json!({ "instanceId": rider, "defId": taunter.id }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P2),
        ));
        let now = carried_at(sink.state, slot(P1, Row::Backrow, 2)).cloned();
        assert_eq!(now.as_ref().map(|unit| unit.def_id.clone()), Some(taunter.id.clone()));
        assert_ne!(now.as_ref().map(|unit| unit.id.clone()), Some(rider.clone()));
        assert_eq!(id_at(sink.state, slot(P1, Row::Backrow, 2)), Some(holder.id.clone()));
        assert_eq!(events_of_type(sink.events, GameEventType::Transformed).len(), 1);
    }

    #[test]
    fn r446_when_its_carrier_leaves_the_unit_steps_down_into_its_lanes_unit_zone_without_leaving_the_field() {
        let TowerGame { mut state, rider, .. } = tower_game("carrier-step-down");
        by_id_mut(&mut state, &rider).damage = 1;
        let exits = state.field_exits.as_ref().and_then(|exits| exits.last.get(&rider).copied());
        let events = destroy_own_backrow(&mut state);
        assert_eq!(id_at(&state, slot(P1, Row::Units, 2)), Some(rider.clone()));
        assert!(!is_carried(&state, by_id(&state, &rider)));
        assert_eq!(by_id(&state, &rider).damage, 1);
        assert_eq!(state.field_exits.as_ref().and_then(|exits| exits.last.get(&rider).copied()), exits);
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Animated)),
            json!([{
                "type": "animated", "player": "p1", "instanceId": rider, "defId": plain.id,
                "backrowLane": 2, "unitLane": 2, "carried": true
            }])
        );
        assert!(state.players.p1.carried.is_none());
    }

    #[test]
    fn r446_its_lane_taken_it_goes_to_the_leftmost_open_unit_zone_with_none_it_is_destroyed_and_an_indestructible_one_waits() {
        let TowerGame { mut state, rider, .. } = tower_game("carrier-no-room");
        put(&mut state, &plain.id, slot(P1, Row::Units, 2), Default::default());
        destroy_own_backrow(&mut state);
        assert_eq!(id_at(&state, slot(P1, Row::Units, 1)), Some(rider.clone()));

        let mut full = tower_game("carrier-full");
        for lane in 1..=5 {
            put(&mut full.state, &plain.id, slot(P1, Row::Units, lane), Default::default());
        }
        destroy_own_backrow(&mut full.state);
        assert!(graveyard_ids(&full.state, P1).contains(&full.rider));

        let mut tough = tower_game("carrier-indestructible");
        for lane in 1..=5 {
            put(&mut tough.state, &plain.id, slot(P1, Row::Units, lane), Default::default());
        }
        by_id_mut(&mut tough.state, &tough.rider).def_id = indestructible.id.clone();
        let mut tough_sink = sink_for(&mut tough.state);
        (destroy_all(json_as(json!({ "side": "self", "rows": ["backrow"] }))).apply)(&mut make_context(
            tough_sink.reborrow(),
            None,
            by(P1),
        ));
        settle(&mut tough_sink);
        assert!(is_carried(tough_sink.state, by_id(tough_sink.state, &tough.rider)));
        // It is destroyed once, not again at every check while it waits.
        let destroyed_before = tough_sink.state.counters.destroyed;
        settle(&mut tough_sink);
        assert_eq!(tough_sink.state.counters.destroyed, destroyed_before);
        // A unit zone opens: it steps down at the next check.
        tough_sink.state.players.p1.units[3] = None;
        settle(&mut tough_sink);
        assert_eq!(id_at(tough_sink.state, slot(P1, Row::Units, 4)), Some(tough.rider.clone()));
    }

    #[test]
    fn r446_a_vanilla_carrier_carries_no_more_its_unit_steps_down() {
        let TowerGame { mut state, tower: placed, rider } = tower_game("carrier-vanilla");
        by_id_mut(&mut state, &placed.id).vanilla = true;
        let mut sink = sink_for(&mut state);
        settle(&mut sink);
        assert_eq!(id_at(sink.state, slot(P1, Row::Units, 2)), Some(rider.clone()));
        assert_eq!(id_at(sink.state, slot(P1, Row::Backrow, 2)), Some(placed.id.clone()));
    }

    #[test]
    fn r446_a_pause_after_the_carrier_left_keeps_the_unit_carried_until_the_list_is_whole_through_a_round_trip_and_a_replay() {
        let run = |seed: &str, round_trip: bool| -> (String, String) {
            let TowerGame { mut state, rider, .. } = tower_game(seed);
            let cast = play_from(&mut state, &wrecker.id, None);
            if let Some(error) = &cast.error {
                panic!("{error}");
            }
            let mut paused = cast.state;
            assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(P1));
            // The Tower is marked; the check waits for the whole list, so the Unit is still where it was.
            assert!(is_carried(&paused, by_id(&paused, &rider)));
            let paused_hash = hash_state(&paused);
            if round_trip {
                paused = json_round_trip(&paused);
            }
            let choice_id = paused.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
            let done = act(
                &paused,
                input(json!({
                    "type": "answer", "choiceId": choice_id, "selection": [{ "pick": "hero", "player": "p2" }],
                    "playerId": "p1"
                })),
            );
            assert_eq!(id_at(&done, slot(P1, Row::Units, 2)), Some(rider.clone()));
            assert!(done.players.p1.graveyard.iter().any(|card| card.def_id == tower.id));
            (paused_hash, hash_state(&done))
        };
        let live = run("carrier-pause", false);
        assert_eq!(run("carrier-pause", true), live);
        assert_eq!(run("carrier-pause", false), live);
    }
}

mod r653_a_carrier_that_fuses_its_unit_takes_one_unit_a_stay {
    use super::*;

    #[test]
    fn r653_the_first_unit_to_stand_on_it_is_noted_and_no_other_may_name_its_zone_even_once_that_one_has_gone() {
        let mut state = playing("fuser-once");
        let holder = put(&mut state, &fuser.id, slot(P1, Row::Backrow, 2), Default::default());
        assert_eq!(stacked_onto(&holder).map(|id| id.to_string()), None);
        let result = play_from(&mut state, &plain.id, Some(json!({ "row": "backrow", "lane": 2 })));
        assert_eq!(result.error, None);
        let rider = first_played(&result.events);
        let mut next = result.state;
        assert_eq!(
            carried_at(&next, slot(P1, Row::Backrow, 2)).map(|unit| unit.id.clone()),
            Some(rider.clone())
        );
        assert_eq!(
            stacked_onto(by_id(&next, &holder.id)).map(|id| id.to_string()),
            Some(rider.clone())
        );
        // The Unit goes (here: destroyed where it stands); the carrier stays, and takes no other.
        let mut sink = sink_for(&mut next);
        (destroy_all(json_as(json!({ "side": "self", "rows": ["units"] }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P1),
        ));
        settle(&mut sink);
        assert!(carried_at(sink.state, slot(P1, Row::Backrow, 2)).is_none());
        let Some(held) = in_hand(sink.state, &plain.id, P1, 1).into_iter().next() else { return };
        assert!(!legal_zones_for(sink.state, P1, &held, None).contains(&zone(Row::Backrow, 2)));
        let refused = play_from(sink.state, &plain.id, Some(json!({ "row": "backrow", "lane": 2 })));
        assert!(refused.error.as_deref().unwrap_or_default().contains("taken its one Unit"));
        // Leaving the field clears the note (R78): back on the field, it is a new arrival and takes one again.
        let gone = live(sink.state, &holder.id);
        bounce_card(&mut sink, &gone);
        let gone = by_id(sink.state, &holder.id);
        assert_eq!(gone.zone.z(), ZoneName::Hand);
        assert_eq!(stacked_onto(gone).map(|id| id.to_string()), None);
    }

    #[test]
    fn r653_r23_an_immutable_one_takes_none_since_its_text_could_not_take_the_unit_in() {
        let mut state = playing("fuser-immutable");
        let holder = put(&mut state, &fuser.id, slot(P1, Row::Backrow, 2), Default::default());
        by_id_mut(&mut state, &holder.id).granted_keywords.push(Keyword::Immutable);
        let Some(card) = in_hand(&mut state, &plain.id, P1, 1).into_iter().next() else { return };
        assert!(!legal_zones_for(&state, P1, &card, None).contains(&zone(Row::Backrow, 2)));
        let refused = play_from(&mut state, &plain.id, Some(json!({ "row": "backrow", "lane": 2 })));
        assert!(refused.error.as_deref().unwrap_or_default().contains("Immutable"));
        by_id_mut(&mut state, &holder.id).granted_keywords.clear();
        assert!(legal_zones_for(&state, P1, &card, None).contains(&zone(Row::Backrow, 2)));
    }
}
