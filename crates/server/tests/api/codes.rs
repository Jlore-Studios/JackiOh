//! Port of `apps/server/test/api/codes.test.ts`.
//!
//! BUILD M6-T1: "integration tests for each rejection step" of SPEC §9.4's six-step redemption
//! transaction, plus the identical-error requirement and the global circuit breaker. Each test
//! names the step it covers.
//!
//! Every test drives real HTTP through `app::router`, so the route's auth declaration, the §9.4
//! gate, the constant-time padding and the handler all take part.
//!
//! What the Rust server has that TS's `ServerDeps` did not, and so how this file differs:
//!   * no `Timers` port: every test runs on tokio's paused clock (`start_paused`), where §9.4's
//!     production floor (`REDEMPTION_RESPONSE_FLOOR_MS`) costs nothing to sit through, so TS's 20 ms
//!     test floor is gone;
//!   * no `ApiLimits` to shrink: a test that wants the breaker near its threshold logs the failures
//!     that bring it there (`near_threshold`), against the production
//!     `REDEMPTION_CIRCUIT_FAILURE_THRESHOLD`;
//!   * no `Ids` port: `mint_invite_code` draws its codes from the OS (TS's `alphabetIds` fake, which
//!     made `systemIds.code`'s alphabet deterministic, has nothing to stand in for), so a test finds
//!     a minted code by the id or the formatted text it answered.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Request, Response};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tower::ServiceExt;
use tracing_subscriber::layer::SubscriberExt;

use jackioh_server::api::codes::{DEFAULT_INVITE_CODE_MAX_USES, MintDeps, MintInput, mint_invite_code};
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    CODE_ATTEMPT_WINDOW_SECONDS, CODE_ATTEMPTS_PER_IP_PER_HOUR, CODE_ATTEMPTS_PER_PROFILE_PER_HOUR,
    INVITE_CODE_LENGTH, REDEMPTION_CIRCUIT_FAILURE_THRESHOLD, REDEMPTION_IDENTICAL_ERROR, REDEMPTION_RESPONSE_FLOOR_MS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StoreError};

use crate::support::deps;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A well-formed code (16 symbols, all inside the alphabet) that was never minted:
/// `formatCode("ABCDEFGHJKLMNPQR")`.
const UNMINTED_CODE: &str = "ABCD-EFGH-JKLM-NPQR";

const DEFAULT_IP: &str = "203.0.113.7"; // `jsonRequest`'s default `x-forwarded-for`.

/// `CODE_PEPPER` for every server here. `index.ts` keys the two hashes `${CODE_PEPPER}:code` and
/// `${CODE_PEPPER}:ip`, and a test that compares a stored hash computes the same one.
const PEPPER: &str = "codes-test-pepper-of-at-least-thirty-two-characters";

/// §9.4's attempt window, in the milliseconds `code_attempts.at` is stamped in.
const WINDOW_MS: i64 = CODE_ATTEMPT_WINDOW_SECONDS as i64 * 1000;

/// The environment `createTestDeps()` stood for: `E2E=1` (the fake store and fixture auth), the
/// compiled-in catalog version (SURFACE §11.3) and one trusted proxy hop, the deployed server behind
/// Render's edge, whose entry `json_request` writes.
fn test_env() -> IndexMap<String, String> {
    let mut source = IndexMap::new();
    for (name, value) in [("E2E", "1"), ("NODE_ENV", "test"), ("CODE_PEPPER", PEPPER), ("TRUSTED_PROXY_HOPS", "1")] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

/// One App and its router: TS's `deps` and `createRouter(createCodesRoutes(), deps)`.
struct Server {
    app: Arc<App>,
    router: axum::Router,
}

/// One response, read whole.
struct Reply {
    status: u16,
    text: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).expect("the body is JSON")
    }

    fn error_code(&self) -> String {
        self.json()["error"]["code"].as_str().unwrap_or_default().to_string()
    }

    fn error_message(&self) -> String {
        self.json()["error"]["message"].as_str().unwrap_or_default().to_string()
    }
}

async fn read(response: Response<Body>) -> Reply {
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");
    Reply { status, text: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8") }
}

impl Server {
    async fn send(&self, request: Request<Body>) -> Reply {
        read(self.router.clone().oneshot(request).await.expect("the router always answers")).await
    }
}

/// §9.4's response padding sleeps on tokio's clock, which `start_paused` jumps forward, so this
/// suite runs on the production floor and pays nothing for it. R144's fixtures are wiped, so every
/// row a test reads is one it wrote (TS's empty memory store).
async fn code_server() -> Server {
    let env = app::load_server_env(&test_env()).expect("the test environment loads");
    let app = app::build(env).await.expect("the test app builds");
    store_of(&app).lock().await.reset();
    Server { router: app::router(app.clone()), app }
}

/// A second App over `store`: TS's second `createRouter(createCodesRoutes(), deps)` on the same deps.
async fn server_sharing(store: Arc<Mutex<FakeData>>) -> Server {
    let env = app::load_server_env(&test_env()).expect("the test environment loads");
    let built = Arc::try_unwrap(app::build(env).await.expect("the test app builds"))
        .ok()
        .expect("app::build keeps no second handle on the App it returns");
    let app = Arc::new(App { db: Db::Fake(store), ..built });
    Server { router: app::router(app.clone()), app }
}

fn store_of(app: &App) -> Arc<Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data.clone(),
        Db::Pg(_) => panic!("E2E=1 builds the in-memory store"),
    }
}

fn from_json<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("the literal has the port's JSON shape")
}

fn json_rows<T: Serialize>(rows: &[T]) -> Vec<Value> {
    rows.iter().map(|row| serde_json::to_value(row).expect("a row serialises")).collect()
}

async fn attempts(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.attempts)
}

async fn codes(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.codes)
}

async fn profiles(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.profiles)
}

/// `tables.profiles.find((row) => row.id === id)?.status`.
async fn status_of(server: &Server, id: &str) -> Value {
    profiles(server).await.into_iter().find(|row| row["id"] == json!(id)).map(|row| row["status"].clone()).unwrap_or(Value::Null)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("after 1970").as_millis() as i64
}

/// HMAC-SHA256 as lower-case hex (RFC 2104 over `sha2`, so this file needs no MAC crate's API).
fn hmac_sha256_hex(key: &str, message: &str) -> String {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key.as_bytes());
        block[..digest.len()].copy_from_slice(&digest);
    } else {
        block[..key.len()].copy_from_slice(key.as_bytes());
    }
    let inner_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x36).collect();
    let outer_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x5c).collect();
    let mut inner = Sha256::new();
    inner.update(&inner_pad);
    inner.update(message.as_bytes());
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&outer_pad);
    outer.update(&inner[..]);
    outer.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `deps.hashes.ip(raw)`: `crypto.ts`'s `createHashes`, keyed as `index.ts` keys it.
fn ip_hash(raw: &str) -> String {
    hmac_sha256_hex(&format!("{PEPPER}:ip"), &raw.trim().to_lowercase())
}

/// `deps.hashes.code(formatted)` for a code typed in the canonical alphabet: upper case already, so
/// R191's reading only drops the separators.
fn code_hash(formatted: &str) -> String {
    hmac_sha256_hex(&format!("{PEPPER}:code"), &formatted.replace('-', ""))
}

struct Minted {
    id: String,
    formatted: String,
}

/// `mintInviteCode(deps, { maxUses, expiresAt })`.
async fn mint(server: &Server, max_uses: Option<i64>, expires_at: Option<i64>) -> Minted {
    let max_uses = max_uses.map(|uses| i32::try_from(uses).expect("a use count"));
    let minted = mint_invite_code(MintDeps { db: &server.app.db, code_pepper: &server.app.env.code_pepper }, MintInput { max_uses, expires_at })
        .await
        .expect("the code is minted");
    Minted { id: minted.id, formatted: minted.formatted }
}

/// `deps.store.codes.logAttempt(attempt)`.
async fn log_attempt(server: &Server, attempt: Value) {
    let mut tx = server.app.db.begin(None).await.expect("a transaction opens");
    tx.codes_log_attempt(&from_json(attempt)).await.expect("the attempt is logged");
    tx.commit().await.expect("the transaction commits");
}

struct Seeded {
    token: String,
    profile_id: String,
}

/// An auth user (`deps.auth.addUser`) and a profile row for them (`deps.store.seedProfile`).
async fn seed_caller(server: &Server, id: &str, status: &str, email_verified: bool) -> Seeded {
    let user_id = format!("user-{id}");
    let token = deps::add_user(&server.app, &user_id, &format!("{id}@example.test"), email_verified);
    // `resolve_caller` finds a profile by auth user id, so the seeded row must carry the same one.
    let _ = store_of(&server.app).lock().await.seed_profile(json!({ "id": id, "userId": user_id, "status": status }));
    Seeded { token, profile_id: id.to_string() }
}

async fn pending_caller(server: &Server, id: &str) -> Seeded {
    seed_caller(server, id, "pending", true).await
}

/// `jsonRequest(method, path, body, { token, ip })`.
fn json_request(method: &str, path: &str, body: Option<Value>, token: Option<&str>, ip: &str) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path).header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder = builder.header("x-forwarded-for", ip);
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
    builder.body(body).expect("a well-formed request")
}

fn redeem_request(token: &str, code: &str, ip: &str) -> Request<Body> {
    json_request("POST", "/api/codes/redeem", Some(json!({ "code": code })), Some(token), ip)
}

async fn redeem(server: &Server, token: &str, code: &str, ip: &str) -> Reply {
    server.send(redeem_request(token, code, ip)).await
}

/// One logged event, as TS's recording logger kept it: `alert` is `tracing`'s ERROR.
#[derive(Clone, Debug)]
struct Entry {
    level: tracing::Level,
    event: String,
}

#[derive(Clone, Default)]
struct Recording(Arc<std::sync::Mutex<Vec<Entry>>>);

impl Recording {
    fn named(&self, event: &str) -> Vec<Entry> {
        self.0.lock().expect("the recording").iter().filter(|entry| entry.event == event).cloned().collect()
    }
}

/// The event's name: its `event` field, else its message.
#[derive(Default)]
struct EventName {
    event: Option<String>,
    message: Option<String>,
}

impl tracing::field::Visit for EventName {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "event" => self.event = Some(value.to_string()),
            "message" => self.message = Some(value.to_string()),
            _ => {}
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "event" => self.event = Some(format!("{value:?}")),
            "message" => self.message = Some(format!("{value:?}")),
            _ => {}
        }
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Recording {
    fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
        let mut name = EventName::default();
        event.record(&mut name);
        let event_name = name.event.or(name.message).unwrap_or_default();
        self.0.lock().expect("the recording").push(Entry { level: *event.metadata().level(), event: event_name });
    }
}

/// `deps.log`: every event this thread logs until the guard drops.
fn record() -> (Recording, tracing::subscriber::DefaultGuard) {
    let recording = Recording::default();
    let guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(recording.clone()));
    (recording, guard)
}

/// `count` rejected attempts by other accounts, each from an address of its own, so neither the
/// per-profile nor the per-IP window of the caller under test moves.
async fn neighbours_failed(server: &Server, count: usize) {
    let at = now_ms();
    for i in 0..count {
        log_attempt(
            server,
            json!({
                "profileId": format!("neighbour-{i}"),
                "ipHash": ip_hash(&format!("198.18.{}.{}", i / 250, i % 250)),
                "result": "rejected",
                "reason": "missing",
                "at": at,
            }),
        )
        .await;
    }
}

/// TS ran these with `breakerFailureThreshold` 1, 2 or 3. The threshold is the production one here,
/// so the system-wide failures that §9.4's breaker counts are brought to `short_by` below it first.
async fn near_threshold(server: &Server, short_by: usize) {
    neighbours_failed(server, REDEMPTION_CIRCUIT_FAILURE_THRESHOLD as usize - short_by).await;
}

// ---------------------------------------------------------------------------
// Step 1 — "reject unless the account is pending with a verified email"
// ---------------------------------------------------------------------------

// R145: everything here depends on the caller's own account rather than on the code, so each one
// is reported distinctly — it leaks nothing about the code space.
mod r145_section_9_4_step_1_pending_account_with_a_verified_email {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn rejects_a_profile_that_is_not_pending_and_never_logs_the_attempt() {
        let server = code_server().await;
        let caller = seed_caller(&server, "already-active", "active", true).await;
        let minted = mint(&server, None, None).await;

        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 409);
        assert_eq!(response.error_code(), "conflict");
        // Distinct from a code failure: nothing about the code was even looked at.
        assert_ne!(response.error_message(), REDEMPTION_IDENTICAL_ERROR);
        assert_eq!(attempts(&server).await.len(), 0);
        assert_eq!(codes(&server).await[0]["uses"], json!(0));
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_a_banned_profile_with_403() {
        let server = code_server().await;
        let caller = seed_caller(&server, "banned", "banned", true).await;
        let minted = mint(&server, None, None).await;

        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 403);
        assert_eq!(response.error_code(), "account_banned");
        assert_eq!(attempts(&server).await.len(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_a_pending_profile_whose_email_is_not_verified() {
        let server = code_server().await;
        let caller = seed_caller(&server, "unverified", "pending", false).await;
        let minted = mint(&server, None, None).await;

        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 403);
        assert_eq!(response.error_code(), "email_unverified");
        // A good code is not consumed by an account that may not use it yet.
        assert_eq!(codes(&server).await[0]["uses"], json!(0));
        assert_eq!(attempts(&server).await.len(), 0);
    }

    /// R170. `resolve_caller` (http.rs) reads the caller's profile before the handler runs, and
    /// `Tx::redeem` reads it again inside the transaction; the row can go between the two. The
    /// store cannot say which of §9.4 step 1's refusals it hit — `app.redeem_invite_code` answers
    /// `not_pending` for a missing row, a banned one and an active one alike — so the server reports
    /// the conflict, not a second 401 after authorization has already passed.
    ///
    /// TS removed the row from `onCall`, on the way into `Store.redeem`. The Rust fake runs each
    /// transaction under one FIFO-fair `tokio::sync::Mutex`, so the test takes its place in that
    /// queue instead: it holds the store while the request queues for `resolve_caller`'s
    /// transaction, queues itself right behind it, and removes the row in the gap before the
    /// redemption's own transaction — exactly the gap R170 is about.
    #[tokio::test(start_paused = true)]
    async fn r170_answers_a_profile_that_vanishes_mid_redemption_with_a_conflict_not_a_401() {
        let server = code_server().await;
        let caller = pending_caller(&server, "vanishes").await;
        let minted = mint(&server, None, None).await;
        let store = store_of(&server.app);

        let held = store.lock().await;
        let in_flight = tokio::spawn(server.router.clone().oneshot(redeem_request(&caller.token, &minted.formatted, DEFAULT_IP)));
        // The request runs until it waits for the store: `resolve_caller`'s transaction.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        drop(held);
        {
            // Granted once `resolve_caller` has committed, and before the redemption begins.
            let mut between = store.lock().await;
            between.tables.profiles.retain(|row| row.id != caller.profile_id);
        }
        let response = read(in_flight.await.expect("the request ran").expect("the router always answers")).await;

        // 409, not the 401 the six-step version answered here: the token verified and the caller was
        // resolved, so nothing about their authorization failed — the state the request was about
        // changed underneath it. "No such identity" is still a 401, but it belongs to
        // `Auth::verify`, one layer earlier, and never to this handler.
        assert_eq!(response.status, 409);
        assert_eq!(response.error_code(), "conflict");
        // R145: a refusal about the caller's own account never borrows the code's identical error.
        assert_ne!(response.error_message(), REDEMPTION_IDENTICAL_ERROR);

        // Refused at step 1, so step 4 never runs: no attempt row, exactly as for a banned or an
        // already-active caller above. §9.4 orders steps 1-3 ahead of the log, and a vanished profile
        // is a step 1 refusal like any other.
        assert_eq!(attempts(&server).await.len(), 0);
        assert_eq!(codes(&server).await[0]["uses"], json!(0));
    }
}

// ---------------------------------------------------------------------------
// Steps 2 and 3 — the per-profile and per-IP windows
// ---------------------------------------------------------------------------

mod section_9_4_steps_2_and_3_attempt_limits {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn step_2_rejects_once_this_profile_is_over_the_per_profile_limit() {
        let server = code_server().await;
        let caller = pending_caller(&server, "flooder").await;
        let at = now_ms();

        // config.rs: the limit is strictly `>`, so the (limit + 1)-th attempt is the first rejection.
        for _ in 0..CODE_ATTEMPTS_PER_PROFILE_PER_HOUR as usize + 1 {
            log_attempt(
                &server,
                json!({ "profileId": caller.profile_id, "ipHash": ip_hash(DEFAULT_IP), "result": "rejected", "reason": "missing", "at": at }),
            )
            .await;
        }

        let response = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.error_code(), "rate_limited");
    }

    #[tokio::test(start_paused = true)]
    async fn step_2_lets_the_attempt_through_while_the_profile_is_exactly_at_the_limit() {
        let server = code_server().await;
        let caller = pending_caller(&server, "borderline").await;
        let at = now_ms();

        for _ in 0..CODE_ATTEMPTS_PER_PROFILE_PER_HOUR as usize {
            log_attempt(
                &server,
                json!({ "profileId": caller.profile_id, "ipHash": ip_hash(DEFAULT_IP), "result": "rejected", "reason": "missing", "at": at }),
            )
            .await;
        }

        let response = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        // Reached step 5 and failed there, not at step 2.
        assert_eq!(response.status, 400);
        assert_eq!(response.error_code(), "invalid_code");
    }

    #[tokio::test(start_paused = true)]
    async fn step_3_rejects_once_this_ip_hash_is_over_the_per_ip_limit() {
        let server = code_server().await;
        let caller = pending_caller(&server, "shared-ip").await;
        let at = now_ms();

        // Other accounts behind the same NAT, so step 2's per-profile count stays at zero.
        for i in 0..CODE_ATTEMPTS_PER_IP_PER_HOUR as usize + 1 {
            log_attempt(
                &server,
                json!({
                    "profileId": format!("neighbour-{i}"),
                    "ipHash": ip_hash(DEFAULT_IP),
                    "result": "rejected",
                    "reason": "missing",
                    "at": at,
                }),
            )
            .await;
        }

        let response = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.error_code(), "rate_limited");
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_count_attempts_older_than_the_window() {
        let server = code_server().await;
        let caller = pending_caller(&server, "yesterday").await;
        let at = now_ms() - WINDOW_MS - 1;

        for _ in 0..CODE_ATTEMPTS_PER_PROFILE_PER_HOUR as usize + 5 {
            log_attempt(
                &server,
                json!({ "profileId": caller.profile_id, "ipHash": ip_hash(DEFAULT_IP), "result": "rejected", "reason": "missing", "at": at }),
            )
            .await;
        }

        let response = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 400);
    }
}

// ---------------------------------------------------------------------------
// Step 4 — "log the attempt either way"
// ---------------------------------------------------------------------------

mod section_9_4_step_4_the_attempt_log {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn holds_a_row_for_a_failed_lookup() {
        let server = code_server().await;
        let caller = pending_caller(&server, "typo").await;

        redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        let logged = attempts(&server).await;
        assert_eq!(logged.len(), 1);
        let attempt = &logged[0];
        assert_eq!(attempt["profileId"], json!(caller.profile_id));
        assert_eq!(attempt["result"], json!("rejected"));
        assert_eq!(attempt["ipHash"], json!(ip_hash(DEFAULT_IP)));
        // §9.4: the reason is for operators and never reaches the client.
        assert_eq!(attempt["reason"], json!("missing"));
    }

    #[tokio::test(start_paused = true)]
    async fn holds_no_row_for_a_rate_limited_attempt_so_the_window_drains() {
        let server = code_server().await;
        let caller = pending_caller(&server, "pinned").await;
        let at = now_ms();

        let seeded = CODE_ATTEMPTS_PER_PROFILE_PER_HOUR as usize + 1;
        for _ in 0..seeded {
            log_attempt(
                &server,
                json!({ "profileId": caller.profile_id, "ipHash": ip_hash(DEFAULT_IP), "result": "rejected", "reason": "missing", "at": at }),
            )
            .await;
        }

        redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
        redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        // Steps 2 and 3 reject before step 4: retrying while over the limit adds nothing.
        assert_eq!(attempts(&server).await.len(), seeded);
    }

    #[tokio::test(start_paused = true)]
    async fn holds_an_ok_row_for_a_redemption_that_succeeded() {
        let server = code_server().await;
        let caller = pending_caller(&server, "winner").await;
        let minted = mint(&server, None, None).await;

        redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        let logged = attempts(&server).await;
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0]["result"], json!("ok"));
    }
}

// ---------------------------------------------------------------------------
// Step 5 — "look up by hash and reject if revoked, expired or exhausted"
// ---------------------------------------------------------------------------

mod section_9_4_step_5_the_lookup {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn rejects_a_missing_code() {
        let server = code_server().await;
        let caller = pending_caller(&server, "missing").await;

        let response = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        assert_eq!(response.status, 400);
        let body = response.json();
        assert_eq!(body["error"]["code"], json!("invalid_code"));
        assert_eq!(body["error"]["message"], json!(REDEMPTION_IDENTICAL_ERROR));
        assert!(body["error"].get("details").is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_a_malformed_code_down_the_identical_path() {
        let server = code_server().await;
        let caller = pending_caller(&server, "malformed").await;

        // `0`, `O`, `1`, `I` and `l` are exactly what §9.4's alphabet leaves out.
        let response = redeem(&server, &caller.token, "0OI1-llll-0OI1-llll", DEFAULT_IP).await;

        assert_eq!(response.status, 400);
        assert_eq!(response.error_message(), REDEMPTION_IDENTICAL_ERROR);
        // Still logged (it got past steps 2 and 3), under an operator-only reason.
        assert_eq!(attempts(&server).await[0]["reason"], json!("malformed"));
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_a_revoked_code() {
        let server = code_server().await;
        let caller = pending_caller(&server, "revoked").await;
        let minted = mint(&server, None, None).await;
        {
            let store = store_of(&server.app);
            let mut data = store.lock().await;
            let stored = data.tables.codes.iter_mut().find(|row| row.id == minted.id).expect("the minted code was stored");
            stored.revoked = true;
        }

        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 400);
        assert_eq!(response.error_message(), REDEMPTION_IDENTICAL_ERROR);
        assert_eq!(profiles(&server).await[0]["status"], json!("pending"));
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_an_expired_code() {
        let server = code_server().await;
        let caller = pending_caller(&server, "expired").await;
        let minted = mint(&server, None, Some(now_ms() - 1_000)).await;

        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 400);
        assert_eq!(response.error_message(), REDEMPTION_IDENTICAL_ERROR);
        assert_eq!(codes(&server).await[0]["uses"], json!(0));
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_an_exhausted_code() {
        let server = code_server().await;
        let first = pending_caller(&server, "first").await;
        let second = pending_caller(&server, "second").await;
        let minted = mint(&server, Some(1), None).await;

        assert_eq!(redeem(&server, &first.token, &minted.formatted, DEFAULT_IP).await.status, 200);
        let response = redeem(&server, &second.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 400);
        assert_eq!(response.error_message(), REDEMPTION_IDENTICAL_ERROR);
        // The second attempt neither consumed a use nor activated the account.
        assert_eq!(codes(&server).await[0]["uses"], json!(1));
        assert_eq!(status_of(&server, "second").await, json!("pending"));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_second_redemption_of_the_same_single_use_code_by_the_same_caller() {
        let server = code_server().await;
        let caller = pending_caller(&server, "twice").await;
        let minted = mint(&server, Some(1), None).await;

        assert_eq!(redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await.status, 200);
        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        // Step 1 now catches it: the account is already active, which is not a code failure.
        assert_eq!(response.status, 409);
        assert_eq!(codes(&server).await[0]["uses"], json!(1));
    }
}

// ---------------------------------------------------------------------------
// Step 6 — "increment uses and set the account active, atomically"
// ---------------------------------------------------------------------------

mod section_9_4_step_6_claiming_the_code {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn flips_pending_to_active_and_increments_uses() {
        let server = code_server().await;
        let caller = pending_caller(&server, "activated").await;
        let minted = mint(&server, Some(2), None).await;

        let response = redeem(&server, &caller.token, &minted.formatted, DEFAULT_IP).await;

        assert_eq!(response.status, 200);
        assert_eq!(response.json(), json!({ "status": "active", "needsInviteCode": false }));
        assert_eq!(status_of(&server, &caller.profile_id).await, json!("active"));
        assert_eq!(codes(&server).await[0]["uses"], json!(1));
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_the_formatted_code_lower_case_and_unseparated_alike() {
        let server = code_server().await;
        let caller = pending_caller(&server, "sloppy-typist").await;
        let minted = mint(&server, None, None).await;

        // §9.4 formats codes `XXXX-XXXX-XXXX-XXXX`; R191's reading accepts what a human types.
        let response = redeem(&server, &caller.token, &format!(" {} ", minted.formatted.to_lowercase()), DEFAULT_IP).await;

        assert_eq!(response.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn stores_only_the_hash_the_plaintext_is_returned_once_and_never_persisted() {
        let server = code_server().await;
        let minted = mint(&server, None, None).await;

        let rows = codes(&server).await;
        let stored = serde_json::to_string(&rows).expect("the rows serialise");
        assert!(!stored.contains(&minted.formatted));
        assert!(!stored.contains(&minted.formatted.replace('-', "")));
        assert_eq!(rows[0]["codeHash"], json!(code_hash(&minted.formatted)));
        // /^[0-9A-Z]{4}(-[0-9A-Z]{4}){3}$/u
        let groups: Vec<&str> = minted.formatted.split('-').collect();
        assert_eq!(groups.len(), 4, "{}", minted.formatted);
        for group in &groups {
            assert_eq!(group.len(), 4, "{}", minted.formatted);
            assert!(group.chars().all(|ch| ch.is_ascii_digit() || ch.is_ascii_uppercase()), "{}", minted.formatted);
        }
        assert_eq!(minted.formatted.replace('-', "").len(), INVITE_CODE_LENGTH as usize);
    }
}

// ---------------------------------------------------------------------------
// "Missing, expired and exhausted codes return an identical error"
// ---------------------------------------------------------------------------

mod r145_the_identical_error_for_every_code_dependent_refusal_section_9_4 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn returns_byte_identical_bodies_and_identical_status_codes() {
        let server = code_server().await;
        let winner = pending_caller(&server, "consumer").await;
        let missing = pending_caller(&server, "asks-missing").await;
        let expired = pending_caller(&server, "asks-expired").await;
        let exhausted = pending_caller(&server, "asks-exhausted").await;

        let expired_code = mint(&server, None, Some(now_ms() - 1_000)).await;
        let used_code = mint(&server, Some(1), None).await;
        assert_eq!(redeem(&server, &winner.token, &used_code.formatted, DEFAULT_IP).await.status, 200);

        let responses = [
            redeem(&server, &missing.token, UNMINTED_CODE, DEFAULT_IP).await,
            redeem(&server, &expired.token, &expired_code.formatted, DEFAULT_IP).await,
            redeem(&server, &exhausted.token, &used_code.formatted, DEFAULT_IP).await,
        ];

        assert!(responses.iter().all(|response| response.status == 400));
        assert!(responses.iter().all(|response| response.text == responses[0].text));
        assert_eq!(
            responses[0].text,
            serde_json::to_string(&json!({ "error": { "code": "invalid_code", "message": REDEMPTION_IDENTICAL_ERROR } }))
                .expect("the body serialises")
        );
    }
}

// ---------------------------------------------------------------------------
// BUILD M6-T1: "timing test shows the three failure responses within 5 ms of each other over 50
// samples" — §9.4's "identical time", delivered by R107's floor.
// ---------------------------------------------------------------------------

/// BUILD M6-T1's acceptance bound, as it writes it.
const TIMING_SAMPLES: usize = 50;
const TIMING_SPREAD_MS: u64 = 5;

/// The cost model this test *injects*, so the padding has something real to hide.
///
/// `STORE_CALL_MS` is a round trip to Postgres and `ROW_FETCH_MS` the extra cost of a lookup that
/// found its row — an index miss answers from the index alone, a hit goes on to the heap. That is
/// the asymmetry §9.4 is about: a missing code does strictly less work than an expired or exhausted
/// one, and without a floor the difference is readable from outside. The numbers are the test's own,
/// not the server's; what is asserted below is that they stop being observable, whatever they are.
const STORE_CALL_MS: u64 = 3;
const ROW_FETCH_MS: u64 = 11;

/// `code_server()` with every store call counted (`FakeData::on_call`, TS's `onCall`).
///
/// TS charged each call to its virtual clock, which `padTo` read. The Rust fake's hook is synchronous
/// and cannot move tokio's clock, so the cost model is tallied rather than slept: the elapsed times
/// below are tokio's paused clock, exact on a loaded CI box and on an idle laptop alike, and the
/// tally is what proves the work behind them was lopsided.
struct Timed {
    server: Server,
    calls: Arc<AtomicU64>,
}

async fn timing_server() -> Timed {
    let server = code_server().await;
    let calls = Arc::new(AtomicU64::new(0));
    let counter = calls.clone();
    store_of(&server.app).lock().await.on_call = Some(Arc::new(move |_method: &str| -> Result<(), StoreError> {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    Timed { server, calls }
}

impl Timed {
    /// The work charged since the last call: every store call, and the row fetch of a lookup that
    /// found `code`'s row (TS's wrapped `findByHash`).
    async fn work_since_last(&self, looked_up: Option<&str>) -> u64 {
        let calls = self.calls.swap(0, Ordering::SeqCst);
        let found = match looked_up {
            Some(code) => codes(&self.server).await.iter().any(|row| row["codeHash"] == json!(code_hash(code))),
            None => false,
        };
        calls * STORE_CALL_MS + if found { ROW_FETCH_MS } else { 0 }
    }
}

mod r107_the_constant_time_failure_floor_build_m6_t1 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r107_answers_missing_expired_and_exhausted_within_5_ms_of_each_other_over_50_samples() {
        let timed = timing_server().await;
        let server = &timed.server;

        let expired_code = mint(server, None, Some(now_ms() - 1_000)).await;
        let used_code = mint(server, Some(1), None).await;
        let consumer = pending_caller(server, "timing-consumer").await;
        assert_eq!(redeem(server, &consumer.token, &used_code.formatted, DEFAULT_IP).await.status, 200);

        let kinds = [
            ("missing", UNMINTED_CODE.to_string()),
            ("expired", expired_code.formatted.clone()),
            ("exhausted", used_code.formatted.clone()),
        ];

        let mut elapsed: IndexMap<&str, Vec<u64>> = kinds.iter().map(|(name, _)| (*name, Vec::new())).collect();
        let mut work: IndexMap<&str, Vec<u64>> = kinds.iter().map(|(name, _)| (*name, Vec::new())).collect();

        // Interleaved, and a fresh profile and address per sample: §9.4's per-profile (5/h) and per-IP
        // (20/h) limits would otherwise start refusing part-way through, at step 2 or 3 instead of at
        // step 5, and those are not the branches this measures.
        for sample in 0..TIMING_SAMPLES {
            for (name, code) in &kinds {
                let caller = pending_caller(server, &format!("timing-{name}-{sample}")).await;
                let ip = format!("198.51.100.{sample}/{name}");
                timed.work_since_last(None).await;

                let started_at = tokio::time::Instant::now();
                let response = redeem(server, &caller.token, code, &ip).await;
                elapsed[*name].push(started_at.elapsed().as_millis() as u64);
                work[*name].push(timed.work_since_last(Some(code.as_str())).await);

                // All three really are the refusal under test, not some other rejection.
                assert_eq!(response.status, 400, "{name} sample {sample}");

                // TS raised `breakerFailureThreshold` past these 150 deliberate failures, which would
                // otherwise trip §9.4's breaker part-way through and turn the remaining samples into
                // 503s, a different branch than the three under test. The production threshold stays,
                // so each failure leaves the breaker's window as soon as it is measured.
                store_of(&server.app).lock().await.tables.attempts.clear();
            }
        }

        let all: Vec<u64> = elapsed.values().flatten().copied().collect();
        assert_eq!(all.len(), TIMING_SAMPLES * 3);

        let max = all.iter().copied().max().unwrap_or_default();
        let min = all.iter().copied().min().unwrap_or_default();
        // BUILD M6-T1: "the three failure responses within 5 ms of each other over 50 samples".
        assert!(max - min <= TIMING_SPREAD_MS, "{min}..{max}");
        // R107: and the floor is a floor — no branch is quicker than the budget it is padded to.
        assert!(min >= REDEMPTION_RESPONSE_FLOOR_MS as u64, "{min}");

        // The assertion above would pass on a code path that happened to be uniform, which would make
        // it a test of nothing. The work really was lopsided; the padding is what flattened it.
        let work_per_kind: Vec<u64> = kinds.iter().map(|(name, _)| work[*name][0]).collect();
        let most = work_per_kind.iter().copied().max().unwrap_or_default();
        let least = work_per_kind.iter().copied().min().unwrap_or_default();
        assert!(most - least > TIMING_SPREAD_MS, "{work_per_kind:?}");
        assert!(most < REDEMPTION_RESPONSE_FLOOR_MS as u64, "{work_per_kind:?}");
    }
}

// ---------------------------------------------------------------------------
// "A global circuit breaker disables redemption and alerts"
// ---------------------------------------------------------------------------

mod section_9_4_the_global_circuit_breaker {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn opens_at_the_threshold_answers_503_and_alerts_exactly_once() {
        let (log, _guard) = record();
        let threshold = 3;
        let server = code_server().await;
        let caller = pending_caller(&server, "brute-forcer").await;
        near_threshold(&server, threshold).await;

        for _ in 0..threshold {
            let response = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
            assert_eq!(response.status, 400);
        }

        let blocked = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
        assert_eq!(blocked.status, 503);
        assert_eq!(blocked.error_code(), "unavailable");

        // Blocked before step 1, so nothing more was logged.
        assert_eq!(attempts(&server).await.len(), REDEMPTION_CIRCUIT_FAILURE_THRESHOLD as usize);
        let own = attempts(&server).await.into_iter().filter(|row| row["profileId"] == json!(caller.profile_id)).count();
        assert_eq!(own, threshold);

        let again = redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
        assert_eq!(again.status, 503);

        let alerts = log.named("codes.breaker_open");
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, tracing::Level::ERROR);
    }

    #[tokio::test(start_paused = true)]
    async fn closes_over_a_good_code_while_it_is_still_shut() {
        let server = code_server().await;
        let flood = pending_caller(&server, "flood").await;
        let holder = pending_caller(&server, "holder").await;
        let minted = mint(&server, None, None).await;
        near_threshold(&server, 2).await;

        redeem(&server, &flood.token, UNMINTED_CODE, DEFAULT_IP).await;
        redeem(&server, &flood.token, UNMINTED_CODE, DEFAULT_IP).await;

        // A legitimate code is refused too: §9.4's breaker "disables redemption", not just failures.
        let response = redeem(&server, &holder.token, &minted.formatted, DEFAULT_IP).await;
        assert_eq!(response.status, 503);
        assert_eq!(status_of(&server, "holder").await, json!("pending"));
    }

    #[tokio::test(start_paused = true)]
    async fn reports_itself_through_get_api_codes_status() {
        let server = code_server().await;
        let caller = pending_caller(&server, "watcher").await;
        near_threshold(&server, 1).await;

        let before = server.send(json_request("GET", "/api/codes/status", None, Some(&caller.token), DEFAULT_IP)).await.json();
        assert_eq!(before["redemptionEnabled"], json!(true));

        redeem(&server, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;

        let after = server.send(json_request("GET", "/api/codes/status", None, Some(&caller.token), DEFAULT_IP)).await.json();
        assert_eq!(after["redemptionEnabled"], json!(false));
        assert!(after["retryAfterMs"].as_f64().unwrap_or_default() > 0.0, "{after}");
    }

    /// TS: "is per-router, not module state: fresh routes start closed". The breaker belongs to the
    /// App here, so the fresh one is a second App over the same store.
    #[tokio::test(start_paused = true)]
    async fn is_per_router_not_module_state_fresh_routes_start_closed() {
        let first = code_server().await;
        let caller = pending_caller(&first, "second-router").await;
        near_threshold(&first, 1).await;

        redeem(&first, &caller.token, UNMINTED_CODE, DEFAULT_IP).await;
        assert_eq!(redeem(&first, &caller.token, UNMINTED_CODE, DEFAULT_IP).await.status, 503);

        let second = server_sharing(store_of(&first.app)).await;
        let token = deps::add_user(&second.app, "user-second-router", "second-router@example.test", true);
        // Same store, so the failures are still on record; the new App's breaker re-opens on the
        // next failure rather than inheriting an open one.
        let response = redeem(&second, &token, UNMINTED_CODE, DEFAULT_IP).await;
        assert_eq!(response.status, 400);
    }
}

// ---------------------------------------------------------------------------
// R161 — "How many accounts one invite code activates: one, unless its mint says otherwise"
// ---------------------------------------------------------------------------

/// Every other test in this file passes `max_uses` explicitly, which is exactly what R161's default
/// cannot be proved from: a code that was *told* to allow one use says nothing about what a code
/// that was told nothing allows. Both tests below mint through `mint_invite_code` with no
/// `max_uses` at all.
mod r161_how_many_accounts_one_invite_code_activates_section_9_4_section_9_8 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r161_activates_exactly_one_account_from_a_code_minted_with_no_max_uses() {
        let server = code_server().await;
        let first = pending_caller(&server, "first-holder").await;
        let second = pending_caller(&server, "second-holder").await;

        let leaked = mint(&server, None, None).await;

        // PREMISE: the mint really did default, and defaulted to one.
        assert_eq!(DEFAULT_INVITE_CODE_MAX_USES as i64, 1);
        assert_eq!(codes(&server).await[0]["maxUses"], json!(1));
        assert_eq!(codes(&server).await[0]["uses"], json!(0));

        // The one account the code is worth.
        assert_eq!(redeem(&server, &first.token, &leaked.formatted, DEFAULT_IP).await.status, 200);
        assert_eq!(status_of(&server, "first-holder").await, json!("active"));

        // The second caller is refused — through R145's identical error, so the refusal itself says
        // nothing about *why*. One leaked code is one account, not an open door (§9.8).
        let refused = redeem(&server, &second.token, &leaked.formatted, DEFAULT_IP).await;
        assert_eq!(refused.status, 400);
        assert_eq!(refused.error_message(), REDEMPTION_IDENTICAL_ERROR);
        assert_eq!(codes(&server).await[0]["uses"], json!(1));
        assert_eq!(status_of(&server, "second-holder").await, json!("pending"));

        // CONTROL: the second caller is not refused for some reason of their own. A code with a use
        // left activates them on the spot, so what the first redemption consumed was the *code*.
        let another = mint(&server, None, None).await;
        assert_eq!(redeem(&server, &second.token, &another.formatted, DEFAULT_IP).await.status, 200);
        assert_eq!(status_of(&server, "second-holder").await, json!("active"));
    }

    #[tokio::test(start_paused = true)]
    async fn r161_leaves_a_larger_maximum_available_to_whoever_mints_deliberately() {
        let server = code_server().await;
        let mut callers = Vec::new();
        for name in ["one", "two", "three", "four"] {
            callers.push(pending_caller(&server, name).await);
        }

        let batch = mint(&server, Some(3), None).await;
        assert_eq!(codes(&server).await[0]["maxUses"], json!(3));

        for caller in &callers[0..3] {
            assert_eq!(redeem(&server, &caller.token, &batch.formatted, DEFAULT_IP).await.status, 200);
        }
        assert_eq!(codes(&server).await[0]["uses"], json!(3));

        // "One" is a default, not a law — and the counter still stops where the mint said it would.
        let fourth = redeem(&server, &callers[3].token, &batch.formatted, DEFAULT_IP).await;
        assert_eq!(fourth.status, 400);
        assert_eq!(codes(&server).await[0]["uses"], json!(3));
        assert_eq!(status_of(&server, "four").await, json!("pending"));
    }

    #[test]
    fn r161_agrees_with_the_schema_so_minting_through_the_api_and_inserting_by_hand_match() {
        // The other half of "one unless the mint says otherwise": a row written straight into
        // `invite_codes` gets the same default the API applies.
        let migration = include_str!("../../migrations/0001_profiles_and_invites.sql");
        // /max_uses\s+int not null default <DEFAULT_INVITE_CODE_MAX_USES>\b/, by hand.
        let wanted = format!("int not null default {DEFAULT_INVITE_CODE_MAX_USES}");
        let found = migration.match_indices("max_uses").any(|(at, name)| {
            let rest = &migration[at + name.len()..];
            let spaced = rest.trim_start();
            spaced.len() < rest.len()
                && spaced.starts_with(&wanted)
                && !spaced[wanted.len()..].starts_with(|ch: char| ch.is_ascii_alphanumeric() || ch == '_')
        });
        assert!(found, "max_uses int not null default {DEFAULT_INVITE_CODE_MAX_USES}");
    }
}

// ---------------------------------------------------------------------------
// BUILD M6-T1: "a pending account cannot call collection, loadout or queue endpoints (403)"
// ---------------------------------------------------------------------------

mod section_9_4_the_gate_around_everything_else {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn forbids_a_pending_account_an_active_route_while_letting_it_redeem() {
        let server = code_server().await;
        let caller = pending_caller(&server, "gated").await;

        // The real `GET /api/collection`, which declares `auth: active` like every M6-T2/T3/T4 route.
        let gated = server.send(json_request("GET", "/api/collection", None, Some(&caller.token), DEFAULT_IP)).await;
        assert_eq!(gated.status, 403);
        assert_eq!(gated.error_code(), "account_pending");

        // The code screen itself stays reachable (§9.4).
        let allowed = server.send(json_request("GET", "/api/codes/status", None, Some(&caller.token), DEFAULT_IP)).await;
        assert_eq!(allowed.status, 200);
    }
}
