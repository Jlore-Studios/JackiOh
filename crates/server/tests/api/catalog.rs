//! Port of `apps/server/test/api/catalog.test.ts`: the catalog and validator bindings
//! (`src/api/catalog.rs`, and `jackioh_engine::validator`, which the handlers call directly): the two
//! places the server reaches data and rules that live in other crates (SPEC §9.4).
//!
//! Two SPEC §11 rulings live here as well:
//!   * R163 — the catalog endpoint a client that ships none can read: whole, unprojected,
//!     unauthenticated, carrying R105's version.
//!   * R164 — where L6's ban list lives: server state, never a flag on a card definition, read
//!     through the catalog handle.
//!
//! What the Rust server does differently, and so what changed here (SURFACE §11.3, part 18's brief):
//!   * the catalog is compiled in (`jackioh_cards::catalog_json()`), so nothing is read from a file
//!     at run time: TS's "refuses to invent a catalog when the file is missing or malformed" is
//!     `crates/cards/build.rs` failing the build, and has no run-time test;
//!   * its version is `jackioh_cards::catalog_version()`, and `CATALOG_VERSION` must equal it or the
//!     server refuses to boot, so "the environment pins the version" became that refusal;
//!   * `loadout-validator.ts` is gone: the binding tests hand `jackioh_engine::validator` the snapshot
//!     a handler builds from the catalog handle (`snapshot` below), JSON in and JSON out (§10.1);
//!   * `GET /api/catalog/:version` is not served, so of R388's four tests only the 404 one remains.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Request};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tower::ServiceExt;

use jackioh_engine::{CardDefs, validator};
use jackioh_server::api::catalog::{self as catalog_api, Catalog, DEPLOYED_COMMIT_HEADER};
use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App};
use jackioh_server::env::load_env;

// ---------------------------------------------------------------------------
// Harness (the parts of `test/fakes/deps.ts` this file used)
// ---------------------------------------------------------------------------

/// The environment `createTestDeps()` stood for: `E2E=1` (the fake store, the fixture auth and
/// R144's fixtures) at the compiled-in catalog version, built by the same two calls `main.rs` makes.
fn test_env() -> IndexMap<String, String> {
    let mut source = IndexMap::new();
    for (name, value) in [("E2E", "1"), ("NODE_ENV", "test"), ("TRUSTED_PROXY_HOPS", "1")] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

async fn app_from(source: &IndexMap<String, String>) -> Arc<App> {
    let env = app::load_server_env(source).expect("the test environment loads");
    app::build(env).await.expect("the test app builds")
}

async fn test_app() -> Arc<App> {
    app_from(&test_env()).await
}

/// `createTestDeps({ catalog })`: the same server, holding `catalog` instead of the one it built.
async fn app_holding(catalog: Catalog) -> Arc<App> {
    let built = Arc::try_unwrap(test_app().await)
        .ok()
        .expect("app::build keeps no second handle on the App it returns");
    Arc::new(App { catalog, ..built })
}

/// One response, read whole: the status, the headers and the body's bytes as text.
struct Reply {
    status: u16,
    headers: HeaderMap,
    text: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).expect("the body is JSON")
    }
}

/// `jsonRequest("GET", path, undefined, { token })` through the whole router (CORS included).
async fn get(app: &Arc<App>, path: &str, token: Option<&str>) -> Reply {
    let mut builder = Request::builder()
        .method("GET")
        .uri(path)
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.7");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let request = builder.body(Body::empty()).expect("a well-formed request");
    let response = app::router(app.clone()).oneshot(request).await.expect("the router always answers");
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");
    Reply { status, headers, text: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8") }
}

fn from_json<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("the literal has the port's JSON shape")
}

/// The catalog file the server loads, as raw defs, so the tests below count it and not a transcription.
fn catalog_file() -> Value {
    serde_json::from_str(jackioh_cards::catalog_json()).expect("catalog.json is JSON")
}

fn sorted_keys(object: &Value) -> Vec<String> {
    let mut keys: Vec<String> = object.as_object().map(|map| map.keys().cloned().collect()).unwrap_or_default();
    keys.sort();
    keys
}

/// The first `count` ids of the catalog that are not tokens, in catalog order.
fn legal(catalog: &Catalog, count: usize) -> Vec<String> {
    catalog.card_ids.iter().filter(|id| !catalog.is_token(id)).take(count).cloned().collect()
}

/// `new Map(ids.map((cardId) => [cardId, 1]))`: one copy of each, as the validator's collection.
fn one_of_each(ids: &[String]) -> Value {
    Value::Object(ids.iter().map(|id| (id.clone(), json!(1))).collect())
}

/// What a handler hands the shared validator for this catalog handle (what TS's
/// `sharedLoadoutValidator` adapter built): its version, every definition, and the ids the handle
/// bans. Bannedness is read through the handle and nowhere else (R164).
fn snapshot(catalog: &Catalog) -> Value {
    let banned: Vec<String> = catalog.card_ids.iter().filter(|id| catalog.is_banned(id)).cloned().collect();
    json!({ "version": catalog.version, "cards": catalog_file(), "banned": banned })
}

/// The validator's verdict as TS's `LoadoutIssue[]`: empty for `{ ok: true }`, else its errors.
fn issues(result: impl Serialize) -> Vec<Value> {
    let value = serde_json::to_value(&result).expect("a verdict serialises");
    if value.get("ok") == Some(&json!(true)) || value.get("Ok").is_some() {
        return Vec::new();
    }
    value
        .get("errors")
        .or_else(|| value.get("Err"))
        .and_then(Value::as_array)
        .cloned()
        .expect("a refusal carries its errors")
}

/// `scope: "trio"` (the default): L1–L6 over `decks` (`validateLoadout`, R253).
fn validate_trio(decks: Value, catalog: &Catalog, owned: Value) -> Vec<Value> {
    issues(validator::validate_loadout(&from_json(json!({
        "decks": decks,
        "catalog": snapshot(catalog),
        "collection": owned,
    }))))
}

/// `scope: "deck"`: one Best-of-1 deck on the one-deck rules L2, L3, L5 and L6 (`validateDeck`, R253).
fn validate_one(deck: Value, catalog: &Catalog, owned: Value) -> Vec<Value> {
    issues(validator::validate_deck(&from_json(json!({
        "deck": deck,
        "catalog": snapshot(catalog),
        "collection": owned,
    }))))
}

/// Three unnamed decks, as `decks: string[][]` reached the adapter.
fn decks(lists: &[Vec<String>]) -> Value {
    Value::Array(lists.iter().map(|cards| json!({ "cards": cards })).collect())
}

fn rules(found: &[Value]) -> Vec<String> {
    found.iter().map(|issue| issue["rule"].as_str().unwrap_or_default().to_string()).collect()
}

/// A complete environment, so `load_env` is asked only about the one variable a test changes.
fn valid_env() -> IndexMap<String, String> {
    let mut source = IndexMap::new();
    for (name, value) in [
        ("SUPABASE_URL", "https://project.supabase.test"),
        ("SUPABASE_SECRET_KEY", "sb_secret_0123456789abcdefghijklmnopqrstuv"),
        ("DATABASE_URL", "postgres://postgres:postgres@localhost:5432/jackioh"),
        ("CODE_PEPPER", "a-pepper-of-at-least-thirty-two-characters"),
        ("PUBLIC_ORIGINS", "https://play.jackioh.test"),
        ("NODE_ENV", "test"),
    ] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

// ---------------------------------------------------------------------------
// catalog (`mod loaded_catalog`: a `mod catalog` inside `catalog.rs` is clippy's module_inception)
// ---------------------------------------------------------------------------

mod loaded_catalog {
    use super::*;

    #[tokio::test]
    async fn loads_crates_cards_catalog_json_compiled_in() {
        let app = test_app().await;
        // §8, §7 and patch v0.2.0 (B2.1): every entry the shipped file holds, across its sets.
        let mut ids = app.catalog.card_ids.clone();
        ids.sort();
        assert_eq!(ids, sorted_keys(&catalog_file()));
        let served = get(&app, "/api/catalog", None).await.json();
        assert!(!served["defs"]["core-001"]["name"].as_str().unwrap_or_default().is_empty());
    }

    #[test]
    fn derives_a_version_from_the_catalog_s_own_bytes_so_it_cannot_drift_from_the_data() {
        let version = catalog_api::version_of("{}");
        // /^c1-[0-9a-f]{12}$/
        let hex = version.strip_prefix("c1-").expect("the c1- prefix");
        assert_eq!(hex.len(), 12, "{version}");
        assert!(hex.chars().all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch)), "{version}");
        assert_eq!(catalog_api::version_of("{}"), catalog_api::version_of("{}"));
        assert_ne!(catalog_api::version_of("{}"), catalog_api::version_of("{ }"));
    }

    /// TS let `loadCatalog({ version })` stamp any version. The compiled-in catalog has one version,
    /// and §9.4's "both halves must agree" is now the boot refusal of SURFACE §11.3.
    #[tokio::test]
    async fn lets_the_environment_pin_the_version_only_to_the_compiled_in_one_section_9_4_both_halves_must_agree() {
        let app = test_app().await;
        assert_eq!(app.catalog.version, jackioh_cards::catalog_version());

        let mut stale = valid_env();
        stale.insert("CATALOG_VERSION".to_string(), "core-2026-09".to_string());
        match load_env(&stale) {
            Ok(_) => panic!("a CATALOG_VERSION other than the compiled-in one must be refused"),
            Err(problems) => assert!(problems.to_string().contains("CATALOG_VERSION"), "{problems}"),
        }
    }

    #[tokio::test]
    async fn marks_tokens_as_tokens_section_9_4_l3_no_token_tagged_cards_in_a_deck() {
        let app = test_app().await;
        let raw = catalog_file();
        let mut tokens: Vec<String> =
            app.catalog.card_ids.iter().filter(|id| app.catalog.is_token(id)).cloned().collect();
        tokens.sort();
        let mut flagged: Vec<String> = sorted_keys(&raw)
            .into_iter()
            .filter(|id| raw[id.as_str()]["token"] == json!(true))
            .collect();
        flagged.sort();
        assert_eq!(tokens, flagged);
    }

    #[test]
    fn catalog_from_keeps_the_ban_hook_available_for_l6() {
        let catalog = catalog_api::catalog_from(CardDefs::default(), "v0");
        assert!(catalog.card_ids.is_empty());
        assert!(!catalog.is_banned("core-001"));
    }
}

// ---------------------------------------------------------------------------
// The loadout validator binding (§9.4: one module, shared)
// ---------------------------------------------------------------------------

mod loadout_validator_binding_section_9_4_one_module_shared {
    use super::*;

    #[tokio::test]
    async fn adapts_the_shared_module_s_verdict_without_restating_a_rule() {
        let app = test_app().await;
        let legal = legal(&app.catalog, 60);
        let owned = one_of_each(&legal);

        let found = validate_trio(
            decks(&[legal[0..20].to_vec(), legal[20..40].to_vec(), legal[40..60].to_vec()]),
            &app.catalog,
            owned.clone(),
        );
        assert_eq!(found, Vec::<Value>::new());

        // One deck short of DECK_SIZE: the shared module names the rule, the deck and the card.
        let short = validate_trio(
            decks(&[legal[0..19].to_vec(), legal[20..40].to_vec(), legal[40..60].to_vec()]),
            &app.catalog,
            owned,
        );
        assert!(rules(&short).contains(&"L2".to_string()));
        assert!(!short[0]["message"].as_str().unwrap_or_default().is_empty());
    }

    #[tokio::test]
    async fn r253_checks_one_best_of_1_deck_on_the_one_deck_rules_when_the_port_asks_for_scope_deck() {
        let app = test_app().await;
        let legal = legal(&app.catalog, 20);
        let owned = one_of_each(&legal);

        // One legal deck: as a trio it fails L1 (one deck, not three); as a deck it passes.
        assert_eq!(rules(&validate_trio(decks(std::slice::from_ref(&legal)), &app.catalog, owned.clone())), vec!["L1"]);
        assert_eq!(validate_one(json!({ "cards": legal }), &app.catalog, owned.clone()), Vec::<Value>::new());

        // One card short: L2, naming the deck by the name the player gave it.
        let short = validate_one(json!({ "name": "Midrange", "cards": legal[0..19] }), &app.catalog, owned);
        assert_eq!(rules(&short), vec!["L2"]);
        assert_eq!(short[0]["deck"], json!(1));
        assert!(short[0]["message"].as_str().unwrap_or_default().contains("Midrange"));
    }

    #[tokio::test]
    async fn r253_names_a_trio_s_decks_in_its_messages_when_the_port_passes_their_names() {
        let app = test_app().await;
        let legal = legal(&app.catalog, 60);
        let owned = one_of_each(&legal);
        let shared = legal[0].clone();
        let mut second = vec![shared.clone()];
        second.extend_from_slice(&legal[21..40]);
        let found = validate_trio(
            json!([
                { "name": "Aggro", "cards": legal[0..20] },
                { "name": "Control", "cards": second },
                { "name": "Tempo", "cards": legal[40..60] },
            ]),
            &app.catalog,
            owned,
        );
        let l4 = found.iter().find(|issue| issue["rule"] == json!("L4")).expect("an L4");
        assert_eq!(l4["cardId"], json!(shared));
        let message = l4["message"].as_str().unwrap_or_default();
        assert!(message.contains("Aggro"), "{message}");
        assert!(message.contains("Control"), "{message}");
        assert!(!message.contains("Deck 1"), "{message}");
    }

    #[tokio::test]
    async fn reports_a_card_the_profile_does_not_own_l5_rather_than_silently_allowing_it() {
        let app = test_app().await;
        let legal = legal(&app.catalog, 60);
        let found = validate_trio(
            decks(&[legal[0..20].to_vec(), legal[20..40].to_vec(), legal[40..60].to_vec()]),
            &app.catalog,
            json!({}),
        );
        assert!(rules(&found).contains(&"L5".to_string()));
    }

    #[tokio::test]
    async fn r141_makes_l5_unreachable_on_its_own_given_r111_s_launch_grant_of_one_copy_of_each() {
        let app = test_app().await;
        let catalog = &app.catalog;
        let legal = legal(catalog, 60);
        // R111's launch grant, exactly: one copy of every non-token card, which is what every active
        // profile owns. The test above owns *nothing*, which R111 makes impossible — so L5 on its own
        // is only reachable there, never in a real collection.
        let every: Vec<String> = catalog.card_ids.iter().filter(|id| !catalog.is_token(id)).cloned().collect();
        let owned = one_of_each(&every);
        let token = catalog.card_ids.iter().find(|id| catalog.is_token(id)).cloned().unwrap_or_default();
        let with_first = |first: Vec<String>| decks(&[first, legal[20..40].to_vec(), legal[40..60].to_vec()]);
        let nineteen_and = |last: &str| {
            let mut first = legal[0..19].to_vec();
            first.push(last.to_string());
            first
        };
        let mut in_two = vec![legal[0].clone()];
        in_two.extend_from_slice(&legal[21..40]);

        let ways = [
            // A second copy of a card: L3 (MAX_COPIES) is broken before L5 can be.
            ("a repeated card", with_first(nineteen_and(legal[0].as_str()))),
            // The same card in two decks: L4.
            ("a card in two decks", decks(&[legal[0..20].to_vec(), in_two, legal[40..60].to_vec()])),
            // A Token, and an id the catalog does not have: L3 and L6.
            ("a token", with_first(nineteen_and(token.as_str()))),
            ("an unknown id", with_first(nineteen_and("core-does-not-exist"))),
        ];

        let mut saw_l5 = false;
        for (name, way) in ways {
            let found = rules(&validate_trio(way, catalog, owned.clone()));

            assert_ne!(found, Vec::<String>::new(), "{name} should be refused");
            // The ruling itself: whenever L5 fires against an R111 collection, the rule that made it
            // reachable fired too. Change R111's quantity or MAX_COPIES and this is the test that goes red.
            if found.contains(&"L5".to_string()) {
                saw_l5 = true;
                assert!(found.iter().any(|rule| rule != "L5"), "{name}: L5 was the only rule");
            }
        }
        // Otherwise the loop above would prove the claim by never reaching L5 at all.
        assert!(saw_l5, "no case reached L5, so this proves nothing");
    }
}

// ---------------------------------------------------------------------------
// R163 — "The catalog a client that ships none can read"
// ---------------------------------------------------------------------------

mod r163_the_catalog_endpoint_section_9_1_section_9_4_r105 {
    use super::*;

    #[tokio::test]
    async fn r163_serves_the_whole_unprojected_catalog_to_a_caller_with_no_account_at_all() {
        let app = test_app().await;

        // PREMISE: the gate is on. The same request with no token is refused by a guarded route
        // (`GET /api/collection` declares `active`), so "the anonymous call worked" is not just
        // "this router lets everybody through": §9.4's gate has to be demonstrably awake.
        let gated = get(&app, "/api/collection", None).await;
        assert_eq!(gated.status, 401);

        let response = get(&app, "/api/catalog", None).await;
        assert_eq!(response.status, 200);
        let body = response.json();

        // R105's version, so a stale client learns it is stale before it builds a deck.
        assert_eq!(body["version"], json!(app.catalog.version));
        assert_eq!(body["version"], json!(jackioh_cards::catalog_version()));

        // Whole: every entry the catalog holds, every one of them.
        let file = catalog_file();
        assert_eq!(body["defs"], file);

        // Unprojected: not one field is trimmed off a card on the way out. A trimmed card would be a
        // second, weaker copy of the catalog, and the deckbuilder's verdict (UX) would stop being the
        // verdict the save runs (law).
        for card_id in sorted_keys(&file) {
            assert_eq!(sorted_keys(&body["defs"][card_id.as_str()]), sorted_keys(&file[card_id.as_str()]), "{card_id}");
        }
    }

    #[test]
    fn r163_declares_auth_none_like_the_file_it_stands_in_for() {
        let routes: Vec<(String, bool)> = app::ROUTES
            .iter()
            .filter(|(_, path, _, _)| path.starts_with("/api/catalog"))
            .map(|(method, path, auth, _)| (format!("{method} {path}"), matches!(auth, AuthLevel::None)))
            .collect();
        // `GET /api/catalog/:version` is not served (SURFACE §11.3).
        assert_eq!(routes.iter().map(|(route, _)| route.as_str()).collect::<Vec<_>>(), vec!["GET /api/catalog"]);
        // "The same bytes for everybody, naming no profile": §9.4's gate is about collection, loadout,
        // queue and match, and card data is none of those.
        assert!(routes.iter().all(|(_, open)| *open));
    }

    #[tokio::test]
    async fn r163_hands_a_pending_account_and_an_anonymous_caller_the_identical_bytes() {
        let app = test_app().await;
        // R144's pending fixture: verified email, `profiles.status = 'pending'`.
        let token = "e2e-token-pending";

        let anonymous = get(&app, "/api/catalog", None).await;
        let pending = get(&app, "/api/catalog", Some(token)).await;

        assert_eq!(anonymous.status, 200);
        assert_eq!(pending.status, 200);
        // It names no profile, so it cannot differ by one.
        assert_eq!(pending.text, anonymous.text);
    }
}

// ---------------------------------------------------------------------------
// R388 — card patch history: every patch's catalog, served by version
// ---------------------------------------------------------------------------

/// SURFACE §11.3 drops `GET /api/catalog/:version` (no client called it), so the three TS tests that
/// read a patch's snapshot through it ("serves every patch in patches.json", "shows what a patch
/// changed", "serves the version this server runs … whatever it is called") have no route to call.
/// The one that holds as written is the refusal: no version, known or not, reads a file.
mod r388_get_api_catalog_version_serves_the_catalog_as_each_patch_left_it_b4_2 {
    use super::*;

    #[tokio::test]
    async fn r388_answers_an_unknown_or_malformed_version_with_404_and_reads_no_file_for_it() {
        let app = test_app().await;
        for version in ["v9.9.9", "patches", "..%2Fcatalog", "v0.2.0.json", "%00"] {
            let response = get(&app, &format!("/api/catalog/{version}"), None).await;
            assert_eq!(response.status, 404, "{version}");
        }
    }
}

// ---------------------------------------------------------------------------
// R164 — "Where L6's ban list lives"
// ---------------------------------------------------------------------------

/// A ban held as server state: the catalog data is untouched, only the handle answers differently.
fn with_ban(catalog: &Catalog, banned_id: &str) -> Catalog {
    Catalog { banned: [banned_id.to_string()].into_iter().collect(), ..catalog.clone() }
}

mod r164_where_l6_s_ban_list_lives_section_9_4_r105 {
    use super::*;

    #[tokio::test]
    async fn r164_reads_bannedness_through_the_catalog_handle_never_off_a_card_definition() {
        let app = test_app().await;
        let catalog = &app.catalog;
        let playable = legal(catalog, 60);
        let victim = playable[0].clone();
        let owned = one_of_each(&playable);
        let loadout = decks(&[playable[0..20].to_vec(), playable[20..40].to_vec(), playable[40..60].to_vec()]);

        // PREMISE: the loadout is legal today, so the L6 below comes from the ban and nothing else.
        assert_eq!(validate_trio(loadout.clone(), catalog, owned.clone()), Vec::<Value>::new());

        let banned = with_ban(catalog, &victim);
        let found = validate_trio(loadout, &banned, owned);

        // L6: "every card exists in the current catalog version and is not banned".
        assert!(rules(&found).contains(&"L6".to_string()));
        let l6 = found.iter().find(|issue| issue["rule"] == json!("L6")).expect("an L6");
        assert_eq!(l6["cardId"], json!(victim));
    }

    #[tokio::test]
    async fn r164_keeps_a_ban_out_of_the_catalog_data_so_r105_s_version_does_not_move() {
        let app = test_app().await;
        let victim = app.catalog.card_ids[0].clone();
        let banned = with_ban(&app.catalog, &victim);

        // The failure R164 exists to prevent: a flag on the card would mean a new R105 version, and
        // §9.4's stale-version rejection would invalidate every saved loadout in the game at once.
        assert_eq!(banned.version, app.catalog.version);
        assert_eq!(banned.card_ids, app.catalog.card_ids);
        let served = get(&app_holding(banned).await, "/api/catalog", None).await.json();
        assert_eq!(served["defs"], catalog_file());
        // A card definition carries no ban flag for anything to have been written to.
        let file = catalog_file();
        for card_id in sorted_keys(&file) {
            let flagged: Vec<String> = sorted_keys(&file[card_id.as_str()])
                .into_iter()
                .filter(|key| key.to_lowercase().contains("ban"))
                .collect();
            assert_eq!(flagged, Vec::<String>::new(), "{card_id}");
        }
    }

    #[tokio::test]
    async fn r164_never_hands_the_client_a_copy_of_the_list_the_served_bytes_do_not_change() {
        let app = test_app().await;
        let victim = app.catalog.card_ids[0].clone();

        // R163's route carries the catalog both sides ship; R164 keeps the ban list out of it, so the
        // client has no copy of a list it has no business being able to disagree with.
        let with = get(&app_holding(with_ban(&app.catalog, &victim)).await, "/api/catalog", None).await;
        let without = get(&app_holding(app.catalog.clone()).await, "/api/catalog", None).await;
        assert_eq!(with.text, without.text);
    }

    #[tokio::test]
    async fn r164_bans_nothing_in_section_8_at_launch_and_holds_the_hook_open_for_when_something_is() {
        let app = test_app().await;
        // "Nothing in §8 is banned at launch" — every card, not just a sample.
        let banned: Vec<&String> = app.catalog.card_ids.iter().filter(|id| app.catalog.is_banned(id)).collect();
        assert!(banned.is_empty(), "{banned:?}");
        // The single hook, which reads the db agent's `cards` table once there is something to ban.
        assert!(!catalog_api::catalog_from(CardDefs::default(), "v0").is_banned("core-001"));
    }

    #[tokio::test]
    async fn r163_reports_the_deployed_commit_in_a_header_when_one_is_known_and_leaves_the_body_alone() {
        let commit = "0123456789abcdef0123456789abcdef01234567";
        let mut deployed = test_env();
        // `env.rs` reads Render's `RENDER_GIT_COMMIT` into the deployed commit `app::build` hands the
        // catalog route.
        deployed.insert("RENDER_GIT_COMMIT".to_string(), commit.to_string());

        let known = get(&app_from(&deployed).await, "/api/catalog", None).await;
        let unknown = get(&test_app().await, "/api/catalog", None).await;

        // deploy-watch.yml compares this header with the commit that was pushed.
        assert_eq!(DEPLOYED_COMMIT_HEADER, "x-deployed-commit");
        assert_eq!(known.headers.get(DEPLOYED_COMMIT_HEADER).and_then(|value| value.to_str().ok()), Some(commit));
        // No commit (a local server, anything but Render): no header, rather than an empty one.
        assert!(unknown.headers.get(DEPLOYED_COMMIT_HEADER).is_none());
        // The header is not the body: the bytes stay the same for everybody.
        assert_eq!(known.text, unknown.text);
    }
}
