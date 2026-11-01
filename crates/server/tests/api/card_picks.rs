//! The picks an account makes for one card of its own (`src/api/card_picks.rs`, SPEC §14.2,
//! R1834000, R1834001).
//!
//! The questions here are the routes' own: that they are three active routes, that each reads and
//! writes the caller's own picks and nobody else's, that a `PUT` changes only the keys it names and
//! refuses an id of another kind or one the account does not own, and that the device's one-time
//! upload never overwrites a card the account holds and keeps an id this build does not know. What
//! the store does with the rows is `tests/store/contract.rs`'s; RLS is the SQL suites'.
//!
//! Every request goes through the whole app (`support::deps::test_app`, the E2E fixtures): the
//! caller is `e2e-p1`, the other account `e2e-p2` and the pending one `e2e-pending`.

use std::sync::Arc;

use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::db::fake::OnCall;
use jackioh_server::economy::store::{CosmeticGrant, EconomyReason};
use serde_json::{Value, json};

use crate::support::deps::{call, test_app};
use crate::support::store::fake;

/// The caller's bearer token and managed-auth user id (`e2e/support/config.ts`'s `e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";
/// The other active account (`e2e-p2`).
const OTHER_TOKEN: &str = "e2e-token-p2";
/// The pending account (`e2e-pending`).
const PENDING_TOKEN: &str = "e2e-token-pending";

/// A card with a full art (`cosmetics.json`'s `full-art-mr-vanilla`), and one with none.
const CARD: &str = "core-008";
const FULL_ART: &str = "full-art-mr-vanilla";
const PLAIN_CARD: &str = "core-002";

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

/// Grants `profile_id` the cosmetic `cosmetic_id` on `card_id`, as a dev grant.
async fn own(app: &App, profile_id: &str, card_id: &str, cosmetic_id: &str) {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("a store transaction");
    tx.economy_cosmetics_grant(&CosmeticGrant {
        profile_id: profile_id.to_owned(),
        card_id: Some(card_id.to_owned()),
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
}

async fn get(app: &Arc<App>, bearer: Option<&str>) -> (u16, Value) {
    let (status, _, body) = call(app, "GET", "/api/card-picks", bearer, Value::Null).await;
    (status, body)
}

async fn put(app: &Arc<App>, bearer: &str, body: Value) -> (u16, Value) {
    let (status, _, body) = call(app, "PUT", "/api/card-picks", Some(bearer), body).await;
    (status, body)
}

async fn upload(app: &Arc<App>, bearer: &str, cards: Value) -> (u16, Value) {
    let (status, _, body) = call(
        app,
        "POST",
        "/api/card-picks/upload",
        Some(bearer),
        json!({ "cards": cards }),
    )
    .await;
    (status, body)
}

/// A refused write's message, after checking it is a 400 `bad_request`.
#[track_caller]
fn refusal((status, body): (u16, Value)) -> String {
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_request", "{body}");
    body["error"]["message"].as_str().unwrap_or_default().to_owned()
}

/// Every store method the fake is asked to run from here on, by name, in order
/// (`tests/api/friends.rs`'s).
async fn record_store_calls(app: &App) -> Arc<std::sync::Mutex<Vec<String>>> {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&calls);
    let hook: OnCall = Arc::new(move |method: &str| {
        sink.lock().expect("the call log").push(method.to_string());
        Ok(())
    });
    fake(app).lock().await.on_call = Some(hook);
    calls
}

/// The economy's and the collection's methods among `calls`, in order: what a handler reads and
/// writes of the account's holdings (resolving the caller reads only `profiles.`).
fn economy_calls(calls: &std::sync::Mutex<Vec<String>>) -> Vec<String> {
    calls
        .lock()
        .expect("the call log")
        .iter()
        .filter(|method| method.starts_with("economy.") || method.starts_with("collection."))
        .cloned()
        .collect()
}

#[tokio::test]
async fn r1834000_routes_are_active_only() {
    let routes: Vec<(String, bool)> = app::ROUTES
        .iter()
        .filter(|(_, path, _, _)| path.starts_with("/api/card-picks"))
        .map(|(method, path, auth, _)| (format!("{method} {path}"), matches!(auth, AuthLevel::Active)))
        .collect();
    assert_eq!(
        routes,
        vec![
            ("GET /api/card-picks".to_owned(), true),
            ("PUT /api/card-picks".to_owned(), true),
            ("POST /api/card-picks/upload".to_owned(), true),
        ]
    );

    let app = test_app().await;
    assert_eq!(get(&app, None).await.0, 401);
    assert_eq!(get(&app, Some(PENDING_TOKEN)).await.0, 403);
    assert_eq!(
        put(
            &app,
            PENDING_TOKEN,
            json!({ "cardId": CARD, "foil": "foil-rainbow" })
        )
        .await
        .0,
        403
    );
    assert_eq!(upload(&app, PENDING_TOKEN, json!([])).await.0, 403);
    // An active account that picked nothing reads an empty list.
    assert_eq!(get(&app, Some(TOKEN)).await, (200, json!({ "cards": [] })));
}

#[tokio::test]
async fn r1834000_a_put_changes_only_the_callers_card_and_only_the_keys_it_names() {
    let app = test_app().await;
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": CARD, "foil": "foil-rainbow", "border": "border-wood" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": CARD, "foil": "foil-rainbow", "border": "border-wood" }] })
    );
    // A key left out keeps its pick; `null` clears one.
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": CARD, "finish": "finish-linen", "border": null }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": CARD, "foil": "foil-rainbow", "finish": "finish-linen" }] })
    );
    // The other account reads its own, none of the caller's.
    assert_eq!(get(&app, Some(OTHER_TOKEN)).await, (200, json!({ "cards": [] })));
    // Clearing every pick drops the card.
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": CARD, "foil": null, "finish": null }),
    )
    .await;
    assert_eq!((status, body), (200, json!({ "cards": [] })));
}

#[tokio::test]
async fn r1834001_refuses_another_kind_and_an_unowned_full_art() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    // An id of another kind, one the registry does not know, and a full art in a foil's slot.
    for (body, message) in [
        (
            json!({ "cardId": CARD, "foil": "border-wood" }),
            "\"foil\" is not a cosmetic id of its kind",
        ),
        (
            json!({ "cardId": CARD, "finish": "finish-from-the-future" }),
            "\"finish\" is not a cosmetic id of its kind",
        ),
        (
            json!({ "cardId": CARD, "border": FULL_ART }),
            "\"border\" is not a cosmetic id of its kind",
        ),
        // A card with no full art has none to print.
        (
            json!({ "cardId": PLAIN_CARD, "fullArt": true }),
            "\"fullArt\" is not a cosmetic id of its kind",
        ),
        // A full art the account does not own.
        (
            json!({ "cardId": CARD, "fullArt": true }),
            "\"fullArt\" is a cosmetic this account does not own",
        ),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
    }
    // Nothing of a refused write is kept, its other keys included.
    let both = json!({ "cardId": CARD, "foil": "foil-rainbow", "fullArt": true });
    refusal(put(&app, TOKEN, both.clone()).await);
    assert_eq!(get(&app, Some(TOKEN)).await, (200, json!({ "cards": [] })));
    // A card that is not in the catalog, and a shape that is not an id, are refused too.
    refusal(
        put(
            &app,
            TOKEN,
            json!({ "cardId": "core-999", "foil": "foil-rainbow" }),
        )
        .await,
    );
    refusal(put(&app, TOKEN, json!({ "cardId": CARD, "foil": 7 })).await);

    // Owned, the same full art is taken, and a starter is owned by every account with no ledger row.
    own(&app, &profile, CARD, FULL_ART).await;
    let (status, body) = put(&app, TOKEN, both).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": CARD, "foil": "foil-rainbow", "fullArt": true }] })
    );
    // Switching a full art off needs no ownership.
    let (status, body) = put(&app, TOKEN, json!({ "cardId": PLAIN_CARD, "fullArt": false })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["cards"][0],
        json!({ "cardId": PLAIN_CARD, "fullArt": false })
    );

    // R1833000's foils are no starters: one owned on another card is not owned on this one, and
    // owned here it is taken.
    own(&app, &profile, PLAIN_CARD, "foil-glitter").await;
    let glitter = json!({ "cardId": CARD, "foil": "foil-glitter" });
    assert_eq!(
        refusal(put(&app, TOKEN, glitter.clone()).await),
        "\"foil\" is a cosmetic this account does not own"
    );
    own(&app, &profile, CARD, "foil-glitter").await;
    let (status, body) = put(&app, TOKEN, glitter).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["cards"],
        json!([
            { "cardId": PLAIN_CARD, "fullArt": false },
            { "cardId": CARD, "foil": "foil-glitter", "fullArt": true },
        ])
    );
}

#[tokio::test]
async fn r1834000_upload_keeps_an_unknown_id_and_never_overwrites() {
    let app = test_app().await;
    // The account already holds a pick for the card, written by a newer device.
    let (status, _) = put(&app, TOKEN, json!({ "cardId": CARD, "foil": "foil-cracked-ice" })).await;
    assert_eq!(status, 200);

    let (status, body) = upload(
        &app,
        TOKEN,
        json!([
            // Held already: never touched.
            { "cardId": CARD, "foil": "foil-rainbow", "border": "border-bone" },
            // An id this build does not know is kept, as the device kept it; one of another kind
            // is left out, since it could never be worn.
            { "cardId": PLAIN_CARD, "foil": "foil-from-a-newer-client", "finish": "border-wood" },
            // A card the catalog does not hold is passed over.
            { "cardId": "core-999", "foil": "foil-rainbow" },
            // A full art the account does not own is left out, and with it the whole entry.
            { "cardId": "core-004", "fullArt": true },
        ]),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let expected = json!({ "cards": [
        { "cardId": PLAIN_CARD, "foil": "foil-from-a-newer-client" },
        { "cardId": CARD, "foil": "foil-cracked-ice" },
    ] });
    assert_eq!(body, expected);
    assert_eq!(get(&app, Some(TOKEN)).await, (200, expected.clone()));
    // A second upload changes nothing either.
    let (status, body) = upload(
        &app,
        TOKEN,
        json!([{ "cardId": PLAIN_CARD, "foil": "foil-rainbow" }]),
    )
    .await;
    assert_eq!((status, body), (200, expected));
    // The other account's picks are its own.
    assert_eq!(get(&app, Some(OTHER_TOKEN)).await, (200, json!({ "cards": [] })));
}

#[tokio::test]
async fn r1834000_an_upload_is_checked_for_shape_before_anything_is_stored() {
    let app = test_app().await;
    for cards in [
        json!("core-008"),
        json!([{ "foil": "foil-rainbow" }]),
        json!([{ "cardId": "Core-008" }]),
        json!([{ "cardId": CARD, "foil": 1 }]),
        json!([{ "cardId": CARD, "fullArt": "yes" }]),
        json!([{ "cardId": CARD, "foil": "x".repeat(41) }]),
    ] {
        let (status, _, body) = call(
            &app,
            "POST",
            "/api/card-picks/upload",
            Some(TOKEN),
            json!({ "cards": cards }),
        )
        .await;
        assert_eq!(status, 400, "{cards}: {body}");
    }
    // More entries than the catalog holds cards.
    let many: Vec<Value> = (0..=app.catalog.card_ids.len())
        .map(|_| json!({ "cardId": CARD, "foil": "foil-rainbow" }))
        .collect();
    assert_eq!(upload(&app, TOKEN, Value::Array(many)).await.0, 400);
    assert_eq!(get(&app, Some(TOKEN)).await, (200, json!({ "cards": [] })));
}

#[tokio::test]
async fn a_put_and_an_upload_each_take_the_accounts_economy_lock_before_they_read() {
    let app = test_app().await;

    let calls = record_store_calls(&app).await;
    let (status, body) = put(&app, TOKEN, json!({ "cardId": CARD, "foil": "foil-rainbow" })).await;
    fake(&app).lock().await.on_call = None;
    assert_eq!(status, 200, "{body}");
    let asked = economy_calls(&calls);
    assert_eq!(
        asked.first().map(String::as_str),
        Some("economy.lock"),
        "{asked:?}"
    );
    assert!(
        asked.iter().any(|method| method == "economy.cardPicks.put"),
        "{asked:?}"
    );

    let calls = record_store_calls(&app).await;
    let (status, body) = upload(
        &app,
        TOKEN,
        json!([{ "cardId": PLAIN_CARD, "border": "border-wood" }]),
    )
    .await;
    fake(&app).lock().await.on_call = None;
    assert_eq!(status, 200, "{body}");
    let asked = economy_calls(&calls);
    assert_eq!(
        asked.first().map(String::as_str),
        Some("economy.lock"),
        "{asked:?}"
    );
    assert!(
        asked.iter().any(|method| method == "economy.cardPicks.upload"),
        "{asked:?}"
    );
}

#[tokio::test]
async fn r2655000_a_pick_of_a_full_art_tier_needs_that_tier_owned_on_its_card() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let signed = "full-art-mr-vanilla-signed";
    // Not owned, another card's tier, and a text that is no full art's id.
    for (body, message) in [
        (
            json!({ "cardId": CARD, "fullArt": signed }),
            "\"fullArt\" is a cosmetic this account does not own",
        ),
        (
            json!({ "cardId": CARD, "fullArt": "full-art-gary-the-gambler-signed" }),
            "\"fullArt\" is not a cosmetic id of its kind",
        ),
        (
            json!({ "cardId": CARD, "fullArt": "foil-rainbow" }),
            "\"fullArt\" must be a boolean or a full-art id or null",
        ),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
    }
    // Owning the Epic owns no other tier, and owning the Mythic owns neither the Epic nor `true`.
    own(&app, &profile, CARD, FULL_ART).await;
    assert_eq!(
        refusal(put(&app, TOKEN, json!({ "cardId": CARD, "fullArt": signed })).await),
        "\"fullArt\" is a cosmetic this account does not own"
    );
    own(&app, &profile, CARD, signed).await;
    let (status, body) = put(&app, TOKEN, json!({ "cardId": CARD, "fullArt": signed })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({ "cards": [{ "cardId": CARD, "fullArt": signed }] }));
    assert_eq!(get(&app, Some(TOKEN)).await, (200, body));
    // `true` is the Epic still, and an upload keeps an owned tier's id.
    let (status, body) = put(&app, TOKEN, json!({ "cardId": CARD, "fullArt": true })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({ "cards": [{ "cardId": CARD, "fullArt": true }] }));
    own(&app, &profile, "core-004", "full-art-gary-the-gambler-animated").await;
    let (status, body) = upload(
        &app,
        TOKEN,
        json!([
            { "cardId": "core-004", "fullArt": "full-art-gary-the-gambler-animated" },
            // Not owned: left out, and with it the whole entry.
            { "cardId": "core-011", "fullArt": "full-art-tempo-timmy-signed" },
        ]),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [
            { "cardId": "core-004", "fullArt": "full-art-gary-the-gambler-animated" },
            { "cardId": CARD, "fullArt": true },
        ] })
    );
}
