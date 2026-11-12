//! The background loops' supervisor (#842). `app::serve` spawns each of its loops through
//! `spawn_supervised`: SPEC §9.5's matchmaker sweep, series sweeper and reaper, the retention purge,
//! the bot pool (R1454) and the store's heartbeat (#3208). A panic out of a loop is logged as
//! `loop.crashed`, and a return, since every loop runs until aborted, as `loop.returned` (#1093), both
//! alerts; either way the loop starts again `LOOP_RESTART_DELAY_SECONDS` later. A loop's per-tick
//! `Result` handling is its own, except that the sweeps feed each step's result to a `FailingStreak`
//! (#3770), which raises one alert for a step that fails on every pass. No SPEC section or ruling:
//! restarting and alerting are operational, not a rule.
//!
//! The panic is caught on each poll of the loop's future (`catch_unwind`), as `actor/registry.rs`
//! catches the engine's, so the loop runs on the supervisor's own task and aborting the returned
//! handle stops both: an abort during the restart wait starts no replacement.

use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::task::Poll;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::actor::registry::panic_text;
use crate::app::now_ms;
use crate::config::{LOOP_FAILING_ALERT_MS, LOOP_FAILING_ALERT_PASSES, LOOP_RESTART_DELAY_SECONDS};

/// Spawns `run(context.clone())`, restarted with a fresh clone each time it panics or returns. The
/// task ends only when its handle is aborted.
pub fn spawn_supervised<T, F, Fut>(name: &'static str, context: T, run: F) -> JoinHandle<()>
where
    T: Clone + Send + 'static,
    F: Fn(T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            let mut running = Box::pin(run(context.clone()));
            // `AssertUnwindSafe`: a loop that panicked is dropped, never polled again.
            let crashed = std::future::poll_fn(|cx| {
                match catch_unwind(AssertUnwindSafe(|| running.as_mut().poll(cx))) {
                    Ok(Poll::Pending) => Poll::Pending,
                    Ok(Poll::Ready(())) => Poll::Ready(None),
                    Err(payload) => Poll::Ready(Some(payload)),
                }
            })
            .await;
            drop(running);
            match crashed {
                Some(payload) => {
                    tracing::error!(event = "loop.crashed", alert = true, "loop" = name, message = %panic_text(payload));
                }
                None => tracing::error!(event = "loop.returned", alert = true, "loop" = name),
            }
            tokio::time::sleep(Duration::from_secs(
                u64::try_from(LOOP_RESTART_DELAY_SECONDS).unwrap_or(0),
            ))
            .await;
            tracing::warn!(event = "loop.restarted", "loop" = name);
        }
    })
}

/// One sweep step's run of failed passes (#3770). A step that fails on every pass has its warning on
/// every pass and nobody told, so each sweep loop holds one `FailingStreak` per step and feeds it each
/// pass's result. Once the step has failed on at least `LOOP_FAILING_ALERT_PASSES` passes in a row
/// over at least `LOOP_FAILING_ALERT_MS`, it raises one alert, `<step>.failing`; the next success
/// logs `<step>.recovered` at info, only for a streak that alerted, and a later streak alerts again.
///
/// The count lives in the loop's future, so a restart of the loop (or of the process) starts it
/// afresh, as `ALERT_WINDOW_MS`'s one notice an hour per event is per process.
pub struct FailingStreak {
    step: &'static str,
    failing_event: String,
    recovered_event: String,
    passes: u32,
    since: i64,
    alerted: bool,
}

impl FailingStreak {
    /// A streak for `step`, whose alert is `<step>.failing` and whose recovery is `<step>.recovered`.
    /// An alert issue shows only the event name, so the step is spelled out in it.
    pub fn new(step: &'static str) -> Self {
        Self {
            step,
            failing_event: format!("{step}.failing"),
            recovered_event: format!("{step}.recovered"),
            passes: 0,
            since: 0,
            alerted: false,
        }
    }

    /// Feeds one pass's result and hands it back for the caller's own handling.
    pub fn track<T, E: std::fmt::Display>(&mut self, result: Result<T, E>) -> Result<T, E> {
        match &result {
            Ok(_) => self.ok(),
            Err(error) => self.failed(&error.to_string()),
        }
        result
    }

    /// One failed pass; `message` is what the alert says when this pass completes the streak.
    pub fn failed(&mut self, message: &str) {
        let now = now_ms();
        if self.passes == 0 {
            self.since = now;
        }
        self.passes = self.passes.saturating_add(1);
        if self.alerted || self.passes < LOOP_FAILING_ALERT_PASSES || now - self.since < LOOP_FAILING_ALERT_MS
        {
            return;
        }
        self.alerted = true;
        tracing::error!(
            event = self.failing_event.as_str(),
            alert = true,
            step = self.step,
            passes = self.passes,
            failingForMs = now - self.since,
            message = %message
        );
    }

    /// One good pass: it ends the streak, and logs the recovery if the streak had alerted.
    pub fn ok(&mut self) {
        if self.alerted {
            tracing::info!(
                event = self.recovered_event.as_str(),
                step = self.step,
                passes = self.passes,
                failedForMs = now_ms() - self.since
            );
        }
        self.passes = 0;
        self.alerted = false;
    }
}
