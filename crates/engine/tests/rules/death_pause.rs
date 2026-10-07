//! A prompt opened by a Death hook of §4.5 step 3, and by the Cry of a cast (SPEC §4.5, §9.3,
//! §10.5, §10.6; R59, R64, R78, R83, R89, R113, R117, R122, R127).
//!
//! §4.5 step 3 runs one Death hook per collected card, in R68's order, and §10.5's cast runs a Cry.
//! Both are effect lists, so both can open a prompt, and the governing rule of this engine is that a
//! sequence spanning a prompt must be resumable out of `state.work` alone: never an effect list, a
//! closure, or a remaining-units array held across the pause (§9.3, R113). Both used to go through
//! the non-resumable `resolve.runHook`, so the effects after the one that asked were applied over the
//! open prompt, nothing was parked, and a second dying card's prompt was discarded in silence —
//! `openPrompt` refuses to overwrite one that is already open.
//!
//! What is pinned here, by observable behaviour and not by reading the implementation:
//!
//!   * R113 and R117 — a Death hook that asks stops the pass where it stands and what is left is
//!     *owed*: the rest of that hook's list, the cards after it in R68's order, and steps 4 and 5.
//!     Each tail runs exactly once, in R68's order, and `state.work` is empty again at the end.
//!   * §9.3 and §10.1 — the paused game survives `JSON.parse(JSON.stringify(state))` and resumes
//!     identically from the round-tripped copy, which is what makes a prompt the same thing in live
//!     play, in a replay and in a test.
//!   * R78 and R89 — the resumed half of a Death hook still reads the snapshot taken just before the
//!     card left the field, which the board cannot supply any more: the instance in the graveyard has
//!     been reset. The buff each fixture unit carries is the visible difference.
//!   * R64 and R83 — a Reborn unit collected in a pass that paused still comes back, at 1 health,
//!     without Reborn and summoning sick, once the answer finishes the pass.
//!   * R70 and R122 — a cast whose Cry asks does not land until the answer: §10.5's step 6 and step 7
//!     are owed rather than run over the open prompt, and `cardResolved` is emitted exactly once.
//!
//! The control cases are the other half: with nothing asking, the same hooks run in the same order in
//! one call and `state.work` never holds anything, so a green run here is not green by vacuity.
//!
//! Fixtures are this file's own: defs are prefixed `dp-` and indexed from 2600, so they cannot
//! collide with another test file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/death-pause.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::play_steps::{PLAY_WORK_KIND, run_of};
use jackioh_engine::prompts::{OpenPromptArgs, open_prompt, resume_self};
use jackioh_engine::reduce::{begin_game, reduce};
use jackioh_engine::resolve::{CastOptions, cast_card};
use jackioh_engine::state_check::{DEATHS_WORK, owed_deaths_of, state_check};
use jackioh_engine::testkit::*;
use jackioh_engine::work::owed_work;
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::zones::card_at;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

/// TS's `def`, its running `nextIndex` (from 2600) passed as `index`.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let face = if type_ == "Unit" {
        json!({ "attack": 1, "health": 1, "keywords": [], "text": name })
    } else {
        json!({ "keywords": [], "text": name })
    };
    let mut card = json!({
        "id": format!("dp-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (death pause)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    });
    if let (Some(card), Some(extra)) = (card.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            card.insert(key.clone(), value.clone());
        }
    }
    json_as(card)
}

/// The note sink: a Field Spell parked in p1's backrow lane 5, whose memory records the order.
fn log_card() -> CardDef {
    def("log", 2601, "Field Spell", json!({}))
}
/// Two units whose Death hook asks their controller something before it finishes.
fn ask_one() -> CardDef {
    def("ask-one", 2602, "Unit", json!({}))
}
fn ask_two() -> CardDef {
    def("ask-two", 2603, "Unit", json!({}))
}
/// The same shape without the question, for the control and for R68's far side.
fn quiet_unit() -> CardDef {
    def("quiet", 2604, "Unit", json!({}))
}
/// R64: a unit with Reborn whose Death hook asks, so step 4 is owed behind the pause too.
fn reborn_asker() -> CardDef {
    def(
        "reborn-asker",
        2605,
        "Unit",
        json!({
            "base": { "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }], "text": "reborn asker" },
            "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }], "text": "reborn asker" },
        }),
    )
}
/// A cast Spell whose Cry asks: #95's recast and §2.4's cast on draw both reach this shape (R58).
fn cast_asker() -> CardDef {
    def("cast-asker", 2606, "Spell", json!({}))
}
/// The same cast without the question.
fn cast_quiet() -> CardDef {
    def("cast-quiet", 2607, "Spell", json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![
        log_card(),
        ask_one(),
        ask_two(),
        quiet_unit(),
        reborn_asker(),
        cast_asker(),
        cast_quiet(),
    ]
}

// ---------------------------------------------------------------------------
// The note log: what ran, in the order it ran.
// ---------------------------------------------------------------------------

const NOTE_LANE: usize = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state.players.p1.backrow.get(NOTE_LANE - 1).and_then(|card| card.as_ref())
}

fn write(state: &mut GameState, entry: &str) {
    let Some(Some(log)) = state.players.p1.backrow.get_mut(NOTE_LANE - 1) else {
        return;
    };
    let mut steps: Vec<Value> = log.memory.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
    steps.push(json!(entry));
    log.memory.insert("steps".to_string(), Value::Array(steps));
}

fn notes(state: &GameState) -> Vec<String> {
    log_of(state)
        .and_then(|log| log.memory.get("steps"))
        .and_then(Value::as_array)
        .map(|steps| steps.iter().filter_map(|step| step.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn note(entry: &str) -> Effect {
    let entry = entry.to_string();
    Effect::new("dp:note", move |ctx| write(&mut *ctx.state, &entry))
}

/// R78 and R89: a note carrying what `ctx.self` says the card's attack buff is. On the field and in
/// the snapshot a Death hook reads it is 3; the instance the board holds after the move has been
/// reset to 0, so this is the one visible difference between reading the snapshot and re-deriving
/// `ctx.self` from the board — which is exactly what a resumed continuation cannot do.
fn note_buff(label: &str) -> Effect {
    let label = label.to_string();
    Effect::new("dp:noteBuff", move |ctx| {
        let buff = match &ctx.self_ {
            None => "none".to_string(),
            Some(card) => card.buffs.attack.to_string(),
        };
        write(&mut *ctx.state, &format!("{label}:{buff}"));
    })
}

/// The one answer every fixture prompt offers.
fn nothing() -> PromptOption {
    PromptOption {
        key: "none".to_string(),
        label: "nothing".to_string(),
        selection: Selection::None,
        cost: None,
        radiant: None,
    }
}

/// §10.6: a prompt for the card's own controller, with one answer, so answering is trivial.
fn ask_controller() -> Effect {
    Effect::new("dp:ask", |ctx| {
        let resume = resume_self(ctx, "asked", IndexMap::new());
        let player = ctx.controller;
        let _ = open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the dying card asks its owner".to_string(),
                options: vec![nothing()],
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// The shape under test: note, ask, and a tail that must run once and only after the answer.
fn asking(name: &'static str) -> CardScripts {
    both(Script {
        death: Some(hook(move |_ctx| {
            vec![note(&format!("ask:{name}")), ask_controller(), note_buff(&format!("tail:{name}"))]
        })),
        resume: IndexMap::from([("asked", hook(move |_ctx| vec![note(&format!("answered:{name}"))]))]),
        ..Script::default()
    })
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (ask_one().id, asking("one")),
        (ask_two().id, asking("two")),
        (
            quiet_unit().id,
            both(Script {
                death: Some(hook(|_ctx| vec![note("quiet:before"), note_buff("quiet:tail")])),
                ..Script::default()
            }),
        ),
        (reborn_asker().id, asking("reborn")),
        (
            cast_asker().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![note("cast:ask"), ask_controller(), note_buff("cast:tail")])),
                resume: IndexMap::from([("asked", hook(|_ctx| vec![note("cast:answered")]))]),
                ..Script::default()
            }),
        ),
        (
            cast_quiet().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![note("cast:quiet")])),
                ..Script::default()
            }),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut registered = registered_scripts().clone();
    registered.extend(scripts());
    register_scripts(registered);
    state
}

/// TS's module-level `let nonce = 0`: every action this file sends gets a fresh nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("dp{nonce}"));
    let action: Action = json_as(action);
    reduce(state, &action)
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result.state
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// Past the mulligans, in p1's main phase, with the note log parked in p1's backrow lane 5.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep = ids(&state.players.p2.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    put(&mut state, &log_card().id, slot(P1, Row::Backrow, NOTE_LANE as i32));
    state
}

/// TS `only`: the first item, which the test expects to be there. Takes the owed work as `owed_work`
/// hands it back, owned or borrowed.
fn only_work<T: std::borrow::Borrow<WorkItem>>(items: Vec<T>) -> WorkItem {
    items
        .into_iter()
        .next()
        .map(|item| item.borrow().clone())
        .expect("expected exactly one item")
}

/// Answer the one open prompt, whoever it belongs to.
fn answer(state: &GameState) -> ReduceResult {
    let pending = state.pending.clone().expect("expected a prompt to be open");
    let result = act_result(
        state,
        json!({
            "type": "answer",
            "choiceId": pending.id,
            "selection": [{ "pick": "none" }],
            "playerId": pending.player_id,
        }),
    );
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

/// `expect(JSON.parse(JSON.stringify(value))).toEqual(value)`: plain data, nothing JSON would drop.
fn survives_json<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let back: T = serde_json::from_value(json_of(value)).expect("plain data survives JSON");
    assert_eq!(&back, value);
}

/// Put a unit on the board with the buff R78 will strip as it leaves, and mark it for the check.
/// A destroy mark is §4.5 step 1's other way in, so the fixture needs no damage arithmetic (§6.3).
fn doomed(state: &mut GameState, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let card = put(state, def_id, slot(player, Row::Units, lane));
    let live = find_instance_mut(state, &card.id).expect("the doomed card on the field");
    live.buffs = AttackHealth { attack: 3, health: 0 };
    live.marked_destroyed = Some(true);
    live.clone()
}

fn in_graveyard(state: &GameState, card: &CardInstance) -> bool {
    state.players[card.owner].graveyard.iter().any(|held| held.id == card.id)
}

/// TS `sinkFor(state, events)`: a sink whose rng starts at the state's cursor, as reduce does; the
/// events it gathered come back.
fn with_sink(state: &mut GameState, run: impl FnOnce(&mut EngineSink<'_>)) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    run(&mut EngineSink::new(state, &mut events, &mut rng));
    events
}

fn instance_ids(events: &[GameEvent], ty: GameEventType) -> Vec<String> {
    events_of_type(events, ty)
        .into_iter()
        .filter_map(|event| json_of(event)["instanceId"].as_str().map(str::to_string))
        .collect()
}

/// The ids a parked pass still owes (`owedDeathsOf(resume)?.owed.map((card) => card.id)`).
fn owed_ids(resume: &Resume) -> Option<Vec<String>> {
    owed_deaths_of(resume).map(|pass| ids(&pass.owed))
}

// ---------------------------------------------------------------------------

mod a_prompt_inside_4_5_step_3s_death_hooks_r89_r113_r117_r122 {
    use super::*;

    #[test]
    fn r113_owes_the_rest_of_the_pass_when_a_death_hook_asks_and_both_tails_run_once_in_r68s_order() {
        let mut state = playing("deaths-owe-remainder");
        // R68: the active player's cards first, by lane, then the opponent's.
        let first = doomed(&mut state, &ask_one().id, P1, 1);
        let second = doomed(&mut state, &ask_two().id, P1, 2);
        let far = doomed(&mut state, &quiet_unit().id, P2, 1);

        let events = with_sink(&mut state, |sink| state_check(sink));

        // Step 1 collected all three at once and step 3 got exactly as far as the first question.
        assert_eq!(notes(&state), vec!["ask:one"]);
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        assert_eq!(
            instance_ids(&events, GameEventType::Destroyed),
            vec![first.id.clone(), second.id.clone(), far.id.clone()]
        );
        // Step 5 has not run either: nothing is reported as having entered a graveyard yet.
        assert!(events_of_type(&events, GameEventType::EnteredGraveyard).is_empty());

        // R113 and R117: the pass parked what it still owes — the asking card (mid-list) and the two
        // cards after it, in R68's order. This is the whole fix; without it they are lost.
        let parked = only_work(owed_work(&state, Some(DEATHS_WORK)));
        assert_eq!(
            owed_ids(&parked.resume),
            Some(vec![first.id.clone(), second.id.clone(), far.id.clone()])
        );
        // §9.3: plain data, so no effect list, closure or live unit array is held across the prompt.
        survives_json(&parked);

        // §10.1: the paused game survives a round trip and resumes from the round-tripped copy.
        let round = round_trip(&state);
        assert_eq!(
            owed_ids(&only_work(owed_work(&round, Some(DEATHS_WORK))).resume),
            Some(vec![first.id.clone(), second.id.clone(), far.id.clone()])
        );

        // The first answer finishes the first hook and runs straight into the second card's question.
        let once = answer(&round);
        assert_eq!(notes(&once.state), vec!["ask:one", "answered:one", "tail:one:3", "ask:two"]);
        assert_eq!(once.state.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        assert_eq!(
            owed_ids(&only_work(owed_work(&once.state, Some(DEATHS_WORK))).resume),
            Some(vec![second.id.clone(), far.id.clone()])
        );

        // And the second finishes the pass: the far side's hook, then steps 4 and 5.
        let twice = answer(&once.state);
        let done = &twice.state;
        assert_eq!(
            notes(done),
            vec![
                "ask:one",
                "answered:one",
                "tail:one:3",
                "ask:two",
                "answered:two",
                "tail:two:3",
                "quiet:before",
                "quiet:tail:3",
            ]
        );
        assert!(done.pending.is_none());
        assert!(done.work.is_empty());
        assert!(owed_work(done, Some(DEATHS_WORK)).is_empty());

        // Step 5 reported all three, once each, and all three are in their owners' graveyards.
        assert_eq!(
            instance_ids(&twice.events, GameEventType::EnteredGraveyard),
            vec![first.id.clone(), second.id.clone(), far.id.clone()]
        );
        assert_eq!(
            [&first, &second, &far].map(|card| in_graveyard(done, card)),
            [true, true, true]
        );
    }

    #[test]
    fn r89_gives_the_resumed_half_of_a_death_hook_the_snapshot_which_the_board_no_longer_holds() {
        let mut state = playing("deaths-snapshot");
        let dying = doomed(&mut state, &ask_one().id, P1, 1);

        with_sink(&mut state, |sink| state_check(sink));
        // R78 has already reset the instance on the board, so the buff is gone from every zone.
        assert_eq!(
            state.players.p1.graveyard.iter().find(|card| card.id == dying.id).map(|card| card.buffs.clone()),
            Some(AttackHealth { attack: 0, health: 0 })
        );
        // And the parked pass carries the snapshot instead, which is where the tail's `ctx.self` comes
        // from: `findInstance` would hand back the reset card above (R89, R127).
        let parked = only_work(owed_work(&state, Some(DEATHS_WORK)));
        let owed = owed_deaths_of(&parked.resume).map(|pass| pass.owed).unwrap_or_default();
        let snapshot = owed.first().cloned().expect("expected exactly one item");
        assert_eq!(snapshot.buffs, AttackHealth { attack: 3, health: 0 });

        let done = answer(&round_trip(&state)).state;
        // "tail:one:3", not "tail:one:0": the effects after the question still read the card as it died.
        assert_eq!(notes(&done), vec!["ask:one", "answered:one", "tail:one:3"]);
    }

    #[test]
    fn r64_r83_still_brings_a_reborn_unit_back_once_the_answer_finishes_the_pass_it_paused() {
        let mut state = playing("deaths-reborn");
        let dying = doomed(&mut state, &reborn_asker().id, P1, 3);

        with_sink(&mut state, |sink| state_check(sink));
        // Step 4 is behind the pause, so the zone is reserved and still empty (R64).
        assert!(state.pending.is_some());
        assert!(card_at(&state, slot(P1, Row::Units, 3)).is_none());
        assert!(state.reserved.contains(&ZoneRef {
            player: P1,
            row: Row::Units,
            lane: 3,
        }));

        let done = answer(&round_trip(&state)).state;
        assert_eq!(notes(&done), vec!["ask:reborn", "answered:reborn", "tail:reborn:3"]);

        let back = card_at(&done, slot(P1, Row::Units, 3));
        assert_eq!(back.map(|card| card.id.clone()), Some(dying.id.clone()));
        // At 1 health, without Reborn, and summoning sick because it entered the field again (R83).
        assert_eq!(back.map(|card| card.damage), Some(1));
        assert_eq!(back.map(|card| card.granted_keywords.clone()), Some(Vec::<Keyword>::new()));
        assert_eq!(back.and_then(|card| card.reborn_spent), Some(true));
        assert_eq!(back.and_then(|card| card.summoned_turn), Some(done.turn));
        assert!(done.reserved.is_empty());
        assert!(done.work.is_empty());
        // It never stayed in the graveyard, so step 5 did not report it (R47).
        assert!(!in_graveyard(&done, &dying));
    }

    #[test]
    fn runs_the_same_hooks_in_the_same_order_with_nothing_to_ask_owing_nothing_at_all() {
        let mut state = playing("deaths-control");
        let first = doomed(&mut state, &quiet_unit().id, P1, 1);
        let second = doomed(&mut state, &quiet_unit().id, P1, 2);
        let far = doomed(&mut state, &quiet_unit().id, P2, 1);

        let events = with_sink(&mut state, |sink| state_check(sink));

        // Every hook ran, in R68's order, inside the one call — and each read its own snapshot.
        assert_eq!(
            notes(&state),
            vec![
                "quiet:before",
                "quiet:tail:3",
                "quiet:before",
                "quiet:tail:3",
                "quiet:before",
                "quiet:tail:3",
            ]
        );
        assert!(state.pending.is_none());
        // Nothing was parked: the machinery only engages at a pause (R117), so this is not the pause
        // path passing by accident.
        assert!(state.work.is_empty());
        assert_eq!(
            instance_ids(&events, GameEventType::EnteredGraveyard),
            vec![first.id.clone(), second.id.clone(), far.id.clone()]
        );
    }
}

mod a_prompt_inside_a_casts_cry_10_5_r70_r113_r122 {
    use super::*;

    #[test]
    fn r113_owes_10_5_steps_6_and_7_when_a_casts_cry_asks_so_the_card_lands_only_on_the_answer() {
        let mut state = playing("cast-owe-tail");
        let card = new_instance(&mut state, &cast_asker().id, P1, Zone::Resolving { player: P1 });

        let events = with_sink(&mut state, |sink| cast_card(sink, &card, CastOptions::default()));

        // The Cry stopped at its question, and the effects after it have not run.
        assert_eq!(notes(&state), vec!["cast:ask"]);
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(P1));
        // R70: it counted as a play immediately, but §10.5 step 7 has not happened — the card is still
        // resolving and nothing has announced it resolved. Running the tail over the open prompt is the
        // bug this pins: it used to land the Spell in the graveyard while the caster was still asked.
        assert_eq!(instance_ids(&events, GameEventType::CardPlayed), vec![card.id.clone()]);
        assert!(events_of_type(&events, GameEventType::CardResolved).is_empty());
        assert!(!in_graveyard(&state, &card));

        // R113: the tail is owed, as plain data, and survives a round trip (§9.3, §10.1). A cast is
        // §10.5's pipeline (R70), so what it owes is the rest of that pipeline, marked as a cast.
        let parked = only_work(owed_work(&state, Some(PLAY_WORK_KIND)));
        let run = json_of(run_of(&parked.resume));
        assert_eq!(run["instanceId"], json!(card.id));
        assert_eq!(run["cast"], json!(true));
        survives_json(&parked);

        let done = answer(&round_trip(&state));
        // The rest of the Cry runs first — still reading its own card — and only then step 7 (R113).
        assert_eq!(notes(&done.state), vec!["cast:ask", "cast:answered", "cast:tail:0"]);
        assert_eq!(instance_ids(&done.events, GameEventType::CardResolved), vec![card.id.clone()]);
        assert!(in_graveyard(&done.state, &card));
        assert!(done.state.pending.is_none());
        assert!(done.state.work.is_empty());
    }

    #[test]
    fn lands_a_cast_whose_cry_asks_nothing_in_the_same_call_owing_nothing_at_all() {
        let mut state = playing("cast-control");
        let card = new_instance(&mut state, &cast_quiet().id, P1, Zone::Resolving { player: P1 });

        let events = with_sink(&mut state, |sink| cast_card(sink, &card, CastOptions::default()));

        assert_eq!(notes(&state), vec!["cast:quiet"]);
        assert!(state.pending.is_none());
        assert!(state.work.is_empty());
        assert_eq!(instance_ids(&events, GameEventType::CardResolved), vec![card.id.clone()]);
        assert!(in_graveyard(&state, &card));
    }
}

mod a_state_check_that_begins_while_a_prompt_is_already_open_4_5_r113_r117_r156 {
    use super::*;

    #[test]
    fn r156_owes_step_3_in_full_rather_than_firing_a_death_hook_into_an_open_prompt() {
        let mut state = playing("deaths-prompt-already-open");
        let dying = doomed(&mut state, &quiet_unit().id, P1, 1);

        // Something else asked first — a trap, an earlier card, the play that killed this one. R156's
        // case is the board settling while that question is still unanswered.
        let events = with_sink(&mut state, |sink| {
            let _ = open_prompt(
                sink,
                OpenPromptArgs {
                    player: P1,
                    kind: PromptKind::Target,
                    aim: None,
                    prompt: "an earlier question, still unanswered".to_string(),
                    options: vec![nothing()],
                    min: None,
                    max: None,
                    budget: None,
                    owner: None,
                    resume: Resume {
                        def_id: String::new(),
                        hook: "resume".to_string(),
                        step: "none".to_string(),
                        radiant: false,
                        instance_id: None,
                        data: IndexMap::new(),
                    },
                },
            );
            assert!(sink.state.pending.is_some());

            state_check(sink);
        });

        // Steps 1 and 2 still run: those are the board settling, not a choice. The card really moved.
        assert!(in_graveyard(&state, &dying));
        assert_eq!(instance_ids(&events, GameEventType::Destroyed), vec![dying.id.clone()]);

        // Step 3 fired nothing. Without R156 the hook runs into the open prompt and whatever it asks is
        // discarded in silence, because `openPrompt` will not overwrite a question already standing.
        assert!(notes(&state).is_empty());

        // The whole of step 3 is owed instead, in R68's order, as plain data (§9.3).
        let parked = only_work(owed_work(&state, Some(DEATHS_WORK)));
        assert_eq!(owed_ids(&parked.resume), Some(vec![dying.id.clone()]));
        survives_json(&parked);

        // The prompt that was standing is untouched: the pass waited for it rather than stepping on it.
        assert_eq!(
            state.pending.as_ref().map(|pending| pending.prompt.as_str()),
            Some("an earlier question, still unanswered")
        );
    }

    #[test]
    fn the_same_death_fires_at_once_with_no_prompt_open_so_the_case_above_is_about_the_prompt() {
        let mut state = playing("deaths-no-prompt-open");
        let dying = doomed(&mut state, &quiet_unit().id, P1, 1);

        with_sink(&mut state, |sink| state_check(sink));

        assert!(state.pending.is_none());
        assert_eq!(notes(&state), vec!["quiet:before", "quiet:tail:3"]);
        assert!(in_graveyard(&state, &dying));
        assert!(owed_work(&state, Some(DEATHS_WORK)).is_empty());
    }
}
