//! The turn loop and mana (M1-T6), and the end of a turn as a resumable sequence (§2.2, §9.3,
//! §10.6; R62, R113, R117, R122, R126, R127).
//!
//! Port of `packages/engine/test/turn.test.ts` (which has no header comment of its own; its two
//! halves are described where they begin, below).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, new_game, put, set_library, sink_for, slot};
use crate::rules::fixtures::scripts::{gravedigger, hinder, mana_well, shredder, x_bolt};

static NONCE: AtomicU32 = AtomicU32::new(0);

/// TS's module `let nonce`, bumped before each use.
fn next_nonce() -> u32 {
    NONCE.fetch_add(1, Ordering::SeqCst) + 1
}

fn action(body: Value) -> Action {
    json_as(body)
}

fn act(state: &GameState, body: Value) -> GameState {
    let nonce = next_nonce();
    let input: ActionInput = json_as(body);
    let result = reduce(state, &input.with_nonce(format!("t{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// `toMatch(/text/)` on an error that must be there.
fn says(error: &Option<String>, text: &str) -> bool {
    error.as_deref().is_some_and(|message| message.contains(text))
}

/// `eventsOfType(events, kind).map((event) => event[field])`, read through each event's JSON.
fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises")[field].clone())
        .collect()
}

/// `Array.prototype.indexOf`: -1 when absent.
fn index_of(order: &[&str], wanted: &str) -> i64 {
    order.iter().position(|entry| *entry == wanted).map_or(-1, |at| at as i64)
}

/// Past the mulligans, in the main phase of turn 1. (TS `playing(seed = "turn", decks?)`.)
fn playing(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    let mut state = begin_game(&new_game(seed, decks)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep = ids(&state.players.p2.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    state
}

fn end_turns(state: &GameState, count: i32) -> GameState {
    let mut next = state.clone();
    for _ in 0..count {
        next = act(&next, json!({ "type": "endTurn", "playerId": next.active }));
    }
    next
}

/// A modifier's `kind`, as TS reads `m.kind`.
fn kind_of(modifier: &PlayerModifier) -> Value {
    serde_json::to_value(modifier).expect("a modifier serialises")["kind"].clone()
}

mod turn_loop_and_mana_m1_t6 {
    use super::*;

    #[test]
    fn refreshes_1_mana_on_turn_1_4_by_the_fourth_turn_and_stays_at_4() {
        let mut state = playing("turn", None);
        assert_eq!(state.players.p1.mana.current, 1);
        assert_eq!(state.players.p1.mana.max, 1);

        state = end_turns(&state, 6); // p1's fourth turn
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(state.players.p1.turns_started, 4);
        assert_eq!(state.players.p1.mana.max, MAX_MANA);

        state = end_turns(&state, 12); // p1's tenth turn
        assert_eq!(state.players.p1.turns_started, 10);
        assert_eq!(state.players.p1.mana.max, MAX_MANA);
    }

    #[test]
    fn mana_well_adds_temporary_mana_above_the_refresh_6() {
        let mut state = playing("mana-well", None);
        put(&mut state, &mana_well().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        state = end_turns(&state, 6);
        assert_eq!(state.players.p1.mana.max, 4);
        assert_eq!(state.players.p1.mana.current, 5);
    }

    #[test]
    fn hinders_modifier_lowers_the_next_refresh_and_mana_never_goes_below_0() {
        let mut state = playing("hinder-mana", None);
        state = end_turns(&state, 1); // p2's first turn
        state.players.p2.mana.next_turn_mod = -1;
        state = end_turns(&state, 2); // back to p2, now their second turn
        assert_eq!(state.players.p2.turns_started, 2);
        // §2.3: the refresh is 1 lower; max mana is still min(turns, 4).
        assert_eq!(state.players.p2.mana.current, 1);
        assert_eq!(state.players.p2.mana.max, 2);

        state.players.p2.mana.next_turn_mod = -5;
        state = end_turns(&state, 2);
        assert_eq!(state.players.p2.mana.max, 3);
        assert_eq!(state.players.p2.mana.current, 0);
    }

    #[test]
    fn r30_clears_a_this_turn_modifier_at_cleanup_and_keeps_a_pending_echo() {
        let mut state = playing("mods", None);
        {
            let mut sink = sink_for(&mut state);
            let turn = sink.state.turn;
            add_modifier(
                &mut sink,
                PlayerId::P1,
                json_as(json!({ "until": "thisTurn", "turn": turn })),
                json_as(json!({ "kind": "costDiscount", "amount": 1 })),
            );
            add_modifier(
                &mut sink,
                PlayerId::P1,
                json_as(json!({ "until": "used" })),
                json_as(json!({ "kind": "echoNextSpell", "amount": 1 })),
            );
        }
        assert_eq!(state.players.p1.mods.len(), 2);

        state = end_turns(&state, 1);
        assert_eq!(
            state.players.p1.mods.iter().map(kind_of).collect::<Vec<_>>(),
            vec![json!("echoNextSpell")]
        );
    }

    #[test]
    fn fires_start_of_turn_triggers_before_the_draw_37_gravedigger() {
        let mut state = playing("gravedigger", None);
        put(&mut state, &gravedigger().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let buried = new_instance(&mut state, "fx-9", PlayerId::P1, Zone::Graveyard { player: PlayerId::P1 });
        state.players.p1.graveyard.push(buried.clone());
        state.players.p1.hand = vec![];

        state = end_turns(&state, 2); // p1's next turn: trigger, then draw
        let hand = ids(&state.players.p1.hand);
        assert_eq!(hand.first(), Some(&buried.id));
        assert_eq!(hand.len(), 2);
        assert!(!state.players.p1.graveyard.iter().any(|card| card.id == buried.id));
    }

    #[test]
    fn r348_refuses_an_x_above_current_mana_and_an_x_of_0() {
        let deck: Vec<String> =
            std::iter::once(x_bolt().id).chain(vanilla_deck(DECK_SIZE - 1, 1)).collect();
        let mut state = playing("x-cost", Some((deck.clone(), vanilla_deck(DECK_SIZE, 21))));
        let bolt = new_instance(&mut state, &x_bolt().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players.p1.hand.push(bolt.clone());

        let too_big = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": bolt.id, "x": 5, "playerId": "p1", "nonce": "x-big" })),
        );
        assert!(says(&too_big.error, "X is above your current mana"));

        // R348: X is at least 1 (`MIN_CHOSEN_X`), named or left out.
        let zero = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": bolt.id, "x": 0, "playerId": "p1", "nonce": "x-zero" })),
        );
        assert!(says(&zero.error, "X must be at least 1"));
        // TS `toBe(state)`: a refusal hands back the input itself (SURFACE §6.1: the input, unchanged).
        assert_eq!(zero.state, state);
        let none = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": bolt.id, "playerId": "p1", "nonce": "x-none" })),
        );
        assert!(says(&none.error, "X must be at least 1"));
        assert_eq!(
            legal_actions(&state, PlayerId::P1)
                .into_iter()
                .flat_map(|action| match action {
                    ActionBody::Play { instance_id, x, .. } if instance_id == bolt.id => vec![x],
                    _ => vec![],
                })
                .collect::<Vec<_>>(),
            vec![Some(1)]
        );

        state = playing("x-cost-2", Some((deck, vanilla_deck(DECK_SIZE, 21))));
        let bolt2 = new_instance(&mut state, &x_bolt().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players.p1.hand.push(bolt2.clone());
        let one = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": bolt2.id, "x": 1, "playerId": "p1", "nonce": "x-one" })),
        );
        assert_eq!(one.error, None);
        assert_eq!(one.state.players.p2.hero.health, 29);
        assert_eq!(one.state.players.p1.mana.current, 0);
    }

    #[test]
    fn r62_runs_a_turn_in_order_refresh_start_triggers_draw_then_end_of_turn() {
        let mut state = playing("r62", None);
        put(&mut state, &gravedigger().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let buried = new_instance(&mut state, "fx-9", PlayerId::P1, Zone::Graveyard { player: PlayerId::P1 });
        state.players.p1.graveyard.push(buried);

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        let nonce = next_nonce();
        let result = reduce(
            &state,
            &action(json!({ "type": "endTurn", "playerId": "p2", "nonce": format!("r62-{nonce}") })),
        );
        let order: Vec<&str> = result
            .events
            .iter()
            .map(|event| event.event_type().as_str())
            .filter(|kind| ["turnEnded", "turnStarted", "manaChanged", "addedToHand", "drawn"].contains(kind))
            .collect();

        // p2's turn ends, then p1's begins: mana, the start-of-turn trigger, then the draw.
        assert_eq!(order.first(), Some(&"turnEnded"));
        assert_eq!(order.get(1), Some(&"turnStarted"));
        assert_eq!(order.get(2), Some(&"manaChanged"));
        assert!(index_of(&order, "addedToHand") < index_of(&order, "drawn"));
    }

    #[test]
    fn r48_keeps_a_next_turn_modifier_through_its_own_turn_and_drops_it_after_the_next_one() {
        let mut state = playing("next-turn-expiry", None);
        {
            let mut sink = sink_for(&mut state);
            let turn = sink.state.turn;
            add_modifier(
                &mut sink,
                PlayerId::P1,
                json_as(json!({ "until": "nextTurnOf", "player": "p1", "fromTurn": turn })),
                json_as(json!({
                    "kind": "costDiscount",
                    "amount": 1,
                    "minCurrentCost": 4,
                })),
            );
        }

        state = end_turns(&state, 1); // p1's turn ended: it must survive cleanup
        assert_eq!(state.players.p1.mods.len(), 1);

        state = end_turns(&state, 2); // p1's next turn came and went
        assert_eq!(state.players.p1.mods.len(), 0);
    }

    #[test]
    fn fires_a_start_of_turn_trigger_on_its_controllers_turn_only_6_2() {
        let mut state = playing("own-turn-only", None);
        put(&mut state, &gravedigger().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let buried = new_instance(&mut state, "fx-9", PlayerId::P1, Zone::Graveyard { player: PlayerId::P1 });
        state.players.p1.graveyard.push(buried.clone());

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" })); // p2's turn starts
        assert_eq!(ids(&state.players.p1.graveyard), vec![buried.id.clone()]);

        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" })); // now p1's turn
        assert_eq!(state.players.p1.graveyard.len(), 0);
    }

    #[test]
    fn r68_resolves_two_end_of_turn_triggers_on_one_side_in_lane_order() {
        let mut state = playing("r68", None);
        let first = put(&mut state, &shredder().id, slot(PlayerId::P1, Row::Units, 1), Default::default());
        let second = put(&mut state, &shredder().id, slot(PlayerId::P1, Row::Units, 3), Default::default());

        let nonce = next_nonce();
        let result = reduce(
            &state,
            &action(json!({ "type": "endTurn", "playerId": "p1", "nonce": format!("r68-{nonce}") })),
        );
        let sources = field_of(&result.events, GameEventType::Damage, "sourceId");

        assert_eq!(sources.first(), Some(&json!(first.id)));
        assert_eq!(sources.last(), Some(&json!(second.id)));
        let seen: BTreeSet<Option<String>> =
            sources.iter().map(|source| source.as_str().map(str::to_owned)).collect();
        let wanted: BTreeSet<Option<String>> = [Some(first.id.clone()), Some(second.id.clone())].into_iter().collect();
        assert_eq!(seen, wanted);
    }

    #[test]
    fn r70_a_cast_counts_as_a_play_and_pays_nothing() {
        let mut state = playing("r70", None);
        let before = state.players.p1.turn_log.cards_played;
        set_library(&mut state, PlayerId::P1, &[hinder().id, "fx-2".to_string()]);
        let events = {
            let mut sink = sink_for(&mut state);
            jackioh_engine::draw::draw(&mut sink, PlayerId::P1, 1);
            sink.events.clone()
        };

        assert_eq!(state.players.p1.turn_log.cards_played, before + 1);
        assert_eq!(state.players.p1.turn_log.played_ids.len() as i32, before + 1);
        assert_eq!(state.counters.played, 1);
        let played = events_of_type(&events, GameEventType::CardPlayed);
        assert_eq!(played.len(), 1);
        assert_eq!(field_of(&events, GameEventType::CardPlayed, "costPaid").first(), Some(&json!(0)));
    }

    #[test]
    fn resets_exertion_at_the_controllers_next_turn_4_1() {
        let mut state = playing("exertion", None);
        let unit = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), Default::default());
        state = act(&state, json!({ "type": "switchPosition", "instanceId": unit.id, "playerId": "p1" }));
        let top = |state: &GameState| {
            state.players.p1.units[0].as_ref().and_then(|pile| pile.first()).map(|card| card.exertion.switched)
        };
        assert_eq!(top(&state), Some(true));
        assert!(
            !legal_actions(&state, PlayerId::P1)
                .iter()
                .any(|action| matches!(action, ActionBody::SwitchPosition { .. }))
        );

        state = end_turns(&state, 2);
        assert_eq!(top(&state), Some(false));
    }
}

// ---------------------------------------------------------------------------
// The end of a turn as a resumable sequence, and the two shapes a delayed continuation takes
// (§2.2, §9.3, §10.6; R62, R113, R117, R122, R126, R127).
//
// Fixtures are this file's own: defs are prefixed `tn-` and indexed from 2400, so they cannot
// collide with another test file's catalog (BUILD §0).
// ---------------------------------------------------------------------------

/// TS `fixtureDef(name, type)`; `index` is the one TS's `nextIndex` counter gave it (2401 on).
fn fixture_def(name: &str, type_: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("tn-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (turn)"),
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

/// The note sink: a Field Spell in p1's backrow lane 5 whose memory records the order things ran.
fn log_card() -> CardDef {
    fixture_def("log", "Field Spell", 2401)
}
/// R126: its continuation sits in the `resume` step table, the shape `runHook` could not reach.
fn table_card() -> CardDef {
    fixture_def("table", "Field Spell", 2402)
}
/// R127: its continuation is a `delayed` hook and it records whether it got a `ctx.self`.
fn self_card() -> CardDef {
    fixture_def("self", "Field Spell", 2403)
}
/// A delayed effect that asks its controller something, so the end of turn pauses inside it.
fn ask_card() -> CardDef {
    fixture_def("ask", "Field Spell", 2404)
}
/// An end-of-turn *trigger* that asks, so the end of turn pauses before the `turnEnded` event.
fn ask_hook_card() -> CardDef {
    fixture_def("askhook", "Field Spell", 2405)
}
/// The end-of-turn trigger behind it in R68's backrow order, which the pause must not skip.
fn second_hook_card() -> CardDef {
    fixture_def("secondhook", "Field Spell", 2406)
}

fn turn_defs() -> Vec<CardDef> {
    vec![log_card(), table_card(), self_card(), ask_card(), ask_hook_card(), second_hook_card()]
}

const NOTE_LANE: i32 = 5;
/// The step the two table-shaped continuations below name (§10.6: script id + step + data).
const STEP: &str = "bolt";

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state.players.p1.backrow[(NOTE_LANE - 1) as usize].as_ref()
}

fn note(name: &str) -> Effect {
    let name = name.to_string();
    Effect::new("tn:note", move |ctx| {
        let Some(log) = ctx.state.players.p1.backrow[(NOTE_LANE - 1) as usize].as_mut() else {
            return;
        };
        let mut steps: Vec<Value> = log.memory.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
        steps.push(json!(name));
        log.memory.insert("steps".to_string(), Value::Array(steps));
    })
}

fn notes(state: &GameState) -> Vec<String> {
    log_of(state)
        .and_then(|log| log.memory.get("steps"))
        .and_then(Value::as_array)
        .map(|steps| steps.iter().filter_map(|step| step.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// §10.6: a prompt for the delayed effect's own controller, with one answer, so answering is trivial.
fn ask_controller() -> Effect {
    Effect::new("tn:ask", |ctx| {
        let player = ctx.controller;
        let resume = resume_self(ctx, "asked", IndexMap::new());
        open_prompt(
            ctx,
            json_as(json!({
                "player": player,
                "kind": "target",
                "prompt": "the delayed effect asks its owner",
                "options": [{ "key": "none", "label": "nothing", "selection": { "pick": "none" } }],
                "resume": resume,
            })),
        );
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// `String(ctx.data.amount)`.
fn js_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

fn turn_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();

    let mut table_resume: IndexMap<&'static str, Hook> = IndexMap::new();
    table_resume.insert(
        STEP,
        hook(|ctx| vec![note(&format!("table:{}", js_string(ctx.data.get("amount"))))]),
    );
    scripts.insert(table_card().id, both(Script { resume: table_resume, ..Script::default() }));

    scripts.insert(
        self_card().id,
        both(Script {
            delayed: Some(hook(|ctx| vec![note(if ctx.self_.is_none() { "noSelf" } else { "self" })])),
            ..Script::default()
        }),
    );

    let asked = || {
        let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
        resume.insert("asked", hook(|_ctx| vec![note("answered")]));
        resume
    };
    scripts.insert(
        ask_card().id,
        both(Script {
            delayed: Some(hook(|_ctx| vec![note("ask1"), ask_controller()])),
            resume: asked(),
            ..Script::default()
        }),
    );
    scripts.insert(
        ask_hook_card().id,
        both(Script {
            end_of_turn: Some(hook(|_ctx| vec![note("trigger1"), ask_controller()])),
            resume: asked(),
            ..Script::default()
        }),
    );
    scripts.insert(
        second_hook_card().id,
        both(Script {
            end_of_turn: Some(hook(|_ctx| vec![note("trigger2")])),
            ..Script::default()
        }),
    );
    scripts
}

/// Past the mulligans, in p1's main phase, with the note log parked in p1's backrow lane 5.
fn turn_game(seed: &str) -> GameState {
    let fresh = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in turn_defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(turn_scripts());
    register_scripts(scripts);
    let mut state = begin_game(&fresh).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep = ids(&state.players.p2.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    put(&mut state, &log_card().id, slot(PlayerId::P1, Row::Backrow, NOTE_LANE), Default::default());
    state
}

/// A `Resume` literal: `{ defId, hook, step, radiant: false, instanceId?, data }`.
fn resume(def_id: &str, hook: &str, step: &str, instance_id: Option<&str>, data: Value) -> Resume {
    Resume {
        def_id: def_id.to_string(),
        hook: hook.to_string(),
        step: step.to_string(),
        radiant: false,
        instance_id: instance_id.map(str::to_owned),
        data: json_as(data),
    }
}

/// Schedule one end-of-turn delayed effect for p1, the way `effects/delay.ts` stores one.
fn delay_at_end_of_p1(state: &mut GameState, resume: Resume) -> String {
    let mut sink = sink_for(state);
    schedule_delayed(
        &mut sink,
        PlayerId::P1,
        DelayedAt {
            phase: Phase::End,
            player: PlayerId::P1,
        },
        resume,
        None,
        None,
    )
    .id
}

fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = next_nonce();
    let input: ActionInput = json_as(body);
    reduce(state, &input.with_nonce(format!("t{nonce}")))
}

fn answer_result(state: &GameState) -> ReduceResult {
    let pending = state.pending.as_ref().expect("expected a prompt to be open");
    act_result(
        state,
        json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }], "playerId": pending.player_id }),
    )
}

fn answer_open_prompt(state: &GameState) -> GameState {
    let result = answer_result(state);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn only<T: Clone>(items: &[T]) -> T {
    items.first().cloned().expect("expected at least one item")
}

fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).expect("serialises")).expect("deserialises")
}

mod r62_r113_r117_r126_r127_delayed_continuations_and_the_end_of_turn_2_2_10_6 {
    use super::*;

    #[test]
    fn r126_re_enters_a_delayed_continuation_that_lives_in_the_resume_step_table_not_only_a_delayed_hook() {
        let mut state = turn_game("r126-step-table");
        let scheduler = put(&mut state, &table_card().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        // The shape `resolve.runHook` could not read: `hook` is the step table, `step` picks the entry.
        // It used to fetch `script.resume` — an object — and call it, which threw.
        delay_at_end_of_p1(
            &mut state,
            resume(&table_card().id, RESUME_HOOK, STEP, Some(&scheduler.id), json!({ "amount": 4 })),
        );

        let ended = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        assert_eq!(notes(&ended), vec!["table:4"]);
        // R62/§10.1: the entry is dropped once it has resolved, so it never fires twice.
        assert!(ended.delayed.is_empty());
        // R117: nothing was owed, because nothing paused — the remainder is only parked at a pause.
        assert!(ended.work.is_empty());
    }

    #[test]
    fn r127_resolves_a_delayed_continuation_whose_instance_is_gone_with_ctx_self_null() {
        let mut state = turn_game("r127-no-instance");
        // Two entries R76 and §10.6 allow and the old reader dropped in silence: one that never had an
        // instance to name, and one whose instance has ceased to exist (#39 exiles itself, #50 dies).
        delay_at_end_of_p1(&mut state, resume(&self_card().id, "delayed", "", None, json!({})));
        delay_at_end_of_p1(
            &mut state,
            resume(&self_card().id, "delayed", "", Some("tn-instance-that-never-was"), json!({})),
        );

        let ended = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        // Both resolved, both with `ctx.self === null`, and R113's "never dropped in silence" holds.
        assert_eq!(notes(&ended), vec!["noSelf", "noSelf"]);
        assert!(ended.delayed.is_empty());
    }

    #[test]
    fn r113_r122_parks_the_rest_of_the_end_of_turn_when_a_delayed_effect_prompts_and_r122s_answer_finishes_it() {
        let mut state = turn_game("r113-end-of-turn-remainder");
        let asking = put(&mut state, &ask_card().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        let scheduler = put(&mut state, &table_card().id, slot(PlayerId::P1, Row::Backrow, 2), Default::default());
        delay_at_end_of_p1(&mut state, resume(&ask_card().id, "delayed", "", Some(&asking.id), json!({})));
        // R68's creation order: this one is due at the same point and waits for the first to finish.
        let second = delay_at_end_of_p1(
            &mut state,
            resume(&table_card().id, RESUME_HOOK, STEP, Some(&scheduler.id), json!({ "amount": 2 })),
        );

        let paused = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));

        assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P1));
        assert_eq!(notes(&paused), vec!["ask1"]);
        // Nothing R62 puts after the pause has happened: the next entry is still due, cleanup has not
        // closed p1's turn log, and p2's turn has not started under the open prompt.
        assert_eq!(
            paused.delayed.iter().map(|effect| effect.id.clone()).collect::<Vec<_>>(),
            vec![second]
        );
        assert_eq!(paused.players.p1.turn_log.unspent_at_end, None);
        assert_eq!(paused.active, PlayerId::P1);

        // R113: the rest of the end of turn is *owed*, as plain JSON, and `work.ts` knows the hook —
        // the handler is registered at module scope by `turn.ts`, not by this test.
        let parked = only(&owed_work(&paused, Some(END_OF_TURN_WORK)));
        assert_eq!(round_trip(&parked), parked);
        assert!(can_resume(&paused, &parked.resume));

        // §10.1: the paused game survives a round trip and resumes from the round-tripped copy.
        let round = round_trip(&paused);
        let answered = answer_open_prompt(&round);

        // R122: the action that answered finishes what the prompt interrupted — the answered step, the
        // delayed entry behind it, then cleanup and the next turn.
        assert_eq!(notes(&answered), vec!["ask1", "answered", "table:2"]);
        assert!(answered.delayed.is_empty());
        assert!(answered.players.p1.turn_log.unspent_at_end.is_some());
        assert!(answered.work.is_empty());
        assert_eq!(answered.pending, None);
        assert_eq!(answered.active, PlayerId::P2);
    }

    #[test]
    fn r62_finishes_the_end_of_turn_triggers_before_the_turn_ended_event_when_one_of_them_prompts() {
        let mut state = turn_game("r62-trigger-pause");
        put(&mut state, &ask_hook_card().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        put(&mut state, &second_hook_card().id, slot(PlayerId::P1, Row::Backrow, 2), Default::default());
        let scheduler = put(&mut state, &table_card().id, slot(PlayerId::P1, Row::Backrow, 3), Default::default());
        delay_at_end_of_p1(
            &mut state,
            resume(&table_card().id, RESUME_HOOK, STEP, Some(&scheduler.id), json!({ "amount": 2 })),
        );

        let ended = act_result(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(ended.error, None);
        let paused = ended.state.clone();

        // The first trigger asked, so nothing R62 puts after the trigger queue has run — the
        // `turnEnded` event of the window included, which #18 Bread and Butter reads (R100).
        assert_eq!(notes(&paused), vec!["trigger1"]);
        assert!(events_of_type(&ended.events, GameEventType::TurnEnded).is_empty());
        assert_eq!(paused.players.p1.turn_log.unspent_at_end, None);
        assert_eq!(paused.active, PlayerId::P1);
        assert_eq!(owed_work(&paused, Some(END_OF_TURN_WORK)).len(), 1);

        let resumed = answer_result(&paused);
        assert_eq!(resumed.error, None);

        // R62's whole order across the pause: the queued triggers first, then the window's event, then
        // the delayed effects, then cleanup and the next turn — and the event is emitted exactly once.
        assert_eq!(notes(&resumed.state), vec!["trigger1", "answered", "trigger2", "table:2"]);
        assert_eq!(field_of(&resumed.events, GameEventType::TurnEnded, "player"), vec![json!("p1")]);
        assert!(resumed.state.players.p1.turn_log.unspent_at_end.is_some());
        assert!(resumed.state.delayed.is_empty());
        assert!(resumed.state.work.is_empty());
        assert_eq!(resumed.state.active, PlayerId::P2);
    }
}
