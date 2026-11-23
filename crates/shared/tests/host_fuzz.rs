//! The two match host machines on seeded random events (#4392; SPEC §9.3, §9.5, §13, R79, R744, R776000,
//! R1253000, R1452, R4013000): `DuelHost` and `DuosHost` played the way a server's actor plays them, with a
//! harness standing in for the host. It keeps the timers the machine sets and takes back, a virtual clock it
//! moves forward, the rows the machine asks it to append (written, refused, or taken by another host) and
//! the last clocks it was asked to persist, and it rebuilds a host from the written rows the way the server
//! does (`match_actor.rs`, `duos/actor.rs`). Each step draws one event from a weighted mix, and after it the
//! harness checks:
//!
//!  1. the log rebuilds the game: whenever no write is outstanding, folding the written rows gives the
//!     game the host holds;
//!  2. the log is well formed: seqs run from 1 with no gap, and no nonce is in two rows;
//!  3. a stale timer changes nothing: firing an id the host no longer holds gives no output and leaves
//!     the host as it was;
//!  4. a match ends once: one `Record`, `Voided` or `CeilingDraw` after the game is over, none before it,
//!     and no row after it;
//!  5. a restart holds the game: a host rebuilt from the log has the game and the next seq of the one it
//!     replaces;
//!
//! and that no output is `Panicked`. Every draw is the engine's `Rng` (CLAUDE.md rule 4 binds a pure
//! crate's tests too), so a failure's seed alone reproduces it: set `REPRO` and run the `repro_one_seed` test.
//!
//! No module or test here is named `fuzz`, `selfplay` or `positions`: the `rust (test tools, wasm)` job
//! skips tests with those names (`.github/ci/rust-test.toml`).

use std::collections::VecDeque;
use std::fmt;

use indexmap::IndexMap;

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::PlayerId::{self, P1, P2};
use jackioh_engine::duos::{DuosState, duos_fold, duos_fold_from, duos_hash};
use jackioh_engine::wire::{DuosAction, DuosActionBody, DuosCreateOptions, DuosSeat};
use jackioh_engine::{
    Action, ActionBody, Aim, AimEnd, DECK_SIZE, GameState, PerPlayer, PlayerView, ReplayInput, Rng,
    hash_state,
};
use jackioh_shared::bots::decide::BotChoice;
use jackioh_shared::config::PersonaName;
use jackioh_shared::host::bot::{BotSeat, r188_forbidden};
use jackioh_shared::host::clock::{MatchClocks, initial_clocks};
use jackioh_shared::host::duos::DuosAck;
use jackioh_shared::host::duos::{DuosDecide, DuosLetGo, ai_forbidden};
use jackioh_shared::host::snapshot::snapshot;
use jackioh_shared::host::timer::{TimerCommand, TimerKey};
use jackioh_shared::host::{
    ActionBy, DecideRequest, DuelEvent, DuelHost, DuelOutput, DuelSetup, DuosHost, DuosHostEvent,
    DuosHostOutput, DuosSetup, DuosTeamB, LetGo, LogOutcome, SERVER_NONCE_PREFIX, SeatMessage,
};

const EPOCH: i64 = 1_700_000_000_000;
const SECOND: i64 = 1000;

/// How many matches each machine plays on every pull request.
const HOST_FUZZ_SEEDS: u64 = 16;
/// How many each plays in the daily sweep.
const HOST_FUZZ_DAILY_SEEDS: u64 = 400;
/// The most steps a match is played for.
const HOST_FUZZ_MAX_STEPS: usize = 600;
/// How many more steps a match that has ended is played for: nothing may change.
const HOST_FUZZ_AFTER_END_STEPS: usize = 30;
/// Every this many steps, the incremental fold is checked against a fold of the whole log from its start.
const HOST_FUZZ_FULL_FOLD_EVERY: usize = 25;
/// The chance, in percent, that an `Append` is left unanswered until a later step.
const HOST_FUZZ_HOLD_PERCENT: u32 = 25;
/// Every this many 1v1 matches, one seats a ladder bot at P2.
const HOST_FUZZ_BOT_EVERY: u64 = 3;
/// How many appends in a hundred a restart finds landed, when it comes while one is outstanding.
const HOST_FUZZ_LANDED_PERCENT: u32 = 50;
/// The chance, in percent, that a bot's answer is for the request before the newest one.
const HOST_FUZZ_STALE_ANSWER_PERCENT: u32 = 15;
/// The chance, in percent, that a legal action drawn is a concession or a draw offer (a quick end).
const HOST_FUZZ_CONCEDE_PERCENT: u32 = 1;
/// Set to rerun one failing seed: `repro_one_seed` plays it alone.
const REPRO: Option<(Machine, u64)> = None;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Machine {
    Duel,
    Duos,
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Machine::Duel => "duel",
            Machine::Duos => "duos",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Invariant {
    LogRebuilds,
    LogWellFormed,
    StaleTimer,
    EndsOnce,
    RestartHolds,
    Panicked,
    Floor,
    PushedViews,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Violation {
    invariant: Invariant,
    detail: String,
}

fn violation(invariant: Invariant, detail: impl Into<String>) -> Violation {
    Violation {
        invariant,
        detail: detail.into(),
    }
}

/// What a failed match reports: where, and which check.
#[derive(Debug)]
struct Failure {
    machine: Machine,
    seed: u64,
    step: usize,
    event: String,
    violation: Violation,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let machine = match self.machine {
            Machine::Duel => "Duel",
            Machine::Duos => "Duos",
        };
        write!(
            f,
            "host fuzz: {} seed {} step {} event {}: {:?}: {}; rerun: set REPRO = Some((Machine::{machine}, {})) in crates/shared/tests/host_fuzz.rs",
            self.machine,
            self.seed,
            self.step,
            self.event,
            self.violation.invariant,
            self.violation.detail,
            self.seed
        )
    }
}

// ---------------------------------------------------------------------------------------------------
// The draws

struct Draw(Rng);

impl Draw {
    fn new(machine: Machine, n: u64) -> Draw {
        Draw(Rng::new(&format!("host-fuzz:{machine}:{n}"), 0))
    }

    /// A number in `0..n`; `n` must not be 0.
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.0.int(i32::try_from(n).expect("a small range"))).expect("a draw is not negative")
    }

    fn chance(&mut self, percent: u32) -> bool {
        self.0.int(100) < i32::try_from(percent).expect("a percent")
    }

    fn weighted<T: Copy>(&mut self, table: &[(u32, T)]) -> T {
        let total: u32 = table.iter().map(|(weight, _)| *weight).sum();
        let mut roll = self.0.int(i32::try_from(total).expect("a small total"));
        for (weight, value) in table {
            let weight = i32::try_from(*weight).expect("a small weight");
            if roll < weight {
                return *value;
            }
            roll -= weight;
        }
        table.last().expect("a table with a row").1
    }

    fn pick<'a, T>(&mut self, list: &'a [T]) -> Option<&'a T> {
        self.0.pick(list)
    }
}

// ---------------------------------------------------------------------------------------------------
// The timers a host keeps

/// What the machine has asked for: the timers it holds, by key, and every one it ever set.
#[derive(Default)]
struct Timers {
    live: IndexMap<TimerKey, (u32, i64)>,
    seen: Vec<(TimerKey, u32)>,
}

impl Timers {
    fn apply(&mut self, command: TimerCommand) {
        match command {
            TimerCommand::Set { key, id, at } => {
                self.live.shift_remove(&key);
                self.live.insert(key, (id, at));
                self.seen.push((key, id));
            }
            TimerCommand::Cancel { key, id } => {
                if self.live.get(&key).is_some_and(|(held, _)| *held == id) {
                    self.live.shift_remove(&key);
                }
            }
        }
    }

    /// The timer due first (ties in the order they were set), taken off the host's list.
    fn take_next(&mut self) -> Option<(TimerKey, u32, i64)> {
        let (key, (id, at)) = self
            .live
            .iter()
            .enumerate()
            .min_by_key(|(order, (_, (_, at)))| (*at, *order))
            .map(|(_, (key, held))| (*key, *held))?;
        self.live.shift_remove(&key);
        Some((key, id, at))
    }

    /// The timers the machine set that it no longer holds: fired, replaced or taken back.
    fn stale(&self) -> Vec<(TimerKey, u32)> {
        self.seen
            .iter()
            .filter(|(key, id)| self.live.get(key).is_none_or(|(held, _)| held != id))
            .copied()
            .collect()
    }

    /// A rebuilt host numbers its timers again.
    fn clear(&mut self) {
        self.live.clear();
        self.seen.clear();
    }
}

// ---------------------------------------------------------------------------------------------------
// What a run reached

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Coverage {
    matches: u64,
    steps: u64,
    restarts: u64,
    failed_writes: u64,
    diverged_writes: u64,
    held_writes: u64,
    stale_timers: u64,
    bot_matches: u64,
    endings: u64,
    nonce_resends: u64,
    nonce_copies: u64,
    disconnects: u64,
    timers_fired: u64,
    decisions: u64,
}

impl Coverage {
    fn add(&mut self, other: &Coverage) {
        self.matches += other.matches;
        self.steps += other.steps;
        self.restarts += other.restarts;
        self.failed_writes += other.failed_writes;
        self.diverged_writes += other.diverged_writes;
        self.held_writes += other.held_writes;
        self.stale_timers += other.stale_timers;
        self.bot_matches += other.bot_matches;
        self.endings += other.endings;
        self.nonce_resends += other.nonce_resends;
        self.nonce_copies += other.nonce_copies;
        self.disconnects += other.disconnects;
        self.timers_fired += other.timers_fired;
        self.decisions += other.decisions;
    }

    fn named(&self) -> [(&'static str, u64); 14] {
        [
            ("matches", self.matches),
            ("steps", self.steps),
            ("restarts", self.restarts),
            ("failed_writes", self.failed_writes),
            ("diverged_writes", self.diverged_writes),
            ("held_writes", self.held_writes),
            ("stale_timers", self.stale_timers),
            ("bot_matches", self.bot_matches),
            ("endings", self.endings),
            ("nonce_resends", self.nonce_resends),
            ("nonce_copies", self.nonce_copies),
            ("disconnects", self.disconnects),
            ("timers_fired", self.timers_fired),
            ("decisions", self.decisions),
        ]
    }
}

/// The least a run must reach, counter by counter, in the order of `Coverage::named`.
type Floors = Coverage;

// ---------------------------------------------------------------------------------------------------
// The checks, each on plain values so a test can break one by hand

/// Invariant 2: seqs are 1, 2, 3, … and no nonce is in two rows.
fn check_log_well_formed(rows: &[(i64, &str)]) -> Result<(), Violation> {
    for (index, (seq, nonce)) in rows.iter().enumerate() {
        let expected = i64::try_from(index).expect("a short log") + 1;
        if *seq != expected {
            return Err(violation(
                Invariant::LogWellFormed,
                format!("row {index} has seq {seq}, expected {expected}"),
            ));
        }
        if rows[..index].iter().any(|(_, earlier)| earlier == nonce) {
            return Err(violation(
                Invariant::LogWellFormed,
                format!("nonce {nonce} is in two rows (seq {seq})"),
            ));
        }
    }
    Ok(())
}

/// Invariant 1 (and 5's, for a rebuilt host): the host's game and the log's fold are one game, and the fold
/// refused no row.
fn check_same_game(
    invariant: Invariant,
    host_hash: &str,
    folded_hash: &str,
    refused_rows: usize,
) -> Result<(), Violation> {
    if refused_rows > 0 {
        return Err(violation(
            invariant,
            format!("the fold refused {refused_rows} row(s) the log holds"),
        ));
    }
    if host_hash != folded_hash {
        return Err(violation(
            invariant,
            format!("the host holds game {host_hash}, the log folds to {folded_hash}"),
        ));
    }
    Ok(())
}

/// Invariant 4: `endings` results written, `over` whether the game is over, `rows_after_end` rows
/// written after the first of them.
fn check_ends_once(endings: usize, over: bool, rows_after_end: usize) -> Result<(), Violation> {
    let expected = usize::from(over);
    if endings != expected {
        return Err(violation(
            Invariant::EndsOnce,
            format!(
                "{endings} ending(s) while the game is {}",
                if over { "over" } else { "not over" }
            ),
        ));
    }
    if rows_after_end > 0 {
        return Err(violation(
            Invariant::EndsOnce,
            format!("{rows_after_end} row(s) written after the match ended"),
        ));
    }
    Ok(())
}

/// Invariant 5: a rebuilt host holds the game and the next seq of the one it replaces.
fn check_restart(before: (&str, i64), after: (&str, i64)) -> Result<(), Violation> {
    if before.0 != after.0 {
        return Err(violation(
            Invariant::RestartHolds,
            format!(
                "the host held game {}, the rebuilt one holds {}",
                before.0, after.0
            ),
        ));
    }
    if before.1 != after.1 {
        return Err(violation(
            Invariant::RestartHolds,
            format!(
                "the host's next seq was {}, the rebuilt one's is {}",
                before.1, after.1
            ),
        ));
    }
    Ok(())
}

/// Invariant 3: a timer the host does not hold gave `outputs` outputs and moved it from `before` to `after`.
fn check_stale_unchanged(before: &str, after: &str, outputs: usize) -> Result<(), Violation> {
    if outputs > 0 {
        return Err(violation(
            Invariant::StaleTimer,
            format!("a stale timer gave {outputs} output(s)"),
        ));
    }
    if before != after {
        return Err(violation(Invariant::StaleTimer, "a stale timer changed the host"));
    }
    Ok(())
}

/// A `Panicked` output is a failure (R1253000 makes it a recorded one, and nothing here may reach it).
fn check_no_panic(panicked_outputs: usize) -> Result<(), Violation> {
    if panicked_outputs > 0 {
        return Err(violation(
            Invariant::Panicked,
            format!("{panicked_outputs} Panicked output(s)"),
        ));
    }
    Ok(())
}

/// The run reached every path it is held to: each counter at its floor.
fn check_floors(coverage: &Coverage, floors: &Floors) -> Result<(), Violation> {
    for ((name, reached), (_, floor)) in coverage.named().into_iter().zip(floors.named()) {
        if reached < floor {
            return Err(violation(
                Invariant::Floor,
                format!("{name}: {reached} is below its floor {floor}"),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------
// One machine played

/// What both machines' rigs give the loop that plays a match.
trait Rig: Sized {
    const MACHINE: Machine;
    fn new(n: u64) -> Self;
    /// One random event, and the checks after it.
    fn step(&mut self) -> Result<(), Violation>;
    /// Whether the game is over (a result written, or the match voided).
    fn over(&self) -> bool;
    /// Whether nothing more can happen (a voided match is forgotten).
    fn gone(&self) -> bool;
    /// The checks of a match's end: the whole log folded from its start.
    fn finish(&mut self) -> Result<(), Violation>;
    fn steps(&self) -> usize;
    fn last_event(&self) -> String;
    fn coverage(&self) -> Coverage;
    /// The log's nonces and the game's hash, for comparing two plays of one seed.
    fn fingerprint(&self) -> String;
}

struct Played {
    coverage: Coverage,
    fingerprint: String,
}

fn play<R: Rig>(n: u64) -> Result<Played, Failure> {
    let mut rig = R::new(n);
    let fail = |rig: &R, violation: Violation| Failure {
        machine: R::MACHINE,
        seed: n,
        step: rig.steps(),
        event: rig.last_event(),
        violation,
    };
    let mut after_end = 0;
    while rig.steps() < HOST_FUZZ_MAX_STEPS && !rig.gone() {
        if rig.over() {
            if after_end >= HOST_FUZZ_AFTER_END_STEPS {
                break;
            }
            after_end += 1;
        }
        rig.step().map_err(|violation| fail(&rig, violation))?;
    }
    rig.finish().map_err(|violation| fail(&rig, violation))?;
    Ok(Played {
        coverage: rig.coverage(),
        fingerprint: rig.fingerprint(),
    })
}

/// The deck a seat is dealt, as the `opening` helpers of `duel.rs` and `duos.rs` deal it.
fn deck_for(seed: &str, salt: &str) -> Vec<String> {
    build_ai_deck(
        &mut Rng::new(&format!("{seed}:{salt}"), 0),
        DECK_SIZE,
        &AiDeckOptions {
            banned: Some(vec![]),
            gated_sets: Some(vec![]),
            ..AiDeckOptions::default()
        },
    )
}

/// The body of a legal action to try: never a concession or a draw offer, but now and then.
fn tame<T: Clone>(draw: &mut Draw, legal: &[T], quick_end: impl Fn(&T) -> bool) -> Option<T> {
    if draw.chance(HOST_FUZZ_CONCEDE_PERCENT) {
        return draw.pick(legal).cloned();
    }
    let calm: Vec<&T> = legal.iter().filter(|body| !quick_end(body)).collect();
    draw.pick(&calm).map(|body| (*body).clone())
}

fn is_quick_end(body: &ActionBody) -> bool {
    matches!(body, ActionBody::Concede | ActionBody::OfferDraw)
}

fn is_duos_quick_end(body: &DuosActionBody) -> bool {
    matches!(
        body,
        DuosActionBody::Concede
            | DuosActionBody::Duel {
                body: ActionBody::Concede | ActionBody::OfferDraw
            }
    )
}

#[derive(Clone, Copy)]
enum Pick {
    Legal,
    Bad,
    Resend,
    Copy,
    Presence,
    Time,
    Stale,
    Relay,
    Bot,
    Restart,
}

#[derive(Clone, Copy)]
enum Presence {
    Connected,
    Attached,
    Hello,
    Left,
    Disconnected,
}

const PRESENCE: [(u32, Presence); 5] = [
    (25, Presence::Connected),
    (25, Presence::Attached),
    (15, Presence::Hello),
    (15, Presence::Left),
    (20, Presence::Disconnected),
];

/// The chance, in percent, that a seat whose socket dropped is back before its grace runs out.
const HOST_FUZZ_RETURN_PERCENT: u32 = 80;

const FREE_STEP: [(u32, Pick); 10] = [
    (35, Pick::Legal),
    (5, Pick::Bad),
    (4, Pick::Resend),
    (4, Pick::Copy),
    (12, Pick::Presence),
    (15, Pick::Time),
    (5, Pick::Stale),
    (5, Pick::Relay),
    (6, Pick::Bot),
    (3, Pick::Restart),
];

#[derive(Clone, Copy)]
enum HeldPick {
    Resolve,
    Relay,
    Resend,
    Restart,
}

const HELD_STEP: [(u32, HeldPick); 4] = [
    (50, HeldPick::Resolve),
    (20, HeldPick::Relay),
    (10, HeldPick::Resend),
    (10, HeldPick::Restart),
];

fn write_outcome(draw: &mut Draw, may_diverge: bool) -> LogOutcome {
    if may_diverge {
        draw.weighted(&[
            (80, LogOutcome::Written),
            (12, LogOutcome::Failed),
            (8, LogOutcome::Diverged),
        ])
    } else {
        draw.weighted(&[(85, LogOutcome::Written), (15, LogOutcome::Failed)])
    }
}

/// The nonce another host wrote a row at the same seq with: its own, unless the row is a server's (R270).
fn other_nonce(nonce: &str, seq: i64) -> String {
    if nonce.starts_with(SERVER_NONCE_PREFIX) {
        nonce.to_string()
    } else {
        format!("other-{seq}")
    }
}

// ---------------------------------------------------------------------------------------------------
// The 1v1 machine

struct DuelStore {
    rows: Vec<(i64, Action)>,
    clocks: MatchClocks,
    finished: bool,
}

/// What a 1v1 match is, whatever its log holds.
struct DuelBase {
    seed: String,
    decks: (Vec<String>, Vec<String>),
    bots: PerPlayer<Option<BotSeat>>,
    ranked: bool,
}

struct DuelRig {
    base: DuelBase,
    host: DuelHost,
    now: i64,
    timers: Timers,
    store: DuelStore,
    held: Option<(i64, Action)>,
    decides: Vec<DecideRequest>,
    draw: Draw,
    endings: usize,
    rows_at_end: Option<usize>,
    voided: bool,
    ceiling: bool,
    panics: usize,
    rebuild_now: bool,
    diverged: bool,
    bad_flag: Option<Violation>,
    shadow: (GameState, usize),
    shadow_refused: usize,
    steps: usize,
    last_event: String,
    coverage: Coverage,
}

impl DuelBase {
    fn opening(&self) -> ReplayInput {
        ReplayInput {
            seed: self.seed.clone(),
            decks: self.decks.clone(),
            ..ReplayInput::default()
        }
    }

    /// The game and the refused rows of the written rows folded from the opening.
    fn fold_rows(&self, rows: &[(i64, Action)]) -> (GameState, usize) {
        let mut input = self.opening();
        input.log = rows.iter().map(|(_, action)| action.clone()).collect();
        let folded = jackioh_engine::fold(&input);
        (folded.state, folded.errors.len())
    }

    /// R639000: each bot's last move's turn, read off the log as `bot_moved_turns` does.
    fn bot_moved_turn(&self, rows: &[(i64, Action)]) -> PerPlayer<Option<i32>> {
        let mut moved = PerPlayer::new(None, None);
        let mut state = jackioh_engine::fold(&self.opening()).state;
        let bot_nonce = format!("{SERVER_NONCE_PREFIX}bot-");
        for (_, action) in rows {
            let result = jackioh_engine::reduce(&state, action);
            if result.error.is_some() {
                continue;
            }
            if action.nonce.starts_with(&bot_nonce) {
                let player = action.player_id;
                let home = if snapshot(&state).seats_swapped {
                    player.opponent()
                } else {
                    player
                };
                moved[home] = Some(state.turn);
            }
            state = result.state;
        }
        moved
    }

    /// What `match_actor.rs` builds a host from: the match row and its log.
    fn setup(&self, store: &DuelStore) -> DuelSetup {
        let rows = &store.rows;
        DuelSetup {
            match_id: "match-fuzz".to_string(),
            seed: self.seed.clone(),
            created_at: EPOCH,
            game: self.fold_rows(rows).0,
            ranked: self.ranked,
            series_game: false,
            finished: store.finished,
            next_seq: rows.iter().fold(0, |highest, (seq, _)| highest.max(*seq)) + 1,
            acked: rows
                .iter()
                .map(|(seq, action)| (action.nonce.clone(), action.player_id, *seq))
                .collect(),
            clocks: store.clocks.clone(),
            bots: self.bots.clone(),
            bot_moved_turn: self.bot_moved_turn(rows),
        }
    }
}

impl DuelRig {
    fn fold_rows(&self) -> (GameState, usize) {
        self.base.fold_rows(&self.store.rows)
    }

    fn count_end(&mut self) {
        self.endings += 1;
        self.coverage.endings += 1;
        self.rows_at_end.get_or_insert(self.store.rows.len());
    }

    fn feed(&mut self, event: DuelEvent) {
        self.last_event = format!("{event:?}");
        let outputs = self.host.step(event, self.now);
        self.drive(outputs);
    }

    /// Carries out what the machine asked for, in order; what a fed answer gives goes first, as a host that
    /// answers at once would run it.
    fn drive(&mut self, outputs: Vec<DuelOutput>) {
        let mut queue: VecDeque<DuelOutput> = outputs.into();
        // The view last sent each home since the last `Pushed`: what the next one must hand over.
        let mut sent: PerPlayer<Option<PlayerView>> = PerPlayer::new(None, None);
        while let Some(output) = queue.pop_front() {
            let mut next: Vec<DuelOutput> = Vec::new();
            match output {
                DuelOutput::Send {
                    home,
                    message: SeatMessage::View { view, .. },
                } => sent[home] = Some(*view),
                DuelOutput::Pushed { views, .. } => {
                    for home in [P1, P2] {
                        let pushed = views[home].as_deref();
                        if pushed != sent[home].as_ref() {
                            self.bad_flag.get_or_insert_with(|| {
                                violation(
                                    Invariant::PushedViews,
                                    format!("a push's view for {home:?} is not the one it sent"),
                                )
                            });
                        }
                        sent[home] = None;
                    }
                }
                DuelOutput::Timer { command } => self.timers.apply(command),
                DuelOutput::PersistClocks { clocks } => self.store.clocks = clocks,
                DuelOutput::Decide { request } => self.decides.push(request),
                DuelOutput::Append { seq, action, .. } => {
                    if self.held.is_some() {
                        self.bad_flag.get_or_insert_with(|| {
                            violation(Invariant::LogWellFormed, "an append while another is outstanding")
                        });
                    } else if self.draw.chance(HOST_FUZZ_HOLD_PERCENT) {
                        self.coverage.held_writes += 1;
                        self.held = Some((seq, action));
                    } else {
                        let outcome = write_outcome(&mut self.draw, self.endings == 0);
                        next = self.resolve(seq, action, outcome);
                    }
                }
                DuelOutput::Record { request } => {
                    self.count_end();
                    self.store.finished = true;
                    self.advance_shadow();
                    let folded = hash_state(&self.shadow.0);
                    if request.final_hash != folded {
                        self.bad_flag.get_or_insert_with(|| {
                            violation(
                                Invariant::LogRebuilds,
                                format!(
                                    "the result holds hash {}, the log folds to {folded}",
                                    request.final_hash
                                ),
                            )
                        });
                    }
                    next = self.host.step(DuelEvent::ResultRecorded, self.now);
                }
                DuelOutput::CeilingDraw { .. } => {
                    self.count_end();
                    self.store.finished = true;
                    self.ceiling = true;
                }
                DuelOutput::Voided { .. } => {
                    self.count_end();
                    self.voided = true;
                }
                DuelOutput::Panicked { .. } => self.panics += 1,
                DuelOutput::LetGo {
                    reason: LetGo::Diverged,
                } => self.rebuild_now = true,
                DuelOutput::OfferRematch { home, .. } => {
                    next = self.host.step(DuelEvent::RematchSent { home }, self.now);
                }
                DuelOutput::BotEmote { home, emote } if self.held.is_none() => {
                    next = self.host.step(DuelEvent::Emote { home, emote }, self.now);
                }
                _ => {}
            }
            for output in next.into_iter().rev() {
                queue.push_front(output);
            }
        }
    }

    /// The store's answer to a row: written, refused, or taken by another host (R776000), and the machine
    /// told it.
    fn resolve(&mut self, seq: i64, action: Action, outcome: LogOutcome) -> Vec<DuelOutput> {
        match outcome {
            LogOutcome::Written => self.store.rows.push((seq, action)),
            LogOutcome::Failed => self.coverage.failed_writes += 1,
            LogOutcome::Diverged => {
                self.coverage.diverged_writes += 1;
                self.diverged = true;
                let nonce = other_nonce(&action.nonce, seq);
                self.store.rows.push((seq, Action { nonce, ..action }));
            }
        }
        self.host.step(DuelEvent::Logged { outcome }, self.now)
    }

    fn resolve_held(&mut self) {
        let Some((seq, action)) = self.held.take() else {
            return;
        };
        let outcome = write_outcome(&mut self.draw, self.endings == 0);
        self.last_event = format!("Logged {{ outcome: {outcome:?} }} for seq {seq}");
        let outputs = self.resolve(seq, action, outcome);
        self.drive(outputs);
    }

    fn advance_shadow(&mut self) {
        if self.shadow.1 > self.store.rows.len() {
            let (state, refused) = self.fold_rows();
            self.shadow = (state, self.store.rows.len());
            self.shadow_refused = refused;
        }
        while self.shadow.1 < self.store.rows.len() {
            let result = jackioh_engine::reduce(&self.shadow.0, &self.store.rows[self.shadow.1].1);
            if result.error.is_some() {
                self.shadow_refused += 1;
            } else {
                self.shadow.0 = result.state;
            }
            self.shadow.1 += 1;
        }
    }

    /// Invariant 1, on the incremental fold, and on a fold from the start now and then.
    fn check_rebuilds(&mut self) -> Result<(), Violation> {
        self.advance_shadow();
        let host_hash = hash_state(self.host.game());
        check_same_game(
            Invariant::LogRebuilds,
            &host_hash,
            &hash_state(&self.shadow.0),
            self.shadow_refused,
        )?;
        if self.steps.is_multiple_of(HOST_FUZZ_FULL_FOLD_EVERY) {
            let (state, refused) = self.fold_rows();
            check_same_game(Invariant::LogRebuilds, &host_hash, &hash_state(&state), refused)?;
        }
        Ok(())
    }

    fn check_all(&mut self) -> Result<(), Violation> {
        if let Some(violation) = self.bad_flag.take() {
            return Err(violation);
        }
        check_no_panic(self.panics)?;
        let rows: Vec<(i64, &str)> = self
            .store
            .rows
            .iter()
            .map(|(seq, action)| (*seq, action.nonce.as_str()))
            .collect();
        check_log_well_formed(&rows)?;
        if self.held.is_none() {
            self.check_rebuilds()?;
        }
        let rows_after_end = self.rows_at_end.map_or(0, |at| self.store.rows.len() - at);
        check_ends_once(self.endings, self.over(), rows_after_end)
    }

    /// A restart: the host stops, and a new one is built from what the store holds and armed. Invariant 5.
    fn restart(&mut self) -> Result<(), Violation> {
        self.coverage.restarts += 1;
        // An append in flight either landed or did not.
        let mut landed = false;
        if let Some((seq, action)) = self.held.take()
            && self.draw.chance(HOST_FUZZ_LANDED_PERCENT)
        {
            self.store.rows.push((seq, action));
            landed = true;
        }
        let before = (hash_state(self.host.game()), self.host.next_seq());
        let comparable = !landed && !self.diverged;
        self.host.step(DuelEvent::Stop, self.now);
        let (host, outputs) = DuelHost::new(self.base.setup(&self.store), self.now);
        self.host = host;
        // Before the host is armed, which may write rows of its own: what the rebuild alone holds.
        let after = (hash_state(self.host.game()), self.host.next_seq());
        self.timers.clear();
        self.decides.clear();
        self.rebuild_now = false;
        self.diverged = false;
        self.drive(outputs);
        self.feed(DuelEvent::Arm);
        let (state, refused) = self.fold_rows();
        self.shadow = (state, self.store.rows.len());
        self.shadow_refused = refused;
        if comparable {
            check_restart((&before.0, before.1), (&after.0, after.1))?;
        }
        Ok(())
    }

    /// Fires a timer and checks it changed nothing, which a timer the host does not hold must not.
    fn fire_checked(&mut self, key: TimerKey, id: u32) -> Result<(), Violation> {
        self.last_event = format!("TimerFired {{ key: {key:?}, id: {id} }} (stale)");
        let before = serde_json::to_string(&self.host).expect("the host serializes");
        let outputs = self.host.step(DuelEvent::TimerFired { key, id }, self.now);
        let after = serde_json::to_string(&self.host).expect("the host serializes");
        check_stale_unchanged(&before, &after, outputs.len())
    }

    /// An event for a seat that is not a bot's, or any when none is.
    fn random_person(&mut self) -> PlayerId {
        let people: Vec<PlayerId> = [P1, P2]
            .into_iter()
            .filter(|home| self.base.bots[*home].is_none())
            .collect();
        *self.draw.pick(&people).unwrap_or(&P1)
    }

    fn legal_body(&mut self, home: PlayerId) -> Option<ActionBody> {
        let legal = self.host.legal(self.host.playing(home));
        tame(&mut self.draw, &legal, is_quick_end)
    }

    fn act(&mut self, home: PlayerId, nonce: String, body: ActionBody) {
        self.feed(DuelEvent::Action {
            by: ActionBy::Home { home },
            nonce,
            body,
        });
    }

    fn relay(&mut self) {
        let home = self.random_person();
        let event = match self.draw.below(3) {
            0 => {
                let aim = self.draw.chance(70).then_some(Aim {
                    source: AimEnd::Hero { player: home },
                    target: self.draw.chance(50).then_some(AimEnd::Hero {
                        player: home.opponent(),
                    }),
                });
                DuelEvent::Aim { home, aim }
            }
            1 => {
                let index = self
                    .draw
                    .chance(70)
                    .then(|| i32::try_from(self.draw.below(4)).unwrap_or(0));
                DuelEvent::Hover { home, index }
            }
            _ => {
                let hand = jackioh_engine::deal_emote_hand(&self.base.seed, home);
                match hand.first() {
                    Some(emote) => DuelEvent::Emote { home, emote: *emote },
                    None => return,
                }
            }
        };
        self.feed(event);
    }

    fn presence(&mut self) {
        let home = self.random_person();
        let event = match self.draw.weighted(&PRESENCE) {
            Presence::Connected => DuelEvent::Connected { home },
            Presence::Attached => DuelEvent::Attached { home },
            Presence::Hello => DuelEvent::Hello { home },
            Presence::Left => DuelEvent::Left { home },
            Presence::Disconnected => {
                self.coverage.disconnects += 1;
                self.feed(DuelEvent::Left { home });
                self.feed(DuelEvent::Disconnected { home });
                if self.draw.chance(HOST_FUZZ_RETURN_PERCENT) {
                    self.feed(DuelEvent::Connected { home });
                    DuelEvent::Attached { home }
                } else {
                    return;
                }
            }
        };
        self.feed(event);
    }

    /// Moves the clock on to the next timer due and fires it.
    fn time(&mut self) {
        let Some((key, id, at)) = self.timers.take_next() else {
            self.now += SECOND;
            return;
        };
        self.now = self.now.max(at);
        self.coverage.timers_fired += 1;
        self.feed(DuelEvent::TimerFired { key, id });
    }

    fn resend(&mut self, own: bool) {
        let clients: Vec<(i64, Action)> = self
            .store
            .rows
            .iter()
            .filter(|(_, action)| !action.nonce.starts_with(SERVER_NONCE_PREFIX))
            .cloned()
            .collect();
        let Some((_, row)) = self.draw.pick(&clients).cloned() else {
            return;
        };
        let owner = [P1, P2]
            .into_iter()
            .find(|home| self.host.playing(*home) == row.player_id);
        let Some(owner) = owner else { return };
        let home = if own { owner } else { owner.opponent() };
        if own {
            self.coverage.nonce_resends += 1;
        } else {
            self.coverage.nonce_copies += 1;
        }
        self.act(home, row.nonce, row.body);
    }

    fn bad_action(&mut self) {
        let home = self.random_person();
        let nonce = format!("bad-{}", self.steps);
        let other = self.host.legal(self.host.playing(home.opponent()));
        let stale: Vec<ActionBody> = self
            .store
            .rows
            .iter()
            .map(|(_, action)| action.body.clone())
            .collect();
        let body = if self.draw.chance(50) {
            tame(&mut self.draw, &other, is_quick_end)
        } else {
            self.draw.pick(&stale).cloned()
        };
        if let Some(body) = body {
            self.act(home, nonce, body);
        }
    }

    /// A bot's decided move, as the search would give it: any legal move, or none (the machine falls back).
    fn bot_answer(&mut self) {
        if self.decides.is_empty() {
            return;
        }
        let at = if self.decides.len() > 1 && self.draw.chance(HOST_FUZZ_STALE_ANSWER_PERCENT) {
            0
        } else {
            self.decides.len() - 1
        };
        let request = self.decides.remove(at);
        let legal: Vec<ActionBody> = self
            .host
            .legal(request.seat)
            .into_iter()
            .filter(|body| !r188_forbidden(body))
            .collect();
        let body = if self.draw.chance(85) {
            self.draw.pick(&legal).cloned()
        } else {
            None
        };
        let version = if self.draw.chance(10) {
            request.version - 1
        } else {
            request.version
        };
        self.coverage.decisions += 1;
        self.feed(DuelEvent::Decided {
            home: request.home,
            version,
            turn: request.turn,
            choice: BotChoice {
                body,
                ai_cursor: request.ai_cursor + 1,
                legal_count: u32::try_from(legal.len()).unwrap_or(u32::MAX),
                lethal: false,
                aim: None,
            },
        });
    }

    fn free_step(&mut self) -> Result<(), Violation> {
        match self.draw.weighted(&FREE_STEP) {
            Pick::Legal => {
                let home = self.random_person();
                if let Some(body) = self.legal_body(home) {
                    let nonce = format!("n{}", self.steps);
                    self.act(home, nonce, body);
                }
            }
            Pick::Bad => self.bad_action(),
            Pick::Resend => self.resend(true),
            Pick::Copy => self.resend(false),
            Pick::Presence => self.presence(),
            Pick::Time => self.time(),
            Pick::Stale => {
                let stale = self.timers.stale();
                if let Some((key, id)) = self.draw.pick(&stale).copied() {
                    self.coverage.stale_timers += 1;
                    self.fire_checked(key, id)?;
                }
            }
            Pick::Relay => self.relay(),
            Pick::Bot => self.bot_answer(),
            Pick::Restart => self.restart()?,
        }
        Ok(())
    }

    /// With a write outstanding the machine awaits the log: only what a host may feed meanwhile is fed.
    fn held_step(&mut self) -> Result<(), Violation> {
        match self.draw.weighted(&HELD_STEP) {
            HeldPick::Resolve => self.resolve_held(),
            HeldPick::Relay => self.relay(),
            HeldPick::Resend => {
                let rows: Vec<(i64, Action)> = self.store.rows.clone();
                if let Some((_, row)) = self.draw.pick(&rows).cloned()
                    && !row.nonce.starts_with(SERVER_NONCE_PREFIX)
                    && let Some(owner) = [P1, P2]
                        .into_iter()
                        .find(|home| self.host.playing(*home) == row.player_id)
                {
                    self.coverage.nonce_resends += 1;
                    self.act(owner, row.nonce, row.body);
                }
            }
            HeldPick::Restart => self.restart()?,
        }
        Ok(())
    }
}

impl Rig for DuelRig {
    const MACHINE: Machine = Machine::Duel;

    fn new(n: u64) -> DuelRig {
        jackioh_cards::register_all();
        let seed = format!("host-fuzz-duel-{n}");
        let decks = (deck_for(&seed, "a"), deck_for(&seed, "b"));
        let seats_a_bot = n.is_multiple_of(HOST_FUZZ_BOT_EVERY);
        let bot = BotSeat {
            strength: 2,
            speed: 1.0,
            persona: PersonaName::Balanced,
            hidden: n.is_multiple_of(2),
        };
        let base = DuelBase {
            seed,
            decks,
            bots: PerPlayer::new(None, seats_a_bot.then_some(bot)),
            ranked: seats_a_bot && (n / HOST_FUZZ_BOT_EVERY).is_multiple_of(2),
        };
        let store = DuelStore {
            rows: Vec::new(),
            clocks: initial_clocks(EPOCH),
            finished: false,
        };
        // The first host is built as the server builds one from an empty log, then armed.
        let setup = base.setup(&store);
        let shadow = (setup.game.clone(), 0);
        let (host, outputs) = DuelHost::new(setup, EPOCH);
        let mut rig = DuelRig {
            base,
            host,
            now: EPOCH,
            timers: Timers::default(),
            store,
            held: None,
            decides: Vec::new(),
            draw: Draw::new(Machine::Duel, n),
            endings: 0,
            rows_at_end: None,
            voided: false,
            ceiling: false,
            panics: 0,
            rebuild_now: false,
            diverged: false,
            bad_flag: None,
            shadow,
            shadow_refused: 0,
            steps: 0,
            last_event: "(new)".to_string(),
            coverage: Coverage {
                matches: 1,
                bot_matches: u64::from(seats_a_bot),
                ..Coverage::default()
            },
        };
        rig.drive(outputs);
        rig.feed(DuelEvent::Arm);
        for home in [P1, P2] {
            if rig.base.bots[home].is_none() && rig.draw.chance(HOST_FUZZ_RETURN_PERCENT) {
                rig.feed(DuelEvent::Connected { home });
                rig.feed(DuelEvent::Attached { home });
            }
        }
        rig
    }

    fn step(&mut self) -> Result<(), Violation> {
        self.steps += 1;
        self.coverage.steps += 1;
        if self.held.is_some() {
            self.held_step()?;
        } else {
            self.free_step()?;
        }
        if self.rebuild_now && !self.voided {
            self.restart()?;
        }
        self.check_all()
    }

    /// Invariant 4's "over", read off the game itself, not off the outputs it is counted against: a result
    /// in the game, a ceiling draw (R1253000, which leaves the game as it is) or a voided match. While a
    /// write is outstanding the game is not yet the log's, so only an ending already seen counts.
    fn over(&self) -> bool {
        if self.held.is_some() {
            return self.endings > 0;
        }
        self.host.game().result.is_some() || self.ceiling || self.voided
    }

    fn gone(&self) -> bool {
        self.voided
    }

    fn finish(&mut self) -> Result<(), Violation> {
        if self.held.is_some() {
            return Ok(());
        }
        let (state, refused) = self.fold_rows();
        check_same_game(
            Invariant::LogRebuilds,
            &hash_state(self.host.game()),
            &hash_state(&state),
            refused,
        )
    }

    fn steps(&self) -> usize {
        self.steps
    }

    fn last_event(&self) -> String {
        self.last_event.clone()
    }

    fn coverage(&self) -> Coverage {
        self.coverage.clone()
    }

    fn fingerprint(&self) -> String {
        let nonces: Vec<&str> = self
            .store
            .rows
            .iter()
            .map(|(_, action)| action.nonce.as_str())
            .collect();
        format!("{} {}", nonces.join(","), hash_state(self.host.game()))
    }
}

// ---------------------------------------------------------------------------------------------------
// The co-op machine

struct DuosStore {
    rows: Vec<(i64, DuosAction)>,
    finished: bool,
}

struct DuosRig {
    options: DuosCreateOptions,
    team_b: DuosTeamB,
    host: DuosHost,
    now: i64,
    timers: Timers,
    store: DuosStore,
    held: Option<(i64, DuosAction)>,
    decides: Vec<DuosDecide>,
    draw: Draw,
    endings: usize,
    rows_at_end: Option<usize>,
    ceiling: bool,
    panics: usize,
    rebuild_now: bool,
    diverged: bool,
    bad_flag: Option<Violation>,
    shadow: (DuosState, usize),
    shadow_refused: usize,
    steps: usize,
    last_event: String,
    coverage: Coverage,
}

impl DuosRig {
    fn fold_rows(&self) -> (DuosState, Option<usize>) {
        let folded = duos_fold(&jackioh_engine::duos::DuosReplayInput {
            options: self.options.clone(),
            log: self.store.rows.iter().map(|(_, action)| action.clone()).collect(),
        });
        (folded.state, folded.refused_at)
    }

    /// What `duos/actor.rs` builds a host from after a restart.
    fn setup(&self) -> DuosSetup {
        let rows = &self.store.rows;
        DuosSetup {
            match_id: "duos-fuzz".to_string(),
            created_at: EPOCH,
            game: self.fold_rows().0,
            finished: self.store.finished,
            next_seq: rows.iter().fold(0, |highest, (seq, _)| highest.max(*seq)) + 1,
            acked: rows
                .iter()
                .map(|(seq, action)| DuosAck {
                    nonce: action.nonce.clone(),
                    seat: action.seat,
                    seq: *seq,
                })
                .collect(),
            team_b: self.team_b,
        }
    }

    fn count_end(&mut self) {
        self.endings += 1;
        self.coverage.endings += 1;
        self.rows_at_end.get_or_insert(self.store.rows.len());
    }

    fn feed(&mut self, event: DuosHostEvent) {
        self.last_event = format!("{event:?}");
        let outputs = self.host.step(event, self.now);
        self.drive(outputs);
    }

    fn drive(&mut self, outputs: Vec<DuosHostOutput>) {
        let mut queue: VecDeque<DuosHostOutput> = outputs.into();
        while let Some(output) = queue.pop_front() {
            let mut next: Vec<DuosHostOutput> = Vec::new();
            match output {
                DuosHostOutput::Timer { command } => self.timers.apply(command),
                DuosHostOutput::Decide { request } => self.decides.push(request),
                DuosHostOutput::Append { seq, action, .. } => {
                    if self.held.is_some() {
                        self.bad_flag.get_or_insert_with(|| {
                            violation(Invariant::LogWellFormed, "an append while another is outstanding")
                        });
                    } else if self.draw.chance(HOST_FUZZ_HOLD_PERCENT) {
                        self.coverage.held_writes += 1;
                        self.held = Some((seq, action));
                    } else {
                        let outcome = write_outcome(&mut self.draw, self.endings == 0);
                        next = self.resolve(seq, action, outcome);
                    }
                }
                DuosHostOutput::Record { .. } => {
                    self.count_end();
                    self.store.finished = true;
                }
                DuosHostOutput::CeilingDraw { .. } => {
                    self.count_end();
                    self.store.finished = true;
                    self.ceiling = true;
                }
                DuosHostOutput::Panicked { .. } => self.panics += 1,
                DuosHostOutput::LetGo {
                    reason: DuosLetGo::Diverged,
                } => self.rebuild_now = true,
                DuosHostOutput::LetGo {
                    reason: DuosLetGo::MatchGone,
                } => {
                    self.bad_flag.get_or_insert_with(|| {
                        violation(
                            Invariant::EndsOnce,
                            "the match was let go as gone while its row exists",
                        )
                    });
                }
                DuosHostOutput::CheckRow => {
                    next = self
                        .host
                        .step(DuosHostEvent::RowChecked { exists: true }, self.now);
                }
                _ => {}
            }
            for output in next.into_iter().rev() {
                queue.push_front(output);
            }
        }
    }

    fn resolve(&mut self, seq: i64, action: DuosAction, outcome: LogOutcome) -> Vec<DuosHostOutput> {
        match outcome {
            LogOutcome::Written => self.store.rows.push((seq, action)),
            LogOutcome::Failed => self.coverage.failed_writes += 1,
            LogOutcome::Diverged => {
                self.coverage.diverged_writes += 1;
                self.diverged = true;
                let nonce = other_nonce(&action.nonce, seq);
                self.store.rows.push((seq, DuosAction { nonce, ..action }));
            }
        }
        self.host.step(DuosHostEvent::Logged { outcome }, self.now)
    }

    fn resolve_held(&mut self) {
        let Some((seq, action)) = self.held.take() else {
            return;
        };
        let outcome = write_outcome(&mut self.draw, self.endings == 0);
        self.last_event = format!("Logged {{ outcome: {outcome:?} }} for seq {seq}");
        let outputs = self.resolve(seq, action, outcome);
        self.drive(outputs);
    }

    fn advance_shadow(&mut self) {
        if self.shadow.1 > self.store.rows.len() {
            let (state, refused) = self.fold_rows();
            self.shadow = (state, self.store.rows.len());
            self.shadow_refused = usize::from(refused.is_some());
        }
        if self.shadow.1 < self.store.rows.len() {
            let tail: Vec<DuosAction> = self.store.rows[self.shadow.1..]
                .iter()
                .map(|(_, action)| action.clone())
                .collect();
            let folded = duos_fold_from(self.shadow.0.clone(), &tail, self.shadow.1);
            self.shadow_refused += usize::from(folded.refused_at.is_some());
            self.shadow = (folded.state, self.store.rows.len());
        }
    }

    fn check_rebuilds(&mut self) -> Result<(), Violation> {
        self.advance_shadow();
        let host_hash = duos_hash(self.host.game());
        check_same_game(
            Invariant::LogRebuilds,
            &host_hash,
            &duos_hash(&self.shadow.0),
            self.shadow_refused,
        )?;
        if self.steps.is_multiple_of(HOST_FUZZ_FULL_FOLD_EVERY) {
            let (state, refused) = self.fold_rows();
            check_same_game(
                Invariant::LogRebuilds,
                &host_hash,
                &duos_hash(&state),
                usize::from(refused.is_some()),
            )?;
        }
        Ok(())
    }

    fn check_all(&mut self) -> Result<(), Violation> {
        if let Some(violation) = self.bad_flag.take() {
            return Err(violation);
        }
        check_no_panic(self.panics)?;
        let rows: Vec<(i64, &str)> = self
            .store
            .rows
            .iter()
            .map(|(seq, action)| (*seq, action.nonce.as_str()))
            .collect();
        check_log_well_formed(&rows)?;
        if self.held.is_none() {
            self.check_rebuilds()?;
        }
        let rows_after_end = self.rows_at_end.map_or(0, |at| self.store.rows.len() - at);
        check_ends_once(self.endings, self.over(), rows_after_end)
    }

    fn restart(&mut self) -> Result<(), Violation> {
        self.coverage.restarts += 1;
        let mut landed = false;
        if let Some((seq, action)) = self.held.take()
            && self.draw.chance(HOST_FUZZ_LANDED_PERCENT)
        {
            self.store.rows.push((seq, action));
            landed = true;
        }
        let before = (duos_hash(self.host.game()), self.host.next_seq());
        let comparable = !landed && !self.diverged;
        self.host.step(DuosHostEvent::Stop, self.now);
        let (host, outputs) = DuosHost::new(self.setup(), self.now);
        self.host = host;
        // Before the host is armed, which may write rows of its own: what the rebuild alone holds.
        let after = (duos_hash(self.host.game()), self.host.next_seq());
        self.timers.clear();
        self.decides.clear();
        self.rebuild_now = false;
        self.diverged = false;
        self.drive(outputs);
        self.feed(DuosHostEvent::Arm);
        let (state, refused) = self.fold_rows();
        self.shadow = (state, self.store.rows.len());
        self.shadow_refused = usize::from(refused.is_some());
        if comparable {
            check_restart((&before.0, before.1), (&after.0, after.1))?;
        }
        Ok(())
    }

    fn fire_checked(&mut self, key: TimerKey, id: u32) -> Result<(), Violation> {
        self.last_event = format!("TimerFired {{ key: {key:?}, id: {id} }} (stale)");
        let before = serde_json::to_string(&self.host).expect("the host serializes");
        let outputs = self.host.step(DuosHostEvent::TimerFired { key, id }, self.now);
        let after = serde_json::to_string(&self.host).expect("the host serializes");
        check_stale_unchanged(&before, &after, outputs.len())
    }

    fn random_person(&mut self) -> DuosSeat {
        *self.draw.pick(self.team_b.people()).expect("a match has people")
    }

    fn act(&mut self, seat: DuosSeat, nonce: String, body: DuosActionBody) {
        self.feed(DuosHostEvent::Action { seat, nonce, body });
    }

    fn presence(&mut self) {
        let seat = self.random_person();
        let event = match self.draw.weighted(&PRESENCE) {
            Presence::Connected => DuosHostEvent::Connected { seat },
            Presence::Attached => DuosHostEvent::Attached { seat },
            Presence::Hello => DuosHostEvent::Hello { seat },
            Presence::Left => DuosHostEvent::Left { seat },
            Presence::Disconnected => {
                self.coverage.disconnects += 1;
                self.feed(DuosHostEvent::Left { seat });
                self.feed(DuosHostEvent::Disconnected { seat });
                if self.draw.chance(HOST_FUZZ_RETURN_PERCENT) {
                    self.feed(DuosHostEvent::Connected { seat });
                    DuosHostEvent::Attached { seat }
                } else {
                    return;
                }
            }
        };
        self.feed(event);
    }

    fn time(&mut self) {
        let Some((key, id, at)) = self.timers.take_next() else {
            self.now += SECOND;
            return;
        };
        self.now = self.now.max(at);
        self.coverage.timers_fired += 1;
        self.feed(DuosHostEvent::TimerFired { key, id });
    }

    fn resend(&mut self, own: bool) {
        let people = self.team_b.people();
        let clients: Vec<(i64, DuosAction)> = self
            .store
            .rows
            .iter()
            .filter(|(_, action)| {
                !action.nonce.starts_with(SERVER_NONCE_PREFIX) && people.contains(&action.seat)
            })
            .cloned()
            .collect();
        let Some((_, row)) = self.draw.pick(&clients).cloned() else {
            return;
        };
        let seat = if own {
            row.seat
        } else {
            let others: Vec<DuosSeat> = people.iter().copied().filter(|seat| *seat != row.seat).collect();
            match self.draw.pick(&others) {
                Some(seat) => *seat,
                None => return,
            }
        };
        if own {
            self.coverage.nonce_resends += 1;
        } else {
            self.coverage.nonce_copies += 1;
        }
        self.act(seat, row.nonce, row.body);
    }

    fn bad_action(&mut self) {
        let seat = self.random_person();
        let nonce = format!("bad-{}", self.steps);
        let stale: Vec<DuosActionBody> = self
            .store
            .rows
            .iter()
            .map(|(_, action)| action.body.clone())
            .collect();
        let body = if self.draw.chance(50) {
            let other = self.random_person();
            let legal = self.host.legal(other);
            tame(&mut self.draw, &legal, is_duos_quick_end)
        } else {
            self.draw.pick(&stale).cloned()
        };
        if let Some(body) = body {
            self.act(seat, nonce, body);
        }
    }

    /// The AI's search, as it would answer: any legal move, none (the machine falls back), or a failure.
    fn ai_answer(&mut self) {
        if self.decides.is_empty() {
            return;
        }
        let at = if self.decides.len() > 1 && self.draw.chance(HOST_FUZZ_STALE_ANSWER_PERCENT) {
            0
        } else {
            self.decides.len() - 1
        };
        let request = self.decides.remove(at);
        let legal: Vec<DuosActionBody> = self
            .host
            .legal(request.seat)
            .into_iter()
            .filter(|body| !ai_forbidden(body))
            .collect();
        let body = if self.draw.chance(85) {
            self.draw.pick(&legal).cloned()
        } else {
            None
        };
        self.coverage.decisions += 1;
        let failed = self.draw.chance(8);
        self.feed(DuosHostEvent::Decided {
            seat: request.seat,
            body,
            cursor: request.cursor + 1,
            failed,
        });
    }

    fn free_step(&mut self) -> Result<(), Violation> {
        match self.draw.weighted(&FREE_STEP) {
            Pick::Legal => {
                let seat = self.random_person();
                let legal = self.host.legal(seat);
                if let Some(body) = tame(&mut self.draw, &legal, is_duos_quick_end) {
                    let nonce = format!("n{}", self.steps);
                    self.act(seat, nonce, body);
                }
            }
            Pick::Bad => self.bad_action(),
            Pick::Resend => self.resend(true),
            Pick::Copy => self.resend(false),
            Pick::Presence => self.presence(),
            Pick::Time => self.time(),
            Pick::Stale => {
                let stale = self.timers.stale();
                if let Some((key, id)) = self.draw.pick(&stale).copied() {
                    self.coverage.stale_timers += 1;
                    self.fire_checked(key, id)?;
                }
            }
            // Co-op has no aims, hovers or emotes: the slot answers an AI's search instead.
            Pick::Relay | Pick::Bot => self.ai_answer(),
            Pick::Restart => self.restart()?,
        }
        Ok(())
    }

    fn held_step(&mut self) -> Result<(), Violation> {
        match self.draw.weighted(&HELD_STEP) {
            HeldPick::Resolve => self.resolve_held(),
            HeldPick::Relay => self.presence_while_held(),
            HeldPick::Resend => {
                let people = self.team_b.people();
                let rows: Vec<(i64, DuosAction)> = self
                    .store
                    .rows
                    .iter()
                    .filter(|(_, action)| {
                        !action.nonce.starts_with(SERVER_NONCE_PREFIX) && people.contains(&action.seat)
                    })
                    .cloned()
                    .collect();
                if let Some((_, row)) = self.draw.pick(&rows).cloned() {
                    self.coverage.nonce_resends += 1;
                    self.act(row.seat, row.nonce, row.body);
                }
            }
            HeldPick::Restart => self.restart()?,
        }
        Ok(())
    }

    /// Who is here changes as the host learns it, even with a write outstanding.
    fn presence_while_held(&mut self) {
        let seat = self.random_person();
        let event = if self.draw.chance(50) {
            DuosHostEvent::Connected { seat }
        } else {
            DuosHostEvent::Left { seat }
        };
        self.feed(event);
    }
}

impl Rig for DuosRig {
    const MACHINE: Machine = Machine::Duos;

    fn new(n: u64) -> DuosRig {
        jackioh_cards::register_all();
        let seed = format!("host-fuzz-duos-{n}");
        let options: DuosCreateOptions = serde_json::from_value(serde_json::json!({
            "seed": seed,
            "decks": {
                "a1": deck_for(&seed, "a1"),
                "a2": deck_for(&seed, "a2"),
                "b1": deck_for(&seed, "b1"),
                "b2": deck_for(&seed, "b2"),
            },
        }))
        .expect("create options");
        let team_b = if n % 4 == 3 {
            DuosTeamB::People
        } else {
            DuosTeamB::Ai
        };
        let game = duos_fold(&jackioh_engine::duos::DuosReplayInput {
            options: options.clone(),
            log: Vec::new(),
        })
        .state;
        let (host, outputs) = DuosHost::new(
            DuosSetup {
                match_id: "duos-fuzz".to_string(),
                created_at: EPOCH,
                game: game.clone(),
                finished: false,
                next_seq: 1,
                acked: Vec::new(),
                team_b,
            },
            EPOCH,
        );
        let mut rig = DuosRig {
            options,
            team_b,
            host,
            now: EPOCH,
            timers: Timers::default(),
            store: DuosStore {
                rows: Vec::new(),
                finished: false,
            },
            held: None,
            decides: Vec::new(),
            draw: Draw::new(Machine::Duos, n),
            endings: 0,
            rows_at_end: None,
            ceiling: false,
            panics: 0,
            rebuild_now: false,
            diverged: false,
            bad_flag: None,
            shadow: (game, 0),
            shadow_refused: 0,
            steps: 0,
            last_event: "(new)".to_string(),
            coverage: Coverage {
                matches: 1,
                ..Coverage::default()
            },
        };
        rig.drive(outputs);
        rig.feed(DuosHostEvent::Arm);
        for seat in team_b.people().to_vec() {
            if rig.draw.chance(HOST_FUZZ_RETURN_PERCENT) {
                rig.feed(DuosHostEvent::Connected { seat });
                rig.feed(DuosHostEvent::Attached { seat });
            }
        }
        rig
    }

    fn step(&mut self) -> Result<(), Violation> {
        self.steps += 1;
        self.coverage.steps += 1;
        if self.held.is_some() {
            self.held_step()?;
        } else {
            self.free_step()?;
        }
        if self.rebuild_now {
            self.restart()?;
        }
        self.check_all()
    }

    /// Invariant 4's "over", read off the game itself, not off the outputs it is counted against: a result
    /// in the game or a ceiling draw (R1253000, which leaves the game as it is). While a write is outstanding
    /// the game is not yet the log's, so only an ending already seen counts.
    fn over(&self) -> bool {
        if self.held.is_some() {
            return self.endings > 0;
        }
        self.host.game().result.is_some() || self.ceiling
    }

    fn gone(&self) -> bool {
        false
    }

    fn finish(&mut self) -> Result<(), Violation> {
        if self.held.is_some() {
            return Ok(());
        }
        let (state, refused) = self.fold_rows();
        check_same_game(
            Invariant::LogRebuilds,
            &duos_hash(self.host.game()),
            &duos_hash(&state),
            usize::from(refused.is_some()),
        )
    }

    fn steps(&self) -> usize {
        self.steps
    }

    fn last_event(&self) -> String {
        self.last_event.clone()
    }

    fn coverage(&self) -> Coverage {
        self.coverage.clone()
    }

    fn fingerprint(&self) -> String {
        let nonces: Vec<&str> = self
            .store
            .rows
            .iter()
            .map(|(_, action)| action.nonce.as_str())
            .collect();
        format!("{} {}", nonces.join(","), duos_hash(self.host.game()))
    }
}

// ---------------------------------------------------------------------------------------------------
// The tests

/// What each machine's pull-request run must reach, about half of what it measured (the run is the same
/// every time, so a harness that stops reaching a path fails here instead of passing on less).
fn floors(machine: Machine) -> Floors {
    match machine {
        Machine::Duel => Floors {
            matches: HOST_FUZZ_SEEDS,
            steps: 600,
            restarts: 30,
            failed_writes: 12,
            diverged_writes: 10,
            held_writes: 30,
            stale_timers: 20,
            bot_matches: 3,
            endings: 8,
            nonce_resends: 30,
            nonce_copies: 25,
            disconnects: 15,
            timers_fired: 55,
            decisions: 5,
        },
        Machine::Duos => Floors {
            matches: HOST_FUZZ_SEEDS,
            steps: 1500,
            restarts: 70,
            failed_writes: 20,
            diverged_writes: 15,
            held_writes: 40,
            stale_timers: 60,
            bot_matches: 0,
            endings: 8,
            nonce_resends: 60,
            nonce_copies: 50,
            disconnects: 40,
            timers_fired: 190,
            decisions: 40,
        },
    }
}

fn play_many<R: Rig>(seeds: u64) -> Coverage {
    let mut total = Coverage::default();
    for n in 0..seeds {
        match play::<R>(n) {
            Ok(played) => total.add(&played.coverage),
            Err(failure) => panic!("{failure}"),
        }
    }
    eprintln!("host fuzz {}: {total:?}", R::MACHINE);
    total
}

#[test]
fn the_duel_host_survives_random_events() {
    let total = play_many::<DuelRig>(HOST_FUZZ_SEEDS);
    if let Err(violation) = check_floors(&total, &floors(Machine::Duel)) {
        panic!("host fuzz: duel: {violation:?}");
    }
}

#[test]
fn the_duos_host_survives_random_events() {
    let total = play_many::<DuosRig>(HOST_FUZZ_SEEDS);
    if let Err(violation) = check_floors(&total, &floors(Machine::Duos)) {
        panic!("host fuzz: duos: {violation:?}");
    }
}

#[test]
#[ignore = "daily: HOST_FUZZ_DAILY_SEEDS matches per machine; the per-PR tests the_duel_host_survives_random_events and the_duos_host_survives_random_events play HOST_FUZZ_SEEDS of them"]
fn daily_host_fuzz_sweep() {
    play_many::<DuelRig>(HOST_FUZZ_DAILY_SEEDS);
    play_many::<DuosRig>(HOST_FUZZ_DAILY_SEEDS);
}

#[test]
fn repro_one_seed() {
    match REPRO {
        Some((Machine::Duel, n)) => {
            if let Err(failure) = play::<DuelRig>(n) {
                panic!("{failure}");
            }
        }
        Some((Machine::Duos, n)) => {
            if let Err(failure) = play::<DuosRig>(n) {
                panic!("{failure}");
            }
        }
        None => {}
    }
}

fn plays_alike<R: Rig>() {
    let first = play::<R>(0).expect("seed 0 plays");
    let second = play::<R>(0).expect("seed 0 plays");
    assert_eq!(first.coverage, second.coverage);
    assert_eq!(first.fingerprint, second.fingerprint);
}

#[test]
fn the_same_seed_plays_the_same_duel() {
    plays_alike::<DuelRig>();
}

#[test]
fn the_same_seed_plays_the_same_duos_match() {
    plays_alike::<DuosRig>();
}

#[test]
fn a_failure_names_its_machine_seed_step_and_event() {
    let failure = Failure {
        machine: Machine::Duos,
        seed: 17,
        step: 42,
        event: "Arm".to_string(),
        violation: violation(Invariant::EndsOnce, "two endings"),
    };
    let text = failure.to_string();
    for part in [
        "duos",
        "seed 17",
        "step 42",
        "event Arm",
        "EndsOnce",
        "REPRO = Some((Machine::Duos, 17))",
    ] {
        assert!(text.contains(part), "{part} is missing from {text}");
    }
}

#[test]
fn the_log_checker_fails_a_missing_row() {
    let error = check_log_well_formed(&[(2, "a"), (3, "b")]).expect_err("a log from seq 2");
    assert_eq!(error.invariant, Invariant::LogWellFormed);
    let gap = check_log_well_formed(&[(1, "a"), (3, "b")]).expect_err("a gap");
    assert_eq!(gap.invariant, Invariant::LogWellFormed);
    assert_eq!(check_log_well_formed(&[(1, "a"), (2, "b")]), Ok(()));
}

#[test]
fn the_log_checker_fails_a_nonce_in_two_rows() {
    let error = check_log_well_formed(&[(1, "a"), (2, "a")]).expect_err("one nonce twice");
    assert_eq!(error.invariant, Invariant::LogWellFormed);
}

/// Plays a rig until its log holds two rows and no write is outstanding.
fn rig_with_rows<R: Rig>(rows: impl Fn(&R) -> (usize, bool)) -> R {
    for n in 0..50 {
        let mut rig = R::new(n);
        for _ in 0..HOST_FUZZ_MAX_STEPS {
            if rig.gone() {
                break;
            }
            rig.step().expect("a fuzz step");
            let (count, held) = rows(&rig);
            if count >= 2 && !held && !rig.over() {
                return rig;
            }
        }
    }
    panic!("no match reached two rows");
}

#[test]
fn the_log_checker_fails_a_log_that_rebuilds_another_game() {
    let mut duel = rig_with_rows::<DuelRig>(|rig| (rig.store.rows.len(), rig.held.is_some()));
    duel.store.rows.pop();
    let error = duel.check_rebuilds().expect_err("a log missing its last row");
    assert_eq!(error.invariant, Invariant::LogRebuilds);

    let mut duos = rig_with_rows::<DuosRig>(|rig| (rig.store.rows.len(), rig.held.is_some()));
    duos.store.rows.pop();
    let error = duos.check_rebuilds().expect_err("a log missing its last row");
    assert_eq!(error.invariant, Invariant::LogRebuilds);
}

#[test]
fn the_end_checker_fails_a_second_record() {
    for (endings, over, after) in [(2, true, 0), (0, true, 0), (1, false, 0), (1, true, 1)] {
        let error = check_ends_once(endings, over, after).expect_err("a bad ending");
        assert_eq!(error.invariant, Invariant::EndsOnce, "{endings} {over} {after}");
    }
    assert_eq!(check_ends_once(1, true, 0), Ok(()));
    assert_eq!(check_ends_once(0, false, 0), Ok(()));
}

#[test]
fn the_restart_checker_fails_a_different_game() {
    let hash = check_restart(("a", 3), ("b", 3)).expect_err("another game");
    assert_eq!(hash.invariant, Invariant::RestartHolds);
    let seq = check_restart(("a", 3), ("a", 4)).expect_err("another next seq");
    assert_eq!(seq.invariant, Invariant::RestartHolds);
    assert_eq!(check_restart(("a", 3), ("a", 3)), Ok(()));

    // A host rebuilt from a log that lost its last row holds another game.
    let mut duel = rig_with_rows::<DuelRig>(|rig| (rig.store.rows.len(), rig.held.is_some()));
    duel.store.rows.pop();
    let error = duel.restart().expect_err("a rebuilt host one row behind");
    assert_eq!(error.invariant, Invariant::RestartHolds);
    let mut duos = rig_with_rows::<DuosRig>(|rig| (rig.store.rows.len(), rig.held.is_some()));
    duos.store.rows.pop();
    let error = duos.restart().expect_err("a rebuilt host one row behind");
    assert_eq!(error.invariant, Invariant::RestartHolds);
}

#[test]
fn the_stale_checker_fails_a_timer_that_changed_the_host() {
    let error = check_stale_unchanged("a", "b", 0).expect_err("a changed host");
    assert_eq!(error.invariant, Invariant::StaleTimer);
    let error = check_stale_unchanged("a", "a", 1).expect_err("an output");
    assert_eq!(error.invariant, Invariant::StaleTimer);
    assert_eq!(check_stale_unchanged("a", "a", 0), Ok(()));

    // A timer the host does hold is not stale: firing it changes the host.
    let mut duel = DuelRig::new(1);
    let (key, (id, _)) = duel
        .timers
        .live
        .first()
        .map(|(key, held)| (*key, *held))
        .expect("an armed timer");
    let error = duel
        .fire_checked(key, id)
        .expect_err("a live timer changed the host");
    assert_eq!(error.invariant, Invariant::StaleTimer);
    let mut duos = DuosRig::new(1);
    let (key, (id, _)) = duos
        .timers
        .live
        .first()
        .map(|(key, held)| (*key, *held))
        .expect("an armed timer");
    let error = duos
        .fire_checked(key, id)
        .expect_err("a live timer changed the host");
    assert_eq!(error.invariant, Invariant::StaleTimer);
}

#[test]
fn the_panic_checker_fails_a_panicked_output() {
    let error = check_no_panic(1).expect_err("a panic");
    assert_eq!(error.invariant, Invariant::Panicked);
    assert_eq!(check_no_panic(0), Ok(()));
}

#[test]
fn a_run_below_its_floor_fails() {
    let error = check_floors(&Coverage::default(), &floors(Machine::Duel)).expect_err("no matches played");
    assert_eq!(error.invariant, Invariant::Floor);
    assert!(error.detail.starts_with("matches"), "{}", error.detail);
    let met = floors(Machine::Duos);
    assert_eq!(check_floors(&met, &floors(Machine::Duos)), Ok(()));
    let short = Coverage {
        stale_timers: met.stale_timers - 1,
        ..met
    };
    let error = check_floors(&short, &floors(Machine::Duos)).expect_err("too few stale timers");
    assert!(error.detail.starts_with("stale_timers"), "{}", error.detail);
}
