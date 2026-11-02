//! Test doubles for server tests (SURFACE §11.2, §11.3): `App` over fake store,
//! fixture auth provider, real router request helper, and paused-clock timers (§9.4).

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::Response;
use serde_json::{Map, Value};
use tower::ServiceExt;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};

use jackioh_server::app::{self, App};
use jackioh_server::auth::{Auth, E2eAuth};

// The fixtures (`e2e/support/config.ts`, SURFACE §11.3)

pub use super::test_app::{TEST_CATALOG_VERSION, TestAppOptions, test_app, test_app_with};

/// The `X-Forwarded-For` entry `json_request` writes by default, as a proxy would, so the tests model
/// the deployed server behind one proxy hop (`render.yaml`). The server's own default is 0 (R190),
/// and the client-address tests cover it by setting `TRUSTED_PROXY_HOPS`.
pub const DEFAULT_TEST_IP: &str = "203.0.113.7";

/// R144: test app over empty store baseline without fixtures.
pub async fn empty_test_app() -> Arc<App> {
    test_app_with(TestAppOptions {
        skip_fixtures: true,
        ..TestAppOptions::default()
    })
    .await
}

// Auth

/// The test app's auth provider.
pub fn fake_auth(app: &App) -> &E2eAuth {
    match &app.auth {
        Auth::E2e(auth) => auth,
        Auth::Supabase(_) => panic!("the test app runs on the fixture auth provider"),
    }
}

/// Registers a user and returns verifying bearer token.
pub fn add_user(app: &App, user_id: &str, email: &str, email_verified: bool) -> String {
    fake_auth(app).add_user(user_id, email, email_verified)
}

// Requests

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

/// Sends JSON request with optional body, token, and IP.
pub fn json_request(
    method: &str,
    path: &str,
    body: &Value,
    token: Option<&str>,
    ip: Option<&str>,
) -> Request {
    let mut builder = axum::http::Request::builder()
        .method(method)
        .uri(format!("https://server.test{path}"))
        .header("content-type", "application/json")
        .header("x-forwarded-for", ip.unwrap_or(DEFAULT_TEST_IP));
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        Body::from(body.to_string())
    };
    builder
        .body(body)
        .unwrap_or_else(|error| panic!("a test request: {error}"))
}

/// Reads response body as JSON.
pub async fn read_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("a response body: {error}"));
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
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

/// `call` with an `Idempotency-Key` header holding `key` (R2124000), as `apps/web/src/net/api.ts`
/// sends a spend.
pub async fn call_with_key(
    app: impl AppHandle,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
    key: &str,
) -> (u16, HeaderMap, Value) {
    let mut request = json_request(method, path, &body, token, None);
    request.headers_mut().insert(
        "idempotency-key",
        HeaderValue::from_str(key).unwrap_or_else(|error| panic!("a header value: {error}")),
    );
    send(app, request).await
}

// The log

/// One line the server logged.
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
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// The lines with this event name, in order.
    pub fn named(&self, event: &str) -> Vec<LogEntry> {
        self.entries()
            .into_iter()
            .filter(|entry| entry.event == event)
            .collect()
    }
}

/// Starts recording this thread's log lines (keep the value alive for as long as you read them).
pub fn record_logs() -> RecordingLogger {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(Recorder {
        entries: entries.clone(),
    });
    let guard = set_log_default(subscriber);
    RecordingLogger {
        entries,
        _guard: guard,
    }
}

/// `tracing::subscriber::set_default` for a test's recorder after a process-wide no-op subscriber.
/// Prevents callsite interest caching from disabling events across parallel test threads.
pub fn set_log_default<S>(subscriber: S) -> tracing::subscriber::DefaultGuard
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    static GLOBAL: std::sync::Once = std::sync::Once::new();
    GLOBAL.call_once(|| {
        let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry());
    });
    tracing::subscriber::set_default(subscriber)
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
            .push(LogEntry {
                level,
                event: name,
                data: Value::Object(data),
            });
    }
}

#[derive(Default)]
struct FieldVisitor {
    fields: Map<String, Value>,
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), Value::String(value.to_string()));
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
