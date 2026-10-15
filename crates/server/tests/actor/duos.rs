//! B35–B42 (#936, `.fullsend/SPEC.md` S6) through `GET /ws/duos`: the hello view, applied and
//! illegal actions, the AI seats, the disconnect takeover, the round clock, the mulligan clock, pings
//! (relayed to the teammate only, shape-checked, rate-limited, never in a view or the replay), the
//! end of a game and the per-match replay, which a finished game has (§13.9). Where a test reads
//! what was logged while the game is live, it reads the store's log.
//!
//! The real router is served over in-memory pipes (`MemoryListener`, as `actor/heartbeat.rs` does,
//! since real TCP races a paused clock), so the upgrade, the auth (`?token=`) and the frames are the
//! wire's. Each client is a small RFC 6455 client whose reader task answers the server's pings and
//! stamps every text frame with the paused clock's instant. A match is made the way players make one:
//! `POST /api/duos/rooms`, then the join; its seats' decks are dealt at random (`random: true`) over
//! the real catalog, and the AI plays B1 and B2.
//!
//! The quiet window. Once both AI seats have answered their mulligans (each human sees
//! `own.mulligan.opponentReady` and `teammate.mulligan.opponentReady`), nothing is owed by the AI
//! and nothing moves until a human acts or `DUOS_MULLIGAN_CLOCK_MS` runs out: the tests that assert
//! that something did NOT happen run there.
//!
//! The WebSocket harness and the account helpers are `pub(super)`: `actor/watch.rs` uses them.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};
use tokio::sync::mpsc;
use tokio::time::Instant;

use jackioh_engine::config::{
    DECK_SIZE, DUOS_OWN_HEALTH, DUOS_PASS_COST, DUOS_POOL_HEALTH, Difficulty, UNIT_ZONES, duos_ai_team,
};
use jackioh_engine::wire::DuosSeat;
use jackioh_server::actor::ws_server::{Socket, SocketFrame};
use jackioh_server::app::{App, now_ms, router};
use jackioh_server::config::{
    DISCONNECT_GRACE_MS, DUOS_AI_STEP_DELAY_MS, DUOS_MULLIGAN_CLOCK_MS, DUOS_ROUND_CLOCK_MS, MAX_SAVED_DECKS,
    PING_RATE_LIMIT, PING_RATE_WINDOW_MS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, Db};
use jackioh_server::duos::registry;
use jackioh_server::duos::store::DuosActionRow;

use super::ws_server::encode_query;
use crate::support::deps::{add_user, call, empty_test_app};

/// S6's socket.
const DUOS_PATH: &str = "/ws/duos";

/// The pipe's buffer each way. A reader task drains it continuously; the size only spares a burst
/// of views a wait.
const PIPE_BYTES: usize = 1 << 20;
/// How many times `settle` yields: enough for a frame to cross the pipe, the pump, the actor and back.
const SETTLE_YIELDS: usize = 256;
/// How far `wait_for` moves the paused clock between two looks.
const WAIT_STEP: Duration = Duration::from_millis(50);
/// How long a test waits for a frame the server answers at once (no timer stands between). Found
/// at the first look when all is well, so the clock does not move.
pub(super) const PROMPTLY: Duration = Duration::from_secs(1);

const TEXT: u8 = 0x1;
const CLOSE: u8 = 0x8;
const PING: u8 = 0x9;
const PONG: u8 = 0xA;
/// The close code a transport reports when the stream ends without a close frame.
const ABNORMAL_CLOSE: u16 = 1006;
/// R148's close for a match the socket may not have.
const CLOSE_NOT_FOUND: u16 = 4404;
/// The epoch-ms stamp a saved deck carries; nothing reads it back.
const SAVED_AT: i64 = 1_700_000_000_000;
/// A close frame that carried no code.
const NO_STATUS: u16 = 1005;

// ---------------------------------------------------------------------------------------------
// Time and numbers
// ---------------------------------------------------------------------------------------------

/// Lets every task woken at this instant run.
pub(super) async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// A config duration, whichever integer type `config.rs` gives it.
pub(super) fn ms<T>(value: T) -> Duration
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    Duration::from_millis(u64::try_from(value).expect("a duration fits a u64"))
}

/// A config number as an `i64`, whichever integer type `config.rs` gives it.
fn int<T>(value: T) -> i64
where
    i64: TryFrom<T>,
    <i64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    i64::try_from(value).expect("a number fits an i64")
}

fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

/// The server's clock (`app::now_ms`, epoch ms) at a past instant of the paused clock.
fn epoch_at(at: Instant) -> i64 {
    let since = Instant::now().saturating_duration_since(at);
    now_ms() - i64::try_from(since.as_millis()).expect("a short span")
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------------------------
// The app served over in-memory pipes
// ---------------------------------------------------------------------------------------------

/// The listener `axum::serve` accepts from: each pipe a test sends is a connection.
struct MemoryListener(mpsc::UnboundedReceiver<DuplexStream>);

impl axum::serve::Listener for MemoryListener {
    type Io = DuplexStream;
    type Addr = ();

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.0.recv().await {
            Some(stream) => (stream, ()),
            None => std::future::pending::<(DuplexStream, ())>().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(())
    }
}

/// The whole app (`app::router`) served over `MemoryListener`.
pub(super) struct Served {
    streams: mpsc::UnboundedSender<DuplexStream>,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub(super) fn serve(app: &Arc<App>) -> Served {
    let (streams, incoming) = mpsc::unbounded_channel();
    let service = router(Arc::clone(app));
    let server = tokio::spawn(async move {
        axum::serve(MemoryListener(incoming), service)
            .await
            .expect("the listener serves");
    });
    Served { streams, server }
}

impl Served {
    /// The upgrade of `path?token=…` over a new pipe; panics unless the server answers 101.
    pub(super) async fn connect(&self, path: &str, token: &str) -> WsClient {
        let (mut stream, server_end) = tokio::io::duplex(PIPE_BYTES);
        self.streams.send(server_end).expect("the listener is serving");
        let request = format!(
            "GET {path}?token={} HTTP/1.1\r\nHost: test\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
            encode_query(token)
        );
        stream
            .write_all(request.as_bytes())
            .await
            .expect("the handshake is written");
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            let read = stream
                .read(&mut byte)
                .await
                .expect("the server answers the handshake");
            assert_ne!(read, 0, "the server ended the connection during the handshake");
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head);
        assert!(
            head.starts_with("HTTP/1.1 101"),
            "the upgrade of {path} was not accepted: {head}"
        );
        WsClient::start(stream)
    }
}

// ---------------------------------------------------------------------------------------------
// A WebSocket client
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct Inbox {
    /// Every text frame, parsed, with the paused clock's instant it arrived at.
    frames: Vec<(Instant, Value)>,
    /// The server's close code, or 1006 once the stream ended without one.
    close: Option<u16>,
}

type Writer = Arc<tokio::sync::Mutex<Option<WriteHalf<DuplexStream>>>>;

/// One client connection. Its reader task records every frame and answers every ping.
pub(super) struct WsClient {
    writer: Writer,
    inbox: Arc<Mutex<Inbox>>,
    reader: tokio::task::JoinHandle<()>,
}

impl Drop for WsClient {
    fn drop(&mut self) {
        self.reader.abort();
    }
}

/// One masked frame, as a client must send them. False when the pipe is gone.
async fn write_frame(write: &mut WriteHalf<DuplexStream>, opcode: u8, payload: &[u8]) -> bool {
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
    write.write_all(&frame).await.is_ok()
}

/// The next frame as it stands on the wire (server frames are unmasked), or `None` at the end.
async fn read_frame<R: AsyncRead + Unpin>(read: &mut R) -> Option<(u8, Vec<u8>)> {
    let mut head = [0u8; 2];
    read.read_exact(&mut head).await.ok()?;
    let opcode = head[0] & 0x0F;
    let mut length = u64::from(head[1] & 0x7F);
    if length == 126 {
        let mut extended = [0u8; 2];
        read.read_exact(&mut extended).await.ok()?;
        length = u64::from(u16::from_be_bytes(extended));
    } else if length == 127 {
        let mut extended = [0u8; 8];
        read.read_exact(&mut extended).await.ok()?;
        length = u64::from_be_bytes(extended);
    }
    let mut payload = vec![0u8; usize::try_from(length).ok()?];
    read.read_exact(&mut payload).await.ok()?;
    Some((opcode, payload))
}

async fn pump(mut read: ReadHalf<DuplexStream>, writer: Writer, inbox: Arc<Mutex<Inbox>>) {
    loop {
        let Some((opcode, payload)) = read_frame(&mut read).await else {
            lock(&inbox).close.get_or_insert(ABNORMAL_CLOSE);
            return;
        };
        match opcode {
            TEXT => {
                let frame = serde_json::from_slice(&payload)
                    .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&payload).into_owned()));
                lock(&inbox).frames.push((Instant::now(), frame));
            }
            CLOSE => {
                let code = if payload.len() >= 2 {
                    u16::from_be_bytes([payload[0], payload[1]])
                } else {
                    NO_STATUS
                };
                lock(&inbox).close.get_or_insert(code);
                return;
            }
            PING => {
                let mut guard = writer.lock().await;
                if let Some(write) = guard.as_mut() {
                    write_frame(write, PONG, &payload).await;
                }
            }
            _ => {}
        }
    }
}

impl WsClient {
    fn start(stream: DuplexStream) -> WsClient {
        let (read, write) = tokio::io::split(stream);
        let writer: Writer = Arc::new(tokio::sync::Mutex::new(Some(write)));
        let inbox = Arc::new(Mutex::new(Inbox::default()));
        let reader = tokio::spawn(pump(read, Arc::clone(&writer), Arc::clone(&inbox)));
        WsClient {
            writer,
            inbox,
            reader,
        }
    }

    pub(super) async fn send_text(&self, text: &str) {
        let mut guard = self.writer.lock().await;
        if let Some(write) = guard.as_mut() {
            write_frame(write, TEXT, text.as_bytes()).await;
        }
    }

    pub(super) async fn send_json(&self, value: Value) {
        self.send_text(&value.to_string()).await;
    }

    /// How many text frames have arrived: an index to read on from.
    pub(super) fn received(&self) -> usize {
        lock(&self.inbox).frames.len()
    }

    pub(super) fn frames(&self) -> Vec<Value> {
        lock(&self.inbox)
            .frames
            .iter()
            .map(|(_, frame)| frame.clone())
            .collect()
    }

    pub(super) fn timed(&self) -> Vec<(Instant, Value)> {
        lock(&self.inbox).frames.clone()
    }

    /// The frames of one `type` from index `from` on.
    pub(super) fn of_type_since(&self, from: usize, kind: &str) -> Vec<Value> {
        self.frames()
            .into_iter()
            .skip(from)
            .filter(|frame| frame["type"] == kind)
            .collect()
    }

    pub(super) fn of_type(&self, kind: &str) -> Vec<Value> {
        self.of_type_since(0, kind)
    }

    /// The server's close code, `Some(1006)` once the stream ended without one, `None` while open.
    pub(super) fn close_code(&self) -> Option<u16> {
        lock(&self.inbox).close
    }

    /// The first frame from index `from` on that `pred` takes, waiting up to `within` on the paused
    /// clock: `(index, instant it arrived, frame)`.
    pub(super) async fn wait_for(
        &self,
        from: usize,
        within: Duration,
        pred: impl Fn(&Value) -> bool,
    ) -> Option<(usize, Instant, Value)> {
        let deadline = Instant::now() + within;
        loop {
            settle().await;
            let found = lock(&self.inbox)
                .frames
                .iter()
                .enumerate()
                .skip(from)
                .find(|(_, (_, frame))| pred(frame))
                .map(|(index, (at, frame))| (index, *at, frame.clone()));
            if found.is_some() {
                return found;
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            tokio::time::sleep(WAIT_STEP.min(deadline - now)).await;
        }
    }

    /// Waits up to `within` for the connection to end.
    pub(super) async fn wait_closed(&self, within: Duration) -> Option<u16> {
        let deadline = Instant::now() + within;
        loop {
            settle().await;
            if let Some(code) = self.close_code() {
                return Some(code);
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            tokio::time::sleep(WAIT_STEP.min(deadline - now)).await;
        }
    }

    /// The transport drops: no close frame, the server reads the end of the stream.
    pub(super) async fn hang_up(&self) {
        self.reader.abort();
        *self.writer.lock().await = None;
        settle().await;
    }
}

// ---------------------------------------------------------------------------------------------
// Accounts and requests
// ---------------------------------------------------------------------------------------------

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

pub(super) fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// One signed-in account: its profile id, its username as shown (`name#tag`) and its token.
pub(super) struct Account {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) token: String,
}

/// An active account, username `base#n`.
pub(super) async fn account(app: &Arc<App>, n: u32, base: &str) -> Account {
    let id = uuid(n);
    let user_id = format!("user-{n}");
    let tag = i64::from(n);
    fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": "active",
        "username": base,
        "usernameTag": tag,
        "usernamePrompted": true,
    }));
    let token = add_user(app, &user_id, &format!("{user_id}@example.test"), true);
    Account {
        id,
        name: format!("{base}#{tag}"),
        token,
    }
}

pub(super) async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

// ---------------------------------------------------------------------------------------------
// A Duos match and its frames
// ---------------------------------------------------------------------------------------------

fn hello(match_id: &str) -> Value {
    json!({ "type": "hello", "matchId": match_id })
}

fn action(nonce: &str, body: Value) -> Value {
    json!({ "type": "action", "nonce": nonce, "body": body })
}

fn ping(kind: &str, target: Value) -> Value {
    json!({ "type": "ping", "ping": kind, "target": target })
}

fn relayed(from: &str, kind: &str, target: &Value) -> Value {
    json!({ "type": "ping", "from": from, "ping": kind, "target": target })
}

fn concede() -> Value {
    json!({ "kind": "concede" })
}

fn duel(body: Value) -> Value {
    json!({ "kind": "duel", "body": body })
}

/// Whether a legal entry is the duel action of `ty`.
fn is_duel(entry: &Value, ty: &str) -> bool {
    entry["kind"] == "duel" && entry["body"]["type"] == ty
}

fn legal(frame: &Value) -> Vec<Value> {
    frame["legal"].as_array().cloned().unwrap_or_default()
}

/// A view frame's round, or -1 for any other frame.
fn round_of(frame: &Value) -> i64 {
    if frame["type"] == "view" {
        frame["view"]["round"].as_i64().unwrap_or(-1)
    } else {
        -1
    }
}

/// The cards in a `PlayerView`'s own hand.
fn hand_len(player_view: &Value) -> usize {
    player_view["you"]["hand"].as_array().map_or(0, Vec::len)
}

/// The instance ids of a `PlayerView`'s own hand.
fn hand_ids(player_view: &Value) -> Vec<String> {
    player_view["you"]["hand"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| card["instanceId"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The first `DECK_SIZE` playable catalog cards `wanted` takes, in `catalog.json` order: a legal deck.
fn deck_where(wanted: impl Fn(&Value) -> bool) -> Vec<String> {
    jackioh_cards::register_all();
    let size = count(DECK_SIZE);
    let cards: Vec<String> = jackioh_cards::CATALOG
        .values()
        .map(|def| serde_json::to_value(def).expect("a card serialises"))
        .filter(|def| {
            def["token"] != json!(true)
                && !def["tags"]
                    .as_array()
                    .is_some_and(|tags| tags.contains(&json!("Token")))
        })
        .filter(|def| wanted(def))
        .filter_map(|def| def["id"].as_str().map(str::to_string))
        .take(size)
        .collect();
    assert_eq!(cards.len(), size, "the catalog holds a deck of them");
    cards
}

/// `owner` owns every card of `cards` and keeps them as their saved deck `deck_id`.
async fn save_deck(app: &Arc<App>, owner: &Account, deck_id: &str, cards: &[String]) {
    let rows: Vec<Value> = cards
        .iter()
        .map(|id| json!({ "cardId": id, "quantity": 1 }))
        .collect();
    let entries: Vec<CollectionEntry> = serde_json::from_value(Value::Array(rows)).expect("entries");
    let deck = serde_json::from_value(json!({
        "id": deck_id,
        "profileId": owner.id,
        "name": "co-op deck",
        "cards": cards,
        "portrait": null,
        "catalogVersion": jackioh_cards::catalog_version(),
        "createdAt": SAVED_AT,
        "updatedAt": SAVED_AT,
    }))
    .expect("a saved deck");
    let mut t = app.db.begin(None).await.expect("a transaction");
    t.collection_upsert_quantities(&owner.id, &entries)
        .await
        .expect("collection.upsertQuantities");
    t.decks_upsert(&deck, MAX_SAVED_DECKS as _)
        .await
        .expect("decks.upsert");
    t.commit().await.expect("the commit");
}

/// The text frames a test socket was sent since it was last drained, parsed.
fn drain(frames: &mut mpsc::Receiver<SocketFrame>) -> Vec<Value> {
    let mut out = Vec::new();
    while let Ok(frame) = frames.try_recv() {
        if let SocketFrame::Text(text) = frame {
            out.push(serde_json::from_str(&text).unwrap_or(Value::String(text)));
        }
    }
    out
}

fn views_in(frames: &[Value]) -> usize {
    frames.iter().filter(|frame| frame["type"] == "view").count()
}

/// The mulligan entry of `legal` that keeps the most cards.
fn keep_whole_hand(frame: &Value) -> Option<Value> {
    legal(frame)
        .into_iter()
        .filter(|entry| is_duel(entry, "mulligan"))
        .max_by_key(|entry| entry["body"]["keep"].as_array().map_or(0, Vec::len))
}

/// Whether a duel's `PlayerView` shows its viewer's mulligan answered: still in the mulligan with
/// `youReady`, or past it. B1: each duel is a normal 1v1 setup, so a duel whose two players have
/// answered leaves its mulligan at once; B2 holds back only main-phase actions until all four have.
fn mulligan_answered(player_view: &Value) -> bool {
    player_view["mulligan"]["youReady"] == json!(true)
        || (player_view["phase"].is_string() && player_view["phase"] != "mulligan")
}

fn last_view(client: &WsClient) -> Value {
    client
        .of_type("view")
        .last()
        .cloned()
        .expect("a view frame has arrived")
}

fn first_view(client: &WsClient) -> Value {
    client
        .of_type("view")
        .first()
        .cloned()
        .expect("a view frame has arrived")
}

/// Sends an action and answers the frame that settled it: its `ack` or its `error`.
async fn act(client: &WsClient, nonce: &str, body: Value) -> Value {
    let from = client.received();
    client.send_json(action(nonce, body)).await;
    client
        .wait_for(from, PROMPTLY, |frame| {
            (frame["type"] == "ack" || frame["type"] == "error") && frame["nonce"] == nonce
        })
        .await
        .map(|(_, _, frame)| frame)
        .unwrap_or_else(|| panic!("nothing answered {nonce}: {:?}", client.frames()))
}

/// A Duos match made by its humans over HTTP; nobody connected yet.
struct Duos {
    app: Arc<App>,
    served: Served,
    host: Account,
    guest: Account,
    outsider: Account,
    match_id: String,
}

async fn duos(difficulty: &str) -> Duos {
    duos_with(difficulty, None).await
}

/// A Duos match whose host brings `host_deck` as a saved deck (dealt at random when `None`).
async fn duos_with(difficulty: &str, host_deck: Option<Vec<String>>) -> Duos {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    let host = account(&app, 1, "Hana").await;
    let guest = account(&app, 2, "Gus").await;
    let outsider = account(&app, 3, "Otto").await;
    let room = match &host_deck {
        Some(cards) => {
            let deck_id = uuid(800);
            save_deck(&app, &host, &deck_id, cards).await;
            json!({ "difficulty": difficulty, "deckId": deck_id })
        }
        None => json!({ "difficulty": difficulty, "random": true }),
    };
    let (status, created) = api(&app, "POST", "/api/duos/rooms", &host.token, room).await;
    assert_eq!(status, 201, "{created}");
    let code = created["code"].as_str().expect("a room code").to_string();
    let (status, joined) = api(
        &app,
        "POST",
        &format!("/api/duos/rooms/{code}/join"),
        &guest.token,
        json!({ "random": true }),
    )
    .await;
    assert_eq!(status, 200, "{joined}");
    let match_id = joined["matchId"].as_str().expect("a match id").to_string();
    let served = serve(&app);
    Duos {
        app,
        served,
        host,
        guest,
        outsider,
        match_id,
    }
}

impl Duos {
    /// `who`'s socket, said hello, with its first view in.
    async fn seat(&self, who: &Account) -> WsClient {
        let client = self.served.connect(DUOS_PATH, &who.token).await;
        client.send_json(hello(&self.match_id)).await;
        client
            .wait_for(0, PROMPTLY, |frame| frame["type"] == "view")
            .await
            .unwrap_or_else(|| panic!("hello pushed no view: {:?}", client.frames()));
        client
    }

    /// Waits for the quiet window: both AI seats have answered their mulligans, before the
    /// mulligan clock could have answered them. Answers the instant the human saw it.
    async fn ai_mulligans_answered(&self, human: &WsClient) -> Instant {
        let within = ms(DUOS_MULLIGAN_CLOCK_MS) - Duration::from_millis(1);
        let (_, at, _) = human
            .wait_for(0, within, |frame| {
                frame["type"] == "view"
                    && frame["view"]["own"]["mulligan"]["opponentReady"] == true
                    && frame["view"]["teammate"]["mulligan"]["opponentReady"] == true
            })
            .await
            .unwrap_or_else(|| {
                panic!(
                    "the AI seats never answered their mulligans: {:?}",
                    human.frames()
                )
            });
        settle().await;
        at
    }

    /// The human keeps their whole opening hand.
    async fn keep(&self, human: &WsClient, nonce: &str) {
        let keep = keep_whole_hand(&last_view(human)).expect("the human owes a mulligan");
        let answer = act(human, nonce, keep).await;
        assert_eq!(answer["type"], "ack", "{answer}");
    }

    /// The match's log as the store holds it.
    async fn logged(&self) -> Vec<DuosActionRow> {
        let mut t = self.app.db.begin(None).await.expect("a transaction");
        let log = t.duos_actions(&self.match_id).await.expect("the log");
        t.commit().await.expect("the commit");
        log
    }

    /// Both people concede (B11), which ends the game: its replay opens (§13.9).
    async fn concede_both(&self, a1: &WsClient, a2: &WsClient) {
        for (client, nonce) in [(a1, "a1-concede"), (a2, "a2-concede")] {
            let answer = act(client, nonce, concede()).await;
            assert_eq!(answer["type"], "ack", "{answer}");
        }
        settle().await;
    }

    async fn replay(&self, who: &Account) -> Value {
        let (status, body) = api(
            &self.app,
            "GET",
            &format!("/api/duos/matches/{}/replay", self.match_id),
            &who.token,
            Value::Null,
        )
        .await;
        assert_eq!(status, 200, "{body}");
        body
    }

    async fn steps(&self, who: &Account) -> Vec<Value> {
        self.replay(who).await["steps"]
            .as_array()
            .cloned()
            .expect("a replay's steps")
    }

    async fn live(&self, who: &Account) -> Value {
        let (status, body) = api(&self.app, "GET", "/api/duos/live", &who.token, Value::Null).await;
        assert_eq!(status, 200, "{body}");
        body["matchId"].clone()
    }
}

// ---------------------------------------------------------------------------------------------
// B35 — hello, applied and refused actions
// ---------------------------------------------------------------------------------------------

mod b35_the_socket {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b35_hello_pushes_each_humans_seat_view_and_legal_actions_before_anything_is_played() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;

        for (client, seat) in [(&a1, "a1"), (&a2, "a2")] {
            let frame = first_view(client);
            let view = &frame["view"];
            assert_eq!(view["seat"], seat, "{view}");
            assert_eq!(view["team"]["team"], "a", "{view}");
            assert_eq!(view["enemy"]["team"], "b", "{view}");
            // Team A is P1 in both duels; the teammate's own view is from the teammate's seat.
            assert_eq!(view["own"]["viewer"], "p1", "{view}");
            assert_eq!(view["teammate"]["viewer"], "p1", "{view}");
            // B2: no round until all four mulligans are in.
            assert_eq!(view["round"], json!(0), "{view}");
            assert!(view["result"].is_null(), "{view}");
            // B12: the humans' team plays DUOS_HUMAN_TEAM's rules.
            assert_eq!(view["team"]["pool"], json!(DUOS_POOL_HEALTH), "{view}");
            assert_eq!(view["team"]["poolCap"], json!(DUOS_POOL_HEALTH), "{view}");
            assert_eq!(view["team"]["passCost"], json!(DUOS_PASS_COST), "{view}");
            assert_eq!(
                view["team"]["sides"],
                json!([
                    { "own": DUOS_OWN_HEALTH, "spent": false },
                    { "own": DUOS_OWN_HEALTH, "spent": false },
                ]),
                "{view}"
            );
            let offered = legal(&frame);
            assert!(offered.contains(&concede()), "{frame}");
            assert!(offered.iter().any(|entry| is_duel(entry, "mulligan")), "{frame}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b33_the_ai_team_plays_by_duos_ai_team_of_the_rooms_difficulty() {
        for (difficulty, tier) in [
            ("easy", Difficulty::Easy),
            ("medium", Difficulty::Medium),
            ("hard", Difficulty::Hard),
        ] {
            let h = duos(difficulty).await;
            let a1 = h.seat(&h.host).await;
            let rules = duos_ai_team(tier);

            let view = first_view(&a1)["view"].clone();

            let enemy = &view["enemy"];
            assert_eq!(enemy["pool"], json!(rules.pool_health), "{difficulty}: {enemy}");
            assert_eq!(
                enemy["poolCap"],
                json!(rules.pool_health),
                "{difficulty}: {enemy}"
            );
            assert_eq!(enemy["passCost"], json!(rules.pass_cost), "{difficulty}: {enemy}");
            for side in enemy["sides"].as_array().expect("the enemy's sides") {
                assert_eq!(side["own"], json!(rules.own_health), "{difficulty}: {enemy}");
                assert_eq!(side["spent"], json!(false), "{difficulty}: {enemy}");
            }
            // B23: what the enemy team has ended and passed is not shown.
            assert_eq!(enemy["ended"], json!([false, false]), "{difficulty}: {enemy}");
            assert_eq!(enemy["passesLeft"], json!([0, 0]), "{difficulty}: {enemy}");
            // The humans' rules do not move with the difficulty.
            assert_eq!(view["team"]["pool"], json!(DUOS_POOL_HEALTH), "{difficulty}");
            assert_eq!(view["team"]["passCost"], json!(DUOS_PASS_COST), "{difficulty}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b35_an_applied_action_is_acked_to_its_sender_and_pushes_fresh_views_to_both_humans() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let (a1_from, a2_from) = (a1.received(), a2.received());
        let keep = keep_whole_hand(&last_view(&a1)).expect("A1 owes its mulligan");

        let answer = act(&a1, "a1-keep", keep).await;

        assert_eq!(answer["type"], "ack", "{answer}");
        assert_eq!(answer["nonce"], "a1-keep");
        assert!(answer["seq"].is_number(), "{answer}");
        settle().await;
        let mine = a1.of_type_since(a1_from, "view");
        let theirs = a2.of_type_since(a2_from, "view");
        let mine = mine.last().expect("A1 got a fresh view");
        let theirs = theirs.last().expect("A2 got a fresh view");
        assert!(mulligan_answered(&mine["view"]["own"]), "{mine}");
        assert!(
            !legal(mine).iter().any(|entry| is_duel(entry, "mulligan")),
            "{mine}"
        );
        // A2 sees its teammate's duel from the teammate's seat: A1 has answered.
        assert!(mulligan_answered(&theirs["view"]["teammate"]), "{theirs}");
        assert!(
            legal(theirs).iter().any(|entry| is_duel(entry, "mulligan")),
            "A2 still owes its own: {theirs}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn b35_an_illegal_action_answers_illegal_action_with_its_nonce_and_changes_nothing() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let logged = h.logged().await.len();
        let (a1_from, a2_from) = (a1.received(), a2.received());

        for (nonce, body) in [
            // B2: no main-phase action before the round opens.
            ("early-end", duel(json!({ "type": "endTurn" }))),
            // B19: a pass names a card in the passer's hand.
            (
                "ghost-pass",
                json!({ "kind": "pass", "instanceId": "no-such-card" }),
            ),
        ] {
            let answer = act(&a1, nonce, body).await;
            assert_eq!(answer["type"], "error", "{nonce}: {answer}");
            assert_eq!(answer["code"], "illegal_action", "{nonce}: {answer}");
            assert_eq!(answer["nonce"], nonce, "{answer}");
        }

        settle().await;
        assert!(a1.of_type_since(a1_from, "ack").is_empty());
        assert!(
            a2.of_type_since(a2_from, "view").is_empty(),
            "nothing changed for the teammate"
        );
        assert!(
            legal(&last_view(&a1))
                .iter()
                .any(|entry| is_duel(entry, "mulligan")),
            "A1 still owes its mulligan"
        );
        assert_eq!(h.logged().await.len(), logged, "nothing was logged");
    }

    #[tokio::test(start_paused = true)]
    async fn b35_b10_a_duel_concede_or_a_draw_offer_is_refused_as_illegal() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let a2_from = a2.received();

        for (nonce, body) in [
            ("duel-concede", duel(json!({ "type": "concede" }))),
            ("draw-offer", duel(json!({ "type": "offerDraw" }))),
            (
                "draw-answer",
                duel(json!({ "type": "answerDraw", "accept": true })),
            ),
        ] {
            let answer = act(&a1, nonce, body).await;
            assert_eq!(answer["type"], "error", "{nonce}: {answer}");
            assert_eq!(answer["code"], "illegal_action", "{nonce}: {answer}");
        }

        settle().await;
        assert!(a2.of_type_since(a2_from, "view").is_empty());
        let view = last_view(&a1);
        assert!(view["view"]["result"].is_null(), "{view}");
        assert!(view["view"]["concedeOfferedBy"].is_null(), "{view}");
    }

    #[tokio::test(start_paused = true)]
    async fn b35_a_frame_that_is_no_message_answers_malformed_and_changes_nothing() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let (a1_from, a2_from) = (a1.received(), a2.received());

        a1.send_text("this is not json").await;
        a1.send_json(json!({ "type": "action", "nonce": "no-body" }))
            .await;
        a1.send_json(json!({ "type": "teleport" })).await;
        settle().await;

        let errors = a1.of_type_since(a1_from, "error");
        assert_eq!(errors.len(), 3, "{errors:?}");
        for error in &errors {
            assert_eq!(error["code"], "malformed", "{error}");
        }
        assert!(a1.of_type_since(a1_from, "ack").is_empty());
        assert!(a2.of_type_since(a2_from, "view").is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn b35_a_hello_for_a_match_the_caller_holds_no_seat_in_gets_no_view() {
        let h = duos("easy").await;
        let outsider = h.served.connect(DUOS_PATH, &h.outsider.token).await;
        outsider.send_json(hello(&h.match_id)).await;
        let host = h.served.connect(DUOS_PATH, &h.host.token).await;
        host.send_json(hello(&uuid(999))).await;

        for client in [&outsider, &host] {
            let view = client
                .wait_for(0, PROMPTLY, |frame| frame["type"] == "view")
                .await;
            assert!(view.is_none(), "{:?}", client.frames());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B36 — the AI seats act
// ---------------------------------------------------------------------------------------------

mod b36_the_ai_seats {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b36_the_ai_seats_answer_their_own_mulligans_before_the_mulligan_clock_and_leave_the_humans_theirs()
     {
        let h = duos("easy").await;
        let started = Instant::now();
        let a1 = h.seat(&h.host).await;

        let at = h.ai_mulligans_answered(&a1).await;

        assert!(at < started + ms(DUOS_MULLIGAN_CLOCK_MS));
        let frame = last_view(&a1);
        assert_eq!(frame["view"]["round"], json!(0), "{frame}");
        assert!(
            legal(&frame).iter().any(|entry| is_duel(entry, "mulligan")),
            "{frame}"
        );
    }

    /// The first view of `round` or later `a1` has had since frame `from`, waited for up to `within`
    /// while the people play along as a person at the table does: each answers a prompt it holds
    /// (the first answer offered), else ends its turn once its legal actions hold that. A prompt a
    /// person holds waits for that person, on a clock of its own (DUOS_PROMPT_CLOCK_MS, §13.3), and
    /// a duel whose new active player is a person's seat gives that person the round seat (§13.3,
    /// R456: a turn cut short as it begins): neither is a step of the AI's. `acted` counts the
    /// people's actions by the instant they were taken, so the views they caused are told apart.
    async fn round_reached(
        a1: &WsClient,
        a2: &WsClient,
        from: usize,
        round: i64,
        within: Duration,
        acted: &mut BTreeMap<Instant, usize>,
    ) -> Option<(usize, Instant)> {
        let deadline = Instant::now() + within;
        loop {
            settle().await;
            let reached = a1
                .timed()
                .iter()
                .enumerate()
                .skip(from)
                .find(|(_, (_, frame))| round_of(frame) >= round)
                .map(|(index, (at, _))| (index, *at));
            if reached.is_some() {
                return reached;
            }
            let mut moved = false;
            for (client, seat) in [(a1, "a1"), (a2, "a2")] {
                let offered = legal(&last_view(client));
                let Some(body) = offered
                    .iter()
                    .find(|entry| is_duel(entry, "answer"))
                    .or_else(|| offered.iter().find(|entry| is_duel(entry, "endTurn")))
                    .cloned()
                else {
                    continue;
                };
                let taken: usize = acted.values().sum();
                *acted.entry(Instant::now()).or_default() += 1;
                let answer = act(client, &format!("{seat}-{taken}"), body).await;
                assert_eq!(answer["type"], "ack", "{answer}");
                moved = true;
            }
            if moved {
                continue;
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            tokio::time::sleep(WAIT_STEP.min(deadline - now)).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b36_the_ai_team_plays_its_round_a_step_at_a_time_and_hands_it_back_before_the_round_clock() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        h.keep(&a1, "a1-keep").await;
        h.keep(&a2, "a2-keep").await;
        a1.wait_for(0, PROMPTLY, |frame| round_of(frame) >= 1)
            .await
            .expect("round 1 opens once all four mulligans are in");
        let round_clock = ms(DUOS_ROUND_CLOCK_MS);
        let mut acted: BTreeMap<Instant, usize> = BTreeMap::new();

        let (r2_index, r2_at) = round_reached(&a1, &a2, 0, 2, round_clock + PROMPTLY, &mut acted)
            .await
            .expect("round 2, the AI team's, opens");
        let (r3_index, r3_at) = round_reached(&a1, &a2, r2_index, 3, round_clock, &mut acted)
            .await
            .expect("the AI team ends its round before the round clock would");

        assert!(r3_at < r2_at + round_clock);
        // One action per DUOS_AI_STEP_DELAY_MS: never more than one per AI seat at one instant, the
        // views the people's own actions caused aside.
        let mut per_instant: BTreeMap<Instant, usize> = BTreeMap::new();
        for (at, frame) in &a1.timed()[r2_index + 1..r3_index.max(r2_index + 1)] {
            if round_of(frame) == 2 {
                *per_instant.entry(*at).or_default() += 1;
            }
        }
        for (at, views) in &mut per_instant {
            *views = views.saturating_sub(acted.get(at).copied().unwrap_or(0));
        }
        assert!(
            per_instant.values().all(|views| *views <= 2),
            "the AI acted in bursts: {per_instant:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// B37 — a human gone past the grace
// ---------------------------------------------------------------------------------------------

mod b37_the_disconnect_takeover {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b37_a_human_gone_past_the_grace_is_played_by_the_ai_shown_to_the_teammate_and_handed_back_on_reconnect()
     {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        settle().await;
        let from = a2.received();
        let dropped = Instant::now();
        a1.hang_up().await;
        let grace = ms(DISCONNECT_GRACE_MS);

        let (taken_index, taken_at, taken) = a2
            .wait_for(from, grace + PROMPTLY, |frame| {
                frame["type"] == "teammate" && frame["aiPlaying"] == true
            })
            .await
            .unwrap_or_else(|| panic!("A2 was never told the AI took A1 over: {:?}", a2.frames()));

        assert!(taken_at >= dropped + grace, "not before the grace has run");
        assert_eq!(taken["connected"], json!(false), "{taken}");
        for frame in a2.frames()[from..taken_index]
            .iter()
            .filter(|frame| frame["type"] == "teammate")
        {
            assert_eq!(
                frame["aiPlaying"],
                json!(false),
                "the AI took over early: {frame}"
            );
        }
        // The team never loses to a disconnect.
        assert!(last_view(&a2)["view"]["result"].is_null());

        let back = h.served.connect(DUOS_PATH, &h.host.token).await;
        let from = a2.received();
        back.send_json(hello(&h.match_id)).await;
        let (_, _, view) = back
            .wait_for(0, PROMPTLY, |frame| frame["type"] == "view")
            .await
            .expect("A1's view on its return");
        assert_eq!(view["view"]["seat"], "a1");
        a2.wait_for(from, PROMPTLY, |frame| {
            frame["type"] == "teammate" && frame["connected"] == true && frame["aiPlaying"] == false
        })
        .await
        .unwrap_or_else(|| panic!("A2 was never told A1 is back: {:?}", a2.frames()));
        assert!(last_view(&a2)["view"]["result"].is_null());
    }

    #[tokio::test(start_paused = true)]
    async fn b37_the_teammate_is_shown_the_grace_deadline_of_the_seat_that_dropped() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        settle().await;
        let from = a2.received();
        let dropped_ms = now_ms();

        a1.hang_up().await;

        let (_, _, clock) = a2
            .wait_for(from, PROMPTLY, |frame| {
                frame["type"] == "clock" && !frame["graceDeadlines"]["a1"].is_null()
            })
            .await
            .unwrap_or_else(|| panic!("no clock frame named A1's grace: {:?}", a2.frames()));
        assert_eq!(
            clock["graceDeadlines"]["a1"],
            json!(dropped_ms + int(DISCONNECT_GRACE_MS)),
            "{clock}"
        );
        assert!(clock["graceDeadlines"]["a2"].is_null(), "{clock}");
    }
}

// ---------------------------------------------------------------------------------------------
// B38 — the round clock
// ---------------------------------------------------------------------------------------------

mod b38_the_round_clock {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b38_a_round_its_seats_leave_open_is_closed_by_the_round_clock() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        h.keep(&a1, "a1-keep").await;
        h.keep(&a2, "a2-keep").await;
        let (r1_index, r1_at, r1) = a1
            .wait_for(0, PROMPTLY, |frame| round_of(frame) >= 1)
            .await
            .expect("round 1 opens once all four mulligans are in");
        let both_open = r1["view"]["team"]["ended"] == json!([false, false]);
        let clock = ms(DUOS_ROUND_CLOCK_MS);
        let slack = ms(DUOS_AI_STEP_DELAY_MS) * 4;

        // Neither human does anything more.
        let (_, r2_at, _) = a1
            .wait_for(r1_index, clock + slack, |frame| round_of(frame) >= 2)
            .await
            .expect("the round clock closes round 1");

        assert!(r2_at <= r1_at + clock + slack);
        if both_open {
            assert!(
                r2_at >= r1_at + clock,
                "the round closed before its clock ran out"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn b38_the_clock_frame_names_the_round_deadline() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let from = a1.received();
        h.keep(&a1, "a1-keep").await;
        h.keep(&a2, "a2-keep").await;
        let (_, opened_at, _) = a1
            .wait_for(from, PROMPTLY, |frame| round_of(frame) >= 1)
            .await
            .expect("round 1 opens once all four mulligans are in");
        let deadline = epoch_at(opened_at) + int(DUOS_ROUND_CLOCK_MS);

        let clock = a1
            .wait_for(from, PROMPTLY, |frame| {
                frame["type"] == "clock" && frame["roundDeadline"] == json!(deadline)
            })
            .await;

        assert!(
            clock.is_some(),
            "no clock frame named {deadline}: {:?}",
            a1.of_type_since(from, "clock")
        );
    }
}

// ---------------------------------------------------------------------------------------------
// B39 — pings
// ---------------------------------------------------------------------------------------------

mod b39_pings {
    use super::*;

    fn at_hero(duel: i64, side: &str) -> Value {
        json!({ "duel": duel, "at": "hero", "side": side })
    }

    #[tokio::test(start_paused = true)]
    async fn b39_a_ping_is_relayed_to_the_teammate_alone_stamped_with_the_senders_seat() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let (a1_from, a2_from) = (a1.received(), a2.received());
        let target = at_hero(0, "p2");

        a1.send_json(ping("thumbsUp", target.clone())).await;
        settle().await;

        assert_eq!(
            a2.of_type_since(a2_from, "ping"),
            vec![relayed("a1", "thumbsUp", &target)]
        );
        assert!(
            a1.of_type_since(a1_from, "ping").is_empty(),
            "the sender gets no echo"
        );
        assert!(a1.of_type_since(a1_from, "error").is_empty());

        // And the other way, A2's own hand named in its own duel.
        let back = json!({ "duel": 1, "at": "hand", "side": "p1", "index": 0 });
        let a1_from = a1.received();
        a2.send_json(ping("careful", back.clone())).await;
        settle().await;
        assert_eq!(
            a1.of_type_since(a1_from, "ping"),
            vec![relayed("a2", "careful", &back)]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn b39_every_ping_kind_and_target_shape_is_relayed() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let own_hand = hand_len(&last_view(&a1)["view"]["own"]);
        assert!(own_hand > 0, "A1 holds an opening hand");
        let last_lane = int(UNIT_ZONES) - 1;
        let pings = [
            ("thumbsUp", at_hero(0, "p1")),
            (
                "thumbsDown",
                json!({ "duel": 1, "at": "unit", "side": "p2", "lane": 0 }),
            ),
            (
                "portal",
                json!({ "duel": 0, "at": "backrow", "side": "p1", "lane": last_lane }),
            ),
            (
                "careful",
                json!({ "duel": 0, "at": "hand", "side": "p1", "index": own_hand - 1 }),
            ),
            (
                "killThis",
                json!({ "duel": 1, "at": "unit", "side": "p2", "lane": last_lane }),
            ),
        ];
        assert!(pings.len() <= count(PING_RATE_LIMIT));
        let a2_from = a2.received();

        for (kind, target) in &pings {
            a1.send_json(ping(kind, target.clone())).await;
        }
        settle().await;

        let expected: Vec<Value> = pings
            .iter()
            .map(|(kind, target)| relayed("a1", kind, target))
            .collect();
        assert_eq!(a2.of_type_since(a2_from, "ping"), expected);
    }

    #[tokio::test(start_paused = true)]
    async fn b39_a_ping_of_the_wrong_shape_is_dropped() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let (a1_from, a2_from) = (a1.received(), a2.received());
        let zones = int(UNIT_ZONES);

        for bad in [
            ping("wave", at_hero(0, "p1")),
            json!({ "type": "ping", "ping": "thumbsUp" }),
            ping("thumbsUp", at_hero(2, "p1")),
            ping("thumbsUp", at_hero(0, "p3")),
            ping("thumbsUp", json!({ "duel": 0, "at": "graveyard", "side": "p1" })),
            ping("thumbsUp", json!({ "duel": 0, "at": "unit", "side": "p1" })),
            ping(
                "thumbsUp",
                json!({ "duel": 0, "at": "unit", "side": "p1", "lane": zones }),
            ),
            ping(
                "thumbsUp",
                json!({ "duel": 0, "at": "backrow", "side": "p2", "lane": -1 }),
            ),
        ] {
            a1.send_json(bad).await;
        }
        settle().await;

        assert!(
            a2.of_type_since(a2_from, "ping").is_empty(),
            "{:?}",
            a2.of_type_since(a2_from, "ping")
        );
        assert!(a1.of_type_since(a1_from, "ping").is_empty());
        assert!(a2.of_type_since(a2_from, "view").is_empty());

        // The socket still relays a good one, once any window the bad ones filled has passed.
        tokio::time::sleep(ms(PING_RATE_WINDOW_MS) + Duration::from_millis(1)).await;
        a1.send_json(ping("thumbsUp", at_hero(0, "p1"))).await;
        settle().await;
        assert_eq!(a2.of_type_since(a2_from, "ping").len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn b39_pings_past_the_rate_limit_are_dropped_silently_until_the_window_passes() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let (a1_from, a2_from) = (a1.received(), a2.received());
        let limit = count(PING_RATE_LIMIT);

        for _ in 0..limit + 2 {
            a1.send_json(ping("careful", at_hero(0, "p2"))).await;
        }
        settle().await;

        assert_eq!(a2.of_type_since(a2_from, "ping").len(), limit);
        assert!(a1.of_type_since(a1_from, "error").is_empty(), "dropped silently");

        tokio::time::sleep(ms(PING_RATE_WINDOW_MS) + Duration::from_millis(1)).await;
        a1.send_json(ping("careful", at_hero(0, "p2"))).await;
        settle().await;
        assert_eq!(a2.of_type_since(a2_from, "ping").len(), limit + 1);
        assert!(a1.of_type_since(a1_from, "error").is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn b39_a_hand_target_on_the_enemy_side_or_past_the_hand_is_dropped() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let frame = last_view(&a1);
        let own_hand = hand_len(&frame["view"]["own"]);
        let mate_hand = hand_len(&frame["view"]["teammate"]);
        assert!(own_hand > 0 && mate_hand > 0, "both humans hold opening hands");
        let a2_from = a2.received();

        for target in [
            json!({ "duel": 0, "at": "hand", "side": "p2", "index": 0 }),
            json!({ "duel": 1, "at": "hand", "side": "p2", "index": 0 }),
            json!({ "duel": 0, "at": "hand", "side": "p1", "index": own_hand }),
            json!({ "duel": 1, "at": "hand", "side": "p1", "index": mate_hand }),
        ] {
            a1.send_json(ping("killThis", target)).await;
        }
        settle().await;
        assert!(
            a2.of_type_since(a2_from, "ping").is_empty(),
            "{:?}",
            a2.of_type_since(a2_from, "ping")
        );

        tokio::time::sleep(ms(PING_RATE_WINDOW_MS) + Duration::from_millis(1)).await;
        let kept = [
            json!({ "duel": 0, "at": "hand", "side": "p1", "index": own_hand - 1 }),
            json!({ "duel": 1, "at": "hand", "side": "p1", "index": mate_hand - 1 }),
        ];
        for target in &kept {
            a1.send_json(ping("portal", target.clone())).await;
        }
        settle().await;
        let expected: Vec<Value> = kept
            .iter()
            .map(|target| relayed("a1", "portal", target))
            .collect();
        assert_eq!(a2.of_type_since(a2_from, "ping"), expected);
    }

    #[tokio::test(start_paused = true)]
    async fn b39_pings_never_enter_a_view_the_action_log_or_the_replay() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let logged = h.logged().await.len();
        let (a1_from, a2_from) = (a1.received(), a2.received());

        for _ in 0..2 {
            a1.send_json(ping("killThis", at_hero(0, "p2"))).await;
            a2.send_json(ping("killThis", at_hero(1, "p2"))).await;
        }
        settle().await;

        assert_eq!(
            a2.of_type_since(a2_from, "ping").len(),
            2,
            "the pings were relayed"
        );
        assert_eq!(
            a1.of_type_since(a1_from, "ping").len(),
            2,
            "the pings were relayed"
        );
        assert!(
            a1.of_type_since(a1_from, "view").is_empty(),
            "a ping pushes no view"
        );
        assert!(
            a2.of_type_since(a2_from, "view").is_empty(),
            "a ping pushes no view"
        );
        assert!(a1.of_type_since(a1_from, "ack").is_empty(), "a ping is no action");
        let log = h.logged().await;
        assert_eq!(log.len(), logged, "a ping is never logged");
        for row in &log {
            assert!(!row.action.to_string().contains("killThis"), "{:?}", row.action);
        }
        assert!(!last_view(&a1).to_string().contains("killThis"));
        assert!(!last_view(&a2).to_string().contains("killThis"));

        // Nor is one in the finished game's replay: a step per logged action, the two concedes.
        h.concede_both(&a1, &a2).await;
        for who in [&h.host, &h.guest] {
            let steps = h.steps(who).await;
            assert_eq!(steps.len(), logged + 3);
            for step in &steps {
                assert!(!step.to_string().contains("killThis"), "{step}");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B40, B41, B42, B11 — the log and the replay, the end of a game and the live route
// ---------------------------------------------------------------------------------------------

mod b40_b42_the_log_the_replay_and_the_end {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b40_b41_each_applied_action_adds_one_replay_step_seen_from_the_callers_own_seat() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        let before = h.logged().await.len();
        let keep = keep_whole_hand(&last_view(&a1)).expect("A1 owes its mulligan");

        let answer = act(&a1, "a1-keep", keep).await;

        assert_eq!(answer["type"], "ack", "{answer}");
        let at = usize::try_from(answer["seq"].as_u64().expect("a seq")).expect("a step");
        settle().await;
        let pushed = last_view(&a1);
        assert_eq!(h.logged().await.len(), before + 1, "one row for one action");

        // §13.9: the replay is the finished game's.
        h.concede_both(&a1, &a2).await;
        let logged = h.logged().await.len();
        let host = h.replay(&h.host).await;
        let guest = h.replay(&h.guest).await;
        assert_eq!(host["seat"], "a1", "{host}");
        assert_eq!(guest["seat"], "a2", "{guest}");
        let host_steps = host["steps"].as_array().cloned().expect("steps");
        let guest_steps = guest["steps"].as_array().cloned().expect("steps");
        assert_eq!(
            host_steps.len(),
            logged + 1,
            "the game as it began, then a step per action"
        );
        assert_eq!(guest_steps.len(), logged + 1);
        assert!(host_steps.iter().all(|step| step["seat"] == "a1"));
        assert!(guest_steps.iter().all(|step| step["seat"] == "a2"));
        // Step `at` is the state right after A1's keep, as A1's own view showed it then.
        let mine = &host_steps[at];
        assert_eq!(
            mine["own"]["you"]["hand"], pushed["view"]["own"]["you"]["hand"],
            "{mine}"
        );
        assert!(mulligan_answered(&mine["own"]), "{mine}");
        assert_eq!(mine["round"], pushed["view"]["round"]);
        // A2's step shows its own duel, not A1's, and A1's answered mulligan as its teammate's.
        let theirs = &guest_steps[at];
        assert!(mulligan_answered(&theirs["teammate"]), "{theirs}");
    }

    #[tokio::test(start_paused = true)]
    async fn b35_b42_b11_two_concedes_end_the_game_after_which_an_action_is_match_over_and_live_is_null() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        assert_eq!(h.live(&h.host).await, json!(h.match_id));
        assert_eq!(h.live(&h.guest).await, json!(h.match_id));
        let a2_from = a2.received();

        let offer = act(&a1, "a1-concede", concede()).await;

        assert_eq!(offer["type"], "ack", "{offer}");
        let (_, _, offered) = a2
            .wait_for(a2_from, PROMPTLY, |frame| {
                frame["type"] == "view" && frame["view"]["concedeOfferedBy"] == "a1"
            })
            .await
            .unwrap_or_else(|| panic!("A2 never saw A1's offer: {:?}", a2.frames()));
        assert!(
            offered["view"]["result"].is_null(),
            "an offer alone ends nothing: {offered}"
        );

        let a1_from = a1.received();
        let agreed = act(&a2, "a2-concede", concede()).await;
        assert_eq!(agreed["type"], "ack", "{agreed}");
        let (_, _, ended) = a1
            .wait_for(a1_from, PROMPTLY, |frame| {
                frame["type"] == "view" && !frame["view"]["result"].is_null()
            })
            .await
            .unwrap_or_else(|| panic!("the game never ended: {:?}", a1.frames()));
        assert_eq!(
            ended["view"]["result"],
            json!({ "outcome": "win", "winner": "b", "reason": "concede" })
        );

        let late = act(&a1, "a1-late", concede()).await;
        assert_eq!(late["type"], "error", "{late}");
        assert_eq!(late["code"], "match_over", "{late}");
        assert_eq!(late["nonce"], "a1-late", "{late}");

        assert!(h.live(&h.host).await.is_null());
        assert!(h.live(&h.guest).await.is_null());
        let steps = h.steps(&h.host).await;
        let last = steps.last().expect("the replay holds the end");
        assert_eq!(last["result"]["reason"], "concede", "{last}");
        assert_eq!(last["seat"], "a1");
    }

    #[tokio::test(start_paused = true)]
    async fn b41_the_replay_is_404_to_an_account_that_holds_no_seat() {
        let h = duos("easy").await;

        let (status, body) = api(
            &h.app,
            "GET",
            &format!("/api/duos/matches/{}/replay", h.match_id),
            &h.outsider.token,
            Value::Null,
        )
        .await;

        assert_eq!(status, 404, "{body}");
        assert!(body["steps"].is_null(), "{body}");
        assert!(h.live(&h.outsider).await.is_null());
    }
}

// ---------------------------------------------------------------------------------------------
// §13.2, R268 — the mulligan clock keeps a person's hand
// ---------------------------------------------------------------------------------------------

mod r268_the_mulligan_clock {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r268_the_mulligan_clock_keeps_each_owing_persons_whole_hand_never_the_ais_mulligan() {
        // Every card costs 4 or more: the AI's own mulligan, which keeps the cheap ones, would send
        // nearly all of the host's opening hand back.
        let costly = deck_where(|def| def["cost"].as_i64().is_some_and(|cost| cost >= 4));
        let h = duos_with("easy", Some(costly)).await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        let dealt_a1 = hand_ids(&first_view(&a1)["view"]["own"]);
        let dealt_a2 = hand_ids(&first_view(&a2)["view"]["own"]);
        assert!(
            !dealt_a1.is_empty() && !dealt_a2.is_empty(),
            "both were dealt a hand"
        );

        // Neither person answers: the clock runs out, and round 1 opens once it has answered both.
        let within = ms(DUOS_MULLIGAN_CLOCK_MS) + ms(DUOS_AI_STEP_DELAY_MS) * 4 + PROMPTLY;
        let (_, _, opened) = a1
            .wait_for(0, within, |frame| round_of(frame) >= 1)
            .await
            .unwrap_or_else(|| panic!("round 1 never opened: {:?}", a1.frames()));

        let holds_a1 = hand_ids(&opened["view"]["own"]);
        let holds_a2 = hand_ids(&opened["view"]["teammate"]);
        for card in &dealt_a1 {
            assert!(
                holds_a1.contains(card),
                "A1 keeps {card}: dealt {dealt_a1:?}, holds {holds_a1:?}"
            );
        }
        for card in &dealt_a2 {
            assert!(
                holds_a2.contains(card),
                "A2 keeps {card}: dealt {dealt_a2:?}, holds {holds_a2:?}"
            );
        }
        // Each person's answer is the one mulligan that keeps every card.
        let log = h.logged().await;
        for seat in ["a1", "a2"] {
            let answer = log
                .iter()
                .find(|row| row.action["seat"] == seat && row.action["body"]["body"]["type"] == "mulligan")
                .unwrap_or_else(|| panic!("{seat}'s mulligan was answered: {log:?}"));
            let kept = answer.action["body"]["body"]["keep"]
                .as_array()
                .map_or(0, Vec::len);
            let dealt = if seat == "a1" { &dealt_a1 } else { &dealt_a2 };
            assert_eq!(kept, dealt.len(), "{seat}: {:?}", answer.action);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B37, B38 — a move the AI decided under one hold is not played under another
// ---------------------------------------------------------------------------------------------

mod b37_b38_the_ais_hold {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b37_b38_a_move_the_ai_decided_for_a_seat_past_its_grace_is_dropped_once_its_person_is_back() {
        // Units of cost 1 or less: playing A1's round 1 outright, the AI always has a card to play.
        let cheap =
            deck_where(|def| def["type"] == "Unit" && def["cost"].as_i64().is_some_and(|cost| cost <= 1));
        let h = duos_with("easy", Some(cheap)).await;
        let a1 = h.seat(&h.host).await;
        let a2 = h.seat(&h.guest).await;
        h.ai_mulligans_answered(&a1).await;
        h.keep(&a1, "a1-keep").await;
        h.keep(&a2, "a2-keep").await;
        let (_, opened_at, _) = a1
            .wait_for(0, PROMPTLY, |frame| round_of(frame) >= 1)
            .await
            .expect("round 1 opens once all four mulligans are in");
        let (grace, round_clock, step) = (
            ms(DISCONNECT_GRACE_MS),
            ms(DUOS_ROUND_CLOCK_MS),
            ms(DUOS_AI_STEP_DELAY_MS),
        );

        // A1 drops so that its grace runs out half a step before round 1's clock: the AI takes A1
        // over outright, the clock then hands it A1's turn's end, and the AI's first move for A1,
        // a step after the grace, is decided while it still plays A1 outright.
        let dropped = opened_at + round_clock - grace - step / 2;
        tokio::time::sleep_until(dropped).await;
        a1.hang_up().await;
        let decided_at = dropped + grace + step;
        tokio::time::sleep_until(decided_at - Duration::from_millis(1)).await;
        let actor = registry(&h.app).get(&h.match_id).expect("the live actor");
        assert!(actor.ai_playing(DuosSeat::A1), "the AI plays A1 outright");
        let logged = actor.next_seq();
        // The decision starts (off the runtime), and before it lands A1's person is back.
        tokio::time::advance(Duration::from_millis(1)).await;
        let (back, _frames) = Socket::channel();
        actor.attach(DuosSeat::A1, back);
        assert_eq!(
            actor.next_seq(),
            logged,
            "the AI's move for A1 is still being decided"
        );

        // The round clock ran out for A1 meanwhile: the AI only ends A1's turn now.
        a2.wait_for(0, step * 4 + PROMPTLY, |frame| round_of(frame) >= 2)
            .await
            .unwrap_or_else(|| panic!("round 1 never closed: {:?}", a2.frames()));
        let moves: Vec<Value> = h
            .logged()
            .await
            .into_iter()
            .filter(|row| row.seq >= logged && row.action["seat"] == "a1")
            .map(|row| row.action["body"].clone())
            .collect();
        assert!(!moves.is_empty(), "A1's turn was ended for it");
        for body in &moves {
            assert_eq!(
                *body,
                json!({ "kind": "duel", "body": { "type": "endTurn" } }),
                "nothing but the end of A1's turn is played for its person: {moves:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// §9.1 — repeated hellos coalesce (R835001's rule)
// ---------------------------------------------------------------------------------------------

mod hello_coalescing {
    use super::*;

    const HELLO: &str = r#"{"type":"hello"}"#;

    #[tokio::test(start_paused = true)]
    async fn hellos_repeated_while_one_is_queued_coalesce_and_a_newer_socket_always_gets_its_own() {
        let h = duos("easy").await;
        let actor = registry(&h.app).get(&h.match_id).expect("the live actor");
        let (socket, mut frames) = Socket::channel();
        actor.attach(DuosSeat::A1, socket.clone());
        actor.idle().await;
        drain(&mut frames);

        // A burst with nothing awaited between the frames: every hello but the first is absorbed.
        for _ in 0..10 {
            socket.receive(HELLO.to_string());
        }
        actor.idle().await;
        assert_eq!(views_in(&drain(&mut frames)), 1);

        // A flood stays far below one view push per frame.
        for _ in 0..200 {
            socket.receive(HELLO.to_string());
        }
        actor.idle().await;
        assert!(views_in(&drain(&mut frames)) <= 10);

        // A hello on a newer socket is never absorbed by the older one's queued Hello.
        let (older, _older_frames) = Socket::channel();
        actor.attach(DuosSeat::A1, older.clone());
        actor.idle().await;
        older.receive(HELLO.to_string());
        let (revived, mut revived_frames) = Socket::channel();
        actor.attach(DuosSeat::A1, revived.clone());
        revived.receive(HELLO.to_string());
        actor.idle().await;
        // The attach's own view, the older socket's queued Hello, and the newer socket's.
        assert_eq!(views_in(&drain(&mut revived_frames)), 3);
    }
}

// ---------------------------------------------------------------------------------------------
// B40, §9.1 — a rebuild, and none for a socket that holds no seat
// ---------------------------------------------------------------------------------------------

mod b40_the_rebuild {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn b40_a_hello_from_an_account_with_no_seat_folds_nothing_and_a_seats_hello_rebuilds_the_match() {
        let h = duos("easy").await;
        let a1 = h.seat(&h.host).await;
        h.ai_mulligans_answered(&a1).await;
        h.keep(&a1, "a1-keep").await;
        let held = registry(&h.app);
        let live = held.get(&h.match_id).expect("the live actor");
        let (hash, next_seq) = (live.hash(), live.next_seq());
        held.stop(&h.match_id).await;
        assert!(!held.has(&h.match_id));

        let outsider = h.served.connect(DUOS_PATH, &h.outsider.token).await;
        outsider.send_json(hello(&h.match_id)).await;
        assert_eq!(outsider.wait_closed(PROMPTLY).await, Some(CLOSE_NOT_FOUND));
        assert!(outsider.of_type("view").is_empty(), "{:?}", outsider.frames());
        assert!(
            !held.has(&h.match_id),
            "nothing was folded for a socket that holds no seat"
        );

        // The host's own hello folds the match back, to the state it was stopped in.
        let back = h.seat(&h.host).await;
        let rebuilt = held.get(&h.match_id).expect("the rebuilt actor");
        assert_eq!(rebuilt.hash(), hash);
        assert_eq!(rebuilt.next_seq(), next_seq);
        assert_eq!(first_view(&back)["view"]["seat"], "a1");
    }
}
