//! The two `modifiers.ts` verbs: `delay` and `addPlayerModifier`
//! (SPEC §2.2, §2.3, §6.3 Cost/Mana, §10.1, §10.6; R30, R48, R62, R68, R76, R86, R126; BUILD
//! M3-T1/M3-T5).
//!
//! `test/modifiers.test.ts` already proves the SUBSYSTEM — `scheduleDelayed`, `dueDelayed`,
//! `addModifier`, `expireModifiers` and R62's two points in the turn loop. This file proves the two
//! EFFECTS in front of it: that what a card file writes lands as the record the subsystem expects,
//! and that the record really comes back at the turn boundary running the card's own script.
//!
//! It also pins ONE remaining gap in `effects/delay.ts`, which is NOT part of this work. It is
//! marked `it.fails` (here `#[should_panic]`), so the runner reports it green while it is broken
//! and turns RED the moment it is fixed — which is the point: a known-failing test naming a real gap
//! is worth more than a passing tautology, and it cannot be forgotten. It carries its fix in a
//! comment. The gap that used to sit beside it — `turn.runDelayed` being unable to re-enter a
//! `resume` step table — is closed, and R126 now states the rule it was missing; its test is in
//! "delay: coming due" below.
//!
//! The fixture cards are registered here on top of the shared fixture catalog, so no shared fixture
//! has to grow for them (CLAUDE.md, BUILD §0).
//!
//! Port of `packages/engine/test/effects-delay.test.ts`.

use std::cell::Cell;

use jackioh_engine::effects::{DELAYED_HOOK, THIS_TURN, add_player_modifier, damage, delay};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::Units,
};

use super::fixtures::combat::indestructible;
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, sink_for, slot};

// ---------------------------------------------------------------------------
// Fixture cards. Indices start above 1500 so they never collide with another test file's locals.
// ---------------------------------------------------------------------------

/// TS `unitDefOf(name, overrides)`, whose module counter handed out 1501 to the first (and only) def;
/// the index is passed in here, since a test file keeps no mutable statics.
fn unit_def_of(name: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": format!("dl-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (delay)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 9, "keywords": [], "text": name },
        "radiant": { "attack": 2, "health": 18, "keywords": [], "text": format!("{name} radiant") },
    }))
}

/// The step every delay below names (§10.6: "script id + step + captured data").
const BOLT_STEP: &str = "bolt";

/// `data.amount` to the enemy hero, so the captured data is observable in hero health.
fn bolt_hook() -> Hook {
    hook(|ctx| {
        let amount = ctx.data.get("amount").and_then(Value::as_i64).unwrap_or(0) as i32;
        vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
    })
}

/// A body that registers the same continuation BOTH ways: as its own `delayed` hook (the shape
/// `turn.runDelayed` can re-enter today) and as an entry in its `resume` step table (the shape
/// `prompts.runResume` and `work.cardStepFor` understand). That is what lets one fixture prove the
/// first spelling works and the second does not, with nothing else different between the two tests.
fn bolt_script() -> Script {
    Script {
        delayed: Some(bolt_hook()),
        resume: [(BOLT_STEP, bolt_hook())].into_iter().collect(),
        ..Script::default()
    }
}

fn bolt() -> CardDef {
    unit_def_of("bolt", 1501)
}

fn dl_defs() -> Vec<CardDef> {
    vec![bolt()]
}

fn dl_scripts() -> IndexMap<String, CardScripts> {
    [(bolt().id, CardScripts { base: bolt_script(), radiant: bolt_script() })].into_iter().collect()
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

thread_local! {
    /// TS's module `let nonce = 0`: one counter per test thread.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    let nonce = NONCE.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let result = reduce(state, &body.with_nonce(format!("dl{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn end_turns(state: &GameState, count: usize) -> GameState {
    let mut next = state.clone();
    for _ in 0..count {
        let active = next.active;
        next = act(&next, json_as(json!({ "type": "endTurn", "playerId": active })));
    }
    next
}

/// Past the mulligans, in p1's main phase of turn 1, with the local defs folded in.
fn playing(seed: &str) -> GameState {
    let fresh = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in dl_defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(dl_scripts());
    register_scripts(scripts);
    let mut state = begin_game(&fresh).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json_as(json!({ "type": "mulligan", "keep": keep, "playerId": "p1" })));
    let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json_as(json!({ "type": "mulligan", "keep": keep, "playerId": "p2" })));
    state
}

/// TS `run`'s options: `{ self?: CardInstance | null; controller?: "p1" | "p2" }`.
#[derive(Default)]
struct RunOptions {
    self_: Option<CardInstance>,
    controller: Option<PlayerId>,
}

/// Apply effects the way `resolve.ts` does, straight onto `state`, and keep the events (TS handed
/// back the sink; its `events` are what the tests read).
fn run(state: &mut GameState, effects: Vec<Effect>, options: RunOptions) -> Vec<GameEvent> {
    let RunOptions { self_, controller } = options;
    let self_ = self_.map(|card| find_instance(state, &card.id).cloned().unwrap_or(card));
    let mut sink = sink_for(state);
    {
        let mut ctx = make_context(sink.sink(), self_, HookOptions { controller, ..Default::default() });
        apply_effects(&effects, &mut ctx);
    }
    let cursor = sink.rng.cursor();
    let events = std::mem::take(&mut sink.events);
    state.rng_cursor = cursor;
    events
}

/// `noUncheckedIndexedAccess` makes every index a maybe; these two narrow it once.
fn only(entries: &[DelayedEffect]) -> &DelayedEffect {
    entries.first().expect("expected one delayed effect")
}

fn one(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("expected a card")
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively;
/// arrays match element for element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

fn assert_matches_object<T: serde::Serialize>(actual: &T, expected: Value) {
    let actual = json_of(actual);
    assert!(matches_object(&actual, &expected), "{actual} does not match {expected}");
}

fn as_scribe(scribe: &CardInstance) -> RunOptions {
    RunOptions { self_: Some(scribe.clone()), controller: Some(P1) }
}

// ---------------------------------------------------------------------------
// delay
// ---------------------------------------------------------------------------

mod delay_scheduling_s10_1_s10_6_r62_r68 {
    use super::*;

    #[test]
    fn r62_stores_one_entry_at_the_named_phase_and_player_owned_by_the_controller_carrying_the_captured_data() {
        let mut state = playing("delay-schedule");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());

        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 3 } })))],
            as_scribe(&scribe),
        );

        assert_eq!(state.delayed.len(), 1);
        assert_matches_object(
            only(&state.delayed),
            json!({
                "owner": "p1",
                "at": { "phase": "end", "player": "p1" },
                "resume": {
                    "defId": bolt().id,
                    "hook": DELAYED_HOOK,
                    "step": BOLT_STEP,
                    "radiant": false,
                    "instanceId": scribe.id,
                    "data": { "amount": 3 },
                },
            }),
        );
        // §10.1 keeps the state JSON-only, so the stored continuation is data and not a closure.
        let round_tripped: Vec<DelayedEffect> =
            serde_json::from_value(json_of(&state.delayed)).expect("the delayed list is JSON");
        assert_eq!(round_tripped, state.delayed);
    }

    #[test]
    fn s10_6_reads_at_player_as_a_player_spec_so_enemy_waits_for_the_opponents_boundary() {
        let mut state = playing("delay-enemy");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());

        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "start", "player": "enemy" }, "step": BOLT_STEP })))],
            as_scribe(&scribe),
        );

        // The owner is still the scheduler — it is whose sequence resolves — but the boundary is p2's.
        assert_matches_object(only(&state.delayed), json!({ "owner": "p1", "at": { "phase": "start", "player": "p2" } }));
        assert!(due_delayed(&state, Phase::Start, P1).is_empty());
        assert_eq!(due_delayed(&state, Phase::Start, P2).len(), 1);
    }

    #[test]
    fn r350_this_turn_waits_for_the_end_of_the_turn_that_is_running_whoever_s_it_is_and_r241_does_not_drop_it() {
        let mut state = playing("delay-this-turn");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());
        assert_eq!(state.active, P1);

        // Made by p1 on p2's turn: an end-of-turn clause of p1's own would be dropped (R241), but "the
        // end of this turn" is p2's turn end.
        state.active = P2;
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": THIS_TURN }, "step": BOLT_STEP, "data": { "amount": 2 } })))],
            as_scribe(&scribe),
        );
        assert_matches_object(only(&state.delayed), json!({ "owner": "p1", "at": { "phase": "end", "player": "p2" } }));

        // Made by p1 on its own turn, it is p1's turn end.
        state.delayed = vec![];
        state.active = P1;
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": THIS_TURN }, "step": BOLT_STEP, "data": { "amount": 2 } })))],
            as_scribe(&scribe),
        );
        assert_matches_object(only(&state.delayed), json!({ "owner": "p1", "at": { "phase": "end", "player": "p1" } }));

        // And it comes due at that turn's end: 2 to p2's hero, as the turn ends.
        let after = end_turns(&state, 1);
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 2);
        assert!(after.delayed.is_empty());
    }

    #[test]
    fn s5_2_records_the_face_that_is_running_so_a_radiant_scheduler_resumes_its_radiant_text() {
        let mut state = playing("delay-radiant");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 2), json_as(json!({ "radiant": true })));

        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP })))],
            as_scribe(&scribe),
        );

        assert!(only(&state.delayed).resume.radiant);
    }

    #[test]
    fn r68_keeps_two_delays_scheduled_in_one_effect_list_in_creation_order_by_seq() {
        let mut state = playing("delay-order");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());

        run(
            &mut state,
            vec![
                delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 1 } }))),
                delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 4 } }))),
            ],
            as_scribe(&scribe),
        );

        let first = state.delayed[0].clone();
        let second = state.delayed[1].clone();
        assert!(first.seq < second.seq);
        assert_ne!(first.id, second.id);

        // R68 is "the order they were created", not the order the array happens to hold them in.
        state.delayed.reverse();
        let amounts: Vec<Option<Value>> = due_delayed(&state, Phase::End, P1)
            .iter()
            .map(|entry| entry.resume.data.get("amount").cloned())
            .collect();
        assert_eq!(amounts, vec![Some(json!(1)), Some(json!(4))]);
    }
}

mod delay_coming_due_s2_2_r62_r76_r86_r126 {
    use super::*;

    #[test]
    fn r62_the_hook_delayed_form_round_trips_a_real_end_of_turn_boundary_with_its_captured_data() {
        let mut state = playing("delay-round-trip");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 3 } })))],
            as_scribe(&scribe),
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        let ended = end_turns(&state, 1);

        // The card's own `delayed` hook ran, reading the amount the continuation carried.
        assert_eq!(ended.players.p2.hero.health, HERO_HEALTH - 3);
        // R62/§10.1: the entry is dropped once it has resolved, so it never fires twice.
        assert!(ended.delayed.is_empty());
    }

    /// R126's other spelling, on the same fixture: `bolt_script` registers the identical continuation
    /// under BOTH keys — its own `delayed` hook (the test above) and an entry in its `resume` step
    /// table (this one) — so the two tests differ in nothing but `resume.hook`.
    ///
    /// This was a gap: `turn.runDelayed` re-entered with `resolve.runHook`, which does `script[name]`
    /// and CALLS the result, so `hook: "resume"` threw "hook is not a function" instead of looking
    /// `resume.step` up in the table, and a card whose continuation sat only in `resume` resolved to
    /// nothing at all. R126 settles it — "a card must never have to register one continuation under
    /// two keys" — and `runDelayed` now goes through `prompts.runResume`, the one reader that
    /// resolves both shapes.
    #[test]
    fn r126_the_step_table_form_re_enters_too_one_reader_resolves_a_hook_or_a_resume_step() {
        let mut state = playing("delay-step-table");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());
        run(
            &mut state,
            vec![delay(json_as(json!({
                "at": { "phase": "end", "player": "self" },
                "step": BOLT_STEP,
                "hook": RESUME_HOOK,
                "data": { "amount": 4 },
            })))],
            as_scribe(&scribe),
        );
        assert_eq!(only(&state.delayed).resume.hook, RESUME_HOOK);

        // The fixture's `resume` table holds exactly this step, and the reader looks it up there.
        let ended = end_turns(&state, 1);
        assert_eq!(ended.players.p2.hero.health, HERO_HEALTH - 4);
        assert!(ended.delayed.is_empty());
    }

    #[test]
    fn r62_a_start_of_turn_delay_fires_at_its_own_controllers_next_start_not_the_opponents() {
        let mut state = playing("delay-start");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "start", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 2 } })))],
            as_scribe(&scribe),
        );

        // p1 ends; p2's whole turn passes with the entry still waiting.
        let after_p1 = end_turns(&state, 1);
        assert_eq!(after_p1.active, P2);
        assert_eq!(after_p1.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(after_p1.delayed.len(), 1);

        let back_to_p1 = end_turns(&after_p1, 1);
        assert_eq!(back_to_p1.active, P1);
        assert_eq!(back_to_p1.players.p2.hero.health, HERO_HEALTH - 2);
        assert!(back_to_p1.delayed.is_empty());
    }

    #[test]
    fn r76_c50_still_fires_after_its_scheduler_has_died_the_continuation_is_found_in_the_graveyard() {
        let mut state = playing("delay-graveyard");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 1), Default::default());
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "start", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 5 } })))],
            as_scribe(&scribe),
        );

        // "Fires even if K-Pop Fanatic died" (§8.2 #50, R76).
        let moving = find_instance(&state, &scribe.id).expect("the scribe").clone();
        move_to_zone(&mut state, &moving, OffFieldZone::Graveyard, Default::default());
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == scribe.id));

        let back_to_p1 = end_turns(&state, 2);
        assert_eq!(back_to_p1.players.p2.hero.health, HERO_HEALTH - 5);
        assert!(back_to_p1.delayed.is_empty());
    }

    #[test]
    fn r86_c39_still_fires_after_its_scheduler_has_exiled_itself_in_the_same_effect_list() {
        let mut state = playing("delay-exile");
        let scribe = put(&mut state, &bolt().id, slot(P1, Units, 3), Default::default());
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 6 } })))],
            as_scribe(&scribe),
        );

        // #39 Recycling Initiative arms the delay and then exiles itself, so the continuation has to
        // come back to a card in the exile pile.
        let moving = find_instance(&state, &scribe.id).expect("the scribe").clone();
        move_to_zone(&mut state, &moving, OffFieldZone::Exile, Default::default());
        assert!(state.players.p1.exile.iter().any(|card| card.id == scribe.id));

        let ended = end_turns(&state, 1);
        assert_eq!(ended.players.p2.hero.health, HERO_HEALTH - 6);
        assert!(ended.delayed.is_empty());
    }

    #[test]
    fn records_a_continuation_with_no_instance_when_the_scheduler_has_no_ctx_self() {
        let mut state = playing("delay-no-self");

        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 7 } })))],
            RunOptions { self_: None, controller: Some(P1) },
        );

        // §10.6: `resumeSelf` leaves `instanceId` out when there is no instance, which is the shape
        // R76 needs. `defId` is empty for the same reason — a null self cannot name its own script.
        let entry = only(&state.delayed);
        assert_eq!(entry.resume.instance_id, None);
        assert_eq!(entry.resume.def_id, "");
        assert_eq!(json_of(&entry.resume.data), json!({ "amount": 7 }));
        // The scheduling half is complete: it is stored, it is owned, and it is due.
        assert_eq!(due_delayed(&state, Phase::End, P1).len(), 1);
    }
}

mod delay_the_remaining_gap_in_effects_delay_ts_not_part_of_this_work {
    use super::*;

    /// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    /// GAP — A DELAY SCHEDULED WITH NO `ctx.self` HAS NO SCRIPT TO RE-ENTER.
    ///
    /// The engine half of this is fixed: `turn.runDelayed` no longer drops an entry whose
    /// `resume.instanceId` is missing, and `prompts.runResume` resumes a continuation whose instance
    /// has ceased to exist with `ctx.self === null` (R127). What is left is in THIS directory:
    /// `prompts.resumeSelf` can only name the def id through `ctx.self`, so a scheduler with a null
    /// self stores `defId: ""` — the passing "records a continuation with no instance" test above
    /// pins exactly that — and `runResume` then finds no face, no hook and nothing to run.
    ///
    /// THE FIX, in `crates/engine/src/effects/delay.rs` (TS `packages/engine/src/effects/delay.ts`):
    /// give `delay` a `defId` option that the card names when it has no instance to speak for it, and
    /// pass it into the stored `Resume` in place of `resumeSelf`'s `ctx.self.defId`. No Core card is in
    /// that position — every card that delays is on the field or parked in `resolving` when it does —
    /// so this stays reported, not built, and this file does not own `src/`.
    ///
    /// THIS TEST IS `it.fails` (`#[should_panic]`): it passes while the gap is open and turns RED the
    /// moment the option lands — delete the `#[should_panic]` then.
    /// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    #[test]
    #[should_panic]
    fn gap_r76_a_delay_scheduled_with_no_ctx_self_stores_no_def_id_so_nothing_re_enters() {
        let mut state = playing("delay-no-self-fires");
        run(
            &mut state,
            vec![delay(json_as(json!({ "at": { "phase": "end", "player": "self" }, "step": BOLT_STEP, "data": { "amount": 7 } })))],
            RunOptions { self_: None, controller: Some(P1) },
        );

        let ended = end_turns(&state, 1);
        assert_eq!(ended.players.p2.hero.health, HERO_HEALTH - 7);
    }
}

// ---------------------------------------------------------------------------
// addPlayerModifier
// ---------------------------------------------------------------------------

/// The five shapes the Core cards really pass, each named by the card that passes it.
fn five_real_shapes(turn: i32) -> Vec<Effect> {
    vec![
        // #35 Lunar Eclipse: "the next Spell you play this turn costs 1 less".
        add_player_modifier(json_as(json!({
            "player": "self",
            "mod": {
                "kind": "costDiscount",
                "amount": 1,
                "onlyType": "Spell",
                "oncePerTurn": true,
                "expiry": { "until": "thisTurn", "turn": turn },
            },
        }))),
        // #77 Professor Curvature: "your 4-cost cards cost 1 less through your next turn" (R48).
        add_player_modifier(json_as(json!({
            "player": "self",
            "mod": {
                "kind": "costDiscount",
                "amount": 1,
                "minCurrentCost": 4,
                "expiry": { "until": "nextTurnOf", "player": "p1", "fromTurn": turn },
            },
        }))),
        // #78 /fullsend: "your cards cost 1 less this turn" and "gain Combo: draw 1".
        add_player_modifier(json_as(json!({
            "player": "self",
            "mod": { "kind": "costDiscount", "amount": 1, "expiry": { "until": "thisTurn", "turn": turn } },
        }))),
        add_player_modifier(json_as(json!({
            "player": "self",
            "mod": { "kind": "comboDraw", "amount": 1, "expiry": { "until": "thisTurn", "turn": turn } },
        }))),
        // #79 Twinspell: "your next spell resolves one more time" — not turn-scoped (§2.2).
        add_player_modifier(json_as(json!({
            "player": "self",
            "mod": { "kind": "echoNextSpell", "amount": 1, "sourceId": "c-twinspell", "expiry": { "until": "used" } },
        }))),
    ]
}

/// `mod.kind`: the modifier's tag, as TS wrote it.
fn kind_of(modifier: &PlayerModifier) -> String {
    json_of(modifier)["kind"].as_str().expect("a modifier kind").to_string()
}

mod add_player_modifier_s2_2_s2_3_s6_3_cost_r30_r48_r65 {
    use super::*;

    #[test]
    fn s10_1_puts_each_of_the_five_real_card_shapes_on_the_named_player_with_an_id_and_a_modifier_changed_event() {
        let mut state = playing("player-mods");
        let turn = state.turn;
        let events = run(&mut state, five_real_shapes(turn), RunOptions { controller: Some(P1), ..Default::default() });

        let mods = state.players.p1.mods.clone();
        assert_eq!(mods.len(), 5);
        // Nothing lands on the opponent: `playerOf` resolved "self" against the controller.
        assert!(state.players.p2.mods.is_empty());

        // `modifiers.addModifier` assigns the id, so every modifier has a distinct one (§9.3: it is
        // derived from `nextSeq`, which makes it deterministic under replay).
        let ids: Vec<String> = mods.iter().map(|m| m.id.clone()).collect();
        assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 5);
        assert!(ids.iter().all(|id| !id.is_empty()));

        // One `modifierChanged` per modifier, in the order the effect list applied them (§10.3).
        let expected: Vec<Value> = ids
            .iter()
            .map(|id| json!({ "type": "modifierChanged", "player": "p1", "modifierId": id, "added": true }))
            .collect();
        assert_eq!(json_of(&events_of_type(&events, GameEventType::ModifierChanged)), Value::Array(expected));

        // The shapes survive as written, `id` apart, so `mana.ts` reads back what the card meant.
        let kinds: Vec<String> = mods.iter().map(kind_of).collect();
        assert_eq!(kinds, vec!["costDiscount", "costDiscount", "costDiscount", "comboDraw", "echoNextSpell"]);
        // R48: every shape is live now except Curvature's, which covers the controller's NEXT turn.
        let live: Vec<bool> = mods.iter().map(|m| modifier_is_live(&state, m)).collect();
        assert_eq!(live, vec![true, false, true, true, true]);
    }

    #[test]
    fn s6_3_cost_the_discount_it_installs_really_discounts_and_enemy_installs_it_on_the_opponent() {
        let mut state = playing("player-mods-cost");
        let mine = one(in_hand(&mut state, &indestructible().id, P1, 1));
        let theirs = one(in_hand(&mut state, &indestructible().id, P2, 1));
        let cost = |state: &GameState, card: &CardInstance| {
            effective_cost(state, find_instance(state, &card.id).expect("a hand card"))
        };
        assert_eq!(cost(&state, &mine), 4);

        let turn = state.turn;
        run(
            &mut state,
            vec![add_player_modifier(json_as(json!({
                "player": "self",
                "mod": { "kind": "costDiscount", "amount": 1, "expiry": { "until": "thisTurn", "turn": turn } },
            })))],
            RunOptions { controller: Some(P1), ..Default::default() },
        );

        // R65: a player discount is part of `effectiveCost`, which is the only place cost is computed.
        assert_eq!(cost(&state, &mine), 3);
        assert_eq!(cost(&state, &theirs), 4);

        run(
            &mut state,
            vec![add_player_modifier(json_as(json!({
                "player": "enemy",
                "mod": { "kind": "costDiscount", "amount": 2, "expiry": { "until": "thisTurn", "turn": turn } },
            })))],
            RunOptions { controller: Some(P1), ..Default::default() },
        );

        assert_eq!(state.players.p2.mods.len(), 1);
        assert_eq!(cost(&state, &theirs), 2);
        assert_eq!(cost(&state, &mine), 3);
    }

    #[test]
    fn s2_2_cleanup_takes_the_this_turn_modifiers_at_that_players_own_turn_end_and_leaves_until_used_standing() {
        let mut state = playing("player-mods-expiry");
        let turn = state.turn;
        let mut events = run(&mut state, five_real_shapes(turn), RunOptions { controller: Some(P1), ..Default::default() });
        let before: Vec<PlayerModifier> = state.players.p1.mods.clone();

        // §2.2's cleanup, which `turn.ts` runs at the end of this player's turn. TS ran it on the same
        // sink, so its events follow the run's on one list.
        {
            let mut sink = sink_for(&mut state);
            expire_modifiers(&mut sink.sink(), P1);
            events.extend(std::mem::take(&mut sink.events));
        }

        let kinds: Vec<String> = state.players.p1.mods.iter().map(kind_of).collect();
        // The three `thisTurn` modifiers are gone: two costDiscounts and the comboDraw.
        assert_eq!(kinds, vec!["costDiscount", "echoNextSpell"]);
        // R48: the next-turn discount survives the turn it was created on.
        let survivor = state.players.p1.mods.iter().find(|m| kind_of(m) == "costDiscount");
        assert_eq!(
            survivor.map(|m| m.expiry.clone()),
            Some(ModifierExpiry::NextTurnOf { player: P1, from_turn: state.turn })
        );
        // §2.2: "Twinspell's pending Echo is not turn-scoped and survives cleanup."
        assert_eq!(
            state.players.p1.mods.iter().find(|m| kind_of(m) == "echoNextSpell").map(|m| m.expiry.clone()),
            Some(ModifierExpiry::Used)
        );

        // One removal event per modifier taken, naming exactly the three that went.
        let removed: Vec<String> = events_of_type(&events, GameEventType::ModifierChanged)
            .iter()
            .filter_map(|event| match event {
                GameEvent::ModifierChanged { added: false, modifier_id, .. } => Some(modifier_id.clone()),
                _ => None,
            })
            .collect();
        let gone: Vec<String> = before
            .iter()
            .filter(|m| !state.players.p1.mods.iter().any(|kept| kept.id == m.id))
            .map(|m| m.id.clone())
            .collect();
        assert_eq!(removed, gone);
        assert_eq!(gone.len(), 3);
    }
}
