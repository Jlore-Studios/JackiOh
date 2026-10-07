//! `state.pending`, the ten prompt kinds and the serializable continuation that makes answering one
//! re-enter the script that asked (SPEC §10.6, BUILD M3-T3).
//!
//! What this file has to prove, in the words of the acceptance list: a state with an open prompt
//! survives `JSON.parse(JSON.stringify(state))` and answering still works; the opponent's `viewFor`
//! shows `pendingFor: playerId` and no options; an `answer` with an option not in `options` errors;
//! `legalActions` lists every option. Around those: `resume` is a script id, a step name and
//! captured data and never a closure (§9.3), chained prompts run to the end of the chain (Private
//! Tutor's three steps, Craft a Card's two Discovers), a prompt in the middle of an effect list
//! parks the tail rather than skipping it, R81's split between play choices and resolution choices,
//! and R98's "a card that asks a question while it resolves is still itself".
//!
//! The fixture cards here have no Core counterpart yet (M4 builds the real ones), so they live in
//! this file rather than in a shared fixture, as aiPolicy.test.ts does for its own (BUILD §0).
//!
//! Port of `packages/engine/test/prompts.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;

use jackioh_engine::effects::{
    add_to_hand, choose_from_hand, choose_mode, choose_target, chosen_options, damage, discover_from_catalog,
    discover_from_graveyard, remember,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};
use crate::rules::fixtures::scripts::{going_long, x_bolt};

// ---------------------------------------------------------------------------
// Fixture definitions
// ---------------------------------------------------------------------------

/// Copies `extra`'s keys over `into`'s (TS's `{ ...defaults, ...extra }`).
fn merge(into: &mut Value, extra: Value) {
    if let (Some(into), Value::Object(extra)) = (into.as_object_mut(), extra) {
        into.extend(extra);
    }
}

/// TS's `def(name, type, extra)`. TS numbered the fixtures with a module counter from 1400; each
/// card's index is written out where it is defined, in the TS file's order.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("pr-{name}"),
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
    merge(&mut card, extra);
    json_as(card)
}

fn unit(name: &str, index: u32, extra: Value) -> CardDef {
    let mut faces = json!({
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": name },
    });
    merge(&mut faces, extra);
    def(name, "Unit", index, faces)
}

/// #26 KY's Private Tutor's shape: three chained steps, each one carrying the last one's pick.
fn tutor() -> CardDef {
    def("tutor", "Spell", 1401, json!({}))
}

/// #66 Craft a Card's shape: two Discovers, the first one's pick carried into the second.
fn crafter() -> CardDef {
    def("crafter", "Spell", 1402, json!({}))
}

/// A prompt in the middle of an effect list, so the tail has to be parked (§9.3).
fn mid_list() -> CardDef {
    def("mid-list", "Spell", 1403, json!({}))
}

/// R98: a card that asks a question while it resolves.
fn asker() -> CardDef {
    def("asker", "Spell", 1404, json!({}))
}

/// R81: a card whose direction travels in the play action (#52 Silly Silas's shape).
fn director() -> CardDef {
    def("director", "Spell", 1405, json!({}))
}

/// R81: a card that pays a Tribute, which is also a play choice.
fn tributer() -> CardDef {
    unit("tributer", 1406, json!({ "cost": 2 }))
}

/// What the chain hands over when every step's pick arrived, and what it hands over otherwise.
fn prize() -> CardDef {
    unit("prize", 1407, json!({}))
}

fn decoy() -> CardDef {
    unit("decoy", 1408, json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![tutor(), crafter(), mid_list(), asker(), director(), tributer(), prize(), decoy()]
}

// ---------------------------------------------------------------------------
// Fixture scripts
// ---------------------------------------------------------------------------

/// The picks every step of a chain has made so far, oldest first (§10.6's "captured data").
fn picked_so_far(ctx: &EffectContext<'_>) -> Vec<String> {
    let mut picked: Vec<String> = match ctx.data.get("picked") {
        Some(Value::Array(held)) => held.iter().filter_map(|value| value.as_str().map(str::to_string)).collect(),
        _ => Vec::new(),
    };
    picked.extend(chosen_options(ctx).into_iter().map(|option| option.to_string()));
    picked
}

fn tutor_script() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "two",
        hook(|ctx| {
            vec![choose_mode(json_as(json!({
                "options": ["tutor-c", "tutor-d"],
                "step": "three",
                "prompt": "Private Tutor 2",
                "data": { "picked": picked_so_far(ctx) },
            })))]
        }),
    );
    resume.insert(
        "three",
        hook(|ctx| {
            vec![choose_mode(json_as(json!({
                "options": ["tutor-e", "tutor-f"],
                "step": "done",
                "prompt": "Private Tutor 3",
                "data": { "picked": picked_so_far(ctx) },
            })))]
        }),
    );
    // The prize proves that all three picks reached the last step, in the order they were made.
    resume.insert(
        "done",
        hook(|ctx| {
            let all = picked_so_far(ctx);
            let as_expected = all.join("+") == "tutor-b+tutor-c+tutor-f";
            vec![
                add_to_hand(json_as(json!({ "defId": if as_expected { prize().id } else { decoy().id } }))),
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": all.len() }))),
            ]
        }),
    );
    Script {
        cry: Some(hook(|_ctx| {
            vec![choose_mode(json_as(json!({
                "options": ["tutor-a", "tutor-b"],
                "step": "two",
                "prompt": "Private Tutor 1",
            })))]
        })),
        resume,
        ..Script::default()
    }
}

fn crafter_script() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "second",
        hook(|ctx| {
            let first = chosen_options(ctx).first().map(|option| option.to_string()).unwrap_or_default();
            vec![discover_from_catalog(json_as(json!({
                "step": "made",
                "query": { "type": "Spell" },
                "prompt": "Craft 2",
                "data": { "first": first },
            })))]
        }),
    );
    resume.insert(
        "made",
        hook(|ctx| {
            let first = ctx.data.get("first").and_then(Value::as_str).unwrap_or_default().to_string();
            let second = chosen_options(ctx).first().map(|option| option.to_string()).unwrap_or_default();
            vec![
                add_to_hand(json_as(json!({ "defId": first }))),
                add_to_hand(json_as(json!({ "defId": second }))),
            ]
        }),
    );
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": "second",
                "query": { "type": "Unit" },
                "prompt": "Craft 1",
            })))]
        })),
        resume,
        ..Script::default()
    }
}

fn mid_list_script() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "after",
        hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 4 })))]),
    );
    Script {
        cry: Some(hook(|_ctx| {
            vec![
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                choose_mode(json_as(json!({ "options": ["left", "right"], "step": "after", "prompt": "mid-list" }))),
                // The tail: it must run after the answer, not before it and not never.
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 2 }))),
            ]
        })),
        resume,
        ..Script::default()
    }
}

/// R98: `remember` writes on `ctx.self`, so it writes nothing at all when the card is gone.
fn asker_script(face: &'static str) -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "check",
        hook(move |ctx| {
            let seen = ctx.self_.as_ref();
            let value = json!({
                "id": seen.map(|card| card.id.clone()),
                "x": ctx.x,
                "grade": seen.and_then(|card| card.counters.grade),
                "face": face,
            });
            let carried = ctx.data.get("carried").and_then(Value::as_i64).unwrap_or(0);
            vec![
                remember(json_as(json!({ "key": "sawSelf", "value": value }))),
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": carried }))),
            ]
        }),
    );
    Script {
        cry: Some(hook(|ctx| {
            vec![choose_mode(json_as(json!({
                "options": ["ask-a", "ask-b"],
                "step": "check",
                "prompt": "asked while resolving",
                "data": { "carried": ctx.x },
            })))]
        })),
        resume,
        ..Script::default()
    }
}

fn director_script() -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Direction,
            options: vec!["left".to_string(), "right".to_string()],
        }],
        cry: Some(hook(|ctx| {
            let amount = if ctx.modes.first().map(String::as_str) == Some("right") { 3 } else { 1 };
            vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
        })),
        ..Script::default()
    }
}

fn tributer_script() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(1),
            ..StaticFlags::default()
        }),
        targets: vec![TargetDecl {
            amount: Some(1),
            ..TargetDecl::tribute(1, 1, json!({ "side": "ally", "of": ["unit"] }))
        }],
        ..Script::default()
    }
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(tutor().id, both(tutor_script()));
    scripts.insert(crafter().id, both(crafter_script()));
    scripts.insert(mid_list().id, both(mid_list_script()));
    scripts.insert(
        asker().id,
        CardScripts {
            base: asker_script("base"),
            radiant: asker_script("radiant"),
        },
    );
    scripts.insert(director().id, both(director_script()));
    scripts.insert(tributer().id, both(tributer_script()));
    scripts
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

fn register() {
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut registered = registered_scripts().clone();
    registered.extend(scripts());
    register_scripts(registered);
}

fn four_mana() -> ManaState {
    ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    }
}

/// p1's main phase on turn 3 with 4 mana; `newGame` re-registers the fixtures, so `register` is after.
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register();
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana = four_mana();
    state
}

static SEQ: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> ReduceResult {
    let seq = SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    reduce(state, &json_as::<ActionInput>(body).with_nonce(format!("pr{seq}")))
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|card| card.id.clone()).collect()
}

/// Past both mulligans, in p1's main phase, the way playChoices-filters.test.ts sets up.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = hand_ids(&state, P1);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" })).state;
    let keep = hand_ids(&state, P2);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" })).state;
    register();
    state.players.p1.mana = four_mana();
    state
}

/// `resolvingCard`'s options.
#[derive(Default)]
struct Resolving {
    player: Option<PlayerId>,
    radiant: bool,
    x: Option<i32>,
    grade: Option<i32>,
}

/// §10.5 step 4: a Spell between its play and its graveyard, which is where a Cry's prompt opens.
fn resolving_card(state: &mut GameState, def_id: &str, options: Resolving) -> CardInstance {
    let player = options.player.unwrap_or(P1);
    let mut card = new_instance(state, def_id, player, Zone::Resolving { player });
    if options.radiant {
        card.radiant = true;
    }
    if let Some(x) = options.x {
        card.x = Some(x);
    }
    if let Some(grade) = options.grade {
        card.counters.grade = Some(grade);
    }
    state.players[player].resolving.push(card.clone());
    card
}

fn mode(option: &str) -> Selection {
    Selection::Mode {
        option: option.to_string(),
    }
}

fn instance(id: &str) -> Selection {
    Selection::Instance {
        instance_id: id.to_string(),
    }
}

fn mode_option(option: &str) -> PromptOption {
    PromptOption {
        key: format!("mode:{option}"),
        label: option.to_string(),
        selection: mode(option),
        cost: None,
        radiant: None,
    }
}

/// A continuation no script services: answering such a prompt just closes it (§10.6).
fn inert_resume() -> Resume {
    Resume {
        def_id: "pr-nothing".to_string(),
        hook: "resume".to_string(),
        step: "none".to_string(),
        radiant: false,
        instance_id: None,
        data: IndexMap::new(),
    }
}

fn prompt_args(player: PlayerId, kind: PromptKind, prompt: &str, options: Vec<PromptOption>) -> OpenPromptArgs {
    OpenPromptArgs {
        player,
        kind,
        aim: None,
        prompt: prompt.to_string(),
        options,
        min: None,
        max: None,
        budget: None,
        owner: None,
        resume: inert_resume(),
    }
}

fn answer_input(player: PlayerId, choice_id: &str, selection: Vec<Selection>) -> AnswerInput {
    AnswerInput {
        player_id: player,
        choice_id: choice_id.to_string(),
        selection,
    }
}

/// `whyAnswerRefused(pending, { playerId, choiceId, selection })`.
fn refusal(pending: &PendingChoice, player: PlayerId, choice_id: &str, selection: Vec<Selection>) -> Option<String> {
    why_answer_refused(pending, &answer_input(player, choice_id, selection)).err().map(|why| why.to_string())
}

/// Answer the open prompt by option key, the way a client would send back what it was offered.
fn answer(sink: &mut EngineSink<'_>, pending: &PendingChoice, keys: &[&str]) -> Option<String> {
    let selection = keys
        .iter()
        .map(|key| {
            pending
                .options
                .iter()
                .find(|option| option.key == *key)
                .unwrap_or_else(|| panic!("\"{}\" never offered {key}", pending.prompt))
                .selection
                .clone()
        })
        .collect();
    answer_prompt(sink, &answer_input(pending.player_id, &pending.id, selection)).err().map(|error| error.to_string())
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

/// Apply one effect outside a card, the way `resolve.ts` does.
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(sink, None, by(controller));
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. As in the TS
/// tests, nothing writes the sink's cursor back.
fn with_sink<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> R {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink)
}

/// TS's `deepValues` walk asserted that nothing reachable from a state fragment is a function. A
/// Rust closure cannot be a field of a serialisable type, so the port asserts what that walk was
/// for: the fragment is plain data, which serialises and reads back equal (§9.3).
fn expect_plain_data<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let json = serde_json::to_value(value).expect("serialisable");
    let back: T = serde_json::from_value(json).expect("deserialisable");
    assert_eq!(&back, value);
}

fn json_of(value: impl Serialize) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively; an array
/// matches element for element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
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

fn keys(options: &[PromptOption]) -> Vec<String> {
    options.iter().map(|option| option.key.clone()).collect()
}

/// The ids a Discover offered (its `mode` selections' options).
fn mode_ids(options: &[PromptOption]) -> Vec<String> {
    options
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Mode { option } => Some(option.clone()),
            _ => None,
        })
        .collect()
}

/// The selection an `answer` action carries (TS `action.selection`).
fn selection_of(action: &ActionBody) -> Vec<Selection> {
    match action {
        ActionBody::Answer { selection, .. } => selection.clone(),
        other => panic!("expected an answer, got {other:?}"),
    }
}

fn error_text(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

fn hand_defs(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|card| card.def_id.clone()).collect()
}

// ---------------------------------------------------------------------------

mod prompts_s10_6_m3_t3 {
    use super::*;

    #[test]
    fn s10_6_a_hero_or_a_cell_is_named_to_the_player_the_prompt_is_for_your_or_enemy_never_by_seat() {
        assert_eq!(hero_option_label(P1, P1), "Your hero");
        assert_eq!(hero_option_label(P2, P1), "Enemy hero");
        assert_eq!(hero_option_label(P1, P2), "Enemy hero");
        assert_eq!(cell_option_label(P1, Row::Units, 3, P1), "Your Unit lane 3");
        assert_eq!(cell_option_label(P2, Row::Backrow, 2, P1), "Enemy Backrow lane 2");

        // A refusal names the pick the same way, to the player who made it.
        let pending = PendingChoice {
            id: "q1".to_string(),
            player_id: P2,
            kind: PromptKind::Target,
            prompt: "Choose a target".to_string(),
            options: vec![PromptOption {
                key: "hero:p1".to_string(),
                label: hero_option_label(P1, P2).to_string(),
                selection: Selection::Hero { player: P1 },
                cost: None,
                radiant: None,
            }],
            min: 1,
            max: 1,
            budget: None,
            resume: inert_resume(),
        };
        assert_eq!(
            refusal(&pending, P2, "q1", vec![Selection::Hero { player: P2 }]),
            Some("Your hero is not one of the options offered".to_string())
        );
        assert_eq!(
            refusal(
                &pending,
                P2,
                "q1",
                vec![Selection::Zone {
                    player: P1,
                    row: Row::Units,
                    lane: 4,
                }]
            ),
            Some("Enemy Unit lane 4 is not one of the options offered".to_string())
        );
    }

    #[test]
    fn s10_6_a_state_with_an_open_prompt_survives_a_json_round_trip_and_still_answers() {
        let mut state = board("round-trip");
        let card = resolving_card(&mut state, &tutor().id, Resolving::default());
        let pending = with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
            sink.state.pending.clone()
        })
        .expect("Private Tutor's first prompt");
        assert_eq!(pending.prompt, "Private Tutor 1");

        // §10.1 keeps the state JSON-only, so the round trip is lossless.
        let mut clone: GameState =
            serde_json::from_str(&serde_json::to_string(&state).expect("a state")).expect("a state");
        assert_eq!(clone, state);
        assert_eq!(clone.pending.as_ref(), Some(&pending));

        // And the clone still answers: the continuation was data, so nothing was left behind.
        with_sink(&mut clone, |clone_sink| {
            let clone_pending = clone_sink.state.pending.clone().expect("the round-tripped prompt");
            assert_eq!(answer(clone_sink, &clone_pending, &["mode:tutor-b"]), None);
            assert_eq!(clone_sink.state.pending.as_ref().expect("the next prompt").prompt, "Private Tutor 2");
        });

        // Answering the clone left the original alone, which is what makes a replay reproducible.
        assert_eq!(state.pending.as_ref().expect("the original prompt").prompt, "Private Tutor 1");
    }

    #[test]
    fn s10_6_resume_is_a_script_id_a_step_and_captured_data_and_never_a_closure_s9_3() {
        let mut state = board("no-closures");
        let card = resolving_card(&mut state, &mid_list().id, Resolving::default());
        with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
        });
        let pending = state.pending.clone().expect("the mid-list prompt");

        // The prompt, its options and its continuation are all plain data.
        expect_plain_data(&pending);
        // So is the parked tail the prompt interrupted.
        assert_eq!(state.work.len(), 1);
        expect_plain_data(&state.work);

        // "script id + step + captured data" (§10.6), and nothing else at all.
        let resume = json_of(&pending.resume);
        assert!(matches_object(
            &resume,
            &json!({
                "defId": mid_list().id,
                "hook": "resume",
                "step": "after",
                "radiant": false,
                "instanceId": card.id,
            })
        ));
        let mut names: Vec<String> = resume.as_object().expect("a resume is an object").keys().cloned().collect();
        names.sort();
        assert_eq!(names, ["data", "defId", "hook", "instanceId", "radiant", "step"]);
        assert!(resume["step"].is_string());
        assert!(resume["data"].is_object());
    }

    #[test]
    fn m3_t3_an_answer_naming_an_option_the_prompt_did_not_offer_is_refused_and_the_prompt_stays_open() {
        let mut state = board("refusals");
        with_sink(&mut state, |sink| {
            let pending = open_prompt(
                sink,
                prompt_args(P1, PromptKind::Mode, "Choose one", vec![mode_option("burn"), mode_option("freeze")]),
            )
            .expect("the mode prompt");
            let offered = pending.options[0].selection.clone();
            let not_offered = mode("never-offered");

            assert!(
                refusal(&pending, P1, &pending.id, vec![not_offered.clone()])
                    .unwrap_or_default()
                    .contains("not one of the options offered")
            );
            assert!(
                answer_prompt(sink, &answer_input(P1, &pending.id, vec![not_offered]))
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_default()
                    .contains("not one of the options offered")
            );
            // A refused answer changes nothing: the prompt is still there to answer.
            assert_eq!(sink.state.pending.as_ref(), Some(&pending));

            // A selection of another kind is not an option either, whatever it carries.
            assert!(
                refusal(&pending, P1, &pending.id, vec![instance("c1")])
                    .unwrap_or_default()
                    .contains("not one of the options offered")
            );

            // The wrong player, a stale choice id and the wrong number of picks are all refused (§10.6).
            assert!(
                refusal(&pending, P2, &pending.id, vec![offered.clone()])
                    .unwrap_or_default()
                    .contains("belongs to the other player")
            );
            assert!(
                refusal(&pending, P1, "q999", vec![offered.clone()])
                    .unwrap_or_default()
                    .contains("no prompt q999 is open")
            );
            assert!(refusal(&pending, P1, &pending.id, vec![]).unwrap_or_default().contains("exactly 1 pick"));
            assert!(
                refusal(&pending, P1, &pending.id, vec![offered.clone(), pending.options[1].selection.clone()])
                    .unwrap_or_default()
                    .contains("exactly 1 pick")
            );

            // The option the prompt did offer is accepted, and the prompt closes.
            assert_eq!(
                answer_prompt(sink, &answer_input(P1, &pending.id, vec![offered])).err().map(|error| error.to_string()),
                None
            );
            assert!(sink.state.pending.is_none());
        });
    }

    #[test]
    fn r60_a_prompt_that_takes_two_picks_refuses_the_same_option_twice() {
        let mut state = board("two-picks");
        let hand = in_hand(&mut state, &plain.id, P1, 3);
        run(
            &mut state,
            choose_from_hand(json_as(json!({ "step": "none", "count": 2, "prompt": "Choose two" }))),
            P1,
        );
        let pending = state.pending.clone().expect("the two-card hand prompt");
        assert_eq!(pending.kind, PromptKind::Hand);
        assert_eq!(pending.min, 2);
        assert_eq!(pending.max, 2);

        let first = instance(&hand[0].id);
        assert!(
            refusal(&pending, P1, &pending.id, vec![first.clone(), first.clone()])
                .unwrap_or_default()
                .contains("picked twice")
        );
        assert_eq!(refusal(&pending, P1, &pending.id, vec![first, instance(&hand[1].id)]), None);
        assert_eq!(state.pending.as_ref(), Some(&pending));
        // The sink is over this very state (TS: `sink.state` is `state`).
        let state_at: *const GameState = &state;
        with_sink(&mut state, |sink| assert!(std::ptr::eq(&*sink.state, state_at)));
    }

    #[test]
    fn m3_t3_prompt_answers_enumerates_every_option_in_the_order_they_were_offered() {
        let mut state = board("enumeration");
        with_sink(&mut state, |sink| {
            let one = open_prompt(
                sink,
                prompt_args(
                    P1,
                    PromptKind::Mode,
                    "one of three",
                    vec![mode_option("a"), mode_option("b"), mode_option("c")],
                ),
            )
            .expect("the one-of-three prompt");
            assert_eq!(
                prompt_answers(&one).iter().map(selection_of).collect::<Vec<_>>(),
                vec![vec![mode("a")], vec![mode("b")], vec![mode("c")]]
            );
            assert!(prompt_answers(&one).iter().all(
                |action| matches!(action, ActionBody::Answer { choice_id, .. } if *choice_id == one.id)
            ));

            // "Choose 1 or 2": the singles first, then the pairs, each pair in offered order.
            sink.state.pending = None;
            let up_to_two = open_prompt(
                sink,
                OpenPromptArgs {
                    min: Some(1),
                    max: Some(2),
                    ..prompt_args(
                        P1,
                        PromptKind::Target,
                        "one or two",
                        vec![mode_option("a"), mode_option("b"), mode_option("c")],
                    )
                },
            )
            .expect("the one-or-two prompt");
            let picks: Vec<Vec<String>> = prompt_answers(&up_to_two)
                .iter()
                .map(|action| {
                    selection_of(action)
                        .into_iter()
                        .map(|pick| match pick {
                            Selection::Mode { option } => option,
                            _ => String::new(),
                        })
                        .collect()
                })
                .collect();
            assert_eq!(
                picks,
                vec![vec!["a"], vec!["b"], vec!["c"], vec!["a", "b"], vec!["a", "c"], vec!["b", "c"]]
            );
            assert!(prompt_answers(&up_to_two).len() <= MAX_PROMPT_ANSWERS);
        });

        // §2.1: a mulligan has its own action, so it is not enumerated as an `answer`.
        let started = begin_game(&new_game("enumeration-mulligan", None)).state;
        let mulligan = mulligan_prompt_for(&started, P1).expect("the mulligan prompt");
        assert_eq!(mulligan.kind, PromptKind::Mulligan);
        assert!(prompt_answers(&mulligan).is_empty());
    }

    #[test]
    fn r211_legal_actions_lists_every_option_of_the_open_prompt_and_concede_for_both_seats_besides_m3_t3() {
        let mut state = board("legal-actions");
        let pending = with_sink(&mut state, |sink| {
            open_prompt(
                sink,
                prompt_args(
                    P1,
                    PromptKind::Mode,
                    "Choose one",
                    vec![mode_option("a"), mode_option("b"), mode_option("c")],
                ),
            )
        })
        .expect("the mode prompt");

        // §10.7: with a prompt open the policy draws from that prompt's answers alone, which is what
        // BUILD M3-T3's "legalActions lists every option" and R44's uniform answering both rest on.
        // R211: `reduce` accepts a concede from either seat while the prompt is open, so both are
        // offered it too — and the policy never takes it (R84).
        let mut expected = prompt_answers(&pending);
        expected.push(ActionBody::Concede);
        assert_eq!(legal_actions(&state, P1), expected);
        assert_eq!(
            legal_actions(&state, P1)
                .iter()
                .filter(|action| matches!(action, ActionBody::Answer { .. }))
                .count(),
            pending.options.len()
        );
        assert_eq!(legal_actions(&state, P2), vec![ActionBody::Concede]);
    }

    #[test]
    fn s10_8_the_opponents_view_shows_pending_for_and_none_of_the_options() {
        let mut state = board("pending-for");
        with_sink(&mut state, |sink| {
            open_prompt(
                sink,
                prompt_args(
                    P1,
                    PromptKind::Discover,
                    "Discover a card",
                    vec![mode_option("first"), mode_option("second"), mode_option("third")],
                ),
            );
        });

        assert_eq!(json_of(&view_for(&state, P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
        let theirs = serde_json::to_string(&view_for(&state, P2)).expect("a view");
        for option in ["first", "second", "third", "Discover a card"] {
            assert!(!theirs.contains(option));
        }
        // The chooser gets the whole prompt, which is the half that makes the hiding meaningful.
        let mine = json_of(&view_for(&state, P1).pending);
        assert!(matches_object(&mine, &json!({ "forYou": true, "kind": "discover", "min": 1, "max": 1 })));
    }

    #[test]
    fn s10_6_chains_private_tutors_three_steps_each_step_carrying_the_picks_before_it() {
        let mut state = board("three-steps");
        let card = resolving_card(&mut state, &tutor().id, Resolving::default());
        let hand_before = state.players.p1.hand.len();

        with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
            let first = sink.state.pending.clone().expect("step 1");
            assert_eq!(first.kind, PromptKind::Mode);
            assert_eq!(first.player_id, P1);
            assert_eq!(keys(&first.options), ["mode:tutor-a", "mode:tutor-b"]);

            assert_eq!(answer(sink, &first, &["mode:tutor-b"]), None);
            let second = sink.state.pending.clone().expect("step 2");
            assert_eq!(second.prompt, "Private Tutor 2");
            assert_ne!(second.id, first.id);

            assert_eq!(answer(sink, &second, &["mode:tutor-c"]), None);
            let third = sink.state.pending.clone().expect("step 3");
            assert_eq!(third.prompt, "Private Tutor 3");

            assert_eq!(answer(sink, &third, &["mode:tutor-f"]), None);
        });
        // The chain is done: no prompt, no parked work, and the last step ran.
        assert!(state.pending.is_none());
        assert!(state.work.is_empty());
        // The prize, not the decoy: all three picks reached the last step, in the order they were made.
        assert!(hand_defs(&state, P1).contains(&prize().id));
        assert!(!hand_defs(&state, P1).contains(&decoy().id));
        assert_eq!(state.players.p1.hand.len(), hand_before + 1);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
    }

    #[test]
    fn s10_6_chains_craft_a_cards_two_discovers_each_drawing_from_its_own_pool() {
        let mut state = board("two-discovers");
        let card = resolving_card(&mut state, &crafter().id, Resolving::default());

        let (first_ids, second_ids) = with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
            let first = sink.state.pending.clone().expect("the first Discover");
            assert_eq!(first.kind, PromptKind::Discover);
            assert_eq!(first.options.len(), 3);
            let first_ids = mode_ids(&first.options);
            assert_eq!(first_ids.len(), 3);

            assert_eq!(answer(sink, &first, &[first.options[0].key.as_str()]), None);
            let second = sink.state.pending.clone().expect("the second Discover");
            assert_eq!(second.kind, PromptKind::Discover);
            assert_eq!(second.options.len(), 3);
            let second_ids = mode_ids(&second.options);
            // A different pool: the first Discover offered Units, the second Spells (§5.1's query).
            assert!(!second_ids.iter().any(|id| first_ids.contains(id)));

            assert_eq!(answer(sink, &second, &[second.options[1].key.as_str()]), None);
            (first_ids, second_ids)
        });
        assert!(state.pending.is_none());
        // Both picks landed: the first came out of `resume.data`, the second out of the answer.
        let hand = hand_defs(&state, P1);
        assert!(hand.contains(&first_ids[0]));
        assert!(hand.contains(&second_ids[1]));
    }

    #[test]
    fn s9_3_a_prompt_in_the_middle_of_an_effect_list_parks_the_tail_and_the_answer_runs_it() {
        let mut state = board("parked-tail");
        let card = resolving_card(&mut state, &mid_list().id, Resolving::default());

        with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
            // The first effect ran, the second opened the prompt, and the third is waiting in state.
            assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 1);
            let pending = sink.state.pending.clone().expect("the mid-list prompt");
            assert_eq!(sink.state.work.len(), 1);
            assert!(matches_object(
                &json_of(&sink.state.work[0].resume),
                &json!({ "defId": mid_list().id, "hook": "cry", "instanceId": card.id })
            ));

            assert_eq!(answer(sink, &pending, &["mode:left"]), None);
            // The answered step ran, then the parked tail: 1 + 4 + 2, and nothing ran twice. The tail runs
            // because answering continues what the prompt interrupted (R113) — nothing else drains here.
            assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH - 7);
            assert!(sink.state.pending.is_none());
            assert!(sink.state.work.is_empty());
        });
    }

    #[test]
    fn r113_registers_the_card_continuation_work_handler_so_a_parked_tail_always_has_an_owner() {
        let mut state = board("tail-has-an-owner");
        let card = resolving_card(&mut state, &mid_list().id, Resolving::default());

        with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
        });
        let owed = state.work[0].clone();

        // The parked tail's hook is the card's own (`cry`), which no engine sequence claims, so the
        // only thing that can run it is the handler `prompts.ts` registers for itself. This file
        // registers none — unlike `pauses.test.ts`, which supplies that wiring on the module's behalf
        // and so cannot notice it missing. R113 raises rather than dropping an item nobody can resume,
        // so a dropped registration turns every mid-list prompt into an error: assert the wiring, not
        // just the behaviour.
        assert!(can_resume(&owed.resume));
        let mut nobody = owed.resume.clone();
        nobody.hook = "__noSuchHook".to_string();
        assert!(!can_resume(&nobody));
    }

    #[test]
    fn r98_a_card_that_asks_a_question_while_it_resolves_is_still_itself() {
        let mut state = board("r98-self");
        // §10.5 step 4 parks a resolving card in `resolving`, which `findInstance` searches (R98).
        let card = resolving_card(
            &mut state,
            &asker().id,
            Resolving {
                radiant: true,
                x: Some(3),
                grade: Some(2),
                ..Resolving::default()
            },
        );

        with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
            let pending = sink.state.pending.clone().expect("the resolving card's prompt");
            assert_eq!(answer(sink, &pending, &["mode:ask-a"]), None);
        });

        // The resumed step found the card, with its counters, its X and its radiant face.
        let live = find_instance(&state, &card.id).expect("the resolving card");
        assert_eq!(
            live.memory.get("sawSelf"),
            Some(&json!({ "id": card.id, "x": 3, "grade": 2, "face": "radiant" }))
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3);
    }

    #[test]
    fn r98_a_card_that_has_left_the_resolving_zone_resumes_with_no_self_from_its_captured_data() {
        let mut state = board("r98-gone");
        let card = resolving_card(
            &mut state,
            &asker().id,
            Resolving {
                x: Some(2),
                ..Resolving::default()
            },
        );

        with_sink(&mut state, |sink| {
            run_hook_resumable(sink, &card, "cry", Default::default());
            let pending = sink.state.pending.clone().expect("the resolving card's prompt");

            // The card ceases to exist before its own question is answered (R11's `gone`).
            sink.state.players.p1.resolving.retain(|held| held.id != card.id);
            assert_eq!(answer(sink, &pending, &["mode:ask-b"]), None);
        });

        // `remember` writes on `ctx.self`, so an empty memory is the proof that self was null. TS held
        // the card object itself; here it is nowhere in the state, and nothing in the state remembers.
        assert!(!card.memory.contains_key("sawSelf"));
        assert!(find_instance(&state, &card.id).is_none());
        assert!(!serde_json::to_string(&state).expect("a state").contains("sawSelf"));
        // The step still ran, on what the Cry captured in `resume.data` rather than on the instance.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
        assert!(state.pending.is_none());
    }

    #[test]
    fn s10_1_a_second_ask_never_overwrites_an_unanswered_prompt_and_no_options_is_no_prompt() {
        let mut state = board("one-prompt");
        with_sink(&mut state, |sink| {
            let first = open_prompt(sink, prompt_args(P1, PromptKind::Mode, "first", vec![mode_option("a")]))
                .expect("the first prompt");
            assert_eq!(
                open_prompt(sink, prompt_args(P2, PromptKind::Mode, "second", vec![mode_option("b")])),
                None
            );
            assert_eq!(sink.state.pending.as_ref(), Some(&first));
        });

        // §6.3: with nothing to offer the effect fizzles and no prompt opens at all.
        let mut empty = board("no-options");
        let opened =
            with_sink(&mut empty, |sink| open_prompt(sink, prompt_args(P1, PromptKind::Target, "nothing to pick", vec![])));
        assert_eq!(opened, None);
        assert!(empty.pending.is_none());
        // The same through the effects library: an empty board offers no target (§6.3, §10.6).
        run(
            &mut empty,
            choose_target(json_as(json!({ "step": "none", "scope": { "side": "any", "of": ["unit"] } }))),
            P1,
        );
        assert!(empty.pending.is_none());
    }

    #[test]
    fn r81_opens_discover_target_mode_and_hand_prompts_and_never_one_of_the_five_play_choices() {
        // §10.6 lists ten kinds; the module names all ten, since the five play choices stay for later sets,
        // and B5 E18's five new ones (prompt-kinds.test.ts proves each of those).
        let mut all: Vec<String> = PROMPT_KINDS.iter().map(|kind| kind.to_string()).collect();
        all.sort();
        assert_eq!(
            all,
            [
                "answer",
                "cell",
                "direction",
                "discover",
                "embiggen",
                "hand",
                "mode",
                "mulligan",
                "number",
                "pick",
                "reward",
                "target",
                "tribute",
                "x",
                "zone",
            ]
        );

        let openers: Vec<(&str, Effect)> = vec![
            ("chooseMode", choose_mode(json_as(json!({ "options": ["a", "b"], "step": "none" })))),
            (
                "chooseTarget",
                choose_target(json_as(json!({ "step": "none", "scope": { "side": "any", "of": ["unit", "hero"] } }))),
            ),
            ("chooseFromHand", choose_from_hand(json_as(json!({ "step": "none" })))),
            (
                "discoverFromCatalog",
                discover_from_catalog(json_as(json!({ "step": "none", "query": { "type": "Unit" } }))),
            ),
            ("discoverFromGraveyard", discover_from_graveyard(json_as(json!({ "step": "none" })))),
        ];

        let mut kinds: IndexSet<PromptKind> = IndexSet::new();
        for (name, effect) in &openers {
            let mut state = board(&format!("kind-{name}"));
            in_hand(&mut state, &plain.id, P1, 2);
            put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
            put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
            let buried = new_instance(&mut state, &plain.id, P1, Zone::Graveyard { player: P1 });
            state.players.p1.graveyard.push(buried);
            run(&mut state, effect.clone(), P1);
            kinds.insert(state.pending.as_ref().unwrap_or_else(|| panic!("{name}'s prompt")).kind);
        }
        // Every kind the effects library can open, plus the mulligan §2.1 opens for itself.
        let mut opened: Vec<String> = kinds.iter().map(|kind| kind.to_string()).collect();
        opened.sort();
        assert_eq!(opened, ["discover", "hand", "mode", "target"]);
        let started = begin_game(&new_game("kind-mulligan", None)).state;
        assert_eq!(mulligan_prompt_for(&started, P1).expect("the mulligan").kind, PromptKind::Mulligan);

        // R81: "No Core card opens an `x`, `embiggen`, `zone`, `tribute` or `direction` prompt, since
        // all five are play choices." Nothing in the effects library can, so nothing built from it can.
        for kind in [
            PromptKind::X,
            PromptKind::Embiggen,
            PromptKind::Zone,
            PromptKind::Tribute,
            PromptKind::Direction,
        ] {
            assert!(!kinds.contains(&kind));
        }
    }

    #[test]
    fn r81_carries_x_embiggen_the_zone_the_tribute_and_the_direction_in_the_play_action_opening_no_prompt() {
        // X: the value travels in `play.x` and is the cost, never a prompt (R65).
        let mut x_state = playing("r81-x");
        let bolt = in_hand(&mut x_state, &x_bolt().id, P1, 1).remove(0);
        let x_played = act(&x_state, json!({ "type": "play", "instanceId": bolt.id, "x": 2, "playerId": "p1" }));
        assert_eq!(x_played.error, None);
        assert!(x_played.state.pending.is_none());
        assert_eq!(x_played.state.players.p2.hero.health, HERO_HEALTH - 2);

        // Embiggen: the price travels in `play.embiggen`.
        let mut big_state = playing("r81-embiggen");
        let long = in_hand(&mut big_state, &going_long().id, P1, 1).remove(0);
        let big_played =
            act(&big_state, json!({ "type": "play", "instanceId": long.id, "embiggen": true, "playerId": "p1" }));
        assert_eq!(big_played.error, None);
        assert!(big_played.state.pending.is_none());
        assert_eq!(big_played.state.players.p1.mana.current, 0);

        // Zone: the lane travels in `play.zone`.
        let mut zone_state = playing("r81-zone");
        let body = in_hand(&mut zone_state, &plain.id, P1, 1).remove(0);
        let zone_played = act(
            &zone_state,
            json!({
                "type": "play",
                "instanceId": body.id,
                "zone": { "row": "units", "lane": 4 },
                "playerId": "p1",
            }),
        );
        assert_eq!(zone_played.error, None);
        assert!(zone_played.state.pending.is_none());
        assert_eq!(
            zone_played.state.players.p1.units[3].as_ref().and_then(|pile| pile.first()).map(|card| card.id.clone()),
            Some(body.id.clone())
        );

        // Direction: a declared `direction` pick travels in `play.modes` (R81's second sentence).
        let mut dir_state = playing("r81-direction");
        let silas = in_hand(&mut dir_state, &director().id, P1, 1).remove(0);
        let dir_played =
            act(&dir_state, json!({ "type": "play", "instanceId": silas.id, "modes": ["right"], "playerId": "p1" }));
        assert_eq!(dir_played.error, None);
        assert!(dir_played.state.pending.is_none());
        assert_eq!(dir_played.state.players.p2.hero.health, HERO_HEALTH - 3);

        // Tribute: the units sacrificed travel in `play.tributes`, and the declaration that names them
        // is answered in `targets` like every other declared pick (#22's meal is read off `ctx.targets`
        // that way). The sacrifice itself is the play validator's (§10.5 step 2, R90); what R81 says
        // here is that neither half pauses the play with a prompt — this is the same action shape
        // `legalActions` enumerates for the card.
        let mut trib_state = playing("r81-tribute");
        let fodder = put(&mut trib_state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let summoner = in_hand(&mut trib_state, &tributer().id, P1, 1).remove(0);
        let trib_played = act(
            &trib_state,
            json!({
                "type": "play",
                "instanceId": summoner.id,
                "tributes": [fodder.id],
                "targets": [{ "pick": "instance", "instanceId": fodder.id }],
                "playerId": "p1",
            }),
        );
        assert_eq!(trib_played.error, None);
        assert!(trib_played.state.pending.is_none());
        // And it really was a Tribute: the unit that paid it is off the field.
        assert!(
            !trib_played
                .state
                .players
                .p1
                .units
                .iter()
                .flatten()
                .flatten()
                .any(|card| card.id == fodder.id)
        );
    }

    #[test]
    fn s10_2_the_reducer_answers_the_open_prompt_and_refuses_an_option_it_did_not_offer() {
        let mut state = playing("reducer-answer");
        let card = in_hand(&mut state, &tutor().id, P1, 1).remove(0);
        let played = act(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
        assert_eq!(played.error, None);
        let pending = played.state.pending.clone().expect("the prompt the Cry opened");
        assert_eq!(pending.prompt, "Private Tutor 1");

        // §10.2's `answer {choiceId, selection}`: an option the prompt never offered comes back as an
        // error on the result, with the state untouched.
        // DISCREPANCY: src/reduce.ts answers `answer` with "prompts arrive with M3" instead of routing
        // it to `prompts.answerPrompt`, which already validates and resumes. The wiring is the
        // reducer's half of M3-T3; `prompts.ts` holds up its end.
        let bad = act(
            &played.state,
            json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "mode", "option": "never-offered" }],
                "playerId": "p1",
            }),
        );
        assert!(error_text(&bad).contains("not one of the options offered"));
        assert_eq!(bad.state, played.state);

        // A prompt blocks every other action while it is open (§9.3).
        assert!(error_text(&act(&played.state, json!({ "type": "endTurn", "playerId": "p1" }))).contains("a prompt is open"));
        // And it is answerable only by its own player (§10.6).
        assert!(
            error_text(&act(
                &played.state,
                json!({
                    "type": "answer",
                    "choiceId": pending.id,
                    "selection": [pending.options[0].selection],
                    "playerId": "p2",
                }),
            ))
            .contains("belongs to the other player")
        );

        // A legal answer re-invokes the script, which opens the next step of the chain.
        let good = act(
            &played.state,
            json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [pending.options[1].selection],
                "playerId": "p1",
            }),
        );
        assert_eq!(good.error, None);
        assert_eq!(good.state.pending.as_ref().expect("step 2").prompt, "Private Tutor 2");
    }
}
