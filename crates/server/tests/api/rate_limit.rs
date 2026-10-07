//! SPEC §9.8's second flood limit: "Per-match rate limit in the actor, **per-account rate limit at
//! the API**", with R109's number — 300 requests per minute per account.
//!
//! The per-match half lives in `crates/server/src/actor/match_actor.rs` and is covered by
//! `tests/actor/match_actor.rs`; this is the API half, enforced in the router for every route at
//! once rather than at the top of each handler, so a new endpoint cannot forget it.
//!
//! The number under test is the real `API_REQUESTS_PER_MINUTE`, read from `config.rs` rather than
//! restated here, so this suite is what makes that constant enforced rather than merely declared.
//!
//! Ported from `apps/server/test/api/rate-limit.test.ts` (part 18). TS built routers of its own
//! (`createRouter([route("GET", "/api/open", "none", …)])`); the Rust server has one route table
//! (`app::ROUTES`), so each TS stand-in route is a real route of the same auth level:
//! `/api/open` (none) is `GET /api/stats/players`, `/api/mine` (user) is `GET /api/auth/me` and
//! `/api/gated` (active) is `GET /api/tutorial`. The limiter lives in `App` (SURFACE §11.2), so
//! TS's "per router" is "per App" here. Time is tokio's paused clock (SURFACE §11.2).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use indexmap::IndexMap;
use serde_json::{Value, json};
use sha2::Sha256;
use tower::ServiceExt;
use tracing_subscriber::layer::SubscriberExt;

use jackioh_server::api::http::{account_key, address_key, create_rate_limiter};
use jackioh_server::app::{self, App};
use jackioh_server::auth::{Auth, SupabaseAuth, SupabaseAuthInput};
use jackioh_server::config::API_REQUESTS_PER_MINUTE;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

use crate::support::deps::{add_user, test_app};

/// R109's allowance, as the server itself carries it.
const LIMIT: usize = API_REQUESTS_PER_MINUTE;
const MINUTE_MS: u64 = 60_000;

/// `jsonRequest`'s default `x-forwarded-for`, the entry one proxy hop writes (`render.yaml`).
const DEFAULT_IP: &str = "203.0.113.7";

/// The stand-ins for TS's own routes: one of each auth level.
const OPEN: &str = "/api/stats/players";
const MINE: &str = "/api/auth/me";
const GATED: &str = "/api/tutorial";

// ---------------------------------------------------------------------------
// Harness (a private copy per file, SURFACE rule 5)
// ---------------------------------------------------------------------------

/// One response, read whole.
struct Reply {
    status: u16,
    #[allow(dead_code)]
    headers: HeaderMap,
    text: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or(Value::Null)
    }
}

/// TS `router(jsonRequest(method, path, body, { token, ip }))`: one request through the real
/// router, with the `x-forwarded-for` entry the deployed proxy writes (the test app trusts one hop,
/// as TS's `createTestDeps` did).
async fn send(app: &Arc<App>, method: &str, path: &str, token: Option<&str>, ip: Option<&str>) -> Reply {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-forwarded-for", ip.unwrap_or(DEFAULT_IP));
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app::router(app.clone())
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("the router answers");
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

/// The test app with another auth provider in place of the fixture one.
async fn app_with_auth(auth: Auth) -> Arc<App> {
    let app = Arc::try_unwrap(test_app().await).unwrap_or_else(|_| panic!("test_app() hands back its only reference"));
    Arc::new(App { auth, ..app })
}

/// Every log line the server writes while the guard lives (TS's `createRecordingLogger`): each
/// event's fields by name, read off `tracing` (SURFACE §11.3: `Logger` → `tracing`, same `event`
/// names).
#[derive(Clone, Default)]
struct Logged(Arc<std::sync::Mutex<Vec<IndexMap<String, String>>>>);

impl Logged {
    fn install(&self) -> tracing::subscriber::DefaultGuard {
        crate::support::deps::set_log_default(tracing_subscriber::registry().with(self.clone()))
    }

    /// The lines whose `event` (or, failing that, whose message) is `name`.
    fn events(&self, name: &str) -> Vec<IndexMap<String, String>> {
        let lines = self.0.lock().expect("log lock");
        lines
            .iter()
            .filter(|line| {
                line.get("event").or_else(|| line.get("message")).map(|event| event.trim_matches('"')) == Some(name)
            })
            .cloned()
            .collect()
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Logged {
    fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        // `api::http::log_warn(event, data)` writes TS's data object as one `data` field of JSON
        // text; its keys are the line's fields, as TS's logger kept them.
        let mut line = fields.0;
        if let Some(Ok(Value::Object(data))) = line.get("data").map(|text| serde_json::from_str::<Value>(text)) {
            line.shift_remove("data");
            for (key, value) in data {
                line.insert(key, value.to_string());
            }
        }
        self.0.lock().expect("log lock").push(line);
    }
}

#[derive(Default)]
struct Fields(IndexMap<String, String>);

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_string(), format!("{value:?}"));
    }
}

/// TS `signIn`: an active account whose token verifies.
async fn sign_in(app: &App, id: &str) -> String {
    let user_id = format!("user-{id}");
    let token = add_user(app, &user_id, &format!("{id}@example.test"), true);
    fake(app).await.seed_profile(json!({ "id": id, "userId": user_id, "status": "active" }));
    token
}

/// Sends `n` requests to one path and returns the status of each.
async fn burst(app: &Arc<App>, n: usize, path: &str, token: Option<&str>, ip: Option<&str>) -> Vec<u16> {
    let mut statuses = Vec::with_capacity(n);
    for _ in 0..n {
        statuses.push(send(app, "GET", path, token, ip).await.status);
    }
    statuses
}

fn count(statuses: &[u16], status: u16) -> usize {
    statuses.iter().filter(|&&candidate| candidate == status).count()
}

// ---------------------------------------------------------------------------
// §9.8: the per-account API rate limit (R109)
// ---------------------------------------------------------------------------

mod the_per_account_api_rate_limit_r109 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r109_lets_an_account_through_300_requests_in_a_minute_and_refuses_the_301st_with_429() {
        let logged = Logged::default();
        let _guard = logged.install();
        let app = test_app().await;
        let token = sign_in(&app, "regular").await;

        let inside = burst(&app, LIMIT, MINE, Some(&token), None).await;
        assert_eq!(count(&inside, 200), LIMIT, "every request inside the allowance is served");

        let overflow = send(&app, "GET", MINE, Some(&token), None).await;
        assert_eq!(overflow.status, 429);
        assert_eq!(overflow.json()["error"]["code"], json!("rate_limited"));

        // §9.8: "Every rejected action is logged with its reason."
        let lines = logged.events("api.rate_limited");
        assert_eq!(lines.len(), 1);
        let key = lines[0].get("key").map(|key| key.trim_matches('"'));
        assert_eq!(key, Some(account_key("regular").as_str()));
    }

    #[tokio::test(start_paused = true)]
    async fn r137_s_reasoning_at_the_api_too_one_account_s_flood_never_spends_another_s_budget() {
        let app = test_app().await;
        let flooder = sign_in(&app, "flooder").await;
        let bystander = sign_in(&app, "bystander").await;

        let statuses = burst(&app, LIMIT + 5, MINE, Some(&flooder), None).await;
        assert_eq!(count(&statuses, 429), 5);

        // The victim of a shared counter would be refused here. The key is the account, so they are not.
        let theirs = send(&app, "GET", MINE, Some(&bystander), None).await;
        assert_eq!(theirs.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r157_keys_a_request_that_names_no_account_on_its_address_and_keeps_those_apart_too() {
        let app = test_app().await;

        let noisy = burst(&app, LIMIT + 1, OPEN, None, Some("203.0.113.9")).await;
        assert_eq!(noisy.last(), Some(&429));

        // A different address has its own budget: the open routes are not one shared bucket either.
        let quiet = send(&app, "GET", OPEN, None, Some("198.51.100.4")).await;
        assert_eq!(quiet.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn counts_a_request_whose_token_did_not_verify_so_a_flood_of_bad_tokens_is_bounded() {
        let app = test_app().await;

        let statuses = burst(&app, LIMIT + 1, MINE, Some("not-a-token"), Some("203.0.113.11")).await;
        // Every one of them is refused for being unauthenticated, until the address runs out of budget.
        assert_eq!(count(&statuses, 401), LIMIT);
        assert_eq!(statuses.last(), Some(&429));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_the_overflow_before_9_4_s_gate_so_a_pending_account_cannot_flood_for_free() {
        let app = test_app().await;
        let user_id = "user-pending";
        let token = add_user(&app, user_id, "pending@example.test", true);
        fake(&app).await.seed_profile(json!({ "id": "pending", "userId": user_id, "status": "pending" }));

        let statuses = burst(&app, LIMIT + 1, GATED, Some(&token), None).await;
        // 403 while the gate is what refuses them (§9.4), then 429 once the budget is gone.
        assert_eq!(count(&statuses, 403), LIMIT);
        assert_eq!(statuses.last(), Some(&429));
    }

    #[tokio::test(start_paused = true)]
    async fn rolls_the_window_a_minute_later_the_account_is_served_again() {
        let app = test_app().await;
        let token = sign_in(&app, "patient").await;

        burst(&app, LIMIT, MINE, Some(&token), None).await;
        assert_eq!(send(&app, "GET", MINE, Some(&token), None).await.status, 429);

        tokio::time::advance(Duration::from_millis(MINUTE_MS)).await;
        assert_eq!(send(&app, "GET", MINE, Some(&token), None).await.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn is_per_app_not_module_state_a_second_app_starts_with_an_empty_window() {
        // TS: "is per router, not module state: a second router starts with an empty window". The
        // limiter is `App::limiter` here, so a second App is what a second TS router was.
        let app = test_app().await;
        let token = sign_in(&app, "twice").await;
        burst(&app, LIMIT, MINE, Some(&token), None).await;
        assert_eq!(send(&app, "GET", MINE, Some(&token), None).await.status, 429);

        let fresh = test_app().await;
        let again = sign_in(&fresh, "twice").await;
        assert_eq!(send(&fresh, "GET", MINE, Some(&again), None).await.status, 200);
    }
}

// ---------------------------------------------------------------------------
// The sliding window itself
// ---------------------------------------------------------------------------

mod the_sliding_window_itself {
    use super::*;

    #[test]
    fn refuses_the_request_over_the_limit_without_recording_it_so_the_window_drains() {
        let limiter = create_rate_limiter(2, 1000);
        assert!(limiter.allow("k", 0));
        assert!(limiter.allow("k", 100));
        // Over the limit: refused, and not counted — otherwise retrying would keep it pinned open.
        assert!(!limiter.allow("k", 200));
        assert!(!limiter.allow("k", 300));

        // The two that *were* counted age out, and nothing the refusals did extends the window.
        assert!(limiter.allow("k", 1100));
    }

    #[test]
    fn drops_a_key_that_has_gone_quiet_for_a_whole_window_so_the_map_does_not_grow_for_ever() {
        let limiter = create_rate_limiter(5, 1000);
        for i in 0..50 {
            limiter.allow(&address_key(&format!("ip-{i}")), 0);
        }
        assert_eq!(limiter.size(), 50);

        limiter.allow(&account_key("still-here"), 2000);
        assert_eq!(limiter.size(), 1);
    }
}

// ---------------------------------------------------------------------------
// A flood of bad tokens costs the auth provider nothing past the address budget
// ---------------------------------------------------------------------------

mod a_flood_of_bad_tokens_costs_the_auth_provider_nothing_past_the_address_budget {
    use super::*;

    const FLOOD: usize = LIMIT + 100;

    /// A stand-in for the Supabase project: `GET /auth/v1/user` answers 401 and counts itself,
    /// and nothing else exists, so the JWKS is never published.
    async fn upstream() -> (String, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let project = axum::Router::new()
            .route(
                "/auth/v1/user",
                axum::routing::get(move || {
                    let counted = counted.clone();
                    async move {
                        counted.fetch_add(1, Ordering::SeqCst);
                        (StatusCode::UNAUTHORIZED, axum::Json(json!({ "msg": "invalid JWT" })))
                    }
                }),
            )
            .fallback(|| async { StatusCode::NOT_FOUND });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let url = format!("http://{}", listener.local_addr().expect("bound"));
        tokio::spawn(async move {
            axum::serve(listener, project).await.expect("the stand-in project serves");
        });
        (url, calls)
    }

    /// The real provider with the network replaced by a counter. Tier 1 fails (no JWKS) and there
    /// is no shared secret, so the only way a token reaches `GET /auth/v1/user` is tier 3.
    async fn flood_harness() -> (Arc<App>, Arc<AtomicUsize>) {
        let (url, calls) = upstream().await;
        let auth = SupabaseAuth::new(SupabaseAuthInput {
            url,
            secret_key: "secret-key".to_string(),
            ..SupabaseAuthInput::default()
        });
        (app_with_auth(Auth::Supabase(auth)).await, calls)
    }

    /// A legacy HS256 token for `sub`, signed with a secret this server was never given (TS:
    /// jose's `SignJWT`, here by hand over `hmac` + `sha2`).
    fn hs256_token(sub: &str, secret: &[u8]) -> String {
        let header = URL_SAFE_NO_PAD.encode(json!({ "alg": "HS256" }).to_string());
        let claims = URL_SAFE_NO_PAD.encode(json!({ "sub": sub }).to_string());
        let signing_input = format!("{header}.{claims}");
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(secret).expect("any key length");
        mac.update(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{signing_input}.{signature}")
    }

    // Real clock: these talk to a socket, and a paused clock would fire the provider's timeouts.

    #[tokio::test]
    async fn never_asks_the_provider_about_a_token_that_is_not_even_a_jwt() {
        let (app, upstream_calls) = flood_harness().await;
        let statuses = burst(&app, FLOOD, MINE, Some("x"), Some("203.0.113.21")).await;
        assert_eq!(count(&statuses, 401), LIMIT);
        assert_eq!(count(&statuses, 429), FLOOD - LIMIT);
        assert_eq!(upstream_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn asks_about_a_legacy_hs256_token_only_until_the_address_budget_is_spent() {
        let (app, upstream_calls) = flood_harness().await;
        let token = hs256_token("user-someone", b"a-secret-this-server-was-never-given");
        let statuses = burst(&app, FLOOD, MINE, Some(&token), Some("203.0.113.22")).await;
        assert_eq!(count(&statuses, 429), FLOOD - LIMIT);
        // One upstream call per request the budget admitted, and none for the ones it refused.
        let calls = upstream_calls.load(Ordering::SeqCst);
        assert!(calls <= LIMIT, "{calls} upstream calls");
        assert!(calls > 0, "tier 3 was never asked");
    }

    #[tokio::test(start_paused = true)]
    async fn still_serves_an_account_from_another_address_while_one_address_is_flooding() {
        let app = test_app().await;
        let token = sign_in(&app, "neighbour").await;
        burst(&app, LIMIT + 1, MINE, Some("not-a-token"), Some("203.0.113.23")).await;
        let theirs = send(&app, "GET", MINE, Some(&token), Some("198.51.100.23")).await;
        assert_eq!(theirs.status, 200);
    }
}
