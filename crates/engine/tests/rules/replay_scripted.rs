// Replay with scripted decks (SPEC §9.2, §9.3; REVIEW B1.2, B8). `replay.test.ts` folds 100 games
// to identical hashes, but its decks are script-less fixture units, so M3's new state queues are
// empty in every fold: the M3 review measured `echoQueue`, `triggerQueue`, `delayed` and the
// dispatch frontier as never reached across all 100 seeds. These decks are scripted — an
// end-of-turn trigger, a Death hook, two traps, a delayed effect, an Echo card and three cards
// that open prompts — so the folds exercise the queues, and the run asserts that they did.
//
// 30 seeds, not 100: a game on these decks runs ~85 actions rather than a vanilla game's handful
// (every prompt is an action of its own), and each seed is played once and then replayed twice —
// once through `fold` for the M1 gate's own check, once step by step to compare every state on the
// way. That is ~2,600 actions and three passes per seed, and it reaches every queue. The run is
// seeded end to end, so what it reaches is fixed rather than sampled: more seeds would add breadth,
// not confidence.
//
// Port of `packages/engine/test/replay-scripted.test.ts`.

use std::sync::OnceLock;

use jackioh_engine::effects::{choose_mode, damage, draw};
use jackioh_engine::subsystems::ai_policy::choose_action;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::setup_catalog;

/// `{ ...into, ...from }` on two JSON objects.
fn spread(into: &mut Value, from: &Value) {
    if let (Some(into), Some(from)) = (into.as_object_mut(), from.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
}

/// TS's module `let nextIndex = 1400`, written out: each def takes the index it had.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    let mut def = json!({
        "id": format!("rs-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (replay)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    spread(&mut def, &extra);
    json_as(def)
}

fn unit(name: &str, index: u32, attack: i32, health: i32, extra: Value) -> CardDef {
    let keywords = extra
        .get("base")
        .and_then(|base| base.get("keywords"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let mut fields = json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    });
    spread(&mut fields, &extra);
    def(name, "Unit", index, fields)
}

/// #13's shape: at your end of turn it pings, so an end-of-turn trigger runs every turn.
fn closer() -> CardDef {
    unit("closer", 1401, 1, 4, json!({}))
}
/// A Death hook (§4.5 step 3), which needs the unit to die in combat first.
fn deathrattle() -> CardDef {
    unit("deathrattle", 1402, 2, 1, json!({}))
}
/// #50's shape: its Cry schedules an effect for your next start of turn (R76), so `delayed` fills.
fn delayer() -> CardDef {
    unit("delayer", 1403, 2, 2, json!({}))
}
/// A Cry that opens a prompt, so a play pauses mid-resolution (§10.6).
fn asker() -> CardDef {
    unit("asker", 1404, 1, 1, json!({}))
}
/// Echo 2 on a Spell whose Cry prompts, so a repeat is left *waiting* in `echoQueue` while a prompt
/// is open. Echo 1 does not reach that state: `playSteps.takeEchoRepeat` drops the entry as it takes
/// the last repeat, so the one repeat is in flight (inside the owed play's own record) rather than
/// in the queue, and every state this run observes has `echoQueue` empty. With two, the first repeat
/// pauses on its fresh prompt while the second is still owed in the queue — which is what §10.5
/// step 6 has to survive a pause for, and what this run must pass through to prove the fold.
fn echo_asker() -> CardDef {
    def("echo-asker", "Spell", 1405, json!({}))
}
/// An ordinary queued trigger (R68).
fn watcher() -> CardDef {
    def("watcher", "Field Spell", 1406, json!({}))
}
/// A queued trigger that prompts, so the triggers behind it stay in `triggerQueue`.
fn ask_watcher() -> CardDef {
    def("ask-watcher", "Field Spell", 1407, json!({}))
}
/// #41's shape: a Trap in the backrow that answers a summon and is consumed (§5.1, R17).
fn snap_trap() -> CardDef {
    def("snap-trap", "Trap", 1408, json!({}))
}
/// #18's shape: a Field Trap that answers the turn end and stays (R62's end-of-turn window).
fn end_trap() -> CardDef {
    def("end-trap", "Field Trap", 1409, json!({}))
}
/// #89's shape: a hand trigger, which answers from the hand rather than the field.
fn corpse() -> CardDef {
    unit("corpse", 1410, 2, 2, json!({}))
}

/// Bodies, so combat happens and units die: the cheapest way to reach the Death hooks.
fn reborn_body() -> CardDef {
    unit(
        "reborn-body",
        1411,
        2,
        2,
        json!({ "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }], "text": "reborn" } }),
    )
}
fn taunter() -> CardDef {
    unit(
        "taunter",
        1412,
        2,
        3,
        json!({ "base": { "attack": 2, "health": 3, "keywords": [{ "kind": "Taunt" }], "text": "taunt" } }),
    )
}
fn shielded() -> CardDef {
    unit(
        "shielded",
        1413,
        2,
        2,
        json!({ "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Divine Shield" }], "text": "shield" } }),
    )
}
fn poisoner() -> CardDef {
    unit(
        "poisoner",
        1414,
        1,
        3,
        json!({ "base": { "attack": 1, "health": 3, "keywords": [{ "kind": "Poisonous" }], "text": "poison" } }),
    )
}
fn rusher() -> CardDef {
    unit(
        "rusher",
        1415,
        2,
        2,
        json!({ "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Rush" }], "text": "rush" } }),
    )
}
fn body_a() -> CardDef {
    unit("body-a", 1416, 3, 2, json!({ "cost": 2 }))
}
fn body_b() -> CardDef {
    unit("body-b", 1417, 2, 4, json!({ "cost": 2 }))
}
fn body_c() -> CardDef {
    unit("body-c", 1418, 4, 3, json!({ "cost": 3 }))
}
/// A plain Spell and a Spell that draws, so the library and fatigue get used too.
fn bolt() -> CardDef {
    def("bolt", "Spell", 1419, json!({}))
}
fn study() -> CardDef {
    def("study", "Spell", 1420, json!({}))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn steps(entries: Vec<(&'static str, Hook)>) -> IndexMap<&'static str, Hook> {
    entries.into_iter().collect()
}

fn ping(amount: i32) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "ping",
            &[GameEventType::CardPlayed],
            move |_ctx, _event| {
                vec![damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": amount }),
                ))]
            },
        )],
        ..Script::default()
    }
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            closer().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            deathrattle().id,
            both(Script {
                death: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            delayer().id,
            both(Script {
                cry: Some(hook(|ctx| {
                    let Some(me) = ctx.self_.clone() else {
                        return vec![];
                    };
                    let controller = ctx.controller;
                    let radiant = ctx.radiant;
                    schedule_delayed(
                        ctx,
                        controller,
                        DelayedAt {
                            phase: Phase::Start,
                            player: controller,
                        },
                        Resume {
                            def_id: "rs-delayer".into(),
                            hook: "delayed".into(),
                            step: "boom".into(),
                            radiant,
                            instance_id: Some(me.id),
                            data: IndexMap::new(),
                        },
                        None,
                        None,
                    );
                    vec![]
                })),
                delayed: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            asker().id,
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["burn", "keep"], "step": "answered" }),
                    ))]
                })),
                resume: steps(vec![(
                    "answered",
                    hook(|ctx| match ctx.targets.first() {
                        Some(Selection::Mode { option }) if option == "burn" => {
                            vec![damage(json_as(
                                json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                            ))]
                        }
                        _ => vec![],
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            echo_asker().id,
            both(Script {
                static_flags: Some(json_as(json!({ "echo": 2 }))),
                cry: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(
                        json!({ "options": ["left", "right"], "step": "answered" }),
                    ))]
                })),
                resume: steps(vec![(
                    "answered",
                    hook(|_ctx| {
                        vec![damage(json_as(
                            json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                        ))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (watcher().id, both(ping(1))),
        (
            ask_watcher().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "ask",
                    &[GameEventType::CardPlayed],
                    |_ctx, _event| {
                        vec![choose_mode(json_as(
                            json!({ "options": ["yes", "no"], "step": "answered" }),
                        ))]
                    },
                )],
                resume: steps(vec![("answered", hook(|_ctx| vec![]))]),
                ..Script::default()
            }),
        ),
        (
            snap_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "snap",
                    &[GameEventType::Summoned],
                    |_ctx, _event| {
                        vec![damage(json_as(
                            json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                        ))]
                    },
                )],
                ..Script::default()
            }),
        ),
        (
            end_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "toll",
                    &[GameEventType::TurnEnded],
                    |_ctx, _event| {
                        vec![damage(json_as(
                            json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                        ))]
                    },
                )],
                ..Script::default()
            }),
        ),
        (
            corpse().id,
            both(Script {
                hand_triggers: vec![TriggerDef::new(
                    "eat",
                    &[GameEventType::Destroyed],
                    |_ctx, _event| {
                        vec![damage(json_as(
                            json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                        ))]
                    },
                )],
                ..Script::default()
            }),
        ),
        (
            bolt().id,
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            study().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![draw(json_as(json!({ "count": 1 })))])),
                ..Script::default()
            }),
        ),
    ]
}

fn defs() -> Vec<CardDef> {
    vec![
        closer(),
        deathrattle(),
        delayer(),
        asker(),
        echo_asker(),
        watcher(),
        ask_watcher(),
        snap_trap(),
        end_trap(),
        corpse(),
        reborn_body(),
        taunter(),
        shielded(),
        poisoner(),
        rusher(),
        body_a(),
        body_b(),
        body_c(),
        bolt(),
        study(),
    ]
}

fn deck() -> Vec<String> {
    defs().into_iter().map(|entry| entry.id).collect()
}

fn decks() -> (Vec<String>, Vec<String>) {
    (deck(), deck())
}

/// The fixture catalog plus these cards; both the live game and the fold need them registered.
fn register_all() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
}

/// How full each state queue ever got, and how many traps fired, over a whole run.
#[derive(Clone, Debug, Default, PartialEq)]
struct Peaks {
    trigger_queue: usize,
    echo_queue: usize,
    delayed: usize,
    dispatch: usize,
    work: usize,
    /// States seen with a card's prompt open — the mulligan does not count (§10.6).
    card_prompts: usize,
    traps_fired: usize,
    actions: usize,
}

fn zero_peaks() -> Peaks {
    Peaks::default()
}

fn observe(peaks: &mut Peaks, state: &GameState, events: &[GameEvent]) {
    peaks.trigger_queue = peaks.trigger_queue.max(state.trigger_queue.len());
    peaks.echo_queue = peaks.echo_queue.max(state.echo_queue.len());
    peaks.delayed = peaks.delayed.max(state.delayed.len());
    peaks.dispatch = peaks.dispatch.max(state.dispatch.len());
    peaks.work = peaks.work.max(state.work.len());
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.kind != PromptKind::Mulligan)
    {
        peaks.card_prompts += 1;
    }
    peaks.traps_fired += events
        .iter()
        .filter(|event| event.event_type() == GameEventType::TrapFired)
        .count();
}

fn merge(into: &mut Peaks, from: &Peaks) {
    into.trigger_queue = into.trigger_queue.max(from.trigger_queue);
    into.echo_queue = into.echo_queue.max(from.echo_queue);
    into.delayed = into.delayed.max(from.delayed);
    into.dispatch = into.dispatch.max(from.dispatch);
    into.work = into.work.max(from.work);
    into.card_prompts += from.card_prompts;
    into.traps_fired += from.traps_fired;
    into.actions += from.actions;
}

const STEP_CAP: usize = 4000;

/// Every state a game passed through, boiled down to what a divergence would show up in.
struct Walk {
    state: GameState,
    /// The state hash after each action, so a divergence names the action it started at.
    hashes: Vec<String>,
    peaks: Peaks,
}

fn start_state(seed: &str) -> GameState {
    begin_game(&create_game(&CreateGameOptions {
        seed: seed.into(),
        decks: decks(),
        ..Default::default()
    }))
    .state
}

/// One game by the random policy of §10.7 (`chooseAction`, so the run draws from exactly the set
/// R84 names), walking every state it passes through on the way. TS's `Walk & { log }`.
fn play_scripted_game(seed: &str) -> (Walk, Vec<Action>) {
    register_all();
    let mut state = start_state(seed);
    let mut policy = create_rng(&format!("policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    let mut peaks = zero_peaks();
    let mut hashes = vec![hash_state(&state)];
    observe(&mut peaks, &state, &[]);

    let mut step = 0;
    while state.result.is_none() {
        if step > STEP_CAP {
            panic!("scripted game {seed} did not finish");
        }
        let player = seat_to_act(&state).expect("a seat to act");
        let Some(chosen) = choose_action(&state, player, &mut policy) else {
            panic!("no legal action for {player} in game {seed}");
        };

        let action = Action::new(chosen, player, format!("a{}", log.len()));
        let ReduceResult {
            state: next,
            events,
            error,
            ..
        } = reduce(&state, &action);
        if let Some(error) = error {
            panic!("{} rejected in {seed}: {error}", action.action_type());
        }
        log.push(action);
        state = next;
        hashes.push(hash_state(&state));
        observe(&mut peaks, &state, &events);
        step += 1;
    }

    peaks.actions = log.len();
    (Walk { state, hashes, peaks }, log)
}

/// The same log again from the seed, action by action, watching the same queues: `fold` returns only
/// the final state, and a queue that filled and emptied in between would leave no trace in it.
fn refold_walking(seed: &str, log: &[Action]) -> Walk {
    register_all();
    let mut state = start_state(seed);
    let mut peaks = zero_peaks();
    let mut hashes = vec![hash_state(&state)];
    observe(&mut peaks, &state, &[]);

    for action in log {
        let ReduceResult {
            state: next,
            events,
            error,
            ..
        } = reduce(&state, action);
        if let Some(error) = error {
            panic!("{} rejected refolding {seed}: {error}", action.action_type());
        }
        state = next;
        hashes.push(hash_state(&state));
        observe(&mut peaks, &state, &events);
    }

    peaks.actions = log.len();
    Walk { state, hashes, peaks }
}

struct SeedRun {
    seed: String,
    live_hash: String,
    replay_hash: String,
    /// The fold's `{ nonce, error }` rejects.
    errors: Vec<(String, String)>,
    live_result: Option<GameResult>,
    replay_result: Option<GameResult>,
    live_hashes: Vec<String>,
    replay_hashes: Vec<String>,
    live_peaks: Peaks,
    /// What the replay passed through, which must be what the live game passed through.
    replay_peaks: Peaks,
}

fn seeds() -> Vec<String> {
    (0..30).map(|i| format!("scripted-{}", i + 1)).collect()
}

struct Run {
    runs: Vec<SeedRun>,
    live: Peaks,
    replayed: Peaks,
}

/// The whole run, computed once: the tests read it, in either order. (Each Rust test runs on its own
/// thread with its own registry override; `run` registers the cards itself on whichever thread
/// computes it, and the result is plain data.)
static CACHED: OnceLock<Run> = OnceLock::new();

fn run() -> &'static Run {
    CACHED.get_or_init(|| {
        let mut runs = Vec::new();
        let mut live = zero_peaks();
        let mut replayed = zero_peaks();

        for seed in seeds() {
            let (played, log) = play_scripted_game(&seed);
            merge(&mut live, &played.peaks);

            // Fold the recorded log from the seed in a fresh state (the M1 gate's own check), and then
            // walk the same log again to compare every state on the way, queues included.
            register_all();
            let folded = fold(&FoldArgs {
                seed: seed.clone(),
                decks: decks(),
                log: log.clone(),
                ..Default::default()
            });
            let walked = refold_walking(&seed, &log);
            merge(&mut replayed, &walked.peaks);

            runs.push(SeedRun {
                live_hash: hash_state(&played.state),
                replay_hash: hash_state(&folded.state),
                errors: folded
                    .errors
                    .iter()
                    .map(|reject| (reject.nonce.clone(), reject.error.clone()))
                    .collect(),
                live_result: played.state.result,
                replay_result: folded.state.result,
                live_hashes: played.hashes,
                replay_hashes: walked.hashes,
                live_peaks: played.peaks,
                replay_peaks: walked.peaks,
                seed,
            });
        }

        Run { runs, live, replayed }
    })
}

mod replay_with_scripted_decks_9_2_9_3 {
    use super::*;

    #[test]
    fn folds_30_scripted_games_to_the_same_state_hash() {
        let runs = &run().runs;
        assert_eq!(runs.len(), seeds().len());
        // §2.6: the scripted deck is a legal deck, which is why there are exactly this many cards.
        assert_eq!(deck().len() as i32, DECK_SIZE);

        for seed in runs {
            assert!(seed.errors.is_empty(), "{}: {:?}", seed.seed, seed.errors);
            assert_eq!(seed.replay_hash, seed.live_hash, "{}", seed.seed);
            assert_eq!(seed.replay_result, seed.live_result, "{}", seed.seed);
            assert!(seed.live_result.is_some(), "{}", seed.seed);
            // Not just the same ending: the same state after every single action, so a queue that
            // filled and emptied in between cannot have differed either.
            assert_eq!(seed.replay_hashes, seed.live_hashes, "{}", seed.seed);
        }
    }

    #[test]
    fn section_10_1_exercises_every_state_queue_on_the_way_so_the_folds_prove_something() {
        let Run { live, replayed, runs } = run();

        // This is the gap B-9 recorded: a fold that never touches a queue proves nothing about it.
        assert!(live.trigger_queue > 0, "triggerQueue never held a trigger");
        assert!(live.echo_queue > 0, "echoQueue never held a repeat");
        assert!(live.delayed > 0, "delayed never held an effect");
        assert!(live.dispatch > 0, "the dispatch frontier was never owed an event");
        assert!(live.work > 0, "work never held an interrupted sequence");
        assert!(live.card_prompts > 0, "no card ever opened a prompt");
        assert!(live.traps_fired > 0, "no trap ever fired");
        // And the games were real games, not two actions and a concede.
        assert!(live.actions > seeds().len() * 10);

        // The replay passed through the same depths, seed by seed: the queues are replayed, not
        // merely absent at both ends.
        assert_eq!(replayed, live);
        for seed in runs {
            assert_eq!(seed.replay_peaks, seed.live_peaks, "{}", seed.seed);
        }
    }

    #[test]
    fn section_9_3_a_different_seed_gives_a_different_hash_with_the_same_scripted_decks() {
        let (a, _) = play_scripted_game("scripted-hash-a");
        let (b, _) = play_scripted_game("scripted-hash-b");
        assert_ne!(hash_state(&a.state), hash_state(&b.state));
    }
}
