//! Port of `packages/engine/test/fixtures/promptHarness.ts`.
//!
//! Helpers the prompts-and-movement tests share (docs/classic-sets.md B5 E13, E16–E18, E26): a board
//! with the fixtures registered, a Spell resolving, an answer by option key, a round trip, and a short
//! replayable game whose opening hands hold the fixture under test (Quickdraw, R225).
//!
//! TS handed back the sink it ran (`{ state, events, rng }`); a Rust sink borrows the state, so
//! `cast_now` and `answer_keys` answer what the sink held besides the state — its events and its rng —
//! and the caller reads the state it passed in.

use std::cell::Cell;

use jackioh_engine::testkit::*;
use serde::Serialize;

use super::catalog::vanilla_deck;
use super::harness::{new_game, setup_catalog};
use super::prompts::register_prompt_fixtures;

/// p1's main phase on turn 3 with 4 mana, the fixtures registered (after `newGame`'s own).
pub fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_prompt_fixtures();
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state.players.p2.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

/// A card of `defId` resolving for `player` (§10.5 step 4), where a Spell's Cry runs. TS's defaults:
/// `player = "p1"`, `radiant = false`.
pub fn resolving_card(state: &mut GameState, def_id: &str, player: PlayerId, radiant: bool) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Resolving { player });
    card.radiant = radiant;
    state.players[player].resolving.push(card.clone());
    card
}

/// What the sink a helper ran held besides the state (TS returned the sink itself). It derefs to its
/// events, the one part of the sink TS's callers read (`sink.events`).
pub struct SinkResult {
    pub events: Vec<GameEvent>,
    pub rng: Rng,
}

impl std::ops::Deref for SinkResult {
    type Target = Vec<GameEvent>;

    fn deref(&self) -> &Vec<GameEvent> {
        &self.events
    }
}

/// Run a resolving card's Cry the resumable way and settle, as the pipeline does; returns the sink.
pub fn cast_now(state: &mut GameState, def_id: &str, player: PlayerId, radiant: bool) -> SinkResult {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    let card = resolving_card(state, def_id, player, radiant);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        prompts::run_hook_resumable(
            &mut sink,
            &card,
            "cry",
            prompts::HookResumableOptions {
                controller: Some(player),
                ..Default::default()
            },
        );
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    SinkResult { events, rng }
}

/// `value`, or a panic naming what was expected (TS threw `expected …`).
pub fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(found) => found,
        None => panic!("expected {what}"),
    }
}

/// What `answer_keys` answers: the sink's events and rng, and the refusal (`null` when accepted).
pub struct AnswerResult {
    pub events: Vec<GameEvent>,
    pub rng: Rng,
    pub error: Option<String>,
}

/// Answer the open prompt by option key, as a client sends back what it was offered.
pub fn answer_keys(state: &mut GameState, keys: &[&str]) -> AnswerResult {
    let pending = must(state.pending.clone(), "an open prompt");
    let selection: Vec<Selection> = keys
        .iter()
        .map(|key| {
            must(pending.options.iter().find(|option| option.key == *key), &format!("option {key}"))
                .selection
                .clone()
        })
        .collect();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    let error = {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let answered = prompts::answer_prompt(
            &mut sink,
            &prompts::AnswerInput {
                player_id: pending.player_id,
                choice_id: pending.id.clone(),
                selection,
            },
        );
        match answered {
            Ok(_) => {
                settle(&mut sink, SettleOptions::default());
                None
            }
            Err(error) => Some(error.message),
        }
    };
    state.rng_cursor = rng.cursor();
    AnswerResult { events, rng, error }
}

/// The open prompt, asserted to be of this kind and held by this player.
pub fn open_as(state: &GameState, kind: PromptKind, player: PlayerId) -> PendingChoice {
    let pending = must(state.pending.clone(), &format!("a {kind} prompt"));
    assert_eq!(pending.kind, kind);
    assert_eq!(pending.player_id, player);
    pending
}

/// §9.3: a paused state survives JSON, and the copy answers exactly as the original does.
pub fn round_trip(state: &GameState) -> GameState {
    let text = serde_json::to_string(state).expect("a state serialises");
    serde_json::from_str(&text).expect("a state parses back")
}

pub fn event_types(events: &[GameEvent]) -> Vec<String> {
    events.iter().map(|event| event.event_type().as_str().to_string()).collect()
}

thread_local! {
    /// TS's module `let nonce`: one counter per test thread, so every action a test sends is fresh.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

/// One action through `reduce`, logged, erroring loudly. `body` is TS's `ActionInput` (an
/// `ActionInput` or its JSON literal).
pub fn act(state: &GameState, body: impl Serialize, log: Option<&mut Vec<Action>>) -> GameState {
    let nonce = NONCE.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let mut fields = serde_json::to_value(body).expect("an action input serialises");
    if let Some(object) = fields.as_object_mut() {
        object.insert("nonce".to_string(), json!(format!("pm{nonce}")));
    }
    let action: Action = json_as(fields);
    let result = reduce(state, &action);
    if let Some(error) = result.error {
        panic!("{} refused: {error}", action.action_type());
    }
    if let Some(log) = log {
        log.push(action);
    }
    result.state
}

/// What `replayable` answers: TS's `{ state, log, decks }`.
pub struct Replayable {
    pub state: GameState,
    pub log: Vec<Action>,
    pub decks: (Vec<String>, Vec<String>),
}

/// A replayable game (§9.2, §9.3): each deck is vanilla fixtures plus the named Quickdraw cards, so the
/// opening hands hold them whatever the shuffle; both mulligans keep everything, and it is p1's first
/// main phase. The log and decks go to `fold`, which must rebuild the very same state. TS's default:
/// `p2Cards = []`.
pub fn replayable(seed: &str, p1_cards: &[String], p2_cards: &[String]) -> Replayable {
    setup_catalog();
    register_prompt_fixtures();
    let mut first = vanilla_deck(DECK_SIZE - p1_cards.len() as i32, 1);
    first.extend(p1_cards.iter().cloned());
    let mut second = vanilla_deck(DECK_SIZE - p2_cards.len() as i32, 21);
    second.extend(p2_cards.iter().cloned());
    let decks = (first, second);
    let mut log: Vec<Action> = Vec::new();
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..CreateGameOptions::default()
    }))
    .state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "playerId": "p1", "keep": keep }),
        Some(&mut log),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "playerId": "p2", "keep": keep }),
        Some(&mut log),
    );
    Replayable { state, log, decks }
}

/// §9.2: fold the log from scratch and compare hashes with the live state.
pub fn expect_replays(seed: &str, decks: &(Vec<String>, Vec<String>), log: &[Action], live: &GameState) {
    let folded = fold(&json_as::<FoldArgs>(json!({ "seed": seed, "decks": decks, "log": log })));
    assert!(folded.errors.is_empty(), "the fold refused nothing");
    assert_eq!(hash_state(&folded.state), hash_state(live));
}

/// The first card in `player`'s hand of this definition.
pub fn hand_card(state: &GameState, player: PlayerId, def_id: &str) -> CardInstance {
    must(
        state.players[player].hand.iter().find(|card| card.def_id == def_id),
        &format!("{def_id} in {player}'s hand"),
    )
    .clone()
}
