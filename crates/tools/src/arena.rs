//! `cargo jackioh arena`: the training arena (SURFACE §14.2; docs/v0.3.0/README.md §8). A referee
//! holds the true state and plays seeded games between two agents, each seeing only `redact(state,
//! seat)` (R185): `self` (this build's AI), `random` (SPEC §10.7's random policy, the gates' baseline)
//! or `bin:<path>` (another build's AI, spawned as `<path> agent`, SURFACE §14.1).
//!
//! At skill `s` (R747000) an AI decision follows the agent with chance `s` and is the skill miss
//! otherwise (R2214000), rolled on the seat's own stream. A paired series (`--paired`, R850000) plays
//! each deal twice with the seats swapped; `--set improve-dev` is the improve lane's dev set (R850001).
//! No base may name a training gate's own seeds (`<lane>` or `<lane>:…`). Each deck is `build_ai_deck`
//! minus its agent's shadow ban (R186), Easy both (R180), unshaped (R976000). Fallbacks and caught
//! search panics are counted per seat (R1489000). So a game is a function of its seed and its two
//! agents, whatever the thread count.
//!
//! Each finished game leaves one `GameRecord` (R376, R378) on a JSONL line in `<--out>/<date>.jsonl`,
//! else `$JACKIOH_TRAINING_OUT/<date>.jsonl`, else stdout; a tally closes the duel lines with agent
//! `a`'s share of the games won and its 95% Wilson interval (a paired series adds its edge).
//! `--forks` hands the trunk to `crate::fork_screen` (R4397000): the control on both seats, the
//! candidate probed at each decision, each fork played out both ways from where it was made.

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Once, mpsc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow, bail};
use jackioh_ai::{
    AI_GATE_BUDGET, AI_SKILL_MAX, AI_SKILL_MIN, AiDeckOptions, Decision, DecisionReason, RejectedAction,
    SearchBudget, SimPanic, build_ai_deck, plays_best, random_action, redact, skill_miss_action,
};
use jackioh_engine::{
    AI_DIFFICULTY, Action, ActionBody, CreateGameOptions, DEV_RECORD_ID_PREFIX, FoldArgs, GameRecord,
    GameResult, GameState, PerPlayer, PlayerId, RecordBuild, Rng, STATS_INTERVAL_Z, Winner, begin_game,
    create_game, fold, hash_state, legal_actions, reduce, seat_to_act, subsystems, summarize_game,
};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};

use crate::agent::{self, AgentInfo};
use crate::gate::MS_PER_SECOND;
use crate::patches::utc_date_of;

/// The referee's ceiling on actions in one game; the engine's turn cap (R2) ends every real game first.
const ARENA_MAX_ACTIONS: usize = 3000;

/// R1006007: the words each of the engine's settle caps panics with (§10.3, R283; `triggers.rs`,
/// `state_check.rs`, `effects/combat.rs`), when a board runs to its cap instead of settling.
pub(crate) const UNSETTLED_PANIC: &str = "did not settle";

thread_local! {
    /// R1006007: on a thread `watch_unsettled` started, what tells its watcher that the work raised an
    /// `UNSETTLED_PANIC` panic.
    static UNSETTLED: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// R1006007: put a panic hook in front of the one in place (which still prints every panic) that tells
/// a watched thread's watcher of each `UNSETTLED_PANIC` panic the thread raises, caught or not. The AI's
/// search catches a panicking line and goes on, so without the hook nothing outside it sees one.
fn tell_unsettled_panics() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info
                .payload_as_str()
                .is_some_and(|message| message.contains(UNSETTLED_PANIC))
            {
                UNSETTLED.with(|tell| {
                    if let Some(tell) = &*tell.borrow() {
                        tell();
                    }
                });
            }
            previous(info);
        }));
    });
}

/// R1006007: `work` on a thread of its own: its value, or `None` as soon as it raises an
/// `UNSETTLED_PANIC` panic, caught or not. A loop the cap stops can take minutes to reach it, and the
/// search may reach it again on every determinization of the decision, so the watcher stops waiting at
/// the first. The thread runs on until its work returns or the process ends.
pub(crate) fn watch_unsettled<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tell_unsettled_panics();
    let (sender, receiver) = mpsc::channel::<Option<T>>();
    let tell = sender.clone();
    std::thread::spawn(move || {
        UNSETTLED.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                let _ = tell.send(None);
            }))
        });
        let _ = sender.send(Some(work()));
    });
    receiver.recv().ok().flatten()
}

/// The directory game records go to when `--out` names none (docs/v0.3.0/README.md §8).
pub(crate) const TRAINING_OUT_ENV: &str = "JACKIOH_TRAINING_OUT";

const DUEL_PERCENTILE: f64 = 0.95;

/// R850001: the fewest games a screen on a seed set plays, and what it plays when `--games` names
/// none: 200 deals, each played twice.
pub(crate) const SCREEN_GAMES: i32 = 400;

/// R4319000: a screen with fewer decided deals than this says nothing (`inconclusive: true`): at 20
/// decided deals the keep rule keeps a change that wins 15 of them, three in four.
pub(crate) const SCREEN_MIN_DECIDED: usize = 20;

/// R4319000: the most games a screen with `--extend` plays: the promotion gate's own maximum.
pub(crate) const SCREEN_MAX_GAMES: i32 = 2000;

/// R4319000: the deals each `--extend` batch after the first plays (200 games).
pub(crate) const SCREEN_EXTEND_DEALS: i32 = 100;

#[derive(clap::Args)]
pub struct Args {
    /// Agent a: self | random | bin:<path>. It sits p1 on odd games and p2 on even ones.
    #[arg(long, value_parser = parse_agent)]
    pub a: AgentSpec,
    /// Agent b: self | random | bin:<path>.
    #[arg(long, value_parser = parse_agent)]
    pub b: AgentSpec,
    /// Games to play: 1..=N. A seed set plays `SCREEN_GAMES` (400) unless this says more.
    #[arg(long)]
    pub games: Option<i32>,
    /// The seed base: game n's seed is `<base>:<n>`, or deal d's `<base>:<d>` when paired. Never a
    /// training gate's own (`improve`, `unban`, or either followed by `:`).
    #[arg(long, conflicts_with = "set", required_unless_present = "set")]
    pub seed: Option<String>,
    /// R850001: a fixed seed set to screen on, played paired.
    #[arg(long, value_enum)]
    pub set: Option<SeedSet>,
    /// R850000: play each deal twice with the seats swapped (an even number of games).
    #[arg(long)]
    pub paired: bool,
    /// R4319000: on a screen (`--set`), keep playing in batches of `SCREEN_EXTEND_DEALS` deals
    /// until `SCREEN_MIN_DECIDED` deals are decided or `SCREEN_MAX_GAMES` games are played.
    #[arg(long)]
    pub extend: bool,
    /// R4397000: the fork screen: each deal of the set once with the control on both seats, the
    /// candidate asked each decision until the game's fork cap, each fork played out both ways.
    /// Needs `--set improve-dev`; the candidate is probed at full skill, so `--a-skill` stays 1.
    #[arg(long)]
    pub forks: bool,
    /// The directory the game records go to (`<dir>/<date>.jsonl`); default `$JACKIOH_TRAINING_OUT`,
    /// else stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// R747000: agent a's skill in [0, 1], the chance each decision follows its AI, the rest the skill
    /// miss (R2214000). 1 plays the AI as it is. Not for `random`.
    #[arg(long, default_value_t = AI_SKILL_MAX)]
    pub a_skill: f64,
    /// R747000: agent b's skill, as `--a-skill`.
    #[arg(long, default_value_t = AI_SKILL_MAX)]
    pub b_skill: f64,
}

/// R850001: the fixed seed sets a screen plays, each paired.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedSet {
    /// The improve lane's dev set: deal `d` is `improve-dev:<d>`.
    ImproveDev,
}

impl SeedSet {
    /// The set's seed base.
    pub(crate) fn base(self) -> &'static str {
        match self {
            SeedSet::ImproveDev => "improve-dev",
        }
    }
}

/// The series `args` name: its seed base, whether it is paired and its games. A seed set is always
/// paired and plays at least `SCREEN_GAMES`; a base that names a training gate's seeds is refused
/// (R850001), as is an odd paired count.
fn series(args: &Args) -> anyhow::Result<(String, bool, i32)> {
    if args.extend && args.set.is_none() {
        bail!("arena: --extend screens a seed set, so it needs --set improve-dev");
    }
    let (base, paired, games) = match (args.set, &args.seed) {
        (Some(set), _) => {
            let games = args.games.unwrap_or(SCREEN_GAMES);
            if games < SCREEN_GAMES {
                bail!(
                    "arena: a screen on a seed set plays at least {SCREEN_GAMES} games, not {games} \
                     (R850001); a quick look plays a base of your own (--seed, --paired)"
                );
            }
            (set.base().to_string(), true, games)
        }
        (None, Some(seed)) => {
            if crate::promote::is_gate_seed_base(seed) {
                bail!(
                    "arena: the seed base \"{seed}\" names a training gate's hold-out seeds, which no \
                     screen plays (R850001): screen on `--set improve-dev`, or on a base of your own"
                );
            }
            let Some(games) = args.games else {
                bail!("arena: a seed base needs --games");
            };
            (seed.clone(), args.paired, games)
        }
        (None, None) => bail!("arena: name a seed set (--set) or a seed base (--seed)"),
    };
    if games < 1 {
        bail!("arena: --games must be at least 1, not {games}");
    }
    if paired && games % 2 != 0 {
        bail!("arena: a paired series plays each deal twice, so its games are even, not {games}");
    }
    Ok((base, paired, games))
}

/// Who plays a seat (SURFACE §14.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentSpec {
    /// This build's AI.
    SelfAi,
    /// SPEC §10.7's random policy.
    Random,
    /// Another build's AI: `<path> agent`.
    Bin(PathBuf),
}

/// clap's parser for `--a`/`--b`, so a bad agent is a usage error. A leading `~/` in a `bin:` path
/// reads as `$HOME/`, since no shell expands it inside `bin:~/…`.
fn parse_agent(text: &str) -> Result<AgentSpec, String> {
    match text {
        "self" => Ok(AgentSpec::SelfAi),
        "random" => Ok(AgentSpec::Random),
        _ => match text.strip_prefix("bin:") {
            Some(path) if !path.is_empty() => Ok(AgentSpec::Bin(expand_home(path))),
            _ => Err(format!("an agent is self, random or bin:<path>, not \"{text}\"")),
        },
    }
}

fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// An agent ready to play, with the label its records carry and the shadow ban (R186) its seats' decks omit.
#[derive(Clone, Debug)]
pub(crate) struct Entrant {
    pub spec: AgentSpec,
    pub label: String,
    /// The agent's generation (SURFACE §14.3); `None` for `random`, which is no generation of the AI.
    pub generation: Option<i64>,
    pub lane: Option<String>,
    pub shadow_ban: Vec<String>,
    /// R747000: the chance each decision follows the agent's AI; `AI_SKILL_MAX` for `random`, which
    /// has none.
    pub skill: f64,
}

/// R747000: a skill is in `AI_SKILL_MIN..=AI_SKILL_MAX`; a NaN is outside it.
fn check_skill(skill: f64) -> anyhow::Result<()> {
    if !(AI_SKILL_MIN..=AI_SKILL_MAX).contains(&skill) {
        bail!("arena: a skill is between {AI_SKILL_MIN} and {AI_SKILL_MAX}, not {skill}");
    }
    Ok(())
}

/// R747000: `entrant` at `skill`. A skill short of full is part of the label (`self-s0.5`), so the
/// records of games at different skills, which share seeds, are separate records (R378).
pub(crate) fn entrant_at(spec: &AgentSpec, skill: f64) -> anyhow::Result<Entrant> {
    check_skill(skill)?;
    let mut ready = entrant(spec)?;
    if skill < AI_SKILL_MAX {
        if *spec == AgentSpec::Random {
            bail!("arena: random plays no AI decision, so it has no skill to set");
        }
        ready.skill = skill;
        ready.label = format!("{}-s{skill}", ready.label);
    }
    Ok(ready)
}

/// Readies an agent: `self` reads this build, `bin:` asks the binary (`info`), and `random` is dealt this
/// build's shadow ban, as the gates deal the baseline's seat.
pub(crate) fn entrant(spec: &AgentSpec) -> anyhow::Result<Entrant> {
    match spec {
        AgentSpec::SelfAi => {
            let info = agent::own_info();
            Ok(Entrant {
                spec: spec.clone(),
                label: "self".to_string(),
                generation: Some(info.generation),
                lane: Some(info.lane),
                shadow_ban: info.shadow_ban,
                skill: AI_SKILL_MAX,
            })
        }
        AgentSpec::Random => Ok(Entrant {
            spec: spec.clone(),
            label: "random".to_string(),
            generation: None,
            lane: None,
            shadow_ban: agent::own_info().shadow_ban,
            skill: AI_SKILL_MAX,
        }),
        AgentSpec::Bin(path) => {
            let info = query_info(path)?;
            Ok(Entrant {
                spec: spec.clone(),
                label: format!("bin-gen{}", info.generation),
                generation: Some(info.generation),
                lane: Some(info.lane),
                shadow_ban: info.shadow_ban,
                skill: AI_SKILL_MAX,
            })
        }
    }
}

pub(crate) fn query_info(path: &Path) -> anyhow::Result<AgentInfo> {
    let mut bin = BinAgent::spawn(path)?;
    let answer = bin.call(&json!({ "op": "info" }))?;
    bin.quit();
    if let Some(error) = answer.get("error") {
        bail!("{}: info answered an error: {error}", path.display());
    }
    serde_json::from_value(answer)
        .with_context(|| format!("{}: info's answer is not an AgentInfo", path.display()))
}

#[derive(Clone, Debug)]
pub(crate) struct ArenaGame {
    /// 1-based, as `gameConfig` numbers a gate's games.
    pub n: i32,
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    pub seats: PerPlayer<AgentSpec>,
    /// R747000: each seat's skill, from its agent's.
    pub skills: PerPlayer<f64>,
    /// Where agent `a` sits.
    pub a_seat: PlayerId,
    /// The agents' labels, a's then b's, for the records.
    pub labels: (String, String),
    /// R850000: whether the game is one of a deal's two, which share its seed.
    pub paired: bool,
    /// R1006005: the most actions one turn may take before the game ends without a result; `None`
    /// (the arena's games, the gates') for no such ceiling, only `ARENA_MAX_ACTIONS`.
    pub turn_actions: Option<usize>,
    /// R1006007: whether a decision whose search ran a board to a settle cap ends the game without a
    /// result (self-play's games); the arena's and the gates' games play on.
    pub stop_unsettled: bool,
    /// R1489000: the budget a `self` seat decides at: `AI_GATE_BUDGET`, the browser's own, but in a
    /// test that starves it. A `bin:` agent decides at its own build's.
    pub self_budget: SearchBudget,
}

/// The budget a test decides at when it plays whole games to prove how a game is dealt, seated,
/// replayed or filed, not how well it is played: `AI_GATE_BUDGET`'s search at about a quarter of its
/// nodes and one determinization. At 60 nodes a seat falls back on two dozen decisions of a game,
/// which R1489000 faults; 80 was the least that filed clean, and 150 leaves room. It is also
/// `run-sim --budget tiny`'s (#3065).
pub(crate) const TEST_BUDGET: SearchBudget = SearchBudget {
    nodes: 150,
    lethal_nodes: 40,
    determinizations: 1,
    ..AI_GATE_BUDGET
};

/// What a game came to, with its seats' fallbacks and caught panics and each seat's decision times.
#[derive(Clone, Debug)]
pub(crate) struct ArenaOutcome {
    pub game: ArenaGame,
    /// `None` when the action ceiling was hit or a controller threw.
    pub result: Option<GameResult>,
    pub log: Vec<Action>,
    /// `hash_state` of the final state.
    pub hash: String,
    pub turns: i32,
    pub rejected: Vec<RejectedAction>,
    pub thrown: Vec<String>,
    /// Each seat's fallbacks: agent answers of "no move", replaced by the random policy on the seat's
    /// stream, and AI decisions off the fallback path (`DecisionReason::Fallback`, R1489000).
    pub fallbacks: PerPlayer<i32>,
    /// R1489000: each seat's AI decisions' `stats.sim_panics`, summed; a `bin:` seat's as its verbose
    /// answers tell them.
    pub sim_panics: PerPlayer<usize>,
    /// The first of them that kept its message (a `self` seat's), by seat; None when there is none.
    pub first_sim_panic: Option<SimPanic>,
    /// Each seat's decision times, in milliseconds.
    pub times: PerPlayer<Vec<f64>>,
}

impl ArenaOutcome {
    /// Whether agent `a` won. A draw, or a game without a result, is nobody's win (SURFACE §14.2).
    pub(crate) fn a_won(&self) -> bool {
        won_by(self.result, self.game.a_seat)
    }

    /// Whether the game ended drawn.
    pub(crate) fn drawn(&self) -> bool {
        matches!(self.result, Some(result) if result.winner == Winner::Draw)
    }

    /// R1489000: what either seat's AI did off its own path, p1 first: refused actions, fallbacks and
    /// panics its search caught, each line naming the seat. Empty when there is none.
    pub(crate) fn faults(&self) -> Vec<String> {
        let mut faults = Vec::new();
        for seat in [PlayerId::P1, PlayerId::P2] {
            let refused = self
                .rejected
                .iter()
                .filter(|rejected| rejected.seat == seat)
                .count();
            if refused > 0 {
                faults.push(format!("{}: {refused} refused actions", seat.as_str()));
            }
            if self.fallbacks[seat] > 0 {
                faults.push(format!("{}: {} fallbacks", seat.as_str(), self.fallbacks[seat]));
            }
            if self.sim_panics[seat] > 0 {
                let first = match &self.first_sim_panic {
                    Some(first) if first.seat == seat => format!(", the first: {}", first.message),
                    _ => String::new(),
                };
                faults.push(format!(
                    "{}: {} search panics{first}",
                    seat.as_str(),
                    self.sim_panics[seat]
                ));
            }
        }
        faults
    }
}

pub(crate) fn won_by(result: Option<GameResult>, seat: PlayerId) -> bool {
    matches!(result, Some(result) if result.winner == Winner::from(seat))
}

/// R850000: the deal game `n` of a paired series plays, 1-based: games `2d − 1` and `2d` are deal `d`.
pub(crate) fn deal_of(n: i32) -> i32 {
    (n + 1) / 2
}

/// Games 1..=games between `a` and `b`, game `n` on `seed_of(n)`, or when `paired` on
/// `seed_of(deal_of(n))`, so each deal is played twice with the seats swapped (R850000). Decks are
/// built here, before any game, so an unbuildable deck fails the run.
pub(crate) fn setups(
    a: &Entrant,
    b: &Entrant,
    games: i32,
    paired: bool,
    seed_of: impl Fn(i32) -> String,
) -> anyhow::Result<Vec<ArenaGame>> {
    let mut out = Vec::new();
    for n in 1..=games {
        let seed = seed_of(if paired { deal_of(n) } else { n });
        out.push(game_setup(seed, n, paired, a, b)?);
    }
    Ok(out)
}

/// Game `n` on `seed`: agent `a` sits p1 on odd `n`, and each seat's deck omits its own agent's shadow ban.
pub(crate) fn game_setup(
    seed: String,
    n: i32,
    paired: bool,
    a: &Entrant,
    b: &Entrant,
) -> anyhow::Result<ArenaGame> {
    let a_seat = if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 };
    let (p1, p2) = if a_seat == PlayerId::P1 { (a, b) } else { (b, a) };
    let decks = (
        deck_for(&seed, PlayerId::P1, p1)?,
        deck_for(&seed, PlayerId::P2, p2)?,
    );
    Ok(ArenaGame {
        n,
        seed,
        decks,
        seats: PerPlayer::new(p1.spec.clone(), p2.spec.clone()),
        skills: PerPlayer::new(p1.skill, p2.skill),
        a_seat,
        labels: (a.label.clone(), b.label.clone()),
        paired,
        turn_actions: None,
        stop_unsettled: false,
        self_budget: AI_GATE_BUDGET,
    })
}

/// A seat's deck: `buildAiDeck` seeded `<seed>:deck:<seat>` at Easy's resources (R180: no handicap),
/// over all three sets (R184, R380), minus the entrant's own shadow ban. R976000: both seats by this
/// one rule, neither shaped, so the gate deals as practice and the quality gates do.
fn deck_for(seed: &str, seat: PlayerId, entrant: &Entrant) -> anyhow::Result<Vec<String>> {
    let handicap = AI_DIFFICULTY.easy;
    let mut rng = Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0);
    let options = AiDeckOptions {
        banned: Some(entrant.shadow_ban.clone()),
        mana_cap: Some(handicap.mana_cap),
        shape: false,
        ..Default::default()
    };
    catch_unwind(AssertUnwindSafe(|| {
        build_ai_deck(&mut rng, handicap.deck_size, &options)
    }))
    .map_err(|panic| {
        anyhow!(
            "{seed}: {} deck: {}",
            seat.as_str(),
            agent::panic_message(&*panic)
        )
    })
}

/// Plays every game in parallel, outcomes in game order. An agent that cannot start fails the run;
/// anything wrong inside a game is that game's.
pub(crate) fn play_games(games: &[ArenaGame]) -> anyhow::Result<Vec<ArenaOutcome>> {
    games.par_iter().map(play_game).collect()
}

/// Who decides for a seat while a game plays (R4397000: the fork screen's probe holds one too,
/// so the candidate is asked exactly as a seated agent would be).
pub(crate) enum Live {
    SelfAi,
    Random,
    Bin(BinAgent),
}

impl Live {
    pub(crate) fn start(spec: &AgentSpec) -> anyhow::Result<Live> {
        Ok(match spec {
            AgentSpec::SelfAi => Live::SelfAi,
            AgentSpec::Random => Live::Random,
            AgentSpec::Bin(path) => Live::Bin(BinAgent::spawn(path)?),
        })
    }

    fn label(&self) -> &'static str {
        match self {
            Live::SelfAi => "self",
            Live::Random => "random",
            Live::Bin(_) => "bin",
        }
    }

    pub(crate) fn finish(self) {
        if let Live::Bin(bin) = self {
            bin.quit();
        }
    }
}

pub(crate) struct BinAgent {
    path: PathBuf,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// `decide`'s request, in SURFACE §14.1's key order.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecideRequest<'a> {
    op: &'static str,
    state: &'a GameState,
    seat: PlayerId,
    rng_seed: &'a str,
    rng_cursor: u32,
    /// R1489000: always true, for the decision's `reason` and `stats.simPanics`.
    verbose: bool,
}

impl BinAgent {
    pub(crate) fn spawn(path: &Path) -> anyhow::Result<BinAgent> {
        let mut child = Command::new(path)
            .arg("agent")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("arena: cannot start `{} agent`", path.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("arena: {} has no stdin", path.display()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("arena: {} has no stdout", path.display()))?;
        Ok(BinAgent {
            path: path.to_path_buf(),
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub(crate) fn call(&mut self, request: &impl Serialize) -> anyhow::Result<Value> {
        let path = self.path.display().to_string();
        serde_json::to_writer(&mut self.stdin, request)
            .with_context(|| format!("arena: writing to {path}"))?;
        self.stdin
            .write_all(b"\n")
            .with_context(|| format!("arena: writing to {path}"))?;
        self.stdin
            .flush()
            .with_context(|| format!("arena: writing to {path}"))?;
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .with_context(|| format!("arena: reading from {path}"))?;
        if read == 0 {
            bail!("arena: {path} closed its stdout");
        }
        serde_json::from_str(&line).with_context(|| format!("arena: {path} answered a line that is not JSON"))
    }

    pub(crate) fn quit(mut self) {
        let _ = serde_json::to_writer(&mut self.stdin, &json!({ "op": "quit" }));
        let _ = self.stdin.write_all(b"\n");
        let _ = self.stdin.flush();
        drop(self.stdin);
        let _ = self.child.wait();
    }

    /// The agent's move on `redact(state, seat)`; the stream moves to the cursor the agent answers
    /// with. The request is verbose, so the answer tells the decision's `reason` and `stats.simPanics`.
    /// `pub(crate)` for the fork screen's probe (R4397000): the candidate gets the same request.
    pub(crate) fn decide(
        &mut self,
        state: &GameState,
        seat: PlayerId,
        rng_seed: &str,
        rng: &mut Rng,
    ) -> Result<Choice, String> {
        let view = redact(state, seat);
        let request = DecideRequest {
            op: "decide",
            state: &view,
            seat,
            rng_seed,
            rng_cursor: rng.cursor(),
            verbose: true,
        };
        let answer = self.call(&request).map_err(|error| format!("{error:#}"))?;
        if let Some(error) = answer.get("error") {
            return Err(match error.as_str() {
                Some(text) => text.to_string(),
                None => error.to_string(),
            });
        }
        let cursor = answer
            .get("rngCursor")
            .and_then(Value::as_u64)
            .and_then(|cursor| u32::try_from(cursor).ok())
            .ok_or_else(|| "the answer has no rngCursor".to_string())?;
        let action: Option<ActionBody> =
            serde_json::from_value(answer.get("action").cloned().unwrap_or(Value::Null))
                .map_err(|error| format!("the answer's action is not an ActionBody: {error}"))?;
        *rng = Rng::new(rng_seed, cursor);
        let (fell_back, sim_panics) = told_of(&answer);
        Ok(Choice {
            action,
            fell_back,
            sim_panics,
            first_sim_panic: None,
        })
    }
}

/// One controller call's answer: the action, `None` for "no move", and what an AI's decision told of
/// itself (R1489000). `pub(crate)` for the fork screen's probe (R4397000), which asks the candidate
/// exactly as `choose_for` asks a seated agent.
#[derive(Default)]
pub(crate) struct Choice {
    pub action: Option<ActionBody>,
    /// The decision came off the AI's fallback path (`DecisionReason::Fallback`).
    pub fell_back: bool,
    /// The panics its search caught (`SearchStats::sim_panics`).
    pub sim_panics: usize,
    /// The first one's message, when the agent kept it (`self` does; a `bin:` agent sends none).
    pub first_sim_panic: Option<String>,
}

impl Choice {
    /// An answer that is only its action: the random policy's, or a skill miss's (R2214000).
    pub(crate) fn plain(action: Option<ActionBody>) -> Choice {
        Choice {
            action,
            ..Choice::default()
        }
    }

    /// `self`'s answer: the decision's action and what it tells of itself.
    pub(crate) fn of(decision: Option<Decision>) -> Choice {
        let Some(decision) = decision else {
            return Choice::default();
        };
        Choice {
            fell_back: decision.reason == DecisionReason::Fallback,
            sim_panics: decision.stats.sim_panics,
            first_sim_panic: decision.first_sim_panic,
            action: Some(decision.action),
        }
    }
}

/// R1489000: what a `bin:` agent's verbose answer tells of its decision: off the fallback path
/// (`"reason":"fallback"`) and the panics its search caught (`stats.simPanics`). An answer that
/// tells neither counts none.
fn told_of(answer: &Value) -> (bool, usize) {
    let fell_back = answer.get("reason").and_then(Value::as_str) == Some(DecisionReason::Fallback.as_str());
    let sim_panics = answer
        .pointer("/stats/simPanics")
        .and_then(Value::as_u64)
        .map_or(0, |count| usize::try_from(count).unwrap_or(usize::MAX));
    (fell_back, sim_panics)
}

/// One controller call: the agent's action, `None` for "no move", or what it threw. `self` decides on
/// `redact(state, seat)`; `random` plays on the referee's state. R747000: a decision the seat's roll
/// does not give the AI is `jackioh_ai::skill_miss_action`'s (R2214000), handed the referee's state as
/// `random` is and drawn on the same stream; a `bin:` agent is asked only when the roll keeps it.
/// `pub(crate)` for the fork screen's probe (R4397000): the candidate is asked at full skill.
pub(crate) fn choose_for(
    live: &mut Live,
    skill: f64,
    state: &GameState,
    seat: PlayerId,
    rng_seed: &str,
    rng: &mut Rng,
    budget: SearchBudget,
) -> Result<Choice, String> {
    if !matches!(live, Live::Random) && !plays_best(skill, rng) {
        return catch_unwind(AssertUnwindSafe(|| skill_miss_action(state, seat, rng)))
            .map(Choice::plain)
            .map_err(|panic| agent::panic_message(&*panic));
    }
    match live {
        Live::SelfAi => {
            let view = redact(state, seat);
            let start = rng.clone();
            let (decision, next) = catch_unwind(AssertUnwindSafe(|| {
                agent::decide_full(&view, seat, start, budget)
            }))
            .map_err(|panic| agent::panic_message(&*panic))?;
            *rng = next;
            Ok(Choice::of(decision))
        }
        Live::Random => catch_unwind(AssertUnwindSafe(|| random_action(state, seat, rng)))
            .map(Choice::plain)
            .map_err(|panic| agent::panic_message(&*panic)),
        Live::Bin(bin) => bin.decide(state, seat, rng_seed, rng),
    }
}

/// What stands in for a refused action: endTurn if legal, else the first legal answer (or mulligan),
/// else any other legal action the random policy would take.
fn replacements_for(state: &GameState, seat: PlayerId) -> Vec<ActionBody> {
    let legal: Vec<ActionBody> = legal_actions(state, seat)
        .into_iter()
        .filter(|action| !subsystems::AI_SKIPPED_ACTIONS.contains(&action.action_type()))
        .collect();
    let is_answer =
        |action: &ActionBody| matches!(action, ActionBody::Answer { .. } | ActionBody::Mulligan { .. });
    let is_end_turn = |action: &ActionBody| matches!(action, ActionBody::EndTurn);
    let mut out: Vec<ActionBody> = legal
        .iter()
        .filter(|&action| is_end_turn(action))
        .cloned()
        .collect();
    out.extend(legal.iter().filter(|&action| is_answer(action)).cloned());
    out.extend(
        legal
            .iter()
            .filter(|&action| !is_end_turn(action) && !is_answer(action))
            .cloned(),
    );
    out
}

/// The chosen action, or, when the reducer refuses it, the first replacement it accepts (recorded).
/// `pub(crate)` for the fork screen's rollouts (R4397000): each arm's first action goes through it.
pub(crate) fn accept(
    state: &GameState,
    seat: PlayerId,
    chosen: &ActionBody,
    nonce: &str,
    rejected: &mut Vec<RejectedAction>,
) -> Option<(Action, GameState)> {
    let action = Action::new(chosen.clone(), seat, nonce);
    let result = reduce(state, &action);
    let Some(error) = result.error else {
        return Some((action, result.state));
    };
    rejected.push(RejectedAction {
        seat,
        action: chosen.clone(),
        error,
    });
    for replacement in replacements_for(state, seat) {
        let alternative = Action::new(replacement, seat, nonce);
        let retry = reduce(state, &alternative);
        if retry.error.is_none() {
            return Some((alternative, retry.state));
        }
    }
    None
}

/// A game in progress, from its first decision to its last, so a fork rollout can play on from a
/// mid-game state beside `play_game` (R4397000). The trunk builds one per game; a rollout builds one
/// per arm, with the fork's streams and the control on both seats.
pub(crate) struct FinishCtx {
    /// Each seat's stream seed (`<seed>:ctl:<seat>` in a game, the fork's own in a rollout arm).
    pub ctl: PerPlayer<String>,
    pub live: PerPlayer<Live>,
    pub skills: PerPlayer<f64>,
    /// R1489000: the budget a `self` seat decides at (a `bin:` agent decides at its own build's).
    pub budget: SearchBudget,
    pub turn_actions: Option<usize>,
    pub stop_unsettled: bool,
    pub log: Vec<Action>,
    pub rejected: Vec<RejectedAction>,
    pub thrown: Vec<String>,
    pub fallbacks: PerPlayer<i32>,
    pub sim_panics: PerPlayer<usize>,
    pub first_sim_panic: Option<SimPanic>,
    pub times: PerPlayer<Vec<f64>>,
    pub rngs: PerPlayer<Rng>,
    pub turn: i32,
    pub this_turn: usize,
    /// R4397000: the fork screen's probe, asking the candidate at each decision; `None` elsewhere.
    pub probe: Option<crate::fork_screen::Probe>,
}

/// What playing on came to: the result, the final hash and turn, everything the loop collected, and
/// the fork screen's probe points and probe faults beside them (R4397000).
pub(crate) struct PlayTail {
    pub result: Option<GameResult>,
    pub hash: String,
    pub turns: i32,
    pub log: Vec<Action>,
    pub rejected: Vec<RejectedAction>,
    pub thrown: Vec<String>,
    pub fallbacks: PerPlayer<i32>,
    pub sim_panics: PerPlayer<usize>,
    pub first_sim_panic: Option<SimPanic>,
    pub times: PerPlayer<Vec<f64>>,
    pub points: Vec<crate::fork_screen::ForkPoint>,
    pub probe_faults: Vec<String>,
}

/// Plays on from `state` to the end through `ctx`, finishing its agents after. The trunk starts here
/// from the game's first state; a fork rollout starts here from its arm's state (R4397000). Rayon
/// runs games and arms in parallel, but every input is owned, so completion order never reaches the
/// output.
pub(crate) fn finish_from(mut state: GameState, mut ctx: FinishCtx) -> PlayTail {
    while state.result.is_none() && ctx.log.len() < ARENA_MAX_ACTIONS {
        // R1006005: a turn that will not end (an endless combo the AI keeps finding worth playing)
        // ends the game without a result once it passes the ceiling.
        if state.turn != ctx.turn {
            ctx.turn = state.turn;
            ctx.this_turn = 0;
        }
        if let Some(ceiling) = ctx.turn_actions
            && ctx.this_turn >= ceiling
        {
            ctx.thrown.push(format!(
                "{} actions in turn {}, the ceiling of one turn (R1006005)",
                ctx.this_turn, ctx.turn
            ));
            break;
        }
        ctx.this_turn += 1;
        let Some(seat) = seat_to_act(&state) else {
            ctx.thrown.push(format!(
                "no seat to act while the game is live (turn {})",
                state.turn
            ));
            break;
        };

        // R4397000: the candidate sees the same request the control's seat got, on the stream cloned
        // before the control's decision moves it.
        let probe_rng = ctx.probe.as_ref().map(|_| ctx.rngs[seat].clone());

        let started = Instant::now();
        let budget = ctx.budget;
        let choice = if ctx.stop_unsettled && matches!(ctx.live[seat], Live::SelfAi) {
            // R1006007: a decision whose search runs a board to a settle cap ends the game without
            // a result, at the cap's first panic: the search would reach the loop again and again.
            let (skill, at, ctl_seed, start) = (
                ctx.skills[seat],
                state.clone(),
                ctx.ctl[seat].clone(),
                ctx.rngs[seat].clone(),
            );
            let watched = watch_unsettled(move || {
                let mut rng = start;
                let choice = choose_for(&mut Live::SelfAi, skill, &at, seat, &ctl_seed, &mut rng, budget);
                (choice, rng)
            });
            let Some((choice, next)) = watched else {
                ctx.thrown.push(format!(
                    "{}: a line the search played did not settle (turn {}, R1006007)",
                    seat.as_str(),
                    state.turn
                ));
                break;
            };
            ctx.rngs[seat] = next;
            choice
        } else {
            choose_for(
                &mut ctx.live[seat],
                ctx.skills[seat],
                &state,
                seat,
                &ctx.ctl[seat],
                &mut ctx.rngs[seat],
                ctx.budget,
            )
        };
        ctx.times[seat].push(started.elapsed().as_secs_f64() * MS_PER_SECOND);
        let choice = match choice {
            Ok(choice) => choice,
            Err(message) => {
                ctx.thrown.push(format!(
                    "{}: controller {} threw: {message}",
                    seat.as_str(),
                    ctx.live[seat].label()
                ));
                break;
            }
        };
        // R1489000: what an AI's decision told of itself, counted as match_.rs's `play_match` counts it.
        if choice.fell_back {
            ctx.fallbacks[seat] += 1;
        }
        ctx.sim_panics[seat] += choice.sim_panics;
        if ctx.first_sim_panic.is_none()
            && let Some(message) = &choice.first_sim_panic
        {
            ctx.first_sim_panic = Some(SimPanic {
                seat,
                message: message.clone(),
            });
        }

        let chosen = match choice.action {
            Some(chosen) => chosen,
            None => {
                ctx.fallbacks[seat] += 1;
                match random_action(&state, seat, &mut ctx.rngs[seat]) {
                    Some(chosen) => chosen,
                    None => {
                        ctx.thrown.push(format!(
                            "no action for {} while the game is live (turn {})",
                            seat.as_str(),
                            state.turn
                        ));
                        break;
                    }
                }
            }
        };

        // R4397000: ask the candidate before the trunk plays on with the control's action; the probe
        // compares its answer to the final played body and records the fork, if any.
        if let (Some(probe), Some(rng)) = (ctx.probe.as_mut(), probe_rng.as_ref()) {
            probe.check(&state, seat, &ctx.ctl[seat], rng, &chosen, ctx.log.len());
        }

        let nonce = format!("m{}", ctx.log.len());
        let current = &state;
        let step = catch_unwind(AssertUnwindSafe(|| {
            accept(current, seat, &chosen, &nonce, &mut ctx.rejected)
        }));
        let accepted = match step {
            Ok(accepted) => accepted,
            Err(panic) => {
                ctx.thrown.push(format!(
                    "{}: reduce threw on \"{}\": {}",
                    seat.as_str(),
                    chosen.action_type().as_str(),
                    agent::panic_message(&*panic)
                ));
                break;
            }
        };
        let Some((action, next)) = accepted else {
            ctx.thrown.push(format!(
                "{}: no legal replacement for a refused \"{}\" (turn {})",
                seat.as_str(),
                chosen.action_type().as_str(),
                state.turn
            ));
            break;
        };

        ctx.log.push(action);
        state = next;
    }

    let PerPlayer { p1, p2 } = ctx.live;
    p1.finish();
    p2.finish();
    let (points, probe_faults) = ctx
        .probe
        .map(crate::fork_screen::Probe::finish)
        .unwrap_or_default();

    PlayTail {
        result: state.result,
        hash: hash_state(&state),
        turns: state.turn,
        log: ctx.log,
        rejected: ctx.rejected,
        thrown: ctx.thrown,
        fallbacks: ctx.fallbacks,
        sim_panics: ctx.sim_panics,
        first_sim_panic: ctx.first_sim_panic,
        times: ctx.times,
        points,
        probe_faults,
    }
}

/// Plays one game with the fork screen's probe beside it, asking the candidate at each decision
/// until the game's fork cap (R4397000). The game always plays on with its own seats' actions; the
/// probe only records where the candidate would have differed, with its query faults beside them.
pub(crate) fn play_game_probed(
    game: &ArenaGame,
    probe: Option<crate::fork_screen::Probe>,
) -> anyhow::Result<(ArenaOutcome, Vec<crate::fork_screen::ForkPoint>, Vec<String>)> {
    let options = CreateGameOptions {
        seed: game.seed.clone(),
        decks: game.decks.clone(),
        ..Default::default()
    };
    let state =
        catch_unwind(AssertUnwindSafe(|| begin_game(&create_game(&options)).state)).map_err(|panic| {
            anyhow!(
                "{}: the game cannot start: {}",
                game.seed,
                agent::panic_message(&*panic)
            )
        })?;

    let ctl = PerPlayer::new(format!("{}:ctl:p1", game.seed), format!("{}:ctl:p2", game.seed));
    let turn = state.turn;
    let ctx = FinishCtx {
        rngs: PerPlayer::new(Rng::new(&ctl.p1, 0), Rng::new(&ctl.p2, 0)),
        ctl,
        live: PerPlayer::new(Live::start(&game.seats.p1)?, Live::start(&game.seats.p2)?),
        skills: PerPlayer::new(game.skills[PlayerId::P1], game.skills[PlayerId::P2]),
        budget: game.self_budget,
        turn_actions: game.turn_actions,
        stop_unsettled: game.stop_unsettled,
        log: Vec::new(),
        rejected: Vec::new(),
        thrown: Vec::new(),
        fallbacks: PerPlayer::new(0, 0),
        sim_panics: PerPlayer::new(0, 0),
        first_sim_panic: None,
        times: PerPlayer::new(Vec::new(), Vec::new()),
        turn,
        this_turn: 0,
        probe,
    };
    let tail = finish_from(state, ctx);

    Ok((
        ArenaOutcome {
            game: game.clone(),
            result: tail.result,
            hash: tail.hash,
            turns: tail.turns,
            log: tail.log,
            rejected: tail.rejected,
            thrown: tail.thrown,
            fallbacks: tail.fallbacks,
            sim_panics: tail.sim_panics,
            first_sim_panic: tail.first_sim_panic,
            times: tail.times,
        },
        tail.points,
        tail.probe_faults,
    ))
}

/// Plays one game: the actor is `seat_to_act` (R265), its stream `<seed>:ctl:<seat>`, the nonce `m<log
/// length>`. "No move" is replaced by the random policy; a refused action is recorded in `rejected`
/// and replaced; a throw ends the game.
pub(crate) fn play_game(game: &ArenaGame) -> anyhow::Result<ArenaOutcome> {
    play_game_probed(game, None).map(|(outcome, _, _)| outcome)
}

fn fold_args(outcome: &ArenaOutcome) -> anyhow::Result<FoldArgs> {
    let args = json!({
        "seed": outcome.game.seed,
        "decks": [outcome.game.decks.0, outcome.game.decks.1],
        "log": outcome.log,
    });
    serde_json::from_value(args).context("arena: a game's fold input does not parse")
}

pub(crate) fn replays(outcome: &ArenaOutcome) -> anyhow::Result<bool> {
    let replay = fold(&fold_args(outcome)?);
    Ok(replay.errors.is_empty() && hash_state(&replay.state) == outcome.hash)
}

/// R378: an arena record's id, `dev:<patch>:arena:<a>-vs-<b>:<seed>`; the agents are part of it, so
/// games sharing seeds against random and against the parent are separate records. A paired game
/// adds agent `a`'s seat (`…:<seed>:p1`), so a deal's two games are two records (R850000).
pub(crate) fn record_id(patch: &str, game: &ArenaGame) -> String {
    let id = format!(
        "{DEV_RECORD_ID_PREFIX}{patch}:arena:{}-vs-{}:{}",
        game.labels.0, game.labels.1, game.seed
    );
    if game.paired {
        format!("{id}:{}", game.a_seat.as_str())
    } else {
        id
    }
}

/// R376, R378: a finished game filed as a development record (`source: "dev"`, `mode: "random"`,
/// both seats piloted by an AI). `None` for a game without a result: only a finished game is a record.
pub(crate) fn game_record(outcome: &ArenaOutcome) -> anyhow::Result<Option<GameRecord>> {
    if outcome.result.is_none() {
        return Ok(None);
    }
    let Some(game) = summarize_game(&fold_args(outcome)?) else {
        return Ok(None);
    };
    let patch = jackioh_cards::catalog_version();
    let record = json!({
        "id": record_id(patch, &outcome.game),
        "source": "dev",
        "mode": "random",
        "patch": patch,
        "pilots": { "p1": "ai", "p2": "ai" },
        "game": game,
    });
    let mut record: GameRecord =
        serde_json::from_value(record).context("arena: a game record does not parse as a GameRecord")?;
    // R1367000: the build that played it.
    record.build = RecordBuild::of(
        crate::selfplay::build_commit().as_deref(),
        jackioh_ai::ai_generation(),
        &record.pilots,
    );
    Ok(Some(record))
}

/// The directory a run's output goes to: `out`, else `$JACKIOH_TRAINING_OUT`, else `None`.
pub(crate) fn training_out(out: Option<&Path>) -> Option<PathBuf> {
    match out {
        Some(dir) => Some(dir.to_path_buf()),
        None => match std::env::var_os(TRAINING_OUT_ENV) {
            Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
            _ => None,
        },
    }
}

/// Where the records go: `<out>/<date>.jsonl`, else `$JACKIOH_TRAINING_OUT/<date>.jsonl`, else `None`.
pub(crate) fn records_path(out: Option<&Path>) -> Option<PathBuf> {
    training_out(out).map(|dir| dir.join(format!("{}.jsonl", utc_date())))
}

pub(crate) fn append_records(path: &Path, records: &[GameRecord]) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("arena: cannot create {}", dir.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("arena: cannot open {}", path.display()))?;
    let mut text = String::new();
    for record in records {
        text.push_str(&jackioh_cards::versions::game_record_line(record));
        text.push('\n');
    }
    file.write_all(text.as_bytes())
        .with_context(|| format!("arena: cannot write {}", path.display()))?;
    Ok(())
}

/// The Unix time now, in whole seconds.
pub(crate) fn unix_now() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

pub(crate) fn utc_date() -> String {
    utc_date_of(unix_now())
}

/// One game's duel line from agent `a`'s side, in key order.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DuelLine<'a> {
    n: i32,
    seed: &'a str,
    subject: PlayerId,
    won: bool,
    draw: bool,
    lost: bool,
    aborted: bool,
    reason: Option<String>,
    turns: i32,
    hash: &'a str,
    rejected: usize,
    thrown: &'a [String],
    fallbacks: i32,
    sim_panics: usize,
    replay_ok: bool,
    decisions: usize,
    ms_sum: i64,
    ms_max: i64,
    ms_p95: i64,
}

fn round_ms(ms: f64) -> i64 {
    (ms + 0.5).floor() as i64
}

/// The per-game duel line: outcome from a's side, `aborted` (no result; no gate counts it a draw),
/// why it ended, hash, whether it replays, both seats' fallbacks and caught search panics, and a's
/// decision count and times. `pub(crate)` for the fork screen's trunk lines (R4397000).
pub(crate) fn duel_line(outcome: &ArenaOutcome, replay_ok: bool) -> anyhow::Result<String> {
    let subject = outcome.game.a_seat;
    let other = subject.opponent();
    let mine = &outcome.times[subject];
    let mut sorted = mine.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let p95_at = (sorted.len() as f64 * DUEL_PERCENTILE).floor() as usize;
    let line = DuelLine {
        n: outcome.game.n,
        seed: &outcome.game.seed,
        subject,
        won: won_by(outcome.result, subject),
        draw: outcome.drawn(),
        lost: won_by(outcome.result, other),
        aborted: outcome.result.is_none(),
        reason: outcome.result.map(|result| result.reason.as_str().to_string()),
        turns: outcome.turns,
        hash: &outcome.hash,
        rejected: outcome.rejected.len(),
        thrown: &outcome.thrown,
        fallbacks: outcome.fallbacks.p1 + outcome.fallbacks.p2,
        sim_panics: outcome.sim_panics.p1 + outcome.sim_panics.p2,
        replay_ok,
        decisions: mine.len(),
        ms_sum: round_ms(mine.iter().sum()),
        ms_max: round_ms(sorted.last().copied().unwrap_or(0.0)),
        ms_p95: round_ms(sorted.get(p95_at).copied().unwrap_or(0.0)),
    };
    Ok(serde_json::to_string(&line)?)
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Tally {
    pub a: String,
    pub b: String,
    pub games: usize,
    pub a_wins: usize,
    pub b_wins: usize,
    pub draws: usize,
    pub aborted: usize,
    /// `a_wins` over `games`, and its 95% Wilson interval.
    pub a_share: f64,
    pub a_low: f64,
    pub a_high: f64,
    /// R850000: a paired series' edge; nothing for an unpaired one.
    #[serde(flatten)]
    pub paired: Option<Paired>,
}

/// R850000: a paired series' deals and agent `a`'s edge over them, the mean of each deal's wins less
/// the other agent's over its two games, with that mean's 95% interval.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Paired {
    pub deals: usize,
    pub edge: f64,
    pub edge_low: f64,
    pub edge_high: f64,
    /// R4319000: the deals whose edge is not 0, split into both-won and both-lost.
    pub decided: usize,
    pub won_both: usize,
    pub lost_both: usize,
    /// R4319000: the deals whose two games end on different hashes, so the two agents played
    /// differently at least once.
    pub diverged: usize,
    /// R4319000: on a screen (`--set`) with fewer than `SCREEN_MIN_DECIDED` decided deals, which
    /// says nothing; absent otherwise, so passing screens and other series print no new key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inconclusive: Option<bool>,
}

/// The 95% Wilson score interval of `wins` in `games`, the engine's (R1994000), which the game
/// figures read too.
pub(crate) fn wilson_interval(wins: usize, games: usize) -> (f64, f64) {
    let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    jackioh_engine::wire::stats::wilson_interval(count(wins), count(games))
}

/// R850000: each deal's edge for the agent whose seat each game names, over games given as (result,
/// that agent's seat) in deal order, two to a deal: its wins less the other agent's, over the deal's
/// two games, from 1 (both won) to −1 (both lost). A draw or a game without a result is nobody's win.
pub(crate) fn deal_edges(games: &[(Option<GameResult>, PlayerId)]) -> Vec<f64> {
    games
        .chunks(2)
        .map(|deal| {
            let wins = deal
                .iter()
                .filter(|(result, seat)| won_by(*result, *seat))
                .count();
            let losses = deal
                .iter()
                .filter(|(result, seat)| won_by(*result, seat.opponent()))
                .count();
            (wins as f64 - losses as f64) / 2.0
        })
        .collect()
}

/// R850000: the mean of the deals' edges and its 95% interval, the mean plus or minus `STATS_INTERVAL_Z`
/// standard errors (the sample deviation over the root of the deals), inside [−1, 1]. Fewer than two
/// deals have no spread to read, so their interval is all of [−1, 1].
pub(crate) fn paired_interval(edges: &[f64]) -> (f64, f64, f64) {
    if edges.is_empty() {
        return (0.0, -1.0, 1.0);
    }
    let n = edges.len() as f64;
    let mean = edges.iter().sum::<f64>() / n;
    if edges.len() < 2 {
        return (mean, -1.0, 1.0);
    }
    let variance = edges.iter().map(|edge| (edge - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let half = STATS_INTERVAL_Z * (variance / n).sqrt();
    (mean, (mean - half).max(-1.0), (mean + half).min(1.0))
}

/// A tally's shares to three places, as the game figures round theirs (R1994000).
pub(crate) use jackioh_engine::wire::stats::round_share;

/// R4319000: how many of the deals' edges a screen decided, split into both-won and both-lost. The
/// edges are exact halves, so a deal with a draw or a game without a result counts as decided when
/// its edge is not 0.
pub(crate) fn decided_of(edges: &[f64]) -> (usize, usize, usize) {
    let mut decided = 0;
    let mut won_both = 0;
    let mut lost_both = 0;
    for edge in edges {
        if *edge != 0.0 {
            decided += 1;
        }
        if *edge == 1.0 {
            won_both += 1;
        } else if *edge == -1.0 {
            lost_both += 1;
        }
    }
    (decided, won_both, lost_both)
}

/// R4319000: how many deals' two games end on different hashes. A trailing lone game (tests only;
/// `series()` forces even paired counts) never diverges.
pub(crate) fn diverged_of(outcomes: &[ArenaOutcome]) -> usize {
    outcomes
        .chunks(2)
        .filter(|deal| deal.len() == 2 && deal[0].hash != deal[1].hash)
        .count()
}

/// R4319000, R4397000: whether `--extend` stops with this many decided deals (or live forks) after
/// this many games: at the floor, or at the cap. The stop looks at how many deals were decided (or
/// forks are live), never which way they went.
pub(crate) fn screen_done(decided: usize, games: i32) -> bool {
    decided >= SCREEN_MIN_DECIDED || games >= SCREEN_MAX_GAMES
}

pub(crate) fn tally(a: &str, b: &str, outcomes: &[ArenaOutcome], paired: bool, screen: bool) -> Tally {
    let games = outcomes.len();
    let a_wins = outcomes.iter().filter(|outcome| outcome.a_won()).count();
    let (low, high) = wilson_interval(a_wins, games);
    Tally {
        a: a.to_string(),
        b: b.to_string(),
        games,
        a_wins,
        b_wins: outcomes
            .iter()
            .filter(|outcome| won_by(outcome.result, outcome.game.a_seat.opponent()))
            .count(),
        draws: outcomes.iter().filter(|outcome| outcome.drawn()).count(),
        aborted: outcomes.iter().filter(|outcome| outcome.result.is_none()).count(),
        a_share: round_share(if games == 0 {
            0.0
        } else {
            a_wins as f64 / games as f64
        }),
        a_low: round_share(low),
        a_high: round_share(high),
        paired: paired.then(|| {
            let results: Vec<(Option<GameResult>, PlayerId)> = outcomes
                .iter()
                .map(|outcome| (outcome.result, outcome.game.a_seat))
                .collect();
            let edges = deal_edges(&results);
            let (edge, edge_low, edge_high) = paired_interval(&edges);
            let (decided, won_both, lost_both) = decided_of(&edges);
            Paired {
                deals: edges.len(),
                edge: round_share(edge),
                edge_low: round_share(edge_low),
                edge_high: round_share(edge_high),
                decided,
                won_both,
                lost_both,
                diverged: diverged_of(outcomes),
                inconclusive: (screen && decided < SCREEN_MIN_DECIDED).then_some(true),
            }
        }),
    }
}

pub(crate) fn records_of(outcomes: &[ArenaOutcome]) -> anyhow::Result<Vec<GameRecord>> {
    let mut records = Vec::new();
    for outcome in outcomes {
        if let Some(record) = game_record(outcome)? {
            records.push(record);
        }
    }
    Ok(records)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    if args.forks {
        return crate::fork_screen::run(args);
    }
    jackioh_cards::register_all();
    let (base, paired, games) = series(&args)?;
    let screen = args.set.is_some();
    let a = entrant_at(&args.a, args.a_skill)?;
    let b = entrant_at(&args.b, args.b_skill)?;
    // R4319000: `--extend` plays a screen in batches after the first, in deal order, until
    // `SCREEN_MIN_DECIDED` deals are decided or `SCREEN_MAX_GAMES` games are played. `setups`
    // hands the seed closure the deal's number when paired (which a screen always is), so the
    // batch's deals continue where the last batch's left off; the edge is read only at the end.
    let mut outcomes: Vec<ArenaOutcome> = Vec::new();
    let mut games_done = 0i32;
    let mut deals_done = 0i32;
    loop {
        let batch_games = if games_done == 0 {
            games
        } else {
            (SCREEN_MAX_GAMES - games_done).min(SCREEN_EXTEND_DEALS * 2)
        };
        let mut batch = setups(&a, &b, batch_games, paired, |d| {
            format!("{base}:{}", deals_done + d)
        })?;
        for game in &mut batch {
            game.n += games_done;
        }
        outcomes.extend(play_games(&batch)?);
        games_done += batch_games;
        deals_done += batch_games / 2;
        if !(args.extend && screen) {
            break;
        }
        let results: Vec<(Option<GameResult>, PlayerId)> = outcomes
            .iter()
            .map(|outcome| (outcome.result, outcome.game.a_seat))
            .collect();
        let (decided, _, _) = decided_of(&deal_edges(&results));
        if screen_done(decided, games_done) {
            break;
        }
    }
    let records = records_of(&outcomes)?;

    let path = records_path(args.out.as_deref());
    let mut lines = Vec::new();
    for outcome in &outcomes {
        lines.push(duel_line(outcome, replays(outcome)?)?);
    }
    let totals = serde_json::to_string(&tally(&a.label, &b.label, &outcomes, paired, screen))?;

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
    use jackioh_engine::GameOverReason;

    const SECONDS_PER_DAY: i64 = 86_400;

    fn self_entrant() -> Entrant {
        entrant(&AgentSpec::SelfAi).unwrap()
    }

    #[test]
    fn agents_parse_as_surface_writes_them() {
        assert_eq!(parse_agent("self"), Ok(AgentSpec::SelfAi));
        assert_eq!(parse_agent("random"), Ok(AgentSpec::Random));
        assert_eq!(
            parse_agent("bin:/opt/jackioh"),
            Ok(AgentSpec::Bin(PathBuf::from("/opt/jackioh")))
        );
        assert!(parse_agent("bin:").is_err());
        assert!(parse_agent("greedy").is_err());
    }

    #[test]
    fn seats_alternate_with_a_on_p1_in_odd_games() {
        jackioh_cards::register_all();
        let a = self_entrant();
        let b = entrant(&AgentSpec::Random).unwrap();
        let games = setups(&a, &b, 4, false, |n| format!("arena-test:{n}")).unwrap();
        let seats: Vec<PlayerId> = games.iter().map(|game| game.a_seat).collect();
        assert_eq!(
            seats,
            vec![PlayerId::P1, PlayerId::P2, PlayerId::P1, PlayerId::P2]
        );
        assert_eq!(games[0].seats.p1, AgentSpec::SelfAi);
        assert_eq!(games[1].seats.p1, AgentSpec::Random);
        assert_eq!(games[2].seed, "arena-test:3");
    }

    #[test]
    #[ignore = "daily: eight full-skill games; r747000_a_mid_skill_game_reproduces_and_replays holds an arena game's replay per pull request"]
    fn two_self_agents_over_four_games_are_deterministic() {
        jackioh_cards::register_all();
        let a = self_entrant();
        let b = self_entrant();
        let games = setups(&a, &b, 4, false, |n| format!("arena-determinism:{n}")).unwrap();
        let first = play_games(&games).unwrap();
        let second = play_games(&games).unwrap();
        let hashes = |outcomes: &[ArenaOutcome]| {
            outcomes
                .iter()
                .map(|outcome| outcome.hash.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(hashes(&first), hashes(&second));
        let records_first = records_of(&first).unwrap();
        let records_second = records_of(&second).unwrap();
        assert_eq!(records_first, records_second);
        assert_eq!(
            serde_json::to_string(&records_first).unwrap(),
            serde_json::to_string(&records_second).unwrap()
        );
        for outcome in &first {
            assert!(
                replays(outcome).unwrap(),
                "{} does not replay to its hash",
                outcome.game.seed
            );
            // Clean, as a gate game must be (B31): `decide` redacts again and must never throw.
            assert_eq!(outcome.thrown, Vec::<String>::new(), "{}", outcome.game.seed);
            assert!(outcome.result.is_some(), "{} has no result", outcome.game.seed);
        }
    }

    #[test]
    fn a_draw_is_not_a_win_and_no_result_is_nobodys() {
        let draw = Some(GameResult {
            winner: Winner::Draw,
            reason: GameOverReason::TurnCap,
        });
        assert!(!won_by(draw, PlayerId::P1));
        assert!(!won_by(draw, PlayerId::P2));
        assert!(!won_by(None, PlayerId::P1));
        let p2 = Some(GameResult {
            winner: Winner::P2,
            reason: GameOverReason::HeroDeath,
        });
        assert!(won_by(p2, PlayerId::P2));
        assert!(!won_by(p2, PlayerId::P1));
    }

    #[test]
    fn the_wilson_interval_brackets_the_share_and_narrows_with_games() {
        // 65 of 100: the improve gate's bar reads as anything from about 55% to 74%.
        let (low, high) = wilson_interval(65, 100);
        assert_eq!((round_share(low), round_share(high)), (0.553, 0.736));
        // Four times the games halves the width, near enough.
        let (low_400, high_400) = wilson_interval(260, 400);
        assert!(high_400 - low_400 < (high - low) * 0.55);
        assert!(low_400 < 0.65 && 0.65 < high_400);
        // It stays in [0, 1] at the ends, and no games says nothing.
        let (none_low, none_high) = wilson_interval(0, 60);
        assert_eq!(none_low, 0.0);
        assert!(none_high > 0.0);
        let (all_low, all_high) = wilson_interval(60, 60);
        assert!(all_low < 1.0);
        assert_eq!(round_share(all_high), 1.0);
        assert_eq!(wilson_interval(0, 0), (0.0, 1.0));
    }

    #[test]
    fn record_ids_name_the_agents_and_the_seed() {
        let game = ArenaGame {
            n: 1,
            seed: "improve:abc:1".to_string(),
            decks: (Vec::new(), Vec::new()),
            seats: PerPlayer::new(AgentSpec::SelfAi, AgentSpec::Random),
            skills: PerPlayer::new(AI_SKILL_MAX, AI_SKILL_MAX),
            a_seat: PlayerId::P1,
            labels: ("self".to_string(), "random".to_string()),
            paired: false,
            turn_actions: None,
            stop_unsettled: false,
            self_budget: AI_GATE_BUDGET,
        };
        assert_eq!(
            record_id("0.2.0", &game),
            "dev:0.2.0:arena:self-vs-random:improve:abc:1"
        );
    }

    #[test]
    fn r850000_a_paired_series_plays_each_deal_twice_with_the_seats_swapped() {
        jackioh_cards::register_all();
        let a = self_entrant();
        let b = entrant(&AgentSpec::Random).unwrap();
        let games = setups(&a, &b, 4, true, |d| format!("t:{d}")).unwrap();
        let seeds: Vec<&str> = games.iter().map(|game| game.seed.as_str()).collect();
        assert_eq!(seeds, vec!["t:1", "t:1", "t:2", "t:2"]);
        let seats: Vec<PlayerId> = games.iter().map(|game| game.a_seat).collect();
        assert_eq!(
            seats,
            vec![PlayerId::P1, PlayerId::P2, PlayerId::P1, PlayerId::P2]
        );
        // One shadow ban on both sides: a deal's two games deal the same decks, the agents swapped.
        assert_eq!(games[0].decks, games[1].decks);
        assert_eq!(games[2].decks, games[3].decks);
        assert_ne!(games[0].decks, games[2].decks);
        assert_eq!(games[0].seats.p1, AgentSpec::SelfAi);
        assert_eq!(games[1].seats.p2, AgentSpec::SelfAi);
        assert_eq!(games[1].seats.p1, AgentSpec::Random);
        // R378: the deal's two games are two records.
        assert!(record_id("0.2.0", &games[0]).ends_with(":self-vs-random:t:1:p1"));
        assert!(record_id("0.2.0", &games[1]).ends_with(":self-vs-random:t:1:p2"));
        assert_eq!(deal_of(1), 1);
        assert_eq!(deal_of(2), 1);
        assert_eq!(deal_of(399), 200);
        assert_eq!(deal_of(400), 200);
    }

    #[test]
    fn r850000_one_ai_on_both_seats_splits_every_deal() {
        jackioh_cards::register_all();
        let a = self_entrant();
        let b = self_entrant();
        let mut games = setups(&a, &b, 2, true, |d| format!("r850000-split:{d}")).unwrap();
        for game in &mut games {
            game.self_budget = TEST_BUDGET;
        }
        let outcomes = play_games(&games).unwrap();
        // The same AI on the same deal plays the same game whichever agent sits where.
        assert_eq!(outcomes[0].hash, outcomes[1].hash);
        assert_eq!(outcomes[0].log, outcomes[1].log);
        assert!(outcomes[0].result.is_some());
        let totals = tally("self", "self", &outcomes, true, false);
        assert_eq!(
            totals.paired,
            Some(Paired {
                deals: 1,
                edge: 0.0,
                edge_low: -1.0,
                edge_high: 1.0,
                decided: 0,
                won_both: 0,
                lost_both: 0,
                diverged: 0,
                inconclusive: None,
            })
        );
        let paired = totals.paired.as_ref().unwrap();
        assert_eq!(paired.decided, 0);
        assert_eq!(paired.diverged, 0);
    }

    #[test]
    fn r850000_the_paired_interval_is_the_deals_mean_and_spread() {
        let ended = |winner| {
            Some(GameResult {
                winner,
                reason: GameOverReason::HeroDeath,
            })
        };
        let draw = Some(GameResult {
            winner: Winner::Draw,
            reason: GameOverReason::TurnCap,
        });
        let games = vec![
            // Both won, then a draw and a game without a result, then one each, then both lost.
            (ended(Winner::P1), PlayerId::P1),
            (ended(Winner::P2), PlayerId::P2),
            (draw, PlayerId::P1),
            (None, PlayerId::P2),
            (ended(Winner::P2), PlayerId::P1),
            (ended(Winner::P2), PlayerId::P2),
            (ended(Winner::P2), PlayerId::P1),
            (ended(Winner::P1), PlayerId::P2),
        ];
        assert_eq!(deal_edges(&games), vec![1.0, 0.0, 0.0, -1.0]);

        let rounded =
            |(mean, low, high): (f64, f64, f64)| (round_share(mean), round_share(low), round_share(high));
        assert_eq!(paired_interval(&[]), (0.0, -1.0, 1.0));
        assert_eq!(paired_interval(&[1.0]), (1.0, -1.0, 1.0));
        assert_eq!(paired_interval(&[0.0; 200]), (0.0, 0.0, 0.0));
        let mut edges = vec![1.0; 60];
        edges.extend([-1.0; 40]);
        edges.extend([0.0; 100]);
        assert_eq!(rounded(paired_interval(&edges)), (0.1, 0.003, 0.197));

        // An unpaired tally prints no edge; a paired one ends with it.
        let unpaired = serde_json::to_string(&tally("self", "self", &[], false, false)).unwrap();
        assert!(!unpaired.contains("\"deals\""), "{unpaired}");
        let paired = serde_json::to_string(&tally("self", "self", &[], true, false)).unwrap();
        assert!(
            paired.ends_with(
                r#""deals":0,"edge":0.0,"edgeLow":-1.0,"edgeHigh":1.0,"decided":0,"wonBoth":0,"lostBoth":0,"diverged":0}"#
            ),
            "{paired}"
        );
    }

    fn screen(
        seed: Option<&str>,
        set: Option<SeedSet>,
        games: Option<i32>,
        paired: bool,
        extend: bool,
    ) -> Args {
        Args {
            a: AgentSpec::SelfAi,
            b: AgentSpec::Random,
            games,
            seed: seed.map(str::to_string),
            set,
            paired,
            extend,
            forks: false,
            out: None,
            a_skill: AI_SKILL_MAX,
            b_skill: AI_SKILL_MAX,
        }
    }

    #[test]
    fn r850001_screens_play_the_dev_set_and_never_a_gates_seeds() {
        assert_eq!(
            series(&screen(None, Some(SeedSet::ImproveDev), None, false, false)).unwrap(),
            ("improve-dev".to_string(), true, SCREEN_GAMES)
        );
        assert_eq!(SCREEN_GAMES, 400);
        assert_eq!(
            series(&screen(None, Some(SeedSet::ImproveDev), Some(800), false, false)).unwrap(),
            ("improve-dev".to_string(), true, 800)
        );
        // A screen on the dev set plays at least 400 games.
        assert!(series(&screen(None, Some(SeedSet::ImproveDev), Some(398), false, false)).is_err());
        assert_eq!(
            series(&screen(Some("improve-try-3"), None, Some(60), false, false)).unwrap(),
            ("improve-try-3".to_string(), false, 60)
        );
        assert_eq!(
            series(&screen(Some("improve-try-3"), None, Some(60), true, false)).unwrap(),
            ("improve-try-3".to_string(), true, 60)
        );
        // A gate's own seeds, the promotion's hold-out deals, are no screen's.
        for gate in ["improve", "improve:abc", "unban:abc:7", "unban"] {
            let refused = series(&screen(Some(gate), None, Some(400), true, false)).unwrap_err();
            assert!(refused.to_string().contains("R850001"), "{gate}: {refused}");
        }
        // A base needs its games, a paired series an even count, and every series a game.
        assert!(series(&screen(Some("improve-try-3"), None, None, false, false)).is_err());
        assert!(series(&screen(Some("improve-try-3"), None, Some(5), true, false)).is_err());
        assert!(series(&screen(None, Some(SeedSet::ImproveDev), Some(401), false, false)).is_err());
        assert!(series(&screen(Some("improve-try-3"), None, Some(0), false, false)).is_err());
        assert!(series(&screen(None, None, Some(400), false, false)).is_err());
        for own in ["improve-dev", "improve-try-3", "unban-try-1", "improvement:1"] {
            assert!(!crate::promote::is_gate_seed_base(own), "{own}");
        }
        // No deal of the dev set is a gate's, whatever the tree.
        let base = SeedSet::ImproveDev.base();
        for d in 1..=deal_of(SCREEN_GAMES) {
            let seed = format!("{base}:{d}");
            assert!(
                !seed.starts_with("improve:") && !seed.starts_with("unban:"),
                "{seed}"
            );
        }
    }

    #[test]
    fn r4319000_extend_needs_a_seed_set() {
        let refused = series(&screen(Some("improve-try-3"), None, Some(60), false, true)).unwrap_err();
        assert!(refused.to_string().contains("--extend"), "{refused}");
        // With a set it behaves as today.
        assert_eq!(
            series(&screen(None, Some(SeedSet::ImproveDev), None, false, true)).unwrap(),
            ("improve-dev".to_string(), true, SCREEN_GAMES)
        );
    }

    fn fake_outcome(result: Option<GameResult>, a_seat: PlayerId, hash: &str, n: i32) -> ArenaOutcome {
        ArenaOutcome {
            game: ArenaGame {
                n,
                seed: format!("r4319000:{n}"),
                decks: (Vec::new(), Vec::new()),
                seats: PerPlayer::new(AgentSpec::SelfAi, AgentSpec::Random),
                skills: PerPlayer::new(AI_SKILL_MAX, AI_SKILL_MAX),
                a_seat,
                labels: ("self".to_string(), "random".to_string()),
                paired: true,
                turn_actions: None,
                stop_unsettled: false,
                self_budget: TEST_BUDGET,
            },
            result,
            log: Vec::new(),
            hash: hash.to_string(),
            turns: 0,
            rejected: Vec::new(),
            thrown: Vec::new(),
            fallbacks: PerPlayer::new(0, 0),
            sim_panics: PerPlayer::new(0, 0),
            first_sim_panic: None,
            times: PerPlayer::new(Vec::new(), Vec::new()),
        }
    }

    #[test]
    fn r4319000_decided_won_lost_and_diverged_come_from_edges_and_hashes() {
        let won = |seat: PlayerId| {
            Some(GameResult {
                winner: Winner::from(seat),
                reason: GameOverReason::HeroDeath,
            })
        };
        let draw = Some(GameResult {
            winner: Winner::Draw,
            reason: GameOverReason::TurnCap,
        });
        let outcomes = vec![
            // Both won on the same hash: decided and wonBoth, not diverged.
            fake_outcome(won(PlayerId::P1), PlayerId::P1, "same", 1),
            fake_outcome(won(PlayerId::P2), PlayerId::P2, "same", 2),
            // Split on different hashes: undecided, diverged.
            fake_outcome(won(PlayerId::P1), PlayerId::P1, "split-a", 3),
            fake_outcome(won(PlayerId::P1), PlayerId::P2, "split-b", 4),
            // A draw plus a win: decided (edge 0.5) but neither both-won nor both-lost,
            // diverged.
            fake_outcome(draw, PlayerId::P1, "draw-a", 5),
            fake_outcome(won(PlayerId::P2), PlayerId::P2, "draw-b", 6),
            // An aborted game plus a loss: decided (edge -0.5), diverged.
            fake_outcome(None, PlayerId::P1, "abort-a", 7),
            fake_outcome(won(PlayerId::P1), PlayerId::P2, "abort-b", 8),
            // Both lost on the same hash: decided and lostBoth, not diverged.
            fake_outcome(won(PlayerId::P2), PlayerId::P1, "lost", 9),
            fake_outcome(won(PlayerId::P1), PlayerId::P2, "lost", 10),
        ];
        let totals = tally("self", "random", &outcomes, true, true);
        let paired = totals.paired.as_ref().unwrap();
        assert_eq!(paired.deals, 5);
        assert_eq!((paired.decided, paired.won_both, paired.lost_both), (4, 1, 1));
        assert_eq!(paired.diverged, 3);
        assert_eq!(paired.inconclusive, Some(true));
    }

    #[test]
    fn r4319000_a_short_screen_calls_itself_inconclusive() {
        let won = |seat: PlayerId| {
            Some(GameResult {
                winner: Winner::from(seat),
                reason: GameOverReason::HeroDeath,
            })
        };
        let both_won = |n: i32| {
            vec![
                fake_outcome(won(PlayerId::P1), PlayerId::P1, "h", n),
                fake_outcome(won(PlayerId::P2), PlayerId::P2, "h", n + 1),
            ]
        };
        let mut full: Vec<ArenaOutcome> = Vec::new();
        for d in 0..20 {
            full.extend(both_won(2 * d + 1));
        }
        // Nineteen decided deals say nothing.
        let short = full[..38].to_vec();
        let json = serde_json::to_string(&tally("self", "random", &short, true, true)).unwrap();
        assert!(json.contains("\"inconclusive\":true"), "{json}");
        // The twentieth clears it.
        let json = serde_json::to_string(&tally("self", "random", &full, true, true)).unwrap();
        assert!(!json.contains("inconclusive"), "{json}");
        // A paired series that is not on a seed set calls itself nothing.
        let json = serde_json::to_string(&tally("self", "random", &short, true, false)).unwrap();
        assert!(!json.contains("inconclusive"), "{json}");
    }

    #[test]
    fn r4319000_extend_stops_on_the_decided_count_alone() {
        assert!(screen_done(SCREEN_MIN_DECIDED, SCREEN_GAMES));
        assert!(!screen_done(SCREEN_MIN_DECIDED - 1, SCREEN_GAMES));
        assert!(screen_done(0, SCREEN_MAX_GAMES));
        assert!(!screen_done(0, SCREEN_MAX_GAMES - 2));
        // Two 200-deal lists with the same decided deals but opposite winners: 19 decided in the
        // first 100-deal batch, the 20th in the second.
        let decided_at = [
            3, 11, 23, 37, 41, 55, 59, 63, 71, 77, 80, 83, 89, 90, 93, 95, 96, 98, 99, 150,
        ];
        for sign in [1.0, -1.0] {
            let mut edges = vec![0.0; 200];
            for d in decided_at {
                edges[d] = sign;
            }
            let (decided_first, _, _) = decided_of(&edges[..100]);
            let (decided_both, _, _) = decided_of(&edges);
            assert_eq!((decided_first, decided_both), (19, 20));
            assert!(!screen_done(decided_first, 200));
            assert!(screen_done(decided_both, 400));
        }
    }

    #[test]
    fn r747000_arena_rejects_skill_outside_unit_range() {
        jackioh_cards::register_all();
        for skill in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert!(
                entrant_at(&AgentSpec::SelfAi, skill).is_err(),
                "{skill} is outside [0, 1]"
            );
        }
        for skill in [0.0, 0.5, 1.0] {
            assert!(entrant_at(&AgentSpec::SelfAi, skill).is_ok(), "{skill}");
        }
        // Random takes no AI decision, so a skill short of full is a mistake, not a setting.
        assert!(entrant_at(&AgentSpec::Random, 0.5).is_err());
        assert!(entrant_at(&AgentSpec::Random, 1.0).is_ok());
    }

    #[test]
    fn r747000_entrant_label_names_nonfull_skill() {
        jackioh_cards::register_all();
        let skilled = entrant_at(&AgentSpec::SelfAi, 0.5).unwrap();
        assert_eq!(skilled.label, "self-s0.5");
        assert_eq!(skilled.skill, 0.5);
        assert_eq!(entrant_at(&AgentSpec::SelfAi, 0.25).unwrap().label, "self-s0.25");
        assert_eq!(entrant_at(&AgentSpec::SelfAi, 0.0).unwrap().label, "self-s0");
        let full = entrant_at(&AgentSpec::SelfAi, 1.0).unwrap();
        assert_eq!(full.label, "self");
        assert_eq!(full.skill, AI_SKILL_MAX);
        assert_eq!(entrant_at(&AgentSpec::Random, 1.0).unwrap().label, "random");

        // The label is in the record's id, so a skilled run's records are not the full-skill run's.
        let games = setups(&skilled, &full, 1, false, |n| format!("r747000-label:{n}")).unwrap();
        assert_eq!(
            record_id("0.2.0", &games[0]),
            "dev:0.2.0:arena:self-s0.5-vs-self:r747000-label:1"
        );
    }

    #[test]
    fn r747000_game_setup_carries_skills_to_seats() {
        jackioh_cards::register_all();
        let a = entrant_at(&AgentSpec::SelfAi, 0.25).unwrap();
        let b = entrant_at(&AgentSpec::SelfAi, 0.75).unwrap();
        let games = setups(&a, &b, 4, false, |n| format!("r747000-seats:{n}")).unwrap();
        for game in &games {
            assert_eq!(game.skills[game.a_seat], 0.25, "game {}", game.n);
            assert_eq!(game.skills[game.a_seat.opponent()], 0.75, "game {}", game.n);
        }
        // Odd games seat a on p1, even games on p2.
        assert_eq!(games[0].skills, PerPlayer::new(0.25, 0.75));
        assert_eq!(games[1].skills, PerPlayer::new(0.75, 0.25));

        // random keeps full skill whichever seat it sits.
        let random = entrant(&AgentSpec::Random).unwrap();
        let games = setups(&a, &random, 2, false, |n| format!("r747000-seats:{n}")).unwrap();
        assert_eq!(games[0].skills, PerPlayer::new(0.25, AI_SKILL_MAX));
        assert_eq!(games[1].skills, PerPlayer::new(AI_SKILL_MAX, 0.25));
    }

    #[test]
    fn r2214000_a_zero_skill_agent_plays_the_miss_and_replays() {
        jackioh_cards::register_all();
        let zero = entrant_at(&AgentSpec::SelfAi, 0.0).unwrap();
        let random = entrant(&AgentSpec::Random).unwrap();
        // The same shadow ban, so the same decks: only who decides differs.
        let skilled = setups(&zero, &random, 2, false, |n| format!("r2214000-zero:{n}")).unwrap();
        let plain = setups(&random, &random, 2, false, |n| format!("r2214000-zero:{n}")).unwrap();
        let first = play_games(&skilled).unwrap();
        let second = play_games(&skilled).unwrap();
        let plain = play_games(&plain).unwrap();
        for ((first, second), plain) in first.iter().zip(&second).zip(&plain) {
            // The miss draws on the seat's own stream, so the game reproduces and its log replays.
            assert_eq!(first.log, second.log, "game {}", first.game.n);
            assert!(replays(first).unwrap(), "game {}", first.game.n);
            assert!(first.result.is_some());
            assert_eq!(first.thrown, Vec::<String>::new());
            // A miss is not the random policy's move.
            assert_eq!(first.game.decks, plain.game.decks);
            assert_ne!(first.log, plain.log, "game {}", first.game.n);
        }
    }

    #[test]
    fn r747000_a_mid_skill_game_reproduces_and_replays() {
        jackioh_cards::register_all();
        let a = entrant_at(&AgentSpec::SelfAi, 0.5).unwrap();
        let b = entrant(&AgentSpec::Random).unwrap();
        let mut games = setups(&a, &b, 2, false, |n| format!("r747000-mid:{n}")).unwrap();
        for game in &mut games {
            game.self_budget = TEST_BUDGET;
        }
        let first = play_games(&games).unwrap();
        let second = play_games(&games).unwrap();
        for (first, second) in first.iter().zip(&second) {
            assert_eq!(first.hash, second.hash, "game {}", first.game.n);
            assert_eq!(first.log, second.log, "game {}", first.game.n);
            assert!(replays(first).unwrap(), "game {}", first.game.n);
            assert!(first.result.is_some());
            assert_eq!(first.thrown, Vec::<String>::new());
        }
    }

    /// R976000: an arena's two seats are dealt by one rule, whatever ban list their entrants carry,
    /// and neither is shaped by the unban lane's dealt-quality table.
    #[test]
    fn r976000_an_arenas_two_seats_are_dealt_by_one_rule_and_neither_is_shaped() {
        jackioh_cards::register_all();
        let candidate = self_entrant();
        let mut parent = self_entrant();
        parent.spec = AgentSpec::Bin(PathBuf::from("parent-jackioh"));
        parent.label = "bin-gen11".to_string();
        parent.shadow_ban.push("core-001".to_string());
        assert_ne!(candidate.shadow_ban, parent.shadow_ban);
        let easy = AI_DIFFICULTY.easy;
        let dealt = |seed: &str, seat: PlayerId, banned: Option<Vec<String>>, shape: bool| {
            build_ai_deck(
                &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
                easy.deck_size,
                &AiDeckOptions {
                    banned,
                    mana_cap: Some(easy.mana_cap),
                    shape,
                    ..Default::default()
                },
            )
        };
        let games = setups(&candidate, &parent, 8, false, |n| format!("r976000:{n}")).unwrap();
        let mut shaping_changed = 0;
        for game in &games {
            let deck_at = |seat: PlayerId| {
                if seat == PlayerId::P1 {
                    &game.decks.0
                } else {
                    &game.decks.1
                }
            };
            let (own, other) = (game.a_seat, game.a_seat.opponent());
            // The candidate's seat is dealt as the daily gates deal, which pass no ban list at all.
            assert_eq!(
                deck_at(own),
                &dealt(&game.seed, own, None, false),
                "game {}: the candidate's seat is not dealt unshaped",
                game.n
            );
            // The parent's seat, minus its own ban list and unshaped, by the same rule.
            assert_eq!(
                deck_at(other),
                &dealt(&game.seed, other, Some(parent.shadow_ban.clone()), false),
                "game {}: the parent's seat is not dealt unshaped",
                game.n
            );
            let shaped = dealt(&game.seed, own, Some(candidate.shadow_ban.clone()), true);
            shaping_changed += i32::from(&shaped != deck_at(own));
        }
        assert!(shaping_changed > 0, "shaping changed no candidate deal");
    }

    #[test]
    fn civil_dates_count_from_the_epoch() {
        let civil_date = |days: i64| utc_date_of(days * SECONDS_PER_DAY);
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(19_723), "2024-01-01");
        assert_eq!(civil_date(19_782), "2024-02-29");
        assert_eq!(civil_date(-1), "1969-12-31");
    }

    #[test]
    fn r1489000_a_self_seat_starved_of_budget_counts_its_fallbacks_in_the_outcome() {
        jackioh_cards::register_all();
        let mut game = setups(
            &self_entrant(),
            &entrant(&AgentSpec::Random).unwrap(),
            1,
            false,
            |n| format!("r1489000-starved:{n}"),
        )
        .unwrap()
        .remove(0);
        assert_eq!(game.self_budget, AI_GATE_BUDGET);
        game.self_budget = SearchBudget {
            nodes: 0,
            lethal_nodes: 0,
            ..AI_GATE_BUDGET
        };
        let outcome = play_game(&game).unwrap();
        let own = game.a_seat;
        assert!(outcome.fallbacks[own] > 0, "{:?}", outcome.fallbacks);
        assert_eq!(outcome.fallbacks[own.opponent()], 0);
        assert_eq!(outcome.sim_panics, PerPlayer::new(0, 0));
        assert_eq!(outcome.first_sim_panic, None);
        assert!(
            outcome
                .faults()
                .contains(&format!("{}: {} fallbacks", own.as_str(), outcome.fallbacks[own])),
            "{:?}",
            outcome.faults()
        );
        let line: Value = serde_json::from_str(&duel_line(&outcome, true).unwrap()).unwrap();
        assert_eq!(line["fallbacks"], json!(outcome.fallbacks[own]));
        assert_eq!(line["simPanics"], json!(0));
    }

    #[test]
    fn r1489000_a_bin_agents_verbose_answer_tells_its_fallback_and_its_search_panics() {
        assert_eq!(
            told_of(
                &json!({"action":{"type":"endTurn"},"rngCursor":3,"reason":"fallback","line":[],"stats":{"nodes":0,"simPanics":2}})
            ),
            (true, 2)
        );
        assert_eq!(
            told_of(
                &json!({"action":{"type":"endTurn"},"rngCursor":3,"reason":"search","line":[],"stats":{"nodes":1,"simPanics":0}})
            ),
            (false, 0)
        );
        assert_eq!(told_of(&json!({"action":null,"rngCursor":0})), (false, 0));
        assert_eq!(
            told_of(
                &json!({"action":{"type":"endTurn"},"rngCursor":3,"reason":"search","line":[],"stats":{"nodes":1}})
            ),
            (false, 0)
        );
    }
}
