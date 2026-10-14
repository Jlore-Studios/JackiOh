//! Play together's social layer (SPEC S7, #936; issues #937, #938, #941): friends by `name#tag`,
//! blocks, presence, challenges and chat between friends. Migration 0036 holds the rows; the store
//! methods are `friends_*`, `blocks_*`, `presence_*`, `challenges_*` and `chat_*` (`db/store.rs`).
//!
//! - `friends`: the friend list, requests (B43–B45) and blocks (B46).
//! - `presence`: the heartbeat and each friend's status, `offline`, `online` or `inMatch` (B47).
//! - `challenges`: a challenge to a friend, which opens a room as `POST /api/rooms` does and is
//!   accepted by joining it (B48, B49).
//! - `chat`: messages between friends, the word filter, the unread counts, reports and the
//!   retention purge (B53).
//!
//! Every route is `AuthLevel::Active` and acts for its caller, named by the token. A player is
//! named by profile id everywhere and shown by username (`PlayerRef`, R1436); no log line carries a
//! username or a message's body. Times leave as ISO-8601 strings (`iso`), as S7's types write them.
//! A refusal with a code of S7's own (`409 {code: "self"}`, `422 {code: "empty"}`, …) is the API's
//! one error shape with that code (`refused`); the rest are `ApiErrorCode`'s.

pub mod challenges;
pub mod chat;
pub mod friends;
pub mod presence;
pub mod shares;
pub mod watch;

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, RateLimiter, Req, create_rate_limiter, json as json_response,
};
use crate::config::{CHAT_MESSAGES_PER_MINUTE, FRIEND_REQUESTS_PER_HOUR};
use crate::db::store::{Profile, StoreError, Tx};

/// Unit conversions, not configuration.
pub(crate) const MS_PER_MINUTE: i64 = 60_000;
pub(crate) const MS_PER_HOUR: i64 = 60 * MS_PER_MINUTE;
pub(crate) const MS_PER_DAY: i64 = 24 * MS_PER_HOUR;
const NANOS_PER_MS: i128 = 1_000_000;

/// `Date.prototype.toISOString`'s form, which S7's time strings take: `2026-10-14T12:34:56.789Z`.
const ISO_FORMAT: &str = "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z";

/// What the social routes keep between requests, one per `App` (as `api::replays::Replays`): their
/// own allowances, on top of R109's API-wide one. A fresh app starts with empty windows, so one
/// test's flood cannot leak into the next.
pub struct Social {
    /// B43: `FRIEND_REQUESTS_PER_HOUR` friend requests an hour per account.
    pub friend_requests: RateLimiter,
    /// B53: `CHAT_MESSAGES_PER_MINUTE` chat messages a minute per sender.
    pub chat: RateLimiter,
}

impl Default for Social {
    fn default() -> Social {
        Social {
            friend_requests: create_rate_limiter(FRIEND_REQUESTS_PER_HOUR, MS_PER_HOUR),
            chat: create_rate_limiter(CHAT_MESSAGES_PER_MINUTE, MS_PER_MINUTE),
        }
    }
}

/// S7's `PlayerRef`: a player as every social answer names one, by profile id, with the username
/// as shown (`Max`, or `Max#3` with a tag, R1432).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRef {
    pub profile_id: String,
    pub username: String,
}

impl PlayerRef {
    pub fn of(profile: &Profile) -> PlayerRef {
        PlayerRef {
            profile_id: profile.id.clone(),
            username: profile.username(),
        }
    }
}

/// The named profiles by id, one store read, for answers that name players. An id whose profile
/// is gone is left out (its rows cascaded with it).
pub(crate) async fn profiles_by_id(
    tx: &mut Tx<'_>,
    ids: &[String],
) -> Result<IndexMap<String, Profile>, StoreError> {
    let mut wanted: Vec<String> = ids.to_vec();
    wanted.sort();
    wanted.dedup();
    Ok(tx
        .profiles_get_many(&wanted)
        .await?
        .into_iter()
        .map(|profile| (profile.id.clone(), profile))
        .collect())
}

/// Epoch ms as an ISO-8601 instant in UTC, as `Date.prototype.toISOString` writes it.
pub fn iso(ms: i64) -> String {
    let Ok(at) = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * NANOS_PER_MS) else {
        return ms.to_string();
    };
    time::format_description::parse_borrowed::<1>(ISO_FORMAT)
        .ok()
        .and_then(|format| at.format(&format).ok())
        .unwrap_or_else(|| ms.to_string())
}

/// An ISO-8601 instant (RFC 3339, the form `iso` writes) as epoch ms, or `None` for anything else.
pub fn parse_iso(text: &str) -> Option<i64> {
    let at = OffsetDateTime::parse(text.trim(), &Rfc3339).ok()?;
    i64::try_from(at.unix_timestamp_nanos().div_euclid(NANOS_PER_MS)).ok()
}

/// A refusal with a code of S7's own, in the API's one error shape (`{ "error": { code, message }
/// }`, what the client's `ApiRequestError` reads): `409 {code: "self"}` and the like. Not an
/// `ApiError`, whose codes are `ApiErrorCode`'s; a refusal is an answer, not a failure, so nothing
/// logs it.
pub(crate) fn refused(status: u16, code: &str, message: &str) -> ApiResult {
    Ok(json_response(
        status,
        json!({ "error": { "code": code, "message": message } }),
    ))
}

/// 404 `not_found`: what every social route answers for a player, request, challenge or message the
/// caller may not reach, a block included, so an answer never tells "blocked" from "unknown" (B43).
pub(crate) fn not_found(message: &str) -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, message)
}

/// 201 with a JSON body.
pub(crate) fn created(body: Value) -> Response {
    json_response(201, body)
}

/// 204 with no body (`DELETE /api/account`'s shape).
pub(crate) fn no_content() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// A `:name` segment of the route's path, or empty (which names nothing).
pub(crate) fn param(req: &Req, name: &str) -> String {
    req.params.get(name).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_writes_what_to_iso_string_writes_and_reads_it_back() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso(1_760_445_296_789), "2025-10-14T12:34:56.789Z");
        assert_eq!(parse_iso("2025-10-14T12:34:56.789Z"), Some(1_760_445_296_789));
        assert_eq!(parse_iso(&iso(1_760_445_296_789)), Some(1_760_445_296_789));
        assert_eq!(parse_iso("2025-10-14T14:34:56.789+02:00"), Some(1_760_445_296_789));
        assert_eq!(parse_iso("yesterday"), None);
        assert_eq!(parse_iso("1760445296789"), None);
    }
}
