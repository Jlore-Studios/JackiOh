//! Ending a turn from an effect (docs/classic-sets.md B5 E10, R456): `endTurn` and `endTurnAfterActions`
//! (`src/effects/turnEnd.ts`), the "your turn ends" rider they leave (`modifiers.cutTurnShort`), and the
//! reducer ending the turn once it is due (`reduce.endDueTurns`).
//!
//! What is pinned: the rest of the effect list and of the action resolve first, then the turn ends as
//! if End turn were pressed, every end-of-turn step included, after `turnCutShort`; a prompt the action
//! opened is answered first; Classic+ #26 drawn at the start of its controller's turn ends that turn
//! before its main phase; "one more action" counts main-phase actions only and not the one that set
//! it; ending the turn yourself uses it; on the other player's turn nothing happens; the AI card Rate
//! Limit ends the opponent's turn after the play that set it off. And the pauses survive JSON and
//! replay.
//!
//! Port of `packages/engine/test/effects-turnEnd.test.ts`.

use std::cell::Cell;

use jackioh_engine::effects::{end_turn, end_turn_after_actions};
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::catalog as catalog_fx;
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, setup_catalog, slot};
use super::fixtures::turn as turn_fx;

fn register() {
    register_catalog(turn_fx::turn_catalog(registered_catalog().clone()));
    let mut registry = registered_scripts().clone();
    registry.extend(turn_fx::scripts());
    register_scripts(registry);
}

fn game(seed: &str) -> GameState {
    let state = new_game(&format!("turn-end-{seed}"), None);
    register();
    state
}

thread_local! {
    /// TS's module-level `let nonce = 0`: every action `act` sends takes the next one. Each Rust test
    /// runs on its own thread, so the count starts at 0 per test; a nonce only has to be unique
    /// within its game (§9.3's dedupe).
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn next_nonce() -> u32 {
    NONCE.with(|nonce| {
        let next = nonce.get() + 1;
        nonce.set(next);
        next
    })
}

/// TS `actResult(state, body)`: the action literal with its nonce, through `reduce`.
fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let mut action = body;
    action["nonce"] = json!(format!("te{}", next_nonce()));
    reduce(state, &json_as::<Action>(action))
}

/// TS `act`'s `{ state, events }`.
struct Acted {
    state: GameState,
    events: Vec<GameEvent>,
}

fn act(state: &GameState, body: Value) -> Acted {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    Acted {
        state: result.state,
        events: result.events,
    }
}

/// Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        )
        .state;
    }
    put(
        &mut state,
        &turn_fx::log_card().id,
        slot(PlayerId::P2, Row::Backrow, turn_fx::LOG_LANE),
        json!({}),
    );
    // R345: nothing but the rule under test ends a turn behind the test's back (R82).
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// TS `play`'s `{ state, events, card }`.
struct Played {
    state: GameState,
    events: Vec<GameEvent>,
    card: CardInstance,
}

/// TS put the card in `state`'s hand (the object the caller holds) and played it from there.
fn play(state: &mut GameState, player: PlayerId, def_id: &str) -> Played {
    let card = in_hand(state, def_id, player, 1).remove(0);
    let Acted { state: after, events } = act(
        state,
        json!({ "type": "play", "instanceId": card.id, "playerId": player }),
    );
    Played {
        state: after,
        events,
        card,
    }
}

fn answer(state: &GameState) -> Acted {
    let pending = state.pending.as_ref().expect("expected a prompt");
    act(
        state,
        json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": pending.player_id }),
    )
}

/// TS `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("serialises")).expect("parses back")
}

fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

/// TS `list.indexOf(x)`: the first position, or -1.
fn index_of(types: &[GameEventType], type_: GameEventType) -> i64 {
    types
        .iter()
        .position(|entry| *entry == type_)
        .map_or(-1, |at| at as i64)
}

/// TS `list.lastIndexOf(x)`: the last position, or -1.
fn last_index_of(types: &[GameEventType], type_: GameEventType) -> i64 {
    types
        .iter()
        .rposition(|entry| *entry == type_)
        .map_or(-1, |at| at as i64)
}

/// `state.players[player].mods.filter((mod) => mod.kind === "turnEnds")`.
fn turn_ends_mods(state: &GameState, player: PlayerId) -> Vec<PlayerModifier> {
    state.players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::TurnEnds { .. }))
        .cloned()
        .collect()
}

/// `turnEndsOf(state, player)` as JSON (the modifier, its kind's fields flat beside `id` and
/// `expiry`), `null` when there is none.
fn turn_ends_json(state: &GameState, player: PlayerId) -> Value {
    to_json(turn_ends_of(state, player))
}

/// TS `makeContext(sinkFor(state), self, { controller })`: the context borrows the state, so the
/// test's effects run inside `f`.
fn with_ctx<R>(
    state: &mut GameState,
    self_: Option<&CardInstance>,
    controller: Option<PlayerId>,
    f: impl FnOnce(&mut EffectContext<'_>) -> R,
) -> R {
    let self_ = self_.cloned();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    let mut ctx = make_context(
        &mut sink,
        self_.as_ref(),
        HookOptions {
            controller,
            ..HookOptions::default()
        },
    );
    f(&mut ctx)
}

/// TS `string[]` for the harness's `setLibrary`, from the ids as written.
fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// TS held the live instance; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// TS wrote through the live instance; Rust writes through the card found by id.
fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn first_cut_in(events: &[GameEvent]) -> Value {
    to_json(
        events
            .iter()
            .find(|event| event.event_type() == GameEventType::TurnCutShort),
    )
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// TS `toMatchObject`: every key `expected` names is in `actual` with a matching value (objects
/// recursively, arrays element by element); `actual` may carry more.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

mod b5_e10_end_your_turn_r456 {
    use super::*;

    #[test]
    fn r456_the_rest_of_the_effect_list_and_of_the_action_resolve_then_the_turn_ends_as_if_end_turn_were_pressed()
     {
        let mut state = playing("cut");
        let Played {
            state: after,
            events,
            card,
        } = play(&mut state, PlayerId::P1, &turn_fx::cutter().id);

        assert_eq!(turn_fx::notes(&after), vec!["after the cut"]);
        assert_eq!(after.active, PlayerId::P2);
        assert_eq!(after.phase, Phase::Main);
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::TurnCutShort)),
            json!([{ "type": "turnCutShort", "player": "p1", "byInstanceId": card.id }])
        );
        let types = types_of(&events);
        // The play resolved whole, then the cut, then every end-of-turn step, then the next turn.
        assert!(
            index_of(&types, GameEventType::CardResolved) < index_of(&types, GameEventType::TurnCutShort)
        );
        assert!(index_of(&types, GameEventType::TurnCutShort) < index_of(&types, GameEventType::TurnEnded));
        assert!(
            index_of(&types, GameEventType::TurnEnded) < last_index_of(&types, GameEventType::TurnStarted)
        );
        // Cleanup ran: the turn log was closed and the rider went with the turn.
        assert_eq!(
            after.players.p1.turn_log.unspent_at_end,
            Some(state.players.p1.mana.current)
        );
        assert!(turn_ends_mods(&after, PlayerId::P1).is_empty());
    }

    #[test]
    fn r456_turn_cut_short_names_its_card_while_the_viewer_may_read_it_and_the_sentinel_once_it_is_in_a_hand_r97()
     {
        let mut state = playing("cut-hidden");
        let Played {
            state: after, card, ..
        } = play(&mut state, PlayerId::P1, &turn_fx::cutter().id);
        let cut_for =
            |viewer: PlayerId, at: &GameState| -> Value { first_cut_in(&view_for(at, viewer).events) };
        assert!(matches_object(
            &cut_for(PlayerId::P2, &after),
            &json!({ "player": "p1", "byInstanceId": card.id }),
        ));
        let mut moved = round_trip(&after);
        let mut landed = moved
            .players
            .p1
            .graveyard
            .iter()
            .find(|held| held.id == card.id)
            .cloned()
            .expect("the cutter in its graveyard");
        move_to_zone(&mut moved, &mut landed, OffFieldZone::Hand, Default::default());
        assert!(matches_object(
            &cut_for(PlayerId::P2, &moved),
            &json!({ "player": "p1", "byInstanceId": HIDDEN_ID }),
        ));
        assert!(matches_object(
            &cut_for(PlayerId::P1, &moved),
            &json!({ "byInstanceId": card.id })
        ));
    }

    #[test]
    fn r456_a_prompt_the_action_opened_is_answered_first_the_paused_game_survives_json_and_replays() {
        // Quickdraw puts the card in p1's opening hand, so the whole game is actions and a replay folds it.
        let seed = "turn-end-replay";
        let decks: (Vec<String>, Vec<String>) = (
            std::iter::once(turn_fx::cut_asker().id)
                .chain(catalog_fx::vanilla_deck(DECK_SIZE - 1, 1))
                .collect(),
            catalog_fx::vanilla_deck(DECK_SIZE, 21),
        );
        setup_catalog();
        register();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut log: Vec<Action> = Vec::new();
        /// TS's `step`: one action, logged under the nonce `rp<log length>`, applied to the state.
        fn step(state: &mut GameState, log: &mut Vec<Action>, body: Value) -> Vec<GameEvent> {
            let mut action = body;
            action["nonce"] = json!(format!("rp{}", log.len()));
            let action: Action = json_as(action);
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            *state = result.state;
            result.events
        }
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            step(
                &mut state,
                &mut log,
                json!({ "type": "mulligan", "keep": keep, "playerId": player }),
            );
        }
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == turn_fx::cut_asker().id)
            .cloned()
            .expect("the cut asker in p1's opening hand");
        step(
            &mut state,
            &mut log,
            json!({ "type": "play", "instanceId": held.id, "playerId": "p1" }),
        );

        // Asked, so the turn has not ended: the rider waits with no actions left.
        assert_eq!(
            state.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );
        assert_eq!(state.active, PlayerId::P1);
        assert!(matches_object(
            &turn_ends_json(&state, PlayerId::P1),
            &json!({ "actionsLeft": 0, "byInstanceId": held.id }),
        ));
        let copy = round_trip(&state);
        assert_eq!(copy, state);

        let pending = state.pending.clone().expect("expected a prompt");
        let actor = seat_to_act(&state).expect("a seat to act");
        let events = step(
            &mut state,
            &mut log,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": actor }),
        );
        assert_eq!(events_of_type(&events, GameEventType::TurnCutShort).len(), 1);
        assert_eq!(state.active, PlayerId::P2);

        assert_eq!(hash_state(&answer(&copy).state), hash_state(&state));
        let replayed = fold(&json_as::<FoldArgs>(
            json!({ "seed": seed, "decks": decks, "log": log }),
        ));
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&state));
    }

    #[test]
    fn r456_on_the_other_players_turn_there_is_no_turn_of_yours_to_end() {
        let mut state = playing("off-turn");
        with_ctx(&mut state, None, Some(PlayerId::P2), |ctx| {
            (end_turn(json_as(json!({}))).apply)(ctx);
            (end_turn_after_actions(json_as(json!({ "actions": 1 }))).apply)(ctx);
            assert!(ctx.state.players.p2.mods.is_empty());
            // "enemy" is the active player's turn, which exists.
            (end_turn(json_as(json!({ "player": "enemy" }))).apply)(ctx);
        });
        assert!(matches_object(
            &turn_ends_json(&state, PlayerId::P1),
            &json!({ "actionsLeft": 0 })
        ));
    }

    #[test]
    fn r456_cast_on_draw_end_your_turn_drawn_at_the_start_of_that_turn_ends_it_before_its_main_phase() {
        let mut state = playing("tempo");
        let tempo_id = turn_fx::tempo().id;
        set_library(&mut state, PlayerId::P2, &owned(&[tempo_id.as_str(), "fx-30"]));
        let drawn = state.players.p2.library.first().cloned();
        let Acted { state: after, events } = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        // p2's turn began, cast the unit it drew, and ended there: it is p1's turn again.
        assert_eq!(after.turn, state.turn + 2);
        assert_eq!(after.active, PlayerId::P1);
        let drawn_id = drawn.as_ref().map(|card| card.id.clone());
        assert!(after.players.p2.units.iter().any(|pile| {
            pile.as_ref()
                .and_then(|pile| pile.first())
                .map(|card| card.id.clone())
                == drawn_id
        }));
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::TurnCutShort)),
            json!([{ "type": "turnCutShort", "player": "p2", "byInstanceId": drawn_id }])
        );
        let p2_ended = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    GameEvent::TurnEnded {
                        player: PlayerId::P2,
                        ..
                    }
                )
            })
            .map_or(-1, |at| at as i64);
        let cut = events
            .iter()
            .position(|event| event.event_type() == GameEventType::TurnCutShort)
            .map_or(-1, |at| at as i64);
        assert!(cut < p2_ended);
    }
}

mod b5_e10_one_more_action_then_your_turn_ends_r456 {
    use super::*;

    #[test]
    fn r456_radiant_drawn_at_the_start_of_the_turn_the_player_takes_one_action_and_the_turn_ends_once_it_resolves()
     {
        let mut state = playing("one-more-drawn");
        let tempo_id = turn_fx::tempo().id;
        set_library(&mut state, PlayerId::P2, &owned(&[tempo_id.as_str(), "fx-30"]));
        let radiant_tempo_id = state.players.p2.library[0].id.clone();
        state.players.p2.library[0].radiant = true;

        let mut started = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert_eq!(started.active, PlayerId::P2);
        assert_eq!(started.phase, Phase::Main);
        assert!(matches_object(
            &turn_ends_json(&started, PlayerId::P2),
            &json!({ "actionsLeft": 1, "byInstanceId": radiant_tempo_id }),
        ));
        // R169: the rider is a badge on both seats.
        let badge = json!({ "label": "Your turn ends after 1 more action" });
        assert!(
            to_json(&view_for(&started, PlayerId::P2).you.modifiers)
                .as_array()
                .is_some_and(|modifiers| modifiers.iter().any(|modifier| matches_object(modifier, &badge)))
        );
        assert!(
            to_json(&view_for(&started, PlayerId::P1).opponent.modifiers)
                .as_array()
                .is_some_and(|modifiers| modifiers.iter().any(|modifier| matches_object(modifier, &badge)))
        );

        let Played {
            state: after, events, ..
        } = play(&mut started, PlayerId::P2, &turn_fx::marker().id);
        assert_eq!(turn_fx::notes(&after), vec!["marker:p2"]);
        assert_eq!(after.active, PlayerId::P1);
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::TurnCutShort)),
            json!([{ "type": "turnCutShort", "player": "p2", "byInstanceId": radiant_tempo_id }])
        );
    }

    #[test]
    fn r456_the_action_that_sets_the_rider_is_not_counted_a_draw_offer_is_not_an_action_and_a_position_switch_is()
     {
        let mut state = playing("count");
        let unit = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let turn = state.turn;
        live_mut(&mut state, &unit.id).summoned_turn = Some(turn);
        let set = play(&mut state, PlayerId::P1, &turn_fx::one_more().id).state;
        assert_eq!(set.active, PlayerId::P1);
        assert_eq!(turn_ends_json(&set, PlayerId::P1)["actionsLeft"], json!(1));

        let offered = act(&set, json!({ "type": "offerDraw", "playerId": "p1" })).state;
        assert_eq!(offered.active, PlayerId::P1);
        assert_eq!(turn_ends_json(&offered, PlayerId::P1)["actionsLeft"], json!(1));

        let Acted { state: after, events } = act(
            &offered,
            json!({ "type": "switchPosition", "instanceId": unit.id, "playerId": "p1" }),
        );
        assert_eq!(after.active, PlayerId::P2);
        assert_eq!(events_of_type(&events, GameEventType::TurnCutShort).len(), 1);
    }

    #[test]
    fn r456_the_last_action_ends_the_turn_once_it_has_resolved_its_prompt_included_and_an_answer_is_no_action_of_its_own()
     {
        let mut state = playing("last-asks");
        let mut set = play(&mut state, PlayerId::P1, &turn_fx::two_more().id).state;
        assert_eq!(turn_ends_json(&set, PlayerId::P1)["actionsLeft"], json!(2));
        let mut first = play(&mut set, PlayerId::P1, &turn_fx::marker().id).state;
        assert_eq!(first.active, PlayerId::P1);
        assert_eq!(turn_ends_json(&first, PlayerId::P1)["actionsLeft"], json!(1));

        let asking = play(&mut first, PlayerId::P1, &turn_fx::questioner().id).state;
        assert_eq!(
            asking.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );
        assert_eq!(asking.active, PlayerId::P1);
        assert_eq!(turn_ends_json(&asking, PlayerId::P1)["actionsLeft"], json!(0));

        let Acted { state: after, events } = answer(&asking);
        assert_eq!(turn_fx::notes(&after), vec!["marker:p1", "questioner:answered"]);
        assert_eq!(after.active, PlayerId::P2);
        assert_eq!(events_of_type(&events, GameEventType::TurnCutShort).len(), 1);
    }

    #[test]
    fn r456_ending_the_turn_yourself_uses_the_actions_up_no_cut_and_the_rider_ends_with_the_turn() {
        let mut state = playing("self-end");
        let set = play(&mut state, PlayerId::P1, &turn_fx::one_more().id).state;
        let Acted {
            state: mut after,
            events,
        } = act(&set, json!({ "type": "endTurn", "playerId": "p1" }));
        assert!(events_of_type(&events, GameEventType::TurnCutShort).is_empty());
        assert_eq!(after.active, PlayerId::P2);
        assert!(turn_ends_mods(&after, PlayerId::P1).is_empty());
        // p2's own actions are p2's: nothing of p1's rider reaches them.
        assert_eq!(
            play(&mut after, PlayerId::P2, &turn_fx::marker().id).state.active,
            PlayerId::P2
        );
    }

    #[test]
    fn r456_with_two_riders_on_one_turn_the_sooner_end_holds_and_names_the_card_that_set_it() {
        let mut state = playing("two-riders");
        let first = put(&mut state, "fx-3", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let second = put(&mut state, "fx-4", slot(PlayerId::P1, Row::Units, 2), json!({}));
        let (first, second) = (live(&state, &first.id), live(&state, &second.id));
        // One sink for all three contexts, as TS's `const sink = sinkFor(state)`.
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            (end_turn_after_actions(json_as(json!({ "actions": 2 }))).apply)(&mut make_context(
                &mut sink,
                Some(&first),
                HookOptions::default(),
            ));
            (end_turn(json_as(json!({}))).apply)(&mut make_context(
                &mut sink,
                Some(&second),
                HookOptions::default(),
            ));
            assert!(matches_object(
                &turn_ends_json(&*sink.state, PlayerId::P1),
                &json!({ "actionsLeft": 0, "byInstanceId": second.id }),
            ));
            (end_turn_after_actions(json_as(json!({ "actions": 1 }))).apply)(&mut make_context(
                &mut sink,
                Some(&first),
                HookOptions::default(),
            ));
        }
        assert!(matches_object(
            &turn_ends_json(&state, PlayerId::P1),
            &json!({ "actionsLeft": 0, "byInstanceId": second.id }),
        ));
        assert_eq!(turn_ends_mods(&state, PlayerId::P1).len(), 1);
    }
}

mod b5_e10_the_opponents_trap_ends_the_turn_r456_the_ai_card_rate_limit {
    use super::*;

    #[test]
    fn r456_the_play_that_set_it_off_resolves_first_then_the_active_players_turn_ends_named_by_the_trap() {
        let mut state = playing("rate-limit");
        let trap = put(
            &mut state,
            &turn_fx::rate_limit().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let Played {
            state: after, events, ..
        } = play(&mut state, PlayerId::P1, &turn_fx::marker().id);

        assert_eq!(turn_fx::notes(&after), vec!["marker:p1"]);
        assert_eq!(after.active, PlayerId::P2);
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::TurnCutShort)),
            json!([{ "type": "turnCutShort", "player": "p1", "byInstanceId": trap.id }])
        );
        let types = types_of(&events);
        assert!(
            index_of(&types, GameEventType::CardResolved) < index_of(&types, GameEventType::TurnCutShort)
        );
        // The trap fired face-up and went to its graveyard: both players may read what cut the turn.
        for viewer in [PlayerId::P1, PlayerId::P2] {
            assert!(matches_object(
                &first_cut_in(&view_for(&after, viewer).events),
                &json!({ "byInstanceId": trap.id }),
            ));
        }
    }
}
