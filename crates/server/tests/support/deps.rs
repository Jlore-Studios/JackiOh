//! The test doubles every server test shares (SURFACE §11.2): an `App` over the fake store and the
//! fixture auth provider, a request helper that goes through the real router, and a recording log.
//! Port of `apps/server/test/fakes/deps.ts`.
//!
//! What changed with the port, and why:
//!
//! - **Time.** TS's manual and virtual timers (`createManualTimers`, `createVirtualTimers`) are
//!   gone: every deadline the server reads comes from `jackioh_server::app::now_ms()` and tokio's
//!   timers, so a test calls `tokio::time::pause()` and `tokio::time::advance(…)` instead
//!   (`#[tokio::test(start_paused = true)]` does the first). A padded response (`padTo`, §9.4) then
//!   costs no real time, as `createVirtualTimers` arranged.
//! - **Auth.** TS's `createFakeAuth` is the fixture provider itself (`auth::E2eAuth`): the three
//!   fixture tokens verify from the start, `add_user` registers more (token `token-<userId>`, as
//!   TS's), `set_email_verified` flips one, and the test app's provider deletes users (TS's fake had
//!   `deleteUser`; `E2eDeletion` chooses otherwise).
//! - **The catalog.** The real one (`crates/cards/catalog.json`), at the compiled-in version: the
//!   server runs the real validator (`jackioh_engine::validator`), so TS's synthetic 24-card
//!   catalog and its permissive and strict validators have no port to plug into. A test that wants
//!   a smaller catalog builds one with `create_test_catalog` and hands it to `test_app_with`.
//! - **Not ported**, because the server has no such port any more (SURFACE §11.3): `createFakeIds`
//!   (ids are `uuid` + `getrandom`), `fakeRandomDealer` (All Random deals through the engine),
//!   `createFakeMatchDirectory` (the registry is `actor::registry::Registry`), `testConfig` and
//!   `testLimits` (the numbers are `jackioh_server::config`'s), `TEST_PATCH_VERSION` (the season is
//!   read off the compiled-in catalog version).
//! - **The log.** TS's `createRecordingLogger` is `record_logs()`: a tracing layer installed as the
//!   thread's default subscriber, which the current-thread runtime of `#[tokio::test]` shares with
//!   every task it runs.

#![allow(dead_code)]

use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::http::HeaderMap;
use axum::response::Response;
use indexmap::IndexMap;
use serde_json::{Map, Value};
use tower::ServiceExt;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};

use jackioh_server::api::catalog::{Catalog, LoadCatalogOptions, catalog_from, load_catalog};
use jackioh_server::app::{self, App};
use jackioh_server::auth::{Auth, E2eAuth, E2eDeletion};
use jackioh_server::env::Env;

// ---------------------------------------------------------------------------
// The fixtures (`e2e/support/config.ts`, SURFACE §11.3)
// ---------------------------------------------------------------------------

/// `e2e-p1`'s bearer token: an active fixture account that owns every card.
pub const P1_TOKEN: &str = "e2e-token-p1";
/// `e2e-p2`'s bearer token: the other active fixture account.
pub const P2_TOKEN: &str = "e2e-token-p2";
/// `e2e-pending`'s bearer token: pending, with a verified email.
pub const PENDING_TOKEN: &str = "e2e-token-pending";

/// The catalog version the test app runs: the compiled-in one, which `env.rs` requires
/// `CATALOG_VERSION` to equal (TS's was the synthetic `"test-1"`).
pub const TEST_CATALOG_VERSION: &str = jackioh_cards::CATALOG_VERSION;

/// The `X-Forwarded-For` entry `json_request` writes by default, as a proxy would, so the tests model
/// the deployed server behind one proxy hop (`render.yaml`). The server's own default is 0 (R190),
/// and the client-address tests cover it by setting `TRUSTED_PROXY_HOPS`.
pub const DEFAULT_TEST_IP: &str = "203.0.113.7";

/// Unit conversion, not configuration (`app.rs`'s `API_RATE_WINDOW_MS`): R109's allowance is per
/// minute.
const API_RATE_WINDOW_MS: i64 = 60_000;

/// R190: the tests' proxy depth (`json_request` writes one entry).
const TEST_TRUSTED_PROXY_HOPS: &str = "1";

// ---------------------------------------------------------------------------
// The environment and the app
// ---------------------------------------------------------------------------

/// The environment the test app runs on: end-to-end mode's placeholders (`app::load_server_env`),
/// `NODE_ENV=test`, one trusted proxy hop, then `overrides` on top.
pub fn test_env(overrides: &[(&str, &str)]) -> Env {
    let mut source: IndexMap<String, String> = IndexMap::new();
    source.insert("E2E".to_string(), "1".to_string());
    source.insert("NODE_ENV".to_string(), "test".to_string());
    source.insert("TRUSTED_PROXY_HOPS".to_string(), TEST_TRUSTED_PROXY_HOPS.to_string());
    for (key, value) in overrides {
        source.insert((*key).to_string(), (*value).to_string());
    }
    app::load_server_env(&source).unwrap_or_else(|error| panic!("the test environment: {error:#}"))
}

/// What `test_app_with` builds differently from `test_app`.
#[derive(Default)]
pub struct TestAppOptions {
    /// Environment variables over `test_env`'s.
    pub env: Vec<(String, String)>,
    /// A catalog instead of the real one (`create_test_catalog`).
    pub catalog: Option<Catalog>,
    /// Leave the store empty: no fixture accounts, no fixture codes (TS `createTestDeps` started
    /// empty; R144's reseed is the end-to-end server's).
    pub skip_fixtures: bool,
    /// What the auth provider's `delete_user` does; `E2eDeletion::Deletes` when absent.
    pub deletion: Option<E2eDeletion>,
    /// A previous app's store, for a restart over it (19.6's series recovery). Used as it stands:
    /// R144's reseed would wipe it, so no fixtures are written.
    pub db: Option<jackioh_server::db::store::Db>,
}

/// SURFACE §11.2: the fake store, the fixture auth provider, the E2E fixtures (R144's reseed).
pub async fn test_app() -> Arc<App> {
    test_app_with(TestAppOptions::default()).await
}

/// TS `createTestDeps(overrides)`: the test app with the overrides `TestAppOptions` names.
pub async fn test_app_with(options: TestAppOptions) -> Arc<App> {
    let overrides: Vec<(&str, &str)> = options.env.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect();
    let env = test_env(&overrides);
    let mut catalog = match options.catalog {
        Some(catalog) => catalog,
        None => load_catalog(LoadCatalogOptions { version: Some(env.catalog_version.to_string()), json: None })
            .await
            .unwrap_or_else(|error| panic!("the catalog: {error}")),
    };
    if catalog.commit.is_none() {
        catalog.commit = env.deployed_commit.clone();
    }

    let auth = E2eAuth::new();
    auth.set_deletion(options.deletion.unwrap_or(E2eDeletion::Deletes));

    let restarted = options.db.is_some();
    let db = options.db.unwrap_or_else(jackioh_server::db::store::Db::fake);
    let app = Arc::new(App {
        env,
        db,
        auth: Auth::E2e(auth),
        matches: jackioh_server::actor::registry::Registry::new(),
        limiter: jackioh_server::api::http::create_rate_limiter(
            jackioh_server::config::API_REQUESTS_PER_MINUTE as _,
            API_RATE_WINDOW_MS as _,
        ),
        catalog,
        breaker: Mutex::new(jackioh_server::api::codes::create_breaker_state()),
    });
    if !options.skip_fixtures && !restarted {
        jackioh_server::api::e2e::seed_e2e_fixtures(&app)
            .await
            .unwrap_or_else(|error| panic!("R144's reseed: {error:?}"));
    }
    app
}

/// TS `createTestCatalog(version, count)`: the first `count` Core cards of the real catalog plus
/// its first token, enough for L3/L6 tests without the whole catalog.
pub fn create_test_catalog(version: &str, count: usize) -> Catalog {
    let mut defs = jackioh_engine::CardDefs::new();
    for (id, def) in jackioh_cards::CATALOG.iter().filter(|(id, def)| id.starts_with("core-") && !def.token).take(count) {
        defs.insert(id.clone(), def.clone());
    }
    if let Some((id, def)) = jackioh_cards::CATALOG.iter().find(|(_, def)| def.token) {
        defs.insert(id.clone(), def.clone());
    }
    catalog_from(defs, version)
}

// ---------------------------------------------------------------------------
// Auth (TS `createFakeAuth`)
// ---------------------------------------------------------------------------

/// The test app's auth provider.
pub fn fake_auth(app: &App) -> &E2eAuth {
    match &app.auth {
        Auth::E2e(auth) => auth,
        Auth::Supabase(_) => panic!("the test app runs on the fixture auth provider"),
    }
}

/// TS `FakeAuth.addUser`: registers a user and returns the bearer token that verifies as them.
pub fn add_user(app: &App, user_id: &str, email: &str, email_verified: bool) -> String {
    fake_auth(app).add_user(user_id, email, email_verified)
}

/// TS `FakeAuth.setEmailVerified`.
pub fn set_email_verified(app: &App, user_id: &str, verified: bool) {
    fake_auth(app).set_email_verified(user_id, verified);
}

// ---------------------------------------------------------------------------
// Requests (TS `jsonRequest`, `readJson`)
// ---------------------------------------------------------------------------

/// Anything that can hand over the app: `Arc<App>` or `&Arc<App>`.
pub trait AppHandle {
    fn app(&self) -> Arc<App>;
}

impl AppHandle for Arc<App> {
    fn app(&self) -> Arc<App> {
        self.clone()
    }
}

impl<T: AppHandle> AppHandle for &T {
    fn app(&self) -> Arc<App> {
        (*self).app()
    }
}

/// TS `jsonRequest(method, path, body, { token, ip })`. `body` is sent when it is not `null`;
/// `ip` is the `X-Forwarded-For` entry, `DEFAULT_TEST_IP` when absent.
pub fn json_request(method: &str, path: &str, body: &Value, token: Option<&str>, ip: Option<&str>) -> Request {
    let mut builder = axum::http::Request::builder()
        .method(method)
        .uri(format!("https://server.test{path}"))
        .header("content-type", "application/json")
        .header("x-forwarded-for", ip.unwrap_or(DEFAULT_TEST_IP));
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let body = if body.is_null() { Body::empty() } else { Body::from(body.to_string()) };
    builder.body(body).unwrap_or_else(|error| panic!("a test request: {error}"))
}

/// The request as it would arrive on a socket from `peer` (R190's fallback address).
pub fn with_peer(mut request: Request, peer: SocketAddr) -> Request {
    request.extensions_mut().insert(ConnectInfo(peer));
    request
}

/// TS `readJson(response)`: the body as JSON; `null` for an empty body, a string for a body that is
/// not JSON.
pub async fn read_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("a response body: {error}"));
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
}

/// One request through the real router (`app::router`, CORS and `api::http::dispatch` included).
pub async fn send(app: impl AppHandle, request: Request) -> (u16, HeaderMap, Value) {
    let response = app::router(app.app())
        .oneshot(request)
        .await
        .unwrap_or_else(|error: std::convert::Infallible| match error {});
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    (status, headers, read_json(response).await)
}

/// SURFACE §11.2: `call(app, method, path, token, body) -> (status, headers, body)`, a
/// `tower::ServiceExt::oneshot` over `app::router`, from `DEFAULT_TEST_IP` behind one proxy hop.
pub async fn call(
    app: impl AppHandle,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (u16, HeaderMap, Value) {
    send(app, json_request(method, path, &body, token, None)).await
}

/// `call` from another client address (the `X-Forwarded-For` entry).
pub async fn call_from(
    app: impl AppHandle,
    ip: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (u16, HeaderMap, Value) {
    send(app, json_request(method, path, &body, token, Some(ip))).await
}

// ---------------------------------------------------------------------------
// The log (TS `createRecordingLogger`)
// ---------------------------------------------------------------------------

/// One line the server logged: TS's `{ level, event, data }`. `level` is `"info"`, `"warn"` or
/// `"alert"` (tracing's ERROR).
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub level: &'static str,
    pub event: String,
    pub data: Value,
}

/// Every line logged on this thread while it lives.
pub struct RecordingLogger {
    entries: Arc<Mutex<Vec<LogEntry>>>,
    _guard: tracing::subscriber::DefaultGuard,
}

impl RecordingLogger {
    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// The lines with this event name, in order.
    pub fn named(&self, event: &str) -> Vec<LogEntry> {
        self.entries().into_iter().filter(|entry| entry.event == event).collect()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
    }
}

/// Starts recording this thread's log lines (keep the value alive for as long as you read them).
pub fn record_logs() -> RecordingLogger {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(Recorder { entries: entries.clone() });
    let guard = tracing::subscriber::set_default(subscriber);
    RecordingLogger { entries, _guard: guard }
}

struct Recorder {
    entries: Arc<Mutex<Vec<LogEntry>>>,
}

impl<S: tracing::Subscriber> Layer<S> for Recorder {
    fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let level = match *event.metadata().level() {
            tracing::Level::ERROR => "alert",
            tracing::Level::WARN => "warn",
            _ => "info",
        };
        let mut data = visitor.fields;
        let name = match data.remove("event") {
            Some(Value::String(name)) => name,
            Some(other) => other.to_string(),
            None => match data.remove("message") {
                Some(Value::String(message)) => message,
                Some(other) => other.to_string(),
                None => String::new(),
            },
        };
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(LogEntry { level, event: name, data: Value::Object(data) });
    }
}

#[derive(Default)]
struct FieldVisitor {
    fields: Map<String, Value>,
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(field.name().to_string(), Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(field.name().to_string(), Value::from(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields.insert(field.name().to_string(), Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.insert(field.name().to_string(), Value::from(value));
    }

    /// `%value` and `?value` fields: JSON when the text is JSON (the server logs lists and objects
    /// as `%serde_json::to_string(..)`), the text otherwise.
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        let text = format!("{value:?}");
        let parsed = serde_json::from_str(&text).unwrap_or(Value::String(text));
        self.fields.insert(field.name().to_string(), parsed);
    }
}
