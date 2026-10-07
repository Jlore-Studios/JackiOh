//! The WebSocket adapter: the only file in `src/actor` that knows a WebSocket library exists.
//!
//! Everything the protocol guarantees is proven against the in-memory socket
//! (`tests/support/socket.rs`), because that is the same `Socket` the actor talks to. This file
//! adds two things and no rules:
//!
//!  - `socket_from_ws`: an axum WebSocket connection as a `Socket`. Text frames only — every
//!    protocol message is JSON (`actor/contracts.rs`), so a binary frame is answered with an
//!    `error` and dropped.
//!  - `handle_match_socket`: the upgrade. It authenticates the access token through `app.auth`,
//!    resolves the profile, applies §9.4's gate (`assert_active`) and refuses unless that profile
//!    is in the match it asked for (§9.1: the client may only read its own view of a match it is
//!    playing). Only then does the registry get the socket.
//!
//! `handle` also bounds what one client can cost before it has authenticated: a frame over
//! `MAX_FRAME_BYTES` closes the socket with 1009 before it is buffered whole, and one client address
//! holds at most `WS_MAX_CONNECTIONS_PER_ADDRESS` sockets, handshakes included.
//!
//! Port of `apps/server/src/match/wsServer.ts` (SURFACE §4.1), with SURFACE §11.3's deltas: the
//! token comes from `?token=` only (what the browser and the e2e Node player send; the unused
//! `Authorization` header and `Sec-WebSocket-Protocol` token paths are dropped, and with them
//! `subprotocolToken`), `jackioh.v1` is still echoed when offered, and there are no heartbeats (TS
//! has none). `ws`'s `noServer` upgrade hook is axum's: `app.rs`'s router sends `GET /ws/match` here
//! (SURFACE §11.2), so the path is ours by construction and TS's `path` option goes. `Socket` lives
//! here because SURFACE §11.2 names it `actor::ws_server::Socket` (`Registry::attach` takes it).

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, FromRequestParts, Query, Request};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use indexmap::IndexMap;
use tokio::sync::mpsc;

use crate::actor::contracts::SocketHandlers;
use crate::actor::protocol::{encode, error_message, SocketErrorCode, MAX_FRAME_BYTES};
use crate::actor::registry::AttachError;
use crate::api::http::{assert_active, client_address, rate_limit_address, ApiError, ApiErrorCode};
use crate::app::{browser_origins, App};
use crate::config::{DEFAULT_TRUSTED_PROXY_HOPS, WS_MAX_CONNECTIONS_PER_ADDRESS};

/// SPEC §9.2: one WebSocket per player, upgraded on the same listener the API serves.
pub const WS_PATH: &str = "/ws/match";

/// The subprotocol a client may name on the handshake. The server echoes only this name back, never
/// anything offered beside it (SURFACE §11.3: no token travels this way any more, but a client that
/// offers the name still gets it).
pub const WS_SUBPROTOCOL: &str = "jackioh.v1";

/// TS `WS_CLOSE`'s shape (SURFACE §4.2: a constant object is a struct named after it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WsClose {
    pub unauthorized: u16,
    pub forbidden: u16,
    pub not_found: u16,
    pub internal: u16,
}

/// SPEC §11 R148: "4401, 4403 and 4404 are private-use mirrors of the HTTP statuses the REST side
/// returns for the same three refusals, with 1011 for an internal fault, so a client reuses one
/// table." R148 also fixes what the socket may learn: every refusal answers with the same error
/// code and only the close code varies, so a socket learns that it may not have this match and
/// never which check said so (§9.1).
pub const WS_CLOSE: WsClose = WsClose { unauthorized: 4401, forbidden: 4403, not_found: 4404, internal: 1011 };

/// RFC 6455's "message too big": what `ws` closes with when a frame passes `maxPayload`.
const CLOSE_MESSAGE_TOO_BIG: u16 = 1009;

/// RFC 6455's "going away": every live socket gets it when the server closes (`close_all`).
const CLOSE_GOING_AWAY: u16 = 1001;

/// RFC 6455's normal closure: `ws.close(code ?? 1000, …)`.
const CLOSE_NORMAL: u16 = 1000;

/// How long a socket the server closed waits for the peer's half of the closing handshake before
/// the connection is dropped: the `ws` library's own `closeTimeout` (30 s), which TS inherited
/// without stating it.
const CLOSE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------------------------

/// What a `Socket` hands its transport: a text frame, or the closing handshake.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocketFrame {
    Text(String),
    Close { code: u16, reason: String },
}

struct SocketInner {
    out: mpsc::UnboundedSender<SocketFrame>,
    open: AtomicBool,
    /// The close handler has run; it runs once, whichever side closed.
    gone: AtomicBool,
    handlers: Mutex<Option<Arc<SocketHandlers>>>,
}

/// One player's connection (TS `Socket`, `actor/contracts.rs`). Text frames only: every protocol
/// message is JSON. The axum adapter (`socket_from_ws`) and the in-memory test socket
/// (`tests/support/socket.rs`) both drive this one type through channels, so the actor is
/// transport-agnostic.
///
/// Clones are the same connection: equality is identity, as TS compared sockets with `===`.
#[derive(Clone)]
pub struct Socket {
    inner: Arc<SocketInner>,
}

impl Socket {
    /// A socket over two channels: the frames it sends go to `out`, and the client's text frames
    /// arrive on `incoming`. When `incoming`'s sender is dropped the transport is gone (a dropped
    /// connection), and the close handler runs. Spawns the reader, so it is called inside a tokio
    /// runtime.
    pub fn new(out: mpsc::UnboundedSender<SocketFrame>, incoming: mpsc::UnboundedReceiver<String>) -> Socket {
        let socket = Socket::detached(out);
        let reader = socket.clone();
        tokio::spawn(async move {
            let mut incoming = incoming;
            while let Some(text) = incoming.recv().await {
                reader.receive(text);
            }
            reader.transport_closed();
        });
        socket
    }

    /// A socket whose frames go to `out` and whose transport calls `receive` and
    /// `transport_closed` itself (the axum adapter does).
    pub fn detached(out: mpsc::UnboundedSender<SocketFrame>) -> Socket {
        Socket {
            inner: Arc::new(SocketInner {
                out,
                open: AtomicBool::new(true),
                gone: AtomicBool::new(false),
                handlers: Mutex::new(None),
            }),
        }
    }

    /// A detached socket and the receiving end of its frames, for a transport (or a test) to drain.
    pub fn channel() -> (Socket, mpsc::UnboundedReceiver<SocketFrame>) {
        let (out, frames) = mpsc::unbounded_channel();
        (Socket::detached(out), frames)
    }

    pub fn is_open(&self) -> bool {
        self.inner.open.load(Ordering::SeqCst)
    }

    /// Sends one text frame. A frame sent after the socket closed goes nowhere, as `ws`'s `send` on
    /// a closing socket did.
    pub fn send(&self, text: impl Into<String>) {
        if !self.is_open() {
            return;
        }
        // The transport is gone if its end of the channel is; the close below is what reports it.
        let _ = self.inner.out.send(SocketFrame::Text(text.into()));
    }

    /// Starts the closing handshake: `ws.close(code ?? 1000, reason ?? "")`. The close handler runs
    /// once, now, as the socket closes (the test socket's and `ws`'s `close` event alike).
    pub fn close(&self, code: Option<u16>, reason: Option<&str>) {
        if !self.inner.open.swap(false, Ordering::SeqCst) {
            return;
        }
        let _ = self.inner.out.send(SocketFrame::Close {
            code: code.unwrap_or(CLOSE_NORMAL),
            reason: reason.unwrap_or("").to_string(),
        });
        self.notify_gone();
    }

    /// Installed once, by the actor, when the socket is attached to a seat.
    pub fn attach(&self, handlers: SocketHandlers) {
        *lock(&self.inner.handlers) = Some(Arc::new(handlers));
    }

    /// The transport's side: the client sent a text frame. Dropped before `attach`, as TS's
    /// `handlers?.message(…)` dropped it.
    pub fn receive(&self, text: String) {
        if !self.is_open() {
            return;
        }
        let handlers = lock(&self.inner.handlers).clone();
        if let Some(handlers) = handlers {
            (handlers.message)(text);
        }
    }

    /// The transport's side: the connection ended (closed by the peer, a transport error, or the
    /// end of our own closing handshake). §9.5's grace is what handles a disconnect.
    pub fn transport_closed(&self) {
        self.inner.open.store(false, Ordering::SeqCst);
        self.notify_gone();
    }

    fn notify_gone(&self) {
        if self.inner.gone.swap(true, Ordering::SeqCst) {
            return;
        }
        let handlers = lock(&self.inner.handlers).clone();
        if let Some(handlers) = handlers {
            (handlers.close)();
        }
    }
}

impl PartialEq for Socket {
    fn eq(&self, other: &Socket) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Socket {}

impl std::fmt::Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket").field("open", &self.is_open()).finish_non_exhaustive()
    }
}

/// A std mutex's guard, recovered when a panic elsewhere poisoned it (nothing here can be left
/// half-written by one).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The error frame every refusal and the binary-frame answer send (`encode(errorMessage(…))`).
fn error_frame(code: SocketErrorCode, message: &str) -> String {
    encode(&error_message(code, message, None).into())
}

/// Whether a read failed because a frame or a message passed the size cap (tungstenite's capacity
/// error: "Space limit exceeded: Message too long: …"). Read off its text: the error type belongs
/// to a crate the server does not name.
fn too_large(error: &axum::Error) -> bool {
    let text = error.to_string();
    text.contains("Message too long") || text.contains("Space limit exceeded")
}

/// An axum WebSocket as a `Socket`, and the task that pumps it. Text frames only: a binary frame is
/// answered with `error` `malformed` and dropped; a transport error is a disconnect. The task ends
/// when the connection does, after telling the socket.
pub fn socket_from_ws(ws: WebSocket) -> (Socket, tokio::task::JoinHandle<()>) {
    let (socket, frames) = Socket::channel();
    let pump = tokio::spawn(pump(ws, socket.clone(), frames));
    (socket, pump)
}

async fn pump(mut ws: WebSocket, socket: Socket, mut frames: mpsc::UnboundedReceiver<SocketFrame>) {
    let mut closing: Option<tokio::time::Instant> = None;
    loop {
        let deadline = closing.map(|since| since + CLOSE_HANDSHAKE_TIMEOUT);
        tokio::select! {
            incoming = ws.recv() => match incoming {
                Some(Ok(Message::Text(text))) => socket.receive(text.as_str().to_owned()),
                Some(Ok(Message::Binary(_))) => {
                    let frame = error_frame(SocketErrorCode::Malformed, "text frames only: every message is JSON");
                    if ws.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                // tungstenite answers a ping itself; nothing else listens for either.
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                // The peer's half of the closing handshake; the stream ends next.
                Some(Ok(Message::Close(_))) => {}
                Some(Err(error)) => {
                    // §9.8: a frame over `MAX_FRAME_BYTES` is refused with 1009 as its length
                    // arrives, never buffered whole.
                    if too_large(&error) {
                        let _ = ws
                            .send(Message::Close(Some(CloseFrame { code: CLOSE_MESSAGE_TOO_BIG, reason: "".into() })))
                            .await;
                    }
                    break;
                }
                None => break,
            },
            outgoing = frames.recv() => match outgoing {
                Some(SocketFrame::Text(text)) => {
                    if ws.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Some(SocketFrame::Close { code, reason }) => {
                    let _ = ws.send(Message::Close(Some(CloseFrame { code, reason: reason.into() }))).await;
                    closing.get_or_insert_with(tokio::time::Instant::now);
                }
                None => break,
            },
            () = async {
                match deadline {
                    Some(at) => tokio::time::sleep_until(at).await,
                    None => std::future::pending::<()>().await,
                }
            } => break,
        }
    }
    socket.transport_closed();
}

// ---------------------------------------------------------------------------------------------
// The upgrade
// ---------------------------------------------------------------------------------------------

/// What refused a socket after its upgrade: a refusal with its close code and message, or a fault
/// (TS's `catch` of anything that is not an `ApiError`).
enum UpgradeFailure {
    Refused { code: u16, message: String },
    Threw(String),
}

impl UpgradeFailure {
    fn refused(code: u16, message: &str) -> UpgradeFailure {
        UpgradeFailure::Refused { code, message: message.to_string() }
    }

    /// An `ApiError` refuses with 4404 when it is a 404 and 4403 otherwise.
    fn api(error: &ApiError) -> UpgradeFailure {
        let code = if error.code == ApiErrorCode::NotFound { WS_CLOSE.not_found } else { WS_CLOSE.forbidden };
        UpgradeFailure::Refused { code, message: error.message.clone() }
    }
}

impl From<crate::db::store::StoreError> for UpgradeFailure {
    fn from(error: crate::db::store::StoreError) -> UpgradeFailure {
        UpgradeFailure::Threw(error.to_string())
    }
}

/// The first value of a query parameter, URL-decoded (`url.searchParams.get(name)`).
fn query_param(query: &[(String, String)], name: &str) -> Option<String> {
    query.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone())
}

/// The query string's pairs, in order. A query that does not parse reads as empty, as a URL with
/// no parameters would.
fn query_pairs(uri: &Uri) -> Vec<(String, String)> {
    match Query::<Vec<(String, String)>>::try_from_uri(uri) {
        Ok(Query(pairs)) => pairs,
        Err(_) => Vec::new(),
    }
}

/// SURFACE §11.3: the token comes from `?token=` only. An empty value is no token.
fn token_from(query: &[(String, String)]) -> Option<String> {
    query_param(query, "token").filter(|token| !token.is_empty())
}

/// The authentication and the attach, in TS's order; any failure is reported to `refuse`.
async fn upgrade_socket(app: &Arc<App>, socket: &Socket, query: &[(String, String)]) -> Result<(), UpgradeFailure> {
    let Some(token) = token_from(query) else {
        return Err(UpgradeFailure::refused(WS_CLOSE.unauthorized, "sign in first"));
    };

    let Ok(user) = app.auth.verify(&token).await else {
        return Err(UpgradeFailure::refused(WS_CLOSE.unauthorized, "sign in first"));
    };

    let mut tx = app.db.begin(None).await?;
    let profile = tx.profiles_get_by_user_id(&user.user_id).await?;
    tx.commit().await?;
    let Some(profile) = profile else {
        return Err(UpgradeFailure::refused(WS_CLOSE.unauthorized, "sign in first"));
    };

    // §9.4's gate, from the same function the HTTP router uses.
    assert_active(&profile).map_err(|error| UpgradeFailure::api(&error))?;

    let asked = query_param(query, "matchId");
    let Some(match_id) = asked.or_else(|| profile.in_match_id.clone()) else {
        return Err(UpgradeFailure::refused(WS_CLOSE.not_found, "you are not in a match"));
    };
    // §9.1: a socket is only ever opened onto a match this profile is playing.
    if profile.in_match_id.as_deref() != Some(match_id.as_str()) {
        return Err(UpgradeFailure::refused(WS_CLOSE.forbidden, "you are not in that match"));
    }

    match app.matches.attach(app, &match_id, &profile.id, socket.clone()).await {
        Ok(()) => Ok(()),
        Err(AttachError::Api(error)) => Err(UpgradeFailure::api(&error)),
        Err(other) => Err(UpgradeFailure::Threw(other.to_string())),
    }
}

/// TS `createMatchSocketHandler(deps)(ws, request)`: authenticate an upgraded socket and hand it to
/// the registry, or refuse it with the one error frame and a close code.
pub async fn handle_match_socket(app: &Arc<App>, socket: &Socket, uri: &Uri) {
    let refuse = |code: u16, message: &str| {
        // One protocol code for every refusal: a socket learns that it may not have this match,
        // never which of the checks said so. A peer already gone drops the frame; the close below
        // is what matters.
        socket.send(error_frame(SocketErrorCode::Forbidden, message));
        socket.close(Some(code), Some(message));
    };

    let query = query_pairs(uri);
    match upgrade_socket(app, socket, &query).await {
        Ok(()) => {}
        Err(UpgradeFailure::Refused { code, message }) => refuse(code, &message),
        Err(UpgradeFailure::Threw(message)) => {
            tracing::error!(event = "ws.upgrade.threw", message = %message);
            refuse(WS_CLOSE.internal, "something went wrong");
        }
    }
}

/// TS `AttachOptions`, minus `path` (the router owns the path). Every field defaults as TS's did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachOptions {
    /// §9.8: a browser attaches credentials to a cross-origin WebSocket handshake automatically, so
    /// an unchecked upgrade is a CSRF surface. An empty or absent list allows any origin, which is
    /// what a Node client (the e2e `wsPlayer` task) sends — it has no `Origin` at all.
    pub allowed_origins: Option<Vec<String>>,
    /// R190's hop count, so a socket is counted against the same client address the API's per-IP
    /// limits read (`client_address` in api/http.rs). Defaults to `DEFAULT_TRUSTED_PROXY_HOPS`.
    pub trusted_proxy_hops: Option<usize>,
    /// Defaults to `WS_MAX_CONNECTIONS_PER_ADDRESS`; a test lowers it.
    pub max_connections_per_address: Option<usize>,
}

impl AttachOptions {
    /// What `index.ts` passed: the browser origins and R190's hop count from the environment.
    pub fn for_app(app: &App) -> AttachOptions {
        AttachOptions {
            allowed_origins: Some(browser_origins(&app.env)),
            // R190: sockets per address are counted on the same address the API's per-IP limits read.
            trusted_proxy_hops: Some(app.env.trusted_proxy_hops),
            max_connections_per_address: None,
        }
    }
}

/// The `X-Forwarded-For` of an upgrade, as the headers that `client_address` reads.
fn forwarded_headers(headers: &HeaderMap) -> HeaderMap {
    let joined: Vec<&str> =
        headers.get_all("x-forwarded-for").iter().filter_map(|value| value.to_str().ok()).collect();
    let mut forwarded = HeaderMap::new();
    if !joined.is_empty() {
        if let Ok(value) = HeaderValue::from_str(&joined.join(",")) {
            forwarded.insert("x-forwarded-for", value);
        }
    }
    forwarded
}

/// R162 makes this list the same one the REST layer reads, "so the two doors cannot diverge" — so
/// the comparison has to match too. `api/cors.rs` canonicalises a trailing slash before comparing;
/// a raw `includes` here meant a hand-written `PUBLIC_ORIGINS=https://play.example/` was accepted
/// by CORS and refused at the upgrade.
fn same_origin(a: &str, b: &str) -> bool {
    fn strip(value: &str) -> String {
        value.trim().trim_end_matches('/').to_lowercase()
    }
    strip(a) == strip(b)
}

fn origin_allowed(allowed: &[String], headers: &HeaderMap) -> bool {
    if allowed.is_empty() {
        return true;
    }
    // No Origin header at all is a non-browser client, which the CSRF concern does not reach.
    let Some(origin) = headers.get("origin").map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
    else {
        return true;
    };
    if origin.is_empty() {
        return true;
    }
    allowed.iter().any(|entry| same_origin(entry, &origin))
}

/// Open and in-progress sockets per client address (`rate_limit_address` form), and every live
/// socket, per running `App` (TS kept both per `attachWebSocketServer` call). Never logged.
#[derive(Default)]
struct Upgrades {
    per_address: IndexMap<String, usize>,
    clients: Vec<Socket>,
}

static UPGRADES: LazyLock<Mutex<IndexMap<usize, Upgrades>>> = LazyLock::new(|| Mutex::new(IndexMap::new()));

/// One `App`'s key in `UPGRADES`: its address, which is stable while it runs.
fn app_key(app: &App) -> usize {
    std::ptr::from_ref(app) as usize
}

/// One counted socket of an address. Dropping it releases the count, which covers both a finished
/// socket and a handshake that never completed (axum drops the upgrade callback then).
struct AddressSlot {
    app: usize,
    address: String,
}

impl AddressSlot {
    /// Counts one more socket for `address`, or `None` when it already holds `max`.
    fn claim(app: usize, address: String, max: usize) -> Option<AddressSlot> {
        let mut upgrades = lock(&UPGRADES);
        let entry = upgrades.entry(app).or_default();
        let held = entry.per_address.get(&address).copied().unwrap_or(0);
        if held >= max {
            return None;
        }
        entry.per_address.insert(address.clone(), held + 1);
        Some(AddressSlot { app, address })
    }
}

impl Drop for AddressSlot {
    fn drop(&mut self) {
        let mut upgrades = lock(&UPGRADES);
        if let Some(entry) = upgrades.get_mut(&self.app) {
            let left = entry.per_address.get(&self.address).copied().unwrap_or(1).saturating_sub(1);
            if left == 0 {
                entry.per_address.shift_remove(&self.address);
            } else {
                entry.per_address.insert(self.address.clone(), left);
            }
        }
    }
}

fn register_client(app: usize, socket: &Socket) {
    lock(&UPGRADES).entry(app).or_default().clients.push(socket.clone());
}

fn unregister_client(app: usize, socket: &Socket) {
    if let Some(entry) = lock(&UPGRADES).get_mut(&app) {
        entry.clients.retain(|client| client != socket);
    }
}

/// TS `AttachedSockets.close`: shuts every live socket of this server with 1001, so a server that is
/// closing really does release its connections.
pub fn close_all(app: &App) {
    let clients: Vec<Socket> =
        lock(&UPGRADES).get(&app_key(app)).map(|entry| entry.clients.clone()).unwrap_or_default();
    for client in clients {
        client.close(Some(CLOSE_GOING_AWAY), Some("server closing"));
    }
}

/// The address an upgrade is counted against: the peer, or R190's forwarded entry.
fn upgrade_address(headers: &HeaderMap, peer: Option<&str>, trusted_proxy_hops: usize) -> String {
    rate_limit_address(&client_address(&forwarded_headers(headers), peer, trusted_proxy_hops))
}

/// The `/ws/match` upgrade (SURFACE §11.2), with `index.ts`'s options.
pub async fn handle(app: Arc<App>, req: Request) -> Response {
    let options = AttachOptions::for_app(&app);
    handle_with(app, req, &options).await
}

/// TS `attachWebSocketServer`'s upgrade listener: a disallowed origin is refused before any token is
/// read, an address over its socket count is refused next, and only then is the handshake taken
/// over, with frames capped at `MAX_FRAME_BYTES` and only `WS_SUBPROTOCOL` echoed.
pub async fn handle_with(app: Arc<App>, req: Request, options: &AttachOptions) -> Response {
    let allowed = options.allowed_origins.clone().unwrap_or_default();
    let trusted_proxy_hops = options.trusted_proxy_hops.unwrap_or(DEFAULT_TRUSTED_PROXY_HOPS);
    let max_per_address = options.max_connections_per_address.unwrap_or(WS_MAX_CONNECTIONS_PER_ADDRESS);

    let (mut parts, _body) = req.into_parts();

    if !origin_allowed(&allowed, &parts.headers) {
        let origin = parts.headers.get("origin").map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned());
        tracing::warn!(event = "ws.upgrade.origin_refused", origin = %origin.unwrap_or_else(|| "undefined".to_string()));
        // TS wrote a bare `HTTP/1.1 403 Forbidden` and destroyed the socket: a status, no body.
        return StatusCode::FORBIDDEN.into_response();
    }

    let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|ConnectInfo(addr)| addr.ip().to_string());
    let address = upgrade_address(&parts.headers, peer.as_deref(), trusted_proxy_hops);
    let key = app_key(&app);
    let Some(slot) = AddressSlot::claim(key, address, max_per_address) else {
        // No address in the log line: the count is the signal.
        tracing::warn!(event = "ws.upgrade.too_many", limit = max_per_address);
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };

    let upgrade = match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
        Ok(upgrade) => upgrade,
        Err(rejection) => return rejection.into_response(),
    };
    let uri = parts.uri.clone();

    upgrade
        // §9.8: every client message is a small JSON frame. Without a cap a frame is buffered
        // whole before `parse_client_message` could refuse it; with it the frame is refused with
        // 1009 as its length arrives.
        .max_message_size(MAX_FRAME_BYTES)
        .max_frame_size(MAX_FRAME_BYTES)
        // Echo only the fixed name, never anything that was offered beside it.
        .protocols([WS_SUBPROTOCOL])
        .on_upgrade(move |ws| async move {
            // Held until the connection ends: the address keeps this socket counted.
            let _slot = slot;
            let (socket, pump) = socket_from_ws(ws);
            register_client(key, &socket);
            handle_match_socket(&app, &socket, &uri).await;
            let _ = pump.await;
            unregister_client(key, &socket);
        })
}
