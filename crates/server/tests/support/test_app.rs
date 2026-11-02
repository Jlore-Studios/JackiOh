//! Test `App` builder (SURFACE §11.2): fake store, fixture auth provider, and R144 end-to-end fixtures.

use std::sync::{Arc, Mutex};

use indexmap::IndexMap;

use jackioh_server::api::catalog::{Catalog, LoadCatalogOptions, load_catalog};
use jackioh_server::app::{self, App};
use jackioh_server::auth::{Auth, E2eAuth, E2eDeletion};
use jackioh_server::env::Env;

// The fixtures (`e2e/support/config.ts`, SURFACE §11.3)

/// The catalog version the test app runs: the compiled-in one, which `env.rs` requires
/// `CATALOG_VERSION` to equal (TS's was the synthetic `"test-1"`).
pub const TEST_CATALOG_VERSION: &str = jackioh_cards::CATALOG_VERSION;

/// Unit conversion, not configuration (`app.rs`'s `API_RATE_WINDOW_MS`): R109's allowance is per
/// minute.
const API_RATE_WINDOW_MS: i64 = 60_000;

/// R190: the tests' proxy depth (`json_request` writes one entry).
const TEST_TRUSTED_PROXY_HOPS: &str = "1";

// The environment and the app

/// The environment the test app runs on: end-to-end mode's placeholders (`app::load_server_env`),
/// `NODE_ENV=test`, one trusted proxy hop, then `overrides` on top.
pub fn test_env(overrides: &[(&str, &str)]) -> Env {
    let mut source: IndexMap<String, String> = IndexMap::new();
    source.insert("E2E".to_string(), "1".to_string());
    source.insert("NODE_ENV".to_string(), "test".to_string());
    source.insert(
        "TRUSTED_PROXY_HOPS".to_string(),
        TEST_TRUSTED_PROXY_HOPS.to_string(),
    );
    for (key, value) in overrides {
        source.insert((*key).to_string(), (*value).to_string());
    }
    app::load_server_env(&source).unwrap_or_else(|error| panic!("the test environment: {error:#}"))
}

/// What `test_app_with` builds differently from `test_app`.
pub struct TestAppOptions {
    /// End-to-end mode (`env.e2e`, R143 seeds, R144 fixtures; SURFACE §11.2). True by default.
    pub e2e: bool,
    /// Environment variables over `test_env`'s.
    pub env: Vec<(String, String)>,
    /// A catalog instead of the real one.
    pub catalog: Option<Catalog>,
    /// Leave store empty with no fixture accounts or codes (R144).
    pub skip_fixtures: bool,
    /// What the auth provider's `delete_user` does; `E2eDeletion::Deletes` when absent.
    pub deletion: Option<E2eDeletion>,
    /// A previous app's store, for a restart over it (19.6's series recovery). Used as it stands:
    /// R144's reseed would wipe it, so no fixtures are written.
    pub db: Option<jackioh_server::db::store::Db>,
}

impl Default for TestAppOptions {
    fn default() -> TestAppOptions {
        TestAppOptions {
            e2e: true,
            env: Vec::new(),
            catalog: None,
            skip_fixtures: false,
            deletion: None,
            db: None,
        }
    }
}

/// SURFACE §11.2: the fake store, the fixture auth provider, the E2E fixtures (R144's reseed).
pub async fn test_app() -> Arc<App> {
    test_app_with(TestAppOptions::default()).await
}

/// TS `createTestDeps(overrides)`: the test app with the overrides `TestAppOptions` names.
pub async fn test_app_with(options: TestAppOptions) -> Arc<App> {
    let overrides: Vec<(&str, &str)> = options
        .env
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let mut env = test_env(&overrides);
    // Outside end-to-end mode `load_env` would demand Supabase and Postgres; the test app keeps the
    // fake store and the fixture auth either way, so the flag is the one thing that changes.
    env.e2e = options.e2e;
    let mut catalog = match options.catalog {
        Some(catalog) => catalog,
        None => load_catalog(LoadCatalogOptions {
            version: Some(env.catalog_version.to_string()),
            json: None,
        })
        .await
        .unwrap_or_else(|error| panic!("the catalog: {error}")),
    };
    if catalog.commit.is_none() {
        catalog.commit = env.deployed_commit.clone();
    }

    let auth = E2eAuth::new();
    auth.set_deletion(options.deletion.unwrap_or(E2eDeletion::Deletes));

    let restarted = options.db.is_some();
    // TS `createMemoryStore({ now: () => timers.now(), … })`: the store stamps rows (and counts §9.4's
    // attempt windows) on the test's clock, which a paused tokio moves, not on the wall clock.
    let db = options.db.unwrap_or_else(|| {
        use jackioh_server::db::fake::{FakeData, default_redemption_settings};
        let data = FakeData::new(Arc::new(app::now_ms), default_redemption_settings(), None);
        jackioh_server::db::store::Db::Fake(Arc::new(tokio::sync::Mutex::new(data)))
    });
    let app = Arc::new(App {
        env,
        db,
        auth: Auth::E2e(auth),
        matches: jackioh_server::actor::registry::Registry::new(),
        limiter: jackioh_server::api::http::create_rate_limiter(
            jackioh_server::config::API_REQUESTS_PER_MINUTE as _,
            API_RATE_WINDOW_MS as _,
        ),
        known_profiles: jackioh_server::api::http::KnownProfiles::default(),
        sign_in_limiter: jackioh_server::api::http::create_rate_limiter(
            jackioh_server::config::USERNAME_SIGN_IN_ATTEMPTS_PER_WINDOW,
            jackioh_server::config::USERNAME_SIGN_IN_WINDOW_MS,
        ),
        catalog,
        breaker: Mutex::new(jackioh_server::api::codes::create_breaker_state()),
        replays: jackioh_server::api::replays::Replays::default(),
        card_stats: jackioh_server::api::stats::KeptReports::default(),
        queue_presence: jackioh_server::api::queue::QueuePresence::new(),
        queue_readout: jackioh_server::api::queue::KeptReadout::default(),
        rematch_offers: jackioh_server::api::rematch::RematchOffers::default(),
        duos_again_offers: jackioh_server::duos::again::DuosAgainOffers::default(),
        seats: jackioh_server::seats::Seats::default(),
        social: jackioh_server::social::Social::default(),
        engine_thread: test_engine_thread,
        bug_reports: jackioh_server::bug_reports::BugReports::recording(),
        ci_runs: jackioh_server::api::ci_runs::CiRuns::canned(),
        economy_live: jackioh_server::config::ECONOMY_LIVE,
        practice_verifications: tokio::sync::Semaphore::new(
            jackioh_server::config::PRACTICE_VERIFY_CONCURRENCY,
        ),
        practice_settles: tokio::sync::watch::Sender::new(0),
        spends: jackioh_server::economy::spend_slots::SpendSlots::default(),
    });
    if options.e2e && !options.skip_fixtures && !restarted {
        jackioh_server::api::e2e::seed_e2e_fixtures(&app)
            .await
            .unwrap_or_else(|error| panic!("R144's reseed: {error:?}"));
    }
    app
}

/// #1035, R767000: sets calling thread's registered cards on blocking pool thread.
fn test_engine_thread() -> Box<dyn FnOnce() + Send> {
    let overrides = jackioh_engine::testkit::thread_overrides();
    Box::new(move || {
        jackioh_cards::register_all();
        overrides.install();
    })
}
