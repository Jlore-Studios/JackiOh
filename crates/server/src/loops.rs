//! The background loops' supervisor (#842). `app::serve` spawns each of its loops through
//! `spawn_supervised`: SPEC §9.5's matchmaker sweep, series sweeper and reaper, the retention purge
//! and the bot pool (R1454). A panic that unwinds out of a loop is caught, logged as `loop.crashed`
//! (an alert) under the loop's name, and the loop starts again `LOOP_RESTART_DELAY_SECONDS` later,
//! logged as `loop.restarted`. Without it the task died for good, and nothing but a stderr line said
//! so. Every loop runs until it is aborted, so one that returns has stopped doing its work: the
//! return is logged as `loop.returned` (an alert) under the loop's name, and the loop starts again
//! the same way (#1093). A loop's per-tick `Result` handling is its own and stays so: only a panic
//! or a return reaches the supervisor. Not in SPEC, and no R-row: restarting is operational, not
//! a rule.
//!
//! The panic is caught on each poll of the loop's future (`std::future::poll_fn` around
//! `catch_unwind`), as `actor/registry.rs` catches the engine's, so the loop runs on the
//! supervisor's own task and aborting the returned handle stops both: an abort during the restart
//! wait starts no replacement.

use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::task::Poll;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::actor::registry::panic_text;
use crate::config::LOOP_RESTART_DELAY_SECONDS;

/// Spawns `run(context.clone())` and, each time it panics or returns, starts it again
/// `LOOP_RESTART_DELAY_SECONDS` later with a fresh clone. The task ends only when its handle is
/// aborted.
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
