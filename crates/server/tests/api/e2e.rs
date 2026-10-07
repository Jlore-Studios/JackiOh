//! BUILD M8's `E2E=1` test server: the in-memory store, R111's launch-grant trigger, R144's reseed,
//! the fixture auth provider, R143's optional seed and the CORS layer.
//!
//! The invite-gate assertions here are the unit-test half of `e2e/cypress/e2e/10-invite-gate.cy.ts`:
//! that spec activates the pending fixture account and spends the good code, so without R144 it
//! passes once and fails on every later run. Running the reseed twice and then redeeming twice is
//! the whole of that claim, and it is checked here rather than in Cypress because a spec cannot
//! restart the server it is talking to.
//!
//! Ported from `apps/server/test/api/e2e.test.ts` (part 18). TS assembled its runtime from ports
//! (`createE2EStore`, `createE2EAuth`, a fake match directory, a route subset); the Rust server has
//! one `App` (SURFACE §11.2), so the harness is `support::deps::test_app()` (FakeStore, the fixture
//! auth, the fixtures) with its store swapped for one over this file's small catalog where the
//! launch grant's size is the point. What SURFACE §11.3 removed is ported as its Rust answer: there
//! is no sign-up route, and the queue body names its mode and its deck by id.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Request};
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::Sha256;
use tower::ServiceExt;

use jackioh_engine::config::DECK_SIZE;
use jackioh_server::api::collection::{LAUNCH_COPIES, LAUNCH_GRANT_REASON};
use jackioh_server::api::cors::is_origin_allowed;
use jackioh_server::api::crypto::normalize_code;
use jackioh_server::api::e2e::{
    E2E_ACCOUNTS, E2E_INVITE_CODES, E2EAccount, E2EInviteCodes, E2ESeedOptions, seed_e2e_fixtures,
    seed_e2e_fixtures_with,
};
use jackioh_server::api::queue::e2e_seed_count;
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{MAX_SAVED_DECKS, REDEMPTION_IDENTICAL_ERROR};
use jackioh_server::db::fake::{E2eStoreOptions, FakeCatalog, create_e2e_store};
use jackioh_server::db::store::{
    CollectionEntry, Db, InviteCode, MatchActionRow, ProfileCreateInput, Room, SavedDeck, Ticket, Tx,
};

use crate::support::deps::{call, test_app};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const PLAYABLE: [&str; 4] = ["core-001", "core-002", "core-003", "core-004"];
const TOKEN: &str = "token-sheep";
const BANNED: &str = "core-666";

/// §9.4 L3 bans Tokens from a deck and L6 bans banned ids, so R111 grants neither.
fn grant_catalog() -> FakeCatalog {
    let mut card_ids: Vec<String> = PLAYABLE.iter().map(|id| id.to_string()).collect();
    card_ids.push(TOKEN.to_string());
    card_ids.push(BANNED.to_string());
    FakeCatalog {
        card_ids,
        is_token: Arc::new(|card_id: &str| card_id == TOKEN),
        is_banned: Arc::new(|card_id: &str| card_id == BANNED),
    }
}

/// End-to-end mode's store over `grant_catalog()`, on the server's clock.
fn e2e_store() -> Db {
    create_e2e_store(E2eStoreOptions {
        catalog: grant_catalog(),
        now: Arc::new(now_ms),
        redemption: None,
    })
}

// ---------------------------------------------------------------------------
// Harness (a private copy per file, SURFACE rule 5)
// ---------------------------------------------------------------------------

/// A port value built from TS's own object literal, so the test depends on the JSON shape only.
fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` with a matching value.
fn assert_matches(actual: &Value, expected: &Value, at: &str) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            for (key, value) in expected {
                let found = actual.get(key).unwrap_or_else(|| panic!("{at}.{key} is missing"));
                assert_matches(found, value, &format!("{at}.{key}"));
            }
        }
        _ => assert_eq!(actual, expected, "{at}"),
    }
}

/// The test app with every field as `test_app()` builds it, for a test to change one.
async fn bare_app() -> App {
    Arc::try_unwrap(test_app().await).unwrap_or_else(|_| panic!("test_app() hands back its only reference"))
}

/// A runtime with end-to-end mode's two ports and the real routes on top: the test app, its store
/// swapped for one over `grant_catalog()`, so the launch grant is four cards.
async fn harness() -> Arc<App> {
    let app = bare_app().await;
    Arc::new(App {
        db: e2e_store(),
        ..app
    })
}

/// §9.4's code hash as the server takes it (TS `createHashes(...).code`, peppered as `index.ts`
/// peppers it, `${CODE_PEPPER}:code`): HMAC-SHA256 of the normalised code (R191), in hex.
struct Hashes {
    pepper: String,
}

impl Hashes {
    fn code(&self, plain: &str) -> String {
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(format!("{}:code", self.pepper).as_bytes())
            .expect("any key length");
        mac.update(normalize_code(plain).as_bytes());
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

fn hashes(app: &App) -> Hashes {
    Hashes {
        pepper: app.env.code_pepper.clone(),
    }
}

/// One store call in a transaction of its own (TS called the store's methods bare).
macro_rules! store {
    ($db:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut t = $db.begin(None).await.expect("begin");
        let out = t.$method($($arg),*).await.unwrap_or_else(|error| panic!("{}: {error:?}", stringify!($method)));
        t.commit().await.expect("commit");
        out
    }};
}

/// The same call, answering its `Result` instead of insisting on `Ok`.
macro_rules! try_store {
    ($db:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut t = $db.begin(None).await.expect("begin");
        let out = t.$method($($arg),*).await;
        if out.is_ok() {
            t.commit().await.expect("commit");
        }
        out
    }};
}

fn account(user_id: &str) -> &'static E2EAccount {
    E2E_ACCOUNTS
        .iter()
        .find(|candidate| candidate.user_id == user_id)
        .unwrap_or_else(|| panic!("no fixture account {user_id}"))
}

fn pending() -> &'static E2EAccount {
    account("e2e-pending")
}

fn p1() -> &'static E2EAccount {
    account("e2e-p1")
}

/// TS `store.grantsFor(profileId)`.
async fn grants_for(db: &Db, profile_id: &str) -> Vec<Value> {
    match db {
        Db::Fake(data) => data
            .lock()
            .await
            .grants_for(profile_id)
            .iter()
            .map(json_of)
            .collect(),
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

async fn create_profile(db: &Db, user_id: &str) -> Value {
    let input: ProfileCreateInput = from(
        json!({ "userId": user_id, "email": format!("{user_id}@example.test"), "rating": 1000, "at": 1 }),
    );
    json_of(&store!(db, profiles_create(&input)))
}

fn id_of(row: &Value) -> String {
    row["id"].as_str().expect("an id").to_string()
}

async fn collection_of(db: &Db, profile_id: &str) -> Value {
    json_of(&store!(db, collection_get(profile_id)))
}

async fn set_status(db: &Db, profile_id: &str, status: &str) {
    store!(db, profiles_set_status(profile_id, from(json!(status))));
}

async fn redeem(app: &Arc<App>, code: &str) -> (u16, Value) {
    let (status, _headers, body) = call(
        app,
        "POST",
        "/api/codes/redeem",
        Some(pending().token),
        json!({ "code": code }),
    )
    .await;
    (status, body)
}

/// One response, read whole.
struct Reply {
    status: u16,
    headers: HeaderMap,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(name)
            .map(|value| value.to_str().expect("an ASCII header"))
    }
}

/// A request with headers of the test's choosing, through the real router (CORS included).
async fn send(app: &Arc<App>, method: &str, path: &str, headers: &[(&str, &str)]) -> Reply {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app::router(app.clone())
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("the router answers");
    Reply {
        status: response.status().as_u16(),
        headers: response.headers().clone(),
    }
}

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

mod the_end_to_end_store {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn rolls_a_transaction_back_when_the_callback_throws_so_a_partial_write_cannot_survive() {
        let db = e2e_store();
        let profile = create_profile(&db, "u1").await;
        let id = id_of(&profile);

        {
            let mut t = db.begin(None).await.expect("begin");
            t.profiles_set_in_match(&id, Some("match-1"))
                .await
                .expect("profiles.setInMatch");
            let entries: Vec<CollectionEntry> = from(json!([{ "cardId": "core-001", "quantity": 3 }]));
            t.collection_upsert_quantities(&id, &entries)
                .await
                .expect("collection.upsertQuantities");
            // "the second write failed": TS threw out of the callback; here the transaction is
            // dropped without a commit, which is the Rust store's rollback (SURFACE §11.2).
            drop(t);
        }

        assert!(json_of(&store!(db, profiles_get_by_id(&id)))["inMatchId"].is_null());
        assert_eq!(collection_of(&db, &id).await, json!([]));
    }

    #[tokio::test(start_paused = true)]
    async fn commits_a_transaction_that_returns_and_lets_a_nested_tx_join_it() {
        let db = e2e_store();
        let profile = create_profile(&db, "u1").await;
        let id = id_of(&profile);

        // The nested `tx` of TS joined the outer one; in Rust the inner step is handed the outer
        // transaction (a second `begin` would wait on the first's lock).
        async fn inner(t: &mut Tx<'_>, id: &str) {
            let entries: Vec<CollectionEntry> = from(json!([{ "cardId": "core-001", "quantity": 2 }]));
            t.collection_upsert_quantities(id, &entries)
                .await
                .expect("collection.upsertQuantities");
        }
        let mut t = db.begin(None).await.expect("begin");
        t.profiles_set_in_match(&id, Some("match-1"))
            .await
            .expect("profiles.setInMatch");
        inner(&mut t, &id).await;
        t.commit().await.expect("commit");

        assert_eq!(
            json_of(&store!(db, profiles_get_by_id(&id)))["inMatchId"],
            json!("match-1")
        );
        assert_eq!(
            collection_of(&db, &id).await,
            json!([{ "cardId": "core-001", "quantity": 2 }])
        );
    }

    #[tokio::test(start_paused = true)]
    async fn claims_an_invite_code_once_9_4_step_6_two_callers_cannot_both_win_the_last_use() {
        let db = e2e_store();
        let code: InviteCode = from(json!({
            "id": "code-1",
            "codeHash": "hash-1",
            "maxUses": 1,
            "uses": 0,
            "revoked": false,
            "expiresAt": null,
            "createdAt": 0,
        }));
        store!(db, codes_insert(&code));
        let claim = || async { store!(db, codes_claim("code-1", 10)) };
        let (first, second) = tokio::join!(claim(), claim());
        assert_eq!([first, second].iter().filter(|won| **won).count(), 1);
        assert_eq!(
            json_of(&store!(db, codes_find_by_hash("hash-1")))["uses"],
            json!(1)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn claims_a_room_once_9_5_the_loser_of_a_join_race_gets_nothing() {
        let db = e2e_store();
        let room: Room = from(json!({
            "code": "ABCDEF",
            "hostProfileId": "host",
            "mode": "bo1",
            "hostDeck": [],
            "hostTrio": null,
            "catalogVersion": "test-1",
            "createdAt": 0,
            "expiresAt": 1_000,
            "guestProfileId": null,
            "matchId": null,
        }));
        store!(db, rooms_create(&room));
        assert!(store!(db, rooms_claim("ABCDEF", "guest-a", "match-a", 1)).is_some());
        assert!(store!(db, rooms_claim("ABCDEF", "guest-b", "match-b", 1)).is_none());
        // ...and an expired room is not joinable at all.
        assert!(store!(db, rooms_claim("ABCDEF", "guest-c", "match-c", 2_000)).is_none());
    }

    fn ticket(id: &str, profile_id: &str) -> Ticket {
        from(json!({
            "id": id,
            "profileId": profile_id,
            "rating": 1000,
            "mode": "bo1",
            "deck": [],
            "trio": null,
            "catalogVersion": "test-1",
            "enqueuedAt": 0,
            "status": "open",
            "matchId": null,
        }))
    }

    #[tokio::test(start_paused = true)]
    async fn claims_a_pair_of_tickets_in_one_statement_9_5_and_refuses_a_second_pairing() {
        let db = e2e_store();
        for (id, profile_id) in [("t1", "p1"), ("t2", "p2"), ("t3", "p3")] {
            store!(db, tickets_insert(&ticket(id, profile_id)));
        }

        assert!(store!(db, tickets_claim_pair("t1", "t2", "match-1", 1)));
        // t1 is already matched, so no second matcher can pair it with anyone.
        assert!(!store!(db, tickets_claim_pair("t1", "t3", "match-2", 1)));
        assert_eq!(json_of(&store!(db, tickets_get("t3")))["status"], json!("open"));
        assert_eq!(json_of(&store!(db, tickets_count_open())), json!(1));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_second_open_ticket_for_one_profile_tickets_profile_queued_key() {
        let db = e2e_store();
        store!(db, tickets_insert(&ticket("t1", "p1")));
        let refused = try_store!(db, tickets_insert(&ticket("t2", "p1"))).expect_err("a second open ticket");
        assert!(refused.to_string().contains("already queued"), "{refused}");
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_replayed_match_seq_9_3() {
        let db = e2e_store();
        let rows: Vec<MatchActionRow> = from(json!([
            { "matchId": "m1", "seq": 1, "action": { "type": "endTurn", "playerId": "p1", "nonce": "n" }, "at": 0 },
        ]));
        store!(db, matches_append_actions(&rows));
        let refused = try_store!(db, matches_append_actions(&rows)).expect_err("a replayed seq");
        assert!(refused.to_string().contains("append-only"), "{refused}");
    }
}

// ---------------------------------------------------------------------------
// R111
// ---------------------------------------------------------------------------

mod r111_the_launch_grant_on_pending_active {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn grants_one_copy_of_every_non_token_unbanned_card_and_writes_both_tables() {
        let db = e2e_store();
        let id = id_of(&create_profile(&db, "u1").await);

        assert_eq!(collection_of(&db, &id).await, json!([]));
        set_status(&db, &id, "active").await;

        let entries = collection_of(&db, &id).await;
        let entries = entries.as_array().expect("a list");
        let mut ids: Vec<&str> = entries
            .iter()
            .map(|entry| entry["cardId"].as_str().expect("an id"))
            .collect();
        ids.sort();
        assert_eq!(ids, PLAYABLE.to_vec());
        for entry in entries {
            assert_eq!(entry["quantity"], json!(LAUNCH_COPIES));
        }

        // §9.4: "Every collection change writes `collection` and `collection_grants`."
        let grants = grants_for(&db, &id).await;
        assert_eq!(grants.len(), PLAYABLE.len());
        for grant in &grants {
            assert_eq!(grant["delta"], json!(LAUNCH_COPIES));
            assert_eq!(grant["reason"], json!(LAUNCH_GRANT_REASON));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn is_idempotent_a_second_pending_active_transition_writes_no_row_at_all() {
        let db = e2e_store();
        let id = id_of(&create_profile(&db, "u1").await);

        set_status(&db, &id, "active").await;
        let after_first = collection_of(&db, &id).await;
        let grants_after_first = grants_for(&db, &id).await.len();

        // Back to pending and active again: the trigger fires, the deltas are all zero, and
        // `collection_grants.delta <> 0` means nothing may be appended.
        set_status(&db, &id, "pending").await;
        set_status(&db, &id, "active").await;

        assert_eq!(collection_of(&db, &id).await, after_first);
        assert_eq!(grants_for(&db, &id).await.len(), grants_after_first);
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_fire_on_any_other_status_change() {
        let db = e2e_store();
        let id = id_of(&create_profile(&db, "u1").await);
        set_status(&db, &id, "banned").await;
        assert_eq!(collection_of(&db, &id).await, json!([]));
        assert!(grants_for(&db, &id).await.is_empty());
    }
}

// ---------------------------------------------------------------------------
// R144
// ---------------------------------------------------------------------------

mod r144_the_reseed_at_boot {
    use super::*;

    /// The invite-code row a plaintext code hashes to, or null.
    async fn by_kind(app: &App, hashes: &Hashes, code: &str) -> Value {
        json_of(&store!(app.db, codes_find_by_hash(&hashes.code(code))))
    }

    #[tokio::test(start_paused = true)]
    async fn seeds_the_three_fixture_accounts_two_of_them_active_and_owning_every_card() {
        let app = harness().await;
        seed_e2e_fixtures(&app).await.expect("the reseed");

        for fixture in E2E_ACCOUNTS.iter() {
            let profile = json_of(&store!(app.db, profiles_get_by_user_id(&fixture.user_id)));
            assert!(!profile.is_null(), "{} has a profile", fixture.user_id);
            assert_eq!(profile["status"], json_of(&fixture.status));
            let owned = collection_of(&app.db, profile["id"].as_str().unwrap_or("")).await;
            // A6: `e2e-p1` and `e2e-p2` "own every card"; `e2e-pending` owns nothing until it redeems.
            let expected = if profile["status"] == json!("active") {
                PLAYABLE.len()
            } else {
                0
            };
            assert_eq!(
                owned.as_array().map_or(0, Vec::len),
                expected,
                "{}",
                fixture.user_id
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn seeds_the_good_expired_and_exhausted_codes_and_never_the_missing_one() {
        let app = harness().await;
        seed_e2e_fixtures(&app).await.expect("the reseed");

        let hashes = hashes(&app);
        assert!(!by_kind(&app, &hashes, E2E_INVITE_CODES.good).await.is_null());
        assert!(!by_kind(&app, &hashes, E2E_INVITE_CODES.expired).await.is_null());
        assert!(!by_kind(&app, &hashes, E2E_INVITE_CODES.exhausted).await.is_null());
        // The whole point of the "missing" fixture: spec 10's first failure kind is a lookup miss.
        assert!(by_kind(&app, &hashes, E2E_INVITE_CODES.missing).await.is_null());

        let expired = by_kind(&app, &hashes, E2E_INVITE_CODES.expired).await;
        assert!(!expired["expiresAt"].is_null());
        assert!(expired["expiresAt"].as_i64().unwrap_or(i64::MAX) < now_ms());

        let exhausted = by_kind(&app, &hashes, E2E_INVITE_CODES.exhausted).await;
        assert!(exhausted["uses"].as_i64() >= exhausted["maxUses"].as_i64().or(Some(0)));
    }

    #[tokio::test(start_paused = true)]
    async fn hashes_a_code_the_way_a_typed_one_is_hashed_so_formatting_and_case_cannot_miss_it() {
        let app = harness().await;
        seed_e2e_fixtures(&app).await.expect("the reseed");
        let typed = E2E_INVITE_CODES.good.to_lowercase().replace('-', " ");
        assert!(store!(app.db, codes_find_by_hash(&hashes(&app).code(&typed))).is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn is_repeatable_reseeding_twice_the_good_code_still_activates_once_and_then_conflicts() {
        let app = harness().await;
        // R144: "the server reseeds ... at boot". Two boots.
        seed_e2e_fixtures(&app).await.expect("the first reseed");
        let first = seed_e2e_fixtures(&app).await.expect("the second reseed");
        assert_eq!(first.granted_cards as usize, PLAYABLE.len());

        let pending_profile = json_of(&store!(app.db, profiles_get_by_user_id(&pending().user_id)));
        assert_eq!(
            pending_profile["status"],
            json!("pending"),
            "the reseed put it back to pending"
        );

        // Spec 10's three failure kinds first, which is also §9.4's per-profile attempt budget being
        // respected: three failures plus one success is four logged attempts.
        for code in [
            E2E_INVITE_CODES.missing,
            E2E_INVITE_CODES.expired,
            E2E_INVITE_CODES.exhausted,
        ] {
            let (status, body) = redeem(&app, code).await;
            assert_eq!(status, 400, "{code}");
            assert_eq!(body["error"]["code"], json!("invalid_code"));
            assert_eq!(body["error"]["message"], json!(REDEMPTION_IDENTICAL_ERROR));
            assert!(
                body["error"].get("details").is_none(),
                "§9.4 keeps the operator-facing reason out of the client"
            );
        }

        let (status, body) = redeem(&app, E2E_INVITE_CODES.good).await;
        assert_eq!(status, 200);
        assert_eq!(body, json!({ "status": "active", "needsInviteCode": false }));

        // R111/R145: an already-active account is a conflict about the *account*, reported distinctly
        // from the three code failures because it leaks nothing about the code space.
        let (status, body) = redeem(&app, E2E_INVITE_CODES.good).await;
        assert_eq!(status, 409);
        assert_ne!(body["error"]["message"], json!(REDEMPTION_IDENTICAL_ERROR));

        // ...and the freshly activated account owns exactly one copy of everything, once.
        let (status, _headers, owned) =
            call(&app, "GET", "/api/collection", Some(pending().token), Value::Null).await;
        assert_eq!(status, 200);
        let entries = owned["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), PLAYABLE.len());
        for entry in entries {
            assert_eq!(entry["quantity"], json!(LAUNCH_COPIES));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_fixture_code_that_9_4_would_call_malformed_rather_than_seeding_a_dead_code() {
        let app = harness().await;
        let options = E2ESeedOptions {
            codes: E2EInviteCodes {
                good: "OOOO-OOOO-OOOO-OOOO",
                ..E2E_INVITE_CODES
            },
            ..E2ESeedOptions::default()
        };
        let refused = seed_e2e_fixtures_with(&app, options)
            .await
            .expect_err("a malformed fixture code");
        assert!(refused.to_string().contains("CODE_ALPHABET"), "{refused}");
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_two_fixture_codes_that_are_the_same_code() {
        let app = harness().await;
        let options = E2ESeedOptions {
            codes: E2EInviteCodes {
                missing: E2E_INVITE_CODES.good,
                ..E2E_INVITE_CODES
            },
            ..E2ESeedOptions::default()
        };
        let refused = seed_e2e_fixtures_with(&app, options)
            .await
            .expect_err("two equal fixture codes");
        assert!(refused.to_string().contains("same code"), "{refused}");
    }
}

// ---------------------------------------------------------------------------
// The fixture auth provider
// ---------------------------------------------------------------------------

mod the_fixture_auth_provider {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn verifies_each_static_token_from_e2e_support_config_ts_and_nothing_else() {
        let app = test_app().await;
        for fixture in E2E_ACCOUNTS.iter() {
            let user = app
                .auth
                .verify(fixture.token)
                .await
                .unwrap_or_else(|_| panic!("{} verifies", fixture.token));
            assert_eq!(user.user_id, fixture.user_id);
            assert_eq!(user.email.as_deref(), Some(fixture.email));
            // §9.4 makes a verified email a precondition of redemption, and spec 10 asserts it on the
            // pending account.
            assert!(user.email_verified);
            assert!(user.app_metadata.is_empty());
        }
        assert!(app.auth.verify("e2e-token-nobody").await.is_err());
        assert!(app.auth.verify("").await.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn signs_in_with_the_fixture_email_and_password_and_refuses_anything_else() {
        let app = test_app().await;
        let session = app
            .auth
            .sign_in(p1().email, p1().password)
            .await
            .expect("the fixture signs in");
        assert_eq!(session.access_token, p1().token);
        assert_eq!(session.user.user_id, p1().user_id);

        assert!(app.auth.sign_in(p1().email, "wrong").await.is_err());
        assert!(
            app.auth
                .sign_in("nobody@jackioh.test", p1().password)
                .await
                .is_err()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn cannot_create_an_account() {
        // TS: `signUp` rejects with an `ApiError`. SURFACE §11.3 drops `POST /api/auth/signup` and
        // `Auth` has no sign-up at all, so the Rust answer is that no such endpoint exists.
        let app = test_app().await;
        let body = json!({ "email": "new@jackioh.test", "password": "password" });
        let (status, _headers, body) = call(&app, "POST", "/api/auth/signup", None, body).await;
        assert_eq!(status, 404);
        assert_eq!(body["error"]["code"], json!("not_found"));
    }

    #[tokio::test(start_paused = true)]
    async fn carries_a_pending_account_through_api_auth_me_exactly_as_the_code_screen_reads_it() {
        let app = harness().await;
        seed_e2e_fixtures(&app).await.expect("the reseed");
        let (status, _headers, body) =
            call(&app, "GET", "/api/auth/me", Some(pending().token), Value::Null).await;
        assert_eq!(status, 200);
        assert_matches(
            &body,
            &json!({ "profile": { "status": "pending" }, "needsInviteCode": true, "emailVerified": true }),
            "me",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn closes_9_4_s_gate_on_the_pending_account() {
        let app = harness().await;
        seed_e2e_fixtures(&app).await.expect("the reseed");
        let (status, _headers, body) =
            call(&app, "GET", "/api/collection", Some(pending().token), Value::Null).await;
        assert_eq!(status, 403);
        assert_matches(&body, &json!({ "error": { "code": "account_pending" } }), "body");
    }

    #[test]
    fn has_not_drifted_from_e2e_support_config_ts_which_is_the_suite_s_source_of_truth() {
        // `e2e/` is its own pnpm root and that file calls `Cypress.expose`, so it cannot be imported:
        // its defaults are read as text instead. If this fails, one of the two files moved alone.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../e2e/support/config.ts");
        let source = std::fs::read_to_string(path).expect("e2e/support/config.ts");
        for fixture in E2E_ACCOUNTS.iter() {
            assert!(
                source.contains(&format!("\"{}\"", fixture.email)),
                "{}'s email",
                fixture.user_id
            );
            assert!(
                source.contains(&format!("\"{}\"", fixture.password)),
                "{}'s password",
                fixture.user_id
            );
            assert!(
                source.contains(&format!("\"{}\"", fixture.token)),
                "{}'s token",
                fixture.user_id
            );
        }
        let codes = [
            E2E_INVITE_CODES.good,
            E2E_INVITE_CODES.missing,
            E2E_INVITE_CODES.expired,
            E2E_INVITE_CODES.exhausted,
        ];
        for code in codes {
            assert!(source.contains(&format!("\"{code}\"")), "the {code} invite code");
        }
    }
}

// ---------------------------------------------------------------------------
// R143
// ---------------------------------------------------------------------------

mod r143_the_optional_seed {
    use super::*;

    /// A client-minted deck id (R256).
    fn uuid(n: usize) -> String {
        format!("00000000-0000-4000-8000-{n:012}")
    }

    /// A legal Best-of-1 deck of the real catalog (the queue judges L2–L6 with the shared
    /// validator, SURFACE §11.3): `DECK_SIZE` distinct cards that are not Tokens.
    fn legal_deck() -> Vec<String> {
        jackioh_cards::CATALOG
            .values()
            .filter(|def| !def.token && !def.tags.iter().any(|tag| tag.as_str() == "Token"))
            .take(DECK_SIZE as usize)
            .map(|def| def.id.clone())
            .collect()
    }

    struct QueueHarness {
        app: Arc<App>,
        tokens: Vec<String>,
        decks: Vec<String>,
    }

    async fn queue_harness(e2e: bool) -> QueueHarness {
        let mut app = bare_app().await;
        app.env.e2e = e2e;
        let app = Arc::new(app);
        seed_e2e_fixtures(&app).await.expect("the reseed");
        let cards = legal_deck();
        let mut tokens = Vec::new();
        let mut decks = Vec::new();
        for (index, fixture) in E2E_ACCOUNTS
            .iter()
            .filter(|candidate| json_of(&candidate.status) == json!("active"))
            .enumerate()
        {
            let profile = json_of(&store!(app.db, profiles_get_by_user_id(&fixture.user_id)));
            let id = profile["id"].as_str().unwrap_or("").to_string();
            // One saved deck each, owned card for card, so `{ mode: "bo1", deckId }` below is a legal
            // Best of 1 on it (R257). TS sent the legacy `{ deckIndex: 0 }`, which SURFACE §11.3 drops.
            let owned: Vec<CollectionEntry> = from(json!(
                cards
                    .iter()
                    .map(|card| json!({ "cardId": card, "quantity": 1 }))
                    .collect::<Vec<_>>()
            ));
            store!(app.db, collection_upsert_quantities(&id, &owned));
            let deck: SavedDeck = from(json!({
                "id": uuid(index + 1),
                "profileId": id,
                "name": "Fixture deck",
                "cards": cards,
                "portrait": null,
                "catalogVersion": jackioh_cards::catalog_version(),
                "createdAt": 0,
                "updatedAt": 0,
            }));
            store!(app.db, decks_upsert(&deck, MAX_SAVED_DECKS as i64));
            tokens.push(fixture.token.to_string());
            decks.push(uuid(index + 1));
        }
        QueueHarness { app, tokens, decks }
    }

    async fn enqueue(h: &QueueHarness, index: usize, seed: Option<Value>) -> (u16, Value) {
        let mut body = json!({ "mode": "bo1", "deckId": h.decks[index] });
        if let Some(seed) = seed {
            body["seed"] = seed;
        }
        let (status, _headers, answer) =
            call(&h.app, "POST", "/api/queue", Some(h.tokens[index].as_str()), body).await;
        (status, answer)
    }

    async fn started_seeds(app: &App) -> Vec<String> {
        match &app.db {
            Db::Fake(data) => data
                .lock()
                .await
                .tables
                .matches
                .iter()
                .map(|row| row.seed.clone())
                .collect(),
            Db::Pg(_) => panic!("the API tests run on the fake store"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn rejects_seed_outside_end_to_end_mode_it_is_refused_never_ignored() {
        let h = queue_harness(false).await;
        let (status, body) = enqueue(&h, 0, Some(json!("spec-05"))).await;
        assert_eq!(status, 400);
        assert_eq!(body["error"]["code"], json!("bad_request"));
        assert!(body["error"]["message"].as_str().unwrap_or("").contains("seed"));
    }

    #[tokio::test(start_paused = true)]
    async fn uses_a_supplied_seed_verbatim_for_the_match_the_pair_becomes_in_end_to_end_mode() {
        let h = queue_harness(true).await;
        let seed = "05-reconnect";
        let held_before = e2e_seed_count();

        let (first, _) = enqueue(&h, 0, Some(json!(seed))).await;
        assert_eq!(first, 200);
        let (second, _) = enqueue(&h, 1, None).await;
        assert_eq!(second, 200);

        // TS read the fake match directory's `started`; the match row the registry writes is the
        // same record here.
        assert_eq!(started_seeds(&h.app).await, vec![seed.to_string()]);
        // The seed is consumed with the pair, so nothing accumulates across a long-running server.
        // TS's map was per test file; the Rust one is a process static the binary's other tests
        // share (part 19's queue.rs), so what is checked is that this test left nothing behind.
        assert!(e2e_seed_count() <= held_before, "the pair's seed is still held");
    }

    #[tokio::test(start_paused = true)]
    async fn still_mints_a_seed_when_none_is_supplied_9_3_the_server_owns_it() {
        let h = queue_harness(true).await;
        for index in 0..h.tokens.len() {
            let (status, _) = enqueue(&h, index, None).await;
            assert_eq!(status, 200);
        }
        let seeds = started_seeds(&h.app).await;
        assert_eq!(seeds.len(), 1);
        // TS's fake ids minted `seed-<n>`; the server's own are 16 random bytes in hex (`crypto.rs`).
        assert_eq!(seeds[0].len(), 32, "{}", seeds[0]);
        assert!(
            seeds[0]
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
            "{}",
            seeds[0]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_seed_that_is_not_a_non_empty_string_even_in_end_to_end_mode() {
        let h = queue_harness(true).await;
        let (status, _) = enqueue(&h, 0, Some(json!(7))).await;
        assert_eq!(status, 400);
    }
}

// ---------------------------------------------------------------------------
// CORS and the catalog route
// ---------------------------------------------------------------------------

mod cors {
    //! TS wrapped a bare handler in `withCors(handler, { origins })`; here the CORS layer is driven
    //! through the real router, whose allow-list in end-to-end mode holds the Vite dev origin
    //! (`index.ts`'s `browserOrigins`). A route that answers 200 without a token stands in for TS's
    //! `nothing` handler.

    use super::*;

    const ALLOWED: &str = "http://localhost:5173";
    const EVIL: &str = "http://evil.example";
    const OPEN_ROUTE: &str = "/api/catalog";

    fn origins() -> Vec<String> {
        vec![ALLOWED.to_string()]
    }

    #[test]
    fn matches_an_origin_regardless_of_a_trailing_slash_and_nothing_else() {
        assert!(is_origin_allowed(&origins(), Some(ALLOWED)));
        assert!(is_origin_allowed(
            &["http://localhost:5173/".to_string()],
            Some(ALLOWED)
        ));
        assert!(!is_origin_allowed(&origins(), Some(EVIL)));
        assert!(!is_origin_allowed(&origins(), None));
    }

    #[tokio::test(start_paused = true)]
    async fn answers_a_preflight_for_an_allowed_origin_with_the_headers_the_client_sends() {
        let app = test_app().await;
        let response = send(
            &app,
            "OPTIONS",
            "/api/decks/00000000-0000-4000-8000-000000000001",
            &[
                ("origin", ALLOWED),
                ("access-control-request-method", "PUT"),
                ("access-control-request-headers", "authorization, content-type"),
            ],
        )
        .await;
        assert_eq!(response.status, 204);
        assert_eq!(response.header("access-control-allow-origin"), Some(ALLOWED));
        let allowed_headers = response.header("access-control-allow-headers").unwrap_or("");
        assert!(allowed_headers.contains("authorization"), "{allowed_headers}");
        assert!(allowed_headers.contains("content-type"), "{allowed_headers}");
        assert!(
            response
                .header("access-control-allow-methods")
                .unwrap_or("")
                .contains("PUT")
        );
        assert_eq!(response.header("vary"), Some("Origin"));
    }

    #[tokio::test(start_paused = true)]
    async fn never_sends_a_wildcard_and_never_sends_credentials() {
        let app = test_app().await;
        let response = send(&app, "GET", OPEN_ROUTE, &[("origin", ALLOWED)]).await;
        assert_eq!(response.header("access-control-allow-origin"), Some(ALLOWED));
        assert_eq!(response.header("access-control-allow-credentials"), None);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_an_unlisted_origin_the_ordinary_response_and_no_cors_headers() {
        let app = test_app().await;
        let response = send(&app, "GET", OPEN_ROUTE, &[("origin", EVIL)]).await;
        assert_eq!(response.status, 200);
        assert_eq!(response.header("access-control-allow-origin"), None);

        let preflight = send(
            &app,
            "OPTIONS",
            OPEN_ROUTE,
            &[("origin", EVIL), ("access-control-request-method", "GET")],
        )
        .await;
        assert_eq!(preflight.header("access-control-allow-origin"), None);
    }

    #[tokio::test(start_paused = true)]
    async fn leaves_a_request_with_no_origin_header_alone_curl_cy_request_the_ws_player_task() {
        let app = test_app().await;
        let response = send(&app, "GET", OPEN_ROUTE, &[]).await;
        assert_eq!(response.status, 200);
        assert_eq!(response.header("access-control-allow-origin"), None);
    }
}

mod get_api_catalog {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn serves_the_whole_card_defs_record_and_the_version_with_no_token() {
        // TS served a one-card catalog of its own; the Rust server serves the compiled-in catalog
        // (SURFACE §11.3), so the record it must serve whole is that one.
        let app = test_app().await;
        let (status, _headers, body) = call(&app, "GET", "/api/catalog", None, Value::Null).await;
        assert_eq!(status, 200);
        let defs: Value = serde_json::from_str(jackioh_cards::catalog_json()).expect("catalog.json");
        assert_eq!(
            body,
            json!({ "version": jackioh_cards::catalog_version(), "defs": defs })
        );
    }
}
