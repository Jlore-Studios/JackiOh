//! The abuse guards a person's socket stands behind (SPEC §9.8): `SeatGuard`, one per human seat,
//! which the 1v1 match actor and the co-op actor (§13.9) each keep (R137, R835000, R835002), and
//! `HelloGate`, which `/ws/duos` and `/ws/watch` hold a socket behind until it says its hello
//! (R835002). Every number is in `crate::config`; none is stated here.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use serde_json::json;
use tokio::sync::Notify;

use crate::actor::contracts::SocketHandlers;
use crate::actor::protocol::{SocketErrorCode, encode, error_message};
use crate::actor::ws_server::Socket;
use crate::api::http::{lock, log_alert};
use crate::app::now_ms;
use crate::config::{
    MALFORMED_FRAMES_PER_WINDOW, MATCH_ACTIONS_PER_SECOND, REJECTION_ALERT_THRESHOLD,
    REJECTION_ALERT_WINDOW_MS, WS_IDLE_TIMEOUT_SECONDS,
};

/// §9.8: the action flood limit, measured over this window.
pub const FLOOD_WINDOW_MS: i64 = 1000;

/// R835002 (CLAUDE.md rule 9: the malformed budget itself is `MALFORMED_FRAMES_PER_WINDOW` in
/// `config.rs`; this close code is wire vocabulary, not a tuned number): RFC 6455's policy
/// violation, sent when a seat's malformed frames cross the budget.
pub const CLOSE_POLICY_VIOLATED: u16 = 1008;

/// Drops the entries of `window` that are `span` or more behind `now`.
fn prune(window: &mut VecDeque<i64>, now: i64, span: i64) {
    while window.front().is_some_and(|at| *at <= now - span) {
        window.pop_front();
    }
}

/// One seat's three windows: its actions, its refusals and its malformed frames.
#[derive(Default)]
pub struct SeatGuard {
    actions: VecDeque<i64>,
    rejections: VecDeque<i64>,
    alerted: bool,
    malformed: VecDeque<i64>,
}

impl SeatGuard {
    /// §9.8's action flood limit, `MATCH_ACTIONS_PER_SECOND` from `crate::config` (R109), held as
    /// one window *per seat*; `false` once the seat is over it, and the action is not counted.
    ///
    /// SPEC §11 R137 settles the scope of the counter: "Per seat, not per match." §9.8 says
    /// "per-match rate limit in the actor" and R109 says "5 actions per second per match", but one
    /// shared per-match counter lets a flooding player spend the *opponent's* budget and have the
    /// victim's legitimate clicks refused, "turning an anti-abuse limit into the abuse". So R137
    /// gives each socket the allowance R109 names and makes the match's aggregate ceiling twice it,
    /// which is also the per-socket reading `docs/architecture.md` §5.1 step 1 asks for
    /// ("rate-limit the socket"). R157 applies the same reasoning to §9.8's per-account half in
    /// `api/http.rs`.
    pub fn action_allowed(&mut self, now: i64) -> bool {
        prune(&mut self.actions, now, FLOOD_WINDOW_MS);
        if self.actions.len() >= MATCH_ACTIONS_PER_SECOND {
            return false;
        }
        self.actions.push_back(now);
        true
    }

    /// R835000: counts one refused action (a reducer refusal or a flood rejection) and answers
    /// the count the first time the seat's window crosses `REJECTION_ALERT_THRESHOLD`: `None`
    /// before that and after the alert has gone, so one alert per match per seat. The window is
    /// pruned to `REJECTION_ALERT_WINDOW_MS` first, so old refusals fall out and the crossing is
    /// strictly over the threshold.
    pub fn note_rejection(&mut self, now: i64) -> Option<usize> {
        prune(&mut self.rejections, now, REJECTION_ALERT_WINDOW_MS);
        self.rejections.push_back(now);
        if self.rejections.len() > REJECTION_ALERT_THRESHOLD && !self.alerted {
            self.alerted = true;
            Some(self.rejections.len())
        } else {
            None
        }
    }

    /// R835002: counts one malformed frame; `true` once the window holds more than
    /// `MALFORMED_FRAMES_PER_WINDOW`, so the seat is a flooder, not a stale client.
    pub fn note_malformed(&mut self, now: i64) -> bool {
        prune(&mut self.malformed, now, REJECTION_ALERT_WINDOW_MS);
        self.malformed.push_back(now);
        self.malformed.len() > MALFORMED_FRAMES_PER_WINDOW
    }
}

/// R835000: raises `match.rejections.unusual` (§9.8) for `profile_id`'s `rejected` refusals in the
/// last `REJECTION_ALERT_WINDOW_MS` of `match_id`.
pub fn alert_unusual_rejections(match_id: &str, profile_id: &str, rejected: usize) {
    log_alert(
        "match.rejections.unusual",
        json!({
            "profileId": profile_id,
            "matchId": match_id,
            "rejected": rejected,
            "windowMs": REJECTION_ALERT_WINDOW_MS,
        }),
    );
}

/// What the gate has heard.
struct GateState<H> {
    /// The first frame that parsed as a hello, once one came.
    hello: Option<H>,
    closed: bool,
    /// The socket's own malformed budget: no seat is known before the hello.
    guard: SeatGuard,
}

struct Gate<H> {
    state: Mutex<GateState<H>>,
    heard: Notify,
    socket: Socket,
    /// The socket's hello grammar: the hello, or the reason a `malformed` frame gives.
    parse: fn(&str) -> Result<H, String>,
}

impl<H> Gate<H> {
    fn receive(&self, text: &str) {
        let mut state = lock(&self.state);
        // B50: after the hello nothing the socket sends is read, and a closed socket is not heard.
        if state.hello.is_some() || state.closed {
            return;
        }
        match (self.parse)(text) {
            Ok(hello) => {
                state.hello = Some(hello);
                drop(state);
                self.heard.notify_one();
            }
            Err(reason) => {
                let spent = state.guard.note_malformed(now_ms());
                // The close calls `closed` back, which takes this lock.
                drop(state);
                self.socket
                    .send(encode(&error_message(SocketErrorCode::Malformed, &reason, None)));
                if spent {
                    self.socket
                        .close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
                }
            }
        }
    }

    fn closed(&self) {
        lock(&self.state).closed = true;
        self.heard.notify_one();
    }
}

/// The socket's handlers until its owner attaches it: they keep the first hello and answer every
/// frame that is not one `malformed`, closing the socket with 1008 past `MALFORMED_FRAMES_PER_WINDOW`
/// of them (R835002). Its handlers hold the gate weakly, so a socket that closes before its hello
/// leaves nothing behind.
pub struct HelloGate<H> {
    gate: Arc<Gate<H>>,
}

impl<H: Clone + Send + 'static> HelloGate<H> {
    /// Installs the gate's handlers on `socket`, `parse` being what makes a frame its hello. Call it
    /// before the transport delivers any frame.
    pub fn install(socket: &Socket, parse: fn(&str) -> Result<H, String>) -> HelloGate<H> {
        let gate = Arc::new(Gate {
            state: Mutex::new(GateState {
                hello: None,
                closed: false,
                guard: SeatGuard::default(),
            }),
            heard: Notify::new(),
            socket: socket.clone(),
            parse,
        });
        let on_message: Weak<Gate<H>> = Arc::downgrade(&gate);
        let on_close = on_message.clone();
        socket.attach(SocketHandlers {
            message: Box::new(move |text: String| {
                if let Some(gate) = on_message.upgrade() {
                    gate.receive(&text);
                }
            }),
            close: Box::new(move || {
                if let Some(gate) = on_close.upgrade() {
                    gate.closed();
                }
            }),
        });
        HelloGate { gate }
    }

    /// The hello once one came; `None` when the socket closed first or said nothing for
    /// `WS_IDLE_TIMEOUT_SECONDS`.
    pub async fn wait(&self) -> Option<H> {
        let limit = Duration::from_secs(u64::try_from(WS_IDLE_TIMEOUT_SECONDS).unwrap_or(0));
        let heard = tokio::time::timeout(limit, async {
            loop {
                let notified = self.gate.heard.notified();
                {
                    let state = lock(&self.gate.state);
                    if let Some(hello) = &state.hello {
                        return Some(hello.clone());
                    }
                    if state.closed {
                        return None;
                    }
                }
                notified.await;
            }
        })
        .await;
        heard.unwrap_or(None)
    }
}
