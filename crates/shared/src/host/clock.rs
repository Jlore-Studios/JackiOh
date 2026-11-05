//! The match clock (BUILD M7-T1, SPEC §9.5, R79), pure: it reads no time and starts no timer.
//!
//! Every call takes `now` (epoch ms) and a [`Fx`] to state the timers it wants, and the host feeds a
//! timer's firing back through [`MatchClock::fire`], which says which clock ran out. The deadlines are
//! none of the engine's (SPEC §9.3 keeps time out of `reduce`): an expiry reports *which* clock ran out,
//! and `host::expiry` turns it into the action the engine sees.
//!
//!  - turn clock (R79): the active player's, paused while a prompt is open for the non-active player;
//!  - holder prompt clock (R79): answers that prompt;
//!  - mulligan clock (R268): one deadline for both seats while open (R265);
//!  - disconnect grace per player (§9.5, R147, R744);
//!  - hard ceiling (R79), measured from `started_at`;
//!  - question sequence clock (R1563006, Meditative+ #20 KY's Competition).
//!
//! Every duration is a named constant of `crate::config` (R79).

use serde::{Deserialize, Serialize};

use jackioh_engine::config::KY_COMPETITION_SECONDS;
use jackioh_engine::{PLAYER_IDS, PerPlayer, PlayerId};

use super::timer::{Countdown, Fx, TimerKey};
use crate::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, MS_PER_SECOND, MULLIGAN_CLOCK_SECONDS,
    PROMPT_CLOCK_SECONDS, TURN_CLOCK_SECONDS,
};

// Unit conversion, not configuration: the definition of a minute (a second's is `crate::config`'s), and
// every lifecycle *duration* comes from `crate::config` (R79) rather than from here.
const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;

/// R79: the ceiling is one deadline measured from the moment the match started.
pub fn match_ceiling_at(started_at: i64) -> i64 {
    started_at + MATCH_CEILING_MINUTES * MS_PER_MINUTE
}

/// The `MatchClocks` a match row carries before its actor has synced once (§9.5: the deadlines are
/// stored on the match so both clients render them). Only the ceiling is known at that point: the
/// turn clock starts when the actor's first `sync` reports who is active.
pub fn initial_clocks(started_at: i64) -> MatchClocks {
    MatchClocks {
        turn_deadline: None,
        prompt_deadline: None,
        grace_deadline: PerPlayer::new(None, None),
        ceiling_at: match_ceiling_at(started_at),
    }
}

/// The deadlines both clients render and `matches.clocks` stores (§9.5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MatchClocks {
    /// Epoch ms the active player's turn clock expires, or null while it is paused.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub turn_deadline: Option<i64>,
    /// Epoch ms the open prompt's own clock expires (R79), or null.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub prompt_deadline: Option<i64>,
    /// Per-player disconnect grace deadlines (§9.5).
    #[cfg_attr(feature = "ts", ts(type = "{ p1: number | null, p2: number | null }"))]
    pub grace_deadline: PerPlayer<Option<i64>>,
    /// Epoch ms the hard ceiling is reached (R79).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub ceiling_at: i64,
}

/// Everything the clock needs to know about the game; none of it is hidden information.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ClockView {
    pub turn: i32,
    pub active: PlayerId,
    /// Who owes the open prompt an answer, or null.
    pub pending_for: Option<PlayerId>,
    /// R265: the seats that still owe their mulligan while both are open, in seat order; empty
    /// outside that window. Non-empty with no `pending_for` is the window the mulligan clock runs
    /// over (R268).
    pub mulligan_owed: Vec<PlayerId>,
    pub over: bool,
    /// R1563006: the question of a sequence open now (Meditative+ #20 KY's Competition), the sequence
    /// clock's whole input; `None` with none open. Public: who holds it, its streak and its prompt id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sequence: Option<SequenceClockView>,
}

/// R1563006: one question of a sequence, as the sequence clock reads it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SequenceClockView {
    pub player: PlayerId,
    /// The right answers so far: 0 on a sequence's first question, where its clock starts.
    pub streak: i32,
    pub prompt_id: String,
}

/// Which clock ran out.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClockExpiry {
    /// R79: the active player's turn clock ran out; `timeout` answers their prompts and ends the turn.
    Turn { player: PlayerId },
    /// R79: a prompt held by the non-active player ran out; `timeout` answers only that prompt.
    Prompt { player: PlayerId },
    /// R268: the one mulligan clock ran out. It names no player because it belongs to both: the
    /// host times out every seat still owing its mulligan at that moment, each with its own `timeout`.
    Mulligan,
    /// §9.5: grace expired; `disconnectExpired` makes it a loss for that player.
    Grace { player: PlayerId },
    /// R79: the hard wall-clock ceiling; `ceilingReached` ends the match in a draw.
    Ceiling,
    /// R1563006: a question sequence's clock ran out; its question is answered Stop for its player.
    Sequence { player: PlayerId },
}

impl ClockExpiry {
    /// The literal the server nonce (`srv-<kind>-<seq>`) and the logs carry.
    pub fn kind(&self) -> &'static str {
        match self {
            ClockExpiry::Turn { .. } => "turn",
            ClockExpiry::Prompt { .. } => "prompt",
            ClockExpiry::Mulligan => "mulligan",
            ClockExpiry::Grace { .. } => "grace",
            ClockExpiry::Ceiling => "ceiling",
            ClockExpiry::Sequence { .. } => "sequence",
        }
    }
}

/// The match clock's state: the countdowns, which of them is armed by which timer, and what is paused
/// or spent. It is part of the host's state, so a host that resumes from a checkpoint resumes the clocks.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MatchClock {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    ceiling_at: i64,
    stopped: bool,

    // The turn clock. `remaining` is the authority while it is paused: R79's pause resumes with the
    // time the clock had left, not from full.
    owner: Option<PlayerId>,
    turn_key: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    remaining: i64,
    turn: Countdown,

    // Non-active holder's prompt clock: `holder` outlives the timer so a later `sync` cannot re-arm.
    holder: Option<PlayerId>,
    prompt: Countdown,

    // R268: the mulligan deadline is set on first sight and never moved, so the second seat gets no
    // extra time. `mulligan_expired` outlives the timer so nothing re-arms over an open window.
    // A rebuilt host (§9.5) arms a fresh full deadline; only the disconnect grace reads its stored
    // deadline back (R744).
    mulligan: Countdown,
    mulligan_open: bool,
    mulligan_expired: bool,

    grace: PerPlayer<Countdown>,

    // R1563006: the question sequence's clock, armed at its first question (`sequence_first`, that
    // question's prompt id) and kept over the questions after it, so each answer spends from one window.
    // `sequence_player` is the first question's: the player the expiry names.
    sequence: Countdown,
    sequence_first: Option<String>,
    sequence_player: Option<PlayerId>,

    // R79: the match ceiling's timer, measured from `started_at`, inherited across rebuilds.
    ceiling: Option<u32>,
}

impl MatchClock {
    /// A clock for a match that started at `started_at` (epoch ms); nothing is armed until
    /// [`MatchClock::arm_ceiling`] and the first [`MatchClock::sync`].
    pub fn new(started_at: i64) -> MatchClock {
        MatchClock {
            ceiling_at: match_ceiling_at(started_at),
            stopped: false,
            owner: None,
            turn_key: None,
            remaining: TURN_CLOCK_SECONDS * MS_PER_SECOND,
            turn: Countdown::default(),
            holder: None,
            prompt: Countdown::default(),
            mulligan: Countdown::default(),
            mulligan_open: false,
            mulligan_expired: false,
            grace: PerPlayer::new(Countdown::default(), Countdown::default()),
            sequence: Countdown::default(),
            sequence_first: None,
            sequence_player: None,
            ceiling: None,
        }
    }

    fn turn_ms() -> i64 {
        TURN_CLOCK_SECONDS * MS_PER_SECOND
    }

    fn prompt_ms() -> i64 {
        PROMPT_CLOCK_SECONDS * MS_PER_SECOND
    }

    fn mulligan_ms() -> i64 {
        MULLIGAN_CLOCK_SECONDS * MS_PER_SECOND
    }

    fn grace_ms() -> i64 {
        DISCONNECT_GRACE_SECONDS * MS_PER_SECOND
    }

    fn sequence_ms() -> i64 {
        i64::from(KY_COMPETITION_SECONDS) * MS_PER_SECOND
    }

    /// Whether the clock was stopped (the game is over, or the host is going away).
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// R79: pausing banks the time left; arming spends it.
    fn pause_turn(&mut self, now: i64, fx: &mut Fx) {
        if self.turn.timer.is_none() {
            return;
        }
        self.remaining = (self.turn.deadline.unwrap_or(now) - now).max(0);
        self.turn.cancel(TimerKey::Turn, fx);
    }

    fn arm_turn(&mut self, now: i64, fx: &mut Fx) {
        if self.stopped || self.turn.timer.is_some() {
            return;
        }
        if self.owner.is_none() {
            return;
        }
        let ms = self.remaining.max(0);
        self.turn.deadline = Some(now + ms);
        self.turn.timer = Some(fx.set(TimerKey::Turn, now + ms));
    }

    fn arm_prompt(&mut self, player: PlayerId, now: i64, fx: &mut Fx) {
        self.arm_prompt_in(player, Self::prompt_ms(), now, fx);
    }

    /// Prompt clock for `player`, running out `ms` from now (`prompt_ms` or the R776000 retry delay).
    fn arm_prompt_in(&mut self, player: PlayerId, ms: i64, now: i64, fx: &mut Fx) {
        self.prompt.cancel(TimerKey::Prompt, fx);
        self.holder = Some(player);
        self.prompt.deadline = Some(now + ms);
        self.prompt.timer = Some(fx.set(TimerKey::Prompt, now + ms));
    }

    fn clear_prompt(&mut self, fx: &mut Fx) {
        self.prompt.cancel(TimerKey::Prompt, fx);
        self.holder = None;
    }

    /// R268: one deadline for the whole window, armed the first time it is seen.
    fn open_mulligan(&mut self, now: i64, fx: &mut Fx) {
        self.mulligan_open = true;
        let deadline = *self.mulligan.deadline.get_or_insert(now + Self::mulligan_ms());
        if self.stopped || self.mulligan.timer.is_some() || self.mulligan_expired {
            return;
        }
        self.mulligan.timer = Some(fx.set(TimerKey::Mulligan, now + (deadline - now).max(0)));
    }

    /// The window closed (both seats answered): its clock goes, and its deadline stays spent.
    fn close_mulligan(&mut self, fx: &mut Fx) {
        self.mulligan_open = false;
        if let Some(id) = self.mulligan.timer.take() {
            fx.cancel(TimerKey::Mulligan, id);
        }
    }

    /// R1563006: the sequence clock for `player`, running out `ms` from now.
    fn arm_sequence_in(&mut self, player: PlayerId, ms: i64, now: i64, fx: &mut Fx) {
        self.sequence.cancel(TimerKey::Sequence, fx);
        self.sequence_player = Some(player);
        self.sequence.deadline = Some(now + ms);
        self.sequence.timer = Some(fx.set(TimerKey::Sequence, now + ms));
    }

    /// R1563006: a sequence's first question (streak 0, a prompt id this clock has not started on) arms
    /// a fresh window; a later question keeps it; none open clears it. A question met mid-sequence with
    /// no window (a rebuilt host) arms a fresh one, as the turn clock does.
    fn sync_sequence(&mut self, view: &ClockView, now: i64, fx: &mut Fx) {
        let Some(question) = &view.sequence else {
            self.sequence.cancel(TimerKey::Sequence, fx);
            self.sequence_first = None;
            return;
        };
        let first =
            question.streak == 0 && self.sequence_first.as_deref() != Some(question.prompt_id.as_str());
        if first || self.sequence_first.is_none() {
            self.sequence_first = Some(question.prompt_id.clone());
            self.arm_sequence_in(question.player, Self::sequence_ms(), now, fx);
        }
    }

    /// Stops every clock. Idempotent.
    pub fn stop(&mut self, fx: &mut Fx) {
        self.stopped = true;
        self.turn.cancel(TimerKey::Turn, fx);
        self.clear_prompt(fx);
        self.close_mulligan(fx);
        self.sequence.cancel(TimerKey::Sequence, fx);
        for player in PLAYER_IDS {
            self.grace[player].cancel(TimerKey::Grace { player }, fx);
        }
        if let Some(id) = self.ceiling.take() {
            fx.cancel(TimerKey::Ceiling, id);
        }
    }

    /// R79: arms the ceiling at the time that is left to it.
    pub fn arm_ceiling(&mut self, now: i64, fx: &mut Fx) {
        self.arm_ceiling_in(self.ceiling_at - now, now, fx);
    }

    /// The ceiling timer, running out `ms` from now (the remainder, or the R776000 retry delay).
    fn arm_ceiling_in(&mut self, ms: i64, now: i64, fx: &mut Fx) {
        if let Some(id) = self.ceiling.take() {
            fx.cancel(TimerKey::Ceiling, id);
        }
        self.ceiling = Some(fx.set(TimerKey::Ceiling, now + ms.max(0)));
    }

    /// Called once after the game begins and again after every state change.
    pub fn sync(&mut self, view: &ClockView, now: i64, fx: &mut Fx) {
        // Once a match is over it stays over: a late `sync` must never re-arm a deadline.
        if self.stopped {
            return;
        }
        if view.over {
            self.stop(fx);
            return;
        }

        // R79: "The turn clock belongs to the active player". Both halves of the key matter — the
        // turn number alone would miss a replaced turn, and the active player alone would miss a
        // turn that somehow stayed with the same player.
        let key = format!("{}:{}", view.turn, view.active);
        if self.turn_key.as_deref() != Some(key.as_str()) {
            self.turn_key = Some(key);
            self.owner = Some(view.active);
            self.turn.cancel(TimerKey::Turn, fx);
            self.remaining = Self::turn_ms();
        }

        let pending_for = view.pending_for;

        // R265, R268: both mulligans open at once. Setup is nobody's turn (§2.1), so neither the turn
        // clock nor a prompt clock runs; the one mulligan deadline covers both seats. The turn clock is
        // paused, not reset, so a question a card asks during setup (§2.4) is still timed by R79, and
        // turn 1 starts a fresh turn clock when the turn key changes.
        if pending_for.is_none() && !view.mulligan_owed.is_empty() {
            self.clear_prompt(fx);
            self.pause_turn(now, fx);
            self.open_mulligan(now, fx);
            return;
        }
        self.close_mulligan(fx);

        match pending_for {
            // R79 gives a prompt held by the non-active player its own clock; the active player is
            // governed by the turn clock (§2.5).
            None => self.clear_prompt(fx),
            Some(holder) if holder == view.active => self.clear_prompt(fx),
            Some(holder) => {
                if Some(holder) != self.holder {
                    self.arm_prompt(holder, now, fx);
                }
            }
        }

        // R79's pause condition is the open prompt itself, not its clock: a prompt whose own clock
        // has already run out is still open, and the turn clock stays paused until it is answered.
        match pending_for {
            Some(holder) if holder != view.active => self.pause_turn(now, fx),
            _ => self.arm_turn(now, fx),
        }

        self.sync_sequence(view, now, fx);
    }

    /// §9.5: the disconnect grace is stored on the match so both clients show it; it runs beside the
    /// turn clock. `deadline` is one stored before a restart (R744): kept, but never later than a fresh
    /// window would end (R147).
    pub fn start_grace(&mut self, player: PlayerId, deadline: Option<i64>, now: i64, fx: &mut Fx) {
        if self.stopped {
            return;
        }
        // SPEC §11 R147: a second grace keeps the first deadline so a flapping socket cannot extend it.
        if self.grace[player].timer.is_some() {
            return;
        }
        // R744: a restart keeps the stored deadline up to a fresh window's end (R147). A past deadline
        // fires at once.
        let fresh = now + Self::grace_ms();
        let at = match deadline {
            None => fresh,
            Some(stored) => stored.min(fresh),
        };
        self.grace[player].deadline = Some(at);
        self.grace[player].timer = Some(fx.set(TimerKey::Grace { player }, now + (at - now).max(0)));
    }

    pub fn clear_grace(&mut self, player: PlayerId, fx: &mut Fx) {
        self.grace[player].cancel(TimerKey::Grace { player }, fx);
    }

    /// R776000: re-arms an expired clock `ms` from now if its action was not logged. Only re-arms
    /// what `sync` leaves expired and waiting; anything cancelling the clock cancels the retry.
    /// Each countdown holds one timer, so an expiry is never reported twice at once.
    pub fn retry(&mut self, expiry: ClockExpiry, ms: i64, now: i64, fx: &mut Fx) {
        if self.stopped {
            return;
        }
        match expiry {
            ClockExpiry::Turn { player } => {
                // A turn clock paused for the other seat's prompt runs out by itself once that prompt is
                // answered.
                if self.owner == Some(player)
                    && self.holder.is_none()
                    && self.turn.timer.is_none()
                    && self.remaining == 0
                {
                    self.remaining = ms;
                    self.arm_turn(now, fx);
                }
            }
            ClockExpiry::Prompt { player } => {
                if self.holder == Some(player) && self.prompt.timer.is_none() {
                    self.arm_prompt_in(player, ms, now, fx);
                }
            }
            ClockExpiry::Mulligan => {
                if self.mulligan_open && self.mulligan_expired && self.mulligan.timer.is_none() {
                    self.mulligan_expired = false;
                    self.mulligan.deadline = Some(now + ms);
                    self.open_mulligan(now, fx);
                }
            }
            // R147: `start_grace` keeps a countdown already running, so only an expired one re-arms.
            ClockExpiry::Grace { player } => self.start_grace(player, Some(now + ms), now, fx),
            ClockExpiry::Ceiling => {
                if self.ceiling.is_none() {
                    self.arm_ceiling_in(ms, now, fx);
                }
            }
            // R1563006: only a window still open on its sequence re-arms.
            ClockExpiry::Sequence { player } => {
                if self.sequence_first.is_some() && self.sequence.timer.is_none() {
                    self.arm_sequence_in(player, ms, now, fx);
                }
            }
        }
    }

    /// The timer `id` of `key` came due: which clock ran out, or `None` when that timer was replaced or
    /// taken back, or the clock is stopped. A countdown that ran out is spent: the host does not hear of
    /// it twice.
    pub fn fire(&mut self, key: TimerKey, id: u32) -> Option<ClockExpiry> {
        let expiry = match key {
            TimerKey::Turn => {
                if !self.turn.armed_by(id) {
                    return None;
                }
                self.turn.take_fired();
                self.turn.deadline = None;
                self.remaining = 0;
                ClockExpiry::Turn { player: self.owner? }
            }
            TimerKey::Prompt => {
                if !self.prompt.armed_by(id) {
                    return None;
                }
                self.prompt.take_fired();
                self.prompt.deadline = None;
                ClockExpiry::Prompt { player: self.holder? }
            }
            TimerKey::Mulligan => {
                if !self.mulligan.armed_by(id) {
                    return None;
                }
                self.mulligan.take_fired();
                self.mulligan_expired = true;
                ClockExpiry::Mulligan
            }
            TimerKey::Sequence => {
                if !self.sequence.armed_by(id) {
                    return None;
                }
                self.sequence.take_fired();
                self.sequence.deadline = None;
                ClockExpiry::Sequence {
                    player: self.sequence_player?,
                }
            }
            TimerKey::Grace { player } => {
                if !self.grace[player].armed_by(id) {
                    return None;
                }
                self.grace[player].take_fired();
                self.grace[player].deadline = None;
                ClockExpiry::Grace { player }
            }
            TimerKey::Ceiling => {
                if self.ceiling != Some(id) {
                    return None;
                }
                self.ceiling = None;
                ClockExpiry::Ceiling
            }
            TimerKey::Aim { .. }
            | TimerKey::Hover { .. }
            | TimerKey::BotOffer { .. }
            | TimerKey::BotReply { .. }
            | TimerKey::BotRetry => return None,
        };
        (!self.stopped).then_some(expiry)
    }

    /// Whether `key` is one of the clock's own timers.
    pub fn owns(key: TimerKey) -> bool {
        matches!(
            key,
            TimerKey::Turn
                | TimerKey::Prompt
                | TimerKey::Mulligan
                | TimerKey::Sequence
                | TimerKey::Ceiling
                | TimerKey::Grace { .. }
        )
    }

    /// Deadlines for `matches.clocks` and the protocol's `clock` message.
    pub fn snapshot(&self) -> MatchClocks {
        MatchClocks {
            turn_deadline: self.turn.deadline,
            // R268: the mulligan deadline is rendered as the prompt deadline held by both seats.
            prompt_deadline: if self.mulligan_open {
                self.mulligan.deadline
            } else {
                self.prompt.deadline
            },
            grace_deadline: PerPlayer::new(self.grace.p1.deadline, self.grace.p2.deadline),
            ceiling_at: self.ceiling_at,
        }
    }

    /// The acting deadline this player is under (R268, R79). Grace countdowns are read from
    /// `snapshot().grace_deadline` (deadline to return, not to act).
    pub fn remaining_for(&self, player: PlayerId, now: i64) -> Option<i64> {
        if self.stopped {
            return None;
        }
        // R268: one clock for both seats, the one that has answered included — it is waiting on it.
        if self.mulligan_open
            && let Some(deadline) = self.mulligan.deadline
        {
            return Some((deadline - now).max(0));
        }
        if self.holder == Some(player)
            && let Some(deadline) = self.prompt.deadline
        {
            return Some((deadline - now).max(0));
        }
        if self.owner != Some(player) {
            return None;
        }
        if let Some(deadline) = self.turn.deadline {
            return Some((deadline - now).max(0));
        }
        Some(self.remaining.max(0))
    }
}
