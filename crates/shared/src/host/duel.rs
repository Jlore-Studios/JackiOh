//! The 1v1 match host's decisions as one pure state machine (#3101, #2956 Part B; SPEC §9.2, §9.3, §9.5).
//!
//! [`DuelHost`] is the state a host holds: the `GameState` and everything the host decides around it
//! (the clocks, who is here and who is away, the aim and choice-hover relays, the log's bookkeeping, a
//! bot's turn, the post-game linger). [`step`] takes it, a [`DuelEvent`] and the time, and hands back
//! the state and the [`DuelOutput`]s the host carries out, in order: a seat's message, a timer to set or
//! take back, a row to append to the log, a move to decide for a seat the host plays, the result to
//! record. Nothing here does I/O, reads a clock or draws a random number the state does not hold: a
//! host that restarts resumes from its state as JSON (`crates/shared/README.md`, the host contract).
//!
//! What the machine decides, and what stays the host's:
//!
//!  - which clock runs and what an expiry becomes (R79, R265, R268, R1563006, R776000) and the grace a
//!    disconnect starts (§9.5, R144, R147, R744), the mulligan and prompt frames a change sends (R265,
//!    R266, §10.6), the seats' swap (R677), the relays of an aim (R738) and a choice hover (R2739001),
//!    the order every view and clock goes out in, what a game's end records (R677, R679, R417, R768),
//!    and a ladder bot (R1452, R639000, R645, #3103): when it owes a move, how long it thinks, when it
//!    shows its aim and plays, what it emotes, whether it lets a clock run out, and what it does after a
//!    ranked game. A bot that passes for a person (`BotSeat.hidden`) does all of it; one that does not
//!    plays as the practice AI does;
//!  - the host owns its sockets, its store, its abuse guards (flood, malformed frames, hello interval),
//!    its emote gate, its watchers, its telemetry and the AI's search, which it runs when the machine
//!    asks (`DuelOutput::Decide`, `DuelHost::job`) and answers with `DuelEvent::Decided`.
//!
//! The machine reduces under whatever catalog the host has set (R388, R767000): it calls the engine as
//! the engine is called, and a host with a pinned catalog runs `step` inside it.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use jackioh_engine::{
    Action, ActionBody, ActionType, Aim, EmoteId, GameOverReason, GameState, PLAYER_IDS, PendingView,
    PerPlayer, PlayerId, PlayerView, Rng, aim_key,
};

use super::bot::{BotSeat, DecideJob, RematchPlan, fallback_action, never_played};
use super::clock::{ClockExpiry, MatchClock, MatchClocks};
use super::relay::{AimSlot, HoverSlot, aim_is_public, hover_is_shown};
use super::rematch::RematchStakes;
use super::snapshot::{MatchSnapshot, clock_view_for, mulligan_key, mulligan_window, snapshot};
use super::timer::{Fx, TimerCommand, TimerKey};
use super::types::{
    ActionAnswer, ActionBy, DecideRequest, DuelEvent, DuelOutput, ExpiryRun, GreetingPart, HostNote, LetGo,
    LogOutcome, RecordRequest, SERVER_NONCE_PREFIX, SeatMessage,
};
use crate::bots::decide::BotChoice;
use crate::bots::persona::{AiEmote, EmotePersona};
use crate::bots::signals::{aim_lead_ms, idles_turn, rematch_plan};
use crate::bots::think::{THINK_MODEL, ThinkFor, decision_deadline_ms, think_ms};
use crate::config::{
    AI_EMOTE, AIM_RELAY_INTERVAL_MS, BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT, BOT_REMATCH_FIT,
    BOT_STRENGTHS, BOT_TIMEOUT_FIT, BOT_WRITE_RETRY_MS, MATCH_SERVER_ACTION_RETRY_MS, STAKE_NORMAL,
};

/// What a host makes a [`DuelHost`] from: the game as the host holds it (the opening a registry began, or
/// the fold a rebuild ran) and what the match row and its log say.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuelSetup {
    pub match_id: String,
    /// The match seed, which a bot's streams are drawn from (R639000).
    pub seed: String,
    /// Epoch ms the match started: the ceiling is measured from here (R79).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub created_at: i64,
    pub game: GameState,
    /// §9.5: whether the match is ranked (a ranked one offers a bot's rematch and a double-or-nothing).
    pub ranked: bool,
    /// R641000: the match is a game of a Conquest series, which offers no rematch (R672).
    pub series_game: bool,
    /// Whether the store already holds the result (a rebuilt host on a finished match).
    pub finished: bool,
    /// The log's next seq.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub next_seq: i64,
    /// The nonce and seq of every row already in the log, in log order (§9.3).
    #[cfg_attr(feature = "ts", ts(type = "Array<[string, number]>"))]
    pub acked: Vec<(String, i64)>,
    /// The clocks as the store holds them: only the grace deadlines are read back (R744).
    pub clocks: MatchClocks,
    /// R1452: the ladder bot at each home seat, which the host plays; `None` for a person.
    pub bots: PerPlayer<Option<BotSeat>>,
    /// R639000: by home seat, the turn each bot last moved in.
    pub bot_moved_turn: PerPlayer<Option<i32>>,
}

/// Where a log write stands: the action reduced, the row asked for, the state not yet moved (§9.3: row
/// before state, so a failed write never leaves the state ahead of the log).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
struct Awaiting {
    action: Action,
    after: GameState,
    fresh: usize,
    last_event: Option<jackioh_engine::GameEventType>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    seq: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    at: i64,
    origin: Origin,
}

/// Who an applied action is for, so the right thing follows its answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "origin", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Origin {
    /// A seat's own, the host's submit or an emote: the answer is all that follows.
    Seat,
    /// A clock's server action (R79) for `player`: the run it belongs to goes on, or the clock runs out
    /// again.
    Clock { expiry: ClockExpiry, player: PlayerId },
    /// A bot's move (R1452): a refused one falls back once; a failed one is decided again.
    Bot {
        home: PlayerId,
        stale: bool,
        previous_turn: Option<i32>,
        base: String,
        retried: bool,
        aimed: bool,
    },
}

/// A bot's plan for the end of a ranked match and when its linger on the match ends (R639000).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
struct Linger {
    plan: RematchPlan,
    /// Epoch ms.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    until: i64,
}

/// When the host was asked for a bot's move, and what it was asked on: the time the decision takes counts
/// toward the think time (R1452).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Asked {
    at: i64,
    first_in_turn: bool,
    clock_left_ms: Option<i64>,
}

/// A bot's decided move, waiting out its think time (R1452, R639000): the aim it shows a beat before and
/// the timer that plays it.
#[derive(Clone, Debug, PartialEq)]
struct Waiting {
    version: i64,
    turn: i32,
    body: Option<ActionBody>,
    /// The aim not yet shown, and the timer that shows it.
    aim: Option<Aim>,
    aim_timer: Option<u32>,
    /// Whether the aim is up, which the move takes down.
    aimed: bool,
    /// The timer that plays the move.
    move_timer: u32,
}

/// An emote a bot's persona rolled, due at `at` (epoch ms): its delay is waited out (R645).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
struct DueEmote {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    at: i64,
    emote: EmoteId,
}

/// The 1v1 host's state: the checkpoint. Serializable, with the `GameState` inside it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuelHost {
    match_id: String,
    seed: String,
    game: GameState,
    ranked: bool,
    series_game: bool,
    /// R1452: the ladder bot at each home seat.
    bots: PerPlayer<Option<BotSeat>>,

    // The log (§9.3)
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    next_seq: i64,
    /// Each nonce's seq, so a resend writes no second row.
    #[cfg_attr(feature = "ts", ts(type = "{ [nonce in string]?: number }"))]
    acks: IndexMap<String, i64>,
    /// R1253000: nonces whose reduce panicked, answered `Panicked` again.
    panicked: IndexSet<String>,
    awaiting: Option<Awaiting>,
    run: Option<ExpiryRun>,

    // Who is here (§9.5, R744)
    connected: PerPlayer<bool>,
    attached_once: PerPlayer<bool>,
    /// R744: stored grace deadlines until the first attach; clock writes keep them meanwhile.
    #[cfg_attr(feature = "ts", ts(type = "{ p1: number | null, p2: number | null } | null"))]
    stored_grace: Option<PerPlayer<Option<i64>>>,

    // The flow of a change
    /// R677: whether Glitch swapped the seats; only the engine's state sets it.
    swapped: bool,
    last_pending_for: Option<PlayerId>,
    /// R265: which seats owed a mulligan at the last push, so a `prompt` frame goes out on a change.
    last_mulligan_owed: String,

    // The life of the match
    finished: bool,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    finished_at: Option<i64>,
    voided: bool,
    stopped: bool,

    // The clocks (R79)
    clock: MatchClock,
    persisted_clocks: MatchClocks,
    fx: Fx,

    // The relays (R738, R2739001)
    aims: PerPlayer<AimSlot>,
    hovers: PerPlayer<HoverSlot>,

    // A bot's turn (R1452). What is in flight (a search the host runs, a think time its timers wait out) is
    // not the checkpoint's: a host made from one asks again, and its timers are its own.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    bot_thinking: PerPlayer<bool>,
    /// Where each bot's two random streams stand: the search's and the think time's.
    bot_cursors: PerPlayer<(u32, u32)>,
    bot_moved_turn: PerPlayer<Option<i32>>,
    bot_retry: Option<u32>,
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    bot_asked: PerPlayer<Option<Asked>>,
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    bot_waiting: PerPlayer<Option<Waiting>>,

    // A bot's chatter (R645, R1344): its persona, the view the persona read last, where its stream stands,
    // the emotes it rolled and the timers that wait them out.
    bot_personas: PerPlayer<Option<EmotePersona>>,
    bot_persona_cursor: PerPlayer<u32>,
    bot_views: PerPlayer<Option<PlayerView>>,
    bot_emotes: PerPlayer<Vec<DueEmote>>,
    bot_emote_timer: PerPlayer<Option<u32>>,
    bot_long_turn_timer: PerPlayer<Option<u32>>,
    /// R1351000: no persona answers a person's emote.
    emotes_muted: bool,

    // The post-game linger (R639000)
    /// Each bot home's plan for the end of a ranked match, and when its linger ends.
    bot_rematch: PerPlayer<Option<Linger>>,
    bot_rematch_answered: PerPlayer<bool>,
    bot_rematch_replying: PerPlayer<bool>,
    bot_offer_timer: PerPlayer<Option<u32>>,
    /// R3213000: the timer that marks the end of each bot home's linger, when its absence is told.
    #[serde(default)]
    bot_leave_timer: PerPlayer<Option<u32>>,
    bot_reply_timer: PerPlayer<Option<u32>>,
    bot_reply_stakes: PerPlayer<RematchStakes>,

    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    out: Vec<DuelOutput>,
}

/// The machine's one function: `host` and `event` in, the next `host` and its outputs out.
pub fn step(mut host: DuelHost, event: DuelEvent, now: i64) -> (DuelHost, Vec<DuelOutput>) {
    let outputs = host.step(event, now);
    (host, outputs)
}

impl DuelHost {
    /// A host for a new or rebuilt match, and the timers it arms at once: the ceiling (R79) and the clocks
    /// the opening game calls for, so callers see them armed before anything is fed.
    pub fn new(setup: DuelSetup, now: i64) -> (DuelHost, Vec<DuelOutput>) {
        let DuelSetup {
            match_id,
            seed,
            created_at,
            game,
            ranked,
            series_game,
            finished,
            next_seq,
            acked,
            clocks,
            bots,
            bot_moved_turn,
        } = setup;
        let opening = snapshot(&game);
        let mut host = DuelHost {
            match_id,
            seed,
            ranked,
            series_game,
            // R1452: a bot's seat is never away, so R744's grace never starts for it.
            attached_once: PerPlayer::new(bots.p1.is_some(), bots.p2.is_some()),
            bots,
            next_seq,
            acks: acked.into_iter().collect(),
            panicked: IndexSet::new(),
            awaiting: None,
            run: None,
            connected: PerPlayer::new(false, false),
            stored_grace: Some(clocks.grace_deadline.clone()),
            swapped: opening.seats_swapped,
            last_pending_for: opening.pending_for,
            last_mulligan_owed: mulligan_key(&mulligan_window(&opening)),
            finished,
            finished_at: finished.then_some(now),
            voided: false,
            stopped: false,
            clock: MatchClock::new(created_at),
            persisted_clocks: clocks,
            fx: Fx::default(),
            aims: PerPlayer::default(),
            hovers: PerPlayer::default(),
            bot_thinking: PerPlayer::new(false, false),
            bot_cursors: PerPlayer::new((0, 0), (0, 0)),
            bot_moved_turn,
            bot_retry: None,
            bot_asked: PerPlayer::new(None, None),
            bot_waiting: PerPlayer::new(None, None),
            bot_personas: PerPlayer::new(None, None),
            bot_persona_cursor: PerPlayer::new(0, 0),
            bot_views: PerPlayer::new(None, None),
            bot_emotes: PerPlayer::new(Vec::new(), Vec::new()),
            bot_emote_timer: PerPlayer::new(None, None),
            bot_long_turn_timer: PerPlayer::new(None, None),
            emotes_muted: false,
            bot_rematch: PerPlayer::new(None, None),
            bot_rematch_answered: PerPlayer::new(false, false),
            bot_rematch_replying: PerPlayer::new(false, false),
            bot_offer_timer: PerPlayer::new(None, None),
            bot_leave_timer: PerPlayer::new(None, None),
            bot_reply_timer: PerPlayer::new(None, None),
            bot_reply_stakes: PerPlayer::new(STAKE_NORMAL, STAKE_NORMAL),
            game,
            out: Vec::new(),
        };
        // R645, R1344: a bot that passes for a person emotes by the persona it was dealt, within the hand
        // dealt to its account, and reads the view of the seat it plays (R677: a rebuilt host's bot may
        // already play the other seat).
        for home in PLAYER_IDS {
            let Some(bot) = host.bots[home].filter(|bot| bot.hidden) else {
                continue;
            };
            let seat = host.playing(home);
            host.bot_personas[home] = Some(
                EmotePersona::new(
                    bot.persona,
                    seat,
                    Some(jackioh_engine::deal_emote_hand(&host.seed, home)),
                )
                .with_delays(BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT),
            );
            host.bot_views[home] = Some(jackioh_engine::view_for(&host.game, seat));
        }
        // R79: the ceiling runs from the match's start, and the turn clock when the match opens.
        host.clock.arm_ceiling(now, &mut host.fx);
        host.clock.sync(&clock_view_for(&opening), now, &mut host.fx);
        let outputs = host.take_out();
        (host, outputs)
    }

    /// One event in, its outputs out.
    pub fn step(&mut self, event: DuelEvent, now: i64) -> Vec<DuelOutput> {
        match event {
            DuelEvent::Arm => self.on_arm(now),
            DuelEvent::Connected { home } => {
                self.connected[home] = true;
                self.attached_once[home] = true;
            }
            DuelEvent::Attached { home } => self.on_attached(home, now),
            DuelEvent::Hello { home } => {
                let player = self.playing(home);
                self.greet(home, player, now);
            }
            DuelEvent::Left { home } => self.on_left(home),
            DuelEvent::Disconnected { home } => self.on_disconnected(home, now),
            DuelEvent::Action { by, nonce, body } => {
                let player = match by {
                    // R677: the seat is the one this account plays when the action runs.
                    ActionBy::Home { home } => self.playing(home),
                    ActionBy::Seat { seat } => seat,
                };
                self.apply(player, nonce, body, Origin::Seat, now);
            }
            DuelEvent::Emote { home, emote } => self.on_emote(home, emote, now),
            DuelEvent::Logged { outcome } => self.on_logged(outcome, now),
            DuelEvent::TimerFired { key, id } => self.on_timer(key, id, now),
            DuelEvent::Aim { home, aim } => {
                let player = self.playing(home);
                self.receive_aim(player, aim, now);
            }
            DuelEvent::Hover { home, index } => {
                let player = self.playing(home);
                self.receive_hover(player, index, now);
            }
            DuelEvent::Decided {
                home,
                version,
                turn,
                choice,
            } => self.on_decided(home, version, turn, choice, now),
            DuelEvent::EmotesMuted => self.emotes_muted = true,
            DuelEvent::ResultRecorded => self.on_result_recorded(now),
            DuelEvent::RematchOffered { from, stakes } => self.on_rematch_offered(from, stakes, now),
            DuelEvent::RematchSent { home } => {
                self.bot_rematch_replying[home] = false;
                // R3213000: a reply that kept the bot past its linger ends its stay as it lands.
                if self.finished
                    && !self.stopped
                    && self.bot_rematch[home].is_some_and(|linger| now >= linger.until)
                {
                    self.emit(DuelOutput::PresenceChanged);
                }
            }
            DuelEvent::Stop => self.on_stop(),
        }
        self.take_out()
    }

    // Reads, for the host's own sends and its checkpoint

    pub fn match_id(&self) -> &str {
        &self.match_id
    }

    /// The game as it stands (never sent to a seat: a seat's window onto it is [`DuelHost::view_for`]).
    pub fn game(&self) -> &GameState {
        &self.game
    }

    /// The log's next seq.
    pub fn next_seq(&self) -> i64 {
        self.next_seq
    }

    /// The newest nonce in the log, which a checkpoint is keyed by.
    pub fn last_nonce(&self) -> Option<&str> {
        self.acks.last().map(|(nonce, _)| nonce.as_str())
    }

    /// Whether the result is written (or the match was finished when the host was made).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn is_voided(&self) -> bool {
        self.voided
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// R672: when the rematch window opened, or `None` while the game is on.
    pub fn finished_at(&self) -> Option<i64> {
        self.finished_at
    }

    /// R677: the seat the account that began the match in `home` plays now — and, read the other way,
    /// the account (by its home) that plays engine seat `home`; a swap is its own inverse.
    pub fn playing(&self, home: PlayerId) -> PlayerId {
        if self.swapped { home.opponent() } else { home }
    }

    /// R639000: by home seat, the turn each bot last moved in.
    pub fn bot_moved_turn(&self) -> &PerPlayer<Option<i32>> {
        &self.bot_moved_turn
    }

    /// The deadlines as the clock holds them, for the protocol's `clock` message and the store.
    pub fn clocks(&self) -> MatchClocks {
        self.clock.snapshot()
    }

    /// The acting deadline `player` is under (R268, R79), in ms from `now`.
    pub fn remaining_for(&self, player: PlayerId, now: i64) -> Option<i64> {
        self.clock.remaining_for(player, now)
    }

    /// §10.8: the only window a seat gets onto the match, with the clock filled in (R79: the engine never
    /// reads a clock, so the view's `clockMs` is filled in here, from the one component that knows).
    pub fn view_for(&self, player: PlayerId, now: i64) -> PlayerView {
        let view = jackioh_engine::view_for(&self.game, player);
        PlayerView {
            clock_ms: self
                .clock
                .remaining_for(player, now)
                .map(|ms| i32::try_from(ms).unwrap_or(i32::MAX)),
            ..view
        }
    }

    /// The actions `player` may take now.
    pub fn legal(&self, player: PlayerId) -> Vec<ActionBody> {
        jackioh_engine::legal_actions(&self.game, player)
    }

    /// R672, R1452, R639000: seats with an open socket; a bot's seat is present during play and, after a
    /// ranked game, lingers until its plan or its pending reply finishes, after which it is absent.
    pub fn presence(&self, now: i64) -> PerPlayer<bool> {
        let playing_on = !self.finished && !self.voided;
        let lingering = |home: PlayerId| {
            self.finished
                && (self.bot_rematch_replying[home]
                    || self.bot_rematch[home].is_some_and(|linger| now < linger.until))
        };
        let open = |home: PlayerId| {
            self.connected[home] || (self.bots[home].is_some() && (playing_on || lingering(home)))
        };
        PerPlayer::new(open(PlayerId::P1), open(PlayerId::P2))
    }

    // Output

    fn flush_timers(&mut self) {
        for command in self.fx.take() {
            self.out.push(DuelOutput::Timer { command });
        }
    }

    fn emit(&mut self, output: DuelOutput) {
        self.flush_timers();
        self.out.push(output);
    }

    fn take_out(&mut self) -> Vec<DuelOutput> {
        self.flush_timers();
        std::mem::take(&mut self.out)
    }

    fn note(&mut self, note: HostNote) {
        self.emit(DuelOutput::Note { note });
    }

    /// Sends to the account playing engine seat `player` now, if it is here.
    fn send(&mut self, player: PlayerId, message: SeatMessage) {
        let home = self.playing(player);
        self.send_to_home(home, message);
    }

    fn send_to_home(&mut self, home: PlayerId, message: SeatMessage) {
        if !self.connected[home] {
            return;
        }
        self.emit(DuelOutput::Send { home, message });
    }

    fn push_view(&mut self, player: PlayerId, now: i64) {
        let home = self.playing(player);
        if !self.connected[home] {
            return;
        }
        let view = Box::new(self.view_for(player, now));
        let legal = self.legal(player);
        self.send_to_home(home, SeatMessage::View { view, legal });
    }

    fn push_clock(&mut self, player: PlayerId, now: i64) {
        let clocks = self.clock.snapshot();
        self.send(player, SeatMessage::Clock { now, clocks });
    }

    /// §9.5: a fresh full view, never a log replay.
    fn greet(&mut self, home: PlayerId, player: PlayerId, now: i64) {
        self.emit(DuelOutput::Greeting {
            home,
            part: GreetingPart::Before,
        });
        self.push_view(player, now);
        self.push_clock(player, now);
        self.emit(DuelOutput::Greeting {
            home,
            part: GreetingPart::After,
        });
    }

    // Arming, and who is here

    fn on_arm(&mut self, now: i64) {
        let snapshot = snapshot(&self.game);
        self.clock.sync(&clock_view_for(&snapshot), now, &mut self.fx);
        // R744, R1452: a bot's seat is here from the start, as an opponent whose socket has attached, so
        // a person who has not opened the match yet is away from now.
        if snapshot.result.is_none() && self.bots.iter().any(|(_, bot)| bot.is_some()) {
            self.start_absent_grace(now);
        }
        self.persist_clocks();
        // A host made from a checkpoint has none of the timers its bots' emotes were waiting on: what they
        // had rolled waits out its delay again (R645).
        for home in PLAYER_IDS {
            self.bot_emote_timer[home] = None;
            self.arm_emotes(home);
            self.bot_long_turn_timer[home] = None;
            if snapshot.result.is_none()
                && let Some(view) = self.bot_views[home].clone()
            {
                self.rearm_long_turn(home, &view, now);
            }
        }
        if snapshot.result.is_some() {
            self.on_terminal(&snapshot, now);
        } else {
            self.drive_bots(now);
        }
    }

    /// R744: starts grace for unattached accounts when the first socket connects, on the seat played
    /// (R677), from the stored deadline or afresh; a bot's as the host arms (R1452). Nothing starts at
    /// boot, so unreturned matches stay reaper draws (R112); repeat-safe (R147).
    fn start_absent_grace(&mut self, now: i64) {
        for home in PLAYER_IDS {
            if self.attached_once[home] {
                continue;
            }
            let player = self.playing(home);
            let stored = self.stored_grace.as_ref().and_then(|grace| grace[player]);
            self.clock.start_grace(player, stored, now, &mut self.fx);
        }
        self.stored_grace = None;
    }

    fn on_attached(&mut self, home: PlayerId, now: i64) {
        let player = self.playing(home);
        self.clock.clear_grace(player, &mut self.fx);
        // R744: an account that has not been here yet is away from now.
        self.start_absent_grace(now);
        self.persist_clocks();
        // §9.5: a fresh full view, never a log replay.
        self.greet(home, player, now);
        self.push_clock(player.opponent(), now);
        self.emit(DuelOutput::Greeting {
            home,
            part: GreetingPart::Watchers,
        });
    }

    fn on_left(&mut self, home: PlayerId) {
        self.connected[home] = false;
        if self.stopped || self.finished {
            return;
        }
        // R738, R2739001: the aim and the choice hover of the seat this account plays (R677) end with
        // its socket.
        let player = self.playing(home);
        self.end_aim(player);
        self.end_hover(player);
    }

    /// §9.5: the grace starts, and "the clock keeps running while a player is disconnected" — on the
    /// seat this account plays (R677).
    fn on_disconnected(&mut self, home: PlayerId, now: i64) {
        if self.stopped || self.finished {
            return;
        }
        let player = self.playing(home);
        self.clock.start_grace(player, None, now, &mut self.fx);
        self.persist_clocks();
        self.push_clock(player.opponent(), now);
    }

    fn on_stop(&mut self) {
        self.stopped = true;
        self.clock.stop(&mut self.fx);
        for player in PLAYER_IDS {
            if let Some(id) = self.aims[player].timer.take() {
                self.fx.cancel(TimerKey::Aim { player }, id);
            }
            if let Some(id) = self.hovers[player].timer.take() {
                self.fx.cancel(TimerKey::Hover { player }, id);
            }
            if let Some(waiting) = self.bot_waiting[player].take() {
                self.fx.cancel(TimerKey::BotMove { player }, waiting.move_timer);
                if let Some(id) = waiting.aim_timer {
                    self.fx.cancel(TimerKey::BotAim { player }, id);
                }
            }
            if let Some(id) = self.bot_emote_timer[player].take() {
                self.fx.cancel(TimerKey::BotEmote { player }, id);
            }
            if let Some(id) = self.bot_long_turn_timer[player].take() {
                self.fx.cancel(TimerKey::BotLongTurn { player }, id);
            }
        }
    }

    // The clocks (R79, §9.5)

    /// The clocks as stored: the clock's own, plus any stored grace not yet handed to it (R744).
    fn clocks_to_store(&self) -> MatchClocks {
        let clocks = self.clock.snapshot();
        let Some(stored) = &self.stored_grace else {
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

    fn persist_clocks(&mut self) {
        let clocks = self.clocks_to_store();
        if clocks == self.persisted_clocks {
            return;
        }
        self.persisted_clocks = clocks.clone();
        // §9.5: "the grace countdown is stored on the match so both clients show it".
        self.emit(DuelOutput::PersistClocks { clocks });
    }

    /// R79: clock expiries (turn, prompt, grace, ceiling) and R268 mulligans become logged server
    /// actions, since `(seed, log)` must reconstruct the match (§9.3).
    pub fn server_actions_for(&self, expiry: ClockExpiry) -> Vec<(PlayerId, ActionBody)> {
        let snapshot = snapshot(&self.game);
        match expiry {
            ClockExpiry::Turn { player } | ClockExpiry::Prompt { player } => {
                vec![(player, ActionBody::Timeout)]
            }
            // R268: seats still owing as this runs (one may have answered since the alarm), in seat
            // order; R146: stamped with own seat.
            ClockExpiry::Mulligan => mulligan_window(&snapshot)
                .into_iter()
                .map(|player| (player, ActionBody::Timeout))
                .collect(),
            // SPEC §11 R146: a disconnect timeout is stamped with the disconnected player (§10.2).
            ClockExpiry::Grace { player } => vec![(player, ActionBody::DisconnectExpired { player })],
            // SPEC §11 R146: the ceiling is no seat's, stamped with the active one by convention; R79
            // draw, R112 reaper draw.
            ClockExpiry::Ceiling => vec![(snapshot.active, ActionBody::CeilingReached)],
            // R1563006: the question still open for that player is answered Stop (R1184001's answer),
            // read when the expiry runs, since the player may have stopped or missed in between.
            ClockExpiry::Sequence { player } => match &snapshot.sequence {
                Some(question) if question.player == player => {
                    jackioh_engine::prompts::stop_answer(&self.game)
                        .map(|stop| vec![(player, stop)])
                        .unwrap_or_default()
                }
                _ => Vec::new(),
            },
        }
    }

    fn on_timer(&mut self, key: TimerKey, id: u32, now: i64) {
        match key {
            TimerKey::Turn
            | TimerKey::Prompt
            | TimerKey::Mulligan
            | TimerKey::Sequence
            | TimerKey::Ceiling
            | TimerKey::Grace { .. } => {
                if let Some(expiry) = self.clock.fire(key, id) {
                    self.on_expire(expiry, now);
                }
            }
            TimerKey::Aim { player } => {
                if self.aims[player].timer != Some(id) {
                    return;
                }
                self.aims[player].timer = None;
                self.flush_aim(player, now);
            }
            TimerKey::Hover { player } => {
                if self.hovers[player].timer != Some(id) {
                    return;
                }
                self.hovers[player].timer = None;
                self.flush_hover(player, now);
            }
            TimerKey::BotOffer { player } => {
                if self.bot_offer_timer[player] != Some(id) {
                    return;
                }
                self.bot_offer_timer[player] = None;
                self.emit(DuelOutput::OfferRematch {
                    home: player,
                    stakes: STAKE_NORMAL,
                    reply: false,
                });
            }
            TimerKey::BotLeaves { player } => {
                if self.bot_leave_timer[player] != Some(id) {
                    return;
                }
                self.bot_leave_timer[player] = None;
                // R3213000: the bot is gone as its linger ends, unless its reply is on its way, which
                // keeps it until the reply lands (`RematchSent`).
                if !self.stopped && !self.bot_rematch_replying[player] {
                    self.emit(DuelOutput::PresenceChanged);
                }
            }
            TimerKey::BotReply { player } => {
                if self.bot_reply_timer[player] != Some(id) {
                    return;
                }
                self.bot_reply_timer[player] = None;
                let stakes = self.bot_reply_stakes[player];
                self.emit(DuelOutput::OfferRematch {
                    home: player,
                    stakes,
                    reply: true,
                });
            }
            TimerKey::BotRetry => {
                if self.bot_retry != Some(id) {
                    return;
                }
                self.bot_retry = None;
                self.drive_bots(now);
            }
            TimerKey::BotMove { player } => {
                if self.bot_waiting[player]
                    .as_ref()
                    .map(|waiting| waiting.move_timer)
                    != Some(id)
                {
                    return;
                }
                if let Some(waiting) = self.bot_waiting[player].take() {
                    self.play_bot_move(
                        player,
                        waiting.version,
                        waiting.turn,
                        waiting.body,
                        waiting.aimed,
                        now,
                    );
                }
            }
            TimerKey::BotAim { player } => {
                let Some(waiting) = self.bot_waiting[player].as_mut() else {
                    return;
                };
                if waiting.aim_timer != Some(id) {
                    return;
                }
                waiting.aim_timer = None;
                self.show_bot_aim(player, now);
            }
            TimerKey::BotEmote { player } => {
                if self.bot_emote_timer[player] != Some(id) {
                    return;
                }
                self.bot_emote_timer[player] = None;
                if self.stopped {
                    return;
                }
                let due: Vec<DueEmote> = self.bot_emotes[player]
                    .iter()
                    .copied()
                    .take_while(|emote| emote.at <= now)
                    .collect();
                self.bot_emotes[player].drain(..due.len());
                for DueEmote { emote, .. } in due {
                    self.emit(DuelOutput::BotEmote { home: player, emote });
                }
                self.arm_emotes(player);
            }
            TimerKey::BotLongTurn { player } => {
                if self.bot_long_turn_timer[player] != Some(id) {
                    return;
                }
                self.bot_long_turn_timer[player] = None;
                if self.finished || self.stopped {
                    return;
                }
                let Some(view) = self.bot_views[player].clone() else {
                    return;
                };
                let intents = self.roll(player, |persona, rng| {
                    persona.on_player_turn_long(rng, &view, now)
                });
                self.schedule_emotes(player, intents, now);
            }
            // A Duos host's timers are none of a 1v1 host's.
            TimerKey::DuosMulligan
            | TimerKey::DuosRound
            | TimerKey::DuosPrompt { .. }
            | TimerKey::DuosGrace { .. }
            | TimerKey::DuosCeiling
            | TimerKey::DuosAi { .. }
            | TimerKey::DuosAiRetry => {}
        }
    }

    /// A clock ran out: its server actions, one after the other, each logged before the next.
    fn on_expire(&mut self, expiry: ClockExpiry, now: i64) {
        if self.stopped || self.finished {
            return;
        }
        let queue = self.server_actions_for(expiry);
        self.run = Some(ExpiryRun { expiry, queue });
        self.advance_run(now);
    }

    fn advance_run(&mut self, now: i64) {
        loop {
            // `stop()` sets `stopped` from outside the queue, so it can land between two of R268's
            // timeouts; nothing is dispatched into a host that is going away or a match that is over.
            let Some(run) = self.run.as_mut() else {
                return;
            };
            if run.queue.is_empty() || self.stopped || self.finished {
                self.run = None;
                return;
            }
            let (player, body) = run.queue.remove(0);
            let expiry = run.expiry;
            self.note(HostNote::ClockExpired {
                kind: expiry.kind().to_string(),
                player,
            });
            // The nonce is derived from the seq this action will occupy: unique, and stable across a
            // rebuild, so a rebuilt host cannot collide with a nonce already in the log. Its prefix is
            // one no client may send (R270), so no client can pre-empt it.
            let nonce = format!("{SERVER_NONCE_PREFIX}{}-{}", expiry.kind(), self.next_seq);
            let origin = Origin::Clock { expiry, player };
            match self.begin_apply(player, nonce.clone(), body, &origin, now) {
                Some(answer) => {
                    if !self.resolve(origin, nonce, answer, now) {
                        return;
                    }
                }
                None => return,
            }
        }
    }

    // Applying an action (§9.3)

    /// The one path every action takes — a client's and the clock's alike: reduce, ask for exactly one
    /// log row at the next gapless seq, and on the row's `Logged` push.
    fn apply(&mut self, player: PlayerId, nonce: String, body: ActionBody, origin: Origin, now: i64) {
        if let Some(answer) = self.begin_apply(player, nonce.clone(), body, &origin, now) {
            self.resolve(origin, nonce, answer, now);
        }
    }

    /// Reduces and asks for the row; the answer if there is one already, `None` when the row is awaited.
    fn begin_apply(
        &mut self,
        player: PlayerId,
        nonce: String,
        body: ActionBody,
        origin: &Origin,
        now: i64,
    ) -> Option<ActionAnswer> {
        if let Some(seq) = self.acks.get(&nonce) {
            return Some(ActionAnswer::Ack { seq: *seq });
        }
        if self.panicked.contains(&nonce) {
            return Some(ActionAnswer::Panicked);
        }
        if self.finished {
            return Some(ActionAnswer::MatchOver);
        }
        // One write at a time: the host feeds `Logged` before the next action.
        if self.awaiting.is_some() {
            return Some(ActionAnswer::NotRecorded);
        }
        // §9.1: the seat is the host's, never the client's.
        let action = Action::new(body, player, nonce.clone());
        // R1253000: `reduce` takes `&state` and returns a new one, so a panic leaves nothing
        // half-applied, as the rebuild's folds rely on.
        let reduced = catch_unwind(AssertUnwindSafe(|| jackioh_engine::reduce(&self.game, &action)));
        let seq = self.next_seq;
        let result = match reduced {
            Ok(result) => result,
            Err(payload) => {
                let message = jackioh_ai::panic_message(&*payload);
                self.panicked.insert(nonce);
                self.emit(DuelOutput::Panicked { action, seq, message });
                return Some(ActionAnswer::Panicked);
            }
        };
        if let Some(reason) = result.error {
            // §9.8: "Every rejected action is logged with its reason."
            self.note(HostNote::ActionRejected {
                player,
                action_type: action.action_type().to_string(),
                reason: reason.clone(),
            });
            // R835000: reducer refusals count toward the unusual-rejection alert, on the account (R677).
            let home = self.playing(player);
            self.emit(DuelOutput::Refused { home });
            return Some(ActionAnswer::Illegal { reason });
        }
        // §9.3: row before state, so a failed write never leaves the state ahead of the log.
        self.emit(DuelOutput::Append {
            seq,
            action: action.clone(),
            at: now,
        });
        self.awaiting = Some(Awaiting {
            last_event: result.events.last().map(jackioh_engine::GameEvent::event_type),
            fresh: result.events.len(),
            after: result.state,
            action,
            seq,
            at: now,
            origin: origin.clone(),
        });
        None
    }

    fn on_logged(&mut self, outcome: LogOutcome, now: i64) {
        let Some(awaiting) = self.awaiting.take() else {
            return;
        };
        let Awaiting {
            action,
            after,
            fresh,
            last_event,
            seq,
            at,
            origin,
        } = awaiting;
        let nonce = action.nonce.clone();
        let answer = match outcome {
            LogOutcome::Written => {
                let player = action.player_id;
                let before = std::mem::replace(&mut self.game, after);
                self.next_seq = seq + 1;
                self.acks.insert(nonce.clone(), seq);
                // R1442: the move as it arrived, on the state it was made in, once the state has moved
                // on with the log, so nothing the telemetry does can leave the two apart.
                let clock_left_ms = self.clock.remaining_for(player, now);
                self.emit(DuelOutput::Applied {
                    before: Some(Box::new(before)),
                    action,
                    seq,
                    at,
                    clock_left_ms,
                    last_event,
                });
                self.after_change(fresh, now);
                ActionAnswer::Ack { seq }
            }
            LogOutcome::Failed | LogOutcome::Diverged => {
                // R776000: the log holds this host's next seq already: it lets go, unless stopped (the
                // registry may hold its rebuilt successor).
                if outcome == LogOutcome::Diverged && !self.stopped {
                    self.emit(DuelOutput::LetGo {
                        reason: LetGo::Diverged,
                    });
                }
                ActionAnswer::NotRecorded
            }
        };
        if self.resolve(origin, nonce, answer, now) {
            self.advance_run(now);
        }
    }

    /// An action has its answer: say it, and let what follows from where it came go on. Answers whether
    /// the clock's run of server actions may go on.
    fn resolve(&mut self, origin: Origin, nonce: String, answer: ActionAnswer, now: i64) -> bool {
        self.emit(DuelOutput::Answer {
            nonce: nonce.clone(),
            answer: answer.clone(),
        });
        match origin {
            Origin::Seat => false,
            Origin::Clock { expiry, player } => {
                // R1253000: `reduce` is pure, so a retry would panic again: the match ends now, as the
                // reaper would end it at the ceiling, and nothing is retried.
                if self.panicked.contains(&nonce) {
                    self.end_as_ceiling_draw(now);
                    self.run = None;
                    return false;
                }
                // R776000: a row the store did not take. The clock that ran out runs out again, and the
                // action is derived again then, under the same nonce while no other row has taken its
                // seq. Ending the run ends R268's loop: the retried mulligan expiry derives the seats
                // still owing.
                if answer.is_internal() {
                    if !self.stopped {
                        self.note(HostNote::ClockRetrying {
                            kind: expiry.kind().to_string(),
                            player,
                            in_ms: MATCH_SERVER_ACTION_RETRY_MS,
                        });
                        self.clock
                            .retry(expiry, MATCH_SERVER_ACTION_RETRY_MS, now, &mut self.fx);
                    }
                    self.run = None;
                    return false;
                }
                true
            }
            Origin::Bot {
                home,
                stale,
                previous_turn,
                base,
                retried,
                aimed,
            } => {
                let player = self.playing(home);
                // A refused fresh move falls back to `fallback_action` once.
                if !retried
                    && answer.is_illegal()
                    && !stale
                    && let Some(body) = fallback_action(&self.game, player)
                {
                    let origin = Origin::Bot {
                        home,
                        stale,
                        previous_turn,
                        base: base.clone(),
                        retried: true,
                        aimed,
                    };
                    let nonce = format!("{base}-r");
                    if let Some(answer) = self.begin_apply(player, nonce.clone(), body, &origin, now) {
                        self.resolve(origin, nonce, answer, now);
                    }
                    return false;
                }
                if !matches!(answer, ActionAnswer::Ack { .. }) {
                    self.bot_moved_turn[home] = previous_turn;
                    self.note(HostNote::BotMoveRefused {
                        player,
                        answer: answer.clone(),
                    });
                    if !answer.is_illegal() {
                        self.drive_bots_after(BOT_WRITE_RETRY_MS, now);
                    } else if stale {
                        self.drive_bots(now);
                    }
                }
                // R738: the aim a bot showed before its move is taken down after it.
                if aimed {
                    let player = self.playing(home);
                    self.receive_aim(player, None, now);
                }
                false
            }
        }
    }

    /// MD-D29, R1127: mints an `Emote` action only if a card hears it; otherwise the relay suffices
    /// (R643). The sender showed it already, so the answer is dropped.
    fn on_emote(&mut self, home: PlayerId, emote: jackioh_engine::EmoteId, now: i64) {
        // R645: a person's emote may earn the bot opposite a reply; bots never answer bots. R1351000: a
        // muted pair is answered by no persona.
        if self.bots[home].is_none() && !self.emotes_muted {
            let foe = home.opponent();
            let intents = self.roll(foe, |persona, rng| persona.on_player_emote(rng, emote, now));
            self.schedule_emotes(foe, intents, now);
        }
        let player = self.playing(home);
        let wanted = ActionBody::Emote { emote };
        if !self.legal(player).contains(&wanted) {
            return;
        }
        let nonce = format!("{SERVER_NONCE_PREFIX}emote-{}", self.next_seq);
        self.apply(player, nonce, wanted, Origin::Seat, now);
    }

    /// Everything the clients need after a state change, in one order for both seats. `fresh` is how many
    /// events the change produced, which the bots' personas read (R645).
    fn after_change(&mut self, fresh: usize, now: i64) {
        let snapshot = snapshot(&self.game);
        if snapshot.seats_swapped != self.swapped {
            self.on_seats_swapped(snapshot.seats_swapped, now);
        }
        self.clock.sync(&clock_view_for(&snapshot), now, &mut self.fx);
        self.emit(DuelOutput::Changed { fresh });
        self.feed_personas(fresh, now);
        self.persist_clocks();

        // R2739001: a hover on a prompt the change answered or replaced is cleared before the view that
        // shows it gone.
        self.end_stale_hovers();
        for player in PLAYER_IDS {
            self.push_view(player, now);
        }
        self.push_prompts(&snapshot);
        for player in PLAYER_IDS {
            self.push_clock(player, now);
        }
        self.emit(DuelOutput::Pushed {
            snapshot: snapshot.clone(),
        });

        if snapshot.result.is_some() {
            self.on_terminal(&snapshot, now);
        } else {
            self.drive_bots(now);
        }
    }

    /// The `prompt` frames for a change in who owes an answer. Each is built off that seat's own
    /// `view_for`, so a frame never carries more than the view it travels with (§10.8).
    fn push_prompts(&mut self, snapshot: &MatchSnapshot) {
        let deadline = self.clock.snapshot().prompt_deadline;

        // R265, R266: no seat holds `pending` here: an owing seat gets its prompt, one that answered
        // learns the other still owes, and neither names the sealed answer.
        let owed = mulligan_window(snapshot);
        let owed_key = mulligan_key(&owed);
        if !owed.is_empty() && owed_key != self.last_mulligan_owed {
            for player in PLAYER_IDS {
                if !owed.contains(&player) {
                    self.send(
                        player,
                        SeatMessage::PromptForOpponent {
                            pending_for: player.opponent(),
                            deadline,
                        },
                    );
                    continue;
                }
                let view = jackioh_engine::view_for(&self.game, player);
                if let Some(PendingView::ForYou(pending)) = &view.pending {
                    self.send(
                        player,
                        SeatMessage::PromptForYou {
                            player,
                            choice_id: pending.choice_id.clone(),
                            kind: pending.kind,
                            deadline,
                        },
                    );
                }
            }
        }
        self.last_mulligan_owed = owed_key;

        // §10.6: one prompt at a time; the player who does not hold it learns only that it is open.
        let pending_for = snapshot.pending_for;
        if let Some(holder) = pending_for
            && Some(holder) != self.last_pending_for
        {
            let view = jackioh_engine::view_for(&self.game, holder);
            if let Some(PendingView::ForYou(pending)) = &view.pending {
                self.send(
                    holder,
                    SeatMessage::PromptForYou {
                        player: holder,
                        choice_id: pending.choice_id.clone(),
                        kind: pending.kind,
                        deadline,
                    },
                );
            }
            self.send(
                holder.opponent(),
                SeatMessage::PromptForOpponent {
                    pending_for: holder,
                    deadline,
                },
            );
        }
        self.last_pending_for = pending_for;
    }

    /// R677: a Glitch swapped the seats. From here every socket reads the view of, and acts for, the
    /// other seat. A disconnect grace belongs to the account, so one running moves with it to the seat
    /// it plays now (restarted there: the clock keeps one deadline per seat, not per account).
    fn on_seats_swapped(&mut self, swapped: bool, now: i64) {
        // R1452: a bot has no socket and is never away.
        let away: Vec<PlayerId> = PLAYER_IDS
            .into_iter()
            .filter(|home| !self.connected[*home] && self.bots[*home].is_none())
            .collect();
        for home in &away {
            let player = self.playing(*home);
            self.clock.clear_grace(player, &mut self.fx);
        }
        self.swapped = swapped;
        for home in &away {
            let player = self.playing(*home);
            self.clock.start_grace(player, None, now, &mut self.fx);
        }
        // R645: a bot's persona reads the seat its bot plays now, from a fresh start: the view it read last
        // is the other seat's, and no difference between the two is anything that happened.
        for home in PLAYER_IDS {
            let seat = self.playing(home);
            if let Some(persona) = self.bot_personas[home].as_mut() {
                persona.set_seat(seat);
                self.bot_views[home] = None;
            }
        }
        self.emit(DuelOutput::SeatsSwapped { swapped });
    }

    // The game's end (§9.5)

    /// "Every ending records a result and clears both players' in-match state." Once.
    fn on_terminal(&mut self, snapshot: &MatchSnapshot, now: i64) {
        let Some(result) = snapshot.result else {
            return;
        };
        if self.finished {
            return;
        }
        self.finished = true;
        self.finished_at = Some(now);
        self.clock.stop(&mut self.fx);
        // R738, R2739001: no aim or choice hover outlives the game.
        for player in PLAYER_IDS {
            self.end_aim(player);
            self.end_hover(player);
        }
        // R639000: under the step that ends the game, so a bot never reads as gone while the result is
        // written and then as present again.
        if result.reason != GameOverReason::Voided {
            self.plan_bot_rematches(now);
        }
        // R679: the one ending that records nothing. Only the engine's state reaches it.
        if result.reason == GameOverReason::Voided {
            self.voided = true;
            self.stopped = true;
            self.emit(DuelOutput::Voided { at: now });
            return;
        }
        let request = RecordRequest {
            outcome: result,
            turns: snapshot.turn,
            at: now,
            seats_swapped: self.swapped,
            // R417, R565: each seat's board as the game ended, from its own side.
            last_boards: (
                jackioh_engine::last_board_for(&self.game, PlayerId::P1),
                jackioh_engine::last_board_for(&self.game, PlayerId::P2),
            ),
            // R768: the final state's hash, which the match's replay is held to.
            final_hash: jackioh_engine::hash_state(&self.game),
        };
        self.emit(DuelOutput::Record { request });
    }

    /// R1253000, R112: a clock action whose `reduce` panicked ends the match as a ceiling draw with
    /// turns 0 and ratings unchanged, written as any ending is.
    fn end_as_ceiling_draw(&mut self, now: i64) {
        if self.finished || self.stopped {
            return;
        }
        self.finished = true;
        self.finished_at = Some(now);
        self.clock.stop(&mut self.fx);
        self.emit(DuelOutput::CeilingDraw { at: now });
    }

    /// R639000, R672: plans a bot's rematch and post-game linger in ranked games. Dropped if the result
    /// fails to land (the host lets the match go).
    fn plan_bot_rematches(&mut self, at: i64) {
        // R641000: a series game offers no rematch, so its bot stays for none. R1555000: nor do two
        // bots offer each other one.
        if !self.ranked || self.series_game || PLAYER_IDS.iter().all(|home| self.bots[*home].is_some()) {
            return;
        }
        // R1452: a bot that does not pass for a person stays for no rematch.
        for home in PLAYER_IDS {
            if self.bots[home].is_some_and(|bot| bot.hidden) {
                let plan = rematch_plan(&self.seed, home, &BOT_REMATCH_FIT);
                self.bot_rematch[home] = Some(Linger {
                    plan,
                    until: at + plan.linger_ms,
                });
                // R3213000: the end of the linger is a change in who is here, which the seats are told.
                self.bot_leave_timer[home] = Some(
                    self.fx
                        .set(TimerKey::BotLeaves { player: home }, at + plan.linger_ms),
                );
            }
        }
    }

    /// R639000, R672: once the result landed, each bot home whose plan offers makes a normal rematch
    /// offer `offer_after_ms` after the game's end.
    fn on_result_recorded(&mut self, now: i64) {
        let Some(at) = self.finished_at else {
            return;
        };
        for home in PLAYER_IDS {
            if let Some(Linger { plan, .. }) = self.bot_rematch[home]
                && plan.offers
            {
                let ms = (at + plan.offer_after_ms - now).max(0);
                if let Some(id) = self.bot_offer_timer[home].take() {
                    self.fx.cancel(TimerKey::BotOffer { player: home }, id);
                }
                self.bot_offer_timer[home] = Some(self.fx.set(TimerKey::BotOffer { player: home }, now + ms));
            }
        }
    }

    /// R639000, R672: a bot accepts a human's rematch offer if its plan permits, it is still lingering and
    /// has not yet answered, replying after `plan.reply_ms` at the matching stakes.
    fn on_rematch_offered(&mut self, from: PlayerId, stakes: RematchStakes, now: i64) {
        let bot = from.opponent();
        if self.bots[bot].is_none() {
            return;
        }
        let Some(Linger { plan, until }) = self.bot_rematch[bot] else {
            return;
        };
        if !plan.accepts || self.bot_rematch_answered[bot] || now >= until {
            return;
        }
        self.bot_rematch_answered[bot] = true;
        self.bot_rematch_replying[bot] = true;
        self.bot_reply_stakes[bot] = stakes;
        let ms = plan.reply_ms.max(0);
        self.bot_reply_timer[bot] = Some(self.fx.set(TimerKey::BotReply { player: bot }, now + ms));
    }

    // The bot seat (R1452)

    /// R1452: asks for the move of every bot owing one on its current seat (R677). The AI's search is the
    /// host's, one step apart from the machine ([`DuelHost::job`], answered by [`DuelEvent::Decided`]);
    /// the pace is the machine's (`on_decided`), and the clocks can still run out meanwhile. R639000: a
    /// bot that passes for a person may let its turn clock run out (`idles_turn`, drawn on the turn's
    /// stream, so every call agrees): then no move starts and the turn clock times it out (R79).
    fn drive_bots(&mut self, now: i64) {
        if self.finished || self.stopped {
            return;
        }
        for home in PLAYER_IDS {
            let Some(bot) = self.bots[home] else {
                continue;
            };
            if self.bot_thinking[home] {
                continue;
            }
            let player = self.playing(home);
            if !jackioh_ai::ai_to_act(&self.game, player) {
                continue;
            }
            let turn = self.game.turn;
            let first_in_turn = self.bot_moved_turn[home] != Some(turn);
            let own_turn_start = first_in_turn
                && self.game.active == player
                && self.game.mulligan.is_none()
                && self.game.pending.is_none();
            let clock_left_ms = self.clock.remaining_for(player, now);
            // Only a running clock can time the bot out; with none, an idle turn would never end.
            if own_turn_start
                && clock_left_ms.is_some()
                && bot.hidden
                && idles_turn(&self.seed, home, turn, &BOT_TIMEOUT_FIT)
            {
                continue;
            }
            let version = self.next_seq;
            self.bot_thinking[home] = true;
            self.bot_asked[home] = Some(Asked {
                at: now,
                first_in_turn,
                clock_left_ms,
            });
            let request = DecideRequest {
                home,
                seat: player,
                version,
                turn,
                first_in_turn,
                own_turn_start,
                clock_left_ms,
                deadline_ms: decision_deadline_ms(clock_left_ms),
                ai_cursor: self.bot_cursors[home].0,
            };
            self.emit(DuelOutput::Decide { request });
        }
    }

    /// What the host runs to answer `request`: the game and the bot's identity, owned so it can run on
    /// another thread or in a slice of a frame (`bots::decide`). `None` when no bot plays at `home`.
    pub fn job(&self, request: &DecideRequest) -> Option<DecideJob> {
        let bot = self.bots[request.home]?;
        Some(DecideJob {
            game: self.game.clone(),
            seat: request.seat,
            home: request.home,
            seed: self.seed.clone(),
            // A bot keeps its index in `BOT_STRENGTHS`; one past the table plays the last.
            strength: *BOT_STRENGTHS
                .get(usize::try_from(bot.strength).unwrap_or(usize::MAX))
                .or(BOT_STRENGTHS.last())?,
            hidden: bot.hidden,
            own_turn_start: request.own_turn_start,
            ai_cursor: request.ai_cursor,
            deadline_ms: request.deadline_ms,
        })
    }

    /// `drive_bots` once `ms` have passed: a bot whose move could not be written decides again at that
    /// pace (`BOT_WRITE_RETRY_MS`).
    fn drive_bots_after(&mut self, ms: i64, now: i64) {
        if self.bot_retry.is_none() {
            self.bot_retry = Some(self.fx.set(TimerKey::BotRetry, now + ms.max(0)));
        }
    }

    /// R1452, R639000: the host has decided the bot's move. A bot that passes for a person waits a
    /// human-paced think time, drawn from R1442's fitted think times for its kind of move and scaled by
    /// its speed, bounded by the clock it had when asked (R79); the time the decision took counts toward
    /// it. A targeted move shows its aim `aim_lead_ms` before it goes (R738); one that does not pass for a
    /// person moves at once, with no aim.
    fn on_decided(&mut self, home: PlayerId, version: i64, turn: i32, choice: BotChoice, now: i64) {
        self.bot_cursors[home].0 = choice.ai_cursor;
        let hidden = self.bots[home].is_some_and(|bot| bot.hidden);
        if self.finished || self.stopped || !hidden {
            self.bot_asked[home] = None;
            self.play_bot_move(home, version, turn, choice.body, false, now);
            return;
        }
        let Some(bot) = self.bots[home] else {
            return;
        };
        let asked = self.bot_asked[home].take().unwrap_or(Asked {
            at: now,
            first_in_turn: false,
            clock_left_ms: None,
        });
        let think = ThinkFor {
            action_kind: choice
                .body
                .as_ref()
                .map_or(ActionType::EndTurn, ActionBody::action_type),
            first_in_turn: asked.first_in_turn,
            legal_count: usize::try_from(choice.legal_count).unwrap_or(usize::MAX),
            lethal: choice.lethal,
        };
        let mut timing = Rng::new(
            &format!("{}:bot-timing:{home}", self.seed),
            self.bot_cursors[home].1,
        );
        let wait = think_ms(&mut timing, &think, &THINK_MODEL, bot.speed, asked.clock_left_ms);
        self.bot_cursors[home].1 = timing.cursor();
        let left = (wait - (now - asked.at).max(0)).max(0);
        // R738: the aim shows for the lead, never longer than what is left of the wait, and not at all
        // unless the relay would send it before the move.
        let lead = aim_lead_ms(&self.seed, home, version).min(left);
        let (aim, lead) = match choice.aim {
            Some(aim) if lead >= AIM_RELAY_INTERVAL_MS => (Some(aim), lead),
            _ => (None, 0),
        };
        if left == 0 && aim.is_none() {
            self.play_bot_move(home, version, turn, choice.body, false, now);
            return;
        }
        let aim_timer = match aim {
            Some(_) if left > lead => Some(self.fx.set(TimerKey::BotAim { player: home }, now + left - lead)),
            _ => None,
        };
        let move_timer = self.fx.set(TimerKey::BotMove { player: home }, now + left);
        self.bot_waiting[home] = Some(Waiting {
            version,
            turn,
            body: choice.body,
            aim,
            aim_timer,
            aimed: false,
            move_timer,
        });
        if aim.is_some() && aim_timer.is_none() {
            self.show_bot_aim(home, now);
        }
    }

    /// R738: the aim the bot at `home` waited to show goes out through the relay a person's goes by.
    fn show_bot_aim(&mut self, home: PlayerId, now: i64) {
        let Some(waiting) = self.bot_waiting[home].as_mut() else {
            return;
        };
        let Some(aim) = waiting.aim.take() else {
            return;
        };
        waiting.aimed = true;
        let player = self.playing(home);
        self.receive_aim(player, Some(aim), now);
    }

    /// R1452: applies a decided bot move with R270's prefix (bypassing R1442's think telemetry). R639000:
    /// concedes and answers draws as the habits drew them, offers one too (R2660001), never an emote, and
    /// takes down the aim it relayed after the move. A move on a stale state still goes in the same turn
    /// if the engine takes it. A refused fresh move falls back to `fallback_action` once; a failed write
    /// is decided again after `BOT_WRITE_RETRY_MS`.
    fn play_bot_move(
        &mut self,
        home: PlayerId,
        version: i64,
        turn: i32,
        body: Option<ActionBody>,
        aimed: bool,
        now: i64,
    ) {
        self.bot_thinking[home] = false;
        if self.finished || self.stopped {
            return;
        }
        let player = self.playing(home);
        let stale = self.next_seq != version;
        let body = body.filter(|body| !never_played(body));
        let body = if !stale {
            body.or_else(|| fallback_action(&self.game, player))
        } else if self.game.turn == turn {
            body
        } else {
            None
        };
        let Some(body) = body else {
            if stale {
                self.drive_bots(now);
            }
            if aimed {
                self.receive_aim(player, None, now);
            }
            return;
        };
        // Set before the move goes, since the move's own push already starts the next one.
        let now_turn = self.game.turn;
        let nonce = format!("{SERVER_NONCE_PREFIX}bot-{}", self.next_seq);
        let previous_turn = self.bot_moved_turn[home].replace(now_turn);
        let origin = Origin::Bot {
            home,
            stale,
            previous_turn,
            base: nonce.clone(),
            retried: false,
            aimed,
        };
        self.apply(player, nonce, body, origin, now);
    }

    // A bot's chatter (R645)

    /// Runs `roll` on the persona of the bot at `home`, with the draws of its own stream
    /// (`{seed}:bot-persona:{home}`, apart from the match rng and the search's), and moves the stream
    /// on. Nothing when the bot has no persona (it does not pass for a person, or no bot plays there).
    fn roll<T: Default>(
        &mut self,
        home: PlayerId,
        roll: impl FnOnce(&mut EmotePersona, &mut dyn FnMut() -> f64) -> T,
    ) -> T {
        let Some(mut persona) = self.bot_personas[home].take() else {
            return T::default();
        };
        let mut rng = Rng::new(
            &format!("{}:bot-persona:{home}", self.seed),
            self.bot_persona_cursor[home],
        );
        let rolled = roll(&mut persona, &mut || rng.next());
        self.bot_persona_cursor[home] = rng.cursor();
        self.bot_personas[home] = Some(persona);
        rolled
    }

    /// R645: each bot's persona reads the change off its own seat's view: the newest `fresh` events that
    /// view carries (the change's own, redacted for that seat) against the view it read last. What it
    /// rolls waits out its delay and goes out as the bot's emote (`schedule_emotes`).
    fn feed_personas(&mut self, fresh: usize, now: i64) {
        for home in PLAYER_IDS {
            if self.bot_personas[home].is_none() {
                continue;
            }
            let seat = self.playing(home);
            let view = jackioh_engine::view_for(&self.game, seat);
            // R2652000: less the face-down rolls this seat's view leaves out of the change.
            let fresh = jackioh_engine::viewed_of_newest(&self.game, seat, fresh);
            let from = view.events.len().saturating_sub(fresh);
            let prev = self.bot_views[home].take();
            let intents = self.roll(home, |persona, rng| {
                persona.on_events(rng, &view.events[from..], prev.as_ref(), &view, now)
            });
            self.rearm_long_turn(home, &view, now);
            self.bot_views[home] = Some(view);
            self.schedule_emotes(home, intents, now);
        }
    }

    /// R645's long-turn trigger: on every change that leaves the person the active seat, a fresh
    /// `AI_EMOTE.long_turn_ms` wait (the persona's once-a-turn check makes a second arm harmless); none on
    /// the bot's own turn or once the game is over.
    fn rearm_long_turn(&mut self, home: PlayerId, view: &PlayerView, now: i64) {
        if let Some(id) = self.bot_long_turn_timer[home].take() {
            self.fx.cancel(TimerKey::BotLongTurn { player: home }, id);
        }
        if self.finished || self.stopped || view.result.is_some() || view.active == view.viewer {
            return;
        }
        self.bot_long_turn_timer[home] = Some(self.fx.set(
            TimerKey::BotLongTurn { player: home },
            now + AI_EMOTE.long_turn_ms,
        ));
    }

    /// R645: each of a bot's intents waits out its own delay, then goes out as the bot's emote.
    fn schedule_emotes(&mut self, home: PlayerId, intents: Vec<AiEmote>, now: i64) {
        for intent in intents {
            let wait = intent.delay_ms.max(0.0).round() as i64;
            self.bot_emotes[home].push(DueEmote {
                at: now + wait,
                emote: intent.emote,
            });
        }
        self.bot_emotes[home].sort_by_key(|emote| emote.at);
        self.arm_emotes(home);
    }

    /// The timer for the bot's next due emote, in place of any it held.
    fn arm_emotes(&mut self, home: PlayerId) {
        if let Some(id) = self.bot_emote_timer[home].take() {
            self.fx.cancel(TimerKey::BotEmote { player: home }, id);
        }
        if let Some(next) = self.bot_emotes[home].first().copied() {
            self.bot_emote_timer[home] = Some(self.fx.set(TimerKey::BotEmote { player: home }, next.at));
        }
    }

    // The opponent's aim (R738)

    /// R738: an aim is relayed to the opponent at most once per `AIM_RELAY_INTERVAL_MS` per seat. Faster
    /// ones are coalesced, not dropped outright: the newest waits out the interval and goes alone, so
    /// the opponent's arrow always ends where the sender's did. Nothing is answered.
    fn receive_aim(&mut self, player: PlayerId, aim: Option<Aim>, now: i64) {
        if self.finished || self.stopped {
            return;
        }
        let slot = &mut self.aims[player];
        slot.pending = Some(aim);
        if slot.timer.is_some() {
            return;
        }
        let wait = match slot.last_at {
            None => -1,
            Some(last_at) => last_at + AIM_RELAY_INTERVAL_MS - now,
        };
        if wait <= 0 {
            self.flush_aim(player, now);
            return;
        }
        slot.timer = Some(self.fx.set(TimerKey::Aim { player }, now + wait));
    }

    /// Relays the seat's pending aim, checked against the state as it stands NOW rather than when the
    /// frame arrived: an aim whose ends name anything the opponent may not see is dropped silently, and
    /// the opponent's arrow is cleared instead if it showed one (R738, R97). A repeat of what the
    /// opponent was last told is not sent again.
    fn flush_aim(&mut self, player: PlayerId, now: i64) {
        let Some(aim) = self.aims[player].pending.take() else {
            return;
        };
        if self.finished || self.stopped {
            return;
        }
        let receiver = player.opponent();
        let shown = match aim {
            None => None,
            Some(aim) => {
                if aim_is_public(&aim, player, &jackioh_engine::view_for(&self.game, receiver)) {
                    Some(aim)
                } else {
                    None
                }
            }
        };
        if aim_key(shown.as_ref()) == aim_key(self.aims[player].relayed.as_ref()) {
            return;
        }
        self.aims[player].last_at = Some(now);
        self.aims[player].relayed = shown;
        self.send(
            receiver,
            SeatMessage::Aim {
                from: player,
                aim: shown,
            },
        );
    }

    /// A seat that leaves, or a match that ends, takes its arrow with it.
    fn end_aim(&mut self, player: PlayerId) {
        let slot = &mut self.aims[player];
        if let Some(id) = slot.timer.take() {
            self.fx.cancel(TimerKey::Aim { player }, id);
        }
        slot.pending = None;
        if slot.relayed.is_none() {
            return;
        }
        slot.relayed = None;
        self.send(
            player.opponent(),
            SeatMessage::Aim {
                from: player,
                aim: None,
            },
        );
    }

    // The opponent's choice hover (R2739001)

    /// R2739001: the id of the prompt `player` holds now, or `None`.
    fn open_prompt_of(&self, player: PlayerId) -> Option<String> {
        self.game
            .pending
            .as_ref()
            .filter(|pending| pending.player_id == player)
            .map(|pending| pending.id.clone())
    }

    /// R2739001: a hover on an option of the seat's own open prompt, relayed to the opponent with
    /// `receive_aim`'s coalescing, at most once per `AIM_RELAY_INTERVAL_MS` per seat. One while the seat
    /// holds no prompt is dropped silently; nothing is answered.
    fn receive_hover(&mut self, player: PlayerId, index: Option<i32>, now: i64) {
        if self.finished || self.stopped {
            return;
        }
        let open = self.open_prompt_of(player);
        if self.hovers[player].prompt != open {
            self.end_hover(player);
            self.hovers[player].prompt = open;
        }
        let slot = &mut self.hovers[player];
        if slot.prompt.is_none() {
            return;
        }
        slot.pending = Some(index);
        if slot.timer.is_some() {
            return;
        }
        let wait = match slot.last_at {
            None => -1,
            Some(last_at) => last_at + AIM_RELAY_INTERVAL_MS - now,
        };
        if wait <= 0 {
            self.flush_hover(player, now);
            return;
        }
        slot.timer = Some(self.fx.set(TimerKey::Hover { player }, now + wait));
    }

    /// Relays the seat's pending hover, checked against the state as it stands NOW: an index into a
    /// prompt no longer open, or one the opponent's view draws no back for (`hover_is_shown`), is dropped
    /// silently, and the opponent's outline cleared instead if it showed one. A repeat of what the
    /// opponent was last told is not sent again.
    fn flush_hover(&mut self, player: PlayerId, now: i64) {
        let Some(index) = self.hovers[player].pending.take() else {
            return;
        };
        if self.finished || self.stopped {
            return;
        }
        let receiver = player.opponent();
        let current = self.hovers[player].prompt == self.open_prompt_of(player);
        let shown = index.filter(|index| {
            current && hover_is_shown(*index, player, &jackioh_engine::view_for(&self.game, receiver))
        });
        if shown == self.hovers[player].relayed {
            return;
        }
        self.hovers[player].last_at = Some(now);
        self.hovers[player].relayed = shown;
        self.send(
            receiver,
            SeatMessage::ChoiceHover {
                from: player,
                index: shown,
            },
        );
    }

    /// An answered prompt, a seat that leaves or a match that ends takes its outline with it.
    fn end_hover(&mut self, player: PlayerId) {
        let slot = &mut self.hovers[player];
        if let Some(id) = slot.timer.take() {
            self.fx.cancel(TimerKey::Hover { player }, id);
        }
        slot.pending = None;
        slot.prompt = None;
        if slot.relayed.is_none() {
            return;
        }
        slot.relayed = None;
        self.send(
            player.opponent(),
            SeatMessage::ChoiceHover {
                from: player,
                index: None,
            },
        );
    }

    /// R2739001: ends each seat's hover whose prompt the state no longer holds open for it.
    fn end_stale_hovers(&mut self) {
        for player in PLAYER_IDS {
            let prompt = self.hovers[player].prompt.clone();
            if prompt.is_some() && prompt != self.open_prompt_of(player) {
                self.end_hover(player);
            }
        }
    }
}

impl DuelOutput {
    /// The timer command this output is, if it is one.
    pub fn timer(&self) -> Option<TimerCommand> {
        match self {
            DuelOutput::Timer { command } => Some(*command),
            _ => None,
        }
    }
}
