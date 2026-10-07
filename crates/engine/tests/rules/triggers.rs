//! The event registry, the trigger queue and the resolution loop of SPEC §10.3 (BUILD M3-T2).
//!
//! The §10.3 diagram is the subject: apply → emit → traps fire immediately → queue the other
//! triggers in R68's order → state check → repeat. Each acceptance item of BUILD M3-T2 has one
//! named test here:
//!
//!   * R68 — two end-of-turn triggers on one side resolve in lane order;
//!   * §10.3 — a trap fires before a queued trigger, because a trap is a response;
//!   * R62 — except in the end-of-turn trap window, the one scheduled exception, which comes after
//!     the end-of-turn triggers;
//!   * R59 — the state check never runs between two hits of one effect;
//!   * §10.3 — a trap that prompts its owner during the opponent's turn pauses the loop, and the
//!     opponent's action with it, until the prompt is answered;
//!   * R1 — `CRY_ON_PLAY_ONLY` makes `summon` never fire Cry while `play` does, and R70's casts
//!     (Cast on draw, a Call to Chaos cast) do fire it.
//!
//! The Echo third of that last acceptance line — "Cast on draw, Echo and Call to Chaos casts do fire
//! it" — lives in `echo.test.ts` ("§6.3 Echo 1 re-resolves the played card once"), since the repeat
//! queue is that file's whole subject and SPEC puts the repeats in `state.echoQueue`.
//!
//! Every fixture here is its own: defs are prefixed `tg-` and indexed above 1400, so they cannot
//! collide with another test file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/triggers.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};
use crate::rules::fixtures::scripts::double_edge;

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

/// TS `makeContext(sink, null, { controller })`'s options.
fn controlled_by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..HookOptions::default()
    }
}

/// TS `JSON.parse(JSON.stringify(x))`.
fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).expect("serialises")).expect("deserialises")
}

// ---------------------------------------------------------------------------
// Fixtures. TS numbered them from a module counter starting at 1400; each def's index is written
// out here in the order TS created them.
// ---------------------------------------------------------------------------

fn def(name: &str, index: u32, type_: &str, extra: Value) -> CardDef {
    let mut literal = json!({
        "id": format!("tg-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (triggers)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Some(from)) = (literal.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(literal)
}

/// TS `unit(name, attack = 2, health = 4, extra = {})`.
fn unit(name: &str, index: u32) -> CardDef {
    let (attack, health) = (2, 4);
    def(
        name,
        index,
        "Unit",
        json!({
            "base": { "attack": attack, "health": health, "keywords": [], "text": name },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": name },
        }),
    )
}

/// The note sink: a Field Spell parked in p1's backrow lane 5, whose memory records the order.
fn log_card() -> CardDef {
    def("log", 1401, "Field Spell", json!({}))
}
/// #13's shape: an end-of-turn hook that says which lane it fired from (R68).
fn closer() -> CardDef {
    unit("closer", 1402)
}
/// An ordinary queued trigger answering a summon: not a trap, so it waits its turn (§10.3).
fn watcher() -> CardDef {
    def("watcher", 1403, "Field Spell", json!({}))
}
/// #41's shape: a Trap answering the same summon. A response, so it goes first (§10.3).
fn snap_trap() -> CardDef {
    def("snap-trap", 1404, "Trap", json!({}))
}
/// #18's shape: a Field Trap that answers the turn end, so it belongs to R62's window.
fn window_trap() -> CardDef {
    def("window-trap", 1405, "Field Trap", json!({}))
}
/// A Trap whose answer is a prompt for its own controller (§10.3's "prompts for the trap's owner").
fn ask_trap() -> CardDef {
    def("ask-trap", 1406, "Trap", json!({}))
}
/// A Cry on a Unit, so `summon` and `play` can be compared on one card (R1).
fn crier() -> CardDef {
    unit("crier", 1407)
}
/// #21's shape: a Spell that casts itself on draw and whose script notes that it ran (R70).
fn draw_caster() -> CardDef {
    def("draw-caster", 1408, "Spell", json!({}))
}
/// A Spell a Call to Chaos cast resolves for free (R70).
fn chaos_spell() -> CardDef {
    def("chaos-spell", 1409, "Spell", json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![
        log_card(),
        closer(),
        watcher(),
        snap_trap(),
        window_trap(),
        ask_trap(),
        crier(),
        draw_caster(),
        chaos_spell(),
    ]
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

fn note(name: impl Into<String>) -> Effect {
    let name: String = name.into();
    Effect::new("tg:note", move |ctx| {
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

fn lane_of(card: Option<&CardInstance>) -> i32 {
    match card.map(|card| &card.zone) {
        Some(Zone::Field { lane, .. }) => *lane,
        _ => 0,
    }
}

/// §10.6: a prompt for the card's own controller, with one answer, so answering is trivial.
fn ask_controller() -> Effect {
    Effect::new("tg:ask", |ctx| {
        let resume = resume_self(ctx, "asked", IndexMap::new());
        let player = ctx.controller;
        open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the trap asks its owner".into(),
                options: vec![PromptOption {
                    key: "none".into(),
                    label: "nothing".into(),
                    selection: Selection::None,
                    cost: None,
                    radiant: None,
                }],
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
            closer().id,
            both(Script {
                end_of_turn: Some(hook(|ctx| vec![note(format!("end:lane{}", lane_of(ctx.self_.as_ref())))])),
                ..Script::default()
            }),
        ),
        (
            watcher().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "watch-summon",
                    &[GameEventType::Summoned],
                    |_ctx, _event| vec![note("trigger")],
                )],
                ..Script::default()
            }),
        ),
        (
            snap_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "snap-summon",
                    &[GameEventType::Summoned],
                    |_ctx, _event| vec![note("trap")],
                )],
                ..Script::default()
            }),
        ),
        (
            window_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "turn-end",
                    &[GameEventType::TurnEnded],
                    |ctx, _event| vec![note(format!("window:{}", ctx.controller))],
                )],
                ..Script::default()
            }),
        ),
        (
            ask_trap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "ask-summon",
                    &[GameEventType::Summoned],
                    |_ctx, _event| vec![ask_controller()],
                )],
                resume: [("asked", hook(|_ctx| vec![note("answered")]))].into_iter().collect(),
                ..Script::default()
            }),
        ),
        (
            crier().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![note("cry")])),
                ..Script::default()
            }),
        ),
        (
            draw_caster().id,
            both(Script {
                static_flags: Some(json_as(json!({ "castOnDraw": true }))),
                cry: Some(hook(|_ctx| vec![note("cry:onDraw")])),
                ..Script::default()
            }),
        ),
        (
            chaos_spell().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![note("cry:cast")])),
                ..Script::default()
            }),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

/// A fresh game whose catalog and script registry also carry this file's fixtures.
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
    action["nonce"] = json!(format!("tg{nonce}"));
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

/// TS `handCard(state, defId, player = "p1")`.
fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(in_hand(state, def_id, player, 1))
}

/// `Number(id.slice(1))`: an instance id's number (NaN when it has none, as in TS).
fn id_number(id: &str) -> f64 {
    id.get(1..).and_then(|digits| digits.parse::<f64>().ok()).unwrap_or(f64::NAN)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn cost_paid_of_plays(events: &[GameEvent]) -> Vec<i32> {
    events_of_type(events, GameEventType::CardPlayed)
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
            _ => None,
        })
        .collect()
}

fn prompt_is_open(error: Option<&str>) -> bool {
    error.is_some_and(|text| text.contains("a prompt is open"))
}

// ---------------------------------------------------------------------------

mod events_triggers_and_the_s10_3_resolution_loop_m3_t2 {
    use super::*;

    #[test]
    fn r68_two_end_of_turn_triggers_on_one_side_resolve_in_lane_order_not_in_creation_order() {
        let mut state = playing("r68-lane-order");
        // Created lane 3 first, so the instance order and the lane order disagree.
        let later = put(&mut state, &closer().id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let earlier = put(&mut state, &closer().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        assert!(id_number(&later.id) < id_number(&earlier.id));

        // The registry itself reads lane 1 before lane 3 (R68's within-a-side order).
        let order: Vec<String> = cards_in_trigger_order(&state)
            .iter()
            .map(|holder| holder.card.id.clone())
            .filter(|id| *id == earlier.id || *id == later.id)
            .collect();
        assert_eq!(order, vec![earlier.id.clone(), later.id.clone()]);

        let ended = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(notes(&ended), strings(&["end:lane1", "end:lane3"]));
    }

    #[test]
    fn s10_3_fires_a_trap_before_a_queued_trigger_because_a_trap_is_a_response() {
        let mut state = playing("trap-before-trigger");
        // Lane order alone would run the Field Spell first: it is the trap's response status that wins.
        put(&mut state, &watcher().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        put(&mut state, &snap_trap().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));
        let watched: Vec<String> = cards_in_trigger_order(&state)
            .iter()
            .map(|holder| holder.card.def_id.clone())
            .filter(|id| *id == watcher().id || *id == snap_trap().id)
            .collect();
        assert_eq!(watched, vec![watcher().id, snap_trap().id]);

        let mut sink = sink_for(&state);
        {
            let mut engine_sink = sink.on(&mut state);
            let mut ctx = make_context(&mut engine_sink, None, controlled_by(PlayerId::P1));
            apply_effects(
                &[effects::summon(json_as(json!({ "defId": crier().id })))],
                &mut ctx,
            );
        }
        assert_eq!(events_of_type(&sink.events, GameEventType::Summoned).len(), 1);

        settle(&mut sink.on(&mut state), SettleOptions::default());
        assert_eq!(notes(&state), strings(&["trap", "trigger"]));
        // §5.1: the Trap is spent and public; the Field Spell that answered is untouched.
        assert!(state.players.p1.graveyard.iter().any(|card| card.def_id == snap_trap().id));
        let fired: Vec<String> = events_of_type(&sink.events, GameEventType::TrapFired)
            .iter()
            .filter_map(|event| match event {
                GameEvent::TrapFired { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(fired, vec![snap_trap().id]);
        assert!(state.trigger_queue.is_empty());
    }

    #[test]
    fn r62_holds_the_end_of_turn_trap_window_back_to_its_scheduled_point_after_the_end_of_turn_triggers() {
        let mut state = playing("r62-window");
        put(&mut state, &closer().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        put(&mut state, &window_trap().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        put(&mut state, &window_trap().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));

        // `turnEnded` is the one event the immediate dispatch withholds: the window owns it (R62).
        let turn_ended = GameEvent::TurnEnded {
            player: PlayerId::P1,
            turn: state.turn,
            unspent_mana: 0,
        };
        assert!(is_trap_window_event(&turn_ended));
        let mut immediate = sink_for(&state);
        assert_eq!(
            fire_traps_for(&mut immediate.on(&mut state), &turn_ended).fired,
            Vec::<String>::new()
        );
        assert!(notes(&state).is_empty());

        // At its scheduled point it fires on both sides, the ending player's traps first (R68).
        let mut scheduled = sink_for(&state);
        assert_eq!(
            run_trap_window(&mut scheduled.on(&mut state), &turn_ended).fired.len(),
            2
        );
        assert_eq!(notes(&state), strings(&["window:p1", "window:p2"]));

        // And in the turn loop the window comes after the end-of-turn triggers, never before them.
        let mut live = playing("r62-window-live");
        put(&mut live, &closer().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        put(&mut live, &window_trap().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        put(&mut live, &window_trap().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));
        let ended = act(&live, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(notes(&ended), strings(&["end:lane1", "window:p1", "window:p2"]));
    }

    #[test]
    fn r59_runs_no_state_check_between_the_hits_of_one_effect_so_one_effect_can_leave_both_heroes_at_0() {
        let mut state = playing("r59-double-edge");
        // #fx-999 Double Edge: 30 to the enemy hero, then draw 1 — and p1's library is empty, so the
        // draw is a fatigue hit on p1's own hero (§2.4, R3). One effect list, two lethal hits.
        state.players.p1.library = vec![];
        state.players.p1.fatigue_count = 0;
        state.players.p1.hero.health = 1;
        let card = hand_card(&mut state, &double_edge().id, PlayerId::P1);

        // The contrast first: a check taken between the two hits would have ended it as a p1 win.
        let cry = script_of(&state, &card.def_id).base.cry.clone();
        let mut split = sink_for(&state);
        let mut split_sink = split.on(&mut state);
        let mut split_ctx = make_context(&mut split_sink, Some(&card), controlled_by(PlayerId::P1));
        let cry_effects = match &cry {
            Some(run) => run(&mut split_ctx),
            None => vec![],
        };
        assert_eq!(cry_effects.len(), 2);
        apply_effects(&cry_effects[..1], &mut split_ctx);
        state_check(&mut split_ctx);
        assert_eq!(
            split_ctx.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath,
            })
        );

        // The whole effect, then one check: both heroes are at 0 in that check, which is a draw (R59).
        let mut whole = playing("r59-double-edge-whole");
        whole.players.p1.library = vec![];
        whole.players.p1.fatigue_count = 0;
        whole.players.p1.hero.health = 1;
        let played_id = hand_card(&mut whole, &double_edge().id, PlayerId::P1).id;
        let played = act_result(
            &whole,
            json!({ "type": "play", "instanceId": played_id, "playerId": "p1" }),
        );
        assert_eq!(played.error, None);
        assert!(played.state.players.p1.hero.health <= 0);
        assert!(played.state.players.p2.hero.health <= 0);
        assert_eq!(
            played.state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::BothHeroesDead,
            })
        );
    }

    #[test]
    fn s10_3_pauses_the_loop_where_it_stands_when_a_trap_prompts_its_owner_keeping_the_queue_behind_it() {
        let mut state = playing("trap-prompt-pause");
        assert_eq!(state.active, PlayerId::P1);
        let waiting = put(&mut state, &watcher().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        put(&mut state, &ask_trap().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));

        let mut sink = sink_for(&state);
        {
            let mut engine_sink = sink.on(&mut state);
            let mut ctx = make_context(&mut engine_sink, None, controlled_by(PlayerId::P1));
            apply_effects(
                &[effects::summon(json_as(json!({ "defId": crier().id })))],
                &mut ctx,
            );
        }
        settle(&mut sink.on(&mut state), SettleOptions::default());

        // The prompt belongs to the trap's owner, who is not the active player (§10.3, §10.6).
        assert_eq!(state.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P2));
        assert_eq!(state.active, PlayerId::P1);
        // And the trigger the trap jumped ahead of is still owed, in state, not on the sink (§9.3).
        assert_eq!(
            state
                .trigger_queue
                .iter()
                .map(|entry| entry.instance_id.clone())
                .collect::<Vec<_>>(),
            vec![waiting.id.clone()]
        );
        assert!(notes(&state).is_empty());
        // §10.1 keeps the paused loop JSON, so it survives a round trip.
        let round: GameState = round_trip(&state);
        assert_eq!(round.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P2));
        assert_eq!(round.trigger_queue.len(), 1);
    }

    #[test]
    fn r118_pauses_the_opponents_action_until_the_traps_prompt_is_answered_without_eating_the_cry_s10_3() {
        let mut state = playing("trap-prompt-blocks-action");
        put(&mut state, &ask_trap().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));
        let crier_id = hand_card(&mut state, &crier().id, PlayerId::P1).id;
        let played = act_result(
            &state,
            json!({
                "type": "play",
                "instanceId": crier_id,
                "playerId": "p1",
                "zone": { "row": "units", "lane": 1 },
            }),
        );
        assert_eq!(played.error, None);

        // p1's play emitted the summon, so p2's trap holds the game before p1 can act again — and it
        // holds it inside §10.5, between step 4's `summoned` and step 5's Cry, because a trap is a
        // response and answers the play event first (§10.3, R17).
        let mut paused = played.state;
        assert_eq!(paused.pending.as_ref().map(|p| p.player_id), Some(PlayerId::P2));
        assert!(notes(&paused).is_empty());
        assert!(prompt_is_open(
            act_result(&paused, json!({ "type": "endTurn", "playerId": "p1" }))
                .error
                .as_deref()
        ));
        let another = hand_card(&mut paused, &crier().id, PlayerId::P1).id;
        assert!(prompt_is_open(
            act_result(
                &paused,
                json!({ "type": "play", "instanceId": another, "playerId": "p1" }),
            )
            .error
            .as_deref()
        ));

        // Once p2 answers, p1's turn carries on: the trap's continuation ran, then the interrupted play
        // resumed at the step after the one that paused, so the Cry fires — once, after the trap, and
        // never dropped (§10.5 step 5, R1, R113's "a work item that cannot be resumed is a lost
        // sequence"). Nothing is pending and nothing is still owed.
        let choice_id = paused.pending.as_ref().map(|p| p.id.clone()).unwrap_or_default();
        let answered = act(
            &paused,
            json!({
                "type": "answer",
                "choiceId": choice_id,
                "selection": [{ "pick": "none" }],
                "playerId": "p2",
            }),
        );
        assert!(answered.pending.is_none());
        assert_eq!(notes(&answered), strings(&["answered", "cry"]));
        assert!(answered.work.is_empty());
        assert_eq!(
            act_result(&answered, json!({ "type": "endTurn", "playerId": "p1" })).error,
            None
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn r1_cry_on_play_only_makes_summon_never_fire_cry_while_playing_the_same_card_from_hand_does() {
        assert!(CRY_ON_PLAY_ONLY);

        let mut state = playing("r1-summon-vs-play");
        let mut sink = sink_for(&state);
        {
            let mut engine_sink = sink.on(&mut state);
            let mut ctx = make_context(&mut engine_sink, None, controlled_by(PlayerId::P1));
            apply_effects(
                &[effects::summon(json_as(json!({ "defId": crier().id })))],
                &mut ctx,
            );
        }
        assert_eq!(events_of_type(&sink.events, GameEventType::Summoned).len(), 1);
        // A summon is not a play: no `cardPlayed`, no Cry (§6.3 Summon, R1).
        assert_eq!(events_of_type(&sink.events, GameEventType::CardPlayed).len(), 0);
        assert!(notes(&state).is_empty());
        settle(&mut sink.on(&mut state), SettleOptions::default());
        assert!(notes(&state).is_empty());

        let crier_id = hand_card(&mut state, &crier().id, PlayerId::P1).id;
        let played = act_result(
            &state,
            json!({
                "type": "play",
                "instanceId": crier_id,
                "playerId": "p1",
                "zone": { "row": "units", "lane": 2 },
            }),
        );
        assert_eq!(played.error, None);
        assert_eq!(events_of_type(&played.events, GameEventType::CardPlayed).len(), 1);
        assert_eq!(notes(&played.state), strings(&["cry"]));
    }

    #[test]
    fn r70_fires_the_cry_for_a_cast_too_a_cast_on_draw_and_a_call_to_chaos_cast_both_count_as_plays() {
        let mut state = playing("r70-casts");
        let mut sink = sink_for(&state);

        // Cast on draw: the card never enters the hand and its script runs at once (§2.4, R70).
        set_library(&mut state, PlayerId::P1, &[draw_caster().id]);
        let before = state.counters.played;
        {
            let mut engine_sink = sink.on(&mut state);
            let mut ctx = make_context(&mut engine_sink, None, controlled_by(PlayerId::P1));
            apply_effects(&[effects::draw(json_as(json!({ "count": 1 })))], &mut ctx);
        }
        assert_eq!(notes(&state), strings(&["cry:onDraw"]));
        assert_eq!(state.counters.played, before + 1);
        assert!(!state.players.p1.hand.iter().any(|card| card.def_id == draw_caster().id));
        assert_eq!(cost_paid_of_plays(&sink.events), vec![0]);

        // A Call to Chaos cast: free, counts as a play, fires the script (R70).
        let chaos = hand_card(&mut state, &chaos_spell().id, PlayerId::P1);
        cast_card(&mut sink.on(&mut state), &chaos, CastOptions::default());
        assert_eq!(notes(&state), strings(&["cry:onDraw", "cry:cast"]));
        assert_eq!(state.counters.played, before + 2);
        assert_eq!(cost_paid_of_plays(&sink.events), vec![0, 0]);
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == chaos.id));
    }
}
