//! What survives a prompt (SPEC §9.3: "mid-action choices are state, not callbacks … identical in
//! live play, replays and tests"): any sequence spanning a prompt must resume out of state alone.
//!
//!  1. a card's own effect list: the effects after the one that opened the prompt are parked as a
//!     `WorkItem` naming the same continuation plus the index to continue from;
//!  2. an engine sequence of named steps (the play pipeline of §10.5, the attack window of §4.2
//!     step 4, the Death hooks of §4.5 step 3, the end of a turn of §2.2): only a step that pauses
//!     owes the steps after it.
//!
//! Both go on `state.work`, ordered by R113 with `state.workCursor`.
//! Surface contract: docs/v0.3.0/SURFACE.md §17 on §8 (`testkit::register_work_handler`).

use serde::Serialize;
use serde::de::DeserializeOwned;

use jackioh_engine::effects::{choose_mode, damage};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{events_of_type, new_game, put, sink_for, slot};

// The handlers the engine registers, registered here too

/// A fixture engine sequence: three steps, the middle one of which asks (see `drive_sequence`).
const SEQUENCE_HOOK: &str = "__pausesTestSequence";
/// Where the fixture sequence's cursor sits inside `resume.data`, as a play run does (§10.5).
const AT_KEY: &str = "__at";

#[derive(Clone, Copy)]
struct SequenceRun {
    at: usize,
    owner: PlayerId,
}

fn sequence_resume(run: SequenceRun) -> Resume {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(AT_KEY.to_string(), json!(run.at));
    resume_at(ResumeAtArgs {
        def_id: String::new(),
        hook: Some(SEQUENCE_HOOK.to_string()),
        step: format!("step{}", run.at),
        data: Some(data),
        ..Default::default()
    })
}

fn sequence_run_of(item: &WorkItem) -> SequenceRun {
    let at = item.resume.data.get(AT_KEY).and_then(Value::as_u64);
    SequenceRun {
        at: at.map_or(0, |at| at as usize),
        owner: item.owner,
    }
}

fn mode_option(option: &str) -> PromptOption {
    PromptOption {
        key: format!("mode:{option}"),
        label: option.to_string(),
        selection: Selection::Mode {
            option: option.to_string(),
        },
        cost: None,
        radiant: None,
    }
}

/// A prompt with two options whose answer does nothing: the pause itself is under test.
fn ask(sink: &mut EngineSink<'_>, player: PlayerId) {
    open_prompt(
        sink,
        OpenPromptArgs {
            player,
            kind: PromptKind::Mode,
            aim: None,
            prompt: "Choose one".to_string(),
            options: vec![mode_option("a"), mode_option("b")],
            min: None,
            max: None,
            budget: None,
            owner: None,
            // Nothing is registered under this step: the answer only closes the prompt (§10.6).
            resume: resume_at(ResumeAtArgs {
                def_id: String::new(),
                step: "none".to_string(),
                ..Default::default()
            }),
        },
    );
}

/// Hit for 1, ask, hit for 2.
const SEQUENCE_STEPS: usize = 3;

fn sequence_step(sink: &mut EngineSink<'_>, at: usize, run: SequenceRun) {
    match at {
        0 => hit(sink, 1),
        1 => ask(sink, run.owner),
        _ => hit(sink, 2),
    }
}

/// The §10.5 discipline of `play_steps.rs`: owe the rest *only* when a step actually pauses (R113).
/// A nested drain must not find the steps owed; the paused step's own tail has already parked at
/// the cursor, so parking here lands behind it.
fn drive_sequence(sink: &mut EngineSink<'_>, run: SequenceRun) {
    for at in run.at..SEQUENCE_STEPS {
        sequence_step(sink, at, run);
        if !paused(sink) {
            continue;
        }
        if at + 1 < SEQUENCE_STEPS {
            push_work(
                sink,
                sequence_resume(SequenceRun { at: at + 1, ..run }),
                Some(run.owner),
            );
        }
        return;
    }
}

fn sequence_handler(sink: &mut EngineSink<'_>, item: &WorkItem) {
    drive_sequence(sink, sequence_run_of(item));
}

// The cards

fn spread(mut base: Value, extra: Value) -> Value {
    if let (Value::Object(base), Value::Object(extra)) = (&mut base, extra) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    base
}

/// Fixture defs are indexed from 1400.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("pz-{name}"),
            "index": index.to_string(),
            "name": name,
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn enemy_hero(amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn choose(step: &str) -> Effect {
    choose_mode(json_as(json!({ "options": ["a", "b"], "step": step })))
}

/// Three effects, the middle one a prompt: the third is the one a lost tail would drop.
fn middle_asker() -> CardDef {
    def("middle-asker", "Spell", 1401, json!({}))
}

/// The same, with a chain: the answered step also asks in the middle of its own list.
fn chain_asker() -> CardDef {
    def("chain-asker", "Spell", 1402, json!({}))
}

/// A unit whose Cry asks in the middle, so a pause can be nested inside an owed sequence.
fn crier() -> CardDef {
    def(
        "crier",
        "Unit",
        1403,
        json!({
            "base": { "attack": 1, "health": 5, "keywords": [], "text": "asks" },
            "radiant": { "attack": 2, "health": 10, "keywords": [], "text": "asks" },
        }),
    )
}

/// A Spell whose continuation "again" owes itself again (the spinning-sequence test).
fn looper() -> CardDef {
    def("looper", "Spell", 1404, json!({}))
}

fn looper_resume() -> Resume {
    resume_at(ResumeAtArgs {
        def_id: looper().id,
        step: "again".to_string(),
        ..Default::default()
    })
}

fn defs() -> Vec<CardDef> {
    vec![middle_asker(), chain_asker(), crier(), looper()]
}

fn resume_table(steps: Vec<(&'static str, Hook)>) -> IndexMap<&'static str, Hook> {
    steps.into_iter().collect()
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            middle_asker().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![enemy_hero(1), choose("picked"), enemy_hero(2)])),
                resume: resume_table(vec![("picked", hook(|_ctx| vec![enemy_hero(4)]))]),
                ..Script::default()
            }),
        ),
        (
            chain_asker().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![enemy_hero(1), choose("second"), enemy_hero(2)])),
                resume: resume_table(vec![
                    (
                        "second",
                        hook(|_ctx| vec![enemy_hero(3), choose("third"), enemy_hero(4)]),
                    ),
                    ("third", hook(|_ctx| vec![enemy_hero(5)])),
                ]),
                ..Script::default()
            }),
        ),
        (
            crier().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![enemy_hero(8), choose("cried"), enemy_hero(16)])),
                resume: resume_table(vec![("cried", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
        (
            looper().id,
            both(Script {
                resume: resume_table(vec![(
                    "again",
                    hook(|_ctx| {
                        vec![Effect::new("oweAgain", |ctx| {
                            owe(ctx, looper_resume());
                        })]
                    }),
                )]),
                ..Script::default()
            }),
        ),
    ]
}

fn game(seed: &str) -> GameState {
    // The seam is per thread, and each test is its own thread.
    register_work_handler(SEQUENCE_HOOK, sequence_handler);
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for entry in defs() {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// A Spell mid-resolution, as §10.5 step 4 leaves one: its continuation still finds it (R98).
fn resolving(state: &mut GameState, def_id: &str) -> CardInstance {
    let card = new_instance(state, def_id, P1, Zone::Resolving { player: P1 });
    state.players.p1.resolving.push(card.clone());
    card
}

/// The enemy hero takes `amount` in a context of the sink's own (R136: its event window starts at
/// the sink's current end).
fn hit(sink: &mut EngineSink<'_>, amount: i32) {
    let effect = enemy_hero(amount);
    let mut ctx = make_context(
        sink,
        None,
        HookOptions {
            controller: Some(P1),
            ..HookOptions::default()
        },
    );
    (effect.apply)(&mut ctx);
}

/// One action's worth of work, with its own event list, as `reduce` gives each action (§9.3): run
/// `body`, then drain whatever a pause owed.
fn act(state: &mut GameState, body: impl FnOnce(&mut EngineSink<'_>)) -> Vec<GameEvent> {
    let mut sink = sink_for(state);
    body(&mut sink);
    drain_work(&mut sink);
    sink.events.clone()
}

/// The `answer` action of §10.6: close the prompt, re-enter the step it named with the selection,
/// then continue what it interrupted.
fn answer(state: &mut GameState, option: &str) -> Vec<GameEvent> {
    assert!(state.pending.is_some());
    let pending = state.pending.clone().expect("no prompt is open");
    let selection = vec![Selection::Mode {
        option: option.to_string(),
    }];
    act(state, |sink| {
        close_prompt(sink);
        run_resume(
            sink,
            &pending.resume,
            ResumeOptions {
                controller: Some(pending.player_id),
                targets: Some(selection),
                ..Default::default()
            },
        );
    })
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn amounts(events: &[GameEvent]) -> Vec<i64> {
    events_of_type(events, GameEventType::Damage)
        .into_iter()
        .map(|event| json_of(event)["amount"].as_i64().unwrap_or_default())
        .collect()
}

fn enemy_health(state: &GameState) -> i32 {
    state.players.p2.hero.health
}

/// Anything in the state that does not survive a JSON round trip (Rust's state types hold no closure).
fn unserializable<T: Serialize + DeserializeOwned + PartialEq>(value: &T, path: &str) -> Vec<String> {
    let back: Option<T> = serde_json::to_value(value)
        .ok()
        .and_then(|json| serde_json::from_value(json).ok());
    if back.as_ref() == Some(value) {
        Vec::new()
    } else {
        vec![path.to_string()]
    }
}

fn revive(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state revives from its JSON")
}

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

fn panics_with(run: impl FnOnce(), text: &str) {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
    let payload = caught.expect_err("expected a panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_default();
    assert!(
        message.contains(text),
        "panic message {message:?} does not contain {text:?}"
    );
}

// A card's own effect list

mod a_prompt_in_the_middle_of_an_effect_list_9_3_10_6 {
    use super::*;

    #[test]
    fn s9_3_runs_the_third_of_three_effects_after_the_answer_having_parked_it_not_dropped_it() {
        let mut state = game("list-pause");
        let card = resolving(&mut state, &middle_asker().id);

        let played = act(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", HookResumableOptions::default());
        });

        // Paused on the second effect: the first has landed, the third has not, and it is owed.
        assert!(state.pending.is_some());
        assert_eq!(amounts(&played), vec![1]);
        assert_eq!(enemy_health(&state), 29);
        assert_eq!(state.work.len(), 1);
        let owed = state.work.first();
        assert!(matches_object(
            &json_of(owed.map(|item| &item.resume)),
            &json!({ "defId": middle_asker().id, "hook": "cry", "instanceId": card.id })
        ));
        assert_eq!(owed.map(|item| item.owner), Some(P1));
        // The continuation names where to pick up (the effect after the one that asked); closures never enter the state (§9.3).
        assert_eq!(
            owed.and_then(|item| paused_of(&item.resume.data))
                .map(|step| step.from),
            Some(2)
        );

        let answered = answer(&mut state, "a");

        // The answered step ran first (4 from `resume.picked`), then the parked tail (2).
        assert_eq!(amounts(&answered), vec![4, 2]);
        assert_eq!(enemy_health(&state), 23);
        assert!(state.work.is_empty());
        assert!(state.pending.is_none());
    }

    #[test]
    fn s10_6_chains_a_second_prompt_inside_the_answer_and_finishes_both_tails_innermost_first() {
        let mut state = game("list-chain");
        let card = resolving(&mut state, &chain_asker().id);

        let played = act(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", HookResumableOptions::default());
        });
        assert_eq!(amounts(&played), vec![1]);
        assert_eq!(owed_work(&state, None).len(), 1);

        // The answered step asks again mid-list, so two tails are owed. Its pause happened *during* a
        // resumption, so R113 puts it ahead of the Cry's tail (`""` is the Cry hook's own step label).
        let first = answer(&mut state, "a");
        assert_eq!(amounts(&first), vec![3]);
        assert!(state.pending.is_some());
        assert_eq!(state.work.len(), 2);
        let steps: Vec<String> = state.work.iter().map(|item| item.resume.step.clone()).collect();
        assert_eq!(steps, vec!["second", ""]);
        assert_eq!(
            peek_work(&state).map(|item| item.resume.step.clone()),
            Some("second".to_string())
        );

        let second = answer(&mut state, "b");

        // 5 from the last step, then the inner tail's 4, then the outer tail's 2: innermost first.
        assert_eq!(amounts(&second), vec![5, 4, 2]);
        assert_eq!(enemy_health(&state), 30 - (1 + 3 + 5 + 4 + 2));
        assert!(state.work.is_empty());
        assert!(state.pending.is_none());
    }
}

// An engine sequence of named steps

mod a_prompt_in_the_middle_of_an_engine_sequence_10_3_10_5 {
    use super::*;

    #[test]
    fn s9_3_resumes_a_three_step_sequence_at_the_step_after_the_one_that_asked() {
        let mut state = game("sequence-pause");

        let started = act(&mut state, |sink| {
            drive_sequence(sink, SequenceRun { at: 0, owner: P1 });
        });

        // Step 2 asked, so step 3 is owed as a work item naming the sequence and its cursor.
        assert_eq!(amounts(&started), vec![1]);
        assert!(is_owed(&state, SEQUENCE_HOOK));
        assert_eq!(state.work.len(), 1);
        assert_eq!(
            peek_work(&state).and_then(|item| item.resume.data.get(AT_KEY)),
            Some(&json!(2))
        );

        let answered = answer(&mut state, "a");

        assert_eq!(amounts(&answered), vec![2]);
        assert!(state.work.is_empty());
        assert_eq!(enemy_health(&state), 27);
    }

    #[test]
    fn s9_3_finishes_a_pause_nested_inside_an_owed_sequence_before_the_sequence_s_own_tail() {
        let mut state = game("sequence-nested");
        let unit = put(&mut state, &crier().id, slot(P1, Row::Units, 1), json!({}));

        // The sequence pauses first, so its tail is owed while the board does something else.
        act(&mut state, |sink| {
            drive_sequence(sink, SequenceRun { at: 0, owner: P1 });
        });
        assert_eq!(state.work.len(), 1);

        // A trigger fires inside the open prompt's answer (§10.3) and its Cry asks mid-list, so its tail
        // parks while the sequence's is still owed. Closing the prompt is the whole answer (§10.6).
        let answered = act(&mut state, |sink| {
            close_prompt(sink);
            run_hook_resumable(sink, &unit, "cry", HookResumableOptions::default());
        });
        assert_eq!(amounts(&answered), vec![8]);
        assert!(state.pending.is_some());
        // The Cry's pause happened during the answer, so R113 puts its tail in front of the sequence's.
        let hooks: Vec<String> = state.work.iter().map(|item| item.resume.hook.clone()).collect();
        assert_eq!(hooks, vec!["cry", SEQUENCE_HOOK]);

        let last = answer(&mut state, "b");

        // The Cry's tail (16) runs before the sequence's remaining step (2): the newer pause is innermost.
        assert_eq!(amounts(&last), vec![16, 2]);
        assert!(state.work.is_empty());
    }
}

// Serializable, and the same on a replay

mod a_paused_state_is_plain_data_9_3_10_1 {
    use super::*;

    #[test]
    fn s9_3_survives_json_parse_json_stringify_state_with_work_owed_and_answers_the_same_way() {
        let mut state = game("round-trip");
        let card = resolving(&mut state, &middle_asker().id);
        act(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", HookResumableOptions::default());
        });
        assert!(has_work(&state));

        let mut round = revive(&state);
        assert_eq!(round, state);
        assert_eq!(round.work, state.work);
        // Nothing owed is a closure or a class instance, anywhere in the state (§10.1).
        assert!(unserializable(&state, "state").is_empty());
        assert_eq!(hash_state(&round), hash_state(&state));

        // And the revived state answers to the same place: the parked tail still runs.
        let live = answer(&mut state, "a");
        let revived = answer(&mut round, "a");
        assert_eq!(amounts(&revived), amounts(&live));
        assert_eq!(enemy_health(&round), enemy_health(&state));
        assert_eq!(hash_state(&round), hash_state(&state));
        assert!(round.work.is_empty());
    }

    #[test]
    fn s9_3_replaying_the_same_steps_reaches_an_identical_state_hash_queue_included() {
        struct Run {
            paused: String,
            done: String,
            owed: Vec<WorkItem>,
        }
        fn run(seed: &str) -> Run {
            let mut state = game(seed);
            let card = resolving(&mut state, &chain_asker().id);
            act(&mut state, |sink| {
                run_hook_resumable(sink, &card, "cry", HookResumableOptions::default());
            });
            answer(&mut state, "a");
            let owed = owed_work(&state, None);
            let paused_hash = hash_state(&state);
            answer(&mut state, "b");
            Run {
                paused: paused_hash,
                done: hash_state(&state),
                owed,
            }
        }

        let first = run("replay-work");
        let second = run("replay-work");

        assert_eq!(second.paused, first.paused);
        assert_eq!(second.done, first.done);
        // Including the queue itself: ids and `seq`s come from state (R68), so a replay builds the live queue.
        assert_eq!(second.owed, first.owed);
        assert_eq!(first.owed.len(), 2);
        // The queue is inside the hash, so a state with work owed is not the state that finished it.
        assert_ne!(first.paused, first.done);
    }
}

// The queue itself

mod the_work_queue_9_3_r68 {
    use super::*;

    fn queue() -> GameState {
        game("queue")
    }

    fn stub(step: &str) -> Resume {
        resume_at(ResumeAtArgs {
            def_id: String::new(),
            step: step.to_string(),
            ..Default::default()
        })
    }

    #[test]
    fn r68_pushes_in_creation_order_ids_and_seq_from_state_and_takes_the_innermost_first() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        let seq_before = sink.state.next_seq;
        // One pause cascade, parked as R113 says: the scope that noticed the prompt parks first and the
        // scope around it behind it, so taking from the front takes the innermost.
        let inner = push_work(&mut sink, stub("inner"), None);
        let outer = push_work(&mut sink, stub("outer"), Some(P2));

        assert_eq!(
            vec![inner.id.clone(), outer.id.clone()],
            vec![format!("w{seq_before}"), format!("w{}", seq_before + 1)]
        );
        assert_eq!(vec![inner.seq, outer.seq], vec![seq_before, seq_before + 1]);
        assert_eq!(sink.state.next_seq, seq_before + 2);
        // The owner defaults to the active player and is otherwise the one given.
        assert_eq!(inner.owner, P1);
        assert_eq!(outer.owner, P2);

        let ids: Vec<String> = sink.state.work.iter().map(|item| item.id.clone()).collect();
        assert_eq!(ids, vec![inner.id.clone(), outer.id.clone()]);
        assert_eq!(peek_work(sink.state), Some(&inner));
        assert_eq!(take_work(sink.state), Some(inner.clone()));
        assert_eq!(take_work(sink.state), Some(outer.clone()));
        assert_eq!(take_work(sink.state), None);
        assert!(!has_work(sink.state));
    }

    #[test]
    fn s9_3_re_queues_a_whole_item_unchanged_so_a_handler_can_wait_behind_another_one() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        let item = push_work(&mut sink, stub("waits"), None);
        let taken = take_work(sink.state);
        assert_eq!(taken.as_ref(), Some(&item));

        owe(&mut sink, item.clone());
        assert_eq!(sink.state.work, vec![item.clone()]);
        // Same id and same `seq`: re-queueing is not a new piece of work (R68's order is kept).
        assert_eq!(
            sink.state.work.first().map(|owed| owed.id.clone()),
            Some(item.id.clone())
        );
        assert_eq!(sink.state.next_seq, item.seq + 1);
    }

    #[test]
    fn s9_3_parks_the_tail_of_a_list_with_the_index_and_selections_it_was_running_with() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert("seen".to_string(), json!(1));
        let plan = WorkPlan::new(
            Resume {
                hook: "cry".to_string(),
                data,
                ..stub("picked")
            },
            P2,
        );
        let step: PausedStep = json_as(json!({
            "from": 3,
            "targets": [{ "pick": "mode", "option": "a" }],
            "modes": ["left"],
        }));
        let item = park_work(&mut sink, &plan, &step);

        assert_eq!(item.owner, P2);
        assert_eq!(item.resume.hook, "cry");
        assert_eq!(
            json_of(paused_of(&item.resume.data)),
            json!({
                "from": 3,
                "targets": [{ "pick": "mode", "option": "a" }],
                "modes": ["left"],
            })
        );
        // The card's own captured data is still its own, beside the control block.
        assert_eq!(item.resume.data.get("seen"), Some(&json!(1)));
        assert!(unserializable(&item, "item").is_empty());
    }

    #[test]
    fn s10_5_drops_the_tail_a_step_parked_once_the_step_has_run_without_asking() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        let kept = push_work(&mut sink, stub("kept"), None);
        let parked = push_work(&mut sink, stub("parked"), None);

        assert_eq!(unpark_work(sink.state, &parked.id), Some(parked.clone()));
        assert_eq!(sink.state.work, vec![kept.clone()]);
        assert_eq!(unpark_work(sink.state, &parked.id), None);

        push_work(
            &mut sink,
            Resume {
                hook: SEQUENCE_HOOK.to_string(),
                ..stub("other")
            },
            None,
        );
        assert_eq!(owed_work(sink.state, Some(SEQUENCE_HOOK)).len(), 1);
        assert_eq!(
            drop_work(sink.state, |item| item.resume.hook == SEQUENCE_HOOK).len(),
            1
        );
        assert!(!is_owed(sink.state, SEQUENCE_HOOK));
        assert_eq!(sink.state.work, vec![kept]);
    }

    #[test]
    fn s9_3_runs_nothing_while_a_prompt_is_open_the_answer_is_another_action() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(AT_KEY.to_string(), json!(2));
        push_work(
            &mut sink,
            Resume {
                hook: SEQUENCE_HOOK.to_string(),
                data,
                ..stub("later")
            },
            None,
        );
        ask(&mut sink, P1);

        assert!(paused(&sink));
        assert!(!run_next_work(&mut sink));
        assert!(!drain_work(&mut sink));
        assert_eq!(sink.state.work.len(), 1);

        // Nor after the game has ended: what was owed stays owed and nothing resolves into a result.
        close_prompt(&mut sink);
        sink.state.result = Some(json_as(json!({ "winner": "p1", "reason": "concede" })));
        assert!(paused(&sink));
        assert!(!run_next_work(&mut sink));
        assert_eq!(sink.state.work.len(), 1);
    }

    #[test]
    fn s9_3_throws_rather_than_drop_work_whose_sequence_registered_no_handler() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        let item = push_work(
            &mut sink,
            Resume {
                hook: "__noSuchSequence".to_string(),
                ..stub("orphan")
            },
            None,
        );
        // No handler registry (SURFACE §6.6): the default arm re-enters a card's own step only when its script has it; an item naming no card raises.
        panics_with(
            || {
                run_work_item(&mut sink, &item);
            },
            "no handler for owed work \"__noSuchSequence\"",
        );
    }

    #[test]
    fn s9_3_stops_a_sequence_that_keeps_owing_work_instead_of_spinning() {
        let mut state = queue();
        let mut sink = sink_for(&mut state);
        // The looper card's continuation owes its own item again (see `scripts()`); the drain must stop.
        push_work(&mut sink, looper_resume(), None);
        panics_with(
            || {
                drain_work(&mut sink);
            },
            &format!("did not drain in {MAX_WORK_STEPS} steps"),
        );
    }
}
