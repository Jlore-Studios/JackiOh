//! R4008000: the codes a caller can only learn by being told them (a room's, a co-op room's, a lobby's)
//! are held by a limit that counts **guesses**, not requests (#4008, part of #3897).
//!
//! A request carries one guess, the path's code; nothing else in a request or frame is read as a code, so
//! a batch (an array, a second field) carries no extra guesses. What is counted is the *distinct code* a
//! caller tried, **right or wrong**: one claim per (account, code) and per (address, code) in the window,
//! so a client that re-reads a lobby which has closed (the web's lobby screen, every `LOBBY_POLL_MS`)
//! spends one guess, not one a poll. Counting only the misses would let the first refusal tell the
//! caller which codes are held. The claim is made in one step, with no `await` between the check and the
//! count, and **before** the answer is given, so requests sent at once cannot all pass a check that none
//! has counted yet: past the allowance a lookup is 429 `rate_limited` whatever its outcome, a right
//! code's too. A malformed code could never be a room's, is refused unread and counts nothing. The
//! people of the code (the host or guest of the room, a member of the lobby) are not guessing, so their
//! own reads claim nothing. Kept in memory, per `App`, like the other per-address limits of §9.8.

use std::future::Future;

use crate::api::http::{
    ApiError, RateLimiter, Req, account_key, address_key, create_rate_limiter, rate_limited,
};
use crate::app::now_ms;
use crate::config::{CODE_TRIES_PER_ACCOUNT_PER_HOUR, CODE_TRIES_PER_ADDRESS_PER_HOUR, CODE_TRY_WINDOW_MS};
use crate::duos::rooms::code_of;

const REFUSED_MESSAGE: &str = "too many codes tried; try again later";

/// The allowances of the code doors, one per `App`.
pub struct CodeGuesses {
    per_account: RateLimiter,
    per_address: RateLimiter,
    /// A limit of one: whether a (who, code) pair was already claimed in the window.
    claimed: RateLimiter,
}

impl Default for CodeGuesses {
    fn default() -> Self {
        CodeGuesses {
            per_account: create_rate_limiter(CODE_TRIES_PER_ACCOUNT_PER_HOUR, CODE_TRY_WINDOW_MS),
            per_address: create_rate_limiter(CODE_TRIES_PER_ADDRESS_PER_HOUR, CODE_TRY_WINDOW_MS),
            claimed: create_rate_limiter(1, CODE_TRY_WINDOW_MS),
        }
    }
}

impl CodeGuesses {
    /// Claims the path's code for the caller and their address, or is 429 `rate_limited` once the account's
    /// or the address's allowance of distinct codes is spent. `own` is a caller reading a code that is
    /// theirs, who is not guessing. Called after the lookup and before its answer is given.
    pub fn claim(&self, req: &Req, account: &str, own: bool) -> Result<(), ApiError> {
        // Malformed: refused unread, counts nothing.
        let Some(code) = code_of(req).filter(|_| !own) else {
            return Ok(());
        };
        let now = now_ms();
        for (allowance, key) in [
            (&self.per_account, account_key(account)),
            (&self.per_address, address_key(&req.address)),
        ] {
            let pair = format!("{key}|{code}");
            // Tried already in the window: it was claimed then, and costs nothing again.
            if self.claimed.retry_after_ms(&pair, now) > 0 {
                continue;
            }
            if !allowance.allow(&key, now) {
                return Err(rate_limited(REFUSED_MESSAGE, allowance.retry_after_ms(&key, now)));
            }
            self.claimed.allow(&pair, now);
        }
        Ok(())
    }

    /// A lookup of the path's code: its answer is given only once the code is claimed.
    pub async fn lookup<T>(
        &self,
        req: &Req,
        account: &str,
        door: impl Future<Output = Result<T, ApiError>>,
    ) -> Result<T, ApiError> {
        let outcome = door.await;
        self.claim(req, account, false)?;
        outcome
    }
}
