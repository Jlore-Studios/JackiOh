//! Plague Counters, extended (docs/classic-sets.md B5 E19, R471, R689): "Place N Plague Counters" as N
//! placements all on the one permanent a single prompt names, "Place N on X" as one placement,
//! placement multipliers, the "placed on this" trigger, stats per token through the layers, removal,
//! and what each player sees.

use std::collections::BTreeSet;

use jackioh_engine::testkit::*;

use jackioh_engine::effects::counters;
use jackioh_engine::effects::{
    CastNewArgs, CastNewDef, cast_new, consume_plague, place_plague, place_plague_each, place_plague_random,
    place_plague_tokens,
};
use jackioh_engine::layers::unit_view;
use jackioh_engine::plague::{
    permanents_on_field, place_plague_on, plague_multiplier_of, plague_on, plague_on_field, remove_plague,
};
use jackioh_engine::replay::hash_state;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::state::{CardInstance, GameState, find_instance, find_instance_mut, new_instance};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::triggers::settle;
use jackioh_engine::view_for::{HIDDEN_ID, HIDDEN_OPTION_LABEL, view_for};
use jackioh_engine::zones::{PlaceOnFieldOptions, place_on_field};

use super::fixtures::generation::{
    Run, act, answer, big_body, body, charger, crawler, dusting, frozen, hand_card, outbreak, plague_book,
    playing, quiet_trap, refusal, replayed, scatter, slime, toxins,
};
use super::fixtures::harness::{put, slot};

fn pick(card: &CardInstance) -> Selection {
    Selection::Instance {
        instance_id: card.id.clone(),
    }
}

/// The effect applied as p1's, the targets picked by instance.
fn run(sink: &mut EngineSink, effect: Effect, self_: Option<&CardInstance>, targets: &[&CardInstance]) {
    let mut ctx = make_context(
        sink,
        self_,
        HookOptions {
            controller: Some(PlayerId::P1),
            targets: Some(targets.iter().map(|card| pick(card)).collect()),
            ..Default::default()
        },
    );
    (effect.apply)(&mut ctx);
}

/// The Plague Counter changes in `events`: `{ id, value, placed? }`.
fn placements(events: &[GameEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == "counterChanged" && event["counter"] == "plague")
        .map(|event| {
            let mut entry = json!({ "id": event["instanceId"], "value": event["value"] });
            if !event["placed"].is_null() {
                entry["placed"] = event["placed"].clone();
            }
            entry
        })
        .collect()
}

fn placement(id: &str, value: i32, placed: Option<i32>) -> Value {
    match placed {
        Some(placed) => json!({ "id": id, "value": value, "placed": placed }),
        None => json!({ "id": id, "value": value }),
    }
}

/// A board with a permanent in each kind of place: p1's unit, p2's unit, p2's face-down trap.
struct Board {
    run: Run,
    mine: CardInstance,
    theirs: CardInstance,
    trap: CardInstance,
}

fn board(seed: &str) -> Board {
    let mut started = playing(seed);
    let mine = put(
        &mut started.state,
        &body.id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({}),
    );
    let theirs = put(
        &mut started.state,
        &big_body.id,
        slot(PlayerId::P2, Row::Units, 2),
        json!({}),
    );
    let trap = put(
        &mut started.state,
        &quiet_trap.id,
        slot(PlayerId::P2, Row::Backrow, 3),
        json!({}),
    );
    Board {
        run: started,
        mine,
        theirs,
        trap,
    }
}

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

/// The events of the last `n` applied actions, in order.
fn last_events(state: &GameState, n: usize) -> Vec<GameEvent> {
    let from = state.applied.len().saturating_sub(n);
    state.applied[from..]
        .iter()
        .flat_map(|entry| entry.events.clone())
        .collect()
}

fn top_of(state: &GameState, player: PlayerId, lane: usize) -> Option<&CardInstance> {
    state.players[player].units[lane - 1]
        .as_ref()
        .and_then(|pile| pile.first())
}

fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn play(card: &CardInstance) -> Value {
    json!({ "type": "play", "instanceId": card.id, "playerId": "p1" })
}

fn play_on(card: &CardInstance, target: &CardInstance) -> Value {
    json!({ "type": "play", "instanceId": card.id, "targets": [pick(target)], "playerId": "p1" })
}

/// Every key `expected` names holds the same value in `actual` (objects and arrays recursively).
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(have), Value::Object(want)) => want
            .iter()
            .all(|(key, value)| have.get(key).is_some_and(|got| matches_object(got, value))),
        (Value::Array(have), Value::Array(want)) => {
            have.len() == want.len()
                && have
                    .iter()
                    .zip(want)
                    .all(|(got, value)| matches_object(got, value))
        }
        _ => actual == expected,
    }
}

fn view_json(state: &GameState, player: PlayerId) -> Value {
    serde_json::to_value(view_for(state, player)).expect("a view serialises")
}

fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

fn thaw(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state serialises")).expect("a state parses")
}

/// A sink over `state`, the rng at the state's cursor; the cursor is never written back.
macro_rules! sink_for {
    ($sink:ident, $state:expr) => {
        let mut rng = Rng::new(&$state.seed, $state.rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        #[allow(unused_mut)]
        let mut $sink = EngineSink::new(&mut $state, &mut events, &mut rng);
    };
}

mod r471_r689_e19_place_n_plague_counters_one_prompt_naming_the_single_target {
    use super::*;

    #[test]
    fn r689_places_every_token_on_the_one_permanent_the_single_prompt_names_over_every_permanent_on_either_side_face_down_included()
     {
        let Board {
            run: mut start,
            mine,
            theirs,
            trap,
        } = board("plague-prompts");
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let mut run = frozen(&start);
        let enemy_before = hero_health(&run.state, PlayerId::P2);

        run = act(&run, play(&book));
        let first = run.state.pending.clone();
        assert_eq!(first.as_ref().map(|p| p.player_id), Some(PlayerId::P1));
        assert_eq!(first.as_ref().map(|p| p.kind), Some(PromptKind::Target));
        // R68's order, the placer's side first: p1's unit, then p2's unit, then p2's face-down trap.
        assert_eq!(
            first.map(|p| p
                .options
                .into_iter()
                .map(|option| option.selection)
                .collect::<Vec<_>>()),
            Some(vec![pick(&mine), pick(&theirs), pick(&trap)])
        );
        // The rest of the Cry waits for the placements (R113).
        assert_eq!(hero_health(&run.state, PlayerId::P2), enemy_before);

        // One answer puts both placements on the same face-down card: no second prompt opens.
        run = answer(&run, pick(&trap), None);
        // `reduce` works on a copy, so the card is read back out of the state the answer returned.
        let trap_now = run.state.players.p2.backrow[2].clone().expect("the trap");
        assert_eq!(plague_on(&trap_now), 2);
        assert!(run.state.pending.is_none());
        assert_eq!(hero_health(&run.state, PlayerId::P2), enemy_before - 1);

        let events = last_events(&run.state, 2);
        assert_eq!(
            placements(&events),
            vec![placement(&trap.id, 1, Some(1)), placement(&trap.id, 2, Some(1))]
        );
    }

    #[test]
    fn r689_a_radiant_faces_larger_count_lands_on_the_one_pick_and_the_paused_prompt_survives_json_and_replays()
     {
        let Board {
            run: mut start,
            theirs,
            ..
        } = board("plague-json");
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        find_instance_mut(&mut start.state, &book.id)
            .expect("the book")
            .radiant = true;
        let mut run = frozen(&start);
        run = act(&run, play(&book));

        // Paused on the one prompt, the Spell's damage parked behind it (R113).
        let paused = run.state.clone();
        let round = thaw(&paused);
        assert_eq!(round, paused);
        let data = round
            .pending
            .as_ref()
            .map(|p| serde_json::to_value(&p.resume.data).expect("data serialises"));
        assert!(matches_object(
            &data.unwrap_or(Value::Null),
            &json!({ "count": 3, "amount": 1 })
        ));

        let live_run = answer(&run, pick(&theirs), None);
        let from_json = answer(
            &Run {
                start: run.start.clone(),
                log: run.log.clone(),
                state: round,
            },
            pick(&theirs),
            None,
        );
        assert_eq!(hash_state(&from_json.state), hash_state(&live_run.state));
        assert_eq!(hash_state(&replayed(&live_run)), hash_state(&live_run.state));
        assert_eq!(top_of(&live_run.state, PlayerId::P2, 2).map(plague_on), Some(3));
        assert_eq!(top_of(&live_run.state, PlayerId::P1, 1).map(plague_on), Some(0));
    }

    #[test]
    fn r471_with_no_permanent_on_the_field_nothing_is_asked_and_the_rest_of_the_list_resolves_at_once() {
        let mut start = playing("plague-empty");
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let mut run = frozen(&start);
        let before = hero_health(&run.state, PlayerId::P2);
        let cursor = run.state.rng_cursor;
        run = act(&run, play(&book));
        assert!(run.state.pending.is_none());
        assert_eq!(hero_health(&run.state, PlayerId::P2), before - 1);
        // R129: a placement with nowhere to go draws nothing.
        assert_eq!(run.state.rng_cursor, cursor);
    }

    #[test]
    fn r471_an_answer_that_names_no_offered_permanent_is_refused_and_the_prompt_stays() {
        let Board {
            run: mut start, mine, ..
        } = board("plague-refused");
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let in_hand = hand_card(&mut start.state, &body.id, PlayerId::P1);
        let mut run = frozen(&start);
        run = act(&run, play(&book));
        let pending = run.state.pending.clone().expect("expected a prompt");
        let refused = run.state.clone();
        let named = refusal(
            &run,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [pick(&in_hand)], "playerId": "p1" }),
        );
        assert!(named.is_some_and(|error| error.contains("not one of the options")));
        let wrong_seat = refusal(
            &run,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [pick(&mine)], "playerId": "p2" }),
        );
        assert!(wrong_seat.is_some_and(|error| error.contains("other player")));
        assert_eq!(run.state, refused);
    }

    #[test]
    fn r471_the_other_player_sees_only_that_a_prompt_is_open_the_placer_sees_an_enemy_face_down_card_by_id_alone()
     {
        let Board {
            run: mut start, trap, ..
        } = board("plague-view");
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let mut run = frozen(&start);
        run = act(&run, play(&book));

        assert_eq!(
            view_json(&run.state, PlayerId::P2)["pending"],
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        let mine = view_json(&run.state, PlayerId::P1)["pending"].clone();
        assert_eq!(mine["forYou"], json!(true), "expected p1's own prompt");
        let trap_option = mine["options"]
            .as_array()
            .and_then(|options| {
                options
                    .iter()
                    .find(|option| option["instanceId"] == trap.id.as_str())
            })
            .cloned();
        assert_eq!(
            trap_option,
            Some(
                json!({ "key": format!("instance:{}", trap.id), "label": HIDDEN_OPTION_LABEL, "instanceId": trap.id })
            )
        );
        assert!(!mine.to_string().contains(&quiet_trap.id));

        // The one answer lands both placements on the trap (R689).
        run = answer(&run, pick(&trap), None);
        // The event names the trap to its controller and hides it from the placer (R97); the count on the
        // card's back is public to both (R471).
        let placer_events: Vec<Value> = view_json(&run.state, PlayerId::P1)["events"]
            .as_array()
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e["type"] == "counterChanged")
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let owner_events: Vec<Value> = view_json(&run.state, PlayerId::P2)["events"]
            .as_array()
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e["type"] == "counterChanged")
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        assert!(matches_object(
            placer_events.last().unwrap_or(&Value::Null),
            &json!({ "instanceId": HIDDEN_ID, "value": 2, "placed": 1 })
        ));
        assert!(matches_object(
            owner_events.last().unwrap_or(&Value::Null),
            &json!({ "instanceId": trap.id, "value": 2, "placed": 1 })
        ));
        assert_eq!(
            view_json(&run.state, PlayerId::P1)["opponent"]["backrow"][2],
            json!({ "faceDown": true, "cost": 1, "plague": 2 })
        );
        let owner_zone = view_json(&run.state, PlayerId::P2)["you"]["backrow"][2].clone();
        assert!(matches_object(
            &owner_zone,
            &json!({ "faceDown": false, "counters": { "plague": 2 } })
        ));
    }
}

mod e19_a_placement_prompt_held_by_the_player_who_is_not_taking_the_turn {
    use super::*;

    #[test]
    fn r471_the_placer_is_the_placing_cards_controller_whose_prompt_it_is_on_either_players_turn_c_90s_reward_d()
     {
        let Board {
            run: mut start,
            theirs,
            ..
        } = board("plague-off-turn");
        {
            // p2's card places on p1's turn: the prompt is p2's, and p1 cannot answer it.
            sink_for!(sink, start.state);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(PlayerId::P2),
                    ..Default::default()
                },
            );
            (place_plague_tokens(json_as(json!({ "count": 1, "amount": 2 }))).apply)(&mut ctx);
        }
        assert_eq!(start.state.active, PlayerId::P1);
        assert_eq!(
            start.state.pending.as_ref().map(|p| p.player_id),
            Some(PlayerId::P2)
        );
        let mut run1 = frozen(&start);
        let pending = run1.state.pending.clone().expect("a prompt");
        let refused = refusal(
            &run1,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [pick(&theirs)], "playerId": "p1" }),
        );
        assert!(refused.is_some_and(|error| error.contains("other player")));
        run1 = answer(&run1, pick(&theirs), Some(PlayerId::P2));
        assert!(run1.state.pending.is_none());
        assert_eq!(top_of(&run1.state, PlayerId::P2, 2).map(plague_on), Some(2));
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));
    }
}

mod r471_e19_one_placement_multipliers_and_the_placed_trigger {
    use super::*;

    #[test]
    fn r471_place_n_on_x_is_one_placement_of_n_on_the_declared_card_radiant_2() {
        let Board {
            run: mut start,
            theirs,
            ..
        } = board("plague-outbreak");
        let spell = hand_card(&mut start.state, &outbreak.id, PlayerId::P1);
        let radiant = hand_card(&mut start.state, &outbreak.id, PlayerId::P1);
        find_instance_mut(&mut start.state, &radiant.id)
            .expect("the radiant spell")
            .radiant = true;
        let mut run = frozen(&start);
        run = act(&run, play_on(&spell, &theirs));
        run = act(&run, play_on(&radiant, &theirs));
        assert!(run.state.pending.is_none());
        let events = last_events(&run.state, 2);
        assert_eq!(
            placements(&events),
            vec![
                placement(&theirs.id, 1, Some(1)),
                placement(&theirs.id, 3, Some(2))
            ]
        );
    }

    #[test]
    fn r471_a_placement_multiplier_on_the_card_receiving_it_doubled_tripled_on_its_radiant_face() {
        let Board { run: mut start, .. } = board("plague-slime");
        let state = &mut start.state;
        let base = put(state, &slime.id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let radiant = put(
            state,
            &slime.id,
            slot(PlayerId::P1, Row::Units, 4),
            json!({ "radiant": true }),
        );
        sink_for!(sink, *state);
        assert_eq!(
            plague_multiplier_of(&*sink.state, live(&*sink.state, &base.id)),
            2
        );
        assert_eq!(
            plague_multiplier_of(&*sink.state, live(&*sink.state, &radiant.id)),
            3
        );

        run(
            &mut sink,
            place_plague(json_as(json!({ "target": { "of": "chosen" }, "amount": 1 }))),
            None,
            &[&base],
        );
        run(
            &mut sink,
            place_plague(json_as(json!({ "amount": 2 }))),
            Some(&radiant),
            &[],
        );
        assert_eq!(plague_on(live(&*sink.state, &base.id)), 2);
        assert_eq!(plague_on(live(&*sink.state, &radiant.id)), 6);
        assert_eq!(
            placements(&*sink.events),
            vec![
                placement(&base.id, 2, Some(2)),
                placement(&radiant.id, 6, Some(6))
            ]
        );

        // A Vanilla slime carries no text, so nothing multiplies (§6.3 Vanilla).
        find_instance_mut(&mut *sink.state, &base.id)
            .expect("the slime")
            .vanilla = true;
        assert_eq!(
            plague_multiplier_of(&*sink.state, live(&*sink.state, &base.id)),
            1
        );
    }

    #[test]
    fn r471_a_fused_cards_multipliers_multiply_a_slime_fused_onto_a_slime_quadruples() {
        let Board { run: mut start, .. } = board("plague-fused-slime");
        let state = &mut start.state;
        let kept = put(state, &slime.id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let other = put(state, &slime.id, slot(PlayerId::P1, Row::Units, 4), json!({}));
        sink_for!(sink, *state);
        let fused = fuse(
            &mut sink,
            FuseArgs {
                ingredients: vec![other.clone()],
                target: Some(kept.clone()),
                ..Default::default()
            },
        )
        .expect("expected a fusion");
        let fused_now = live(&*sink.state, &fused.id).clone();
        assert_eq!(plague_multiplier_of(&*sink.state, &fused_now), 4);
        assert_eq!(place_plague_on(&mut sink, &fused_now, 1), 4);
    }

    #[test]
    fn r471_whenever_plague_counters_are_placed_on_this_answers_once_per_placement_never_a_removal() {
        let Board { run: mut start, .. } = board("plague-crawler");
        let worm = put(
            &mut start.state,
            &crawler.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let spell = hand_card(&mut start.state, &outbreak.id, PlayerId::P1);
        find_instance_mut(&mut start.state, &spell.id)
            .expect("the spell")
            .radiant = true;
        let mut run = frozen(&start);
        let before = hero_health(&run.state, PlayerId::P2);

        // One placement of 2: one answer.
        run = act(&run, play_on(&spell, &worm));
        assert_eq!(hero_health(&run.state, PlayerId::P2), before - 1);

        // Two placements on it: one answer (R689); all three triggers still wait for the whole effect and
        // the Spell (R59, R68), so the health lands where two answers put it.
        run = act(&run, play(&book));
        run = answer(&run, pick(&worm), None);
        assert!(run.state.pending.is_none());
        assert_eq!(hero_health(&run.state, PlayerId::P2), before - 1 - 1 - 2);

        // Taking tokens off is no placement.
        let live_worm = top_of(&run.state, PlayerId::P1, 3).cloned().expect("the crawler");
        sink_for!(sink, run.state);
        assert_eq!(remove_plague(&mut sink, &live_worm, 1), 1);
        assert_eq!(placements(&*sink.events), vec![placement(&live_worm.id, 3, None)]);
    }

    #[test]
    fn r471_every_gain_is_a_placement_core_91s_plus_1_plague_counter_plague_reports_placed_and_is_multiplied()
    {
        let Board { run: mut start, .. } = board("plague-gain");
        let on_slime = put(
            &mut start.state,
            &slime.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        sink_for!(sink, start.state);
        run(
            &mut sink,
            counters::plague(json_as(json!({ "amount": 1 }))),
            Some(&on_slime),
            &[],
        );
        run(
            &mut sink,
            counters::plague(json_as(json!({ "amount": -1 }))),
            Some(&on_slime),
            &[],
        );
        assert_eq!(
            placements(&*sink.events),
            vec![
                placement(&on_slime.id, 2, Some(2)),
                placement(&on_slime.id, 1, None)
            ]
        );
    }

    #[test]
    fn r471_r13_a_placement_lands_only_on_a_permanent_on_the_field_never_a_hand_card_or_one_dormant_under_a_stack()
     {
        let Board {
            run: mut start, mine, ..
        } = board("plague-off-field");
        let state = &mut start.state;
        let in_hand = hand_card(state, &body.id, PlayerId::P1);
        // A Stack card played on p1's lane 1, so `mine` lies dormant under it.
        let mut top = new_instance(
            state,
            &big_body.id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        assert!(
            place_on_field(
                state,
                &mut top,
                slot(PlayerId::P1, Row::Units, 1),
                PlaceOnFieldOptions { stack: Some(true) }
            ),
            "expected a pile"
        );
        let mine_now = live(state, &mine.id).clone();
        sink_for!(sink, *state);
        assert_eq!(place_plague_on(&mut sink, &in_hand, 1), 0);
        assert_eq!(place_plague_on(&mut sink, &mine_now, 1), 0);
        assert_eq!(place_plague_on(&mut sink, &top, 0), 0);
        assert!(sink.events.is_empty());
        assert!(
            !permanents_on_field(&*sink.state, None::<PlayerId>)
                .iter()
                .any(|card| card.id == mine.id)
        );
    }
}

mod e19_placements_over_a_scope_at_random_and_removals {
    use super::*;

    #[test]
    fn r471_on_each_permanent_is_one_placement_on_each_both_sides_face_down_included_classic_63() {
        let Board {
            run: mut start,
            mine,
            theirs,
            trap,
        } = board("plague-each");
        let spell = hand_card(&mut start.state, &dusting.id, PlayerId::P1);
        let mut run = frozen(&start);
        run = act(&run, play(&spell));
        let events = last_events(&run.state, 1);
        assert_eq!(
            placements(&events),
            vec![
                placement(&mine.id, 1, Some(1)),
                placement(&theirs.id, 1, Some(1)),
                placement(&trap.id, 1, Some(1)),
            ]
        );
        assert_eq!(plague_on_field(&run.state, None::<PlayerId>), 3);
        assert_eq!(plague_on_field(&run.state, Some(PlayerId::P2)), 2);
    }

    #[test]
    fn r60_a_random_placement_on_n_units_picks_n_different_ones_all_of_them_when_fewer_and_nothing_on_an_empty_board()
     {
        let Board {
            run: mut start,
            mine,
            theirs,
            ..
        } = board("plague-random");
        let third = put(
            &mut start.state,
            &body.id,
            slot(PlayerId::P2, Row::Units, 4),
            json!({}),
        );
        {
            sink_for!(sink, start.state);
            run(
                &mut sink,
                place_plague_random(json_as(json!({ "count": 2, "amount": 1 }))),
                None,
                &[],
            );
            let hit: Vec<String> = placements(&*sink.events)
                .iter()
                .map(|entry| entry["id"].as_str().unwrap_or_default().to_string())
                .collect();
            assert_eq!(hit.len(), 2);
            assert_eq!(hit.iter().collect::<BTreeSet<_>>().len(), 2);
            assert!(
                hit.iter()
                    .all(|id| [&mine.id, &theirs.id, &third.id].contains(&id))
            );
        }

        let mut lone = playing("plague-random-lone");
        let only = put(
            &mut lone.state,
            &body.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        {
            sink_for!(lone_sink, lone.state);
            run(
                &mut lone_sink,
                place_plague_random(json_as(json!({ "count": 2, "amount": 1 }))),
                None,
                &[],
            );
            assert_eq!(
                placements(&*lone_sink.events),
                vec![placement(&only.id, 1, Some(1))]
            );
        }

        let mut empty = playing("plague-random-empty");
        sink_for!(empty_sink, empty.state);
        let cursor = empty_sink.rng.cursor();
        run(
            &mut empty_sink,
            place_plague_random(json_as(json!({ "count": 2, "amount": 1 }))),
            None,
            &[],
        );
        assert_eq!(empty_sink.rng.cursor(), cursor);
        assert!(empty_sink.events.is_empty());
    }

    #[test]
    fn r471_a_random_placement_played_as_a_spell_replays_the_same_picks() {
        let Board { run: mut start, .. } = board("plague-random-replay");
        put(
            &mut start.state,
            &body.id,
            slot(PlayerId::P2, Row::Units, 4),
            json!({}),
        );
        let spell = hand_card(&mut start.state, &scatter.id, PlayerId::P1);
        let mut run = frozen(&start);
        run = act(&run, play(&spell));
        assert_eq!(hash_state(&replayed(&run)), hash_state(&run.state));
    }

    #[test]
    fn r471_consume_plague_takes_tokens_off_floors_at_0_and_reports_no_placement_the_payment_helper_says_how_many_came_off()
     {
        let Board {
            run: mut start,
            theirs,
            ..
        } = board("plague-consume");
        sink_for!(sink, start.state);
        place_plague_on(&mut sink, &theirs, 3);
        run(
            &mut sink,
            consume_plague(json_as(json!({ "target": { "of": "chosen" } }))),
            None,
            &[&theirs],
        );
        assert_eq!(plague_on(live(&*sink.state, &theirs.id)), 2);
        let now = live(&*sink.state, &theirs.id).clone();
        assert_eq!(remove_plague(&mut sink, &now, 5), 2);
        assert_eq!(live(&*sink.state, &theirs.id).counters.plague, None);
        let now = live(&*sink.state, &theirs.id).clone();
        assert_eq!(remove_plague(&mut sink, &now, 1), 0);
        assert_eq!(
            placements(&*sink.events),
            vec![
                placement(&theirs.id, 3, Some(3)),
                placement(&theirs.id, 2, None),
                placement(&theirs.id, 0, None),
            ]
        );
    }
}

mod e19_stats_per_token_auras_and_self_layers {
    use super::*;

    #[test]
    fn r471_r689_an_aura_reading_each_units_tokens_buffs_its_side_and_shrinks_the_other_both_placements_land_on_the_one_pick()
     {
        let Board {
            run: mut start,
            mine,
            theirs,
            ..
        } = board("plague-aura");
        put(
            &mut start.state,
            &toxins.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        put(
            &mut start.state,
            &body.id,
            slot(PlayerId::P2, Row::Units, 4),
            json!({}),
        ); // 1/1
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let mut run = frozen(&start);

        run = act(&run, play(&book));
        run = answer(&run, pick(&mine), None);
        assert!(run.state.pending.is_none());
        let ally = top_of(&run.state, PlayerId::P1, 1).expect("the ally");
        let view = unit_view(&run.state, ally);
        assert_eq!((view.attack, view.max_health), (3, 3));
        let enemy = top_of(&run.state, PlayerId::P2, 2).expect("the enemy");
        assert_eq!(theirs.id, enemy.id);
        let view = unit_view(&run.state, enemy);
        assert_eq!((view.attack, view.max_health), (3, 5));
    }

    #[test]
    fn r689_a_minus_x_minus_x_from_the_placements_kills_at_the_state_check_after_the_whole_effect() {
        let Board { run: mut start, .. } = board("plague-aura-death");
        put(
            &mut start.state,
            &toxins.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let frail = put(
            &mut start.state,
            &body.id,
            slot(PlayerId::P2, Row::Units, 4),
            json!({}),
        ); // 1/1
        let book = hand_card(&mut start.state, &plague_book.id, PlayerId::P1);
        let mut run = frozen(&start);

        run = act(&run, play(&book));
        run = answer(&run, pick(&frail), None);
        // Both placements landed before any state check, so the 1/1 is gone only now that the effect ended.
        assert!(run.state.pending.is_none());
        assert!(run.state.players.p2.units[3].is_none());
        assert_eq!(
            of_type(&last_events(&run.state, 1), "destroyed")
                .iter()
                .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>(),
            vec![frail.id.clone()]
        );
    }

    #[test]
    fn r471_a_self_layer_reads_the_cards_own_tokens_classic_69_plus_2_attack_per_token() {
        let Board { run: mut start, .. } = board("plague-self-layer");
        let unit = put(
            &mut start.state,
            &charger.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        assert_eq!(unit_view(&start.state, live(&start.state, &unit.id)).attack, 4);
        {
            sink_for!(sink, start.state);
            place_plague_on(&mut sink, &unit, 2);
        }
        assert_eq!(unit_view(&start.state, live(&start.state, &unit.id)).attack, 8);
    }
}

mod e19_tokens_spent_as_mana_the_removal_the_play_pipeline_pays_with {
    use super::*;

    #[test]
    fn r471_remove_plague_is_the_one_way_tokens_come_off_a_card_and_place_plague_on_the_one_way_they_go_on() {
        let Board { run: mut start, .. } = board("plague-payment");
        let field = put(
            &mut start.state,
            &toxins.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        sink_for!(sink, start.state);
        run(
            &mut sink,
            place_plague_each(json_as(
                json!({ "scope": { "side": "self", "rows": ["backrow"] }, "amount": 4 }),
            )),
            None,
            &[],
        );
        assert_eq!(plague_on(live(&*sink.state, &field.id)), 4);
        let now = live(&*sink.state, &field.id).clone();
        assert_eq!(remove_plague(&mut sink, &now, 3), 3);
        assert_eq!(plague_on(live(&*sink.state, &field.id)), 1);
    }

    #[test]
    fn r471_a_placement_prompt_effect_with_a_count_below_1_asks_nothing() {
        let Board { run: mut start, .. } = board("plague-zero");
        sink_for!(sink, start.state);
        run(
            &mut sink,
            place_plague_tokens(json_as(json!({ "count": 0 }))),
            None,
            &[],
        );
        assert!(sink.state.pending.is_none());
        assert!(sink.events.is_empty());
    }
}

mod r452_e19_placements_under_a_random_cast_b5_e12 {
    use super::*;

    fn random_cast_of_the_book(target_enemies: bool) -> Effect {
        let how = if target_enemies {
            json!({ "random": true, "targetEnemies": true })
        } else {
            json!({ "random": true })
        };
        cast_new(CastNewArgs {
            def: CastNewDef::from(plague_book.id.clone()),
            radiant: None,
            how: json_as(how),
        })
    }

    #[test]
    fn r452_r471_a_random_cast_places_each_token_on_a_random_permanent_itself_and_asks_nothing() {
        let Board { run: mut start, .. } = board("plague-random-cast");
        sink_for!(sink, start.state);
        run(&mut sink, random_cast_of_the_book(false), None, &[]);
        settle(&mut sink, Default::default());
        assert!(sink.state.pending.is_none());
        assert!(of_type(&*sink.events, "promptOpened").is_empty());
        // Its two placements both landed, and the rest of its text ran after them.
        assert_eq!(placements(&*sink.events).len(), 2);
        assert_eq!(hero_health(&*sink.state, PlayerId::P2), 29);
    }

    #[test]
    fn r452_a_cast_that_targets_enemies_places_on_enemy_permanents_whenever_there_is_one() {
        for seed in ["plague-enemies-1", "plague-enemies-2", "plague-enemies-3"] {
            let Board {
                run: mut start,
                theirs,
                trap,
                ..
            } = board(seed);
            sink_for!(sink, start.state);
            run(&mut sink, random_cast_of_the_book(true), None, &[]);
            settle(&mut sink, Default::default());
            for placed in placements(&*sink.events) {
                assert!(
                    [theirs.id.as_str(), trap.id.as_str()]
                        .contains(&placed["id"].as_str().unwrap_or_default())
                );
            }
        }
    }
}
