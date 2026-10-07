//! R280 (SPEC §10.8, §10.9): the number a formula comes to now, as `viewFor` carries it on a card
//! view (`preview`), and a Fuse's list (R102).
//!
//! Every card here is a test-only definition whose script carries a `vi.fn` `preview`, so each test
//! controls what the hook answers and reads back every question the engine asked it. The pattern is
//! conditionActive.test.ts's: the definitions go on top of the fixture catalog and scripts, and the
//! registries are put back in `afterAll`. The real Core hooks are proved in packages/cards
//! (test/preview.test.ts).
//!
//! Where the view carries it (§10.8), both seats:
//!   - the viewer's own hand, in any phase and on either turn;
//!   - a unit on top of its pile, either seat's;
//!   - a backrow card face-up to the viewer: a Field Spell and a fired Field Trap for both, a
//!     face-down Trap or Field Trap for its controller alone (R33).
//! Never: the opponent's hand, a library card, a card buried under a Stack, a graveyard, exile or
//! resolving card — and the hook is not even asked about those. Absent, never `[]`, when the hook
//! answers nothing or the card has none.
//!
//! Port of `packages/engine/test/preview.test.ts`. A `vi.fn` hook is a hook that records each question
//! it is asked (the parts of its `ConditionContext` the tests read) in a thread-local log; `mockClear`
//! empties its entries. The registries are the testkit's per-thread override (SURFACE §8) and each test
//! runs on its own thread, so TS's `beforeAll`/`afterAll` save and restore have nothing left to do, and
//! TS's `beforeEach` clear is the fresh thread's empty log. TS held the live card `put` and `inHand`
//! returned; here a card is re-read from the state (`live`) and written through `find_instance_mut`.

use std::cell::Cell;

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};

use crate::rules::fixtures::harness::{in_hand, new_game, put, set_library, sink_for, slot};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Which `vi.fn` answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HookId {
    Base,
    Radiant,
    Empty,
    A,
    B,
}

/// One question a hook was asked: what the tests read off TS's recorded `ConditionContext`.
#[derive(Clone, Debug)]
struct Asked {
    hook: HookId,
    self_id: String,
    controller: PlayerId,
    radiant: bool,
    zone: ConditionZone,
    your_turn: bool,
    turn: i32,
    /// The context's keys as TS names them, sorted (TS `Object.keys(ctx).sort()`).
    keys: Vec<&'static str>,
}

thread_local! {
    /// Every question every hook has been asked since its last clear (TS `hook.mock.calls`).
    static CALLS: Cell<Vec<Asked>> = const { Cell::new(Vec::new()) };
}

fn record(hook: HookId, ctx: &ConditionContext<'_>) {
    // Exhaustive, no `..`: this compiles only while the context is these six fields and nothing else —
    // no rng and no event sink (R280: the hook is a pure read).
    let ConditionContext {
        state,
        self_,
        controller,
        radiant,
        zone,
        your_turn,
    } = *ctx;
    let asked = Asked {
        hook,
        self_id: self_.id.clone(),
        controller,
        radiant,
        zone,
        your_turn,
        turn: state.turn,
        keys: vec!["controller", "radiant", "self", "state", "yourTurn", "zone"],
    };
    CALLS.with(|cell| {
        let mut calls = cell.take();
        calls.push(asked);
        cell.set(calls);
    });
}

fn calls() -> Vec<Asked> {
    CALLS.with(|cell| {
        let calls = cell.take();
        cell.set(calls.clone());
        calls
    })
}

fn calls_of(hook: HookId) -> Vec<Asked> {
    calls().into_iter().filter(|call| call.hook == hook).collect()
}

/// TS `hook.mockClear()` for each of `hooks`.
fn clear(hooks: &[HookId]) {
    CALLS.with(|cell| {
        let calls: Vec<Asked> = cell.take().into_iter().filter(|call| !hooks.contains(&call.hook)).collect();
        cell.set(calls);
    });
}

fn value(label: &str, value: i32) -> PreviewValue {
    PreviewValue {
        label: label.to_string(),
        value,
        display: None,
        ids: None,
    }
}

/// One labelled number per question, naming the card, the face and the zone it was asked about, so
/// a test can tell from the view alone which card's hook answered — and so an id never seen in a
/// view proves the hook's answer for that card did not leak into it.
fn answer(ctx: &ConditionContext<'_>) -> Vec<PreviewValue> {
    let face = if ctx.radiant { 20 } else { 10 };
    let zone = if ctx.zone == ConditionZone::Hand { 1 } else { 2 };
    vec![value(&format!("n({})", ctx.self_.id), face + zone)]
}

fn base_hook() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        record(HookId::Base, &ctx);
        answer(&ctx)
    })
}

fn radiant_hook() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        record(HookId::Radiant, &ctx);
        answer(&ctx)
    })
}

/// Answers nothing: a card whose formula has no number to show.
fn empty_hook() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        record(HookId::Empty, &ctx);
        Vec::new()
    })
}

/// Two ingredients' hooks for R102's concatenation, answering fixed labels.
fn hook_a() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        record(HookId::A, &ctx);
        vec![value("A", 1)]
    })
}

fn hook_b() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        record(HookId::B, &ctx);
        vec![value("B1", 2), value("B2", 3)]
    })
}

/// TS `{ ...base, ...extra }`: every key of `extra` written over `base`.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(target), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    base
}

/// TS `def(name, type, extra = {})`. TS's module counter `nextIndex` (from 2800) is written out as the
/// index each definition got, in the order the file made them.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("pv-{name}"),
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

fn unit_def(name: &str, index: i32, extra: Value) -> CardDef {
    def(
        name,
        index,
        "Unit",
        spread(
            json!({
                "base": { "attack": 2, "health": 3, "keywords": [], "text": name },
                "radiant": { "attack": 4, "health": 6, "keywords": [], "text": name },
            }),
            extra,
        ),
    )
}

/// TS `STACK_TEXT` spread under the given stats.
fn stack_face(attack: i32, health: i32) -> Value {
    json!({ "attack": attack, "health": health, "keywords": [{ "kind": "Stack" }], "text": "stack" })
}

fn pv_unit() -> CardDef {
    unit_def("unit", 2801, json!({}))
}
fn pv_spell() -> CardDef {
    def("spell", 2802, "Spell", json!({}))
}
fn pv_trap() -> CardDef {
    def("trap", 2803, "Trap", json!({}))
}
fn pv_field_trap() -> CardDef {
    def("field-trap", 2804, "Field Trap", json!({}))
}
fn pv_field_spell() -> CardDef {
    def("field-spell", 2805, "Field Spell", json!({}))
}
/// A hooked Stack unit, to sit on top of a pile.
fn pv_stack() -> CardDef {
    unit_def("stack", 2806, json!({ "base": stack_face(1, 1), "radiant": stack_face(2, 2) }))
}
/// A Stack unit with no hook, to bury a hooked card under.
fn plain_stack() -> CardDef {
    unit_def("plain-stack", 2807, json!({ "base": stack_face(1, 1), "radiant": stack_face(2, 2) }))
}
/// A registered script that declares no `preview`.
fn quiet_unit() -> CardDef {
    unit_def("quiet-unit", 2808, json!({}))
}
/// A registered def with no script entry at all.
fn unscripted_unit() -> CardDef {
    unit_def("unscripted-unit", 2809, json!({}))
}
/// A hook that answers `[]`.
fn empty_unit() -> CardDef {
    unit_def("empty-unit", 2810, json!({}))
}
/// A transient def with no script: it lives in `state.transientDefs` only.
fn transient_unit() -> CardDef {
    unit_def("transient-unit", 2811, json!({}))
}
/// R102's ingredients: two hooked units, and two that name cards (R279's `refs`).
fn pv_a() -> CardDef {
    unit_def("a", 2812, json!({ "refs": ["core-t-rush", "core-t-sheep"] }))
}
fn pv_b() -> CardDef {
    unit_def("b", 2813, json!({ "refs": ["core-t-sheep", "core-t-bread"] }))
}

fn defs() -> Vec<CardDef> {
    vec![
        pv_unit(),
        pv_spell(),
        pv_trap(),
        pv_field_trap(),
        pv_field_spell(),
        pv_stack(),
        plain_stack(),
        quiet_unit(),
        unscripted_unit(),
        empty_unit(),
        pv_a(),
        pv_b(),
    ]
}

fn both(base: Option<PreviewHook>, radiant: Option<PreviewHook>) -> CardScripts {
    CardScripts {
        base: Script {
            preview: base,
            ..Script::default()
        },
        radiant: Script {
            preview: radiant,
            ..Script::default()
        },
    }
}

/// TS `HOOKED`.
fn hooked() -> CardScripts {
    both(Some(base_hook()), Some(radiant_hook()))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(pv_unit().id, hooked());
    scripts.insert(pv_spell().id, hooked());
    scripts.insert(pv_trap().id, hooked());
    scripts.insert(pv_field_trap().id, hooked());
    scripts.insert(pv_field_spell().id, hooked());
    scripts.insert(pv_stack().id, hooked());
    scripts.insert(plain_stack().id, both(None, None));
    scripts.insert(quiet_unit().id, both(None, None));
    scripts.insert(empty_unit().id, both(Some(empty_hook()), Some(empty_hook())));
    scripts.insert(pv_a().id, both(Some(hook_a()), Some(hook_a())));
    scripts.insert(pv_b().id, both(Some(hook_b()), Some(hook_b())));
    scripts
}

/// p1's main phase on turn 3, no prompt, no result, with this file's defs and scripts registered.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn one(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("expected a card")
}

/// The card as it stands in `state` now (TS held the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {}", card.id))
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("no card {}", card.id))
}

fn own_hand(view: &PlayerView) -> Vec<CardView> {
    let HandView::Cards(hand) = &view.you.hand else {
        panic!("the viewer's own hand must travel in full (§10.8)");
    };
    hand.clone()
}

fn hand_card(view: &PlayerView, instance_id: &str) -> CardView {
    own_hand(view)
        .into_iter()
        .find(|card| card.instance_id == instance_id)
        .unwrap_or_else(|| panic!("{instance_id} is not in the viewer's hand"))
}

/// The list the view carries, or `None` when the key is absent. A key holding `[]` fails here.
fn shown_list(preview: &Option<Vec<PreviewValue>>) -> Option<Vec<PreviewValue>> {
    let list = preview.as_ref()?;
    assert!(!list.is_empty(), "preview is absent rather than empty");
    Some(list.clone())
}

fn shown_card(card: &CardView) -> Option<Vec<PreviewValue>> {
    shown_list(&card.preview)
}

fn shown_unit(unit: &Option<UnitView>) -> Option<Vec<PreviewValue>> {
    shown_list(&unit.as_ref().expect("no card at that place in the view").preview)
}

fn shown_backrow(card: &Option<BackrowView>) -> Option<Vec<PreviewValue>> {
    match card.as_ref().expect("no card at that place in the view") {
        BackrowView::Public(public) => shown_list(&public.preview),
        // A face-down marker has no `preview` key at all.
        BackrowView::FaceDown(_) => None,
    }
}

/// Every question either face's hook was asked since the last clear.
fn asked() -> Vec<Asked> {
    let mut out = calls_of(HookId::Base);
    out.extend(calls_of(HookId::Radiant));
    out
}

fn asked_about(instance_id: &str) -> Vec<Asked> {
    asked().into_iter().filter(|ctx| ctx.self_id == instance_id).collect()
}

fn clear_hooks() {
    clear(&[HookId::Base, HookId::Radiant]);
}

fn inert_resume() -> Value {
    json!({ "defId": "", "hook": "resume", "step": "none", "radiant": false, "data": {} })
}

fn open_mode_prompt(state: &mut GameState, player: PlayerId) {
    let option = |name: &str| json!({ "key": format!("mode:{name}"), "label": name, "selection": { "pick": "mode", "option": name } });
    {
        let mut sink = sink_for(state);
        open_prompt(
            &mut sink,
            json_as(json!({
                "player": player,
                "kind": "mode",
                "prompt": "Choose one",
                "options": [option("left"), option("right")],
                "resume": inert_resume(),
            })),
        );
    }
    assert!(state.pending.is_some());
}

fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("serialises")
}

/// TS `"key" in object`, on the serialised object.
fn has_key(value: &impl serde::Serialize, key: &str) -> bool {
    serde_json::to_value(value)
        .expect("serialises")
        .as_object()
        .is_some_and(|object| object.contains_key(key))
}

fn side(view: &PlayerView, viewer: PlayerId) -> &SideView {
    if viewer == P1 { &view.you } else { &view.opponent }
}

// ---------------------------------------------------------------------------
// The viewer's hand
// ---------------------------------------------------------------------------

/// R280 preview in the viewer's own hand
mod r280_preview_in_the_viewer_s_own_hand {
    use super::*;

    #[test]
    fn r280_a_hand_card_carries_its_hook_s_list_asked_as_a_hand_card_of_its_controller_s() {
        let mut state = game("r280-hand");
        let spell = one(in_hand(&mut state, &pv_spell().id, P1, 1));
        let unit = one(in_hand(&mut state, &pv_unit().id, P1, 1));

        let view = view_for(&state, P1);

        assert_eq!(
            shown_card(&hand_card(&view, &spell.id)),
            Some(vec![value(&format!("n({})", spell.id), 11)])
        );
        assert_eq!(
            shown_card(&hand_card(&view, &unit.id)),
            Some(vec![value(&format!("n({})", unit.id), 11)])
        );
        let calls = asked_about(&spell.id);
        assert!(!calls.is_empty());
        for ctx in calls {
            assert_eq!(ctx.zone, ConditionZone::Hand);
            assert_eq!(ctx.controller, P1);
            assert!(!ctx.radiant);
            assert!(ctx.your_turn);
            assert_eq!(ctx.turn, state.turn);
        }
    }

    #[test]
    fn r280_unlike_the_glow_a_hand_card_carries_it_on_the_opponent_s_turn_outside_the_main_phase_and_with_a_prompt_open() {
        let mut state = game("r280-hand-any-phase");
        let spell = one(in_hand(&mut state, &pv_spell().id, P1, 1));

        state.active = P2;
        assert_eq!(
            shown_card(&hand_card(&view_for(&state, P1), &spell.id)),
            Some(vec![value(&format!("n({})", spell.id), 11)])
        );
        assert!(asked_about(&spell.id).iter().all(|ctx| !ctx.your_turn));

        state.active = P1;
        for phase in [Phase::Mulligan, Phase::Start, Phase::End] {
            state.phase = phase;
            assert!(shown_card(&hand_card(&view_for(&state, P1), &spell.id)).is_some(), "{phase}");
        }
        state.phase = Phase::Main;
        open_mode_prompt(&mut state, P1);
        assert!(shown_card(&hand_card(&view_for(&state, P1), &spell.id)).is_some());
    }

    #[test]
    fn r280_each_seat_s_own_hand_carries_it_in_its_own_view_and_the_other_seat_s_view_carries_nothing_of_it() {
        let mut state = game("r280-hand-both-seats");
        let mine = one(in_hand(&mut state, &pv_spell().id, P1, 1));
        let theirs = one(in_hand(&mut state, &pv_spell().id, P2, 1));

        let p2_view = view_for(&state, P2);
        assert_eq!(
            shown_card(&hand_card(&p2_view, &theirs.id)),
            Some(vec![value(&format!("n({})", theirs.id), 11)])
        );
        assert_eq!(p2_view.opponent.hand, HandView::Count { count: 1 });
        assert!(!to_json(&p2_view).contains(&format!("n({})", mine.id)));

        clear_hooks();
        let p1_view = view_for(&state, P1);
        assert!(shown_card(&hand_card(&p1_view, &mine.id)).is_some());
        assert_eq!(p1_view.opponent.hand, HandView::Count { count: 1 });
        assert!(!to_json(&p1_view).contains(&format!("n({})", theirs.id)));
        // Not only unshown: never asked. The opponent's hand card is hidden (§9.1).
        assert!(asked_about(&theirs.id).is_empty());
    }

    #[test]
    fn r280_a_radiant_card_asks_its_radiant_face_s_hook_with_radiant_true() {
        let mut state = game("r280-hand-radiant");
        let plain = one(in_hand(&mut state, &pv_spell().id, P1, 1));
        let shiny = one(in_hand(&mut state, &pv_spell().id, P1, 1));
        live_mut(&mut state, &shiny).radiant = true;

        let view = view_for(&state, P1);

        assert_eq!(
            shown_card(&hand_card(&view, &shiny.id)),
            Some(vec![value(&format!("n({})", shiny.id), 21)])
        );
        assert_eq!(
            shown_card(&hand_card(&view, &plain.id)),
            Some(vec![value(&format!("n({})", plain.id), 11)])
        );
        assert_eq!(
            calls_of(HookId::Radiant).into_iter().map(|ctx| ctx.self_id).collect::<Vec<_>>(),
            vec![shiny.id.clone()]
        );
        assert_eq!(
            calls_of(HookId::Base).into_iter().map(|ctx| ctx.self_id).collect::<Vec<_>>(),
            vec![plain.id.clone()]
        );
    }
}

// ---------------------------------------------------------------------------
// The field: the top of a unit pile, the backrow
// ---------------------------------------------------------------------------

/// R280 preview on the field
mod r280_preview_on_the_field {
    use super::*;

    #[test]
    fn r280_a_unit_on_top_of_its_pile_carries_it_in_both_seats_views_asked_for_its_own_controller() {
        let mut state = game("r280-unit");
        let unit = put(&mut state, &pv_unit().id, slot(P1, Row::Units, 2), json!({}));

        let own = view_for(&state, P1);
        let other = view_for(&state, P2);

        let expected = Some(vec![value(&format!("n({})", unit.id), 12)]);
        assert_eq!(shown_unit(&own.you.units[1]), expected);
        assert_eq!(shown_unit(&other.opponent.units[1]), expected);
        // The number is the card's, so p2's view asks the hook about p1's card as p1's.
        for ctx in asked_about(&unit.id) {
            assert_eq!(ctx.zone, ConditionZone::Field);
            assert_eq!(ctx.controller, P1);
            assert!(ctx.your_turn);
        }
    }

    #[test]
    fn r280_a_unit_the_viewer_s_opponent_controls_is_asked_with_that_controller_s_your_turn() {
        let mut state = game("r280-unit-theirs");
        let theirs = put(&mut state, &pv_unit().id, slot(P2, Row::Units, 1), json!({}));

        assert!(shown_unit(&view_for(&state, P1).opponent.units[0]).is_some());
        let calls = asked_about(&theirs.id);
        assert!(!calls.is_empty());
        for ctx in calls {
            assert_eq!(ctx.controller, P2);
            assert!(!ctx.your_turn);
        }
    }

    #[test]
    fn r280_a_field_spell_and_a_fired_field_trap_carry_it_in_both_seats_views() {
        let mut state = game("r280-backrow-public");
        let field_spell = put(&mut state, &pv_field_spell().id, slot(P1, Row::Backrow, 1), json!({}));
        let fired_field_trap = put(&mut state, &pv_field_trap().id, slot(P1, Row::Backrow, 2), json!({}));
        live_mut(&mut state, &fired_field_trap).face_up = Some(true);

        for viewer in [P1, P2] {
            let view = view_for(&state, viewer);
            let side = side(&view, viewer);
            assert!(
                matches!(&side.backrow[0], Some(BackrowView::Public(card)) if !card.face_down && card.instance_id == field_spell.id),
                "{viewer}"
            );
            assert_eq!(
                shown_backrow(&side.backrow[0]),
                Some(vec![value(&format!("n({})", field_spell.id), 12)]),
                "{viewer}"
            );
            assert!(
                matches!(&side.backrow[1], Some(BackrowView::Public(card)) if !card.face_down && card.instance_id == fired_field_trap.id),
                "{viewer}"
            );
            assert_eq!(
                shown_backrow(&side.backrow[1]),
                Some(vec![value(&format!("n({})", fired_field_trap.id), 12)]),
                "{viewer}"
            );
        }
    }

    #[test]
    fn r280_a_face_down_trap_or_field_trap_carries_it_for_its_controller_alone_and_is_not_asked_for_the_other_seat() {
        let mut state = game("r280-backrow-face-down");
        let trap = put(&mut state, &pv_trap().id, slot(P1, Row::Backrow, 1), json!({}));
        let field_trap = put(&mut state, &pv_field_trap().id, slot(P1, Row::Backrow, 2), json!({}));

        let own = view_for(&state, P1);
        assert_eq!(
            shown_backrow(&own.you.backrow[0]),
            Some(vec![value(&format!("n({})", trap.id), 12)])
        );
        assert_eq!(
            shown_backrow(&own.you.backrow[1]),
            Some(vec![value(&format!("n({})", field_trap.id), 12)])
        );

        clear_hooks();
        let other = view_for(&state, P2);
        let marker = Some(BackrowView::FaceDown(FaceDownBackrowView {
            face_down: true,
            cost: Some(1),
            plague: None,
            buried: None,
            marks: None,
        }));
        assert_eq!(other.opponent.backrow[0], marker);
        assert_eq!(other.opponent.backrow[1], marker);
        assert!(!to_json(&other).contains(&format!("n({})", trap.id)));
        assert!(asked_about(&trap.id).is_empty());
        assert!(asked_about(&field_trap.id).is_empty());
    }

    #[test]
    fn r280_r33_a_face_down_trap_follows_control_a_stolen_one_shows_to_its_new_controller_not_its_owner() {
        let mut state = game("r280-backrow-stolen");
        let trap = put(&mut state, &pv_trap().id, slot(P2, Row::Backrow, 3), json!({}));
        live_mut(&mut state, &trap).owner = P1;

        assert_eq!(
            shown_backrow(&view_for(&state, P2).you.backrow[2]),
            Some(vec![value(&format!("n({})", trap.id), 12)])
        );
        clear_hooks();
        assert_eq!(
            view_for(&state, P1).opponent.backrow[2],
            Some(BackrowView::FaceDown(FaceDownBackrowView {
                face_down: true,
                cost: Some(1),
                plague: None,
                buried: None,
                marks: None,
            }))
        );
        assert!(asked_about(&trap.id).is_empty());
    }

    #[test]
    fn r280_a_buried_card_is_never_asked_only_the_top_of_its_pile_carries_one() {
        let mut state = game("r280-buried");
        let buried = put(&mut state, &pv_unit().id, slot(P1, Row::Units, 3), json!({}));
        let mut top = new_instance(&mut state, &plain_stack().id, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(&mut state, &mut top, &slot(P1, Row::Units, 3), json_as(json!({ "stack": true }))));

        for viewer in [P1, P2] {
            let view = view_for(&state, viewer);
            let side = side(&view, viewer);
            let unit = side.units[2].as_ref().expect("a unit in lane 3");
            assert_eq!(unit.instance_id, top.id, "{viewer}");
            assert_eq!(unit.buried, 1, "{viewer}");
            assert_eq!(shown_unit(&side.units[2]), None, "{viewer}");
        }
        assert!(asked_about(&buried.id).is_empty());
        assert_eq!(preview_of(&state, &live(&state, &buried), P1, ConditionZone::Field), None);
        assert!(asked_about(&buried.id).is_empty());

        // A hooked top of a pile does carry its own, still without asking about the card under it.
        let hooked_pile = put(&mut state, &pv_unit().id, slot(P1, Row::Units, 4), json!({}));
        let mut hooked_top = new_instance(&mut state, &pv_stack().id, P1, Zone::Hand { player: P1 });
        assert!(place_on_field(
            &mut state,
            &mut hooked_top,
            &slot(P1, Row::Units, 4),
            json_as(json!({ "stack": true }))
        ));
        clear_hooks();
        assert_eq!(
            shown_unit(&view_for(&state, P2).opponent.units[3]),
            Some(vec![value(&format!("n({})", hooked_top.id), 12)])
        );
        assert!(asked_about(&hooked_pile.id).is_empty());
    }
}

// ---------------------------------------------------------------------------
// Nowhere else
// ---------------------------------------------------------------------------

/// R280 preview is nowhere else
mod r280_preview_is_nowhere_else {
    use super::*;

    #[test]
    fn r280_library_graveyard_exile_and_resolving_cards_never_carry_it_and_are_never_asked() {
        let mut state = game("r280-piles");
        let in_library = set_library(&mut state, P1, &[pv_spell().id]).into_iter().next();
        let in_grave = new_instance(&mut state, &pv_unit().id, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(in_grave.clone());
        let in_exile = new_instance(&mut state, &pv_spell().id, P1, Zone::Exile { player: P1 });
        state.players.p1.exile.push(in_exile.clone());
        let resolving = new_instance(&mut state, &pv_spell().id, P1, Zone::Resolving { player: P1 });
        state.players.p1.resolving.push(resolving.clone());

        for viewer in [P1, P2] {
            let view = view_for(&state, viewer);
            let side = side(&view, viewer);
            for card in side.graveyard.iter().chain(side.exile.iter()).chain(side.resolving.iter()) {
                assert!(!has_key(card, "preview"), "{} in {viewer}'s view", card.instance_id);
            }
            assert!(side.graveyard.iter().any(|card| card.instance_id == in_grave.id));
            assert!(side.exile.iter().any(|card| card.instance_id == in_exile.id));
            assert!(side.resolving.iter().any(|card| card.instance_id == resolving.id));
            assert!(!to_json(&view).contains("\"preview\""));
        }
        let in_library = in_library.expect("the library card was not made");
        for card in [&in_library, &in_grave, &in_exile, &resolving] {
            assert!(asked_about(&card.id).is_empty());
        }
    }

    #[test]
    fn r280_preview_of_refuses_without_asking_a_card_its_viewer_may_not_read_where_it_is_asked_about() {
        let mut state = game("r280-direct-refusals");
        let their_hand = one(in_hand(&mut state, &pv_spell().id, P2, 1));
        let their_trap = put(&mut state, &pv_trap().id, slot(P2, Row::Backrow, 1), json!({}));
        let library = set_library(&mut state, P1, &[pv_spell().id]).into_iter().next();
        let unit = put(&mut state, &pv_unit().id, slot(P1, Row::Units, 1), json!({}));
        let my_hand = one(in_hand(&mut state, &pv_spell().id, P1, 1));
        let library = library.expect("the library card was not made");

        let at = |card: &CardInstance| live(&state, card);
        assert_eq!(preview_of(&state, &at(&their_hand), P1, ConditionZone::Hand), None);
        assert_eq!(preview_of(&state, &at(&their_trap), P1, ConditionZone::Field), None);
        assert_eq!(preview_of(&state, &at(&library), P1, ConditionZone::Hand), None);
        assert_eq!(preview_of(&state, &at(&library), P1, ConditionZone::Field), None);
        // A card asked about as where it is not: a field unit as a hand card, a hand card as a field one.
        assert_eq!(preview_of(&state, &at(&unit), P1, ConditionZone::Hand), None);
        assert_eq!(preview_of(&state, &at(&my_hand), P1, ConditionZone::Field), None);
        assert!(asked().is_empty());

        // And the positive controls, asked once each.
        assert_eq!(
            preview_of(&state, &at(&their_trap), P2, ConditionZone::Field),
            Some(vec![value(&format!("n({})", their_trap.id), 12)])
        );
        assert_eq!(
            preview_of(&state, &at(&my_hand), P1, ConditionZone::Hand),
            Some(vec![value(&format!("n({})", my_hand.id), 11)])
        );
        assert_eq!(asked().len(), 2);
    }

    #[test]
    fn r280_the_key_is_absent_never_empty_when_the_hook_answers_nothing_or_the_card_has_none() {
        let mut state = game("r280-absent");
        state.transient_defs.insert(transient_unit().id, transient_unit());
        for player in [P1, P2] {
            in_hand(&mut state, &empty_unit().id, player, 1);
            in_hand(&mut state, &quiet_unit().id, player, 1);
            in_hand(&mut state, &unscripted_unit().id, player, 1);
            in_hand(&mut state, &transient_unit().id, player, 1);
        }
        put(&mut state, &empty_unit().id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &quiet_unit().id, slot(P1, Row::Units, 2), json!({}));
        put(&mut state, &unscripted_unit().id, slot(P2, Row::Units, 1), json!({}));
        put(&mut state, &transient_unit().id, slot(P2, Row::Units, 2), json!({}));

        for viewer in [P1, P2] {
            assert!(!to_json(&view_for(&state, viewer)).contains("\"preview\""), "{viewer}");
        }
        // The empty answer was asked for, and it is the answer that made the key absent.
        assert!(!calls_of(HookId::Empty).is_empty());
    }

    #[test]
    fn r280_a_vanilla_card_has_no_text_so_no_preview_s6_3() {
        let mut state = game("r280-vanilla");
        let unit = put(&mut state, &pv_unit().id, slot(P1, Row::Units, 1), json!({}));
        live_mut(&mut state, &unit).vanilla = true;

        assert_eq!(shown_unit(&view_for(&state, P1).you.units[0]), None);
        assert!(asked_about(&unit.id).is_empty());
    }
}

// ---------------------------------------------------------------------------
// A read, and nothing more
// ---------------------------------------------------------------------------

/// R280 the hook is a pure read
mod r280_the_hook_is_a_pure_read {
    use super::*;

    #[test]
    fn r280_the_hook_is_handed_no_rng_and_no_event_sink_so_it_can_neither_draw_nor_emit() {
        let mut state = game("r280-context");
        one(in_hand(&mut state, &pv_spell().id, P1, 1));
        put(&mut state, &pv_unit().id, slot(P2, Row::Units, 1), json!({}));

        view_for(&state, P1);

        let calls = asked();
        assert!(!calls.is_empty());
        for ctx in calls {
            assert_eq!(ctx.keys, vec!["controller", "radiant", "self", "state", "yourTurn", "zone"]);
        }
    }

    #[test]
    fn r280_building_the_view_with_previews_leaves_the_state_exactly_as_it_was_and_the_view_holds_copies() {
        let mut state = game("r280-unchanged");
        in_hand(&mut state, &pv_spell().id, P1, 1);
        put(&mut state, &pv_unit().id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &pv_trap().id, slot(P2, Row::Backrow, 1), json!({}));
        let before = hash_state(&state);
        let cursor = state.rng_cursor;

        let views = [view_for(&state, P1), view_for(&state, P2)];

        assert_eq!(hash_state(&state), before);
        assert_eq!(state.rng_cursor, cursor);

        // A list the view carries is its own: changing it changes no later view.
        let mut list = shown_unit(&views[0].you.units[0]).expect("the unit carries a preview");
        let first = list.first_mut().expect("the preview has an entry");
        first.value = -1;
        assert_eq!(
            shown_unit(&view_for(&state, P1).you.units[0])
                .and_then(|list| list.first().map(|entry| entry.value)),
            Some(12)
        );
    }
}

// ---------------------------------------------------------------------------
// R102: a fused card's list, and its refs
// ---------------------------------------------------------------------------

/// R280 a fusion's preview is its ingredients' lists in ingredient order (R102)
mod r280_r102_a_fusion_s_preview_is_its_ingredients_lists_in_ingredient_order {
    use super::*;

    /// Craft a fusion of `def_ids` into p1's hand through the real R77 `fuse`, the path #99 takes.
    fn craft(state: &mut GameState, def_ids: &[String]) -> CardInstance {
        let ingredients: Vec<CardInstance> =
            def_ids.iter().map(|def_id| one(in_hand(state, def_id, P1, 1))).collect();
        let mut sink = sink_for(state);
        fuse(
            &mut sink,
            FuseArgs {
                ingredients,
                to_hand: Some(P1),
                ..FuseArgs::default()
            },
        )
        .expect("the fusion did not happen")
    }

    fn labels(list: Option<Vec<PreviewValue>>) -> Option<Vec<String>> {
        list.map(|entries| entries.into_iter().map(|entry| entry.label).collect())
    }

    #[test]
    fn r280_a_crafted_fusion_carries_a_s_list_then_b_s_each_hook_asked_about_the_fused_card_itself() {
        let mut state = game("r280-fused-two");
        let fused = craft(&mut state, &[pv_a().id, pv_b().id]);

        assert_eq!(
            shown_card(&hand_card(&view_for(&state, P1), &fused.id)),
            Some(vec![value("A", 1), value("B1", 2), value("B2", 3)])
        );
        let mut ingredient_calls = calls_of(HookId::A);
        ingredient_calls.extend(calls_of(HookId::B));
        for ctx in ingredient_calls {
            assert_eq!(ctx.self_id, fused.id);
            assert_eq!(ctx.controller, P1);
            assert_eq!(ctx.zone, ConditionZone::Hand);
        }

        // The other order is the other list: ingredient order, not a sorted one.
        let reversed = craft(&mut state, &[pv_b().id, pv_a().id]);
        assert_eq!(
            labels(shown_card(&hand_card(&view_for(&state, P1), &reversed.id))),
            Some(vec!["B1".to_string(), "B2".to_string(), "A".to_string()])
        );
    }

    #[test]
    fn r280_one_hooked_ingredient_s_list_is_the_fusion_s_and_a_fusion_of_none_carries_no_key() {
        let mut state = game("r280-fused-one");
        let with_hook = craft(&mut state, &[quiet_unit().id, pv_a().id]);
        let without_hook = craft(&mut state, &[quiet_unit().id, plain_stack().id]);

        let view = view_for(&state, P1);
        assert_eq!(shown_card(&hand_card(&view, &with_hook.id)), Some(vec![value("A", 1)]));
        assert_eq!(shown_card(&hand_card(&view, &without_hook.id)), None);
    }

    #[test]
    fn r280_r77_a_fusion_kept_on_the_field_the_target_shows_the_list_to_both_seats() {
        let mut state = game("r280-fused-field");
        let target = put(&mut state, &pv_a().id, slot(P1, Row::Units, 2), json!({}));
        let ingredient = one(in_hand(&mut state, &pv_b().id, P1, 1));
        let fused = {
            let mut sink = sink_for(&mut state);
            fuse(
                &mut sink,
                FuseArgs {
                    ingredients: vec![ingredient],
                    target: Some(target.clone()),
                    ..FuseArgs::default()
                },
            )
        };
        assert_eq!(fused.map(|card| card.id), Some(target.id.clone()));

        // The played ingredients come first and the target last (`fuse`), as the fused name says.
        let target_def = live(&state, &target).def_id;
        assert_eq!(state.transient_defs.get(&target_def).map(|def| def.name.as_str()), Some("b + a"));
        let labels_shown = Some(vec!["B1".to_string(), "B2".to_string(), "A".to_string()]);
        assert_eq!(labels(shown_unit(&view_for(&state, P1).you.units[1])), labels_shown);
        assert_eq!(labels(shown_unit(&view_for(&state, P2).opponent.units[1])), labels_shown);
    }

    #[test]
    fn r102_r279_a_fused_definition_names_every_card_its_ingredients_name_their_refs_union_in_order() {
        let mut state = game("r102-fused-refs");
        let both = craft(&mut state, &[pv_a().id, pv_b().id]);
        let one_ = craft(&mut state, &[pv_b().id, quiet_unit().id]);
        let none = craft(&mut state, &[quiet_unit().id, plain_stack().id]);

        assert_eq!(
            state.transient_defs.get(&both.def_id).and_then(|def| def.refs.clone()),
            Some(vec!["core-t-rush".to_string(), "core-t-sheep".to_string(), "core-t-bread".to_string()])
        );
        assert_eq!(
            state.transient_defs.get(&one_.def_id).and_then(|def| def.refs.clone()),
            Some(vec!["core-t-sheep".to_string(), "core-t-bread".to_string()])
        );
        let none_def = state.transient_defs.get(&none.def_id);
        assert!(none_def.is_some());
        assert!(!has_key(none_def.expect("defined"), "refs"));
    }
}
