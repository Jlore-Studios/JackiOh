//! The HTTP layer: an axum `Request` in, an axum `Response` out, and TS's own router in between
//! (port of `apps/server/src/api/http.ts`).
//!
//! Handlers are plain async functions so every endpoint is unit-testable without a listener, and
//! the composition root (`app.rs`) mounts `dispatch` as the one fallback of its axum `Router`
//! (SURFACE §11.2). Route declarations carry their own auth requirement, which is how SPEC §9.4's
//! "a pending account can see the code screen and nothing else" is enforced in one place instead of
//! at the top of every handler.
//!
//! TS's matcher is kept rather than axum's routing (SURFACE §11.3): routes are tried in
//! registration order, and a wrong method on a known path answers **404** `not_found`
//! "method not allowed for this path".
//!
//! `api/deps.ts` is not ported (SURFACE §11.3): its defaults are read from `crate::config`
//! directly, `floodLimits.apiRequestsPerMinute` is `API_REQUESTS_PER_MINUTE`, and its
//! `consoleLogger` is `log_info`/`log_warn`/`log_alert` below over `tracing`.

use std::collections::VecDeque;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};

use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::app::{App, Handler, Route};
use crate::auth::AuthUser;
use crate::config::{
    API_MAX_BODY_BYTES, API_REQUESTS_PER_MINUTE, IPV6_RATE_LIMIT_PREFIX_BITS, MAX_TRUSTED_PROXY_HOPS,
    RATING_START,
};
use crate::db::store::{Profile, ProfileCreateInput, ProfileStatus, StoreError};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Every error code the API answers with (TS `ApiErrorCode`), serialised as TS's literals.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    BadRequest,
    Unauthorized,
    EmailUnverified,
    AccountPending,
    AccountBanned,
    NotFound,
    Conflict,
    AlreadyInMatch,
    AlreadyQueued,
    InvalidCode,
    UpdateRequired,
    LoadoutInvalid,
    MatchNotFinished,
    SeriesGame,
    DoubleRequiresRanked,
    RateLimited,
    Unavailable,
    Internal,
}

impl ApiErrorCode {
    /// TS's `STATUS` table: the HTTP status each code answers with.
    pub fn status(self) -> u16 {
        match self {
            ApiErrorCode::BadRequest => 400,
            ApiErrorCode::Unauthorized => 401,
            ApiErrorCode::EmailUnverified => 403,
            ApiErrorCode::AccountPending => 403,
            ApiErrorCode::AccountBanned => 403,
            ApiErrorCode::NotFound => 404,
            ApiErrorCode::Conflict => 409,
            ApiErrorCode::AlreadyInMatch => 409,
            ApiErrorCode::AlreadyQueued => 409,
            ApiErrorCode::InvalidCode => 400,
            ApiErrorCode::UpdateRequired => 409,
            ApiErrorCode::LoadoutInvalid => 422,
            ApiErrorCode::MatchNotFinished => 422,
            ApiErrorCode::SeriesGame => 422,
            ApiErrorCode::DoubleRequiresRanked => 422,
            ApiErrorCode::RateLimited => 429,
            ApiErrorCode::Unavailable => 503,
            ApiErrorCode::Internal => 500,
        }
    }

    /// The code as the client reads it (`"bad_request"`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            ApiErrorCode::BadRequest => "bad_request",
            ApiErrorCode::Unauthorized => "unauthorized",
            ApiErrorCode::EmailUnverified => "email_unverified",
            ApiErrorCode::AccountPending => "account_pending",
            ApiErrorCode::AccountBanned => "account_banned",
            ApiErrorCode::NotFound => "not_found",
            ApiErrorCode::Conflict => "conflict",
            ApiErrorCode::AlreadyInMatch => "already_in_match",
            ApiErrorCode::AlreadyQueued => "already_queued",
            ApiErrorCode::InvalidCode => "invalid_code",
            ApiErrorCode::UpdateRequired => "update_required",
            ApiErrorCode::LoadoutInvalid => "loadout_invalid",
            ApiErrorCode::MatchNotFinished => "match_not_finished",
            ApiErrorCode::SeriesGame => "series_game",
            ApiErrorCode::DoubleRequiresRanked => "double_requires_ranked",
            ApiErrorCode::RateLimited => "rate_limited",
            ApiErrorCode::Unavailable => "unavailable",
            ApiErrorCode::Internal => "internal",
        }
    }
}

impl std::fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The one error shape the client ever sees: `{ "error": { code, message, details? } }`.
///
/// `retry_after_ms` is the wait a `rate_limited` refusal carries (R192); `rate_limited` writes it
/// both here and as `details.retryAfterMs`, which is what the body shows. An `Internal` error is
/// TS's "a thrown non-`ApiError`": `dispatch` logs its message and answers "something went wrong".
#[derive(Clone, Debug)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    pub details: Option<Value>,
    pub retry_after_ms: Option<i64>,
}

impl ApiError {
    pub fn new(code: ApiErrorCode, message: impl Into<String>) -> ApiError {
        ApiError {
            code,
            message: message.into(),
            details: None,
            retry_after_ms: None,
        }
    }

    pub fn with_details(code: ApiErrorCode, message: impl Into<String>, details: Value) -> ApiError {
        ApiError {
            code,
            message: message.into(),
            details: Some(details),
            retry_after_ms: None,
        }
    }

    /// TS's `throw new Error(message)` inside a handler: the router turns it into a 500 whose body
    /// says only "something went wrong", and logs `message` as `handler.threw`.
    pub fn internal(message: impl Into<String>) -> ApiError {
        ApiError::new(ApiErrorCode::Internal, message)
    }

    /// TS `error.status`.
    pub fn status(&self) -> u16 {
        self.code.status()
    }

    /// TS `error.body()`.
    pub fn body(&self) -> Value {
        let mut error = Map::new();
        error.insert("code".to_string(), Value::String(self.code.as_str().to_string()));
        error.insert("message".to_string(), Value::String(self.message.clone()));
        if let Some(details) = &self.details {
            error.insert("details".to_string(), details.clone());
        }
        json!({ "error": Value::Object(error) })
    }

    /// The body as the client receives it, its keys in TS's order (`code`, `message`, `details`).
    fn body_text(&self) -> String {
        #[derive(Serialize)]
        struct Fields<'a> {
            code: &'a str,
            message: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            details: Option<&'a Value>,
        }
        #[derive(Serialize)]
        struct Envelope<'a> {
            error: Fields<'a>,
        }
        let envelope = Envelope {
            error: Fields {
                code: self.code.as_str(),
                message: &self.message,
                details: self.details.as_ref(),
            },
        };
        serde_json::to_string(&envelope).unwrap_or_else(|_| self.body().to_string())
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

/// A store failure is TS's thrown `Error` from inside a handler: a 500.
impl From<StoreError> for ApiError {
    fn from(error: StoreError) -> ApiError {
        ApiError::internal(error.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> ApiError {
        ApiError::internal(error.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        error_response(&self)
    }
}

pub fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::new(ApiErrorCode::BadRequest, message)
}

/// SPEC §11 R192: a refusal because the caller is going too fast says so, with how long to wait.
/// The wait travels as `details.retryAfterMs` and, through `error_response`, as `Retry-After`.
pub fn rate_limited(message: impl Into<String>, retry_after_ms: i64) -> ApiError {
    ApiError {
        code: ApiErrorCode::RateLimited,
        message: message.into(),
        details: Some(json!({ "retryAfterMs": retry_after_ms })),
        retry_after_ms: Some(retry_after_ms),
    }
}

/// The wait of a `rate_limited` error when it is a finite number >= 0, else none: the field, or
/// failing that `details.retryAfterMs` (an error built by hand with that detail).
fn retry_after_ms_of(error: &ApiError) -> Option<f64> {
    if error.code != ApiErrorCode::RateLimited {
        return None;
    }
    if let Some(wait) = error.retry_after_ms
        && wait >= 0
    {
        return Some(wait as f64);
    }
    let wait = error
        .details
        .as_ref()?
        .as_object()?
        .get("retryAfterMs")?
        .as_f64()?;
    if !wait.is_finite() || wait < 0.0 {
        return None;
    }
    Some(wait)
}

const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";
const NO_STORE: &str = "no-store";

fn json_response(status: u16, text: String, cache_control: &str, retry_after: Option<String>) -> Response {
    let mut response = Response::new(Body::from(text));
    *response.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(JSON_CONTENT_TYPE));
    if let Ok(value) = HeaderValue::from_str(cache_control) {
        headers.insert(header::CACHE_CONTROL, value);
    }
    if let Some(wait) = retry_after
        && let Ok(value) = HeaderValue::from_str(&wait)
    {
        headers.insert(header::RETRY_AFTER, value);
    }
    response
}

/// `content-type: application/json; charset=utf-8` and `cache-control: no-store`.
pub fn json(status: u16, body: Value) -> Response {
    json_response(status, body.to_string(), NO_STORE, None)
}

pub fn ok(body: Value) -> Response {
    json(200, body)
}

/// `ok` for anything serialisable: the body as its JSON.
pub fn ok_of<T: Serialize>(body: &T) -> ApiResult {
    Ok(ok(to_json(body)?))
}

/// A value as JSON, a failure being a 500 (a map keyed by something that is not a string).
pub fn to_json<T: Serialize>(value: &T) -> Result<Value, ApiError> {
    Ok(serde_json::to_value(value)?)
}

/// The one error response. A `rate_limited` error whose wait is a finite number >= 0 also carries
/// `Retry-After` in whole seconds, rounded up (R192). No other error gains a header: R145's
/// `invalid_code` in particular stays byte for byte and header for header what it was.
pub fn error_response(error: &ApiError) -> Response {
    let text = error.body_text();
    match retry_after_ms_of(error) {
        None => json_response(error.status(), text, NO_STORE, None),
        Some(wait) => {
            let seconds = (wait / 1000.0).ceil();
            json_response(
                error.status(),
                text,
                NO_STORE,
                Some(format!("{}", seconds as i64)),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// What every handler is handed (TS `ApiRequest`, SURFACE §11.2).
///
/// - `caller`: set when the route declares `AuthLevel::User` or `Active` (TS `user` and `profile`).
/// - `params`: the `:name` segments of the route's path.
/// - `query`: the URL's search parameters, the first value of each name (TS `searchParams.get`).
/// - `body`: the parsed JSON body; `{}` for a request without one. Anything but a JSON object is
///   refused before a handler runs.
/// - `address`: §9.4, §9.8: the client address is only ever seen hashed (TS `ipHash`).
pub struct Req {
    pub caller: Option<Caller>,
    pub params: IndexMap<String, String>,
    pub query: IndexMap<String, String>,
    pub body: Value,
    pub address: String,
    pub headers: HeaderMap,
}

/// The signed-in caller of a `user` or `active` route.
pub struct Caller {
    pub profile: Profile,
    pub user: AuthUser,
}

/// What the host knows about the connection a request arrived on, which the request itself does
/// not carry. `dispatch` fills it from axum's `ConnectInfo`; a test inserts one by hand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestContext {
    pub peer_address: Option<String>,
}

/// The address a request is keyed on when neither the forwarded chain nor the socket names one.
pub const UNKNOWN_CLIENT_ADDRESS: &str = "unknown";

/// A header's value as text (every value of a repeated header, joined as the Fetch API's
/// `Headers.get` joins them), or none when the header is absent.
fn header_text(headers: &HeaderMap, name: &str) -> Option<String> {
    let values: Vec<String> = headers
        .get_all(name)
        .iter()
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(values.join(", "))
    }
}

/// Every `X-Forwarded-For` entry, split on commas and trimmed, with empty entries dropped.
fn forwarded_entries(headers: &HeaderMap) -> Vec<String> {
    let Some(raw) = header_text(headers, "x-forwarded-for") else {
        return Vec::new();
    };
    raw.split(',')
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// SPEC §11 R190: the address a per-IP limit counts (§9.4 step 3, R157).
///
/// Behind a proxy every request arrives from the proxy, so the client's address is read from
/// `X-Forwarded-For` — but only from the entry the deployment's own proxies wrote. Each trusted hop
/// appends one entry on the right, so with `trusted_proxy_hops` hops the client is
/// `entries[length - hops]`. Everything to the left of it was written by the caller, who can put
/// anything there: reading the leftmost entry, as this function once did, let a caller pick a fresh
/// bucket for every request by changing one header. `CF-Connecting-IP` and `X-Real-IP` are never
/// read, for the same reason: nothing in this deployment writes them, so a caller can.
///
/// With fewer entries than trusted hops (none included), or with `trusted_proxy_hops` 0, the
/// request came from something other than the proxy chain and is keyed on the socket's peer
/// address, or on `UNKNOWN_CLIENT_ADDRESS` when the host gave none.
pub fn client_address(headers: &HeaderMap, peer_address: Option<&str>, trusted_proxy_hops: usize) -> String {
    let entries = forwarded_entries(headers);
    if trusted_proxy_hops >= 1
        && entries.len() >= trusted_proxy_hops
        && let Some(entry) = entries.get(entries.len() - trusted_proxy_hops)
    {
        return entry.clone();
    }
    let peer = peer_address.map(str::trim).unwrap_or("");
    if peer.is_empty() {
        UNKNOWN_CLIENT_ADDRESS.to_string()
    } else {
        peer.to_string()
    }
}

/// An IPv6 address as its eight 16-bit groups, or none when it is not one. A trailing dotted IPv4
/// (`::ffff:192.0.2.1`) is the last two groups.
fn ipv6_groups(address: &str) -> Option<[u16; 8]> {
    address.parse::<Ipv6Addr>().ok().map(|parsed| parsed.segments())
}

/// TS `isIPv4`: a dotted quad, each octet 0–255 without a leading zero.
fn is_ipv4(address: &str) -> bool {
    address.parse::<Ipv4Addr>().is_ok()
}

/// 1 to `max` ASCII digits.
fn is_digits(text: &str, max: usize) -> bool {
    !text.is_empty() && text.len() <= max && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// TS `/^\d{1,3}(?:\.\d{1,3}){3}$/`.
fn is_dotted_quad_shape(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 4 && parts.iter().all(|part| is_digits(part, 3))
}

/// TS `/^\[([^\]]+)\](?::\d+)?$/`: the bracketed host, when the address is one.
fn bracketed_host(address: &str) -> Option<&str> {
    let rest = address.strip_prefix('[')?;
    let close = rest.find(']')?;
    let host = &rest[..close];
    if host.is_empty() {
        return None;
    }
    let after = &rest[close + 1..];
    if after.is_empty() {
        return Some(host);
    }
    let port = after.strip_prefix(':')?;
    if is_digits(port, usize::MAX) {
        Some(host)
    } else {
        None
    }
}

/// TS `/^(\d{1,3}(?:\.\d{1,3}){3}):\d+$/`: the IPv4 address without its port, when it has one.
fn ipv4_without_port(address: &str) -> Option<&str> {
    let (host, port) = address.split_once(':')?;
    if is_dotted_quad_shape(host) && is_digits(port, usize::MAX) {
        Some(host)
    } else {
        None
    }
}

/// SPEC §11 R190: the key a per-IP limit counts an address under, before it is hashed, with the
/// IPv6 prefix at `IPV6_RATE_LIMIT_PREFIX_BITS` (TS's default argument).
pub fn rate_limit_address(raw: &str) -> String {
    rate_limit_address_with_prefix(raw, IPV6_RATE_LIMIT_PREFIX_BITS as i64)
}

/// SPEC §11 R190: the key a per-IP limit counts an address under, before it is hashed.
///
/// An IPv4 address is itself. An IPv6 address is its first `prefix_bits` bits (a /56 by default):
/// a host holds at least a /64 and often a whole /56 delegation, so keying on the full address gave
/// one host 2^64 fresh buckets for §9.4 step 3 and R157, and keying on the /64 still gave it 256. An
/// IPv4-mapped IPv6 address (`::ffff:192.0.2.1`, how a dual-stack socket reports an IPv4 peer) is
/// the IPv4 address, so one caller is one key whichever way it arrived. Brackets, a zone and an
/// IPv4 port are dropped first. Anything else (the `UNKNOWN_CLIENT_ADDRESS` marker, a malformed
/// entry) is keyed as written, lower-cased.
pub fn rate_limit_address_with_prefix(raw: &str, prefix_bits: i64) -> String {
    let lowered = raw.trim().to_lowercase();
    let mut address: &str = &lowered;
    if let Some(host) = bracketed_host(address) {
        address = host;
    }
    if let Some(host) = ipv4_without_port(address) {
        address = host;
    }
    let address = match address.find('%') {
        Some(at) => &address[..at],
        None => address,
    };
    if is_ipv4(address) {
        return address.to_string();
    }

    let Some(groups) = ipv6_groups(address) else {
        return raw.trim().to_lowercase();
    };
    let mapped = groups[..5].iter().all(|group| *group == 0) && groups[5] == 0xffff;
    if mapped {
        let high = groups[6];
        let low = groups[7];
        return format!("{}.{}.{}.{}", high >> 8, high & 0xff, low >> 8, low & 0xff);
    }
    let bits = prefix_bits.clamp(0, 128);
    let masked: Vec<String> = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let keep = (bits - index as i64 * 16).clamp(0, 16);
            let value = if keep == 0 {
                0
            } else {
                u32::from(*group) & ((0xffff_u32 << (16 - keep)) & 0xffff)
            };
            format!("{value:x}")
        })
        .collect();
    format!("{}/{}", masked.join(":"), bits)
}

/// The token of an `Authorization: Bearer <token>` header (the scheme in any case).
pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = header_text(headers, "authorization")?;
    let trimmed = raw.trim();
    let scheme = trimmed.get(..6)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let rest = &trimmed[6..];
    let token = rest.trim_start();
    if token.len() == rest.len() || token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

/// The refusal for a body past `API_MAX_BODY_BYTES`, raised before any JSON parsing.
fn body_too_large() -> ApiError {
    bad_request("the request body is too large")
}

/// The body as text, refused once it passes `API_MAX_BODY_BYTES`. A declared `content-length` over
/// the cap is refused without reading; otherwise the body is read up to the cap and abandoned the
/// moment it passes it, so a huge body is never buffered whole.
async fn read_body_text(headers: &HeaderMap, body: Body) -> Result<String, ApiError> {
    let cap = API_MAX_BODY_BYTES;
    if let Some(declared) = header_text(headers, "content-length")
        && let Ok(length) = declared.trim().parse::<f64>()
        && length > cap as f64
    {
        return Err(body_too_large());
    }
    let bytes = axum::body::to_bytes(body, cap)
        .await
        .map_err(|_| body_too_large())?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    // TS's `TextDecoder` drops a leading byte-order mark.
    Ok(match text.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_string(),
        None => text,
    })
}

async fn read_body(method: &Method, headers: &HeaderMap, body: Body) -> Result<Value, ApiError> {
    if *method == Method::GET || *method == Method::HEAD {
        return Ok(Value::Object(Map::new()));
    }
    let text = read_body_text(headers, body).await?;
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    let parsed: Value =
        serde_json::from_str(&text).map_err(|_| bad_request("the request body must be JSON"))?;
    if !parsed.is_object() {
        return Err(bad_request("the request body must be a JSON object"));
    }
    Ok(parsed)
}

// Small typed readers, so no handler repeats a `match` on the value's type.

pub fn str(body: &Value, key: &str) -> Result<String, ApiError> {
    match body.get(key) {
        Some(Value::String(value)) if !value.is_empty() => Ok(value.clone()),
        _ => Err(bad_request(format!("\"{key}\" must be a string"))),
    }
}

pub fn optional_str(body: &Value, key: &str) -> Result<Option<String>, ApiError> {
    match body.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(bad_request(format!("\"{key}\" must be a string"))),
    }
}

pub fn bool(body: &Value, key: &str) -> Result<bool, ApiError> {
    match body.get(key) {
        Some(Value::Bool(value)) => Ok(*value),
        _ => Err(bad_request(format!("\"{key}\" must be a boolean"))),
    }
}

fn strings_of(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

pub fn string_list(body: &Value, key: &str) -> Result<Vec<String>, ApiError> {
    body.get(key)
        .and_then(strings_of)
        .ok_or_else(|| bad_request(format!("\"{key}\" must be an array of strings")))
}

pub fn deck_list(body: &Value, key: &str) -> Result<Vec<Vec<String>>, ApiError> {
    let bad = || bad_request(format!("\"{key}\" must be an array of arrays of card ids"));
    let decks = body.get(key).and_then(Value::as_array).ok_or_else(bad)?;
    decks
        .iter()
        .map(|deck| strings_of(deck).ok_or_else(bad))
        .collect()
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// What a handler returns: its response, or the error `dispatch` answers with. Every handler, here
/// and in part 19, is `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult`; `app.rs` boxes each
/// into its `Handler` with `h!` and lists them in `ROUTES` as `(method, path, AuthLevel, handler)`,
/// in TS's `allRoutes()` order (TS's `Route`, `route()` and `create*Routes()`).
pub type ApiResult = Result<Response, ApiError>;

/// TS `RouteAuth`.
///
/// - `None`: open (the catalog, the public statistics).
/// - `User`: a valid access token and a profile, whatever its status — the code screen (§9.4).
/// - `Active`: additionally `profiles.status == active`, which is what gives a pending account a
///   403 from collection, decks and queue (§9.4, BUILD M6-T1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuthLevel {
    None,
    User,
    Active,
}

/// TS `decodeURIComponent`, or none where it would throw: a `%` not followed by two hex digits, or
/// bytes that are not UTF-8.
fn decode_uri_component(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let high = (*bytes.get(at + 1)? as char).to_digit(16)?;
            let low = (*bytes.get(at + 2)? as char).to_digit(16)?;
            out.push((high * 16 + low) as u8);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// A path segment decoded, or as it came when it is not valid percent-encoding (`%E0`, `%ZZ`, a lone
/// `%`). Undecodable, the segment names nothing, and each handler already refuses a value that
/// names nothing in its own words (a deck id that is not a UUID is a 400, an unknown series a 404).
fn decode_segment(raw: &str) -> String {
    decode_uri_component(raw).unwrap_or_else(|| raw.to_string())
}

fn match_path(pattern: &str, path: &str) -> Option<IndexMap<String, String>> {
    let want: Vec<&str> = pattern.split('/').collect();
    let got: Vec<&str> = path.split('/').collect();
    if want.len() != got.len() {
        return None;
    }
    let mut params = IndexMap::new();
    for (segment, actual) in want.iter().zip(got.iter()) {
        if let Some(name) = segment.strip_prefix(':') {
            if actual.is_empty() {
                return None;
            }
            params.insert(name.to_string(), decode_segment(actual));
            continue;
        }
        if segment != actual {
            return None;
        }
    }
    Some(params)
}

/// `application/x-www-form-urlencoded` decoding of one name or value: `+` is a space, `%XX` a byte
/// (a `%` that is not followed by two hex digits stays as written), then UTF-8 with replacement.
fn decode_form_component(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte == b'+' {
            out.push(b' ');
            at += 1;
            continue;
        }
        if byte == b'%' {
            let high = bytes.get(at + 1).and_then(|b| (*b as char).to_digit(16));
            let low = bytes.get(at + 2).and_then(|b| (*b as char).to_digit(16));
            if let (Some(high), Some(low)) = (high, low) {
                out.push((high * 16 + low) as u8);
                at += 3;
                continue;
            }
        }
        out.push(byte);
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The URL's search parameters, the first value of each name, as TS's `url.searchParams.get` reads.
fn parse_query(query: &str) -> IndexMap<String, String> {
    let mut out = IndexMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let name = decode_form_component(name);
        if !out.contains_key(&name) {
            out.insert(name, decode_form_component(value));
        }
    }
    out
}

/// Resolves the caller. A token that does not verify is 401; a verified user with no profile row
/// gets one created lazily as `pending` (§9.4: the account exists the moment auth says so, and it
/// stays pending until a code is redeemed).
pub async fn resolve_caller(app: &App, headers: &HeaderMap) -> Result<Caller, ApiError> {
    let Some(token) = bearer_token(headers) else {
        return Err(ApiError::new(ApiErrorCode::Unauthorized, "sign in first"));
    };
    let user = match app.auth.verify(&token).await {
        Ok(user) => user,
        Err(_) => return Err(ApiError::new(ApiErrorCode::Unauthorized, "sign in first")),
    };

    let mut tx = app.db.begin(None).await?;
    if let Some(existing) = tx.profiles_get_by_user_id(&user.user_id).await? {
        tx.commit().await?;
        return Ok(Caller {
            profile: existing,
            user,
        });
    }

    let profile = tx
        .profiles_create(&ProfileCreateInput {
            user_id: user.user_id.clone(),
            email: user.email.clone().unwrap_or_default(),
            rating: RATING_START,
            at: now_ms(),
            display_name: None,
        })
        .await?;
    tx.commit().await?;
    Ok(Caller { profile, user })
}

/// §9.4: the gate. Public so a non-HTTP caller (the WebSocket upgrade) uses the same rule.
pub fn assert_active(profile: &Profile) -> Result<(), ApiError> {
    if matches!(profile.status, ProfileStatus::Banned) {
        return Err(ApiError::new(
            ApiErrorCode::AccountBanned,
            "this account is banned",
        ));
    }
    if matches!(profile.status, ProfileStatus::Pending) {
        return Err(ApiError::new(
            ApiErrorCode::AccountPending,
            "redeem an invite code to activate this account",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// §9.8's per-account rate limit
// ---------------------------------------------------------------------------

/// Unit conversion, not configuration: R109 states its API allowance *per minute*, and
/// `API_REQUESTS_PER_MINUTE` is the number itself.
const API_RATE_WINDOW_MS: i64 = 60_000;

/// §9.8, R109: the bucket a request is counted in. Never one shared counter — see `dispatch`.
pub fn account_key(profile_id: &str) -> String {
    format!("account:{profile_id}")
}

/// The bucket for a request that named no account: an `AuthLevel::None` route, or a token that did
/// not verify. Prefixed apart from `account_key` so an IP hash can never land in a profile's bucket.
pub fn address_key(ip_hash: &str) -> String {
    format!("address:{ip_hash}")
}

#[derive(Default)]
struct RateLimiterState {
    hits: IndexMap<String, VecDeque<i64>>,
    swept_at: i64,
}

/// §9.8: "per-account rate limit at the API", R109: 300 requests per minute per account.
///
/// One sliding window per key, the same shape as the actor's per-seat `flood_exceeded`
/// (`actor/match_actor.rs`) — the other half of the same §9.8 row — so the two limits are one
/// mechanism read twice rather than two mechanisms that can drift.
///
/// A request that is over the limit is **not** recorded: like §9.4's attempt log (`codes.rs`), the
/// window drains, so a caller who keeps hammering cannot pin their own counter open for ever.
///
/// Idle keys are swept once per window rather than left to accumulate, so a long-lived process does
/// not hold a timestamp list for every account that ever called it.
///
/// The limiter is the router's state (TS held it in `createRouter`'s closure), so it also holds the
/// router's R190 signal, the fewest `X-Forwarded-For` entries seen (see `dispatch`). One lives
/// on each `App`: a fresh app starts with an empty window, so one test's flood cannot leak into
/// the next.
pub struct RateLimiter {
    limit: usize,
    window_ms: i64,
    state: Mutex<RateLimiterState>,
    fewest_forwarded: Mutex<Option<usize>>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl RateLimiter {
    pub fn new(limit: usize, window_ms: i64) -> RateLimiter {
        RateLimiter {
            limit,
            window_ms,
            state: Mutex::new(RateLimiterState::default()),
            fewest_forwarded: Mutex::new(None),
        }
    }

    /// True while this key is still inside its allowance; false once it is over it.
    pub fn allow(&self, key: &str, now: i64) -> bool {
        let window = self.window_ms;
        let mut state = lock(&self.state);
        if now - state.swept_at >= window {
            state.swept_at = now;
            state
                .hits
                .retain(|_, times| times.back().copied().unwrap_or(0) > now - window);
        }

        let times = state.hits.entry(key.to_string()).or_default();
        while times.front().is_some_and(|at| *at <= now - window) {
            times.pop_front();
        }
        if times.len() >= self.limit {
            return false;
        }
        times.push_back(now);
        true
    }

    /// R192: how long until this key has room again. 0 while it has room; otherwise the time until
    /// the oldest counted hit leaves the window (`oldest_counted_hit + window_ms - now`).
    pub fn retry_after_ms(&self, key: &str, now: i64) -> i64 {
        let window = self.window_ms;
        let state = lock(&self.state);
        let counted: Vec<i64> = state
            .hits
            .get(key)
            .map(|times| times.iter().copied().filter(|at| *at > now - window).collect())
            .unwrap_or_default();
        if counted.len() < self.limit {
            return 0;
        }
        let oldest = counted.first().copied().unwrap_or(now);
        (oldest + window - now).max(0)
    }

    /// How many keys are being tracked, so a test can see idle ones dropped.
    pub fn size(&self) -> usize {
        lock(&self.state).hits.len()
    }

    /// R190: records a request's (capped) forwarded-entry count; true when it is fewer than any
    /// before it, which is when the operator's line is logged.
    fn note_forwarded(&self, capped: usize) -> bool {
        let mut fewest = lock(&self.fewest_forwarded);
        match *fewest {
            Some(seen) if capped >= seen => false,
            _ => {
                *fewest = Some(capped);
                true
            }
        }
    }
}

/// TS `createRateLimiter(limit, windowMs)`.
pub fn create_rate_limiter(limit: usize, window_ms: i64) -> RateLimiter {
    RateLimiter::new(limit, window_ms)
}

/// §9.8's "per-account rate limit at the API": `API_REQUESTS_PER_MINUTE` a minute, the limiter
/// every `App` holds.
pub fn api_rate_limiter() -> RateLimiter {
    create_rate_limiter(API_REQUESTS_PER_MINUTE, API_RATE_WINDOW_MS)
}

impl Default for RateLimiter {
    fn default() -> RateLimiter {
        api_rate_limiter()
    }
}

/// The router (TS `createRouter(allRoutes(), deps)`'s closure): `routes` (`app.rs`'s `ROUTES`) tried
/// in order. It is the one fallback of `app::router`, behind the CORS layer, which answers
/// preflights first.
///
/// §9.8's "per-account rate limit at the API" is `app.limiter`. R137's reasoning applies to this
/// half of §9.8 as much as to the actor's: a single shared counter lets one caller spend everybody
/// else's budget, turning the anti-abuse limit into the abuse. So the key is the **account** —
/// `profiles.id`, the row §9.4 owns, not the auth user id and not the bearer token, either of which
/// a caller can hold several of for one account.
///
/// SPEC §11 R157 settles what to key a request that names no account on: §9.8 and R109 say "per
/// account", and an open route and a token that did not verify have none, so such a request "is
/// counted against its IP hash instead, in a namespace of its own" — never on one shared bucket,
/// which would be exactly the failure R137 describes. R157 also fixes the account key as
/// `profiles.id` rather than the auth user or the bearer token. The two namespaces are kept apart
/// by their prefixes.
///
/// R190's operator signal: the FEWEST `X-Forwarded-For` entries any request has carried. A caller
/// can add entries on the left but never remove the ones the deployment's own proxies append, so
/// the lowest count seen is the number of hops those proxies add, and `TRUSTED_PROXY_HOPS` must not
/// be more than it (docs/architecture.md §10). One sample would not do: the first request after a
/// deploy may be a scanner that wrote its own entries, and raising the hops to its count would
/// hand the key back to the caller. So a line is logged each time a request carries fewer entries
/// than any before it. Counts past `MAX_TRUSTED_PROXY_HOPS + 1` are all "too many", so a caller
/// can cause at most a handful of lines per router. No address is ever logged: two numbers only.
pub async fn dispatch(app: &Arc<App>, routes: &[Route], request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path().to_string();
    let context = parts
        .extensions
        .get::<RequestContext>()
        .cloned()
        .or_else(|| {
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| RequestContext {
                    peer_address: Some(info.0.ip().to_string()),
                })
        })
        .unwrap_or_default();
    let trusted_proxy_hops = app.env.trusted_proxy_hops;

    if parts.headers.contains_key("x-forwarded-for") {
        let entries = forwarded_entries(&parts.headers).len();
        let capped = entries.min(MAX_TRUSTED_PROXY_HOPS + 1);
        if app.limiter.note_forwarded(capped) {
            log_info(
                "api.forwarded_for",
                json!({ "fewestEntries": entries, "trustedProxyHops": trusted_proxy_hops }),
            );
        }
    }

    let mut path_matched = false;
    let mut body = Some(body);
    for (method, pattern, auth, handler) in routes {
        let Some(params) = match_path(pattern, &path) else {
            continue;
        };
        path_matched = true;
        if *method != parts.method.as_str() {
            continue;
        }
        let body = body.take().unwrap_or_else(Body::empty);
        let outcome = run_route(
            app,
            *auth,
            *handler,
            params,
            &parts,
            body,
            &context,
            trusted_proxy_hops,
            &path,
        )
        .await;
        return match outcome {
            Ok(response) => response,
            Err(error) if error.code == ApiErrorCode::Internal => {
                log_warn("handler.threw", json!({ "path": path, "message": error.message }));
                error_response(&ApiError::internal("something went wrong"))
            }
            Err(error) => error_response(&error),
        };
    }

    if path_matched {
        return error_response(&ApiError::new(
            ApiErrorCode::NotFound,
            "method not allowed for this path",
        ));
    }
    error_response(&ApiError::new(ApiErrorCode::NotFound, "no such endpoint"))
}

/// One matched route, from the address hash to the handler (the body of TS's `try`).
#[allow(clippy::too_many_arguments)]
async fn run_route(
    app: &Arc<App>,
    auth: AuthLevel,
    handler: Handler,
    params: IndexMap<String, String>,
    parts: &axum::http::request::Parts,
    body: Body,
    context: &RequestContext,
    trusted_proxy_hops: usize,
    path: &str,
) -> ApiResult {
    // §9.4, §9.8: the client address is only ever seen hashed. Read once, for the limiter and for
    // the request alike, and read as R190 says: the rightmost trusted hop, then the peer, with an
    // IPv6 client counted by its /56.
    let hashes = crate::api::crypto::hashes_for_pepper(&app.env.code_pepper);
    let address = hashes.ip(&rate_limit_address(&client_address(
        &parts.headers,
        context.peer_address.as_deref(),
        trusted_proxy_hops,
    )));

    // An address whose own budget is spent is refused BEFORE its token is resolved. Resolving a
    // token that does not verify locally can cost a round trip to the auth provider, and a flood of
    // bad tokens fills exactly this bucket (below), so without this check the limiter counted the
    // flood but still paid for every request of it. Only looked at, not counted: a request that
    // names an account is counted against the account, as R157 says.
    if auth != AuthLevel::None {
        let key = address_key(&address);
        let wait = app.limiter.retry_after_ms(&key, now_ms());
        if wait > 0 {
            log_warn("api.rate_limited", json!({ "path": path, "key": key }));
            return Err(rate_limited("too many requests; slow down", wait));
        }
    }

    // Resolved before the account's rate limit because only auth knows which account a request
    // belongs to, and the account is R109's key. A refusal is held rather than returned, so a flood
    // of bad tokens is still counted — against its address, since it named no account.
    let mut caller: Option<Caller> = None;
    let mut auth_error: Option<ApiError> = None;
    if auth != AuthLevel::None {
        match resolve_caller(app, &parts.headers).await {
            Ok(resolved) => caller = Some(resolved),
            // A failure that is not a refusal (the store) is TS's rethrown error: a 500 at once.
            Err(error) if error.code == ApiErrorCode::Internal => return Err(error),
            Err(error) => auth_error = Some(error),
        }
    }

    // §9.8: "per-account rate limit at the API" (R109: 300 a minute). Checked before the body is
    // read and before §9.4's gate, so a flood costs the least work this router can manage.
    let key = match &caller {
        None => address_key(&address),
        Some(resolved) => account_key(&resolved.profile.id),
    };
    let now = now_ms();
    if !app.limiter.allow(&key, now) {
        // §9.8: "Every rejected action is logged with its reason."
        log_warn("api.rate_limited", json!({ "path": path, "key": key }));
        // R192: the refusal says how long until the oldest counted request leaves the window.
        return Err(rate_limited(
            "too many requests; slow down",
            app.limiter.retry_after_ms(&key, now),
        ));
    }

    if let Some(error) = auth_error {
        return Err(error);
    }
    if auth == AuthLevel::Active
        && let Some(resolved) = &caller
    {
        assert_active(&resolved.profile)?;
    }

    let req = Req {
        caller,
        params,
        query: parse_query(parts.uri.query().unwrap_or("")),
        body: read_body(&parts.method, &parts.headers, body).await?,
        address,
        headers: parts.headers.clone(),
    };
    handler(app, req).await
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

/// TS `timers.now()`: epoch milliseconds, the server's one clock (`app::now_ms`: the wall clock
/// anchored once and advanced by tokio's, so a test's `tokio::time::pause()` and `advance()` move it;
/// SURFACE §11.3: `Timers` → `tokio::time`). Re-exported so every API module reads the same clock.
pub use crate::app::now_ms;

// ---------------------------------------------------------------------------
// Logging (TS `consoleLogger`, SURFACE §11.3: `tracing` JSON lines with the same event names)
// ---------------------------------------------------------------------------

/// One line at `info`: the event's name and its data.
pub fn log_info(event: &str, data: Value) {
    tracing::info!(event = event, data = %data);
}

/// One line at `warn`.
pub fn log_warn(event: &str, data: Value) {
    tracing::warn!(event = event, data = %data);
}

/// §9.4: the circuit breaker alerts; §9.8: unusual rejection rates raise an alert. At `error`,
/// marked `alert`.
pub fn log_alert(event: &str, data: Value) {
    tracing::error!(event = event, alert = true, data = %data);
}
