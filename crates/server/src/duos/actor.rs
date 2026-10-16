//! One actor per Duos co-op match (fullsend SPEC S6, B35–B40), as `actor/match_actor.rs` is one per
//! 1v1 match: it holds the `DuosState` in memory, one socket per person (seats a1 and a2), the clocks
//! and the AI's seats, and it appends every applied action to `duos_match_actions`.
//!
//!  - The rules are the engine's Duos wrapper (`duos::engine`): `duos_reduce` refuses an illegal
//!    action and the actor relays its reason verbatim (B35); nothing here restates a rule.
//!  - The only per-seat payload is `duos_view_for(state, seat)` with `duos_legal_actions` for the same
//!    seat (B23), so a socket never sees an opponent's hidden card or another seat's array.
//!  - Every action goes through one path, a person's and the AI's alike (`apply`): reduce, append one
//!    row at the next gapless seq, then push. A rebuilt actor folds `(seed, options, log)`
//!    (`duos_fold`, B40) and answers a nonce it has logged with its original ack.
//!  - The AI plays b1 and b2 always, a person's seat whose disconnect outlived the grace until they
//!    are back (B37), a seat whose round clock ran out (B38) and a prompt whose clock ran out: each
//!    seat waits `DUOS_AI_STEP_DELAY_MS`, then `jackioh_ai::duos_decide` decides on the blocking pool,
//!    off the runtime as a ladder bot's search is (R1452), and its move comes back through `apply`
//!    with a server nonce (`srv-ai-<seq>`, R270's prefix). The AI never concedes and never offers or
//!    takes a draw (R188).
//!  - Clocks live here, never in the engine: the mulligan clock over the four open mulligans, a round
//!    clock for a round with a person in it, a prompt clock for a prompt a person holds outside one, and
//!    each person's disconnect grace (the 1v1's `DISCONNECT_GRACE_SECONDS`). None of them is an action
//!    the engine sees: an expiry hands the seat to the AI, whose moves are ordinary logged actions, so
//!    `(seed, log)` still folds the match exactly. The mulligan clock's expiry keeps each owing
//!    person's whole hand (§13.2, R268): the AI's own mulligan never answers for a person (it may for a
//!    seat past its grace, which the AI plays outright).
//!  - The hard ceiling (§9.5, R1047000) is the one clock the engine does see: `MATCH_CEILING_MS` after
//!    the row's `created_at`, never stored, the actor sends the engine's own `ceilingReached` as one
//!    server action, and its `ceiling` draw ends the game and writes the result like any other end.
//!    A rebuilt actor that arms past the deadline sends it at once, before anyone plays on.
//!  - A move the AI decided is played only while the AI still plays the seat the way it decided it:
//!    one decided for a seat past its grace is dropped once its person is back, whatever clock has
//!    run out for them meanwhile, and the seat decides again (B37, B38).
//!  - Every engine call runs under the catalog the match was dealt from (R767000, `Pinned`), so a
//!    rebuilt actor plays on as the match began.
//!  - A `hello` repeated while one for the same socket is still queued is absorbed by it (§9.1, as
//!    R835001 has a 1v1 socket's): a flood costs one view push, not one per frame.
//!  - Pings are relayed to the teammate's socket only, rate-limited, and never logged, folded or
//!    viewed (B39).
//!
//! Every task runs on one serialized queue (one tokio task draining an inbox, each task run as its
//! own spawned task so one that panics is logged and the queue goes on), and the lock is never held
//! across an `.await` or while a socket is closed (a close calls back into the actor).

use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use jackioh_ai::{AI_BUDGET, AiOptions, duos_decide};
use jackioh_engine::duos::DuosState;
use jackioh_engine::wire::{ActionBody, DUOS_SEATS, DuosAction, DuosActionBody, DuosSeat, DuosView};
use jackioh_engine::{Rng, mulligan_owed};

use crate::actor::clock::{Timer, after};
use crate::actor::contracts::{SocketHandlers, one_tx};
use crate::actor::protocol::{SERVER_NONCE_PREFIX, SocketErrorCode};
use crate::actor::registry::panic_text;
use crate::actor::ws_server::Socket;
use crate::app::{App, now_ms};
use crate::bots::seat::r188_forbidden;
use crate::config::{
    BOT_DECISION_DEADLINE_MS, BOT_WRITE_RETRY_MS, DISCONNECT_GRACE_MS, DUOS_AI_STEP_DELAY_MS,
    DUOS_MULLIGAN_CLOCK_MS, DUOS_PROMPT_CLOCK_MS, DUOS_ROUND_CLOCK_MS, MALFORMED_FRAMES_PER_WINDOW,
    MATCH_ACTIONS_PER_SECOND, MATCH_RECORD_RESULT_ATTEMPTS, MATCH_RECORD_RESULT_BACKOFF_MS,
    MATCH_SERVER_ACTION_RETRY_MS, PING_RATE_LIMIT, PING_RATE_WINDOW_MS, REJECTION_ALERT_WINDOW_MS,
};
use crate::db::store::StoreError;
use crate::duos::ceiling_at;
use crate::duos::engine::{self, Pinned};
use crate::duos::protocol::{
    DuosClientMessage, DuosServerMessage, GraceDeadlines, PingAt, PingKind, PingTarget, ack_message, encode,
    error_message, parse_duos_message,
};
use crate::duos::registry::registry;
use crate::duos::store::{DuosActionRow, DuosMatch, action_json, seat_of_logged};

/// The people's seats: the room's host and guest (S6). Their sockets, graces and AI takeovers are
/// indexed in this order.
pub const HUMAN_SEATS: [DuosSeat; 2] = [DuosSeat::A1, DuosSeat::A2];

/// §9.8: the action flood limit is counted over this window (as the 1v1 actor's).
const FLOOD_WINDOW_MS: i64 = 1000;

/// RFC 6455's policy violation: a seat past `MALFORMED_FRAMES_PER_WINDOW` (R835002).
const CLOSE_POLICY_VIOLATED: u16 = 1008;

/// RFC 6455's normal closure and "going away".
const CLOSE_NORMAL: u16 = 1000;
const CLOSE_GOING_AWAY: u16 = 1001;

/// A seat's index in `DUOS_SEATS` order (a1, a2, b1, b2).
fn slot(seat: DuosSeat) -> usize {
    match seat {
        DuosSeat::A1 => 0,
        DuosSeat::A2 => 1,
        DuosSeat::B1 => 2,
        DuosSeat::B2 => 3,
    }
}

/// A person's index in `HUMAN_SEATS`, or none for an AI seat.
fn human(seat: DuosSeat) -> Option<usize> {
    match seat {
        DuosSeat::A1 => Some(0),
        DuosSeat::A2 => Some(1),
        DuosSeat::B1 | DuosSeat::B2 => None,
    }
}

// ---------------------------------------------------------------------------
// Reading the state (public bookkeeping only; nothing here is shown to a socket)
// ---------------------------------------------------------------------------

/// The id of the prompt `seat` holds in its duel, if any.
fn held_prompt(state: &DuosState, seat: DuosSeat) -> Option<String> {
    let duel = state.duels.get(seat.duel())?;
    duel.pending
        .as_ref()
        .filter(|pending| pending.player_id == seat.player())
        .map(|pending| pending.id.clone())
}

/// Whether `seat` still owes its mulligan (B2).
fn owes_mulligan(state: &DuosState, seat: DuosSeat) -> bool {
    state
        .duels
        .get(seat.duel())
        .is_some_and(|duel| mulligan_owed(duel).contains(&seat.player()))
}

/// Whether `seat` owes an answer: a prompt or its mulligan.
fn owes_answer(state: &DuosState, seat: DuosSeat) -> bool {
    held_prompt(state, seat).is_some() || owes_mulligan(state, seat)
}

/// Whether `seat` is its duel's round seat and has not ended (B4).
fn acting_round_seat(state: &DuosState, seat: DuosSeat) -> bool {
    state.round >= 1
        && state.round_seats.get(seat.duel()) == Some(&seat)
        && !state.ended.get(seat.duel()).copied().unwrap_or(true)
}

fn is_end_turn(body: &DuosActionBody) -> bool {
    matches!(
        body,
        DuosActionBody::Duel {
            body: ActionBody::EndTurn
        }
    )
}

fn is_answer(body: &DuosActionBody) -> bool {
    matches!(
        body,
        DuosActionBody::Duel {
            body: ActionBody::Answer { .. } | ActionBody::Mulligan { .. }
        }
    )
}

/// R188: what the AI never plays for any seat: a concession (the team's or the duel's), a draw offer,
/// an accepted draw or an emote.
fn ai_forbidden(body: &DuosActionBody) -> bool {
    match body {
        DuosActionBody::Concede => true,
        DuosActionBody::Duel { body } => r188_forbidden(body),
        DuosActionBody::Pass { .. } => false,
    }
}

/// What the AI plays when `duos_decide` gave nothing it may play: ending the turn, else an answer,
/// else the first legal action it may take.
fn fallback(legal: &[DuosActionBody]) -> Option<DuosActionBody> {
    let allowed = || legal.iter().filter(|body| !ai_forbidden(body));
    allowed()
        .find(|body| is_end_turn(body))
        .or_else(|| allowed().find(|body| is_answer(body)))
        .or_else(|| allowed().next())
        .cloned()
}

// ---------------------------------------------------------------------------
// The AI's seats
// ---------------------------------------------------------------------------

/// Why a person's seat (a1, a2) is in the AI's hands for a while.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Forced {
    /// The mulligan clock (§13.2, R268): its mulligan is answered by keeping the whole hand.
    KeepHand,
    /// B38: this round, its prompts are answered and its turn ended for it.
    FinishRound(i32),
    /// The prompt clock: this one prompt is answered for it.
    AnswerPrompt(String),
}

/// How the AI plays a seat now.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    /// b1 and b2, and a person's seat past its grace (B36, B37): everything it owes.
    Full,
    /// Its mulligan, by keeping the whole hand: never the AI's own mulligan (R268).
    KeepHand,
    /// Its prompts and mulligan, then its end of turn (B38).
    FinishRound,
    /// That prompt only.
    AnswerPrompt(String),
}

/// Whether `seat` owes the game something the AI would play in `mode`.
fn owes(pinned: Pinned, state: &DuosState, seat: DuosSeat, mode: &Mode) -> bool {
    match mode {
        Mode::Full => engine::seats_to_act(pinned, state).contains(&seat),
        Mode::KeepHand => owes_mulligan(state, seat),
        Mode::FinishRound => {
            owes_answer(state, seat) || engine::legal(pinned, state, seat).iter().any(is_end_turn)
        }
        Mode::AnswerPrompt(choice) => held_prompt(state, seat).as_deref() == Some(choice.as_str()),
    }
}

/// R268: the mulligan that keeps every card of the hand, the legal one keeping the most.
fn keep_whole_hand(legal: &[DuosActionBody]) -> Option<DuosActionBody> {
    legal
        .iter()
        .filter_map(|body| match body {
            DuosActionBody::Duel {
                body: ActionBody::Mulligan { keep },
            } => Some((keep.len(), body)),
            _ => None,
        })
        .max_by_key(|(kept, _)| *kept)
        .map(|(_, body)| body.clone())
}

/// What the AI plays for `seat` in `mode` without a decision of its own: the whole hand kept, the
/// turn ended, or `fallback`'s pick.
fn without_search(
    mode: &Mode,
    state: &DuosState,
    seat: DuosSeat,
    legal: &[DuosActionBody],
) -> Option<DuosActionBody> {
    match mode {
        Mode::KeepHand => keep_whole_hand(legal),
        Mode::FinishRound if !owes_answer(state, seat) => {
            legal.iter().find(|body| is_end_turn(body)).cloned()
        }
        _ => fallback(legal),
    }
}

/// One decision the AI owes a seat, taken off the actor and run on the blocking pool.
struct AiJob {
    state: DuosState,
    seat: DuosSeat,
    mode: Option<Mode>,
    seed: String,
    cursor: u32,
    pinned: Pinned,
}

impl AiJob {
    /// B26–B29: `duos_decide` on the seat's own stream (`{seed}:duos-ai:{seat}`, the server's, never
    /// the match rng), within `BOT_DECISION_DEADLINE_MS` of search. Answers the move, or none, and
    /// where the stream stands.
    fn decide(&self) -> (Option<DuosActionBody>, u32) {
        jackioh_cards::register_all();
        let Some(mode) = &self.mode else {
            return (None, self.cursor);
        };
        if !owes(self.pinned, &self.state, self.seat, mode) {
            return (None, self.cursor);
        }
        let legal = engine::legal(self.pinned, &self.state, self.seat);
        match mode {
            Mode::KeepHand => (keep_whole_hand(&legal), self.cursor),
            Mode::FinishRound if !owes_answer(&self.state, self.seat) => {
                (legal.iter().find(|body| is_end_turn(body)).cloned(), self.cursor)
            }
            _ => {
                let (body, cursor) = self.ask();
                (body.or_else(|| fallback(&legal)), cursor)
            }
        }
    }

    fn ask(&self) -> (Option<DuosActionBody>, u32) {
        let wait = Duration::from_millis(u64::try_from(BOT_DECISION_DEADLINE_MS).unwrap_or(0));
        let deadline = std::time::Instant::now() + wait;
        let should_stop = move || std::time::Instant::now() >= deadline;
        // The practice AI's budget (the browser's), cut short by the deadline.
        let mut options = AiOptions {
            rng: Rng::new(&format!("{}:duos-ai:{}", self.seed, self.seat), self.cursor),
            budget: AI_BUDGET,
            should_stop: Some(&should_stop),
        };
        let body = self
            .pinned
            .run(|| duos_decide(&self.state, self.seat, &mut options))
            .filter(|body| !ai_forbidden(body));
        (body, options.rng.cursor())
    }
}

// ---------------------------------------------------------------------------
// The actor
// ---------------------------------------------------------------------------

/// What a new or rebuilt actor starts from.
pub struct DuosActorInput {
    pub row: DuosMatch,
    /// The live state: begun (a new match) or folded from `log` (a rebuild).
    pub state: DuosState,
    /// The rows already logged: the seq counter and the nonce map are rebuilt from them.
    pub log: Vec<DuosActionRow>,
    /// R767000: the catalog the match was dealt from, which every engine call runs under.
    pub pinned: Pinned,
}

/// Which clock ran out, with the timer that ran it, so an expiry a later arm replaced is ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expiry {
    Mulligan { timer: u64 },
    Round { round: i32, timer: u64 },
    Prompt { seat: DuosSeat, timer: u64 },
    Grace { seat: DuosSeat, timer: u64 },
    Ceiling { timer: u64 },
}

impl Expiry {
    fn kind(&self) -> &'static str {
        match self {
            Expiry::Mulligan { .. } => "mulligan",
            Expiry::Round { .. } => "round",
            Expiry::Prompt { .. } => "prompt",
            Expiry::Grace { .. } => "grace",
            Expiry::Ceiling { .. } => "ceiling",
        }
    }
}

/// One task on the actor's serialized queue.
enum Task {
    Arm,
    /// R835001: `generation` is the socket generation that sent it (`Core::socket_generation`).
    Hello {
        seat: DuosSeat,
        generation: u64,
    },
    Attach {
        seat: DuosSeat,
    },
    Disconnect {
        seat: DuosSeat,
    },
    Action {
        seat: DuosSeat,
        nonce: String,
        body: DuosActionBody,
    },
    Submit {
        seat: DuosSeat,
        nonce: String,
        body: DuosActionBody,
        reply: oneshot::Sender<DuosServerMessage>,
    },
    Expire(Expiry),
    /// The move the AI decided for `seat`, in `mode`, on the state as it stood when the log's next
    /// seq was `version`.
    AiMove {
        seat: DuosSeat,
        version: i64,
        cursor: u32,
        mode: Option<Mode>,
        body: Option<DuosActionBody>,
    },
    Idle(oneshot::Sender<()>),
}

impl Task {
    fn what(&self) -> String {
        match self {
            Task::Arm => "arm".to_string(),
            Task::Hello { .. } => "hello".to_string(),
            Task::Attach { .. } => "attach".to_string(),
            Task::Disconnect { .. } => "disconnect".to_string(),
            Task::Action { .. } | Task::Submit { .. } => "action".to_string(),
            Task::Expire(expiry) => format!("expiry:{}", expiry.kind()),
            Task::AiMove { .. } => "ai-move".to_string(),
            Task::Idle(_) => "idle".to_string(),
        }
    }
}

/// A deadline and the timer that reports it.
struct Countdown {
    deadline: i64,
    timer: Timer,
}

fn armed_by(countdown: Option<&Countdown>, timer: u64) -> bool {
    countdown.is_some_and(|countdown| countdown.timer.id() == timer)
}

struct Core {
    state: DuosState,
    next_seq: i64,
    /// §9.3: nonce -> the seat that sent it and the seq it was logged at, rebuilt from the log.
    acks: IndexMap<String, (DuosSeat, i64)>,
    /// By `HUMAN_SEATS`.
    sockets: [Option<Socket>; 2],
    /// B37: the AI plays this person's seat until they are back.
    ai_playing: [bool; 2],
    grace: [Option<Countdown>; 2],
    mulligan: Option<Countdown>,
    mulligan_expired: bool,
    /// The round the clock runs for, and its countdown.
    round_clock: Option<(i32, Countdown)>,
    /// The round whose clock already ran out, so it is not armed again.
    round_expired: Option<i32>,
    /// By `HUMAN_SEATS`: the prompt the clock runs for, and its countdown.
    prompt_clocks: [Option<(String, Countdown)>; 2],
    /// By `HUMAN_SEATS`: the prompt whose clock already ran out.
    prompt_expired: [Option<String>; 2],
    /// R1047000: the hard ceiling's countdown (from the row), or its retry's.
    ceiling: Option<Countdown>,
    /// By `DUOS_SEATS` (only a person's is ever set).
    forced: [Option<Forced>; 4],
    /// By `DUOS_SEATS`: a decision is under way for it.
    ai_busy: [bool; 4],
    /// By `DUOS_SEATS`: where each seat's AI stream stands. A rebuilt actor starts them again.
    ai_cursor: [u32; 4],
    /// By `HUMAN_SEATS`: §9.8's per-seat action window (R137).
    recent_actions: [VecDeque<i64>; 2],
    /// By `HUMAN_SEATS`: R835002's malformed frames.
    malformed_at: [VecDeque<i64>; 2],
    /// By `HUMAN_SEATS`: B39's ping window.
    pings: [VecDeque<i64>; 2],
    /// By `HUMAN_SEATS` (R835001's rule): the socket generation whose Hello is queued and unrun, so
    /// a repeat from that socket is absorbed; a Hello from a newer socket always enqueues.
    hello_pending: [Option<u64>; 2],
    /// By `HUMAN_SEATS`: bumped on every attach, the first included.
    socket_generation: [u64; 2],
    /// The result is written (or the row already had one when the actor was built).
    finished: bool,
    finished_at: Option<i64>,
    stopped: bool,
}

struct Shared {
    app: Arc<App>,
    row: DuosMatch,
    /// R767000: the catalog every engine call runs under.
    pinned: Pinned,
    inbox: mpsc::UnboundedSender<Task>,
    core: Mutex<Core>,
}

/// A cheap handle (`Clone`) on one co-op match's actor.
#[derive(Clone)]
pub struct DuosActor {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for DuosActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuosActor")
            .field("match_id", &self.shared.row.id)
            .finish()
    }
}

fn join_error_text(error: tokio::task::JoinError) -> String {
    if error.is_panic() {
        panic_text(error.into_panic())
    } else {
        error.to_string()
    }
}

/// The serialized queue: one task at a time.
async fn run_queue(shared: Weak<Shared>, mut inbox: mpsc::UnboundedReceiver<Task>) {
    while let Some(task) = inbox.recv().await {
        let Some(strong) = shared.upgrade() else { break };
        let actor = DuosActor { shared: strong };
        let match_id = actor.shared.row.id.clone();
        let what = task.what();
        if let Err(error) = tokio::spawn(async move { actor.run(task).await }).await {
            tracing::error!(event = "duos.task.threw", matchId = %match_id, what = %what, message = %join_error_text(error));
        }
    }
}

/// Builds the actor and arms it. Must be called inside a tokio runtime.
pub fn create_duos_actor(app: Arc<App>, input: DuosActorInput) -> DuosActor {
    let DuosActorInput {
        row,
        state,
        log,
        pinned,
    } = input;
    let next_seq = log.iter().fold(0, |highest, entry| highest.max(entry.seq)) + 1;
    let acks: IndexMap<String, (DuosSeat, i64)> = log
        .iter()
        .filter_map(|entry| {
            seat_of_logged(&entry.action).map(|seat| (entry.nonce.clone(), (seat, entry.seq)))
        })
        .collect();
    let finished = row.ended_at.is_some();
    let core = Core {
        state,
        next_seq,
        acks,
        sockets: [None, None],
        ai_playing: [false, false],
        grace: [None, None],
        mulligan: None,
        mulligan_expired: false,
        round_clock: None,
        round_expired: None,
        prompt_clocks: [None, None],
        prompt_expired: [None, None],
        ceiling: None,
        forced: [None, None, None, None],
        ai_busy: [false; 4],
        ai_cursor: [0; 4],
        recent_actions: [VecDeque::new(), VecDeque::new()],
        malformed_at: [VecDeque::new(), VecDeque::new()],
        pings: [VecDeque::new(), VecDeque::new()],
        hello_pending: [None, None],
        socket_generation: [0, 0],
        finished,
        finished_at: finished.then(now_ms),
        stopped: false,
    };
    let (inbox, inbox_rx) = mpsc::unbounded_channel();
    let shared = Arc::new(Shared {
        app,
        row,
        pinned,
        inbox,
        core: Mutex::new(core),
    });
    let actor = DuosActor { shared };
    actor.enqueue(Task::Arm);
    tokio::spawn(run_queue(Arc::downgrade(&actor.shared), inbox_rx));
    actor
}

impl DuosActor {
    fn lock(&self) -> MutexGuard<'_, Core> {
        self.shared.core.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn match_id_str(&self) -> &str {
        &self.shared.row.id
    }

    fn enqueue(&self, task: Task) {
        if self.shared.inbox.send(task).is_err() {
            tracing::warn!(event = "duos.queue.closed", matchId = %self.match_id_str());
        }
    }

    async fn run(&self, task: Task) {
        match task {
            Task::Arm => self.on_arm().await,
            Task::Hello { seat, generation } => {
                // §9.5: a fresh full view, never a log replay. R835001: the flag is cleared only for
                // the generation that set it, so a newer socket's queued Hello stays marked.
                let mut core = self.lock();
                if let Some(index) = human(seat)
                    && core.hello_pending[index] == Some(generation)
                {
                    core.hello_pending[index] = None;
                }
                self.push_seat(&core, seat);
            }
            Task::Attach { seat } => self.on_attach(seat),
            Task::Disconnect { seat } => self.on_disconnect(seat),
            Task::Action { seat, nonce, body } => {
                let reply = self.apply(seat, nonce, body).await;
                let core = self.lock();
                self.send(&core, seat, &reply);
            }
            Task::Submit {
                seat,
                nonce,
                body,
                reply,
            } => {
                let answer = self.apply(seat, nonce, body).await;
                let _ = reply.send(answer);
            }
            Task::Expire(Expiry::Ceiling { timer }) => self.on_ceiling(timer).await,
            Task::Expire(expiry) => self.on_expire(expiry),
            Task::AiMove {
                seat,
                version,
                cursor,
                mode,
                body,
            } => self.on_ai_move(seat, version, cursor, mode, body).await,
            Task::Idle(done) => {
                let _ = done.send(());
            }
        }
    }

    // -----------------------------------------------------------------
    // Sending. A socket only ever carries its own seat's view (B23).
    // -----------------------------------------------------------------

    fn socket_for<'c>(&self, core: &'c Core, seat: DuosSeat) -> Option<&'c Socket> {
        let index = human(seat)?;
        core.sockets[index].as_ref().filter(|socket| socket.is_open())
    }

    fn send(&self, core: &Core, seat: DuosSeat, message: &DuosServerMessage) {
        if let Some(socket) = self.socket_for(core, seat) {
            socket.send(encode(message));
        }
    }

    /// The deadline `seat` is playing against: the mulligans' in round 0, its round's while the round
    /// clock runs, else the prompt clock it holds.
    fn deadline_for(&self, core: &Core, seat: DuosSeat) -> Option<i64> {
        if core.finished || core.state.result.is_some() {
            return None;
        }
        if core.state.round == 0 {
            return core.mulligan.as_ref().map(|countdown| countdown.deadline);
        }
        if let Some((round, countdown)) = &core.round_clock
            && *round == core.state.round
        {
            return Some(countdown.deadline);
        }
        let index = human(seat)?;
        core.prompt_clocks[index]
            .as_ref()
            .map(|(_, countdown)| countdown.deadline)
    }

    /// B35: the seat's view and its own legal actions, off one state. The engine never reads a clock
    /// (R79), so the own duel's `clockMs` is filled in here with the deadline the seat plays against.
    fn push_view(&self, core: &Core, seat: DuosSeat) {
        let Some(socket) = self.socket_for(core, seat) else {
            return;
        };
        let mut view: DuosView = engine::view(self.shared.pinned, &core.state, seat);
        let now = now_ms();
        view.own.clock_ms = self
            .deadline_for(core, seat)
            .map(|at| i32::try_from((at - now).max(0)).unwrap_or(i32::MAX));
        let legal = engine::legal(self.shared.pinned, &core.state, seat);
        socket.send(encode(&DuosServerMessage::View {
            view: Box::new(view),
            legal,
        }));
    }

    fn push_clock(&self, core: &Core, seat: DuosSeat) {
        let grace = |index: usize| core.grace[index].as_ref().map(|countdown| countdown.deadline);
        self.send(
            core,
            seat,
            &DuosServerMessage::Clock {
                now: now_ms(),
                round_deadline: self.deadline_for(core, seat),
                grace_deadlines: GraceDeadlines {
                    a1: grace(0),
                    a2: grace(1),
                },
            },
        );
    }

    /// S6, B37: what `seat` is told about its teammate.
    fn push_teammate(&self, core: &Core, seat: DuosSeat) {
        let mate = seat.teammate();
        let message = DuosServerMessage::Teammate {
            connected: self.socket_for(core, mate).is_some(),
            ai_playing: human(mate).is_some_and(|index| core.ai_playing[index]),
        };
        self.send(core, seat, &message);
    }

    /// Everything a seat that (re)joins needs: its view, its clock and its teammate.
    fn push_seat(&self, core: &Core, seat: DuosSeat) {
        self.push_view(core, seat);
        self.push_clock(core, seat);
        self.push_teammate(core, seat);
    }

    // -----------------------------------------------------------------
    // Clocks
    // -----------------------------------------------------------------

    /// A countdown of `ms` whose expiry comes back on the queue as `expiry(timer id)`.
    fn countdown(&self, ms: i64, expiry: impl FnOnce(u64) -> Expiry + Send + 'static) -> Countdown {
        let actor = Arc::downgrade(&self.shared);
        let timer = after(ms, move |id| {
            if let Some(shared) = actor.upgrade() {
                DuosActor { shared }.enqueue(Task::Expire(expiry(id)));
            }
        });
        Countdown {
            deadline: now_ms() + ms,
            timer,
        }
    }

    /// B37: the person in `seat` is away; the AI takes the seat when the grace runs out.
    fn start_grace(&self, core: &mut Core, seat: DuosSeat) {
        let Some(index) = human(seat) else { return };
        if core.grace[index].is_some() || core.ai_playing[index] || core.finished {
            return;
        }
        core.grace[index] =
            Some(self.countdown(DISCONNECT_GRACE_MS, move |timer| Expiry::Grace { seat, timer }));
    }

    /// Brings the mulligan, round and prompt clocks up to date with the state. Each is armed once for
    /// what it runs over (the mulligans, a round, a prompt) and never armed again over the same one.
    fn sync_clocks(&self, core: &mut Core) {
        let over = core.finished || core.stopped || core.state.result.is_some();

        if !over && core.state.round == 0 {
            if core.mulligan.is_none() && !core.mulligan_expired {
                core.mulligan =
                    Some(self.countdown(DUOS_MULLIGAN_CLOCK_MS, |timer| Expiry::Mulligan { timer }));
            }
        } else {
            core.mulligan = None;
        }

        // B38: a round with a person among its round seats runs the round clock.
        let round = core.state.round;
        let with_people =
            !over && round >= 1 && core.state.round_seats.iter().any(|seat| human(*seat).is_some());
        if with_people {
            let armed = core.round_clock.as_ref().is_some_and(|(held, _)| *held == round);
            if !armed && core.round_expired != Some(round) {
                let countdown =
                    self.countdown(DUOS_ROUND_CLOCK_MS, move |timer| Expiry::Round { round, timer });
                core.round_clock = Some((round, countdown));
            }
        } else {
            core.round_clock = None;
        }

        // A prompt a person holds outside a round of their own (the AI team's round, or after they
        // ended). Inside one, the round clock covers it, and its expiry answers it (B38).
        for seat in HUMAN_SEATS {
            let Some(index) = human(seat) else { continue };
            let held = if over {
                None
            } else {
                held_prompt(&core.state, seat)
            };
            let covered = acting_round_seat(&core.state, seat);
            match held {
                Some(choice) if !covered => {
                    let armed = core.prompt_clocks[index]
                        .as_ref()
                        .is_some_and(|(held, _)| *held == choice);
                    if !armed && core.prompt_expired[index].as_deref() != Some(choice.as_str()) {
                        let countdown =
                            self.countdown(DUOS_PROMPT_CLOCK_MS, move |timer| Expiry::Prompt { seat, timer });
                        core.prompt_clocks[index] = Some((choice, countdown));
                    }
                }
                _ => core.prompt_clocks[index] = None,
            }
        }
    }

    /// An expiry hands the seats it covers to the AI (B37, B38), which plays them through `apply`.
    fn on_expire(&self, expiry: Expiry) {
        {
            let mut core = self.lock();
            if core.stopped || core.finished {
                return;
            }
            match expiry {
                Expiry::Mulligan { timer } => {
                    if !armed_by(core.mulligan.as_ref(), timer) {
                        return;
                    }
                    core.mulligan = None;
                    core.mulligan_expired = true;
                    // §13.2, R268: each person still owing keeps the hand they were dealt.
                    for seat in HUMAN_SEATS {
                        if owes_mulligan(&core.state, seat) {
                            core.forced[slot(seat)] = Some(Forced::KeepHand);
                        }
                    }
                }
                Expiry::Round { round, timer } => {
                    let current = core
                        .round_clock
                        .as_ref()
                        .is_some_and(|(held, countdown)| *held == round && countdown.timer.id() == timer);
                    if !current {
                        return;
                    }
                    core.round_clock = None;
                    core.round_expired = Some(round);
                    if core.state.round == round {
                        let seats: Vec<DuosSeat> = core.state.round_seats.clone();
                        for seat in seats {
                            if acting_round_seat(&core.state, seat) {
                                core.forced[slot(seat)] = Some(Forced::FinishRound(round));
                            }
                        }
                    }
                }
                Expiry::Prompt { seat, timer } => {
                    let Some(index) = human(seat) else { return };
                    let Some((choice, countdown)) = core.prompt_clocks[index].take() else {
                        return;
                    };
                    if countdown.timer.id() != timer {
                        core.prompt_clocks[index] = Some((choice, countdown));
                        return;
                    }
                    core.prompt_expired[index] = Some(choice.clone());
                    if held_prompt(&core.state, seat).as_deref() == Some(choice.as_str()) {
                        core.forced[slot(seat)] = Some(Forced::AnswerPrompt(choice));
                    }
                }
                // `run` hands the ceiling to `on_ceiling`.
                Expiry::Ceiling { .. } => return,
                Expiry::Grace { seat, timer } => {
                    let Some(index) = human(seat) else { return };
                    if !armed_by(core.grace[index].as_ref(), timer) {
                        return;
                    }
                    core.grace[index] = None;
                    if self.socket_for(&core, seat).is_none() {
                        // B37: the AI plays the seat until its person is back; the team never loses
                        // to a disconnect.
                        core.ai_playing[index] = true;
                        tracing::info!(event = "duos.seat.ai_playing", matchId = %self.match_id_str(), seat = %seat);
                        self.push_teammate(&core, seat.teammate());
                    }
                }
            }
            tracing::info!(event = "duos.clock.expired", matchId = %self.match_id_str(), kind = expiry.kind());
            for seat in HUMAN_SEATS {
                self.push_clock(&core, seat);
            }
        }
        self.drive_ai();
    }

    // -----------------------------------------------------------------
    // Arming, sockets coming and going
    // -----------------------------------------------------------------

    async fn on_arm(&self) {
        let (over, past_ceiling) = {
            let mut core = self.lock();
            // As R744 has it for a 1v1 match: a person no socket has reached yet is away from now,
            // so a seat nobody opens is the AI's once the grace runs out.
            for seat in HUMAN_SEATS {
                if self.socket_for(&core, seat).is_none() {
                    self.start_grace(&mut core, seat);
                }
            }
            self.sync_clocks(&mut core);
            let over = core.state.result.is_some();
            let live = !over && !core.finished;
            let left = ceiling_at(&self.shared.row) - now_ms();
            if live && left > 0 {
                core.ceiling = Some(self.countdown(left, |timer| Expiry::Ceiling { timer }));
            }
            (over, live && left <= 0)
        };
        if over {
            self.on_terminal().await;
        } else if past_ceiling {
            // R1047000: folded back past its ceiling, the match ends before anyone plays on.
            self.reach_ceiling().await;
        } else {
            self.drive_ai();
        }
    }

    /// R1047000: the ceiling's countdown ran out (or its retry's did).
    async fn on_ceiling(&self, timer: u64) {
        {
            let mut core = self.lock();
            if !armed_by(core.ceiling.as_ref(), timer) {
                return;
            }
            core.ceiling = None;
        }
        self.reach_ceiling().await;
    }

    /// R1047000: one `ceilingReached` in duel 0, stamped a1 (R146: the ceiling is no seat's), ends the
    /// whole game as the duel's draw does (`ceiling`), and `after_change` writes the result. A write
    /// that could not be logged is tried again `MATCH_SERVER_ACTION_RETRY_MS` later (R776000).
    async fn reach_ceiling(&self) {
        let seq = {
            let core = self.lock();
            if core.stopped || core.finished || core.state.result.is_some() {
                return;
            }
            core.next_seq
        };
        tracing::info!(event = "duos.clock.expired", matchId = %self.match_id_str(), kind = "ceiling");
        let reply = self
            .apply(
                DuosSeat::A1,
                format!("{SERVER_NONCE_PREFIX}ceiling-{seq}"),
                DuosActionBody::Duel {
                    body: ActionBody::CeilingReached,
                },
            )
            .await;
        if matches!(
            reply,
            DuosServerMessage::Error {
                code: SocketErrorCode::Internal,
                ..
            }
        ) {
            let mut core = self.lock();
            if !core.stopped && !core.finished {
                tracing::warn!(event = "duos.ceiling.retrying", matchId = %self.match_id_str());
                core.ceiling =
                    Some(self.countdown(MATCH_SERVER_ACTION_RETRY_MS, |timer| Expiry::Ceiling { timer }));
            }
        }
    }

    fn on_attach(&self, seat: DuosSeat) {
        {
            let mut core = self.lock();
            let Some(index) = human(seat) else { return };
            core.grace[index] = None;
            if std::mem::replace(&mut core.ai_playing[index], false) {
                tracing::info!(event = "duos.seat.back", matchId = %self.match_id_str(), seat = %seat);
            }
            // B35: the seat's view and legal actions, its clock, and its teammate.
            self.push_seat(&core, seat);
            let mate = seat.teammate();
            self.push_teammate(&core, mate);
            self.push_clock(&core, mate);
        }
        self.drive_ai();
    }

    fn on_disconnect(&self, seat: DuosSeat) {
        let mut core = self.lock();
        if core.stopped || self.socket_for(&core, seat).is_some() {
            return;
        }
        self.start_grace(&mut core, seat);
        let mate = seat.teammate();
        self.push_teammate(&core, mate);
        self.push_clock(&core, mate);
    }

    fn on_socket_gone(&self, seat: DuosSeat) {
        {
            let mut core = self.lock();
            let Some(index) = human(seat) else { return };
            // A socket already replaced by an open one, or let go by `halt`, is not a disconnect.
            if core.sockets[index].as_ref().is_none_or(Socket::is_open) {
                return;
            }
            core.sockets[index] = None;
            if core.stopped {
                return;
            }
            tracing::info!(event = "duos.socket.closed", matchId = %self.match_id_str(), seat = %seat);
        }
        self.enqueue(Task::Disconnect { seat });
    }

    // -----------------------------------------------------------------
    // Frames from a socket
    // -----------------------------------------------------------------

    fn on_frame(&self, seat: DuosSeat, text: &str) {
        let Some(index) = human(seat) else { return };
        let message = parse_duos_message(text);
        let mut core = self.lock();
        match message {
            Err(malformed) => {
                tracing::warn!(event = "duos.frame.malformed", matchId = %self.match_id_str(), seat = %seat, reason = %malformed.reason);
                let now = now_ms();
                let recent = &mut core.malformed_at[index];
                while recent
                    .front()
                    .is_some_and(|at| *at <= now - REJECTION_ALERT_WINDOW_MS)
                {
                    recent.pop_front();
                }
                recent.push_back(now);
                let over = recent.len() > MALFORMED_FRAMES_PER_WINDOW;
                self.send(
                    &core,
                    seat,
                    &error_message(SocketErrorCode::Malformed, &malformed.reason, None),
                );
                if over {
                    // R835002. The guard goes first: the close calls back into the actor.
                    let socket = core.sockets[index].clone();
                    drop(core);
                    if let Some(socket) = socket {
                        socket.close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
                    }
                }
            }
            Ok(DuosClientMessage::Hello { .. }) => {
                // R835001: a hello while this socket's last one is still queued is absorbed by it.
                let generation = core.socket_generation[index];
                if core.hello_pending[index] != Some(generation) {
                    core.hello_pending[index] = Some(generation);
                    self.enqueue(Task::Hello { seat, generation });
                }
            }
            Ok(DuosClientMessage::Ping { ping, target }) => self.relay_ping(&mut core, seat, ping, target),
            Ok(DuosClientMessage::Action { nonce, body }) => {
                let now = now_ms();
                let recent = &mut core.recent_actions[index];
                while recent.front().is_some_and(|at| *at <= now - FLOOD_WINDOW_MS) {
                    recent.pop_front();
                }
                if recent.len() >= MATCH_ACTIONS_PER_SECOND {
                    // §9.8: the overflow is refused; the socket stays open.
                    tracing::warn!(event = "duos.action.flooded", matchId = %self.match_id_str(), seat = %seat);
                    self.send(
                        &core,
                        seat,
                        &error_message(
                            SocketErrorCode::RateLimited,
                            "too many actions; slow down",
                            Some(&nonce),
                        ),
                    );
                    return;
                }
                recent.push_back(now);
                self.enqueue(Task::Action { seat, nonce, body });
            }
        }
    }

    /// B39: a ping goes to the teammate's socket only, at most `PING_RATE_LIMIT` per
    /// `PING_RATE_WINDOW_MS`. A hand target naming another side than the sender's own team's, or a
    /// position past that hand, is dropped, as is the overflow: silently, as an emote is (R643).
    fn relay_ping(&self, core: &mut Core, seat: DuosSeat, ping: PingKind, target: PingTarget) {
        let Some(index) = human(seat) else { return };
        if core.stopped {
            return;
        }
        if target.at == PingAt::Hand {
            let size = core
                .state
                .duels
                .get(usize::from(target.duel))
                .map(|duel| duel.players[target.side].hand.len());
            let shown =
                target.side == seat.player() && target.index.zip(size).is_some_and(|(at, size)| at < size);
            if !shown {
                return;
            }
        } else if core.state.duels.get(usize::from(target.duel)).is_none() {
            return;
        }
        let now = now_ms();
        let window = &mut core.pings[index];
        while window.front().is_some_and(|at| *at <= now - PING_RATE_WINDOW_MS) {
            window.pop_front();
        }
        if window.len() >= PING_RATE_LIMIT {
            return;
        }
        window.push_back(now);
        self.send(
            core,
            seat.teammate(),
            &DuosServerMessage::Ping {
                from: seat,
                ping,
                target,
            },
        );
    }

    // -----------------------------------------------------------------
    // Applying an action
    // -----------------------------------------------------------------

    /// The one path every action takes, a person's and the AI's alike (B35, B40): reduce, append
    /// exactly one row at the next gapless seq, then push. Answers the ack or the error.
    async fn apply(&self, seat: DuosSeat, nonce: String, body: DuosActionBody) -> DuosServerMessage {
        let (action, result, seq) = {
            let core = self.lock();
            if let Some((owner, seq)) = core.acks.get(&nonce) {
                // §9.3: a reused nonce answers its original ack, with no second reduce or row.
                return if *owner == seat {
                    ack_message(&nonce, *seq)
                } else {
                    error_message(
                        SocketErrorCode::Malformed,
                        "that nonce is already taken",
                        Some(&nonce),
                    )
                };
            }
            if core.finished || core.state.result.is_some() {
                return error_message(
                    SocketErrorCode::MatchOver,
                    "this match already has a result",
                    Some(&nonce),
                );
            }
            // §9.1: the seat is the server's, never the client's.
            let action = DuosAction {
                seat,
                nonce: nonce.clone(),
                body,
            };
            let result = engine::reduce(self.shared.pinned, &core.state, &action);
            (action, result, core.next_seq)
        };

        if let Some(reason) = &result.error {
            let reason = reason.to_string();
            tracing::warn!(event = "duos.action.rejected", matchId = %self.match_id_str(), seat = %seat, reason = %reason);
            return error_message(SocketErrorCode::IllegalAction, &reason, Some(&nonce));
        }

        // §9.3: the row goes in before the new state is kept, so the log is never behind the state.
        let logged: Result<Value, StoreError> =
            action_json(action.seat, &action.nonce, &action.body).map_err(StoreError::from);
        let appended = match logged {
            Ok(value) => {
                let row = DuosActionRow {
                    match_id: self.shared.row.id.clone(),
                    seq,
                    nonce: nonce.clone(),
                    action: value,
                    at: now_ms(),
                };
                one_tx!(self.shared.app.db, |t| t.duos_actions_append(&row).await?)
            }
            Err(error) => Err(error),
        };
        if let Err(error) = appended {
            tracing::error!(event = "duos.append.failed", matchId = %self.match_id_str(), seq = seq, message = %error);
            if matches!(error, StoreError::SeqClash(_)) {
                self.let_go("duos.log.diverged");
            }
            return error_message(
                SocketErrorCode::Internal,
                "the action could not be recorded; try again",
                Some(&nonce),
            );
        }

        {
            let mut core = self.lock();
            core.state = result.state;
            core.next_seq = seq + 1;
            core.acks.insert(nonce.clone(), (seat, seq));
        }
        self.after_change().await;
        ack_message(&nonce, seq)
    }

    /// B35: after every applied action, both people get fresh views and clocks; then the result is
    /// written, or the AI moves on.
    async fn after_change(&self) {
        let over = {
            let mut core = self.lock();
            self.sync_clocks(&mut core);
            for seat in HUMAN_SEATS {
                self.push_view(&core, seat);
                self.push_clock(&core, seat);
            }
            core.state.result.is_some()
        };
        if over {
            self.on_terminal().await;
        } else {
            self.drive_ai();
        }
    }

    /// B40: the end writes `result` and `ended_at` once. A write that never lands lets the actor go,
    /// so the next socket rebuilds it from the log and the rebuilt actor writes it as it arms.
    async fn on_terminal(&self) {
        let (result, at) = {
            let mut core = self.lock();
            if core.finished {
                return;
            }
            let Some(result) = core.state.result.as_ref() else {
                return;
            };
            let result = match serde_json::to_value(result) {
                Ok(result) => result,
                Err(error) => {
                    tracing::error!(event = "duos.result.unencodable", matchId = %self.match_id_str(), message = %error);
                    return;
                }
            };
            let at = now_ms();
            core.finished = true;
            core.finished_at = Some(at);
            core.mulligan = None;
            core.round_clock = None;
            core.prompt_clocks = [None, None];
            core.ceiling = None;
            core.grace = [None, None];
            core.forced = [None, None, None, None];
            (result, at)
        };
        if !self.record_result(&result, at).await {
            self.let_go("duos.result.unrecorded");
            return;
        }
        tracing::info!(event = "duos.over", matchId = %self.match_id_str(), result = %result);
    }

    /// `MATCH_RECORD_RESULT_ATTEMPTS` tries, the waits doubling from `MATCH_RECORD_RESULT_BACKOFF_MS`.
    /// A write that finds the result already there has landed too.
    async fn record_result(&self, result: &Value, at: i64) -> bool {
        let mut wait_ms = MATCH_RECORD_RESULT_BACKOFF_MS;
        for attempt in 1..=MATCH_RECORD_RESULT_ATTEMPTS {
            let written = one_tx!(self.shared.app.db, |t| t
                .duos_matches_finish(&self.shared.row.id, result, at)
                .await?);
            match written {
                Ok(_) => return true,
                Err(error) => {
                    tracing::error!(event = "duos.result.failed", matchId = %self.match_id_str(), attempt = attempt, message = %error);
                }
            }
            if attempt < MATCH_RECORD_RESULT_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(u64::try_from(wait_ms).unwrap_or(0))).await;
                wait_ms *= 2;
            }
        }
        false
    }

    /// The registry forgets this actor and both sockets close as `halt` closes them; the next socket
    /// folds the match again from its log (as R1437 and R776000 have the 1v1 actor let go).
    fn let_go(&self, event: &'static str) {
        if self.lock().stopped {
            return;
        }
        registry(&self.shared.app).forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = event, matchId = %self.match_id_str());
    }

    // -----------------------------------------------------------------
    // The AI's seats (B36–B38)
    // -----------------------------------------------------------------

    fn ai_mode(&self, core: &Core, seat: DuosSeat) -> Option<Mode> {
        let Some(index) = human(seat) else {
            return Some(Mode::Full);
        };
        if core.ai_playing[index] {
            return Some(Mode::Full);
        }
        match &core.forced[slot(seat)] {
            Some(Forced::KeepHand) if owes_mulligan(&core.state, seat) => Some(Mode::KeepHand),
            Some(Forced::FinishRound(round)) if *round == core.state.round => Some(Mode::FinishRound),
            Some(Forced::AnswerPrompt(choice))
                if held_prompt(&core.state, seat).as_deref() == Some(choice.as_str()) =>
            {
                Some(Mode::AnswerPrompt(choice.clone()))
            }
            _ => None,
        }
    }

    /// B36: starts a decision for every AI-played seat that owes the game something and is not already
    /// deciding. Each waits `DUOS_AI_STEP_DELAY_MS`, reads the state as it stands then, decides on the
    /// blocking pool and queues its move.
    fn drive_ai(&self) {
        let mut core = self.lock();
        if core.finished || core.stopped || core.state.result.is_some() {
            return;
        }
        for seat in DUOS_SEATS {
            let index = slot(seat);
            if core.ai_busy[index] {
                continue;
            }
            let Some(mode) = self.ai_mode(&core, seat) else {
                continue;
            };
            if !owes(self.shared.pinned, &core.state, seat, &mode) {
                continue;
            }
            core.ai_busy[index] = true;
            let weak = Arc::downgrade(&self.shared);
            tokio::spawn(async move {
                let step = Duration::from_millis(u64::try_from(DUOS_AI_STEP_DELAY_MS).unwrap_or(0));
                tokio::time::sleep(step).await;
                let Some((job, version)) = Self::job_for(&weak, seat) else {
                    return;
                };
                let cursor = job.cursor;
                let mode = job.mode.clone();
                let decided = tokio::task::spawn_blocking(move || {
                    std::panic::catch_unwind(AssertUnwindSafe(|| job.decide()))
                })
                .await;
                let (body, cursor) = match decided {
                    Ok(Ok(decided)) => decided,
                    Ok(Err(payload)) => {
                        tracing::error!(event = "duos.ai.threw", seat = %seat, message = %panic_text(payload));
                        (None, cursor)
                    }
                    Err(error) => {
                        tracing::error!(event = "duos.ai.threw", seat = %seat, message = %join_error_text(error));
                        (None, cursor)
                    }
                };
                if let Some(shared) = weak.upgrade() {
                    DuosActor { shared }.enqueue(Task::AiMove {
                        seat,
                        version,
                        cursor,
                        mode,
                        body,
                    });
                }
            });
        }
    }

    /// The decision `seat` owes, on the state as it stands now, and the log's next seq.
    fn job_for(weak: &Weak<Shared>, seat: DuosSeat) -> Option<(AiJob, i64)> {
        let shared = weak.upgrade()?;
        let actor = DuosActor { shared };
        let core = actor.lock();
        let job = AiJob {
            state: core.state.clone(),
            seat,
            mode: actor.ai_mode(&core, seat),
            seed: actor.shared.row.seed.clone(),
            cursor: core.ai_cursor[slot(seat)],
            pinned: actor.shared.pinned,
        };
        Some((job, core.next_seq))
    }

    /// Plays the AI's move for `seat` while the AI still holds the seat the way it decided the move
    /// (`decided`), as a server action. A move decided in another mode than the seat's now is dropped
    /// and the seat decides again: a person back from past their grace is never played for, even when
    /// a clock has handed the AI their prompts or their turn's end meanwhile (B37, B38). A move
    /// decided on a state the log has since moved past (the other duel, a pass, a person's answer)
    /// still goes when the engine takes it; refused, the seat decides again on the state as it is, as
    /// a ladder bot does (R1452). A fresh move the engine refuses is traded for the fallback once; a
    /// move that could not be written is decided again `BOT_WRITE_RETRY_MS` later.
    async fn on_ai_move(
        &self,
        seat: DuosSeat,
        version: i64,
        cursor: u32,
        decided: Option<Mode>,
        body: Option<DuosActionBody>,
    ) {
        let (held, current) = {
            let mut core = self.lock();
            let index = slot(seat);
            core.ai_busy[index] = false;
            core.ai_cursor[index] = cursor;
            if core.finished || core.stopped {
                return;
            }
            (self.ai_mode(&core, seat), core.next_seq)
        };
        let Some(mode) = held.filter(|mode| decided.as_ref() == Some(mode)) else {
            self.drive_ai();
            return;
        };
        let Some(body) = body else {
            // Nothing to play on the state it read; a newer state may owe something.
            if current != version {
                self.drive_ai();
            }
            return;
        };
        let nonce = format!("{SERVER_NONCE_PREFIX}ai-{current}");
        let mut reply = self.apply(seat, nonce.clone(), body).await;
        let refused = matches!(
            reply,
            DuosServerMessage::Error {
                code: SocketErrorCode::IllegalAction,
                ..
            }
        );
        if refused && current != version {
            self.drive_ai();
            return;
        }
        if refused {
            let retry = {
                let core = self.lock();
                if owes(self.shared.pinned, &core.state, seat, &mode) {
                    let legal = engine::legal(self.shared.pinned, &core.state, seat);
                    without_search(&mode, &core.state, seat, &legal)
                } else {
                    None
                }
            };
            if let Some(retry) = retry {
                reply = self.apply(seat, format!("{nonce}-r"), retry).await;
            }
        }
        let DuosServerMessage::Error { code, message, .. } = &reply else {
            return;
        };
        tracing::warn!(event = "duos.ai.refused", matchId = %self.match_id_str(), seat = %seat, code = ?code, message = %message);
        match code {
            SocketErrorCode::MatchOver => {}
            SocketErrorCode::Internal => {
                // A match whose row is gone (an account deleted with it) takes no more rows: the
                // actor lets go rather than retry for ever.
                let row = one_tx!(self.shared.app.db, |t| t
                    .duos_matches_get(&self.shared.row.id)
                    .await?);
                if matches!(row, Ok(None)) {
                    self.let_go("duos.match.gone");
                } else {
                    self.drive_ai_after(BOT_WRITE_RETRY_MS);
                }
            }
            _ => self.drive_ai_after(DUOS_AI_STEP_DELAY_MS),
        }
    }

    fn drive_ai_after(&self, ms: i64) {
        let actor = Arc::downgrade(&self.shared);
        let wait = Duration::from_millis(u64::try_from(ms).unwrap_or(0));
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            if let Some(shared) = actor.upgrade() {
                DuosActor { shared }.drive_ai();
            }
        });
    }

    // -----------------------------------------------------------------
    // The actor's surface
    // -----------------------------------------------------------------

    pub fn match_id(&self) -> &str {
        &self.shared.row.id
    }

    /// The match's row as the actor was built with it.
    pub fn row(&self) -> &DuosMatch {
        &self.shared.row
    }

    /// The seat `profile_id` holds: a1 for the room's host, a2 for its guest, none otherwise.
    pub fn seat_of(&self, profile_id: &str) -> Option<DuosSeat> {
        if self.shared.row.a1 == profile_id {
            Some(DuosSeat::A1)
        } else if self.shared.row.a2 == profile_id {
            Some(DuosSeat::A2)
        } else {
            None
        }
    }

    /// Hands a connection to the person in `seat` (a1 or a2), replacing (and closing) the one they
    /// held. The seat's view, legal actions, clock and teammate follow on the queue (B35).
    pub fn attach(&self, seat: DuosSeat, socket: Socket) {
        let Some(index) = human(seat) else {
            socket.close(Some(CLOSE_NORMAL), Some("that seat is the AI's"));
            return;
        };
        let previous = {
            let mut core = self.lock();
            if core.stopped {
                None
            } else {
                // R835001: a new socket generation, so its hello is never absorbed by an older one's.
                core.socket_generation[index] += 1;
                Some(core.sockets[index].replace(socket.clone()))
            }
        };
        let Some(previous) = previous else {
            // The actor is going away: the client reconnects, and the next socket rebuilds it.
            socket.close(Some(CLOSE_GOING_AWAY), Some("the actor is going away"));
            return;
        };
        let on_message = Arc::downgrade(&self.shared);
        let on_close = on_message.clone();
        socket.attach(SocketHandlers {
            message: Box::new(move |text: String| {
                if let Some(shared) = on_message.upgrade() {
                    DuosActor { shared }.on_frame(seat, &text);
                }
            }),
            close: Box::new(move || {
                if let Some(shared) = on_close.upgrade() {
                    DuosActor { shared }.on_socket_gone(seat);
                }
            }),
        });
        if let Some(previous) = previous
            && previous != socket
        {
            previous.close(Some(CLOSE_NORMAL), Some("replaced by a new socket"));
        }
        tracing::info!(event = "duos.socket.attached", matchId = %self.match_id_str(), seat = %seat);
        self.enqueue(Task::Attach { seat });
    }

    /// Lets go of that person's socket, which starts their grace.
    pub fn detach(&self, seat: DuosSeat) {
        let Some(index) = human(seat) else { return };
        let socket = self.lock().sockets[index].clone();
        let Some(socket) = socket else { return };
        if socket.is_open() {
            socket.close(Some(CLOSE_NORMAL), Some("detached"));
        }
        self.on_socket_gone(seat);
    }

    /// Applies one action for `seat` through the path a socket's takes, and answers its ack or error.
    pub async fn submit(&self, seat: DuosSeat, nonce: String, body: DuosActionBody) -> DuosServerMessage {
        let (reply, answer) = oneshot::channel();
        let failed = error_message(
            SocketErrorCode::Internal,
            "the match is not running",
            Some(&nonce),
        );
        self.enqueue(Task::Submit {
            seat,
            nonce,
            body,
            reply,
        });
        answer.await.unwrap_or(failed)
    }

    /// Resolves once every task queued before this call has run.
    pub async fn idle(&self) {
        let (done, drained) = oneshot::channel();
        if self.shared.inbox.send(Task::Idle(done)).is_err() {
            return;
        }
        let _ = drained.await;
    }

    /// `seat`'s view as its socket would get it (without the clock).
    pub fn view_for(&self, seat: DuosSeat) -> DuosView {
        engine::view(self.shared.pinned, &self.lock().state, seat)
    }

    pub fn legal_for(&self, seat: DuosSeat) -> Vec<DuosActionBody> {
        engine::legal(self.shared.pinned, &self.lock().state, seat)
    }

    /// `duos_hash` of the live state, for a rebuild's check; never sent to a socket.
    pub fn hash(&self) -> String {
        engine::hash(&self.lock().state)
    }

    /// The next seq the log will take.
    pub fn next_seq(&self) -> i64 {
        self.lock().next_seq
    }

    /// B37: whether the AI plays this person's seat now.
    pub fn ai_playing(&self, seat: DuosSeat) -> bool {
        human(seat).is_some_and(|index| self.lock().ai_playing[index])
    }

    /// Whether the result is written.
    pub fn is_finished(&self) -> bool {
        self.lock().finished
    }

    /// When the result was written (or the finished actor was rebuilt).
    pub fn finished_at(&self) -> Option<i64> {
        self.lock().finished_at
    }

    /// Whether either person holds an open socket.
    pub fn has_open_socket(&self) -> bool {
        let core = self.lock();
        HUMAN_SEATS
            .into_iter()
            .any(|seat| self.socket_for(&core, seat).is_some())
    }

    /// Drops the actor: its clocks stop, its sockets go, its log stays.
    pub async fn stop(&self) {
        self.halt();
        self.idle().await;
    }

    /// `stop` without waiting for the queue, so a task on it can call it.
    fn halt(&self) {
        let sockets = {
            let mut core = self.lock();
            core.stopped = true;
            core.mulligan = None;
            core.round_clock = None;
            core.prompt_clocks = [None, None];
            core.ceiling = None;
            core.grace = [None, None];
            [core.sockets[0].take(), core.sockets[1].take()]
        };
        for socket in sockets.into_iter().flatten() {
            if socket.is_open() {
                socket.close(Some(CLOSE_GOING_AWAY), Some("the actor is going away"));
            }
        }
    }
}
