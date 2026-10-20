//! The cosmetics an account owns (`src/api/cosmetics.rs`, SPEC §14.2, R1313000).
//!
//! The questions here are the route's own: that it is one active `GET`, that it answers the
//! caller's own cosmetics and nobody else's, keyed on the verified token, and that a caller with no
//! token is refused. What the store holds is granted straight into it, as the ledger's tests in
//! `tests/store/contract.rs` do; no route writes a cosmetic yet.
//!
//! Every request goes through the whole app (`support::deps::test_app`, the E2E fixtures): the
//! caller is `e2e-p1`, the other account `e2e-p2` and the pending one `e2e-pending`.

use std::sync::Arc;

use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::economy::store::{CosmeticGrant, EconomyReason};
use serde_json::{Value, json};

use crate::support::deps::{call, test_app};

/// The caller's bearer token and managed-auth user id (`e2e/support/config.ts`'s `e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";
/// The other active account (`e2e-p2`).
const OTHER_TOKEN: &str = "e2e-token-p2";
const OTHER_USER: &str = "e2e-p2";
/// The pending account (`e2e-pending`).
const PENDING_TOKEN: &str = "e2e-token-pending";

async fn profile_id_of(app: &App, user_id: &str) -> String {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_get_by_user_id(user_id)
        .await
        .expect("profiles.getByUserId")
        .unwrap_or_else(|| panic!("the fixture account {user_id} has a profile"));
    tx.commit().await.expect("commit");
    profile.id
}

/// Grants `profile_id` the cosmetic `cosmetic_id`, on `card_id` or, when that is `None`, on the
/// account, as a dev grant.
async fn own(app: &App, profile_id: &str, card_id: Option<&str>, cosmetic_id: &str) {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("a store transaction");
    let granted = tx
        .economy_cosmetics_grant(&CosmeticGrant {
            profile_id: profile_id.to_owned(),
            card_id: card_id.map(str::to_owned),
            cosmetic_id: cosmetic_id.to_owned(),
            delta: 1,
            reason: EconomyReason::Dev,
            reference: None,
            dev: true,
            at: app::now_ms(),
        })
        .await
        .expect("economy.cosmetics.grant");
    tx.commit().await.expect("commit");
    assert!(granted, "{cosmetic_id} was not owned before");
}

async fn get(app: &Arc<App>, bearer: Option<&str>) -> (u16, Value) {
    let (status, _, body) = call(app, "GET", "/api/cosmetics", bearer, Value::Null).await;
    (status, body)
}

#[test]
fn r1313000_the_route_is_one_active_get() {
    let routes: Vec<(String, bool)> = app::ROUTES
        .iter()
        .filter(|(_, path, _, _)| path.starts_with("/api/cosmetics"))
        .map(|(method, path, auth, _)| (format!("{method} {path}"), matches!(auth, AuthLevel::Active)))
        .collect();
    assert_eq!(routes, vec![("GET /api/cosmetics".to_owned(), true)]);
}

#[tokio::test]
async fn r1313000_get_api_cosmetics_answers_the_callers_own_and_nobody_elses() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let other = profile_id_of(&app, OTHER_USER).await;
    own(&app, &profile, Some("core-008"), "full-art-mr-vanilla").await;
    own(&app, &profile, None, "table-felt").await;
    own(&app, &other, Some("core-004"), "full-art-gary-the-gambler").await;
    own(&app, &other, None, "field-rain").await;

    assert_eq!(
        get(&app, Some(TOKEN)).await,
        (
            200,
            json!({ "cards": [{ "cardId": "core-008", "cosmeticId": "full-art-mr-vanilla" }], "account": ["table-felt"] })
        )
    );
    assert_eq!(
        get(&app, Some(OTHER_TOKEN)).await,
        (
            200,
            json!({ "cards": [{ "cardId": "core-004", "cosmeticId": "full-art-gary-the-gambler" }], "account": ["field-rain"] })
        )
    );
    // A query naming another profile changes nothing: the profile is the verified token's.
    let (status, _, body) = call(
        &app,
        "GET",
        &format!("/api/cosmetics?profileId={other}"),
        Some(TOKEN),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["cards"][0]["cosmeticId"], "full-art-mr-vanilla");
}

#[tokio::test]
async fn r1313000_get_api_cosmetics_refuses_a_signed_out_caller() {
    let app = test_app().await;
    let (status, body) = get(&app, None).await;
    assert_eq!(status, 401, "{body}");
    // An active account that owns nothing past the starter cosmetics reads two empty lists, and a
    // pending one is turned away like every active route turns it.
    assert_eq!(
        get(&app, Some(TOKEN)).await,
        (200, json!({ "cards": [], "account": [] }))
    );
    assert_eq!(get(&app, Some(PENDING_TOKEN)).await.0, 403);
}
