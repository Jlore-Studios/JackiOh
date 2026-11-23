//! `cargo jackioh arena --forks`: the fork screen (R4397000). Where the deal screen plays the
//! candidate against the control and reads whole games, the fork screen measures a change at the
//! moves it changes, each played out both ways from where it was made.
//!
//! The trunk plays each deal of the set once with the control on both seats. At each trunk decision
//! the candidate gets the same request the control's seat got — the same `redact(state, seat)`, seat,
//! stream seed and cursor (CLAUDE.md rule 7: never the true state) — and when its action differs
//! (`fork_key`) that decision is a fork. The trunk always plays on with the control's action; a game
//! keeps its first `FORK_MAX_PER_GAME` forks, and the candidate is not asked after that.
//!
//! Each fork plays `FORK_ROLLOUTS` pairs from the fork's state: pair r plays the state once after the
//! candidate's action and once after the control's, each through the referee's `accept` (a refused
//! action is replaced as today; a `None` first action is a fault), then both arms play on to the end
//! with the control on both seats, on the streams `<deal>:fork:<k>:<r>:ctl:<seat>`. A fork's edge is
//! the forking seat's wins after the candidate's action less its wins after the control's, over
//! `FORK_ROLLOUTS`. The control plays both arms on, so a fork measures the changed move alone and
//! not what the changed AI would do later: a change whose gain compounds reads low, never high.
//! Rejected: playing the candidate's arm on with the candidate (the arms would then differ by more
//! than the fork, and the read is the deal screen's again), and forcing the knob's cards into the
//! deals (helps only knobs tied to named cards; #4246's change diverged in 21 deals unaided).
//!
//! Every rollout arm that throws, falls back, catches a search panic or aborts (R1489000)
//! contributes no win and is counted and named by its deal, fork and pair. Arms assemble in
//! `(deal, fork, pair, arm)` order, so the tally is byte-identical whatever the thread count.

use std::collections::BTreeMap;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, bail};
use jackioh_ai::{AI_GATE_BUDGET, AI_SKILL_MAX, SearchBudget};
use jackioh_engine::{
    Action, ActionBody, GameRecord, GameState, PerPlayer, PlayerId, ReplayInput, Rng, fold,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::agent::AgentInfo;
use crate::arena::{
    AgentSpec, ArenaGame, ArenaOutcome, Entrant, FinishCtx, Live, SCREEN_EXTEND_DEALS, SCREEN_MAX_GAMES,
    SCREEN_MIN_DECIDED, accept, append_records, choose_for, duel_line, entrant_at, finish_from, game_setup,
    paired_interval, play_game_probed, records_of, records_path, replays, round_share, screen_done, won_by,
};

/// R4397000: the most forks one trunk game keeps, its first ones.
pub(crate) const FORK_MAX_PER_GAME: usize = 2;

/// R4397000: the pairs each fork plays, once after each action.
pub(crate) const FORK_ROLLOUTS: usize = 8;

/// R4397000: the trunk deals the fork screen plays by default, each once: the dev set's deals
/// (`training/improve.md`, How you measure), the same deals the paired screen plays twice.
pub(crate) const FORK_TRUNK_DEALS: i32 = 200;

/// R4607000: the save file's format version, in its header: a save written by another version is
/// refused, since its lines may read differently.
pub(crate) const FORK_SAVE_VERSION: u32 = 1;

/// R4607000: the save file's first line: what the run that wrote it screened, so running the same
/// command again on the file refuses a run that would play something else, naming the field.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveHeader {
    pub v: u32,
    pub set: String,
    pub games: i32,
    pub extend: bool,
    pub budget: SearchBudget,
    pub fork_rollouts: usize,
    pub fork_max_per_game: usize,
    pub catalog: String,
    pub a_label: String,
    pub b_label: String,
    /// The agents' `info` answers: `None` for `random`, which is no generation of the AI.
    pub a_info: Option<AgentInfo>,
    pub b_info: Option<AgentInfo>,
}

/// R4607000: one saved fork: where the candidate differed and the two actions it is played both
/// ways from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedFork {
    pub seat: PlayerId,
    pub at_move: usize,
    pub candidate: Option<ActionBody>,
    pub control: Option<ActionBody>,
}

/// R4607000: what one rollout arm came to, in its fork's `(pair, arm)` slot order (the
/// candidate's arm then the control's, pair by pair). A faulted arm contributes no win.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedArm {
    pub won: bool,
    pub fault: Option<String>,
}

/// R4607000: one finished deal as the save file holds it: its number, its duel line (which
/// carries the decision times, so the saved string is replayed, never recomputed), its record,
/// its trunk hash, its forks and each arm's win and fault. `rollouts` counts the arms that
/// played (faulted ones included), as the tally's `rollouts` does: slots that faulted before
/// they could play are faults but no rollouts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DealLine {
    pub deal: i32,
    pub duel: String,
    pub record: Option<GameRecord>,
    pub hash: String,
    pub forks: Vec<SavedFork>,
    pub probe_faults: Vec<String>,
    pub arms: Vec<SavedArm>,
    pub rollouts: usize,
}

/// R4607000: what resuming a save played and read: the tally and the duel lines in deal
/// order, and how many games this call played (none when the save was already finished). The
/// records stay in the save file, which is the record store.
#[derive(Debug)]
pub(crate) struct SaveReport {
    pub tally: String,
    pub duels: Vec<String>,
    pub games_played: usize,
}

/// What a scripted probe answers with: the deal, the move and the control's final played body.
type ScriptedAnswer = fn(i32, usize, &ActionBody) -> Option<ActionBody>;

/// One trunk game played with its probe: the outcome, the forks and the probe faults.
type TrunkPlayed = (ArenaOutcome, Vec<ForkPoint>, Vec<String>);

/// The action's identity for the fork comparison: the canonical JSON of a body
/// (`jackioh_ai::action_key`), `"none"` for "no move".
pub(crate) fn fork_key(action: &Option<ActionBody>) -> String {
    match action {
        None => "none".to_string(),
        Some(body) => jackioh_ai::action_key(body),
    }
}

/// Where the candidate would have made a different move: the trunk decision both actions answer,
/// with the candidate's and the control's final played bodies (`None` for "no move").
#[derive(Clone, Debug)]
pub(crate) struct ForkPoint {
    /// 1-based: the trunk game (hence the deal) the fork was found in.
    pub deal: i32,
    /// 1-based within the game.
    pub fork_no: usize,
    pub seat: PlayerId,
    /// The trunk log's length at the decision.
    pub at_move: usize,
    pub candidate: Option<ActionBody>,
    pub control: Option<ActionBody>,
}

/// Asks the candidate at each trunk decision until the game's fork cap (R4397000).
pub(crate) struct Probe {
    live: Option<Live>,
    scripted: Option<ScriptedAnswer>,
    budget: SearchBudget,
    deal: i32,
    forks_this_game: usize,
    pub points: Vec<ForkPoint>,
    pub faults: Vec<String>,
}

impl Probe {
    /// Finishes the probe's candidate agent at the trunk game's end, mirroring `Live::finish`:
    /// a `bin:` child gets its `quit`, so no agent process outlives its game (R4397000).
    pub(crate) fn finish(mut self) -> (Vec<ForkPoint>, Vec<String>) {
        if let Some(live) = self.live.take() {
            live.finish();
        }
        (self.points, self.faults)
    }

    /// A probe of `candidate` at `budget` for trunk deal `deal`. Spawning a `bin:` candidate that
    /// cannot start fails the run before any game is counted, as a seated agent's does.
    pub(crate) fn for_game(candidate: &AgentSpec, budget: SearchBudget, deal: i32) -> anyhow::Result<Probe> {
        Ok(Probe {
            live: Some(Live::start(candidate)?),
            scripted: None,
            budget,
            deal,
            forks_this_game: 0,
            points: Vec::new(),
            faults: Vec::new(),
        })
    }

    /// A probe answering `f` of the deal, the move and the control's final played body, for the
    /// tests. Echoing the control is `|_, _, control| Some(control.clone())`: a scripted candidate
    /// that truly has no move answers `None`.
    #[cfg(test)]
    pub(crate) fn scripted(deal: i32, answered: ScriptedAnswer) -> Probe {
        Probe {
            live: None,
            scripted: Some(answered),
            budget: jackioh_ai::AI_GATE_BUDGET,
            deal,
            forks_this_game: 0,
            points: Vec::new(),
            faults: Vec::new(),
        }
    }

    /// Asks the candidate the same request the control's seat got — the same redacted view, seat,
    /// stream seed and pre-decision cursor — and records the fork when its action differs from the
    /// control's final played body. The trunk always plays on with the control's action. A
    /// throwing or erroring candidate query is recorded and yields no fork.
    pub(crate) fn check(
        &mut self,
        state: &GameState,
        seat: PlayerId,
        ctl_seed: &str,
        rng: &Rng,
        control: &ActionBody,
        at_move: usize,
    ) {
        if self.forks_this_game >= FORK_MAX_PER_GAME {
            return;
        }
        let candidate = match (self.live.as_mut(), self.scripted) {
            (_, Some(answered)) => answered(self.deal, at_move, control),
            (Some(live), None) => {
                let mut moved = rng.clone();
                // At full skill (CLAUDE.md rule 9's `AI_SKILL_MAX`), exactly as `choose_for` asks a
                // seated agent: the candidate sees what the control's seat saw.
                match choose_for(live, AI_SKILL_MAX, state, seat, ctl_seed, &mut moved, self.budget) {
                    Ok(choice) => choice.action,
                    Err(fault) => {
                        self.faults.push(format!(
                            "deal {} move {at_move}: candidate probe: {fault}",
                            self.deal
                        ));
                        return;
                    }
                }
            }
            (None, None) => return,
        };
        if fork_key(&candidate) != fork_key(&Some(control.clone())) {
            self.forks_this_game += 1;
            self.points.push(ForkPoint {
                deal: self.deal,
                fork_no: self.forks_this_game,
                seat,
                at_move,
                candidate,
                control: Some(control.clone()),
            });
        }
    }
}

/// One arm of a fork pair: after the candidate's action or after the control's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Arm {
    Cand,
    Ctl,
}

impl Arm {
    fn as_str(self) -> &'static str {
        match self {
            Arm::Cand => "cand",
            Arm::Ctl => "ctl",
        }
    }
}

/// What one rollout arm came to. A faulted arm contributes no win; its fault names its deal, fork,
/// pair and arm (R1489000).
pub(crate) struct ArmOutcome {
    /// The arm's final hash and log, for the tests that rerun a pair to show both arms start from
    /// the same state and play on the same streams; the tally reads only `won` and `fault`.
    #[cfg(test)]
    pub hash: String,
    #[cfg(test)]
    pub log: Vec<Action>,
    /// Whether the forking seat won: false when faulted, drawn or without a result.
    pub won: bool,
    pub fault: Option<String>,
}

/// Refolds the trunk game's prefix before the fork: the state both arms start from (R4397000).
fn refold(game: &ArenaGame, log: &[Action]) -> jackioh_engine::ReplayResult {
    let input = ReplayInput {
        seed: game.seed.clone(),
        decks: game.decks.clone(),
        log: log.to_vec(),
        ..Default::default()
    };
    fold(&input)
}

/// An arm ready to play on: the fork's state after the arm's first action, with the prefix log.
struct ArmJob {
    game: ArenaGame,
    deal: i32,
    fork_no: usize,
    pair: usize,
    arm: Arm,
    seat: PlayerId,
    log: Vec<Action>,
    state: GameState,
}

/// A slot of a fork's pair in `(pair, arm)` order: an arm to play, or the fault it faulted with
/// before it could play. The job rides boxed: it holds the game and its state, thousands of times
/// the fault's size.
enum ArmSlot {
    Play(Box<ArmJob>),
    Fault(String),
}

/// Each arm's stream seeds: pair r plays both arms on the same streams, and every pair on its own.
fn arm_streams(seed: &str, fork_no: usize, pair: usize) -> (String, String) {
    (
        format!("{seed}:fork:{fork_no}:{pair}:ctl:p1"),
        format!("{seed}:fork:{fork_no}:{pair}:ctl:p2"),
    )
}

/// Builds a fork's slots: one refold for the pair's shared state, each arm's first action through
/// the referee's `accept` with the trunk's nonce (R4397000). A refold error faults every arm of
/// every pair, so each pair still holds its two slots; a `None` first action or an action with no
/// legal replacement faults that arm.
fn fork_slots(outcome: &ArenaOutcome, fork: &ForkPoint) -> Vec<ArmSlot> {
    let prefix = &outcome.log[..fork.at_move.min(outcome.log.len())];
    let refolded = refold(&outcome.game, prefix);
    if let Some(first) = refolded.errors.first() {
        let mut slots = Vec::with_capacity(FORK_ROLLOUTS * 2);
        for pair in 1..=FORK_ROLLOUTS {
            for arm in [Arm::Cand, Arm::Ctl] {
                slots.push(ArmSlot::Fault(format!(
                    "deal {} fork {} pair {pair} {}: refold: {}: {}",
                    fork.deal,
                    fork.fork_no,
                    arm.as_str(),
                    first.nonce,
                    first.error
                )));
            }
        }
        return slots;
    }
    let mut slots = Vec::with_capacity(FORK_ROLLOUTS * 2);
    for pair in 1..=FORK_ROLLOUTS {
        for (arm, first) in [(Arm::Cand, &fork.candidate), (Arm::Ctl, &fork.control)] {
            let name = || {
                format!(
                    "deal {} fork {} pair {pair} {}",
                    fork.deal,
                    fork.fork_no,
                    arm.as_str()
                )
            };
            let Some(action) = first else {
                slots.push(ArmSlot::Fault(format!("{}: no move", name())));
                continue;
            };
            let nonce = format!("m{}", prefix.len());
            let mut rejected = Vec::new();
            let accepted = catch_unwind(AssertUnwindSafe(|| {
                accept(&refolded.state, fork.seat, action, &nonce, &mut rejected)
            }));
            match accepted {
                Ok(Some((played, state))) => {
                    let mut log = prefix.to_vec();
                    log.push(played);
                    slots.push(ArmSlot::Play(Box::new(ArmJob {
                        game: outcome.game.clone(),
                        deal: fork.deal,
                        fork_no: fork.fork_no,
                        pair,
                        arm,
                        seat: fork.seat,
                        log,
                        state,
                    })));
                }
                Ok(None) => slots.push(ArmSlot::Fault(format!(
                    "{}: no legal replacement for a refused \"{}\"",
                    name(),
                    action.action_type().as_str()
                ))),
                Err(panic) => slots.push(ArmSlot::Fault(format!(
                    "{}: reduce threw on \"{}\": {}",
                    name(),
                    action.action_type().as_str(),
                    jackioh_ai::panic_message(&*panic)
                ))),
            }
        }
    }
    slots
}

/// Plays one arm to the end with the control on both seats (R4397000): the trunk's skills, budget
/// and unsettled handling, fresh fork streams, this turn's count restarted (the ceiling still
/// bounds the arm, from a full turn's allowance).
fn play_arm(job: ArmJob) -> anyhow::Result<ArmOutcome> {
    let (p1, p2) = arm_streams(&job.game.seed, job.fork_no, job.pair);
    let ctl = PerPlayer::new(p1, p2);
    let ctx = FinishCtx {
        rngs: PerPlayer::new(Rng::new(&ctl.p1, 0), Rng::new(&ctl.p2, 0)),
        ctl,
        live: PerPlayer::new(Live::start(&job.game.seats.p1)?, Live::start(&job.game.seats.p2)?),
        skills: PerPlayer::new(job.game.skills[PlayerId::P1], job.game.skills[PlayerId::P2]),
        budget: job.game.self_budget,
        turn_actions: job.game.turn_actions,
        stop_unsettled: job.game.stop_unsettled,
        log: job.log,
        rejected: Vec::new(),
        thrown: Vec::new(),
        fallbacks: PerPlayer::new(0, 0),
        sim_panics: PerPlayer::new(0, 0),
        first_sim_panic: None,
        times: PerPlayer::new(Vec::new(), Vec::new()),
        turn: job.state.turn,
        this_turn: 0,
        probe: None,
    };
    let tail = finish_from(job.state, ctx);
    let name = format!(
        "deal {} fork {} pair {} {}",
        job.deal,
        job.fork_no,
        job.pair,
        job.arm.as_str()
    );
    let mut reasons: Vec<String> = tail.thrown.clone();
    let fallbacks = tail.fallbacks.p1 + tail.fallbacks.p2;
    if fallbacks > 0 {
        reasons.push(format!("{fallbacks} fallbacks"));
    }
    let sim_panics = tail.sim_panics.p1 + tail.sim_panics.p2;
    if sim_panics > 0 {
        reasons.push(format!("{sim_panics} search panics"));
    }
    if tail.result.is_none() && tail.thrown.is_empty() {
        reasons.push(format!("no result after {} actions", tail.log.len()));
    }
    let fault = (!reasons.is_empty()).then(|| format!("{}: {}", name, reasons.join("; ")));
    Ok(ArmOutcome {
        #[cfg(test)]
        hash: tail.hash,
        #[cfg(test)]
        log: tail.log,
        won: fault.is_none() && won_by(tail.result, job.seat),
        fault,
    })
}

/// One fork with its rollout edge: the forking seat's candidate-arm wins less its control-arm
/// wins, over `FORK_ROLLOUTS` (-1 to 1).
pub(crate) struct ForkEdge {
    pub deal: i32,
    pub edge: f64,
}

/// Plays one pair of a fork's arms, the candidate's then the control's, for the tests that rerun a
/// pair to show both arms start from the same state and play on the same streams.
#[cfg(test)]
pub(crate) fn play_pair(
    outcome: &ArenaOutcome,
    fork: &ForkPoint,
    pair: usize,
) -> anyhow::Result<(ArmOutcome, ArmOutcome)> {
    assert!(
        (1..=FORK_ROLLOUTS).contains(&pair),
        "pair {pair} is not in 1..={FORK_ROLLOUTS}"
    );
    let mut slots = fork_slots(outcome, fork).into_iter().skip((pair - 1) * 2).take(2);
    let mut arms = Vec::with_capacity(2);
    for slot in slots.by_ref() {
        match slot {
            ArmSlot::Play(job) => arms.push(play_arm(*job)?),
            ArmSlot::Fault(fault) => arms.push(ArmOutcome {
                hash: String::new(),
                log: Vec::new(),
                won: false,
                fault: Some(fault),
            }),
        }
    }
    assert_eq!(
        arms.len(),
        2,
        "pair {pair} of deal {} fork {}",
        fork.deal,
        fork.fork_no
    );
    let mut arms = arms.into_iter();
    Ok((arms.next().unwrap(), arms.next().unwrap()))
}

/// R4607000: what one rollout slot came to: whether its arm won and faulted, and whether it
/// played at all. A slot that faulted before it could play (a refold error, no move, no legal
/// replacement, a refused reduce) is a fault but no rollout; an arm that played and faulted on
/// the way is both.
struct SlotDone {
    won: bool,
    fault: Option<String>,
    played: bool,
}

/// Plays every fork's pairs and reads each fork's edge (R4397000). Slots build sequentially in
/// `(deal, fork, pair, arm)` order; arms play on rayon's pool but assemble in that same order, so
/// completion order never reaches the output. Returns the edges in fork order, the faults in slot
/// order (refold and accept faults among them), and the arms that played, faulted arms included. A
/// control agent that cannot start fails the run before anything is counted, as a trunk game's does.
pub(crate) fn play_forks(
    jobs: &[(&ArenaOutcome, &ForkPoint)],
) -> anyhow::Result<(Vec<ForkEdge>, Vec<String>, usize)> {
    let grid = play_arm_grid(jobs)?;
    let mut edges = Vec::with_capacity(jobs.len());
    let mut faults = Vec::new();
    let mut rollouts = 0usize;
    for (index, row) in grid.iter().enumerate() {
        let (_, fork) = jobs[index];
        edges.push(ForkEdge {
            deal: fork.deal,
            edge: edge_of_slots(&row.iter().map(|slot| slot.won).collect::<Vec<_>>()),
        });
        for slot in row {
            if let Some(fault) = &slot.fault {
                faults.push(fault.clone());
            }
            rollouts += usize::from(slot.played);
        }
    }
    Ok((edges, faults, rollouts))
}

/// R4607000: one fork's edge from its arms' wins in `(pair, arm)` slot order: the forking seat's
/// candidate-arm wins less its control-arm wins, over `FORK_ROLLOUTS` (-1 to 1). Slots run (cand,
/// ctl) per pair, so even slots are the candidate's arms. Shared by the live tally and the saved
/// one, so the two cannot diverge.
fn edge_of_slots(won: &[bool]) -> f64 {
    let (mut cand_wins, mut ctl_wins) = (0i32, 0i32);
    for (slot, win) in won.iter().enumerate() {
        if *win {
            if slot % 2 == 0 {
                cand_wins += 1;
            } else {
                ctl_wins += 1;
            }
        }
    }
    f64::from(cand_wins - ctl_wins) / FORK_ROLLOUTS as f64
}

/// Plays every fork's pairs into their slots: one `SlotDone` per `(pair, arm)` slot, in fork
/// order. What assembly order the pool finishes in never reaches the grid. R4607000: the save
/// path plays one deal's forks through this, so a saved arm is the arm the live tally read.
fn play_arm_grid(jobs: &[(&ArenaOutcome, &ForkPoint)]) -> anyhow::Result<Vec<Vec<SlotDone>>> {
    let mut slots: Vec<Vec<ArmSlot>> = jobs
        .iter()
        .map(|(outcome, fork)| fork_slots(outcome, fork))
        .collect();
    // The playable arms, each tagged with its (fork, slot) home.
    let mut runs: Vec<(usize, usize, Box<ArmJob>)> = Vec::new();
    for (fork, arms) in slots.iter_mut().enumerate() {
        for (slot, arm) in arms.iter_mut().enumerate() {
            let taken = std::mem::replace(arm, ArmSlot::Fault(String::new()));
            match taken {
                ArmSlot::Play(job) => runs.push((fork, slot, job)),
                fault => *arm = fault,
            }
        }
    }
    let mut grid: Vec<Vec<Option<ArmOutcome>>> = slots
        .iter()
        .map(|arms| arms.iter().map(|_| None).collect())
        .collect();
    let outcomes: Vec<(usize, usize, ArmOutcome)> = runs
        .into_par_iter()
        .map(|(fork, slot, job)| play_arm(*job).map(|done| (fork, slot, done)))
        .collect::<anyhow::Result<Vec<_>>>()?;
    for (fork, slot, done) in outcomes {
        grid[fork][slot] = Some(done);
    }
    let mut out = Vec::with_capacity(jobs.len());
    for (arms, row) in slots.iter().zip(grid.iter()) {
        let mut fork_done = Vec::with_capacity(arms.len());
        for (arm, done) in arms.iter().zip(row.iter()) {
            match (arm, done) {
                (_, Some(done)) => fork_done.push(SlotDone {
                    won: done.won,
                    fault: done.fault.clone(),
                    played: true,
                }),
                (ArmSlot::Fault(fault), None) if !fault.is_empty() => fork_done.push(SlotDone {
                    won: false,
                    fault: Some(fault.clone()),
                    played: false,
                }),
                // No playable arm and no fault: no slot builder leaves this, but an empty
                // placeholder must not read as a played arm.
                _ => fork_done.push(SlotDone {
                    won: false,
                    fault: None,
                    played: false,
                }),
            }
        }
        out.push(fork_done);
    }
    Ok(out)
}

/// The fork screen's tally, closing its trunk duel lines: the trunk games and forks, the live
/// forks (whose edge is not 0), the forks per game, the rollout arms played (faulted ones
/// included), and the fork edge — the mean over forked deals of the deal's forks' mean edge — with
/// its 95% interval over the forked deals (a deal's forks are not independent, so each deal counts
/// once). `faults` holds probe faults in trunk order, then rollout faults in slot order.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForkTally {
    pub a: String,
    pub b: String,
    pub trunk_games: usize,
    pub forks: usize,
    pub live_forks: usize,
    pub forks_per_game: f64,
    pub rollouts: usize,
    pub fork_edge: f64,
    pub fork_edge_low: f64,
    pub fork_edge_high: f64,
    pub faults: Vec<String>,
    /// On a screen with fewer than `SCREEN_MIN_DECIDED` live forks, which says nothing; absent
    /// otherwise, as the deal screen's `inconclusive` is (R4319000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inconclusive: Option<bool>,
}

pub(crate) fn fork_tally(
    a: &str,
    b: &str,
    trunk_games: usize,
    edges: &[ForkEdge],
    rollouts: usize,
    faults: Vec<String>,
) -> ForkTally {
    let forks = edges.len();
    let live_forks = edges.iter().filter(|edge| edge.edge != 0.0).count();
    // One mean per forked deal, in deal order: one game's forks are not independent.
    let mut deals: Vec<(i32, f64, usize)> = Vec::new();
    for edge in edges {
        match deals.last_mut() {
            Some(last) if last.0 == edge.deal => {
                last.1 += edge.edge;
                last.2 += 1;
            }
            _ => deals.push((edge.deal, edge.edge, 1)),
        }
    }
    for (_, sum, count) in &mut deals {
        *sum /= *count as f64;
    }
    let means: Vec<f64> = deals.iter().map(|(_, mean, _)| *mean).collect();
    let (edge, edge_low, edge_high) = paired_interval(&means);
    ForkTally {
        a: a.to_string(),
        b: b.to_string(),
        trunk_games,
        forks,
        live_forks,
        forks_per_game: round_share(if trunk_games == 0 {
            0.0
        } else {
            forks as f64 / trunk_games as f64
        }),
        rollouts,
        fork_edge: round_share(edge),
        fork_edge_low: round_share(edge_low),
        fork_edge_high: round_share(edge_high),
        faults,
        inconclusive: (live_forks < SCREEN_MIN_DECIDED).then_some(true),
    }
}

/// R4607000: plays one deal as a unit: its trunk game, then its forks' arms. Returns the
/// deal's save line: the duel line with its decision times, the game record, the trunk hash,
/// the forks and each arm's win and fault.
pub(crate) fn play_deal(game: &ArenaGame, mut probe: Probe) -> anyhow::Result<DealLine> {
    probe.deal = game.n;
    let (outcome, points, probe_faults) = play_game_probed(game, Some(probe))?;
    let jobs: Vec<(&ArenaOutcome, &ForkPoint)> = points.iter().map(|point| (&outcome, point)).collect();
    let grid = play_arm_grid(&jobs)?;
    let mut arms = Vec::new();
    let mut rollouts = 0usize;
    for row in &grid {
        for slot in row {
            arms.push(SavedArm {
                won: slot.won,
                fault: slot.fault.clone(),
            });
            rollouts += usize::from(slot.played);
        }
    }
    Ok(DealLine {
        deal: game.n,
        duel: duel_line(&outcome, replays(&outcome)?)?,
        record: crate::arena::game_record(&outcome)?,
        hash: outcome.hash.clone(),
        forks: points
            .iter()
            .map(|point| SavedFork {
                seat: point.seat,
                at_move: point.at_move,
                candidate: point.candidate.clone(),
                control: point.control.clone(),
            })
            .collect(),
        probe_faults,
        arms,
        rollouts,
    })
}

/// R4607000: appends one finished deal's line to the save file and flushes it, so a killed run
/// loses at most its unfinished deals.
fn append_deal_line(path: &Path, line: &DealLine) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("arena: cannot open {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(line)?)?;
    file.flush()
        .with_context(|| format!("arena: cannot write {}", path.display()))?;
    Ok(())
}

/// R4607000: reads a save file: its header and its deal lines by deal number. A last line that
/// does not parse is the torn tail a killed run leaves and is dropped with a warning; a failing
/// earlier line and a repeated deal number are errors naming the line, since the file is not the
/// run's own torn tail.
fn load_save(path: &Path) -> anyhow::Result<(SaveHeader, BTreeMap<i32, DealLine>, bool)> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("arena: cannot read {}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("arena: save {} is empty", path.display()))?;
    let header: SaveHeader = serde_json::from_str(header)
        .with_context(|| format!("arena: save {}'s header does not parse", path.display()))?;
    let rest: Vec<&str> = lines.collect();
    let mut map = BTreeMap::new();
    let mut torn = false;
    for (index, line) in rest.iter().enumerate() {
        // The header is line 1, so the first deal line is line 2.
        let at = index + 2;
        let is_last = index + 1 == rest.len();
        match serde_json::from_str::<DealLine>(line) {
            Ok(deal) => {
                if map.insert(deal.deal, deal).is_some() {
                    bail!("arena: save {} repeats a deal on line {at}", path.display());
                }
            }
            Err(_) if is_last => {
                eprintln!(
                    "arena: save {}'s last line is torn and is played again",
                    path.display()
                );
                torn = true;
            }
            Err(error) => {
                bail!(
                    "arena: save {}'s line {at} does not parse: {error}",
                    path.display()
                );
            }
        }
    }
    for (deal, line) in &map {
        if line.arms.len() != line.forks.len() * FORK_ROLLOUTS * 2 {
            bail!(
                "arena: save {}'s deal {deal} holds {} arms for {} forks",
                path.display(),
                line.arms.len(),
                line.forks.len()
            );
        }
    }
    Ok((header, map, torn))
}

/// R4607000: refuses a save that is not this run's, naming the first field that differs, before
/// any game is played.
fn check_header(saved: &SaveHeader, expected: &SaveHeader, path: &Path) -> anyhow::Result<()> {
    let at = path.display();
    if saved.v != expected.v {
        bail!("arena: save {at} is version {}, not {}", saved.v, expected.v);
    }
    if saved.set != expected.set {
        bail!("arena: save {at} is for set {}, not {}", saved.set, expected.set);
    }
    if saved.games != expected.games {
        bail!(
            "arena: save {at} is for --games {}, not {}",
            saved.games,
            expected.games
        );
    }
    if saved.extend != expected.extend {
        bail!(
            "arena: save {at} is for --extend {}, not {}",
            saved.extend,
            expected.extend
        );
    }
    if saved.budget != expected.budget {
        bail!(
            "arena: save {at} is for budget {:?}, not {:?}",
            saved.budget,
            expected.budget
        );
    }
    if saved.fork_rollouts != expected.fork_rollouts {
        bail!(
            "arena: save {at} is for FORK_ROLLOUTS {}, not {}",
            saved.fork_rollouts,
            expected.fork_rollouts
        );
    }
    if saved.fork_max_per_game != expected.fork_max_per_game {
        bail!(
            "arena: save {at} is for FORK_MAX_PER_GAME {}, not {}",
            saved.fork_max_per_game,
            expected.fork_max_per_game
        );
    }
    if saved.catalog != expected.catalog {
        bail!(
            "arena: save {at} is for catalog {}, not {}",
            saved.catalog,
            expected.catalog
        );
    }
    if saved.a_label != expected.a_label {
        bail!(
            "arena: save {at} is for candidate {}, not {}",
            saved.a_label,
            expected.a_label
        );
    }
    if saved.b_label != expected.b_label {
        bail!(
            "arena: save {at} is for control {}, not {}",
            saved.b_label,
            expected.b_label
        );
    }
    if saved.a_info != expected.a_info {
        bail!("arena: save {at} is for another candidate's info, not this one's");
    }
    if saved.b_info != expected.b_info {
        bail!("arena: save {at} is for another control's info, not this one's");
    }
    Ok(())
}

/// R4607000: the identity check: plays the first saved deal's trunk again and refuses the file
/// unless its hash and its forks match. This is one game, and it catches a rebuilt binary of
/// another AI behind the same labels.
fn identity_check(game: &ArenaGame, probe: Probe, saved: &DealLine) -> anyhow::Result<()> {
    let (outcome, points, _) = play_game_probed(game, Some(probe))?;
    if outcome.hash != saved.hash {
        bail!(
            "arena: save: deal {}'s trunk hash is {}, not the saved {}",
            saved.deal,
            outcome.hash,
            saved.hash
        );
    }
    let same = points.len() == saved.forks.len()
        && points.iter().zip(saved.forks.iter()).all(|(point, fork)| {
            point.seat == fork.seat
                && point.at_move == fork.at_move
                && fork_key(&point.candidate) == fork_key(&fork.candidate)
                && fork_key(&point.control) == fork_key(&fork.control)
        });
    if !same {
        bail!(
            "arena: save: deal {}'s forks differ from the saved ones",
            saved.deal
        );
    }
    Ok(())
}

/// R4607000: the saved deals' edges in deal order, recomputed from the saved arms through the
/// same `edge_of_slots` the live tally reads.
fn edges_of(lines: &[&DealLine]) -> Vec<ForkEdge> {
    let mut edges = Vec::new();
    for line in lines {
        for fork in 0..line.forks.len() {
            let arms = &line.arms[fork * FORK_ROLLOUTS * 2..(fork + 1) * FORK_ROLLOUTS * 2];
            edges.push(ForkEdge {
                deal: line.deal,
                edge: edge_of_slots(&arms.iter().map(|arm| arm.won).collect::<Vec<_>>()),
            });
        }
    }
    edges
}

/// R4607000: the tally and the duel lines from the saved deals' lines in deal order: what
/// one uninterrupted run prints. Probe faults read in deal order, then arm faults in slot
/// order, as the live screen's single batch orders them; the duel lines are the saved strings,
/// since they carry the decision times.
fn tally_from_lines(header: &SaveHeader, lines: &[&DealLine]) -> anyhow::Result<(String, Vec<String>)> {
    let edges = edges_of(lines);
    let rollouts = lines.iter().map(|line| line.rollouts).sum();
    let mut faults = Vec::new();
    for line in lines {
        faults.extend(line.probe_faults.iter().cloned());
    }
    for line in lines {
        for arm in &line.arms {
            if let Some(fault) = &arm.fault {
                faults.push(fault.clone());
            }
        }
    }
    let tally = fork_tally(
        &header.a_label,
        &header.b_label,
        lines.len(),
        &edges,
        rollouts,
        faults,
    );
    Ok((
        serde_json::to_string(&tally)?,
        lines.iter().map(|line| line.duel.clone()).collect(),
    ))
}

/// R4607000: the next batch after `games_done` games with `live` live forks, matching the live
/// screen's stops: the first batch plays `initial_deals`, later batches only with `--extend`,
/// none once the screen is decided or capped. `--extend` totals count the saved deals and the
/// new ones together, since `games_done` covers both.
fn next_batch(games_done: i32, live: usize, extend: bool, initial_deals: i32) -> Option<(i32, i32)> {
    if games_done == 0 {
        return Some((1, initial_deals));
    }
    if !extend {
        return None;
    }
    if screen_done(live, games_done) {
        return None;
    }
    let len = (SCREEN_MAX_GAMES - games_done).min(SCREEN_EXTEND_DEALS);
    if len <= 0 {
        return None;
    }
    Some((games_done + 1, len))
}

/// R4607000: resumes a save file: refuses a foreign file by field name, replays nothing when the
/// save is finished (a finished save is never read a second time, R850001), else plays the first
/// saved deal's trunk once as an identity check and then only the deals the file is missing, in
/// batches the live screen would stop at. Each finished deal is appended and flushed at once;
/// readers sort by deal, so completion order never reaches the output.
pub(crate) fn resume_save(
    path: &Path,
    expected: &SaveHeader,
    base: &str,
    b: &Entrant,
    initial_deals: i32,
    extend: bool,
    make_probe: &(dyn Fn(&ArenaGame) -> anyhow::Result<Probe> + Sync),
) -> anyhow::Result<SaveReport> {
    let mut map: BTreeMap<i32, DealLine> = if path.exists() {
        let (header, map, torn) = load_save(path)?;
        check_header(&header, expected, path)?;
        if torn {
            // Heal the file the killed run left: the torn tail is dropped and the kept
            // lines are rewritten in deal order, so the next append lands on whole lines.
            let mut healed = serde_json::to_string(&header)?;
            healed.push('\n');
            for line in map.values() {
                healed.push_str(&serde_json::to_string(line)?);
                healed.push('\n');
            }
            std::fs::write(path, healed)
                .with_context(|| format!("arena: cannot write {}", path.display()))?;
        }
        map
    } else {
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("arena: cannot create {}", dir.display()))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .with_context(|| format!("arena: cannot open {}", path.display()))?;
        writeln!(file, "{}", serde_json::to_string(expected)?)?;
        file.flush()
            .with_context(|| format!("arena: cannot write {}", path.display()))?;
        BTreeMap::new()
    };
    let live_of = |map: &BTreeMap<i32, DealLine>| {
        edges_of(&map.values().collect::<Vec<_>>())
            .iter()
            .filter(|edge| edge.edge != 0.0)
            .count()
    };
    let mut target = initial_deals;
    let mut games_played = 0usize;
    // A finished save plays nothing and reprints its tally, without even the identity game.
    if (1..=target).all(|deal| map.contains_key(&deal))
        && next_batch(target, live_of(&map), extend, initial_deals).is_none()
    {
        let (tally, duels) = tally_from_lines(expected, &map.values().collect::<Vec<_>>())?;
        return Ok(SaveReport {
            tally,
            duels,
            games_played,
        });
    }
    if let Some(first) = map.keys().next().copied() {
        let mut game = trunk_setups(b, base, first, 1)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("arena: save: deal {first} has no setup"))?;
        game.self_budget = expected.budget;
        let saved = map[&first].clone();
        identity_check(&game, make_probe(&game)?, &saved)?;
    }
    loop {
        let missing: Vec<i32> = (1..=target).filter(|deal| !map.contains_key(deal)).collect();
        if !missing.is_empty() {
            let played: Vec<(i32, DealLine)> = missing
                .par_iter()
                .map(|&deal| {
                    let mut game = trunk_setups(b, base, deal, 1)?
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("arena: save: deal {deal} has no setup"))?;
                    game.self_budget = expected.budget;
                    play_deal(&game, make_probe(&game)?).map(|line| (deal, line))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            // One writer at a time, each line flushed at once; readers sort by deal.
            static APPEND: Mutex<()> = Mutex::new(());
            let _guard = APPEND
                .lock()
                .map_err(|_| anyhow::anyhow!("arena: save: the save writer is poisoned"))?;
            for (deal, line) in played {
                append_deal_line(path, &line)?;
                map.insert(deal, line);
            }
            games_played += missing.len();
        }
        match next_batch(target, live_of(&map), extend, initial_deals) {
            None => break,
            Some((_, len)) => {
                target += len;
            }
        }
    }
    let (tally, duels) = tally_from_lines(expected, &map.values().collect::<Vec<_>>())?;
    Ok(SaveReport {
        tally,
        duels,
        games_played,
    })
}

/// Trunk deal games `first..first+batch`, each played once with the control on both seats: the
/// game `game_setup` deals for the control's shadow ban on each seat, with both record labels the
/// control's, so the records read control-vs-control honestly (R4397000).
fn trunk_setups(b: &Entrant, base: &str, first: i32, batch: i32) -> anyhow::Result<Vec<ArenaGame>> {
    (0..batch)
        .map(|i| {
            let n = first + i;
            game_setup(format!("{base}:{n}"), n, false, b, b)
        })
        .collect()
}

/// R4607000: an entrant's `info` answer for the save header: `None` for `random`, which is no
/// generation of the AI.
fn info_of(entrant: &Entrant) -> Option<AgentInfo> {
    Some(AgentInfo {
        generation: entrant.generation?,
        lane: entrant.lane.clone()?,
        shadow_ban: entrant.shadow_ban.clone(),
    })
}

/// Plays the trunk games in parallel, each with its own candidate probe, collecting outcomes and
/// points in game order (R4397000). A candidate that cannot start fails the run before any game is
/// counted, as a seated agent's does.
fn play_trunk(games: &[ArenaGame], candidate: &AgentSpec) -> anyhow::Result<Vec<TrunkPlayed>> {
    games
        .par_iter()
        .map(|game| {
            let probe = Probe::for_game(candidate, game.self_budget, game.n)?;
            play_game_probed(game, Some(probe))
        })
        .collect()
}

/// Runs the fork screen: the trunk, each fork's rollout pairs, and the tally closing the trunk
/// duel lines (R4397000). `--extend` plays on in 100-deal batches until `SCREEN_MIN_DECIDED` forks
/// are live or `SCREEN_MAX_GAMES` games are played, reading the edge only at the end; the stop
/// looks at how many forks are live, never which way they went.
pub fn run(args: crate::arena::Args) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let Some(set) = args.set else {
        bail!("arena: --forks screens a seed set, so it needs --set improve-dev");
    };
    if args.paired {
        bail!("arena: --forks plays each deal once, so it takes no --paired");
    }
    if args.a_skill != AI_SKILL_MAX {
        bail!("arena: --forks probes the candidate's AI at full skill, so --a-skill stays 1");
    }
    let deals = args.games.unwrap_or(FORK_TRUNK_DEALS);
    if deals < 1 {
        bail!("arena: --games must be at least 1, not {deals}");
    }
    let base = set.base().to_string();
    let a = entrant_at(&args.a, args.a_skill)?;
    let b = entrant_at(&args.b, args.b_skill)?;
    // R4607000: with `--save` each finished deal is appended to the file as soon as it is done,
    // and a rerun plays only the deals still missing. `--out` names no records file here: the
    // save file is the record store, so resuming never files a record twice.
    if let Some(save) = args.save.as_deref() {
        let expected = SaveHeader {
            v: FORK_SAVE_VERSION,
            set: set.base().to_string(),
            games: deals,
            extend: args.extend,
            budget: AI_GATE_BUDGET,
            fork_rollouts: FORK_ROLLOUTS,
            fork_max_per_game: FORK_MAX_PER_GAME,
            catalog: jackioh_cards::catalog_version().to_string(),
            a_label: a.label.clone(),
            b_label: b.label.clone(),
            a_info: info_of(&a),
            b_info: info_of(&b),
        };
        let report = resume_save(save, &expected, &base, &b, deals, args.extend, &|game| {
            Probe::for_game(&args.a, game.self_budget, game.n)
        })?;
        let mut stdout = std::io::stdout().lock();
        for line in &report.duels {
            writeln!(stdout, "{line}")?;
        }
        writeln!(stdout, "{}", report.tally)?;
        writeln!(stdout, "{} deals in {}", report.duels.len(), save.display())?;
        return Ok(());
    }
    let mut trunk: Vec<ArenaOutcome> = Vec::new();
    let mut forks: Vec<(usize, ForkPoint)> = Vec::new();
    let mut faults: Vec<String> = Vec::new();
    let mut edges: Vec<ForkEdge> = Vec::new();
    let mut rollouts = 0usize;
    let mut games_done = 0i32;
    loop {
        let batch = if games_done == 0 {
            deals
        } else {
            (SCREEN_MAX_GAMES - games_done).min(SCREEN_EXTEND_DEALS)
        };
        if batch <= 0 {
            break;
        }
        let games = trunk_setups(&b, &base, games_done + 1, batch)?;
        let played = play_trunk(&games, &args.a)?;
        let batch_from = forks.len();
        for (outcome, points, probe_faults) in played {
            let index = trunk.len();
            trunk.push(outcome);
            faults.extend(probe_faults);
            for point in points {
                forks.push((index, point));
            }
        }
        let batch_jobs: Vec<(&ArenaOutcome, &ForkPoint)> = forks[batch_from..]
            .iter()
            .map(|(index, point)| (&trunk[*index], point))
            .collect();
        let (batch_edges, batch_faults, batch_rollouts) = play_forks(&batch_jobs)?;
        edges.extend(batch_edges);
        faults.extend(batch_faults);
        rollouts += batch_rollouts;
        games_done += batch;
        if !args.extend {
            break;
        }
        if screen_done(edges.iter().filter(|edge| edge.edge != 0.0).count(), games_done) {
            break;
        }
    }
    let tally = fork_tally(&a.label, &b.label, trunk.len(), &edges, rollouts, faults);
    let records = records_of(&trunk)?;

    let path = records_path(args.out.as_deref());
    let mut lines = Vec::new();
    for outcome in &trunk {
        lines.push(duel_line(outcome, replays(outcome)?)?);
    }
    let totals = serde_json::to_string(&tally)?;

    match &path {
        Some(path) => {
            append_records(path, &records)?;
            let mut stdout = std::io::stdout().lock();
            for line in &lines {
                writeln!(stdout, "{line}")?;
            }
            writeln!(stdout, "{totals}")?;
            writeln!(stdout, "{} records appended to {}", records.len(), path.display())?;
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            for record in &records {
                writeln!(stdout, "{}", jackioh_cards::versions::game_record_line(record))?;
            }
            for line in &lines {
                eprintln!("{line}");
            }
            eprintln!("{totals}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{TEST_BUDGET, entrant};
    use jackioh_ai::AI_GATE_BUDGET;

    /// The whole tally of `jobs` as one JSON line, for the thread-count test.
    fn tally_of(jobs: &[(&ArenaOutcome, &ForkPoint)], games: usize) -> String {
        let (edges, faults, rollouts) = play_forks(jobs).unwrap();
        serde_json::to_string(&fork_tally("flip", "self", games, &edges, rollouts, faults)).unwrap()
    }

    /// A trunk game with the control on both seats at the tests' budget.
    fn trunk_game(seed: &str, n: i32) -> ArenaGame {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        let mut game = game_setup(seed.to_string(), n, false, &control, &control).unwrap();
        game.self_budget = TEST_BUDGET;
        game
    }

    /// The candidate's answer with every bit flipped: `EndTurn` for anything else, a mulligan for
    /// it, so it differs from whatever the control played.
    fn flip(_deal: i32, _at_move: usize, control: &ActionBody) -> Option<ActionBody> {
        Some(match control {
            ActionBody::EndTurn => ActionBody::Mulligan { keep: Vec::new() },
            _ => ActionBody::EndTurn,
        })
    }

    /// The candidate echoing the control's final played body.
    fn echo(_deal: i32, _at_move: usize, control: &ActionBody) -> Option<ActionBody> {
        Some(control.clone())
    }

    #[test]
    fn r4397000_self_against_self_finds_no_fork_and_reads_inconclusive() {
        let games = vec![trunk_game("r4397000-self:1", 1)];
        let played = play_trunk(&games, &AgentSpec::SelfAi).unwrap();
        assert_eq!(played.len(), 1);
        for (outcome, points, probe_faults) in &played {
            assert!(outcome.result.is_some(), "deal {}", outcome.game.n);
            assert!(points.is_empty(), "deal {}: {points:?}", outcome.game.n);
            assert!(probe_faults.is_empty(), "{probe_faults:?}");
        }
        let (edges, faults, rollouts) = play_forks(&[]).unwrap();
        assert!(edges.is_empty() && faults.is_empty() && rollouts == 0);
        let tally = fork_tally("self", "self", played.len(), &edges, rollouts, faults);
        assert_eq!(tally.forks, 0);
        assert_eq!(tally.live_forks, 0);
        assert_eq!(tally.inconclusive, Some(true));
        let json = serde_json::to_string(&tally).unwrap();
        assert!(json.contains("\"inconclusive\":true"), "{json}");
        assert!(json.contains("\"forks\":0"), "{json}");
    }

    #[test]
    fn r4397000_an_always_differing_candidate_forks_every_game_at_the_cap() {
        let games = vec![trunk_game("r4397000-cap:1", 1), trunk_game("r4397000-cap:2", 2)];
        let played: Vec<(ArenaOutcome, Vec<ForkPoint>, Vec<String>)> = games
            .par_iter()
            .map(|game| {
                let probe = Probe::scripted(game.n, flip);
                play_game_probed(game, Some(probe)).unwrap()
            })
            .collect();
        let mut forks: Vec<(usize, ForkPoint)> = Vec::new();
        for (index, (outcome, points, probe_faults)) in played.iter().enumerate() {
            assert!(probe_faults.is_empty(), "{probe_faults:?}");
            assert_eq!(points.len(), FORK_MAX_PER_GAME, "deal {}", outcome.game.n);
            for (fork_no, point) in points.iter().enumerate() {
                assert_eq!((point.deal, point.fork_no), (outcome.game.n, fork_no + 1));
                assert_eq!(point.at_move, fork_no, "deal {}", point.deal);
            }
            for point in points {
                forks.push((index, point.clone()));
            }
        }
        assert_eq!(forks.len(), games.len() * FORK_MAX_PER_GAME);

        // One pair played twice: both arms start from the trunk's prefix and play on the same
        // streams, so both runs hash the same.
        let (index, fork) = &forks[0];
        let outcome = &played[*index].0;
        let (cand, ctl) = play_pair(outcome, fork, 1).unwrap();
        let (cand_again, ctl_again) = play_pair(outcome, fork, 1).unwrap();
        assert_eq!(cand.hash, cand_again.hash);
        assert_eq!(ctl.hash, ctl_again.hash);
        let prefix = &outcome.log[..fork.at_move];
        for arm in [&cand, &ctl] {
            assert_eq!(arm.log[..fork.at_move], prefix[..]);
            assert_eq!(arm.log[fork.at_move].nonce, format!("m{}", fork.at_move));
        }
        // The control's arm replays the accepted trunk action as-is. The candidate's arm plays its
        // fork action through the same `accept` as the trunk, so a refused flip reads back as the
        // trunk's replacement; either way the fork itself is real.
        assert_eq!(ctl.log[fork.at_move].body, fork.control.clone().unwrap());
        assert_ne!(fork_key(&fork.candidate), fork_key(&fork.control));
    }

    /// R4397000's tally over one fork, byte-identical on one thread and on four: one fork's
    /// 16 arms twice, several minutes, so the daily run holds it (#4462) while
    /// `r4397000_an_always_differing_candidate_forks_every_game_at_the_cap` keeps the trunk
    /// cap and the pair determinism on every pull request.
    #[test]
    #[ignore = "daily: one fork's 16 arms on one thread and on four, several minutes; r4397000_an_always_differing_candidate_forks_every_game_at_the_cap keeps the trunk cap and the pair determinism per pull request"]
    fn r4397000_fork_tally_is_byte_identical_on_one_thread_and_on_four() {
        let games = vec![trunk_game("r4397000-tally:1", 1)];
        let played: Vec<(ArenaOutcome, Vec<ForkPoint>, Vec<String>)> = games
            .par_iter()
            .map(|game| {
                let probe = Probe::scripted(game.n, flip);
                play_game_probed(game, Some(probe)).unwrap()
            })
            .collect();
        let mut forks: Vec<(usize, ForkPoint)> = Vec::new();
        for (index, (outcome, points, probe_faults)) in played.iter().enumerate() {
            assert!(probe_faults.is_empty(), "{probe_faults:?}");
            assert_eq!(points.len(), FORK_MAX_PER_GAME, "deal {}", outcome.game.n);
            for point in points {
                forks.push((index, point.clone()));
            }
        }
        assert_eq!(forks.len(), games.len() * FORK_MAX_PER_GAME);

        // The whole tally is byte-identical on one thread and on four. One fork is enough to show
        // the order does not follow completion (its 16 arms play on rayon's pool); the last fork
        // of a game has the fewest moves left to play. All four forks played 64 arms twice and
        // took 8 minutes (#4441).
        let (index, last) = &forks[FORK_MAX_PER_GAME - 1];
        let jobs: Vec<(&ArenaOutcome, &ForkPoint)> = vec![(&played[*index].0, last)];
        let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let four = rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        let expected = one.install(|| tally_of(&jobs, games.len()));
        assert!(!expected.is_empty());
        let played_rollouts = format!("\"rollouts\":{}", jobs.len() * FORK_ROLLOUTS * 2);
        assert!(expected.contains(&played_rollouts), "{expected}");
        assert_eq!(four.install(|| tally_of(&jobs, games.len())), expected);
    }

    #[test]
    fn r4397000_a_candidate_differing_once_finds_exactly_that_fork() {
        fn once(deal: i32, at_move: usize, control: &ActionBody) -> Option<ActionBody> {
            if (deal, at_move) == (1, 0) {
                flip(deal, at_move, control)
            } else {
                echo(deal, at_move, control)
            }
        }
        let games = vec![trunk_game("r4397000-once:1", 1), trunk_game("r4397000-once:2", 2)];
        let mut found = Vec::new();
        for game in &games {
            let probe = Probe::scripted(game.n, once);
            let (outcome, points, probe_faults) = play_game_probed(game, Some(probe)).unwrap();
            assert!(probe_faults.is_empty(), "{probe_faults:?}");
            for point in points {
                found.push((outcome.log[0].player_id, point));
            }
        }
        assert_eq!(found.len(), 1, "{found:?}");
        let (first_seat, fork) = &found[0];
        assert_eq!((fork.deal, fork.at_move), (1, 0));
        assert_eq!(fork.seat, *first_seat);
        assert_eq!(fork.fork_no, 1);
        assert_ne!(fork_key(&fork.candidate), fork_key(&fork.control));
    }

    #[test]
    fn r4397000_a_rollout_fallback_is_counted_and_named() {
        fn once(deal: i32, at_move: usize, control: &ActionBody) -> Option<ActionBody> {
            if (deal, at_move) == (1, 0) {
                flip(deal, at_move, control)
            } else {
                echo(deal, at_move, control)
            }
        }
        let game = trunk_game("r4397000-starved:1", 1);
        let probe = Probe::scripted(game.n, once);
        let (mut outcome, points, _) = play_game_probed(&game, Some(probe)).unwrap();
        assert_eq!(points.len(), 1);
        let fork = points.into_iter().next().unwrap();
        // The rollout decides starved, as `r1489000_a_self_seat_starved_of_budget_counts_its_fallbacks_in_the_outcome`
        // starves the trunk: every decision falls back, so both arms fault and neither wins.
        outcome.game.self_budget = SearchBudget {
            nodes: 0,
            lethal_nodes: 0,
            ..AI_GATE_BUDGET
        };
        let (cand, ctl) = play_pair(&outcome, &fork, 1).unwrap();
        for (arm, done) in [("cand", &cand), ("ctl", &ctl)] {
            let fault = done
                .fault
                .as_ref()
                .unwrap_or_else(|| panic!("{arm} has no fault"));
            assert!(
                fault.starts_with(&format!("deal 1 fork 1 pair 1 {arm}: ")),
                "{fault}"
            );
            assert!(!done.won, "{arm} won despite its fault");
        }
    }

    #[test]
    fn r4397000_fork_edge_averages_a_deals_forks_first() {
        // Deal 1's forks read +1 twice, deal 2's reads +0.5, deal 3's reads 0: the plain mean of
        // the four forks would be +0.625, but the tally means each deal first.
        let edges = vec![
            ForkEdge { deal: 1, edge: 1.0 },
            ForkEdge { deal: 1, edge: 1.0 },
            ForkEdge { deal: 2, edge: 0.5 },
            ForkEdge { deal: 3, edge: 0.0 },
        ];
        let tally = fork_tally("flip", "self", 2, &edges, 64, Vec::new());
        assert_eq!((tally.forks, tally.live_forks), (4, 3));
        assert_eq!(tally.forks_per_game, 2.0);
        assert_eq!(tally.rollouts, 64);
        assert_eq!(tally.fork_edge, 0.5);
        assert_eq!(tally.inconclusive, Some(true));
        // Deal means [1.0, 0.5, 0.0]: the mean 0.5 with the deals' spread, not the forks'.
        let (edge, low, high) = paired_interval(&[1.0, 0.5, 0.0]);
        assert_eq!(
            (tally.fork_edge_low, tally.fork_edge_high),
            (round_share(low), round_share(high))
        );
        assert_eq!(edge, 0.5);
    }

    #[test]
    fn r4397000_extend_plays_on_until_live_forks() {
        assert!(screen_done(SCREEN_MIN_DECIDED, FORK_TRUNK_DEALS));
        assert!(!screen_done(SCREEN_MIN_DECIDED - 1, FORK_TRUNK_DEALS));
        assert!(screen_done(0, SCREEN_MAX_GAMES));
        assert!(!screen_done(0, SCREEN_MAX_GAMES - 2));
    }

    /// R4607000: a path of its own in the temp dir, so the tests' saves never touch the
    /// worktree (the builder deletes the save before finishing; the tests delete theirs).
    fn save_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("jackioh-fork-save-{}-{name}.jsonl", std::process::id()))
    }

    /// R4607000: the candidate differing only at `at_move == 1`, echoing the control elsewhere:
    /// one fork a deal, late enough that its arms are cheap.
    fn late_fork(deal: i32, at_move: usize, control: &ActionBody) -> Option<ActionBody> {
        if (deal, at_move) == (deal, 1) {
            flip(deal, at_move, control)
        } else {
            Some(control.clone())
        }
    }

    /// R4607000: the candidate differing only at another move, for the identity check's refusal.
    fn other_fork(deal: i32, at_move: usize, control: &ActionBody) -> Option<ActionBody> {
        if at_move == 0 {
            flip(deal, at_move, control)
        } else {
            Some(control.clone())
        }
    }

    /// R4607000: the tests' header: two dev-set deals at the tests' budget, self against self.
    fn test_header(games: i32) -> SaveHeader {
        let own = crate::agent::own_info();
        SaveHeader {
            v: FORK_SAVE_VERSION,
            set: "r4607000-dev".to_string(),
            games,
            extend: false,
            budget: TEST_BUDGET,
            fork_rollouts: FORK_ROLLOUTS,
            fork_max_per_game: FORK_MAX_PER_GAME,
            catalog: jackioh_cards::catalog_version().to_string(),
            a_label: "self".to_string(),
            b_label: "self".to_string(),
            a_info: Some(own.clone()),
            b_info: Some(own),
        }
    }

    /// R4607000: one header line then one line per deal, as the save file holds them.
    fn write_save(path: &std::path::Path, header: &SaveHeader, lines: &[DealLine]) {
        let mut text = serde_json::to_string(header).unwrap();
        text.push('\n');
        for line in lines {
            text.push_str(&serde_json::to_string(line).unwrap());
            text.push('\n');
        }
        std::fs::write(path, text).unwrap();
    }

    /// R4607000: the save file's deal lines, parsed.
    fn read_lines(path: &std::path::Path) -> Vec<DealLine> {
        let text = std::fs::read_to_string(path).unwrap();
        text.lines()
            .skip(1)
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// R4607000: the tests' probe factory: scripted, answering `answered`, counting its calls.
    fn scripted_factory(
        calls: &std::sync::atomic::AtomicUsize,
        answered: ScriptedAnswer,
    ) -> impl Fn(&ArenaGame) -> anyhow::Result<Probe> + Sync + '_ {
        move |game: &ArenaGame| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Probe::scripted(game.n, answered))
        }
    }

    /// R4607000: a deal line as the assertions compare it: everything but the duel line, whose
    /// decision times a rerun never reproduces.
    fn line_without_duel(line: &DealLine) -> serde_json::Value {
        let mut value = serde_json::to_value(line).unwrap();
        value.as_object_mut().unwrap().remove("duel");
        value
    }

    #[test]
    fn r4607000_resume_after_cut_prints_the_same_tally() {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        let header = test_header(2);
        let base = "r4607000-cut";
        let full = save_path("cut-full");
        let cut = save_path("cut-cut");
        let _ = std::fs::remove_file(&full);
        let _ = std::fs::remove_file(&cut);

        // One uninterrupted run over both deals.
        let fresh_calls = std::sync::atomic::AtomicUsize::new(0);
        let report = resume_save(
            &full,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&fresh_calls, late_fork),
        )
        .unwrap();
        assert_eq!(report.games_played, 2);
        assert_eq!(fresh_calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        let full_lines = read_lines(&full);
        assert_eq!(full_lines.len(), 2);
        for line in &full_lines {
            assert_eq!(line.forks.len(), 1, "deal {}", line.deal);
        }

        // The same file cut back to its first deal's line, as a killed run leaves it.
        let text = std::fs::read_to_string(&full).unwrap();
        let mut kept = text.lines().take(2).collect::<Vec<_>>().join("\n");
        kept.push('\n');
        std::fs::write(&cut, &kept).unwrap();
        let resume_calls = std::sync::atomic::AtomicUsize::new(0);
        let resumed = resume_save(
            &cut,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&resume_calls, late_fork),
        )
        .unwrap();
        // The identity game plus the one missing deal, and nothing else.
        assert_eq!(resumed.games_played, 1);
        assert_eq!(resume_calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(resumed.tally, report.tally);
        // The resumed file ends with the same deal lines: the same deals with the same
        // hashes, forks, arms and records. Only the duel lines differ, in their decision
        // times, which no rerun reproduces.
        let cut_lines = read_lines(&cut);
        assert_eq!(cut_lines.len(), 2);
        for (cut_line, full_line) in cut_lines.iter().zip(full_lines.iter()) {
            assert_eq!(cut_line.deal, full_line.deal);
            assert_eq!(line_without_duel(cut_line), line_without_duel(full_line));
        }

        let _ = std::fs::remove_file(&full);
        let _ = std::fs::remove_file(&cut);
    }

    #[test]
    fn r4607000_a_torn_last_line_is_played_again() {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        let header = test_header(2);
        let base = "r4607000-torn";
        let full = save_path("torn-full");
        let torn = save_path("torn-torn");
        let _ = std::fs::remove_file(&full);
        let _ = std::fs::remove_file(&torn);

        let fresh_calls = std::sync::atomic::AtomicUsize::new(0);
        let report = resume_save(
            &full,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&fresh_calls, late_fork),
        )
        .unwrap();
        let text = std::fs::read_to_string(&full).unwrap();
        let kept: Vec<&str> = text.lines().collect();
        assert_eq!(kept.len(), 3);
        // The first deal's line whole, then a torn fragment of the second's.
        let mut torn_text = kept[..2].join("\n");
        torn_text.push('\n');
        torn_text.push_str(&kept[2][..kept[2].len().min(20)]);
        std::fs::write(&torn, &torn_text).unwrap();

        let resume_calls = std::sync::atomic::AtomicUsize::new(0);
        let resumed = resume_save(
            &torn,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&resume_calls, late_fork),
        )
        .unwrap();
        assert_eq!(resumed.games_played, 1);
        assert_eq!(resumed.tally, report.tally);
        let torn_lines = read_lines(&torn);
        assert_eq!(torn_lines.len(), 2);
        for (torn_line, full_line) in torn_lines.iter().zip(read_lines(&full).iter()) {
            assert_eq!(torn_line.deal, full_line.deal);
            assert_eq!(line_without_duel(torn_line), line_without_duel(full_line));
        }

        let _ = std::fs::remove_file(&full);
        let _ = std::fs::remove_file(&torn);
    }

    #[test]
    fn r4607000_resume_refuses_a_changed_header_before_any_game() {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        let header = test_header(1);
        let base = "r4607000-refuse";
        let path = save_path("refuse");
        let _ = std::fs::remove_file(&path);

        let fresh_calls = std::sync::atomic::AtomicUsize::new(0);
        resume_save(
            &path,
            &header,
            base,
            &control,
            1,
            false,
            &scripted_factory(&fresh_calls, late_fork),
        )
        .unwrap();
        let file = std::fs::read(&path).unwrap();
        assert_eq!(read_lines(&path).len(), 1);

        // Each mutation refuses the file by naming its field, plays nothing, and leaves the
        // file byte-identical.
        let own = crate::agent::own_info();
        let mut other_info = own.clone();
        other_info.generation += 1;
        let mut mutated: Vec<(SaveHeader, &str)> = Vec::new();
        let mut header = header.clone();
        header.v += 1;
        mutated.push((header.clone(), "version"));
        header = test_header(1);
        header.set = "other-set".to_string();
        mutated.push((header.clone(), "set"));
        header = test_header(1);
        header.games = 99;
        mutated.push((header.clone(), "--games"));
        header = test_header(1);
        header.extend = true;
        mutated.push((header.clone(), "--extend"));
        header = test_header(1);
        header.budget = SearchBudget {
            nodes: TEST_BUDGET.nodes + 1,
            ..TEST_BUDGET
        };
        mutated.push((header.clone(), "budget"));
        header = test_header(1);
        header.fork_rollouts = FORK_ROLLOUTS + 1;
        mutated.push((header.clone(), "FORK_ROLLOUTS"));
        header = test_header(1);
        header.fork_max_per_game = FORK_MAX_PER_GAME + 1;
        mutated.push((header.clone(), "FORK_MAX_PER_GAME"));
        header = test_header(1);
        header.catalog = "0.0.0-test".to_string();
        mutated.push((header.clone(), "catalog"));
        header = test_header(1);
        header.a_label = "other".to_string();
        mutated.push((header.clone(), "candidate"));
        header = test_header(1);
        header.b_label = "other".to_string();
        mutated.push((header.clone(), "control"));
        header = test_header(1);
        header.a_info = Some(other_info.clone());
        mutated.push((header.clone(), "candidate"));
        header = test_header(1);
        header.b_info = Some(other_info.clone());
        mutated.push((header.clone(), "control"));
        for (saved, field) in mutated {
            write_save(&path, &saved, &read_lines(&path));
            // The refusal must leave the file as this mutation wrote it.
            let before = std::fs::read(&path).unwrap();
            let calls = std::sync::atomic::AtomicUsize::new(0);
            let refused = resume_save(
                &path,
                &test_header(1),
                base,
                &control,
                1,
                false,
                &scripted_factory(&calls, late_fork),
            )
            .unwrap_err();
            assert!(refused.to_string().contains(field), "{field}: {refused}");
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
            assert_eq!(std::fs::read(&path).unwrap(), before);
            // Restore the good header for the next mutation.
            write_save(&path, &test_header(1), &read_lines(&path));
        }
        // The file ends as the uninterrupted run wrote it.
        assert_eq!(std::fs::read(&path).unwrap(), file);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn r4607000_a_different_candidate_fails_the_identity_game() {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        // A screen of two deals with only the first saved: not finished, so the resume runs
        // the identity game on the saved deal.
        let header = test_header(2);
        let base = "r4607000-identity";
        let path = save_path("identity");
        let _ = std::fs::remove_file(&path);

        let game = trunk_game(&format!("{base}:1"), 1);
        let line = play_deal(&game, Probe::scripted(1, late_fork)).unwrap();
        assert_eq!(line.forks.len(), 1, "deal {}", line.deal);
        write_save(&path, &header, &[line]);

        // The same header but a candidate that differs at another move: exactly one trunk
        // game is played, then the file is refused for its forks.
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let refused = resume_save(
            &path,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&calls, other_fork),
        )
        .unwrap_err();
        assert!(refused.to_string().contains("forks"), "{refused}");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn r4607000_a_rebuilt_trunk_fails_the_identity_game() {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        // A screen of two deals with only the first saved: not finished, so the resume runs
        // the identity game on the saved deal.
        let header = test_header(2);
        let base = "r4607000-rebuilt";
        let path = save_path("rebuilt");
        let _ = std::fs::remove_file(&path);

        let game = trunk_game(&format!("{base}:1"), 1);
        let line = play_deal(&game, Probe::scripted(1, late_fork)).unwrap();
        assert_eq!(line.forks.len(), 1, "deal {}", line.deal);
        write_save(&path, &header, &[line]);

        // The same header on another seed base replays the trunk to another hash: exactly one
        // trunk game is played, then the file is refused for its trunk hash.
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let refused = resume_save(
            &path,
            &header,
            "r4607000-rebuilt-elsewhere",
            &control,
            2,
            false,
            &scripted_factory(&calls, late_fork),
        )
        .unwrap_err();
        assert!(refused.to_string().contains("trunk hash"), "{refused}");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn r4607000_a_finished_save_plays_nothing_and_reprints_its_tally() {
        jackioh_cards::register_all();
        let control = entrant(&AgentSpec::SelfAi).unwrap();
        let header = test_header(2);
        let base = "r4607000-finished";
        let path = save_path("finished");
        let _ = std::fs::remove_file(&path);

        let fresh_calls = std::sync::atomic::AtomicUsize::new(0);
        let report = resume_save(
            &path,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&fresh_calls, late_fork),
        )
        .unwrap();
        let file = std::fs::read(&path).unwrap();

        // Run again on the finished save: no game is played, not even the identity game, and
        // the tally is the same string.
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let again = resume_save(
            &path,
            &header,
            base,
            &control,
            2,
            false,
            &scripted_factory(&calls, late_fork),
        )
        .unwrap();
        assert_eq!(again.games_played, 0);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(again.tally, report.tally);
        assert_eq!(again.duels, report.duels);
        assert_eq!(std::fs::read(&path).unwrap(), file);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn r4607000_extend_counts_saved_and_new_live_forks_together() {
        assert_eq!(next_batch(0, 0, false, 200), Some((1, 200)));
        // Saved 200 games with 19 live forks keep going at deal 201; the 20th stops.
        assert_eq!(next_batch(200, 19, true, 200), Some((201, 100)));
        assert_eq!(next_batch(200, 20, true, 200), None);
        // Without `--extend` the first batch is the whole screen.
        assert_eq!(next_batch(200, 0, false, 200), None);
        // The cap is respected: 10 deals left, then nothing.
        assert_eq!(next_batch(1990, 0, true, 200), Some((1991, 10)));
        assert_eq!(next_batch(2000, 0, true, 200), None);
    }

    #[test]
    fn r4397000_forks_needs_a_seed_set_full_skill_and_no_paired() {
        let args = |set: Option<crate::arena::SeedSet>, paired: bool, skill: f64| crate::arena::Args {
            a: AgentSpec::SelfAi,
            b: AgentSpec::Random,
            games: None,
            seed: None,
            set,
            paired,
            extend: false,
            forks: true,
            save: None,
            out: None,
            a_skill: skill,
            b_skill: AI_SKILL_MAX,
        };
        let refused = run(args(None, false, AI_SKILL_MAX)).unwrap_err();
        assert!(refused.to_string().contains("--set improve-dev"), "{refused}");
        let set = Some(crate::arena::SeedSet::ImproveDev);
        let refused = run(args(set, true, AI_SKILL_MAX)).unwrap_err();
        assert!(refused.to_string().contains("--paired"), "{refused}");
        let refused = run(args(set, false, 0.5)).unwrap_err();
        assert!(refused.to_string().contains("--a-skill"), "{refused}");
    }
}
