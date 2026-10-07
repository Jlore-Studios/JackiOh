//! Port of `apps/server/test/api/collection.test.ts`.
//!
//! SPEC §9.4's entitlement ledger, and §9.8's first row ("Claiming unowned cards → the collection
//! is server-owned").
//!
//! Two of BUILD M6-T2's three acceptance items are proved here:
//!  - "a grant with `reason` writes both tables or neither (fault-injection test)" — the fake
//!    store's `on_call` seam fails one of the two writes mid-transaction, once each way round;
//!  - "a direct insert attempt through the public API is impossible (no endpoint)" — asserted over
//!    the route table itself (`app::ROUTES`), so a mutating route cannot be added without this file
//!    going red.
//!
//! The third ('a stale `catalogVersion` gets "update required"') belongs to the deck endpoints and
//! lives in `decks.rs`.
//!
//! The catalog is the real one (`jackioh_cards`, compiled in): TS's `createTestCatalog()` (24
//! synthetic ids and one token) has no Rust stand-in, since the server reads its catalog from the
//! cards crate. Every expectation below is computed from the catalog handle, as TS's were.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::Request;
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tower::ServiceExt;

use jackioh_server::api::catalog::Catalog;
use jackioh_server::api::collection::{GrantInput, LAUNCH_GRANT_REASON, grant_cards, grant_entire_catalog, owned_map};
use jackioh_server::api::http::{ApiError, ApiErrorCode, AuthLevel};
use jackioh_server::app::{self, App};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StoreError};

use crate::support::deps;

const PROFILE: &str = "p1";
const USER: &str = "u1";

/// The environment `createTestDeps()` stood for: `E2E=1` (the fake store and fixture auth) at the
/// compiled-in catalog version (SURFACE §11.3).
fn test_env() -> IndexMap<String, String> {
    let mut source = IndexMap::new();
    for (name, value) in [("E2E", "1"), ("NODE_ENV", "test"), ("TRUSTED_PROXY_HOPS", "1")] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

/// One App and its router: TS's `deps` and `createRouter(createCollectionRoutes(), deps)`.
struct Server {
    app: Arc<App>,
    router: axum::Router,
}

/// One response, read whole.
struct Reply {
    status: u16,
    text: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).expect("the body is JSON")
    }
}

async fn built_app() -> App {
    let env = app::load_server_env(&test_env()).expect("the test environment loads");
    let built = app::build(env).await.expect("the test app builds");
    Arc::try_unwrap(built).ok().expect("app::build keeps no second handle on the App it returns")
}

/// `createTestDeps(overrides)`: an App, with R144's fixtures wiped so its store starts as empty as
/// TS's memory store (the launch grant R144 hands the active fixtures would otherwise fill both
/// ledger tables before a test begins).
async fn serve(app: App) -> Server {
    let app = Arc::new(app);
    store_of(&app).lock().await.reset();
    Server { router: app::router(app.clone()), app }
}

async fn fresh_server() -> Server {
    serve(built_app().await).await
}

/// `createTestDeps({ catalog })`.
async fn server_holding(catalog: Catalog) -> Server {
    let built = built_app().await;
    serve(App { catalog, ..built }).await
}

impl Server {
    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.expect("the router always answers");
        let status = response.status().as_u16();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");
        Reply { status, text: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8") }
    }
}

/// `jsonRequest(method, path, body, { token })`.
fn json_request(method: &str, path: &str, body: Option<Value>, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.7");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
    builder.body(body).expect("a well-formed request")
}

fn store_of(app: &App) -> Arc<Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => data.clone(),
        Db::Pg(_) => panic!("E2E=1 builds the in-memory store"),
    }
}

fn from_json<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("the literal has the port's JSON shape")
}

fn json_rows<T: Serialize>(rows: &[T]) -> Vec<Value> {
    rows.iter().map(|row| serde_json::to_value(row).expect("a row serialises")).collect()
}

/// `deps.store.tables.collection`, as TS's rows read.
async fn collection_rows(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.collection)
}

/// `deps.store.tables.grants`, as TS's rows read.
async fn grant_rows(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.grants)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("after 1970").as_millis() as i64
}

/// An active profile row and an auth user who verifies as it; answers the bearer token.
async fn active_profile(server: &Server, id: &str, user_id: &str) -> String {
    let _ = store_of(&server.app).lock().await.seed_profile(json!({ "id": id, "userId": user_id, "status": "active" }));
    deps::add_user(&server.app, user_id, &format!("{id}@example.test"), true)
}

/// `beforeEach`: fresh deps and an active `p1` with its token.
async fn setup() -> (Server, String) {
    let server = fresh_server().await;
    let token = active_profile(&server, PROFILE, USER).await;
    (server, token)
}

/// `grantCards(deps, { profileId, entries, reason })`, `entries` as TS's literal.
async fn grant(server: &Server, profile_id: &str, entries: Value, reason: &str) -> Result<(), ApiError> {
    grant_cards(
        &server.app,
        GrantInput { profile_id: profile_id.to_string(), entries: from_json(entries), reason: reason.to_string() },
    )
    .await
}

/// `ownedMap(deps, profileId)`, quantities widened so a literal compares whatever their width.
async fn owned(server: &Server, profile_id: &str) -> IndexMap<String, i64> {
    owned_map(&server.app, profile_id)
        .await
        .expect("the collection reads")
        .into_iter()
        .collect()
}

/// `deps.store.onCall = (method) => { if (method === failing) throw new Error("injected fault"); }`.
async fn fail_on(server: &Server, failing: &'static str) {
    store_of(&server.app).lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
        if method == failing { Err(StoreError::Other("injected fault".to_string())) } else { Ok(()) }
    }));
}

/// The row without its `at`, which the server stamps from its own clock.
fn without_at(row: &Value) -> Value {
    let mut row = row.clone();
    if let Some(object) = row.as_object_mut() {
        object.remove("at");
    }
    row
}

/// A ban held as server state: the catalog data is untouched, only the handle answers differently.
fn with_ban(catalog: &Catalog, banned_id: &str) -> Catalog {
    Catalog { banned: [banned_id.to_string()].into_iter().collect(), ..catalog.clone() }
}

// ---------------------------------------------------------------------------
// grantCards (§9.4: one transaction, both tables)
// ---------------------------------------------------------------------------

mod grant_cards_section_9_4_one_transaction_both_tables {
    use super::*;

    #[tokio::test]
    async fn writes_both_tables_or_neither_a_fault_on_collection_append_grants_leaves_collection_unchanged() {
        let (server, _token) = setup().await;
        fail_on(&server, "collection.appendGrants").await;

        let error = grant(&server, PROFILE, json!([{ "cardId": "core-001", "quantity": 1 }]), "reward")
            .await
            .expect_err("the injected fault fails the grant");
        assert!(format!("{error:?}").contains("injected fault"), "{error:?}");

        assert_eq!(collection_rows(&server).await, Vec::<Value>::new());
        assert_eq!(grant_rows(&server).await, Vec::<Value>::new());
    }

    #[tokio::test]
    async fn writes_both_tables_or_neither_a_fault_on_collection_upsert_quantities_leaves_grants_unchanged() {
        let (server, _token) = setup().await;
        fail_on(&server, "collection.upsertQuantities").await;

        let error = grant(&server, PROFILE, json!([{ "cardId": "core-001", "quantity": 1 }]), "reward")
            .await
            .expect_err("the injected fault fails the grant");
        assert!(format!("{error:?}").contains("injected fault"), "{error:?}");

        assert_eq!(grant_rows(&server).await, Vec::<Value>::new());
        assert_eq!(collection_rows(&server).await, Vec::<Value>::new());
    }

    #[tokio::test]
    async fn the_mirror_case_cannot_pass_by_accident_a_fault_on_either_write_rolls_back_an_existing_row_too() {
        let (server, _token) = setup().await;
        grant(&server, PROFILE, json!([{ "cardId": "core-001", "quantity": 1 }]), "reward").await.expect("the grant");
        let before = collection_rows(&server).await;

        fail_on(&server, "collection.appendGrants").await;
        let error = grant(&server, PROFILE, json!([{ "cardId": "core-001", "quantity": 1 }]), "reward")
            .await
            .expect_err("the injected fault fails the grant");
        assert!(format!("{error:?}").contains("injected fault"), "{error:?}");

        assert_eq!(collection_rows(&server).await, before);
        assert_eq!(grant_rows(&server).await.len(), 1);
    }

    #[tokio::test]
    async fn the_happy_path_leaves_matching_rows_in_both_tables_and_the_grant_carries_the_reason() {
        let (server, _token) = setup().await;
        let before = now_ms();
        grant(
            &server,
            PROFILE,
            json!([{ "cardId": "core-001", "quantity": 1 }, { "cardId": "core-002", "quantity": 2 }]),
            "reward",
        )
        .await
        .expect("the grant");
        let after = now_ms();

        assert_eq!(
            collection_rows(&server).await,
            vec![
                json!({ "profileId": PROFILE, "cardId": "core-001", "quantity": 1 }),
                json!({ "profileId": PROFILE, "cardId": "core-002", "quantity": 2 }),
            ]
        );
        let grants = grant_rows(&server).await;
        assert_eq!(
            grants.iter().map(without_at).collect::<Vec<_>>(),
            vec![
                json!({ "profileId": PROFILE, "cardId": "core-001", "delta": 1, "reason": "reward" }),
                json!({ "profileId": PROFILE, "cardId": "core-002", "delta": 2, "reason": "reward" }),
            ]
        );
        // TS stamped `deps.timers.now()` on both; the server's own clock is read once per grant.
        let stamps: Vec<i64> = grants.iter().map(|row| row["at"].as_i64().expect("an epoch-ms stamp")).collect();
        assert_eq!(stamps[0], stamps[1]);
        assert!(stamps[0] >= before && stamps[0] <= after, "{stamps:?} not in {before}..={after}");
    }

    #[tokio::test]
    async fn the_ledger_accumulates_a_second_grant_adds_to_the_quantity_and_appends_its_own_row() {
        let (server, _token) = setup().await;
        let entries = json!([{ "cardId": "core-001", "quantity": 1 }]);
        grant(&server, PROFILE, entries.clone(), "reward").await.expect("the first grant");
        grant(&server, PROFILE, entries, "admin").await.expect("the second grant");

        assert_eq!(owned(&server, PROFILE).await, IndexMap::from([("core-001".to_string(), 2)]));
        let reasons: Vec<Value> = grant_rows(&server).await.iter().map(|row| row["reason"].clone()).collect();
        assert_eq!(reasons, vec![json!("reward"), json!("admin")]);
    }

    #[tokio::test]
    async fn sums_a_card_repeated_inside_one_grant_instead_of_writing_it_twice() {
        let (server, _token) = setup().await;
        grant(
            &server,
            PROFILE,
            json!([{ "cardId": "core-001", "quantity": 1 }, { "cardId": "core-001", "quantity": 2 }]),
            "reward",
        )
        .await
        .expect("the grant");

        assert_eq!(collection_rows(&server).await, vec![json!({ "profileId": PROFILE, "cardId": "core-001", "quantity": 3 })]);
        assert_eq!(grant_rows(&server).await.len(), 1);
    }

    /// TS also tried `1.5`; a quantity is an integer here (SURFACE §4.3), so a fraction cannot be
    /// sent at all.
    #[tokio::test]
    async fn refuses_a_delta_that_is_not_a_positive_whole_number_and_writes_nothing() {
        let (server, _token) = setup().await;
        for quantity in [0, -1] {
            let error = grant(&server, PROFILE, json!([{ "cardId": "core-001", "quantity": quantity }]), "reward")
                .await
                .expect_err("the grant is refused");
            assert!(matches!(error.code, ApiErrorCode::BadRequest), "{quantity}: {error:?}");
        }
        assert_eq!(collection_rows(&server).await, Vec::<Value>::new());
        assert_eq!(grant_rows(&server).await, Vec::<Value>::new());
    }

    #[tokio::test]
    async fn an_empty_grant_opens_no_transaction_and_appends_no_zero_delta_row() {
        let (server, _token) = setup().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        store_of(&server.app).lock().await.on_call = Some(Arc::new(move |_method: &str| -> Result<(), StoreError> {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        grant(&server, PROFILE, json!([]), "reward").await.expect("an empty grant is no error");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(grant_rows(&server).await, Vec::<Value>::new());
    }
}

// ---------------------------------------------------------------------------
// grantEntireCatalog (BUILD M6-T2: launch mode grants every card)
// ---------------------------------------------------------------------------

mod grant_entire_catalog_build_m6_t2_launch_mode_grants_every_card {
    use super::*;

    #[tokio::test]
    async fn grants_every_non_token_non_banned_id_exactly_once() {
        let banned = "core-003";
        let base = built_app().await;
        let catalog = with_ban(&base.catalog, banned);
        let local = server_holding(catalog).await;
        active_profile(&local, PROFILE, USER).await;

        grant_entire_catalog(&local.app, PROFILE, Some(LAUNCH_GRANT_REASON)).await.expect("the launch grant");

        let catalog = &local.app.catalog;
        let mut expected: Vec<String> =
            catalog.card_ids.iter().filter(|id| !catalog.is_token(id) && id.as_str() != banned).cloned().collect();
        let owned = owned(&local, PROFILE).await;
        let mut keys: Vec<String> = owned.keys().cloned().collect();
        keys.sort();
        expected.sort();
        assert_eq!(keys, expected);
        assert!(owned.values().all(|quantity| *quantity == 1));
        assert_eq!(grant_rows(&local).await.len(), expected.len());
    }

    #[tokio::test]
    async fn skips_tokens_section_9_4_l3_bans_them_from_a_deck_so_owning_one_is_meaningless() {
        let (server, _token) = setup().await;
        grant_entire_catalog(&server.app, PROFILE, Some(LAUNCH_GRANT_REASON)).await.expect("the launch grant");
        let owned = owned(&server, PROFILE).await;
        let catalog = &server.app.catalog;
        let tokens: Vec<&String> = catalog.card_ids.iter().filter(|id| catalog.is_token(id)).collect();
        assert!(!tokens.is_empty());
        for card_id in tokens {
            assert!(!owned.contains_key(card_id.as_str()), "{card_id}");
        }
    }

    #[tokio::test]
    async fn is_idempotent_a_second_call_neither_doubles_a_quantity_nor_appends_a_duplicate_row() {
        let (server, _token) = setup().await;
        grant_entire_catalog(&server.app, PROFILE, Some(LAUNCH_GRANT_REASON)).await.expect("the launch grant");
        let collection = collection_rows(&server).await;
        let grants = grant_rows(&server).await;

        grant_entire_catalog(&server.app, PROFILE, Some(LAUNCH_GRANT_REASON)).await.expect("the second launch grant");

        assert_eq!(collection_rows(&server).await, collection);
        assert_eq!(grant_rows(&server).await, grants);
    }

    #[tokio::test]
    async fn tops_up_a_profile_that_already_owns_part_of_the_catalog_without_re_granting_the_rest() {
        let (server, _token) = setup().await;
        grant(&server, PROFILE, json!([{ "cardId": "core-001", "quantity": 1 }]), "reward").await.expect("the grant");
        grant_entire_catalog(&server.app, PROFILE, Some(LAUNCH_GRANT_REASON)).await.expect("the launch grant");

        let for_core_001: Vec<Value> =
            grant_rows(&server).await.into_iter().filter(|row| row["cardId"] == json!("core-001")).collect();
        assert_eq!(for_core_001.len(), 1);
        assert_eq!(for_core_001[0]["reason"], json!("reward"));
        assert_eq!(owned(&server, PROFILE).await.get("core-001"), Some(&1));
    }

    #[tokio::test]
    async fn records_the_reason_it_was_given_so_the_ledger_distinguishes_a_launch_grant() {
        let (server, _token) = setup().await;
        grant_entire_catalog(&server.app, PROFILE, Some("admin")).await.expect("the grant");
        let mut reasons: Vec<Value> = grant_rows(&server).await.iter().map(|row| row["reason"].clone()).collect();
        reasons.dedup();
        assert_eq!(reasons, vec![json!("admin")]);
    }
}

// ---------------------------------------------------------------------------
// ownedMap (§9.4 L5's input)
// ---------------------------------------------------------------------------

mod owned_map_section_9_4_l5_s_input {
    use super::*;

    #[tokio::test]
    async fn is_empty_for_a_profile_that_owns_nothing() {
        let (server, _token) = setup().await;
        assert_eq!(owned(&server, PROFILE).await, IndexMap::<String, i64>::new());
    }

    #[tokio::test]
    async fn does_not_leak_another_profile_s_entitlements() {
        let (server, _token) = setup().await;
        let _ = store_of(&server.app).lock().await.seed_profile(json!({ "id": "p2", "userId": "u2", "status": "active" }));
        grant(&server, "p2", json!([{ "cardId": "core-001", "quantity": 1 }]), "reward").await.expect("the grant");
        assert_eq!(owned(&server, PROFILE).await, IndexMap::<String, i64>::new());
        assert_eq!(owned(&server, "p2").await, IndexMap::from([("core-001".to_string(), 1)]));
    }
}

// ---------------------------------------------------------------------------
// The routes (§9.4: no client path writes the collection)
// ---------------------------------------------------------------------------

mod the_routes_section_9_4_no_client_path_writes_the_collection {
    use super::*;

    #[test]
    fn exposes_get_api_collection_and_nothing_else() {
        let routes: Vec<(String, bool)> = app::ROUTES
            .iter()
            .filter(|(_, path, _, _)| path.starts_with("/api/collection"))
            .map(|(method, path, auth, _)| (format!("{method} {path}"), matches!(auth, AuthLevel::Active)))
            .collect();
        assert_eq!(routes.iter().map(|(route, _)| route.as_str()).collect::<Vec<_>>(), vec!["GET /api/collection"]);
        assert!(routes.iter().all(|(route, _)| route.starts_with("GET ")));
        assert!(routes.iter().all(|(_, active)| *active));
    }

    #[tokio::test]
    async fn has_no_mutating_route_at_all_post_put_patch_and_delete_are_not_routed() {
        let (server, token) = setup().await;
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            let response = server
                .send(json_request(
                    method,
                    "/api/collection",
                    Some(json!({ "cardId": "core-001", "quantity": 99 })),
                    Some(&token),
                ))
                .await;
            assert!([404, 405].contains(&response.status), "{method}: {}", response.status);
            assert_eq!(collection_rows(&server).await, Vec::<Value>::new());
            assert_eq!(grant_rows(&server).await, Vec::<Value>::new());
        }
    }

    #[tokio::test]
    async fn get_returns_the_catalog_version_and_the_owned_entries() {
        let (server, token) = setup().await;
        grant(
            &server,
            PROFILE,
            json!([{ "cardId": "core-002", "quantity": 1 }, { "cardId": "core-001", "quantity": 2 }]),
            "reward",
        )
        .await
        .expect("the grant");

        let response = server.send(json_request("GET", "/api/collection", None, Some(&token))).await;
        assert_eq!(response.status, 200);
        assert_eq!(
            response.json(),
            json!({
                "catalogVersion": server.app.catalog.version,
                "entries": [
                    { "cardId": "core-001", "quantity": 2 },
                    { "cardId": "core-002", "quantity": 1 },
                ],
            })
        );
    }

    #[tokio::test]
    async fn a_pending_account_gets_403_section_9_4_no_collection_before_an_invite_code_is_redeemed() {
        let local = fresh_server().await;
        let _ = store_of(&local.app).lock().await.seed_profile(json!({ "id": PROFILE, "userId": USER, "status": "pending" }));
        let pending_token = deps::add_user(&local.app, USER, "pending@example.test", true);

        let response = local.send(json_request("GET", "/api/collection", None, Some(&pending_token))).await;
        assert_eq!(response.status, 403);
        assert_eq!(response.json()["error"]["code"], json!("account_pending"));
    }

    #[tokio::test]
    async fn an_unauthenticated_request_gets_401() {
        let (server, _token) = setup().await;
        let response = server.send(json_request("GET", "/api/collection", None, None)).await;
        assert_eq!(response.status, 401);
    }
}
