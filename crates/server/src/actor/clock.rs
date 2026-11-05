//! The tokio half of the match clock (BUILD M7-T1, SPEC §9.5, R79).
//!
//! The clock itself — which deadlines run, what an expiry becomes — is `jackioh_shared::host`'s, pure
//! (#3101): it states the timers it wants as data and is told when one comes due. This module is what
//! keeps a wait on `tokio::time`: [`Timer`], one scheduled callback that is cancelled by dropping it, and
//! [`after`], which starts one. `now_ms()` reads tokio's clock (SURFACE §11.2), so a test's
//! `tokio::time::pause()` and `advance()` move every deadline.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub use jackioh_shared::host::clock::{initial_clocks, match_ceiling_at};

static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// One scheduled callback. Dropping it cancels it. The callback is handed its timer's id, so a
/// callback already running when it was canceled can see that it is stale.
pub(crate) struct Timer {
    id: u64,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Timer {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn cancel(self) {
        drop(self);
    }

    /// The callback has run: let go of the timer without aborting the task that is running it.
    pub(crate) fn fired(mut self) {
        self.handle = None;
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

/// Runs `f` once `ms` milliseconds of tokio time have passed (at once for `ms <= 0`). Must be
/// called inside a tokio runtime.
pub(crate) fn after(ms: i64, f: impl FnOnce(u64) + Send + 'static) -> Timer {
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let wait = Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0));
    let handle = tokio::spawn(async move {
        tokio::time::sleep(wait).await;
        f(id);
    });
    Timer {
        id,
        handle: Some(handle),
    }
}
