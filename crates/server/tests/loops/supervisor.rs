//! `loops::spawn_supervised` (#842): a loop that panics is logged under its name and started again
//! `LOOP_RESTART_DELAY_SECONDS` later; one that returns is not. Paused tokio time, so every tick
//! lands at a known instant.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;

use jackioh_server::config::LOOP_RESTART_DELAY_SECONDS;
use jackioh_server::loops::spawn_supervised;

use crate::support::deps::record_logs;

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
}

#[tokio::test(start_paused = true)]
async fn a_loop_that_returns_is_not_restarted() {
    let logs = record_logs();
    let starts = Arc::new(AtomicUsize::new(0));
    let handle = spawn_supervised(
        "test.once",
        Arc::clone(&starts),
        |starts: Arc<AtomicUsize>| async move {
            starts.fetch_add(1, Ordering::SeqCst);
        },
    );
    handle.await.expect("the supervisor ends when its loop returns");
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert!(logs.named("loop.crashed").is_empty());
    assert!(logs.named("loop.restarted").is_empty());
}
