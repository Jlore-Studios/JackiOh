//! The deploy probes (`src/api/health.rs`, #861): `GET /healthz` answers 200 with the catalog
//! version and `x-deployed-commit` and never touches the store; `GET /readyz` gives the same
//! answer, but only once the store answers `select 1`, and until then 503 `unavailable`.

use serde_json::Value;

use jackioh_server::api::catalog::DEPLOYED_COMMIT_HEADER;
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app;
use jackioh_server::db::pg::{PostgresStoreOptions, create_postgres_store};
use jackioh_server::db::store::Db;

use crate::support::deps::{TestAppOptions, call, test_app_with};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn with_commit(db: Option<Db>) -> TestAppOptions {
    TestAppOptions {
        env: vec![("RENDER_GIT_COMMIT".into(), COMMIT.into())],
        db,
        ..TestAppOptions::default()
    }
}

/// A Postgres pool that can never answer: its address held no listener from the moment the port
/// was taken.
fn unreachable_store() -> Db {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port for the refused address");
    let port = listener.local_addr().expect("the bound address").port();
    drop(listener);
    Db::Pg(
        create_postgres_store(&PostgresStoreOptions {
            connection_string: format!("postgres://jackioh:unused@127.0.0.1:{port}/jackioh"),
            max: None,
        })
        .expect("a pool over a refused address builds"),
    )
}

#[tokio::test]
async fn healthz_answers_200_with_the_catalog_version_and_the_deployed_commit() {
    let app = test_app_with(with_commit(None)).await;
    let (status, headers, body) = call(&app, "GET", "/healthz", None, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], Value::String("ok".to_string()));
    assert_eq!(
        body["version"],
        Value::String(jackioh_cards::catalog_version().to_string())
    );
    assert_eq!(
        headers
            .get(DEPLOYED_COMMIT_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        COMMIT
    );
}

#[tokio::test]
async fn healthz_answers_200_with_the_database_down() {
    let app = test_app_with(with_commit(Some(unreachable_store()))).await;
    let (status, _, _) = call(&app, "GET", "/healthz", None, Value::Null).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn readyz_answers_200_once_the_store_answers() {
    let app = test_app_with(with_commit(None)).await;
    let (status, headers, body) = call(&app, "GET", "/readyz", None, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], Value::String("ok".to_string()));
    assert_eq!(
        body["version"],
        Value::String(jackioh_cards::catalog_version().to_string())
    );
    assert_eq!(
        headers
            .get(DEPLOYED_COMMIT_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        COMMIT
    );
}

#[tokio::test(start_paused = true)]
async fn readyz_answers_503_while_the_database_does_not() {
    let app = test_app_with(with_commit(Some(unreachable_store()))).await;
    let (status, _, body) = call(&app, "GET", "/readyz", None, Value::Null).await;
    assert_eq!(status, 503);
    assert_eq!(body["error"]["code"], Value::String("unavailable".to_string()));
}

#[test]
fn both_probes_are_open_get_routes() {
    for path in ["/healthz", "/readyz"] {
        let route = app::ROUTES
            .iter()
            .find(|route| route.0 == "GET" && route.1 == path)
            .unwrap_or_else(|| panic!("ROUTES holds GET {path}"));
        assert_eq!(route.2, AuthLevel::None);
    }
}

#[test]
fn render_yaml_s_health_check_is_the_readiness_probe() {
    let render = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../render.yaml"))
        .expect("render.yaml reads");
    assert!(
        render
            .lines()
            .any(|line| line.trim() == "healthCheckPath: /readyz"),
        "render.yaml's healthCheckPath is /readyz"
    );
}
