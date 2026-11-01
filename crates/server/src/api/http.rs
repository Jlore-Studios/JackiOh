//! The HTTP layer: an axum `Request` in and an axum `Response` out.
//!
//! Handlers are plain async functions, so every endpoint is unit-testable without a listener, and
//! `app.rs` mounts `dispatch` as the one fallback of its axum `Router` (SURFACE §11.2). Route
//! declarations carry their own auth requirement, so SPEC §9.4's "a pending account can see the code
//! screen and nothing else" is enforced in one place instead of in every handler.
//!
//! Routes are tried in registration order (SURFACE §11.3), and a wrong method on a known path
//! answers **404** `not_found` "method not allowed for this path".

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
    API_MAX_BODY_BYTES, API_REJECTION_ALERT_THRESHOLD, API_REJECTION_ALERT_WINDOW_MS,
    API_REQUESTS_PER_MINUTE, IPV6_RATE_LIMIT_PREFIX_BITS, KNOWN_PROFILES_MAX, MAX_TRUSTED_PROXY_HOPS,
    RATING_START,
};
use crate::db::store::{Profile, ProfileCreateInput, ProfileStatus, StoreError};

// Errors

/// Every error code the API answers with, serialized as snake_case literals.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    BadRequest,
    Unauthorized,
    EmailUnverified,
    AccountPending,
    AccountBanned,
    /// R757000: the age screen's answer is under `MIN_ACCOUNT_AGE_YEARS`; the account is deleted.
    UnderAge,
    /// R757000, R757001: the account owes the age check or agreement to the terms in force.
    ConsentOwed,
    NotFound,
    Conflict,
    /// R768: what was asked for is gone for good (a replay whose log the retention purge took).
    Gone,
    AlreadyInMatch,
    AlreadyQueued,
    InvalidCode,
    UpdateRequired,
    LoadoutInvalid,
    MatchNotFinished,
    SeriesGame,
    DoubleRequiresRanked,
    /// R768: a replay that cannot be shown; `details.reason` is the engine's `ReplayRefusal`.
    ReplayUnavailable,
    RateLimited,
    Unavailable,
    Internal,
    /// R1300000: a dev route asked by an account `DEV_PROFILE_IDS` does not list.
    Forbidden,
    /// R2124000: a spend's `Idempotency-Key` came before with another route or body.
    IdempotencyKeyReused,
}

impl ApiErrorCode {
    /// The HTTP status each code answers with.
    pub fn status(self) -> u16 {
        match self {
            ApiErrorCode::BadRequest => 400,
            ApiErrorCode::Unauthorized => 401,
            ApiErrorCode::EmailUnverified => 403,
            ApiErrorCode::AccountPending => 403,
            ApiErrorCode::AccountBanned => 403,
            ApiErrorCode::UnderAge => 403,
            ApiErrorCode::ConsentOwed => 403,
            ApiErrorCode::NotFound => 404,
            ApiErrorCode::Conflict => 409,
            ApiErrorCode::Gone => 410,
            ApiErrorCode::AlreadyInMatch => 409,
            ApiErrorCode::AlreadyQueued => 409,
            ApiErrorCode::InvalidCode => 400,
            ApiErrorCode::UpdateRequired => 409,
            ApiErrorCode::LoadoutInvalid => 422,
            ApiErrorCode::MatchNotFinished => 422,
            ApiErrorCode::SeriesGame => 422,
            ApiErrorCode::DoubleRequiresRanked => 422,
            ApiErrorCode::ReplayUnavailable => 422,
            ApiErrorCode::RateLimited => 429,
            ApiErrorCode::Unavailable => 503,
            ApiErrorCode::Internal => 500,
            ApiErrorCode::Forbidden => 403,
            ApiErrorCode::IdempotencyKeyReused => 422,
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
            ApiErrorCode::UnderAge => "under_age",
            ApiErrorCode::ConsentOwed => "consent_owed",
            ApiErrorCode::NotFound => "not_found",
            ApiErrorCode::Conflict => "conflict",
            ApiErrorCode::Gone => "gone",
            ApiErrorCode::AlreadyInMatch => "already_in_match",
            ApiErrorCode::AlreadyQueued => "already_queued",
            ApiErrorCode::InvalidCode => "invalid_code",
            ApiErrorCode::UpdateRequired => "update_required",
            ApiErrorCode::LoadoutInvalid => "loadout_invalid",
            ApiErrorCode::MatchNotFinished => "match_not_finished",
            ApiErrorCode::SeriesGame => "series_game",
            ApiErrorCode::DoubleRequiresRanked => "double_requires_ranked",
            ApiErrorCode::ReplayUnavailable => "replay_unavailable",
            ApiErrorCode::RateLimited => "rate_limited",
            ApiErrorCode::Unavailable => "unavailable",
            ApiErrorCode::Internal => "internal",
            ApiErrorCode::Forbidden => "forbidden",
            ApiErrorCode::IdempotencyKeyReused => "idempotency_key_reused",
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
/// both here and as `details.retryAfterMs`, which is what the body shows.
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

    /// A 500 error whose body says only "something went wrong", logging `message` as `handler.threw`.
    pub fn internal(message: impl Into<String>) -> ApiError {
        ApiError::new(ApiErrorCode::Internal, message)
    }

    pub fn status(&self) -> u16 {
        self.code.status()
    }

    pub fn body(&self) -> Value {
        let mut error = Map::new();
        error.insert("code".to_string(), Value::String(self.code.as_str().to_string()));
        error.insert("message".to_string(), Value::String(self.message.clone()));
        if let Some(details) = &self.details {
            error.insert("details".to_string(), details.clone());
        }
        json!({ "error": Value::Object(error) })
    }

    /// The body as the client receives it (`code`, `message`, `details`).
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

/// A store failure is a 500. A fenced claim is the one store answer a person can do something
/// about, a 503 (R1523000).
impl From<StoreError> for ApiError {
    fn from(error: StoreError) -> ApiError {
        match error {
            // R1523000: a claim fenced by a newer server's epoch wrote nothing; the caller retries,
            // and the retry reaches the server that holds the seats now.
            StoreError::Fenced => ApiError::new(
                ApiErrorCode::Unavailable,
                "a new server is taking over; try again",
            ),
            error => ApiError::internal(error.to_string()),
        }
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

/// `ok` for anything serializable: the body as its JSON.
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

// Request

/// What every handler is handed (SURFACE §11.2).
///
/// - `caller`: set when the route declares `AuthLevel::User` or `Active`.
/// - `params`: the `:name` segments of the route's path.
/// - `query`: the URL's search parameters, the first value of each name.
/// - `body`: the parsed JSON body; `{}` for a request without one. Anything but a JSON object is
///   refused before a handler runs.
/// - `address`: §9.4, §9.8: the client address is only ever seen hashed.
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

/// The caller of a route that declared `User` or `Active`. The router resolved it before the
/// handler ran; the guard is here because `Req` types it optional, and answers 401 "sign in
/// first", as the router does.
pub fn caller_of(req: &Req) -> Result<&Caller, ApiError> {
    req.caller
        .as_ref()
        .ok_or_else(|| ApiError::new(ApiErrorCode::Unauthorized, "sign in first"))
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
/// Behind a proxy it is `X-Forwarded-For`'s `entries[length - hops]`, the entry the deployment's own
/// proxies wrote: everything left of it the caller wrote, and reading it would let a caller pick a
/// fresh bucket per request. `CF-Connecting-IP` and `X-Real-IP` are never read: nothing here writes
/// them, so a caller can. With fewer entries than hops, or hops 0, it is the socket's peer address.
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

/// A dotted quad, each octet 0–255 without a leading zero.
fn is_ipv4(address: &str) -> bool {
    address.parse::<Ipv4Addr>().is_ok()
}

/// 1 to `max` ASCII digits.
fn is_digits(text: &str, max: usize) -> bool {
    !text.is_empty() && text.len() <= max && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_dotted_quad_shape(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 4 && parts.iter().all(|part| is_digits(part, 3))
}

/// The bracketed host, when the address is one.
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

/// The IPv4 address without its port, when it has one.
fn ipv4_without_port(address: &str) -> Option<&str> {
    let (host, port) = address.split_once(':')?;
    if is_dotted_quad_shape(host) && is_digits(port, usize::MAX) {
        Some(host)
    } else {
        None
    }
}

/// SPEC §11 R190: the key a per-IP limit counts an address under, before it is hashed, with the
/// IPv6 prefix at `IPV6_RATE_LIMIT_PREFIX_BITS`.
pub fn rate_limit_address(raw: &str) -> String {
    rate_limit_address_with_prefix(raw, IPV6_RATE_LIMIT_PREFIX_BITS as i64)
}

/// SPEC §11 R190: the key a per-IP limit counts an address under, before it is hashed.
///
/// An IPv6 address is its first `prefix_bits` bits: a host holds at least a /64 and often a /56, so
/// a finer key would give one host many fresh buckets (§9.4 step 3, R157). An IPv4-mapped IPv6
/// address is the IPv4 address. Brackets, a zone and an IPv4 port are dropped first; anything else
/// is keyed as written, lower-cased.
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

// Routes

/// What a handler returns: its response, or the error `dispatch` answers with. Every handler is
/// `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult`; `app.rs` boxes each with `h!` and
/// lists them in `ROUTES` as `(method, path, AuthLevel, handler)`.
pub type ApiResult = Result<Response, ApiError>;

/// What a route requires of its caller.
///
/// - `None`: open (the catalog, the public statistics).
/// - `User`: a valid access token and a profile, whatever its status: the code screen (§9.4).
/// - `Active`: also `profiles.status == active`, which gives a pending account a 403 from
///   collection, decks and queue (§9.4, BUILD M6-T1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuthLevel {
    None,
    User,
    Active,
}

/// Percent-decodes a URI component, or None for a `%` not followed by two hex digits or for bytes that
/// are not UTF-8.
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

/// URL search parameters, taking the first value of each name.
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
/// gets one created lazily as `pending` (§9.4: the account exists the moment auth says so).
/// SPEC §11 R2478000: a user `known_profiles` knows whose budget is spent is refused with R192's 429
/// before the store is read (`path` names it in the log line); every profile resolved is remembered.
pub async fn resolve_caller(app: &App, headers: &HeaderMap, path: &str) -> Result<Caller, ApiError> {
    let Some(token) = bearer_token(headers) else {
        return Err(ApiError::new(ApiErrorCode::Unauthorized, "sign in first"));
    };
    let user = match app.auth.verify(&token).await {
        Ok(user) => user,
        Err(_) => return Err(ApiError::new(ApiErrorCode::Unauthorized, "sign in first")),
    };

    let now = now_ms();
    if let Some((key, wait)) = spent_account(app, &user.user_id, now) {
        return Err(refuse_rate_limited(app, path, &key, now, wait));
    }

    let mut tx = app.db.begin(None).await?;
    if let Some(existing) = tx.profiles_get_by_user_id(&user.user_id).await? {
        tx.commit().await?;
        app.known_profiles.remember(&user.user_id, &existing.id);
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
        })
        .await?;
    tx.commit().await?;
    app.known_profiles.remember(&user.user_id, &profile.id);
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

// §9.8's per-account rate limit

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

/// SPEC §11 R2478000: the profile id each verified auth user was last resolved to, so a request from
/// an account whose R109 budget is spent is refused before the store is read. It only refuses early:
/// every request it lets by still reads its profile. One per `App`, for the limiter's reason.
pub struct KnownProfiles {
    max: usize,
    ids: Mutex<IndexMap<String, String>>,
}

impl KnownProfiles {
    /// At most `max` users, and at least one.
    pub fn new(max: usize) -> KnownProfiles {
        KnownProfiles {
            max: max.max(1),
            ids: Mutex::new(IndexMap::new()),
        }
    }

    /// The profile id `user_id` was last resolved to, while it is held.
    pub fn get(&self, user_id: &str) -> Option<String> {
        lock(&self.ids).get(user_id).cloned()
    }

    /// Remembers that `user_id` resolved to `profile_id`; a new user past the bound drops the one
    /// remembered longest ago.
    pub fn remember(&self, user_id: &str, profile_id: &str) {
        let mut ids = lock(&self.ids);
        if let Some(held) = ids.get_mut(user_id) {
            if held.as_str() != profile_id {
                *held = profile_id.to_string();
            }
            return;
        }
        while ids.len() >= self.max {
            ids.shift_remove_index(0);
        }
        ids.insert(user_id.to_string(), profile_id.to_string());
    }

    /// How many users are held, so a test can see the bound.
    pub fn size(&self) -> usize {
        lock(&self.ids).len()
    }
}

impl Default for KnownProfiles {
    fn default() -> KnownProfiles {
        KnownProfiles::new(KNOWN_PROFILES_MAX)
    }
}

/// SPEC §11 R2478000: the R109 key of the account `user_id` was last resolved to, and its R192
/// wait, when `known_profiles` knows it and its budget is spent; `None` otherwise. Only looked at,
/// never counted: what the map lets by is counted against the profile the store answers (R157).
fn spent_account(app: &App, user_id: &str, now: i64) -> Option<(String, i64)> {
    let key = account_key(&app.known_profiles.get(user_id)?);
    let wait = app.limiter.retry_after_ms(&key, now);
    (wait > 0).then_some((key, wait))
}

/// SPEC §11 R2478000: the profile behind a socket upgrade whose token verified, the upgrade counted
/// as one request against its account's R109 budget, as `run_route` counts. A known account whose
/// budget is spent is refused before the store is read; a socket never creates a profile (`Ok(None)`).
/// A refusal is R192's `rate_limited`, logged and counted as the router's are (R1717000).
pub async fn upgrade_profile(app: &App, user: &AuthUser, path: &str) -> Result<Option<Profile>, ApiError> {
    let now = now_ms();
    if let Some((key, wait)) = spent_account(app, &user.user_id, now) {
        return Err(refuse_rate_limited(app, path, &key, now, wait));
    }
    let mut tx = app.db.begin(None).await?;
    let profile = tx.profiles_get_by_user_id(&user.user_id).await?;
    tx.commit().await?;
    let Some(profile) = profile else {
        return Ok(None);
    };
    app.known_profiles.remember(&user.user_id, &profile.id);
    let key = account_key(&profile.id);
    let now = now_ms();
    if !app.limiter.allow(&key, now) {
        let wait = app.limiter.retry_after_ms(&key, now);
        return Err(refuse_rate_limited(app, path, &key, now, wait));
    }
    Ok(Some(profile))
}

/// R1717000: one key's refused requests inside `API_REJECTION_ALERT_WINDOW_MS`, at most
/// `API_REJECTION_ALERT_THRESHOLD + 1` of them, and when it last raised `api.rejections.unusual`.
#[derive(Default)]
struct Refusals {
    times: VecDeque<i64>,
    alerted_at: Option<i64>,
}

#[derive(Default)]
struct RateLimiterState {
    hits: IndexMap<String, VecDeque<i64>>,
    swept_at: i64,
    refusals: IndexMap<String, Refusals>,
    refusals_swept_at: i64,
}

/// §9.8: "per-account rate limit at the API", R109: 300 requests per minute per account. One sliding
/// window per key, the shape of the actor's per-seat `flood_exceeded` (`actor/match_actor.rs`), so
/// §9.8's two halves are one mechanism. A request over the limit is not recorded, so a caller who
/// keeps hammering cannot pin their counter open; refusals are counted apart (R1717000). It also
/// holds the router's R190 signal. One per `App`, so one test's flood cannot leak into the next.
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

    /// R1717000: counts one refused request against `key` and answers the count when it raises
    /// `api.rejections.unusual`: once the window holds more than `API_REJECTION_ALERT_THRESHOLD`
    /// refusals, then not again until a whole `API_REJECTION_ALERT_WINDOW_MS` has passed since its last
    /// alert. Each key's record is capped and swept once a window, so a flood holds bounded state.
    pub fn note_rejection(&self, key: &str, now: i64) -> Option<usize> {
        let window = API_REJECTION_ALERT_WINDOW_MS;
        let mut state = lock(&self.state);
        if now - state.refusals_swept_at >= window {
            state.refusals_swept_at = now;
            state
                .refusals
                .retain(|_, refusals| refusals.times.back().is_some_and(|at| *at > now - window));
        }

        let refusals = state.refusals.entry(key.to_string()).or_default();
        while refusals.times.front().is_some_and(|at| *at <= now - window) {
            refusals.times.pop_front();
        }
        refusals.times.push_back(now);
        if refusals.times.len() > API_REJECTION_ALERT_THRESHOLD + 1 {
            refusals.times.pop_front();
        }
        let quiet = refusals.alerted_at.is_none_or(|at| at <= now - window);
        if refusals.times.len() > API_REJECTION_ALERT_THRESHOLD && quiet {
            refusals.alerted_at = Some(now);
            Some(refusals.times.len())
        } else {
            None
        }
    }

    /// How many keys hold refusals, so a test can see quiet ones dropped.
    pub fn rejected_keys(&self) -> usize {
        lock(&self.state).refusals.len()
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

/// The router: `routes` (`app.rs`'s `ROUTES`) tried in order, the one fallback of `app::router`,
/// behind the CORS layer, which answers preflights first.
///
/// §9.8's per-account limit is `app.limiter`, keyed (SPEC §11 R157) on `profiles.id` (§9.4), not the
/// auth user or token a caller can hold several of, since one shared counter lets one caller spend
/// everybody's budget (R137); a request naming no account (R109) counts against its IP hash.
///
/// R190's operator signal: a line each time a request carries fewer `X-Forwarded-For` entries than
/// any before. A caller can add entries but never remove the proxies', so the fewest is the hops they
/// add, and `TRUSTED_PROXY_HOPS` must not be more (docs/architecture.md §10). No address is logged.
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

/// One matched route, from the address hash to the handler.
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
    // §9.4, §9.8: the client address is only ever seen hashed, read once as R190 says.
    let hashes = crate::api::crypto::hashes_for_pepper(&app.env.code_pepper);
    let address = hashes.ip(&rate_limit_address(&client_address(
        &parts.headers,
        context.peer_address.as_deref(),
        trusted_proxy_hops,
    )));

    // An address whose own budget is spent is refused BEFORE its token is resolved: resolving a bad
    // token can cost a round trip to the auth provider, and a flood of bad tokens fills exactly this
    // bucket. Only looked at, not counted: a request naming an account counts against it (R157).
    if auth != AuthLevel::None {
        let key = address_key(&address);
        let now = now_ms();
        let wait = app.limiter.retry_after_ms(&key, now);
        if wait > 0 {
            return Err(refuse_rate_limited(app, path, &key, now, wait));
        }
    }

    // Resolved before the account's rate limit because only auth knows which account a request
    // belongs to, and the account is R109's key. A refusal is held rather than returned, so a flood
    // of bad tokens is still counted — against its address, since it named no account.
    let mut caller: Option<Caller> = None;
    let mut auth_error: Option<ApiError> = None;
    if auth != AuthLevel::None {
        match resolve_caller(app, &parts.headers, path).await {
            Ok(resolved) => caller = Some(resolved),
            Err(error) if error.code == ApiErrorCode::Internal => return Err(error),
            // R2478000: a known account whose budget is spent, refused before its profile was read;
            // already logged and counted.
            Err(error) if error.code == ApiErrorCode::RateLimited => return Err(error),
            Err(error) => auth_error = Some(error),
        }
    }

    // §9.8 (R109): checked before the body is read and before §9.4's gate, so a flood costs the
    // least work this router can manage.
    let key = match &caller {
        None => address_key(&address),
        Some(resolved) => account_key(&resolved.profile.id),
    };
    let now = now_ms();
    if !app.limiter.allow(&key, now) {
        // R192: the refusal says how long until the oldest counted request leaves the window.
        let wait = app.limiter.retry_after_ms(&key, now);
        return Err(refuse_rate_limited(app, path, &key, now, wait));
    }

    if let Some(error) = auth_error {
        return Err(error);
    }
    if auth == AuthLevel::Active
        && let Some(resolved) = &caller
    {
        assert_active(&resolved.profile)?;
    }

    // SPEC §11 R897000: every signed-in request passing auth and the gate refreshes its caller's
    // queue presence; a queued client already polls `GET /api/auth/me`.
    if let Some(resolved) = &caller {
        app.queue_presence.touch(&resolved.profile.id, now_ms());
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

/// §9.8's API rate limit refusing one request: "every rejected action is logged with its reason",
/// and R1717000 counts it, so a key refused past `API_REJECTION_ALERT_THRESHOLD` inside the window
/// raises `api.rejections.unusual`.
fn refuse_rate_limited(app: &App, path: &str, key: &str, now: i64, wait: i64) -> ApiError {
    log_warn("api.rate_limited", json!({ "path": path, "key": key }));
    if let Some(rejected) = app.limiter.note_rejection(key, now) {
        alert_unusual_api_rejections(key, rejected);
    }
    rate_limited("too many requests; slow down", wait)
}

/// R1717000: raises `api.rejections.unusual` (§9.8) for `key`'s `rejected` refused requests in the
/// last `API_REJECTION_ALERT_WINDOW_MS`, the API's half of `match.rejections.unusual` (R835000,
/// `actor/guard.rs`).
fn alert_unusual_api_rejections(key: &str, rejected: usize) {
    log_alert(
        "api.rejections.unusual",
        json!({ "key": key, "rejected": rejected, "windowMs": API_REJECTION_ALERT_WINDOW_MS }),
    );
}

// Timing

/// Epoch milliseconds, the server's one clock: the wall clock anchored once and advanced by tokio's,
/// so a test's `tokio::time::pause()` and `advance()` move it (SURFACE §11.3). Re-exported so every
/// API module reads the same clock.
pub use crate::app::now_ms;

// Logging (SURFACE §11.3: `tracing` JSON lines with the same event names)

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard the account, consent, code and room routes share: a request that reached a handler
    /// with no caller is the router's own 401.
    #[test]
    fn a_request_with_no_caller_is_401_sign_in_first() {
        let req = Req {
            caller: None,
            params: IndexMap::new(),
            query: IndexMap::new(),
            body: json!({}),
            address: String::new(),
            headers: HeaderMap::new(),
        };
        let Err(refused) = caller_of(&req) else {
            panic!("no caller is no caller");
        };
        assert_eq!(refused.code, ApiErrorCode::Unauthorized);
        assert_eq!(refused.message, "sign in first");
        assert_eq!(refused.status(), 401);
    }
}
