//! `cargo jackioh promote`: the promotion gate of the AI training lanes (SURFACE §14.2;
//! docs/v0.3.0/README.md §8; training/README.md).
//!
//! ```text
//! cargo jackioh promote --lane improve|unban|learn --parent-bin <path to main's jackioh> [--dry-run] [--verify]
//! ```
//!
//! The candidate is this build's AI (`self`). It plays the lane's `games` against the parent, the AI
//! on `main` (`bin:<parent-bin>`, SURFACE §14.1), seats alternating (the candidate sits p1 on odd
//! games), on the seeds `"<lane>:<tree>:<k>"` where `<tree>` is `git rev-parse HEAD:crates/ai/src`, so
//! committing `generation.json` (or the history line) does not change them. A paired gate (R850000)
//! plays deals: deal `d`'s seed is `"<lane>:<tree>:<d>"`, played twice with the seats swapped, as
//! game `2d − 1` (the candidate p1) and game `2d` (the candidate p2). Those deals are the lane's
//! hold-out set (R850001): named for the committed AI, read only by it, and never a screen's (`arena`
//! refuses their base). Each seat's deck is `build_ai_deck` over every set that ships minus its own
//! AI's shadow ban (R186), with no handicap (Easy both, R180), both seats by that one rule and
//! neither shaped (R976000). The games run on rayon's pool (`RAYON_NUM_THREADS` is respected); no
//! result depends on the thread count.
//!
//! It promotes when:
//!
//! | Lane      | games                          | vs parent                            | shadow bans                      |
//! |-----------|--------------------------------|--------------------------------------|----------------------------------|
//! | `improve` | 400: 200 deals, paired         | ≥ `TRAINING_IMPROVE.vs_parent` (220) | —                                |
//! | `unban`   | 100, unpaired                  | ≥ `TRAINING_UNBAN.vs_parent` (60)    | strictly fewer than the parent's |
//! | `learn`   | 400: 200 deals, paired         | ≥ `TRAINING_LEARN.vs_parent` (220)   | —                                |
//!
//! The `learn` lane is the learned evaluation's (#1012, R1012000): a network a script retrained, no
//! LLM in the loop (`training/learn.md`), gated like `improve` on hold-out deals of its own.
//!
//! It plays no games against SPEC §10.7's random policy (R732001): the lanes dropped them once
//! generation 9, the first AI dealt the Meditative set (issue #732), had beaten random in at least 90
//! of 100, and a promotion's record says `"vsRandom": null`. The daily quality gates (`cargo jackioh
//! gate --full`) still hold the AI on `main` to their floor against random.
//!
//! A draw is not a win, and neither is a game without a result. Without `--dry-run` or `--verify`, a
//! pass writes `crates/ai/generation.json` and appends the same object to
//! `training/history/<lane>.jsonl`. `--verify` (CI) recomputes and compares with the branch's
//! `generation.json`. The report (Markdown, the lane's pull request body) goes to stdout. Exit 0 on a
//! pass, 1 on a failed gate.
//!
//! Every run that plays its games, whatever its mode and whether it passes, also rewrites
//! `$JACKIOH_TRAINING_OUT/last-pass.md` when that directory is set (the training box sets it; CI does
//! not): the latest pass, in Markdown, which `training/loop.sh` puts at the top of the lane's draft
//! pull request. It counts the games the candidate won against the parent, the AI on `main`, as p1,
//! as p2 and in all, and shows each game as a square of a grid (`last_pass`).
//!
//! The unban lane has nothing to do while the parent keeps no card out of its decks (generation 8
//! lifted the last shadow ban, #668, and generation 9's sweep of the Meditative set banned cards
//! again, #732): no candidate has strictly fewer bans than none, so the gate cannot pass, and against
//! such a parent `promote --lane unban` says so and exits 1, with any flag, before it plays a game or
//! writes anything (`lane_done`).
//!
//! Because the seeds come from the committed AI, a promotion (and a verification) refuses to run on
//! a `crates/ai/src` with uncommitted changes: they would be measured on another AI's seeds. A dry
//! run of an unpaired gate measures them anyway, on HEAD's seeds, and says so; a paired gate's
//! hold-out deals are read only by the committed AI they are named for, so its dry run refuses them
//! too (R850001, `dirty_refusal`).

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::ValueEnum;
use jackioh_engine::{
    GameResult, PlayerId, TRAINING_IMPROVE, TRAINING_LEARN, TRAINING_UNBAN, TrainingGate, Winner,
};
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
    /// Beat the parent: ≥ 220 of 400 games against it, 200 deals each played twice (R850000).
    Improve,
    /// Play well with fewer shadow bans: strictly fewer than the parent's and ≥ 60 of 100 against it.
    /// Idle while the parent has none (`lane_done`).
    Unban,
    /// The learned evaluation, retrained by a script (R1012000): improve's bar on its own deals.
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

    /// The lane's gate (CLAUDE.md rule 9: the numbers live in `crates/engine/src/config.rs`).
    pub(crate) fn gate(self) -> TrainingGate {
        match self {
            Lane::Improve => TRAINING_IMPROVE,
            Lane::Unban => TRAINING_UNBAN,
            Lane::Learn => TRAINING_LEARN,
        }
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

/// What a promotion run counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GateCounts {
    pub games: i32,
    pub vs_parent: i32,
    pub shadow_ban: usize,
    pub parent_shadow_ban: usize,
}

/// The rules of the lane's gate that `counts` fails, in the table's order; empty when it promotes.
pub(crate) fn gate_failures(lane: Lane, counts: &GateCounts) -> Vec<String> {
    let gate = lane.gate();
    let mut failures = Vec::new();
    if counts.vs_parent < gate.vs_parent {
        failures.push(format!(
            "vs parent: {}/{} wins, {} needed",
            counts.vs_parent, counts.games, gate.vs_parent
        ));
    }
    if lane == Lane::Unban && counts.shadow_ban >= counts.parent_shadow_ban {
        failures.push(format!(
            "shadow bans: {}, which is not strictly fewer than the parent's {}",
            counts.shadow_ban, counts.parent_shadow_ban
        ));
    }
    failures
}

/// The refusal to give when the lane has nothing to do, else `None`. The unban lane has nothing to do
/// while its parent, the AI on `main`, keeps no card out of its own decks: no candidate has strictly
/// fewer shadow bans than none, so its gate fails whatever the games say, and `run` plays none
/// (#668). It reads the parent's list, not this build's `SHADOW_BAN`: a candidate with none against a
/// parent with one is the promotion that emptied the table (generation 8's).
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

/// Why a run in `mode` refuses a `crates/ai/src` with uncommitted changes, else `None`. A promotion
/// or a verification would measure them on another AI's seeds. A paired gate's dry run would read
/// its hold-out deals with an AI they are not named for (R850001), so only an unpaired gate's dry run
/// measures them, on HEAD's seeds.
pub(crate) fn dirty_refusal(lane: Lane, mode: &str) -> Option<String> {
    if mode != "dry run" {
        return Some(format!(
            "promote: {AI_SOURCE} has uncommitted changes. The gate's seeds come from `git rev-parse \
             HEAD:{AI_SOURCE}`, so commit the change first, then run the {mode} again (training/README.md)."
        ));
    }
    lane.gate().paired.then(|| {
        format!(
            "promote: {AI_SOURCE} has uncommitted changes, and the {} lane's gate plays hold-out deals \
             that only the committed AI they are named for reads (R850001). Screen the change on the \
             dev set (`cargo jackioh arena --set improve-dev`), or commit it to the stack and run the \
             gate on the commit (training/improve.md).",
            lane.as_str()
        )
    })
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
    let first = game_seed(lane, tree, 1);
    if lane.gate().paired {
        format!(
            "Deals `{first}` to `{}`, each played twice with the seats swapped: the candidate sits p1 \
             in a deal's first game (the odd games) and p2 in its second.",
            game_seed(lane, tree, arena::deal_of(games))
        )
    } else {
        format!(
            "Seeds `{first}` to `{}`; the candidate sits p1 on odd games.",
            game_seed(lane, tree, games)
        )
    }
}

/// R850000: a paired gate's edge, the candidate's wins less the parent's over each deal's two games,
/// in points of the games, with its 95% interval; `None` for an unpaired gate. `games` are (result,
/// the candidate's seat) in game order.
fn paired_text(lane: Lane, games: &[(Option<GameResult>, PlayerId)]) -> Option<String> {
    lane.gate().paired.then(|| {
        let (edge, low, high) = arena::paired_interval(&arena::deal_edges(games));
        format!(
            "Paired by deal, the candidate's wins less the parent's come to {:+.1} points of the games \
             (95% interval {:+.1} to {:+.1}).",
            edge * PERCENT,
            low * PERCENT,
            high * PERCENT
        )
    })
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
/// with one square per game. `games` are (result, the candidate's seat) in game order; `at` is when
/// the run ended; `dirty` says a dry run measured uncommitted changes on HEAD's seeds.
#[allow(clippy::too_many_arguments)]
fn last_pass(
    lane: Lane,
    mode: &str,
    at: &str,
    dirty: bool,
    record: &GenerationRecord,
    parent: &str,
    games: &[(Option<GameResult>, PlayerId)],
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
    let measured = if dirty {
        ", measuring uncommitted changes on HEAD's seeds,"
    } else {
        ""
    };
    let verdict = match (failures.is_empty(), mode) {
        (true, "promotion") => "clears the gate and is promoted",
        (true, _) => "clears the gate",
        (false, _) => "falls short of the gate",
    };
    let seeds = seeds_text(lane, &record.tree, all.games() as i32);
    let (low, high) = arena::wilson_interval(all.won, all.games());

    let mut out = format!(
        "## Latest pass: beat the AI on `main` {} times in {}, {} needed\n\n",
        all.won,
        all.games(),
        gate.vs_parent
    );
    out.push_str(&format!(
        "{run} of the `{}` lane's gate at {at}{measured} against the parent, {parent}: it {verdict}. \
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
        "{} wins put its true win rate at {} (95% interval); the gate needs {}. Shadow bans: {} \
         (the parent's {}){ban_rule}.\n\n",
        all.won,
        interval_text(low, high),
        gate.vs_parent,
        record.shadow_ban,
        record.parent_shadow_ban
    ));
    if let Some(paired) = paired_text(lane, games) {
        out.push_str(&format!("{paired}\n\n"));
    }
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
    out.push_str(
        "| Opponent | Wins | Needs | 95% interval | Draws | No result |\n|---|---|---|---|---|---|\n",
    );
    out.push_str(&format!(
        "| parent: {parent_name} | {} | {} | {} | {parent_draws} | {parent_aborted} |\n\n",
        record.vs_parent,
        gate.vs_parent,
        interval_text(low, high)
    ));
    if let Some(paired) = paired_text(lane, &results_of(vs_parent)) {
        out.push_str(&format!("{paired}\n\n"));
    }
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
         without a result. No game is played against random (R732001).\n\n"
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

    let dirty = ai_source_dirty(&repo)?;
    if dirty {
        if let Some(refusal) = dirty_refusal(args.lane, mode) {
            bail!("{refusal}");
        }
        eprintln!(
            "promote: {AI_SOURCE} has uncommitted changes; this dry run measures them on HEAD's seeds, and \
             the promotion will play other seeds once they are committed"
        );
    }

    let tree = ai_tree(&repo)?;
    let candidate = arena::entrant(&AgentSpec::SelfAi)?;
    let parent_generation = parent
        .generation
        .with_context(|| format!("promote: {} reported no generation", args.parent_bin.display()))?;

    let gate = args.lane.gate();
    let seed_of = |k: i32| game_seed(args.lane, &tree, k);
    let games = arena::setups(&candidate, &parent, gate.games, gate.paired, seed_of)?;
    if gate.paired {
        eprintln!(
            "promote: {} lane, {} games against gen {parent_generation}: deals {} to {}, each played twice with the seats swapped",
            args.lane.as_str(),
            gate.games,
            game_seed(args.lane, &tree, 1),
            game_seed(args.lane, &tree, arena::deal_of(gate.games))
        );
    } else {
        eprintln!(
            "promote: {} lane, {} games against gen {parent_generation}, on seeds {} to {}",
            args.lane.as_str(),
            gate.games,
            game_seed(args.lane, &tree, 1),
            game_seed(args.lane, &tree, gate.games)
        );
    }
    let outcomes = arena::play_games(&games)?;
    if let Some(path) = arena::records_path(None) {
        arena::append_records(&path, &arena::records_of(&outcomes)?)?;
    }
    let vs_parent = &outcomes[..];

    let counts = GateCounts {
        games: gate.games,
        vs_parent: wins_of(&results_of(vs_parent)),
        shadow_ban: candidate.shadow_ban.len(),
        parent_shadow_ban: parent.shadow_ban.len(),
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
    let failures = gate_failures(args.lane, &counts);
    print!(
        "{}",
        report(args.lane, mode, &record, &counts, &parent, vs_parent, &failures)?
    );
    if let Some(dir) = arena::training_out(None) {
        let pass = last_pass(
            args.lane,
            mode,
            &utc_minute_of(arena::unix_now()),
            dirty,
            &record,
            &parent_name(&parent, &record),
            &results_of(vs_parent),
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
    use jackioh_engine::{GameOverReason, Winner};
    use std::process::Command;

    fn counts(lane: Lane, vs_parent: i32, shadow_ban: usize, parent_shadow_ban: usize) -> GateCounts {
        GateCounts {
            games: lane.gate().games,
            vs_parent,
            shadow_ban,
            parent_shadow_ban,
        }
    }

    #[test]
    fn r850000_the_improve_gate_needs_220_of_400_paired_games() {
        let improve = |vs_parent, shadow_ban| counts(Lane::Improve, vs_parent, shadow_ban, 11);
        assert!(gate_failures(Lane::Improve, &improve(220, 11)).is_empty());
        assert!(gate_failures(Lane::Improve, &improve(400, 11)).is_empty());
        assert_eq!(
            gate_failures(Lane::Improve, &improve(219, 11)),
            vec!["vs parent: 219/400 wins, 220 needed".to_string()]
        );
        // The improve lane has no shadow-ban rule: more bans than the parent's still promote.
        assert!(gate_failures(Lane::Improve, &improve(220, 12)).is_empty());
        assert_eq!(
            TRAINING_IMPROVE,
            TrainingGate {
                games: 400,
                paired: true,
                vs_parent: 220
            }
        );
        // Two games to a deal, and the unban lane's gate as it was before.
        assert_eq!(TRAINING_IMPROVE.games % 2, 0);
        assert_eq!(
            TRAINING_UNBAN,
            TrainingGate {
                games: 100,
                paired: false,
                vs_parent: 60
            }
        );
    }

    #[test]
    fn r1012000_the_learn_lane_gates_like_improve_on_hold_out_deals_of_its_own() {
        let learn = |vs_parent| counts(Lane::Learn, vs_parent, 0, 0);
        assert_eq!(TRAINING_LEARN, TRAINING_IMPROVE);
        assert!(gate_failures(Lane::Learn, &learn(220)).is_empty());
        assert_eq!(
            gate_failures(Lane::Learn, &learn(219)),
            vec!["vs parent: 219/400 wins, 220 needed".to_string()]
        );
        // Its deals are its own: `learn:<tree>:<d>`, never a screen's (R850001), never an export's.
        assert_eq!(game_seed(Lane::Learn, "abc", 7), "learn:abc:7");
        assert!(is_gate_seed_base("learn") && is_gate_seed_base("learn:abc"));
        assert!(!is_gate_seed_base("learned-dev"));
        // A paired gate's dry run reads only the committed AI.
        assert!(dirty_refusal(Lane::Learn, "dry run").is_some());
    }

    #[test]
    fn r732001_the_unban_gate_needs_strictly_fewer_bans_and_60_against_the_parent() {
        let unban = |vs_parent, shadow_ban| counts(Lane::Unban, vs_parent, shadow_ban, 11);
        assert!(gate_failures(Lane::Unban, &unban(60, 10)).is_empty());
        assert!(gate_failures(Lane::Unban, &unban(80, 0)).is_empty());
        // Strictly fewer: as many bans as the parent is a failure, more is too.
        assert_eq!(gate_failures(Lane::Unban, &unban(60, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &unban(60, 12)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &unban(59, 10)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &unban(59, 11)).len(), 2);
        // 60 against the parent passes the unban lane and fails the improve lane.
        assert!(!gate_failures(Lane::Improve, &unban(60, 10)).is_empty());
        assert_eq!(TRAINING_UNBAN.vs_parent, 60);
        assert_eq!(TRAINING_UNBAN.games, 100);
    }

    #[test]
    fn r850001_a_dry_run_of_the_paired_gate_measures_only_the_committed_ai() {
        // The improve lane's hold-out deals are read by the committed AI alone, dry run or not.
        let held_out = dirty_refusal(Lane::Improve, "dry run").expect("the paired gate refuses");
        assert!(held_out.contains("R850001"), "{held_out}");
        assert!(held_out.contains("arena --set improve-dev"), "{held_out}");
        assert!(dirty_refusal(Lane::Improve, "verify").is_some());
        assert!(dirty_refusal(Lane::Improve, "promotion").is_some());
        // The unban lane's unpaired gate still measures uncommitted changes in a dry run.
        assert!(dirty_refusal(Lane::Unban, "promotion").is_some());
        assert!(dirty_refusal(Lane::Unban, "verify").is_some());
        assert_eq!(dirty_refusal(Lane::Unban, "dry run"), None);
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
        assert!(!gate_failures(Lane::Unban, &counts(Lane::Unban, 100, 0, 0)).is_empty());
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

        // 219 wins and 181 draws against the parent is a failed gate: the draws are not wins.
        let mut series: Vec<(Option<GameResult>, PlayerId)> =
            (0..219).map(|_| (result(Winner::P1), PlayerId::P1)).collect();
        series.extend((0..181).map(|_| (draw, PlayerId::P1)));
        let measured = counts(Lane::Improve, wins_of(&series), 11, 11);
        assert_eq!(measured.vs_parent, 219);
        assert_eq!(gate_failures(Lane::Improve, &measured).len(), 1);
    }

    #[test]
    fn the_report_reads_the_gates_bar_as_an_interval() {
        // 220 of 400, the improve lane's bar, is a true win rate anywhere from about 50% to 60%.
        let (low, high) = arena::wilson_interval(220, 400);
        assert_eq!(interval_text(low, high), "50–60%");
        // 60 of 100, the unban lane's, anywhere from about 50% to 69%.
        let (low, high) = arena::wilson_interval(60, 100);
        assert_eq!(interval_text(low, high), "50–69%");
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
        let measured = GateCounts {
            games: 12,
            vs_parent: wins_of(&games),
            shadow_ban: 11,
            parent_shadow_ban: 11,
        };
        assert_eq!(measured.vs_parent, 6);
        let failures = gate_failures(Lane::Improve, &measured);
        let pass = last_pass(
            Lane::Improve,
            "dry run",
            "2026-10-14 03:12 UTC",
            false,
            &sample_record(),
            "gen 8 (improve), `main` at `0123abc`",
            &games,
            &failures,
        );
        assert!(
            pass.starts_with("## Latest pass: beat the AI on `main` 6 times in 12, 220 needed\n"),
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
            pass.contains("Short of the gate:\n\n- vs parent: 6/12 wins, 220 needed\n"),
            "{pass}"
        );
        // One square per game, ten to a row, each row labelled with its games.
        assert!(
            pass.contains("```text\n 1–10  🟩🟩🟥🟥⬜⬛🟩🟩🟩🟥\n11–12  🟥🟩\n```\n"),
            "{pass}"
        );
    }

    #[test]
    fn the_latest_pass_names_its_run_and_what_the_unban_lane_needs() {
        // A full gate of 100: ten rows, their labels right-aligned.
        let won_all = seated(
            &(0..TRAINING_UNBAN.games)
                .map(|k| Some(if k % 2 == 0 { Winner::P1 } else { Winner::P2 }))
                .collect::<Vec<_>>(),
        );
        let mut record = sample_record();
        record.lane = "unban".to_string();
        record.shadow_ban = 10;
        let promoted = last_pass(
            Lane::Unban,
            "promotion",
            "2026-10-14 03:12 UTC",
            false,
            &record,
            "gen 8 (unban)",
            &won_all,
            &[],
        );
        assert!(promoted.starts_with("## Latest pass: beat the AI on `main` 100 times in 100, 60 needed\n"));
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
        // The unban gate is unpaired: its games have a seed each, and it reads no edge.
        assert!(
            promoted.contains(
                "Seeds `unban:4b825dc:1` to `unban:4b825dc:100`; the candidate sits p1 on odd games."
            ),
            "{promoted}"
        );
        assert!(!promoted.contains("Paired by deal"), "{promoted}");
        assert!(
            promoted.contains("\n  1–10  🟩🟩🟩🟩🟩🟩🟩🟩🟩🟩\n"),
            "{promoted}"
        );
        assert!(
            promoted.contains("\n91–100  🟩🟩🟩🟩🟩🟩🟩🟩🟩🟩\n```\n"),
            "{promoted}"
        );

        // A dry run on uncommitted changes says it measured them on HEAD's seeds; a re-check names itself.
        let dirty = last_pass(
            Lane::Unban,
            "dry run",
            "at",
            true,
            &record,
            "gen 8",
            &won_all,
            &[],
        );
        assert!(
            dirty.contains("measuring uncommitted changes on HEAD's seeds,"),
            "{dirty}"
        );
        assert!(dirty.contains("it clears the gate."), "{dirty}");
        let verify = last_pass(
            Lane::Unban,
            "verify",
            "at",
            false,
            &record,
            "gen 8",
            &won_all,
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
    fn the_record_serialises_in_surface_order() {
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

        // Generation 0's record (the port) never verifies as a promotion. Its text, not the live
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
}
