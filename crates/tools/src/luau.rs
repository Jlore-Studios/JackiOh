//! `cargo jackioh luau diff` and `cargo jackioh luau bench` (#442's L13 and L15; groundwork for its
//! part 7, #559).
//!
//! Both hold this build to a parent: `jackioh` built from the base commit, whatever the cards are
//! written in. Nothing in either is specific to Luau.
//!
//! `diff --parent-bin <path> --cards <ids> [--games N] [--seed base]` plays N games (200 by default,
//! seeds base..base+N-1, 1 by default, which `golden record` keeps within 1–240) with the cards forced
//! into both decks, recorded by `golden record` on this build into a temporary file. The parent's
//! `golden check --file` then replays the file as the oracle, and its report is printed as it is: each
//! divergence's seed, step, which hash and the action. Exit 1 on any mismatch, with the traces kept
//! where the error says. `--traces <path>` checks a `golden record` file instead of recording one, and
//! never deletes it.
//!
//! `bench --parent-bin <path> [--record <path>]` is the performance gate (L13). It measures three
//! numbers on the parent and then on this build, on the same machine in the same run: the wall time of
//! `fuzz --seeds 200`; the slowest decision of `gate`'s perf gate, scaled as `gate` reports it; and the
//! mean time of one golden step, `golden check` over this checkout's traces on one thread. It exits 1
//! when this build's slowest decision is over `BENCH_BUDGETS.decision_ratio` times the parent's or not
//! under `AI_GATE.max_decision_ms`, or when either other number is over its ratio times the parent's.
//! `--record` writes the numbers, the machine and the date as JSON, whether the bench passed or not.
//! Only `bench_failures`, a pure function of the two measurements and the budgets, is unit-tested; the
//! timings are not.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Output};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use jackioh_ai::AI_GATE;
use jackioh_cards::register_all;
use serde::Serialize;
use serde_json::Value;

use crate::gate::MS_PER_SECOND;
use crate::golden::{GAMES_PATH, record_seeds};

/// L15: the games `diff` plays when `--games` is not given.
const DIFF_GAMES: u32 = 200;

/// The first seed `diff` plays when `--seed` is not given.
const DIFF_FIRST_SEED: u32 = 1;

/// L13: the seeds `fuzz` plays in the bench, CI's own gate.
const BENCH_FUZZ_SEEDS: u32 = 200;

/// The golden replay's thread count in the bench: one, so a step's time is not shared with another's.
const BENCH_GOLDEN_THREADS: &str = "1";

/// Where the bench reads the machine's CPU model, when the machine has one.
const CPUINFO: &str = "/proc/cpuinfo";

/// L13: what a build may cost against its parent, and the gate's own cap on one decision.
pub(crate) const BENCH_BUDGETS: BenchBudgets = BenchBudgets {
    decision_ratio: 1.5,
    max_decision_ms: AI_GATE.max_decision_ms,
    fuzz_ratio: 2.0,
    step_ratio: 2.0,
};

/// `luau diff`, `luau bench`.
#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Replay games of this build, with cards forced, on a parent build; exit 1 on a mismatch.
    Diff(DiffArgs),
    /// Time this build against a parent build; exit 1 when it is over budget.
    Bench(BenchArgs),
}

#[derive(clap::Args)]
struct DiffArgs {
    /// The parent `jackioh`, built from the base commit: the oracle.
    #[arg(long, value_name = "PATH")]
    parent_bin: PathBuf,
    /// Card ids to force into both decks of every game; a token goes in through a card whose `refs`
    /// name it (R279).
    #[arg(
        long,
        value_name = "IDS",
        value_delimiter = ',',
        required_unless_present = "traces"
    )]
    cards: Vec<String>,
    /// How many games to play, from `--seed` on (the golden seeds are 1–240).
    #[arg(long, value_name = "N", default_value_t = DIFF_GAMES)]
    games: u32,
    /// The first golden seed.
    #[arg(long, value_name = "BASE", default_value_t = DIFF_FIRST_SEED)]
    seed: u32,
    /// Check this `golden record` file instead of recording one.
    #[arg(long, value_name = "PATH", conflicts_with = "cards")]
    traces: Option<PathBuf>,
}

#[derive(clap::Args)]
struct BenchArgs {
    /// The parent `jackioh`, built from the base commit.
    #[arg(long, value_name = "PATH")]
    parent_bin: PathBuf,
    /// Write the numbers, the machine and the date here as JSON.
    #[arg(long, value_name = "PATH")]
    record: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    register_all();
    match args.command {
        Command::Diff(diff) => run_diff(&diff),
        Command::Bench(bench) => run_bench(&bench),
    }
}

// ---------------------------------------------------------------------------------------------
// diff
// ---------------------------------------------------------------------------------------------

/// A path in the temporary directory for this run's traces.
fn temp_traces() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("jackioh-luau-diff-{}-{stamp}.jsonl", std::process::id()))
}

fn run_diff(args: &DiffArgs) -> Result<()> {
    let parent = args.parent_bin.display();
    let (traces, recorded) = match &args.traces {
        Some(path) => (path.clone(), false),
        None => {
            if args.games == 0 {
                bail!("--games takes at least 1");
            }
            let last = args.seed.checked_add(args.games - 1).with_context(|| {
                format!(
                    "--seed {} with --games {} runs past the last seed",
                    args.seed, args.games
                )
            })?;
            let path = temp_traces();
            let seeds: Vec<u32> = (args.seed..=last).collect();
            let wrote = record_seeds(&seeds, &args.cards, &path)?;
            println!(
                "luau diff: recorded {} games (seeds {}–{last}, {} steps) with {} forced",
                wrote.games,
                args.seed,
                wrote.steps,
                wrote.forced.join(", ")
            );
            (path, true)
        }
    };

    let output = Process::new(&args.parent_bin)
        .args(["golden", "check", "--file"])
        .arg(&traces)
        .output()
        .with_context(|| format!("running {parent} golden check"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");

    if output.status.success() {
        if recorded {
            // A leftover temporary file is no harm, so a failed removal is not an error.
            let _ = fs::remove_file(&traces);
        }
        println!("luau diff: 0 mismatches against {parent}");
        return Ok(());
    }
    match summary_counts(&stdout) {
        Some((_, diverged)) if diverged > 0 => bail!(
            "luau diff: {diverged} mismatch(es) against {parent}; the traces are kept at {}",
            traces.display()
        ),
        _ => bail!(
            "luau diff: {parent} could not check {}: {}",
            traces.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

/// `(steps, diverged)` from `golden check`'s summary line, `golden check: N of M games replayed
/// identically (S steps), D diverged`.
fn summary_counts(stdout: &str) -> Option<(u64, u64)> {
    let line = stdout.lines().find(|line| line.starts_with("golden check: "))?;
    let (_, after) = line.split_once('(')?;
    let (steps, rest) = after.split_once(" steps), ")?;
    let (diverged, _) = rest.split_once(" diverged")?;
    Some((steps.parse().ok()?, diverged.parse().ok()?))
}

// ---------------------------------------------------------------------------------------------
// bench
// ---------------------------------------------------------------------------------------------

/// What the bench measures on one build, all in milliseconds.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BenchNumbers {
    /// The wall time of `fuzz --seeds 200`.
    pub fuzz_ms: f64,
    /// The slowest decision of `gate`'s perf gate, scaled as `gate` reports it.
    pub slowest_decision_ms: f64,
    /// The mean time of one golden step, native, on one thread.
    pub golden_step_ms: f64,
}

/// What a build may cost: ratios of the parent's numbers, and the perf gate's cap on one decision.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BenchBudgets {
    pub decision_ratio: f64,
    pub max_decision_ms: f64,
    pub fuzz_ratio: f64,
    pub step_ratio: f64,
}

/// Why `current` is over budget against `parent`, one reason per breach (none when it is within every
/// one). A number exactly at its ratio passes; the cap fails at it, as the perf gate does.
pub(crate) fn bench_failures(
    current: &BenchNumbers,
    parent: &BenchNumbers,
    budgets: &BenchBudgets,
) -> Vec<String> {
    let mut reasons: Vec<String> = Vec::new();
    if current.slowest_decision_ms > budgets.decision_ratio * parent.slowest_decision_ms {
        reasons.push(format!(
            "slowest decision: {:.1} ms is over {}× the parent's {:.1} ms",
            current.slowest_decision_ms, budgets.decision_ratio, parent.slowest_decision_ms
        ));
    }
    if current.slowest_decision_ms >= budgets.max_decision_ms {
        reasons.push(format!(
            "slowest decision: {:.1} ms is not under AI_GATE.max_decision_ms ({} ms)",
            current.slowest_decision_ms, budgets.max_decision_ms
        ));
    }
    if current.fuzz_ms > budgets.fuzz_ratio * parent.fuzz_ms {
        reasons.push(format!(
            "fuzz --seeds {BENCH_FUZZ_SEEDS}: {:.1} ms is over {}× the parent's {:.1} ms",
            current.fuzz_ms, budgets.fuzz_ratio, parent.fuzz_ms
        ));
    }
    if current.golden_step_ms > budgets.step_ratio * parent.golden_step_ms {
        reasons.push(format!(
            "golden step: {:.3} ms is over {}× the parent's {:.3} ms",
            current.golden_step_ms, budgets.step_ratio, parent.golden_step_ms
        ));
    }
    reasons
}

/// The largest `ms` of every `[gate perf]` line that carries a timing: the perf games' slowest
/// decision and each wide board's.
fn slowest_decision(stdout: &str) -> Option<f64> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("[gate perf] "))
        .filter_map(|rest| rest.find('{').map(|at| &rest[at..]))
        .filter_map(|json| serde_json::from_str::<Value>(json).ok())
        .filter_map(|timing| timing["ms"].as_f64())
        .reduce(f64::max)
}

/// Runs `bin args` with `env` added and answers its output and its wall time in ms.
fn run_timed(bin: &Path, args: &[&str], env: &[(&str, &str)]) -> Result<(Output, f64)> {
    let started = Instant::now();
    let output = Process::new(bin)
        .args(args)
        .envs(env.iter().copied())
        .output()
        .with_context(|| format!("running {} {}", bin.display(), args.join(" ")))?;
    Ok((output, started.elapsed().as_secs_f64() * MS_PER_SECOND))
}

/// The three numbers of `bin`, through commands every build has.
fn measure(bin: &Path) -> Result<BenchNumbers> {
    let name = bin.display();
    println!("luau bench: measuring {name}");

    let fuzz_seeds = BENCH_FUZZ_SEEDS.to_string();
    let (fuzzed, fuzz_ms) = run_timed(bin, &["fuzz", "--seeds", &fuzz_seeds], &[])?;
    if !fuzzed.status.success() {
        bail!(
            "{name} fuzz --seeds {fuzz_seeds} failed:\n{}{}",
            String::from_utf8_lossy(&fuzzed.stdout),
            String::from_utf8_lossy(&fuzzed.stderr)
        );
    }

    // `gate` may fail an AI quality gate on a build that is slower or weaker; its timings stand.
    let (gated, _) = run_timed(bin, &["gate"], &[])?;
    let slowest_decision_ms = slowest_decision(&String::from_utf8_lossy(&gated.stdout))
        .ok_or_else(|| anyhow!("{name} gate printed no [gate perf] timing"))?;

    // This checkout's traces for both builds, so the same steps are replayed on each.
    let (replayed, replay_ms) = run_timed(
        bin,
        &["golden", "check", "--file", GAMES_PATH],
        &[("RAYON_NUM_THREADS", BENCH_GOLDEN_THREADS)],
    )?;
    let replayed_out = String::from_utf8_lossy(&replayed.stdout);
    let steps = match summary_counts(&replayed_out) {
        Some((steps, _)) if replayed.status.success() && steps > 0 => steps,
        _ => bail!(
            "{name} golden check --file {GAMES_PATH} did not replay clean:\n{replayed_out}{}",
            String::from_utf8_lossy(&replayed.stderr)
        ),
    };
    Ok(BenchNumbers {
        fuzz_ms,
        slowest_decision_ms,
        golden_step_ms: replay_ms / steps as f64,
    })
}

/// The machine the numbers are from.
#[derive(Serialize)]
struct Machine {
    os: &'static str,
    arch: &'static str,
    threads: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu: Option<String>,
}

impl Machine {
    fn here() -> Machine {
        Machine {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            threads: std::thread::available_parallelism().map_or(1, usize::from),
            cpu: cpu_model(),
        }
    }
}

/// The text after the colon of the first `model name` line of `CPUINFO`.
fn cpu_model() -> Option<String> {
    fs::read_to_string(CPUINFO)
        .ok()?
        .lines()
        .find(|line| line.starts_with("model name"))
        .and_then(|line| line.split_once(':'))
        .map(|(_, model)| model.trim().to_string())
}

/// What `--record` writes (part 4 or part 7 makes `docs/v0.4.0/perf.json` of it at the branch point).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchRecord {
    date: String,
    machine: Machine,
    parent_bin: String,
    parent: BenchNumbers,
    current: BenchNumbers,
    budgets: BenchBudgets,
    pass: bool,
    failures: Vec<String>,
}

fn run_bench(args: &BenchArgs) -> Result<()> {
    let parent = measure(&args.parent_bin)?;
    let this_bin = std::env::current_exe().context("finding this binary")?;
    let current = measure(&this_bin)?;

    let machine = Machine::here();
    println!(
        "luau bench: parent {} against this build {} on {} {} ({} threads{})",
        args.parent_bin.display(),
        this_bin.display(),
        machine.os,
        machine.arch,
        machine.threads,
        machine
            .cpu
            .as_deref()
            .map(|cpu| format!(", {cpu}"))
            .unwrap_or_default()
    );
    for (label, parent_ms, current_ms) in [
        ("fuzz --seeds 200", parent.fuzz_ms, current.fuzz_ms),
        (
            "slowest decision",
            parent.slowest_decision_ms,
            current.slowest_decision_ms,
        ),
        ("golden step", parent.golden_step_ms, current.golden_step_ms),
    ] {
        println!(
            "  {label:<20} parent {parent_ms:>10.3} ms  this {current_ms:>10.3} ms  ×{:.2}",
            current_ms / parent_ms
        );
    }

    let failures = bench_failures(&current, &parent, &BENCH_BUDGETS);
    if let Some(path) = &args.record {
        let record = BenchRecord {
            date: crate::arena::utc_date(),
            machine,
            parent_bin: args.parent_bin.display().to_string(),
            parent,
            current,
            budgets: BENCH_BUDGETS,
            pass: failures.is_empty(),
            failures: failures.clone(),
        };
        if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(&record)?;
        fs::write(path, format!("{text}\n")).with_context(|| format!("writing {}", path.display()))?;
    }
    if failures.is_empty() {
        println!("luau bench: within every budget");
        return Ok(());
    }
    bail!("luau bench: over budget:\n  {}", failures.join("\n  "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent() -> BenchNumbers {
        BenchNumbers {
            fuzz_ms: 100.0,
            slowest_decision_ms: 100.0,
            golden_step_ms: 100.0,
        }
    }

    fn current(fuzz_ms: f64, slowest_decision_ms: f64, golden_step_ms: f64) -> BenchNumbers {
        BenchNumbers {
            fuzz_ms,
            slowest_decision_ms,
            golden_step_ms,
        }
    }

    fn failures(current: BenchNumbers, parent: BenchNumbers) -> Vec<String> {
        bench_failures(&current, &parent, &BENCH_BUDGETS)
    }

    #[test]
    fn every_number_at_or_under_its_budget_passes() {
        assert!(failures(current(200.0, 150.0, 200.0), parent()).is_empty());
        assert!(failures(current(199.5, 149.5, 199.5), parent()).is_empty());
        assert!(failures(current(50.0, 50.0, 50.0), parent()).is_empty());
    }

    #[test]
    fn the_slowest_decision_fails_over_one_and_a_half_times_the_parents() {
        let reasons = failures(current(100.0, 150.5, 100.0), parent());
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].starts_with("slowest decision"), "{reasons:?}");
    }

    #[test]
    fn the_slowest_decision_fails_at_or_over_the_gates_cap() {
        let slow_parent = current(100.0, 1400.0, 100.0);
        assert!(failures(current(100.0, 1499.5, 100.0), slow_parent).is_empty());
        for ms in [1500.0, 1600.0] {
            let reasons = failures(current(100.0, ms, 100.0), slow_parent);
            assert_eq!(reasons.len(), 1, "{ms}: {reasons:?}");
            assert!(reasons[0].contains("AI_GATE.max_decision_ms"), "{reasons:?}");
        }
    }

    #[test]
    fn fuzz_fails_over_twice_the_parents() {
        let reasons = failures(current(200.5, 100.0, 100.0), parent());
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].starts_with("fuzz --seeds"), "{reasons:?}");
    }

    #[test]
    fn the_golden_step_fails_over_twice_the_parents() {
        let reasons = failures(current(100.0, 100.0, 200.5), parent());
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(reasons[0].starts_with("golden step"), "{reasons:?}");
    }

    #[test]
    fn every_breach_is_reported() {
        let reasons = failures(current(300.0, 1600.0, 300.0), parent());
        assert_eq!(reasons.len(), 4, "{reasons:?}");
    }

    #[test]
    fn the_golden_summary_gives_the_steps_and_the_divergences() {
        assert_eq!(
            summary_counts("golden check: 240 of 240 games replayed identically (22571 steps), 0 diverged\n"),
            Some((22571, 0))
        );
        assert_eq!(
            summary_counts(
                "seed jackioh-fuzz-1 step 0: `s` expected 00000000, Rust has 1a2b3c4d\n\
                 golden check: 1 of 2 games replayed identically (95 steps), 1 diverged\n"
            ),
            Some((95, 1))
        );
        assert_eq!(summary_counts("hotseat fixture: folds to a798906b"), None);
    }

    #[test]
    fn the_slowest_decision_is_the_largest_ms_of_every_perf_line() {
        let stdout = "gate perf: one decision at AI_BUDGET (smoke: 1 game(s))\n\
            [gate perf] 21 decision(s); slowest: {\"game\":1,\"turn\":7,\"ms\":98.5,\"rawMs\":12.0,\"nodes\":900,\"reason\":\"budget\"}\n\
            [gate perf] wide-hard: {\"ms\":468.25,\"rawMs\":50.0,\"nodes\":900,\"reason\":\"budget\"}\n\
            [gate perf] wide-easy: {\"ms\":292.75,\"rawMs\":30.0,\"nodes\":900,\"reason\":\"budget\"}\n";
        assert_eq!(slowest_decision(stdout), Some(468.25));
        assert_eq!(
            slowest_decision("gate perf: nothing timed\nsomething else\n"),
            None
        );
    }
}
