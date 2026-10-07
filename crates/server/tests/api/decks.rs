//! Saved decks and trios (`crates/server/src/api/decks.rs`, SPEC §9.4, R250–R256) and the
//! queue-time helpers the queue and the rooms share (`read_mode_choice`, `freeze_choice`,
//! `assert_not_in_series`; R253, R257, R264).
//!
//! What this file is NOT: a test of D1–D4, T1–T3 or L1–L6. Those rules live in the shared validator
//! (`jackioh_engine::validator`) and are tested there, rule by rule and message by message. Here the
//! questions are the endpoints' own: is the id a UUID, is the catalog current, are the draft issues
//! passed through untouched, what does each store outcome answer, and is another profile's id
//! indistinguishable from a missing one. Every expected issue below is the shared module's own
//! verdict for the same input, computed here, never a sentence typed out.
//!
//! The rulings, by their test titles:
//!  - R250: a save checks structure only (D1–D4), at most `MAX_SAVED_DECKS` decks;
//!  - R252: T1–T3, at most `MAX_SAVED_TRIOS` trios, a deleted deck empties its slots;
//!  - R256: `PUT` is an idempotent upsert keyed by the client's id;
//!  - R341: a trio import is checked like every save and written all or nothing, under both caps
//!    (R340);
//!  - R165: a profile with nothing saved is refused as a deck failure, not a missing resource.
//!
//! Ported from `apps/server/test/api/decks.test.ts` (part 18). TS ran on `createTestDeps()` — a
//! 24-card synthetic catalog and a validator port it could swap; the Rust test app serves the
//! compiled-in catalog and the handlers call the shared validator directly (SURFACE §11.3), so the
//! cards here are the real catalog's playable ids, and where TS recorded what its validator port
//! was asked, the test asserts what the real validator then answers. R257's legacy body (no `mode`,
//! a `deckIndex`) is gone (SURFACE §11.3), and its tests say so.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use jackioh_engine::config::DECK_SIZE;
use jackioh_engine::validator::{
    DeckDraftInput, check_deck_draft, check_import_room, check_trio_draft, normalize_name, validate_deck,
};
use jackioh_engine::wire::emotes::{PORTRAIT_IDS, is_portrait_id};
use jackioh_server::api::decks::{assert_not_in_series, freeze_choice, read_mode_choice};
use jackioh_server::api::http::{ApiError, ApiErrorCode, AuthLevel};
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{DECK_NAME_MAX_LENGTH, DRAFT_ISSUES_REPORTED_MAX, MAX_SAVED_DECKS, MAX_SAVED_TRIOS};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, Db, SavedDeck, SavedTrio, SeriesRow, StoreError};

use crate::support::deps::{add_user, call, test_app};

const PROFILE: &str = "p1";
const OTHER: &str = "p2";

/// A client-minted id (R256): a UUID, made readable by its last digits.
fn uuid(n: usize) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

// ---------------------------------------------------------------------------
// Harness (a private copy per file, SURFACE rule 5)
// ---------------------------------------------------------------------------

fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// `{ ...base, ...extra }` on JSON objects.
fn merged(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            into.insert(key, value);
        }
    }
    base
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

/// The fake store behind the test app (TS `deps.store`).
async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

/// One store call in a transaction of its own (TS called the store's methods bare).
macro_rules! store {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut t = $app.db.begin(None).await.expect("begin");
        let out = t.$method($($arg),*).await.unwrap_or_else(|error| panic!("{}: {error:?}", stringify!($method)));
        t.commit().await.expect("commit");
        out
    }};
}

/// The version the server's catalog is at (TS `deps.catalog.version`).
fn catalog_version() -> String {
    jackioh_cards::catalog_version().to_string()
}

/// TS `catalog.isToken`: a Token by flag or by tag.
fn is_token(card_id: &str) -> bool {
    jackioh_cards::CATALOG
        .get(card_id)
        .is_some_and(|def| def.token || def.tags.iter().any(|tag| tag.as_str() == "Token"))
}

/// Distinct playable ids of the catalog, from `start` (TS `cards(target, count, start)`).
fn cards(count: usize, start: usize) -> Vec<String> {
    jackioh_cards::CATALOG_IDS.iter().filter(|id| !is_token(id)).skip(start).take(count).cloned().collect()
}

/// One Token of the catalog (TS's `"token-sheep"`).
fn a_token() -> String {
    jackioh_cards::CATALOG_IDS.iter().find(|id| is_token(id)).cloned().expect("the catalog has a Token")
}

/// R250 D3, R251: a deckable card is one the current catalog has and that is not a Token.
fn is_deckable(card_id: &str) -> bool {
    jackioh_cards::CATALOG.contains_key(card_id) && !is_token(card_id)
}

/// The shared module's verdict for a deck draft, exactly as the route must pass it on.
fn draft_issues(name: &str, deck: &[String]) -> Value {
    let deckable = |card_id: &str| is_deckable(card_id);
    json_of(&check_deck_draft(&DeckDraftInput {
        name: normalize_name(name),
        cards: deck.to_vec(),
        is_deckable: &deckable,
        name_max_length: DECK_NAME_MAX_LENGTH,
        portrait: None,
        is_portrait: None,
    }))
}

/// The shared module's verdict for a trio draft.
fn trio_issues(name: &str, deck_ids: Value) -> Value {
    json_of(&check_trio_draft(&from(json!({
        "name": normalize_name(name),
        "deckIds": deck_ids,
        "nameMaxLength": DECK_NAME_MAX_LENGTH,
    }))))
}

fn first_message(issues: &Value) -> Value {
    issues[0]["message"].clone()
}

/// TS `beforeEach`: a fresh app with two active profiles.
struct Ctx {
    app: Arc<App>,
    token: String,
    other_token: String,
}

async fn active_profile(app: &App, id: &str) -> String {
    let user_id = format!("user-{id}");
    fake(app).await.seed_profile(json!({ "id": id, "userId": user_id, "status": "active" }));
    add_user(app, &user_id, &format!("{id}@example.test"), true)
}

async fn setup() -> Ctx {
    let app = test_app().await;
    let token = active_profile(&app, PROFILE).await;
    let other_token = active_profile(&app, OTHER).await;
    Ctx { app, token, other_token }
}

fn deck_body(overrides: Value) -> Value {
    merged(json!({ "name": "Aggro", "cards": cards(4, 0), "catalogVersion": catalog_version() }), overrides)
}

/// A trio import's body (R341): three decks of disjoint cards, ids from `base`, and the trio.
fn import_body(base: usize, overrides: Value) -> Value {
    let slots: Vec<Value> = (0..3)
        .map(|slot| {
            json!({ "id": uuid(base + 1 + slot), "name": format!("Imported {}", slot + 1), "cards": cards(3, slot * 3) })
        })
        .collect();
    merged(
        json!({ "catalogVersion": catalog_version(), "trio": { "id": uuid(base), "name": "Shared trio" }, "slots": slots }),
        overrides,
    )
}

impl Ctx {
    /// `support::deps::call`, whose body `Value::Null` sends none.
    async fn request(&self, method: &str, path: &str, body: Option<Value>, bearer: &str) -> (u16, Value) {
        let (status, _headers, answer) = call(&self.app, method, path, Some(bearer), body.unwrap_or(Value::Null)).await;
        (status, answer)
    }

    async fn put_deck(&self, body: Value, id: &str, bearer: &str) -> (u16, Value) {
        self.request("PUT", &format!("/api/decks/{id}"), Some(body), bearer).await
    }

    async fn put_trio(&self, body: Value, id: &str, bearer: &str) -> (u16, Value) {
        self.request("PUT", &format!("/api/trios/{id}"), Some(body), bearer).await
    }

    async fn post_import(&self, body: Value, bearer: &str) -> (u16, Value) {
        self.request("POST", "/api/trios/import", Some(body), bearer).await
    }

    async fn delete(&self, path: &str, bearer: &str) -> (u16, Value) {
        self.request("DELETE", path, None, bearer).await
    }

    async fn get_decks(&self, bearer: &str) -> Value {
        let (status, body) = self.request("GET", "/api/decks", None, bearer).await;
        assert_eq!(status, 200);
        body
    }

    async fn decks_table(&self) -> Vec<Value> {
        fake(&self.app).await.tables.decks.iter().map(json_of).collect()
    }

    async fn trios_table(&self) -> Vec<Value> {
        fake(&self.app).await.tables.trios.iter().map(json_of).collect()
    }
}

fn names(listed: &Value, list: &str, key: &str) -> Vec<Value> {
    listed[list].as_array().expect("a list").iter().map(|row| row[key].clone()).collect()
}

fn error_code(body: &Value) -> &Value {
    &body["error"]["code"]
}

// ---------------------------------------------------------------------------
// Decks
// ---------------------------------------------------------------------------

mod saved_decks_9_4_r250_r256 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r250_creates_a_deck_reads_it_back_and_replaces_it_keeping_when_it_was_made() {
        let ctx = setup().await;
        let (status, created) = ctx.put_deck(deck_body(json!({})), &uuid(1), &ctx.token).await;
        assert_eq!(status, 200);
        let first = created["deck"].clone();
        let now = now_ms();
        assert_eq!(
            first,
            json!({
                "id": uuid(1),
                "name": "Aggro",
                "cards": cards(4, 0),
                // R641: a body that names no portrait saves `null` — `vanilla` wherever it is shown.
                "portrait": null,
                "catalogVersion": catalog_version(),
                "createdAt": now,
                "updatedAt": now,
            })
        );
        // The owner is the caller; the answer carries no profile id (`SavedDeck` in apps/web's api.ts).
        assert!(first.get("profileId").is_none());

        tokio::time::advance(Duration::from_millis(5_000)).await;
        let (status, renamed) =
            ctx.put_deck(deck_body(json!({ "name": "  Aggro   v2 ", "cards": cards(2, 10) })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 200);
        let second = renamed["deck"].clone();
        // Stored as `normalize_name` leaves it; the same row, made when it was made, changed now.
        assert_eq!(second["name"], json!("Aggro v2"));
        assert_eq!(second["cards"], json!(cards(2, 10)));
        assert_eq!(second["createdAt"], first["createdAt"]);
        assert_eq!(second["updatedAt"], json!(now_ms()));

        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(listed["catalogVersion"], json!(catalog_version()));
        assert_eq!(listed["decks"], json!([second]));
        assert_eq!(listed["trios"], json!([]));
        assert_eq!(
            listed["limits"],
            json!({ "decks": MAX_SAVED_DECKS, "trios": MAX_SAVED_TRIOS, "nameLength": DECK_NAME_MAX_LENGTH })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r250_saves_a_draft_an_incomplete_deck_and_an_unowned_card_are_both_kept() {
        let ctx = setup().await;
        // Nothing is granted to this profile, and one card is far short of a legal deck: the save
        // judges neither (L2, L5 are the queue's), only D1–D4.
        let (status, _) = ctx.put_deck(deck_body(json!({ "cards": cards(1, 0) })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(json_of(&store!(ctx.app, collection_get(PROFILE))), json!([]));
        assert_eq!(ctx.decks_table().await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r256_is_an_idempotent_upsert_the_same_put_twice_makes_one_deck_not_two() {
        let ctx = setup().await;
        let body = deck_body(json!({}));
        let (_, once) = ctx.put_deck(body.clone(), &uuid(1), &ctx.token).await;
        // A retry after a dropped connection: same id, same body.
        let (status, again) = ctx.put_deck(body, &uuid(1), &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(again["deck"], once["deck"]);
        assert_eq!(ctx.decks_table().await.len(), 1);
        assert_eq!(ctx.get_decks(&ctx.token).await["decks"].as_array().map(Vec::len), Some(1));
    }

    #[tokio::test(start_paused = true)]
    async fn r256_answers_an_id_that_is_not_even_valid_percent_encoding_as_a_bad_id_400_never_a_500() {
        let ctx = setup().await;
        for raw in ["%E0%A4%A", "%ZZ", "%"] {
            let (put, _) = ctx.put_deck(deck_body(json!({})), raw, &ctx.token).await;
            assert_eq!(put, 400, "PUT /api/decks/{raw}");
            let (removed, _) = ctx.delete(&format!("/api/trios/{raw}"), &ctx.token).await;
            assert_eq!(removed, 400, "DELETE /api/trios/{raw}");
        }
        assert!(ctx.decks_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r256_lists_decks_oldest_first_the_order_a_legacy_deck_index_counts_in() {
        let ctx = setup().await;
        ctx.put_deck(deck_body(json!({ "name": "First" })), &uuid(9), &ctx.token).await;
        tokio::time::advance(Duration::from_millis(1_000)).await;
        ctx.put_deck(deck_body(json!({ "name": "Second" })), &uuid(3), &ctx.token).await;
        tokio::time::advance(Duration::from_millis(1_000)).await;
        // Updating the first does not move it: the order is by creation.
        ctx.put_deck(deck_body(json!({ "name": "First again" })), &uuid(9), &ctx.token).await;

        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(names(&listed, "decks", "name"), vec![json!("First again"), json!("Second")]);
    }

    #[tokio::test(start_paused = true)]
    async fn r250_lists_at_most_draft_issues_reported_max_issues_so_a_body_of_junk_cannot_buy_a_huge_answer() {
        let ctx = setup().await;
        let junk: Vec<String> = (0..DRAFT_ISSUES_REPORTED_MAX * 4).map(|i| format!("junk-{i}")).collect();
        let (status, body) = ctx.put_deck(deck_body(json!({ "cards": junk })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 400);
        assert_eq!(body["error"]["details"].as_array().map(Vec::len), Some(DRAFT_ISSUES_REPORTED_MAX));
        // The first issue is still the message: the one a player reads.
        assert_eq!(body["error"]["message"], first_message(&draft_issues("Aggro", &junk)));
        assert!(ctx.decks_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_a_stale_catalog_version_with_409_update_required_before_judging_the_cards() {
        let ctx = setup().await;
        // A card the stale client knows and this catalog does not: the useful answer is the update,
        // not D3's "not a card a deck can hold".
        let (status, body) = ctx
            .put_deck(
                deck_body(json!({ "catalogVersion": "stale-0", "cards": ["core-from-the-future"] })),
                &uuid(1),
                &ctx.token,
            )
            .await;
        assert_eq!(status, 409);
        assert_eq!(*error_code(&body), json!("update_required"));
        assert_eq!(body["error"]["message"], json!("update required"));
        assert!(ctx.decks_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r250_refuses_a_deck_past_max_saved_decks_with_a_409_naming_the_limit() {
        let ctx = setup().await;
        for n in 1..=MAX_SAVED_DECKS {
            let (status, _) = ctx.put_deck(deck_body(json!({ "name": format!("Deck {n}") })), &uuid(n), &ctx.token).await;
            assert_eq!(status, 200);
        }
        let (status, body) =
            ctx.put_deck(deck_body(json!({ "name": "One too many" })), &uuid(MAX_SAVED_DECKS + 1), &ctx.token).await;

        assert_eq!(status, 409);
        assert_eq!(*error_code(&body), json!("conflict"));
        assert_eq!(body["error"]["details"], json!({ "limit": MAX_SAVED_DECKS }));
        assert_eq!(ctx.decks_table().await.len(), MAX_SAVED_DECKS);

        // The control: at the cap, an existing deck can still be saved — the cap is on creating.
        let (status, _) = ctx.put_deck(deck_body(json!({ "name": "Renamed" })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn answers_another_profile_s_deck_id_as_404_and_leaves_that_deck_alone() {
        let ctx = setup().await;
        assert_eq!(ctx.put_deck(deck_body(json!({ "name": "Mine" })), &uuid(1), &ctx.other_token).await.0, 200);

        let (status, body) = ctx.put_deck(deck_body(json!({ "name": "Hijacked" })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 404);
        assert_eq!(*error_code(&body), json!("not_found"));
        assert_eq!(json_of(&store!(ctx.app, decks_get(&uuid(1))))["name"], json!("Mine"));

        // DELETE of it is "nothing to delete", exactly as for an id nobody has.
        let (_, removed) = ctx.delete(&format!("/api/decks/{}", uuid(1)), &ctx.token).await;
        assert_eq!(removed, json!({ "deleted": false }));
        let (_, missing) = ctx.delete(&format!("/api/decks/{}", uuid(77)), &ctx.token).await;
        assert_eq!(missing, json!({ "deleted": false }));
        assert_eq!(ctx.decks_table().await.len(), 1);
        // …and GET never lists it for the wrong profile.
        assert_eq!(ctx.get_decks(&ctx.token).await["decks"], json!([]));
    }

    #[tokio::test(start_paused = true)]
    async fn deletes_a_deck_idempotently() {
        let ctx = setup().await;
        ctx.put_deck(deck_body(json!({})), &uuid(1), &ctx.token).await;
        let (_, first) = ctx.delete(&format!("/api/decks/{}", uuid(1)), &ctx.token).await;
        assert_eq!(first, json!({ "deleted": true }));
        let (status, second) = ctx.delete(&format!("/api/decks/{}", uuid(1)), &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(second, json!({ "deleted": false }));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_an_id_that_is_not_a_uuid_and_a_malformed_body_before_touching_the_store() {
        let ctx = setup().await;
        let touched = Arc::new(AtomicUsize::new(0));
        let seen = touched.clone();
        fake(&ctx.app).await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
            if method.starts_with("decks.") || method.starts_with("trios.") {
                seen.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        }));
        for id in ["not-a-uuid".to_string(), "1234".to_string(), format!("{}x", uuid(1))] {
            assert_eq!(ctx.put_deck(deck_body(json!({})), &id, &ctx.token).await.0, 400, "{id}");
            assert_eq!(ctx.delete(&format!("/api/decks/{id}"), &ctx.token).await.0, 400, "{id}");
        }
        for body in [
            json!({ "cards": [], "catalogVersion": catalog_version() }),
            json!({ "name": 7, "cards": [], "catalogVersion": catalog_version() }),
            json!({ "name": "A", "cards": "core-001", "catalogVersion": catalog_version() }),
            json!({ "name": "A", "cards": [1, 2], "catalogVersion": catalog_version() }),
            json!({ "name": "A", "cards": [] }),
        ] {
            assert_eq!(ctx.put_deck(body.clone(), &uuid(1), &ctx.token).await.0, 400, "{body}");
        }
        assert_eq!(touched.load(Ordering::SeqCst), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn accepts_an_upper_case_uuid_as_the_same_id_postgres_would_print() {
        let ctx = setup().await;
        let upper = uuid(5).to_uppercase();
        assert_eq!(ctx.put_deck(deck_body(json!({})), &upper, &ctx.token).await.0, 200);
        assert_eq!(ctx.decks_table().await[0]["id"], json!(uuid(5)));
    }
}

/// R250's draft refusals, one per D-rule case (TS's table of `it`s).
mod r250_refuses_a_draft_that_breaks_d1_d4_with_the_shared_module_s_own_issues {
    use super::*;

    /// One case: the draft is refused for `rule`, as a 400 carrying every issue, and nothing is written.
    async fn refuses(rule: &str, name: &str, deck: Vec<String>) {
        let ctx = setup().await;
        let expected = draft_issues(name, &deck);
        // PREMISE: the shared module really refuses this draft, and for the rule named.
        let rules: Vec<Value> = expected.as_array().expect("issues").iter().map(|issue| issue["rule"].clone()).collect();
        assert!(rules.contains(&json!(rule)), "{rule} is among {rules:?}");

        let (status, body) = ctx.put_deck(deck_body(json!({ "name": name, "cards": deck })), &uuid(1), &ctx.token).await;

        assert_eq!(status, 400);
        assert_eq!(*error_code(&body), json!("bad_request"));
        assert_eq!(body["error"]["details"], expected);
        assert_eq!(body["error"]["message"], first_message(&expected));
        assert!(ctx.decks_table().await.is_empty());
    }

    fn with(mut deck: Vec<String>, more: &[String]) -> Vec<String> {
        deck.extend_from_slice(more);
        deck
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d1_case_1_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D1", "   ", cards(2, 0)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d1_case_2_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D1", &"x".repeat(DECK_NAME_MAX_LENGTH + 1), cards(2, 0)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d1_case_3_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D1", "Bad\u{0007}name", cards(2, 0)).await;
    }

    // A right-to-left override makes a name show text it does not hold; a zero-width space hides one.

    #[tokio::test(start_paused = true)]
    async fn r250_d1_case_4_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D1", "Aggro\u{202e}orez", cards(2, 0)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d1_case_5_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D1", "\u{200b}", cards(2, 0)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d2_case_6_is_a_400_carrying_every_issue_and_writes_nothing() {
        // One more than any deck holds: the catalog has room for it.
        refuses("D2", "Big", cards(DECK_SIZE as usize + 1, 0)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d3_case_7_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D3", "Token", with(cards(2, 0), &[a_token()])).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d3_case_8_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D3", "Unknown", with(cards(2, 0), &["core-does-not-exist".to_string()])).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r250_d4_case_9_is_a_400_carrying_every_issue_and_writes_nothing() {
        refuses("D4", "Twice", with(cards(2, 0), &cards(1, 0))).await;
    }
}

/// R641's `portrait` field and D5's check on it (§9.4): a saved deck may name one of the portrait
/// ids — or `null`, `vanilla` wherever the deck is shown — and nothing else.
mod the_deck_s_portrait_r641_d5 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn d5_saves_every_portrait_id_and_echoes_it_on_the_deck() {
        let ctx = setup().await;
        for (index, portrait) in PORTRAIT_IDS.iter().enumerate() {
            let body = merged(deck_body(json!({})), json!({ "name": format!("Portrait {portrait}"), "portrait": portrait }));
            let (status, saved) = ctx.put_deck(body, &uuid(10 + index), &ctx.token).await;
            assert_eq!(status, 200);
            assert_eq!(saved["deck"]["portrait"], json!(portrait));
            // The row keeps what was saved, so a later read back — or a freeze — meets it.
            assert_eq!(ctx.decks_table().await.last().map(|row| row["portrait"].clone()), Some(json!(portrait)));
        }
        assert_eq!(ctx.decks_table().await.len(), PORTRAIT_IDS.len());
    }

    #[tokio::test(start_paused = true)]
    async fn r641_accepts_portrait_null_and_absent_alike_both_read_back_null() {
        let ctx = setup().await;
        for (index, body) in [merged(deck_body(json!({})), json!({ "portrait": null })), deck_body(json!({}))]
            .into_iter()
            .enumerate()
        {
            let (status, saved) = ctx.put_deck(body, &uuid(20 + index), &ctx.token).await;
            assert_eq!(status, 200);
            assert!(saved["deck"]["portrait"].is_null());
        }
        // Both wrote `null` to the row; `null` is what a pre-portrait row reads as too.
        let portraits: Vec<Value> = ctx.decks_table().await.iter().map(|row| row["portrait"].clone()).collect();
        assert_eq!(portraits, vec![Value::Null, Value::Null]);
    }

    #[tokio::test(start_paused = true)]
    async fn d5_refuses_an_id_outside_the_roster_with_the_shared_module_s_own_issue_and_writes_nothing() {
        let ctx = setup().await;
        // PREMISE: the expected issue is the validator's own verdict for the same input — computed
        // here against the same `is_portrait_id` the route passes, never a sentence typed out.
        let deckable = |card_id: &str| is_deckable(card_id);
        let portrait = |value: &str| is_portrait_id(&Value::from(value));
        let expected = json_of(&check_deck_draft(&DeckDraftInput {
            name: normalize_name("Aggro"),
            cards: cards(4, 0),
            is_deckable: &deckable,
            name_max_length: DECK_NAME_MAX_LENGTH,
            portrait: Some("ulfric".to_string()),
            is_portrait: Some(&portrait),
        }));
        assert_eq!(expected, json!([{ "rule": "D5", "message": "\"portrait\" is not a known portrait id." }]));

        let (status, body) = ctx.put_deck(deck_body(json!({ "portrait": "ulfric" })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 400);
        assert_eq!(*error_code(&body), json!("bad_request"));
        assert_eq!(body["error"]["details"], expected);
        assert_eq!(body["error"]["message"], first_message(&expected));
        assert!(ctx.decks_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r641_refuses_a_portrait_that_is_not_a_string_and_keeps_a_saved_one_on_re_save() {
        let ctx = setup().await;
        let (status, body) = ctx.put_deck(deck_body(json!({ "portrait": 42 })), &uuid(1), &ctx.token).await;
        assert_eq!(status, 400);
        assert_eq!(*error_code(&body), json!("bad_request"));
        assert_eq!(body["error"]["message"], json!("\"portrait\" must be a string"));
        assert!(ctx.decks_table().await.is_empty());

        // A re-save names the new portrait; it lands on the same row.
        let (_, first) = ctx.put_deck(deck_body(json!({ "portrait": "gary" })), &uuid(1), &ctx.token).await;
        let id = first["deck"]["id"].as_str().expect("an id").to_string();
        let body = merged(deck_body(json!({})), json!({ "name": "Re-saved", "portrait": "felinors" }));
        let (status, again) = ctx.put_deck(body, &id, &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(again["deck"]["portrait"], json!("felinors"));
        assert_eq!(ctx.decks_table().await.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Trios
// ---------------------------------------------------------------------------

mod saved_trios_9_4_r252 {
    use super::*;

    async fn three_decks(ctx: &Ctx, bearer: &str, base: usize) -> [String; 3] {
        let ids = [uuid(base), uuid(base + 1), uuid(base + 2)];
        for (index, id) in ids.iter().enumerate() {
            let (status, _) = ctx.put_deck(deck_body(json!({ "name": format!("Deck {}", index + 1) })), id, bearer).await;
            assert_eq!(status, 200);
        }
        ids
    }

    #[tokio::test(start_paused = true)]
    async fn r252_saves_a_trio_of_three_slots_any_of_them_empty_and_reads_it_back() {
        let ctx = setup().await;
        let [a, _, c] = three_decks(&ctx, &ctx.token, 1).await;
        let (status, body) =
            ctx.put_trio(json!({ "name": " Main  trio ", "deckIds": [a, null, c] }), &uuid(101), &ctx.token).await;
        assert_eq!(status, 200);
        let saved = body["trio"].clone();
        let now = now_ms();
        assert_eq!(
            saved,
            json!({ "id": uuid(101), "name": "Main trio", "deckIds": [a, null, c], "createdAt": now, "updatedAt": now })
        );
        assert_eq!(ctx.get_decks(&ctx.token).await["trios"], json!([saved]));
    }

    #[tokio::test(start_paused = true)]
    async fn r252_saves_a_trio_whose_decks_share_cards_that_is_the_queue_s_to_judge_r253() {
        let ctx = setup().await;
        // `deck_body` gives every deck the same four cards, so all three decks overlap completely.
        let ids = three_decks(&ctx, &ctx.token, 1).await;
        assert_eq!(ctx.put_trio(json!({ "name": "Loose", "deckIds": ids }), &uuid(101), &ctx.token).await.0, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r256_re_puts_a_trio_idempotently() {
        let ctx = setup().await;
        let ids = three_decks(&ctx, &ctx.token, 1).await;
        let body = json!({ "name": "Trio", "deckIds": ids });
        let (_, once) = ctx.put_trio(body.clone(), &uuid(101), &ctx.token).await;
        let (status, again) = ctx.put_trio(body, &uuid(101), &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(again["trio"], once["trio"]);
        assert_eq!(ctx.trios_table().await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r252_refuses_t1_t3_with_the_shared_module_s_own_issues() {
        let ctx = setup().await;
        let [a, b, _] = three_decks(&ctx, &ctx.token, 1).await;
        let drafts = [
            ("", json!([a, b, null])),
            ("Two slots", json!([a, b])),
            ("Twice", json!([a, a, b])),
        ];
        for (name, deck_ids) in drafts {
            let expected = trio_issues(name, deck_ids.clone());
            assert_ne!(expected, json!([]));
            let (status, body) = ctx.put_trio(json!({ "name": name, "deckIds": deck_ids }), &uuid(101), &ctx.token).await;
            assert_eq!(status, 400);
            assert_eq!(*error_code(&body), json!("bad_request"));
            assert_eq!(body["error"]["details"], expected);
            assert_eq!(body["error"]["message"], first_message(&expected));
        }
        assert!(ctx.trios_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r252_refuses_a_trio_past_max_saved_trios_with_a_409_naming_the_limit() {
        let ctx = setup().await;
        let ids = three_decks(&ctx, &ctx.token, 1).await;
        for n in 1..=MAX_SAVED_TRIOS {
            let body = json!({ "name": format!("Trio {n}"), "deckIds": ids });
            assert_eq!(ctx.put_trio(body, &uuid(200 + n), &ctx.token).await.0, 200);
        }
        let (status, body) = ctx.put_trio(json!({ "name": "Too many", "deckIds": ids }), &uuid(300), &ctx.token).await;
        assert_eq!(status, 409);
        assert_eq!(*error_code(&body), json!("conflict"));
        assert_eq!(body["error"]["details"], json!({ "limit": MAX_SAVED_TRIOS }));
        assert_eq!(ctx.trios_table().await.len(), MAX_SAVED_TRIOS);
    }

    #[tokio::test(start_paused = true)]
    async fn r252_refuses_a_slot_naming_a_deck_this_profile_has_not_saved_with_details_unknown_deck() {
        let ctx = setup().await;
        let [a, b, _] = three_decks(&ctx, &ctx.token, 1).await;
        let [foreign, _, _] = three_decks(&ctx, &ctx.other_token, 50).await;

        for stranger in [uuid(999), foreign] {
            let (status, body) =
                ctx.put_trio(json!({ "name": "Trio", "deckIds": [a, b, stranger] }), &uuid(101), &ctx.token).await;
            assert_eq!(status, 409);
            assert_eq!(*error_code(&body), json!("conflict"));
            assert_eq!(body["error"]["details"], json!({ "unknownDeck": true }));
        }
        assert!(ctx.trios_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn answers_another_profile_s_trio_id_as_404() {
        let ctx = setup().await;
        let theirs = three_decks(&ctx, &ctx.other_token, 50).await;
        let body = json!({ "name": "Theirs", "deckIds": theirs });
        assert_eq!(ctx.put_trio(body, &uuid(101), &ctx.other_token).await.0, 200);
        let mine = three_decks(&ctx, &ctx.token, 1).await;

        let (status, _) = ctx.put_trio(json!({ "name": "Mine now", "deckIds": mine }), &uuid(101), &ctx.token).await;
        assert_eq!(status, 404);
        assert_eq!(json_of(&store!(ctx.app, trios_get(&uuid(101))))["name"], json!("Theirs"));
        let (_, removed) = ctx.delete(&format!("/api/trios/{}", uuid(101)), &ctx.token).await;
        assert_eq!(removed, json!({ "deleted": false }));
    }

    #[tokio::test(start_paused = true)]
    async fn r252_empties_every_slot_that_named_a_deleted_deck_and_keeps_the_trio() {
        let ctx = setup().await;
        let [a, b, c] = three_decks(&ctx, &ctx.token, 1).await;
        ctx.put_trio(json!({ "name": "Keeps", "deckIds": [a, b, c] }), &uuid(101), &ctx.token).await;
        ctx.put_trio(json!({ "name": "Also", "deckIds": [b, null, a] }), &uuid(102), &ctx.token).await;

        let (_, removed) = ctx.delete(&format!("/api/decks/{a}"), &ctx.token).await;
        assert_eq!(removed, json!({ "deleted": true }));

        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(names(&listed, "trios", "deckIds"), vec![json!([null, b, c]), json!([b, null, null])]);
    }

    #[tokio::test(start_paused = true)]
    async fn deletes_a_trio_idempotently_and_leaves_its_decks() {
        let ctx = setup().await;
        let ids = three_decks(&ctx, &ctx.token, 1).await;
        ctx.put_trio(json!({ "name": "Gone", "deckIds": ids }), &uuid(101), &ctx.token).await;
        let (_, first) = ctx.delete(&format!("/api/trios/{}", uuid(101)), &ctx.token).await;
        assert_eq!(first, json!({ "deleted": true }));
        let (_, second) = ctx.delete(&format!("/api/trios/{}", uuid(101)), &ctx.token).await;
        assert_eq!(second, json!({ "deleted": false }));
        assert_eq!(ctx.get_decks(&ctx.token).await["decks"].as_array().map(Vec::len), Some(3));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_slots_that_are_not_deck_ids_or_null() {
        let ctx = setup().await;
        for deck_ids in [json!("nope"), json!([1, null, null]), json!(["not-a-uuid", null, null])] {
            let (status, _) = ctx.put_trio(json!({ "name": "Bad", "deckIds": deck_ids }), &uuid(101), &ctx.token).await;
            assert_eq!(status, 400, "{deck_ids}");
        }
    }
}

// ---------------------------------------------------------------------------
// A trio import (R340, R341)
// ---------------------------------------------------------------------------

mod a_trio_import_9_4_r340_r341 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r341_makes_the_code_s_decks_and_the_trio_naming_them_in_one_request() {
        let ctx = setup().await;
        let (status, body) = ctx.post_import(import_body(500, json!({})), &ctx.token).await;
        assert_eq!(status, 200);
        let decks: Vec<Value> = body["decks"]
            .as_array()
            .expect("decks")
            .iter()
            .map(|deck| json!([deck["id"], deck["name"], deck["cards"]]))
            .collect();
        assert_eq!(
            decks,
            vec![
                json!([uuid(501), "Imported 1", cards(3, 0)]),
                json!([uuid(502), "Imported 2", cards(3, 3)]),
                json!([uuid(503), "Imported 3", cards(3, 6)]),
            ]
        );
        assert_matches(
            &body["trio"],
            &json!({ "id": uuid(500), "name": "Shared trio", "deckIds": [uuid(501), uuid(502), uuid(503)] }),
            "trio",
        );
        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(names(&listed, "decks", "id"), vec![json!(uuid(501)), json!(uuid(502)), json!(uuid(503))]);
        assert_eq!(names(&listed, "trios", "id"), vec![json!(uuid(500))]);
    }

    #[tokio::test(start_paused = true)]
    async fn r341_lists_the_imported_decks_in_their_slots_order_whatever_their_ids_and_after_the_decks_already_saved() {
        let ctx = setup().await;
        assert_eq!(ctx.put_deck(deck_body(json!({ "name": "Older" })), &uuid(999), &ctx.token).await.0, 200);
        tokio::time::advance(Duration::from_millis(1)).await;
        // Descending ids: a tie on the instant would list them backwards.
        let body = import_body(
            500,
            json!({
                "slots": [
                    { "id": uuid(803), "name": "First", "cards": cards(1, 0) },
                    { "id": uuid(802), "name": "Second", "cards": cards(1, 1) },
                    { "id": uuid(801), "name": "Third", "cards": cards(1, 2) },
                ],
            }),
        );
        assert_eq!(ctx.post_import(body, &ctx.token).await.0, 200);
        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(
            names(&listed, "decks", "name"),
            vec![json!("Older"), json!("First"), json!("Second"), json!("Third")]
        );
        assert_eq!(listed["trios"][0]["deckIds"], json!([uuid(803), uuid(802), uuid(801)]));
    }

    #[tokio::test(start_paused = true)]
    async fn r341_keeps_an_empty_slot_empty_unowned_cards_and_cards_the_decks_share_drafts_judged_at_queue() {
        let ctx = setup().await;
        let shared = cards(2, 0);
        let body = import_body(
            600,
            json!({
                "slots": [
                    { "id": uuid(601), "name": "One", "cards": shared },
                    null,
                    { "id": uuid(603), "name": "Three", "cards": shared },
                ],
            }),
        );
        let (status, saved) = ctx.post_import(body, &ctx.token).await;
        assert_eq!(status, 200);
        assert_eq!(saved["trio"]["deckIds"], json!([uuid(601), null, uuid(603)]));
        assert_eq!(names(&saved, "decks", "cards"), vec![json!(shared), json!(shared)]);
    }

    #[tokio::test(start_paused = true)]
    async fn r341_is_idempotent_the_same_ids_again_update_what_the_first_attempt_made_and_take_no_new_slot() {
        let ctx = setup().await;
        assert_eq!(ctx.post_import(import_body(500, json!({})), &ctx.token).await.0, 200);
        for n in 0..MAX_SAVED_DECKS - 3 {
            let (status, _) = ctx.put_deck(deck_body(json!({ "name": format!("Filler {n}") })), &uuid(700 + n), &ctx.token).await;
            assert_eq!(status, 200);
        }
        // At the deck cap now, and the retry still lands: its decks are already this profile's.
        assert_eq!(ctx.post_import(import_body(500, json!({})), &ctx.token).await.0, 200);
        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(listed["decks"].as_array().map(Vec::len), Some(MAX_SAVED_DECKS));
        assert_eq!(listed["trios"].as_array().map(Vec::len), Some(1));
    }

    #[tokio::test(start_paused = true)]
    async fn r340_refuses_an_import_past_the_deck_cap_with_exactly_the_slots_it_needs_and_writes_nothing() {
        let ctx = setup().await;
        for n in 0..MAX_SAVED_DECKS - 1 {
            let (status, _) = ctx.put_deck(deck_body(json!({ "name": format!("Deck {n}") })), &uuid(700 + n), &ctx.token).await;
            assert_eq!(status, 200);
        }
        let (status, body) = ctx.post_import(import_body(500, json!({})), &ctx.token).await;
        assert_eq!(status, 409);
        let room = json_of(&check_import_room(&from(json!({
            "saved": { "decks": MAX_SAVED_DECKS - 1, "trios": 0 },
            "limits": { "decks": MAX_SAVED_DECKS, "trios": MAX_SAVED_TRIOS },
            "adding": { "decks": 3, "trios": 1 },
        }))));
        assert_eq!(room["ok"], json!(false));
        assert_eq!(
            body["error"],
            json!({
                "code": "conflict",
                "message": format!("{} Nothing was imported.", room["message"].as_str().unwrap_or("")),
                "details": {
                    "decksShort": 2,
                    "triosShort": 0,
                    "limits": { "decks": MAX_SAVED_DECKS, "trios": MAX_SAVED_TRIOS },
                },
            })
        );
        let listed = ctx.get_decks(&ctx.token).await;
        assert_eq!(listed["decks"].as_array().map(Vec::len), Some(MAX_SAVED_DECKS - 1));
        assert_eq!(listed["trios"], json!([]));
    }

    #[tokio::test(start_paused = true)]
    async fn r340_refuses_an_import_past_the_trio_cap_the_same_way() {
        let ctx = setup().await;
        for n in 0..MAX_SAVED_TRIOS {
            let body = json!({ "name": format!("Trio {n}"), "deckIds": [null, null, null] });
            assert_eq!(ctx.put_trio(body, &uuid(800 + n), &ctx.token).await.0, 200);
        }
        let (status, body) = ctx.post_import(import_body(500, json!({})), &ctx.token).await;
        assert_eq!(status, 409);
        assert_matches(&body["error"]["details"], &json!({ "decksShort": 0, "triosShort": 1 }), "details");
        assert!(ctx.decks_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r341_rolls_every_deck_back_when_a_later_write_fails_so_a_failure_part_way_leaves_nothing() {
        let ctx = setup().await;
        let upserts = Arc::new(AtomicUsize::new(0));
        let counted = upserts.clone();
        fake(&ctx.app).await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
            if method == "trios.upsert" {
                return Err(StoreError::Other("the database went away".to_string()));
            }
            if method == "decks.upsert" {
                counted.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        }));
        let (status, _) = ctx.post_import(import_body(500, json!({})), &ctx.token).await;
        assert_eq!(status, 500);
        assert_eq!(upserts.load(Ordering::SeqCst), 3);
        fake(&ctx.app).await.on_call = None;
        assert!(ctx.decks_table().await.is_empty());
        assert!(ctx.trios_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r341_checks_what_the_client_sends_like_any_save_d1_d4_per_deck_t1_for_the_trio_the_catalog_the_shape() {
        let ctx = setup().await;
        let (status, unknown_card) = ctx
            .post_import(
                import_body(
                    500,
                    json!({
                        "slots": [
                            { "id": uuid(501), "name": "Fine", "cards": cards(2, 0) },
                            { "id": uuid(502), "name": "Bad", "cards": ["core-999"] },
                            null,
                        ],
                    }),
                ),
                &ctx.token,
            )
            .await;
        assert_eq!(status, 400);
        let issue = draft_issues("Bad", &["core-999".to_string()]);
        assert_eq!(
            unknown_card["error"]["message"],
            json!(format!("Deck 2 (“Bad”): {}", issue[0]["message"].as_str().unwrap_or("")))
        );

        let (status, no_name) =
            ctx.post_import(import_body(500, json!({ "trio": { "id": uuid(500), "name": "   " } })), &ctx.token).await;
        assert_eq!(status, 400);
        let trio_issue = trio_issues("", json!([null, null, null]));
        assert_eq!(no_name["error"]["message"], first_message(&trio_issue));

        let (status, stale) = ctx.post_import(import_body(500, json!({ "catalogVersion": "stale" })), &ctx.token).await;
        assert_eq!(status, 409);
        assert_eq!(*error_code(&stale), json!("update_required"));

        let (status, twice) = ctx
            .post_import(
                import_body(
                    500,
                    json!({
                        "slots": [
                            { "id": uuid(501), "name": "A", "cards": [] },
                            { "id": uuid(501), "name": "B", "cards": [] },
                            null,
                        ],
                    }),
                ),
                &ctx.token,
            )
            .await;
        assert_eq!(status, 400);
        assert_eq!(twice["error"]["message"], first_message(&trio_issues("T", json!([uuid(501), uuid(501), null]))));

        for bad in [
            import_body(500, json!({ "slots": [null, null] })),
            import_body(500, json!({ "slots": "three" })),
            import_body(500, json!({ "trio": { "id": "not-a-uuid", "name": "T" } })),
            import_body(500, json!({ "trio": null })),
            import_body(500, json!({ "slots": [{ "id": uuid(1), "name": 5, "cards": [] }, null, null] })),
            import_body(500, json!({ "slots": [{ "id": uuid(1), "name": "N", "cards": [7] }, null, null] })),
        ] {
            let (status, _) = ctx.post_import(bad.clone(), &ctx.token).await;
            assert_eq!(status, 400, "{bad}");
        }
        assert!(ctx.decks_table().await.is_empty());
        assert!(ctx.trios_table().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r341_answers_ids_another_profile_owns_as_missing_and_writes_nothing_of_the_import() {
        let ctx = setup().await;
        assert_eq!(ctx.put_deck(deck_body(json!({})), &uuid(502), &ctx.other_token).await.0, 200);
        let (status, _) = ctx.post_import(import_body(500, json!({})), &ctx.token).await;
        assert_eq!(status, 404);
        let ids: Vec<Value> = ctx.decks_table().await.iter().map(|row| row["id"].clone()).collect();
        assert_eq!(ids, vec![json!(uuid(502))]);
        assert!(ctx.trios_table().await.is_empty());
    }
}

// ---------------------------------------------------------------------------
// §9.4's gate
// ---------------------------------------------------------------------------

mod the_routes_9_4_a_pending_account_sees_no_decks {
    use super::*;

    #[test]
    fn declares_every_deck_and_trio_route_active() {
        // TS read `createDeckRoutes()`; the Rust server has one table, `app::ROUTES` of
        // `(method, path, AuthLevel, handler)` in `allRoutes()` order, and the deck routes are the
        // ones under these two prefixes.
        let routes: Vec<_> = app::ROUTES
            .iter()
            .filter(|route| route.1.starts_with("/api/decks") || route.1.starts_with("/api/trios"))
            .collect();
        let listed: Vec<String> = routes.iter().map(|route| format!("{} {}", route.0, route.1)).collect();
        assert_eq!(
            listed,
            vec![
                "GET /api/decks",
                "PUT /api/decks/:id",
                "DELETE /api/decks/:id",
                "PUT /api/trios/:id",
                "POST /api/trios/import",
                "DELETE /api/trios/:id",
            ]
        );
        assert!(routes.iter().all(|route| matches!(route.2, AuthLevel::Active)));
    }

    #[tokio::test(start_paused = true)]
    async fn a_pending_account_gets_403_from_each_and_nothing_is_written() {
        let ctx = setup().await;
        fake(&ctx.app).await.seed_profile(json!({ "id": "pending", "userId": "user-pending", "status": "pending" }));
        let pending = add_user(&ctx.app, "user-pending", "pending@example.test", true);

        let responses = [
            ctx.request("GET", "/api/decks", None, &pending).await,
            ctx.put_deck(deck_body(json!({})), &uuid(1), &pending).await,
            ctx.delete(&format!("/api/decks/{}", uuid(1)), &pending).await,
            ctx.put_trio(json!({ "name": "T", "deckIds": [null, null, null] }), &uuid(101), &pending).await,
            ctx.delete(&format!("/api/trios/{}", uuid(101)), &pending).await,
            ctx.post_import(import_body(500, json!({})), &pending).await,
        ];
        for (status, body) in &responses {
            assert_eq!(*status, 403);
            assert_eq!(*error_code(body), json!("account_pending"));
        }
        assert!(ctx.decks_table().await.is_empty());
        assert!(ctx.trios_table().await.is_empty());
    }
}

// ---------------------------------------------------------------------------
// The queue-time helpers
// ---------------------------------------------------------------------------

mod read_mode_choice_r257 {
    use super::*;

    fn read(body: Value) -> Result<Value, ApiError> {
        read_mode_choice(&body).map(|choice| json_of(&choice))
    }

    #[test]
    fn reads_the_three_modes() {
        assert_eq!(read(json!({ "mode": "bo1", "deckId": uuid(1) })).ok(), Some(json!({ "mode": "bo1", "deckId": uuid(1) })));
        assert_eq!(read(json!({ "mode": "bo3", "trioId": uuid(2) })).ok(), Some(json!({ "mode": "bo3", "trioId": uuid(2) })));
        assert_eq!(read(json!({ "mode": "random" })).ok(), Some(json!({ "mode": "random" })));
        // Fields another mode would need are not read, so a lobby that sends them does no harm.
        assert_eq!(read(json!({ "mode": "random", "deckId": "whatever" })).ok(), Some(json!({ "mode": "random" })));
    }

    #[test]
    fn r257_s_legacy_body_is_gone_no_mode_and_a_deck_index_are_refused_and_a_deck_index_beside_a_deck_id_is_not_read() {
        // TS read `{ deckId }` as Best of 1 and took a `deckIndex` in place of `deckId`. SURFACE
        // §11.3 drops both forms: the mode is always named and the deck always by id.
        for body in [json!({ "deckId": uuid(1) }), json!({ "deckIndex": 0 }), json!({ "mode": "bo1", "deckIndex": 4 })] {
            let refused = read(body.clone()).expect_err("a legacy body");
            assert!(matches!(refused.code, ApiErrorCode::BadRequest), "{body}");
        }
        assert_eq!(
            read(json!({ "mode": "bo1", "deckId": uuid(3), "deckIndex": 0 })).ok(),
            Some(json!({ "mode": "bo1", "deckId": uuid(3) }))
        );
    }

    #[test]
    fn refuses_anything_malformed_with_a_400() {
        for body in [
            json!({ "mode": "bo5" }),
            json!({ "mode": 1 }),
            json!({}),
            json!({ "mode": "bo1" }),
            json!({ "deckId": "not-a-uuid" }),
            json!({ "deckId": 7 }),
            json!({ "deckIndex": -1 }),
            json!({ "deckIndex": 1.5 }),
            json!({ "deckIndex": "0" }),
            json!({ "mode": "bo3" }),
            json!({ "mode": "bo3", "trioId": "nope" }),
            json!({ "mode": "bo3", "deckId": uuid(1) }),
        ] {
            let refused = read(body.clone()).expect_err("a malformed body");
            assert!(matches!(refused.code, ApiErrorCode::BadRequest), "{body}");
        }
    }
}

mod freeze_choice_r253_what_a_ticket_or_a_room_keeps {
    //! TS swapped in a validator that approved everything and recorded what it was asked. The Rust
    //! freeze calls the shared validator itself (SURFACE §11.3), so what TS read off the recording
    //! is read off the real verdict instead: a legal deck freezes, and an illegal one is refused in
    //! the validator's own words, naming the deck by its saved name.

    use super::*;

    fn saved_deck(id: &str, name: &str, deck: &[String], profile_id: &str, at: i64) -> SavedDeck {
        from(json!({
            "id": id,
            "profileId": profile_id,
            "name": name,
            "cards": deck,
            "portrait": null,
            "catalogVersion": "old-0",
            "createdAt": at,
            "updatedAt": at,
        }))
    }

    /// A legal deck of the catalog: `DECK_SIZE` distinct cards from `start`.
    fn legal(start: usize) -> Vec<String> {
        cards(DECK_SIZE as usize, start)
    }

    /// L5: the profile owns one copy of each card.
    async fn own(app: &App, profile_id: &str, deck: &[String]) {
        let entries: Vec<CollectionEntry> =
            from(json!(deck.iter().map(|card| json!({ "cardId": card, "quantity": 1 })).collect::<Vec<_>>()));
        store!(app, collection_upsert_quantities(profile_id, &entries));
    }

    async fn save(app: &App, deck: SavedDeck) {
        store!(app, decks_upsert(&deck, MAX_SAVED_DECKS as i64));
    }

    async fn freeze(app: &App, profile_id: &str, choice: Value) -> Result<Value, ApiError> {
        let choice = read_mode_choice(&choice).expect("a well-formed choice");
        freeze_choice(app, profile_id, &choice).await.map(|frozen| json_of(&frozen))
    }

    fn rules_of(error: &ApiError) -> Vec<Value> {
        error
            .details
            .as_ref()
            .and_then(Value::as_array)
            .map(|issues| issues.iter().map(|issue| issue["rule"].clone()).collect())
            .unwrap_or_default()
    }

    #[tokio::test(start_paused = true)]
    async fn r253_freezes_a_best_of_1_deck_by_id_checked_as_one_deck_with_its_own_name() {
        let ctx = setup().await;
        let deck = legal(0);
        own(&ctx.app, PROFILE, &deck).await;
        save(&ctx.app, saved_deck(&uuid(1), "Aggro", &deck, PROFILE, 0)).await;

        let frozen = freeze(&ctx.app, PROFILE, json!({ "mode": "bo1", "deckId": uuid(1) })).await.expect("frozen");

        // R642: the deck's portrait freezes with it (R641's `null` here — `vanilla` when dealt). The
        // deck was saved under "old-0": the CURRENT catalog judges it, whatever it was saved under.
        assert_eq!(frozen, json!({ "mode": "bo1", "deck": { "name": "Aggro", "cards": deck, "portrait": null } }));

        // One deck, by its own name: a short one is refused by L2 in a sentence naming "Short", and
        // never by L1, which only a trio answers to.
        let short = cards(3, 0);
        save(&ctx.app, saved_deck(&uuid(2), "Short", &short, PROFILE, 0)).await;
        let refused = freeze(&ctx.app, PROFILE, json!({ "mode": "bo1", "deckId": uuid(2) })).await.expect_err("short");
        assert!(matches!(refused.code, ApiErrorCode::LoadoutInvalid));
        assert!(refused.message.contains("Short"), "{}", refused.message);
        let rules = rules_of(&refused);
        assert!(rules.contains(&json!("L2")) && !rules.contains(&json!("L1")), "{rules:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn r253_freezes_a_copy_editing_the_saved_deck_afterwards_cannot_reach_it() {
        let ctx = setup().await;
        let deck = legal(0);
        own(&ctx.app, PROFILE, &deck).await;
        save(&ctx.app, saved_deck(&uuid(1), "Aggro", &deck, PROFILE, 0)).await;
        let frozen = freeze(&ctx.app, PROFILE, json!({ "mode": "bo1", "deckId": uuid(1) })).await.expect("frozen");
        save(&ctx.app, saved_deck(&uuid(1), "Changed", &cards(3, 10), PROFILE, 0)).await;
        assert_eq!(frozen, json!({ "mode": "bo1", "deck": { "name": "Aggro", "cards": deck, "portrait": null } }));
    }

    #[tokio::test(start_paused = true)]
    async fn r642_freezes_the_deck_s_portrait_with_it_a_later_re_save_cannot_reach_the_copy() {
        let ctx = setup().await;
        let deck = legal(0);
        own(&ctx.app, PROFILE, &deck).await;
        let gary_deck = merged(json_of(&saved_deck(&uuid(1), "Gary's", &deck, PROFILE, 0)), json!({ "portrait": "gary" }));
        save(&ctx.app, from(gary_deck.clone())).await;

        let frozen = freeze(&ctx.app, PROFILE, json!({ "mode": "bo1", "deckId": uuid(1) })).await.expect("frozen");
        // The saved deck's portrait changes afterwards; the frozen one does not.
        save(&ctx.app, from(merged(gary_deck, json!({ "portrait": "timmy" })))).await;
        assert_eq!(frozen, json!({ "mode": "bo1", "deck": { "name": "Gary's", "cards": deck, "portrait": "gary" } }));
    }

    #[tokio::test(start_paused = true)]
    async fn r165_makes_queueing_with_nothing_saved_a_deck_failure_422_not_a_missing_resource() {
        // TS asked for `deckIndex: 0` of an empty list; the Rust choice names a deck by id, and with
        // nothing saved that id names nothing, which is the same refusal.
        let ctx = setup().await;
        let error = freeze(&ctx.app, PROFILE, json!({ "mode": "bo1", "deckId": uuid(1) })).await.expect_err("refused");
        assert!(matches!(error.code, ApiErrorCode::LoadoutInvalid));
        assert!(!matches!(error.code, ApiErrorCode::NotFound));
        assert!(error.message.to_lowercase().contains("deck"), "{}", error.message);
        // The premise: this profile really has saved nothing.
        assert_eq!(json_of(&store!(ctx.app, decks_list(PROFILE))), json!([]));
    }

    #[tokio::test(start_paused = true)]
    async fn r165_answers_a_deck_or_trio_that_is_gone_or_is_another_profile_s_the_same_way() {
        let ctx = setup().await;
        save(&ctx.app, saved_deck(&uuid(1), "Theirs", &cards(2, 0), OTHER, 0)).await;
        for choice in [
            json!({ "mode": "bo1", "deckId": uuid(1) }),
            json!({ "mode": "bo1", "deckId": uuid(2) }),
            json!({ "mode": "bo3", "trioId": uuid(3) }),
        ] {
            let error = freeze(&ctx.app, PROFILE, choice.clone()).await.expect_err("refused");
            assert!(matches!(error.code, ApiErrorCode::LoadoutInvalid), "{choice}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r253_passes_the_validator_s_issues_through_as_a_422_first_sentence_as_the_message() {
        let ctx = setup().await;
        // Short (L2) and holding a card this profile does not own (L5): TS injected two issues of
        // this shape; here they are the shared validator's own verdict on the deck.
        let deck = cards(3, 0);
        own(&ctx.app, PROFILE, &deck[1..]).await;
        save(&ctx.app, saved_deck(&uuid(1), "Aggro", &deck, PROFILE, 0)).await;
        let collection: IndexMap<String, i32> = deck[1..].iter().map(|card| (card.clone(), 1)).collect();
        let catalog: Value = serde_json::from_str(jackioh_cards::catalog_json()).expect("catalog.json");
        let verdict = json_of(&validate_deck(&from(json!({
            "deck": { "name": "Aggro", "cards": deck },
            "catalog": { "version": catalog_version(), "cards": catalog, "banned": [] },
            "collection": collection,
        }))));
        let issues = verdict["errors"].clone();

        let error = freeze(&ctx.app, PROFILE, json!({ "mode": "bo1", "deckId": uuid(1) })).await.expect_err("refused");
        assert!(matches!(error.code, ApiErrorCode::LoadoutInvalid));
        assert_eq!(json!(error.message), issues[0]["message"]);
        assert_eq!(error.details, Some(issues.clone()));
        let rules = rules_of(&error);
        assert!(rules.contains(&json!("L2")) && rules.contains(&json!("L5")), "{rules:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn r253_freezes_a_trio_s_three_decks_in_slot_order_checked_together_with_their_names() {
        let ctx = setup().await;
        let [one, two, three] = [legal(0), legal(DECK_SIZE as usize), legal(2 * DECK_SIZE as usize)];
        for deck in [&one, &two, &three] {
            own(&ctx.app, PROFILE, deck).await;
        }
        save(&ctx.app, saved_deck(&uuid(1), "One", &one, PROFILE, 0)).await;
        save(&ctx.app, saved_deck(&uuid(2), "Two", &two, PROFILE, 0)).await;
        save(&ctx.app, saved_deck(&uuid(3), "Three", &three, PROFILE, 0)).await;
        let trio: SavedTrio = from(json!({
            "id": uuid(9),
            "profileId": PROFILE,
            "name": "Main",
            "deckIds": [uuid(3), uuid(1), uuid(2)],
            "createdAt": 0,
            "updatedAt": 0,
        }));
        store!(ctx.app, trios_upsert(&trio, MAX_SAVED_TRIOS as i64));

        let frozen = freeze(&ctx.app, PROFILE, json!({ "mode": "bo3", "trioId": uuid(9) })).await.expect("frozen");

        assert_eq!(
            frozen,
            json!({
                "mode": "bo3",
                "trio": {
                    "name": "Main",
                    "decks": [
                        { "name": "Three", "cards": three, "portrait": null },
                        { "name": "One", "cards": one, "portrait": null },
                        { "name": "Two", "cards": two, "portrait": null },
                    ],
                },
            })
        );

        // Checked together, each by its own name: cut "Three" short and the trio is refused in a
        // sentence that names it.
        save(&ctx.app, saved_deck(&uuid(3), "Three", &three[..3], PROFILE, 0)).await;
        let refused = freeze(&ctx.app, PROFILE, json!({ "mode": "bo3", "trioId": uuid(9) })).await.expect_err("short");
        assert!(matches!(refused.code, ApiErrorCode::LoadoutInvalid));
        assert!(refused.message.contains("Three"), "{}", refused.message);
    }

    #[tokio::test(start_paused = true)]
    async fn r253_hands_the_validator_only_the_filled_slots_so_an_empty_one_is_l1_s_to_refuse() {
        let ctx = setup().await;
        let deck = legal(0);
        own(&ctx.app, PROFILE, &deck).await;
        save(&ctx.app, saved_deck(&uuid(1), "One", &deck, PROFILE, 0)).await;
        let trio: SavedTrio = from(json!({
            "id": uuid(9),
            "profileId": PROFILE,
            "name": "Gappy",
            "deckIds": [uuid(1), null, null],
            "createdAt": 0,
            "updatedAt": 0,
        }));
        store!(ctx.app, trios_upsert(&trio, MAX_SAVED_TRIOS as i64));
        // A frozen trio with a hole in it is never the answer: the one filled deck goes to the
        // validator alone, and L1 refuses a trio of one.
        let refused = freeze(&ctx.app, PROFILE, json!({ "mode": "bo3", "trioId": uuid(9) })).await.expect_err("gappy");
        assert!(matches!(refused.code, ApiErrorCode::LoadoutInvalid));
        assert!(rules_of(&refused).contains(&json!("L1")), "{:?}", rules_of(&refused));
    }

    #[tokio::test(start_paused = true)]
    async fn r258_freezes_nothing_for_all_random_and_asks_the_validator_nothing() {
        let ctx = setup().await;
        // Nothing is saved or owned: All Random needs neither.
        assert_eq!(freeze(&ctx.app, PROFILE, json!({ "mode": "random" })).await.expect("frozen"), json!({ "mode": "random" }));
    }
}

mod assert_not_in_series_r264 {
    use super::*;

    fn series(status: &str) -> SeriesRow {
        let trio = json!({ "name": "T", "decks": [{ "name": "a", "cards": [] }, { "name": "b", "cards": [] }, { "name": "c", "cards": [] }] });
        from(json!({
            "id": format!("series-{status}"),
            "sides": [
                { "profileId": PROFILE, "trio": trio, "wins": 0, "pick": null },
                { "profileId": OTHER, "trio": trio, "wins": 0, "pick": null },
            ],
            "catalogVersion": catalog_version(),
            "ranked": false,
            "seedBase": "seed",
            "status": status,
            "games": [],
            "nextMatchId": format!("match-{status}"),
            "pickDeadline": null,
            "winner": null,
            "endReason": null,
            "ratingBefore": null,
            "ratingAfter": null,
            "createdAt": 0,
            "updatedAt": 0,
            "endedAt": null,
            "version": 1,
        }))
    }

    #[tokio::test(start_paused = true)]
    async fn r264_refuses_a_profile_in_a_series_that_is_not_over_naming_the_series() {
        let ctx = setup().await;
        store!(ctx.app, series_create(&series("picking")));
        let error = assert_not_in_series(&ctx.app, PROFILE).await.expect_err("in a series");
        assert!(matches!(error.code, ApiErrorCode::AlreadyInMatch));
        assert_eq!(error.message, "Finish your Conquest series first.");
        assert_eq!(error.details, Some(json!({ "seriesId": "series-picking" })));
    }

    #[tokio::test(start_paused = true)]
    async fn r264_s_control_a_series_that_is_over_holds_nobody() {
        let ctx = setup().await;
        store!(ctx.app, series_create(&series("over")));
        assert!(assert_not_in_series(&ctx.app, PROFILE).await.is_ok());
    }
}
