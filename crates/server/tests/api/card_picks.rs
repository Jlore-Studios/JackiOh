//! Card picks per account (`src/api/card_picks.rs`, SPEC §14.2, R1834000, R1834001).
//! Verifies account isolation, PUT partial updates, validation, and upload idempotence.

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

/// R3015005: what a `PUT` of a level the account does not own on the card answers.
const NOT_OWNED: &str = "\"fullArt\" is a cosmetic this account does not own";
/// R3015005: what a `PUT` of a level the card is not offered answers.
const NOT_OFFERED: &str = "\"fullArt\" is not offered on this card";
/// R3015005: what a `PUT` naming a retired full art in `fullArt` answers.
const RETIRED: &str = "that cosmetic is retired: \"fullArt\" names a level";
/// R3015005: what a `PUT` listing any augment answers.
const AUGMENTS: &str = "\"augments\" are retired: a card wears one level, named in \"fullArt\"";

#[tokio::test]
async fn r2655000_r3015005_a_retired_full_art_is_no_pick_and_true_needs_full_art_s_first_level() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let signed = "full-art-mr-vanilla-signed";
    // A retired full art's id, owned or not, its card's or another's, is no level; a text that is no id is no pick.
    for (body, message) in [
        (json!({ "cardId": CARD, "fullArt": signed }), RETIRED),
        (
            json!({ "cardId": CARD, "fullArt": "full-art-gary-the-gambler-signed" }),
            RETIRED,
        ),
        (
            json!({ "cardId": CARD, "fullArt": "foil-rainbow" }),
            "\"fullArt\" must be a boolean or a full-art id or null",
        ),
        (json!({ "cardId": CARD, "fullArt": true }), NOT_OWNED),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
    }
    // Owning the still Epic owns Full art's first level alone (R3015006): `true` and that level, never the next.
    own(&app, &profile, CARD, FULL_ART).await;
    assert_eq!(
        refusal(
            put(
                &app,
                TOKEN,
                json!({ "cardId": CARD, "fullArt": "version-full-art-animated" })
            )
            .await
        ),
        NOT_OWNED
    );
    assert_eq!(
        refusal(put(&app, TOKEN, json!({ "cardId": CARD, "fullArt": signed })).await),
        RETIRED
    );
    let (status, body) = put(&app, TOKEN, json!({ "cardId": CARD, "fullArt": true })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({ "cards": [{ "cardId": CARD, "fullArt": true }] }));
    // The Mythic owns every level of Full art's ladder.
    own(&app, &profile, CARD, signed).await;
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": CARD, "fullArt": "version-full-art-signed" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": CARD, "fullArt": "version-full-art-signed" }] })
    );
    assert_eq!(get(&app, Some(TOKEN)).await, (200, body));
    // An upload keeps an owned tier's id as the level it maps to.
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
            { "cardId": "core-004", "fullArt": "version-full-art-animated" },
            { "cardId": CARD, "fullArt": "version-full-art-signed" },
        ] })
    );
}

#[tokio::test]
async fn r2715000_r3015005_an_animated_art_is_no_pick_and_maps_to_the_default_animated_level() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let nothing = json!({ "cards": [] });
    // An Animated art's id, owned or not, its card's or another's, is no level: 400, and nothing is written; nor is
    // the Default variant's animated level beside the still Epic alone.
    own(&app, &profile, CARD, FULL_ART).await;
    own(&app, &profile, "core-004", "animated-gary-the-gambler").await;
    for (body, message) in [
        (
            json!({ "cardId": CARD, "fullArt": "animated-mr-vanilla" }),
            RETIRED,
        ),
        (
            json!({ "cardId": CARD, "fullArt": "animated-gary-the-gambler" }),
            RETIRED,
        ),
        (
            json!({ "cardId": CARD, "fullArt": "animated-Mr" }),
            "\"fullArt\" must be a boolean or a full-art id or null",
        ),
        (
            json!({ "cardId": CARD, "fullArt": "version-animated" }),
            NOT_OWNED,
        ),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
        assert_eq!(get(&app, Some(TOKEN)).await, (200, nothing.clone()), "{body}");
    }
    // Owned on its card, it owns that level, picked by its id and read back as written.
    own(&app, &profile, CARD, "animated-mr-vanilla").await;
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": CARD, "fullArt": "version-animated" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": CARD, "fullArt": "version-animated" }] })
    );
    assert_eq!(get(&app, Some(TOKEN)).await, (200, body));
    // `true` is Full art's first level, the still Epic's look, and an upload keeps an owned Animated art as the
    // level it maps to.
    let (status, body) = put(&app, TOKEN, json!({ "cardId": CARD, "fullArt": true })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({ "cards": [{ "cardId": CARD, "fullArt": true }] }));
    let (status, body) = upload(
        &app,
        TOKEN,
        json!([
            { "cardId": "core-004", "fullArt": "animated-gary-the-gambler" },
            // Not owned: left out, and with it the whole entry.
            { "cardId": "core-011", "fullArt": "animated-tempo-timmy" },
        ]),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [
            { "cardId": "core-004", "fullArt": "version-animated" },
            { "cardId": CARD, "fullArt": true },
        ] })
    );
}

/// R2725001: a card Full art's ladder is offered on that has no retired full art of its own: Carnivorous Cube, an
/// Epic Unit.
const OFFERED: &str = "core-022";
/// A card Full art's ladder is not offered on: Speciesist, a Common Unit (R2725001).
const BARE: &str = "core-002";

#[tokio::test]
async fn r2725001_a_pick_not_offered_is_400_and_writes_nothing() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let nothing = json!({ "cards": [] });
    // Owned on Speciesist by a grant no shop makes, Full art's levels are still not offered there.
    for id in [
        "version-full-art",
        "version-full-art-animated",
        "augment-animated",
    ] {
        own(&app, &profile, BARE, id).await;
    }
    let calls = record_store_calls(&app).await;
    for (body, message) in [
        (
            json!({ "cardId": BARE, "fullArt": "version-full-art" }),
            NOT_OFFERED,
        ),
        (
            json!({ "cardId": BARE, "fullArt": "version-full-art-animated" }),
            NOT_OFFERED,
        ),
        // Offered, but not owned there.
        (
            json!({ "cardId": OFFERED, "fullArt": "version-full-art" }),
            NOT_OWNED,
        ),
        // R3015005: no augment is picked any more, owned, offered or known or not.
        (
            json!({ "cardId": BARE, "augments": ["augment-animated"] }),
            AUGMENTS,
        ),
        (
            json!({ "cardId": BARE, "foil": "foil-rainbow", "augments": ["augment-signature"] }),
            AUGMENTS,
        ),
        (
            json!({ "cardId": OFFERED, "augments": ["augment-sparkles"] }),
            AUGMENTS,
        ),
        // A shape the keys never take.
        (
            json!({ "cardId": OFFERED, "augments": "augment-animated" }),
            "\"augments\" must be an array of augment ids or null",
        ),
        (
            json!({ "cardId": OFFERED, "augments": ["augment-animated", "augment-animated"] }),
            "\"augments\" must be an array of augment ids or null",
        ),
        (
            json!({ "cardId": OFFERED, "augments": ["foil-rainbow"] }),
            "\"augments\" must be an array of augment ids or null",
        ),
        (
            json!({ "cardId": OFFERED, "fullArt": "version-Full" }),
            "\"fullArt\" must be a boolean or a full-art id or null",
        ),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
        assert_eq!(get(&app, Some(TOKEN)).await, (200, nothing.clone()), "{body}");
    }
    assert!(
        economy_calls(&calls)
            .iter()
            .all(|call| call != "economy.cardPicks.put"),
        "{:?}",
        economy_calls(&calls)
    );
    // An upload leaves the unoffered and the unowned out, and every augment, and with them every entry that held
    // only those.
    let (status, body) = upload(
        &app,
        TOKEN,
        json!([
            { "cardId": BARE, "fullArt": "version-full-art", "augments": ["augment-animated"] },
            { "cardId": OFFERED, "augments": ["augment-signature", "augment-sparkles"] },
        ]),
    )
    .await;
    assert_eq!((status, body), (200, nothing));
}

#[tokio::test]
async fn r2725000_r3015005_an_owned_level_round_trips_and_reaches_the_other_seat() {
    use jackioh_server::cosmetics::registry;
    use jackioh_server::economy::store::{CardPicks, FullArtPick};
    use jackioh_server::loadout::{AccountPicks, freeze_loadout};
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    for id in [
        "version-full-art",
        "version-full-art-animated",
        "version-full-art-signed",
    ] {
        own(&app, &profile, OFFERED, id).await;
    }
    // A level, written and read back as written.
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": OFFERED, "fullArt": "version-full-art-signed" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let written = json!({ "cards": [{ "cardId": OFFERED, "fullArt": "version-full-art-signed" }] });
    assert_eq!(body, written);
    assert_eq!(get(&app, Some(TOKEN)).await, (200, written));
    // `false` is the normal face, the Default variant's first level.
    let (status, body) = put(&app, TOKEN, json!({ "cardId": OFFERED, "fullArt": false })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": OFFERED, "fullArt": false }] })
    );
    // R1834002, R1834004: what the match freezes is what the other seat, a watcher and a replay are shown: the
    // level, by its id, with no augment.
    let freeze = |owned: &jackioh_server::economy::store::OwnedCosmetics, picks: &[CardPicks]| {
        freeze_loadout(
            registry(),
            owned,
            &AccountPicks::default(),
            None,
            picks,
            &[OFFERED.to_string()],
        )
    };
    let (status, _) = put(
        &app,
        TOKEN,
        json!({ "cardId": OFFERED, "fullArt": "version-full-art-animated" }),
    )
    .await;
    assert_eq!(status, 200);
    let mut tx = app.db.begin(Some(&profile)).await.expect("a store transaction");
    let owned = tx
        .economy_cosmetics_get(&profile)
        .await
        .expect("economy.cosmetics.get");
    let picks = tx
        .economy_card_picks_get(&profile)
        .await
        .expect("economy.cardPicks.get");
    tx.commit().await.expect("commit");
    let loadout = freeze(&owned, &picks);
    assert_eq!(
        loadout.card(OFFERED),
        Some(&CardPicks {
            card_id: OFFERED.to_string(),
            full_art: Some(FullArtPick::Id("version-full-art-animated".to_string())),
            ..CardPicks::default()
        })
    );
    assert_eq!(
        serde_json::to_value(loadout.card(OFFERED)).expect("a frozen card serializes"),
        json!({ "cardId": OFFERED, "fullArt": "version-full-art-animated" })
    );
    // A pick written before #3015, the version with both augments, still reads back and freezes as the level it
    // drew; clearing its augments with null leaves the version alone.
    let before = CardPicks {
        card_id: OFFERED.to_string(),
        full_art: Some(FullArtPick::Id("version-full-art".to_string())),
        augments: vec!["augment-animated".to_string(), "augment-signature".to_string()],
        ..CardPicks::default()
    };
    let mut tx = app.db.begin(Some(&profile)).await.expect("a store transaction");
    tx.economy_card_picks_put(&profile, &before, app::now_ms())
        .await
        .expect("economy.cardPicks.put");
    tx.commit().await.expect("commit");
    assert_eq!(
        get(&app, Some(TOKEN)).await,
        (
            200,
            json!({ "cards": [{
                "cardId": OFFERED,
                "fullArt": "version-full-art",
                "augments": ["augment-animated", "augment-signature"],
            }] })
        )
    );
    assert_eq!(
        freeze(&owned, &[before])
            .card(OFFERED)
            .and_then(|card| card.full_art.clone()),
        Some(FullArtPick::Id("version-full-art-signed".to_string()))
    );
    let (status, body) = put(&app, TOKEN, json!({ "cardId": OFFERED, "augments": null })).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": OFFERED, "fullArt": "version-full-art" }] })
    );
}

#[tokio::test]
async fn r2949000_r3015005_a_pick_listing_an_augment_is_400_and_writes_nothing() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let nothing = json!({ "cards": [] });
    // Each of the three owned on its own, and offered there.
    for id in ["version-full-art", "augment-animated", "augment-signature"] {
        own(&app, &profile, OFFERED, id).await;
    }
    let calls = record_store_calls(&app).await;
    for body in [
        json!({ "cardId": OFFERED, "augments": ["augment-signature"] }),
        json!({ "cardId": OFFERED, "fullArt": "version-full-art", "augments": ["augment-signature"] }),
        json!({ "cardId": OFFERED, "fullArt": false, "augments": ["augment-animated", "augment-signature"] }),
        json!({ "cardId": OFFERED, "augments": ["augment-animated", "augment-signature"] }),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), AUGMENTS, "{body}");
        assert_eq!(get(&app, Some(TOKEN)).await, (200, nothing.clone()), "{body}");
    }
    assert!(
        economy_calls(&calls)
            .iter()
            .all(|call| call != "economy.cardPicks.put"),
        "{:?}",
        economy_calls(&calls)
    );
    // An empty list clears, so a level written with it passes, and the signed level is a level of its own, which
    // the account does not own.
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": OFFERED, "fullArt": "version-full-art", "augments": [] }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({ "cards": [{ "cardId": OFFERED, "fullArt": "version-full-art" }] })
    );
    assert_eq!(
        refusal(
            put(
                &app,
                TOKEN,
                json!({ "cardId": OFFERED, "fullArt": "version-full-art-signed" })
            )
            .await
        ),
        NOT_OWNED
    );
}

/// The checks every one of #2940's to #3017's parts runs on its versions (R3015005): each not offered on `bare`
/// (Speciesist but for part 6's) though owned there; `unowned` offered on `card` and not owned; `owned` offered and
/// owned there, and round-tripping, an augment beside it refused; `skipped`, a level of `owned`'s ladder above it,
/// not owned; and an upload of `upload` on `bare` left out.
async fn a_part_s_versions(
    bare: &str,
    versions: &[&str],
    card: &str,
    unowned: &str,
    owned: &[&str],
    skipped: &str,
    upload_id: &str,
) {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    let nothing = json!({ "cards": [] });
    for id in versions {
        own(&app, &profile, bare, id).await;
    }
    for id in owned.iter().chain(&["augment-animated", "augment-signature"]) {
        own(&app, &profile, card, id).await;
    }
    let calls = record_store_calls(&app).await;
    let refused = versions
        .iter()
        .map(|id| (json!({ "cardId": bare, "fullArt": id }), NOT_OFFERED))
        .chain([
            (json!({ "cardId": card, "fullArt": unowned }), NOT_OWNED),
            (json!({ "cardId": card, "fullArt": skipped }), NOT_OWNED),
            (
                json!({ "cardId": card, "fullArt": owned[0], "augments": ["augment-animated", "augment-signature"] }),
                AUGMENTS,
            ),
            (json!({ "cardId": card, "fullArt": owned[0], "augments": ["augment-signature"] }), AUGMENTS),
        ]);
    for (body, message) in refused {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
        assert_eq!(get(&app, Some(TOKEN)).await, (200, nothing.clone()), "{body}");
    }
    assert!(
        economy_calls(&calls)
            .iter()
            .all(|call| call != "economy.cardPicks.put"),
        "{:?}",
        economy_calls(&calls)
    );
    // Where it is offered and owned, each round-trips; a later augment is refused and leaves it as it was.
    let mut worn = nothing.clone();
    for id in owned {
        worn = json!({ "cards": [{ "cardId": card, "fullArt": id }] });
        let (status, body) = put(&app, TOKEN, json!({ "cardId": card, "fullArt": id })).await;
        assert_eq!((status, body), (200, worn.clone()), "{id}");
        assert_eq!(
            refusal(
                put(
                    &app,
                    TOKEN,
                    json!({ "cardId": card, "augments": ["augment-animated"] })
                )
                .await
            ),
            AUGMENTS,
            "{id}"
        );
        assert_eq!(get(&app, Some(TOKEN)).await, (200, worn.clone()), "{id}");
    }
    // An upload leaves an unoffered version out, and with it the entry that held only that.
    let (status, body) = upload(&app, TOKEN, json!([{ "cardId": bare, "fullArt": upload_id }])).await;
    assert_eq!((status, body), (200, worn));
}

/// R2940000: a card offered each of #2940's three versions: Transmogulate, a Legendary Spell.
const ALL_SIX: &str = "core-083";

#[tokio::test]
async fn r2940000_a_version_not_offered_is_400_and_writes_nothing() {
    // R3015000: Alt full art is Alt art's second level, its signed level above its animated one.
    a_part_s_versions(
        BARE,
        &["version-alt-art", "version-sketch", "version-alt-full-art"],
        ALL_SIX,
        "version-alt-art",
        &["version-alt-full-art", "version-sketch"],
        "version-alt-full-art-signed",
        "version-sketch",
    )
    .await;
}

/// R2942000: a card offered each of #2942's three versions: Scratch Pad, a Mythic Spell.
const ALL_THREE: &str = "meditativeplus-026";

#[tokio::test]
async fn r2942000_r2942002_a_version_not_offered_or_an_augment_beside_one_is_400_and_writes_nothing() {
    a_part_s_versions(
        BARE,
        &["version-extended-art", "version-pixel", "version-comic"],
        ALL_THREE,
        "version-extended-art",
        &["version-pixel", "version-comic"],
        "version-pixel-animated",
        "version-pixel",
    )
    .await;
}

/// R2943000: a card offered both of #2943's versions: BOOM! Big Max, a Legendary Unit.
const PART_FOUR: &str = "classic-080";

#[tokio::test]
async fn r2943000_r2943002_a_version_not_offered_or_an_augment_beside_one_is_400_and_writes_nothing() {
    a_part_s_versions(
        BARE,
        &["version-set-showcase", "version-out-of-frame"],
        PART_FOUR,
        "version-set-showcase",
        &["version-out-of-frame"],
        "version-out-of-frame-animated",
        "version-set-showcase",
    )
    .await;
}

/// R2944000: a card offered both of #2944's versions: Ceaseless Void, a Mythic Unit.
const PART_FIVE: &str = "core-100";

#[tokio::test]
async fn r2944000_r2944002_a_version_not_offered_or_an_augment_beside_one_is_400_and_writes_nothing() {
    a_part_s_versions(
        BARE,
        &["version-immersive", "version-prismatic"],
        PART_FIVE,
        "version-prismatic",
        &["version-immersive"],
        "version-immersive-animated",
        "version-prismatic",
    )
    .await;
}

/// R3017001: each of #3017's pairs splits every card between its two, so Speciesist and Scratch Pad between them
/// are offered all six: Scratch Pad Night, Stained-glass window and Tarot card, Speciesist the other three.
const PART_SIX: &str = "meditativeplus-026";

#[tokio::test]
async fn r3017000_a_part_six_level_not_offered_is_400_and_writes_nothing() {
    a_part_s_versions(
        BARE,
        &["version-night", "version-stained-glass", "version-tarot"],
        PART_SIX,
        "version-tarot",
        &["version-night", "version-stained-glass"],
        "version-night-animated",
        "version-night",
    )
    .await;
    a_part_s_versions(
        PART_SIX,
        &["version-neon", "version-watercolor", "version-woodcut"],
        BARE,
        "version-woodcut",
        &["version-neon", "version-watercolor"],
        "version-neon-animated",
        "version-neon",
    )
    .await;
}

#[tokio::test]
async fn r2725002_r3015006_a_signed_full_art_owner_picks_full_art_s_signed_level() {
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    // Owning Mr. Vanilla's retired signed full art counts as owning Full art's three levels on it, and the Default
    // variant's animated level; its Animated art counts as that level on Gary the Gambler.
    own(&app, &profile, CARD, "full-art-mr-vanilla-signed").await;
    own(&app, &profile, "core-004", "animated-gary-the-gambler").await;
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": CARD, "fullArt": "version-full-art-signed" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": "core-004", "fullArt": "version-animated" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    // But the Animated art owns no level of Full art's, and the signed full art owns nothing elsewhere.
    for body in [
        json!({ "cardId": "core-004", "fullArt": "version-full-art" }),
        json!({ "cardId": "core-004", "fullArt": "version-full-art-signed" }),
        json!({ "cardId": "core-011", "fullArt": "version-animated" }),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), NOT_OWNED, "{body}");
    }
    assert_eq!(
        get(&app, Some(TOKEN)).await,
        (
            200,
            json!({ "cards": [
                { "cardId": "core-004", "fullArt": "version-animated" },
                { "cardId": CARD, "fullArt": "version-full-art-signed" },
            ] })
        )
    );
}

#[tokio::test]
async fn r3015005_a_pick_names_one_owned_offered_level() {
    use jackioh_server::cosmetics::registry;
    use jackioh_server::economy::store::FullArtPick;
    use jackioh_server::loadout::{AccountPicks, freeze_loadout};
    let app = test_app().await;
    let profile = profile_id_of(&app, USER).await;
    // Carnivorous Cube owns Full art's first two levels; Speciesist owns them too, unoffered there.
    for card in [OFFERED, BARE] {
        for id in ["version-full-art", "version-full-art-animated"] {
            own(&app, &profile, card, id).await;
        }
    }
    let nothing = json!({ "cards": [] });
    for (body, message) in [
        // Unowned: the next level up.
        (
            json!({ "cardId": OFFERED, "fullArt": "version-full-art-signed" }),
            NOT_OWNED,
        ),
        // Unoffered, though owned.
        (
            json!({ "cardId": BARE, "fullArt": "version-full-art-animated" }),
            NOT_OFFERED,
        ),
        // Retired, and an augment beside an owned level.
        (
            json!({ "cardId": OFFERED, "fullArt": "full-art-mr-vanilla" }),
            RETIRED,
        ),
        (
            json!({ "cardId": OFFERED, "fullArt": "version-full-art-animated", "augments": ["augment-signature"] }),
            AUGMENTS,
        ),
        // An id no level has.
        (
            json!({ "cardId": OFFERED, "fullArt": "version-from-a-newer-client" }),
            "\"fullArt\" is not a cosmetic id of its kind",
        ),
    ] {
        assert_eq!(refusal(put(&app, TOKEN, body.clone()).await), message, "{body}");
        assert_eq!(get(&app, Some(TOKEN)).await, (200, nothing.clone()), "{body}");
    }
    // A valid pick is written, and a match freezes it by its id.
    let (status, body) = put(
        &app,
        TOKEN,
        json!({ "cardId": OFFERED, "fullArt": "version-full-art-animated" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let mut tx = app.db.begin(Some(&profile)).await.expect("a store transaction");
    let owned = tx
        .economy_cosmetics_get(&profile)
        .await
        .expect("economy.cosmetics.get");
    let picks = tx
        .economy_card_picks_get(&profile)
        .await
        .expect("economy.cardPicks.get");
    tx.commit().await.expect("commit");
    let loadout = freeze_loadout(
        registry(),
        &owned,
        &AccountPicks::default(),
        None,
        &picks,
        &[OFFERED.to_string(), BARE.to_string()],
    );
    assert_eq!(
        loadout.card(OFFERED).and_then(|card| card.full_art.clone()),
        Some(FullArtPick::Id("version-full-art-animated".to_string()))
    );
    assert!(loadout.card(OFFERED).is_some_and(|card| card.augments.is_empty()));
    assert_eq!(loadout.card(BARE), None);
}
