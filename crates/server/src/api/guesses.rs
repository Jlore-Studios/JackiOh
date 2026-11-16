//! R4008000: the codes a caller can only learn by being told them (a room's, a co-op room's, a lobby's)
//! are held by a limit that counts **guesses**, not requests (#4008, part of #3897).
//!
//! A request carries one guess, the path's code; nothing else in a request or frame is read as a code, so
//! a batch (an array, a second field) carries no extra guesses. What is counted is the *distinct wrong
//! code* a caller tried: one 404 per (account, code) and per (address, code) in the window, so a client
//! that re-reads a lobby which has closed (the web's lobby screen, every `LOBBY_POLL_MS`) spends one
//! guess, not one a poll. A malformed code could never be a room's, is refused unread and counts
//! nothing. Once an account's or an address's allowance is spent every code lookup is 429
//! `rate_limited` until the oldest miss leaves the window, a right code's too: counting without
//! refusing would not hold anything. The people of the code (the host or guest of the room, a member of
//! the lobby) are not guessing, so their own reads skip the gate. Kept in memory, per `App`, like the
//! other per-address limits of §9.8.

use std::future::Future;

use crate::api::http::{
    ApiError, ApiErrorCode, RateLimiter, Req, account_key, address_key, create_rate_limiter, rate_limited,
};
use crate::app::now_ms;
use crate::config::{
    CODE_MISS_WINDOW_MS, CODE_MISSES_PER_ACCOUNT_PER_HOUR, CODE_MISSES_PER_ADDRESS_PER_HOUR,
};
use crate::duos::rooms::code_of;

const REFUSED_MESSAGE: &str = "too many wrong codes; try again later";

/// The allowances of the code doors, one per `App`.
pub struct CodeGuesses {
    per_account: RateLimiter,
    per_address: RateLimiter,
    /// A limit of one: whether a (who, code) pair was already counted in the window.
    counted: RateLimiter,
}

impl Default for CodeGuesses {
    fn default() -> Self {
        CodeGuesses {
            per_account: create_rate_limiter(CODE_MISSES_PER_ACCOUNT_PER_HOUR, CODE_MISS_WINDOW_MS),
            per_address: create_rate_limiter(CODE_MISSES_PER_ADDRESS_PER_HOUR, CODE_MISS_WINDOW_MS),
            counted: create_rate_limiter(1, CODE_MISS_WINDOW_MS),
        }
    }
}

impl CodeGuesses {
    /// 429 `rate_limited` while the account's or the address's allowance of wrong codes is spent;
    /// `own` is a caller reading a code that is theirs, who is not guessing.
    pub fn gate(&self, req: &Req, account: &str, own: bool) -> Result<(), ApiError> {
        if own {
            return Ok(());
        }
        let now = now_ms();
        let wait = self
            .per_account
            .retry_after_ms(&account_key(account), now)
            .max(self.per_address.retry_after_ms(&address_key(&req.address), now));
        if wait > 0 {
            return Err(rate_limited(REFUSED_MESSAGE, wait));
        }
        Ok(())
    }

    /// Counts `outcome` as a wrong guess when it is a 404, and passes it on.
    pub fn settle<T>(
        &self,
        req: &Req,
        account: &str,
        own: bool,
        outcome: Result<T, ApiError>,
    ) -> Result<T, ApiError> {
        if !own && matches!(&outcome, Err(error) if error.code == ApiErrorCode::NotFound) {
            self.miss(req, account);
        }
        outcome
    }

    /// A lookup of the path's code: gated before it runs, a 404 counted after.
    pub async fn lookup<T>(
        &self,
        req: &Req,
        account: &str,
        door: impl Future<Output = Result<T, ApiError>>,
    ) -> Result<T, ApiError> {
        self.gate(req, account, false)?;
        let outcome = door.await;
        self.settle(req, account, false, outcome)
    }

    /// A wrong code a door answered without an error (a cancel of a room that is not there): gated,
    /// then counted.
    pub fn wrong(&self, req: &Req, account: &str) -> Result<(), ApiError> {
        self.gate(req, account, false)?;
        self.miss(req, account);
        Ok(())
    }

    fn miss(&self, req: &Req, account: &str) {
        // Malformed: refused unread, counts nothing.
        let Some(code) = code_of(req) else { return };
        let now = now_ms();
        for (allowance, key) in [
            (&self.per_account, account_key(account)),
            (&self.per_address, address_key(&req.address)),
        ] {
            // Once per (who, code) per window.
            if self.counted.allow(&format!("{key}|{code}"), now) {
                allowance.allow(&key, now);
            }
        }
    }
}
