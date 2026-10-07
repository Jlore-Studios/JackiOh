//! Port of `apps/server/test/api/client-address.test.ts`.
//!
//! R190, docs/polish/5-sign-in.md B10, B11 and B12: which address a per-IP limit counts.
//!
//! §9.4 step 3 (20 redemptions per IP hash per hour) and R157's anonymous API bucket both key on
//! "the IP hash". Behind a proxy every request arrives from the proxy, so the caller's address is
//! read from `X-Forwarded-For`, but only from the entry the deployment's own proxies wrote: the
//! `TRUSTED_PROXY_HOPS`-th from the right. Everything to its left was written by the caller. The
//! server once read the leftmost entry, which let a caller pick a fresh bucket per request by
//! changing one header.
//!
//!  - B10: with one trusted hop, the rightmost entry is the key, in `code_attempts.ipHash` and in
//!    the anonymous limiter alike.
//!  - B11: `CF-Connecting-IP` and `X-Real-IP` are never read; too few entries fall back to the
//!    socket's peer address (axum's `ConnectInfo<SocketAddr>`, TS's `RequestContext.peerAddress`),
//!    and no peer to `UNKNOWN_CLIENT_ADDRESS`.
//!  - B12: `TRUSTED_PROXY_HOPS` is parsed by `load_env` and defaults to 0 (no proxy trusted), and the
//!    router reports the fewest entries any request carried, never an address.
//!  - An IPv6 client is counted by its /56, and an IPv4-mapped address as the IPv4 address.
//!
//! Rust deltas: the router is the App's own (`app::router`, SURFACE §11.2), so TS's one-route
//! `openRouter` is `GET /api/catalog` (R163: `auth: none`) on it, and "a second router" is a second
//! App; the redemption floor is the production 250 ms on tokio's paused clock (`start_paused`); the
//! log is `tracing`, read back by the recording layer below.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tower::ServiceExt;
use tracing_subscriber::layer::SubscriberExt;

use jackioh_server::api::http::{UNKNOWN_CLIENT_ADDRESS, client_address, rate_limit_address};
use jackioh_server::app::{self, App};
use jackioh_server::config::{
    API_REQUESTS_PER_MINUTE, CODE_ATTEMPTS_PER_IP_PER_HOUR, DEFAULT_TRUSTED_PROXY_HOPS,
    IPV6_RATE_LIMIT_PREFIX_BITS, MAX_TRUSTED_PROXY_HOPS, REDEMPTION_IDENTICAL_ERROR,
};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::Db;
use jackioh_server::env::{SERVER_ONLY_ENV_VARS, load_env};

use crate::support::deps;

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

/// R109's allowance, as the server carries it.
const LIMIT: usize = API_REQUESTS_PER_MINUTE;

/// A well-formed code inside R104's alphabet that is never minted.
const UNMINTED_CODE: &str = "ABCD-EFGH-JKMN-PQRT";

/// The address our own proxy saw, and wrote last.
const CLIENT: &str = "203.0.113.20";
/// The socket's peer: the proxy itself, or a direct caller.
const PEER: &str = "192.0.2.10";

/// `CODE_PEPPER` for every server here. `index.ts` keys the IP hash `${CODE_PEPPER}:ip`, and a test
/// that compares a stored `ipHash` computes the same one (`ip_hash`).
const PEPPER: &str = "client-address-test-pepper-of-32-characters-or-more";

/// The environment `createTestDeps()` stood for: `E2E=1` (the fake store and fixture auth), the
/// compiled-in catalog version (SURFACE §11.3) and one trusted proxy hop, the deployed server
/// behind Render's edge (`render.yaml`).
fn test_env() -> IndexMap<String, String> {
    let mut source = IndexMap::new();
    for (name, value) in [("E2E", "1"), ("NODE_ENV", "test"), ("CODE_PEPPER", PEPPER), ("TRUSTED_PROXY_HOPS", "1")] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

/// `test_env()` with `TRUSTED_PROXY_HOPS` set to `hops`.
fn env_with_hops(hops: &str) -> IndexMap<String, String> {
    let mut source = test_env();
    source.insert("TRUSTED_PROXY_HOPS".to_string(), hops.to_string());
    source
}

/// `deps` as a server with no `TRUSTED_PROXY_HOPS` configured sees them.
fn env_without_hops() -> IndexMap<String, String> {
    let mut source = test_env();
    source.shift_remove("TRUSTED_PROXY_HOPS");
    source
}

/// One App and its router: TS's `deps` and `createRouter(…, deps)`. The router is built once, so
/// whatever it keeps per router (TS kept the limiter and the forwarded-for minimum) lasts the test.
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

    fn error_code(&self) -> String {
        self.json()["error"]["code"].as_str().unwrap_or_default().to_string()
    }
}

/// The server `createTestDeps(overrides)` built, from `source`, with R144's fixtures wiped so its
/// store starts as empty as TS's memory store did.
async fn server_from(source: &IndexMap<String, String>) -> Server {
    let env = app::load_server_env(source).expect("the test environment loads");
    let app = app::build(env).await.expect("the test app builds");
    store_of(&app).lock().await.reset();
    Server { router: app::router(app.clone()), app }
}

impl Server {
    /// `router(request, { peerAddress })`: the peer travels as axum's `ConnectInfo`, which the host
    /// fills from the socket; `None` is a request whose host named no peer.
    async fn send(&self, mut request: Request<Body>, peer: Option<&str>) -> Reply {
        if let Some(peer) = peer {
            let ip: IpAddr = peer.parse().expect("a peer address");
            request.extensions_mut().insert(ConnectInfo(SocketAddr::new(ip, 0)));
        }
        let response = self.router.clone().oneshot(request).await.expect("the router always answers");
        let status = response.status().as_u16();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");
        Reply { status, text: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8") }
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

/// `deps.hashes.ip(raw)`: `crypto.ts`'s `createHashes`, keyed as `index.ts` keys it.
fn ip_hash(raw: &str) -> String {
    hmac_sha256_hex(&format!("{PEPPER}:ip"), &raw.trim().to_lowercase())
}

fn headers(entries: &[(&str, &str)]) -> HeaderMap {
    let mut built = HeaderMap::new();
    for (name, value) in entries {
        built.append(
            HeaderName::from_bytes(name.as_bytes()).expect("a header name"),
            HeaderValue::from_str(value).expect("a header value"),
        );
    }
    built
}

fn request(method: &str, path: &str, entries: &[(&str, &str)], body: Body) -> Request<Body> {
    let mut built = Request::builder().method(method).uri(path).body(body).expect("a well-formed request");
    *built.headers_mut() = headers(entries);
    built
}

/// A redemption request with exactly the headers given, and nothing added.
fn redeem_request(token: &str, extra: &[(&str, &str)]) -> Request<Body> {
    let bearer = format!("Bearer {token}");
    let mut entries = vec![("content-type", "application/json"), ("authorization", bearer.as_str())];
    entries.extend_from_slice(extra);
    request("POST", "/api/codes/redeem", &entries, Body::from(json!({ "code": UNMINTED_CODE }).to_string()))
}

/// A request to an `auth: none` route, with exactly the headers given.
fn open_request(extra: &[(&str, &str)]) -> Request<Body> {
    request("GET", "/api/catalog", extra, Body::empty())
}

/// Redemption on tokio's paused clock with the production floor, as `codes.rs` runs it.
async fn code_server() -> Server {
    server_from(&test_env()).await
}

struct Seeded {
    token: String,
    profile_id: String,
}

/// An auth user with a verified email, and a pending profile for them.
async fn seed_caller(server: &Server, id: &str) -> Seeded {
    let user_id = format!("user-{id}");
    let token = deps::add_user(&server.app, &user_id, &format!("{id}@example.test"), true);
    let _ = store_of(&server.app).lock().await.seed_profile(json!({ "id": id, "userId": user_id, "status": "pending" }));
    Seeded { token, profile_id: id.to_string() }
}

/// An `auth: none` router: TS's one-route `openRouter`, here the App's own router, whose
/// `GET /api/catalog` is open (R163).
async fn open_server(source: &IndexMap<String, String>) -> Server {
    server_from(source).await
}

/// Puts `address` over §9.4 step 3's per-IP limit, through other people's attempts.
async fn flood_address(server: &Server, address: &str) {
    let hash = ip_hash(address);
    let at = now_ms();
    for i in 0..CODE_ATTEMPTS_PER_IP_PER_HOUR as usize + 1 {
        let mut tx = server.app.db.begin(None).await.expect("a transaction opens");
        tx.codes_log_attempt(&from_json(json!({
            "profileId": format!("neighbour-{i}"),
            "ipHash": hash,
            "result": "rejected",
            "reason": "missing",
            "at": at,
        })))
        .await
        .expect("the attempt is logged");
        tx.commit().await.expect("the transaction commits");
    }
}

/// The ip hash of the one attempt `profile_id` has on record.
async fn attempt_ip_hash(server: &Server, profile_id: &str) -> String {
    let rows: Vec<Value> = json_rows(&store_of(&server.app).lock().await.tables.attempts)
        .into_iter()
        .filter(|row| row["profileId"] == json!(profile_id))
        .collect();
    assert_eq!(rows.len(), 1, "{profile_id}: {rows:?}");
    rows[0]["ipHash"].as_str().unwrap_or_default().to_string()
}

// ---------------------------------------------------------------------------------------------
// The log, as TS's `createRecordingLogger` kept it
// ---------------------------------------------------------------------------------------------

/// One logged event: TS's `{ level, event, data }`. The event's name is its `event` field, else its
/// message; a snake_case field name is read as TS's camelCase key.
#[derive(Clone, Debug, Serialize)]
struct Entry {
    level: String,
    event: String,
    data: serde_json::Map<String, Value>,
}

#[derive(Clone, Default)]
struct Recording(Arc<std::sync::Mutex<Vec<Entry>>>);

impl Recording {
    fn entries(&self) -> Vec<Entry> {
        self.0.lock().expect("the recording").clone()
    }

    fn forwarded(&self) -> Vec<Entry> {
        self.entries().into_iter().filter(|entry| entry.event == "api.forwarded_for").collect()
    }
}

#[derive(Default)]
struct Fields(serde_json::Map<String, Value>);

fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in name.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

impl Fields {
    fn put(&mut self, field: &tracing::field::Field, value: Value) {
        self.0.insert(camel(field.name()), value);
    }
}

impl tracing::field::Visit for Fields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.put(field, Value::from(value));
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.put(field, Value::from(value));
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.put(field, Value::from(value));
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.put(field, Value::from(value));
    }
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.put(field, json!(value));
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.put(field, Value::from(format!("{value:?}")));
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Recording {
    fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let mut data = fields.0;
        let name = data
            .remove("event")
            .or_else(|| data.remove("message"))
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default();
        let level = event.metadata().level().to_string().to_lowercase();
        self.0.lock().expect("the recording").push(Entry { level, event: name, data });
    }
}

/// Records every event this thread logs until the guard drops (`#[tokio::test]` runs the server's
/// futures on the test's own thread).
fn record() -> (Recording, tracing::subscriber::DefaultGuard) {
    let recording = Recording::default();
    let guard = crate::support::deps::set_log_default(tracing_subscriber::registry().with(recording.clone()));
    (recording, guard)
}

// ---------------------------------------------------------------------------------------------
// client_address
// ---------------------------------------------------------------------------------------------

mod r190_client_address {
    use super::*;

    #[test]
    fn r190_b10_takes_the_rightmost_x_forwarded_for_entry_with_one_trusted_hop() {
        let forwarded = headers(&[("x-forwarded-for", &format!("198.51.100.1, {CLIENT}"))]);
        assert_eq!(client_address(&forwarded, Some(PEER), 1), CLIENT);
    }

    #[test]
    fn r190_b10_counts_the_trusted_hops_from_the_right() {
        let forwarded = headers(&[("x-forwarded-for", "198.51.100.1, 198.51.100.2, 198.51.100.3")]);
        assert_eq!(client_address(&forwarded, Some(PEER), 1), "198.51.100.3");
        assert_eq!(client_address(&forwarded, Some(PEER), 2), "198.51.100.2");
        assert_eq!(client_address(&forwarded, Some(PEER), 3), "198.51.100.1");
    }

    #[test]
    fn r190_b10_trims_every_entry_and_drops_empty_ones() {
        let forwarded = headers(&[("x-forwarded-for", &format!(" 198.51.100.1 ,, {CLIENT}  , "))]);
        assert_eq!(client_address(&forwarded, Some(PEER), 1), CLIENT);
        assert_eq!(client_address(&forwarded, Some(PEER), 2), "198.51.100.1");
    }

    #[test]
    fn r190_b10_reads_every_x_forwarded_for_header_not_just_the_first() {
        let forwarded = headers(&[("x-forwarded-for", "198.51.100.1"), ("x-forwarded-for", CLIENT)]);
        assert_eq!(client_address(&forwarded, Some(PEER), 1), CLIENT);
        assert_eq!(client_address(&forwarded, Some(PEER), 2), "198.51.100.1");
    }

    #[test]
    fn r190_b10_gives_the_same_answer_whatever_the_caller_writes_to_the_left() {
        for spoofed in ["1.1.1.1", "10.0.0.1, 10.0.0.2", "not-an-address", "::1, 127.0.0.1"] {
            let forwarded = headers(&[("x-forwarded-for", &format!("{spoofed}, {CLIENT}"))]);
            assert_eq!(client_address(&forwarded, Some(PEER), 1), CLIENT, "{spoofed}");
        }
    }

    #[test]
    fn r190_b11_falls_back_to_the_peer_when_there_are_fewer_entries_than_trusted_hops() {
        let forwarded = headers(&[("x-forwarded-for", CLIENT)]);
        assert_eq!(client_address(&forwarded, Some(PEER), 2), PEER);
        assert_eq!(client_address(&forwarded, None, 2), UNKNOWN_CLIENT_ADDRESS);
    }

    #[test]
    fn r190_b11_uses_the_peer_when_there_is_no_x_forwarded_for() {
        assert_eq!(client_address(&HeaderMap::new(), Some(PEER), 1), PEER);
    }

    #[test]
    fn r190_b11_answers_unknown_client_address_with_no_header_and_no_peer() {
        assert_eq!(client_address(&HeaderMap::new(), None, 1), UNKNOWN_CLIENT_ADDRESS);
    }

    #[test]
    fn r190_b11_treats_an_x_forwarded_for_of_only_commas_and_spaces_as_none() {
        assert_eq!(client_address(&headers(&[("x-forwarded-for", " , ,, ")]), Some(PEER), 1), PEER);
    }

    #[test]
    fn r190_b11_never_reads_cf_connecting_ip_or_x_real_ip() {
        let vendor = [("cf-connecting-ip", "198.51.100.66"), ("x-real-ip", "198.51.100.77")];
        assert_eq!(client_address(&headers(&vendor), Some(PEER), 1), PEER);
        assert_eq!(client_address(&headers(&vendor), None, 1), UNKNOWN_CLIENT_ADDRESS);
        let mut with_forwarded = vendor.to_vec();
        with_forwarded.push(("x-forwarded-for", CLIENT));
        assert_eq!(client_address(&headers(&with_forwarded), Some(PEER), 1), CLIENT);
    }

    #[test]
    fn r190_b12_ignores_the_header_entirely_with_zero_trusted_hops() {
        let forwarded = headers(&[("x-forwarded-for", &format!("198.51.100.1, {CLIENT}"))]);
        assert_eq!(client_address(&forwarded, Some(PEER), 0), PEER);
        assert_eq!(client_address(&forwarded, None, 0), UNKNOWN_CLIENT_ADDRESS);
    }
}

// ---------------------------------------------------------------------------------------------
// The router: §9.4 step 3 and R157's anonymous bucket
// ---------------------------------------------------------------------------------------------

mod r190_the_per_ip_keys_behind_a_proxy {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r190_b10_hashes_the_rightmost_entry_into_code_attempts_ip_hash() {
        let server = code_server().await;
        let first = seed_caller(&server, "left-one").await;
        let second = seed_caller(&server, "left-two").await;

        server.send(redeem_request(&first.token, &[("x-forwarded-for", &format!("198.51.100.1, {CLIENT}"))]), Some(PEER)).await;
        server
            .send(redeem_request(&second.token, &[("x-forwarded-for", &format!("198.51.100.2, 10.9.8.7, {CLIENT}"))]), Some(PEER))
            .await;

        assert_eq!(attempt_ip_hash(&server, &first.profile_id).await, ip_hash(CLIENT));
        assert_eq!(attempt_ip_hash(&server, &second.profile_id).await, ip_hash(CLIENT));
        assert_ne!(attempt_ip_hash(&server, &first.profile_id).await, ip_hash("198.51.100.1"));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b10_a_fresh_leftmost_entry_does_not_escape_section_9_4_step_3() {
        let server = code_server().await;
        let caller = seed_caller(&server, "spoofer").await;
        flood_address(&server, CLIENT).await;

        let spoofed = server
            .send(redeem_request(&caller.token, &[("x-forwarded-for", &format!("198.51.100.250, {CLIENT}"))]), Some(PEER))
            .await;
        assert_eq!(spoofed.status, 429);
        assert_eq!(spoofed.error_code(), "rate_limited");

        // CONTROL: the same caller from an address with room reaches the lookup and fails there.
        let elsewhere = server
            .send(redeem_request(&caller.token, &[("x-forwarded-for", &format!("{CLIENT}, 203.0.113.21"))]), Some(PEER))
            .await;
        assert_eq!(elsewhere.status, 400);
        let body = elsewhere.json();
        assert_eq!(body["error"]["code"], json!("invalid_code"));
        assert_eq!(body["error"]["message"], json!(REDEMPTION_IDENTICAL_ERROR));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b10_with_two_trusted_hops_entries_left_of_the_second_from_right_never_change_the_bucket() {
        let server = server_from(&env_with_hops("2")).await;
        let caller = seed_caller(&server, "two-hop-spoofer").await;
        flood_address(&server, CLIENT).await;

        // Ours: `CLIENT` as the edge saw it, then the edge as the load balancer saw it.
        let spoofed = server
            .send(
                redeem_request(&caller.token, &[("x-forwarded-for", &format!("198.51.100.251, 198.51.100.252, {CLIENT}, 10.0.0.5"))]),
                Some(PEER),
            )
            .await;

        assert_eq!(spoofed.status, 429);
        assert_eq!(spoofed.error_code(), "rate_limited");
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b11_a_fresh_x_real_ip_does_not_escape_section_9_4_step_3_for_a_flooded_peer() {
        let server = code_server().await;
        let caller = seed_caller(&server, "real-ip-spoofer").await;
        flood_address(&server, PEER).await;

        let response = server.send(redeem_request(&caller.token, &[("x-real-ip", "198.51.100.253")]), Some(PEER)).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.error_code(), "rate_limited");
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b11_a_fresh_cf_connecting_ip_does_not_escape_section_9_4_step_3_for_a_flooded_peer() {
        let server = code_server().await;
        let caller = seed_caller(&server, "cf-spoofer").await;
        flood_address(&server, PEER).await;

        let response = server.send(redeem_request(&caller.token, &[("cf-connecting-ip", "198.51.100.254")]), Some(PEER)).await;

        assert_eq!(response.status, 429);
        assert_eq!(response.error_code(), "rate_limited");
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b10_r157_s_anonymous_bucket_is_shared_by_every_request_whose_rightmost_entry_matches() {
        let server = open_server(&test_env()).await;

        for i in 0..LIMIT {
            let spoofed = format!("10.{}.{}.1", i % 250, i / 250);
            let response = server.send(open_request(&[("x-forwarded-for", &format!("{spoofed}, {CLIENT}"))]), Some(PEER)).await;
            assert_eq!(response.status, 200, "request {i}");
        }

        let next = server.send(open_request(&[("x-forwarded-for", &format!("172.16.0.1, {CLIENT}"))]), Some(PEER)).await;
        assert_eq!(next.status, 429);
        assert_eq!(next.error_code(), "rate_limited");

        // CONTROL: a different client behind the same proxy has its own bucket.
        let other = server.send(open_request(&[("x-forwarded-for", "172.16.0.1, 203.0.113.21")]), Some(PEER)).await;
        assert_eq!(other.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b11_keys_a_request_with_no_x_forwarded_for_on_the_peer_whatever_cf_connecting_ip_says() {
        let server = code_server().await;
        let caller = seed_caller(&server, "direct").await;

        server
            .send(redeem_request(&caller.token, &[("cf-connecting-ip", "198.51.100.66"), ("x-real-ip", "198.51.100.77")]), Some(PEER))
            .await;

        assert_eq!(attempt_ip_hash(&server, &caller.profile_id).await, ip_hash(PEER));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b11_cf_connecting_ip_and_x_real_ip_cannot_open_a_fresh_anonymous_bucket() {
        let server = open_server(&test_env()).await;

        for i in 0..LIMIT {
            let spoofed = format!("10.{}.{}.2", i % 250, i / 250);
            let response =
                server.send(open_request(&[("cf-connecting-ip", &spoofed), ("x-real-ip", &spoofed)]), Some(PEER)).await;
            assert_eq!(response.status, 200, "request {i}");
        }

        let next = server.send(open_request(&[("cf-connecting-ip", "172.16.0.2"), ("x-real-ip", "172.16.0.2")]), Some(PEER)).await;
        assert_eq!(next.status, 429);

        // CONTROL: a different peer is a different bucket.
        let other = server.send(open_request(&[]), Some("192.0.2.11")).await;
        assert_eq!(other.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b11_keys_a_request_with_no_header_and_no_peer_on_unknown_client_address() {
        let server = code_server().await;
        let without_context = seed_caller(&server, "no-context").await;
        let null_peer = seed_caller(&server, "null-peer").await;

        server.send(redeem_request(&without_context.token, &[]), None).await;
        server.send(redeem_request(&null_peer.token, &[("x-real-ip", "198.51.100.77")]), None).await;

        assert_eq!(attempt_ip_hash(&server, &without_context.profile_id).await, ip_hash(UNKNOWN_CLIENT_ADDRESS));
        assert_eq!(attempt_ip_hash(&server, &null_peer.profile_id).await, ip_hash(UNKNOWN_CLIENT_ADDRESS));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b11_falls_back_to_the_peer_when_a_request_carries_fewer_entries_than_the_trusted_hops() {
        let server = server_from(&env_with_hops("2")).await;
        let short = seed_caller(&server, "one-entry").await;
        let full = seed_caller(&server, "two-entries").await;

        server.send(redeem_request(&short.token, &[("x-forwarded-for", CLIENT)]), Some(PEER)).await;
        server.send(redeem_request(&full.token, &[("x-forwarded-for", &format!("{CLIENT}, 10.0.0.5"))]), Some(PEER)).await;

        assert_eq!(attempt_ip_hash(&server, &short.profile_id).await, ip_hash(PEER));
        assert_eq!(attempt_ip_hash(&server, &full.profile_id).await, ip_hash(CLIENT));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b12_zero_trusted_hops_keys_on_the_peer_even_when_x_forwarded_for_is_present() {
        let server = server_from(&env_with_hops("0")).await;
        let caller = seed_caller(&server, "zero-hops").await;

        server.send(redeem_request(&caller.token, &[("x-forwarded-for", &format!("198.51.100.1, {CLIENT}"))]), Some(PEER)).await;

        assert_eq!(attempt_ip_hash(&server, &caller.profile_id).await, ip_hash(PEER));
    }

    /// `app::router` puts the CORS layer in front of the API (SURFACE §11.2), so this is TS's
    /// `withCors(router)` with an empty origin list in all but the list.
    #[tokio::test(start_paused = true)]
    async fn r190_b11_with_cors_hands_the_request_context_through_to_the_router() {
        let server = code_server().await;
        let caller = seed_caller(&server, "through-cors").await;

        server.send(redeem_request(&caller.token, &[]), Some(PEER)).await;

        assert_eq!(attempt_ip_hash(&server, &caller.profile_id).await, ip_hash(PEER));
    }
}

// ---------------------------------------------------------------------------------------------
// B12: TRUSTED_PROXY_HOPS in the environment
// ---------------------------------------------------------------------------------------------

/// A complete, valid environment for the server (crates/server/.env.example's table), minus the hops.
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
    // SURFACE §11.3: the server refuses any version but the one it was compiled with.
    source.insert("CATALOG_VERSION".to_string(), jackioh_cards::catalog_version().to_string());
    source
}

fn with_hops(source: IndexMap<String, String>, hops: &str) -> IndexMap<String, String> {
    let mut source = source;
    source.insert("TRUSTED_PROXY_HOPS".to_string(), hops.to_string());
    source
}

/// Every problem `load_env` reports for `source`, which must be refused.
fn problems(source: &IndexMap<String, String>) -> String {
    match load_env(source) {
        Ok(_) => panic!("the environment was accepted: {source:?}"),
        Err(problems) => problems.to_string(),
    }
}

mod r190_b12_trusted_proxy_hops {
    use super::*;

    #[test]
    fn r190_b12_defaults_to_default_trusted_proxy_hops_when_it_is_not_set() {
        let env = load_env(&valid_env()).expect("a valid environment");
        assert_eq!(env.trusted_proxy_hops as i64, DEFAULT_TRUSTED_PROXY_HOPS as i64);
    }

    #[test]
    fn r190_b12_accepts_every_integer_from_0_to_max_trusted_proxy_hops() {
        for hops in 0..=MAX_TRUSTED_PROXY_HOPS as i64 {
            let env = load_env(&with_hops(valid_env(), &hops.to_string())).expect("a valid environment");
            assert_eq!(env.trusted_proxy_hops as i64, hops, "{hops}");
        }
    }

    #[test]
    fn r190_b12_lists_a_value_above_max_trusted_proxy_hops_as_a_problem() {
        let above = (MAX_TRUSTED_PROXY_HOPS as i64 + 1).to_string();
        assert!(problems(&with_hops(valid_env(), &above)).contains("TRUSTED_PROXY_HOPS"));
    }

    #[test]
    fn r190_b12_lists_a_negative_value_as_a_problem() {
        assert!(problems(&with_hops(valid_env(), "-1")).contains("TRUSTED_PROXY_HOPS"));
    }

    #[test]
    fn r190_b12_lists_a_fractional_value_as_a_problem() {
        assert!(problems(&with_hops(valid_env(), "1.5")).contains("TRUSTED_PROXY_HOPS"));
    }

    #[test]
    fn r190_b12_lists_a_value_that_is_not_a_number_as_a_problem() {
        for value in ["one", "NaN", "1 hop"] {
            assert!(problems(&with_hops(valid_env(), value)).contains("TRUSTED_PROXY_HOPS"), "{value}");
        }
    }

    #[test]
    fn r190_b12_adds_its_problem_to_the_others_rather_than_replacing_them() {
        let mut without_pepper = valid_env();
        without_pepper.shift_remove("CODE_PEPPER");
        let found = problems(&with_hops(without_pepper, "-1"));
        assert!(found.contains("TRUSTED_PROXY_HOPS"), "{found}");
        assert!(found.contains("CODE_PEPPER"), "{found}");
    }

    #[test]
    fn r190_b12_is_a_server_only_variable() {
        assert!(SERVER_ONLY_ENV_VARS.contains(&"TRUSTED_PROXY_HOPS"));
    }
}

// ---------------------------------------------------------------------------------------------
// B12: the api.forwarded_for calibration log
// ---------------------------------------------------------------------------------------------

mod r190_b12_the_api_forwarded_for_log {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r190_b12_logs_the_fewest_entries_seen_so_far_each_time_a_request_carries_fewer_and_never_an_address() {
        let (log, _guard) = record();
        let server = open_server(&test_env()).await;
        let chain = ["198.51.100.61", "198.51.100.62", "203.0.113.63"];

        server.send(open_request(&[("x-forwarded-for", &chain.join(", "))]), Some(PEER)).await;
        server.send(open_request(&[("x-forwarded-for", "198.51.100.64, 203.0.113.65, 203.0.113.60")]), Some(PEER)).await;
        server.send(open_request(&[("x-forwarded-for", "203.0.113.66")]), Some(PEER)).await;
        server.send(open_request(&[("x-forwarded-for", "198.51.100.64, 203.0.113.65")]), Some(PEER)).await;

        let data: Vec<Value> = log.forwarded().into_iter().map(|entry| Value::Object(entry.data)).collect();
        assert_eq!(
            data,
            vec![
                json!({ "fewestEntries": chain.len(), "trustedProxyHops": 1 }),
                json!({ "fewestEntries": 1, "trustedProxyHops": 1 }),
            ]
        );
        assert!(log.forwarded().iter().all(|entry| entry.level == "info"));

        let everything = serde_json::to_string(&log.entries()).expect("the log serialises");
        let mut addresses = chain.to_vec();
        addresses.extend(["198.51.100.64", "203.0.113.65", "203.0.113.66", PEER]);
        for address in addresses {
            assert!(!everything.contains(address), "{address}");
        }

        // A second router reports for itself: a second App (TS kept the minimum per router).
        let second = open_server(&test_env()).await;
        second.send(open_request(&[("x-forwarded-for", "203.0.113.67")]), Some(PEER)).await;
        assert_eq!(log.forwarded().len(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b12_a_caller_who_writes_its_own_entries_cannot_raise_the_count_the_operator_reads() {
        // The adversarial panel's finding: one sample from whoever sent the first X-Forwarded-For
        // request was the whole signal, and a scanner could make it say anything. Callers only ever
        // ADD entries, so the minimum over every request is the proxies' own count.
        let (log, _guard) = record();
        let server = open_server(&test_env()).await;

        server.send(open_request(&[("x-forwarded-for", "10.0.0.1, 10.0.0.2, 10.0.0.3, 203.0.113.70")]), Some(PEER)).await;
        server.send(open_request(&[("x-forwarded-for", "203.0.113.71")]), Some(PEER)).await;

        let latest = log.forwarded().last().map(|entry| entry.data["fewestEntries"].clone());
        assert_eq!(latest, Some(json!(1)));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b12_a_hostile_caller_can_cause_only_a_handful_of_lines_per_router() {
        let (log, _guard) = record();
        let server = open_server(&test_env()).await;

        // Every request one entry shorter than the one before, from far more entries than any proxy.
        for count in (1..=40).rev() {
            let chain: Vec<String> = (0..count).map(|i| format!("10.0.{}.{}", i / 250, i % 250)).collect();
            server.send(open_request(&[("x-forwarded-for", &chain.join(", "))]), Some(PEER)).await;
        }

        assert!(log.forwarded().len() <= MAX_TRUSTED_PROXY_HOPS + 2, "{:?}", log.forwarded());
        assert_eq!(log.forwarded().last().map(|entry| entry.data["fewestEntries"].clone()), Some(json!(1)));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b12_waits_for_the_first_request_that_carries_x_forwarded_for() {
        let (log, _guard) = record();
        let server = open_server(&test_env()).await;

        server.send(open_request(&[]), Some(PEER)).await;
        server.send(open_request(&[("x-real-ip", "198.51.100.77")]), Some(PEER)).await;
        assert_eq!(log.forwarded().len(), 0);

        server.send(open_request(&[("x-forwarded-for", "198.51.100.68, 203.0.113.69")]), Some(PEER)).await;
        let forwarded = log.forwarded();
        assert_eq!(forwarded.len(), 1);
        assert_eq!(forwarded[0].level, "info");
        assert_eq!(forwarded[0].event, "api.forwarded_for");
        assert_eq!(Value::Object(forwarded[0].data.clone()), json!({ "fewestEntries": 2, "trustedProxyHops": 1 }));
    }

    #[tokio::test(start_paused = true)]
    async fn r190_b12_reports_the_trusted_hops_the_router_was_given_and_the_default_when_none_was() {
        let (log, _guard) = record();
        let hops = std::cmp::min(MAX_TRUSTED_PROXY_HOPS as i64, 2);
        let server = open_server(&env_with_hops(&hops.to_string())).await;
        server.send(open_request(&[("x-forwarded-for", "198.51.100.70")]), Some(PEER)).await;
        let data: Vec<Value> = log.forwarded().into_iter().map(|entry| Value::Object(entry.data)).collect();
        assert_eq!(data, vec![json!({ "fewestEntries": 1, "trustedProxyHops": hops })]);

        let (unset_log, _unset_guard) = record();
        let unset = open_server(&env_without_hops()).await;
        unset.send(open_request(&[("x-forwarded-for", "198.51.100.70")]), Some(PEER)).await;
        let data: Vec<Value> = unset_log.forwarded().into_iter().map(|entry| Value::Object(entry.data)).collect();
        assert_eq!(data, vec![json!({ "fewestEntries": 1, "trustedProxyHops": DEFAULT_TRUSTED_PROXY_HOPS as i64 })]);
    }
}

// ---------------------------------------------------------------------------------------------
// R190: the default trusts no proxy
// ---------------------------------------------------------------------------------------------

mod r190_the_default_number_of_trusted_hops {
    use super::*;

    #[test]
    fn r190_is_zero_so_a_server_with_no_proxy_in_front_never_reads_a_caller_s_x_forwarded_for() {
        assert_eq!(DEFAULT_TRUSTED_PROXY_HOPS as i64, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r190_a_direct_caller_rotating_x_forwarded_for_stays_in_its_peer_s_r157_bucket_under_the_default() {
        let server = open_server(&env_without_hops()).await;
        let direct = Some("198.51.100.200");

        for i in 0..LIMIT {
            let spoofed = format!("10.{}.{}.9", i % 250, i / 250);
            assert_eq!(server.send(open_request(&[("x-forwarded-for", &spoofed)]), direct).await.status, 200, "request {i}");
        }
        let next = server.send(open_request(&[("x-forwarded-for", "172.16.9.9")]), direct).await;
        assert_eq!(next.status, 429);
    }
}

// ---------------------------------------------------------------------------------------------
// R190: an IPv6 client is counted by its /56
// ---------------------------------------------------------------------------------------------

mod r190_rate_limit_address {
    use super::*;

    #[test]
    fn r190_keys_every_address_in_one_ipv6_56_alike_and_different_56s_apart() {
        let key = rate_limit_address("2001:db8:1:2::1");
        assert_eq!(IPV6_RATE_LIMIT_PREFIX_BITS as i64, 56);
        for same in [
            "2001:db8:1:2::2",
            "2001:DB8:1:2:ffff:ffff:ffff:ffff",
            "2001:0db8:0001:0002:0:0:0:9",
            "[2001:db8:1:2::3]:443",
            // Another /64 of the same /56: one home's delegation is one key.
            "2001:db8:1:ff::1",
        ] {
            assert_eq!(rate_limit_address(same), key, "{same}");
        }
        assert_ne!(rate_limit_address("2001:db8:1:102::1"), key);
    }

    #[test]
    fn r190_one_host_s_56_delegation_cannot_be_split_into_a_bucket_per_64() {
        // The panel's pair: two /64s of one typical home /56.
        assert_eq!(rate_limit_address("2a02:8108:1:abff::1"), rate_limit_address("2a02:8108:1:ab00::1"));
        assert_eq!(rate_limit_address("2a02:8108:1:ab00::1"), format!("2a02:8108:1:ab00:0:0:0:0/{IPV6_RATE_LIMIT_PREFIX_BITS}"));
    }

    #[test]
    fn r190_reads_an_ipv4_mapped_ipv6_address_as_the_ipv4_address() {
        assert_eq!(rate_limit_address("::ffff:192.0.2.1"), "192.0.2.1");
        assert_eq!(rate_limit_address("::ffff:c000:201"), "192.0.2.1");
    }

    #[test]
    fn r190_keeps_an_ipv4_address_whole_without_a_port_and_anything_else_as_written() {
        assert_eq!(rate_limit_address("192.0.2.1"), "192.0.2.1");
        assert_eq!(rate_limit_address("192.0.2.1:8080"), "192.0.2.1");
        assert_eq!(rate_limit_address(UNKNOWN_CLIENT_ADDRESS), UNKNOWN_CLIENT_ADDRESS);
        assert_eq!(rate_limit_address(" Not-An-Address "), "not-an-address");
    }

    #[tokio::test(start_paused = true)]
    async fn r190_a_host_rotating_addresses_inside_its_56_does_not_escape_section_9_4_step_3() {
        let server = code_server().await;
        let caller = seed_caller(&server, "rotator").await;
        flood_address(&server, &rate_limit_address("2001:db8:1:2::1")).await;

        let rotated = server.send(redeem_request(&caller.token, &[("x-forwarded-for", "2001:db8:1:ee::2")]), Some("10.0.0.1")).await;

        assert_eq!(rotated.status, 429);
        assert_eq!(rotated.error_code(), "rate_limited");
    }

    #[tokio::test(start_paused = true)]
    async fn r190_an_ipv4_peer_reported_as_ipv4_mapped_ipv6_shares_its_bucket() {
        let server = code_server().await;
        let caller = seed_caller(&server, "dual-stack").await;

        server.send(redeem_request(&caller.token, &[]), Some("::ffff:192.0.2.10")).await;

        assert_eq!(attempt_ip_hash(&server, &caller.profile_id).await, ip_hash("192.0.2.10"));
    }
}
