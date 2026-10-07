//! The one seam between the server and the engine (SPEC §9.3, CLAUDE.md rule 7). Port of
//! `apps/server/src/match/engine.ts` and `apps/server/src/match/engine.real.ts`.
//!
//! The engine is pure and server-agnostic: it never reads a clock, opens a socket or touches
//! Postgres. These functions are how the actor reaches it — `create_game` / `begin_game` / `reduce`
//! to advance the match, `legal_actions` for the client's greying-out, `view_for` for the only thing
//! a socket is allowed to carry (§10.8), and `fold` to rebuild a crashed actor from
//! `(seed, decks, log)` (§9.5).
//!
//! `EngineState` holds both hands and both libraries, so nothing outside the actor inspects it.
//! Everything the rest of the server needs about the state comes back through `view_for` (per
//! player) or `snapshot` (public bookkeeping the clock and the results writer need).
//!
//! SURFACE §11.3: plain functions that call `jackioh_engine` directly. TS's `EnginePort` type, its
//! test injection (`setEnginePort`), the dynamic import that kept the server bootable without an
//! engine (`loadEnginePort`), `REQUIRED_ENGINE_EXPORTS` and `EngineUnavailableError` are not
//! ported: the server links the engine, so an engine that is missing an export does not compile.
//! Tests install their cards with the testkit's thread-local override instead (SURFACE §8).
//!
//! WHY `register_all()` IS HERE. `create_game` looks its card definitions up in the engine's
//! registered catalog, and `jackioh_cards` is the one crate that owns the catalog and the scripts
//! (SPEC §10.9, BUILD M4-T2). Without this call `registered_catalog()` is empty and every real match
//! panics on the first card of the first deck. It is idempotent (a `OnceLock`), so calling it on
//! every entry point that reads the catalog costs nothing. The client does the same in its own
//! composition root; neither assembles a catalog of its own, which would be a second source of card
//! data. Under the testkit the thread-local override is consulted first, so a test's cards win.
//!
//! Nothing here decides a rule. It renames engine functions and projects the public bookkeeping
//! the clock needs.
//!
//! All Random's deck (R258) is dealt here too, by `jackioh_ai::build_ai_deck` — the same weighted
//! draw `apps/web/src/practice/core.ts` deals a human who asks for a random deck, with nothing
//! banned. It reads the registered catalog, so it belongs behind the same `register_all()` as the
//! match itself, and the ai crate is pure and seeded like the engine: one seed, one deck, in any
//! process.

use serde::{Deserialize, Serialize};

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::{
    Action, ActionBody, CreateGameOptions, DECK_SIZE, GameResult, GameState, GameSummary, LastBoardEntry,
    Phase, PlayerId, PlayerView, Rng,
};

pub use jackioh_engine::{FoldArgs, FoldResult, ReduceResult};

/// TS's opaque `EngineState` brand: the engine's own state, held only by the actor.
pub type EngineState = GameState;

/// R417: each seat's last board in seat order, a setup input beside the decks (C+ #29).
pub type LastBoards = (Vec<LastBoardEntry>, Vec<LastBoardEntry>);

/// TS `CreateGameArgs`: `{ seed, decks, catalog?, lastBoards?, glitchBoards?, dealt? }`, the
/// engine's own `CreateGameOptions`.
pub type CreateGameArgs = CreateGameOptions;

/// The public bookkeeping the clock (`actor/clock.rs`) and the results writer (`api/results.rs`)
/// need. Nothing here is hidden information: both clients already see all of it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchSnapshot {
    /// Player-turn counter (§2.5).
    pub turn: i32,
    pub active: PlayerId,
    /// Who owes the open prompt an answer, or null (§10.6).
    pub pending_for: Option<PlayerId>,
    /// R265: while both mulligans are open (phase `mulligan`, `pending_for` null), the seats that
    /// still owe theirs, in seat order; empty otherwise. Public (R266: that a seat is ready is
    /// public, what it kept is not), and the mulligan clock (R268) and its expiry read nothing else.
    pub mulligan_owed: Vec<PlayerId>,
    pub phase: Phase,
    pub result: Option<GameResult>,
    /// R677: whether a Glitch has left the accounts holding each other's seat (an odd number of
    /// swaps, the engine's `seats_swapped`). Public: the `glitched` event announced every swap. The
    /// account that began the match in p1 plays p2 while this is true, and the other way round.
    pub seats_swapped: bool,
}

fn registered() {
    jackioh_cards::register_all();
}

/// The `CreateGameArgs` a match is created with: no catalog (the registered one) and no handicaps
/// (a server match is Easy both, R180), the rest as the match row froze it.
pub fn create_game_args(
    seed: &str,
    decks: &(Vec<String>, Vec<String>),
    last_boards: Option<LastBoards>,
    glitch_boards: Option<LastBoards>,
    dealt: Option<Vec<PlayerId>>,
) -> CreateGameArgs {
    CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.clone(),
        catalog: None,
        handicaps: None,
        dealt,
        last_boards,
        glitch_boards,
    }
}

/// The `FoldArgs` a match folds with: the same setup `create_game_args` builds, plus its log.
pub fn fold_args(
    seed: &str,
    decks: &(Vec<String>, Vec<String>),
    log: Vec<Action>,
    last_boards: Option<LastBoards>,
    glitch_boards: Option<LastBoards>,
    dealt: Option<Vec<PlayerId>>,
) -> FoldArgs {
    FoldArgs {
        seed: seed.to_string(),
        decks: decks.clone(),
        log,
        catalog: None,
        handicaps: None,
        dealt,
        last_boards,
        glitch_boards,
    }
}

/// TS `createGame`. Panics, as TS threw, on a deck the catalog refuses.
pub fn create_game(args: &CreateGameArgs) -> EngineState {
    registered();
    jackioh_engine::create_game(args)
}

pub fn begin_game(state: &EngineState) -> ReduceResult {
    registered();
    jackioh_engine::begin_game(state)
}

pub fn reduce(state: &EngineState, action: &Action) -> ReduceResult {
    registered();
    jackioh_engine::reduce(state, action)
}

pub fn legal_actions(state: &EngineState, player: PlayerId) -> Vec<ActionBody> {
    jackioh_engine::legal_actions(state, player)
}

/// SPEC §10.8: the only window a player gets onto the match.
pub fn view_for(state: &EngineState, player: PlayerId) -> PlayerView {
    jackioh_engine::view_for(state, player)
}

/// §9.5: a crashed actor rebuilds its state by folding `(seed, log)`.
pub fn fold(args: &FoldArgs) -> FoldResult {
    registered();
    jackioh_engine::fold(args)
}

pub fn hash_state(state: &EngineState) -> String {
    jackioh_engine::hash_state(state)
}

/// TS `snapshot`: the public bookkeeping, read off the state's own fields and the engine's own
/// answers to the two questions that are not a field.
pub fn snapshot(state: &EngineState) -> MatchSnapshot {
    MatchSnapshot {
        turn: state.turn,
        active: state.active,
        pending_for: state.pending.as_ref().map(|pending| pending.player_id),
        // R265: the engine's own answer to "who still owes a mulligan", not a read of its fields.
        mulligan_owed: jackioh_engine::mulligan_owed(state),
        phase: state.phase,
        result: state.result,
        // R677: the engine's own answer to "do the accounts hold each other's seat".
        seats_swapped: jackioh_engine::seats_swapped(state),
    }
}

/// SPEC §11 R258: an All Random deck of card ids in library order, from the game's own weighted
/// random deck-builder with nothing banned, seeded so the same seed deals the same deck in any
/// process. Callers seed it `"{seed}:p1-deck"` and `"{seed}:p2-deck"`. It lives here because the
/// deck-builder needs the registered catalog, and this module is the one path to it.
pub fn deal_random_deck(seed: &str) -> Vec<String> {
    registered();
    // R258: `banned: []` is practice's "random deck for a human" (no shadow-ban: R186's list shapes
    // the AI's own decks, not a player's), and `DECK_SIZE` is the size L2 asks of every deck.
    build_ai_deck(
        &mut Rng::new(seed, 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(vec![]),
            ..AiDeckOptions::default()
        },
    )
}

/// R417, R565: each seat's last board from a finished game, seat order — every card on the field,
/// both sides, minus what that seat could not read (the other side's face-down cards, R33).
pub fn last_boards(state: &EngineState) -> LastBoards {
    (
        jackioh_engine::last_board_for(state, PlayerId::P1),
        jackioh_engine::last_board_for(state, PlayerId::P2),
    )
}

/// SPEC §11 R376: a finished game's record for the card statistics — each seat's opening hand,
/// draws and plays, and the ending — read off `(seed, decks, log)`; `None` when the log leaves the
/// game without a result. Folded like `fold`.
pub fn summarize_game(args: &FoldArgs) -> Option<GameSummary> {
    registered();
    jackioh_engine::summarize_game(args)
}
