//! Port of `packages/engine/test/conditionActive.test.ts` (its l.765–825, the B10 block that reads
//! SPEC.md's wording and the rulings index, is dropped by #133: `cargo jackioh spec check` reads ids
//! and test names, never prose).
//!
//! R195 (SPEC §10.8, §10.9): the engine's yellow glow, `conditionActive`, as `viewFor` surfaces it.
//!
//! Every card here is a test-only definition whose script carries a `vi.fn` `conditionMet`, so each
//! test controls what the hook answers and can read back every question the engine asked it. The
//! definitions are registered on top of the fixture catalog and scripts (the pattern viewFor.test.ts
//! uses for its own defs), and the registries are put back in `afterAll`.
//!
//! The rules under test (docs/polish/7-mobile-ux.md, S2), first match wins:
//!   1. the game is over                                   -> false
//!   2. zone "field" and the card is not the viewer's      -> false, hook not called
//!   3. zone "hand" outside the viewer's own main phase with no prompt open -> false, hook not called
//!   4. the running face has no hook (a transient def with no script included) -> false
//!   5. otherwise the hook's answer, and only an answer of exactly `true` lights the card.
//! The key is absent otherwise: never `false`, never on the opponent's cards.
//!
//! R196 is proved here too, through a real `fuse` (R77): a fused card's hook is its ingredients'
//! hooks or-ed, so it glows when any ingredient's condition holds.
//!
//! In Rust a `vi.fn` is a `ConditionHook` built per test (`Hooks::new`): its answer is an
//! `Arc<AtomicBool>` the test sets, and every question it is asked goes down an mpsc channel the test
//! reads back (`Hooks::calls`). Each test thread registers its own (the testkit's thread-local
//! registries, SURFACE §8), which is what TS's `beforeEach` reset and `afterAll` restore did.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// TS's `def(name, type, extra)`, its `nextIndex` (from 1950) written out as the index each def gets.
fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    let mut literal = json!({
        "id": format!("ca-{name}"),
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
    });
    spread(&mut literal, extra);
    json_as(literal)
}

/// `{ ...target, ...extra }` on JSON objects.
fn spread(target: &mut Value, extra: Value) {
    if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
}

fn unit_def(name: &str, index: i32, extra: Value) -> CardDef {
    let mut faces = json!({
        "base": { "attack": 2, "health": 3, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 6, "keywords": [], "text": name },
    });
    spread(&mut faces, extra);
    def(name, "Unit", index, faces)
}

/// `STACK_TEXT` spread into a face with the given stats.
fn stack_face(attack: i32, health: i32) -> Value {
    json!({ "attack": attack, "health": health, "keywords": [{ "kind": "Stack" }], "text": "stack" })
}

// Hooked cards, one of each kind of place a card can sit.
fn glow_unit() -> CardDef {
    unit_def("glow-unit", 1951, json!({}))
}
fn glow_spell() -> CardDef {
    def("glow-spell", "Spell", 1952, json!({}))
}
fn glow_trap() -> CardDef {
    def("glow-trap", "Trap", 1953, json!({}))
}
fn glow_field_trap() -> CardDef {
    def("glow-field-trap", "Field Trap", 1954, json!({}))
}
fn glow_field_spell() -> CardDef {
    def("glow-field-spell", "Field Spell", 1955, json!({}))
}
/// A hooked Stack unit, to sit on top of a pile.
fn glow_stack() -> CardDef {
    unit_def("glow-stack", 1956, json!({ "base": stack_face(1, 1), "radiant": stack_face(2, 2) }))
}
/// A Stack unit with no hook, to bury a hooked card under.
fn plain_stack() -> CardDef {
    unit_def("plain-stack", 1957, json!({ "base": stack_face(1, 1), "radiant": stack_face(2, 2) }))
}
/// A registered script that declares no `conditionMet`.
fn quiet_unit() -> CardDef {
    unit_def("quiet-unit", 1958, json!({}))
}
/// A registered def with no script entry at all.
fn unscripted_unit() -> CardDef {
    unit_def("unscripted-unit", 1959, json!({}))
}
/// A transient def with no script: it lives in `state.transientDefs` only, never in a registry.
fn transient_unit() -> CardDef {
    unit_def("transient-unit", 1960, json!({}))
}
/// Two hooked units whose hooks answer independently, for R196's fusions.
fn glow_a() -> CardDef {
    unit_def("glow-a", 1961, json!({}))
}
fn glow_b() -> CardDef {
    unit_def("glow-b", 1962, json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![
        glow_unit(),
        glow_spell(),
        glow_trap(),
        glow_field_trap(),
        glow_field_spell(),
        glow_stack(),
        plain_stack(),
        quiet_unit(),
        unscripted_unit(),
        glow_a(),
        glow_b(),
    ]
}

/// Which `vi.fn` was asked: `baseHook`, `radiantHook`, `hookA` or `hookB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    Base,
    Radiant,
    A,
    B,
}

/// One question a hook was asked: its `ConditionContext`, kept as the values it carried (the
/// context borrows the state, so the state is kept by address, for TS's `toBe(state)`).
#[derive(Clone, Debug)]
struct Asked {
    which: Which,
    state_addr: usize,
    turn: i32,
    self_: CardInstance,
    controller: PlayerId,
    radiant: bool,
    zone: ConditionZone,
    your_turn: bool,
}

/// The four `vi.fn`s of one test: what each answers, and every question each was asked since its
/// last clear. A fresh one per `game()` is TS's `beforeEach` (answers reset, mocks cleared).
struct Hooks {
    base_answer: Arc<AtomicBool>,
    radiant_answer: Arc<AtomicBool>,
    answer_a: Arc<AtomicBool>,
    answer_b: Arc<AtomicBool>,
    inbox: Receiver<Asked>,
    log: Cell<Vec<Asked>>,
}

fn recording(which: Which, answer: Arc<AtomicBool>, outbox: Sender<Asked>) -> ConditionHook {
    condition_hook(move |ctx: ConditionContext<'_>| {
        let _ = outbox.send(Asked {
            which,
            state_addr: ctx.state as *const GameState as usize,
            turn: ctx.state.turn,
            self_: ctx.self_.clone(),
            controller: ctx.controller,
            radiant: ctx.radiant,
            zone: ctx.zone,
            your_turn: ctx.your_turn,
        });
        answer.load(Ordering::SeqCst)
    })
}

fn hooked_script(hook: &ConditionHook) -> Script {
    Script {
        condition_met: Some(hook.clone()),
        ..Script::default()
    }
}

impl Hooks {
    /// The hooks (base and radiant answer `true`, A and B `false`) and this file's `SCRIPTS`.
    fn new() -> (Hooks, IndexMap<String, CardScripts>) {
        let (outbox, inbox) = channel();
        let hooks = Hooks {
            base_answer: Arc::new(AtomicBool::new(true)),
            radiant_answer: Arc::new(AtomicBool::new(true)),
            answer_a: Arc::new(AtomicBool::new(false)),
            answer_b: Arc::new(AtomicBool::new(false)),
            inbox,
            log: Cell::new(Vec::new()),
        };
        let base_hook = recording(Which::Base, hooks.base_answer.clone(), outbox.clone());
        let radiant_hook = recording(Which::Radiant, hooks.radiant_answer.clone(), outbox.clone());
        let hook_a = recording(Which::A, hooks.answer_a.clone(), outbox.clone());
        let hook_b = recording(Which::B, hooks.answer_b.clone(), outbox);

        let hooked = CardScripts {
            base: hooked_script(&base_hook),
            radiant: hooked_script(&radiant_hook),
        };
        let empty = || CardScripts {
            base: Script::default(),
            radiant: Script::default(),
        };
        let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
        for def in [glow_unit(), glow_spell(), glow_trap(), glow_field_trap(), glow_field_spell(), glow_stack()] {
            scripts.insert(def.id, hooked.clone());
        }
        scripts.insert(plain_stack().id, empty());
        scripts.insert(quiet_unit().id, empty());
        scripts.insert(
            glow_a().id,
            CardScripts {
                base: hooked_script(&hook_a),
                radiant: hooked_script(&hook_a),
            },
        );
        scripts.insert(
            glow_b().id,
            CardScripts {
                base: hooked_script(&hook_b),
                radiant: hooked_script(&hook_b),
            },
        );
        (hooks, scripts)
    }

    fn pulled(&self) -> Vec<Asked> {
        let mut log = self.log.take();
        while let Ok(call) = self.inbox.try_recv() {
            log.push(call);
        }
        self.log.set(log.clone());
        log
    }

    /// `<hook>.mock.calls`, as contexts.
    fn calls(&self, which: Which) -> Vec<Asked> {
        self.pulled().into_iter().filter(|call| call.which == which).collect()
    }

    /// `<hook>.mockClear()`.
    fn clear(&self, which: Which) {
        let mut log = self.pulled();
        log.retain(|call| call.which != which);
        self.log.set(log);
    }

    fn answer(&self, which: Which, answer: bool) {
        let flag = match which {
            Which::Base => &self.base_answer,
            Which::Radiant => &self.radiant_answer,
            Which::A => &self.answer_a,
            Which::B => &self.answer_b,
        };
        flag.store(answer, Ordering::SeqCst);
    }

    /// Every question either face's hook was asked since the last clear (base face first).
    fn asked(&self) -> Vec<Asked> {
        let mut all = self.calls(Which::Base);
        all.extend(self.calls(Which::Radiant));
        all
    }

    fn asked_about(&self, instance_id: &str) -> Vec<Asked> {
        self.asked().into_iter().filter(|ctx| ctx.self_.id == instance_id).collect()
    }
}

/// p1's main phase on turn 3, no prompt, no result, with this file's defs and scripts registered.
fn game(seed: &str) -> (GameState, Hooks) {
    let mut state = new_game(seed, None);
    let (hooks, scripts) = Hooks::new();
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts {
        registry.insert(id, script);
    }
    register_scripts(registry);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    (state, hooks)
}

fn one(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("expected a card")
}

/// The instance as the state holds it now (TS reads its live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("{id} is in no zone"))
}

fn own_hand(view: &PlayerView) -> Vec<CardView> {
    match &view.you.hand {
        HandView::Cards(cards) => cards.clone(),
        HandView::Count { .. } => panic!("the viewer's own hand must travel in full (§10.8)"),
    }
}

fn hand_card(view: &PlayerView, instance_id: &str) -> CardView {
    own_hand(view)
        .into_iter()
        .find(|card| card.instance_id == instance_id)
        .unwrap_or_else(|| panic!("{instance_id} is not in the viewer's hand"))
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// `"conditionActive" in card`.
fn has_key<T: serde::Serialize>(card: &T, key: &str) -> bool {
    to_json(card).get(key).is_some()
}

/// The flag as the view carries it. `true` when the key is present, which must then hold exactly
/// `true`; `false` when the key is absent. A key present with any other value fails here, because
/// R195 makes the key absent rather than `false`.
fn glows<T: serde::Serialize>(card: Option<&T>) -> bool {
    let Some(card) = card else {
        panic!("no card at that place in the view");
    };
    match to_json(card).get("conditionActive") {
        None => false,
        Some(flag) => {
            assert_eq!(flag, &json!(true));
            true
        }
    }
}

/// Jest's `toMatchObject` over serialised JSON: every key `expected` names is in `actual` with a
/// matching value (objects by subset, arrays element by element and of the same length).
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

fn view_json_mentions(view: &PlayerView, needle: &str) -> bool {
    serde_json::to_string(view).expect("serialises").contains(needle)
}

/// A fresh sink over `state`, its rng at the state's cursor as reduce builds it (TS `sinkFor`).
fn with_sink<R>(state: &mut GameState, run: impl FnOnce(&mut EngineSink) -> R) -> R {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    run(&mut sink)
}

fn inert_resume() -> Resume {
    json_as(json!({ "defId": "", "hook": "resume", "step": "none", "radiant": false, "data": {} }))
}

fn mode_option(option: &str) -> PromptOption {
    json_as(json!({ "key": format!("mode:{option}"), "label": option, "selection": { "pick": "mode", "option": option } }))
}

fn open_mode_prompt(state: &mut GameState, player: PlayerId) {
    with_sink(state, |sink| {
        open_prompt(
            sink,
            OpenPromptArgs {
                player,
                kind: PromptKind::Mode,
                aim: None,
                prompt: "Choose one".to_string(),
                options: vec![mode_option("left"), mode_option("right")],
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume: inert_resume(),
            },
        );
    });
    assert!(state.pending.is_some());
}

// ---------------------------------------------------------------------------
// B1: the viewer's hand, in their own main phase
// ---------------------------------------------------------------------------

/// `describe("conditionActive in the viewer's hand (R195, B1)")`.
mod r195_condition_active_in_the_viewer_s_hand_b1 {
    use super::*;

    #[test]
    fn r195_b1_a_hand_card_whose_hook_holds_carries_condition_active_true_in_the_viewer_s_main_phase() {
        let (mut state, hooks) = game("r195-hand-true");
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        let unit = one(in_hand(&mut state, &glow_unit().id, P1, 1));

        let view = view_for(&state, P1);

        assert!(glows(Some(&hand_card(&view, &spell.id))));
        assert!(glows(Some(&hand_card(&view, &unit.id))));
        // The hook was asked as a hand card of the viewer's, on the viewer's own turn.
        let calls = hooks.asked_about(&spell.id);
        assert!(!calls.is_empty());
        for ctx in &calls {
            assert_eq!(ctx.zone, ConditionZone::Hand);
            assert_eq!(ctx.controller, P1);
            assert!(ctx.your_turn);
            assert!(!ctx.radiant);
            assert_eq!(ctx.turn, state.turn);
        }
    }

    #[test]
    fn r195_b1_when_the_hook_returns_false_the_key_is_absent_never_false() {
        let (mut state, hooks) = game("r195-hand-false");
        hooks.answer(Which::Base, false);
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));

        let view = view_for(&state, P1);
        let card = hand_card(&view, &spell.id);

        assert!(!has_key(&card, "conditionActive"));
        assert!(!view_json_mentions(&view, "conditionActive"));
        // It was asked, and said no: the absence is the hook's answer, not a skipped question.
        assert!(!hooks.asked_about(&spell.id).is_empty());
    }

    // TS's `it("R195 B1: only an answer of exactly true lights the card")` (l.256) hands the hook a
    // truthy non-boolean (`1`, `"yes"`). A Rust `ConditionHook` returns `bool`, so no hook can answer
    // anything but `true` or `false`: the test has no Rust form (spec-gaps-part-26-2.md).

    #[test]
    fn r195_b1_a_radiant_card_asks_its_radiant_face_s_hook_with_radiant_true() {
        let (mut state, hooks) = game("r195-hand-radiant");
        hooks.answer(Which::Base, false);
        hooks.answer(Which::Radiant, true);
        let plain = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        let shiny = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        find_instance_mut(&mut state, &shiny.id).expect("in hand").radiant = true;

        let view = view_for(&state, P1);

        assert!(glows(Some(&hand_card(&view, &shiny.id))));
        assert!(!glows(Some(&hand_card(&view, &plain.id))));
        let radiant_asked: Vec<String> = hooks.calls(Which::Radiant).iter().map(|ctx| ctx.self_.id.clone()).collect();
        assert!(radiant_asked.contains(&shiny.id));
        assert!(!radiant_asked.contains(&plain.id));
        for ctx in hooks.calls(Which::Radiant) {
            assert!(ctx.radiant);
        }
        for ctx in hooks.calls(Which::Base) {
            assert!(!ctx.radiant);
        }
    }

    #[test]
    fn r195_b1_condition_active_hands_the_hook_the_card_its_controller_its_face_the_zone_and_the_turn_and_asks_once() {
        let (mut state, hooks) = game("r195-direct-hand");
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));

        assert!(condition_active(&state, live(&state, &spell.id), P1, ConditionZone::Hand));

        let calls = hooks.calls(Which::Base);
        assert_eq!(calls.len(), 1);
        let Some(ctx) = calls.first() else {
            panic!("the hook was not called");
        };
        assert_eq!(ctx.state_addr, &state as *const GameState as usize);
        assert_eq!(&ctx.self_, live(&state, &spell.id));
        assert_eq!(ctx.controller, P1);
        assert!(!ctx.radiant);
        assert_eq!(ctx.zone, ConditionZone::Hand);
        assert!(ctx.your_turn);

        hooks.answer(Which::Base, false);
        assert!(!condition_active(&state, live(&state, &spell.id), P1, ConditionZone::Hand));
    }

    #[test]
    fn r195_b1_a_card_with_no_hook_in_the_same_hand_carries_no_key() {
        let (mut state, _hooks) = game("r195-hand-mixed");
        let hooked = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        let quiet = one(in_hand(&mut state, &quiet_unit().id, P1, 1));
        let unscripted = one(in_hand(&mut state, &unscripted_unit().id, P1, 1));

        let view = view_for(&state, P1);

        assert!(glows(Some(&hand_card(&view, &hooked.id))));
        assert!(!has_key(&hand_card(&view, &quiet.id), "conditionActive"));
        assert!(!has_key(&hand_card(&view, &unscripted.id), "conditionActive"));
    }
}

// ---------------------------------------------------------------------------
// B2: never in hand outside the viewer's own main phase with no prompt and no result
// ---------------------------------------------------------------------------

/// `describe("conditionActive stays off a hand card outside its playable window (R195, B2)")`.
mod r195_condition_active_stays_off_a_hand_card_outside_its_playable_window_b2 {
    use super::*;

    fn hand_call_count(hooks: &Hooks) -> usize {
        hooks.asked().iter().filter(|ctx| ctx.zone == ConditionZone::Hand).count()
    }

    #[test]
    fn r195_b2_during_the_opponent_s_turn_the_viewer_s_hand_carries_no_flag_and_the_hook_is_not_asked() {
        let (mut state, hooks) = game("r195-b2-their-turn");
        state.active = P2;
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));

        let view = view_for(&state, P1);

        assert!(!has_key(&hand_card(&view, &spell.id), "conditionActive"));
        assert_eq!(hand_call_count(&hooks), 0);
    }

    #[test]
    fn r195_b2_the_non_active_seat_s_own_hand_carries_no_flag_either_from_its_own_view() {
        let (mut state, hooks) = game("r195-b2-p2-hand");
        let theirs = one(in_hand(&mut state, &glow_spell().id, P2, 1));

        let view = view_for(&state, P2);

        assert!(!has_key(&hand_card(&view, &theirs.id), "conditionActive"));
        assert_eq!(hand_call_count(&hooks), 0);
    }

    #[test]
    fn r195_b2_during_the_mulligan_and_every_other_non_main_phase_the_hand_carries_no_flag() {
        for phase in [Phase::Setup, Phase::Mulligan, Phase::Start, Phase::End] {
            let (mut state, hooks) = game(&format!("r195-b2-{phase}"));
            hooks.clear(Which::Base);
            state.phase = phase;
            let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));

            let view = view_for(&state, P1);

            assert!(!has_key(&hand_card(&view, &spell.id), "conditionActive"), "{phase}");
            assert_eq!(hand_call_count(&hooks), 0, "{phase}");
        }
    }

    #[test]
    fn r195_b2_with_the_viewer_s_own_prompt_open_the_hand_carries_no_flag() {
        let (mut state, hooks) = game("r195-b2-own-prompt");
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        open_mode_prompt(&mut state, P1);

        let view = view_for(&state, P1);

        assert!(!has_key(&hand_card(&view, &spell.id), "conditionActive"));
        assert_eq!(hand_call_count(&hooks), 0);
    }

    #[test]
    fn r195_b2_with_the_opponent_s_prompt_open_the_hand_carries_no_flag() {
        let (mut state, hooks) = game("r195-b2-their-prompt");
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        open_mode_prompt(&mut state, P2);

        let view = view_for(&state, P1);

        assert!(!has_key(&hand_card(&view, &spell.id), "conditionActive"));
        assert_eq!(hand_call_count(&hooks), 0);
    }

    #[test]
    fn r195_b2_after_the_game_has_ended_nothing_carries_the_flag_hand_or_field() {
        let (mut state, hooks) = game("r195-b2-over");
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        put(&mut state, &glow_unit().id, slot(P1, Row::Units, 1));
        put(&mut state, &glow_trap().id, slot(P1, Row::Backrow, 1));
        state.result = Some(GameResult {
            winner: Winner::P2,
            reason: GameOverReason::HeroDeath,
        });

        let view = view_for(&state, P1);

        assert!(!has_key(&hand_card(&view, &spell.id), "conditionActive"));
        assert!(!glows(view.you.units[0].as_ref()));
        assert!(!glows(view.you.backrow[0].as_ref()));
        assert!(!view_json_mentions(&view, "conditionActive"));
        assert_eq!(hand_call_count(&hooks), 0);
    }

    #[test]
    fn r195_b2_condition_active_refuses_a_hand_card_outside_the_window_without_asking_the_hook() {
        let (mut state, hooks) = game("r195-b2-direct");
        let spell = one(in_hand(&mut state, &glow_spell().id, P1, 1));

        state.active = P2;
        assert!(!condition_active(&state, live(&state, &spell.id), P1, ConditionZone::Hand));
        state.active = P1;
        state.phase = Phase::Mulligan;
        assert!(!condition_active(&state, live(&state, &spell.id), P1, ConditionZone::Hand));
        state.phase = Phase::Main;
        open_mode_prompt(&mut state, P1);
        assert!(!condition_active(&state, live(&state, &spell.id), P1, ConditionZone::Hand));

        assert!(hooks.calls(Which::Base).is_empty());
    }
}

// ---------------------------------------------------------------------------
// B3: the viewer's own units (top of the pile) and backrow, on either turn
// ---------------------------------------------------------------------------

/// `describe("conditionActive on the viewer's own field (R195, B3)")`.
mod r195_condition_active_on_the_viewer_s_own_field_b3 {
    use super::*;

    #[test]
    fn r195_b3_a_unit_the_viewer_controls_carries_the_flag_on_the_viewer_s_turn_asked_as_field_with_yourturn_true() {
        let (mut state, hooks) = game("r195-b3-unit-own-turn");
        let unit = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 2));

        let view = view_for(&state, P1);

        assert!(glows(view.you.units[1].as_ref()));
        let calls = hooks.asked_about(&unit.id);
        assert!(!calls.is_empty());
        for ctx in &calls {
            assert_eq!(ctx.zone, ConditionZone::Field);
            assert_eq!(ctx.controller, P1);
            assert!(ctx.your_turn);
        }
    }

    #[test]
    fn r195_b3_the_same_unit_still_carries_it_on_the_opponent_s_turn_asked_with_yourturn_false() {
        let (mut state, hooks) = game("r195-b3-unit-their-turn");
        state.active = P2;
        let unit = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 1));

        let view = view_for(&state, P1);

        assert!(glows(view.you.units[0].as_ref()));
        let calls = hooks.asked_about(&unit.id);
        assert!(!calls.is_empty());
        for ctx in &calls {
            assert_eq!(ctx.zone, ConditionZone::Field);
            assert!(!ctx.your_turn);
        }
    }

    #[test]
    fn r195_b3_a_trap_a_field_trap_and_a_field_spell_in_the_viewer_s_backrow_carry_the_flag_on_either_turn() {
        for active in [P1, P2] {
            let (mut state, hooks) = game(&format!("r195-b3-backrow-{active}"));
            hooks.clear(Which::Base);
            state.active = active;
            let trap = put(&mut state, &glow_trap().id, slot(P1, Row::Backrow, 1));
            let field_trap = put(&mut state, &glow_field_trap().id, slot(P1, Row::Backrow, 2));
            let field_spell = put(&mut state, &glow_field_spell().id, slot(P1, Row::Backrow, 3));

            let view = view_for(&state, P1);

            for (lane, card) in [(0usize, &trap), (1, &field_trap), (2, &field_spell)] {
                let entry = view.you.backrow[lane].as_ref();
                assert!(
                    matches_object(&to_json(&entry), &json!({ "faceDown": false, "instanceId": card.id })),
                    "{} on {active}'s turn",
                    card.def_id
                );
                assert!(glows(entry), "{} on {active}'s turn", card.def_id);
                for ctx in hooks.asked_about(&card.id) {
                    assert_eq!(ctx.zone, ConditionZone::Field);
                    assert_eq!(ctx.your_turn, active == P1);
                }
            }
        }
    }

    #[test]
    fn r195_b3_a_field_card_is_asked_outside_the_main_phase_and_with_a_prompt_open_and_still_carries_the_flag() {
        let (mut state, _hooks) = game("r195-b3-field-prompt");
        put(&mut state, &glow_unit().id, slot(P1, Row::Units, 1));
        put(&mut state, &glow_trap().id, slot(P1, Row::Backrow, 1));
        open_mode_prompt(&mut state, P2);

        let with_prompt = view_for(&state, P1);
        assert!(glows(with_prompt.you.units[0].as_ref()));
        assert!(glows(with_prompt.you.backrow[0].as_ref()));

        // The end-of-turn window: not the main phase, and the field is still asked.
        state.phase = Phase::End;
        let at_end = view_for(&state, P1);
        assert!(glows(at_end.you.units[0].as_ref()));
        assert!(glows(at_end.you.backrow[0].as_ref()));
    }

    #[test]
    fn r195_b3_a_field_card_whose_hook_returns_false_carries_no_key() {
        let (mut state, hooks) = game("r195-b3-field-false");
        hooks.answer(Which::Base, false);
        put(&mut state, &glow_unit().id, slot(P1, Row::Units, 1));
        put(&mut state, &glow_trap().id, slot(P1, Row::Backrow, 1));

        let view = view_for(&state, P1);

        assert!(!glows(view.you.units[0].as_ref()));
        assert!(!glows(view.you.backrow[0].as_ref()));
        assert!(!view_json_mentions(&view, "conditionActive"));
    }

    #[test]
    fn r195_b3_a_unit_the_viewer_controls_but_does_not_own_carries_it_its_owner_s_view_does_not() {
        let (mut state, hooks) = game("r195-b3-stolen");
        let stolen = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 3));
        find_instance_mut(&mut state, &stolen.id).expect("on the field").owner = P2;

        assert!(glows(view_for(&state, P1).you.units[2].as_ref()));

        hooks.clear(Which::Base);
        assert!(!glows(view_for(&state, P2).opponent.units[2].as_ref()));
        assert!(hooks.asked_about(&stolen.id).is_empty());
    }

    #[test]
    fn r195_b3_graveyard_exile_and_resolving_cards_never_carry_the_flag_and_are_never_asked() {
        let (mut state, hooks) = game("r195-b3-piles");
        let in_grave = new_instance(&mut state, &glow_unit().id, P1, Zone::Graveyard { player: P1 });
        state.players[P1].graveyard.push(in_grave.clone());
        let in_exile = new_instance(&mut state, &glow_spell().id, P1, Zone::Exile { player: P1 });
        state.players[P1].exile.push(in_exile.clone());
        let resolving = new_instance(&mut state, &glow_spell().id, P1, Zone::Resolving { player: P1 });
        state.players[P1].resolving.push(resolving.clone());

        let view = view_for(&state, P1);

        for card in view.you.graveyard.iter().chain(&view.you.exile).chain(&view.you.resolving) {
            assert!(!has_key(card, "conditionActive"), "{}", card.instance_id);
        }
        // Not vacuous: all three piles really travelled.
        assert!(view.you.graveyard.iter().any(|card| card.instance_id == in_grave.id));
        assert!(view.you.exile.iter().any(|card| card.instance_id == in_exile.id));
        assert!(view.you.resolving.iter().any(|card| card.instance_id == resolving.id));
        for card in [&in_grave, &in_exile, &resolving] {
            assert!(hooks.asked_about(&card.id).is_empty());
        }
    }

    #[test]
    fn r195_b3_a_buried_card_is_never_asked_only_the_top_of_its_pile_is() {
        let (mut state, hooks) = game("r195-b3-buried");
        let buried = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 3));
        let top = new_instance(&mut state, &plain_stack().id, P1, Zone::Hand { player: P1 });
        let top_id = top.id.clone();
        assert!(place_on_field(
            &mut state,
            top,
            slot(P1, Row::Units, 3),
            json_as(json!({ "stack": true }))
        ));

        let view = view_for(&state, P1);

        assert!(matches_object(
            &to_json(&view.you.units[2]),
            &json!({ "instanceId": top_id, "buried": 1 })
        ));
        // The top has no hook, and the hooked card under it lends it nothing.
        assert!(!glows(view.you.units[2].as_ref()));
        assert!(hooks.asked_about(&buried.id).is_empty());
    }

    #[test]
    fn r195_b3_a_hooked_top_of_a_stack_pile_carries_the_flag_while_the_hooked_card_under_it_is_not_asked() {
        let (mut state, hooks) = game("r195-b3-hooked-top");
        let buried = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 4));
        let top = new_instance(&mut state, &glow_stack().id, P1, Zone::Hand { player: P1 });
        let top_id = top.id.clone();
        assert!(place_on_field(
            &mut state,
            top,
            slot(P1, Row::Units, 4),
            json_as(json!({ "stack": true }))
        ));

        let view = view_for(&state, P1);

        assert!(matches_object(
            &to_json(&view.you.units[3]),
            &json!({ "instanceId": top_id, "buried": 1 })
        ));
        assert!(glows(view.you.units[3].as_ref()));
        assert!(!hooks.asked_about(&top_id).is_empty());
        assert!(hooks.asked_about(&buried.id).is_empty());
    }

    #[test]
    fn r195_b3_condition_active_asks_a_field_card_on_the_opponent_s_turn_with_yourturn_false() {
        let (mut state, hooks) = game("r195-b3-direct");
        state.active = P2;
        let unit = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 1));

        assert!(condition_active(&state, live(&state, &unit.id), P1, ConditionZone::Field));
        let calls = hooks.calls(Which::Base);
        assert_eq!(calls.len(), 1);
        let ctx = &calls[0];
        assert_eq!(
            (ctx.zone, ctx.controller, ctx.your_turn),
            (ConditionZone::Field, P1, false)
        );
    }
}

// ---------------------------------------------------------------------------
// B4: never on the opponent's side, and never without a hook
// ---------------------------------------------------------------------------

/// `describe("conditionActive is the viewer's alone (R195, B4)")`.
mod r195_condition_active_is_the_viewer_s_alone_b4 {
    use super::*;

    #[test]
    fn r195_b4_the_opponent_s_view_of_p1_s_glowing_cards_carries_no_key_anywhere_and_p1_s_cards_are_not_asked() {
        let (mut state, hooks) = game("r195-b4-opponent-view");
        let hand = one(in_hand(&mut state, &glow_spell().id, P1, 1));
        let unit = put(&mut state, &glow_unit().id, slot(P1, Row::Units, 1));
        let trap = put(&mut state, &glow_trap().id, slot(P1, Row::Backrow, 1));
        let field_spell = put(&mut state, &glow_field_spell().id, slot(P1, Row::Backrow, 2));
        let fired_field_trap = put(&mut state, &glow_field_trap().id, slot(P1, Row::Backrow, 3));
        find_instance_mut(&mut state, &fired_field_trap.id).expect("in the backrow").face_up = Some(true);
        // p2's own hooked unit, so p2's view is not simply a view with nothing to ask about.
        let their_unit = put(&mut state, &glow_unit().id, slot(P2, Row::Units, 2));

        // p1 sees all five of its own cards glowing.
        let mine = view_for(&state, P1);
        assert!(glows(Some(&hand_card(&mine, &hand.id))));
        assert!(glows(mine.you.units[0].as_ref()));
        assert!(glows(mine.you.backrow[0].as_ref()));
        assert!(glows(mine.you.backrow[1].as_ref()));
        assert!(glows(mine.you.backrow[2].as_ref()));
        // And p2's unit from the other side carries nothing.
        assert!(!glows(mine.opponent.units[1].as_ref()));

        hooks.clear(Which::Base);
        hooks.clear(Which::Radiant);
        let theirs = view_for(&state, P2);

        assert_eq!(to_json(&theirs.opponent.hand), json!({ "count": 1 }));
        assert!(!glows(theirs.opponent.units[0].as_ref()));
        assert_eq!(to_json(&theirs.opponent.backrow[0]), json!({ "faceDown": true, "cost": 1 }));
        assert!(matches_object(
            &to_json(&theirs.opponent.backrow[1]),
            &json!({ "faceDown": false, "instanceId": field_spell.id })
        ));
        assert!(!glows(theirs.opponent.backrow[1].as_ref()));
        assert!(matches_object(
            &to_json(&theirs.opponent.backrow[2]),
            &json!({ "faceDown": false, "instanceId": fired_field_trap.id })
        ));
        assert!(!glows(theirs.opponent.backrow[2].as_ref()));
        // p2's own unit is the positive control: the view does ask, just never about p1's cards.
        assert!(glows(theirs.you.units[1].as_ref()));

        let calls = hooks.asked();
        assert!(!calls.is_empty());
        for ctx in &calls {
            assert_eq!(ctx.self_.controller, P2);
            assert_eq!(ctx.controller, P2);
        }
        for card in [&hand, &unit, &trap, &field_spell, &fired_field_trap] {
            assert!(hooks.asked_about(&card.id).is_empty());
        }
        assert!(!hooks.asked_about(&their_unit.id).is_empty());
    }

    #[test]
    fn r195_b4_condition_active_refuses_a_field_card_the_viewer_does_not_control_without_asking_the_hook() {
        let (mut state, hooks) = game("r195-b4-direct");
        let theirs = put(&mut state, &glow_unit().id, slot(P2, Row::Units, 1));

        assert!(!condition_active(&state, live(&state, &theirs.id), P1, ConditionZone::Field));
        assert!(hooks.calls(Which::Base).is_empty());
        assert!(hooks.calls(Which::Radiant).is_empty());
    }

    #[test]
    fn r195_b4_cards_with_no_hook_never_carry_the_key_on_either_seat_a_transient_def_with_no_script_included() {
        let (mut state, _hooks) = game("r195-b4-no-hook");
        state.transient_defs.insert(transient_unit().id, transient_unit());

        for player in [P1, P2] {
            in_hand(&mut state, &quiet_unit().id, player, 1);
            in_hand(&mut state, &unscripted_unit().id, player, 1);
            in_hand(&mut state, &transient_unit().id, player, 1);
        }
        put(&mut state, &quiet_unit().id, slot(P1, Row::Units, 1));
        put(&mut state, &unscripted_unit().id, slot(P1, Row::Units, 2));
        let transient = put(&mut state, &transient_unit().id, slot(P1, Row::Units, 3));
        put(&mut state, &transient_unit().id, slot(P2, Row::Units, 1));
        put(&mut state, &quiet_unit().id, slot(P2, Row::Units, 2));

        for active in [P1, P2] {
            state.active = active;
            for viewer in [P1, P2] {
                let view = view_for(&state, viewer);
                assert!(
                    !view_json_mentions(&view, "conditionActive"),
                    "{viewer}'s view on {active}'s turn"
                );
            }
        }
        assert!(!condition_active(&state, live(&state, &transient.id), P1, ConditionZone::Field));
        state.active = P1;
        let transient_in_hand = one(in_hand(&mut state, &transient_unit().id, P1, 1));
        assert!(!condition_active(&state, live(&state, &transient_in_hand.id), P1, ConditionZone::Hand));
    }
}

// ---------------------------------------------------------------------------
// R196: a fused card glows when any ingredient's condition holds
// ---------------------------------------------------------------------------

/// `describe("R196 a fusion's conditionMet is its ingredients' hooks or-ed")`.
mod r196_a_fusion_s_condition_met_is_its_ingredients_hooks_or_ed {
    use super::*;

    /// Craft a fusion of `def_ids` into p1's hand through the real R77 `fuse`, the path #99 takes.
    fn craft(state: &mut GameState, def_ids: &[String]) -> CardInstance {
        let ingredients: Vec<CardInstance> = def_ids.iter().map(|def_id| one(in_hand(state, def_id, P1, 1))).collect();
        let fused = with_sink(state, |sink| {
            fuse(
                sink,
                FuseArgs {
                    ingredients,
                    to_hand: Some(P1),
                    ..Default::default()
                },
            )
        });
        fused.expect("the fusion did not happen")
    }

    #[test]
    fn r196_a_fusion_of_two_hooked_cards_glows_when_either_hook_holds_and_not_when_neither_does() {
        let (mut state, hooks) = game("r196-two-hooks");
        let fused = craft(&mut state, &[glow_a().id, glow_b().id]);
        assert_eq!(live(&state, &fused.id).zone, Zone::Hand { player: P1 });

        let cases: [(bool, bool, bool); 4] = [
            (false, false, false),
            (true, false, true),
            (false, true, true),
            (true, true, true),
        ];
        for (a, b, expected) in cases {
            hooks.answer(Which::A, a);
            hooks.answer(Which::B, b);
            assert_eq!(
                glows(Some(&hand_card(&view_for(&state, P1), &fused.id))),
                expected,
                "A {a}, B {b}"
            );
        }
    }

    #[test]
    fn r196_each_ingredient_s_hook_is_asked_about_the_fused_card_itself_and_only_an_answer_of_exactly_true_counts() {
        let (mut state, hooks) = game("r196-context");
        let fused = craft(&mut state, &[glow_a().id, glow_b().id]);

        // TS answers `1` and `"yes"`, truthy and not `true`; a Rust hook answers a `bool`, so the
        // nearest answers are `false` both (spec-gaps-part-26-2.md): neither lights the card, and the
        // or-ed hook must still ask both ingredients.
        hooks.answer(Which::A, false);
        hooks.answer(Which::B, false);
        assert!(!condition_active(&state, live(&state, &fused.id), P1, ConditionZone::Hand));
        let asked_a = hooks.calls(Which::A);
        let asked_b = hooks.calls(Which::B);
        assert!(!asked_a.is_empty());
        assert!(!asked_b.is_empty());
        for ctx in asked_a.iter().chain(&asked_b) {
            assert_eq!(ctx.self_.id, fused.id);
            assert_eq!(ctx.controller, P1);
            assert_eq!(ctx.zone, ConditionZone::Hand);
            assert!(ctx.your_turn);
        }
    }

    #[test]
    fn r196_one_hooked_ingredient_s_hook_is_the_fusion_s_and_a_fusion_with_no_hooked_ingredient_never_glows() {
        let (mut state, hooks) = game("r196-one-hook");
        let with_hook = craft(&mut state, &[glow_a().id, quiet_unit().id]);
        let without_hook = craft(&mut state, &[quiet_unit().id, plain_stack().id]);

        hooks.answer(Which::A, true);
        assert!(glows(Some(&hand_card(&view_for(&state, P1), &with_hook.id))));
        hooks.answer(Which::A, false);
        assert!(!glows(Some(&hand_card(&view_for(&state, P1), &with_hook.id))));
        assert!(!glows(Some(&hand_card(&view_for(&state, P1), &without_hook.id))));
        assert!(!condition_active(&state, live(&state, &without_hook.id), P1, ConditionZone::Hand));
    }

    #[test]
    fn r196_r77_a_fusion_kept_on_the_field_glows_for_its_controller_when_either_hook_holds() {
        let (mut state, hooks) = game("r196-field");
        let target = put(&mut state, &glow_a().id, slot(P1, Row::Units, 2));
        let ingredient = one(in_hand(&mut state, &glow_b().id, P1, 1));
        let fused = with_sink(&mut state, |sink| {
            let target = find_instance(sink.state, &target.id).cloned();
            fuse(
                sink,
                FuseArgs {
                    ingredients: vec![ingredient],
                    target,
                    ..Default::default()
                },
            )
        });
        assert_eq!(fused.map(|card| card.id), Some(target.id.clone()));

        hooks.answer(Which::B, true);
        assert!(glows(view_for(&state, P1).you.units[1].as_ref()));
        assert!(
            hooks
                .calls(Which::B)
                .iter()
                .any(|ctx| ctx.zone == ConditionZone::Field && ctx.self_.id == target.id)
        );
        // Still the controller's alone (R195).
        assert!(!glows(view_for(&state, P2).opponent.units[1].as_ref()));

        hooks.answer(Which::B, false);
        assert!(!glows(view_for(&state, P1).you.units[1].as_ref()));
    }
}
