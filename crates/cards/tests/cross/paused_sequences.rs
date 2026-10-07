//! Sequences a prompt interrupts, and the state check around them (SPEC §2.4, §4.5, §9.3, §10.3,
//! §10.6, R53, R59, R89, R113, R127, R156, R158, R174). Found by the polish-4 edge-case hunt, round 5
//! (docs/polish/4-edge-cases.md, lens L7); every case here failed before its fix.
//!
//!  - R59: the check runs after a whole delayed effect or a whole cast-on-draw cast, never between its
//!    parts, so a unit one of them brought to 0 before it asks is still there for the answer to save;
//!    and once the answer has finished a delayed effect its check runs before the next one (R174).
//!  - R127: a continuation whose card has ceased to exist still names its script when it asks again.
//!  - §10.3, R113: a trap's list is resumable like any other, and the trap ends (consumed, checked)
//!    only once its list has.
//!  - R53, R113: a forced run waits for a Death hook's question before its next combat.
//!  - R89: the step a Death hook's prompt re-enters reads the unit as it died.
//!
//! Round 6 (lens L7 again) added the rest of this file's cases: the traps a play's event is still
//! owed behind a trap that asked answer before the play goes on (R17, R118); a power whose draw's
//! cast asks runs no check (R59); My Pawn's AI turn goes on after the other player's answer (R44); a
//! cast asks for its declared choices (R70, R81); Call to Chaos's partner waits for its recursion
//! (R87); a trap owed an event, and a list's tail, meet the board the pause left (R174); a Death pass
//! a nested death paused still owes the rest (R156); a delayed effect made while its point resolves
//! waits for the next (R68); and step 3's hooks resume by the holders the step began with.
//!
//! Round 7 (lens L7) added the cases at the end: a queued trigger's tail after a prompt runs the face
//! its head ran (§5.2, R113); step 3's hooks, the played card's Cry and the traps owed its
//! `cardPlayed` follow the stays they began with, not a Reborn body or a card in a hand (R174, R118);
//! the answer to a cast's own choice goes on with the draw chain rather than the loop (R122), and the
//! traps answer the first cast at the next cast's step-4 window (R70, round 9);
//! and a list a prompt split still reads what its head summoned (R136) and the stays it began with,
//! in the step the answer re-enters as much as in its tail (R174).
//!
//! Round 8 (lens L7) added the last cases: an Echo repeat's fresh pick is aimed at the stay it was
//! made on (R174); a trap that declined an event is not offered it again, and the traps a question
//! kept waiting meet it in the order the dispatch had (R99, R113); a card named by id after the
//! list's own sacrifice is gone, while one picked at a prompt after it is picked on the stay offered
//! (R174, §10.6); a played card a Tribute's question discarded is not placed too (R226); and the
//! check follows a trigger that asked at step 4 before the Cry (R59, R118).
//!
//! No Core card opens a prompt from a delayed effect, a cast on draw, a trap's list or a Death hook,
//! so each case builds the prompting continuation out of engine verbs on a fixture card (a transient
//! def, the way a fusion's is held) and uses real cards for everything else.
//!
//! Round 9 (lens L7) added four: a prompt the engine opens for a fused card as a whole (the ping of the
//! one power R43 activates) comes back to one ingredient (R102); a clause over a set it read off the
//! board resumes over that set, so #94 draws every 2-cost card it began with (R66, R113); a cast's
//! step 4 is a window, so Sheepish answers a cast Unit before its Cry (R70, R17); and a cast makes its
//! choices before it is placed, so a cast Unit is not offered itself (R70, R90).
//!
//! Round 10 (lens L7) added six: a Spell its own list returned to its hand before it asked resumes
//! with no self (R98); a sacrifice whose Death asks leaves the check to the end of the whole list
//! (R59, §4.5); a start-of-game clause that asks as its card arrives waits for the answer (R151,
//! R113); a cast card a step-3 hook's answer exiled stays in exile (R226, R70); the answered step's
//! delayed effect watching the Reborn body it picked is scheduled (R174); and the engine's own
//! `answerPrompt`, handed an Echo repeat's fresh pick, finishes the repeat (R122).
//!
//! Port of `packages/cards/test/paused-sequences.test.ts`.

use jackioh_cards::register_all;
use jackioh_engine::effects;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

const TEMPO_TIMMY: &str = "core-011";
const KPOP_FANATIC: &str = "core-050";
const RENO: &str = "core-053";
const MANA_WELL: &str = "core-006";
const MOTHS_TO_THE_FLAME: &str = "core-009";
const ME_AND_MR_TOKEN: &str = "core-015";
const SHEEPISH: &str = "core-041";
const SHEEP_TOKEN: &str = "core-t-sheep";
const RUSH_TOKEN: &str = "core-t-rush";
const MY_PAWN: &str = "core-096";
const HEROIC_POWER: &str = "core-098";
const CALL_TO_CHAOS: &str = "core-095";
const RIGHT_HOUSE_DEFENDER: &str = "core-003";
const UNLICENSED_EXPERIMENTATION: &str = "core-085";
const HIT_JOB: &str = "core-016";

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

/// TS `sinkFor(s)`: `{ state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) }`.
/// The sink writes the scenario's state in place; the events it collects and the draws it makes go
/// nowhere, as in TS.
fn with_sink<R>(s: &mut Scenario, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> R {
    let state = s.state_mut();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink)
}

/// A fixture definition written as the TS `CardDef` literal.
fn fixture_def(id: &str, ty: &str, cost: i32, face: &Value) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": ty,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": face,
    }))
}

/// TS `{ ...stats, keywords, text: id }` for a Unit, `{ keywords, text: id }` for any other type.
fn fixture_face(id: &str, ty: &str, stats: &Value, keywords: &Value) -> Value {
    if ty == "Unit" {
        json!({ "attack": stats["attack"], "health": stats["health"], "keywords": keywords, "text": id })
    } else {
        json!({ "keywords": keywords, "text": id })
    }
}

/// TS `registerScripts({ ...registeredScripts(), [id]: scripts })`.
fn register_fixture_scripts(id: &str, scripts: CardScripts) {
    let mut registry = registered_scripts().clone();
    registry.insert(id.to_string(), scripts);
    register_scripts(registry);
}

/// A fixture card: a transient def in the match state and its script in the registry. `stats` is
/// TS's `stats = { attack: 2, health: 2 }` (`None` takes the default).
fn fixture(s: &mut Scenario, id: &str, ty: &str, script: Script, stats: Option<Value>) {
    let stats = stats.unwrap_or_else(|| json!({ "attack": 2, "health": 2 }));
    let face = fixture_face(id, ty, &stats, &json!([]));
    s.state_mut().transient_defs.insert(id.to_string(), fixture_def(id, ty, 0, &face));
    register_fixture_scripts(id, CardScripts { base: script.clone(), radiant: script });
}

fn place_fixture(s: &mut Scenario, def_id: &str, player: PlayerId, row: Row, lane: i32) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    assert!(
        place_on_field(s.state_mut(), card.clone(), ZoneSlot { player, row, lane }, Default::default()),
        "could not place {def_id}"
    );
    s.card(&card.id).clone()
}

/// A start-of-turn delayed effect of p1's that re-enters the fixture card's `armed` step.
fn delay_armed(s: &mut Scenario, card: &CardInstance) {
    let resume = Resume {
        def_id: card.def_id.clone(),
        hook: "resume".to_string(),
        step: "armed".to_string(),
        radiant: false,
        instance_id: Some(card.id.clone()),
        data: IndexMap::new(),
    };
    with_sink(s, |sink| {
        schedule_delayed(sink, P1, DelayedAt { phase: Phase::Start, player: P1 }, resume, None, None);
    });
}

/// TS `const ANY_UNIT = { side: "any", of: ["unit"] }`.
fn any_unit() -> Value {
    json!({ "side": "any", "of": ["unit"] })
}

/// A script's `staticFlags`, written as the TS literal.
fn flags(literal: Value) -> Option<StaticFlags> {
    Some(json_as(literal))
}

/// A declared choice, written as the TS literal.
fn decl(literal: Value) -> TargetDecl {
    json_as(literal)
}

/// TS `String(ctx.data[key])`.
fn data_string(ctx: &EffectContext<'_>, key: &str) -> String {
    match ctx.data.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "undefined".to_string(),
    }
}

/// `state.pending?.kind`.
fn pending_kind(s: &Scenario) -> Option<PromptKind> {
    s.state().pending.as_ref().map(|pending| pending.kind)
}

/// The def ids of a pile, in order.
fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

/// TS `findIndex`: the first index matching, or -1.
fn find_index(events: &[GameEvent], matches: impl Fn(&GameEvent) -> bool) -> i64 {
    events.iter().position(matches).map_or(-1, |at| at as i64)
}

/// TS `types.indexOf(type)`: the first index of the event type, or -1.
fn type_index(types: &[&str], ty: &str) -> i64 {
    types.iter().position(|seen| *seen == ty).map_or(-1, |at| at as i64)
}

/// The event types, in order (TS `events.map((event) => event.type)`).
fn types_of(events: &[GameEvent]) -> Vec<&'static str> {
    events.iter().map(|event| event.event_type().as_str()).collect()
}

/// The `when` every "opponent plays a card" trap below shares: `ctx.event.type === "cardPlayed" &&
/// ctx.event.player !== ctx.controller`.
fn played_by_opponent(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
}

/// `ctx.event.type === "cardResolved" && ctx.event.player !== ctx.controller && ctx.event.permanent`.
fn permanent_resolved_by_opponent(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardResolved { player, permanent, .. } if *player != ctx.controller && *permanent)
}

/// `ctx.event.type === "cardPlayed" ? ctx.event.instanceId : ""`.
fn played_id(event: &GameEvent) -> String {
    match event {
        GameEvent::CardPlayed { instance_id, .. } => instance_id.clone(),
        _ => String::new(),
    }
}

/// `ctx.event.type === "cardResolved" ? ctx.event.instanceId : ""`.
fn resolved_id(event: &GameEvent) -> String {
    match event {
        GameEvent::CardResolved { instance_id, .. } => instance_id.clone(),
        _ => String::new(),
    }
}

/// A fresh instance put on top of `player`'s library (TS `library.unshift(card)`).
fn on_top_of_library(s: &mut Scenario, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Library { player });
    s.state_mut().players[player].library.insert(0, card.clone());
    card
}

mod r174_r59_a_delayed_effect_that_asks_is_still_followed_by_the_check_before_the_next_one {
    use super::*;

    #[test]
    fn r174_k_pop_fanatic_s_steal_fizzles_on_a_target_the_delayed_effect_before_it_killed_once_its_prompt_was_answered_r59_r68()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [KPOP_FANATIC, RENO], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO], "library": [RENO, RENO] },
        }));
        // D1: a start-of-turn delayed effect of p1's, created first, that asks for a unit and deals it 3.
        fixture(
            &mut s,
            "edge-r5-pinger",
            "Field Spell",
            Script {
                resume: IndexMap::from([
                    (
                        "armed",
                        hook(|_ctx| vec![effects::choose_target(json_as(json!({ "step": "picked", "scope": any_unit() })))]),
                    ),
                    (
                        "picked",
                        hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 })))]),
                    ),
                ]),
                ..Script::default()
            },
            None,
        );
        let pinger = place_fixture(&mut s, "edge-r5-pinger", P1, Row::Backrow, 5);
        delay_armed(&mut s, &pinger);

        // D2: #50's steal of the 3/3 Tempo Timmy, due at the same point and created after D1 (R68).
        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        s.play(KPOP_FANATIC, json!({ "targets": [{ "pick": "instance", "instanceId": timmy.id }] }));
        s.start_turn();
        assert_eq!(pending_kind(&s), Some(PromptKind::Target));

        // D1 deals Timmy its 3. §4.5 runs after that whole delayed effect, before D2 (R59), so Timmy
        // has died as p2's by the time the steal fires, and the steal fizzles (R76, R174).
        s.answer(json!(timmy.id));

        assert!(!s
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::ControlChanged { instance_id, .. } if *instance_id == timmy.id)));
        s.expect_in_zone(&timmy.id, "graveyard");
    }
}

mod r59_no_state_check_between_the_halves_of_one_delayed_effect_or_one_cast {
    use super::*;

    #[test]
    fn r59_a_unit_a_delayed_effect_brought_to_0_before_its_prompt_is_still_standing_to_be_saved_by_the_answer_r156() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] },
        }));
        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        let timmy_id = timmy.id.clone();
        // One delayed effect: deal the 3/3 Timmy 5, then ask for a unit and give it +10 health.
        fixture(
            &mut s,
            "edge-r5-saver",
            "Field Spell",
            Script {
                resume: IndexMap::from([
                    (
                        "armed",
                        hook(move |_ctx| {
                            vec![
                                effects::damage(json_as(json!({ "to": { "of": "instance", "instanceId": timmy_id }, "amount": 5 }))),
                                effects::choose_target(json_as(json!({ "step": "save", "scope": any_unit() }))),
                            ]
                        }),
                    ),
                    (
                        "save",
                        hook(|_ctx| vec![effects::buff(json_as(json!({ "target": { "of": "chosen" }, "health": 10 })))]),
                    ),
                ]),
                ..Script::default()
            },
            None,
        );
        let saver = place_fixture(&mut s, "edge-r5-saver", P1, Row::Backrow, 5);
        delay_armed(&mut s, &saver);

        s.start_turn();
        let pending = must(s.state().pending.clone(), "the delayed effect's target prompt");
        assert!(pending
            .options
            .iter()
            .any(|option| matches!(&option.selection, Selection::Instance { instance_id } if *instance_id == timmy.id)));
        // The question is part of the delayed effect, so §4.5 has not run yet: Timmy is where the
        // prompt offered it, on the field at -2 (R59: never between the parts of one effect).
        s.expect_in_zone(&timmy.id, "field");

        s.answer(json!(timmy.id));
        s.expect_in_zone(&timmy.id, "field");
        assert_eq!(s.stats(&timmy.id).health, 8);
    }

    #[test]
    fn r59_a_unit_a_cast_on_draw_card_brought_to_0_before_its_prompt_is_still_standing_to_be_saved_by_the_answer_2_4_r158()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] },
        }));
        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        let timmy_id = timmy.id.clone();
        // A cast-on-draw Spell: deal the 3/3 Timmy 5, then ask for a unit and give it +10 health.
        fixture(
            &mut s,
            "edge-r5-cod",
            "Spell",
            Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                cry: Some(hook(move |_ctx| {
                    vec![
                        effects::damage(json_as(json!({ "to": { "of": "instance", "instanceId": timmy_id }, "amount": 5 }))),
                        effects::choose_target(json_as(json!({ "step": "save", "scope": any_unit() }))),
                    ]
                })),
                resume: IndexMap::from([(
                    "save",
                    hook(|_ctx| vec![effects::buff(json_as(json!({ "target": { "of": "chosen" }, "health": 10 })))]),
                )]),
                ..Script::default()
            },
            None,
        );
        on_top_of_library(&mut s, "edge-r5-cod", P1);

        // The turn's draw casts it (§2.4), and the cast asks mid-Cry.
        s.start_turn();
        must(s.state().pending.clone(), "the cast's target prompt");
        // R59: the check follows the whole cast, so Timmy is still on the field while the cast asks.
        s.expect_in_zone(&timmy.id, "field");

        s.answer(json!(timmy.id));
        s.expect_in_zone(&timmy.id, "field");
        assert_eq!(s.stats(&timmy.id).health, 8);
    }
}

mod r127_a_continuation_with_no_instance_keeps_its_script_across_its_own_prompt {
    use super::*;

    #[test]
    fn r127_a_delayed_effect_whose_card_has_ceased_to_exist_still_resolves_the_step_its_prompt_asked_for_r113() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "backrow": [MANA_WELL], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] },
        }));
        fixture(
            &mut s,
            "edge-r5-orphan",
            "Field Spell",
            Script {
                resume: IndexMap::from([
                    (
                        "armed",
                        hook(|_ctx| vec![effects::choose_target(json_as(json!({ "step": "picked", "scope": any_unit() })))]),
                    ),
                    (
                        "picked",
                        hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 })))]),
                    ),
                ]),
                ..Script::default()
            },
            None,
        );
        let orphan = place_fixture(&mut s, "edge-r5-orphan", P1, Row::Backrow, 5);
        delay_armed(&mut s, &orphan);
        // The card that scheduled it is fused away onto p1's Mana Well and ceases to exist (R86, R102),
        // as a K-Pop Fanatic #85 fuses does; its delayed effect still fires, named by its def (R127).
        let well = must(s.backrow(P1, 1).cloned(), "p1's Mana Well");
        must(
            with_sink(&mut s, |sink| {
                subsystems::fuse(sink, json_as(json!({ "ingredients": [orphan], "target": well })))
            }),
            "the fusion",
        );
        s.expect_in_zone(&orphan.id, "gone");

        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        s.start_turn();
        must(s.state().pending.clone(), "the orphaned delayed effect's target prompt");
        s.answer(json!(timmy.id));

        // The answer re-enters the step the prompt named, so Timmy takes the 3 and dies.
        s.expect_in_zone(&timmy.id, "graveyard");
    }
}

mod s10_3_r113_a_trap_s_list_is_resumable_like_any_other {
    use super::*;

    #[test]
    fn r113_a_trap_whose_list_asks_twice_asks_its_second_question_after_the_first_is_answered_before_the_play_goes_on_10_3()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO, RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        fixture(
            &mut s,
            "edge-r5-asking-trap",
            "Trap",
            Script {
                triggers: vec![
                    TriggerDef::new("edge-r5-asks", &[GameEventType::CardPlayed], |_ctx, _event| {
                        vec![
                            effects::choose_mode(json_as(
                                json!({ "options": ["first"], "step": "one", "prompt": "trap: first question" }),
                            )),
                            effects::choose_mode(json_as(
                                json!({ "options": ["second"], "step": "two", "prompt": "trap: second question" }),
                            )),
                        ]
                    })
                    .with_when(played_by_opponent),
                ],
                resume: IndexMap::from([("one", hook(|_ctx| vec![])), ("two", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r5-asking-trap", P2, Row::Backrow, 1);

        s.play(RENO, json!({}));
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.prompt.as_str()),
            Some("trap: first question")
        );
        // §5.1, §10.3: the trap has not finished, so it is not consumed yet, and the play waits.
        assert!(s.pile(P2, "graveyard").is_empty());
        s.answer(json!("first"));
        // §10.3: the trap resolves to completion, prompts included, before the play continues; its
        // second question is never dropped (R113).
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.prompt.as_str()),
            Some("trap: second question")
        );
        assert!(s.pile(P2, "graveyard").is_empty());
        s.answer(json!("second"));
        // Its list is done: the trap reaches the graveyard, and the play has gone on to its end.
        assert_eq!(def_ids(&s.pile(P2, "graveyard")), vec!["edge-r5-asking-trap"]);
        assert!(s.state().pending.is_none());
        assert_eq!(s.unit(P1, 1).map(|card| card.def_id.clone()), Some(RENO.to_string()));
    }
}

mod r53_r113_a_forced_run_waits_for_a_death_hook_s_question_before_its_next_combat {
    use super::*;

    #[test]
    fn r53_moths_to_the_flame_s_next_forced_attack_waits_until_the_death_hook_of_the_unit_the_last_one_killed_has_been_answered_r59_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "field": [{ "def": TEMPO_TIMMY, "lane": 2 }], "hand": [RENO] },
            "p2": { "field": [MOTHS_TO_THE_FLAME], "hand": [RENO], "library": [RENO, RENO] },
        }));
        // A 1/1 in p1's lane 1 whose Death asks p1 for a unit.
        fixture(
            &mut s,
            "edge-r5-last-word",
            "Unit",
            Script {
                death: Some(hook(|_ctx| vec![effects::choose_target(json_as(json!({ "step": "said", "scope": any_unit() })))])),
                resume: IndexMap::from([("said", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            Some(json!({ "attack": 1, "health": 1 })),
        );
        let last_word = place_fixture(&mut s, "edge-r5-last-word", P1, Row::Units, 1);
        let timmy = must(s.unit(P1, 2).cloned(), "p1's Tempo Timmy");
        let moths = must(s.unit(P2, 1).cloned(), "p2's Moths to the Flame");

        // p2's start of turn: every p1 unit attacks Moths, lane 1 first (R53). Moths strikes the 1/1
        // back and it dies in that combat's check, whose Death hook asks p1 something.
        s.start_turn();
        s.expect_in_zone(&last_word.id, "graveyard");
        assert_eq!(must(s.state().pending.clone(), "the Death hook's question").player_id, P1);
        // §4.5 step 3 is part of the first combat's check (R59), and the run is a sequence spanning the
        // question (R113): Timmy's combat comes after the answer, not over the open prompt.
        assert_eq!(s.card(&moths.id).damage, 1);
        assert_eq!(s.card(&timmy.id).damage, 0);

        s.answer(json!(timmy.id));
        assert_eq!(s.card(&moths.id).damage, 4);
        assert_eq!(s.card(&timmy.id).damage, 1);
    }
}

mod r89_a_death_hook_s_answered_step_reads_the_unit_as_it_died {
    use super::*;

    #[test]
    fn r89_the_step_a_death_hook_s_prompt_re_enters_reads_the_snapshot_not_the_instance_r78_has_reset_4_5_step_3_r156()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "field": [TEMPO_TIMMY], "hand": [RENO] },
            "p2": { "hand": [RENO] },
        }));
        // A 2/2 whose Death asks for an enemy target and deals it 1 plus the attack buffs it died with.
        fixture(
            &mut s,
            "edge-r5-grudge",
            "Unit",
            Script {
                death: Some(hook(|_ctx| {
                    vec![effects::choose_target(json_as(
                        json!({ "step": "revenge", "scope": { "side": "enemy", "of": ["unit", "hero"] } }),
                    ))]
                })),
                resume: IndexMap::from([(
                    "revenge",
                    hook(|ctx| {
                        let buffs = ctx.self_.as_ref().map_or(0, |card| card.buffs.attack);
                        vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 + buffs })))]
                    }),
                )]),
                ..Script::default()
            },
            Some(json!({ "attack": 2, "health": 2 })),
        );
        let grudge = place_fixture(&mut s, "edge-r5-grudge", P2, Row::Units, 1);
        must(find_instance_mut(s.state_mut(), &grudge.id), "the grudge on the field").buffs.attack += 3;
        assert_eq!(unit_view(s.state(), s.card(&grudge.id)).attack, 5);

        // First Strike: Timmy kills it without being struck back.
        s.attack(TEMPO_TIMMY, &grudge.id);
        let pending = must(s.state().pending.clone(), "the Death hook's prompt");
        assert_eq!(pending.player_id, P2);
        s.answer(json!("hero:p1"));

        // R89: the hook reads the snapshot taken as the unit died, +3 attack included: 1 + 3 = 4.
        s.expect_health(P1, 26);
    }
}

/// A face-down trap that asks its controller one question whenever the opponent plays a card.
fn asking_trap_on_play(s: &mut Scenario, id: &str) {
    let prompt = format!("{id}: a question");
    fixture(
        s,
        id,
        "Trap",
        Script {
            triggers: vec![
                TriggerDef::new(format!("{id}:asks"), &[GameEventType::CardPlayed], move |_ctx, _event| {
                    vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "answered", "prompt": prompt })))]
                })
                .with_when(played_by_opponent),
            ],
            resume: IndexMap::from([("answered", hook(|_ctx| vec![]))]),
            ..Script::default()
        },
        None,
    );
}

mod r17_r118_r427_the_traps_an_event_is_still_owed_answer_it_before_the_interrupted_play_goes_on {
    use super::*;

    #[test]
    fn r17_r427_a_trap_that_asked_at_the_play_s_cardplayed_holds_the_play_until_answered_and_sheepish_then_answers_its_resolution_after_the_cry_r118_10_3()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ME_AND_MR_TOKEN, RENO], "mana": 4 },
            "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }], "hand": [RENO] },
        }));
        asking_trap_on_play(&mut s, "edge-r6-l7-asks-on-play");
        place_fixture(&mut s, "edge-r6-l7-asks-on-play", P2, Row::Backrow, 1);

        s.play(ME_AND_MR_TOKEN, json!({}));
        assert_eq!(must(s.state().pending.clone(), "the first trap's question").player_id, P2);
        // §10.3, R118: the trap answering `cardPlayed` resolves before the play goes on, so the Cry waits.
        let rush_tokens = |s: &Scenario| {
            [1, 2, 3, 4, 5]
                .into_iter()
                .filter(|&lane| s.unit(P1, lane).is_some_and(|unit| unit.def_id == RUSH_TOKEN))
                .count()
        };
        assert_eq!(rush_tokens(&s), 0);
        s.answer(json!("ok"));

        // The play went on: the Cry summoned its Rush Token, the play resolved, and only then did
        // Sheepish answer it (R427): Me and Mr Token is a Sheep, and the token its Cry made stands.
        assert!(s.state().pending.is_none());
        assert_eq!(s.unit(P1, 1).map(|card| card.def_id.clone()), Some(SHEEP_TOKEN.to_string()));
        assert_eq!(
            rush_tokens(&s),
            1,
            "the Cry of a unit Sheepish transforms has already resolved (R427)"
        );
    }
}

mod r59_activating_a_power_whose_draw_s_cast_is_still_asking_runs_no_state_check {
    use super::*;

    #[test]
    fn r59_a_unit_the_power_s_draw_s_cast_brought_to_0_before_it_asked_is_still_there_to_be_saved_r156_2_4() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "backrow": [HEROIC_POWER], "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] },
        }));
        let power = must(s.backrow(P1, 1).cloned(), "p1's Heroic Power");
        // R43: the power lives on the instance; this one is "lose 2 health, draw 1".
        must(find_instance_mut(s.state_mut(), &power.id), "p1's Heroic Power")
            .memory
            .insert("power".to_string(), json!("draw"));
        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        let timmy_id = timmy.id.clone();
        // A cast-on-draw Spell: deal the 3/3 Timmy 5, then ask for a unit and give it +10 health.
        fixture(
            &mut s,
            "edge-r6-l7-power-cod",
            "Spell",
            Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                cry: Some(hook(move |_ctx| {
                    vec![
                        effects::damage(json_as(json!({ "to": { "of": "instance", "instanceId": timmy_id }, "amount": 5 }))),
                        effects::choose_target(json_as(json!({ "step": "save", "scope": any_unit() }))),
                    ]
                })),
                resume: IndexMap::from([(
                    "save",
                    hook(|_ctx| vec![effects::buff(json_as(json!({ "target": { "of": "chosen" }, "health": 10 })))]),
                )]),
                ..Script::default()
            },
            None,
        );
        on_top_of_library(&mut s, "edge-r6-l7-power-cod", P1);

        s.activate(&power.id, json!({}));
        must(s.state().pending.clone(), "the cast's target prompt");
        // R59: the check follows the whole power, and its draw's cast is still asking, so Timmy is still
        // on the field at -2 where the prompt offered it.
        s.expect_in_zone(&timmy.id, "field");

        s.answer(json!(timmy.id));
        s.expect_in_zone(&timmy.id, "field");
        assert_eq!(s.stats(&timmy.id).health, 8);
    }
}

mod r44_r113_my_pawn_s_ai_turn_is_a_sequence_and_the_other_player_s_prompt_only_pauses_it {
    use super::*;

    #[test]
    fn r44_the_ai_still_plays_out_the_rest_of_the_turn_once_the_opponent_has_answered_the_trap_its_play_set_off_r152_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "field": [TEMPO_TIMMY], "hand": [RENO, TEMPO_TIMMY], "mana": 4 },
            "p2": { "health": 3, "backrow": [{ "def": MY_PAWN, "lane": 1 }], "hand": [RENO] },
        }));
        asking_trap_on_play(&mut s, "edge-r6-l7-asks-ai");
        place_fixture(&mut s, "edge-r6-l7-asks-ai", P2, Row::Backrow, 2);
        let turn = s.state().turn;

        // Timmy's 3 would be lethal: My Pawn cancels it and hands p1's turn to the AI (R44), whose play
        // sets off p2's trap, which asks p2.
        let timmy = must(s.unit(P1, 1).cloned(), "p1's Tempo Timmy");
        s.attack(&timmy.id, "hero");
        assert!(s.state().players.p1.ai_turn);
        let pending = must(s.state().pending.clone(), "p2's trap question during the AI turn");
        assert_eq!(pending.player_id, P2);

        s.answer(json!("ok"));
        // R44: the AI plays out the rest of the turn while p1 is locked out, so once p2 has answered it
        // goes on until the turn ends; the turn is not left to a player the lockout keeps out of it.
        assert!(s.state().pending.is_none());
        assert!(s.state().turn > turn, "the AI turn should have played on to its end");
        assert!(!s.state().players.p1.ai_turn);
    }
}

mod r70_r81_a_cast_asks_for_the_choices_its_card_declares {
    use super::*;

    #[test]
    fn r70_a_cast_on_draw_spell_that_declares_a_target_asks_the_caster_for_it_r81() {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] },
        }));
        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        let declared: Vec<TargetDecl> =
            vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }))];
        fixture(
            &mut s,
            "edge-r6-l7-targeted-cod",
            "Spell",
            Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                targets: declared,
                cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 })))])),
                ..Script::default()
            },
            None,
        );
        on_top_of_library(&mut s, "edge-r6-l7-targeted-cod", P1);

        s.start_turn();
        // R81: a choice made during resolution — a cast's among them — is a prompt, and R70 gives it to
        // the caster. The play action that would have carried it never happened.
        let pending = must(s.state().pending.clone(), "the cast's target prompt");
        assert_eq!(pending.player_id, P1);
        s.answer(json!(timmy.id));
        s.expect_in_zone(&timmy.id, "graveyard");
    }
}

mod r87_r113_r423_call_to_chaos_s_play_waits_for_the_cast_its_recursion_is_still_asking_about {
    use super::*;

    #[test]
    fn r87_r423_a_radiant_call_to_chaos_whose_three_include_the_recursion_resolves_it_last_and_nothing_after_it_happens_while_its_cast_asks_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": CALL_TO_CHAOS, "radiant": true }, RENO, RENO],
                "library": [RENO, RENO, RENO],
                "health": 20,
                "mana": 4,
            },
            "p2": { "hand": [RENO] },
        }));
        // An onPlayHook that asks p1 something at §10.5 step 3 of every play, a cast's included (R70).
        fixture(
            &mut s,
            "edge-r6-l7-asks-on-every-play",
            "Field Spell",
            Script {
                on_play_hook: Some(hook(|_ctx| {
                    vec![effects::choose_mode(json_as(
                        json!({ "options": ["ok"], "step": "answered", "prompt": "on-play question" }),
                    ))]
                })),
                resume: IndexMap::from([("answered", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-l7-asks-on-every-play", P1, Row::Backrow, 1);
        let played = s.card(CALL_TO_CHAOS).clone();

        s.play(CALL_TO_CHAOS, json!({}));
        // The play's own step 3 asks first.
        must(s.state().pending.clone(), "the hook's question for the play of Call to Chaos");
        // R423: pin the roll the answer's Cry makes to three that include the recursion (its first rng
        // draw is the roll).
        let seed = s.state().seed.clone();
        let mut cursor: Option<u32> = None;
        let mut at: u32 = 0;
        while at < 5000 && cursor.is_none() {
            let names: Vec<String> =
                subsystems::roll_chaos_effects(&mut Rng::new(&seed, at), true, subsystems::CHAOS_EFFECTS)
                    .iter()
                    .map(|effect| effect.name.to_string())
                    .collect();
            if names.iter().any(|name| name == "recast") && !names.iter().any(|name| name == "draw") {
                cursor = Some(at);
            }
            at += 1;
        }
        // TS `expect(cursor).toBeGreaterThanOrEqual(0)`: a cursor was found.
        assert!(cursor.is_some());
        s.state_mut().rng_cursor = must(cursor, "a cursor whose radiant roll includes the recursion and not the draw");
        s.answer(json!("ok"));

        // The Cry rolled three; the recursion is the list's last, so it resolved last, and its cast's
        // step 3 asks.
        must(s.state().pending.clone(), "the hook's question for the cast Call to Chaos");
        let events: Vec<GameEvent> = s.last_events().to_vec();
        let opened = events
            .iter()
            .rposition(|event| matches!(event, GameEvent::PromptOpened { .. }));
        let after = types_of(&events[opened.map_or(0, |at| at + 1)..]);
        // R87, R113: a cast that is asking has not resolved, so nothing may have happened after the prompt —
        // not the cast's resolution, and not the played card's own landing.
        assert_eq!(
            after,
            Vec::<&str>::new(),
            "events after the cast's prompt opened: {}",
            after.join(", ")
        );
        assert!(!events
            .iter()
            .any(|event| matches!(event, GameEvent::CardResolved { instance_id, .. } if *instance_id == played.id)));

        s.answer(json!("ok"));
        assert!(s
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::CardResolved { instance_id, .. } if *instance_id == played.id)));
    }
}

mod r174_a_trap_owed_an_event_behind_another_trap_s_question_meets_the_event_as_the_board_now_stands {
    use super::*;

    #[test]
    fn r174_unlicensed_experimentation_owed_a_play_s_cardresolved_fuses_nothing_out_of_a_graveyard_once_the_trap_before_it_has_killed_the_card_r61()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [TEMPO_TIMMY, RENO], "mana": 4 },
            "p2": { "field": [RENO], "backrow": [{ "def": UNLICENSED_EXPERIMENTATION, "lane": 2 }], "hand": [RENO] },
        }));
        // Lane 1, ahead of #85 in R68's order: asks about the opponent's resolved permanent, then
        // destroys it.
        fixture(
            &mut s,
            "edge-r6-l7-asks-then-kills",
            "Trap",
            Script {
                triggers: vec![
                    TriggerDef::new("edge-r6-l7-asks-then-kills:asks", &[GameEventType::CardResolved], |_ctx, event| {
                        vec![effects::choose_mode(json_as(json!({
                            "options": ["ok"],
                            "step": "answered",
                            "prompt": "a question",
                            "data": { "played": resolved_id(event) },
                        })))]
                    })
                    .with_when(permanent_resolved_by_opponent),
                ],
                resume: IndexMap::from([(
                    "answered",
                    hook(|ctx| {
                        vec![effects::destroy(json_as(
                            json!({ "target": { "of": "instance", "instanceId": data_string(ctx, "played") } }),
                        ))]
                    }),
                )]),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-l7-asks-then-kills", P2, Row::Backrow, 1);
        let reno = must(s.unit(P2, 1).cloned(), "p2's Reno");

        s.play(TEMPO_TIMMY, json!({}));
        must(s.state().pending.clone(), "the first trap's question");
        s.answer(json!("ok"));

        // The played Timmy died before #85 was offered the event, so the play is no longer in play:
        // #85 is not set off (R61, R99), and Timmy stays in p1's graveyard rather than being fused away.
        let timmy = must(
            s.pile(P1, "graveyard").iter().find(|card| card.def_id == TEMPO_TIMMY).cloned(),
            "Timmy in p1's graveyard",
        );
        s.expect_in_zone(&timmy.id, "graveyard");
        assert_eq!(s.card(&reno.id).def_id, RENO);
        assert_eq!(
            s.backrow(P2, 2).map(|card| card.def_id.clone()),
            Some(UNLICENSED_EXPERIMENTATION.to_string())
        );
    }
}

mod r174_r113_a_list_s_tail_after_a_prompt_still_meets_the_stay_the_play_chose {
    use super::*;

    #[test]
    fn r174_a_unit_the_list_sacrificed_before_its_prompt_is_gone_for_the_damage_after_it_even_back_through_reborn_r83_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "field": [RIGHT_HOUSE_DEFENDER], "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        let saintess = must(s.unit(P1, 1).cloned(), "p1's Right-house defender");
        // A 0-cost Spell: sacrifice your chosen unit, ask something, then deal the chosen unit 5.
        fixture(
            &mut s,
            "edge-r6-l7-sac-ask-hit",
            "Spell",
            Script {
                targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit"] } }))],
                cry: Some(hook(|_ctx| {
                    vec![
                        effects::sacrifice(json_as(json!({ "target": { "of": "chosen" } }))),
                        effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "answered", "prompt": "a question" }))),
                        effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 }))),
                    ]
                })),
                resume: IndexMap::from([("answered", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            None,
        );
        let spell = new_instance(s.state_mut(), "edge-r6-l7-sac-ask-hit", P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(spell.clone());

        s.play(&spell.id, json!({ "targets": [{ "pick": "instance", "instanceId": saintess.id }] }));
        must(s.state().pending.clone(), "the Spell's question");
        // The sacrifice is a death in full: Reborn has already brought the Saintess back, a new arrival.
        assert_eq!(s.card(&saintess.id).reborn_spent, Some(true));
        s.answer(json!("ok"));

        // R174: the damage was aimed at the stay the play chose, which the sacrifice ended; the list is
        // one effect list whether or not a prompt split it (R113), so the damage fizzles as it does with
        // no question between (a fused Cube+Sorcerer, re-entry.test.ts). The body stands at 1 health.
        s.expect_in_zone(&saintess.id, "field");
        s.expect_stats(&saintess.id, json!({ "health": 1 }));
    }
}

mod r156_r113_a_death_pass_whose_hook_ends_in_a_nested_death_that_asks_still_owes_the_rest_of_the_pass {
    use super::*;

    #[test]
    fn r156_the_reborn_unit_collected_with_a_unit_whose_death_sacrificed_an_asking_unit_still_comes_back_once_the_question_is_answered_r64_4_5()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": HIT_JOB, "radiant": true }, RENO], "field": [{ "def": RIGHT_HOUSE_DEFENDER, "lane": 2 }], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // Lane 5: a 2/2 whose Death asks its controller something.
        fixture(
            &mut s,
            "edge-r6-l7-asking-death",
            "Unit",
            Script {
                death: Some(hook(|_ctx| {
                    vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "answered", "prompt": "a last word" })))]
                })),
                resume: IndexMap::from([("answered", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            Some(json!({ "attack": 2, "health": 2 })),
        );
        let asker = place_fixture(&mut s, "edge-r6-l7-asking-death", P1, Row::Units, 5);
        let asker_id = asker.id.clone();
        // Lane 1: a 2/2 whose Death sacrifices the asker, as its last effect.
        fixture(
            &mut s,
            "edge-r6-l7-sacrificing-death",
            "Unit",
            Script {
                death: Some(hook(move |_ctx| {
                    vec![effects::sacrifice(json_as(json!({ "target": { "of": "instance", "instanceId": asker_id } })))]
                })),
                ..Script::default()
            },
            Some(json!({ "attack": 2, "health": 2 })),
        );
        let sacrificer = place_fixture(&mut s, "edge-r6-l7-sacrificing-death", P1, Row::Units, 1);
        let defender = must(s.unit(P1, 2).cloned(), "p1's Right-house defender");

        // Radiant Hit Job on the defender destroys it and the unit beside it in lane 1 (lane 3 is empty).
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": defender.id }] }));
        s.expect_in_zone(&sacrificer.id, "graveyard");
        s.expect_in_zone(&asker.id, "graveyard");
        must(s.state().pending.clone(), "the sacrificed unit's Death question");
        // R156: while the question stands the Reborn unit waits in the graveyard, its zone reserved.
        s.expect_in_zone(&defender.id, "graveyard");

        s.answer(json!("ok"));
        // §4.5 step 4 is still owed once the last Death hook has run: the defender comes back.
        s.expect_in_zone(&defender.id, "field");
        assert_eq!(s.card(&defender.id).reborn_spent, Some(true));
    }
}

mod r68_r62_a_delayed_effect_made_while_its_point_is_resolving_waits_for_the_next_one {
    use super::*;

    #[test]
    fn r68_a_start_of_turn_delayed_effect_scheduled_by_the_answer_to_an_earlier_one_s_question_fires_at_the_next_start_of_turn_not_this_one_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "hand": [RENO] },
        }));
        // A p1 Field Spell whose start-of-turn delayed effect asks, and whose answer schedules "at the
        // start of your next turn: deal 5 damage to the enemy hero".
        fixture(
            &mut s,
            "edge-r6-l7-reschedules",
            "Field Spell",
            Script {
                resume: IndexMap::from([
                    (
                        "armed",
                        hook(|_ctx| {
                            vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "answered", "prompt": "a question" })))]
                        }),
                    ),
                    (
                        "answered",
                        hook(|_ctx| {
                            vec![effects::delay(json_as(
                                json!({ "at": { "phase": "start", "player": "self" }, "step": "later", "hook": "resume" }),
                            ))]
                        }),
                    ),
                    (
                        "later",
                        hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))]),
                    ),
                ]),
                ..Script::default()
            },
            None,
        );
        let card = place_fixture(&mut s, "edge-r6-l7-reschedules", P1, Row::Backrow, 5);
        let resume = Resume {
            def_id: card.def_id.clone(),
            hook: "resume".to_string(),
            step: "armed".to_string(),
            radiant: false,
            instance_id: Some(card.id.clone()),
            data: IndexMap::new(),
        };
        with_sink(&mut s, |sink| {
            schedule_delayed(sink, P1, DelayedAt { phase: Phase::Start, player: P1 }, resume, None, None);
        });

        s.start_turn();
        must(s.state().pending.clone(), "the delayed effect's question");
        s.answer(json!("ok"));

        // The new delayed effect was made at this start of turn, so it is due at p1's next one (R62: the
        // stage resolves the effects due as it begins, in creation order, R68), as it would be had the
        // first one not asked anything.
        s.expect_health(P2, 30);
        assert_eq!(
            s.state()
                .delayed
                .iter()
                .filter(|effect| effect.at.phase == Phase::Start && effect.at.player == P1)
                .count(),
            1
        );
    }
}

mod s10_5_step_3_r113_step_3_s_hooks_resume_where_they_stopped_whatever_the_answer_did_to_the_board {
    use super::*;

    #[test]
    fn s10_5_step_3_the_second_onplayhook_still_runs_for_the_play_when_the_first_one_s_answer_took_its_own_card_off_the_field_r113_r153()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [TEMPO_TIMMY, RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // Lane 1: asks at every play of its controller's, and the answer returns it to its owner's hand.
        fixture(
            &mut s,
            "edge-r6-l7-hook-leaves",
            "Field Spell",
            Script {
                on_play_hook: Some(hook(|_ctx| {
                    vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "answered", "prompt": "a question" })))]
                })),
                resume: IndexMap::from([(
                    "answered",
                    hook(|_ctx| vec![effects::bounce(json_as(json!({ "target": { "of": "self" } })))]),
                )]),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-l7-hook-leaves", P1, Row::Backrow, 1);
        // Lane 2: deals the enemy hero 1 at every play.
        fixture(
            &mut s,
            "edge-r6-l7-hook-pings",
            "Field Spell",
            Script {
                on_play_hook: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
                })),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-l7-hook-pings", P1, Row::Backrow, 2);

        s.play(TEMPO_TIMMY, json!({}));
        must(s.state().pending.clone(), "the first hook's question");
        s.answer(json!("ok"));

        // Both hooks were on the field when the play reached step 3, in R68's order: the first one
        // leaving on its own answer does not drop the second (R113: a sequence is never dropped).
        s.expect_health(P2, 29);
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lens L7): the stays and faces a paused or queued list carries, the plays a trap answering
// at step 4 has taken off the field, and the order an answer to a cast's own choice finishes in.
// ---------------------------------------------------------------------------

/// A fixture def with its own base and radiant scripts and a face of its own, held like a fusion's.
/// `options` is the TS literal `{ stats?: { attack, health }; keywords?: Keyword[] }` (`json!({})`
/// for none).
fn fixture_faces(state: &mut GameState, id: &str, ty: &str, scripts: CardScripts, options: Value) {
    let stats = options.get("stats").cloned().unwrap_or_else(|| json!({ "attack": 2, "health": 2 }));
    let keywords = options.get("keywords").cloned().unwrap_or_else(|| json!([]));
    let face = fixture_face(id, ty, &stats, &keywords);
    state.transient_defs.insert(id.to_string(), fixture_def(id, ty, 0, &face));
    register_fixture_scripts(id, scripts);
}

mod s5_2_r113_a_queued_trigger_s_parked_tail_runs_the_face_its_head_ran {
    use super::*;

    #[test]
    fn r113_a_trigger_whose_card_turned_radiant_while_it_waited_in_the_queue_finishes_on_the_radiant_text_after_its_prompt_5_2()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // Lane 2: on any play, deal the enemy hero 1 (radiant 2), ask something, then deal it 10
        // (radiant 20). The same trigger id on both faces, as a card's radiant face keeps its triggers.
        let asking = |first: i32, second: i32| -> Script {
            Script {
                triggers: vec![TriggerDef::new("edge-r7-l7-face", &[GameEventType::CardPlayed], move |_ctx, _event| {
                    vec![
                        effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": first }))),
                        effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" }))),
                        effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": second }))),
                    ]
                })],
                resume: IndexMap::from([("ok", hook(|_ctx| vec![]))]),
                ..Script::default()
            }
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-face",
            "Unit",
            CardScripts { base: asking(1, 10), radiant: asking(2, 20) },
            json!({}),
        );
        let faced = place_fixture(&mut s, "edge-r7-l7-face", P1, Row::Units, 2);
        let faced_id = faced.id.clone();

        // Lane 1, ahead of it in R68's order: on any play, make the lane-2 unit Radiant.
        let radiates = Script {
            triggers: vec![TriggerDef::new("edge-r7-l7-radiates", &[GameEventType::CardPlayed], move |_ctx, _event| {
                vec![effects::set_radiant(json_as(json!({ "instanceId": faced_id })))]
            })],
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-radiates",
            "Unit",
            CardScripts { base: radiates.clone(), radiant: radiates },
            json!({}),
        );
        place_fixture(&mut s, "edge-r7-l7-radiates", P1, Row::Units, 1);

        // Both triggers are queued on Reno's `cardPlayed` while the lane-2 unit is still base. The first
        // makes it Radiant, so by the time its own trigger resolves it runs the radiant text (§5.2:
        // "ongoing triggers use the radiant text from then on"; `runQueuedTrigger` reads the card again).
        s.play(RENO, json!({ "zone": 3 }));
        assert!(s.card(&faced.id).radiant);
        must(s.state().pending.clone(), "the lane-2 trigger's question");
        // Its first damage came from the radiant text.
        s.expect_health(P2, 28);

        s.answer(json!("ok"));
        // One trigger, one face: the rest of its list after the prompt is the radiant text's 20, not the
        // base text's 10 (R113: the paused list continues, it is not a different list).
        s.expect_health(P2, 8);
    }
}

mod r174_10_5_step_3_step_3_s_hooks_are_the_stays_the_step_began_with {
    use super::*;

    #[test]
    fn r174_a_reborn_body_is_not_run_as_an_onplayhook_holder_of_the_play_whose_earlier_hook_s_answer_killed_it_10_5_step_3_r83()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // Lane 2: a Reborn unit whose onPlayHook deals the enemy hero 5.
        let pings = Script {
            on_play_hook: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))])),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-pinger",
            "Unit",
            CardScripts { base: pings.clone(), radiant: pings },
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        let pinger = place_fixture(&mut s, "edge-r7-l7-pinger", P1, Row::Units, 2);
        let pinger_id = pinger.id.clone();

        // Lane 1, ahead of it in R68's order: its onPlayHook asks, and the answer sacrifices the pinger.
        let asks = Script {
            on_play_hook: Some(hook(|_ctx| {
                vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" })))]
            })),
            resume: IndexMap::from([(
                "ok",
                hook(move |_ctx| {
                    vec![effects::sacrifice(json_as(json!({ "target": { "of": "instance", "instanceId": pinger_id } })))]
                }),
            )]),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-asker",
            "Unit",
            CardScripts { base: asks.clone(), radiant: asks },
            json!({}),
        );
        place_fixture(&mut s, "edge-r7-l7-asker", P1, Row::Units, 1);

        s.play(RENO, json!({ "zone": 3 }));
        must(s.state().pending.clone(), "the lane-1 hook's question");
        s.answer(json!("ok"));

        // The sacrifice is a death in full, and Reborn has put a new body in lane 2 (§4.5 step 4).
        assert_eq!(s.card(&pinger.id).reborn_spent, Some(true));
        s.expect_in_zone(&pinger.id, "field");
        // R174: the stay step 3 began with has ended, and the body is a new arrival that was not a holder
        // when the play reached step 3 (as a start-of-turn hook queued before a death does not fire for
        // the Reborn body, R174), so it deals nothing for this play.
        s.expect_health(P2, 30);
    }
}

mod r1_r118_r174_a_played_unit_a_trap_killed_at_step_4_does_not_cry_with_its_reborn_body {
    use super::*;

    #[test]
    fn r118_a_trap_that_asks_and_then_destroys_the_played_reborn_unit_leaves_the_play_no_cry_to_resolve_r1_r174_10_5_step_5()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // p1's 0-cost Reborn unit whose Cry deals the enemy hero 5.
        let cries = Script {
            cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))])),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-crier",
            "Unit",
            CardScripts { base: cries.clone(), radiant: cries },
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        let crier = new_instance(s.state_mut(), "edge-r7-l7-crier", P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(crier.clone());

        // p2's trap: when the opponent plays a card, ask p2 something, then destroy the played card.
        let trap = Script {
            triggers: vec![
                TriggerDef::new("edge-r7-l7-kill-on-play", &[GameEventType::CardPlayed], |_ctx, event| {
                    vec![effects::choose_mode(json_as(json!({
                        "options": ["ok"],
                        "step": "ok",
                        "prompt": "a question",
                        "data": { "played": played_id(event) },
                    })))]
                })
                .with_when(played_by_opponent),
            ],
            resume: IndexMap::from([(
                "ok",
                hook(|ctx| {
                    vec![effects::destroy(json_as(
                        json!({ "target": { "of": "instance", "instanceId": data_string(ctx, "played") } }),
                    ))]
                }),
            )]),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-kill-on-play",
            "Trap",
            CardScripts { base: trap.clone(), radiant: trap },
            json!({}),
        );
        place_fixture(&mut s, "edge-r7-l7-kill-on-play", P2, Row::Backrow, 1);

        s.play(&crier.id, json!({ "zone": 1 }));
        assert_eq!(must(s.state().pending.clone(), "the trap's question").player_id, P2);
        s.answer(json!("ok"));

        // The trap resolved to completion before the play went on (§10.3): the unit died and Reborn put a
        // new body in its zone (§4.5 step 4).
        assert_eq!(s.card(&crier.id).reborn_spent, Some(true));
        s.expect_in_zone(&crier.id, "field");
        // R118: the Cry is lost where the trap has taken the card off the field. The body is a new
        // arrival (R174, R83), and R1: Reborn never fires a Cry.
        s.expect_health(P2, 30);
    }
}

/// p2's trap that asks p2 when the opponent plays a card, and then applies `after` to the played card.
fn ask_then_on_played(s: &mut Scenario, id: &str, after: impl Fn(String) -> Effect + Send + Sync + 'static) {
    let trap = Script {
        triggers: vec![
            TriggerDef::new(id, &[GameEventType::CardPlayed], |_ctx, event| {
                vec![effects::choose_mode(json_as(json!({
                    "options": ["ok"],
                    "step": "ok",
                    "prompt": "a question",
                    "data": { "played": played_id(event) },
                })))]
            })
            .with_when(played_by_opponent),
        ],
        resume: IndexMap::from([("ok", hook(move |ctx| vec![after(data_string(ctx, "played"))]))]),
        ..Script::default()
    };
    fixture_faces(s.state_mut(), id, "Trap", CardScripts { base: trap.clone(), radiant: trap }, json!({}));
    place_fixture(s, id, P2, Row::Backrow, 1);
}

mod r174_r17_sheepish_owed_the_play_behind_a_trap_that_took_the_unit_off_the_field_transforms_nothing {
    use super::*;

    #[test]
    fn r174_a_unit_the_first_trap_s_answer_bounced_to_its_controller_s_hand_is_not_turned_into_a_sheep_token_card_there_r17_8_41()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [TEMPO_TIMMY, RENO], "mana": 4 },
            "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }], "hand": [RENO] },
        }));
        ask_then_on_played(&mut s, "edge-r7-l7-bounce-on-play", |played| {
            effects::bounce(json_as(json!({ "target": { "of": "instance", "instanceId": played } })))
        });
        let timmy = s.card(TEMPO_TIMMY).clone();

        s.play(TEMPO_TIMMY, json!({ "zone": 1 }));
        assert_eq!(must(s.state().pending.clone(), "the first trap's question").player_id, P2);
        s.answer(json!("ok"));

        // The first trap resolved to completion (§10.3): Timmy is back in p1's hand.
        assert!(s.state().pending.is_none());
        // R174: a trap answering a play meets it as no longer in play once an earlier trap answering the
        // same play has taken the card off the field. Sheepish transforms the unit the opponent played
        // (§8 #41), on the field; it never reaches into a hand to rewrite a card there.
        assert!(
            !s.hand(P1).iter().any(|card| card.def_id == SHEEP_TOKEN),
            "a Sheep Token card in p1's hand"
        );
        s.expect_in_zone(&timmy.id, "hand");
    }

    #[test]
    fn r174_a_unit_the_first_trap_s_answer_killed_and_reborn_brought_back_is_not_transformed_as_the_unit_that_was_played_r17_r83()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }], "hand": [RENO] },
        }));
        let plain = Script::default();
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-reborn-unit",
            "Unit",
            CardScripts { base: plain.clone(), radiant: plain },
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        let unit = new_instance(s.state_mut(), "edge-r7-l7-reborn-unit", P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(unit.clone());
        ask_then_on_played(&mut s, "edge-r7-l7-destroy-on-play", |played| {
            effects::destroy(json_as(json!({ "target": { "of": "instance", "instanceId": played } })))
        });

        s.play(&unit.id, json!({ "zone": 1 }));
        must(s.state().pending.clone(), "the first trap's question");
        s.answer(json!("ok"));

        // The unit died and its Reborn body stands in lane 1: a new arrival nobody played (R83).
        assert!(s
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == unit.id)));
        // R174: the play Sheepish answers is no longer in play, so the body is not transformed.
        let lane1 = s.unit(P1, 1).cloned();
        assert_eq!(
            lane1.as_ref().map(|card| card.def_id.as_str()),
            Some("edge-r7-l7-reborn-unit"),
            "lane 1 should hold the Reborn body, not a Sheep Token"
        );
        assert_eq!(lane1.as_ref().map(|card| card.id.clone()), Some(unit.id.clone()));
        assert_eq!(lane1.as_ref().and_then(|card| card.reborn_spent), Some(true));
    }
}

const BEAR_HONEYPOT: &str = "core-060";

mod r122_2_4_the_answer_to_a_cast_s_own_choice_goes_on_with_the_draw_chain_not_the_resolution_loop {
    use super::*;

    #[test]
    fn r122_the_cast_on_draw_card_under_a_cast_that_asked_for_its_target_is_drawn_and_cast_before_bear_honeypot_answers_the_first_cast_r113_r70_2_4()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "backrow": [BEAR_HONEYPOT], "hand": [RENO] },
        }));
        // Top card: a cast-on-draw Spell that declares a target, so its cast asks for it (R70, R81).
        let targeted = Script {
            static_flags: flags(json!({ "castOnDraw": true })),
            targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit", "hero"] } }))],
            cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 })))])),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-cod-targeted",
            "Spell",
            CardScripts { base: targeted.clone(), radiant: targeted },
            json!({}),
        );
        // Under it: a cast-on-draw Spell that deals 1 damage to each enemy unit.
        let sweep = Script {
            static_flags: flags(json!({ "castOnDraw": true })),
            cry: Some(hook(|_ctx| vec![effects::damage_all(json_as(json!({ "amount": 1, "side": "enemy" })))])),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-cod-sweep",
            "Spell",
            CardScripts { base: sweep.clone(), radiant: sweep },
            json!({}),
        );
        let first = new_instance(s.state_mut(), "edge-r7-l7-cod-targeted", P1, Zone::Library { player: P1 });
        let second = new_instance(s.state_mut(), "edge-r7-l7-cod-sweep", P1, Zone::Library { player: P1 });
        // TS `library.unshift(first, second)`: both on top, `first` above `second`.
        for (at, card) in [first.clone(), second.clone()].into_iter().enumerate() {
            s.state_mut().players.p1.library.insert(at, card);
        }

        // p1's turn draw casts the first; its cast asks p1 for a target (R70, R81).
        s.start_turn();
        assert_eq!(must(s.state().pending.clone(), "the cast's target prompt").player_id, P1);
        s.answer(json!([{ "pick": "hero", "player": "p2" }]));

        // §2.4: the draw repeats as soon as the cast has resolved, so the sweep is drawn and cast inside
        // the same draw. R122: the answer goes on with what the prompt interrupted, the draw chain, and
        // not with the resolution loop. The traps meet the first cast's resolution at the sweep's own
        // announce (§10.5 step 3a), the first window a cast opens, as a play's does (R70, R448): every
        // event so far reaches the traps there, before the sweep moves or resolves. So Bear Honeypot's
        // tokens arrive before the sweep's 1 damage to each enemy unit, and it hits them.
        let events: Vec<GameEvent> = s.last_events().to_vec();
        let types = types_of(&events);
        let sweep_drawn = find_index(
            &events,
            |event| matches!(event, GameEvent::Drawn { instance_id, .. } if *instance_id == second.id),
        );
        let sweep_announced = find_index(
            &events,
            |event| matches!(event, GameEvent::CardAnnounced { instance_id, .. } if *instance_id == second.id),
        );
        let sweep_cast = find_index(
            &events,
            |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == second.id),
        );
        let trap_fired = type_index(&types, "trapFired");
        assert!(sweep_drawn >= 0, "events: {}", types.join(", "));
        assert!(sweep_announced > sweep_drawn, "events: {}", types.join(", "));
        assert!(trap_fired > sweep_announced, "events: {}", types.join(", "));
        assert!(sweep_cast > trap_fired, "events: {}", types.join(", "));
        let tokens: Vec<CardInstance> = [1, 2, 3, 4, 5]
            .into_iter()
            .filter_map(|lane| s.unit(P2, lane).cloned())
            .filter(|unit| unit.def_id == RUSH_TOKEN)
            .collect();
        assert_eq!(tokens.len(), 2);
        for token in &tokens {
            assert_eq!(token.damage, 1);
        }
    }
}

mod r136_r113_a_list_a_prompt_split_still_reads_the_events_its_own_head_emitted {
    use super::*;

    #[test]
    fn r136_the_rush_token_a_trap_s_list_summoned_before_its_prompt_is_still_one_of_they_that_attack_after_the_answer_r113_8_60()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO, RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // p2's trap, #60 Bear Honeypot's list with a question in the middle: when the opponent's card
        // resolves, summon a Rush Token, ask p2 something, then the tokens this list summoned attack it.
        let trap = Script {
            triggers: vec![
                TriggerDef::new("edge-r7-l7-honeypot-asks", &[GameEventType::CardResolved], |_ctx, event| {
                    vec![
                        effects::summon(json_as(json!({ "defId": RUSH_TOKEN }))),
                        effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" }))),
                        effects::forced_attacks(json_as(json!({
                            "attackers": { "side": "self", "defId": RUSH_TOKEN, "summonedThisScript": true },
                            "target": { "instanceId": resolved_id(event) },
                        }))),
                    ]
                })
                .with_when(permanent_resolved_by_opponent),
            ],
            resume: IndexMap::from([("ok", hook(|_ctx| vec![]))]),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-honeypot-asks",
            "Trap",
            CardScripts { base: trap.clone(), radiant: trap },
            json!({}),
        );
        place_fixture(&mut s, "edge-r7-l7-honeypot-asks", P2, Row::Backrow, 1);

        s.play(RENO, json!({ "zone": 1 }));
        let reno = must(s.unit(P1, 1).cloned(), "p1's Reno");
        must(s.state().pending.clone(), "the trap's question");
        let token = must(s.unit(P2, 1).cloned(), "the Rush Token the trap summoned");
        assert_eq!(token.def_id, RUSH_TOKEN);
        s.answer(json!("ok"));

        // R113: the list after the prompt is the same list, and R136's "the events its own script
        // emitted" include the summon before the prompt: the token attacks Reno (3 damage) and dies to
        // Reno's 4 strike back.
        assert_eq!(s.card(&reno.id).damage, 3);
        s.expect_in_zone(&token.id, "gone");
    }
}

mod r174_r113_this_unit_later_in_a_list_a_prompt_split_is_the_stay_the_list_began_with {
    use super::*;

    #[test]
    fn r174_a_cry_that_sacrificed_its_own_unit_before_its_prompt_does_not_buff_the_reborn_body_after_the_answer_r83_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // A 0-cost 2/2 Reborn unit whose Cry sacrifices itself, asks something, then gives itself +5/+5.
        let cry = Script {
            cry: Some(hook(|_ctx| {
                vec![
                    effects::sacrifice(json_as(json!({ "target": { "of": "self" } }))),
                    effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" }))),
                    effects::buff(json_as(json!({ "target": { "of": "self" }, "attack": 5, "health": 5 }))),
                ]
            })),
            resume: IndexMap::from([("ok", hook(|_ctx| vec![]))]),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-self-sac",
            "Unit",
            CardScripts { base: cry.clone(), radiant: cry },
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        let unit = new_instance(s.state_mut(), "edge-r7-l7-self-sac", P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(unit.clone());

        s.play(&unit.id, json!({ "zone": 1 }));
        must(s.state().pending.clone(), "the Cry's question");
        // The sacrifice is a death in full: Reborn has already put a new body in lane 1 (§4.5 step 4).
        assert_eq!(s.card(&unit.id).reborn_spent, Some(true));
        s.answer(json!("ok"));

        // R174: an effect later in the same list aimed at a card an earlier effect took off the field
        // fizzles even once the card is back, and a prompt between them changes none of this (R113). The
        // body is a new arrival (R83): it stays the printed 2/2 at 1 health.
        s.expect_in_zone(&unit.id, "field");
        s.expect_stats(&unit.id, json!({ "attack": 2, "maxHealth": 2, "health": 1 }));
    }
}

mod r174_r113_the_step_a_prompt_s_answer_re_enters_reads_the_stays_the_resolution_began_with {
    use super::*;

    #[test]
    fn r174_a_delayed_steal_an_answered_step_schedules_on_a_unit_its_own_list_sacrificed_before_the_prompt_fizzles_reborn_body_or_not_r76_r83_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        let plain = Script::default();
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-enemy-reborn",
            "Unit",
            CardScripts { base: plain.clone(), radiant: plain },
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        let victim = place_fixture(&mut s, "edge-r7-l7-enemy-reborn", P2, Row::Units, 1);

        // A 0-cost Spell: sacrifice the chosen enemy unit, ask something, and on the answer schedule
        // "at the start of your next turn, steal it", watching it (#50 K-Pop Fanatic's steal, R76).
        let spell = Script {
            targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }))],
            cry: Some(hook(|ctx| {
                let id = match ctx.targets.first() {
                    Some(Selection::Instance { instance_id }) => instance_id.clone(),
                    _ => String::new(),
                };
                vec![
                    effects::sacrifice(json_as(json!({ "target": { "of": "chosen" }, "allowEnemy": true }))),
                    effects::choose_mode(json_as(
                        json!({ "options": ["ok"], "step": "ok", "prompt": "a question", "data": { "victim": id } }),
                    )),
                ]
            })),
            resume: IndexMap::from([
                (
                    "ok",
                    hook(|ctx| {
                        vec![effects::delay(json_as(json!({
                            "at": { "phase": "start", "player": "self" },
                            "step": "steal",
                            "hook": "resume",
                            "watch": data_string(ctx, "victim"),
                            "data": { "victim": ctx.data.get("victim").cloned() },
                        })))]
                    }),
                ),
                (
                    "steal",
                    hook(|ctx| vec![effects::steal(json_as(json!({ "instanceId": data_string(ctx, "victim") })))]),
                ),
            ]),
            ..Script::default()
        };
        fixture_faces(
            s.state_mut(),
            "edge-r7-l7-sac-then-steal",
            "Spell",
            CardScripts { base: spell.clone(), radiant: spell },
            json!({}),
        );
        let card = new_instance(s.state_mut(), "edge-r7-l7-sac-then-steal", P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(card.clone());

        s.play(&card.id, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
        must(s.state().pending.clone(), "the Spell's question");
        // The sacrifice is a death in full: the victim's Reborn body already stands in p2's lane 1.
        assert_eq!(s.card(&victim.id).reborn_spent, Some(true));
        s.answer(json!("ok"));

        // R174: the steal is aimed at the stay the play chose, which the sacrifice ended before the
        // question. The answer continues the same resolution (R113, §10.6), so it is never scheduled for
        // the body that came back, as it is not when a fused card's parts do the same (re-entry.test.ts).
        let scheduled: Vec<DelayedEffect> = s
            .state()
            .delayed
            .iter()
            .filter(|effect| effect.resume.def_id == "edge-r7-l7-sac-then-steal")
            .cloned()
            .collect();
        assert_eq!(scheduled, Vec::<DelayedEffect>::new());
    }
}

// ---------------------------------------------------------------------------
// Round 8 (lens L7): an Echo repeat's fresh picks, the traps a paused dispatch still owes and their
// order, a card named by id or picked at a prompt after the list's own sacrifice, a played card a
// Tribute's question discarded, and the check after a trigger that asked at step 4.
// ---------------------------------------------------------------------------

const FULLSEND: &str = "core-078";

fn same(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

fn in_hand(s: &mut Scenario, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    s.state_mut().players[player].hand.push(card.clone());
    card
}

fn on_library_top(s: &mut Scenario, def_ids: &[&str], player: PlayerId) -> Vec<CardInstance> {
    let cards: Vec<CardInstance> = def_ids
        .iter()
        .map(|def_id| new_instance(s.state_mut(), def_id, player, Zone::Library { player }))
        .collect();
    // TS `library.unshift(...cards)`: all on top, in their order.
    for (at, card) in cards.iter().enumerate() {
        s.state_mut().players[player].library.insert(at, card.clone());
    }
    cards
}

// ---------------------------------------------------------------------------------------------
// 1. An Echo repeat's fresh choices and the stays they were made on (R174, §10.5 step 6)
// ---------------------------------------------------------------------------------------------

mod r174_10_5_step_6_an_echo_repeat_s_fresh_target_is_aimed_at_the_stay_it_was_chosen_on {
    use super::*;

    /// p2's 5/5 Reborn unit in lane 1, a p1 Spell "deal 1 damage to target enemy unit", and /fullsend.
    fn board(echo: i32) -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            // The Radiant /fullsend: the face that grants "Combo: Draw 1" since patch v0.1.1.
            "p1": { "hand": [{ "def": FULLSEND, "radiant": true }], "library": [RENO, RENO, RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        fixture_faces(
            s.state_mut(),
            "edge-r8-reborn-5-5",
            "Unit",
            same(Script::default()),
            json!({ "stats": { "attack": 5, "health": 5 }, "keywords": [{ "kind": "Reborn" }] }),
        );
        let victim = place_fixture(&mut s, "edge-r8-reborn-5-5", P2, Row::Units, 1);
        let id = format!("edge-r8-ping-echo-{echo}");
        fixture_faces(
            s.state_mut(),
            &id,
            "Spell",
            same(Script {
                static_flags: flags(json!({ "echo": echo })),
                targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }))],
                cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 })))])),
                ..Script::default()
            }),
            json!({}),
        );
        let spell = in_hand(&mut s, &id, P1);
        // A cast-on-draw Spell that destroys the victim.
        let victim_id = victim.id.clone();
        fixture_faces(
            s.state_mut(),
            "edge-r8-cod-destroyer",
            "Spell",
            same(Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                cry: Some(hook(move |_ctx| {
                    vec![effects::destroy(json_as(json!({ "target": { "of": "instance", "instanceId": victim_id } })))]
                })),
                ..Script::default()
            }),
            json!({}),
        );
        (s, victim, spell)
    }

    #[test]
    fn r174_a_target_chosen_at_the_repeat_s_prompt_that_the_repeat_s_combo_draw_killed_is_gone_for_the_repeat_s_script_reborn_body_or_not_r81_r83()
     {
        register_all();
        // Control, no Echo: the play's own Combo draw (step 5, before the script) casts the destroyer, and
        // the script's damage, aimed at the stay step 1 checked, fizzles on the Reborn body.
        let (mut control, control_victim, control_spell) = board(0);
        on_library_top(&mut control, &["edge-r8-cod-destroyer"], P1);
        control.play(FULLSEND, json!({}));
        control.play(
            &control_spell.id,
            json!({ "targets": [{ "pick": "instance", "instanceId": control_victim.id }] }),
        );
        assert_eq!(control.card(&control_victim.id).reborn_spent, Some(true));
        control.expect_in_zone(&control_victim.id, "field");
        control.expect_stats(&control_victim.id, json!({ "health": 1 }));

        // Echo 1. The first resolution's Combo draw takes a Reno; the repeat's takes the destroyer.
        let (mut s, victim, spell) = board(1);
        on_library_top(&mut s, &[RENO, "edge-r8-cod-destroyer"], P1);
        // /fullsend: "this turn your cards gain 'Combo: draw 1'", and it is the card played earlier.
        s.play(FULLSEND, json!({}));
        s.play(&spell.id, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
        // The first resolution hit the 5/5 for 1; the repeat asks for a fresh target (R81).
        assert_eq!(s.card(&victim.id).damage, 1);
        let pending = must(s.state().pending.clone(), "the Echo repeat's target prompt");
        assert!(pending.prompt.contains("Echo"));
        s.answer(json!(victim.id));

        // The repeat is step 5 again: its Combo draw casts the destroyer, which kills the victim, and
        // Reborn puts a new body in lane 1 (§4.5 step 4, R83) before the repeat's script runs.
        let events: Vec<GameEvent> = s.last_events().to_vec();
        let reborn = find_index(
            &events,
            |event| matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == victim.id),
        );
        assert!(reborn >= 0, "the victim's Reborn body came back during the repeat");
        // R174: the repeat's damage was aimed at the stay chosen at its prompt, which the cast ended, as
        // the control's was aimed at step 1's. The body is a new arrival and stands at 1 health; before
        // the fix the repeat's damage landed on it and it died a second time.
        s.expect_in_zone(&victim.id, "field");
        s.expect_stats(&victim.id, json!({ "health": 1 }));
    }
}

// ---------------------------------------------------------------------------------------------
// 2. The traps an event is still owed after a question are the ones not yet offered it (§10.3, R99)
// ---------------------------------------------------------------------------------------------

/// p2's trap in `lane`: when the opponent plays a card, and only once p1's hero is below 30, deal it 5.
fn conditional_trap(s: &mut Scenario, id: &str, lane: i32) -> CardInstance {
    fixture_faces(
        s.state_mut(),
        id,
        "Trap",
        same(Script {
            triggers: vec![
                TriggerDef::new(format!("{id}:hits"), &[GameEventType::CardPlayed], |_ctx, _event| {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))]
                })
                .with_when(|ctx, event| played_by_opponent(ctx, event) && ctx.state.players.p1.hero.health < 30),
            ],
            ..Script::default()
        }),
        json!({}),
    );
    place_fixture(s, id, P2, Row::Backrow, lane)
}

mod s10_3_r99_a_trap_that_declined_an_event_is_not_offered_it_again_because_a_later_trap_asked {
    use super::*;

    #[test]
    fn r99_the_lane_1_trap_that_declined_the_play_is_still_armed_after_the_lane_2_trap_s_question_is_answered_as_it_is_with_no_question_r113_10_3()
     {
        register_all();
        // Control: the lane-2 trap pings p1 at once. The lane-1 trap met the play first, with p1 at 30,
        // declined it, and is not offered it again after the ping.
        let mut control = scenario(json!({ "p1": { "hand": [TEMPO_TIMMY], "mana": 4 }, "p2": { "hand": [RENO] } }));
        let control_guard = conditional_trap(&mut control, "edge-r8-guard-control", 1);
        fixture_faces(
            control.state_mut(),
            "edge-r8-pinger-now",
            "Trap",
            same(Script {
                triggers: vec![
                    TriggerDef::new("edge-r8-pinger-now:pings", &[GameEventType::CardPlayed], |_ctx, _event| {
                        vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
                    })
                    .with_when(played_by_opponent),
                ],
                ..Script::default()
            }),
            json!({}),
        );
        place_fixture(&mut control, "edge-r8-pinger-now", P2, Row::Backrow, 2);
        control.play(TEMPO_TIMMY, json!({ "zone": 1 }));
        control.expect_health(P1, 29);
        control.expect_in_zone(&control_guard.id, "field");

        // The same board, but the lane-2 trap asks p2 something before it pings.
        let mut s = scenario(json!({ "p1": { "hand": [TEMPO_TIMMY], "mana": 4 }, "p2": { "hand": [RENO] } }));
        let guard = conditional_trap(&mut s, "edge-r8-guard", 1);
        fixture_faces(
            s.state_mut(),
            "edge-r8-pinger-asks",
            "Trap",
            same(Script {
                triggers: vec![
                    TriggerDef::new("edge-r8-pinger-asks:asks", &[GameEventType::CardPlayed], |_ctx, _event| {
                        vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" })))]
                    })
                    .with_when(played_by_opponent),
                ],
                resume: IndexMap::from([(
                    "ok",
                    hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]),
                )]),
                ..Script::default()
            }),
            json!({}),
        );
        place_fixture(&mut s, "edge-r8-pinger-asks", P2, Row::Backrow, 2);
        s.play(TEMPO_TIMMY, json!({ "zone": 1 }));
        assert_eq!(must(s.state().pending.clone(), "the lane-2 trap's question").player_id, P2);
        s.answer(json!("ok"));

        // §10.3: the traps check the event as it is dispatched, each once. The lane-1 trap was offered
        // the play before the lane-2 trap fired and declined it; the question only paused the dispatch
        // after that (R113 resumes where it stopped), so it does not get a second look at the same play.
        s.expect_health(P1, 29);
        s.expect_in_zone(&guard.id, "field");
        assert!(!s.card(&guard.id).face_up.unwrap_or(false));
    }
}

mod s10_3_r113_the_traps_still_owed_an_event_after_a_question_meet_it_in_the_order_the_dispatch_had {
    use super::*;

    /// A trap that answers every `cardPlayed` and does nothing else (R61: it fires and is consumed).
    fn silent_trap(s: &mut Scenario, id: &str, player: PlayerId, lane: i32) -> CardInstance {
        fixture_faces(
            s.state_mut(),
            id,
            "Trap",
            same(Script {
                triggers: vec![TriggerDef::new(format!("{id}:fires"), &[GameEventType::CardPlayed], |_ctx, _event| {
                    vec![]
                })],
                ..Script::default()
            }),
            json!({}),
        );
        place_fixture(s, id, player, Row::Backrow, lane)
    }

    /// p1's lane-1 trap: on any play, rotate the rings (p1's seat) — at once, or after a question.
    fn rotating_trap(s: &mut Scenario, asks: bool) {
        let turn = effects::rotate(json_as(json!({ "direction": "left" })));
        let at_once = turn.clone();
        let id = if asks { "edge-r8-rotor-asks" } else { "edge-r8-rotor-now" };
        fixture_faces(
            s.state_mut(),
            id,
            "Trap",
            same(Script {
                triggers: vec![TriggerDef::new(
                    if asks { "edge-r8-rotor-asks:fires" } else { "edge-r8-rotor-now:fires" },
                    &[GameEventType::CardPlayed],
                    move |_ctx, _event| {
                        if asks {
                            vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" })))]
                        } else {
                            vec![at_once.clone()]
                        }
                    },
                )],
                resume: IndexMap::from([("ok", hook(move |_ctx| vec![turn.clone()]))]),
                ..Script::default()
            }),
            json!({}),
        );
        place_fixture(s, id, P1, Row::Backrow, 1);
    }

    fn crossed_to_p1(s: &Scenario, id: &str) -> bool {
        s.events().iter().any(
            |event| matches!(event, GameEvent::ControlChanged { instance_id, controller, .. } if instance_id == id && *controller == P1),
        )
    }

    fn fired_order(s: &Scenario, ids: &[&str]) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::TrapFired { instance_id, .. } if ids.contains(&instance_id.as_str()) => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r113_p2_s_lane_4_and_lane_5_traps_fire_in_the_order_the_play_s_dispatch_offered_them_though_the_answer_rotated_the_lane_5_one_onto_p1_s_side_10_3_r68()
     {
        register_all();
        // Control: the rotation happens at once. The dispatch offers the play to p1's rotor, then to
        // p2's lane-4 and lane-5 traps in the order it read as it began (R68), whichever side the
        // rotation has put them on by then.
        let mut control = scenario(json!({ "p1": { "hand": [TEMPO_TIMMY], "mana": 4 }, "p2": { "hand": [RENO] } }));
        rotating_trap(&mut control, false);
        let a0 = silent_trap(&mut control, "edge-r8-silent-a0", P2, 4);
        let b0 = silent_trap(&mut control, "edge-r8-silent-b0", P2, 5);
        control.play(TEMPO_TIMMY, json!({ "zone": 2 }));
        assert!(crossed_to_p1(&control, &b0.id), "the rotation carried the lane-5 trap across to p1");
        assert_eq!(fired_order(&control, &[a0.id.as_str(), b0.id.as_str()]), vec![a0.id.clone(), b0.id.clone()]);

        // The same, with the rotor asking p1 first and rotating on the answer.
        let mut s = scenario(json!({ "p1": { "hand": [TEMPO_TIMMY], "mana": 4 }, "p2": { "hand": [RENO] } }));
        rotating_trap(&mut s, true);
        let a = silent_trap(&mut s, "edge-r8-silent-a", P2, 4);
        let b = silent_trap(&mut s, "edge-r8-silent-b", P2, 5);
        s.play(TEMPO_TIMMY, json!({ "zone": 2 }));
        must(s.state().pending.clone(), "the rotor's question");
        s.answer(json!("ok"));
        assert!(crossed_to_p1(&s, &b.id));

        // R113: the dispatch the question paused resumes where it stopped — the traps it still owed, in
        // the order it had (as the end-of-turn window's remainder keeps its owed list's order). Before
        // the fix the remainder was re-read from a fresh scan in which p1's side comes first, so the
        // lane-5 trap the rotation moved fired ahead of the lane-4 one.
        assert_eq!(fired_order(&s, &[a.id.as_str(), b.id.as_str()]), vec![a.id.clone(), b.id.clone()]);
    }
}

// ---------------------------------------------------------------------------------------------
// 3. A card named by id later in a list a prompt split is still aimed at its stay (R174, R113)
// ---------------------------------------------------------------------------------------------

mod r174_r113_an_effect_naming_a_card_by_id_after_a_prompt_meets_the_stay_the_run_began_with {
    use super::*;

    #[test]
    fn r174_the_rush_token_a_trap_s_list_summoned_does_not_attack_the_reborn_body_of_the_played_unit_the_answer_sacrificed_r53_8_60()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // p1's 0-cost 2/2 Reborn unit.
        fixture_faces(
            s.state_mut(),
            "edge-r8-played-reborn",
            "Unit",
            same(Script::default()),
            json!({ "keywords": [{ "kind": "Reborn" }] }),
        );
        let played = in_hand(&mut s, "edge-r8-played-reborn", P1);
        // p2's trap, #60 Bear Honeypot's list with a question in it: when the opponent's permanent
        // resolves, summon a Rush Token and ask p2; the answer sacrifices the played unit; then the
        // tokens this list summoned attack the played unit.
        fixture_faces(
            s.state_mut(),
            "edge-r8-honeypot-sacrifices",
            "Trap",
            same(Script {
                triggers: vec![
                    TriggerDef::new("edge-r8-honeypot-sacrifices:fires", &[GameEventType::CardResolved], |_ctx, event| {
                        let id = resolved_id(event);
                        vec![
                            effects::summon(json_as(json!({ "defId": RUSH_TOKEN }))),
                            effects::choose_mode(json_as(
                                json!({ "options": ["ok"], "step": "ok", "prompt": "a question", "data": { "played": id } }),
                            )),
                            effects::forced_attacks(json_as(json!({
                                "attackers": { "side": "self", "defId": RUSH_TOKEN, "summonedThisScript": true },
                                "target": { "instanceId": id },
                            }))),
                        ]
                    })
                    .with_when(permanent_resolved_by_opponent),
                ],
                resume: IndexMap::from([(
                    "ok",
                    hook(|ctx| {
                        vec![effects::sacrifice(json_as(json!({
                            "target": { "of": "instance", "instanceId": data_string(ctx, "played") },
                            "allowEnemy": true,
                        })))]
                    }),
                )]),
                ..Script::default()
            }),
            json!({}),
        );
        place_fixture(&mut s, "edge-r8-honeypot-sacrifices", P2, Row::Backrow, 1);

        s.play(&played.id, json!({ "zone": 1 }));
        must(s.state().pending.clone(), "the trap's question");
        let token = must(s.unit(P2, 1).cloned(), "the Rush Token the trap summoned");
        s.answer(json!("ok"));

        // The answer's sacrifice is a death in full: the played unit's Reborn body came back in lane 1
        // (R83) before the tail ran.
        let events: Vec<GameEvent> = s.last_events().to_vec();
        let died = find_index(
            &events,
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == played.id),
        );
        let reborn = find_index(
            &events,
            |event| matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == played.id),
        );
        assert!(died >= 0, "the answer sacrificed the played unit");
        assert!(reborn > died, "Reborn brought its body back");
        // R174: "they attack it" is aimed at the played unit's stay, which the answer ended before the
        // tail ran — the list is one run whether or not a prompt split it (R113), and the answered step
        // and the tail share its mark. #60's tokens do not attack a Reborn body; before the fix the
        // token attacked it and the 1-health body died a second time.
        let attacked = events.iter().any(|event| {
            matches!(event, GameEvent::AttackDeclared { forced, target_id, .. } if *forced && *target_id == played.id)
        });
        assert!(!attacked, "the token was made to attack the Reborn body");
        assert_eq!(s.card(&token.id).damage, 0);
        s.expect_in_zone(&played.id, "field");
        s.expect_stats(&played.id, json!({ "health": 1 }));
    }

    #[test]
    fn r174_damage_the_answered_step_aims_by_id_at_a_unit_its_own_list_sacrificed_before_the_prompt_misses_the_reborn_body_r83_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        fixture_faces(
            s.state_mut(),
            "edge-r8-enemy-reborn",
            "Unit",
            same(Script::default()),
            json!({ "stats": { "attack": 2, "health": 3 }, "keywords": [{ "kind": "Reborn" }] }),
        );
        let victim = place_fixture(&mut s, "edge-r8-enemy-reborn", P2, Row::Units, 1);
        // A 0-cost Spell: sacrifice the chosen enemy unit, ask something, then (the answered step) deal
        // 5 damage to that unit, named by the id the Cry carried into the prompt.
        fixture_faces(
            s.state_mut(),
            "edge-r8-sac-ask-hit-by-id",
            "Spell",
            same(Script {
                targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }))],
                cry: Some(hook(|ctx| {
                    let id = match ctx.targets.first() {
                        Some(Selection::Instance { instance_id }) => instance_id.clone(),
                        _ => String::new(),
                    };
                    vec![
                        effects::sacrifice(json_as(json!({ "target": { "of": "chosen" }, "allowEnemy": true }))),
                        effects::choose_mode(json_as(
                            json!({ "options": ["ok"], "step": "ok", "prompt": "a question", "data": { "victim": id } }),
                        )),
                    ]
                })),
                resume: IndexMap::from([(
                    "ok",
                    hook(|ctx| {
                        vec![effects::damage(json_as(
                            json!({ "to": { "of": "instance", "instanceId": data_string(ctx, "victim") }, "amount": 5 }),
                        ))]
                    }),
                )]),
                ..Script::default()
            }),
            json!({}),
        );
        let spell = in_hand(&mut s, "edge-r8-sac-ask-hit-by-id", P1);

        s.play(&spell.id, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
        must(s.state().pending.clone(), "the Spell's question");
        assert_eq!(s.card(&victim.id).reborn_spent, Some(true));
        s.answer(json!("ok"));

        // R174: the damage is aimed at the stay the play chose, which the sacrifice ended before the
        // question; the answered step continues the same resolution (R113), and naming the card by id
        // rather than as "the chosen one" does not make the Reborn body the card it was aimed at.
        s.expect_in_zone(&victim.id, "field");
        s.expect_stats(&victim.id, json!({ "health": 1 }));
    }
}

// ---------------------------------------------------------------------------------------------
// 4. A target picked at a prompt is aimed at the stay it was picked on (R174, §10.6)
// ---------------------------------------------------------------------------------------------

mod r174_10_6_a_card_picked_at_a_prompt_is_aimed_at_the_stay_the_prompt_offered {
    use super::*;

    /// p2's 2/3 Reborn unit in lane 1 and Tempo Timmy in lane 2; p1's Spell: sacrifice the chosen
    /// enemy unit, then ask for an enemy unit and deal it 5.
    fn board() -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "field": [{ "def": TEMPO_TIMMY, "lane": 2 }], "hand": [RENO] },
        }));
        fixture_faces(
            s.state_mut(),
            "edge-r8-enemy-reborn-2",
            "Unit",
            same(Script::default()),
            json!({ "stats": { "attack": 2, "health": 3 }, "keywords": [{ "kind": "Reborn" }] }),
        );
        let victim = place_fixture(&mut s, "edge-r8-enemy-reborn-2", P2, Row::Units, 1);
        fixture_faces(
            s.state_mut(),
            "edge-r8-sac-then-pick",
            "Spell",
            same(Script {
                targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } }))],
                cry: Some(hook(|_ctx| {
                    vec![
                        effects::sacrifice(json_as(json!({ "target": { "of": "chosen" }, "allowEnemy": true }))),
                        effects::choose_target(json_as(json!({
                            "step": "hit",
                            "scope": { "side": "enemy", "of": ["unit"] },
                            "prompt": "deal 5 to an enemy unit",
                        }))),
                    ]
                })),
                resume: IndexMap::from([(
                    "hit",
                    hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 })))]),
                )]),
                ..Script::default()
            }),
            json!({}),
        );
        let spell = in_hand(&mut s, "edge-r8-sac-then-pick", P1);
        (s, victim, spell)
    }

    #[test]
    fn r174_the_reborn_body_a_prompt_offered_after_the_list_s_own_sacrifice_takes_the_answered_step_s_damage_when_it_is_picked_r83_r113()
     {
        register_all();
        // Control: picking the unit that never left, the answered step's 5 lands and kills it.
        let (mut control, control_victim, control_spell) = board();
        control.play(
            &control_spell.id,
            json!({ "targets": [{ "pick": "instance", "instanceId": control_victim.id }] }),
        );
        let timmy = must(control.unit(P2, 2).cloned(), "p2's Tempo Timmy");
        control.answer(json!(timmy.id));
        control.expect_in_zone(&timmy.id, "graveyard");

        let (mut s, victim, spell) = board();
        s.play(&spell.id, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
        // The sacrifice is a death in full, and Reborn has put the body back before the prompt opened:
        // the prompt offers it, a unit on the field now (R83).
        assert_eq!(s.card(&victim.id).reborn_spent, Some(true));
        let pending = must(s.state().pending.clone(), "the Spell's target prompt");
        let offered = pending
            .options
            .iter()
            .any(|option| matches!(&option.selection, Selection::Instance { instance_id } if *instance_id == victim.id));
        assert!(offered, "the prompt offers the Reborn body");
        s.answer(json!(victim.id));

        // §10.6: the answer re-invokes the script with the selection, and the selection is a card on
        // the field as the prompt offered it. R174 aims an effect at the stay it was chosen on — here,
        // the body's — so the 5 damage lands and kills it. Before the fix the answered step judged the
        // pick against the mark the Cry began with, before the sacrifice, called the body "gone", and
        // the offered option did nothing.
        let hit = s
            .last_events()
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == victim.id));
        assert!(hit, "the picked Reborn body takes the answered step's damage");
        s.expect_in_zone(&victim.id, "graveyard");
    }
}

// ---------------------------------------------------------------------------------------------
// 5. A Tribute's Death that asks while the played card is between the hand and the field (§10.5)
// ---------------------------------------------------------------------------------------------

/// Every pile holding this id, in every zone of both sides (§10.1: a card is in exactly one).
fn piles_holding(state: &GameState, id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for player in [P1, P2] {
        let side = &state.players[player];
        let piles: Vec<(&str, Vec<&CardInstance>)> = vec![
            ("hand", side.hand.iter().collect()),
            ("library", side.library.iter().collect()),
            ("graveyard", side.graveyard.iter().collect()),
            ("exile", side.exile.iter().collect()),
            ("resolving", side.resolving.iter().collect()),
            ("backrow", side.backrow.iter().flatten().collect()),
            ("units", side.units.iter().flat_map(|pile| pile.iter().flatten()).collect()),
        ];
        for (name, cards) in piles {
            if cards.iter().any(|card| card.id == id) {
                out.push(format!("{player}.{name}"));
            }
        }
    }
    out
}

mod r226_10_1_a_card_being_played_is_never_left_in_two_zones_by_a_question_its_tribute_asks {
    use super::*;

    #[test]
    fn r226_a_tribute_whose_death_has_its_controller_discard_the_card_being_played_does_not_also_put_that_card_on_the_field_10_1_10_5_r113()
     {
        register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO] },
        }));
        // p1's 1/1 in lane 1 whose Death asks its controller to discard a card.
        fixture_faces(
            s.state_mut(),
            "edge-r8-discarding-death",
            "Unit",
            same(Script {
                death: Some(hook(|_ctx| {
                    vec![effects::choose_from_hand(json_as(json!({ "step": "gone", "prompt": "discard a card" })))]
                })),
                resume: IndexMap::from([(
                    "gone",
                    hook(|_ctx| vec![effects::discard(json_as(json!({ "target": { "of": "chosen" } })))]),
                )]),
                ..Script::default()
            }),
            json!({ "stats": { "attack": 1, "health": 1 } }),
        );
        let fodder = place_fixture(&mut s, "edge-r8-discarding-death", P1, Row::Units, 1);
        // p1's 0-cost 3/3 with Tribute 1.
        fixture_faces(
            s.state_mut(),
            "edge-r8-tribute-one",
            "Unit",
            same(Script { static_flags: flags(json!({ "tribute": 1 })), ..Script::default() }),
            json!({ "stats": { "attack": 3, "health": 3 } }),
        );
        let played = in_hand(&mut s, "edge-r8-tribute-one", P1);

        s.play(&played.id, json!({ "zone": 2, "tributes": [fodder.id] }));
        let pending = must(s.state().pending.clone(), "the tributed unit's Death question");
        let offers_played = pending
            .options
            .iter()
            .any(|option| matches!(&option.selection, Selection::Instance { instance_id } if *instance_id == played.id));
        if offers_played {
            s.answer(json!(played.id));
        } else {
            let key = must(pending.options.first(), "a hand card to discard").key.clone();
            s.answer(json!(key));
        }

        // §10.1: every card is in exactly one zone. Offered while it waits between step 2 and step 4 in
        // its owner's hand, the card being played is discarded to the graveyard by the answer; step 4
        // then puts it on the field too, so it stands in lane 2 and lies in the graveyard at once.
        // Either the question does not offer the card being played (it is leaving the hand, as R90 says
        // of the play's own choices), or a card discarded that way is not played.
        assert_eq!(
            piles_holding(s.state(), &played.id).len(),
            1,
            "the card being played is in exactly one pile"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 6. The step-4 loop a trigger's question interrupted still checks before the Cry (R59, R118)
// ---------------------------------------------------------------------------------------------

mod r59_10_5_step_4_a_trigger_that_answered_the_play_and_asked_is_followed_by_the_check_before_the_cry {
    use super::*;

    /// p1's 0-cost unit whose Cry deals the enemy hero 1 per enemy unit on the field.
    fn counter(s: &mut Scenario) -> CardInstance {
        fixture_faces(
            s.state_mut(),
            "edge-r8-counting-cry",
            "Unit",
            same(Script {
                cry: Some(hook(|ctx| {
                    let enemies = active_units_of(ctx.state, P2).len();
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": enemies })))]
                })),
                ..Script::default()
            }),
            json!({}),
        );
        in_hand(s, "edge-r8-counting-cry", P1)
    }

    /// p1's lane-1 unit: whenever its controller plays another card, deal 5 to `victim` — at once, or on the answer to a question.
    fn striker(s: &mut Scenario, victim_id: &str, asks: bool) {
        let id = if asks { "edge-r8-striker-asks" } else { "edge-r8-striker-now" };
        let hit = effects::damage(json_as(json!({ "to": { "of": "instance", "instanceId": victim_id }, "amount": 5 })));
        let on_answer = hit.clone();
        fixture_faces(
            s.state_mut(),
            id,
            "Unit",
            same(Script {
                triggers: vec![TriggerDef::new(format!("{id}:fires"), &[GameEventType::CardPlayed], move |ctx, event| {
                    let another_of_mine = match event {
                        GameEvent::CardPlayed { player, instance_id, .. } => {
                            *player == ctx.controller
                                && Some(instance_id.as_str()) != ctx.self_.as_ref().map(|card| card.id.as_str())
                        }
                        _ => false,
                    };
                    if !another_of_mine {
                        vec![]
                    } else if asks {
                        vec![effects::choose_mode(json_as(json!({ "options": ["ok"], "step": "ok", "prompt": "a question" })))]
                    } else {
                        vec![hit.clone()]
                    }
                })],
                resume: IndexMap::from([("ok", hook(move |_ctx| vec![on_answer.clone()]))]),
                ..Script::default()
            }),
            json!({}),
        );
        place_fixture(s, id, P1, Row::Units, 1);
    }

    #[test]
    fn r59_the_unit_the_trigger_s_answer_killed_has_died_before_the_played_card_s_cry_counts_the_board_as_it_has_when_nothing_asks_r118_r113()
     {
        register_all();
        // Control: the trigger deals its 5 at once. §10.5 step 4's loop runs it, then the check (R59),
        // and Tempo Timmy has died before the Cry counts the enemy's units: 0 damage.
        let mut control = scenario(json!({ "p1": { "hand": [RENO], "mana": 4 }, "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] } }));
        let timmy0 = must(control.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        striker(&mut control, &timmy0.id, false);
        let counting = counter(&mut control);
        control.play(&counting.id, json!({ "zone": 2 }));
        control.expect_in_zone(&timmy0.id, "graveyard");
        control.expect_health(P2, 30);

        // The same trigger, asking p1 first and dealing its 5 on the answer.
        let mut s = scenario(json!({ "p1": { "hand": [RENO], "mana": 4 }, "p2": { "field": [TEMPO_TIMMY], "hand": [RENO] } }));
        let timmy = must(s.unit(P2, 1).cloned(), "p2's Tempo Timmy");
        striker(&mut s, &timmy.id, true);
        let counting = counter(&mut s);
        s.play(&counting.id, json!({ "zone": 2 }));
        must(s.state().pending.clone(), "the trigger's question");
        s.answer(json!("ok"));

        // R59: the check follows the whole trigger, answered step included, and step 4's loop is where
        // the play left it (R113, R118) — so Timmy, at -2, dies before step 5. Before the fix the
        // re-entered loop held the check (`holdCheck`) because nothing had resolved inside it yet, and
        // the Cry counted a dead unit still on the field: p2 took 1.
        s.expect_in_zone(&timmy.id, "graveyard");
        s.expect_health(P2, 30);
    }
}

// ---------------------------------------------------------------------------
// Round 9: prompts a fused power opens, lists read off the board, and casts (R102, R66, R70)
// ---------------------------------------------------------------------------

const EXPERIMENTATION: &str = "core-085";

const GENNS_GREED: &str = "core-094";
const BIGOT: &str = "core-002"; // Unit, 2
const SEVEN_SEVEN: &str = "core-025"; // Unit, 4

/// A fixture card: a transient def in the match state and its script in the registry. `cost` is TS's
/// `cost = 0` (every call names it).
fn fixture_card(s: &mut Scenario, id: &str, ty: &str, script: Script, cost: i32) {
    let face = fixture_face(id, ty, &json!({ "attack": 2, "health": 2 }), &json!([]));
    s.state_mut().transient_defs.insert(id.to_string(), fixture_def(id, ty, cost, &face));
    register_fixture_scripts(id, CardScripts { base: script.clone(), radiant: script });
}

/// Put a fresh instance of `defId` into p1's library at `at` (0 is the top).
fn into_library(s: &mut Scenario, def_id: &str, at: usize) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, P1, Zone::Library { player: P1 });
    s.state_mut().players.p1.library.insert(at, card.clone());
    card
}

mod r102_r43_a_fused_heroic_power_s_prompted_power {
    use super::*;

    #[test]
    fn r102_a_heroic_power_fused_onto_a_heroic_power_discovers_once_its_first_ingredient_s_power_answering_the_prompt_r43_r752_10_6()
     {
        register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [HEROIC_POWER], "library": [RENO, RENO] },
            "p2": {
                "hand": [RENO],
                "library": [RENO, RENO],
                "backrow": [
                    { "def": EXPERIMENTATION, "lane": 1 },
                    { "def": HEROIC_POWER, "lane": 2 },
                ],
            },
        }));
        let played = must(
            g.state().players.p1.hand.iter().find(|card| card.def_id == HEROIC_POWER).cloned(),
            "p1's Heroic Power",
        );
        must(find_instance_mut(g.state_mut(), &played.id), "p1's Heroic Power")
            .memory
            .insert(subsystems::POWER_KEY.to_string(), json!("burn"));
        let kept = must(g.backrow(P2, 2).cloned(), "p2's Heroic Power");
        must(find_instance_mut(g.state_mut(), &kept.id), "p2's Heroic Power")
            .memory
            .insert(subsystems::POWER_KEY.to_string(), json!("discover"));

        // p1 plays its Heroic Power (which uses nothing, R752), and p2's #85 fuses it onto p2's own.
        g.play(&played.id, json!({ "zone": 1 }));
        let fused = g.card(&kept.id).clone();
        assert!(fused.def_id.starts_with("t-"));
        assert_eq!(
            subsystems::power_of(&fused).map(|power| power.name.to_string()).as_deref(),
            Some("discover")
        );
        // Each ingredient's text has the rolled power (R102): the alias names the first ingredient's.
        assert_eq!(
            subsystems::power_ability_of(g.state(), &fused).map(|decl| decl.id.clone()).as_deref(),
            Some("discover")
        );

        if g.state().active == P1 {
            g.end_turn();
        }
        assert_eq!(g.state().active, P2);
        assert_eq!(g.state().phase, Phase::Main);
        let hand_before = g.state().players.p2.hand.len();
        // One activation of the card's power: Witness Value Discovers (§10.6), and the answer finishes it.
        g.activate(&fused.id, json!({}));
        assert_eq!(pending_kind(&g), Some(PromptKind::Discover));
        let picked = must(
            g.state().pending.as_ref().and_then(|pending| pending.options.first()).cloned(),
            "an offered Unit",
        );
        g.answer(json!(picked.key));

        // Once: one Unit to hand, and the card's use is spent for both ingredients' copies of the power.
        assert_eq!(g.state().players.p2.hand.len(), hand_before + 1);
        assert!(subsystems::used_this_turn(g.state(), g.card(&kept.id)));
    }
}

mod r66_r113_a_card_s_list_resumed_after_a_prompt_keeps_the_effects_it_built {
    use super::*;

    #[test]
    fn r66_genn_s_greed_still_draws_every_2_cost_card_after_one_it_drew_was_cast_and_asked_r113_8_94_r135() {
        register_all();
        let mut g = scenario(json!({ "p1": { "hand": [GENNS_GREED], "library": [SEVEN_SEVEN, BIGOT, BIGOT, SEVEN_SEVEN] } }));
        // A 2-cost cast-on-draw Spell whose cast asks its caster something (R70, R81).
        fixture_card(
            &mut g,
            "edge-r9-cod-asks",
            "Spell",
            Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                cry: Some(hook(|_ctx| {
                    vec![effects::choose_mode(json_as(json!({ "options": ["a", "b"], "step": "picked", "prompt": "pick one" })))]
                })),
                resume: IndexMap::from([("picked", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            2,
        );
        // Library, top down: a 4-cost 7/7, the fixture (2), Bigot (2), Bigot (2), a 4-cost 7/7.
        let cod = into_library(&mut g, "edge-r9-cod-asks", 1);
        let others: Vec<CardInstance> = g
            .state()
            .players
            .p1
            .library
            .iter()
            .filter(|card| card.id != cod.id)
            .cloned()
            .collect();
        let (top, first, second) = (others.first(), others.get(1), others.get(2));
        let bigots: Vec<CardInstance> = g
            .state()
            .players
            .p1
            .library
            .iter()
            .filter(|card| card.def_id == BIGOT)
            .cloned()
            .collect();
        assert_eq!(bigots.len(), 2);
        assert_eq!(first.map(|card| card.def_id.as_str()), Some(BIGOT));
        assert_eq!(second.map(|card| card.def_id.as_str()), Some(BIGOT));
        assert_eq!(top.map(|card| card.def_id.as_str()), Some(SEVEN_SEVEN));

        // "Draw every 2-cost card from your library": the fixture, then both Bigots. The fixture is
        // drawn first and cast (§2.4), and its cast asks.
        g.play(GENNS_GREED, json!({}));
        assert_eq!(pending_kind(&g), Some(PromptKind::Mode));
        g.answer(json!("a"));
        assert!(g.state().pending.is_none());

        // Both Bigots were 2-cost cards in the library as the draw clause began, so both are drawn.
        for bigot in &bigots {
            g.expect_in_zone(&bigot.id, "hand");
        }
    }
}

mod r70_r17_r427_a_cast_unit_and_sheepish {
    use super::*;

    #[test]
    fn r70_r427_a_cast_on_draw_unit_the_opponent_s_sheepish_answers_is_a_sheep_once_its_cry_has_resolved_r17_10_5_step_7()
     {
        register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "backrow": [{ "def": SHEEPISH, "lane": 1 }], "hand": [RENO], "health": 30 },
        }));
        fixture_card(
            &mut g,
            "edge-r9-cod-unit",
            "Unit",
            Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))])),
                ..Script::default()
            },
            1,
        );
        let cod = into_library(&mut g, "edge-r9-cod-unit", 0);

        g.start_turn();
        assert!(g.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
        // Sheepish answered the cast Unit: a Sheep Token stands where it was.
        assert_eq!(g.unit(P1, 1).map(|card| card.def_id.clone()), Some("core-t-sheep".to_string()));
        assert!(g
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::Transformed { instance_id, .. } if *instance_id == cod.id)));
        // R427: a cast is a play (R70), and Sheepish answers its resolution, after the Cry: the 5 landed.
        g.expect_health(P2, 25);
        let types = types_of(g.events());
        assert!(type_index(&types, "transformed") > type_index(&types, "damage"));
    }
}

/// `selection.pick`: the name the selection's tag carries on the wire.
fn pick_name(selection: &Selection) -> &'static str {
    match selection {
        Selection::Instance { .. } => "instance",
        Selection::Hero { .. } => "hero",
        Selection::Zone { .. } => "zone",
        Selection::Mode { .. } => "mode",
        Selection::None => "none",
    }
}

mod r70_r90_a_cast_unit_s_own_choices {
    use super::*;

    #[test]
    fn r70_a_cast_on_draw_unit_asked_for_its_declared_target_is_not_offered_itself_as_a_play_of_it_never_is_r81_r90()
     {
        register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [RENO], "library": [RENO, RENO] },
            "p2": { "field": [SEVEN_SEVEN], "hand": [RENO] },
        }));
        let enemy = must(g.unit(P2, 1).cloned(), "p2's 7/7");
        fixture_card(
            &mut g,
            "edge-r9-cod-unit-targeted",
            "Unit",
            Script {
                static_flags: flags(json!({ "castOnDraw": true })),
                targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] } }))],
                cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 })))])),
                ..Script::default()
            },
            1,
        );
        let cod = into_library(&mut g, "edge-r9-cod-unit-targeted", 0);

        g.start_turn();
        let pending = must(g.state().pending.clone(), "the cast's target prompt");
        assert_eq!(pending.player_id, P1);
        let offered: Vec<String> = pending
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Instance { instance_id } => instance_id.clone(),
                other => pick_name(other).to_string(),
            })
            .collect();
        // A play of this card offers the units on the field as it is played, and never itself.
        assert!(offered.contains(&enemy.id));
        assert!(!offered.contains(&cod.id));
    }
}

mod r70_10_3_a_cast_s_step_4_window_inside_an_effect_offers_each_event_once {
    use super::*;

    #[test]
    fn r70_a_cast_s_step_4_window_inside_a_spell_s_draw_does_not_offer_the_spell_s_own_play_to_the_triggers_a_second_time_10_3()
     {
        register_all();
        // The cast runs inside #5 Stockpile's "draw 2", whose context is the only sink its step 4 has.
        // The play's `cardPlayed` was dispatched by the play's own step 4, before the draw: the cast's
        // window must not collect it again (§10.3: an event is offered once).
        let mut g = scenario(json!({
            "p1": { "hand": ["core-005", RENO], "field": [{ "def": SEVEN_SEVEN, "lane": 5 }], "library": [RENO, RENO, RENO] },
            "p2": { "hand": [RENO], "library": [RENO, RENO] },
        }));
        // p1's unit counts its controller's plays: 1 damage to the enemy hero for each `cardPlayed`.
        fixture_card(
            &mut g,
            "edge-r9-play-counter",
            "Unit",
            Script {
                triggers: vec![
                    TriggerDef::new("edge-r9-play-counter", &[GameEventType::CardPlayed], |_ctx, _event| {
                        vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
                    })
                    .with_when(|ctx, event| matches!(event, GameEvent::CardPlayed { player, .. } if *player == ctx.controller)),
                ],
                ..Script::default()
            },
            0,
        );
        place_fixture(&mut g, "edge-r9-play-counter", P1, Row::Units, 1);
        // A cast-on-draw Unit with no text on top of the library, so Stockpile's first draw casts it.
        fixture_card(
            &mut g,
            "edge-r9-cod-plain",
            "Unit",
            Script { static_flags: flags(json!({ "castOnDraw": true })), ..Script::default() },
            1,
        );
        let cod = into_library(&mut g, "edge-r9-cod-plain", 0);

        g.play("core-005", json!({}));

        // Two plays — Stockpile and the cast Unit (R70) — so two hits, not three.
        let played: Vec<String> = g
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert!(played.contains(&cod.id));
        g.expect_health(P2, 28);
    }
}

// ---------------------------------------------------------------------------------------------
// Round 10 (lens L7)
// ---------------------------------------------------------------------------------------------

mod r98_a_card_that_left_the_resolving_zone_before_its_question_is_answered {
    use super::*;

    #[test]
    fn r98_a_spell_its_own_list_returned_to_its_hand_before_it_asked_resumes_with_no_self_10_6() {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        fixture(
            &mut s,
            "edge-r10-homing-spell",
            "Spell",
            Script {
                // "Return this to your hand. Choose one: …" — then the answered step says "this costs 2 more".
                cry: Some(hook(|_ctx| {
                    vec![
                        effects::add_to_hand(json_as(json!({ "instance": { "of": "self" } }))),
                        effects::choose_mode(json_as(json!({ "options": ["a", "b"], "step": "picked", "prompt": "pick one" }))),
                    ]
                })),
                resume: IndexMap::from([(
                    "picked",
                    hook(|_ctx| vec![effects::set_cost_mod(json_as(json!({ "target": { "of": "self" }, "amount": 2 })))]),
                )]),
                ..Script::default()
            },
            None,
        );
        let spell = in_hand(&mut s, "edge-r10-homing-spell", P1);

        s.play(&spell.id, json!({}));
        assert_eq!(pending_kind(&s), Some(PromptKind::Mode));
        // The Spell has left the resolving zone: it is back in its owner's hand.
        s.expect_in_zone(&spell.id, "hand");
        s.answer(json!("a"));
        assert!(s.state().pending.is_none());

        // R98: "A card that has left the resolving zone before its own prompt is answered resumes with
        // no self" — so the answered step's "this" names nothing, and the card in hand is untouched.
        assert_eq!(s.card(&spell.id).cost_mod, 0);
    }
}

mod r59_a_sacrifice_whose_death_asks_inside_a_list {
    use super::*;

    /// A Spell: "deal 5 to the 1/5; sacrifice the other unit; restore 5 health to the 1/5".
    fn set_up(asking_death: bool) -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        fixture(
            &mut s,
            "edge-r10-dying",
            "Unit",
            if asking_death {
                Script {
                    death: Some(hook(|_ctx| {
                        vec![effects::choose_mode(json_as(
                            json!({ "options": ["a", "b"], "step": "picked", "prompt": "pick one" }),
                        ))]
                    })),
                    resume: IndexMap::from([("picked", hook(|_ctx| vec![]))]),
                    ..Script::default()
                }
            } else {
                Script::default()
            },
            None,
        );
        fixture(&mut s, "edge-r10-plain", "Unit", Script::default(), Some(json!({ "attack": 1, "health": 5 })));
        let dying = place_fixture(&mut s, "edge-r10-dying", P1, Row::Units, 1);
        let plain = place_fixture(&mut s, "edge-r10-plain", P1, Row::Units, 2);
        let (plain_id, dying_id) = (plain.id.clone(), dying.id.clone());
        fixture(
            &mut s,
            "edge-r10-sac-spell",
            "Spell",
            Script {
                cry: Some(hook(move |_ctx| {
                    vec![
                        effects::damage(json_as(json!({ "to": { "of": "instance", "instanceId": plain_id }, "amount": 5 }))),
                        effects::sacrifice(json_as(json!({ "target": { "of": "instance", "instanceId": dying_id } }))),
                        effects::heal(json_as(json!({ "target": { "of": "instance", "instanceId": plain_id }, "amount": 5 }))),
                    ]
                })),
                ..Script::default()
            },
            None,
        );
        let spell = in_hand(&mut s, "edge-r10-sac-spell", P1);
        (s, plain, spell)
    }

    #[test]
    fn r59_the_state_check_waits_for_the_whole_cry_after_a_sacrificed_unit_s_death_asked_so_the_heal_saves_the_unit_4_5_r113()
     {
        register_all();
        // Control: with a Death that asks nothing, the check runs after the whole Cry and the 1/5 lives.
        let (mut control, control_plain, control_spell) = set_up(false);
        control.play(&control_spell.id, json!({}));
        control.expect_in_zone(&control_plain.id, "field");

        let (mut s, plain, spell) = set_up(true);
        s.play(&spell.id, json!({}));
        assert_eq!(pending_kind(&s), Some(PromptKind::Mode));
        s.answer(json!("a"));
        assert!(s.state().pending.is_none());

        // §4.5 and R59: the check runs after "a card's whole Cry, spell, trap or triggered script",
        // never between the effects of one. The Death's question splits the list across two actions
        // (R113), which changes nothing: the heal lands before the check, as it does without a question.
        s.expect_in_zone(&plain.id, "field");
        assert_eq!(unit_view(s.state(), s.card(&plain.id)).health, 5);
    }
}

mod r151_r113_a_start_of_game_clause_that_asks_run_as_its_card_arrives_in_a_hand {
    use super::*;

    #[test]
    fn r151_the_rest_of_an_arriving_card_s_start_of_game_list_waits_for_the_answer_to_its_question_r113_9_3() {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        // "Start of game: choose one; then deal 3 damage to the enemy hero." R151 runs it when the card
        // arrives in a hand, as it does for #98's roll.
        fixture(
            &mut s,
            "edge-r10-asks-at-start",
            "Unit",
            Script {
                start_of_game: Some(hook(|_ctx| {
                    vec![
                        effects::choose_mode(json_as(json!({ "options": ["a", "b"], "step": "picked", "prompt": "pick one" }))),
                        effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 }))),
                    ]
                })),
                resume: IndexMap::from([("picked", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            None,
        );
        fixture(
            &mut s,
            "edge-r10-gift",
            "Spell",
            Script {
                cry: Some(hook(|_ctx| vec![effects::add_to_hand(json_as(json!({ "defId": "edge-r10-asks-at-start" })))])),
                ..Script::default()
            },
            None,
        );
        let spell = in_hand(&mut s, "edge-r10-gift", P1);
        let before = s.state().players.p2.hero.health;

        s.play(&spell.id, json!({}));
        assert_eq!(pending_kind(&s), Some(PromptKind::Mode));
        // §9.3, R113: a choice is state, and the effects after it wait for the answer.
        assert_eq!(s.state().players.p2.hero.health, before);

        s.answer(json!("a"));
        assert!(s.state().pending.is_none());
        assert_eq!(s.state().players.p2.hero.health, before - 3);
    }
}

mod r226_r70_a_cast_card_an_onplayhook_s_answer_moved_before_step_4 {
    use super::*;

    #[test]
    fn r226_a_cast_on_draw_card_the_step_3_hook_s_answer_exiled_stays_in_exile_and_is_not_placed_r70_10_5() {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        fixture(
            &mut s,
            "edge-r10-cod",
            "Unit",
            Script { static_flags: flags(json!({ "castOnDraw": true })), ..Script::default() },
            Some(json!({ "attack": 3, "health": 3 })),
        );
        let cod = on_top_of_library(&mut s, "edge-r10-cod", P1);
        let (asked_about, exiled) = (cod.id.clone(), cod.id.clone());
        // A permanent whose onPlayHook (§10.5 step 3, R153) asks about the cast card, and exiles it on "exile".
        fixture(
            &mut s,
            "edge-r10-hook",
            "Unit",
            Script {
                on_play_hook: Some(hook(move |ctx| {
                    if ctx.data.get("playedId").and_then(Value::as_str) == Some(asked_about.as_str()) {
                        vec![effects::choose_mode(json_as(json!({ "options": ["exile it", "keep it"], "step": "picked" })))]
                    } else {
                        vec![]
                    }
                })),
                resume: IndexMap::from([(
                    "picked",
                    hook(move |ctx| {
                        if effects::chosen_options(ctx).first().is_some_and(|option| option == "exile it") {
                            vec![effects::exile(json_as(json!({ "target": { "of": "instance", "instanceId": exiled } })))]
                        } else {
                            vec![]
                        }
                    }),
                )]),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r10-hook", P1, Row::Units, 1);
        fixture(
            &mut s,
            "edge-r10-draw-one",
            "Spell",
            Script { cry: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 1 })))])), ..Script::default() },
            None,
        );
        let spell = in_hand(&mut s, "edge-r10-draw-one", P1);

        s.play(&spell.id, json!({}));
        // The draw cast the Unit (§2.4, R70), and its step 3 is asking.
        assert_eq!(pending_kind(&s), Some(PromptKind::Mode));
        s.answer(json!("exile it"));
        assert!(s.state().pending.is_none());

        // R226's rule for a card taken away before §10.5 step 4 can place it: it stays where that move put
        // it and is not played. Exile is a pile nothing takes a card back out of (§6.3).
        s.expect_in_zone(&cod.id, "exile");
        assert!(!s
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == cod.id)));
    }
}

mod r174_a_card_picked_at_a_prompt_watched_by_a_delayed_effect_the_answered_step_schedules {
    use super::*;

    #[test]
    fn r174_the_answered_step_s_delayed_effect_watching_the_reborn_body_it_just_picked_is_scheduled_r76_10_6() {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        fixture(&mut s, "edge-r10-reborn", "Unit", Script::default(), Some(json!({ "attack": 1, "health": 3 })));
        let reborn = place_fixture(&mut s, "edge-r10-reborn", P1, Row::Units, 1);
        must(find_instance_mut(s.state_mut(), &reborn.id), "the Reborn unit")
            .granted_keywords
            .push(Keyword::Reborn);
        let reborn_id = reborn.id.clone();
        // "Sacrifice your Reborn unit. Choose a friendly unit: at the start of your next turn, give it +2/+2."
        fixture(
            &mut s,
            "edge-r10-watcher",
            "Spell",
            Script {
                cry: Some(hook(move |_ctx| {
                    vec![
                        effects::sacrifice(json_as(json!({ "target": { "of": "instance", "instanceId": reborn_id } }))),
                        effects::choose_target(json_as(json!({ "step": "picked", "scope": { "side": "ally", "of": ["unit"] } }))),
                    ]
                })),
                resume: IndexMap::from([
                    (
                        "picked",
                        hook(|ctx| {
                            let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                                return vec![];
                            };
                            vec![effects::delay(json_as(json!({
                                "at": { "phase": "start", "player": "self" },
                                "step": "later",
                                "hook": RESUME_HOOK,
                                "data": { "id": instance_id },
                                "watch": instance_id,
                            })))]
                        }),
                    ),
                    (
                        "later",
                        hook(|ctx| match ctx.data.get("id") {
                            Some(Value::String(id)) => vec![effects::buff(json_as(
                                json!({ "target": { "of": "instance", "instanceId": id }, "attack": 2, "health": 2 }),
                            ))],
                            _ => vec![],
                        }),
                    ),
                ]),
                ..Script::default()
            },
            None,
        );
        let spell = in_hand(&mut s, "edge-r10-watcher", P1);

        s.play(&spell.id, json!({}));
        // The sacrifice's Reborn put the body back before the list asked, so the prompt offers it.
        assert_eq!(pending_kind(&s), Some(PromptKind::Target));
        s.expect_in_zone(&reborn.id, "field");
        s.answer(json!([{ "pick": "instance", "instanceId": reborn.id }]));
        assert!(s.state().pending.is_none());

        // R174: "A card picked at a prompt, though, is picked on the stay the prompt offered it on …
        // and the answered step's effect lands on it" — a delayed effect aimed at it by id included.
        assert_eq!(
            s.state()
                .delayed
                .iter()
                .filter(|effect| effect.watch.as_deref() == Some(reborn.id.as_str()))
                .count(),
            1
        );
    }
}

mod r122_r113_the_engine_s_answer_called_directly_on_the_prompt_an_echo_repeat_opened {
    use super::*;

    #[test]
    fn r122_answerprompt_on_an_echo_repeat_s_fresh_pick_finishes_the_repeat_and_lands_the_spell_as_the_answer_action_does_r113_10_5_step_6()
     {
        register_all();
        let mut s = scenario(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        // "Deal 1 damage to a target. Echo 1": the repeat asks its target again (R81, §10.6).
        fixture(
            &mut s,
            "edge-r10-echo-ping",
            "Spell",
            Script {
                targets: vec![decl(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }))],
                static_flags: flags(json!({ "echo": 1 })),
                cry: Some(hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 })))])),
                ..Script::default()
            },
            None,
        );
        let spell = in_hand(&mut s, "edge-r10-echo-ping", P1);
        let before = s.state().players.p2.hero.health;

        s.play(&spell.id, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
        let pending = must(s.state().pending.clone(), "the repeat's fresh pick");
        assert_eq!(pending.kind, PromptKind::Target);
        assert_eq!(s.state().players.p2.hero.health, before - 1);

        // A caller driving the engine directly answers through the engine's own `answerPrompt` (R122:
        // "which matters for any caller driving the engine directly rather than through the reducer").
        let refused = with_sink(&mut s, |sink| {
            answer_prompt(
                sink,
                AnswerInput {
                    player_id: pending.player_id,
                    choice_id: pending.id.clone(),
                    selection: vec![Selection::Hero { player: P2 }],
                },
            )
        });
        // TS `expect(refused).toBeNull()`: no refusal.
        assert!(refused.is_ok(), "{refused:?}");

        // R122: the answer re-enters what the prompt interrupted and drains what is owed — the repeat
        // resolves and the Spell reaches its graveyard, rather than stopping short of it (R113: a
        // sequence is never dropped in silence).
        let state = s.state();
        assert!(state.pending.is_none());
        assert_eq!(state.players.p2.hero.health, before - 2);
        assert!(!state.players.p1.resolving.iter().any(|card| card.id == spell.id));
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == spell.id));
    }
}
