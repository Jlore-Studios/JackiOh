//! The delayed kinds of patch v0.2.0 (docs/classic-sets.md B5 E27, E28, R458): a destroy at the start
//! of your next turn, aimed at a unit (Classic #20) or read then over a scope (its Radiant face); your
//! hand discarded at the end of this turn or of your *next* turn (Classic #37); `delay`'s `next`; and
//! a start-of-turn effect for the rest of the game (Classic+ #52), which runs in R62's delayed stage
//! among the delayed effects in creation order, once per turn, and stacks.
//!
//! Port of `packages/engine/test/delayed-kinds.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::delay::{DELAYED_DESTROY_HOOK, DELAYED_DISCARD_HAND_HOOK};
use jackioh_engine::effects::discard_hand_at_turn_end;
use jackioh_engine::reduce::{begin_game, reduce};
use jackioh_engine::replay::{fold, hash_state};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::testkit::*;
use jackioh_engine::turn::START_OF_TURN_WORK;
use jackioh_engine::view_for::view_for;
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::zones::{MoveToZoneOptions, OffFieldZone, PlaceOnFieldOptions, move_to_zone, place_on_field};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, setup_catalog, slot};
use crate::rules::fixtures::turn::{
    LOG_LANE, contract, contract_ask, doom, doom_all, hurrah, later, log_card, notes, reminder, turn_catalog,
    TURN_SCRIPTS,
};

fn register() {
    register_catalog(turn_catalog(registered_catalog().clone()));
    let mut scripts = registered_scripts().clone();
    scripts.extend(TURN_SCRIPTS.clone());
    register_scripts(scripts);
}

/// TS's module-level `let nonce = 0`: every action this file sends gets a fresh nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
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

/// `reduce` with a fresh nonce; `body` is the TS `ActionInput` literal (its `playerId` included).
fn act(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("dk{nonce}"));
    let action: Action = json_as(action);
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(&format!("delayed-kinds-{seed}"), None)).state;
    register();
    for player in [P1, P2] {
        let keep = ids(&state.players[player].hand);
        state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player })).state;
    }
    put(&mut state, &log_card().id, slot(P2, Row::Backrow, LOG_LANE), json!({}));
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// Put `def_id` in `player`'s hand and play it, `extra` merged into the action. (TS also handed back
/// the card it put in hand, which no test here reads.)
fn play(state: &mut GameState, player: PlayerId, def_id: &str, extra: Value) -> ReduceResult {
    let card = in_hand(state, def_id, player, 1).into_iter().next().expect("a card to play");
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": player });
    if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            body.insert(key.clone(), value.clone());
        }
    }
    act(state, body)
}

/// End the active player's turn.
fn pass(state: &GameState) -> ReduceResult {
    act(state, json!({ "type": "endTurn", "playerId": state.active }))
}

fn on_field(state: &GameState, id: &str) -> bool {
    [P1, P2].iter().any(|&player| {
        state.players[player]
            .units
            .iter()
            .any(|pile| pile.as_ref().and_then(|pile| pile.first()).is_some_and(|card| card.id == id))
    })
}

fn instance_ids(events: &[GameEvent], ty: GameEventType) -> Vec<Value> {
    events_of_type(events, ty)
        .into_iter()
        .map(|event| json_of(event)["instanceId"].clone())
        .collect()
}

/// The `discarded` events of `owner`'s cards.
fn discarded_of(events: &[GameEvent], owner: PlayerId) -> Vec<Value> {
    events_of_type(events, GameEventType::Discarded)
        .into_iter()
        .map(json_of)
        .filter(|event| event["owner"] == json!(owner))
        .collect()
}

mod b5_e27_a_destroy_at_the_start_of_your_next_turn_r458 {
    use super::*;

    #[test]
    fn r458_the_chosen_unit_is_destroyed_at_the_start_of_its_makers_next_turn_not_the_opponents() {
        let mut state = playing("doom");
        let victim = put(&mut state, "fx-25", slot(P2, Row::Units, 1), json!({}));
        let cast = play(
            &mut state,
            P1,
            &doom().id,
            json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }),
        )
        .state;
        let entries: Vec<Value> =
            cast.delayed.iter().map(|entry| json!([entry.resume.hook, entry.at, entry.watch])).collect();
        assert_eq!(
            entries,
            vec![json!([DELAYED_DESTROY_HOOK, { "phase": "start", "player": "p1" }, victim.id])]
        );

        let theirs = pass(&cast).state;
        assert_eq!(theirs.active, P2);
        assert!(on_field(&theirs, &victim.id));

        let ReduceResult { state: mine, events, .. } = pass(&theirs);
        assert_eq!(mine.active, P1);
        assert!(!on_field(&mine, &victim.id));
        assert!(ids(&mine.players.p2.graveyard).contains(&victim.id));
        assert_eq!(instance_ids(&events, GameEventType::Destroyed), vec![json!(victim.id)]);
        assert!(mine.delayed.is_empty());
    }

    #[test]
    fn r458_it_is_a_destroy_indestructible_ignores_it_r46() {
        let mut state = playing("doom-indestructible");
        let victim = put(&mut state, "fx-25", slot(P2, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &victim.id).expect("the victim").granted_keywords =
            vec![json_as(json!({ "kind": "Indestructible" }))];
        let cast = play(
            &mut state,
            P1,
            &doom().id,
            json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }),
        )
        .state;
        let mine = pass(&pass(&cast).state).state;
        assert!(on_field(&mine, &victim.id));
    }

    #[test]
    fn r458_r174_it_fizzles_once_the_unit_leaves_the_field_even_when_the_unit_comes_back() {
        let mut state = playing("doom-fizzle");
        let victim = put(&mut state, "fx-25", slot(P2, Row::Units, 1), json!({}));
        let mut cast = play(
            &mut state,
            P1,
            &doom().id,
            json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }),
        )
        .state;
        let mut card = cast.players.p2.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .cloned()
            .expect("the victim on the field");
        move_to_zone(&mut cast, &mut card, OffFieldZone::Hand, MoveToZoneOptions::default());
        assert!(cast.delayed.is_empty());
        assert!(place_on_field(&mut cast, &mut card, slot(P2, Row::Units, 1), PlaceOnFieldOptions::default()));
        let mine = pass(&pass(&cast).state).state;
        assert!(on_field(&mine, &victim.id));
    }

    #[test]
    fn r458_all_enemy_units_then_the_ones_standing_as_it_resolves_not_a_list_fixed_when_it_was_made() {
        let mut state = playing("doom-all");
        let early = put(&mut state, "fx-25", slot(P2, Row::Units, 1), json!({}));
        let mine = put(&mut state, "fx-5", slot(P1, Row::Units, 1), json!({}));
        let cast = play(&mut state, P1, &doom_all().id, json!({})).state;
        let entries: Vec<Value> = cast.delayed.iter().map(|entry| json!([entry.resume.hook, entry.watch])).collect();
        assert_eq!(entries, vec![json!([DELAYED_DESTROY_HOOK, null])]);

        let mut theirs = pass(&cast).state;
        let late = put(&mut theirs, "fx-26", slot(P2, Row::Units, 2), json!({}));
        let back = pass(&theirs).state;
        assert!(!on_field(&back, &early.id));
        assert!(!on_field(&back, &late.id));
        assert!(on_field(&back, &mine.id));
    }
}

mod b5_e27_your_hand_discarded_at_the_end_of_this_turn_or_your_next_r458 {
    use super::*;

    #[test]
    fn r458_this_turn_at_the_end_of_the_turn_it_was_made_on_the_makers_hand_is_discarded_the_other_hand_kept() {
        let mut state = playing("hurrah");
        let cast = play(&mut state, P1, &hurrah().id, json!({})).state;
        let held = cast.players.p1.hand.len();
        assert!(held > 0);
        let other = cast.players.p2.hand.len();
        let ReduceResult { state: after, events, .. } = pass(&cast);
        let discarded = discarded_of(&events, P1);
        assert_eq!(discarded.len(), held);
        assert!(after.players.p1.hand.is_empty());
        // p2 drew for their turn; nothing of theirs was discarded.
        assert!(after.players.p2.hand.len() >= other);
    }

    #[test]
    fn r458_your_next_turn_the_end_of_the_turn_it_was_made_on_passes_it_by_the_end_of_your_next_one_discards() {
        let mut state = playing("hurrah-next");
        let card = in_hand(&mut state, &hurrah().id, P1, 1).into_iter().next().expect("the hurrah");
        find_instance_mut(&mut state, &card.id).expect("the hurrah").radiant = true;
        let cast = act(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" })).state;
        let entries: Vec<Value> =
            cast.delayed.iter().map(|entry| json!([entry.resume.hook, entry.not_before])).collect();
        assert_eq!(entries, vec![json!([DELAYED_DISCARD_HAND_HOOK, state.turn + 1])]);

        let after_first = pass(&cast).state;
        assert_eq!(after_first.players.p1.hand.len(), cast.players.p1.hand.len());
        let mine = pass(&after_first).state;
        assert!(!mine.players.p1.hand.is_empty());
        let ReduceResult { state: after, events, .. } = pass(&mine);
        assert!(after.players.p1.hand.is_empty());
        assert!(!discarded_of(&events, P1).is_empty());
    }

    #[test]
    fn r458_made_on_the_opponents_turn_this_turn_is_theirs_and_your_next_turn_is_the_next_one() {
        let state = playing("hurrah-theirs");
        let mut theirs = pass(&state).state;
        assert_eq!(theirs.active, P2);
        {
            let mut events = Vec::new();
            let mut rng = Rng::new(&theirs.seed, theirs.rng_cursor);
            let mut sink = EngineSink::new(&mut theirs, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (discard_hand_at_turn_end(json_as(json!({ "turn": "this" }))).apply)(&mut ctx);
            (discard_hand_at_turn_end(json_as(json!({ "turn": "next" }))).apply)(&mut ctx);
        }
        let entries: Vec<Value> =
            theirs.delayed.iter().map(|entry| json!([entry.owner, entry.at, entry.not_before])).collect();
        assert_eq!(
            entries,
            vec![
                json!(["p1", { "phase": "end", "player": "p2" }, null]),
                json!(["p1", { "phase": "end", "player": "p1" }, theirs.turn + 1]),
            ]
        );
        let held = theirs.players.p1.hand.len();
        let ReduceResult { state: mine, events, .. } = pass(&theirs);
        // Discarded at the end of p2's turn; the one card left is the draw of p1's own turn.
        assert_eq!(discarded_of(&events, P1).len(), held);
        let drawn: Vec<Value> = events_of_type(&events, GameEventType::Drawn)
            .into_iter()
            .map(json_of)
            .filter(|event| event["player"] == json!("p1"))
            .map(|event| event["instanceId"].clone())
            .collect();
        assert_eq!(
            mine.players.p1.hand.iter().map(|card| json!(card.id)).collect::<Vec<_>>(),
            drawn
        );
        let hooks: Vec<String> = mine.delayed.iter().map(|entry| entry.resume.hook.clone()).collect();
        assert_eq!(hooks, vec![DELAYED_DISCARD_HAND_HOOK.to_string()]);
    }

    #[test]
    fn r458_delays_next_a_card_step_at_the_end_of_the_makers_next_turn_not_this_one() {
        let mut state = playing("later");
        let cast = play(&mut state, P1, &later().id, json!({})).state;
        let three = pass(&pass(&cast).state).state;
        assert!(notes(&three).is_empty());
        let four = pass(&three).state;
        assert_eq!(notes(&four), vec![format!("later:{}", state.turn + 2)]);
    }
}

mod b5_e28_for_the_rest_of_the_game_at_the_start_of_your_turn_r458 {
    use super::*;

    fn modifier_labels(view: &Value) -> Vec<Value> {
        view["modifiers"]
            .as_array()
            .map(|mods| mods.iter().map(|entry| entry["label"].clone()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn r458_it_runs_at_each_start_of_its_players_turn_from_the_next_one_as_the_players_effect_and_shows_as_a_badge() {
        let mut state = playing("contract");
        let cast = play(&mut state, P1, &contract().id, json!({})).state;
        let label = json!("At the start of your turn, take a note");
        assert!(modifier_labels(&json_of(view_for(&cast, P1))["you"]).contains(&label));
        assert!(modifier_labels(&json_of(view_for(&cast, P2))["opponent"]).contains(&label));
        assert!(notes(&cast).is_empty());

        let theirs = pass(&cast).state;
        assert!(notes(&theirs).is_empty());
        let three = pass(&theirs).state;
        let five = pass(&pass(&three).state).state;
        assert_eq!(
            notes(&five),
            vec![
                format!("tick:p1:{}:1:no-self", state.turn + 2),
                format!("tick:p1:{}:1:no-self", state.turn + 4),
            ]
        );
    }

    #[test]
    fn r458_several_stack_and_they_run_with_the_delayed_effects_in_creation_order_r68() {
        let mut state = playing("stack");
        let mut next = play(&mut state, P1, &reminder().id, json!({})).state;
        next = play(&mut next, P1, &contract().id, json!({})).state;
        next = play(&mut next, P1, &contract().id, json!({})).state;
        next = play(&mut next, P1, &reminder().id, json!({})).state;
        let turn = pass(&pass(&next).state).state.turn;
        let back = pass(&pass(&next).state).state;
        assert_eq!(
            notes(&back),
            vec![
                "delayed".to_string(),
                format!("tick:p1:{turn}:1:no-self"),
                format!("tick:p1:{turn}:1:no-self"),
                "delayed".to_string(),
            ]
        );
    }

    /// The replayed game's step: `reduce` with a nonce from the log's length, logged once accepted.
    fn step(state: &mut GameState, log: &mut Vec<Action>, body: Value) {
        let mut action = body;
        action["nonce"] = json!(format!("rp{}", log.len()));
        let action: Action = json_as(action);
        let result = reduce(state, &action);
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        log.push(action);
        *state = result.state;
    }

    #[test]
    fn r458_one_that_asks_pauses_the_start_of_the_turn_runs_once_that_turn_and_the_pause_survives_json_and_replay() {
        let seed = "delayed-kinds-replay";
        let mut first_deck = vec![contract_ask().id];
        first_deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let decks = (first_deck, vanilla_deck(DECK_SIZE, 21));
        setup_catalog();
        register();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..Default::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        for player in [P1, P2] {
            let keep = ids(&state.players[player].hand);
            step(&mut state, &mut log, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
        }
        // The note log is not part of an action-built game, so this one reads the prompt and the work.
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == contract_ask().id)
            .cloned()
            .expect("the asking contract in hand");
        step(&mut state, &mut log, json!({ "type": "play", "instanceId": held.id, "playerId": "p1" }));
        step(&mut state, &mut log, json!({ "type": "endTurn", "playerId": "p1" }));
        step(&mut state, &mut log, json!({ "type": "endTurn", "playerId": "p2" }));

        // p1's start of turn 3 is asking, before its draw and main phase.
        assert_eq!(state.active, P1);
        assert_eq!(state.phase, Phase::Start);
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        let hooks: Vec<String> = state.work.iter().map(|item| item.resume.hook.clone()).collect();
        assert_eq!(hooks, vec!["delayed".to_string(), START_OF_TURN_WORK.to_string()]);
        let modifier = state
            .players
            .p1
            .mods
            .iter()
            .map(json_of)
            .find(|entry| entry["kind"] == json!("startOfTurnEffect"));
        assert!(modifier.is_some_and(|entry| matches_object(&entry, &json!({ "ranTurn": state.turn }))));
        let copy: GameState = serde_json::from_value(json_of(&state)).expect("a state survives JSON");
        assert_eq!(copy, state);

        let pending = state.pending.clone().expect("expected a prompt");
        let hand = state.players.p1.hand.len();
        step(
            &mut state,
            &mut log,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": "p1" }),
        );
        assert_eq!(state.phase, Phase::Main);
        assert!(state.pending.is_none());
        // It ran once: the answer finished it and the turn went on to its draw, not back to the effect.
        assert_eq!(state.players.p1.hand.len(), hand + 1);

        let replayed = fold(&json_as(json!({ "seed": seed, "decks": decks, "log": log })));
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&state));
    }
}
