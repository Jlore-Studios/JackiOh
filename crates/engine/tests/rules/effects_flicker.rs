//! Flicker (docs/classic-sets.md B5 E22, R444): the card leaves the field and re-enters the same zone at
//! once — R78's reset, summoning sick, no Cry, no Death — and counts as summoned. A unit token comes back
//! (R444, as R175 brings one back through Reborn); a Trap re-enters face-down (R33, R227); an animated card
//! stays a Unit in its zone. Played through `reduce` with a pause after the flicker, a round trip, a
//! replay, and both seats' views. Fixtures: `fixtures/field.ts`.
//!
//! Port of `packages/engine/test/effects-flicker.test.ts`.

use jackioh_engine::effects::flicker::{flicker, flicker_card};
use jackioh_engine::effects::steal::steal;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::field::{act, act_result, blink, flush, playing, spatula, tesla, watcher};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    match find_instance(state, id) {
        Some(card) => card,
        None => panic!("no card {id}"),
    }
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

/// TS `sinkFor(state)`: the events and rng of a sink over `state`, the rng starting at the state's
/// cursor as reduce does. The state is lent to it call by call (`on`), so a test reads the state in
/// between as TS read its live objects.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> Sink {
    Sink { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
}

impl Sink {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }

    /// `effect.apply(makeContext(sink, null, { controller }))`.
    fn apply(&mut self, state: &mut GameState, effect: Effect, controller: PlayerId) {
        let mut sink = self.on(state);
        let mut ctx = make_context(&mut sink, None, HookOptions { controller: Some(controller), ..Default::default() });
        (effect.apply)(&mut ctx);
    }

    /// `flickerCard(sink, card)`, the card as it stands in the state now.
    fn flicker_card(&mut self, state: &mut GameState, id: &str) -> bool {
        let card = by_id(state, id).clone();
        flicker_card(&mut self.on(state), &card)
    }
}

/// `cardAt(state, ref)?.id`.
fn id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, &at).map(|card| card.id.clone())
}

fn as_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One field of each event, as TS's `.map((e) => e.<key>)` read it.
fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match as_json(events) {
        Value::Array(items) => items.into_iter().map(|item| item.get(key).cloned().unwrap_or(Value::Null)).collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

/// vitest's `toMatchObject`: every key `expected` names holds a matching value in `actual` (objects
/// recursively, arrays element by element and of the same length), other keys of `actual` ignored.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

fn expect_match_object<T: serde::Serialize>(actual: &T, expected: Value) {
    let actual = as_json(actual);
    assert!(matches_object(&actual, &expected), "{actual} does not match {expected}");
}

fn event_types(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

mod b5_e22_flicker {
    use super::*;

    #[test]
    fn leaves_and_re_enters_the_same_zone_at_once_reset_summoning_sick_in_attack_position_no_cry_no_death() {
        let mut state = playing("flicker-basic");
        let unit = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 3));
        {
            let card = live_mut(&mut state, &unit.id);
            card.damage = 2;
            card.buffs = AttackHealth { attack: 1, health: 1 };
            card.granted_keywords.push(Keyword::Taunt);
            card.counters.plague = Some(2);
            card.memory.insert("kept".to_string(), json!(true));
            card.position = Some(Position::Def);
            card.summoned_turn = Some(0);
            card.exertion = Exertion { attacked: true, switched: false, attacks: None };
        }
        let mut sink = sink_for(&state);
        let mark = exit_mark(&state);
        assert!(sink.flicker_card(&mut state, &unit.id));

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 3)), Some(unit.id.clone()));
        let card = by_id(&state, &unit.id);
        assert_eq!(card.damage, 0);
        assert_eq!(card.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(card.granted_keywords, Vec::<Keyword>::new());
        assert_eq!(card.counters, Counters::default());
        assert_eq!(card.memory, IndexMap::<String, Value>::new());
        assert_eq!(card.position, Some(Position::Atk));
        assert_eq!(card.summoned_turn, Some(state.turn));
        assert_eq!(card.exertion, Exertion { attacked: false, switched: false, attacks: None });
        // It left the field (R174: what was aimed at its stay is gone), and it counts as summoned.
        assert!(left_field_after(&state, mark, &unit.id));
        assert_eq!(event_types(&sink.events), vec![GameEventType::Flickered, GameEventType::Summoned]);
        assert_eq!(
            as_json(&events_of_type(&sink.events, GameEventType::Flickered)),
            json!([{ "type": "flickered", "player": "p1", "instanceId": unit.id, "defId": plain().id, "row": "units", "lane": 3 }])
        );
        assert!(!sink.events.iter().any(|event| matches!(
            event.event_type(),
            GameEventType::Destroyed | GameEventType::EnteredGraveyard
        )));
    }

    #[test]
    fn r444_a_flickered_unit_token_comes_back_it_re_enters_its_zone_and_never_ceases_to_exist() {
        let mut state = playing("flicker-token");
        let token = new_instance(&mut state, "fx-token-rush", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        place_on_field(&mut state, &token, &slot(PlayerId::P1, Row::Units, 2), Default::default());
        live_mut(&mut state, &token.id).damage = 1;
        let mut sink = sink_for(&state);
        assert!(sink.flicker_card(&mut state, &token.id));
        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 2)), Some(token.id.clone()));
        assert_eq!(by_id(&state, &token.id).zone, Zone::Field { player: PlayerId::P1, row: Row::Units, lane: 2 });
        assert_eq!(by_id(&state, &token.id).damage, 0);
        assert_eq!(state.players[PlayerId::P1].graveyard, Vec::<CardInstance>::new());
    }

    #[test]
    fn counts_as_summoned_an_enemy_tesla_answers_the_flicker_of_a_unit_on_the_other_side() {
        let mut state = playing("flicker-summoned");
        let zapper = put(&mut state, &tesla().id, slot(PlayerId::P2, Row::Backrow, 1));
        let unit = put(&mut state, &stacker().id, slot(PlayerId::P1, Row::Units, 1));
        let mut sink = sink_for(&state);
        sink.apply(&mut state, flicker(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id } }))), PlayerId::P1);
        // Offer the flicker's events to the traps, as the resolution loop does.
        settle(&mut sink.on(&mut state), Default::default());
        assert_eq!(pluck(&events_of_type(&sink.events, GameEventType::Summoned), "instanceId"), vec![json!(unit.id)]);
        assert_eq!(pluck(&events_of_type(&sink.events, GameEventType::TrapFired), "instanceId"), vec![json!(zapper.id)]);
        assert_eq!(by_id(&state, &unit.id).damage, 4);
    }

    #[test]
    fn r174_ends_every_delayed_effect_aimed_at_the_card_what_returns_is_a_new_arrival() {
        let mut state = playing("flicker-watchers");
        let unit = put(&mut state, &plain().id, slot(PlayerId::P2, Row::Units, 1));
        let mut sink = sink_for(&state);
        schedule_delayed(
            &mut sink.on(&mut state),
            PlayerId::P1,
            DelayedAt { phase: Phase::Start, player: PlayerId::P1 },
            Resume {
                def_id: plain().id,
                hook: "delayed".to_string(),
                step: String::new(),
                radiant: false,
                instance_id: None,
                data: IndexMap::new(),
            },
            Some(unit.id.clone()),
            None,
        );
        assert_eq!(state.delayed.len(), 1);
        sink.flicker_card(&mut state, &unit.id);
        assert_eq!(state.delayed, Vec::<DelayedEffect>::new());
    }

    #[test]
    fn a_stolen_unit_re_enters_the_same_zone_on_its_thief_s_side() {
        let mut state = playing("flicker-stolen");
        let unit = put(&mut state, &plain().id, slot(PlayerId::P2, Row::Units, 1));
        let mut sink = sink_for(&state);
        sink.apply(&mut state, steal(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id } }))), PlayerId::P1);
        let at = by_id(&state, &unit.id).zone.clone();
        sink.flicker_card(&mut state, &unit.id);
        assert_eq!(by_id(&state, &unit.id).zone, at);
        assert_eq!(by_id(&state, &unit.id).controller, PlayerId::P1);
        assert_eq!(by_id(&state, &unit.id).owner, PlayerId::P2);
    }

    #[test]
    fn flickers_every_card_a_scope_names_and_never_a_card_dormant_under_a_stack() {
        let mut state = playing("flicker-scope");
        let low = put(&mut state, &plain().id, slot(PlayerId::P2, Row::Units, 1));
        let top = put(&mut state, &stacker().id, slot(PlayerId::P2, Row::Units, 2));
        state.players[PlayerId::P2].units[1] = None;
        place_on_field(&mut state, &top, &slot(PlayerId::P2, Row::Units, 1), json_as(json!({ "stack": true })));
        let other = put(&mut state, &plain().id, slot(PlayerId::P2, Row::Units, 3));
        let mut sink = sink_for(&state);
        sink.apply(&mut state, flicker(json_as(json!({ "scope": { "side": "enemy" } }))), PlayerId::P1);
        assert_eq!(
            pluck(&events_of_type(&sink.events, GameEventType::Flickered), "instanceId"),
            vec![json!(top.id), json!(other.id)]
        );
        assert!(is_buried(&state, by_id(&state, &low.id)));
        assert!(!sink.flicker_card(&mut state, &low.id));
        let lane_one: Option<Vec<String>> =
            state.players[PlayerId::P2].units[0].as_ref().map(|pile| pile.iter().map(|card| card.id.clone()).collect());
        assert_eq!(lane_one, Some(vec![top.id.clone(), low.id.clone()]));
    }

    #[test]
    fn r227_r97_a_face_down_trap_re_enters_face_down_under_a_fresh_id_and_the_other_seat_cannot_link_the_two() {
        let mut state = playing("flicker-trap");
        let trap = put(&mut state, &watcher().id, slot(PlayerId::P2, Row::Backrow, 2));
        let old_id = trap.id.clone();
        let mut sink = sink_for(&state);
        sink.flicker_card(&mut state, &trap.id);
        // TS's live object took the fresh id; here the card is the one now in its zone.
        let trap = card_at(&state, &slot(PlayerId::P2, Row::Backrow, 2)).cloned().expect("the trap is back in its zone");
        assert_ne!(trap.id, old_id);
        assert_eq!(trap.def_id, watcher().id);
        assert!(find_instance(&state, &old_id).is_none());
        assert_eq!(trap.face_up, None);
        let summoned = events_of_type(&sink.events, GameEventType::Summoned);
        assert_eq!(
            as_json(&summoned),
            json!([{
                "type": "summoned", "player": "p2", "instanceId": trap.id, "defId": watcher().id,
                "row": "backrow", "lane": 2, "formerId": old_id,
            }])
        );
        state.applied.push(AppliedAction { nonce: "flicker-trap".to_string(), events: sink.events.clone() });
        let theirs = view_for(&state, PlayerId::P1);
        expect_match_object(
            &events_of_type(&theirs.events, GameEventType::Flickered),
            json!([{ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID, "lane": 2 }]),
        );
        expect_match_object(
            &events_of_type(&theirs.events, GameEventType::Summoned),
            json!([{ "instanceId": HIDDEN_ID, "defId": HIDDEN_ID }]),
        );
        assert!(!as_json(&theirs.events).to_string().contains(&old_id));
        let mine = view_for(&state, PlayerId::P2);
        expect_match_object(
            &events_of_type(&mine.events, GameEventType::Summoned),
            json!([{ "instanceId": trap.id, "formerId": old_id }]),
        );
    }

    #[test]
    fn an_animated_card_stays_a_unit_in_its_unit_zone_face_up_and_its_home_is_let_go() {
        let mut state = playing("flicker-animated");
        let card = put(&mut state, &spatula().id, slot(PlayerId::P1, Row::Backrow, 1));
        let mut sink = sink_for(&state);
        animate_card(&mut sink.on(&mut state), &card, Default::default());
        assert!(home_of(&state, &card.id).is_some());
        sink.flicker_card(&mut state, &card.id);
        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 1)), Some(card.id.clone()));
        assert_eq!(by_id(&state, &card.id).face_up, Some(true));
        assert!(home_of(&state, &card.id).is_none());
    }
}

mod b5_e22_flicker_inside_a_play_that_pauses_r113 {
    use super::*;

    fn cast(seed: &str) -> (GameState, String) {
        let mut state = playing(seed);
        let unit = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 2));
        live_mut(&mut state, &unit.id).damage = 2;
        let Some(spell) = in_hand(&mut state, &blink().id, PlayerId::P1, 1).into_iter().next() else {
            panic!("no card");
        };
        flush(&mut state, PlayerId::P1, 10);
        let result = act_result(
            &state,
            json_as(json!({
                "type": "play",
                "instanceId": spell.id,
                "targets": [{ "pick": "instance", "instanceId": unit.id }],
                "playerId": "p1",
            })),
        );
        if let Some(error) = result.error {
            panic!("{error}");
        }
        (result.state, unit.id)
    }

    fn answer_hero_p2(state: &GameState) -> GameState {
        let choice_id = state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
        act(
            state,
            json_as(json!({
                "type": "answer",
                "choiceId": choice_id,
                "selection": [{ "pick": "hero", "player": "p2" }],
                "playerId": "p1",
            })),
        )
    }

    #[test]
    fn the_flicker_has_happened_when_the_list_asks_and_the_answer_finishes_the_play() {
        let (state, unit) = cast("flicker-pause");
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P1));
        assert_eq!(by_id(&state, &unit).damage, 0);
        let done = answer_hero_p2(&state);
        assert!(done.pending.is_none());
        assert_eq!(id_at(&done, slot(PlayerId::P1, Row::Units, 2)), Some(unit));
    }

    #[test]
    fn the_paused_state_survives_a_json_round_trip_and_a_replay_to_the_same_hashes() {
        let run = |round_trip: bool| -> (String, String) {
            let (state, _) = cast("flicker-pause-replay");
            let paused = hash_state(&state);
            let from: GameState = if round_trip {
                serde_json::from_value(as_json(&state)).expect("the state parses")
            } else {
                state
            };
            let done = answer_hero_p2(&from);
            (paused, hash_state(&done))
        };
        let live = run(false);
        assert_eq!(run(true), live);
        assert_eq!(run(false), live);
    }

    #[test]
    fn both_seats_read_the_flicker_of_a_public_unit() {
        let (state, unit) = cast("flicker-pause-view");
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let events = view_for(&state, viewer).events;
            expect_match_object(
                &events_of_type(&events, GameEventType::Flickered),
                json!([{ "instanceId": unit, "defId": plain().id, "row": "units", "lane": 2 }]),
            );
        }
    }
}
