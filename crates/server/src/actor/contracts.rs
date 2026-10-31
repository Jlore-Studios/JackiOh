//! The seams inside `actor`: socket transport, match clock and results writer.
//!
//! Separated so the actor never imports a WebSocket library, the clock never imports a timer,
//! and the results writer never imports the actor (SURFACE §11.2, §11.3).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use jackioh_engine::{GameResult, LastBoardEntry, PlayerId};

use crate::db::store::MatchSeat;

pub use crate::actor::clock::MatchClock;

/// What the actor installs on a socket when it attaches it to a seat.
pub struct SocketHandlers {
    /// A text frame from the client.
    pub message: Box<dyn Fn(String) + Send + Sync>,
    /// The connection is gone (the client closed it, the transport dropped, or the server closed it).
    pub close: Box<dyn Fn() + Send + Sync>,
}

// Clock (SPEC §9.5, R79)

/// Everything the clock needs to know about the game; none of it is hidden information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClockView {
    pub turn: i32,
    pub active: PlayerId,
    /// Who owes the open prompt an answer, or null.
    pub pending_for: Option<PlayerId>,
    /// R265: the seats that still owe their mulligan while both are open, in seat order; empty
    /// outside that window. Non-empty with no `pending_for` is the window the mulligan clock runs
    /// over (R268).
    pub mulligan_owed: Vec<PlayerId>,
    pub over: bool,
    /// R1563006: the question of a sequence open now (Meditative+ #20 KY's Competition), the sequence
    /// clock's whole input; `None` with none open. Public: who holds it, its streak and its prompt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<SequenceClockView>,
}

/// R1563006: one question of a sequence, as the sequence clock reads it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SequenceClockView {
    pub player: PlayerId,
    /// The right answers so far: 0 on a sequence's first question, where its clock starts.
    pub streak: i32,
    pub prompt_id: String,
}

/// Which clock ran out.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClockExpiry {
    /// R79: the active player's turn clock ran out; `timeout` answers their prompts and ends the turn.
    Turn { player: PlayerId },
    /// R79: a prompt held by the non-active player ran out; `timeout` answers only that prompt.
    Prompt { player: PlayerId },
    /// R268: the one mulligan clock ran out. It names no player because it belongs to both: the
    /// actor times out every seat still owing its mulligan at that moment, each with its own
    /// `timeout`.
    Mulligan,
    /// §9.5: grace expired; `disconnectExpired` makes it a loss for that player.
    Grace { player: PlayerId },
    /// R79: the hard wall-clock ceiling; `ceilingReached` ends the match in a draw.
    Ceiling,
    /// R1563006: a question sequence's clock ran out; its question is answered Stop for its player.
    Sequence { player: PlayerId },
}

impl ClockExpiry {
    /// The literal the server nonce (`srv-<kind>-<seq>`) and the logs carry.
    pub fn kind(&self) -> &'static str {
        match self {
            ClockExpiry::Turn { .. } => "turn",
            ClockExpiry::Prompt { .. } => "prompt",
            ClockExpiry::Mulligan => "mulligan",
            ClockExpiry::Grace { .. } => "grace",
            ClockExpiry::Ceiling => "ceiling",
            ClockExpiry::Sequence { .. } => "sequence",
        }
    }
}

/// Called when a deadline passes; handler queues expiry on serialized queue without blocking.
pub type ExpiryHandler = Arc<dyn Fn(ClockExpiry) + Send + Sync>;

/// Input for creating a match clock (SURFACE §11.3).
#[derive(Clone)]
pub struct CreateMatchClockInput {
    /// When the match started; the ceiling is measured from here (R79). Epoch ms.
    pub started_at: i64,
    pub on_expire: ExpiryHandler,
}

// Results (SPEC §9.5, M7-T2)

/// Terminal outcome: the engine's `GameResult`.
pub type TerminalOutcome = GameResult;

/// R417: each seat's board, seat order.
pub type SeatBoards = (Vec<LastBoardEntry>, Vec<LastBoardEntry>);

/// What `record_result` takes (§9.5: every ending records a result and clears players' state).
/// Idempotent per match.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordResultInput {
    pub match_id: String,
    /// Seat order, `.0` is p1.
    pub seats: (MatchSeat, MatchSeat),
    pub outcome: TerminalOutcome,
    pub turns: i32,
    pub at: i64,
    /// R417, R565: each seat's board as this game ended, read from that seat's own side (seat
    /// order). Absent when the writer could not read the game: the reaper (R112).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_boards: Option<SeatBoards>,
    /// R768: `hash_state` of the game's final state, which its replay's fold is held to. Absent
    /// when the writer could not read the game: the reaper (R112).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_hash: Option<String>,
}

/// R679: input for `void_match`. Forgets a voided match (no result, no rating, row and log cleared)
/// and logs one line. A voided game of a Conquest series is played again.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoidMatchInput {
    pub match_id: String,
    /// The match's two profiles, seat order as it began (`MatchRow.players`), for the log line.
    pub players: (String, String),
    pub at: i64,
}

/// The actor's dependencies: the `App` itself (SURFACE §11.3).
pub type ActorDeps = Arc<crate::app::App>;

/// `one_tx!(db, |t| expr)`: single store call (SURFACE §11.2) with `begin` … `commit`.
/// `expr` runs with `t: Tx` and returns `Result<_, StoreError>`. Dropping rolls back on error.
macro_rules! one_tx {
    ($db:expr, |$t:ident| $body:expr) => {
        async {
            #[allow(unused_mut)]
            let mut $t = $db.begin(None).await?;
            let out = $body;
            $t.commit().await?;
            Ok::<_, $crate::db::store::StoreError>(out)
        }
        .await
    };
}

pub(crate) use one_tx;
