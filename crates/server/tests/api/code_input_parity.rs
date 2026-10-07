//! Port of `apps/server/test/api/code-input-parity.test.ts`.
//!
//! R191, docs/polish/5-sign-in.md B5 and B6: the server redeems exactly what the shared reading
//! says a typed or pasted code is.
//!
//! B5 runs every row of the shared table (`crates/engine/tests/fixtures/code-input-cases.json`,
//! SURFACE §10.4, the JSON form of `packages/shared/test/fixtures/code-input-cases.ts`) through the
//! real router. A row with a canonical code redeems a code minted as that canonical; a row without
//! one gets R145's identical error byte for byte, and the code the row was mangled from (the table's
//! base code, minted beside it) is left untouched. The web's code field is tested against the same
//! rows, so the two ends cannot drift apart without one of them going red.
//!
//! B6 is the input cap: a code longer than `CODE_INPUT_MAX_LENGTH` is refused unread, logged as
//! malformed, and padded to the redemption floor (R107) like every other outcome.
//!
//! Every test here runs on tokio's paused clock (`start_paused`) with the production floor, which
//! costs nothing to sit through; the B5 rows each run on a fresh server with their own profile and
//! address, so §9.4 steps 2 and 3 never refuse. TS generated one `it` per table row; Rust has no
//! test generator without a macro, so each half of B5 is one test that names the failing row.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{HeaderMap, Request};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tower::ServiceExt;

use jackioh_server::api::codes::DEFAULT_INVITE_CODE_MAX_USES;
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    API_MAX_BODY_BYTES, CODE_INPUT_MAX_LENGTH, INVITE_CODE_FORMAT, INVITE_CODE_GROUP_SIZE, INVITE_CODE_SEPARATOR,
    REDEMPTION_IDENTICAL_ERROR, REDEMPTION_RESPONSE_FLOOR_MS,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;

use crate::support::deps;

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

/// The table's base code: every malformed row is a mangled spelling of it.
const BASE_CODE: &str = "ABCDEFGHJKMNPQRS";

/// `CODE_PEPPER` for every server here. `index.ts` keys the code hash `${CODE_PEPPER}:code`, and the
/// code a test puts on record is hashed the same way.
const PEPPER: &str = "code-input-parity-test-pepper-of-32-characters-or-more";

/// The shared table (SURFACE §10.4): one row per typed or pasted input.
fn code_input_cases() -> Vec<Value> {
    let table: Value = serde_json::from_str(include_str!("../../../engine/tests/fixtures/code-input-cases.json"))
        .expect("the shared table is JSON");
    table.as_array().cloned().expect("the shared table is an array of rows")
}

/// R145's refusal, exactly as it goes over the wire.
fn identical_body() -> String {
    serde_json::to_string(&json!({ "error": { "code": "invalid_code", "message": REDEMPTION_IDENTICAL_ERROR } }))
        .expect("the body serialises")
}

/// The environment `createTestDeps()` stood for: `E2E=1` (the fake store and fixture auth), the
/// compiled-in catalog version (SURFACE §11.3) and one trusted proxy hop, whose entry
/// `json_request` writes.
fn test_env() -> IndexMap<String, String> {
    let mut source = IndexMap::new();
    for (name, value) in [("E2E", "1"), ("NODE_ENV", "test"), ("CODE_PEPPER", PEPPER), ("TRUSTED_PROXY_HOPS", "1")] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

/// One App and its router: TS's `deps` and `createRouter(createCodesRoutes(), deps)`.
struct Server {
    app: Arc<App>,
    router: axum::Router,
}

/// One response, read whole.
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

/// A fresh server with R144's fixtures wiped, so the code a test mints is the only one on record.
async fn fresh_server() -> Server {
    let env = app::load_server_env(&test_env()).expect("the test environment loads");
    let app = app::build(env).await.expect("the test app builds");
    store_of(&app).lock().await.reset();
    Server { router: app::router(app.clone()), app }
}

impl Server {
    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.expect("the router always answers");
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");
        Reply { status, headers, text: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8") }
    }
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

async fn attempts(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.attempts)
}

async fn codes(server: &Server) -> Vec<Value> {
    json_rows(&store_of(&server.app).lock().await.tables.codes)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("after 1970").as_millis() as i64
}

/// HMAC-SHA256 as lower-case hex (RFC 2104 over `sha2`, so this file needs no MAC crate's API).
fn hmac_sha256_hex(key: &str, message: &str) -> String {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key.as_bytes());
        block[..digest.len()].copy_from_slice(&digest);
    } else {
        block[..key.len()].copy_from_slice(key.as_bytes());
    }
    let inner_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x36).collect();
    let outer_pad: Vec<u8> = block.iter().map(|byte| byte ^ 0x5c).collect();
    let mut inner = Sha256::new();
    inner.update(&inner_pad);
    inner.update(message.as_bytes());
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&outer_pad);
    outer.update(&inner[..]);
    outer.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `formatCode(raw)`: groups of `INVITE_CODE_GROUP_SIZE` joined by `INVITE_CODE_SEPARATOR` (§9.4).
fn formatted(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    chars
        .chunks(INVITE_CODE_GROUP_SIZE as usize)
        .map(|group| group.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(INVITE_CODE_SEPARATOR)
}

/// `mintInviteCode(deps)` on `depsMinting(code)`: TS's `Ids` fake made the mint draw exactly
/// `code`, so the test decides which code exists. The Rust mint draws from the OS, so the row is
/// written as the mint writes it: hashed (`${CODE_PEPPER}:code` over the canonical code), unused,
/// `DEFAULT_INVITE_CODE_MAX_USES` uses, no expiry. Answers the code as `formatCode` shows it.
async fn mint_exactly(server: &Server, code: &str) -> String {
    let mut tx = server.app.db.begin(None).await.expect("a transaction opens");
    tx.codes_insert(&from_json(json!({
        // A UUID's shape, which `invite_codes.id` is in Postgres.
        "id": "00000000-0000-4000-8000-000000000001",
        "codeHash": hmac_sha256_hex(&format!("{PEPPER}:code"), code),
        "maxUses": DEFAULT_INVITE_CODE_MAX_USES,
        "uses": 0,
        "revoked": false,
        "expiresAt": null,
        "createdAt": now_ms(),
    })))
    .await
    .expect("the code is stored");
    tx.commit().await.expect("the transaction commits");
    formatted(code)
}

struct Seeded {
    token: String,
    profile_id: String,
}

/// An auth user with a verified email (`deps.auth.addUser`) and a profile row (`seedProfile`).
async fn seed_caller(server: &Server, id: &str, status: &str) -> Seeded {
    let user_id = format!("user-{id}");
    let token = deps::add_user(&server.app, &user_id, &format!("{id}@example.test"), true);
    let _ = store_of(&server.app).lock().await.seed_profile(json!({ "id": id, "userId": user_id, "status": status }));
    Seeded { token, profile_id: id.to_string() }
}

/// `jsonRequest("POST", "/api/codes/redeem", body, { token, ip })`.
fn redeem_request(body: Value, token: &str, ip: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/codes/redeem")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .header("x-forwarded-for", ip)
        .body(Body::from(body.to_string()))
        .expect("a well-formed request")
}

async fn redeem(server: &Server, token: &str, code: &str, ip: &str) -> Reply {
    server.send(redeem_request(json!({ "code": code }), token, ip)).await
}

/// `tables.profiles.find((row) => row.id === profileId)?.status`.
async fn status_of(server: &Server, profile_id: &str) -> Value {
    json_rows(&store_of(&server.app).lock().await.tables.profiles)
        .into_iter()
        .find(|row| row["id"] == json!(profile_id))
        .map(|row| row["status"].clone())
        .unwrap_or(Value::Null)
}

/// `minted.padEnd(length, " ")`.
fn pad_end(text: &str, length: usize) -> String {
    format!("{text:<length$}")
}

// ---------------------------------------------------------------------------------------------
// B5: every table row, redeemed
// ---------------------------------------------------------------------------------------------

mod r191_b5_the_server_redeems_every_row_of_the_shared_table_as_the_client_reads_it {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r191_b5_redeems_every_row_with_a_canonical_code() {
        let mut ran = 0;
        for (index, row) in code_input_cases().iter().enumerate() {
            let Some(canonical) = row["canonical"].as_str() else { continue };
            let name = row["name"].as_str().unwrap_or_default();
            let input = row["input"].as_str().expect("every row has an input");
            let ip = format!("198.51.100.{}", index + 1);

            let server = fresh_server().await;
            let caller = seed_caller(&server, &format!("parity-{index}"), "pending").await;
            let minted = mint_exactly(&server, canonical).await;
            // PREMISE: the code on record is the row's canonical code.
            assert_eq!(minted.split(INVITE_CODE_FORMAT.separator).collect::<String>(), canonical, "{name}");

            let response = redeem(&server, &caller.token, input, &ip).await;

            assert_eq!(response.status, 200, "{name}: {}", response.text);
            assert_eq!(status_of(&server, &caller.profile_id).await, json!("active"), "{name}");
            assert_eq!(codes(&server).await[0]["uses"], json!(1), "{name}");
            let results: Vec<Value> = attempts(&server).await.iter().map(|attempt| attempt["result"].clone()).collect();
            assert_eq!(results, vec![json!("ok")], "{name}");
            ran += 1;
        }
        assert!(ran > 0, "the table has rows with a canonical code");
    }

    #[tokio::test(start_paused = true)]
    async fn r191_b5_refuses_every_row_without_one_with_r145_s_identical_error() {
        let mut ran = 0;
        for (index, row) in code_input_cases().iter().enumerate() {
            if !row["canonical"].is_null() {
                continue;
            }
            let name = row["name"].as_str().unwrap_or_default();
            let input = row["input"].as_str().expect("every row has an input");
            let ip = format!("198.51.100.{}", index + 1);

            // The base code exists, so a server that dropped, mapped or truncated its way to it
            // would activate the account here.
            let server = fresh_server().await;
            let caller = seed_caller(&server, &format!("parity-{index}"), "pending").await;
            mint_exactly(&server, BASE_CODE).await;

            let response = redeem(&server, &caller.token, input, &ip).await;

            assert_eq!(response.status, 400, "{name}");
            assert_eq!(response.text, identical_body(), "{name}");
            assert!(response.headers.get("retry-after").is_none(), "{name}");
            assert_eq!(status_of(&server, &caller.profile_id).await, json!("pending"), "{name}");
            assert_eq!(codes(&server).await[0]["uses"], json!(0), "{name}");
            // Past steps 2 and 3, so logged (step 4), under the operator-only reason.
            let logged = attempts(&server).await;
            assert_eq!(logged.len(), 1, "{name}");
            assert_eq!(logged[0]["reason"], json!("malformed"), "{name}");
            ran += 1;
        }
        assert!(ran > 0, "the table has rows without a canonical code");
    }
}

mod r191_an_empty_code_is_read_like_every_other_input_that_holds_no_code {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r191_empty_answers_exactly_as_dashes_and_a_space_do_the_account_s_own_refusal_included_never_bad_request() {
        let server = fresh_server().await;
        mint_exactly(&server, BASE_CODE).await;
        let active = seed_caller(&server, "empty-active", "active").await;
        let pending = seed_caller(&server, "empty-pending", "pending").await;

        // An active account is refused for its own state before any code is read (R145).
        let mut active_answers: Vec<String> = Vec::new();
        for (index, code) in ["", "----", " "].iter().enumerate() {
            let response = redeem(&server, &active.token, code, &format!("198.51.100.{}", 220 + index)).await;
            assert_eq!(response.status, 409, "{code:?}");
            active_answers.push(response.text);
        }
        active_answers.dedup();
        assert_eq!(active_answers.len(), 1, "{active_answers:?}");

        // A pending one gets R145's identical error, and the attempt is logged as malformed.
        let empty = redeem(&server, &pending.token, "", "198.51.100.230").await;
        assert_eq!(empty.status, 400);
        assert_eq!(empty.text, identical_body());
        let reasons: Vec<Value> = attempts(&server)
            .await
            .into_iter()
            .filter(|row| row["profileId"] == json!(pending.profile_id))
            .map(|row| row["reason"].clone())
            .collect();
        assert_eq!(reasons, vec![json!("malformed")]);
    }

    #[tokio::test(start_paused = true)]
    async fn r191_a_code_that_is_not_a_string_at_all_is_still_a_malformed_request() {
        let server = fresh_server().await;
        let caller = seed_caller(&server, "number-code", "pending").await;
        let response = server.send(redeem_request(json!({ "code": 42 }), &caller.token, "198.51.100.231")).await;
        assert_eq!(response.status, 400);
        assert_eq!(response.json()["error"]["code"], json!("bad_request"));
    }
}

// ---------------------------------------------------------------------------------------------
// B6: the input cap
// ---------------------------------------------------------------------------------------------

mod r191_b6_a_code_longer_than_code_input_max_length {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r191_b6_refuses_a_minted_code_padded_past_the_cap_unread_with_the_identical_error() {
        let server = fresh_server().await;
        let caller = seed_caller(&server, "padded-past-cap", "pending").await;
        let minted = mint_exactly(&server, BASE_CODE).await;

        // The code itself is good; only the whitespace around it pushes the input over the cap.
        let input = pad_end(&minted, CODE_INPUT_MAX_LENGTH as usize + 1);
        let response = redeem(&server, &caller.token, &input, "198.51.100.201").await;

        assert_eq!(response.status, 400);
        assert_eq!(response.text, identical_body());
        assert!(response.headers.get("retry-after").is_none());
        assert_eq!(status_of(&server, &caller.profile_id).await, json!("pending"));
        assert_eq!(codes(&server).await[0]["uses"], json!(0));
        let logged = attempts(&server).await;
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0]["reason"], json!("malformed"));
    }

    #[tokio::test(start_paused = true)]
    async fn r191_b6_still_redeems_a_minted_code_padded_to_exactly_the_cap() {
        let server = fresh_server().await;
        let caller = seed_caller(&server, "padded-to-cap", "pending").await;
        let minted = mint_exactly(&server, BASE_CODE).await;

        let input = pad_end(&minted, CODE_INPUT_MAX_LENGTH as usize);
        assert_eq!(input.len(), CODE_INPUT_MAX_LENGTH as usize);
        let response = redeem(&server, &caller.token, &input, "198.51.100.202").await;

        assert_eq!(response.status, 200);
        assert_eq!(status_of(&server, &caller.profile_id).await, json!("active"));
    }

    #[tokio::test(start_paused = true)]
    async fn r191_b6_answers_a_huge_code_with_the_same_bytes_as_a_missing_one_and_logs_it_as_malformed() {
        let server = fresh_server().await;
        let huge = seed_caller(&server, "huge-code", "pending").await;
        let missing = seed_caller(&server, "missing-code", "pending").await;
        mint_exactly(&server, BASE_CODE).await;

        // Well inside the body limit, far past the input cap.
        let huge_code = BASE_CODE.repeat(API_MAX_BODY_BYTES as usize / (2 * BASE_CODE.len()));
        assert!(huge_code.len() > CODE_INPUT_MAX_LENGTH as usize);
        let huge_response = redeem(&server, &huge.token, &huge_code, "198.51.100.203").await;
        let missing_response = redeem(&server, &missing.token, "ABCD-EFGH-JKMN-PQRT", "198.51.100.204").await;

        assert_eq!(huge_response.status, 400);
        assert_eq!(missing_response.status, 400);
        assert_eq!(vec![huge_response.text, missing_response.text], vec![identical_body(), identical_body()]);

        let huge_attempts: Vec<Value> =
            attempts(&server).await.into_iter().filter(|row| row["profileId"] == json!(huge.profile_id)).collect();
        assert_eq!(huge_attempts.len(), 1);
        assert_eq!(huge_attempts[0]["result"], json!("rejected"));
        assert_eq!(huge_attempts[0]["reason"], json!("malformed"));
    }

    /// TS ran this one on `createVirtualTimers` with the production floor; tokio's paused clock is
    /// that clock here, and `Instant` reads it.
    #[tokio::test(start_paused = true)]
    async fn r191_b6_pads_the_refusal_to_the_redemption_floor_like_every_other_outcome_r107() {
        let server = fresh_server().await;
        let too_long = seed_caller(&server, "floor-too-long", "pending").await;
        let missing = seed_caller(&server, "floor-missing", "pending").await;
        let minted = mint_exactly(&server, BASE_CODE).await;

        let started_too_long = tokio::time::Instant::now();
        let too_long_response =
            redeem(&server, &too_long.token, &pad_end(&minted, CODE_INPUT_MAX_LENGTH as usize + 1), "198.51.100.205").await;
        let too_long_elapsed = started_too_long.elapsed().as_millis() as u64;

        let started_missing = tokio::time::Instant::now();
        let missing_response = redeem(&server, &missing.token, "ABCD-EFGH-JKMN-PQRT", "198.51.100.206").await;
        let missing_elapsed = started_missing.elapsed().as_millis() as u64;

        assert_eq!(too_long_response.status, 400);
        assert_eq!(missing_response.status, 400);
        assert!(too_long_elapsed >= REDEMPTION_RESPONSE_FLOOR_MS as u64, "{too_long_elapsed}");
        assert!(missing_elapsed >= REDEMPTION_RESPONSE_FLOOR_MS as u64, "{missing_elapsed}");
    }
}
