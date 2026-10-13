//! Helpers shared by the AI tests (docs/polish/3-ai.md §Tests). Not a test file itself.
//!
//! `register_cards` loads the real catalog and card scripts: every helper below that needs them
//! calls it first, and a test that builds a `scenario` itself calls it before. Real mid-game states
//! come from `create_game`/`begin_game` plus §10.7's random policy.
//!
//! Scenario ids start at `c1` and follow the setup literal. `redact` also hides a card in the
//! seat's OWN library whose id was minted for the opponent's opening deck (R73), and `create_game`
//! mints p1's deck first, so a scenario seen from p2 would have its own low-numbered library cards
//! read as p1's. Every scenario-based AI test therefore puts the AI in p1's seat and keeps the
//! scenario under 20 instances; states for p2's seat come from real `create_game` games.

#![allow(dead_code)]

use std::borrow::Borrow;
use std::cell::Cell;

use jackioh_ai::{AI_BUDGET, AiOptions, AiTurnResult, SearchBudget, action_key, play_ai_turn};
use jackioh_engine::testkit::{
    Action, ActionBody, ActionType, CardInstance, CatalogQueryArgs, CreateGameOptions, DECK_SIZE, GameState,
    PLAYER_IDS, Phase, PlayerId, Value, begin_game, create_game, create_rng, json, json_as, legal_actions,
    opponent_of, query, reduce, seat_to_act, subsystems,
};

pub use jackioh_engine::testkit::scenario;

/// The real catalog and card scripts, once per process.
pub fn register_cards() {
    jackioh_cards::register_all();
}

/// The AI's seat in every scenario-built test (see the header).
pub const AI: PlayerId = PlayerId::P1;
/// The human's seat in every scenario-built test.
pub const HUMAN: PlayerId = PlayerId::P2;

// Card lookups

/// Every non-token id of every set, sorted: the pool AI decks and determinizations draw from (R380).
pub fn ai_pool() -> Vec<String> {
    register_cards();
    let mut ids: Vec<String> = query(&CatalogQueryArgs::default())
        .iter()
        .map(|def| def.id.clone())
        .collect();
    ids.sort();
    ids
}

/// Every Trap and Field Trap id of every set: the only defs a face-down sample may take.
pub fn trap_pool() -> Vec<String> {
    register_cards();
    let mut ids: Vec<String> = query(&json_as::<CatalogQueryArgs>(
        json!({ "type": ["Trap", "Field Trap"] }),
    ))
    .iter()
    .map(|def| def.id.clone())
    .collect();
    ids.sort();
    ids
}

/// Every instance in every zone of both players, Stack piles included.
pub fn every_card(state: &GameState) -> Vec<&CardInstance> {
    PLAYER_IDS
        .into_iter()
        .flat_map(move |player| {
            let side = &state.players[player];
            side.hand
                .iter()
                .chain(side.library.iter())
                .chain(side.graveyard.iter())
                .chain(side.exile.iter())
                .chain(side.resolving.iter())
                .chain(side.units.iter().flatten().flatten())
                .chain(side.backrow.iter().flatten())
        })
        .collect()
}

pub fn card_by_id<'a>(state: &'a GameState, id: &str) -> Option<&'a CardInstance> {
    every_card(state).into_iter().find(|card| card.id == id)
}

/// Whether `player`'s unit zones hold a card of this def (a Stack pile included).
pub fn on_field(state: &GameState, player: PlayerId, def_id: &str) -> bool {
    state.players[player]
        .units
        .iter()
        .any(|pile| pile.iter().flatten().any(|card| card.def_id == def_id))
}

pub fn in_graveyard(state: &GameState, player: PlayerId, def_id: &str) -> bool {
    state.players[player]
        .graveyard
        .iter()
        .any(|card| card.def_id == def_id)
}

// Actions

/// Whether `action` is one of `legal_actions(state, seat)`, compared by `action_key`.
pub fn is_legal(state: &GameState, seat: PlayerId, action: impl Borrow<ActionBody>) -> bool {
    let key = action_key(action.borrow());
    legal_actions(state, seat)
        .iter()
        .any(|legal| action_key(legal) == key)
}

thread_local! {
    /// One count per test thread, so every default nonce is fresh.
    static NONCE_COUNTER: Cell<u32> = const { Cell::new(0) };
}

/// One action through `reduce`, panicking with the engine's refusal.
pub fn act(state: &GameState, player_id: PlayerId, body: impl Borrow<ActionBody>) -> GameState {
    let count = NONCE_COUNTER.with(|counter| {
        counter.set(counter.get() + 1);
        counter.get()
    });
    act_with_nonce(state, player_id, body, &format!("support-{count}"))
}

/// `act` with the nonce given.
pub fn act_with_nonce(
    state: &GameState,
    player_id: PlayerId,
    body: impl Borrow<ActionBody>,
    nonce: &str,
) -> GameState {
    let body: &ActionBody = body.borrow();
    let result = reduce(state, &Action::new(body.clone(), player_id, nonce));
    if let Some(error) = result.error {
        panic!("{} for {} refused: {}", body.action_type(), player_id, error);
    }
    result.state
}

pub fn clone(state: &GameState) -> GameState {
    state.clone()
}

// Puzzles (P1–P14)

pub struct PuzzleRun {
    /// The scenario's state before the AI moved.
    pub start: GameState,
    /// `play_ai_turn`'s whole result.
    pub turn: AiTurnResult,
    /// The state after the AI's turn.
    pub end: GameState,
}

/// The puzzle runner: `scenario({ active: "p1", turn: 9, … })` with the AI as p1, then one AI turn
/// through `play_ai_turn` at `AI_BUDGET` (the budget the browser plays at). `setup` is the scenario
/// literal; its keys win over the three defaults.
pub fn run_puzzle(name: &str, setup: Value) -> PuzzleRun {
    run_puzzle_with(name, setup, AI_BUDGET)
}

/// `run_puzzle` with the budget given.
pub fn run_puzzle_with(name: &str, setup: Value, budget: SearchBudget) -> PuzzleRun {
    register_cards();
    let mut options = json!({ "seed": format!("puzzle-{name}"), "active": AI, "turn": 9 });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), setup) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    let s = scenario(options);
    let start = s.state().clone();
    let mut ai = AiOptions::with_budget(create_rng(&format!("puzzle:{name}"), 0), budget);
    let turn = play_ai_turn(&start, AI, &mut ai);
    let end = turn.state.clone();
    PuzzleRun { start, turn, end }
}

/// A readable trace of a turn's decisions, for failure messages.
pub fn trace(turn: &AiTurnResult) -> String {
    turn.decisions
        .iter()
        .map(|decision| {
            format!(
                "{}:{}",
                decision.reason,
                serde_json::to_string(&decision.action).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// P8–P10's scripted reply: `attacker` attacks with everything it has (the enemy hero first) and
/// answers any prompt with its first legal answer. It stops once its starting turn is over: R82
/// auto-ends a turn with nothing left to do, so the defender's empty turn can end in the same
/// action and give the attacker a later turn's main phase (B18).
pub fn all_out_attack(start: &GameState, attacker: PlayerId) -> GameState {
    let mut state = start.clone();
    let hero_target = format!("hero-{}", opponent_of(attacker));
    for step in 0..200 {
        if state.result.is_some() {
            return state;
        }
        if state.turn != start.turn {
            return state;
        }
        if let Some(pending) = &state.pending {
            let holder = pending.player_id;
            let Some(answer) = legal_actions(&state, holder).into_iter().next() else {
                return state;
            };
            state = act_with_nonce(&state, holder, answer, &format!("all-out-{step}"));
            continue;
        }
        if state.active != attacker || state.phase != Phase::Main {
            return state;
        }
        let attacks: Vec<ActionBody> = legal_actions(&state, attacker)
            .into_iter()
            .filter(|action| action.action_type() == ActionType::Attack)
            .collect();
        let pick = attacks
            .iter()
            .find(
                |action| matches!(action, ActionBody::Attack { target_id, .. } if *target_id == hero_target),
            )
            .or_else(|| attacks.first())
            .cloned();
        let Some(pick) = pick else {
            return state;
        };
        state = act_with_nonce(&state, attacker, pick, &format!("all-out-{step}"));
    }
    panic!("the scripted all-out attack did not finish in 200 steps");
}

// Real games

/// Two distinct-card decks from one seeded shuffle (the fuzz suite's construction), of Core cards so
/// that the fixed deals the tests were written against stay the same.
pub fn random_decks(seed: &str) -> (Vec<String>, Vec<String>) {
    random_decks_sized(seed, (DECK_SIZE, DECK_SIZE))
}

/// `random_decks` with the deck sizes given.
pub fn random_decks_sized(seed: &str, sizes: (i32, i32)) -> (Vec<String>, Vec<String>) {
    register_cards();
    let mut core: Vec<String> = query(&json_as::<CatalogQueryArgs>(json!({ "set": "Core" })))
        .iter()
        .map(|def| def.id.clone())
        .collect();
    core.sort();
    let shuffled = create_rng(&format!("ai-test-decks:{seed}"), 0).shuffle(&core);
    let first = sizes.0 as usize;
    let second = sizes.1 as usize;
    (
        shuffled[..first].to_vec(),
        shuffled[first..first + second].to_vec(),
    )
}

/// A freshly dealt game: `begin_game` done, p1's mulligan open.
pub fn dealt_game(seed: &str) -> GameState {
    register_cards();
    begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: random_decks(seed),
        ..CreateGameOptions::default()
    }))
    .state
}

/// A game played by §10.7's random policy (`subsystems::choose_action`), returning the state after
/// every `every`-th action (the dealt state first). Stops at the result or at `max_actions`.
pub fn random_policy_states(seed: &str, every: usize, max_actions: usize) -> Vec<GameState> {
    let mut state = dealt_game(seed);
    let mut policy = create_rng(&format!("ai-test-policy:{seed}"), 0);
    let mut states: Vec<GameState> = vec![state.clone()];
    let mut n = 0;
    while state.result.is_none() && n < max_actions {
        let player = seat_to_act(&state).expect("a live game has a seat to act");
        let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else {
            break;
        };
        let kind = chosen.action_type();
        let result = reduce(&state, &Action::new(chosen, player, format!("rp-{n}")));
        if let Some(error) = result.error {
            panic!("{seed}: {kind} refused: {error}");
        }
        state = result.state;
        if (n + 1) % every == 0 {
            states.push(state.clone());
        }
        n += 1;
    }
    states
}
