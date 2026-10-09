//! `cargo jackioh arena`: the training arena (SURFACE §14.2; docs/v0.3.0/README.md §8). A referee
//! holds the true state and plays seeded games between two agents, each seeing only `redact(state,
//! seat)` (R185):
//!
//! - `self`: this build's AI (`agent::decide_with`, the same call `cargo jackioh agent` answers with);
//! - `random`: SPEC §10.7's random policy (`jackioh_ai::random_action`), the quality gates' baseline;
//! - `bin:<path>`: another build's AI, spawned as `<path> agent` and spoken to in JSON lines
//!   (SURFACE §14.1), which is how a promotion plays the AI on `main`.
//!
//! Game `n`'s seed is `<base>:<n>`; agent `a` sits p1 on odd games. Each seat's deck is
//! `build_ai_deck` over all three sets minus its own agent's shadow ban (R186), with no handicap
//! (Easy both, R180). A refused action is replaced by endTurn or the first legal answer; a
//! controller that throws ends the game without a result. So a game is a function of its seed and
//! its two agents, whatever the thread count.
//!
//! Every finished game leaves one `GameRecord` (R376; `source: "dev"`, `mode: "random"`, R378) on a
//! JSONL line: in `<--out>/<date>.jsonl`, else `$JACKIOH_TRAINING_OUT/<date>.jsonl`, else on stdout.
//! The duel lines go to stdout when the records went to a file and to stderr when they took stdout;
//! a tally closes them.

use std::io::{BufRead, BufReader, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow, bail};
use jackioh_ai::{AiDeckOptions, RejectedAction, build_ai_deck, random_action, redact};
use jackioh_engine::{
    AI_DIFFICULTY, Action, ActionBody, CreateGameOptions, DEV_RECORD_ID_PREFIX, FoldArgs, GameRecord,
    GameResult, GameState, PerPlayer, PlayerId, Rng, Winner, begin_game, create_game, fold, hash_state,
    legal_actions, reduce, seat_to_act, subsystems, summarize_game,
};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};

use crate::agent::{self, AgentInfo};
use crate::gate::MS_PER_SECOND;
use crate::patches::utc_date_of;

/// The referee's ceiling on actions in one game; the engine's turn cap (R2) ends every real game first.
const ARENA_MAX_ACTIONS: usize = 3000;

/// The directory game records go to when `--out` names none (docs/v0.3.0/README.md §8).
pub(crate) const TRAINING_OUT_ENV: &str = "JACKIOH_TRAINING_OUT";

const DUEL_PERCENTILE: f64 = 0.95;

#[derive(clap::Args)]
pub struct Args {
    /// Agent a: self | random | bin:<path>. It sits p1 on odd games and p2 on even ones.
    #[arg(long, value_parser = parse_agent)]
    pub a: AgentSpec,
    /// Agent b: self | random | bin:<path>.
    #[arg(long, value_parser = parse_agent)]
    pub b: AgentSpec,
    /// Games to play: 1..=N.
    #[arg(long)]
    pub games: i32,
    /// The seed base: game n's seed is `<base>:<n>`.
    #[arg(long)]
    pub seed: String,
    /// The directory the game records go to (`<dir>/<date>.jsonl`); default `$JACKIOH_TRAINING_OUT`,
    /// else stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
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
            })
        }
        AgentSpec::Random => Ok(Entrant {
            spec: spec.clone(),
            label: "random".to_string(),
            generation: None,
            lane: None,
            shadow_ban: agent::own_info().shadow_ban,
        }),
        AgentSpec::Bin(path) => {
            let info = query_info(path)?;
            Ok(Entrant {
                spec: spec.clone(),
                label: format!("bin-gen{}", info.generation),
                generation: Some(info.generation),
                lane: Some(info.lane),
                shadow_ban: info.shadow_ban,
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
    /// Where agent `a` sits.
    pub a_seat: PlayerId,
    /// The agents' labels, a's then b's, for the records.
    pub labels: (String, String),
}

/// What a game came to: match.ts's `MatchRecord` minus search statistics, plus each seat's decision times.
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
    /// Agent answers of "no move", replaced by the random policy on the seat's stream.
    pub fallbacks: i32,
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
}

pub(crate) fn won_by(result: Option<GameResult>, seat: PlayerId) -> bool {
    matches!(result, Some(result) if result.winner == Winner::from(seat))
}

/// Games 1..=games between `a` and `b`. Decks are built here, before any game, so an unbuildable deck fails the run.
pub(crate) fn setups(
    a: &Entrant,
    b: &Entrant,
    games: i32,
    seed_of: impl Fn(i32) -> String,
) -> anyhow::Result<Vec<ArenaGame>> {
    let mut out = Vec::new();
    for n in 1..=games {
        out.push(game_setup(seed_of(n), n, a, b)?);
    }
    Ok(out)
}

/// Game `n` on `seed`: agent `a` sits p1 on odd `n`, and each seat's deck omits its own agent's shadow ban.
pub(crate) fn game_setup(seed: String, n: i32, a: &Entrant, b: &Entrant) -> anyhow::Result<ArenaGame> {
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
        a_seat,
        labels: (a.label.clone(), b.label.clone()),
    })
}

/// A seat's deck: `buildAiDeck` seeded `<seed>:deck:<seat>` at Easy's resources (R180: no handicap),
/// over all three sets (R184, R380).
fn deck_for(seed: &str, seat: PlayerId, entrant: &Entrant) -> anyhow::Result<Vec<String>> {
    let handicap = AI_DIFFICULTY.easy;
    let mut rng = Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0);
    let options = AiDeckOptions {
        banned: Some(entrant.shadow_ban.clone()),
        mana_cap: Some(handicap.mana_cap),
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

enum Live {
    SelfAi,
    Random,
    Bin(BinAgent),
}

impl Live {
    fn start(spec: &AgentSpec) -> anyhow::Result<Live> {
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

    fn finish(self) {
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

    /// The agent's move on `redact(state, seat)`; the stream moves to the cursor the agent answers with.
    fn decide(
        &mut self,
        state: &GameState,
        seat: PlayerId,
        rng_seed: &str,
        rng: &mut Rng,
    ) -> Result<Option<ActionBody>, String> {
        let view = redact(state, seat);
        let request = DecideRequest {
            op: "decide",
            state: &view,
            seat,
            rng_seed,
            rng_cursor: rng.cursor(),
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
        Ok(action)
    }
}

/// One controller call (match.ts's `chooseFor`): the agent's action, `None` for "no move", or what it
/// threw. `self` decides on `redact(state, seat)`; `random` plays on the referee's state.
fn choose_for(
    live: &mut Live,
    state: &GameState,
    seat: PlayerId,
    rng_seed: &str,
    rng: &mut Rng,
) -> Result<Option<ActionBody>, String> {
    match live {
        Live::SelfAi => {
            let view = redact(state, seat);
            let start = rng.clone();
            let (action, next) = catch_unwind(AssertUnwindSafe(|| agent::decide_with(&view, seat, start)))
                .map_err(|panic| agent::panic_message(&*panic))?;
            *rng = next;
            Ok(action)
        }
        Live::Random => catch_unwind(AssertUnwindSafe(|| random_action(state, seat, rng)))
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
fn accept(
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

/// Plays one game (match.ts's `playMatch`): the actor is `seat_to_act` (R265), its stream
/// `createRng("<seed>:ctl:<seat>")`, the nonce `m<log length>`. "No move" is replaced by the random
/// policy; a refused action is recorded in `rejected` and replaced; a throw ends the game.
pub(crate) fn play_game(game: &ArenaGame) -> anyhow::Result<ArenaOutcome> {
    let options = CreateGameOptions {
        seed: game.seed.clone(),
        decks: game.decks.clone(),
        ..Default::default()
    };
    let mut state =
        catch_unwind(AssertUnwindSafe(|| begin_game(&create_game(&options)).state)).map_err(|panic| {
            anyhow!(
                "{}: the game cannot start: {}",
                game.seed,
                agent::panic_message(&*panic)
            )
        })?;

    let ctl = PerPlayer::new(format!("{}:ctl:p1", game.seed), format!("{}:ctl:p2", game.seed));
    let mut rngs = PerPlayer::new(Rng::new(&ctl.p1, 0), Rng::new(&ctl.p2, 0));
    let mut live = PerPlayer::new(Live::start(&game.seats.p1)?, Live::start(&game.seats.p2)?);

    let mut log: Vec<Action> = Vec::new();
    let mut rejected: Vec<RejectedAction> = Vec::new();
    let mut thrown: Vec<String> = Vec::new();
    let mut fallbacks = 0;
    let mut times: PerPlayer<Vec<f64>> = PerPlayer::new(Vec::new(), Vec::new());

    while state.result.is_none() && log.len() < ARENA_MAX_ACTIONS {
        let Some(seat) = seat_to_act(&state) else {
            thrown.push(format!(
                "no seat to act while the game is live (turn {})",
                state.turn
            ));
            break;
        };

        let started = Instant::now();
        let choice = choose_for(&mut live[seat], &state, seat, &ctl[seat], &mut rngs[seat]);
        times[seat].push(started.elapsed().as_secs_f64() * MS_PER_SECOND);
        let chosen = match choice {
            Ok(chosen) => chosen,
            Err(message) => {
                thrown.push(format!(
                    "{}: controller {} threw: {message}",
                    seat.as_str(),
                    live[seat].label()
                ));
                break;
            }
        };

        let chosen = match chosen {
            Some(chosen) => chosen,
            None => {
                fallbacks += 1;
                match random_action(&state, seat, &mut rngs[seat]) {
                    Some(chosen) => chosen,
                    None => {
                        thrown.push(format!(
                            "no action for {} while the game is live (turn {})",
                            seat.as_str(),
                            state.turn
                        ));
                        break;
                    }
                }
            }
        };

        let nonce = format!("m{}", log.len());
        let current = &state;
        let step = catch_unwind(AssertUnwindSafe(|| {
            accept(current, seat, &chosen, &nonce, &mut rejected)
        }));
        let accepted = match step {
            Ok(accepted) => accepted,
            Err(panic) => {
                thrown.push(format!(
                    "{}: reduce threw on \"{}\": {}",
                    seat.as_str(),
                    chosen.action_type().as_str(),
                    agent::panic_message(&*panic)
                ));
                break;
            }
        };
        let Some((action, next)) = accepted else {
            thrown.push(format!(
                "{}: no legal replacement for a refused \"{}\" (turn {})",
                seat.as_str(),
                chosen.action_type().as_str(),
                state.turn
            ));
            break;
        };

        log.push(action);
        state = next;
    }

    let PerPlayer { p1, p2 } = live;
    p1.finish();
    p2.finish();

    Ok(ArenaOutcome {
        game: game.clone(),
        result: state.result,
        hash: hash_state(&state),
        turns: state.turn,
        log,
        rejected,
        thrown,
        fallbacks,
        times,
    })
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
/// games sharing seeds against random and against the parent are separate records.
pub(crate) fn record_id(patch: &str, game: &ArenaGame) -> String {
    format!(
        "{DEV_RECORD_ID_PREFIX}{patch}:arena:{}-vs-{}:{}",
        game.labels.0, game.labels.1, game.seed
    )
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
    Ok(Some(serde_json::from_value(record).context(
        "arena: a game record does not parse as a GameRecord",
    )?))
}

/// Where the records go: `<out>/<date>.jsonl`, else `$JACKIOH_TRAINING_OUT/<date>.jsonl`, else `None`.
pub(crate) fn records_path(out: Option<&Path>) -> Option<PathBuf> {
    let dir = match out {
        Some(dir) => dir.to_path_buf(),
        None => match std::env::var_os(TRAINING_OUT_ENV) {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => return None,
        },
    };
    Some(dir.join(format!("{}.jsonl", utc_date())))
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
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    file.write_all(text.as_bytes())
        .with_context(|| format!("arena: cannot write {}", path.display()))?;
    Ok(())
}

pub(crate) fn utc_date() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    utc_date_of(i64::try_from(seconds).unwrap_or(i64::MAX))
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
    replay_ok: bool,
    decisions: usize,
    ms_sum: i64,
    ms_max: i64,
    ms_p95: i64,
}

fn round_ms(ms: f64) -> i64 {
    (ms + 0.5).floor() as i64
}

/// duel.ts's per-game line: outcome from a's side, `aborted` (no result; no gate counts it a draw),
/// why it ended, hash, whether it replays, and a's decision count and times.
fn duel_line(outcome: &ArenaOutcome, replay_ok: bool) -> anyhow::Result<String> {
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
        fallbacks: outcome.fallbacks,
        replay_ok,
        decisions: mine.len(),
        ms_sum: round_ms(mine.iter().sum()),
        ms_max: round_ms(sorted.last().copied().unwrap_or(0.0)),
        ms_p95: round_ms(sorted.get(p95_at).copied().unwrap_or(0.0)),
    };
    Ok(serde_json::to_string(&line)?)
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Tally {
    pub a: String,
    pub b: String,
    pub games: usize,
    pub a_wins: usize,
    pub b_wins: usize,
    pub draws: usize,
    pub aborted: usize,
}

pub(crate) fn tally(a: &str, b: &str, outcomes: &[ArenaOutcome]) -> Tally {
    Tally {
        a: a.to_string(),
        b: b.to_string(),
        games: outcomes.len(),
        a_wins: outcomes.iter().filter(|outcome| outcome.a_won()).count(),
        b_wins: outcomes
            .iter()
            .filter(|outcome| won_by(outcome.result, outcome.game.a_seat.opponent()))
            .count(),
        draws: outcomes.iter().filter(|outcome| outcome.drawn()).count(),
        aborted: outcomes.iter().filter(|outcome| outcome.result.is_none()).count(),
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
    jackioh_cards::register_all();
    if args.games < 1 {
        bail!("arena: --games must be at least 1, not {}", args.games);
    }
    let a = entrant(&args.a)?;
    let b = entrant(&args.b)?;
    let base = args.seed.clone();
    let games = setups(&a, &b, args.games, |n| format!("{base}:{n}"))?;
    let outcomes = play_games(&games)?;
    let records = records_of(&outcomes)?;

    let path = records_path(args.out.as_deref());
    let mut lines = Vec::new();
    for outcome in &outcomes {
        lines.push(duel_line(outcome, replays(outcome)?)?);
    }
    let totals = serde_json::to_string(&tally(&a.label, &b.label, &outcomes))?;

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
                writeln!(stdout, "{}", serde_json::to_string(record)?)?;
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
        let games = setups(&a, &b, 4, |n| format!("arena-test:{n}")).unwrap();
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
    fn two_self_agents_over_four_games_are_deterministic() {
        jackioh_cards::register_all();
        let a = self_entrant();
        let b = self_entrant();
        let games = setups(&a, &b, 4, |n| format!("arena-determinism:{n}")).unwrap();
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
    fn record_ids_name_the_agents_and_the_seed() {
        let game = ArenaGame {
            n: 1,
            seed: "improve:abc:1".to_string(),
            decks: (Vec::new(), Vec::new()),
            seats: PerPlayer::new(AgentSpec::SelfAi, AgentSpec::Random),
            a_seat: PlayerId::P1,
            labels: ("self".to_string(), "random".to_string()),
        };
        assert_eq!(
            record_id("0.2.0", &game),
            "dev:0.2.0:arena:self-vs-random:improve:abc:1"
        );
    }

    #[test]
    fn civil_dates_count_from_the_epoch() {
        let civil_date = |days: i64| utc_date_of(days * SECONDS_PER_DAY);
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(19_723), "2024-01-01");
        assert_eq!(civil_date(19_782), "2024-02-29");
        assert_eq!(civil_date(-1), "1969-12-31");
    }
}
