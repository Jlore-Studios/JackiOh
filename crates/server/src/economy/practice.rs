//! Practice earning's check (SPEC §14.8, §9.9, #1303): a practice game runs in the browser, so the
//! server takes no word of the client's for its result. It folds the claimed log from the setup its
//! seed was issued for and re-runs every AI move, and only a game that passes pays
//! ([[R1303000]]–[[R1303003]]).
//!
//! This mirrors the browser's worker (`apps/web/src/practice/core.ts`), function by function:
//!
//! - [`opening`] is its `startGame` for a free game: the human's deck (a list of cards, or the
//!   random deck dealt on `"{seed}:human-deck"` and counted dealt, R433), the AI's deck dealt on
//!   `"{seed}:ai-deck"` (`actor::engine::deal_practice_ai_deck`), the tier's handicap on the AI's
//!   seat alone (R180), and the human's last board, the server's own ([[R1303003]]). No game the
//!   server folds is a sandbox or holds a custom card (R923000, R915005: `create_game_args`).
//! - [`ai_action`] is its `aiStep`: `decide` on the AI's own stream, `"{seed}:ai"`, kept for the
//!   whole game, then the fallback (`fallbackActions`) when the decision is one R188 forbids or one
//!   the engine refuses. The AI reads the true state only through `ai_to_act` and `redact` (R185),
//!   as in play. There is no clock ([[R1303002]]): every decision runs to `AI_BUDGET`, and a move
//!   the browser's wall-clock cap cut short to another one does not match.
//! - [`verify`] is its `apply`, action by action, under the worker's nonces (`h<n>` the human's,
//!   `a<n>` the AI's, each counting only accepted actions), each sent as the seat its player plays
//!   then: a Glitch swap moves both (R677). A game that passes is read once more by the engine's
//!   `summarize_game`, for what the quests count ([[R1304001]]).
//!
//! Every engine call goes through `actor::engine` under the game's `Pinned` catalog (R767000), and
//! the AI runs inside `Pinned::run`, so it reads the catalog the game was played on. Nothing here
//! reads a clock or the store: `api::practice` checks the seed, runs this off the runtime and pays.

use serde::{Deserialize, Serialize};

use jackioh_ai::{AiOptions, ai_to_act, decide};
use jackioh_engine::{
    AI_DIFFICULTY, Action, ActionBody, GameOverReason, GameState, LastBoardEntry, PerPlayerOpt, PlayerId,
    Rng, SeatSummary, Winner,
};

use crate::actor::engine::{self, CreateGameArgs, FoldArgs, Pinned};
use crate::bots::seat::r188_forbidden;
use crate::economy::rewards::is_player_action;
use crate::economy::store::{PracticeDeck, PracticeSetup};

/// Why a practice claim pays nothing, as its answer names it (`{ paid: false, reason }`).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimRefusal {
    /// §14.12: the economy is off ([[R1302006]]).
    EconomyOff,
    /// The account was issued no seed by that name, or its seed was pruned ([[R1303001]]).
    UnknownSeed,
    /// The seed was claimed before, paid or refused ([[R1303001]]).
    AlreadyClaimed,
    /// The seed lapsed unclaimed after `PRACTICE_SEED_TTL_SECONDS` ([[R1303001]]).
    Expired,
    /// The log does not fold from the seed's setup: an action the engine refuses, one under the
    /// wrong nonce or seat, one after the end, or a setup that does not begin.
    DoesNotFold,
    /// An AI action in the log is not the one the AI chooses there ([[R1303002]]).
    AiMismatch,
    /// The log folds to a game that has not ended.
    NotFinished,
    /// The game ended voided by a Glitch (R679), which records nothing ([[R1303000]]).
    Voided,
    /// The game ended, but not with the result claimed.
    ResultMismatch,
}

/// The result a claim says the human had, read from the seat they ended in (R677).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClaimedResult {
    Win,
    Loss,
    Draw,
}

impl ClaimedResult {
    /// As `GameSummary::won` reads a result: `Some(true)` a win, `Some(false)` a loss, `None` a draw.
    pub fn won(self) -> Option<bool> {
        match self {
            ClaimedResult::Win => Some(true),
            ClaimedResult::Loss => Some(false),
            ClaimedResult::Draw => None,
        }
    }
}

/// A claim that passed: what the reward hook is paid with, and the board the human took away.
#[derive(Clone, Debug, PartialEq)]
pub struct Verified {
    /// The seat the human ended in (R677).
    pub player: PlayerId,
    /// The result for the human, as `GameSummary::won`.
    pub won: Option<bool>,
    /// The human's actions that earn XP ([[R1302004]]).
    pub actions: i64,
    /// How many of the log's actions were the AI's, each re-run.
    pub ai_moves: i64,
    /// R508: the board the human takes from the game, from the seat they ended in.
    pub last_board: Vec<LastBoardEntry>,
    /// §14.9: what the seat they ended in did, as the engine's `summarize_game` reads it off the
    /// log (R376), for the quests ([[R1304001]]); `None` when the fold reaches no result.
    pub seat: Option<SeatSummary>,
    /// §14.9: the attacks the human took, for the quests.
    pub attacks: i64,
}

/// R188, R923000: what the practice AI never plays, not even as a fallback: `core.ts`'s
/// `isForbiddenAiAction`, the bots' R188 list and a sandbox tool.
fn forbidden(body: &ActionBody) -> bool {
    r188_forbidden(body) || matches!(body, ActionBody::Sandbox { .. })
}

/// `body` and `rest` in seat order, as `home` and its opponent.
fn seat_ordered<T>(home: PlayerId, body: T, rest: T) -> (T, T) {
    match home {
        PlayerId::P1 => (body, rest),
        PlayerId::P2 => (rest, body),
    }
}

/// The game a practice seed's setup begins, as the worker's `startGame` begins it: `DoesNotFold`
/// when the engine refuses to begin it. Panics, as `create_game` does, on a deck the catalog
/// refuses; the caller runs it off the runtime, where a panic is a refusal too.
pub fn opening(pinned: Pinned, seed: &str, setup: &PracticeSetup) -> Result<GameState, ClaimRefusal> {
    begin(pinned, &opening_args(pinned, seed, setup))
}

/// The setup [`opening`] creates its game with.
fn opening_args(pinned: Pinned, seed: &str, setup: &PracticeSetup) -> CreateGameArgs {
    pinned.run(|| {
        let handicap = AI_DIFFICULTY[setup.difficulty];
        let human = setup.human_seat;
        let (human_deck, dealt) = match &setup.deck {
            PracticeDeck::Cards { cards } => (cards.clone(), None),
            PracticeDeck::Random { lean } => (
                engine::deal_random_deck(&format!("{seed}:human-deck"), engine::lean_of(*lean)),
                Some(vec![human]),
            ),
        };
        let decks = seat_ordered(human, human_deck, engine::deal_practice_ai_deck(seed, &handicap));
        let boards =
            (!setup.last_board.is_empty()).then(|| seat_ordered(human, setup.last_board.clone(), Vec::new()));
        let mut args = engine::create_game_args(seed, &decks, boards, None, dealt);
        let (p1, p2) = seat_ordered(human.opponent(), Some(handicap), None);
        args.handicaps = Some(PerPlayerOpt { p1, p2 });
        args
    })
}

/// The game `args` creates, begun: `DoesNotFold` when the engine refuses to begin it.
fn begin(pinned: Pinned, args: &CreateGameArgs) -> Result<GameState, ClaimRefusal> {
    let begun = engine::begin_game(pinned, &engine::create_game(pinned, args));
    match begun.error {
        Some(_) => Err(ClaimRefusal::DoesNotFold),
        None => Ok(begun.state),
    }
}

/// The action the practice AI takes in `state` from `seat` under `nonce`, as the worker's `aiStep`
/// takes it: `decide`'s choice, unless R188 forbids it or the engine refuses it, else the first
/// legal action the engine accepts, `endTurn` first; `None` when the AI owes nothing. `rng` is the
/// AI's stream, left where the decision left it.
pub fn ai_action(
    pinned: Pinned,
    state: &GameState,
    seat: PlayerId,
    nonce: &str,
    rng: &mut Rng,
) -> Option<ActionBody> {
    let accepted = |body: &ActionBody| {
        !forbidden(body)
            && engine::reduce(pinned, state, &Action::new(body.clone(), seat, nonce))
                .error
                .is_none()
    };
    let chosen = pinned.run(|| {
        if !ai_to_act(state, seat) {
            return None;
        }
        let mut options = AiOptions::new(rng.clone());
        let decision = decide(state, seat, &mut options);
        *rng = options.rng;
        Some(decision.map(|decision| decision.action))
    })?;
    if let Some(body) = chosen.filter(|body| accepted(body)) {
        return Some(body);
    }
    let legal = engine::legal_actions(pinned, state, seat);
    let (end_turn, rest): (Vec<ActionBody>, Vec<ActionBody>) = legal
        .into_iter()
        .partition(|body| matches!(body, ActionBody::EndTurn));
    end_turn.into_iter().chain(rest).find(|body| accepted(body))
}

/// A practice claim checked: `log` folded from `setup` on `seed`, every AI action re-run, to a
/// finished game whose result for the human is `claimed`.
pub fn verify(
    pinned: Pinned,
    seed: &str,
    setup: &PracticeSetup,
    claimed: ClaimedResult,
    log: &[Action],
) -> Result<Verified, ClaimRefusal> {
    let human_home = setup.human_seat;
    let ai_home = human_home.opponent();
    let args = opening_args(pinned, seed, setup);
    let mut state = begin(pinned, &args)?;
    let mut ai_rng = Rng::new(&format!("{seed}:ai"), 0);
    let (mut humans, mut ais, mut actions, mut attacks) = (0_i64, 0_i64, 0_i64, 0_i64);
    for action in log {
        if state.result.is_some() {
            return Err(ClaimRefusal::DoesNotFold);
        }
        if action.nonce == format!("a{ais}") {
            let seat = engine::seat_played_by(&state, ai_home);
            if action.player_id != seat {
                return Err(ClaimRefusal::DoesNotFold);
            }
            if ai_action(pinned, &state, seat, &action.nonce, &mut ai_rng).as_ref() != Some(&action.body) {
                return Err(ClaimRefusal::AiMismatch);
            }
            ais += 1;
        } else if action.nonce == format!("h{humans}") {
            if action.player_id != engine::seat_played_by(&state, human_home) {
                return Err(ClaimRefusal::DoesNotFold);
            }
            if is_player_action(&action.body) {
                actions += 1;
            }
            if matches!(action.body, ActionBody::Attack { .. }) {
                attacks += 1;
            }
            humans += 1;
        } else {
            return Err(ClaimRefusal::DoesNotFold);
        }
        let next = engine::reduce(pinned, &state, action);
        if next.error.is_some() {
            return Err(ClaimRefusal::DoesNotFold);
        }
        state = next.state;
    }
    let Some(result) = state.result else {
        return Err(ClaimRefusal::NotFinished);
    };
    if result.reason == GameOverReason::Voided {
        return Err(ClaimRefusal::Voided);
    }
    let player = engine::seat_played_by(&state, human_home);
    let won = match result.winner {
        Winner::Draw => None,
        winner => Some(winner == Winner::from(player)),
    };
    if won != claimed.won() {
        return Err(ClaimRefusal::ResultMismatch);
    }
    let (p1, p2) = engine::last_boards(&state);
    // The quests read what the game did off the same setup and log, as a ladder result reads its
    // match's (`api::results`).
    let game = engine::summarize_game(
        pinned,
        &FoldArgs {
            seed: args.seed,
            decks: args.decks,
            log: log.to_vec(),
            catalog: args.catalog,
            handicaps: args.handicaps,
            dealt: args.dealt,
            last_boards: args.last_boards,
            glitch_boards: args.glitch_boards,
            sandbox: args.sandbox,
            custom_cards: args.custom_cards,
        },
    );
    Ok(Verified {
        player,
        won,
        actions,
        ai_moves: ais,
        last_board: match player {
            PlayerId::P1 => p1,
            PlayerId::P2 => p2,
        },
        seat: game.map(|game| game.seats[player].clone()),
        attacks,
    })
}
