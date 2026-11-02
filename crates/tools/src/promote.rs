//! `cargo jackioh promote`: the promotion gate of the AI training lanes (SURFACE §14.2;
//! docs/v0.3.0/README.md §8; training/README.md).
//!
//! ```text
//! cargo jackioh promote --lane improve|unban|learn --parent-bin <path to main's jackioh> [--dry-run] [--verify]
//! ```
//!
//! The candidate, this build's AI (`self`), plays deals against the parent, the AI on `main`
//! (`bin:<parent-bin>`, SURFACE §14.1), each twice with the seats swapped (R850000). Deal `d`'s seed is
//! `"<lane>:<tree>:<d>"`, `<tree>` the git tree of `crates/ai/src`, so committing `generation.json`
//! moves no seed. Those deals are the lane's hold-out set (R850001), read only by the committed AI they
//! are named for (`dirty_refusal`) and never by a screen. Each seat's deck is `build_ai_deck` over every
//! set that ships minus its own AI's shadow ban (R186), Easy both (R180), neither shaped (R976000). No
//! result depends on rayon's thread count.
//!
//! It promotes when the lane's sequential test passes (R1679000, `sprt`), every game it counts is clean
//! on either seat (R1489000, `unclean_games`) and the lane's own rule holds (`Lane`; `learn` is #1012's,
//! R1012000). It plays no games against SPEC §10.7's random policy (R732001), which generation 9, the
//! first AI dealt the Meditative set (#732), beat; the daily `gate --full` still holds `main` to it.
//! Without `--dry-run` or `--verify`, a pass writes `crates/ai/generation.json` and appends it to
//! `training/history/<lane>.jsonl`; `--verify` (CI) recomputes and compares with the branch's. The
//! report goes to stdout; exit 0 on a pass, 1 on a failed gate.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::ValueEnum;
use jackioh_engine::{GameResult, PlayerId, TRAINING_GATE, TrainingGate, Winner};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::arena::{self, AgentSpec, ArenaOutcome, Entrant};
use crate::patches::{git, repo_root, utc_date_of};

/// A share as a percentage, for the report.
const PERCENT: f64 = 100.0;

/// The AI's source, whose git tree names the seeds.
pub(crate) const AI_SOURCE: &str = "crates/ai/src";

/// The main AI's generation record (SURFACE §14.3), relative to the repository root.
pub(crate) const GENERATION_PATH: &str = "crates/ai/generation.json";

/// The file in `$JACKIOH_TRAINING_OUT` that every run of the gate rewrites with its pass: the top of
/// the lane's draft pull request (training/loop.sh).
pub(crate) const LAST_PASS_FILE: &str = "last-pass.md";

/// Games per row of the latest pass's grid.
const PASS_GRID_ROW: usize = 10;

/// Seconds in a day, an hour and a minute, for the time of a pass.
const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;

/// The fields `--verify` holds a branch's `generation.json` to. The date is when the lane ran, and
/// the parent commit is the `main` it ran against, which may have moved on since without its AI
/// moving; every number and every name of what was measured must match.
const VERIFIED_FIELDS: &[&str] = &[
    "generation",
    "lane",
    "tree",
    "vsRandom",
    "vsParent",
    "shadowBan",
    "parentShadowBan",
];

/// The training lanes (docs/v0.3.0/README.md §8; `learn`, #1012).
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// Beat the parent: the sequential test on paired deals passes (R850000, R1679000).
    Improve,
    /// Play well with fewer shadow bans: strictly fewer than the parent's, and the test passes.
    /// Idle while the parent has none (`lane_done`).
    Unban,
    /// The learned evaluation, retrained by a script (R1012000): improve's test on its own deals.
    Learn,
}

impl Lane {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Lane::Improve => "improve",
            Lane::Unban => "unban",
            Lane::Learn => "learn",
        }
    }

    /// The lane's gate (CLAUDE.md rule 9: the numbers live in `crates/engine/src/config.rs`): every
    /// lane's is `TRAINING_GATE` (R1679000).
    pub(crate) fn gate(self) -> TrainingGate {
        TRAINING_GATE
    }
}

#[derive(clap::Args)]
pub struct Args {
    /// The lane whose gate to apply.
    #[arg(long, value_enum)]
    pub lane: Lane,
    /// The parent: a `jackioh` binary built from `main` (training/loop.sh keeps it at ~/parent-jackioh).
    #[arg(long)]
    pub parent_bin: PathBuf,
    /// Measure and report; write nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// Recompute and compare with the branch's `crates/ai/generation.json`; write nothing (CI).
    #[arg(long)]
    pub verify: bool,
}

/// What a promotion run counted: the games of the deals the test read, and its verdict.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GateCounts {
    pub games: i32,
    pub vs_parent: i32,
    pub shadow_ban: usize,
    pub parent_shadow_ban: usize,
    pub test: Sprt,
}

/// R1679000: where the sequential test stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Its ratio reached the upper bound: the candidate promotes.
    Pass,
    /// It fell to the lower bound: the candidate is no stronger, or not stronger enough.
    Fail,
    /// It read `max_deals` without leaving its bounds: no evidence either way, which fails.
    Capped,
    /// It has fewer deals than it needs and is inside its bounds: play more.
    Open,
}

/// R1679000: the sequential test as it stands after the deals it read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sprt {
    /// The deals it read: up to the one it stopped at, or every one it was given.
    pub deals: usize,
    /// The log-likelihood ratio of H1 to H0 after them.
    pub llr: f64,
    pub verdict: Verdict,
}

/// R1679000: the test's bounds on the log-likelihood ratio, `ln(β / (1 − α))` below (a fail) and
/// `ln((1 − β) / α)` above (a pass): Wald's, for errors of `alpha` and `beta`.
pub(crate) fn sprt_bounds(gate: &TrainingGate) -> (f64, f64) {
    (
        (gate.beta / (1.0 - gate.alpha)).ln(),
        ((1.0 - gate.beta) / gate.alpha).ln(),
    )
}

/// R1679000: the test over `edges`, each deal's edge for the candidate in deal order
/// (`arena::deal_edges`). After `n` deals of edges summing to `S`, the ratio is that of two normal laws
/// with the deals' own variance `σ²` (never below `variance_floor`), means `edge_h1` and `edge_h0`:
/// `(edge_h1 − edge_h0) · (S − n · (edge_h0 + edge_h1) / 2) / σ²`. From deal `min_deals` on it stops at
/// the first deal whose ratio leaves the bounds (`sprt_bounds`); at none, it is `Capped` once it has read
/// `max_deals` and `Open` before. Deals past `max_deals` are never read. The sums run in deal order, so
/// the same edges give the same ratio to the bit.
pub(crate) fn sprt(gate: &TrainingGate, edges: &[f64]) -> Sprt {
    let (lower, upper) = sprt_bounds(gate);
    let cap = usize::try_from(gate.max_deals).unwrap_or(0);
    let first = usize::try_from(gate.min_deals).unwrap_or(0);
    let (mut sum, mut squares, mut llr) = (0.0, 0.0, 0.0);
    for (read, edge) in edges.iter().take(cap).enumerate() {
        let deals = read + 1;
        sum += edge;
        squares += edge * edge;
        let n = deals as f64;
        let mean = sum / n;
        let variance = (squares / n - mean * mean).max(gate.variance_floor);
        llr = (gate.edge_h1 - gate.edge_h0) * (sum - n * (gate.edge_h0 + gate.edge_h1) / 2.0) / variance;
        if deals >= first {
            if llr >= upper {
                return Sprt {
                    deals,
                    llr,
                    verdict: Verdict::Pass,
                };
            }
            if llr <= lower {
                return Sprt {
                    deals,
                    llr,
                    verdict: Verdict::Fail,
                };
            }
        }
    }
    let deals = edges.len().min(cap);
    let verdict = if deals >= cap {
        Verdict::Capped
    } else {
        Verdict::Open
    };
    Sprt { deals, llr, verdict }
}

/// The rules of the lane's gate that `counts` fails, in the table's order; empty when it promotes.
pub(crate) fn gate_failures(lane: Lane, counts: &GateCounts) -> Vec<String> {
    let gate = lane.gate();
    let (lower, upper) = sprt_bounds(&gate);
    let test = &counts.test;
    let mut failures = Vec::new();
    match test.verdict {
        Verdict::Pass => {}
        Verdict::Fail => failures.push(format!(
            "vs parent: the sequential test fails it after {} deals ({}/{} wins): its log-likelihood \
             ratio {:.2} reached {lower:.2}",
            test.deals, counts.vs_parent, counts.games, test.llr
        )),
        Verdict::Capped => failures.push(format!(
            "vs parent: the sequential test is undecided after {} deals, the most it plays ({}/{} \
             wins): its log-likelihood ratio {:.2} is between {lower:.2} and {upper:.2}",
            test.deals, counts.vs_parent, counts.games, test.llr
        )),
        Verdict::Open => failures.push(format!(
            "vs parent: the sequential test has read {} deals, fewer than it needs ({}/{} wins)",
            test.deals, counts.vs_parent, counts.games
        )),
    }
    if lane == Lane::Unban && counts.shadow_ban >= counts.parent_shadow_ban {
        failures.push(format!(
            "shadow bans: {}, which is not strictly fewer than the parent's {}",
            counts.shadow_ban, counts.parent_shadow_ban
        ));
    }
    failures
}

/// The refusal to give when the lane has nothing to do, else `None`: the unban lane while its parent
/// keeps no card out of its own decks, since no candidate has strictly fewer shadow bans than none, so
/// `run` plays no games (#668). It reads the parent's list, not this build's `SHADOW_BAN`: a candidate
/// with none against a parent with one is the promotion that emptied the table (generation 8's).
pub(crate) fn lane_done(lane: Lane, parent: &Entrant) -> Option<String> {
    (lane == Lane::Unban && parent.shadow_ban.is_empty()).then(|| {
        format!(
            "promote: the shadow ban list is empty, so the unban lane has nothing to do: the parent ({}) \
             keeps no card out of its decks, no candidate can have strictly fewer bans than none, and \
             the gate cannot pass. No game was played and nothing was written (training/unban.md).",
            parent.label
        )
    })
}

/// The candidate's wins over games given as (result, the candidate's seat). A draw is not a win, and
/// a game without a result (the action ceiling, a controller that threw) is nobody's.
pub(crate) fn wins_of(games: &[(Option<GameResult>, PlayerId)]) -> i32 {
    games
        .iter()
        .filter(|(result, seat)| arena::won_by(*result, *seat))
        .count() as i32
}

/// The (result, candidate seat) pairs of a series in which the candidate is agent `a`.
fn results_of(outcomes: &[ArenaOutcome]) -> Vec<(Option<GameResult>, PlayerId)> {
    outcomes
        .iter()
        .map(|outcome| (outcome.result, outcome.game.a_seat))
        .collect()
}

/// R1489000: one line per game that is not clean, as the quality gates hold theirs (B31): its seed,
/// the candidate's seat, and what it showed: `ArenaOutcome::faults` on either seat, any throw, and
/// a log that does not fold back to its hash. `replayed` holds each game's `arena::replays`.
pub(crate) fn unclean_games(outcomes: &[ArenaOutcome], replayed: &[bool]) -> Vec<String> {
    outcomes
        .iter()
        .zip(replayed)
        .filter_map(|(outcome, &folds_back)| {
            let mut shown = outcome.faults();
            shown.extend(outcome.thrown.iter().cloned());
            if !folds_back {
                shown.push("its log does not fold back to its hash".to_string());
            }
            (!shown.is_empty()).then(|| {
                format!(
                    "unclean game {} (the candidate {}): {}",
                    outcome.game.seed,
                    outcome.game.a_seat.as_str(),
                    shown.join("; ")
                )
            })
        })
        .collect()
}

/// Every rule the run fails: the lane's (`gate_failures`), then each game that is not clean
/// (`unclean_games`, R1489000). Empty when it promotes.
pub(crate) fn promotion_failures(
    lane: Lane,
    counts: &GateCounts,
    outcomes: &[ArenaOutcome],
    replayed: &[bool],
) -> Vec<String> {
    let mut failures = gate_failures(lane, counts);
    failures.extend(unclean_games(outcomes, replayed));
    failures
}

/// Game `k`'s seed, or a paired gate's deal `k`'s: `"<lane>:<tree>:<k>"`.
pub(crate) fn game_seed(lane: Lane, tree: &str, k: i32) -> String {
    format!("{}:{tree}:{k}", lane.as_str())
}

/// R850001: whether an arena seed base would play a training gate's seeds, its lane's name alone or
/// followed by `:`. Those are the gate's hold-out deals, which no screen plays.
pub(crate) fn is_gate_seed_base(base: &str) -> bool {
    Lane::value_variants().iter().any(|lane| {
        base.strip_prefix(lane.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(':'))
    })
}

/// Why a run in `mode` refuses a `crates/ai/src` with uncommitted changes: a promotion or a
/// verification would measure them on another AI's seeds, and a dry run would read the lane's
/// hold-out deals with an AI they are not named for (R850001).
pub(crate) fn dirty_refusal(lane: Lane, mode: &str) -> String {
    if mode != "dry run" {
        return format!(
            "promote: {AI_SOURCE} has uncommitted changes. The gate's seeds come from `git rev-parse \
             HEAD:{AI_SOURCE}`, so commit the change first, then run the {mode} again (training/README.md)."
        );
    }
    format!(
        "promote: {AI_SOURCE} has uncommitted changes, and the {} lane's gate plays hold-out deals \
         that only the committed AI they are named for reads (R850001). Screen the change on the \
         dev set (`cargo jackioh arena --set improve-dev`), or commit it to the stack and run the \
         gate on the commit (training/improve.md).",
        lane.as_str()
    )
}

/// SURFACE §14.2's `<tree>`: `git rev-parse HEAD:crates/ai/src`.
pub(crate) fn ai_tree(repo: &Path) -> anyhow::Result<String> {
    Ok(git(repo, &["rev-parse", &format!("HEAD:{AI_SOURCE}")], &[])?
        .trim()
        .to_string())
}

/// Whether `crates/ai/src` differs from HEAD (staged, unstaged or untracked).
fn ai_source_dirty(repo: &Path) -> anyhow::Result<bool> {
    Ok(!git(repo, &["status", "--porcelain", "--", AI_SOURCE], &[])?
        .trim()
        .is_empty())
}

/// The `main` commit the parent is: the branch's merge base with `origin/main` (a lane's branch is
/// cut from it and rebased on it), else with `main`; `None` when neither is known.
fn parent_commit(repo: &Path) -> Option<String> {
    ["origin/main", "main"].into_iter().find_map(|base| {
        git(repo, &["merge-base", "HEAD", base], &[])
            .ok()
            .map(|commit| commit.trim().to_string())
    })
}

/// A promotion's record: `crates/ai/generation.json` and its history line, in SURFACE §14.2's key order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerationRecord {
    pub generation: i64,
    pub lane: String,
    /// The `main` commit the parent was built from.
    pub parent: Option<String>,
    /// `git rev-parse HEAD:crates/ai/src`, the seeds' name.
    pub tree: String,
    /// `"<wins>/<games>"` against random. `None` (`null`) since R732001: the lanes play no random
    /// games. Generation 9's record, and every promotion before it, holds a count.
    pub vs_random: Option<String>,
    /// `"<wins>/<games>"` against the parent.
    pub vs_parent: String,
    pub shadow_ban: usize,
    pub parent_shadow_ban: usize,
    /// The UTC date of the run, `YYYY-MM-DD`.
    pub date: String,
}

/// The fields of `VERIFIED_FIELDS` on which a branch's `generation.json` and a recomputed record
/// disagree, each as one line.
pub(crate) fn verify_mismatches(claimed: &Value, measured: &GenerationRecord) -> Vec<String> {
    let measured = serde_json::to_value(measured).unwrap_or(Value::Null);
    VERIFIED_FIELDS
        .iter()
        .filter_map(|key| {
            let said = claimed.get(*key).unwrap_or(&Value::Null);
            let found = measured.get(*key).unwrap_or(&Value::Null);
            (said != found).then(|| format!("{key}: generation.json says {said}, the gate measured {found}"))
        })
        .collect()
}

/// Appends the record to the lane's history. A last line of the same generation and lane (the
/// promotion run again, say after a rebase) is replaced, so one generation is one line.
fn write_history(path: &Path, record: &GenerationRecord) -> anyhow::Result<()> {
    let line = serde_json::to_string(record)?;
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().filter(|line| !line.trim().is_empty()).collect();
    if let Some(last) = lines.last()
        && let Ok(previous) = serde_json::from_str::<Value>(last)
        && previous.get("generation") == Some(&json!(record.generation))
        && previous.get("lane") == Some(&json!(record.lane))
    {
        lines.pop();
    }
    lines.push(&line);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("promote: cannot create {}", dir.display()))?;
    }
    std::fs::write(path, format!("{}\n", lines.join("\n")))
        .with_context(|| format!("promote: cannot write {}", path.display()))
}

/// Writes `crates/ai/generation.json`, pretty-printed as generation 0's was.
fn write_generation(path: &Path, record: &GenerationRecord) -> anyhow::Result<()> {
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(record)?))
        .with_context(|| format!("promote: cannot write {}", path.display()))
}

/// Draws and games without a result in a series, for the report.
fn draws_and_aborted(outcomes: &[ArenaOutcome]) -> (usize, usize) {
    (
        outcomes.iter().filter(|outcome| outcome.drawn()).count(),
        outcomes.iter().filter(|outcome| outcome.result.is_none()).count(),
    )
}

/// A win-rate interval as whole percentages, `55–74%`.
fn interval_text(low: f64, high: f64) -> String {
    format!("{:.0}–{:.0}%", low * PERCENT, high * PERCENT)
}

/// The sentence that names a pass's `games` seeds and seats, for the report and the latest pass.
fn seeds_text(lane: Lane, tree: &str, games: i32) -> String {
    format!(
        "Deals `{}` to `{}`, each played twice with the seats swapped: the candidate sits p1 in a \
         deal's first game (the odd games) and p2 in its second.",
        game_seed(lane, tree, 1),
        game_seed(lane, tree, arena::deal_of(games))
    )
}

/// R850000: the gate's edge, the candidate's wins less the parent's over each deal's two games, in
/// points of the games, with its 95% interval. `games` are (result, the candidate's seat) in game order.
fn paired_text(games: &[(Option<GameResult>, PlayerId)]) -> String {
    let (edge, low, high) = arena::paired_interval(&arena::deal_edges(games));
    format!(
        "Paired by deal, the candidate's wins less the parent's come to {:+.1} points of the games \
         (95% interval {:+.1} to {:+.1}).",
        edge * PERCENT,
        low * PERCENT,
        high * PERCENT
    )
}

/// R1679000: the sentence on the sequential test: its hypotheses and errors, the deals it read, its
/// ratio against its bounds, and what that says.
fn test_text(gate: &TrainingGate, test: &Sprt) -> String {
    let (lower, upper) = sprt_bounds(gate);
    let said = match test.verdict {
        Verdict::Pass => "it passes",
        Verdict::Fail => "it fails",
        Verdict::Capped => "undecided at the most deals it plays, it fails",
        Verdict::Open => "it needs more deals",
    };
    format!(
        "The sequential test (R1679000) weighs an edge of {:+.1} points against {:+.1}, with a {:.0}% \
         chance of promoting the first and {:.0}% of failing the second, over {} to {} deals: after {} \
         deals its log-likelihood ratio is {:.2}, against bounds of {lower:.2} and {upper:.2}, so {said}.",
        gate.edge_h0 * PERCENT,
        gate.edge_h1 * PERCENT,
        gate.alpha * PERCENT,
        gate.beta * PERCENT,
        gate.min_deals,
        gate.max_deals,
        test.deals,
        test.llr
    )
}

/// The parent as the report names it: its generation and lane, and the `main` commit it was built from.
fn parent_name(parent: &Entrant, record: &GenerationRecord) -> String {
    match (&parent.lane, &record.parent) {
        (Some(parent_lane), Some(commit)) => format!(
            "gen {} ({parent_lane}), `main` at `{commit}`",
            parent.generation.unwrap_or_default()
        ),
        (Some(parent_lane), None) => format!("gen {} ({parent_lane})", parent.generation.unwrap_or_default()),
        _ => parent.label.clone(),
    }
}

/// How one game of a pass went for the candidate. A draw is not a win, and neither is a game
/// without a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Won,
    Lost,
    Drawn,
    NoResult,
}

impl Mark {
    fn of(result: Option<GameResult>, seat: PlayerId) -> Mark {
        match result {
            None => Mark::NoResult,
            Some(ended) if ended.winner == Winner::Draw => Mark::Drawn,
            Some(_) if arena::won_by(result, seat) => Mark::Won,
            Some(_) => Mark::Lost,
        }
    }

    /// The game's square in the pass's grid.
    fn square(self) -> &'static str {
        match self {
            Mark::Won => "🟩",
            Mark::Lost => "🟥",
            Mark::Drawn => "⬜",
            Mark::NoResult => "⬛",
        }
    }
}

/// The games of a pass counted by how they went for the candidate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Tally {
    won: usize,
    lost: usize,
    drawn: usize,
    no_result: usize,
}

impl Tally {
    fn of(marks: impl IntoIterator<Item = Mark>) -> Tally {
        let mut tally = Tally::default();
        for mark in marks {
            match mark {
                Mark::Won => tally.won += 1,
                Mark::Lost => tally.lost += 1,
                Mark::Drawn => tally.drawn += 1,
                Mark::NoResult => tally.no_result += 1,
            }
        }
        tally
    }

    fn games(&self) -> usize {
        self.won + self.lost + self.drawn + self.no_result
    }
}

/// `2026-10-14 03:12 UTC` for a Unix time.
fn utc_minute_of(unix_seconds: i64) -> String {
    let of_day = unix_seconds.rem_euclid(SECONDS_PER_DAY);
    format!(
        "{} {:02}:{:02} UTC",
        utc_date_of(unix_seconds),
        of_day / SECONDS_PER_HOUR,
        of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE
    )
}

/// The latest pass, in Markdown, for the top of the lane's draft pull request (training/loop.sh): how
/// many games the candidate beat the parent in, a table of them as p1, as p2 and in all, and a grid
/// with one square per game. `games` are (result, the candidate's seat) in game order, the games of
/// the deals `test` read; `at` is when the run ended.
#[allow(clippy::too_many_arguments)]
fn last_pass(
    lane: Lane,
    mode: &str,
    at: &str,
    record: &GenerationRecord,
    parent: &str,
    games: &[(Option<GameResult>, PlayerId)],
    test: &Sprt,
    failures: &[String],
) -> String {
    let gate = lane.gate();
    let marks: Vec<Mark> = games
        .iter()
        .map(|(result, seat)| Mark::of(*result, *seat))
        .collect();
    let as_seat = |seat: PlayerId| {
        Tally::of(
            games
                .iter()
                .zip(&marks)
                .filter(|((_, candidate), _)| *candidate == seat)
                .map(|(_, mark)| *mark),
        )
    };
    let all = Tally::of(marks.iter().copied());
    let run = match mode {
        "dry run" => "A dry run",
        "verify" => "A re-check (`--verify`)",
        _ => "The promotion run",
    };
    let verdict = match (failures.is_empty(), mode) {
        (true, "promotion") => "clears the gate and is promoted",
        (true, _) => "clears the gate",
        (false, _) => "falls short of the gate",
    };
    let seeds = seeds_text(lane, &record.tree, all.games() as i32);
    let (low, high) = arena::wilson_interval(all.won, all.games());

    let says = match test.verdict {
        Verdict::Pass => "the sequential test passes it",
        Verdict::Fail => "the sequential test fails it",
        Verdict::Capped => "the sequential test is undecided at its cap",
        Verdict::Open => "the sequential test needs more deals",
    };
    let mut out = format!(
        "## Latest pass: beat the AI on `main` {} times in {}; {says}\n\n",
        all.won,
        all.games()
    );
    out.push_str(&format!(
        "{run} of the `{}` lane's gate at {at} against the parent, {parent}: it {verdict}. \
         {seeds} Every run of the gate on the training box rewrites this section.\n\n",
        lane.as_str()
    ));
    out.push_str("| The candidate | Beat the bot | Lost | Drawn | No result |\n|---|---|---|---|---|\n");
    for (label, tally) in [
        ("as p1 (odd games)", as_seat(PlayerId::P1)),
        ("as p2 (even games)", as_seat(PlayerId::P2)),
    ] {
        out.push_str(&format!(
            "| {label} | {} | {} | {} | {} |\n",
            tally.won, tally.lost, tally.drawn, tally.no_result
        ));
    }
    out.push_str(&format!(
        "| **all {} games** | **{}** | **{}** | **{}** | **{}** |\n\n",
        all.games(),
        all.won,
        all.lost,
        all.drawn,
        all.no_result
    ));
    let ban_rule = match lane {
        Lane::Unban => ", and the unban lane needs strictly fewer than the parent's",
        Lane::Improve | Lane::Learn => "",
    };
    out.push_str(&format!(
        "{} wins put its true win rate at {} (95% interval). Shadow bans: {} (the parent's \
         {}){ban_rule}.\n\n",
        all.won,
        interval_text(low, high),
        record.shadow_ban,
        record.parent_shadow_ban
    ));
    out.push_str(&format!(
        "{}\n\n{}\n\n",
        paired_text(games),
        test_text(&gate, test)
    ));
    if !failures.is_empty() {
        out.push_str("Short of the gate:\n\n");
        for failure in failures {
            out.push_str(&format!("- {failure}\n"));
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "Game by game ({} beat the bot, {} lost, {} drawn, {} no result):\n\n```text\n",
        Mark::Won.square(),
        Mark::Lost.square(),
        Mark::Drawn.square(),
        Mark::NoResult.square()
    ));
    let rows: Vec<(String, String)> = marks
        .chunks(PASS_GRID_ROW)
        .enumerate()
        .map(|(row, chunk)| {
            let from = row * PASS_GRID_ROW + 1;
            let label = format!("{from}–{}", from + chunk.len() - 1);
            (label, chunk.iter().map(|mark| mark.square()).collect())
        })
        .collect();
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    for (label, squares) in &rows {
        out.push_str(&format!("{label:>width$}  {squares}\n"));
    }
    out.push_str("```\n");
    out
}

/// Rewrites `<dir>/last-pass.md` with the pass, through a temporary file, so the loop never reads
/// half of one.
fn write_last_pass(dir: &Path, text: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("promote: cannot create {}", dir.display()))?;
    let path = dir.join(LAST_PASS_FILE);
    let next = dir.join(format!("{LAST_PASS_FILE}.new"));
    std::fs::write(&next, text).with_context(|| format!("promote: cannot write {}", next.display()))?;
    std::fs::rename(&next, &path).with_context(|| format!("promote: cannot write {}", path.display()))
}

/// The run's report, in Markdown: the lane's pull request body (training/loop.sh).
#[allow(clippy::too_many_arguments)]
fn report(
    lane: Lane,
    mode: &str,
    record: &GenerationRecord,
    counts: &GateCounts,
    parent: &Entrant,
    vs_parent: &[ArenaOutcome],
    failures: &[String],
) -> anyhow::Result<String> {
    let gate = lane.gate();
    let verdict = match (failures.is_empty(), mode) {
        (true, "promotion") => "promoted",
        (true, _) => "passes",
        (false, _) => "does not pass",
    };
    let (parent_draws, parent_aborted) = draws_and_aborted(vs_parent);
    let parent_name = parent_name(parent, record);
    let seeds = seeds_text(lane, &record.tree, counts.games);
    let mut out = String::new();
    out.push_str(&format!(
        "## AI gen {} ({}): {verdict} ({mode})\n\n",
        record.generation,
        lane.as_str()
    ));
    let (low, high) = arena::wilson_interval(counts.vs_parent.max(0) as usize, counts.games.max(0) as usize);
    out.push_str("| Opponent | Wins | 95% interval | Draws | No result |\n|---|---|---|---|---|\n");
    out.push_str(&format!(
        "| parent: {parent_name} | {} | {} | {parent_draws} | {parent_aborted} |\n\n",
        record.vs_parent,
        interval_text(low, high)
    ));
    out.push_str(&format!(
        "{}\n\n{}\n\n",
        paired_text(&results_of(vs_parent)),
        test_text(&gate, &counts.test)
    ));
    let ban_rule = match lane {
        Lane::Unban => "; the unban lane needs strictly fewer than the parent's",
        Lane::Improve | Lane::Learn => "",
    };
    out.push_str(&format!(
        "Shadow bans: {} (parent {}){ban_rule}.\n\n",
        record.shadow_ban, record.parent_shadow_ban
    ));
    out.push_str(&format!(
        "{seeds} Each seat's deck is built by one rule, from every set that ships minus its own AI's \
         shadow ban, with no handicap and no shaping (R976000). A draw is not a win, nor is a game \
         without a result. No game is played against random (R732001). Every game must also be clean \
         on either seat: no refused action, throw, fallback or panic caught in an AI's search, and a \
         log that folds back to its hash (R1489000).\n\n"
    ));
    if !failures.is_empty() {
        out.push_str("Fails:\n\n");
        for failure in failures {
            out.push_str(&format!("- {failure}\n"));
        }
        out.push('\n');
    }
    out.push_str(&format!("```json\n{}\n```\n", serde_json::to_string(record)?));
    Ok(out)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let repo = repo_root();
    let mode = if args.verify {
        "verify"
    } else if args.dry_run {
        "dry run"
    } else {
        "promotion"
    };

    let parent = arena::entrant(&AgentSpec::Bin(args.parent_bin.clone()))?;
    if let Some(done) = lane_done(args.lane, &parent) {
        bail!("{done}");
    }

    // Every gate plays hold-out deals (R850001, R1679000), so no run measures uncommitted changes.
    if ai_source_dirty(&repo)? {
        bail!("{}", dirty_refusal(args.lane, mode));
    }

    let tree = ai_tree(&repo)?;
    let candidate = arena::entrant(&AgentSpec::SelfAi)?;
    let parent_generation = parent
        .generation
        .with_context(|| format!("promote: {} reported no generation", args.parent_bin.display()))?;

    let gate = args.lane.gate();
    let seed_of = |k: i32| game_seed(args.lane, &tree, k);
    let games = arena::setups(&candidate, &parent, 2 * gate.max_deals, true, seed_of)?;
    eprintln!(
        "promote: {} lane against gen {parent_generation}: the sequential test on deals {} to at most {}, \
         each played twice with the seats swapped, {} deals at a time",
        args.lane.as_str(),
        game_seed(args.lane, &tree, 1),
        game_seed(args.lane, &tree, gate.max_deals),
        gate.batch_deals
    );
    // R1679000: a batch at a time, read in deal order; the games past the deal it stops at are set
    // aside, so the deals it reads, and so its verdict, never depend on the thread count.
    let batch = 2 * usize::try_from(gate.batch_deals.max(1)).unwrap_or(1);
    let mut outcomes: Vec<ArenaOutcome> = Vec::new();
    let test = loop {
        let from = outcomes.len();
        let to = (from + batch).min(games.len());
        outcomes.extend(arena::play_games(&games[from..to])?);
        let test = sprt(&gate, &arena::deal_edges(&results_of(&outcomes)));
        eprintln!(
            "promote: {} deals played: log-likelihood ratio {:.2}",
            outcomes.len() / 2,
            test.llr
        );
        if test.verdict != Verdict::Open || to == games.len() {
            break test;
        }
    };
    outcomes.truncate(2 * test.deals);
    if let Some(path) = arena::records_path(None) {
        arena::append_records(&path, &arena::records_of(&outcomes)?)?;
    }
    let vs_parent = &outcomes[..];
    let replayed = vs_parent
        .iter()
        .map(arena::replays)
        .collect::<anyhow::Result<Vec<bool>>>()?;

    let counts = GateCounts {
        games: i32::try_from(vs_parent.len()).unwrap_or(i32::MAX),
        vs_parent: wins_of(&results_of(vs_parent)),
        shadow_ban: candidate.shadow_ban.len(),
        parent_shadow_ban: parent.shadow_ban.len(),
        test,
    };
    let record = GenerationRecord {
        generation: parent_generation + 1,
        lane: args.lane.as_str().to_string(),
        parent: parent_commit(&repo),
        tree: tree.clone(),
        vs_random: None,
        vs_parent: format!("{}/{}", counts.vs_parent, counts.games),
        shadow_ban: counts.shadow_ban,
        parent_shadow_ban: counts.parent_shadow_ban,
        date: arena::utc_date(),
    };
    let failures = promotion_failures(args.lane, &counts, vs_parent, &replayed);
    print!(
        "{}",
        report(args.lane, mode, &record, &counts, &parent, vs_parent, &failures)?
    );
    if let Some(dir) = arena::training_out(None) {
        let pass = last_pass(
            args.lane,
            mode,
            &utc_minute_of(arena::unix_now()),
            &record,
            &parent_name(&parent, &record),
            &results_of(vs_parent),
            &counts.test,
            &failures,
        );
        // The pass is for people watching the lane: a file the box cannot write never fails the gate.
        if let Err(error) = write_last_pass(&dir, &pass) {
            eprintln!("{error:#}");
        }
    }

    if args.verify {
        let path = repo.join(GENERATION_PATH);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("promote: cannot read {}", path.display()))?;
        let claimed: Value = serde_json::from_str(&text)
            .with_context(|| format!("promote: {} is not JSON", path.display()))?;
        let mismatches = verify_mismatches(&claimed, &record);
        if !mismatches.is_empty() {
            println!("\nVerification failed:\n");
            for mismatch in &mismatches {
                println!("- {mismatch}");
            }
            bail!("promote: {GENERATION_PATH} does not match the gate's measurement");
        }
        if claimed.get("parent") != Some(&json!(record.parent)) {
            eprintln!(
                "promote: note: {GENERATION_PATH} names parent {}, and the parent is now {}; its AI measured the same",
                claimed.get("parent").unwrap_or(&Value::Null),
                json!(record.parent)
            );
        }
    }
    if !failures.is_empty() {
        bail!(
            "promote: the {} gate is not passed: {}",
            args.lane.as_str(),
            failures.join("; ")
        );
    }
    if mode == "promotion" {
        write_generation(&repo.join(GENERATION_PATH), &record)?;
        let history = repo.join(format!("training/history/{}.jsonl", args.lane.as_str()));
        write_history(&history, &record)?;
        eprintln!(
            "promote: wrote {GENERATION_PATH} and {}; commit them with your change as `AI gen {} ({}): <what changed>`",
            history.display(),
            record.generation,
            args.lane.as_str()
        );
    }
    Ok(())
}

/// docs/v0.3.0/README.md V26: `promote` promotes exactly by §8's table and writes `generation.json`
/// and the history line.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_ai::{RejectedAction, SimPanic};
    use jackioh_engine::{ActionBody, GameOverReason, PerPlayer, Winner};
    use std::process::Command;

    /// The test as it stands after `deals` deals, its ratio `llr` and its verdict.
    fn tested(deals: usize, llr: f64, verdict: Verdict) -> Sprt {
        Sprt { deals, llr, verdict }
    }

    /// A run that read 400 deals and passed, with `vs_parent` wins of their 800 games.
    fn counts(vs_parent: i32, shadow_ban: usize, parent_shadow_ban: usize) -> GateCounts {
        GateCounts {
            games: 800,
            vs_parent,
            shadow_ban,
            parent_shadow_ban,
            test: tested(400, 3.1, Verdict::Pass),
        }
    }

    /// The same run, the test as `test` says.
    fn counts_testing(test: Sprt, shadow_ban: usize, parent_shadow_ban: usize) -> GateCounts {
        GateCounts {
            games: 2 * test.deals as i32,
            vs_parent: test.deals as i32,
            shadow_ban,
            parent_shadow_ban,
            test,
        }
    }

    #[test]
    fn r850000_r1679000_the_improve_gate_is_the_sequential_test_on_paired_deals() {
        let improve = |test, shadow_ban| counts_testing(test, shadow_ban, 11);
        assert!(gate_failures(Lane::Improve, &improve(tested(400, 3.1, Verdict::Pass), 11)).is_empty());
        assert_eq!(
            gate_failures(Lane::Improve, &improve(tested(250, -2.95, Verdict::Fail), 11)),
            vec![
                "vs parent: the sequential test fails it after 250 deals (250/500 wins): its \
                 log-likelihood ratio -2.95 reached -2.94"
                    .to_string()
            ]
        );
        assert_eq!(
            gate_failures(Lane::Improve, &improve(tested(1000, 1.5, Verdict::Capped), 11)),
            vec![
                "vs parent: the sequential test is undecided after 1000 deals, the most it plays \
                 (1000/2000 wins): its log-likelihood ratio 1.50 is between -2.94 and 2.94"
                    .to_string()
            ]
        );
        assert_eq!(
            gate_failures(Lane::Improve, &improve(tested(6, 0.4, Verdict::Open), 11)).len(),
            1
        );
        // The improve lane has no shadow-ban rule: more bans than the parent's still promote.
        assert!(gate_failures(Lane::Improve, &improve(tested(400, 3.1, Verdict::Pass), 12)).is_empty());
        assert_eq!(
            TRAINING_GATE,
            TrainingGate {
                min_deals: 100,
                max_deals: 1000,
                batch_deals: 100,
                edge_h0: 0.0,
                edge_h1: 0.06,
                alpha: 0.05,
                beta: 0.05,
                variance_floor: 0.01,
            }
        );
        // Every lane's gate is the one test.
        for lane in Lane::value_variants() {
            assert_eq!(lane.gate(), TRAINING_GATE);
        }
    }

    #[test]
    fn r1012000_the_learn_lane_gates_like_improve_on_hold_out_deals_of_its_own() {
        let learn = |test| counts_testing(test, 0, 0);
        assert_eq!(Lane::Learn.gate(), Lane::Improve.gate());
        assert!(gate_failures(Lane::Learn, &learn(tested(300, 2.95, Verdict::Pass))).is_empty());
        assert_eq!(
            gate_failures(Lane::Learn, &learn(tested(300, -3.0, Verdict::Fail))).len(),
            1
        );
        // Its deals are its own: `learn:<tree>:<d>`, never a screen's (R850001), never an export's.
        assert_eq!(game_seed(Lane::Learn, "abc", 7), "learn:abc:7");
        assert!(is_gate_seed_base("learn") && is_gate_seed_base("learn:abc"));
        assert!(!is_gate_seed_base("learned-dev"));
        // A paired gate's dry run reads only the committed AI.
        assert!(dirty_refusal(Lane::Learn, "dry run").contains("R850001"));
    }

    #[test]
    fn r732001_r1679000_the_unban_gate_needs_strictly_fewer_bans_and_the_test_to_pass() {
        let (pass, fail) = (tested(400, 3.1, Verdict::Pass), tested(400, -3.1, Verdict::Fail));
        let unban = |test, shadow_ban| counts_testing(test, shadow_ban, 11);
        assert!(gate_failures(Lane::Unban, &unban(pass, 10)).is_empty());
        assert!(gate_failures(Lane::Unban, &unban(pass, 0)).is_empty());
        // Strictly fewer: as many bans as the parent is a failure, more is too.
        assert_eq!(gate_failures(Lane::Unban, &unban(pass, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &unban(pass, 12)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &unban(fail, 10)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &unban(fail, 11)).len(), 2);
        // The unban lane's test is the improve lane's (R1679000): no lower bar of its own.
        assert_eq!(Lane::Unban.gate(), Lane::Improve.gate());
    }

    #[test]
    fn r1679000_the_test_stops_at_the_first_deal_past_a_bound_from_its_minimum_on() {
        // A gate small enough to work by hand: bounds ln(1/19) and ln(19), about -2.94 and 2.94.
        let gate = TrainingGate {
            min_deals: 2,
            max_deals: 6,
            batch_deals: 2,
            edge_h0: 0.0,
            edge_h1: 0.5,
            alpha: 0.05,
            beta: 0.05,
            variance_floor: 0.01,
        };
        let (lower, upper) = sprt_bounds(&gate);
        assert!((lower + 19f64.ln()).abs() < 1e-12 && (upper - 19f64.ln()).abs() < 1e-12);
        // Two deals both won: a mean of 1 and no spread, so the floor: 0.5 · (2 − 0.5) / 0.01 = 75.
        assert_eq!(sprt(&gate, &[1.0, 1.0]), tested(2, 75.0, Verdict::Pass));
        assert_eq!(sprt(&gate, &[-1.0, -1.0]), tested(2, -125.0, Verdict::Fail));
        // The first deal alone is past the bound, but the test reads two before it may stop.
        assert_eq!(sprt(&gate, &[1.0]).verdict, Verdict::Open);
        assert_eq!(sprt(&gate, &[1.0]).deals, 1);
        // Inside the bounds to the cap: −0.25, 0.14, 0.67, 1.37, then 0.5 · (4 − 1.5) / (5/9) = 2.25
        // after the sixth deal, the most it reads, so it is capped there and the seventh is never read.
        let capped = sprt(&gate, &[1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        assert_eq!((capped.deals, capped.verdict), (6, Verdict::Capped));
        assert!((capped.llr - 2.25).abs() < 1e-12, "{capped:?}");
        // It stops at the first deal past a bound: the deals after it change nothing.
        let early = sprt(&gate, &[1.0, 1.0, -1.0, -1.0, -1.0]);
        assert_eq!(early, sprt(&gate, &[1.0, 1.0]));
        // An AI against itself splits every deal: an edge of 0 at no spread, which the floor fails.
        assert_eq!(sprt(&TRAINING_GATE, &vec![0.0; 200]).verdict, Verdict::Fail);
        assert_eq!(sprt(&TRAINING_GATE, &vec![0.0; 200]).deals, 100);
        // A quarter of the deals won twice and the rest split passes the real gate at its 100th deal:
        // 0.06 · (25 − 3) / 0.1875 = 7.04.
        let gain: Vec<f64> = (0..300).map(|d| if d % 4 == 0 { 1.0 } else { 0.0 }).collect();
        let passed = sprt(&TRAINING_GATE, &gain);
        assert_eq!((passed.deals, passed.verdict), (100, Verdict::Pass));
        assert!((passed.llr - 7.04).abs() < 1e-9, "{passed:?}");
    }

    /// Deal edges as #1058's paired deals came out: won twice `both` of the time less half the edge,
    /// lost twice `both` less half, split the rest, with a mean edge of `edge`, from a seeded stream.
    fn deals_like_1058(edge: f64, deals: usize, seed: u64) -> Vec<f64> {
        const BOTH: f64 = 0.27;
        let mut state = seed;
        (0..deals)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let draw = (state >> 11) as f64 / (1u64 << 53) as f64;
                if draw < (BOTH + edge) / 2.0 {
                    1.0
                } else if draw < BOTH {
                    -1.0
                } else {
                    0.0
                }
            })
            .collect()
    }

    #[test]
    fn r1679000_read_a_batch_at_a_time_the_test_stops_where_it_would_on_every_deal_at_once() {
        // `run` plays `batch_deals` at a time and stops at the first batch whose test is not open: the
        // verdict is the one the whole sequence gives, so where it stops never depends on the batches.
        let batch = TRAINING_GATE.batch_deals as usize;
        for seed in 1..=40 {
            let edges = deals_like_1058(0.03, TRAINING_GATE.max_deals as usize, seed);
            let whole = sprt(&TRAINING_GATE, &edges);
            let batched = (1..)
                .map(|k| sprt(&TRAINING_GATE, &edges[..(k * batch).min(edges.len())]))
                .find(|test| test.verdict != Verdict::Open)
                .unwrap();
            assert_eq!(batched, whole, "seed {seed}");
            assert_eq!(batched.llr.to_bits(), whole.llr.to_bits(), "seed {seed}");
        }
    }

    #[test]
    fn r1679000_on_deals_like_1058s_an_equal_successor_rarely_passes_and_one_at_53_percent_mostly_does() {
        let runs = 400;
        let rate = |edge: f64| {
            let passes = (0..runs)
                .filter(|run| {
                    let edges = deals_like_1058(edge, TRAINING_GATE.max_deals as usize, 1 + *run as u64);
                    sprt(&TRAINING_GATE, &edges).verdict == Verdict::Pass
                })
                .count();
            passes as f64 / runs as f64
        };
        // α and β are 5%: an equal successor passes about 5% of the time, one with the edge it is
        // built for (+0.06, a 53% win rate) about 90%, where 220 of 400 passed it about 15%.
        let equal = rate(0.0);
        let gain = rate(0.06);
        assert!(equal < 0.08, "an equal successor passed {equal}");
        assert!(gain > 0.85, "a 53% successor passed {gain}");
        assert!(rate(-0.06) < 0.01);
    }

    #[test]
    fn r850001_a_dry_run_of_the_paired_gate_measures_only_the_committed_ai() {
        // A lane's hold-out deals are read by the committed AI alone, dry run or not.
        let held_out = dirty_refusal(Lane::Improve, "dry run");
        assert!(held_out.contains("R850001"), "{held_out}");
        assert!(held_out.contains("arena --set improve-dev"), "{held_out}");
        assert!(dirty_refusal(Lane::Improve, "verify").contains("commit the change first"));
        assert!(dirty_refusal(Lane::Improve, "promotion").contains("commit the change first"));
        // The unban lane's dry run refuses them as well.
        assert!(dirty_refusal(Lane::Unban, "dry run").contains("R850001"));
        // Nor does a screen play the gate's deals: its base names neither lane.
        assert!(is_gate_seed_base("improve"));
        assert!(is_gate_seed_base("improve:4b825dc"));
        assert!(is_gate_seed_base("unban:4b825dc:7"));
        assert!(!is_gate_seed_base("improve-dev"));
        assert!(!is_gate_seed_base("improve-try-3"));
        assert!(!is_gate_seed_base("unban-try-1"));
    }

    #[test]
    fn r732001_a_promotion_records_no_count_against_random() {
        let mut record = sample_record();
        record.vs_random = None;
        let line = serde_json::to_string(&record).unwrap();
        assert!(line.contains(r#""vsRandom":null"#), "{line}");
        // --verify holds the field like the rest: a claimed count against random is a mismatch.
        let mut claimed = serde_json::to_value(&record).unwrap();
        assert!(verify_mismatches(&claimed, &record).is_empty());
        claimed["vsRandom"] = json!("93/100");
        assert_eq!(verify_mismatches(&claimed, &record).len(), 1);
    }

    /// A parent as `arena::entrant` readies a `bin:` one, keeping `shadow_ban` out of its decks.
    fn parent_banning(shadow_ban: &[&str]) -> Entrant {
        Entrant {
            spec: AgentSpec::Bin(PathBuf::from("parent-jackioh")),
            label: "bin-gen8".to_string(),
            generation: Some(8),
            lane: Some("unban".to_string()),
            shadow_ban: shadow_ban.iter().map(|id| id.to_string()).collect(),
            skill: jackioh_ai::AI_SKILL_MAX,
        }
    }

    #[test]
    fn the_unban_lane_has_nothing_to_do_while_the_parents_shadow_ban_is_empty() {
        let done = lane_done(Lane::Unban, &parent_banning(&[])).expect("an empty parent ban idles the lane");
        assert!(done.contains("the unban lane has nothing to do"), "{done}");
        assert!(done.contains("No game was played"), "{done}");
        // Whatever the games would say, the gate cannot pass: strictly fewer bans than none.
        assert!(!gate_failures(Lane::Unban, &counts(100, 0, 0)).is_empty());
        // Generation 8's own promotion, against a parent with one ban left, still plays its games.
        assert_eq!(lane_done(Lane::Unban, &parent_banning(&["core-076"])), None);
        // The improve lane has no ban rule: an empty ban list changes nothing for it.
        assert_eq!(lane_done(Lane::Improve, &parent_banning(&[])), None);
    }

    #[test]
    fn draws_and_games_without_a_result_count_as_losses() {
        let result = |winner| {
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
            (result(Winner::P1), PlayerId::P1),
            (draw, PlayerId::P1),
            (draw, PlayerId::P2),
            (None, PlayerId::P2),
            (result(Winner::P2), PlayerId::P1),
            (result(Winner::P2), PlayerId::P2),
        ];
        assert_eq!(wins_of(&games), 2);

        // A deal of a win and a draw is half an edge, and of a draw and a loss half one the other way:
        // a deal drawn twice is no evidence either way, and the test fails a run of them.
        let deals = seated(&[
            Some(Winner::P1),
            Some(Winner::Draw),
            Some(Winner::Draw),
            Some(Winner::P1),
        ]);
        assert_eq!(arena::deal_edges(&deals), vec![0.5, -0.5]);
        let drawn = seated(&vec![Some(Winner::Draw); 400]);
        let test = sprt(&TRAINING_GATE, &arena::deal_edges(&drawn));
        assert_eq!((test.deals, test.verdict), (100, Verdict::Fail));
        assert_eq!(wins_of(&drawn), 0);
    }

    #[test]
    fn the_report_reads_the_wins_as_an_interval() {
        // 220 of 400, the improve lane's bar before R1679000, is a true win rate anywhere from about
        // 50% to 60%; 60 of 100, the unban lane's, anywhere from about 50% to 69%.
        let (low, high) = arena::wilson_interval(220, 400);
        assert_eq!(interval_text(low, high), "50–60%");
        let (low, high) = arena::wilson_interval(60, 100);
        assert_eq!(interval_text(low, high), "50–69%");
        // The test's own sentence names its hypotheses, its errors, its deals and its bounds.
        let said = test_text(&TRAINING_GATE, &tested(412, 2.97, Verdict::Pass));
        assert_eq!(
            said,
            "The sequential test (R1679000) weighs an edge of +0.0 points against +6.0, with a 5% chance \
             of promoting the first and 5% of failing the second, over 100 to 1000 deals: after 412 deals \
             its log-likelihood ratio is 2.97, against bounds of -2.94 and 2.94, so it passes."
        );
        assert!(
            test_text(&TRAINING_GATE, &tested(1000, 0.1, Verdict::Capped))
                .ends_with("so undecided at the most deals it plays, it fails.")
        );
    }

    /// Games as the gate seats them (the candidate p1 on odd games), each ended as `winners` says:
    /// `Some(winner)`, or `None` for a game without a result.
    fn seated(winners: &[Option<Winner>]) -> Vec<(Option<GameResult>, PlayerId)> {
        winners
            .iter()
            .enumerate()
            .map(|(k, winner)| {
                let result = winner.map(|winner| GameResult {
                    winner,
                    reason: match winner {
                        Winner::Draw => GameOverReason::TurnCap,
                        _ => GameOverReason::HeroDeath,
                    },
                });
                let seat = if k % 2 == 0 { PlayerId::P1 } else { PlayerId::P2 };
                (result, seat)
            })
            .collect()
    }

    #[test]
    fn the_latest_pass_counts_the_games_the_candidate_beat_the_bot_in_by_seat_and_game_by_game() {
        let (p1, p2, draw) = (Some(Winner::P1), Some(Winner::P2), Some(Winner::Draw));
        // Games 1 to 12: won as p1 and as p2, lost as each, a draw, a game without a result, and so on.
        let games = seated(&[p1, p2, p2, p1, draw, None, p1, p2, p1, p1, p2, p2]);
        let test = sprt(&TRAINING_GATE, &arena::deal_edges(&games));
        let measured = GateCounts {
            games: 12,
            vs_parent: wins_of(&games),
            shadow_ban: 11,
            parent_shadow_ban: 11,
            test,
        };
        assert_eq!(measured.vs_parent, 6);
        assert_eq!((test.deals, test.verdict), (6, Verdict::Open));
        let failures = gate_failures(Lane::Improve, &measured);
        let pass = last_pass(
            Lane::Improve,
            "dry run",
            "2026-10-14 03:12 UTC",
            &sample_record(),
            "gen 8 (improve), `main` at `0123abc`",
            &games,
            &test,
            &failures,
        );
        assert!(
            pass.starts_with(
                "## Latest pass: beat the AI on `main` 6 times in 12; the sequential test needs more deals\n"
            ),
            "{pass}"
        );
        assert!(pass.contains("A dry run of the `improve` lane's gate at 2026-10-14 03:12 UTC against the parent, gen 8 (improve), `main` at `0123abc`: it falls short of the gate."), "{pass}");
        // The improve gate is paired: twelve games are six deals, each played twice.
        assert!(
            pass.contains("Deals `improve:4b825dc:1` to `improve:4b825dc:6`, each played twice with the seats swapped: the candidate sits p1 in a deal's first game (the odd games) and p2 in its second."),
            "{pass}"
        );
        // Deals 1 to 6 come to 1, -1, 0, 1, 0 and 0 for the candidate.
        assert!(
            pass.contains("Paired by deal, the candidate's wins less the parent's come to +16.7 points of the games (95% interval -43.6 to +76.9).\n\n"),
            "{pass}"
        );
        // A draw and a game without a result are counted, but neither is a win.
        assert!(pass.contains("| as p1 (odd games) | 3 | 2 | 1 | 0 |\n"), "{pass}");
        assert!(
            pass.contains("| as p2 (even games) | 3 | 2 | 0 | 1 |\n"),
            "{pass}"
        );
        assert!(
            pass.contains("| **all 12 games** | **6** | **4** | **1** | **1** |\n"),
            "{pass}"
        );
        assert!(
            pass.contains("Short of the gate:\n\n- vs parent: the sequential test has read 6 deals, fewer than it needs (6/12 wins)\n"),
            "{pass}"
        );
        assert!(
            pass.contains("after 6 deals its log-likelihood ratio is"),
            "{pass}"
        );
        // One square per game, ten to a row, each row labeled with its games.
        assert!(
            pass.contains("```text\n 1–10  🟩🟩🟥🟥⬜⬛🟩🟩🟩🟥\n11–12  🟥🟩\n```\n"),
            "{pass}"
        );
    }

    #[test]
    fn the_latest_pass_names_its_run_and_what_the_unban_lane_needs() {
        // 100 games, 50 deals each won twice: ten rows, their labels right-aligned.
        let won_all = seated(
            &(0..100)
                .map(|k| Some(if k % 2 == 0 { Winner::P1 } else { Winner::P2 }))
                .collect::<Vec<_>>(),
        );
        let passed = tested(50, 150.0, Verdict::Pass);
        let mut record = sample_record();
        record.lane = "unban".to_string();
        record.shadow_ban = 10;
        let promoted = last_pass(
            Lane::Unban,
            "promotion",
            "2026-10-14 03:12 UTC",
            &record,
            "gen 8 (unban)",
            &won_all,
            &passed,
            &[],
        );
        assert!(promoted.starts_with(
            "## Latest pass: beat the AI on `main` 100 times in 100; the sequential test passes it\n"
        ));
        assert!(
            promoted.contains("The promotion run of the `unban` lane's gate"),
            "{promoted}"
        );
        assert!(
            promoted.contains("it clears the gate and is promoted."),
            "{promoted}"
        );
        assert!(
            promoted.contains("Shadow bans: 10 (the parent's 11), and the unban lane needs strictly fewer than the parent's."),
            "{promoted}"
        );
        assert!(!promoted.contains("Short of the gate"), "{promoted}");
        // The unban gate is paired too: its games are deals, and it reads their edge.
        assert!(
            promoted.contains(
                "Deals `unban:4b825dc:1` to `unban:4b825dc:50`, each played twice with the seats swapped"
            ),
            "{promoted}"
        );
        assert!(promoted.contains("Paired by deal"), "{promoted}");
        assert!(
            promoted.contains("\n  1–10  🟩🟩🟩🟩🟩🟩🟩🟩🟩🟩\n"),
            "{promoted}"
        );
        assert!(
            promoted.contains("\n91–100  🟩🟩🟩🟩🟩🟩🟩🟩🟩🟩\n```\n"),
            "{promoted}"
        );

        // A dry run and a re-check name themselves.
        let dry = last_pass(
            Lane::Unban,
            "dry run",
            "at",
            &record,
            "gen 8",
            &won_all,
            &passed,
            &[],
        );
        assert!(dry.contains("A dry run of the `unban` lane's gate"), "{dry}");
        assert!(dry.contains("it clears the gate."), "{dry}");
        let verify = last_pass(
            Lane::Unban,
            "verify",
            "at",
            &record,
            "gen 8",
            &won_all,
            &passed,
            &[],
        );
        assert!(
            verify.contains("A re-check (`--verify`) of the `unban` lane's gate"),
            "{verify}"
        );
    }

    #[test]
    fn a_pass_is_timed_to_the_minute_in_utc() {
        assert_eq!(utc_minute_of(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_minute_of(1_791_947_579), "2026-10-14 03:12 UTC");
    }

    #[test]
    fn the_latest_pass_replaces_the_one_before_it() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("jackioh-pass-{}-{stamp}", std::process::id()));
        write_last_pass(&dir, "first\n").unwrap();
        write_last_pass(&dir, "second\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join(LAST_PASS_FILE)).unwrap(),
            "second\n"
        );
        assert!(!dir.join(format!("{LAST_PASS_FILE}.new")).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn seeds_name_the_lane_the_tree_and_the_game() {
        assert_eq!(game_seed(Lane::Improve, "4b825dc", 7), "improve:4b825dc:7");
        assert_eq!(game_seed(Lane::Unban, "4b825dc", 100), "unban:4b825dc:100");
    }

    /// Runs git in `dir` for a test, with an identity and no signing, panicking on failure.
    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=JackiOh tests",
                "-c",
                "user.email=tests@jackioh.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn the_seeds_are_stable_across_a_generation_json_only_commit() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("jackioh-promote-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(dir.join(AI_SOURCE)).unwrap();
        git_in(&dir, &["init", "-q"]);
        std::fs::write(dir.join(AI_SOURCE).join("lib.rs"), "pub fn decide() {}\n").unwrap();
        std::fs::write(dir.join(GENERATION_PATH), "{\"generation\": 0}\n").unwrap();
        git_in(&dir, &["add", "-A"]);
        git_in(&dir, &["commit", "-q", "-m", "AI source"]);
        let tree = ai_tree(&dir).unwrap();
        assert!(!ai_source_dirty(&dir).unwrap());

        // A promotion commits generation.json and the history line: the seeds do not move.
        std::fs::write(dir.join(GENERATION_PATH), "{\"generation\": 1}\n").unwrap();
        std::fs::create_dir_all(dir.join("training/history")).unwrap();
        std::fs::write(dir.join("training/history/improve.jsonl"), "{\"generation\":1}\n").unwrap();
        git_in(&dir, &["add", "-A"]);
        git_in(&dir, &["commit", "-q", "-m", "AI gen 1 (improve): record"]);
        assert_eq!(ai_tree(&dir).unwrap(), tree);
        assert_eq!(
            game_seed(Lane::Improve, &ai_tree(&dir).unwrap(), 1),
            game_seed(Lane::Improve, &tree, 1)
        );

        // A change to the AI's source is another AI: other seeds, and dirty until it is committed.
        std::fs::write(
            dir.join(AI_SOURCE).join("lib.rs"),
            "pub fn decide() { let _ = 1; }\n",
        )
        .unwrap();
        assert!(ai_source_dirty(&dir).unwrap());
        assert_eq!(ai_tree(&dir).unwrap(), tree);
        git_in(&dir, &["commit", "-q", "-am", "AI change"]);
        assert_ne!(ai_tree(&dir).unwrap(), tree);

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn sample_record() -> GenerationRecord {
        GenerationRecord {
            generation: 8,
            lane: "improve".to_string(),
            parent: Some("0123abc".to_string()),
            tree: "4b825dc".to_string(),
            vs_random: Some("93/100".to_string()),
            vs_parent: "87/100".to_string(),
            shadow_ban: 11,
            parent_shadow_ban: 11,
            date: "2026-11-02".to_string(),
        }
    }

    #[test]
    fn the_record_serializes_in_surface_order() {
        assert_eq!(
            serde_json::to_string(&sample_record()).unwrap(),
            r#"{"generation":8,"lane":"improve","parent":"0123abc","tree":"4b825dc","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}"#
        );
    }

    #[test]
    fn verify_ignores_the_date_and_the_parent_commit_and_nothing_else() {
        let measured = sample_record();
        let mut claimed = serde_json::to_value(&measured).unwrap();
        claimed["date"] = json!("2026-10-30");
        claimed["parent"] = json!("fedcba9");
        assert!(verify_mismatches(&claimed, &measured).is_empty());

        claimed["vsParent"] = json!("88/100");
        claimed["generation"] = json!(9);
        let mismatches = verify_mismatches(&claimed, &measured);
        assert_eq!(mismatches.len(), 2);
        assert!(mismatches[0].starts_with("generation:"));
        assert!(mismatches[1].starts_with("vsParent:"));

        // Generation 0's record never verifies as a promotion. Its text, not the live
        // crates/ai/generation.json, which every promotion replaces and no lane may test around.
        let port = json!({
            "generation": 0,
            "lane": "port",
            "parent": null,
            "vsRandom": "94/100",
            "vsParent": null,
            "shadowBan": 11,
            "date": "2026-10-07",
            "source": "packages/ai at 91cc43c (94/100, 35/50, 47/50 in TS)"
        });
        assert!(!verify_mismatches(&port, &measured).is_empty());
    }

    #[test]
    fn the_history_keeps_one_line_per_generation() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("jackioh-history-{}-{stamp}", std::process::id()));
        let path = dir.join("training/history/improve.jsonl");
        let mut record = sample_record();
        write_history(&path, &record).unwrap();
        record.vs_parent = "86/100".to_string();
        write_history(&path, &record).unwrap();
        record.generation = 9;
        write_history(&path, &record).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"vsParent\":\"86/100\""));
        assert!(lines[1].starts_with("{\"generation\":9,"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A clean finished win of game `n`: the candidate on p1 on odd games, p2 on even ones.
    fn clean_win(n: i32) -> ArenaOutcome {
        let a_seat = if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 };
        ArenaOutcome {
            game: arena::ArenaGame {
                seed: game_seed(Lane::Improve, "4b825dc", arena::deal_of(n)),
                n,
                decks: (Vec::new(), Vec::new()),
                seats: PerPlayer::new(AgentSpec::SelfAi, AgentSpec::SelfAi),
                skills: PerPlayer::new(jackioh_ai::AI_SKILL_MAX, jackioh_ai::AI_SKILL_MAX),
                a_seat,
                labels: ("self".to_string(), "bin-gen8".to_string()),
                paired: true,
                turn_actions: None,
                stop_unsettled: false,
                self_budget: jackioh_ai::AI_GATE_BUDGET,
            },
            result: Some(GameResult {
                winner: Winner::from(a_seat),
                reason: GameOverReason::HeroDeath,
            }),
            log: Vec::new(),
            hash: "h".to_string(),
            turns: 9,
            rejected: Vec::new(),
            thrown: Vec::new(),
            fallbacks: PerPlayer::new(0, 0),
            sim_panics: PerPlayer::new(0, 0),
            first_sim_panic: None,
            times: PerPlayer::new(Vec::new(), Vec::new()),
        }
    }

    #[test]
    fn r1489000_one_game_with_a_search_panic_fails_the_gate_with_every_game_won_and_names_its_seed() {
        let mut games: Vec<ArenaOutcome> = (1..=4).map(clean_win).collect();
        let replayed = vec![true; 4];
        assert!(promotion_failures(Lane::Improve, &counts(400, 11, 11), &games, &replayed).is_empty());
        games[2].sim_panics.p1 = 1;
        games[2].first_sim_panic = Some(SimPanic {
            seat: PlayerId::P1,
            message: "a guessed world broke".into(),
        });
        assert_eq!(wins_of(&results_of(&games)), 4);
        assert_eq!(
            promotion_failures(Lane::Improve, &counts(400, 11, 11), &games, &replayed),
            vec![
                "unclean game improve:4b825dc:2 (the candidate p1): p1: 1 search panics, the first: a \
                 guessed world broke"
                    .to_string()
            ]
        );
    }

    #[test]
    fn r1489000_a_refusal_a_throw_a_fallback_or_a_replay_mismatch_on_either_seat_fails_the_gate() {
        let mut games: Vec<ArenaOutcome> = (1..=4).map(clean_win).collect();
        let mut replayed = vec![true; 4];
        games[0].fallbacks.p2 = 2;
        games[1].rejected.push(RejectedAction {
            seat: PlayerId::P2,
            action: ActionBody::EndTurn,
            error: "refused".into(),
        });
        games[1].thrown.push("p1: controller bin threw: boom".to_string());
        replayed[3] = false;
        assert_eq!(
            promotion_failures(Lane::Improve, &counts(400, 11, 11), &games, &replayed),
            vec![
                "unclean game improve:4b825dc:1 (the candidate p1): p2: 2 fallbacks".to_string(),
                "unclean game improve:4b825dc:1 (the candidate p2): p2: 1 refused actions; p1: \
                 controller bin threw: boom"
                    .to_string(),
                "unclean game improve:4b825dc:2 (the candidate p2): its log does not fold back to its \
                 hash"
                    .to_string(),
            ]
        );
        let failures = promotion_failures(
            Lane::Improve,
            &counts_testing(tested(300, -3.0, Verdict::Fail), 11, 11),
            &games,
            &replayed,
        );
        assert_eq!(failures.len(), 4);
        assert!(failures[0].starts_with("vs parent: the sequential test fails it after 300 deals"));
    }
}
