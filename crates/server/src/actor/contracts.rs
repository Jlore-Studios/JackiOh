//! The seams inside `actor`: the socket transport, the match clock and the results writer. Port of
//! `apps/server/src/match/contracts.ts`.
//!
//! They are separated so the actor (M6-T4) never imports a WebSocket library, the clock (M7-T1)
//! never imports a timer, and the results writer (M7-T2) never imports the actor. In TS everything
//! was driven from injected ports, which is what made the fake-timer tests possible. In Rust
//! (SURFACE §11.3) the ports go: `Timers` is `tokio::time` (a test pauses and advances it),
//! `Logger` is `tracing`, the engine is called directly (`actor::engine`), and `RecordResult` /
//! `VoidMatch` are `crate::api::results::{record_result, void_match}`. What stays here is the data
//! that crosses the seams and the one transport type, `Socket`, which `ws_server.rs` builds over a
//! real WebSocket and a test builds over a channel.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

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

/// What travels from the actor to the transport: a text frame, or the close (code, reason).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketFrame {
    Text(String),
    Close { code: u16, reason: String },
}

static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(1);

struct SocketInner {
    id: u64,
    open: AtomicBool,
    /// `handlers.close` runs at most once, whichever of `close` and `gone` comes first.
    gone: AtomicBool,
    out: mpsc::UnboundedSender<SocketFrame>,
    handlers: Mutex<Option<Arc<SocketHandlers>>>,
}

/// One player's connection. Text frames only: every protocol message is JSON. A WebSocket
/// (`ws_server.rs`) and the in-memory test socket both are this, so the actor is
/// transport-agnostic.
///
/// The transport owns the receiving end of the frame channel: it writes every `SocketFrame::Text`
/// to the peer and closes the connection on `SocketFrame::Close`. It hands every text frame the peer
/// sends to `receive`, and calls `gone` once the connection has ended. A `Socket` is a cheap handle
/// (`Clone`); two handles are the same connection exactly when `same` says so (TS compared sockets
/// by identity).
#[derive(Clone)]
pub struct Socket {
    inner: Arc<SocketInner>,
}

impl std::fmt::Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket").field("id", &self.inner.id).field("open", &self.is_open()).finish()
    }
}

/// Why `Socket::send` did not deliver (TS `send` threw).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SocketSendError {
    #[error("send on a closed socket")]
    Closed,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Socket {
    /// A socket whose frames go to `out`; its receiver is the transport's (or the test's).
    pub fn new(out: mpsc::UnboundedSender<SocketFrame>) -> Socket {
        Socket {
            inner: Arc::new(SocketInner {
                id: NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed),
                open: AtomicBool::new(true),
                gone: AtomicBool::new(false),
                out,
                handlers: Mutex::new(None),
            }),
        }
    }

    /// A socket and the receiving end of its frames, for a transport or a test to drain.
    pub fn channel() -> (Socket, mpsc::UnboundedReceiver<SocketFrame>) {
        let (out, frames) = mpsc::unbounded_channel();
        (Socket::new(out), frames)
    }

    /// This connection's identity, unique in the process.
    pub fn id(&self) -> u64 {
        self.inner.id
    }

    /// TS `===` on sockets.
    pub fn same(&self, other: &Socket) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// TS `isOpen` (`ws.readyState === OPEN`): neither side has closed it and the transport still
    /// reads its frames.
    pub fn is_open(&self) -> bool {
        self.inner.open.load(Ordering::SeqCst) && !self.inner.out.is_closed()
    }

    pub fn send(&self, text: String) -> Result<(), SocketSendError> {
        if !self.is_open() {
            return Err(SocketSendError::Closed);
        }
        self.inner.out.send(SocketFrame::Text(text)).map_err(|_| SocketSendError::Closed)
    }

    /// TS `close(code?, reason?)`: `ws.close(code ?? 1000, reason ?? "")`. The socket reads closed at
    /// once, and the attached `close` handler runs (a WebSocket's close event; the TS fake called it
    /// synchronously too). Closing twice does nothing more.
    pub fn close(&self, code: Option<u16>, reason: Option<&str>) {
        if !self.inner.open.swap(false, Ordering::SeqCst) {
            return;
        }
        let _ = self.inner.out.send(SocketFrame::Close {
            code: code.unwrap_or(1000),
            reason: reason.unwrap_or("").to_string(),
        });
        self.notify_gone();
    }

    /// Installed once, by the actor, when the socket is attached to a seat.
    pub fn attach(&self, handlers: SocketHandlers) {
        *lock(&self.inner.handlers) = Some(Arc::new(handlers));
    }

    /// Transport → actor: the peer sent a text frame. Dropped while nothing is attached, and once
    /// the socket is closed.
    pub fn receive(&self, text: String) {
        if !self.inner.open.load(Ordering::SeqCst) {
            return;
        }
        let handlers = lock(&self.inner.handlers).clone();
        if let Some(handlers) = handlers {
            (handlers.message)(text);
        }
    }

    /// Transport → actor: the connection ended (the peer closed it, or the transport dropped). A
    /// transport error is a disconnect too; §9.5's grace is what handles it.
    pub fn gone(&self) {
        self.inner.open.store(false, Ordering::SeqCst);
        self.notify_gone();
    }

    fn notify_gone(&self) {
        if self.inner.gone.swap(true, Ordering::SeqCst) {
            return;
        }
        // The handlers are cloned out first, so a handler that touches this socket again cannot
        // deadlock on its lock.
        let handlers = lock(&self.inner.handlers).clone();
        if let Some(handlers) = handlers {
            (handlers.close)();
        }
    }
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
