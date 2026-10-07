//! `cargo jackioh sweep` (R186, R390; SURFACE §12): the run that decides the AI's shadow ban, in two
//! passes. Port of `packages/ai/scripts/sweep.ts` (`pnpm ai:sweep`).
//!
//! Pass 1: for every non-token card of every set and every tier in AI_SWEEP.tiers, `sweepCard` forces
//! the card into AI decks on that tier's handicap against the greedy baseline (AI_SWEEP.seedsPerCard
//! games each, at AI_GATE_BUDGET). `atRiskIds` reads the at-risk cards off pass 1 (plus today's
//! SHADOW_BAN and SHADOW_WATCH). Pass 2: `sweepAtRisk` forces each at-risk card into
//! AI_SWEEP.seedsPerCardAtRisk more games per tier, with every at-risk card dealt more often as
//! filler. `sweepVerdict` joins a card's tiers and passes. This prints the flagged rows of both
//! passes, the at-risk cards, the suspects, the unswept cards, then the ready-made SHADOW_BAN and
//! SHADOW_WATCH entries and the header line for `crates/ai/src/shadow_ban.rs`. It always exits 0: the
//! tables are copied in by hand from this output, never edited to taste.
//!
//! ```text
//! cargo jackioh sweep                              both passes over every card, then the report
//! cargo jackioh sweep core-011 classic-020         only these ids (pass 2: those of them at risk)
//! cargo jackioh sweep --json core-011 …            pass 1 only: one JSON SweepResult per line (a slice)
//! cargo jackioh sweep --pass2 a.jsonl,b.jsonl …    pass 2 from every slice's pass-1 lines: one JSON
//!                                                  SweepPass2 per line, for the at-risk ids listed
//!                                                  after the files (default every at-risk id); slice it too
//! cargo jackioh sweep --report a.jsonl p2.jsonl …  the report from pass-1 and pass-2 lines
//! ```
//!
//! Pass 2 needs every slice's pass 1 first, because the at-risk list it boosts is the whole sweep's.
//! A slice writes each line as it lands, so one cut short keeps what it finished: rerun the rest.
//! Tooling, so it may read the clock (`Instant`, for decision timing), read files and write to the
//! console; `crates/ai` stays pure and receives the clock as `now`.

use std::io::Write as _;
use std::time::Instant;

use anyhow::{Context, Result};
use indexmap::IndexSet;
use serde_json::Value;

use jackioh_ai::{
    AI_GATE_BUDGET, AI_SWEEP, SHADOW_BAN, SHADOW_WATCH, SweepFlag, SweepOptions, SweepPass2, SweepResult, SweepStats,
    at_risk_ids, pass2_keep_out, pass2_stats, sweep_at_risk, sweep_card, sweep_verdict,
};
use jackioh_engine::catalog::{CatalogQueryArgs, query};
use jackioh_engine::config::Difficulty;

use crate::arena::utc_date;
use crate::gate::{MS_PER_SECOND, literal};

/// The mean evaluate change per play, and each card's seconds, printed to this many decimals.
const DECIMALS: usize = 1;

/// Enough decimals to hold the exact expansion of any f64 whose value can be a rounding tie
/// (`to_fixed`): a tie at a few decimals is a multiple of a power of two's reciprocal, whose
/// expansion ends long before this.
const EXACT_DIGITS: usize = 40;

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
        return if x > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let exact = format!("{:.*}", EXACT_DIGITS, x.abs());
    let (whole, frac) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let tie = frac.as_bytes().get(digits) == Some(&b'5') && frac[digits + 1..].bytes().all(|byte| byte == b'0');
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
    jackioh_cards::CATALOG.get(def_id).map_or_else(|| def_id.to_string(), |def| def.name.clone())
}

/// A sweep flag's name, as TS wrote it (`neverPlayed` …).
fn flag_names(flags: &[SweepFlag]) -> String {
    flags.iter().map(|flag| literal(flag)).collect::<Vec<_>>().join(", ")
}

fn mean_delta(stats: &SweepStats) -> String {
    if stats.eval_delta_count == 0 {
        return "n/a".to_string();
    }
    to_fixed(stats.eval_delta_sum / f64::from(stats.eval_delta_count as i32), DECIMALS)
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
        mean_delta(stats),
    ]
    .map(|cell| format!(" {} ", cell.replace('|', "\\|")));
    format!("|{}|", cells.join("|"))
}

const TABLE_HEAD: [&str; 2] = [
    "| id | name | tier | flags | drawn/games | affordable turns | plays | errors | timeouts | mean eval delta |",
    "|---|---|---|---|---|---|---|---|---|---|",
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

/// Each result is also handed to `emit` as it lands, so a slice cut short keeps what it finished.
fn pass1(ids: &[String], mut emit: impl FnMut(&SweepResult)) -> Vec<SweepResult> {
    let now = milliseconds_since(Instant::now());
    let mut results: Vec<SweepResult> = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        for tier in AI_SWEEP.tiers.iter().copied() {
            let result = timed(
                &format!("pass 1 {}/{} {id} {} @{tier}", index + 1, ids.len(), name_of(id)),
                || sweep_card(id, &SweepOptions { seeds: None, now: Some(&now), tier: Some(tier) }),
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

fn pass2(ids: &[String], at_risk: &[String], keep_out: &[String], mut emit: impl FnMut(&SweepPass2)) -> Vec<SweepPass2> {
    let now = milliseconds_since(Instant::now());
    let mut results: Vec<SweepPass2> = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        for tier in AI_SWEEP.tiers.iter().copied() {
            let result = timed(
                &format!("pass 2 {}/{} {id} {} @{tier}", index + 1, ids.len(), name_of(id)),
                || sweep_at_risk(id, at_risk, keep_out, &SweepOptions { seeds: None, now: Some(&now), tier: Some(tier) }),
                |r| format!("{} at-risk card(s) dealt, {} suspect line(s)", r.cards.len(), r.suspects.len()),
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

fn report(first: &[SweepResult], second: &[SweepPass2], elapsed: Option<i64>) -> String {
    let ids = sorted_unique(first.iter().map(|result| result.stats.def_id.clone()));
    let at_risk = at_risk_ids(first, SHADOW_BAN, SHADOW_WATCH);
    let forced = sorted_unique(second.iter().map(|result| result.forced.clone()));
    let verdicts: Vec<_> = sorted_unique(ids.iter().chain(forced.iter()).cloned())
        .iter()
        .map(|id| {
            let own: Vec<SweepResult> = first.iter().filter(|result| &result.stats.def_id == id).cloned().collect();
            sweep_verdict(&own, second)
        })
        .collect();
    let banned: Vec<_> = verdicts.iter().filter(|verdict| verdict.reason.is_some()).collect();
    let watched: Vec<_> = verdicts.iter().filter(|verdict| verdict.watch.is_some()).collect();
    let unswept: Vec<_> = verdicts.iter().filter(|verdict| verdict.unswept).collect();
    let suspects: Vec<_> = second.iter().flat_map(|result| result.suspects.iter()).collect();

    // The run's date for shadow_ban.rs's header.
    let date = utc_date();
    let budget = serde_json::to_string(&AI_GATE_BUDGET).unwrap_or_default();
    let tiers = AI_SWEEP.tiers.iter().map(|tier| tier.as_str()).collect::<Vec<_>>().join(" and ");
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
    out.push("## Pass 1 flags (a `neverPlayed` or `selfHarm` here only puts a card at risk)".to_string());
    out.push(String::new());
    let flagged: Vec<&SweepResult> = first.iter().filter(|result| !result.flags.is_empty()).collect();
    if flagged.is_empty() {
        out.push("No card was flagged at any tier.".to_string());
    } else {
        out.extend(TABLE_HEAD.iter().map(|line| (*line).to_string()));
        out.extend(flagged.iter().map(|result| row(&result.stats, result.tier.as_str(), &flag_names(&result.flags))));
    }
    out.push(String::new());
    out.push(format!("## At risk ({}): pass 1 at half strength, SHADOW_BAN and SHADOW_WATCH", at_risk.len()));
    out.push(String::new());
    out.push(if at_risk.is_empty() {
        "None.".to_string()
    } else {
        at_risk.iter().map(|id| format!("{id} {}", name_of(id))).collect::<Vec<_>>().join(", ")
    });
    let missing: Vec<&String> = at_risk.iter().filter(|id| ids.contains(id) && !forced.contains(id)).collect();
    if !missing.is_empty() {
        out.push(String::new());
        out.push(format!(
            "Not swept in pass 2 (no evidence, no ban for neverPlayed or selfHarm): {}",
            missing.iter().map(|id| id.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    out.push(String::new());
    out.push("## Pass 2: every at-risk card over every game it was dealt in, forced or filler".to_string());
    out.push(String::new());
    if second.is_empty() {
        out.push("Pass 2 did not run.".to_string());
    } else {
        out.extend(TABLE_HEAD.iter().map(|line| (*line).to_string()));
        let dealt = sorted_unique(second.iter().flat_map(|result| result.cards.iter().map(|card| card.def_id.clone())));
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
    out.push("## Suspects (an error or timeout in a pass-2 game that also dealt this at-risk card as filler)".to_string());
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
        unswept.iter().map(|verdict| format!("- {} {}", verdict.def_id, name_of(&verdict.def_id))).collect::<Vec<_>>().join("\n")
    });
    out.push(String::new());
    out.push("## For crates/ai/src/shadow_ban.rs".to_string());
    out.push(String::new());
    out.push("Header line:".to_string());
    out.push(String::new());
    out.push(format!("// Sweep of record: {date} (UTC), `cargo jackioh sweep`, {passes}."));
    out.push(String::new());
    out.push("SHADOW_BAN entries:".to_string());
    out.push(String::new());
    out.push("```rust".to_string());
    for verdict in &banned {
        out.push(format!("    ({}, {}),", json_string(&verdict.def_id), json_string(verdict.reason.as_deref().unwrap_or_default())));
    }
    out.push("```".to_string());
    out.push(String::new());
    out.push("SHADOW_WATCH entries:".to_string());
    out.push(String::new());
    out.push("```rust".to_string());
    for verdict in &watched {
        out.push(format!("    ({}, {}),", json_string(&verdict.def_id), json_string(verdict.watch.as_deref().unwrap_or_default())));
    }
    out.push("```".to_string());
    format!("{}\n", out.join("\n"))
}

/// `JSON.stringify(text)`: a quoted string with JSON's escapes, which Rust's string literals read the
/// same for every character a card id or a ban reason holds.
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
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
    line.as_object().is_some_and(|object| object.contains_key("forced"))
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
        print!("{}", report(&pass1_lines(&lines)?, &pass2_lines(&lines)?, None));
        return Ok(());
    }

    let pool: Vec<String> = query(&CatalogQueryArgs::default()).iter().map(|def| def.id.clone()).collect();
    let pick_ids = |requested: &[String]| -> Vec<String> {
        let unknown: Vec<&str> = requested.iter().filter(|id| !pool.contains(id)).map(String::as_str).collect();
        if !unknown.is_empty() {
            eprintln!("[ai:sweep] not non-token card ids, skipped: {}", unknown.join(", "));
        }
        if requested.is_empty() { pool.clone() } else { pool.iter().filter(|id| requested.contains(id)).cloned().collect() }
    };

    if let Some(files) = &args.pass2 {
        let files: Vec<String> = files.split(',').filter(|file| !file.is_empty()).map(str::to_string).collect();
        let first = pass1_lines(&read_lines(&files)?)?;
        let at_risk = at_risk_ids(&first, SHADOW_BAN, SHADOW_WATCH);
        let ids: Vec<String> = pick_ids(&args.ids).into_iter().filter(|id| at_risk.contains(id)).collect();
        pass2(&ids, &at_risk, &pass2_keep_out(&first, SHADOW_BAN), emit_line::<SweepPass2>);
        return Ok(());
    }

    let ids = pick_ids(&args.ids);
    let started = Instant::now();
    let first = if args.json { pass1(&ids, emit_line::<SweepResult>) } else { pass1(&ids, |_| ()) };
    if args.json {
        return Ok(());
    }
    let at_risk = at_risk_ids(&first, SHADOW_BAN, SHADOW_WATCH);
    let risky: Vec<String> = ids.iter().filter(|id| at_risk.contains(id)).cloned().collect();
    let second = pass2(&risky, &at_risk, &pass2_keep_out(&first, SHADOW_BAN), |_| ());
    let elapsed = started.elapsed().as_secs_f64().round() as i64;
    print!("{}", report(&first, &second, Some(elapsed)));
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
        assert!(is_pass2(&serde_json::json!({ "forced": "core-011", "tier": "easy" })));
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
        assert!(line.ends_with(" | 5/8 | 4 | 0 | 0 | 0 | n/a |"), "{line}");
    }
}
