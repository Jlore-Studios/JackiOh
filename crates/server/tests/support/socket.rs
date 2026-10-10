//! In-memory `Socket`s over mpsc channels. Surface contract: docs/v0.3.0/SURFACE.md §11.2.
//!
//! `create_socket_pair` makes two fake clients. The server half (`FakeSocket::socket`) is the same
//! `Socket` the real `/ws/match` upgrade hands to `Registry::attach`, so the actor behaves as on
//! the wire.
//!
//! The actor is a task, so every read first drains what has arrived on the channel. A test that
//! waits awaits `next_frame`/`next_of_type` (or `settle`); one that asserts nothing happened reads
//! `sent` after `settle`, never by waiting (tokio's paused clock would auto-advance and fire the
//! match's timers).
//!
//! Every method takes `&self`, as every test holds its sockets (part 31); the state sits behind locks.

#![allow(dead_code)]

use std::sync::{Mutex, MutexGuard, PoisonError};

use jackioh_server::actor::ws_server::{Socket, SocketFrame};
use serde_json::Value;
use tokio::sync::mpsc;

/// How many times `settle` yields to the scheduler: enough for a frame to cross the inbox, the
/// actor's reducer and the outgoing channel, on the current-thread runtime `#[tokio::test]` uses.
const SETTLE_YIELDS: usize = 64;

/// How many frames a fake socket's outgoing queue holds. Far past `WS_OUTBOX_MAX_FRAMES`: a test reads
/// frames when it chooses, so the actor tests never meet the cap; R1437's is proved on
/// `Socket::channel()` (`actor/heartbeat.rs`).
const FAKE_OUTBOX_FRAMES: usize = 1 << 16;

/// The close code the transport reports when the connection drops without a close frame (1006).
const ABNORMAL_CLOSE: u16 = 1006;

/// Let the actor and every other task on this runtime run until they are idle.
pub async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// One fake client connection.
pub struct FakeSocket {
    /// The server half, until a test takes it to attach (`socket()`).
    socket: Mutex<Option<Socket>>,
    /// What the server sent: text frames and its close.
    outgoing: tokio::sync::Mutex<mpsc::Receiver<SocketFrame>>,
    state: Mutex<State>,
}

struct State {
    /// What the client sends; `None` once the transport is gone (closed or dropped).
    incoming: Option<mpsc::UnboundedSender<String>>,
    /// Every frame the server sent, in order.
    sent: Vec<String>,
    close_code: Option<u16>,
    close_reason: Option<String>,
}

impl State {
    fn record(&mut self, frame: SocketFrame) {
        if self.close_code.is_some() {
            return;
        }
        match frame {
            SocketFrame::Text(text) => self.sent.push(text),
            SocketFrame::Close { code, reason } => {
                self.close_code = Some(code);
                self.close_reason = Some(reason);
                // The transport is gone for the actor too: the end of the client's stream tells it.
                self.incoming = None;
            }
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeSocket {
    /// The server half, for `Registry::attach`. Taken once; a reconnect is a new `FakeSocket`.
    pub fn socket(&self) -> Socket {
        lock(&self.socket)
            .take()
            .expect("this fake socket's server half was already attached")
    }

    /// Moves every frame the server has sent into `sent`, and notes its close. A frame sent after
    /// the close is dropped. A `next_frame` in flight holds the receiver and records for itself.
    fn drain(&self) -> MutexGuard<'_, State> {
        let mut state = lock(&self.state);
        if let Ok(mut outgoing) = self.outgoing.try_lock() {
            while let Ok(frame) = outgoing.try_recv() {
                state.record(frame);
            }
        }
        state
    }

    pub fn is_open(&self) -> bool {
        self.drain().close_code.is_none()
    }

    /// The close code: the server's, `1006` after `drop()`, or `None` while open.
    pub fn close_code(&self) -> Option<u16> {
        self.drain().close_code
    }

    /// The reason the server's close carried (`"voided"` for R679's 4410), or `None`.
    pub fn close_reason(&self) -> Option<String> {
        self.drain().close_reason.clone()
    }

    /// Every frame the server sent, in order.
    pub fn sent(&self) -> Vec<String> {
        self.drain().sent.clone()
    }

    /// Every frame the server sent, parsed.
    pub fn messages(&self) -> Vec<Value> {
        self.drain().sent.iter().map(|text| parse(text)).collect()
    }

    /// The last frame the server sent, parsed.
    pub fn last(&self) -> Option<Value> {
        self.drain().sent.last().map(|text| parse(text))
    }

    /// Frames of one `type`, parsed.
    pub fn of_type(&self, frame_type: &str) -> Vec<Value> {
        self.messages()
            .into_iter()
            .filter(|message| message["type"] == frame_type)
            .collect()
    }

    /// Simulate the client sending a frame.
    pub fn receive(&self, text: &str) {
        let state = self.drain();
        let Some(incoming) = &state.incoming else {
            panic!("receive on a closed socket");
        };
        // A frame sent after the actor dropped its receiver is lost, as on a real connection.
        let _ = incoming.send(text.to_string());
    }

    /// Simulate the client sending JSON.
    pub fn receive_json(&self, value: Value) {
        self.receive(&value.to_string());
    }

    /// Simulate the transport dropping: no close frame, code 1006, and the actor sees the stream end.
    #[allow(clippy::should_implement_trait)]
    pub fn drop(&self) {
        let mut state = self.drain();
        if state.close_code.is_some() {
            return;
        }
        state.close_code = Some(ABNORMAL_CLOSE);
        state.incoming = None;
    }

    /// Forget every frame received so far.
    pub fn clear(&self) {
        self.drain().sent.clear();
    }

    /// The next text frame the server sends, parsed, or `None` once it has closed. Also kept in `sent`.
    pub async fn next_frame(&self) -> Option<Value> {
        let mut outgoing = self.outgoing.lock().await;
        loop {
            if lock(&self.state).close_code.is_some() {
                return None;
            }
            let frame = outgoing.recv().await?;
            let mut state = lock(&self.state);
            let before = state.sent.len();
            state.record(frame);
            if state.sent.len() > before {
                return state.sent.last().map(|text| parse(text));
            }
        }
    }

    /// Waits for the next frame of one `type`, skipping (and keeping in `sent`) every other one.
    pub async fn next_of_type(&self, frame_type: &str) -> Option<Value> {
        loop {
            let frame = self.next_frame().await?;
            if frame["type"] == frame_type {
                return Some(frame);
            }
        }
    }
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text)
        .unwrap_or_else(|error| panic!("the server sent a frame that is not JSON ({error}): {text}"))
}

pub fn create_fake_socket() -> FakeSocket {
    let (outgoing_tx, outgoing_rx) = mpsc::channel::<SocketFrame>(FAKE_OUTBOX_FRAMES);
    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<String>();
    FakeSocket {
        socket: Mutex::new(Some(Socket::new(outgoing_tx, incoming_rx))),
        outgoing: tokio::sync::Mutex::new(outgoing_rx),
        state: Mutex::new(State {
            incoming: Some(incoming_tx),
            sent: Vec::new(),
            close_code: None,
            close_reason: None,
        }),
    }
}

/// Two fake sockets, one per seat.
pub struct SocketPair {
    pub p1: FakeSocket,
    pub p2: FakeSocket,
}

pub fn create_socket_pair() -> SocketPair {
    SocketPair {
        p1: create_fake_socket(),
        p2: create_fake_socket(),
    }
}
