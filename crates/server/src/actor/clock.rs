//! The match clock (BUILD M7-T1, SPEC §9.5, R79).
//!
//! Deadlines on `tokio::time`, none visible to the engine (SPEC §9.3 keeps time out of `reduce`):
//! an expiry reports *which* clock ran out, turned into the action the engine sees:
//!  - turn clock (R79): active player's, paused while a prompt is open for non-active player;
//!  - holder prompt clock (R79): answers that prompt;
//!  - mulligan clock (R268): one deadline for both seats while open (R265);
//!  - disconnect grace per player (§9.5);
//!  - hard ceiling (R79), measured from `started_at`;
//!  - question sequence clock (R1563006, Meditative+ #20 KY's Competition).
//!
//! `now_ms()` reads tokio's clock (SURFACE §11.2). Every duration comes from `crate::config` (R79).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use jackioh_engine::{PLAYER_IDS, PerPlayer, PlayerId};

use crate::actor::contracts::{ClockExpiry, ClockView, CreateMatchClockInput, ExpiryHandler};
use crate::api::http::lock;
use crate::app::now_ms;
use crate::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, MS_PER_SECOND, MULLIGAN_CLOCK_SECONDS,
    PROMPT_CLOCK_SECONDS, TURN_CLOCK_SECONDS,
};
use crate::db::store::MatchClocks;
use jackioh_engine::config::KY_COMPETITION_SECONDS;

// Unit conversion, not configuration: the definition of a minute (a second's is `crate::config`'s),
// and every lifecycle *duration* comes from `crate::config` (R79) rather than from here.
const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;

static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// One scheduled callback. Dropping it cancels it. The callback is handed its timer's id, so a
/// callback already running when it was canceled can see that it is stale.
pub(crate) struct Timer {
    id: u64,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Timer {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn cancel(self) {
        drop(self);
    }

    /// The callback has run: let go of the timer without aborting the task that is running it.
    pub(crate) fn fired(mut self) {
        self.handle = None;
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

/// Runs `f` once `ms` milliseconds of tokio time have passed (at once for `ms <= 0`). Must be
/// called inside a tokio runtime.
pub(crate) fn after(ms: i64, f: impl FnOnce(u64) + Send + 'static) -> Timer {
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let wait = Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0));
    let handle = tokio::spawn(async move {
        tokio::time::sleep(wait).await;
        f(id);
    });
    Timer {
        id,
        handle: Some(handle),
    }
}

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

struct Countdown {
    timer: Option<Timer>,
    deadline: Option<i64>,
}

fn idle() -> Countdown {
    Countdown {
        timer: None,
        deadline: None,
    }
}

impl Countdown {
    fn cancel(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
        self.deadline = None;
    }

    fn armed_by(&self, id: u64) -> bool {
        self.timer.as_ref().map(Timer::id) == Some(id)
    }

    fn take_fired(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.fired();
        }
    }
}

struct ClockState {
    /// The `Arc` this state lives in, for the timers' callbacks.
    me: Weak<Mutex<ClockState>>,
    on_expire: ExpiryHandler,

    turn_ms: i64,
    prompt_ms: i64,
    mulligan_ms: i64,
    grace_ms: i64,
    ceiling_at: i64,

    stopped: bool,

    // The turn clock. `remaining` is the authority while it is paused: R79's pause resumes with the
    // time the clock had left, not from full.
    owner: Option<PlayerId>,
    turn_key: Option<String>,
    remaining: i64,
    turn: Countdown,

    // Non-active holder's prompt clock: `holder` outlives timer so later `sync` cannot re-arm.
    holder: Option<PlayerId>,
    prompt: Countdown,

    // R268: mulligan deadline is set on first sight and never moved, so second seat gets no
    // extra time. Expired outlives timer so nothing re-arms over an open window.
    // A rebuilt actor (§9.5) arms a fresh full deadline; only disconnect grace reads its stored
    // deadline back (R744).
    mulligan: Countdown,
    mulligan_open: bool,
    mulligan_expired: bool,

    grace: PerPlayer<Countdown>,

    // R1563006: the question sequence's clock, armed at its first question (`sequence_first`, that
    // question's prompt id) and kept over the questions after it, so each answer spends from one window.
    sequence_ms: i64,
    sequence: Countdown,
    sequence_first: Option<String>,

    // R79: match ceiling timer measured from `started_at`, inherited across rebuilds.
    ceiling: Option<Timer>,
}

/// A timer's callback: under the lock, `update` brings the clock up to date and names the expiry to
/// report, if any (nothing once stopped). The report goes out after the lock is let go, so the
/// actor's handler never runs inside the clock.
fn fire(me: &Weak<Mutex<ClockState>>, update: impl FnOnce(&mut ClockState) -> Option<ClockExpiry>) {
    let Some(state) = me.upgrade() else { return };
    let report = {
        let mut clock = lock(&state);
        match update(&mut clock) {
            Some(expiry) if !clock.stopped => Some((expiry, clock.on_expire.clone())),
            _ => None,
        }
    };
    if let Some((expiry, on_expire)) = report {
        on_expire(expiry);
    }
}

impl ClockState {
    /// R79: pausing banks the time left; arming spends it.
    fn pause_turn(&mut self) {
        if self.turn.timer.is_none() {
            return;
        }
        let now = now_ms();
        self.remaining = (self.turn.deadline.unwrap_or(now) - now).max(0);
        self.turn.cancel();
    }

    fn arm_turn(&mut self) {
        if self.stopped || self.turn.timer.is_some() {
            return;
        }
        let Some(player) = self.owner else { return };
        let ms = self.remaining.max(0);
        self.turn.deadline = Some(now_ms() + ms);
        let me = self.me.clone();
        self.turn.timer = Some(after(ms, move |id| {
            fire(&me, |clock| {
                if !clock.turn.armed_by(id) {
                    return None;
                }
                clock.turn.take_fired();
                clock.turn.deadline = None;
                clock.remaining = 0;
                Some(ClockExpiry::Turn { player })
            });
        }));
    }

    fn arm_prompt(&mut self, player: PlayerId) {
        self.arm_prompt_in(player, self.prompt_ms);
    }

    /// Prompt clock for `player`, running out `ms` from now (`prompt_ms` or R776000 retry delay).
    fn arm_prompt_in(&mut self, player: PlayerId, ms: i64) {
        self.prompt.cancel();
        self.holder = Some(player);
        self.prompt.deadline = Some(now_ms() + ms);
        let me = self.me.clone();
        self.prompt.timer = Some(after(ms, move |id| {
            fire(&me, |clock| {
                if !clock.prompt.armed_by(id) {
                    return None;
                }
                clock.prompt.take_fired();
                clock.prompt.deadline = None;
                Some(ClockExpiry::Prompt { player })
            });
        }));
    }

    fn clear_prompt(&mut self) {
        self.prompt.cancel();
        self.holder = None;
    }

    /// R268: one deadline for the whole window, armed the first time it is seen.
    fn open_mulligan(&mut self) {
        self.mulligan_open = true;
        let now = now_ms();
        let deadline = *self.mulligan.deadline.get_or_insert(now + self.mulligan_ms);
        if self.stopped || self.mulligan.timer.is_some() || self.mulligan_expired {
            return;
        }
        let me = self.me.clone();
        self.mulligan.timer = Some(after((deadline - now).max(0), move |id| {
            fire(&me, |clock| {
                if !clock.mulligan.armed_by(id) {
                    return None;
                }
                clock.mulligan.take_fired();
                clock.mulligan_expired = true;
                Some(ClockExpiry::Mulligan)
            });
        }));
    }

    /// The window closed (both seats answered): its clock goes, and its deadline stays spent.
    fn close_mulligan(&mut self) {
        self.mulligan_open = false;
        if let Some(timer) = self.mulligan.timer.take() {
            timer.cancel();
        }
    }

    /// R1563006: the sequence clock for `player`, running out `ms` from now.
    fn arm_sequence_in(&mut self, player: PlayerId, ms: i64) {
        self.sequence.cancel();
        self.sequence.deadline = Some(now_ms() + ms);
        let me = self.me.clone();
        self.sequence.timer = Some(after(ms, move |id| {
            fire(&me, |clock| {
                if !clock.sequence.armed_by(id) {
                    return None;
                }
                clock.sequence.take_fired();
                clock.sequence.deadline = None;
                Some(ClockExpiry::Sequence { player })
            });
        }));
    }

    /// R1563006: a sequence's first question (streak 0, a prompt id this clock has not started on) arms
    /// a fresh window; a later question keeps it; none open clears it. A question met mid-sequence with
    /// no window (a rebuilt actor) arms a fresh one, as the turn clock does.
    fn sync_sequence(&mut self, view: &ClockView) {
        let Some(question) = &view.sequence else {
            self.sequence.cancel();
            self.sequence_first = None;
            return;
        };
        let first =
            question.streak == 0 && self.sequence_first.as_deref() != Some(question.prompt_id.as_str());
        if first || self.sequence_first.is_none() {
            self.sequence_first = Some(question.prompt_id.clone());
            self.arm_sequence_in(question.player, self.sequence_ms);
        }
    }

    fn stop(&mut self) {
        self.stopped = true;
        self.turn.cancel();
        self.clear_prompt();
        self.close_mulligan();
        self.sequence.cancel();
        for player in PLAYER_IDS {
            self.grace[player].cancel();
        }
        if let Some(timer) = self.ceiling.take() {
            timer.cancel();
        }
    }

    fn arm_ceiling(&mut self) {
        self.arm_ceiling_in(self.ceiling_at - now_ms());
    }

    /// Ceiling timer, running out `ms` from now (remainder or R776000 retry delay).
    fn arm_ceiling_in(&mut self, ms: i64) {
        let me = self.me.clone();
        self.ceiling = Some(after(ms.max(0), move |id| {
            fire(&me, |clock| {
                if clock.ceiling.as_ref().map(Timer::id) != Some(id) {
                    return None;
                }
                if let Some(timer) = clock.ceiling.take() {
                    timer.fired();
                }
                Some(ClockExpiry::Ceiling)
            });
        }));
    }

    fn sync(&mut self, view: &ClockView) {
        // Once a match is over it stays over: a late `sync` must never re-arm a deadline.
        if self.stopped {
            return;
        }
        if view.over {
            self.stop();
            return;
        }

        // R79: "The turn clock belongs to the active player". Both halves of the key matter — the
        // turn number alone would miss a replaced turn, and the active player alone would miss a
        // turn that somehow stayed with the same player.
        let key = format!("{}:{}", view.turn, view.active);
        if self.turn_key.as_deref() != Some(key.as_str()) {
            self.turn_key = Some(key);
            self.owner = Some(view.active);
            self.turn.cancel();
            self.remaining = self.turn_ms;
        }

        let pending_for = view.pending_for;

        // R265, R268: both mulligans open at once. Setup is nobody's turn (§2.1), so neither the turn
        // clock nor a prompt clock runs; the one mulligan deadline covers both seats. The turn clock is
        // paused, not reset, so a question a card asks during setup (§2.4) is still timed by R79, and
        // turn 1 starts a fresh turn clock when the turn key changes.
        if pending_for.is_none() && !view.mulligan_owed.is_empty() {
            self.clear_prompt();
            self.pause_turn();
            self.open_mulligan();
            return;
        }
        self.close_mulligan();

        match pending_for {
            // R79 gives prompt held by non-active player its own clock; active player is governed
            // by turn clock (§2.5).
            None => self.clear_prompt(),
            Some(holder) if holder == view.active => self.clear_prompt(),
            Some(holder) => {
                if Some(holder) != self.holder {
                    self.arm_prompt(holder);
                }
            }
        }

        // R79's pause condition is the open prompt itself, not its clock: a prompt whose own clock
        // has already run out is still open, and the turn clock stays paused until it is answered.
        match pending_for {
            Some(holder) if holder != view.active => self.pause_turn(),
            _ => self.arm_turn(),
        }

        self.sync_sequence(view);
    }

    // §9.5: disconnect grace stored on match so both clients show it; runs beside turn clock.
    fn start_grace(&mut self, player: PlayerId, deadline: Option<i64>) {
        if self.stopped {
            return;
        }
        // SPEC §11 R147: second grace keeps first deadline so flapping socket cannot extend grace.
        if self.grace[player].timer.is_some() {
            return;
        }
        // R744: restart keeps stored deadline up to fresh window end (R147). Past deadline fires at once.
        let now = now_ms();
        let fresh = now + self.grace_ms;
        let at = match deadline {
            None => fresh,
            Some(stored) => stored.min(fresh),
        };
        self.grace[player].deadline = Some(at);
        let me = self.me.clone();
        self.grace[player].timer = Some(after((at - now).max(0), move |id| {
            fire(&me, |clock| {
                if !clock.grace[player].armed_by(id) {
                    return None;
                }
                clock.grace[player].take_fired();
                clock.grace[player].deadline = None;
                Some(ClockExpiry::Grace { player })
            });
        }));
    }

    fn clear_grace(&mut self, player: PlayerId) {
        self.grace[player].cancel();
    }

    /// R776000: re-arm an expired clock `ms` from now if its action was not logged. Only re-arms
    /// what `sync` leaves expired and waiting; anything canceling the clock cancels the retry.
    /// Each countdown holds one timer, so an expiry is never reported twice at once.
    fn retry(&mut self, expiry: ClockExpiry, ms: i64) {
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
                    self.arm_turn();
                }
            }
            ClockExpiry::Prompt { player } => {
                if self.holder == Some(player) && self.prompt.timer.is_none() {
                    self.arm_prompt_in(player, ms);
                }
            }
            ClockExpiry::Mulligan => {
                if self.mulligan_open && self.mulligan_expired && self.mulligan.timer.is_none() {
                    self.mulligan_expired = false;
                    self.mulligan.deadline = Some(now_ms() + ms);
                    self.open_mulligan();
                }
            }
            // R147: `start_grace` keeps a countdown already running, so only an expired one re-arms.
            ClockExpiry::Grace { player } => self.start_grace(player, Some(now_ms() + ms)),
            ClockExpiry::Ceiling => {
                if self.ceiling.is_none() {
                    self.arm_ceiling_in(ms);
                }
            }
            // R1563006: only a window still open on its sequence re-arms.
            ClockExpiry::Sequence { player } => {
                if self.sequence_first.is_some() && self.sequence.timer.is_none() {
                    self.arm_sequence_in(player, ms);
                }
            }
        }
    }

    fn snapshot(&self) -> MatchClocks {
        MatchClocks {
            turn_deadline: self.turn.deadline,
            // R268: mulligan deadline is rendered as the prompt deadline held by both seats.
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
    fn remaining_for(&self, player: PlayerId) -> Option<i64> {
        if self.stopped {
            return None;
        }
        let now = now_ms();
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

/// A cheap handle (`Clone`) on one match's clock. Dropping the last handle cancels every outstanding
/// timer.
#[derive(Clone)]
pub struct MatchClock {
    state: Arc<Mutex<ClockState>>,
}

impl std::fmt::Debug for MatchClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchClock")
            .field("clocks", &self.snapshot())
            .finish()
    }
}

impl MatchClock {
    /// Called once after `begin_game` and again after every state change.
    pub fn sync(&self, view: &ClockView) {
        lock(&self.state).sync(view);
    }

    /// §9.5: the countdown is stored on the match so both clients show it. `deadline` is one stored
    /// before a restart (R744): kept, but never later than a fresh window would end (R147).
    pub fn start_grace(&self, player: PlayerId, deadline: Option<i64>) {
        lock(&self.state).start_grace(player, deadline);
    }

    pub fn clear_grace(&self, player: PlayerId) {
        lock(&self.state).clear_grace(player);
    }

    /// R776000: the clock that ran out runs out once more `ms` from now (see `ClockState::retry`).
    pub fn retry(&self, expiry: ClockExpiry, ms: i64) {
        lock(&self.state).retry(expiry, ms);
    }

    /// Deadlines for `matches.clocks` and the protocol's `clock` message.
    pub fn snapshot(&self) -> MatchClocks {
        lock(&self.state).snapshot()
    }

    /// Milliseconds left on the deadline that currently belongs to this player, or `None`.
    pub fn remaining_for(&self, player: PlayerId) -> Option<i64> {
        lock(&self.state).remaining_for(player)
    }

    /// Cancels every outstanding timer. Idempotent.
    pub fn stop(&self) {
        lock(&self.state).stop();
    }
}

/// Must be called inside a tokio runtime (it arms the ceiling at once).
pub fn create_match_clock(input: CreateMatchClockInput) -> MatchClock {
    let CreateMatchClockInput {
        started_at,
        on_expire,
    } = input;
    let state = Arc::new_cyclic(|me| {
        Mutex::new(ClockState {
            me: me.clone(),
            on_expire,
            turn_ms: TURN_CLOCK_SECONDS * MS_PER_SECOND,
            prompt_ms: PROMPT_CLOCK_SECONDS * MS_PER_SECOND,
            mulligan_ms: MULLIGAN_CLOCK_SECONDS * MS_PER_SECOND,
            grace_ms: DISCONNECT_GRACE_SECONDS * MS_PER_SECOND,
            ceiling_at: match_ceiling_at(started_at),
            stopped: false,
            owner: None,
            turn_key: None,
            remaining: TURN_CLOCK_SECONDS * MS_PER_SECOND,
            turn: idle(),
            holder: None,
            prompt: idle(),
            mulligan: idle(),
            mulligan_open: false,
            mulligan_expired: false,
            grace: PerPlayer::new(idle(), idle()),
            sequence_ms: i64::from(KY_COMPETITION_SECONDS) * MS_PER_SECOND,
            sequence: idle(),
            sequence_first: None,
            ceiling: None,
        })
    });
    lock(&state).arm_ceiling();
    MatchClock { state }
}
