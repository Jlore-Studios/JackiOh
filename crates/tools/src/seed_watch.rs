//! The fuzz waves' watch (#3158). A wave's seeds run under rayon and its summary prints only once the
//! last one ends, so super.yml's `fuzz` job, cut off at 90 minutes, left a log naming no seed. `watched`
//! prints to stderr, every `HEARTBEAT_SECS`, how many seeds are done and which have run longest. When
//! one seed has run past `SEED_STALL_BOUND_SECS`, it prints that seed and the command that plays it
//! alone, then ends the process with exit code 1. After the wave it prints the slowest seed. Only the
//! wall clock is read, and never by a game.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError, mpsc};
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use rayon::prelude::*;

/// How often, in seconds, a running wave prints its progress line to stderr.
pub const HEARTBEAT_SECS: u64 = 60;

/// How long, in seconds, one seed may run before the wave stops as stalled (#3158). A wave's slowest
/// seed is far under it: on four cores, with super.yml's quarter of 2,500 seeds a wave (500 for Duos),
/// shard 1's waves took 519 s, 557 s (`--handicap`) and 293 s (`--duos`) and shard 3's 527 s, 562 s
/// and 304 s; measured again with this watch, shard 1's took 505 s, 548 s and 283 s, their slowest
/// seeds 6.6 s (seed 235), 18.3 s (seed 541) and 10.7 s (seed 135). It is that far over so that a
/// loaded runner's wall time never fails a healthy seed (`CHECKPOINT_TIME_BOUND_MS` was loosened for
/// that once, #1803), and a stall still stops within `HEARTBEAT_SECS` of passing it, well inside the
/// job's 90 minutes.
pub const SEED_STALL_BOUND_SECS: u64 = 600;

/// How many of the seeds running longest a progress line names.
pub const IN_FLIGHT_SHOWN: usize = 3;

/// What the wave's seeds share with the watching thread.
#[derive(Default)]
struct Watch {
    /// The seeds running now, each with the instant it began.
    running: Mutex<IndexMap<u32, Instant>>,
    /// How many seeds have ended.
    done: AtomicU32,
    /// The slowest seed that has ended, and how long it took.
    slowest: Mutex<Option<(u32, Duration)>>,
}

/// The mutex's value, a poisoned one's included: a seed that panicked holding it changed nothing the
/// watch cannot read.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `run` over seeds `from..from + ran` under rayon, results in seed order, watched as the header says.
pub(crate) fn watched<T: Send>(
    label: &str,
    from: u32,
    ran: u32,
    reproduce: impl Fn(u32) -> String + Sync,
    run: impl Fn(u32) -> T + Sync + Send,
) -> Vec<T> {
    let watch = Watch::default();
    let shared = &watch;
    let reproduce = &reproduce;
    let started = Instant::now();
    let (stop, stopped) = mpsc::channel::<()>();
    let results = std::thread::scope(|scope| {
        scope.spawn(move || {
            while let Err(mpsc::RecvTimeoutError::Timeout) =
                stopped.recv_timeout(Duration::from_secs(HEARTBEAT_SECS))
            {
                let now = Instant::now();
                let ages: Vec<(u32, Duration)> = lock(&shared.running)
                    .iter()
                    .map(|(seed, began)| (*seed, now.duration_since(*began)))
                    .collect();
                let bound = Duration::from_secs(SEED_STALL_BOUND_SECS);
                if let Some(line) = stall_line(label, &ages, bound, reproduce) {
                    eprintln!("{line}");
                    std::process::exit(1);
                }
                let done = shared.done.load(Ordering::Relaxed);
                eprintln!("{}", heartbeat_line(label, done, ran, started.elapsed(), &ages));
            }
        });
        let results: Vec<T> = (from..from + ran)
            .into_par_iter()
            .map(|seed| {
                let began = Instant::now();
                lock(&shared.running).insert(seed, began);
                let result = run(seed);
                lock(&shared.running).swap_remove(&seed);
                shared.done.fetch_add(1, Ordering::Relaxed);
                keep_slowest(&mut lock(&shared.slowest), seed, began.elapsed());
                result
            })
            .collect();
        drop(stop);
        results
    });
    eprintln!(
        "{}",
        finished_line(label, ran, started.elapsed(), *lock(&watch.slowest))
    );
    results
}

/// A copy of `ages`, the longest-running seed first; of two that have run as long, the lower seed.
fn longest_first(ages: &[(u32, Duration)]) -> Vec<(u32, Duration)> {
    let mut sorted = ages.to_vec();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted
}

/// The line that stops the wave when its longest-running seed has run past `bound`, naming the seed
/// and the command that plays it alone; `None` while every seed is at or under it.
fn stall_line(
    label: &str,
    ages: &[(u32, Duration)],
    bound: Duration,
    reproduce: &dyn Fn(u32) -> String,
) -> Option<String> {
    let (seed, age) = *longest_first(ages).first()?;
    (age > bound).then(|| {
        format!(
            "[{label}] seed {seed} has run {}s, past SEED_STALL_BOUND_SECS ({}s): it stalled, so the wave stops here, failing\n  reproduce: {}",
            age.as_secs(),
            bound.as_secs(),
            reproduce(seed)
        )
    })
}

/// The progress line: how many of the wave's seeds are done, how long it has run, and the
/// `IN_FLIGHT_SHOWN` seeds running longest (`none` when no seed is running).
fn heartbeat_line(label: &str, done: u32, ran: u32, elapsed: Duration, ages: &[(u32, Duration)]) -> String {
    let longest: Vec<String> = longest_first(ages)
        .into_iter()
        .take(IN_FLIGHT_SHOWN)
        .map(|(seed, age)| format!("seed {seed} ({}s)", age.as_secs()))
        .collect();
    let list = if longest.is_empty() {
        "none".to_string()
    } else {
        longest.join(", ")
    };
    format!(
        "[{label}] {done} of {ran} seeds done in {}s; running longest: {list}",
        elapsed.as_secs()
    )
}

/// Keeps `seed` as the slowest when none is kept yet, when it took longer, or when it took as long and
/// is the lower seed, so the line names the same seed whichever thread ended first.
fn keep_slowest(slowest: &mut Option<(u32, Duration)>, seed: u32, took: Duration) {
    let replaces = match *slowest {
        None => true,
        Some((kept, kept_took)) => took > kept_took || (took == kept_took && seed < kept),
    };
    if replaces {
        *slowest = Some((seed, took));
    }
}

/// The line after the wave: how many seeds it ran, how long it took, and its slowest seed.
fn finished_line(label: &str, ran: u32, elapsed: Duration, slowest: Option<(u32, Duration)>) -> String {
    let slowest = slowest.map_or_else(
        || "none".to_string(),
        |(seed, took)| format!("seed {seed}, {}ms", took.as_millis()),
    );
    format!(
        "[{label}] {ran} seeds in {}s; slowest: {slowest}",
        elapsed.as_secs()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn fuzz_command(seed: u32) -> String {
        format!("cargo jackioh fuzz --from {seed} --seeds 1")
    }

    #[test]
    fn a_watched_wave_returns_every_result_in_seed_order() {
        let expected: Vec<u32> = (5..45).map(|s| s * 3).collect();
        assert_eq!(
            watched("test", 5, 40, |s| format!("again {s}"), |s| s * 3),
            expected
        );
        let one_thread = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let alone = one_thread.install(|| watched("test", 5, 40, |s| format!("again {s}"), |s| s * 3));
        assert_eq!(alone, expected);
    }

    #[test]
    fn a_seed_past_the_bound_is_named_with_its_command() {
        let ages = [(3, secs(5)), (812, secs(601)), (9, secs(700))];
        let line = stall_line("w", &ages, secs(600), &fuzz_command).expect("seed 9 is past the bound");
        assert!(line.contains("seed 9 has run 700s"), "{line}");
        assert!(line.contains("SEED_STALL_BOUND_SECS (600s)"), "{line}");
        assert!(
            line.contains("reproduce: cargo jackioh fuzz --from 9 --seeds 1"),
            "{line}"
        );
    }

    #[test]
    fn no_seed_at_or_under_the_bound_is_a_stall() {
        assert_eq!(stall_line("w", &[(1, secs(600))], secs(600), &fuzz_command), None);
        assert_eq!(stall_line("w", &[], secs(600), &fuzz_command), None);
    }

    #[test]
    fn a_heartbeat_counts_the_done_seeds_and_names_the_longest_running() {
        let ages = [(1, secs(1)), (2, secs(9)), (3, secs(4)), (4, secs(9))];
        assert_eq!(
            heartbeat_line("w", 1200, 2500, secs(300), &ages),
            "[w] 1200 of 2500 seeds done in 300s; running longest: seed 2 (9s), seed 4 (9s), seed 3 (4s)"
        );
        assert!(
            heartbeat_line("w", 2500, 2500, secs(520), &[]).ends_with("running longest: none"),
            "{}",
            heartbeat_line("w", 2500, 2500, secs(520), &[])
        );
    }

    #[test]
    fn the_slowest_seed_is_kept_and_a_tie_keeps_the_lower_seed() {
        let mut slowest = None;
        keep_slowest(&mut slowest, 9, Duration::from_millis(1500));
        keep_slowest(&mut slowest, 4, Duration::from_millis(900));
        assert_eq!(slowest, Some((9, Duration::from_millis(1500))));
        keep_slowest(&mut slowest, 7, Duration::from_millis(1500));
        assert_eq!(slowest, Some((7, Duration::from_millis(1500))));
        keep_slowest(&mut slowest, 8, Duration::from_millis(1500));
        assert_eq!(slowest, Some((7, Duration::from_millis(1500))));
        assert_eq!(
            finished_line("w", 40, secs(12), slowest),
            "[w] 40 seeds in 12s; slowest: seed 7, 1500ms"
        );
        assert_eq!(
            finished_line("w", 40, secs(12), None),
            "[w] 40 seeds in 12s; slowest: none"
        );
    }
}
