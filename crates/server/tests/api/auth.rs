//! SPEC §11 R159, R160, R194, R665 and R1443 — the rulings `src/auth.rs` and `src/api/auth.rs` make
//! about §9.4's front door.
//!
//!  - **R159**: §9.4 step 1's "verified email" is read from the auth provider, and only the
//!    *positive* answer may be remembered, briefly and per user id. A provider that cannot be
//!    reached fails closed.
//!  - **R160**: sign-up and sign-in answer identically for every outcome that depends on whether an
//!    account exists, so neither endpoint becomes an account-enumeration oracle. No sign-up is
//!    brokered at all (SURFACE §11.3) and sign-in by email is for the E2E fixtures only, so R160 is
//!    held by those two doors and by R1443's sign-in by username.
//!  - **R194**: a session the provider has ended is not honoured here either.
//!  - **R665**: an account with an authenticator app is honoured only at `aal2`.
//!  - **R1443**: a username signs in through the server, which asks the provider with the account's
//!    address and never sends that address back.
//!
//! Nothing here reaches the internet or the wall clock. [`GoTrue`] is a scripted GoTrue on
//! `127.0.0.1` answering the five endpoints the provider calls (the JWKS, `/auth/v1/user`, the admin
//! user lookup, the admin delete and the password grant) and counting every call. Its JWKS is empty, so tier 1 fails
//! *locally*; the cache clock is [`Clock`], a manual one, through the provider's `now` seam.
//!
//! Tokens are signed with `jsonwebtoken` against `SUPABASE_JWT_SECRET`, which is `verify`'s tier 2.
//! They carry an `exp` far in the future, so the expiry check passes without the test reading a clock.

use std::io;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use indexmap::{IndexMap, IndexSet};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use tower::ServiceExt;

use jackioh_server::app::App;
use jackioh_server::auth::{Auth, AuthError, SupabaseAuth, SupabaseAuthInput};
use jackioh_server::config::{AUTH_PROVIDER_TIMEOUT_SECONDS, AUTH_SESSION_LIVE_CACHE_SECONDS};
use jackioh_server::db::store::{Db, Profile};

use crate::support::deps::{call, test_app};

// Fixtures

pub(crate) const JWT_SECRET: &str = "hs256-secret-used-only-by-this-test";

/// GoTrue user ids are UUIDs: the real admin client refuses anything else before it asks.
const ALICE: &str = "a11ce000-0000-4000-8000-000000000001";
const BOB: &str = "b0b00000-0000-4000-8000-000000000002";

/// 2100-01-01: an expiry no test run reaches.
const FAR_FUTURE: i64 = 4_102_444_800;

/// `EMAIL_CONFIRMED_CACHE_TTL_MS` in the provider. R159 says only "briefly", so it is read as an
/// order of magnitude: the tests check that *some* window exists and that it *ends* (ten windows on).
const CACHE_TTL_MS: i64 = 30_000;

/// `AUTH_SESSION_LIVE_CACHE_SECONDS`, read as an order of magnitude (like `CACHE_TTL_MS`).
const LIVE_TTL_MS: i64 = AUTH_SESSION_LIVE_CACHE_SECONDS * 1000;

/// A confirmed GoTrue user: `email_confirmed_at` is the only field §9.4 step 1 reads.
pub(crate) fn confirmed_user(user_id: &str) -> Value {
    json!({
        "id": user_id,
        "email": format!("{user_id}@example.test"),
        "email_confirmed_at": "2026-01-01T00:00:00.000Z",
        "app_metadata": { "provider": "email" },
    })
}

fn unconfirmed_user(user_id: &str) -> Value {
    let mut user = confirmed_user(user_id);
    user["email_confirmed_at"] = Value::Null;
    user
}

/// GoTrue's user with these factors (`(status, factor_type)`), as `/auth/v1/user` and the admin
/// lookup both send it.
fn user_with_factors(user_id: &str, factors: &[(&str, &str)]) -> Value {
    let mut user = confirmed_user(user_id);
    user["factors"] = factors
        .iter()
        .enumerate()
        .map(|(index, (status, factor_type))| {
            json!({ "id": format!("factor-{index}"), "factor_type": factor_type, "status": status })
        })
        .collect();
    user
}

/// A confirmed user whose authenticator app is verified.
fn enrolled_user(user_id: &str) -> Value {
    user_with_factors(user_id, &[("verified", "totp")])
}

/// `{ userId: count }`, for comparing with [`GoTrue::lookups`].
fn counts(entries: &[(&str, u32)]) -> IndexMap<String, u32> {
    entries
        .iter()
        .map(|(user, count)| ((*user).to_string(), *count))
        .collect()
}

// A manual clock for the provider's caches

/// Epoch ms that move only when a test says so, read by the provider's caches through
/// `SupabaseAuthInput.now`. Tokio's clock is left alone: a paused runtime jumps to the next timer
/// whenever it waits on a socket, so the scripted GoTrue's requests would time out.
#[derive(Clone)]
pub(crate) struct Clock(Arc<AtomicI64>);

impl Clock {
    pub(crate) fn new() -> Clock {
        Clock(Arc::new(AtomicI64::new(1_700_000_000_000)))
    }

    pub(crate) fn now(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }

    /// Advances the clock by `ms`, as work would.
    pub(crate) async fn charge(&self, ms: i64) {
        self.0.fetch_add(ms.max(0), Ordering::SeqCst);
    }
}

// A scripted GoTrue

/// What the admin lookup of a user id answers.
pub(crate) enum AdminReply {
    /// 200 with this user (GoTrue's raw JSON).
    Ok(Value),
    /// 404: the auth server has no such user any more.
    Missing,
    /// 500: nobody useful answered.
    Unavailable,
}

/// What `GET /auth/v1/user` answers for a bearer token.
pub(crate) enum UserReply {
    /// 200 with this user: the token's session is live.
    Live(Value),
    /// 403 `session_not_found`: GoTrue's answer once `/logout` has deleted the session.
    Ended,
    /// 503: neither live nor ended, as the provider sees it.
    Unreachable,
    /// Never answers.
    Hang,
}

/// What the admin delete of a user id answers.
#[derive(Clone, Copy)]
pub(crate) enum DeletionReply {
    Deleted,
    Missing,
    Unavailable,
}

/// What `POST /auth/v1/token` answers: R1443's password grant.
#[derive(Clone)]
pub(crate) enum GrantReply {
    /// 200 with this body (GoTrue's raw JSON).
    Tokens(Value),
    /// 400 `invalid_credentials`: GoTrue's answer to a wrong password or an unconfirmed address.
    Refused,
    /// 429: GoTrue's own per-address limit.
    RateLimited,
    /// 500: nobody useful answered.
    Unavailable,
}

type AdminScript = Arc<dyn Fn(&str) -> AdminReply + Send + Sync>;
type UserScript = Arc<dyn Fn(&str) -> UserReply + Send + Sync>;

struct GoTrueState {
    admin: AdminScript,
    user: UserScript,
    deletion: DeletionReply,
    lookups: IndexMap<String, u32>,
    user_calls: u32,
    deleted: Vec<String>,
    /// Users a delete removed: every later admin lookup answers 404 for them, as GoTrue does.
    gone: IndexSet<String>,
    grant: GrantReply,
    /// Every password grant asked for: its query and its body.
    grants: Vec<(Option<String>, Value)>,
}

/// GoTrue on `127.0.0.1:<port>`, scripted per test and counting what it is asked.
#[derive(Clone)]
pub(crate) struct GoTrue {
    pub(crate) url: String,
    state: Arc<Mutex<GoTrueState>>,
}

async fn jwks() -> Json<Value> {
    // No keys: every token fails tier 1 here and reaches the shared secret.
    Json(json!({ "keys": [] }))
}

async fn admin_user(State(state): State<Arc<Mutex<GoTrueState>>>, Path(id): Path<String>) -> Response {
    let reply = {
        let mut state = state.lock().expect("GoTrue state");
        *state.lookups.entry(id.clone()).or_insert(0) += 1;
        if state.gone.contains(&id) {
            AdminReply::Missing
        } else {
            (state.admin.clone())(&id)
        }
    };
    match reply {
        AdminReply::Ok(user) => (StatusCode::OK, Json(user)).into_response(),
        AdminReply::Missing => (
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "error_code": "user_not_found", "msg": "User not found" })),
        )
            .into_response(),
        AdminReply::Unavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "msg": "Internal Server Error" })),
        )
            .into_response(),
    }
}

async fn admin_delete(State(state): State<Arc<Mutex<GoTrueState>>>, Path(id): Path<String>) -> Response {
    let reply = {
        let mut state = state.lock().expect("GoTrue state");
        state.deleted.push(id.clone());
        let reply = state.deletion;
        if matches!(reply, DeletionReply::Deleted) {
            state.gone.insert(id);
        }
        reply
    };
    match reply {
        DeletionReply::Deleted => (StatusCode::OK, Json(json!({}))).into_response(),
        DeletionReply::Missing => (
            StatusCode::NOT_FOUND,
            Json(json!({ "code": 404, "error_code": "user_not_found", "msg": "User not found" })),
        )
            .into_response(),
        DeletionReply::Unavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "msg": "Internal Server Error" })),
        )
            .into_response(),
    }
}

async fn user_by_token(State(state): State<Arc<Mutex<GoTrueState>>>, headers: HeaderMap) -> Response {
    let bearer = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .trim_start_matches("Bearer ")
        .to_string();
    let reply = {
        let mut state = state.lock().expect("GoTrue state");
        state.user_calls += 1;
        (state.user.clone())(&bearer)
    };
    match reply {
        UserReply::Live(user) => (StatusCode::OK, Json(user)).into_response(),
        UserReply::Ended => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": 403,
                "error_code": "session_not_found",
                "msg": "Session from session_id claim in JWT does not exist",
            })),
        )
            .into_response(),
        UserReply::Unreachable => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "msg": "upstream unavailable" })),
        )
            .into_response(),
        UserReply::Hang => std::future::pending::<Response>().await,
    }
}

async fn token(
    State(state): State<Arc<Mutex<GoTrueState>>>,
    RawQuery(query): RawQuery,
    Json(body): Json<Value>,
) -> Response {
    let reply = {
        let mut state = state.lock().expect("GoTrue state");
        state.grants.push((query, body));
        state.grant.clone()
    };
    match reply {
        GrantReply::Tokens(body) => (StatusCode::OK, Json(body)).into_response(),
        GrantReply::Refused => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "code": 400,
                "error_code": "invalid_credentials",
                "msg": "Invalid login credentials",
            })),
        )
            .into_response(),
        GrantReply::RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "code": 429, "error_code": "over_request_rate_limit", "msg": "Request rate limit reached" })),
        )
            .into_response(),
        GrantReply::Unavailable => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "code": 500, "msg": "Internal Server Error" })),
        )
            .into_response(),
    }
}

impl GoTrue {
    /// Starts one. The admin lookup answers a confirmed user, deletes succeed, `/auth/v1/user`
    /// answers as unreachable and a password grant is refused until a test scripts them.
    pub(crate) async fn start() -> GoTrue {
        let state = Arc::new(Mutex::new(GoTrueState {
            admin: Arc::new(|user_id: &str| AdminReply::Ok(confirmed_user(user_id))),
            user: Arc::new(|_: &str| UserReply::Unreachable),
            deletion: DeletionReply::Deleted,
            lookups: IndexMap::new(),
            user_calls: 0,
            deleted: Vec::new(),
            gone: IndexSet::new(),
            grant: GrantReply::Refused,
            grants: Vec::new(),
        }));
        let routes = axum::Router::new()
            .route("/auth/v1/.well-known/jwks.json", get(jwks))
            .route("/auth/v1/user", get(user_by_token))
            .route("/auth/v1/admin/users/{id}", get(admin_user).delete(admin_delete))
            .route("/auth/v1/token", post(token))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a free local port");
        let url = format!("http://{}", listener.local_addr().expect("the bound address"));
        tokio::spawn(async move {
            axum::serve(listener, routes)
                .await
                .expect("the scripted GoTrue serves");
        });
        GoTrue { url, state }
    }

    /// The issuer the provider expects: `${SUPABASE_URL}/auth/v1`.
    pub(crate) fn issuer(&self) -> String {
        format!("{}/auth/v1", self.url)
    }

    pub(crate) fn answer_admin(&self, reply: impl Fn(&str) -> AdminReply + Send + Sync + 'static) {
        self.state.lock().expect("GoTrue state").admin = Arc::new(reply);
    }

    fn answer_user(&self, reply: impl Fn(&str) -> UserReply + Send + Sync + 'static) {
        self.state.lock().expect("GoTrue state").user = Arc::new(reply);
    }

    pub(crate) fn answer_deletion(&self, reply: DeletionReply) {
        self.state.lock().expect("GoTrue state").deletion = reply;
    }

    /// How many times the auth server was actually asked, per user id.
    fn lookups(&self) -> IndexMap<String, u32> {
        self.state.lock().expect("GoTrue state").lookups.clone()
    }

    fn total(&self) -> u32 {
        self.lookups().values().sum()
    }

    fn user_calls(&self) -> u32 {
        self.state.lock().expect("GoTrue state").user_calls
    }

    pub(crate) fn deleted(&self) -> Vec<String> {
        self.state.lock().expect("GoTrue state").deleted.clone()
    }

    fn answer_grant(&self, reply: GrantReply) {
        self.state.lock().expect("GoTrue state").grant = reply;
    }

    /// Every password grant asked for, in order: its query and its body.
    fn grants(&self) -> Vec<(Option<String>, Value)> {
        self.state.lock().expect("GoTrue state").grants.clone()
    }
}

/// The Supabase provider under test, pointed at `gotrue`, with its caches on `clock`.
pub(crate) fn supabase_auth(gotrue: &GoTrue, clock: &Clock) -> Auth {
    let clock = clock.clone();
    Auth::Supabase(SupabaseAuth::new(SupabaseAuthInput {
        url: gotrue.url.clone(),
        secret_key: "secret-key".to_string(),
        jwt_secret: Some(JWT_SECRET.to_string()),
        now: Some(Arc::new(move || clock.now())),
        ..SupabaseAuthInput::default()
    }))
}

/// An HS256 token over `claims`, with §9.2's audience and an expiry far ahead.
pub(crate) fn sign(gotrue: &GoTrue, user_id: &str, mut claims: Value) -> String {
    claims["sub"] = json!(user_id);
    claims["iss"] = json!(gotrue.issuer());
    claims["aud"] = json!("authenticated");
    claims["exp"] = json!(FAR_FUTURE);
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("an HS256 token")
}

/// The provider, its GoTrue and its clock.
pub(crate) struct Harness {
    pub(crate) gotrue: GoTrue,
    pub(crate) clock: Clock,
    pub(crate) auth: Auth,
}

impl Harness {
    pub(crate) async fn new() -> Harness {
        let gotrue = GoTrue::start().await;
        let clock = Clock::new();
        let auth = supabase_auth(&gotrue, &clock);
        Harness { gotrue, clock, auth }
    }

    /// A tier-2 token: signed with the legacy shared secret, with §9.2's issuer and audience.
    pub(crate) fn token_for(&self, user_id: &str) -> String {
        sign(
            &self.gotrue,
            user_id,
            json!({ "email": format!("{user_id}@example.test"), "app_metadata": { "provider": "email" } }),
        )
    }

    /// A tier-2 token that names its provider session, as every Supabase access token does.
    fn session_token_for(&self, user_id: &str, session_id: &str) -> String {
        sign(
            &self.gotrue,
            user_id,
            json!({
                "email": format!("{user_id}@example.test"),
                "session_id": session_id,
                "app_metadata": { "provider": "email" },
            }),
        )
    }

    /// A tier-2 token at an assurance level, optionally naming its provider session.
    fn token_at(&self, user_id: &str, aal: &str, session_id: Option<&str>) -> String {
        let mut claims = json!({
            "email": format!("{user_id}@example.test"),
            "aal": aal,
            "app_metadata": { "provider": "email" },
        });
        if let Some(session_id) = session_id {
            claims["session_id"] = json!(session_id);
        }
        sign(&self.gotrue, user_id, claims)
    }

    /// The suite's server with this harness's GoTrue behind a provider of its own (same GoTrue,
    /// same clock).
    pub(crate) async fn app(&self) -> Arc<App> {
        app_with(supabase_auth(&self.gotrue, &self.clock)).await
    }
}

/// `support::deps::test_app()` (the fake store, the E2E fixtures) with `auth` in place of the
/// fixture auth. The App is fresh, so nothing else holds it yet.
pub(crate) async fn app_with(auth: Auth) -> Arc<App> {
    let Ok(mut app) = Arc::try_unwrap(test_app().await) else {
        panic!("support::deps::test_app() keeps a second handle on its App");
    };
    app.auth = auth;
    Arc::new(app)
}

/// One request through the real router, answered with its status and its exact body bytes (R145's
/// and R160's comparisons are byte for byte), written as `support::deps::call` writes a request.
pub(crate) async fn raw(
    app: &Arc<App>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (u16, String) {
    raw_from(app, "203.0.113.7", method, path, token, body).await
}

/// `raw` from the client address `address` (the suite trusts one proxy hop, so the last
/// `X-Forwarded-For` entry is the client's, R190).
pub(crate) async fn raw_from(
    app: &Arc<App>,
    address: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (u16, String) {
    let mut request = axum::http::Request::builder()
        .method(method)
        .uri(format!("https://server.test{path}"))
        .header("content-type", "application/json")
        .header("x-forwarded-for", address)
        .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            40_000,
        ))));
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = body.map_or_else(axum::body::Body::empty, |body| {
        axum::body::Body::from(body.to_string())
    });
    let response = jackioh_server::app::router(app.clone())
        .oneshot(request.body(body).expect("a request"))
        .await
        .expect("the router answers");
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the body");
    (status, String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"))
}

/// A store row from its JSON (camelCase).
pub(crate) fn row<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("a store row")
}

/// The fake store behind `app`.
pub(crate) fn fake_of(app: &App) -> &Arc<tokio::sync::Mutex<jackioh_server::db::fake::FakeData>> {
    let Db::Fake(data) = &app.db else {
        panic!("support::deps::test_app() runs on the fake store");
    };
    data
}

/// The fixture profile a user id resolves to (`e2e-p1`, …).
async fn profile_of(app: &Arc<App>, user_id: &str) -> Profile {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let profile = tx
        .profiles_get_by_user_id(user_id)
        .await
        .expect("a read")
        .expect("the fixture profile");
    tx.commit().await.expect("a commit");
    profile
}

/// A writer `tracing` can log into, so a test can read the lines a request wrote.
#[derive(Clone, Default)]
pub(crate) struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("captured log").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Captured {
    /// Every line logged so far.
    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("captured log")).into_owned()
    }

    /// Routes this thread's `tracing` output here until the guard drops (`#[tokio::test]` runs the
    /// test and every task it spawns on this one thread).
    pub(crate) fn install(&self) -> tracing::subscriber::DefaultGuard {
        let writer = self.clone();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_writer(move || writer.clone())
            .finish();
        crate::support::deps::set_log_default(subscriber)
    }
}

// R159

/// R159 — how long a verified email stays verified (§9.2, §9.4).
mod r159_how_long_a_verified_email_stays_verified {
    use super::*;

    #[tokio::test]
    async fn r159_remembers_a_confirmed_email_briefly_and_per_user_id_and_asks_again_once_it_lapses() {
        let h = Harness::new().await;
        let alice = h.token_for(ALICE);
        let bob = h.token_for(BOB);

        // PREMISE: the yes came from the auth server, not from the token.
        let first = h.auth.verify(&alice).await.expect("alice verifies");
        assert_eq!(first.user_id, ALICE);
        assert!(first.email_verified);
        assert_eq!(h.gotrue.lookups(), counts(&[(ALICE, 1)]));

        h.clock.charge(1).await;
        assert!(
            h.auth
                .verify(&alice)
                .await
                .expect("alice verifies")
                .email_verified
        );
        assert_eq!(h.gotrue.lookups(), counts(&[(ALICE, 1)]));

        // Per user id: Alice's yes says nothing about Bob, who is asked for on his own.
        assert!(h.auth.verify(&bob).await.expect("bob verifies").email_verified);
        assert_eq!(h.gotrue.lookups(), counts(&[(ALICE, 1), (BOB, 1)]));

        // …and the window ends. Ten times the constant is past any reading of "briefly".
        h.clock.charge(CACHE_TTL_MS * 10).await;
        assert!(
            h.auth
                .verify(&alice)
                .await
                .expect("alice verifies")
                .email_verified
        );
        assert_eq!(h.gotrue.lookups(), counts(&[(ALICE, 2), (BOB, 1)]));
    }

    #[tokio::test]
    async fn r159_never_caches_the_no_so_an_account_that_has_just_clicked_its_link_is_unlocked_at_once() {
        let h = Harness::new().await;
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(unconfirmed_user(user_id)));
        let alice = h.token_for(ALICE);

        let before = h.auth.verify(&alice).await.expect("alice verifies");
        assert_eq!(before.user_id, ALICE);
        assert!(!before.email_verified);
        assert_eq!(h.gotrue.total(), 1);

        // Asked again immediately: a negative is never remembered, so the provider is consulted afresh.
        assert!(
            !h.auth
                .verify(&alice)
                .await
                .expect("alice verifies")
                .email_verified
        );
        assert_eq!(h.gotrue.total(), 2);

        // The link is clicked. The clock does NOT move: R159's whole point is that the code screen
        // unlocks now rather than after a cache window.
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(confirmed_user(user_id)));
        assert!(
            h.auth
                .verify(&alice)
                .await
                .expect("alice verifies")
                .email_verified
        );
        assert_eq!(h.gotrue.total(), 3);
    }

    #[tokio::test]
    async fn r159_fails_closed_when_the_provider_cannot_be_reached_the_identity_stands_the_email_does_not() {
        let h = Harness::new().await;
        h.gotrue.answer_admin(|_| AdminReply::Unavailable);
        let alice = h.token_for(ALICE);

        let outage = h.auth.verify(&alice).await.expect("the identity stands");
        // The signature proved who this is, so the identity survives…
        assert_eq!(outage.user_id, ALICE);
        assert_eq!(
            outage.email.as_deref(),
            Some(format!("{ALICE}@example.test").as_str())
        );
        assert_eq!(
            serde_json::to_value(&outage.app_metadata).expect("JSON"),
            json!({ "provider": "email" })
        );
        // …but nothing proved the email, so §9.4 step 1 must not pass.
        assert!(!outage.email_verified);
        assert_eq!(h.gotrue.total(), 1);

        // An outage caches nothing either, in either direction: the next call asks again and the
        // recovered provider is believed immediately, with no clock movement.
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(confirmed_user(user_id)));
        assert!(
            h.auth
                .verify(&alice)
                .await
                .expect("alice verifies")
                .email_verified
        );
        assert_eq!(h.gotrue.total(), 2);
    }

    #[tokio::test]
    async fn r159_caches_only_what_the_provider_gave_a_deleted_user_is_refused_never_remembered() {
        let h = Harness::new().await;
        h.gotrue.answer_admin(|_| AdminReply::Missing);
        let alice = h.token_for(ALICE);

        // "The cache can only ever shorten the path to a yes the provider already gave" — a `missing`
        // is not a yes, so it writes nothing and the token is simply not honoured.
        assert!(h.auth.verify(&alice).await.is_err());
        assert!(h.auth.verify(&alice).await.is_err());
        assert_eq!(h.gotrue.total(), 2);
    }
}

// R194: a session the provider has ended is not honoured here either

/// The provider with `/auth/v1/user` scripted (R194's session check) and the admin lookup counted.
async fn with_sessions(user: impl Fn(&str) -> UserReply + Send + Sync + 'static) -> Harness {
    let h = Harness::new().await;
    h.gotrue.answer_user(user);
    h
}

/// R194 — an ended session's access token is refused here too (§9.2, §9.4).
mod r194_an_ended_sessions_access_token_is_refused_here_too {
    use super::*;

    #[tokio::test]
    async fn r194_refuses_a_well_signed_unexpired_token_whose_session_the_provider_has_ended() {
        let h = with_sessions(|_| UserReply::Ended).await;
        let token = h.session_token_for(ALICE, "session-revoked");

        // The signature and the audience are good: only the provider knows the session is gone.
        assert!(h.auth.verify(&token).await.is_err());
        assert_eq!(h.gotrue.user_calls(), 1);

        let app = h.app().await;
        let (status, _, _) = call(&app, "GET", "/api/auth/me", Some(&token), Value::Null).await;
        assert_eq!(status, 401);
    }

    #[tokio::test]
    async fn r194_remembers_a_live_session_only_briefly_so_an_ending_takes_effect_within_the_window() {
        let h = with_sessions(|_| UserReply::Live(confirmed_user(ALICE))).await;
        let token = h.session_token_for(ALICE, "session-a");

        let first = h.auth.verify(&token).await.expect("a live session");
        assert_eq!(first.user_id, ALICE);
        assert!(first.email_verified);
        assert_eq!(h.gotrue.user_calls(), 1);
        assert_eq!(h.gotrue.total(), 0);

        // Signed out elsewhere. Inside the window the live answer still stands…
        h.gotrue.answer_user(|_| UserReply::Ended);
        h.clock.charge(1).await;
        assert_eq!(
            h.auth.verify(&token).await.expect("still remembered").user_id,
            ALICE
        );
        assert_eq!(h.gotrue.user_calls(), 1);

        // …and once it lapses the provider is asked again, and the token is refused.
        h.clock.charge(LIVE_TTL_MS).await;
        assert!(h.auth.verify(&token).await.is_err());
        assert_eq!(h.gotrue.user_calls(), 2);
    }

    #[tokio::test]
    async fn r194_remembers_each_session_on_its_own_one_sessions_answer_says_nothing_about_anothers() {
        let h = with_sessions(|_| UserReply::Live(confirmed_user(ALICE))).await;
        let kept = h.session_token_for(ALICE, "session-kept");
        let other = h.session_token_for(ALICE, "session-other");

        assert_eq!(h.auth.verify(&kept).await.expect("a live session").user_id, ALICE);
        let ended = other.clone();
        h.gotrue.answer_user(move |token| {
            if token == ended {
                UserReply::Ended
            } else {
                UserReply::Live(confirmed_user(ALICE))
            }
        });
        // The same user, another session: asked on its own, and refused.
        assert!(h.auth.verify(&other).await.is_err());
        assert_eq!(h.auth.verify(&kept).await.expect("still live").user_id, ALICE);
        assert_eq!(h.gotrue.user_calls(), 2);
    }

    #[tokio::test]
    async fn r194_a_provider_that_cannot_be_reached_signs_nobody_out_and_proves_no_email() {
        let h = with_sessions(|_| UserReply::Unreachable).await;
        let token = h.session_token_for(ALICE, "session-a");

        let outage = h.auth.verify(&token).await.expect("the identity stands");
        assert_eq!(outage.user_id, ALICE);
        // The admin lookup still decides the email, as before the session check existed (R159).
        assert_eq!(h.gotrue.total(), 1);
        // Nothing was remembered: the next call asks the provider again.
        h.gotrue.answer_user(|_| UserReply::Ended);
        assert!(h.auth.verify(&token).await.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn r194_the_session_check_cannot_hang_a_request_it_goes_out_with_a_timeout() {
        // `/auth/v1/user` never answers, and the request still comes back, on the provider's own
        // timeout. The clock is paused so the timeout costs no real time.
        let h = with_sessions(|_| UserReply::Hang).await;
        let token = h.session_token_for(ALICE, "session-a");
        let bound = Duration::from_secs(u64::try_from(AUTH_PROVIDER_TIMEOUT_SECONDS * 2).expect("seconds"));
        let verified = tokio::time::timeout(bound, h.auth.verify(&token))
            .await
            .expect("the session check gave up on its own timeout");
        // Nobody answered, so the identity stands (R159's outage rule).
        assert_eq!(verified.expect("the identity stands").user_id, ALICE);
        assert_eq!(h.gotrue.user_calls(), 1);
    }

    #[tokio::test]
    async fn r194_never_honours_a_token_on_another_users_answer() {
        let h = with_sessions(|_| UserReply::Live(confirmed_user(BOB))).await;
        let token = h.session_token_for(ALICE, "session-a");
        assert!(h.auth.verify(&token).await.is_err());
    }

    #[tokio::test]
    async fn r194_a_token_that_names_no_session_is_verified_as_before_the_admin_lookup_no_session_check() {
        // Any call to `/auth/v1/user` is counted; a token with no `session_id` must not make one.
        let h = with_sessions(|_| UserReply::Ended).await;
        let token = h.token_for(ALICE);
        assert!(
            h.auth
                .verify(&token)
                .await
                .expect("alice verifies")
                .email_verified
        );
        assert_eq!(h.gotrue.user_calls(), 0);
        assert_eq!(h.gotrue.total(), 1);
    }
}

// R160

/// R160 — the identical sign-up and sign-in error (§9.2, §9.4, §9.8; extends R145).
///
/// The Supabase provider brokers no password by email (SURFACE §11.3: `sign_in` answers
/// unavailable, and `/api/auth/signup` is gone), so the doors R160 guards here are the E2E fixture
/// sign-in (`/api/auth/signin`, under `E2E=1` only) and the absent sign-up route. The sign-in by
/// username is R1443's, below.
mod r160_r145_the_identical_sign_up_and_sign_in_error {
    use super::*;

    async fn sign_in(app: &Arc<App>, email: &str, password: &str) -> (u16, String) {
        raw(
            app,
            "POST",
            "/api/auth/signin",
            None,
            Some(json!({ "email": email, "password": password })),
        )
        .await
    }

    #[tokio::test]
    async fn r160_answers_no_such_account_and_wrong_password_byte_identically_at_sign_in() {
        let app = test_app().await;
        let responses = [
            sign_in(&app, "ghost@example.test", "hunter2").await, // no such account
            sign_in(&app, "e2e-p1@jackioh.test", "hunter2").await, // wrong password
            sign_in(&app, "e2e-pending@jackioh.test", "hunter2").await, // a pending account's, also wrong
        ];

        assert!(responses.iter().all(|(status, _)| *status == 401));
        let bodies: IndexSet<&str> = responses.iter().map(|(_, body)| body.as_str()).collect();
        assert_eq!(bodies.len(), 1);
        for (_, body) in &responses {
            assert!(!body.to_lowercase().contains("invalid login credentials"));
            assert!(!body.contains("ghost@example.test"));
        }
    }

    #[tokio::test]
    async fn r160_brokers_no_sign_up_so_an_address_that_has_an_account_looks_exactly_like_a_new_one() {
        // The route is gone (sign-up is the browser's, against Supabase Auth, §9.2), so every
        // address gets the router's one "no such endpoint".
        let app = test_app().await;
        let body = |email: &str| json!({ "email": email, "password": "hunter2" });
        let taken = raw(
            &app,
            "POST",
            "/api/auth/signup",
            None,
            Some(body("e2e-p1@jackioh.test")),
        )
        .await;
        let fresh = raw(
            &app,
            "POST",
            "/api/auth/signup",
            None,
            Some(body("fresh@example.test")),
        )
        .await;
        assert_eq!(taken.0, 404);
        assert_eq!(taken, fresh);
        assert!(!taken.1.contains("already registered"));
    }

    #[tokio::test]
    async fn r160s_control_the_same_endpoint_still_tells_four_other_outcomes_apart() {
        // Control: an endpoint answering the same to everything would prove nothing about enumeration.
        let app = test_app().await;

        let refused = sign_in(&app, "x@y.test", "p").await;
        assert_eq!(refused.0, 401);

        let accepted = sign_in(&app, "e2e-p1@jackioh.test", "e2e-p1-password").await;
        assert_eq!(accepted.0, 200);
        assert!(accepted.1.contains("accessToken"));
        assert_ne!(accepted.1, refused.1);

        let malformed = raw(
            &app,
            "POST",
            "/api/auth/signin",
            None,
            Some(json!({ "email": "x@y.test" })),
        )
        .await;
        assert_eq!(malformed.0, 400);
        assert_ne!(malformed.1, refused.1);

        // A provider that brokers no password (every Supabase deployment): 503, saying where
        // sign-in happens (§9.2).
        let h = Harness::new().await;
        let disabled = sign_in(&h.app().await, "x@y.test", "p").await;
        assert_eq!(disabled.0, 503);
        assert_ne!(disabled.1, refused.1);

        let distinct: IndexSet<&str> = [&refused.1, &accepted.1, &malformed.1, &disabled.1]
            .into_iter()
            .map(String::as_str)
            .collect();
        assert_eq!(distinct.len(), 4);
    }

    #[tokio::test]
    async fn r160_flattens_a_provider_refusal_too_not_only_one_that_names_the_account() {
        // The fixture provider's refusal is a plain error ("invalid login credentials"); none of
        // its words, nor the address, reach the client.
        let app = test_app().await;
        let (status, body) = sign_in(&app, "e2e-p2@jackioh.test", "not-the-password").await;
        assert_eq!(status, 401);
        assert!(!body.to_lowercase().contains("invalid login credentials"));
        assert!(!body.contains("e2e-p2@jackioh.test"));
    }

    #[tokio::test]
    async fn r160_keeps_the_503_for_an_unconfigured_password_path_an_unavailable_not_a_flattened_401() {
        // The one rejection that is not about an account: passed through unchanged.
        let h = Harness::new().await;
        let refused = h.auth.sign_in("a@b.test", "p").await;
        assert!(matches!(refused, Err(AuthError::Unavailable { .. })));
    }
}

// R1443

/// R1443 — signing in with a username (§9.4, R160, R190, R192, R665).
///
/// The fixtures hold the usernames R144's reseed gives them, bare: `e2e_p1`, `e2e_p2`.
mod r1443_username_sign_in {
    use super::*;
    use jackioh_server::config::{USERNAME_SIGN_IN_ATTEMPTS_PER_WINDOW, USERNAME_SIGN_IN_WINDOW_MS};

    /// The address `raw` sends from.
    const ADDRESS: &str = "203.0.113.7";

    async fn sign_in_from(app: &Arc<App>, address: &str, username: &str, password: &str) -> (u16, String) {
        raw_from(
            app,
            address,
            "POST",
            "/api/auth/username-signin",
            None,
            Some(json!({ "username": username, "password": password })),
        )
        .await
    }

    async fn sign_in(app: &Arc<App>, username: &str, password: &str) -> (u16, String) {
        sign_in_from(app, ADDRESS, username, password).await
    }

    fn body_of(text: &str) -> Value {
        serde_json::from_str(text).expect("a JSON body")
    }

    async fn claim(app: &Arc<App>, token: &str, username: &str) {
        let (status, _, body) = call(
            app,
            "PUT",
            "/api/username",
            Some(token),
            json!({ "username": username }),
        )
        .await;
        assert_eq!(status, 200, "claim {username}: {body}");
    }

    #[tokio::test]
    async fn r1443_signs_in_by_username_in_any_case_and_with_a_tag() {
        let app = test_app().await;
        let (status, text) = sign_in(&app, "E2E_P1", "e2e-p1-password").await;
        assert_eq!(status, 200, "{text}");
        let body = body_of(&text);
        assert_eq!(body["session"]["accessToken"], json!("e2e-token-p1"));
        assert_eq!(body["emailVerified"], json!(true));
        assert_eq!(body["secondFactor"], Value::Null);

        // `Max` is p2's, bare; `Max#1` is p1's.
        claim(&app, "e2e-token-p2", "Max").await;
        claim(&app, "e2e-token-p1", "Max#1").await;
        let (status, text) = sign_in(&app, "max", "e2e-p2-password").await;
        assert_eq!(status, 200, "{text}");
        assert_eq!(body_of(&text)["session"]["accessToken"], json!("e2e-token-p2"));
        let (status, text) = sign_in(&app, " MAX#1 ", "e2e-p1-password").await;
        assert_eq!(status, 200, "{text}");
        assert_eq!(body_of(&text)["session"]["accessToken"], json!("e2e-token-p1"));
        // The tag names the account: `Max#1` with p2's password is not p2.
        assert_eq!(sign_in(&app, "Max#1", "e2e-p2-password").await.0, 401);
        // The name p1 gave up names nobody now.
        assert_eq!(sign_in(&app, "e2e_p1", "e2e-p1-password").await.0, 401);
    }

    #[tokio::test]
    async fn r1443_no_such_name_a_wrong_password_and_a_malformed_name_read_byte_identically() {
        let app = test_app().await;
        let responses = [
            sign_in(&app, "ghost", "hunter2").await,            // no such name
            sign_in(&app, "e2e_p1", "hunter2").await,           // wrong password
            sign_in(&app, "e2e_p1#x", "hunter2").await,         // no name at all
            sign_in(&app, "e2e_p1#1", "e2e-p1-password").await, // a tag the account does not carry
        ];
        assert!(
            responses.iter().all(|(status, _)| *status == 401),
            "{responses:?}"
        );
        let bodies: IndexSet<&str> = responses.iter().map(|(_, body)| body.as_str()).collect();
        assert_eq!(bodies.len(), 1);
        let body = responses[0].1.as_str();
        assert!(!body.contains('@'), "{body}");
        assert!(!body.to_lowercase().contains("invalid login credentials"));
        assert_ne!(
            body,
            sign_in_email_refusal(&app).await,
            "its own sentence, not the email door's"
        );

        let missing = raw(
            &app,
            "POST",
            "/api/auth/username-signin",
            None,
            Some(json!({ "username": "e2e_p1" })),
        )
        .await;
        assert_eq!(missing.0, 400);
        let missing = raw(
            &app,
            "POST",
            "/api/auth/username-signin",
            None,
            Some(json!({ "password": "e2e-p1-password" })),
        )
        .await;
        assert_eq!(missing.0, 400);
    }

    /// The email door's refusal, for telling the two apart.
    async fn sign_in_email_refusal(app: &Arc<App>) -> String {
        raw(
            app,
            "POST",
            "/api/auth/signin",
            None,
            Some(json!({ "email": "ghost@example.test", "password": "hunter2" })),
        )
        .await
        .1
    }

    #[tokio::test(start_paused = true)]
    async fn r1443_limits_attempts_per_address_and_per_name() {
        let app = test_app().await;
        for _ in 0..USERNAME_SIGN_IN_ATTEMPTS_PER_WINDOW {
            assert_eq!(sign_in_from(&app, "198.51.100.1", "e2e_p1", "wrong").await.0, 401);
        }
        // The address is spent, whichever name it tries next and with the right password…
        let (status, text) = sign_in_from(&app, "198.51.100.1", "e2e_p2", "e2e-p2-password").await;
        assert_eq!(status, 429, "{text}");
        let wait = body_of(&text)["error"]["details"]["retryAfterMs"]
            .as_i64()
            .expect("R192's wait");
        assert!(wait > 0 && wait <= USERNAME_SIGN_IN_WINDOW_MS, "{wait}");
        // …and so is the name, from any other address.
        let (status, _) = sign_in_from(&app, "198.51.100.2", "E2E_P1", "e2e-p1-password").await;
        assert_eq!(status, 429);
        // Another name from another address is untouched: the refused try above never reached it.
        let (status, _) = sign_in_from(&app, "198.51.100.2", "e2e_p2", "e2e-p2-password").await;
        assert_eq!(status, 200);

        // The wait it named is the wait.
        tokio::time::advance(Duration::from_millis(u64::try_from(wait).expect("a wait"))).await;
        let (status, _) = sign_in_from(&app, "198.51.100.1", "e2e_p1", "e2e-p1-password").await;
        assert_eq!(status, 200);
    }

    #[tokio::test]
    async fn r1443_asks_the_provider_with_the_accounts_address_and_never_sends_it_back() {
        let h = Harness::new().await;
        h.gotrue.answer_grant(GrantReply::Tokens(json!({
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "expires_in": 3600,
            "user": {
                "id": "e2e-p1",
                "email": "e2e-p1@jackioh.test",
                "email_confirmed_at": "2026-10-01T00:00:00Z",
                "factors": [{ "id": "factor-1", "factor_type": "totp", "status": "verified" }],
            },
        })));
        let app = h.app().await;

        let (status, text) = sign_in(&app, "E2E_P1", "pw").await;
        assert_eq!(status, 200, "{text}");
        assert_eq!(
            body_of(&text),
            json!({
                "session": { "accessToken": "access-1", "refreshToken": "refresh-1", "expiresIn": 3600 },
                "emailVerified": true,
                "secondFactor": "factor-1",
            })
        );
        assert!(!text.contains("e2e-p1@jackioh.test"));
        assert_eq!(
            h.gotrue.grants(),
            vec![(
                Some("grant_type=password".to_string()),
                json!({ "email": "e2e-p1@jackioh.test", "password": "pw" }),
            )]
        );
    }

    #[tokio::test]
    async fn r1443_turns_the_providers_refusal_limit_and_outage_into_ours() {
        let h = Harness::new().await;
        let app = h.app().await;
        let log = Captured::default();
        let _guard = log.install();

        // A refusal (the scripted GoTrue's default) reads as no such name does, and no such name
        // never reaches the provider.
        let refused = sign_in(&app, "e2e_p1", "pw-never-logged").await;
        let ghost = sign_in(&app, "ghost", "pw-never-logged").await;
        assert_eq!(refused.0, 401);
        assert_eq!(refused, ghost);
        assert_eq!(h.gotrue.grants().len(), 1);
        let logged = log.text();
        assert!(logged.contains("auth.username_signin_rejected"), "{logged}");
        for secret in ["e2e-p1@jackioh.test", "e2e_p1", "pw-never-logged"] {
            assert!(!logged.contains(secret), "the log names {secret}: {logged}");
        }

        h.gotrue.answer_grant(GrantReply::RateLimited);
        let (status, text) = sign_in(&app, "e2e_p1", "pw").await;
        assert_eq!(status, 429);
        assert_eq!(
            body_of(&text)["error"]["details"]["retryAfterMs"],
            json!(USERNAME_SIGN_IN_WINDOW_MS)
        );
        assert_ne!(text, refused.1);

        h.gotrue.answer_grant(GrantReply::Unavailable);
        assert_eq!(sign_in(&app, "e2e_p1", "pw").await.0, 503);

        // An answer that is not a session is no session: no token, or an authenticator app the
        // session could never be raised past (R665) for want of its id.
        for answer in [
            json!({ "refresh_token": "refresh-1" }),
            json!({
                "access_token": "access-1",
                "user": { "factors": [{ "factor_type": "totp", "status": "verified" }] },
            }),
        ] {
            h.gotrue.answer_grant(GrantReply::Tokens(answer));
            assert_eq!(sign_in(&app, "e2e_p1", "pw").await.0, 503);
        }
    }
}

// `currentMatchId` on /api/auth/me (§9.5)

/// The read that makes a two-player game reachable. In both lobby flows only ONE player's HTTP
/// response carries the match id (the joiner of a room, or whoever enqueued second), and no other
/// route names a profile's match, so the other player needs `/api/auth/me` to learn it.
///
/// `profiles.current_match_id` is set when a match starts and cleared by every ending (§9.5), so it
/// is the authoritative answer to "am I in a match", and the way back in after a reload that lost
/// the URL. The caller is the E2E fixture `e2e-p1` (`e2e-token-p1`).
mod api_auth_me_reports_the_callers_own_current_match {
    use super::*;

    const TOKEN: &str = "e2e-token-p1";

    #[tokio::test]
    async fn is_null_for_an_account_that_is_not_in_a_match() {
        let app = test_app().await;
        let profile = profile_of(&app, "e2e-p1").await;
        assert_eq!(profile.in_match_id, None, "premise: not in a match");

        let (status, _, body) = call(&app, "GET", "/api/auth/me", Some(TOKEN), Value::Null).await;
        assert_eq!(status, 200);
        assert_eq!(body["currentMatchId"], Value::Null);
    }

    #[tokio::test]
    async fn names_the_match_once_the_player_is_in_one_which_is_what_the_waiting_player_reads() {
        let app = test_app().await;
        let profile = profile_of(&app, "e2e-p1").await;
        let mut tx = app
            .db
            .begin(Some(profile.id.as_str()))
            .await
            .expect("a transaction");
        tx.profiles_set_in_match(&profile.id, Some("match-42"))
            .await
            .expect("a write");
        tx.commit().await.expect("a commit");

        let (status, _, body) = call(&app, "GET", "/api/auth/me", Some(TOKEN), Value::Null).await;
        assert_eq!(status, 200);
        assert_eq!(
            body["currentMatchId"],
            json!("match-42"),
            "the id /play navigates to"
        );
    }

    #[tokio::test]
    async fn names_the_callers_own_series_while_it_is_not_over_and_only_then_r259_r264() {
        let app = test_app().await;
        let profile = profile_of(&app, "e2e-p1").await;
        let read = || async {
            call(&app, "GET", "/api/auth/me", Some(TOKEN), Value::Null)
                .await
                .2
        };

        assert_eq!(
            read().await["currentSeriesId"],
            Value::Null,
            "premise: in no series"
        );

        // Between the games of a series nobody is in a match, and this is how the player who waited
        // learns there is a deck to pick.
        let trio = json!({
            "name": "t",
            "decks": [{ "name": "a", "cards": [] }, { "name": "b", "cards": [] }, { "name": "c", "cards": [] }],
        });
        let series = json!({
            "id": "series-7",
            "sides": [
                { "profileId": profile.id, "trio": trio, "wins": 0, "pick": null },
                { "profileId": "rival", "trio": trio, "wins": 0, "pick": null },
            ],
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": false,
            "seedBase": "s",
            "status": "picking",
            "games": [],
            "nextMatchId": "reserved",
            "pickDeadline": null,
            "winner": null,
            "endReason": null,
            "ratingBefore": null,
            "ratingAfter": null,
            "createdAt": 0,
            "updatedAt": 0,
            "endedAt": null,
            "version": 1,
        });
        let mut tx = app.db.begin(None).await.expect("a transaction");
        tx.series_create(&row(series.clone())).await.expect("a write");
        tx.commit().await.expect("a commit");

        let during = read().await;
        assert_eq!(during["currentSeriesId"], json!("series-7"));
        assert_eq!(during["currentMatchId"], Value::Null);

        // Over, it is nobody's current series.
        let mut over = series;
        over["status"] = json!("over");
        over["version"] = json!(2);
        let mut tx = app.db.begin(None).await.expect("a transaction");
        assert!(
            tx.series_update(&row(over)).await.expect("a write"),
            "the compare-and-set lands"
        );
        tx.commit().await.expect("a commit");
        assert_eq!(read().await["currentSeriesId"], Value::Null);
    }
}

// GET /api/profile (§9.5) — the account screen's read

/// `/api/profile` reports identity and the ladder record. The caller is the active fixture `e2e-p1`.
mod api_profile_reports_identity_and_the_ladder_record {
    use super::*;

    const TOKEN: &str = "e2e-token-p1";

    #[tokio::test]
    async fn reports_the_address_the_account_is_tied_to_so_a_player_can_see_who_they_are() {
        let app = test_app().await;
        let (status, _, body) = call(&app, "GET", "/api/profile", Some(TOKEN), Value::Null).await;
        assert_eq!(status, 200);
        assert_eq!(body["email"], json!("e2e-p1@jackioh.test"));
        assert_eq!(body["status"], json!("active"));
    }

    /// Nothing played is not the same claim as a 0% win rate, so it is null rather than 0.
    #[tokio::test]
    async fn is_a_null_win_rate_not_zero_before_any_match_is_finished() {
        let app = test_app().await;
        let (_, _, body) = call(&app, "GET", "/api/profile", Some(TOKEN), Value::Null).await;
        assert_eq!(body["record"], json!({ "wins": 0, "losses": 0, "draws": 0 }));
        assert_eq!(body["winRate"], Value::Null);
    }

    /// A winnerless row is a DRAW (§9.5 makes the ceiling, a mutual hero death and an accepted draw
    /// all winnerless), and a draw counts as played while being neither a win nor a loss — so one
    /// win, one loss and one draw is a third, not a half.
    #[tokio::test]
    async fn counts_wins_losses_and_draws_from_results_and_rates_on_all_three() {
        let app = test_app().await;
        let profile = profile_of(&app, "e2e-p1").await;
        let other = profile_of(&app, "e2e-p2").await;

        let result = |match_id: &str, winner: Option<&str>| {
            json!({
                "matchId": match_id,
                "players": [profile.id, other.id],
                "winnerProfileId": winner,
                "reason": if winner.is_none() { "match-ceiling" } else { "concede" },
                "turns": 3,
                "ratingBefore": [1000, 1000],
                "ratingAfter": [1000, 1000],
                "endedAt": 10,
            })
        };
        let mut tx = app.db.begin(None).await.expect("a transaction");
        tx.results_insert(&row(result("m1", Some(profile.id.as_str()))))
            .await
            .expect("a write");
        tx.results_insert(&row(result("m2", Some(other.id.as_str()))))
            .await
            .expect("a write");
        tx.results_insert(&row(result("m3", None)))
            .await
            .expect("a write");
        tx.commit().await.expect("a commit");

        let (_, _, body) = call(&app, "GET", "/api/profile", Some(TOKEN), Value::Null).await;
        assert_eq!(body["record"], json!({ "wins": 1, "losses": 1, "draws": 1 }));
        let win_rate = body["winRate"].as_f64().expect("a win rate");
        assert!((win_rate - 1.0 / 3.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn r612_sends_no_rating_here_or_in_api_auth_me() {
        let app = test_app().await;
        for path in ["/api/profile", "/api/auth/me"] {
            let (_, _, body) = call(&app, "GET", path, Some(TOKEN), Value::Null).await;
            let text = body.to_string().to_lowercase();
            for word in ["rating", "deviation", "volatility"] {
                assert!(!text.contains(word), "{path} names {word}");
            }
        }
    }
}

// R665: an account with an authenticator app is honoured only at `aal2`

/// R665 — two-step sign-in: an account with an authenticator app needs an aal2 token.
mod r665_two_step_sign_in_an_account_with_an_authenticator_app_needs_an_aal2_token {
    use super::*;

    #[tokio::test]
    async fn r665_refuses_an_aal1_token_for_an_account_with_a_verified_factor_and_takes_its_aal2_token() {
        let h = Harness::new().await;
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(enrolled_user(user_id)));

        // A password alone (aal1) is not enough once the account has an authenticator app…
        assert!(h.auth.verify(&h.token_at(ALICE, "aal1", None)).await.is_err());
        // …the code's level is, and the identity and the verified email come through as ever.
        let ok = h
            .auth
            .verify(&h.token_at(ALICE, "aal2", None))
            .await
            .expect("aal2 verifies");
        assert_eq!(ok.user_id, ALICE);
        assert!(ok.email_verified);

        // Through the router the aal1 token cannot even read the code screen's `/api/auth/me`.
        let app = h.app().await;
        let aal1 = h.token_at(ALICE, "aal1", None);
        let (status, _, _) = call(&app, "GET", "/api/auth/me", Some(&aal1), Value::Null).await;
        assert_eq!(status, 401);
    }

    #[tokio::test]
    async fn r665_an_account_without_a_factor_is_unchanged_aal1_or_no_aal_claim_at_all_is_honoured() {
        let h = Harness::new().await;
        assert_eq!(
            h.auth
                .verify(&h.token_at(ALICE, "aal1", None))
                .await
                .expect("aal1 verifies")
                .user_id,
            ALICE
        );
        assert_eq!(
            h.auth
                .verify(&h.token_for(BOB))
                .await
                .expect("no aal verifies")
                .user_id,
            BOB
        );
    }

    #[tokio::test]
    async fn r665_reads_the_factor_from_the_providers_user_only_a_verified_totp_factor_counts() {
        let unfinished =
            with_sessions(|_| UserReply::Live(user_with_factors(ALICE, &[("unverified", "totp")]))).await;
        let token = unfinished.token_at(ALICE, "aal1", Some("s-1"));
        assert_eq!(
            unfinished
                .auth
                .verify(&token)
                .await
                .expect("aal1 verifies")
                .user_id,
            ALICE
        );

        let other_kind =
            with_sessions(|_| UserReply::Live(user_with_factors(ALICE, &[("verified", "phone")]))).await;
        let token = other_kind.token_at(ALICE, "aal1", Some("s-2"));
        assert_eq!(
            other_kind
                .auth
                .verify(&token)
                .await
                .expect("aal1 verifies")
                .user_id,
            ALICE
        );

        let enrolled = with_sessions(|_| {
            UserReply::Live(user_with_factors(
                ALICE,
                &[("unverified", "totp"), ("verified", "totp")],
            ))
        })
        .await;
        assert!(
            enrolled
                .auth
                .verify(&enrolled.token_at(ALICE, "aal1", Some("s-3")))
                .await
                .is_err()
        );
        let token = enrolled.token_at(ALICE, "aal2", Some("s-4"));
        assert_eq!(
            enrolled.auth.verify(&token).await.expect("aal2 verifies").user_id,
            ALICE
        );
    }

    #[tokio::test]
    async fn r665_an_outage_cannot_lower_the_bar_the_last_answer_that_the_account_has_a_factor_stands() {
        let h = Harness::new().await;
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(enrolled_user(user_id)));
        assert!(h.auth.verify(&h.token_at(ALICE, "aal1", None)).await.is_err());

        // The provider goes away after the cached answer has lapsed: an aal1 token is still refused,
        // while the aal2 one keeps its identity (R159's outage rule).
        h.clock.charge(CACHE_TTL_MS * 10).await;
        h.gotrue.answer_admin(|_| AdminReply::Unavailable);
        assert!(h.auth.verify(&h.token_at(ALICE, "aal1", None)).await.is_err());
        assert_eq!(
            h.auth
                .verify(&h.token_at(ALICE, "aal2", None))
                .await
                .expect("aal2 verifies")
                .user_id,
            ALICE
        );
    }

    #[tokio::test]
    async fn r665_removing_the_factor_lowers_the_bar_again_once_the_provider_says_so() {
        let h = Harness::new().await;
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(enrolled_user(user_id)));
        assert!(h.auth.verify(&h.token_at(ALICE, "aal1", None)).await.is_err());

        h.clock.charge(CACHE_TTL_MS * 10).await;
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(confirmed_user(user_id)));
        assert_eq!(
            h.auth
                .verify(&h.token_at(ALICE, "aal1", None))
                .await
                .expect("aal1 verifies")
                .user_id,
            ALICE
        );
    }
}
