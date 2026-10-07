//! The seams inside `actor`: the socket transport, the match clock and the results writer. Port of
//! `apps/server/src/match/contracts.ts`.
//!
//! They are separated so the actor (M6-T4) never imports a WebSocket library, the clock (M7-T1)
//! never imports a timer, and the results writer (M7-T2) never imports the actor. In TS everything
//! was driven from injected ports, which is what made the fake-timer tests possible. In Rust
//! (SURFACE §11.3) the ports go: `Timers` is `tokio::time` (a test pauses and advances it),
//! `Logger` is `tracing`, the engine is called directly (`actor::engine`), and `RecordResult` /
//! `VoidMatch` are `crate::api::results::{record_result, void_match}`. What stays here is the data
//! that crosses the seams and the handlers the actor installs on a socket. The transport type,
//! `Socket`, is `actor::ws_server::Socket` (SURFACE §11.2), which a WebSocket and a test channel
//! both drive.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use jackioh_engine::{GameResult, LastBoardEntry, PlayerId};

use crate::db::store::MatchSeat;

pub use crate::actor::clock::MatchClock;

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// What the actor installs on a socket when it attaches it to a seat (TS `SocketHandlers`).
pub struct SocketHandlers {
    /// A text frame from the client.
    pub message: Box<dyn Fn(String) + Send + Sync>,
    /// The connection is gone (the client closed it, the transport dropped, or the server closed it).
    pub close: Box<dyn Fn() + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Clock (SPEC §9.5, R79)
// ---------------------------------------------------------------------------

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
}

/// Which clock ran out, tagged on `kind` as TS's union was.
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
}

impl ClockExpiry {
    /// TS `expiry.kind`: the literal the server nonce (`srv-<kind>-<seq>`) and the logs carry.
    pub fn kind(&self) -> &'static str {
        match self {
            ClockExpiry::Turn { .. } => "turn",
            ClockExpiry::Prompt { .. } => "prompt",
            ClockExpiry::Mulligan => "mulligan",
            ClockExpiry::Grace { .. } => "grace",
            ClockExpiry::Ceiling => "ceiling",
        }
    }
}

/// TS `onExpire`: what the clock calls when a deadline passes (never while it holds its own lock).
/// The actor's handler puts the expiry on its serialized queue and never blocks.
pub type ExpiryHandler = Arc<dyn Fn(ClockExpiry) + Send + Sync>;

/// TS `CreateMatchClockInput`, without `timers` and `config` (SURFACE §11.3: `tokio::time` and the
/// constants of `crate::config`).
#[derive(Clone)]
pub struct CreateMatchClockInput {
    /// When the match started; the ceiling is measured from here (R79). Epoch ms.
    pub started_at: i64,
    pub on_expire: ExpiryHandler,
}

/// TS `CreateMatchClock`: `crate::actor::clock::create_match_clock` has this type.
pub type CreateMatchClock = fn(CreateMatchClockInput) -> MatchClock;

// ---------------------------------------------------------------------------
// Results (SPEC §9.5, M7-T2)
// ---------------------------------------------------------------------------

/// TS `TerminalOutcome = { winner: PlayerId | "draw"; reason: GameOverReason }`: the engine's
/// `GameResult`, the same two fields.
pub type TerminalOutcome = GameResult;

/// R417: each seat's board, seat order.
pub type SeatBoards = (Vec<LastBoardEntry>, Vec<LastBoardEntry>);

/// What `crate::api::results::record_result(app, input)` (TS's `RecordResult` port, bound to
/// `createRecordResult`) takes. §9.5: "Every ending records a result and clears both players'
/// in-match state." One row per match; calling it twice for the same match is a no-op that returns
/// the row already written.
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
}

/// R679: what the actor knows of a match a Glitch voided, as
/// `crate::api::results::void_match(app, input)` (TS's `VoidMatch` port, bound to
/// `createVoidMatch`) takes it. That call forgets a voided match — no result, no rating, no record,
/// no last board; the row and its log go and both players are let go — and logs the one line that
/// names it. A voided game of a Conquest series is played again.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoidMatchInput {
    pub match_id: String,
    /// The match's two profiles, seat order as it began (`MatchRow.players`), for the log line.
    pub players: (String, String),
    pub at: i64,
}

// ---------------------------------------------------------------------------
// The actor's dependencies
// ---------------------------------------------------------------------------

/// TS `ActorDeps = { store, timers, config, log, engine, createClock, recordResult, voidMatch }`:
/// in Rust the `App` itself (its `db` is the store; the rest are direct calls, SURFACE §11.3).
pub type ActorDeps = Arc<crate::app::App>;

// ---------------------------------------------------------------------------
// One store call (TS's `deps.store.<sub>.<method>(…)` outside a `tx`)
// ---------------------------------------------------------------------------

/// `one_tx!(db, |t| expr)`: TS's single store call outside a `tx`, as SURFACE §11.2's
/// `begin` … `commit`: `expr` runs with `t: Tx` and may use `.await` and `?` (on `StoreError`);
/// the whole is a `Result<_, StoreError>`. Dropping the transaction on an error rolls it back.
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
