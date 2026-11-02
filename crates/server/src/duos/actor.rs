//! One actor per Duos co-op match (fullsend SPEC S6, B35–B40): it holds `DuosState`, one socket per
//! person (a1, a2), the clocks and the AI's seats, and appends every action to `duos_match_actions`.
//! `duos_reduce`'s refusals are relayed verbatim (B35); a socket gets only its seat's view (B23).
//! Every action takes `apply` (B40), under the dealt catalog (R767000). The AI plays b1, b2 and a
//! person's seat past its grace (B37) or clock (B38), off the runtime (R1452), with R270's nonces,
//! never conceding or drawing (R188). Clocks live here, never in the engine, so an expiry hands the
//! seat to the AI and `(seed, log)` still folds; a lapsed mulligan keeps the hand (§13.2, R268). The
//! ceiling (§9.5, R1047000) is the engine's own action. Seats keep the 1v1's guards (§9.1, R835001,
//! R137, R835000, R835002); a hello sooner than `WATCH_HELLO_INTERVAL_MS` after the socket's last
//! answered one is dropped (R2686000). A panicking server action ends in the ceiling's draw
//! (R1253000); pings reach the teammate only, never logged (B39). One serialized queue; the lock is
//! never held across an `.await` or while a socket is closed (a close calls back into the actor).

use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use jackioh_ai::{AiOptions, duos_decide};
use jackioh_engine::duos::DuosState;
use jackioh_engine::wire::{
    ActionBody, DUOS_SEATS, DuosAction, DuosActionBody, DuosResult, DuosSeat, DuosView,
};
use jackioh_engine::{Rng, mulligan_owed};

use crate::actor::clock::{Timer, after};
use crate::actor::contracts::{SocketHandlers, one_tx};
use crate::actor::guard::{CLOSE_POLICY_VIOLATED, SeatGuard, alert_unusual_rejections};
use crate::actor::protocol::{ACTION_PANICKED_MESSAGE, SERVER_NONCE_PREFIX, SocketErrorCode};
use crate::actor::registry::panic_text;
use crate::actor::ws_server::{CLOSE_GOING_AWAY, CLOSE_NORMAL, Socket};
use crate::app::{App, now_ms};
use crate::bots::seat::r188_forbidden;
use crate::bug_reports::{Finding, FoundIn, RefusedRow};
use crate::config::{
    BOT_DECISION_DEADLINE_MS, BOT_WRITE_RETRY_MS, DISCONNECT_GRACE_MS, DUOS_AI_BUDGET, DUOS_AI_STEP_DELAY_MS,
    DUOS_MULLIGAN_CLOCK_MS, DUOS_PROMPT_CLOCK_MS, DUOS_ROUND_CLOCK_MS, MATCH_RECORD_RESULT_ATTEMPTS,
    MATCH_RECORD_RESULT_BACKOFF_MS, MATCH_SERVER_ACTION_RETRY_MS, PING_RATE_LIMIT, PING_RATE_WINDOW_MS,
};
use crate::db::store::StoreError;
use crate::duos::engine::{self, Pinned};
use crate::duos::protocol::{
    DuosClientMessage, DuosServerMessage, GraceDeadlines, PingAt, PingKind, PingTarget, ack_message, encode,
    error_message, parse_duos_message,
};
use crate::duos::registry::registry;
use crate::duos::store::{DuosActionRow, DuosMatch, action_json, seat_of_logged};
use crate::duos::{ceiling_at, ceiling_draw, finish_match};
use crate::economy::rewards::GameEconomy;
use crate::seats::Seat;

/// Human player seats, host and guest (S6): the per-person arrays' order.
pub const HUMAN_SEATS: [DuosSeat; 2] = [DuosSeat::A1, DuosSeat::A2];

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

/// R188: forbidden AI moves: concessions, draws, and emotes.
fn ai_forbidden(body: &DuosActionBody) -> bool {
    match body {
        DuosActionBody::Concede => true,
        DuosActionBody::Duel { body } => r188_forbidden(body),
        DuosActionBody::Pass { .. } => false,
    }
}

/// Fallback move when `duos_decide` yields no playable action.
fn fallback(legal: &[DuosActionBody]) -> Option<DuosActionBody> {
    let allowed = || legal.iter().filter(|body| !ai_forbidden(body));
    allowed()
        .find(|body| is_end_turn(body))
        .or_else(|| allowed().find(|body| is_answer(body)))
        .or_else(|| allowed().next())
        .cloned()
}

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

/// Fallback action for `seat` in `mode` without AI search.
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
    /// B26–B29: `duos_decide` on `{seed}:duos-ai:{seat}` (never the match rng) within
    /// `BOT_DECISION_DEADLINE_MS`.
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
        // R2575000: the practice AI's budget before #2575 doubled it, cut short by the deadline.
        let mut options = AiOptions {
            rng: Rng::new(&format!("{}:duos-ai:{}", self.seed, self.seat), self.cursor),
            budget: DUOS_AI_BUDGET,
            should_stop: Some(&should_stop),
        };
        let body = self
            .pinned
            .run(|| duos_decide(&self.state, self.seat, &mut options))
            .filter(|body| !ai_forbidden(body));
        (body, options.rng.cursor())
    }
}

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
    /// AI decision for `seat` in `mode` at state `version`.
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

/// R1253000: the answer a panicked action gets, first and on every resend of its nonce.
fn panicked_answer(nonce: &str) -> DuosServerMessage {
    error_message(SocketErrorCode::Internal, ACTION_PANICKED_MESSAGE, Some(nonce))
}

fn armed_by(countdown: Option<&Countdown>, timer: u64) -> bool {
    countdown.is_some_and(|countdown| countdown.timer.id() == timer)
}

struct Core {
    state: DuosState,
    next_seq: i64,
    /// §9.3: nonce -> (seat, seq) rebuilt from log.
    acks: IndexMap<String, (DuosSeat, i64)>,
    /// R1253000: nonces whose `reduce` panicked; not `acks`, which holds only what the log holds.
    panicked: IndexSet<String>,
    sockets: [Option<Socket>; 2],
    /// B37: whether AI plays human seat during disconnect.
    ai_playing: [bool; 2],
    grace: [Option<Countdown>; 2],
    mulligan: Option<Countdown>,
    mulligan_expired: bool,
    round_clock: Option<(i32, Countdown)>,
    round_expired: Option<i32>,
    prompt_clocks: [Option<(String, Countdown)>; 2],
    prompt_expired: [Option<String>; 2],
    /// R1047000: hard ceiling countdown.
    ceiling: Option<Countdown>,
    forced: [Option<Forced>; 4],
    ai_busy: [bool; 4],
    /// Each seat's AI stream cursor; a rebuilt actor starts them again.
    ai_cursor: [u32; 4],
    /// By `HUMAN_SEATS`: action flood (R137, §9.8), refusals (R835000), malformed frames (R835002), hello
    /// interval (R2686000).
    guards: [SeatGuard; 2],
    /// By `HUMAN_SEATS`: B39's ping window.
    pings: [VecDeque<i64>; 2],
    /// By `HUMAN_SEATS`: pending Hello socket generation to absorb duplicates (R835001).
    hello_pending: [Option<u64>; 2],
    socket_generation: [u64; 2],
    finished: bool,
    finished_at: Option<i64>,
    /// By `HUMAN_SEATS`: whether daily Jlore cap held back pay (R1483000).
    jlore_capped: [bool; 2],
    /// By `HUMAN_SEATS`: payout per player answered by result write (#1657); `None`, nothing told.
    economy: [Option<GameEconomy>; 2],
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
        panicked: IndexSet::new(),
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
        guards: [SeatGuard::default(), SeatGuard::default()],
        pings: [VecDeque::new(), VecDeque::new()],
        hello_pending: [None, None],
        socket_generation: [0, 0],
        finished,
        finished_at: finished.then(now_ms),
        jlore_capped: [false, false],
        economy: [None, None],
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
                // §9.5 full view; clear Hello flag only for matching generation, so a newer socket's
                // queued Hello stays marked (R835001).
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

    // Sockets only carry their own seat's view (B23).

    fn socket_for<'c>(&self, core: &'c Core, seat: DuosSeat) -> Option<&'c Socket> {
        let index = human(seat)?;
        core.sockets[index].as_ref().filter(|socket| socket.is_open())
    }

    fn send(&self, core: &Core, seat: DuosSeat, message: &DuosServerMessage) {
        if let Some(socket) = self.socket_for(core, seat) {
            socket.send(encode(message));
        }
    }

    /// Deadline `seat` plays against: mulligan, round, or prompt clock.
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

    /// B35: view and legal actions; fills in `clock_ms` deadline outside engine (R79).
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
            jlore_capped: human(seat).map(|index| core.jlore_capped[index]).unwrap_or(false),
            economy: human(seat).and_then(|index| core.economy[index].clone()),
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

    /// Syncs mulligan, round and prompt clocks with state (armed once per instance).
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

        // B38: round with human seat runs round clock.
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

        // Prompt held outside own round (inside one, round clock covers it, B38).
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
                        // B37: AI plays the seat until its person is back: no loss by disconnect.
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

    async fn on_arm(&self) {
        let (over, past_ceiling) = {
            let mut core = self.lock();
            // R744: seat with no socket starts grace, so one nobody opens goes to the AI.
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
            // R1483000: an actor rebuilt on a match already finished learns what the cap held
            // back from the ledger; one that is about to write it does so in `on_terminal`.
            if self.lock().finished {
                self.load_jlore_caps(false).await;
            }
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

    /// R1047000: `ceilingReached` in duel 0, stamped a1 (R146: no seat's), ends the game as a
    /// draw; retried on a log error (R776000).
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
            // Open replacement or halted socket is not a disconnect.
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

    fn on_frame(&self, seat: DuosSeat, text: &str) {
        let Some(index) = human(seat) else { return };
        let message = parse_duos_message(text);
        let mut core = self.lock();
        match message {
            Err(malformed) => {
                tracing::warn!(event = "duos.frame.malformed", matchId = %self.match_id_str(), seat = %seat, reason = %malformed.reason);
                let over = core.guards[index].note_malformed(now_ms());
                self.send(
                    &core,
                    seat,
                    &error_message(SocketErrorCode::Malformed, &malformed.reason, None),
                );
                if over {
                    // R835002: close past the budget; the lock goes first, as the close calls back.
                    let socket = core.sockets[index].clone();
                    drop(core);
                    if let Some(socket) = socket {
                        socket.close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
                    }
                }
            }
            Ok(DuosClientMessage::Hello { .. }) => {
                // R835001: a hello while this socket's last one is still queued is absorbed by it.
                // R2686000: one within WATCH_HELLO_INTERVAL_MS of the last answered is dropped, the
                // socket left open; a newer generation (a reattach) always enqueues.
                let generation = core.socket_generation[index];
                if core.hello_pending[index] != Some(generation)
                    && core.guards[index].hello_answer_allowed(generation, now_ms())
                {
                    core.hello_pending[index] = Some(generation);
                    self.enqueue(Task::Hello { seat, generation });
                }
            }
            Ok(DuosClientMessage::Ping { ping, target }) => self.relay_ping(&mut core, seat, ping, target),
            Ok(DuosClientMessage::Action { nonce, body }) => {
                let now = now_ms();
                if !core.guards[index].action_allowed(now) {
                    // §9.8: the overflow is refused; the socket stays open.
                    tracing::warn!(event = "duos.action.flooded", matchId = %self.match_id_str(), seat = %seat);
                    // R835000: flood refusal counts toward unusual-rejection alert.
                    self.note_rejection(&mut core, index, now);
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
                self.enqueue(Task::Action { seat, nonce, body });
            }
        }
    }

    /// R835000: records rejection, alerting on threshold crossing.
    fn note_rejection(&self, core: &mut Core, index: usize, now: i64) {
        if let Some(rejected) = core.guards[index].note_rejection(now) {
            let profile_id = [&self.shared.row.a1, &self.shared.row.a2][index];
            alert_unusual_rejections(self.match_id_str(), profile_id, rejected);
        }
    }

    /// B39: relays rate-limited pings to teammate; a hand target off the team's own hand is dropped
    /// silently (R643).
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

    /// Applied action path for player and AI (B35, B40): reduce, append one gapless row, then push.
    async fn apply(&self, seat: DuosSeat, nonce: String, body: DuosActionBody) -> DuosServerMessage {
        let reduced = {
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
            if core.panicked.contains(&nonce) {
                return panicked_answer(&nonce);
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
            // R1253000: pure `reduce` leaves nothing half-applied on panic, as the fold relies on.
            let reduced = std::panic::catch_unwind(AssertUnwindSafe(|| {
                engine::reduce(self.shared.pinned, &core.state, &action)
            }));
            let seq = core.next_seq;
            match reduced {
                Ok(result) => Ok((action, result, seq)),
                Err(payload) => Err((action, seq, panic_text(payload))),
            }
        };
        // After the lock: `on_reduce_panicked` takes it again and awaits.
        let (action, result, seq) = match reduced {
            Ok(applied) => applied,
            Err((action, seq, message)) => return self.on_reduce_panicked(&action, seq, &message).await,
        };

        if let Some(reason) = &result.error {
            let reason = reason.to_string();
            tracing::warn!(event = "duos.action.rejected", matchId = %self.match_id_str(), seat = %seat, reason = %reason);
            // R835000: count person refusals; AI/ceiling carry server nonces (R270) and never do.
            if let Some(index) = human(seat)
                && !nonce.starts_with(SERVER_NONCE_PREFIX)
            {
                let mut core = self.lock();
                self.note_rejection(&mut core, index, now_ms());
            }
            return error_message(SocketErrorCode::IllegalAction, &reason, Some(&nonce));
        }

        // §9.3: row before state, so the log is never behind the state.
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

    /// R1253000: handles panicked `reduce`. Caches nonce failure and files finding with `bug_reports`.
    /// A server action would be made and panic again, so it ends the match as ceiling draw.
    async fn on_reduce_panicked(&self, action: &DuosAction, seq: i64, message: &str) -> DuosServerMessage {
        let kind = match &action.body {
            DuosActionBody::Duel { body } => body.action_type().to_string(),
            DuosActionBody::Pass { .. } => "pass".to_string(),
            DuosActionBody::Concede => "concede".to_string(),
        };
        tracing::error!(event = "duos.action.panicked", matchId = %self.match_id_str(), seq = seq, seat = %action.seat, "type" = %kind, message = %message);
        self.lock().panicked.insert(action.nonce.clone());
        self.shared.app.bug_reports.file(
            self.shared.app.env.deployed_commit.as_deref(),
            Finding {
                match_id: self.match_id_str().to_string(),
                found_in: FoundIn::DuosLive,
                catalog_version: self.shared.row.options.catalog_version.clone(),
                actions: usize::try_from(seq - 1).unwrap_or(0),
                refused: vec![RefusedRow {
                    seq,
                    seat: action.seat.to_string(),
                    action: kind,
                }],
            },
        );
        if action.nonce.starts_with(SERVER_NONCE_PREFIX) {
            self.end_as_ceiling_draw().await;
        }
        panicked_answer(&action.nonce)
    }

    /// R1253000, R1047001: ends match as ceiling draw (B40) and lets go as `end_abandoned` does.
    async fn end_as_ceiling_draw(&self) {
        let result = ceiling_draw();
        let at = now_ms();
        {
            let mut core = self.lock();
            if core.finished || core.stopped {
                return;
            }
            core.finished = true;
            core.finished_at = Some(at);
        }
        if self.record_result(&result, at).await.is_none() {
            self.let_go("duos.result.unrecorded");
            return;
        }
        self.release_seats();
        self.let_go("duos.panicked.drawn");
    }

    /// R1523000: once the result has landed, both people leave the match's seats.
    fn release_seats(&self) {
        let row = &self.shared.row;
        let seat = Seat::Duos(row.id.clone());
        for person in [&row.a1, &row.a2] {
            self.shared.app.seats.release(person, &seat);
        }
    }

    /// B35: pushes fresh views and clocks after every action, then writes the result or moves the AI.
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

    /// B40: writes `result` and `ended_at` once; a lost write lets go, for a rebuild to write it.
    async fn on_terminal(&self) {
        let (result, at) = {
            let mut core = self.lock();
            if core.finished {
                return;
            }
            let Some(result) = core.state.result.clone() else {
                return;
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
        let Some(economy) = self.record_result(&result, at).await else {
            self.let_go("duos.result.unrecorded");
            return;
        };
        self.release_seats();
        self.lock().economy = economy;
        self.load_jlore_caps(true).await;
        tracing::info!(event = "duos.over", matchId = %self.match_id_str(), outcome = %result.outcome, reason = %result.reason);
    }

    /// R1483000: checks if daily Jlore cap held back either seat's pay. Pushes updated view
    /// with `jlore_capped` and payment (#1657) to affected seats; a failed read is only logged.
    async fn load_jlore_caps(&self, announce: bool) {
        let caps = match super::duos_game_jlore_caps(&self.shared.app, self.match_id_str()).await {
            Ok(caps) => caps,
            Err(error) => {
                tracing::warn!(event = "duos.jloreCaps.failed", matchId = %self.match_id_str(), message = %error);
                [false, false]
            }
        };
        let mut core = self.lock();
        if caps.iter().any(|capped| *capped) {
            core.jlore_capped = caps;
        }
        for seat in HUMAN_SEATS {
            if announce
                && human(seat).is_some_and(|index| core.jlore_capped[index] || core.economy[index].is_some())
            {
                self.push_view(&core, seat);
            }
        }
    }

    /// Retries `finish_match` ([[R1502000]]) with backoff; a result already there has landed too.
    /// Returns payout per `HUMAN_SEATS` (#1657).
    async fn record_result(&self, result: &DuosResult, at: i64) -> Option<[Option<GameEconomy>; 2]> {
        let mut wait_ms = MATCH_RECORD_RESULT_BACKOFF_MS;
        for attempt in 1..=MATCH_RECORD_RESULT_ATTEMPTS {
            let written = one_tx!(self.shared.app.db, |t| finish_match(
                &mut t,
                &self.shared.app,
                &self.shared.row,
                result,
                at
            )
            .await?);
            match written {
                Ok(paid) => return Some(paid.unwrap_or_default()),
                Err(error) => {
                    tracing::error!(event = "duos.result.failed", matchId = %self.match_id_str(), attempt = attempt, message = %error);
                }
            }
            if attempt < MATCH_RECORD_RESULT_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(u64::try_from(wait_ms).unwrap_or(0))).await;
                wait_ms *= 2;
            }
        }
        None
    }

    /// Forgets actor and closes sockets, for the next socket to fold the log (R1437, R776000).
    fn let_go(&self, event: &'static str) {
        if self.lock().stopped {
            return;
        }
        registry(&self.shared.app).forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = event, matchId = %self.match_id_str());
    }

    // The AI's seats (B36–B38)

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

    /// B36: starts background decision for every AI seat needing a move (`DUOS_AI_STEP_DELAY_MS`).
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

    /// Plays AI move for `seat` if still in matching mode, else decides again: a person back is never
    /// played for (B37, B38). A refused move decides again (R1452), a fresh one falls back once; a
    /// failed write is decided again after `BOT_WRITE_RETRY_MS`.
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
                // Deleted match row takes no more rows; lets go rather than retrying forever.
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

    /// Attaches connection to person in `seat` (a1/a2), closing the old; view and clock follow (B35).
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
                // R835001: new socket generation so hello is never absorbed by older one.
                core.socket_generation[index] += 1;
                Some(core.sockets[index].replace(socket.clone()))
            }
        };
        let Some(previous) = previous else {
            // Actor stopping; client reconnects.
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

    /// Detaches socket, starting disconnect grace.
    pub fn detach(&self, seat: DuosSeat) {
        let Some(index) = human(seat) else { return };
        let socket = self.lock().sockets[index].clone();
        let Some(socket) = socket else { return };
        if socket.is_open() {
            socket.close(Some(CLOSE_NORMAL), Some("detached"));
        }
        self.on_socket_gone(seat);
    }

    /// Submits action for `seat`, returning ack or error.
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

    /// R2092000: which human seats hold open sockets, for Play again's status (`HUMAN_SEATS`,
    /// CLAUDE.md rule 7).
    pub fn presence(&self) -> [bool; 2] {
        let core = self.lock();
        HUMAN_SEATS.map(|seat| self.socket_for(&core, seat).is_some())
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
