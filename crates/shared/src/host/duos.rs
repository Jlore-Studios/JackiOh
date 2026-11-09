//! The Duos co-op match host's decisions as one pure state machine (#3101's second part; fullsend SPEC
//! S4 to S6, B35 to B40, §13), the 1v1 machine's twin (`duel.rs`): the same `step(host, event, now) ->
//! (host, outputs)`, the same shapes of event and output, on a `DuosState` of two duels and four seats —
//! a1 and a2 are people, b1 and b2 the AI's.
//!
//! What the machine decides, and what stays the host's:
//!
//!  - which clock runs (the mulligan clock over the four mulligans, the round clock, a prompt clock for a
//!    person outside its round, a disconnect grace and the hard ceiling) and what an expiry hands the AI:
//!    a lapsed mulligan keeps the hand (§13.2, R268), a lapsed round answers the prompts and ends the turn
//!    of every round seat (B38), a lapsed prompt answers that prompt alone, a lapsed grace hands the seat
//!    to the AI until its person is back (B37, no loss by disconnect);
//!  - the AI protocol: when a seat owes the game something it plays (b1 and b2 always, a person's seat in
//!    the mode an expiry forced or past its grace), the pause before it acts (B36), the move it falls back
//!    on, what a refused or unwritten move comes to, and the ceiling's retry (R776000, R1047000);
//!  - the order every view, clock and teammate frame goes out in, and the game's end (B40, R1253000).
//!
//! The host owns its sockets, its store, its abuse guards, the pings (B39), its watchers (R2881000) and
//! the AI's search itself: the machine asks for it (`DuosHostOutput::Decide`) and is told what it came
//! to (`DuosHostEvent::Decided`), so every AI call is one separate, deadline-bounded step.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use jackioh_engine::duos::{DuosState, duos_legal_actions, duos_reduce, duos_seats_to_act, duos_view_for};
use jackioh_engine::wire::{
    ActionBody, DUOS_SEATS, DuosAction, DuosActionBody, DuosResult, DuosSeat, DuosView,
};
use jackioh_engine::{CombatForecast, combat_forecast, mulligan_owed};

use super::bot::r188_forbidden;
use super::clock::match_ceiling_at;
use super::timer::{Fx, TimerCommand, TimerKey};
use super::types::{ActionAnswer, GreetingPart, LogOutcome, SERVER_NONCE_PREFIX};
use crate::config::{
    BOT_WRITE_RETRY_MS, DISCONNECT_GRACE_SECONDS, DUOS_AI_STEP_DELAY_MS, DUOS_MULLIGAN_CLOCK_MS,
    DUOS_PROMPT_CLOCK_MS, DUOS_ROUND_CLOCK_MS, MATCH_SERVER_ACTION_RETRY_MS, MS_PER_SECOND,
};

/// Human player seats, host and guest (S6): the per-person arrays' order.
pub const HUMAN_SEATS: [DuosSeat; 2] = [DuosSeat::A1, DuosSeat::A2];

/// A seat's index in `DUOS_SEATS` order (a1, a2, b1, b2).
pub fn slot(seat: DuosSeat) -> usize {
    match seat {
        DuosSeat::A1 => 0,
        DuosSeat::A2 => 1,
        DuosSeat::B1 => 2,
        DuosSeat::B2 => 3,
    }
}

/// A person's index in `HUMAN_SEATS`, or none for an AI seat.
pub fn human(seat: DuosSeat) -> Option<usize> {
    match seat {
        DuosSeat::A1 => Some(0),
        DuosSeat::A2 => Some(1),
        DuosSeat::B1 | DuosSeat::B2 => None,
    }
}

/// R1047000: when a match created at `created_at` reaches the hard ceiling (§9.5, R79). Derived from the row
/// and never stored, so a rebuilt host keeps the match's own deadline.
pub fn ceiling_at(created_at: i64) -> i64 {
    match_ceiling_at(created_at)
}

/// The id of the prompt `seat` holds in its duel, if any.
pub fn held_prompt(state: &DuosState, seat: DuosSeat) -> Option<String> {
    let duel = state.duels.get(seat.duel())?;
    duel.pending
        .as_ref()
        .filter(|pending| pending.player_id == seat.player())
        .map(|pending| pending.id.clone())
}

/// Whether `seat` still owes its mulligan (B2).
pub fn owes_mulligan(state: &DuosState, seat: DuosSeat) -> bool {
    state
        .duels
        .get(seat.duel())
        .is_some_and(|duel| mulligan_owed(duel).contains(&seat.player()))
}

/// Whether `seat` owes an answer: a prompt or its mulligan.
pub fn owes_answer(state: &DuosState, seat: DuosSeat) -> bool {
    held_prompt(state, seat).is_some() || owes_mulligan(state, seat)
}

/// Whether `seat` is its duel's round seat and has not ended (B4).
pub fn acting_round_seat(state: &DuosState, seat: DuosSeat) -> bool {
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
pub fn ai_forbidden(body: &DuosActionBody) -> bool {
    match body {
        DuosActionBody::Concede => true,
        DuosActionBody::Duel { body } => r188_forbidden(body),
        DuosActionBody::Pass { .. } => false,
    }
}

/// Fallback move when the AI yields no playable action: `EndTurn` if legal, else an answer, else the first
/// legal action not forbidden.
pub fn fallback(legal: &[DuosActionBody]) -> Option<DuosActionBody> {
    let allowed = || legal.iter().filter(|body| !ai_forbidden(body));
    allowed()
        .find(|body| is_end_turn(body))
        .or_else(|| allowed().find(|body| is_answer(body)))
        .or_else(|| allowed().next())
        .cloned()
}

/// R268: the mulligan that keeps every card of the hand, the legal one keeping the most.
pub fn keep_whole_hand(legal: &[DuosActionBody]) -> Option<DuosActionBody> {
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

/// Why a person's seat (a1, a2) is in the AI's hands for a while.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "forced", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosForced {
    /// The mulligan clock (§13.2, R268): its mulligan is answered by keeping the whole hand.
    KeepHand,
    /// B38: this round, its prompts are answered and its turn ended for it.
    FinishRound { round: i32 },
    /// The prompt clock: this one prompt is answered for it.
    AnswerPrompt { choice: String },
}

/// How the AI plays a seat now.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosAiMode {
    /// b1 and b2, and a person's seat past its grace (B36, B37): everything it owes.
    Full,
    /// Its mulligan, by keeping the whole hand: never the AI's own mulligan (R268).
    KeepHand,
    /// Its prompts and mulligan, then its end of turn (B38).
    FinishRound,
    /// That prompt only.
    AnswerPrompt { choice: String },
}

/// Whether `seat` owes the game something the AI would play in `mode`.
pub fn owes(state: &DuosState, seat: DuosSeat, mode: &DuosAiMode) -> bool {
    match mode {
        DuosAiMode::Full => duos_seats_to_act(state).contains(&seat),
        DuosAiMode::KeepHand => owes_mulligan(state, seat),
        DuosAiMode::FinishRound => {
            owes_answer(state, seat) || duos_legal_actions(state, seat).iter().any(is_end_turn)
        }
        DuosAiMode::AnswerPrompt { choice } => held_prompt(state, seat).as_deref() == Some(choice.as_str()),
    }
}

/// Fallback action for `seat` in `mode` without the AI's search.
pub fn without_search(
    mode: &DuosAiMode,
    state: &DuosState,
    seat: DuosSeat,
    legal: &[DuosActionBody],
) -> Option<DuosActionBody> {
    match mode {
        DuosAiMode::KeepHand => keep_whole_hand(legal),
        DuosAiMode::FinishRound if !owes_answer(state, seat) => {
            legal.iter().find(|body| is_end_turn(body)).cloned()
        }
        _ => fallback(legal),
    }
}

/// What a seat is sent. The host encodes it for its own transport; a seat's `View` holds `view_for` of that
/// seat and nothing else (B23, CLAUDE.md rule 7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "message", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosSeatMessage {
    /// B35: the seat's view, its clock filled in (R79), and its own legal actions.
    View {
        view: Box<DuosView>,
        legal: Vec<DuosActionBody>,
        /// R3477000: the engine's forecast of the attacks among `legal` (`DuosHost::forecast`); never a
        /// watcher's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        forecast: Option<CombatForecast>,
    },
    /// S6: the deadline the seat is playing against (its round's, its prompt's, or the mulligans'), both
    /// people's disconnect graces, and the host's `now` to measure them by.
    Clock {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        round_deadline: Option<i64>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        grace_a1: Option<i64>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        grace_a2: Option<i64>,
    },
    /// S6, B37: whether the teammate's socket is open, and whether the AI plays their seat.
    Teammate { connected: bool, ai_playing: bool },
}

/// The host must run the AI's search for `seat` in `mode`, and answer with [`DuosHostEvent::Decided`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosDecide {
    pub seat: DuosSeat,
    pub mode: DuosAiMode,
    /// The log's next seq when the request was made.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub version: i64,
    /// Where the seat's AI stream stands (`{seed}:duos-ai:{seat}`).
    pub cursor: u32,
}

/// Something worth a line in the host's log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "note", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosNote {
    /// A clock ran out and hands its seats to the AI.
    ClockExpired { kind: String },
    /// B37: the AI plays the seat until its person is back.
    SeatAiPlaying { seat: DuosSeat },
    /// B37: the person is back.
    SeatBack { seat: DuosSeat },
    /// Every rejected action is logged with its reason (§9.8).
    ActionRejected { seat: DuosSeat, reason: String },
    /// The ceiling's action could not be logged: it is tried again (R776000).
    CeilingRetrying,
    /// An AI seat's move was refused or could not be written.
    AiRefused { seat: DuosSeat, answer: ActionAnswer },
}

/// Why a host should let the match go.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum DuosLetGo {
    /// R776000: the log holds the host's next seq already; a rebuilt host takes over.
    Diverged,
    /// The match row is gone from the store, which takes no more rows.
    MatchGone,
}

/// An event the host feeds the machine. `Connected`, `Left` and nothing else stand for one fact the host
/// learns at once; the rest are the host's turns, one at a time, in order (the machine awaits the log
/// between an action and its `Logged`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "event", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosHostEvent {
    /// The host is up (a new one, or one made from a checkpoint): the clocks are brought up to date with
    /// the game and a game already over is ended or the AI is driven.
    Arm,
    /// The person at `seat` has a socket.
    Connected { seat: DuosSeat },
    /// Its greeting: the view, the clock and the teammate (B35), and its grace ends (B37).
    Attached { seat: DuosSeat },
    /// A hello from a socket already attached: the greeting again.
    Hello { seat: DuosSeat },
    /// The person's socket is gone.
    Left { seat: DuosSeat },
    /// Its grace begins and the teammate is told (B37).
    Disconnected { seat: DuosSeat },
    /// An action for a seat, a person's or the host's own.
    Action {
        seat: DuosSeat,
        nonce: String,
        body: DuosActionBody,
    },
    /// The log took (or refused) the row [`DuosHostOutput::Append`] asked for.
    Logged { outcome: LogOutcome },
    /// A timer the machine set came due.
    TimerFired { key: TimerKey, id: u32 },
    /// The AI's search, as [`DuosHostOutput::Decide`] asked for it. `failed`: the search threw, and no
    /// fallback is played for it.
    Decided {
        seat: DuosSeat,
        body: Option<DuosActionBody>,
        cursor: u32,
        failed: bool,
    },
    /// The store's answer to [`DuosHostOutput::CheckRow`].
    RowChecked { exists: bool },
    /// The host is going away: its clocks stop.
    Stop,
}

/// An output the host carries out, in the order given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "output", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosHostOutput {
    /// A message for the person at `seat`, if its socket is open.
    Send {
        seat: DuosSeat,
        message: DuosSeatMessage,
    },
    /// Set or take back a timer.
    Timer { command: TimerCommand },
    /// §9.3: write this row at `seq` (row before state), then feed [`DuosHostEvent::Logged`].
    Append {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seq: i64,
        action: DuosAction,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// The answer to an action the machine took, by its nonce.
    Answer { nonce: String, answer: ActionAnswer },
    /// A seat's greeting is at this part: the host adds its own messages here (the watchers' count).
    Greeting { seat: DuosSeat, part: GreetingPart },
    /// A person's reducer refusal, which an abuse guard counts (R835000).
    Refused { seat: DuosSeat },
    /// R1253000: `reduce` panicked on this action; the host reports it.
    Panicked {
        action: DuosAction,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seq: i64,
        message: String,
    },
    /// Every view and clock of a change has been sent: where watchers' views are recorded (R2881000).
    Pushed,
    /// Run the AI's search for a seat.
    Decide { request: DuosDecide },
    /// The game is over: record it (B40). Nothing is fed back.
    Record {
        result: DuosResult,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// R1253000, R1047001: a server action's `reduce` panicked: the match ends as a ceiling draw. Record
    /// it, then let the match go.
    CeilingDraw {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// An AI seat's move could not be written: does the match row still exist? Feed
    /// [`DuosHostEvent::RowChecked`].
    CheckRow,
    /// Let the match go: forget it and stop.
    LetGo { reason: DuosLetGo },
    /// A line for the host's log.
    Note { note: DuosNote },
}

/// What a Duos host is made from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosSetup {
    pub match_id: String,
    /// Epoch ms the match was created: the ceiling is measured from here (R1047000).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub created_at: i64,
    /// The live state: begun (a new match) or folded from the log (a rebuild).
    pub game: DuosState,
    /// Whether the store already holds the result (a rebuilt host on a finished match).
    pub finished: bool,
    /// The log's next seq.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub next_seq: i64,
    /// The nonce, seat and seq of every row already in the log (§9.3).
    pub acked: Vec<DuosAck>,
}

/// One row of the log, as the machine remembers it: whose nonce it was and where it landed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosAck {
    pub nonce: String,
    pub seat: DuosSeat,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub seq: i64,
}

/// A countdown: the deadline the seat is shown and the timer that reports it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosArmed {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub deadline: i64,
    pub timer: u32,
}

/// The AI's search in flight for a seat: what it was asked on, and the move to fall back on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
struct DuosJob {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    version: i64,
    mode: DuosAiMode,
    fallback: Option<DuosActionBody>,
    cursor: u32,
}

/// Where a log write stands: the action reduced, the row asked for, the state not yet moved (§9.3).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
struct DuosAwaiting {
    action: DuosAction,
    after: DuosState,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    seq: i64,
    origin: DuosOrigin,
}

/// Who an applied action is for, so the right thing follows its answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "origin", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum DuosOrigin {
    /// A person's own, or the host's submit: the answer is all that follows.
    Seat,
    /// The ceiling's (R1047000): an action the log would not take runs the clock out again.
    Ceiling,
    /// An AI seat's move (B36): a refused one falls back once; a failed one is decided again.
    Ai {
        seat: DuosSeat,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        version: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        current: i64,
        mode: DuosAiMode,
        base: String,
        retried: bool,
    },
}

/// The Duos host's state: the checkpoint. Serializable, with the `DuosState` inside it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosHost {
    match_id: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    created_at: i64,
    game: DuosState,

    // The log (§9.3)
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    next_seq: i64,
    /// §9.3: nonce -> (seat, seq), rebuilt from the log.
    acks: IndexMap<String, DuosAck>,
    /// R1253000: nonces whose `reduce` panicked; not `acks`, which holds only what the log holds.
    panicked: IndexSet<String>,
    awaiting: Option<DuosAwaiting>,

    // The people (B37)
    /// By `HUMAN_SEATS`: whether the person's socket is open.
    connected: [bool; 2],
    /// By `HUMAN_SEATS`: whether the AI plays the person's seat.
    ai_playing: [bool; 2],

    // The clocks
    grace: [Option<DuosArmed>; 2],
    mulligan: Option<DuosArmed>,
    mulligan_expired: bool,
    round_clock: Option<(i32, DuosArmed)>,
    round_expired: Option<i32>,
    prompt_clocks: [Option<(String, DuosArmed)>; 2],
    prompt_expired: [Option<String>; 2],
    /// R1047000: the hard ceiling's countdown.
    ceiling: Option<DuosArmed>,

    // The AI's seats (B36 to B38)
    forced: [Option<DuosForced>; 4],
    ai_busy: [bool; 4],
    /// Each seat's AI stream cursor; a rebuilt host starts them again.
    ai_cursor: [u32; 4],
    ai_jobs: [Option<DuosJob>; 4],
    /// The id of the timer that holds each seat's pause before it acts.
    ai_timer: [Option<u32>; 4],
    ai_retry: Option<u32>,

    // The life of the match
    finished: bool,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    finished_at: Option<i64>,
    stopped: bool,

    fx: Fx,
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    out: Vec<DuosHostOutput>,
}

/// The machine's one function: `host` and `event` in, the next `host` and its outputs out.
pub fn step_duos(mut host: DuosHost, event: DuosHostEvent, now: i64) -> (DuosHost, Vec<DuosHostOutput>) {
    let outputs = host.step(event, now);
    (host, outputs)
}

fn grace_ms() -> i64 {
    DISCONNECT_GRACE_SECONDS * MS_PER_SECOND
}

impl DuosHost {
    /// A host for a new or rebuilt co-op match. Nothing is armed until [`DuosHostEvent::Arm`].
    pub fn new(setup: DuosSetup, now: i64) -> (DuosHost, Vec<DuosHostOutput>) {
        let DuosSetup {
            match_id,
            created_at,
            game,
            finished,
            next_seq,
            acked,
        } = setup;
        let host = DuosHost {
            match_id,
            created_at,
            game,
            next_seq,
            acks: acked.into_iter().map(|ack| (ack.nonce.clone(), ack)).collect(),
            panicked: IndexSet::new(),
            awaiting: None,
            connected: [false, false],
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
            ai_jobs: [None, None, None, None],
            ai_timer: [None; 4],
            ai_retry: None,
            finished,
            finished_at: finished.then_some(now),
            stopped: false,
            fx: Fx::default(),
            out: Vec::new(),
        };
        (host, Vec::new())
    }

    /// One event in, its outputs out.
    pub fn step(&mut self, event: DuosHostEvent, now: i64) -> Vec<DuosHostOutput> {
        match event {
            DuosHostEvent::Arm => self.on_arm(now),
            DuosHostEvent::Connected { seat } => {
                if let Some(index) = human(seat) {
                    self.connected[index] = true;
                }
            }
            DuosHostEvent::Attached { seat } => self.on_attached(seat, now),
            DuosHostEvent::Hello { seat } => self.push_seat(seat, now),
            DuosHostEvent::Left { seat } => {
                if let Some(index) = human(seat) {
                    self.connected[index] = false;
                }
            }
            DuosHostEvent::Disconnected { seat } => self.on_disconnected(seat, now),
            DuosHostEvent::Action { seat, nonce, body } => {
                self.apply(seat, nonce, body, DuosOrigin::Seat, now);
            }
            DuosHostEvent::Logged { outcome } => self.on_logged(outcome, now),
            DuosHostEvent::TimerFired { key, id } => self.on_timer(key, id, now),
            DuosHostEvent::Decided {
                seat,
                body,
                cursor,
                failed,
            } => self.on_decided(seat, body, cursor, failed, now),
            DuosHostEvent::RowChecked { exists } => self.on_row_checked(exists, now),
            DuosHostEvent::Stop => self.on_stop(),
        }
        self.take_out()
    }

    // Reads, for the host's own sends and its checkpoint

    pub fn match_id(&self) -> &str {
        &self.match_id
    }

    /// The game as it stands (never sent to a seat: a seat's window onto it is [`DuosHost::view_for`]).
    pub fn game(&self) -> &DuosState {
        &self.game
    }

    /// The log's next seq.
    pub fn next_seq(&self) -> i64 {
        self.next_seq
    }

    /// Whether the result is written (or the match was finished when the host was made).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// When the result was written (or the finished host was made).
    pub fn finished_at(&self) -> Option<i64> {
        self.finished_at
    }

    /// B37: whether the AI plays this person's seat now.
    pub fn ai_playing(&self, seat: DuosSeat) -> bool {
        human(seat).is_some_and(|index| self.ai_playing[index])
    }

    /// The deadline `seat` plays against: the mulligan, round or prompt clock.
    fn deadline_for(&self, seat: DuosSeat) -> Option<i64> {
        if self.finished || self.game.result.is_some() {
            return None;
        }
        if self.game.round == 0 {
            return self.mulligan.map(|armed| armed.deadline);
        }
        if let Some((round, armed)) = &self.round_clock
            && *round == self.game.round
        {
            return Some(armed.deadline);
        }
        let index = human(seat)?;
        self.prompt_clocks[index]
            .as_ref()
            .map(|(_, armed)| armed.deadline)
    }

    /// `seat`'s view with the `clock_ms` deadline filled in outside the engine (R79): what its socket is
    /// shown, and (R2881000) what its watchers are.
    pub fn view_for(&self, seat: DuosSeat, now: i64) -> DuosView {
        let mut view = duos_view_for(&self.game, seat);
        view.own.clock_ms = self
            .deadline_for(seat)
            .map(|at| i32::try_from((at - now).max(0)).unwrap_or(i32::MAX));
        view
    }

    /// What `seat` may send now.
    pub fn legal(&self, seat: DuosSeat) -> Vec<DuosActionBody> {
        duos_legal_actions(&self.game, seat)
    }

    /// R3477000: what each attack `seat` is offered now would do, read from its duel's face-up board
    /// alone (`combat_forecast`, R3100000). Duos offers fewer of a duel's attacks than the duel allows
    /// (`duos_legal_actions`), so only the entries of the attacks `legal` lists are kept, in its order.
    pub fn forecast(&self, seat: DuosSeat) -> CombatForecast {
        let legal = self.legal(seat);
        let mut forecast = combat_forecast(&self.game.duels[seat.duel()], seat.player());
        forecast.attacks.retain(|entry| {
            legal.iter().any(|action| {
                matches!(action, DuosActionBody::Duel {
                    body: ActionBody::Attack { attacker_id, target_id },
                } if *attacker_id == entry.attacker_id && *target_id == entry.target_id)
            })
        });
        forecast
    }

    // Output

    fn flush_timers(&mut self) {
        for command in self.fx.take() {
            self.out.push(DuosHostOutput::Timer { command });
        }
    }

    fn emit(&mut self, output: DuosHostOutput) {
        self.flush_timers();
        self.out.push(output);
    }

    fn take_out(&mut self) -> Vec<DuosHostOutput> {
        self.flush_timers();
        std::mem::take(&mut self.out)
    }

    fn note(&mut self, note: DuosNote) {
        self.emit(DuosHostOutput::Note { note });
    }

    /// Whether the person at `seat` has an open socket.
    fn here(&self, seat: DuosSeat) -> bool {
        human(seat).is_some_and(|index| self.connected[index])
    }

    fn send(&mut self, seat: DuosSeat, message: DuosSeatMessage) {
        if !self.here(seat) {
            return;
        }
        self.emit(DuosHostOutput::Send { seat, message });
    }

    /// B35: view and legal actions.
    fn push_view(&mut self, seat: DuosSeat, now: i64) {
        if !self.here(seat) {
            return;
        }
        let legal = self.legal(seat);
        let view = Box::new(self.view_for(seat, now));
        let forecast = Some(self.forecast(seat));
        self.send(
            seat,
            DuosSeatMessage::View {
                view,
                legal,
                forecast,
            },
        );
    }

    fn push_clock(&mut self, seat: DuosSeat, now: i64) {
        if !self.here(seat) {
            return;
        }
        let grace = |index: usize, host: &DuosHost| host.grace[index].map(|armed| armed.deadline);
        let message = DuosSeatMessage::Clock {
            now,
            round_deadline: self.deadline_for(seat),
            grace_a1: grace(0, self),
            grace_a2: grace(1, self),
        };
        self.send(seat, message);
    }

    /// S6, B37: what `seat` is told about its teammate.
    fn push_teammate(&mut self, seat: DuosSeat) {
        let mate = seat.teammate();
        let message = DuosSeatMessage::Teammate {
            connected: self.here(mate),
            ai_playing: human(mate).is_some_and(|index| self.ai_playing[index]),
        };
        self.send(seat, message);
    }

    /// Everything a seat that (re)joins needs: its view, its clock and its teammate.
    fn push_seat(&mut self, seat: DuosSeat, now: i64) {
        self.push_view(seat, now);
        self.push_clock(seat, now);
        self.push_teammate(seat);
    }

    // Countdowns

    fn arm(&mut self, key: TimerKey, ms: i64, now: i64) -> DuosArmed {
        let at = now + ms;
        DuosArmed {
            deadline: at,
            timer: self.fx.set(key, at),
        }
    }

    fn cancel(&mut self, key: TimerKey, armed: Option<DuosArmed>) {
        if let Some(armed) = armed {
            self.fx.cancel(key, armed.timer);
        }
    }

    /// B37: the person in `seat` is away; the AI takes the seat when the grace runs out.
    fn start_grace(&mut self, seat: DuosSeat, now: i64) {
        let Some(index) = human(seat) else { return };
        if self.grace[index].is_some() || self.ai_playing[index] || self.finished {
            return;
        }
        let armed = self.arm(TimerKey::DuosGrace { seat }, grace_ms(), now);
        self.grace[index] = Some(armed);
    }

    /// Syncs mulligan, round and prompt clocks with the state (armed once per instance).
    fn sync_clocks(&mut self, now: i64) {
        let over = self.finished || self.stopped || self.game.result.is_some();

        if !over && self.game.round == 0 {
            if self.mulligan.is_none() && !self.mulligan_expired {
                let armed = self.arm(TimerKey::DuosMulligan, DUOS_MULLIGAN_CLOCK_MS, now);
                self.mulligan = Some(armed);
            }
        } else {
            let old = self.mulligan.take();
            self.cancel(TimerKey::DuosMulligan, old);
        }

        // B38: a round with a human seat runs the round clock.
        let round = self.game.round;
        let with_people =
            !over && round >= 1 && self.game.round_seats.iter().any(|seat| human(*seat).is_some());
        if with_people {
            let armed = self.round_clock.as_ref().is_some_and(|(held, _)| *held == round);
            if !armed && self.round_expired != Some(round) {
                let old = self.round_clock.take();
                self.cancel(TimerKey::DuosRound, old.map(|(_, armed)| armed));
                let countdown = self.arm(TimerKey::DuosRound, DUOS_ROUND_CLOCK_MS, now);
                self.round_clock = Some((round, countdown));
            }
        } else {
            let old = self.round_clock.take();
            self.cancel(TimerKey::DuosRound, old.map(|(_, armed)| armed));
        }

        // A prompt held outside the person's own round (inside one, the round clock covers it, B38).
        for seat in HUMAN_SEATS {
            let Some(index) = human(seat) else { continue };
            let held = if over { None } else { held_prompt(&self.game, seat) };
            let covered = acting_round_seat(&self.game, seat);
            match held {
                Some(choice) if !covered => {
                    let armed = self.prompt_clocks[index]
                        .as_ref()
                        .is_some_and(|(held, _)| *held == choice);
                    if !armed && self.prompt_expired[index].as_deref() != Some(choice.as_str()) {
                        let old = self.prompt_clocks[index].take();
                        self.cancel(TimerKey::DuosPrompt { seat }, old.map(|(_, armed)| armed));
                        let countdown = self.arm(TimerKey::DuosPrompt { seat }, DUOS_PROMPT_CLOCK_MS, now);
                        self.prompt_clocks[index] = Some((choice, countdown));
                    }
                }
                _ => {
                    let old = self.prompt_clocks[index].take();
                    self.cancel(TimerKey::DuosPrompt { seat }, old.map(|(_, armed)| armed));
                }
            }
        }
    }

    /// Every clock goes: the match is over, or the host is.
    fn drop_clocks(&mut self) {
        let old = self.mulligan.take();
        self.cancel(TimerKey::DuosMulligan, old);
        let old = self.round_clock.take();
        self.cancel(TimerKey::DuosRound, old.map(|(_, armed)| armed));
        for seat in HUMAN_SEATS {
            if let Some(index) = human(seat) {
                let old = self.prompt_clocks[index].take();
                self.cancel(TimerKey::DuosPrompt { seat }, old.map(|(_, armed)| armed));
                let old = self.grace[index].take();
                self.cancel(TimerKey::DuosGrace { seat }, old);
            }
        }
        let old = self.ceiling.take();
        self.cancel(TimerKey::DuosCeiling, old);
    }

    // Arming, and who is here

    fn on_arm(&mut self, now: i64) {
        // R744: a seat with no socket starts grace, so one nobody opens goes to the AI.
        for seat in HUMAN_SEATS {
            if !self.here(seat) {
                self.start_grace(seat, now);
            }
        }
        self.sync_clocks(now);
        // R2881000: the watchers' record starts with the state the host opens on.
        self.emit(DuosHostOutput::Pushed);
        let over = self.game.result.is_some();
        let live = !over && !self.finished;
        let left = ceiling_at(self.created_at) - now;
        if live && left > 0 {
            let armed = self.arm(TimerKey::DuosCeiling, left, now);
            self.ceiling = Some(armed);
        }
        if over {
            self.on_terminal(now);
        } else if live && left <= 0 {
            // R1047000: folded back past its ceiling, the match ends before anyone plays on.
            self.reach_ceiling(now);
        } else {
            self.drive_ai(now);
        }
    }

    fn on_attached(&mut self, seat: DuosSeat, now: i64) {
        let Some(index) = human(seat) else { return };
        let old = self.grace[index].take();
        self.cancel(TimerKey::DuosGrace { seat }, old);
        if std::mem::replace(&mut self.ai_playing[index], false) {
            self.note(DuosNote::SeatBack { seat });
        }
        // B35: the seat's view and legal actions, its clock, and its teammate. R2881000: and the watcher
        // count, which a socket that was away missed.
        self.push_seat(seat, now);
        self.emit(DuosHostOutput::Greeting {
            seat,
            part: GreetingPart::Watchers,
        });
        let mate = seat.teammate();
        self.push_teammate(mate);
        self.push_clock(mate, now);
        self.drive_ai(now);
    }

    fn on_disconnected(&mut self, seat: DuosSeat, now: i64) {
        if self.stopped || self.here(seat) {
            return;
        }
        self.start_grace(seat, now);
        let mate = seat.teammate();
        self.push_teammate(mate);
        self.push_clock(mate, now);
    }

    fn on_stop(&mut self) {
        self.stopped = true;
        self.drop_clocks();
    }

    // The clocks run out

    fn on_timer(&mut self, key: TimerKey, id: u32, now: i64) {
        match key {
            TimerKey::DuosMulligan
            | TimerKey::DuosRound
            | TimerKey::DuosPrompt { .. }
            | TimerKey::DuosGrace { .. } => self.on_expire(key, id, now),
            TimerKey::DuosCeiling => {
                if !self.ceiling.is_some_and(|armed| armed.timer == id) {
                    return;
                }
                self.ceiling = None;
                self.reach_ceiling(now);
            }
            TimerKey::DuosAi { seat } => {
                let index = slot(seat);
                if self.ai_timer[index] != Some(id) {
                    return;
                }
                self.ai_timer[index] = None;
                self.start_job(seat, now);
            }
            TimerKey::DuosAiRetry => {
                if self.ai_retry != Some(id) {
                    return;
                }
                self.ai_retry = None;
                self.drive_ai(now);
            }
            // A 1v1 host's timers are none of a Duos host's.
            _ => {}
        }
    }

    /// An expiry hands the seats it covers to the AI (B37, B38), which plays them through `apply`.
    fn on_expire(&mut self, key: TimerKey, id: u32, now: i64) {
        if self.stopped || self.finished {
            return;
        }
        let kind = match key {
            TimerKey::DuosMulligan => {
                if !self.mulligan.is_some_and(|armed| armed.timer == id) {
                    return;
                }
                self.mulligan = None;
                self.mulligan_expired = true;
                // §13.2, R268: each person still owing keeps the hand they were dealt.
                for seat in HUMAN_SEATS {
                    if owes_mulligan(&self.game, seat) {
                        self.forced[slot(seat)] = Some(DuosForced::KeepHand);
                    }
                }
                "mulligan"
            }
            TimerKey::DuosRound => {
                let Some((round, armed)) = self.round_clock else {
                    return;
                };
                if armed.timer != id {
                    return;
                }
                self.round_clock = None;
                self.round_expired = Some(round);
                if self.game.round == round {
                    let seats: Vec<DuosSeat> = self.game.round_seats.clone();
                    for seat in seats {
                        if acting_round_seat(&self.game, seat) {
                            self.forced[slot(seat)] = Some(DuosForced::FinishRound { round });
                        }
                    }
                }
                "round"
            }
            TimerKey::DuosPrompt { seat } => {
                let Some(index) = human(seat) else { return };
                let Some((choice, armed)) = self.prompt_clocks[index].take() else {
                    return;
                };
                if armed.timer != id {
                    self.prompt_clocks[index] = Some((choice, armed));
                    return;
                }
                self.prompt_expired[index] = Some(choice.clone());
                if held_prompt(&self.game, seat).as_deref() == Some(choice.as_str()) {
                    self.forced[slot(seat)] = Some(DuosForced::AnswerPrompt { choice });
                }
                "prompt"
            }
            TimerKey::DuosGrace { seat } => {
                let Some(index) = human(seat) else { return };
                if !self.grace[index].is_some_and(|armed| armed.timer == id) {
                    return;
                }
                self.grace[index] = None;
                if !self.here(seat) {
                    // B37: the AI plays the seat until its person is back: no loss by disconnect.
                    self.ai_playing[index] = true;
                    self.note(DuosNote::SeatAiPlaying { seat });
                    self.push_teammate(seat.teammate());
                }
                "grace"
            }
            _ => return,
        };
        self.note(DuosNote::ClockExpired {
            kind: kind.to_string(),
        });
        for seat in HUMAN_SEATS {
            self.push_clock(seat, now);
        }
        self.drive_ai(now);
    }

    /// R1047000: `ceilingReached` in duel 0, stamped a1 (R146: no seat's), ends the game as a draw; retried
    /// on a log error (R776000).
    fn reach_ceiling(&mut self, now: i64) {
        if self.stopped || self.finished || self.game.result.is_some() {
            return;
        }
        self.note(DuosNote::ClockExpired {
            kind: "ceiling".to_string(),
        });
        let nonce = format!("{SERVER_NONCE_PREFIX}ceiling-{}", self.next_seq);
        self.apply(
            DuosSeat::A1,
            nonce,
            DuosActionBody::Duel {
                body: ActionBody::CeilingReached,
            },
            DuosOrigin::Ceiling,
            now,
        );
    }

    // Applying an action (B35, B40)

    /// The one path every action takes — a person's and the AI's alike: reduce, ask for one gapless log
    /// row, and on the row's `Logged` push.
    fn apply(&mut self, seat: DuosSeat, nonce: String, body: DuosActionBody, origin: DuosOrigin, now: i64) {
        if let Some(answer) = self.begin_apply(seat, nonce.clone(), body, &origin, now) {
            self.resolve(origin, nonce, answer, now);
        }
    }

    fn begin_apply(
        &mut self,
        seat: DuosSeat,
        nonce: String,
        body: DuosActionBody,
        origin: &DuosOrigin,
        now: i64,
    ) -> Option<ActionAnswer> {
        if let Some(ack) = self.acks.get(&nonce) {
            // §9.3: a reused nonce answers its original ack, with no second reduce or row.
            return Some(if ack.seat == seat {
                ActionAnswer::Ack { seq: ack.seq }
            } else {
                ActionAnswer::NonceTaken
            });
        }
        if self.panicked.contains(&nonce) {
            return Some(ActionAnswer::Panicked);
        }
        if self.finished || self.game.result.is_some() {
            return Some(ActionAnswer::MatchOver);
        }
        // One write at a time: the host feeds `Logged` before the next action.
        if self.awaiting.is_some() {
            return Some(ActionAnswer::NotRecorded);
        }
        // §9.1: the seat is the host's, never the client's.
        let action = DuosAction {
            seat,
            nonce: nonce.clone(),
            body,
        };
        // R1253000: pure `reduce` leaves nothing half-applied on panic, as the fold relies on.
        let reduced = catch_unwind(AssertUnwindSafe(|| duos_reduce(&self.game, &action)));
        let seq = self.next_seq;
        let result = match reduced {
            Ok(result) => result,
            Err(payload) => {
                let message = jackioh_ai::panic_message(&*payload);
                self.panicked.insert(nonce.clone());
                self.emit(DuosHostOutput::Panicked { action, seq, message });
                // A server action would be made and panic again, so it ends the match as a ceiling draw.
                if nonce.starts_with(SERVER_NONCE_PREFIX) {
                    self.end_as_ceiling_draw(now);
                }
                return Some(ActionAnswer::Panicked);
            }
        };
        if let Some(error) = result.error {
            let reason = error.to_string();
            self.note(DuosNote::ActionRejected {
                seat,
                reason: reason.clone(),
            });
            // R835000: count person refusals; AI and ceiling carry server nonces (R270) and never do.
            if human(seat).is_some() && !nonce.starts_with(SERVER_NONCE_PREFIX) {
                self.emit(DuosHostOutput::Refused { seat });
            }
            return Some(ActionAnswer::Illegal { reason });
        }
        // §9.3: row before state, so the log is never behind the state.
        self.emit(DuosHostOutput::Append {
            seq,
            action: action.clone(),
            at: now,
        });
        self.awaiting = Some(DuosAwaiting {
            action,
            after: result.state,
            seq,
            origin: origin.clone(),
        });
        None
    }

    fn on_logged(&mut self, outcome: LogOutcome, now: i64) {
        let Some(awaiting) = self.awaiting.take() else {
            return;
        };
        let DuosAwaiting {
            action,
            after,
            seq,
            origin,
        } = awaiting;
        let nonce = action.nonce.clone();
        let answer = match outcome {
            LogOutcome::Written => {
                self.game = after;
                self.next_seq = seq + 1;
                self.acks.insert(
                    nonce.clone(),
                    DuosAck {
                        nonce: nonce.clone(),
                        seat: action.seat,
                        seq,
                    },
                );
                self.after_change(now);
                ActionAnswer::Ack { seq }
            }
            LogOutcome::Failed | LogOutcome::Diverged => {
                // R776000: the log holds this host's next seq already: it lets go (unless stopped).
                if outcome == LogOutcome::Diverged {
                    self.emit(DuosHostOutput::LetGo {
                        reason: DuosLetGo::Diverged,
                    });
                }
                ActionAnswer::NotRecorded
            }
        };
        self.resolve(origin, nonce, answer, now);
    }

    /// An action has its answer: say it, and let what follows from where it came go on.
    fn resolve(&mut self, origin: DuosOrigin, nonce: String, answer: ActionAnswer, now: i64) {
        self.emit(DuosHostOutput::Answer {
            nonce: nonce.clone(),
            answer: answer.clone(),
        });
        match origin {
            DuosOrigin::Seat => {}
            DuosOrigin::Ceiling => {
                // R776000: a row the store did not take. The ceiling runs out again, and the action is
                // derived again then.
                if answer.is_internal() && !self.stopped && !self.finished {
                    self.note(DuosNote::CeilingRetrying);
                    let old = self.ceiling.take();
                    self.cancel(TimerKey::DuosCeiling, old);
                    let armed = self.arm(TimerKey::DuosCeiling, MATCH_SERVER_ACTION_RETRY_MS, now);
                    self.ceiling = Some(armed);
                }
            }
            DuosOrigin::Ai {
                seat,
                version,
                current,
                mode,
                base,
                retried,
            } => {
                let refused = answer.is_illegal();
                if !retried && refused && current != version {
                    self.drive_ai(now);
                    return;
                }
                if !retried && refused {
                    let retry = if owes(&self.game, seat, &mode) {
                        let legal = duos_legal_actions(&self.game, seat);
                        without_search(&mode, &self.game, seat, &legal)
                    } else {
                        None
                    };
                    if let Some(body) = retry {
                        let origin = DuosOrigin::Ai {
                            seat,
                            version,
                            current,
                            mode,
                            base: base.clone(),
                            retried: true,
                        };
                        self.apply(seat, format!("{base}-r"), body, origin, now);
                        return;
                    }
                }
                if matches!(answer, ActionAnswer::Ack { .. }) {
                    return;
                }
                self.note(DuosNote::AiRefused {
                    seat,
                    answer: answer.clone(),
                });
                match answer {
                    ActionAnswer::MatchOver => {}
                    // A deleted match row takes no more rows: the host lets go rather than retrying for
                    // ever, which the store says.
                    ActionAnswer::Panicked | ActionAnswer::NotRecorded => self.emit(DuosHostOutput::CheckRow),
                    _ => self.drive_ai_after(DUOS_AI_STEP_DELAY_MS, now),
                }
            }
        }
    }

    fn on_row_checked(&mut self, exists: bool, now: i64) {
        if exists {
            self.drive_ai_after(BOT_WRITE_RETRY_MS, now);
        } else {
            self.emit(DuosHostOutput::LetGo {
                reason: DuosLetGo::MatchGone,
            });
        }
    }

    /// B35: pushes fresh views and clocks after every action, then writes the result or moves the AI.
    fn after_change(&mut self, now: i64) {
        self.sync_clocks(now);
        for seat in HUMAN_SEATS {
            self.push_view(seat, now);
            self.push_clock(seat, now);
        }
        // R2881000: the watchers' record of the change, sent SPECTATE_DELAY_MS on.
        self.emit(DuosHostOutput::Pushed);
        if self.game.result.is_some() {
            self.on_terminal(now);
        } else {
            self.drive_ai(now);
        }
    }

    /// B40: the result and `ended_at`, once; a lost write lets go, for a rebuild to write it.
    fn on_terminal(&mut self, now: i64) {
        if self.finished {
            return;
        }
        let Some(result) = self.game.result.clone() else {
            return;
        };
        self.finished = true;
        self.finished_at = Some(now);
        self.drop_clocks();
        self.forced = [None, None, None, None];
        self.emit(DuosHostOutput::Record { result, at: now });
    }

    /// R1253000, R1047001: ends the match as a ceiling draw (B40) after a server action's `reduce`
    /// panicked.
    fn end_as_ceiling_draw(&mut self, now: i64) {
        if self.finished || self.stopped {
            return;
        }
        self.finished = true;
        self.finished_at = Some(now);
        self.emit(DuosHostOutput::CeilingDraw { at: now });
    }

    // The AI's seats (B36 to B38)

    fn ai_mode(&self, seat: DuosSeat) -> Option<DuosAiMode> {
        let Some(index) = human(seat) else {
            return Some(DuosAiMode::Full);
        };
        if self.ai_playing[index] {
            return Some(DuosAiMode::Full);
        }
        match &self.forced[slot(seat)] {
            Some(DuosForced::KeepHand) if owes_mulligan(&self.game, seat) => Some(DuosAiMode::KeepHand),
            Some(DuosForced::FinishRound { round }) if *round == self.game.round => {
                Some(DuosAiMode::FinishRound)
            }
            Some(DuosForced::AnswerPrompt { choice })
                if held_prompt(&self.game, seat).as_deref() == Some(choice.as_str()) =>
            {
                Some(DuosAiMode::AnswerPrompt {
                    choice: choice.clone(),
                })
            }
            _ => None,
        }
    }

    /// B36: starts the pause before every AI seat that owes a move acts (`DUOS_AI_STEP_DELAY_MS`).
    fn drive_ai(&mut self, now: i64) {
        if self.finished || self.stopped || self.game.result.is_some() {
            return;
        }
        for seat in DUOS_SEATS {
            let index = slot(seat);
            if self.ai_busy[index] {
                continue;
            }
            let Some(mode) = self.ai_mode(seat) else {
                continue;
            };
            if !owes(&self.game, seat, &mode) {
                continue;
            }
            self.ai_busy[index] = true;
            let at = now + DUOS_AI_STEP_DELAY_MS;
            self.ai_timer[index] = Some(self.fx.set(TimerKey::DuosAi { seat }, at));
        }
    }

    /// `drive_ai` once `ms` have passed: an AI seat whose move could not be written decides again at that
    /// pace.
    fn drive_ai_after(&mut self, ms: i64, now: i64) {
        if self.ai_retry.is_none() {
            self.ai_retry = Some(self.fx.set(TimerKey::DuosAiRetry, now + ms.max(0)));
        }
    }

    /// The pause is over: the decision `seat` owes, on the state as it stands now and the log's next seq.
    /// A mulligan kept whole, or a turn simply ended, needs no search and is played at once; the rest asks
    /// the host for the AI's.
    fn start_job(&mut self, seat: DuosSeat, now: i64) {
        let index = slot(seat);
        // A match that ended, or a host that stopped, while the seat paused has nothing to decide.
        if self.finished || self.stopped {
            self.ai_busy[index] = false;
            return;
        }
        let mode = self.ai_mode(seat);
        let version = self.next_seq;
        let cursor = self.ai_cursor[index];
        let Some(mode_now) = mode.clone() else {
            return self.on_ai_move(seat, version, cursor, None, None, now);
        };
        if !owes(&self.game, seat, &mode_now) {
            return self.on_ai_move(seat, version, cursor, mode, None, now);
        }
        let legal = duos_legal_actions(&self.game, seat);
        match &mode_now {
            DuosAiMode::KeepHand => {
                let body = keep_whole_hand(&legal);
                self.on_ai_move(seat, version, cursor, mode, body, now);
            }
            DuosAiMode::FinishRound if !owes_answer(&self.game, seat) => {
                let body = legal.iter().find(|body| is_end_turn(body)).cloned();
                self.on_ai_move(seat, version, cursor, mode, body, now);
            }
            _ => {
                self.ai_jobs[index] = Some(DuosJob {
                    version,
                    mode: mode_now.clone(),
                    fallback: fallback(&legal),
                    cursor,
                });
                self.emit(DuosHostOutput::Decide {
                    request: DuosDecide {
                        seat,
                        mode: mode_now,
                        version,
                        cursor,
                    },
                });
            }
        }
    }

    /// The AI's search came back (or threw): what it found, or the job's fallback, is played.
    fn on_decided(
        &mut self,
        seat: DuosSeat,
        body: Option<DuosActionBody>,
        cursor: u32,
        failed: bool,
        now: i64,
    ) {
        let index = slot(seat);
        let Some(job) = self.ai_jobs[index].take() else {
            return;
        };
        let (body, cursor) = if failed {
            (None, job.cursor)
        } else {
            (body.filter(|body| !ai_forbidden(body)).or(job.fallback), cursor)
        };
        self.on_ai_move(seat, job.version, cursor, Some(job.mode), body, now);
    }

    /// Plays the AI's move for `seat` if it is still in the mode it was decided in, else decides again: a
    /// person back is never played for (B37, B38). A refused move decides again (R1452), a fresh one falls
    /// back once; a failed write is decided again after `BOT_WRITE_RETRY_MS`.
    fn on_ai_move(
        &mut self,
        seat: DuosSeat,
        version: i64,
        cursor: u32,
        decided: Option<DuosAiMode>,
        body: Option<DuosActionBody>,
        now: i64,
    ) {
        let index = slot(seat);
        self.ai_busy[index] = false;
        self.ai_cursor[index] = cursor;
        if self.finished || self.stopped {
            return;
        }
        let held = self.ai_mode(seat);
        let current = self.next_seq;
        let Some(mode) = held.filter(|mode| decided.as_ref() == Some(mode)) else {
            self.drive_ai(now);
            return;
        };
        let Some(body) = body else {
            // Nothing to play on the state it read; a newer state may owe something.
            if current != version {
                self.drive_ai(now);
            }
            return;
        };
        let nonce = format!("{SERVER_NONCE_PREFIX}ai-{current}");
        let origin = DuosOrigin::Ai {
            seat,
            version,
            current,
            mode,
            base: nonce.clone(),
            retried: false,
        };
        self.apply(seat, nonce, body, origin, now);
    }
}
