//! The end of a game (SPEC §2.5, R216): every way a game ends goes through `end_game`, so a finished
//! game has one shape whatever ended it.
//!
//! Port of `packages/engine/src/gameOver.ts`.

use crate::config::LOSER_DRAWS_REASONS;
use crate::script::EngineSink;
use crate::wire::{GameEvent, GameOverReason, GameResult, Phase, PlayerId, Winner};

/// §2.5: the game is over. R216: nothing happens after that, so a question still open when it ends —
/// a Discover the other seat conceded under (R211), a Death hook's question the state check that found
/// a hero at 0 left standing (R156) — can never be answered: `legal_actions` offers nothing once there
/// is a result, and a prompt left in `state.pending` would be shown to a seat that can do nothing with
/// it (§10.8). It is closed with the game, and `gameOver` is the last event of the action. So are
/// the mulligans, when a player concedes or a clock ends the game while they are open (R265): a
/// mulligan no one can answer is no longer open.
///
/// `winner` is TS's `PlayerId | "draw"`: a `PlayerId` or a `Winner` both pass.
pub fn end_game(sink: &mut EngineSink, winner: impl Into<Winner>, reason: GameOverReason) {
    let winner: Winner = winner.into();
    let state = &mut *sink.state;
    state.result = Some(GameResult { winner, reason });
    state.phase = Phase::Over;
    state.pending = None;
    state.mulligan = None;
    sink.events.push(GameEvent::GameOver { winner, reason });
}

/// R1843004: how a finished game ended for one seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatOutcome {
    Win,
    Draw,
    Loss,
}

/// R1843004: how a game that ended `winner` by `reason` went for `player`: the winner's win, a draw
/// for both when no one won, and the other player's loss, except after an ending that leaves them a
/// draw (`LOSER_DRAWS_REASONS`, Stinkyscape #148.1's `fled`). The ladder, the record, the rewards, the
/// card statistics and the AI all read a seat's result through this.
pub fn seat_outcome(winner: Winner, reason: GameOverReason, player: PlayerId) -> SeatOutcome {
    match winner.player() {
        None => SeatOutcome::Draw,
        Some(won) if won == player => SeatOutcome::Win,
        Some(_) if LOSER_DRAWS_REASONS.contains(&reason) => SeatOutcome::Draw,
        Some(_) => SeatOutcome::Loss,
    }
}

/// `seat_outcome` for a finished game's `result`.
pub fn outcome_for(result: &GameResult, player: PlayerId) -> SeatOutcome {
    seat_outcome(result.winner, result.reason, player)
}
