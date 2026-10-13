//! A prompt opened inside the end-of-turn trap window (SPEC §2.2, §9.3, §10.3, §10.6; R62, R100,
//! R113).
//!
//! The window delivers one `turnEnded` event to every armed trap on both sides, in R68's order, so
//! it can span a prompt and must be resumable through `state.work`: only the event and the ids of
//! the traps that have not seen it, as plain JSON (§9.3, R113).
//!
//!   * R113: a trap that prompts stops the window; the rest is owed, and the answer fires it in the
//!     window's order. A JSON round trip of the paused game resumes identically.
//!   * R100 and R33: the resumed window offers the event only to the traps still owed it, so the
//!     Field Trap that prompted does not fire twice.
//!   * R62: the window finishes before the end-of-turn delayed effects and cleanup, prompt or not.
//!
//! Fixtures are this file's own: `tw-` defs indexed from 2200 (BUILD §0).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// The sink: its rng starts at the state's cursor, as reduce's does. `EngineSink` borrows the state,
// so the event list and the rng live here and each call borrows the state again.

struct SinkFor {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> SinkFor {
    SinkFor {
        events: Vec::new(),
        rng: Rng::new(&state.seed, state.rng_cursor),
    }
}

impl SinkFor {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).expect("serialises")).expect("deserialises")
}

// Fixtures.

fn def(name: &str, index: u32, type_: &str) -> CardDef {
    json_as(json!({
        "id": format!("tw-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (trap window)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// The note sink: a Field Spell parked in p1's backrow lane 5, whose memory records the order.
fn log_card() -> CardDef {
    def("log", 2201, "Field Spell")
}
/// A Field Trap on `turnEnded` that prompts its own controller.
fn ask_trap() -> CardDef {
    def("ask", 2202, "Field Trap")
}
/// The trap behind it in R68's order, on the ending player's own side.
fn second_trap() -> CardDef {
    def("second", 2203, "Field Trap")
}
/// And the opponent's, which R62 puts last in the window.
fn third_trap() -> CardDef {
    def("third", 2204, "Field Trap")
}
/// A card holding an end-of-turn delayed effect, which R62 puts after the whole window.
fn delayed_card() -> CardDef {
    def("delayed", 2205, "Field Spell")
}

fn defs() -> Vec<CardDef> {
    vec![
        log_card(),
        ask_trap(),
        second_trap(),
        third_trap(),
        delayed_card(),
    ]
}

// The note log: what fired, in the order it fired.

const NOTE_LANE: usize = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state
        .players
        .p1
        .backrow
        .get(NOTE_LANE - 1)
        .and_then(Option::as_ref)
}

fn log_of_mut(state: &mut GameState) -> Option<&mut CardInstance> {
    state
        .players
        .p1
        .backrow
        .get_mut(NOTE_LANE - 1)
        .and_then(Option::as_mut)
}

fn note(name: &str) -> Effect {
    let name = name.to_string();
    Effect::new("tw:note", move |ctx| {
        let Some(log) = log_of_mut(ctx.state) else {
            return;
        };
        let mut steps: Vec<Value> = log
            .memory
            .get("steps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        steps.push(json!(name));
        log.memory.insert("steps".into(), Value::Array(steps));
    })
}

fn notes(state: &GameState) -> Vec<String> {
    log_of(state)
        .and_then(|log| log.memory.get("steps"))
        .and_then(Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .filter_map(|step| step.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn none_option() -> PromptOption {
    PromptOption {
        key: "none".into(),
        label: "nothing".into(),
        selection: Selection::None,
        cost: None,
        radiant: None,
    }
}

/// §10.6: a prompt for the trap's own controller, with one answer, so answering is trivial.
fn ask_controller() -> Effect {
    Effect::new("tw:ask", |ctx| {
        let resume = resume_self(ctx, "asked", IndexMap::new());
        let player = ctx.controller;
        open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the window's trap asks its owner".into(),
                options: vec![none_option()],
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

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            ask_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "tw-ask-end",
                    &[GameEventType::TurnEnded],
                    |_ctx, _event| vec![note("window1"), ask_controller()],
                )],
                resume: [("asked", hook(|_ctx| vec![note("answered")]))]
                    .into_iter()
                    .collect(),
                ..Script::default()
            }),
        ),
        (
            second_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "tw-second-end",
                    &[GameEventType::TurnEnded],
                    |_ctx, _event| vec![note("window2")],
                )],
                ..Script::default()
            }),
        ),
        (
            third_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "tw-third-end",
                    &[GameEventType::TurnEnded],
                    |_ctx, _event| vec![note("window3")],
                )],
                ..Script::default()
            }),
        ),
        (
            delayed_card().id,
            both(Script {
                delayed: Some(hook(|_ctx| vec![note("delayed")])),
                ..Script::default()
            }),
        ),
    ]
}

// Harness.

fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
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
    state
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// The nonce is added here.
fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("tw{nonce}"));
    reduce(state, &json_as::<Action>(action))
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// Past the mulligans, in p1's main phase, with the note log parked in p1's backrow lane 5.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": hand_ids(&state, PlayerId::P1), "playerId": "p1" }),
    );
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": hand_ids(&state, PlayerId::P2), "playerId": "p2" }),
    );
    put(
        &mut state,
        &log_card().id,
        slot(PlayerId::P1, Row::Backrow, NOTE_LANE as i32),
        json!({}),
    );
    state
}

fn only<T>(items: Vec<T>) -> T {
    items.into_iter().next().expect("expected at least one item")
}

/// Answer the one open prompt, whoever it belongs to.
fn answer(state: &GameState) -> ReduceResult {
    let pending = state.pending.as_ref().expect("expected a prompt to be open");
    act_result(
        state,
        json!({
            "type": "answer",
            "choiceId": pending.id,
            "selection": [{ "pick": "none" }],
            "playerId": pending.player_id,
        }),
    )
}

fn turn_ended_in(state: &GameState, player: PlayerId) -> GameEvent {
    GameEvent::TurnEnded {
        player,
        turn: state.turn,
        unspent_mana: state.players[player].mana.current,
    }
}

fn trap_fired_ids(events: &[GameEvent]) -> Vec<String> {
    events_of_type(events, GameEventType::TrapFired)
        .iter()
        .filter_map(|event| match event {
            GameEvent::TrapFired { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

mod a_prompt_inside_the_end_of_turn_trap_window_s2_2_r62_r100_r113 {
    use super::*;

    #[test]
    fn r113_owes_the_rest_of_the_window_when_a_trap_prompts_and_the_answer_fires_the_traps_it_had_not_reached()
     {
        let mut state = playing("window-owes-remainder");
        let asking = put(
            &mut state,
            &ask_trap().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let second = put(
            &mut state,
            &second_trap().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let third = put(
            &mut state,
            &third_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );

        let ended = act_result(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(ended.error, None);
        let paused = &ended.state;

        // The first trap of the window fired and is waiting on its own controller (§10.6, R52).
        assert_eq!(paused.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P1));
        assert_eq!(notes(paused), strings(&["window1"]));
        assert_eq!(trap_fired_ids(&ended.events), vec![asking.id.clone()]);

        // R113: the window parked what it still owes: the event, and the traps that have not seen it,
        // in the window's order.
        let parked = only(owed_work(paused, Some(TRAP_WINDOW_WORK)));
        let owed = owed_window_of(&parked.resume);
        assert_eq!(
            owed.as_ref().map(|window| window.event.event_type()),
            Some(GameEventType::TurnEnded)
        );
        assert_eq!(
            owed.as_ref().map(|window| window.owed.clone()),
            Some(vec![second.id.clone(), third.id.clone()])
        );
        // Plain data (§9.3).
        assert_eq!(round_trip(&parked), parked);

        // §10.1: the paused game survives a round trip and resumes from the round-tripped copy.
        let round: GameState = round_trip(paused);
        assert_eq!(
            owed_window_of(&only(owed_work(&round, Some(TRAP_WINDOW_WORK))).resume).map(|window| window.owed),
            Some(vec![second.id.clone(), third.id.clone()])
        );

        let resumed = answer(&round);
        assert_eq!(resumed.error, None);
        let answered = &resumed.state;

        // The owed traps fire on the answer, in R62's order: the ending player's side first.
        assert_eq!(
            notes(answered),
            strings(&["window1", "answered", "window2", "window3"])
        );
        assert_eq!(
            trap_fired_ids(&resumed.events),
            vec![second.id.clone(), third.id.clone()]
        );
        assert!(answered.pending.is_none());
        assert!(owed_work(answered, Some(TRAP_WINDOW_WORK)).is_empty());
        assert!(answered.work.is_empty());
        // R100 and R33: the Field Trap that prompted is still on the field, face up, and fired once.
        assert_eq!(
            card_at(answered, slot(PlayerId::P1, Row::Backrow, 1)).map(|c| c.id.clone()),
            Some(asking.id.clone())
        );
        assert_eq!(
            card_at(answered, slot(PlayerId::P1, Row::Backrow, 1)).and_then(|c| c.face_up),
            Some(true)
        );
    }

    #[test]
    fn r100_re_offers_the_event_to_nobody_who_has_already_seen_it_so_one_turn_end_is_one_firing() {
        let mut state = playing("window-no-refire");
        let asking = put(
            &mut state,
            &ask_trap().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let second = put(
            &mut state,
            &second_trap().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );

        let paused = act_result(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert!(paused.pending.is_some());
        // A Field Trap stays on the field after firing; the owed list is what stops a re-offer (§5.1, R33).
        assert_eq!(
            card_at(&paused, slot(PlayerId::P1, Row::Backrow, 1)).map(|c| c.id.clone()),
            Some(asking.id.clone())
        );
        assert_eq!(
            owed_window_of(&only(owed_work(&paused, Some(TRAP_WINDOW_WORK))).resume)
                .map(|window| window.owed),
            Some(vec![second.id.clone()])
        );

        let answered = answer(&paused);
        assert_eq!(
            notes(&answered.state),
            strings(&["window1", "answered", "window2"])
        );
        assert_eq!(trap_fired_ids(&answered.events), vec![second.id.clone()]);
    }

    #[test]
    fn r113_owes_the_whole_window_when_a_prompt_is_already_open_at_its_scheduled_point() {
        let mut state = playing("window-already-paused");
        let first = put(
            &mut state,
            &second_trap().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let second = put(
            &mut state,
            &third_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );

        // A prompt an end-of-turn trigger left open: the window has delivered its event to nobody.
        let mut sink = sink_for(&state);
        open_prompt(
            &mut sink.on(&mut state),
            OpenPromptArgs {
                player: PlayerId::P1,
                kind: PromptKind::Target,
                aim: None,
                prompt: "something earlier is still asking".into(),
                options: vec![none_option()],
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume: resume_at(json_as(json!({ "defId": log_card().id, "step": "noop" }))),
            },
        );

        let ended = turn_ended_in(&state, PlayerId::P1);
        let dispatch = run_trap_window(&mut sink.on(&mut state), &ended);
        assert_eq!(dispatch.fired, Vec::<String>::new());
        assert!(dispatch.paused);
        assert!(notes(&state).is_empty());
        // Every matched trap is owed the event.
        assert_eq!(
            owed_window_of(&only(owed_work(&state, Some(TRAP_WINDOW_WORK))).resume).map(|window| window.owed),
            Some(vec![first.id.clone(), second.id.clone()])
        );

        close_prompt(&mut sink.on(&mut state));
        settle(&mut sink.on(&mut state), SettleOptions::default());
        assert_eq!(notes(&state), strings(&["window2", "window3"]));
        assert!(state.work.is_empty());
    }

    // The joint acceptance criterion for §2.2's end of turn: when the window pauses, the steps after
    // it (delayed effects, cleanup) wait behind its owed remainder, so R62's order survives the pause.
    #[test]
    fn r62_finishes_the_window_before_the_end_of_turn_delayed_effects_and_before_cleanup() {
        let mut state = playing("window-before-delayed");
        put(
            &mut state,
            &ask_trap().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        put(
            &mut state,
            &second_trap().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let scheduler = put(
            &mut state,
            &delayed_card().id,
            slot(PlayerId::P1, Row::Backrow, 3),
            json!({}),
        );

        let mut sink = sink_for(&state);
        let scheduled = schedule_delayed(
            &mut sink.on(&mut state),
            PlayerId::P1,
            DelayedAt {
                phase: Phase::End,
                player: PlayerId::P1,
            },
            Resume {
                def_id: delayed_card().id,
                hook: "delayed".into(),
                step: String::new(),
                radiant: false,
                instance_id: Some(scheduler.id.clone()),
                data: IndexMap::new(),
            },
            None,
            None,
        );

        let paused = act_result(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert_eq!(paused.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P1));

        // The window is unfinished: the delayed effect is still due and cleanup has not run.
        assert_eq!(notes(&paused), strings(&["window1"]));
        assert_eq!(
            paused
                .delayed
                .iter()
                .map(|effect| effect.id.clone())
                .collect::<Vec<_>>(),
            vec![scheduled.id.clone()]
        );
        assert_eq!(paused.players.p1.turn_log.unspent_at_end, None);

        let answered = answer(&paused).state;
        assert_eq!(
            notes(&answered),
            strings(&["window1", "answered", "window2", "delayed"])
        );
        assert!(answered.delayed.is_empty());
        assert!(answered.players.p1.turn_log.unspent_at_end.is_some());
        assert!(answered.work.is_empty());
    }
}
