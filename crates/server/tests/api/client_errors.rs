//! `POST /api/client-errors` (#3619): a crash in a player's browser, raised as the `client.error`
//! alert. The limits are per address and per account, the body is capped, and nothing but five known
//! fields reaches the log.

use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::api::http::AuthLevel;
use jackioh_server::app;
use jackioh_server::config::{
    CLIENT_ERROR_BODY_MAX_BYTES, CLIENT_ERROR_MESSAGE_MAX_CHARS, CLIENT_ERROR_REPORTS_PER_ACCOUNT,
    CLIENT_ERROR_REPORTS_PER_ADDRESS, CLIENT_ERROR_WINDOW_MS,
};

use crate::support::deps::{DEFAULT_TEST_IP, json_request, record_logs, send, test_app};

const PATH: &str = "/api/client-errors";

fn report() -> Value {
    json!({
        "trigger": "error",
        "message": "TypeError: boom",
        "stack": "TypeError: boom\n    at f (https://play.jackioh.test/assets/index-abc.js:1:2)",
        "route": "/play",
        "catalogVersion": "0.6.4",
    })
}

fn ip(n: usize) -> String {
    format!("198.51.100.{n}")
}

#[tokio::test(start_paused = true)]
async fn a_report_raises_the_client_error_alert() {
    let app = test_app().await;
    let logs = record_logs();
    let mut body = report();
    body["hand"] = json!(["secret-card"]);
    let (status, _, answer) = send(&app, json_request("POST", PATH, &body, None, None)).await;
    assert_eq!(status, 200);
    assert_eq!(answer, json!({ "reported": true }));

    let lines = logs.named("client.error");
    assert_eq!(lines.len(), 1, "one alert");
    assert_eq!(lines[0].level, "alert");
    assert_eq!(
        lines[0].data["data"],
        report(),
        "five known fields, the extra one dropped"
    );
    let everything =
        serde_json::to_string(&logs.entries().iter().map(|e| e.data.clone()).collect::<Vec<_>>())
            .expect("json");
    assert!(!everything.contains("secret-card"), "{everything}");
    assert!(!everything.contains(DEFAULT_TEST_IP), "{everything}");
}

#[tokio::test(start_paused = true)]
async fn long_fields_are_cut_and_a_huge_body_is_refused() {
    let app = test_app().await;
    let logs = record_logs();
    let mut body = report();
    body["message"] = json!("m".repeat(5_000));
    let (status, _, _) = send(&app, json_request("POST", PATH, &body, None, None)).await;
    assert_eq!(status, 200);
    let lines = logs.named("client.error");
    let kept = lines[0].data["data"]["message"].as_str().expect("message");
    assert_eq!(kept.chars().count(), CLIENT_ERROR_MESSAGE_MAX_CHARS);

    let mut huge = report();
    huge["stack"] = json!("s".repeat(CLIENT_ERROR_BODY_MAX_BYTES + 1));
    let (status, _, _) = send(&app, json_request("POST", PATH, &huge, None, None)).await;
    assert_eq!(status, 400);
    assert_eq!(
        logs.named("client.error").len(),
        1,
        "the huge body raised nothing"
    );
}

#[tokio::test(start_paused = true)]
async fn a_bad_trigger_or_a_missing_message_or_stack_is_400() {
    let app = test_app().await;
    let logs = record_logs();
    for (field, value) in [
        ("trigger", json!("nonsense")),
        ("trigger", Value::Null),
        ("message", Value::Null),
        ("stack", Value::Null),
    ] {
        let mut body = report();
        body[field] = value;
        let (status, _, _) = send(&app, json_request("POST", PATH, &body, None, None)).await;
        assert_eq!(status, 400, "{field}");
    }
    assert!(logs.named("client.error").is_empty());
}

#[tokio::test(start_paused = true)]
async fn one_address_is_limited_per_window() {
    let app = test_app().await;
    for n in 0..CLIENT_ERROR_REPORTS_PER_ADDRESS {
        let (status, _, _) = send(
            &app,
            json_request("POST", PATH, &report(), None, Some("203.0.113.50")),
        )
        .await;
        assert_eq!(status, 200, "report {n}");
    }
    let (status, _, answer) = send(
        &app,
        json_request("POST", PATH, &report(), None, Some("203.0.113.50")),
    )
    .await;
    assert_eq!(status, 429);
    assert!(answer["error"]["details"]["retryAfterMs"].as_i64().expect("wait") > 0);

    let (status, _, _) = send(
        &app,
        json_request("POST", PATH, &report(), None, Some("203.0.113.51")),
    )
    .await;
    assert_eq!(status, 200, "another address");

    tokio::time::advance(Duration::from_millis(CLIENT_ERROR_WINDOW_MS as u64)).await;
    let (status, _, _) = send(
        &app,
        json_request("POST", PATH, &report(), None, Some("203.0.113.50")),
    )
    .await;
    assert_eq!(status, 200, "a window later");
}

#[tokio::test(start_paused = true)]
async fn one_account_is_limited_across_addresses() {
    let app = test_app().await;
    for n in 0..CLIENT_ERROR_REPORTS_PER_ACCOUNT {
        let (status, _, _) = send(
            &app,
            json_request("POST", PATH, &report(), Some("e2e-token-p1"), Some(&ip(n))),
        )
        .await;
        assert_eq!(status, 200, "report {n}");
    }
    let (status, _, _) = send(
        &app,
        json_request("POST", PATH, &report(), Some("e2e-token-p1"), Some(&ip(200))),
    )
    .await;
    assert_eq!(status, 429);
    let (status, _, _) = send(&app, json_request("POST", PATH, &report(), None, Some(&ip(201)))).await;
    assert_eq!(status, 200, "a signed-out visitor is not that account");
}

#[tokio::test(start_paused = true)]
async fn an_unverifiable_token_reports_as_anonymous() {
    let app = test_app().await;
    let (status, _, _) = send(
        &app,
        json_request("POST", PATH, &report(), Some("nonsense"), None),
    )
    .await;
    assert_eq!(status, 200);
}

#[test]
fn the_route_is_a_post_open_to_anyone() {
    assert!(app::ROUTES.iter().any(|(method, path, auth, _)| *method == "POST"
        && *path == PATH
        && matches!(auth, AuthLevel::None)));
}
