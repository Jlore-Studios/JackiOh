//! Practice earning's check (SPEC §14.8, §9.9, #1303): a practice game runs in the browser, so the
//! server takes no word of the client's for its result. It folds the claimed log from the setup its
//! seed was issued for, and only a game that folds to the claimed result pays ([[R1303000]]–[[R1303003]]).
//! It re-runs no AI move ([[R2560000]]): an AI action is held to its nonce and seat and reduced like any
//! other, since practice pay is held to the day's Jlore cap ([[R1483000]]).
//!
//! The rules of the check are the practice session's, `jackioh_shared::host::practice` (#3099): the
//! browser's worker plays the same machine, so [`verify`] steps it natively over the claimed game
//! (`check_claim`), and the game it folds is exactly the one the worker dealt (the human's deck, a list or
//! the random deck dealt on `"{seed}:human-deck"` and counted dealt, R433; the AI's on
//! `"{seed}:ai-deck"`; the tier's handicap on the AI's seat alone, R180; the human's last board,
//! [[R1303003]]; no sandbox, R923000), under the worker's nonces and seats (R677). What stays here is
//! this server's: the stored setup's mapping to the machine's config, the refusals as the store counts
//! them, and what the economy reads off the record (XP actions, the quests' attacks). The catalog is the
//! game's `Pinned` one (R767000); nothing here reads a clock or the store.

use serde::{Deserialize, Serialize};

use jackioh_engine::{Action, ActionBody, LastBoardEntry, PlayerId, SeatSummary};
use jackioh_shared::host::practice::{self as session, PracticeConfig, PracticeDeckChoice, PracticeResult};

use crate::actor::engine::{self, Pinned};
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
    /// The log folds to a game that has not ended.
    NotFinished,
    /// The game ended voided by a Glitch (R679), which records nothing ([[R1303000]]).
    Voided,
    /// The game ended, but not with the result claimed.
    ResultMismatch,
}

/// How a practice claim ended, as migration 0078's `practice_claim_counts` counts it by UTC day
/// (#2884): paid, or refused for one reason. A claim whose seed burned counts once, by the settle
/// that marked it paid or refused ([[R2422000]]); a refusal before the burn counts once per request.
/// A new refusal joins [`ClaimBucket::name`], [`ClaimBucket::ALL`] and a new migration's
/// `practice_claim_counts_outcome_check`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimBucket {
    Paid,
    Refused(ClaimRefusal),
}

impl ClaimBucket {
    /// Every bucket: paid, then each refusal in its declaration order.
    pub const ALL: &[ClaimBucket] = &[
        ClaimBucket::Paid,
        ClaimBucket::Refused(ClaimRefusal::EconomyOff),
        ClaimBucket::Refused(ClaimRefusal::UnknownSeed),
        ClaimBucket::Refused(ClaimRefusal::AlreadyClaimed),
        ClaimBucket::Refused(ClaimRefusal::Expired),
        ClaimBucket::Refused(ClaimRefusal::DoesNotFold),
        ClaimBucket::Refused(ClaimRefusal::NotFinished),
        ClaimBucket::Refused(ClaimRefusal::Voided),
        ClaimBucket::Refused(ClaimRefusal::ResultMismatch),
    ];

    /// The bucket's `outcome`: `paid`, or the refusal as its answer names it.
    pub fn name(self) -> &'static str {
        match self {
            ClaimBucket::Paid => "paid",
            ClaimBucket::Refused(reason) => match reason {
                ClaimRefusal::EconomyOff => "economy_off",
                ClaimRefusal::UnknownSeed => "unknown_seed",
                ClaimRefusal::AlreadyClaimed => "already_claimed",
                ClaimRefusal::Expired => "expired",
                ClaimRefusal::DoesNotFold => "does_not_fold",
                ClaimRefusal::NotFinished => "not_finished",
                ClaimRefusal::Voided => "voided",
                ClaimRefusal::ResultMismatch => "result_mismatch",
            },
        }
    }
}

/// The result a claim says the human had, read from the seat they ended in (R677).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClaimedResult {
    Win,
    Loss,
    Draw,
}

impl ClaimedResult {
    /// As `GameSummary::won` reads a result.
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
    /// How many of the log's actions were the AI's.
    pub ai_moves: i64,
    /// R508: the board the human takes from the game, from the seat they ended in.
    pub last_board: Vec<LastBoardEntry>,
    /// §14.9: what the seat they ended in did, as the engine's `summarize_game` reads it off the
    /// log (R376), for the quests ([[R1304001]]); `None` when the fold reaches no result.
    pub seat: Option<SeatSummary>,
    /// §14.9: the attacks the human took, for the quests.
    pub attacks: i64,
}

impl From<PracticeResult> for ClaimedResult {
    fn from(result: PracticeResult) -> Self {
        match result {
            PracticeResult::Win => ClaimedResult::Win,
            PracticeResult::Loss => ClaimedResult::Loss,
            PracticeResult::Draw => ClaimedResult::Draw,
        }
    }
}

impl From<ClaimedResult> for PracticeResult {
    fn from(result: ClaimedResult) -> Self {
        match result {
            ClaimedResult::Win => PracticeResult::Win,
            ClaimedResult::Loss => PracticeResult::Loss,
            ClaimedResult::Draw => PracticeResult::Draw,
        }
    }
}

impl From<session::ClaimRefusal> for ClaimRefusal {
    fn from(refusal: session::ClaimRefusal) -> Self {
        match refusal {
            session::ClaimRefusal::DoesNotFold => ClaimRefusal::DoesNotFold,
            session::ClaimRefusal::NotFinished => ClaimRefusal::NotFinished,
            session::ClaimRefusal::Voided => ClaimRefusal::Voided,
            session::ClaimRefusal::ResultMismatch => ClaimRefusal::ResultMismatch,
        }
    }
}

/// The game a practice seed's setup was issued for, as the practice session deals it: the human at its
/// seat on `setup`'s tier, its deck a list (a preset's or a saved deck's) or the random deck dealt from
/// the seed, and the last board the server kept for it (R1303003).
pub fn config_of(seed: &str, setup: &PracticeSetup) -> PracticeConfig {
    let deck = match &setup.deck {
        PracticeDeck::Cards { cards } => PracticeDeckChoice::Saved {
            index: 1,
            cards: cards.clone(),
            portrait: None,
            cosmetics: None,
        },
        PracticeDeck::Random { lean } => PracticeDeckChoice::Random {
            lean_set: *lean,
            lean_newest: None,
        },
    };
    let mut config = PracticeConfig::free(seed, setup.difficulty, setup.human_seat, deck);
    config.last_board = (!setup.last_board.is_empty()).then(|| setup.last_board.clone());
    config
}

/// A practice claim checked: `log` folded from `setup` on `seed`, each action under its player's next
/// nonce and from the seat that player plays then (R677), to a finished game whose result for the human
/// is `claimed`. No AI action is re-run ([[R2560000]]). Panics, as the engine does, on a setup it cannot
/// create; the caller runs it off the runtime, where a panic is a refusal too.
pub fn verify(
    pinned: Pinned,
    seed: &str,
    setup: &PracticeSetup,
    claimed: ClaimedResult,
    log: &[Action],
) -> Result<Verified, ClaimRefusal> {
    let config = config_of(seed, setup);
    let checked = engine::run_pinned(pinned, || {
        session::check_claim(&config, &setup.catalog_version, claimed.into(), log)
    })?;
    let record = checked.record;
    let human = &record.human_actions;
    Ok(Verified {
        player: record.human_seat,
        won: ClaimedResult::from(record.result).won(),
        actions: human.iter().filter(|body| is_player_action(body)).count() as i64,
        ai_moves: i64::from(record.ai_moves),
        last_board: record.last_board,
        seat: checked.seat,
        attacks: human
            .iter()
            .filter(|body| matches!(body, ActionBody::Attack { .. }))
            .count() as i64,
    })
}
