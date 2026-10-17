//! `loops::spawn_supervised` (#842, #1093): a loop that panics or returns is logged under its
//! name and started again `LOOP_RESTART_DELAY_SECONDS` later, and aborting its handle stops it
//! for good. Paused tokio time, so every tick lands at a known instant.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;

use jackioh_server::config::LOOP_RESTART_DELAY_SECONDS;
use jackioh_server::loops::spawn_supervised;

use crate::support::deps::{RecordingLogger, record_logs};

/// The test loop's cadence.
const TICK: Duration = Duration::from_secs(1);

fn restart_delay() -> Duration {
    Duration::from_secs(u64::try_from(LOOP_RESTART_DELAY_SECONDS).expect("a positive delay"))
}

/// Ticks every `TICK`, counting each tick in `ticks`, and panics in the first tick it ever runs.
async fn panics_in_its_first_tick(ticks: Arc<AtomicUsize>) {
    loop {
        tokio::time::sleep(TICK).await;
        if ticks.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("the first tick fails");
        }
    }
}

/// Ticks once, a `TICK` after it starts, counting the tick in `ticks`, and returns.
async fn returns_after_its_first_tick(ticks: Arc<AtomicUsize>) {
    tokio::time::sleep(TICK).await;
    ticks.fetch_add(1, Ordering::SeqCst);
}

/// Ticks every `TICK`, counting each tick in `ticks`, and never ends on its own.
async fn ticks_forever(ticks: Arc<AtomicUsize>) {
    loop {
        tokio::time::sleep(TICK).await;
        ticks.fetch_add(1, Ordering::SeqCst);
    }
}

/// The supervisor's lines (`loop.*`), in the order they were logged.
fn loop_events(logs: &RecordingLogger) -> Vec<String> {
    logs.entries()
        .into_iter()
        .map(|entry| entry.event)
        .filter(|event| event.starts_with("loop."))
        .collect()
}

#[tokio::test(start_paused = true)]
async fn a_loop_that_panics_is_logged_and_restarted() {
    let logs = record_logs();
    let ticks = Arc::new(AtomicUsize::new(0));
    let handle = spawn_supervised("test.ticker", Arc::clone(&ticks), panics_in_its_first_tick);

    // Tick 1 (at 1 s) panics: the crash is logged, and the restart waits out the delay.
    tokio::time::sleep(TICK + TICK / 2).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 1);
    let crashed = logs.named("loop.crashed");
    assert_eq!(crashed.len(), 1);
    assert_eq!(crashed[0].level, "alert");
    assert_eq!(crashed[0].data["loop"], json!("test.ticker"));
    assert_eq!(crashed[0].data["message"], json!("the first tick fails"));
    assert!(logs.named("loop.restarted").is_empty());

    // Restarted at 1 s + the delay, the loop ticks again a `TICK` later, and once more after that.
    tokio::time::sleep(restart_delay() + TICK * 2).await;
    handle.abort();
    assert_eq!(ticks.load(Ordering::SeqCst), 3);
    let restarted = logs.named("loop.restarted");
    assert_eq!(restarted.len(), 1);
    assert_eq!(restarted[0].level, "warn");
    assert_eq!(restarted[0].data["loop"], json!("test.ticker"));
    assert_eq!(logs.named("loop.crashed").len(), 1);
    assert!(logs.named("loop.returned").is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_loop_that_returns_is_alerted_and_restarted() {
    let logs = record_logs();
    let ticks = Arc::new(AtomicUsize::new(0));
    let handle = spawn_supervised("test.once", Arc::clone(&ticks), returns_after_its_first_tick);

    // Tick 1 (at 1 s) returns: the return is an alert under the loop's name, and the restart
    // waits out the delay.
    tokio::time::sleep(TICK + TICK / 2).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 1);
    let returned = logs.named("loop.returned");
    assert_eq!(returned.len(), 1);
    assert_eq!(returned[0].level, "alert");
    assert_eq!(returned[0].data["loop"], json!("test.once"));
    assert!(logs.named("loop.restarted").is_empty());

    // Still waiting, half a `TICK` before the restart at 1 s + the delay.
    tokio::time::sleep(restart_delay() - TICK).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 1);
    assert!(logs.named("loop.restarted").is_empty());

    // Restarted at 1 s + the delay, the loop ticks and returns again a `TICK` later.
    tokio::time::sleep(TICK * 2).await;
    handle.abort();
    assert_eq!(ticks.load(Ordering::SeqCst), 2);
    let restarted = logs.named("loop.restarted");
    assert_eq!(restarted.len(), 1);
    assert_eq!(restarted[0].level, "warn");
    assert_eq!(restarted[0].data["loop"], json!("test.once"));
    assert_eq!(
        loop_events(&logs),
        ["loop.returned", "loop.restarted", "loop.returned"]
    );
    assert!(logs.named("loop.crashed").is_empty());
}

#[tokio::test(start_paused = true)]
async fn aborting_a_loop_in_its_restart_wait_starts_no_replacement() {
    let logs = record_logs();
    let ticks = Arc::new(AtomicUsize::new(0));
    let handle = spawn_supervised("test.aborted", Arc::clone(&ticks), returns_after_its_first_tick);

    // The loop returned at 1 s and the supervisor is waiting out the delay.
    tokio::time::sleep(TICK + TICK / 2).await;
    assert_eq!(logs.named("loop.returned").len(), 1);
    handle.abort();
    let ended = handle.await.expect_err("an aborted supervisor does not finish");
    assert!(ended.is_cancelled());

    tokio::time::sleep(restart_delay() * 2).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 1);
    assert!(logs.named("loop.restarted").is_empty());
    assert_eq!(logs.named("loop.returned").len(), 1);
}

#[tokio::test(start_paused = true)]
async fn aborting_a_running_loop_stops_it_without_a_restart() {
    let logs = record_logs();
    let ticks = Arc::new(AtomicUsize::new(0));
    let handle = spawn_supervised("test.forever", Arc::clone(&ticks), ticks_forever);

    tokio::time::sleep(TICK * 2 + TICK / 2).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 2);
    handle.abort();
    let ended = handle.await.expect_err("an aborted supervisor does not finish");
    assert!(ended.is_cancelled());

    tokio::time::sleep(restart_delay() + TICK * 2).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 2);
    assert!(loop_events(&logs).is_empty());
}
