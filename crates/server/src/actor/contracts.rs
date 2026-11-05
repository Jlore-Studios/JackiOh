//! The seams inside `actor`: socket transport, the match clock's types and results writer.
//!
//! Separated so the actor never imports a WebSocket library, the clock never imports a timer
//! (the machine's timers are data, `jackioh_shared::host::timer`), and the results writer never
//! imports the actor (SURFACE §11.2, §11.3).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use jackioh_engine::{GameResult, LastBoardEntry};

use crate::db::store::MatchSeat;

/// What the actor installs on a socket when it attaches it to a seat.
pub struct SocketHandlers {
    /// A text frame from the client.
    pub message: Box<dyn Fn(String) + Send + Sync>,
    /// The connection is gone (the client closed it, the transport dropped, or the server closed it).
    pub close: Box<dyn Fn() + Send + Sync>,
}

// Clock (SPEC §9.5, R79)

// The clock is the match host's, pure (`jackioh_shared::host`, #3101): what it reads of a game and which
// clock ran out are its types, and the actor only names them in its logs and tests.
pub use jackioh_shared::host::clock::{ClockExpiry, ClockView, SequenceClockView};

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
