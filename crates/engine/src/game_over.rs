//! The end of a game (SPEC §2.5, R216): every way a game ends goes through `end_game`, so a finished
//! game has one shape whatever ended it.
//!
//! Port of `packages/engine/src/gameOver.ts`.

use crate::script::EngineSink;
use crate::wire::{GameEvent, GameOverReason, GameResult, Phase, Winner};

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
