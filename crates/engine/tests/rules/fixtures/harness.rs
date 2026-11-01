//! Small helpers shared by the engine tests: a registered fixture catalog, a sink and placement.
//!
//! The registries are the testkit's thread-local override, so `setup_catalog` registers for the
//! calling test's thread only. Surface contract: docs/v0.3.0/SURFACE.md §8.
//! `put` hands back an owned copy of the placed card; a test that changes it writes through
//! `find_instance_mut`.
//! The rules tests take `both`, `spread`, `json_of`, `events_json`, the nonce, the action, card and
//! effect helpers, the sink bench (`Bench`), `sink_for`, `ids_of`, `types_of` and `hand_ids` from here
//! rather than a copy (#1777, #1997, #2024; crates/tools/tests/acceptance.rs refuses a copy, a local
//! `static NONCE` and a local struct of just `events` and `rng`). The runs several files had copied
//! word for word (`run_as_p1`, `run_with_options`, `run_with_targets`, `apply_now`, `run_on_sink`) and
//! the game past both mulligans (`past_the_mulligans`) are here too (#2160); a file that calls one
//! `run` or `playing` imports it under that name.

use std::borrow::Borrow;
use std::cell::Cell;

use jackioh_engine::testkit::*;
use serde::Serialize;

use super::catalog::{vanilla_catalog, vanilla_deck};
use super::combat::{COMBAT_SCRIPTS, combat_catalog};
use super::scripts::{FIXTURE_SCRIPTS, fixture_catalog};

/// The fixture catalog every engine test starts from, with its scripts.
pub fn setup_catalog() {
    register_catalog(combat_catalog(fixture_catalog(vanilla_catalog(2 * DECK_SIZE, 1))));
    let mut merged: IndexMap<String, CardScripts> = FIXTURE_SCRIPTS.clone();
    merged.extend(COMBAT_SCRIPTS.clone());
    register_scripts(merged);
}

/// A game in setup; `None` deals the vanilla decks `fx-1`…`fx-20` and `fx-21`…`fx-40`.
pub fn new_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    setup_catalog();
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21))),
        ..CreateGameOptions::default()
    })
}

/// A sink whose rng starts at the state's cursor, as reduce does. Its event list starts empty, and
/// it and the rng are leaked, so the sink borrows only the state. A test that reads the events back
/// takes a `Bench`.
pub fn sink_for(state: &mut GameState) -> EngineSink<'_> {
    let rng: &mut Rng = Box::leak(Box::new(Rng::new(&state.seed, state.rng_cursor)));
    let events: &mut Vec<GameEvent> = Box::leak(Box::default());
    EngineSink::new(state, events, rng)
}

/// A sink's events and rng (from the state's seed and cursor, as reduce starts one). The state stays the
/// test's, lent with the bench to one engine call at a time, so the test reads both between calls.
pub struct Bench {
    pub events: Vec<GameEvent>,
    pub rng: Rng,
}

impl Bench {
    pub fn new(state: &GameState) -> Bench {
        Bench {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    pub fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }

    pub fn run<T>(&mut self, state: &mut GameState, call: impl FnOnce(&mut EngineSink<'_>) -> T) -> T {
        call(&mut self.sink(state))
    }
}

pub fn slot(player: PlayerId, row: Row, lane: i32) -> ZoneSlot {
    ZoneSlot { player, row, lane }
}

/// A new instance of `def_id`, owned by the slot's player, placed straight into the slot.
/// `options` is `json!({})` or `json!({ "radiant": true })`.
pub fn put(state: &mut GameState, def_id: &str, at: ZoneSlot, options: Value) -> CardInstance {
    let player = at.player;
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    if options.get("radiant").and_then(Value::as_bool) == Some(true) {
        card.radiant = true;
    }
    // R1081000: a Flying Unit put in a backrow zone stands there, as one played into it does.
    let stands = at.row == Row::Backrow && unit_has(state, &card, KeywordKind::Flying);
    let options = PlaceOnFieldOptions::default();
    let placed = if stands {
        place_standing_on_field(state, &mut card, at, options)
    } else {
        place_on_field(state, &mut card, at, options)
    };
    if !placed {
        panic!("could not place {def_id} in {} {}", at.row, at.lane);
    }
    find_instance(state, &card.id).cloned().unwrap_or(card)
}

pub fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId, count: i32) -> Vec<CardInstance> {
    (0..count)
        .map(|_| {
            let card = new_instance(state, def_id, player, Zone::Hand { player });
            state.players[player].hand.push(card.clone());
            card
        })
        .collect()
}

/// The player's library, top first, as their own deck: each card known to its owner (R311) and
/// dealt, so not Created (R943), as the testkit scenario's setup deals its piles.
pub fn set_library(
    state: &mut GameState,
    player: PlayerId,
    def_ids: &[impl AsRef<str>],
) -> Vec<CardInstance> {
    let mut cards: Vec<CardInstance> = def_ids
        .iter()
        .map(|def_id| new_dealt_instance(state, def_id.as_ref(), player, Zone::Library { player }))
        .collect();
    for card in cards.iter_mut() {
        show_to_owner(card);
    }
    state.players[player].library = cards.clone();
    cards
}

/// `type_` is a `GameEventType` or its string (`"damage"`).
pub fn events_of_type(events: &[GameEvent], type_: impl ToString) -> Vec<GameEvent> {
    let wanted = type_.to_string();
    events
        .iter()
        .filter(|event| event.event_type().as_str() == wanted)
        .cloned()
        .collect()
}

pub fn events_json(events: &[GameEvent], kind: impl ToString) -> Vec<Value> {
    events_of_type(events, kind).into_iter().map(json_of).collect()
}

pub fn ids_of<C: Borrow<CardInstance>>(cards: &[C]) -> Vec<String> {
    cards.iter().map(|card| card.borrow().id.clone()).collect()
}

pub fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

pub fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    ids_of(&state.players[player].hand)
}

pub fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// `extra`'s keys replace `base`'s (the cards crate's `merged`); anything but two objects keeps `base`.
pub fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

thread_local! {
    /// The rules tests' one nonce counter (#1997). libtest runs each test on a thread of its own, so a
    /// test's nonces count `act-1`, `act-2`, … on every run.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

/// The calling test's next nonce: fresh within the test, so `reduce`'s dedupe (§9.3) never hands back
/// an earlier action's result in place of this one.
pub fn next_nonce() -> String {
    NONCE.with(|count| {
        count.set(count.get() + 1);
        format!("act-{}", count.get())
    })
}

/// `body` as an action with the next nonce. `body` is an `ActionInput` or its JSON literal, `playerId`
/// included; a nonce it carries is replaced.
pub fn next_action(body: impl Serialize) -> Action {
    json_as::<ActionInput>(json_of(body)).with_nonce(next_nonce())
}

pub fn act_result(state: &GameState, body: impl Serialize) -> ReduceResult {
    reduce(state, &next_action(body))
}

pub fn act_ok(state: &GameState, body: impl Serialize) -> ReduceResult {
    let action = next_action(body);
    accepted(&action, reduce(state, &action))
}

pub fn act(state: &GameState, body: impl Serialize) -> GameState {
    act_ok(state, body).state
}

/// `act_ok`, the action appended to `log`, for a test that replays what it played (§9.2).
pub fn act_logged(state: &GameState, body: impl Serialize, log: &mut Vec<Action>) -> ReduceResult {
    let action = next_action(body);
    let result = accepted(&action, reduce(state, &action));
    log.push(action);
    result
}

/// `act` with the nonce given, so two copies of one state sent the same action stay equal, `applied` and all.
pub fn act_with_nonce(state: &GameState, body: impl Serialize, nonce: &str) -> GameState {
    let action = json_as::<ActionInput>(json_of(body)).with_nonce(nonce);
    accepted(&action, reduce(state, &action)).state
}

fn accepted(action: &Action, result: ReduceResult) -> ReduceResult {
    if let Some(error) = &result.error {
        panic!("{} refused: {error}", action.action_type());
    }
    result
}

/// The card as it stands in `state` now, by its id or a copy of it.
pub fn live(state: &GameState, card: impl Into<CardRef>) -> CardInstance {
    let id = ref_id(card.into());
    find_instance(state, &id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

pub fn live_mut(state: &mut GameState, card: impl Into<CardRef>) -> &mut CardInstance {
    let id = ref_id(card.into());
    find_instance_mut(state, &id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn ref_id(card: CardRef) -> String {
    match card {
        CardRef::Text(id) | CardRef::Instance { id, .. } => id,
    }
}

/// One effect applied straight through its `apply` as `controller`'s, with no card of its own and no
/// settle, the rng's cursor written back; the events it emitted.
pub fn run_effect(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(controller),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// `run_effect` as p1's: the effect applied for p1 on a sink over `state`, the rng cursor written
/// back; its events.
pub fn run_as_p1(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    run_effect(state, effect, PlayerId::P1)
}

/// One effect applied straight through its `apply` under `options`, as `resolve.ts` does, with no
/// card of its own and no settle, the rng's cursor written back; the events it emitted.
pub fn run_with_options(state: &mut GameState, effect: Effect, options: HookOptions) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, options);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// TS `run(state, effect, self, targets, controller)`: one effect on a sink over `state`, the rng
/// cursor written back; its events.
pub fn run_with_targets(
    state: &mut GameState,
    effect: Effect,
    self_: Option<&CardInstance>,
    targets: Vec<Selection>,
    controller: PlayerId,
) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            self_,
            HookOptions {
                controller: Some(controller),
                targets: Some(targets),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// Apply one effect for p1 and settle, as a resolving card's would.
pub fn apply_now(state: &mut GameState, effect: Effect) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
}

/// TS `sinkFor(state)`: run `f` on a sink over `state` and hand back what it emitted.
pub fn run_on_sink<R>(
    state: &mut GameState,
    f: impl FnOnce(&mut EngineSink<'_>) -> R,
) -> (R, Vec<GameEvent>) {
    let mut sink = sink_for(state);
    let out = f(&mut sink);
    let events = sink.events.clone();
    (out, events)
}

/// Past the mulligans, both hands kept, in the main phase of p1's first turn.
pub fn past_the_mulligans(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            json_as::<ActionInput>(json!({ "type": "mulligan", "keep": keep, "playerId": player })),
        );
    }
    state
}

#[derive(Clone, Debug)]
pub struct RandomGame {
    pub state: GameState,
    pub log: Vec<Action>,
    pub decks: (Vec<String>, Vec<String>),
}

/// A full game played by the random policy of §10.7: uniform over the legal actions, ending the
/// turn when it is the only option or with AI_END_TURN_PROBABILITY. Concedes and draw offers are
/// left out, so a game ends by hero death or the turn cap (M1 gate).
pub fn play_random_game(seed: &str, deck_pair: Option<(Vec<String>, Vec<String>)>) -> RandomGame {
    let decks = deck_pair.unwrap_or_else(|| (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)));
    let mut state = begin_game(&new_game(seed, Some(decks.clone()))).state;
    let mut policy = Rng::new(&format!("policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();

    let mut step = 0;
    while state.result.is_none() {
        if step > 4000 {
            panic!("game {seed} did not finish");
        }
        let player = seat_to_act(&state).unwrap_or(state.active);
        let actions: Vec<ActionBody> = legal_actions(&state, player)
            .into_iter()
            .filter(|action| {
                !matches!(
                    action.action_type(),
                    ActionType::Concede | ActionType::OfferDraw | ActionType::AnswerDraw
                )
            })
            .collect();
        if actions.is_empty() {
            panic!("no legal action for {player} in game {seed}");
        }

        let others: Vec<&ActionBody> = actions
            .iter()
            .filter(|action| action.action_type() != ActionType::EndTurn)
            .collect();
        let end_turn = actions
            .iter()
            .find(|action| action.action_type() == ActionType::EndTurn);
        // The chance is drawn only when something besides ending the turn exists and ending it is offered.
        let take_end = others.is_empty() || (end_turn.is_some() && policy.chance(AI_END_TURN_PROBABILITY));
        let chosen: ActionBody = if take_end {
            match end_turn {
                Some(end) => end.clone(),
                None => others[policy.int(others.len() as i32) as usize].clone(),
            }
        } else {
            others[policy.int(others.len() as i32) as usize].clone()
        };

        let action = Action::new(chosen, player, format!("a{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{} rejected in {seed}: {error}", action.action_type());
        }
        log.push(action);
        state = result.state;
        step += 1;
    }

    RandomGame { state, log, decks }
}

pub fn json_of(value: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("serializes")
}

/// A value through JSON and back, as a paused state crosses a socket or a store (§10.1).
pub fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(json_of(value)).expect("parses back")
}

/// Every key the expected object names matches, recursively; an array matches element for element
/// and in length; anything else is equal.
pub fn matches_object(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
    use serde_json::Value;
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
