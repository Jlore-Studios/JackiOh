//! The composition root (SURFACE §11.2): the only file that knows about the host, the framework and
//! the driver. Port of `apps/server/src/index.ts`.
//!
//! Everything below it depends on `App`, so this is where the abstract runtime becomes a concrete
//! one — Supabase Auth for identity, Postgres for the store, axum for HTTP and the WebSocket on the
//! same port, and the real engine for the rules. Hand it a Supabase project's URL and keys and it
//! runs (see README.md).
//!
//! Nothing here decides a rule or states a constant: every number is `crate::config`'s (R79).
//!
//! BUILD M8's `E2E=1` mode is chosen here and nowhere else. When `env.e2e` is true this file binds
//! two things differently — the in-memory store of `db/fake.rs` and the fixture provider of
//! `auth.rs` (`E2eAuth`) — and runs R144's reseed before the port opens. Everything else, the
//! route table included, is the same server. `env.rs` refuses `E2E` together with
//! `NODE_ENV=production`, so none of it is reachable in a production deployment.
//!
//! Not ported (SURFACE §11.3): `loadStore`, `STORE_EXPORT_CANDIDATES` and `StoreUnavailableError`
//! (the store is `db::store::Db`, chosen here), `createRuntime`'s port overrides and the `Runtime`
//! type (tests build an `App` through `build`, `tests/support/deps.rs`), and `RunningServer.close`
//! (the process serves until it is stopped).

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::anyhow;
use axum::Router;
use axum::extract::Request;
use axum::response::Response;
use indexmap::IndexMap;

use crate::actor;
use crate::api;
use crate::api::cors::{CorsOptions, VITE_DEV_ORIGINS};
use crate::api::http::{ApiResult, AuthLevel, Req};
use crate::auth::{Auth, E2eAuth, SupabaseAuth, SupabaseAuthInput};
use crate::db;
use crate::env::{self, Env};

/// The server: every port bound exactly once (TS `ServerDeps` + the registry, SURFACE §11.2).
pub struct App {
    pub env: Env,
    pub db: db::store::Db,
    pub auth: Auth,
    /// The live match actors (part 19). Queue, rooms and series start matches through it while the
    /// actor inside it reduces them.
    pub matches: actor::registry::Registry,
    /// §9.8's per-account rate limit at the API (R109, R157), one per app: a fresh app starts with an
    /// empty window, so one test's flood cannot leak into the next.
    pub limiter: api::http::RateLimiter,
    /// The catalog this build ships, its version and the deployed commit.
    pub catalog: api::catalog::Catalog,
    /// §9.4's redemption circuit breaker (R106), held here for the same reason as the limiter
    /// (TS kept it in `createCodesRoutes()`'s closure).
    pub breaker: Mutex<api::codes::BreakerState>,
}

// ---------------------------------------------------------------------------
// The clock
// ---------------------------------------------------------------------------

/// TS `systemTimers.now()`: epoch milliseconds. Anchored once to the wall clock and advanced by
/// tokio's clock, so a test that calls `tokio::time::pause()` and `advance()` moves every deadline
/// the server reads (SURFACE §11.2: they replace TS's manual timers), while a deployment reads the
/// wall clock as TS did.
pub fn now_ms() -> i64 {
    static ANCHOR: OnceLock<(tokio::time::Instant, i64)> = OnceLock::new();
    let (at, epoch_ms) = *ANCHOR.get_or_init(|| {
        let wall = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_millis() as i64)
            .unwrap_or(0);
        (tokio::time::Instant::now(), wall)
    });
    epoch_ms + tokio::time::Instant::now().saturating_duration_since(at).as_millis() as i64
}

// ---------------------------------------------------------------------------
// BUILD M8's `E2E=1` mode
// ---------------------------------------------------------------------------

/// Not in SPEC, and no R-row: boot ergonomics for the test mode, not a rule. These are placeholders
/// so `E2E=1 jackioh-server` — the command `e2e/README.md` documents — boots in a checkout with no
/// `.env`. The one value here that is a rule is `CATALOG_VERSION`, and it is not decided here: it
/// is the version this build compiled in (`jackioh_cards::catalog_version()`, SURFACE §11.3), which
/// `env.rs` requires the variable to equal.
///
/// `env.rs` validates the whole contract whatever the mode, and end-to-end mode reaches neither
/// Supabase (the fixture provider replaces it) nor Postgres (the in-memory store replaces it), so
/// demanding a project URL and a connection string would only be a puzzle for whoever runs the
/// suite. Every value below is filled in ONLY when the variable is unset, so a deployment that does
/// configure one keeps it, and none of this is reachable outside end-to-end mode — `env.rs` refuses
/// `E2E` together with `NODE_ENV=production`.
///
/// `CODE_PEPPER` is a throwaway: the fixture invite codes are checked into `e2e/support/config.ts`,
/// so their hashes protect nothing. The specs read the catalog version back from the server rather
/// than asserting a literal.
fn e2e_env_defaults() -> Vec<(&'static str, String)> {
    vec![
        ("SUPABASE_URL", "https://e2e-fixture-auth.invalid".to_string()),
        ("SUPABASE_SECRET_KEY", "e2e-fixture-auth-has-no-supabase".to_string()),
        ("DATABASE_URL", "memory://e2e-fixture-store".to_string()),
        ("CODE_PEPPER", "e2e-fixture-code-pepper-not-a-secret-abcdefgh".to_string()),
        ("PUBLIC_ORIGINS", VITE_DEV_ORIGINS.join(",")),
        ("CATALOG_VERSION", jackioh_cards::catalog_version().to_string()),
    ]
}

fn e2e_requested(source: &IndexMap<String, String>) -> bool {
    let raw = source.get("E2E").map(|value| value.trim());
    raw == Some("1") || raw == Some("true")
}

/// `env::load_env`, with the end-to-end placeholders applied first when `E2E` asks for them. The
/// production path is `load_env(environment)` exactly: when `E2E` is unset or false this function
/// adds nothing and changes no message.
pub fn load_server_env(source: &IndexMap<String, String>) -> anyhow::Result<Env> {
    if !e2e_requested(source) {
        return env::load_env(source).map_err(|error| anyhow!("{error}"));
    }
    let mut merged = source.clone();
    for (key, value) in e2e_env_defaults() {
        let unset = merged.get(key).is_none_or(|current| current.trim().is_empty());
        if unset {
            merged.insert(key.to_string(), value);
        }
    }
    env::load_env(&merged).map_err(|error| anyhow!("{error}"))
}

/// The browser origins this deployment trusts: `PUBLIC_ORIGINS` for CORS and the WebSocket `Origin`
/// check (`env.rs`), plus Vite's dev origins in end-to-end mode.
pub fn browser_origins(env: &Env) -> Vec<String> {
    let configured: Vec<String> = env.public_origins.iter().map(|origin| origin.to_string()).collect();
    if !env.e2e {
        return configured;
    }
    let mut origins = configured.clone();
    for origin in VITE_DEV_ORIGINS {
        if !configured.iter().any(|existing| existing == origin) {
            origins.push(origin.to_string());
        }
    }
    origins
}

// ---------------------------------------------------------------------------
// The route table
// ---------------------------------------------------------------------------

/// What a handler returns before it is awaited: boxed, so every route fits one table.
pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = ApiResult> + Send + 'a>>;

/// Every handler, boxed by `h!`: `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult`. SURFACE
/// §11.2 writes `&App`, but a handler that starts a match hands `Registry::start` the `&Arc<App>` its
/// actor keeps, so every handler takes the `Arc` (part 31; `.fullsend/notes/spec-gaps.md`).
pub type Handler = for<'a> fn(&'a Arc<App>, Req) -> HandlerFuture<'a>;

/// One route: method, path (`:name` segments land in `req.params`), auth level, handler — TS's
/// `route(method, path, auth, handler)`.
pub type Route = (&'static str, &'static str, AuthLevel, Handler);

/// Wraps `pub async fn name(app: &Arc<App>, req: Req) -> ApiResult` into a `Handler`.
macro_rules! h {
    ($handler:path) => {{
        fn boxed<'a>(app: &'a Arc<App>, req: Req) -> HandlerFuture<'a> {
            Box::pin($handler(app, req))
        }
        boxed as Handler
    }};
}

/// Every route the server serves, in one table, in TS `allRoutes()`'s order: the order the router
/// tries them in (`api::http::dispatch`), which tests assert. `GET /api/catalog` reports the
/// deployed commit in a header (`api::catalog`). Not ported (SURFACE §11.3): `POST
/// /api/auth/signup` and `GET /api/catalog/:version`.
///
/// Auth levels (§9.4): `None` is open; `User` is a verified token and a profile of any status (the
/// code screen); `Active` also needs `status = 'active'`.
pub static ROUTES: &[Route] = &[
    // api/auth.rs
    ("POST", "/api/auth/signin", AuthLevel::None, h!(api::auth::sign_in)),
    ("GET", "/api/profile", AuthLevel::Active, h!(api::auth::get_profile)),
    ("DELETE", "/api/account", AuthLevel::User, h!(api::auth::delete_account)),
    ("GET", "/api/auth/me", AuthLevel::User, h!(api::auth::get_me)),
    // api/catalog.rs
    ("GET", "/api/catalog", AuthLevel::None, h!(api::catalog::get_catalog)),
    // api/codes.rs
    ("POST", "/api/codes/redeem", AuthLevel::User, h!(api::codes::redeem)),
    ("GET", "/api/codes/status", AuthLevel::User, h!(api::codes::get_status)),
    // api/collection.rs
    ("GET", "/api/collection", AuthLevel::Active, h!(api::collection::get_collection)),
    // api/decks.rs
    ("GET", "/api/decks", AuthLevel::Active, h!(api::decks::list_decks)),
    ("PUT", "/api/decks/:id", AuthLevel::Active, h!(api::decks::put_deck)),
    ("DELETE", "/api/decks/:id", AuthLevel::Active, h!(api::decks::delete_deck)),
    ("PUT", "/api/trios/:id", AuthLevel::Active, h!(api::decks::put_trio)),
    ("POST", "/api/trios/import", AuthLevel::Active, h!(api::decks::import_trio)),
    ("DELETE", "/api/trios/:id", AuthLevel::Active, h!(api::decks::delete_trio)),
    // api/queue.rs
    ("POST", "/api/queue", AuthLevel::Active, h!(api::queue::enqueue)),
    ("DELETE", "/api/queue", AuthLevel::Active, h!(api::queue::dequeue)),
    ("GET", "/api/queue/population", AuthLevel::User, h!(api::queue::population)),
    // actor/rooms.rs (TS `match/rooms.ts`'s `create` and `join`)
    ("POST", "/api/rooms", AuthLevel::Active, h!(actor::rooms::create)),
    ("POST", "/api/rooms/:code/join", AuthLevel::Active, h!(actor::rooms::join)),
    // api/series.rs
    ("GET", "/api/series/:id", AuthLevel::Active, h!(api::series::get_series)),
    ("POST", "/api/series/:id/pick", AuthLevel::Active, h!(api::series::pick)),
    ("POST", "/api/series/:id/forfeit", AuthLevel::Active, h!(api::series::forfeit)),
    ("GET", "/api/matches/:matchId/series", AuthLevel::Active, h!(api::series::match_series)),
    // api/tutorial.rs
    ("GET", "/api/tutorial", AuthLevel::Active, h!(api::tutorial::get_tutorial)),
    ("PUT", "/api/tutorial", AuthLevel::Active, h!(api::tutorial::put_tutorial)),
    // api/ranked.rs
    ("GET", "/api/ranked", AuthLevel::Active, h!(api::ranked::get_ranked)),
    ("GET", "/api/leaderboard", AuthLevel::Active, h!(api::ranked::get_leaderboard)),
    ("GET", "/api/matches/:matchId/ranks", AuthLevel::Active, h!(api::ranked::get_match_ranks)),
    // api/rematch.rs
    ("POST", "/api/matches/:matchId/rematch", AuthLevel::Active, h!(api::rematch::offer_rematch)),
    ("GET", "/api/matches/:matchId/rematch", AuthLevel::Active, h!(api::rematch::rematch_status)),
    // api/settings.rs
    ("GET", "/api/settings", AuthLevel::Active, h!(api::settings::get_settings)),
    ("PUT", "/api/settings", AuthLevel::Active, h!(api::settings::put_settings)),
    // api/stats.rs
    ("GET", "/api/stats/cards", AuthLevel::None, h!(api::stats::get_cards)),
    ("GET", "/api/stats/cards/:id", AuthLevel::None, h!(api::stats::get_card)),
    ("GET", "/api/stats/player", AuthLevel::Active, h!(api::stats::get_player)),
    ("PUT", "/api/stats/player", AuthLevel::Active, h!(api::stats::put_player)),
    ("GET", "/api/stats/players", AuthLevel::None, h!(api::stats::get_players)),
];

/// TS `allRoutes()`.
pub fn all_routes() -> &'static [Route] {
    ROUTES
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

/// Unit conversion, not configuration: R109 states its API allowance *per minute*, and
/// `API_REQUESTS_PER_MINUTE` is the number itself (TS `http.ts`'s `API_RATE_WINDOW_MS`).
const API_RATE_WINDOW_MS: i64 = 60_000;

/// TS `createRuntime(env)` plus `start()`'s E2E half: assembles the app from the environment. Every
/// port is bound exactly once, here. Under `E2E=1`: the fake store, the fixture provider and R144's
/// reseed.
pub async fn build(env: Env) -> anyhow::Result<Arc<App>> {
    let mut catalog = api::catalog::load_catalog(api::catalog::LoadCatalogOptions {
        version: Some(env.catalog_version.to_string()),
        json: None,
    })
    .await?;
    catalog.commit = env.deployed_commit.clone();

    // BUILD M8, R144: end-to-end mode swaps exactly two things — the store and the auth provider —
    // and nothing else about this file changes.
    let (db, auth) = if env.e2e {
        // TS `createE2EStore({ catalog, now: timers.now })`: R111's launch grant reads the catalog.
        let (tokens, bans) = (catalog.clone(), catalog.clone());
        let store = db::fake::create_e2e_store(db::fake::E2eStoreOptions {
            catalog: db::fake::FakeCatalog {
                card_ids: catalog.card_ids.clone(),
                is_token: Arc::new(move |card_id: &str| tokens.is_token(card_id)),
                is_banned: Arc::new(move |card_id: &str| bans.is_banned(card_id)),
            },
            now: Arc::new(now_ms),
            redemption: None,
        });
        (store, Auth::E2e(E2eAuth::new()))
    } else {
        // The pool connects lazily, as `pg`'s did: a store that cannot be reached fails the first
        // request that needs it, loudly, rather than the boot.
        let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy(&env.database_url)?;
        let auth = Auth::Supabase(SupabaseAuth::new(SupabaseAuthInput {
            url: env.supabase_url.to_string(),
            secret_key: env.supabase_secret_key.to_string(),
            jwks_url: Some(env.supabase_jwks_url.to_string()),
            jwt_secret: env.supabase_jwt_secret.clone(),
            key_set: None,
            now: None,
        }));
        (db::store::Db::Pg(pool), auth)
    };

    let app = Arc::new(App {
        env,
        db,
        auth,
        matches: actor::registry::Registry::new(),
        limiter: api::http::create_rate_limiter(crate::config::API_REQUESTS_PER_MINUTE as _, API_RATE_WINDOW_MS as _),
        catalog,
        breaker: Mutex::new(api::codes::create_breaker_state()),
    });

    if app.env.e2e {
        // Loud, and at `alert` level, because a server holding fixture accounts that accept three
        // hard-coded bearer tokens must never be mistaken for a real one.
        tracing::error!(
            event = "server.e2e_mode",
            warning = "BUILD M8 fixture mode: static test tokens, an in-memory store and no database. Never a production deployment.",
            store = "in-memory",
            auth = "fixture",
        );
        // R144: reseeded on every start, before the port opens, so no request can land on half a
        // fixture set and so spec 10 is repeatable run after run.
        api::e2e::seed_e2e_fixtures(&app).await.map_err(|error| anyhow!("{error:?}"))?;
    }
    Ok(app)
}

const WS_PATH: &str = "/ws/match";

/// A WebSocket handshake (`Upgrade: websocket`), which `actor::ws_server::handle` answers; anything
/// else on the same path is an ordinary request for the API router (TS's `ws` server only ever saw
/// upgrades).
fn is_websocket_upgrade(request: &Request) -> bool {
    request
        .headers()
        .get("upgrade")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"))
}

/// The API: CORS first (preflights never reach the router), then TS's own router over `ROUTES`.
/// The browser and the API are separate origins (§9.2); without CORS every `fetch` from `apps/web`
/// is blocked before a handler runs.
async fn serve_api(app: Arc<App>, cors: Arc<CorsOptions>, request: Request) -> Response {
    api::cors::with_cors(&cors, request, move |request| async move {
        api::http::dispatch(&app, ROUTES, request).await
    })
    .await
}

/// SURFACE §11.2: `/ws/match` for the match socket (SPEC §9.2: one WebSocket per player, upgraded on
/// the same listener the API serves) and one fallback that runs `api::http::dispatch`, so TS's
/// matcher — its order and its 404 for a wrong method — decides every API answer, never axum's.
///
/// R190: the socket's peer address travels with the request (`ConnectInfo<SocketAddr>`, which
/// `serve` installs), because a request that did not come through the trusted proxy chain is keyed
/// on it rather than on anything the caller wrote.
pub fn router(app: Arc<App>) -> Router {
    let cors = Arc::new(CorsOptions { origins: browser_origins(&app.env) });

    let ws_app = app.clone();
    let ws_cors = cors.clone();
    let socket = move |request: Request| {
        let app = ws_app.clone();
        let cors = ws_cors.clone();
        async move {
            if is_websocket_upgrade(&request) {
                actor::ws_server::handle(app, request).await
            } else {
                serve_api(app, cors, request).await
            }
        }
    };

    let fallback = move |request: Request| {
        let app = app.clone();
        let cors = cors.clone();
        async move { serve_api(app, cors, request).await }
    };

    Router::new().route(WS_PATH, axum::routing::any(socket)).fallback(fallback)
}

/// One line of JSON per event on stdout (TS `consoleLogger`): enough for a hosted log drain,
/// nothing to configure. `RUST_LOG` narrows it; the default is `info`.
fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_target(false)
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .try_init();
}

/// TS `start(env)`: build, open the season, listen on `$PORT`, spawn the background loops.
pub async fn serve(env: Env) -> anyhow::Result<()> {
    init_logging();
    let app = build(env).await?;

    // R609: the build's season is open before the first request, with its soft reset if this build
    // begins one. A failure is loud but not fatal: the first rated game opens it in its own
    // transaction all the same.
    match api::ranked::open_season(&app).await {
        Ok(opened) => tracing::info!(
            event = "season.current",
            "seasonId" = %opened.season.id,
            opened = ?opened.opened,
            reset = ?opened.reset,
        ),
        Err(error) => tracing::error!(event = "season.open_failed", message = %format!("{error:?}")),
    }

    let origins = browser_origins(&app.env);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", app.env.port)).await?;
    tracing::info!(
        event = "server.listening",
        port = %app.env.port,
        "catalogVersion" = %app.catalog.version,
        origins = %serde_json::to_string(&origins).unwrap_or_default(),
        e2e = app.env.e2e,
        "trustedProxyHops" = %app.env.trusted_proxy_hops,
    );

    // §9.5: pairing runs on enqueue plus a sweeper (3 s), and a reaper resolves anything past the
    // ceiling (30 s). R260, R263: the series sweeper (5 s) runs the pick clock and starts a game a
    // restart left unstarted. The retention purge (`api/retention.rs`) runs once now, since a free
    // instance may sleep before an hour is up, and then on its interval. Each loop logs its own
    // failures and tries again on its next tick.
    tokio::spawn(api::queue::run_matchmaker(app.clone()));
    tokio::spawn(api::series::run_sweeper(app.clone()));
    tokio::spawn(api::results::run_reaper(app.clone()));
    tokio::spawn(api::retention::run_purge(app.clone()));

    axum::serve(listener, router(app).into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
