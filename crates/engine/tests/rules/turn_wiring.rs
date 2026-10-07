//! The turn stages patch v0.2.0 adds to R62's order (docs/classic-sets.md B3.1, B3.3; SPEC §2.2): the
//! Brittle tick right after the mana refresh, the "Animated on your turn" cards stepping into their
//! unit zones after it and before the delayed effects, and those cards going home as cleanup's last
//! step. The bodies are `brittle.ts`'s and `animated.ts`'s; what is proved here is the wiring — where
//! each stage runs, that each settles before the next, and that a stage which asks something parks the
//! rest of the turn on `state.work` (R113, R117) so each later stage runs exactly once, through a JSON
//! round trip and a replay.
//!
//! So the two modules are replaced by test doubles for this file only (`vi.mock`): a double that
//! records when it ran, and in some tests asks its player something or emits an event, exactly as a
//! Brittle crumble's Death hook or an arrival's trap would. Their real behaviour is their own tests'.
//!
//! Port of `packages/engine/test/turn-wiring.test.ts`. Rust has no module mocking, so the doubles go
//! in through the testkit's thread-local stage doubles (the seam this file asks part 31 for, in the
//! spirit of SURFACE §8's thread-local registries): `mock_brittle_tick`, `mock_animate_at_turn_start`
//! and `mock_return_at_cleanup` replace `brittle::brittle_tick`, `animated::animate_at_turn_start` and
//! `animated::return_at_cleanup` for the calling thread (each `#[test]` is its own thread, so a double
//! never leaks into another test, as `vi.mock` stays inside its file). Only the three stage bodies are
//! doubled: every other function of the two modules stays the real one. `vi.fn`'s call record is a
//! channel the recording doubles send each call's player down.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;

use jackioh_engine::testkit::*;
use jackioh_engine::testkit::{mock_animate_at_turn_start, mock_brittle_tick, mock_return_at_cleanup};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{in_hand, new_game, put, set_library, setup_catalog, slot};
use crate::rules::fixtures::turn::{
    LOG_LANE, TURN_SCRIPTS, cast_spell, clock, crumble_watcher, log_card, note, notes, reminder,
    turn_catalog, write,
};

/// The log card also answers the doubles' prompts, and holds a delayed step for them.
fn log_scripts() -> CardScripts {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert("brittled", hook(|_ctx| vec![note("brittle:answered")]));
    resume.insert("animated", hook(|_ctx| vec![note("animate:answered")]));
    resume.insert("returned", hook(|_ctx| vec![note("return:answered")]));
    CardScripts {
        base: Script {
            resume,
            delayed: Some(hook(|_ctx| vec![note("late-delayed")])),
            ..Script::default()
        },
        radiant: Script::default(),
    }
}

fn register() {
    register_catalog(turn_catalog(registered_catalog().clone()));
    let mut scripts = registered_scripts().clone();
    scripts.extend(TURN_SCRIPTS.clone());
    scripts.insert(log_card().id, log_scripts());
    register_scripts(scripts);
}

/// A test double's prompt for `player`, whose answer re-enters the log card's `step`. The turn hands
/// every stage its whole sink; the Animated stages declare only the narrower `FieldSink` they use.
fn ask(sink: &mut EngineSink<'_>, player: PlayerId, step: &str) {
    open_prompt(
        sink,
        json_as(json!({
            "player": player,
            "kind": "target",
            "prompt": "a stage asks",
            "options": [{ "key": "none", "label": "nothing", "selection": { "pick": "none" } }],
            "resume": { "defId": log_card().id, "hook": "resume", "step": step, "radiant": false, "data": {} },
        })),
    );
}

/// `vi.mocked(stage).mock.calls`, by the player each call was handed (the two stages a test reads).
struct StageCalls {
    brittle: mpsc::Receiver<PlayerId>,
    returned: mpsc::Receiver<PlayerId>,
}

/// What each stage does unless a test says otherwise: note that it ran, with what it can see. (TS's
/// `beforeEach(recordingDoubles)`: every test calls it first.)
fn recording_doubles() -> StageCalls {
    let (brittle_tx, brittle) = mpsc::channel();
    let (return_tx, returned) = mpsc::channel();
    mock_brittle_tick(move |sink: &mut EngineSink<'_>, player: PlayerId| {
        let _ = brittle_tx.send(player);
        let mana = sink.state.players[player].mana.current;
        write(sink.state, &format!("brittle:{player}:{mana}"));
    });
    mock_animate_at_turn_start(|sink: &mut EngineSink<'_>, player: PlayerId| {
        write(sink.state, &format!("animate:{player}"));
    });
    mock_return_at_cleanup(move |sink: &mut EngineSink<'_>, player: PlayerId| {
        let _ = return_tx.send(player);
        let open = sink.state.players[player].turn_log.unspent_at_end.is_none();
        write(
            sink.state,
            &format!("return:{player}:{}", if open { "open" } else { "closed" }),
        );
    });
    StageCalls { brittle, returned }
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> (GameState, Vec<GameEvent>) {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    let result = reduce(state, &input.with_nonce(format!("tw{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    (result.state, result.events)
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(&format!("turn-wiring-{seed}"), None)).state;
    register();
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = ids(&state.players[player].hand);
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        )
        .0;
    }
    put(
        &mut state,
        &log_card().id,
        slot(PlayerId::P2, Row::Backrow, LOG_LANE),
        Default::default(),
    );
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// p1 has a clock (start- and end-of-turn hooks), a reminder scheduled for the start of their next
/// turn, and a cast-on-draw card on top of their library: every stage of p1's next start leaves a note.
fn staged(seed: &str) -> GameState {
    let mut state = playing(seed);
    put(
        &mut state,
        &clock().id,
        slot(PlayerId::P1, Row::Backrow, 1),
        Default::default(),
    );
    let card = in_hand(&mut state, &reminder().id, PlayerId::P1, 1)[0].clone();
    state = act(
        &state,
        json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
    )
    .0;
    set_library(&mut state, PlayerId::P1, &[cast_spell().id, "fx-9".to_string()]);
    state
}

/// p1 ends turn 1 and p2 turn 2: p1's turn 3 is starting.
fn to_turn_three(state: &GameState) -> (GameState, Vec<GameEvent>) {
    let theirs = act(state, json!({ "type": "endTurn", "playerId": "p1" })).0;
    act(&theirs, json!({ "type": "endTurn", "playerId": "p2" }))
}

fn answer(state: &GameState) -> (GameState, Vec<GameEvent>) {
    let pending = state.pending.as_ref().expect("expected a prompt");
    act(
        state,
        json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": pending.player_id }),
    )
}

fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("serialises")).expect("deserialises")
}

/// The notes from `from` on, so a test reads one turn's stages.
fn since(state: &GameState, from: usize) -> Vec<String> {
    notes(state).into_iter().skip(from).collect()
}

/// The entries that are not p2's (`!entry.includes("p2")`).
fn without_p2(entries: Vec<String>) -> Vec<String> {
    entries
        .into_iter()
        .filter(|entry| !entry.contains("p2"))
        .collect()
}

/// `work.map((item) => [item.resume.hook, item.resume.step])`.
fn work_steps(state: &GameState) -> Vec<(String, String)> {
    state
        .work
        .iter()
        .map(|item| (item.resume.hook.clone(), item.resume.step.clone()))
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
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

mod r62_s_start_of_a_turn_with_the_brittle_and_animated_stages_b3_1_b3_3 {
    use super::*;

    #[test]
    fn r62_refresh_then_the_brittle_tick_then_the_animated_stage_then_delayed_effects_triggers_and_the_draw()
    {
        let calls = recording_doubles();
        let state = staged("order");
        let before = notes(&state).len();
        let (after, _) = to_turn_three(&state);
        let p1: Vec<String> = since(&after, before)
            .into_iter()
            .filter(|entry| !entry.ends_with(":p2") && !entry.contains(":p2:"))
            .collect();
        // The tick sees the refreshed mana (turn 3: two crystals).
        assert_eq!(
            p1,
            vec![
                "end-of-turn:p1",
                "return:p1:closed",
                "brittle:p1:2",
                "animate:p1",
                "delayed",
                "start-of-turn:p1",
                "cast-spell"
            ]
        );
        // And the other player's turn ran the same stages for p2, on p2's turn only.
        assert_eq!(
            since(&after, before)
                .into_iter()
                .filter(|entry| entry.contains("p2"))
                .collect::<Vec<_>>(),
            vec!["brittle:p2:1", "animate:p2", "return:p2:closed"]
        );
        assert!(calls.brittle.try_iter().any(|player| player == PlayerId::P1));
    }

    #[test]
    fn r62_a_prompt_inside_the_brittle_stage_parks_the_rest_of_the_start_the_answer_runs_each_later_stage_once()
     {
        let _calls = recording_doubles();
        let state = staged("brittle-asks");
        mock_brittle_tick(|sink: &mut EngineSink<'_>, player: PlayerId| {
            write(sink.state, &format!("brittle:{player}"));
            if player == PlayerId::P1 && sink.state.turn == 3 {
                ask(sink, player, "brittled");
            }
        });
        let before = notes(&state).len();
        let paused = to_turn_three(&state).0;

        assert_eq!(
            paused.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );
        assert_eq!(paused.phase, Phase::Start);
        assert_eq!(
            work_steps(&paused),
            vec![(START_OF_TURN_WORK.to_string(), "brittle".to_string())]
        );
        assert_eq!(
            without_p2(since(&paused, before)),
            vec!["end-of-turn:p1", "return:p1:closed", "brittle:p1"]
        );

        let copy = round_trip(&paused);
        assert_eq!(copy, paused);
        let live = answer(&paused).0;
        assert_eq!(
            without_p2(since(&live, before)),
            vec![
                "end-of-turn:p1",
                "return:p1:closed",
                "brittle:p1",
                "brittle:answered",
                "animate:p1",
                "delayed",
                "start-of-turn:p1",
                "cast-spell",
            ]
        );
        assert_eq!(live.phase, Phase::Main);
        assert!(live.work.is_empty());
        assert_eq!(hash_state(&answer(&copy).0), hash_state(&live));
    }

    #[test]
    fn r62_a_trigger_the_brittle_stages_own_events_wake_may_ask_too_the_stages_settle_pauses_and_the_rest_waits()
     {
        let _calls = recording_doubles();
        let mut state = staged("brittle-event");
        put(
            &mut state,
            &crumble_watcher().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            Default::default(),
        );
        mock_brittle_tick(|sink: &mut EngineSink<'_>, player: PlayerId| {
            write(sink.state, &format!("brittle:{player}"));
            let Some(card) = sink.state.players[player].hand.first().cloned() else {
                return;
            };
            if player != PlayerId::P1 {
                return;
            }
            sink.events.push(GameEvent::Crumbled {
                instance_id: card.id,
                def_id: card.def_id,
                owner: player,
                zone: ZoneName::Field,
            });
        });
        let before = notes(&state).len();
        let paused = to_turn_three(&state).0;
        assert_eq!(
            paused.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );
        assert_eq!(
            without_p2(since(&paused, before)),
            vec!["end-of-turn:p1", "return:p1:closed", "brittle:p1", "crumble-seen"]
        );
        let last = paused
            .work
            .last()
            .map(|item| serde_json::to_value(&item.resume).expect("serialises"));
        assert!(last.is_some_and(|resume| matches_object(
            &resume,
            &json!({ "hook": START_OF_TURN_WORK, "step": "brittle" })
        )));

        let live = answer(&paused).0;
        assert_eq!(
            without_p2(since(&live, before)),
            vec![
                "end-of-turn:p1",
                "return:p1:closed",
                "brittle:p1",
                "crumble-seen",
                "crumble-answered",
                "animate:p1",
                "delayed",
                "start-of-turn:p1",
                "cast-spell",
            ]
        );
    }

    #[test]
    fn r62_a_prompt_inside_the_animated_stage_parks_the_delayed_effects_the_triggers_and_the_draw() {
        let _calls = recording_doubles();
        let state = staged("animate-asks");
        mock_animate_at_turn_start(|sink: &mut EngineSink<'_>, player: PlayerId| {
            write(sink.state, &format!("animate:{player}"));
            if player == PlayerId::P1 && sink.state.turn == 3 {
                ask(sink, player, "animated");
            }
        });
        let before = notes(&state).len();
        let paused = to_turn_three(&state).0;
        assert_eq!(
            work_steps(&paused),
            vec![(START_OF_TURN_WORK.to_string(), "animate".to_string())]
        );
        let copy = round_trip(&paused);
        let live = answer(&paused).0;
        assert_eq!(
            without_p2(since(&live, before)),
            vec![
                "end-of-turn:p1",
                "return:p1:closed",
                "brittle:p1:2",
                "animate:p1",
                "animate:answered",
                "delayed",
                "start-of-turn:p1",
                "cast-spell",
            ]
        );
        assert_eq!(hash_state(&answer(&copy).0), hash_state(&live));
    }

    #[test]
    fn r62_the_delayed_effects_due_at_a_start_are_the_ones_that_existed_as_the_turn_began_not_one_a_stage_made()
     {
        let _calls = recording_doubles();
        let state = playing("late-delayed");
        mock_brittle_tick(|sink: &mut EngineSink<'_>, player: PlayerId| {
            write(sink.state, &format!("brittle:{player}"));
            if player != PlayerId::P1 || sink.state.turn != 3 {
                return;
            }
            schedule_delayed(
                sink,
                PlayerId::P1,
                DelayedAt {
                    phase: Phase::Start,
                    player: PlayerId::P1,
                },
                Resume {
                    def_id: log_card().id,
                    hook: "delayed".to_string(),
                    step: "late".to_string(),
                    radiant: false,
                    instance_id: None,
                    data: IndexMap::new(),
                },
                None,
                None,
            );
        });
        let three = to_turn_three(&state).0;
        assert!(!notes(&three).contains(&"late-delayed".to_string()));
        assert_eq!(three.delayed.len(), 1);
        let five = to_turn_three(&three).0;
        assert_eq!(
            notes(&five)
                .iter()
                .filter(|entry| *entry == "late-delayed")
                .count(),
            1
        );
        assert!(five.delayed.is_empty());
    }
}

mod r62_s_cleanup_with_the_animated_return_as_its_last_step_b3_1 {
    use super::*;

    #[test]
    fn r62_the_return_runs_after_every_end_of_turn_step_and_cleanups_own_steps_before_the_turn_cap_and_the_next_turn()
     {
        let calls = recording_doubles();
        let mut state = playing("return-order");
        put(
            &mut state,
            &clock().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let (after, events) = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(
            notes(&after)
                .into_iter()
                .filter(|entry| entry.contains("p1"))
                .collect::<Vec<_>>(),
            vec!["end-of-turn:p1", "return:p1:closed"]
        );
        let types: Vec<GameEventType> = events.iter().map(GameEvent::event_type).collect();
        let first_ended = types
            .iter()
            .position(|kind| *kind == GameEventType::TurnEnded)
            .map_or(-1, |at| at as i64);
        let last_started = types
            .iter()
            .rposition(|kind| *kind == GameEventType::TurnStarted)
            .map_or(-1, |at| at as i64);
        assert!(first_ended < last_started);
        assert_eq!(calls.returned.try_iter().last(), Some(PlayerId::P1));
    }

    #[test]
    fn r62_a_prompt_inside_the_return_parks_the_turn_cap_and_the_next_turn_the_answer_starts_the_next_turn_once()
     {
        let _calls = recording_doubles();
        let state = playing("return-asks");
        mock_return_at_cleanup(|sink: &mut EngineSink<'_>, player: PlayerId| {
            write(sink.state, &format!("return:{player}"));
            if player == PlayerId::P1 {
                ask(sink, player, "returned");
            }
        });
        let paused = act(&state, json!({ "type": "endTurn", "playerId": "p1" })).0;
        assert_eq!(paused.active, PlayerId::P1);
        assert_eq!(paused.phase, Phase::End);
        assert_eq!(
            work_steps(&paused),
            vec![(END_OF_TURN_WORK.to_string(), "next".to_string())]
        );
        let copy = round_trip(&paused);
        let (live, events) = answer(&paused);
        assert_eq!(live.active, PlayerId::P2);
        assert_eq!(live.phase, Phase::Main);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type() == GameEventType::TurnStarted)
                .count(),
            1
        );
        assert_eq!(
            notes(&live),
            vec!["return:p1", "return:answered", "brittle:p2:1", "animate:p2"]
        );
        assert_eq!(hash_state(&answer(&copy).0), hash_state(&live));
    }

    /// TS's `step` and `answerOpen` closures over `state` and `log`.
    struct Played {
        state: GameState,
        log: Vec<Action>,
    }

    impl Played {
        fn step(&mut self, body: Value) {
            let input: ActionInput = json_as(body);
            let action = input.with_nonce(format!("rp{}", self.log.len()));
            let result = reduce(&self.state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            self.log.push(action);
            self.state = result.state;
        }

        fn answer_open(&mut self) {
            let pending = self.state.pending.clone().expect("expected a prompt");
            self.step(json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "none" }],
                "playerId": pending.player_id,
            }));
        }
    }

    #[test]
    fn r62_a_game_paused_in_these_stages_replays_from_its_log_to_the_same_state_9_3() {
        let _calls = recording_doubles();
        mock_brittle_tick(|sink: &mut EngineSink<'_>, player: PlayerId| {
            if sink.state.turn == 3 {
                ask(sink, player, "brittled");
            }
        });
        mock_return_at_cleanup(|sink: &mut EngineSink<'_>, player: PlayerId| {
            if sink.state.turn == 2 {
                ask(sink, player, "returned");
            }
        });
        let seed = "turn-wiring-replay";
        let decks = (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21));
        setup_catalog();
        register();
        let mut game = Played {
            state: begin_game(&create_game(&CreateGameOptions {
                seed: seed.to_string(),
                decks: decks.clone(),
                ..Default::default()
            }))
            .state,
            log: vec![],
        };
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep = ids(&game.state.players[player].hand);
            game.step(json!({ "type": "mulligan", "keep": keep, "playerId": player }));
        }
        game.step(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p1" }));
        game.step(json!({ "type": "setAutoEndTurn", "enabled": false, "playerId": "p2" }));
        game.step(json!({ "type": "endTurn", "playerId": "p1" }));
        game.step(json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(
            game.state
                .work
                .iter()
                .map(|item| item.resume.hook.clone())
                .collect::<Vec<_>>(),
            vec![END_OF_TURN_WORK]
        );
        game.answer_open();
        assert_eq!(
            game.state
                .work
                .iter()
                .map(|item| item.resume.hook.clone())
                .collect::<Vec<_>>(),
            vec![START_OF_TURN_WORK]
        );
        game.answer_open();
        assert_eq!(game.state.phase, Phase::Main);
        assert_eq!(game.state.turn, 3);

        let replayed = fold(&FoldArgs {
            seed: seed.to_string(),
            decks,
            log: game.log.clone(),
            ..Default::default()
        });
        assert!(replayed.errors.is_empty());
        assert_eq!(hash_state(&replayed.state), hash_state(&game.state));
    }
}
