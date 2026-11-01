//! Every route that writes under an account's economy lock for a press takes the account's turn
//! first (SPEC §14.11, R2378000, issue #2610): `PUT /api/card-picks`, `POST /api/card-picks/upload`,
//! `POST /api/practice/seeds`, `DELETE /api/practice/seeds/:seed` and
//! `POST /api/quests/:questId/replace`. The tests hold the turns with claims of their own, as
//! `tests/api/tokens.rs` does. A fake transaction holds the fake store's mutex, so a free mutex
//! while a request waits shows it waits holding no transaction.

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::watch;

use jackioh_server::app::App;
use jackioh_server::config::{
    ECONOMY_SPENDS_RUNNING_MAX, ECONOMY_SPENDS_WAITING_MAX, SPEND_BUSY_RETRY_AFTER_MS,
};
use jackioh_server::economy::spend_slots::SPEND_BUSY_MESSAGE;

use crate::support::deps::{call, test_app as built_app};
use crate::support::store::{fake_lock, fake_ref};

/// The caller's bearer token and managed-auth user id (`e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";
/// How many times `until_claims` lets the spawned requests run before it gives up.
const YIELDS: usize = 1_000;

/// The test app with the economy on (R1302006), and the caller's profile id.
async fn setup() -> (Arc<App>, String) {
    let mut app = built_app().await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .economy_live = true;
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_get_by_user_id(USER)
        .await
        .expect("profiles.getByUserId")
        .expect("e2e-p1's profile");
    tx.commit().await.expect("commit");
    (app, profile.id)
}

/// `POST /api/practice/seeds`' body: an Easy random-deck game on this build's catalog.
fn seed_request() -> Value {
    json!({
        "difficulty": "easy", "humanSeat": "p2", "deck": { "kind": "random", "lean": null },
        "catalog": jackioh_cards::catalog_version(),
    })
}

/// A practice seed issued to the caller through the route.
async fn issue_seed(app: &Arc<App>) -> String {
    let (status, _, body) = call(app, "POST", "/api/practice/seeds", Some(TOKEN), seed_request()).await;
    assert_eq!(status, 201, "{body}");
    body["seed"].as_str().expect("a seed").to_string()
}

/// The five routes: method, path, body and the status each answers once it runs.
fn routes(seed: &str) -> Vec<(&'static str, String, Value, u16)> {
    vec![
        (
            "PUT",
            "/api/card-picks".into(),
            json!({ "cardId": "core-008", "fullArt": false }),
            200,
        ),
        (
            "POST",
            "/api/card-picks/upload".into(),
            json!({ "cards": [{ "cardId": "core-002", "fullArt": false }] }),
            200,
        ),
        ("POST", "/api/practice/seeds".into(), seed_request(), 201),
        ("DELETE", format!("/api/practice/seeds/{seed}"), Value::Null, 200),
        // No pool entry has this id, so the replace answers 404 whatever the top-up drew.
        (
            "POST",
            "/api/quests/no-such-quest/replace".into(),
            Value::Null,
            404,
        ),
    ]
}

/// Lets the spawned requests run until `profile_id` has `claims` requests running or waiting.
async fn until_claims(app: &App, profile_id: &str, claims: usize) {
    for _ in 0..YIELDS {
        if app.spends.claims(profile_id) == claims {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!(
        "{profile_id} has {} requests, not {claims}",
        app.spends.claims(profile_id)
    );
}

/// `count` turns of `profile_id`'s own, running or waiting, each held until the sender this
/// answers is dropped.
async fn hold_turns(app: &Arc<App>, profile_id: &str, count: usize) -> watch::Sender<()> {
    let (release, held) = watch::channel(());
    for _ in 0..count {
        let (app, mut held, profile_id) = (Arc::clone(app), held.clone(), profile_id.to_owned());
        tokio::spawn(async move {
            let _turn = app.spends.claim(&profile_id).await;
            // Errs once the sender is dropped.
            held.changed().await.ok();
        });
    }
    until_claims(app, profile_id, count).await;
    release
}

/// As many requests as an account runs and keeps waiting at once.
fn full() -> usize {
    ECONOMY_SPENDS_RUNNING_MAX + ECONOMY_SPENDS_WAITING_MAX
}

/// The refusal of a request sent past the waiting ones.
fn assert_busy(what: &str, status: u16, body: &Value) {
    assert_eq!(status, 429, "{what}: {body}");
    assert_eq!(body["error"]["code"], json!("rate_limited"), "{what}: {body}");
    assert_eq!(
        body["error"]["message"],
        json!(SPEND_BUSY_MESSAGE),
        "{what}: {body}"
    );
    assert_eq!(
        body["error"]["details"]["retryAfterMs"],
        json!(SPEND_BUSY_RETRY_AFTER_MS),
        "{what}: {body}"
    );
}

#[tokio::test]
async fn r2378000_each_route_on_the_economy_lock_waits_its_turn_holding_no_transaction_then_runs() {
    let (app, p1) = setup().await;
    let seed = issue_seed(&app).await;

    // One route at a time: all five at once would overfill `ECONOMY_SPENDS_WAITING_MAX`.
    for (method, path, body, expected) in routes(&seed) {
        let release = hold_turns(&app, &p1, ECONOMY_SPENDS_RUNNING_MAX).await;
        let request = {
            let (app, path, body) = (Arc::clone(&app), path.clone(), body);
            tokio::spawn(async move { call(&app, method, &path, Some(TOKEN), body).await })
        };
        until_claims(&app, &p1, ECONOMY_SPENDS_RUNNING_MAX + 1).await;
        assert!(!request.is_finished(), "{method} {path} waits its turn");
        assert!(
            fake_ref(&app).try_lock().is_ok(),
            "{method} {path} waits holding no transaction"
        );

        drop(release);
        let (status, _, answer) = request.await.expect("the request's task");
        assert_eq!(status, expected, "{method} {path}: {answer}");
        until_claims(&app, &p1, 0).await;
    }
}

#[tokio::test]
async fn r2378000_each_route_on_the_economy_lock_is_refused_past_the_waiting_turns_and_writes_nothing() {
    let (app, p1) = setup().await;
    let seed = issue_seed(&app).await;
    let before = fake_lock(&app).await.tables.economy.clone();
    let release = hold_turns(&app, &p1, full()).await;

    for (method, path, body, _) in routes(&seed) {
        let (status, _, answer) = call(&app, method, &path, Some(TOKEN), body).await;
        assert_busy(&format!("{method} {path}"), status, &answer);
    }
    assert_eq!(
        fake_lock(&app).await.tables.economy,
        before,
        "a refused request writes nothing"
    );
    assert_eq!(app.spends.claims(&p1), full(), "and takes no place");

    drop(release);
    until_claims(&app, &p1, 0).await;
    let (status, _, answer) = call(
        &app,
        "DELETE",
        &format!("/api/practice/seeds/{seed}"),
        Some(TOKEN),
        Value::Null,
    )
    .await;
    assert_eq!(
        status, 200,
        "the seed the refused give-back left behind: {answer}"
    );
}
