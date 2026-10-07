//! The match clock (BUILD M7-T1, SPEC §9.5, R79). Port of `apps/server/src/match/clock.ts`.
//!
//! Five independent deadlines, all of them scheduled on `tokio::time` (TS: the injected `Timers`
//! port) and none of them visible to the engine — SPEC §9.3 keeps time out of `reduce`, so an expiry
//! here only reports *which* clock ran out and the actor turns that into the `timeout`,
//! `disconnectExpired` or `ceilingReached` action that the engine does see:
//!
//!  - the turn clock (R79: `TURN_CLOCK_SECONDS`), which belongs to the active player, is reset when
//!    the turn changes, and **pauses while a prompt is open for the non-active player**;
//!  - that non-active holder's prompt clock (R79: `PROMPT_CLOCK_SECONDS`), whose expiry answers only
//!    that prompt;
//!  - the mulligan clock (R268: `MULLIGAN_CLOCK_SECONDS`), one deadline for both seats while both
//!    mulligans are open (R265). Setup is nobody's turn, so the turn clock does not run under it;
//!    it is reported as the prompt deadline, which it is for both seats at once;
//!  - a disconnect grace countdown per player (§9.5: `DISCONNECT_GRACE_SECONDS`), which does **not**
//!    pause the turn clock — §9.5: "The clock keeps running while a player is disconnected";
//!  - the hard wall-clock ceiling (R79: `MATCH_CEILING_MINUTES`), measured from `started_at`.
//!
//! Nothing in this file reads the system clock directly: `now_ms()` (`app::now_ms`) reads tokio's clock, which is
//! what makes the M7-T1 tests exact rather than approximate (`tokio::time::pause()` and `advance()`
//! replace TS's manual timers, SURFACE §11.2).
//!
//! Every number comes from `crate::config` (`src/config.ts`), so R79's values are stated once.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use jackioh_engine::{PLAYER_IDS, PerPlayer, PlayerId};

use crate::api::http::lock;
use crate::actor::contracts::{ClockExpiry, ClockView, CreateMatchClockInput, ExpiryHandler};
use crate::app::now_ms;
use crate::config::{
    DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES, MULLIGAN_CLOCK_SECONDS, PROMPT_CLOCK_SECONDS, TURN_CLOCK_SECONDS,
};
use crate::db::store::MatchClocks;

// Unit conversions, not configuration: these two are the definitions of a second and a minute,
// and every lifecycle *duration* comes from `crate::config` (R79) rather than from here.
const MS_PER_SECOND: i64 = 1000;
const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;

// ---------------------------------------------------------------------------
// Time (TS's `Timers` port: `now()` and `after(ms, fn) → { cancel }`)
// ---------------------------------------------------------------------------

static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// TS `Timer`: one scheduled callback. Dropping it cancels it (TS `cancel()`), so a countdown
/// cancels by letting go of its timer. The callback is handed its timer's id, so a callback that
/// was already running when it was cancelled can see that it is stale.
pub(crate) struct Timer {
    id: u64,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Timer {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// TS `timer.cancel()`.
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

/// TS `timers.after(ms, fn)`: runs `f` once `ms` milliseconds of tokio time have passed (at once
/// for `ms <= 0`). Must be called inside a tokio runtime.
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

// ---------------------------------------------------------------------------
// The match row's clocks
// ---------------------------------------------------------------------------

/// R79: the ceiling is one deadline measured from the moment the match started.
pub fn match_ceiling_at(started_at: i64) -> i64 {
    started_at + MATCH_CEILING_MINUTES as i64 * MS_PER_MINUTE
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

// ---------------------------------------------------------------------------
// The clock
// ---------------------------------------------------------------------------

struct Countdown {
    timer: Option<Timer>,
    deadline: Option<i64>,
}

/// TS `idle()`.
fn idle() -> Countdown {
    Countdown {
        timer: None,
        deadline: None,
    }
}

impl Countdown {
    /// TS `cancel(countdown)`.
    fn cancel(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
        self.deadline = None;
    }

    /// Whether the timer with this id is the one this countdown holds now.
    fn armed_by(&self, id: u64) -> bool {
        self.timer.as_ref().map(Timer::id) == Some(id)
    }

    /// Its timer has run: `timer = null` without cancelling the running callback.
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

    // The non-active holder's prompt clock. `holder` outlives its timer on purpose: the prompt is
    // still open after the clock runs out (the actor answers it with `timeout`), and a later `sync`
    // that still reports the same holder must not re-arm a second 30 s window.
    holder: Option<PlayerId>,
    prompt: Countdown,

    // R268: the mulligan clock. `deadline` is set the first time the window is seen and never moved
    // again — not when one seat answers, and not when a later `sync` reports the window still open —
    // so the seat that answers second gets no more time than the one that answered first. Like the
    // prompt clock's `holder`, `expired` outlives the timer: the window is still open after the clock
    // runs out (the actor times the owing seats out), and nothing re-arms a second window over it.
    //
    // R268: a rebuilt actor (§9.5) builds a new clock, which sees the window for the first time and
    // arms a fresh full deadline, exactly as the turn clock restarts from full on a rebuild. A
    // crash in the window therefore gives both seats at most one more `MULLIGAN_CLOCK_SECONDS`;
    // reading the stored `promptDeadline` back instead would be stricter, but the stored row is
    // written after the fact and may be stale, and only the disconnect grace reads its stored
    // deadline back (R744).
    mulligan: Countdown,
    mulligan_open: bool,
    mulligan_expired: bool,

    grace: PerPlayer<Countdown>,

    // R79: one timer for the whole match, measured from `started_at` — so a clock rebuilt after a
    // crash inherits the ceiling the match already had rather than starting a fresh hour.
    ceiling: Option<Timer>,
}

/// A timer's callback: under the lock, `update` brings the clock up to date and names the expiry to
/// report, if any (TS's `report`: nothing once stopped). The report goes out after the lock is let
/// go, so the actor's handler never runs inside the clock.
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
        self.prompt.cancel();
        self.holder = Some(player);
        self.prompt.deadline = Some(now_ms() + self.prompt_ms);
        let me = self.me.clone();
        self.prompt.timer = Some(after(self.prompt_ms, move |id| {
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

    fn stop(&mut self) {
        self.stopped = true;
        self.turn.cancel();
        self.clear_prompt();
        self.close_mulligan();
        for player in PLAYER_IDS {
            self.grace[player].cancel();
        }
        if let Some(timer) = self.ceiling.take() {
            timer.cancel();
        }
    }

    fn arm_ceiling(&mut self) {
        let me = self.me.clone();
        self.ceiling = Some(after((self.ceiling_at - now_ms()).max(0), move |id| {
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
        // paused rather than reset, so a question a card asks during setup on either side of the
        // window (a cast-on-draw card in the deal or in a replacement draw, §2.4) is still timed by
        // R79 as before, and turn 1 starts a fresh turn clock when the turn key changes.
        if pending_for.is_none() && !view.mulligan_owed.is_empty() {
            self.clear_prompt();
            self.pause_turn();
            self.open_mulligan();
            return;
        }
        self.close_mulligan();

        match pending_for {
            // R79 gives its own clock to a prompt held by the *non-active* player. A prompt the active
            // player owes is governed by their turn clock instead (§2.5: "When the active player's
            // turn clock runs out, their open prompts are answered by the AI policy and the turn
            // ends"), so it gets no second, shorter deadline.
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
    }

    // §9.5: "Disconnect grace (60 s) and concede end the match as a loss" and "the grace countdown
    // is stored on the match so both clients show it". It runs beside the turn clock, never
    // instead of it.
    fn start_grace(&mut self, player: PlayerId, deadline: Option<i64>) {
        if self.stopped {
            return;
        }
        // SPEC §11 R147: "A second disconnect grace starting before the first is cleared keeps the
        // first deadline, so a socket that flaps cannot extend its own grace indefinitely and stall
        // the match."
        if self.grace[player].timer.is_some() {
            return;
        }
        // R744: a deadline stored before a restart is kept but held to the end of a fresh window, so a
        // rebuild can shorten a grace and never extend one (R147). One already past fires at once.
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

    fn snapshot(&self) -> MatchClocks {
        MatchClocks {
            turn_deadline: self.turn.deadline,
            // R268: during the window the mulligan deadline is the prompt deadline both clients render —
            // it is a prompt deadline, held by both seats at once. The two never run together, so the
            // stored `MatchClocks` keeps its shape.
            prompt_deadline: if self.mulligan_open { self.mulligan.deadline } else { self.prompt.deadline },
            grace_deadline: PerPlayer::new(self.grace.p1.deadline, self.grace.p2.deadline),
            ceiling_at: self.ceiling_at,
        }
    }

    /// The acting deadline this player is under. The mulligan clock, while it runs, is both seats'
    /// (R268). Otherwise a prompt clock they hold outranks the turn clock, and a paused turn clock
    /// still reports its banked remainder, which is the frozen number the clients render while
    /// R79's pause is in effect. The grace countdowns are read from `snapshot().grace_deadline`: they
    /// are not a deadline to act by, they are a deadline to come back by, and both clients show them
    /// for both players.
    fn remaining_for(&self, player: PlayerId) -> Option<i64> {
        if self.stopped {
            return None;
        }
        let now = now_ms();
        // R268: one clock for both seats, the one that has answered included — it is waiting on it.
        if self.mulligan_open {
            if let Some(deadline) = self.mulligan.deadline {
                return Some((deadline - now).max(0));
            }
        }
        if self.holder == Some(player) {
            if let Some(deadline) = self.prompt.deadline {
                return Some((deadline - now).max(0));
            }
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

/// TS `MatchClock`: a cheap handle (`Clone`) on one match's clock. Dropping the last handle cancels
/// every outstanding timer.
#[derive(Clone)]
pub struct MatchClock {
    state: Arc<Mutex<ClockState>>,
}

impl std::fmt::Debug for MatchClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchClock").field("clocks", &self.snapshot()).finish()
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

/// TS `createMatchClock`. Must be called inside a tokio runtime (it arms the ceiling at once).
pub fn create_match_clock(input: CreateMatchClockInput) -> MatchClock {
    let CreateMatchClockInput { started_at, on_expire } = input;
    let state = Arc::new_cyclic(|me| {
        Mutex::new(ClockState {
            me: me.clone(),
            on_expire,
            turn_ms: TURN_CLOCK_SECONDS as i64 * MS_PER_SECOND,
            prompt_ms: PROMPT_CLOCK_SECONDS as i64 * MS_PER_SECOND,
            mulligan_ms: MULLIGAN_CLOCK_SECONDS as i64 * MS_PER_SECOND,
            grace_ms: DISCONNECT_GRACE_SECONDS as i64 * MS_PER_SECOND,
            ceiling_at: match_ceiling_at(started_at),
            stopped: false,
            owner: None,
            turn_key: None,
            remaining: TURN_CLOCK_SECONDS as i64 * MS_PER_SECOND,
            turn: idle(),
            holder: None,
            prompt: idle(),
            mulligan: idle(),
            mulligan_open: false,
            mulligan_expired: false,
            grace: PerPlayer::new(idle(), idle()),
            ceiling: None,
        })
    });
    lock(&state).arm_ceiling();
    MatchClock { state }
}
