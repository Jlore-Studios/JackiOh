//! R1437 (SPEC §9.5, §9.8): a match socket that stops answering is found and dropped, and one that
//! stops reading is let go.
//!
//!  - the pump pings every `WS_PING_INTERVAL_SECONDS`; a socket that has sent no frame of any kind
//!    for `WS_IDLE_TIMEOUT_SECONDS` is dropped, which is a disconnect: its seat's grace starts and the
//!    opponent is shown it;
//!  - a socket that answers its pings stays;
//!  - a socket's outgoing queue holds `WS_OUTBOX_MAX_FRAMES` frames, and one more closes it.
//!
//! The first two run the real `/ws/match` upgrade and the real registry on tokio's paused clock. Real
//! TCP races a paused clock (the header of `actor/ws_server.rs`), so the connection is an in-memory
//! pipe (`MemoryListener`): `axum::serve` accepts the server end of a `tokio::io::duplex`, and the
//! test is the client on the other. Every duration is read from `config.rs`, so these tests state
//! R1437's numbers nowhere.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::mpsc;

use jackioh_server::actor::contracts::SocketHandlers;
use jackioh_server::actor::ws_server::{Socket, SocketFrame, WS_PATH};
use jackioh_server::app::{App, now_ms, router};
use jackioh_server::config::{
    DISCONNECT_GRACE_MS, WS_IDLE_TIMEOUT_SECONDS, WS_OUTBOX_MAX_FRAMES, WS_PING_INTERVAL_SECONDS,
};

use super::ws_server::{encode_query, seed_matches};
use crate::support::socket::{FakeSocket, create_fake_socket, settle};

/// The pipe's buffer each way: far more than a match's first frames, so a write never waits on the
/// pump and the paused clock has nothing to race.
const PIPE_BYTES: usize = 64 * 1024;

const IDLE_TIMEOUT: Duration = Duration::from_secs(WS_IDLE_TIMEOUT_SECONDS as u64);
const PING_INTERVAL: Duration = Duration::from_secs(WS_PING_INTERVAL_SECONDS as u64);

const PING: u8 = 0x9;
const PONG: u8 = 0xA;

/// The listener `axum::serve` accepts from: each stream a test sends is a connection.
struct MemoryListener(mpsc::UnboundedReceiver<DuplexStream>);

impl axum::serve::Listener for MemoryListener {
    type Io = DuplexStream;
    type Addr = ();

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.0.recv().await {
            Some(stream) => (stream, ()),
            // The test is over: nothing accepts any more.
            None => std::future::pending::<(DuplexStream, ())>().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(())
    }
}

struct Served {
    app: Arc<App>,
    streams: mpsc::UnboundedSender<DuplexStream>,
    server: tokio::task::JoinHandle<()>,
    /// Each profile's token, in profile order: `profile-1` plays p1 of `match-1`.
    tokens: Vec<String>,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.server.abort();
    }
}

/// `match-1` between `profile-1` and `profile-2`, started on the real registry, with the app served
/// over `MemoryListener`.
async fn served() -> Served {
    let (app, tokens) = seed_matches(1).await;
    let (streams, incoming) = mpsc::unbounded_channel();
    let service = router(Arc::clone(&app));
    let server = tokio::spawn(async move {
        axum::serve(MemoryListener(incoming), service)
            .await
            .expect("the listener serves");
    });
    Served {
        app,
        streams,
        server,
        tokens,
    }
}

impl Served {
    /// `profile-2` attached with an in-memory socket, which is how the opponent sees what p1 does.
    async fn attach_opponent(&self) -> FakeSocket {
        let opponent = create_fake_socket();
        self.app
            .matches
            .attach(&self.app, "match-1", "profile-2", opponent.socket())
            .await
            .expect("the opponent attaches");
        opponent
    }

    /// The upgrade of `profile-1` onto `match-1`, over a new pipe.
    async fn connect_p1(&self) -> Client {
        let (stream, server_end) = tokio::io::duplex(PIPE_BYTES);
        self.streams.send(server_end).expect("the listener is serving");
        let mut client = Client { stream };
        let request = format!(
            "GET {WS_PATH}?matchId=match-1&token={} HTTP/1.1\r\nHost: test\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n",
            encode_query(&self.tokens[0])
        );
        client
            .stream
            .write_all(request.as_bytes())
            .await
            .expect("the handshake is written");
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            let read = client
                .stream
                .read(&mut byte)
                .await
                .expect("the server answers the handshake");
            assert_ne!(read, 0, "the server ended the connection during the handshake");
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head);
        assert!(
            head.starts_with("HTTP/1.1 101"),
            "the upgrade was not accepted: {head}"
        );
        client
    }
}

/// A hand-rolled RFC 6455 client over the pipe, as in `actor/ws_server.rs`.
struct Client {
    stream: DuplexStream,
}

impl Client {
    /// One masked frame, as a client must send them.
    async fn send_frame(&mut self, opcode: u8, payload: &[u8]) {
        let mask = [0x12u8, 0x34, 0x56, 0x78];
        let mut frame = vec![0x80 | opcode];
        frame.push(0x80 | u8::try_from(payload.len()).expect("a short payload"));
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        self.stream.write_all(&frame).await.expect("the frame is written");
    }

    /// The next frame as it stands on the wire (server frames are unmasked), or `None` at the end
    /// of the stream.
    async fn read_frame_now(&mut self) -> Option<(u8, Vec<u8>)> {
        let mut head = [0u8; 2];
        self.stream.read_exact(&mut head).await.ok()?;
        let opcode = head[0] & 0x0F;
        let mut length = u64::from(head[1] & 0x7F);
        if length == 126 {
            let mut extended = [0u8; 2];
            self.stream.read_exact(&mut extended).await.ok()?;
            length = u64::from(u16::from_be_bytes(extended));
        } else if length == 127 {
            let mut extended = [0u8; 8];
            self.stream.read_exact(&mut extended).await.ok()?;
            length = u64::from_be_bytes(extended);
        }
        let mut payload = vec![0u8; usize::try_from(length).ok()?];
        self.stream.read_exact(&mut payload).await.ok()?;
        Some((opcode, payload))
    }

    /// The first `view` frame: the actor pushes one when the registry attaches the socket (§9.5).
    async fn first_view(&mut self) -> Value {
        loop {
            let (opcode, payload) = self
                .read_frame_now()
                .await
                .expect("a view frame after the attach");
            if opcode != 0x1 {
                continue;
            }
            let message: Value = serde_json::from_slice(&payload).expect("a JSON frame");
            if message["type"] == "view" {
                return message["view"].clone();
            }
        }
    }
}

#[tokio::test(start_paused = true)]
async fn r1437_drops_a_socket_that_answers_no_ping_after_the_idle_timeout_and_shows_the_opponent_its_grace() {
    let served = served().await;
    let opponent = served.attach_opponent().await;
    let mut p1 = served.connect_p1().await;
    p1.first_view().await;
    let opened = tokio::time::Instant::now();

    // Pings arrive and none is answered, until the server ends the connection.
    let mut pings = 0;
    while let Some((opcode, _)) = p1.read_frame_now().await {
        if opcode == PING {
            pings += 1;
        }
        assert!(
            opened.elapsed() <= IDLE_TIMEOUT + PING_INTERVAL,
            "the server never dropped a socket that answers nothing"
        );
    }
    assert_eq!(opened.elapsed().as_secs(), IDLE_TIMEOUT.as_secs());
    // A ping at each interval, and none at the instant of the drop.
    assert_eq!(pings, (WS_IDLE_TIMEOUT_SECONDS - 1) / WS_PING_INTERVAL_SECONDS);

    // The drop is a disconnect: the seat is absent and its grace runs, shown to the opponent. The
    // clock has not moved since the close, so the deadline is exactly one grace from now.
    settle().await;
    let presence = served
        .app
        .matches
        .presence_of("match-1")
        .expect("the match is live");
    assert!(!presence.p1);
    assert!(presence.p2);
    let shown = opponent.of_type("clock").last().cloned().expect("a clock frame");
    assert_eq!(
        shown["clocks"]["graceDeadline"]["p1"],
        json!(now_ms() + DISCONNECT_GRACE_MS)
    );
}

#[tokio::test(start_paused = true)]
async fn r1437_keeps_a_socket_that_answers_its_pings_open_past_the_idle_timeout() {
    const { assert!(WS_IDLE_TIMEOUT_SECONDS >= 3 * WS_PING_INTERVAL_SECONDS) };
    let served = served().await;
    let _opponent = served.attach_opponent().await;
    let mut p1 = served.connect_p1().await;
    p1.first_view().await;
    let opened = tokio::time::Instant::now();

    // Three idle timeouts' worth of pings, each answered with a pong that carries its payload.
    let mut pongs = 0;
    while opened.elapsed() < IDLE_TIMEOUT * 3 {
        let (opcode, payload) = p1
            .read_frame_now()
            .await
            .expect("the server kept a socket that answers its pings");
        assert_ne!(opcode, 0x8, "the server closed a socket that answers its pings");
        if opcode == PING {
            p1.send_frame(PONG, &payload).await;
            pongs += 1;
        }
    }
    assert!(pongs >= 3 * WS_IDLE_TIMEOUT_SECONDS / WS_PING_INTERVAL_SECONDS);
    settle().await;
    let presence = served
        .app
        .matches
        .presence_of("match-1")
        .expect("the match is live");
    assert!(presence.p1);
}

#[test]
fn r1437_overflowing_the_outbox_closes_the_socket_instead_of_growing_the_queue() {
    let (socket, mut frames) = Socket::channel();
    let closed = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&closed);
    socket.attach(SocketHandlers {
        message: Box::new(|_| {}),
        close: Box::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        }),
    });

    for number in 0..WS_OUTBOX_MAX_FRAMES {
        socket.send(number.to_string());
    }
    assert!(socket.is_open(), "a full outbox is not an overflow");

    socket.send("one too many");
    assert!(!socket.is_open());
    // Not the close handler: the actor sends while it holds its own lock, and that handler takes it.
    assert_eq!(closed.load(Ordering::SeqCst), 0);
    socket.send("after the close");

    // The queue holds what it held, nothing more, and then it ends.
    let mut held = 0;
    while let Ok(frame) = frames.try_recv() {
        assert_eq!(frame, SocketFrame::Text(held.to_string()));
        held += 1;
    }
    assert_eq!(held, WS_OUTBOX_MAX_FRAMES);
    assert_eq!(frames.try_recv(), Err(mpsc::error::TryRecvError::Disconnected));

    // The transport finds it ended and reports the close, once.
    socket.transport_closed();
    assert_eq!(closed.load(Ordering::SeqCst), 1);
}
