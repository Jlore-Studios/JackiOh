//! The public bookkeeping the clock and the results writer need of a game (nothing here is hidden
//! information: both clients already see all of it), and the two reads of it the host's flow shares.

use serde::{Deserialize, Serialize};

use jackioh_engine::{GameResult, GameState, Phase, PlayerId};

use super::clock::{ClockView, SequenceClockView};

/// The public bookkeeping the match clock (`clock.rs`) and the results writer need.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
    #[cfg_attr(feature = "ts", ts(skip))]
    pub sequence: Option<jackioh_engine::prompts::SequenceQuestion>,
}

/// Public bookkeeping read off state fields and engine queries.
pub fn snapshot(state: &GameState) -> MatchSnapshot {
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

/// R265: the seats that owe a mulligan while the window is open — both mulligans open and no other
/// prompt in front of them — and nothing otherwise. The engine never reports both at once; the
/// clock (`clock.rs`) reads the window the same way.
pub fn mulligan_window(snapshot: &MatchSnapshot) -> Vec<PlayerId> {
    if snapshot.pending_for.is_none() && snapshot.result.is_none() {
        snapshot.mulligan_owed.clone()
    } else {
        Vec::new()
    }
}

/// What the match clock reads of a snapshot.
pub fn clock_view_for(snapshot: &MatchSnapshot) -> ClockView {
    ClockView {
        turn: snapshot.turn,
        active: snapshot.active,
        pending_for: snapshot.pending_for,
        mulligan_owed: snapshot.mulligan_owed.clone(),
        over: snapshot.result.is_some() || snapshot.phase == Phase::Over,
        sequence: snapshot.sequence.as_ref().map(|question| SequenceClockView {
            player: question.player,
            streak: question.streak,
            prompt_id: question.prompt_id.clone(),
        }),
    }
}

/// The key that tells one mulligan window's owing seats from another's, so a `prompt` frame goes out
/// only on a change.
pub fn mulligan_key(owed: &[PlayerId]) -> String {
    owed.iter()
        .map(|player| player.as_str())
        .collect::<Vec<_>>()
        .join(",")
}
