//! `cargo jackioh sweep` (R186, R390; SURFACE §12): the run that decides the AI's shadow ban, in two
//! passes. Port of `packages/ai/scripts/sweep.ts` (`pnpm ai:sweep`).
//!
//! Pass 1: for every non-token card of every set and every tier in AI_SWEEP.tiers, `sweepCard` forces
//! the card into AI decks on that tier's handicap against the greedy baseline (AI_SWEEP.seedsPerCard
//! games each, at AI_GATE_BUDGET). `atRiskIds` reads the at-risk cards off pass 1 (plus today's
//! SHADOW_BAN and SHADOW_WATCH). Pass 2: `sweepAtRisk` forces each at-risk card into
//! AI_SWEEP.seedsPerCardAtRisk more games per tier, with every at-risk card dealt more often as
//! filler. `sweepVerdict` joins a card's tiers and passes. This prints the flagged rows of both
//! passes, the at-risk cards, the suspects, the unswept cards, then the SHADOW_BAN and SHADOW_WATCH
//! entries and the header line for `crates/ai/src/shadow_ban.rs`. Those tables are never copied in by
//! hand or edited to taste: `--write` rewrites the header block and both tables of that file from the
//! reports of record (this output, committed under `training/history/`, oldest first), and `--check`
//! (and the `shadow_ban_rs_is_what_its_reports_of_record_dictate` test) fails with a line diff when
//! the file is anything else. Both read the one section of a report that `--report` prints for the
//! file, through one parser (`parse_block`).
//!
//! ```text
//! cargo jackioh sweep                              both passes over every card, then the report
//! cargo jackioh sweep core-011 classic-020         only these ids (pass 2: those of them at risk)
//! cargo jackioh sweep --json core-011 …            pass 1 only: one JSON SweepResult per line (a slice)
//! cargo jackioh sweep --pass2 a.jsonl,b.jsonl …    pass 2 from every slice's pass-1 lines: one JSON
//!                                                  SweepPass2 per line, for the at-risk ids listed
//!                                                  after the files (default every at-risk id); slice it too
//! cargo jackioh sweep --report a.jsonl p2.jsonl …  the report from pass-1 and pass-2 lines
//! cargo jackioh sweep --write [reports …]          rewrite shadow_ban.rs's header block and tables from
//!                                                  the reports of record (default: its header's)
//! cargo jackioh sweep --check [reports …]          fail with a diff unless the file is what they dictate
//! ```
//!
//! Pass 2 needs every slice's pass 1 first, because the at-risk list it boosts is the whole sweep's.
//! A slice writes each line as it lands, so one cut short keeps what it finished: rerun the rest.
//! Tooling, so it may read the clock (`Instant`, for decision timing), read files and write to the
//! console; `crates/ai` stays pure and receives the clock as `now`, and R977000's re-timer, which times
//! again a decision the clock found slow against the perf gate's yardstick, as `retime` (`still_slow`).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Component, Path};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use indexmap::IndexSet;
use serde_json::Value;

use jackioh_ai::{
    AI_GATE_BUDGET, AI_SWEEP, SHADOW_BAN, SHADOW_UNBANNED, SHADOW_WATCH, SlowDecision, SweepFlag,
    SweepOptions, SweepPass2, SweepResult, SweepStats, at_risk_ids, pass2_keep_out, pass2_stats,
    sweep_at_risk, sweep_card, sweep_verdict,
};
use jackioh_engine::catalog::{CatalogQueryArgs, query};
use jackioh_engine::testkit::preview_every_set;
use jackioh_engine::wire::SetName;

use crate::arena::utc_date;
use crate::gate::{MS_PER_SECOND, literal, time_decision, yardstick_ms};
use crate::patches::repo_root;

/// The mean evaluate change per play, and each card's seconds, printed to this many decimals.
const DECIMALS: usize = 1;

/// Enough decimals to hold the exact expansion of any f64 whose value can be a rounding tie
/// (`to_fixed`): a tie at a few decimals is a multiple of a power of two's reciprocal, whose
/// expansion ends long before this.
const EXACT_DIGITS: usize = 40;

/// The file `--write` rewrites and `--check` holds (R186), from the repository root.
const SHADOW_BAN_RS: &str = "crates/ai/src/shadow_ban.rs";

/// The heading of a report's section for that file: `block_lines` prints it with `##`, the committed
/// reports hold it with `###`.
const BLOCK_HEADING: &str = "For crates/ai/src/shadow_ban.rs";

/// The line that opens `shadow_ban.rs`'s header block, the part of it `--write` owns.
const HEADER_BEGIN: &str =
    "// BEGIN `cargo jackioh sweep --write`: the reports of record (R186), oldest first. Never edit by hand.";

/// The line that closes that block.
const HEADER_END: &str = "// END `cargo jackioh sweep --write`";

/// Before each report's header line in the block: the report's path from the repository root.
const REPORT_PREFIX: &str = "// Report of record: ";

/// What a report's header line starts with (the block holds it as it printed).
const SWEEP_PREFIX: &str = "// Sweep of record: ";

/// The line that opens `SHADOW_BAN` in `shadow_ban.rs`.
const BAN_OPEN: &str = "pub const SHADOW_BAN: &[(&str, &str)] = &[";

/// The line that opens `SHADOW_WATCH` in `shadow_ban.rs`.
const WATCH_OPEN: &str = "pub const SHADOW_WATCH: &[(&str, &str)] = &[";

/// The line that closes a table written over several lines.
const TABLE_CLOSE: &str = "];";

/// R732000, R1186000: the sets whose cards the shadow ban holds.
const BAN_SETS: [SetName; 2] = [SetName::Meditative, SetName::MeditativePlus];

/// R732000, R1186000: the shadow ban holds at most each of `BAN_SETS`' non-token cards divided by
/// this: half.
const BAN_DIVISOR: usize = 2;

/// `cargo jackioh sweep …`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Pass 1 only: one JSON SweepResult per line, as each lands (a slice of the sweep).
    #[arg(long)]
    pub json: bool,
    /// Pass 2 from every slice's pass-1 lines in these comma-separated files: one JSON SweepPass2 per
    /// line, for the at-risk ids listed (default every at-risk id).
    #[arg(long, value_name = "FILES")]
    pub pass2: Option<String>,
    /// The report from the pass-1 and pass-2 lines in these files.
    #[arg(long, num_args = 1.., value_name = "FILE")]
    pub report: Option<Vec<String>>,
    /// Rewrite crates/ai/src/shadow_ban.rs's header block, SHADOW_BAN and SHADOW_WATCH from these reports
    /// of record (`--report`'s output, oldest first; none: the ones its header block names).
    #[arg(long, num_args = 0.., value_name = "REPORT", conflicts_with = "check")]
    pub write: Option<Vec<String>>,
    /// Fail with a line diff when crates/ai/src/shadow_ban.rs is not what these reports dictate (none:
    /// its header block's).
    #[arg(long, num_args = 0.., value_name = "REPORT")]
    pub check: Option<Vec<String>>,
    /// Card ids to sweep (default every non-token card).
    pub ids: Vec<String>,
}

/// `Number.prototype.toFixed(digits)`: `x` rounded to `digits` decimals, an exact tie going to the
/// larger magnitude (JS picks "the larger n"), where Rust's `{:.N}` rounds an exact tie to even.
pub(crate) fn to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let exact = format!("{:.*}", EXACT_DIGITS, x.abs());
    let (whole, frac) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let tie =
        frac.as_bytes().get(digits) == Some(&b'5') && frac[digits + 1..].bytes().all(|byte| byte == b'0');
    if !tie {
        return format!("{sign}{:.*}", digits, x.abs());
    }
    // Round the kept digits up by one in their last place, carrying as far as it goes.
    let mut kept: Vec<u8> = format!("{whole}{}", &frac[..digits]).into_bytes();
    let mut at = kept.len();
    loop {
        if at == 0 {
            kept.insert(0, b'1');
            break;
        }
        at -= 1;
        if kept[at] == b'9' {
            kept[at] = b'0';
        } else {
            kept[at] += 1;
            break;
        }
    }
    let text = String::from_utf8(kept).unwrap_or_default();
    if digits == 0 {
        return format!("{sign}{text}");
    }
    let (int_part, frac_part) = text.split_at(text.len() - digits);
    format!("{sign}{int_part}.{frac_part}")
}

fn name_of(def_id: &str) -> String {
    jackioh_cards::CATALOG
        .get(def_id)
        .map_or_else(|| def_id.to_string(), |def| def.name.clone())
}

/// A sweep flag's name, as TS wrote it (`neverPlayed` …).
fn flag_names(flags: &[SweepFlag]) -> String {
    flags.iter().map(literal).collect::<Vec<_>>().join(", ")
}

fn mean_delta(stats: &SweepStats) -> String {
    if stats.eval_delta_count == 0 {
        return "n/a".to_string();
    }
    to_fixed(stats.eval_delta_sum / f64::from(stats.eval_delta_count), DECIMALS)
}

fn row(stats: &SweepStats, tier: &str, flags: &str) -> String {
    let cells = [
        stats.def_id.clone(),
        name_of(&stats.def_id),
        tier.to_string(),
        flags.to_string(),
        format!("{}/{}", stats.drawn_games, stats.games),
        stats.affordable_turns.to_string(),
        stats.plays.to_string(),
        stats.errors.to_string(),
        stats.timeouts.to_string(),
        stats.raw_timeouts.to_string(),
        mean_delta(stats),
    ]
    .map(|cell| format!(" {} ", cell.replace('|', "\\|")));
    format!("|{}|", cells.join("|"))
}

const TABLE_HEAD: [&str; 2] = [
    "| id | name | tier | flags | drawn/games | affordable turns | plays | errors | timeouts | raw timeouts | mean eval delta |",
    "|---|---|---|---|---|---|---|---|---|---|---|",
];

fn timed<T>(label: &str, run: impl FnOnce() -> T, describe: impl FnOnce(&T) -> String) -> T {
    let started = Instant::now();
    let result = run();
    let seconds = to_fixed(started.elapsed().as_secs_f64(), DECIMALS);
    eprintln!("[ai:sweep] {label}: {} ({seconds}s)", describe(&result));
    result
}

/// The clock `sweepCard` and `sweepAtRisk` time decisions with: milliseconds since `origin`
/// (TS: `performance.now`).
fn milliseconds_since(origin: Instant) -> impl Fn() -> f64 {
    move || origin.elapsed().as_secs_f64() * MS_PER_SECOND
}

/// R977000: whether a decision the sweep's clock found slow is still slower than
/// AI_SWEEP.decisionMs timed by the perf gate's method (`gate::time_decision`). The yardstick's
/// first run warms the caches and is no measurement, as in the perf gate (`warmed`). A yardstick
/// that cannot run keeps the clock's verdict.
fn still_slow(slow: &SlowDecision<'_>, warmed: &Cell<bool>) -> bool {
    let limit = f64::from(AI_SWEEP.decision_ms);
    if !warmed.replace(true)
        && let Err(error) = yardstick_ms()
    {
        eprintln!("[ai:sweep] the yardstick: {error:#}");
        return true;
    }
    match time_decision(slow.state, slow.seat, slow.rng, slow.budget, limit) {
        Ok(timing) => timing.ms > limit,
        Err(error) => {
            eprintln!("[ai:sweep] re-timing a slow decision: {error:#}");
            true
        }
    }
}

/// Each result is also handed to `emit` as it lands, so a slice cut short keeps what it finished.
fn pass1(ids: &[String], mut emit: impl FnMut(&SweepResult)) -> Vec<SweepResult> {
    let now = milliseconds_since(Instant::now());
    let warmed = Cell::new(false);
    let retime = |slow: &SlowDecision<'_>| still_slow(slow, &warmed);
    let mut results: Vec<SweepResult> = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        for tier in AI_SWEEP.tiers.iter().copied() {
            let result = timed(
                &format!("pass 1 {}/{} {id} {} @{tier}", index + 1, ids.len(), name_of(id)),
                || {
                    sweep_card(
                        id,
                        &SweepOptions {
                            seeds: None,
                            now: Some(&now),
                            retime: Some(&retime),
                            tier: Some(tier),
                        },
                    )
                },
                |r| {
                    if !r.flags.is_empty() {
                        flag_names(&r.flags)
                    } else if r.unswept {
                        "unswept".to_string()
                    } else {
                        "clean".to_string()
                    }
                },
            );
            emit(&result);
            results.push(result);
        }
    }
    results
}

fn pass2(
    ids: &[String],
    at_risk: &[String],
    keep_out: &[String],
    mut emit: impl FnMut(&SweepPass2),
) -> Vec<SweepPass2> {
    let now = milliseconds_since(Instant::now());
    let warmed = Cell::new(false);
    let retime = |slow: &SlowDecision<'_>| still_slow(slow, &warmed);
    let mut results: Vec<SweepPass2> = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        for tier in AI_SWEEP.tiers.iter().copied() {
            let result = timed(
                &format!("pass 2 {}/{} {id} {} @{tier}", index + 1, ids.len(), name_of(id)),
                || {
                    sweep_at_risk(
                        id,
                        at_risk,
                        keep_out,
                        &SweepOptions {
                            seeds: None,
                            now: Some(&now),
                            retime: Some(&retime),
                            tier: Some(tier),
                        },
                    )
                },
                |r| {
                    format!(
                        "{} at-risk card(s) dealt, {} suspect line(s)",
                        r.cards.len(),
                        r.suspects.len()
                    )
                },
            );
            emit(&result);
            results.push(result);
        }
    }
    results
}

/// `[...new Set(xs)].sort()`.
fn sorted_unique(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut unique: Vec<String> = ids.into_iter().collect::<IndexSet<_>>().into_iter().collect();
    unique.sort();
    unique
}

/// The section of a report that is for `shadow_ban.rs`: the header line and the rows of both tables, as
/// (id, reason) pairs in the order the report lists them.
#[derive(Debug, Clone, PartialEq)]
struct Block {
    header: String,
    ban: Vec<(String, String)>,
    watch: Vec<(String, String)>,
}

/// `date` is the run's date for `shadow_ban.rs`'s header.
fn report(first: &[SweepResult], second: &[SweepPass2], elapsed: Option<i64>, date: &str) -> String {
    let ids = sorted_unique(first.iter().map(|result| result.stats.def_id.clone()));
    let at_risk = at_risk_ids(first, SHADOW_BAN, SHADOW_WATCH);
    let forced = sorted_unique(second.iter().map(|result| result.forced.clone()));
    let verdicts: Vec<_> = sorted_unique(ids.iter().chain(forced.iter()).cloned())
        .iter()
        .map(|id| {
            let own: Vec<SweepResult> = first
                .iter()
                .filter(|result| &result.stats.def_id == id)
                .cloned()
                .collect();
            sweep_verdict(&own, second)
        })
        .collect();
    let banned: Vec<_> = verdicts
        .iter()
        .filter(|verdict| verdict.reason.is_some())
        .collect();
    let watched: Vec<_> = verdicts
        .iter()
        .filter(|verdict| verdict.watch.is_some())
        .collect();
    let unswept: Vec<_> = verdicts.iter().filter(|verdict| verdict.unswept).collect();
    let suspects: Vec<_> = second.iter().flat_map(|result| result.suspects.iter()).collect();

    let budget = serde_json::to_string(&AI_GATE_BUDGET).unwrap_or_default();
    let tiers = AI_SWEEP
        .tiers
        .iter()
        .map(|tier| tier.as_str())
        .collect::<Vec<_>>()
        .join(" and ");
    let passes = format!(
        "pass 1 over {} card(s) at {tiers}, {} seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over {} at-risk card(s), {} seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×{}), budget AI_GATE_BUDGET {budget}",
        ids.len(),
        AI_SWEEP.seeds_per_card,
        forced.len(),
        AI_SWEEP.seeds_per_card_at_risk,
        AI_SWEEP.at_risk_boost,
    );

    let mut out: Vec<String> = Vec::new();
    out.push("# AI shadow-ban sweep".to_string());
    out.push(String::new());
    out.push(format!(
        "Run {date} (UTC): {passes}; {} banned, {} watched, {} unswept, {} suspect line(s){}.",
        banned.len(),
        watched.len(),
        unswept.len(),
        suspects.len(),
        elapsed.map_or_else(String::new, |seconds| format!(", {seconds}s"))
    ));
    out.push(String::new());
    out.push(
        "## Pass 1 flags (a `neverPlayed`, `selfHarm` or `selfKill` here only puts a card at risk)"
            .to_string(),
    );
    out.push(String::new());
    let flagged: Vec<&SweepResult> = first.iter().filter(|result| !result.flags.is_empty()).collect();
    if flagged.is_empty() {
        out.push("No card was flagged at any tier.".to_string());
    } else {
        out.extend(TABLE_HEAD.iter().map(|line| (*line).to_string()));
        out.extend(
            flagged
                .iter()
                .map(|result| row(&result.stats, result.tier.as_str(), &flag_names(&result.flags))),
        );
    }
    out.push(String::new());
    out.push(format!(
        "## At risk ({}): pass 1 at half strength, SHADOW_BAN and SHADOW_WATCH",
        at_risk.len()
    ));
    out.push(String::new());
    out.push(if at_risk.is_empty() {
        "None.".to_string()
    } else {
        at_risk
            .iter()
            .map(|id| format!("{id} {}", name_of(id)))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let missing: Vec<&String> = at_risk
        .iter()
        .filter(|id| ids.contains(id) && !forced.contains(id))
        .collect();
    if !missing.is_empty() {
        out.push(String::new());
        out.push(format!(
            "Not swept in pass 2 (no evidence, no ban for neverPlayed, selfHarm or selfKill): {}",
            missing
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push(String::new());
    out.push("## Pass 2: every at-risk card over every game it was dealt in, forced or filler".to_string());
    out.push(String::new());
    if second.is_empty() {
        out.push("Pass 2 did not run.".to_string());
    } else {
        out.extend(TABLE_HEAD.iter().map(|line| (*line).to_string()));
        let dealt = sorted_unique(
            second
                .iter()
                .flat_map(|result| result.cards.iter().map(|card| card.def_id.clone())),
        );
        for id in &dealt {
            let verdict = verdicts.iter().find(|entry| &entry.def_id == id);
            for tier in AI_SWEEP.tiers.iter().copied() {
                let total = pass2_stats(second, id, tier);
                if total.games > 0 {
                    let flags = match verdict {
                        Some(verdict) if verdict.reason.is_some() => flag_names(&verdict.flags),
                        _ => "cleared".to_string(),
                    };
                    out.push(row(&total, tier.as_str(), &flags));
                }
            }
        }
    }
    out.push(String::new());
    out.push(
        "## Suspects (an error or timeout in a pass-2 game that also dealt this at-risk card as filler)"
            .to_string(),
    );
    out.push(String::new());
    out.push(if suspects.is_empty() {
        "None.".to_string()
    } else {
        suspects
            .iter()
            .map(|s| {
                format!(
                    "- suspect: {} {}: {} error(s), {} timeout(s) in {} (forced {} {}); banned only if its own games repeat it",
                    s.def_id,
                    name_of(&s.def_id),
                    s.errors,
                    s.timeouts,
                    s.seed,
                    s.forced,
                    name_of(&s.forced)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    });
    out.push(String::new());
    out.push("## Unswept (never affordable at any tier: no evidence either way)".to_string());
    out.push(String::new());
    out.push(if unswept.is_empty() {
        "None.".to_string()
    } else {
        unswept
            .iter()
            .map(|verdict| format!("- {} {}", verdict.def_id, name_of(&verdict.def_id)))
            .collect::<Vec<_>>()
            .join("\n")
    });
    out.push(String::new());
    let block = Block {
        header: format!("{SWEEP_PREFIX}{date} (UTC), `cargo jackioh sweep`, {passes}."),
        ban: banned
            .iter()
            .map(|verdict| (verdict.def_id.clone(), verdict.reason.clone().unwrap_or_default()))
            .collect(),
        watch: watched
            .iter()
            .map(|verdict| (verdict.def_id.clone(), verdict.watch.clone().unwrap_or_default()))
            .collect(),
    };
    out.extend(block_lines(&block));
    format!("{}\n", out.join("\n"))
}

/// `JSON.stringify(text)`: a quoted string with JSON's escapes, which Rust's string literals read the
/// same for every character a card id or a ban reason holds.
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}

/// The report's section for `shadow_ban.rs`, as `report` prints it.
fn block_lines(block: &Block) -> Vec<String> {
    let mut out = vec![
        format!("## {BLOCK_HEADING}"),
        String::new(),
        "Header line:".to_string(),
        String::new(),
        block.header.clone(),
    ];
    for (label, rows) in [
        ("SHADOW_BAN entries:", &block.ban),
        ("SHADOW_WATCH entries:", &block.watch),
    ] {
        out.push(String::new());
        out.push(label.to_string());
        out.push(String::new());
        out.push("```rust".to_string());
        out.extend(
            rows.iter()
                .map(|(id, reason)| format!("    ({}, {}),", json_string(id), json_string(reason))),
        );
        out.push("```".to_string());
    }
    out
}

/// Reads only the section `block_lines` writes, never the report's prose (V23): the first heading named
/// `BLOCK_HEADING` at any depth (`##` as printed, `###` as committed), then, each after the last, the
/// header line, the `SHADOW_BAN` rows and the `SHADOW_WATCH` rows.
fn parse_block(text: &str) -> Result<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let heading = lines
        .iter()
        .position(|line| line.starts_with('#') && line.trim_start_matches('#').trim() == BLOCK_HEADING)
        .with_context(|| format!("no `{BLOCK_HEADING}` heading"))?;
    let lines = &lines[heading + 1..];
    let label = lines
        .iter()
        .position(|line| *line == "Header line:")
        .context("no `Header line:` after the heading")?;
    let lines = &lines[label + 1..];
    let header = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .context("no header line after `Header line:`")?;
    if !header.starts_with(SWEEP_PREFIX) {
        bail!("the header line does not start with `{SWEEP_PREFIX}`: {header}");
    }
    let (ban, used) = parse_table(lines, "SHADOW_BAN entries:")?;
    let (watch, _) = parse_table(&lines[used..], "SHADOW_WATCH entries:")?;
    Ok(Block {
        header: (*header).to_string(),
        ban,
        watch,
    })
}

/// The rows of the table `label` names in `lines`, and how many lines it took, its closing fence last.
fn parse_table(lines: &[&str], label: &str) -> Result<(Vec<(String, String)>, usize)> {
    let at = lines
        .iter()
        .position(|line| *line == label)
        .with_context(|| format!("no `{label}` line"))?;
    let open = lines[at..]
        .iter()
        .position(|line| *line == "```rust")
        .with_context(|| format!("no ```rust fence after `{label}`"))?
        + at
        + 1;
    let close = lines[open..]
        .iter()
        .position(|line| *line == "```")
        .with_context(|| format!("no closing fence after `{label}`"))?
        + open;
    let rows = lines[open..close]
        .iter()
        .map(|line| parse_row(line))
        .collect::<Result<Vec<_>>>()
        .with_context(|| format!("reading the `{label}` rows"))?;
    Ok((rows, close + 1))
}

/// One `    ("<id>", "<reason>"),` row, its two strings JSON-escaped.
fn parse_row(line: &str) -> Result<(String, String)> {
    let inner = line
        .strip_prefix("    (")
        .and_then(|rest| rest.strip_suffix("),"))
        .with_context(|| format!("not a `    (\"<id>\", \"<reason>\"),` row: {line}"))?;
    serde_json::from_str(&format!("[{inner}]")).with_context(|| format!("not an id and a reason: {line}"))
}

/// What the reports of record dictate for `shadow_ban.rs`: each report's path and header line, oldest
/// first, and the rows of both tables.
#[derive(Debug, Clone, PartialEq)]
struct Record {
    reports: Vec<(String, String)>,
    ban: Vec<(String, String)>,
    watch: Vec<(String, String)>,
}

/// The tables the reports of record (path and block, oldest first) dictate. A card takes the row of
/// the latest report that lists it: a ban takes it off `SHADOW_WATCH` and a watch row off
/// `SHADOW_BAN` (R600). Each sweep of record covered a set alone, so each keeps the earlier reports'
/// rows for the other sets: the 2026-10-12 sweep the Meditative set (R732000), #1186's the
/// Meditative+ set (R1186000). `SHADOW_BAN` holds `BAN_SETS`' cards only, an older sweep's bans of
/// the earlier sets left out (R732000), less `unbanned`, the unban lane's removals (R732001). Refuses
/// an unsorted list, a ban in the newest report outside `BAN_SETS`, more than half of a set banned
/// (R732000, R1186000) and an `unbanned` id no report of record bans.
fn compose(reports: &[(String, Block)], unbanned: &[&str]) -> Result<Record> {
    let Some((newest_path, newest)) = reports.last() else {
        bail!("no report of record");
    };
    for (path, block) in reports {
        ascending(
            block.ban.iter().map(|(id, _)| id.as_str()),
            &format!("{path}'s SHADOW_BAN entries"),
        )?;
        ascending(
            block.watch.iter().map(|(id, _)| id.as_str()),
            &format!("{path}'s SHADOW_WATCH entries"),
        )?;
    }
    ascending(unbanned.iter().copied(), "SHADOW_UNBANNED")?;
    for (id, _) in &newest.ban {
        in_ban_sets(id, newest_path)?;
    }
    let mut ban: BTreeMap<String, String> = BTreeMap::new();
    let mut watch: BTreeMap<String, String> = BTreeMap::new();
    for (_, block) in reports {
        for (id, why) in &block.ban {
            watch.remove(id);
            ban.insert(id.clone(), why.clone());
        }
        for (id, why) in &block.watch {
            ban.remove(id);
            watch.insert(id.clone(), why.clone());
        }
    }
    ban.retain(|id, _| {
        jackioh_cards::CATALOG
            .get(id.as_str())
            .is_some_and(|def| BAN_SETS.contains(&def.set))
    });
    at_most_half(&ban)?;
    if let Some(id) = unbanned.iter().find(|id| !ban.contains_key(**id)) {
        bail!("SHADOW_UNBANNED names {id}, which no report of record bans");
    }
    Ok(Record {
        reports: reports
            .iter()
            .map(|(path, block)| (path.clone(), block.header.clone()))
            .collect(),
        ban: ban
            .into_iter()
            .filter(|(id, _)| !unbanned.contains(&id.as_str()))
            .collect(),
        watch: watch.into_iter().collect(),
    })
}

/// Fails on the first pair of `ids` that is not in strictly ascending order.
fn ascending<'a>(ids: impl IntoIterator<Item = &'a str>, what: &str) -> Result<()> {
    let ids: Vec<&str> = ids.into_iter().collect();
    match ids.windows(2).find(|pair| pair[0] >= pair[1]) {
        Some(pair) => bail!("{what} are not sorted by id: {} before {}", pair[0], pair[1]),
        None => Ok(()),
    }
}

/// R732000, R1186000: a card `path` bans is one of `BAN_SETS`'.
fn in_ban_sets(id: &str, path: &str) -> Result<()> {
    match jackioh_cards::CATALOG.get(id) {
        Some(def) if BAN_SETS.contains(&def.set) => Ok(()),
        other => bail!(
            "{path} bans {id}, {}: the shadow ban holds Meditative and Meditative+ cards only (R732000, R1186000)",
            other.map_or_else(
                || "no catalog card".to_string(),
                |def| format!("a {:?} card", def.set)
            )
        ),
    }
}

/// R732000, R1186000: `ban` holds no more than half of each of `BAN_SETS`' non-token cards.
fn at_most_half(ban: &BTreeMap<String, String>) -> Result<()> {
    for set in BAN_SETS {
        let cards = jackioh_cards::CATALOG
            .values()
            .filter(|def| def.set == set && !def.token)
            .count();
        let banned = ban
            .keys()
            .filter(|id| {
                jackioh_cards::CATALOG
                    .get(id.as_str())
                    .is_some_and(|def| def.set == set)
            })
            .count();
        let most = cards / BAN_DIVISOR;
        if banned > most {
            bail!(
                "the reports of record ban {banned} {set:?} cards, more than half of its {cards} (R732000, R1186000: at most {most})"
            );
        }
    }
    Ok(())
}

/// The indices of `shadow_ban.rs`'s header block's opening and closing lines.
fn header_span(lines: &[String]) -> Result<(usize, usize)> {
    let begin = lines
        .iter()
        .position(|line| line == HEADER_BEGIN)
        .with_context(|| format!("{SHADOW_BAN_RS} has no `{HEADER_BEGIN}` line"))?;
    let end = lines[begin..]
        .iter()
        .position(|line| line == HEADER_END)
        .map(|offset| offset + begin)
        .with_context(|| format!("{SHADOW_BAN_RS} has no `{HEADER_END}` line after its begin line"))?;
    Ok((begin, end))
}

/// The indices of the first and last line of the table `open` opens: one line when it is empty.
fn table_span(lines: &[String], open: &str) -> Result<(usize, usize)> {
    let from = lines
        .iter()
        .position(|line| line.starts_with(open))
        .with_context(|| format!("{SHADOW_BAN_RS} has no `{open}` line"))?;
    if lines[from] == format!("{open}{TABLE_CLOSE}") {
        return Ok((from, from));
    }
    let to = lines[from..]
        .iter()
        .position(|line| line == TABLE_CLOSE)
        .map(|offset| offset + from)
        .with_context(|| format!("{SHADOW_BAN_RS}'s `{open}` table is never closed"))?;
    Ok((from, to))
}

/// A table as rustfmt writes it: one row over four lines (every row is wider than rustfmt's one-line
/// limit), an empty table on its opening line.
fn table_lines(open: &str, rows: &[(String, String)]) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("{open}{TABLE_CLOSE}")];
    }
    let mut out = vec![open.to_string()];
    for (id, reason) in rows {
        out.push("    (".to_string());
        out.push(format!("        {},", json_string(id)));
        out.push(format!("        {},", json_string(reason)));
        out.push("    ),".to_string());
    }
    out.push(TABLE_CLOSE.to_string());
    out
}

/// `file` (`shadow_ban.rs`) with its header block and both tables as `record` dictates, every other line
/// as it was.
fn render(file: &str, record: &Record) -> Result<String> {
    let mut lines: Vec<String> = file.split('\n').map(str::to_string).collect();
    let (begin, end) = header_span(&lines)?;
    let header = record
        .reports
        .iter()
        .flat_map(|(path, header)| [format!("{REPORT_PREFIX}{path}"), header.clone()]);
    lines.splice(begin + 1..end, header.collect::<Vec<_>>());
    for (open, rows) in [(BAN_OPEN, &record.ban), (WATCH_OPEN, &record.watch)] {
        let (from, to) = table_span(&lines, open)?;
        lines.splice(from..=to, table_lines(open, rows));
    }
    Ok(lines.join("\n"))
}

/// The paths of the reports of record `file`'s header block names, oldest first.
fn record_paths(file: &str) -> Result<Vec<String>> {
    let lines: Vec<String> = file.split('\n').map(str::to_string).collect();
    let (begin, end) = header_span(&lines)?;
    Ok(lines[begin + 1..end]
        .iter()
        .filter_map(|line| line.strip_prefix(REPORT_PREFIX))
        .map(str::to_string)
        .collect())
}

/// Fails with a line diff unless `file` is what `record` dictates.
fn check_file(file: &str, record: &Record) -> Result<()> {
    let expected = render(file, record)?;
    if expected == file {
        return Ok(());
    }
    let old: Vec<&str> = file.split('\n').collect();
    let new: Vec<&str> = expected.split('\n').collect();
    bail!(
        "{SHADOW_BAN_RS} is not what its reports of record dictate (`-` the file has it, `+` the record dictates it; `cargo jackioh sweep --write` rewrites it):\n{}",
        line_diff(&old, &new).join("\n")
    )
}

/// The lines that differ between `old` and `new` as `-old` and `+new` lines, in file order: the shared
/// head and tail dropped, a longest common subsequence between them.
fn line_diff(old: &[&str], new: &[&str]) -> Vec<String> {
    let head = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let (old, new) = (&old[head..], &new[head..]);
    let tail = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (old, new) = (&old[..old.len() - tail], &new[..new.len() - tail]);
    let (n, m) = (old.len(), new.len());
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old[i] == new[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out: Vec<String> = Vec::new();
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            i += 1;
            j += 1;
        } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push(format!("-{}", old[i]));
            i += 1;
        } else {
            out.push(format!("+{}", new[j]));
            j += 1;
        }
    }
    out
}

/// The record `reports` dictate for `file`: the reports named, or those `file`'s header block names.
fn record_of(file: &str, reports: &[String], unbanned: &[&str]) -> Result<Record> {
    let paths = if reports.is_empty() {
        record_paths(file)?
    } else {
        reports.to_vec()
    };
    let mut blocks: Vec<(String, Block)> = Vec::new();
    for path in paths {
        if Path::new(&path)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            bail!("{path}: name a report of record by its path from the repository root");
        }
        let text =
            std::fs::read_to_string(repo_root().join(&path)).with_context(|| format!("reading {path}"))?;
        let block = parse_block(&text).with_context(|| format!("reading {path}"))?;
        blocks.push((path, block));
    }
    compose(&blocks, unbanned)
}

/// `--write`.
fn run_write(reports: &[String]) -> Result<()> {
    let path = repo_root().join(SHADOW_BAN_RS);
    let file = std::fs::read_to_string(&path).with_context(|| format!("reading {SHADOW_BAN_RS}"))?;
    let record = record_of(&file, reports, SHADOW_UNBANNED)?;
    let written = render(&file, &record)?;
    let changed = written != file;
    if changed {
        std::fs::write(&path, written).with_context(|| format!("writing {SHADOW_BAN_RS}"))?;
    }
    println!(
        "{SHADOW_BAN_RS}: {} SHADOW_BAN and {} SHADOW_WATCH entries from {} report(s) of record{}",
        record.ban.len(),
        record.watch.len(),
        record.reports.len(),
        if changed { "" } else { ", unchanged" }
    );
    Ok(())
}

/// `--check`.
fn run_check(reports: &[String]) -> Result<()> {
    let file = std::fs::read_to_string(repo_root().join(SHADOW_BAN_RS))
        .with_context(|| format!("reading {SHADOW_BAN_RS}"))?;
    let record = record_of(&file, reports, SHADOW_UNBANNED)?;
    check_file(&file, &record)?;
    println!(
        "{SHADOW_BAN_RS}: as its {} report(s) of record dictate",
        record.reports.len()
    );
    Ok(())
}

fn read_lines(files: &[String]) -> Result<Vec<Value>> {
    let mut lines: Vec<Value> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).with_context(|| format!("reading {file}"))?;
        for line in text.split('\n').filter(|line| !line.trim().is_empty()) {
            lines.push(serde_json::from_str(line).with_context(|| format!("parsing a line of {file}"))?);
        }
    }
    Ok(lines)
}

fn is_pass2(line: &Value) -> bool {
    line.as_object()
        .is_some_and(|object| object.contains_key("forced"))
}

/// The pass-1 results among `lines`.
fn pass1_lines(lines: &[Value]) -> Result<Vec<SweepResult>> {
    lines
        .iter()
        .filter(|line| !is_pass2(line))
        .map(|line| serde_json::from_value::<SweepResult>(line.clone()).context("reading a pass-1 line"))
        .collect()
}

/// The pass-2 results among `lines`.
fn pass2_lines(lines: &[Value]) -> Result<Vec<SweepPass2>> {
    lines
        .iter()
        .filter(|line| is_pass2(line))
        .map(|line| serde_json::from_value::<SweepPass2>(line.clone()).context("reading a pass-2 line"))
        .collect()
}

/// One result as a JSON line on stdout, flushed, so a slice cut short keeps what it finished.
fn emit_line<T: serde::Serialize>(result: &T) {
    let mut stdout = std::io::stdout().lock();
    let line = serde_json::to_string(result).unwrap_or_default();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// `cargo jackioh sweep`.
pub fn run(args: Args) -> Result<()> {
    jackioh_cards::register_all();

    if let Some(files) = &args.report {
        let lines = read_lines(files)?;
        print!(
            "{}",
            report(&pass1_lines(&lines)?, &pass2_lines(&lines)?, None, &utc_date())
        );
        return Ok(());
    }
    if let Some(reports) = &args.write {
        return run_write(reports);
    }
    if let Some(reports) = &args.check {
        return run_check(reports);
    }

    // R1420: as `fuzz` does, every set is previewed, so a set that has not shipped yet is swept
    // (its ids are in the pool) and its cards are dealt as filler, as they will be once it ships.
    let _preview = preview_every_set();
    let pool: Vec<String> = query(&CatalogQueryArgs::default())
        .iter()
        .map(|def| def.id.clone())
        .collect();
    let pick_ids = |requested: &[String]| -> Vec<String> {
        let unknown: Vec<&str> = requested
            .iter()
            .filter(|id| !pool.contains(id))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            eprintln!(
                "[ai:sweep] not non-token card ids, skipped: {}",
                unknown.join(", ")
            );
        }
        if requested.is_empty() {
            pool.clone()
        } else {
            pool.iter().filter(|id| requested.contains(id)).cloned().collect()
        }
    };

    if let Some(files) = &args.pass2 {
        let files: Vec<String> = files
            .split(',')
            .filter(|file| !file.is_empty())
            .map(str::to_string)
            .collect();
        let first = pass1_lines(&read_lines(&files)?)?;
        let at_risk = at_risk_ids(&first, SHADOW_BAN, SHADOW_WATCH);
        let ids: Vec<String> = pick_ids(&args.ids)
            .into_iter()
            .filter(|id| at_risk.contains(id))
            .collect();
        pass2(
            &ids,
            &at_risk,
            &pass2_keep_out(&first, SHADOW_BAN),
            emit_line::<SweepPass2>,
        );
        return Ok(());
    }

    let ids = pick_ids(&args.ids);
    let started = Instant::now();
    let first = if args.json {
        pass1(&ids, emit_line::<SweepResult>)
    } else {
        pass1(&ids, |_| ())
    };
    if args.json {
        return Ok(());
    }
    let at_risk = at_risk_ids(&first, SHADOW_BAN, SHADOW_WATCH);
    let risky: Vec<String> = ids.iter().filter(|id| at_risk.contains(id)).cloned().collect();
    let second = pass2(&risky, &at_risk, &pass2_keep_out(&first, SHADOW_BAN), |_| ());
    let elapsed = started.elapsed().as_secs_f64().round() as i64;
    print!("{}", report(&first, &second, Some(elapsed), &utc_date()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_rounds_as_javascript_does() {
        assert_eq!(to_fixed(2.25, 1), "2.3");
        assert_eq!(to_fixed(-2.25, 1), "-2.3");
        assert_eq!(to_fixed(0.75, 1), "0.8");
        assert_eq!(to_fixed(9.95, 1), "9.9");
        assert_eq!(to_fixed(9.75, 1), "9.8");
        assert_eq!(to_fixed(99.95, 0), "100");
        assert_eq!(to_fixed(0.5, 0), "1");
        assert_eq!(to_fixed(-0.04, 1), "-0.0");
        assert_eq!(to_fixed(-0.0, 1), "0.0");
        assert_eq!(to_fixed(12.34, 1), "12.3");
        assert_eq!(to_fixed(-40.0, 1), "-40.0");
        assert_eq!(to_fixed(f64::NAN, 1), "NaN");
    }

    #[test]
    fn the_date_is_iso_shaped() {
        let date = utc_date();
        assert_eq!(date.len(), 10, "{date}");
        assert_eq!(&date[4..5], "-");
        assert_eq!(&date[7..8], "-");
    }

    #[test]
    fn a_line_is_pass_2_when_it_names_its_forced_card() {
        assert!(is_pass2(
            &serde_json::json!({ "forced": "core-011", "tier": "easy" })
        ));
        assert!(!is_pass2(&serde_json::json!({ "defId": "core-011" })));
        assert!(!is_pass2(&serde_json::json!([1, 2])));
    }

    #[test]
    fn a_rows_cells_escape_the_table_bar() {
        jackioh_cards::register_all();
        let stats: SweepStats = serde_json::from_value(serde_json::json!({
            "defId": "core-011", "games": 8, "drawnGames": 5, "affordableTurns": 4, "plays": 0,
            "errors": 0, "timeouts": 0, "evalDeltaSum": 0, "evalDeltaCount": 0
        }))
        .unwrap();
        let line = row(&stats, "easy", "a|b");
        assert!(line.starts_with("| core-011 | "), "{line}");
        assert!(line.contains(" a\\|b "), "{line}");
        assert!(line.ends_with(" | 5/8 | 4 | 0 | 0 | 0 | 0 | n/a |"), "{line}");
    }

    #[test]
    fn r977000_a_row_shows_the_raw_timeouts_beside_the_timeouts() {
        jackioh_cards::register_all();
        let stats: SweepStats = serde_json::from_value(serde_json::json!({
            "defId": "core-011", "games": 8, "drawnGames": 5, "affordableTurns": 4, "plays": 2,
            "errors": 0, "timeouts": 1, "rawTimeouts": 3, "evalDeltaSum": 0, "evalDeltaCount": 0
        }))
        .unwrap();
        let line = row(&stats, "easy", "timeout");
        assert!(line.ends_with(" | 5/8 | 4 | 2 | 0 | 1 | 3 | n/a |"), "{line}");
        assert!(TABLE_HEAD[0].contains("| errors | timeouts | raw timeouts |"));
        let bars = line.matches('|').count();
        assert!(TABLE_HEAD.iter().all(|head| head.matches('|').count() == bars));
    }

    /// Pass-1 and pass-2 lines as the slices emit them: meditative-011 and -012 time out, core-001 and
    /// meditative-022 are at risk (pass 2 cleared meditative-022's deal but not its watch).
    const FIXTURE_LINES: &str = r#"{"defId":"meditative-011","games":8,"drawnGames":7,"affordableTurns":7,"plays":2,"errors":0,"timeouts":3,"evalDeltaSum":3.6,"evalDeltaCount":2,"tier":"easy","flags":["timeout"],"unswept":false}
{"defId":"meditative-011","games":8,"drawnGames":5,"affordableTurns":9,"plays":2,"errors":0,"timeouts":1,"evalDeltaSum":12.4,"evalDeltaCount":2,"tier":"hard","flags":["timeout"],"unswept":false}
{"defId":"meditative-012","games":8,"drawnGames":5,"affordableTurns":1,"plays":5,"errors":0,"timeouts":11,"evalDeltaSum":80,"evalDeltaCount":5,"tier":"easy","flags":["timeout"],"unswept":false}
{"defId":"meditative-012","games":8,"drawnGames":4,"affordableTurns":2,"plays":3,"errors":0,"timeouts":0,"evalDeltaSum":30,"evalDeltaCount":3,"tier":"hard","flags":[],"unswept":false}
{"defId":"meditative-022","games":8,"drawnGames":6,"affordableTurns":12,"plays":1,"errors":0,"timeouts":0,"evalDeltaSum":-4.8,"evalDeltaCount":1,"tier":"easy","flags":[],"unswept":false}
{"defId":"meditative-022","games":8,"drawnGames":3,"affordableTurns":2,"plays":2,"errors":0,"timeouts":0,"evalDeltaSum":4,"evalDeltaCount":2,"tier":"hard","flags":[],"unswept":false}
{"defId":"core-001","games":8,"drawnGames":6,"affordableTurns":23,"plays":0,"errors":0,"timeouts":0,"evalDeltaSum":0,"evalDeltaCount":0,"tier":"easy","flags":["neverPlayed"],"unswept":false}
{"defId":"core-001","games":8,"drawnGames":2,"affordableTurns":2,"plays":1,"errors":0,"timeouts":0,"evalDeltaSum":1.5,"evalDeltaCount":1,"tier":"hard","flags":[],"unswept":false}
{"forced":"meditative-022","tier":"easy","games":24,"cards":[{"defId":"meditative-022","games":24,"drawnGames":18,"affordableTurns":30,"plays":12,"errors":0,"timeouts":0,"evalDeltaSum":24,"evalDeltaCount":12}],"suspects":[]}
"#;

    const FIXTURE_REPORT: &str = "training/history/sweep-fixture.md";
    const FIXTURE_DATE: &str = "2026-10-18";
    const WATCH_CORE_002: &str = "at risk: hard: pass 1 affordable on 7 turns over 8 games, never played";

    fn pair(id: &str, reason: &str) -> (String, String) {
        (id.to_string(), reason.to_string())
    }

    /// The fixture lines' report, as `report` prints it for `date`.
    fn fixture_report(date: &str) -> String {
        let lines: Vec<Value> = FIXTURE_LINES
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        report(
            &pass1_lines(&lines).unwrap(),
            &pass2_lines(&lines).unwrap(),
            None,
            date,
        )
    }

    /// What that report's `shadow_ban.rs` section holds, traced by hand through `sweep_verdict`.
    fn fixture_block() -> Block {
        let budget = serde_json::to_string(&AI_GATE_BUDGET).unwrap();
        Block {
            header: format!(
                "// Sweep of record: {FIXTURE_DATE} (UTC), `cargo jackioh sweep`, pass 1 over 4 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 1 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {budget}."
            ),
            ban: vec![
                pair(
                    "meditative-011",
                    "timeout: easy: 3 decision(s) over 2000 ms or game(s) past 600 actions; hard: 1 decision(s) over 2000 ms or game(s) past 600 actions",
                ),
                pair(
                    "meditative-012",
                    "timeout: easy: 11 decision(s) over 2000 ms or game(s) past 600 actions",
                ),
            ],
            watch: vec![
                pair(
                    "core-001",
                    "at risk: easy: pass 1 affordable on 23 turns over 8 games, never played",
                ),
                pair(
                    "meditative-022",
                    "at risk: easy: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change -4.8",
                ),
            ],
        }
    }

    /// A `shadow_ban.rs` with the header block and both tables the writer owns, and a row of its own.
    fn fixture_file() -> String {
        format!(
            "//! The fixture's module doc.\n\n{HEADER_BEGIN}\n{HEADER_END}\n\n/// The ban.\n{BAN_OPEN}];\n\n/// The watch.\n{WATCH_OPEN}\n    (\n        \"core-002\",\n        \"{WATCH_CORE_002}\",\n    ),\n];\n"
        )
    }

    /// What `--write` makes of `fixture_file` from `fixture_block`.
    fn fixture_written() -> String {
        let block = fixture_block();
        let mut rows = String::new();
        for (open, table) in [(BAN_OPEN, &block.ban), (WATCH_OPEN, &block.watch)] {
            rows.push_str(&format!("{open}\n"));
            for (id, reason) in table {
                rows.push_str(&format!(
                    "    (\n        \"{id}\",\n        \"{reason}\",\n    ),\n"
                ));
            }
            rows.push_str("];\n");
            if open == BAN_OPEN {
                rows.push_str("\n/// The watch.\n");
            }
        }
        format!(
            "//! The fixture's module doc.\n\n{HEADER_BEGIN}\n// Report of record: {FIXTURE_REPORT}\n{}\n{HEADER_END}\n\n/// The ban.\n{rows}",
            block.header
        )
    }

    fn fixture_record() -> Record {
        compose(&[(FIXTURE_REPORT.to_string(), fixture_block())], &[]).unwrap()
    }

    fn block_of(ban: &[String], watch: &[&str]) -> Block {
        Block {
            header: format!("{SWEEP_PREFIX}{FIXTURE_DATE} (UTC), fixture."),
            ban: ban.iter().map(|id| pair(id, "timeout: easy: fixture")).collect(),
            watch: watch.iter().map(|id| pair(id, "at risk: fixture")).collect(),
        }
    }

    fn meditative_ids(first: usize, last: usize) -> Vec<String> {
        (first..=last).map(|n| format!("meditative-{n:03}")).collect()
    }

    fn compose_one(block: Block, unbanned: &[&str]) -> Result<Record> {
        compose(&[(FIXTURE_REPORT.to_string(), block)], unbanned)
    }

    #[test]
    fn the_report_block_reads_back_as_it_was_printed() {
        jackioh_cards::register_all();
        let printed = fixture_report(FIXTURE_DATE);
        assert_eq!(parse_block(&printed).unwrap(), fixture_block());
        // A committed report holds the section one heading level down.
        let committed = printed.replace("## For crates", "### For crates");
        assert_ne!(committed, printed);
        assert_eq!(parse_block(&committed).unwrap(), fixture_block());
    }

    #[test]
    fn the_report_block_is_the_text_the_report_always_printed() {
        let block = fixture_block();
        let lines = block_lines(&block);
        assert_eq!(lines[0], "## For crates/ai/src/shadow_ban.rs");
        assert_eq!(lines[2], "Header line:");
        assert_eq!(lines[4], block.header);
        assert!(lines.iter().any(|line| line == "SHADOW_BAN entries:"));
        assert!(lines.iter().any(|line| line == "SHADOW_WATCH entries:"));
        assert_eq!(lines.last().map(String::as_str), Some("```"));
        assert!(parse_block("# Report\n\nNo section.\n").is_err());
        assert!(parse_block("## For crates/ai/src/shadow_ban.rs\n\nHeader line:\n\n// nope\n").is_err());
    }

    #[test]
    fn a_fixture_sweep_through_write_gives_the_expected_tables_and_header_byte_for_byte() {
        jackioh_cards::register_all();
        let record = fixture_record();
        let written = render(&fixture_file(), &record).unwrap();
        assert_eq!(written, fixture_written());
        assert!(written.contains("        \"meditative-011\",\n"));
        assert!(!written.contains("core-002"), "the stale row is gone");
        // A second rewrite changes nothing, and the check passes.
        assert_eq!(render(&written, &record).unwrap(), written);
        check_file(&written, &record).unwrap();
        assert!(check_file(&fixture_file(), &record).is_err());
    }

    #[test]
    fn write_leaves_the_unban_lanes_removals_out_of_shadow_ban() {
        jackioh_cards::register_all();
        let record = compose_one(fixture_block(), &["meditative-012"]).unwrap();
        assert_eq!(
            record.ban.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            ["meditative-011"]
        );
        assert_eq!(record.watch.len(), 2);
        let error = compose_one(fixture_block(), &["meditative-099"]).unwrap_err();
        assert!(error.to_string().contains("SHADOW_UNBANNED"), "{error}");
    }

    #[test]
    fn check_fails_with_a_diff_when_a_row_is_added_removed_or_reordered_by_hand_or_the_header_is_stale() {
        jackioh_cards::register_all();
        let record = fixture_record();
        let good = fixture_written();
        let core_001 = "    (\n        \"core-001\",\n        \"at risk: easy: pass 1 affordable on 23 turns over 8 games, never played\",\n    ),\n";
        let meditative_022 = "    (\n        \"meditative-022\",\n        \"at risk: easy: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change -4.8\",\n    ),\n";
        assert!(good.contains(core_001) && good.contains(meditative_022));
        let diff = |file: &str| check_file(file, &record).unwrap_err().to_string();

        let added = good.replace(
            core_001,
            &format!("    (\n        \"core-005\",\n        \"{WATCH_CORE_002}\",\n    ),\n{core_001}"),
        );
        let error = diff(&added);
        assert!(error.contains("-        \"core-005\","), "{error}");
        assert!(error.contains("cargo jackioh sweep --write"), "{error}");

        let removed = good.replace(core_001, "");
        let error = diff(&removed);
        assert!(error.contains("+        \"core-001\","), "{error}");

        let reordered = good
            .replace(core_001, "@@")
            .replace(meditative_022, core_001)
            .replace("@@", meditative_022);
        let error = diff(&reordered);
        let names_a_row = |sign: char| {
            error.lines().any(|line| {
                line.starts_with(sign)
                    && (line.contains("\"core-001\"") || line.contains("\"meditative-022\""))
            })
        };
        assert!(names_a_row('-') && names_a_row('+'), "{error}");

        let stale = good.replace("// Sweep of record: 2026-10-18", "// Sweep of record: 2026-10-11");
        let error = diff(&stale);
        assert!(error.contains("\n-// Sweep of record: 2026-10-11"), "{error}");
        assert!(error.contains("\n+// Sweep of record: 2026-10-18"), "{error}");
    }

    #[test]
    fn write_refuses_a_pre_meditative_ban_and_more_than_half_the_meditative_set() {
        jackioh_cards::register_all();
        let error = compose_one(block_of(&["core-001".to_string()], &[]), &[]).unwrap_err();
        assert!(error.to_string().contains("R732000"), "{error}");
        let error = compose_one(block_of(&["nonesuch-001".to_string()], &[]), &[]).unwrap_err();
        assert!(error.to_string().contains("no catalog card"), "{error}");
        let error = compose_one(block_of(&meditative_ids(1, 52), &[]), &[]).unwrap_err();
        assert!(error.to_string().contains("more than half"), "{error}");
        assert!(compose_one(block_of(&meditative_ids(1, 51), &[]), &[]).is_ok());
        // Only the newest report's bans are held to it: an older sweep of the earlier sets banned
        // them, and they are left out of the table.
        let older = block_of(&["core-001".to_string()], &[]);
        let newest = block_of(&meditative_ids(1, 3), &[]);
        let both = [("a.md".to_string(), older), ("b.md".to_string(), newest)];
        assert_eq!(
            compose(&both, &[]).unwrap().ban,
            block_of(&meditative_ids(1, 3), &[]).ban
        );
    }

    /// The non-token Meditative+ ids, in catalog order.
    fn meditative_plus_ids() -> Vec<String> {
        jackioh_cards::CATALOG
            .values()
            .filter(|def| def.set == SetName::MeditativePlus && !def.token)
            .map(|def| def.id.clone())
            .collect()
    }

    #[test]
    fn r1186000_write_holds_meditative_plus_to_half_of_its_own_set() {
        jackioh_cards::register_all();
        let ids = meditative_plus_ids();
        let most = ids.len() / BAN_DIVISOR;
        let mut over = ids[..=most].to_vec();
        over.sort();
        let error = compose_one(block_of(&over, &[]), &[]).unwrap_err();
        assert!(error.to_string().contains("more than half"), "{error}");
        assert!(error.to_string().contains("R1186000"), "{error}");
        let mut half = ids[..most].to_vec();
        half.sort();
        assert!(compose_one(block_of(&half, &[]), &[]).is_ok());
        // Each set is held to its own half: half of Meditative+ and half of Meditative together.
        let both = [
            ("a.md".to_string(), block_of(&meditative_ids(1, 51), &[])),
            ("b.md".to_string(), block_of(&half, &[])),
        ];
        assert_eq!(compose(&both, &[]).unwrap().ban.len(), 51 + most);
    }

    #[test]
    fn r1186000_a_meditative_plus_sweep_keeps_the_meditative_bans_and_a_later_watch_row_lifts_a_ban() {
        jackioh_cards::register_all();
        let meditative = block_of(&meditative_ids(11, 13), &["meditative-022"]);
        let plus = meditative_plus_ids()[0].clone();
        let newest = block_of(std::slice::from_ref(&plus), &["meditative-012"]);
        let reports = [("a.md".to_string(), meditative), ("b.md".to_string(), newest)];
        let record = compose(&reports, &["meditative-013"]).unwrap();
        assert_eq!(
            record.ban.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            ["meditative-011", plus.as_str()]
        );
        assert_eq!(
            record.watch.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            ["meditative-012", "meditative-022"]
        );
        // The unban lane's removals are checked against every report's bans, not the newest's alone.
        let error = compose(&reports, &["meditative-014"]).unwrap_err();
        assert!(error.to_string().contains("no report of record bans"), "{error}");
    }

    #[test]
    fn write_refuses_unsorted_rows() {
        jackioh_cards::register_all();
        let unsorted_watch = block_of(&[], &["meditative-022", "core-001"]);
        let twice = block_of(&["meditative-011".to_string(), "meditative-011".to_string()], &[]);
        for (block, unbanned) in [
            (unsorted_watch, vec![]),
            (twice, vec![]),
            (
                block_of(&meditative_ids(11, 12), &[]),
                vec!["meditative-012", "meditative-011"],
            ),
        ] {
            let error = compose_one(block, &unbanned).unwrap_err();
            assert!(error.to_string().contains("not sorted"), "{error}");
        }
    }

    #[test]
    fn a_later_report_wins_a_watch_row_and_a_ban_takes_a_card_off_the_watch() {
        jackioh_cards::register_all();
        let mut older = block_of(&[], &["core-001", "core-002", "meditative-011"]);
        older.watch[0].1 = "at risk: older".to_string();
        let mut newest = block_of(&["meditative-011".to_string()], &["core-002", "meditative-022"]);
        newest.watch[0].1 = "at risk: newer".to_string();
        let record = compose(&[("a.md".to_string(), older), ("b.md".to_string(), newest)], &[]).unwrap();
        assert_eq!(
            record.watch,
            [
                pair("core-001", "at risk: older"),
                pair("core-002", "at risk: newer"),
                pair("meditative-022", "at risk: fixture"),
            ]
        );
        assert_eq!(compose(&[], &[]).unwrap_err().to_string(), "no report of record");
    }

    #[test]
    fn an_empty_table_renders_on_one_line_and_reads_back() {
        jackioh_cards::register_all();
        let mut block = fixture_block();
        block.ban.clear();
        let record = compose_one(block, &[]).unwrap();
        let written = render(&fixture_written(), &record).unwrap();
        assert!(
            written.contains("\npub const SHADOW_BAN: &[(&str, &str)] = &[];\n"),
            "{written}"
        );
        assert_eq!(render(&written, &record).unwrap(), written);
        // And a table that had rows grows back from the one line.
        assert_eq!(render(&written, &fixture_record()).unwrap(), fixture_written());
        // The report prints an empty table as an empty fence, which reads back empty.
        let printed = block_lines(&Block {
            ban: Vec::new(),
            ..fixture_block()
        })
        .join("\n");
        assert!(parse_block(&printed).unwrap().ban.is_empty());
    }

    #[test]
    fn write_and_check_take_reports_from_the_command_line_or_the_header() {
        use clap::Parser;
        #[derive(Parser)]
        struct Cli {
            #[command(flatten)]
            args: Args,
        }
        let parse =
            |words: &[&str]| Cli::try_parse_from(std::iter::once("sweep").chain(words.iter().copied()));
        assert_eq!(parse(&["--write"]).unwrap().args.write, Some(Vec::new()));
        assert_eq!(
            parse(&["--write", "a.md", "b.md"]).unwrap().args.write,
            Some(vec!["a.md".to_string(), "b.md".to_string()])
        );
        assert_eq!(parse(&["--check"]).unwrap().args.check, Some(Vec::new()));
        assert!(parse(&["--write", "--check"]).is_err());
        assert!(parse(&[]).unwrap().args.write.is_none());

        assert_eq!(record_paths(&fixture_written()).unwrap(), [FIXTURE_REPORT]);
        assert!(record_paths("no block here\n").is_err());
        for path in ["../x.md", "/x.md", "./x.md"] {
            let error = record_of("", &[path.to_string()], &[]).unwrap_err();
            assert!(
                error.to_string().contains("path from the repository root"),
                "{error}"
            );
        }
    }

    #[test]
    fn the_line_diff_names_each_changed_line_once_in_file_order() {
        let old = ["a", "b", "c", "d", "e"];
        let new = ["a", "c", "b", "d", "e"];
        assert_eq!(line_diff(&old, &new), ["-b", "+b"].map(String::from));
        assert!(line_diff(&old, &old).is_empty());
        assert_eq!(line_diff(&[], &["x"]), ["+x"].map(String::from));
        assert_eq!(line_diff(&["x"], &[]), ["-x"].map(String::from));
    }

    #[test]
    fn shadow_ban_rs_is_what_its_reports_of_record_dictate() {
        jackioh_cards::register_all();
        let file = std::fs::read_to_string(repo_root().join(SHADOW_BAN_RS)).unwrap();
        let record = record_of(&file, &[], SHADOW_UNBANNED).unwrap_or_else(|error| panic!("{error:#}"));
        if let Err(error) = check_file(&file, &record) {
            panic!("{error:#}");
        }
        let owned = |rows: &[(&str, &str)]| rows.iter().map(|(id, why)| pair(id, why)).collect::<Vec<_>>();
        assert_eq!(record.ban, owned(SHADOW_BAN));
        assert_eq!(record.watch, owned(SHADOW_WATCH));
    }
}
