//! Watching a friend's live game (SPEC S7, B50, B51; #939).
//!
//! `/ws/watch` is authenticated as `/ws/match` is (the token from `?token=`, §9.4's gate) and then
//! takes one frame, the hello: `{"type":"hello","matchId":"…","friendId":"…"}`. It is let in when
//! the two accounts are friends with neither blocking the other, the friend holds a seat in the live
//! match it names, a 1v1, ranked or unranked, in which the watcher holds no seat (a player never
//! sees the other seat), and the friend allows spectators (`allowSpectators`, `api/settings.rs`).
//! The actor then judges room (SPECTATORS_PER_MATCH) and from there sends the friend's seat's views,
//! each SPECTATE_DELAY_MS after the state it shows, with `legal: []` (`MatchActor::attach_watcher`).
//!
//! Every refusal is the one `error` frame, `forbidden`, and a close with R148's 4403 (4401 before
//! sign-in, 1011 for a fault), all with the same message, so a socket learns that it may not watch,
//! never which check said so. No frame after the hello reaches the reducer: once the actor has the
//! socket a later hello is answered with the view it was last sent (R1103000, so R1441's wake probe
//! hears a live socket), and every other frame is dropped unread. A watcher leaving changes nothing
//! in the match but the count its players see (B51).
//! Before the hello the socket waits behind the gate `/ws/duos` waits behind (`actor::guard::HelloGate`,
//! R835002): a frame that is not the hello is answered `malformed` and past
//! `MALFORMED_FRAMES_PER_WINDOW` of them the socket is closed, and a hello that does not come within
//! `WS_IDLE_TIMEOUT_SECONDS` is refused like any other.
//!
//! Being let in is no standing leave (§9.8, "the friend can turn it off"; B46): when the friend turns
//! `allowSpectators` off (`revoke_all_of`), or the two unfriend or either blocks the other
//! (`revoke_between`), the watcher's socket gets the same `forbidden` and close at once
//! (`MatchActor::revoke_watchers`). A hello that passed its checks just before such a change is
//! judged again once its socket is in (`admit`), so the change never misses it.
//!
//! S7's `Friend.watchable`, for the friends list, is `social::presence::standings`'s, which judges
//! what this module's checks judge for every friend at once (`Tx::presence_watchable`).

use std::future::Future;
use std::sync::Arc;

use axum::http::Uri;
use serde_json::Value;

use crate::actor::guard::HelloGate;
use crate::actor::match_actor::WATCH_NOT_ALLOWED as NOT_ALLOWED;
use crate::actor::protocol::{SocketErrorCode, encode, error_message};
use crate::actor::registry::AttachError;
use crate::actor::ws_server::{Socket, WS_CLOSE, upgrade_token};
use crate::api::http::assert_active;
use crate::app::App;
use crate::db::store::{MatchPhase, MatchStatus, Profile, StoreError, Tx};
use crate::social::friends::are_friends;

/// The watcher's one frame (S7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchHello {
    pub match_id: String,
    pub friend_id: String,
}

/// Total, never panics: a JSON object whose `type` is `"hello"`, with `matchId` and `friendId`
/// non-empty strings; any other field is ignored. The error is the reason a `malformed` frame gives.
pub fn parse_watch_hello(text: &str) -> Result<WatchHello, String> {
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        return Err("every frame must be JSON".to_string());
    };
    let Some(frame) = parsed.as_object() else {
        return Err("every frame must be a JSON object".to_string());
    };
    if frame.get("type").and_then(Value::as_str) != Some("hello") {
        return Err(r#"a watch socket's first frame is "hello""#.to_string());
    }
    let text_of = |key: &str| {
        frame
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    match (text_of("matchId"), text_of("friendId")) {
        (Some(match_id), Some(friend_id)) => Ok(WatchHello { match_id, friend_id }),
        _ => Err(r#""hello" needs "matchId" and "friendId""#.to_string()),
    }
}

/// The watch socket's handlers until its hello is answered: `actor::guard::HelloGate`, the gate
/// `/ws/duos` waits behind too, with `parse_watch_hello` for the hello.
pub type WatchListener = HelloGate<WatchHello>;

/// Puts the watch's frame handlers on `socket`, before anything can arrive on it (the upgrade calls
/// this before its pump starts): the first hello is kept and every frame after it dropped unread
/// (B50). The handlers hold the gate weakly and the gate belongs to `serve`, so a socket refused or
/// gone frees itself; the actor's handlers replace them once it takes the socket.
pub fn listen(socket: &Socket) -> WatchListener {
    HelloGate::install(socket, parse_watch_hello)
}

/// `listen`, then `serve`. The handlers go on when this is called, before the future it answers
/// first runs, so a hello sent straight after the call is heard.
pub fn handle_watch_socket<'a>(
    app: &'a Arc<App>,
    socket: &'a Socket,
    uri: &'a Uri,
) -> impl Future<Output = ()> + Send + 'a {
    let listener = listen(socket);
    serve(app, socket, uri, listener)
}

/// Why a watch socket was not handed to the actor.
enum WatchFailure {
    Refused {
        code: u16,
        message: String,
    },
    Threw(String),
    /// The socket went first; there is no one to answer.
    Gone,
}

fn refused(code: u16, message: &str) -> WatchFailure {
    WatchFailure::Refused {
        code,
        message: message.to_string(),
    }
}

impl From<StoreError> for WatchFailure {
    fn from(error: StoreError) -> WatchFailure {
        WatchFailure::Threw(error.to_string())
    }
}

/// A watch socket's life after `listen`: who it is, its hello, B50's checks, and the actor; or the
/// one refusal.
pub async fn serve(app: &Arc<App>, socket: &Socket, uri: &Uri, listener: WatchListener) {
    // A socket that went before its handlers were in has no close left to tell them.
    if !socket.is_open() {
        return;
    }
    let refuse = |code: u16, message: &str| {
        socket.send(encode(&error_message(SocketErrorCode::Forbidden, message, None)));
        socket.close(Some(code), Some(message));
    };
    match admit(app, socket, uri, listener).await {
        Ok(()) | Err(WatchFailure::Gone) => {}
        Err(WatchFailure::Refused { code, message }) => refuse(code, &message),
        Err(WatchFailure::Threw(message)) => {
            tracing::error!(event = "ws.watch.threw", message = %message);
            refuse(WS_CLOSE.internal, "something went wrong");
        }
    }
}

async fn admit(
    app: &Arc<App>,
    socket: &Socket,
    uri: &Uri,
    listener: WatchListener,
) -> Result<(), WatchFailure> {
    let watcher = authenticate(app, uri).await?;
    let Some(hello) = listener.wait().await else {
        // R835002: still open means no hello came in `WS_IDLE_TIMEOUT_SECONDS`; gone, or closed for
        // its malformed frames, has no one to answer.
        return Err(if socket.is_open() {
            refused(WS_CLOSE.forbidden, NOT_ALLOWED)
        } else {
            WatchFailure::Gone
        });
    };
    if !may_watch(app, &watcher.id, &hello).await? {
        tracing::info!(event = "watch.refused", matchId = %hello.match_id);
        return Err(refused(WS_CLOSE.forbidden, NOT_ALLOWED));
    }
    match app
        .matches
        .attach_watcher(
            app,
            &hello.match_id,
            &hello.friend_id,
            &watcher.id,
            socket.clone(),
        )
        .await
    {
        Ok(()) => {
            // An unfriend, a block or watching turned off that committed after the checks above, and
            // revoked before this socket was in, is read now: the socket is in, so a change from here
            // on revokes it, and one before is seen by this read.
            if !may_watch(app, &watcher.id, &hello).await? {
                tracing::info!(event = "watch.revoked_at_hello", matchId = %hello.match_id);
                app.matches.revoke_watchers(|watcher_id, friend_id| {
                    watcher_id == watcher.id && friend_id == hello.friend_id
                });
            }
            Ok(())
        }
        Err(AttachError::Api(error)) => {
            tracing::info!(event = "watch.refused", matchId = %hello.match_id, reason = %error.message);
            Err(refused(WS_CLOSE.forbidden, NOT_ALLOWED))
        }
        Err(AttachError::Internal(message)) => Err(WatchFailure::Threw(message)),
    }
}

/// The account behind the upgrade, read as `/ws/match` reads it: the token, its profile, §9.4's
/// gate.
async fn authenticate(app: &Arc<App>, uri: &Uri) -> Result<Profile, WatchFailure> {
    let Some(token) = upgrade_token(uri) else {
        return Err(refused(WS_CLOSE.unauthorized, "sign in first"));
    };
    let Ok(user) = app.auth.verify(&token).await else {
        return Err(refused(WS_CLOSE.unauthorized, "sign in first"));
    };
    let mut tx = app.db.begin(None).await?;
    let profile = tx.profiles_get_by_user_id(&user.user_id).await?;
    tx.commit().await?;
    let Some(profile) = profile else {
        return Err(refused(WS_CLOSE.unauthorized, "sign in first"));
    };
    assert_active(&profile).map_err(|error| refused(WS_CLOSE.forbidden, &error.message))?;
    Ok(profile)
}

/// B50, as the store answers it: `watcher_id` may watch `hello`'s friend in `hello`'s match. Room is
/// the actor's to judge, under its own lock (`MatchActor::attach_watcher`).
pub async fn may_watch(app: &App, watcher_id: &str, hello: &WatchHello) -> Result<bool, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let allowed = watchable_in(&mut tx, watcher_id, &hello.friend_id, &hello.match_id).await?;
    tx.commit().await?;
    Ok(allowed)
}

/// B51: how many sockets watch `match_id` now; none for a match whose actor is not in memory.
pub fn watcher_count(app: &App, match_id: &str) -> usize {
    app.matches.watcher_count(match_id)
}

/// B50: `friend_id` turned watching off: every socket watching their seat is let go. Answers how
/// many went.
pub fn revoke_all_of(app: &App, friend_id: &str) -> usize {
    app.matches
        .revoke_watchers(|_watcher, friend| friend == friend_id)
}

/// B50, B46: `x` and `y` are no longer friends, or one blocked the other: a socket of either
/// watching the other's seat is let go. Answers how many went.
pub fn revoke_between(app: &App, x: &str, y: &str) -> usize {
    app.matches
        .revoke_watchers(|watcher, friend| (watcher == x && friend == y) || (watcher == y && friend == x))
}

/// B50's store checks, in one transaction: `viewer_id` and `friend_id` are two accounts that are
/// friends with neither blocking the other; `match_id` is live, ranked or unranked, seats
/// `friend_id` and not `viewer_id`; and `friend_id` allows spectators. A Duos match is no row of
/// `matches`, so it is never one.
async fn watchable_in(
    tx: &mut Tx<'_>,
    viewer_id: &str,
    friend_id: &str,
    match_id: &str,
) -> Result<bool, StoreError> {
    if !are_friends(tx, viewer_id, friend_id).await? {
        return Ok(false);
    }
    // `matches_seats` first: it reads any id, where `matches_get` is handed match ids only.
    let live = tx
        .matches_seats(match_id)
        .await?
        .is_some_and(|seats| seats.phase == MatchPhase::Live);
    if !live {
        return Ok(false);
    }
    let Some(row) = tx.matches_get(match_id).await? else {
        return Ok(false);
    };
    let seated = |id: &str| row.players.0 == id || row.players.1 == id;
    if row.status != MatchStatus::Live || !seated(friend_id) || seated(viewer_id) {
        return Ok(false);
    }
    tx.profiles_allows_spectators(friend_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hello_names_the_match_and_the_friend() {
        let hello = parse_watch_hello(r#"{"type":"hello","matchId":"m-1","friendId":"p-2","extra":1}"#);
        assert_eq!(
            hello,
            Ok(WatchHello {
                match_id: "m-1".to_string(),
                friend_id: "p-2".to_string(),
            })
        );
    }

    #[test]
    fn anything_else_is_malformed() {
        for text in [
            "not json",
            "[]",
            r#"{"type":"action","matchId":"m-1","friendId":"p-2"}"#,
            r#"{"type":"hello","matchId":"m-1"}"#,
            r#"{"type":"hello","matchId":"","friendId":"p-2"}"#,
            r#"{"type":"hello","matchId":"m-1","friendId":7}"#,
        ] {
            assert!(parse_watch_hello(text).is_err(), "{text}");
        }
    }
}
