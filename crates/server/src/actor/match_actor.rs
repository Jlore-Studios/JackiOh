//! One actor per match (SPEC §9.2, §9.3, §9.5, BUILD M6-T4). Port of `apps/server/src/match/actor.ts`.
//!
//! "The match actor is the only stateful component: it holds the state in memory, one WebSocket per
//! player, an alarm for the turn clock, and it appends every resolved action to the log" (§9.2).
//! This file is that, and nothing else: it owns no rules, no timer and no socket library.
//!
//!  - The rules live in `jackioh_engine`, reached through `actor::engine` (SURFACE §11.3: no port).
//!    §9.3: "`reduce` refuses illegal actions itself and returns the reason", so a refusal is relayed
//!    verbatim and never re-derived here.
//!  - The clock lives in `actor::clock` (R79). Every expiry comes back as a `ClockExpiry` and is
//!    dispatched as a *server action* through the same path a client action takes, because
//!    "(seed, log) reconstructs any match" (§9.3) — a timeout that skipped the log would break that.
//!    The mulligan clock (R268) is the one expiry that becomes more than one action: a `timeout` for
//!    each seat still owing its mulligan, in seat order, each its own row.
//!  - The results row lives behind `crate::api::results::record_result` (§9.5, M7-T2).
//!  - The only per-player payload is `engine::view_for(state, player)` (§9.1, §10.8). No socket
//!    ever sees a state, the other hand, library order, or the other seat's view.
//!
//! Every task runs on one serialized queue, so the actor really is single-threaded: a socket frame
//! and a clock alarm can never interleave two `reduce` calls or two log appends. In Rust the queue
//! is one tokio task per match draining an mpsc inbox, one task at a time (each run as its own
//! spawned task, so a task that panics is logged as `match.task.threw` and the queue goes on, as
//! TS's `fireAndForget` caught a throw). What TS ran synchronously outside the queue (taking a
//! socket, parsing a frame, relaying an emote or an aim, a socket going away) runs synchronously
//! here too, on the caller's thread, under the actor's lock — which is never held across an
//! `.await` and never while a socket is closed (a close calls back into the actor).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::{mpsc, oneshot};

use jackioh_engine::{
    Action, ActionBody, Aim, AimEnd, EmoteId, GameEvent, GameOverReason, HandView, PLAYER_IDS, PendingView,
    PerPlayer, PlayerId, PlayerView, PortraitId, aim_key, deal_emote_hand, emote_gate, hand_holds,
    portrait_or_default,
};

use crate::actor::clock::{Timer, after, create_match_clock};
use crate::actor::contracts::{
    ActorDeps, ClockExpiry, ClockView, CreateMatchClockInput, ExpiryHandler, MatchClock, RecordResultInput,
    SocketHandlers, VoidMatchInput, one_tx,
};
use crate::actor::engine::{self, EngineState, LastBoards, MatchSnapshot};
use crate::actor::protocol::{
    ClientMessage, MATCH_VOIDED_CLOSE_REASON, SERVER_NONCE_PREFIX, ServerMessage, SocketErrorCode,
    ack_message, aim_relay_message, clock_message, emote_relay_message, encode, error_message,
    parse_client_message, portraits_message, prompt_for_opponent, prompt_for_you, view_message,
};
use crate::actor::telemetry::{self, LIVE_PILOTS, Recorder};
use crate::actor::ws_server::Socket;
use crate::app::now_ms;
use crate::config::{
    AIM_RELAY_INTERVAL_MS, MATCH_ACTIONS_PER_SECOND, MATCH_RECORD_RESULT_ATTEMPTS,
    MATCH_RECORD_RESULT_BACKOFF_MS, MATCH_VOIDED_CLOSE_CODE,
};
use crate::db::store::{MatchActionRow, MatchClocks, MatchRow, MatchSeat, MatchStatus};
use crate::ranked::ladder::RankTier;

const PLAYERS: [PlayerId; 2] = PLAYER_IDS;

/// §9.8: the action flood limit, measured over this window.
const FLOOD_WINDOW_MS: i64 = 1000;

/// TS `MatchActorInput`.
pub struct MatchActorInput {
    /// TS `match` (a Rust keyword): the row as it stands.
    pub match_row: MatchRow,
    /// The live state, when the caller already has one. `None`, the actor folds `(seed, decks, log)`.
    pub state: Option<EngineState>,
    /// The rows already in `match_actions`: the seq counter and the nonce map are rebuilt from them.
    pub log: Vec<MatchActionRow>,
    /// R679: called once, synchronously, when a Glitch voids the match, before the store forgets it —
    /// the registry drops the actor here, so nothing can reach a match that no longer exists.
    pub on_voided: Option<Box<dyn Fn() + Send + Sync>>,
    /// R1442: the play telemetry a rebuild replayed from `log` (`telemetry::replay`). `None`, the
    /// actor starts recording from the match's creation.
    pub telemetry: Option<Recorder>,
}

/// R417, R678: the boards a match was created with — its seats' last boards and the two other
/// players' boards a Glitch may put on the field — as `create_game` and `fold` take them:
/// `(last_boards, glitch_boards)`.
pub fn last_boards_of(match_row: &MatchRow) -> (Option<LastBoards>, Option<LastBoards>) {
    (match_row.last_boards.clone(), match_row.glitch_boards.clone())
}

/// One task on the actor's serialized queue (TS's `enqueue`d closures, named).
enum Task {
    /// The clock has to be armed before the first action (R79).
    Arm,
    Hello {
        home: PlayerId,
    },
    Action {
        home: PlayerId,
        nonce: String,
        body: ActionBody,
    },
    Submit {
        player: PlayerId,
        nonce: String,
        body: ActionBody,
        reply: oneshot::Sender<ServerMessage>,
    },
    Expire(ClockExpiry),
    Disconnect {
        home: PlayerId,
    },
    Attach {
        home: PlayerId,
    },
    /// TS `idle()`: answered once every task queued before it has run.
    Idle(oneshot::Sender<()>),
}

impl Task {
    /// TS's `what` label for `match.task.threw`.
    fn what(&self) -> String {
        match self {
            Task::Arm => "arm".to_string(),
            Task::Hello { .. } => "hello".to_string(),
            Task::Action { body, .. } | Task::Submit { body, .. } => format!("action:{}", body.action_type()),
            Task::Expire(expiry) => format!("expiry:{}", expiry.kind()),
            Task::Disconnect { .. } => "disconnect".to_string(),
            Task::Attach { .. } => "attach".to_string(),
            Task::Idle(_) => "idle".to_string(),
        }
    }
}

/// R738: one seat's aim relay (see `Core::aims`).
struct AimSlot {
    /// The newest aim received and not yet relayed: `None` when there is none (TS `undefined`),
    /// `Some(None)` an aim that has ended (TS `null`).
    pending: Option<Option<Aim>>,
    /// The one wait armed to relay it.
    timer: Option<Timer>,
    /// When the seat's last relay went; `None` is never (TS `-Infinity`).
    last_at: Option<i64>,
    /// What the opponent was last told, so a repeat is never sent and a seat that leaves mid-aim has
    /// its arrow cleared.
    relayed: Option<Aim>,
}

fn empty_aim_slot() -> AimSlot {
    AimSlot {
        pending: None,
        timer: None,
        last_at: None,
        relayed: None,
    }
}

/// The actor's in-memory state (§9.2) and the append-only log's bookkeeping (§9.3).
struct Core {
    state: EngineState,
    next_seq: i64,
    /// §9.3: "every action carries a client nonce, deduped server-side". `reduce` dedupes internally
    /// too, but the actor must not write a second log row or push a second view, so it keeps the ack
    /// it already answered with. Rebuilt from the log, so a client that reconnects and retries an
    /// action from before the crash still gets its original ack.
    acks: IndexMap<String, ServerMessage>,
    /// Keyed by the seat each account BEGAN in (`home`).
    sockets: PerPlayer<Option<Socket>>,
    /// R744: which accounts (keyed by home, like `sockets`) have had a socket on this actor.
    attached_once: PerPlayer<bool>,
    /// One window per seat; see `flood_exceeded`.
    recent_actions: PerPlayer<VecDeque<i64>>,
    /// R643: each seat's recent emote sends, the one input the shared `emote_gate` decides on. The
    /// client runs the same gate against its own copy, so the two can never disagree about the
    /// limit, and a dropped emote is simply never relayed — not an error, not a rejected action.
    emote_history: PerPlayer<Vec<i64>>,
    /// R738: each seat's aim relay.
    aims: PerPlayer<AimSlot>,
    /// R679: set once a Glitch voided the match; nothing more is sent, written or attached.
    voided: bool,
    /// A rebuilt actor whose log already ends in a result and whose row is already `finished` has
    /// nothing left to record. One that is still `live` crashed between the terminal action and the
    /// results write, so it heals itself on arming (§9.5: every ending records a result).
    finished: bool,
    stopped: bool,
    persisted_clocks: MatchClocks,
    /// R744: the grace deadlines the match row held when this actor was built, keyed by engine seat
    /// as stored, until the first attach hands them to the clock (`start_absent_grace`). Until then a
    /// write of the clocks keeps them, so the arm task's first `persist_clocks` cannot erase them.
    stored_grace: Option<PerPlayer<Option<i64>>>,
    /// R677: whether the accounts hold each other's seat, as of the last state change. Every socket
    /// is keyed by the seat its account began in (`home`), and every engine read by the seat played
    /// now, so this one flag is the whole mapping. Only the engine's state sets it — a Glitch's
    /// swap, which a client reaches only by legally playing Glitch — never a frame.
    swapped: bool,
    last_pending_for: Option<PlayerId>,
    /// R265: which seats owed a mulligan at the last push, so a `prompt` frame goes out on a change.
    last_mulligan_owed: String,
    /// R1442: the match's play telemetry, written once its result has landed.
    telemetry: Recorder,
    /// R1442: each player's ladder tier as the actor armed (seat order, `MatchRow.players`), the
    /// telemetry's rank bucket; `None` until then, or when it could not be read.
    tiers: Option<PerPlayer<RankTier>>,
}

struct Shared {
    deps: ActorDeps,
    match_row: MatchRow,
    /// The seats as the match began: `.0` is the account that started in p1.
    seats: (MatchSeat, MatchSeat),
    /// R642: the pair the `portraits` frame carries, `vanilla` for a match that predates them.
    portraits: (PortraitId, PortraitId),
    /// R1341, R1342: each account's emote hand, keyed by the seat it began in (`home`) like the
    /// sockets, dealt once from the match seed when the actor is built — the same eight after a
    /// restart, a reconnect or a Glitch's swap (R677), since a hand belongs to the player.
    emote_hands: PerPlayer<Vec<EmoteId>>,
    clock: MatchClock,
    inbox: mpsc::UnboundedSender<Task>,
    on_voided: Option<Box<dyn Fn() + Send + Sync>>,
    core: Mutex<Core>,
}

/// TS `MatchActor`: a cheap handle (`Clone`) on one match's actor.
#[derive(Clone)]
pub struct MatchActor {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for MatchActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchActor")
            .field("match_id", &self.shared.match_row.id)
            .finish()
    }
}

/// R677: the seat the account that began in `home` plays now — and, read the other way, the account
/// (by its home) that plays engine seat `seat`; a swap is its own inverse.
fn playing(core: &Core, home: PlayerId) -> PlayerId {
    if core.swapped { home.opponent() } else { home }
}

fn mulligan_key(owed: &[PlayerId]) -> String {
    owed.iter()
        .map(|player| player.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn panic_message(error: tokio::task::JoinError) -> String {
    if !error.is_panic() {
        return error.to_string();
    }
    jackioh_ai::panic_message(&*error.into_panic())
}

/// The serialized queue: one task at a time, so nothing interleaves (TS's `enqueue` chain).
async fn run_queue(shared: Weak<Shared>, mut inbox: mpsc::UnboundedReceiver<Task>) {
    while let Some(task) = inbox.recv().await {
        let Some(strong) = shared.upgrade() else { break };
        let actor = MatchActor { shared: strong };
        let match_id = actor.shared.match_row.id.clone();
        let what = task.what();
        if let Err(error) = tokio::spawn(async move { actor.run(task).await }).await {
            tracing::error!(event = "match.task.threw", matchId = %match_id, what = %what, message = %panic_message(error));
        }
    }
}

/// TS `createMatchActor`. Must be called inside a tokio runtime: it spawns the actor's queue and arms
/// the clock.
pub fn create_match_actor(deps: ActorDeps, input: MatchActorInput) -> MatchActor {
    let MatchActorInput {
        match_row,
        state,
        log,
        on_voided,
        telemetry,
    } = input;
    let seats = (
        MatchSeat {
            profile_id: match_row.players.0.clone(),
            player: PlayerId::P1,
            deck: match_row.decks.0.clone(),
            portrait: None,
        },
        MatchSeat {
            profile_id: match_row.players.1.clone(),
            player: PlayerId::P2,
            deck: match_row.decks.1.clone(),
            portrait: None,
        },
    );

    let state = match state {
        Some(state) => state,
        None => {
            let (last_boards, glitch_boards) = last_boards_of(&match_row);
            let actions = log.iter().map(|row| row.action.clone()).collect();
            engine::fold(&engine::fold_args(
                &match_row.seed,
                &match_row.decks,
                actions,
                last_boards,
                glitch_boards,
                None,
            ))
            .state
        }
    };

    let next_seq = log.iter().fold(0, |highest, row| highest.max(row.seq)) + 1;
    let acks: IndexMap<String, ServerMessage> = log
        .iter()
        .map(|row| (row.action.nonce.clone(), ack_message(&row.action.nonce, row.seq)))
        .collect();

    let portraits = match_row
        .portraits
        .unwrap_or((portrait_or_default(None), portrait_or_default(None)));
    let emote_hands = PerPlayer::new(
        deal_emote_hand(&match_row.seed, PlayerId::P1),
        deal_emote_hand(&match_row.seed, PlayerId::P2),
    );
    let opening = engine::snapshot(&state);
    let telemetry = telemetry.unwrap_or_else(|| Recorder::new(&match_row.id, match_row.created_at, &opening));

    let core = Core {
        state,
        next_seq,
        acks,
        sockets: PerPlayer::new(None, None),
        attached_once: PerPlayer::new(false, false),
        recent_actions: PerPlayer::new(VecDeque::new(), VecDeque::new()),
        emote_history: PerPlayer::new(Vec::new(), Vec::new()),
        aims: PerPlayer::new(empty_aim_slot(), empty_aim_slot()),
        voided: false,
        finished: matches!(match_row.status, MatchStatus::Finished),
        stopped: false,
        persisted_clocks: match_row.clocks.clone(),
        stored_grace: Some(match_row.clocks.grace_deadline.clone()),
        swapped: opening.seats_swapped,
        last_pending_for: opening.pending_for,
        last_mulligan_owed: mulligan_key(&mulligan_window(&opening)),
        telemetry,
        tiers: None,
    };

    let (inbox, inbox_rx) = mpsc::unbounded_channel();
    let expiries = inbox.downgrade();
    let on_expire: ExpiryHandler = Arc::new(move |expiry: ClockExpiry| {
        if let Some(inbox) = expiries.upgrade() {
            let _ = inbox.send(Task::Expire(expiry));
        }
    });
    let clock = create_match_clock(CreateMatchClockInput {
        started_at: match_row.created_at,
        on_expire,
    });

    let shared = Arc::new(Shared {
        deps,
        match_row,
        seats,
        portraits,
        emote_hands,
        clock,
        inbox,
        on_voided,
        core: Mutex::new(core),
    });
    let actor = MatchActor { shared };

    // The clock has to be armed before the first action, not on the first one (R79: the turn clock
    // is already running when the match opens). TS's arm task ran its synchronous half (the sync)
    // in the microtask after `createMatchActor` returned, so the actor a caller got back was already
    // armed: the same here, before anything can read it; the queued half persists the clocks and
    // resolves a match that was already over.
    actor.shared.clock.sync(&clock_view_for(&opening));
    actor.enqueue(Task::Arm);
    tokio::spawn(run_queue(Arc::downgrade(&actor.shared), inbox_rx));
    actor
}

impl MatchActor {
    fn lock(&self) -> MutexGuard<'_, Core> {
        self.shared.core.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn match_id_str(&self) -> &str {
        &self.shared.match_row.id
    }

    /// TS `fireAndForget(task)`: onto the queue; a task that panics is logged by `run_queue`.
    fn enqueue(&self, task: Task) {
        if self.shared.inbox.send(task).is_err() {
            tracing::warn!(event = "match.queue.closed", matchId = %self.match_id_str());
        }
    }

    async fn run(&self, task: Task) {
        match task {
            Task::Arm => {
                let snapshot = engine::snapshot(&self.lock().state);
                self.shared.clock.sync(&clock_view_for(&snapshot));
                self.persist_clocks().await;
                // R1442: the rank bucket is the tier each player holds as the match arms.
                if !self.lock().finished {
                    let tiers =
                        telemetry::ladder_tiers_or_none(&self.shared.deps, &self.shared.match_row.players)
                            .await;
                    self.lock().tiers = tiers;
                }
                if snapshot.result.is_some() {
                    self.on_terminal(&snapshot).await;
                }
            }
            Task::Hello { home } => {
                // §9.5: "Reconnect gets a fresh full view, never a log replay."
                let core = self.lock();
                let player = playing(&core, home);
                self.push_view(&core, player);
                self.push_clock(&core, player);
                // R642: portraits ride again on a reconnect, as on join, with the account's hand (R1342).
                self.send_to_home(&core, home, &self.portraits_for(home));
            }
            Task::Action { home, nonce, body } => {
                // R677: the seat is the one this account plays when the action runs — a swap queued
                // ahead of it has already moved the account — and the reply goes to the account that
                // sent it.
                let player = playing(&self.lock(), home);
                let reply = self.apply_action(player, nonce, body).await;
                let core = self.lock();
                self.send_to_home(&core, home, &reply);
            }
            Task::Submit {
                player,
                nonce,
                body,
                reply,
            } => {
                let answer = self.apply_action(player, nonce, body).await;
                let _ = reply.send(answer);
            }
            Task::Expire(expiry) => self.on_expire(expiry).await,
            Task::Disconnect { home } => {
                // §9.5: grace starts, and "the clock keeps running while a player is disconnected" — on
                // the seat this account plays (R677).
                let player = playing(&self.lock(), home);
                self.shared.clock.start_grace(player, None);
                self.persist_clocks().await;
                let core = self.lock();
                self.push_clock(&core, player.opponent());
            }
            Task::Attach { home } => {
                let player = {
                    let mut core = self.lock();
                    let player = playing(&core, home);
                    self.shared.clock.clear_grace(player);
                    // R744: an account that has not been here yet is away from now.
                    self.start_absent_grace(&mut core);
                    player
                };
                self.persist_clocks().await;
                // §9.5: a fresh full view, never a log replay. R642: the portraits ride with it, and
                // R1342 the account's own emote hand with them.
                let core = self.lock();
                self.push_view(&core, player);
                self.push_clock(&core, player);
                self.send_to_home(&core, home, &self.portraits_for(home));
                self.push_clock(&core, player.opponent());
            }
            Task::Idle(done) => {
                let _ = done.send(());
            }
        }
    }

    // ---------------------------------------------------------------------
    // Sending. §10.8: a socket only ever carries this player's own view.
    // ---------------------------------------------------------------------

    /// R642, R1342: the portraits frame for the account that began in `home`, with its own hand.
    fn portraits_for(&self, home: PlayerId) -> ServerMessage {
        portraits_message(
            self.shared.portraits.0,
            self.shared.portraits.1,
            &self.shared.emote_hands[home],
        )
    }

    /// Sends to the account playing engine seat `player` now.
    fn send(&self, core: &Core, player: PlayerId, message: &ServerMessage) {
        self.send_to_home(core, playing(core, player), message);
    }

    /// Sends to the account that began the match in `home`, whatever seat it plays.
    fn send_to_home(&self, core: &Core, home: PlayerId, message: &ServerMessage) {
        let Some(socket) = &core.sockets[home] else { return };
        if !socket.is_open() {
            return;
        }
        socket.send(encode(message));
    }

    fn view_of(&self, core: &Core, player: PlayerId) -> PlayerView {
        // R79: the engine never reads a clock, so the view's `clockMs` is filled in here from the one
        // component that knows. Nothing else about the view is touched.
        let view = engine::view_for(&core.state, player);
        PlayerView {
            clock_ms: self
                .shared
                .clock
                .remaining_for(player)
                .map(|ms| i32::try_from(ms).unwrap_or(i32::MAX)),
            ..view
        }
    }

    /// §10.8's view and, beside it, BUILD M5-T2's array ("the client never computes legality itself;
    /// it asks `legalActions` and greys out the rest").
    ///
    /// `player` is passed to BOTH calls, which is the whole security property: a socket is only ever
    /// handed the actions its own seat may take. `legal_actions(state, player.opponent())` would name
    /// every `play` in the opponent's hand and so hand over the hidden information §9.1 lists first.
    ///
    /// Both are read off the same `state` under the same lock, so the array is always true of the
    /// view it travels with — the actor's queue means no `reduce` can land between them.
    fn push_view(&self, core: &Core, player: PlayerId) {
        let message = view_message(
            self.view_of(core, player),
            &engine::legal_actions(&core.state, player),
        );
        self.send(core, player, &message);
    }

    fn push_clock(&self, core: &Core, player: PlayerId) {
        self.send(
            core,
            player,
            &clock_message(now_ms(), self.shared.clock.snapshot()),
        );
    }

    // ---------------------------------------------------------------------
    // The clock (R79, §9.5)
    // ---------------------------------------------------------------------

    /// R79, exactly: the turn clock ends the active player's turn; a prompt clock answers only that
    /// prompt; grace becomes a loss; the ceiling becomes a draw. R268 adds the mulligan clock, which
    /// times out every seat still owing its mulligan. Each one is a real action, appended to the log
    /// like any other, because `(seed, log)` must reconstruct the match (§9.3).
    fn server_actions_for(&self, core: &Core, expiry: ClockExpiry) -> Vec<(PlayerId, ActionBody)> {
        let snapshot = engine::snapshot(&core.state);
        match expiry {
            ClockExpiry::Turn { player } | ClockExpiry::Prompt { player } => {
                vec![(player, ActionBody::Timeout)]
            }
            // R268: one clock for both seats, so on expiry every seat still owing is timed out — read
            // when the expiry runs, since a seat may have answered between the alarm and this task — in
            // seat order, each stamped with its own seat (R146: a timeout belongs to the player whose
            // clock ran out). What a timed-out mulligan keeps is the engine's business (R268: the whole
            // hand); a seat that has already answered is owed nothing and gets no row.
            ClockExpiry::Mulligan => mulligan_window(&snapshot)
                .into_iter()
                .map(|player| (player, ActionBody::Timeout))
                .collect(),
            // SPEC §11 R146: "a disconnect timeout belongs to the player who disconnected, because the
            // loss is theirs". `disconnectExpired` names the player in its body (§10.2), so the seat
            // the action is *stamped* with would otherwise be free; R146 fixes it so folding the log
            // reads one seat rather than "whoever happened to be active".
            ClockExpiry::Grace { player } => vec![(player, ActionBody::DisconnectExpired { player })],
            // SPEC §11 R146: reaching the turn ceiling "belongs to neither and is stamped with the
            // active seat as a convention", so a fold never has to guess. R79 makes it a draw and R112
            // covers the reaper's version of the same ending.
            ClockExpiry::Ceiling => vec![(snapshot.active, ActionBody::CeilingReached)],
        }
    }

    async fn on_expire(&self, expiry: ClockExpiry) {
        let actions = {
            let core = self.lock();
            if core.stopped || core.finished {
                return;
            }
            self.server_actions_for(&core, expiry)
        };
        for (player, body) in actions {
            // `stop()` sets `stopped` from outside the queue, so it can land between two of R268's
            // timeouts; nothing is dispatched into an actor that is going away or a match that is over.
            let next_seq = {
                let core = self.lock();
                if core.stopped || core.finished {
                    return;
                }
                core.next_seq
            };
            tracing::info!(event = "match.clock.expired", matchId = %self.match_id_str(), kind = expiry.kind(), player = %player);
            // The nonce is derived from the seq this action will occupy: unique, and stable across a
            // rebuild, so a rebuilt actor cannot collide with a nonce already in the log. Its prefix is
            // one no client may send (R270), so no client can pre-empt it.
            let nonce = format!("{SERVER_NONCE_PREFIX}{}-{next_seq}", expiry.kind());
            self.apply_action(player, nonce, body).await;
        }
    }

    /// The clocks as stored: the clock's own, plus any stored grace not yet handed to it (R744).
    fn clocks_to_store(&self, core: &Core) -> MatchClocks {
        let clocks = self.shared.clock.snapshot();
        let Some(stored) = &core.stored_grace else {
            return clocks;
        };
        let grace = PerPlayer::new(
            clocks.grace_deadline.p1.or(stored.p1),
            clocks.grace_deadline.p2.or(stored.p2),
        );
        MatchClocks {
            grace_deadline: grace,
            ..clocks
        }
    }

    async fn persist_clocks(&self) {
        let clocks = {
            let mut core = self.lock();
            let clocks = self.clocks_to_store(&core);
            if clocks == core.persisted_clocks {
                return;
            }
            core.persisted_clocks = clocks.clone();
            clocks
        };
        // §9.5: "the grace countdown is stored on the match so both clients show it".
        let written = one_tx!(self.shared.deps.db, |t| t
            .matches_set_clocks(self.match_id_str(), &clocks)
            .await?);
        if let Err(error) = written {
            tracing::warn!(event = "match.clocks.persistFailed", matchId = %self.match_id_str(), message = %error);
        }
    }

    // ---------------------------------------------------------------------
    // Applying an action
    // ---------------------------------------------------------------------

    /// Pushes everything the clients need after a state change, in one order for both seats.
    async fn after_change(&self) {
        let snapshot = {
            let mut core = self.lock();
            let snapshot = engine::snapshot(&core.state);
            if snapshot.seats_swapped != core.swapped {
                self.on_seats_swapped(&mut core, snapshot.seats_swapped);
            }
            self.shared.clock.sync(&clock_view_for(&snapshot));
            snapshot
        };
        self.persist_clocks().await;

        {
            let mut core = self.lock();
            for player in PLAYERS {
                self.push_view(&core, player);
            }
            self.push_prompts(&mut core, &snapshot);
            for player in PLAYERS {
                self.push_clock(&core, player);
            }
            // R1442: a seat's think time runs from this push.
            core.telemetry.on_pushed(&snapshot, now_ms());
        }

        if snapshot.result.is_some() {
            self.on_terminal(&snapshot).await;
        }
    }

    /// The `prompt` frames for a change in who owes an answer. Each is built off that seat's own
    /// `view_for`, so a frame never carries more than the view it travels with (§10.8).
    fn push_prompts(&self, core: &mut Core, snapshot: &MatchSnapshot) {
        let deadline = self.shared.clock.snapshot().prompt_deadline;

        // R265, R266: while both mulligans are open neither seat holds `pending`, and each is sent the
        // frame that fits it — a seat that owes its mulligan its own prompt, a seat that has answered
        // only that the other seat still owes one. The sealed answer is in neither: R266 makes that a
        // seat is ready public and what it kept private, and the opponent's frame names no choice.
        let owed = mulligan_window(snapshot);
        let owed_key = mulligan_key(&owed);
        if !owed.is_empty() && owed_key != core.last_mulligan_owed {
            for player in PLAYERS {
                if !owed.contains(&player) {
                    self.send(core, player, &prompt_for_opponent(player.opponent(), deadline));
                    continue;
                }
                let view = engine::view_for(&core.state, player);
                if let Some(PendingView::ForYou(pending)) = &view.pending {
                    self.send(
                        core,
                        player,
                        &prompt_for_you(player, &pending.choice_id, pending.kind, deadline),
                    );
                }
            }
        }
        core.last_mulligan_owed = owed_key;

        // §10.6: one prompt at a time; the player who does not hold it learns only that it is open.
        let pending_for = snapshot.pending_for;
        if let Some(holder) = pending_for
            && Some(holder) != core.last_pending_for
        {
            let view = engine::view_for(&core.state, holder);
            if let Some(PendingView::ForYou(pending)) = &view.pending {
                self.send(
                    core,
                    holder,
                    &prompt_for_you(holder, &pending.choice_id, pending.kind, deadline),
                );
            }
            self.send(core, holder.opponent(), &prompt_for_opponent(holder, deadline));
        }
        core.last_pending_for = pending_for;
    }

    /// R677: a Glitch swapped the seats. From here every socket reads the view of, and acts for, the
    /// other seat. A disconnect grace belongs to the account, so one running moves with it to the seat
    /// it plays now (restarted there: the clock keeps one deadline per seat, not per account).
    fn on_seats_swapped(&self, core: &mut Core, now: bool) {
        let away: Vec<PlayerId> = PLAYERS
            .into_iter()
            .filter(|home| core.sockets[*home].is_none())
            .collect();
        for home in &away {
            self.shared.clock.clear_grace(playing(core, *home));
        }
        core.swapped = now;
        for home in &away {
            self.shared.clock.start_grace(playing(core, *home), None);
        }
        tracing::info!(event = "match.seats.swapped", matchId = %self.match_id_str(), swapped = core.swapped);
    }

    /// R677: the seats as the results writer credits them — engine seat p1 with the account that plays
    /// it NOW, and its deck — so the winning seat's current account gets the win (Elo included), and
    /// the last board each seat ended with goes to the account that ended it.
    fn credited_seats(&self, swapped: bool) -> (MatchSeat, MatchSeat) {
        let (first, second) = &self.shared.match_row.players;
        let (first_deck, second_deck) = &self.shared.match_row.decks;
        (
            MatchSeat {
                profile_id: if swapped { second } else { first }.clone(),
                player: PlayerId::P1,
                deck: first_deck.clone(),
                portrait: None,
            },
            MatchSeat {
                profile_id: if swapped { first } else { second }.clone(),
                player: PlayerId::P2,
                deck: second_deck.clone(),
                portrait: None,
            },
        )
    }

    /// R679: a Glitch voided the match. It never happened: no result, no rating move, no last board
    /// and no game record. Both sockets are closed with `MATCH_VOIDED_CLOSE_CODE` (the last view they
    /// got already shows the game over with reason `voided`), the registry drops the actor, and
    /// `void_match` removes the match from the store and logs the one line abuse checks read.
    async fn on_void(&self) {
        let sockets = {
            let mut core = self.lock();
            core.voided = true;
            core.stopped = true;
            PLAYERS.map(|home| core.sockets[home].take())
        };
        for socket in sockets.into_iter().flatten() {
            if socket.is_open() {
                socket.close(Some(MATCH_VOIDED_CLOSE_CODE), Some(MATCH_VOIDED_CLOSE_REASON));
            }
        }
        if let Some(on_voided) = &self.shared.on_voided {
            on_voided();
        }
        let input = VoidMatchInput {
            match_id: self.shared.match_row.id.clone(),
            players: self.shared.match_row.players.clone(),
            at: now_ms(),
        };
        if let Err(error) = crate::api::results::void_match(&self.shared.deps, input).await {
            // The row stays live, so the next socket for it rebuilds this actor, folds to the same void
            // and tries again.
            tracing::error!(event = "match.void.failed", matchId = %self.match_id_str(), message = %error);
        }
    }

    /// §9.5: "Every ending records a result and clears both players' in-match state." Once.
    async fn on_terminal(&self, snapshot: &MatchSnapshot) {
        let Some(result) = snapshot.result else { return };
        let (swapped, last_boards) = {
            let mut core = self.lock();
            if core.finished {
                return;
            }
            core.finished = true;
            self.shared.clock.stop();
            // R738: no aim outlives the game.
            for player in PLAYERS {
                self.end_aim(&mut core, player);
            }
            (core.swapped, engine::last_boards(&core.state))
        };
        // R679: the one ending that records nothing. Only the engine's state reaches it.
        if result.reason == GameOverReason::Voided {
            self.on_void().await;
            return;
        }
        let at = now_ms();

        // The results row, the rating move and clearing `inMatchId` are `api/results.rs` (M7-T2); the
        // actor never writes them itself.
        let input = RecordResultInput {
            match_id: self.shared.match_row.id.clone(),
            // R677: credited by who plays each seat now, not by who began in it.
            seats: self.credited_seats(swapped),
            outcome: result,
            turns: snapshot.turn,
            at,
            // R417, R565: each seat's board as the game ended, from its own side.
            last_boards: Some(last_boards),
        };
        // The write finishes the match row in the same transaction, so nothing else here does: a row
        // finished apart from it would outlive a failed write, with no result and both players stuck
        // in the match. R1437: a write that never lands leaves the match `live` for the next socket.
        if !self.record_result_with_retries(input).await {
            self.let_go_unrecorded();
            return;
        }
        // R1442: the play telemetry goes after the result, so its write can never cost it.
        let played = {
            let core = self.lock();
            core.telemetry.finish(&LIVE_PILOTS, core.tiers.as_ref())
        };
        telemetry::record(&self.shared.deps, &played).await;

        tracing::info!(
            event = "match.over",
            matchId = %self.match_id_str(),
            winner = %result.winner,
            reason = %result.reason,
            turns = snapshot.turn,
        );
    }

    /// R1437: `record_result`, tried up to `MATCH_RECORD_RESULT_ATTEMPTS` times, the first wait
    /// `MATCH_RECORD_RESULT_BACKOFF_MS` and each next one twice the last. A repeat is safe: a write
    /// that finds the match's row already written answers that row (`api/results.rs`). It runs on the
    /// actor's queue, which the game's end has left nothing to do but refuse actions and push views.
    /// Answers whether the result landed.
    async fn record_result_with_retries(&self, input: RecordResultInput) -> bool {
        let mut wait_ms = MATCH_RECORD_RESULT_BACKOFF_MS;
        for attempt in 1..=MATCH_RECORD_RESULT_ATTEMPTS {
            match crate::api::results::record_result(&self.shared.deps, input.clone()).await {
                Ok(_) => return true,
                Err(error) => {
                    tracing::error!(event = "match.recordResult.failed", matchId = %self.match_id_str(), attempt = attempt, message = %error);
                }
            }
            if attempt < MATCH_RECORD_RESULT_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(wait_ms as u64)).await;
                wait_ms *= 2;
            }
        }
        false
    }

    /// R1437: the result never landed, so the match is still `live` and both players are still in it.
    /// The registry forgets this actor (as R679's void does, since `stop` would wait on the queue this
    /// runs in) and both sockets close as `stop` closes them, which the client reconnects on: the next
    /// socket rebuilds the actor from the log, and the rebuilt actor writes the result as it arms.
    fn let_go_unrecorded(&self) {
        self.shared.deps.matches.forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = "match.result.unrecorded", matchId = %self.match_id_str());
    }

    /// The one path every action takes — a client's and the clock's alike. Reduce, append exactly one
    /// log row at the next gapless seq, then push.
    async fn apply_action(&self, player: PlayerId, nonce: String, body: ActionBody) -> ServerMessage {
        let (action, result, seq) = {
            let core = self.lock();
            if let Some(existing) = core.acks.get(&nonce) {
                return existing.clone();
            }
            if core.finished {
                return error_message(
                    SocketErrorCode::MatchOver,
                    "this match already has a result",
                    Some(&nonce),
                );
            }
            // §9.1: the seat is the server's, never the client's.
            let action = Action::new(body, player, nonce.clone());
            let result = engine::reduce(&core.state, &action);
            (action, result, core.next_seq)
        };

        if let Some(reason) = &result.error {
            // §9.8: "Every rejected action is logged with its reason."
            tracing::warn!(
                event = "match.action.rejected",
                matchId = %self.match_id_str(),
                player = %player,
                "type" = %action.action_type(),
                reason = %reason,
            );
            return error_message(SocketErrorCode::IllegalAction, reason, Some(&nonce));
        }

        // §9.3: append-only. The row goes in *before* the new state is committed, so a failed write
        // can never leave the in-memory state ahead of the log that has to reconstruct it.
        let row = MatchActionRow {
            match_id: self.shared.match_row.id.clone(),
            seq,
            action,
            at: now_ms(),
        };
        let appended = one_tx!(self.shared.deps.db, |t| t
            .matches_append_actions(std::slice::from_ref(&row))
            .await?);
        if let Err(error) = appended {
            tracing::error!(event = "match.append.failed", matchId = %self.match_id_str(), seq = seq, message = %error);
            return error_message(
                SocketErrorCode::Internal,
                "the action could not be recorded; try again",
                Some(&nonce),
            );
        }

        let ack = ack_message(&nonce, seq);
        {
            let mut guard = self.lock();
            let core = &mut *guard;
            // R1442: the move as it arrived, on the state it was made in.
            core.telemetry.on_action(
                &core.state,
                &row.action,
                seq,
                row.at,
                self.shared.clock.remaining_for(player),
                result.events.last().map(GameEvent::event_type),
            );
            core.state = result.state;
            core.next_seq = seq + 1;
            core.acks.insert(nonce, ack.clone());
        }

        self.after_change().await;
        ack
    }

    // ---------------------------------------------------------------------
    // Sockets
    // ---------------------------------------------------------------------

    /// A frame from the account that began in `home`; the seat it acts for is read as it runs.
    fn on_frame(&self, home: PlayerId, text: &str) {
        let message = parse_client_message(text);
        let mut core = self.lock();
        let player = playing(&core, home);

        match message {
            Err(malformed) => {
                // A bad frame is answered, not fatal: the actor stays up and the other seat is untouched.
                tracing::warn!(event = "match.frame.malformed", matchId = %self.match_id_str(), player = %player, reason = %malformed.reason);
                self.send_to_home(
                    &core,
                    home,
                    &error_message(SocketErrorCode::Malformed, &malformed.reason, None),
                );
            }
            Ok(ClientMessage::Hello(_)) => {
                // §9.5: "Reconnect gets a fresh full view, never a log replay."
                self.enqueue(Task::Hello { home });
            }
            Ok(ClientMessage::Emote(emote)) => {
                // R1342: an emote outside the sender's dealt hand is refused first, as silently as a
                // rate-limited one and without spending the limit: the menu never offers it, so only
                // a stale or a hand-made client sends one.
                if !hand_holds(&self.shared.emote_hands[home], emote.emote) {
                    tracing::warn!(event = "match.emote.outside_hand", matchId = %self.match_id_str(), player = %player, emote = %emote.emote);
                    return;
                }
                // R643: same gate the client ran. A fail is a silent drop — no error, no rejected-action
                // log; a pass relays to the opponent only (the sender already showed it locally).
                let now = now_ms();
                let gate = emote_gate(&core.emote_history[home], now);
                if !gate.ok {
                    return;
                }
                let mut sent_at = gate.sent_at;
                sent_at.push(now);
                core.emote_history[home] = sent_at;
                self.send(
                    &core,
                    player.opponent(),
                    &emote_relay_message(player, emote.emote),
                );
                // R1442: recorded until the result, which the telemetry is written with.
                if !core.finished {
                    let turn = core.state.turn;
                    core.telemetry.on_emote(home, emote.emote, turn, now);
                }
            }
            Ok(ClientMessage::Aim(aim)) => self.receive_aim(&mut core, player, aim.aim),
            Ok(ClientMessage::Action(action)) => {
                let now = now_ms();
                if flood_exceeded(&mut core, home, now) {
                    // §9.8: reject the overflow; the socket stays open.
                    tracing::warn!(event = "match.action.flooded", matchId = %self.match_id_str(), player = %player);
                    self.send_to_home(
                        &core,
                        home,
                        &error_message(
                            SocketErrorCode::RateLimited,
                            "too many actions; slow down",
                            Some(&action.nonce),
                        ),
                    );
                    return;
                }
                self.enqueue(Task::Action {
                    home,
                    nonce: action.nonce,
                    body: action.body,
                });
            }
        }
    }

    // ---------------------------------------------------------------------
    // The opponent's aim (R738)
    // ---------------------------------------------------------------------

    /// R738: an aim is relayed to the opponent at most once per `AIM_RELAY_INTERVAL_MS` per seat.
    /// Faster ones are coalesced, not dropped outright: the newest waits out the interval and goes
    /// alone, so the opponent's arrow always ends where the sender's did. Nothing is answered.
    fn receive_aim(&self, core: &mut Core, player: PlayerId, aim: Option<Aim>) {
        if core.finished || core.stopped {
            return;
        }
        let slot = &mut core.aims[player];
        slot.pending = Some(aim);
        if slot.timer.is_some() {
            return;
        }
        let wait = match slot.last_at {
            None => -1,
            Some(last_at) => last_at + AIM_RELAY_INTERVAL_MS - now_ms(),
        };
        if wait <= 0 {
            self.flush_aim(core, player);
            return;
        }
        let actor = Arc::downgrade(&self.shared);
        slot.timer = Some(after(wait, move |id| {
            let Some(shared) = actor.upgrade() else { return };
            let actor = MatchActor { shared };
            let mut core = actor.lock();
            if core.aims[player].timer.as_ref().map(Timer::id) != Some(id) {
                return;
            }
            if let Some(timer) = core.aims[player].timer.take() {
                timer.fired();
            }
            actor.flush_aim(&mut core, player);
        }));
    }

    /// Relays the seat's pending aim, checked against the state as it stands NOW rather than when the
    /// frame arrived: an aim whose ends name anything the opponent may not see is dropped silently,
    /// and the opponent's arrow is cleared instead if it showed one (R738, R97). A repeat of what
    /// the opponent was last told is not sent again.
    fn flush_aim(&self, core: &mut Core, player: PlayerId) {
        let Some(aim) = core.aims[player].pending.take() else {
            return;
        };
        if core.finished || core.stopped {
            return;
        }
        let receiver = player.opponent();
        let shown = match aim {
            None => None,
            Some(aim) => {
                if aim_is_public(&aim, player, &engine::view_for(&core.state, receiver)) {
                    Some(aim)
                } else {
                    None
                }
            }
        };
        if aim_key(shown.as_ref()) == aim_key(core.aims[player].relayed.as_ref()) {
            return;
        }
        core.aims[player].last_at = Some(now_ms());
        core.aims[player].relayed = shown;
        self.send(core, receiver, &aim_relay_message(player, shown));
    }

    /// A seat that leaves, or a match that ends, takes its arrow with it.
    fn end_aim(&self, core: &mut Core, player: PlayerId) {
        let slot = &mut core.aims[player];
        if let Some(timer) = slot.timer.take() {
            timer.cancel();
        }
        slot.pending = None;
        if slot.relayed.is_none() {
            return;
        }
        slot.relayed = None;
        self.send(core, player.opponent(), &aim_relay_message(player, None));
    }

    fn on_socket_gone(&self, home: PlayerId) {
        {
            let mut core = self.lock();
            // A socket that has already been replaced (the seat holds its open successor) or dropped
            // by `stop()` is not a disconnect: only the seat's own socket, now closed, is.
            if core.sockets[home].as_ref().is_none_or(Socket::is_open) {
                return;
            }
            core.sockets[home] = None;
            if core.stopped || core.finished {
                return;
            }
            // R738: the aim of the seat this account plays (R677) ends with its socket.
            let player = playing(&core, home);
            self.end_aim(&mut core, player);
            tracing::info!(event = "match.socket.closed", matchId = %self.match_id_str(), player = %player, home = %home);
        }
        self.enqueue(Task::Disconnect { home });
    }

    /// R744: an account that has had no socket on this actor (the actor was rebuilt after a restart,
    /// or the player never opened the match) is disconnected, so its grace starts with the first
    /// socket that does attach, on the seat it plays (R677): at the deadline stored on the match when
    /// there is one, else a fresh window. Nothing starts at boot, so a match no socket returns to stays
    /// the reaper's draw (R112) and nobody loses by seat order. Safe to repeat: a running grace keeps
    /// its deadline (R147).
    fn start_absent_grace(&self, core: &mut Core) {
        for home in PLAYERS {
            if core.attached_once[home] {
                continue;
            }
            let player = playing(core, home);
            let stored = core.stored_grace.as_ref().and_then(|grace| grace[player]);
            self.shared.clock.start_grace(player, stored);
        }
        core.stored_grace = None;
    }

    // ---------------------------------------------------------------------
    // The actor's surface (TS `MatchActor`)
    // ---------------------------------------------------------------------

    pub fn match_id(&self) -> &str {
        &self.shared.match_row.id
    }

    /// The seats as the match began: `.0` is the account that started in p1.
    pub fn seats(&self) -> &(MatchSeat, MatchSeat) {
        &self.shared.seats
    }

    /// The seat a profile BEGAN the match in, or `None` when it is not in this match. It names the
    /// account's connection, not the seat it plays: after a Glitch's swap (R677) the account that
    /// began in p1 plays p2, and the actor routes its socket there (`playing`).
    pub fn seat_of(&self, profile_id: &str) -> Option<PlayerId> {
        let (first, second) = &self.shared.seats;
        [first, second]
            .into_iter()
            .find(|seat| seat.profile_id == profile_id)
            .map(|seat| seat.player)
    }

    /// Hands a connection to the account that began in `home` (`seat_of`). Replaces (and closes) a
    /// socket that account already held.
    pub fn attach(&self, home: PlayerId, socket: Socket) {
        let previous = {
            let mut core = self.lock();
            if core.voided {
                None
            } else {
                let previous = core.sockets[home].replace(socket.clone());
                core.attached_once[home] = true;
                Some(previous)
            }
        };
        let Some(previous) = previous else {
            // R679: the match no longer exists; a socket that arrives late hears only that.
            socket.close(Some(MATCH_VOIDED_CLOSE_CODE), Some(MATCH_VOIDED_CLOSE_REASON));
            return;
        };

        let on_message = Arc::downgrade(&self.shared);
        let on_close = on_message.clone();
        socket.attach(SocketHandlers {
            message: Box::new(move |text: String| {
                if let Some(shared) = on_message.upgrade() {
                    MatchActor { shared }.on_frame(home, &text);
                }
            }),
            close: Box::new(move || {
                if let Some(shared) = on_close.upgrade() {
                    MatchActor { shared }.on_socket_gone(home);
                }
            }),
        });
        if let Some(previous) = previous
            && previous != socket
        {
            previous.close(Some(1000), Some("replaced by a new socket"));
        }

        let player = playing(&self.lock(), home);
        tracing::info!(event = "match.socket.attached", matchId = %self.match_id_str(), player = %player, home = %home);
        self.enqueue(Task::Attach { home });
    }

    /// §9.5: lets go of that account's socket and starts its disconnect grace.
    pub fn detach(&self, home: PlayerId) {
        let socket = self.lock().sockets[home].clone();
        let Some(socket) = socket else { return };
        if socket.is_open() {
            socket.close(Some(1000), Some("detached"));
        }
        // A transport that does not call back synchronously still leaves the seat empty.
        self.on_socket_gone(home);
    }

    /// Applies one action as engine seat `player` — the seat is stamped here, never taken from the
    /// client (§9.1). Answers the `ack` or the `error` the client is sent.
    pub async fn submit(&self, player: PlayerId, nonce: String, body: ActionBody) -> ServerMessage {
        let (reply, answer) = oneshot::channel();
        let failed = error_message(
            SocketErrorCode::Internal,
            "the match is not running",
            Some(&nonce),
        );
        self.enqueue(Task::Submit {
            player,
            nonce,
            body,
            reply,
        });
        answer.await.unwrap_or(failed)
    }

    /// Resolves when the actor's queue has drained (every task queued before this call has run).
    pub async fn idle(&self) {
        let (done, drained) = oneshot::channel();
        if self.shared.inbox.send(Task::Idle(done)).is_err() {
            return;
        }
        let _ = drained.await;
    }

    pub fn view_for(&self, player: PlayerId) -> PlayerView {
        let core = self.lock();
        self.view_of(&core, player)
    }

    pub fn snapshot(&self) -> MatchSnapshot {
        engine::snapshot(&self.lock().state)
    }

    /// For the registry's hash checks and the reaper; never sent to a socket.
    pub fn engine_state(&self) -> EngineState {
        self.lock().state.clone()
    }

    pub fn clocks(&self) -> MatchClocks {
        self.shared.clock.snapshot()
    }

    /// Drops the actor: stops the clock and lets both sockets go, leaving the log alone.
    pub async fn stop(&self) {
        self.halt();
        self.idle().await;
    }

    /// `stop` without waiting for the queue to drain, so a task on the queue can call it.
    fn halt(&self) {
        let sockets = {
            let mut core = self.lock();
            core.stopped = true;
            self.shared.clock.stop();
            for player in PLAYERS {
                if let Some(timer) = core.aims[player].timer.take() {
                    timer.cancel();
                }
            }
            PLAYERS.map(|home| core.sockets[home].take())
        };
        for socket in sockets.into_iter().flatten() {
            if socket.is_open() {
                socket.close(Some(1001), Some("the actor is going away"));
            }
        }
    }

    /// R672: which seats hold an open socket right now. The registry's `presence_of` reads this, so a
    /// rematch offer learns whether the opponent is still on the match. A seat is present while it
    /// holds a socket that is still open.
    pub fn presence(&self) -> PerPlayer<bool> {
        let core = self.lock();
        let open = |home: PlayerId| core.sockets[home].as_ref().is_some_and(Socket::is_open);
        PerPlayer::new(open(PlayerId::P1), open(PlayerId::P2))
    }
}

/// §9.8's action flood limit, `MATCH_ACTIONS_PER_SECOND` from `crate::config` (R109), held as one
/// window *per seat*.
///
/// SPEC §11 R137 settles the scope of the counter: "Per seat, not per match." §9.8 says
/// "per-match rate limit in the actor" and R109 says "5 actions per second per match", but one
/// shared per-match counter lets a flooding player spend the *opponent's* budget and have the
/// victim's legitimate clicks refused, "turning an anti-abuse limit into the abuse". So R137 gives
/// each socket the allowance R109 names and makes the match's aggregate ceiling twice it, which is
/// also the per-socket reading `docs/architecture.md` §5.1 step 1 asks for ("rate-limit the
/// socket"). R157 applies the same reasoning to §9.8's per-account half in `api/http.rs`.
fn flood_exceeded(core: &mut Core, player: PlayerId, now: i64) -> bool {
    let recent = &mut core.recent_actions[player];
    while recent.front().is_some_and(|at| *at <= now - FLOOD_WINDOW_MS) {
        recent.pop_front();
    }
    if recent.len() >= MATCH_ACTIONS_PER_SECOND {
        return true;
    }
    recent.push_back(now);
    false
}

fn clock_view_for(snapshot: &MatchSnapshot) -> ClockView {
    ClockView {
        turn: snapshot.turn,
        active: snapshot.active,
        pending_for: snapshot.pending_for,
        mulligan_owed: snapshot.mulligan_owed.clone(),
        over: snapshot.result.is_some() || snapshot.phase == jackioh_engine::Phase::Over,
    }
}

/// R738, R97, R177: whether every end of `sender`'s aim names something `view` (the receiver's own)
/// shows. A hero and a zone of the field are public wherever they are — a face-down card is named
/// by its zone, never itself — so a zone need only exist. A hand card is named by its position, and
/// only in the sender's own hand, which the receiver sees as that many backs.
pub fn aim_is_public(aim: &Aim, sender: PlayerId, view: &PlayerView) -> bool {
    match &aim.source {
        AimEnd::Hand { player, index } => {
            if *player != sender {
                return false;
            }
            let hand = if sender == view.viewer {
                &view.you.hand
            } else {
                &view.opponent.hand
            };
            let count = match hand {
                HandView::Cards(cards) => cards.len(),
                HandView::Count { count } => usize::try_from(*count).unwrap_or(0),
            };
            if *index >= count {
                return false;
            }
        }
        source => {
            if !end_is_public(source, view) {
                return false;
            }
        }
    }
    match &aim.target {
        None => true,
        Some(AimEnd::Hand { .. }) => false,
        Some(target) => end_is_public(target, view),
    }
}

fn end_is_public(end: &AimEnd, view: &PlayerView) -> bool {
    match end {
        AimEnd::Hand { .. } => false,
        AimEnd::Hero { .. } => true,
        AimEnd::Zone { player, row, lane } => {
            let side = if *player == view.viewer {
                &view.you
            } else {
                &view.opponent
            };
            i64::from(*lane) <= side.locks.row(*row).len() as i64
        }
    }
}

/// R265: the seats that owe a mulligan while the window is open — both mulligans open and no other
/// prompt in front of them — and nothing otherwise. The engine never reports both at once; the
/// clock (`clock.rs`) reads the window the same way.
fn mulligan_window(snapshot: &MatchSnapshot) -> Vec<PlayerId> {
    if snapshot.pending_for.is_none() && snapshot.result.is_none() {
        snapshot.mulligan_owed.clone()
    } else {
        Vec::new()
    }
}
