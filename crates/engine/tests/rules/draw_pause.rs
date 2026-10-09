//! A prompt opened by a cast-on-draw card, inside §2.4's two draw loops (SPEC §2.4, §9.3, §10.6;
//! R3, R4, R58, R70, R113, R117, R122).
//!
//! §2.4's "Cast on draw" fires a whole play from inside a draw, and a play can ask: #7 Jewelosco
//! Scarab's Discover, or anything Call to Chaos reaches. Neither draw loop may draw on past the prompt.
//!   * §9.3, R113, R117: a cast that asks stops the chain; the rest is owed to `state.work` and the
//!     answer draws exactly those cards, once each (likewise the whole draws a "draw N" has not made).
//!   * §9.3, §10.1: the paused game survives a JSON round trip and resumes identically.
//!   * R58: the owed chain carries its counter, so resuming cannot evade the cap.
//!   * R4, R3: hand-cap burn and fatigue still apply past a pause; the paused draw counts once.
//!   * R70, R122: the paused cast lands on the answer (§10.5 steps 6, 7), before the draw owed behind it.
//!
//! Controls: with nothing asking, the chain and "draw N" run in one call and `state.work` stays empty.
//! Fixtures: defs prefixed `dr-`, indexed from 2700, so they cannot collide with another file (BUILD §0).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::draw::{
    DRAW_CHAIN_WORK, DRAW_COUNT_WORK, DrawOutcome, draw, draw_one, owed_draw_chain_of, owed_draw_count_of,
};
use jackioh_engine::play_steps::PLAY_WORK_KIND;
use jackioh_engine::prompts::{OpenPromptArgs, open_prompt, resume_self};
use jackioh_engine::testkit::*;
use jackioh_engine::work::owed_work;

use crate::rules::fixtures::harness::{events_of_type, new_game, put, set_library, slot};

// Fixtures.

/// None of this file's cards is a Unit, so each face is `{ keywords: [], text }`.
fn def(name: &str, type_: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("dr-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (draw pause)"),
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
    def("log", "Field Spell", 2701)
}

/// §2.4: a cast-on-draw Spell whose Cry asks its controller something — #7's shape through a draw.
fn ask_on_draw() -> CardDef {
    def("ask-on-draw", "Spell", 2702)
}

/// The same, cast on draw, with nothing to ask: the rest of a chain, and the control.
fn quiet_on_draw() -> CardDef {
    def("quiet-on-draw", "Spell", 2703)
}

/// An ordinary card: drawn to hand, and the end of any chain that reaches it (R58).
fn plain() -> CardDef {
    def("plain", "Spell", 2704)
}

fn defs() -> Vec<CardDef> {
    vec![log_card(), ask_on_draw(), quiet_on_draw(), plain()]
}

// The note log: what ran, in the order it ran.
const NOTE_LANE: usize = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state
        .players
        .p1
        .backrow
        .get(NOTE_LANE - 1)
        .and_then(Option::as_ref)
}

fn write(state: &mut GameState, entry: &str) {
    let Some(log) = state
        .players
        .p1
        .backrow
        .get_mut(NOTE_LANE - 1)
        .and_then(Option::as_mut)
    else {
        return;
    };
    let mut steps: Vec<Value> = log
        .memory
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    steps.push(json!(entry));
    log.memory.insert("steps".to_string(), Value::Array(steps));
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

fn note(entry: &str) -> Effect {
    let entry = entry.to_string();
    Effect::new("dr:note", move |ctx| write(&mut *ctx.state, &entry))
}

/// §10.6: a prompt for the card's own controller, with one answer, so answering is trivial.
fn ask_controller() -> Effect {
    Effect::new("dr:ask", |ctx| {
        let resume = resume_self(ctx, "asked", Default::default());
        let player = ctx.controller;
        open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the cast-on-draw card asks its owner".to_string(),
                options: vec![PromptOption {
                    key: "none".to_string(),
                    label: "nothing".to_string(),
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

fn cast_on_draw() -> Option<StaticFlags> {
    Some(json_as(json!({ "castOnDraw": true })))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert("asked", hook(|_ctx| vec![note("answered")]));
    let mut scripts = IndexMap::new();
    scripts.insert(
        ask_on_draw().id,
        both(Script {
            static_flags: cast_on_draw(),
            cry: Some(hook(|_ctx| vec![note("ask"), ask_controller(), note("ask:tail")])),
            resume,
            ..Script::default()
        }),
    );
    scripts.insert(
        quiet_on_draw().id,
        both(Script {
            static_flags: cast_on_draw(),
            cry: Some(hook(|_ctx| vec![note("quiet")])),
            ..Script::default()
        }),
    );
    scripts.insert(plain().id, both(Script::default()));
    scripts
}

// Harness.
fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let n = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("dr{n}"));
    reduce(state, &json_as::<Action>(action))
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Past the mulligans, in p1's main phase, with the note log parked in p1's backrow lane 5.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    );
    put(
        &mut state,
        &log_card().id,
        slot(PlayerId::P1, Row::Backrow, NOTE_LANE as i32),
        Default::default(),
    );
    // The turn's own draw and the opening hands are behind us; every count below starts here.
    state.players.p1.hand = vec![];
    state.counters.drawn = 0;
    state
}

fn only<T: Clone>(items: &[T]) -> T {
    items.first().cloned().expect("expected exactly one item")
}

/// Answer the one open prompt, whoever it belongs to.
fn answer(state: &GameState) -> ReduceResult {
    let pending = state.pending.as_ref().expect("expected a prompt to be open");
    let result = act_result(
        state,
        json!({
            "type": "answer",
            "choiceId": pending.id,
            "selection": [{ "pick": "none" }],
            "playerId": pending.player_id,
        }),
    );
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state serialises")).expect("and parses")
}

/// The `drawn` events of one player. `state.counters.drawn` is both players' (R55), and an `answer`
/// that leaves p1 with no legal action auto-ends the turn, so p2's own start-of-turn draw lands in
/// the same action — which says nothing about the draw under test.
fn drawn_by(events: &[GameEvent], player: PlayerId) -> Vec<String> {
    events_of_type(events, GameEventType::Drawn)
        .iter()
        .filter_map(
            |event| match serde_json::to_value(event).expect("an event serialises") {
                drawn if drawn["player"] == json!(player) => drawn["defId"].as_str().map(str::to_string),
                _ => None,
            },
        )
        .collect()
}

fn hand_of(state: &GameState) -> Vec<String> {
    state.players.p1.hand.iter().map(|c| c.def_id.clone()).collect()
}

fn library_of(state: &GameState) -> Vec<String> {
    state
        .players
        .p1
        .library
        .iter()
        .map(|c| c.def_id.clone())
        .collect()
}

fn strings(ids: &[String]) -> Vec<String> {
    ids.to_vec()
}

/// A sink's events and rng (from the state's cursor), lent with the state to one engine call.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

fn hooks_of(state: &GameState) -> Vec<String> {
    state.work.iter().map(|item| item.resume.hook.clone()).collect()
}

fn burned_count(events: &[GameEvent]) -> usize {
    events_of_type(events, GameEventType::Burned).len()
}

mod r58_r113_r117_r122_a_prompt_inside_s2_4s_cast_on_draw_chain {
    use super::*;

    #[test]
    fn r158_draws_nothing_more_while_a_cast_on_draw_prompt_is_open_and_owes_the_rest_of_the_chain() {
        let mut state = playing("draw-chain-owe");
        set_library(
            &mut state,
            PlayerId::P1,
            &[
                quiet_on_draw().id,
                ask_on_draw().id,
                quiet_on_draw().id,
                plain().id,
            ],
        );

        let mut bench = Bench::new(&state);
        draw_one(&mut bench.sink(&mut state), PlayerId::P1, None);
        let events = bench.events;

        // The chain cast the first card, drew the asker, cast it — and stopped at its question.
        assert_eq!(notes(&state), vec!["quiet".to_string(), "ask".to_string()]);
        assert_eq!(
            state.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );

        // Exactly two cards have left the library, the two already cast; the cards behind are undrawn.
        assert_eq!(state.counters.drawn, 2);
        assert_eq!(
            drawn_by(&events, PlayerId::P1),
            vec![quiet_on_draw().id, ask_on_draw().id]
        );
        assert_eq!(library_of(&state), vec![quiet_on_draw().id, plain().id]);
        assert_eq!(hand_of(&state), Vec::<String>::new());

        // R113 and R117: the remainder is owed — one more draw, continuing the chain at the count it
        // had. §9.3: plain data, so no closure and no captured library is held across the prompt.
        let parked = only(&owed_work(&state, Some(DRAW_CHAIN_WORK)));
        // R217: the chain is the draw's own, so the item that finishes it closes it (`owns`).
        let owed = owed_draw_chain_of(&parked.resume).expect("an owed draw chain");
        assert_eq!(owed.player, PlayerId::P1);
        assert_eq!(owed.chain, 2);
        assert!(owed.owns);
        let reparsed: WorkItem =
            serde_json::from_value(serde_json::to_value(&parked).expect("a work item serialises"))
                .expect("and parses");
        assert_eq!(reparsed, parked);
        // R113's order, innermost first: the effects of the Cry after the one that asked, then the
        // cast's own tail (§10.5 steps 6 and 7), and only last the draw the chain still owes.
        assert_eq!(
            hooks_of(&state),
            vec![
                "cry".to_string(),
                PLAY_WORK_KIND.to_string(),
                DRAW_CHAIN_WORK.to_string()
            ]
        );

        // §10.1: the paused game survives a round trip and resumes from the round-tripped copy.
        let round = round_trip(&state);
        let owed = owed_draw_chain_of(&only(&owed_work(&round, Some(DRAW_CHAIN_WORK))).resume)
            .expect("an owed draw chain");
        assert_eq!(owed.player, PlayerId::P1);
        assert_eq!(owed.chain, 2);
        // R217: the draw that began this chain finishes it, so the item closes it.
        assert!(owed.owns);

        let done = answer(&round);
        // R113: the rest of the Cry, then the cast's tail, and only then the draw that was owed.
        assert_eq!(
            notes(&done.state),
            ["quiet", "ask", "answered", "ask:tail", "quiet"]
                .map(str::to_string)
                .to_vec()
        );
        // The two cards still in the library were drawn, once each: the third was cast on draw, the
        // fourth ended the chain in hand. Nothing was drawn twice and nothing was skipped.
        assert_eq!(done.state.counters.drawn, 4);
        assert_eq!(
            drawn_by(&done.events, PlayerId::P1),
            vec![quiet_on_draw().id, plain().id]
        );
        assert_eq!(library_of(&done.state), Vec::<String>::new());
        assert_eq!(hand_of(&done.state), vec![plain().id]);
        assert_eq!(done.state.pending, None);
        assert_eq!(done.state.work, Vec::<WorkItem>::new());
        // R70: each of the three cast cards counted as a play exactly once, across the pause.
        assert_eq!(done.state.counters.played, 3);
    }

    #[test]
    fn r58_resumes_a_chain_at_the_count_it_had_so_the_cap_cannot_be_evaded_by_pausing() {
        let mut state = playing("draw-chain-cap");
        // Two casts short of the cap: the asker, then one more cast, then one the cap must refuse.
        set_library(
            &mut state,
            PlayerId::P1,
            &[
                ask_on_draw().id,
                quiet_on_draw().id,
                quiet_on_draw().id,
                plain().id,
            ],
        );

        draw_one(
            &mut Bench::new(&state).sink(&mut state),
            PlayerId::P1,
            Some((CAST_ON_DRAW_CHAIN_CAP - 2).into()),
        );

        assert!(state.pending.is_some());
        // The owed count is the one the chain would have passed on, not zero.
        assert_eq!(
            owed_draw_chain_of(&only(&owed_work(&state, Some(DRAW_CHAIN_WORK))).resume)
                .map(|owed| owed.chain),
            Some(CAST_ON_DRAW_CHAIN_CAP - 1)
        );

        let done = answer(&round_trip(&state)).state;
        // One more cast fits under the cap; the next cast-on-draw card is at the cap, so it goes to the
        // hand uncast and ends the chain (R58).
        assert_eq!(
            notes(&done),
            ["ask", "answered", "ask:tail", "quiet"]
                .map(str::to_string)
                .to_vec()
        );
        assert_eq!(hand_of(&done), vec![quiet_on_draw().id]);
        assert_eq!(library_of(&done), vec![plain().id]);
        assert_eq!(done.work, Vec::<WorkItem>::new());
    }

    #[test]
    fn r4_burns_the_card_a_resumed_chain_draws_into_a_full_hand_counting_that_draw_once() {
        let mut state = playing("draw-chain-hand-cap");
        for _ in 0..HAND_CAP {
            let card = new_instance(
                &mut state,
                &plain().id,
                PlayerId::P1,
                Zone::Hand { player: PlayerId::P1 },
            );
            state.players.p1.hand.push(card);
        }
        set_library(&mut state, PlayerId::P1, &[ask_on_draw().id, plain().id]);

        draw_one(&mut Bench::new(&state).sink(&mut state), PlayerId::P1, None);
        // R58: a cast-on-draw card is cast even with a full hand, and nothing has burned yet.
        assert_eq!(state.counters.drawn, 1);
        assert_eq!(
            state
                .players
                .p1
                .graveyard
                .iter()
                .map(|c| c.def_id.clone())
                .collect::<Vec<_>>(),
            Vec::<String>::new()
        );

        let done = answer(&round_trip(&state));
        // The owed draw happened once: counted once, burned once, and the hand is still at the cap.
        assert_eq!(done.state.counters.drawn, 2);
        assert_eq!(burned_count(&done.events), 1);
        assert_eq!(done.state.players.p1.hand.len(), HAND_CAP as usize);
        // The asker resolved to the graveyard (§10.5 step 7) and the burned card joined it.
        assert_eq!(
            done.state
                .players
                .p1
                .graveyard
                .iter()
                .map(|c| c.def_id.clone())
                .collect::<Vec<_>>(),
            vec![ask_on_draw().id, plain().id]
        );
        assert_eq!(done.state.work, Vec::<WorkItem>::new());
    }

    #[test]
    fn r3_still_takes_fatigue_when_the_owed_draw_finds_the_library_empty() {
        let mut state = playing("draw-chain-fatigue");
        set_library(&mut state, PlayerId::P1, &[ask_on_draw().id]);

        draw_one(&mut Bench::new(&state).sink(&mut state), PlayerId::P1, None);
        // The pause happens before the empty-library draw, so no fatigue has been taken yet.
        assert_eq!(state.players.p1.fatigue_count, 0);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);

        let done = answer(&round_trip(&state));
        // The owed draw found nothing and dealt R3's first point of fatigue, exactly once.
        assert_eq!(done.state.players.p1.fatigue_count, 1);
        assert_eq!(done.state.players.p1.hero.health, HERO_HEALTH - 1);
        let amounts: Vec<Value> = events_of_type(&done.events, GameEventType::Damage)
            .iter()
            .map(|event| serde_json::to_value(event).expect("an event serialises")["amount"].clone())
            .collect();
        assert_eq!(amounts, vec![json!(1)]);
        // No card was drawn by the owed draw — there was none to draw — so nothing reached p1's hand.
        assert_eq!(drawn_by(&done.events, PlayerId::P1), Vec::<String>::new());
        assert_eq!(hand_of(&done.state), Vec::<String>::new());
        assert_eq!(done.state.work, Vec::<WorkItem>::new());
    }

    #[test]
    fn runs_a_whole_chain_in_one_call_when_nothing_asks_owing_nothing_at_all() {
        let mut state = playing("draw-chain-control");
        set_library(
            &mut state,
            PlayerId::P1,
            &[quiet_on_draw().id, quiet_on_draw().id, plain().id],
        );

        let mut bench = Bench::new(&state);
        draw_one(&mut bench.sink(&mut state), PlayerId::P1, None);
        let events = bench.events;

        // Both casts and the card that ended the chain, inside the one call.
        assert_eq!(notes(&state), vec!["quiet".to_string(), "quiet".to_string()]);
        assert_eq!(state.counters.drawn, 3);
        assert_eq!(hand_of(&state), vec![plain().id]);
        assert_eq!(library_of(&state), Vec::<String>::new());
        assert_eq!(events_of_type(&events, GameEventType::CardPlayed).len(), 2);
        assert_eq!(state.pending, None);
        // Nothing was parked: the machinery only engages at a pause (R117).
        assert_eq!(state.work, Vec::<WorkItem>::new());
    }
}

mod r58_r113_r117_r122_a_prompt_inside_s2_4s_draw_n_loop {
    use super::*;

    #[test]
    fn r113_stops_a_draw_n_at_the_draw_that_asked_and_owes_the_draws_it_has_not_made() {
        let mut state = playing("draw-count-owe");
        set_library(
            &mut state,
            PlayerId::P1,
            &[ask_on_draw().id, plain().id, plain().id],
        );

        let mut bench = Bench::new(&state);
        draw(&mut bench.sink(&mut state), PlayerId::P1, 3);
        let events = bench.events;

        // Only the first draw happened: the two behind it are the answering action's (§2.4, R122).
        assert_eq!(state.counters.drawn, 1);
        assert_eq!(drawn_by(&events, PlayerId::P1), vec![ask_on_draw().id]);
        assert_eq!(library_of(&state), vec![plain().id, plain().id]);
        assert_eq!(hand_of(&state), Vec::<String>::new());

        // R113: the interrupted chain is owed ahead of the whole draws that are still to come, and the
        // cast's own tail ahead of both. §9.3: all plain data.
        assert_eq!(
            hooks_of(&state),
            vec![
                "cry".to_string(),
                PLAY_WORK_KIND.to_string(),
                DRAW_CHAIN_WORK.to_string(),
                DRAW_COUNT_WORK.to_string(),
            ]
        );
        let parked = only(&owed_work(&state, Some(DRAW_COUNT_WORK)));
        let owed = owed_draw_count_of(&parked.resume).expect("an owed draw count");
        assert_eq!(owed.player, PlayerId::P1);
        assert_eq!(owed.count, 2);
        let reparsed: WorkItem =
            serde_json::from_value(serde_json::to_value(&parked).expect("a work item serialises"))
                .expect("and parses");
        assert_eq!(reparsed, parked);

        let done = answer(&round_trip(&state));
        // The chain's own owed draw first, then the two whole draws: three draws in all, once each.
        assert_eq!(done.state.counters.drawn, 3);
        assert_eq!(drawn_by(&done.events, PlayerId::P1), vec![plain().id, plain().id]);
        assert_eq!(hand_of(&done.state), vec![plain().id, plain().id]);
        assert_eq!(library_of(&done.state), Vec::<String>::new());
        assert_eq!(done.state.pending, None);
        assert_eq!(done.state.work, Vec::<WorkItem>::new());
    }

    #[test]
    fn draws_the_whole_of_a_draw_n_in_one_call_when_nothing_asks_owing_nothing_at_all() {
        let mut state = playing("draw-count-control");
        set_library(
            &mut state,
            PlayerId::P1,
            &[quiet_on_draw().id, plain().id, plain().id, plain().id],
        );

        let mut bench = Bench::new(&state);
        let outcomes = draw(&mut bench.sink(&mut state), PlayerId::P1, 3);

        // Draw 1 cast the top card and chained into the next; draws 2 and 3 took a card each.
        assert_eq!(
            outcomes,
            vec![DrawOutcome::Cast, DrawOutcome::Drawn, DrawOutcome::Drawn]
        );
        assert_eq!(state.counters.drawn, 4);
        assert_eq!(hand_of(&state), strings(&[plain().id, plain().id, plain().id]));
        assert_eq!(library_of(&state), Vec::<String>::new());
        assert_eq!(state.pending, None);
        assert_eq!(state.work, Vec::<WorkItem>::new());
    }
}
