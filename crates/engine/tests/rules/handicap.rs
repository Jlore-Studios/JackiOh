//! Port of `packages/engine/test/handicap.test.ts`.
//!
//! The per-seat handicap (SPEC §9.9; R180–R184, R290; docs/polish/3-ai.md B1–B8).
//!
//! Practice gives the AI seat more resources than a human: a bigger deck, extra mana crystals up to
//! a higher cap, an extra opening card and, on Hard, a second draw each turn. The tutorial's
//! opponent (AI_TUTORIAL, R290) gets fewer: a 12-card deck, 3 crystals at most and a hero that
//! starts at 20. The human seat always plays with this spec's own numbers, and a game with no
//! handicap must hash and replay exactly as it did before the field existed. Everything here is observed through `createGame`, `beginGame`,
//! `reduce`, `fold` and the state they return; nothing reads how the rules are implemented.
//!
//! Fixtures: the engine's vanilla catalog (`fx-1`..`fx-40`, every one a 1-cost 2/2), the Hinder and
//! Going Long fixtures from ./fixtures/scripts, and two cast-on-draw Spells of this file's own
//! (prefixed `hc-`, indexed from 2800) for R183's draw chain.
//!
//! (TS threw from `createGame`, `fold` and `validateHandicap`. Here `create_game` and `fold` panic with
//! TS's message, caught by `refusal`; `validate_handicap` answers `Result` with TS's message. TS also
//! handed `validateHandicap` objects its type does not allow — a missing field, a string, `null`, a
//! fraction, `NaN`, `Infinity`. Rust's `Handicap` holds `i32`s, so such a value is built as JSON and
//! counts as refused when it does not even deserialise into a `Handicap` (`handicap_refused`); see
//! `.fullsend/notes/spec-gaps-part-26-3.md` for the three that JSON cannot carry.)

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{token_def, vanilla_catalog, vanilla_deck};
use crate::rules::fixtures::harness::setup_catalog;
use crate::rules::fixtures::scripts::{going_long, heroic_power, hinder};

/// `Partial<Record<PlayerId, Handicap>>`.
type Handicaps = PerPlayerOpt<Handicap>;

fn on_p1(handicap: Handicap) -> Handicaps {
    PerPlayerOpt {
        p1: Some(handicap),
        p2: None,
    }
}

fn on_p2(handicap: Handicap) -> Handicaps {
    PerPlayerOpt {
        p1: None,
        p2: Some(handicap),
    }
}

fn on_both(p1: Handicap, p2: Handicap) -> Handicaps {
    PerPlayerOpt {
        p1: Some(p1),
        p2: Some(p2),
    }
}

// ---------------------------------------------------------------------------
// Fixtures: two cast-on-draw Spells for R183 (the shape draw-pause.test.ts uses).
// ---------------------------------------------------------------------------

fn spell(id: &str, index: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index,
        "name": format!("{id} (handicap)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

/// §2.4 cast on draw, and its Cry asks its controller a one-option question (§10.6).
fn ask_on_draw() -> CardDef {
    spell("hc-ask-on-draw", "2801")
}

/// §2.4 cast on draw, asking nothing: the chain simply continues (R58).
fn quiet_on_draw() -> CardDef {
    spell("hc-quiet-on-draw", "2802")
}

fn ask_controller() -> Effect {
    Effect::new("hc:ask", |ctx| {
        let resume = prompts::resume_self(ctx, "asked", IndexMap::new());
        let player = ctx.controller;
        let _ = prompts::open_prompt(
            ctx,
            prompts::OpenPromptArgs {
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

fn local_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert("asked", hook(|_ctx| vec![]));
    scripts.insert(
        ask_on_draw().id,
        both(Script {
            static_flags: Some(json_as(json!({ "castOnDraw": true }))),
            cry: Some(hook(|_ctx| vec![ask_controller()])),
            resume,
            ..Script::default()
        }),
    );
    scripts.insert(
        quiet_on_draw().id,
        both(Script {
            static_flags: Some(json_as(json!({ "castOnDraw": true }))),
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    scripts
}

/// The engine's fixture catalog plus this file's two Spells.
fn register_all() {
    setup_catalog();
    let mut catalog = catalog::registered_catalog().clone();
    catalog.insert(ask_on_draw().id, ask_on_draw());
    catalog.insert(quiet_on_draw().id, quiet_on_draw());
    register_catalog(catalog);
    let mut scripts = scripts::registered_scripts();
    scripts.extend(local_scripts());
    register_scripts(scripts);
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

fn size_for(handicaps: Option<&Handicaps>, player: PlayerId) -> i32 {
    handicaps
        .and_then(|seats| seats.get(player))
        .map_or(DECK_SIZE, |handicap| handicap.deck_size)
}

/// A legal deck pair for these handicaps: vanilla units, `fx-1` up, one per card id.
fn decks_for(handicaps: Option<&Handicaps>) -> (Vec<String>, Vec<String>) {
    (
        vanilla_deck(size_for(handicaps, PlayerId::P1), 1),
        vanilla_deck(size_for(handicaps, PlayerId::P2), 1),
    )
}

fn game(seed: &str, handicaps: Option<Handicaps>, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    register_all();
    let decks = decks.unwrap_or_else(|| decks_for(handicaps.as_ref()));
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks,
        handicaps,
        ..Default::default()
    })
}

/// TS's module-level `let nonce`; an atomic so that tests running side by side never share a nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

struct Stepped {
    state: GameState,
    events: Vec<GameEvent>,
}

fn input(player: PlayerId, body: ActionBody) -> ActionInput {
    ActionInput {
        body,
        player_id: player,
    }
}

fn step(state: &GameState, body: ActionInput) -> Stepped {
    let n = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let action = body.with_nonce(format!("hc{n}"));
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{} refused: {error}", action.action_type());
    }
    Stepped {
        state: result.state,
        events: result.events,
    }
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    step(state, body).state
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

/// Both mulligans kept whole: turn 1, p1's main phase.
fn started(seed: &str, handicaps: Option<Handicaps>, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    let mut state = begin_game(&game(seed, handicaps, decks)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));
    let keep = ids(&state.players.p2.hand);
    state = act(&state, input(PlayerId::P2, ActionBody::Mulligan { keep }));
    state
}

/// One endTurn, checked to have passed exactly one turn (no auto-ended cascade).
fn pass_turn(state: &GameState) -> Stepped {
    let turn = state.turn;
    let next = step(state, input(state.active, ActionBody::EndTurn));
    assert_eq!(next.state.turn, turn + 1, "endTurn passed exactly one turn");
    next
}

/// endTurn until `seat` is active having started `turns` turns.
fn advance_to(state: GameState, seat: PlayerId, turns: i32) -> GameState {
    let mut next = state;
    for _ in 0..60 {
        if next.active == seat && next.players[seat].turns_started == turns {
            return next;
        }
        next = pass_turn(&next).state;
    }
    panic!("{seat} never reached turn {turns}");
}

/// Each seat's mana.max at the start of each of its turns, over `turns` player-turns.
fn maxes_by_turn(seed: &str, handicaps: Option<Handicaps>, turns: usize) -> PerPlayer<Vec<i32>> {
    let mut state = started(seed, handicaps, None);
    let mut seen: PerPlayer<Vec<i32>> = PerPlayer::new(vec![state.players.p1.mana.max], vec![]);
    while seen.p1.len() + seen.p2.len() < turns {
        state = pass_turn(&state).state;
        let active = state.active;
        seen[active].push(state.players[active].mana.max);
    }
    seen
}

fn on_top_of_library(state: &mut GameState, player: PlayerId, def_ids: &[String]) -> Vec<CardInstance> {
    let cards: Vec<CardInstance> = def_ids
        .iter()
        .map(|def_id| state::new_instance(state, def_id, player, Zone::Library { player }))
        .collect();
    let mut library = cards.clone();
    library.append(&mut state.players[player].library);
    state.players[player].library = library;
    cards
}

fn drawn_by(events: &[GameEvent], player: PlayerId) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Drawn {
                player: drawer,
                instance_id,
                ..
            } if *drawer == player => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// The amounts of every `damage` event on a hero, by its target id (`hero-p2`).
fn hero_hits(events: &[GameEvent], target: &str) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } if target_id == target => Some(*amount),
            _ => None,
        })
        .collect()
}

fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state serialises")).expect("a state parses")
}

struct Live {
    state: GameState,
    log: Vec<Action>,
    decks: (Vec<String>, Vec<String>),
}

/// A full random-policy game (§10.7) under these handicaps, as fold must reproduce it. `observe`, when
/// given, sees the dealt state and every state after it.
fn play_random(seed: &str, handicaps: Handicaps, mut observe: Option<&mut dyn FnMut(&GameState)>) -> Live {
    let decks = decks_for(Some(&handicaps));
    let mut state = begin_game(&game(seed, Some(handicaps), Some(decks.clone()))).state;
    if let Some(observe) = observe.as_deref_mut() {
        observe(&state);
    }
    let mut policy = Rng::new(&format!("handicap-policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    while state.result.is_none() {
        assert!(log.len() <= 5000, "{seed} did not finish");
        let player = seat_to_act(&state).expect("a seat to act");
        let Some(chosen) = subsystems::ai_policy::choose_action(&state, player, &mut policy) else {
            panic!("{seed}: no legal action for {player}");
        };
        let action = Action::new(chosen, player, format!("hr{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{seed}: {} refused: {error}", action.action_type());
        }
        log.push(action);
        state = result.state;
        if let Some(observe) = observe.as_deref_mut() {
            observe(&state);
        }
    }
    Live { state, log, decks }
}

fn fold_with(seed: &str, live: &Live, handicaps: Option<Handicaps>) -> FoldResult {
    fold(&FoldArgs {
        seed: seed.to_string(),
        decks: live.decks.clone(),
        log: live.log.clone(),
        handicaps,
        ..Default::default()
    })
}

/// What a panic said, or `None` when `run` returned: TS's `expect(() => …).toThrow(…)`.
fn refusal(run: impl FnOnce()) -> Option<String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(()) => None,
        Err(payload) => Some(
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_string()))
                .unwrap_or_default(),
        ),
    }
}

/// Why `createGame` refused these options, or `None` when it made a game.
fn create_refusal(options: CreateGameOptions) -> Option<String> {
    refusal(move || {
        create_game(&options);
    })
}

/// `createGame` on options written as JSON: refused when the typed options cannot hold them (a
/// fractional number), else as `create_refusal`.
fn create_refusal_json(options: Value) -> Option<String> {
    match serde_json::from_value::<CreateGameOptions>(options) {
        Err(error) => Some(error.to_string()),
        Ok(options) => create_refusal(options),
    }
}

fn options(seed: &str, decks: (Vec<String>, Vec<String>), handicaps: Handicaps) -> CreateGameOptions {
    CreateGameOptions {
        seed: seed.to_string(),
        decks,
        handicaps: Some(handicaps),
        ..Default::default()
    }
}

/// A handicap as JSON with one field replaced.
fn with_field(handicap: &Handicap, field: &str, value: Value) -> Value {
    let mut json = serde_json::to_value(handicap).expect("a handicap serialises");
    json[field] = value;
    json
}

/// `expect(() => validateHandicap(h, label)).toThrow()` for a value TS's type does not hold: refused
/// where the typed `Handicap` is read from it, or else by `validate_handicap`.
fn handicap_refused(value: Value, label: &str) -> bool {
    match serde_json::from_value::<Handicap>(value) {
        Err(_) => true,
        Ok(handicap) => state::validate_handicap(&handicap, label).is_err(),
    }
}

/// Every piece of `pieces` appears in `text`, in order (TS's `/a.*b/`).
fn in_order(text: &str, pieces: &[&str]) -> bool {
    let mut rest = text;
    for piece in pieces {
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    true
}

fn keys_sorted(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

// ---------------------------------------------------------------------------
// R180: the handicap table and what "no handicap" means.
// ---------------------------------------------------------------------------

mod r180_handicaps_the_table_the_default_and_replay {
    use super::*;

    #[test]
    fn r180_b8_ai_difficulty_is_s9_9s_table_easy_is_human_handicap_and_human_handicap_is_this_specs_numbers()
    {
        assert_eq!(
            HUMAN_HANDICAP,
            Handicap {
                deck_size: DECK_SIZE,
                mana_bonus: 0,
                mana_cap: MAX_MANA,
                extra_opening_cards: 0,
                extra_draws_per_turn: 0,
                hero_health: None,
            }
        );
        assert_eq!(AI_DIFFICULTY.easy, HUMAN_HANDICAP);
        assert_eq!(
            AI_DIFFICULTY.medium,
            Handicap {
                deck_size: 25,
                mana_bonus: 1,
                mana_cap: 5,
                extra_opening_cards: 1,
                extra_draws_per_turn: 0,
                hero_health: None,
            }
        );
        assert_eq!(
            AI_DIFFICULTY.hard,
            Handicap {
                deck_size: 30,
                mana_bonus: 1,
                mana_cap: 7,
                extra_opening_cards: 1,
                extra_draws_per_turn: 1,
                hero_health: None,
            }
        );
        assert_eq!(
            DIFFICULTIES.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            ["easy", "medium", "hard"]
        );
        assert_eq!(
            keys_sorted(&serde_json::to_value(AI_DIFFICULTY).expect("the table serialises")),
            ["easy", "hard", "medium"]
        );
        assert_eq!(DRAWS_PER_TURN, 1);
    }

    #[test]
    fn r180_b8_every_tiers_handicap_is_a_valid_one_and_easys_deck_is_the_loadouts_20() {
        for &difficulty in DIFFICULTIES {
            assert!(state::validate_handicap(&AI_DIFFICULTY[difficulty], difficulty.as_str()).is_ok());
        }
        assert!(state::validate_handicap(&HUMAN_HANDICAP, "human").is_ok());
        assert_eq!(AI_DIFFICULTY.easy.deck_size, DECK_SIZE);
    }

    #[test]
    fn r180_b1_no_handicaps_human_handicap_or_easy_store_nothing_and_hash_exactly_like_the_plain_game() {
        let decks = decks_for(None);
        let plain = game("r180-b1", None, Some(decks.clone()));
        let variants: Vec<Handicaps> = vec![
            PerPlayerOpt::default(),
            on_p1(HUMAN_HANDICAP),
            on_p2(HUMAN_HANDICAP),
            on_both(AI_DIFFICULTY.easy, AI_DIFFICULTY.easy),
            // TS's `{ ...HUMAN_HANDICAP }`: a copy equal to it, which a Rust const always is.
            on_p2(HUMAN_HANDICAP),
        ];
        for handicaps in variants {
            let label = serde_json::to_string(&handicaps).expect("handicaps serialise");
            let state = game("r180-b1", Some(handicaps), Some(decks.clone()));
            for player in PLAYER_IDS {
                assert!(state.players[player].handicap.is_none(), "{label}: {player}");
                assert_eq!(
                    state::handicap_of(&state.players[player]),
                    HUMAN_HANDICAP,
                    "{label}: {player}"
                );
            }
            assert_eq!(hash_state(&state), hash_state(&plain), "{label}");
            assert_eq!(
                hash_state(&begin_game(&state).state),
                hash_state(&begin_game(&plain).state),
                "{label}"
            );
        }
    }

    #[test]
    fn r180_b1_a_handicap_that_differs_from_human_handicap_in_one_field_is_stored_as_a_copy_on_that_seat_only()
     {
        let bonus = Handicap {
            mana_bonus: 1,
            ..HUMAN_HANDICAP
        };
        let decks = decks_for(None);
        let plain = game("r180-b1-stored", None, Some(decks.clone()));
        let state = game("r180-b1-stored", Some(on_p2(bonus)), Some(decks));

        assert_eq!(state.players.p2.handicap, Some(bonus));
        // TS `not.toBe`: the stored handicap is a copy, never the caller's object.
        assert!(
            state
                .players
                .p2
                .handicap
                .as_ref()
                .is_some_and(|stored| !std::ptr::eq(stored, &bonus))
        );
        assert!(state.players.p1.handicap.is_none());
        assert_eq!(state::handicap_of(&state.players.p2), bonus);
        assert_ne!(hash_state(&state), hash_state(&plain));

        let hard = game("r180-b1-hard", Some(on_p1(AI_DIFFICULTY.hard)), None);
        assert_eq!(state::handicap_of(&hard.players.p1), AI_DIFFICULTY.hard);
        assert_eq!(state::handicap_of(&hard.players.p2), HUMAN_HANDICAP);
    }

    #[test]
    fn r180_b1_with_handicaps_for_both_seats_only_the_one_that_differs_from_human_handicap_is_stored() {
        let state = game(
            "r180-b1-both",
            Some(on_both(HUMAN_HANDICAP, AI_DIFFICULTY.medium)),
            None,
        );
        assert!(state.players.p1.handicap.is_none());
        assert_eq!(state.players.p2.handicap, Some(AI_DIFFICULTY.medium));
    }

    #[test]
    fn r180_validate_handicap_refuses_a_handicap_with_a_missing_field_or_a_field_of_the_wrong_type() {
        let mut missing = serde_json::to_value(AI_DIFFICULTY.hard).expect("a handicap serialises");
        missing
            .as_object_mut()
            .expect("a handicap is an object")
            .remove("manaCap");
        assert!(handicap_refused(missing, "p2"));
        assert!(handicap_refused(
            with_field(&AI_DIFFICULTY.hard, "manaBonus", json!("1")),
            "p2"
        ));
        assert!(handicap_refused(
            with_field(&AI_DIFFICULTY.hard, "deckSize", Value::Null),
            "p2"
        ));
    }

    #[test]
    fn r180_validate_handicap_refuses_a_negative_fractional_or_non_finite_field_and_a_deck_size_outside_1_library_cap()
     {
        let bad: Vec<Value> = vec![
            with_field(&HUMAN_HANDICAP, "manaBonus", json!(-1)),
            with_field(&HUMAN_HANDICAP, "manaCap", json!(4.5)),
            with_field(&HUMAN_HANDICAP, "extraOpeningCards", json!(-2)),
            // JSON has no NaN or Infinity: serde_json writes both as null, which no count deserialises from.
            with_field(&HUMAN_HANDICAP, "extraDrawsPerTurn", json!(f64::NAN)),
            with_field(&HUMAN_HANDICAP, "manaBonus", json!(f64::INFINITY)),
            with_field(&HUMAN_HANDICAP, "deckSize", json!(0)),
            with_field(&HUMAN_HANDICAP, "deckSize", json!(LIBRARY_CAP + 1)),
            with_field(&HUMAN_HANDICAP, "deckSize", json!(20.5)),
        ];
        for handicap in bad {
            let label = handicap.to_string();
            assert!(handicap_refused(handicap, "p2"), "{label}");
        }
        assert!(
            state::validate_handicap(
                &Handicap {
                    deck_size: 1,
                    ..HUMAN_HANDICAP
                },
                "p2"
            )
            .is_ok()
        );
        assert!(
            state::validate_handicap(
                &Handicap {
                    deck_size: LIBRARY_CAP,
                    ..HUMAN_HANDICAP
                },
                "p2"
            )
            .is_ok()
        );
        assert!(
            state::validate_handicap(
                &Handicap {
                    mana_cap: 0,
                    mana_bonus: 0,
                    ..HUMAN_HANDICAP
                },
                "p2"
            )
            .is_ok()
        );
    }

    #[test]
    fn r180_every_field_is_checked_a_negative_or_fractional_value_in_any_one_field_is_refused() {
        let fields = [
            "deckSize",
            "manaBonus",
            "manaCap",
            "extraOpeningCards",
            "extraDrawsPerTurn",
        ];
        for field in fields {
            for bad in [json!(-1), json!(0.5)] {
                let label = format!("{field} = {bad}");
                assert!(
                    handicap_refused(with_field(&AI_DIFFICULTY.medium, field, bad), "p2"),
                    "{label}"
                );
            }
        }
    }

    #[test]
    fn r180_create_game_refuses_an_invalid_handicap_before_the_game_exists() {
        register_all();
        let decks = decks_for(None);
        assert!(
            create_refusal(options(
                "bad",
                decks.clone(),
                on_p2(Handicap {
                    mana_bonus: -1,
                    ..HUMAN_HANDICAP
                })
            ))
            .is_some()
        );
        assert!(
            create_refusal_json(json!({
                "seed": "bad",
                "decks": [decks.0.clone(), decks.1.clone()],
                "handicaps": { "p1": with_field(&HUMAN_HANDICAP, "manaCap", json!(1.5)) },
            }))
            .is_some()
        );
        assert!(
            create_refusal(options(
                "bad",
                decks.clone(),
                on_p2(Handicap {
                    deck_size: LIBRARY_CAP + 1,
                    ..HUMAN_HANDICAP
                })
            ))
            .is_some()
        );
    }

    #[test]
    fn r180_b7_fold_with_the_handicaps_reproduces_a_handicapped_random_policy_games_hash() {
        let cases: Vec<(&str, Handicaps)> = vec![
            ("r180-fold-medium", on_p2(AI_DIFFICULTY.medium)),
            ("r180-fold-hard", on_p2(AI_DIFFICULTY.hard)),
            ("r180-fold-hard-p1", on_p1(AI_DIFFICULTY.hard)),
        ];
        for (seed, handicaps) in cases {
            let live = play_random(seed, handicaps.clone(), None);
            register_all();
            let replayed = fold_with(seed, &live, Some(handicaps));
            assert!(replayed.errors.is_empty(), "{seed}");
            assert_eq!(hash_state(&replayed.state), hash_state(&live.state), "{seed}");
            assert_eq!(replayed.state.result, live.state.result, "{seed}");
        }
    }

    #[test]
    fn r180_b7_the_same_fold_without_the_handicaps_throws_on_the_25_or_30_card_deck() {
        let medium = play_random("r180-fold-missing-medium", on_p2(AI_DIFFICULTY.medium), None);
        register_all();
        let message = refusal(|| {
            fold_with("r180-fold-missing-medium", &medium, None);
        });
        assert!(message.is_some_and(|m| m.contains("p2: deck must hold exactly 20")));

        let hard = play_random("r180-fold-missing-hard", on_p1(AI_DIFFICULTY.hard), None);
        register_all();
        let message = refusal(|| {
            fold_with("r180-fold-missing-hard", &hard, None);
        });
        assert!(message.is_some_and(|m| m.contains("p1: deck must hold exactly 20")));
    }

    #[test]
    fn r180_b7_fold_given_human_handicap_explicitly_replays_an_unhandicapped_game_exactly_as_fold_without_it()
    {
        let live = play_random("r180-fold-human", PerPlayerOpt::default(), None);
        register_all();
        let bare = fold_with("r180-fold-human", &live, None);
        let explicit = fold_with(
            "r180-fold-human",
            &live,
            Some(on_both(HUMAN_HANDICAP, AI_DIFFICULTY.easy)),
        );
        assert!(bare.errors.is_empty());
        assert!(explicit.errors.is_empty());
        assert_eq!(hash_state(&explicit.state), hash_state(&live.state));
        assert_eq!(hash_state(&bare.state), hash_state(&live.state));
    }

    #[test]
    fn r180_b7_a_fold_given_different_handicaps_does_not_reproduce_the_game() {
        let live = play_random("r180-fold-wrong", on_p2(AI_DIFFICULTY.hard), None);
        register_all();
        let wrong = fold_with(
            "r180-fold-wrong",
            &live,
            Some(on_p2(Handicap {
                mana_bonus: 0,
                ..AI_DIFFICULTY.hard
            })),
        );
        let same = wrong.errors.is_empty() && hash_state(&wrong.state) == hash_state(&live.state);
        assert!(!same);
    }
}

// ---------------------------------------------------------------------------
// R181: max mana.
// ---------------------------------------------------------------------------

mod r181_max_mana_under_a_handicap {
    use super::*;

    #[test]
    fn r181_b3_max_mana_for_is_min_turns_started_plus_mana_bonus_mana_cap_for_every_tier_and_s2_3_for_a_human()
     {
        for &difficulty in DIFFICULTIES {
            let h = AI_DIFFICULTY[difficulty];
            let mut state = game(&format!("r181-unit-{difficulty}"), Some(on_p2(h)), None);
            for turns in 0..=10 {
                state.players.p2.turns_started = turns;
                state.players.p1.turns_started = turns;
                assert_eq!(
                    mana::max_mana_for(&state.players.p2),
                    (turns + h.mana_bonus).min(h.mana_cap),
                    "{difficulty} after {turns} turns"
                );
                assert_eq!(
                    mana::max_mana_for(&state.players.p1),
                    turns.min(MAX_MANA),
                    "human after {turns} turns"
                );
            }
        }
    }

    #[test]
    fn r181_b3_a_persistent_modifier_applies_after_the_cap_a_next_turn_one_moves_only_that_refresh_and_both_floor_at_0()
     {
        let mut state = game("r181-mods", Some(on_p2(AI_DIFFICULTY.medium)), None);
        let side = &mut state.players.p2;
        let cap = AI_DIFFICULTY.medium.mana_cap;

        // §2.3 as task 4 read it: the next-turn rider is spent on one refresh and never reaches max mana.
        side.turns_started = 9;
        side.mana.next_turn_mod = 2;
        assert_eq!(mana::max_mana_for(side), cap);
        mana::refresh_mana(side);
        assert_eq!(
            (side.mana.max, side.mana.current, side.mana.next_turn_mod),
            (cap, cap + 2, 0)
        );

        side.mana.next_turn_mod = -1;
        mana::refresh_mana(side);
        assert_eq!(
            (side.mana.max, side.mana.current, side.mana.next_turn_mod),
            (cap, cap - 1, 0)
        );

        side.mana.perm_mod = 1;
        assert_eq!(mana::max_mana_for(side), cap + 1);

        side.turns_started = 0;
        side.mana.perm_mod = -5;
        side.mana.next_turn_mod = -1;
        assert_eq!(mana::max_mana_for(side), 0);
        mana::refresh_mana(side);
        assert_eq!((side.mana.max, side.mana.current), (0, 0));
    }

    #[test]
    fn r181_b3_a_medium_seat_refreshes_to_2_on_its_first_turn_and_to_5_from_its_fourth_the_human_is_unchanged()
     {
        let seen = maxes_by_turn("r181-medium", Some(on_p2(AI_DIFFICULTY.medium)), 12);
        assert_eq!(seen.p2, [2, 3, 4, 5, 5, 5]);
        assert_eq!(seen.p1, [1, 2, 3, 4, 4, 4]);
    }

    #[test]
    fn r181_b3_a_hard_seat_refreshes_to_7_from_its_sixth_turn_the_human_is_unchanged() {
        let seen = maxes_by_turn("r181-hard", Some(on_p2(AI_DIFFICULTY.hard)), 14);
        assert_eq!(seen.p2, [2, 3, 4, 5, 6, 7, 7]);
        assert_eq!(seen.p1, [1, 2, 3, 4, 4, 4, 4]);
    }

    #[test]
    fn r181_b3_a_handicapped_p1_gets_its_bonus_from_turn_1_and_no_handicap_is_s2_3s_min_turns_4() {
        let medium = maxes_by_turn("r181-medium-p1", Some(on_p1(AI_DIFFICULTY.medium)), 12);
        assert_eq!(medium.p1, [2, 3, 4, 5, 5, 5]);
        assert_eq!(medium.p2, [1, 2, 3, 4, 4, 4]);

        let plain = maxes_by_turn("r181-plain", None, 12);
        assert_eq!(plain.p1, [1, 2, 3, 4, 4, 4]);
        assert_eq!(plain.p2, [1, 2, 3, 4, 4, 4]);
    }

    #[test]
    fn r181_b3_hinder_drawn_by_the_human_still_costs_a_medium_seat_one_crystal_on_its_next_refresh() {
        let mut state = started("r181-hinder", Some(on_p2(AI_DIFFICULTY.medium)), None);
        state = pass_turn(&state).state; // p2's first turn: 2
        assert_eq!(state.players.p2.mana.max, 2);

        on_top_of_library(&mut state, PlayerId::P1, &[hinder().id]);
        state = pass_turn(&state).state; // p1 draws Hinder, cast on draw
        state = pass_turn(&state).state; // p2's second turn: max min(2 + 1, 5), refreshed to one less
        assert_eq!(state.active, PlayerId::P2);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (3, 2));

        state = advance_to(state, PlayerId::P2, 3);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (4, 4));
    }

    #[test]
    fn r181_b3_hinder_at_the_cap_takes_the_medium_seat_below_its_cap_not_back_to_it() {
        let mut state = advance_to(
            started("r181-hinder-cap", Some(on_p2(AI_DIFFICULTY.medium)), None),
            PlayerId::P2,
            4,
        );
        assert_eq!(state.players.p2.mana.max, 5);

        on_top_of_library(&mut state, PlayerId::P1, &[hinder().id]);
        state = advance_to(state, PlayerId::P2, 5);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (5, 4));

        state = advance_to(state, PlayerId::P2, 6);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (5, 5));
    }

    #[test]
    fn r181_b3_a_next_turn_gain_lifts_a_capped_hard_seats_refresh_above_its_cap_once_and_a_persistent_one_lifts_its_max_every_refresh()
     {
        let mut state = advance_to(
            started("r181-gain", Some(on_p2(AI_DIFFICULTY.hard)), None),
            PlayerId::P2,
            6,
        );
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (7, 7));

        state.players.p2.mana.next_turn_mod = 2;
        state = advance_to(state, PlayerId::P2, 7);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (7, 9));

        state = advance_to(state, PlayerId::P2, 8);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (7, 7));

        state.players.p2.mana.perm_mod = 1;
        state = advance_to(state, PlayerId::P2, 9);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (8, 8));
        state = advance_to(state, PlayerId::P2, 10);
        assert_eq!((state.players.p2.mana.max, state.players.p2.mana.current), (8, 8));
    }
}

// ---------------------------------------------------------------------------
// R182: the opening hand.
// ---------------------------------------------------------------------------

mod r182_the_opening_hand_under_a_handicap {
    use super::*;

    fn prompt_kind(state: &GameState, player: PlayerId) -> Option<PromptKind> {
        setup::mulligan_prompt_for(state, player).map(|prompt| prompt.kind)
    }

    fn prompt_options(state: &GameState, player: PlayerId) -> Option<usize> {
        setup::mulligan_prompt_for(state, player).map(|prompt| prompt.options.len())
    }

    #[test]
    fn r182_b4_opening_hand_size_is_s2_1s_table_entry_plus_extra_opening_cards() {
        for &difficulty in DIFFICULTIES {
            let extra = AI_DIFFICULTY[difficulty].extra_opening_cards;
            let as_p2 = game(
                &format!("r182-size-p2-{difficulty}"),
                Some(on_p2(AI_DIFFICULTY[difficulty])),
                None,
            );
            assert_eq!(setup::opening_hand_size(&as_p2, PlayerId::P1), OPENING_DRAW[0]);
            assert_eq!(
                setup::opening_hand_size(&as_p2, PlayerId::P2),
                OPENING_DRAW[1] + extra
            );

            let as_p1 = game(
                &format!("r182-size-p1-{difficulty}"),
                Some(on_p1(AI_DIFFICULTY[difficulty])),
                None,
            );
            assert_eq!(
                setup::opening_hand_size(&as_p1, PlayerId::P1),
                OPENING_DRAW[0] + extra
            );
            assert_eq!(setup::opening_hand_size(&as_p1, PlayerId::P2), OPENING_DRAW[1]);
        }
    }

    #[test]
    fn r182_b4_the_mulligan_offers_5_cards_to_a_medium_or_hard_p2_and_3_to_the_human_p1() {
        for difficulty in [Difficulty::Medium, Difficulty::Hard] {
            let h = AI_DIFFICULTY[difficulty];
            let mut state = begin_game(&game(&format!("r182-p2-{difficulty}"), Some(on_p2(h)), None)).state;

            assert_eq!(prompt_kind(&state, PlayerId::P1), Some(PromptKind::Mulligan));
            assert_eq!(
                prompt_options(&state, PlayerId::P1),
                Some(OPENING_DRAW[0] as usize),
                "{difficulty}"
            );
            assert_eq!(state.players.p1.hand.len(), OPENING_DRAW[0] as usize);

            let keep = ids(&state.players.p1.hand);
            state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));
            assert_eq!(prompt_kind(&state, PlayerId::P2), Some(PromptKind::Mulligan));
            assert_eq!(prompt_options(&state, PlayerId::P2), Some(5), "{difficulty}");
            assert_eq!(state.players.p2.hand.len(), 5);
            assert_eq!(state.players.p2.library.len(), (h.deck_size - 5) as usize);
        }
    }

    #[test]
    fn r182_b4_the_mulligan_offers_4_cards_to_a_medium_or_hard_p1_and_4_to_the_human_p2() {
        for difficulty in [Difficulty::Medium, Difficulty::Hard] {
            let h = AI_DIFFICULTY[difficulty];
            let mut state = begin_game(&game(&format!("r182-p1-{difficulty}"), Some(on_p1(h)), None)).state;

            assert_eq!(prompt_options(&state, PlayerId::P1), Some(4), "{difficulty}");
            assert_eq!(state.players.p1.hand.len(), 4);
            assert_eq!(state.players.p1.library.len(), (h.deck_size - 4) as usize);

            let keep = ids(&state.players.p1.hand);
            state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));
            assert_eq!(
                prompt_options(&state, PlayerId::P2),
                Some(OPENING_DRAW[1] as usize),
                "{difficulty}"
            );
            assert_eq!(state.players.p2.hand.len(), OPENING_DRAW[1] as usize);
        }
    }

    #[test]
    fn r182_b4_an_easy_seat_opens_exactly_as_a_human_does() {
        let mut state = begin_game(&game(
            "r182-easy",
            Some(on_both(AI_DIFFICULTY.easy, AI_DIFFICULTY.easy)),
            None,
        ))
        .state;
        assert_eq!(
            prompt_options(&state, PlayerId::P1),
            Some(OPENING_DRAW[0] as usize)
        );
        let keep = ids(&state.players.p1.hand);
        state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));
        assert_eq!(
            prompt_options(&state, PlayerId::P2),
            Some(OPENING_DRAW[1] as usize)
        );
    }

    #[test]
    fn r182_b4_a_quickdraw_card_replaces_one_of_the_medium_seats_five_opening_draws() {
        let h = AI_DIFFICULTY.medium;
        let mut p2_deck = vec![going_long().id];
        p2_deck.extend(vanilla_deck(h.deck_size - 1, 1));
        let mut state = begin_game(&game(
            "r182-quickdraw",
            Some(on_p2(h)),
            Some((vanilla_deck(DECK_SIZE, 1), p2_deck)),
        ))
        .state;
        let keep = ids(&state.players.p1.hand);
        state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));

        let hand = def_ids(&state.players.p2.hand);
        assert_eq!(hand.len(), 5);
        assert!(hand.contains(&going_long().id));
        assert_eq!(
            hand.iter().filter(|def_id| **def_id != going_long().id).count(),
            4
        );
        assert_eq!(prompt_options(&state, PlayerId::P2), Some(5));
        assert_eq!(state.players.p2.library.len(), (h.deck_size - 5) as usize);
    }

    #[test]
    fn r182_b4_two_quickdraw_cards_replace_two_of_the_medium_seats_five_opening_draws() {
        let h = AI_DIFFICULTY.medium;
        let mut p2_deck = vec![going_long().id, heroic_power().id];
        p2_deck.extend(vanilla_deck(h.deck_size - 2, 1));
        let mut state = begin_game(&game(
            "r182-quickdraw-two",
            Some(on_p2(h)),
            Some((vanilla_deck(DECK_SIZE, 1), p2_deck)),
        ))
        .state;
        let keep = ids(&state.players.p1.hand);
        state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));

        let hand = def_ids(&state.players.p2.hand);
        assert_eq!(hand.len(), 5);
        assert!(hand.contains(&going_long().id));
        assert!(hand.contains(&heroic_power().id));
        assert_eq!(state.players.p2.library.len(), (h.deck_size - 5) as usize);
    }

    #[test]
    fn r182_b4_the_hard_seats_mulligan_returns_and_redraws_its_whole_five_card_hand_per_s2_1() {
        let mut state = begin_game(&game("r182-mulligan", Some(on_p2(AI_DIFFICULTY.hard)), None)).state;
        let keep = ids(&state.players.p1.hand);
        state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }));

        let before = ids(&state.players.p2.hand);
        assert_eq!(before.len(), 5);
        let keep = vec![before[0].clone()];
        let returned = before[1..].to_vec();

        state = act(
            &state,
            input(PlayerId::P2, ActionBody::Mulligan { keep: keep.clone() }),
        );
        let after = ids(&state.players.p2.hand);
        // The turn has begun (p1's), so p2's hand is exactly its redrawn opening hand.
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(after.len(), 5);
        assert!(after.contains(&keep[0]));
        for id in &returned {
            assert!(!after.contains(id));
            assert!(state.players.p2.library.iter().any(|c| c.id == *id));
        }
        assert_eq!(
            state.players.p2.library.len(),
            (AI_DIFFICULTY.hard.deck_size - 5) as usize
        );
    }
}

// ---------------------------------------------------------------------------
// R183: extra draws per turn.
// ---------------------------------------------------------------------------

mod r183_extra_draws_per_turn {
    use super::*;

    #[test]
    fn r183_b5_a_hard_seats_turn_start_makes_two_draws_and_its_hand_grows_by_2_the_human_draws_one() {
        let mut state = started("r183-two", Some(on_p2(AI_DIFFICULTY.hard)), None);
        let hand_before = state.players.p2.hand.len();
        let library_before = state.players.p2.library.len();

        let p2_turn = pass_turn(&state);
        state = p2_turn.state;
        assert_eq!(
            drawn_by(&p2_turn.events, PlayerId::P2).len(),
            (DRAWS_PER_TURN + 1) as usize
        );
        assert_eq!(state.players.p2.hand.len(), hand_before + 2);
        assert_eq!(state.players.p2.library.len(), library_before - 2);

        let p1_hand_before = state.players.p1.hand.len();
        let p1_turn = pass_turn(&state);
        assert_eq!(
            drawn_by(&p1_turn.events, PlayerId::P1).len(),
            DRAWS_PER_TURN as usize
        );
        assert_eq!(p1_turn.state.players.p1.hand.len(), p1_hand_before + 1);
    }

    #[test]
    fn r183_b5_a_medium_seat_still_draws_one_card_a_turn() {
        let state = started("r183-medium", Some(on_p2(AI_DIFFICULTY.medium)), None);
        let hand_before = state.players.p2.hand.len();
        let p2_turn = pass_turn(&state);
        assert_eq!(drawn_by(&p2_turn.events, PlayerId::P2).len(), 1);
        assert_eq!(p2_turn.state.players.p2.hand.len(), hand_before + 1);
    }

    #[test]
    fn r183_b5_from_an_empty_library_the_hard_seats_two_draws_are_two_fatigue_steps_n_then_n_plus_1() {
        let mut state = started("r183-fatigue", Some(on_p2(AI_DIFFICULTY.hard)), None);
        state.players.p2.library = vec![];
        state.players.p2.fatigue_count = 2;
        let hand_before = state.players.p2.hand.len();

        let p2_turn = pass_turn(&state);
        assert_eq!(hero_hits(&p2_turn.events, "hero-p2"), [3, 4]);
        assert_eq!(p2_turn.state.players.p2.fatigue_count, 4);
        assert_eq!(p2_turn.state.players.p2.hero.health, HERO_HEALTH - 7);
        assert_eq!(drawn_by(&p2_turn.events, PlayerId::P2), Vec::<String>::new());
        assert_eq!(p2_turn.state.players.p2.hand.len(), hand_before);
    }

    #[test]
    fn r183_b5_a_medium_seat_with_an_empty_library_takes_a_single_fatigue_step() {
        let mut state = started("r183-medium-fatigue", Some(on_p2(AI_DIFFICULTY.medium)), None);
        state.players.p2.library = vec![];
        let p2_turn = pass_turn(&state);
        assert_eq!(hero_hits(&p2_turn.events, "hero-p2"), [1]);
        assert_eq!(p2_turn.state.players.p2.fatigue_count, 1);
        assert_eq!(p2_turn.state.players.p2.hero.health, HERO_HEALTH - 1);
    }

    #[test]
    fn r183_b5_easy_seats_draw_exactly_one_card_each_turn_on_both_sides() {
        let mut state = started(
            "r183-easy",
            Some(on_both(AI_DIFFICULTY.easy, AI_DIFFICULTY.easy)),
            None,
        );
        for _ in 0..4 {
            let next = pass_turn(&state);
            assert_eq!(
                drawn_by(&next.events, next.state.active).len(),
                1,
                "turn {}",
                next.state.turn
            );
            state = next.state;
        }
    }

    #[test]
    fn r183_b5_each_of_the_hard_seats_draws_makes_its_own_hand_cap_check() {
        let mut state = started("r183-hand-cap", Some(on_p2(AI_DIFFICULTY.hard)), None);
        while state.players.p2.hand.len() < (HAND_CAP - 1) as usize {
            let card = state::new_instance(
                &mut state,
                "fx-40",
                PlayerId::P2,
                Zone::Hand { player: PlayerId::P2 },
            );
            state.players.p2.hand.push(card);
        }

        let drawn_before = state.counters.drawn;
        let p2_turn = pass_turn(&state);
        // Two draws were made (R55's counter), the first filled the hand and the second burned (R4).
        assert_eq!(p2_turn.state.counters.drawn - drawn_before, 2);
        assert_eq!(
            p2_turn
                .events
                .iter()
                .filter(|e| matches!(
                    e,
                    GameEvent::Burned {
                        owner: PlayerId::P2,
                        ..
                    }
                ))
                .count(),
            1
        );
        assert_eq!(p2_turn.state.players.p2.hand.len(), HAND_CAP as usize);
    }

    #[test]
    fn r183_b6_a_cast_on_draw_card_met_by_the_first_draw_runs_its_chain_before_the_second_draw() {
        let mut state = started("r183-chain", Some(on_p2(AI_DIFFICULTY.hard)), None);
        let placed = on_top_of_library(
            &mut state,
            PlayerId::P2,
            &[quiet_on_draw().id, "fx-35".to_string(), "fx-36".to_string()],
        );
        let (quiet, first, second) = (placed[0].clone(), placed[1].clone(), placed[2].clone());
        let hand_before = state.players.p2.hand.len();

        let p2_turn = pass_turn(&state);
        let order: Vec<String> = p2_turn
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn {
                    player: PlayerId::P2,
                    instance_id,
                    ..
                } => Some(format!("drawn:{instance_id}")),
                GameEvent::CardPlayed { instance_id, .. } if *instance_id == quiet.id => {
                    Some("cast".to_string())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            order,
            [
                format!("drawn:{}", quiet.id),
                "cast".to_string(),
                format!("drawn:{}", first.id),
                format!("drawn:{}", second.id)
            ]
        );
        let hand = ids(&p2_turn.state.players.p2.hand);
        assert!(hand.contains(&first.id));
        assert!(hand.contains(&second.id));
        assert!(!hand.contains(&quiet.id));
        assert_eq!(hand.len(), hand_before + 2);
        assert_eq!(p2_turn.state.phase, Phase::Main);
        assert!(p2_turn.state.work.is_empty());
    }

    #[test]
    fn r183_b6_a_prompt_opened_by_the_first_draw_owes_the_second_draw_and_the_answer_makes_it() {
        let mut state = started("r183-prompt", Some(on_p2(AI_DIFFICULTY.hard)), None);
        let placed = on_top_of_library(
            &mut state,
            PlayerId::P2,
            &[ask_on_draw().id, "fx-35".to_string(), "fx-36".to_string()],
        );
        let (ask, first, second) = (placed[0].clone(), placed[1].clone(), placed[2].clone());
        let hand_before = ids(&state.players.p2.hand);

        let paused = pass_turn(&state);
        // The first draw cast the asker and stopped: nothing further was drawn.
        assert_eq!(
            paused.state.pending.as_ref().map(|p| p.player_id),
            Some(PlayerId::P2)
        );
        assert_eq!(drawn_by(&paused.events, PlayerId::P2), vec![ask.id.clone()]);
        assert_eq!(
            ids(&paused.state.players.p2.library[..2]),
            vec![first.id.clone(), second.id.clone()]
        );
        assert_eq!(ids(&paused.state.players.p2.hand), hand_before);
        // R183 and R158: the second whole draw is owed on state.work, as plain data.
        let owed = work::owed_work(&paused.state, Some(draw::DRAW_COUNT_WORK));
        assert_eq!(owed.len(), 1);
        assert_eq!(
            draw::owed_draw_count_of(&owed[0].resume).map(|owed| (owed.player, owed.count)),
            Some((PlayerId::P2, 1))
        );

        let pending = paused
            .state
            .pending
            .clone()
            .expect("expected the cast-on-draw prompt");
        let answered = step(
            &round_trip(&paused.state),
            input(
                PlayerId::P2,
                ActionBody::Answer {
                    choice_id: pending.id.clone(),
                    selection: vec![Selection::None],
                },
            ),
        );

        // The rest of the first draw's chain, then the second draw: both cards, once each.
        assert_eq!(
            drawn_by(&answered.events, PlayerId::P2),
            vec![first.id.clone(), second.id.clone()]
        );
        let hand = ids(&answered.state.players.p2.hand);
        assert!(hand.contains(&first.id));
        assert!(hand.contains(&second.id));
        assert_eq!(hand.len(), hand_before.len() + 2);
        assert!(answered.state.pending.is_none());
        assert_eq!(answered.state.phase, Phase::Main);
        assert_eq!(answered.state.active, PlayerId::P2);
        assert!(answered.state.work.is_empty());
    }

    #[test]
    fn r183_b6_the_owed_draw_is_made_once_a_repeated_or_second_answer_draws_nothing_more() {
        let mut state = started("r183-prompt-once", Some(on_p2(AI_DIFFICULTY.hard)), None);
        on_top_of_library(
            &mut state,
            PlayerId::P2,
            &[
                ask_on_draw().id,
                "fx-35".to_string(),
                "fx-36".to_string(),
                "fx-37".to_string(),
            ],
        );
        let paused = pass_turn(&state).state;
        let pending = paused.pending.clone().expect("expected the cast-on-draw prompt");

        let answer = Action::new(
            ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![Selection::None],
            },
            PlayerId::P2,
            "r183-once-answer",
        );
        let first = reduce(&paused, &answer);
        assert!(first.error.is_none());
        let hand = ids(&first.state.players.p2.hand);
        let library = ids(&first.state.players.p2.library);

        // The same submission again (same nonce) is deduplicated: nothing changes.
        let repeated = reduce(&first.state, &answer);
        assert!(repeated.error.is_none());
        assert_eq!(ids(&repeated.state.players.p2.hand), hand);
        assert_eq!(ids(&repeated.state.players.p2.library), library);

        // A fresh answer to the closed prompt is refused, and draws nothing.
        let second = reduce(
            &first.state,
            &Action {
                nonce: "r183-once-again".to_string(),
                ..answer.clone()
            },
        );
        assert!(second.error.is_some());
        assert_eq!(ids(&second.state.players.p2.hand), hand);
        assert_eq!(ids(&second.state.players.p2.library), library);
    }
}

// ---------------------------------------------------------------------------
// R184: the deck size of a handicapped seat.
// ---------------------------------------------------------------------------

mod r184_deck_size_for_a_handicapped_seat {
    use super::*;

    #[test]
    fn r184_b2_a_hard_p2_takes_a_30_card_distinct_token_free_deck_a_medium_p2_a_25_card_one() {
        let hard = game(
            "r184-hard",
            Some(on_p2(AI_DIFFICULTY.hard)),
            Some((vanilla_deck(DECK_SIZE, 1), vanilla_deck(30, 1))),
        );
        assert_eq!(hard.players.p2.library.len(), 30);
        assert_eq!(hard.players.p1.library.len(), DECK_SIZE as usize);

        let medium = game(
            "r184-medium",
            Some(on_p2(AI_DIFFICULTY.medium)),
            Some((vanilla_deck(DECK_SIZE, 1), vanilla_deck(25, 1))),
        );
        assert_eq!(medium.players.p2.library.len(), 25);
    }

    #[test]
    fn r184_b2_a_hard_p2_with_a_20_card_deck_is_refused_naming_the_seat_and_its_handicap() {
        register_all();
        let message = create_refusal(options(
            "r184-short",
            (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 1)),
            on_p2(AI_DIFFICULTY.hard),
        ));
        assert!(
            message.is_some_and(|m| m.contains("p2: deck must hold exactly 30 cards (its handicap, R184)"))
        );
    }

    #[test]
    fn r184_b2_one_card_short_or_over_is_refused_for_a_hard_p2() {
        register_all();
        for size in [29, 31] {
            let message = create_refusal(options(
                &format!("r184-{size}"),
                (vanilla_deck(DECK_SIZE, 1), vanilla_deck(size, 1)),
                on_p2(AI_DIFFICULTY.hard),
            ));
            assert!(
                message.is_some_and(|m| m.contains("p2: deck must hold exactly 30 cards")),
                "{size} cards"
            );
        }
    }

    #[test]
    fn r184_b2_a_card_missing_from_the_catalog_in_the_hard_deck_is_refused_naming_the_seat() {
        register_all();
        let mut with_ghost = vanilla_deck(29, 1);
        with_ghost.push("fx-does-not-exist".to_string());
        let message = create_refusal(options(
            "r184-ghost",
            (vanilla_deck(DECK_SIZE, 1), with_ghost),
            on_p2(AI_DIFFICULTY.hard),
        ));
        assert!(message.is_some_and(|m| in_order(&m, &["p2", "not in the catalog (§9.4 L6)"])));
    }

    #[test]
    fn r184_b2_an_easy_handicap_keeps_the_20_card_rule_so_a_25_card_deck_is_refused_with_s2_6s_message() {
        register_all();
        let message = create_refusal(options(
            "r184-easy",
            (vanilla_deck(DECK_SIZE, 1), vanilla_deck(25, 1)),
            on_p2(AI_DIFFICULTY.easy),
        ));
        assert!(message.is_some_and(|m| m.contains("p2: deck must hold exactly 20 cards (§2.6 L2)")));
    }

    #[test]
    fn r184_b2_a_medium_p1_takes_25_cards_while_the_human_p2_is_still_held_to_20() {
        let state = game(
            "r184-medium-p1",
            Some(on_p1(AI_DIFFICULTY.medium)),
            Some((vanilla_deck(25, 1), vanilla_deck(DECK_SIZE, 1))),
        );
        assert_eq!(state.players.p1.library.len(), 25);
        assert_eq!(state.players.p2.library.len(), DECK_SIZE as usize);

        register_all();
        let message = create_refusal(options(
            "r184-medium-p1-bad",
            (vanilla_deck(25, 1), vanilla_deck(25, 1)),
            on_p1(AI_DIFFICULTY.medium),
        ));
        assert!(message.is_some_and(|m| m.contains("p2: deck must hold exactly 20 cards (§2.6 L2)")));
    }

    #[test]
    fn r184_b2_a_medium_p2_with_a_30_card_deck_is_refused() {
        register_all();
        let message = create_refusal(options(
            "r184-long",
            (vanilla_deck(DECK_SIZE, 1), vanilla_deck(30, 1)),
            on_p2(AI_DIFFICULTY.medium),
        ));
        assert!(message.is_some_and(|m| m.contains("p2: deck must hold exactly 25 cards")));
    }

    #[test]
    fn r184_b2_the_unhandicapped_p1_is_still_held_to_20_with_s2_6s_own_message() {
        register_all();
        let message = create_refusal(options(
            "r184-p1",
            (vanilla_deck(30, 1), vanilla_deck(30, 1)),
            on_p2(AI_DIFFICULTY.hard),
        ));
        assert!(message.is_some_and(|m| m.contains("p1: deck must hold exactly 20 cards (§2.6 L2)")));
    }

    #[test]
    fn r184_b2_a_duplicate_id_in_the_hard_deck_is_refused_naming_the_seat() {
        register_all();
        let mut with_duplicate = vanilla_deck(29, 1);
        with_duplicate.push("fx-1".to_string());
        let message = create_refusal(options(
            "r184-dup",
            (vanilla_deck(DECK_SIZE, 1), with_duplicate),
            on_p2(AI_DIFFICULTY.hard),
        ));
        assert!(message.is_some_and(|m| in_order(&m, &["p2", "appears twice", "§2.6 L3"])));
    }

    #[test]
    fn r184_b2_a_token_card_in_the_hard_deck_is_refused_naming_the_seat() {
        register_all();
        let mut with_token = vanilla_deck(29, 1);
        with_token.push(token_def("rush", [Tag::Token]).id);
        let message = create_refusal(options(
            "r184-token",
            (vanilla_deck(DECK_SIZE, 1), with_token),
            on_p2(AI_DIFFICULTY.hard),
        ));
        assert!(message.is_some_and(|m| in_order(&m, &["p2", "is a Token card", "§2.6 L3"])));
    }

    #[test]
    fn r184_b2_validate_deck_holds_any_size_exactly_one_card_for_a_size_of_1_none_refused() {
        let catalog = vanilla_catalog(40, 1);
        assert!(state::validate_deck(&["fx-1".to_string()], &catalog, "p2", 1).is_ok());
        assert!(state::validate_deck(&[], &catalog, "p2", 1).is_err_and(|e| e.message.contains("p2")));
        assert!(
            state::validate_deck(&["fx-1".to_string(), "fx-2".to_string()], &catalog, "p2", 1)
                .is_err_and(|e| e.message.contains("p2"))
        );
    }

    #[test]
    fn r184_b2_validate_deck_keeps_s2_6s_message_byte_identical_at_deck_size_and_names_r184_at_any_other_size()
     {
        let catalog = vanilla_catalog(40, 1);
        // TS's default `size` is DECK_SIZE; Rust passes it.
        assert!(state::validate_deck(&vanilla_deck(DECK_SIZE, 1), &catalog, "p1", DECK_SIZE).is_ok());
        assert!(state::validate_deck(&vanilla_deck(DECK_SIZE, 1), &catalog, "p1", DECK_SIZE).is_ok());
        assert!(state::validate_deck(&vanilla_deck(30, 1), &catalog, "p2", 30).is_ok());

        let via_validate = state::validate_deck(&vanilla_deck(19, 1), &catalog, "p1", DECK_SIZE)
            .err()
            .map(|error| error.message)
            .unwrap_or_default();
        let via_create = create_refusal(CreateGameOptions {
            seed: "r184-msg".to_string(),
            decks: (vanilla_deck(19, 1), vanilla_deck(DECK_SIZE, 21)),
            catalog: Some(catalog.clone()),
            ..Default::default()
        })
        .unwrap_or_default();
        assert!(via_validate.contains("exactly 20 cards (§2.6 L2)"));
        assert!(!via_validate.contains("R184"));
        assert_eq!(via_create, via_validate);

        assert!(
            state::validate_deck(&vanilla_deck(25, 1), &catalog, "p2", 30).is_err_and(|e| e
                .message
                .contains("p2: deck must hold exactly 30 cards (its handicap, R184)"))
        );
        assert!(
            state::validate_deck(&vanilla_deck(30, 1), &catalog, "p2", DECK_SIZE)
                .is_err_and(|e| e.message.contains("exactly 20 cards (§2.6 L2)"))
        );
    }
}

// ---------------------------------------------------------------------------
// R290: the tutorial's handicap, the one below Easy.
// ---------------------------------------------------------------------------

/// The five fields every handicap has; `heroHealth` (R290) is the optional sixth.
const FIVE_FIELDS: &[&str] = &[
    "deckSize",
    "manaBonus",
    "manaCap",
    "extraOpeningCards",
    "extraDrawsPerTurn",
];

/// The tutorial hero's starting health, read through the engine's own rule.
fn tutorial_health() -> i32 {
    state::starting_hero_health(Some(&AI_TUTORIAL))
}

/// AI_TUTORIAL without its heroHealth: what a fold that forgot the field would be given.
fn without_hero_health(handicap: Handicap) -> Handicap {
    Handicap {
        hero_health: None,
        ..handicap
    }
}

fn field_of(handicap: &Handicap, field: &str) -> i64 {
    serde_json::to_value(handicap).expect("a handicap serialises")[field]
        .as_i64()
        .expect("a handicap field is a number")
}

fn five_fields_sorted() -> Vec<String> {
    let mut fields: Vec<String> = FIVE_FIELDS.iter().map(|field| field.to_string()).collect();
    fields.sort();
    fields
}

mod r290_the_tutorial_handicap_ai_tutorial {
    use super::*;

    #[test]
    fn r290_ai_tutorial_is_a_12_card_deck_at_most_3_mana_a_20_health_hero_and_nothing_extra_and_a_valid_handicap()
     {
        assert_eq!(
            AI_TUTORIAL,
            Handicap {
                deck_size: 12,
                mana_bonus: 0,
                mana_cap: 3,
                extra_opening_cards: 0,
                extra_draws_per_turn: 0,
                hero_health: Some(20),
            }
        );
        assert_eq!(tutorial_health(), 20);
        assert!(state::validate_handicap(&AI_TUTORIAL, "p2").is_ok());
        assert!(state::validate_handicap(&AI_TUTORIAL, "p1").is_ok());
    }

    #[test]
    fn r290_ai_tutorial_is_no_practice_tier_not_in_difficulties_not_a_key_of_ai_difficulty_equal_to_none_of_them()
     {
        assert_eq!(
            DIFFICULTIES.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            ["easy", "medium", "hard"]
        );
        let keys = keys_sorted(&serde_json::to_value(AI_DIFFICULTY).expect("the table serialises"));
        assert_eq!(keys, ["easy", "hard", "medium"]);
        assert!(!keys.contains(&"tutorial".to_string()));
        assert!(![AI_DIFFICULTY.easy, AI_DIFFICULTY.medium, AI_DIFFICULTY.hard].contains(&AI_TUTORIAL));
        for &difficulty in DIFFICULTIES {
            assert_ne!(AI_DIFFICULTY[difficulty], AI_TUTORIAL, "{difficulty}");
            // The three tiers never set the field, so they start at HERO_HEALTH (SPEC §9.9's table).
            assert!(AI_DIFFICULTY[difficulty].hero_health.is_none(), "{difficulty}");
            assert_eq!(
                state::starting_hero_health(Some(&AI_DIFFICULTY[difficulty])),
                HERO_HEALTH,
                "{difficulty}"
            );
        }
        assert!(HUMAN_HANDICAP.hero_health.is_none());
        assert_eq!(state::starting_hero_health(Some(&HUMAN_HANDICAP)), HERO_HEALTH);
        assert_eq!(state::starting_hero_health(None), HERO_HEALTH);
    }

    #[test]
    fn r290_ai_tutorial_is_below_a_humans_resources_every_field_at_or_under_easys_and_the_three_it_changes_strictly_under()
     {
        const { assert!(AI_TUTORIAL.deck_size < DECK_SIZE) };
        const { assert!(AI_TUTORIAL.mana_cap < MAX_MANA) };
        assert!(tutorial_health() < HERO_HEALTH);
        assert_eq!(AI_TUTORIAL.mana_bonus, 0);
        assert_eq!(AI_TUTORIAL.extra_opening_cards, 0);
        assert_eq!(AI_TUTORIAL.extra_draws_per_turn, 0);

        let mut changed: Vec<&str> = Vec::new();
        for field in FIVE_FIELDS {
            assert!(
                field_of(&AI_TUTORIAL, field) <= field_of(&HUMAN_HANDICAP, field),
                "{field}"
            );
            if field_of(&AI_TUTORIAL, field) != field_of(&HUMAN_HANDICAP, field) {
                changed.push(field);
            }
        }
        assert!(tutorial_health() <= state::starting_hero_health(Some(&HUMAN_HANDICAP)));
        if tutorial_health() != state::starting_hero_health(Some(&HUMAN_HANDICAP)) {
            changed.push("heroHealth");
        }
        assert_eq!(changed, ["deckSize", "manaCap", "heroHealth"]);
        // Easy is a human's resources exactly (R180), so "below a human" is "below Easy".
        assert_eq!(AI_DIFFICULTY.easy, HUMAN_HANDICAP);
    }

    #[test]
    fn r290_create_game_a_tutorial_p2_takes_12_cards_and_its_hero_starts_at_20_stored_as_a_copy_p1_keeps_hero_health()
     {
        let state = game("r290-create", Some(on_p2(AI_TUTORIAL)), None);
        assert_eq!(state.players.p2.library.len(), AI_TUTORIAL.deck_size as usize);
        assert_eq!(state.players.p1.library.len(), DECK_SIZE as usize);
        assert_eq!(
            state.players.p2.hero,
            HeroState {
                health: tutorial_health(),
                armor: 0
            }
        );
        assert_eq!(
            state.players.p1.hero,
            HeroState {
                health: HERO_HEALTH,
                armor: 0
            }
        );

        assert_eq!(state.players.p2.handicap, Some(AI_TUTORIAL));
        // TS `not.toBe`: the stored handicap is a copy, never the constant itself.
        assert!(
            state
                .players
                .p2
                .handicap
                .as_ref()
                .is_some_and(|stored| !std::ptr::eq(stored, &AI_TUTORIAL))
        );
        assert!(state.players.p1.handicap.is_none());
        assert_eq!(state::handicap_of(&state.players.p2), AI_TUTORIAL);
        assert_eq!(state::handicap_of(&state.players.p1), HUMAN_HANDICAP);
        // The stored copy survives the JSON round trip a paused or restarted match takes.
        assert_eq!(round_trip(&state).players.p2.handicap, Some(AI_TUTORIAL));
        assert_eq!(hash_state(&round_trip(&state)), hash_state(&state));

        // The setup and the mulligans do not reset it, and both seats' views show it.
        let begun = started("r290-create", Some(on_p2(AI_TUTORIAL)), None);
        assert_eq!(begun.players.p2.hero.health, tutorial_health());
        assert_eq!(begun.players.p1.hero.health, HERO_HEALTH);
        assert_eq!(
            view_for(&begun, PlayerId::P1).opponent.hero.health,
            tutorial_health()
        );
        assert_eq!(view_for(&begun, PlayerId::P1).you.hero.health, HERO_HEALTH);
        assert_eq!(view_for(&begun, PlayerId::P2).you.hero.health, tutorial_health());
    }

    #[test]
    fn r290_create_game_a_tutorial_p1_is_the_same_and_the_p2_human_keeps_hero_health_and_its_20_cards() {
        let state = game("r290-create-p1", Some(on_p1(AI_TUTORIAL)), None);
        assert_eq!(state.players.p1.library.len(), AI_TUTORIAL.deck_size as usize);
        assert_eq!(state.players.p2.library.len(), DECK_SIZE as usize);
        assert_eq!(state.players.p1.hero.health, tutorial_health());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(state.players.p1.handicap, Some(AI_TUTORIAL));
        assert!(state.players.p2.handicap.is_none());
    }

    #[test]
    fn r290_a_tutorial_seats_deck_is_held_to_exactly_12_r184_20_11_or_13_cards_are_refused_and_the_human_is_still_held_to_20()
     {
        register_all();
        for size in [DECK_SIZE, AI_TUTORIAL.deck_size - 1, AI_TUTORIAL.deck_size + 1] {
            let message = create_refusal(options(
                &format!("r290-deck-{size}"),
                (vanilla_deck(DECK_SIZE, 1), vanilla_deck(size, 1)),
                on_p2(AI_TUTORIAL),
            ));
            assert!(
                message
                    .is_some_and(|m| m.contains("p2: deck must hold exactly 12 cards (its handicap, R184)")),
                "{size} cards"
            );
        }
        let message = create_refusal(options(
            "r290-deck-human",
            (
                vanilla_deck(AI_TUTORIAL.deck_size, 1),
                vanilla_deck(AI_TUTORIAL.deck_size, 1),
            ),
            on_p2(AI_TUTORIAL),
        ));
        assert!(message.is_some_and(|m| m.contains("p1: deck must hold exactly 20 cards (§2.6 L2)")));
    }

    #[test]
    fn r290_hero_health_absent_or_equal_to_hero_health_stores_nothing_a_humans_handicap_with_hero_health_30_hashes_like_the_plain_game()
     {
        let decks = decks_for(None);
        let plain = game("r290-human", None, Some(decks.clone()));
        let variants: Vec<Handicaps> = vec![
            on_p2(Handicap {
                hero_health: Some(HERO_HEALTH),
                ..HUMAN_HANDICAP
            }),
            on_p1(Handicap {
                hero_health: Some(HERO_HEALTH),
                ..HUMAN_HANDICAP
            }),
            on_both(
                Handicap {
                    hero_health: Some(HERO_HEALTH),
                    ..AI_DIFFICULTY.easy
                },
                Handicap {
                    hero_health: Some(HERO_HEALTH),
                    ..HUMAN_HANDICAP
                },
            ),
        ];
        for handicaps in variants {
            let label = serde_json::to_string(&handicaps).expect("handicaps serialise");
            let state = game("r290-human", Some(handicaps), Some(decks.clone()));
            for player in PLAYER_IDS {
                assert!(state.players[player].handicap.is_none(), "{label}: {player}");
                assert_eq!(
                    state::handicap_of(&state.players[player]),
                    HUMAN_HANDICAP,
                    "{label}: {player}"
                );
                assert_eq!(
                    state.players[player].hero.health, HERO_HEALTH,
                    "{label}: {player}"
                );
            }
            assert_eq!(hash_state(&state), hash_state(&plain), "{label}");
            assert_eq!(
                hash_state(&begin_game(&state).state),
                hash_state(&begin_game(&plain).state),
                "{label}"
            );
        }
    }

    #[test]
    fn r290_medium_and_hard_store_the_five_fields_and_no_hero_health_key_and_a_hero_health_of_30_added_to_them_changes_nothing()
     {
        for difficulty in [Difficulty::Medium, Difficulty::Hard] {
            let h = AI_DIFFICULTY[difficulty];
            let decks = decks_for(Some(&on_p2(h)));
            let tier = game(&format!("r290-{difficulty}"), Some(on_p2(h)), Some(decks.clone()));
            let stored = serde_json::to_value(tier.players.p2.handicap).expect("a handicap serialises");
            assert_eq!(keys_sorted(&stored), five_fields_sorted(), "{difficulty}");
            assert_eq!(tier.players.p2.handicap, Some(h), "{difficulty}");
            assert_eq!(tier.players.p2.hero.health, HERO_HEALTH, "{difficulty}");

            let explicit = game(
                &format!("r290-{difficulty}"),
                Some(on_p2(Handicap {
                    hero_health: Some(HERO_HEALTH),
                    ..h
                })),
                Some(decks),
            );
            let stored = serde_json::to_value(explicit.players.p2.handicap).expect("a handicap serialises");
            assert_eq!(keys_sorted(&stored), five_fields_sorted(), "{difficulty}");
            assert_eq!(explicit.players.p2.handicap, Some(h), "{difficulty}");
            assert_eq!(hash_state(&explicit), hash_state(&tier), "{difficulty}");
            assert_eq!(
                hash_state(&begin_game(&explicit).state),
                hash_state(&begin_game(&tier).state),
                "{difficulty}"
            );
        }
    }

    #[test]
    fn r290_a_hero_health_other_than_30_is_stored_beside_the_other_fields_even_on_a_handicap_that_is_otherwise_a_humans()
     {
        let decks = decks_for(None);
        let plain = game("r290-only-health", None, Some(decks.clone()));
        let only_health = Handicap {
            hero_health: Some(tutorial_health()),
            ..HUMAN_HANDICAP
        };
        let state = game("r290-only-health", Some(on_p2(only_health)), Some(decks));
        assert_eq!(state.players.p2.handicap, Some(only_health));
        assert_eq!(state.players.p2.hero.health, tutorial_health());
        assert_ne!(hash_state(&state), hash_state(&plain));

        let medium_at_25 = Handicap {
            hero_health: Some(25),
            ..AI_DIFFICULTY.medium
        };
        let medium = game("r290-medium-25", Some(on_p2(medium_at_25)), None);
        assert_eq!(medium.players.p2.handicap, Some(medium_at_25));
        assert_eq!(medium.players.p2.hero.health, 25);
    }

    #[test]
    fn r290_validate_handicap_refuses_a_hero_health_that_is_not_a_positive_integer_and_accepts_1_and_20() {
        // 0 and -1 are integers the type holds: refused by validate_handicap, with TS's message.
        for hero_health in [0, -1] {
            let result = state::validate_handicap(
                &Handicap {
                    hero_health: Some(hero_health),
                    ..AI_TUTORIAL
                },
                "p2",
            );
            assert!(
                result.is_err_and(|e| e
                    .message
                    .contains("p2: handicap heroHealth must be a positive integer (R290)")),
                "{hero_health}"
            );
        }
        // 2.5 and "20" are not a count at all: the typed handicap refuses them where it is read. (NaN,
        // Infinity and null: see the spec-gaps file.)
        for hero_health in [json!(2.5), json!("20")] {
            let label = hero_health.to_string();
            assert!(
                handicap_refused(with_field(&AI_TUTORIAL, "heroHealth", hero_health), "p2"),
                "{label}"
            );
        }
        assert!(
            state::validate_handicap(
                &Handicap {
                    hero_health: Some(1),
                    ..AI_TUTORIAL
                },
                "p2"
            )
            .is_ok()
        );
        assert!(
            state::validate_handicap(
                &Handicap {
                    hero_health: Some(20),
                    ..AI_TUTORIAL
                },
                "p2"
            )
            .is_ok()
        );
        assert!(state::validate_handicap(&without_hero_health(AI_TUTORIAL), "p2").is_ok());
    }

    #[test]
    fn r290_create_game_refuses_an_invalid_hero_health_before_it_looks_at_the_deck() {
        register_all();
        for hero_health in [0, -1] {
            let message = create_refusal(options(
                "r290-bad-health",
                // A 20-card p2 deck is wrong for AI_TUTORIAL too; the handicap is named first (R180).
                (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 1)),
                on_p2(Handicap {
                    hero_health: Some(hero_health),
                    ..AI_TUTORIAL
                }),
            ));
            assert!(
                message
                    .is_some_and(|m| m.contains("p2: handicap heroHealth must be a positive integer (R290)")),
                "{hero_health}"
            );
        }
        // 2.5: the typed options cannot hold it, so no game is made.
        assert!(
            create_refusal_json(json!({
                "seed": "r290-bad-health",
                "decks": [vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 1)],
                "handicaps": { "p2": with_field(&AI_TUTORIAL, "heroHealth", json!(2.5)) },
            }))
            .is_some()
        );
        let message = create_refusal(options(
            "r290-bad-health-p1",
            (vanilla_deck(AI_TUTORIAL.deck_size, 1), vanilla_deck(DECK_SIZE, 1)),
            on_p1(Handicap {
                hero_health: Some(0),
                ..AI_TUTORIAL
            }),
        ));
        assert!(message.is_some_and(|m| m.contains("p1: handicap heroHealth")));
    }

    #[test]
    fn r290_max_mana_for_a_tutorial_seat_is_min_turns_started_3_it_never_exceeds_3_and_the_humans_is_s2_3s() {
        let mut state = game("r290-mana-unit", Some(on_p2(AI_TUTORIAL)), None);
        for turns in 0..=12 {
            state.players.p2.turns_started = turns;
            state.players.p1.turns_started = turns;
            assert_eq!(
                mana::max_mana_for(&state.players.p2),
                turns.min(AI_TUTORIAL.mana_cap),
                "tutorial after {turns} turns"
            );
            assert!(
                mana::max_mana_for(&state.players.p2) <= AI_TUTORIAL.mana_cap,
                "tutorial after {turns} turns"
            );
            assert_eq!(
                mana::max_mana_for(&state.players.p1),
                turns.min(MAX_MANA),
                "human after {turns} turns"
            );
        }
    }

    #[test]
    fn r290_a_tutorial_seat_refreshes_to_1_2_3_3_on_its_first_four_turns_as_p2_or_p1_while_the_human_reaches_4()
     {
        let as_p2 = maxes_by_turn("r290-mana", Some(on_p2(AI_TUTORIAL)), 12);
        assert_eq!(as_p2.p2, [1, 2, 3, 3, 3, 3]);
        assert_eq!(as_p2.p1, [1, 2, 3, 4, 4, 4]);

        let as_p1 = maxes_by_turn("r290-mana-p1", Some(on_p1(AI_TUTORIAL)), 12);
        assert_eq!(as_p1.p1, [1, 2, 3, 3, 3, 3]);
        assert_eq!(as_p1.p2, [1, 2, 3, 4, 4, 4]);

        let fourth = advance_to(
            started("r290-mana-current", Some(on_p2(AI_TUTORIAL)), None),
            PlayerId::P2,
            4,
        );
        assert_eq!(
            (fourth.players.p2.mana.max, fourth.players.p2.mana.current),
            (3, 3)
        );
    }

    #[test]
    fn r290_fold_with_ai_tutorial_reproduces_a_tutorial_random_policy_game_whose_tutorial_seat_never_had_more_than_3_max_mana()
     {
        let cases: Vec<(&str, Handicaps, PlayerId)> = vec![
            ("r290-fold-p2", on_p2(AI_TUTORIAL), PlayerId::P2),
            ("r290-fold-p1", on_p1(AI_TUTORIAL), PlayerId::P1),
        ];
        for (seed, handicaps, seat) in cases {
            let mut highest = 0;
            let mut observe = |state: &GameState| highest = highest.max(state.players[seat].mana.max);
            let live = play_random(seed, handicaps.clone(), Some(&mut observe));
            assert_eq!(
                highest, AI_TUTORIAL.mana_cap,
                "{seed}: the tutorial seat reached its cap"
            );
            let seat_deck = if seat == PlayerId::P1 {
                &live.decks.0
            } else {
                &live.decks.1
            };
            assert_eq!(seat_deck.len(), AI_TUTORIAL.deck_size as usize, "{seed}");

            register_all();
            let replayed = fold_with(seed, &live, Some(handicaps));
            assert!(replayed.errors.is_empty(), "{seed}");
            assert_eq!(hash_state(&replayed.state), hash_state(&live.state), "{seed}");
            assert_eq!(replayed.state.result, live.state.result, "{seed}");
        }
    }

    #[test]
    fn r290_the_same_fold_without_the_handicap_throws_on_the_12_card_deck_and_without_hero_health_it_does_not_reproduce_the_game()
     {
        let seed = "r290-fold-missing";
        let live = play_random(seed, on_p2(AI_TUTORIAL), None);
        register_all();
        let message = refusal(|| {
            fold_with(seed, &live, None);
        });
        assert!(message.is_some_and(|m| m.contains("p2: deck must hold exactly 20")));

        let forgot = fold_with(seed, &live, Some(on_p2(without_hero_health(AI_TUTORIAL))));
        let same = forgot.errors.is_empty() && hash_state(&forgot.state) == hash_state(&live.state);
        assert!(!same);
    }

    #[test]
    fn r290_a_tutorial_hero_at_20_dies_at_0_like_any_hero_a_20_point_fatigue_hit_ends_the_game_a_19_point_one_leaves_it_at_1()
     {
        // p2's library is empty, so its turn-start draw is a fatigue step of fatigueCount + 1 (R183's
        // fatigue test reads the same rule): fatigueCount 19 makes the hit 20.
        let mut lethal = started("r290-death", Some(on_p2(AI_TUTORIAL)), None);
        lethal.players.p2.library = vec![];
        lethal.players.p2.fatigue_count = tutorial_health() - 1;
        let ended = step(&lethal, input(PlayerId::P1, ActionBody::EndTurn));
        assert_eq!(hero_hits(&ended.events, "hero-p2"), [tutorial_health()]);
        assert_eq!(ended.state.players.p2.hero.health, 0);
        assert_eq!(
            ended.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath
            })
        );
        assert_eq!(
            ended
                .events
                .iter()
                .filter(|e| matches!(e, GameEvent::GameOver { .. }))
                .count(),
            1
        );

        let mut survives = started("r290-death", Some(on_p2(AI_TUTORIAL)), None);
        survives.players.p2.library = vec![];
        survives.players.p2.fatigue_count = tutorial_health() - 2;
        let alive = pass_turn(&survives).state;
        assert_eq!(alive.players.p2.hero.health, 1);
        assert!(alive.result.is_none());
        assert_eq!(alive.active, PlayerId::P2);

        // The same 20-point hit on a human's hero leaves it at 10: the tutorial hero's 20 is what ended it.
        let mut human = started("r290-death", None, None);
        human.players.p2.library = vec![];
        human.players.p2.fatigue_count = tutorial_health() - 1;
        let human_after = pass_turn(&human).state;
        assert_eq!(
            human_after.players.p2.hero.health,
            HERO_HEALTH - tutorial_health()
        );
        assert!(human_after.result.is_none());
    }

    #[test]
    fn r290_20_is_where_the_tutorial_hero_starts_not_a_cap_a_heal_takes_it_past_20_and_a_heal_up_to_30_lifts_it_to_30()
     {
        let mut state = started("r290-heal", Some(on_p2(AI_TUTORIAL)), None);
        // TS's `{ state, events: [] }`; the Rust sink carries an rng too, which neither heal draws from.
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        // §3: a hero has no maximum health, so nothing holds the tutorial hero at its starting 20.
        assert_eq!(damage::heal_hero(&mut sink, PlayerId::P2, 5), 5);
        assert_eq!(sink.state.players.p2.hero.health, tutorial_health() + 5);
        // "Heal up to N" (#53 Reno's 30) raises the hero to N whatever it started at.
        assert_eq!(
            damage::heal_hero_up_to(&mut sink, PlayerId::P2, HERO_HEALTH),
            HERO_HEALTH - tutorial_health() - 5
        );
        assert_eq!(sink.state.players.p2.hero.health, HERO_HEALTH);
        // The stored handicap is untouched by either: its heroHealth records where the hero started.
        assert_eq!(sink.state.players.p2.handicap, Some(AI_TUTORIAL));
    }
}
