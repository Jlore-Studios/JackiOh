//! The one seam between the server and the engine (SPEC §9.3, CLAUDE.md rule 7).
//!
//! Calls `jackioh_engine` directly (SURFACE §11.3): `create_game`, `reduce`, `view_for` (§10.8),
//! `fold` (§9.5) and replays (R768). Tests override cards via the testkit (SURFACE §8).
//!
//! `register_all()` initializes the engine's registered catalog from `jackioh_cards` (SPEC §10.9,
//! BUILD M4-T2); it is idempotent.
//!
//! All Random decks (R258, R1372) and practice AI decks (#1303) are dealt here via `jackioh_ai`.
//! Every catalog-reading function runs under the match's `Pinned` catalog (R388, R767000).

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde::{Deserialize, Serialize};

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::{
    Action, ActionBody, CreateGameOptions, DECK_SIZE, GameResult, GameState, GameSummary, Handicap,
    LastBoardEntry, LeanPick, Phase, PlayerId, PlayerView, ReplayCheckpoints, ReplayOpen, ReplayPage,
    ReplayRecord, ReplayRefusal, Rng, SetName, ShippedCatalog,
};

pub use jackioh_engine::{FoldArgs, FoldResult, ReduceResult};

use crate::actor::registry::panic_text;

/// The engine's own state, held only by the actor.
pub type EngineState = GameState;

/// R417: each seat's last board in seat order, a setup input beside the decks (C+ #29).
pub type LastBoards = (Vec<LastBoardEntry>, Vec<LastBoardEntry>);

/// The engine's own `CreateGameOptions`.
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
    /// R1563006: the question of a sequence open now, the sequence clock's input (public).
    pub sequence: Option<jackioh_engine::prompts::SequenceQuestion>,
}

fn registered() {
    jackioh_cards::register_all();
}

/// #1035: what a blocking job's thread runs before its engine call (`App::engine_thread`). It is
/// called on the async thread that hands the job over, and answers the setup the job runs first, so a
/// test's thread-local cards go with the job; production's is `registered_thread`.
pub type EngineThread = fn() -> Box<dyn FnOnce() + Send>;

/// Production's `EngineThread`: the blocking pool's thread reads the registered catalog, as the
/// runtime's threads do.
pub fn registered_thread() -> Box<dyn FnOnce() + Send> {
    Box::new(registered)
}

/// #1035: run blocking engine work on tokio's blocking pool so folds do not block runtime workers.
/// Sets thread-locals (`with_shipped`, R767000) and runs inside `catch_unwind`.
/// `Err` is a panic's text or join error.
pub async fn off_runtime<T: Send + 'static>(
    thread: EngineThread,
    run: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let setup = thread();
    tokio::task::spawn_blocking(move || {
        setup();
        catch_unwind(AssertUnwindSafe(run)).map_err(panic_text)
    })
    .await
    .map_err(|error| error.to_string())?
}

/// The `CreateGameArgs` a match is created with: no catalog override (the caller's `Pinned` sets
/// the catalog, R767000) and no handicaps (a server match is Easy both, R180), the rest as the
/// match row froze it.
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
        // R923000: no match the server runs is a practice sandbox.
        sandbox: None,
        // R915005: nor holds a custom card, which plays only in practice.
        custom_cards: None,
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
        // R923000, R915005: as `create_game_args`, never a sandbox and no custom card.
        sandbox: None,
        custom_cards: None,
    }
}

/// Panics on a deck the catalog refuses.
pub fn create_game(pinned: Pinned, args: &CreateGameArgs) -> EngineState {
    registered();
    pinned.run(|| jackioh_engine::create_game(args))
}

pub fn begin_game(pinned: Pinned, state: &EngineState) -> ReduceResult {
    registered();
    pinned.run(|| jackioh_engine::begin_game(state))
}

pub fn reduce(pinned: Pinned, state: &EngineState, action: &Action) -> ReduceResult {
    registered();
    pinned.run(|| jackioh_engine::reduce(state, action))
}

pub fn legal_actions(pinned: Pinned, state: &EngineState, player: PlayerId) -> Vec<ActionBody> {
    registered();
    pinned.run(|| jackioh_engine::legal_actions(state, player))
}

/// SPEC §10.8: the only window a player gets onto the match.
pub fn view_for(pinned: Pinned, state: &EngineState, player: PlayerId) -> PlayerView {
    registered();
    pinned.run(|| jackioh_engine::view_for(state, player))
}

/// R2652000: how many of a change's `fresh` events `player`'s view carries (a face-down trap's flip it
/// leaves out), so the bots' personas slice that view by the change alone (R645).
pub fn viewed_of_newest(state: &EngineState, player: PlayerId, fresh: usize) -> usize {
    jackioh_engine::viewed_of_newest(state, player, fresh)
}

/// §9.5: a crashed actor rebuilds its state by folding `(seed, log)`.
pub fn fold(pinned: Pinned, args: &FoldArgs) -> FoldResult {
    registered();
    pinned.run(|| jackioh_engine::fold(args))
}

pub fn hash_state(state: &EngineState) -> String {
    jackioh_engine::hash_state(state)
}

/// R768: a finished match's log folded once, checked against this build's catalog version and the
/// match's recorded hash. Panics, as `fold` does, on a setup `create_game` refuses.
pub fn replay_open(args: &FoldArgs, record: &ReplayRecord) -> ReplayOpen {
    registered();
    jackioh_engine::replay_open(args, record, jackioh_cards::catalog_version())
}

/// R767000: the catalog a game played on `played_on` folds under: `None` for this build's own,
/// the shipped one it carries, or `earlier_patch`.
pub fn catalog_for(played_on: &str) -> Result<Option<&'static ShippedCatalog>, ReplayRefusal> {
    registered();
    jackioh_engine::catalog_for(played_on, jackioh_cards::catalog_version())
}

/// R767000: `run` on this thread under `shipped` (`None`: this build's own catalog), for that call.
pub fn with_shipped<T>(shipped: Option<&'static ShippedCatalog>, run: impl FnOnce() -> T) -> T {
    registered();
    jackioh_engine::with_shipped(shipped, run)
}

/// R388, R767000: the catalog a match is played, rebuilt and replayed under: this build's own
/// (`Pinned::default()`), or the shipped catalog of the version it began on. Every function here
/// that reads a card takes one; a caller with no match (`bots::calibrate`, a test) passes the default.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pinned(Option<&'static ShippedCatalog>);

impl Pinned {
    /// The catalog of `version`, or why this build cannot fold a game played on it
    /// (`earlier_patch`: neither its own version nor one it carries).
    pub fn of(version: &str) -> Result<Pinned, ReplayRefusal> {
        catalog_for(version).map(Pinned)
    }

    /// R767000: the catalog an opened replay's checkpoints were folded under, as `replay_page` reads it.
    pub fn of_checkpoints(checkpoints: &ReplayCheckpoints) -> Pinned {
        registered();
        Pinned(
            checkpoints
                .catalog_version
                .as_deref()
                .and_then(jackioh_engine::shipped_catalog),
        )
    }

    /// `run` under this catalog, on this thread, for that call.
    pub fn run<T>(self, run: impl FnOnce() -> T) -> T {
        with_shipped(self.0, run)
    }
}

/// R768: steps `[from, from + count)` of an opened replay, each `view_for(state_k, seat)`.
pub fn replay_page(checkpoints: &ReplayCheckpoints, seat: PlayerId, from: usize, count: usize) -> ReplayPage {
    registered();
    jackioh_engine::replay_page(checkpoints, seat, from, count)
}

/// R677: the seat the account that began the match in `home` plays in `state`.
pub fn seat_played_by(state: &EngineState, home: PlayerId) -> PlayerId {
    jackioh_engine::seat_played_by(state, home)
}

/// Public bookkeeping read off state fields and engine queries.
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
        sequence: jackioh_engine::prompts::sequence_question_of(state),
    }
}

/// SPEC §11 R258: All Random deck in library order, seeded so the same seed deals the same deck.
/// R1372, R924000: `lean` is the set the deck leans on, at least half from that set (R1370).
/// Dealt from every set, soft gate (R1390) included.
pub fn deal_random_deck(seed: &str, lean: Option<SetName>) -> Vec<String> {
    deal_random_deck_with_strength(seed, lean, 0.0)
}

/// #750 (R1454): `deal_random_deck` leaning on the cards that win, as far as `strength` (0 to 1) says
/// (`AiDeckOptions::strength`): how the bot pool deals a ladder bot's random decks. At 0.0 it is
/// `deal_random_deck`'s deal exactly, which is what every player's deal stays.
pub fn deal_random_deck_with_strength(seed: &str, lean: Option<SetName>, strength: f64) -> Vec<String> {
    registered();
    // R258: `banned: []` is practice's "random deck for a human" (no shadow-ban: R186's list shapes
    // the AI's own decks, not a player's), and `DECK_SIZE` is the size L2 asks of every deck.
    build_ai_deck(
        &mut Rng::new(seed, 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(vec![]),
            lean_set: lean,
            gated_sets: Some(vec![]),
            strength: Some(strength),
            ..AiDeckOptions::default()
        },
    )
}

/// §9.9, #1303: deck practice AI plays in a free game on `seed`: weighted draw under shadow
/// ban (R186) and soft gate (R1390), at tier deck size and curved to mana cap, on stream
/// `"{seed}:ai-deck"`. Reads catalog caller's `Pinned` runs it under.
pub fn deal_practice_ai_deck(seed: &str, handicap: &Handicap) -> Vec<String> {
    registered();
    build_ai_deck(
        &mut Rng::new(&format!("{seed}:ai-deck"), 0),
        handicap.deck_size,
        &AiDeckOptions {
            mana_cap: Some(handicap.mana_cap),
            ..AiDeckOptions::default()
        },
    )
}

/// R641000: `deal_random_deck`'s deal with no lean and none of `excluded`'s cards in it: how the bot
/// pool deals a ladder bot's trio a deck that shares no card with the decks it holds already (R253).
pub fn deal_random_deck_excluding(seed: &str, excluded: &[String]) -> Vec<String> {
    deal_random_deck_excluding_with_strength(seed, excluded, 0.0)
}

/// #750: `deal_random_deck_excluding` leaning on the cards that win, as `deal_random_deck_with_strength`
/// does: a ladder bot's trio decks.
pub fn deal_random_deck_excluding_with_strength(
    seed: &str,
    excluded: &[String],
    strength: f64,
) -> Vec<String> {
    registered();
    build_ai_deck(
        &mut Rng::new(seed, 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(excluded.to_vec()),
            lean_set: None,
            gated_sets: Some(vec![]),
            strength: Some(strength),
            ..AiDeckOptions::default()
        },
    )
}

/// R1372, R924000: the set a seat's All Random deck leans on, from its player's "More cards from"
/// pick: "newest" is the newest set that ships when the deck is dealt (R1371), a set is that set, and
/// no pick is no lean.
pub fn lean_of(lean: Option<LeanPick>) -> Option<SetName> {
    lean.map(LeanPick::resolve)
}

/// R1834003: every card dealt from a deck in a pristine game, by instance id, seat and dealt card id.
/// Minted cards (R943 Created tokens/copies) are excluded. Owner can change later (R12);
/// cosmetics are only sent for cards still showing what was dealt.
pub fn dealt_cards(state: &EngineState) -> BTreeMap<String, (PlayerId, String)> {
    let mut dealt = BTreeMap::new();
    for seat in [PlayerId::P1, PlayerId::P2] {
        let side = &state.players[seat];
        let field = side
            .units
            .iter()
            .flatten()
            .flatten()
            .chain(side.backrow.iter().flatten())
            .chain(side.carried.iter().flatten().flatten());
        let cards = side
            .library
            .iter()
            .chain(&side.hand)
            .chain(&side.graveyard)
            .chain(&side.exile)
            .chain(&side.resolving)
            .chain(field);
        for card in cards.filter(|card| card.created.is_none()) {
            dealt.insert(card.id.clone(), (card.owner, card.def_id.clone()));
        }
    }
    dealt
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
pub fn summarize_game(pinned: Pinned, args: &FoldArgs) -> Option<GameSummary> {
    registered();
    pinned.run(|| jackioh_engine::summarize_game(args))
}
