//! Watching a friend's live game (SPEC S7, B50, B51; #939).
//!
//! `/ws/watch` is authenticated as `/ws/match` is (R1379000, §9.4's gate) and then takes one frame, the
//! hello. It is let in when the two accounts are friends with neither blocking the other and the friend
//! holds a seat in the live 1v1 it names, or a person's seat in the live co-op game it names (R2881000),
//! in which the watcher holds none and whose other person neither blocks nor is blocked by the watcher;
//! no setting turns watching off (R2615000). The match's actor, the 1v1's
//! or the co-op one, then judges room (SPECTATORS_PER_MATCH) and sends the friend's seat's views, each
//! SPECTATE_DELAY_MS after the state it shows, with `legal: []`.
//!
//! Every refusal is one `forbidden` frame and a close with R148's 4403 (4401 before sign-in, 4429 past
//! the account's budget (R2478000), 1011 for a fault), all with one message, so a socket never learns
//! which check said so. A later hello is answered with the view last sent (R1103000, so R1441's wake
//! probe hears a live socket), every other frame is dropped unanswered within R2609000's limits, and
//! before the hello the socket waits behind `HelloGate` (R835002). Being let in is no standing leave
//! (§9.8; B46): an unfriend or a block closes the watcher's socket at once, and so does a block between
//! a co-op watcher and the friend's teammate.
//!
//! S7's `Friend.watchable` is `social::presence::standings`'s, which judges what these checks judge for
//! every friend at once (`Tx::presence_watchable`; `Tx::presence_in_duos` and `Tx::presence_duos_blocked`
//! for a co-op game).

use std::sync::Arc;

use serde_json::Value;

use crate::actor::guard::HelloGate;
use crate::actor::match_actor::WATCH_NOT_ALLOWED as NOT_ALLOWED;
use crate::actor::protocol::{SocketErrorCode, encode, error_message};
use crate::actor::registry::AttachError;
use crate::actor::ws_server::{Socket, UpgradeFailure, WATCH_PATH, WS_CLOSE, authenticate_upgrade};
use crate::app::App;
use crate::db::store::{MatchPhase, MatchStatus, StoreError, Tx};
use crate::duos::registry::{registry, registry_if_made};
use crate::social::friends::are_friends;

/// Which kind of game a hello's match is, once B50's checks let it be watched (R2881000).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Watched {
    /// A live 1v1 match, on `app.matches`.
    Match,
    /// A live co-op game, on the co-op registry (`duos::registry`).
    Duos,
}

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

/// The watch socket's handlers until its hello is answered: the gate `/ws/duos` waits behind too.
pub type WatchListener = HelloGate<WatchHello>;

/// Puts the watch's frame handlers on `socket` before its pump starts: the first hello is kept and
/// every frame after it dropped unread (B50). The handlers hold the gate weakly and `serve` owns it,
/// so a socket refused or gone frees itself; the actor's handlers replace them once it takes it.
pub fn listen(socket: &Socket) -> WatchListener {
    HelloGate::install(socket, parse_watch_hello)
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

impl From<UpgradeFailure> for WatchFailure {
    fn from(failure: UpgradeFailure) -> WatchFailure {
        match failure {
            UpgradeFailure::Refused { code, message } => WatchFailure::Refused { code, message },
            UpgradeFailure::Threw(message) => WatchFailure::Threw(message),
        }
    }
}

/// A watch socket's life after `listen`: who it is, its hello, B50's checks, and the actor; or the
/// one refusal.
pub async fn serve(app: &Arc<App>, socket: &Socket, token: Option<String>, listener: WatchListener) {
    // A socket that went before its handlers were in has no close left to tell them.
    if !socket.is_open() {
        return;
    }
    let refuse = |code: u16, message: &str| {
        socket.send(encode(&error_message(SocketErrorCode::Forbidden, message, None)));
        socket.close(Some(code), Some(message));
    };
    match admit(app, socket, token, listener).await {
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
    token: Option<String>,
    listener: WatchListener,
) -> Result<(), WatchFailure> {
    // The account behind the upgrade, read as `/ws/match` reads it: the token (R1379000), its
    // profile, §9.4's gate.
    let watcher = authenticate_upgrade(app, token, WATCH_PATH).await?;
    let Some(hello) = listener.wait().await else {
        // R835002: still open means no hello came in `WS_IDLE_TIMEOUT_SECONDS`; gone, or closed for
        // its malformed frames, has no one to answer.
        return Err(if socket.is_open() {
            refused(WS_CLOSE.forbidden, NOT_ALLOWED)
        } else {
            WatchFailure::Gone
        });
    };
    let Some(watched) = may_watch(app, &watcher.id, &hello).await? else {
        tracing::info!(event = "watch.refused", matchId = %hello.match_id);
        return Err(refused(WS_CLOSE.forbidden, NOT_ALLOWED));
    };
    let (match_id, friend_id) = (&hello.match_id, &hello.friend_id);
    let attached = match watched {
        Watched::Match => {
            app.matches
                .attach_watcher(app, match_id, friend_id, &watcher.id, socket.clone())
                .await
        }
        // R2881000: a co-op game's watcher is the co-op actor's.
        Watched::Duos => {
            registry(app)
                .attach_watcher(app, match_id, friend_id, &watcher.id, socket.clone())
                .await
        }
    };
    match attached {
        Ok(()) => {
            // An unfriend or a block committed after the checks above, and revoked before this socket
            // was in, is read now; from here on such a change revokes the socket itself.
            if may_watch(app, &watcher.id, &hello).await? != Some(watched) {
                tracing::info!(event = "watch.revoked_at_hello", matchId = %hello.match_id);
                let this_one = |watcher_id: &str, friend_id: &str| {
                    watcher_id == watcher.id && friend_id == hello.friend_id
                };
                revoke(app, this_one, |watcher_id, friend_id, _| {
                    this_one(watcher_id, friend_id)
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

/// B50, as the store answers it: `watcher_id` may watch `hello`'s friend in `hello`'s match, a 1v1 one
/// or a co-op one (R2881000), or none. Room is the actor's to judge, under its own lock
/// (`MatchActor::attach_watcher`, `DuosActor::attach_watcher`).
pub async fn may_watch(
    app: &App,
    watcher_id: &str,
    hello: &WatchHello,
) -> Result<Option<Watched>, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let allowed = watchable_in(&mut tx, watcher_id, &hello.friend_id, &hello.match_id).await?;
    tx.commit().await?;
    Ok(allowed)
}

/// B51: how many sockets watch `match_id` now, a 1v1 match or a co-op game (R2881000); none for one
/// whose actor is not in memory.
pub fn watcher_count(app: &App, match_id: &str) -> usize {
    app.matches.watcher_count(match_id) + registry_if_made(app).map_or(0, |duos| duos.watcher_count(match_id))
}

/// B50, B45: `x` and `y` are no longer friends: a socket of either watching the other's seat, in a
/// 1v1 match or a co-op game, is let go. Answers how many went.
pub fn revoke_between(app: &App, x: &str, y: &str) -> usize {
    let between = |a: &str, b: &str| (a == x && b == y) || (a == y && b == x);
    revoke(app, between, |watcher, friend, _| between(watcher, friend))
}

/// B50, B46: one of `x` and `y` blocked the other: as `revoke_between`, and a socket of either
/// watching a co-op seat whose teammate is the other is let go too, since a watcher is shown the
/// teammate's duel (§13.6, R2881000). Answers how many went.
pub fn revoke_blocked(app: &App, x: &str, y: &str) -> usize {
    let between = |a: &str, b: &str| (a == x && b == y) || (a == y && b == x);
    revoke(app, between, |watcher, friend, teammate| {
        between(watcher, friend) || between(watcher, teammate)
    })
}

/// B50: every watch by `profile_id`, whoever it watches, is let go (its account is deleted, and its
/// friendships with it). Answers how many went.
pub fn revoke_watcher(app: &App, profile_id: &str) -> usize {
    revoke(
        app,
        |watcher, _| watcher == profile_id,
        |watcher, _, _| watcher == profile_id,
    )
}

/// Lets go every watcher of a 1v1 match for which `in_match(watcher's account, its friend's)` holds,
/// and of a co-op game (R2881000) for which `in_duos(watcher's account, its friend's, the friend's
/// teammate's)` does. A co-op registry this app never made holds no watcher.
fn revoke(
    app: &App,
    in_match: impl Fn(&str, &str) -> bool,
    in_duos: impl Fn(&str, &str, &str) -> bool,
) -> usize {
    app.matches.revoke_watchers(in_match)
        + registry_if_made(app).map_or(0, |duos| duos.revoke_watchers(in_duos))
}

/// B50's store checks, in one transaction: `viewer_id` and `friend_id` are friends with neither
/// blocking the other; `match_id` is live, seats `friend_id` and not `viewer_id`. The match is a row
/// of `matches` (a 1v1 match) or, when it is none, of `duos_matches`, whose people hold a1 and a2,
/// and then neither blocks the other with the friend's teammate either, whose duel a watcher is
/// shown (§13.6, R2881000).
async fn watchable_in(
    tx: &mut Tx<'_>,
    viewer_id: &str,
    friend_id: &str,
    match_id: &str,
) -> Result<Option<Watched>, StoreError> {
    if !are_friends(tx, viewer_id, friend_id).await? {
        return Ok(None);
    }
    // `matches_seats` first: it reads any id, where `matches_get` is handed match ids only.
    let Some(seats) = tx.matches_seats(match_id).await? else {
        // R2881000: no 1v1 match; a co-op game, perhaps (`duos_matches_get` reads any id).
        let Some(row) = tx.duos_matches_get(match_id).await? else {
            return Ok(None);
        };
        let seated = |id: &str| row.a1 == id || row.a2 == id;
        if row.ended_at.is_some() || !seated(friend_id) || seated(viewer_id) {
            return Ok(None);
        }
        let teammate = if row.a1 == friend_id { &row.a2 } else { &row.a1 };
        let blocked = tx.blocks_between(viewer_id, teammate).await?;
        return Ok((!blocked).then_some(Watched::Duos));
    };
    if seats.phase != MatchPhase::Live {
        return Ok(None);
    }
    let Some(row) = tx.matches_get(match_id).await? else {
        return Ok(None);
    };
    let seated = |id: &str| row.players.0 == id || row.players.1 == id;
    let live = row.status == MatchStatus::Live && seated(friend_id) && !seated(viewer_id);
    Ok(live.then_some(Watched::Match))
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
