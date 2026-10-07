//! R192, docs/polish/5-sign-in.md B7, B8, B9 and B13: what the code screen is told, and what it is
//! never told.
//!
//!  - B7: `GET /api/codes/status` says how many tries this account has left in §9.4's window.
//!  - B8: a refusal at §9.4 step 2 or 3 is a 429 `rate_limited` whose wait is the whole attempt
//!    window (`details.retryAfterMs` and `Retry-After`), while every code-dependent refusal keeps
//!    R145's identical bytes and gains no header.
//!  - B9: R109's API-wide limiter says how long until its oldest counted request leaves the window.
//!  - B13: a request body larger than `API_MAX_BODY_BYTES` is refused before it is parsed.
//!
//! Ported from `apps/server/test/api/redeem-feedback.test.ts` (part 18). TS ran redemption on the
//! real clock with `testLimits()`' 20 ms floor and the API limiter on `createManualTimers`; here
//! both run on tokio's paused clock (SURFACE §11.2), where the production 250 ms floor's sleep costs
//! nothing and `tokio::time::advance` drives the limiter. TS's `ApiLimits` port is gone: the limits
//! are `config.rs`'s own numbers, so where TS shrank one (`breakerFailureThreshold: 1`) the test
//! fills the window up to the real threshold instead. TS's `alphabetIds()` is not needed: the
//! server's own ids mint codes inside R104's alphabet.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{HeaderMap, Request};
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::Sha256;
use tower::ServiceExt;

use jackioh_server::api::codes::{MintDeps, MintInput, mint_invite_code};
use jackioh_server::api::http::{create_rate_limiter, error_response, rate_limited};
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{
    API_MAX_BODY_BYTES, API_REQUESTS_PER_MINUTE, CODE_ATTEMPT_WINDOW_SECONDS, CODE_ATTEMPTS_PER_IP_PER_HOUR,
    CODE_ATTEMPTS_PER_PROFILE_PER_HOUR, REDEMPTION_CIRCUIT_FAILURE_THRESHOLD, REDEMPTION_CIRCUIT_WINDOW_SECONDS,
    REDEMPTION_IDENTICAL_ERROR,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CodeAttempt, Db};

use crate::support::deps::{add_user, test_app};

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

const MS_PER_SECOND: i64 = 1000;

/// R109's allowance, as the server carries it.
const LIMIT: usize = API_REQUESTS_PER_MINUTE as usize;

/// A well-formed code inside R104's alphabet that is never minted.
const UNMINTED_CODE: &str = "ABCD-EFGH-JKMN-PQRT";

/// `jsonRequest`'s default `x-forwarded-for`.
const DEFAULT_IP: &str = "203.0.113.7";

/// The stand-ins for TS's own `/api/mine` (user) and `/api/open` (none) routes.
const MINE: &str = "/api/auth/me";
const OPEN: &str = "/api/stats/players";

/// R145's refusal, exactly as it goes over the wire.
fn identical_body() -> String {
    json!({ "error": { "code": "invalid_code", "message": REDEMPTION_IDENTICAL_ERROR } }).to_string()
}

/// §9.4 step 2's allowance (TS `deps.limits.redeemPerProfilePerHour`).
fn per_profile() -> i64 {
    CODE_ATTEMPTS_PER_PROFILE_PER_HOUR as i64
}

/// §9.4 step 3's allowance (TS `deps.limits.redeemPerIpPerHour`).
fn per_ip() -> i64 {
    CODE_ATTEMPTS_PER_IP_PER_HOUR as i64
}

/// The attempt window (TS `deps.limits.redeemWindowMs`).
fn window_ms() -> i64 {
    CODE_ATTEMPT_WINDOW_SECONDS as i64 * MS_PER_SECOND
}

/// The breaker's cooldown (TS `deps.limits.breakerCooldownMs`, `defaultLimits()`'s value).
fn breaker_cooldown_ms() -> i64 {
    REDEMPTION_CIRCUIT_WINDOW_SECONDS as i64 * MS_PER_SECOND
}

fn retry_after_header_for(ms: i64) -> String {
    ((ms + MS_PER_SECOND - 1) / MS_PER_SECOND).to_string()
}

// ---------------------------------------------------------------------------------------------
// Harness (a private copy per file, SURFACE rule 5)
// ---------------------------------------------------------------------------------------------

fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One response, read whole.
struct Reply {
    status: u16,
    headers: HeaderMap,
    text: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or(Value::Null)
    }

    fn retry_after(&self) -> Option<&str> {
        self.headers.get("retry-after").map(|value| value.to_str().expect("an ASCII header"))
    }
}

/// One request through the real router, its body sent exactly as given.
async fn send(app: &Arc<App>, method: &str, path: &str, token: Option<&str>, ip: &str, body: Option<String>) -> Reply {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-forwarded-for", ip);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = body.map_or_else(Body::empty, Body::from);
    let response = app::router(app.clone()).oneshot(request.body(body).expect("request")).await.expect("answers");
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
    Reply { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
}

async fn read_response(response: axum::response::Response) -> Reply {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
    Reply { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
}

/// The fake store behind the test app (TS `deps.store`).
async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

/// §9.4's address hash as the server takes it (TS `createHashes(...).ip`, peppered as `index.ts`
/// peppers it, `${CODE_PEPPER}:ip`): HMAC-SHA256 of the trimmed, lower-cased address, in hex.
struct Hashes {
    pepper: String,
}

impl Hashes {
    fn ip(&self, raw: &str) -> String {
        let mut mac =
            <Hmac<Sha256> as KeyInit>::new_from_slice(format!("{}:ip", self.pepper).as_bytes()).expect("any key length");
        mac.update(raw.trim().to_lowercase().as_bytes());
        mac.finalize().into_bytes().iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

fn hashes(app: &App) -> Hashes {
    Hashes { pepper: app.env.code_pepper.clone() }
}

/// TS `mintInviteCode(deps, { maxUses, expiresAt })`: the new code's id and its formatted text.
async fn mint(app: &App, max_uses: Option<i32>, expires_at: Option<i64>) -> (String, String) {
    let minted = mint_invite_code(MintDeps { db: &app.db, code_pepper: &app.env.code_pepper }, MintInput { max_uses, expires_at })
        .await
        .expect("mintInviteCode");
    (minted.id, minted.formatted)
}

/// TS `codeDeps()`: the test app, whose redemption limits are `config.rs`'s.
async fn code_app() -> Arc<App> {
    test_app().await
}

struct Caller {
    token: String,
    profile_id: String,
}

/// A pending account with a verified email: the caller §9.4's redemption is for.
async fn seed_caller(app: &App, id: &str) -> Caller {
    let user_id = format!("user-{id}");
    let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
    fake(app).await.seed_profile(json!({ "id": id, "userId": user_id, "status": "pending" }));
    Caller { token, profile_id: id.to_string() }
}

async fn redeem(app: &Arc<App>, token: &str, code: &str, ip: &str) -> Reply {
    send(app, "POST", "/api/codes/redeem", Some(token), ip, Some(json!({ "code": code }).to_string())).await
}

async fn code_status(app: &Arc<App>, token: &str) -> Value {
    let reply = send(app, "GET", "/api/codes/status", Some(token), DEFAULT_IP, None).await;
    assert_eq!(reply.status, 200);
    reply.json()
}

fn remaining(status: &Value) -> i64 {
    status["attemptsRemaining"].as_i64().expect("attemptsRemaining")
}

/// TS `logAttempts`: `count` rejected attempts, as §9.4 step 4 writes them.
async fn log_attempts(app: &App, count: i64, profile_id: Option<&str>, ip_hash: &str, at: i64) {
    let attempt: CodeAttempt =
        from(json!({ "profileId": profile_id, "ipHash": ip_hash, "result": "rejected", "reason": "missing", "at": at }));
    for _ in 0..count {
        let mut t = app.db.begin(None).await.expect("begin");
        t.codes_log_attempt(&attempt).await.expect("codes.logAttempt");
        t.commit().await.expect("commit");
    }
}

async fn attempts_logged(app: &App) -> usize {
    fake(app).await.tables.attempts.len()
}

// ---------------------------------------------------------------------------------------------
// B7: tries left
// ---------------------------------------------------------------------------------------------

mod r192_b7_get_api_codes_status_reports_the_tries_left_in_the_window {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r192_b7_gives_a_fresh_pending_account_redeem_per_profile_per_hour_plus_1_tries() {
        let app = code_app().await;
        let caller = seed_caller(&app, "fresh").await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(status["redemptionEnabled"], json!(true));
        assert!(status["retryAfterMs"].is_number());
        assert_eq!(remaining(&status), per_profile() + 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_takes_one_try_off_for_each_attempt_this_profile_made_in_the_window() {
        let app = code_app().await;
        let caller = seed_caller(&app, "counted").await;
        let made = 2;
        log_attempts(&app, made, Some(&caller.profile_id), &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(remaining(&status), per_profile() + 1 - made);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_never_reports_fewer_than_zero_tries() {
        let app = code_app().await;
        let caller = seed_caller(&app, "overdrawn").await;
        log_attempts(&app, per_profile() + 3, Some(&caller.profile_id), &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(remaining(&status), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_ignores_another_profile_s_attempts_from_the_same_address() {
        let app = code_app().await;
        let caller = seed_caller(&app, "bystander").await;
        let neighbour = seed_caller(&app, "neighbour").await;
        log_attempts(&app, per_profile(), Some(&neighbour.profile_id), &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(remaining(&status), per_profile() + 1);
        // And the neighbour's own count is theirs.
        assert_eq!(remaining(&code_status(&app, &neighbour.token).await), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_does_not_count_attempts_older_than_the_window() {
        let app = code_app().await;
        let caller = seed_caller(&app, "yesterday").await;
        let at = now_ms() - window_ms() - 1;
        log_attempts(&app, per_profile() + 1, Some(&caller.profile_id), &hashes(&app).ip(DEFAULT_IP), at).await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(remaining(&status), per_profile() + 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_drops_by_exactly_one_after_a_refused_redemption() {
        let app = code_app().await;
        let caller = seed_caller(&app, "one-miss").await;

        let before = remaining(&code_status(&app, &caller.token).await);
        let refused = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
        assert_eq!(refused.status, 400);
        let after = remaining(&code_status(&app, &caller.token).await);

        assert_eq!(after, before - 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_says_when_an_account_with_no_tries_left_gets_one_back_its_oldest_attempt_leaving_the_window() {
        let app = code_app().await;
        let caller = seed_caller(&app, "waiting").await;
        let ip_hash = hashes(&app).ip(DEFAULT_IP);
        let now = now_ms();
        let age = window_ms() / 4;
        log_attempts(&app, 1, Some(&caller.profile_id), &ip_hash, now - age).await;
        log_attempts(&app, per_profile(), Some(&caller.profile_id), &ip_hash, now).await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(remaining(&status), 0);
        let expected = window_ms() - age;
        let wait = status["attemptsRetryAfterMs"].as_i64().expect("attemptsRetryAfterMs");
        assert!(wait <= expected, "{wait} > {expected}");
        // The request itself may take a moment on the server's clock.
        assert!(wait > expected - 5_000, "{wait} <= {expected} - 5000");
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_states_no_wait_while_the_account_has_tries_left() {
        let app = code_app().await;
        let caller = seed_caller(&app, "not-waiting").await;
        log_attempts(&app, per_profile(), Some(&caller.profile_id), &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let status = code_status(&app, &caller.token).await;

        assert_eq!(remaining(&status), 1);
        assert_eq!(status["attemptsRetryAfterMs"], json!(0));
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b7_agrees_with_9_4_step_2_one_try_left_is_let_through_none_left_is_refused() {
        let app = code_app().await;
        let last = seed_caller(&app, "last-try").await;
        let none = seed_caller(&app, "no-tries").await;
        let at = now_ms();
        let ip_hash = hashes(&app).ip(DEFAULT_IP);
        log_attempts(&app, per_profile(), Some(&last.profile_id), &ip_hash, at).await;
        log_attempts(&app, per_profile() + 1, Some(&none.profile_id), &ip_hash, at).await;

        assert_eq!(remaining(&code_status(&app, &last.token).await), 1);
        assert_eq!(remaining(&code_status(&app, &none.token).await), 0);

        // One left: the attempt reaches the lookup (step 5) and fails there.
        let last_try = redeem(&app, &last.token, UNMINTED_CODE, "198.51.100.71").await;
        assert_eq!(last_try.status, 400);
        assert_eq!(last_try.json()["error"]["code"], json!("invalid_code"));
        // None left: step 2 refuses it as a rate limit.
        let no_try = redeem(&app, &none.token, UNMINTED_CODE, "198.51.100.72").await;
        assert_eq!(no_try.status, 429);
        assert_eq!(no_try.json()["error"]["code"], json!("rate_limited"));
    }
}

// ---------------------------------------------------------------------------------------------
// B8: a rate limit is reported as a rate limit; R145's identical error stays identical
// ---------------------------------------------------------------------------------------------

mod r192_b8_refusals_at_9_4_steps_2_and_3 {
    use super::*;

    /// `count` rejected attempts from as many other accounts behind one address.
    async fn flood_address(app: &App, count: i64, prefix: &str, ip_hash: &str, at: i64) {
        for i in 0..count {
            log_attempts(app, 1, Some(&format!("{prefix}-{i}")), ip_hash, at).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b8_step_2_answers_429_rate_limited_with_the_whole_attempt_window_as_its_wait() {
        let app = code_app().await;
        let caller = seed_caller(&app, "step-two").await;
        log_attempts(&app, per_profile() + 1, Some(&caller.profile_id), &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let response = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.retry_after(), Some(retry_after_header_for(window_ms()).as_str()));
        let body = response.json();
        assert_eq!(body["error"]["code"], json!("rate_limited"));
        assert_eq!(body["error"]["details"], json!({ "retryAfterMs": window_ms() }));
        // R145 is not borrowed: a rate limit is about the caller, never about the code.
        let message = body["error"]["message"].as_str().unwrap_or("");
        assert_ne!(message, REDEMPTION_IDENTICAL_ERROR);
        assert!(!message.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b8_step_3_answers_the_same_way_for_a_flooded_address() {
        let app = code_app().await;
        let caller = seed_caller(&app, "step-three").await;
        flood_address(&app, per_ip() + 1, "neighbour", &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let response = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.retry_after(), Some(retry_after_header_for(window_ms()).as_str()));
        let body = response.json();
        assert_eq!(body["error"]["code"], json!("rate_limited"));
        assert_eq!(body["error"]["details"], json!({ "retryAfterMs": window_ms() }));
        assert_ne!(body["error"]["message"], json!(REDEMPTION_IDENTICAL_ERROR));
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b8_a_refusal_at_step_2_costs_no_try_nothing_is_logged_and_the_count_stays_at_zero() {
        let app = code_app().await;
        let caller = seed_caller(&app, "pinned-at-zero").await;
        let seeded = per_profile() + 1;
        log_attempts(&app, seeded, Some(&caller.profile_id), &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        let first = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
        let second = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!([first.status, second.status], [429, 429]);
        assert_eq!(attempts_logged(&app).await as i64, seeded);
        assert_eq!(remaining(&code_status(&app, &caller.token).await), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b8_step_3_can_refuse_a_profile_that_still_has_tries_left_and_says_so_as_a_rate_limit() {
        let app = code_app().await;
        let caller = seed_caller(&app, "shared-network").await;
        flood_address(&app, per_ip() + 1, "stranger", &hashes(&app).ip(DEFAULT_IP), now_ms()).await;

        // Advisory only: the profile's own window is untouched.
        assert_eq!(remaining(&code_status(&app, &caller.token).await), per_profile() + 1);
        let response = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 429);
        let body = response.json();
        assert_eq!(body["error"]["code"], json!("rate_limited"));
        assert_eq!(body["error"]["details"], json!({ "retryAfterMs": window_ms() }));
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b8_keeps_every_code_dependent_refusal_at_r145_s_exact_bytes_with_no_retry_after() {
        let app = code_app().await;
        let consumer = seed_caller(&app, "consumer").await;
        let (_, expired) = mint(&app, None, Some(now_ms() - MS_PER_SECOND)).await;
        let (_, exhausted) = mint(&app, Some(1), None).await;
        let (revoked_id, revoked) = mint(&app, None, None).await;
        {
            let mut data = fake(&app).await;
            let row = data.tables.codes.iter_mut().find(|row| row.id == revoked_id).expect("the revoked code was stored");
            row.revoked = true;
        }
        assert_eq!(redeem(&app, &consumer.token, &exhausted, "198.51.100.80").await.status, 200);

        let kinds = [
            ("missing", UNMINTED_CODE.to_string()),
            ("malformed", "ABCD-EFGH-JKMN-PQR0".to_string()),
            ("expired", expired),
            ("exhausted", exhausted),
            ("revoked", revoked),
        ];

        for (index, (name, code)) in kinds.iter().enumerate() {
            let caller = seed_caller(&app, &format!("asks-{name}")).await;
            let response = redeem(&app, &caller.token, code, &format!("198.51.100.{}", 81 + index)).await;

            assert_eq!(response.status, 400, "{name}");
            assert_eq!(response.text, identical_body(), "{name}");
            assert_eq!(response.retry_after(), None, "{name}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b8_gives_the_breaker_s_503_no_retry_after_only_rate_limited_carries_one() {
        // TS lowered `breakerFailureThreshold` to 1. The threshold is `config.rs`'s here, so the
        // window is filled to one short of it with other accounts' failures from another address,
        // and the caller's own miss is the one that crosses it.
        let app = code_app().await;
        let caller = seed_caller(&app, "trips-breaker").await;
        flood_address(&app, REDEMPTION_CIRCUIT_FAILURE_THRESHOLD as i64 - 1, "earlier", "hash-elsewhere", now_ms())
            .await;

        assert_eq!(redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await.status, 400);
        let blocked = redeem(&app, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(blocked.status, 503);
        assert_eq!(blocked.retry_after(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn r192_a_pause_the_database_answers_is_reported_by_the_status_too_and_the_next_press_costs_no_try() {
        let app = code_app().await;
        // The operator paused redemption in the database (app.settings.redemption_enabled = false).
        fake(&app).await.redemption.enabled = Arc::new(|| false);
        let caller = seed_caller(&app, "paused").await;
        let (_, minted) = mint(&app, None, None).await;

        // Before any redemption this process cannot know about the database's switch.
        assert_eq!(code_status(&app, &caller.token).await["redemptionEnabled"], json!(true));
        assert_eq!(redeem(&app, &caller.token, &minted, DEFAULT_IP).await.status, 503);

        // Now it does: the status says paused, with the breaker's cooldown as the wait…
        let paused = code_status(&app, &caller.token).await;
        assert_eq!(paused["redemptionEnabled"], json!(false));
        let wait = paused["retryAfterMs"].as_i64().expect("retryAfterMs");
        assert!(wait > 0);
        assert!(wait <= breaker_cooldown_ms());

        // …and a second press is refused before the store logs another attempt.
        assert_eq!(redeem(&app, &caller.token, &minted, DEFAULT_IP).await.status, 503);
        assert_eq!(remaining(&code_status(&app, &caller.token).await), remaining(&paused));
    }
}

mod r192_b8_rate_limited_and_error_response {
    use super::*;

    #[tokio::test]
    async fn r192_b8_turns_the_wait_into_a_429_with_details_retry_after_ms_and_whole_second_retry_after() {
        let response = read_response(error_response(rate_limited("slow down", 1_500))).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.retry_after(), Some("2"));
        assert_eq!(
            response.json(),
            json!({ "error": { "code": "rate_limited", "message": "slow down", "details": { "retryAfterMs": 1_500 } } })
        );
    }

    #[test]
    fn r192_b8_rounds_the_header_up_to_whole_seconds() {
        let cases: [(i64, String); 6] = [
            (0, "0".to_string()),
            (1, "1".to_string()),
            (999, "1".to_string()),
            (MS_PER_SECOND, "1".to_string()),
            (MS_PER_SECOND + 1, "2".to_string()),
            (window_ms(), retry_after_header_for(window_ms())),
        ];
        for (ms, header) in cases {
            let response = error_response(rate_limited("wait", ms));
            let found = response.headers().get("retry-after").map(|value| value.to_str().expect("ASCII"));
            assert_eq!(found, Some(header.as_str()), "{ms}");
        }
    }

    #[test]
    fn r192_b8_adds_no_retry_after_for_a_negative_or_non_finite_wait() {
        // TS also tried NaN and Infinity; the wait is an `i64` here (`ApiError::retry_after_ms`), so
        // a negative one is the only wait without a header that the type can hold.
        for ms in [-1_i64] {
            let response = error_response(rate_limited("wait", ms));
            assert_eq!(response.status().as_u16(), 429, "{ms}");
            assert!(response.headers().get("retry-after").is_none(), "{ms}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B9: the API-wide limiter says how long to wait
// ---------------------------------------------------------------------------------------------

mod r192_b9_the_limiter_s_wait {
    use super::*;

    #[test]
    fn r192_b9_is_0_while_a_key_has_room_and_for_a_key_it_has_never_seen() {
        let limiter = create_rate_limiter(2, 1_000);
        assert_eq!(limiter.retry_after_ms("never-seen", 0), 0);
        assert!(limiter.allow("k", 0));
        assert_eq!(limiter.retry_after_ms("k", 50), 0);
    }

    #[test]
    fn r192_b9_is_the_time_until_the_oldest_counted_hit_leaves_the_window() {
        let limiter = create_rate_limiter(2, 1_000);
        assert!(limiter.allow("k", 0));
        assert!(limiter.allow("k", 100));

        assert_eq!(limiter.retry_after_ms("k", 200), 800);
        assert_eq!(limiter.retry_after_ms("k", 999), 1);
    }

    #[test]
    fn r192_b9_is_not_moved_by_refused_hits_which_are_never_counted() {
        let limiter = create_rate_limiter(2, 1_000);
        limiter.allow("k", 0);
        limiter.allow("k", 100);
        assert!(!limiter.allow("k", 200));
        assert!(!limiter.allow("k", 300));

        assert_eq!(limiter.retry_after_ms("k", 400), 600);
    }

    #[test]
    fn r192_b9_moves_on_to_the_next_oldest_hit_once_the_oldest_has_left() {
        let limiter = create_rate_limiter(2, 1_000);
        limiter.allow("k", 0);
        limiter.allow("k", 100);
        assert!(limiter.allow("k", 1_000));

        assert_eq!(limiter.retry_after_ms("k", 1_050), 50);
    }
}

mod r192_b9_the_api_limiter_s_429_r109 {
    use super::*;

    async fn sign_in(app: &App, id: &str) -> String {
        let user_id = format!("user-{id}");
        let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
        fake(app).await.seed_profile(json!({ "id": id, "userId": user_id, "status": "active" }));
        token
    }

    fn wait_of(reply: &Reply) -> i64 {
        reply.json()["error"]["details"]["retryAfterMs"].as_i64().expect("the 429 carries details.retryAfterMs")
    }

    async fn advance(ms: i64) {
        tokio::time::advance(Duration::from_millis(u64::try_from(ms).expect("a wait ahead"))).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b9_carries_details_retry_after_ms_exact_to_the_millisecond_and_a_matching_retry_after() {
        let app = test_app().await;
        let token = sign_in(&app, "b9-account").await;
        let mine = async || send(&app, "GET", MINE, Some(&token), DEFAULT_IP, None).await;
        const GAP_MS: i64 = 1_000;

        // The oldest counted request, then the rest of the allowance one gap later.
        assert_eq!(mine().await.status, 200);
        advance(GAP_MS).await;
        for _ in 1..LIMIT {
            assert_eq!(mine().await.status, 200);
        }
        advance(GAP_MS).await;

        let refused = mine().await;
        assert_eq!(refused.status, 429);
        assert_eq!(refused.json()["error"]["code"], json!("rate_limited"));
        let wait = wait_of(&refused);
        assert!(wait > 0);
        assert_eq!(refused.retry_after(), Some(retry_after_header_for(wait).as_str()));

        // One millisecond early is still refused, and says so.
        advance(wait - 1).await;
        let early = mine().await;
        assert_eq!(early.status, 429);
        assert_eq!(wait_of(&early), 1);

        // On the dot, the oldest request has left the window and one slot is free.
        advance(1).await;
        assert_eq!(mine().await.status, 200);

        // The next wait is for the next-oldest request, which came one gap after the first.
        let next = mine().await;
        assert_eq!(next.status, 429);
        assert_eq!(wait_of(&next), GAP_MS);
        assert_eq!(next.retry_after(), Some(retry_after_header_for(GAP_MS).as_str()));
    }

    #[tokio::test(start_paused = true)]
    async fn r192_b9_carries_the_wait_on_an_anonymous_flood_s_429_too_r157() {
        let app = test_app().await;
        let open = async || send(&app, "GET", OPEN, None, "203.0.113.90", None).await;

        for _ in 0..LIMIT {
            assert_eq!(open().await.status, 200);
        }
        let refused = open().await;

        assert_eq!(refused.status, 429);
        assert_eq!(refused.json()["error"]["code"], json!("rate_limited"));
        let wait = wait_of(&refused);
        assert!(wait > 0);
        assert_eq!(refused.retry_after(), Some(retry_after_header_for(wait).as_str()));
    }
}

// ---------------------------------------------------------------------------------------------
// B13: the body cap
// ---------------------------------------------------------------------------------------------

mod b13_a_request_body_larger_than_api_max_body_bytes {
    use super::*;

    const TOO_LARGE_MESSAGE: &str = "the request body is too large";

    fn cap() -> usize {
        API_MAX_BODY_BYTES as usize
    }

    async fn raw_redeem(app: &Arc<App>, token: &str, body: String, ip: &str) -> Reply {
        send(app, "POST", "/api/codes/redeem", Some(token), ip, Some(body)).await
    }

    /// `{"code":"AAA…"}`, exactly `bytes` long.
    fn code_body_of_bytes(bytes: usize) -> String {
        let shell = json!({ "code": "" }).to_string();
        let body = json!({ "code": "A".repeat(bytes - shell.len()) }).to_string();
        assert_eq!(body.len(), bytes);
        body
    }

    #[tokio::test(start_paused = true)]
    async fn b13_is_refused_with_400_bad_request_and_never_reaches_the_redemption() {
        let app = code_app().await;
        let caller = seed_caller(&app, "big-body").await;

        let response = raw_redeem(&app, &caller.token, code_body_of_bytes(cap() + 1), "198.51.100.91").await;

        assert_eq!(response.status, 400);
        let body = response.json();
        assert_eq!(body["error"]["code"], json!("bad_request"));
        assert_eq!(body["error"]["message"], json!(TOO_LARGE_MESSAGE));
        // Not an attempt: the handler never saw a code.
        assert_eq!(attempts_logged(&app).await, 0);
        let data = fake(&app).await;
        let row = data.tables.profiles.iter().find(|row| row.id == caller.profile_id).expect("the caller's row");
        assert_eq!(json_of(&row.status), json!("pending"));
    }

    #[tokio::test(start_paused = true)]
    async fn b13_is_refused_for_its_size_before_json_parsing_even_when_it_is_not_json() {
        let app = code_app().await;
        let caller = seed_caller(&app, "big-garbage").await;

        let garbage = format!("{{{}", "x".repeat(cap()));
        let response = raw_redeem(&app, &caller.token, garbage, "198.51.100.92").await;

        assert_eq!(response.status, 400);
        let body = response.json();
        assert_eq!(body["error"]["code"], json!("bad_request"));
        assert_eq!(body["error"]["message"], json!(TOO_LARGE_MESSAGE));
    }

    #[tokio::test(start_paused = true)]
    async fn b13_counts_bytes_not_characters() {
        let app = code_app().await;
        let caller = seed_caller(&app, "wide-body").await;

        // Two UTF-8 bytes per "é": under the cap in characters, over it in bytes.
        let body = json!({ "code": "é".repeat(cap().div_ceil(2)) }).to_string();
        assert!(body.encode_utf16().count() < cap());
        assert!(body.len() > cap());
        let response = raw_redeem(&app, &caller.token, body, "198.51.100.93").await;

        assert_eq!(response.status, 400);
        let parsed = response.json();
        assert_eq!(parsed["error"]["code"], json!("bad_request"));
        assert_eq!(parsed["error"]["message"], json!(TOO_LARGE_MESSAGE));
    }

    #[tokio::test(start_paused = true)]
    async fn b13_reads_a_body_of_exactly_api_max_body_bytes() {
        let app = code_app().await;
        let caller = seed_caller(&app, "full-body").await;

        let response = raw_redeem(&app, &caller.token, code_body_of_bytes(cap()), "198.51.100.94").await;

        // Read and handled: the code is far past the input cap, so it is R145's identical error, not
        // a refusal of the body.
        assert_eq!(response.status, 400);
        assert_eq!(response.text, identical_body());
        assert_eq!(attempts_logged(&app).await, 1);
    }
}
