//! `/ws/match` over a real listener and a real WebSocket client: what one client can cost the
//! process before its token is even checked (§9.8), and how a client hands over its token.
//!
//!  - a frame over `MAX_FRAME_BYTES` is refused with 1009, so it is never buffered whole (the
//!    library's default ceiling is far larger);
//!  - one client address holds at most `WS_MAX_CONNECTIONS_PER_ADDRESS` sockets, handshakes
//!    included;
//!  - the token travels as `?token=`, and `WS_SUBPROTOCOL` is still echoed when it is offered.
//!
//! The whole app is served (`app::router`, which mounts `/ws/match`) and the registry is the real
//! one, so the actor's first `view` frame names the seat (`viewer: "p1"`). The client is a small
//! hand-rolled RFC 6455 client over a `TcpStream`.
//! Surface contract: docs/v0.3.0/SURFACE.md §2, §11.2, §11.3.
//!
//! `the_graceful_shutdown` serves through `app::serve_until` instead: SIGTERM (and Ctrl-C) drains
//! instead of cutting (#861).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use jackioh_server::actor::protocol::MAX_FRAME_BYTES;
use jackioh_server::actor::ws_server::{WS_PATH, WS_SUBPROTOCOL};
use jackioh_server::app::{App, router};
use jackioh_server::config::WS_MAX_CONNECTIONS_PER_ADDRESS;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

use crate::support::deps::{add_user, test_app};

/// How long a read waits before the test calls the socket silent. Real time: the listener does
/// real I/O, so tokio's clock is not paused in this file.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// The per-address count may be process-wide, so the tests of this file run one at a time: each opens
/// up to the cap from 127.0.0.1.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// Every playable catalog id, in `catalog.json` order: real decks, so the real registry can start
/// the match the socket attaches to.
fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| {
            !def.token
                && !serde_json::to_value(&def.tags)
                    .expect("tags")
                    .as_array()
                    .is_some_and(|tags| tags.contains(&json!("Token")))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

fn deck_size() -> usize {
    usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size")
}

/// A token is opaque; it is percent-encoded into the query so any token survives the URL.
pub(super) fn encode_query(raw: &str) -> String {
    raw.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (byte as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

struct Listening {
    port: u16,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Listening {
    fn drop(&mut self) {
        // Every client terminated, the sockets closed and the listener released.
        self.server.abort();
    }
}

/// One active player per seat of `matches` live matches, `profile-<k>` (user `user-<k>`) in
/// `match-<m>` with `k` = 2m − 1 and 2m, every match started on the real registry. Returns the app
/// and each profile's token, in profile order.
pub(super) async fn seed_matches(matches: usize) -> (Arc<App>, Vec<String>) {
    let app = test_app().await;
    let pool = playable();
    let size = deck_size();
    let mut tokens = Vec::new();
    for m in 1..=matches {
        let match_id = format!("match-{m}");
        let mut seats = Vec::new();
        for (seat, k) in [("p1", 2 * m - 1), ("p2", 2 * m)] {
            let profile_id = format!("profile-{k}");
            let user_id = format!("user-{k}");
            tokens.push(add_user(&app, &user_id, &format!("{user_id}@example.test"), true));
            fake(&app).lock().await.seed_profile(json!({
                "id": profile_id,
                "userId": user_id,
                "status": "active",
                "inMatchId": match_id,
            }));
            let deck: Vec<String> = if seat == "p1" {
                pool[..size].to_vec()
            } else {
                pool[size..size * 2].to_vec()
            };
            seats.push(json!({ "profileId": profile_id, "player": seat, "deck": deck }));
        }
        app.matches
            .start(
                &app,
                from(json!({
                    "matchId": match_id,
                    "seed": format!("seed-ws-{m}"),
                    "catalogVersion": jackioh_cards::catalog_version(),
                    "ranked": false,
                    "seats": seats,
                })),
            )
            .await
            .expect("the registry starts the match");
    }
    (app, tokens)
}

/// `seed_matches` served on a real listener. Returns the listener and each profile's token.
async fn listen_with(matches: usize) -> (Listening, Vec<String>) {
    let (app, tokens) = seed_matches(matches).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let port = listener.local_addr().expect("a bound address").port();
    let service = router(Arc::clone(&app)).into_make_service_with_connect_info::<std::net::SocketAddr>();
    let server = tokio::spawn(async move {
        axum::serve(listener, service).await.expect("the listener serves");
    });
    (Listening { port, server }, tokens)
}

async fn listen() -> (Listening, String) {
    let (listening, tokens) = listen_with(1).await;
    let token = tokens.into_iter().next().expect("profile-1's token");
    (listening, token)
}

/// What a handshake came to: an upgraded socket, or the HTTP status that refused it.
enum Opened {
    Open(Client),
    Refused(u16),
}

impl Opened {
    fn open(self) -> Client {
        match self {
            Opened::Open(client) => client,
            Opened::Refused(status) => panic!("the upgrade was refused with {status}"),
        }
    }

    fn refused_status(self) -> u16 {
        match self {
            Opened::Open(_) => panic!("the upgrade was accepted"),
            Opened::Refused(status) => status,
        }
    }
}

struct Client {
    stream: TcpStream,
    /// The `Sec-WebSocket-Protocol` the server echoed, if any.
    protocol: Option<String>,
}

async fn open(port: u16, query: &str, protocols: &[&str]) -> Opened {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("the listener accepts");
    let mut request = format!(
        "GET {WS_PATH}?{query} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n"
    );
    if !protocols.is_empty() {
        request.push_str(&format!("Sec-WebSocket-Protocol: {}\r\n", protocols.join(", ")));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("the handshake is written");

    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        let read = tokio::time::timeout(READ_TIMEOUT, stream.read(&mut byte))
            .await
            .expect("the server answers the handshake")
            .unwrap_or(0);
        if read == 0 {
            break;
        }
        head.push(byte[0]);
    }
    let text = String::from_utf8_lossy(&head).to_string();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    if status != 101 {
        return Opened::Refused(status);
    }
    let protocol = text.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("sec-websocket-protocol")
            .then(|| value.trim().to_string())
    });
    Opened::Open(Client { stream, protocol })
}

fn query(match_id: &str, token: &str) -> String {
    format!("matchId={}&token={}", encode_query(match_id), encode_query(token))
}

impl Client {
    /// One masked frame. A frame the server refuses may meet a socket it has already closed, so a
    /// failed write is not the test's failure; what the server answered is.
    async fn send_frame(&mut self, opcode: u8, payload: &[u8]) {
        let mask = [0x12u8, 0x34, 0x56, 0x78];
        let mut frame = vec![0x80 | opcode];
        match payload.len() {
            n if n < 126 => frame.push(0x80 | u8::try_from(n).expect("a short length")),
            n if n <= 0xFFFF => {
                frame.push(0x80 | 126);
                frame.extend_from_slice(&u16::try_from(n).expect("a 16-bit length").to_be_bytes());
            }
            n => {
                frame.push(0x80 | 127);
                frame.extend_from_slice(&u64::try_from(n).expect("a 64-bit length").to_be_bytes());
            }
        }
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        let _ = self.stream.write_all(&frame).await;
    }

    async fn send_text(&mut self, text: &str) {
        self.send_frame(0x1, text.as_bytes()).await;
    }

    /// The client's close: a close frame with 1000, then the server's answer or the end.
    async fn close(&mut self) -> u16 {
        self.send_frame(0x8, &1000u16.to_be_bytes()).await;
        self.close_code().await
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

    async fn read_frame(&mut self, wait: Duration) -> Read {
        match tokio::time::timeout(wait, self.read_frame_now()).await {
            Err(_) => Read::Silent,
            Ok(Some((opcode, payload))) => Read::Frame(opcode, payload),
            Ok(None) => Read::Ended,
        }
    }

    /// Reads to the server's close frame and answers its code; 1006 when the stream just ends, as
    /// `ws` reported an abnormal closure.
    async fn close_code(&mut self) -> u16 {
        loop {
            match self.read_frame(READ_TIMEOUT).await {
                Read::Frame(0x8, payload) => {
                    return if payload.len() >= 2 {
                        u16::from_be_bytes([payload[0], payload[1]])
                    } else {
                        1005
                    };
                }
                Read::Frame(..) => continue,
                Read::Ended => return 1006,
                Read::Silent => panic!("the socket never closed"),
            }
        }
    }

    async fn next_message(&mut self, wait: Duration) -> Option<Value> {
        loop {
            match self.read_frame(wait).await {
                Read::Frame(0x1, payload) => return serde_json::from_slice(&payload).ok(),
                Read::Frame(0x8, _) | Read::Ended | Read::Silent => return None,
                Read::Frame(..) => continue,
            }
        }
    }

    /// The first `view` frame: the actor pushes one when the registry attaches the socket (§9.5).
    async fn first_view(&mut self) -> Value {
        loop {
            let message = self
                .next_message(READ_TIMEOUT)
                .await
                .expect("a view frame after the attach");
            if message["type"] == "view" {
                return message["view"].clone();
            }
        }
    }

    /// Whether the socket is still open after `wait`: every frame that arrives meanwhile is read (the
    /// `malformed` answer among them), and none of them may be a close or the end of the stream.
    async fn still_open_after(&mut self, wait: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return true;
            }
            match self.read_frame(left).await {
                Read::Frame(0x8, _) | Read::Ended => return false,
                Read::Frame(..) => continue,
                Read::Silent => return true,
            }
        }
    }
}

enum Read {
    Frame(u8, Vec<u8>),
    /// Nothing arrived in the time given.
    Silent,
    /// The stream ended (or failed) without a close frame.
    Ended,
}

/// A slot is freed when the server has seen its socket close,
/// which may land a moment after the client saw it.
async fn open_when_free(port: u16, query: &str, protocols: &[&str]) -> Client {
    for _ in 0..50 {
        match open(port, query, protocols).await {
            Opened::Open(client) => return client,
            Opened::Refused(429) => tokio::time::sleep(Duration::from_millis(20)).await,
            Opened::Refused(status) => panic!("the upgrade was refused with {status}"),
        }
    }
    panic!("the slot was never freed");
}

mod a_frame_is_capped_before_it_is_buffered {
    //! §9.8: a frame is capped before it is buffered.
    use super::*;

    #[tokio::test]
    async fn closes_with_1009_on_a_65_kib_text_frame() {
        let _serial = SERIAL.lock().await;
        let (listening, token) = listen().await;
        let mut client = open(listening.port, &query("match-1", &token), &[WS_SUBPROTOCOL])
            .await
            .open();
        // The registry attached the socket: its seat's first view arrives.
        assert_eq!(client.first_view().await["viewer"], "p1");

        let frame = "x".repeat(65 * 1024);
        const { assert!(65 * 1024 > MAX_FRAME_BYTES) };
        client.send_text(&frame).await;
        assert_eq!(client.close_code().await, 1009);
    }

    #[tokio::test]
    async fn still_takes_a_frame_at_the_cap() {
        let _serial = SERIAL.lock().await;
        let (listening, token) = listen().await;
        let mut client = open(listening.port, &query("match-1", &token), &[WS_SUBPROTOCOL])
            .await
            .open();
        assert_eq!(client.first_view().await["viewer"], "p1");

        client.send_text(&"x".repeat(MAX_FRAME_BYTES)).await;
        assert!(client.still_open_after(Duration::from_millis(50)).await);
    }
}

mod sockets_per_client_address {
    //! §9.8: sockets per client address.
    use super::*;

    #[tokio::test]
    async fn refuses_the_upgrade_past_the_cap_with_429_and_frees_the_slot_when_a_socket_closes() {
        let _serial = SERIAL.lock().await;
        let cap = WS_MAX_CONNECTIONS_PER_ADDRESS;
        // Every socket is its own account's, so the registry replaces none of them: `cap` live
        // sockets from one address, two seats a match.
        let (listening, tokens) = listen_with(cap.div_ceil(2)).await;
        let mut held = Vec::new();
        for (k, token) in tokens.iter().enumerate().take(cap) {
            let match_id = format!("match-{}", k / 2 + 1);
            let mut client = open(listening.port, &query(&match_id, token), &[WS_SUBPROTOCOL])
                .await
                .open();
            client.first_view().await;
            held.push(client);
        }

        let third = open(listening.port, &query("match-1", &tokens[0]), &[WS_SUBPROTOCOL]).await;
        assert_eq!(third.refused_status(), 429);

        let mut first = held.remove(0);
        first.close().await;
        let mut fourth =
            open_when_free(listening.port, &query("match-1", &tokens[0]), &[WS_SUBPROTOCOL]).await;
        assert_eq!(fourth.first_view().await["viewer"], "p1");
    }

    #[tokio::test]
    async fn counts_a_socket_refused_for_a_bad_token_only_until_it_closes() {
        let _serial = SERIAL.lock().await;
        let cap = WS_MAX_CONNECTIONS_PER_ADDRESS;
        let (listening, _token) = listen().await;
        // More refused sockets than the cap, one after another: each holds its slot only until it
        // closes, so none of them meets a 429.
        for _ in 0..=cap {
            let mut refused = open_when_free(
                listening.port,
                &query("match-1", "not-a-token"),
                &[WS_SUBPROTOCOL],
            )
            .await;
            assert_eq!(refused.close_code().await, 4401);
        }
    }
}

mod the_token_on_the_handshake {
    //! SURFACE §11.3: one token path, `?token=` (what the browser and the e2e Node player send), and
    //! `jackioh.v1` is still echoed when it is offered; these hold the server to exactly that.
    use super::*;

    #[tokio::test]
    async fn authenticates_the_socket_and_echoes_only_the_protocol_name() {
        let _serial = SERIAL.lock().await;
        let (listening, token) = listen().await;
        let mut client = open(listening.port, &query("match-1", &token), &[WS_SUBPROTOCOL])
            .await
            .open();
        assert_eq!(client.protocol.as_deref(), Some(WS_SUBPROTOCOL));
        // profile-1 holds seat p1 of match-1.
        let view = client.first_view().await;
        assert_eq!(view["viewer"], "p1");
    }

    #[tokio::test]
    async fn reads_the_token_from_the_query_and_never_from_the_protocol_header() {
        let _serial = SERIAL.lock().await;
        let (listening, token) = listen().await;
        // A token offered beside the protocol name is not a credential: the socket is
        // refused as one with no token at all, and the token is never echoed.
        let mut client = open(listening.port, "matchId=match-1", &[WS_SUBPROTOCOL, &token])
            .await
            .open();
        assert_eq!(client.protocol.as_deref(), Some(WS_SUBPROTOCOL));
        let refusal = client
            .next_message(READ_TIMEOUT)
            .await
            .expect("an error frame before the close");
        assert_eq!(refusal["type"], "error");
        assert_eq!(client.close_code().await, 4401);

        // Without the protocol name nothing is echoed, and the query's token still authenticates.
        let mut plain = open(listening.port, &query("match-1", &token), &[]).await.open();
        assert_eq!(plain.protocol, None);
        assert_eq!(plain.first_view().await["viewer"], "p1");
    }
}

mod a_finished_matchs_seats_may_reopen_its_socket {
    //! §9.5: every ending clears both seats' in-match state, but the rematch window keeps a
    //! finished match's seats coming back — presence is the opponent's socket being open, so a
    //! seat that left may reopen its socket and read as here again. Anyone else is still
    //! refused (§9.1).
    use super::*;

    use jackioh_server::app::now_ms;
    use jackioh_server::db::store::MatchRow;

    /// The finished match's clocks, as `tests/api/rematch.rs` seeds them.
    fn clocks(now: i64) -> Value {
        json!({
            "turnDeadline": null,
            "promptDeadline": null,
            "graceDeadline": { "p1": null, "p2": null },
            "ceilingAt": now + 3_600_000,
        })
    }

    /// Two profiles with tokens and no in-match state, and the finished match between them:
    /// exactly what a recorded result leaves behind. Returns the app and both seats' tokens.
    async fn seed_finished_seats() -> (Arc<App>, String, String) {
        let app = test_app().await;
        let pool = playable();
        let size = deck_size();
        let mut tokens = Vec::new();
        for k in [1, 2] {
            let profile_id = format!("reopen-profile-{k}");
            let user_id = format!("reopen-user-{k}");
            tokens.push(add_user(&app, &user_id, &format!("{user_id}@example.test"), true));
            fake(&app).lock().await.seed_profile(json!({
                "id": profile_id,
                "userId": user_id,
                "status": "active",
            }));
        }
        let now = now_ms();
        let row: MatchRow = from(json!({
            "id": "match-reopened",
            "seed": "seed-reopened",
            "players": ["reopen-profile-1", "reopen-profile-2"],
            "decks": [pool[..size].to_vec(), pool[size..size * 2].to_vec()],
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": false,
            "status": "finished",
            "createdAt": now,
            "finishedAt": now,
            "clocks": clocks(now),
            "portraits": ["vanilla", "vanilla"],
        }));
        let mut tx = app.db.begin(None).await.expect("begin");
        tx.matches_create(&row).await.expect("matches_create");
        tx.commit().await.expect("commit");
        let mut tokens = tokens.into_iter();
        (
            app,
            tokens.next().expect("seat one's token"),
            tokens.next().expect("seat two's token"),
        )
    }

    /// `seed_finished_seats`'s app served on a real listener.
    async fn serve_finished(app: Arc<App>) -> Listening {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let port = listener.local_addr().expect("a bound address").port();
        let service = router(app).into_make_service_with_connect_info::<std::net::SocketAddr>();
        let server = tokio::spawn(async move {
            axum::serve(listener, service).await.expect("the listener serves");
        });
        Listening { port, server }
    }

    #[tokio::test]
    async fn a_seat_reopens_its_socket_on_its_finished_match() {
        let _serial = SERIAL.lock().await;
        let (app, _p1, p2) = seed_finished_seats().await;
        let listening = serve_finished(app).await;
        let mut client = open(listening.port, &query("match-reopened", &p2), &[WS_SUBPROTOCOL])
            .await
            .open();
        assert_eq!(client.first_view().await["viewer"], "p2");
    }

    #[tokio::test]
    async fn another_profiles_finished_match_is_still_refused() {
        let _serial = SERIAL.lock().await;
        let (app, _p1, _p2) = seed_finished_seats().await;
        let user_id = "reopen-user-3";
        let token = add_user(&app, user_id, &format!("{user_id}@example.test"), true);
        fake(&app).lock().await.seed_profile(json!({
            "id": "reopen-profile-3",
            "userId": user_id,
            "status": "active",
        }));
        let listening = serve_finished(app).await;
        let mut client = open(
            listening.port,
            &query("match-reopened", &token),
            &[WS_SUBPROTOCOL],
        )
        .await
        .open();
        let refusal = client
            .next_message(READ_TIMEOUT)
            .await
            .expect("an error frame before the close");
        assert_eq!(refusal["type"], "error");
        assert_eq!(client.close_code().await, 4403);
    }
}

mod banned_profiles_are_refused_the_match_socket {
    //! §9.4's gate on `/ws/match` (issue #887): a banned account's socket is refused, and
    //! unbanning opens it again.
    use super::*;
    use crate::support::deps::empty_test_app;
    use jackioh_server::db::store::ProfileStatus;

    /// One live match between profile-1 and profile-2, served: `seed_matches`' first match, but
    /// over an app without R144's fixtures, whose `profiles_create` ids (`profile-1`, …) collide
    /// with the seats' — banning by id would land on the fixture's row instead of the seat's.
    async fn listen_bannable() -> (Arc<App>, Listening, String) {
        let app = empty_test_app().await;
        let pool = playable();
        let size = deck_size();
        let token = add_user(&app, "user-1", "user-1@example.test", true);
        add_user(&app, "user-2", "user-2@example.test", true);
        fake(&app).lock().await.seed_profile(json!({
            "id": "profile-1",
            "userId": "user-1",
            "status": "active",
            "inMatchId": "match-1",
        }));
        fake(&app).lock().await.seed_profile(json!({
            "id": "profile-2",
            "userId": "user-2",
            "status": "active",
            "inMatchId": "match-1",
        }));
        app.matches
            .start(
                &app,
                from(json!({
                    "matchId": "match-1",
                    "seed": "seed-ws-ban",
                    "catalogVersion": jackioh_cards::catalog_version(),
                    "ranked": false,
                    "seats": [
                        { "profileId": "profile-1", "player": "p1", "deck": pool[..size].to_vec() },
                        { "profileId": "profile-2", "player": "p2", "deck": pool[size..size * 2].to_vec() },
                    ],
                })),
            )
            .await
            .expect("the registry starts the match");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let port = listener.local_addr().expect("a bound address").port();
        let service = router(Arc::clone(&app)).into_make_service_with_connect_info::<std::net::SocketAddr>();
        let server = tokio::spawn(async move {
            axum::serve(listener, service).await.expect("the listener serves");
        });
        (app, Listening { port, server }, token)
    }

    async fn set_status(app: &Arc<App>, profile_id: &str, status: ProfileStatus) {
        let mut tx = app.db.begin(Some(profile_id)).await.expect("a transaction opens");
        tx.profiles_set_status(profile_id, status)
            .await
            .expect("the status is set");
        tx.commit().await.expect("the transaction commits");
    }

    #[tokio::test]
    async fn banned_profile_is_refused_the_socket_and_unbanned_is_opened() {
        let _serial = SERIAL.lock().await;
        let (app, listening, token) = listen_bannable().await;

        // Banned through the store: the handshake completes, then the socket is refused with an
        // error frame and the forbidden close — `assert_active` runs after the upgrade
        // (`upgrade_socket`), so there is no HTTP status to read here, only no socket and no view.
        set_status(&app, "profile-1", ProfileStatus::Banned).await;
        let mut refused = open(listening.port, &query("match-1", &token), &[WS_SUBPROTOCOL])
            .await
            .open();
        let refusal = refused
            .next_message(READ_TIMEOUT)
            .await
            .expect("an error frame before the close");
        assert_eq!(refusal["type"], "error");
        assert_eq!(refused.close_code().await, 4403);

        // Unbanned: the same handshake opens onto the seat's first view.
        set_status(&app, "profile-1", ProfileStatus::Active).await;
        let mut client = open(listening.port, &query("match-1", &token), &[WS_SUBPROTOCOL])
            .await
            .open();
        assert_eq!(client.first_view().await["viewer"], "p1");
    }
}

mod the_graceful_shutdown {
    //! SIGTERM (and Ctrl-C) drains instead of cutting (#861): an in-flight request finishes, every
    //! match socket gets close frame 1001, the background loops stop, and the process exits 0
    //! inside the drain bound — or the drain stops waiting at the bound for a socket that never
    //! answers the close.
    use super::*;

    use jackioh_server::app::{serve_until, shutdown_signal};
    use jackioh_server::config::SHUTDOWN_DRAIN_SECONDS;

    use crate::support::deps::record_logs;

    /// One raw HTTP request, left open for its answer.
    async fn request(port: u16, path: &str) -> TcpStream {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("the listener accepts");
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .expect("the request is written");
        stream
    }

    /// The request's status, once the server answers and closes.
    async fn status_of(mut stream: TcpStream) -> u16 {
        let mut raw = Vec::new();
        tokio::time::timeout(READ_TIMEOUT, stream.read_to_end(&mut raw))
            .await
            .expect("the server answers the request")
            .expect("the request's bytes read");
        String::from_utf8_lossy(&raw)
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0)
    }

    #[tokio::test]
    async fn a_sigterm_lets_the_request_in_flight_finish_closes_every_socket_with_1001_stops_the_loops_and_returns()
     {
        let _serial = SERIAL.lock().await;
        let logs = record_logs();
        let shutdown = shutdown_signal().expect("the shutdown signal installs");
        let drain = Duration::from_secs(u64::try_from(SHUTDOWN_DRAIN_SECONDS).unwrap_or(0));
        let (app, tokens) = seed_matches(1).await;
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let port = listener.local_addr().expect("a bound address").port();
        let server = tokio::spawn(serve_until(Arc::clone(&app), listener, shutdown, drain));
        let mut client = open(port, &query("match-1", &tokens[0]), &[WS_SUBPROTOCOL])
            .await
            .open();
        assert_eq!(client.first_view().await["viewer"], "p1");

        // Hold the fake store's lock so the readiness probe's `ping` cannot answer: the request
        // below is in flight while SIGTERM arrives.
        let tables = fake(&app);
        let guard = tables.lock().await;
        let inflight = request(port, "/readyz").await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let started = std::time::Instant::now();
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(format!("kill -TERM {}", std::process::id()))
            .status()
            .await
            .expect("the kill runs");
        let deadline = tokio::time::Instant::now() + READ_TIMEOUT;
        while logs.named("server.draining").is_empty() {
            assert!(tokio::time::Instant::now() < deadline, "the drain starts");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        drop(guard);
        assert_eq!(status_of(inflight).await, 200);
        assert_eq!(client.close_code().await, 1001);
        client.send_frame(0x8, &1000u16.to_be_bytes()).await;
        tokio::time::timeout(drain, server)
            .await
            .expect("serve_until ends inside the drain")
            .expect("serve_until's task ends")
            .expect("serve_until answers Ok");
        assert!(started.elapsed() < drain, "the drain ends inside its bound");
        let stopped: Vec<String> = logs
            .named("loop.stopped")
            .iter()
            .map(|entry| entry.data["loop"].as_str().unwrap_or("").to_string())
            .collect();
        assert_eq!(
            stopped,
            vec![
                "matchmaker".to_string(),
                "series.sweeper".to_string(),
                "matches.reaper".to_string(),
                "retention.purge".to_string(),
            ]
        );
        let halted = logs.named("server.stopped");
        assert_eq!(halted.len(), 1);
        assert_eq!(halted[0].data["matches"], json!(1));
        assert!(logs.named("server.drain_timed_out").is_empty());
    }

    #[tokio::test]
    async fn stops_waiting_at_the_drain_bound_for_a_socket_that_never_answers_the_close() {
        let _serial = SERIAL.lock().await;
        let logs = record_logs();
        let (stop, waiting) = tokio::sync::oneshot::channel::<()>();
        let drain = Duration::from_millis(500);
        let (app, tokens) = seed_matches(1).await;
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let port = listener.local_addr().expect("a bound address").port();
        let server = tokio::spawn(serve_until(
            Arc::clone(&app),
            listener,
            async move {
                let _ = waiting.await;
            },
            drain,
        ));
        let mut client = open(port, &query("match-1", &tokens[0]), &[WS_SUBPROTOCOL])
            .await
            .open();
        assert_eq!(client.first_view().await["viewer"], "p1");

        let started = std::time::Instant::now();
        stop.send(()).expect("serve_until waits for the shutdown");
        tokio::time::timeout(READ_TIMEOUT, server)
            .await
            .expect("serve_until ends")
            .expect("serve_until's task ends")
            .expect("serve_until answers Ok");
        assert!(started.elapsed() >= drain, "the drain waits out its bound");
        assert_eq!(client.close_code().await, 1001);
        assert_eq!(logs.named("server.drain_timed_out").len(), 1);
        assert!(logs.named("server.stopped").is_empty());
    }
}
