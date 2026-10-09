//! Invite codes and SPEC §9.4's six-step redemption transaction.
//!
//! All six steps are `Tx::redeem`, one store call (`select app.redeem_invite_code(...)`, migration
//! 0001 §6, in Postgres; the same steps in SPEC's order in memory). Its ordering consequences are
//! stated in `db/store.rs`, `redeem`. §9.8's "Invite code brute force" row names the same
//! mitigations (9.4).
//!
//! Still this file's: R107's response floor ("identical time" is padding, and SQL cannot pad);
//! R106's circuit breaker (alerts, backs `GET /api/codes/status`, disables redemption before the
//! store is touched; `Tx::redeem` may also answer `CircuitOpen`, the same 503); R145's distinct
//! refusals (the store answers `NotPending` for "no such profile", "banned" and "already active";
//! they are decided here from the router's profile). Every constant comes from `crate::config`.

use std::sync::{Arc, Mutex};

use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha2::Sha256;

use crate::api::crypto::{
    canonical_invite_code, format_code, is_well_formed_code, normalize_code, random_code,
};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json, lock, rate_limited};
use crate::app::App;
use crate::config::{
    CODE_ATTEMPT_WINDOW_SECONDS, CODE_ATTEMPTS_PER_PROFILE_PER_HOUR, INVITE_CODE_LENGTH,
    REDEMPTION_CIRCUIT_FAILURE_THRESHOLD, REDEMPTION_CIRCUIT_WINDOW_SECONDS, REDEMPTION_IDENTICAL_ERROR,
    REDEMPTION_RESPONSE_FLOOR_MS,
};
use crate::db::store::{Db, InviteCode, Profile, ProfileStatus, RedeemInviteCodeInput, RedeemResult};

// Wording and defaults SPEC does not pin down

// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
//   Topic: How many accounts one invite code activates
//   Ruling: An invite code is single-use unless its mint says otherwise. §9.4 gives a code a
//     `uses` counter and an "exhausted" state but no default; a multi-use default would turn one
//     leaked code into an open door, the failure §9.8's brute-force row is about. A larger
//     `max_uses` stays available to whoever mints deliberately.
//   Affects: §9.4, §9.8, R106, R107; migration `0001_profiles_and_invites.sql`.
//
// Matches `invite_codes.max_uses int not null default 1` in migration 0001.
pub const DEFAULT_INVITE_CODE_MAX_USES: i32 = 1;

// R145 decides which refusals may be distinct: the identical error covers "every outcome that
// depends on the **code**", while outcomes that depend only on the caller's own account are
// "reported distinctly, because they leak nothing about the code space". The rate-limit and
// breaker refusals are the same kind.
//
// Not in SPEC: the strings themselves; a client branches on the `ApiError` code, never the
// sentence. Only `REDEMPTION_IDENTICAL_ERROR` (from crate::config) is spec-mandated.
const RATE_LIMITED_MESSAGE: &str = "Too many invite code attempts. Try again later.";
const BREAKER_MESSAGE: &str = "Invite redemption is temporarily unavailable. Try again later.";
const ALREADY_ACTIVE_MESSAGE: &str = "This account is already active.";
// Mirrors `assert_active`'s wording in http.rs so a banned account reads the same sentence
// wherever it is turned away.
const BANNED_MESSAGE: &str = "this account is banned";
const EMAIL_UNVERIFIED_MESSAGE: &str = "Verify your email address before redeeming an invite code.";

/// Unit conversion, not configuration.
const MS_PER_SECOND: i64 = 1000;

/// The redemption half of the API limits, straight from config.
struct Limits {
    redeem_per_profile_per_hour: i64,
    redeem_window_ms: i64,
    redeem_constant_ms: u64,
    breaker_failure_threshold: i64,
    breaker_window_ms: i64,
    breaker_cooldown_ms: i64,
}

fn limits() -> Limits {
    Limits {
        redeem_per_profile_per_hour: CODE_ATTEMPTS_PER_PROFILE_PER_HOUR,
        redeem_window_ms: CODE_ATTEMPT_WINDOW_SECONDS * MS_PER_SECOND,
        redeem_constant_ms: REDEMPTION_RESPONSE_FLOOR_MS as u64,
        breaker_failure_threshold: REDEMPTION_CIRCUIT_FAILURE_THRESHOLD,
        breaker_window_ms: REDEMPTION_CIRCUIT_WINDOW_SECONDS * MS_PER_SECOND,
        breaker_cooldown_ms: REDEMPTION_CIRCUIT_WINDOW_SECONDS * MS_PER_SECOND,
    }
}

/// §9.4: "Missing, expired and exhausted codes return an identical error in identical time."
///
/// One constructor, no `details`, so all serialise to the same bytes. A revoked or malformed code
/// takes this path too (§9.8's brute-force row). The reason stays in `code_attempts.reason`,
/// which §9.4 says is "never returned to the client".
fn identical_code_error() -> ApiError {
    ApiError::new(ApiErrorCode::InvalidCode, REDEMPTION_IDENTICAL_ERROR)
}

/// §9.4: codes are stored hashed — a keyed SHA-256 (HMAC) of the normalized code under the
/// `${CODE_PEPPER}:code` pepper, hex. Separate code and IP domains keep their hashes apart.
fn code_hash(code_pepper: &str, plain: &str) -> String {
    let key = format!("{code_pepper}:code");
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key.as_bytes())
        .unwrap_or_else(|error| panic!("an HMAC key of any length: {error}"));
    mac.update(normalize_code(plain).as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// Minting

/// The two things minting touches, rather than the whole `App`, so `cli/mint_code.rs` needs only
/// a store and a pepper (bring-up checklist step 7).
#[derive(Clone, Copy)]
pub struct MintDeps<'a> {
    pub db: &'a Db,
    /// `env.code_pepper`.
    pub code_pepper: &'a str,
}

/// Input to minting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MintInput {
    /// `DEFAULT_INVITE_CODE_MAX_USES` when absent.
    pub max_uses: Option<i32>,
    /// Epoch ms, or `None` for "never expires".
    pub expires_at: Option<i64>,
}

/// A freshly minted code: its row id and its plaintext, formatted, returned exactly once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MintedCode {
    pub id: String,
    pub formatted: String,
}

/// Minting's errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MintError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Store(String),
}

/// Mints one code: `INVITE_CODE_LENGTH` characters (§9.4's 80 bits over the 32-symbol alphabet),
/// formatted `XXXX-XXXX-XXXX-XXXX` for display, stored **hashed** and never in plaintext. The
/// plaintext is returned exactly once, to whoever asked for it; nothing persists it.
pub async fn mint_invite_code(deps: MintDeps<'_>, input: MintInput) -> Result<MintedCode, MintError> {
    let max_uses = input.max_uses.unwrap_or(DEFAULT_INVITE_CODE_MAX_USES);
    if max_uses < 1 {
        return Err(MintError::Invalid(format!(
            "maxUses must be a positive integer (got {max_uses})"
        )));
    }

    let plain = random_code(INVITE_CODE_LENGTH as _);
    // A code outside `CODE_ALPHABET` could never be redeemed (redemption rejects it as malformed),
    // so fail loudly here rather than handing ops a dead code.
    if !is_well_formed_code(&normalize_code(&plain), INVITE_CODE_LENGTH as _) {
        return Err(MintError::Invalid(format!(
            "ids.code() produced a code outside CODE_ALPHABET; {INVITE_CODE_LENGTH} symbols from §9.4's alphabet are required"
        )));
    }

    let code = InviteCode {
        id: uuid::Uuid::new_v4().to_string(),
        code_hash: code_hash(deps.code_pepper, &plain),
        max_uses: i64::from(max_uses),
        uses: 0,
        revoked: false,
        expires_at: input.expires_at,
        created_at: crate::app::now_ms(),
    };
    let mut tx = deps
        .db
        .begin(None)
        .await
        .map_err(|error| MintError::Store(format!("{error:?}")))?;
    tx.codes_insert(&code)
        .await
        .map_err(|error| MintError::Store(format!("{error:?}")))?;
    tx.commit()
        .await
        .map_err(|error| MintError::Store(format!("{error:?}")))?;
    Ok(MintedCode {
        id: code.id,
        formatted: format_code(&plain),
    })
}

// The circuit breaker (§9.4)

/// §9.4: "A global circuit breaker disables redemption and alerts when system-wide failures cross
/// a threshold in a window."
///
/// A value, not module state: `App` holds one (`app.breaker`), so each test gets a fresh one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BreakerState {
    /// Epoch ms until which redemption is disabled; 0 while closed.
    pub open_until: i64,
    /// How many times this breaker has opened. One alert per opening.
    pub openings: u32,
}

pub fn create_breaker_state() -> BreakerState {
    BreakerState {
        open_until: 0,
        openings: 0,
    }
}

// Redemption

/// Success, or a refusal as a value so it can be padded; only a store fault is raised (the outer
/// `Err` of `redeem_code`).
#[derive(Debug)]
pub enum RedeemOutcome {
    Ok,
    Refused(ApiError),
}

/// Input to `redeem_code`.
#[derive(Debug)]
pub struct RedeemInput<'a> {
    /// The caller's profile as the router resolved it for this request. Read here only for R145's
    /// three distinct account refusals; the authority on "is this account still pending" is
    /// `Tx::redeem`, which re-reads it under a row lock inside the transaction.
    pub profile: &'a Profile,
    /// Whatever the client typed; normalized and hashed here, never stored.
    pub plain_code: &'a str,
    pub ip_hash: &'a str,
    /// §9.4 step 1's "verified email", from the caller's `AuthUser.email_verified`, which `auth.rs`
    /// takes from the auth server's `email_confirmed_at`, never from a user-editable claim.
    ///
    /// Not in SPEC: `Profile` carries no verification flag and the authority is the auth provider.
    /// How long a positive answer may be remembered is the proposed ruling in `auth.rs`.
    pub email_verified: bool,
    /// Omitted by a direct caller, in which case this redemption gets a breaker of its own.
    pub breaker: Option<&'a Mutex<BreakerState>>,
}

pub async fn redeem_code(app: &App, input: RedeemInput<'_>) -> Result<RedeemOutcome, ApiError> {
    let own_breaker = Mutex::new(create_breaker_state());
    let breaker = input.breaker.unwrap_or(&own_breaker);
    let limits = limits();
    let now = crate::app::now_ms();

    // R106's breaker, checked before step 1: while it is open nothing touches the profile, the
    // attempt log or the code table.
    if now < lock(breaker).open_until {
        return Ok(RedeemOutcome::Refused(ApiError::new(
            ApiErrorCode::Unavailable,
            BREAKER_MESSAGE,
        )));
    }

    // R145's three account-shaped refusals, which `Tx::redeem` answers as one `NotPending` and §9.4
    // requires distinguishable, decided from the router's profile. The store re-checks under its
    // lock; a change in between is caught as `NotPending`, below.
    let profile = input.profile;
    if matches!(profile.status, ProfileStatus::Banned) {
        return Ok(RedeemOutcome::Refused(ApiError::new(
            ApiErrorCode::AccountBanned,
            BANNED_MESSAGE,
        )));
    }
    if matches!(profile.status, ProfileStatus::Active) {
        // Not a code failure: redemption is only the pending → active transition (§9.4), so an active
        // account asking again is a conflict.
        return Ok(RedeemOutcome::Refused(ApiError::new(
            ApiErrorCode::Conflict,
            ALREADY_ACTIVE_MESSAGE,
        )));
    }
    // §9.4 step 1's "verified email", from the access token (R159). The store may still answer
    // `EmailUnverified`; refusing first keeps an unverified caller out of the attempt log.
    if !input.email_verified {
        return Ok(RedeemOutcome::Refused(ApiError::new(
            ApiErrorCode::EmailUnverified,
            EMAIL_UNVERIFIED_MESSAGE,
        )));
    }

    // §9.4: codes are "stored hashed", so the plaintext is hashed here. R191 fixes the reading
    // (shared with the client's code field): NFKC, upper case, separators removed, then exactly the
    // code's length in R104's alphabet, nothing dropped or mapped; input over `CODE_INPUT_MAX_LENGTH`
    // is not read. A code that could never exist travels as `None`, not refused early: §9.4 logs the
    // attempt (step 4) before the lookup (step 5), so a malformed code must cost the same row.
    let code_hash =
        canonical_invite_code(input.plain_code).map(|canonical| code_hash(&app.env.code_pepper, &canonical));

    // §9.4: "Redemption is one server-side transaction." This is it.
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let result = tx
        .redeem(&RedeemInviteCodeInput {
            profile_id: profile.id.clone(),
            code_hash,
            ip_hash: input.ip_hash.to_string(),
        })
        .await?;
    tx.commit().await?;
    let failed = !matches!(result, RedeemResult::Ok);

    if matches!(result, RedeemResult::CircuitOpen) {
        // The database's own switch (`app.settings.redemption_enabled`) is off. Mirrored in the
        // process's breaker for its cooldown, so `GET /api/codes/status` says "paused" (R192) and further
        // presses are refused before the store, each of which would log an attempt and spend one of the
        // account's tries. No alert: an operator flipped the switch.
        let mut state = lock(breaker);
        state.open_until = state.open_until.max(now + limits.breaker_cooldown_ms);
    }

    let outcome = outcome_for(result, &limits);
    if failed {
        note_failure(app, breaker, now).await?;
    }
    Ok(outcome)
}

/// `Tx::redeem`'s result as the response §9.4 owes. Every code-dependent result collapses onto
/// R145's one identical error; everything else depends only on the caller's own account or on the
/// service's availability, and says so.
fn outcome_for(result: RedeemResult, limits: &Limits) -> RedeemOutcome {
    match result {
        RedeemResult::Ok => RedeemOutcome::Ok,
        // The store re-read the profile under its lock and found it no longer pending: a concurrent
        // redemption, a ban, or the row gone. It cannot say which (`app.redeem_invite_code` answers
        // `not_pending` for all three), so R170 fixes one answer, the conflict: nothing about the
        // caller's authorization failed, the state changed.
        RedeemResult::NotPending => {
            RedeemOutcome::Refused(ApiError::new(ApiErrorCode::Conflict, ALREADY_ACTIVE_MESSAGE))
        }
        RedeemResult::EmailUnverified => RedeemOutcome::Refused(ApiError::new(
            ApiErrorCode::EmailUnverified,
            EMAIL_UNVERIFIED_MESSAGE,
        )),
        // §9.4 steps 2 and 3. Which window refused is not told apart: the per-IP one would say something
        // about other accounts behind the address.
        //
        // R192: a rate limit with its wait, never folded into R145's identical error. The wait is the
        // whole attempt window, an upper bound.
        RedeemResult::RateLimitedProfile | RedeemResult::RateLimitedIp => {
            RedeemOutcome::Refused(rate_limited(RATE_LIMITED_MESSAGE, limits.redeem_window_ms))
        }
        RedeemResult::CircuitOpen => {
            RedeemOutcome::Refused(ApiError::new(ApiErrorCode::Unavailable, BREAKER_MESSAGE))
        }
        RedeemResult::InvalidCode => RedeemOutcome::Refused(identical_code_error()),
    }
}

/// §9.4: the breaker "disables redemption and alerts when system-wide failures cross a threshold
/// in a window". Counted after the transaction commits, so the failure just logged is included.
async fn note_failure(app: &App, breaker: &Mutex<BreakerState>, now: i64) -> Result<(), ApiError> {
    let limits = limits();
    let mut tx = app.db.begin(None).await?;
    let failures = tx.codes_count_failures(now - limits.breaker_window_ms).await?;
    tx.commit().await?;
    if (failures as i64) < limits.breaker_failure_threshold {
        return Ok(());
    }
    let mut state = lock(breaker);
    // Already open: never alert twice for one opening.
    if now < state.open_until {
        return Ok(());
    }

    state.open_until = now + limits.breaker_cooldown_ms;
    state.openings += 1;
    tracing::error!(
        event = "codes.breaker_open",
        failures = failures as i64,
        threshold = limits.breaker_failure_threshold,
        "windowMs" = limits.breaker_window_ms,
        "openUntil" = state.open_until,
    );
    Ok(())
}

// Routes

/// §9.4: "Missing, expired and exhausted codes return an identical error in identical time."
/// Every redemption response is padded to the same floor measured from `started_at`, on the tokio
/// clock, so a test with `tokio::time::pause()` measures it without waiting.
async fn pad_to(started_at: tokio::time::Instant, floor_ms: u64) {
    tokio::time::sleep_until(started_at + std::time::Duration::from_millis(floor_ms)).await;
}

/// The code as sent. Any string is read, the empty one included: R191's reading calls it
/// malformed and it gets R145's identical error and its attempt row, after the account's own
/// checks. Only a non-string is a malformed request.
fn plain_code_of(body: &Value) -> Result<String, ApiError> {
    match body.get("code") {
        Some(Value::String(value)) => Ok(value.clone()),
        _ => Err(ApiError::new(
            ApiErrorCode::BadRequest,
            "\"code\" must be a string",
        )),
    }
}

/// Never answers a refusal as an `Err`: every rejection comes back as a value, so it can be padded.
async fn redeem_for_request(
    app: &App,
    req: &Req,
    breaker: &Mutex<BreakerState>,
) -> Result<RedeemOutcome, ApiError> {
    // `AuthLevel::User` guarantees the caller; the guard is here because `Req` types it optional.
    let Some(caller) = req.caller.as_ref() else {
        return Ok(RedeemOutcome::Refused(ApiError::new(
            ApiErrorCode::Unauthorized,
            "sign in first",
        )));
    };
    let plain_code = match plain_code_of(&req.body) {
        Ok(plain_code) => plain_code,
        Err(error) => return Ok(RedeemOutcome::Refused(error)),
    };
    // A store fault is not a code oracle (it does not depend on which code was sent), so it is
    // left to the router: 500, logged, unpadded.
    redeem_code(
        app,
        RedeemInput {
            profile: &caller.profile,
            plain_code: &plain_code,
            ip_hash: &req.address,
            email_verified: caller.user.email_verified,
            breaker: Some(breaker),
        },
    )
    .await
}

/// `POST /api/codes/redeem` (`AuthLevel::User`, not `Active`: a pending account is exactly the
/// caller this endpoint is for — §9.4: "Redeeming an invite code flips pending to active").
///
/// §9.4's breaker lives in `app.breaker`, so each test can drive it from a known state.
pub async fn redeem(app: &Arc<App>, req: Req) -> ApiResult {
    // Measured from the handler's first line: §9.4's "identical time" is about the response the
    // client sees, not about the lookup alone.
    let started_at = tokio::time::Instant::now();
    let outcome = redeem_for_request(app, &req, &app.breaker).await?;
    // Every branch is padded to the same floor — the successes too, and the breaker's 503 —
    // so no timing difference survives to distinguish missing from expired from exhausted.
    pad_to(started_at, limits().redeem_constant_ms).await;
    match outcome {
        RedeemOutcome::Ok => Ok(json(200, json!({ "status": "active", "needsInviteCode": false }))),
        RedeemOutcome::Refused(error) => Err(error),
    }
}

/// `GET /api/codes/status` (`AuthLevel::User`).
///
/// Lets the code screen say "redemption is paused" instead of making the player guess after a 503.
///
/// `attemptsRemaining` (R192): tries left in §9.4 step 2's window, from this profile's attempts
/// only (other accounts behind the address would leak). The `+ 1` is config.rs's reading of step
/// 2's strict "more than": attempt `redeemPerProfilePerHour + 2` is the first refused. Advisory:
/// the per-IP window can refuse sooner, as a 429 with its wait.
///
/// `attemptsRetryAfterMs` (R192): until the oldest counted attempt leaves the window; 0 while the
/// account has tries. Like the count, it is this profile's own.
pub async fn get_status(app: &Arc<App>, req: Req) -> ApiResult {
    let limits = limits();
    let now = crate::app::now_ms();
    let open_until = lock(&app.breaker).open_until;
    let open = now < open_until;
    let since = now - limits.redeem_window_ms;
    let profile_id = req.caller.as_ref().map(|caller| caller.profile.id.clone());

    let mut attempts: i64 = 0;
    let mut attempts_retry_after_ms: i64 = 0;
    if let Some(profile_id) = profile_id.as_deref() {
        let mut tx = app.db.begin(Some(profile_id)).await?;
        attempts = tx.codes_count_attempts_by_profile(profile_id, since).await? as i64;
        let remaining = (limits.redeem_per_profile_per_hour + 1 - attempts).max(0);
        if remaining == 0 {
            let oldest = tx.codes_oldest_attempt_at_by_profile(profile_id, since).await?;
            attempts_retry_after_ms = match oldest {
                None => 0,
                Some(oldest) => (oldest + limits.redeem_window_ms - now).max(0),
            };
        }
        tx.commit().await?;
    }
    let attempts_remaining = (limits.redeem_per_profile_per_hour + 1 - attempts).max(0);

    Ok(json(
        200,
        json!({
            "redemptionEnabled": !open,
            "retryAfterMs": if open { open_until - now } else { 0 },
            "attemptsRemaining": attempts_remaining,
            "attemptsRetryAfterMs": attempts_retry_after_ms,
        }),
    ))
}
