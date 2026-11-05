//! One actor per Duos co-op match (fullsend SPEC S6, B35–B40): it holds one socket per person (a1, a2)
//! and carries out what the match host's machine asks of it, and appends every action to
//! `duos_match_actions`. It decides no clock, grace or AI rule and owns no timer of its own: the decisions
//! are `jackioh_shared::host::duos`'s (#3101's second part) — which clock runs (the mulligan, round and
//! prompt clocks, the ceiling and its retry), what an expiry hands the AI (a lapsed mulligan keeps the
//! hand, §13.2, R268), the grace after which the AI plays a person's seat (B37) and when a person is back,
//! the AI's pause and what it falls back on (B36, B38), the order every view and clock goes out in — and
//! the actor feeds the machine its events (a frame, a socket gone, a timer that fired, a row written, the
//! AI's search) and does what it answers with: a tokio timer, a socket send, a store write, the AI's
//! search on the blocking pool. `duos_reduce`'s refusals are relayed verbatim (B35); a socket gets only its
//! seat's view (B23). Every action takes `apply` (B40), under the dealt catalog (R767000). The AI plays b1,
//! b2 and a person's seat past its grace (B37) or clock (B38), off the runtime (R1452), with R270's
//! nonces, never conceding or drawing (R188). Clocks live in the machine, never in the engine, so an
//! expiry hands the seat to the AI and `(seed, log)` still folds. The ceiling (§9.5, R1047000) is the
//! engine's own action. Seats keep the 1v1's guards (§9.1, R835001, R137, R835000, R835002); a hello sooner
//! than `WATCH_HELLO_INTERVAL_MS` after the socket's last answered one is dropped (R2686000). A panicking
//! server action ends in the ceiling's draw (R1253000); pings reach the teammate only, never logged
//! (B39). A friend's watcher of a person's seat is sent that seat's view `SPECTATE_DELAY_MS` late with no
//! legal action, and nothing else the seat is sent (R2881000, as a 1v1 watch is). One serialized queue; the
//! lock is never held across an `.await` or while a socket is closed (a close calls back into the actor).

use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use indexmap::IndexMap;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use jackioh_ai::{AiOptions, duos_decide};
use jackioh_engine::Rng;
use jackioh_engine::wire::{DuosAction, DuosActionBody, DuosSeat, DuosView};
use jackioh_shared::host::duos::{DuosAck, DuosDecide, DuosLetGo, DuosNote, DuosSeatMessage, ai_forbidden};
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{
    ActionAnswer, DuosHost, DuosHostEvent, DuosHostOutput, DuosSetup, GreetingPart, LogOutcome,
};

use crate::actor::clock::{Timer, after};
use crate::actor::contracts::{SocketHandlers, one_tx};
use crate::actor::guard::{CLOSE_POLICY_VIOLATED, SeatGuard, alert_unusual_rejections};
use crate::actor::protocol::{ACTION_PANICKED_MESSAGE, SocketErrorCode};
use crate::actor::registry::panic_text;
use crate::actor::ws_server::{CLOSE_GOING_AWAY, CLOSE_NORMAL, Socket};
use crate::app::{App, now_ms};
use crate::bug_reports::{Finding, FoundIn, RefusedRow};
use crate::config::{
    BOT_DECISION_DEADLINE_MS, DUOS_AI_BUDGET, MATCH_RECORD_RESULT_ATTEMPTS, MATCH_RECORD_RESULT_BACKOFF_MS,
    PING_RATE_LIMIT, PING_RATE_WINDOW_MS,
};
use crate::db::store::StoreError;
use crate::duos::engine::{self, Pinned};
use crate::duos::protocol::{
    DuosClientMessage, DuosServerMessage, GraceDeadlines, PingAt, PingKind, PingTarget, ack_message, encode,
    error_message, parse_duos_message,
};
use crate::duos::registry::registry;
use crate::duos::store::{DuosActionRow, DuosMatch, action_json, seat_of_logged};
use crate::duos::{ceiling_draw, finish_match};
use crate::economy::rewards::GameEconomy;
use crate::seats::Seat;

/// Human player seats, host and guest (S6): the per-person arrays' order.
pub use jackioh_shared::host::duos::HUMAN_SEATS;

use jackioh_shared::host::duos::human;

/// One decision the AI owes a seat, taken off the actor and run on the blocking pool: the search itself.
/// What to ask, and what a search that finds nothing falls back on, are the machine's.
struct AiJob {
    state: jackioh_engine::duos::DuosState,
    seat: DuosSeat,
    seed: String,
    cursor: u32,
    pinned: Pinned,
}

impl AiJob {
    /// B26–B29: `duos_decide` on `{seed}:duos-ai:{seat}` (never the match rng) within
    /// `BOT_DECISION_DEADLINE_MS`.
    fn ask(&self) -> (Option<DuosActionBody>, u32) {
        jackioh_cards::register_all();
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
    pub state: jackioh_engine::duos::DuosState,
    /// The rows already logged: the seq counter and the nonce map are rebuilt from them.
    pub log: Vec<DuosActionRow>,
    /// R767000: the catalog the match was dealt from, which every engine call runs under.
    pub pinned: Pinned,
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
    /// A timer the machine set came due: a clock ran out (B37, B38), the ceiling (R1047000), or an AI seat's
    /// pause is over (B36).
    Timer {
        key: TimerKey,
        id: u32,
    },
    /// The AI's search for `seat`, as `Decide` asked for it.
    Decided {
        seat: DuosSeat,
        body: Option<DuosActionBody>,
        cursor: u32,
        failed: bool,
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
            Task::Timer { key, .. } => format!("expiry:{}", timer_name(*key)),
            Task::Decided { .. } => "ai-move".to_string(),
            Task::Idle(_) => "idle".to_string(),
        }
    }
}

/// The word a timer is named by in a log label.
fn timer_name(key: TimerKey) -> &'static str {
    match key {
        TimerKey::DuosMulligan => "mulligan",
        TimerKey::DuosRound => "round",
        TimerKey::DuosPrompt { .. } => "prompt",
        TimerKey::DuosGrace { .. } => "grace",
        TimerKey::DuosCeiling => "ceiling",
        TimerKey::DuosAi { .. } => "ai",
        TimerKey::DuosAiRetry => "ai-retry",
        _ => "other",
    }
}

/// R1253000: the answer a panicked action gets, first and on every resend of its nonce.
fn panicked_answer(nonce: &str) -> DuosServerMessage {
    error_message(SocketErrorCode::Internal, ACTION_PANICKED_MESSAGE, Some(nonce))
}

/// The frame the machine's answer to an action is sent as (§9.3).
fn answer_message(nonce: &str, answer: ActionAnswer) -> DuosServerMessage {
    match answer {
        ActionAnswer::Ack { seq } => ack_message(nonce, seq),
        ActionAnswer::Illegal { reason } => {
            error_message(SocketErrorCode::IllegalAction, &reason, Some(nonce))
        }
        ActionAnswer::MatchOver => error_message(
            SocketErrorCode::MatchOver,
            "this match already has a result",
            Some(nonce),
        ),
        ActionAnswer::Panicked => panicked_answer(nonce),
        ActionAnswer::NotRecorded => error_message(
            SocketErrorCode::Internal,
            "the action could not be recorded; try again",
            Some(nonce),
        ),
        ActionAnswer::NonceTaken => error_message(
            SocketErrorCode::Malformed,
            "that nonce is already taken",
            Some(nonce),
        ),
    }
}

struct Core {
    /// The co-op host's machine: the game, the clocks, the log's bookkeeping, the AI's seats.
    host: DuosHost,
    /// The timers the machine asked for, by key, each with the id it was asked under.
    timers: IndexMap<TimerKey, (u32, Timer)>,
    sockets: [Option<Socket>; 2],
    /// By `HUMAN_SEATS`: action flood (R137, §9.8), refusals (R835000), malformed frames (R835002), hello
    /// interval (R2686000).
    guards: [SeatGuard; 2],
    /// By `HUMAN_SEATS`: B39's ping window.
    pings: [VecDeque<i64>; 2],
    /// By `HUMAN_SEATS`: pending Hello socket generation to absorb duplicates (R835001).
    hello_pending: [Option<u64>; 2],
    socket_generation: [u64; 2],
    /// By `HUMAN_SEATS`: whether daily Jlore cap held back pay (R1483000).
    jlore_capped: [bool; 2],
    /// By `HUMAN_SEATS`: payout per player answered by result write (#1657); `None`, nothing told.
    economy: [Option<GameEconomy>; 2],
    /// R2881000: what the actor keeps for the friends watching a person's seat.
    spectate: DuosSpectate,
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
    let acked: Vec<DuosAck> = log
        .iter()
        .filter_map(|entry| {
            seat_of_logged(&entry.action).map(|seat| DuosAck {
                nonce: entry.nonce.clone(),
                seat,
                seq: entry.seq,
            })
        })
        .collect();
    let (host, _) = DuosHost::new(
        DuosSetup {
            match_id: row.id.clone(),
            created_at: row.created_at,
            game: state,
            finished: row.ended_at.is_some(),
            next_seq,
            acked,
        },
        now_ms(),
    );
    let core = Core {
        host,
        timers: IndexMap::new(),
        sockets: [None, None],
        guards: [SeatGuard::default(), SeatGuard::default()],
        pings: [VecDeque::new(), VecDeque::new()],
        hello_pending: [None, None],
        socket_generation: [0, 0],
        jlore_capped: [false, false],
        economy: [None, None],
        spectate: DuosSpectate::default(),
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
                {
                    let mut core = self.lock();
                    if let Some(index) = human(seat)
                        && core.hello_pending[index] == Some(generation)
                    {
                        core.hello_pending[index] = None;
                    }
                }
                self.drive(DuosHostEvent::Hello { seat }).await;
            }
            Task::Attach { seat } => {
                self.drive(DuosHostEvent::Attached { seat }).await;
            }
            Task::Disconnect { seat } => {
                self.drive(DuosHostEvent::Disconnected { seat }).await;
            }
            Task::Action { seat, nonce, body } => {
                let reply = self.answer_action(seat, nonce, body).await;
                let core = self.lock();
                self.send(&core, seat, &reply);
            }
            Task::Submit {
                seat,
                nonce,
                body,
                reply,
            } => {
                let answer = self.answer_action(seat, nonce, body).await;
                let _ = reply.send(answer);
            }
            Task::Timer { key, id } => {
                {
                    // The timer that reported is spent; a newer one under the same key is not.
                    let mut core = self.lock();
                    if core.timers.get(&key).is_some_and(|(held, _)| *held == id) {
                        core.timers.shift_remove(&key);
                    }
                }
                self.drive(DuosHostEvent::TimerFired { key, id }).await;
            }
            Task::Decided {
                seat,
                body,
                cursor,
                failed,
            } => {
                self.drive(DuosHostEvent::Decided {
                    seat,
                    body,
                    cursor,
                    failed,
                })
                .await;
            }
            Task::Idle(done) => {
                let _ = done.send(());
            }
        }
    }

    /// Arms the machine: the clocks are brought up to date, a person with no socket starts its grace
    /// (R744), a match folded back past its ceiling ends before anyone plays on (R1047000), and a game
    /// that is over is ended.
    async fn on_arm(&self) {
        let (over, was_finished) = {
            let core = self.lock();
            (core.host.game().result.is_some(), core.host.is_finished())
        };
        self.drive(DuosHostEvent::Arm).await;
        // R1483000: an actor rebuilt on a match already finished learns what the cap held back from the
        // ledger; one that is about to write it does so in `on_record`.
        if over && was_finished {
            self.load_jlore_caps(false).await;
        }
    }

    // The machine: events in, outputs carried out

    /// Steps the machine under the match's pinned catalog (R767000).
    fn step_in(&self, core: &mut Core, event: DuosHostEvent) -> Vec<DuosHostOutput> {
        let now = now_ms();
        crate::actor::engine::run_pinned(self.shared.pinned, || core.host.step(event, now))
    }

    /// Feeds the machine one event and carries out what it answers with, the awaits included, in order.
    /// The answers it gave to actions, by nonce.
    async fn drive(&self, event: DuosHostEvent) -> Vec<(String, ActionAnswer)> {
        let outputs = {
            let mut core = self.lock();
            self.step_in(&mut core, event)
        };
        let mut answers = Vec::new();
        self.exec(outputs, &mut answers).await;
        answers
    }

    /// One action for a seat, through the machine (B35, B40): the frame its sender is answered with.
    async fn answer_action(&self, seat: DuosSeat, nonce: String, body: DuosActionBody) -> DuosServerMessage {
        let answers = self
            .drive(DuosHostEvent::Action {
                seat,
                nonce: nonce.clone(),
                body,
            })
            .await;
        match answers.into_iter().find(|(answered, _)| *answered == nonce) {
            Some((nonce, answer)) => answer_message(&nonce, answer),
            None => answer_message(&nonce, ActionAnswer::NotRecorded),
        }
    }

    /// Carries out `outputs` in order: what needs no await under the lock as it comes, the rest apart.
    async fn exec(&self, outputs: Vec<DuosHostOutput>, answers: &mut Vec<(String, ActionAnswer)>) {
        for output in outputs {
            let deferred = {
                let mut core = self.lock();
                self.run_sync(&mut core, output, answers)
            };
            if let Some(output) = deferred {
                Box::pin(self.run_async(output, answers)).await;
            }
        }
    }

    /// Carries out `outputs` that need no await (those of a socket's arrival or going), under the lock the
    /// caller holds.
    fn exec_now(&self, core: &mut Core, outputs: Vec<DuosHostOutput>) {
        let mut ignored = Vec::new();
        for output in outputs {
            if let Some(output) = self.run_sync(core, output, &mut ignored) {
                tracing::error!(event = "duos.output.deferred", matchId = %self.match_id_str(), output = ?output);
            }
        }
    }

    /// One output that needs no await; `Some(output)` back when it does.
    fn run_sync(
        &self,
        core: &mut Core,
        output: DuosHostOutput,
        answers: &mut Vec<(String, ActionAnswer)>,
    ) -> Option<DuosHostOutput> {
        match output {
            DuosHostOutput::Send { seat, message } => self.send_seat(core, seat, message),
            DuosHostOutput::Timer { command } => self.apply_timer(core, command),
            DuosHostOutput::Answer { nonce, answer } => answers.push((nonce, answer)),
            DuosHostOutput::Greeting { seat, part } => {
                // R2881000: the watcher count, which a socket that was away missed.
                if part == GreetingPart::Watchers
                    && let Some(index) = human(seat)
                {
                    self.push_watchers_to(core, index);
                }
            }
            DuosHostOutput::Refused { seat } => {
                // R835000: person refusals count toward the unusual-rejection alert.
                if let Some(index) = human(seat) {
                    self.note_rejection(core, index, now_ms());
                }
            }
            DuosHostOutput::Panicked { action, seq, message } => {
                self.on_reduce_panicked(&action, seq, &message)
            }
            DuosHostOutput::Pushed => {
                // R2881000: the watchers' record of the change, sent SPECTATE_DELAY_MS on.
                self.spectate_record_in(core);
            }
            DuosHostOutput::Decide { request } => self.spawn_search(core, request),
            DuosHostOutput::Note { note } => self.log_note(note),
            other => return Some(other),
        }
        None
    }

    /// One output that awaits: a write to the store, or the end of the match.
    async fn run_async(&self, output: DuosHostOutput, answers: &mut Vec<(String, ActionAnswer)>) {
        match output {
            DuosHostOutput::Append { seq, action, at } => {
                // §9.3: row before state, so the log is never behind the state.
                let logged: Result<Value, StoreError> =
                    action_json(action.seat, &action.nonce, &action.body).map_err(StoreError::from);
                let appended = match logged {
                    Ok(value) => {
                        let row = DuosActionRow {
                            match_id: self.shared.row.id.clone(),
                            seq,
                            nonce: action.nonce.clone(),
                            action: value,
                            at,
                        };
                        one_tx!(self.shared.app.db, |t| t.duos_actions_append(&row).await?)
                    }
                    Err(error) => Err(error),
                };
                let outcome = match appended {
                    Ok(()) => LogOutcome::Written,
                    Err(error) => {
                        tracing::error!(event = "duos.append.failed", matchId = %self.match_id_str(), seq = seq, message = %error);
                        if matches!(error, StoreError::SeqClash(_)) {
                            LogOutcome::Diverged
                        } else {
                            LogOutcome::Failed
                        }
                    }
                };
                let outputs = {
                    let mut core = self.lock();
                    self.step_in(&mut core, DuosHostEvent::Logged { outcome })
                };
                self.exec(outputs, answers).await;
            }
            DuosHostOutput::Record { result, at } => self.on_record(&result, at).await,
            DuosHostOutput::CeilingDraw { at } => self.end_as_ceiling_draw(at).await,
            DuosHostOutput::CheckRow => {
                // A deleted match row takes no more rows; the store says whether it is still there.
                let row = one_tx!(self.shared.app.db, |t| t
                    .duos_matches_get(&self.shared.row.id)
                    .await?);
                let exists = !matches!(row, Ok(None));
                let outputs = {
                    let mut core = self.lock();
                    self.step_in(&mut core, DuosHostEvent::RowChecked { exists })
                };
                self.exec(outputs, answers).await;
            }
            DuosHostOutput::LetGo { reason } => match reason {
                DuosLetGo::Diverged => self.let_go("duos.log.diverged"),
                DuosLetGo::MatchGone => self.let_go("duos.match.gone"),
            },
            other => {
                tracing::error!(event = "duos.output.unhandled", matchId = %self.match_id_str(), output = ?other);
            }
        }
    }

    fn log_note(&self, note: DuosNote) {
        match note {
            DuosNote::ClockExpired { kind } => {
                tracing::info!(event = "duos.clock.expired", matchId = %self.match_id_str(), kind = %kind);
            }
            DuosNote::SeatAiPlaying { seat } => {
                tracing::info!(event = "duos.seat.ai_playing", matchId = %self.match_id_str(), seat = %seat);
            }
            DuosNote::SeatBack { seat } => {
                tracing::info!(event = "duos.seat.back", matchId = %self.match_id_str(), seat = %seat);
            }
            DuosNote::ActionRejected { seat, reason } => {
                tracing::warn!(event = "duos.action.rejected", matchId = %self.match_id_str(), seat = %seat, reason = %reason);
            }
            DuosNote::CeilingRetrying => {
                tracing::warn!(event = "duos.ceiling.retrying", matchId = %self.match_id_str());
            }
            DuosNote::AiRefused { seat, answer } => {
                if let DuosServerMessage::Error { code, message, .. } = answer_message("", answer) {
                    tracing::warn!(event = "duos.ai.refused", matchId = %self.match_id_str(), seat = %seat, code = ?code, message = %message);
                }
            }
        }
    }

    /// Carries out a timer the machine asked for: a tokio wait that comes back, on the queue, as the
    /// machine's own `TimerFired`.
    fn apply_timer(&self, core: &mut Core, command: TimerCommand) {
        match command {
            TimerCommand::Set { key, id, at } => {
                let actor = Arc::downgrade(&self.shared);
                let timer = after(at - now_ms(), move |_| {
                    if let Some(shared) = actor.upgrade() {
                        DuosActor { shared }.enqueue(Task::Timer { key, id });
                    }
                });
                core.timers.insert(key, (id, timer));
            }
            TimerCommand::Cancel { key, id } => {
                if core.timers.get(&key).is_some_and(|(held, _)| *held == id) {
                    core.timers.shift_remove(&key);
                }
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

    /// `seat`'s view with the `clock_ms` deadline filled in outside the engine (R79): what its socket is
    /// shown, and (R2881000) what its watchers are.
    fn seat_view(&self, core: &Core, seat: DuosSeat) -> DuosView {
        crate::actor::engine::run_pinned(self.shared.pinned, || core.host.view_for(seat, now_ms()))
    }

    /// A message of the machine's, framed for the person's socket.
    fn send_seat(&self, core: &mut Core, seat: DuosSeat, message: DuosSeatMessage) {
        let frame = match message {
            DuosSeatMessage::View { view, legal } => {
                let index = human(seat);
                DuosServerMessage::View {
                    view,
                    legal,
                    jlore_capped: index.map(|index| core.jlore_capped[index]).unwrap_or(false),
                    economy: index.and_then(|index| core.economy[index].clone()),
                }
            }
            DuosSeatMessage::Clock {
                now,
                round_deadline,
                grace_a1,
                grace_a2,
            } => DuosServerMessage::Clock {
                now,
                round_deadline,
                grace_deadlines: GraceDeadlines {
                    a1: grace_a1,
                    a2: grace_a2,
                },
            },
            DuosSeatMessage::Teammate {
                connected,
                ai_playing,
            } => DuosServerMessage::Teammate {
                connected,
                ai_playing,
            },
        };
        self.send(core, seat, &frame);
    }

    /// B35: view and legal actions, sent anew.
    fn push_view(&self, core: &mut Core, seat: DuosSeat) {
        if self.socket_for(core, seat).is_none() {
            return;
        }
        let view = Box::new(self.seat_view(core, seat));
        let legal = crate::actor::engine::run_pinned(self.shared.pinned, || core.host.legal(seat));
        self.send_seat(core, seat, DuosSeatMessage::View { view, legal });
    }

    // The end of the match (B40)

    /// B40: writes `result` and `ended_at` once; a lost write lets go, for a rebuild to write it.
    async fn on_record(&self, result: &jackioh_engine::wire::DuosResult, at: i64) {
        let Some(economy) = self.record_result(result, at).await else {
            self.let_go("duos.result.unrecorded");
            return;
        };
        self.release_seats();
        self.lock().economy = economy;
        self.load_jlore_caps(true).await;
        tracing::info!(event = "duos.over", matchId = %self.match_id_str(), outcome = %result.outcome, reason = %result.reason);
    }

    /// R1253000, R1047001: ends the match as a ceiling draw (B40) and lets go as `end_abandoned` does.
    async fn end_as_ceiling_draw(&self, at: i64) {
        let result = ceiling_draw();
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
                self.push_view(&mut core, seat);
            }
        }
    }

    /// Retries `finish_match` ([[R1502000]]) with backoff; a result already there has landed too.
    /// Returns payout per `HUMAN_SEATS` (#1657).
    async fn record_result(
        &self,
        result: &jackioh_engine::wire::DuosResult,
        at: i64,
    ) -> Option<[Option<GameEconomy>; 2]> {
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
        if self.lock().host.is_stopped() {
            return;
        }
        registry(&self.shared.app).forget(self.match_id_str());
        self.halt();
        tracing::warn!(event = event, matchId = %self.match_id_str());
    }

    /// R1253000: `reduce` panicked on this action. The machine caches the nonce failure; this files the
    /// finding with `bug_reports` (seq and action type, omitting private panic text).
    fn on_reduce_panicked(&self, action: &DuosAction, seq: i64, message: &str) {
        let kind = match &action.body {
            DuosActionBody::Duel { body } => body.action_type().to_string(),
            DuosActionBody::Pass { .. } => "pass".to_string(),
            DuosActionBody::Concede => "concede".to_string(),
        };
        tracing::error!(event = "duos.action.panicked", matchId = %self.match_id_str(), seq = seq, seat = %action.seat, "type" = %kind, message = %message);
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
    }

    // The AI's seats (B36–B38)

    /// B26–B29: the AI's search for the seat the machine asked for, off the actor, on the blocking pool; the
    /// move it finds comes back as a `Decided` task. The pause before it (B36) and what a search that
    /// finds nothing falls back on are the machine's.
    fn spawn_search(&self, core: &Core, request: DuosDecide) {
        let DuosDecide { seat, cursor, .. } = request;
        let job = AiJob {
            state: core.host.game().clone(),
            seat,
            seed: self.shared.row.seed.clone(),
            cursor,
            pinned: self.shared.pinned,
        };
        let weak = Arc::downgrade(&self.shared);
        tokio::spawn(async move {
            let decided =
                tokio::task::spawn_blocking(move || std::panic::catch_unwind(AssertUnwindSafe(|| job.ask())))
                    .await;
            let (body, cursor, failed) = match decided {
                Ok(Ok((body, cursor))) => (body, cursor, false),
                Ok(Err(payload)) => {
                    tracing::error!(event = "duos.ai.threw", seat = %seat, message = %panic_text(payload));
                    (None, cursor, true)
                }
                Err(error) => {
                    tracing::error!(event = "duos.ai.threw", seat = %seat, message = %join_error_text(error));
                    (None, cursor, true)
                }
            };
            if let Some(shared) = weak.upgrade() {
                DuosActor { shared }.enqueue(Task::Decided {
                    seat,
                    body,
                    cursor,
                    failed,
                });
            }
        });
    }

    // Sockets

    /// R835000: records rejection, alerting on threshold crossing.
    fn note_rejection(&self, core: &mut Core, index: usize, now: i64) {
        if let Some(rejected) = core.guards[index].note_rejection(now) {
            let profile_id = [&self.shared.row.a1, &self.shared.row.a2][index];
            alert_unusual_rejections(self.match_id_str(), profile_id, rejected);
        }
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
            let outputs = self.step_in(&mut core, DuosHostEvent::Left { seat });
            self.exec_now(&mut core, outputs);
            if core.host.is_stopped() {
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

    /// B39: relays rate-limited pings to teammate; a hand target off the team's own hand is dropped
    /// silently (R643).
    fn relay_ping(&self, core: &mut Core, seat: DuosSeat, ping: PingKind, target: PingTarget) {
        let Some(index) = human(seat) else { return };
        if core.host.is_stopped() {
            return;
        }
        if target.at == PingAt::Hand {
            let size = core
                .host
                .game()
                .duels
                .get(usize::from(target.duel))
                .map(|duel| duel.players[target.side].hand.len());
            let shown =
                target.side == seat.player() && target.index.zip(size).is_some_and(|(at, size)| at < size);
            if !shown {
                return;
            }
        } else if core.host.game().duels.get(usize::from(target.duel)).is_none() {
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
            if core.host.is_stopped() {
                None
            } else {
                // R835001: new socket generation so hello is never absorbed by older one.
                core.socket_generation[index] += 1;
                let previous = core.sockets[index].replace(socket.clone());
                // R744: the person is here, so no grace is started for it as the queue runs.
                let outputs = self.step_in(&mut core, DuosHostEvent::Connected { seat });
                self.exec_now(&mut core, outputs);
                Some(previous)
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
        engine::view(self.shared.pinned, self.lock().host.game(), seat)
    }

    /// `duos_hash` of the live state, for a rebuild's check; never sent to a socket.
    pub fn hash(&self) -> String {
        engine::hash(self.lock().host.game())
    }

    /// The next seq the log will take.
    pub fn next_seq(&self) -> i64 {
        self.lock().host.next_seq()
    }

    /// B37: whether the AI plays this person's seat now.
    pub fn ai_playing(&self, seat: DuosSeat) -> bool {
        self.lock().host.ai_playing(seat)
    }

    /// Whether the result is written.
    pub fn is_finished(&self) -> bool {
        self.lock().host.is_finished()
    }

    /// When the result was written (or the finished actor was rebuilt).
    pub fn finished_at(&self) -> Option<i64> {
        self.lock().host.finished_at()
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
            // The machine stops its clocks and takes their timers back.
            let outputs = self.step_in(&mut core, DuosHostEvent::Stop);
            self.exec_now(&mut core, outputs);
            [core.sockets[0].take(), core.sockets[1].take()]
        };
        for socket in sockets.into_iter().flatten() {
            if socket.is_open() {
                socket.close(Some(CLOSE_GOING_AWAY), Some("the actor is going away"));
            }
        }
        // R2881000: and so do its watchers.
        self.spectate_close(CLOSE_GOING_AWAY, "the actor is going away");
    }
}

// Watchers (#2881, R2881000)
//
// A friend's watch of a person's seat, as a 1v1 match's (`actor::match_actor`, B50, B51): each change
// records each person's seat's view, `legal: []`, with the clock as that seat was shown it, and a
// watcher is sent each view of its friend's seat SPECTATE_DELAY_MS after the state it shows, by one
// armed wait. That view is all a watcher is sent of the game: never a ping, a `clock` or `teammate`
// frame, `jloreCapped` or `economy`; beside it, its own emote hand (R2990001) and the other watchers'
// emotes. Nothing a watcher's socket sends reaches the reducer, the log, a ping or a clock: its emotes
// alone are relayed, live, to both people and the other watchers, from its own hand under its own
// limit (R2990000-R2990003). Past its malformed budget it is closed with 1008. Its leaving changes only
// the count both people are told.

use std::collections::BTreeMap;

use jackioh_engine::{EmoteId, emote_gate_within, hand_holds};

use crate::actor::match_actor::{MATCH_OVER_CLOSE_REASON, WATCH_NOT_ALLOWED, WatchRefused};
use crate::actor::protocol::{spectator_emote_message, spectator_hand_message, watchers_frame};
use crate::actor::ws_server::WS_CLOSE;
use crate::config::{SPECTATE_DELAY_MS, SPECTATORS_PER_MATCH};
use crate::social::watch::{
    SPECTATOR_EMOTE_LIMITS, WatchFrame, deal_spectator_emote_hand, parse_watch_frame, send_spectator_emote,
};

/// One recorded view of a person's seat: its encoded `view` frame, when the state it shows was reached,
/// and its place in the record.
struct DuosSpectateFrame {
    seq: u64,
    shown_at: i64,
    frame: String,
}

/// One watching socket: the account behind it, the `HUMAN_SEATS` index of its friend's seat, the last
/// recorded view it was sent, and (R2609000) its own `SeatGuard`, holding a seat's malformed budget and
/// its answered hellos. R2990000, R2990001: and the name its emotes are shown under, and its own hand.
struct DuosWatcher {
    id: u64,
    profile_id: String,
    name: String,
    home: usize,
    socket: Socket,
    sent: Option<u64>,
    guard: SeatGuard,
    emote_hand: Vec<EmoteId>,
}

/// What the actor keeps for its watchers. Each person's seat's record (by `HUMAN_SEATS`) holds the
/// newest view at least SPECTATE_DELAY_MS old and every view since: all a watcher can still be owed,
/// and nothing older.
#[derive(Default)]
struct DuosSpectate {
    records: [VecDeque<DuosSpectateFrame>; 2],
    watchers: Vec<DuosWatcher>,
    next_seq: u64,
    next_id: u64,
    /// The one wait armed for the next view due to a watcher, and when it is due.
    timer: Option<(Timer, i64)>,
    /// Set once the actor stops: no watcher is taken or sent anything more.
    closed: bool,
    /// R2990002: each watching account's recent emote sends for `emote_gate_within`, kept by account
    /// for the actor's life, so a new socket of the account starts from where its last one stopped.
    emote_sent_at: BTreeMap<String, Vec<i64>>,
}

/// R1107000: closes the sockets of the watchers `DuosActor::spectate_let_go_finished` let go, each with
/// CLOSE_NORMAL. Called without the actor's lock, which each socket's close handler takes; the handler
/// finds its watcher already out of the list.
fn close_finished_watchers(watchers: Vec<DuosWatcher>) {
    for watcher in watchers {
        if watcher.socket.is_open() {
            watcher
                .socket
                .close(Some(CLOSE_NORMAL), Some(MATCH_OVER_CLOSE_REASON));
        }
    }
}

impl DuosActor {
    /// B50, R2881000: `profile_id`'s watcher socket on the person's `seat`. A newer socket of the account
    /// replaces its older one; otherwise a game with SPECTATORS_PER_MATCH watchers, both seats' counted
    /// together, is `Full`. An AI seat, or an actor going away, is `Closed`. Its hellos are answered
    /// (R1103000), its emotes relayed under `name` (R2990000), its other frames count against its
    /// malformed budget (R2609000).
    pub fn attach_watcher(
        &self,
        seat: DuosSeat,
        profile_id: &str,
        name: &str,
        socket: Socket,
    ) -> Result<(), WatchRefused> {
        let Some(home) = human(seat) else {
            return Err(WatchRefused::Closed);
        };
        // R2990001: the watcher's own hand, the same on every socket of the account.
        let hand = deal_spectator_emote_hand(&self.shared.row.seed, profile_id);
        let (id, replaced, done) = {
            let mut core = self.lock();
            if core.host.is_stopped() || core.spectate.closed {
                return Err(WatchRefused::Closed);
            }
            let at = core
                .spectate
                .watchers
                .iter()
                .position(|watcher| watcher.profile_id == profile_id);
            let replaced = at.map(|at| core.spectate.watchers.remove(at));
            if replaced.is_none() && core.spectate.watchers.len() >= SPECTATORS_PER_MATCH {
                return Err(WatchRefused::Full);
            }
            // Those already watching are sent what has come due first, so each record is down to the
            // view the newcomer starts from.
            self.spectate_flush(&mut core, now_ms());
            let done = self.spectate_let_go_finished(&mut core);
            let id = core.spectate.next_id;
            core.spectate.next_id += 1;
            core.spectate.watchers.push(DuosWatcher {
                id,
                profile_id: profile_id.to_string(),
                name: name.to_string(),
                home,
                socket: socket.clone(),
                sent: None,
                guard: SeatGuard::default(),
                emote_hand: hand.clone(),
            });
            (id, replaced, done)
        };

        let actor = Arc::downgrade(&self.shared);
        let heard = actor.clone();
        socket.attach(SocketHandlers {
            // B50: nothing a watcher sends reaches the reducer or a ping. R1103000: a hello is
            // answered; R2990000: an emote is relayed, and one naming no pool id answered `malformed`;
            // R2609000: the latter and anything else count against the watcher's malformed budget.
            message: Box::new(move |text: String| {
                let Some(shared) = heard.upgrade() else { return };
                let actor = DuosActor { shared };
                match parse_watch_frame(&text) {
                    WatchFrame::Hello => actor.answer_watcher_hello(id),
                    WatchFrame::Emote(emote) => actor.relay_spectator_emote(id, emote),
                    WatchFrame::Malformed(reason) => actor.note_watcher_junk(id, Some(&reason)),
                    WatchFrame::Junk => actor.note_watcher_junk(id, None),
                }
            }),
            close: Box::new(move || {
                if let Some(shared) = actor.upgrade() {
                    DuosActor { shared }.detach_watcher(id);
                }
            }),
        });
        let replacing = replaced.is_some();
        if let Some(previous) = replaced {
            // Already out of the list, so its close handler finds nothing to take off.
            previous
                .socket
                .close(Some(CLOSE_NORMAL), Some("replaced by a new socket"));
        }
        // R1107000: a watcher already in may just have had the finished game's last view.
        close_finished_watchers(done);
        // A socket that went before its handler was in has no close left to run.
        if !socket.is_open() {
            self.detach_watcher(id);
            return Ok(());
        }

        let mut core = self.lock();
        // R2990001: the watcher's own emote hand, to it alone, before any view.
        socket.send(encode(&spectator_hand_message(&hand)));
        // The newcomer's first view, when one is already SPECTATE_DELAY_MS old.
        self.spectate_flush(&mut core, now_ms());
        if !replacing {
            self.push_watchers(&core);
        }
        tracing::info!(event = "duos.watcher.attached", matchId = %self.match_id_str(), seat = %seat, watchers = core.spectate.watchers.len());
        // R1107000: the newcomer's first view may be the finished game's last, and then it goes at once.
        let done = self.spectate_let_go_finished(&mut core);
        drop(core);
        close_finished_watchers(done);
        Ok(())
    }

    /// B51: how many sockets watch this game now.
    pub fn watcher_count(&self) -> usize {
        self.lock().spectate.watchers.len()
    }

    /// B50, B46: revokes the watchers for which `revoked(watcher's account, its friend's, the friend's
    /// teammate's)` holds (unfriended, blocked, a deleted account), sending forbidden and closing with
    /// R148's 4403; updates both people's count (B51). Answers how many.
    pub fn revoke_watchers(&self, revoked: &dyn Fn(&str, &str, &str) -> bool) -> usize {
        let gone: Vec<DuosWatcher> = {
            let mut core = self.lock();
            let row = &self.shared.row;
            let friends = [row.a1.as_str(), row.a2.as_str()];
            let (gone, kept): (Vec<DuosWatcher>, Vec<DuosWatcher>) =
                std::mem::take(&mut core.spectate.watchers)
                    .into_iter()
                    .partition(|watcher| {
                        revoked(
                            &watcher.profile_id,
                            friends[watcher.home],
                            friends[1 - watcher.home],
                        )
                    });
            core.spectate.watchers = kept;
            if !gone.is_empty() && !core.host.is_stopped() {
                tracing::info!(event = "duos.watcher.revoked", matchId = %self.match_id_str(), revoked = gone.len(), watchers = core.spectate.watchers.len());
                self.push_watchers(&core);
            }
            gone
        };
        // Already out of the list, so their close handlers find nothing to take off.
        let refusal = encode(&error_message(
            SocketErrorCode::Forbidden,
            WATCH_NOT_ALLOWED,
            None,
        ));
        for watcher in &gone {
            if watcher.socket.is_open() {
                watcher.socket.send(&refusal);
                watcher
                    .socket
                    .close(Some(WS_CLOSE.forbidden), Some(WATCH_NOT_ALLOWED));
            }
        }
        gone.len()
    }

    /// R1103000: a watcher's later hello is answered with its last view again, so its page hears the
    /// socket is alive (R1441's probe); one sent no view yet is sent nothing. R2609000: at most once per
    /// `WATCH_HELLO_INTERVAL_MS`; a hello that finds no view to send starts no interval.
    fn answer_watcher_hello(&self, id: u64) {
        let mut core = self.lock();
        if core.host.is_stopped() || core.spectate.closed {
            return;
        }
        let DuosSpectate {
            records, watchers, ..
        } = &mut core.spectate;
        let Some(watcher) = watchers.iter_mut().find(|watcher| watcher.id == id) else {
            return;
        };
        let Some(record) = records[watcher.home]
            .iter()
            .find(|record| watcher.sent == Some(record.seq))
        else {
            return;
        };
        // R2609000: the watcher's id is its one generation (R2686000 gives a seat's socket the same gate).
        if !watcher.guard.hello_answer_allowed(id, now_ms()) {
            return;
        }
        watcher.socket.send(record.frame.as_str());
    }

    /// R2609000: a frame of a watcher's that is neither a hello nor an emote is still unanswered
    /// (R1103000) but counts against the watcher's own `MALFORMED_FRAMES_PER_WINDOW`, as a seat's does
    /// (R835002); R2990000: an emote frame naming no pool id counts too, and is answered `malformed`
    /// with `answer` first, as a 1v1 watcher's is. Past the budget the socket is closed with 1008, and
    /// its close handler takes the watcher off and tells both people the new count (B51).
    fn note_watcher_junk(&self, id: u64, answer: Option<&str>) {
        let socket = {
            let mut core = self.lock();
            if core.host.is_stopped() || core.spectate.closed {
                return;
            }
            let Some(watcher) = core.spectate.watchers.iter_mut().find(|watcher| watcher.id == id) else {
                return;
            };
            let over = watcher.guard.note_malformed(now_ms());
            if let Some(reason) = answer {
                watcher
                    .socket
                    .send(encode(&error_message(SocketErrorCode::Malformed, reason, None)));
            }
            if !over {
                return;
            }
            watcher.socket.clone()
        };
        tracing::warn!(event = "duos.watcher.flooded", matchId = %self.match_id_str());
        // The close calls `detach_watcher` back, which takes the actor's lock.
        socket.close(Some(CLOSE_POLICY_VIOLATED), Some("too many malformed frames"));
    }

    /// R2990000: watcher `id`'s emote, as a 1v1 watcher's (`MatchActor::relay_spectator_emote`): one
    /// its hand does not hold (R2990001) or past its limit (R2990002) is dropped silently; one let
    /// through goes out live as `spectatorEmote` to both people's own sockets (an AI seat has none to
    /// hear it) and every other watcher, never back to its sender, and (R2990003) to none that blocks
    /// it or is blocked by it. It is never queued, so no reducer, log, hash or record sees it.
    fn relay_spectator_emote(&self, id: u64, emote: EmoteId) {
        let (sender_id, frame, recipients) = {
            let mut core = self.lock();
            if core.host.is_stopped() || core.spectate.closed {
                return;
            }
            let Some(watcher) = core.spectate.watchers.iter().find(|watcher| watcher.id == id) else {
                return;
            };
            if !hand_holds(&watcher.emote_hand, emote) {
                tracing::warn!(event = "duos.spectator_emote.outside_hand", matchId = %self.match_id_str(), emote = %emote);
                return;
            }
            let sender_id = watcher.profile_id.clone();
            let frame = encode(&spectator_emote_message(&watcher.name, emote));
            let now = now_ms();
            let history = core
                .spectate
                .emote_sent_at
                .get(&sender_id)
                .map_or(&[][..], Vec::as_slice);
            let gate = emote_gate_within(history, now, SPECTATOR_EMOTE_LIMITS);
            if !gate.ok {
                return;
            }
            let mut sent_at = gate.sent_at;
            sent_at.push(now);
            core.spectate.emote_sent_at.insert(sender_id.clone(), sent_at);
            let row = &self.shared.row;
            let people =
                [&row.a1, &row.a2]
                    .into_iter()
                    .zip(&core.sockets)
                    .filter_map(|(profile_id, socket)| {
                        socket.as_ref().map(|socket| (profile_id.clone(), socket.clone()))
                    });
            let watchers = core
                .spectate
                .watchers
                .iter()
                .filter(|other| other.id != id)
                .map(|other| (other.profile_id.clone(), other.socket.clone()));
            let recipients: Vec<(String, Socket)> = people.chain(watchers).collect();
            (sender_id, frame, recipients)
        };
        tokio::spawn(send_spectator_emote(
            self.shared.app.clone(),
            sender_id,
            recipients,
            frame,
        ));
    }

    /// A watcher's socket went: it leaves, and both people hear the new count (B51).
    fn detach_watcher(&self, id: u64) {
        let mut core = self.lock();
        let before = core.spectate.watchers.len();
        core.spectate.watchers.retain(|watcher| watcher.id != id);
        if core.spectate.watchers.len() == before || core.host.is_stopped() {
            return;
        }
        tracing::info!(event = "duos.watcher.detached", matchId = %self.match_id_str(), watchers = core.spectate.watchers.len());
        self.push_watchers(&core);
    }

    /// B51: the count, to both people's own sockets.
    fn push_watchers(&self, core: &Core) {
        let frame = watchers_frame(core.spectate.watchers.len());
        for index in 0..HUMAN_SEATS.len() {
            self.send_text_to_seat(core, index, &frame);
        }
    }

    /// B51: the count, to the person in `HUMAN_SEATS[index]`, when anyone watches: what its socket
    /// missed while it was away.
    fn push_watchers_to(&self, core: &Core, index: usize) {
        let count = core.spectate.watchers.len();
        if count > 0 {
            self.send_text_to_seat(core, index, &watchers_frame(count));
        }
    }

    fn send_text_to_seat(&self, core: &Core, index: usize, text: &str) {
        if let Some(socket) = &core.sockets[index]
            && socket.is_open()
        {
            socket.send(text);
        }
    }

    /// B50: records each person's seat's view of the state as it stands now, the clock's time left in
    /// it as the seat was shown, and sends whatever has come due.
    fn spectate_record_in(&self, core: &mut Core) {
        if core.spectate.closed {
            return;
        }
        let now = now_ms();
        for (index, seat) in HUMAN_SEATS.into_iter().enumerate() {
            let frame = encode(&DuosServerMessage::View {
                view: Box::new(self.seat_view(core, seat)),
                legal: Vec::new(),
                jlore_capped: false,
                economy: None,
            });
            let seq = core.spectate.next_seq;
            core.spectate.next_seq += 1;
            core.spectate.records[index].push_back(DuosSpectateFrame {
                seq,
                shown_at: now,
                frame,
            });
        }
        self.spectate_flush(core, now);
    }

    /// Sends each watcher, oldest first, every view of its friend's seat that has come due
    /// (SPECTATE_DELAY_MS after the state it shows) and that it has not had; then drops each record's
    /// views older than its newest due one, and arms the wait for the next view due.
    fn spectate_flush(&self, core: &mut Core, now: i64) {
        let DuosSpectate {
            records,
            watchers,
            timer,
            closed,
            ..
        } = &mut core.spectate;
        if *closed {
            return;
        }
        for watcher in watchers.iter_mut() {
            for record in &records[watcher.home] {
                if record.shown_at + SPECTATE_DELAY_MS > now {
                    break;
                }
                if watcher.sent.is_some_and(|sent| record.seq <= sent) {
                    continue;
                }
                watcher.socket.send(record.frame.as_str());
                watcher.sent = Some(record.seq);
            }
        }
        for record in records.iter_mut() {
            while record
                .get(1)
                .is_some_and(|next| next.shown_at + SPECTATE_DELAY_MS <= now)
            {
                record.pop_front();
            }
        }

        let next_due = watchers
            .iter()
            .filter_map(|watcher| {
                records[watcher.home]
                    .iter()
                    .map(|record| record.shown_at + SPECTATE_DELAY_MS)
                    .find(|due| *due > now)
            })
            .min();
        if timer.as_ref().map(|(_, due)| *due) == next_due {
            return;
        }
        if let Some((armed, _)) = timer.take() {
            armed.cancel();
        }
        let Some(due) = next_due else { return };
        let actor = Arc::downgrade(&self.shared);
        let armed = after(due - now, move |id| {
            let Some(shared) = actor.upgrade() else { return };
            let actor = DuosActor { shared };
            let mut core = actor.lock();
            if core.spectate.timer.as_ref().map(|(armed, _)| armed.id()) != Some(id) {
                return;
            }
            if let Some((armed, _)) = core.spectate.timer.take() {
                armed.fired();
            }
            actor.spectate_flush(&mut core, now_ms());
            let done = actor.spectate_let_go_finished(&mut core);
            drop(core);
            close_finished_watchers(done);
        });
        *timer = Some((armed, due));
    }

    /// R1107000, B51: removes the watchers that have been sent the view showing a finished game's
    /// result and tells both people the count. Returns the watchers for `close_finished_watchers`;
    /// called after each `spectate_flush` that can send that view. A game ended with no result in its
    /// state (R1253000's draw) is let go by `halt` instead.
    #[must_use]
    fn spectate_let_go_finished(&self, core: &mut Core) -> Vec<DuosWatcher> {
        if !core.host.is_finished() || core.host.game().result.is_none() || core.spectate.closed {
            return Vec::new();
        }
        let records = &core.spectate.records;
        let (done, kept): (Vec<DuosWatcher>, Vec<DuosWatcher>) = std::mem::take(&mut core.spectate.watchers)
            .into_iter()
            .partition(|watcher| {
                records[watcher.home]
                    .back()
                    .is_some_and(|last| watcher.sent == Some(last.seq))
            });
        core.spectate.watchers = kept;
        if !done.is_empty() && !core.host.is_stopped() {
            tracing::info!(event = "duos.watcher.finished", matchId = %self.match_id_str(), finished = done.len(), watchers = core.spectate.watchers.len());
            self.push_watchers(core);
        }
        done
    }

    /// The actor stops: every watcher's socket closes with `code` and `reason`, the wait is canceled,
    /// and nothing more is kept or taken.
    fn spectate_close(&self, code: u16, reason: &str) {
        let watchers = {
            let mut core = self.lock();
            let spectate = &mut core.spectate;
            spectate.closed = true;
            if let Some((armed, _)) = spectate.timer.take() {
                armed.cancel();
            }
            spectate.records = Default::default();
            std::mem::take(&mut spectate.watchers)
        };
        for watcher in watchers {
            if watcher.socket.is_open() {
                watcher.socket.close(Some(code), Some(reason));
            }
        }
    }
}
