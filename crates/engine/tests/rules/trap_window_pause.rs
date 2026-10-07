//! A prompt opened inside the end-of-turn trap window (SPEC §2.2, §9.3, §10.3, §10.6; R62, R100,
//! R113).
//!
//! The window of §2.2 delivers one `turnEnded` event to every armed trap on both sides, in R68's
//! order. It is therefore a sequence that can span a prompt, and the governing rule of this engine
//! is that such a sequence must be resumable through `state.work`: never an effect list, a closure
//! or a remaining-traps array held across the pause, only the step's name and its captured data as
//! plain JSON (§9.3, R113). `traps.runTrapWindow` parks the event plus the ids of the traps that
//! have not seen it as a `TRAP_WINDOW_WORK` item, and `triggers.settle`'s drain finishes the window
//! once the answer arrives.
//!
//! The three things pinned here:
//!
//!   * R113 — a trap that prompts stops the window and the rest of it is *owed*, not dropped. The
//!     answer fires the traps that had not seen the event, in the window's own order, and a
//!     `JSON.parse(JSON.stringify(state))` of the paused game resumes identically.
//!   * R100 and R33 — the resumed window offers the event only to the traps still owed it, so the
//!     Field Trap that prompted, which stays on the field after firing, does not fire a second time.
//!   * R62 — the window finishes before the end-of-turn delayed effects and before cleanup, whether
//!     or not a prompt interrupted it.
//!
//! Fixtures are this file's own: defs are prefixed `tw-` and indexed from 2200, so they cannot
//! collide with another test file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/trap-window-pause.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// The sink: TS `sinkFor(state)`, a sink whose rng starts at the state's cursor, as reduce does.
// Rust's `EngineSink` borrows the state, so the event list and the rng live here and each call
// borrows the state again; the state is read between calls as TS read `state`.
// ---------------------------------------------------------------------------

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

/// TS `JSON.parse(JSON.stringify(x))`.
fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).expect("serialises")).expect("deserialises")
}

// ---------------------------------------------------------------------------
// Fixtures. TS numbered them from a module counter starting at 2200; each def's index is written
// out here in the order TS created them.
// ---------------------------------------------------------------------------

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
/// #18's shape, but it asks first: a Field Trap on `turnEnded` that prompts its own controller.
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
    vec![log_card(), ask_trap(), second_trap(), third_trap(), delayed_card()]
}

// ---------------------------------------------------------------------------
// The note log: what fired, in the order it fired.
// ---------------------------------------------------------------------------

const NOTE_LANE: usize = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state.players.p1.backrow.get(NOTE_LANE - 1).and_then(Option::as_ref)
}

fn log_of_mut(state: &mut GameState) -> Option<&mut CardInstance> {
    state.players.p1.backrow.get_mut(NOTE_LANE - 1).and_then(Option::as_mut)
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
        .map(|steps| steps.iter().filter_map(|step| step.as_str().map(str::to_string)).collect())
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
                resume: [("asked", hook(|_ctx| vec![note("answered")]))].into_iter().collect(),
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

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

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

/// TS's module-level `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// `body` is the TS `ActionInput` literal; the nonce is added here.
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
    state.players[player].hand.iter().map(|card| card.id.clone()).collect()
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
    put(&mut state, &log_card().id, slot(PlayerId::P1, Row::Backrow, NOTE_LANE as i32), json!({}));
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

// ---------------------------------------------------------------------------

mod a_prompt_inside_the_end_of_turn_trap_window_s2_2_r62_r100_r113 {
    use super::*;

    #[test]
    fn r113_owes_the_rest_of_the_window_when_a_trap_prompts_and_the_answer_fires_the_traps_it_had_not_reached() {
        let mut state = playing("window-owes-remainder");
        let asking = put(&mut state, &ask_trap().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let second = put(&mut state, &second_trap().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));
        let third = put(&mut state, &third_trap().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));

        let ended = act_result(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(ended.error, None);
        let paused = &ended.state;

        // The first trap of the window fired and is waiting on its own controller (§10.6, R52).
        assert_eq!(paused.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P1));
        assert_eq!(notes(paused), strings(&["window1"]));
        assert_eq!(trap_fired_ids(&ended.events), vec![asking.id.clone()]);

        // R113: the window parked what it still owes — the event, and the traps that have not seen it,
        // in the window's order. This is the whole fix: without it the rest of the window is lost.
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
        // Plain data, so no effect list, closure or live trap array is held across the prompt (§9.3).
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

        // The traps the pause stopped the window from reaching fire on the answer, in R62's order:
        // the ending player's side first, then the opponent's.
        assert_eq!(
            notes(answered),
            strings(&["window1", "answered", "window2", "window3"])
        );
        assert_eq!(
            trap_fired_ids(&resumed.events),
            vec![second.id.clone(), third.id.clone()]
        );
        // Nothing is owed any more, and no prompt is left open.
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
        let asking = put(&mut state, &ask_trap().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let second = put(&mut state, &second_trap().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));

        let paused = act_result(&state, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert!(paused.pending.is_some());
        // The trap that fired is a Field Trap: it is still on the field, so re-offering it the event
        // is a real risk and the owed list is what rules it out (§5.1, R33).
        assert_eq!(
            card_at(&paused, slot(PlayerId::P1, Row::Backrow, 1)).map(|c| c.id.clone()),
            Some(asking.id.clone())
        );
        assert_eq!(
            owed_window_of(&only(owed_work(&paused, Some(TRAP_WINDOW_WORK))).resume).map(|window| window.owed),
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
        let first = put(&mut state, &second_trap().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let second = put(&mut state, &third_trap().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));

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
        // Every matched trap is owed the event: a bare `if (pending !== null) return;` would drop them.
        assert_eq!(
            owed_window_of(&only(owed_work(&state, Some(TRAP_WINDOW_WORK))).resume).map(|window| window.owed),
            Some(vec![first.id.clone(), second.id.clone()])
        );

        // And once the prompt is gone the drain delivers it, in the window's order.
        close_prompt(&mut sink.on(&mut state));
        settle(&mut sink.on(&mut state), SettleOptions::default());
        assert_eq!(notes(&state), strings(&["window2", "window3"]));
        assert!(state.work.is_empty());
    }

    // NOTE — this one fails today, and the half that is missing is not the window's.
    //
    // It is the joint acceptance criterion for §2.2's end of turn: `traps.runTrapWindow` now owes its
    // remainder (the three tests above), but `turn.ts`'s `endTurn` does not notice that the window
    // paused. It calls `runTrapWindow` and walks straight on to `runDelayed`, `cleanup` and the next
    // `startTurn` with the prompt still open, so the delayed effect below resolves *inside* the
    // unfinished window and cleanup closes the turn log before the window's last traps have fired —
    // R62's order, broken on the pause path.
    //
    // The fix is the same shape as the window's and belongs to `turn.ts`: park the steps after the
    // window as a work item of its own when `state.pending !== null`. Parked at `state.workCursor` it
    // lands behind the window's remainder, so the drain finishes the window first and R62's order
    // survives the pause. `traps.ts` cannot do it — `runDelayed` and `cleanup` are `turn.ts`'s alone.
    #[test]
    fn r62_finishes_the_window_before_the_end_of_turn_delayed_effects_and_before_cleanup() {
        let mut state = playing("window-before-delayed");
        put(&mut state, &ask_trap().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        put(&mut state, &second_trap().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));
        let scheduler = put(&mut state, &delayed_card().id, slot(PlayerId::P1, Row::Backrow, 3), json!({}));

        // §2.2's order for the end of a turn: triggers, window, delayed effects, cleanup (R62).
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

        // The window is unfinished, so nothing R62 puts after it has happened: the delayed effect is
        // still due and cleanup has not closed p1's turn log.
        assert_eq!(notes(&paused), strings(&["window1"]));
        assert_eq!(
            paused.delayed.iter().map(|effect| effect.id.clone()).collect::<Vec<_>>(),
            vec![scheduled.id.clone()]
        );
        assert_eq!(paused.players.p1.turn_log.unspent_at_end, None);

        // The answer finishes the window first, and only then the rest of R62's order runs.
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
