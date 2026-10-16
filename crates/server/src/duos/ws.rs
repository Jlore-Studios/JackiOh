//! The `/ws/duos` socket's first moments (fullsend SPEC S6, B35): authentication as `/ws/match` has
//! it, then the `hello` that names the match, then the hand-off to the co-op actor.
//!
//! The upgrade itself (origin check, per-address count, frame cap, subprotocol) is
//! `actor::ws_server::handle_duos`, the same doors `/ws/match` goes through. It installs a
//! `HelloGate` on the socket before the socket's pump starts, so a `hello` the client sends the
//! moment the socket opens is held while the token is checked rather than dropped. The gate is the
//! one `/ws/watch` waits behind too (`actor::guard`, R835002): it answers a frame that is not the
//! hello `malformed` and closes the socket past `MALFORMED_FRAMES_PER_WINDOW` of them. Then
//! `handle_duos_socket` verifies the token (`?token=`), applies §9.4's gate (`assert_active`), waits
//! for the hello (at most `WS_IDLE_TIMEOUT_SECONDS`), and attaches the socket to the seat the profile
//! holds in that match, folding the actor back first when it is not in memory. Every refusal answers
//! one `error` frame with code `forbidden` and closes with R148's codes (4401, 4403, 4404, and 1011
//! for a fault), so a socket learns that it may not have the match, never which check said so.
//!
//! The match is the hello's `matchId`, else the upgrade's `?matchId=`, else the profile's live co-op
//! match (B42).

use std::sync::Arc;

use jackioh_engine::wire::DuosSeat;

use crate::actor::guard;
use crate::actor::protocol::SocketErrorCode;
use crate::actor::ws_server::{Socket, WS_CLOSE};
use crate::api::http::{ApiErrorCode, assert_active};
use crate::app::App;
use crate::db::store::{Profile, StoreError};
use crate::duos::live_match_of;
use crate::duos::protocol::{DuosClientMessage, encode, error_message, parse_duos_message};
use crate::duos::registry::{DuosAttachError, registry};

/// The socket's handlers until the actor attaches it (`actor::guard::HelloGate`, which `/ws/watch`
/// shares): they keep the first hello, the match it names being optional.
pub struct HelloGate(guard::HelloGate<Option<String>>);

impl HelloGate {
    /// Installs the gate's handlers on `socket`. Call it before the transport delivers any frame.
    pub fn install(socket: &Socket) -> HelloGate {
        HelloGate(guard::HelloGate::install(socket, duos_hello))
    }
}

/// The hello's `matchId`, or the reason a frame that is not the hello gives.
fn duos_hello(text: &str) -> Result<Option<String>, String> {
    match parse_duos_message(text) {
        Ok(DuosClientMessage::Hello { match_id }) => Ok(match_id),
        Ok(_) => Err("send hello with your matchId first".to_string()),
        Err(malformed) => Err(malformed.reason),
    }
}

/// Why a socket is refused: R148's close code and the message, or a fault.
enum Refusal {
    Refused { code: u16, message: String },
    Threw(String),
}

impl Refusal {
    fn refused(code: u16, message: &str) -> Refusal {
        Refusal::Refused {
            code,
            message: message.to_string(),
        }
    }
}

impl From<StoreError> for Refusal {
    fn from(error: StoreError) -> Refusal {
        Refusal::Threw(error.to_string())
    }
}

/// The token's profile, active (§9.4), as `/ws/match` authenticates its sockets.
async fn authenticate(app: &Arc<App>, token: Option<String>) -> Result<Profile, Refusal> {
    let Some(token) = token.filter(|token| !token.is_empty()) else {
        return Err(Refusal::refused(WS_CLOSE.unauthorized, "sign in first"));
    };
    let Ok(user) = app.auth.verify(&token).await else {
        return Err(Refusal::refused(WS_CLOSE.unauthorized, "sign in first"));
    };
    let mut tx = app.db.begin(None).await?;
    let profile = tx.profiles_get_by_user_id(&user.user_id).await?;
    tx.commit().await?;
    let Some(profile) = profile else {
        return Err(Refusal::refused(WS_CLOSE.unauthorized, "sign in first"));
    };
    assert_active(&profile).map_err(|error| {
        let code = if error.code == ApiErrorCode::NotFound {
            WS_CLOSE.not_found
        } else {
            WS_CLOSE.forbidden
        };
        Refusal::Refused {
            code,
            message: error.message.clone(),
        }
    })?;
    Ok(profile)
}

async fn attach_socket(
    app: &Arc<App>,
    socket: &Socket,
    gate: &HelloGate,
    token: Option<String>,
    query_match_id: Option<String>,
) -> Result<DuosSeat, Refusal> {
    let profile = authenticate(app, token).await?;
    let Some(asked) = gate.0.wait().await else {
        return Err(Refusal::refused(
            WS_CLOSE.forbidden,
            "send hello with your matchId first",
        ));
    };
    let match_id = match asked.or(query_match_id) {
        Some(match_id) => match_id,
        None => match live_match_of(app, &profile.id).await? {
            Some(match_id) => match_id,
            None => {
                return Err(Refusal::refused(
                    WS_CLOSE.not_found,
                    "you are not in a co-op match",
                ));
            }
        },
    };
    registry(app)
        .attach(app, &match_id, &profile.id, socket.clone())
        .await
        .map_err(|error| match error {
            DuosAttachError::NotFound => Refusal::refused(WS_CLOSE.not_found, "no such match"),
            DuosAttachError::Internal(message) => Refusal::Threw(message),
        })
}

/// B35: authenticates an upgraded `/ws/duos` socket, waits for its hello and hands it to the co-op
/// actor, or refuses it with one error frame and a close code. `gate` is the one
/// `HelloGate::install` put on the socket before its transport started.
pub async fn handle_duos_socket(
    app: &Arc<App>,
    socket: &Socket,
    gate: HelloGate,
    token: Option<String>,
    query_match_id: Option<String>,
) {
    let refuse = |code: u16, message: &str| {
        socket.send(encode(&error_message(SocketErrorCode::Forbidden, message, None)));
        socket.close(Some(code), Some(message));
    };
    match attach_socket(app, socket, &gate, token, query_match_id).await {
        Ok(_) => {}
        Err(Refusal::Refused { code, message }) => refuse(code, &message),
        Err(Refusal::Threw(message)) => {
            tracing::error!(event = "duos.ws.upgrade.threw", message = %message);
            refuse(WS_CLOSE.internal, "something went wrong");
        }
    }
    // The gate goes here: on attach the actor's handlers replaced its own.
}
