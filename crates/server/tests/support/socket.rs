//! In-memory `Socket`s (← `apps/server/test/fakes/socket.ts`, SURFACE §11.2: "a fake socket over an
//! mpsc channel").
//!
//! `create_fake_socket` is one end the actor talks to and the test reads; `create_socket_pair` makes
//! two of them so a test can drive two "clients" that look exactly like WebSocket peers without
//! opening a port. The server half (`FakeSocket::socket`) is the same `Socket` the real `/ws/match`
//! upgrade hands to `Registry::attach` — two channels, one for the frames the server sends and one
//! for the text frames the client sends — so what the actor does here is what it does on the wire.
//!
//! TS's fake was synchronous: a frame the actor sent was in `sent` before `send` returned. Here the
//! actor is a task, so every read below first drains what has arrived on the channel; a test that
//! waits for the actor awaits `next_frame`/`next_of_type` (or `settle`), and a test that asserts that
//! nothing happened reads `sent` after `settle`, never by waiting for a frame that never comes (with
//! tokio's paused clock, an idle wait would let the clock auto-advance and fire the match's timers).

#![allow(dead_code)]

use jackioh_server::actor::ws_server::{Socket, SocketFrame};
use serde_json::Value;
use tokio::sync::mpsc;

/// How many times `settle` yields to the scheduler: enough for a frame to cross the inbox, the
/// actor's reducer and the outgoing channel, on the current-thread runtime `#[tokio::test]` uses.
const SETTLE_YIELDS: usize = 64;

/// The close code the transport reports when the connection drops without a close frame
/// (TS's `drop()`, RFC 6455's 1006).
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
    socket: Option<Socket>,
    /// What the server sent: text frames and its close.
    outgoing: mpsc::UnboundedReceiver<SocketFrame>,
    /// What the client sends; `None` once the transport is gone (closed or dropped).
    incoming: Option<mpsc::UnboundedSender<String>>,
    /// Every frame the server sent, in order.
    sent: Vec<String>,
    close_code: Option<u16>,
    close_reason: Option<String>,
}

impl FakeSocket {
    /// The server half of this connection, for `Registry::attach`. Taken once: one WebSocket is one
    /// socket, and a reconnect is a new `FakeSocket`.
    pub fn socket(&mut self) -> Socket {
        self.socket.take().expect("this fake socket's server half was already attached")
    }

    /// Moves every frame the server has sent so far into `sent`, and notes its close. A frame sent
    /// after the close is not delivered, as on a real connection.
    fn drain(&mut self) {
        while let Ok(frame) = self.outgoing.try_recv() {
            self.record(frame);
        }
    }

    fn record(&mut self, frame: SocketFrame) {
        if self.close_code.is_some() {
            return;
        }
        match frame {
            SocketFrame::Text(text) => self.sent.push(text),
            SocketFrame::Close { code, reason } => {
                self.close_code = Some(code);
                self.close_reason = Some(reason);
                // TS `close()` called the attached handlers' `close`: the transport is gone for the
                // actor too, which the end of the client's stream tells it.
                self.incoming = None;
            }
        }
    }

    pub fn is_open(&mut self) -> bool {
        self.drain();
        self.close_code.is_none()
    }

    /// The code the connection closed with: the server's close, `1006` after `drop()`, or `None`
    /// while it is open.
    pub fn close_code(&mut self) -> Option<u16> {
        self.drain();
        self.close_code
    }

    /// The reason the server's close carried (`"voided"` for R679's 4410), or `None`.
    pub fn close_reason(&mut self) -> Option<String> {
        self.drain();
        self.close_reason.clone()
    }

    /// Every frame the server sent, in order.
    pub fn sent(&mut self) -> &[String] {
        self.drain();
        &self.sent
    }

    /// Every frame the server sent, parsed.
    pub fn messages(&mut self) -> Vec<Value> {
        self.drain();
        self.sent.iter().map(|text| parse(text)).collect()
    }

    /// The last frame the server sent, parsed.
    pub fn last(&mut self) -> Option<Value> {
        self.drain();
        self.sent.last().map(|text| parse(text))
    }

    /// Frames of one `type`, parsed.
    pub fn of_type(&mut self, frame_type: &str) -> Vec<Value> {
        self.messages().into_iter().filter(|message| message["type"] == frame_type).collect()
    }

    /// Simulate the client sending a frame.
    pub fn receive(&mut self, text: &str) {
        self.drain();
        let Some(incoming) = &self.incoming else {
            panic!("receive on a closed socket");
        };
        // The server half dropping its receiver is the actor letting the socket go; a frame sent
        // after that is lost, as on a real connection the server stopped reading.
        let _ = incoming.send(text.to_string());
    }

    /// Simulate the client sending JSON.
    pub fn receive_json(&mut self, value: &Value) {
        self.receive(&value.to_string());
    }

    /// Simulate the transport dropping: no close frame, code 1006, and the actor sees the stream end.
    #[allow(clippy::should_implement_trait)]
    pub fn drop(&mut self) {
        self.drain();
        if self.close_code.is_some() {
            return;
        }
        self.close_code = Some(ABNORMAL_CLOSE);
        self.incoming = None;
    }

    /// Forget every frame received so far.
    pub fn clear(&mut self) {
        self.drain();
        self.sent.clear();
    }

    /// Waits for the next text frame the server sends and answers it parsed, or `None` once the
    /// server has closed (or dropped) its half. The frame is also kept in `sent`.
    pub async fn next_frame(&mut self) -> Option<Value> {
        loop {
            if self.close_code.is_some() {
                return None;
            }
            let frame = self.outgoing.recv().await?;
            let before = self.sent.len();
            self.record(frame);
            if self.sent.len() > before {
                return self.sent.last().map(|text| parse(text));
            }
        }
    }

    /// Waits for the next frame of one `type`, skipping (and keeping in `sent`) every other one.
    pub async fn next_of_type(&mut self, frame_type: &str) -> Option<Value> {
        loop {
            let frame = self.next_frame().await?;
            if frame["type"] == frame_type {
                return Some(frame);
            }
        }
    }
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("the server sent a frame that is not JSON ({error}): {text}"))
}

pub fn create_fake_socket() -> FakeSocket {
    let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel::<SocketFrame>();
    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<String>();
    FakeSocket {
        socket: Some(Socket::new(outgoing_tx, incoming_rx)),
        outgoing: outgoing_rx,
        incoming: Some(incoming_tx),
        sent: Vec::new(),
        close_code: None,
        close_reason: None,
    }
}

/// Two fake sockets, one per seat, for a two-client test.
pub struct SocketPair {
    pub p1: FakeSocket,
    pub p2: FakeSocket,
}

/// Two fake sockets, one per seat, for a two-client test.
pub fn create_socket_pair() -> SocketPair {
    SocketPair { p1: create_fake_socket(), p2: create_fake_socket() }
}
