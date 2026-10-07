//! The Transform variants of patch v0.2.0 (docs/classic-sets.md B5 E24; §6.3 Transform, R23, R35,
//! R57): the cards beneath a Stack pile become copies of its top (Classic+ #4 Juhan Biggest Bat), and
//! a card becomes a random card of a pool (Classic+ #73.1 Classic Golem).
//!
//! Port of `packages/engine/test/transform-variants.test.ts`.

use jackioh_engine::effects::{transform_beneath, transform_random};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::generation::{
    act, body, classic_plus_unit, classic_spell, classic_unit, frozen, fuse_a, fuse_b, hand_card, immutable,
    juhan, playing, replayed,
};
use crate::rules::fixtures::harness::{events_of_type, put, sink_for, slot};

fn run(sink: &mut EngineSink<'_>, effect: Effect, self_: Option<&CardInstance>) {
    let mut ctx = make_context(
        sink.reborrow(),
        self_,
        HookOptions {
            controller: Some(PlayerId::P1),
            ..Default::default()
        },
    );
    (effect.apply)(&mut ctx);
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// The card as it stands in the state now (TS held the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    must(find_instance(state, &card.id).cloned(), "the card in the state")
}

/// p1's lane 1 holding `below` bottom to top — the last one is the top of the pile.
fn pile(state: &mut GameState, def_ids: &[String], owner: PlayerId) -> Vec<CardInstance> {
    let mut placed = vec![];
    for def_id in def_ids {
        let mut card = new_instance(&mut *state, def_id, owner, Zone::Hand { player: owner });
        if !place_on_field(
            &mut *state,
            &mut card,
            &slot(PlayerId::P1, Row::Units, 1),
            json_as(json!({ "stack": true })),
        ) {
            panic!("could not stack");
        }
        placed.push(card);
    }
    placed
}

/// `eventsOfType(events, kind)`, each event as its JSON.
fn of_type(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

/// `toMatchObject`: every key of `expected` is in `actual` with a matching value.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual.iter().zip(expected).all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

/// `state.players.p1.units[index]?.[0]`.
fn top_of(state: &GameState, index: usize) -> Option<CardInstance> {
    state.players.p1.units[index].as_ref().and_then(|cards| cards.first()).cloned()
}

mod e24_the_cards_beneath_a_stack_become_copies_of_its_top_classic_plus_4 {
    use super::*;

    #[test]
    fn r57_each_dormant_card_is_replaced_by_a_copy_of_the_top_its_face_and_buffs_the_old_cards_owner_place_and_position() {
        let mut start = playing("transform-beneath");
        let below = pile(&mut start.state, &[fuse_a.id.clone(), body.id.clone()], PlayerId::P2);
        // The bottom card is the opponent's, stolen onto p1's lane long ago: the copy keeps that owner.
        let bottom_card = must(below.first().cloned(), "the bottom card");
        let middle_card = must(below.get(1).cloned(), "the middle card");
        must(find_instance_mut(&mut start.state, &middle_card.id), "the middle card").position = Some(Position::Def);
        let top = hand_card(&mut start.state, &juhan.id, PlayerId::P1);
        {
            let card = must(find_instance_mut(&mut start.state, &top.id), "the top");
            card.radiant = true;
            card.buffs = AttackHealth { attack: 1, health: 1 };
        }
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            json!({ "type": "play", "instanceId": top.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
        );

        let lane = must(run1.state.players.p1.units[0].clone(), "the pile");
        assert_eq!(lane.len(), 3);
        assert_eq!(lane[0].id, top.id);
        for copy in &lane[1..] {
            assert_eq!(copy.def_id, juhan.id);
            assert!(copy.radiant);
            assert_eq!(copy.buffs, AttackHealth { attack: 1, health: 1 });
            assert_eq!(copy.owner, PlayerId::P2);
            assert_eq!(copy.controller, PlayerId::P1);
        }
        assert_eq!(lane[1].position, Some(Position::Def));
        // The replaced cards ceased to exist: no graveyard, no Death (R35).
        assert!(find_instance(&run1.state, &bottom_card.id).is_none());
        assert!(find_instance(&run1.state, &middle_card.id).is_none());
        assert!(run1.state.players.p2.graveyard.is_empty());
        let applied = run1.state.applied.last().map(|applied| applied.events.clone()).unwrap_or_default();
        assert_eq!(
            of_type(&applied, GameEventType::Transformed)
                .into_iter()
                .map(|event| json!([event["fromDefId"], event["toDefId"]]))
                .collect::<Vec<_>>(),
            vec![json!([body.id, juhan.id]), json!([fuse_a.id, juhan.id])]
        );
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));

        // They stay dormant (R13) and resume in order when the top leaves.
        let mut after = run1.state;
        assert!(unit_view(&after, &lane[1]).attack > 0);
        remove_from_field(&mut after, &lane[0], Default::default());
        assert_eq!(top_of(&after, 0).map(|card| card.id), Some(lane[1].id.clone()));
    }

    #[test]
    fn r23_an_immutable_card_beneath_stays_as_it_is() {
        let mut start = playing("transform-beneath-immutable");
        let below = pile(&mut start.state, &[immutable.id.clone(), body.id.clone()], PlayerId::P1);
        let (locked, unlocked) = (below[0].clone(), must(below.get(1).cloned(), "a card"));
        let top = must(pile(&mut start.state, &[juhan.id.clone()], PlayerId::P1).first().cloned(), "the top");
        {
            let mut sink = sink_for(&mut start.state);
            run(&mut sink, transform_beneath(Default::default()), Some(&top));
        }
        let lane = must(start.state.players.p1.units[0].clone(), "the pile");
        assert_eq!(
            lane.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            vec![juhan.id.clone(), juhan.id.clone(), immutable.id.clone()]
        );
        assert_eq!(lane[2].id, locked.id);
        assert!(find_instance(&start.state, &unlocked.id).is_none());
    }

    #[test]
    fn e24_a_card_that_is_not_the_top_of_a_unit_pile_changes_nothing() {
        let mut start = playing("transform-beneath-not-top");
        let below = pile(&mut start.state, &[body.id.clone(), juhan.id.clone()], PlayerId::P1);
        let under = must(below.first().cloned(), "the dormant card");
        let mut sink = sink_for(&mut start.state);
        run(&mut sink, transform_beneath(Default::default()), Some(&under));
        let spell = hand_card(&mut *sink.state, &juhan.id, PlayerId::P1);
        run(&mut sink, transform_beneath(Default::default()), Some(&spell));
        assert!(sink.events.is_empty());
    }
}

mod e24_a_card_becomes_a_random_card_of_a_pool_classic_plus_73_1 {
    use super::*;

    #[test]
    fn r35_on_the_field_only_a_card_of_its_row_a_unit_becomes_a_random_classic_or_classic_plus_unit_never_a_spell_of_the_pool() {
        let mut start = playing("transform-random");
        let golem = put(&mut start.state, &fuse_b.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        must(find_instance_mut(&mut start.state, &golem.id), "the golem").damage = 1;
        let golem = live(&start.state, &golem);
        let events = {
            let mut sink = sink_for(&mut start.state);
            run(
                &mut sink,
                transform_random(json_as(
                    json!({ "instanceId": golem.id, "query": { "set": ["Classic", "Classic+"] } }),
                )),
                Some(&golem),
            );
            sink.events.clone()
        };

        let now = must(top_of(&start.state, 1), "the new Unit");
        assert!([classic_unit.id.clone(), classic_plus_unit.id.clone()].contains(&now.def_id));
        assert_ne!(now.def_id, classic_spell.id);
        assert_ne!(now.id, golem.id);
        assert_eq!(now.damage, 0);
        assert!(!now.radiant);
        assert!(is_sick(&start.state, &now));
        // TS read `golem.zone` off the replaced object, now `{ z: "gone", player: "p1" }`: a Rust test
        // holds no live object, so the card that ceased to exist (R35) is read as gone from the state.
        assert!(find_instance(&start.state, &golem.id).is_none());
        assert_eq!(events_of_type(&events, GameEventType::Transformed).len(), 1);
    }

    #[test]
    fn r424_ready_to_attack_the_new_unit_is_not_summoning_sick_this_turn_radiant_keep_keeps_the_old_face() {
        let mut start = playing("transform-random-ready");
        let golem = put(
            &mut start.state,
            &fuse_b.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({ "radiant": true }),
        );
        let turn = start.state.turn;
        must(find_instance_mut(&mut start.state, &golem.id), "the golem").summoned_turn = Some(turn);
        let golem = live(&start.state, &golem);
        {
            let mut sink = sink_for(&mut start.state);
            run(
                &mut sink,
                transform_random(json_as(json!({
                    "instanceId": golem.id,
                    "query": { "set": "Classic+" },
                    "radiant": "keep",
                    "readyToAttack": true,
                }))),
                Some(&golem),
            );
        }
        let now = must(top_of(&start.state, 2), "the new Unit");
        assert_eq!(now.def_id, classic_plus_unit.id);
        assert!(now.radiant);
        assert!(!is_sick(&start.state, &now));
        assert_eq!(
            serde_json::to_value(now.exertion).expect("serialises"),
            json!({ "attacked": false, "switched": false })
        );
    }

    #[test]
    fn r129_a_refusal_draws_nothing_an_immutable_card_an_empty_pool_a_pool_with_nothing_of_its_row() {
        let mut start = playing("transform-random-refused");
        let locked = put(&mut start.state, &immutable.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let unlocked = put(&mut start.state, &body.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let mut sink = sink_for(&mut start.state);
        let cursor = sink.rng.cursor();
        run(
            &mut sink,
            transform_random(json_as(json!({ "instanceId": locked.id, "query": { "set": "Classic" } }))),
            None,
        );
        run(
            &mut sink,
            transform_random(json_as(json!({ "instanceId": unlocked.id, "query": { "tags": ["Pancake"] } }))),
            None,
        );
        run(
            &mut sink,
            transform_random(json_as(
                json!({ "instanceId": unlocked.id, "query": { "defId": [classic_spell.id] } }),
            )),
            None,
        );
        assert_eq!(sink.rng.cursor(), cursor);
        assert!(sink.events.is_empty());
        assert_eq!(top_of(&start.state, 0).map(|card| card.id), Some(locked.id.clone()));
        assert_eq!(top_of(&start.state, 1).map(|card| card.id), Some(unlocked.id.clone()));
    }

    #[test]
    fn b4_1_the_running_card_never_becomes_itself_and_a_card_in_hand_is_replaced_there_hidden_from_the_other_player() {
        let mut start = playing("transform-random-hand");
        let held = hand_card(&mut start.state, &body.id, PlayerId::P1);
        let running = put(&mut start.state, &body.id, slot(PlayerId::P1, Row::Units, 4), json!({}));
        let events = {
            let mut sink = sink_for(&mut start.state);
            // The pool names the running card's own definition; it is excluded (R387).
            run(
                &mut sink,
                transform_random(json_as(
                    json!({ "instanceId": held.id, "query": { "defId": [body.id, classic_unit.id] } }),
                )),
                Some(&running),
            );
            sink.events.clone()
        };
        let state = &mut start.state;
        let replaced = must(state.players.p1.hand.last().cloned(), "the new hand card");
        assert_eq!(replaced.def_id, classic_unit.id);
        assert_eq!(def_of(state, &replaced.def_id).set, SetName::Classic);
        state.applied.push(AppliedAction {
            nonce: "transform-view".to_string(),
            events,
        });
        let theirs = of_type(&view_for(state, PlayerId::P2).events, GameEventType::Transformed).pop();
        assert!(theirs.is_some_and(|event| matches_object(
            &event,
            &json!({ "instanceId": HIDDEN_ID, "fromDefId": HIDDEN_ID, "toDefId": HIDDEN_ID })
        )));
        let mine = of_type(&view_for(state, PlayerId::P1).events, GameEventType::Transformed).pop();
        assert!(mine.is_some_and(|event| matches_object(&event, &json!({ "toDefId": classic_unit.id }))));
    }
}
