//! One turn at a time per account on its economy (SPEC §14.11, R2378000, issues #2378 and #2610).
//! Each of the economy's spend routes (a pack bought or opened, a token redeemed, a cosmetic pack
//! opened, a chase bought) and each other route that writes under the account's economy lock for a
//! press (a card's picks saved or uploaded, a practice seed asked for or given back, a quest
//! replaced) claims its account's turn before it reads the store and holds it until it answers.
//! `ECONOMY_SPENDS_RUNNING_MAX` run at once; a request sent while they run waits its turn here, in
//! the order sent, holding no database connection, so a burst of presses no longer holds the pool's
//! connections waiting on the account's economy lock (R1301003) until every checkout times out. Up
//! to `ECONOMY_SPENDS_WAITING_MAX` wait; one sent past them is refused at once, 429 `rate_limited`
//! stating `SPEND_BUSY_RETRY_AFTER_MS` (R192), and writes nothing.
//!
//! The turns live in this process, one set per `App` (`App::spends`), so parallel tests never share
//! them; while a deploy hands over, two processes keep their own, and the economy lock still orders
//! every writer of an account's economy. A spend's place is given back however its request ends (an
//! answer, a refusal, a failure, or a wait dropped with its request), since dropping it gives it
//! back. The mutex is held only for an in-memory check and write, never across an `.await`.

use std::sync::{Arc, Mutex, MutexGuard};

use indexmap::IndexMap;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::api::http::{ApiError, rate_limited};
use crate::config::{ECONOMY_SPENDS_RUNNING_MAX, ECONOMY_SPENDS_WAITING_MAX, SPEND_BUSY_RETRY_AFTER_MS};

/// What a spend sent past the waiting ones is told (R2378000).
pub const SPEND_BUSY_MESSAGE: &str =
    "your last purchase or opening is still going through; try again in a moment";

/// Every account's spends, one set per `App` (`App::spends`).
#[derive(Default)]
pub struct SpendSlots {
    accounts: Mutex<IndexMap<String, Account>>,
}

/// One account's spends: the turns its running spends hold, and how many spends run or wait.
struct Account {
    turns: Arc<Semaphore>,
    claims: usize,
}

impl SpendSlots {
    fn lock(&self) -> MutexGuard<'_, IndexMap<String, Account>> {
        self.accounts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The account's turn (R2378000): at once while fewer than `ECONOMY_SPENDS_RUNNING_MAX` of its
    /// spends run, else once the spends sent before it have answered. One sent while
    /// `ECONOMY_SPENDS_WAITING_MAX` already wait is refused at once with 429 `rate_limited`.
    pub async fn claim(&self, profile_id: &str) -> Result<SpendSlot<'_>, ApiError> {
        let turns = {
            let mut accounts = self.lock();
            let account = accounts.entry(profile_id.to_string()).or_insert_with(|| Account {
                turns: Arc::new(Semaphore::new(ECONOMY_SPENDS_RUNNING_MAX)),
                claims: 0,
            });
            if account.claims >= ECONOMY_SPENDS_RUNNING_MAX + ECONOMY_SPENDS_WAITING_MAX {
                tracing::info!(event = "economy.spend_busy", "profileId" = %profile_id);
                return Err(rate_limited(SPEND_BUSY_MESSAGE, SPEND_BUSY_RETRY_AFTER_MS));
            }
            account.claims += 1;
            Arc::clone(&account.turns)
        };
        // Counted from here, so a wait dropped with its request gives its place back too.
        let place = Place {
            slots: self,
            profile_id: profile_id.to_string(),
        };
        let turn = turns
            .acquire_owned()
            .await
            .map_err(|_| ApiError::internal("an account's spend turns were closed"))?;
        Ok(SpendSlot {
            _turn: turn,
            _place: place,
        })
    }

    /// How many of the account's spends run or wait.
    pub fn claims(&self, profile_id: &str) -> usize {
        self.lock().get(profile_id).map_or(0, |account| account.claims)
    }
}

/// A spend's place among its account's, running or waiting. Dropped, it is given back, and an
/// account with no spend left is forgotten.
struct Place<'a> {
    slots: &'a SpendSlots,
    profile_id: String,
}

impl Drop for Place<'_> {
    fn drop(&mut self) {
        let mut accounts = self.slots.lock();
        let gone = match accounts.get_mut(&self.profile_id) {
            Some(account) => {
                account.claims = account.claims.saturating_sub(1);
                account.claims == 0
            }
            None => false,
        };
        if gone {
            accounts.swap_remove(&self.profile_id);
        }
    }
}

/// A spend's turn (R2378000), held until it answers. Dropped, the turn passes to the spend of the
/// account that has waited longest.
#[must_use = "a turn dropped at once lets the next spend run beside this one"]
pub struct SpendSlot<'a> {
    // Declared first, so dropped first: the turn passes on before the place is given back.
    _turn: OwnedSemaphorePermit,
    _place: Place<'a>,
}

#[cfg(test)]
mod tests {
    use std::future::{Future, poll_fn};
    use std::pin::{Pin, pin};
    use std::task::Poll;

    use super::*;
    use crate::api::http::ApiErrorCode;

    /// Polls a claim once: `Ready` with its answer when it has its turn or is refused, `Pending`
    /// while it waits.
    async fn poll_once<F: Future>(mut claim: Pin<&mut F>) -> Poll<F::Output> {
        poll_fn(|cx| Poll::Ready(claim.as_mut().poll(cx))).await
    }

    /// A claim that must answer at once, with its turn or its refusal.
    async fn at_once<'a>(slots: &'a SpendSlots, profile_id: &str) -> Result<SpendSlot<'a>, ApiError> {
        match poll_once(pin!(slots.claim(profile_id))).await {
            Poll::Ready(answer) => answer,
            Poll::Pending => panic!("{profile_id}'s claim waits"),
        }
    }

    fn assert_busy(answer: Result<SpendSlot<'_>, ApiError>) {
        let Err(error) = answer else {
            panic!("a spend sent past the waiting ones was let in");
        };
        assert_eq!(error.code, ApiErrorCode::RateLimited);
        assert_eq!(error.message, SPEND_BUSY_MESSAGE);
        assert_eq!(error.retry_after_ms, Some(SPEND_BUSY_RETRY_AFTER_MS));
    }

    #[tokio::test]
    async fn r2378000_a_spend_sent_while_one_runs_waits_its_turn_then_runs() {
        let slots = SpendSlots::default();
        let running = at_once(&slots, "p1").await.expect("the first spend runs");
        let mut waiting = pin!(slots.claim("p1"));
        assert!(poll_once(waiting.as_mut()).await.is_pending(), "the second waits");
        assert_eq!(slots.claims("p1"), 2);

        drop(running);
        let Poll::Ready(Ok(next)) = poll_once(waiting.as_mut()).await else {
            panic!("the second runs once the first has answered");
        };
        assert_eq!(slots.claims("p1"), 1);
        drop(next);
        assert_eq!(slots.claims("p1"), 0);
        assert!(slots.lock().is_empty(), "an account with no spend is forgotten");
    }

    #[tokio::test]
    async fn r2378000_a_spend_sent_past_the_waiting_ones_is_refused_at_once() {
        let slots = SpendSlots::default();
        let mut running = Vec::new();
        for _ in 0..ECONOMY_SPENDS_RUNNING_MAX {
            running.push(at_once(&slots, "p1").await.expect("a running spend"));
        }
        let mut waiting = Vec::new();
        for _ in 0..ECONOMY_SPENDS_WAITING_MAX {
            let mut claim = Box::pin(slots.claim("p1"));
            assert!(poll_once(claim.as_mut()).await.is_pending());
            waiting.push(claim);
        }
        let full = ECONOMY_SPENDS_RUNNING_MAX + ECONOMY_SPENDS_WAITING_MAX;
        assert_busy(at_once(&slots, "p1").await);
        assert_eq!(slots.claims("p1"), full, "a refusal takes no place");

        drop(waiting);
        assert_eq!(
            slots.claims("p1"),
            ECONOMY_SPENDS_RUNNING_MAX,
            "a wait dropped with its request gives its place back"
        );
        let mut again = pin!(slots.claim("p1"));
        assert!(
            poll_once(again.as_mut()).await.is_pending(),
            "the next spend waits again instead of being refused"
        );
        drop(running);
        assert!(matches!(poll_once(again.as_mut()).await, Poll::Ready(Ok(_))));
        assert_eq!(slots.claims("p1"), 0);
    }

    #[tokio::test]
    async fn r2378000_each_account_has_its_own_turns() {
        let slots = SpendSlots::default();
        let mut p1 = Vec::new();
        for _ in 0..ECONOMY_SPENDS_RUNNING_MAX {
            p1.push(at_once(&slots, "p1").await.expect("p1's spend"));
        }
        let p2 = at_once(&slots, "p2").await;
        assert!(p2.is_ok(), "p2 runs while p1's spends run");
        assert_eq!(
            (slots.claims("p1"), slots.claims("p2")),
            (ECONOMY_SPENDS_RUNNING_MAX, 1)
        );
    }
}
