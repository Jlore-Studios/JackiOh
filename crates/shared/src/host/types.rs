//! What crosses the host machine's boundary: the events a host feeds it, the outputs it carries out,
//! and the messages a seat is sent (the host contract, `crates/shared/README.md`). Every type is serde
//! and, behind `ts`, ts-rs, so a host in TypeScript or Luau reads the same shapes.

use serde::{Deserialize, Serialize};

use jackioh_engine::{
    Action, ActionBody, Aim, EmoteId, GameEventType, GameResult, LastBoardEntry, PlayerId, PlayerView,
    PromptKind,
};

use crate::bots::decide::BotChoice;

use super::clock::{ClockExpiry, MatchClocks};
use super::rematch::RematchStakes;
use super::snapshot::MatchSnapshot;
use super::timer::{TimerCommand, TimerKey};

/// R270: the prefix of every nonce the host mints for an action of its own (a clock's, a bot's, an
/// emote's). No client may send one, so none can pre-empt it.
pub const SERVER_NONCE_PREFIX: &str = "srv-";

/// Who an action is for.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "by", rename_all = "camelCase")]
pub enum ActionBy {
    /// The account that began the match in `home`: the seat it plays is read when the action runs
    /// (R677), so a swap queued ahead of it has already moved the account.
    Home { home: PlayerId },
    /// The engine seat itself (the host's own submit).
    Seat { seat: PlayerId },
}

/// How an action came out: the answer its sender is given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "answer", rename_all = "camelCase")]
pub enum ActionAnswer {
    /// §9.3: the nonce was accepted (or had been) and the log holds it at `seq`.
    Ack {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seq: i64,
    },
    /// §9.3: the reducer refused the action; `reason` is its message, relayed verbatim.
    Illegal { reason: String },
    /// The match already has a result; no further action will be applied.
    MatchOver,
    /// R1253000: `reduce` panicked on the action, now and on every resend of its nonce.
    Panicked,
    /// The log did not take the row: the action was not applied, and may be sent again.
    NotRecorded,
}

impl ActionAnswer {
    /// Whether this is the `internal` family of answers: the host failed, not the action.
    pub fn is_internal(&self) -> bool {
        matches!(self, ActionAnswer::Panicked | ActionAnswer::NotRecorded)
    }

    /// Whether the answer is a refusal of the action itself.
    pub fn is_illegal(&self) -> bool {
        matches!(self, ActionAnswer::Illegal { .. })
    }
}

/// What a seat is sent. The host encodes it for its own transport; a seat's `View` holds `view_for` of
/// that seat and nothing else (CLAUDE.md rule 7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "message", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SeatMessage {
    /// §10.8: the seat's view, with its clock filled in (R79), and its own legal actions.
    View {
        view: Box<PlayerView>,
        legal: Vec<ActionBody>,
    },
    /// §9.5: the deadlines both clients render; `now` is the host's clock at send time.
    Clock {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
        clocks: MatchClocks,
    },
    /// §10.6: the prompt the seat must answer.
    PromptForYou {
        player: PlayerId,
        choice_id: String,
        kind: PromptKind,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        deadline: Option<i64>,
    },
    /// §10.6: that a prompt is open for the other seat, and nothing about it.
    PromptForOpponent {
        pending_for: PlayerId,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        deadline: Option<i64>,
    },
    /// R738: the opponent's aim, relayed — never to the sender.
    Aim { from: PlayerId, aim: Option<Aim> },
    /// R2739001: the option the opponent's open card picker hovers, relayed — never to the sender.
    ChoiceHover { from: PlayerId, index: Option<i32> },
}

/// Where in a seat's greeting the host adds what is its own (loadouts before the view, portraits after
/// the clock, the watchers' count last): the machine orders the greeting, the host composes these.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum GreetingPart {
    Before,
    After,
    Watchers,
}

/// Something worth a line in the host's log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "note", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostNote {
    /// A clock ran out and the host acts for the seat.
    ClockExpired { kind: String, player: PlayerId },
    /// The action of a clock that ran out could not be logged: the clock runs out again in `in_ms`.
    ClockRetrying {
        kind: String,
        player: PlayerId,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        in_ms: i64,
    },
    /// §9.8: every rejected action is logged with its reason.
    ActionRejected {
        player: PlayerId,
        action_type: String,
        reason: String,
    },
    /// A bot's move was refused or could not be written.
    BotMoveRefused { player: PlayerId, answer: ActionAnswer },
}

/// The host must decide a move for the AI or ladder bot at `home`, and answer with
/// [`DuelEvent::Decided`]: one deadline-bounded step, apart from every other (`DuelHost::job` makes the
/// job to run, `bots::decide` is what it runs). The machine paces the move it is told: its think time,
/// its aim, the moment it is played.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DecideRequest {
    /// The account's home seat, which the bot's streams are keyed by.
    pub home: PlayerId,
    /// The engine seat it plays now (R677).
    pub seat: PlayerId,
    /// The log's next seq when the request was made: a move decided on an older state is stale.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub version: i64,
    pub turn: i32,
    /// Whether this is the bot's first move of the turn.
    pub first_in_turn: bool,
    /// R639000: whether this is the first move of the bot's own turn, the one a concession comes at.
    pub own_turn_start: bool,
    /// The acting clock the bot has left (R79), or none when none runs for it.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub clock_left_ms: Option<i64>,
    /// R1452: the most wall-clock time the decision may take (`BOT_DECISION_DEADLINE_MS`, and never so
    /// long that less than `BOT_CLOCK_MARGIN_MS` is left on the clock), past which it plays the best line
    /// found so far. A host with no clock may ignore it: the search ends at the strength's budget.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub deadline_ms: i64,
    /// Where the bot's search stream stands.
    pub ai_cursor: u32,
}

/// What the store must record when the game ends (SPEC §9.5: every ending records a result and clears
/// both players' in-match state). The host knows who the accounts are; the machine says which of them
/// each engine seat is credited to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RecordRequest {
    pub outcome: GameResult,
    pub turns: i32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
    /// R677: whether the accounts hold each other's seat, so engine seat p1 is credited to the account
    /// that began in p2 (its win, its rating, its deck's last board).
    pub seats_swapped: bool,
    /// R417, R565: each seat's board as the game ended, from its own side, in engine seat order.
    pub last_boards: (Vec<LastBoardEntry>, Vec<LastBoardEntry>),
    /// R768: the final state's hash, which the match's replay is held to.
    pub final_hash: String,
}

/// Why a host should let the match go.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum LetGo {
    /// R776000: the log holds the host's next seq already; a rebuilt host takes over.
    Diverged,
}

/// An event the host feeds the machine. Events that stand for one fact the host learns at once
/// (`Connected`, `Left`, `Aim`, `Hover`) may be fed the moment it is learned; the rest are the host's
/// turns, one at a time, in order (the machine awaits the log between an action and its `Logged`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "event", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuelEvent {
    /// The host is up (a new one, or one made from a checkpoint): the clocks are brought up to date with
    /// the game, and a game already over is ended.
    Arm,
    /// The account that began in `home` has a socket (R744: it has now been here).
    Connected { home: PlayerId },
    /// Its greeting: the fresh full view, never a log replay (§9.5), and its disconnect grace ends.
    Attached { home: PlayerId },
    /// A hello from a socket already attached: the fresh full view again.
    Hello { home: PlayerId },
    /// The account's socket is gone: its aim and choice hover end with it.
    Left { home: PlayerId },
    /// Its grace begins (§9.5), and the opponent is told.
    Disconnected { home: PlayerId },
    /// An action for a seat, a client's, the host's own or an emote's.
    Action {
        by: ActionBy,
        nonce: String,
        body: ActionBody,
    },
    /// MD-D29, R1127: an emote that passed the rate gate, minted as an `Emote` action if a card hears it.
    Emote {
        home: PlayerId,
        emote: jackioh_engine::EmoteId,
    },
    /// The log took (or refused) the row [`DuelOutput::Append`] asked for.
    Logged { outcome: LogOutcome },
    /// A timer the machine armed came due.
    TimerFired { key: TimerKey, id: u32 },
    /// R738: the account's aim, or its end.
    Aim { home: PlayerId, aim: Option<Aim> },
    /// R2739001: the account's choice hover, or its end.
    Hover { home: PlayerId, index: Option<i32> },
    /// A bot's move, decided as [`DuelOutput::Decide`] asked (`bots::decide`).
    Decided {
        home: PlayerId,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        version: i64,
        turn: i32,
        choice: BotChoice,
    },
    /// R1351000: the two accounts block each other, or one does: from now on a person's emote is
    /// answered by no persona (the host relays none either), though a card that hears emotes still hears it.
    EmotesMuted,
    /// The result [`DuelOutput::Record`] asked for has been written.
    ResultRecorded,
    /// A rematch offer the account at `from` made reached the host (R639000: a bot may meet it).
    RematchOffered { from: PlayerId, stakes: RematchStakes },
    /// A rematch offer the bot at `home` was asked to make has been sent.
    RematchSent { home: PlayerId },
    /// The host is going away: its clocks and relays stop.
    Stop,
}

/// What the log did with a row.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum LogOutcome {
    Written,
    /// The row was not taken; the action was not applied.
    Failed,
    /// R776000: the log holds this seq already: another host has taken the match over.
    Diverged,
}

/// An output the host carries out, in the order given.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "output", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuelOutput {
    /// A message for the account that began in `home`.
    Send { home: PlayerId, message: SeatMessage },
    /// Set or take back a timer.
    Timer { command: TimerCommand },
    /// §9.5: the clocks changed; the store keeps them so both clients show them after a restart.
    PersistClocks { clocks: MatchClocks },
    /// §9.3: write this row at `seq` (row before state), then feed [`DuelEvent::Logged`].
    Append {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seq: i64,
        action: Action,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// The answer to an action the machine took, by its nonce.
    Answer { nonce: String, answer: ActionAnswer },
    /// A seat's greeting is at this part: the host adds its own messages here.
    Greeting { home: PlayerId, part: GreetingPart },
    /// A reducer refusal for the account at `home`, which an abuse guard counts (R835000).
    Refused { home: PlayerId },
    /// R1253000: `reduce` panicked on this action; the host reports it.
    Panicked {
        action: Action,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seq: i64,
        message: String,
    },
    /// An action was applied and logged: what play telemetry (R1442) reads. `before` is the state it
    /// was made on; it is for a host in the same process, and is not serialized.
    Applied {
        #[serde(skip)]
        #[cfg_attr(feature = "ts", ts(skip))]
        before: Option<Box<jackioh_engine::GameState>>,
        action: Action,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seq: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
        /// The acting seat's clock as the change landed.
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        clock_left_ms: Option<i64>,
        last_event: Option<GameEventType>,
    },
    /// R677: a Glitch swapped the seats; a bot's persona reads the seat it plays now.
    SeatsSwapped { swapped: bool },
    /// The change produced `fresh` events, which a bot's persona reads (R645).
    Changed { fresh: usize },
    /// Every view, prompt and clock of a change has been sent: the point a think time starts from, and
    /// where watchers' views are recorded.
    Pushed { snapshot: MatchSnapshot },
    /// Decide a move for a seat the host plays.
    Decide { request: DecideRequest },
    /// The game is over: record it. Feed [`DuelEvent::ResultRecorded`] once written.
    Record { request: RecordRequest },
    /// R679: a Glitch voided the match: no result, no ratings, no records. The host closes its
    /// sockets and forgets the match.
    Voided {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// R1253000, R112: a clock action's `reduce` panicked: the match ends as a ceiling draw with turns
    /// 0 and ratings unchanged. Record it, then let the match go.
    CeilingDraw {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// R645: the bot at `home` sends an emote of its persona. The host sends it as it sends a person's
    /// (the hand check, the rate gate (R643), the relay to the opponent, telemetry) and feeds
    /// [`DuelEvent::Emote`] if it let it through, so a card that hears emotes hears the bot's too (R1127).
    BotEmote { home: PlayerId, emote: EmoteId },
    /// R639000: the bot at `home` makes a rematch offer at `stakes` (`reply`: in answer to a person's).
    /// A reply is awaited: feed [`DuelEvent::RematchSent`] once it has been made, so the bot stays on the
    /// match until then.
    OfferRematch {
        home: PlayerId,
        stakes: RematchStakes,
        reply: bool,
    },
    /// Let the match go: forget it and stop.
    LetGo { reason: LetGo },
    /// A line for the host's log.
    Note { note: HostNote },
}

/// A clock expiry and what is left of its server actions: R268's mulligan clock times out every seat
/// still owing, in seat order, one after the other.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryRun {
    pub expiry: ClockExpiry,
    pub queue: Vec<(PlayerId, ActionBody)>,
}
