//! Copy the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; SPEC §8.6 row 57, R399,
//! R545–R547), proved on fixture cards (`fixtures/copiedText.ts`): a copier declares and resolves the
//! last Spell's choices and script on the face it was played on, its X and Echo, the continuations of
//! its prompts, its Cast on draw, its preview and glow; the copy is fixed as the play begins; it records
//! the Spell it copied, never itself; and the opponent's view never names it in hand.
//!
//! Port of `packages/engine/test/copied-text.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::draw::draw;
use jackioh_engine::query::last_spell_played;
use jackioh_engine::reduce::{begin_game, legal_actions, reduce};
use jackioh_engine::resolve::{CastOptions, cast_card};
use jackioh_engine::subsystems::copied_text::{
    COPIED_TEXT_KEY, copied_text_of, running_script_of, text_face_of,
};
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::{SettleOptions, settle};
use jackioh_engine::view_for::view_for;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::copied_text::{CT, with_copied_text};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

/// TS's module-level `let nonce = 0`: every action this file sends gets a fresh nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. The events and
/// the rng are kept beside the state, so the test can read the state between calls as TS's shared
/// objects let it.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Sink {
    fn for_state(state: &GameState) -> Sink {
        Sink {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn game(seed: &str, mana: i32) -> GameState {
    let mut ready = begin_game(&with_copied_text(new_game(seed, None))).state;
    for player in [P1, P2] {
        let keep = ids(&ready.players[player].hand);
        ready = act(&ready, player, json!({ "type": "mulligan", "keep": keep })).state;
    }
    for player in [P1, P2] {
        ready.players[player].mana.current = mana;
        ready.players[player].mana.max = mana;
    }
    ready
}

fn attempt(state: &GameState, player: PlayerId, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let body: ActionBody = json_as(body);
    reduce(state, &Action::new(body, player, format!("ct-{nonce}")))
}

fn act(state: &GameState, player: PlayerId, body: Value) -> ReduceResult {
    let result = attempt(state, player, body);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// The refusal text, or nothing when the action was accepted.
fn error_of(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

fn hand(state: &mut GameState, player: PlayerId, def_id: &str, radiant: bool) -> CardInstance {
    let card = in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("no card");
    let held = find_instance_mut(state, &card.id).expect("no card");
    held.radiant = radiant;
    held.clone()
}

fn at_p2() -> Value {
    json!([{ "pick": "hero", "player": "p2" }])
}

/// Play a Spell from p1's hand at p2's hero (a bolt) or with no choices.
fn play_spell(mut state: GameState, def_id: &str, radiant: bool, player: PlayerId) -> GameState {
    let card = hand(&mut state, player, def_id, radiant);
    let mut body = json!({ "type": "play", "instanceId": card.id });
    if def_id == CT.bolt.id {
        let other = if player == P1 { P2 } else { P1 };
        body["targets"] = json!([{ "pick": "hero", "player": other }]);
    }
    act(&state, player, body).state
}

fn hero_hits(events: &[GameEvent], player: PlayerId) -> Vec<i64> {
    let hero = format!("hero-{player}");
    events_of_type(events, GameEventType::Damage)
        .into_iter()
        .map(json_of)
        .filter(|event| event["targetId"] == json!(hero))
        .filter_map(|event| event["amount"].as_i64())
        .collect()
}

fn instance_ids(events: &[GameEvent], ty: GameEventType) -> Vec<Value> {
    events_of_type(events, ty)
        .into_iter()
        .map(|event| json_of(event)["instanceId"].clone())
        .collect()
}

fn plays_of(state: &GameState, player: PlayerId, instance_id: &str) -> Vec<ActionBody> {
    legal_actions(state, player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
        .collect()
}

/// One field of every play, as JSON (`plays.map((play) => play.<key>)`).
fn field_of_plays(plays: &[ActionBody], key: &str) -> Vec<Value> {
    plays.iter().map(|play| json_of(play)[key].clone()).collect()
}

/// p2's unit in `lane`, the top of its pile (`state.players.p2.units[lane - 1]?.[0]`).
fn top_unit(state: &GameState, player: PlayerId, lane: usize) -> Option<&CardInstance> {
    state.players[player]
        .units
        .get(lane - 1)
        .and_then(|pile| pile.as_ref())
        .and_then(|pile| pile.first())
}

/// The hand card a view shows for `instance_id`, when the hand is a list (`Array.isArray(view.you.hand)`).
fn held_in(hand: &Value, instance_id: &str) -> Value {
    hand.as_array()
        .and_then(|cards| cards.iter().find(|held| held["instanceId"] == json!(instance_id)))
        .cloned()
        .unwrap_or(Value::Null)
}

fn pending_id(state: &GameState) -> String {
    state
        .pending
        .as_ref()
        .map(|pending| pending.id.clone())
        .unwrap_or_default()
}

mod e14_a_copier_has_the_last_spells_text_classic_57_echo {
    use super::*;

    #[test]
    fn r399_with_no_spell_played_yet_it_has_no_text_its_play_declares_nothing_resolves_nothing_records_nothing()
     {
        let mut state = game("ct-empty", 9);
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        assert_eq!(json_of(copied_text_of(&state, &echo)), Value::Null);
        assert_eq!(
            json_of(plays_of(&state, P1, &echo.id)),
            json!([{ "type": "play", "instanceId": echo.id }])
        );
        let health = state.players.p2.hero.health;
        let ReduceResult {
            state: after, events, ..
        } = act(&state, P1, json!({ "type": "play", "instanceId": echo.id }));
        assert_eq!(after.players.p2.hero.health, health);
        assert_eq!(events_of_type(&events, GameEventType::CardPlayed).len(), 1);
        assert_eq!(events_of_type(&events, GameEventType::Damage).len(), 0);
        assert_eq!(json_of(last_spell_played(&after)), Value::Null);
        assert!(ids(&after.players.p1.graveyard).contains(&echo.id));
    }

    #[test]
    fn r399_it_declares_the_copied_spells_targets_and_legal_actions_offers_exactly_the_copied_spells() {
        let mut state = play_spell(game("ct-declare", 9), &CT.bolt.id, false, P1);
        put(&mut state, &CT.dummy.id, slot(P2, Row::Units, 1), json!({}));
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        let bolt = hand(&mut state, P1, &CT.bolt.id, false);
        assert_eq!(
            field_of_plays(&plays_of(&state, P1, &echo.id), "targets"),
            field_of_plays(&plays_of(&state, P1, &bolt.id), "targets")
        );
        assert!(plays_of(&state, P1, &echo.id).len() > 1);
        // A play the copied text refuses is refused, by the same rule.
        let refused = attempt(&state, P1, json!({ "type": "play", "instanceId": echo.id }));
        assert!(error_of(&refused).contains("target"));
        let health = state.players.p2.hero.health;
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": echo.id, "targets": at_p2() }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, health - 2);
    }

    #[test]
    fn r399_it_resolves_the_copied_face_a_radiant_bolts_number_through_the_copied_definitions_declared_value_r386()
     {
        let mut radiant = play_spell(game("ct-face-r", 9), &CT.bolt.id, true, P1);
        let echo = hand(&mut radiant, P1, &CT.echo.id, false);
        let before = radiant.players.p2.hero.health;
        let events = act(
            &radiant,
            P1,
            json!({ "type": "play", "instanceId": echo.id, "targets": at_p2() }),
        )
        .events;
        assert_eq!(hero_hits(&events, P2), vec![4]);
        assert_eq!(radiant.players.p2.hero.health, before);

        let mut base = play_spell(game("ct-face-b", 9), &CT.bolt.id, false, P1);
        let echo2 = hand(&mut base, P1, &CT.echo.id, false);
        let base_events = act(
            &base,
            P1,
            json!({ "type": "play", "instanceId": echo2.id, "targets": at_p2() }),
        )
        .events;
        assert_eq!(hero_hits(&base_events, P2), vec![2]);
    }

    #[test]
    fn r399_b5_e4_a_played_copier_records_the_spell_it_copied_on_its_face_never_itself_so_a_second_one_copies_the_same_spell()
     {
        let mut state = play_spell(game("ct-record", 9), &CT.ping.id, true, P1);
        state = play_spell(state, &CT.echo.id, false, P1);
        assert_eq!(
            json_of(last_spell_played(&state)),
            json!({ "defId": CT.ping.id, "radiant": true })
        );
        let second = hand(&mut state, P1, &CT.echo.id, false);
        assert_eq!(
            json_of(copied_text_of(&state, &second)),
            json!({ "defId": CT.ping.id, "radiant": true })
        );
        let ReduceResult {
            state: after, events, ..
        } = act(&state, P1, json!({ "type": "play", "instanceId": second.id }));
        assert_eq!(hero_hits(&events, P2), vec![3]);
        assert_eq!(
            json_of(last_spell_played(&after)),
            json!({ "defId": CT.ping.id, "radiant": true })
        );
    }

    #[test]
    fn r399_the_copied_modes_are_declared_with_the_play_and_resolved() {
        let mut state = game("ct-modes", 9);
        let modal = hand(&mut state, P1, &CT.modal.id, false);
        let mut after = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": modal.id, "modes": ["hit"] }),
        )
        .state;
        let echo = hand(&mut after, P1, &CT.echo.id, false);
        assert_eq!(
            field_of_plays(&plays_of(&after, P1, &echo.id), "modes"),
            vec![json!(["hit"]), json!(["heal"])]
        );
        after.players.p1.hero.health = 20;
        let healed = act(
            &after,
            P1,
            json!({ "type": "play", "instanceId": echo.id, "modes": ["heal"] }),
        )
        .state;
        assert_eq!(healed.players.p1.hero.health, 22);
    }

    #[test]
    fn r545_an_x_cost_text_x_is_chosen_with_the_play_from_1_up_to_the_mana_left_once_the_copiers_own_price_is_paid()
     {
        let mut state = game("ct-x", 4);
        let x_bolt = hand(&mut state, P1, &CT.x_bolt.id, false);
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": x_bolt.id, "x": 1 }),
        )
        .state;
        assert_eq!(state.players.p1.mana.current, 3);
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        // 3 mana: the copier costs 1, so X is 1 or 2.
        assert_eq!(
            field_of_plays(&plays_of(&state, P1, &echo.id), "x"),
            vec![json!(1), json!(2)]
        );
        assert!(
            error_of(&attempt(
                &state,
                P1,
                json!({ "type": "play", "instanceId": echo.id, "x": 3 })
            ))
            .contains("X is above")
        );
        assert!(
            error_of(&attempt(
                &state,
                P1,
                json!({ "type": "play", "instanceId": echo.id })
            ))
            .contains("X must be at least")
        );
        let ReduceResult {
            state: after, events, ..
        } = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": echo.id, "x": 2 }),
        );
        assert_eq!(hero_hits(&events, P2), vec![2]);
        assert_eq!(after.players.p1.mana.current, 2);
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::CardPlayed).first())["costPaid"],
            json!(1)
        );
    }

    #[test]
    fn r545_with_no_mana_left_after_the_copiers_price_an_x_cost_text_cannot_be_played_at_all() {
        let mut state = game("ct-x-none", 2);
        let x_bolt = hand(&mut state, P1, &CT.x_bolt.id, false);
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": x_bolt.id, "x": 1 }),
        )
        .state;
        assert_eq!(state.players.p1.mana.current, 1);
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        assert_eq!(json_of(plays_of(&state, P1, &echo.id)), json!([]));
        assert!(
            error_of(&attempt(
                &state,
                P1,
                json!({ "type": "play", "instanceId": echo.id, "x": 1 })
            ))
            .contains("X is above")
        );
    }

    #[test]
    fn r545_b5_e12_a_cast_copier_with_an_x_cost_text_asks_its_caster_for_x_as_any_cast_x_card() {
        let mut state = game("ct-x-cast", 3);
        let x_bolt = hand(&mut state, P1, &CT.x_bolt.id, false);
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": x_bolt.id, "x": 1 }),
        )
        .state;
        let echo = new_instance(&mut state, &CT.echo.id, P1, Zone::Hand { player: P1 });
        let mut sink = Sink::for_state(&state);
        cast_card(&mut sink.on(&mut state), &echo, CastOptions::default());
        settle(&mut sink.on(&mut state), SettleOptions::default());
        state.rng_cursor = sink.rng.cursor();
        let pending = state.pending.clone();
        assert_eq!(json_of(&pending)["kind"], json!("number"));
        let pending = pending.expect("a prompt");
        // A cast pays nothing: X up to the caster's current mana (2).
        let labels: Vec<String> = pending
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect();
        assert_eq!(labels, vec!["1".to_string(), "2".to_string()]);
        let events = act(
            &state,
            P1,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "mode", "option": "2" }] }),
        )
        .events;
        assert_eq!(hero_hits(&events, P2), vec![2]);
    }

    #[test]
    fn r545_an_embiggen_text_resolves_at_its_base_price_the_copier_pays_its_own_and_offers_no_embiggen_choice()
     {
        let mut state = game("ct-embiggen", 9);
        let bigger = hand(&mut state, P1, &CT.bigger.id, false);
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": bigger.id, "embiggen": true }),
        )
        .state;
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        assert_eq!(
            json_of(plays_of(&state, P1, &echo.id)),
            json!([{ "type": "play", "instanceId": echo.id }])
        );
        assert!(
            error_of(&attempt(
                &state,
                P1,
                json!({ "type": "play", "instanceId": echo.id, "embiggen": true })
            ))
            .contains("embiggen")
        );
        let events = act(&state, P1, json!({ "type": "play", "instanceId": echo.id })).events;
        assert_eq!(hero_hits(&events, P2), vec![1]);
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::CardPlayed).first())["costPaid"],
            json!(1)
        );
    }

    #[test]
    fn r546_a_prompt_in_the_copied_text_resumes_into_the_copied_definition_after_a_json_round_trip_with_its_declared_number()
     {
        let mut state = game("ct-prompt", 9);
        let dummy = put(&mut state, &CT.dummy.id, slot(P2, Row::Units, 1), json!({}));
        let asker = hand(&mut state, P1, &CT.asker.id, true);
        state = act(&state, P1, json!({ "type": "play", "instanceId": asker.id })).state;
        assert_eq!(
            state
                .pending
                .as_ref()
                .map(|pending| pending.resume.def_id.clone()),
            Some(CT.asker.id.clone())
        );
        let choice = pending_id(&state);
        state = act(
            &state,
            P1,
            json!({ "type": "answer", "choiceId": choice, "selection": [{ "pick": "instance", "instanceId": dummy.id }] }),
        )
        .state;
        assert_eq!(top_unit(&state, P2, 1).map(|card| card.damage), Some(6));

        let echo = hand(&mut state, P1, &CT.echo.id, false);
        state = act(&state, P1, json!({ "type": "play", "instanceId": echo.id })).state;
        let pending = state.pending.clone().expect("a prompt");
        assert_eq!(pending.player_id, P1);
        // The continuation names the copied Spell's definition and face, with the copier as its card.
        assert_eq!(pending.resume.def_id, CT.asker.id);
        assert!(pending.resume.radiant);
        assert_eq!(pending.resume.instance_id.as_deref(), Some(echo.id.as_str()));
        let round = round_trip(&state);
        let ReduceResult {
            state: after, events, ..
        } = act(
            &round,
            P1,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "instance", "instanceId": dummy.id }] }),
        );
        let amounts: Vec<Value> = events_of_type(&events, GameEventType::Damage)
            .into_iter()
            .map(|event| json_of(event)["amount"].clone())
            .collect();
        assert_eq!(amounts, vec![json!(6)]);
        // 6 + 6 on a 1/9: the copied text's declared number killed it.
        assert_eq!(
            instance_ids(&events, GameEventType::Destroyed),
            vec![json!(dummy.id)]
        );
        assert!(ids(&after.players.p1.graveyard).contains(&echo.id));
    }

    #[test]
    fn r546_the_copy_is_fixed_as_the_play_begins_a_spell_its_own_resolution_casts_changes_neither_it_nor_its_echo_repeat()
     {
        let mut state = play_spell(game("ct-fixed", 9), &CT.caster.id, false, P1);
        assert_eq!(
            json_of(last_spell_played(&state)),
            json!({ "defId": CT.ping.id, "radiant": false })
        );
        // Every caster play leaves the ping it cast as the last Spell, so the record is set to the one a
        // caster that had cast nothing would leave; then a Radiant copier copies it.
        state.last_spell = Some(PlayRecord {
            def_id: CT.caster.id.clone(),
            radiant: false,
        });
        let echo = hand(&mut state, P1, &CT.echo.id, true);
        let ReduceResult {
            state: after, events, ..
        } = act(&state, P1, json!({ "type": "play", "instanceId": echo.id }));
        // Each resolution casts a ping (1) and then deals 5: twice, though the ping is last after the first.
        assert_eq!(hero_hits(&events, P2), vec![1, 5, 1, 5]);
        assert_eq!(
            json_of(last_spell_played(&after)),
            json!({ "defId": CT.ping.id, "radiant": false })
        );
        assert!(
            after
                .players
                .p1
                .graveyard
                .iter()
                .find(|card| card.id == echo.id)
                .and_then(|card| card.memory.get(COPIED_TEXT_KEY))
                .is_none()
        );
    }

    #[test]
    fn r546_radiant_echo_1_repeats_the_copied_text_asking_a_fresh_pick_for_the_repeat() {
        let mut state = play_spell(game("ct-repeat", 9), &CT.bolt.id, false, P1);
        let dummy = put(&mut state, &CT.dummy.id, slot(P2, Row::Units, 1), json!({}));
        let echo = hand(&mut state, P1, &CT.echo.id, true);
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": echo.id, "targets": at_p2() }),
        )
        .state;
        let pending = state.pending.clone().expect("a prompt");
        assert!(pending.prompt.contains("Echo"));
        let round = round_trip(&state);
        let after = act(
            &round,
            P1,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "instance", "instanceId": dummy.id }] }),
        )
        .state;
        assert_eq!(top_unit(&after, P2, 1).map(|card| card.damage), Some(2));
    }

    #[test]
    fn r546_a_copied_faces_own_echo_x_adds_to_the_copiers_own() {
        let mut base = play_spell(game("ct-echo-sum-b", 9), &CT.echo_spell.id, false, P1);
        let echo = hand(&mut base, P1, &CT.echo.id, false);
        assert_eq!(
            hero_hits(
                &act(&base, P1, json!({ "type": "play", "instanceId": echo.id })).events,
                P2
            ),
            vec![1, 1]
        );
        let mut radiant = play_spell(game("ct-echo-sum-r", 9), &CT.echo_spell.id, false, P1);
        let echo_r = hand(&mut radiant, P1, &CT.echo.id, true);
        assert_eq!(
            hero_hits(
                &act(&radiant, P1, json!({ "type": "play", "instanceId": echo_r.id })).events,
                P2
            ),
            vec![1, 1, 1]
        );
    }

    #[test]
    fn r399_b5_e1_a_countered_spell_is_never_recorded_so_it_is_not_copied_nor_is_a_countered_copier() {
        let mut state = play_spell(game("ct-countered", 9), &CT.ping.id, false, P1);
        put(&mut state, &CT.counter.id, slot(P1, Row::Backrow, 1), json!({}));
        state = act(&state, P1, json!({ "type": "endTurn" })).state;
        state.players.p2.mana.current = 9;
        let bolt = hand(&mut state, P2, &CT.bolt.id, false);
        state = act(
            &state,
            P2,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [{ "pick": "hero", "player": "p1" }] }),
        )
        .state;
        assert_eq!(
            json_of(last_spell_played(&state)),
            json!({ "defId": CT.ping.id, "radiant": false })
        );
        // The trap is spent; a second trap counters p2's copier, which records nothing either.
        put(&mut state, &CT.counter.id, slot(P1, Row::Backrow, 2), json!({}));
        let echo = hand(&mut state, P2, &CT.echo.id, false);
        assert_eq!(
            json_of(copied_text_of(&state, &echo)),
            json!({ "defId": CT.ping.id, "radiant": false })
        );
        let events = act(&state, P2, json!({ "type": "play", "instanceId": echo.id })).events;
        assert_eq!(events_of_type(&events, GameEventType::Countered).len(), 1);
        assert_eq!(hero_hits(&events, P1), Vec::<i64>::new());
    }

    #[test]
    fn r399_a_field_spell_is_never_the_last_spell() {
        let mut state = play_spell(game("ct-field", 9), &CT.ping.id, false, P1);
        let field = hand(&mut state, P1, &CT.field.id, false);
        state = act(&state, P1, json!({ "type": "play", "instanceId": field.id })).state;
        assert_eq!(
            json_of(last_spell_played(&state)),
            json!({ "defId": CT.ping.id, "radiant": false })
        );
    }

    #[test]
    fn r399_a_cast_copier_copies_the_spell_that_is_last_as_the_cast_begins_r70() {
        let mut state = play_spell(game("ct-cast", 9), &CT.ping.id, true, P1);
        let echo = new_instance(&mut state, &CT.echo.id, P1, Zone::Hand { player: P1 });
        let mut sink = Sink::for_state(&state);
        cast_card(&mut sink.on(&mut state), &echo, CastOptions::default());
        settle(&mut sink.on(&mut state), SettleOptions::default());
        assert_eq!(hero_hits(&sink.events, P2), vec![3]);
        assert_eq!(
            json_of(last_spell_played(&state)),
            json!({ "defId": CT.ping.id, "radiant": true })
        );
    }

    #[test]
    fn r547_a_copier_drawn_while_the_last_spell_casts_on_draw_is_cast_and_the_draw_repeats() {
        let mut state = game("ct-draw-cast", 9);
        let first = new_instance(&mut state, &CT.draw_cast.id, P1, Zone::Hand { player: P1 });
        let mut sink = Sink::for_state(&state);
        cast_card(&mut sink.on(&mut state), &first, CastOptions::default());
        settle(&mut sink.on(&mut state), SettleOptions::default());
        assert_eq!(
            json_of(last_spell_played(&state)),
            json!({ "defId": CT.draw_cast.id, "radiant": false })
        );
        let library = set_library(&mut state, P1, &[CT.echo.id.clone(), CT.ping.id.clone()]);
        let echo = library.first().cloned().expect("the copier on top");
        let next = library.get(1).cloned().expect("the ping beneath");
        let mut drawing = Sink::for_state(&state);
        draw(&mut drawing.on(&mut state), P1, 1);
        settle(&mut drawing.on(&mut state), SettleOptions::default());
        assert_eq!(hero_hits(&drawing.events, P2), vec![1]);
        assert!(ids(&state.players.p1.graveyard).contains(&echo.id));
        assert!(ids(&state.players.p1.hand).contains(&next.id));
    }

    #[test]
    fn r547_a_copier_drawn_while_the_last_spell_does_not_cast_on_draw_goes_to_the_hand() {
        let mut state = play_spell(game("ct-draw-plain", 9), &CT.ping.id, false, P1);
        let library = set_library(&mut state, P1, &[CT.echo.id.clone(), CT.ping.id.clone()]);
        let echo = library.first().cloned().expect("the copier on top");
        let mut drawing = Sink::for_state(&state);
        draw(&mut drawing.on(&mut state), P1, 1);
        settle(&mut drawing.on(&mut state), SettleOptions::default());
        assert!(ids(&state.players.p1.hand).contains(&echo.id));
        assert_eq!(hero_hits(&drawing.events, P2), Vec::<i64>::new());
    }

    #[test]
    fn r547_the_copied_faces_preview_and_yellow_glow_answer_for_the_copier_in_hand_on_that_faces_numbers() {
        let mut state = game("ct-glow", 9);
        let glow = hand(&mut state, P1, &CT.glow.id, true);
        state = act(
            &state,
            P1,
            json!({ "type": "play", "instanceId": glow.id, "targets": at_p2() }),
        )
        .state;
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        assert!(running_script_of(&state, &echo).preview.is_some());
        assert!(matches_object(
            &json_of(text_face_of(&state, &echo)),
            &json!({ "id": echo.id, "defId": CT.glow.id, "radiant": true })
        ));
        let view = json_of(view_for(&state, P1));
        let card = held_in(&view["you"]["hand"], &echo.id);
        assert_eq!(card["preview"], json!([{ "label": "damage", "value": 4 }]));
        assert_eq!(card["conditionActive"], json!(true));
    }

    #[test]
    fn r399_r243_the_owners_hand_view_carries_the_copied_face_and_its_numbers_the_opponents_never_names_the_card()
     {
        let mut empty = game("ct-view-empty", 9);
        let blank = hand(&mut empty, P1, &CT.echo.id, false);
        let blank_view = json_of(view_for(&empty, P1));
        let blank_card = held_in(&blank_view["you"]["hand"], &blank.id);
        assert!(!blank_card.is_null());
        assert!(blank_card.get("copies").is_none());

        let mut state = play_spell(game("ct-view", 9), &CT.bolt.id, true, P1);
        let echo = hand(&mut state, P1, &CT.echo.id, false);
        let own = json_of(view_for(&state, P1));
        let card = held_in(&own["you"]["hand"], &echo.id);
        assert_eq!(card["defId"], json!(CT.echo.id));
        assert_eq!(card["cost"], json!(1));
        assert_eq!(
            card["copies"],
            json!({ "defId": CT.bolt.id, "radiant": true, "params": { "damage": 4 } })
        );
        let theirs = json_of(view_for(&state, P2));
        assert_eq!(
            theirs["opponent"]["hand"],
            json!({ "count": state.players.p1.hand.len() })
        );
        assert!(
            !serde_json::to_string(&theirs)
                .expect("a view serialises")
                .contains(&echo.id)
        );
    }
}
