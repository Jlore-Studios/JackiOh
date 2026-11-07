//! The social socket (SPEC §9.4, §9.5, R3214000): `/ws/social`, one per signed-in tab, over which the
//! server says what changed instead of the tab asking every `SOCIAL_POLL_MS` and an open chat every
//! `CHAT_POLL_MS`.
//!
//! It signs in as `/ws/match` does (`authenticate_upgrade`: the offered `jackioh.bearer.` subprotocol,
//! never the address (R1379000), one request against the account's R109 budget (R2478000), §9.4's
//! gate), and an account holds at most `SOCIAL_SOCKETS_PER_ACCOUNT` of them. Every refusal is one
//! `forbidden` frame and a close with R148's codes, as `/ws/watch`'s is. While it stays open its
//! session is checked again every `SOCIAL_SESSION_CHECK_SECONDS` (the token, the profile, the gate),
//! so a session the provider ended (R194) or an account banned meanwhile is cut off as a polling tab
//! would be refused.
//!
//! The server sends three frames, each only what its account may read:
//! - `{type: "social", incoming, outgoing, unread}`: `GET /api/social`'s own body for the socket's
//!   account (`social_snapshot`), on connect, after every change to its challenges or unread counts,
//!   when one of its challenges lapses, and in answer to a hello;
//! - `{type: "chat", friendId}`: a message between it and that friend was sent; no body rides it,
//!   the tab reads the conversation itself;
//! - `{type: "friends"}`: its friends list changed (a request, an accept, a removal, a block); the tab
//!   reads the list itself. A friend's status is not pushed: the list keeps its own read for that.
//!
//! The socket takes one frame, `{type: "hello"}`, answered with a `social` frame at most once a
//! `WATCH_HELLO_INTERVAL_MS` (R2609000's interval), so a woken page's probe (R1441) hears a live
//! socket. Anything else is answered `malformed` and counted against R835002's budget, past which
//! the socket is closed with 1008.
//!
//! A push is made after its handler's commit, on the success path only, and a failed read is logged
//! and dropped: the tab's next hello, or its polls once the socket is gone, read it again.

use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::actor::contracts::SocketHandlers;
use crate::actor::guard::{CLOSE_POLICY_VIOLATED, SeatGuard};
use crate::actor::protocol::{SocketErrorCode, encode, error_message};
use crate::actor::ws_server::{SOCIAL_PATH, Socket, UpgradeFailure, WS_CLOSE, authenticate_upgrade};
use crate::api::http::{ApiError, assert_active, lock};
use crate::app::{App, now_ms};
use crate::config::{SOCIAL_SESSION_CHECK_SECONDS, SOCIAL_SOCKETS_PER_ACCOUNT};
use crate::social::reads::social_snapshot;

/// What the socket past an account's `SOCIAL_SOCKETS_PER_ACCOUNT` is refused with.
pub const TOO_MANY_TABS: &str = "too many open tabs";

/// What a frame other than a hello is answered with.
const ONLY_HELLO: &str = r#"a social socket takes only "hello""#;

/// The one generation a social socket's hellos are answered under (`SeatGuard::hello_answer_allowed`):
/// a social socket is never reattached.
const HELLO_GENERATION: u64 = 0;

/// The open social sockets of each account, one per `App` (`Social::live`). Never logged.
#[derive(Default)]
pub struct LiveSockets {
    by_account: Mutex<IndexMap<String, Vec<Socket>>>,
}

impl LiveSockets {
    /// Counts `socket` for `id`, or `false` when `id` already holds `SOCIAL_SOCKETS_PER_ACCOUNT` open
    /// ones. A socket already closed no longer counts, whether or not its connection has ended.
    fn try_add(&self, id: &str, socket: &Socket) -> bool {
        let mut by_account = lock(&self.by_account);
        let sockets = by_account.entry(id.to_string()).or_default();
        sockets.retain(Socket::is_open);
        if sockets.len() >= SOCIAL_SOCKETS_PER_ACCOUNT {
            return false;
        }
        sockets.push(socket.clone());
        true
    }

    fn remove(&self, id: &str, socket: &Socket) {
        let mut by_account = lock(&self.by_account);
        if let Some(sockets) = by_account.get_mut(id) {
            sockets.retain(|held| held != socket);
            if sockets.is_empty() {
                by_account.shift_remove(id);
            }
        }
    }

    /// `id`'s open sockets.
    fn of(&self, id: &str) -> Vec<Socket> {
        lock(&self.by_account)
            .get(id)
            .map(|sockets| {
                sockets
                    .iter()
                    .filter(|socket| socket.is_open())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// One admitted social socket. `/ws/social`'s upgrade holds it until the connection ends, and then
/// it leaves its account's sockets; its handlers and its session check hold it weakly.
pub struct LiveSocket {
    app: Weak<App>,
    socket: Socket,
    id: String,
    /// R835002's malformed budget and R2609000's hello interval, this socket's own.
    guard: Mutex<SeatGuard>,
}

impl Drop for LiveSocket {
    fn drop(&mut self) {
        if let Some(app) = self.app.upgrade() {
            app.social.live.remove(&self.id, &self.socket);
        }
    }
}

impl LiveSocket {
    fn receive(&self, text: &str) {
        let now = now_ms();
        if is_hello(text) {
            if !lock(&self.guard).hello_answer_allowed(HELLO_GENERATION, now) {
                return;
            }
            let Some(app) = self.app.upgrade() else { return };
            let socket = self.socket.clone();
            let id = self.id.clone();
            tokio::spawn(async move { send_social(&app, &id, &[socket]).await });
            return;
        }
        // The close calls no handler of this file back, but the lock is let go first all the same.
        let spent = lock(&self.guard).note_malformed(now);
        self.socket.send(encode(&error_message(
            SocketErrorCode::Malformed,
            ONLY_HELLO,
            None,
        )));
        if spent {
            self.socket
                .close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
        }
    }
}

/// A JSON object whose `type` is `"hello"`; any other field is ignored.
fn is_hello(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .is_some_and(|frame| frame.get("type").and_then(Value::as_str) == Some("hello"))
}

/// One `forbidden` frame and a close with `code`, as `/ws/watch` refuses.
fn refuse(socket: &Socket, code: u16, message: &str) {
    socket.send(encode(&error_message(SocketErrorCode::Forbidden, message, None)));
    socket.close(Some(code), Some(message));
}

fn refuse_for(socket: &Socket, failure: UpgradeFailure) {
    match failure {
        UpgradeFailure::Refused { code, message } => refuse(socket, code, &message),
        UpgradeFailure::Threw(message) => {
            tracing::error!(event = "ws.social.threw", message = %message);
            refuse(socket, WS_CLOSE.internal, "something went wrong");
        }
    }
}

/// The `social` frame for `id`: its own `GET /api/social` body, typed.
async fn social_frame(app: &App, id: &str) -> Result<String, ApiError> {
    let mut frame = social_snapshot(app, id).await?;
    if let Value::Object(fields) = &mut frame {
        fields.insert("type".to_string(), json!("social"));
    }
    Ok(frame.to_string())
}

/// Reads `id`'s `social` frame and sends it to `sockets`, or logs why it could not.
async fn send_social(app: &App, id: &str, sockets: &[Socket]) {
    match social_frame(app, id).await {
        Ok(frame) => {
            for socket in sockets {
                socket.send(frame.as_str());
            }
        }
        Err(error) => {
            tracing::warn!(event = "social.live.read_failed", profileId = %id, message = %error.message);
        }
    }
}

/// R3214000: each of `ids` that holds a social socket is sent its own `social` frame, read now; an
/// account with none costs no read. Called after the change's commit.
pub async fn push_social(app: &App, ids: &[&str]) {
    for id in ids {
        let sockets = app.social.live.of(id);
        if !sockets.is_empty() {
            send_social(app, id, &sockets).await;
        }
    }
}

/// R3214000: `push_social` to `ids` once `after_ms` has passed (a challenge lapsing), unless the app
/// has stopped by then.
pub fn push_social_after(app: &Arc<App>, ids: Vec<String>, after_ms: i64) {
    let app = Arc::downgrade(app);
    let wait = Duration::from_millis(u64::try_from(after_ms).unwrap_or(0));
    tokio::spawn(async move {
        tokio::time::sleep(wait).await;
        let Some(app) = app.upgrade() else { return };
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        push_social(&app, &ids).await;
    });
}

fn send_to(app: &App, id: &str, frame: &str) {
    for socket in app.social.live.of(id) {
        socket.send(frame);
    }
}

/// R3214000: a message from `sender` to `recipient` was sent: each one's sockets are told whom the
/// conversation is with, and nothing of the message.
pub fn nudge_chat(app: &App, sender: &str, recipient: &str) {
    send_to(
        app,
        recipient,
        &json!({ "type": "chat", "friendId": sender }).to_string(),
    );
    send_to(
        app,
        sender,
        &json!({ "type": "chat", "friendId": recipient }).to_string(),
    );
}

/// R3214000: the friends list of each of `ids` changed.
pub fn nudge_friends(app: &App, ids: &[&str]) {
    let frame = json!({ "type": "friends" }).to_string();
    for id in ids {
        send_to(app, id, &frame);
    }
}

/// A social socket's life after its upgrade: who it is, its room under the account's cap, its first
/// `social` frame and its session check; or the one refusal. `None` when it was not let in.
pub async fn serve(app: &Arc<App>, socket: &Socket, token: Option<String>) -> Option<Arc<LiveSocket>> {
    // A socket that went before it was read has no one to answer.
    if !socket.is_open() {
        return None;
    }
    let profile = match authenticate_upgrade(app, token.clone(), SOCIAL_PATH).await {
        Ok(profile) => profile,
        Err(failure) => {
            refuse_for(socket, failure);
            return None;
        }
    };
    if !app.social.live.try_add(&profile.id, socket) {
        tracing::info!(event = "ws.social.too_many", profileId = %profile.id, limit = SOCIAL_SOCKETS_PER_ACCOUNT);
        refuse(socket, WS_CLOSE.forbidden, TOO_MANY_TABS);
        return None;
    }
    let live = Arc::new(LiveSocket {
        app: Arc::downgrade(app),
        socket: socket.clone(),
        id: profile.id,
        guard: Mutex::new(SeatGuard::default()),
    });
    let heard = Arc::downgrade(&live);
    socket.attach(SocketHandlers {
        message: Box::new(move |text: String| {
            if let Some(live) = heard.upgrade() {
                live.receive(&text);
            }
        }),
        // Dropping the `LiveSocket` once the connection has ended is what lets go of it.
        close: Box::new(|| {}),
    });
    if !socket.is_open() {
        return None;
    }
    match social_frame(app, &live.id).await {
        Ok(frame) => socket.send(frame),
        Err(error) => {
            refuse_for(socket, UpgradeFailure::Threw(error.message));
            return None;
        }
    }
    watch_session(&live, token.unwrap_or_default());
    Some(live)
}

/// R3214000: every `SOCIAL_SESSION_CHECK_SECONDS` while `live` is open, its session is checked again
/// (`session_holds`); a refusal closes it with its code and words. A check that could not be made is
/// logged and the next one tries again.
fn watch_session(live: &Arc<LiveSocket>, token: String) {
    let live = Arc::downgrade(live);
    let every = Duration::from_secs(u64::try_from(SOCIAL_SESSION_CHECK_SECONDS).unwrap_or(1));
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(every).await;
            let Some(held) = live.upgrade() else { return };
            if !held.socket.is_open() {
                return;
            }
            let Some(app) = held.app.upgrade() else { return };
            match session_holds(&app, &token, &held.id).await {
                Ok(()) => {}
                Err(UpgradeFailure::Refused { code, message }) => {
                    tracing::info!(event = "ws.social.session_ended", profileId = %held.id, code);
                    refuse(&held.socket, code, &message);
                    return;
                }
                Err(UpgradeFailure::Threw(message)) => {
                    tracing::warn!(event = "ws.social.session_check_failed", profileId = %held.id, message = %message);
                }
            }
        }
    });
}

/// R3214000: what the upgrade checked still holds for `id`: `token` verifies (a session the provider
/// ended does not, R194), as the user whose profile is `id`, and that profile passes §9.4's gate. It
/// is no request, so it is not counted against R109.
async fn session_holds(app: &App, token: &str, id: &str) -> Result<(), UpgradeFailure> {
    let signed_out = || UpgradeFailure::refused(WS_CLOSE.unauthorized, "sign in first");
    let Ok(user) = app.auth.verify(token).await else {
        return Err(signed_out());
    };
    let mut tx = app.db.begin(Some(&user.user_id)).await?;
    let profile = tx.profiles_get_by_user_id(&user.user_id).await?;
    tx.commit().await?;
    let Some(profile) = profile.filter(|profile| profile.id == id) else {
        return Err(signed_out());
    };
    assert_active(&profile).map_err(|error| UpgradeFailure::refused(WS_CLOSE.forbidden, &error.message))
}
